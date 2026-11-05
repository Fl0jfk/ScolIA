//! Discipline et assiduité (socle 2, lot S2-4) : **punitions** (mesures éducatives), **sanctions**
//! (disciplinaires, lecture), **procédures disciplinaires**, et **signalements d'assiduité**.
//!
//! Routes d'`ent-api/src/vie_scolaire.rs`. Les droits (`punitions.donner`, `discipline.gerer`,
//! `assiduite.signaler`) sont décidés par le serveur ; l'écran ne propose les actions que selon
//! les droits effectifs du moteur et traite les refus (403).

pub mod modele;

mod punitions;
mod signalements;

pub use punitions::PageDiscipline;
pub use signalements::PageSignalements;
