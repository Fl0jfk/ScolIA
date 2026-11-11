//! Accueil « Aujourd'hui » (socle 29, lot P1) : la page est **composée par le serveur**
//! (`GET /portail/aujourdhui`, une requête) ; chaque profil reçoit ses tuiles, sous ses droits.
//! Le navigateur ne décide d'aucun droit et n'assemble rien : il affiche. Une source pas encore
//! codée n'a pas de tuile ; une source en erreur donne « indisponible, réessayer » sans faire
//! tomber la page (R-PO12). Aucune note n'est transportée (affichage sur geste, DP-03).

pub mod modele;

use ent_web_api::types_portail::{
    ActionPortail, EtatTuile, LignePortail, PortailAujourdhui, TuilePortail,
};
use ent_web_api::{ApiError, Creneau};
use ent_web_ui::{
    Bouton, EnTetePage, EtatVide, GrilleBento, Largeur, LienBouton, Squelette, Tuile, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;

use crate::composants_metier::seances::{libelle_moment, lien_appel, prochaine, trier, Moment};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue, heure_courte, maintenant};

#[component]
pub fn PageAccueil() -> impl IntoView {
    let jour = aujourdhui();
    let portail = LocalResource::new(|| async { session::client().portail_aujourdhui().await });

    view! {
        <Title text="Aujourd'hui"/>
        <EnTetePage
            surtitre=date_longue(jour)
            titre="Bonjour".to_string()
            sous_titre="Voici votre journée.".to_string()
        />
        {move || match portail.get() {
            None => view! {
                <GrilleBento libelle="Tableau de bord">
                    <Tuile id="chargement" largeur=Largeur::Douze>
                        <Squelette lignes=4 libelle="Chargement de votre journée…"/>
                    </Tuile>
                </GrilleBento>
            }
                .into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| portail.refetch())/>
            }
                .into_any(),
            Some(Ok(page)) => view! { <PortailRendu page=page jour=jour on_reessayer=Callback::new(move |_| portail.refetch())/> }.into_any(),
        }}
    }
}

#[component]
fn PortailRendu(
    page: PortailAujourdhui,
    jour: chrono::NaiveDate,
    on_reessayer: Callback<()>,
) -> impl IntoView {
    let droits = use_droits();
    let prof = modele::est_professeur(page.profil);
    let vide = page.tuiles.is_empty();
    let tuiles = page.tuiles;
    view! {
        <GrilleBento libelle="Tableau de bord">
            {move || (prof && droits.get().faire_appel).then(|| view! { <BlocProchainCours jour=jour/> })}
            {tuiles
                .into_iter()
                .map(|t| view! { <TuileServeur tuile=t on_reessayer=on_reessayer/> })
                .collect_view()}
            {vide.then(|| view! {
                <Tuile id="rien" largeur=Largeur::Douze>
                    <EtatVide titre="Rien à afficher pour l'instant" message="Votre accueil se remplira au fil des modules ouverts à votre profil."/>
                </Tuile>
            })}
        </GrilleBento>
    }
}

/// Séance en cours ou prochaine du professeur (existant conservé : « Faire l'appel »).
#[component]
fn BlocProchainCours(jour: chrono::NaiveDate) -> impl IntoView {
    let creneaux =
        LocalResource::new(
            move || async move { session::client().mes_creneaux(jour).await.map(trier) },
        );
    view! { <TuileProchainCours creneaux=creneaux jour=jour/> }
}

/// Une tuile rendue par le serveur : une information, une action, un état vide signifiant.
#[component]
fn TuileServeur(tuile: TuilePortail, on_reessayer: Callback<()>) -> impl IntoView {
    let id = format!("tuile-{}", tuile.code.replace('.', "-"));
    let largeur = modele::largeur(tuile.largeur);
    let TuilePortail { titre, etat, principal, nombre, lignes, action, .. } = tuile;
    let corps = match etat {
        EtatTuile::Indisponible => view! {
            <p class="texte-attenue">"Indisponible pour le moment."</p>
            <p><Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| on_reessayer.run(()))>"Réessayer"</Bouton></p>
        }
            .into_any(),
        EtatTuile::Vide => view! { <p class="texte-attenue">{principal.unwrap_or_default()}</p> }.into_any(),
        EtatTuile::Ok => view! {
            {principal.map(|p| match nombre {
                Some(n) if n > 0 => view! {
                    <p class="accueil-chiffre">
                        <span class="accueil-chiffre__valeur">{n.to_string()}</span>
                        <span class="accueil-chiffre__libelle">{modele::sans_nombre(&p)}</span>
                    </p>
                }.into_any(),
                _ => view! { <p class="accueil-principal">{p}</p> }.into_any(),
            })}
            <ul class="ui-liste">
                {lignes.into_iter().map(|l| view! { <LigneServeur ligne=l/> }).collect_view()}
            </ul>
            {action.map(|a| view! { <BoutonAction action=a/> })}
        }
            .into_any(),
    };
    view! { <Tuile id=id titre=titre largeur=largeur>{corps}</Tuile> }
}

#[component]
fn BoutonAction(action: ActionPortail) -> impl IntoView {
    view! { <p class="accueil-cours__actions"><LienBouton href=action.lien variante=Variante::Secondaire>{action.libelle}</LienBouton></p> }
}

#[component]
fn LigneServeur(ligne: LignePortail) -> impl IntoView {
    let classe = modele::classe_ton(ligne.ton);
    let contenu = view! {
        <span class="ui-liste__principal">
            <span class="ui-liste__titre">{ligne.libelle}</span>
            {ligne.detail.map(|d| view! { <span class="ui-liste__detail">{d}</span> })}
        </span>
    };
    match ligne.lien {
        Some(href) => view! { <li class=classe><a class="ui-liste__lien" href=href>{contenu}</a></li> }.into_any(),
        None => view! { <li class=classe>{contenu}</li> }.into_any(),
    }
}

#[component]
fn TuileProchainCours(
    creneaux: LocalResource<Result<Vec<Creneau>, ApiError>>,
    jour: chrono::NaiveDate,
) -> impl IntoView {
    let droits = use_droits();
    view! {
        <Tuile id="prochain-cours" largeur=Largeur::Huit>
            {move || match creneaux.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement de votre journée…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| creneaux.refetch())/> }.into_any(),
                Some(Ok(list)) => match prochaine(&list, maintenant()) {
                    Some((c, m)) => {
                        let horaires = format!(
                            "{} – {}{}",
                            heure_courte(&c.heure_debut),
                            heure_courte(&c.heure_fin),
                            c.salle.as_ref().filter(|s| !s.is_empty()).map(|s| format!(" · Salle {s}")).unwrap_or_default(),
                        );
                        let lien = lien_appel(&c, jour);
                        view! {
                            <div class="accueil-cours">
                                <p class="accueil-cours__moment" class:accueil-cours__moment--maintenant=m == Moment::EnCours>
                                    {if m == Moment::EnCours { "Cours en cours".to_string() } else { format!("Prochain cours · {}", libelle_moment(m).to_lowercase()) }}
                                </p>
                                <h2 class="accueil-cours__titre">{format!("{} · {}", c.classe, c.matiere_libelle)}</h2>
                                <p class="accueil-cours__detail">{horaires}</p>
                            </div>
                            <div class="accueil-cours__actions">
                                {move || droits.get().faire_appel.then(|| {
                                    let lien = lien.clone();
                                    view! { <LienBouton href=lien>"Faire l'appel"</LienBouton> }
                                })}
                                <LienBouton href=navigation::EMPLOI_DU_TEMPS variante=Variante::Secondaire>"Emploi du temps"</LienBouton>
                            </div>
                        }
                            .into_any()
                    }
                    None => {
                        let (titre, message) = if list.is_empty() {
                            ("Pas de cours aujourd'hui", "Votre emploi du temps ne compte aucune séance ce jour.")
                        } else {
                            ("Journée terminée", "Toutes vos séances du jour sont passées.")
                        };
                        view! {
                            <EtatVide titre=titre message=message>
                                <LienBouton href=navigation::EMPLOI_DU_TEMPS variante=Variante::Secondaire>"Voir la semaine"</LienBouton>
                            </EtatVide>
                        }
                            .into_any()
                    }
                },
            }}
        </Tuile>
    }
}

