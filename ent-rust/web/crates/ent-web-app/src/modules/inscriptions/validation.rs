//! Validation d'une inscription admise (INS-05) : rapprochement **humain** des responsables et
//! de l'élève avec les fiches existantes, récapitulatif, puis confirmation. Le serveur refuse
//! la validation tant qu'une correspondance n'a pas été tranchée ; l'écran ne fait que les
//! présenter (`GET …/correspondances`) et envoyer les choix (`POST …/valider`).

use ent_web_api::types_inscriptions::CorrespondancesDto;
use ent_web_ui::{
    use_notifications, Alerte, Bouton, Case, EtatVide, Niveau, Selection, Squelette, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use super::message_echec;
use super::modele::{
    construire_validation, libelle_candidat, options_foyer, options_personne, ChoixSaisi,
    CHOIX_NOUVEAU,
};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn ValidationDossier(
    id: Uuid,
    /// Nom des responsables déclarés par rang, pour nommer les lignes de rapprochement.
    noms: Vec<(i16, String)>,
    recharger: Callback<()>,
) -> impl IntoView {
    let correspondances =
        LocalResource::new(
            move || async move { session::client().correspondances_dossier(id).await },
        );
    let recharger_liste = Callback::new(move |_| correspondances.refetch());
    let noms = StoredValue::new(noms);
    view! {
        <section class="pile carte" aria-labelledby="ins-validation">
            <h4 id="ins-validation">"Valider l'inscription"</h4>
            {move || match correspondances.get() {
                None => view! { <Squelette lignes=3 libelle="Recherche des fiches existantes…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger_liste/> }.into_any(),
                Some(Ok(c)) => view! { <FormulaireValidation id=id corr=c noms=noms.get_value() recharger=recharger/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn FormulaireValidation(
    id: Uuid,
    corr: CorrespondancesDto,
    noms: Vec<(i16, String)>,
    recharger: Callback<()>,
) -> impl IntoView {
    let notifications = use_notifications();
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let verifie = RwSignal::new(false);
    let eleve_choix = RwSignal::new(String::new());
    let confirme_resp = RwSignal::new(false);

    // Un couple de signaux (fiche, foyer) par responsable déclaré qui a des correspondances.
    let lignes: Vec<_> = corr
        .responsables
        .iter()
        .map(|r| {
            (
                r.clone(),
                RwSignal::new(String::new()),
                RwSignal::new(String::new()),
            )
        })
        .collect();
    let rien_a_choisir =
        corr.responsables.iter().all(|r| r.candidats.is_empty()) && corr.eleves.is_empty();
    let corr = StoredValue::new(corr);
    let signaux: Vec<(RwSignal<String>, RwSignal<String>)> =
        lignes.iter().map(|(_, p, f)| (*p, *f)).collect();
    let signaux = StoredValue::new(signaux);

    let valider = Callback::new(move |_| {
        let choix: Vec<ChoixSaisi> = signaux.with_value(|s| {
            s.iter()
                .map(|(p, f)| ChoixSaisi {
                    personne: p.get_untracked(),
                    foyer: f.get_untracked(),
                })
                .collect()
        });
        let corps = corr.with_value(|c| {
            construire_validation(
                c,
                &choix,
                &eleve_choix.get_untracked(),
                confirme_resp.get_untracked(),
            )
        });
        let corps = match corps {
            Ok(c) => c,
            Err(m) => return erreur.set(Some(m)),
        };
        if !verifie.get_untracked() {
            return erreur.set(Some(
                "Cochez la case de vérification avant de valider.".into(),
            ));
        }
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client().valider_dossier(id, &corps).await;
            en_cours.set(false);
            match r {
                Ok(_) => {
                    notifications.succes(
                        "Inscription validée : la scolarisation et l'inscription annuelle prévue sont créées.".to_string(),
                    );
                    recharger.run(());
                }
                Err(e) => erreur.set(Some(message_echec(e))),
            }
        });
    });

    let eleves = corr.with_value(|c| c.eleves.clone());
    let a_eleves = !eleves.is_empty();
    let options_eleve: Vec<(String, String)> =
        std::iter::once((String::new(), "— Choisir —".to_string()))
            .chain(eleves.iter().map(|e| {
                (
                    e.eleve_id.to_string(),
                    format!(
                "Rattacher à la fiche existante : {} {}{}{} ({}, {} responsable(s) déjà lié(s))",
                e.prenom,
                e.nom,
                e.date_naissance
                    .map(|d| format!(", né(e) le {}", d.format("%d/%m/%Y")))
                    .unwrap_or_default(),
                if e.ine_identique { ", même INE" } else { "" },
                e.statut,
                e.responsables_lies
            ),
                )
            }))
            .chain(std::iter::once((
                CHOIX_NOUVEAU.to_string(),
                "Créer une nouvelle fiche (homonyme, autre élève)".to_string(),
            )))
            .collect();
    let a_responsables_lies = eleves.iter().any(|e| e.responsables_lies > 0);

    view! {
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        {if rien_a_choisir {
            view! { <EtatVide titre="Aucune fiche existante ne correspond" message="Les responsables et l'élève seront créés à la validation."/> }.into_any()
        } else {
            view! {
                <Alerte niveau=Niveau::Info titre="Rapprochement avec les fiches existantes">
                    <p>"Aucune fiche n'est rattachée automatiquement : une personne de l'établissement tranche pour chaque correspondance, pour ne jamais ouvrir un dossier à la mauvaise personne."</p>
                </Alerte>
            }.into_any()
        }}
        {a_eleves.then(|| view! {
            <fieldset class="ui-champ">
                <legend>"Élève"</legend>
                <Selection id="ins-val-eleve" libelle="Une fiche élève ressemble à ce dossier" valeur=eleve_choix options=options_eleve.clone() requis=true/>
                {a_responsables_lies.then(|| view! {
                    <Case id="ins-val-eleve-conf" libelle="Je confirme que les responsables déjà liés à cette fiche peuvent être rattachés au dossier" coche=confirme_resp/>
                })}
            </fieldset>
        })}
        {lignes.into_iter().filter(|(r, _, _)| !r.candidats.is_empty()).map(|(r, personne, foyer)| {
            let nom = noms.iter().find(|(rang, _)| *rang == r.rang).map(|(_, n)| n.clone()).unwrap_or_default();
            let legende = format!("Responsable {} : {}", r.rang, nom);
            let options = options_personne(&r);
            let candidats = StoredValue::new(r.candidats.clone());
            let id_p = format!("ins-val-p{}", r.rang);
            let id_f = format!("ins-val-f{}", r.rang);
            view! {
                <fieldset class="ui-champ">
                    <legend>{legende}</legend>
                    <Selection id=id_p libelle="Fiche existante qui correspond" valeur=personne options=options requis=true/>
                    {move || {
                        let choisi = personne.get();
                        let cand = candidats.with_value(|c| c.iter().find(|c| c.personne_id.to_string() == choisi).cloned());
                        cand.filter(|c| !c.foyers.is_empty()).map(|c| view! {
                            <p class="aide">{format!("Fiche choisie : {}", libelle_candidat(&c))}</p>
                            <Selection id=id_f.clone() libelle="Foyer" valeur=foyer options=options_foyer(&c) requis=true/>
                        })
                    }}
                </fieldset>
            }
        }).collect_view()}
        <Case id="ins-val-verifie" libelle="J'ai vérifié les rapprochements ci-dessus : l'inscription crée la scolarisation à la rentrée et les liens avec les responsables" coche=verifie/>
        <Bouton variante=Variante::Primaire en_cours=en_cours on_click=valider>"Valider l'inscription"</Bouton>
    }
}
