//! Écran « Progressions » (`/web/cahier/progressions`, professeur, lot C9) : le plan privé de l'année (étapes, notions, durée en
//! séances), son partage avec un cours ou une discipline, sa planification sur les séances d'un cours (brouillons, jamais publiés) et
//! la reconduction d'une année à l'autre. Aucune donnée d'élève n'existe dans une progression.
//!
//! Confidentialité, séances libres, idempotence et non-copie des données d'élèves sont décidées par le serveur (`modele_c9`).

use chrono::NaiveDate;
use ent_web_api::cahier_c2_types::CoursSaisieDto;
use ent_web_api::cahier_c9_types::{
    CopierCoursBody, PartageBody, PlanifierBody, ProgressionResumeDto, ReconduireBody,
};
use ent_web_ui::{
    AccesRefuse, Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Selection, Squelette, Taille,
    TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele_c9::{corps_progression, phrase_copie, phrase_plan, phrase_taille};
use crate::noyau::coquille::use_droits;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_longue;

pub const ADRESSE_PROGRESSIONS: &str = "/web/cahier/progressions";

#[component]
pub fn PageProgressions() -> impl IntoView {
    let droits = use_droits();
    view! {
        <Title text="Progressions"/>
        {move || {
            if droits.get().cahier_ecrire {
                view! { <Contenu/> }.into_any()
            } else {
                view! { <AccesRefuse message="Les progressions sont réservées aux enseignants."/> }.into_any()
            }
        }}
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().cdt_progressions().await });
    let cours = LocalResource::new(|| async { session::client().cdt_cours().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Progressions".to_string()
            sous_titre="Votre plan de l'année, privé par défaut. Planifier crée des brouillons sur les séances libres : rien n'est publié.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match (liste.get(), cours.get()) {
            (None, _) | (_, None) => view! { <Squelette lignes=3 libelle="Chargement de vos progressions…"/> }.into_any(),
            (Some(Err(e)), _) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            (_, Some(Err(e))) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| cours.refetch())/> }.into_any(),
            (Some(Ok(l)), Some(Ok(c))) => view! {
                <section class="cdt-seance" aria-label="Mes progressions">
                    <h2 class="cdt-travail__titre">"Mes progressions"</h2>
                    {if l.mes.is_empty() {
                        view! { <EtatVide titre="Aucune progression" message="Créez votre première progression ci-dessous."/> }.into_any()
                    } else {
                        view! { <ul class="cdt-liste">{l.mes.into_iter().map(|p| view! { <li><Ligne p=p cours=c.clone() ecriture=ecriture/></li> }).collect_view()}</ul> }.into_any()
                    }}
                </section>
                {(!l.partagees.is_empty()).then(|| view! {
                    <section class="cdt-seance" aria-label="Progressions partagées avec moi">
                        <h2 class="cdt-travail__titre">"Partagées avec moi"</h2>
                        <ul class="cdt-liste">{l.partagees.into_iter().map(|p| view! { <li><Ligne p=p cours=c.clone() ecriture=ecriture/></li> }).collect_view()}</ul>
                    </section>
                })}
                <Copie cours=c.clone() ecriture=ecriture/>
            }.into_any(),
        }}
        <Nouvelle ecriture=ecriture/>
    }
}

#[component]
fn Ligne(p: ProgressionResumeDto, cours: Vec<CoursSaisieDto>, ecriture: Ecriture) -> impl IntoView {
    let id = p.id;
    let mienne = p.mienne;
    let cours_choisi = RwSignal::new(
        cours
            .first()
            .map(|c| c.cours_id.to_string())
            .unwrap_or_default(),
    );
    let options: Vec<(String, String)> = cours
        .iter()
        .map(|c| {
            (
                c.cours_id.to_string(),
                format!("{} — {}", c.libelle, c.cible),
            )
        })
        .collect();
    let jusqu = RwSignal::new(String::new());
    let detail = format!(
        "{} · modifiée le {}",
        phrase_taille(&p),
        date_longue(p.modifie_le.date_naive())
    );
    let sous = match (&p.matiere_libelle, &p.niveau_libelle) {
        (Some(m), Some(n)) => Some(format!("{m} · {n}")),
        (Some(m), None) => Some(m.clone()),
        (None, Some(n)) => Some(n.clone()),
        _ => None,
    };
    let planifier = move |apercu: bool| {
        let Ok(c) = Uuid::parse_str(&cours_choisi.get_untracked()) else {
            ecriture.erreur.set(Some("Choisissez un cours.".into()));
            return;
        };
        let j: Option<NaiveDate> =
            NaiveDate::parse_from_str(jusqu.get_untracked().trim(), "%Y-%m-%d").ok();
        let b = PlanifierBody {
            cours_id: c,
            a_partir_du: None,
            jusqu_au: j,
            apercu,
        };
        ecriture.lancer(async move {
            Ok(phrase_plan(
                &session::client().cdt_progression_planifier(id, &b).await?,
            ))
        });
    };
    let partager = move |_| {
        let Ok(c) = Uuid::parse_str(&cours_choisi.get_untracked()) else {
            ecriture.erreur.set(Some("Choisissez un cours.".into()));
            return;
        };
        let b = PartageBody {
            cours_id: Some(c),
            matiere_id: None,
        };
        ecriture.lancer(async move {
            session::client().cdt_progression_partager(id, &b).await?;
            Ok("Progression partagée avec ce cours : ses intervenants la lisent, sans accès à votre cahier.".to_string())
        });
    };
    let reconduire = move |_| {
        ecriture.lancer(async move {
            session::client()
                .cdt_progression_reconduire(id, &ReconduireBody { annee_id: None })
                .await?;
            Ok(
                "Copie privée créée, sans dates : retrouvez-la dans « Mes progressions »."
                    .to_string(),
            )
        });
    };
    view! {
        <article class="cdt-travail">
            <header class="cdt-travail__entete">
                <p class="cdt-travail__titre">{p.titre.clone()}</p>
                <Badge libelle=if mienne { "Privée".to_string() } else { format!("Partagée par {}", p.auteur) } ton=if mienne { TonEtat::Neutre } else { TonEtat::Info }/>
            </header>
            {sous.map(|s| view! { <p class="cdt-travail__detail">{s}</p> })}
            <p class="cdt-travail__detail">{detail}</p>
            <div class="cdt-form">
                <Selection id=format!("prog-cours-{id}") libelle="Cours" valeur=cours_choisi options=options/>
                <ChampTexte id=format!("prog-jusqu-{id}") libelle="Répartir jusqu'au (facultatif)" valeur=jusqu genre="date"/>
            </div>
            <div class="cdt-actions">
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(move |_| planifier(true))>"Aperçu de la planification"</Bouton>
                <Bouton taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(move |_| planifier(false))>"Planifier sur les séances libres"</Bouton>
                {mienne.then(|| view! { <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(partager)>"Partager avec ce cours"</Bouton> })}
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(reconduire)>"Reconduire (copie sans dates)"</Bouton>
            </div>
        </article>
    }
}

/// Copie des entrées d'un ancien cours vers un nouveau : brouillons, les vôtres seulement, aucune donnée d'élève.
#[component]
fn Copie(cours: Vec<CoursSaisieDto>, ecriture: Ecriture) -> impl IntoView {
    let options: Vec<(String, String)> = cours
        .iter()
        .map(|c| {
            (
                c.cours_id.to_string(),
                format!("{} — {}", c.libelle, c.cible),
            )
        })
        .collect();
    let source = RwSignal::new(
        cours
            .first()
            .map(|c| c.cours_id.to_string())
            .unwrap_or_default(),
    );
    let cible = RwSignal::new(
        cours
            .get(1)
            .or(cours.first())
            .map(|c| c.cours_id.to_string())
            .unwrap_or_default(),
    );
    let copier = move |apercu: bool| {
        let (Ok(s), Ok(c)) = (
            Uuid::parse_str(&source.get_untracked()),
            Uuid::parse_str(&cible.get_untracked()),
        ) else {
            ecriture.erreur.set(Some(
                "Choisissez le cours d'origine et le cours de destination.".into(),
            ));
            return;
        };
        let b = CopierCoursBody {
            source_cours_id: s,
            cible_cours_id: c,
            a_partir_du: None,
            apercu,
        };
        ecriture.lancer(
            async move { Ok(phrase_copie(&session::client().cdt_cours_copier(&b).await?)) },
        );
    };
    view! {
        <section class="cdt-seance" aria-label="Copier d'un cours à l'autre">
            <h2 class="cdt-travail__titre">"Copier d'un cours à l'autre"</h2>
            <p class="cdt-resume">"Vos entrées du cours d'origine sont copiées en brouillon sur les prochaines séances libres du cours de destination. Ni remises, ni travaux, ni élèves."</p>
            <div class="cdt-form">
                <Selection id="copie-source" libelle="Cours d'origine" valeur=source options=options.clone()/>
                <Selection id="copie-cible" libelle="Cours de destination" valeur=cible options=options/>
            </div>
            <div class="cdt-actions">
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(move |_| copier(true))>"Aperçu"</Bouton>
                <Bouton taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(move |_| copier(false))>"Copier en brouillon"</Bouton>
            </div>
        </section>
    }
}

#[component]
fn Nouvelle(ecriture: Ecriture) -> impl IntoView {
    let titre = RwSignal::new(String::new());
    let etapes = RwSignal::new(String::new());
    let creer = move |_| match corps_progression(&titre.get_untracked(), &etapes.get_untracked()) {
        Err(m) => ecriture.erreur.set(Some(m)),
        Ok(b) => ecriture.lancer(async move {
            session::client().cdt_progression_creer(&b).await?;
            Ok(
                "Progression créée : elle reste privée tant que vous ne la partagez pas."
                    .to_string(),
            )
        }),
    };
    view! {
        <section class="cdt-seance" aria-label="Nouvelle progression">
            <h2 class="cdt-travail__titre">"Nouvelle progression"</h2>
            <div class="cdt-form">
                <ChampTexte id="prog-titre" libelle="Titre" valeur=titre requis=true/>
                <ZoneTexte id="prog-etapes" libelle="Étapes, une par ligne : titre | notions | durée en séances" valeur=etapes requis=true aide="Exemple : Les fractions | définition ; comparaison | 3"/>
                <Bouton en_cours=ecriture.en_cours on_click=Callback::new(creer)>"Créer la progression"</Bouton>
            </div>
        </section>
    }
}
