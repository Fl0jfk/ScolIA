//! Signaux d'absence répétée : prendre en charge, clore (note obligatoire, confirmée).

use ent_web_api::{ApiError, SignalAbsence};
use ent_web_ui::{
    use_notifications, Alerte, Badge, Bouton, Dialogue, EtatVide, Niveau, Squelette, Taille,
    Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use super::gestionnaire_echec;
use super::modele::{
    actions_signal, libelle_progression, remplacer_signal, signaux_actifs, valider_note_signal,
    StatutSignal,
};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

fn nom_signal(s: &SignalAbsence) -> String {
    s.eleve_label
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("Élève")
        .to_string()
}

#[component]
pub fn SectionSignaux(
    signaux: LocalResource<Result<Vec<SignalAbsence>, ApiError>>,
    /// Signaux non clos affichés (partagés avec le tableau de bord de la page).
    liste: RwSignal<Vec<SignalAbsence>>,
    annonce: RwSignal<String>,
) -> impl IntoView {
    let droits = use_droits();
    let notifications = use_notifications();
    Effect::new(move |_| {
        if let Some(Ok(l)) = signaux.get() {
            liste.set(signaux_actifs(l));
        }
    });

    let erreur = RwSignal::new(None::<String>);
    let echec = gestionnaire_echec(erreur);
    // Signal en cours de traitement (un seul à la fois : pas de double envoi).
    let occupe = RwSignal::new(None::<Uuid>);

    // Clôture.
    let a_clore = RwSignal::new(None::<SignalAbsence>);
    let clore_ouvert = RwSignal::new(false);
    let note = RwSignal::new(String::new());
    let erreur_note = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        if !clore_ouvert.get() {
            note.set(String::new());
            erreur_note.set(None);
        }
    });

    let prendre = Callback::new(move |s: SignalAbsence| {
        if occupe.get_untracked().is_some() {
            return;
        }
        erreur.set(None);
        occupe.set(Some(s.id));
        spawn_local(async move {
            let r = session::client().prendre_signal(s.id).await;
            occupe.set(None);
            match r {
                Ok(nouveau) => {
                    liste.update(|l| remplacer_signal(l, nouveau));
                    let texte = format!("Signal de {} pris en charge.", nom_signal(&s));
                    notifications.succes(texte.clone());
                    annonce.set(texte);
                }
                Err(e) => echec.run(e),
            }
        });
    });
    let demander_cloture = Callback::new(move |s: SignalAbsence| {
        erreur.set(None);
        a_clore.set(Some(s));
        clore_ouvert.set(true);
    });
    let confirmer_cloture = move || {
        let Some(s) = a_clore.get_untracked() else {
            return;
        };
        match valider_note_signal(&note.get_untracked()) {
            Err(m) => erreur_note.set(Some(m)),
            Ok(n) => {
                erreur_note.set(None);
                occupe.set(Some(s.id));
                spawn_local(async move {
                    let r = session::client().clore_signal(s.id, n).await;
                    occupe.set(None);
                    match r {
                        Ok(nouveau) => {
                            liste.update(|l| remplacer_signal(l, nouveau));
                            let texte = format!("Signal de {} clos.", nom_signal(&s));
                            notifications.succes(texte.clone());
                            annonce.set(texte);
                            clore_ouvert.set(false);
                        }
                        Err(e) => match e {
                            ApiError::NonConnecte => echec.run(e),
                            autre => erreur_note.set(Some(autre.message_utilisateur())),
                        },
                    }
                });
            }
        }
    };
    let en_cours = Signal::derive(move || occupe.get().is_some());

    view! {
        <section class="vs-signaux" aria-labelledby="vs-signaux-titre">
            <h2 id="vs-signaux-titre" class="vs-section-titre">"Absences répétées"</h2>
            <p class="texte-attenue">
                "Élèves dont le nombre d'absences depuis la rentrée a atteint le seuil de l'établissement."
            </p>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {move || match signaux.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des signaux…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| signaux.refetch())/> }.into_any(),
                Some(Ok(_)) => {
                    let l = liste.get();
                    if l.is_empty() {
                        view! { <EtatVide titre="Aucun signal en cours" message="Aucun élève n'a atteint le seuil d'absences."/> }.into_any()
                    } else {
                        view! {
                            <ul class="vs-signal-liste">
                                {l
                                    .into_iter()
                                    .map(|s| {
                                        let statut = StatutSignal::depuis(&s);
                                        let (peut_prendre, peut_clore) = actions_signal(&s, droits.get_untracked());
                                        let nom = nom_signal(&s);
                                        let progression = libelle_progression(&s);
                                        let pour_prendre = s.clone();
                                        let pour_clore = s.clone();
                                        let libelle_prendre = format!("Prendre en charge le signal de {nom}");
                                        let libelle_clore = format!("Clore le signal de {nom}");
                                        let note_vs = s.note_vs.clone().filter(|n| !n.trim().is_empty());
                                        let traite_par = s.traite_par_label.clone();
                                        view! {
                                            <li class="vs-signal">
                                                <div class="vs-signal__texte">
                                                    <p class="vs-signal__nom">{nom}</p>
                                                    <p class="vs-signal__detail">{progression}</p>
                                                    {traite_par.map(|t| view! { <p class="vs-signal__detail">{format!("Pris en charge par {t}")}</p> })}
                                                    {note_vs.map(|n| view! { <p class="vs-signal__detail">{format!("Note : {n}")}</p> })}
                                                </div>
                                                <Badge libelle=statut.libelle() ton=statut.ton()/>
                                                <div class="vs-signal__actions">
                                                    {peut_prendre
                                                        .then(|| view! {
                                                            <Bouton
                                                                variante=Variante::Secondaire
                                                                taille=Taille::Compacte
                                                                desactive=en_cours
                                                                libelle_accessible=libelle_prendre
                                                                on_click=Callback::new(move |_| prendre.run(pour_prendre.clone()))
                                                            >
                                                                "Prendre en charge"
                                                            </Bouton>
                                                        })}
                                                    {peut_clore
                                                        .then(|| view! {
                                                            <Bouton
                                                                variante=Variante::Secondaire
                                                                taille=Taille::Compacte
                                                                desactive=en_cours
                                                                libelle_accessible=libelle_clore
                                                                on_click=Callback::new(move |_| demander_cloture.run(pour_clore.clone()))
                                                            >
                                                                "Clore"
                                                            </Bouton>
                                                        })}
                                                </div>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        }
                            .into_any()
                    }
                }
            }}
        </section>

        <Dialogue ouvert=clore_ouvert titre="Clore le signal ?" id="vs-signal-clore">
            <p>
                {move || {
                    a_clore
                        .get()
                        .map(|s| format!(
                            "Le signal d'absences répétées de {} sera clos. Indiquez ce qui a été fait ; la note reste dans le dossier.",
                            nom_signal(&s),
                        ))
                        .unwrap_or_default()
                }}
            </p>
            <ZoneTexte
                id="vs-signal-note"
                libelle="Note de clôture"
                valeur=note
                requis=true
                erreur=erreur_note
                desactive=en_cours
            />
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire desactive=en_cours on_click=Callback::new(move |_| clore_ouvert.set(false))>
                    "Annuler"
                </Bouton>
                <Bouton variante=Variante::Primaire en_cours=en_cours on_click=Callback::new(move |_| confirmer_cloture())>
                    "Clore le signal"
                </Bouton>
            </div>
        </Dialogue>
    }
}
