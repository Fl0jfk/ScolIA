//! Absences des personnels (socle 15, lots A11a à A11f) : déclarer et « mes absences » (agent), files de
//! traitement et fiche d'une absence (valideur, instructeur), « ce matin » avec propositions de
//! remplacement, « mes remplacements » (réponses du remplaçant), heures et états mensuels (A11d), remplacement de
//! courte durée (A11e) et paramétrage (A11f).
//!
//! Routes d'`ent-api` : `/personnels/absences/*` (réutilisées, aucune réécriture). Le serveur décide
//! de tout : droits §23, circuit, confidentialité (un professeur ne lit jamais un motif), traitements
//! permis, chevauchements. L'écran propose ce que la personne peut tenter et affiche les refus.
//!
//! Limites assumées (voir `docs/socles/15_ABSENCES_A11_ABC.md`) : les instants des absences sont
//! affichés en heure de Paris (le DTO ne donne pas le jour de l'établissement) ; le « travail prévu »
//! (ABP-118) renvoie au cahier de textes, sans écriture ici.

pub mod modele;
pub mod modele_heures;
pub mod modele_parametrage;
pub mod modele_rcd;

mod ce_matin;
mod commun;
mod declarer;
mod etats_heures;
mod fiche;
mod fiche_pieces;
mod fichiers;
mod files;
mod heures;
mod mes_absences;
mod paie;
mod parametrage;
mod rcd;
mod remplacements;
mod signaler;

pub use ce_matin::PageCeMatin;
pub use declarer::PageDeclarerAbsence;
pub use etats_heures::PageEtatsHeures;
pub use fiche::PageFicheAbsence;
pub use files::PageFilesAbsences;
pub use heures::PageMesHeures;
pub use mes_absences::PageMesAbsences;
pub use parametrage::PageParametrageAbsences;
pub use rcd::PageRcd;
pub use remplacements::PageMesRemplacements;
pub use signaler::PageSignalerAbsence;
