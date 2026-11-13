//! Mon agenda (socle 19, lot AG1) : la semaine de la personne connectée, toutes sources (événements et
//! réunions, rendez-vous, cours, fermetures), avec le détail d'un événement, la réponse à une
//! invitation, la création d'un événement et sa publication à une audience.
//!
//! Aucune règle métier ici : le serveur décide des droits, de la place du lecteur (R-AG5), de
//! l'audience sous les droits de l'auteur (R-AG8) et du fuseau de l'établissement ; les sources
//! absentes sont **dites** (R-AG1). La date du jour n'est posée qu'après l'hydratation.

pub mod modele;
pub mod salles;

use chrono::NaiveDate;
use ent_types::agenda::{
    AnnulationEvenement, DemandePublication, ElementAgenda, EvenementDetail, ReponseCoupon,
    ReponseInvitation, VueAgenda,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    classes_bouton, Alerte, Annonce, Badge, Bouton, Case, ChampTexte, Dialogue, EnTetePage,
    EtatVide, Icone, Niveau, NomIcone, Selection, Squelette, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::coquille::{use_droits, use_droits_moteur};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use modele::{
    composer_audience, composer_creneau_semaine, composer_evenement, est_annule, horaire,
    libelle_reponse, libelle_semaine, libelle_source, libelle_sources_indisponibles,
    libelle_statut, lundi_de, peut_repondre, regrouper_par_jour, resume_apercu, semaine_voisine,
    ton_reponse, ton_statut, SaisieEvenement, AUDIENCES, SOURCES,
};

type Donnees = Result<Option<VueAgenda>, ApiError>;

#[component]
pub fn PageMonAgenda() -> impl IntoView {
    let droits = use_droits();
    let moteur = use_droits_moteur();
    // La date du jour n'est posée qu'après l'hydratation (jamais dans le HTML du serveur).
    let lundi = RwSignal::new(None::<NaiveDate>);
    Effect::new(move |_| lundi.set(Some(lundi_de(aujourdhui()))));
    let coches: Vec<(&'static str, &'static str, RwSignal<bool>)> = SOURCES
        .iter()
        .map(|(c, l)| (*c, *l, RwSignal::new(true)))
        .collect();
    let coches = StoredValue::new(coches);
    let rechargement = RwSignal::new(0u32);
    let donnees: LocalResource<Donnees> = LocalResource::new(move || {
        let l = lundi.get();
        let _ = rechargement.get();
        let choisies: Vec<&'static str> = coches.with_value(|c| {
            c.iter()
                .filter(|(_, _, on)| on.get())
                .map(|(code, _, _)| *code)
                .collect()
        });
        async move {
            match l {
                None => Ok(None),
                Some(l) if choisies.is_empty() => Ok(Some(VueAgenda {
                    du: l,
                    au: l,
                    elements: vec![],
                    sources_indisponibles: vec![],
                })),
                Some(l) => session::client()
                    .agenda_vue(l, modele::dimanche_de(l), &choisies, None)
                    .await
                    .map(Some),
            }
        }
    });
    let decaler = move |d: i64| {
        lundi.update(|l| {
            if let Some(n) = l.and_then(|x| semaine_voisine(x, d)) {
                *l = Some(n);
            }
        });
    };
    let choisi = RwSignal::new(None::<Uuid>);
    let detail_ouvert = RwSignal::new(false);
    let nouveau_ouvert = RwSignal::new(false);
    let recharger = Callback::new(move |_| rechargement.update(|n| *n += 1));
    let ouvrir = Callback::new(move |id: Uuid| {
        choisi.set(Some(id));
        detail_ouvert.set(true);
    });
    let titre_semaine = Signal::derive(move || {
        lundi
            .get()
            .map(libelle_semaine)
            .unwrap_or_else(|| "Semaine".to_string())
    });
    view! {
        <Title text="Mon agenda"/>
        <EnTetePage
            surtitre="Agenda".to_string()
            titre="Mon agenda".to_string()
            sous_titre="Vos réunions, rendez-vous, cours et fermetures de la semaine.".to_string()
        />
        <Annonce message=titre_semaine/>
        <nav class="agd-navigation" aria-label="Changer de semaine">
            <Bouton
                variante=Variante::Secondaire
                taille=Taille::Compacte
                libelle_accessible="Semaine précédente"
                on_click=Callback::new(move |_| decaler(-1))
            >
                <Icone nom=NomIcone::ChevronGauche petite=true/>
            </Bouton>
            <Bouton
                variante=Variante::Secondaire
                taille=Taille::Compacte
                on_click=Callback::new(move |_| lundi.set(Some(lundi_de(aujourdhui()))))
            >
                "Aujourd'hui"
            </Bouton>
            <Bouton
                variante=Variante::Secondaire
                taille=Taille::Compacte
                libelle_accessible="Semaine suivante"
                on_click=Callback::new(move |_| decaler(1))
            >
                <Icone nom=NomIcone::ChevronDroite petite=true/>
            </Bouton>
            <h2 class="agd-semaine-titre">{move || titre_semaine.get()}</h2>
            {move || {
                droits.get().personnel.then(|| view! {
                    <a class=classes_bouton(Variante::Secondaire, Taille::Compacte) href="/web/salles">
                        "Réserver une salle"
                    </a>
                    <Bouton
                        variante=Variante::Primaire
                        taille=Taille::Compacte
                        on_click=Callback::new(move |_| nouveau_ouvert.set(true))
                    >
                        "Nouvel événement"
                    </Bouton>
                })
            }}
        </nav>
        <fieldset class="agd-sources">
            <legend>"Afficher"</legend>
            {coches
                .get_value()
                .into_iter()
                .map(|(code, libelle, on)| {
                    view! { <Case id=format!("agd-source-{code}") libelle=libelle coche=on/> }
                })
                .collect_view()}
        </fieldset>
        {move || match donnees.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement de l'agenda…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| donnees.refetch())/>
            }
            .into_any(),
            Some(Ok(None)) => view! { <Squelette lignes=5 libelle="Chargement de l'agenda…"/> }.into_any(),
            Some(Ok(Some(vue))) => {
                let l = lundi.get_untracked().unwrap_or(vue.du);
                view! { <SemaineAgenda vue=vue lundi=l ouvrir=ouvrir/> }.into_any()
            }
        }}
        {move || {
            moteur.get().a("agenda.semaine.gerer").then(|| view! { <FormulaireSemaine recharger=recharger/> })
        }}
        <Dialogue ouvert=detail_ouvert titre="Événement" id="agd-detail">
            {move || {
                choisi.get().map(|id| view! {
                    <DetailEvenement id=id recharger=recharger fermer=detail_ouvert/>
                })
            }}
        </Dialogue>
        <Dialogue ouvert=nouveau_ouvert titre="Nouvel événement" id="agd-nouveau">
            <FormulaireEvenement
                ouvert=nouveau_ouvert
                recharger=recharger
                ouvrir=ouvrir
            />
        </Dialogue>
    }
}

#[component]
fn SemaineAgenda(vue: VueAgenda, lundi: NaiveDate, ouvrir: Callback<Uuid>) -> impl IntoView {
    let auj = aujourdhui();
    let absentes = libelle_sources_indisponibles(&vue.sources_indisponibles);
    let jours = regrouper_par_jour(&vue, lundi);
    view! {
        {absentes.map(|m| view! { <Alerte niveau=Niveau::Info titre=m/> })}
        <div class="agd-semaine">
            {jours
                .into_iter()
                .map(|j| {
                    let id = format!("agd-jour-{}", j.jour);
                    let titre_id = id.clone();
                    let est_auj = j.jour == auj;
                    let jour = j.jour;
                    view! {
                        <section class="agd-jour" class:agd-jour--aujourdhui=est_auj aria-labelledby=titre_id>
                            <h3 id=id class="agd-jour__titre">
                                {j.titre.clone()}
                                {est_auj.then(|| view! { <Badge libelle="Aujourd'hui" ton=TonEtat::Info/> })}
                            </h3>
                            {if j.elements.is_empty() {
                                view! { <p class="texte-attenue">"Rien de prévu."</p> }.into_any()
                            } else {
                                view! {
                                    <ul class="agd-elements">
                                        {j.elements
                                            .into_iter()
                                            .map(|e| view! { <LigneElement e=e jour=jour ouvrir=ouvrir/> })
                                            .collect_view()}
                                    </ul>
                                }
                                .into_any()
                            }}
                        </section>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn LigneElement(e: ElementAgenda, jour: NaiveDate, ouvrir: Callback<Uuid>) -> impl IntoView {
    let annule = est_annule(&e);
    let source = libelle_source(&e.source);
    let statut = libelle_statut(&e.statut).map(|l| (l, ton_statut(&e.statut)));
    let reponse = e.reponse.clone().filter(|r| r == "en_attente");
    let contenu = view! {
        <span class="agd-element__heure">{horaire(&e, jour)}</span>
        <span class="agd-element__titre">{e.titre.clone()}</span>
        {e.lieu.clone().map(|l| view! { <span class="agd-element__lieu texte-attenue">{l}</span> })}
        <span class="agd-element__badges">
            <Badge libelle=source/>
            {statut.map(|(l, t)| view! { <Badge libelle=l ton=t/> })}
            {reponse.map(|r| view! { <Badge libelle=libelle_reponse(&r) ton=ton_reponse(&r)/> })}
        </span>
    };
    match (e.source.as_str(), e.id) {
        ("evenements", Some(id)) => view! {
            <li class="agd-element" class:agd-element--annule=annule>
                <button
                    type="button"
                    class="agd-element__ouvrir"
                    aria-label=format!("Ouvrir : {}", e.titre)
                    on:click=move |_| ouvrir.run(id)
                >
                    {contenu}
                </button>
            </li>
        }
        .into_any(),
        _ => view! { <li class="agd-element" class:agd-element--annule=annule>{contenu}</li> }
            .into_any(),
    }
}

// ---------------------------------------------------------------------------
// Détail d'un événement
// ---------------------------------------------------------------------------

#[component]
fn DetailEvenement(id: Uuid, recharger: Callback<()>, fermer: RwSignal<bool>) -> impl IntoView {
    let detail =
        LocalResource::new(move || async move { session::client().agenda_evenement(id).await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        detail.refetch();
        recharger.run(());
    }));
    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || match detail.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de l'événement…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| detail.refetch())/>
            }
            .into_any(),
            Some(Ok(d)) => view! { <CorpsEvenement d=d ecriture=ecriture fermer=fermer/> }.into_any(),
        }}
    }
}

fn libelle_plage(d: &EvenementDetail) -> String {
    let h = |t: chrono::NaiveTime| t.format("%H:%M").to_string();
    match (d.heure_debut, d.heure_fin) {
        (Some(a), Some(b)) if d.jour == d.jour_fin => {
            format!(
                "{} · {}–{}",
                crate::noyau::temps::date_longue(d.jour),
                h(a),
                h(b)
            )
        }
        (Some(a), Some(b)) => format!(
            "Du {} à {} au {} à {}",
            crate::noyau::temps::date_longue(d.jour),
            h(a),
            crate::noyau::temps::date_longue(d.jour_fin),
            h(b)
        ),
        _ if d.jour_fin > d.jour => format!(
            "Du {} au {}",
            crate::noyau::temps::date_longue(d.jour),
            crate::noyau::temps::date_longue(d.jour_fin)
        ),
        _ => format!(
            "{} · journée entière",
            crate::noyau::temps::date_longue(d.jour)
        ),
    }
}

#[component]
fn CorpsEvenement(d: EvenementDetail, ecriture: Ecriture, fermer: RwSignal<bool>) -> impl IntoView {
    let id = d.id;
    let reponse = RwSignal::new(String::new());
    let motif = RwSignal::new(String::new());
    let peut = peut_repondre(&d.statut, d.ma_reponse.as_deref());
    let ics = format!("/agenda/evenements/{id}/ics");
    let statut = libelle_statut(&d.statut).map(|l| (l, ton_statut(&d.statut)));
    let coupon = d.coupon.clone();
    let organisateur = d.je_suis_organisateur;
    let brouillon = d.statut == "brouillon";
    let actif = matches!(d.statut.as_str(), "brouillon" | "publie");
    view! {
        <article class="agd-detail" aria-labelledby="agd-detail-titre">
            <h2 id="agd-detail-titre">{d.titre.clone()}</h2>
            <p>
                {statut.map(|(l, t)| view! { <Badge libelle=l ton=t/> })}
                {" "}
                {d.ma_reponse.clone().map(|r| view! { <Badge libelle=libelle_reponse(&r) ton=ton_reponse(&r)/> })}
            </p>
            <p>{libelle_plage(&d)}</p>
            {d.lieu.clone().map(|l| view! { <p>{format!("Lieu : {l}")}</p> })}
            {d.organisateur_nom.clone().map(|n| view! { <p class="texte-attenue">{format!("Organisé par {n}")}</p> })}
            {d.description.clone().map(|t| view! { <p>{t}</p> })}
            {d.motif_annulation.clone().map(|m| view! {
                <Alerte niveau=Niveau::Erreur titre=format!("Annulé — {m}")/>
            })}
            {peut.then(|| view! {
                <form
                    class="agd-reponse"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        let r = reponse.get_untracked();
                        if r.is_empty() {
                            return;
                        }
                        let m = motif.get_untracked();
                        ecriture.lancer(async move {
                            session::client()
                                .agenda_repondre(id, &ReponseInvitation {
                                    reponse: r,
                                    motif_excuse: (!m.trim().is_empty()).then_some(m),
                                })
                                .await?;
                            Ok("Réponse enregistrée.".to_string())
                        });
                    }
                >
                    <Selection
                        id="agd-reponse"
                        libelle="Ma réponse"
                        valeur=reponse
                        options=vec![
                            (String::new(), "— Choisir —".to_string()),
                            ("oui".to_string(), "Je participe".to_string()),
                            ("peut_etre".to_string(), "Peut-être".to_string()),
                            ("non".to_string(), "Je ne participe pas".to_string()),
                        ]
                    />
                    <ChampTexte id="agd-motif" libelle="Motif (facultatif)" valeur=motif/>
                    <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>
                        "Enregistrer ma réponse"
                    </Bouton>
                </form>
            })}
            {coupon.map(|c| view! { <CouponEvenement id=id c=c actif=actif ecriture=ecriture/> })}
            <p class="agd-actions">
                <a class=classes_bouton(Variante::Secondaire, Taille::Compacte) href=ics download="evenement.ics">
                    "Ajouter à mon agenda (iCal)"
                </a>
            </p>
            {organisateur.then(|| view! { <GestionOrganisateur d=d.clone() ecriture=ecriture brouillon=brouillon actif=actif/> })}
            <p>
                <Bouton variante=Variante::Discret on_click=Callback::new(move |_| fermer.set(false))>
                    "Fermer"
                </Bouton>
            </p>
        </article>
    }
}

#[component]
fn CouponEvenement(
    id: Uuid,
    c: ent_types::agenda::CouponDto,
    actif: bool,
    ecriture: Ecriture,
) -> impl IntoView {
    let mon = c.mon_choix;
    let (question, choix, decomptes) = (c.question, c.choix, c.decompte);
    view! {
        <section class="agd-coupon" aria-labelledby="agd-coupon-q">
            <h3 id="agd-coupon-q">{question}</h3>
            <ul class="agd-coupon__choix">
                {choix
                    .into_iter()
                    .enumerate()
                    .map(|(i, libelle)| {
                        let n = (i + 1) as i16;
                        let decompte = decomptes.as_ref().and_then(|d| d.get(i).copied());
                        view! {
                            <li>
                                <Bouton
                                    variante=if mon == Some(n) { Variante::Primaire } else { Variante::Secondaire }
                                    taille=Taille::Compacte
                                    desactive=!actif
                                    en_cours=ecriture.en_cours
                                    on_click=Callback::new(move |_| {
                                        ecriture.lancer(async move {
                                            session::client().agenda_repondre_coupon(id, &ReponseCoupon { choix: n }).await?;
                                            Ok("Réponse enregistrée.".to_string())
                                        });
                                    })
                                >
                                    {libelle}
                                </Bouton>
                                {decompte.map(|n| view! { <span class="texte-attenue">{format!(" {n} réponse(s)")}</span> })}
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </section>
    }
}

#[component]
fn GestionOrganisateur(
    d: EvenementDetail,
    ecriture: Ecriture,
    brouillon: bool,
    actif: bool,
) -> impl IntoView {
    let id = d.id;
    let participants = d.participants.clone().unwrap_or_default();
    view! {
        <section class="agd-organisateur" aria-labelledby="agd-orga">
            <h3 id="agd-orga">"Gestion de l'événement"</h3>
            <p class="texte-attenue">
                {format!(
                    "Réponses : {} oui, {} peut-être, {} non, {} en attente.",
                    d.nb_oui, d.nb_peut_etre, d.nb_non, d.nb_en_attente
                )}
            </p>
            {(!participants.is_empty()).then(|| view! {
                <details>
                    <summary>{format!("{} participants", participants.len())}</summary>
                    <ul class="agd-participants">
                        {participants
                            .into_iter()
                            .map(|p| view! {
                                <li>
                                    {p.nom.unwrap_or_else(|| "(sans nom)".to_string())}
                                    {" — "}
                                    {libelle_reponse(&p.reponse)}
                                    {p.motif_excuse.map(|m| format!(" ({m})"))}
                                </li>
                            })
                            .collect_view()}
                    </ul>
                </details>
            })}
            {brouillon.then(|| view! { <PublicationEvenement id=id ecriture=ecriture/> })}
            {actif.then(|| view! { <AnnulationFormulaire id=id ecriture=ecriture/> })}
        </section>
    }
}

#[component]
fn PublicationEvenement(id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let audience = RwSignal::new("aucune".to_string());
    let classe = RwSignal::new(String::new());
    let classes = LocalResource::new(|| async { session::client().classes(aujourdhui()).await });
    let apercu = RwSignal::new(None::<(DemandePublication, String)>);
    let erreur = RwSignal::new(None::<String>);
    // Toute modification des choix invalide l'aperçu (on ne publie que ce qui a été prévisualisé).
    Effect::new(move |_| {
        audience.track();
        classe.track();
        apercu.set(None);
    });
    let options_audience: Vec<(String, String)> = AUDIENCES
        .iter()
        .map(|(c, l)| (c.to_string(), l.to_string()))
        .collect();
    view! {
        <form
            class="agd-publication"
            on:submit=move |ev| {
                ev.prevent_default();
                match composer_audience(&audience.get_untracked(), &classe.get_untracked()) {
                    Err(m) => erreur.set(Some(m)),
                    Ok(audiences) => {
                        erreur.set(None);
                        let demande = DemandePublication { audiences, empreinte: None, apercu_seulement: true, ..Default::default() };
                        let sortie = apercu;
                        ecriture.lancer(async move {
                            let a = session::client().agenda_apercu_publication(id, &demande).await?;
                            let texte = resume_apercu(&a);
                            sortie.set(Some((DemandePublication { empreinte: Some(a.empreinte), apercu_seulement: false, ..demande }, texte)));
                            Ok("Aperçu calculé.".to_string())
                        });
                    }
                }
            }
        >
            <h4>"Publier"</h4>
            <Selection id="agd-audience" libelle="Prévenir" valeur=audience options=options_audience/>
            {move || {
                (audience.get() == "classe").then(|| match classes.get() {
                    Some(Ok(l)) => {
                        let mut options = vec![(String::new(), "— Choisir une classe —".to_string())];
                        options.extend(l.into_iter().map(|c| (c.id.to_string(), c.libelle)));
                        view! { <Selection id="agd-classe" libelle="Classe" valeur=classe options=options/> }.into_any()
                    }
                    _ => view! { <Squelette lignes=1 libelle="Chargement des classes…"/> }.into_any(),
                })
            }}
            {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            <Bouton variante=Variante::Secondaire soumettre=true en_cours=ecriture.en_cours>
                "Voir l'aperçu"
            </Bouton>
            {move || {
                apercu.get().map(|(demande, texte)| view! {
                    <p role="status">{texte}</p>
                    <Bouton
                        variante=Variante::Primaire
                        en_cours=ecriture.en_cours
                        on_click=Callback::new(move |_| {
                            let d = demande.clone();
                            ecriture.lancer(async move {
                                session::client().agenda_publier(id, &d).await?;
                                Ok("Événement publié : les personnes concernées sont prévenues.".to_string())
                            });
                        })
                    >
                        "Confirmer et publier"
                    </Bouton>
                })
            }}
        </form>
    }
}

#[component]
fn AnnulationFormulaire(id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let motif = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    view! {
        <form
            class="agd-annulation"
            on:submit=move |ev| {
                ev.prevent_default();
                let m = motif.get_untracked();
                if m.trim().is_empty() {
                    erreur.set(Some("Indiquez le motif : il est communiqué aux participants.".into()));
                    return;
                }
                erreur.set(None);
                ecriture.lancer(async move {
                    session::client().agenda_annuler(id, &AnnulationEvenement { motif: m, portee: None }).await?;
                    Ok("Événement annulé : les participants sont prévenus.".to_string())
                });
            }
        >
            <h4>"Annuler l'événement"</h4>
            <ChampTexte id="agd-annuler-motif" libelle="Motif (communiqué aux participants)" valeur=motif requis=true/>
            {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            <Bouton variante=Variante::Danger soumettre=true en_cours=ecriture.en_cours>
                "Annuler l'événement"
            </Bouton>
        </form>
    }
}

// ---------------------------------------------------------------------------
// Nouvel événement
// ---------------------------------------------------------------------------

#[component]
fn FormulaireEvenement(
    ouvert: RwSignal<bool>,
    recharger: Callback<()>,
    ouvrir: Callback<Uuid>,
) -> impl IntoView {
    let type_code = RwSignal::new("reunion".to_string());
    let titre = RwSignal::new(String::new());
    let jour = RwSignal::new(String::new());
    let jour_fin = RwSignal::new(String::new());
    let h_debut = RwSignal::new("17:00".to_string());
    let h_fin = RwSignal::new("18:00".to_string());
    let lieu = RwSignal::new(String::new());
    let confidentialite = RwSignal::new("participants".to_string());
    let erreur = RwSignal::new(None::<String>);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| recharger.run(())));
    let options_type: Vec<(String, String)> = ent_types::agenda::TYPES_INTEGRES
        .iter()
        .map(|(c, l, _)| (c.to_string(), l.to_string()))
        .collect();
    view! {
        <RetourEcriture ecriture=ecriture/>
        <form
            on:submit=move |ev| {
                ev.prevent_default();
                let (t, ti, j, jf, hd, hf, l, c) = (
                    type_code.get_untracked(),
                    titre.get_untracked(),
                    jour.get_untracked(),
                    jour_fin.get_untracked(),
                    h_debut.get_untracked(),
                    h_fin.get_untracked(),
                    lieu.get_untracked(),
                    confidentialite.get_untracked(),
                );
                match composer_evenement(&SaisieEvenement {
                    type_code: &t,
                    titre: &ti,
                    jour: &j,
                    jour_fin: &jf,
                    heure_debut: &hd,
                    heure_fin: &hf,
                    lieu: &l,
                    confidentialite: &c,
                }) {
                    Err(m) => erreur.set(Some(m)),
                    Ok(n) => {
                        erreur.set(None);
                        ecriture.lancer(async move {
                            let c = session::client().agenda_creer_evenement(&n).await?;
                            ouvert.set(false);
                            ouvrir.run(c.id);
                            Ok("Brouillon créé : publiez-le depuis son détail.".to_string())
                        });
                    }
                }
            }
        >
            <Selection id="agd-type" libelle="Type" valeur=type_code options=options_type/>
            <ChampTexte id="agd-titre" libelle="Titre" valeur=titre requis=true/>
            <ChampTexte id="agd-jour" libelle="Jour" genre="date" valeur=jour requis=true/>
            <ChampTexte id="agd-jour-fin" libelle="Dernier jour (si plusieurs jours)" genre="date" valeur=jour_fin/>
            <ChampTexte id="agd-h-debut" libelle="Heure de début (vide : journée entière)" genre="time" valeur=h_debut/>
            <ChampTexte id="agd-h-fin" libelle="Heure de fin" genre="time" valeur=h_fin/>
            <ChampTexte id="agd-lieu" libelle="Lieu" valeur=lieu/>
            <Selection
                id="agd-confidentialite"
                libelle="Visibilité"
                valeur=confidentialite
                options=vec![
                    ("participants".to_string(), "Les participants seulement".to_string()),
                    ("etablissement".to_string(), "Tout l'établissement (partie publique)".to_string()),
                    ("restreint".to_string(), "Restreinte (titre jamais montré hors participants)".to_string()),
                ]
            />
            {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>
                "Créer le brouillon"
            </Bouton>
        </form>
    }
}

/// Feuille de semaine de l'établissement (direction) : un créneau visible de tous, sans avis.
#[component]
fn FormulaireSemaine(recharger: Callback<()>) -> impl IntoView {
    let titre = RwSignal::new(String::new());
    let jour = RwSignal::new(String::new());
    let h_debut = RwSignal::new("08:30".to_string());
    let h_fin = RwSignal::new("09:30".to_string());
    let lieu = RwSignal::new(String::new());
    let notes = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| recharger.run(())));
    view! {
        <details class="agd-semaine-saisie">
            <summary>"Feuille de semaine de l'établissement"</summary>
            <RetourEcriture ecriture=ecriture/>
            <form
                on:submit=move |ev| {
                    ev.prevent_default();
                    match composer_creneau_semaine(
                        &titre.get_untracked(),
                        &jour.get_untracked(),
                        &h_debut.get_untracked(),
                        &h_fin.get_untracked(),
                        &lieu.get_untracked(),
                        &notes.get_untracked(),
                    ) {
                        Err(m) => erreur.set(Some(m)),
                        Ok(c) => {
                            erreur.set(None);
                            ecriture.lancer(async move {
                                session::client().agenda_semaine_ajouter(&c).await?;
                                Ok("Créneau ajouté à la feuille de semaine.".to_string())
                            });
                        }
                    }
                }
            >
                <ChampTexte id="agd-sem-titre" libelle="Intitulé" valeur=titre requis=true/>
                <ChampTexte id="agd-sem-jour" libelle="Jour" genre="date" valeur=jour requis=true/>
                <ChampTexte id="agd-sem-debut" libelle="Début" genre="time" valeur=h_debut requis=true/>
                <ChampTexte id="agd-sem-fin" libelle="Fin" genre="time" valeur=h_fin requis=true/>
                <ChampTexte id="agd-sem-lieu" libelle="Lieu" valeur=lieu/>
                <ChampTexte id="agd-sem-notes" libelle="Notes (visibles de vous seul)" valeur=notes/>
                {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>
                    "Ajouter à la feuille"
                </Bouton>
            </form>
        </details>
    }
}

/// Évite l'avertissement « type inutilisé » quand la page est compilée sans rendu d'un composant.
#[allow(dead_code)]
fn _vide() -> impl IntoView {
    view! { <EtatVide titre="" message=""/> }
}
