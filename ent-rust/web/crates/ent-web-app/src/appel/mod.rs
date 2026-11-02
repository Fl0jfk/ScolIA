//! Module d'appel : choix de la séance puis feuille d'appel.

pub mod modele;
mod feuille;

use chrono::NaiveDate;
use ent_web_api::{ApiError, Creneau};
use ent_web_ui::{Alerte, Bouton, EtatVide, Niveau, Squelette, Variante};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_query_map};
use uuid::Uuid;

use crate::entete::EnTete;
use crate::session;

/// Date du jour (heure locale du navigateur ; heure du serveur pendant le rendu).
pub fn aujourdhui() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// Libellé d'une séance : « 08:00–09:00 · Potions · 4B · Donjons ».
pub fn libelle_creneau(c: &Creneau) -> String {
    let heures = format!(
        "{}–{}",
        c.heure_debut.get(..5).unwrap_or(&c.heure_debut),
        c.heure_fin.get(..5).unwrap_or(&c.heure_fin)
    );
    let mut parts = vec![heures, c.matiere_libelle.clone(), c.classe.clone()];
    if let Some(s) = c.salle.as_ref().filter(|s| !s.is_empty()) {
        parts.push(s.clone());
    }
    parts.join(" · ")
}

/// Classes distinctes des séances, triées.
pub fn classes_de(creneaux: &[Creneau]) -> Vec<String> {
    let mut v: Vec<String> = creneaux.iter().map(|c| c.classe.clone()).collect();
    v.sort();
    v.dedup();
    v
}

#[component]
pub fn PageAppel() -> impl IntoView {
    let navigate = use_navigate();
    let query = use_query_map();

    // Sans jeton : renvoi vers la connexion (dans le navigateur uniquement).
    Effect::new(move |_| {
        if session::jeton().is_none() {
            navigate("/web/connexion?retour=/web/appel", Default::default());
        }
    });

    let jour = RwSignal::new(
        query
            .with_untracked(|q| q.get("jour"))
            .and_then(|j| j.parse().ok())
            .unwrap_or_else(aujourdhui),
    );
    let classe = RwSignal::new(String::new());
    let seance = RwSignal::new(None::<Uuid>);

    let moi = LocalResource::new(|| async { session::client().moi().await });
    let creneaux = LocalResource::new(move || {
        let j = jour.get();
        async move {
            let mut list = session::client().mes_creneaux(j).await?;
            list.sort_by(|a, b| a.heure_debut.cmp(&b.heure_debut).then(a.classe.cmp(&b.classe)));
            Ok::<_, ApiError>(list)
        }
    });

    // Changer de jour ou de classe annule la séance choisie.
    Effect::new(move |prec: Option<(NaiveDate, String)>| {
        let courant = (jour.get(), classe.get());
        if prec.is_some_and(|p| p != courant) {
            seance.set(None);
        }
        courant
    });

    view! {
        <Title text="Appel"/>
        <div class="app">
            // Lien d'évitement dans la page, pas à la racine de l'App : placé juste après
            // les balises leptos_meta, il cassait l'hydratation (voir LEPTOS_CALL_PROTOTYPE.md).
            <a class="lien-evitement" href="#contenu">"Aller au contenu"</a>
            <EnTete moi=moi/>
            <main id="contenu" class="page">
                <div class="page__entete">
                    <h1>"Faire l'appel"</h1>
                    <p class="page__soustitre">
                        "Choisissez une séance, marquez les élèves, puis validez."
                    </p>
                </div>

                <section class="carte selection" aria-labelledby="selection-titre">
                    <h2 id="selection-titre" class="ui-sr">"Choix de la séance"</h2>
                    <div class="champ">
                        <label for="jour">"Date"</label>
                        <input
                            id="jour"
                            type="date"
                            prop:value=move || jour.get().to_string()
                            on:change=move |ev| {
                                if let Ok(j) = event_target_value(&ev).parse() {
                                    jour.set(j);
                                }
                            }
                        />
                    </div>
                    {move || match creneaux.get() {
                        None => view! { <Squelette lignes=2 libelle="Chargement des séances…"/> }.into_any(),
                        Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| creneaux.refetch())/> }.into_any(),
                        Some(Ok(list)) if list.is_empty() => {
                            view! {
                                <EtatVide
                                    titre="Aucune séance ce jour"
                                    message="Vous n'avez pas de cours à appeler à cette date."
                                />
                            }
                                .into_any()
                        }
                        Some(Ok(list)) => view! { <ChoixSeance creneaux=list classe=classe seance=seance/> }.into_any(),
                    }}
                </section>

                {move || {
                    let choix = seance.get();
                    let j = jour.get();
                    let detail = choix.and_then(|id| {
                        creneaux.get().and_then(Result::ok).and_then(|l| l.into_iter().find(|c| c.id == id))
                    });
                    match (choix, detail) {
                        (Some(id), Some(c)) => view! { <feuille::FeuilleAppel creneau_id=id jour=j creneau=c/> }.into_any(),
                        _ => ().into_any(),
                    }
                }}
            </main>
        </div>
    }
}

#[component]
fn ChoixSeance(creneaux: Vec<Creneau>, classe: RwSignal<String>, seance: RwSignal<Option<Uuid>>) -> impl IntoView {
    let classes = classes_de(&creneaux);
    let plusieurs_classes = classes.len() > 1;
    let creneaux = StoredValue::new(creneaux);
    let filtres = move || {
        let c = classe.get();
        creneaux.with_value(|l| {
            l.iter()
                .filter(|x| c.is_empty() || x.classe == c)
                .cloned()
                .collect::<Vec<_>>()
        })
    };

    view! {
        {plusieurs_classes
            .then(|| {
                view! {
                    <div class="champ">
                        <label for="classe">"Classe"</label>
                        <select
                            id="classe"
                            on:change=move |ev| classe.set(event_target_value(&ev))
                            prop:value=move || classe.get()
                        >
                            <option value="">"Toutes les classes"</option>
                            {classes
                                .iter()
                                .map(|c| view! { <option value=c.clone()>{c.clone()}</option> })
                                .collect_view()}
                        </select>
                    </div>
                }
            })}
        <div class="champ champ--large">
            <label for="seance">"Séance"</label>
            <select
                id="seance"
                on:change=move |ev| seance.set(event_target_value(&ev).parse().ok())
                prop:value=move || seance.get().map(|s| s.to_string()).unwrap_or_default()
            >
                <option value="">"Choisir une séance…"</option>
                {move || {
                    filtres()
                        .into_iter()
                        .map(|c| view! { <option value=c.id.to_string()>{libelle_creneau(&c)}</option> })
                        .collect_view()
                }}
            </select>
        </div>
    }
}

/// Erreur de chargement avec bouton « Réessayer » ; propose la reconnexion si la session a expiré.
#[component]
pub fn ErreurChargement(erreur: ApiError, on_reessayer: Callback<()>) -> impl IntoView {
    let session_expiree = erreur == ApiError::NonConnecte;
    if session_expiree {
        session::effacer();
    }
    view! {
        <Alerte niveau=Niveau::Erreur titre=erreur.message_utilisateur()>
            {if session_expiree {
                view! { <a href="/web/connexion?retour=/web/appel">"Se reconnecter"</a> }.into_any()
            } else {
                view! {
                    <Bouton variante=Variante::Secondaire on_click=on_reessayer>
                        "Réessayer"
                    </Bouton>
                }
                    .into_any()
            }}
        </Alerte>
    }
}
