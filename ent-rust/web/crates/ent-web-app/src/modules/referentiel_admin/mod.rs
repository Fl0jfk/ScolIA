//! Administration du référentiel scolaire (socle 2, lot S2-2) : années et bascule, sites,
//! niveaux et MEF, classes et groupes (ateliers, responsables), périodes, calendrier, grille
//! horaire, génération des séances.
//!
//! Routes d'`ent-api/src/referentiel_s2.rs`. Les droits (`referentiel.lire`, `referentiel.gerer`,
//! `roles.gerer` pour les responsables) sont décidés par le serveur ; l'écran affiche les actions
//! d'écriture selon les droits effectifs du moteur et traite les refus (403). Aucune règle métier
//! ici : `modele.rs` guide la saisie, le serveur reste l'autorité.

pub mod modele;

mod annees;
mod calendrier;
mod classes;
mod grille;
mod structure;

pub use annees::PageAnnees;
pub use calendrier::PageCalendrier;
pub use classes::PageClassesAdmin;
pub use grille::PageGrille;
pub use structure::PageStructure;

use ent_web_ui::{Annonce, EnTetePage, Icone, NomIcone};
use leptos::prelude::*;
use leptos_meta::Title;

use crate::noyau::coquille::use_droits_moteur;
pub(crate) use crate::noyau::ecriture::{message_echec, Ecriture, RetourEcriture};
use crate::noyau::navigation;

/// Sections de l'administration : (adresse, libellé).
pub const SECTIONS: [(&str, &str); 6] = [
    ("/web/referentiel", "Vue d'ensemble"),
    ("/web/referentiel/annees", "Années et bascule"),
    ("/web/referentiel/structure", "Sites, niveaux et MEF"),
    ("/web/referentiel/classes", "Classes et groupes"),
    ("/web/referentiel/calendrier", "Périodes et calendrier"),
    ("/web/referentiel/grille", "Grille horaire et séances"),
];

/// Droit d'écriture sur le référentiel (moteur) : masque les boutons d'écriture sinon. Le serveur
/// refuse de toute façon une écriture sans `referentiel.gerer`.
pub(crate) fn peut_gerer() -> Signal<bool> {
    let moteur = use_droits_moteur();
    Signal::derive(move || moteur.get().peut_gerer_referentiel())
}

/// En-tête commun et navigation entre les sections (lien courant annoncé par `aria-current`).
#[component]
pub(crate) fn EnTeteReferentiel(
    #[prop(into)] titre: String,
    #[prop(into)] sous_titre: String,
    #[prop(into)] courant: &'static str,
) -> impl IntoView {
    view! {
        <EnTetePage
            surtitre="Administration".to_string()
            titre=titre
            sous_titre=sous_titre
        />
        <nav class="ra-sections" aria-label="Sections du référentiel">
            <ul>
                {SECTIONS
                    .iter()
                    .map(|(href, libelle)| {
                        let actif = *href == courant;
                        view! {
                            <li>
                                <a
                                    href=*href
                                    class=if actif { "ra-sections__lien ra-sections__lien--actif" } else { "ra-sections__lien" }
                                    aria-current=actif.then_some("page")
                                >
                                    {*libelle}
                                </a>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </nav>
    }
}

/// Vue d'ensemble : accès aux sections et rappel des droits.
#[component]
pub fn PageReferentiel() -> impl IntoView {
    let gerer = peut_gerer();
    let annonce = RwSignal::new(String::new());
    view! {
        <Title text="Référentiel"/>
        <Annonce message=Signal::derive(move || annonce.get())/>
        <EnTeteReferentiel
            titre="Référentiel scolaire"
            sous_titre="Structure de l'établissement : années, sites, niveaux, classes, calendrier et grille horaire."
            courant=navigation::REFERENTIEL
        />
        <p class="texte-attenue" role="status">
            {move || {
                if gerer.get() {
                    "Vous pouvez consulter et modifier le référentiel."
                } else {
                    "Vous pouvez consulter le référentiel ; sa modification est réservée aux personnes habilitées."
                }
            }}
        </p>
        <ul class="ra-cartes">
            {SECTIONS
                .iter()
                .skip(1)
                .map(|(href, libelle)| {
                    view! {
                        <li class="ra-carte">
                            <Icone nom=NomIcone::Salle/>
                            <a href=*href>{*libelle}</a>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}
