//! Écrans « Élèves » (socle 2, lots E9a et E9b) : la liste et la recherche, puis la fiche hub d'un
//! élève (synthèse, tuiles et pastilles d'E8, identité, historique).
//!
//! Routes d'`ent-api` : `GET /eleves/liste`, `GET /eleves/pastilles`, `GET /eleves/{id}/synthese`,
//! `GET /eleves/{id}`, `GET /eleves/{id}/identite/historique`, `GET /eleves/{id}/identifiants`,
//! `GET /eleves/{id}/consultations`, `GET /eleves/{id}/photo`. **Le serveur décide de tout** : droits,
//! masquage par groupe de champs, périmètre, élève protégé (absent, jamais « refusé »). L'écran ne
//! calcule aucun droit : il affiche ce qui arrive, dit pourquoi une information manque, et n'ajoute
//! jamais un lien vers un écran qui n'existe pas.

mod fiche;
mod liste;
pub mod modele;

pub use fiche::PageFicheHub;
pub use liste::PageListeEleves;
