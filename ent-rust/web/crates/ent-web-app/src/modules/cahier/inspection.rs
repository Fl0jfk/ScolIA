//! Écran 8 du socle 20 : **accès d'inspection** (direction, `/web/cahier/inspection`) et **journal de consultation** (le professeur
//! concerné y lit qui a lu son cahier et quand) ; lecture de l'inspecteur (`/web/cahier/inspection/lecture`).
//!
//! L'inspecteur voit exactement ce qu'on lui ouvre, pour une durée bornée : entrées et consignes publiées, jamais de remise, de retour
//! ni de nom d'élève ; chaque lecture est journalisée avant d'être servie. Bornes, droits et journal sont décidés par le serveur.

use ent_web_api::cahier_c8_types::{
    AccesInspectionDto, CahierInspectionDto, ConsultationDto, CoursRefDto,
};
use ent_web_ui::{
    AccesRefuse, Badge, Bouton, Case, ChampTexte, EnTetePage, EtatVide, LienBouton, Selection,
    Squelette, Taille, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele_c8::{
    corps_acces, etat_acces, libelle_motif, periode_par_defaut, phrase_validite,
};
use super::tenue_visa::ton;
use crate::noyau::coquille::{use_droits, use_droits_moteur};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_heure_paris, date_longue};

pub const ADRESSE_INSPECTION: &str = "/web/cahier/inspection";
pub const ADRESSE_LECTURE_INSPECTION: &str = "/web/cahier/inspection/lecture";

#[component]
pub fn PageInspection() -> impl IntoView {
    let droits = use_droits();
    let moteur = use_droits_moteur();
    view! {
        <Title text="Accès d'inspection"/>
        {move || {
            let direction = moteur.get().a("cahier.acces_inspection");
            if direction || droits.get().cahier_ecrire {
                view! { <Contenu direction=direction/> }.into_any()
            } else {
                view! { <AccesRefuse message="Les accès d'inspection sont gérés par la direction ; le professeur concerné lit son journal."/> }.into_any()
            }
        }}
    }
}

#[component]
fn Contenu(direction: bool) -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().cdt_acces_inspection().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    let journal: RwSignal<Option<Vec<ConsultationDto>>> = RwSignal::new(None);
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Accès d'inspection".to_string()
            sous_titre="Qui peut lire quels cours, jusqu'à quand ; chaque lecture est journalisée et visible du professeur concerné.".to_string()
        >
            {direction.then(|| view! { <LienBouton href=super::ADRESSE_VISA variante=Variante::Secondaire taille=Taille::Compacte>"Tenue et visa"</LienBouton> })}
        </EnTetePage>
        <RetourEcriture ecriture=ecriture/>
        {move || match liste.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des accès…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun accès" message="Aucun inspecteur n'a d'accès au cahier."/> }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="cdt-liste">{l.into_iter().map(|a| view! { <li><LigneAcces a=a direction=direction ecriture=ecriture journal=journal/></li> }).collect_view()}</ul>
            }.into_any(),
        }}
        {move || journal.get().map(|j| view! { <Journal j=j/> })}
        {direction.then(|| view! { <Octroi ecriture=ecriture/> })}
    }
}

#[component]
fn LigneAcces(
    a: AccesInspectionDto,
    direction: bool,
    ecriture: Ecriture,
    journal: RwSignal<Option<Vec<ConsultationDto>>>,
) -> impl IntoView {
    let id = a.id;
    let (etat, t) = etat_acces(&a.etat);
    let actif = a.etat == "actif" || a.etat == "a_venir";
    let cours = a
        .cours
        .iter()
        .map(|c| format!("{} ({})", c.libelle, c.public))
        .collect::<Vec<_>>()
        .join(", ");
    let qui = format!("{} — {}", a.beneficiaire, libelle_motif(&a.motif));
    let validite = phrase_validite(&a);
    let cible = a.agent_cible.clone().map(|n| format!("Rendez-vous de {n}"));
    let ouvrir_journal = move |_| {
        ecriture.lancer(async move {
            let j = session::client().cdt_acces_journal(id).await?;
            journal.set(Some(j));
            Ok("Journal ouvert.".to_string())
        });
    };
    let prolonger = move |_| {
        ecriture.lancer(async move {
            session::client().cdt_acces_prolonger(id, 30).await?;
            Ok("Accès prolongé de 30 jours.".to_string())
        });
    };
    let revoquer = move |_| {
        ecriture.lancer(async move {
            session::client().cdt_acces_revoquer(id).await?;
            Ok("Accès révoqué : effet immédiat.".to_string())
        });
    };
    view! {
        <article class="cdt-travail">
            <header class="cdt-travail__entete">
                <p class="cdt-travail__titre">{qui}</p>
                <Badge libelle=etat ton=ton(t)/>
            </header>
            {cible.map(|c| view! { <p class="cdt-travail__detail">{c}</p> })}
            <p class="cdt-travail__detail">{format!("Cours ouverts : {cours}")}</p>
            <p class="cdt-travail__detail">{validite}</p>
            <div class="cdt-actions">
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(ouvrir_journal)>"Journal de consultation"</Bouton>
                {direction.then(|| view! {
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(prolonger)>"Prolonger de 30 jours"</Bouton>
                })}
                {(direction && actif).then(|| view! {
                    <Bouton variante=Variante::Danger taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(revoquer)>"Révoquer"</Bouton>
                })}
            </div>
        </article>
    }
}

#[component]
fn Journal(j: Vec<ConsultationDto>) -> impl IntoView {
    view! {
        <section class="cdt-seance" aria-label="Journal de consultation">
            <h2 class="cdt-travail__titre">"Journal de consultation"</h2>
            {if j.is_empty() {
                view! { <p class="cdt-resume">"Aucune lecture pour l'instant."</p> }.into_any()
            } else {
                view! {
                    <ul class="cdt-liste">
                        {j.into_iter().map(|c| view! { <li>{format!("{} — {} a lu {}", date_heure_paris(c.le), c.qui, c.cours.unwrap_or_else(|| "un cours".into()))}</li> }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

/// Formulaire d'octroi (direction) : compte, cours, période du cahier, durée, motif.
#[component]
fn Octroi(ecriture: Ecriture) -> impl IntoView {
    // F11 : les cours viennent du droit d'octroi lui-même, pas du tableau de tenue (qui exige `cahier.viser`).
    let tenue_cours =
        LocalResource::new(|| async { session::client().cdt_cours_ouvrables().await });
    let compte = RwSignal::new(String::new());
    let (d0, a0) = periode_par_defaut(aujourdhui());
    let du = RwSignal::new(d0.to_string());
    let au = RwSignal::new(a0.to_string());
    let duree = RwSignal::new("30".to_string());
    let motif = RwSignal::new("inspection".to_string());
    let cochees: RwSignal<Vec<(Uuid, RwSignal<bool>)>> = RwSignal::new(Vec::new());
    let accorder = move |_| {
        let choisis: Vec<Uuid> = cochees
            .get_untracked()
            .iter()
            .filter(|(_, c)| c.get_untracked())
            .map(|(i, _)| *i)
            .collect();
        match corps_acces(
            &compte.get_untracked(),
            choisis,
            &du.get_untracked(),
            &au.get_untracked(),
            &duree.get_untracked(),
            &motif.get_untracked(),
        ) {
            Err(m) => ecriture.erreur.set(Some(m)),
            Ok(b) => ecriture.lancer(async move {
                session::client().cdt_acces_accorder(&b).await?;
                Ok("Accès accordé et journalisé. Aucun avis n'est encore envoyé au professeur concerné (socle 8) : il retrouve cet accès, et chaque lecture, dans son journal d'inspection. Prévenez-le vous-même.".to_string())
            }),
        }
    };
    view! {
        <section class="cdt-seance" aria-label="Accorder un accès">
            <h2 class="cdt-travail__titre">"Accorder un accès"</h2>
            <div class="cdt-form">
                <ChampTexte id="insp-compte" libelle="Identifiant du compte invité de l'inspecteur" valeur=compte requis=true/>
                <Selection id="insp-motif" libelle="Motif" valeur=motif options=vec![("inspection".to_string(), "Inspection".to_string()), ("rdv_carriere".to_string(), "Rendez-vous de carrière".to_string()), ("autre".to_string(), "Autre".to_string())]/>
                <p class="cdt-resume">"Pour un rendez-vous de carrière, prévenez vous-même le professeur : l'avis automatique (socle 8) n'est pas encore branché, il est un prérequis à l'usage courant de ce motif."</p>
                <ChampTexte id="insp-du" libelle="Cahier : du" valeur=du genre="date"/>
                <ChampTexte id="insp-au" libelle="Cahier : au" valeur=au genre="date"/>
                <ChampTexte id="insp-duree" libelle="Durée de l'accès en jours (90 au plus)" valeur=duree genre="number"/>
                {move || match tenue_cours.get() {
                    Some(Ok(l)) => {
                        let liste: Vec<(CoursRefDto, RwSignal<bool>)> = l.into_iter().map(|x| (x, RwSignal::new(false))).collect();
                        cochees.set(liste.iter().map(|(c, s)| (c.cours_id, *s)).collect());
                        view! {
                            <fieldset class="cdt-form">
                                <legend>"Cours à ouvrir"</legend>
                                {liste.into_iter().map(|(c, s)| view! { <Case id=format!("insp-c-{}", c.cours_id) libelle=format!("{} — {}", c.libelle, c.public) coche=s/> }).collect_view()}
                            </fieldset>
                        }.into_any()
                    }
                    Some(Err(_)) => ().into_any(),
                    None => view! { <Squelette lignes=2 libelle="Chargement des cours…"/> }.into_any(),
                }}
                <Bouton en_cours=ecriture.en_cours on_click=Callback::new(accorder)>"Accorder l'accès"</Bouton>
            </div>
        </section>
    }
}

/// Lecture de l'inspecteur : ses accès, puis le cours choisi (chaque ouverture est journalisée).
#[component]
pub fn PageLectureInspection() -> impl IntoView {
    let acces = LocalResource::new(|| async { session::client().cdt_mes_acces_inspection().await });
    let lecture: RwSignal<Option<CahierInspectionDto>> = RwSignal::new(None);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| acces.refetch()));
    view! {
        <Title text="Lecture d'inspection"/>
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Lecture d'inspection".to_string()
            sous_titre="Vous lisez les cours qui vous ont été ouverts, pour une durée bornée. Chaque lecture est enregistrée.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match acces.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de vos accès…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| acces.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun accès" message="Aucun cahier ne vous est ouvert."/> }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="cdt-liste">
                    {l.into_iter().map(|a| {
                        let (etat, t) = etat_acces(&a.etat);
                        let actif = a.etat == "actif";
                        let validite = phrase_validite(&a);
                        let id = a.id;
                        view! {
                            <li class="cdt-travail">
                                <header class="cdt-travail__entete">
                                    <p class="cdt-travail__titre">{libelle_motif(&a.motif)}</p>
                                    <Badge libelle=etat ton=ton(t)/>
                                </header>
                                <p class="cdt-travail__detail">{validite}</p>
                                {actif.then(|| view! {
                                    <div class="cdt-actions">
                                        {a.cours.iter().map(|c| {
                                            let cid = c.cours_id;
                                            let aria = format!("Lire le cahier de {}", c.libelle);
                                            let ouvrir = move |_| ecriture.lancer(async move {
                                                let r = session::client().cdt_inspection_lire(id, cid).await?;
                                                lecture.set(Some(r));
                                                Ok("Cahier ouvert : cette lecture est journalisée.".to_string())
                                            });
                                            view! { <Bouton taille=Taille::Compacte libelle_accessible=aria en_cours=ecriture.en_cours on_click=Callback::new(ouvrir)>{format!("{} ({})", c.libelle, c.public)}</Bouton> }
                                        }).collect_view()}
                                    </div>
                                })}
                            </li>
                        }
                    }).collect_view()}
                </ul>
            }.into_any(),
        }}
        {move || lecture.get().map(|c| view! { <Cahier c=c/> })}
    }
}

#[component]
fn Cahier(c: CahierInspectionDto) -> impl IntoView {
    view! {
        <section class="cdt-seance" aria-label="Cahier du cours">
            <h2 class="cdt-travail__titre">{format!("{} — {}", c.cours.libelle, c.cours.public)}</h2>
            <p class="cdt-travail__detail">{format!("Du {} au {} · accès jusqu'au {}", date_longue(c.du), date_longue(c.au), date_longue(c.valable_au.date_naive()))}</p>
            <h3 class="cdt-travail__titre">"Séances"</h3>
            {c.entrees.into_iter().map(|e| view! {
                <article class="cdt-travail">
                    <p class="cdt-travail__titre">{format!("{} · {} · {}", date_longue(e.jour), e.titre.unwrap_or_else(|| "(sans titre)".into()), e.auteur)}</p>
                    <p class="cdt-travail__consigne">{e.contenu}</p>
                    {(!e.notions.is_empty()).then(|| view! { <p class="cdt-travail__detail">{format!("Notions : {}", e.notions.join(" ; "))}</p> })}
                </article>
            }).collect_view()}
            <h3 class="cdt-travail__titre">"Travaux donnés"</h3>
            {c.travaux.into_iter().map(|t| view! {
                <article class="cdt-travail">
                    <p class="cdt-travail__titre">{format!("{} · {} · pour le {}", date_longue(t.donne_le), t.titre, date_longue(t.echeance_jour))}</p>
                    <p class="cdt-travail__consigne">{t.consigne}</p>
                    <p class="cdt-travail__detail">{format!("{}{}", t.nature, t.duree_estimee_min.map(|d| format!(" · {d} min")).unwrap_or_default())}</p>
                </article>
            }).collect_view()}
        </section>
    }
}
