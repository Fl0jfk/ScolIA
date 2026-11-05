//! Années scolaires et bascule d'année : statut, préparation de l'année suivante (reconduction),
//! montée des élèves avec aperçu, ouverture et clôture avec confirmation.

use chrono::NaiveDate;
use ent_types::referentiel_admin::{AnneeStatutDto, RapportLotDto};
use ent_web_api::ClasseApi;
use ent_web_ui::{
    Badge, Bouton, Case, ChampTexte, Confirmation, Dialogue, EtatVide, Selection, Squelette,
    Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele::{
    actions_annee, annee_courante, annee_suivante, apercu_ouverture, correspondances_choisies,
    etapes_bascule, libelle_annee_suivante, libelle_statut_annee, lire_date, lot_applicable,
    proposer_correspondances, resume_lot, resume_ouverture, resume_reconduction, ton_statut_annee,
    trier_annees, EtatEtape,
};
use super::{peut_gerer, Ecriture, EnTeteReferentiel, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_complete};

#[component]
pub fn PageAnnees() -> impl IntoView {
    let annees =
        LocalResource::new(|| async { session::client().annees_statut().await.map(trier_annees) });
    view! {
        <Title text="Années et bascule"/>
        <EnTeteReferentiel
            titre="Années et bascule d'année"
            sous_titre="Statut des années scolaires, préparation de la suivante, montée des élèves, ouverture et clôture."
            courant="/web/referentiel/annees"
        />
        {move || match annees.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des années…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| annees.refetch())/>
            }
            .into_any(),
            Some(Ok(liste)) => view! {
                <ContenuAnnees annees=liste recharger=Callback::new(move |_| annees.refetch())/>
            }
            .into_any(),
        }}
    }
}

#[component]
fn ContenuAnnees(annees: Vec<AnneeStatutDto>, recharger: Callback<()>) -> impl IntoView {
    let gerer = peut_gerer();
    let ecriture = Ecriture::nouveau(recharger);
    let etapes = etapes_bascule(&annees);
    let courante = annee_courante(&annees).cloned();
    let cible = annees
        .iter()
        .filter(|a| a.statut == "preparation")
        .min_by_key(|a| a.date_debut)
        .cloned();
    let montee = courante.clone().zip(cible);
    let vide = annees.is_empty();
    let proposition = annees.first().and_then(|a| {
        let suivant = libelle_annee_suivante(&a.libelle)?;
        Some((suivant, a.date_fin))
    });
    let cartes = annees
        .iter()
        .map(|a| {
            view! { <CarteAnnee annee=a.clone() toutes=annees.clone() ecriture=ecriture/> }
        })
        .collect_view();
    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="ra-section" aria-labelledby="ra-etapes">
            <h2 id="ra-etapes" class="vs-section-titre">"Bascule d'année : les étapes"</h2>
            <ol class="ra-etapes">
                {etapes
                    .into_iter()
                    .map(|e| {
                        let ton = match e.etat {
                            EtatEtape::Fait => ent_web_ui::TonEtat::Ok,
                            EtatEtape::AFaire => ent_web_ui::TonEtat::Avertissement,
                            EtatEtape::Attente => ent_web_ui::TonEtat::Neutre,
                        };
                        view! {
                            <li class="ra-etape">
                                <span>{e.libelle}</span>
                                <Badge libelle=e.etat.libelle() ton=ton/>
                            </li>
                        }
                    })
                    .collect_view()}
            </ol>
        </section>
        <section class="ra-section" aria-labelledby="ra-annees">
            <h2 id="ra-annees" class="vs-section-titre">"Années scolaires"</h2>
            {if vide {
                view! {
                    <EtatVide
                        titre="Aucune année scolaire"
                        message="Créez la première année ci-dessous."
                    />
                }
                .into_any()
            } else {
                view! { <ul class="ra-liste">{cartes}</ul> }.into_any()
            }}
        </section>
        {montee
            .map(|(source, cible)| {
                view! { <MonteeEleves source=source cible=cible/> }
            })}
        {move || {
            gerer
                .get()
                .then(|| {
                    view! { <FormulaireAnnee ecriture=ecriture proposition=proposition.clone()/> }
                })
        }}
    }
}

#[component]
fn CarteAnnee(
    annee: AnneeStatutDto,
    toutes: Vec<AnneeStatutDto>,
    ecriture: Ecriture,
) -> impl IntoView {
    let gerer = peut_gerer();
    let actions = actions_annee(&annee, &toutes);
    let courante = annee_courante(&toutes).cloned();
    let suivante = annee_suivante(&annee, &toutes).cloned();
    let (id, libelle) = (annee.id, annee.libelle.clone());
    let libelle_a = libelle.clone();
    let statut = annee.statut.clone();
    let avec_services = RwSignal::new(false);
    let forcer_ouvert = RwSignal::new(false);
    let preparer_ouvert = RwSignal::new(false);
    let ouvrir_ouvert = RwSignal::new(false);
    let cloturer_ouvert = RwSignal::new(false);
    let apercu = apercu_ouverture(courante.as_ref(), &annee);

    let preparer = Callback::new(move |_| {
        let avec = avec_services.get_untracked();
        ecriture.lancer(async move {
            let p = session::client().preparer_annee_suivante(id, avec).await?;
            Ok(format!(
                "Année {} préparée : {}",
                p.annee.libelle,
                resume_reconduction(&p.rapport)
            ))
        });
    });
    let ouvrir = Callback::new(move |_| {
        ecriture.lancer(async move {
            let r = session::client().ouvrir_annee(id).await?;
            Ok(resume_ouverture(&r))
        });
    });
    let cloturer = move |forcer: bool| {
        ecriture.lancer(async move {
            let a = session::client().cloturer_annee(id, forcer).await?;
            Ok(format!("Année {} clôturée.", a.libelle))
        });
    };
    let titre_preparer = format!("Préparer l'année suivante de {libelle} ?");
    let titre_ouvrir = format!("Ouvrir l'année {libelle} ?");
    let titre_cloturer = format!("Clôturer l'année {libelle} ?");
    let msg_preparer = format!(
        "Les classes, groupes et périodes de {libelle} seront reconduits sur l'année suivante, \
         qui sera créée en préparation. Aucun élève n'est déplacé à cette étape."
    );
    let msg_cloturer = format!(
        "L'année {libelle} passera en lecture seule. Si une période de notes est encore \
         ouverte, le serveur refuse : vous pourrez alors forcer la clôture (action tracée)."
    );
    let ids = (
        format!("conf-prep-{id}"),
        format!("conf-ouvrir-{id}"),
        format!("conf-clot-{id}"),
        format!("conf-force-{id}"),
    );
    let dates = format!(
        "du {} au {}",
        date_complete(annee.date_debut),
        date_complete(annee.date_fin)
    );
    view! {
        <li class="ra-carte-annee">
            <div>
                <strong>{annee.libelle.clone()}</strong>
                <span class="texte-attenue">{format!(" — {dates}")}</span>
            </div>
            <Badge libelle=libelle_statut_annee(&statut) ton=ton_statut_annee(&statut)/>
            {move || {
                gerer
                    .get()
                    .then(|| {
                        view! {
                            <div class="ra-actions">
                                {actions
                                    .preparer_suivante
                                    .then(|| {
                                        view! {
                                            <Bouton
                                                variante=Variante::Secondaire
                                                en_cours=ecriture.en_cours
                                                libelle_accessible=format!("Préparer l'année suivante de {}", libelle_a)
                                                on_click=Callback::new(move |_| preparer_ouvert.set(true))
                                            >
                                                "Préparer la suivante"
                                            </Bouton>
                                        }
                                    })}
                                {actions
                                    .ouvrir
                                    .then(|| {
                                        view! {
                                            <Bouton
                                                en_cours=ecriture.en_cours
                                                libelle_accessible=format!("Ouvrir l'année {}", libelle_a)
                                                on_click=Callback::new(move |_| ouvrir_ouvert.set(true))
                                            >
                                                "Ouvrir cette année"
                                            </Bouton>
                                        }
                                    })}
                                {actions
                                    .cloturer
                                    .then(|| {
                                        view! {
                                            <Bouton
                                                variante=Variante::Danger
                                                en_cours=ecriture.en_cours
                                                libelle_accessible=format!("Clôturer l'année {}", libelle_a)
                                                on_click=Callback::new(move |_| cloturer_ouvert.set(true))
                                            >
                                                "Clôturer"
                                            </Bouton>
                                            // Après un refus du serveur (période de notes ouverte) : forcer, tracé.
                                            {move || {
                                                ecriture
                                                    .erreur
                                                    .get()
                                                    .is_some()
                                                    .then(|| {
                                                        view! {
                                                            <Bouton
                                                                variante=Variante::Discret
                                                                en_cours=ecriture.en_cours
                                                                on_click=Callback::new(move |_| forcer_ouvert.set(true))
                                                            >
                                                                "Forcer la clôture…"
                                                            </Bouton>
                                                        }
                                                    })
                                            }}
                                        }
                                    })}
                            </div>
                        }
                    })
            }}
            {suivante
                .filter(|_| statut == "courante")
                .map(|s| view! {
                    <p class="texte-attenue ra-note">
                        {format!("Année suivante : {} ({}).", s.libelle, libelle_statut_annee(&s.statut))}
                    </p>
                })}
        <Dialogue ouvert=preparer_ouvert titre=titre_preparer id=ids.0>
            <p>{msg_preparer}</p>
            <Case
                id=format!("avec-services-{id}")
                libelle="Reconduire aussi les services d'enseignement (sans professeur principal)"
                coche=avec_services
            />
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| preparer_ouvert.set(false))>
                    "Annuler"
                </Bouton>
                <Bouton on_click=Callback::new(move |_| {
                    preparer_ouvert.set(false);
                    preparer.run(());
                })>
                    "Préparer"
                </Bouton>
            </div>
        </Dialogue>
        <Confirmation
            ouvert=ouvrir_ouvert
            id=ids.1
            titre=titre_ouvrir
            message=apercu
            libelle_confirmer="Ouvrir l'année"
            danger=true
            on_confirmer=ouvrir
        />
        <Confirmation
            ouvert=cloturer_ouvert
            id=ids.2
            titre=titre_cloturer
            message=msg_cloturer
            libelle_confirmer="Clôturer"
            danger=true
            on_confirmer=Callback::new(move |_| cloturer(false))
        />
        <Confirmation
            ouvert=forcer_ouvert
            id=ids.3
            titre="Forcer la clôture ?"
            message="La clôture passe outre une période de notes encore ouverte. L'action est tracée dans le journal."
            libelle_confirmer="Forcer la clôture"
            danger=true
            on_confirmer=Callback::new(move |_| cloturer(true))
        />
        </li>
    }
}

/// Formulaire de création d'une année (non chevauchante : le serveur le vérifie).
#[component]
fn FormulaireAnnee(ecriture: Ecriture, proposition: Option<(String, NaiveDate)>) -> impl IntoView {
    let (libelle_init, debut_init, fin_init) = match proposition {
        Some((l, fin_precedente)) => {
            let debut = fin_precedente + chrono::Duration::days(60);
            let fin = debut + chrono::Duration::days(300);
            (l, debut.to_string(), fin.to_string())
        }
        None => (String::new(), String::new(), String::new()),
    };
    let libelle = RwSignal::new(libelle_init);
    let debut = RwSignal::new(debut_init);
    let fin = RwSignal::new(fin_init);
    let erreur = RwSignal::new(None::<String>);
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let (l, d, f) = (
            libelle.get_untracked(),
            lire_date(&debut.get_untracked()),
            lire_date(&fin.get_untracked()),
        );
        match (l.trim().is_empty(), d, f) {
            (false, Some(d), Some(f)) if d < f => {
                erreur.set(None);
                ecriture.lancer(async move {
                    session::client().creer_annee(l.trim(), d, f).await?;
                    Ok(format!("Année {} créée.", l.trim()))
                });
            }
            _ => erreur.set(Some(
                "Renseignez un libellé et deux dates valides (le début précède la fin).".into(),
            )),
        }
    };
    view! {
        <section class="ra-section" aria-labelledby="ra-creer">
            <h2 id="ra-creer" class="vs-section-titre">"Créer une année"</h2>
            <p class="texte-attenue">
                "Réservé aux cas où « Préparer la suivante » ne convient pas (première année, rattrapage). \
                 Une année ne peut pas chevaucher une autre."
            </p>
            <form class="ra-formulaire" on:submit=envoyer novalidate>
                <ChampTexte id="annee-libelle" libelle="Libellé (ex. 2027-2028)" valeur=libelle requis=true/>
                <ChampTexte id="annee-debut" libelle="Début" valeur=debut genre="date" requis=true/>
                <ChampTexte
                    id="annee-fin"
                    libelle="Fin"
                    valeur=fin
                    genre="date"
                    requis=true
                    erreur=Signal::derive(move || erreur.get())
                />
                <Bouton soumettre=true en_cours=ecriture.en_cours>"Créer l'année"</Bouton>
            </form>
        </section>
    }
}

/// Montée des élèves : une classe cible par classe source, aperçu serveur, puis application.
#[component]
fn MonteeEleves(source: AnneeStatutDto, cible: AnneeStatutDto) -> impl IntoView {
    let gerer = peut_gerer();
    let (source_id, cible_id) = (source.id, cible.id);
    let classes = LocalResource::new(move || async move {
        let jour = aujourdhui();
        let s = session::client().classes_annee(source_id, jour).await?;
        let c = session::client().classes_annee(cible_id, jour).await?;
        Ok::<_, ent_web_api::ApiError>((s, c))
    });
    let (titre_s, titre_c) = (source.libelle.clone(), cible.libelle.clone());
    view! {
        <section class="ra-section" aria-labelledby="ra-montee">
            <h2 id="ra-montee" class="vs-section-titre">
                {format!("Montée des élèves : {titre_s} → {titre_c}")}
            </h2>
            <p class="texte-attenue">
                "Choisissez la classe d'arrivée de chaque classe d'origine (préremplie quand le code suit \
                 la montée habituelle : à vérifier). L'aperçu ne modifie rien ; l'application demande une confirmation."
            </p>
            {move || match classes.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des classes…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| classes.refetch())/>
                }
                .into_any(),
                Some(Ok((s, c))) => {
                    if s.is_empty() || c.is_empty() {
                        view! {
                            <EtatVide
                                titre="Rien à monter"
                                message="L'année d'origine ou l'année cible n'a pas encore de classes : préparez d'abord l'année suivante."
                            />
                        }
                        .into_any()
                    } else if gerer.get() {
                        view! { <TableauMontee source_id=source_id cible_id=cible_id sources=s cibles=c/> }.into_any()
                    } else {
                        view! { <p class="texte-attenue">"La montée est réservée aux personnes habilitées."</p> }.into_any()
                    }
                }
            }}
        </section>
    }
}

#[component]
fn TableauMontee(
    source_id: Uuid,
    cible_id: Uuid,
    sources: Vec<ClasseApi>,
    cibles: Vec<ClasseApi>,
) -> impl IntoView {
    let recharger = Callback::new(|_| {});
    let ecriture = Ecriture::nouveau(recharger);
    let propositions = proposer_correspondances(&sources, &cibles);
    let mut options = vec![(String::new(), "— ne pas monter —".to_string())];
    options.extend(cibles.iter().map(|c| (c.id.to_string(), c.libelle.clone())));
    let choix: Vec<(ClasseApi, RwSignal<String>)> = sources
        .into_iter()
        .zip(propositions)
        .map(|(s, (_, cible))| {
            let init = cible.map(|c| c.to_string()).unwrap_or_default();
            (s, RwSignal::new(init))
        })
        .collect();
    let valeurs = {
        let choix = choix.clone();
        Memo::new(move |_| {
            choix
                .iter()
                .map(|(s, v)| (s.id, v.get().parse::<Uuid>().ok()))
                .collect::<Vec<_>>()
        })
    };
    let apercu = RwSignal::new(None::<RapportLotDto>);
    // Toute modification des choix invalide l'aperçu : on n'applique que ce qui a été prévisualisé.
    Effect::new(move |_| {
        valeurs.track();
        apercu.set(None);
    });
    let confirmer = RwSignal::new(false);
    let lancer_apercu = Callback::new(move |_| {
        let c = correspondances_choisies(&valeurs.get_untracked());
        if c.is_empty() {
            ecriture
                .erreur
                .set(Some("Choisissez au moins une classe d'arrivée.".into()));
            return;
        }
        ecriture.erreur.set(None);
        ecriture.en_cours.set(true);
        leptos::task::spawn_local(async move {
            match session::client()
                .monter_eleves(cible_id, source_id, &c, true)
                .await
            {
                Ok(r) => apercu.set(Some(r)),
                Err(e) => ecriture.erreur.set(Some(super::message_echec(e))),
            }
            ecriture.en_cours.set(false);
        });
    });
    let appliquer = Callback::new(move |_| {
        let c = correspondances_choisies(&valeurs.get_untracked());
        ecriture.lancer(async move {
            let r = session::client()
                .monter_eleves(cible_id, source_id, &c, false)
                .await?;
            Ok(format!(
                "Montée appliquée : {} affectation(s) créée(s).",
                r.a_creer
            ))
        });
        apercu.set(None);
    });
    view! {
        <RetourEcriture ecriture=ecriture/>
        <ul class="ra-liste">
            {choix
                .into_iter()
                .map(|(s, v)| {
                    view! {
                        <li class="ra-ligne">
                            <span>{format!("{} ({} élève(s))", s.libelle, s.effectif)}</span>
                            <Selection
                                id=format!("montee-{}", s.id)
                                libelle=format!("Classe d'arrivée de {}", s.libelle)
                                valeur=v
                                options=options.clone()
                            />
                        </li>
                    }
                })
                .collect_view()}
        </ul>
        <div class="ra-actions">
            <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=lancer_apercu>
                "Voir l'aperçu"
            </Bouton>
            <Bouton
                desactive=Signal::derive(move || !lot_applicable(apercu.get().as_ref()))
                en_cours=ecriture.en_cours
                on_click=Callback::new(move |_| confirmer.set(true))
            >
                "Appliquer la montée"
            </Bouton>
        </div>
        {move || {
            apercu
                .get()
                .map(|r| {
                    view! {
                        <div class="ra-apercu" role="region" aria-label="Aperçu de la montée">
                            <h3 class="jvs-sous-titre">"Aperçu (aucune modification enregistrée)"</h3>
                            <ul>
                                {resume_lot(&r)
                                    .into_iter()
                                    .map(|l| view! { <li>{l}</li> })
                                    .collect_view()}
                            </ul>
                        </div>
                    }
                })
        }}
        <Confirmation
            ouvert=confirmer
            id="conf-montee"
            titre="Appliquer la montée ?"
            message="Les affectations prévisualisées seront créées dans l'année cible. Les élèves déjà affectés ailleurs ne sont pas modifiés."
            libelle_confirmer="Appliquer"
            on_confirmer=appliquer
        />
        <p class="texte-attenue">
            <a href=navigation::REFERENTIEL>"Retour à la vue d'ensemble"</a>
        </p>
    }
}
