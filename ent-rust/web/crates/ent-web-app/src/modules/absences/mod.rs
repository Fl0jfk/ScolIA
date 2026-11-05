//! Absences de la famille (par enfant, avec justification) et de l'élève (lecture seule).
//!
//! Routes existantes d'`ent-api` : `GET /famille/enfants`, `GET /eleves/{id}/absences` et
//! `/retards` (parent avec `peut_voir_vs`, ou l'élève lui-même), `GET /famille/enfants/{id}/
//! resume` (absences à justifier quand le lien ne permet pas de lire l'historique),
//! `GET /motifs-absence`, `POST /absences/{id}/justifier` et `/annuler-justificatif`.
//! Les droits (lien parent, drapeaux) sont vérifiés par le serveur ; l'écran ne propose que
//! ce qui sera accepté et traite les refus.
//!
//! Limite de l'API : la justification est un **texte** (`motif_justif`) ; il n'existe pas de
//! route de dépôt de pièce jointe pour les absences.

pub mod modele;

use ent_web_api::{Absence, ApiError, Enfant, Moi, MotifAbsence, Retard};
use ent_web_ui::{
    use_notifications, AccesRefuse, Alerte, Annonce, Badge, Bouton, Chargement, Dialogue,
    EnTetePage, EtatVide, Niveau, OptionSegment, Segmente, Selection, Squelette, Taille, Variante,
    ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::absences::{
    libelle_periode, libelle_retard, nombre_de_jours, StatutAbsence,
};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::{reconnexion, session};
use modele::{
    a_justifier, acces, composer_justification, enfant_choisi, message_sans_droit_de_justifier,
    nom_enfant, options_motifs, peut_justifier, peut_retirer, recentes_d_abord, Acces,
};

#[component]
pub fn PageAbsences() -> impl IntoView {
    let droits = use_droits();
    let ctx = session::use_session();
    view! {
        <Title text="Absences"/>
        <EnTetePage
            titre="Absences".to_string()
            sous_titre="Absences et retards.".to_string()
        />
        {move || match ctx.moi.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des absences…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| ctx.moi.refetch())/> }.into_any(),
            Some(Ok(moi)) => {
                let d = droits.get();
                if d.portail_famille {
                    view! { <VueFamille/> }.into_any()
                } else if d.mes_absences {
                    view! { <VueEleve moi=moi/> }.into_any()
                } else {
                    view! { <AccesRefuse message="Cet écran est réservé aux familles et aux élèves."/> }.into_any()
                }
            }
        }}
    }
}

// ---------------------------------------------------------------------------
// Famille
// ---------------------------------------------------------------------------

#[component]
fn VueFamille() -> impl IntoView {
    let enfants = LocalResource::new(|| async { session::client().enfants_famille().await });
    // Motifs proposés pour justifier : chargés une fois ; en cas d'échec seul « Autre » reste.
    let motifs = LocalResource::new(|| async { session::client().motifs_absence().await });
    view! {
        {move || match enfants.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de vos enfants…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| enfants.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide
                    titre="Aucun enfant rattaché"
                    message="Aucun enfant n'est lié à votre compte. Contactez le secrétariat de l'établissement."
                />
            }
                .into_any(),
            Some(Ok(l)) => view! { <ContenuFamille enfants=l motifs=motifs/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuFamille(
    enfants: Vec<Enfant>,
    motifs: LocalResource<Result<Vec<MotifAbsence>, ApiError>>,
) -> impl IntoView {
    let liste = StoredValue::new(enfants.clone());
    let choix = RwSignal::new(
        enfant_choisi(&enfants, None)
            .map(|i| i.to_string())
            .unwrap_or_default(),
    );
    let courant = Memo::new(move |_| {
        let demande = choix.get().parse::<Uuid>().ok();
        liste.with_value(|l| {
            let id = enfant_choisi(l, demande)?;
            l.iter().find(|e| e.id == id).cloned()
        })
    });
    let selecteur = (enfants.len() > 1).then(|| {
        let options: Vec<(String, String)> = enfants
            .iter()
            .map(|e| (e.id.to_string(), nom_enfant(e)))
            .collect();
        if enfants.len() <= 3 {
            let options = options
                .into_iter()
                .map(|(v, l)| OptionSegment::new(v, l))
                .collect();
            view! { <Segmente nom="enfant" legende="Enfant" options=options valeur=choix/> }
                .into_any()
        } else {
            view! { <Selection id="enfant" libelle="Enfant" valeur=choix options=options/> }
                .into_any()
        }
    });
    view! {
        {selecteur.map(|s| view! { <div class="abs-enfants">{s}</div> })}
        {move || courant.get().map(|e| view! { <FicheEnfant enfant=e motifs=motifs/> })}
    }
}

type Donnees = Result<Vec<Absence>, ApiError>;

#[component]
fn FicheEnfant(
    enfant: Enfant,
    motifs: LocalResource<Result<Vec<MotifAbsence>, ApiError>>,
) -> impl IntoView {
    let niveau = acces(&enfant);
    let id = enfant.id;
    let prenom = enfant.prenom.clone();
    let prenom_dialogue = enfant.prenom.clone();
    let titre_enfant = format!(
        "{}{}",
        nom_enfant(&enfant),
        enfant
            .classe
            .as_deref()
            .filter(|c| !c.is_empty())
            .map(|c| format!(" · {c}"))
            .unwrap_or_default()
    );
    let notifications = use_notifications();
    let annonce = RwSignal::new(String::new());

    let absences: LocalResource<Donnees> = LocalResource::new(move || async move {
        let client = session::client();
        match niveau {
            Acces::Complet => client.absences_eleve(id).await,
            Acces::JustificationSeule => client
                .resume_enfant(id)
                .await
                .map(|r| r.absences_a_justifier),
            Acces::Aucun => Ok(Vec::new()),
        }
    });
    let retards: LocalResource<Result<Vec<Retard>, ApiError>> =
        LocalResource::new(move || async move {
            if niveau == Acces::Complet {
                session::client().retards_eleve(id).await
            } else {
                Ok(Vec::new())
            }
        });

    // Justification.
    let ouvert = RwSignal::new(false);
    let cible = RwSignal::new(None::<Absence>);
    let motif_code = RwSignal::new(String::new());
    let commentaire = RwSignal::new(String::new());
    let erreur_champ = RwSignal::new(None::<String>);
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let echec = gestionnaire_echec(erreur);
    let echec_dialogue = gestionnaire_echec(erreur_champ);
    Effect::new(move |_| {
        if ouvert.get() {
            commentaire.set(String::new());
            erreur_champ.set(None);
            let premier = options_motifs(
                &motifs
                    .get_untracked()
                    .and_then(Result::ok)
                    .unwrap_or_default(),
            )
            .into_iter()
            .next()
            .map(|(c, _)| c)
            .unwrap_or_default();
            motif_code.set(premier);
        }
    });

    let justifier = Callback::new(move |a: Absence| {
        erreur.set(None);
        cible.set(Some(a));
        ouvert.set(true);
    });
    let envoyer = move || {
        let Some(a) = cible.get_untracked() else {
            return;
        };
        let liste = motifs
            .get_untracked()
            .and_then(Result::ok)
            .unwrap_or_default();
        match composer_justification(
            &liste,
            &motif_code.get_untracked(),
            &commentaire.get_untracked(),
        ) {
            Err(m) => erreur_champ.set(Some(m)),
            Ok(texte) => {
                erreur_champ.set(None);
                en_cours.set(true);
                spawn_local(async move {
                    let r = session::client().justifier_absence(a.id, texte).await;
                    en_cours.set(false);
                    match r {
                        Ok(_) => {
                            ouvert.set(false);
                            let t = "Justification envoyée : la vie scolaire va l'examiner.";
                            notifications.succes(t);
                            annonce.set(t.to_string());
                            absences.refetch();
                        }
                        Err(e) => echec_dialogue.run(e),
                    }
                });
            }
        }
    };
    let retirer = Callback::new(move |a: Absence| {
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client().retirer_justificatif(a.id).await;
            en_cours.set(false);
            match r {
                Ok(_) => {
                    let t = "Justification retirée.";
                    notifications.succes(t);
                    annonce.set(t.to_string());
                    absences.refetch();
                }
                Err(e) => echec.run(e),
            }
        });
    });

    let enfant_pour_liste = enfant.clone();
    let peut_agir = enfant.peut_justifier_absence;
    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>
        <p class="abs-enfant">{titre_enfant}</p>
        {match niveau {
            Acces::Aucun => view! {
                <Alerte niveau=Niveau::Info titre=format!("Vous n'avez pas accès aux absences de {prenom}.")>
                    <p>"Votre lien avec cet enfant ne vous permet ni de consulter ni de justifier ses absences. Adressez-vous à la vie scolaire pour le modifier."</p>
                </Alerte>
            }
                .into_any(),
            Acces::JustificationSeule => view! {
                <Alerte niveau=Niveau::Info titre=format!("Seules les absences à justifier de {prenom} sont affichées.")>
                    <p>"Votre lien vous permet de justifier les absences, mais pas de consulter l'historique des absences et des retards."</p>
                </Alerte>
            }
                .into_any(),
            Acces::Complet if !peut_agir => view! {
                <Alerte niveau=Niveau::Info titre="Consultation seule">
                    <p>{message_sans_droit_de_justifier(&prenom)}</p>
                </Alerte>
            }
                .into_any(),
            Acces::Complet => ().into_any(),
        }}
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}

        {(niveau != Acces::Aucun)
            .then(|| view! {
                <section class="abs-section" aria-labelledby="abs-titre">
                    <h2 id="abs-titre" class="vs-section-titre">"Absences"</h2>
                    {move || match absences.get() {
                        None => view! { <Squelette lignes=3 libelle="Chargement des absences…"/> }.into_any(),
                        Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| absences.refetch())/> }.into_any(),
                        Some(Ok(l)) => {
                            let nb = a_justifier(&l);
                            view! {
                                {(nb > 0 && peut_agir)
                                    .then(|| view! {
                                        <p class="abs-a-justifier" role="status">
                                            {format!("{nb} absence{} à justifier", if nb > 1 { "s" } else { "" })}
                                        </p>
                                    })}
                                <ListeAbsences
                                    absences=l
                                    enfant=Some(enfant_pour_liste.clone())
                                    on_justifier=justifier
                                    on_retirer=retirer
                                    occupe=Signal::derive(move || en_cours.get())
                                />
                            }
                                .into_any()
                        }
                    }}
                </section>
            })}

        {(niveau == Acces::Complet)
            .then(|| view! {
                <section class="abs-section" aria-labelledby="ret-titre">
                    <h2 id="ret-titre" class="vs-section-titre">"Retards"</h2>
                    <SectionRetards retards=retards/>
                </section>
            })}

        <Dialogue ouvert=ouvert titre="Justifier l'absence" id="abs-justifier">
            <p class="abs-dialogue__periode">
                {move || cible.get().map(|a| format!("Absence de {} : {}", prenom_dialogue, libelle_periode(a.debut, a.fin))).unwrap_or_default()}
            </p>
            {move || match motifs.get() {
                None => view! { <Chargement libelle="Chargement des motifs…"/> }.into_any(),
                Some(r) => {
                    let options = options_motifs(&r.unwrap_or_default());
                    view! {
                        <Selection id="abs-motif" libelle="Motif" valeur=motif_code options=options requis=true desactive=en_cours/>
                    }
                        .into_any()
                }
            }}
            <ZoneTexte
                id="abs-commentaire"
                libelle="Commentaire"
                valeur=commentaire
                aide="Facultatif, sauf pour « Autre motif »."
                erreur=erreur_champ
                desactive=en_cours
            />
            <p class="vs-note">
                "Pièce jointe : le dépôt de fichier n'est pas encore disponible. Décrivez le motif ; si un document est exigé, remettez-le à la vie scolaire."
            </p>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire desactive=en_cours on_click=Callback::new(move |_| ouvert.set(false))>
                    "Annuler"
                </Bouton>
                <Bouton variante=Variante::Primaire en_cours=en_cours on_click=Callback::new(move |_| envoyer())>
                    "Envoyer la justification"
                </Bouton>
            </div>
        </Dialogue>
    }
}

#[component]
fn ListeAbsences(
    absences: Vec<Absence>,
    /// Enfant dont le lien décide des actions ; `None` : lecture seule (élève).
    enfant: Option<Enfant>,
    on_justifier: Callback<Absence>,
    on_retirer: Callback<Absence>,
    occupe: Signal<bool>,
) -> impl IntoView {
    if absences.is_empty() {
        return view! {
            <EtatVide titre="Aucune absence" message="Aucune absence n'est enregistrée."/>
        }
        .into_any();
    }
    let absences = recentes_d_abord(absences);
    view! {
        <ul class="abs-liste">
            {absences
                .into_iter()
                .map(|a| {
                    let statut = StatutAbsence::depuis(&a);
                    let jours = nombre_de_jours(a.debut, a.fin);
                    let justifiable = enfant.as_ref().is_some_and(|e| peut_justifier(e, &a));
                    let retirable = enfant.as_ref().is_some_and(|e| peut_retirer(e, &a));
                    let periode = libelle_periode(a.debut, a.fin);
                    let pour_justifier = a.clone();
                    let pour_retirer = a.clone();
                    let libelle_j = format!("Justifier l'absence : {periode}");
                    let libelle_r = format!("Retirer ma justification : {periode}");
                    let justif = a.motif_justif.clone().filter(|m| !m.trim().is_empty());
                    let decision = a.decision_note.clone().filter(|m| !m.trim().is_empty());
                    view! {
                        <li class="abs-carte" data-statut=statut.code()>
                            <div class="abs-carte__tete">
                                <p class="abs-carte__periode">
                                    {periode}
                                    {(jours > 1).then(|| view! { <span class="abs-carte__jours">{format!(" ({jours} jours)")}</span> })}
                                </p>
                                <Badge libelle=statut.libelle() ton=statut.ton()/>
                            </div>
                            <p class="abs-carte__detail">{format!("Motif relevé : {}", a.motif)}</p>
                            {justif.map(|m| view! { <p class="abs-carte__detail">{format!("Justification déposée : {m}")}</p> })}
                            {decision.map(|m| view! { <p class="abs-carte__detail">{format!("Réponse de la vie scolaire : {m}")}</p> })}
                            {(statut == StatutAbsence::JustificatifEnAttente)
                                .then(|| view! { <p class="abs-carte__detail texte-attenue">"En attente de la décision de la vie scolaire."</p> })}
                            {justifiable
                                .then(|| view! {
                                    <div class="abs-carte__actions">
                                        <Bouton
                                            taille=Taille::Compacte
                                            desactive=occupe
                                            libelle_accessible=libelle_j
                                            on_click=Callback::new(move |_| on_justifier.run(pour_justifier.clone()))
                                        >
                                            "Justifier"
                                        </Bouton>
                                    </div>
                                })}
                            {retirable
                                .then(|| view! {
                                    <div class="abs-carte__actions">
                                        <Bouton
                                            variante=Variante::Secondaire
                                            taille=Taille::Compacte
                                            desactive=occupe
                                            libelle_accessible=libelle_r
                                            on_click=Callback::new(move |_| on_retirer.run(pour_retirer.clone()))
                                        >
                                            "Retirer ma justification"
                                        </Bouton>
                                    </div>
                                })}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

#[component]
fn SectionRetards(retards: LocalResource<Result<Vec<Retard>, ApiError>>) -> impl IntoView {
    view! {
        {move || match retards.get() {
            None => view! { <Squelette lignes=2 libelle="Chargement des retards…"/> }.into_any(),
            Some(Err(ApiError::Interdit(_))) => view! { <p class="texte-attenue">"Les retards ne sont pas accessibles avec votre profil."</p> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| retards.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun retard" message="Aucun retard n'est enregistré."/> }.into_any(),
            Some(Ok(mut l)) => {
                l.sort_by_key(|r| std::cmp::Reverse(r.jour));
                view! {
                    <ul class="abs-liste">
                        {l
                            .into_iter()
                            .map(|r| view! {
                                <li class="abs-carte">
                                    <div class="abs-carte__tete">
                                        <p class="abs-carte__periode">{libelle_periode(r.jour, r.jour)}</p>
                                        <Badge libelle=libelle_retard(&r) ton=ent_web_ui::TonEtat::Avertissement/>
                                    </div>
                                    {r.motif.clone().filter(|m| !m.trim().is_empty()).map(|m| view! { <p class="abs-carte__detail">{format!("Motif : {m}")}</p> })}
                                </li>
                            })
                            .collect_view()}
                    </ul>
                }
                    .into_any()
            }
        }}
    }
}

/// Même réaction que l'écran de vie scolaire : session expirée → connexion, sinon message.
fn gestionnaire_echec(erreur: RwSignal<Option<String>>) -> Callback<ApiError> {
    Callback::new(move |e: ApiError| match e {
        ApiError::NonConnecte => {
            reconnexion::signaler_session_perdue();
            erreur.set(Some(reconnexion::MESSAGE_SESSION_EXPIREE.into()));
        }
        autre => erreur.set(Some(autre.message_utilisateur())),
    })
}

// ---------------------------------------------------------------------------
// Élève (lecture seule)
// ---------------------------------------------------------------------------

#[component]
fn VueEleve(moi: Moi) -> impl IntoView {
    let Some(eleve_id) = moi.eleve_id else {
        return view! {
            <EtatVide
                titre="Compte non lié à une fiche élève"
                message="Vos absences ne peuvent pas être affichées. Contactez le secrétariat."
            />
        }
        .into_any();
    };
    let absences: LocalResource<Donnees> =
        LocalResource::new(move || async move { session::client().absences_eleve(eleve_id).await });
    let retards: LocalResource<Result<Vec<Retard>, ApiError>> =
        LocalResource::new(move || async move { session::client().retards_eleve(eleve_id).await });
    view! {
        <section class="abs-section" aria-labelledby="abs-titre">
            <h2 id="abs-titre" class="vs-section-titre">"Mes absences"</h2>
            {move || match absences.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement de vos absences…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| absences.refetch())/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ListeAbsences
                        absences=l
                        enfant=None
                        on_justifier=Callback::new(|_| {})
                        on_retirer=Callback::new(|_| {})
                        occupe=Signal::derive(|| false)
                    />
                }
                    .into_any(),
            }}
            <p class="vs-note">"Vos responsables légaux justifient les absences ; vous les consultez ici."</p>
        </section>
        <section class="abs-section" aria-labelledby="ret-titre">
            <h2 id="ret-titre" class="vs-section-titre">"Mes retards"</h2>
            <SectionRetards retards=retards/>
        </section>
    }
    .into_any()
}
