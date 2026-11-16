//! Fiche d'une absence (ABP-120, 134, 150, 159, 172) : en-tête (agent, dates, état, qui doit agir),
//! puis sections « décision et actions », pièces, fil, séances touchées, historique.
//!
//! Même fiche pour l'agent (sa propre absence : confirmer, prolonger, écourter, annuler, pièces) et pour
//! les valideurs et instructeurs (valider, refuser, requalifier, traiter, clore, rouvrir…). Le serveur
//! répond **404 uniforme** hors du cercle autorisé : l'écran dit « introuvable ou inaccessible », sans
//! distinguer les deux. Aucune décision n'est proposée sur sa propre absence (R-AP10).

use chrono::{NaiveDate, NaiveTime};
use ent_types::absences_personnels::{
    AbsenceDto, ChangerFinBody, DecisionBody, DecisionExterneBody, ImpactDto, JournalAbsenceDto,
    NatureDto, RequalifierBody,
};
use ent_web_api::{ActionDecision, ApiError};
use ent_web_ui::{
    AccesRefuse, Badge, Bouton, ChampTexte, Dialogue, EtatVide, LienBouton, OptionSegment,
    Segmente, Selection, Squelette, Taille, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use super::commun::GardePersonnel;
use super::fiche_pieces::{OngletFil, OngletPieces};
use super::modele::{
    actions_agent, actions_fiche, libelle_acteur, libelle_action_etape, libelle_action_journal,
    libelle_etat, libelle_etat_impact, libelle_periode, libelle_role_circuit, libelle_seance,
    libelle_source_circuit, libelle_traitement, nom_agent, spec_decision, ton_etat,
    ton_etat_impact, DECISIONS_EXTERNES, EMPLOYEURS_EXTERNES, TRAITEMENTS,
};
use crate::composants_metier::confirmation_detaillee::ConfirmationDetaillee;
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;

#[component]
pub fn PageFicheAbsence() -> impl IntoView {
    view! {
        <Title text="Absence"/>
        <GardePersonnel titre="Absence">
            <Chargement/>
        </GardePersonnel>
    }
}

#[component]
fn Chargement() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|s| Uuid::parse_str(&s).ok())
    };
    let version = RwSignal::new(0u32);
    // La section choisie survit au rechargement de la fiche après une écriture.
    let onglet = RwSignal::new("decision".to_string());
    let fiche = LocalResource::new(move || {
        let _ = version.get();
        async move {
            match id() {
                Some(i) => session::client().absp_absence(i).await,
                None => Err(ApiError::Introuvable("identifiant invalide".into())),
            }
        }
    });
    // « Est-ce la mienne ? » : décidé par la liste de MES absences (portée `moi`), jamais par un nom.
    let mes = LocalResource::new(|| async { session::client().absp_mes_absences().await });
    view! {
        {move || match fiche.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement de l'absence…"/> }.into_any(),
            Some(Err(ApiError::Introuvable(_))) => view! {
                <EtatVide
                    titre="Absence introuvable"
                    message="Elle n'existe pas, ou vous n'y avez pas accès."
                >
                    <LienBouton href="/web/personnel/absences" variante=Variante::Secondaire>"Mes absences"</LienBouton>
                </EtatVide>
            }
            .into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| fiche.refetch())/> }.into_any(),
            Some(Ok(a)) => {
                let identifiant = a.id;
                let mienne = match mes.get() {
                    None => None,
                    Some(Ok(l)) => Some(l.iter().any(|m| m.id == identifiant)),
                    Some(Err(_)) => None, // dans le doute, aucune écriture proposée
                };
                view! { <Fiche a=a mienne=mienne version=version onglet=onglet/> }.into_any()
            }
        }}
    }
}

#[component]
fn Fiche(
    a: AbsenceDto,
    mienne: Option<bool>,
    version: RwSignal<u32>,
    onglet: RwSignal<String>,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let id = a.id;
    let periode = libelle_periode(a.debut, a.fin);
    let agent = nom_agent(&a.agent_label);
    let etat = a.etat.clone();
    let est_mienne = mienne.unwrap_or(false);

    let etape =
        LocalResource::new(move || async move { session::client().absp_prochaine_etape(id).await });

    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="absp-entete" aria-labelledby="absp-fiche-titre">
            <div class="absp-carte__tete">
                <h2 id="absp-fiche-titre" class="absp-carte__titre">{if est_mienne { "Mon absence".to_string() } else { agent }}</h2>
                <Badge libelle=libelle_etat(&etat) ton=ton_etat(&etat)/>
            </div>
            <p class="absp-carte__detail">{periode}</p>
            {a.fin_provisoire.then(|| view! { <p class="absp-carte__detail">"La fin est une estimation : elle sera confirmée."</p> })}
            {a.nature_libelle.clone().map(|n| view! { <p class="absp-carte__detail"><strong>"Nature : "</strong>{n}</p> })}
            {a.texte_libre.clone().map(|t| view! { <p class="absp-carte__detail"><strong>"Précision : "</strong>{t}</p> })}
            {a.traitement.clone().map(|t| view! { <p class="absp-carte__detail"><strong>"Traitement des heures : "</strong>{libelle_traitement(&t)}</p> })}
            {(a.decision == "externe").then(|| view! {
                <p class="absp-carte__detail">"L'employeur (rectorat, DSDEN) décide : l'établissement enregistre sa décision."</p>
            })}
            {match a.confirmee_par.as_deref() {
                Some("rappel") => Some("Confirmée par un rappel tracé."),
                Some(_) => Some("Confirmée par la personne."),
                None => None,
            }
            .map(|t| view! { <p class="absp-carte__detail">{t}</p> })}
            {move || etape.get().and_then(Result::ok).map(|e| view! {
                <p class="absp-etape" role="status">
                    <strong>"Prochaine étape : "</strong>
                    {format!("{} {}.", libelle_acteur(&e.acteur), libelle_action_etape(&e.action))}
                </p>
            })}
        </section>

        <div class="absp-onglets">
        <Segmente
                nom="absp-onglet"
                legende="Section de la fiche"
                valeur=onglet
                options=vec![
                    OptionSegment::new("decision", "Décision et actions"),
                    OptionSegment::new("pieces", "Pièces"),
                    OptionSegment::new("fil", "Fil"),
                    OptionSegment::new("impact", "Séances touchées"),
                    OptionSegment::new("historique", "Historique"),
                ]
            />
        </div>
        {move || {
            let a = a.clone();
            match onglet.get().as_str() {
                "pieces" => view! { <OngletPieces a=a mienne=mienne instructeur=moteur.get().a("personnels.absences.instruire")/> }.into_any(),
                "fil" => view! { <OngletFil id=id/> }.into_any(),
                "impact" => view! { <OngletImpact id=id mienne=est_mienne a=a/> }.into_any(),
                "historique" => view! { <OngletHistorique id=id/> }.into_any(),
                _ => view! { <OngletDecision a=a mienne=mienne ecriture=ecriture/> }.into_any(),
            }
        }}
    }
}

// ───────────────────────────── Décision et actions ─────────────────────────────

#[component]
fn OngletDecision(a: AbsenceDto, mienne: Option<bool>, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let id = a.id;
    let ouvert_confirmer = RwSignal::new(false);
    let ouvert_fin = RwSignal::new(false);
    let fin_ecourter = RwSignal::new(false);
    let ouvert_requalifier = RwSignal::new(false);
    let ouvert_externe = RwSignal::new(false);
    let ouvert_arret = RwSignal::new(false);
    let decisions: Vec<(ActionDecision, RwSignal<bool>)> = [
        ActionDecision::Valider,
        ActionDecision::Refuser,
        ActionDecision::Precision,
        ActionDecision::Traiter,
        ActionDecision::Clore,
        ActionDecision::Rouvrir,
        ActionDecision::Annuler,
    ]
    .into_iter()
    .map(|d| (d, RwSignal::new(false)))
    .collect();
    let ouvrir = {
        let decisions = decisions.clone();
        move |action: ActionDecision| {
            if let Some((_, o)) = decisions.iter().find(|(d, _)| *d == action) {
                o.set(true);
            }
        }
    };
    let circuit = a.circuit.clone();
    let a_boutons = a.clone();

    let boutons = move || {
        // Tant que « est-ce la mienne ? » n'est pas connu, aucune écriture n'est proposée.
        let mienne = mienne?;
        let maintenant = chrono::Utc::now();
        let m = moteur.get();
        let f = actions_fiche(
            &a_boutons,
            m.a("personnels.absences.valider"),
            m.a("personnels.absences.instruire"),
            mienne,
            maintenant,
        );
        let g = actions_agent(&a_boutons, maintenant);
        let (annuler, prolonger, ecourter) = if mienne {
            (g.annuler, g.prolonger, g.ecourter)
        } else {
            (f.annuler, f.prolonger, f.ecourter)
        };
        let ouvrir = ouvrir.clone();
        let b = |libelle: &'static str, variante: Variante, cb: Callback<()>| {
            view! { <Bouton variante=variante taille=Taille::Compacte on_click=cb>{libelle}</Bouton> }
        };
        let rien = !(g.confirmer && mienne)
            && f.aucune()
            && !(mienne && (annuler || prolonger || ecourter));
        Some(
            view! {
                <div class="absp-boutons" role="group" aria-label="Actions sur cette absence">
                    {(mienne && g.confirmer).then(|| b("Confirmer mon absence", Variante::Primaire, Callback::new(move |_| ouvert_confirmer.set(true))))}
                    {f.valider.then(|| { let o = ouvrir.clone(); b("Valider", Variante::Primaire, Callback::new(move |_| o(ActionDecision::Valider))) })}
                    {f.refuser.then(|| { let o = ouvrir.clone(); b("Refuser", Variante::Danger, Callback::new(move |_| o(ActionDecision::Refuser))) })}
                    {f.demander_precision.then(|| { let o = ouvrir.clone(); b("Demander une précision", Variante::Secondaire, Callback::new(move |_| o(ActionDecision::Precision))) })}
                    {f.requalifier.then(|| b("Changer la nature", Variante::Secondaire, Callback::new(move |_| ouvert_requalifier.set(true))))}
                    {f.arret_recu.then(|| b("Noter la réception de l'arrêt", Variante::Secondaire, Callback::new(move |_| ouvert_arret.set(true))))}
                    {f.decision_externe.then(|| b("Enregistrer la décision de l'employeur", Variante::Primaire, Callback::new(move |_| ouvert_externe.set(true))))}
                    {f.traiter.then(|| { let o = ouvrir.clone(); b("Traiter", Variante::Primaire, Callback::new(move |_| o(ActionDecision::Traiter))) })}
                    {f.clore.then(|| { let o = ouvrir.clone(); b("Clôturer", Variante::Primaire, Callback::new(move |_| o(ActionDecision::Clore))) })}
                    {prolonger.then(|| b("Prolonger", Variante::Secondaire, Callback::new(move |_| { fin_ecourter.set(false); ouvert_fin.set(true); })))}
                    {ecourter.then(|| b("Reprise anticipée", Variante::Secondaire, Callback::new(move |_| { fin_ecourter.set(true); ouvert_fin.set(true); })))}
                    {f.rouvrir.then(|| { let o = ouvrir.clone(); b("Rouvrir", Variante::Secondaire, Callback::new(move |_| o(ActionDecision::Rouvrir))) })}
                    {annuler.then(|| { let o = ouvrir.clone(); b("Annuler l'absence", Variante::Danger, Callback::new(move |_| o(ActionDecision::Annuler))) })}
                </div>
                {rien.then(|| view! { <p class="texte-attenue">"Aucune action n'est possible pour vous sur cette absence, dans son état actuel."</p> })}
            }
            .into_any(),
        )
    };

    view! {
        <section class="absp-section" aria-labelledby="absp-decision">
            <h2 id="absp-decision" class="vs-section-titre">"Décision et actions"</h2>
            {boutons}
            {mienne.unwrap_or(false).then(|| view! {
                <p class="texte-attenue">"Vous ne pouvez pas valider votre propre absence : elle est instruite par votre valideur."</p>
            })}
        </section>
        <section class="absp-section" aria-labelledby="absp-circuit">
            <h2 id="absp-circuit" class="vs-section-titre">"Circuit"</h2>
            {if circuit.is_empty() {
                view! { <p class="texte-attenue">"Le circuit n'est pas communiqué."</p> }.into_any()
            } else {
                view! {
                    <ul class="absp-liste absp-liste--serree">
                        {circuit.iter().map(|c| view! {
                            <li class="absp-ligne">
                                <span>{libelle_role_circuit(&c.role)}</span>
                                <span class="texte-attenue">{libelle_source_circuit(&c.source)}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            }}
        </section>

        {decisions.into_iter().map(|(action, ouvert)| view! {
            <DialogueDecision action=action id_absence=id ouvert=ouvert ecriture=ecriture/>
        }).collect_view()}
        <DialogueConfirmer id_absence=id ouvert=ouvert_confirmer ecriture=ecriture/>
        <DialogueFin id_absence=id ouvert=ouvert_fin ecourter=fin_ecourter ecriture=ecriture/>
        <DialogueRequalifier id_absence=id ouvert=ouvert_requalifier ecriture=ecriture/>
        <DialogueDecisionExterne id_absence=id ouvert=ouvert_externe ecriture=ecriture/>
        <ConfirmationDetaillee
            ouvert=ouvert_arret
            id="absp-dlg-arret"
            titre="Noter la réception de l'arrêt"
            message="L'arrêt de travail est parvenu à l'établissement. L'échéance de 48 heures cesse de courir."
            libelle_confirmer="Noter la réception"
            on_confirmer=Callback::new(move |_| {
                ecriture.lancer(async move {
                    session::client().absp_arret_recu(id).await?;
                    Ok("Réception de l'arrêt notée.".to_string())
                });
            })
        />
    }
}

/// Boîte de dialogue d'une décision à corps commun : champs selon `spec_decision`.
#[component]
fn DialogueDecision(
    action: ActionDecision,
    id_absence: Uuid,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let spec = spec_decision(action);
    let prefixe = format!("absp-{}", action.segment());
    let traitement = RwSignal::new("sans_incidence".to_string());
    let raison = RwSignal::new(
        spec.raisons
            .first()
            .map(|(c, _)| (*c).to_string())
            .unwrap_or_default(),
    );
    let voie = RwSignal::new("rectorat".to_string());
    let note = RwSignal::new(String::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let n = note.get_untracked();
        let corps = DecisionBody {
            traitement: spec.avec_traitement.then(|| traitement.get_untracked()),
            raison_code: (!spec.raisons.is_empty()).then(|| raison.get_untracked()),
            note: (!n.trim().is_empty()).then(|| n.trim().to_string()),
            voie_transmission: spec.avec_voie.then(|| voie.get_untracked()),
        };
        ouvert.set(false);
        note.set(String::new());
        ecriture.lancer(async move {
            session::client()
                .absp_decider(id_absence, action, &corps)
                .await?;
            Ok(spec.message_ok.to_string())
        });
    };
    let options_traitement: Vec<(String, String)> = TRAITEMENTS
        .iter()
        .map(|c| ((*c).to_string(), libelle_traitement(c)))
        .collect();
    let options_raison: Vec<(String, String)> = spec
        .raisons
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let options_voie: Vec<(String, String)> = super::modele::VOIES_TRANSMISSION
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    view! {
        <Dialogue ouvert=ouvert titre=spec.titre id=format!("absp-dlg-{}", action.segment())>
            <form class="absp-formulaire" on:submit=envoyer>
                {spec.avec_traitement.then(|| view! {
                    <Selection
                        id=format!("{prefixe}-traitement")
                        libelle="Traitement des heures"
                        valeur=traitement
                        options=options_traitement
                        aide="Pour une nature qui ne laisse pas le choix, le traitement est imposé : le serveur l'applique ou l'explique."
                    />
                })}
                {(!spec.raisons.is_empty()).then(|| view! {
                    <Selection id=format!("{prefixe}-raison") libelle="Raison" valeur=raison options=options_raison/>
                })}
                {spec.avec_voie.then(|| view! {
                    <Selection id=format!("{prefixe}-voie") libelle="Voie de transmission" valeur=voie options=options_voie/>
                })}
                <ZoneTexte
                    id=format!("{prefixe}-note")
                    libelle="Note (facultatif)"
                    valeur=note
                    aide="Visible des seules personnes qui instruisent. Aucune information de santé."
                />
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true variante=if spec.danger { Variante::Danger } else { Variante::Primaire }>
                        {spec.bouton}
                    </Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

/// Confirmation d'une absence signalée : l'agent peut en préciser la nature (ABP-112, 117).
#[component]
fn DialogueConfirmer(
    id_absence: Uuid,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let nature = RwSignal::new(String::new());
    let natures = LocalResource::new(|| async { session::client().absp_natures().await });
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let n = nature.get_untracked();
        ouvert.set(false);
        ecriture.lancer(async move {
            session::client()
                .absp_confirmer(id_absence, (!n.is_empty()).then_some(n))
                .await?;
            Ok("Absence confirmée.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Confirmer mon absence" id="absp-dlg-confirmer">
            <form class="absp-formulaire" on:submit=envoyer>
                <p>"Cette absence a été signalée à votre nom. Confirmez-la, et précisez sa nature si vous le souhaitez."</p>
                {move || natures.get().and_then(Result::ok).map(|l| {
                    let mut o = vec![(String::new(), "Je préciserai plus tard".to_string())];
                    o.extend(l.into_iter().map(|n: NatureDto| (n.code, n.libelle)));
                    view! { <Selection id="absp-confirmer-nature" libelle="Nature" valeur=nature options=o/> }
                })}
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Confirmer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

/// Nouvelle dernière journée : prolonger (ABP-110) ou reprise anticipée (ABP-111).
#[component]
fn DialogueFin(
    id_absence: Uuid,
    ouvert: RwSignal<bool>,
    ecourter: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let jour = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Ok(fin) = NaiveDate::parse_from_str(jour.get_untracked().trim(), "%Y-%m-%d") else {
            erreur.set(Some("Indiquez une date valide.".into()));
            return;
        };
        erreur.set(None);
        let reprise = ecourter.get_untracked();
        ouvert.set(false);
        ecriture.lancer(async move {
            session::client()
                .absp_changer_fin(
                    id_absence,
                    reprise,
                    &ChangerFinBody {
                        fin,
                        heure_fin: None::<NaiveTime>,
                    },
                )
                .await?;
            Ok(if reprise {
                "Reprise anticipée enregistrée : les séances au-delà sont à reprendre.".to_string()
            } else {
                "Absence prolongée : les séances touchées sont recalculées.".to_string()
            })
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Dernière journée d'absence" id="absp-dlg-fin">
            <form class="absp-formulaire" on:submit=envoyer novalidate=true>
                <p>
                    {move || if ecourter.get() {
                        "Reprise anticipée : indiquez la nouvelle dernière journée d'absence. Les réponses déjà posées au-delà ne sont jamais levées seules."
                    } else {
                        "Prolongation : indiquez la nouvelle dernière journée d'absence (incluse)."
                    }}
                </p>
                <ChampTexte
                    id="absp-fin-jour"
                    libelle="Nouvelle dernière journée"
                    genre="date"
                    valeur=jour
                    requis=true
                    erreur=Signal::derive(move || erreur.get())
                />
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

/// Changer la nature (ABP-128) : le traitement est recalculé, les corrections transmises.
#[component]
fn DialogueRequalifier(
    id_absence: Uuid,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let nature = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let natures = LocalResource::new(|| async { session::client().absp_natures().await });
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let code = nature.get_untracked();
        if code.is_empty() {
            return;
        }
        let n = note.get_untracked();
        ouvert.set(false);
        ecriture.lancer(async move {
            session::client()
                .absp_requalifier(
                    id_absence,
                    &RequalifierBody {
                        nature_code: code,
                        raison_code: None,
                        note: (!n.trim().is_empty()).then(|| n.trim().to_string()),
                    },
                )
                .await?;
            Ok("Nature modifiée : le traitement des heures est recalculé.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Changer la nature" id="absp-dlg-requalifier">
            <form class="absp-formulaire" on:submit=envoyer>
                <p>"Le traitement des heures est recalculé et les corrections sont transmises. L'historique garde l'ancienne nature."</p>
                {move || natures.get().and_then(Result::ok).map(|l| {
                    let mut o = vec![(String::new(), "Choisir…".to_string())];
                    o.extend(l.into_iter().map(|n: NatureDto| (n.code, n.libelle)));
                    view! { <Selection id="absp-requalifier-nature" libelle="Nouvelle nature" valeur=nature options=o requis=true/> }
                })}
                <ZoneTexte id="absp-requalifier-note" libelle="Note (facultatif)" valeur=note/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Changer la nature"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

/// Décision de l'employeur (rectorat, DSDEN) enregistrée par l'établissement (ABP-288, 289).
#[component]
fn DialogueDecisionExterne(
    id_absence: Uuid,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let employeur = RwSignal::new("rectorat".to_string());
    let decision = RwSignal::new("accordee".to_string());
    let nature = RwSignal::new(String::new());
    let le = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let natures = LocalResource::new(|| async { session::client().absp_natures().await });
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Ok(jour) = NaiveDate::parse_from_str(le.get_untracked().trim(), "%Y-%m-%d") else {
            erreur.set(Some("Indiquez la date de la décision.".into()));
            return;
        };
        let requalifiee = decision.get_untracked() == "requalifiee";
        let code = nature.get_untracked();
        if requalifiee && code.is_empty() {
            erreur.set(Some("Choisissez la nouvelle nature.".into()));
            return;
        }
        erreur.set(None);
        let corps = DecisionExterneBody {
            employeur: employeur.get_untracked(),
            decision: decision.get_untracked(),
            nature_code: requalifiee.then_some(code),
            le: jour,
        };
        ouvert.set(false);
        ecriture.lancer(async move {
            session::client()
                .absp_decision_externe(id_absence, &corps)
                .await?;
            Ok("Décision de l'employeur enregistrée.".to_string())
        });
    };
    let paires = |l: &[(&str, &str)]| -> Vec<(String, String)> {
        l.iter()
            .map(|(c, t)| ((*c).to_string(), (*t).to_string()))
            .collect()
    };
    view! {
        <Dialogue ouvert=ouvert titre="Décision de l'employeur" id="absp-dlg-externe">
            <form class="absp-formulaire" on:submit=envoyer novalidate=true>
                <Selection id="absp-ext-employeur" libelle="Qui a décidé ?" valeur=employeur options=paires(EMPLOYEURS_EXTERNES)/>
                <Selection id="absp-ext-decision" libelle="Décision" valeur=decision options=paires(DECISIONS_EXTERNES)/>
                {move || (decision.get() == "requalifiee").then(|| {
                    let mut o = vec![(String::new(), "Choisir…".to_string())];
                    if let Some(Ok(l)) = natures.get() {
                        o.extend(l.into_iter().map(|n: NatureDto| (n.code, n.libelle)));
                    }
                    view! { <Selection id="absp-ext-nature" libelle="Nouvelle nature" valeur=nature options=o/> }
                })}
                <ChampTexte
                    id="absp-ext-date"
                    libelle="Date de la décision"
                    genre="date"
                    valeur=le
                    requis=true
                    erreur=Signal::derive(move || erreur.get())
                />
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

// ───────────────────────────── Séances touchées ─────────────────────────────

#[component]
fn OngletImpact(id: Uuid, mienne: bool, a: AbsenceDto) -> impl IntoView {
    let impacts =
        LocalResource::new(move || async move { session::client().absp_impact(id).await });
    let retour = LocalResource::new(move || async move {
        if mienne {
            session::client().absp_retour(id).await.map(Some)
        } else {
            Ok(None)
        }
    });
    let finie = a.fin < chrono::Utc::now();
    view! {
        <section class="absp-section" aria-labelledby="absp-impact">
            <h2 id="absp-impact" class="vs-section-titre">"Séances touchées"</h2>
            <p class="texte-attenue">"Vue d'organisation : elle ne dit jamais la nature de l'absence."</p>
            {move || match impacts.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des séances…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| impacts.refetch())/> }.into_any(),
                Some(Ok(v)) => liste_impacts(&v, "Aucune séance n'est touchée par cette absence."),
            }}
        </section>
        {(mienne && finie).then(|| view! {
            <section class="absp-section" aria-labelledby="absp-retour">
                <h2 id="absp-retour" class="vs-section-titre">"Au retour : ce qui a été fait"</h2>
                {move || match retour.get() {
                    None => view! { <Squelette lignes=2 libelle="Chargement…"/> }.into_any(),
                    Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| retour.refetch())/> }.into_any(),
                    Some(Ok(None)) => ().into_any(),
                    Some(Ok(Some(v))) => liste_impacts(&v, "Aucune séance à reprendre."),
                }}
            </section>
        })}
    }
}

fn liste_impacts(v: &[ImpactDto], vide: &'static str) -> AnyView {
    if v.is_empty() {
        return view! { <p class="texte-attenue">{vide}</p> }.into_any();
    }
    view! {
        <ul class="absp-liste absp-liste--serree">
            {v.iter().map(|i| {
                let jour = i.seance.as_ref().map(|s| crate::noyau::temps::date_longue(s.jour));
                view! {
                    <li class="absp-ligne">
                        <span>
                            {jour.map(|j| format!("{j} · ")).unwrap_or_default()}
                            {libelle_seance(i)}
                        </span>
                        <Badge libelle=libelle_etat_impact(&i.etat) ton=ton_etat_impact(&i.etat)/>
                        {i.a_revoir.then(|| view! { <span class="texte-attenue">"À revoir : l'emploi du temps a changé"</span> })}
                    </li>
                }
            }).collect_view()}
        </ul>
    }
    .into_any()
}

// ───────────────────────────── Historique ─────────────────────────────

#[component]
fn OngletHistorique(id: Uuid) -> impl IntoView {
    let journal =
        LocalResource::new(move || async move { session::client().absp_historique(id).await });
    view! {
        <section class="absp-section" aria-labelledby="absp-historique">
            <h2 id="absp-historique" class="vs-section-titre">"Historique"</h2>
            <p class="texte-attenue">"Journal en ajout seul : rien n'est jamais effacé."</p>
            {move || match journal.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement de l'historique…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_))) => view! { <AccesRefuse message="L'historique est réservé aux personnes qui instruisent cette absence."/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| journal.refetch())/> }.into_any(),
                Some(Ok(v)) if v.is_empty() => view! { <p class="texte-attenue">"Aucun fait enregistré."</p> }.into_any(),
                Some(Ok(v)) => view! {
                    <ol class="absp-liste absp-liste--serree">
                        {v.iter().rev().map(|f| view! { <LigneJournal f=f.clone()/> }).collect_view()}
                    </ol>
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn LigneJournal(f: JournalAbsenceDto) -> impl IntoView {
    let detail = match (&f.valeur_avant, &f.valeur_apres) {
        (Some(a), Some(b)) => Some(format!("{a} → {b}")),
        (None, Some(b)) => Some(b.clone()),
        _ => None,
    };
    view! {
        <li class="absp-ligne">
            <span>
                <strong>{libelle_action_journal(&f.action)}</strong>
                {detail.map(|d| format!(" : {d}"))}
            </span>
            <span class="texte-attenue">{date_heure_paris(f.le)}</span>
        </li>
    }
}
