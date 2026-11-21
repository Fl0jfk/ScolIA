//! Consultation famille / élève (EVA-13, ORI-03) : relevé de la période (**évaluations
//! publiées seulement**), bulletins publiés (PDF), avis du conseil sur les vœux après la
//! publication du bulletin et réponse du responsable. Le personnel autorisé voit la même page.

use ent_types::evaluations::{ReleveEleveDto, VoeuAvisDto};
use ent_web_ui::{
    Alerte, Badge, Bouton, EnTetePage, EtatVide, Niveau, Squelette, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use super::bulletins::telecharger_pdf;
use super::modele::{libelle_code, moyenne_fr, nombre_fr, texte_resultat, AVIS};
use super::{ChoixPeriode, RACINE};
use crate::noyau::coquille::use_droits;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn PageEvaluationsEleve() -> impl IntoView {
    let p = use_params_map();
    let id = Memo::new(move |_| {
        p.with(|p| p.get("id"))
            .and_then(|s| Uuid::parse_str(&s).ok())
    });
    view! {
        <Title text="Notes et bulletins"/>
        <EnTetePage
            surtitre="Évaluations".to_string()
            titre="Notes et bulletins".to_string()
            sous_titre="Évaluations publiées, bulletins et orientation.".to_string()
        />
        <p class="eva-periode"><a href=RACINE>"Évaluations"</a></p>
        {move || match id.get() {
            Some(e) => view! { <Eleve eleve=e/> }.into_any(),
            None => view! { <EtatVide titre="Élève introuvable" message="Adresse invalide."/> }.into_any(),
        }}
    }
}

#[component]
fn Eleve(eleve: Uuid) -> impl IntoView {
    let periodes = LocalResource::new(|| async { session::client().periodes_notes().await });
    let choix = RwSignal::new(String::new());
    view! {
        {move || match periodes.get() {
            None => view! { <Squelette lignes=2 libelle="Chargement des périodes…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| periodes.refetch())/> }.into_any(),
            Some(Ok(l)) => view! { <div class="jvs-barre"><ChoixPeriode valeur=choix periodes=l/></div> }.into_any(),
        }}
        {move || Uuid::parse_str(&choix.get()).ok().map(|p| view! { <Releve eleve=eleve periode=p/> })}
        <Bulletins eleve=eleve/>
        <Orientation eleve=eleve/>
    }
}

#[component]
fn Releve(eleve: Uuid, periode: Uuid) -> impl IntoView {
    let releve =
        LocalResource::new(
            move || async move { session::client().releve_eleve(eleve, periode).await },
        );
    view! {
        <section class="ra-section" aria-labelledby="fam-releve">
            <h2 id="fam-releve" class="vs-section-titre">"Relevé de notes"</h2>
            {move || match releve.get() {
                None => view! { <Squelette lignes=5 libelle="Chargement du relevé…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| releve.refetch())/> }.into_any(),
                Some(Ok(r)) => view! { <TableauReleve releve=r/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn TableauReleve(releve: ReleveEleveDto) -> impl IntoView {
    if releve.matieres.iter().all(|m| m.evaluations.is_empty()) {
        return view! { <EtatVide titre="Aucune note publiée" message="Aucune évaluation publiée sur cette période."/> }.into_any();
    }
    view! {
        <p>{format!("Moyenne générale : {}", moyenne_fr(releve.moyenne_generale))}</p>
        <div class="eva-cadre">
            <table class="ui-tableau">
                <caption class="ui-sr">"Notes publiées par matière"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Matière"</th>
                        <th scope="col">"Évaluations"</th>
                        <th scope="col">"Moyenne"</th>
                        <th scope="col">"Classe"</th>
                    </tr>
                </thead>
                <tbody>
                    {releve.matieres.into_iter().map(|m| view! {
                        <tr>
                            <th scope="row">{m.matiere_libelle.clone()}</th>
                            <td>
                                <ul class="eva-notes">
                                    {m.evaluations.iter().map(|e| {
                                        let v = texte_resultat(&e.statut, e.valeur);
                                        let v = if e.statut == "note" { format!("{v}/{}", nombre_fr(e.sur)) } else { v };
                                        view! {
                                            <li>{format!("{} ({}, coef {}) : {v}", e.titre, e.date_devoir.format("%d/%m"), nombre_fr(e.coef))}</li>
                                        }
                                    }).collect_view()}
                                </ul>
                            </td>
                            <td class="eva-moyenne">{moyenne_fr(m.moyenne)}</td>
                            <td>{moyenne_fr(m.moyenne_classe)}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </div>
        <p class="texte-attenue">"Absent, dispensé et non noté ne comptent pas dans la moyenne."</p>
    }
    .into_any()
}

#[component]
fn Bulletins(eleve: Uuid) -> impl IntoView {
    let liste =
        LocalResource::new(move || async move { session::client().bulletins_eleve(eleve).await });
    let erreur: RwSignal<Option<String>> = RwSignal::new(None);
    view! {
        <section class="ra-section" aria-labelledby="fam-bulletins">
            <h2 id="fam-bulletins" class="vs-section-titre">"Bulletins"</h2>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {move || match liste.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des bulletins…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun bulletin" message="Aucun bulletin publié pour le moment."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|b| {
                            let bb = b.clone();
                            view! {
                                <li class="jvs-ligne">
                                    <span>{format!("Version {} · {}", b.version, b.publie_le.map(|d| d.format("publié le %d/%m/%Y").to_string()).unwrap_or_else(|| b.statut.clone()))}</span>
                                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte on_click=Callback::new(move |_| telecharger_pdf(&bb, erreur))>"Télécharger le PDF"</Bouton>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn Orientation(eleve: Uuid) -> impl IntoView {
    let voeux =
        LocalResource::new(move || async move { session::client().orientation_eleve(eleve).await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| voeux.refetch()));
    let droits = use_droits();
    view! {
        {move || match voeux.get() {
            Some(Ok(l)) if !l.is_empty() => view! {
                <section class="ra-section" aria-labelledby="fam-orientation">
                    <h2 id="fam-orientation" class="vs-section-titre">"Orientation"</h2>
                    <RetourEcriture ecriture=ecriture/>
                    <ul class="jvs-liste">
                        {l.into_iter().map(|v| view! { <LigneVoeu voeu=v responsable=droits.get_untracked().portail_famille ecriture=ecriture/> }).collect_view()}
                    </ul>
                </section>
            }
            .into_any(),
            _ => ().into_any(),
        }}
    }
}

#[component]
fn LigneVoeu(voeu: VoeuAvisDto, responsable: bool, ecriture: Ecriture) -> impl IntoView {
    let repondre = move |reponse: &'static str| {
        let id = voeu.avis_id;
        Callback::new(move |_| {
            if let Some(id) = id {
                ecriture.lancer(async move {
                    session::client().repondre_avis(id, reponse).await?;
                    Ok("Réponse transmise à l'établissement.".into())
                });
            }
        })
    };
    let avis = voeu.avis.clone();
    view! {
        <li class="jvs-ligne jvs-ligne--bloc">
            <span>{format!("Vœu {} : {}", voeu.rang, voeu.filiere_libelle)}</span>
            {match avis {
                None => view! { <span class="texte-attenue">"Avis du conseil non encore communiqué."</span> }.into_any(),
                Some(a) => view! {
                    <span>
                        <Badge libelle=format!("Avis : {}", libelle_code(&AVIS, Some(&a))) ton=TonEtat::Info/>
                        {(!voeu.motivation.is_empty()).then(|| format!(" {}", voeu.motivation))}
                    </span>
                }
                .into_any(),
            }}
            {voeu.reponse_famille.clone().map(|r| view! { <Badge libelle=format!("Votre réponse : {r}") ton=TonEtat::Ok/> })}
            {(responsable && voeu.avis.is_some() && voeu.reponse_famille.is_none()).then(|| view! {
                <span class="eva-actions">
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte en_cours=ecriture.en_cours on_click=repondre("accord")>"Accord"</Bouton>
                    <Bouton variante=Variante::Discret taille=Taille::Compacte en_cours=ecriture.en_cours on_click=repondre("desaccord")>"Désaccord"</Bouton>
                </span>
            })}
        </li>
    }
}
