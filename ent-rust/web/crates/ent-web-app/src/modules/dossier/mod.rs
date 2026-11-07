//! Fiche élève 360° (socle 2, lot S2-1) : identité, scolarité, responsables et contacts,
//! accès aux sections existantes du dossier, chronologie, actions métier.
//!
//! Routes d'`ent-api` : `GET /dossier/eleves/{id}/fiche|chronologie`, `POST /dossier/personnes`,
//! `POST /dossier/eleves/{id}/responsables|contacts`, `POST /dossier/liens/{id}/suspendre|
//! retablir`, `PUT /dossier/eleves/{id}/payeurs`, `POST|DELETE /dossier/contacts/{id}…`.
//! Le serveur décide de tout (droits, sections, minimisation, actions proposées) : l'écran
//! n'affiche que ce que la fiche contient et que les actions qu'elle annonce.

mod actions;
pub mod modele;

use ent_web_api::{ApiError, EntreeChronologieDto, FicheEleveDto, ResponsableDto};
use ent_web_ui::{
    Alerte, Badge, Bouton, EnTetePage, EtatVide, FiltrePastille, LienBouton, Niveau, Squelette,
    Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use actions::{
    DialogueContact, DialoguePayeurs, DialogueResponsable, DialogueRetablir, DialogueSuspension,
};
use modele::{
    coordonnees, droits_du_lien, etat_lien, filtrer_chronologie, libelle_domaine, libelle_garde,
    lien_action, message_masque, nom_personne, peut, sections_accessibles, EtatLien,
    ACTION_PAYEURS, ACTION_PERSONNES, ACTION_SUSPENDRE,
};

/// Nombre d'entrées de chronologie demandées au serveur.
const CHRONOLOGIE_MAX: usize = 200;

/// Adresse de la fiche d'un élève.
pub fn adresse_fiche(eleve_id: Uuid) -> String {
    format!("{}/{eleve_id}", navigation::ELEVES)
}

#[component]
pub fn PageFicheEleve() -> impl IntoView {
    let params = use_params_map();
    let id = Memo::new(move |_| {
        params
            .with(|p| p.get("id"))
            .and_then(|i| i.parse::<Uuid>().ok())
    });
    let fiche = LocalResource::new(move || {
        let id = id.get();
        async move {
            match id {
                Some(id) => session::client().fiche_dossier(id).await,
                None => Err(ApiError::Introuvable("identifiant d'élève invalide".into())),
            }
        }
    });
    let recharger = Callback::new(move |_| fiche.refetch());
    view! {
        <Title text="Dossier de l'élève"/>
        <nav class="fil-ariane" aria-label="Fil d'Ariane">
            <a href=navigation::ACCUEIL>"Aujourd'hui"</a>
            <span aria-hidden="true">" › "</span>
            <span aria-current="page">
                {move || match fiche.get() {
                    Some(Ok(f)) => format!("{} {}", f.prenom, f.nom.to_uppercase()),
                    _ => "Dossier de l'élève".to_string(),
                }}
            </span>
        </nav>
        {move || match fiche.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement du dossier…"/> }.into_any(),
            Some(Err(ApiError::Introuvable(_))) => view! {
                <EtatVide titre="Dossier inaccessible" message="Cet élève n'existe pas, n'appartient pas à votre établissement, ou son dossier ne vous est pas ouvert.">
                    <LienBouton href=navigation::ACCUEIL variante=Variante::Secondaire>"Revenir à l'accueil"</LienBouton>
                </EtatVide>
            }
                .into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any(),
            Some(Ok(f)) => view! { <ContenuFiche fiche=f recharger=recharger/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuFiche(fiche: FicheEleveDto, recharger: Callback<()>) -> impl IntoView {
    let famille = use_droits().get_untracked().portail_famille;
    let eleve_id = fiche.eleve_id;
    let titre = format!("{} {}", fiche.prenom, fiche.nom.to_uppercase());
    let surtitre = fiche
        .classe
        .clone()
        .map(|c| format!("Classe {c}"))
        .unwrap_or_else(|| "Sans classe".into());
    let mut details = vec![match fiche.statut.as_str() {
        "actif" => "Scolarisé".to_string(),
        "sorti" => "Sorti de l'établissement".to_string(),
        "preinscrit" => "Préinscrit".to_string(),
        autre => autre.to_string(),
    }];
    if let Some(a) = fiche.age_ans {
        details.push(format!("{a} ans"));
    }
    if fiche.majeur {
        details.push("majeur".into());
    }
    if let Some(r) = fiche.regime.clone().filter(|r| !r.trim().is_empty()) {
        details.push(format!("régime {r}"));
    }
    let sous_titre = details.join(" · ");

    let actions = StoredValue::new(fiche.actions.clone());
    let gerer = peut(&fiche.actions, ACTION_PERSONNES);
    let payeurs = peut(&fiche.actions, ACTION_PAYEURS);
    let suspendre = peut(&fiche.actions, ACTION_SUSPENDRE);
    let peut_ajouter_contact = gerer || famille;

    // Dialogues d'action.
    let ouvrir_responsable = RwSignal::new(false);
    let ouvrir_payeurs = RwSignal::new(false);
    let ouvrir_contact = RwSignal::new(false);
    let a_suspendre = RwSignal::new(None::<(Uuid, String)>);
    let a_retablir = RwSignal::new(None::<(Uuid, String)>);
    let responsables = StoredValue::new(fiche.responsables.clone());
    let sections = sections_accessibles(&fiche, famille);
    let alertes = fiche.alertes.clone();
    let interdictions = fiche.interdictions.clone();
    let contacts = fiche.contacts.clone();
    let liste_responsables = fiche.responsables.clone();

    view! {
        <EnTetePage titre=titre surtitre=surtitre sous_titre=sous_titre>
            {gerer.then(|| view! {
                <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| ouvrir_responsable.set(true))>
                    "Ajouter un responsable"
                </Bouton>
            })}
        </EnTetePage>
        {(!alertes.is_empty()).then(|| view! {
            <Alerte niveau=Niveau::Avertissement titre="Points à vérifier">
                <ul>{alertes.into_iter().map(|a| view! { <li>{a}</li> }).collect_view()}</ul>
            </Alerte>
        })}

        {(!interdictions.is_empty()).then(|| view! {
            <Alerte niveau=Niveau::Erreur titre="Interdictions en cours">
                <ul>{interdictions.into_iter().map(|i| {
                    let periode = match i.au {
                        Some(au) => format!("du {} au {}", i.du, au),
                        None => format!("depuis le {}", i.du),
                    };
                    let nature = if i.bloquante { "" } else { " (signalement à vérifier, non bloquant)" };
                    view! {
                        <li>
                            <strong>{i.personne_libelle}</strong>
                            {format!(" — {} {periode}{nature} : ", i.objet.join(", "))}
                            {i.consigne}
                        </li>
                    }
                }).collect_view()}</ul>
            </Alerte>
        })}

        <section class="classe-section" aria-labelledby="fiche-responsables">
            <div class="fiche-section__tete">
                <h2 id="fiche-responsables">"Responsables"</h2>
                {payeurs.then(|| view! {
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte on_click=Callback::new(move |_| ouvrir_payeurs.set(true))>
                        "Définir les payeurs"
                    </Bouton>
                })}
            </div>
            <TableResponsables
                responsables=liste_responsables
                suspendre=suspendre
                a_suspendre=a_suspendre
                a_retablir=a_retablir
            />
            {gerer.then(|| view! { <actions::LiensEnAttente eleve_id=eleve_id recharger=recharger/> })}
        </section>

        <section class="classe-section" aria-labelledby="fiche-contacts">
            <div class="fiche-section__tete">
                <h2 id="fiche-contacts">"Contacts d'urgence"</h2>
                {peut_ajouter_contact.then(|| view! {
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte on_click=Callback::new(move |_| ouvrir_contact.set(true))>
                        {if gerer { "Ajouter un contact" } else { "Proposer un contact" }}
                    </Bouton>
                })}
            </div>
            <actions::ListeContacts contacts=contacts gerer=gerer recharger=recharger/>
        </section>

        <section class="classe-section" aria-labelledby="fiche-sections">
            <h2 id="fiche-sections">"Dossier"</h2>
            {if sections.is_empty() {
                view! { <p class="texte-attenue">"Aucune autre section du dossier ne vous est ouverte."</p> }.into_any()
            } else {
                view! {
                    <ul class="ui-liste">
                        {sections.into_iter().map(|s| view! {
                            <li class="ui-liste__element">
                                <div class="ui-liste__principal">
                                    <span class="ui-liste__titre">{s.titre}</span>
                                    <span class="ui-liste__detail">{s.detail}</span>
                                </div>
                                {s.lien.map(|l| view! { <a href=l>"Ouvrir"<span class="ui-sr">{format!(" : {}", s.titre)}</span></a> })}
                            </li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>

        {move || {
            let liste = actions.get_value();
            (!liste.is_empty()).then(|| view! {
                <section class="classe-section" aria-labelledby="fiche-actions">
                    <h2 id="fiche-actions">"Actions possibles"</h2>
                    <ul class="ui-liste">
                        {liste.into_iter().map(|a| {
                            let lien = lien_action(&a.code);
                            view! {
                                <li class="ui-liste__element">
                                    <span class="ui-liste__principal">{a.libelle.clone()}</span>
                                    {lien.map(|l| view! { <a href=l>"Ouvrir"<span class="ui-sr">{format!(" : {}", a.libelle)}</span></a> })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                </section>
            })
        }}

        <Chronologie eleve_id=eleve_id/>

        <DialogueResponsable ouvert=ouvrir_responsable eleve_id=eleve_id recharger=recharger/>
        <DialoguePayeurs ouvert=ouvrir_payeurs eleve_id=eleve_id responsables=responsables recharger=recharger/>
        <DialogueContact ouvert=ouvrir_contact eleve_id=eleve_id proposition=!gerer recharger=recharger/>
        <DialogueSuspension lien=a_suspendre recharger=recharger/>
        <DialogueRetablir lien=a_retablir recharger=recharger/>
    }
}

#[component]
fn TableResponsables(
    responsables: Vec<ResponsableDto>,
    suspendre: bool,
    a_suspendre: RwSignal<Option<(Uuid, String)>>,
    a_retablir: RwSignal<Option<(Uuid, String)>>,
) -> impl IntoView {
    if responsables.is_empty() {
        return view! { <p class="texte-attenue">"Aucun responsable enregistré."</p> }.into_any();
    }
    let jour = aujourdhui();
    view! {
        <div class="ui-tableau-cadre">
            <div class="ui-tableau-defile">
                <table class="ui-tableau">
                    <caption class="ui-sr">"Responsables de l'élève"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Personne"</th>
                            <th scope="col">"Lien"</th>
                            <th scope="col">"État"</th>
                            <th scope="col">"Droits"</th>
                            <th scope="col">"Coordonnées"</th>
                            {suspendre.then(|| view! { <th scope="col"><span class="ui-sr">"Actions"</span></th> })}
                        </tr>
                    </thead>
                    <tbody>
                        {responsables.into_iter().map(|r| {
                            let nom = nom_personne(&r.personne);
                            let etat = etat_lien(&r, jour);
                            let ton = match etat {
                                EtatLien::Actif => TonEtat::Ok,
                                EtatLien::Suspendu => TonEtat::Mal,
                                EtatLien::Termine | EtatLien::AVenir => TonEtat::Neutre,
                            };
                            let mut lien = r.lien.clone();
                            if r.autorite_parentale {
                                lien.push_str(" · autorité parentale");
                            }
                            let garde = libelle_garde(r.garde.as_deref());
                            let droits = droits_du_lien(&r);
                            let droits = if let Some(m) = message_masque(&r.masque, "F3") {
                                m
                            } else if droits.is_empty() {
                                "aucun".to_string()
                            } else {
                                droits.join(", ")
                            };
                            let payeur = r.est_payeur.then(|| format!("payeur {} %", r.part_paiement.unwrap_or(0)));
                            let coords = coordonnees(&r.personne);
                            let mut coords = if let Some(m) = message_masque(&r.masque, "F1") {
                                m
                            } else if coords.is_empty() {
                                "—".to_string()
                            } else {
                                coords.join(" · ")
                            };
                            if let Some(m) = message_masque(&r.masque, "F2") {
                                coords.push_str(&format!(" · adresse : {m}"));
                            }
                            let compte = if r.personne.a_compte { "avec compte" } else { "sans compte" };
                            let (lien_id, pour) = (r.lien_id, nom.clone());
                            let suspendu = r.suspendu_le.is_some();
                            view! {
                                <tr>
                                    <th scope="row">
                                        {nom}
                                        <span class="ui-liste__detail">{format!(" ({compte})")}</span>
                                    </th>
                                    <td>{lien}<br/><span class="texte-attenue">{garde}</span></td>
                                    <td><Badge libelle=etat.libelle() ton=ton/></td>
                                    <td>{droits}{payeur.map(|p| view! { <br/><span class="texte-attenue">{p}</span> })}</td>
                                    <td>{coords}</td>
                                    {suspendre.then(|| view! {
                                        <td>
                                            {if suspendu {
                                                view! {
                                                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                                        libelle_accessible=format!("Rétablir le lien de {pour}")
                                                        on_click=Callback::new(move |_| a_retablir.set(Some((lien_id, pour.clone()))))>
                                                        "Rétablir"
                                                    </Bouton>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <Bouton variante=Variante::Danger taille=Taille::Compacte
                                                        libelle_accessible=format!("Suspendre le lien de {pour}")
                                                        on_click=Callback::new(move |_| a_suspendre.set(Some((lien_id, pour.clone()))))>
                                                        "Suspendre"
                                                    </Bouton>
                                                }.into_any()
                                            }}
                                        </td>
                                    })}
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
    .into_any()
}

#[component]
fn Chronologie(eleve_id: Uuid) -> impl IntoView {
    let chrono = LocalResource::new(move || async move {
        session::client()
            .chronologie_eleve(eleve_id, CHRONOLOGIE_MAX)
            .await
    });
    let domaine = RwSignal::new(None::<String>);
    view! {
        <section class="classe-section" aria-labelledby="fiche-chronologie">
            <h2 id="fiche-chronologie">"Chronologie"</h2>
            {move || match chrono.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement de la chronologie…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_))) => view! {
                    <p class="texte-attenue">"La chronologie de cet élève ne vous est pas ouverte."</p>
                }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| chrono.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <p class="texte-attenue">"Aucun fait enregistré pour le moment."</p> }.into_any(),
                Some(Ok(l)) => view! { <ListeChronologie entrees=l domaine=domaine/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn ListeChronologie(
    entrees: Vec<EntreeChronologieDto>,
    domaine: RwSignal<Option<String>>,
) -> impl IntoView {
    let domaines = modele::domaines_presents(&entrees);
    let total = entrees.len();
    let toutes = StoredValue::new(entrees);
    let visibles = Memo::new(move |_| {
        let d = domaine.get();
        toutes.with_value(|t| filtrer_chronologie(t, d.as_deref()))
    });
    view! {
        <div class="appel-pastilles" role="group" aria-label="Filtrer la chronologie par domaine">
            <FiltrePastille
                libelle="Tout"
                nombre=Signal::derive(move || total)
                actif=Signal::derive(move || domaine.get().is_none())
                teinte="present"
                on_click=Callback::new(move |_| domaine.set(None))
            />
            {domaines.into_iter().map(|(d, n)| {
                let code = d.clone();
                let code_actif = d.clone();
                view! {
                    <FiltrePastille
                        libelle=libelle_domaine(&d)
                        nombre=Signal::derive(move || n)
                        actif=Signal::derive(move || domaine.get().as_deref() == Some(code_actif.as_str()))
                        teinte="en_attente"
                        on_click=Callback::new(move |_| domaine.set(Some(code.clone())))
                    />
                }
            }).collect_view()}
        </div>
        <ol class="ui-liste fiche-chronologie">
            <For
                each=move || visibles.get()
                key=|e| (e.quand, e.titre.clone(), e.source.clone())
                children=move |e| {
                    let date = e.quand.format("%d/%m/%Y").to_string();
                    view! {
                        <li class="ui-liste__element">
                            <span class="fiche-chronologie__date">{date}</span>
                            <div class="ui-liste__principal">
                                <span class="ui-liste__titre">{e.titre.clone()}</span>
                                <span class="ui-liste__detail">{libelle_domaine(&e.domaine)}</span>
                            </div>
                        </li>
                    }
                }
            />
        </ol>
    }
}
