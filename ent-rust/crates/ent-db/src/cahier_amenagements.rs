//! Socle 20 : **port « aménagements utiles »** du socle 35 (`20_CAHIER_DE_TEXTES.md` §13.1), **en simulation** tant que le
//! socle 35 n'existe pas. Un seul point à remplacer : `cdt_amenagements_utiles`.
//!
//! Le cahier ne connaît que des **faits utiles** d'un catalogue fermé (délai majoré en %, version allégée, police agrandie,
//! document audio, lecture de consigne, usage de l'ordinateur, « ne pas copier au tableau ») : jamais un diagnostic, jamais un
//! intitulé de plan (PAP, PPS), jamais un motif. Ce qui sort d'ici est un code et, pour le délai majoré, un pourcentage.

use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::FromRow;
use uuid::Uuid;

use crate::ref_commun::db;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Catalogue fermé des faits utiles.
pub const CODES_AMENAGEMENT: [&str; 7] = [
    "temps_majore",
    "version_allegee",
    "police_agrandie",
    "document_audio",
    "lecture_consigne",
    "ordinateur",
    "pas_de_copie_tableau",
];

/// Un fait d'aménagement utile, applicable à une date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmenagementUtile {
    pub code: String,
    /// Pourcentage de temps en plus (`temps_majore` seulement).
    pub valeur_pct: Option<i16>,
}

impl AmenagementUtile {
    /// Libellé neutre d'une pastille : un fait, jamais un diagnostic.
    pub fn pastille(&self) -> String {
        match self.code.as_str() {
            "temps_majore" => match self.valeur_pct {
                Some(p) => format!("Temps majoré (+{p} %)"),
                None => "Temps majoré".to_string(),
            },
            "version_allegee" => "Version allégée".to_string(),
            "police_agrandie" => "Police agrandie".to_string(),
            "document_audio" => "Document audio".to_string(),
            "lecture_consigne" => "Lecture de la consigne".to_string(),
            "ordinateur" => "Ordinateur autorisé".to_string(),
            "pas_de_copie_tableau" => "Ne pas copier au tableau".to_string(),
            _ => "Aménagement".to_string(),
        }
    }
}

#[derive(Debug, FromRow)]
struct Ligne {
    eleve_id: Uuid,
    code: String,
    valeur_pct: Option<i16>,
}

impl PgEnt {
    /// Faits d'aménagement utiles de ces élèves **à la date donnée** (une requête, jamais une par élève). Port du socle 35
    /// (`accompagnement::amenagements_utiles`) : ici lu dans la table de simulation.
    pub(crate) async fn cdt_amenagements_utiles(
        &self,
        etab: Uuid,
        eleves: &[Uuid],
        jour: NaiveDate,
    ) -> Result<HashMap<Uuid, Vec<AmenagementUtile>>, EntDbError> {
        if eleves.is_empty() {
            return Ok(HashMap::new());
        }
        let lignes: Vec<Ligne> = sqlx::query_as(
            "SELECT eleve_id, code, valeur_pct FROM ent_cdt_amenagement_simule
              WHERE etablissement_id = $1 AND eleve_id = ANY($2) AND du <= $3 AND (au IS NULL OR au >= $3)
              ORDER BY eleve_id, code",
        )
        .bind(etab)
        .bind(eleves)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let mut sortie: HashMap<Uuid, Vec<AmenagementUtile>> = HashMap::new();
        for l in lignes {
            sortie.entry(l.eleve_id).or_default().push(AmenagementUtile { code: l.code, valeur_pct: l.valeur_pct });
        }
        Ok(sortie)
    }
}
