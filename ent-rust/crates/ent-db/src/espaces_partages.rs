//! Socle 32, lot ET1 — partages nominatifs entre personnels (`32_BUREAUTIQUE.md` §1 principe 4, §8.3, §17.4 ;
//! ETR-130 à ETR-134 ; NX32-14 à NX32-16 ; R-ET2, R-ET9, R-ET12, R-ET13 ; ND32-03, ND32-08).
//!
//! * Un partage vise l'**identifiant** d'un élément ou d'un dossier (jamais un chemin : un déplacement ne le casse pas, R-ET13,
//!   ND32-08), une **personne nommée** de l'établissement (agent actif résolu par le cœur : R-ET12, ND32-03), en lecture ou en
//!   écriture, **daté** (début = jour du partage, fin facultative). Aucun lien public (R-ET9).
//! * Qui partage : l'**auteur** de l'élément ou du dossier, ou l'animateur de l'espace (ETR-130). Un destinataire, même en
//!   écriture, ne repartage pas.
//! * Un fichier non analysé ne se partage pas ; un partage ne donne jamais plus que ce que l'espace permet de lire : il
//!   ne rend pas lisible un élément supprimé ni non analysé.
//! * En ET1, **aucun partage vers un élève** : les destinataires sont des agents (le partage aux élèves suivis par un
//!   professeur arrive avec les membres calculés, lot ET2).

use crate::espaces::*;
use crate::espaces_contenu::{acces_dossier, acces_element, Acces};
use crate::espaces_service::*;
use crate::notifications::db;
use crate::service::{EntDbError, PgEnt};
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use sqlx::Row;
use std::collections::BTreeSet;
use uuid::Uuid;

fn vue(r: &sqlx::postgres::PgRow) -> Option<PartageVue> {
    Some(PartageVue {
        id: r.get(0),
        element_id: r.get(1),
        dossier_id: r.get(2),
        destinataire_personne_id: r.get(3),
        destinataire_nom: r.get(4),
        droit: DroitPartage::depuis_code(&r.get::<String, _>(5))?,
        debut: r.get(6),
        fin: r.get(7),
    })
}

const SELECT_PARTAGE: &str = "SELECT s.id, s.element_id, s.dossier_id, s.destinataire_personne_id,
        btrim(COALESCE(p.prenom || ' ', '') || p.nom), s.droit, s.debut, s.fin
   FROM ent_espace_partage s
   JOIN ent_personne p ON p.etablissement_id = s.etablissement_id AND p.id = s.destinataire_personne_id";

impl PgEnt {
    /// Partage un élément ou un dossier avec des personnes nommées (ETR-130, ETR-150) : auteur ou animateur ; destinataires
    /// = agents actifs, distincts, différents de moi ; un partage actif existant du même destinataire est **remplacé**
    /// (droit et date de fin), jamais empilé ; avis neutre aux destinataires (ETR-152).
    pub async fn espaces_partager(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        d: DemandePartage,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<PartageVue>, EntDbError> {
        self.exiger_action_etab(etab, user, role, ent_auth::Action::EspacesPartager).await?;
        if d.element_id.is_some() == d.dossier_id.is_some() {
            return Err(invalide("Indiquez soit un fichier, soit un dossier à partager."));
        }
        if d.destinataires.is_empty() || d.destinataires.len() > PARTAGES_PAR_GESTE_MAX {
            return Err(invalide("Choisissez entre 1 et 50 collègues."));
        }
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        if d.fin.is_some_and(|f: NaiveDate| f < jour) {
            return Err(invalide("La date de fin ne peut pas être passée."));
        }
        // Cible et droit de partager : auteur, ou animateur de l'espace.
        let (espace_id, auteur, acces): (Uuid, Uuid, Acces) = if let Some(el_id) = d.element_id {
            let (el, e, a) = acces_element(&mut tx, etab, el_id, moi, jour, false, false).await?;
            exiger_ouvert(&e)?;
            if el.etat != "disponible" {
                return Err(conflit("Ce fichier est en cours d'analyse : il pourra être partagé ensuite."));
            }
            (el.espace_id, el.auteur, a)
        } else {
            let (dl, e, a) = acces_dossier(&mut tx, etab, d.dossier_id.unwrap_or_default(), moi, jour, false).await?;
            exiger_ouvert(&e)?;
            (dl.espace_id, dl.auteur, a)
        };
        if !(acces.anime() || (auteur == moi.personne && acces.role.is_some())) {
            return Err(interdit("Seul l'auteur ou l'animateur peut partager cet élément."));
        }
        let mut vus = BTreeSet::new();
        for n in &d.destinataires {
            if n.destinataire_personne_id == moi.personne || !vus.insert(n.destinataire_personne_id) {
                return Err(invalide("Un destinataire ne peut être choisi qu'une fois, et vous ne vous partagez pas à vous-même."));
            }
            exiger_collegue_actif(&mut tx, etab, n.destinataire_personne_id, jour).await?;
        }
        let mut out = Vec::new();
        for n in &d.destinataires {
            // Remplacement d'un partage actif existant (même cible, même destinataire).
            let existant: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM ent_espace_partage
                  WHERE etablissement_id = $1 AND destinataire_personne_id = $2 AND retire_le IS NULL
                    AND element_id IS NOT DISTINCT FROM $3 AND dossier_id IS NOT DISTINCT FROM $4
                  FOR UPDATE",
            )
            .bind(etab)
            .bind(n.destinataire_personne_id)
            .bind(d.element_id)
            .bind(d.dossier_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let (id, nouveau) = match existant {
                Some(id) => {
                    sqlx::query(
                        "UPDATE ent_espace_partage SET droit = $3, fin = $4 WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(n.droit.code())
                    .bind(d.fin)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    (id, false)
                }
                None => {
                    let id = Uuid::new_v4();
                    sqlx::query(
                        "INSERT INTO ent_espace_partage
                            (id, etablissement_id, espace_id, element_id, dossier_id, destinataire_personne_id, droit, debut, fin,
                             par_personne_id, cree_le)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                    )
                    .bind(id)
                    .bind(etab)
                    .bind(espace_id)
                    .bind(d.element_id)
                    .bind(d.dossier_id)
                    .bind(n.destinataire_personne_id)
                    .bind(n.droit.code())
                    .bind(jour)
                    .bind(d.fin)
                    .bind(moi.personne)
                    .bind(maintenant)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    (id, true)
                }
            };
            journaliser(&mut tx, etab, Some(espace_id), d.element_id, Some(moi.personne), if nouveau { "partage.cree" } else { "partage.modifie" }, Some(n.droit.code()), maintenant).await?;
            let evt = publier_evenement(
                &mut tx,
                etab,
                "espaces.partage_cree",
                id,
                Some(user),
                json!({ "espace_id": espace_id, "partage_id": id, "droit": n.droit.code() }),
            )
            .await?;
            if nouveau {
                inviter(&mut tx, etab, n.destinataire_personne_id, id, evt, maintenant).await?;
            }
            let l = sqlx::query(&format!("{SELECT_PARTAGE} WHERE s.etablissement_id = $1 AND s.id = $2"))
                .bind(etab)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            if let Some(v) = vue(&l) {
                out.push(v);
            }
        }
        toucher_activite(&mut tx, etab, espace_id, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// À qui est partagé un élément ou un dossier (ETR-134) : auteur ou animateur.
    pub async fn espaces_partages_de(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Option<Uuid>,
        dossier_id: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<PartageVue>, EntDbError> {
        if element_id.is_some() == dossier_id.is_some() {
            return Err(invalide("Indiquez soit un fichier, soit un dossier."));
        }
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (auteur, acces) = if let Some(el) = element_id {
            let (e, _, a) = acces_element(&mut tx, etab, el, moi, jour, false, false).await?;
            (e.auteur, a)
        } else {
            let (d, _, a) = acces_dossier(&mut tx, etab, dossier_id.unwrap_or_default(), moi, jour, false).await?;
            (d.auteur, a)
        };
        if !(acces.anime() || (auteur == moi.personne && acces.role.is_some())) {
            return Err(introuvable());
        }
        let l = sqlx::query(&format!(
            "{SELECT_PARTAGE}
              WHERE s.etablissement_id = $1 AND s.retire_le IS NULL
                AND s.element_id IS NOT DISTINCT FROM $2 AND s.dossier_id IS NOT DISTINCT FROM $3
                AND s.debut <= $4 AND (s.fin IS NULL OR s.fin >= $4)
              ORDER BY lower(p.nom), lower(COALESCE(p.prenom, ''))"
        ))
        .bind(etab)
        .bind(element_id)
        .bind(dossier_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l.iter().filter_map(vue).collect())
    }

    /// Droit de gérer un partage existant : celui qui l'a posé, l'auteur de la cible, l'animateur de l'espace.
    async fn partage_gerable(
        &self,
        tx: &mut sqlx::PgConnection,
        etab: Uuid,
        partage: Uuid,
        moi: Moi,
        jour: NaiveDate,
    ) -> Result<(Uuid, Option<Uuid>, bool, bool), EntDbError> {
        let l = sqlx::query(
            "SELECT s.espace_id, s.element_id, s.dossier_id, s.par_personne_id, s.destinataire_personne_id
               FROM ent_espace_partage s
              WHERE s.etablissement_id = $1 AND s.id = $2 AND s.retire_le IS NULL FOR UPDATE",
        )
        .bind(etab)
        .bind(partage)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let (espace_id, element, dossier, par, destinataire): (Uuid, Option<Uuid>, Option<Uuid>, Uuid, Uuid) =
            (l.get(0), l.get(1), l.get(2), l.get(3), l.get(4));
        let esp = ligne_espace(tx, etab, espace_id, false).await?.ok_or_else(introuvable)?;
        let role = role_dans_espace(tx, etab, &esp, moi.personne, jour).await?;
        let auteur: Uuid = if let Some(el) = element {
            sqlx::query_scalar("SELECT auteur_personne_id FROM ent_espace_element WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(el)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?
        } else {
            sqlx::query_scalar("SELECT cree_par_personne_id FROM ent_espace_dossier WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(dossier.unwrap_or_default())
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?
        };
        let gestionnaire = role.is_some_and(|r| r.est_animateur()) || par == moi.personne || (auteur == moi.personne && role.is_some());
        let destinataire_moi = destinataire == moi.personne;
        if !gestionnaire && !destinataire_moi {
            return Err(introuvable());
        }
        Ok((espace_id, element, gestionnaire, destinataire_moi))
    }

    /// Retire un partage (ETR-133) : l'auteur de la cible, celui qui l'a posé ou l'animateur. Effet immédiat.
    pub async fn espaces_partage_retirer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        partage_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (espace, element, gestionnaire, _) = self.partage_gerable(&mut tx, etab, partage_id, moi, jour).await?;
        if !gestionnaire {
            return Err(interdit("Seul l'auteur ou l'animateur peut retirer ce partage."));
        }
        sqlx::query(
            "UPDATE ent_espace_partage SET retire_le = $3, retire_par_personne_id = $4, retire_motif_code = 'auteur'
              WHERE etablissement_id = $1 AND id = $2 AND retire_le IS NULL",
        )
        .bind(etab)
        .bind(partage_id)
        .bind(maintenant)
        .bind(moi.personne)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, Some(espace), element, Some(moi.personne), "partage.retire", None, maintenant).await?;
        publier_evenement(&mut tx, etab, "espaces.partage_retire", partage_id, Some(user), json!({ "espace_id": espace, "partage_id": partage_id })).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Change la date de fin d'un partage (ETR-133) ; `None` retire la fin.
    pub async fn espaces_partage_modifier_fin(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        partage_id: Uuid,
        fin: Option<NaiveDate>,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (espace, element, gestionnaire, _) = self.partage_gerable(&mut tx, etab, partage_id, moi, jour).await?;
        if !gestionnaire {
            return Err(interdit("Seul l'auteur ou l'animateur peut modifier ce partage."));
        }
        if fin.is_some_and(|f| f < jour) {
            return Err(invalide("La date de fin ne peut pas être passée."));
        }
        sqlx::query("UPDATE ent_espace_partage SET fin = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(partage_id)
            .bind(fin)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journaliser(&mut tx, etab, Some(espace), element, Some(moi.personne), "partage.modifie", Some("fin"), maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Quitte un partage reçu (ETR-132, NX32-16) : le destinataire seul.
    pub async fn espaces_partage_quitter(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        partage_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (espace, element, _, destinataire_moi) = self.partage_gerable(&mut tx, etab, partage_id, moi, jour).await?;
        if !destinataire_moi {
            return Err(introuvable());
        }
        sqlx::query(
            "UPDATE ent_espace_partage SET retire_le = $3, retire_par_personne_id = $4, retire_motif_code = 'destinataire'
              WHERE etablissement_id = $1 AND id = $2 AND retire_le IS NULL",
        )
        .bind(etab)
        .bind(partage_id)
        .bind(maintenant)
        .bind(moi.personne)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, Some(espace), element, Some(moi.personne), "partage.quitte", None, maintenant).await?;
        publier_evenement(&mut tx, etab, "espaces.partage_quitte", partage_id, Some(user), json!({ "espace_id": espace, "partage_id": partage_id })).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// « Partagés avec moi » (ETR-131, NX32-02) : partages actifs dont je suis destinataire, à la date. Un élément
    /// supprimé, non analysé ou dont l'espace est effacé n'apparaît pas. **Aucune écriture.**
    pub async fn espaces_partages_avec_moi(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<PartageRecu>, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let l = sqlx::query(
            "SELECT s.id, s.espace_id, s.element_id, s.dossier_id, COALESCE(el.nom, d.nom),
                    btrim(COALESCE(pp.prenom || ' ', '') || pp.nom), s.droit, s.debut, s.fin, (s.vu_le IS NOT NULL)
               FROM ent_espace_partage s
               JOIN ent_espace e ON e.etablissement_id = s.etablissement_id AND e.id = s.espace_id AND e.etat <> 'efface'
               JOIN ent_personne pp ON pp.etablissement_id = s.etablissement_id AND pp.id = s.par_personne_id
               LEFT JOIN ent_espace_element el ON el.etablissement_id = s.etablissement_id AND el.id = s.element_id
               LEFT JOIN ent_espace_dossier d ON d.etablissement_id = s.etablissement_id AND d.id = s.dossier_id
              WHERE s.etablissement_id = $1 AND s.destinataire_personne_id = $2 AND s.retire_le IS NULL
                AND s.debut <= $3 AND (s.fin IS NULL OR s.fin >= $3)
                AND ((el.id IS NOT NULL AND el.etat = 'disponible') OR (d.id IS NOT NULL AND d.supprime_le IS NULL))
              ORDER BY s.cree_le DESC",
        )
        .bind(etab)
        .bind(moi.personne)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l.iter()
            .filter_map(|r| {
                Some(PartageRecu {
                    partage_id: r.get(0),
                    espace_id: r.get(1),
                    element_id: r.get(2),
                    dossier_id: r.get(3),
                    nom: r.get(4),
                    de_nom: r.get(5),
                    droit: DroitPartage::depuis_code(&r.get::<String, _>(6))?,
                    debut: r.get(7),
                    fin: r.get(8),
                    vu: r.get(9),
                })
            })
            .collect())
    }
}
