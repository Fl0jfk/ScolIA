//! Absences de la famille (par enfant, avec « donner le motif ») et de l'élève (lecture seule).
//!
//! Socle 12, AS14c : les absences sont présentées **par épisode** (`GET /assiduite/eleves/{id}/episodes`,
//! libellé à liste fermée) et le motif est donné par `POST /assiduite/episodes/{id}/justificatifs`
//! (voir `episodes.rs`). Restent sur les routes existantes : `GET /famille/enfants`, `GET
//! /eleves/{id}/retards`, la déclaration d'absence à l'avance (`DialogueDeclaration`).
//! Les droits (lien parent, drapeaux, majorité de l'élève) sont vérifiés par le serveur ; l'écran ne
//! propose que ce qui sera accepté et traite les refus.
//!
//! Limite : le dépôt d'une **pièce** (photo du mot, certificat) n'est pas encore proposé ici ; le
//! serveur sait la recevoir (`fichier`), l'écran suivra.

mod episodes;
pub mod modele;

use ent_web_api::{ApiError, Enfant, Moi, Retard};
use ent_web_ui::{
    use_notifications, AccesRefuse, Alerte, Annonce, Badge, Bouton, EnTetePage, EtatVide, Niveau,
    OptionSegment, Segmente, Selection, Squelette,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::absences::{libelle_periode, libelle_retard};
use crate::composants_metier::declaration::{DialogueDeclaration, Origine};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use episodes::{SectionCommunications, SectionEpisodes};
use modele::{acces, enfant_choisi, message_sans_droit_de_justifier, nom_enfant, Acces};

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
            Some(Ok(l)) => view! { <ContenuFamille enfants=l/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuFamille(enfants: Vec<Enfant>) -> impl IntoView {
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
        {move || courant.get().map(|e| view! { <FicheEnfant enfant=e/> })}
    }
}

#[component]
fn FicheEnfant(enfant: Enfant) -> impl IntoView {
    let niveau = acces(&enfant);
    let id = enfant.id;
    let prenom = enfant.prenom.clone();
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
    let retards: LocalResource<Result<Vec<Retard>, ApiError>> =
        LocalResource::new(move || async move {
            if niveau == Acces::Complet {
                session::client().retards_eleve(id).await
            } else {
                Ok(Vec::new())
            }
        });

    // Déclaration d'absence (S2-4) : possible pour un lien qui peut justifier ; le serveur revérifie.
    let decl_ouvert = RwSignal::new(false);
    let decl_eleve = nom_enfant(&enfant);
    let declaree = Callback::new(move |_| {
        let t = "Absence déclarée : la vie scolaire est prévenue.";
        notifications.succes(t);
        annonce.set(t.to_string());
    });

    let peut_agir = enfant.peut_justifier_absence;
    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>
        <p class="abs-enfant">{titre_enfant}</p>
        {peut_agir
            .then(|| view! {
                <div class="abs-declarer">
                    <Bouton on_click=Callback::new(move |_| decl_ouvert.set(true))>
                        "Déclarer une absence"
                    </Bouton>
                </div>
            })}
        <DialogueDeclaration
            ouvert=decl_ouvert
            origine=Origine::Famille
            eleve=Signal::derive(move || Some((id, decl_eleve.clone())))
            on_fait=declaree
        />
        {match niveau {
            Acces::Aucun => view! {
                <Alerte niveau=Niveau::Info titre=format!("Vous n'avez pas accès aux absences de {prenom}.")>
                    <p>"Votre lien avec cet enfant ne vous permet ni de consulter ni de justifier ses absences. Adressez-vous à la vie scolaire pour le modifier."</p>
                </Alerte>
            }
                .into_any(),
            Acces::JustificationSeule => view! {
                <Alerte niveau=Niveau::Info titre=format!("Seules les absences de {prenom} sont affichées.")>
                    <p>"Votre lien vous permet de donner le motif des absences, mais pas de consulter l'historique des retards."</p>
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

        {(niveau != Acces::Aucun)
            .then(|| view! { <SectionEpisodes eleve_id=id peut_agir=peut_agir/> })}

        {(niveau == Acces::Complet)
            .then(|| view! {
                <section class="abs-section" aria-labelledby="ret-titre">
                    <h2 id="ret-titre" class="vs-section-titre">"Retards"</h2>
                    <SectionRetards retards=retards/>
                </section>
                <SectionCommunications eleve_id=id/>
            })}
    }
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
    let retards: LocalResource<Result<Vec<Retard>, ApiError>> =
        LocalResource::new(move || async move { session::client().retards_eleve(eleve_id).await });
    view! {
        <SectionEpisodes eleve_id=eleve_id peut_agir=false/>
        <p class="vs-note">"Vos responsables légaux donnent le motif des absences ; vous les consultez ici."</p>
        <section class="abs-section" aria-labelledby="ret-titre">
            <h2 id="ret-titre" class="vs-section-titre">"Mes retards"</h2>
            <SectionRetards retards=retards/>
        </section>
    }
    .into_any()
}
