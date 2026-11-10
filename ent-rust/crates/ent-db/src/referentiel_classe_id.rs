//! Référentiel S2-2 : `classe_id` à côté du texte `classe` (V1) : rattachement idempotent
//! des données existantes et comparaison texte ↔ identifiant.
//!
//! **Tables concernées (9)** : `ent_eleve`, `ent_eleve_periode` (kind `classe`),
//! `ent_eleve_scolarite_annee`, `ent_appel`, `ent_edt_creneau`, `ent_devoir`,
//! `ent_cahier_texte`, `ent_conseil_classe`, `ent_annonce` (`cible_classe`).
//!
//! **Double écriture** : assurée par la base (triggers de la migration 111) quel que soit le
//! code appelant. **Rattachement** : fonction SQL `ent_rattacher_classes` (migration 112),
//! idempotente, exécutée à la migration et relançable ici.

use ent_auth::Action;
use serde::{Deserialize, Serialize};

use crate::ref_commun::{db, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// (table, colonne texte, filtre supplémentaire)
pub const TABLES_CLASSE: [(&str, &str, &str); 9] = [
    ("ent_eleve", "classe", ""),
    ("ent_eleve_periode", "valeur", "AND t.kind = 'classe'"),
    ("ent_eleve_scolarite_annee", "classe", ""),
    ("ent_appel", "classe", ""),
    ("ent_edt_creneau", "classe", ""),
    ("ent_devoir", "classe", ""),
    ("ent_cahier_texte", "classe", ""),
    ("ent_conseil_classe", "classe", ""),
    ("ent_annonce", "cible_classe", ""),
];

/// État de cohérence texte ↔ `classe_id` d'une table pour un établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparaisonClasse {
    pub table: String,
    pub avec_texte: i64,
    pub avec_classe_id: i64,
    /// Texte renseigné mais pas d'identifiant : reste à rattacher.
    pub sans_classe_id: i64,
    /// Identifiant dont le code diffère du texte.
    pub incoherentes: i64,
    /// Identifiant pointant une classe d'un autre établissement (ne doit jamais arriver).
    pub hors_etablissement: i64,
}

impl ComparaisonClasse {
    pub fn est_saine(&self) -> bool {
        self.sans_classe_id == 0 && self.incoherentes == 0 && self.hors_etablissement == 0
    }
}

impl PgEnt {
    /// Compare, table par table, le texte `classe` et `classe_id` (lecture seule).
    pub async fn ref_comparer_classe_id(
        &self,
        etablissement_id: uuid::Uuid,
    ) -> Result<Vec<ComparaisonClasse>, EntDbError> {
        let mut out = Vec::new();
        for (table, col, extra) in TABLES_CLASSE {
            let sql = format!(
                "SELECT
                   count(*) FILTER (WHERE btrim(coalesce(t.{col}, '')) <> ''),
                   count(*) FILTER (WHERE t.classe_id IS NOT NULL),
                   count(*) FILTER (WHERE btrim(coalesce(t.{col}, '')) <> '' AND t.classe_id IS NULL),
                   count(*) FILTER (WHERE t.classe_id IS NOT NULL AND c.code IS DISTINCT FROM btrim(t.{col})),
                   count(*) FILTER (WHERE t.classe_id IS NOT NULL
                                      AND c.etablissement_id IS DISTINCT FROM t.etablissement_id)
                 FROM {table} t LEFT JOIN ent_classe c ON c.id = t.classe_id
                WHERE t.etablissement_id = $1 {extra}"
            );
            let (avec_texte, avec_id, sans, incoh, hors): (i64, i64, i64, i64, i64) =
                sqlx::query_as(&sql)
                    .bind(etablissement_id)
                    .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
                    .await
                    .map_err(db)?;
            out.push(ComparaisonClasse {
                table: table.to_string(),
                avec_texte,
                avec_classe_id: avec_id,
                sans_classe_id: sans,
                incoherentes: incoh,
                hors_etablissement: hors,
            });
        }
        Ok(out)
    }

    /// Comparaison pour l'acteur (droit de lecture du référentiel).
    pub async fn ref_comparaison_classe_id(
        &self,
        a: &Acteur<'_>,
    ) -> Result<Vec<ComparaisonClasse>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        self.ref_comparer_classe_id(a.etablissement_id).await
    }

    /// Relance le rattachement (idempotent) pour l'établissement : renvoie, par table, le
    /// nombre de lignes rattachées (0 quand tout l'est déjà).
    pub async fn ref_rattacher_classes(
        &self,
        a: &Acteur<'_>,
    ) -> Result<Vec<(String, i64)>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        self.rattacher_classes_etab(a.etablissement_id).await
    }

    pub(crate) async fn rattacher_classes_etab(
        &self,
        etablissement_id: uuid::Uuid,
    ) -> Result<Vec<(String, i64)>, EntDbError> {
        sqlx::query_as("SELECT o_table, o_rattachees FROM ent_rattacher_classes($1)")
            .bind(etablissement_id)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
            .await
            .map_err(db)
    }
}
