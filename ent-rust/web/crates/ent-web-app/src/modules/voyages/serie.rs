//! Séries hebdomadaires de sorties : page « Nouvelle série » (gabarit + règle hebdomadaire), résultat
//! lisible (séances créées, dates écartées) et « Valider toute la série » pour qui en a le droit.
//! L'écran n'ajoute aucune règle : le serveur calcule les dates, écarte fériés et vacances et vérifie le
//! droit de valider séance par séance ; ses refus sont affichés tels quels.

use ent_types::voyages::{ResultatSeanceDto, ResultatSerieDto};
use ent_web_ui::{
    Badge, Bouton, Case, ChampTexte, EnTetePage, ResumeErreurs, Selection, Squelette, TonEtat,
    ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele::{periode, DESTINATIONS, NATURES};
use super::modele_serie::{
    bilan_validation, composer_serie, libelle_motif_ecarte, resume_creation, SaisieSerie,
    JOURS_SEMAINE,
};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_complete, date_heure_paris};

fn options(liste: &[(&str, &str)]) -> Vec<(String, String)> {
    liste
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect()
}

/// Place le focus sur un élément après le rendu (annonce visuelle et clavier du résultat).
#[cfg(target_arch = "wasm32")]
fn focaliser(id: &'static str) {
    use leptos::wasm_bindgen::JsCast;
    leptos::prelude::set_timeout(
        move || {
            if let Some(el) = leptos::prelude::document()
                .get_element_by_id(id)
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = el.focus();
            }
        },
        std::time::Duration::from_millis(60),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn focaliser(_id: &'static str) {}

#[component]
pub fn PageNouvelleSerie() -> impl IntoView {
    let classes = LocalResource::new(|| async { session::client().classes(aujourdhui()).await });
    let titre = RwSignal::new(String::new());
    let jour = RwSignal::new("1".to_string());
    let heure_debut = RwSignal::new("13:30".to_string());
    let heure_fin = RwSignal::new("15:30".to_string());
    let du = RwSignal::new(String::new());
    let au = RwSignal::new(String::new());
    let nature = RwSignal::new("obligatoire".to_string());
    let destination = RwSignal::new("proximite".to_string());
    let pays = RwSignal::new(String::new());
    let lieu = RwSignal::new(String::new());
    let objectifs = RwSignal::new(String::new());
    let effectif = RwSignal::new(String::new());
    let adultes = RwSignal::new(String::new());
    let rdv = RwSignal::new(String::new());
    let car = RwSignal::new(false);
    let choisies = RwSignal::new(Vec::<Uuid>::new());
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let resultat = RwSignal::new(None::<ResultatSerieDto>);
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    // Dates proposées par défaut : posées après l'hydratation, jamais dans le HTML du serveur.
    Effect::new(move |_| {
        if du.get_untracked().is_empty() {
            du.set(aujourdhui().format("%Y-%m-%d").to_string());
        }
    });
    let erreur_de = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs
                .get()
                .into_iter()
                .find(|(c, _)| c == champ)
                .map(|(_, m)| m)
        })
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieSerie {
            titre: titre.get_untracked(),
            jour: jour.get_untracked(),
            heure_debut: heure_debut.get_untracked(),
            heure_fin: heure_fin.get_untracked(),
            du: du.get_untracked(),
            au: au.get_untracked(),
            nature: nature.get_untracked(),
            destination: destination.get_untracked(),
            pays: pays.get_untracked(),
            lieu: lieu.get_untracked(),
            objectifs: objectifs.get_untracked(),
            effectif: effectif.get_untracked(),
            adultes: adultes.get_untracked(),
            point_rendez_vous: rdv.get_untracked(),
            besoin_car: car.get_untracked(),
            classes: choisies.get_untracked(),
        };
        match composer_serie(&saisie) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    let r = session::client().voyage_serie_creer(&corps).await?;
                    let message = resume_creation(&r);
                    resultat.set(Some(r));
                    focaliser("voy-serie-resultat");
                    Ok(message)
                });
            }
        }
    };
    view! {
        <Title text="Nouvelle série de sorties"/>
        <EnTetePage
            surtitre="Sorties et voyages".to_string()
            titre="Nouvelle série de sorties".to_string()
            sous_titre="Une même sortie chaque semaine : les séances sont créées en brouillon, jours fériés et vacances écartés.".to_string()
        />
        <p><a href=navigation::VOYAGES>"Retour aux sorties"</a></p>
        <form class="voy-formulaire" on:submit=envoyer>
            <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
            <ChampTexte id="voy-serie-titre" libelle="Titre des séances" valeur=titre requis=true erreur=erreur_de("voy-serie-titre")/>
            <Selection id="voy-serie-jour" libelle="Jour de la semaine" valeur=jour options=options(&JOURS_SEMAINE) requis=true erreur=erreur_de("voy-serie-jour")/>
            <ChampTexte id="voy-serie-heure-debut" libelle="Heure de début" genre="time" valeur=heure_debut requis=true erreur=erreur_de("voy-serie-heure-debut")/>
            <ChampTexte id="voy-serie-heure-fin" libelle="Heure de fin" genre="time" valeur=heure_fin requis=true erreur=erreur_de("voy-serie-heure-fin")/>
            <ChampTexte id="voy-serie-du" libelle="Première semaine à partir du" genre="date" valeur=du requis=true erreur=erreur_de("voy-serie-du")/>
            <ChampTexte id="voy-serie-au" libelle="Dernière semaine jusqu'au" genre="date" valeur=au requis=true erreur=erreur_de("voy-serie-au")/>
            <Selection id="voy-serie-nature" libelle="Nature" valeur=nature options=options(&NATURES) requis=true erreur=erreur_de("voy-serie-nature")/>
            <Selection id="voy-serie-destination" libelle="Destination" valeur=destination options=options(&DESTINATIONS) requis=true erreur=erreur_de("voy-serie-destination")/>
            {move || (destination.get() == "etranger").then(|| view! {
                <ChampTexte id="voy-serie-pays" libelle="Pays" valeur=pays requis=true erreur=erreur_de("voy-serie-pays")/>
            })}
            <ChampTexte id="voy-serie-lieu" libelle="Lieu" valeur=lieu aide="Nom du lieu ou de la ville."/>
            <ChampTexte id="voy-serie-effectif" libelle="Nombre d'élèves prévu" genre="number" valeur=effectif erreur=erreur_de("voy-serie-effectif")/>
            <ChampTexte id="voy-serie-adultes" libelle="Nombre d'adultes prévu" genre="number" valeur=adultes erreur=erreur_de("voy-serie-adultes")/>
            <ChampTexte id="voy-serie-rdv" libelle="Point de rendez-vous" valeur=rdv/>
            <Case id="voy-serie-car" libelle="Un car est nécessaire" coche=car/>
            <ZoneTexte id="voy-serie-objectifs" libelle="Objectifs pédagogiques" valeur=objectifs/>
            <fieldset id="voy-serie-classes" class="voy-classes">
                <legend>"Classes concernées"</legend>
                {move || match classes.get() {
                    None => view! { <Squelette lignes=2 libelle="Chargement des classes…"/> }.into_any(),
                    Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| classes.refetch())/> }.into_any(),
                    Some(Ok(l)) => l
                        .into_iter()
                        .map(|c| {
                            let cid = c.id;
                            let coche = RwSignal::new(false);
                            Effect::new(move |_| {
                                let c = coche.get();
                                choisies.update(|v| {
                                    v.retain(|x| *x != cid);
                                    if c {
                                        v.push(cid);
                                    }
                                });
                            });
                            view! { <Case id=format!("voy-serie-classe-{cid}") libelle=c.libelle.clone() coche=coche/> }
                        })
                        .collect_view()
                        .into_any(),
                }}
                {move || erreur_de("voy-serie-classes").get().map(|m| view! { <p class="ui-champ__erreur">{m}</p> })}
            </fieldset>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Créer la série"</Bouton>
        </form>
        <RetourEcriture ecriture=ecriture/>
        {move || resultat.get().map(|r| view! { <ResultatSerie r=r/> })}
    }
}

/// Séances créées, dates écartées (avec leur motif) et validation de toute la série.
#[component]
fn ResultatSerie(r: ResultatSerieDto) -> impl IntoView {
    let moteur = use_droits_moteur();
    let serie_id = r.serie_id;
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let bilan = RwSignal::new(None::<Vec<ResultatSeanceDto>>);
    let valider = Callback::new(move |_| {
        ecriture.lancer(async move {
            let seances = session::client().voyage_serie_valider(serie_id).await?;
            let message = bilan_validation(&seances).message();
            bilan.set(Some(seances));
            Ok(message)
        });
    });
    let resume = resume_creation(&r);
    view! {
        <section aria-labelledby="voy-serie-resultat" class="voy-resultat-serie">
            <h2 id="voy-serie-resultat" class="vs-section-titre" tabindex="-1">"Série créée"</h2>
            <p>{resume}</p>
            <h3>"Séances"</h3>
            <ol class="voy-liste voy-liste--simple">
                {r.sorties.iter().map(|s| view! {
                    <li class="voy-ligne">
                        <a href=format!("{}/{}", navigation::VOYAGES, s.id)>{s.titre.clone()}</a>
                        <span>{periode(s.debut, s.fin)}</span>
                    </li>
                }).collect_view()}
            </ol>
            <h3>"Dates écartées"</h3>
            {if r.ecartees.is_empty() {
                view! { <p class="texte-attenue">"Aucune date écartée : toutes les semaines ont une séance."</p> }.into_any()
            } else {
                view! {
                    <ul class="voy-liste voy-liste--simple">
                        {r.ecartees.iter().map(|e| view! {
                            <li class="voy-ligne">
                                <span>{date_complete(e.jour)}</span>
                                <Badge libelle=libelle_motif_ecarte(&e.motif) ton=TonEtat::Avertissement/>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
            {move || moteur.get().a("voyages.valider").then(|| view! {
                <div class="voy-actions">
                    <Bouton en_cours=ecriture.en_cours on_click=valider>"Valider toute la série"</Bouton>
                </div>
                <RetourEcriture ecriture=ecriture/>
            })}
            {move || bilan.get().map(|seances| {
                let b = bilan_validation(&seances);
                view! {
                    <div class="voy-bilan" role="status">
                        <p><strong>{b.message()}</strong></p>
                        {(!b.refusees.is_empty()).then(|| view! {
                            <ul class="voy-liste voy-liste--simple">
                                {b.refusees.iter().map(|s| view! {
                                    <li class="voy-ligne">
                                        <a href=format!("{}/{}", navigation::VOYAGES, s.sortie_id)>{s.titre.clone()}</a>
                                        <span>{date_heure_paris(s.debut)}</span>
                                        <span>{s.raison.clone().unwrap_or_else(|| "Refusée".into())}</span>
                                    </li>
                                }).collect_view()}
                            </ul>
                        })}
                    </div>
                }
            })}
        </section>
    }
}
