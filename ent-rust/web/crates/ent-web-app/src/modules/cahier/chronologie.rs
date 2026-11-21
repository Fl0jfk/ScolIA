//! Écran 3 du socle 20 « Cahier du cours » en lecture (`/web/cahier/chronologie?cours=&du=&au=`) : la chronologie des entrées et
//! travaux **publiés** d'un cours, vue par la direction et par les intervenants, avec les marques « Modifié après visa », les
//! visas, leurs observations et la réponse du professeur. Les brouillons n'y figurent jamais ; aucun élève n'y est nommé.

use chrono::NaiveDate;
use ent_web_api::cahier_c8_types::{ChronologieDto, VisaCahierDto};
use ent_web_ui::{
    AccesRefuse, Badge, Bouton, EnTetePage, EtatVide, LienBouton, Squelette, Taille, TonEtat,
    Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use super::modele_c8::{lire_jour, periode_par_defaut, phrase_visa};
use crate::noyau::coquille::{use_droits, use_droits_moteur};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};

pub const ADRESSE_CHRONOLOGIE: &str = "/web/cahier/chronologie";

#[component]
pub fn PageChronologie() -> impl IntoView {
    let droits = use_droits();
    let moteur = use_droits_moteur();
    let q = use_query_map();
    let cours = Memo::new(move |_| q.get().get("cours").and_then(|v| Uuid::parse_str(&v).ok()));
    let (d0, a0) = periode_par_defaut(aujourdhui());
    let du = Memo::new(move |_| q.get().get("du").and_then(|v| lire_jour(&v)).unwrap_or(d0));
    let au = Memo::new(move |_| q.get().get("au").and_then(|v| lire_jour(&v)).unwrap_or(a0));
    view! {
        <Title text="Cahier du cours"/>
        {move || {
            if !(droits.get().cahier_ecrire || moteur.get().a("cahier.viser")) {
                return view! { <AccesRefuse message="Le cahier du cours est lu par la direction et par ses intervenants."/> }.into_any();
            }
            match cours.get() {
                Some(c) => view! { <Contenu cours=c du=du.get() au=au.get()/> }.into_any(),
                None => view! { <EtatVide titre="Cours introuvable" message="L'adresse de cette page est incorrecte."/> }.into_any(),
            }
        }}
    }
}

#[component]
fn Contenu(cours: Uuid, du: NaiveDate, au: NaiveDate) -> impl IntoView {
    let chrono = LocalResource::new(move || async move {
        session::client().cdt_chronologie(cours, du, au).await
    });
    view! {
        {move || match chrono.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement du cahier…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| chrono.refetch())/> }.into_any(),
            Some(Ok(c)) => view! { <Corps c=c chrono=chrono/> }.into_any(),
        }}
    }
}

#[component]
fn Corps(
    c: ChronologieDto,
    chrono: LocalResource<Result<ChronologieDto, ent_web_api::ApiError>>,
) -> impl IntoView {
    let titre = format!("{} — {}", c.cours.libelle, c.cours.public);
    let periode = format!("Du {} au {}", date_longue(c.du), date_longue(c.au));
    let ecriture = Ecriture::nouveau(Callback::new(move |_| chrono.refetch()));
    view! {
        <EnTetePage surtitre="Cahier de textes".to_string() titre=titre sous_titre=periode>
            <LienBouton href=super::ADRESSE_VISA variante=Variante::Secondaire taille=Taille::Compacte>"Tenue et visa"</LienBouton>
        </EnTetePage>
        <RetourEcriture ecriture=ecriture/>
        <section class="cdt-seance" aria-label="Visas du cours">
            <h2 class="cdt-travail__titre">"Visas"</h2>
            {if c.visas.is_empty() {
                view! { <p class="cdt-resume">"Aucun visa pour ce cours."</p> }.into_any()
            } else {
                view! { <ul class="cdt-liste">{c.visas.into_iter().map(|v| view! { <li><Visa v=v ecriture=ecriture/></li> }).collect_view()}</ul> }.into_any()
            }}
        </section>
        <section class="cdt-seance" aria-label="Entrées et travaux">
            <h2 class="cdt-travail__titre">"Entrées et travaux publiés"</h2>
            {if c.items.is_empty() {
                view! { <EtatVide titre="Rien de publié" message="Aucune entrée ni aucun travail publié sur la période."/> }.into_any()
            } else {
                view! {
                    <ul class="cdt-liste">
                        {c.items.into_iter().map(|i| {
                            let genre = if i.objet == "travail" { "Travail" } else { "Séance" };
                            view! {
                                <li class="cdt-travail">
                                    <header class="cdt-travail__entete">
                                        <p class="cdt-travail__titre">{format!("{} · {} · {}", date_longue(i.jour), genre, i.titre.clone().unwrap_or_else(|| "(sans titre)".into()))}</p>
                                        {i.modifie_apres_visa.then(|| view! { <Badge libelle="Modifié après visa" ton=TonEtat::Avertissement/> })}
                                    </header>
                                    <p class="cdt-travail__consigne">{i.contenu.clone()}</p>
                                    <p class="cdt-travail__detail">{format!("Version {} · {} · modifié le {}", i.version, i.auteur, date_longue(i.modifie_le.date_naive()))}</p>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

/// Un visa : sa date, son viseur, ce qu'il a figé, l'observation et la réponse du professeur.
#[component]
fn Visa(v: VisaCahierDto, ecriture: Ecriture) -> impl IntoView {
    // La direction observe, elle ne répond pas : le formulaire de réponse est celui des intervenants.
    let peut_repondre = !use_droits_moteur().get_untracked().a("cahier.viser");
    let phrase = format!(
        "{} — visa du {} par {}",
        phrase_visa(&v),
        date_longue(v.vise_le.date_naive()),
        v.vise_par
    );
    let apres = v.modifies_apres_visa;
    view! {
        <article class="cdt-travail">
            <header class="cdt-travail__entete">
                <p class="cdt-travail__titre">{v.libelle_cible.clone()}</p>
                {(apres > 0).then(|| view! { <Badge libelle=format!("{apres} modifié(s) après visa") ton=TonEtat::Avertissement/> })}
            </header>
            <p class="cdt-travail__detail">{phrase}</p>
            {v.observations.into_iter().map(|o| {
                let id = o.id;
                let reponse = RwSignal::new(String::new());
                let repondre = move |_| {
                    let t = reponse.get_untracked();
                    ecriture.lancer(async move {
                        session::client().cdt_repondre(id, t).await?;
                        Ok("Réponse enregistrée.".to_string())
                    });
                };
                view! {
                    <blockquote class="cdt-travail__consigne">
                        <p>{format!("Observation de {} : {}", o.par, o.texte)}</p>
                        {o.reponses.into_iter().map(|r| view! { <p class="cdt-travail__detail">{format!("Réponse de {} : {}", r.par, r.texte)}</p> }).collect_view()}
                        {peut_repondre.then(|| view! {
                            <ZoneTexte id=format!("rep-{id}") libelle="Votre réponse" valeur=reponse/>
                            <Bouton taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(repondre)>"Répondre"</Bouton>
                        })}
                    </blockquote>
                }
            }).collect_view()}
        </article>
    }
}
