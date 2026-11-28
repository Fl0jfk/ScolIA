//! Fiches reliées (transverse « individu », migration 279, `docs/socles/TRANSVERSE_INDIVIDU.md`).
//!
//! - **Mes autres comptes** (`/web/securite/mes-comptes`, tout compte) : ce que l'établissement
//!   reconnaît (nombres, liens datés, jamais l'autre fiche), les demandes reçues d'un autre compte à
//!   confirmer ou refuser, et la déclaration « cet autre compte est aussi moi » (réponse neutre).
//! - **Fiches reliées** (`/web/direction/fiches-reliees`, `personnes.individu.relier`) : propositions
//!   (ressemblances, jamais la coordonnée), relier deux fiches choisies, liens actifs, lever
//!   (`personnes.individu.lever`, direction, jamais par la personne concernée).
//!
//! Le serveur décide de tout ; un bouton d'écriture n'est montré que si le moteur de droits le
//! donne, et un refus (403, « levée par soi ») est affiché tel quel.

pub mod modele;

use ent_types::dossier::PersonneTrouveeDto;
use ent_types::individu::{
    DeclarationRecue, EcarterPropositionBody, LienIndividu, MonIndividu, PropositionIndividu,
    RelierFichesBody, MOTIFS_ECARTEE, MOTIFS_LEVEE, MOTIFS_LIEN,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Bouton, ChampTexte, Dialogue, EnTetePage, EtatVide, Niveau, Selection, Squelette,
    Taille, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::confirmation_detaillee::ConfirmationDetaillee;
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;
use modele::*;

// ───────────────────────── Mes autres comptes ─────────────────────────

#[component]
pub fn PageMesComptes() -> impl IntoView {
    let moi = LocalResource::new(|| async { session::client().individu_moi().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| moi.refetch()));
    view! {
        <Title text="Mes autres comptes"/>
        <EnTetePage
            surtitre="Compte".to_string()
            titre="Mes autres comptes".to_string()
            sous_titre="Vous avez plusieurs comptes ou plusieurs fiches dans l'établissement (personnel et parent, par exemple) ? \
                L'établissement peut les reconnaître comme une seule personne.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match moi.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de vos comptes…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| moi.refetch())/>
            }.into_any(),
            Some(Ok(m)) => view! { <EtatIndividu moi=m ecriture=ecriture/> }.into_any(),
        }}
        <Declarer ecriture=ecriture/>
        <section class="abs-section" aria-labelledby="ind-pourquoi">
            <h2 id="ind-pourquoi" class="vs-section-titre">"Pourquoi ?"</h2>
            <p>
                "Certaines décisions ne se prennent jamais pour soi-même ni pour son enfant : une aide accordée à son \
                 propre enfant, une seconde validation de sa propre demande, une absence que l'on validerait soi-même. \
                 Reconnaître vos comptes comme une seule personne empêche qu'un second compte contourne ces règles. \
                 Cela n'ouvre aucun accès supplémentaire, et vous êtes prévenu(e) de chaque lien posé ou levé."
            </p>
        </section>
    }
}

#[component]
fn EtatIndividu(moi: MonIndividu, ecriture: Ecriture) -> impl IntoView {
    let resume = resume_moi(&moi);
    let fiches = resume_fiches(&moi);
    let liens = moi.liens.clone();
    let recues = moi.declarations_recues.clone();
    view! {
        <section class="abs-section" aria-labelledby="ind-etat">
            <h2 id="ind-etat" class="vs-section-titre">"Ce que l'établissement reconnaît"</h2>
            <p>{resume}</p>
            {fiches.map(|f| view! { <p>{f}</p> })}
            {(!liens.is_empty()).then(|| view! {
                <ul class="ui-liste" aria-label="Liens entre vos fiches">
                    {liens.into_iter().map(|l| {
                        let quand = date_heure_paris(l.pose_le);
                        let origine = libelle_origine(&l.origine);
                        let motif = libelle_motif_lien(&l.motif_code);
                        view! {
                            <li class="ui-liste__element">
                                <div class="ui-liste__principal">
                                    <span class="ui-liste__titre">{origine}</span>
                                    <span class="ui-liste__detail">{format!("{motif} · depuis le {quand}")}</span>
                                </div>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            })}
            <p class="securite__aide">
                "Un lien vous semble faux ? Signalez-le au secrétariat : seule la direction peut le lever."
            </p>
        </section>
        <DemandesRecues recues=recues ecriture=ecriture/>
    }
}

#[component]
fn DemandesRecues(recues: Vec<DeclarationRecue>, ecriture: Ecriture) -> impl IntoView {
    let choisie = RwSignal::new(None::<(Uuid, String)>);
    let ouvert = RwSignal::new(false);
    let message = Signal::derive(move || {
        choisie
            .get()
            .map(|(_, d)| format!("Confirmez-vous que le compte « {d} » est bien le vôtre ?"))
            .unwrap_or_default()
    });
    let rappel = Signal::derive(|| EFFET_LIEN.to_string());
    let repondre = move |id: Uuid, confirmer: bool| {
        ecriture.lancer(async move {
            session::client()
                .individu_repondre(id, confirmer)
                .await
                .map(|()| message_reponse(confirmer).to_string())
        });
    };
    view! {
        <section class="abs-section" aria-labelledby="ind-recues">
            <h2 id="ind-recues" class="vs-section-titre">"Demandes à confirmer"</h2>
            {if recues.is_empty() {
                view! {
                    <EtatVide titre="Aucune demande en attente"
                        message="Quand un autre compte déclare être aussi vous, la demande apparaît ici pendant 24 heures."/>
                }.into_any()
            } else {
                view! {
                    <p>"Un autre compte de l'établissement déclare être aussi vous. Ne confirmez que si ce compte est bien le vôtre. \
                        Après un refus, ce compte ne peut plus vous le redemander pendant 30 jours."</p>
                    <ul class="ui-liste">
                        {recues.into_iter().map(|d| {
                            let id = d.id;
                            let nom = d.declarant.clone();
                            let detail = format!(
                                "Demandé le {} · à confirmer avant le {}",
                                date_heure_paris(d.declare_le),
                                date_heure_paris(d.expire_le)
                            );
                            // Le nom accessible commence par le libellé visible (WCAG 2.5.3, commande vocale).
                            let acc_confirmer = format!("C'est moi : le compte {nom} est le mien");
                            let acc_refuser = format!("Ce n'est pas moi : refuser la demande du compte {nom}");
                            view! {
                                <li class="ui-liste__element">
                                    <div class="ui-liste__principal">
                                        <span class="ui-liste__titre">{format!("Compte « {} »", d.declarant)}</span>
                                        <span class="ui-liste__detail">{detail}</span>
                                    </div>
                                    <Bouton variante=Variante::Primaire taille=Taille::Compacte
                                        desactive=Signal::derive(move || ecriture.en_cours.get())
                                        libelle_accessible=acc_confirmer
                                        on_click=Callback::new(move |_| {
                                            choisie.set(Some((id, nom.clone())));
                                            ouvert.set(true);
                                        })>
                                        "C'est moi"
                                    </Bouton>
                                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                        desactive=Signal::derive(move || ecriture.en_cours.get())
                                        libelle_accessible=acc_refuser
                                        on_click=Callback::new(move |_| repondre(id, false))>
                                        "Ce n'est pas moi"
                                    </Bouton>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
            <ConfirmationDetaillee
                ouvert=ouvert
                id="ind-confirmer"
                titre="Confirmer votre autre compte"
                message=message
                rappel=rappel
                libelle_confirmer="Oui, c'est mon compte"
                on_confirmer=Callback::new(move |_| {
                    if let Some((id, _)) = choisie.get_untracked() {
                        repondre(id, true);
                    }
                })
            />
        </section>
    }
}

#[component]
fn Declarer(ecriture: Ecriture) -> impl IntoView {
    let email = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let envoyer = move || match adresse_a_envoyer(&email.get_untracked()) {
        Err(m) => erreur.set(Some(m.to_string())),
        Ok(e) => {
            erreur.set(None);
            ecriture.lancer(async move {
                let r = session::client().individu_declarer(&e).await;
                // La saisie n'est effacée qu'après l'accord du serveur (un refus la laisse à corriger).
                if r.is_ok() {
                    email.set(String::new());
                }
                r.map(|()| MESSAGE_DECLARATION.to_string())
            });
        }
    };
    view! {
        <section class="abs-section" aria-labelledby="ind-declarer">
            <h2 id="ind-declarer" class="vs-section-titre">"Déclarer un autre compte"</h2>
            <p>
                "Saisissez l'adresse e-mail de votre autre compte dans l'établissement. La demande se confirme ensuite \
                 depuis cet autre compte, dans les 24 heures : sans confirmation, elle ne vaut rien. Cinq demandes par jour au plus."
            </p>
            // `novalidate` : le contrôle de l'écran (`adresse_a_envoyer`) affiche son message, lié au champ, au lieu de
            // la bulle du navigateur, qui bloquait l'envoi sans rien annoncer.
            <form class="ra-formulaire" novalidate on:submit=move |ev| { ev.prevent_default(); envoyer(); }>
                <ChampTexte id="ind-email" libelle="Adresse e-mail de l'autre compte" valeur=email genre="email"
                    autocomplete="off" erreur=Signal::derive(move || erreur.get())/>
                <Bouton soumettre=true en_cours=Signal::derive(move || ecriture.en_cours.get())>
                    "Envoyer la demande"
                </Bouton>
            </form>
        </section>
    }
}

// ───────────────────────── Fiches reliées (direction, secrétariat) ─────────────────────────

/// Paire en cours de liaison : deux fiches, désignation, vient d'une proposition.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PaireARelier {
    a: Uuid,
    b: Uuid,
    designation: String,
    depuis_proposition: bool,
}

/// État des boîtes de dialogue de la page (une boîte par action, titre statique).
#[derive(Clone, Copy)]
struct Dialogues {
    relier: RwSignal<bool>,
    paire: RwSignal<Option<PaireARelier>>,
    motif_lien: RwSignal<String>,
    erreur_lien: RwSignal<Option<String>>,
    /// Compteur des liaisons manuelles confirmées (efface les deux choix).
    relies_manuels: RwSignal<u32>,
    ecarter: RwSignal<bool>,
    a_ecarter: RwSignal<Option<PaireARelier>>,
    motif_ecart: RwSignal<String>,
    erreur_ecart: RwSignal<Option<String>>,
    lever: RwSignal<bool>,
    a_lever: RwSignal<Option<(Uuid, String)>>,
    motif_levee: RwSignal<String>,
    erreur_levee: RwSignal<Option<String>>,
}

impl Dialogues {
    fn new() -> Self {
        Self {
            relier: RwSignal::new(false),
            paire: RwSignal::new(None),
            motif_lien: RwSignal::new(AUCUN.to_string()),
            erreur_lien: RwSignal::new(None),
            relies_manuels: RwSignal::new(0),
            ecarter: RwSignal::new(false),
            a_ecarter: RwSignal::new(None),
            motif_ecart: RwSignal::new(AUCUN.to_string()),
            erreur_ecart: RwSignal::new(None),
            lever: RwSignal::new(false),
            a_lever: RwSignal::new(None),
            motif_levee: RwSignal::new(AUCUN.to_string()),
            erreur_levee: RwSignal::new(None),
        }
    }

    fn ouvrir_relier(&self, p: PaireARelier) {
        self.paire.set(Some(p));
        self.motif_lien.set(AUCUN.to_string());
        self.erreur_lien.set(None);
        self.relier.set(true);
    }

    fn ouvrir_ecarter(&self, p: PaireARelier) {
        self.a_ecarter.set(Some(p));
        self.motif_ecart.set(AUCUN.to_string());
        self.erreur_ecart.set(None);
        self.ecarter.set(true);
    }

    fn ouvrir_lever(&self, id: Uuid, designation: String) {
        self.a_lever.set(Some((id, designation)));
        self.motif_levee.set(AUCUN.to_string());
        self.erreur_levee.set(None);
        self.lever.set(true);
    }
}

#[component]
pub fn PageFichesReliees() -> impl IntoView {
    let moteur = use_droits_moteur();
    let liens =
        LocalResource::new(|| async { session::client().individu_liens().await.map(trier_liens) });
    let propositions = LocalResource::new(|| async {
        session::client()
            .individu_propositions()
            .await
            .map(trier_propositions)
    });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        liens.refetch();
        propositions.refetch();
    }));
    let peut_relier = Memo::new(move |_| moteur.get().a_etablissement("personnes.individu.relier"));
    let peut_lever = Memo::new(move |_| moteur.get().a_etablissement("personnes.individu.lever"));
    let peut_chercher = Memo::new(move |_| moteur.get().a_etablissement("personnes.gerer"));
    let d = Dialogues::new();
    view! {
        <Title text="Fiches reliées"/>
        <EnTetePage
            surtitre="Direction et secrétariat".to_string()
            titre="Fiches reliées".to_string()
            sous_titre="Reconnaître deux fiches ou deux comptes d'une même personne, pour que les règles « jamais pour \
                soi ni pour son enfant » la voient aussi depuis son autre compte.".to_string()
        />
        <Alerte niveau=Niveau::Info titre="Un lien n'ajoute que des refus">
            <p>{EFFET_LIEN}</p>
        </Alerte>
        <RetourEcriture ecriture=ecriture/>

        <section class="abs-section" aria-labelledby="ind-propositions">
            <h2 id="ind-propositions" class="vs-section-titre">"Propositions"</h2>
            <p>"Fiches qui partagent une adresse e-mail ou un téléphone, dont l'une au moins est un membre du personnel. \
                Rien n'est relié sans votre décision."</p>
            {move || match propositions.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des propositions…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| propositions.refetch())/>
                }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucune proposition" message="Aucune ressemblance à examiner pour l'instant."/>
                }.into_any(),
                Some(Ok(l)) => view! {
                    <ListePropositions liste=l ecriture=ecriture peut_relier=peut_relier dialogues=d/>
                }.into_any(),
            }}
        </section>

        <section class="abs-section" aria-labelledby="ind-relier">
            <h2 id="ind-relier" class="vs-section-titre">"Relier deux fiches"</h2>
            {move || {
                if !peut_relier.get() {
                    view! { <p>"Relier des fiches demande le droit de relier les fiches d'une même personne (direction et secrétariat par défaut)."</p> }.into_any()
                } else if !peut_chercher.get() {
                    view! { <p>"Chercher une fiche demande le droit de gérer les personnes : utilisez les propositions."</p> }.into_any()
                } else {
                    view! { <RelierManuel ecriture=ecriture dialogues=d/> }.into_any()
                }
            }}
        </section>

        <section class="abs-section" aria-labelledby="ind-liens">
            <h2 id="ind-liens" class="vs-section-titre">"Liens actifs"</h2>
            {move || match liens.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des liens…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liens.refetch())/>
                }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucun lien" message="Aucune fiche n'est reliée à une autre dans l'établissement."/>
                }.into_any(),
                Some(Ok(l)) => view! {
                    <TableauLiens liste=l ecriture=ecriture peut_lever=peut_lever dialogues=d/>
                }.into_any(),
            }}
        </section>

        <DialogueRelier ecriture=ecriture d=d/>
        <DialogueEcarter ecriture=ecriture d=d/>
        <DialogueLever ecriture=ecriture d=d/>
    }
}

#[component]
fn ListePropositions(
    liste: Vec<PropositionIndividu>,
    ecriture: Ecriture,
    peut_relier: Memo<bool>,
    dialogues: Dialogues,
) -> impl IntoView {
    let d = dialogues;
    view! {
        <ul class="ui-liste">
            {liste.into_iter().map(|p| {
                let designation = designation_paire(&p.nom_a, &p.nom_b);
                let raisons = libelle_raisons(&p.raisons);
                let paire = PaireARelier {
                    a: p.personne_a,
                    b: p.personne_b,
                    designation: designation.clone(),
                    depuis_proposition: true,
                };
                let paire_ecart = paire.clone();
                let acc_relier = format!("Relier {designation}");
                let acc_ecarter = format!("Écarter la proposition {designation}");
                view! {
                    <li class="ui-liste__element">
                        <div class="ui-liste__principal">
                            <span class="ui-liste__titre">{designation.clone()}</span>
                            <span class="ui-liste__detail">{raisons}</span>
                        </div>
                        {move || peut_relier.get().then(|| {
                            let paire = paire.clone();
                            let paire_ecart = paire_ecart.clone();
                            let acc_relier = acc_relier.clone();
                            let acc_ecarter = acc_ecarter.clone();
                            view! {
                                <Bouton variante=Variante::Primaire taille=Taille::Compacte
                                    desactive=Signal::derive(move || ecriture.en_cours.get())
                                    libelle_accessible=acc_relier
                                    on_click=Callback::new(move |_| d.ouvrir_relier(paire.clone()))>
                                    "Relier…"
                                </Bouton>
                                <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                    desactive=Signal::derive(move || ecriture.en_cours.get())
                                    libelle_accessible=acc_ecarter
                                    on_click=Callback::new(move |_| d.ouvrir_ecarter(paire_ecart.clone()))>
                                    "Écarter…"
                                </Bouton>
                            }
                        })}
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}

#[component]
fn TableauLiens(
    liste: Vec<LienIndividu>,
    ecriture: Ecriture,
    peut_lever: Memo<bool>,
    dialogues: Dialogues,
) -> impl IntoView {
    let d = dialogues;
    let n = liste.len();
    view! {
        <div class="ui-tableau-cadre">
            <table class="ui-tableau ui-tableau--cartes">
                <caption class="ui-sr">{libelle_nombre_liens(n)}</caption>
                <thead>
                    <tr>
                        <th scope="col">"Fiches"</th>
                        <th scope="col">"Origine"</th>
                        <th scope="col">"Motif"</th>
                        <th scope="col">"Depuis"</th>
                        <th scope="col"><span class="ui-sr">"Actions"</span></th>
                    </tr>
                </thead>
                <tbody>
                    {liste.into_iter().map(|l| {
                        let designation = designation_paire(&l.nom_a, &l.nom_b);
                        let id = l.id;
                        let acc = format!("Lever le lien entre {designation}");
                        let pour_dialogue = designation.clone();
                        view! {
                            <tr>
                                <th scope="row">{designation}</th>
                                <td data-label="Origine">{libelle_origine(&l.origine)}</td>
                                <td data-label="Motif">{libelle_motif_lien(&l.motif_code)}</td>
                                <td data-label="Depuis">{date_heure_paris(l.pose_le)}</td>
                                <td>
                                    {move || peut_lever.get().then(|| {
                                        let acc = acc.clone();
                                        let pour_dialogue = pour_dialogue.clone();
                                        view! {
                                            <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                                desactive=Signal::derive(move || ecriture.en_cours.get())
                                                libelle_accessible=acc
                                                on_click=Callback::new(move |_| d.ouvrir_lever(id, pour_dialogue.clone()))>
                                                "Lever…"
                                            </Bouton>
                                        }
                                    })}
                                </td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

/// Place le focus sur le premier élément du sélecteur, après le rendu.
#[cfg(target_arch = "wasm32")]
fn focaliser(selecteur: String) {
    use leptos::wasm_bindgen::JsCast;
    leptos::prelude::set_timeout(
        move || {
            if let Some(el) = leptos::prelude::document()
                .query_selector(&selecteur)
                .ok()
                .flatten()
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = el.focus();
            }
        },
        std::time::Duration::from_millis(60),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn focaliser(_selecteur: String) {}

/// Choix d'une fiche : recherche (deux caractères au moins), puis « Choisir ». Aucune coordonnée
/// n'est affichée (nom, compte, élèves liés seulement). Le focus suit le choix (« Changer », puis le champ).
#[component]
fn ChoixFiche(
    #[prop(into)] id: String,
    #[prop(into)] libelle: String,
    choisie: RwSignal<Option<FicheChoisie>>,
    autre: RwSignal<Option<FicheChoisie>>,
) -> impl IntoView {
    let saisie = RwSignal::new(String::new());
    let requete = RwSignal::new(None::<String>);
    let trouvees = LocalResource::new(move || {
        let q = requete.get();
        async move {
            match q {
                None => Ok::<Vec<PersonneTrouveeDto>, ApiError>(Vec::new()),
                Some(q) => session::client().personnes_rechercher(&q).await,
            }
        }
    });
    let champ = format!("{id}-recherche");
    let liste_libelle = format!("Fiches trouvées pour : {libelle}");
    let titre = libelle.clone();
    // Le bouton activé disparaît (« Choisir » devient « Changer » et inversement) : le focus irait sur la page.
    let vers_changer = format!("#{id} button");
    let vers_champ = format!("#{champ}");
    view! {
        <fieldset id=id>
            <legend>{titre}</legend>
            {move || match choisie.get() {
                Some(f) => view! {
                    <p>
                        <span>{f.libelle}</span>
                        " "
                        <Bouton variante=Variante::Discret taille=Taille::Compacte
                            on_click=Callback::new({
                                let vers_champ = vers_champ.clone();
                                move |_| {
                                    choisie.set(None);
                                    focaliser(vers_champ.clone());
                                }
                            })>
                            "Changer"
                        </Bouton>
                    </p>
                }.into_any(),
                None => {
                    let champ = champ.clone();
                    let liste_libelle = liste_libelle.clone();
                    let vers_changer = vers_changer.clone();
                    view! {
                        <form class="ra-formulaire" on:submit=move |ev| {
                            ev.prevent_default();
                            requete.set(recherche_a_envoyer(&saisie.get_untracked()));
                        }>
                            <ChampTexte id=champ libelle="Nom, prénom, e-mail ou téléphone" valeur=saisie genre="search"
                                autocomplete="off" aide="Deux caractères au moins."/>
                            <Bouton soumettre=true variante=Variante::Secondaire>"Chercher"</Bouton>
                        </form>
                        {move || match (requete.get(), trouvees.get()) {
                            (None, _) => ().into_any(),
                            (Some(_), None) => view! { <Squelette lignes=2 libelle="Recherche…"/> }.into_any(),
                            (Some(_), Some(Err(e))) => view! {
                                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| trouvees.refetch())/>
                            }.into_any(),
                            (Some(_), Some(Ok(v))) => {
                                let c = fiches_choisissables(&v, autre.get_untracked().map(|f| f.id));
                                if c.is_empty() {
                                    view! { <EtatVide titre="Aucune fiche trouvée"/> }.into_any()
                                } else {
                                    let liste_libelle = liste_libelle.clone();
                                    let nombre = libelle_nombre_fiches(c.len());
                                    view! {
                                        <p role="status">{nombre}</p>
                                        <ul class="ui-liste" aria-label=liste_libelle>
                                            {c.into_iter().map(|f| {
                                                let acc = format!("Choisir {}", f.libelle);
                                                let vers_changer = vers_changer.clone();
                                                let texte = f.libelle.clone();
                                                view! {
                                                    <li class="ui-liste__element">
                                                        <div class="ui-liste__principal"><span class="ui-liste__titre">{texte}</span></div>
                                                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                                            libelle_accessible=acc
                                                            on_click=Callback::new(move |_| {
                                                                choisie.set(Some(f.clone()));
                                                                focaliser(vers_changer.clone());
                                                            })>
                                                            "Choisir"
                                                        </Bouton>
                                                    </li>
                                                }
                                            }).collect_view()}
                                        </ul>
                                    }.into_any()
                                }
                            }
                        }}
                    }.into_any()
                }
            }}
        </fieldset>
    }
}

#[component]
fn RelierManuel(ecriture: Ecriture, dialogues: Dialogues) -> impl IntoView {
    let d = dialogues;
    let a = RwSignal::new(None::<FicheChoisie>);
    let b = RwSignal::new(None::<FicheChoisie>);
    let erreur = RwSignal::new(None::<String>);
    let ouvrir = move |_| {
        let (fa, fb) = (a.get_untracked(), b.get_untracked());
        // Même contrôle que l'envoi (le motif se choisit ensuite dans la boîte de dialogue).
        match lien_a_envoyer(fa.as_ref(), fb.as_ref(), MOTIFS_LIEN[0]) {
            Err(m) => erreur.set(Some(m.to_string())),
            Ok((ia, ib, _)) => {
                erreur.set(None);
                let designation = designation_paire(
                    &fa.map(|f| f.libelle).unwrap_or_default(),
                    &fb.map(|f| f.libelle).unwrap_or_default(),
                );
                d.ouvrir_relier(PaireARelier {
                    a: ia,
                    b: ib,
                    designation,
                    depuis_proposition: false,
                });
            }
        }
    };
    // Une fois la liaison manuelle confirmée, les deux choix sont effacés (pas au premier passage).
    Effect::new(move |avant: Option<u32>| {
        let n = d.relies_manuels.get();
        if avant.is_some_and(|v| v != n) {
            a.set(None);
            b.set(None);
        }
        n
    });
    view! {
        <p>"Choisissez les deux fiches de la même personne : par exemple sa fiche de personnel et sa fiche de parent."</p>
        <div class="voy-formulaire voy-formulaire--ligne">
            <ChoixFiche id="ind-fiche-a" libelle="Première fiche" choisie=a autre=b/>
            <ChoixFiche id="ind-fiche-b" libelle="Seconde fiche" choisie=b autre=a/>
        </div>
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        <Bouton desactive=Signal::derive(move || ecriture.en_cours.get())
            on_click=Callback::new(ouvrir)>
            "Relier ces deux fiches…"
        </Bouton>
    }
}

#[component]
fn DialogueRelier(ecriture: Ecriture, d: Dialogues) -> impl IntoView {
    let options = options_motif_lien();
    let confirmer = move |_| {
        let Some(p) = d.paire.get_untracked() else {
            return;
        };
        let motif = match motif_valide(&d.motif_lien.get_untracked(), &MOTIFS_LIEN) {
            Ok(m) => m,
            Err(m) => return d.erreur_lien.set(Some(m.to_string())),
        };
        d.relier.set(false);
        d.paire.set(None);
        let manuel = !p.depuis_proposition;
        let body = RelierFichesBody {
            personne_a: p.a,
            personne_b: p.b,
            motif_code: motif,
            depuis_proposition: p.depuis_proposition,
        };
        ecriture.lancer(async move {
            let r = session::client().individu_relier(&body).await;
            // Les deux choix ne sont effacés qu'une fois le lien posé (un refus les laisse à corriger).
            if r.is_ok() && manuel {
                d.relies_manuels.update(|n| *n += 1);
            }
            r.map(|_| "Fiches reliées : les comptes concernés sont prévenus.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=d.relier titre="Relier deux fiches" id="ind-dlg-relier">
            <p>{move || d.paire.get().map(|p| format!("Relier {} : ce sont les fiches d'une même personne.", p.designation)).unwrap_or_default()}</p>
            <p class="confirmation__rappel"><strong>{EFFET_LIEN}</strong></p>
            <Selection id="ind-motif-lien" libelle="Motif" valeur=d.motif_lien options=options requis=true
                erreur=Signal::derive(move || d.erreur_lien.get())/>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| d.relier.set(false))>"Retour"</Bouton>
                <Bouton on_click=Callback::new(confirmer)>"Relier"</Bouton>
            </div>
        </Dialogue>
    }
}

#[component]
fn DialogueEcarter(ecriture: Ecriture, d: Dialogues) -> impl IntoView {
    let options = options_motif_ecartee();
    let confirmer = move |_| {
        let Some(p) = d.a_ecarter.get_untracked() else {
            return;
        };
        let motif = match motif_valide(&d.motif_ecart.get_untracked(), &MOTIFS_ECARTEE) {
            Ok(m) => m,
            Err(m) => return d.erreur_ecart.set(Some(m.to_string())),
        };
        d.ecarter.set(false);
        d.a_ecarter.set(None);
        let body = EcarterPropositionBody {
            personne_a: p.a,
            personne_b: p.b,
            motif_code: motif,
        };
        ecriture.lancer(async move {
            session::client()
                .individu_ecarter(&body)
                .await
                .map(|()| "Proposition écartée : elle ne sera plus proposée.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=d.ecarter titre="Écarter une proposition" id="ind-dlg-ecarter">
            <p>{move || d.a_ecarter.get().map(|p| format!("Écarter la proposition {} ?", p.designation)).unwrap_or_default()}</p>
            <p>"Elle ne sera plus proposée. Vous pourrez toujours relier ces fiches plus tard si vous changez d'avis."</p>
            <Selection id="ind-motif-ecart" libelle="Motif" valeur=d.motif_ecart options=options requis=true
                erreur=Signal::derive(move || d.erreur_ecart.get())/>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| d.ecarter.set(false))>"Retour"</Bouton>
                <Bouton on_click=Callback::new(confirmer)>"Écarter"</Bouton>
            </div>
        </Dialogue>
    }
}

#[component]
fn DialogueLever(ecriture: Ecriture, d: Dialogues) -> impl IntoView {
    let options = options_motif_levee();
    let confirmer = move |_| {
        let Some((id, _)) = d.a_lever.get_untracked() else {
            return;
        };
        let motif = match motif_valide(&d.motif_levee.get_untracked(), &MOTIFS_LEVEE) {
            Ok(m) => m,
            Err(m) => return d.erreur_levee.set(Some(m.to_string())),
        };
        d.lever.set(false);
        d.a_lever.set(None);
        ecriture.lancer(async move {
            session::client()
                .individu_lever(id, &motif)
                .await
                .map(|()| "Lien levé : les comptes concernés sont prévenus.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=d.lever titre="Lever un lien" id="ind-dlg-lever">
            <p>{move || d.a_lever.get().map(|(_, n)| format!("Lever le lien entre {n} ?")).unwrap_or_default()}</p>
            <p class="confirmation__rappel"><strong>
                "Lever retire des refus : les deux fiches redeviennent deux personnes pour toutes les règles. \
                 La personne concernée ne peut jamais lever son propre lien."
            </strong></p>
            <Selection id="ind-motif-levee" libelle="Motif" valeur=d.motif_levee options=options requis=true
                erreur=Signal::derive(move || d.erreur_levee.get())/>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| d.lever.set(false))>"Retour"</Bouton>
                <Bouton variante=Variante::Danger on_click=Callback::new(confirmer)>"Lever le lien"</Bouton>
            </div>
        </Dialogue>
    }
}
