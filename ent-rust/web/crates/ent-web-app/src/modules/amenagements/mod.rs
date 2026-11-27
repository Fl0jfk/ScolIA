//! Aménagements des élèves d'un professeur et contraintes publiées (socle 2, lot S2-11).
//!
//! Pour les enseignants : leurs élèves aménagés (tiers-temps, police adaptée…) et les contraintes
//! publiées à leur intention (« ne pas courir »…) — **jamais** le diagnostic, le type de
//! dispositif ni la fiche santé. Pour la restauration et les sorties : les contraintes de leur
//! audience. Routes d'`ent-api/src/sante.rs` ; le serveur décide des droits (`amenagements.lire`,
//! `contraintes.lire`, `repas.*`, `sorties.*`).

pub mod modele;

use ent_types::sante::{AmenagementsEleveDto, ContraintesEleveDto};
use ent_web_ui::{Badge, EnTetePage, EtatVide, Squelette, TonEtat};
use leptos::prelude::*;
use leptos_meta::Title;

use crate::composants_metier::sante::{libelle_audience, libelle_portee_amenagement, nom_eleve};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use modele::audiences_proposees;

#[component]
pub fn PageAmenagements() -> impl IntoView {
    let moteur = use_droits_moteur();
    view! {
        <Title text="Aménagements"/>
        <EnTetePage
            surtitre="Élèves".to_string()
            titre="Aménagements et contraintes".to_string()
            sous_titre="Ce que vous devez savoir pour accueillir chaque élève. Aucune information médicale n'est affichée ici.".to_string()
        />
        {move || {
            let m = moteur.get();
            view! {
                {m.peut_lire_amenagements().then(|| view! { <SectionAmenagements/> })}
                {audiences_proposees(&m).into_iter().map(|a| view! { <SectionContraintes audience=a/> }).collect_view()}
            }
        }}
    }
}

#[component]
fn SectionAmenagements() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().mes_amenagements().await });
    view! {
        <section class="ra-section" aria-labelledby="ame-eleves">
            <h2 id="ame-eleves" class="vs-section-titre">"Mes élèves aménagés"</h2>
            {move || match liste.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement des aménagements…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun aménagement" message="Aucun de vos élèves n'a d'aménagement en cours."/> }.into_any(),
                Some(Ok(l)) => view! { <TableauAmenagements liste=l/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn TableauAmenagements(liste: Vec<AmenagementsEleveDto>) -> impl IntoView {
    view! {
        <ul class="jvs-liste">
            {liste.into_iter().map(|e| view! {
                <li class="jvs-ligne">
                    <div>
                        <strong>{nom_eleve(&e.prenom, &e.nom)}</strong>
                        {e.classe.clone().map(|c| format!(" · {c}"))}
                    </div>
                    <ul class="jvs-liste jvs-liste--discrete" aria-label=format!("Aménagements de {}", nom_eleve(&e.prenom, &e.nom))>
                        {e.amenagements.iter().map(|a| view! {
                            <li>
                                {a.libelle.clone()}
                                " "
                                <Badge libelle=libelle_portee_amenagement(&a.portee) ton=TonEtat::Info/>
                            </li>
                        }).collect_view()}
                    </ul>
                </li>
            }).collect_view()}
        </ul>
    }
}

#[component]
fn SectionContraintes(audience: &'static str) -> impl IntoView {
    let liste =
        LocalResource::new(
            move || async move { session::client().contraintes_liste(audience).await },
        );
    let titre_id = format!("con-{audience}");
    let pour = titre_id.clone();
    view! {
        <section class="ra-section" aria-labelledby=pour>
            <h2 id=titre_id class="vs-section-titre">{format!("Contraintes — {}", libelle_audience(audience))}</h2>
            {move || match liste.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des contraintes…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune contrainte" message="Aucune contrainte publiée pour ce public."/> }.into_any(),
                Some(Ok(l)) => view! { <ListeContraintes liste=l/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn ListeContraintes(liste: Vec<ContraintesEleveDto>) -> impl IntoView {
    view! {
        <ul class="jvs-liste">
            {liste.into_iter().map(|e| view! {
                <li class="jvs-ligne">
                    <div>
                        <strong>{nom_eleve(&e.prenom, &e.nom)}</strong>
                        {e.classe.clone().map(|c| format!(" · {c}"))}
                    </div>
                    <ul class="jvs-liste jvs-liste--discrete">
                        {e.contraintes.iter().map(|c| view! {
                            <li>{c.libelle.clone()}{c.date_fin.map(|d| format!(" (jusqu'au {})", crate::noyau::temps::date_complete(d)))}</li>
                        }).collect_view()}
                    </ul>
                </li>
            }).collect_view()}
        </ul>
    }
}
