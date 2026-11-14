//! Emploi du temps : semaine de la personne connectée (route existante
//! `/edt/mes-creneaux`, un appel par jour ouvré ; remplacements gérés par le serveur).

pub mod modele;

use chrono::NaiveDate;
use ent_web_api::{ApiError, Creneau};
use ent_web_ui::{EnTetePage, Icone, LienBouton, NomIcone, Squelette, Taille, Variante};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;

use crate::composants_metier::seances::{lien_appel, trier};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue, heure_courte};
use modele::{jours_ouvres, libelle_semaine, semaine_demandee, semaine_voisine};

type Semaine = Vec<(NaiveDate, Vec<Creneau>)>;

#[component]
pub fn PageEmploiDuTemps() -> impl IntoView {
    let query = use_query_map();
    let lundi =
        Memo::new(move |_| semaine_demandee(query.with(|q| q.get("semaine")), aujourdhui()));
    let semaine = LocalResource::new(move || {
        let l = lundi.get();
        async move {
            let client = session::client();
            let mut jours: Semaine = Vec::with_capacity(5);
            for j in jours_ouvres(l) {
                jours.push((j, trier(client.mes_creneaux(j).await?)));
            }
            Ok::<_, ApiError>(jours)
        }
    });
    let adresse = move |decalage: i64| {
        let cible = semaine_voisine(lundi.get(), decalage).unwrap_or_else(|| lundi.get());
        format!("{}?semaine={}", navigation::EMPLOI_DU_TEMPS, cible)
    };

    view! {
        <Title text="Emploi du temps"/>
        <EnTetePage
            surtitre="Emploi du temps".to_string()
            titre=Signal::derive(move || libelle_semaine(lundi.get()))
        >
            <nav class="edt-semaines" aria-label="Changer de semaine">
                <a class="ui-btn ui-btn--secondaire ui-btn--compact" href=move || adresse(-1) aria-label="Semaine précédente">
                    <Icone nom=NomIcone::ChevronGauche petite=true/>
                </a>
                <LienBouton href=navigation::EMPLOI_DU_TEMPS variante=Variante::Secondaire taille=Taille::Compacte>
                    "Cette semaine"
                </LienBouton>
                <a class="ui-btn ui-btn--secondaire ui-btn--compact" href=move || adresse(1) aria-label="Semaine suivante">
                    <Icone nom=NomIcone::ChevronDroite petite=true/>
                </a>
            </nav>
        </EnTetePage>
        {move || match semaine.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement de l'emploi du temps…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| semaine.refetch())/> }.into_any(),
            Some(Ok(jours)) => view! { <GrilleSemaine jours=jours/> }.into_any(),
        }}
    }
}

#[component]
fn GrilleSemaine(jours: Semaine) -> impl IntoView {
    let droits = use_droits();
    let aujourd_hui = aujourdhui();
    view! {
        <div class="edt-semaine">
            {jours
                .into_iter()
                .map(|(jour, seances)| {
                    let est_aujourdhui = jour == aujourd_hui;
                    let id_titre = format!("jour-{jour}");
                    view! {
                        <section class="edt-jour" class:edt-jour--aujourdhui=est_aujourdhui aria-labelledby=id_titre.clone()>
                            <h2 id=id_titre.clone() class="edt-jour__titre">
                                {date_longue(jour)}
                                {est_aujourdhui.then(|| view! { <span class="ui-badge ui-badge--info">"Aujourd'hui"</span> })}
                            </h2>
                            {if seances.is_empty() {
                                view! { <p class="edt-jour__vide">"Aucun cours"</p> }.into_any()
                            } else {
                                view! {
                                    <ul class="edt-jour__seances">
                                        {seances
                                            .into_iter()
                                            .map(|c| {
                                                let lien = lien_appel(&c, jour);
                                                let salle = c.salle.clone().filter(|s| !s.is_empty());
                                                view! {
                                                    <li class="edt-seance">
                                                        <p class="edt-seance__heure">
                                                            {format!("{} – {}", heure_courte(&c.heure_debut), heure_courte(&c.heure_fin))}
                                                        </p>
                                                        <p class="edt-seance__titre">{c.matiere_libelle.clone()}</p>
                                                        <p class="edt-seance__detail">
                                                            {c.classe.clone()}
                                                            {salle.map(|s| format!(" · Salle {s}"))}
                                                            {(c.semaine_ab != "toutes").then(|| format!(" · Semaine {}", c.semaine_ab.to_uppercase()))}
                                                        </p>
                                                        {move || droits.get().faire_appel.then(|| {
                                                            let lien = lien.clone();
                                                            view! {
                                                                <a class="edt-seance__appel" href=lien>
                                                                    <Icone nom=NomIcone::Appel petite=true/>
                                                                    "Faire l'appel"
                                                                </a>
                                                            }
                                                        })}
                                                    </li>
                                                }
                                            })
                                            .collect_view()}
                                    </ul>
                                }
                                    .into_any()
                            }}
                        </section>
                    }
                })
                .collect_view()}
        </div>
    }
}
