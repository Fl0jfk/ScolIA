//! Dossier disciplinaire d'un élève : punitions, sanctions (lecture), procédures.

use ent_types::discipline::{ProcedureDto, PunitionDto, SanctionDto, TypeVsDto};
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, Confirmation, Dialogue, EnTetePage, EtatVide, Niveau,
    Selection, Squelette, Taille, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele::{
    actions_procedure, composer_convocation, composer_decision, composer_procedure,
    composer_punition, libelle_instance, libelle_statut_procedure, libelle_statut_punition,
    libelle_statut_sanction, libelle_type, lire_date, punition_cloturable, ton_statut_procedure,
    ton_statut_punition, SaisieProcedure, SaisiePunition, INSTANCES,
};
use crate::composants_metier::recherche_eleve::RechercheEleve;
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_complete};

#[component]
pub fn PageDiscipline() -> impl IntoView {
    let moteur = use_droits_moteur();
    let eleve = RwSignal::new(None::<(Uuid, String)>);
    view! {
        <Title text="Discipline"/>
        <EnTetePage
            surtitre="Vie scolaire".to_string()
            titre="Punitions et sanctions".to_string()
            sous_titre="Les punitions (mesures éducatives) et les sanctions (disciplinaires, au dossier) sont distinctes.".to_string()
        />
        {move || match eleve.get() {
            None => view! {
                <section class="jvs-recherche" aria-labelledby="disc-recherche">
                    <h2 id="disc-recherche" class="vs-section-titre">"Élève concerné"</h2>
                    <RechercheEleve
                        id="disc-eleve-q"
                        libelle="Rechercher un élève (nom, prénom ou classe)"
                        action="Ouvrir le dossier de"
                        on_choisir=Callback::new(move |e| eleve.set(Some(e)))
                    />
                </section>
            }
            .into_any(),
            Some((id, nom)) => view! {
                <p class="disc-eleve">
                    <strong>{nom}</strong>
                    " "
                    <Bouton
                        variante=Variante::Discret
                        taille=Taille::Compacte
                        libelle_accessible="Changer d'élève"
                        on_click=Callback::new(move |_| eleve.set(None))
                    >
                        "Changer d'élève"
                    </Bouton>
                </p>
                <DossierDiscipline eleve_id=id/>
            }
            .into_any(),
        }}
        {move || moteur.get().peut_gerer_discipline().then(|| view! { <EffacementSanctions/> })}
    }
}

#[component]
fn DossierDiscipline(eleve_id: Uuid) -> impl IntoView {
    let punitions =
        LocalResource::new(
            move || async move { session::client().punitions_eleve(eleve_id).await },
        );
    let types = LocalResource::new(|| async { session::client().types_punition().await });
    let sanctions =
        LocalResource::new(
            move || async move { session::client().sanctions_eleve(eleve_id).await },
        );
    let procedures =
        LocalResource::new(
            move || async move { session::client().procedures_eleve(eleve_id).await },
        );
    let recharger = Callback::new(move |_| {
        punitions.refetch();
        types.refetch();
        sanctions.refetch();
        procedures.refetch();
    });
    let ecriture = Ecriture::nouveau(recharger);
    let moteur = use_droits_moteur();
    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="ra-section" aria-labelledby="disc-punitions">
            <h2 id="disc-punitions" class="vs-section-titre">"Punitions"</h2>
            <p class="texte-attenue">
                "Mesures éducatives données par un enseignant ou la vie scolaire (retenue, devoir supplémentaire…). \
                 Elles ne sont pas des sanctions."
            </p>
            {move || match (punitions.get(), types.get()) {
                (None, _) => view! { <Squelette lignes=3 libelle="Chargement des punitions…"/> }.into_any(),
                (Some(Err(e)), _) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| punitions.refetch())/>
                }
                .into_any(),
                (Some(Ok(l)), t) => {
                    let t = t.and_then(Result::ok).unwrap_or_default();
                    view! { <ListePunitions punitions=l types=t ecriture=ecriture/> }.into_any()
                }
            }}
            {move || {
                moteur
                    .get()
                    .peut_donner_punition()
                    .then(|| match types.get() {
                        Some(Ok(t)) => view! { <FormulairePunition eleve_id=eleve_id types=t ecriture=ecriture/> }.into_any(),
                        _ => ().into_any(),
                    })
            }}
        </section>
        <section class="ra-section" aria-labelledby="disc-sanctions">
            <h2 id="disc-sanctions" class="vs-section-titre">"Sanctions"</h2>
            <p class="texte-attenue">
                "Mesures disciplinaires inscrites au dossier de l'élève. Consultation seulement : \
                 la procédure et son contradictoire sont suivies ci-dessous."
            </p>
            {move || match sanctions.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des sanctions…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| sanctions.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <ListeSanctions sanctions=l/> }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="disc-procedures">
            <h2 id="disc-procedures" class="vs-section-titre">"Procédures disciplinaires"</h2>
            {move || match procedures.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des procédures…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| procedures.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <ListeProcedures procedures=l ecriture=ecriture/> }.into_any(),
            }}
            {move || moteur.get().peut_gerer_discipline().then(|| view! { <FormulaireProcedure eleve_id=eleve_id ecriture=ecriture/> })}
        </section>
    }
}

#[component]
fn ListePunitions(
    punitions: Vec<PunitionDto>,
    types: Vec<TypeVsDto>,
    ecriture: Ecriture,
) -> impl IntoView {
    if punitions.is_empty() {
        return view! { <EtatVide titre="Aucune punition" message="Aucune punition enregistrée pour cet élève."/> }.into_any();
    }
    let moteur = use_droits_moteur();
    view! {
        <ul class="jvs-liste">
            {punitions
                .into_iter()
                .map(|p| {
                    let (id, statut) = (p.id, p.statut.clone());
                    let type_l = libelle_type(&types, p.type_id);
                    let detail = {
                        let mut d = format!("{} — {}", date_complete(p.jour), p.motif);
                        if let Some(e) = p.echeance {
                            d.push_str(&format!(" · à rendre le {}", date_complete(e)));
                        }
                        if let Some(m) = p.duree_minutes {
                            d.push_str(&format!(" · {m} min"));
                        }
                        d
                    };
                    view! {
                        <li class="jvs-ligne">
                            <div>
                                <strong>{type_l.clone()}</strong>
                                <span class="texte-attenue">{format!(" — {detail}")}</span>
                            </div>
                            <Badge libelle=libelle_statut_punition(&statut) ton=ton_statut_punition(&statut)/>
                            {move || {
                                (moteur.get().peut_donner_punition() && punition_cloturable(&statut))
                                    .then(|| {
                                        let a = format!("Marquer la punition {} comme effectuée", type_l);
                                        let b = format!("Annuler la punition {}", type_l);
                                        view! {
                                            <div class="ra-actions">
                                                <Bouton
                                                    variante=Variante::Secondaire
                                                    taille=Taille::Compacte
                                                    en_cours=ecriture.en_cours
                                                    libelle_accessible=a
                                                    on_click=Callback::new(move |_| {
                                                        ecriture.lancer(async move {
                                                            session::client().clore_punition(id, "effectuee").await?;
                                                            Ok("Punition marquée effectuée.".to_string())
                                                        });
                                                    })
                                                >
                                                    "Effectuée"
                                                </Bouton>
                                                <Bouton
                                                    variante=Variante::Discret
                                                    taille=Taille::Compacte
                                                    en_cours=ecriture.en_cours
                                                    libelle_accessible=b
                                                    on_click=Callback::new(move |_| {
                                                        ecriture.lancer(async move {
                                                            session::client().clore_punition(id, "annulee").await?;
                                                            Ok("Punition annulée.".to_string())
                                                        });
                                                    })
                                                >
                                                    "Annuler"
                                                </Bouton>
                                            </div>
                                        }
                                    })
                            }}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

#[component]
fn FormulairePunition(eleve_id: Uuid, types: Vec<TypeVsDto>, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let actifs: Vec<TypeVsDto> = types.into_iter().filter(|t| t.actif).collect();
    if actifs.is_empty() {
        let amorcer = Callback::new(move |_| {
            ecriture.lancer(async move {
                let t = session::client().amorcer_types_punition().await?;
                Ok(format!("{} type(s) de punition disponible(s).", t.len()))
            });
        });
        return view! {
            <p class="texte-attenue">"Aucun type de punition n'est défini pour l'établissement."</p>
            {move || moteur.get().peut_gerer_discipline().then(|| view! {
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=amorcer>
                    "Charger les types usuels"
                </Bouton>
            })}
        }
        .into_any();
    }
    let options: Vec<(String, String)> =
        std::iter::once((String::new(), "— choisir —".to_string()))
            .chain(actifs.iter().map(|t| (t.id.to_string(), t.libelle.clone())))
            .collect();
    let (type_id, jour, motif, echeance, duree) = (
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
    );
    // La date du jour est posée après l'hydratation (aucune date dans le HTML du serveur).
    Effect::new(move |_| {
        if jour.get_untracked().is_empty() {
            jour.set(aujourdhui().to_string());
        }
    });
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let err = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs.with(|e| e.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()))
        })
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisiePunition {
            type_id: type_id.get_untracked(),
            jour: jour.get_untracked(),
            motif: motif.get_untracked(),
            echeance: echeance.get_untracked(),
            duree: duree.get_untracked(),
        };
        match composer_punition(&saisie) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    session::client().donner_punition(eleve_id, &corps).await?;
                    Ok("Punition enregistrée.".to_string())
                });
                motif.set(String::new());
                echeance.set(String::new());
                duree.set(String::new());
            }
        }
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="disc-punition-nouvelle">
            <h3 id="disc-punition-nouvelle" class="jvs-sous-titre">"Donner une punition"</h3>
            <Selection id="pun-type" libelle="Type" valeur=type_id options=options requis=true erreur=err("pun-type")/>
            <ChampTexte id="pun-jour" libelle="Date" valeur=jour genre="date" requis=true erreur=err("pun-jour")/>
            <ZoneTexte id="pun-motif" libelle="Motif" valeur=motif requis=true erreur=err("pun-motif")/>
            <ChampTexte id="pun-echeance" libelle="À rendre pour le (facultatif)" valeur=echeance genre="date" erreur=err("pun-echeance")/>
            <ChampTexte id="pun-duree" libelle="Durée en minutes (facultatif)" valeur=duree genre="number" inputmode="numeric" erreur=err("pun-duree")/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer la punition"</Bouton>
        </form>
    }
    .into_any()
}

#[component]
fn ListeSanctions(sanctions: Vec<SanctionDto>) -> impl IntoView {
    if sanctions.is_empty() {
        return view! { <EtatVide titre="Aucune sanction" message="Aucune sanction au dossier."/> }
            .into_any();
    }
    view! {
        <ul class="jvs-liste">
            {sanctions
                .into_iter()
                .map(|s| {
                    let titre = s.type_label.clone().unwrap_or_else(|| "Sanction".into());
                    let mut detail = date_complete(s.jour);
                    if let Some(m) = s.motif_label.as_deref() {
                        detail.push_str(&format!(" — {m}"));
                    }
                    if let Some(d) = s.duree_minutes {
                        detail.push_str(&format!(" · {d} min"));
                    }
                    view! {
                        <li class="jvs-ligne">
                            <div>
                                <strong>{titre}</strong>
                                <span class="texte-attenue">{format!(" — {detail}")}</span>
                            </div>
                            <Badge libelle=libelle_statut_sanction(&s.statut)/>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

#[component]
fn ListeProcedures(procedures: Vec<ProcedureDto>, ecriture: Ecriture) -> impl IntoView {
    if procedures.is_empty() {
        return view! { <EtatVide titre="Aucune procédure" message="Aucune procédure disciplinaire pour cet élève."/> }.into_any();
    }
    let cible = RwSignal::new(None::<ProcedureDto>);
    let (o_obs, o_conv, o_dec, o_classer) = (
        RwSignal::new(false),
        RwSignal::new(false),
        RwSignal::new(false),
        RwSignal::new(false),
    );
    let moteur = use_droits_moteur();
    view! {
        <ul class="jvs-liste">
            {procedures
                .into_iter()
                .map(|p| {
                    let a = actions_procedure(&p);
                    let intitule = format!("{} — {}", libelle_instance(&p.instance), date_complete(p.engagee_le));
                    let pour = p.clone();
                    let mut etapes = Vec::new();
                    if let Some(o) = p.observations_recueillies_le {
                        etapes.push(format!("observations recueillies le {}", date_complete(o)));
                    }
                    if let (Some(c), Some(e)) = (p.conseil_le, p.convocation_envoyee_le) {
                        etapes.push(format!("convocation envoyée le {}, conseil le {}", date_complete(e), date_complete(c)));
                    }
                    if let (Some(dec), Some(le)) = (p.decision.as_deref(), p.decision_le) {
                        etapes.push(format!("décision du {} : {dec}", date_complete(le)));
                    }
                    view! {
                        <li class="jvs-ligne jvs-ligne--bloc">
                            <div>
                                <strong>{intitule.clone()}</strong>
                                <p class="texte-attenue">{p.faits.clone()}</p>
                                {(!etapes.is_empty()).then(|| view! {
                                    <ul class="ra-etapes-procedure">
                                        {etapes.into_iter().map(|e| view! { <li>{e}</li> }).collect_view()}
                                    </ul>
                                })}
                            </div>
                            <Badge libelle=libelle_statut_procedure(&p.statut) ton=ton_statut_procedure(&p.statut)/>
                            {move || {
                                moteur
                                    .get()
                                    .peut_gerer_discipline()
                                    .then(|| {
                                        let ouvrir = move |flag: RwSignal<bool>, p: ProcedureDto| {
                                            Callback::new(move |_| {
                                                cible.set(Some(p.clone()));
                                                flag.set(true);
                                            })
                                        };
                                        view! {
                                            <div class="ra-actions">
                                                {a.observations.then(|| view! {
                                                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=format!("Enregistrer les observations de l'élève : {intitule}") on_click=ouvrir(o_obs, pour.clone())>
                                                        "Observations de l'élève"
                                                    </Bouton>
                                                })}
                                                {a.convoquer.then(|| view! {
                                                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=format!("Convoquer le conseil : {intitule}") on_click=ouvrir(o_conv, pour.clone())>
                                                        "Convoquer le conseil"
                                                    </Bouton>
                                                })}
                                                {a.decider.then(|| view! {
                                                    <Bouton taille=Taille::Compacte libelle_accessible=format!("Enregistrer la décision : {intitule}") on_click=ouvrir(o_dec, pour.clone())>
                                                        "Décision"
                                                    </Bouton>
                                                })}
                                                {a.classer.then(|| view! {
                                                    <Bouton variante=Variante::Discret taille=Taille::Compacte libelle_accessible=format!("Classer sans suite : {intitule}") on_click=ouvrir(o_classer, pour.clone())>
                                                        "Classer sans suite"
                                                    </Bouton>
                                                })}
                                            </div>
                                        }
                                    })
                            }}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
        <DialogueObservations cible=cible ouvert=o_obs ecriture=ecriture/>
        <DialogueConvocation cible=cible ouvert=o_conv ecriture=ecriture/>
        <DialogueDecision cible=cible ouvert=o_dec ecriture=ecriture/>
        <Confirmation
            ouvert=o_classer
            id="conf-classer"
            titre="Classer la procédure sans suite ?"
            message="La procédure sera close ; aucune sanction n'en découle. Cette action est tracée."
            libelle_confirmer="Classer sans suite"
            danger=true
            on_confirmer=Callback::new(move |_| {
                if let Some(p) = cible.get_untracked() {
                    ecriture.lancer(async move {
                        session::client().classer_procedure(p.id).await?;
                        Ok("Procédure classée sans suite.".to_string())
                    });
                }
            })
        />
    }
    .into_any()
}

/// Champ date prérempli au jour (posé après l'hydratation).
fn date_du_jour() -> RwSignal<String> {
    let s = RwSignal::new(String::new());
    Effect::new(move |_| {
        if s.get_untracked().is_empty() {
            s.set(aujourdhui().to_string());
        }
    });
    s
}

fn erreur_de(
    erreurs: RwSignal<Vec<(String, String)>>,
    champ: &'static str,
) -> Signal<Option<String>> {
    Signal::derive(move || {
        erreurs.with(|e| e.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()))
    })
}

#[component]
fn DialogueObservations(
    cible: RwSignal<Option<ProcedureDto>>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let jour = date_du_jour();
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(p) = cible.get_untracked() else {
            return;
        };
        match lire_date(&jour.get_untracked()) {
            None => erreurs.set(vec![("obs-jour".into(), "Date invalide.".into())]),
            Some(le) => {
                erreurs.set(Vec::new());
                ouvert.set(false);
                ecriture.lancer(async move {
                    session::client().recueillir_observations(p.id, le).await?;
                    Ok("Observations de l'élève enregistrées.".to_string())
                });
            }
        }
    };
    view! {
        <Dialogue ouvert=ouvert titre="Observations de l'élève" id="dialogue-observations">
            <form class="ra-formulaire" on:submit=envoyer novalidate>
                <p>"Date à laquelle l'élève (et sa famille) ont pu présenter leurs observations."</p>
                <ChampTexte id="obs-jour" libelle="Date" valeur=jour genre="date" requis=true erreur=erreur_de(erreurs, "obs-jour")/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Annuler"</Bouton>
                    <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
fn DialogueConvocation(
    cible: RwSignal<Option<ProcedureDto>>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let envoi = date_du_jour();
    let conseil = RwSignal::new(String::new());
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(p) = cible.get_untracked() else {
            return;
        };
        match composer_convocation(
            &envoi.get_untracked(),
            &conseil.get_untracked(),
            p.delai_convocation_jours,
        ) {
            Err(e) => erreurs.set(e),
            Ok((d, c)) => {
                erreurs.set(Vec::new());
                ouvert.set(false);
                ecriture.lancer(async move {
                    session::client().convoquer_conseil(p.id, d, c).await?;
                    Ok("Conseil de discipline convoqué.".to_string())
                });
            }
        }
    };
    view! {
        <Dialogue ouvert=ouvert titre="Convoquer le conseil de discipline" id="dialogue-convocation">
            <form class="ra-formulaire" on:submit=envoyer novalidate>
                {move || cible.get().map(|p| view! {
                    <p>{format!("Délai minimal entre la convocation et le conseil : {} jour(s).", p.delai_convocation_jours)}</p>
                })}
                <ChampTexte id="conv-envoi" libelle="Convocation envoyée le" valeur=envoi genre="date" requis=true erreur=erreur_de(erreurs, "conv-envoi")/>
                <ChampTexte id="conv-conseil" libelle="Conseil le" valeur=conseil genre="date" requis=true erreur=erreur_de(erreurs, "conv-conseil")/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Annuler"</Bouton>
                    <Bouton soumettre=true en_cours=ecriture.en_cours>"Convoquer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
fn DialogueDecision(
    cible: RwSignal<Option<ProcedureDto>>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let texte = RwSignal::new(String::new());
    let jour = date_du_jour();
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(p) = cible.get_untracked() else {
            return;
        };
        match composer_decision(&texte.get_untracked(), &jour.get_untracked()) {
            Err(e) => erreurs.set(e),
            Ok((t, j)) => {
                erreurs.set(Vec::new());
                ouvert.set(false);
                texte.set(String::new());
                ecriture.lancer(async move {
                    session::client().decider_procedure(p.id, &t, j).await?;
                    Ok("Décision enregistrée.".to_string())
                });
            }
        }
    };
    view! {
        <Dialogue ouvert=ouvert titre="Décision" id="dialogue-decision">
            <form class="ra-formulaire" on:submit=envoyer novalidate>
                <ZoneTexte id="dec-texte" libelle="Décision" valeur=texte requis=true erreur=erreur_de(erreurs, "dec-texte")/>
                <ChampTexte id="dec-jour" libelle="Date de la décision" valeur=jour genre="date" requis=true erreur=erreur_de(erreurs, "dec-jour")/>
                <p class="texte-attenue">"La sanction éventuelle est prononcée séparément ; elle est rattachée à la procédure par le serveur."</p>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Annuler"</Bouton>
                    <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer la décision"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
fn FormulaireProcedure(eleve_id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let instance = RwSignal::new("chef_etablissement".to_string());
    let faits = RwSignal::new(String::new());
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let options: Vec<(String, String)> = INSTANCES
        .iter()
        .map(|(k, l)| (k.to_string(), l.to_string()))
        .collect();
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let s = SaisieProcedure {
            instance: instance.get_untracked(),
            faits: faits.get_untracked(),
        };
        match composer_procedure(eleve_id, &s) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                faits.set(String::new());
                ecriture.lancer(async move {
                    session::client().engager_procedure(&corps).await?;
                    Ok("Procédure disciplinaire engagée.".to_string())
                });
            }
        }
    };
    view! {
        <form class="ra-formulaire" on:submit=envoyer novalidate aria-labelledby="disc-proc-nouvelle">
            <h3 id="disc-proc-nouvelle" class="jvs-sous-titre">"Engager une procédure"</h3>
            <Alerte niveau=Niveau::Info titre="Le contradictoire est obligatoire">
                <p>"L'élève doit pouvoir présenter ses observations avant toute décision ; le serveur refuse une décision prématurée."</p>
            </Alerte>
            <Selection id="proc-instance" libelle="Instance" valeur=instance options=options erreur=erreur_de(erreurs, "proc-instance")/>
            <ZoneTexte id="proc-faits" libelle="Faits reprochés" valeur=faits requis=true erreur=erreur_de(erreurs, "proc-faits")/>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Engager la procédure"</Bouton>
        </form>
    }
}

/// Effacement des sanctions échues (règles de l'établissement) : idempotent, tracé.
#[component]
fn EffacementSanctions() -> impl IntoView {
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let confirmer = RwSignal::new(false);
    let lancer = Callback::new(move |_| {
        ecriture.lancer(async move {
            let r = session::client().effacer_sanctions(aujourdhui()).await?;
            Ok(format!(
                "{} sanction(s) examinée(s), {} effacée(s).",
                r.examinees,
                r.effacees.len()
            ))
        });
    });
    view! {
        <section class="ra-section" aria-labelledby="disc-effacement">
            <h2 id="disc-effacement" class="vs-section-titre">"Effacement des sanctions"</h2>
            <p class="texte-attenue">
                "Efface le texte libre des sanctions dont le délai est échu selon les règles de l'établissement \
                 (la trace d'existence est conservée). L'opération est idempotente et tracée."
            </p>
            <RetourEcriture ecriture=ecriture/>
            <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| confirmer.set(true))>
                "Effacer les sanctions échues"
            </Bouton>
            <Confirmation
                ouvert=confirmer
                id="conf-effacement"
                titre="Effacer les sanctions échues ?"
                message="Le texte libre des sanctions échues sera définitivement vidé. La liste des sanctions concernées n'est pas prévisualisée par le serveur."
                libelle_confirmer="Effacer"
                danger=true
                on_confirmer=lancer
            />
        </section>
    }
}
