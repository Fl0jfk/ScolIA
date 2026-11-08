//! Liste unique « mes enseignements » (D-022 bis) : classes entières et groupes d'un
//! professeur, avec matière et effectif, calculée par la **même règle** que
//! `eleves_autorises` (portée D-022). Vue sur le « cours » (`ent_service_enseignement`,
//! socle 5 S1) ; l'effectif vient du public du cours (`cours.rs`), membres individuels compris.

use chrono::NaiveDate;
use ent_auth::{Action, Role};
use ent_types::referentiel::EnseignementDto;
use uuid::Uuid;

use crate::evaluations::db;
use crate::service::{EntDbError, PgEnt};

type Ligne = (Uuid, Option<Uuid>, Option<Uuid>, Option<Uuid>, String, String, String, Option<Uuid>, bool);

impl PgEnt {
    /// Enseignements (services) de l'acteur pour l'année scolaire qui contient `jour`.
    /// Un acteur sans service obtient une liste vide ; jamais ceux d'un autre professeur.
    pub async fn mes_enseignements_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<EnseignementDto>, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        let rows: Vec<Ligne> = sqlx::query_as(
            "SELECT s.id, s.classe_id, s.groupe_id, COALESCE(s.classe_id, g.classe_id),
                    COALESCE(c.code, g.code, ''), COALESCE(cp.code, c.code, ''),
                    COALESCE(m.libelle, NULLIF(s.matiere_libelle, ''), '—'),
                    s.matiere_id, s.professeur_principal
               FROM ent_service_enseignement s
               LEFT JOIN ent_matiere m ON m.id = s.matiere_id AND m.etablissement_id = s.etablissement_id
               LEFT JOIN ent_classe c ON c.id = s.classe_id AND c.etablissement_id = s.etablissement_id
               LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
               LEFT JOIN ent_classe cp ON cp.id = g.classe_id AND cp.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1 AND s.prof_user_id = $2
                AND EXISTS (SELECT 1 FROM ent_annee_scolaire y
                             WHERE y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
                               AND y.date_debut <= $3 AND y.date_fin >= $3)
              ORDER BY 6, 5, 7",
        )
        .bind(etab)
        .bind(acteur)
        .bind(jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        // Même règle que partout : les élèves que l'acteur peut suivre (None = tous).
        let autorises = self
            .eleves_autorises(etab, acteur, role, Action::SuiviEleve)
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for (service_id, classe_id, groupe_id, classe_parente, cible_code, classe_code, matiere, matiere_id, pp) in
            rows
        {
            // Public du cours par la règle unique (membres individuels compris).
            let membres = self.public_cours(etab, jour, Some(service_id), None, None).await?;
            let effectif = membres
                .iter()
                .filter(|(_, id)| autorises.as_ref().is_none_or(|a| a.contains(id)))
                .count() as i64;
            let nature = if classe_id.is_some() { "classe" } else { "groupe" };
            let libelle = if classe_id.is_some() {
                format!("{matiere} · {cible_code}")
            } else {
                format!("{matiere} · {cible_code} (groupe)")
            };
            out.push(EnseignementDto {
                service_id,
                nature: nature.to_string(),
                classe_id: classe_parente,
                groupe_id,
                cible_code,
                classe_code,
                matiere_id,
                matiere_libelle: matiere,
                effectif,
                professeur_principal: pp,
                libelle,
            });
        }
        Ok(out)
    }
}
