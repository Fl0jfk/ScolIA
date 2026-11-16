//! Assiduité de la vie scolaire (socle 12, AS14b) : file « à qualifier », justificatifs à examiner,
//! file « sans réponse » (relance graduée) et avis aux familles (prévus, partis, démentis).
//! L'accueil (AS-216) est dans `accueil.rs`.
//!
//! Routes d'`ent-api` : `GET /assiduite/episodes`, `/assiduite/episodes/{id}`, `POST …/qualification`,
//! `POST /assiduite/episodes/masse[/apercu]`, `…/justificatifs`, `/assiduite/relances/sans-reponse`,
//! `/assiduite/eleves/{id}/avis`. Le serveur décide des droits (`absences.lire_motif`,
//! `absences.qualifier`, `assiduite.suivre`) ; l'écran traite les refus (403) et ne propose pas ce
//! qui serait refusé. Une qualification n'est jamais posée d'office : elle est confirmée par une
//! personne, avec la version lue (conflit si l'épisode a changé).

pub mod accueil;
pub mod modele;

use chrono::NaiveDate;
use ent_types::assiduite::{
    ApercuMasseDto, AvisDemiJourneeDto, EpisodeDto, EpisodeSansReponseDto, JustificatifDto,
    NatureMotifDto,
};
use ent_web_api::{ApiError, FiltreEpisodesApi};
use ent_web_ui::{
    use_notifications, Alerte, Annonce, Badge, Bouton, EnTetePage, EtatVide, Niveau, OptionSegment,
    Segmente, Selection, Squelette, Taille, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::absences::libelle_periode;
use crate::composants_metier::recherche_eleve::RechercheEleve;
use crate::noyau::ecriture::message_echec;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;
use modele::{
    corps_masse, corps_qualification, etat_avis, libelle_cause, libelle_depose_par, libelle_ecart,
    libelle_etape, libelle_etat_episode, libelle_famille, libelle_periode_avis,
    libelle_presentation_justificatif, libelle_qualification, libelle_source,
    libelles_famille_pour, natures_legitimes, nb_seances_actives, nom_eleve, trier_episodes,
    trier_sans_reponse, EtatAvis, SaisieQualification, MAX_MASSE, QUALIFICATIONS,
};

/// Fenêtre de la file : trois mois, pour ne pas réveiller l'année passée.
const JOURS_FILE: i64 = 92;

#[component]
pub fn PageAssiduite() -> impl IntoView {
    let vue = RwSignal::new("a_qualifier".to_string());
    let natures = LocalResource::new(|| async { session::client().natures_motif().await });
    let natures_liste =
        Signal::derive(move || natures.get().and_then(Result::ok).unwrap_or_default());
    let options = vec![
        OptionSegment::new("a_qualifier", "À qualifier"),
        OptionSegment::new("justificatifs", "Justificatifs"),
        OptionSegment::new("sans_reponse", "Sans réponse"),
        OptionSegment::new("avis", "Avis aux familles"),
    ];
    view! {
        <Title text="Assiduité"/>
        <EnTetePage
            titre="Assiduité".to_string()
            sous_titre="Absences à qualifier, justificatifs, relances et avis aux familles.".to_string()
        />
        <div class="asd-onglets">
            <Segmente nom="asd-vue" legende="Vue" options=options valeur=vue/>
        </div>
        {move || match vue.get().as_str() {
            "justificatifs" => view! { <FileEpisodes avec_justificatif=true natures=natures_liste/> }.into_any(),
            "sans_reponse" => view! { <SansReponse/> }.into_any(),
            "avis" => view! { <AvisFamilles/> }.into_any(),
            _ => view! { <FileEpisodes avec_justificatif=false natures=natures_liste/> }.into_any(),
        }}
    }
}

// ---------------------------------------------------------------------------
// File des épisodes à qualifier
// ---------------------------------------------------------------------------

#[component]
fn FileEpisodes(avec_justificatif: bool, natures: Signal<Vec<NatureMotifDto>>) -> impl IntoView {
    let ouvert = RwSignal::new(None::<Uuid>);
    let selection = RwSignal::new(Vec::<Uuid>::new());
    let liste = LocalResource::new(move || async move {
        let aujourdhui = crate::noyau::temps::aujourdhui();
        session::client()
            .episodes(&FiltreEpisodesApi {
                etat: Some("a_qualifier"),
                depuis: Some(aujourdhui - chrono::Duration::days(JOURS_FILE)),
                avec_justificatif: avec_justificatif.then_some(true),
                limite: Some(200),
                ..Default::default()
            })
            .await
            .map(trier_episodes)
    });
    let recharger = Callback::new(move |_| {
        selection.set(vec![]);
        liste.refetch();
    });
    view! {
        {move || match liste.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement des absences à qualifier…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide
                    titre=if avec_justificatif { "Aucun justificatif à examiner" } else { "Rien à qualifier" }
                    message="Les nouvelles absences apparaissent ici dès que l'appel est fait."
                />
            }
                .into_any(),
            Some(Ok(l)) => view! {
                <TableEpisodes episodes=l ouvert=ouvert selection=selection natures=natures recharger=recharger/>
            }
                .into_any(),
        }}
    }
}

#[component]
fn TableEpisodes(
    episodes: Vec<EpisodeDto>,
    ouvert: RwSignal<Option<Uuid>>,
    selection: RwSignal<Vec<Uuid>>,
    natures: Signal<Vec<NatureMotifDto>>,
    recharger: Callback<()>,
) -> impl IntoView {
    let total = episodes.len();
    let ids: Vec<Uuid> = episodes.iter().map(|e| e.id).collect();
    let tous = StoredValue::new(ids);
    let noms = StoredValue::new(
        episodes
            .iter()
            .map(|e| (e.id, nom_eleve(e)))
            .collect::<Vec<(Uuid, String)>>(),
    );
    view! {
        <p class="asd-compte" role="status">
            {format!("{total} absence{} à qualifier", if total > 1 { "s" } else { "" })}
        </p>
        <BarreMasse selection=selection recharger=recharger/>
        <div class="ui-tableau-defile">
            <table class="ui-tableau asd-tableau">
                <caption class="ui-sr">"Absences à qualifier"</caption>
                <thead>
                    <tr>
                        <th scope="col" class="asd-case">
                            <input
                                type="checkbox"
                                aria-label="Sélectionner toutes les absences affichées"
                                prop:checked=move || {
                                    let n = selection.with(Vec::len);
                                    n > 0 && n == tous.with_value(Vec::len)
                                }
                                on:change=move |ev| {
                                    if event_target_checked(&ev) {
                                        selection.set(tous.get_value().into_iter().take(MAX_MASSE).collect());
                                    } else {
                                        selection.set(vec![]);
                                    }
                                }
                            />
                        </th>
                        <th scope="col">"Élève"</th>
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
                            let nom = nom_eleve(&e);
                            let periode = libelle_periode(e.jour_debut, e.jour_fin);
                            let nb = nb_seances_actives(&e);
                            let libelle_ouvrir = format!("Ouvrir l'absence de {nom} : {periode}");
                            let libelle_case = format!("Sélectionner l'absence de {nom} : {periode}");
                            view! {
                                <tr class:asd-ligne--ouverte=move || ouvert.get() == Some(id)>
                                    <td class="asd-case">
                                        <input
                                            type="checkbox"
                                            aria-label=libelle_case
                                            prop:checked=move || selection.with(|s| s.contains(&id))
                                            on:change=move |ev| {
                                                let coche = event_target_checked(&ev);
                                                selection.update(|s| {
                                                    s.retain(|x| *x != id);
                                                    if coche && s.len() < MAX_MASSE {
                                                        s.push(id);
                                                    }
                                                });
                                            }
                                        />
                                    </td>
                                    <th scope="row">{nom}</th>
                                    <td>{periode}</td>
                                    <td>{format!("{nb} · {}", libelle_source(&e.source))}</td>
                                    <td>
                                        {e.prevenue.then(|| view! { <Badge libelle="Prévenue" ton=TonEtat::Info/> })}
                                        {(e.justificatifs > 0).then(|| view! { <Badge libelle=format!("{} justificatif(s)", e.justificatifs) ton=TonEtat::Avertissement/> })}
                                        {(e.propositions_en_attente > 0).then(|| view! { <Badge libelle="Proposition d'un AED"/> })}
                                    </td>
                                    <td>
                                        <Bouton
                                            variante=Variante::Secondaire
                                            taille=Taille::Compacte
                                            libelle_accessible=libelle_ouvrir
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
        {move || {
            ouvert
                .get()
                .map(|id| {
                    let nom = noms.with_value(|n| {
                        n.iter()
                            .find(|(i, _)| *i == id)
                            .map(|(_, n)| n.clone())
                            .unwrap_or_else(|| "Élève".to_string())
                    });
                    view! {
                        <DetailEpisode
                            id=id
                            nom=nom
                            natures=natures
                            on_change=recharger
                            on_fermer=Callback::new(move |_| ouvert.set(None))
                        />
                    }
                })
        }}
    }
}

/// Qualification en masse : aperçu (rien n'est écrit), puis confirmation avec l'empreinte.
#[component]
fn BarreMasse(selection: RwSignal<Vec<Uuid>>, recharger: Callback<()>) -> impl IntoView {
    let notifications = use_notifications();
    let qualification = RwSignal::new("sans_motif".to_string());
    let apercu = RwSignal::new(None::<ApercuMasseDto>);
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let annonce = RwSignal::new(String::new());
    // Toute modification des choix invalide l'aperçu (même règle que la montée de classes).
    Effect::new(move |_| {
        selection.track();
        qualification.track();
        apercu.set(None);
    });
    let demander = move || {
        let ids = selection.get_untracked();
        match corps_masse(&ids, &qualification.get_untracked(), "", None) {
            Err(m) => erreur.set(Some(m.to_string())),
            Ok(corps) => {
                erreur.set(None);
                en_cours.set(true);
                spawn_local(async move {
                    match session::client().apercu_qualification_masse(&corps).await {
                        Ok(a) => apercu.set(Some(a)),
                        Err(e) => erreur.set(Some(message_echec(e))),
                    }
                    en_cours.set(false);
                });
            }
        }
    };
    let options_qualif: Vec<(String, String)> = QUALIFICATIONS
        .iter()
        .map(|(c, l)| (c.to_string(), l.to_string()))
        .collect();
    let confirmer = move || {
        let Some(a) = apercu.get_untracked() else {
            return;
        };
        let corps = match corps_masse(
            &a.retenus,
            &qualification.get_untracked(),
            "",
            Some(a.empreinte.clone()),
        ) {
            Ok(c) => c,
            Err(m) => {
                erreur.set(Some(m.to_string()));
                return;
            }
        };
        en_cours.set(true);
        spawn_local(async move {
            match session::client().qualifier_masse(&corps).await {
                Ok(b) => {
                    let t = format!("{} absence(s) qualifiée(s).", b.qualifies.len());
                    notifications.succes(t.clone());
                    annonce.set(t);
                    apercu.set(None);
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
            en_cours.set(false);
        });
    };
    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>
        <Show when=move || selection.with(|s| !s.is_empty())>
            <div class="asd-masse" role="region" aria-label="Qualification en masse">
                <span>{move || format!("{} sélectionnée(s)", selection.with(Vec::len))}</span>
                <Selection
                    id="asd-masse-qualif"
                    libelle="Qualifier comme"
                    valeur=qualification
                    options=options_qualif.clone()
                />
                <Bouton variante=Variante::Secondaire en_cours=en_cours on_click=Callback::new(move |_| demander())>
                    "Voir l'aperçu"
                </Bouton>
                <Bouton variante=Variante::Discret on_click=Callback::new(move |_| selection.set(vec![]))>
                    "Désélectionner"
                </Bouton>
            </div>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {move || {
                apercu
                    .get()
                    .map(|a| {
                        let retenus = a.retenus.len();
                        view! {
                            <Alerte niveau=Niveau::Info titre=format!("Aperçu : {retenus} absence(s) seront qualifiées « {} ».", libelle_qualification(&qualification.get()))>
                                {(!a.ecartes.is_empty())
                                    .then(|| view! {
                                        <p>{format!("{} écartée(s) :", a.ecartes.len())}</p>
                                        <ul>
                                            {a.ecartes.iter().map(|(_, c)| view! { <li>{libelle_ecart(c)}</li> }).collect_view()}
                                        </ul>
                                    })}
                                <Bouton en_cours=en_cours desactive=Signal::derive(move || retenus == 0) on_click=Callback::new(move |_| confirmer())>
                                    "Confirmer la qualification"
                                </Bouton>
                            </Alerte>
                        }
                    })
            }}
        </Show>
    }
}

// ---------------------------------------------------------------------------
// Détail d'un épisode et qualification individuelle
// ---------------------------------------------------------------------------

#[component]
fn DetailEpisode(
    id: Uuid,
    nom: String,
    natures: Signal<Vec<NatureMotifDto>>,
    on_change: Callback<()>,
    on_fermer: Callback<()>,
) -> impl IntoView {
    let detail = LocalResource::new(move || async move { session::client().episode(id).await });
    let justifs =
        LocalResource::new(
            move || async move { session::client().justificatifs_episode(id).await },
        );
    view! {
        <section class="asd-detail" aria-labelledby="asd-detail-titre" tabindex="-1">
            {move || match detail.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement de l'absence…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| detail.refetch())/> }.into_any(),
                Some(Ok(d)) => {
                    let e = d.episode.clone();
                    let nom = nom.clone();
                    let eleve_id = e.eleve_id;
                    let qualifiable = matches!(e.etat.as_str(), "ouvert" | "a_qualifier");
                    let apres = Callback::new(move |_| {
                        detail.refetch();
                        justifs.refetch();
                        on_change.run(());
                    });
                    view! {
                        <div class="asd-detail__tete">
                            <h2 id="asd-detail-titre" class="vs-section-titre">
                                {format!("{nom} — {}", libelle_periode(e.jour_debut, e.jour_fin))}
                            </h2>
                            <Bouton variante=Variante::Discret on_click=on_fermer libelle_accessible="Fermer le détail de l'absence">
                                "Fermer"
                            </Bouton>
                        </div>
                        <p>
                            <Badge libelle=libelle_etat_episode(&e.etat) ton=TonEtat::Info/>
                            {(e.etat == "qualifie").then(|| view! { <Badge libelle=libelle_qualification(&e.qualification)/> })}
                            {e.prevenue.then(|| view! { <Badge libelle="Prévenue par la famille" ton=TonEtat::Ok/> })}
                        </p>
                        <h3>"Séances"</h3>
                        <ul class="asd-seances">
                            {e.seances
                                .iter()
                                .map(|s| {
                                    let mut t = libelle_periode(s.jour, s.jour);
                                    if let Some(p) = s.periode.as_deref() {
                                        t.push_str(&format!(" · {}", libelle_periode_avis(p)));
                                    }
                                    if s.retiree {
                                        t.push_str(" · retirée (ne compte plus)");
                                    }
                                    if s.prevenue {
                                        t.push_str(" · prévenue");
                                    }
                                    view! { <li>{t}</li> }
                                })
                                .collect_view()}
                        </ul>
                        {(!e.textes.is_empty())
                            .then(|| view! {
                                <h3>"Déclaré par la famille ou l'accueil"</h3>
                                <ul class="asd-textes">
                                    {e.textes.iter().map(|t| view! { <li>{t.clone()}</li> }).collect_view()}
                                </ul>
                            })}
                        <Justificatifs id=id justifs=justifs/>
                        {(!d.propositions.is_empty())
                            .then(|| view! {
                                <h3>"Propositions des AED"</h3>
                                <ul>
                                    {d.propositions
                                        .iter()
                                        .map(|p| view! { <li>{format!("{}{}", libelle_qualification(&p.qualification), p.note.as_deref().map(|n| format!(" — {n}")).unwrap_or_default())}</li> })
                                        .collect_view()}
                                </ul>
                            })}
                        {if qualifiable {
                            view! { <FormulaireQualification episode=e.clone() natures=natures apres=apres proposition=d.propositions.iter().find(|p| p.etat == "en_attente").map(|p| (p.id, p.qualification.clone()))/> }.into_any()
                        } else {
                            view! {
                                <p class="texte-attenue">
                                    {format!(
                                        "Déjà qualifiée : {}. Famille : « {} ».",
                                        libelle_qualification(&e.qualification),
                                        libelle_famille(e.libelle_famille_code.as_deref().unwrap_or("")),
                                    )}
                                </p>
                            }
                                .into_any()
                        }}
                        <h3>"Historique"</h3>
                        <ol class="asd-histo">
                            {d.historique
                                .iter()
                                .map(|h| view! {
                                    <li>{format!("{} · {} · {}", date_heure_paris(h.cree_le), libelle_etat_episode(&h.etat), libelle_qualification(&h.qualification))}</li>
                                })
                                .collect_view()}
                        </ol>
                        <AvisEleve eleve_id=eleve_id titre="Avis aux familles pour cet élève" sous_section=true/>
                    }
                        .into_any()
                }
            }}
        </section>
    }
}

#[component]
fn Justificatifs(
    id: Uuid,
    justifs: LocalResource<Result<Vec<JustificatifDto>, ApiError>>,
) -> impl IntoView {
    view! {
        <h3>"Justificatifs"</h3>
        {move || match justifs.get() {
            None => view! { <Squelette lignes=1 libelle="Chargement des justificatifs…"/> }.into_any(),
            Some(Err(ApiError::Interdit(_) | ApiError::Introuvable(_))) => view! { <p class="texte-attenue">"Les justificatifs ne sont pas accessibles avec votre profil."</p> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| justifs.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <p class="texte-attenue">"Aucun justificatif déposé."</p> }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="asd-justifs">
                    {l.into_iter().map(|j| view! { <LigneJustificatif episode=id j=j/> }).collect_view()}
                </ul>
            }
                .into_any(),
        }}
    }
}

#[component]
fn LigneJustificatif(episode: Uuid, j: JustificatifDto) -> impl IntoView {
    let lien = RwSignal::new(None::<String>);
    let erreur = RwSignal::new(None::<String>);
    let jid = j.id;
    let est_piece = j.presente == "depose" && j.detruit_le.is_none();
    let obtenir = move || {
        erreur.set(None);
        spawn_local(async move {
            match session::client().lien_justificatif(episode, jid).await {
                Ok(l) => lien.set(Some(l.url)),
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    };
    let titre = format!(
        "{} · déposé par {} le {}",
        libelle_presentation_justificatif(&j.presente),
        libelle_depose_par(&j.depose_qualite),
        date_heure_paris(j.cree_le)
    );
    view! {
        <li>
            <strong>{titre}</strong>
            {j.medical.then(|| view! { <Badge libelle="Médical" ton=TonEtat::Avertissement/> })}
            {j.detruit_le.map(|_| view! { <Badge libelle="Pièce détruite"/> })}
            {j.texte.clone().map(|t| view! { <p class="asd-texte">{t}</p> })}
            {est_piece
                .then(|| view! {
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte on_click=Callback::new(move |_| obtenir())>
                        "Obtenir la pièce"
                    </Bouton>
                })}
            {move || lien.get().map(|u| view! {
                <a class="ui-btn ui-btn--primaire ui-btn--compact" href=u rel="noopener noreferrer">
                    "Télécharger la pièce (lien valable quelques minutes)"
                </a>
            })}
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        </li>
    }
}

#[component]
fn FormulaireQualification(
    episode: EpisodeDto,
    natures: Signal<Vec<NatureMotifDto>>,
    apres: Callback<()>,
    proposition: Option<(Uuid, String)>,
) -> impl IntoView {
    let notifications = use_notifications();
    let qualification = RwSignal::new(String::new());
    let nature = RwSignal::new(String::new());
    let libelle = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let version = episode.version;
    let id = episode.id;
    // Changer de qualification remet le libellé famille « par défaut ».
    Effect::new(move |_| {
        qualification.track();
        libelle.set(String::new());
    });
    let envoyer = move || {
        let saisie = SaisieQualification {
            qualification: qualification.get_untracked(),
            nature_code: nature.get_untracked(),
            libelle_famille: libelle.get_untracked(),
            note: note.get_untracked(),
        };
        // La proposition d'un AED n'est transmise que si la qualification choisie est la sienne.
        let acceptee = proposition
            .as_ref()
            .filter(|(_, q)| *q == saisie.qualification)
            .map(|(id, _)| *id);
        match corps_qualification(&saisie, version, acceptee) {
            Err(m) => erreur.set(Some(m.to_string())),
            Ok(corps) => {
                erreur.set(None);
                en_cours.set(true);
                spawn_local(async move {
                    match session::client().qualifier_episode(id, &corps).await {
                        Ok(_) => {
                            notifications.succes("Absence qualifiée : la famille en est informée selon les règles de l'établissement.");
                            apres.run(());
                        }
                        Err(ApiError::Conflit(_)) => erreur.set(Some(
                            "Cette absence a été modifiée entre-temps : elle est rechargée, relisez avant de qualifier.".into(),
                        )),
                        Err(e) => erreur.set(Some(message_echec(e))),
                    }
                    en_cours.set(false);
                });
            }
        }
    };
    let options_qualif = {
        let mut v = vec![(String::new(), "Choisir…".to_string())];
        v.extend(
            QUALIFICATIONS
                .iter()
                .map(|(c, l)| (c.to_string(), l.to_string())),
        );
        v
    };
    view! {
        <h3>"Qualifier"</h3>
        <form class="asd-form" on:submit=move |ev| { ev.prevent_default(); envoyer(); } novalidate>
            <Selection id="asd-qualif" libelle="Qualification" valeur=qualification options=options_qualif requis=true/>
            <Show when=move || qualification.get() == "reputee_legitime">
                {move || {
                    let mut o = vec![(String::new(), "Choisir…".to_string())];
                    o.extend(natures_legitimes(&natures.get()));
                    view! { <Selection id="asd-nature" libelle="Motif légitime" valeur=nature options=o requis=true/> }
                }}
            </Show>
            <Show when=move || !qualification.get().is_empty()>
                {move || {
                    let mut o = vec![(String::new(), "Libellé par défaut".to_string())];
                    o.extend(libelles_famille_pour(&qualification.get()).into_iter().map(|(c, l)| (c.to_string(), l.to_string())));
                    view! { <Selection id="asd-libelle" libelle="Ce que voit la famille" valeur=libelle options=o/> }
                }}
            </Show>
            <ZoneTexte
                id="asd-note"
                libelle="Note interne (jamais visible de la famille ni du professeur)"
                valeur=note
            />
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            <Bouton soumettre=true en_cours=en_cours>"Qualifier l'absence"</Bouton>
        </form>
    }
}

// ---------------------------------------------------------------------------
// Sans réponse (relance graduée)
// ---------------------------------------------------------------------------

#[component]
fn SansReponse() -> impl IntoView {
    let liste = LocalResource::new(|| async {
        let sr = session::client().sans_reponse().await?;
        // Les noms viennent de la file des épisodes (le registre de relance ne porte que les identifiants).
        let eps = session::client()
            .episodes(&FiltreEpisodesApi {
                etat: Some("tous"),
                limite: Some(200),
                ..Default::default()
            })
            .await
            .unwrap_or_default();
        Ok::<_, ApiError>((trier_sans_reponse(sr), eps))
    });
    view! {
        {move || match liste.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de la file…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            Some(Ok((l, _))) if l.is_empty() => view! {
                <EtatVide
                    titre="Aucune absence sans réponse"
                    message="Les familles ont répondu, ou la relance n'est pas encore arrivée au 5e jour scolaire."
                />
            }
                .into_any(),
            Some(Ok((l, eps))) => view! { <ListeSansReponse l=l eps=eps/> }.into_any(),
        }}
    }
}

#[component]
fn ListeSansReponse(l: Vec<EpisodeSansReponseDto>, eps: Vec<EpisodeDto>) -> impl IntoView {
    view! {
        <ul class="ui-liste">
            {l.into_iter()
                .map(|r| {
                    let nom = eps
                        .iter()
                        .find(|e| e.id == r.absence_id)
                        .map(nom_eleve)
                        .unwrap_or_else(|| "Élève".to_string());
                    view! {
                        <li class="ui-liste__element">
                            <div class="ui-liste__principal">
                                <span class="ui-liste__titre">{format!("{nom} — {}", libelle_periode(r.jour_debut, r.jour_fin))}</span>
                                <span class="ui-liste__detail">{libelle_etape(&r.derniere_etape)}</span>
                            </div>
                            {r.courrier_en_cours.then(|| view! { <Badge libelle="Courrier en cours" ton=TonEtat::Info/> })}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}

// ---------------------------------------------------------------------------
// Avis aux familles (prévus, partis, démentis)
// ---------------------------------------------------------------------------

#[component]
fn AvisFamilles() -> impl IntoView {
    let eleve = RwSignal::new(None::<(Uuid, String)>);
    view! {
        <p class="vs-note">
            "Pour un élève : les avis par demi-journée envoyés à sa famille, ceux qui sont prévus, et les démentis."
        </p>
        <RechercheEleve
            id="asd-avis-recherche"
            libelle="Rechercher un élève"
            action="Voir les avis de"
            on_choisir=Callback::new(move |e| eleve.set(Some(e)))
        />
        {move || eleve.get().map(|(id, nom)| view! { <AvisEleve eleve_id=id titre=format!("Avis aux familles : {nom}")/> })}
    }
}

#[component]
fn AvisEleve(
    eleve_id: Uuid,
    #[prop(into)] titre: String,
    /// Dans le détail d'un épisode (h3) ; seul sur la page, le titre est de niveau 2.
    #[prop(optional)]
    sous_section: bool,
) -> impl IntoView {
    let du: NaiveDate = crate::noyau::temps::aujourdhui() - chrono::Duration::days(JOURS_FILE);
    let avis = LocalResource::new(move || async move {
        session::client().avis_eleve(eleve_id, Some(du)).await
    });
    let etiquette = titre.clone();
    view! {
        <section class="asd-avis" aria-label=etiquette>
            {if sous_section { view! { <h3>{titre}</h3> }.into_any() } else { view! { <h2 class="vs-section-titre">{titre}</h2> }.into_any() }}
            {move || match avis.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des avis…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_) | ApiError::Introuvable(_))) => view! { <p class="texte-attenue">"Les avis aux familles ne sont pas accessibles avec votre profil."</p> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| avis.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <p class="texte-attenue">"Aucun avis sur cette période."</p> }.into_any(),
                Some(Ok(l)) => view! { <ListeAvis l=l/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn ListeAvis(l: Vec<AvisDemiJourneeDto>) -> impl IntoView {
    view! {
        <ul class="asd-avis__liste">
            {l.into_iter()
                .map(|a| {
                    let etat = etat_avis(&a);
                    let ton = match etat {
                        EtatAvis::Prevu | EtatAvis::EnCours => TonEtat::Info,
                        EtatAvis::Parti => TonEtat::Ok,
                        EtatAvis::Annule => TonEtat::Neutre,
                        EtatAvis::Dementi => TonEtat::Avertissement,
                    };
                    let cause = libelle_cause(a.cause_code.as_deref());
                    view! {
                        <li>
                            <span>{format!("{} · {}", libelle_periode(a.jour, a.jour), libelle_periode_avis(&a.periode))}</span>
                            <Badge libelle=etat.libelle() ton=ton/>
                            {(!cause.is_empty()).then(|| view! { <span class="texte-attenue">{format!(" — {cause}")}</span> })}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}

/// Droits d'affichage : l'entrée de navigation n'est proposée qu'à qui peut lire les épisodes.
pub fn peut_voir_assiduite(moteur: &crate::noyau::droits_moteur::DroitsMoteur) -> bool {
    moteur.a("absences.lire_motif")
}
