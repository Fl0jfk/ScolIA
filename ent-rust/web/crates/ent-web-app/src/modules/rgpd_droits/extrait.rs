//! Section **Extrait** de la fiche d'une demande (socle 27, lot G3b-2) : assembler, état des sections, relecture des
//! éléments marqués (accepter ou lever avec motif), soumission au DPO. Logique pure : `modele_extrait.rs`.
//! Le contenu n'est lu que sur demande (lecture tracée côté serveur) ; `ui.rs` n'est pas étendu.

use ent_web_api::rgpd_extrait_types::{
    AssemblerBody, DecisionMasquageBody, ExtraitDto, MarquageDto,
};
use ent_web_api::ApiError;
use ent_web_ui::{Alerte, Badge, Bouton, ChampTexte, Niveau, Squelette, TonEtat, Variante};
use leptos::prelude::*;
use uuid::Uuid;

use super::modele_extrait::{
    cible, libelle_decision_marquage, libelle_raison, libelle_statut_section, phrase_etat,
    restants, valeur_marquee,
};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn SectionExtrait(demande: Uuid, ouverte: bool) -> impl IntoView {
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let extraits = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().rgpd_extraits(demande).await }
    });
    view! {
        <section class="classe-section" aria-labelledby="rgpd-extrait-titre">
            <h2 id="rgpd-extrait-titre">"Extrait"</h2>
            <RetourEcriture ecriture=ecriture/>
            {ouverte.then(|| view! {
                <Bouton variante=Variante::Secondaire
                    desactive=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(move |_| {
                        ecriture.lancer(async move {
                            session::client()
                                .rgpd_extrait_assembler(demande, &AssemblerBody::default())
                                .await
                                .map(|_| "Extrait assemblé.".to_string())
                        });
                    })>
                    "Assembler l'extrait"
                </Bouton>
            })}
            {move || match extraits.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement de l'extrait…"/> }.into_any(),
                Some(Err(ApiError::Introuvable(_))) => view! { <p>"Aucun extrait."</p> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| extraits.refetch())/>
                }.into_any(),
                Some(Ok(v)) => {
                    let courants: Vec<ExtraitDto> = v.into_iter().filter(|e| !e.perime).collect();
                    if courants.is_empty() {
                        view! { <p class="texte-attenue">"Aucun extrait assemblé pour le moment."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="ui-liste">
                                {courants.into_iter().map(|e| view! {
                                    <CarteExtrait e=e demande=demande ecriture=ecriture ouverte=ouverte/>
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    }
                }
            }}
        </section>
    }
}

#[component]
fn CarteExtrait(e: ExtraitDto, demande: Uuid, ecriture: Ecriture, ouverte: bool) -> impl IntoView {
    let id = e.id;
    let relecture = RwSignal::new(false);
    let phrase = phrase_etat(&e);
    let ton = if e.remissible_en_l_etat {
        TonEtat::Ok
    } else {
        TonEtat::Avertissement
    };
    let sections = e.sections.clone();
    let titre = format!(
        "Version {} · {} section(s) · {} ligne(s)",
        e.version, e.nb_sections, e.nb_lignes
    );
    let avis = e.avis_dpo_code.clone().map(|c| {
        format!(
            "Avis du DPO : {} — {}",
            if c == "favorable" {
                "favorable"
            } else {
                "avec réserves"
            },
            e.avis_dpo_texte.clone().unwrap_or_default()
        )
    });
    let soumis = e.soumis_dpo;
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{titre}</span>
                <span class="ui-liste__detail"><Badge libelle=phrase ton=ton/></span>
                <ul>
                    {sections.into_iter().map(|s| view! {
                        <li>
                            {format!("{} : {}", s.fournisseur, libelle_statut_section(&s.statut))}
                            {s.remise_incomplete_motif.map(|m| format!(" (remise décidée : {m})"))}
                        </li>
                    }).collect_view()}
                </ul>
                {avis.map(|a| view! { <p>{a}</p> })}
            </div>
            {ouverte.then(|| view! {
                <div class="ui-liste__actions">
                    <Bouton variante=Variante::Secondaire
                        on_click=Callback::new(move |_| relecture.update(|r| *r = !*r))>
                        "Relecture"
                    </Bouton>
                    {(!soumis).then(|| view! {
                        <Bouton variante=Variante::Secondaire
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            on_click=Callback::new(move |_| {
                                ecriture.lancer(async move {
                                    session::client().rgpd_extrait_soumettre_dpo(demande, id).await
                                        .map(|_| "Extrait soumis au DPO.".to_string())
                                });
                            })>
                            "Soumettre au DPO"
                        </Bouton>
                    })}
                </div>
            })}
            {move || relecture.get().then(|| view! { <Relecture demande=demande extrait=id ecriture=ecriture/> })}
        </li>
    }
}

/// Relecture : le contenu est lu (trace côté serveur), chaque élément marqué est montré avec sa valeur.
#[component]
fn Relecture(demande: Uuid, extrait: Uuid, ecriture: Ecriture) -> impl IntoView {
    let version = RwSignal::new(0u32);
    let motif = RwSignal::new(String::new());
    let contenu = LocalResource::new(move || {
        let _ = version.get();
        async move {
            session::client()
                .rgpd_extrait_contenu(demande, extrait)
                .await
        }
    });
    view! {
        <div class="rgpd-carte">
            <h3>"Relecture"</h3>
            <ChampTexte id=format!("rgpd-motif-{extrait}") libelle="Motif (obligatoire pour lever un marquage)".to_string()
                valeur=motif/>
            {move || match contenu.get() {
                None => view! { <Squelette lignes=4 libelle="Ouverture du contenu…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| contenu.refetch())/>
                }.into_any(),
                Some(Ok(c)) => {
                    let reste = restants(&c.marquages);
                    let lignes: Vec<(MarquageDto, String)> = c.marquages.iter()
                        .map(|m| (m.clone(), valeur_marquee(&c, m))).collect();
                    let reserves = c.elements_reserves_direction;
                    view! {
                        <p>{format!("{reste} marquage(s) restant à décider.")}</p>
                        // R-RG46 (M4) : ce qui est réservé à la direction n'est ni listé ni situé ; seul le nombre est dit.
                        {(reserves > 0).then(|| view! {
                            <Alerte niveau=Niveau::Info titre=format!("{reserves} élément(s) réservé(s) à la direction.")>
                                <p>"Ils ne sont pas affichés ici. La direction les relit avant la remise."</p>
                            </Alerte>
                        })}
                        {(c.marquages.is_empty()).then(|| view! {
                            <Alerte niveau=Niveau::Info titre="Aucun élément marqué.".to_string()>
                                <p>"Relisez tout de même le contenu avant la remise."</p>
                            </Alerte>
                        })}
                        <ul class="ui-liste">
                            {lignes.into_iter().map(|(m, valeur)| {
                                let mid = m.id;
                                view! {
                                    <li class="ui-liste__element">
                                        <div class="ui-liste__principal">
                                            <span class="ui-liste__titre">{cible(&m)}</span>
                                            <span class="ui-liste__detail">
                                                {format!("{} · {}", libelle_raison(&m.raison), libelle_decision_marquage(&m.decision))}
                                            </span>
                                            <mark>{valeur}</mark>
                                        </div>
                                        <div class="ui-liste__actions">
                                            <Bouton variante=Variante::Secondaire
                                                desactive=Signal::derive(move || ecriture.en_cours.get())
                                                on_click=Callback::new(move |_| {
                                                    let b = DecisionMasquageBody { decision: "masque".into(), motif: None };
                                                    ecriture.lancer(async move {
                                                        session::client().rgpd_masquage_decider(demande, mid, &b).await
                                                            .map(|_| { version.update(|v| *v += 1); "Marquage accepté.".to_string() })
                                                    });
                                                })>
                                                "Masquer"
                                            </Bouton>
                                            <Bouton variante=Variante::Secondaire
                                                desactive=Signal::derive(move || ecriture.en_cours.get() || motif.get().trim().len() < 3)
                                                on_click=Callback::new(move |_| {
                                                    let b = DecisionMasquageBody { decision: "leve".into(), motif: Some(motif.get()) };
                                                    ecriture.lancer(async move {
                                                        session::client().rgpd_masquage_decider(demande, mid, &b).await
                                                            .map(|_| { version.update(|v| *v += 1); "Marquage levé.".to_string() })
                                                    });
                                                })>
                                                "Lever"
                                            </Bouton>
                                        </div>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }
            }}
        </div>
    }
}
