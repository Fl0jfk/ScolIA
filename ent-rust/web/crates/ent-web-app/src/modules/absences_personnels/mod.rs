//! Absences des personnels (socle 15, lots A11a à A11c) : déclarer et « mes absences » (agent), files de
//! traitement et fiche d'une absence (valideur, instructeur), « ce matin » avec propositions de
//! remplacement, et « mes remplacements » (réponses du remplaçant).
//!
//! Routes d'`ent-api` : `/personnels/absences/*` (réutilisées, aucune réécriture). Le serveur décide
//! de tout : droits §23, circuit, confidentialité (un professeur ne lit jamais un motif), traitements
//! permis, chevauchements. L'écran propose ce que la personne peut tenter et affiche les refus.
//!
//! Limites assumées (voir `docs/socles/15_ABSENCES_A11_ABC.md`) : les instants des absences sont
//! affichés en heure de Paris (le DTO ne donne pas le jour de l'établissement) ; le « travail prévu »
//! (ABP-118) renvoie au cahier de textes, sans écriture ici.

pub mod modele;

mod ce_matin;
mod commun;
mod declarer;
mod fiche;
mod fiche_pieces;
mod fichiers;
mod files;
mod mes_absences;
mod remplacements;
mod signaler;

pub use ce_matin::PageCeMatin;
pub use declarer::PageDeclarerAbsence;
pub use fiche::PageFicheAbsence;
pub use files::PageFilesAbsences;
pub use mes_absences::PageMesAbsences;
pub use remplacements::PageMesRemplacements;
pub use signaler::PageSignalerAbsence;
