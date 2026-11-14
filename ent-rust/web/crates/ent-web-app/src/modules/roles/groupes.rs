//! Responsables des groupes et ateliers : ils ouvrent la portée « mes groupes » des rôles.
//!
//! L'API ne permet que de **remplacer** la liste des responsables d'un groupe
//! (`PUT /referentiel/groupes/{id}/responsables`) : la liste actuelle n'est pas lisible,
//! l'écran le dit et demande confirmation.

use ent_web_api::{ApiError, PersonnelApi};
use ent_web_ui::{
    Alerte, Annonce, Bouton, ChampTexte, Dialogue, EnTetePage, EtatVide, Niveau, Selection,
    Squelette, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use std::collections::BTreeSet;
use uuid::Uuid;

use super::editeur::charger_cibles;
use super::modele::{candidats, Cible};
use super::{retour_connexion, GardeRoles};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;

/// Message pour un refus d'écriture sur les responsables d'un groupe.
pub(crate) fn message_erreur_responsables(e: &ApiError) -> String {
    match e {
        ApiError::Interdit(_) => "Vous n'avez pas le droit de définir les responsables de ce groupe.".into(),
        ApiError::Invalide(_) => "Responsables refusés : seules des personnes du personnel de l'établissement peuvent être responsables.".into(),
        ApiError::Introuvable(_) => "Ce groupe n'existe plus. Rechargez la page.".into(),
        autre => autre.message_utilisateur(),
    }
}

#[derive(Clone)]
struct Donnees {
    groupes: Vec<Cible>,
    personnel: Vec<PersonnelApi>,
}

async fn charger() -> Result<Donnees, ApiError> {
    let groupes: Vec<Cible> = charger_cibles()
        .await?
        .into_iter()
        .filter(|c| c.groupe)
        .collect();
    let personnel = session::client().personnel("").await?;
    Ok(Donnees { groupes, personnel })
}

#[component]
pub fn PageResponsables() -> impl IntoView {
    view! {
        <Title text="Responsables des groupes"/>
        <EnTetePage
            titre="Responsables des groupes et ateliers".to_string()
            surtitre="Rôles et droits".to_string()
            sous_titre="Une personne responsable d'un groupe reçoit les droits « mes groupes » de ses rôles sur les élèves de ce groupe.".to_string()
        >
            <a class=ent_web_ui::classes_bouton(Variante::Secondaire, ent_web_ui::Taille::Normale) href=navigation::ROLES>
                "Retour aux rôles"
            </a>
        </EnTetePage>
        <GardeRoles>
            <Chargeur/>
        </GardeRoles>
    }
}

#[component]
fn Chargeur() -> impl IntoView {
    let donnees = LocalResource::new(|| async { charger().await });
    view! {
        {move || match donnees.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement des groupes…"/> }.into_any(),
            Some(Err(e)) => {
                view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| donnees.refetch())/> }
                    .into_any()
            }
            Some(Ok(d)) if d.groupes.is_empty() => view! {
                <EtatVide
                    titre="Aucun groupe ni atelier"
                    message="Les groupes se créent dans le référentiel de l'établissement, rattachés à une classe."
                />
            }
            .into_any(),
            Some(Ok(d)) => view! { <Formulaire donnees=d/> }.into_any(),
        }}
    }
}

#[component]
fn Formulaire(donnees: Donnees) -> impl IntoView {
    let Donnees { groupes, personnel } = donnees;
    let notifications = ent_web_ui::use_notifications();
    let reconnecter = retour_connexion();
    let personnel = StoredValue::new(personnel);
    let groupe = RwSignal::new(String::new());
    let recherche = RwSignal::new(String::new());
    let choisis = RwSignal::new(BTreeSet::<Uuid>::new());
    let erreur = RwSignal::new(None::<String>);
    let annonce = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let confirmation = RwSignal::new(false);

    let nom_groupe: Vec<(Uuid, String)> =
        groupes.iter().map(|g| (g.id, g.libelle.clone())).collect();
    let nom_groupe = StoredValue::new(nom_groupe);
    let options: Vec<(String, String)> =
        std::iter::once((String::new(), "Choisir un groupe…".to_string()))
            .chain(
                groupes
                    .iter()
                    .map(|g| (g.id.to_string(), g.libelle.clone())),
            )
            .collect();

    let propositions =
        Memo::new(move |_| personnel.with_value(|p| candidats(p, &[], &recherche.get())));
    let groupe_id = Memo::new(move |_| Uuid::parse_str(&groupe.get()).ok());
    // Changer de groupe repart d'une liste vide : on ne montre jamais une sélection d'un autre groupe.
    Effect::new(move |_| {
        let _ = groupe.get();
        choisis.set(BTreeSet::new());
        erreur.set(None);
    });

    let enregistrer = {
        let reconnecter = reconnecter.clone();
        Callback::new(move |_: ()| {
            let Some(id) = groupe_id.get_untracked() else {
                return;
            };
            let liste: Vec<Uuid> = choisis.get_untracked().into_iter().collect();
            en_cours.set(true);
            erreur.set(None);
            let reconnecter = reconnecter.clone();
            spawn_local(async move {
                let resultat = session::client()
                    .definir_responsables_groupe(id, &liste)
                    .await;
                en_cours.set(false);
                confirmation.set(false);
                match resultat {
                    Ok(r) => {
                        let nom = nom_groupe.with_value(|n| {
                            n.iter()
                                .find(|(g, _)| *g == id)
                                .map(|(_, l)| l.clone())
                                .unwrap_or_default()
                        });
                        let texte = format!("Responsables de « {nom} » enregistrés ({}).", r.len());
                        notifications.succes(texte.clone());
                        annonce.set(texte);
                    }
                    Err(ApiError::NonConnecte) => reconnecter(),
                    Err(e) => {
                        let m = message_erreur_responsables(&e);
                        annonce.set(m.clone());
                        erreur.set(Some(m));
                    }
                }
            });
        })
    };

    view! {
        <form
            class="securite__bloc"
            novalidate=true
            on:submit=move |ev| {
                ev.prevent_default();
                if groupe_id.get_untracked().is_some() {
                    confirmation.set(true);
                }
            }
        >
            <Alerte niveau=Niveau::Info titre="L'enregistrement remplace la liste des responsables du groupe.">
                <p>"La liste actuelle n'est pas affichée : cochez toutes les personnes qui doivent être responsables."</p>
            </Alerte>
            <Selection id="groupe-choix" libelle="Groupe ou atelier" valeur=groupe options=options requis=true/>
            <ChampTexte id="responsable-recherche" libelle="Filtrer le personnel" valeur=recherche genre="search"
                aide="Nom ou adresse e-mail."/>
            <fieldset class="roles__cibles-champ">
                <legend>"Responsables"</legend>
                <div class="roles__cibles-liste">
                    {move || {
                        propositions
                            .get()
                            .into_iter()
                            .map(|p| {
                                let uid = p.user_id;
                                let id = format!("responsable-{uid}");
                                let pour = id.clone();
                                view! {
                                    <label class="ui-case" for=pour>
                                        <input
                                            id=id
                                            type="checkbox"
                                            prop:checked=move || choisis.with(|c| c.contains(&uid))
                                            on:change=move |ev| {
                                                let coche = event_target_checked(&ev);
                                                choisis.update(|c| {
                                                    if coche { c.insert(uid); } else { c.remove(&uid); }
                                                });
                                            }
                                        />
                                        <span>{format!("{} — {}", p.label, p.email)}</span>
                                    </label>
                                }
                            })
                            .collect_view()
                    }}
                </div>
            </fieldset>
            <p class="securite__aide">{move || format!("{} personne(s) sélectionnée(s).", choisis.with(|c| c.len()))}</p>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            <div>
                <Bouton soumettre=true variante=Variante::Primaire en_cours=en_cours
                    desactive=Signal::derive(move || groupe_id.get().is_none())>
                    "Enregistrer les responsables"
                </Bouton>
            </div>
        </form>
        <Dialogue ouvert=confirmation titre="Remplacer les responsables ?" id="dialogue-responsables">
            <p>
                {move || {
                    let n = choisis.with(|c| c.len());
                    if n == 0 {
                        "Plus aucune personne ne sera responsable de ce groupe.".to_string()
                    } else {
                        format!("Les {n} personne(s) cochées deviennent les seules responsables de ce groupe.")
                    }
                }}
            </p>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| confirmation.set(false))>
                    "Annuler"
                </Bouton>
                <Bouton variante=Variante::Primaire en_cours=en_cours on_click=enregistrer>"Confirmer"</Bouton>
            </div>
        </Dialogue>
        <Annonce message=Signal::derive(move || annonce.get())/>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_responsables_sans_corps_brut() {
        assert!(
            message_erreur_responsables(&ApiError::Interdit("x".into())).contains("pas le droit")
        );
        assert!(message_erreur_responsables(&ApiError::Invalide("y".into())).contains("personnel"));
        assert!(
            !message_erreur_responsables(&ApiError::Invalide("secret".into())).contains("secret")
        );
    }
}
