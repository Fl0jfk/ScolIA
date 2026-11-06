//! Préparation et tenue du conseil de classe (EVA-08, EVA-09, ORI-02) : tableau calculé par
//! le serveur (moyennes, absences, retards, sanctions, appréciations manquantes), puis par
//! élève l'appréciation du conseil, la mention, la décision, l'avis d'orientation et l'avis
//! sur les vœux. Les formulaires n'apparaissent que si le serveur indique `peut_decider`.

use ent_types::evaluations::{
    AvisVoeuBody, DecisionConseilBody, PreparationConseilDto, PreparationEleveDto, VoeuAvisDto,
};
use ent_web_ui::{
    Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Selection, Squelette, Statistique, TonEtat,
    Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use super::modele::{libelle_code, moyenne_fr, option_code, AVIS, DECISIONS, MENTIONS};
use super::{lien_classe, BadgePeriode, RACINE};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn PageConseil() -> impl IntoView {
    let p = use_params_map();
    let id = Memo::new(move |_| {
        p.with(|p| p.get("id"))
            .and_then(|s| Uuid::parse_str(&s).ok())
    });
    view! {
        <Title text="Conseil de classe"/>
        {move || match id.get() {
            Some(c) => view! { <Conseil conseil=c/> }.into_any(),
            None => view! { <EtatVide titre="Conseil introuvable" message="Ouvrez le conseil depuis la page Évaluations."/> }.into_any(),
        }}
    }
}

#[component]
fn Conseil(conseil: Uuid) -> impl IntoView {
    let prep =
        LocalResource::new(
            move || async move { session::client().preparation_conseil(conseil).await },
        );
    let ecriture = Ecriture::nouveau(Callback::new(move |_| prep.refetch()));
    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || match prep.get() {
            None => view! { <Squelette lignes=8 libelle="Préparation du conseil…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| prep.refetch())/> }.into_any(),
            Some(Ok(p)) => view! { <Preparation prep=p ecriture=ecriture/> }.into_any(),
        }}
    }
}

#[component]
fn Preparation(prep: PreparationConseilDto, ecriture: Ecriture) -> impl IntoView {
    let c = prep.conseil.clone();
    let peut = prep.peut_decider;
    let conseil_id = c.id;
    let lien = match (c.classe_id, c.periode_id) {
        (Some(cl), Some(pe)) => Some(lien_classe(cl, pe)),
        _ => None,
    };
    let (manq, gen) = (
        prep.appreciations_manquantes,
        prep.appreciations_generales_manquantes,
    );
    let ton = |n: u32| {
        if n > 0 {
            TonEtat::Avertissement
        } else {
            TonEtat::Ok
        }
    };
    let (ton_manq, ton_gen) = (ton(manq), ton(gen));
    let moyenne_classe = prep.moyenne_classe;
    view! {
        <EnTetePage
            surtitre="Conseil de classe".to_string()
            titre=format!("{} · {}", c.classe, c.periode_libelle)
            sous_titre=format!("{} à {}{}", c.jour.format("%d/%m/%Y"), c.heure.format("%H:%M"), if c.lieu.is_empty() { String::new() } else { format!(" · {}", c.lieu) })
        />
        <p class="eva-periode">
            {prep.periode.as_ref().map(|p| view! { <BadgePeriode statut=p.statut.clone()/> })}
            " "<a href=RACINE>"Toutes les évaluations"</a>
            {lien.map(|l| view! { " · "<a href=l>"Moyennes de la classe"</a> })}
        </p>
        <div class="jvs-stats">
            <Statistique libelle="Moyenne de la classe" valeur=Signal::derive(move || moyenne_fr(moyenne_classe))/>
            <Statistique libelle="Appréciations de matière manquantes" valeur=Signal::derive(move || manq.to_string()) ton=ton_manq/>
            <Statistique libelle="Appréciations générales manquantes" valeur=Signal::derive(move || gen.to_string()) ton=ton_gen/>
        </div>
        {(!peut).then(|| view! { <p class="texte-attenue">"Consultation : les décisions sont saisies par la direction ou le professeur principal."</p> })}
        <ul class="jvs-liste eva-conseil">
            {prep.eleves.into_iter().map(|e| view! { <LigneEleve eleve=e conseil=conseil_id peut=peut ecriture=ecriture/> }).collect_view()}
        </ul>
    }
}

#[component]
fn LigneEleve(
    eleve: PreparationEleveDto,
    conseil: Uuid,
    peut: bool,
    ecriture: Ecriture,
) -> impl IntoView {
    let d = eleve.decision.clone();
    let appreciation = RwSignal::new(
        d.as_ref()
            .map(|x| x.appreciation.clone())
            .unwrap_or_default(),
    );
    let mention = RwSignal::new(
        d.as_ref()
            .and_then(|x| x.mention.clone())
            .unwrap_or_default(),
    );
    let decision = RwSignal::new(
        d.as_ref()
            .and_then(|x| x.decision_code.clone())
            .unwrap_or_default(),
    );
    let avis = RwSignal::new(
        d.as_ref()
            .and_then(|x| x.avis_orientation.clone())
            .unwrap_or_default(),
    );
    let eleve_id = eleve.eleve_id;
    let nom = format!("{} {}", eleve.prenom, eleve.nom);
    let nom_msg = nom.clone();
    let enregistrer = Callback::new(move |_| {
        let body = DecisionConseilBody {
            eleve_id,
            appreciation: appreciation.get_untracked(),
            mention: option_code(&mention.get_untracked()),
            decision_code: option_code(&decision.get_untracked()),
            avis_orientation: option_code(&avis.get_untracked()),
        };
        let nom = nom_msg.clone();
        ecriture.lancer(async move {
            session::client().decider_conseil(conseil, &body).await?;
            Ok(format!("Décision enregistrée pour {nom}."))
        });
    });
    let opts = |l: &[(&str, &str)]| -> Vec<(String, String)> {
        l.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    };
    let p = eleve.eleve_id.simple().to_string();
    let manquantes = eleve.appreciations_manquantes.clone();
    view! {
        <li class="jvs-ligne jvs-ligne--bloc eva-eleve">
            <div class="eva-eleve__tete">
                <strong>{nom.clone()}</strong>
                <span>{format!("Moyenne générale : {}", moyenne_fr(eleve.moyenne_generale))}</span>
                <span class="texte-attenue">{format!(
                    "{} absence(s) dont {} non justifiée(s) · {} retard(s) · {} sanction(s)",
                    eleve.absences, eleve.absences_non_justifiees, eleve.retards, eleve.sanctions
                )}</span>
                {(!manquantes.is_empty()).then(|| view! {
                    <Badge libelle=format!("Appréciation manquante : {}", manquantes.join(", ")) ton=TonEtat::Avertissement/>
                })}
                {eleve.appreciation_generale.is_none().then(|| view! { <Badge libelle="Sans appréciation générale" ton=TonEtat::Avertissement/> })}
                {d.as_ref().map(|x| view! {
                    <span>{format!("Mention : {} · Décision : {}", libelle_code(&MENTIONS, x.mention.as_deref()), libelle_code(&DECISIONS, x.decision_code.as_deref()))}</span>
                })}
            </div>
            {eleve.appreciation_generale.clone().map(|a| view! { <p class="eva-citation">{a}</p> })}
            {peut.then(|| view! {
                <details class="eva-details">
                    <summary>{format!("Décision du conseil — {nom}")}</summary>
                    <div class="eva-formulaire">
                        <ZoneTexte id=format!("ap-{p}") libelle="Appréciation du conseil" valeur=appreciation/>
                        <Selection id=format!("me-{p}") libelle="Mention" valeur=mention options=opts(&MENTIONS)/>
                        <Selection id=format!("de-{p}") libelle="Décision" valeur=decision options=opts(&DECISIONS)/>
                        <ChampTexte id=format!("or-{p}") libelle="Avis d'orientation" valeur=avis/>
                        <Bouton variante=Variante::Primaire en_cours=ecriture.en_cours on_click=enregistrer>"Enregistrer"</Bouton>
                    </div>
                    {(!eleve.voeux.is_empty()).then(|| view! {
                        <h3 class="jvs-sous-titre">"Vœux d'orientation"</h3>
                        <ul class="jvs-liste">
                            {eleve.voeux.iter().cloned().map(|v| view! { <LigneVoeu voeu=v conseil=conseil ecriture=ecriture/> }).collect_view()}
                        </ul>
                    })}
                </details>
            })}
        </li>
    }
}

#[component]
fn LigneVoeu(voeu: VoeuAvisDto, conseil: Uuid, ecriture: Ecriture) -> impl IntoView {
    let avis = RwSignal::new(voeu.avis.clone().unwrap_or_default());
    let motivation = RwSignal::new(voeu.motivation.clone());
    let voeu_id = voeu.voeu_id;
    let enregistrer = Callback::new(move |_| {
        let a = avis.get_untracked();
        if a.is_empty() {
            ecriture.erreur.set(Some("Choisissez un avis.".into()));
            return;
        }
        let body = AvisVoeuBody {
            voeu_id,
            avis: a,
            motivation: motivation.get_untracked(),
        };
        ecriture.lancer(async move {
            session::client().avis_voeu(conseil, &body).await?;
            Ok("Avis sur le vœu enregistré.".into())
        });
    });
    let opts: Vec<(String, String)> = AVIS
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let p = voeu.voeu_id.simple().to_string();
    view! {
        <li class="jvs-ligne jvs-ligne--bloc">
            <span>{format!("Vœu {} : {}", voeu.rang, voeu.filiere_libelle)}</span>
            {voeu.reponse_famille.clone().map(|r| view! { <Badge libelle=format!("Famille : {r}") ton=TonEtat::Info/> })}
            <div class="eva-formulaire">
                <Selection id=format!("av-{p}") libelle="Avis du conseil" valeur=avis options=opts/>
                <ChampTexte id=format!("mo-{p}") libelle="Motivation" valeur=motivation/>
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=enregistrer>"Enregistrer l'avis"</Bouton>
            </div>
        </li>
    }
}
