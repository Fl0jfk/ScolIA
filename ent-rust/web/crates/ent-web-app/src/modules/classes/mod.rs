//! Classes : référentiel scolaire (liste, détail, « Mes classes » de l'enseignant).
//!
//! Données : routes `/referentiel/*` d'`ent-api` ; droits, effectifs et minimisation de
//! l'identité sont décidés par le serveur. L'écran n'ajoute aucune règle métier : le bouton
//! « Amorcer » n'est proposé qu'au personnel de gestion, et un refus serveur (403) reste
//! traité comme tel.

pub mod modele;

use ent_web_api::{ApiError, ClasseApi, ClasseDetailApi};
use ent_web_ui::{
    Annonce, Badge, Bouton, EnTetePage, EtatVide, Icone, LienBouton, NomIcone, Recherche,
    Squelette, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use modele::{
    a_des_dates_naissance, description_classe, enseignant_du_service, filtrer_classes,
    identite_eleve, libelle_effectif, matiere_du_service, resume_amorcage, trier_classes,
};

fn adresse_classe(id: Uuid) -> String {
    format!("{}/{id}", navigation::CLASSES)
}

/// Écran « Classes » : toutes les classes de l'année courante.
#[component]
pub fn PageClasses() -> impl IntoView {
    let droits = use_droits();
    let notifications = ent_web_ui::use_notifications();
    let classes = LocalResource::new(|| async {
        session::client()
            .classes(aujourdhui())
            .await
            .map(trier_classes)
    });
    let recherche = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let annonce = RwSignal::new(String::new());

    let amorcer = Callback::new(move |_| {
        if en_cours.get_untracked() {
            return;
        }
        en_cours.set(true);
        spawn_local(async move {
            match session::client().amorcer_referentiel().await {
                Ok(rapport) => {
                    let texte = resume_amorcage(&rapport);
                    notifications.succes(texte.clone());
                    annonce.set(texte);
                    classes.refetch();
                }
                Err(e) => annonce.set(e.message_utilisateur()),
            }
            en_cours.set(false);
        });
    });

    view! {
        <Title text="Classes"/>
        <EnTetePage
            surtitre="Établissement".to_string()
            titre="Classes".to_string()
            sous_titre="Classes de l'année en cours, avec leur effectif.".to_string()
        >
            {move || {
                droits
                    .get()
                    .gerer_classes
                    .then(|| {
                        view! {
                            <Bouton variante=Variante::Secondaire en_cours=en_cours on_click=amorcer>
                                "Amorcer depuis l'existant"
                            </Bouton>
                        }
                    })
            }}
        </EnTetePage>
        <Annonce message=annonce/>
        {move || match classes.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement des classes…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| classes.refetch())/> }.into_any(),
            Some(Ok(liste)) => view! { <TableauClasses classes=liste recherche=recherche/> }.into_any(),
        }}
    }
}

#[component]
fn TableauClasses(classes: Vec<ClasseApi>, recherche: RwSignal<String>) -> impl IntoView {
    let droits = use_droits();
    if classes.is_empty() {
        return view! {
            <EtatVide
                titre="Aucune classe pour cette année"
                message="Le référentiel est vide. Les classes sont déduites des élèves et de l'emploi du temps existants."
            >
                {move || {
                    droits
                        .get()
                        .gerer_classes
                        .then(|| view! { <p>"Utilisez « Amorcer depuis l'existant » pour les créer."</p> })
                }}
            </EtatVide>
        }
        .into_any();
    }
    let toutes = StoredValue::new(classes);
    let visibles = Memo::new(move |_| {
        let q = recherche.get();
        toutes.with_value(|c| filtrer_classes(c, &q))
    });
    view! {
        <div class="ui-tableau-cadre">
            <div class="ui-tableau-outils">
                <Recherche id="recherche-classe" libelle="Rechercher une classe" valeur=recherche/>
                <span class="classes-compteur" role="status">
                    {move || {
                        let n = visibles.get().len();
                        if n == 1 { "1 classe".to_string() } else { format!("{n} classes") }
                    }}
                </span>
            </div>
            <div class="ui-tableau-defile">
                <table class="ui-tableau">
                    <caption class="ui-sr">"Classes de l'année en cours"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Classe"</th>
                            <th scope="col">"Niveau"</th>
                            <th scope="col">"Effectif"</th>
                            <th scope="col">"Professeur principal"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || visibles.get()
                            key=|c| c.id
                            children=|c| {
                                let lien = adresse_classe(c.id);
                                view! {
                                    <tr>
                                        <th scope="row"><a href=lien>{c.code.clone()}</a></th>
                                        <td>{c.niveau.clone().unwrap_or_else(|| "—".into())}</td>
                                        <td>{libelle_effectif(c.effectif)}</td>
                                        <td>{c.professeur_principal_label.clone().unwrap_or_else(|| "—".into())}</td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>
        </div>
        {move || {
            visibles
                .get()
                .is_empty()
                .then(|| view! { <EtatVide titre="Aucune classe ne correspond à la recherche"/> })
        }}
    }
    .into_any()
}

/// Écran « Mes classes » : classes de l'enseignant, accès à l'emploi du temps et à l'appel.
#[component]
pub fn PageMesClasses() -> impl IntoView {
    let classes = LocalResource::new(|| async {
        session::client()
            .mes_classes(aujourdhui())
            .await
            .map(trier_classes)
    });
    view! {
        <Title text="Mes classes"/>
        <EnTetePage
            surtitre="Enseignement".to_string()
            titre="Mes classes".to_string()
            sous_titre="Les classes où vous avez un service d'enseignement.".to_string()
        />
        {move || match classes.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de vos classes…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| classes.refetch())/> }.into_any(),
            Some(Ok(liste)) if liste.is_empty() => view! {
                <EtatVide
                    titre="Aucune classe"
                    message="Aucun service d'enseignement ne vous est rattaché pour cette année. Demandez au secrétariat de vérifier le référentiel."
                />
            }.into_any(),
            Some(Ok(liste)) => view! {
                <ul class="classes-cartes">
                    {liste
                        .into_iter()
                        .map(|c| view! { <CarteClasse classe=c/> })
                        .collect_view()}
                </ul>
            }.into_any(),
        }}
    }
}

#[component]
fn CarteClasse(classe: ClasseApi) -> impl IntoView {
    let lien = adresse_classe(classe.id);
    let titre_id = format!("classe-{}", classe.id);
    view! {
        <li class="classe-carte">
            <h2 id=titre_id.clone() class="classe-carte__titre">
                <a href=lien>{classe.code.clone()}</a>
            </h2>
            <p class="classe-carte__detail">{description_classe(&classe)}</p>
            <nav class="classe-carte__actions" aria-labelledby=titre_id>
                <LienBouton href=navigation::EMPLOI_DU_TEMPS variante=Variante::Secondaire taille=ent_web_ui::Taille::Compacte>
                    <Icone nom=NomIcone::Agenda petite=true/>
                    "Emploi du temps"
                </LienBouton>
                <LienBouton href=navigation::APPEL variante=Variante::Secondaire taille=ent_web_ui::Taille::Compacte>
                    <Icone nom=NomIcone::Appel petite=true/>
                    "Faire l'appel"
                </LienBouton>
            </nav>
        </li>
    }
}

/// Détail d'une classe : élèves affectés et services d'enseignement.
#[component]
pub fn PageClasse() -> impl IntoView {
    let params = use_params_map();
    let id = Memo::new(move |_| {
        params
            .with(|p| p.get("id"))
            .and_then(|i| i.parse::<Uuid>().ok())
    });
    let detail = LocalResource::new(move || {
        let id = id.get();
        async move {
            match id {
                Some(id) => session::client().classe(id, aujourdhui()).await,
                None => Err(ApiError::Introuvable(
                    "identifiant de classe invalide".into(),
                )),
            }
        }
    });
    view! {
        <Title text="Classe"/>
        <nav class="fil-ariane" aria-label="Fil d'Ariane">
            <a href=navigation::ACCUEIL>"Aujourd'hui"</a>
            <span aria-hidden="true">" › "</span>
            <a href=navigation::CLASSES>"Classes"</a>
            <span aria-hidden="true">" › "</span>
            <span aria-current="page">
                {move || match detail.get() {
                    Some(Ok(d)) => d.classe.code,
                    _ => "Classe".to_string(),
                }}
            </span>
        </nav>
        {move || match detail.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement de la classe…"/> }.into_any(),
            Some(Err(ApiError::Introuvable(_))) => view! {
                <EtatVide titre="Classe introuvable" message="Cette classe n'existe pas ou n'appartient pas à votre établissement.">
                    <LienBouton href=navigation::CLASSES variante=Variante::Secondaire>"Retour aux classes"</LienBouton>
                </EtatVide>
            }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| detail.refetch())/> }.into_any(),
            Some(Ok(d)) => view! { <DetailClasse detail=d/> }.into_any(),
        }}
    }
}

#[component]
fn DetailClasse(detail: ClasseDetailApi) -> impl IntoView {
    let titre = detail.classe.code.clone();
    let description = description_classe(&detail.classe);
    let avec_naissance = a_des_dates_naissance(&detail.eleves);
    let eleves = detail.eleves;
    let services = detail.services;
    let groupes = detail.groupes;
    view! {
        <EnTetePage titre=titre sous_titre=description>
            <LienBouton href=navigation::EMPLOI_DU_TEMPS variante=Variante::Secondaire>"Emploi du temps"</LienBouton>
        </EnTetePage>
        <section class="classe-section" aria-labelledby="classe-eleves">
            <h2 id="classe-eleves">"Élèves"</h2>
            {if eleves.is_empty() {
                view! { <EtatVide titre="Aucun élève affecté" message="Aucun élève n'est affecté à cette classe aujourd'hui."/> }.into_any()
            } else {
                view! {
                    <div class="ui-tableau-cadre">
                        <div class="ui-tableau-defile">
                            <table class="ui-tableau">
                                <caption class="ui-sr">"Élèves de la classe"</caption>
                                <thead>
                                    <tr>
                                        <th scope="col">"Élève"</th>
                                        {avec_naissance.then(|| view! { <th scope="col">"Date de naissance"</th> })}
                                    </tr>
                                </thead>
                                <tbody>
                                    {eleves
                                        .into_iter()
                                        .map(|e| {
                                            let naissance = e
                                                .date_naissance
                                                .map(|d| d.format("%d/%m/%Y").to_string())
                                                .unwrap_or_else(|| "—".into());
                                            view! {
                                                <tr>
                                                    <th scope="row">
                                                        <a href=format!("{}/{}", navigation::ELEVES, e.id)>{identite_eleve(&e)}</a>
                                                    </th>
                                                    {avec_naissance.then(|| view! { <td>{naissance}</td> })}
                                                </tr>
                                            }
                                        })
                                        .collect_view()}
                                </tbody>
                            </table>
                        </div>
                    </div>
                }
                .into_any()
            }}
        </section>
        <section class="classe-section" aria-labelledby="classe-services">
            <h2 id="classe-services">"Enseignements"</h2>
            {if services.is_empty() {
                view! { <EtatVide titre="Aucun service d'enseignement" message="Aucun enseignant n'est rattaché à cette classe."/> }.into_any()
            } else {
                view! {
                    <div class="ui-tableau-cadre">
                        <div class="ui-tableau-defile">
                            <table class="ui-tableau">
                                <caption class="ui-sr">"Services d'enseignement de la classe"</caption>
                                <thead>
                                    <tr>
                                        <th scope="col">"Matière"</th>
                                        <th scope="col">"Enseignant"</th>
                                        <th scope="col">"Rôle"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {services
                                        .into_iter()
                                        .map(|s| {
                                            view! {
                                                <tr>
                                                    <th scope="row">{matiere_du_service(&s)}</th>
                                                    <td>{enseignant_du_service(&s)}</td>
                                                    <td>
                                                        {s.professeur_principal.then(|| view! { <Badge libelle="Professeur principal" ton=TonEtat::Info/> })}
                                                    </td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()}
                                </tbody>
                            </table>
                        </div>
                    </div>
                }
                .into_any()
            }}
        </section>
        {(!groupes.is_empty()).then(|| view! {
            <section class="classe-section" aria-labelledby="classe-groupes">
                <h2 id="classe-groupes">"Groupes"</h2>
                <ul class="classes-groupes">
                    {groupes
                        .into_iter()
                        .map(|g| view! { <li>{format!("{} ({})", g.libelle, g.code)}</li> })
                        .collect_view()}
                </ul>
            </section>
        })}
    }
}
