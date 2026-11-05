//! « Journée » de la vie scolaire et de l'accueil (socle 2, lot S2-4) : absents du jour (prévenus
//! ou non), retards, déclarations, vérification des appels (non faits, incohérences, relance),
//! absence annoncée à l'accueil.
//!
//! Routes d'`ent-api` : `GET /vs/tableau-jour`, `POST /vs/appels/relancer`,
//! `POST /vs/declarations/accueil`, `GET /eleves/search`, `GET /motifs-absence`. Le serveur
//! décide des droits (`absences.lire`, `appel.verifier`, `absences.declarer`) ; l'écran n'est
//! proposé qu'aux profils qui les ont par défaut et traite les refus (403).

pub mod modele;

use chrono::NaiveDate;
use ent_web_api::{ApiError, EleveTrouve, TableauJour};
use ent_web_ui::{
    use_notifications, AccesRefuse, Alerte, Annonce, Badge, Bouton, ChampTexte, EnTetePage,
    EtatVide, Niveau, Squelette, Statistique, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::declaration::{
    libelle_periode_declaration, libelle_source, DialogueDeclaration, Origine,
};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::temps::{aujourdhui, date_longue};
use crate::noyau::{reconnexion, session};
use modele::{
    compteurs, libelle_absent, libelle_etat_appel, libelle_incoherence, libelle_incoherence_eleve,
    libelle_retard_appel, libelle_seance, nom_complet, ton_etat_appel, trier_absents,
};

fn message_echec(e: ApiError) -> String {
    match e {
        ApiError::NonConnecte => {
            reconnexion::signaler_session_perdue();
            reconnexion::MESSAGE_SESSION_EXPIREE.to_string()
        }
        autre => autre.message_utilisateur(),
    }
}

#[component]
pub fn PageJourneeVs() -> impl IntoView {
    let droits = use_droits();
    let ctx = session::use_session();
    view! {
        <Title text="Journée"/>
        {move || match ctx.moi.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement de la journée…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| ctx.moi.refetch())/> }.into_any(),
            Some(Ok(_)) if droits.get().trancher_justificatif => view! { <EcranJournee/> }.into_any(),
            Some(Ok(_)) => view! {
                <EnTetePage titre="Journée".to_string()/>
                <AccesRefuse message="La journée de la vie scolaire n'est pas accessible avec votre profil."/>
            }
                .into_any(),
        }}
    }
}

#[component]
fn EcranJournee() -> impl IntoView {
    let notifications = use_notifications();
    let annonce = RwSignal::new(String::new());
    // Le jour est posé après l'hydratation : aucune date dans le HTML rendu côté serveur.
    let jour_texte = RwSignal::new(String::new());
    Effect::new(move |_| {
        if jour_texte.get_untracked().is_empty() {
            jour_texte.set(aujourdhui().to_string());
        }
    });
    let jour =
        Memo::new(move |_| NaiveDate::parse_from_str(jour_texte.get().trim(), "%Y-%m-%d").ok());
    let tableau = LocalResource::new(move || {
        let j = jour.get();
        async move {
            match j {
                Some(j) => session::client().tableau_du_jour(j).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let erreur = RwSignal::new(None::<String>);

    let ouvert = RwSignal::new(false);
    let eleve = RwSignal::new(None::<(Uuid, String)>);
    let fait = Callback::new(move |_| {
        let t = "Absence enregistrée : elle sera rapprochée de l'appel.";
        notifications.succes(t);
        annonce.set(t.to_string());
        tableau.refetch();
    });

    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>
        <EnTetePage
            titre="Journée".to_string()
            sous_titre="Absents, retards, absences annoncées et appels à vérifier.".to_string()
            surtitre=Signal::derive(move || jour.get().map(date_longue).unwrap_or_default())
        />
        <div class="jvs-barre">
            <ChampTexte id="jvs-jour" libelle="Jour" valeur=jour_texte genre="date"/>
            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| tableau.refetch())>
                "Actualiser"
            </Bouton>
            <Bouton on_click=Callback::new(move |_| { eleve.set(None); ouvert.set(true); })>
                "Absence annoncée à l'accueil"
            </Bouton>
        </div>
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        {move || match tableau.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement de la journée…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| tableau.refetch())/> }.into_any(),
            Some(Ok(None)) => view! { <EtatVide titre="Date invalide" message="Choisissez un jour valide."/> }.into_any(),
            Some(Ok(Some(t))) => view! { <ContenuJournee t=t tableau=tableau erreur=erreur annonce=annonce/> }.into_any(),
        }}
        <ChoixEleveAccueil eleve=eleve ouvert=ouvert/>
        <DialogueDeclaration
            ouvert=ouvert
            origine=Origine::Accueil
            eleve=Signal::derive(move || eleve.get())
            on_fait=fait
        />
    }
}

#[component]
fn ContenuJournee(
    t: TableauJour,
    tableau: LocalResource<Result<Option<TableauJour>, ApiError>>,
    erreur: RwSignal<Option<String>>,
    annonce: RwSignal<String>,
) -> impl IntoView {
    let notifications = use_notifications();
    let c = compteurs(&t);
    let jour = t.jour;
    let en_cours = RwSignal::new(false);

    let relancer = Callback::new(move |creneau: Uuid| {
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client().relancer_appel(creneau, jour).await;
            en_cours.set(false);
            match r {
                Ok(()) => {
                    let m = "Relance envoyée à l'enseignant.";
                    notifications.succes(m);
                    annonce.set(m.to_string());
                    tableau.refetch();
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    });

    let ton_non_prevenus = if c.non_prevenus > 0 {
        TonEtat::Avertissement
    } else {
        TonEtat::Ok
    };
    let ton_appels = if c.appels_a_verifier > 0 {
        TonEtat::Mal
    } else {
        TonEtat::Ok
    };
    let absents = trier_absents(t.absents.clone());
    let seances = t.verification.seances_sans_appel.clone();
    let incoherences = t.verification.incoherences.clone();
    let retards = t.retards.clone();
    let declarations = t.declarations.clone();
    view! {
        <div class="jvs-stats">
            <Statistique libelle="Absents" valeur=Signal::derive(move || c.absents.to_string())/>
            <Statistique
                libelle="Non prévenus"
                valeur=Signal::derive(move || c.non_prevenus.to_string())
                ton=ton_non_prevenus
            />
            <Statistique libelle="Retards" valeur=Signal::derive(move || c.retards.to_string())/>
            <Statistique
                libelle="Appels à vérifier"
                valeur=Signal::derive(move || c.appels_a_verifier.to_string())
                ton=ton_appels
            />
        </div>

        <section class="jvs-section" aria-labelledby="jvs-appels">
            <h2 id="jvs-appels" class="vs-section-titre">"Appels à vérifier"</h2>
            {if seances.is_empty() {
                view! { <p class="texte-attenue">"Tous les appels attendus à cette heure sont faits."</p> }.into_any()
            } else {
                view! {
                    <ul class="jvs-liste">
                        {seances
                            .into_iter()
                            .map(|s| {
                                let id = s.creneau_id;
                                let intitule = libelle_seance(&s);
                                let detail = libelle_retard_appel(&s);
                                let etat = s.etat.clone();
                                view! {
                                    <li class="jvs-ligne">
                                        <div>
                                            <strong>{intitule.clone()}</strong>
                                            <span class="texte-attenue">{format!(" — {detail}")}</span>
                                        </div>
                                        <Badge libelle=libelle_etat_appel(&etat) ton=ton_etat_appel(&etat)/>
                                        <Bouton
                                            variante=Variante::Secondaire
                                            taille=Taille::Compacte
                                            en_cours=en_cours
                                            libelle_accessible=format!("Relancer l'enseignant : {intitule}")
                                            on_click=Callback::new(move |_| relancer.run(id))
                                        >
                                            "Relancer"
                                        </Bouton>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                }
                    .into_any()
            }}
            {(!incoherences.is_empty())
                .then(|| view! {
                    <h3 class="jvs-sous-titre">"Incohérences entre appels"</h3>
                    <ul class="jvs-liste">
                        {incoherences
                            .into_iter()
                            .map(|i| view! {
                                <li class="jvs-ligne">
                                    <div>
                                        <strong>{libelle_incoherence_eleve(&i)}</strong>
                                        <span class="texte-attenue">{format!(" — {}", i.detail)}</span>
                                    </div>
                                    <Badge libelle=libelle_incoherence(&i.kind) ton=TonEtat::Avertissement/>
                                </li>
                            })
                            .collect_view()}
                    </ul>
                })}
        </section>

        <section class="jvs-section" aria-labelledby="jvs-absents">
            <h2 id="jvs-absents" class="vs-section-titre">"Absents"</h2>
            {if absents.is_empty() {
                view! { <EtatVide titre="Aucun absent" message="Aucune absence relevée ce jour."/> }.into_any()
            } else {
                view! {
                    <ul class="jvs-liste">
                        {absents
                            .into_iter()
                            .map(|a| {
                                let seances = a.seances.join(", ");
                                view! {
                                    <li class="jvs-ligne">
                                        <div>
                                            <strong>{libelle_absent(&a)}</strong>
                                            {(!seances.is_empty()).then(|| view! { <span class="texte-attenue">{format!(" — {seances}")}</span> })}
                                        </div>
                                        {if a.prevenue {
                                            view! { <Badge libelle="Prévenue" ton=TonEtat::Ok/> }.into_any()
                                        } else {
                                            view! { <Badge libelle="Non prévenue" ton=TonEtat::Avertissement/> }.into_any()
                                        }}
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                }
                    .into_any()
            }}
        </section>

        <section class="jvs-section" aria-labelledby="jvs-decl">
            <h2 id="jvs-decl" class="vs-section-titre">"Absences annoncées"</h2>
            {if declarations.is_empty() {
                view! { <p class="texte-attenue">"Aucune absence annoncée pour ce jour."</p> }.into_any()
            } else {
                view! {
                    <ul class="jvs-liste">
                        {declarations
                            .into_iter()
                            .map(|d| {
                                let nom = nom_complet(&d.prenom, &d.nom);
                                let periode = libelle_periode_declaration(&d.declaration);
                                let par = match d.declaration.declarant_nom.as_deref() {
                                    Some(n) => format!("{} ({n})", libelle_source(&d.declaration.source)),
                                    None => libelle_source(&d.declaration.source).to_string(),
                                };
                                view! {
                                    <li class="jvs-ligne">
                                        <div>
                                            <strong>{nom}</strong>
                                            <span class="texte-attenue">{format!(" — {periode} · {par}")}</span>
                                        </div>
                                        <Badge libelle=d.declaration.motif.clone()/>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                }
                    .into_any()
            }}
        </section>

        <section class="jvs-section" aria-labelledby="jvs-retards">
            <h2 id="jvs-retards" class="vs-section-titre">"Retards"</h2>
            {if retards.is_empty() {
                view! { <p class="texte-attenue">"Aucun retard relevé."</p> }.into_any()
            } else {
                view! {
                    <ul class="jvs-liste">
                        {retards
                            .into_iter()
                            .map(|r| view! {
                                <li class="jvs-ligne">
                                    <strong>{nom_complet(&r.prenom, &r.nom)}</strong>
                                    <span class="texte-attenue">
                                        {r.minutes.map(|m| format!("{m} min")).unwrap_or_else(|| "durée non précisée".into())}
                                    </span>
                                </li>
                            })
                            .collect_view()}
                    </ul>
                }
                    .into_any()
            }}
        </section>
    }
}

/// Recherche de l'élève concerné par une absence annoncée à l'accueil (`/eleves/search`).
#[component]
fn ChoixEleveAccueil(
    eleve: RwSignal<Option<(Uuid, String)>>,
    ouvert: RwSignal<bool>,
) -> impl IntoView {
    let requete = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let resultats = RwSignal::new(None::<Vec<EleveTrouve>>);
    let en_cours = RwSignal::new(false);
    let chercher = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        let texte = requete.get_untracked();
        if texte.trim().chars().count() < 2 {
            erreur.set(Some("Saisissez au moins 2 caractères.".into()));
            return;
        }
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            match session::client().messagerie_chercher_eleves(&texte).await {
                Ok(l) => resultats.set(Some(l)),
                Err(e) => {
                    resultats.set(None);
                    erreur.set(Some(message_echec(e)));
                }
            }
            en_cours.set(false);
        });
    };
    view! {
        <section class="jvs-recherche" aria-labelledby="jvs-recherche-titre">
            <h2 id="jvs-recherche-titre" class="vs-section-titre">"Élève concerné"</h2>
            <p class="texte-attenue">
                "Recherchez l'élève, puis choisissez-le pour saisir l'absence annoncée à l'accueil."
            </p>
            <form role="search" on:submit=chercher novalidate>
                <ChampTexte
                    id="jvs-eleve-q"
                    libelle="Rechercher un élève (nom, prénom ou classe)"
                    valeur=requete
                    genre="search"
                    erreur=Signal::derive(move || erreur.get())
                    autocomplete="off"
                />
                <Bouton soumettre=true en_cours=en_cours>"Rechercher"</Bouton>
            </form>
            {move || resultats.get().map(|liste| {
                if liste.is_empty() {
                    view! { <p class="texte-attenue" role="status">"Aucun élève ne correspond."</p> }.into_any()
                } else {
                    view! {
                        <ul class="jvs-liste" aria-label="Élèves trouvés">
                            {liste
                                .into_iter()
                                .map(|e| {
                                    let libelle = match e.classe.as_deref().filter(|c| !c.is_empty()) {
                                        Some(c) => format!("{} · {c}", nom_complet(&e.prenom, &e.nom)),
                                        None => nom_complet(&e.prenom, &e.nom),
                                    };
                                    let pour_choix = libelle.clone();
                                    let id = e.id;
                                    view! {
                                        <li>
                                            <Bouton
                                                variante=Variante::Secondaire
                                                libelle_accessible=format!("Annoncer une absence pour {libelle}")
                                                on_click=Callback::new(move |_| {
                                                    eleve.set(Some((id, pour_choix.clone())));
                                                    ouvert.set(true);
                                                })
                                            >
                                                {libelle}
                                            </Bouton>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                        .into_any()
                }
            })}
        </section>
    }
}
