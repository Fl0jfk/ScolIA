//! Changements d'emploi du temps pour l'élève et sa famille (socle 2, S2-5) : annulations,
//! remplacements, changements de salle et déplacements à venir, par enfant. Route
//! `GET /eleves/{id}/edt/changements` : jamais de motif libre, seulement le motif public d'un lot.
//! Lecture seule ; le lien parent-enfant est vérifié par le serveur.

pub mod modele;

use chrono::NaiveDate;
use ent_web_api::{Enfant, Moi};
use ent_web_ui::{
    AccesRefuse, Annonce, Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Squelette,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::choix_enfant::{enfant_choisi, ChoixEnfant};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use modele::{grouper, periode_par_defaut, valider_periode, GroupeJour};

#[component]
pub fn PageChangementsEdt() -> impl IntoView {
    let droits = use_droits();
    let ctx = session::use_session();
    view! {
        <Title text="Changements d'emploi du temps"/>
        <EnTetePage
            titre="Changements d'emploi du temps".to_string()
            sous_titre="Cours annulés, remplacés, déplacés ou changés de salle.".to_string()
        />
        {move || match ctx.moi.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des changements…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| ctx.moi.refetch())/> }.into_any(),
            Some(Ok(moi)) => {
                let d = droits.get();
                if d.portail_famille {
                    view! { <VueFamille/> }.into_any()
                } else if d.mes_absences {
                    view! { <VueEleve moi=moi/> }.into_any()
                } else {
                    view! { <AccesRefuse message="Cet écran est réservé aux familles et aux élèves."/> }.into_any()
                }
            }
        }}
    }
}

#[component]
fn VueFamille() -> impl IntoView {
    let enfants = LocalResource::new(|| async { session::client().enfants_famille().await });
    view! {
        {move || match enfants.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de vos enfants…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| enfants.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide
                    titre="Aucun enfant rattaché"
                    message="Aucun enfant n'est lié à votre compte. Contactez le secrétariat de l'établissement."
                />
            }
            .into_any(),
            Some(Ok(l)) => view! { <ChangementsEnfants enfants=l/> }.into_any(),
        }}
    }
}

#[component]
fn ChangementsEnfants(enfants: Vec<Enfant>) -> impl IntoView {
    let liste = StoredValue::new(enfants.clone());
    let choisi = RwSignal::new(
        enfant_choisi(&enfants, None)
            .map(|i| i.to_string())
            .unwrap_or_default(),
    );
    let eleve = Signal::derive(move || {
        let demande = choisi.get().parse::<Uuid>().ok();
        liste.with_value(|l| enfant_choisi(l, demande))
    });
    view! {
        <ChoixEnfant enfants=enfants choisi=choisi/>
        <ListeChangements eleve=eleve/>
    }
}

#[component]
fn VueEleve(moi: Moi) -> impl IntoView {
    match moi.eleve_id {
        Some(id) => view! { <ListeChangements eleve=Signal::derive(move || Some(id))/> }.into_any(),
        None => view! {
            <EtatVide
                titre="Compte non lié à une fiche élève"
                message="Vos changements d'emploi du temps ne peuvent pas être affichés. Contactez le secrétariat."
            />
        }
        .into_any(),
    }
}

#[component]
fn ListeChangements(eleve: Signal<Option<Uuid>>) -> impl IntoView {
    // Période appliquée (posée après l'hydratation : la date du jour n'est jamais dans le HTML serveur).
    let periode = RwSignal::new(None::<(NaiveDate, NaiveDate)>);
    let du = RwSignal::new(String::new());
    let au = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let (d, a) = periode_par_defaut(aujourdhui());
        du.set(d.to_string());
        au.set(a.to_string());
        periode.set(Some((d, a)));
    });
    let appliquer =
        Callback::new(
            move |_| match valider_periode(&du.get_untracked(), &au.get_untracked()) {
                Ok(p) => {
                    erreur.set(None);
                    periode.set(Some(p));
                }
                Err(m) => erreur.set(Some(m)),
            },
        );
    let donnees = LocalResource::new(move || {
        let e = eleve.get();
        let p = periode.get();
        async move {
            match (e, p) {
                (Some(e), Some((d, a))) => {
                    session::client().changements_edt(e, d, a).await.map(Some)
                }
                _ => Ok(None),
            }
        }
    });
    let annonce = Signal::derive(move || match donnees.get() {
        Some(Ok(Some(l))) => match l.len() {
            0 => "Aucun changement sur la période.".to_string(),
            1 => "1 changement sur la période.".to_string(),
            n => format!("{n} changements sur la période."),
        },
        _ => String::new(),
    });
    view! {
        <Annonce message=annonce/>
        <form
            class="chg-periode"
            on:submit=move |ev| {
                ev.prevent_default();
                appliquer.run(());
            }
        >
            <ChampTexte id="chg-du" libelle="Du" genre="date" valeur=du requis=true erreur=Signal::derive(move || erreur.get())/>
            <ChampTexte id="chg-au" libelle="Au" genre="date" valeur=au requis=true/>
            <Bouton soumettre=true>"Afficher"</Bouton>
        </form>
        {move || match donnees.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des changements…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| donnees.refetch())/> }.into_any(),
            Some(Ok(None)) => view! { <Squelette lignes=3 libelle="Chargement des changements…"/> }.into_any(),
            Some(Ok(Some(l))) if l.is_empty() => view! {
                <EtatVide
                    titre="Aucun changement"
                    message="Tous les cours de la période ont lieu comme prévu."
                />
            }
            .into_any(),
            Some(Ok(Some(l))) => view! { <Groupes groupes=grouper(&l)/> }.into_any(),
        }}
        <p class="texte-attenue">
            <a href=navigation::EDT_SEMAINE>"Voir l'emploi du temps de la semaine"</a>
        </p>
    }
}

#[component]
fn Groupes(groupes: Vec<GroupeJour>) -> impl IntoView {
    view! {
        <div class="chg-jours">
            {groupes
                .into_iter()
                .map(|g| {
                    let id = format!("chg-jour-{}", g.jour);
                    let titre_id = id.clone();
                    view! {
                        <section class="chg-jour" aria-labelledby=titre_id>
                            <h2 id=id class="chg-jour__titre">{g.titre.clone()}</h2>
                            <ul class="chg-liste">
                                {g.changements
                                    .into_iter()
                                    .map(|c| view! {
                                        <li class="chg-carte">
                                            <p class="chg-carte__heure">{c.horaire.clone()}</p>
                                            <p class="chg-carte__cours">{c.cours.clone()}</p>
                                            <p class="chg-carte__nature">
                                                <Badge libelle=c.nature.libelle() ton=c.nature.ton()/>
                                            </p>
                                            {c.detail.clone().map(|d| view! { <p class="chg-carte__detail">{d}</p> })}
                                            {c.motif.clone().map(|m| view! { <p class="chg-carte__motif">{format!("Motif : {m}")}</p> })}
                                        </li>
                                    })
                                    .collect_view()}
                            </ul>
                        </section>
                    }
                })
                .collect_view()}
        </div>
    }
}
