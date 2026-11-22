//! Écran 7 du socle 20 « Tenue et visa » (`/web/cahier/visa`, direction) : le tableau de tenue des cahiers par cours sur une
//! période, et le visa en un écran. Le visa fige la référence de ce qui est vu ; ce qui est modifié ensuite est marqué
//! « Modifié après visa ». Aucun nom d'élève, aucun classement public : des agrégats par cours.
//!
//! Droits, référence figée, observation et contrôle d'un cahier vide sont décidés par le serveur (`modele_c8` formate).

use chrono::NaiveDate;
use ent_web_api::cahier_c8_types::TenueLigneDto;
use ent_web_ui::{
    AccesRefuse, Badge, Bouton, Case, ChampTexte, EnTetePage, EtatVide, LienBouton, Squelette,
    Taille, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele_c8::{
    corps_visa, etat_ligne, lire_jour, periode_par_defaut, phrase_dates, phrase_taux, phrase_visa,
    Ton,
};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

pub const ADRESSE_VISA: &str = "/web/cahier/visa";

pub(crate) fn ton(t: Ton) -> TonEtat {
    match t {
        Ton::Ok => TonEtat::Ok,
        Ton::Info => TonEtat::Info,
        Ton::Avertissement => TonEtat::Avertissement,
        Ton::Neutre => TonEtat::Neutre,
        Ton::Mal => TonEtat::Mal,
    }
}

#[component]
pub fn PageTenueVisa() -> impl IntoView {
    let moteur = use_droits_moteur();
    view! {
        <Title text="Tenue et visa"/>
        {move || {
            if moteur.get().a("cahier.viser") {
                view! { <Contenu/> }.into_any()
            } else {
                view! { <AccesRefuse message="Le tableau de tenue et le visa sont réservés à la direction."/> }.into_any()
            }
        }}
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let (d0, a0) = periode_par_defaut(aujourdhui());
    let du = RwSignal::new(d0.to_string());
    let au = RwSignal::new(a0.to_string());
    let tenue = LocalResource::new(move || {
        let (d, a) = (lire_jour(&du.get()), lire_jour(&au.get()));
        async move {
            match (d, a) {
                (Some(d), Some(a)) => session::client().cdt_tenue(d, a).await.map(Some),
                _ => Ok(None),
            }
        }
    });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| tenue.refetch()));
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Tenue et visa".to_string()
            sous_titre="Où en sont les cahiers : séances renseignées, travaux donnés, dernier visa. Un visa fige ce que vous avez lu.".to_string()
        >
            <LienBouton href=super::ADRESSE_ARCHIVES variante=Variante::Secondaire taille=Taille::Compacte>"Clôture et archives"</LienBouton>
            <LienBouton href=super::ADRESSE_INSPECTION variante=Variante::Secondaire taille=Taille::Compacte>"Accès d'inspection"</LienBouton>
        </EnTetePage>
        <RetourEcriture ecriture=ecriture/>
        <section class="cdt-seance" aria-label="Période">
            <div class="cdt-form">
                <ChampTexte id="tenue-du" libelle="Du" valeur=du genre="date"/>
                <ChampTexte id="tenue-au" libelle="Au" valeur=au genre="date"/>
            </div>
        </section>
        {move || match tenue.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement du tableau de tenue…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| tenue.refetch())/> }.into_any(),
            Some(Ok(None)) => view! { <EtatVide titre="Période illisible" message="Indiquez deux dates valides."/> }.into_any(),
            Some(Ok(Some(t))) if t.lignes.is_empty() => view! { <EtatVide titre="Aucun cours" message="Aucune séance, aucun travail sur cette période."/> }.into_any(),
            Some(Ok(Some(t))) => {
                let (du, au) = (t.du, t.au);
                view! {
                    <ul class="cdt-liste">
                        {t.lignes.into_iter().map(|l| view! { <li><LigneCours ligne=l du=du au=au ecriture=ecriture/></li> }).collect_view()}
                    </ul>
                }.into_any()
            }
        }}
    }
}

#[component]
fn LigneCours(
    ligne: TenueLigneDto,
    du: NaiveDate,
    au: NaiveDate,
    ecriture: Ecriture,
) -> impl IntoView {
    let cours: Uuid = ligne.cours_id;
    let (etat, t) = etat_ligne(&ligne);
    let detail = format!(
        "{} · {} travail(aux) donné(s)",
        phrase_taux(&ligne),
        ligne.travaux_donnes
    );
    let dates = phrase_dates(&ligne);
    let intervenants = if ligne.intervenants.is_empty() {
        "Aucun intervenant sur la période".to_string()
    } else {
        ligne.intervenants.join(", ")
    };
    let apres = ligne.modifies_apres_visa;
    let ajoutes = ligne.ajoutes_apres_visa;
    let observation = RwSignal::new(String::new());
    let vide = RwSignal::new(false);
    let ouvert = RwSignal::new(false);
    let viser = move |_| {
        let b = corps_visa(
            cours,
            du,
            au,
            &observation.get_untracked(),
            vide.get_untracked(),
        );
        ecriture.lancer(async move {
            let v = session::client().cdt_viser(&b).await?;
            Ok(phrase_visa(&v))
        });
    };
    let href = format!(
        "{}?cours={cours}&du={du}&au={au}",
        super::ADRESSE_CHRONOLOGIE
    );
    let aria = format!("Viser le cahier de {}", ligne.libelle);
    view! {
        <article class="cdt-travail">
            <header class="cdt-travail__entete">
                <p class="cdt-travail__titre">{format!("{} — {}", ligne.libelle, ligne.public)}</p>
                <Badge libelle=etat ton=ton(t)/>
            </header>
            <p class="cdt-travail__detail">{intervenants}</p>
            <p class="cdt-travail__detail">{detail}</p>
            <p class="cdt-travail__detail">{dates}</p>
            {(apres > 0).then(|| view! { <p class="cdt-resume" role="status">{format!("{apres} élément(s) modifié(s) depuis le dernier visa.")}</p> })}
            {(ajoutes > 0).then(|| view! { <p class="cdt-resume" role="status">{format!("{ajoutes} élément(s) publié(s) depuis le dernier visa, absents de ce visa.")}</p> })}
            <div class="cdt-actions">
                <LienBouton href=href variante=Variante::Secondaire taille=Taille::Compacte>"Lire le cahier"</LienBouton>
                <Bouton taille=Taille::Compacte libelle_accessible=aria on_click=Callback::new(move |_| ouvert.update(|o| *o = !*o))>"Viser…"</Bouton>
            </div>
            {move || ouvert.get().then(|| view! {
                <div class="cdt-form">
                    <ZoneTexte id=format!("visa-obs-{cours}") libelle="Observation pour les intervenants (facultative)" valeur=observation/>
                    <Case id=format!("visa-vide-{cours}") libelle="Je confirme le visa d'un cahier sans contenu publié" coche=vide/>
                    <Bouton en_cours=ecriture.en_cours on_click=Callback::new(viser)>"Viser cette période"</Bouton>
                </div>
            })}
        </article>
    }
}
