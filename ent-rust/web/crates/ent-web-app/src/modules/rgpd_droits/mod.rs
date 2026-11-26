//! **Guichet des demandes de droits** (socle 27, lot G3a ; `27_RGPD_ARCHIVES.md` §9, §18.2 point 4).
//!
//! Trois écrans pour le guichet et deux pour la personne :
//!
//! | Écran | Adresse | Qui |
//! |---|---|---|
//! | File des demandes (échéances, retards) | `/web/rgpd/demandes` | `droits.traiter` |
//! | Saisie d'une demande reçue hors ligne | `/web/rgpd/demandes/saisir` | `droits.recevoir` |
//! | Fiche d'une demande (horloge, identité, droits, journal) | `/web/rgpd/demandes/:id` | `droits.traiter` |
//! | Mes données : suivi de mes demandes | `/web/mes-donnees` | toute personne connectée |
//! | Déposer une demande | `/web/mes-donnees/demande` | toute personne connectée |
//! | Demande sans compte (ancien élève, parent sans compte) | `/web/demande-droits` | **public**, hors coquille |
//!
//! Règles d'écran appliquées : la file ne porte **que des métadonnées** (R-RG32 : aucun nom, aucun texte de
//! périmètre) ; aucune page n'est bloquée sur l'absence d'un droit côté client (le serveur répond 403 et
//! `ErreurChargement` l'affiche, `FRONTEND_LEPTOS.md` §12) ; les boutons d'écriture restent masqués tant
//! que `/roles/mes-droits` n'a pas répondu ; aucune date du jour n'est rendue par le serveur (les échéances
//! viennent du serveur, qui donne aussi son jour de référence).

pub mod extrait;
pub mod modele;
pub mod modele_extrait;
pub mod modele_remise;
pub mod remise;

use ent_web_api::rgpd_droits_types::{
    AffectationBody, ArbitrageIdentiteBody, ClotureBody, DecisionBody, DemandeDto, DepotBody,
    DepotPublicBody, DroitBody, IdentiteBody, MotifBody, PrecisionIdentiteBody, ProlongationBody,
    QualificationBody, ReferentielGuichetDto, SaisieBody, SuiviDemandeDto, TableauDemandesDto,
    TitulairesBody,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, EnTetePage, EtatVide, FiltrePastille, GrilleBento, Largeur,
    LienBouton, Niveau, Recherche, Selection, Squelette, Statistique, Taille, TonEtat, Tuile,
    Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use modele::{
    decision_complete, depot_complet, depot_public_complet, filtrer, journal_recent_d_abord,
    libelle_canal, libelle_decision, libelle_droit, libelle_etat, libelle_evenement,
    libelle_mode_identite, libelle_qualification, libelle_qualite, libelle_qualite_constatee,
    motif_recevable, obstacle_cloture, phrase_echeance, piece_depuis_texte, teinte_echeance,
    trier_suivi, Filtre,
};

/// Seuil de repli quand le référentiel du guichet n'a pas (encore) répondu : la coloration « échéance
/// proche » suit S-544, dont la valeur réelle vient du serveur.
const SEUIL_PROCHE_REPLI: i64 = 10;

/// Vocabulaire de repli, le temps que `GET /rgpd/guichet/referentiel` réponde (il est invisible du serveur,
/// qui n'a pas la session). Il rend la **structure** de l'écran immédiate — règle de rendu de
/// `FRONTEND_LEPTOS.md` §3 — et reste strictement un sous-ensemble de ce que le serveur accepte : le
/// serveur refuse de toute façon une valeur qu'il ne connaît pas.
const CANAUX_REPLI: [&str; 5] = ["courrier", "courriel", "guichet", "dpo", "cnil"];
const QUALITES_REPLI: [&str; 9] = [
    "titulaire_autorite",
    "soi_meme",
    "eleve_mineur",
    "parent_sans_autorite",
    "representant_majeur_protege",
    "mandataire",
    "ancien_eleve",
    "personnel",
    // Un tiers ne dépose pas en ligne, mais il écrit : une demande CADA d'un tiers s'enregistre au
    // guichet (RGP-316) et se qualifie ensuite.
    "tiers",
];
const DROITS_REPLI: [&str; 7] = [
    "acces",
    "rectification",
    "effacement",
    "limitation",
    "opposition",
    "portabilite",
    "consultations",
];

/// Options d'un `<select>` : code technique en valeur, libellé lisible en texte.
fn options<S: AsRef<str>>(codes: &[S], libelle: fn(&str) -> &'static str) -> Vec<(String, String)> {
    codes
        .iter()
        .map(|c| {
            let c = c.as_ref();
            (c.to_string(), libelle(c).to_string())
        })
        .collect()
}

// ═══════════════════════════════════ File du guichet ═══════════════════════════════════

#[component]
pub fn PageGuichetDroits() -> impl IntoView {
    let moteur = use_droits_moteur();
    let closes = RwSignal::new(false);
    let file = LocalResource::new(move || {
        let avec_closes = closes.get();
        async move { session::client().rgpd_file(avec_closes).await }
    });
    let referentiel = LocalResource::new(|| async { session::client().rgpd_referentiel().await });
    view! {
        <Title text="Demandes de droits"/>
        <EnTetePage
            surtitre="Conformité".to_string()
            titre="Demandes de droits".to_string()
            sous_titre="Chaque demande a une échéance légale d'un mois à compter de sa réception.".to_string()
        >
            {move || moteur.get().peut_recevoir_demandes_droits().then(|| view! {
                <LienBouton href="/web/rgpd/demandes/saisir" variante=Variante::Primaire>
                    "Saisir une demande reçue"
                </LienBouton>
            })}
        </EnTetePage>
        {move || match file.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement des demandes…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| file.refetch())/>
            }.into_any(),
            Some(Ok(t)) => {
                let seuil = match referentiel.get() {
                    Some(Ok(r)) => r.alerte_echeance_jours.iter().copied().max().unwrap_or(SEUIL_PROCHE_REPLI),
                    _ => SEUIL_PROCHE_REPLI,
                };
                view! { <File tableau=t seuil=seuil closes=closes/> }.into_any()
            }
        }}
    }
}

#[component]
fn File(tableau: TableauDemandesDto, seuil: i64, closes: RwSignal<bool>) -> impl IntoView {
    let filtre = RwSignal::new(Filtre::Toutes);
    let recherche = RwSignal::new(String::new());
    let lignes = StoredValue::new(tableau.lignes.clone());
    let compteurs = (
        tableau.ouvertes,
        tableau.en_retard,
        tableau.echeance_proche,
        tableau.identite_a_verifier,
        tableau.a_qualifier,
        tableau.closes,
        tableau.identite_a_arbitrer,
        tableau.titulaires_a_decider,
    );
    let jour = tableau.jour;
    let ton_arbitrer = if compteurs.6 > 0 {
        TonEtat::Avertissement
    } else {
        TonEtat::Neutre
    };
    let ton_titulaires = if compteurs.7 > 0 {
        TonEtat::Avertissement
    } else {
        TonEtat::Neutre
    };
    let ton_retard = if compteurs.1 > 0 {
        TonEtat::Mal
    } else {
        TonEtat::Ok
    };
    let ton_proche = if compteurs.2 > 0 {
        TonEtat::Avertissement
    } else {
        TonEtat::Neutre
    };
    // Choisir « closes » recharge la file avec les demandes closes des douze derniers mois.
    let poser = move |f: Filtre| {
        filtre.set(f);
        closes.set(f == Filtre::Closes);
    };
    let visibles = Memo::new(move |_| {
        lignes.with_value(|l| filtrer(l, filtre.get(), &recherche.get(), seuil))
    });
    view! {
        <GrilleBento libelle="Compteurs des demandes".to_string()>
            <Tuile id="rgpd-compteurs".to_string() largeur=Largeur::Douze>
                <div class="rgpd-stats">
                    <Statistique libelle="Ouvertes".to_string()
                        valeur=Signal::derive(move || compteurs.0.to_string())/>
                    <Statistique libelle="En retard".to_string()
                        valeur=Signal::derive(move || compteurs.1.to_string())
                        ton=ton_retard/>
                    <Statistique libelle="Échéance proche".to_string()
                        valeur=Signal::derive(move || compteurs.2.to_string())
                        ton=ton_proche/>
                    <Statistique libelle="Identité à vérifier".to_string()
                        valeur=Signal::derive(move || compteurs.3.to_string())/>
                    <Statistique libelle="À qualifier".to_string()
                        valeur=Signal::derive(move || compteurs.4.to_string())/>
                    <Statistique libelle="Qualité à arbitrer".to_string()
                        valeur=Signal::derive(move || compteurs.6.to_string())
                        ton=ton_arbitrer/>
                    <Statistique libelle="Titulaires à décider".to_string()
                        valeur=Signal::derive(move || compteurs.7.to_string())
                        ton=ton_titulaires/>
                </div>
                <p class="texte-attenue">
                    {format!("Échéances calculées au {} (jour de l'établissement).", jour.format("%d/%m/%Y"))}
                </p>
            </Tuile>
        </GrilleBento>
        <div class="rgpd-filtres" role="group" aria-label="Filtrer les demandes">
            <FiltrePastille libelle="Toutes".to_string() teinte="neutre".to_string()
                nombre=Signal::derive(move || compteurs.0)
                actif=Signal::derive(move || filtre.get() == Filtre::Toutes)
                on_click=Callback::new(move |_| poser(Filtre::Toutes))/>
            <FiltrePastille libelle="En retard".to_string() teinte="mal".to_string()
                nombre=Signal::derive(move || compteurs.1)
                actif=Signal::derive(move || filtre.get() == Filtre::EnRetard)
                on_click=Callback::new(move |_| poser(Filtre::EnRetard))/>
            <FiltrePastille libelle="Échéance proche".to_string() teinte="avert".to_string()
                nombre=Signal::derive(move || compteurs.2)
                actif=Signal::derive(move || filtre.get() == Filtre::EcheanceProche)
                on_click=Callback::new(move |_| poser(Filtre::EcheanceProche))/>
            <FiltrePastille libelle="Identité à vérifier".to_string() teinte="info".to_string()
                nombre=Signal::derive(move || compteurs.3)
                actif=Signal::derive(move || filtre.get() == Filtre::IdentiteAVerifier)
                on_click=Callback::new(move |_| poser(Filtre::IdentiteAVerifier))/>
            <FiltrePastille libelle="À qualifier".to_string() teinte="info".to_string()
                nombre=Signal::derive(move || compteurs.4)
                actif=Signal::derive(move || filtre.get() == Filtre::AQualifier)
                on_click=Callback::new(move |_| poser(Filtre::AQualifier))/>
            <FiltrePastille libelle="Closes".to_string() teinte="neutre".to_string()
                nombre=Signal::derive(move || compteurs.5)
                actif=Signal::derive(move || filtre.get() == Filtre::Closes)
                on_click=Callback::new(move |_| poser(Filtre::Closes))/>
        </div>
        <Recherche id="rgpd-recherche".to_string() libelle="Chercher une référence".to_string()
            valeur=recherche
            placeholder="2027-014".to_string()/>
        {move || {
            let l = visibles.get();
            if l.is_empty() {
                view! {
                    <EtatVide
                        titre="Aucune demande"
                        message="Aucune demande ne correspond à ce filtre."
                    />
                }.into_any()
            } else {
                view! {
                    <table class="ui-tableau">
                        <caption class="ui-sr">
                            "Demandes de droits : référence, arrivée, personne, droits, état, échéance"
                        </caption>
                        <thead>
                            <tr>
                                <th scope="col">"Référence"</th>
                                <th scope="col">"Arrivée"</th>
                                <th scope="col">"Qualité"</th>
                                <th scope="col">"Droits"</th>
                                <th scope="col">"État"</th>
                                <th scope="col">"Échéance"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {l.into_iter().map(|d| {
                                let href = format!("/web/rgpd/demandes/{}", d.id);
                                let droits = d.droits.iter().map(|c| libelle_droit(c)).collect::<Vec<_>>().join(", ");
                                let teinte = teinte_echeance(d.jours_restants, d.en_retard, seuil);
                                let ton = match teinte {
                                    "mal" => TonEtat::Mal,
                                    "avert" => TonEtat::Avertissement,
                                    _ => TonEtat::Neutre,
                                };
                                let echeance = format!(
                                    "{} — {}",
                                    d.echeance_effective.format("%d/%m/%Y"),
                                    phrase_echeance(d.jours_restants, d.en_retard)
                                );
                                let arrivee = format!(
                                    "{} · {}",
                                    d.recue_le.format("%d/%m/%Y"),
                                    libelle_canal(&d.canal)
                                );
                                view! {
                                    <tr>
                                        <th scope="row">
                                            <a href=href>{d.reference.clone()}</a>
                                            {d.saisie_tardive.then(|| view! {
                                                <Badge libelle="Saisie tardive".to_string() ton=TonEtat::Avertissement/>
                                            })}
                                        </th>
                                        <td>{arrivee}</td>
                                        <td>{libelle_qualite(&d.qualite)}</td>
                                        <td>{droits}</td>
                                        <td>
                                            {libelle_etat(&d.etat)}
                                            {(d.qualification == "a_qualifier").then(|| view! {
                                                <Badge libelle="À qualifier".to_string() ton=TonEtat::Info/>
                                            })}
                                        </td>
                                        <td><Badge libelle=echeance ton=ton/></td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                }.into_any()
            }
        }}
    }
}

// ═══════════════════════════════════ Saisie d'un courrier ═══════════════════════════════════

#[component]
pub fn PageSaisirDemandeDroit() -> impl IntoView {
    let referentiel = LocalResource::new(|| async { session::client().rgpd_referentiel().await });
    view! {
        <Title text="Saisir une demande reçue"/>
        <EnTetePage
            surtitre="Conformité".to_string()
            titre="Saisir une demande reçue".to_string()
            sous_titre="La date de réception est celle du courrier : c'est elle qui fait courir le délai légal.".to_string()
        />
        // Le formulaire est rendu **tout de suite** (vocabulaire de repli) : le serveur n'a pas la session,
        // et une saisie de courrier ne doit pas attendre un aller-retour pour s'afficher.
        <FormulaireSaisie/>
        // La grille de vérification d'identité, elle, vient du serveur : elle s'ajoute quand il répond.
        {move || match referentiel.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement du guichet…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| referentiel.refetch())/>
            }.into_any(),
            Some(Ok(r)) => view! { <GrilleIdentite referentiel=r/> }.into_any(),
        }}
    }
}

#[component]
fn FormulaireSaisie() -> impl IntoView {
    let canal = RwSignal::new("courrier".to_string());
    let recue_le = RwSignal::new(String::new());
    let nom = RwSignal::new(String::new());
    let contact = RwSignal::new(String::new());
    let qualite = RwSignal::new("titulaire_autorite".to_string());
    let concernee = RwSignal::new(String::new());
    let eleve_id = RwSignal::new(String::new());
    let perimetre = RwSignal::new(String::new());
    let droits = RwSignal::new(vec!["acces".to_string()]);
    let cree = RwSignal::new(None::<Uuid>);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {}));

    let canaux = options(&CANAUX_REPLI, libelle_canal);
    let qualites = options(&QUALITES_REPLI, libelle_qualite);
    let liste_droits: Vec<String> = DROITS_REPLI.iter().map(|d| (*d).to_string()).collect();

    let envoyer = move |_| {
        let sujet = eleve_id.get().trim().to_string();
        let (sujet_type, sujet) = match Uuid::parse_str(&sujet) {
            Ok(u) => ("eleve".to_string(), Some(u)),
            Err(_) => ("inconnu".to_string(), None),
        };
        let b = SaisieBody {
            canal: canal.get(),
            recue_le: match chrono::NaiveDate::parse_from_str(&recue_le.get(), "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => {
                    ecriture
                        .erreur
                        .set(Some("Indiquez la date de réception du courrier.".into()));
                    return;
                }
            },
            demandeur_compte_id: None,
            demandeur_personne_id: None,
            demandeur_nom: Some(nom.get()),
            demandeur_contact: Some(contact.get()).filter(|c| !c.trim().is_empty()),
            qualite: qualite.get(),
            sujet_type,
            sujet_id: sujet,
            personne_concernee: Some(concernee.get()).filter(|c| !c.trim().is_empty()),
            droits: droits.get(),
            perimetre_texte: Some(perimetre.get()).filter(|p| !p.trim().is_empty()),
            perimetre_du: None,
            perimetre_au: None,
        };
        ecriture.lancer(async move {
            session::client().rgpd_saisir(&b).await.map(|d| {
                cree.set(Some(d.id));
                format!("Demande {} enregistrée.", d.reference)
            })
        });
    };

    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || cree.get().map(|id| {
            let href = format!("/web/rgpd/demandes/{id}");
            view! {
                <Alerte niveau=Niveau::Succes titre="Demande enregistrée".to_string()>
                    <p>"Vérifiez l'identité du demandeur avant toute réponse."</p>
                    <p><LienBouton href=href variante=Variante::Primaire>"Ouvrir la fiche"</LienBouton></p>
                </Alerte>
            }
        })}
        <section class="classe-section" aria-labelledby="rgpd-saisie">
            <h2 id="rgpd-saisie">"La demande"</h2>
            <Selection id="rgpd-canal".to_string() libelle="Comment la demande est arrivée".to_string()
                valeur=canal options=canaux requis=true/>
            <ChampTexte id="rgpd-recue".to_string() libelle="Date de réception (celle du courrier)".to_string()
                valeur=recue_le genre="date" requis=true
                aide="Le délai légal court à partir de cette date, pas de la saisie.".to_string()/>
            <ChampTexte id="rgpd-nom".to_string() libelle="Nom du demandeur".to_string()
                valeur=nom requis=true/>
            <ChampTexte id="rgpd-contact".to_string() libelle="Contact indiqué (facultatif)".to_string()
                valeur=contact
                aide="La réponse partira par un moyen déjà connu du dossier, jamais par celui-ci seul.".to_string()/>
            <Selection id="rgpd-qualite".to_string() libelle="À quel titre".to_string()
                valeur=qualite options=qualites requis=true/>
            <ChampTexte id="rgpd-eleve".to_string() libelle="Identifiant de l'élève concerné (si connu)".to_string()
                valeur=eleve_id
                aide="Laissez vide si la personne n'est pas (encore) rapprochée d'un dossier.".to_string()/>
            <ChampTexte id="rgpd-concernee".to_string() libelle="Personne concernée, telle qu'écrite".to_string()
                valeur=concernee/>
            <fieldset class="rgpd-choix">
                <legend>"Droits exercés"</legend>
                {liste_droits.into_iter().map(|code| {
                    let pour = code.clone();
                    let id = format!("rgpd-droit-{code}");
                    let pour_label = id.clone();
                    let coche = Signal::derive({
                        let c = code.clone();
                        move || droits.get().contains(&c)
                    });
                    view! {
                        <label class="ui-case" for=pour_label>
                            <input id=id type="checkbox"
                                prop:checked=move || coche.get()
                                on:change=move |_| droits.update(|v| {
                                    if let Some(i) = v.iter().position(|x| *x == pour) {
                                        v.remove(i);
                                    } else {
                                        v.push(pour.clone());
                                    }
                                })/>
                            <span>{libelle_droit(&code)}</span>
                        </label>
                    }
                }).collect_view()}
            </fieldset>
            <ZoneTexte id="rgpd-perimetre".to_string() libelle="Périmètre demandé, dans les mots du demandeur".to_string()
                valeur=perimetre/>
            <Bouton variante=Variante::Primaire
                desactive=Signal::derive(move || ecriture.en_cours.get() || droits.get().is_empty()
                    || nom.get().trim().len() < 2 || recue_le.get().is_empty())
                on_click=Callback::new(envoyer)>
                "Enregistrer la demande"
            </Bouton>
        </section>
    }
}

/// Grille de vérification d'identité (RGP-307), servie par le référentiel du guichet.
#[component]
fn GrilleIdentite(referentiel: ReferentielGuichetDto) -> impl IntoView {
    view! {
        <GrilleBento libelle="Vérification d'identité".to_string()>
            <Tuile id="rgpd-grille".to_string() titre="Avant de répondre".to_string() largeur=Largeur::Douze>
                <ol class="rgpd-grille">
                    {referentiel.grille_identite.into_iter().map(|e| view! {
                        <li>
                            <strong>{e.libelle}</strong>
                            {e.obligatoire.then(|| view! {
                                <Badge libelle="Obligatoire".to_string() ton=TonEtat::Avertissement/>
                            })}
                            <p class="texte-attenue">{e.geste}</p>
                        </li>
                    }).collect_view()}
                </ol>
            </Tuile>
        </GrilleBento>
    }
}

// ═══════════════════════════════════ Fiche d'une demande ═══════════════════════════════════

#[component]
pub fn PageDemandeDroit() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|s| Uuid::parse_str(&s).ok())
    };
    let version = RwSignal::new(0u32);
    let fiche = LocalResource::new(move || {
        let _ = version.get();
        async move {
            match id() {
                Some(i) => session::client().rgpd_demande(i).await,
                None => Err(ApiError::Introuvable("identifiant invalide".into())),
            }
        }
    });
    let referentiel = LocalResource::new(|| async { session::client().rgpd_referentiel().await });
    view! {
        <Title text="Demande de droit"/>
        {move || match fiche.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement de la demande…"/> }.into_any(),
            Some(Err(ApiError::Introuvable(_))) => view! {
                <EtatVide titre="Demande introuvable"
                    message="Elle n'existe pas, ou vous n'y avez pas accès.">
                    <LienBouton href="/web/rgpd/demandes" variante=Variante::Secondaire>
                        "Retour aux demandes"
                    </LienBouton>
                </EtatVide>
            }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| fiche.refetch())/>
            }.into_any(),
            Some(Ok(d)) => {
                let r = match referentiel.get() {
                    Some(Ok(r)) => Some(r),
                    _ => None,
                };
                view! { <Fiche d=d referentiel=r version=version/> }.into_any()
            }
        }}
    }
}

#[component]
fn Fiche(
    d: DemandeDto,
    referentiel: Option<ReferentielGuichetDto>,
    version: RwSignal<u32>,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let id = d.id;
    let ouverte = !matches!(d.etat.as_str(), "close" | "refusee");
    let obstacle = obstacle_cloture(&d);
    let journal = journal_recent_d_abord(&d.journal);
    let echeance = format!(
        "{} — {}",
        d.echeance_effective.format("%d/%m/%Y"),
        phrase_echeance(d.jours_restants, d.en_retard)
    );
    let ton_echeance = if d.en_retard {
        TonEtat::Mal
    } else {
        TonEtat::Neutre
    };
    // Tout ce que la vue affiche est calculé AVANT `view!` : les blocs de `view!` deviennent des
    // fermetures, et une lecture de champ à l'intérieur déplacerait `d` (piège de `FRONTEND_LEPTOS.md` §10).
    let pour_gestes = d.clone();
    let perimetre_texte = d.perimetre_texte.clone();
    let prolongation_motif = d.prolongation_motif.clone();
    let demandeur_nom = d
        .demandeur_nom
        .clone()
        .unwrap_or_else(|| "Compte de l'ENT".into());
    let personne_concernee =
        d.personne_concernee
            .clone()
            .unwrap_or_else(|| match d.sujet_type.as_str() {
                "eleve" => "Un élève du dossier".into(),
                "agent" => "Un agent".into(),
                "personne" => "Une personne du dossier".into(),
                _ => "Non rapprochée".into(),
            });
    let droits_vus = d.droits.clone();
    let identites_vues = d.identites.clone();
    let (reference, canal, etat, qualite, qualification) = (
        d.reference.clone(),
        d.canal.clone(),
        d.etat.clone(),
        d.qualite.clone(),
        d.qualification.clone(),
    );
    let (recue_le, echeance_le, prolongee_jusqu_au) =
        (d.recue_le, d.echeance_le, d.prolongee_jusqu_au);
    let (attente_jours, reouvertures) = (d.identite_attente_jours, d.reouvertures);
    let (saisie_tardive, en_retard, piece_a_detruire) =
        (d.saisie_tardive, d.en_retard, d.piece_a_detruire);
    let (destruction_en_attente, arbitrage_identite, titulaires_a_decider) = (
        d.destruction_en_attente,
        d.identite_arbitrage_code.is_some(),
        d.titulaires_a_decider,
    );
    let obstacle_vue = obstacle.clone();
    let obstacle_gestes = obstacle.clone();
    let referentiel_gestes = referentiel.clone();
    view! {
        <Title text="Demande de droit"/>
        <EnTetePage
            surtitre="Conformité".to_string()
            titre=format!("Demande {reference}")
            sous_titre=format!(
                "{} · reçue le {} · {}",
                libelle_canal(&canal),
                recue_le.format("%d/%m/%Y"),
                libelle_etat(&etat)
            )
            compacte=true
        >
            <LienBouton href="/web/rgpd/demandes" variante=Variante::Secondaire>"Toutes les demandes"</LienBouton>
        </EnTetePage>
        <RetourEcriture ecriture=ecriture/>
        {saisie_tardive.then(|| view! {
            <Alerte niveau=Niveau::Avertissement
                titre="Cette demande a été saisie bien après sa réception".to_string()>
                <p>"Le délai légal court depuis la réception : il est déjà entamé. Relevez le courrier plus souvent."</p>
            </Alerte>
        })}
        {en_retard.then(|| view! {
            <Alerte niveau=Niveau::Erreur titre="Échéance dépassée".to_string()>
                <p>"Répondez sans attendre. Le retard reste visible : il ne se réécrit pas."</p>
            </Alerte>
        })}
        {(etat == "identite_a_verifier").then(|| view! {
            <Alerte niveau=Niveau::Info titre="Identité à établir".to_string()>
                <p>"Reprenez contact par un moyen déjà connu du dossier, jamais par celui que la demande indique."</p>
            </Alerte>
        })}
        {(piece_a_detruire && !destruction_en_attente).then(|| view! {
            <Alerte niveau=Niveau::Avertissement titre="Une pièce d'identité est détenue".to_string()>
                <p>"Sa destruction sera demandée à la clôture ; seule la mention « vérifiée le, par » restera."</p>
            </Alerte>
        })}
        {destruction_en_attente.then(|| view! {
            <Alerte niveau=Niveau::Avertissement titre="Destruction de la pièce en attente".to_string()>
                <p>"La demande est close, mais la pièce d'identité n'est pas encore détruite. Elle est réessayée \
                    automatiquement ; elle ne sera dite « détruite » qu'une fois confirmée."</p>
            </Alerte>
        })}
        {arbitrage_identite.then(|| view! {
            <Alerte niveau=Niveau::Avertissement titre="Qualité du demandeur à arbitrer".to_string()>
                <p>"La personne est identifiée, mais sa qualité n'est pas compatible avec celle qu'elle invoque. \
                    Rien ne peut lui être accordé avant l'arbitrage de la direction."</p>
            </Alerte>
        })}
        {titulaires_a_decider.then(|| view! {
            <Alerte niveau=Niveau::Info titre="Titulaires de l'autorité parentale : décision à prendre".to_string()>
                <p>"L'élève a déposé pour lui-même : décidez d'informer — ou non — ses titulaires avant de clore."</p>
            </Alerte>
        })}

        <GrilleBento libelle="La demande".to_string()>
            <Tuile id="rgpd-horloge".to_string() titre="Horloge".to_string() largeur=Largeur::Six>
                <dl class="rgpd-definitions">
                    <dt>"Reçue le"</dt><dd>{recue_le.format("%d/%m/%Y").to_string()}</dd>
                    <dt>"Échéance légale"</dt><dd>{echeance_le.format("%d/%m/%Y").to_string()}</dd>
                    {prolongee_jusqu_au.map(|p| view! {
                        <dt>"Prolongée jusqu'au"</dt>
                        <dd>{p.format("%d/%m/%Y").to_string()}</dd>
                    })}
                    <dt>"Échéance qui s'applique"</dt>
                    <dd><Badge libelle=echeance ton=ton_echeance/></dd>
                    {(attente_jours > 0).then(|| view! {
                        <dt>"Attente d'une précision d'identité"</dt>
                        <dd>{format!("{attente_jours} jour(s) — le délai légal continue de courir")}</dd>
                    })}
                    {(reouvertures > 0).then(|| view! {
                        <dt>"Réouvertures"</dt>
                        <dd>{reouvertures.to_string()}</dd>
                    })}
                </dl>
                {prolongation_motif.map(|m| view! {
                    <p class="texte-attenue">{format!("Motif de la prolongation : {m}")}</p>
                })}
            </Tuile>
            <Tuile id="rgpd-demandeur".to_string() titre="Demandeur".to_string() largeur=Largeur::Six>
                <dl class="rgpd-definitions">
                    <dt>"Nom"</dt>
                    <dd>{demandeur_nom}</dd>
                    <dt>"À quel titre"</dt><dd>{libelle_qualite(&qualite)}</dd>
                    <dt>"Personne concernée"</dt>
                    <dd>{personne_concernee}</dd>
                    <dt>"Qualification"</dt><dd>{libelle_qualification(&qualification)}</dd>
                </dl>
                {perimetre_texte.map(|p| view! {
                    <p>{format!("Périmètre demandé : {p}")}</p>
                })}
            </Tuile>
        </GrilleBento>

        <extrait::SectionExtrait demande=id ouverte=ouverte/>
        <remise::SectionReponseRemise demande=id ouverte=ouverte/>

        <section class="classe-section" aria-labelledby="rgpd-droits-titre">
            <h2 id="rgpd-droits-titre">"Droits exercés"</h2>
            <ul class="ui-liste">
                {droits_vus.iter().map(|x| {
                    let ton = match x.decision.as_str() {
                        "accorde" => TonEtat::Ok,
                        "refuse" => TonEtat::Mal,
                        "partiel" => TonEtat::Avertissement,
                        "en_examen" => TonEtat::Info,
                        _ => TonEtat::Neutre,
                    };
                    view! {
                        <li class="ui-liste__element">
                            <div class="ui-liste__principal">
                                <span class="ui-liste__titre">{libelle_droit(&x.droit)}</span>
                                <span class="ui-liste__detail">
                                    {x.motif_texte.clone().unwrap_or_default()}
                                </span>
                            </div>
                            <Badge libelle=libelle_decision(&x.decision).to_string() ton=ton/>
                        </li>
                    }
                }).collect_view()}
            </ul>
            {obstacle_vue.map(|m| view! { <p class="texte-attenue">{m}</p> })}
        </section>

        <section class="classe-section" aria-labelledby="rgpd-identite-titre">
            <h2 id="rgpd-identite-titre">"Identité et qualité"</h2>
            {if identites_vues.is_empty() {
                view! { <EtatVide titre="Aucune vérification enregistrée"/> }.into_any()
            } else {
                view! {
                    <ul class="ui-liste">
                        {identites_vues.iter().map(|i| {
                            let quand = i.verifie_le
                                .map(|t| t.format("%d/%m/%Y").to_string())
                                .unwrap_or_else(|| "en attente".into());
                            let detail = i.qualite_constatee.as_deref()
                                .map(|q| libelle_qualite_constatee(q).to_string())
                                .unwrap_or_default();
                            view! {
                                <li class="ui-liste__element">
                                    <div class="ui-liste__principal">
                                        <span class="ui-liste__titre">{libelle_mode_identite(&i.mode)}</span>
                                        <span class="ui-liste__detail">{format!("{quand} · {detail}")}</span>
                                    </div>
                                    {(i.piece_detenue && !i.destruction_en_attente).then(|| view! {
                                        <Badge libelle="Pièce détenue".to_string() ton=TonEtat::Avertissement/>
                                    })}
                                    {i.destruction_en_attente.then(|| view! {
                                        <Badge libelle="Destruction en attente".to_string() ton=TonEtat::Avertissement/>
                                    })}
                                    {(i.resultat == "a_arbitrer").then(|| view! {
                                        <Badge libelle="À arbitrer".to_string() ton=TonEtat::Avertissement/>
                                    })}
                                    {i.piece_detruite_le.map(|_| view! {
                                        <Badge libelle="Pièce détruite".to_string() ton=TonEtat::Ok/>
                                    })}
                                    {(i.resultat == "echouee").then(|| view! {
                                        <Badge libelle="Échec".to_string() ton=TonEtat::Mal/>
                                    })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>

        {move || (ouverte && moteur.get().peut_traiter_demandes_droits())
            .then(|| view! {
                <Gestes d=pour_gestes.clone() referentiel=referentiel_gestes.clone() ecriture=ecriture
                    obstacle=obstacle_gestes.clone() id=id/>
            })}
        {move || (!ouverte && moteur.get().peut_arbitrer_demandes_droits())
            .then(|| view! { <Reouverture ecriture=ecriture id=id/> })}

        <section class="classe-section" aria-labelledby="rgpd-journal-titre">
            <h2 id="rgpd-journal-titre">"Journal"</h2>
            <ol class="rgpd-journal">
                {journal.into_iter().map(|l| view! {
                    <li>
                        <span class="rgpd-journal__quand">{l.le.format("%d/%m/%Y %H:%M").to_string()}</span>
                        <span class="rgpd-journal__quoi">{libelle_evenement(&l.evenement)}</span>
                        {l.detail.map(|t| view! { <span class="texte-attenue">{t}</span> })}
                        {l.par_role.map(|r| view! { <span class="texte-attenue">{format!("({r})")}</span> })}
                    </li>
                }).collect_view()}
            </ol>
        </section>
    }
}

/// Les gestes possibles sur une demande ouverte. Chacun est un formulaire court : le serveur reste seul
/// juge (403 si le droit manque, 409 si l'état ne le permet pas).
#[component]
fn Gestes(
    d: DemandeDto,
    referentiel: Option<ReferentielGuichetDto>,
    ecriture: Ecriture,
    obstacle: Option<String>,
    id: Uuid,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let arbitrage_identite = d.identite_arbitrage_code.is_some();
    // Qualification.
    let qualification = RwSignal::new(if d.qualification == "a_qualifier" {
        "rgpd".to_string()
    } else {
        d.qualification.clone()
    });
    let motif_qualif = RwSignal::new(String::new());
    // Identité.
    let mode = RwSignal::new("code_courriel".to_string());
    let note_identite = RwSignal::new(String::new());
    let piece_ref = RwSignal::new(String::new());
    let motif_arbitrage = RwSignal::new(String::new());
    let precision = RwSignal::new(String::new());
    // Décision.
    let droit_decide = RwSignal::new(
        d.droits
            .iter()
            .find(|x| x.decision == "en_examen")
            .map(|x| x.droit.clone())
            .unwrap_or_else(|| "acces".to_string()),
    );
    let decision = RwSignal::new("accorde".to_string());
    let motif_code = RwSignal::new(String::new());
    let motif_texte = RwSignal::new(String::new());
    // Prolongation, clôture, refus.
    let jusqu_au = RwSignal::new(String::new());
    let motif_prolongation = RwSignal::new(String::new());
    let resume = RwSignal::new(String::new());
    let motif_refus = RwSignal::new(String::new());
    // Droit à ajouter.
    let droit_ajout = RwSignal::new("rectification".to_string());
    // Titulaires (élève mineur).
    let motif_titulaires = RwSignal::new(String::new());

    let droits_disponibles = referentiel
        .as_ref()
        .map(|r| r.droits.clone())
        .unwrap_or_else(|| vec!["acces".into()]);
    let modeles = referentiel
        .as_ref()
        .map(|r| r.modeles_reponse.clone())
        .unwrap_or_default();
    let droits_de_la_demande: Vec<String> = d.droits.iter().map(|x| x.droit.clone()).collect();
    let est_eleve_mineur = d.qualite == "eleve_mineur";
    let titulaires_decides =
        d.titulaires_informes_le.is_some() || d.titulaires_non_informes_motif.is_some();

    view! {
        <section class="classe-section" aria-labelledby="rgpd-gestes">
            <h2 id="rgpd-gestes">"Instruire"</h2>

            <article class="rgpd-carte">
                <h3>"Qualifier"</h3>
                <Selection id="rgpd-g-qualif".to_string() libelle="Nature de la demande".to_string()
                    valeur=qualification
                    options=vec![
                        ("rgpd".to_string(), "RGPD".to_string()),
                        ("cada".to_string(), "Document administratif (CADA)".to_string()),
                        ("document".to_string(), "Simple demande de document".to_string()),
                    ]/>
                <ChampTexte id="rgpd-g-qualif-motif".to_string() libelle="Motif de la qualification".to_string()
                    valeur=motif_qualif requis=true/>
                <Bouton variante=Variante::Secondaire
                    desactive=Signal::derive(move || ecriture.en_cours.get() || !motif_recevable(&motif_qualif.get()))
                    on_click=Callback::new(move |_| {
                        let b = QualificationBody { qualification: qualification.get(), motif: motif_qualif.get() };
                        ecriture.lancer(async move {
                            session::client().rgpd_qualifier(id, &b).await
                                .map(|_| "Qualification enregistrée.".to_string())
                        });
                    })>
                    "Qualifier"
                </Bouton>
            </article>

            {moteur.get().peut_recevoir_demandes_droits().then(|| view! {
                <article class="rgpd-carte">
                    <h3>"Identité et qualité"</h3>
                    <Selection id="rgpd-g-mode".to_string() libelle="Comment l'identité a été établie".to_string()
                        valeur=mode
                        options=vec![
                            ("code_courriel".to_string(), "Code envoyé au courriel du dossier".to_string()),
                            ("piece".to_string(), "Pièce d'identité consultée".to_string()),
                            ("mandat".to_string(), "Mandat".to_string()),
                            ("session_forte".to_string(), "Connexion de la personne".to_string()),
                        ]/>
                    <ChampTexte id="rgpd-g-note".to_string() libelle="Note de vérification (facultative)".to_string()
                        valeur=note_identite/>
                    <ChampTexte id="rgpd-g-piece".to_string()
                        libelle="Référence de la pièce d'identité (mode « pièce »)".to_string()
                        valeur=piece_ref
                        aide="Identifiant de la pièce déposée au coffre de l'établissement (type « pièce d'identité »).".to_string()/>
                    <p class="texte-attenue">
                        "Une pièce enregistrée ici est détruite à la clôture ; seule la mention de la vérification reste."
                    </p>
                    <Bouton variante=Variante::Secondaire
                        desactive=Signal::derive(move || ecriture.en_cours.get())
                        on_click=Callback::new(move |_| {
                            let Ok(piece) = piece_depuis_texte(&piece_ref.get()) else {
                                ecriture.erreur.set(Some("La référence de la pièce n'est pas un identifiant valide.".into()));
                                return;
                            };
                            let b = IdentiteBody {
                                mode: mode.get(),
                                reussie: true,
                                note: Some(note_identite.get()).filter(|n| n.trim().len() >= 3),
                                piece_document_id: piece.filter(|_| mode.get() == "piece"),
                            };
                            ecriture.lancer(async move {
                                session::client().rgpd_verifier_identite(id, &b).await
                                    .map(|_| "Identité vérifiée.".to_string())
                            });
                        })>
                        "Identité établie"
                    </Bouton>
                    <Bouton variante=Variante::Secondaire
                        desactive=Signal::derive(move || ecriture.en_cours.get())
                        on_click=Callback::new(move |_| {
                            let b = IdentiteBody {
                                mode: mode.get(),
                                reussie: false,
                                note: Some(note_identite.get()).filter(|n| n.trim().len() >= 3),
                                piece_document_id: None,
                            };
                            ecriture.lancer(async move {
                                session::client().rgpd_verifier_identite(id, &b).await
                                    .map(|_| "Échec de vérification enregistré.".to_string())
                            });
                        })>
                        "Identité non établie"
                    </Bouton>
                    <ChampTexte id="rgpd-g-precision".to_string()
                        libelle="Demander une précision d'identité".to_string()
                        valeur=precision
                        aide="Le délai légal continue de courir : l'attente est seulement comptée.".to_string()/>
                    <Bouton variante=Variante::Secondaire
                        desactive=Signal::derive(move || ecriture.en_cours.get() || !motif_recevable(&precision.get()))
                        on_click=Callback::new(move |_| {
                            let b = PrecisionIdentiteBody { demande: precision.get() };
                            ecriture.lancer(async move {
                                session::client().rgpd_demander_identite(id, &b).await
                                    .map(|_| "Précision demandée.".to_string())
                            });
                        })>
                        "Demander la précision"
                    </Bouton>
                </article>
            })}

            {(arbitrage_identite && moteur.get().peut_arbitrer_demandes_droits()).then(|| view! {
                <article class="rgpd-carte">
                    <h3>"Arbitrer la qualité du demandeur"</h3>
                    <p>
                        "Reconnaître la qualité invoquée suppose de l'avoir établie (jugement, mandat, pièce vue) ; \
                         le motif est conservé. Le refuser laisse la demande bloquée : refusez-la ensuite avec son motif."
                    </p>
                    <ChampTexte id="rgpd-g-arbitrage".to_string() libelle="Motif de l'arbitrage".to_string()
                        valeur=motif_arbitrage requis=true/>
                    <Bouton variante=Variante::Secondaire
                        desactive=Signal::derive(move || ecriture.en_cours.get() || !motif_recevable(&motif_arbitrage.get()))
                        on_click=Callback::new(move |_| {
                            let b = ArbitrageIdentiteBody { accepte: true, motif: motif_arbitrage.get() };
                            ecriture.lancer(async move {
                                session::client().rgpd_arbitrer_identite(id, &b).await
                                    .map(|_| "Qualité reconnue.".to_string())
                            });
                        })>
                        "Reconnaître la qualité"
                    </Bouton>
                    <Bouton variante=Variante::Secondaire
                        desactive=Signal::derive(move || ecriture.en_cours.get() || !motif_recevable(&motif_arbitrage.get()))
                        on_click=Callback::new(move |_| {
                            let b = ArbitrageIdentiteBody { accepte: false, motif: motif_arbitrage.get() };
                            ecriture.lancer(async move {
                                session::client().rgpd_arbitrer_identite(id, &b).await
                                    .map(|_| "Qualité non reconnue.".to_string())
                            });
                        })>
                        "Ne pas la reconnaître"
                    </Bouton>
                </article>
            })}

            {est_eleve_mineur.then(|| view! {
                <article class="rgpd-carte">
                    <h3>"Titulaires de l'autorité parentale"</h3>
                    <p>
                        "L'élève a déposé pour lui-même. Les titulaires sont informés, sauf motif sérieux \
                         (sécurité de l'élève) : cette décision appartient à la direction et reste tracée."
                    </p>
                    {titulaires_decides.then(|| view! {
                        <p class="texte-attenue">"Une décision est déjà enregistrée ; elle peut être revue."</p>
                    })}
                    <Bouton variante=Variante::Secondaire
                        desactive=Signal::derive(move || ecriture.en_cours.get())
                        on_click=Callback::new(move |_| {
                            let b = TitulairesBody { informer: true, motif: None };
                            ecriture.lancer(async move {
                                session::client().rgpd_titulaires(id, &b).await
                                    .map(|_| "Titulaires informés.".to_string())
                            });
                        })>
                        "Informer les titulaires"
                    </Bouton>
                    {moteur.get().peut_arbitrer_demandes_droits().then(|| view! {
                        <>
                            <ChampTexte id="rgpd-g-titulaires".to_string()
                                libelle="Motif sérieux pour ne pas informer".to_string()
                                valeur=motif_titulaires/>
                            <Bouton variante=Variante::Secondaire
                                desactive=Signal::derive(move || ecriture.en_cours.get()
                                    || !motif_recevable(&motif_titulaires.get()))
                                on_click=Callback::new(move |_| {
                                    let b = TitulairesBody {
                                        informer: false,
                                        motif: Some(motif_titulaires.get()),
                                    };
                                    ecriture.lancer(async move {
                                        session::client().rgpd_titulaires(id, &b).await
                                            .map(|_| "Décision de ne pas informer enregistrée.".to_string())
                                    });
                                })>
                                "Ne pas informer (motivé)"
                            </Bouton>
                        </>
                    })}
                </article>
            })}

            <article class="rgpd-carte">
                <h3>"Décider d'un droit"</h3>
                <Selection id="rgpd-g-droit".to_string() libelle="Droit".to_string()
                    valeur=droit_decide
                    options=options(&droits_de_la_demande, libelle_droit)/>
                <Selection id="rgpd-g-decision".to_string() libelle="Décision".to_string()
                    valeur=decision
                    options=vec![
                        ("accorde".to_string(), "Accordé".to_string()),
                        ("partiel".to_string(), "Partiel".to_string()),
                        ("refuse".to_string(), "Refusé".to_string()),
                        ("sans_objet".to_string(), "Sans objet".to_string()),
                    ]/>
                <ChampTexte id="rgpd-g-motif-code".to_string() libelle="Article invoqué".to_string()
                    valeur=motif_code
                    aide="Obligatoire pour un refus ou un partiel (exemple : art17_3_b).".to_string()/>
                <ZoneTexte id="rgpd-g-motif-texte".to_string() libelle="Réponse motivée".to_string()
                    valeur=motif_texte/>
                {(!modeles.is_empty()).then(|| view! {
                    <details class="rgpd-replie">
                        <summary>"Modèles de réponse"</summary>
                        <ul class="ui-liste">
                            {modeles.into_iter().map(|m| {
                                let (code, texte, libelle) = (m.motif_code.clone(), m.texte.clone(), m.libelle.clone());
                                view! {
                                    <li class="ui-liste__element">
                                        <div class="ui-liste__principal">
                                            <span class="ui-liste__titre">{libelle}</span>
                                            <span class="ui-liste__detail">{m.texte.clone()}</span>
                                        </div>
                                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                            on_click=Callback::new(move |_| {
                                                motif_code.set(code.clone());
                                                motif_texte.set(texte.clone());
                                            })>
                                            "Utiliser"
                                        </Bouton>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    </details>
                })}
                <Bouton variante=Variante::Primaire
                    desactive=Signal::derive(move || ecriture.en_cours.get()
                        || !decision_complete(&decision.get(), &motif_code.get(), &motif_texte.get()))
                    on_click=Callback::new(move |_| {
                        let b = DecisionBody {
                            droit: droit_decide.get(),
                            decision: decision.get(),
                            motif_code: Some(motif_code.get()).filter(|c| !c.trim().is_empty()),
                            motif_texte: Some(motif_texte.get()).filter(|t| t.trim().len() >= 3),
                        };
                        ecriture.lancer(async move {
                            session::client().rgpd_decider(id, &b).await
                                .map(|_| "Décision enregistrée.".to_string())
                        });
                    })>
                    "Enregistrer la décision"
                </Bouton>
            </article>

            <article class="rgpd-carte">
                <h3>"Ajouter un droit"</h3>
                <Selection id="rgpd-g-ajout".to_string() libelle="Droit à ajouter".to_string()
                    valeur=droit_ajout
                    options=options(&droits_disponibles, libelle_droit)/>
                <Bouton variante=Variante::Secondaire
                    desactive=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(move |_| {
                        let b = DroitBody { droit: droit_ajout.get() };
                        ecriture.lancer(async move {
                            session::client().rgpd_ajouter_droit(id, &b).await
                                .map(|_| "Droit ajouté.".to_string())
                        });
                    })>
                    "Ajouter"
                </Bouton>
            </article>

            <article class="rgpd-carte">
                <h3>"Affectation"</h3>
                <Bouton variante=Variante::Secondaire
                    desactive=Signal::derive(move || ecriture.en_cours.get())
                    on_click=Callback::new(move |_| {
                        let b = AffectationBody { gestionnaire_personne_id: None };
                        ecriture.lancer(async move {
                            session::client().rgpd_affecter(id, &b).await
                                .map(|_| "Affectation retirée.".to_string())
                        });
                    })>
                    "Retirer l'affectation"
                </Bouton>
            </article>

            {moteur.get().peut_arbitrer_demandes_droits().then(|| view! {
                <article class="rgpd-carte">
                    <h3>"Arbitrages de la direction"</h3>
                    <ChampTexte id="rgpd-g-jusqu".to_string() libelle="Prolonger jusqu'au".to_string()
                        valeur=jusqu_au genre="date"
                        aide="Deux mois au plus, et seulement avant l'échéance du premier mois (art. 12.3).".to_string()/>
                    <ChampTexte id="rgpd-g-prolong-motif".to_string() libelle="Motif de la prolongation".to_string()
                        valeur=motif_prolongation/>
                    <Bouton variante=Variante::Secondaire
                        desactive=Signal::derive(move || ecriture.en_cours.get()
                            || jusqu_au.get().is_empty() || !motif_recevable(&motif_prolongation.get()))
                        on_click=Callback::new(move |_| {
                            let Ok(j) = chrono::NaiveDate::parse_from_str(&jusqu_au.get(), "%Y-%m-%d") else {
                                ecriture.erreur.set(Some("Date de prolongation invalide.".into()));
                                return;
                            };
                            let b = ProlongationBody { jusqu_au: j, motif: motif_prolongation.get() };
                            ecriture.lancer(async move {
                                session::client().rgpd_prolonger(id, &b).await
                                    .map(|_| "Délai prolongé et demandeur informé.".to_string())
                            });
                        })>
                        "Prolonger"
                    </Bouton>
                    <ZoneTexte id="rgpd-g-refus".to_string()
                        libelle="Refuser la demande (motif obligatoire)".to_string()
                        valeur=motif_refus
                        aide="Un refus pour demande manifestement excessive est une décision motivée, jamais automatique.".to_string()/>
                    <Bouton variante=Variante::Secondaire
                        desactive=Signal::derive(move || ecriture.en_cours.get() || !motif_recevable(&motif_refus.get()))
                        on_click=Callback::new(move |_| {
                            let b = MotifBody { motif: motif_refus.get() };
                            ecriture.lancer(async move {
                                session::client().rgpd_refuser(id, &b).await
                                    .map(|_| "Demande refusée, motif enregistré.".to_string())
                            });
                        })>
                        "Refuser la demande"
                    </Bouton>
                </article>
            })}

            <article class="rgpd-carte">
                <h3>"Clôturer"</h3>
                {obstacle.clone().map(|m| view! {
                    <Alerte niveau=Niveau::Info titre=m/>
                })}
                <ZoneTexte id="rgpd-g-resume".to_string() libelle="Résumé de la réponse remise".to_string()
                    valeur=resume requis=true/>
                <Bouton variante=Variante::Primaire
                    desactive=Signal::derive(move || ecriture.en_cours.get()
                        || !motif_recevable(&resume.get()) || obstacle.is_some())
                    on_click=Callback::new(move |_| {
                        let b = ClotureBody { reponse_resume: resume.get() };
                        ecriture.lancer(async move {
                            session::client().rgpd_clore(id, &b).await
                                .map(|d| if d.destruction_en_attente {
                                    "Demande close. La pièce d'identité sera détruite dès que le stockage répond (réessai automatique).".to_string()
                                } else {
                                    "Demande close.".to_string()
                                })
                        });
                    })>
                    "Clôturer la demande"
                </Bouton>
            </article>
        </section>
    }
}

#[component]
fn Reouverture(ecriture: Ecriture, id: Uuid) -> impl IntoView {
    let motif = RwSignal::new(String::new());
    view! {
        <section class="classe-section" aria-labelledby="rgpd-reouverture">
            <h2 id="rgpd-reouverture">"Rouvrir"</h2>
            <p>
                "Une réclamation rouvre la demande. L'horloge ne repart pas à zéro : l'échéance d'origine \
                 reste la référence, et le retard éventuel reste visible."
            </p>
            <ChampTexte id="rgpd-reouverture-motif".to_string() libelle="Motif de la réouverture".to_string()
                valeur=motif requis=true/>
            <Bouton variante=Variante::Secondaire
                desactive=Signal::derive(move || ecriture.en_cours.get() || !motif_recevable(&motif.get()))
                on_click=Callback::new(move |_| {
                    let b = MotifBody { motif: motif.get() };
                    ecriture.lancer(async move {
                        session::client().rgpd_rouvrir(id, &b).await
                            .map(|_| "Demande rouverte.".to_string())
                    });
                })>
                "Rouvrir la demande"
            </Bouton>
        </section>
    }
}

// ═══════════════════════════════════ Côté personne ═══════════════════════════════════

/// « Mes données » : suivi des demandes de la personne connectée (RGP-155).
#[component]
pub fn PageMesDonnees() -> impl IntoView {
    let mes = LocalResource::new(|| async { session::client().rgpd_mes_demandes().await });
    view! {
        <Title text="Mes données"/>
        <EnTetePage
            titre="Mes données".to_string()
            sous_titre="Vos demandes et leur avancement. L'établissement répond dans un délai d'un mois.".to_string()
        >
            <LienBouton href="/web/mes-donnees/demande" variante=Variante::Primaire>
                "Déposer une demande"
            </LienBouton>
        </EnTetePage>
        {move || match mes.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de vos demandes…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| mes.refetch())/>
            }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune demande"
                    message="Vous n'avez déposé aucune demande concernant vos données."/>
            }.into_any(),
            Some(Ok(l)) => view! { <Suivi demandes=trier_suivi(l)/> }.into_any(),
        }}
        <remise::MesExtraits/>
    }
}

#[component]
fn Suivi(demandes: Vec<SuiviDemandeDto>) -> impl IntoView {
    view! {
        <ul class="ui-liste">
            {demandes.into_iter().map(|s| {
                let droits = s.droits.iter().map(|c| libelle_droit(c)).collect::<Vec<_>>().join(", ");
                let ton = if s.close_le.is_some() { TonEtat::Neutre } else { TonEtat::Info };
                view! {
                    <li class="ui-liste__element">
                        <div class="ui-liste__principal">
                            <span class="ui-liste__titre">{format!("Demande {}", s.reference)}</span>
                            <span class="ui-liste__detail">{s.avancement.clone()}</span>
                            <span class="ui-liste__detail">{droits}</span>
                            {s.prolongation_motif.clone().map(|m| view! {
                                <span class="ui-liste__detail">{format!("Délai prolongé : {m}")}</span>
                            })}
                            {s.reponse_resume.clone().map(|r| view! {
                                <span class="ui-liste__detail">{format!("Réponse : {r}")}</span>
                            })}
                        </div>
                        <Badge libelle=libelle_etat(&s.etat).to_string() ton=ton/>
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}

/// Dépôt d'une demande par la personne connectée (RGP-140).
#[component]
pub fn PageDeposerDemandeDroit() -> impl IntoView {
    let qualite = RwSignal::new("soi_meme".to_string());
    let eleve_id = RwSignal::new(String::new());
    let droits = RwSignal::new(vec!["acces".to_string()]);
    let perimetre = RwSignal::new(String::new());
    let deposee = RwSignal::new(None::<String>);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {}));
    let tous_les_droits = [
        "acces",
        "rectification",
        "effacement",
        "limitation",
        "opposition",
        "consultations",
    ];
    let envoyer = move |_| {
        let sujet = eleve_id.get().trim().to_string();
        let sujet_id = Uuid::parse_str(&sujet).ok();
        let q = qualite.get();
        if !depot_complet(&q, sujet_id.is_some(), &droits.get()) {
            ecriture.erreur.set(Some(
                "Choisissez au moins un droit, et l'élève concerné si vous demandez pour votre enfant."
                    .into(),
            ));
            return;
        }
        let b = DepotBody {
            qualite: q,
            sujet_type: if sujet_id.is_some() {
                "eleve".to_string()
            } else {
                "personne".to_string()
            },
            sujet_id,
            droits: droits.get(),
            perimetre_texte: Some(perimetre.get()).filter(|p| !p.trim().is_empty()),
            perimetre_du: None,
            perimetre_au: None,
        };
        ecriture.lancer(async move {
            session::client().rgpd_deposer(&b).await.map(|d| {
                deposee.set(Some(d.reference.clone()));
                format!("Demande {} déposée.", d.reference)
            })
        });
    };
    view! {
        <Title text="Déposer une demande"/>
        <EnTetePage
            surtitre="Mes données".to_string()
            titre="Déposer une demande".to_string()
            sous_titre="L'établissement répond dans un délai d'un mois à compter de la réception.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || deposee.get().map(|r| view! {
            <Alerte niveau=Niveau::Succes titre=format!("Demande {r} déposée")>
                <p>"Vous recevrez un accusé de réception avec la date d'échéance et le contact du délégué à la protection des données."</p>
                <p><LienBouton href="/web/mes-donnees" variante=Variante::Primaire>"Suivre ma demande"</LienBouton></p>
            </Alerte>
        })}
        <section class="classe-section" aria-labelledby="rgpd-depot">
            <h2 id="rgpd-depot">"Votre demande"</h2>
            <Selection id="rgpd-d-qualite".to_string() libelle="À quel titre".to_string()
                valeur=qualite
                options=vec![
                    ("soi_meme".to_string(), "Pour moi-même".to_string()),
                    ("titulaire_autorite".to_string(), "Pour mon enfant (titulaire de l'autorité parentale)".to_string()),
                    ("eleve_mineur".to_string(), "Je suis l'élève".to_string()),
                    ("personnel".to_string(), "Je suis un personnel de l'établissement".to_string()),
                ]/>
            <ChampTexte id="rgpd-d-eleve".to_string()
                libelle="Identifiant de l'élève concerné (pour une demande sur votre enfant)".to_string()
                valeur=eleve_id
                aide="Inutile pour vos propres données : l'établissement retrouve votre dossier.".to_string()/>
            <fieldset class="rgpd-choix">
                <legend>"Ce que vous demandez"</legend>
                {tous_les_droits.into_iter().map(|code| {
                    let id = format!("rgpd-d-droit-{code}");
                    let pour_label = id.clone();
                    let coche = Signal::derive(move || droits.get().iter().any(|x| x == code));
                    view! {
                        <label class="ui-case" for=pour_label>
                            <input id=id type="checkbox"
                                prop:checked=move || coche.get()
                                on:change=move |_| droits.update(|v| {
                                    if let Some(i) = v.iter().position(|x| x == code) {
                                        v.remove(i);
                                    } else {
                                        v.push(code.to_string());
                                    }
                                })/>
                            <span>{libelle_droit(code)}</span>
                        </label>
                    }
                }).collect_view()}
            </fieldset>
            <ZoneTexte id="rgpd-d-perimetre".to_string()
                libelle="Précisez votre demande (période, domaine)".to_string()
                valeur=perimetre/>
            <p class="texte-attenue">
                "Les éléments qui concernent une autre personne sont retirés avant remise : la copie ne peut \
                 pas porter atteinte aux droits d'autrui."
            </p>
            <Bouton variante=Variante::Primaire
                desactive=Signal::derive(move || ecriture.en_cours.get() || droits.get().is_empty())
                on_click=Callback::new(envoyer)>
                "Déposer la demande"
            </Bouton>
        </section>
    }
}

// ═══════════════════════════ Page publique : dépôt sans compte (F7) ═══════════════════════════

/// Dépôt d'une demande **sans compte** (RGP-142) : ancien élève, parent sans compte. Page publique, hors
/// coquille, sans session. L'établissement vient de l'adresse du site ; la réponse est **invariable** : elle
/// ne dit jamais si l'établissement détient des données sur la personne (R-RG42, CL27-66).
#[component]
pub fn PageDemandeDroitsPublique() -> impl IntoView {
    let nom = RwSignal::new(String::new());
    let contact = RwSignal::new(String::new());
    let qualite = RwSignal::new("ancien_eleve".to_string());
    let personne = RwSignal::new(String::new());
    let droits = RwSignal::new(vec!["acces".to_string()]);
    let perimetre = RwSignal::new(String::new());
    let reponse = RwSignal::new(None::<String>);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {}));
    let tous_les_droits = [
        "acces",
        "rectification",
        "effacement",
        "limitation",
        "opposition",
    ];
    let envoyer = move |_| {
        if !depot_public_complet(&nom.get(), &contact.get(), &personne.get(), &droits.get()) {
            ecriture.erreur.set(Some(
                "Indiquez votre nom, un moyen de vous joindre, la personne concernée et au moins un droit."
                    .into(),
            ));
            return;
        }
        let b = DepotPublicBody {
            nom: nom.get(),
            contact: contact.get(),
            qualite: qualite.get(),
            personne_concernee: personne.get(),
            droits: droits.get(),
            perimetre_texte: Some(perimetre.get()).filter(|p| !p.trim().is_empty()),
        };
        ecriture.lancer(async move {
            session::client().rgpd_deposer_public(&b).await.map(|r| {
                reponse.set(Some(r.message));
                "Demande enregistrée.".to_string()
            })
        });
    };
    view! {
        <Title text="Exercer mes droits sur mes données"/>
        <EnTetePage
            titre="Exercer mes droits sur mes données".to_string()
            sous_titre="Pour les personnes qui n'ont pas de compte : ancien élève, parent sans compte.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || reponse.get().map(|m| view! {
            <Alerte niveau=Niveau::Succes titre="Demande enregistrée".to_string()>
                <p>{m}</p>
            </Alerte>
        })}
        <section class="classe-section" aria-labelledby="rgpd-public">
            <h2 id="rgpd-public">"Votre demande"</h2>
            <ChampTexte id="rgpd-p-nom".to_string() libelle="Votre nom".to_string() valeur=nom requis=true/>
            <ChampTexte id="rgpd-p-contact".to_string() libelle="Un moyen de vous joindre".to_string()
                valeur=contact requis=true
                aide="Il aide l'établissement à retrouver votre dossier. La réponse part par un moyen déjà connu de son dossier, ou vous est remise au guichet.".to_string()/>
            <Selection id="rgpd-p-qualite".to_string() libelle="À quel titre".to_string()
                valeur=qualite
                options=vec![
                    ("ancien_eleve".to_string(), "Je suis un ancien élève".to_string()),
                    ("soi_meme".to_string(), "Pour moi-même".to_string()),
                    ("titulaire_autorite".to_string(), "Pour mon enfant (titulaire de l'autorité parentale)".to_string()),
                ]/>
            <ChampTexte id="rgpd-p-personne".to_string() libelle="Nom de la personne concernée".to_string()
                valeur=personne requis=true/>
            <fieldset class="rgpd-choix">
                <legend>"Ce que vous demandez"</legend>
                {tous_les_droits.into_iter().map(|code| {
                    let id = format!("rgpd-p-droit-{code}");
                    let pour_label = id.clone();
                    let coche = Signal::derive(move || droits.get().iter().any(|x| x == code));
                    view! {
                        <label class="ui-case" for=pour_label>
                            <input id=id type="checkbox"
                                prop:checked=move || coche.get()
                                on:change=move |_| droits.update(|v| {
                                    if let Some(i) = v.iter().position(|x| x == code) {
                                        v.remove(i);
                                    } else {
                                        v.push(code.to_string());
                                    }
                                })/>
                            <span>{libelle_droit(code)}</span>
                        </label>
                    }
                }).collect_view()}
            </fieldset>
            <ZoneTexte id="rgpd-p-perimetre".to_string()
                libelle="Précisez votre demande (période, domaine)".to_string()
                valeur=perimetre/>
            <p class="texte-attenue">
                "Une pièce d'identité vous sera demandée avant toute remise ; elle est consultée puis détruite. \
                 L'établissement répond dans un délai d'un mois à compter de la réception."
            </p>
            <Bouton variante=Variante::Primaire
                desactive=Signal::derive(move || ecriture.en_cours.get() || droits.get().is_empty())
                on_click=Callback::new(envoyer)>
                "Envoyer la demande"
            </Bouton>
        </section>
    }
}
