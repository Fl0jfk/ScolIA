//! Socle 21 (évaluations) : **port minimal `evaluations::calendrier`**, lecture seule, sans note ni résultat.
//!
//! Le socle 20 (agenda de l'élève, charge de travail) lit les évaluations annoncées d'élèves sur une période. La brique
//! `evaluations` V1 (`21_EVALUATIONS.md` §6, `EVA-117`) n'existe pas encore sur `ENT` : cette lecture minimale s'appuie sur
//! les devoirs de l'existant (`ent_devoir`, lot S2-3) dont la date tombe dans la période. Elle ne sort que le jour, la
//! matière et le titre ; jamais une note, une moyenne, un coefficient ni une statistique. Quand la brique V1 sera livrée,
//! seul le corps de [`PgEnt::eva_calendrier`] change : la signature et les appelants restent.
//!
//! Appartenance : l'élève est dans le public du cours du service du devoir **au jour de l'établissement** (`ent_public_cours_jour`) ;
//! un devoir sans service (existant historique) vise sa classe ou son groupe, par l'affectation en cours.

use chrono::NaiveDate;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::db;
use crate::service::{EntDbError, PgEnt};

/// Une évaluation annoncée (sans résultat).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationPrevue {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub matiere: String,
    pub titre: String,
    /// Cours du service du devoir, quand il existe.
    pub cours_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
}

/// Une évaluation prévue, pour un élève donné.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationEleve {
    pub eleve_id: Uuid,
    pub evaluation: EvaluationPrevue,
}

#[derive(Debug, FromRow)]
struct LigneRow {
    eleve_id: Uuid,
    id: Uuid,
    jour: NaiveDate,
    matiere: String,
    titre: String,
    cours_id: Option<Uuid>,
    classe_id: Option<Uuid>,
}

impl PgEnt {
    /// `evaluations::calendrier(eleves, du, au)` : évaluations annoncées de ces élèves, triées par élève puis jour.
    /// `aujourdhui` : jour de l'établissement, qui date l'appartenance au public.
    pub(crate) async fn eva_calendrier(
        tx: &mut PgConnection,
        etab: Uuid,
        eleves: &[Uuid],
        du: NaiveDate,
        au: NaiveDate,
        aujourdhui: NaiveDate,
    ) -> Result<Vec<EvaluationEleve>, EntDbError> {
        if eleves.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<LigneRow> = sqlx::query_as(
            "SELECT e.eleve_id, d.id, d.date_devoir AS jour, COALESCE(m.libelle, 'Évaluation') AS matiere, d.titre,
                    s.cours_id, d.classe_id
               FROM unnest($2::uuid[]) AS e(eleve_id)
               JOIN ent_devoir d ON d.etablissement_id = $1 AND d.date_devoir BETWEEN $3 AND $4
               LEFT JOIN ent_matiere m ON m.etablissement_id = d.etablissement_id AND m.id = d.matiere_id
               LEFT JOIN ent_service_enseignement s ON s.etablissement_id = d.etablissement_id AND s.id = d.service_id
              WHERE (d.service_id IS NOT NULL AND s.cours_id IS NOT NULL
                     AND EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $5, s.cours_id, NULL, e.eleve_id, false)))
                 OR (d.service_id IS NULL
                     AND EXISTS (SELECT 1 FROM ent_affectation a
                                  WHERE a.etablissement_id = $1 AND a.eleve_id = e.eleve_id
                                    AND ((a.classe_id IS NOT NULL AND a.classe_id = d.classe_id)
                                      OR (a.groupe_id IS NOT NULL AND a.groupe_id = d.groupe_id))
                                    AND a.date_debut <= $5 AND (a.date_fin IS NULL OR a.date_fin >= $5)))
              ORDER BY e.eleve_id, d.date_devoir, d.titre, d.id",
        )
        .bind(etab)
        .bind(eleves)
        .bind(du)
        .bind(au)
        .bind(aujourdhui)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| EvaluationEleve {
                eleve_id: r.eleve_id,
                evaluation: EvaluationPrevue {
                    id: r.id,
                    jour: r.jour,
                    matiere: r.matiere,
                    titre: r.titre,
                    cours_id: r.cours_id,
                    classe_id: r.classe_id,
                },
            })
            .collect())
    }
}
