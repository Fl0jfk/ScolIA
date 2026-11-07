//! Liste des groupes et ateliers d'une année avec leurs responsables (écran d'administration
//! du référentiel). Lecture seule, droit `referentiel.lire` (moteur D-017).

use ent_auth::Action;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::ref_commun::{db, Acteur};
use crate::service::{EntDbError, PgEnt};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponsableGroupe {
    pub user_id: Uuid,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupeAdmin {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    /// Nul pour un groupe transverse (atelier, option…).
    pub classe_id: Option<Uuid>,
    pub responsables: Vec<ResponsableGroupe>,
}

#[derive(FromRow)]
struct LigneGroupe {
    id: Uuid,
    annee_id: Uuid,
    code: String,
    libelle: String,
    classe_id: Option<Uuid>,
}

impl PgEnt {
    /// Groupes de l'année demandée (année courante par défaut) et leurs responsables.
    pub async fn ref_groupes(
        &self,
        a: &Acteur<'_>,
        annee_id: Option<Uuid>,
    ) -> Result<Vec<GroupeAdmin>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let lignes: Vec<LigneGroupe> = sqlx::query_as(
            "SELECT g.id, g.annee_id, g.code, g.libelle, g.classe_id
               FROM ent_groupe g
              WHERE g.etablissement_id = $1
                AND g.annee_id = COALESCE($2, (SELECT id FROM ent_annee_scolaire
                                                WHERE etablissement_id = $1 AND courante LIMIT 1))
              ORDER BY g.code",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
        let resp: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT r.groupe_id, r.user_id, COALESCE(u.display_name, u.email)
               FROM ent_groupe_responsable r
               JOIN ent_user u ON u.id = r.user_id
              WHERE r.etablissement_id = $1 AND r.groupe_id = ANY($2)
              ORDER BY 3",
        )
        .bind(a.etablissement_id)
        .bind(&ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(|l| GroupeAdmin {
                responsables: resp
                    .iter()
                    .filter(|(g, _, _)| *g == l.id)
                    .map(|(_, u, label)| ResponsableGroupe {
                        user_id: *u,
                        label: label.clone(),
                    })
                    .collect(),
                id: l.id,
                annee_id: l.annee_id,
                code: l.code,
                libelle: l.libelle,
                classe_id: l.classe_id,
            })
            .collect())
    }
}
