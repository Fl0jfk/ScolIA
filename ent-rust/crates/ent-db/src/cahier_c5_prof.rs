//! Socle 20, lot C5 : **suivi, constats, retours et ramassage**, côté professeur (`20_CAHIER_DE_TEXTES.md` §10.2, §10.3,
//! CDT-184 à CDT-201).
//!
//! * Le droit est celui de l'**intervenant en saisie à la date** sur le cours (R-C2, R-C13) : ni la direction, ni un autre
//!   professeur, ni un remplaçant de séance ne lisent ni ne corrigent les copies (la direction lit sur motif journalisé,
//!   `cdt_piece_lien`).
//! * Un constat (`non_rendu`, `dispense`, `excuse`, `rendu_papier`) est un **fait** en ajout seul (R-C12) ; `non_rendu`
//!   n'est constatable qu'à partir de la clôture du ramassage et jamais pour un élève qui a déposé.
//! * Une prolongation individuelle est en ajout seul (la plus récente fait foi) ; le motif est codé, interne, jamais servi.
//! * Un retour rendu n'écrase rien : toute correction est une nouvelle version, la précédente reste.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_auth::{Action, Portee, Role};
use ent_events::types;
use ent_types::cahier_c5::*;
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::cahier::{publier_cdt, FichierDepot};
use crate::cahier_c2::{conflit, invalide};
use crate::cahier_c5::*;
use crate::cahier_c8::db;
use crate::ref_commun::{introuvable, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Élèves au plus d'une prolongation ou d'un constat en une fois.
const MAX_LOT: usize = 400;
/// Critères au plus d'une grille de retour.
const MAX_CRITERES: usize = 20;

#[derive(Debug, Clone, FromRow)]
struct EleveLigne {
    id: Uuid,
    nom: String,
    prenom: String,
}

fn nom_archive(nom: &str, prenom: &str, titre: &str, rang: usize, ext: &str) -> String {
    let net = |s: &str| {
        s.chars()
            .map(|c| if "/\\:*?\"<>|\n\r\t".contains(c) || c.is_control() { ' ' } else { c })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let base = format!("{} {} – {}", net(nom).to_uppercase(), net(prenom), net(titre));
    let base: String = base.chars().take(120).collect();
    if rang <= 1 {
        format!("{base}.{ext}")
    } else {
        format!("{base} ({rang}).{ext}")
    }
}

impl PgEnt {
    /// Travail du professeur : charge et contrôle le droit de saisie à la date sur son cours.
    async fn travail_du_prof(&self, a: &Acteur<'_>, travail_id: Uuid) -> Result<TravailC5, EntDbError> {
        let etab = a.etablissement_id;
        let t = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            charger_travail(&mut tx, etab, travail_id).await?
        };
        self.cdt_exiger_saisie(a, t.cours_id, None).await?;
        Ok(t)
    }

    /// Élèves concernés par un travail : le public à la date (ou la liste restreinte), plus quiconque porte déjà une
    /// remise (un élève parti ne fait pas disparaître sa copie).
    async fn eleves_du_travail(&self, tx: &mut PgConnection, etab: Uuid, t: &TravailC5, jour: NaiveDate) -> Result<Vec<EleveLigne>, EntDbError> {
        sqlx::query_as(
            "SELECT e.id, e.nom, e.prenom FROM ent_eleve e
              WHERE e.etablissement_id = $1
                AND (
                  (EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $3, $2, NULL, e.id, false))
                   AND ($4 = 'cours' OR EXISTS (SELECT 1 FROM ent_cdt_travail_cible k
                                                 WHERE k.etablissement_id = $1 AND k.travail_id = $5 AND k.eleve_id = e.id)))
                  OR EXISTS (SELECT 1 FROM ent_cdt_remise r WHERE r.etablissement_id = $1 AND r.travail_id = $5 AND r.eleve_id = e.id)
                  OR EXISTS (SELECT 1 FROM ent_cdt_groupe_membre m JOIN ent_cdt_groupe g ON g.etablissement_id = m.etablissement_id AND g.id = m.groupe_id
                              WHERE m.etablissement_id = $1 AND g.travail_id = $5 AND m.eleve_id = e.id AND m.jusqu_au IS NULL))
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(t.cours_id)
        .bind(jour)
        .bind(&t.public)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    /// CDT-184 : le tableau de suivi d'un travail à rendre (une requête par nature de donnée, jamais une par élève).
    pub async fn cdt_suivi(&self, a: &Acteur<'_>, travail_id: Uuid) -> Result<SuiviC5Dto, EntDbError> {
        let t = self.travail_du_prof(a, travail_id).await?;
        if t.rendu == "aucun" {
            return Err(conflit("ce travail n'est pas à rendre"));
        }
        let etab = a.etablissement_id;
        self.cdt_projeter_echus(etab).await?;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleves = self.eleves_du_travail(&mut tx, etab, &t, jour).await?;
        let ids: Vec<Uuid> = eleves.iter().map(|e| e.id).collect();
        let effectives: HashMap<Uuid, (DateTime<Utc>, bool)> = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
            "SELECT e.id, ent_cdt_echeance_effective($1, $2, e.id, $3) FROM unnest($4::uuid[]) AS e(id)",
        )
        .bind(etab)
        .bind(t.id)
        .bind(t.echeance_le)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(id, eff)| (id, (eff, eff != t.echeance_le)))
        .collect();
        let jours_cloture: i32 = sqlx::query_scalar("SELECT COALESCE((SELECT r.cloture_ramassage_jours FROM ent_cdt_reglage r WHERE r.etablissement_id = $1 AND r.annee_id = $2), 14)::int")
            .bind(etab)
            .bind(t.annee_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let cloture = t.cloture_ramassage_le.unwrap_or(t.echeance_le + Duration::days(i64::from(jours_cloture)));
        let cloture_atteinte = Utc::now() > cloture;

        // Groupes et appartenances actives.
        let groupes_lignes: Vec<(Uuid, String)> =
            sqlx::query_as("SELECT id, libelle FROM ent_cdt_groupe WHERE etablissement_id = $1 AND travail_id = $2 ORDER BY libelle, id")
                .bind(etab)
                .bind(t.id)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
        let membres: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT m.groupe_id, m.eleve_id FROM ent_cdt_groupe_membre m JOIN ent_cdt_groupe g ON g.etablissement_id = m.etablissement_id AND g.id = m.groupe_id
              WHERE m.etablissement_id = $1 AND g.travail_id = $2 AND m.jusqu_au IS NULL",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let groupe_de: HashMap<Uuid, Uuid> = membres.iter().map(|(g, e)| (*e, *g)).collect();
        let libelle_groupe: HashMap<Uuid, String> = groupes_lignes.iter().cloned().collect();

        // Remises, dépôts, retours, constats, demandes, signalements.
        let remises: Vec<RemiseRowC5> = sqlx::query_as(&format!("{SELECT_REMISE_C5} AND travail_id = $2"))
            .bind(etab)
            .bind(t.id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let remise_eleve: HashMap<Uuid, &RemiseRowC5> = remises.iter().filter_map(|r| r.eleve_id.map(|e| (e, r))).collect();
        let remise_groupe: HashMap<Uuid, &RemiseRowC5> = remises.iter().filter_map(|r| r.groupe_id.map(|g| (g, r))).collect();
        let remise_ids: Vec<Uuid> = remises.iter().map(|r| r.id).collect();
        let depots: HashMap<Uuid, (i64, DateTime<Utc>)> = sqlx::query_as::<_, (Uuid, i64, DateTime<Utc>)>(
            "SELECT remise_id, count(*), max(recu_le) FROM ent_cdt_depot WHERE etablissement_id = $1 AND remise_id = ANY($2) GROUP BY remise_id",
        )
        .bind(etab)
        .bind(&remise_ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(r, n, d)| (r, (n, d)))
        .collect();
        let retours: HashMap<Uuid, String> = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT remise_id, etat FROM ent_cdt_retour WHERE etablissement_id = $1 AND remise_id = ANY($2)",
        )
        .bind(etab)
        .bind(&remise_ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let constats: HashMap<Uuid, String> = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT DISTINCT ON (eleve_id) eleve_id, statut FROM ent_cdt_statut
              WHERE etablissement_id = $1 AND travail_id = $2 ORDER BY eleve_id, cree_le DESC, id DESC",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let demandes: HashMap<Uuid, Uuid> = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT eleve_id, id FROM ent_cdt_delai_demande WHERE etablissement_id = $1 AND travail_id = $2 AND etat = 'ouverte'",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let signalements: HashMap<Uuid, DateTime<Utc>> = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
            "SELECT eleve_id, max(cree_le) FROM ent_cdt_depot_signalement WHERE etablissement_id = $1 AND travail_id = $2 GROUP BY eleve_id",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();

        let mut comptes = ComptesSuiviDto { attendus: eleves.len() as i32, ..Default::default() };
        let mut lignes = Vec::new();
        for e in &eleves {
            let groupe = groupe_de.get(&e.id).copied();
            let remise = remise_eleve.get(&e.id).copied().or_else(|| groupe.and_then(|g| remise_groupe.get(&g).copied()));
            let depot = remise.and_then(|r| depots.get(&r.id));
            let constat = constats.get(&e.id).map(String::as_str).filter(|c| *c != CONSTAT_ATTENDU);
            let (eff, prolonge) = effectives.get(&e.id).copied().unwrap_or((t.echeance_le, false));
            // Un constat du professeur prime ; sinon le dépôt ; sinon « attendu ».
            let statut = match (constat, depot, remise) {
                (Some(c), _, _) => c.to_string(),
                (None, Some(_), Some(r)) if r.en_retard => "en_retard".to_string(),
                (None, Some(_), _) => "rendu".to_string(),
                _ => "attendu".to_string(),
            };
            match statut.as_str() {
                "rendu" => comptes.rendus += 1,
                "en_retard" => {
                    comptes.rendus += 1;
                    comptes.en_retard += 1;
                }
                "rendu_papier" => comptes.rendu_papier += 1,
                "non_rendu" => comptes.non_rendus += 1,
                "dispense" => comptes.dispenses += 1,
                "excuse" => comptes.excuses += 1,
                _ => {}
            }
            let retour = match remise.and_then(|r| retours.get(&r.id)).map(String::as_str) {
                Some("rendu") if depot.is_some() => {
                    comptes.retours_rendus += 1;
                    "rendu"
                }
                Some("brouillon") => "brouillon",
                _ => {
                    if depot.is_some() {
                        comptes.retours_a_faire += 1;
                        "a_faire"
                    } else {
                        "aucun"
                    }
                }
            };
            let constat_propose = (cloture_atteinte && depot.is_none() && constat.is_none()).then(|| CONSTAT_NON_RENDU.to_string());
            lignes.push(LigneSuiviDto {
                eleve_id: e.id,
                nom: e.nom.clone(),
                prenom: e.prenom.clone(),
                groupe: groupe.and_then(|g| libelle_groupe.get(&g).cloned()),
                statut: Some(statut),
                remise_id: remise.map(|r| r.id),
                dernier_depot_le: depot.map(|d| d.1),
                nb_depots: depot.map(|d| d.0 as i32).unwrap_or(0),
                echeance_effective_le: eff,
                prolonge,
                retour: retour.to_string(),
                delai_demande_id: demandes.get(&e.id).copied(),
                signalement_le: signalements.get(&e.id).copied(),
                constat_propose,
            });
        }
        let mut groupes = Vec::new();
        for (id, libelle) in groupes_lignes {
            let noms: Vec<MembreGroupeDto> = membres
                .iter()
                .filter(|(g, _)| *g == id)
                .filter_map(|(_, e)| eleves.iter().find(|x| x.id == *e))
                .map(|x| MembreGroupeDto { eleve_id: x.id, nom: x.nom.clone(), prenom: x.prenom.clone() })
                .collect();
            let complet = t.taille_groupe.is_some_and(|n| noms.len() as i64 >= i64::from(n));
            groupes.push(GroupeDto { id, libelle, membres: noms, complet });
        }
        Ok(SuiviC5Dto {
            travail_id: t.id,
            titre: t.titre.clone(),
            cours_id: t.cours_id,
            rendu: t.rendu.clone(),
            echeance_le: t.echeance_le,
            cloture_ramassage_le: cloture,
            cloture_atteinte,
            remise_par_groupe: t.remise_par_groupe,
            comptes,
            lignes,
            groupes,
        })
    }

    /// CDT-201 : fixer la clôture du ramassage d'un travail (défaut S20-09).
    pub async fn cdt_cloture_fixer(&self, a: &Acteur<'_>, travail_id: Uuid, body: ClotureBody) -> Result<DateTime<Utc>, EntDbError> {
        let t = self.travail_du_prof(a, travail_id).await?;
        let etab = a.etablissement_id;
        self.cdt_exiger_annee_ouverte(etab, t.annee_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let instant = instant_local(&mut tx, etab, body.cloture_jour, body.cloture_heure).await?;
        if instant < t.echeance_le {
            return Err(invalide("la clôture du ramassage ne peut pas précéder l'échéance"));
        }
        sqlx::query("UPDATE ent_cdt_travail SET cloture_ramassage_le = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(t.id)
            .bind(instant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(instant)
    }

    /// CDT-192 : dépôt au nom de l'élève (copie papier scannée, remise à l'oral) : « déposé par M. X le … », sans pénalité
    /// liée au canal (CC-42).
    pub async fn cdt_deposer_au_nom(
        &self,
        a: &Acteur<'_>,
        travail_id: Uuid,
        eleve_id: Uuid,
        body: DepotEntree,
        recu_le: DateTime<Utc>,
    ) -> Result<AccuseDto, EntDbError> {
        let t = self.travail_du_prof(a, travail_id).await?;
        self.cdt_exiger_annee_ouverte(a.etablissement_id, t.annee_id).await?;
        self.cdt_deposer_pour(a, travail_id, eleve_id, body.texte, None, body.fichiers, recu_le, true).await
    }

    /// CDT-193 : constater en lot `rendu_papier`, `non_rendu`, `dispense`, `excuse` (ou revenir à `attendu`).
    pub async fn cdt_constater(&self, a: &Acteur<'_>, travail_id: Uuid, body: ConstatsBody) -> Result<usize, EntDbError> {
        let t = self.travail_du_prof(a, travail_id).await?;
        let etab = a.etablissement_id;
        self.cdt_exiger_annee_ouverte(etab, t.annee_id).await?;
        if body.constats.is_empty() || body.constats.len() > MAX_LOT {
            return Err(invalide("de 1 à 400 constats à la fois"));
        }
        for c in &body.constats {
            if ![CONSTAT_RENDU_PAPIER, CONSTAT_NON_RENDU, CONSTAT_DISPENSE, CONSTAT_EXCUSE, CONSTAT_ATTENDU].contains(&c.statut.as_str()) {
                return Err(invalide("constat inconnu"));
            }
        }
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleves: HashSet<Uuid> = self.eleves_du_travail(&mut tx, etab, &t, jour).await?.into_iter().map(|e| e.id).collect();
        let jours_cloture: i32 = sqlx::query_scalar("SELECT COALESCE((SELECT r.cloture_ramassage_jours FROM ent_cdt_reglage r WHERE r.etablissement_id = $1 AND r.annee_id = $2), 14)::int")
            .bind(etab)
            .bind(t.annee_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let cloture = t.cloture_ramassage_le.unwrap_or(t.echeance_le + Duration::days(i64::from(jours_cloture)));
        for c in &body.constats {
            if !eleves.contains(&c.eleve_id) {
                return Err(invalide("un élève hors du public du travail ne peut pas être constaté"));
            }
            if c.statut == CONSTAT_NON_RENDU {
                if Utc::now() <= cloture {
                    return Err(conflit("« non rendu » ne se constate qu'à partir de la clôture du ramassage"));
                }
                let a_depose: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_cdt_depot d JOIN ent_cdt_remise r ON r.etablissement_id = d.etablissement_id AND r.id = d.remise_id
                      WHERE r.etablissement_id = $1 AND r.travail_id = $2
                        AND (r.eleve_id = $3 OR r.groupe_id IN (SELECT m.groupe_id FROM ent_cdt_groupe_membre m WHERE m.etablissement_id = $1 AND m.eleve_id = $3 AND m.jusqu_au IS NULL)))",
                )
                .bind(etab)
                .bind(t.id)
                .bind(c.eleve_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if a_depose {
                    return Err(conflit("cet élève a déposé : il n'est pas « non rendu »"));
                }
            }
            sqlx::query("INSERT INTO ent_cdt_statut (id, etablissement_id, travail_id, eleve_id, statut, par_user_id) VALUES ($1, $2, $3, $4, $5, $6)")
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(t.id)
                .bind(c.eleve_id)
                .bind(&c.statut)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        publier_cdt(&mut tx, a, types::CAHIER_STATUT_CONSTATE, t.id, None, json!({ "travail_id": t.id, "nb": body.constats.len() })).await?;
        tx.commit().await.map_err(db)?;
        Ok(body.constats.len())
    }

    /// Recalcule le « en retard » des remises d'un élève après une prolongation (CC-41) : le dernier dépôt humain (hors
    /// « au nom de ») est comparé à la nouvelle échéance effective.
    pub(crate) async fn recalculer_retard(tx: &mut PgConnection, etab: Uuid, t: &TravailC5, eleve: Uuid) -> Result<(), EntDbError> {
        sqlx::query(
            "UPDATE ent_cdt_remise r SET en_retard = COALESCE(
                    (SELECT NOT d.au_nom_de AND d.recu_le > ent_cdt_echeance_effective($1, $2, $3, $4)
                       FROM ent_cdt_depot d WHERE d.etablissement_id = r.etablissement_id AND d.id = r.dernier_depot_id), false),
                    modifie_le = now()
              WHERE r.etablissement_id = $1 AND r.travail_id = $2 AND r.dernier_depot_id IS NOT NULL
                AND (r.eleve_id = $3 OR r.groupe_id IN (SELECT m.groupe_id FROM ent_cdt_groupe_membre m
                                                         WHERE m.etablissement_id = $1 AND m.eleve_id = $3 AND m.jusqu_au IS NULL))",
        )
        .bind(etab)
        .bind(t.id)
        .bind(eleve)
        .bind(t.echeance_le)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Ok(())
    }

    /// CDT-194 : accorder un délai à des élèves (aménagement, absence, problème technique). Motif codé, interne.
    pub async fn cdt_prolonger(&self, a: &Acteur<'_>, travail_id: Uuid, body: ProlongationBody) -> Result<usize, EntDbError> {
        let t = self.travail_du_prof(a, travail_id).await?;
        let etab = a.etablissement_id;
        self.cdt_exiger_annee_ouverte(etab, t.annee_id).await?;
        if !MOTIFS_PROLONGATION.contains(&body.motif.as_str()) {
            return Err(invalide("motif de prolongation inconnu"));
        }
        if body.eleve_ids.is_empty() || body.eleve_ids.len() > MAX_LOT {
            return Err(invalide("de 1 à 400 élèves à la fois"));
        }
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let instant = instant_local(&mut tx, etab, body.echeance_jour, body.echeance_heure).await?;
        let eleves: HashSet<Uuid> = self.eleves_du_travail(&mut tx, etab, &t, jour).await?.into_iter().map(|e| e.id).collect();
        for e in &body.eleve_ids {
            if !eleves.contains(e) {
                return Err(invalide("un élève hors du public du travail ne peut pas recevoir de délai"));
            }
            let (actuelle, _, _) = echeances_de(&mut tx, etab, &t, *e).await?;
            if instant <= actuelle {
                return Err(invalide("le délai accordé doit dépasser l'échéance actuelle de l'élève"));
            }
        }
        for e in &body.eleve_ids {
            sqlx::query("INSERT INTO ent_cdt_echeance_eleve (id, etablissement_id, travail_id, eleve_id, echeance_le, motif_code, par_user_id) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(t.id)
                .bind(e)
                .bind(instant)
                .bind(&body.motif)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            Self::recalculer_retard(&mut tx, etab, &t, *e).await?;
            // Une demande de délai ouverte pour cet élève est désormais satisfaite.
            sqlx::query(
                "UPDATE ent_cdt_delai_demande SET etat = 'acceptee', decide_par_user_id = $4, decide_le = now()
                  WHERE etablissement_id = $1 AND travail_id = $2 AND eleve_id = $3 AND etat = 'ouverte'",
            )
            .bind(etab)
            .bind(t.id)
            .bind(e)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        publier_cdt(&mut tx, a, types::CAHIER_DELAI_ACCORDE, t.id, None, json!({ "travail_id": t.id, "nb": body.eleve_ids.len() })).await?;
        tx.commit().await.map_err(db)?;
        Ok(body.eleve_ids.len())
    }

    /// CDT-195 : demandes de délai en attente d'un travail.
    pub async fn cdt_delais_attente(&self, a: &Acteur<'_>, travail_id: Uuid) -> Result<Vec<DelaiAttenteDto>, EntDbError> {
        let t = self.travail_du_prof(a, travail_id).await?;
        let etab = a.etablissement_id;
        let lignes: Vec<(Uuid, Uuid, String, String, Option<String>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT d.id, d.eleve_id, e.nom, e.prenom, d.message, d.cree_le FROM ent_cdt_delai_demande d
               JOIN ent_eleve e ON e.etablissement_id = d.etablissement_id AND e.id = d.eleve_id
              WHERE d.etablissement_id = $1 AND d.travail_id = $2 AND d.etat = 'ouverte' ORDER BY d.cree_le, d.id",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(|(id, eleve_id, nom, prenom, message, cree_le)| DelaiAttenteDto { id, travail_id: t.id, eleve_id, nom, prenom, message, cree_le })
            .collect())
    }

    /// CDT-195 : répondre à une demande (acceptée avec une nouvelle échéance, ou refusée avec un motif facultatif).
    pub async fn cdt_delai_decider(&self, a: &Acteur<'_>, demande_id: Uuid, body: DelaiDecisionBody) -> Result<(), EntDbError> {
        let etab = a.etablissement_id;
        let (travail_id, eleve_id): (Uuid, Uuid) = sqlx::query_as(
            "SELECT travail_id, eleve_id FROM ent_cdt_delai_demande WHERE etablissement_id = $1 AND id = $2 AND etat = 'ouverte'",
        )
        .bind(etab)
        .bind(demande_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let t = self.travail_du_prof(a, travail_id).await?;
        let reponse = body.reponse.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
        if reponse.as_ref().is_some_and(|r| r.chars().count() > 1000) {
            return Err(invalide("réponse trop longue"));
        }
        if body.accepter {
            let (Some(jour), Some(heure)) = (body.echeance_jour, body.echeance_heure) else {
                return Err(invalide("indiquez la nouvelle échéance accordée"));
            };
            self.cdt_prolonger(
                a,
                travail_id,
                ProlongationBody { eleve_ids: vec![eleve_id], echeance_jour: jour, echeance_heure: heure, motif: "autre".into() },
            )
            .await?;
            // `cdt_prolonger` a déjà marqué la demande acceptée ; on y ajoute la réponse et le lien à la prolongation.
            let mut tx_reponse = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query(
                "UPDATE ent_cdt_delai_demande SET reponse = $3, echeance_eleve_id =
                    (SELECT x.id FROM ent_cdt_echeance_eleve x WHERE x.etablissement_id = $1 AND x.travail_id = $4 AND x.eleve_id = $5
                      ORDER BY x.cree_le DESC, x.id DESC LIMIT 1)
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(demande_id)
            .bind(&reponse)
            .bind(t.id)
            .bind(eleve_id)
            .execute(&mut *tx_reponse)
            .await
            .map_err(db)?;
            tx_reponse.commit().await.map_err(db)?;
        } else {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query(
                "UPDATE ent_cdt_delai_demande SET etat = 'refusee', reponse = $3, decide_par_user_id = $4, decide_le = now()
                  WHERE etablissement_id = $1 AND id = $2 AND etat = 'ouverte'",
            )
            .bind(etab)
            .bind(demande_id)
            .bind(&reponse)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            publier_cdt(&mut tx, a, types::CAHIER_DELAI_DECIDE, t.id, Some(eleve_id), json!({ "travail_id": t.id, "demande_id": demande_id, "acceptee": false })).await?;
            tx.commit().await.map_err(db)?;
        }
        Ok(())
    }

    // ───────────────────────────── retours ─────────────────────────────

    async fn remise_du_prof(&self, a: &Acteur<'_>, remise_id: Uuid) -> Result<(RemiseRowC5, TravailC5), EntDbError> {
        let etab = a.etablissement_id;
        let (r, t) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let r: RemiseRowC5 = sqlx::query_as(&format!("{SELECT_REMISE_C5} AND id = $2"))
                .bind(etab)
                .bind(remise_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?;
            let t = charger_travail(&mut tx, etab, r.travail_id).await?;
            (r, t)
        };
        self.cdt_exiger_saisie(a, t.cours_id, None).await?;
        Ok((r, t))
    }

    /// CDT-07 / 187 / 189 / 190 : enregistrer le retour d'une copie (brouillon), le rendre, ou le corriger (nouvelle version).
    pub async fn cdt_retour_enregistrer(&self, a: &Acteur<'_>, remise_id: Uuid, body: RetourC5Body, fichiers: Vec<FichierDepot>) -> Result<RetourProfDto, EntDbError> {
        let (r, t) = self.remise_du_prof(a, remise_id).await?;
        let etab = a.etablissement_id;
        self.cdt_exiger_annee_ouverte(etab, t.annee_id).await?;
        if r.dernier_depot_id.is_none() {
            return Err(conflit("aucune copie déposée : rien à corriger"));
        }
        let commentaire = body.commentaire.trim().to_string();
        if commentaire.chars().count() > 4000 {
            return Err(invalide("commentaire trop long"));
        }
        if body.criteres.len() > MAX_CRITERES {
            return Err(invalide("trop de critères"));
        }
        for c in &body.criteres {
            if c.libelle.trim().is_empty() || c.libelle.chars().count() > 200 || c.niveau.as_deref().is_some_and(|n| !NIVEAUX_CRITERE.contains(&n)) {
                return Err(invalide("critère invalide"));
            }
        }
        let formats: Vec<String> = vec![];
        let prets = self.cdt_preparer_fichiers(etab, &fichiers, &formats).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Verrou de la remise : deux corrections simultanées s'ordonnent.
        sqlx::query("SELECT 1 FROM ent_cdt_remise WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(r.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        // C-D8 : la note liée existe pour cet élève, de la main du correcteur, et n'est pas déjà portée par un autre retour.
        if let Some(n) = body.note_id {
            let Some(eleve) = r.eleve_id else {
                return Err(invalide("une note ne se lie pas à la copie d'un groupe"));
            };
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_note n WHERE n.etablissement_id = $1 AND n.id = $2 AND n.eleve_id = $3
                                   AND n.auteur_user_id = $4
                                   AND NOT EXISTS (SELECT 1 FROM ent_cdt_retour o WHERE o.etablissement_id = n.etablissement_id
                                                    AND o.note_id = n.id AND o.remise_id <> $5))",
            )
            .bind(etab)
            .bind(n)
            .bind(eleve)
            .bind(a.user_id)
            .bind(r.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(invalide("note introuvable pour cet élève"));
            }
        }
        let existant: Option<(Uuid, String, i32, String, Option<Uuid>, Option<DateTime<Utc>>, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, etat, version, commentaire, note_id, rendu_le, auteur_user_id FROM ent_cdt_retour WHERE etablissement_id = $1 AND remise_id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(r.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let agent: Option<Uuid> = sqlx::query_scalar(
            "SELECT ag.id FROM ent_agent ag JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
              WHERE ag.etablissement_id = $1 AND pe.user_id = $2 ORDER BY ag.id LIMIT 1",
        )
        .bind(etab)
        .bind(a.user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (retour_id, version, deja_rendu) = match existant {
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_cdt_retour (id, etablissement_id, remise_id, commentaire, note_id, montrer_note, auteur_user_id, auteur_agent_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                )
                .bind(id)
                .bind(etab)
                .bind(r.id)
                .bind(&commentaire)
                .bind(body.note_id)
                .bind(body.montrer_note && body.note_id.is_some())
                .bind(a.user_id)
                .bind(agent)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                (id, 1, false)
            }
            Some((id, etat, version, ancien_commentaire, ancienne_note, ancien_rendu, ancien_auteur)) => {
                let rendu = etat == "rendu";
                if rendu {
                    // Correction d'un retour rendu : la version précédente reste (R-C29 : versions lisibles).
                    sqlx::query(
                        "INSERT INTO ent_cdt_retour_version (id, etablissement_id, retour_id, version, commentaire, note_id, rendu_le, auteur_user_id)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(etab)
                    .bind(id)
                    .bind(version)
                    .bind(&ancien_commentaire)
                    .bind(ancienne_note)
                    .bind(ancien_rendu)
                    .bind(ancien_auteur)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                }
                sqlx::query(
                    "UPDATE ent_cdt_retour SET commentaire = $3, note_id = $4, montrer_note = $5, version = version + $6,
                            auteur_user_id = $7, auteur_agent_id = COALESCE($8, auteur_agent_id), modifie_le = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(&commentaire)
                .bind(body.note_id)
                .bind(body.montrer_note && body.note_id.is_some())
                .bind(i32::from(rendu))
                .bind(a.user_id)
                .bind(agent)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                // Les critères sont ceux de la version courante.
                sqlx::query("DELETE FROM ent_cdt_retour_critere WHERE etablissement_id = $1 AND retour_id = $2")
                    .bind(etab)
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                (id, version + i32::from(rendu), rendu)
            }
        };
        for (i, c) in body.criteres.iter().enumerate() {
            sqlx::query("INSERT INTO ent_cdt_retour_critere (etablissement_id, retour_id, ordre, libelle, niveau) VALUES ($1, $2, $3, $4, $5)")
                .bind(etab)
                .bind(retour_id)
                .bind((i + 1) as i16)
                .bind(c.libelle.trim())
                .bind(&c.niveau)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let deja: i16 = sqlx::query_scalar("SELECT count(*)::smallint FROM ent_cdt_piece_jointe WHERE etablissement_id = $1 AND porteur = 'retour' AND porteur_id = $2 AND retiree_le IS NULL")
            .bind(etab)
            .bind(retour_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if i64::from(deja) + prets.len() as i64 > MAX_PIECES_DEPOT {
            return Err(conflit("nombre maximal de pièces atteint"));
        }
        attacher_pieces(&mut tx, etab, "retour", retour_id, deja, &prets, "retour").await?;
        // Rendre : un retour déjà rendu le reste (visible sitôt corrigé) ; un brouillon devient rendu si on le demande.
        if body.rendre && !deja_rendu {
            self.rendre_retour(&mut tx, a, etab, &t, &r, retour_id).await?;
        } else if deja_rendu {
            publier_cdt(&mut tx, a, types::CAHIER_RETOUR_MODIFIE, t.id, r.eleve_id, json!({ "travail_id": t.id, "remise_id": r.id, "version": version })).await?;
        }
        let avertissements = self.avertir_prenoms(&mut tx, etab, &t, &r, &commentaire).await?;
        let dto = Self::retour_prof_dto(&mut tx, etab, retour_id, r.id, avertissements).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    async fn rendre_retour(
        &self,
        tx: &mut PgConnection,
        a: &Acteur<'_>,
        etab: Uuid,
        t: &TravailC5,
        r: &RemiseRowC5,
        retour_id: Uuid,
    ) -> Result<(), EntDbError> {
        let n = sqlx::query("UPDATE ent_cdt_retour SET etat = 'rendu', rendu_le = now(), modifie_le = now() WHERE etablissement_id = $1 AND id = $2 AND etat = 'brouillon'")
            .bind(etab)
            .bind(retour_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
        if n > 0 {
            publier_cdt(tx, a, types::CAHIER_COPIE_RENDUE, t.id, r.eleve_id, json!({ "travail_id": t.id, "remise_id": r.id })).await?;
        }
        Ok(())
    }

    /// CC-38 : contrôle doux. Un prénom d'un **autre** élève du cours dans le commentaire d'une copie est signalé à l'auteur.
    async fn avertir_prenoms(&self, tx: &mut PgConnection, etab: Uuid, t: &TravailC5, r: &RemiseRowC5, texte: &str) -> Result<Vec<String>, EntDbError> {
        if texte.trim().is_empty() {
            return Ok(vec![]);
        }
        let jour = self.jour_etab(etab).await?;
        let eleves = self.eleves_du_travail(tx, etab, t, jour).await?;
        let propres: HashSet<Uuid> = match (r.eleve_id, r.groupe_id) {
            (Some(e), _) => HashSet::from([e]),
            (_, Some(g)) => sqlx::query_scalar::<_, Uuid>("SELECT eleve_id FROM ent_cdt_groupe_membre WHERE etablissement_id = $1 AND groupe_id = $2")
                .bind(etab)
                .bind(g)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?
                .into_iter()
                .collect(),
            _ => HashSet::new(),
        };
        let mots: HashSet<String> = texte
            .split(|c: char| !c.is_alphanumeric())
            .filter(|m| m.chars().count() >= 3)
            .map(str::to_lowercase)
            .collect();
        let mut sortie = Vec::new();
        for e in eleves.iter().filter(|e| !propres.contains(&e.id)) {
            if mots.contains(&e.prenom.to_lowercase()) && !sortie.contains(&e.prenom) {
                sortie.push(e.prenom.clone());
            }
        }
        Ok(sortie.into_iter().map(|p| format!("Le prénom « {p} » est celui d'un autre élève du cours : relisez avant de rendre.")).collect())
    }

    async fn retour_prof_dto(tx: &mut PgConnection, etab: Uuid, retour_id: Uuid, remise_id: Uuid, avertissements: Vec<String>) -> Result<RetourProfDto, EntDbError> {
        #[derive(FromRow)]
        struct Row {
            etat: String,
            version: i32,
            commentaire: String,
            montrer_note: bool,
            note_id: Option<Uuid>,
            rendu_le: Option<DateTime<Utc>>,
        }
        let r: Row = sqlx::query_as("SELECT etat, version, commentaire, montrer_note, note_id, rendu_le FROM ent_cdt_retour WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(retour_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let criteres: Vec<(String, Option<String>)> = sqlx::query_as("SELECT libelle, niveau FROM ent_cdt_retour_critere WHERE etablissement_id = $1 AND retour_id = $2 ORDER BY ordre")
            .bind(etab)
            .bind(retour_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let pieces: Vec<(Uuid, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT p.id, p.nom, o.mime, o.taille FROM ent_cdt_piece_jointe p JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.id = p.objet_id
              WHERE p.etablissement_id = $1 AND p.porteur = 'retour' AND p.porteur_id = $2 AND p.retiree_le IS NULL ORDER BY p.ordre, p.id",
        )
        .bind(etab)
        .bind(retour_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(RetourProfDto {
            retour_id,
            remise_id,
            etat: r.etat,
            version: r.version,
            commentaire: r.commentaire,
            criteres: criteres.into_iter().map(|(libelle, niveau)| CritereDto { libelle, niveau }).collect(),
            montrer_note: r.montrer_note,
            note_id: r.note_id,
            rendu_le: r.rendu_le,
            pieces: pieces.into_iter().map(|(id, nom, mime, taille)| PieceC5Dto { id, nom, mime, taille }).collect(),
            avertissements,
        })
    }

    /// Le retour d'une copie, brouillon compris (professeur en saisie).
    pub async fn cdt_retour_lire_prof(&self, a: &Acteur<'_>, remise_id: Uuid) -> Result<Option<RetourProfDto>, EntDbError> {
        let (r, _t) = self.remise_du_prof(a, remise_id).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ent_cdt_retour WHERE etablissement_id = $1 AND remise_id = $2")
            .bind(etab)
            .bind(r.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        match id {
            None => Ok(None),
            Some(id) => Ok(Some(Self::retour_prof_dto(&mut tx, etab, id, r.id, vec![]).await?)),
        }
    }

    /// CDT-188 : rendre toutes les copies corrigées d'un travail en un geste (les brouillons non vides).
    pub async fn cdt_retours_rendre_en_lot(&self, a: &Acteur<'_>, travail_id: Uuid) -> Result<usize, EntDbError> {
        let t = self.travail_du_prof(a, travail_id).await?;
        let etab = a.etablissement_id;
        self.cdt_exiger_annee_ouverte(etab, t.annee_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT rt.id, rt.remise_id FROM ent_cdt_retour rt JOIN ent_cdt_remise r ON r.etablissement_id = rt.etablissement_id AND r.id = rt.remise_id
              WHERE rt.etablissement_id = $1 AND r.travail_id = $2 AND rt.etat = 'brouillon'
                AND (length(btrim(rt.commentaire)) > 0
                     OR EXISTS (SELECT 1 FROM ent_cdt_retour_critere c WHERE c.etablissement_id = rt.etablissement_id AND c.retour_id = rt.id)
                     OR EXISTS (SELECT 1 FROM ent_cdt_piece_jointe p WHERE p.etablissement_id = rt.etablissement_id AND p.porteur = 'retour' AND p.porteur_id = rt.id AND p.retiree_le IS NULL))
              ORDER BY rt.id FOR UPDATE OF rt",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut n = 0;
        for (retour_id, remise_id) in lignes {
            let r: RemiseRowC5 = sqlx::query_as(&format!("{SELECT_REMISE_C5} AND id = $2"))
                .bind(etab)
                .bind(remise_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            self.rendre_retour(&mut tx, a, etab, &t, &r, retour_id).await?;
            n += 1;
        }
        tx.commit().await.map_err(db)?;
        Ok(n)
    }

    /// CDT-191 : rouvrir une remise (« à refaire ») avec une nouvelle échéance individuelle ; la première remise reste.
    pub async fn cdt_remise_rouvrir(&self, a: &Acteur<'_>, remise_id: Uuid, body: RouvrirBody) -> Result<(), EntDbError> {
        let (r, t) = self.remise_du_prof(a, remise_id).await?;
        let etab = a.etablissement_id;
        self.cdt_exiger_annee_ouverte(etab, t.annee_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let instant = instant_local(&mut tx, etab, body.echeance_jour, body.echeance_heure).await?;
        if instant <= Utc::now() {
            return Err(invalide("la nouvelle échéance doit être à venir"));
        }
        let courante: RemiseRowC5 = sqlx::query_as(&format!("{SELECT_REMISE_C5} AND id = $2 FOR UPDATE"))
            .bind(etab)
            .bind(r.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if courante.etat != "deposee" {
            return Err(conflit("seule une remise déposée se rouvre"));
        }
        let membres: Vec<Uuid> = match (courante.eleve_id, courante.groupe_id) {
            (Some(e), _) => vec![e],
            (_, Some(g)) => sqlx::query_scalar("SELECT eleve_id FROM ent_cdt_groupe_membre WHERE etablissement_id = $1 AND groupe_id = $2 AND jusqu_au IS NULL")
                .bind(etab)
                .bind(g)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?,
            _ => vec![],
        };
        if t.cloture_ramassage_le.is_some_and(|c| instant > c) {
            return Err(conflit("la clôture du ramassage précède cette échéance : repoussez-la d'abord"));
        }
        for e in &membres {
            sqlx::query("INSERT INTO ent_cdt_echeance_eleve (id, etablissement_id, travail_id, eleve_id, echeance_le, motif_code, par_user_id) VALUES ($1, $2, $3, $4, $5, 'autre', $6)")
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(t.id)
                .bind(e)
                .bind(instant)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        sqlx::query("UPDATE ent_cdt_remise SET etat = 'a_refaire', version = version + 1, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(r.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        publier_cdt(&mut tx, a, types::CAHIER_REMISE_ROUVERTE, t.id, courante.eleve_id, json!({ "travail_id": t.id, "remise_id": r.id })).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// CDT-199 : signaler un contenu préoccupant d'une copie à la CPE, à la direction ou à la protection de l'enfance. La
    /// personne saisie lit la copie, journalisée (`cdt_piece_lien`) ; le professeur ne transmet jamais la copie ailleurs.
    pub async fn cdt_copie_signaler(&self, a: &Acteur<'_>, remise_id: Uuid, body: SignalementCopieBody) -> Result<(), EntDbError> {
        if !["cpe", "direction", "protection"].contains(&body.vers.as_str()) {
            return Err(invalide("destinataire du signalement inconnu"));
        }
        let (r, t) = self.remise_du_prof(a, remise_id).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("INSERT INTO ent_cdt_signalement_copie (id, etablissement_id, remise_id, vers, par_user_id) VALUES ($1, $2, $3, $4, $5)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(r.id)
            .bind(&body.vers)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        publier_cdt(&mut tx, a, types::CAHIER_COPIE_SIGNALEE, t.id, r.eleve_id, json!({ "travail_id": t.id, "remise_id": r.id, "vers": body.vers })).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// CDT-185 : la liste des copies à télécharger en bloc, nommées « NOM Prénom – titre » ; chaque pièce s'ouvre par un
    /// lien court (`cdt_piece_lien`). Seul le dépôt courant de chaque élève (l'historique reste consultable à part).
    pub async fn cdt_copies_a_telecharger(&self, a: &Acteur<'_>, travail_id: Uuid) -> Result<Vec<CopieATelechargerDto>, EntDbError> {
        let t = self.travail_du_prof(a, travail_id).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, String, String, Uuid, String)> = sqlx::query_as(
            "SELECT e.id, e.nom, e.prenom, p.id, p.nom
               FROM ent_cdt_remise r
               JOIN ent_cdt_piece_jointe p ON p.etablissement_id = r.etablissement_id AND p.porteur = 'depot' AND p.porteur_id = r.dernier_depot_id AND p.retiree_le IS NULL
               JOIN ent_eleve e ON e.etablissement_id = r.etablissement_id
                AND e.id = COALESCE(r.eleve_id, (SELECT m.eleve_id FROM ent_cdt_groupe_membre m WHERE m.etablissement_id = r.etablissement_id AND m.groupe_id = r.groupe_id ORDER BY m.eleve_id LIMIT 1))
              WHERE r.etablissement_id = $1 AND r.travail_id = $2 AND r.dernier_depot_id IS NOT NULL
              ORDER BY e.nom, e.prenom, p.ordre, p.id",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut rang: HashMap<Uuid, usize> = HashMap::new();
        let sortie: Vec<CopieATelechargerDto> = lignes
            .into_iter()
            .map(|(eleve_id, nom, prenom, piece_id, nom_piece)| {
                let n = rang.entry(eleve_id).or_insert(0);
                *n += 1;
                let ext = nom_piece.rsplit_once('.').map(|(_, e)| e).unwrap_or("bin");
                CopieATelechargerDto { eleve_id, piece_id, nom_archive: nom_archive(&nom, &prenom, &t.titre, *n, ext) }
            })
            .collect();
        publier_cdt(&mut tx, a, types::CAHIER_COPIES_TELECHARGEES, t.id, None, json!({ "travail_id": t.id, "nb": sortie.len() })).await?;
        tx.commit().await.map_err(db)?;
        Ok(sortie)
    }

    /// CDT-196 : relancer les non-rendus (élèves et, au choix, familles). Sans `confirmer`, un **aperçu** seulement : l'envoi
    /// est une action sensible soumise à confirmation humaine. Une seule relance par élève et par jour.
    pub async fn cdt_relancer(&self, a: &Acteur<'_>, travail_id: Uuid, body: RelanceBody) -> Result<RelanceDto, EntDbError> {
        let t = self.travail_du_prof(a, travail_id).await?;
        let etab = a.etablissement_id;
        let suivi = self.cdt_suivi(a, travail_id).await?;
        let cibles: Vec<Uuid> = suivi
            .lignes
            .iter()
            .filter(|l| l.statut.as_deref() == Some("attendu") && l.echeance_effective_le < Utc::now())
            .filter(|l| body.eleve_ids.is_empty() || body.eleve_ids.contains(&l.eleve_id))
            .map(|l| l.eleve_id)
            .collect();
        let texte = "Un travail attendu n'a pas encore été remis. Vous pouvez le déposer dans le cahier de textes ; en cas de difficulté, écrivez à votre professeur.".to_string();
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let deja: Vec<Uuid> = sqlx::query_scalar("SELECT eleve_id FROM ent_cdt_relance WHERE etablissement_id = $1 AND travail_id = $2 AND jour = $3")
            .bind(etab)
            .bind(t.id)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let a_relancer: Vec<Uuid> = cibles.iter().copied().filter(|e| !deja.contains(e)).collect();
        if !body.confirmer {
            return Ok(RelanceDto { envoyee: false, destinataires: a_relancer.len() as i32, deja_relances: (cibles.len() - a_relancer.len()) as i32, texte });
        }
        for e in &a_relancer {
            sqlx::query("INSERT INTO ent_cdt_relance (id, etablissement_id, travail_id, eleve_id, aux_familles, jour, par_user_id) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING")
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(t.id)
                .bind(e)
                .bind(body.aux_familles)
                .bind(jour)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        if !a_relancer.is_empty() {
            // Un événement par envoi : le socle 16 ouvre un fil par élève et par foyer ; la charge ne porte que des identifiants.
            publier_cdt(&mut tx, a, types::CAHIER_RELANCE_NON_RENDUS, t.id, None, json!({ "travail_id": t.id, "eleve_ids": a_relancer, "aux_familles": body.aux_familles })).await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(RelanceDto { envoyee: true, destinataires: a_relancer.len() as i32, deja_relances: (cibles.len() - a_relancer.len()) as i32, texte })
    }

    // ───────────────────────────── panne (R-C11) ─────────────────────────────

    /// R-C11 : une panne déclarée par la direction prolonge, de sa durée (au moins une heure), les échéances en ligne qui
    /// tombaient dans les deux heures qui la suivent ou pendant elle, pour tous les élèves du public. Rejouable : une
    /// panne ne prolonge un travail qu'une fois.
    pub async fn cdt_panne_declarer(&self, a: &Acteur<'_>, body: PanneBody) -> Result<PanneDto, EntDbError> {
        let etab = a.etablissement_id;
        let droit = self
            .droits_effectifs(etab, a.user_id, a.role)
            .await?
            .iter()
            .any(|d| d.action == Action::CahierParametrer && d.portee == Portee::Etablissement);
        if !droit || Role::parse(a.role).is_none() {
            return Err(EntDbError::Forbidden);
        }
        if body.fin_le <= body.debut_le || body.fin_le - body.debut_le > Duration::days(3) {
            return Err(invalide("panne : période invalide"));
        }
        let duree = (body.fin_le - body.debut_le).max(Duration::hours(1));
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let panne_id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_cdt_panne (id, etablissement_id, debut_le, fin_le, declare_par_user_id) VALUES ($1, $2, $3, $4, $5)")
            .bind(panne_id)
            .bind(etab)
            .bind(body.debut_le)
            .bind(body.fin_le)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let travaux: Vec<Uuid> = sqlx::query_scalar(
            "SELECT t.id FROM ent_cdt_travail t
              WHERE t.etablissement_id = $1 AND t.etat = 'publie' AND t.rendu = 'en_ligne'
                AND t.echeance_le > $2 - interval '2 hours' AND t.echeance_le < $3 + interval '2 hours' AND t.echeance_le >= $2
              ORDER BY t.id",
        )
        .bind(etab)
        .bind(body.debut_le)
        .bind(body.fin_le)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let minutes = duree.num_minutes() as i32;
        for tid in &travaux {
            let t = charger_travail(&mut tx, etab, *tid).await?;
            let eleves = self.eleves_du_travail(&mut tx, etab, &t, jour).await?;
            for e in eleves {
                let (actuelle, _, _) = echeances_de(&mut tx, etab, &t, e.id).await?;
                sqlx::query(
                    "INSERT INTO ent_cdt_echeance_eleve (id, etablissement_id, travail_id, eleve_id, echeance_le, motif_code, par_user_id, panne_id)
                     VALUES ($1, $2, $3, $4, $5, 'panne', $6, $7) ON CONFLICT DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(t.id)
                .bind(e.id)
                .bind(actuelle + duree)
                .bind(a.user_id)
                .bind(panne_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                Self::recalculer_retard(&mut tx, etab, &t, e.id).await?;
            }
            publier_cdt(&mut tx, a, types::CAHIER_PROLONGATION_PANNE, t.id, None, json!({ "travail_id": t.id, "panne_id": panne_id, "minutes": minutes })).await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(PanneDto { panne_id, travaux_prolonges: travaux.len() as i32, prolongation_minutes: minutes })
    }
}
