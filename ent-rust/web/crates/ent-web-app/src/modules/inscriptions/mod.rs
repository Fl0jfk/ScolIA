//! Admissions, inscriptions et réinscriptions (socle 2, lot S2-3 ; D-014).
//!
//! Écrans :
//! - `/web/preinscription` et `/web/invitation` (publics, `famille.rs`) ;
//! - `/web/inscriptions` (personnel) : campagnes, dossiers, instruction (pièces, étapes,
//!   commentaires, relance), décision et validation ;
//! - `/web/reinscription` (responsables connectés) : confirmer la réinscription ou
//!   annoncer un départ.
//!
//! Routes d'`ent-api` : `/inscriptions/*`, `/preinscription/*`. Le serveur décide de tout
//! (droits `inscriptions.gerer` / `inscriptions.decider`, transitions, effets de la
//! validation) ; l'écran n'affiche que les actions annoncées par le dossier.

mod famille;
pub mod modele;
mod validation;

pub use famille::{PageInvitation, PagePreinscription};

use ent_web_api::types_inscriptions::{
    CampagneBody, CampagneDto, CommentaireBody, ConfirmationReinscriptionBody, DecisionBody,
    DepartBody, DossierDto, PieceDecisionBody, StatutBody,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    use_notifications, AccesRefuse, Alerte, Badge, Bouton, Case, ChampTexte, Dialogue, EnTetePage, EtatVide,
    Niveau, Selection, Squelette, Taille, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::{reconnexion, session};
use modele::{
    piece_a_decider,
    libelle_nature, libelle_regime, libelle_statut, libelle_statut_campagne, niveaux_depuis_texte,
    peut, pieces_depuis_texte, ton_statut, REGIMES,
};

/// Message d'échec d'une écriture ; une session perdue ouvre la reconnexion.
fn message_echec(e: ApiError) -> String {
    match e {
        ApiError::NonConnecte => {
            reconnexion::signaler_session_perdue();
            reconnexion::MESSAGE_SESSION_EXPIREE.into()
        }
        autre => autre.message_utilisateur(),
    }
}

// ---------------------------------------------------------------------------
// Personnel : campagnes et instruction
// ---------------------------------------------------------------------------

#[component]
pub fn PageInscriptions() -> impl IntoView {
    let campagnes =
        LocalResource::new(|| async { session::client().campagnes_inscription().await });
    let recharger = Callback::new(move |_| campagnes.refetch());
    let choisie = RwSignal::new(None::<Uuid>);
    view! {
        <Title text="Inscriptions"/>
        <EnTetePage titre="Inscriptions" sous_titre="Campagnes d'admission et de réinscription, instruction des dossiers".to_string()/>
        {move || match campagnes.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement des campagnes…"/> }.into_any(),
            Some(Err(ApiError::Interdit(_))) => view! {
                <AccesRefuse message="La gestion des inscriptions est réservée aux personnes qui en ont le droit."/>
            }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
            Some(Ok(liste)) => view! {
                <ListeCampagnes campagnes=liste choisie=choisie recharger=recharger/>
                <NouvelleCampagne recharger=recharger/>
                {move || choisie.get().map(|c| view! { <DossiersCampagne campagne_id=c/> })}
            }.into_any(),
        }}
    }
}

#[component]
fn ListeCampagnes(
    campagnes: Vec<CampagneDto>,
    choisie: RwSignal<Option<Uuid>>,
    recharger: Callback<()>,
) -> impl IntoView {
    let notifications = use_notifications();
    // Ouvrir ou fermer une campagne touche les familles : confirmation avant d'agir.
    let confirmer = RwSignal::new(false);
    let cible = RwSignal::new(None::<(Uuid, bool, String)>);
    let appliquer = Callback::new(move |_| {
        let Some((id, ouverte, _)) = cible.get_untracked() else { return };
        confirmer.set(false);
        spawn_local(async move {
            let client = session::client();
            let r = if ouverte {
                client.fermer_campagne(id).await.map(|_| "Campagne fermée.".to_string())
            } else {
                client.ouvrir_campagne(id).await.map(|o| format!("Campagne ouverte ({} dossiers préparés).", o.dossiers_prepares))
            };
            match r {
                Ok(m) => { notifications.succes(m); recharger.run(()); }
                Err(e) => { notifications.info(message_echec(e)); }
            }
        })
    });
    if campagnes.is_empty() {
        return view! { <EtatVide titre="Aucune campagne" message="Créez une campagne d'admission ou de réinscription ci-dessous."/> }.into_any();
    }
    view! {
        <section aria-labelledby="ins-campagnes" class="pile">
            <h2 id="ins-campagnes">"Campagnes"</h2>
            <table class="tableau">
                <thead><tr>
                    <th scope="col">"Campagne"</th><th scope="col">"Nature"</th><th scope="col">"Année"</th>
                    <th scope="col">"État"</th><th scope="col">"Dossiers"</th><th scope="col">"Actions"</th>
                </tr></thead>
                <tbody>
                    {campagnes.into_iter().map(|c| {
                        let id = c.id;
                        let total: i64 = c.compteurs.iter().map(|x| x.nombre).sum();
                        let ouverte = c.statut == "ouverte";
                        let libelle_campagne = c.libelle.clone();
                        let basculer = Callback::new(move |_| {
                            cible.set(Some((id, ouverte, libelle_campagne.clone())));
                            confirmer.set(true);
                        });
                        view! {
                            <tr>
                                <th scope="row">{c.libelle.clone()}</th>
                                <td>{libelle_nature(&c.nature)}</td>
                                <td>{c.annee_cible.clone()}</td>
                                <td><Badge libelle=libelle_statut_campagne(&c.statut) ton=ton_statut(&c.statut)/></td>
                                <td>{total}</td>
                                <td>
                                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire on_click=Callback::new(move |_| choisie.set(Some(id)))>"Dossiers"</Bouton>
                                    <Bouton taille=Taille::Compacte variante=Variante::Discret on_click=basculer>{if ouverte { "Fermer" } else { "Ouvrir" }}</Bouton>
                                </td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <Dialogue ouvert=confirmer titre="Confirmer l'action" id="ins-confirmer-campagne">
                <p>{move || match cible.get() {
                    Some((_, true, l)) => format!("Fermer la campagne « {l} » ? Les familles ne pourront plus déposer de nouveau dossier ; les dossiers déjà déposés restent instruits."),
                    Some((_, false, l)) => format!("Ouvrir la campagne « {l} » ? Les dossiers sont préparés et les familles peuvent déposer leur demande."),
                    None => String::new(),
                }}</p>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| confirmer.set(false))>"Annuler"</Bouton>
                    <Bouton variante=Variante::Primaire on_click=appliquer>{move || if matches!(cible.get(), Some((_, true, _))) { "Fermer la campagne" } else { "Ouvrir la campagne" }}</Bouton>
                </div>
            </Dialogue>
        </section>
    }.into_any()
}

#[component]
fn NouvelleCampagne(recharger: Callback<()>) -> impl IntoView {
    let notifications = use_notifications();
    let nature = RwSignal::new("admission".to_string());
    let libelle = RwSignal::new(String::new());
    let annee = RwSignal::new(String::new());
    let niveaux = RwSignal::new(String::new());
    let pieces = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let creer = Callback::new(move |_| {
        let n = match niveaux_depuis_texte(&niveaux.get_untracked()) {
            Ok(n) => n,
            Err(m) => return erreur.set(Some(m)),
        };
        let p = match pieces_depuis_texte(&pieces.get_untracked()) {
            Ok(p) => p,
            Err(m) => return erreur.set(Some(m)),
        };
        let m = message.get_untracked();
        let b = CampagneBody {
            nature: nature.get_untracked(),
            libelle: libelle.get_untracked(),
            annee_cible: annee.get_untracked().trim().to_string(),
            message_famille: (!m.trim().is_empty()).then_some(m),
            niveaux: n,
            pieces: p,
            ..Default::default()
        };
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client().creer_campagne(&b).await;
            en_cours.set(false);
            match r {
                Ok(c) => {
                    notifications.succes(format!("Campagne « {} » créée (brouillon).", c.libelle));
                    libelle.set(String::new());
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    });
    view! {
        <details class="pile">
            <summary>"Nouvelle campagne"</summary>
            <form class="pile" on:submit=move |ev| { ev.prevent_default(); creer.run(()); }>
                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <Selection id="ins-nature" libelle="Nature" valeur=nature options=vec![
                    ("admission".to_string(), "Admission (nouveaux élèves)".to_string()),
                    ("reinscription".to_string(), "Réinscription (élèves actuels)".to_string()),
                ]/>
                <ChampTexte id="ins-libelle" libelle="Libellé" valeur=libelle requis=true/>
                <ChampTexte id="ins-annee" libelle="Année cible" valeur=annee requis=true placeholder="2027-2028"/>
                <ChampTexte id="ins-niveaux" libelle="Niveaux ouverts" valeur=niveaux aide="Codes séparés par des virgules, capacité facultative : 6EME:120, 5EME"/>
                <ZoneTexte id="ins-pieces" libelle="Pièces demandées" valeur=pieces aide="Une par ligne : code ; libellé ; facultative (obligatoire si rien n'est précisé)"/>
                <ZoneTexte id="ins-message" libelle="Message aux familles" valeur=message/>
                <Bouton variante=Variante::Primaire soumettre=true en_cours=en_cours>"Créer la campagne"</Bouton>
            </form>
        </details>
    }
}

#[component]
fn DossiersCampagne(campagne_id: Uuid) -> impl IntoView {
    let dossiers = LocalResource::new(move || async move {
        session::client()
            .dossiers_inscription(Some(campagne_id))
            .await
    });
    let recharger = Callback::new(move |_| dossiers.refetch());
    let ouvert = RwSignal::new(None::<Uuid>);
    view! {
        <section aria-labelledby="ins-dossiers" class="pile">
            <h2 id="ins-dossiers">"Dossiers"</h2>
            {move || match dossiers.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement des dossiers…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun dossier pour cette campagne"/> }.into_any(),
                Some(Ok(l)) => view! {
                    <table class="tableau">
                        <thead><tr>
                            <th scope="col">"Élève"</th><th scope="col">"Niveau"</th><th scope="col">"État"</th>
                            <th scope="col">"Pièces manquantes"</th><th scope="col"><span class="visuellement-cache">"Ouvrir"</span></th>
                        </tr></thead>
                        <tbody>
                            {l.into_iter().map(|d| {
                                let id = d.id;
                                view! {
                                    <tr>
                                        <th scope="row">{format!("{} {}", d.eleve_nom.to_uppercase(), d.eleve_prenom)}</th>
                                        <td>{d.niveau_demande.clone().unwrap_or_default()}</td>
                                        <td><Badge libelle=libelle_statut(&d.statut) ton=ton_statut(&d.statut)/></td>
                                        <td>{d.pieces_manquantes}</td>
                                        <td><Bouton taille=Taille::Compacte variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(Some(id)))>"Instruire"</Bouton></td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                }.into_any(),
            }}
            {move || ouvert.get().map(|id| view! { <Instruction id=id apres=recharger/> })}
        </section>
    }
}

#[component]
fn Instruction(id: Uuid, apres: Callback<()>) -> impl IntoView {
    let dossier =
        LocalResource::new(move || async move { session::client().dossier_inscription(id).await });
    let recharger = Callback::new(move |_| {
        dossier.refetch();
        apres.run(());
    });
    view! {
        <section aria-labelledby="ins-instruction" class="pile carte">
            {move || match dossier.get() {
                None => view! { <Squelette lignes=6 libelle="Chargement du dossier…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
                Some(Ok(d)) => view! { <DetailDossier dossier=d recharger=recharger/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn DetailDossier(dossier: DossierDto, recharger: Callback<()>) -> impl IntoView {
    let notifications = use_notifications();
    let id = dossier.id;
    let erreur = RwSignal::new(None::<String>);
    let motif = RwSignal::new(String::new());
    let commentaire = RwSignal::new(String::new());
    let visible = RwSignal::new(false);
    let decision = RwSignal::new("admis".to_string());
    let executer = move |futur: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, ApiError>>>,
    >| {
        spawn_local(async move {
            match futur.await {
                Ok(m) => {
                    erreur.set(None);
                    notifications.succes(m);
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        })
    };
    let motif_opt = move || {
        let m = motif.get_untracked();
        (!m.trim().is_empty()).then_some(m)
    };
    let statut = move |vers: &'static str| {
        Callback::new(move |_| {
            let b = StatutBody {
                statut: vers.into(),
                motif: motif_opt(),
            };
            executer(Box::pin(async move {
                session::client()
                    .changer_statut_dossier(id, &b)
                    .await
                    .map(|_| "Étape mise à jour.".to_string())
            }))
        })
    };
    let actions = StoredValue::new(dossier.actions.clone());
    let noms = StoredValue::new(
        dossier
            .responsables
            .iter()
            .map(|r| {
                (
                    r.rang,
                    format!(
                        "{} {}",
                        r.formulaire.prenom.clone().unwrap_or_default(),
                        r.formulaire.nom
                    )
                    .trim()
                    .to_string(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let a = move |code: &str| actions.with_value(|v| v.iter().any(|x| x == code));
    let titre = format!(
        "{} {} — {}",
        dossier.eleve.prenom,
        dossier.eleve.nom.to_uppercase(),
        libelle_nature(&dossier.nature)
    );
    view! {
        <h3 id="ins-instruction">{titre}</h3>
        <p>
            <Badge libelle=libelle_statut(&dossier.statut) ton=ton_statut(&dossier.statut)/>
            {dossier.eleve.niveau_demande.clone().map(|n| format!(" · niveau {n}"))}
            {dossier.eleve.regime_demande.clone().map(|r| format!(" · {}", libelle_regime(&r)))}
        </p>
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        <h4>"Responsables"</h4>
        <ul class="liste">
            {dossier.responsables.iter().map(|r| {
                let reserve = r.formulaire.reserve.clone().unwrap_or_default();
                view! {
                    <li>
                        {format!(
                            "{} {} {}{}{}",
                            r.formulaire.prenom.clone().unwrap_or_default(),
                            r.formulaire.nom,
                            r.formulaire.email.clone().map(|e| format!("· {e}")).unwrap_or_else(|| "· sans e-mail".into()),
                            if r.formulaire.meme_foyer { "" } else { " · foyer distinct" },
                            r.invitation.clone().map(|i| format!(" · invitation {i}")).unwrap_or_default(),
                        )}
                        {reserve.ne_pas_solliciter.then(|| view! { " " <Badge libelle="À ne pas solliciter (à confirmer par la direction)" ton=TonEtat::Avertissement/> })}
                        {reserve.adresse_a_proteger.then(|| view! { " " <Badge libelle="Adresse protégée" ton=TonEtat::Avertissement/> })}
                    </li>
                }
            }).collect_view()}
        </ul>
        <h4>"Pièces"</h4>
        <table class="tableau">
            <thead><tr><th scope="col">"Pièce"</th><th scope="col">"État"</th><th scope="col">"Fichier"</th><th scope="col">"Décision"</th></tr></thead>
            <tbody>
                {dossier.pieces.iter().map(|p| {
                    let code = StoredValue::new(p.code.clone());
                    let decider = move |statut: &'static str| Callback::new(move |_| {
                        let b = PieceDecisionBody { statut: statut.into(), motif: motif_opt() };
                        let c = code.get_value();
                        executer(Box::pin(async move {
                            session::client().decider_piece(id, &c, &b).await.map(|_| "Pièce mise à jour.".to_string())
                        }))
                    });
                    view! {
                        <tr>
                            <th scope="row">{format!("{}{}", p.libelle, if p.obligatoire { " *" } else { "" })}</th>
                            <td><Badge libelle=libelle_statut(&p.statut) ton=ton_statut(&p.statut)/></td>
                            <td>{p.nom_fichier.clone().unwrap_or_default()}</td>
                            <td>{(a("pieces") && piece_a_decider(&p.statut)).then(|| view! {
                                <Bouton taille=Taille::Compacte variante=Variante::Secondaire on_click=decider("acceptee")>"Accepter"</Bouton>
                                <Bouton taille=Taille::Compacte variante=Variante::Discret on_click=decider("refusee")>"Refuser (motif)"</Bouton>
                            })}</td>
                        </tr>
                    }
                }).collect_view()}
            </tbody>
        </table>
        <ChampTexte id="ins-motif" libelle="Motif (refus de pièce, dossier incomplet, décision)" valeur=motif/>
        <div class="barre-actions">
            {a("statut:en_instruction").then(|| view! { <Bouton variante=Variante::Secondaire on_click=statut("en_instruction")>"Passer en instruction"</Bouton> })}
            {a("statut:incomplet").then(|| view! { <Bouton variante=Variante::Secondaire on_click=statut("incomplet")>"Déclarer incomplet"</Bouton> })}
            {a("relancer").then(|| view! {
                <Bouton variante=Variante::Discret on_click=Callback::new(move |_| executer(Box::pin(async move {
                    session::client().relancer_dossier(id).await.map(|_| "Relance envoyée.".to_string())
                })))>"Relancer les pièces manquantes"</Bouton>
            })}
        </div>
        {a("decider").then(|| view! {
            <form class="pile" on:submit=move |ev| {
                ev.prevent_default();
                let b = DecisionBody { decision: decision.get_untracked(), motif: motif_opt() };
                executer(Box::pin(async move { session::client().decider_dossier(id, &b).await.map(|_| "Décision enregistrée.".to_string()) }));
            }>
                <Selection id="ins-decision" libelle="Décision" valeur=decision options=vec![
                    ("admis".to_string(), "Admis".to_string()),
                    ("liste_attente".to_string(), "Liste d'attente (motif)".to_string()),
                    ("refuse".to_string(), "Refusé (motif)".to_string()),
                ]/>
                <Bouton variante=Variante::Primaire soumettre=true>"Enregistrer la décision"</Bouton>
            </form>
        })}
        {a("valider").then(|| view! {
            <validation::ValidationDossier id=id noms=noms.get_value() recharger=recharger/>
        })}
        {a("commenter").then(|| view! {
            <form class="pile" on:submit=move |ev| {
                ev.prevent_default();
                let b = CommentaireBody { texte: commentaire.get_untracked(), visible_famille: visible.get_untracked() };
                commentaire.set(String::new());
                executer(Box::pin(async move { session::client().commenter_dossier(id, &b).await.map(|_| "Commentaire ajouté.".to_string()) }));
            }>
                <ZoneTexte id="ins-commentaire" libelle="Commentaire" valeur=commentaire requis=true/>
                <Case id="ins-visible" libelle="Visible de la famille" coche=visible/>
                <Bouton variante=Variante::Secondaire soumettre=true>"Ajouter"</Bouton>
            </form>
        })}
        <h4>"Suivi"</h4>
        <ol class="liste">
            {dossier.suivi.iter().map(|s| view! {
                <li>{format!(
                    "{} — {}{}",
                    s.quand.format("%d/%m/%Y %H:%M"),
                    s.texte.clone().unwrap_or_else(|| libelle_statut(s.statut_apres.as_deref().unwrap_or_default()).to_string()),
                    if s.visible_famille { "" } else { " (interne)" },
                )}</li>
            }).collect_view()}
        </ol>
    }
}

// ---------------------------------------------------------------------------
// Responsables connectés : réinscription
// ---------------------------------------------------------------------------

#[component]
pub fn PageReinscription() -> impl IntoView {
    let dossiers = LocalResource::new(|| async { session::client().mes_reinscriptions().await });
    let recharger = Callback::new(move |_| dossiers.refetch());
    view! {
        <Title text="Réinscription"/>
        <EnTetePage titre="Réinscription" sous_titre="Confirmer la scolarité de l'année prochaine".to_string()/>
        {move || match dossiers.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement…"/> }.into_any(),
            Some(Err(ApiError::Interdit(_))) => view! { <AccesRefuse message="Réservé aux responsables légaux."/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune réinscription en cours" message="L'établissement ouvrira la campagne de réinscription en temps voulu."/>
            }.into_any(),
            Some(Ok(l)) => l.into_iter().map(|d| view! { <CarteReinscription dossier=d recharger=recharger/> }).collect_view().into_any(),
        }}
    }
}

#[component]
fn CarteReinscription(dossier: DossierDto, recharger: Callback<()>) -> impl IntoView {
    let notifications = use_notifications();
    let id = dossier.id;
    let regime = RwSignal::new(dossier.eleve.regime_demande.clone().unwrap_or_default());
    let motif = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let options: Vec<(String, String)> = std::iter::once((String::new(), "Inchangé".to_string()))
        .chain(
            REGIMES
                .iter()
                .map(|r| (r.to_string(), libelle_regime(r).to_string())),
        )
        .collect();
    let confirmer = Callback::new(move |_| {
        let r = regime.get_untracked();
        let b = ConfirmationReinscriptionBody {
            regime: (!r.is_empty()).then_some(r),
        };
        spawn_local(async move {
            match session::client().confirmer_reinscription(id, &b).await {
                Ok(_) => {
                    notifications.succes("Réinscription confirmée.".to_string());
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        })
    });
    let depart = Callback::new(move |_| {
        let b = DepartBody {
            motif: motif.get_untracked(),
            date_depart: None,
        };
        spawn_local(async move {
            match session::client().annoncer_depart(id, &b).await {
                Ok(_) => {
                    notifications.succes("Départ annoncé à l'établissement.".to_string());
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        })
    });
    let modifiable = peut(&dossier, "confirmer");
    let pid = format!("rei-regime-{id}");
    let mid = format!("rei-motif-{id}");
    view! {
        <article class="carte pile" aria-label=format!("Réinscription de {}", dossier.eleve.prenom)>
            <h2>{format!("{} {}", dossier.eleve.prenom, dossier.eleve.nom.to_uppercase())}</h2>
            <p>
                {format!("{} · ", dossier.campagne_libelle)}
                <Badge libelle=libelle_statut(&dossier.statut) ton=ton_statut(&dossier.statut)/>
            </p>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {modifiable.then(|| view! {
                <Selection id=pid.clone() libelle="Régime l'an prochain" valeur=regime options=options.clone()/>
                <Bouton variante=Variante::Primaire on_click=confirmer>"Confirmer la réinscription"</Bouton>
                <details>
                    <summary>"Mon enfant quitte l'établissement"</summary>
                    <ChampTexte id=mid.clone() libelle="Motif du départ" valeur=motif requis=true/>
                    <Bouton variante=Variante::Danger on_click=depart>"Annoncer le départ"</Bouton>
                </details>
            })}
        </article>
    }
}
