//! **Accès aux dossiers** (socle 27, lot G4-2 ; `27_RGPD_ARCHIVES.md` §11.4, §11.5, RGP-193 à RGP-198).
//!
//! | Écran | Adresse | Qui |
//! |---|---|---|
//! | Alertes, contrôles internes, réglages | `/web/rgpd/acces` | `acces.consulter_traces` (direction, DPO) |
//! | Traces d'un sujet | section de la fiche d'une demande ouverte | idem ; le report et le nom d'un agent : `droits.arbitrer` |
//!
//! L'écran ne calcule aucun droit : le serveur décide. Les traces montrent la fonction et le service, **jamais le nom**
//! (hors arbitrage motivé) ; aucun total par agent, aucun classement. L'ouverture d'un contrôle demande la **passkey
//! récente** de la direction, liée au contrôle demandé. Logique pure : `modele.rs` ; `ui.rs` n'est pas étendu.

pub mod modele;

use ent_web_api::acces_traces_types::{
    AlerteAccesDto, AlerteSuiteBody, ChangementReglageDto, ControleCloreBody, ControleDemandeBody,
    ControleDto, ControleOuvrirBody, DifferBody, LigneReglageDto, NomAgentBody, ReglageAccesBody,
    TraceLigneDto, TracesSujetDto,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, EnTetePage, EtatVide, Niveau, Selection, Squelette, TonEtat,
    Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::webauthn;

use modele::{
    alerte_a_traiter, controle_recevable, jours_depuis_texte, libelle_issue,
    libelle_motif_controle, libelle_suite, phrase_alerte, phrase_controle, precisions, quand, qui,
    report_recevable,
};

fn echec_navigateur(m: String) -> ApiError {
    ApiError::Invalide(m)
}

// ═══════════════════════════════════ Page « Accès aux dossiers » ═══════════════════════════════════

#[component]
pub fn PageAccesDossiers() -> impl IntoView {
    let moteur = use_droits_moteur();
    // Agent et motif choisis pour un contrôle : une alerte les préremplit.
    let agent = RwSignal::new(String::new());
    let motif = RwSignal::new("verification_periodique".to_string());
    let version = RwSignal::new(0u32);
    view! {
        <Title text="Accès aux dossiers"/>
        <EnTetePage
            surtitre="Conformité".to_string()
            titre="Accès aux dossiers".to_string()
            sous_titre="Alertes d'accès, contrôles internes et réglages. Les consultations montrent la fonction et le service, jamais le nom d'un agent.".to_string()
        />
        {move || if moteur.get().peut_consulter_traces_acces() || moteur.get().peut_traiter_demandes_droits() {
            view! {
                <SectionAlertes agent=agent motif=motif version=version/>
                <SectionControles agent=agent motif=motif version=version/>
                <SectionReglages/>
            }.into_any()
        } else {
            view! {
                <EtatVide titre="Écran réservé"
                    message="Les alertes et les contrôles d'accès sont réservés à la direction et au délégué à la protection des données."/>
            }.into_any()
        }}
    }
}

// ───────────────────────────────────── Alertes ─────────────────────────────────────

#[component]
fn SectionAlertes(
    agent: RwSignal<String>,
    motif: RwSignal<String>,
    version: RwSignal<u32>,
) -> impl IntoView {
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let alertes = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().acces_alertes().await }
    });
    view! {
        <section class="classe-section" aria-labelledby="acces-alertes-titre">
            <h2 id="acces-alertes-titre">"Alertes d'accès"</h2>
            <p>
                "Ces alertes ne sont jamais envoyées à la personne concernée. Une suite est à donner à chacune : \
                 rien à signaler, manquement constaté, ou ouverture d'un contrôle."
            </p>
            <RetourEcriture ecriture=ecriture/>
            {move || match alertes.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des alertes…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| alertes.refetch())/>
                }.into_any(),
                Some(Ok(v)) if v.is_empty() => view! {
                    <p class="texte-attenue">"Aucune alerte pour le moment."</p>
                }.into_any(),
                Some(Ok(v)) => view! {
                    <ul class="ui-liste">
                        {v.into_iter().map(|a| view! {
                            <CarteAlerte a=a ecriture=ecriture agent=agent motif=motif/>
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </section>
    }
}

#[component]
fn CarteAlerte(
    a: AlerteAccesDto,
    ecriture: Ecriture,
    agent: RwSignal<String>,
    motif: RwSignal<String>,
) -> impl IntoView {
    let id = a.id;
    let a_traiter = alerte_a_traiter(&a);
    let agent_id = a.agent_compte_id;
    let suite = move |code: &'static str| {
        ecriture.lancer(async move {
            session::client()
                .acces_alerte_suite(
                    id,
                    &AlerteSuiteBody {
                        suite: code.to_string(),
                    },
                )
                .await?;
            Ok("Suite enregistrée.".to_string())
        });
    };
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{a.libelle.clone()}</span>
                <span class="ui-liste__detail">
                    {format!("{} — {}", a.agent_nom, phrase_alerte(&a))}
                </span>
                <span class="ui-liste__detail">
                    <Badge libelle=libelle_suite(&a.suite).to_string()
                        ton=if a_traiter { TonEtat::Avertissement } else { TonEtat::Ok }/>
                </span>
                {a_traiter.then(|| view! {
                    <div>
                        <Bouton variante=Variante::Secondaire
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            on_click=Callback::new(move |_| suite("rien"))>
                            "Rien à signaler"
                        </Bouton>
                        <Bouton variante=Variante::Secondaire
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            on_click=Callback::new(move |_| suite("violation"))>
                            "Manquement constaté"
                        </Bouton>
                        <Bouton variante=Variante::Secondaire
                            on_click=Callback::new(move |_| {
                                agent.set(agent_id.to_string());
                                motif.set("alerte".to_string());
                            })>
                            "Préparer un contrôle"
                        </Bouton>
                        <Bouton variante=Variante::Secondaire
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            on_click=Callback::new(move |_| suite("controle"))>
                            "Contrôle déjà ouvert"
                        </Bouton>
                    </div>
                })}
            </div>
        </li>
    }
}

// ───────────────────────────────────── Contrôles internes ─────────────────────────────────────

#[component]
fn SectionControles(
    agent: RwSignal<String>,
    motif: RwSignal<String>,
    version: RwSignal<u32>,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let controles = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().acces_controles().await }
    });
    let annuaire = LocalResource::new(|| async { session::client().absp_annuaire().await });
    let texte = RwSignal::new(String::new());
    let duree = RwSignal::new("15".to_string());
    let invalide = RwSignal::new(None::<String>);
    let traces_de = RwSignal::new(None::<Uuid>);
    let ouvrir = move || {
        let agent_txt = agent.get_untracked();
        let agent_id = Uuid::parse_str(&agent_txt).ok();
        let (m, t, j) = (
            motif.get_untracked(),
            texte.get_untracked(),
            jours_depuis_texte(&duree.get_untracked()),
        );
        if let Err(e) = controle_recevable(agent_id.is_some(), &m, &t, j) {
            invalide.set(Some(e.to_string()));
            return;
        }
        invalide.set(None);
        let demande = ControleDemandeBody {
            agent_compte_id: agent_id.unwrap_or_default(),
            motif_code: m,
            motif_texte: t,
            jours: j,
        };
        ecriture.lancer(async move {
            let client = session::client();
            let defi = client.acces_controle_defi(&demande).await?;
            let assertion = webauthn::signer_defi(&defi.options)
                .await
                .map_err(echec_navigateur)?;
            client
                .acces_controle_ouvrir(&ControleOuvrirBody {
                    demande,
                    defi_id: defi.defi_id,
                    credential: assertion,
                })
                .await?;
            Ok("Contrôle ouvert.".to_string())
        });
    };
    let motifs: Vec<(String, String)> = [
        "verification_periodique",
        "signalement",
        "alerte",
        "plainte",
    ]
    .iter()
    .map(|c| ((*c).to_string(), libelle_motif_controle(c).to_string()))
    .collect();
    view! {
        <section class="classe-section" aria-labelledby="acces-controles-titre">
            <h2 id="acces-controles-titre">"Contrôles internes"</h2>
            <p>
                "Un contrôle porte sur un agent, pour un motif précis, pendant 31 jours au plus. Il donne accès à la \
                 liste datée des consultations de l'agent, sans statistique ni classement. L'ouverture demande votre passkey."
            </p>
            <RetourEcriture ecriture=ecriture/>
            {move || match controles.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des contrôles…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| controles.refetch())/>
                }.into_any(),
                Some(Ok(v)) if v.is_empty() => view! {
                    <p class="texte-attenue">"Aucun contrôle."</p>
                }.into_any(),
                Some(Ok(v)) => view! {
                    <ul class="ui-liste">
                        {v.into_iter().map(|c| view! {
                            <CarteControle c=c ecriture=ecriture traces_de=traces_de/>
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
            {move || traces_de.get().map(|id| view! { <TracesDuControle id=id/> })}
            {move || moteur.get().peut_ouvrir_controle_acces().then(|| view! {
                <fieldset>
                    <legend>"Ouvrir un contrôle"</legend>
                    {move || match annuaire.get() {
                        Some(Ok(a)) => {
                            let mut options = vec![(String::new(), "— Choisir un agent —".to_string())];
                            options.extend(a.into_iter().filter_map(|x| {
                                x.user_id.map(|u| (u.to_string(), x.nom_affiche))
                            }));
                            view! {
                                <Selection id="acces-controle-agent".to_string() libelle="Agent concerné".to_string()
                                    valeur=agent options=options requis=true/>
                            }.into_any()
                        }
                        _ => view! {
                            <p class="texte-attenue">"Annuaire des agents indisponible."</p>
                        }.into_any(),
                    }}
                    <Selection id="acces-controle-motif".to_string() libelle="Motif".to_string()
                        valeur=motif options=motifs.clone() requis=true/>
                    <ZoneTexte id="acces-controle-texte".to_string() libelle="Précision du motif".to_string()
                        valeur=texte requis=true/>
                    <ChampTexte id="acces-controle-duree".to_string() libelle="Durée (jours, 31 au plus)".to_string()
                        valeur=duree requis=true/>
                    {move || invalide.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                    <Bouton variante=Variante::Primaire
                        desactive=Signal::derive(move || ecriture.en_cours.get())
                        on_click=Callback::new(move |_| ouvrir())>
                        "Ouvrir le contrôle (passkey)"
                    </Bouton>
                </fieldset>
            })}
        </section>
    }
}

#[component]
fn CarteControle(
    c: ControleDto,
    ecriture: Ecriture,
    traces_de: RwSignal<Option<Uuid>>,
) -> impl IntoView {
    let id = c.id;
    let issue = RwSignal::new("rien".to_string());
    let ouvert = c.ouvert;
    let echu = c.clos_le.is_none() && !c.ouvert;
    let clore = move || {
        let i = issue.get_untracked();
        ecriture.lancer(async move {
            session::client()
                .acces_controle_clore(id, &ControleCloreBody { issue: i })
                .await?;
            Ok("Contrôle clos.".to_string())
        });
    };
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">
                    {format!("{} — {}", c.agent_nom, libelle_motif_controle(&c.motif_code))}
                </span>
                <span class="ui-liste__detail">{c.motif_texte.clone()}</span>
                <span class="ui-liste__detail">
                    <Badge libelle=phrase_controle(&c)
                        ton=if ouvert { TonEtat::Avertissement } else { TonEtat::Neutre }/>
                </span>
                {ouvert.then(|| view! {
                    <Bouton variante=Variante::Secondaire
                        on_click=Callback::new(move |_| traces_de.set(Some(id)))>
                        "Voir les consultations de l'agent"
                    </Bouton>
                })}
                {(ouvert || echu).then(|| view! {
                    <div>
                        <Selection id=format!("acces-issue-{id}") libelle="Issue".to_string() valeur=issue
                            options=vec![
                                ("rien".to_string(), libelle_issue("rien").to_string()),
                                ("violation".to_string(), libelle_issue("violation").to_string()),
                                ("transmis".to_string(), libelle_issue("transmis").to_string()),
                            ]/>
                        <Bouton variante=Variante::Secondaire
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            on_click=Callback::new(move |_| clore())>
                            "Clore le contrôle"
                        </Bouton>
                    </div>
                })}
            </div>
        </li>
    }
}

#[component]
fn TracesDuControle(id: Uuid) -> impl IntoView {
    let traces =
        LocalResource::new(
            move || async move { session::client().acces_controle_traces(id).await },
        );
    view! {
        <section class="classe-section" aria-labelledby="acces-traces-controle-titre">
            <h3 id="acces-traces-controle-titre">"Consultations de l'agent"</h3>
            <p class="texte-attenue">"La lecture de cette liste est elle-même tracée."</p>
            {move || match traces.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement des consultations…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| traces.refetch())/>
                }.into_any(),
                Some(Ok(t)) => view! { <ListeTraces lignes=t.lignes peut_arbitrer=false demande=None ecriture=None/> }.into_any(),
            }}
        </section>
    }
}

// ───────────────────────────────────── Réglages ─────────────────────────────────────

#[component]
fn SectionReglages() -> impl IntoView {
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let reglages = LocalResource::new(move || {
        let _ = version.get();
        async move { session::client().acces_reglages().await }
    });
    view! {
        <section class="classe-section" aria-labelledby="acces-reglages-titre">
            <h2 id="acces-reglages-titre">"Réglages des accès"</h2>
            <p>
                "Durée de conservation des traces (12 mois au moins), seuils des alertes par métier. La valeur par défaut \
                 est indiquée ; chaque modification est journalisée."
            </p>
            <RetourEcriture ecriture=ecriture/>
            {move || match reglages.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement des réglages…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| reglages.refetch())/>
                }.into_any(),
                Some(Ok(r)) => view! {
                    <ul class="ui-liste">
                        {r.lignes.into_iter().map(|l| view! { <LigneReglage l=l ecriture=ecriture/> }).collect_view()}
                    </ul>
                    <Historique h=r.historique/>
                }.into_any(),
            }}
        </section>
    }
}

#[component]
fn LigneReglage(l: LigneReglageDto, ecriture: Ecriture) -> impl IntoView {
    let valeur = RwSignal::new(l.valeur.to_string());
    let code = l.code.clone();
    let (min, max) = (l.min, l.max);
    let invalide = RwSignal::new(None::<String>);
    let enregistrer = move || {
        let v = jours_depuis_texte(&valeur.get_untracked());
        if v < i64::from(min) || v > i64::from(max) {
            invalide.set(Some(format!("La valeur va de {min} à {max}.")));
            return;
        }
        invalide.set(None);
        let code = code.clone();
        ecriture.lancer(async move {
            session::client()
                .acces_reglage_modifier(&ReglageAccesBody {
                    reglage: code,
                    valeur: i32::try_from(v).unwrap_or(0),
                })
                .await?;
            Ok("Réglage enregistré.".to_string())
        });
    };
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{l.libelle.clone()}</span>
                <span class="ui-liste__detail">{format!("Par défaut : {} — de {} à {}", l.defaut, l.min, l.max)}</span>
                <ChampTexte id=format!("acces-reglage-{}", l.code) libelle="Valeur".to_string() valeur=valeur/>
                {move || invalide.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <Bouton variante=Variante::Secondaire
                    desactive=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(move |_| enregistrer())>
                    "Enregistrer"
                </Bouton>
            </div>
        </li>
    }
}

#[component]
fn Historique(h: Vec<ChangementReglageDto>) -> impl IntoView {
    if h.is_empty() {
        return view! { <p class="texte-attenue">"Aucune modification des réglages."</p> }
            .into_any();
    }
    view! {
        <h3>"Historique des modifications"</h3>
        <ul class="ui-liste">
            {h.into_iter().map(|c| view! {
                <li class="ui-liste__element">
                    <span class="ui-liste__detail">{format!(
                        "{} — {} : {} → {}",
                        quand(c.le),
                        c.reglage,
                        c.ancienne.unwrap_or_else(|| "(défaut)".to_string()),
                        c.nouvelle
                    )}</span>
                </li>
            }).collect_view()}
        </ul>
    }
    .into_any()
}

// ═══════════════════════════════════ Traces d'un sujet (fiche d'une demande) ═══════════════════════════════════

/// Section de la fiche d'une demande **ouverte** : les consultations du dossier de la personne concernée. Elles ne se lisent
/// que sur demande (la lecture est tracée côté serveur).
#[component]
pub fn SectionTracesDemande(demande: Uuid, ouverte: bool) -> impl IntoView {
    let moteur = use_droits_moteur();
    let resultat = RwSignal::new(None::<Result<Vec<TracesSujetDto>, ApiError>>);
    let en_cours = RwSignal::new(false);
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let charger = move || {
        en_cours.set(true);
        leptos::task::spawn_local(async move {
            resultat.set(Some(session::client().acces_traces_demande(demande).await));
            en_cours.set(false);
        });
    };
    view! {
        {move || (ouverte && moteur.get().peut_consulter_traces_acces()).then(|| view! {
            <section class="classe-section" aria-labelledby="acces-traces-demande-titre">
                <h2 id="acces-traces-demande-titre">"Consultations du dossier"</h2>
                <p>
                    "Qui a consulté le dossier de la personne concernée (fonction et service, jamais le nom). \
                     Ni santé, ni suivi psychologique. La lecture de cette liste est elle-même tracée."
                </p>
                <RetourEcriture ecriture=ecriture/>
                <Bouton variante=Variante::Secondaire
                    desactive=Signal::derive(move || en_cours.get())
                    on_click=Callback::new(move |_| charger())>
                    "Afficher les consultations"
                </Bouton>
                {move || match resultat.get() {
                    None => ().into_any(),
                    Some(Err(e)) => view! {
                        <Alerte niveau=Niveau::Erreur titre=format!("Consultations indisponibles : {e}")/>
                    }.into_any(),
                    Some(Ok(v)) => view! {
                        {v.into_iter().map(|s| {
                            let titre = if s.libelle.trim().is_empty() { "Personne concernée".to_string() } else { s.libelle.clone() };
                            view! {
                                <h3>{titre}</h3>
                                <ListeTraces lignes=s.lignes
                                    peut_arbitrer=moteur.get().peut_arbitrer_demandes_droits()
                                    demande=Some(demande) ecriture=Some(ecriture)/>
                            }
                        }).collect_view()}
                    }.into_any(),
                }}
            </section>
        })}
    }
}

#[component]
fn ListeTraces(
    lignes: Vec<TraceLigneDto>,
    peut_arbitrer: bool,
    demande: Option<Uuid>,
    ecriture: Option<Ecriture>,
) -> impl IntoView {
    if lignes.is_empty() {
        return view! { <p class="texte-attenue">"Aucune consultation."</p> }.into_any();
    }
    view! {
        <ul class="ui-liste">
            {lignes.into_iter().map(|l| view! {
                <LigneTrace l=l peut_arbitrer=peut_arbitrer demande=demande ecriture=ecriture/>
            }).collect_view()}
        </ul>
    }
    .into_any()
}

#[component]
fn LigneTrace(
    l: TraceLigneDto,
    peut_arbitrer: bool,
    demande: Option<Uuid>,
    ecriture: Option<Ecriture>,
) -> impl IntoView {
    let id = l.id;
    let motif = RwSignal::new(String::new());
    let jours = RwSignal::new("30".to_string());
    let invalide = RwSignal::new(None::<String>);
    let agir = peut_arbitrer && ecriture.is_some();
    let nom = move || {
        let (Some(d), Some(e)) = (demande, ecriture) else {
            return;
        };
        let m = motif.get_untracked();
        if let Err(msg) = report_recevable(&m, 1) {
            invalide.set(Some(msg.to_string()));
            return;
        }
        invalide.set(None);
        e.lancer(async move {
            session::client()
                .acces_nom_agent(
                    d,
                    &NomAgentBody {
                        trace_id: id,
                        motif: m,
                    },
                )
                .await?;
            Ok(
                "Nom communiqué pour cette demande : il figurera dans le prochain extrait."
                    .to_string(),
            )
        });
    };
    let differer = move || {
        let Some(e) = ecriture else { return };
        let (m, j) = (
            motif.get_untracked(),
            jours_depuis_texte(&jours.get_untracked()),
        );
        if let Err(msg) = report_recevable(&m, j) {
            invalide.set(Some(msg.to_string()));
            return;
        }
        invalide.set(None);
        e.lancer(async move {
            session::client()
                .acces_differer(&DifferBody {
                    trace_id: id,
                    motif: m,
                    jours: j,
                })
                .await?;
            Ok("Affichage différé pour la famille.".to_string())
        });
    };
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{format!("{} — {}", quand(l.le), qui(&l))}</span>
                <span class="ui-liste__detail">{format!("{} · {}", l.nature, l.section)}</span>
                {precisions(&l).into_iter().map(|p| view! { <span class="ui-liste__detail">{p}</span> }).collect_view()}
                {l.agent_nom.clone().map(|n| view! {
                    <span class="ui-liste__detail">{format!("Agent (nom communiqué sur arbitrage) : {n}")}</span>
                })}
                {agir.then(|| view! {
                    <div>
                        <ZoneTexte id=format!("acces-motif-{id}") libelle="Motif de la décision".to_string() valeur=motif/>
                        <ChampTexte id=format!("acces-jours-{id}") libelle="Durée du report (jours, 90 au plus)".to_string() valeur=jours/>
                        {move || invalide.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                        <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| nom())>
                            "Communiquer le nom de l'agent"
                        </Bouton>
                        <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| differer())>
                            "Différer l'affichage pour la famille"
                        </Bouton>
                    </div>
                })}
            </div>
        </li>
    }
}
