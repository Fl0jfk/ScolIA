//! Répercussions Brain = écritures **directes** sur les tables canoniques.
//!
//! Règle produit :
//! - Pas de copie de `classe` / `status` (déjà sur `ent_eleve`).
//! - Ex. séjour scolaire → une ligne dans la table **absence** liée à `eleve_id`.
//! - Les écrans (notes, VS, famille…) **lisent** ces tables ; ils ne recopient pas.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Écriture additionnelle (quand la commande socle ne suffit pas).
/// Variantes ajoutées au fil des gestes réels — jamais un « notify silo ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrainAction {
    /// Écriture directe dans `ent_absence` (motif = sejour, maladie, …).
    EnsureAbsence {
        etablissement_id: Uuid,
        eleve_id: Uuid,
        motif: String,
        debut: String,
        fin: String,
    },
}
