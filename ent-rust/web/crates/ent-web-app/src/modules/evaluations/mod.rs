//! Évaluations (socle 2, lot S2-7) : accueil, grille professeur, vue de classe, préparation
//! du conseil, bulletins et exports, consultation famille / élève.
//!
//! Routes d'`ent-api/src/evaluations.rs`. Le serveur décide des droits et calcule toutes les
//! moyennes (arrondies selon la règle de l'établissement) : l'écran affiche et n'envoie que
//! les modifications. Les boutons d'écriture suivent les droits effectifs du moteur
//! (`use_droits_moteur`), le serveur revérifie chaque action.

mod bulletins;
mod classe;
mod conseil;
mod famille;
mod grille;
pub mod modele;

pub use bulletins::PageBulletins;
pub use classe::PageVueClasse;
pub use conseil::PageConseil;
pub use famille::PageEvaluationsEleve;
pub use grille::PageGrille;

use std::collections::BTreeMap;

use chrono::NaiveTime;
use ent_types::evaluations::{CreerConseilBody, PeriodeNotesDto, ServiceEvaluationDto};
use ent_types::referentiel::EnseignementDto;
use ent_web_ui::{
    Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Selection, Squelette, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::coquille::{use_droits, use_droits_moteur};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use modele::{libelle_statut_periode, periode_par_defaut, services_hors_enseignements};

/// Racine des écrans d'évaluation.
pub const RACINE: &str = "/web/evaluations";

pub fn lien_grille(service: Uuid, periode: Uuid) -> String {
    format!("{RACINE}/grille?service={service}&periode={periode}")
}

pub fn lien_classe(classe: Uuid, periode: Uuid) -> String {
    format!("{RACINE}/classes/{classe}?periode={periode}")
}

pub fn lien_bulletins(classe: Uuid, periode: Uuid) -> String {
    format!("{RACINE}/bulletins?classe={classe}&periode={periode}")
}

pub fn lien_conseil(conseil: Uuid) -> String {
    format!("{RACINE}/conseils/{conseil}")
}

pub fn lien_eleve(eleve: Uuid) -> String {
    format!("{RACINE}/eleves/{eleve}")
}

fn ton_periode(statut: &str) -> TonEtat {
    match statut {
        "ouverte" => TonEtat::Ok,
        "fermee" => TonEtat::Mal,
        _ => TonEtat::Avertissement,
    }
}

/// Badge du statut (verrou) d'une période.
#[component]
pub fn BadgePeriode(#[prop(into)] statut: String) -> impl IntoView {
    let ton = ton_periode(&statut);
    view! { <Badge libelle=libelle_statut_periode(&statut) ton=ton/> }
}

/// Choix de la période de notes. La valeur par défaut (période du jour) est posée après
/// l'hydratation : aucune date ne dépend de l'horloge dans le HTML du serveur.
#[component]
pub fn ChoixPeriode(valeur: RwSignal<String>, periodes: Vec<PeriodeNotesDto>) -> impl IntoView {
    let liste = StoredValue::new(periodes.clone());
    Effect::new(move |_| {
        if valeur.get_untracked().is_empty() {
            if let Some(id) = liste.with_value(|l| periode_par_defaut(l, aujourdhui())) {
                valeur.set(id.to_string());
            }
        }
    });
    let options: Vec<(String, String)> = std::iter::once((String::new(), "Choisir…".to_string()))
        .chain(periodes.iter().map(|p| {
            (
                p.id.to_string(),
                format!("{} ({})", p.libelle, libelle_statut_periode(&p.statut)),
            )
        }))
        .collect();
    view! { <Selection id="eva-periode" libelle="Période" valeur=valeur options=options/> }
}

#[component]
pub fn PageEvaluations() -> impl IntoView {
    let droits = use_droits();
    view! {
        <Title text="Évaluations"/>
        <EnTetePage
            surtitre="Pédagogie".to_string()
            titre="Évaluations".to_string()
            sous_titre="Notes, appréciations, conseils de classe et bulletins.".to_string()
        />
        {move || {
            let d = droits.get();
            if d.personnel {
                view! { <AccueilPersonnel/> }.into_any()
            } else if d.portail_famille {
                view! { <AccueilFamille/> }.into_any()
            } else {
                view! { <AccueilEleve/> }.into_any()
            }
        }}
    }
}

#[component]
fn AccueilFamille() -> impl IntoView {
    let enfants = LocalResource::new(|| async { session::client().enfants_famille().await });
    view! {
        {move || match enfants.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de vos enfants…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| enfants.refetch())/> }.into_any(),
            Some(Ok(l)) if l.iter().all(|e| !e.peut_voir_notes) => view! {
                <EtatVide titre="Aucun accès aux notes" message="L'établissement ne vous a pas ouvert la consultation des notes."/>
            }
            .into_any(),
            Some(Ok(l)) => view! {
                <ul class="jvs-liste">
                    {l.into_iter()
                        .filter(|e| e.peut_voir_notes)
                        .map(|e| view! {
                            <li class="jvs-ligne">
                                <span>{format!("{} {}", e.prenom, e.nom)}</span>
                                <a href=lien_eleve(e.id)>"Notes et bulletins"</a>
                            </li>
                        })
                        .collect_view()}
                </ul>
            }
            .into_any(),
        }}
    }
}

#[component]
fn AccueilEleve() -> impl IntoView {
    let ctx = session::use_session();
    view! {
        {move || match ctx.moi.get() {
            Some(Ok(m)) => match m.eleve_id {
                Some(id) => view! { <p><a href=lien_eleve(id)>"Mes notes et bulletins"</a></p> }.into_any(),
                None => view! { <EtatVide titre="Aucune fiche liée" message="Ce compte n'est relié à aucune fiche élève."/> }.into_any(),
            },
            _ => view! { <Squelette lignes=2 libelle="Chargement…"/> }.into_any(),
        }}
    }
}

#[component]
fn AccueilPersonnel() -> impl IntoView {
    let periodes = LocalResource::new(|| async { session::client().periodes_notes().await });
    let services = LocalResource::new(|| async { session::client().services_evaluables().await });
    // Liste unique « mes enseignements » (classes entières et groupes, avec effectif).
    let enseignements = LocalResource::new(|| async {
        session::client()
            .mes_enseignements(aujourdhui())
            .await
            .unwrap_or_default()
    });
    let choix = RwSignal::new(String::new());
    view! {
        {move || match periodes.get() {
            None => view! { <Squelette lignes=2 libelle="Chargement des périodes…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| periodes.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune période de notes" message="La direction n'a pas encore créé de période."/>
            }
            .into_any(),
            Some(Ok(l)) => view! {
                <div class="jvs-barre"><ChoixPeriode valeur=choix periodes=l.clone()/></div>
                <PanneauPeriode periodes=l choix=choix on_change=Callback::new(move |_| periodes.refetch())/>
            }
            .into_any(),
        }}
        {move || match (services.get(), Uuid::parse_str(&choix.get()).ok()) {
            (None, _) => view! { <Squelette lignes=4 libelle="Chargement des services…"/> }.into_any(),
            (Some(Err(e)), _) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| services.refetch())/> }.into_any(),
            (Some(Ok(_)), None) => view! { <EtatVide titre="Choisissez une période" message="Les grilles s'ouvrent pour une période."/> }.into_any(),
            (Some(Ok(s)), Some(p)) => view! { <ListesPersonnel services=s enseignements=enseignements.get().unwrap_or_default() periode=p/> }.into_any(),
        }}
        <SectionConseils/>
    }
}

/// Statut de la période choisie et, pour la direction, son changement (EVA-07).
#[component]
fn PanneauPeriode(
    periodes: Vec<PeriodeNotesDto>,
    choix: RwSignal<String>,
    on_change: Callback<()>,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let liste = StoredValue::new(periodes);
    let ecriture = Ecriture::nouveau(on_change);
    let courante = Memo::new(move |_| {
        let id = Uuid::parse_str(&choix.get()).ok()?;
        liste.with_value(|l| l.iter().find(|p| p.id == id).cloned())
    });
    let changer = move |statut: &'static str| {
        Callback::new(move |_| {
            if let Some(p) = courante.get_untracked() {
                ecriture.lancer(async move {
                    let r = session::client()
                        .changer_statut_periode(p.id, statut)
                        .await?;
                    Ok(format!(
                        "Période « {} » : {}.",
                        r.libelle,
                        libelle_statut_periode(&r.statut)
                    ))
                });
            }
        })
    };
    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || courante.get().map(|p| {
            let peut = moteur.get().a_etablissement("periodes.verrouiller");
            view! {
                <p class="eva-periode">
                    <BadgePeriode statut=p.statut.clone()/>
                    " "{format!("du {} au {}", p.date_debut.format("%d/%m/%Y"), p.date_fin.format("%d/%m/%Y"))}
                </p>
                {peut.then(|| view! {
                    <div class="eva-actions" role="group" aria-label="Verrou de la période">
                        <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=changer("ouverte")>"Ouvrir"</Bouton>
                        <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=changer("partielle_appreciations")>"Verrouiller les notes"</Bouton>
                        <Bouton variante=Variante::Danger en_cours=ecriture.en_cours on_click=changer("fermee")>"Fermer la période"</Bouton>
                    </div>
                })}
            }
        })}
    }
}

#[component]
fn ListesPersonnel(
    services: Vec<ServiceEvaluationDto>,
    enseignements: Vec<EnseignementDto>,
    periode: Uuid,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    // Classes (dédoublonnées) issues des services : vue de classe et bulletins.
    let mut classes: BTreeMap<String, Uuid> = BTreeMap::new();
    for s in &services {
        if let Some(c) = s.classe_id {
            classes.insert(s.cible_code.clone(), c);
        }
    }
    let classes: Vec<(String, Uuid)> = classes.into_iter().collect();
    let autres = services_hors_enseignements(&services, &enseignements);
    view! {
        {(!enseignements.is_empty()).then(|| view! {
            <section class="ra-section" aria-labelledby="eva-enseignements">
                <h2 id="eva-enseignements" class="vs-section-titre">"Mes enseignements"</h2>
                <ul class="jvs-liste">
                    {enseignements.iter().map(|e| view! {
                        <li class="jvs-ligne">
                            <span>
                                <strong>{e.libelle.clone()}</strong>
                                " · "{match e.effectif { 1 => "1 élève".to_string(), n => format!("{n} élèves") }}
                                {e.professeur_principal.then(|| view! { " " <Badge libelle="Professeur principal" ton=TonEtat::Info/> })}
                            </span>
                            <a href=lien_grille(e.service_id, periode)>"Ouvrir la grille"</a>
                        </li>
                    }).collect_view()}
                </ul>
            </section>
        })}
        <section class="ra-section" aria-labelledby="eva-services">
            <h2 id="eva-services" class="vs-section-titre">{if enseignements.is_empty() { "Grilles de notes" } else { "Autres grilles de l'établissement" }}</h2>
            {let services = autres;
             if services.is_empty() && !enseignements.is_empty() {
                ().into_any()
            } else if services.is_empty() {
                view! { <EtatVide titre="Aucun service" message="Aucun enseignement ne vous est rattaché cette année."/> }.into_any()
            } else {
                view! {
                    <ul class="jvs-liste">
                        {services.into_iter().map(|s| view! {
                            <li class="jvs-ligne">
                                <span>
                                    <strong>{s.cible_code.clone()}</strong>" · "{s.matiere_libelle.clone()}
                                    {s.professeur_principal.then(|| view! { " " <Badge libelle="Professeur principal" ton=TonEtat::Info/> })}
                                </span>
                                <a href=lien_grille(s.service_id, periode)>"Ouvrir la grille"</a>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>
        {(!classes.is_empty()).then(|| view! {
            <section class="ra-section" aria-labelledby="eva-classes">
                <h2 id="eva-classes" class="vs-section-titre">"Classes"</h2>
                <ul class="jvs-liste">
                    {classes.into_iter().map(|(code, id)| view! {
                        <li class="jvs-ligne">
                            <strong>{code}</strong>
                            <span class="eva-liens">
                                <a href=lien_classe(id, periode)>"Moyennes de la classe"</a>
                                {move || moteur.get().a_etablissement("bulletins.publier").then(|| view! {
                                    " · "<a href=lien_bulletins(id, periode)>"Bulletins et livrets"</a>
                                })}
                            </span>
                        </li>
                    }).collect_view()}
                </ul>
            </section>
        })}
    }
}

/// Conseils de classe rattachés à une période (S2-7) et création (direction ou PP).
#[component]
fn SectionConseils() -> impl IntoView {
    let moteur = use_droits_moteur();
    let conseils = LocalResource::new(|| async { session::client().conseils_evaluation().await });
    view! {
        <section class="ra-section" aria-labelledby="eva-conseils">
            <h2 id="eva-conseils" class="vs-section-titre">"Conseils de classe"</h2>
            {move || match conseils.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des conseils…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| conseils.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun conseil" message="Aucun conseil planifié pour vos classes."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|c| view! {
                            <li class="jvs-ligne">
                                <span>
                                    <strong>{c.classe.clone()}</strong>" · "{c.periode_libelle.clone()}" · "
                                    {format!("{} à {}", c.jour.format("%d/%m/%Y"), c.heure.format("%H:%M"))}
                                </span>
                                <a href=lien_conseil(c.id)>"Préparer le conseil"</a>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
            {move || moteur.get().a("conseils.gerer").then(|| view! {
                <NouveauConseil on_cree=Callback::new(move |_| conseils.refetch())/>
            })}
        </section>
    }
}

#[component]
fn NouveauConseil(on_cree: Callback<()>) -> impl IntoView {
    let periodes = LocalResource::new(|| async { session::client().periodes_notes().await });
    let services = LocalResource::new(|| async { session::client().services_evaluables().await });
    let (classe, periode, jour, heure, lieu) = (
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new("17:00".to_string()),
        RwSignal::new(String::new()),
    );
    let ecriture = Ecriture::nouveau(on_cree);
    let creer = Callback::new(move |_| {
        let (Ok(c), Ok(p), Ok(j), Ok(h)) = (
            Uuid::parse_str(&classe.get_untracked()),
            Uuid::parse_str(&periode.get_untracked()),
            chrono::NaiveDate::parse_from_str(&jour.get_untracked(), "%Y-%m-%d"),
            NaiveTime::parse_from_str(&heure.get_untracked(), "%H:%M"),
        ) else {
            ecriture.erreur.set(Some(
                "Classe, période, date et heure sont obligatoires.".into(),
            ));
            return;
        };
        let body = CreerConseilBody {
            classe_id: c,
            periode_id: p,
            jour: j,
            heure: h,
            lieu: lieu.get_untracked(),
        };
        ecriture.lancer(async move {
            let c = session::client().creer_conseil(&body).await?;
            Ok(format!("Conseil de {} planifié.", c.classe))
        });
    });
    view! {
        <details class="eva-details">
            <summary>"Planifier un conseil"</summary>
            <RetourEcriture ecriture=ecriture/>
            {move || match (periodes.get(), services.get()) {
                (Some(Ok(p)), Some(Ok(s))) => {
                    let mut classes: BTreeMap<String, Uuid> = BTreeMap::new();
                    for x in &s {
                        if let Some(c) = x.classe_id {
                            classes.insert(x.cible_code.clone(), c);
                        }
                    }
                    let options_c: Vec<(String, String)> = std::iter::once((String::new(), "Choisir…".to_string()))
                        .chain(classes.into_iter().map(|(code, id)| (id.to_string(), code)))
                        .collect();
                    view! {
                        <div class="eva-formulaire">
                            <Selection id="cons-classe" libelle="Classe" valeur=classe options=options_c requis=true/>
                            <ChoixPeriode valeur=periode periodes=p/>
                            <ChampTexte id="cons-jour" libelle="Date" valeur=jour genre="date" requis=true/>
                            <ChampTexte id="cons-heure" libelle="Heure" valeur=heure genre="time" requis=true/>
                            <ChampTexte id="cons-lieu" libelle="Lieu" valeur=lieu/>
                            <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=creer>"Planifier"</Bouton>
                        </div>
                    }
                    .into_any()
                }
                (Some(Err(e)), _) | (_, Some(Err(e))) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| { periodes.refetch(); services.refetch(); })/> }.into_any(),
                _ => view! { <Squelette lignes=2 libelle="Chargement…"/> }.into_any(),
            }}
        </details>
    }
}
