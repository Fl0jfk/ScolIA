//! Éléments communs aux écrans des absences des personnels.

use ent_types::absences_personnels::AbsenceDto;
use ent_web_api::ApiError;
use ent_web_ui::{AccesRefuse, Badge, EnTetePage, Squelette};
use leptos::prelude::*;

use super::modele::{libelle_etat, libelle_periode, nom_agent, ton_etat};
use super::modele_heures::Noms;
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

/// Garde de page : attend la session, refuse hors personnel. Les droits fins (valider, organiser…)
/// restent jugés par le serveur ; un 403 est affiché par `ErreurChargement`.
#[component]
pub fn GardePersonnel(
    titre: &'static str,
    #[prop(optional, into)] sous_titre: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let droits = use_droits();
    let ctx = session::use_session();
    view! {
        {move || match ctx.moi.get() {
            None => view! {
                <EnTetePage titre=titre.to_string()/>
                <Squelette lignes=4 libelle="Chargement…"/>
            }
            .into_any(),
            Some(Err(e)) => view! {
                <EnTetePage titre=titre.to_string()/>
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| ctx.moi.refetch())/>
            }
            .into_any(),
            Some(Ok(_)) if !droits.get().personnel => view! {
                <EnTetePage titre=titre.to_string()/>
                <AccesRefuse message="Cet écran est réservé au personnel de l'établissement."/>
            }
            .into_any(),
            Some(Ok(_)) => view! {
                <EnTetePage titre=titre.to_string() sous_titre=sous_titre.clone().unwrap_or_default()/>
                {children()}
            }
            .into_any(),
        }}
    }
}

/// Une absence en carte : agent, période, état, nature **seulement si l'API la fournit**.
#[component]
pub fn CarteAbsence(a: AbsenceDto, #[prop(optional)] avec_agent: bool) -> impl IntoView {
    let periode = libelle_periode(a.debut, a.fin);
    let href = format!("/web/personnel/absences/{}", a.id);
    let lien_libelle = format!("Ouvrir l'absence : {periode}");
    let titre = if avec_agent {
        nom_agent(&a.agent_label)
    } else {
        periode.clone()
    };
    let sous_titre = avec_agent.then(|| periode.clone());
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{titre}</strong>
                <Badge libelle=libelle_etat(&a.etat) ton=ton_etat(&a.etat)/>
            </div>
            {sous_titre.map(|s| view! { <p class="absp-carte__detail">{s}</p> })}
            {a.nature_libelle.clone().map(|n| view! { <p class="absp-carte__detail">{n}</p> })}
            {a.fin_provisoire.then(|| view! { <p class="absp-carte__detail">"Fin à confirmer"</p> })}
            <p class="absp-carte__actions">
                <a class="absp-lien" href=href aria-label=lien_libelle>"Ouvrir"</a>
            </p>
        </li>
    }
}

/// Noms des agents de l'établissement (annuaire borné à 200). Un échec n'est pas une erreur d'écran : on
/// affiche « Agent non listé » plutôt qu'un identifiant (le serveur reste juge de l'accès à chaque donnée).
pub fn charger_noms() -> LocalResource<Noms> {
    LocalResource::new(|| async {
        match session::client().absp_annuaire().await {
            Ok(a) => Noms::depuis_annuaire(&a),
            Err(_) => Noms::default(),
        }
    })
}

/// Lance un export (CSV) et propose l'enregistrement ; un échec s'affiche dans `erreur`, jamais en silence.
pub fn lancer_export<F>(
    futur: F,
    nom: &'static str,
    erreur: RwSignal<Option<String>>,
    ecriture: crate::noyau::ecriture::Ecriture,
) where
    F: std::future::Future<Output = Result<Vec<u8>, ApiError>> + 'static,
{
    erreur.set(None);
    leptos::task::spawn_local(async move {
        match futur.await {
            Ok(octets) => {
                match crate::noyau::telechargement::enregistrer_telechargement(
                    nom, "text/csv", &octets,
                ) {
                    Ok(()) => ecriture.annonce.set(format!("Fichier {nom} enregistré.")),
                    Err(m) => erreur.set(Some(m)),
                }
            }
            Err(e) => erreur.set(Some(crate::noyau::ecriture::message_echec(e))),
        }
    });
}
