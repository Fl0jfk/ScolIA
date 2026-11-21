//! Cahier de textes, travail à faire / à rendre, copies et agenda (lot S2-6, D7).
//!
//! Routes d'`ent-api` : `/cahier/*` (voir `ent-web-api/src/cahier.rs`). Les droits (portée « mes
//! classes », élève sur lui-même, responsable selon le lien), la visibilité par classe ou par
//! groupe, le retard et l'alerte de surcharge sont décidés par le serveur ; l'écran s'adapte
//! aux refus. Trois écrans :
//! * `/web/cahier` — professeur : contenu des séances, travail à donner, charge de la classe ;
//! * `/web/cahier/travaux/:id` — professeur : copies reçues et retour à l'élève ;
//! * `/web/agenda` — élève et famille : travail à faire, échéances, rendu en ligne.
//!
//! Lot C2 (saisie et publication) :
//! * `/web/cahier/cours` — professeur : cahier du cours (entrées par séance, brouillon, programmation, plage calme,
//!   modification avec ou sans avis, historique, retrait, copie) ;
//! * `/web/cahier/travail` — professeur : donner un travail (un ou plusieurs cours, échéance-séance).
//!
//! Lot C5 (rendu et retour) :
//! * `/web/cahier/rendu/:id` — élève et famille : situation d'un travail à rendre, dépôt, accusé, retour, demande de délai ;
//! * `/web/cahier/suivi/:id` — professeur : tableau de suivi, constats, délais, clôture, relance, retours.
//!
//! Lot C6 (continuité) : `/web/cahier/continuite` — reports proposés, élèves en continuité.
//! Lot C7 (adaptations) : `/web/cahier/adaptations/:id` — variantes et aménagements d'un travail ; `/web/cahier/aesh` — AESH.
//!
//! Lot C8 (visa, inspection, archive) : `/web/cahier/visa` — tenue et visa ; `/web/cahier/chronologie` — cahier du cours en lecture ;
//! `/web/cahier/inspection` — accès d'inspection et journal ; `/web/cahier/inspection/lecture` — lecture de l'inspecteur ;
//! `/web/cahier/archives` — clôture, archives, exports.
//! Lot C9 (progressions) : `/web/cahier/progressions`.
//!
//! Lot C3 (agenda, cahier de l'élève et de la famille) :
//! * `/web/cahier/agenda` — élève et famille : agenda de la semaine, « ce que j'ai manqué », la semaine, calendrier.

mod adaptations;
mod aesh;
mod agenda;
mod archives;
mod bandeau_charge;
mod charge_classe;
mod chronologie;
mod continuite;
mod copies;
mod cours;
pub mod fichiers;
mod inspection;
pub mod modele;
pub mod modele_c2;
pub mod modele_c3;
pub mod modele_c4;
pub mod modele_c5;
pub mod modele_c6;
pub mod modele_c7;
pub mod modele_c8;
pub mod modele_c9;
mod mon_cahier;
mod prof;
mod progressions;
mod rendu;
mod suivi_rendus;
mod tenue_visa;
mod travail;

pub use adaptations::{PageAdaptations, ADRESSE_ADAPTATIONS};
pub use aesh::{PageAesh, ADRESSE_AESH};
pub use agenda::PageAgenda;
pub use archives::{PageArchives, ADRESSE_ARCHIVES};
pub use charge_classe::{PageChargeClasses, ADRESSE_CHARGE};
pub use chronologie::{PageChronologie, ADRESSE_CHRONOLOGIE};
pub use continuite::{PageContinuite, ADRESSE_CONTINUITE};
pub use copies::PageCopies;
pub use cours::PageCahierCours;
pub use inspection::{
    PageInspection, PageLectureInspection, ADRESSE_INSPECTION, ADRESSE_LECTURE_INSPECTION,
};
pub use mon_cahier::{PageMonCahier, ADRESSE_MON_CAHIER};
pub use prof::PageCahier;
pub use progressions::{PageProgressions, ADRESSE_PROGRESSIONS};
pub use rendu::{PageRendu, ADRESSE_RENDU};
pub use suivi_rendus::{PageSuiviRendus, ADRESSE_SUIVI};
pub use tenue_visa::{PageTenueVisa, ADRESSE_VISA};
pub use travail::PageDonnerTravail;

use ent_web_api::ApiError;
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use crate::noyau::{reconnexion, session};

/// Traitement uniforme d'un échec d'écriture : session perdue → fenêtre de reconnexion
/// (la saisie est conservée) ; sinon message clair dans l'écran.
pub(crate) fn gestionnaire_echec(erreur: RwSignal<Option<String>>) -> Callback<ApiError> {
    Callback::new(move |e: ApiError| match e {
        ApiError::NonConnecte => {
            reconnexion::signaler_session_perdue();
            erreur.set(Some(reconnexion::MESSAGE_SESSION_EXPIREE.into()));
        }
        autre => erreur.set(Some(autre.message_utilisateur())),
    })
}

/// Demande un lien signé court pour la pièce puis l'ouvre (le client ne connaît jamais la
/// clé de stockage ; le lien expire en quelques dizaines de secondes).
pub(crate) fn ouvrir_piece(piece_id: Uuid, erreur: RwSignal<Option<String>>) {
    let echec = gestionnaire_echec(erreur);
    erreur.set(None);
    spawn_local(async move {
        let client = session::client();
        match client.cahier_lien_piece(piece_id).await {
            Ok(lien) => {
                if let Err(m) = fichiers::ouvrir_adresse(&client.adresse_lien(&lien)) {
                    erreur.set(Some(m));
                }
            }
            Err(e) => echec.run(e),
        }
    });
}
