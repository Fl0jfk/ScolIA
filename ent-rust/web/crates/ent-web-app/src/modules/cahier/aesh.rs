//! Écran C7 « Élèves accompagnés » (`/web/cahier/aesh`) : l'AESH voit, pour les **seuls élèves accompagnés à la date**, leur
//! agenda (avec **leur** version des travaux) et ajoute une note d'accompagnement lue des professeurs du cours et du PP.
//! Aucune copie, aucun retour, aucune note de l'élève, aucune coche : le serveur répond 403 à tout autre accès.

use ent_web_api::cahier_c3_types::AgendaLigneDto;
use ent_web_api::cahier_c7_types::{EleveAccompagneDto, NoteAccompagnementBody};
use ent_web_ui::{Bouton, EnTetePage, EtatVide, Selection, Squelette, Taille, Variante, ZoneTexte};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::modele_c7::valider_note;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};

pub const ADRESSE_AESH: &str = "/web/cahier/aesh";

#[component]
pub fn PageAesh() -> impl IntoView {
    let eleves = LocalResource::new(|| async { session::client().cdt_aesh_eleves().await });
    view! {
        <Title text="Élèves accompagnés"/>
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Élèves accompagnés".to_string()
            sous_titre="L'agenda de chaque élève que vous accompagnez, tel qu'il le voit. Vos notes sont lues de ses professeurs.".to_string()
        />
        {move || match eleves.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des élèves accompagnés…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| eleves.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun élève accompagné" message="Aucun élève ne vous est affecté aujourd'hui."/> }.into_any(),
            Some(Ok(l)) => view! { <Choix eleves=l/> }.into_any(),
        }}
    }
}

#[component]
fn Choix(eleves: Vec<EleveAccompagneDto>) -> impl IntoView {
    let options: Vec<(String, String)> = eleves
        .iter()
        .map(|e| (e.eleve_id.to_string(), format!("{} {}", e.nom, e.prenom)))
        .collect();
    let choix = RwSignal::new(eleves[0].eleve_id.to_string());
    view! {
        <Selection id="aesh-eleve" libelle="Élève accompagné" valeur=choix options=options/>
        {move || choix.get().parse::<Uuid>().ok().map(|id| view! { <Agenda eleve=id/> })}
    }
}

#[component]
fn Agenda(eleve: Uuid) -> impl IntoView {
    let agenda = LocalResource::new(move || async move {
        let auj = aujourdhui();
        session::client()
            .cdt_aesh_agenda(eleve, auj, auj + chrono::Duration::days(14))
            .await
    });
    view! {
        {move || match agenda.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de l'agenda…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| agenda.refetch())/> }.into_any(),
            Some(Ok(a)) if a.lignes.is_empty() => view! { <EtatVide titre="Rien à faire" message="Aucun travail annoncé sur les deux prochaines semaines."/> }.into_any(),
            Some(Ok(a)) => view! {
                <ul class="cdt-liste">
                    {a.lignes.into_iter().map(|l| view! { <li><Ligne eleve=eleve ligne=l/></li> }).collect_view()}
                </ul>
            }.into_any(),
        }}
    }
}

#[component]
fn Ligne(eleve: Uuid, ligne: AgendaLigneDto) -> impl IntoView {
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let note = RwSignal::new(String::new());
    let id = ligne.id;
    let est_travail = ligne.genre == "travail";
    let envoyer = move |_| match valider_note(&note.get_untracked()) {
        Err(m) => ecriture.erreur.set(Some(m)),
        Ok(texte) => {
            ecriture.lancer(async move {
                let b = NoteAccompagnementBody {
                    travail_id: Some(id),
                    entree_id: None,
                    eleve_id: eleve,
                    texte,
                };
                session::client().cdt_aesh_note(&b).await?;
                note.set(String::new());
                Ok(
                    "Note d'accompagnement enregistrée : les professeurs du cours la liront."
                        .to_string(),
                )
            });
        }
    };
    view! {
        <article class="cdt-travail">
            <header class="cdt-travail__entete">
                <h3 class="cdt-travail__titre">{format!("{} — {}", ligne.matiere, ligne.titre)}</h3>
            </header>
            <p class="cdt-travail__detail">{format!("Pour le {}", date_longue(ligne.jour))}</p>
            {ligne.consigne.clone().filter(|c| !c.trim().is_empty()).map(|c| view! { <p class="cdt-travail__consigne">{c}</p> })}
            {est_travail.then(|| view! {
                <details class="cdt-seance__saisie">
                    <summary>"Ajouter une note d'accompagnement"</summary>
                    <RetourEcriture ecriture=ecriture/>
                    <ZoneTexte id=format!("aesh-note-{id}") libelle="Note (ex. : consigne reformulée en classe)" valeur=note/>
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=Callback::new(envoyer)>"Enregistrer la note"</Bouton>
                </details>
            })}
        </article>
    }
}
