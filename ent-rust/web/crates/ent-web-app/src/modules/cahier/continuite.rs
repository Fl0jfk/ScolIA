//! Écran C6 « Continuité » (`/web/cahier/continuite`) : les reports proposés quand une séance est annulée (un seul écran, une
//! grève = un lot), puis les continuités d'élèves que l'acteur gère (PP, vie scolaire, direction).
//!
//! Rien n'est avancé sans accord : une échéance qui serait avancée se confirme explicitement ; une proposition sans séance
//! d'arrivée se décide à part. Aucun motif d'absence ni d'exclusion n'existe ici.

use ent_web_api::cahier_c6_types::{ContinuiteDto, ReportDto};
use ent_web_ui::{
    AccesRefuse, Alerte, Badge, Bouton, EnTetePage, EtatVide, Niveau, Squelette, Taille, TonEtat,
    Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele_c6::{
    corps_decision, corps_lot, ids_acceptables_en_lot, message_lot, phrase_report,
};
use crate::noyau::coquille::use_droits;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};

pub const ADRESSE_CONTINUITE: &str = "/web/cahier/continuite";

#[component]
pub fn PageContinuite() -> impl IntoView {
    let droits = use_droits();
    view! {
        <Title text="Continuité"/>
        {move || {
            let d = droits.get();
            if d.cahier_ecrire || d.voir_classes {
                view! { <Contenu/> }.into_any()
            } else {
                view! { <AccesRefuse message="La continuité est réservée à l'équipe pédagogique et à la vie scolaire."/> }.into_any()
            }
        }}
    }
}

#[component]
fn Contenu() -> impl IntoView {
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Continuité".to_string()
            sous_titre="Séances annulées, élèves en continuité à distance : rien ne disparaît, rien n'est avancé sans votre accord.".to_string()
        />
        <Reports/>
        <Continuites/>
    }
}

#[component]
fn Reports() -> impl IntoView {
    let reports = LocalResource::new(|| async { session::client().cdt_reports().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| reports.refetch()));
    view! {
        <section class="cdt-seance" aria-label="Séances annulées">
            <h2 class="cdt-travail__titre">"Vos séances annulées"</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || match reports.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des propositions…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| reports.refetch())/> }.into_any(),
                Some(Ok(r)) if r.propositions.is_empty() => view! { <EtatVide titre="Rien à reporter" message="Aucune séance annulée n'a de contenu ou de travail à reporter."/> }.into_any(),
                Some(Ok(r)) => {
                    let lot = ids_acceptables_en_lot(&r.propositions);
                    let nb_lot = lot.len();
                    let accepter_lot = move |_| {
                        let Some(b) = corps_lot(lot.clone(), "accepter") else { return };
                        ecriture.lancer(async move {
                            let r = session::client().cdt_reports_lot(&b).await?;
                            Ok(message_lot(&r))
                        });
                    };
                    view! {
                        <p class="cdt-resume" role="status">{format!("{} proposition(s) à traiter.", r.nb_ouvertes)}</p>
                        {(nb_lot > 1).then(|| view! {
                            <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=Callback::new(accepter_lot)>
                                {format!("Accepter les {nb_lot} reports sans échéance avancée")}
                            </Bouton>
                        })}
                        <ul class="cdt-liste">
                            {r.propositions.into_iter().map(|p| view! { <li><LigneReport report=p ecriture=ecriture/></li> }).collect_view()}
                        </ul>
                    }
                    .into_any()
                }
            }}
        </section>
    }
}

#[component]
fn LigneReport(report: ReportDto, ecriture: Ecriture) -> impl IntoView {
    let phrase = phrase_report(&report);
    let ouverte = report.etat == "proposee";
    let a_cible = report.vers.is_some();
    let avance = report.avance_l_echeance;
    let r1 = report.clone();
    let r2 = report.clone();
    let confirme = RwSignal::new(false);
    let id: Uuid = report.id;
    let decider = move |accepter: bool, r: ReportDto| {
        let corps = corps_decision(accepter, &r, confirme.get_untracked());
        ecriture.lancer(async move {
            session::client().cdt_report_decider(id, &corps).await?;
            Ok(if accepter { "Report appliqué : élèves et familles prévenus une seule fois.".to_string() } else { "Proposition refusée : rien ne change.".to_string() })
        });
    };
    let (etat, ton) = match report.etat.as_str() {
        "proposee" => ("À traiter", TonEtat::Info),
        "appliquee" => ("Appliqué", TonEtat::Ok),
        "appliquee_auto" => ("Appliqué seul à l'heure de la séance", TonEtat::Ok),
        _ => ("Refusé", TonEtat::Neutre),
    };
    view! {
        <article class="cdt-travail">
            <header class="cdt-travail__entete">
                <p class="cdt-travail__titre">{phrase}</p>
                <Badge libelle=etat ton=ton/>
            </header>
            {(ouverte && avance).then(|| view! {
                <Alerte niveau=Niveau::Avertissement titre="Ce report avancerait l'échéance : confirmez-le explicitement pour l'appliquer."/>
                <label class="ui-case"><input type="checkbox" prop:checked=move || confirme.get() on:change=move |ev| confirme.set(event_target_checked(&ev))/>
                    <span>"Je confirme que l'échéance soit avancée"</span></label>
            })}
            {ouverte.then(|| view! {
                <div class="cdt-actions">
                    <Bouton taille=Taille::Compacte desactive=!a_cible en_cours=ecriture.en_cours on_click=Callback::new(move |_| decider(true, r1.clone()))>"Accepter le report"</Bouton>
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(move |_| decider(false, r2.clone()))>"Refuser"</Bouton>
                </div>
            })}
        </article>
    }
}

#[component]
fn Continuites() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().cdt_continuites().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    view! {
        <section class="cdt-seance" aria-label="Continuités d'élèves">
            <h2 class="cdt-travail__titre">"Élèves en continuité"</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || match liste.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des continuités…"/> }.into_any(),
                // Un professeur qui n'est pas PP n'a pas cette liste : le serveur répond 403, l'écran se tait.
                Some(Err(_)) => ().into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune continuité" message="Aucun élève n'est en continuité à distance."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="cdt-liste">
                        {l.into_iter().map(|c| view! { <li><LigneContinuite c=c ecriture=ecriture/></li> }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </section>
    }
}

#[component]
fn LigneContinuite(c: ContinuiteDto, ecriture: Ecriture) -> impl IntoView {
    let id = c.id;
    let periode = match c.au {
        Some(au) => format!("du {} au {}", date_longue(c.du), date_longue(au)),
        None => format!("depuis le {}", date_longue(c.du)),
    };
    let nom = format!("{} {}", c.prenom, c.nom);
    let aria = format!("Clore la continuité de {nom}");
    let close = c.close;
    // La nature n'est servie qu'à la vie scolaire ; jamais de motif.
    let nature = c.nature.map(|n| if n == "exclusion" { "Exclusion temporaire" } else { "Absence longue" });
    let clore = move |_| {
        ecriture.lancer(async move {
            session::client().cdt_continuite_clore(id, aujourdhui()).await?;
            Ok("Continuité close.".to_string())
        });
    };
    view! {
        <article class="cdt-travail">
            <header class="cdt-travail__entete">
                <p class="cdt-travail__titre">{nom}</p>
                {nature.map(|n| view! { <Badge libelle=n ton=TonEtat::Neutre/> })}
            </header>
            <p class="cdt-travail__detail">{format!("{periode} · {} cours", c.cours_ids.len())}</p>
            {(!close).then(|| view! {
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=aria en_cours=ecriture.en_cours on_click=Callback::new(clore)>"Clore aujourd'hui"</Bouton>
            })}
        </article>
    }
}
