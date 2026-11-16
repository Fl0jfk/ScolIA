//! **Tableau de bord de la vie scolaire en panneaux** (socle 12, AS14b-bis), monté sur l'accueil des profils qui
//! vérifient les appels (CPE, AED, vie scolaire, direction) : un bandeau « Urgent » toujours visible, puis cinq
//! panneaux (absents à traiter, exclus de cours, appels non faits, incohérences d'appel, familles à rappeler),
//! disposés selon l'heure ou selon le choix épinglé de la personne (mémorisé côté serveur).
//!
//! **Réutilise** la file du matin (AS6, `GET /assiduite/file-du-matin`) et les actions déjà offertes par les écrans
//! AS14b / AS6 ; n'ajoute que les exclus du jour, l'heure de bascule et la disposition (routes
//! `ent-api/src/assiduite_tableau.rs`). Le serveur décide de tout droit : l'écran masque les boutons que le moteur
//! de droits ne donne pas et traduit les refus.
//!
//! Heure : celle de l'établissement, donnée par le serveur. Paramètre `?heure=HH:MM` dans l'adresse : **simule
//! l'heure pour la disposition seulement** (essais, formation) ; il ne change aucune donnée ni aucun droit.
//! Le rafraîchissement automatique (chaque minute) saute son tour quand le focus est dans le tableau : un
//! utilisateur au clavier ne perd jamais sa place.

pub mod modele;

use chrono::{NaiveDate, NaiveTime};
use ent_web_api::assiduite_types::{
    ConfigTableauDto, EntreeFileDto, ExcluDuJourDto, ExclusDuJourDto, FileDuMatinDto,
    TracerContactBody,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, Dialogue, EtatVide, Niveau, Selection, Squelette, Taille,
    TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::droits_moteur::DroitsMoteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::lire_heure;
use modele::{
    appliquer, depuis, disposition, est_epinglee, est_prise, heure_courte, libelle_categorie,
    libelle_compteur, libelle_detail, lignes_visibles, moment, nom_complet, regrouper, titre_ligne,
    vers_preference, Modif, Moment, Panneau, PanneauAffiche, Regroupement, ISSUES_ALERTE,
    ISSUES_CONTACT, RAISONS_A_LA_PLACE, RAISONS_PREVENIR,
};

/// Période du rafraîchissement automatique.
#[cfg(target_arch = "wasm32")]
const RAFRAICHISSEMENT: std::time::Duration = std::time::Duration::from_secs(60);

/// Ce que la personne est en train de traiter dans une boîte de dialogue.
#[derive(Clone, Debug, PartialEq)]
enum Cible {
    Contact {
        eleve_id: Uuid,
        absence_id: Option<Uuid>,
        nom: String,
    },
    Traiter {
        alerte_id: Uuid,
        nom: String,
    },
    Prevenir {
        eleve_id: Uuid,
        nom: String,
    },
    ALaPlace {
        creneau_id: Uuid,
        titre: String,
    },
}

/// Contexte partagé par les lignes (tous les champs sont `Copy`).
#[derive(Clone, Copy)]
struct Ctx {
    ecriture: Ecriture,
    moteur: Signal<DroitsMoteur>,
    jour: Signal<Option<NaiveDate>>,
    ouvrir: Callback<Cible>,
}

/// Données lues, gardées d'un chargement au suivant : un échec de rafraîchissement n'efface rien.
#[derive(Clone, Debug, PartialEq)]
struct Donnees {
    file: FileDuMatinDto,
    exclus: Option<ExclusDuJourDto>,
}

fn libelle_fond(m: Moment, bascule: NaiveTime) -> String {
    match m {
        Moment::AvantBascule => format!("Avant {} : les absents d'abord", heure_courte(bascule)),
        Moment::ApresBascule => format!(
            "Depuis {} : les exclus et les appels d'abord",
            heure_courte(bascule)
        ),
    }
}

/// Focus actuellement dans le tableau ? (le rafraîchissement automatique saute alors son tour)
#[cfg(target_arch = "wasm32")]
fn focus_dans_le_tableau() -> bool {
    use wasm_bindgen::JsCast;
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return false;
    };
    match (doc.get_element_by_id("tdb"), doc.active_element()) {
        (Some(tableau), Some(actif)) => {
            tableau.contains(Some(actif.unchecked_ref::<web_sys::Node>()))
        }
        _ => false,
    }
}

#[component]
pub fn TableauVieScolaire() -> impl IntoView {
    let moteur = use_droits_moteur();
    let requete = use_query_map();
    let simulee = Memo::new(move |_| requete.get().get("heure").and_then(|h| lire_heure(&h)));

    let file = LocalResource::new(|| async { session::client().file_du_matin(None).await });
    let exclus = LocalResource::new(|| async { session::client().exclus_du_jour().await });
    let config = LocalResource::new(|| async { session::client().config_tableau().await });

    let recharger = Callback::new(move |_: ()| {
        file.refetch();
        exclus.refetch();
    });
    #[cfg(target_arch = "wasm32")]
    if let Ok(h) = set_interval_with_handle(
        move || {
            if !focus_dans_le_tableau() {
                recharger.run(());
            }
        },
        RAFRAICHISSEMENT,
    ) {
        on_cleanup(move || h.clear());
    }

    // Dernières données lues (jamais effacées par un échec) et dernier échec de chargement.
    let donnees = RwSignal::new(None::<Donnees>);
    let echec = RwSignal::new(None::<ApiError>);
    Effect::new(move |_| match (file.get(), exclus.get()) {
        (Some(Ok(f)), Some(x)) => {
            echec.set(None);
            donnees.set(Some(Donnees {
                file: f,
                exclus: x.ok(),
            }));
        }
        (Some(Err(e)), _) => echec.set(Some(e)),
        _ => {}
    });

    // Réglage de l'établissement et disposition personnelle (copies locales, mises à jour après chaque écriture).
    let bascule = RwSignal::new(modele_defaut_bascule());
    let pref = RwSignal::new(None::<Vec<ent_web_api::assiduite_types::PreferencePanneauDto>>);
    let config_en_echec = RwSignal::new(false);
    Effect::new(move |_| {
        if let Some(r) = config.get() {
            match r {
                Ok(ConfigTableauDto {
                    heure_bascule,
                    preference,
                }) => {
                    config_en_echec.set(false);
                    bascule.set(heure_bascule);
                    pref.set(preference);
                }
                Err(_) => config_en_echec.set(true),
            }
        }
    });
    // Repli sans épingler : valable le temps de la page, en mode automatique seulement.
    let replis_auto = RwSignal::new(Vec::<Panneau>::new());
    let tout_voir = RwSignal::new(Vec::<Panneau>::new());
    let dispo_ouverte = RwSignal::new(false);

    let heure = Memo::new(move |_| {
        simulee
            .get()
            .or_else(|| donnees.with(|d| d.as_ref().map(|d| d.file.heure_locale)))
    });
    let moment_courant = Memo::new(move |_| {
        heure
            .get()
            .map_or(Moment::AvantBascule, |h| moment(h, bascule.get()))
    });
    let epinglee = Memo::new(move |_| pref.with(|p| est_epinglee(p.as_deref())));
    let affiche = Memo::new(move |_| {
        let mut d = pref.with(|p| disposition(moment_courant.get(), p.as_deref()));
        if !epinglee.get() {
            let replis = replis_auto.get();
            for x in d.iter_mut().filter(|x| replis.contains(&x.panneau)) {
                x.replie = true;
            }
        }
        d
    });
    let regroupement = Memo::new(move |_| {
        donnees.with(|d| {
            d.as_ref()
                .map(|d| regrouper(&d.file, d.exclus.as_ref()))
                .unwrap_or_default()
        })
    });
    let jour = Signal::derive(move || donnees.with(|d| d.as_ref().map(|d| d.file.jour)));

    let ecriture = Ecriture::nouveau(recharger);

    // Boîtes de dialogue : une par action, une cible partagée.
    let cible = RwSignal::new(None::<Cible>);
    let ouvre_contact = RwSignal::new(false);
    let ouvre_traiter = RwSignal::new(false);
    let ouvre_prevenir = RwSignal::new(false);
    let ouvre_a_la_place = RwSignal::new(false);
    let ouvrir = Callback::new(move |c: Cible| {
        let o = match &c {
            Cible::Contact { .. } => ouvre_contact,
            Cible::Traiter { .. } => ouvre_traiter,
            Cible::Prevenir { .. } => ouvre_prevenir,
            Cible::ALaPlace { .. } => ouvre_a_la_place,
        };
        cible.set(Some(c));
        o.set(true);
    });
    let ctx = Ctx {
        ecriture,
        moteur,
        jour,
        ouvrir,
    };

    // Changement de disposition : épingle (PUT) ; en automatique, replier reste local.
    let modifier = Callback::new(move |m: Modif| {
        if let (Modif::Repli(p), false) = (m, epinglee.get_untracked()) {
            replis_auto.update(|v| {
                if let Some(i) = v.iter().position(|x| *x == p) {
                    v.remove(i);
                } else {
                    v.push(p);
                }
            });
            return;
        }
        let nouvelle = appliquer(affiche.get_untracked(), m);
        ecriture.lancer(async move {
            let r = session::client()
                .poser_preference_tableau(vers_preference(&nouvelle))
                .await?;
            pref.set(Some(r));
            replis_auto.set(Vec::new());
            Ok("Disposition enregistrée sur votre compte.".to_string())
        });
    });
    let revenir_auto = Callback::new(move |_: ()| {
        ecriture.lancer(async move {
            session::client().effacer_preference_tableau().await?;
            pref.set(None);
            replis_auto.set(Vec::new());
            Ok("Disposition automatique rétablie.".to_string())
        });
    });

    view! {
        <div id="tdb" class="tdb">
            <header class="tdb-entete">
                <h2 id="tdb-titre" class="tdb-titre">"Vie scolaire du jour"</h2>
                <p class="tdb-mode">
                    {move || {
                        let m = moment_courant.get();
                        if epinglee.get() {
                            "Disposition choisie par vous".to_string()
                        } else {
                            libelle_fond(m, bascule.get())
                        }
                    }}
                </p>
                <div class="tdb-entete__actions">
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                        on_click=Callback::new(move |_| recharger.run(()))>
                        "Actualiser"
                    </Bouton>
                    <button type="button" class="ui-btn ui-btn--secondaire ui-btn--compact"
                        aria-expanded=move || dispo_ouverte.get().to_string()
                        aria-controls="tdb-dispo"
                        on:click=move |_| dispo_ouverte.update(|o| *o = !*o)>
                        "Disposition"
                    </button>
                </div>
            </header>
            <RetourEcriture ecriture=ecriture/>
            {move || config_en_echec.get().then(|| view! {
                <Alerte niveau=Niveau::Avertissement
                    titre="Votre disposition n'a pas pu être chargée : la disposition automatique est affichée."/>
            })}
            <Disposition ouverte=dispo_ouverte affiche=affiche epinglee=epinglee bascule=bascule
                moteur=moteur modifier=modifier revenir_auto=revenir_auto ecriture=ecriture/>
            {move || match (donnees.get(), echec.get()) {
                (None, None) => view! { <Squelette lignes=5 libelle="Chargement de la vie scolaire du jour…"/> }.into_any(),
                (None, Some(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| recharger.run(()))/>
                }.into_any(),
                (Some(_), e) => view! {
                    {e.map(|_| view! {
                        <Alerte niveau=Niveau::Avertissement
                            titre="L'actualisation a échoué : les données affichées datent du dernier chargement."/>
                    })}
                    <Contenu regroupement=regroupement affiche=affiche ctx=ctx tout_voir=tout_voir
                        modifier=modifier/>
                }.into_any(),
            }}
            <Dialogues cible=cible ctx=ctx ouvre_contact=ouvre_contact ouvre_traiter=ouvre_traiter
                ouvre_prevenir=ouvre_prevenir ouvre_a_la_place=ouvre_a_la_place/>
        </div>
    }
}

fn modele_defaut_bascule() -> NaiveTime {
    NaiveTime::from_hms_opt(11, 0, 0).unwrap_or_default()
}

// ─── Contenu : bandeau et panneaux ────────────────────────────────────────────────────────

#[component]
fn Contenu(
    regroupement: Memo<Regroupement>,
    affiche: Memo<Vec<PanneauAffiche>>,
    ctx: Ctx,
    tout_voir: RwSignal<Vec<Panneau>>,
    modifier: Callback<Modif>,
) -> impl IntoView {
    let urgent = Memo::new(move |_| regroupement.with(|r| r.urgent.clone()));
    view! {
        <BandeauUrgent urgent=urgent ctx=ctx/>
        {move || (regroupement.with(|r| r.ignorees) > 0).then(|| view! {
            <Alerte niveau=Niveau::Info
                titre=format!("{} ligne(s) de la file n'ont pas de panneau et ne sont pas affichées ici.", regroupement.with(|r| r.ignorees))/>
        })}
        {move || tout_masque_msg(affiche)}
        <div class="tdb-grille">
            <For each=move || affiche.get() key=|p| (p.panneau, p.grand, p.replie, p.masque)
                children=move |p| {
                    (!p.masque).then(|| view! {
                        <PanneauVue p=p regroupement=regroupement ctx=ctx tout_voir=tout_voir
                            modifier=modifier/>
                    })
                }/>
        </div>
    }
}

fn tout_masque_msg(affiche: Memo<Vec<PanneauAffiche>>) -> Option<impl IntoView> {
    affiche.with(|d| modele::tout_masque(d)).then(|| {
        view! {
            <EtatVide titre="Tous les panneaux sont masqués"
                message="Ouvrez « Disposition » pour les afficher de nouveau. Le bandeau Urgent reste toujours visible."/>
        }
    })
}

/// Bandeau « Urgent » : jamais masquable, discret quand il est vide.
#[component]
fn BandeauUrgent(urgent: Memo<Vec<EntreeFileDto>>, ctx: Ctx) -> impl IntoView {
    let classe = move || {
        if urgent.with(Vec::is_empty) {
            "tdb-urgent tdb-urgent--vide"
        } else {
            "tdb-urgent tdb-urgent--plein"
        }
    };
    view! {
        <section class=classe aria-labelledby="tdb-urgent-titre">
            <h3 id="tdb-urgent-titre" class="tdb-urgent__titre">
                "Urgent"
                {move || {
                    let n = urgent.with(Vec::len);
                    (n > 0).then(|| view! { <span class="tdb-urgent__compteur">{format!(" ({n})")}</span> })
                }}
            </h3>
            {move || if urgent.with(Vec::is_empty) {
                view! { <p class="tdb-urgent__rien">"Rien d'urgent"</p> }.into_any()
            } else {
                view! {
                    <ul class="tdb-liste">
                        <For each=move || urgent.get() key=|e| format!("{e:?}")
                            children=move |e| view! { <LigneFile e=e ctx=ctx urgent=true/> }/>
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn PanneauVue(
    p: PanneauAffiche,
    regroupement: Memo<Regroupement>,
    ctx: Ctx,
    tout_voir: RwSignal<Vec<Panneau>>,
    modifier: Callback<Modif>,
) -> impl IntoView {
    let panneau = p.panneau;
    let code = panneau.code();
    let id_titre = format!("tdb-p-{code}-titre");
    let titre_ref = id_titre.clone();
    let id_corps = format!("tdb-p-{code}-corps");
    let compte = Memo::new(move |_| regroupement.with(|r| r.compte(panneau)));
    let tout = Memo::new(move |_| tout_voir.with(|v| v.contains(&panneau)));
    let classe = if p.grand {
        "tdb-panneau tdb-panneau--grand"
    } else {
        "tdb-panneau"
    };
    let replie = p.replie;
    let corps_id = id_corps.clone();
    view! {
        <section class=classe aria-labelledby=titre_ref>
            <header class="tdb-panneau__entete">
                <h3 id=id_titre class="tdb-panneau__titre">{panneau.titre()}</h3>
                <span class="tdb-compteur">
                    {move || {
                        let n = compte.get();
                        let ton = if n == 0 { TonEtat::Neutre } else { TonEtat::Avertissement };
                        view! { <Badge libelle=libelle_compteur(n) ton=ton/> }
                    }}
                </span>
                <button type="button" class="ui-btn ui-btn--discret ui-btn--compact tdb-repli"
                    aria-expanded=(!replie).to_string()
                    aria-controls=id_corps
                    on:click=move |_| modifier.run(Modif::Repli(panneau))>
                    {if replie { "Déplier" } else { "Replier" }}
                    <span class="ui-sr">{format!(" {}", panneau.titre())}</span>
                </button>
            </header>
            <div id=corps_id class="tdb-panneau__corps" hidden=replie>
                {move || {
                    let n = compte.get();
                    if n == 0 {
                        return view! { <p class="tdb-vide">{panneau.vide()}</p> }.into_any();
                    }
                    let visibles = if tout.get() { n } else { lignes_visibles(p.grand) };
                    view! {
                        <ul class="tdb-liste">
                            {if panneau == Panneau::Exclus {
                                let l = regroupement.with(|r| r.exclus.iter().take(visibles).cloned().collect::<Vec<_>>());
                                view! {
                                    <For each=move || l.clone() key=|x| format!("{x:?}")
                                        children=move |x| view! { <LigneExclu x=x ctx=ctx/> }/>
                                }.into_any()
                            } else {
                                let l = regroupement.with(|r| {
                                    let src = match panneau {
                                        Panneau::Absents => &r.absents,
                                        Panneau::Appels => &r.appels,
                                        Panneau::Incoherences => &r.incoherences,
                                        _ => &r.familles,
                                    };
                                    src.iter().take(visibles).cloned().collect::<Vec<_>>()
                                });
                                view! {
                                    <For each=move || l.clone() key=|e| format!("{e:?}")
                                        children=move |e| view! { <LigneFile e=e ctx=ctx urgent=false/> }/>
                                }.into_any()
                            }}
                        </ul>
                        {(n > lignes_visibles(p.grand)).then(|| view! {
                            <button type="button" class="ui-btn ui-btn--discret ui-btn--compact"
                                aria-expanded=tout.get().to_string()
                                on:click=move |_| tout_voir.update(|v| {
                                    if let Some(i) = v.iter().position(|x| *x == panneau) { v.remove(i); } else { v.push(panneau); }
                                })>
                                {if tout.get() { "Voir moins".to_string() } else { format!("Voir tout ({n})") }}
                            </button>
                        })}
                    }.into_any()
                }}
            </div>
        </section>
    }
}

// ─── Lignes et actions ────────────────────────────────────────────────────────────────────

fn lien_fiche(eleve: Option<Uuid>, nom: &str) -> Option<impl IntoView> {
    let id = eleve?;
    Some(view! {
        <a class="tdb-lien" href=format!("/web/eleves/{id}") aria-label=format!("Fiche de {nom}")>"Fiche"</a>
    })
}

#[component]
fn LigneFile(e: EntreeFileDto, ctx: Ctx, urgent: bool) -> impl IntoView {
    let titre = titre_ligne(&e);
    let raison =
        libelle_detail(e.detail_code.as_deref()).unwrap_or_else(|| libelle_categorie(&e.categorie));
    let quand = depuis(e.depuis, if urgent { e.minutes_depuis } else { None });
    let prise = est_prise(&e);
    let cat = e.categorie.clone();
    let eleve = e.eleve_id;
    let moteur = ctx.moteur;
    let ecriture = ctx.ecriture;
    let nom = titre.clone();

    let je_prends = e.alerte_id.map(|alerte_id| {
        move || {
            (moteur.get().a("assiduite.suivre") && !prise).then(|| {
                let n = nom.clone();
                view! {
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire en_cours=ecriture.en_cours
                        libelle_accessible=format!("Je prends : {n}")
                        on_click=Callback::new(move |_| {
                            ecriture.lancer(async move {
                                session::client().alerte_prendre(alerte_id).await?;
                                Ok("Vous avez pris cette alerte (verrou de 15 minutes).".to_string())
                            });
                        })>
                        "Je prends"
                    </Bouton>
                }
            })
        }
    });
    let traiter = e.alerte_id.map(|alerte_id| {
        let nom = titre.clone();
        move || {
            moteur.get().a("assiduite.suivre").then(|| {
                let n = nom.clone();
                view! {
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire
                        libelle_accessible=format!("Traiter l'alerte : {n}")
                        on_click=Callback::new(move |_| ctx.ouvrir.run(Cible::Traiter { alerte_id, nom: n.clone() }))>
                        "Traiter…"
                    </Bouton>
                }
            })
        }
    });
    let contact = eleve.map(|eleve_id| {
        let nom = titre.clone();
        let absence_id = e.absence_id;
        move || {
            moteur.get().a("assiduite.suivre").then(|| {
                let n = nom.clone();
                view! {
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire
                        libelle_accessible=format!("Tracer un contact avec la famille de {n}")
                        on_click=Callback::new(move |_| ctx.ouvrir.run(Cible::Contact { eleve_id, absence_id, nom: n.clone() }))>
                        "Contact…"
                    </Bouton>
                }
            })
        }
    });
    let prevenir = eleve.filter(|_| matches!(cat.as_str(), "sans_nouvelles" | "non_jointe")).map(|eleve_id| {
        let nom = titre.clone();
        move || {
            moteur.get().a("assiduite.alerter").then(|| {
                let n = nom.clone();
                view! {
                    <Bouton taille=Taille::Compacte variante=Variante::Danger
                        libelle_accessible=format!("Prévenir maintenant la famille de {n}")
                        on_click=Callback::new(move |_| ctx.ouvrir.run(Cible::Prevenir { eleve_id, nom: n.clone() }))>
                        "Prévenir maintenant…"
                    </Bouton>
                }
            })
        }
    });
    let vigilance = eleve.filter(|_| cat == "sans_nouvelles").map(|eleve_id| {
        let (nom, deja) = (titre.clone(), e.vigilance);
        move || {
            moteur.get().a("absences.qualifier").then(|| {
                let n = nom.clone();
                view! {
                    <Bouton taille=Taille::Compacte variante=Variante::Discret en_cours=ecriture.en_cours
                        libelle_accessible=format!("{} : {n}", if deja { "Lever la vigilance" } else { "Mettre sous vigilance" })
                        on_click=Callback::new(move |_| {
                            ecriture.lancer(async move {
                                session::client().vigilance(eleve_id, !deja).await?;
                                Ok(if deja { "Vigilance levée." } else { "Élève mis sous vigilance." }.to_string())
                            });
                        })>
                        {if deja { "Lever la vigilance" } else { "Vigilance" }}
                    </Bouton>
                }
            })
        }
    });
    let a_la_place = e.creneau_id.filter(|_| cat == "appel_non_fait").map(|creneau_id| {
        let jour = ctx.jour;
        let (n_relance, n_place) = (titre.clone(), titre.clone());
        move || {
            let relancer = moteur.get().a("appel.verifier").then(|| {
                let n = n_relance.clone();
                view! {
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire en_cours=ecriture.en_cours
                        libelle_accessible=format!("Relancer l'intervenant : {n}")
                        on_click=Callback::new(move |_| {
                            let Some(j) = jour.get_untracked() else { return };
                            ecriture.lancer(async move {
                                session::client().relancer_appel(creneau_id, j).await?;
                                Ok("Relance envoyée à l'intervenant.".to_string())
                            });
                        })>
                        "Relancer"
                    </Bouton>
                }
            });
            let place = moteur.get().a("appel.faire_tout").then(|| {
                let n = n_place.clone();
                view! {
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire
                        libelle_accessible=format!("Faire l'appel à la place : {n}")
                        on_click=Callback::new(move |_| ctx.ouvrir.run(Cible::ALaPlace { creneau_id, titre: n.clone() }))>
                        "Faire l'appel…"
                    </Bouton>
                }
            });
            view! { {relancer}{place} }
        }
    });
    let ouvrir_journee = (cat == "incoherence").then(|| {
        view! { <a class="tdb-lien" href="/web/journee">"Ouvrir la journée"</a> }
    });
    let fiche = lien_fiche(eleve, &titre);

    view! {
        <li class="tdb-ligne">
            <div class="tdb-ligne__texte">
                <strong class="tdb-ligne__qui">{titre}</strong>
                <span class="tdb-ligne__raison">{raison}</span>
                {e.classe.clone().map(|c| view! { <span class="tdb-ligne__meta">{format!("Classe {c}")}</span> })}
                {match e.mineur { Some(true) => Some("Mineur"), Some(false) => Some("Majeur"), None => None }
                    .map(|m| view! { <span class="tdb-ligne__meta">{m}</span> })}
                {e.vigilance.then(|| view! { <Badge libelle="Sous vigilance" ton=TonEtat::Avertissement/> })}
                {quand.map(|q| view! { <span class="tdb-ligne__meta">{q}</span> })}
                {prise.then(|| view! { <Badge libelle="Pris en charge" ton=TonEtat::Info/> })}
                {(e.contacts_sans_succes > 0).then(|| view! {
                    <span class="tdb-ligne__meta">{format!("{} tentative(s) sans succès", e.contacts_sans_succes)}</span>
                })}
                {(!e.raisons.is_empty()).then(|| view! {
                    <span class="tdb-ligne__meta">
                        {format!("Aussi : {}", e.raisons.iter().map(|r| libelle_categorie(r)).collect::<Vec<_>>().join(", "))}
                    </span>
                })}
            </div>
            <div class="tdb-ligne__actions">
                {je_prends}
                {traiter}
                {contact}
                {prevenir}
                {vigilance}
                {a_la_place}
                {ouvrir_journee}
                {fiche}
            </div>
        </li>
    }
}

#[component]
fn LigneExclu(x: ExcluDuJourDto, ctx: Ctx) -> impl IntoView {
    let nom = nom_complet(Some(&x.prenom), Some(&x.nom));
    let quand = depuis(Some(x.depuis), Some(x.minutes_depuis))
        .map(|d| d.replacen("depuis", "exclu depuis", 1));
    let (moteur, ecriture) = (ctx.moteur, ctx.ecriture);
    let (appel_id, eleve_id) = (x.appel_id, x.eleve_id);
    let n = nom.clone();
    let arrive = move || {
        moteur.get().a("localisation.poser").then(|| {
            let n = n.clone();
            view! {
                <Bouton taille=Taille::Compacte variante=Variante::Secondaire en_cours=ecriture.en_cours
                    libelle_accessible=format!("Confirmer l'arrivée de {n}")
                    on_click=Callback::new(move |_| {
                        ecriture.lancer(async move {
                            session::client().confirmer_arrivee_vs(appel_id, eleve_id).await?;
                            Ok("Arrivée à la vie scolaire confirmée.".to_string())
                        });
                    })>
                    "Arrivé"
                </Bouton>
            }
        })
    };
    view! {
        <li class="tdb-ligne">
            <div class="tdb-ligne__texte">
                <strong class="tdb-ligne__qui">{nom.clone()}</strong>
                <span class="tdb-ligne__raison">"Exclu de cours, arrivée non confirmée"</span>
                {x.classe.clone().map(|c| view! { <span class="tdb-ligne__meta">{format!("Classe {c}")}</span> })}
                <span class="tdb-ligne__meta">{if x.mineur { "Mineur" } else { "Majeur" }}</span>
                {quand.map(|q| view! { <span class="tdb-ligne__meta">{q}</span> })}
                {x.urgent.then(|| view! { <Badge libelle="Dans le bandeau Urgent" ton=TonEtat::Mal/> })}
            </div>
            <div class="tdb-ligne__actions">
                {arrive}
                {lien_fiche(Some(x.eleve_id), &nom)}
            </div>
        </li>
    }
}

// ─── Disposition ──────────────────────────────────────────────────────────────────────────

#[component]
fn Disposition(
    ouverte: RwSignal<bool>,
    affiche: Memo<Vec<PanneauAffiche>>,
    epinglee: Memo<bool>,
    bascule: RwSignal<NaiveTime>,
    moteur: Signal<DroitsMoteur>,
    modifier: Callback<Modif>,
    revenir_auto: Callback<()>,
    ecriture: Ecriture,
) -> impl IntoView {
    let lignes_dispo = move || {
        affiche
            .get()
            .into_iter()
            .enumerate()
            .collect::<Vec<(usize, PanneauAffiche)>>()
    };
    let heure_saisie = RwSignal::new(String::new());
    let erreur_heure = RwSignal::new(None::<String>);
    Effect::new(move |_| heure_saisie.set(heure_courte(bascule.get())));
    let regler = Callback::new(
        move |_| match lire_heure(heure_saisie.get_untracked().trim()) {
            Some(h) => {
                erreur_heure.set(None);
                ecriture.lancer(async move {
                    let r = session::client().regler_heure_bascule(h).await?;
                    bascule.set(r);
                    Ok(format!("Heure de bascule réglée à {}.", heure_courte(r)))
                });
            }
            None => erreur_heure.set(Some("Saisissez une heure valide (HH:MM).".into())),
        },
    );
    view! {
        <div id="tdb-dispo" class="tdb-dispo" hidden=move || !ouverte.get()>
            <p class="tdb-dispo__mode">
                {move || if epinglee.get() {
                    "Votre ordre est épinglé : il ne change plus avec l'heure et vous le retrouvez sur tout poste."
                } else {
                    "Disposition automatique : l'ordre suit l'heure. Choisir un ordre, masquer un panneau ou en replier un l'épingle sur votre compte."
                }}
            </p>
            <ol class="tdb-dispo__liste">
                <For each=lignes_dispo
                    key=|(i, p)| (*i, p.panneau, p.masque, p.replie)
                    children=move |(i, p)| {
                        let (haut, bas) = (i == 0, i == 4);
                        let t = p.panneau.titre();
                        view! {
                            <li class="tdb-dispo__ligne">
                                <span class="tdb-dispo__nom">
                                    {t}
                                    {p.masque.then(|| view! { <span class="tdb-ligne__meta">" (masqué)"</span> })}
                                </span>
                                <Bouton taille=Taille::Compacte variante=Variante::Discret desactive=haut
                                    libelle_accessible=format!("Monter {t}")
                                    on_click=Callback::new(move |_| modifier.run(Modif::Monter(i)))>"Monter"</Bouton>
                                <Bouton taille=Taille::Compacte variante=Variante::Discret desactive=bas
                                    libelle_accessible=format!("Descendre {t}")
                                    on_click=Callback::new(move |_| modifier.run(Modif::Descendre(i)))>"Descendre"</Bouton>
                                <Bouton taille=Taille::Compacte variante=Variante::Discret
                                    libelle_accessible=format!("{} {t}", if p.masque { "Afficher" } else { "Masquer" })
                                    on_click=Callback::new(move |_| modifier.run(Modif::Masque(p.panneau)))>
                                    {if p.masque { "Afficher" } else { "Masquer" }}
                                </Bouton>
                            </li>
                        }
                    }/>
            </ol>
            <p class="tdb-dispo__note">"Le bandeau Urgent ne se range pas et ne se masque jamais."</p>
            <div class="tdb-dispo__actions">
                <Bouton variante=Variante::Secondaire desactive=Signal::derive(move || !epinglee.get())
                    on_click=Callback::new(move |_| revenir_auto.run(()))>
                    "Revenir à automatique"
                </Bouton>
            </div>
            {move || moteur.get().a("assiduite.regle_gerer").then(|| view! {
                <div class="tdb-dispo__bascule">
                    <ChampTexte id="tdb-bascule" libelle="Heure de bascule de l'établissement" valeur=heure_saisie
                        genre="time" erreur=erreur_heure
                        aide="Avant cette heure, les absents passent en premier ; ensuite, les exclus et les appels non faits."/>
                    <Bouton variante=Variante::Secondaire on_click=regler>"Enregistrer l'heure"</Bouton>
                </div>
            })}
        </div>
    }
}

// ─── Dialogues d'action ───────────────────────────────────────────────────────────────────

fn options_avec_invite(liste: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut v = vec![(String::new(), "Choisir…".to_string())];
    v.extend(
        liste
            .iter()
            .map(|(c, l)| ((*c).to_string(), (*l).to_string())),
    );
    v
}

/// Boîte « un choix de la liste fermée, puis confirmer ». Le titre est statique (règle du design system).
#[component]
fn DialogueChoix(
    id: &'static str,
    titre: &'static str,
    ouvert: RwSignal<bool>,
    #[prop(into)] consigne: Signal<String>,
    libelle: &'static str,
    options: Vec<(String, String)>,
    confirmer: &'static str,
    danger: bool,
    on_confirmer: Callback<String>,
) -> impl IntoView {
    let variante_confirmer = if danger {
        Variante::Danger
    } else {
        Variante::Primaire
    };
    let valeur = RwSignal::new(String::new());
    Effect::new(move |_| {
        if !ouvert.get() {
            valeur.set(String::new());
        }
    });
    view! {
        <Dialogue ouvert=ouvert titre=titre id=id>
            <p>{move || consigne.get()}</p>
            <Selection id=format!("{id}-choix") libelle=libelle valeur=valeur options=options requis=true/>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                <Bouton variante=variante_confirmer
                    desactive=Signal::derive(move || valeur.get().is_empty())
                    on_click=Callback::new(move |_| {
                        let v = valeur.get_untracked();
                        if !v.is_empty() {
                            ouvert.set(false);
                            on_confirmer.run(v);
                        }
                    })>
                    {confirmer}
                </Bouton>
            </div>
        </Dialogue>
    }
}

#[component]
fn Dialogues(
    cible: RwSignal<Option<Cible>>,
    ctx: Ctx,
    ouvre_contact: RwSignal<bool>,
    ouvre_traiter: RwSignal<bool>,
    ouvre_prevenir: RwSignal<bool>,
    ouvre_a_la_place: RwSignal<bool>,
) -> impl IntoView {
    let ecriture = ctx.ecriture;
    let jour = ctx.jour;
    let confirmer_contact = Callback::new(move |issue: String| {
        if let Some(Cible::Contact {
            eleve_id,
            absence_id,
            ..
        }) = cible.get_untracked()
        {
            ecriture.lancer(async move {
                session::client()
                    .tracer_contact(&TracerContactBody {
                        eleve_id,
                        absence_id,
                        canal: "telephone".into(),
                        issue,
                    })
                    .await?;
                Ok("Contact tracé : toute l'équipe le voit.".to_string())
            });
        }
    });
    let confirmer_traiter = Callback::new(move |code: String| {
        if let Some(Cible::Traiter { alerte_id, .. }) = cible.get_untracked() {
            ecriture.lancer(async move {
                session::client().alerte_traiter(alerte_id, &code).await?;
                Ok("Alerte traitée.".to_string())
            });
        }
    });
    let confirmer_prevenir = Callback::new(move |raison: String| {
        if let Some(Cible::Prevenir { eleve_id, .. }) = cible.get_untracked() {
            ecriture.lancer(async move {
                let r = session::client().prevenir_maintenant(eleve_id, &raison).await?;
                Ok(if r.deja_fait {
                    "Déjà fait aujourd'hui : aucun second avis n'est envoyé.".to_string()
                } else if r.sans_destinataire {
                    "Aucun responsable joignable par l'ENT : appelez la famille (la ligne reste dans la file).".to_string()
                } else {
                    "Avis urgent envoyé aux responsables.".to_string()
                })
            });
        }
    });
    let confirmer_place = Callback::new(move |raison: String| {
        if let Some(Cible::ALaPlace { creneau_id, .. }) = cible.get_untracked() {
            let Some(j) = jour.get_untracked() else {
                return;
            };
            ecriture.lancer(async move {
                session::client()
                    .appel_a_la_place(creneau_id, j, &raison)
                    .await?;
                Ok(
                    "Appel ouvert à la place de l'intervenant : faites-le depuis la page Appel."
                        .to_string(),
                )
            });
        }
    });
    let nom_de = move || match cible.get() {
        Some(
            Cible::Contact { nom, .. } | Cible::Traiter { nom, .. } | Cible::Prevenir { nom, .. },
        ) => nom,
        Some(Cible::ALaPlace { titre, .. }) => titre,
        None => String::new(),
    };
    view! {
        <DialogueChoix id="tdb-dlg-contact" titre="Tracer un contact avec la famille" ouvert=ouvre_contact
            consigne=Signal::derive(move || format!("{} : quelle est l'issue de votre appel téléphonique ? Toute l'équipe le voit, la famille jamais.", nom_de()))
            libelle="Issue du contact" options=options_avec_invite(&ISSUES_CONTACT)
            confirmer="Tracer le contact" danger=false on_confirmer=confirmer_contact/>
        <DialogueChoix id="tdb-dlg-traiter" titre="Traiter l'alerte" ouvert=ouvre_traiter
            consigne=Signal::derive(move || format!("{} : comment l'alerte se termine-t-elle ?", nom_de()))
            libelle="Issue de l'alerte" options=options_avec_invite(&ISSUES_ALERTE)
            confirmer="Clore l'alerte" danger=false on_confirmer=confirmer_traiter/>
        <DialogueChoix id="tdb-dlg-prevenir" titre="Prévenir maintenant la famille" ouvert=ouvre_prevenir
            consigne=Signal::derive(move || format!("Un avis URGENT part immédiatement aux responsables de {}, sans délai de confirmation. La décision est tracée à votre nom.", nom_de()))
            libelle="Raison" options=options_avec_invite(&RAISONS_PREVENIR)
            confirmer="Envoyer l'avis urgent" danger=true on_confirmer=confirmer_prevenir/>
        <DialogueChoix id="tdb-dlg-place" titre="Faire l'appel à la place de l'intervenant" ouvert=ouvre_a_la_place
            consigne=Signal::derive(move || format!("{} : l'appel s'ouvre à votre nom, la substitution est tracée.", nom_de()))
            libelle="Raison" options=options_avec_invite(&RAISONS_A_LA_PLACE)
            confirmer="Ouvrir l'appel" danger=false on_confirmer=confirmer_place/>
    }
}
