//! Écrans des files des personnels (socle 29, lot P4) : « Mes files » (des nombres), le contenu d'une
//! file (qui a pris quoi, prendre, relâcher, préparer, réattribuer) et la supervision par nombres.
//! Routes : `GET /afaire/files`, `GET /afaire/files/{code}`, `POST /afaire/{id}/prendre|relacher|preparee|
//! reattribuer`, `GET /afaire/supervision`, `GET /afaire/orphelines`.
//! Le serveur décide de tout (membres calculés, première prise gagnante, droits) ; l'écran n'affiche que
//! ce qu'il reçoit, et un « déjà prise » est dit en clair, jamais contourné.

pub mod modele;

use ent_types::afaire::{
    ActionTache, Collegue, FileAfaire, MesFiles, NiveauPrise, Orpheline, Supervision, TacheFile,
    TuileFile,
};
use ent_web_ui::{
    Alerte, Bouton, EnTetePage, EtatVide, Niveau, Selection, Squelette, Taille, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use modele::{
    libelle_delai, libelle_etat, libelle_plus_ancienne, lien_tuile, peut_prendre, prise_par_moi,
    resume_tuile,
};

// ───────────────────────────── Mes files ─────────────────────────────

#[component]
pub fn PageMesFiles() -> impl IntoView {
    let files = LocalResource::new(|| async { session::client().mes_files().await });
    view! {
        <Title text="Mes files"/>
        <EnTetePage
            titre="Mes files".to_string()
            sous_titre="Ce que votre service doit traiter, sans doublon entre collègues.".to_string()
        />
        {move || match files.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de vos files…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| files.refetch())/>
            }.into_any(),
            Some(Ok(m)) => view! { <Tuiles files=m/> }.into_any(),
        }}
    }
}

#[component]
fn Tuiles(files: MesFiles) -> impl IntoView {
    if files.tuiles.is_empty() {
        return view! { <EtatVide titre="Vous n'êtes membre d'aucune file"/> }.into_any();
    }
    let maintenant = chrono::Utc::now();
    view! {
        <ul class="ui-liste">
            {files.tuiles.into_iter().map(|t| view! { <Tuile tuile=t maintenant=maintenant/> }).collect_view()}
        </ul>
    }
    .into_any()
}

#[component]
fn Tuile(tuile: TuileFile, maintenant: chrono::DateTime<chrono::Utc>) -> impl IntoView {
    let resume = resume_tuile(&tuile);
    let ancien = libelle_plus_ancienne(tuile.plus_ancienne, maintenant);
    let lien = lien_tuile(&tuile);
    let libelle = tuile.libelle.clone();
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{tuile.libelle}</span>
                <span class="ui-liste__detail">
                    {resume}
                    {ancien.map(|a| format!(" · {a}")).unwrap_or_default()}
                </span>
            </div>
            {lien.map(|l| view! {
                <a class="ui-bouton ui-bouton--secondaire" href=l aria-label=format!("Ouvrir la file : {libelle}")>"Ouvrir"</a>
            })}
        </li>
    }
}

// ───────────────────────────── Une file ─────────────────────────────

#[component]
pub fn PageFile() -> impl IntoView {
    let params = use_params_map();
    let code = move || params.with(|p| p.get("code").unwrap_or_default());
    let file = LocalResource::new(move || {
        let c = code();
        async move { session::client().file_afaire(&c).await }
    });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| file.refetch()));
    view! {
        <Title text="File"/>
        <RetourEcriture ecriture=ecriture/>
        {move || match file.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de la file…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| file.refetch())/>
            }.into_any(),
            Some(Ok(f)) => view! { <ContenuFile file=f ecriture=ecriture/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuFile(file: FileAfaire, ecriture: Ecriture) -> impl IntoView {
    let FileAfaire {
        libelle,
        deux_niveaux,
        taches,
        peut_reattribuer,
        collegues,
        ..
    } = file;
    let vide = taches.is_empty();
    let collegues = StoredValue::new(collegues);
    view! {
        <EnTetePage
            titre=libelle
            sous_titre="Une tâche prise par un collègue n'est plus à vous : première prise gagnante.".to_string()
        />
        <p><a class="ui-bouton ui-bouton--secondaire" href="/web/mes-files">"Toutes mes files"</a></p>
        {if vide {
            view! { <EtatVide titre="Rien à traiter dans cette file"/> }.into_any()
        } else {
            view! {
                <ul class="ui-liste">
                    {taches.into_iter().map(|t| view! {
                        <LigneTache tache=t deux_niveaux=deux_niveaux peut_reattribuer=peut_reattribuer
                            collegues=collegues ecriture=ecriture/>
                    }).collect_view()}
                </ul>
            }.into_any()
        }}
    }
}

#[component]
fn LigneTache(
    tache: TacheFile,
    deux_niveaux: bool,
    peut_reattribuer: bool,
    collegues: StoredValue<Vec<Collegue>>,
    ecriture: Ecriture,
) -> impl IntoView {
    let id = tache.id;
    let etat = libelle_etat(&tache);
    let libre_decider = peut_prendre(&tache, NiveauPrise::Decider);
    let libre_preparer = deux_niveaux && peut_prendre(&tache, NiveauPrise::Preparer);
    let ma_prise_decider = prise_par_moi(&tache, NiveauPrise::Decider);
    let ma_prise_preparer = prise_par_moi(&tache, NiveauPrise::Preparer);
    let ouvrir = tache.action.clone();
    let titre = tache.titre.clone();
    let reattribution = RwSignal::new(false);
    let prise_niveau = if tache
        .prises
        .iter()
        .any(|p| p.niveau == NiveauPrise::Preparer)
    {
        NiveauPrise::Preparer
    } else {
        NiveauPrise::Decider
    };
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">{tache.titre}</span>
                <span class="ui-liste__detail">{etat}</span>
            </div>
            {libre_decider.then(|| view! {
                <Bouton variante=Variante::Primaire taille=Taille::Compacte
                    libelle_accessible=format!("Prendre en charge : {titre}")
                    on_click=Callback::new(move |_| ecriture.lancer(async move {
                        session::client().a_faire_prendre(id, NiveauPrise::Decider).await
                            .map(|()| "Tâche prise en charge.".to_string())
                    }))>"Prendre"</Bouton>
            })}
            {libre_preparer.then(|| view! {
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                    on_click=Callback::new(move |_| ecriture.lancer(async move {
                        session::client().a_faire_prendre(id, NiveauPrise::Preparer).await
                            .map(|()| "Préparation prise en charge.".to_string())
                    }))>"Préparer"</Bouton>
            })}
            {ma_prise_preparer.then(|| view! {
                <Bouton variante=Variante::Primaire taille=Taille::Compacte
                    on_click=Callback::new(move |_| ecriture.lancer(async move {
                        session::client().a_faire_preparee(id).await
                            .map(|()| "Remise à celui qui décide.".to_string())
                    }))>"Préparation terminée"</Bouton>
            })}
            {(ma_prise_decider || ma_prise_preparer).then(|| view! {
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                    on_click=Callback::new(move |_| ecriture.lancer(async move {
                        session::client().a_faire_relacher(id).await
                            .map(|()| "Tâche relâchée.".to_string())
                    }))>"Relâcher"</Bouton>
            })}
            {ouvrir.map(|a| match a {
                ActionTache::Lien { libelle, lien } => view! {
                    <a class="ui-bouton ui-bouton--secondaire" href=lien>{libelle}</a>
                }.into_any(),
                ActionTache::Carte { .. } => ().into_any(),
            })}
            {peut_reattribuer.then(|| view! {
                <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                    on_click=Callback::new(move |_| reattribution.update(|r| *r = !*r))>"Réattribuer"</Bouton>
            })}
        </li>
        {move || reattribution.get().then(|| view! {
            <li class="ui-liste__element">
                <Reattribution tache_id=id niveau=prise_niveau collegues=collegues
                    ferme=reattribution ecriture=ecriture/>
            </li>
        })}
    }
}

/// Choix d'un collègue de la file (liste fermée fournie par le serveur), puis confirmation.
#[component]
fn Reattribution(
    tache_id: Uuid,
    niveau: NiveauPrise,
    collegues: StoredValue<Vec<Collegue>>,
    ferme: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let choix = RwSignal::new(String::new());
    let options: Vec<(String, String)> =
        std::iter::once((String::new(), "Choisir un collègue…".to_string()))
            .chain(
                collegues
                    .get_value()
                    .into_iter()
                    .map(|c| (c.compte_id.to_string(), c.nom)),
            )
            .collect();
    view! {
        <div role="group" aria-label="Réattribuer la tâche">
            <Selection id=format!("reattribuer-{tache_id}") libelle="Collègue de la file"
                valeur=choix options=options requis=true/>
            <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| {
                let Ok(vers) = choix.get_untracked().parse::<Uuid>() else { return };
                ferme.set(false);
                ecriture.lancer(async move {
                    session::client().a_faire_reattribuer(tache_id, vers, niveau).await
                        .map(|()| "Tâche réattribuée.".to_string())
                });
            })>"Confirmer"</Bouton>
            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ferme.set(false))>"Annuler"</Bouton>
        </div>
    }
}

// ───────────────────────────── Supervision ─────────────────────────────

#[component]
pub fn PageSupervisionFiles() -> impl IntoView {
    let supervision =
        LocalResource::new(|| async { session::client().a_faire_supervision().await });
    let orphelines = LocalResource::new(|| async { session::client().a_faire_orphelines().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        supervision.refetch();
        orphelines.refetch();
    }));
    view! {
        <Title text="Supervision des files"/>
        <EnTetePage
            titre="Supervision des files".to_string()
            sous_titre="Des nombres par file : jamais de nom, jamais de classement.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match supervision.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des nombres…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| supervision.refetch())/>
            }.into_any(),
            Some(Ok(s)) => view! { <Nombres supervision=s/> }.into_any(),
        }}
        {move || match orphelines.get() {
            Some(Ok(o)) => view! { <Orphelines orphelines=o ecriture=ecriture/> }.into_any(),
            _ => ().into_any(),
        }}
    }
}

#[component]
fn Nombres(supervision: Supervision) -> impl IntoView {
    let Supervision { lignes, orphelines } = supervision;
    view! {
        {(orphelines > 0).then(|| view! {
            <Alerte niveau=Niveau::Avertissement titre=format!("{orphelines} tâche(s) sans destinataire servi : voir ci-dessous.")/>
        })}
        {if lignes.is_empty() {
            view! { <EtatVide titre="Aucune tâche ouverte dans les files"/> }.into_any()
        } else {
            view! {
                <div class="ui-tableau-cadre">
                    <table class="ui-tableau">
                        <caption class="visuellement-cache">"Tâches ouvertes par file et par nature"</caption>
                        <thead>
                            <tr>
                                <th scope="col">"File"</th>
                                <th scope="col">"Nature"</th>
                                <th scope="col">"Ouvertes"</th>
                                <th scope="col">"En retard"</th>
                                <th scope="col">"Délai médian (7 jours)"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {lignes.into_iter().map(|l| view! {
                                <tr>
                                    <th scope="row">{l.libelle}</th>
                                    <td>{l.nature}</td>
                                    <td>{l.ouvertes}</td>
                                    <td>{l.en_retard}</td>
                                    <td>{libelle_delai(l.delai_median_minutes)}</td>
                                </tr>
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            }.into_any()
        }}
    }
}

#[component]
fn Orphelines(orphelines: Vec<Orpheline>, ecriture: Ecriture) -> impl IntoView {
    if orphelines.is_empty() {
        return ().into_any();
    }
    view! {
        <section aria-labelledby="orphelines-titre">
            <h2 id="orphelines-titre">"Tâches sans destinataire servi"</h2>
            <p class="texte-attenue">
                "Le rapprochement les rend à la file de repli. Une tâche n'est close « sans objet » qu'après vérification auprès de son propriétaire."
            </p>
            <ul class="ui-liste">
                {orphelines.into_iter().map(|o| {
                    let id = o.id;
                    view! {
                        <li class="ui-liste__element">
                            <div class="ui-liste__principal"><span class="ui-liste__titre">{o.titre}</span></div>
                            <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                on_click=Callback::new(move |_| ecriture.lancer(async move {
                                    session::client().a_faire_clore_orpheline(id).await
                                        .map(|()| "Tâche close sans objet.".to_string())
                                }))>"Clore sans objet"</Bouton>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </section>
    }
    .into_any()
}
