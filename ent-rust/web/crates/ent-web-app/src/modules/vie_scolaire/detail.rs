//! Détail d'une absence (boîte de dialogue) : identité, période, justification déposée,
//! historique de l'élève, décisions avec étape de confirmation.

use std::collections::HashMap;

use ent_web_api::{Absence, ApiError, Retard};
use ent_web_ui::{
    use_notifications, Alerte, Badge, Bouton, Dialogue, Niveau, Squelette, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use super::gestionnaire_echec;
use super::modele::{appliquer_decision, decisions_possibles, memoriser, valider_note, Decision};
use crate::composants_metier::absences::{
    a_un_justificatif, chronologie, libelle_periode, libelle_retard, nom_eleve, nombre_de_jours,
    Evenement, StatutAbsence,
};
use crate::noyau::coquille::use_droits;

type DonneesHistorique = Option<(
    Result<Vec<Absence>, ApiError>,
    Result<Vec<Retard>, ApiError>,
)>;

/// Nombre d'événements de l'historique affichés.
const HISTORIQUE_MAX: usize = 8;

#[derive(Clone, Copy)]
struct EtatDetail {
    confirmation: RwSignal<Option<Decision>>,
    note: RwSignal<String>,
    erreur: RwSignal<Option<String>>,
    en_cours: RwSignal<bool>,
    ouvert: RwSignal<bool>,
    decidees: RwSignal<Vec<Absence>>,
    annonce: RwSignal<String>,
    absences: LocalResource<Result<Vec<Absence>, ApiError>>,
    historique: LocalResource<DonneesHistorique>,
    echec: Callback<ApiError>,
}

#[component]
pub fn DetailAbsence(
    selection: RwSignal<Option<Uuid>>,
    ouvert: RwSignal<bool>,
    toutes: Memo<Vec<Absence>>,
    classes: RwSignal<HashMap<Uuid, String>>,
    decidees: RwSignal<Vec<Absence>>,
    annonce: RwSignal<String>,
    absences: LocalResource<Result<Vec<Absence>, ApiError>>,
) -> impl IntoView {
    let courant = Memo::new(move |_| {
        selection
            .get()
            .and_then(|id| toutes.with(|t| t.iter().find(|a| a.id == id).cloned()))
    });
    let eleve_courant = Memo::new(move |_| courant.with(|c| c.as_ref().map(|a| a.eleve_id)));
    // Historique : seulement quand le détail est ouvert (pas de requêtes inutiles).
    let historique: LocalResource<DonneesHistorique> = LocalResource::new(move || {
        let eleve = if ouvert.get() {
            eleve_courant.get()
        } else {
            None
        };
        async move {
            let id = eleve?;
            let client = crate::noyau::session::client();
            let a = client.absences_eleve(id).await;
            let r = client.retards_eleve(id).await;
            Some((a, r))
        }
    });

    let confirmation = RwSignal::new(None::<Decision>);
    let note = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let echec = gestionnaire_echec(erreur);
    Effect::new(move |_| {
        if !ouvert.get() {
            confirmation.set(None);
            note.set(String::new());
            erreur.set(None);
        }
    });
    let etat = EtatDetail {
        confirmation,
        note,
        erreur,
        en_cours,
        ouvert,
        decidees,
        annonce,
        absences,
        historique,
        echec,
    };

    view! {
        <Dialogue ouvert=ouvert titre="Détail de l'absence" id="vs-detail">
            {move || match courant.get() {
                None => view! {
                    <p>"Cette absence n'est plus dans la liste."</p>
                    <div class="ui-dialogue__actions">
                        <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Fermer"</Bouton>
                    </div>
                }
                    .into_any(),
                Some(a) => {
                    let classe = classes.with(|c| c.get(&a.eleve_id).cloned());
                    view! { <ContenuDetail absence=a classe=classe etat=etat/> }.into_any()
                }
            }}
        </Dialogue>
    }
}

#[component]
fn ContenuDetail(absence: Absence, classe: Option<String>, etat: EtatDetail) -> impl IntoView {
    let droits = use_droits();
    let notifications = use_notifications();
    let statut = StatutAbsence::depuis(&absence);
    let nom = nom_eleve(&absence);
    let periode = libelle_periode(absence.debut, absence.fin);
    let jours = nombre_de_jours(absence.debut, absence.fin);
    let a_justificatif = a_un_justificatif(&absence);
    let motif_justif = absence.motif_justif.clone().unwrap_or_default();
    let decision_note = absence
        .decision_note
        .clone()
        .filter(|n| !n.trim().is_empty());
    let motif = absence.motif.clone();
    let absence_courante = StoredValue::new(absence.clone());

    let lancer = move |d: Decision| {
        let a = absence_courante.get_value();
        match valider_note(d, &etat.note.get_untracked()) {
            Err(m) => etat.erreur.set(Some(m)),
            Ok(n) => {
                etat.erreur.set(None);
                etat.en_cours.set(true);
                spawn_local(async move {
                    let r = crate::noyau::session::client()
                        .decider_absence(a.id, d.api(), n)
                        .await;
                    etat.en_cours.set(false);
                    match r {
                        Ok(nouvelle) => {
                            let nouvelle = appliquer_decision(&a, nouvelle);
                            etat.decidees.update(|v| memoriser(v, nouvelle));
                            let texte = d.resultat(&nom_eleve(&a));
                            notifications.succes(texte.clone());
                            etat.annonce.set(texte);
                            etat.ouvert.set(false);
                            etat.absences.refetch();
                        }
                        Err(e) => etat.echec.run(e),
                    }
                });
            }
        }
    };

    let decisions = decisions_possibles(statut, droits.get_untracked());
    let message_confirmation = {
        let nom = nom.clone();
        let periode = periode.clone();
        move |d: Decision| d.message(&nom, &periode)
    };

    view! {
        <div class="vs-detail">
            <div class="vs-detail__entete">
                <div>
                    <p class="vs-detail__nom">{nom.clone()}</p>
                    <p class="vs-detail__classe">{classe.unwrap_or_else(|| "Classe non précisée".into())}</p>
                </div>
                <Badge libelle=statut.libelle() ton=statut.ton()/>
            </div>
            <dl class="vs-detail__liste">
                <div>
                    <dt>"Période"</dt>
                    <dd>{format!("{periode} ({jours} jour{})", if jours > 1 { "s" } else { "" })}</dd>
                </div>
                <div>
                    <dt>"Motif relevé"</dt>
                    <dd>{motif}</dd>
                </div>
                <div>
                    <dt>"Justificatif déposé"</dt>
                    <dd>
                        {if a_justificatif {
                            motif_justif
                        } else {
                            "Aucun justificatif déposé par la famille.".to_string()
                        }}
                    </dd>
                </div>
                {decision_note
                    .map(|n| view! {
                        <div>
                            <dt>"Note de décision"</dt>
                            <dd>{n}</dd>
                        </div>
                    })}
            </dl>
            <p class="vs-note">"Le serveur n'enregistre que le motif écrit : pas de pièce jointe pour l'instant."</p>

            <section aria-labelledby="vs-historique-titre">
                <h3 id="vs-historique-titre" class="vs-detail__sous-titre">"Historique de l'élève"</h3>
                <Historique historique=etat.historique/>
            </section>

            {move || etat.erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}

            {move || match etat.confirmation.get() {
                None => view! {
                    <div class="ui-dialogue__actions">
                        <Bouton variante=Variante::Discret on_click=Callback::new(move |_| etat.ouvert.set(false))>"Fermer"</Bouton>
                        {decisions
                            .iter()
                            .map(|d| {
                                let d = *d;
                                view! {
                                    <Bouton
                                        variante=if d.danger() { Variante::Danger } else { Variante::Primaire }
                                        on_click=Callback::new(move |_| {
                                            etat.erreur.set(None);
                                            etat.confirmation.set(Some(d));
                                        })
                                    >
                                        {d.libelle(a_justificatif)}
                                    </Bouton>
                                }
                            })
                            .collect_view()}
                    </div>
                    {decisions.is_empty().then(|| view! {
                        <p class="vs-note">"Aucune décision n'est possible sur cette absence avec votre profil."</p>
                    })}
                }
                    .into_any(),
                Some(d) => view! {
                    <div class="vs-confirmation" role="group" aria-labelledby="vs-confirmation-titre">
                        <h3 id="vs-confirmation-titre" class="vs-detail__sous-titre">{d.titre_confirmation()}</h3>
                        <p>{message_confirmation(d)}</p>
                        <ZoneTexte
                            id="vs-note"
                            libelle=d.libelle_note()
                            valeur=etat.note
                            requis=d.note_obligatoire()
                            desactive=etat.en_cours
                        />
                        <div class="ui-dialogue__actions">
                            <Bouton
                                variante=Variante::Secondaire
                                desactive=etat.en_cours
                                on_click=Callback::new(move |_| {
                                    etat.erreur.set(None);
                                    etat.confirmation.set(None);
                                })
                            >
                                "Retour"
                            </Bouton>
                            <Bouton
                                variante=if d.danger() { Variante::Danger } else { Variante::Primaire }
                                en_cours=etat.en_cours
                                on_click=Callback::new(move |_| lancer(d))
                            >
                                {d.libelle_confirmer()}
                            </Bouton>
                        </div>
                    </div>
                }
                    .into_any(),
            }}
        </div>
    }
}

#[component]
fn Historique(historique: LocalResource<DonneesHistorique>) -> impl IntoView {
    view! {
        {move || match historique.get() {
            None | Some(None) => view! { <Squelette lignes=3 libelle="Chargement de l'historique…"/> }.into_any(),
            Some(Some((absences, retards))) => {
                if let Err(ApiError::Interdit(_)) = absences {
                    return view! { <p class="texte-attenue">"Historique non accessible avec votre profil."</p> }.into_any();
                }
                let echec_partiel = absences.is_err() || retards.is_err();
                let evenements = chronologie(absences.unwrap_or_default(), retards.unwrap_or_default());
                view! {
                    {echec_partiel
                        .then(|| view! { <p class="texte-attenue">"Une partie de l'historique n'a pas pu être chargée."</p> })}
                    {if evenements.is_empty() {
                        view! { <p class="texte-attenue">"Aucune absence ni retard enregistré."</p> }.into_any()
                    } else {
                        let total = evenements.len();
                        view! {
                            <ul class="vs-historique">
                                {evenements
                                    .into_iter()
                                    .take(HISTORIQUE_MAX)
                                    .map(|e| match e {
                                        Evenement::Absence(a) => {
                                            let s = StatutAbsence::depuis(&a);
                                            view! {
                                                <li>
                                                    <span class="vs-historique__date">{libelle_periode(a.debut, a.fin)}</span>
                                                    <span>"Absence"</span>
                                                    <Badge libelle=s.libelle() ton=s.ton()/>
                                                </li>
                                            }.into_any()
                                        }
                                        Evenement::Retard(r) => view! {
                                            <li>
                                                <span class="vs-historique__date">{libelle_periode(r.jour, r.jour)}</span>
                                                <span>{libelle_retard(&r)}</span>
                                            </li>
                                        }.into_any(),
                                    })
                                    .collect_view()}
                            </ul>
                            {(total > HISTORIQUE_MAX)
                                .then(|| view! { <p class="texte-attenue">{format!("{} événement(s) plus ancien(s) non affichés.", total - HISTORIQUE_MAX)}</p> })}
                        }
                            .into_any()
                    }}
                }
                    .into_any()
            }
        }}
    }
}
