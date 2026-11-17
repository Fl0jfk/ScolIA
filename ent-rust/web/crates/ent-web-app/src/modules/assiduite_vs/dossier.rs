//! Dossier d'assiduité d'un élève (socle 12, AS14d) : tout ce que la vie scolaire et la direction ont
//! besoin de lire et de faire pour **un** élève, sur l'année scolaire (R131-6) : épisodes et leur
//! qualification (individuelle, requalification tracée, justificatifs), décompte par mois avec
//! l'explication de chaque demi-journée, rectifications postérieures à l'arrêt d'un mois, contacts et
//! courriers, signalements, chronologie.
//!
//! Aucune règle n'est recalculée ici : les routes sont celles d'AS3 (épisodes, qualification,
//! requalification), AS4 (demi-journées expliquées, rectifications), AS5 (contacts, courriers, avis),
//! et le décompte et les signalements du mois. Le serveur reste juge : une section refusée (403) est
//! dite, jamais masquée par un silence ; un motif médical ou un justificatif hors du cercle autorisé
//! n'est jamais rendu par le serveur.

use chrono::Datelike;
use ent_types::assiduite::{ContactDto, CourrierDto, EpisodeDto, NatureMotifDto, RectificationDto};
use ent_types::discipline::{DemiJourneeDto, SignalementDto};
use ent_types::vie_scolaire::EleveFicheDto;
use ent_web_api::{ApiError, FiltreEpisodesApi};
use ent_web_ui::{
    use_notifications, Alerte, Badge, Bouton, EtatVide, LienBouton, Niveau, OptionSegment,
    Segmente, Selection, Squelette, Statistique, Taille, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_params_map};
use uuid::Uuid;

use super::dossier_modele::{
    bornes_annee_scolaire, chronologie, compter_episodes, corps_requalification, etat_demi_journee,
    explication_demi_journee, franchit_le_seuil, grouper_par_mois, libelle_annee_scolaire,
    libelle_canal, libelle_demi_journee, libelle_issue_contact, libelle_mois,
    libelle_nature_courrier, libelle_statut_courrier, libelle_statut_signalement,
    phrase_rectification, resume_signalement, ton_statut_signalement, total_annee,
    trier_episodes_dossier, MoisDecompte, SaisieRequalification, MOTIFS_REQUALIFICATION,
};
use super::modele::{
    libelle_qualification, libelles_famille_pour, natures_legitimes, QUALIFICATIONS,
};
use super::{AvisEleve, DetailEpisode};
use crate::composants_metier::absences::libelle_periode;
use crate::composants_metier::recherche_eleve::RechercheEleve;
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::message_echec;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

/// Seuil de repli quand la règle n'a pas pu être lue : l'écran le dit et n'en tire aucun « atteint ».
const SEUIL_INCONNU: i32 = 0;

fn id_param() -> Memo<Option<Uuid>> {
    let params = use_params_map();
    Memo::new(move |_| {
        params
            .with(|p| p.get("id"))
            .and_then(|i| i.parse::<Uuid>().ok())
    })
}

#[component]
pub fn PageDossierAssiduite() -> impl IntoView {
    let id = id_param();
    view! {
        <Title text="Dossier d'assiduité"/>
        <nav class="fil-ariane" aria-label="Fil d'Ariane">
            <a href=navigation::ASSIDUITE>"Assiduité"</a>
            <span aria-hidden="true">" › "</span>
            <span aria-current="page">"Dossier d'un élève"</span>
        </nav>
        {move || match id.get() {
            None => view! { <EtatVide titre="Élève introuvable" message="Identifiant d'élève invalide."/> }.into_any(),
            Some(id) => view! { <DossierEleve eleve_id=id/> }.into_any(),
        }}
    }
}

/// Choix d'un élève pour ouvrir son dossier (recherche par nom, 2 caractères au moins).
#[component]
pub fn ChoixDossier() -> impl IntoView {
    let navigate = use_navigate();
    view! {
        <section aria-labelledby="choix-dossier">
            <h2 id="choix-dossier" class="vs-section-titre">"Dossier d'assiduité d'un élève"</h2>
            <p class="texte-attenue">
                "Absences, décompte par mois, contacts, rectifications et signalements de l'année. Chaque consultation est enregistrée."
            </p>
            <RechercheEleve
                id="dossier-recherche"
                libelle="Rechercher un élève"
                action="Ouvrir le dossier de"
                on_choisir=Callback::new(move |(id, _): (Uuid, String)| {
                    navigate(&format!("{}/eleves/{id}", navigation::ASSIDUITE), Default::default());
                })
            />
        </section>
    }
}

// ───────────────────────────── Chargements ─────────────────────────────

/// Décompte de l'année : demi-journées par mois, rectifications, signalements.
#[derive(Clone, Debug)]
struct DonneesDecompte {
    /// Seuil de la règle en vigueur ; `None` si elle n'a pas pu être lue.
    seuil: Option<i32>,
    mois: Vec<MoisDecompte>,
    rectifications: Vec<RectificationDto>,
    signalements: Vec<SignalementDto>,
    /// Ce qui n'a pas pu être lu (refus ou indisponibilité), dit à l'écran.
    manques: Vec<&'static str>,
}

async fn charger_decompte(eleve_id: Uuid) -> Result<DonneesDecompte, ApiError> {
    let aujourd = aujourdhui();
    let (du, au) = bornes_annee_scolaire(aujourd);
    let client = session::client();
    let demi: Vec<DemiJourneeDto> = client.demi_journees_eleve(eleve_id, du, au).await?;
    let mut manques: Vec<&'static str> = Vec::new();
    let seuil = match client.regle_assiduite().await {
        Ok(r) => Some(r.seuil_signalement),
        Err(_) => {
            manques.push("Le seuil de signalement de l'établissement n'a pas pu être lu.");
            None
        }
    };
    let mois = grouper_par_mois(&demi, seuil.unwrap_or(SEUIL_INCONNU));
    let mut rectifications = Vec::new();
    let mut signalements = Vec::new();
    let mut rectifs_refusees = false;
    let mut signalements_refuses = false;
    for m in &mois {
        // Rectifications : seulement pour un mois terminé (un mois en cours n'est pas arrêté).
        if m.mois
            < crate::noyau::temps::aujourdhui()
                .with_day(1)
                .unwrap_or(aujourd)
        {
            match client.rectifications_mois(m.mois).await {
                Ok(l) => rectifications.extend(l.into_iter().filter(|r| r.eleve_id == eleve_id)),
                Err(_) => rectifs_refusees = true,
            }
        }
        // Signalements : seulement pour un mois où l'élève a atteint le seuil.
        if matches!(m.etat, super::dossier_modele::EtatSeuil::Atteint) {
            match client.signalements(m.mois).await {
                Ok(l) => signalements.extend(l.into_iter().filter(|s| s.eleve_id == eleve_id)),
                Err(_) => signalements_refuses = true,
            }
        }
    }
    if rectifs_refusees {
        manques
            .push("Les rectifications des mois arrêtés ne sont pas accessibles avec vos droits.");
    }
    if signalements_refuses {
        manques.push("Les signalements ne sont pas accessibles avec vos droits.");
    }
    Ok(DonneesDecompte {
        seuil,
        mois,
        rectifications,
        signalements,
        manques,
    })
}

/// Contacts et courriers : `None` = section non accessible avec ces droits (`assiduite.suivre`).
#[derive(Clone, Debug)]
struct DonneesSuivi {
    contacts: Option<Vec<ContactDto>>,
    courriers: Option<Vec<CourrierDto>>,
}

async fn charger_suivi(eleve_id: Uuid) -> Result<DonneesSuivi, ApiError> {
    let client = session::client();
    let contacts = match client.contacts_eleve(eleve_id).await {
        Ok(l) => Some(l),
        Err(ApiError::Interdit(_) | ApiError::Introuvable(_)) => None,
        Err(e) => return Err(e),
    };
    let courriers = match client.courriers_eleve(eleve_id).await {
        Ok(l) => Some(l),
        Err(ApiError::Interdit(_) | ApiError::Introuvable(_)) => None,
        Err(e) => return Err(e),
    };
    Ok(DonneesSuivi {
        contacts,
        courriers,
    })
}

// ───────────────────────────── Page ─────────────────────────────

#[component]
fn DossierEleve(eleve_id: Uuid) -> impl IntoView {
    let vue = RwSignal::new("resume".to_string());
    let fiche =
        LocalResource::new(move || async move { session::client().fiche_eleve(eleve_id).await });
    let natures = LocalResource::new(|| async { session::client().natures_motif().await });
    let natures_liste =
        Signal::derive(move || natures.get().and_then(Result::ok).unwrap_or_default());
    let episodes = LocalResource::new(move || async move {
        let (du, _) = bornes_annee_scolaire(aujourdhui());
        session::client()
            .episodes(&FiltreEpisodesApi {
                eleve_id: Some(eleve_id),
                depuis: Some(du),
                etat: Some("tous"),
                limite: Some(200),
                ..Default::default()
            })
            .await
            .map(trier_episodes_dossier)
    });
    let decompte = LocalResource::new(move || async move { charger_decompte(eleve_id).await });
    let suivi = LocalResource::new(move || async move { charger_suivi(eleve_id).await });
    let recharger = Callback::new(move |_| {
        episodes.refetch();
        decompte.refetch();
        suivi.refetch();
    });
    let options = vec![
        OptionSegment::new("resume", "Résumé"),
        OptionSegment::new("absences", "Absences"),
        OptionSegment::new("decompte", "Décompte"),
        OptionSegment::new("suivi", "Contacts et courriers"),
        OptionSegment::new("chronologie", "Chronologie"),
    ];
    view! {
        {move || match fiche.get() {
            None => view! { <Squelette lignes=2 libelle="Chargement de l'élève…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| fiche.refetch())/> }.into_any(),
            Some(Ok(f)) => view! { <EnTeteEleve fiche=f/> }.into_any(),
        }}
        <div class="asd-onglets">
            <Segmente nom="dossier-vue" legende="Section du dossier" options=options valeur=vue/>
        </div>
        {move || match vue.get().as_str() {
            "absences" => view! { <OngletAbsences episodes=episodes natures=natures_liste recharger=recharger/> }.into_any(),
            "decompte" => view! { <OngletDecompte decompte=decompte/> }.into_any(),
            "suivi" => view! { <OngletSuivi suivi=suivi eleve_id=eleve_id/> }.into_any(),
            "chronologie" => view! { <OngletChronologie episodes=episodes decompte=decompte suivi=suivi/> }.into_any(),
            _ => view! { <OngletResume episodes=episodes decompte=decompte vue=vue/> }.into_any(),
        }}
    }
}

#[component]
fn EnTeteEleve(fiche: EleveFicheDto) -> impl IntoView {
    let nom = format!("{} {}", fiche.prenom, fiche.nom).trim().to_string();
    let classe = fiche.classe.clone().unwrap_or_default();
    let annee = RwSignal::new(String::new());
    // L'année est posée après l'hydratation : aucune date dans le HTML rendu côté serveur.
    Effect::new(move |_| {
        if annee.get_untracked().is_empty() {
            annee.set(libelle_annee_scolaire(aujourdhui()));
        }
    });
    view! {
        <header class="asd-dossier__tete">
            <h1 class="ui-titre-page">{format!("Dossier d'assiduité : {nom}")}</h1>
            <p class="texte-attenue">
                {if classe.is_empty() { String::new() } else { format!("Classe {classe} · ") }}
                {move || format!("année scolaire {}", annee.get())}
                " · dossier de l'année seulement (R131-6). Chaque consultation est enregistrée."
            </p>
        </header>
    }
}

// ───────────────────────────── Résumé ─────────────────────────────

#[component]
fn OngletResume(
    episodes: LocalResource<Result<Vec<EpisodeDto>, ApiError>>,
    decompte: LocalResource<Result<DonneesDecompte, ApiError>>,
    vue: RwSignal<String>,
) -> impl IntoView {
    view! {
        <section class="asd-resume" aria-labelledby="dossier-resume">
            <h2 id="dossier-resume" class="vs-section-titre">"Où en est cet élève"</h2>
            {move || match decompte.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement du décompte…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| decompte.refetch())/> }.into_any(),
                Some(Ok(d)) => {
                    let (comptees, non_just) = total_annee(&d.mois);
                    let courant = d.mois.first().cloned();
                    let seuil = d.seuil;
                    let ton_mois = courant.as_ref().map(|m| m.etat.ton()).unwrap_or(TonEtat::Neutre);
                    let libelle_mois_courant = courant
                        .as_ref()
                        .map(|m| format!("Dernier mois avec absences ({})", libelle_mois(m.mois)))
                        .unwrap_or_else(|| "Dernier mois avec absences".to_string());
                    let touches: Vec<RectificationDto> = d.rectifications.iter().filter(|r| r.touche_signalement).cloned().collect();
                    let ton_annee = if non_just > 0 { TonEtat::Avertissement } else { TonEtat::Ok };
                    let non_just_mois = courant.as_ref().map(|m| m.non_justifiees).unwrap_or(0);
                    let etat_mois = courant.as_ref().map(|m| m.etat);
                    view! {
                        <div class="jvs-stats">
                            <Statistique libelle="Demi-journées comptées (année)" valeur=Signal::derive(move || comptees.to_string())/>
                            <Statistique libelle="Dont non justifiées (année)" valeur=Signal::derive(move || non_just.to_string()) ton=ton_annee/>
                            <Statistique libelle=libelle_mois_courant valeur=Signal::derive(move || format!("{non_just_mois} non justifiée(s)")) ton=ton_mois/>
                        </div>
                        <p>
                            {etat_mois.map(|e| view! { <Badge libelle=e.libelle() ton=e.ton()/> })}
                            {" "}
                            <span class="texte-attenue">
                                {match seuil {
                                    Some(s) => format!("Seuil de l'établissement : {s} demi-journée(s) non justifiée(s) dans le mois."),
                                    None => "Seuil de l'établissement non lu : aucun « seuil atteint » n'est affiché.".to_string(),
                                }}
                            </span>
                        </p>
                        {(!d.manques.is_empty()).then(|| view! {
                            <Alerte niveau=Niveau::Info titre="Certaines données ne sont pas affichées">
                                <ul>{d.manques.iter().map(|m| view! { <li>{*m}</li> }).collect_view()}</ul>
                            </Alerte>
                        })}
                        {touches.into_iter().map(|r| view! {
                            <Alerte niveau=Niveau::Avertissement titre="Rectification qui touche un signalement">
                                <p>{phrase_rectification(&r)}</p>
                            </Alerte>
                        }).collect_view()}
                    }
                    .into_any()
                }
            }}
            {move || match episodes.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des absences…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| episodes.refetch())/> }.into_any(),
                Some(Ok(l)) => {
                    let c = compter_episodes(&l);
                    let ton_attente = if c.a_qualifier > 0 { TonEtat::Avertissement } else { TonEtat::Ok };
                    view! {
                        <div class="jvs-stats">
                            <Statistique libelle="Épisodes d'absence (année)" valeur=Signal::derive(move || c.total.to_string())/>
                            <Statistique libelle="À qualifier" valeur=Signal::derive(move || c.a_qualifier.to_string()) ton=ton_attente/>
                            <Statistique libelle="Qualifiés non justifiés" valeur=Signal::derive(move || c.non_justifies.to_string())/>
                        </div>
                        {(c.a_qualifier > 0).then(|| view! {
                            <p>
                                <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| vue.set("absences".to_string()))>
                                    {format!("Qualifier les {} absence(s) en attente", c.a_qualifier)}
                                </Bouton>
                            </p>
                        })}
                    }
                    .into_any()
                }
            }}
        </section>
    }
}

// ───────────────────────────── Absences (épisodes) ─────────────────────────────

#[component]
fn OngletAbsences(
    episodes: LocalResource<Result<Vec<EpisodeDto>, ApiError>>,
    natures: Signal<Vec<NatureMotifDto>>,
    recharger: Callback<()>,
) -> impl IntoView {
    let ouvert = RwSignal::new(None::<Uuid>);
    view! {
        <section aria-labelledby="dossier-absences">
            <h2 id="dossier-absences" class="vs-section-titre">"Épisodes d'absence de l'année"</h2>
            {move || match episodes.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement des absences…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| episodes.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucune absence cette année" message="Aucun épisode d'absence n'est enregistré pour cet élève sur l'année scolaire."/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <ListeEpisodes episodes=l ouvert=ouvert/> }.into_any(),
            }}
            {move || {
                ouvert.get().map(|id| view! {
                    <DetailEpisode
                        id=id
                        nom="Épisode".to_string()
                        natures=natures
                        on_change=recharger
                        on_fermer=Callback::new(move |_| ouvert.set(None))
                    />
                })
            }}
        </section>
    }
}

#[component]
fn ListeEpisodes(episodes: Vec<EpisodeDto>, ouvert: RwSignal<Option<Uuid>>) -> impl IntoView {
    view! {
        <div class="ui-tableau-defile">
            <table class="ui-tableau asd-tableau">
                <caption class="ui-sr">"Épisodes d'absence de l'année scolaire"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Période"</th>
                        <th scope="col">"Séances"</th>
                        <th scope="col">"État"</th>
                        <th scope="col"><span class="ui-sr">"Action"</span></th>
                    </tr>
                </thead>
                <tbody>
                    {episodes
                        .into_iter()
                        .map(|e| {
                            let id = e.id;
                            let periode = libelle_periode(e.jour_debut, e.jour_fin);
                            let actives = e.seances.iter().filter(|s| !s.retiree).count();
                            let ton = match (e.etat.as_str(), e.qualification.as_str()) {
                                ("retire", _) => TonEtat::Neutre,
                                ("qualifie", "reputee_legitime" | "excuse_valable") => TonEtat::Ok,
                                ("qualifie", _) => TonEtat::Mal,
                                _ => TonEtat::Avertissement,
                            };
                            let etat = if e.etat == "qualifie" {
                                libelle_qualification(&e.qualification).to_string()
                            } else {
                                super::modele::libelle_etat_episode(&e.etat).to_string()
                            };
                            let accessible = format!("Ouvrir l'absence : {periode}");
                            view! {
                                <tr class:asd-ligne--ouverte=move || ouvert.get() == Some(id)>
                                    <th scope="row">{periode}</th>
                                    <td>{format!("{actives} séance(s)")}</td>
                                    <td>
                                        <Badge libelle=etat ton=ton/>
                                        {e.prevenue.then(|| view! { <Badge libelle="Prévenue" ton=TonEtat::Info/> })}
                                        {(e.justificatifs > 0).then(|| view! { <Badge libelle=format!("{} justificatif(s)", e.justificatifs) ton=TonEtat::Avertissement/> })}
                                    </td>
                                    <td>
                                        <Bouton
                                            variante=Variante::Secondaire
                                            taille=Taille::Compacte
                                            libelle_accessible=accessible
                                            on_click=Callback::new(move |_| ouvert.set(Some(id)))
                                        >
                                            "Ouvrir"
                                        </Bouton>
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

/// Requalifier un épisode déjà qualifié : motif codé obligatoire, conservé dans l'historique ;
/// un mois déjà arrêté n'est pas réécrit, la différence est inscrite comme rectification (R-AS16).
#[component]
pub(super) fn FormulaireRequalification(
    episode: EpisodeDto,
    natures: Signal<Vec<NatureMotifDto>>,
    apres: Callback<()>,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let notifications = use_notifications();
    let qualification = RwSignal::new(String::new());
    let motif = RwSignal::new(String::new());
    let nature = RwSignal::new(String::new());
    let libelle = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let (id, version, actuelle) = (episode.id, episode.version, episode.qualification.clone());
    Effect::new(move |_| {
        qualification.track();
        libelle.set(String::new());
    });
    let envoyer = Callback::new(move |_: ()| {
        let saisie = SaisieRequalification {
            qualification: qualification.get_untracked(),
            motif_code: motif.get_untracked(),
            nature_code: nature.get_untracked(),
            libelle_famille: libelle.get_untracked(),
            note: note.get_untracked(),
        };
        match corps_requalification(&actuelle, &saisie, version) {
            Err(m) => erreur.set(Some(m.to_string())),
            Ok(corps) => {
                erreur.set(None);
                en_cours.set(true);
                spawn_local(async move {
                    match session::client().requalifier_episode(id, &corps).await {
                        Ok(_) => {
                            notifications.succes(
                                "Absence requalifiée : le changement est conservé dans l'historique.",
                            );
                            apres.run(());
                        }
                        Err(ApiError::Conflit(_)) => erreur.set(Some(
                            "Cette absence a été modifiée entre-temps : elle est rechargée, relisez avant de requalifier.".into(),
                        )),
                        Err(e) => erreur.set(Some(message_echec(e))),
                    }
                    en_cours.set(false);
                });
            }
        }
    });
    let options_qualif = {
        let mut v = vec![(String::new(), "Choisir…".to_string())];
        v.extend(
            QUALIFICATIONS
                .iter()
                .filter(|(c, _)| *c != episode.qualification)
                .map(|(c, l)| (c.to_string(), l.to_string())),
        );
        v
    };
    let options_motif = {
        let mut v = vec![(String::new(), "Choisir…".to_string())];
        v.extend(
            MOTIFS_REQUALIFICATION
                .iter()
                .map(|(c, l)| (c.to_string(), l.to_string())),
        );
        v
    };
    view! {
        {move || {
            moteur.get().a("absences.qualifier").then(|| {
                let options_qualif = options_qualif.clone();
                let options_motif = options_motif.clone();
                view! {
                    <h3>"Requalifier"</h3>
                    <p class="texte-attenue">
                        "Le changement est tracé dans l'historique. Si le mois est déjà arrêté, il n'est pas réécrit : la différence apparaît comme rectification."
                    </p>
                    <form class="asd-form" on:submit=move |ev| { ev.prevent_default(); envoyer.run(()); } novalidate>
                        <Selection id="req-qualif" libelle="Nouvelle qualification" valeur=qualification options=options_qualif requis=true/>
                        <Show when=move || qualification.get() == "reputee_legitime">
                            {move || {
                                let mut o = vec![(String::new(), "Choisir…".to_string())];
                                o.extend(natures_legitimes(&natures.get()));
                                view! { <Selection id="req-nature" libelle="Motif légitime" valeur=nature options=o requis=true/> }
                            }}
                        </Show>
                        <Show when=move || !qualification.get().is_empty()>
                            {move || {
                                let mut o = vec![(String::new(), "Libellé par défaut".to_string())];
                                o.extend(libelles_famille_pour(&qualification.get()).into_iter().map(|(c, l)| (c.to_string(), l.to_string())));
                                view! { <Selection id="req-libelle" libelle="Ce que voit la famille" valeur=libelle options=o/> }
                            }}
                        </Show>
                        <Selection id="req-motif" libelle="Pourquoi requalifier" valeur=motif options=options_motif requis=true/>
                        <ZoneTexte id="req-note" libelle="Note interne (jamais visible de la famille ni du professeur)" valeur=note/>
                        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                        <Bouton soumettre=true en_cours=en_cours>"Requalifier l'absence"</Bouton>
                    </form>
                }
            })
        }}
    }
}

// ───────────────────────────── Décompte ─────────────────────────────

#[component]
fn OngletDecompte(decompte: LocalResource<Result<DonneesDecompte, ApiError>>) -> impl IntoView {
    view! {
        <section aria-labelledby="dossier-decompte">
            <h2 id="dossier-decompte" class="vs-section-titre">"Décompte par demi-journée et par mois"</h2>
            <p class="texte-attenue">
                "Le décompte est calculé par le serveur selon la règle de l'établissement en vigueur le jour concerné ; chaque ligne en donne l'explication. Un mois arrêté n'est jamais réécrit : les corrections postérieures sont listées à part."
            </p>
            {move || match decompte.get() {
                None => view! { <Squelette lignes=5 libelle="Chargement du décompte…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| decompte.refetch())/> }.into_any(),
                Some(Ok(d)) if d.mois.is_empty() => view! {
                    <EtatVide titre="Aucune demi-journée d'absence" message="Aucune demi-journée n'est comptée pour cet élève cette année."/>
                }
                .into_any(),
                Some(Ok(d)) => {
                    let rectifs = d.rectifications.clone();
                    let seuil = d.seuil;
                    view! {
                        {(!d.manques.is_empty()).then(|| view! {
                            <Alerte niveau=Niveau::Info titre="Certaines données ne sont pas affichées">
                                <ul>{d.manques.iter().map(|m| view! { <li>{*m}</li> }).collect_view()}</ul>
                            </Alerte>
                        })}
                        {d.mois.into_iter().map(|m| {
                            let rectifs_du_mois: Vec<RectificationDto> = rectifs.iter().filter(|r| r.mois == m.mois).cloned().collect();
                            view! { <MoisDuDossier mois=m rectifications=rectifs_du_mois seuil=seuil/> }
                        }).collect_view()}
                    }
                    .into_any()
                }
            }}
        </section>
    }
}

#[component]
fn MoisDuDossier(
    mois: MoisDecompte,
    rectifications: Vec<RectificationDto>,
    seuil: Option<i32>,
) -> impl IntoView {
    let titre_id = format!("mois-{}", mois.mois.format("%Y-%m"));
    let titre = libelle_mois(mois.mois);
    let resume = format!(
        "{} demi-journée(s) comptée(s), dont {} non justifiée(s)",
        mois.comptees, mois.non_justifiees
    );
    let etat = mois.etat;
    let lignes = mois.lignes.clone();
    let seuil_connu = seuil.unwrap_or(SEUIL_INCONNU);
    view! {
        <section class="asd-mois" aria-labelledby=titre_id.clone()>
            <h3 id=titre_id.clone() class="asd-mois__titre">
                {titre.clone()}
                " "
                {(seuil_connu > 0).then(|| view! { <Badge libelle=etat.libelle() ton=etat.ton()/> })}
            </h3>
            <p>{resume}</p>
            <div class="ui-tableau-defile">
                <table class="ui-tableau asd-tableau">
                    <caption class="ui-sr">{format!("Demi-journées de {titre}")}</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Demi-journée"</th>
                            <th scope="col">"État"</th>
                            <th scope="col">"Explication"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {lignes.iter().map(|l| {
                            let (libelle, ton) = etat_demi_journee(l);
                            view! {
                                <tr>
                                    <th scope="row">{libelle_demi_journee(l)}</th>
                                    <td><Badge libelle=libelle ton=ton/></td>
                                    <td>{explication_demi_journee(l)}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
            {(!rectifications.is_empty()).then(|| view! {
                <h4>"Rectifications postérieures à l'arrêt du mois"</h4>
                <ul class="asd-rectifs">
                    {rectifications.iter().map(|r| {
                        let franchit = seuil.map(|s| franchit_le_seuil(r, s)).unwrap_or(false);
                        view! {
                            <li>
                                {phrase_rectification(r)}
                                {r.touche_signalement.then(|| view! { <Badge libelle="Touche un signalement" ton=TonEtat::Avertissement/> })}
                                {franchit.then(|| view! { <Badge libelle="Change la position par rapport au seuil" ton=TonEtat::Mal/> })}
                            </li>
                        }
                    }).collect_view()}
                </ul>
            })}
        </section>
    }
}

// ───────────────────────────── Contacts et courriers ─────────────────────────────

#[component]
fn OngletSuivi(
    suivi: LocalResource<Result<DonneesSuivi, ApiError>>,
    eleve_id: Uuid,
) -> impl IntoView {
    view! {
        <section aria-labelledby="dossier-suivi">
            <h2 id="dossier-suivi" class="vs-section-titre">"Contacts avec la famille et courriers"</h2>
            {move || match suivi.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement du suivi…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| suivi.refetch())/> }.into_any(),
                Some(Ok(s)) => view! {
                    {match s.contacts {
                        None => view! {
                            <Alerte niveau=Niveau::Info titre="Contacts non accessibles">
                                <p>"Le suivi des contacts avec la famille n'est pas ouvert à votre rôle."</p>
                            </Alerte>
                        }.into_any(),
                        Some(l) if l.is_empty() => view! { <EtatVide titre="Aucun contact tracé" message="Aucun appel, message ou rencontre n'est tracé pour cet élève."/> }.into_any(),
                        Some(l) => view! {
                            <h3>"Contacts"</h3>
                            <ul class="asd-liste">
                                {l.into_iter().map(|c| view! {
                                    <li>
                                        <strong>{format!("{} : {}", libelle_canal(&c.canal), libelle_issue_contact(&c.issue))}</strong>
                                        {format!(" · {}", crate::noyau::temps::date_heure_paris(c.cree_le))}
                                        {c.note_interne.clone().filter(|n| !n.trim().is_empty()).map(|n| view! { <p class="texte-attenue">{format!("Note interne : {n}")}</p> })}
                                    </li>
                                }).collect_view()}
                            </ul>
                        }.into_any(),
                    }}
                    {match s.courriers {
                        None => ().into_any(),
                        Some(l) if l.is_empty() => view! { <p class="texte-attenue">"Aucun courrier préparé pour cet élève."</p> }.into_any(),
                        Some(l) => view! {
                            <h3>"Courriers"</h3>
                            <ul class="asd-liste">
                                {l.into_iter().map(|c| view! {
                                    <li>{format!("{} : {} · préparé le {}", libelle_nature_courrier(&c.nature), libelle_statut_courrier(&c.statut), crate::noyau::temps::date_heure_paris(c.prepare_le))}</li>
                                }).collect_view()}
                            </ul>
                        }.into_any(),
                    }}
                }.into_any(),
            }}
            <AvisEleve eleve_id=eleve_id titre="Avis aux familles" sous_section=true/>
        </section>
    }
}

// ───────────────────────────── Chronologie ─────────────────────────────

#[component]
fn OngletChronologie(
    episodes: LocalResource<Result<Vec<EpisodeDto>, ApiError>>,
    decompte: LocalResource<Result<DonneesDecompte, ApiError>>,
    suivi: LocalResource<Result<DonneesSuivi, ApiError>>,
) -> impl IntoView {
    view! {
        <section aria-labelledby="dossier-chrono">
            <h2 id="dossier-chrono" class="vs-section-titre">"Chronologie du dossier"</h2>
            <p class="texte-attenue">
                "Absences, contacts, courriers, rectifications et signalements de l'année, du plus récent au plus ancien."
            </p>
            {move || match (episodes.get(), decompte.get(), suivi.get()) {
                (Some(Ok(e)), Some(Ok(d)), Some(Ok(s))) => {
                    let evts = chronologie(
                        &e,
                        s.contacts.as_deref().unwrap_or_default(),
                        s.courriers.as_deref().unwrap_or_default(),
                        &d.rectifications,
                        &d.signalements,
                    );
                    if evts.is_empty() {
                        view! { <EtatVide titre="Dossier vide" message="Rien n'est encore enregistré pour cet élève cette année."/> }.into_any()
                    } else {
                        view! {
                            <ol class="asd-chrono">
                                {evts.into_iter().map(|x| view! {
                                    <li>
                                        <Badge libelle=x.genre.libelle()/>
                                        " "
                                        <strong>{x.titre.clone()}</strong>
                                        {format!(" · {}", x.quand)}
                                        {x.detail.clone().map(|t| view! { <p class="texte-attenue">{t}</p> })}
                                    </li>
                                }).collect_view()}
                            </ol>
                        }.into_any()
                    }
                }
                (Some(Err(e)), _, _) | (_, Some(Err(e)), _) | (_, _, Some(Err(e))) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| { episodes.refetch(); decompte.refetch(); suivi.refetch(); })/>
                }.into_any(),
                _ => view! { <Squelette lignes=5 libelle="Chargement de la chronologie…"/> }.into_any(),
            }}
            {move || {
                decompte.get().and_then(Result::ok).map(|d| {
                    (!d.signalements.is_empty()).then(|| view! {
                        <h3>"Signalements de l'année"</h3>
                        <ul class="asd-liste">
                            {d.signalements.iter().map(|s| view! {
                                <li>
                                    <Badge libelle=libelle_statut_signalement(&s.statut) ton=ton_statut_signalement(&s.statut)/>
                                    {format!(" {} : {}", libelle_mois(s.mois), resume_signalement(s))}
                                </li>
                            }).collect_view()}
                        </ul>
                        <p>
                            <LienBouton href=navigation::SIGNALEMENTS variante=Variante::Secondaire>"Ouvrir les signalements"</LienBouton>
                        </p>
                    })
                })
            }}
        </section>
    }
}
