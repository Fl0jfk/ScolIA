//! « Mes remplacements » (ABP-158, 176, 177, 178) : ce qu'on me demande de remplacer, en un geste.
//! Accepter ; refuser (motif légitime codé si je suis volontaire RCD sur mon créneau, sinon sans motif,
//! car un collègue hors RCD n'a aucune obligation) ; accuser réception d'une désignation.
//!
//! Le message de la sollicitation est celui du serveur : il ne contient **jamais** la nature de
//! l'absence ni le nom de l'absent (R-AP41). Aucune contrainte d'un collègue n'est nommée à un autre.

use ent_types::absences_personnels::SollicitationDto;
use ent_web_api::ApiError;
use ent_web_ui::{Badge, Bouton, EtatVide, Selection, Squelette, Taille, Variante};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::commun::GardePersonnel;
use super::modele::{
    libelle_etat_sollicitation, libelle_seance, sollicitation_ouverte, ton_etat_sollicitation,
    REFUS_CODES,
};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{a_paris, date_heure_paris, date_longue};
use ent_types::absences_personnels::ImpactDto;

#[component]
pub fn PageMesRemplacements() -> impl IntoView {
    view! {
        <Title text="Mes remplacements"/>
        <GardePersonnel
            titre="Mes remplacements"
            sous_titre="Les séances qu'on vous demande de remplacer."
        >
            <Contenu/>
        </GardePersonnel>
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let liste = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_mes_sollicitations().await }
    });
    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || match liste.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de vos demandes…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            Some(Ok(v)) if v.is_empty() => view! {
                <EtatVide titre="Aucune demande de remplacement" message="Quand on vous demandera de remplacer une séance, vous pourrez accepter ou refuser ici, en un geste."/>
            }
            .into_any(),
            Some(Ok(v)) => {
                let (ouvertes, autres): (Vec<_>, Vec<_>) = v.into_iter().partition(|s| sollicitation_ouverte(&s.etat));
                view! {
                    <section class="absp-section" aria-labelledby="absp-a-repondre">
                        <h2 id="absp-a-repondre" class="vs-section-titre">"À répondre"</h2>
                        {if ouvertes.is_empty() {
                            view! { <p class="texte-attenue">"Rien n'attend votre réponse."</p> }.into_any()
                        } else {
                            view! {
                                <ul class="absp-liste">
                                    {ouvertes.into_iter().map(|s| view! { <CarteSollicitation s=s ecriture=ecriture/> }).collect_view()}
                                </ul>
                            }
                            .into_any()
                        }}
                    </section>
                    {(!autres.is_empty()).then(|| view! {
                        <section class="absp-section" aria-labelledby="absp-historique-sollicitations">
                            <h2 id="absp-historique-sollicitations" class="vs-section-titre">"Déjà traitées"</h2>
                            <ul class="absp-liste">
                                {autres.into_iter().map(|s| view! { <CarteSollicitation s=s ecriture=ecriture/> }).collect_view()}
                            </ul>
                        </section>
                    })}
                }
                .into_any()
            }
        }}
    }
}

/// Un conflit à l'acceptation n'a pas de texte utile côté serveur (« valeur déjà existante ») : on dit les
/// causes connues (A4) sans en désigner une que l'on ne peut pas vérifier.
fn motif_refus_acceptation(e: ApiError) -> ApiError {
    match e {
        ApiError::Conflit(_) => ApiError::Conflit(
            "la demande n'a pas pu être acceptée : elle a expiré, la séance est déjà pourvue, l'absence n'est pas encore validée, ou vous avez déjà une séance à cette heure".into(),
        ),
        autre => autre,
    }
}

/// Résumé d'une séance de sollicitation (sans nature : le DTO n'en porte pas).
fn resume_seance(s: &SollicitationDto) -> String {
    match &s.seance {
        Some(seance) => {
            let i = ImpactDto {
                id: s.impact_id,
                absence_id: Uuid::nil(),
                type_impact: "seance".into(),
                seance: Some(seance.clone()),
                debut: chrono::DateTime::UNIX_EPOCH,
                fin: chrono::DateTime::UNIX_EPOCH,
                etat: s.etat.clone(),
                a_revoir: false,
            };
            format!("{} · {}", date_longue(seance.jour), libelle_seance(&i))
        }
        None => "Séance à remplacer".to_string(),
    }
}

#[component]
fn CarteSollicitation(s: SollicitationDto, ecriture: Ecriture) -> impl IntoView {
    let id = s.id;
    let ouverte = sollicitation_ouverte(&s.etat);
    let designee = s.etat == "designee";
    let resume = resume_seance(&s);
    let refus = RwSignal::new(String::new());
    let options: Vec<(String, String)> = REFUS_CODES
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let avant = s
        .expire_le
        .map(|e| format!("Répondre avant {}.", a_paris(e).format("%H:%M")));
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{resume.clone()}</strong>
                <Badge libelle=libelle_etat_sollicitation(&s.etat) ton=ton_etat_sollicitation(&s.etat)/>
            </div>
            <p class="absp-carte__detail">{s.message.clone()}</p>
            <p class="absp-carte__detail">{format!("Demandé le {}.", date_heure_paris(s.envoyee_le))}</p>
            {ouverte.then(|| avant.clone()).flatten().map(|t| view! { <p class="absp-carte__detail">{t}</p> })}
            {(ouverte && designee).then(|| view! {
                <p class="absp-carte__actions">
                    <Bouton
                        taille=Taille::Compacte
                        libelle_accessible=format!("Accuser réception : {resume}")
                        on_click=Callback::new(move |_| {
                            ecriture.lancer(async move {
                                session::client().absp_accuser(id).await?;
                                Ok("Réception accusée.".to_string())
                            });
                        })
                    >
                        "Accuser réception"
                    </Bouton>
                </p>
            })}
            {(ouverte && !designee).then(|| {
                let resume_a = resume.clone();
                let resume_r = resume.clone();
                view! {
                    <div class="absp-reponse">
                        <p class="absp-carte__actions">
                            <Bouton
                                taille=Taille::Compacte
                                libelle_accessible=format!("Accepter : {resume_a}")
                                on_click=Callback::new(move |_| {
                                    ecriture.lancer(async move {
                                        session::client().absp_accepter(id).await.map_err(motif_refus_acceptation)?;
                                        Ok("Remplacement accepté : la séance est à vous.".to_string())
                                    });
                                })
                            >
                                "Accepter"
                            </Bouton>
                        </p>
                        <Selection
                            id=format!("absp-refus-{id}")
                            libelle="Si vous refusez, pourquoi ?"
                            valeur=refus
                            options=options
                            aide="Un motif n'est exigé que si vous êtes volontaire pour le remplacement de courte durée sur ce créneau."
                        />
                        <p class="absp-carte__actions">
                            <Bouton
                                variante=Variante::Secondaire
                                taille=Taille::Compacte
                                libelle_accessible=format!("Refuser : {resume_r}")
                                on_click=Callback::new(move |_| {
                                    let code = refus.get_untracked();
                                    ecriture.lancer(async move {
                                        session::client()
                                            .absp_refuser_sollicitation(id, (!code.is_empty()).then_some(code))
                                            .await?;
                                        Ok("Demande refusée.".to_string())
                                    });
                                })
                            >
                                "Refuser"
                            </Bouton>
                        </p>
                    </div>
                }
            })}
        </li>
    }
}
