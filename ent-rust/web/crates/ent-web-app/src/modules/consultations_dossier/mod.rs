//! **Consultations du dossier** (socle 27, lot G4-1 ; `27_RGPD_ARCHIVES.md` §11.2, §11.4, RGP-192).
//!
//! | Écran | Adresse | Qui |
//! |---|---|---|
//! | Qui a consulté le dossier | `/web/mes-donnees/consultations` | toute personne connectée (ses propres dossiers) |
//!
//! La famille voit, pour chaque enfant dont elle est titulaire, **qui** (fonction et service, jamais le nom), **quand**,
//! **quoi** (nature, section) et le motif d'une consultation hors périmètre, regroupés par jour, sur l'année
//! scolaire en cours et la précédente. L'élève majeur voit son dossier, l'agent le sien. L'écran ne calcule aucun droit
//! ni aucune exclusion (santé, conformité, support, protections : tout est filtré par le serveur, R-RG29, R-RG46).
//!
//! Le sélecteur de motif d'une consultation hors périmètre (RGP-191) est celui de la fiche élève (`MotifRequis`).

pub mod modele;

use ent_web_api::acces_traces_types::{ConsultationsDossierDto, MotifConsultationDto};
use ent_web_api::ApiError;
use ent_web_ui::{Alerte, Bouton, EnTetePage, EtatVide, Niveau, Squelette, Variante};
use leptos::prelude::*;
use leptos_meta::Title;

use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{a_paris, date_complete};

use modele::{qui, quoi, synthese, titre_dossier};

#[component]
pub fn PageConsultationsDossier() -> impl IntoView {
    let page = LocalResource::new(|| async { session::client().acces_consultations().await });
    view! {
        <Title text="Consultations du dossier"/>
        <EnTetePage
            titre="Consultations du dossier".to_string()
            sous_titre="Qui a consulté le dossier, quand et pourquoi : l'année scolaire en cours et la précédente. Seules la fonction et le service sont indiqués, jamais le nom.".to_string()
        />
        {move || match page.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des consultations…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| page.refetch())/>
            }.into_any(),
            Some(Ok(c)) => view! { <Dossiers consultations=c/> }.into_any(),
        }}
    }
}

#[component]
fn Dossiers(consultations: ConsultationsDossierDto) -> impl IntoView {
    if consultations.dossiers.is_empty() {
        return view! {
            <EtatVide titre="Aucun dossier à afficher"
                message="Cette page montre les consultations des dossiers dont vous êtes titulaire, ou de votre propre dossier."/>
        }
        .into_any();
    }
    view! {
        {consultations.dossiers.into_iter().map(|d| {
            let titre = titre_dossier(&d);
            let titre_aria = titre.clone();
            let phrase = synthese(d.total);
            view! {
                <section class="ui-section" aria-label=titre_aria>
                    <h2 class="ui-section__titre">{titre}</h2>
                    <p class="ui-liste__detail">{phrase}</p>
                    {d.jours.into_iter().map(|j| view! {
                        <h3 class="ui-section__sous-titre">{date_complete(j.jour)}</h3>
                        <ul class="ui-liste">
                            {j.lignes.into_iter().map(|l| {
                                let heure = a_paris(l.le).format("%H:%M").to_string();
                                view! {
                                    <li class="ui-liste__element">
                                        <div class="ui-liste__principal">
                                            <span class="ui-liste__titre">{qui(&l)}</span>
                                            <span class="ui-liste__detail">{quoi(&l)}</span>
                                        </div>
                                        <span class="ui-liste__meta">{heure}</span>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    }).collect_view()}
                </section>
            }
        }).collect_view()}
    }
    .into_any()
}

/// Reconnaît la réponse 409 « motif requis » à l'ouverture d'une fiche hors périmètre (RGP-191).
pub fn est_motif_requis(e: &ApiError) -> bool {
    matches!(e, ApiError::Conflit(m) if m.contains("motif_requis"))
}

/// Sélecteur de motif (liste S-542) affiché à la place de la fiche : l'agent choisit un motif, la fiche est alors
/// servie et la consultation tracée avec ce motif. Un seul geste.
#[component]
pub fn MotifRequis(
    /// Appelé avec le code du motif choisi.
    on_choisir: Callback<String>,
) -> impl IntoView {
    let motifs = LocalResource::new(|| async { session::client().acces_motifs().await });
    view! {
        <Alerte niveau=Niveau::Info titre="Cette fiche est hors de votre périmètre".to_string()>
            "Pour l'ouvrir, indiquez le motif de votre consultation. Il est conservé et visible de la famille."
        </Alerte>
        {move || match motifs.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des motifs…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| motifs.refetch())/>
            }.into_any(),
            Some(Ok(l)) => view! { <ListeMotifs motifs=l on_choisir=on_choisir/> }.into_any(),
        }}
    }
}

#[component]
fn ListeMotifs(motifs: Vec<MotifConsultationDto>, on_choisir: Callback<String>) -> impl IntoView {
    view! {
        <ul class="ui-liste">
            {motifs.into_iter().map(|m| {
                let code = m.code.clone();
                view! {
                    <li class="ui-liste__element">
                        <span class="ui-liste__titre">{m.libelle.clone()}</span>
                        <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| on_choisir.run(code.clone()))>
                            "Choisir ce motif"
                        </Bouton>
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}
