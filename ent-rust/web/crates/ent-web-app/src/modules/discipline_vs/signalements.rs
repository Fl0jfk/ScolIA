//! Signalements d'assiduité : décompte du mois en demi-journées, proposition par la vie scolaire,
//! validation / refus / émission par la direction.

use ent_types::discipline::{DecompteMoisDto, SignalementDto};
use ent_web_ui::{
    Badge, Bouton, ChampTexte, Confirmation, EnTetePage, EtatVide, Squelette, Statistique, Taille,
    TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use std::collections::HashMap;
use uuid::Uuid;

use super::modele::{
    decisions_signalement, libelle_statut_signalement, lire_mois, resume_signalement,
    ton_statut_signalement,
};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

#[component]
pub fn PageSignalements() -> impl IntoView {
    // Le mois est posé après l'hydratation : aucune date dans le HTML rendu côté serveur.
    let mois_texte = RwSignal::new(String::new());
    Effect::new(move |_| {
        if mois_texte.get_untracked().is_empty() {
            mois_texte.set(aujourdhui().format("%Y-%m").to_string());
        }
    });
    let mois = Memo::new(move |_| lire_mois(&mois_texte.get()));
    view! {
        <Title text="Signalements d'assiduité"/>
        <EnTetePage
            surtitre="Vie scolaire".to_string()
            titre="Signalements d'assiduité".to_string()
            sous_titre="Élèves ayant atteint le seuil de demi-journées non justifiées dans le mois.".to_string()
        />
        <div class="jvs-barre">
            <ChampTexte id="sig-mois" libelle="Mois" valeur=mois_texte genre="month"/>
        </div>
        {move || match mois.get() {
            None => view! { <EtatVide titre="Mois invalide" message="Choisissez un mois."/> }.into_any(),
            Some(m) => view! { <ContenuMois mois=m/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuMois(mois: chrono::NaiveDate) -> impl IntoView {
    let decompte =
        LocalResource::new(move || async move { session::client().decompte_mensuel(mois).await });
    let signalements =
        LocalResource::new(move || async move { session::client().signalements(mois).await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        decompte.refetch();
        signalements.refetch();
    }));
    let moteur = use_droits_moteur();
    let proposer = Callback::new(move |_| {
        ecriture.lancer(async move {
            let l = session::client().proposer_signalements(mois).await?;
            Ok(format!(
                "{} signalement(s) proposé(s) pour ce mois.",
                l.len()
            ))
        });
    });
    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="ra-section" aria-labelledby="sig-decompte">
            <h2 id="sig-decompte" class="vs-section-titre">"Décompte du mois"</h2>
            {move || match decompte.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement du décompte…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| decompte.refetch())/>
                }
                .into_any(),
                Some(Ok(d)) => view! { <TableauDecompte decompte=d/> }.into_any(),
            }}
        </section>
        <section class="ra-section" aria-labelledby="sig-liste">
            <h2 id="sig-liste" class="vs-section-titre">"Signalements"</h2>
            {move || {
                moteur.get().peut_signaler_assiduite().then(|| view! {
                    <p>
                        <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=proposer>
                            "Proposer les signalements du mois"
                        </Bouton>
                        <span class="texte-attenue">" Idempotent : ne recrée pas un signalement déjà proposé."</span>
                    </p>
                })
            }}
            {move || match signalements.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des signalements…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| signalements.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucun signalement" message="Aucun signalement proposé pour ce mois."/>
                }
                .into_any(),
                Some(Ok(l)) => {
                    let noms = match decompte.get() {
                        Some(Ok(d)) => noms_depuis_decompte(&d),
                        _ => HashMap::new(),
                    };
                    view! { <ListeSignalements signalements=l noms=noms ecriture=ecriture/> }.into_any()
                }
            }}
        </section>
    }
}

/// Noms des élèves connus par le décompte (les signalements ne portent que l'identifiant).
pub fn noms_depuis_decompte(d: &DecompteMoisDto) -> HashMap<Uuid, String> {
    d.eleves
        .iter()
        .map(|e| {
            (
                e.eleve_id,
                format!("{} {}", e.prenom, e.nom).trim().to_string(),
            )
        })
        .collect()
}

#[component]
fn TableauDecompte(decompte: DecompteMoisDto) -> impl IntoView {
    let seuil = decompte.seuil_signalement;
    let atteints = decompte.eleves.iter().filter(|e| e.seuil_atteint).count();
    let ton_atteints = if atteints > 0 {
        TonEtat::Avertissement
    } else {
        TonEtat::Ok
    };
    if decompte.eleves.is_empty() {
        return view! { <EtatVide titre="Aucune absence ce mois-ci" message="Aucun élève n'a de demi-journée d'absence comptée."/> }.into_any();
    }
    view! {
        <div class="jvs-stats">
            <Statistique libelle="Élèves concernés" valeur=Signal::derive({ let n = decompte.eleves.len(); move || n.to_string() })/>
            <Statistique
                libelle="Seuil atteint"
                valeur=Signal::derive(move || atteints.to_string())
                ton=ton_atteints
            />
        </div>
        <p class="texte-attenue">{format!("Seuil de signalement de l'établissement : {seuil} demi-journée(s) non justifiée(s).")}</p>
        <div class="ra-tableau-conteneur">
            <table class="ra-tableau">
                <caption class="ui-sr">"Demi-journées d'absence du mois par élève"</caption>
                <thead>
                    <tr>
                        <th scope="col">"Élève"</th>
                        <th scope="col">"Demi-journées comptées"</th>
                        <th scope="col">"Dont non justifiées"</th>
                        <th scope="col">"Seuil"</th>
                    </tr>
                </thead>
                <tbody>
                    {decompte
                        .eleves
                        .into_iter()
                        .map(|e| {
                            view! {
                                <tr>
                                    <th scope="row">{format!("{} {}", e.prenom, e.nom)}</th>
                                    <td>{e.demi_journees_comptees}</td>
                                    <td>{e.demi_journees_non_justifiees}</td>
                                    <td>
                                        {if e.seuil_atteint {
                                            view! { <Badge libelle="Atteint" ton=TonEtat::Avertissement/> }.into_any()
                                        } else {
                                            view! { <Badge libelle="Non atteint" ton=TonEtat::Neutre/> }.into_any()
                                        }}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

#[component]
fn ListeSignalements(
    signalements: Vec<SignalementDto>,
    noms: HashMap<Uuid, String>,
    ecriture: Ecriture,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let cible = RwSignal::new(None::<(Uuid, &'static str, &'static str, String)>);
    let confirmer = RwSignal::new(false);
    view! {
        <ul class="jvs-liste">
            {signalements
                .into_iter()
                .map(|s| {
                    let nom = noms.get(&s.eleve_id).cloned().unwrap_or_else(|| "Élève".into());
                    let (id, statut, resume) = (s.id, s.statut.clone(), resume_signalement(&s));
                    let decisions = decisions_signalement(&statut);
                    let nom_affiche = nom.clone();
                    view! {
                        <li class="jvs-ligne">
                            <div>
                                <strong>{nom_affiche}</strong>
                                <span class="texte-attenue">{format!(" — {resume}")}</span>
                            </div>
                            <Badge libelle=libelle_statut_signalement(&statut) ton=ton_statut_signalement(&statut)/>
                            {move || {
                                moteur
                                    .get()
                                    .peut_signaler_assiduite()
                                    .then(|| {
                                        let nom = nom.clone();
                                        view! {
                                            <div class="ra-actions">
                                                {decisions
                                                    .iter()
                                                    .map(|(action, libelle)| {
                                                        let (action, libelle) = (*action, *libelle);
                                                        let nom = nom.clone();
                                                        let accessible = format!("{libelle} : {nom}");
                                                        view! {
                                                            <Bouton
                                                                variante=if action == "refuser" { Variante::Discret } else { Variante::Secondaire }
                                                                taille=Taille::Compacte
                                                                en_cours=ecriture.en_cours
                                                                libelle_accessible=accessible
                                                                on_click=Callback::new(move |_| {
                                                                    cible.set(Some((id, action, libelle, nom.clone())));
                                                                    confirmer.set(true);
                                                                })
                                                            >
                                                                {libelle}
                                                            </Bouton>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </div>
                                        }
                                    })
                            }}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
        <Confirmation
            ouvert=confirmer
            id="conf-signalement"
            titre="Confirmer la décision ?"
            message="La décision est enregistrée et tracée. Elle ne notifie ni l'élève ni sa famille depuis cet écran."
            libelle_confirmer="Confirmer"
            on_confirmer=Callback::new(move |_| {
                if let Some((id, action, libelle, _)) = cible.get_untracked() {
                    ecriture.lancer(async move {
                        session::client().decider_signalement(id, action).await?;
                        Ok(format!("{libelle} : fait."))
                    });
                }
            })
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_types::discipline::DecompteEleveDto;

    #[test]
    fn noms_des_eleves_du_decompte() {
        let d = DecompteMoisDto {
            mois: chrono::NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            seuil_signalement: 4,
            eleves: vec![DecompteEleveDto {
                eleve_id: Uuid::from_u128(1),
                nom: "Potter".into(),
                prenom: "Harry".into(),
                demi_journees_comptees: 5,
                demi_journees_non_justifiees: 4,
                seuil_atteint: true,
            }],
        };
        let m = noms_depuis_decompte(&d);
        assert_eq!(
            m.get(&Uuid::from_u128(1)).map(String::as_str),
            Some("Harry Potter")
        );
    }
}
