//! Écran C7 « Adaptations d'un travail » (`/web/cahier/adaptations/:id`, professeur en saisie) : les **variantes** (consigne,
//! durée, échéance propres à des élèves nommés — chaque élève lit sa version comme la seule, un camarade ne voit rien), les
//! **pastilles** du cours (faits utiles, jamais un diagnostic) et « appliquer les aménagements » (aperçu puis application).
//! Le libellé d'une variante est interne : il n'est jamais montré aux élèves.

use ent_web_api::cahier_c7_types::{PastillesCoursDto, VarianteDto};
use ent_web_ui::{
    AccesRefuse, Bouton, Case, ChampTexte, EnTetePage, EtatVide, LienBouton, Squelette, Taille,
    Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_params_map, use_query_map};
use uuid::Uuid;

use super::modele_c7::{corps_variante, libelle_pastilles, phrase_amenagements};
use crate::noyau::coquille::use_droits;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;

pub const ADRESSE_ADAPTATIONS: &str = "/web/cahier/adaptations";

#[component]
pub fn PageAdaptations() -> impl IntoView {
    let droits = use_droits();
    let params = use_params_map();
    let query = use_query_map();
    let id = Memo::new(move |_| params.with(|p| p.get("id")).and_then(|s| s.parse::<Uuid>().ok()));
    let cours = Memo::new(move |_| query.with(|q| q.get("cours")).and_then(|s| s.parse::<Uuid>().ok()));
    view! {
        <Title text="Adaptations"/>
        {move || {
            if !droits.get().cahier_ecrire {
                return view! { <AccesRefuse message="Les adaptations sont réservées aux enseignants du cours."/> }.into_any();
            }
            match (id.get(), cours.get()) {
                (Some(t), Some(c)) => view! { <Contenu travail=t cours=c/> }.into_any(),
                _ => view! { <EtatVide titre="Travail introuvable" message="L'adresse de cette page est incorrecte."/> }.into_any(),
            }
        }}
    }
}

#[component]
fn Contenu(travail: Uuid, cours: Uuid) -> impl IntoView {
    let variantes = LocalResource::new(move || async move { session::client().cdt_variantes(travail).await });
    let pastilles = LocalResource::new(move || async move { session::client().cdt_pastilles(cours).await });
    view! {
        <EnTetePage surtitre="Cahier de textes".to_string() titre="Adaptations du travail".to_string()
            sous_titre="Chaque élève voit sa version comme la seule. Rien ici n'est montré aux élèves ni à leurs camarades.".to_string()>
            <LienBouton href=navigation::CAHIER variante=Variante::Secondaire taille=Taille::Compacte>"Retour au cahier"</LienBouton>
        </EnTetePage>
        {move || match pastilles.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des élèves du cours…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| pastilles.refetch())/> }.into_any(),
            Some(Ok(p)) => view! {
                <Amenagements travail=travail pastilles=p.clone()/>
                <Variantes travail=travail pastilles=p variantes=variantes/>
            }.into_any(),
        }}
    }
}

/// Pastilles du cours et « appliquer les aménagements ».
#[component]
fn Amenagements(travail: Uuid, pastilles: PastillesCoursDto) -> impl IntoView {
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let phrase = RwSignal::new(String::new());
    let apercu = move |confirmer: bool| {
        spawn_local_ecriture(ecriture, phrase, travail, confirmer);
    };
    view! {
        <section class="cdt-seance" aria-label="Aménagements">
            <h2 class="cdt-travail__titre">"Aménagements des élèves du cours"</h2>
            <RetourEcriture ecriture=ecriture/>
            <table class="cdt-tableau">
                <caption class="ui-sr">"Faits utiles par élève"</caption>
                <thead><tr><th scope="col">"Élève"</th><th scope="col">"Faits utiles"</th></tr></thead>
                <tbody>
                    {pastilles.eleves.into_iter().map(|e| view! {
                        <tr><th scope="row">{format!("{} {}", e.nom, e.prenom)}</th><td>{libelle_pastilles(&e.pastilles)}</td></tr>
                    }).collect_view()}
                </tbody>
            </table>
            <div class="cdt-actions">
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| apercu(false))>"Aperçu : appliquer les aménagements"</Bouton>
                <Bouton en_cours=ecriture.en_cours on_click=Callback::new(move |_| apercu(true))>"Appliquer les aménagements"</Bouton>
            </div>
            {move || (!phrase.get().is_empty()).then(|| view! { <p class="cdt-resume" role="status">{phrase.get()}</p> })}
        </section>
    }
}

fn spawn_local_ecriture(ecriture: Ecriture, phrase: RwSignal<String>, travail: Uuid, confirmer: bool) {
    ecriture.lancer(async move {
        let r = session::client().cdt_amenagements(travail, confirmer).await?;
        let p = phrase_amenagements(&r);
        phrase.set(p.clone());
        Ok(p)
    });
}

/// Variantes : créer, attribuer à des élèves nommés, retirer.
#[component]
fn Variantes(
    travail: Uuid,
    pastilles: PastillesCoursDto,
    variantes: LocalResource<Result<Vec<VarianteDto>, ent_web_api::ApiError>>,
) -> impl IntoView {
    let ecriture = Ecriture::nouveau(Callback::new(move |_| variantes.refetch()));
    let libelle = RwSignal::new(String::new());
    let consigne = RwSignal::new(String::new());
    let duree = RwSignal::new(String::new());
    let jour = RwSignal::new(String::new());
    let heure = RwSignal::new("18:00".to_string());
    let eleves = StoredValue::new(pastilles.eleves.clone());
    let selection: Vec<(Uuid, RwSignal<bool>)> = pastilles.eleves.iter().map(|e| (e.eleve_id, RwSignal::new(false))).collect();
    let selection = StoredValue::new(selection);
    let choisis = move || selection.get_value().iter().filter(|(_, s)| s.get_untracked()).map(|(id, _)| *id).collect::<Vec<_>>();
    let creer = move |_| {
        match corps_variante(&libelle.get_untracked(), &consigne.get_untracked(), &duree.get_untracked(), &jour.get_untracked(), &heure.get_untracked()) {
            Err(m) => ecriture.erreur.set(Some(m)),
            Ok(b) => ecriture.lancer(async move {
                session::client().cdt_variante_creer(travail, &b).await?;
                Ok("Version créée : attribuez-la à des élèves nommés.".to_string())
            }),
        }
    };
    view! {
        <section class="cdt-seance" aria-label="Variantes">
            <h2 class="cdt-travail__titre">"Versions du travail"</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || match variantes.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des versions…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| variantes.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <p class="cdt-resume">"Aucune variante : tous les élèves lisent la consigne commune."</p> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="cdt-liste">
                        {l.into_iter().map(|v| {
                            let id = v.id;
                            let noms = v.eleves.iter().map(|e| format!("{} {}", e.prenom, e.nom)).collect::<Vec<_>>().join(", ");
                            let attribuer = move |_| {
                                let ids = choisis();
                                if ids.is_empty() { ecriture.erreur.set(Some("Cochez au moins un élève.".into())); return; }
                                ecriture.lancer(async move { session::client().cdt_variante_attribuer(id, ids).await?; Ok("Version attribuée.".to_string()) });
                            };
                            let retirer = move |_| {
                                let ids = choisis();
                                if ids.is_empty() { ecriture.erreur.set(Some("Cochez au moins un élève.".into())); return; }
                                ecriture.lancer(async move { session::client().cdt_variante_retirer(id, ids).await?; Ok("Les élèves cochés reviennent à la consigne commune.".to_string()) });
                            };
                            view! {
                                <li class="cdt-travail">
                                    <p class="cdt-travail__titre">{format!("{} (nom interne)", v.libelle_interne)}</p>
                                    <p class="cdt-travail__consigne">{v.consigne.clone()}</p>
                                    <p class="cdt-travail__detail">{if noms.is_empty() { "Aucun élève attribué.".to_string() } else { format!("Attribuée à : {noms}") }}</p>
                                    <div class="cdt-actions">
                                        <Bouton taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(attribuer)>"Attribuer aux élèves cochés"</Bouton>
                                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(retirer)>"Retirer aux élèves cochés"</Bouton>
                                    </div>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
            <fieldset class="cdt-form">
                <legend>"Élèves concernés (pour attribuer ou retirer)"</legend>
                {selection.get_value().into_iter().map(|(id, coche)| {
                    let nom = eleves.get_value().iter().find(|e| e.eleve_id == id).map(|e| format!("{} {}", e.nom, e.prenom)).unwrap_or_default();
                    view! { <Case id=format!("var-sel-{id}") libelle=nom coche=coche/> }
                }).collect_view()}
            </fieldset>
            <div class="cdt-form">
                <h3 class="cdt-travail__titre">"Nouvelle version"</h3>
                <ChampTexte id="var-libelle" libelle="Nom interne (jamais montré aux élèves)" valeur=libelle requis=true/>
                <ZoneTexte id="var-consigne" libelle="Consigne de cette version" valeur=consigne requis=true/>
                <ChampTexte id="var-duree" libelle="Durée estimée en minutes (facultatif)" valeur=duree genre="number"/>
                <ChampTexte id="var-jour" libelle="Échéance propre (facultatif)" valeur=jour genre="date"/>
                <ChampTexte id="var-heure" libelle="Heure" valeur=heure genre="time"/>
                <Bouton on_click=Callback::new(creer) en_cours=ecriture.en_cours>"Créer la version"</Bouton>
            </div>
        </section>
    }
}
