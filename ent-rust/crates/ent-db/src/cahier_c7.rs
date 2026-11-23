//! Socle 20, lot C7 : **adaptations** (`20_CAHIER_DE_TEXTES.md` §13) — variantes, aménagements appliqués, AESH, réglages
//! d'affichage de l'élève.
//!
//! * **Variantes** : consigne, durée, échéance propres à des élèves nommés. L'élève lit **sa** version comme la seule (le
//!   serveur la substitue dans l'agenda et la charge) ; aucun indicateur, aucun compteur, aucun libellé de variante ne figure dans
//!   une réponse destinée à un élève ou à une famille : un camarade ne détecte pas une variante (test de réponse identique).
//!   Le libellé interne n'est lu que du professeur en saisie.
//! * **Aménagements** : le port du socle 35 est **simulé** (`cahier_amenagements.rs`) ; le cahier ne connaît que des faits utiles
//!   d'un catalogue fermé. « Appliquer les aménagements » majore l'échéance effective (motif codé interne `amenagement`, jamais
//!   servi) sur demande du professeur, avec aperçu d'abord.
//! * **AESH** : voit l'agenda, le cahier et les variantes des **seuls élèves accompagnés à la date** ; ajoute une note
//!   d'accompagnement lue des professeurs du cours et du PP ; ne lit ni copies, ni retours, ni notes.
//! * **Réglages d'affichage** : l'outil de l'élève (taille de police, interligne, lecture audio, animations), sans intervention du
//!   professeur ; jamais lu d'un adulte.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_auth::Role;
use ent_events::types;
use ent_types::cahier_c3::AgendaDto;
use ent_types::cahier_c6::EntreeRemplacementDto;
use ent_types::cahier_c6::SeanceReportDto;
use ent_types::cahier_c7::*;
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::cahier::publier_cdt;
use crate::cahier_c2::{conflit, invalide, texte_borne};
use crate::cahier_c3::lignes_agenda;
use crate::cahier_c5::{charger_travail, echeances_de, eleve_dans_public, instant_local, jour_heure_locaux, TravailC5};
use crate::cahier_c8::db;
use crate::ref_commun::{introuvable, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Élèves au plus d'une attribution en une fois.
const MAX_ATTRIBUTION: usize = 60;
/// Variantes au plus par travail.
const MAX_VARIANTES: i64 = 12;
/// Plus longue période d'agenda demandée par l'AESH (jours).
const MAX_PERIODE_JOURS: i64 = 100;
/// Majoration maximale d'une échéance par un aménagement (jours).
const MAJORATION_MAX_JOURS: i64 = 14;

#[derive(Debug, FromRow)]
struct VarianteRow {
    id: Uuid,
    travail_id: Uuid,
    libelle_interne: String,
    consigne: String,
    duree_estimee_min: Option<i16>,
    echeance_le: Option<DateTime<Utc>>,
    version: i32,
}

impl PgEnt {
    /// Travail dont l'acteur a la saisie (R-C2) : chargé avec son cours.
    async fn travail_saisie_c7(&self, a: &Acteur<'_>, travail_id: Uuid) -> Result<TravailC5, EntDbError> {
        let etab = a.etablissement_id;
        let t = charger_travail(&mut *TenantTx::begin(self.pool(), etab).await?, etab, travail_id).await?;
        self.cdt_exiger_saisie(a, t.cours_id, None).await?;
        self.cdt_exiger_annee_ouverte(etab, t.annee_id).await?;
        if t.etat == "retire" {
            return Err(conflit("ce travail a été retiré de la diffusion"));
        }
        Ok(t)
    }

    async fn variante_dto(&self, tx: &mut PgConnection, etab: Uuid, v: VarianteRow) -> Result<VarianteDto, EntDbError> {
        let eleves: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT e.id, e.nom, e.prenom FROM ent_cdt_variante_eleve ve JOIN ent_eleve e ON e.etablissement_id = ve.etablissement_id AND e.id = ve.eleve_id
              WHERE ve.etablissement_id = $1 AND ve.variante_id = $2 AND ve.retiree_le IS NULL ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(v.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(VarianteDto {
            id: v.id,
            travail_id: v.travail_id,
            libelle_interne: v.libelle_interne,
            consigne: v.consigne,
            duree_estimee_min: v.duree_estimee_min,
            echeance_le: v.echeance_le,
            version: v.version,
            eleves: eleves.into_iter().map(|(eleve_id, nom, prenom)| EleveVarianteDto { eleve_id, nom, prenom }).collect(),
        })
    }

    /// CDT-266 : les variantes d'un travail et les élèves nommés auxquels elles sont attribuées (professeur en saisie seulement).
    pub async fn cdt_variantes_lister(&self, a: &Acteur<'_>, travail_id: Uuid) -> Result<Vec<VarianteDto>, EntDbError> {
        let etab = a.etablissement_id;
        let t = {
            let t = charger_travail(&mut *TenantTx::begin(self.pool(), etab).await?, etab, travail_id).await?;
            self.cdt_exiger_saisie(a, t.cours_id, None).await?;
            t
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<VarianteRow> = sqlx::query_as(
            "SELECT id, travail_id, libelle_interne, consigne, duree_estimee_min, echeance_le, version FROM ent_cdt_variante
              WHERE etablissement_id = $1 AND travail_id = $2 ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        for v in lignes {
            sortie.push(self.variante_dto(&mut tx, etab, v).await?);
        }
        Ok(sortie)
    }

    async fn variante_corps(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        body: &VarianteBody,
    ) -> Result<(String, String, Option<DateTime<Utc>>), EntDbError> {
        let libelle = texte_borne(&body.libelle_interne, 80, true, "libellé interne vide ou trop long (80 caractères au plus)")?;
        let consigne = texte_borne(&body.consigne, 20_000, true, "consigne vide ou trop longue (20 000 caractères au plus)")?;
        if body.duree_estimee_min.is_some_and(|d| !(1..=600).contains(&d)) {
            return Err(invalide("durée estimée : de 1 à 600 minutes"));
        }
        let echeance = match (body.echeance_jour, body.echeance_heure) {
            (None, None) => None,
            (Some(j), Some(h)) => {
                let i = instant_local(tx, etab, j, h).await?;
                if i <= Utc::now() {
                    return Err(invalide("l'échéance d'une variante doit être à venir"));
                }
                Some(i)
            }
            _ => return Err(invalide("indiquez le jour et l'heure de l'échéance, ou aucun des deux")),
        };
        Ok((libelle, consigne, echeance))
    }

    /// CDT-266 : crée une variante (consigne, durée, échéance) ; aucune attribution tant qu'elle n'est pas faite.
    pub async fn cdt_variante_creer(&self, a: &Acteur<'_>, travail_id: Uuid, body: VarianteBody) -> Result<VarianteDto, EntDbError> {
        let t = self.travail_saisie_c7(a, travail_id).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (libelle, consigne, echeance) = self.variante_corps(&mut tx, etab, &body).await?;
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_cdt_variante WHERE etablissement_id = $1 AND travail_id = $2")
            .bind(etab)
            .bind(t.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if n >= MAX_VARIANTES {
            return Err(conflit("nombre maximal de variantes atteint pour ce travail"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_cdt_variante (id, etablissement_id, travail_id, libelle_interne, consigne, duree_estimee_min, echeance_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(etab)
        .bind(t.id)
        .bind(&libelle)
        .bind(&consigne)
        .bind(body.duree_estimee_min)
        .bind(echeance)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let v: VarianteRow = sqlx::query_as(
            "SELECT id, travail_id, libelle_interne, consigne, duree_estimee_min, echeance_le, version FROM ent_cdt_variante WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let dto = self.variante_dto(&mut tx, etab, v).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    async fn variante_du_prof(&self, a: &Acteur<'_>, variante_id: Uuid) -> Result<(Uuid, TravailC5), EntDbError> {
        let etab = a.etablissement_id;
        let travail: Uuid = sqlx::query_scalar("SELECT travail_id FROM ent_cdt_variante WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(variante_id)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        let t = self.travail_saisie_c7(a, travail).await?;
        Ok((variante_id, t))
    }

    /// CDT-266 : modifie une variante ; les élèves auxquels elle est attribuée sont prévenus **un par un** (aucun autre élève).
    pub async fn cdt_variante_modifier(&self, a: &Acteur<'_>, variante_id: Uuid, body: VarianteBody) -> Result<VarianteDto, EntDbError> {
        let (_, t) = self.variante_du_prof(a, variante_id).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (libelle, consigne, echeance) = self.variante_corps(&mut tx, etab, &body).await?;
        sqlx::query(
            "UPDATE ent_cdt_variante SET libelle_interne = $3, consigne = $4, duree_estimee_min = $5, echeance_le = $6, version = version + 1
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(variante_id)
        .bind(&libelle)
        .bind(&consigne)
        .bind(body.duree_estimee_min)
        .bind(echeance)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let concernes: Vec<Uuid> = sqlx::query_scalar("SELECT eleve_id FROM ent_cdt_variante_eleve WHERE etablissement_id = $1 AND variante_id = $2 AND retiree_le IS NULL")
            .bind(etab)
            .bind(variante_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        for e in concernes {
            publier_cdt(&mut tx, a, types::CAHIER_TRAVAIL_MODIFIE, t.id, Some(e), json!({ "travail_id": t.id })).await?;
        }
        let v: VarianteRow = sqlx::query_as(
            "SELECT id, travail_id, libelle_interne, consigne, duree_estimee_min, echeance_le, version FROM ent_cdt_variante WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(variante_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let dto = self.variante_dto(&mut tx, etab, v).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// CDT-266 : attribue la variante à des élèves nommés du public du travail (R-C25) ; une seule variante active par élève.
    pub async fn cdt_variante_attribuer(&self, a: &Acteur<'_>, variante_id: Uuid, body: AttributionBody) -> Result<VarianteDto, EntDbError> {
        let (_, t) = self.variante_du_prof(a, variante_id).await?;
        let etab = a.etablissement_id;
        if body.eleve_ids.is_empty() || body.eleve_ids.len() > MAX_ATTRIBUTION {
            return Err(invalide("de 1 à 60 élèves à la fois"));
        }
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        for e in &body.eleve_ids {
            if !eleve_dans_public(&mut tx, etab, &t, *e, jour).await? {
                return Err(invalide("un élève hors du public du travail ne peut pas recevoir de variante"));
            }
        }
        for e in &body.eleve_ids {
            // Une seule variante active par élève : l'ancienne attribution (autre variante du même travail) est retirée.
            sqlx::query(
                "UPDATE ent_cdt_variante_eleve ve SET retiree_le = now()
                   FROM ent_cdt_variante v
                  WHERE ve.etablissement_id = $1 AND ve.eleve_id = $2 AND ve.retiree_le IS NULL AND ve.variante_id <> $3
                    AND v.etablissement_id = ve.etablissement_id AND v.id = ve.variante_id AND v.travail_id = $4",
            )
            .bind(etab)
            .bind(e)
            .bind(variante_id)
            .bind(t.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO ent_cdt_variante_eleve (etablissement_id, variante_id, eleve_id, attribuee_par_user_id)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (variante_id, eleve_id) DO UPDATE SET retiree_le = NULL, attribuee_le = now(), attribuee_par_user_id = EXCLUDED.attribuee_par_user_id",
            )
            .bind(etab)
            .bind(variante_id)
            .bind(e)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            publier_cdt(&mut tx, a, types::CAHIER_VARIANTE_ATTRIBUEE, t.id, Some(*e), json!({ "travail_id": t.id })).await?;
        }
        let v: VarianteRow = sqlx::query_as(
            "SELECT id, travail_id, libelle_interne, consigne, duree_estimee_min, echeance_le, version FROM ent_cdt_variante WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(variante_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let dto = self.variante_dto(&mut tx, etab, v).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// CDT-266 : retire la variante à des élèves (ils reviennent à la version commune).
    pub async fn cdt_variante_retirer(&self, a: &Acteur<'_>, variante_id: Uuid, body: AttributionBody) -> Result<VarianteDto, EntDbError> {
        let (_, t) = self.variante_du_prof(a, variante_id).await?;
        let etab = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("UPDATE ent_cdt_variante_eleve SET retiree_le = now() WHERE etablissement_id = $1 AND variante_id = $2 AND eleve_id = ANY($3) AND retiree_le IS NULL")
            .bind(etab)
            .bind(variante_id)
            .bind(&body.eleve_ids)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        for e in &body.eleve_ids {
            publier_cdt(&mut tx, a, types::CAHIER_TRAVAIL_MODIFIE, t.id, Some(*e), json!({ "travail_id": t.id })).await?;
        }
        let v: VarianteRow = sqlx::query_as(
            "SELECT id, travail_id, libelle_interne, consigne, duree_estimee_min, echeance_le, version FROM ent_cdt_variante WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(variante_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let dto = self.variante_dto(&mut tx, etab, v).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // ───────────────────────────── aménagements appliqués ─────────────────────────────

    /// CDT-267 : les pastilles des élèves d'un cours (faits utiles seulement) pour le professeur en saisie.
    pub async fn cdt_pastilles_cours(&self, a: &Acteur<'_>, cours: Uuid) -> Result<PastillesCoursDto, EntDbError> {
        self.cdt_exiger_saisie(a, cours, None).await?;
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let eleves: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT e.id, e.nom, e.prenom FROM ent_public_cours_jour($1, $3, $2, NULL, NULL, false) p
               JOIN ent_eleve e ON e.etablissement_id = $1 AND e.id = p.eleve_id ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(cours)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = eleves.iter().map(|e| e.0).collect();
        let mut pastilles = self.cdt_amenagements_utiles(etab, &ids, jour).await?;
        Ok(PastillesCoursDto {
            cours_id: cours,
            eleves: eleves
                .into_iter()
                .map(|(eleve_id, nom, prenom)| ElevePastillesDto {
                    eleve_id,
                    nom,
                    prenom,
                    pastilles: pastilles.remove(&eleve_id).unwrap_or_default().iter().map(|p| p.pastille()).collect(),
                })
                .collect(),
        })
    }

    /// CDT-267 : « appliquer les aménagements » à un travail : l'échéance effective des élèves qui ont un **délai majoré** est
    /// majorée du pourcentage de leur fait utile appliqué au temps donné (de la publication à l'échéance), arrondi au quart
    /// d'heure supérieur, au plus quatorze jours. Sans `confirmer`, un aperçu : rien n'est écrit. Idempotent : un élève déjà au
    /// moins aussi bien servi n'est pas touché. Le motif codé `amenagement` reste interne.
    pub async fn cdt_amenagements_appliquer(&self, a: &Acteur<'_>, travail_id: Uuid, confirmer: bool) -> Result<ApplicationAmenagementsDto, EntDbError> {
        let t = self.travail_saisie_c7(a, travail_id).await?;
        let etab = a.etablissement_id;
        if t.etat != "publie" {
            return Err(conflit("les aménagements s'appliquent à un travail publié"));
        }
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let debut: DateTime<Utc> = t.visible_a.unwrap_or(t.echeance_le - Duration::days(1));
        let donne = (t.echeance_le - debut).max(Duration::zero());
        let eleves: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT e.id, e.nom, e.prenom FROM ent_public_cours_jour($1, $3, $2, NULL, NULL, false) p
               JOIN ent_eleve e ON e.etablissement_id = $1 AND e.id = p.eleve_id
              WHERE $4 = 'cours' OR EXISTS (SELECT 1 FROM ent_cdt_travail_cible k WHERE k.etablissement_id = $1 AND k.travail_id = $5 AND k.eleve_id = e.id)
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(t.cours_id)
        .bind(jour)
        .bind(&t.public)
        .bind(t.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = eleves.iter().map(|e| e.0).collect();
        drop(tx);
        let amenagements = self.cdt_amenagements_utiles(etab, &ids, jour).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut sortie = Vec::new();
        for (eleve_id, nom, prenom) in eleves {
            let Some(pct) = amenagements.get(&eleve_id).and_then(|l| l.iter().find(|x| x.code == "temps_majore")).and_then(|x| x.valeur_pct) else {
                continue;
            };
            // Arrondi au pourcent puis au quart d'heure supérieurs (durées positives : pas de `div_ceil`, instable sur i64).
            let extra_min = (donne.num_minutes() * i64::from(pct) + 99) / 100;
            let extra_min = (extra_min + 14) / 15 * 15;
            let extra = Duration::minutes(extra_min).min(Duration::days(MAJORATION_MAX_JOURS));
            let nouvelle = t.echeance_le + extra;
            let (actuelle, _, _) = echeances_de(&mut tx, etab, &t, eleve_id).await?;
            if actuelle >= nouvelle {
                continue;
            }
            if confirmer {
                sqlx::query(
                    "INSERT INTO ent_cdt_echeance_eleve (id, etablissement_id, travail_id, eleve_id, echeance_le, motif_code, par_user_id)
                     VALUES ($1, $2, $3, $4, $5, 'amenagement', $6)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(t.id)
                .bind(eleve_id)
                .bind(nouvelle)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                Self::recalculer_retard(&mut tx, etab, &t, eleve_id).await?;
            }
            let (j, h) = jour_heure_locaux(&mut tx, etab, nouvelle).await?;
            sortie.push(EcheanceMajoreeDto { eleve_id, nom, prenom, echeance_le: nouvelle, echeance_jour: j, echeance_heure: h.format("%H:%M").to_string() });
        }
        if confirmer && !sortie.is_empty() {
            publier_cdt(&mut tx, a, types::CAHIER_DELAI_ACCORDE, t.id, None, json!({ "travail_id": t.id, "nb": sortie.len() })).await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(ApplicationAmenagementsDto { appliques: if confirmer { sortie.len() as i32 } else { 0 }, eleves: sortie, applique: confirmer })
    }

    // ───────────────────────────── AESH ─────────────────────────────

    /// Élèves accompagnés **à la date** par ce compte (port du socle 35 : l'affectation d'AESH, `ent_aesh_affectation`).
    async fn eleves_accompagnes(&self, etab: Uuid, user: Uuid, jour: NaiveDate) -> Result<Vec<EleveAccompagneDto>, EntDbError> {
        let lignes: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT DISTINCT e.id, e.nom, e.prenom FROM ent_aesh_affectation f
               JOIN ent_eleve e ON e.etablissement_id = f.etablissement_id AND e.id = f.eleve_id
              WHERE f.etablissement_id = $1 AND f.aesh_user_id = $2 AND f.date_debut <= $3 AND (f.date_fin IS NULL OR f.date_fin >= $3)
                AND (e.date_sortie IS NULL OR e.date_sortie > $3)
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(user)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(lignes.into_iter().map(|(eleve_id, nom, prenom)| EleveAccompagneDto { eleve_id, nom, prenom }).collect())
    }

    async fn exiger_accompagne(&self, a: &Acteur<'_>, eleve: Uuid) -> Result<(), EntDbError> {
        let jour = self.jour_etab(a.etablissement_id).await?;
        if self.eleves_accompagnes(a.etablissement_id, a.user_id, jour).await?.iter().any(|e| e.eleve_id == eleve) {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// CDT-284 : les élèves accompagnés aujourd'hui par l'AESH connecté.
    pub async fn cdt_aesh_eleves(&self, a: &Acteur<'_>) -> Result<Vec<EleveAccompagneDto>, EntDbError> {
        let jour = self.jour_etab(a.etablissement_id).await?;
        self.eleves_accompagnes(a.etablissement_id, a.user_id, jour).await
    }

    /// L'agenda d'un élève accompagné (avec **sa** version des travaux : variantes comprises). Ni coche, ni copies, ni retours.
    pub async fn cdt_aesh_agenda(&self, a: &Acteur<'_>, eleve: Uuid, du: NaiveDate, au: NaiveDate) -> Result<AgendaDto, EntDbError> {
        if au < du || (au - du).num_days() > MAX_PERIODE_JOURS {
            return Err(invalide("période invalide (100 jours au plus)"));
        }
        self.exiger_accompagne(a, eleve).await?;
        let etab = a.etablissement_id;
        self.cdt_projeter_echus(etab).await?;
        let aujourdhui = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = lignes_agenda(&mut tx, etab, eleve, du, au, aujourdhui, false).await?;
        Ok(AgendaDto { eleve_id: eleve, du, au, vue: "aesh".into(), lignes })
    }

    /// Le cahier d'un cours de l'élève accompagné : entrées publiées. Documents : par les liens courts ordinaires (C2).
    pub async fn cdt_aesh_cahier(&self, a: &Acteur<'_>, eleve: Uuid, cours: Uuid) -> Result<Vec<EntreeRemplacementDto>, EntDbError> {
        self.exiger_accompagne(a, eleve).await?;
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (dans,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $3, $2, NULL, $4, false))")
            .bind(etab)
            .bind(cours)
            .bind(jour)
            .bind(eleve)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if !dans {
            return Err(introuvable());
        }
        let lignes: Vec<(NaiveDate, chrono::NaiveTime, Uuid, Option<String>, String)> = sqlx::query_as(
            "SELECT s.jour, s.heure_debut, s.id, e.titre, e.contenu FROM ent_cdt_entree e
               JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
              WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.etat = 'publiee' AND e.visible_a <= now()
              ORDER BY s.jour DESC, s.heure_debut DESC LIMIT 60",
        )
        .bind(etab)
        .bind(cours)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(|(j, h, sid, titre, contenu)| EntreeRemplacementDto {
                seance: SeanceReportDto { seance_id: sid, jour: j, heure_debut: h.format("%H:%M").to_string() },
                titre,
                contenu,
            })
            .collect())
    }

    /// CDT-284 : l'AESH ajoute une note d'accompagnement sur un travail ou une entrée qui concerne son élève.
    pub async fn cdt_note_accompagnement_ajouter(&self, a: &Acteur<'_>, body: NoteAccompagnementBody) -> Result<NoteAccompagnementDto, EntDbError> {
        let etab = a.etablissement_id;
        let texte = texte_borne(&body.texte, 2000, true, "note vide ou trop longue (2 000 caractères au plus)")?;
        if body.travail_id.is_some() == body.entree_id.is_some() {
            return Err(invalide("indiquez un travail ou une entrée, pas les deux"));
        }
        self.exiger_accompagne(a, body.eleve_id).await?;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let agent: Option<Uuid> = sqlx::query_scalar("SELECT ent_cdt_agent_du_compte($1, $2)")
            .bind(etab)
            .bind(a.user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let agent = agent.ok_or_else(|| conflit("ce compte n'est rattaché à aucun agent de l'établissement"))?;
        // La cible concerne l'élève : travail publié de son public, ou entrée publiée de l'un de ses cours.
        let concerne: bool = match (body.travail_id, body.entree_id) {
            (Some(t), _) => {
                let travail = charger_travail(&mut tx, etab, t).await?;
                travail.etat == "publie" && eleve_dans_public(&mut tx, etab, &travail, body.eleve_id, jour).await?
            }
            (_, Some(e)) => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_cdt_entree en WHERE en.etablissement_id = $1 AND en.id = $2 AND en.etat = 'publiee'
                                  AND EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $4, en.cours_id, NULL, $3, false)))",
            )
            .bind(etab)
            .bind(e)
            .bind(body.eleve_id)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?,
            _ => false,
        };
        if !concerne {
            return Err(introuvable());
        }
        let id = Uuid::new_v4();
        let cree_le: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO ent_cdt_note_accompagnement (id, etablissement_id, entree_id, travail_id, eleve_id, agent_aesh_id, texte)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING cree_le",
        )
        .bind(id)
        .bind(etab)
        .bind(body.entree_id)
        .bind(body.travail_id)
        .bind(body.eleve_id)
        .bind(agent)
        .bind(&texte)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let (nom, prenom): (String, String) = sqlx::query_as("SELECT nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(body.eleve_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let auteur: String = sqlx::query_scalar("SELECT ent_cdt_nom_affichage($1, $2)").bind(etab).bind(a.user_id).fetch_one(&mut *tx).await.map_err(db)?;
        publier_cdt(&mut tx, a, types::CAHIER_NOTE_ACCOMPAGNEMENT, body.travail_id.or(body.entree_id).unwrap_or(id), None, json!({ "note_id": id })).await?;
        tx.commit().await.map_err(db)?;
        Ok(NoteAccompagnementDto { id, eleve_id: body.eleve_id, nom, prenom, texte, auteur, cree_le })
    }

    /// Les notes d'accompagnement d'un travail ou d'une entrée : professeurs du cours (saisie) et PP des élèves concernés.
    pub async fn cdt_notes_accompagnement_lire(&self, a: &Acteur<'_>, travail: Option<Uuid>, entree: Option<Uuid>) -> Result<Vec<NoteAccompagnementDto>, EntDbError> {
        let etab = a.etablissement_id;
        if travail.is_some() == entree.is_some() {
            return Err(invalide("indiquez un travail ou une entrée"));
        }
        let cours: Uuid = match (travail, entree) {
            (Some(t), _) => sqlx::query_scalar("SELECT cours_id FROM ent_cdt_travail WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(t),
            (_, Some(e)) => sqlx::query_scalar("SELECT cours_id FROM ent_cdt_entree WHERE etablissement_id = $1 AND id = $2").bind(etab).bind(e),
            _ => return Err(invalide("indiquez un travail ou une entrée")),
        }
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let jour = self.jour_etab(etab).await?;
        let en_saisie = self.cdt_exiger_saisie(a, cours, None).await.is_ok();
        let lignes: Vec<(Uuid, Uuid, String, String, String, DateTime<Utc>, Uuid)> = sqlx::query_as(
            "SELECT n.id, e.id, e.nom, e.prenom, n.texte, n.cree_le, n.agent_aesh_id FROM ent_cdt_note_accompagnement n
               JOIN ent_eleve e ON e.etablissement_id = n.etablissement_id AND e.id = n.eleve_id
              WHERE n.etablissement_id = $1 AND (($2::uuid IS NOT NULL AND n.travail_id = $2) OR ($3::uuid IS NOT NULL AND n.entree_id = $3))
              ORDER BY n.cree_le, n.id",
        )
        .bind(etab)
        .bind(travail)
        .bind(entree)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        for (id, eleve_id, nom, prenom, texte, cree_le, agent) in lignes {
            let lisible = en_saisie || self.est_pp_de(a, eleve_id, jour).await?;
            if !lisible {
                continue;
            }
            let auteur: String = sqlx::query_scalar(
                "SELECT COALESCE(NULLIF(btrim(coalesce(pe.prenom, '') || ' ' || coalesce(pe.nom, '')), ''), 'Accompagnant') FROM ent_agent ag
                   JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id WHERE ag.etablissement_id = $1 AND ag.id = $2",
            )
            .bind(etab)
            .bind(agent)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?
            .unwrap_or_else(|| "Accompagnant".into());
            sortie.push(NoteAccompagnementDto { id, eleve_id, nom, prenom, texte, auteur, cree_le });
        }
        if sortie.is_empty() && !en_saisie && !matches!(Role::parse(a.role), Some(Role::Prof)) {
            return Err(EntDbError::Forbidden);
        }
        Ok(sortie)
    }

    /// Le compte est-il professeur principal de la classe de cet élève ?
    async fn est_pp_de(&self, a: &Acteur<'_>, eleve: Uuid, jour: NaiveDate) -> Result<bool, EntDbError> {
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement s JOIN ent_affectation af ON af.etablissement_id = s.etablissement_id AND af.classe_id = s.classe_id
              WHERE s.etablissement_id = $1 AND s.professeur_principal AND af.eleve_id = $2 AND s.prof_user_id = $3
                AND af.date_debut <= $4 AND (af.date_fin IS NULL OR af.date_fin >= $4))",
        )
        .bind(a.etablissement_id)
        .bind(eleve)
        .bind(a.user_id)
        .bind(jour)
        .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)
    }

    // ───────────────────────────── réglages d'affichage ─────────────────────────────

    /// CDT-?? : les réglages d'affichage de l'élève (lui seul ; jamais un adulte).
    pub async fn cdt_affichage_lire(&self, a: &Acteur<'_>) -> Result<AffichageDto, EntDbError> {
        if Role::parse(a.role) != Some(Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let eleve = self.eleve_du_compte(a).await?.ok_or(EntDbError::Forbidden)?;
        let r: Option<(String, String, bool, bool)> = sqlx::query_as(
            "SELECT taille_police, interligne, lecture_audio, animations_reduites FROM ent_cdt_affichage WHERE etablissement_id = $1 AND eleve_id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(r.map(|(taille_police, interligne, lecture_audio, animations_reduites)| AffichageDto { taille_police, interligne, lecture_audio, animations_reduites })
            .unwrap_or_default())
    }

    /// Règle l'affichage de l'élève : sans intervention du professeur, sans trace pour un adulte.
    pub async fn cdt_affichage_regler(&self, a: &Acteur<'_>, body: AffichageDto) -> Result<AffichageDto, EntDbError> {
        if Role::parse(a.role) != Some(Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        if !TAILLES_POLICE.contains(&body.taille_police.as_str()) || !INTERLIGNES.contains(&body.interligne.as_str()) {
            return Err(invalide("taille de police ou interligne inconnu"));
        }
        let etab = a.etablissement_id;
        let eleve = self.eleve_du_compte(a).await?.ok_or(EntDbError::Forbidden)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_cdt_affichage (etablissement_id, eleve_id, taille_police, interligne, lecture_audio, animations_reduites)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (eleve_id) DO UPDATE SET taille_police = EXCLUDED.taille_police, interligne = EXCLUDED.interligne,
                lecture_audio = EXCLUDED.lecture_audio, animations_reduites = EXCLUDED.animations_reduites, modifie_le = now()",
        )
        .bind(etab)
        .bind(eleve)
        .bind(&body.taille_police)
        .bind(&body.interligne)
        .bind(body.lecture_audio)
        .bind(body.animations_reduites)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(body)
    }
}
