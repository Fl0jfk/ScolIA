//! Socle 8 — brique « notifications » (lots N1 à N3), en module de `ent-db` (voir `contrat.rs`).
//!
//! - `contrat` : `NatureAvisDecl`, `AvisSurEvenement`, types du contrat.
//! - `catalogue` : natures déclarées et émissions par événement.
//! - `assemblage` : vérifications et lint de minimisation.
//! - `destinataires` : qui prévenir (adaptateur provisoire N1, personnes en N2).
//! - `service` : `aviser`, `annuler_avis`, `etat`.
//! - `consommateur` : `notifications.avis`.
//! - `expedition` : planificateur et remise des avis échus (ENT, ancien moteur, personnes sans compte).
//! - `liens` : liens nominatifs (jeton haché, expirant, à ouvertures limitées).

pub mod assemblage;
pub mod catalogue;
pub mod consommateur;
pub mod contrat;
pub mod destinataires;
pub mod expedition;
pub mod liens;
pub mod reglages;
pub mod temps;
pub mod service;
