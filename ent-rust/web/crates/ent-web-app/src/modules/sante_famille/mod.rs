//! Santé de l'enfant côté famille (socle 2, lot S2-11) : projet d'accueil individualisé (PAI) à
//! lire et à signer, passages à l'infirmerie que l'établissement a rendus visibles (sans note ni
//! soins), aménagements pédagogiques. Routes d'`ent-api/src/sante.rs` ; le serveur ne répond
//! que pour un enfant au lien actif et ne renvoie jamais la note interne du PAI.

pub mod modele;

use ent_types::sante::PaiCompletDto;
use ent_web_api::Enfant;
use ent_web_ui::{
    Alerte, Badge, Bouton, EnTetePage, EtatVide, Niveau, OptionSegment, Segmente, Selection,
    Squelette, TonEtat,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::sante::{
    libelle_info_famille, libelle_issue, libelle_motif, libelle_partie, libelle_portee_amenagement,
};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_complete;
use modele::{enfant_choisi, peut_signer};

#[component]
pub fn PageSanteEnfant() -> impl IntoView {
    let enfants = LocalResource::new(|| async { session::client().enfants_famille().await });
    view! {
        <Title text="Santé de mon enfant"/>
        <EnTetePage
            titre="Santé de mon enfant".to_string()
            sous_titre="Projet d'accueil individualisé, passages à l'infirmerie et aménagements.".to_string()
        />
        {move || match enfants.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de vos enfants…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| enfants.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun enfant rattaché" message="Aucun élève n'est rattaché à votre compte."/> }.into_any(),
            Some(Ok(l)) => view! { <ContenuFamille enfants=l/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuFamille(enfants: Vec<Enfant>) -> impl IntoView {
    let choix = RwSignal::new(
        enfants
            .first()
            .map(|e| e.id.to_string())
            .unwrap_or_default(),
    );
    let options: Vec<(String, String)> = enfants
        .iter()
        .map(|e| (e.id.to_string(), format!("{} {}", e.prenom, e.nom)))
        .collect();
    let liste = StoredValue::new(enfants.clone());
    let courant = Memo::new(move |_| enfant_choisi(&liste.get_value(), &choix.get()));
    let selecteur = (enfants.len() > 1).then(|| {
        if enfants.len() <= 3 {
            let segments: Vec<OptionSegment> = options
                .iter()
                .cloned()
                .map(|(v, l)| OptionSegment::new(v, l))
                .collect();
            view! { <Segmente nom="enfant-sante" legende="Enfant" options=segments valeur=choix/> }.into_any()
        } else {
            view! { <Selection id="enfant-sante" libelle="Enfant" valeur=choix options=options.clone()/> }.into_any()
        }
    });
    view! {
        {selecteur}
        {move || courant.get().map(|id| view! { <FicheEnfant eleve_id=id/> })}
    }
}

#[component]
fn FicheEnfant(eleve_id: Uuid) -> impl IntoView {
    let pai =
        LocalResource::new(move || async move { session::client().pai_complet(eleve_id).await });
    let passages =
        LocalResource::new(move || async move { session::client().passages_eleve(eleve_id).await });
    let amenagements =
        LocalResource::new(
            move || async move { session::client().amenagements_eleve(eleve_id).await },
        );
    let ecriture = Ecriture::nouveau(Callback::new(move |_| pai.refetch()));
    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="ra-section" aria-labelledby="sf-pai">
            <h2 id="sf-pai" class="vs-section-titre">"Projet d'accueil individualisé (PAI)"</h2>
            {move || match pai.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement du PAI…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| pai.refetch())/> }.into_any(),
                Some(Ok(None)) => view! { <EtatVide titre="Pas de PAI" message="Aucun PAI actif n'est enregistré pour votre enfant."/> }.into_any(),
                Some(Ok(Some(p))) => view! { <ContenuPai p=p eleve_id=eleve_id ecriture=ecriture/> }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="sf-passages">
            <h2 id="sf-passages" class="vs-section-titre">"Passages à l'infirmerie"</h2>
            {move || match passages.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des passages…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| passages.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun passage" message="Aucun passage n'a été porté à votre connaissance."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|p| view! {
                            <li class="jvs-ligne">
                                <span><strong>{date_complete(p.jour)}</strong>{format!(" — {} · {}", libelle_motif(&p.motif), libelle_issue(&p.issue))}</span>
                                <Badge libelle=libelle_info_famille(&p.famille_information) ton=TonEtat::Neutre/>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="sf-amenagements">
            <h2 id="sf-amenagements" class="vs-section-titre">"Aménagements pédagogiques"</h2>
            {move || match amenagements.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des aménagements…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| amenagements.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun aménagement" message="Aucun aménagement en cours."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|a| view! {
                            <li class="jvs-ligne">
                                <strong>{a.libelle}</strong>
                                <span class="texte-attenue">{libelle_portee_amenagement(&a.portee)}</span>
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
        </section>
    }
}

#[component]
fn ContenuPai(p: PaiCompletDto, eleve_id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let signer = peut_signer(&p);
    let famille_a_signe = p.signatures.iter().any(|s| s.partie == "famille");
    view! {
        <dl class="ra-definitions">
            <dt>"Période"</dt>
            <dd>{format!("depuis le {}{}", date_complete(p.pai.date_debut), p.pai.date_fin.map(|f| format!(" jusqu'au {}", date_complete(f))).unwrap_or_default())}</dd>
            <dt>"Motif"</dt>
            <dd>{p.pai.motif.clone()}</dd>
            <dt>"Mesures d'accueil"</dt>
            <dd>{p.pai.mesures_accueil.clone()}</dd>
            <dt>"Protocole d'urgence"</dt>
            <dd>{if p.pai.protocole_urgence.is_empty() { "—".to_string() } else { p.pai.protocole_urgence.clone() }}</dd>
            <dt>"Trousse d'urgence"</dt>
            <dd>{if p.trousse_urgence { format!("Oui — {}", p.trousse_lieu) } else { "Non".to_string() }}</dd>
        </dl>
        <h3 class="jvs-sous-titre">"Signatures"</h3>
        <ul class="jvs-liste">
            {p.signatures.iter().map(|s| view! {
                <li class="jvs-ligne">
                    <span><strong>{libelle_partie(&s.partie)}</strong>{format!(" — {}", s.signataire_nom)}</span>
                    <Badge libelle="Signé" ton=TonEtat::Ok/>
                </li>
            }).collect_view()}
            {p.parties_manquantes.iter().map(|m| view! {
                <li class="jvs-ligne">
                    <span>{libelle_partie(m)}</span>
                    <Badge libelle="En attente" ton=TonEtat::Avertissement/>
                </li>
            }).collect_view()}
        </ul>
        {if p.complet {
            view! { <Alerte niveau=Niveau::Succes titre="Le PAI est signé par toutes les parties."/> }.into_any()
        } else if famille_a_signe {
            view! { <p class="texte-attenue">"Vous avez signé ce PAI. Les autres parties n'ont pas encore signé."</p> }.into_any()
        } else {
            ().into_any()
        }}
        {signer.then(|| view! {
            <Bouton en_cours=ecriture.en_cours
                on_click=Callback::new(move |_| {
                    ecriture.lancer(async move {
                        session::client()
                            .signer_pai(eleve_id, &ent_types::sante::SignerPaiBody { partie: "famille".into(), signataire_nom: None })
                            .await?;
                        Ok("Merci : votre signature est enregistrée.".to_string())
                    });
                })>
                "Signer le PAI"
            </Bouton>
        })}
    }
}
