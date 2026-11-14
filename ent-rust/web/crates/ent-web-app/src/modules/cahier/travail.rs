//! Écran 2 du lot C2 : **donner un travail** en trois gestes (cours, contenu, travail), pour un ou plusieurs
//! cours, avec nature, durée proposée, échéance (prochaine séance par défaut), rendu, usage de l'IA,
//! brouillon, publication ou programmation.
//!
//! Route `/web/cahier/travail?cours=<id>&seance=<id>`. Le serveur contrôle l'échéance (après le don, dans l'année,
//! jour ouvré, hors plage calme), les droits à la date et renvoie les avertissements non bloquants.
//! Limite : le public restreint à une liste d'élèves existe côté API (`eleve_ids`) ; l'écran ne le propose pas
//! encore (aucune liste d'élèves d'un cours exposée au client).

use ent_web_api::cahier_c2_types::{NatureTravailDto, TravauxCreesDto};
use ent_web_ui::{
    AccesRefuse, Alerte, Bouton, Case, ChampTexte, EnTetePage, EtatVide, GroupeChoix, LienBouton,
    Niveau, Selection, Squelette, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use super::bandeau_charge::BandeauCharge;
use super::cours::ADRESSE_COURS;
use super::modele::Erreurs;
use super::modele_c2::{message_avertissement, valider_travail_c2, SaisieTravailC2};
use crate::noyau::coquille::use_droits;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn PageDonnerTravail() -> impl IntoView {
    let droits = use_droits();
    view! {
        <Title text="Donner un travail"/>
        {move || {
            if droits.get().cahier_ecrire {
                view! { <ContenuDonner/> }.into_any()
            } else {
                view! { <AccesRefuse message="Donner un travail est réservé aux enseignants."/> }.into_any()
            }
        }}
    }
}

#[component]
fn ContenuDonner() -> impl IntoView {
    let query = use_query_map();
    let cours_demande = Memo::new(move |_| {
        query
            .with(|q| q.get("cours"))
            .and_then(|c| Uuid::parse_str(c.trim()).ok())
    });
    let seance_demandee = Memo::new(move |_| {
        query
            .with(|q| q.get("seance"))
            .and_then(|c| Uuid::parse_str(c.trim()).ok())
    });
    let cours = LocalResource::new(|| async { session::client().cdt_cours().await });
    let natures = LocalResource::new(|| async { session::client().cdt_natures().await });
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Donner un travail".to_string()
            sous_titre="Par défaut, le travail est pour la prochaine séance du cours. Rien n'est visible des élèves avant la publication.".to_string()
        >
            <LienBouton href=ADRESSE_COURS variante=Variante::Secondaire>"Retour au cahier du cours"</LienBouton>
        </EnTetePage>
        {move || match (cours.get(), natures.get()) {
            (None, _) | (_, None) => view! { <Squelette lignes=4 libelle="Chargement…"/> }.into_any(),
            (Some(Err(e)), _) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| cours.refetch())/> }.into_any(),
            (_, Some(Err(e))) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| natures.refetch())/> }.into_any(),
            (Some(Ok(c)), Some(Ok(_))) if c.is_empty() => view! {
                <EtatVide titre="Aucun cours" message="Vous n'êtes intervenant en saisie sur aucun cours aujourd'hui."/>
            }.into_any(),
            (Some(Ok(c)), Some(Ok(n))) => view! {
                <FormulaireDonner cours=c natures=n cours_initial=cours_demande.get() seance=seance_demandee.get()/>
            }.into_any(),
        }}
    }
}

fn poser(es: &Erreurs, cibles: &[(&'static str, RwSignal<Option<String>>)]) {
    for (champ, signal) in cibles {
        signal.set(es.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()));
    }
}

#[component]
fn FormulaireDonner(
    cours: Vec<ent_web_api::cahier_c2_types::CoursSaisieDto>,
    natures: Vec<NatureTravailDto>,
    cours_initial: Option<Uuid>,
    seance: Option<Uuid>,
) -> impl IntoView {
    let choisis: Vec<(Uuid, RwSignal<bool>)> = cours
        .iter()
        .map(|c| (c.cours_id, RwSignal::new(cours_initial == Some(c.cours_id))))
        .collect();
    if !choisis.iter().any(|(_, s)| s.get_untracked()) {
        if let Some((_, s)) = choisis.first() {
            s.set(true);
        }
    }
    let nature = RwSignal::new(natures.first().map(|n| n.code.clone()).unwrap_or_default());
    let duree_proposee: Vec<(String, Option<i16>)> = natures
        .iter()
        .map(|n| (n.code.clone(), n.duree_defaut_min))
        .collect();
    let options_nature: Vec<(String, String)> = natures
        .iter()
        .map(|n| (n.code.clone(), n.libelle.clone()))
        .collect();
    let titre = RwSignal::new(String::new());
    let consigne = RwSignal::new(String::new());
    let duree = RwSignal::new(String::new());
    let echeance = RwSignal::new("prochaine".to_string());
    let jour = RwSignal::new(String::new());
    let heure = RwSignal::new(String::new());
    let rendu = RwSignal::new("aucun".to_string());
    let retard = RwSignal::new(true);
    let formats = RwSignal::new(String::new());
    let ia = RwSignal::new("non".to_string());
    let publication = RwSignal::new("maintenant".to_string());
    let publie_jour = RwSignal::new(String::new());
    let publie_heure = RwSignal::new(String::new());
    let (e_cours, e_titre, e_consigne, e_nature, e_duree, e_jour, e_heure, e_pj, e_ph) = (
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
    );
    let resultat = RwSignal::new(None::<TravauxCreesDto>);
    // Le bandeau de charge (C4) signale ici qu'il affichait une alerte : enregistrer = passer outre, la dérogation est comptée.
    let alerte_active = RwSignal::new(false);
    let bandeau_cours = choisis.clone();
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let proposition = move || {
        let code = nature.get();
        duree_proposee
            .iter()
            .find(|(c, _)| *c == code)
            .and_then(|(_, d)| *d)
            .map(|d| {
                format!(
                    "Durée proposée pour cette nature : {d} min (laissez vide pour la retenir)."
                )
            })
    };
    let envoyer = {
        let choisis = choisis.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            let saisie = SaisieTravailC2 {
                cours: choisis
                    .iter()
                    .filter(|(_, s)| s.get_untracked())
                    .map(|(id, _)| *id)
                    .collect(),
                seance_donnee: seance,
                echeance: echeance.get_untracked(),
                jour: jour.get_untracked(),
                heure: heure.get_untracked(),
                nature: nature.get_untracked(),
                titre: titre.get_untracked(),
                consigne: consigne.get_untracked(),
                duree: duree.get_untracked(),
                rendu: rendu.get_untracked(),
                retard_accepte: retard.get_untracked(),
                formats: formats.get_untracked(),
                ia: ia.get_untracked(),
                publication: publication.get_untracked(),
                publie_jour: publie_jour.get_untracked(),
                publie_heure: publie_heure.get_untracked(),
            };
            match valider_travail_c2(&saisie) {
                Err(es) => poser(
                    &es,
                    &[
                        ("cours", e_cours),
                        ("titre", e_titre),
                        ("consigne", e_consigne),
                        ("nature", e_nature),
                        ("duree", e_duree),
                        ("jour", e_jour),
                        ("heure", e_heure),
                        ("publie_jour", e_pj),
                        ("publie_heure", e_ph),
                    ],
                ),
                Ok(corps) => {
                    for s in [
                        e_cours, e_titre, e_consigne, e_nature, e_duree, e_jour, e_heure, e_pj,
                        e_ph,
                    ] {
                        s.set(None);
                    }
                    resultat.set(None);
                    let derogation = alerte_active.get_untracked();
                    ecriture.lancer(async move {
                        let r = session::client().cdt_travaux_creer(&corps).await?;
                        if derogation {
                            // Le serveur recalcule l'alerte : rien n'est compté sans alerte réelle ; un échec ne gêne pas.
                            for t in &r.travaux {
                                let _ = session::client().cdt_alerte_ignorer(t.id).await;
                            }
                        }
                        let n = r.travaux.len();
                        resultat.set(Some(r));
                        Ok(if n > 1 {
                            format!("{n} travaux créés, reliés.")
                        } else {
                            "Travail créé.".to_string()
                        })
                    });
                }
            }
        }
    };
    let options_echeance = vec![
        (
            "prochaine".to_string(),
            "Pour la prochaine séance du cours (proposé)".to_string(),
        ),
        ("date".to_string(), "Pour une date et une heure".to_string()),
    ];
    let options_rendu = vec![
        ("aucun".to_string(), "Pas de rendu".to_string()),
        ("en_ligne".to_string(), "À rendre en ligne".to_string()),
        ("en_classe".to_string(), "À rendre en classe".to_string()),
    ];
    let options_ia = vec![
        ("non".to_string(), "IA non autorisée".to_string()),
        (
            "recherche".to_string(),
            "IA autorisée pour la recherche".to_string(),
        ),
        (
            "relecture".to_string(),
            "IA autorisée pour la relecture".to_string(),
        ),
        ("libre".to_string(), "IA librement autorisée".to_string()),
    ];
    let options_publication = vec![
        (
            "brouillon".to_string(),
            "Brouillon (invisible des élèves)".to_string(),
        ),
        ("maintenant".to_string(), "Publier maintenant".to_string()),
        (
            "programmer".to_string(),
            "Programmer la publication".to_string(),
        ),
    ];
    let plusieurs = cours.len() > 1;
    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || resultat.get().map(|r| view! {
            <div class="cdt-resultat" role="status">
                {r.avertissements.iter().map(|a| view! { <Alerte niveau=Niveau::Avertissement titre=message_avertissement(a).to_string()/> }).collect_view()}
                <ul class="cdt-liste">
                    {r.travaux.iter().map(|t| view! {
                        <li>{format!("{} · {} · pour le {} à {} · {}", t.titre, t.nature, t.echeance_jour, t.echeance_heure, t.etat)}</li>
                    }).collect_view()}
                </ul>
            </div>
        })}
        <form class="cdt-form" on:submit=envoyer novalidate=true>
            <fieldset class="cdt-cours-choix">
                <legend>{if plusieurs { "Cours concernés (un travail par cours, reliés)" } else { "Cours" }}</legend>
                {cours.iter().zip(choisis.iter()).map(|(c, (_, sel))| view! {
                    <Case id=format!("cdt-d-cours-{}", c.cours_id) libelle=format!("{} · {}", c.libelle, c.cible) coche=*sel/>
                }).collect_view()}
                {move || e_cours.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            </fieldset>
            <Selection id="cdt-d-nature" libelle="Nature du travail" valeur=nature options=options_nature erreur=e_nature requis=true/>
            <ChampTexte id="cdt-d-titre" libelle="Titre" valeur=titre erreur=e_titre requis=true/>
            <ZoneTexte id="cdt-d-consigne" libelle="Consigne" valeur=consigne erreur=e_consigne requis=true
                aide="Texte simple : titres, listes, liens https. Aucun nom d'élève."/>
            <ChampTexte id="cdt-d-duree" libelle="Durée estimée (minutes)" valeur=duree genre="number" erreur=e_duree/>
            <p class="texte-attenue" role="note">{move || proposition().unwrap_or_default()}</p>
            <GroupeChoix nom="cdt-d-echeance" legende="Échéance" valeur=echeance options=options_echeance/>
            {move || (echeance.get() == "date").then(|| view! {
                <ChampTexte id="cdt-d-jour" libelle="Jour de l'échéance" valeur=jour genre="date" erreur=e_jour requis=true/>
                <ChampTexte id="cdt-d-heure" libelle="Heure (heure de l'établissement, 19 h par défaut ; pas d'échéance entre 20 h et 7 h)" valeur=heure genre="time" erreur=e_heure/>
            })}
            <BandeauCharge cours=bandeau_cours mode=echeance jour=jour nature=nature duree=duree alerte_active=alerte_active/>
            <Selection id="cdt-d-rendu" libelle="Mode de rendu" valeur=rendu options=options_rendu/>
            {move || (rendu.get() == "en_ligne").then(|| view! {
                <Case id="cdt-d-retard" libelle="Accepter un rendu en retard" coche=retard/>
                <ChampTexte id="cdt-d-formats" libelle="Formats acceptés (extensions séparées par des virgules, facultatif)" valeur=formats/>
            })}
            <Selection id="cdt-d-ia" libelle="Usage de l'IA" valeur=ia options=options_ia
                aide="Affiché aux élèves ; « non » par défaut."/>
            <GroupeChoix nom="cdt-d-publication" legende="Publication" valeur=publication options=options_publication/>
            {move || (publication.get() == "programmer").then(|| view! {
                <ChampTexte id="cdt-d-pjour" libelle="Jour de publication" valeur=publie_jour genre="date" erreur=e_pj requis=true/>
                <ChampTexte id="cdt-d-pheure" libelle="Heure (heure de l'établissement, 7 h par défaut)" valeur=publie_heure genre="time" erreur=e_ph/>
            })}
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer le travail"</Bouton>
        </form>
    }
}
