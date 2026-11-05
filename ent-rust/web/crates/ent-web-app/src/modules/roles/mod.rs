//! Administration des rôles (D-017) : liste, création / modification d'un rôle avec son
//! tableau « action × portée », membres, responsables des groupes et ateliers.
//!
//! Routes d'`ent-api` utilisées, inchangées : `/roles/*`, `/referentiel/groupes/{id}/responsables`,
//! `/personnel/search`, `/referentiel/classes*`. Le serveur reste l'autorité (droit
//! `roles.gerer`, anti-escalade, portées admises) : l'écran n'est proposé qu'aux personnes
//! dont `/roles/mes-droits` contient `roles.gerer`, et chaque refus est expliqué.

mod editeur;
mod groupes;
pub mod modele;

pub use editeur::{PageNouveauRole, PageRole};
pub use groupes::PageResponsables;

use ent_web_api::RoleApi;
use ent_web_ui::{
    classes_bouton, AccesRefuse, Badge, EnTetePage, EtatVide, LienBouton, Squelette, Taille,
    TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_location, use_navigate};
use uuid::Uuid;

use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use modele::{peut_gerer_roles, resume_role, trier_roles};

/// Adresse de l'édition d'un rôle.
pub(crate) fn adresse_role(id: Uuid) -> String {
    format!("{}/{id}", navigation::ROLES)
}

/// Retour à la connexion après un 401 survenu pendant une action.
pub(crate) fn retour_connexion() -> impl Fn() + Clone {
    let navigate = use_navigate();
    let location = use_location();
    move || {
        session::effacer();
        let chemin = session::chemin_complet(
            &location.pathname.get_untracked(),
            &location.search.get_untracked(),
        );
        navigate(&session::adresse_connexion(&chemin), Default::default());
    }
}

/// Réserve ses enfants aux personnes qui ont `roles.gerer` (d'après `/roles/mes-droits`).
#[component]
pub(crate) fn GardeRoles(children: ChildrenFn) -> impl IntoView {
    let droits = LocalResource::new(|| async { session::client().mes_droits().await });
    view! {
        {move || match droits.get() {
            None => view! { <Squelette lignes=4 libelle="Vérification de vos droits…"/> }.into_any(),
            Some(Err(e)) => {
                view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| droits.refetch())/> }
                    .into_any()
            }
            Some(Ok(d)) if peut_gerer_roles(&d) => children().into_any(),
            Some(Ok(_)) => view! {
                <AccesRefuse message="La gestion des rôles est réservée aux personnes qui ont ce droit (administration, direction)."/>
            }
            .into_any(),
        }}
    }
}

/// Écran « Rôles et droits » : liste des rôles de l'établissement.
#[component]
pub fn PageRoles() -> impl IntoView {
    view! {
        <Title text="Rôles et droits"/>
        <EnTetePage
            titre="Rôles et droits".to_string()
            surtitre="Établissement".to_string()
            sous_titre="Composez les rôles de votre établissement et attribuez-les au personnel.".to_string()
        >
            <LienBouton href=navigation::ROLES_NOUVEAU variante=Variante::Primaire>
                "Nouveau rôle"
            </LienBouton>
            <LienBouton href=navigation::ROLES_GROUPES variante=Variante::Secondaire>
                "Responsables des groupes"
            </LienBouton>
        </EnTetePage>
        <GardeRoles>
            <ListeRoles/>
        </GardeRoles>
    }
}

#[component]
fn ListeRoles() -> impl IntoView {
    let roles = LocalResource::new(|| async { session::client().roles().await.map(trier_roles) });
    view! {
        <p class="roles__aide">
            "Une personne sans rôle attribué garde les droits de son profil. Un rôle attribué s'ajoute à ces droits ; "
            "plusieurs rôles se cumulent. Les familles et les élèves ne reçoivent jamais de rôle."
        </p>
        {move || match roles.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des rôles…"/> }.into_any(),
            Some(Err(e)) => {
                view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| roles.refetch())/> }
                    .into_any()
            }
            Some(Ok(liste)) if liste.is_empty() => view! {
                <EtatVide
                    titre="Aucun rôle créé"
                    message="Créez un rôle à partir d'un modèle (professeur, CPE, secrétariat, comptabilité, accueil…) ou depuis zéro."
                />
            }
            .into_any(),
            Some(Ok(liste)) => view! { <ul class="roles__liste">{liste.into_iter().map(carte_role).collect_view()}</ul> }
                .into_any(),
        }}
    }
}

fn carte_role(r: RoleApi) -> impl IntoView {
    let href = adresse_role(r.id);
    let resume = resume_role(&r);
    let description = r.description.clone();
    let issu = r.modele.as_deref().map(|m| {
        format!(
            "Issu du profil « {} »",
            crate::noyau::droits::libelle_role(m)
        )
    });
    let nom = r.nom.clone();
    let libelle_lien = format!("Modifier le rôle {}", r.nom);
    view! {
        <li class="roles__carte">
            <div class="roles__texte">
                <h2 class="roles__nom">{nom}</h2>
                {description.map(|d| view! { <p class="roles__description">{d}</p> })}
                <p class="roles__meta">
                    <span>{resume}</span>
                    {issu.map(|i| view! { <Badge libelle=i ton=TonEtat::Neutre/> })}
                </p>
            </div>
            <a class=classes_bouton(Variante::Secondaire, Taille::Normale) href=href aria-label=libelle_lien>
                "Modifier"
            </a>
        </li>
    }
}
