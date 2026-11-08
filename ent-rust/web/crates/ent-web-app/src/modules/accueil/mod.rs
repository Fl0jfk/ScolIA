//! Accueil « Aujourd'hui » : tableau de bord Bento selon le profil.
//! Données : routes existantes (`/edt/mes-creneaux`, `/messages/fils`, `/annonces`),
//! droits vérifiés par le serveur. Pas de tuile « Appels du jour » (route non autorisée).

pub mod modele;

use ent_web_api::{Annonce, ApiError, Creneau};
use ent_web_ui::{
    Badge, EnTetePage, EtatVide, GrilleBento, Largeur, LienBouton, Squelette, TonEtat, Tuile,
    Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;

use crate::composants_metier::absences::{eleves_absents, nom_eleve, Compteurs, StatutAbsence};
use crate::composants_metier::seances::{
    libelle_moment, lien_appel, moment, prochaine, trier, Moment,
};
use crate::noyau::coquille::{use_droits, use_droits_moteur};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue, heure_courte, maintenant};
use modele::{
    fils_recents, libelle_a_justifier, libelle_absents, libelle_en_attente, libelle_non_lus,
};

/// Texte raccourci sans couper un caractère.
fn tronquer(texte: &str, max: usize) -> String {
    let t = texte.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        format!("{}…", t.chars().take(max).collect::<String>().trim_end())
    }
}

#[component]
pub fn PageAccueil() -> impl IntoView {
    let droits = use_droits();
    let moteur = use_droits_moteur();
    let jour = aujourdhui();
    let creneaux =
        LocalResource::new(
            move || async move { session::client().mes_creneaux(jour).await.map(trier) },
        );
    let fils = LocalResource::new(|| async { session::client().fils_messages().await });
    let annonces = LocalResource::new(|| async { session::client().annonces().await });

    view! {
        <Title text="Aujourd'hui"/>
        <EnTetePage
            surtitre=date_longue(jour)
            titre="Bonjour".to_string()
            sous_titre="Voici votre journée.".to_string()
        />
        <GrilleBento libelle="Tableau de bord">
            {move || {
                droits
                    .get()
                    .emploi_du_temps
                    .then(|| {
                        view! {
                            <TuileProchainCours creneaux=creneaux jour=jour/>
                            <TuileSeances creneaux=creneaux/>
                        }
                    })
            }}
            {move || {
                (!droits.get().emploi_du_temps)
                    .then(|| {
                        view! {
                            <Tuile id="espace" titre="Votre espace">
                                <p>
                                    "Votre espace complet arrive dans cette nouvelle interface. "
                                    "En attendant, retrouvez tous les services dans l'interface actuelle."
                                </p>
                                <p><LienBouton href="/app" variante=Variante::Secondaire>"Ouvrir l'interface actuelle"</LienBouton></p>
                            </Tuile>
                        }
                    })
            }}
            {move || droits.get().trancher_justificatif.then(|| view! { <TuileVieScolaire/> })}
            {move || moteur.get().a_etablissement("liens.suspendre").then(|| view! { <TuileProtections/> })}
            {move || droits.get().portail_famille.then(|| view! { <TuileAbsencesFamille/> })}
            <TuileMessages fils=fils/>
            <TuileAnnonces annonces=annonces/>
        </GrilleBento>
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

#[component]
fn TuileSeances(creneaux: LocalResource<Result<Vec<Creneau>, ApiError>>) -> impl IntoView {
    let action = ViewFn::from(|| view! { <a href=navigation::EMPLOI_DU_TEMPS>"Semaine"</a> });
    view! {
        <Tuile id="seances" titre="Séances du jour" largeur=Largeur::Quatre action=action>
            {move || match creneaux.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des séances…"/> }.into_any(),
                Some(Err(_)) => view! { <p class="texte-attenue">"Séances indisponibles."</p> }.into_any(),
                Some(Ok(list)) if list.is_empty() => view! { <p class="texte-attenue">"Aucune séance aujourd'hui."</p> }.into_any(),
                Some(Ok(list)) => {
                    let heure = maintenant();
                    view! {
                        <ul class="ui-liste">
                            {list
                                .into_iter()
                                .map(|c| {
                                    let m = moment(&c, heure);
                                    let detail = c.salle.clone().filter(|s| !s.is_empty()).map(|s| format!("Salle {s}")).unwrap_or_else(|| "Salle non précisée".into());
                                    view! {
                                        <li class="ui-liste__element">
                                            <span class="ui-liste__heure">{heure_courte(&c.heure_debut).to_string()}</span>
                                            <span class="ui-liste__principal">
                                                <span class="ui-liste__titre">{format!("{} · {}", c.classe, c.matiere_libelle)}</span>
                                                <span class="ui-liste__detail">{detail}</span>
                                            </span>
                                            {match m {
                                                Moment::EnCours => Some(view! { <Badge libelle="En cours" ton=TonEtat::Info/> }),
                                                Moment::Terminee => Some(view! { <Badge libelle="Terminé"/> }),
                                                _ => None,
                                            }}
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                        .into_any()
                }
            }}
        </Tuile>
    }
}

#[component]
fn TuileMessages(
    fils: LocalResource<Result<Vec<ent_web_api::FilMessages>, ApiError>>,
) -> impl IntoView {
    let action = ViewFn::from(|| view! { <a href=navigation::MESSAGES>"Ouvrir la messagerie"</a> });
    view! {
        <Tuile id="messages" titre="Messages" largeur=Largeur::Six action=action>
            {move || match fils.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des messages…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_))) => view! { <p class="texte-attenue">"Messagerie non disponible pour votre profil."</p> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| fils.refetch())/> }.into_any(),
                Some(Ok(list)) => {
                    let (recents, non_lus) = fils_recents(list, 3);
                    view! {
                        <p class="accueil-chiffre">
                            <span class="accueil-chiffre__valeur">{non_lus}</span>
                            <span class="accueil-chiffre__libelle">{libelle_non_lus(non_lus)}</span>
                        </p>
                        {if recents.is_empty() {
                            view! { <p class="texte-attenue">"Aucune conversation."</p> }.into_any()
                        } else {
                            view! {
                                <ul class="ui-liste">
                                    {recents
                                        .into_iter()
                                        .map(|f| view! {
                                            <li class="ui-liste__element">
                                                <span class="ui-liste__principal">
                                                    <a class="ui-liste__titre" href=format!("{}/{}", navigation::MESSAGES, f.id)>{f.sujet.clone()}</a>
                                                    <span class="ui-liste__detail">{f.eleve_label.clone().unwrap_or_else(|| "Conversation".into())}</span>
                                                </span>
                                                {(f.nb_non_lus > 0).then(|| view! { <Badge libelle=format!("{} non lu(s)", f.nb_non_lus) ton=TonEtat::Info/> })}
                                            </li>
                                        })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }}
                    }
                        .into_any()
                }
            }}
        </Tuile>
    }
}

#[component]
fn TuileAnnonces(annonces: LocalResource<Result<Vec<Annonce>, ApiError>>) -> impl IntoView {
    view! {
        <Tuile id="annonces" titre="Annonces de l'établissement" largeur=Largeur::Six accent=true>
            {move || match annonces.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des annonces…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_))) => view! { <p>"Annonces non disponibles pour votre profil."</p> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| annonces.refetch())/> }.into_any(),
                Some(Ok(mut list)) => {
                    list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
                    if list.is_empty() {
                        view! { <p>"Aucune annonce pour le moment."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="ui-liste">
                                {list
                                    .into_iter()
                                    .take(2)
                                    .map(|a| view! {
                                        <li class="ui-liste__element accueil-annonce">
                                            <span class="ui-liste__principal">
                                                <span class="ui-liste__titre">{a.titre.clone()}</span>
                                                <span class="accueil-annonce__corps">{tronquer(&a.corps, 160)}</span>
                                            </span>
                                        </li>
                                    })
                                    .collect_view()}
                            </ul>
                        }
                            .into_any()
                    }
                }
            }}
        </Tuile>
    }
}

/// Tuile du personnel de vie scolaire : absences du jour (issues des appels clos et des
/// déclarations des familles), justificatifs à trancher.
/// Direction : marques « ne pas solliciter » et adresses protégées à traiter (démo P1 n° 4).
#[component]
fn TuileProtections() -> impl IntoView {
    let alertes = LocalResource::new(|| async { session::client().alertes_protection().await });
    let action =
        ViewFn::from(|| view! { <a href=navigation::PROTECTIONS>"Traiter les protections"</a> });
    view! {
        <Tuile id="protections" titre="Protections à confirmer" largeur=Largeur::Six action=action>
            {move || match alertes.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des protections…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_))) => view! { <p class="texte-attenue">"Protections non disponibles pour votre profil."</p> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| alertes.refetch())/> }.into_any(),
                Some(Ok(a)) => {
                    let nps = a.ne_pas_solliciter_a_confirmer;
                    let adresses = a.adresses_a_traiter.len();
                    view! {
                        <p class="accueil-chiffre">
                            <span class="accueil-chiffre__valeur">{nps.len()}</span>
                            <span class="accueil-chiffre__libelle">{if nps.len() > 1 { "marques « ne pas solliciter » à confirmer" } else { "marque « ne pas solliciter » à confirmer" }}</span>
                        </p>
                        <p class="texte-attenue">{if adresses == 0 { "Aucune adresse protégée à traiter.".to_string() } else { format!("{adresses} adresse(s) protégée(s) à traiter.") }}</p>
                        <ul class="ui-liste">
                            {nps.into_iter().take(3).map(|p| view! {
                                <li class="ui-liste__element"><span class="ui-liste__principal">
                                    <span class="ui-liste__titre">{format!("{} {}", p.prenom.unwrap_or_default(), p.nom).trim().to_string()}</span>
                                </span></li>
                            }).collect_view()}
                        </ul>
                    }.into_any()
                }
            }}
        </Tuile>
    }
}

#[component]
fn TuileVieScolaire() -> impl IntoView {
    let jour = aujourdhui();
    let absences =
        LocalResource::new(move || async move { session::client().absences_a_traiter(jour).await });
    let action =
        ViewFn::from(|| view! { <a href=navigation::VIE_SCOLAIRE>"Traiter les absences"</a> });
    view! {
        <Tuile id="vie-scolaire" titre="Absences du jour" largeur=Largeur::Six action=action>
            {move || match absences.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des absences…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_))) => view! { <p class="texte-attenue">"Absences non disponibles pour votre profil."</p> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| absences.refetch())/> }.into_any(),
                Some(Ok(mut l)) => {
                    let absents = eleves_absents(&l);
                    let attente = Compteurs::de(&l).en_attente;
                    // Les justificatifs à trancher d'abord.
                    l.sort_by_key(|a| StatutAbsence::depuis(a) != StatutAbsence::JustificatifEnAttente);
                    view! {
                        <p class="accueil-chiffre">
                            <span class="accueil-chiffre__valeur">{absents}</span>
                            <span class="accueil-chiffre__libelle">{libelle_absents(absents)}</span>
                        </p>
                        <p class="texte-attenue">{libelle_en_attente(attente)}</p>
                        {if l.is_empty() {
                            view! { <p class="texte-attenue">"Les absences relevées par les appels clos apparaissent ici."</p> }.into_any()
                        } else {
                            view! {
                                <ul class="ui-liste">
                                    {l
                                        .into_iter()
                                        .take(3)
                                        .map(|a| {
                                            let s = StatutAbsence::depuis(&a);
                                            view! {
                                                <li class="ui-liste__element">
                                                    <span class="ui-liste__principal">
                                                        <span class="ui-liste__titre">{nom_eleve(&a)}</span>
                                                        <span class="ui-liste__detail">{a.motif.clone()}</span>
                                                    </span>
                                                    <Badge libelle=s.libelle() ton=s.ton()/>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }}
                    }
                        .into_any()
                }
            }}
        </Tuile>
    }
}

/// Tuile famille : absences encore à justifier, enfant par enfant (seulement ceux dont le
/// lien autorise la justification).
#[component]
fn TuileAbsencesFamille() -> impl IntoView {
    let a_justifier = LocalResource::new(|| async {
        let client = session::client();
        let enfants = client.enfants_famille().await?;
        let mut v: Vec<(String, usize)> = Vec::new();
        let mut aucun_droit = true;
        for e in enfants.iter().filter(|e| e.peut_justifier_absence) {
            aucun_droit = false;
            match client.resume_enfant(e.id).await {
                Ok(r) => v.push((e.prenom.clone(), r.absences_a_justifier.len())),
                Err(ApiError::NonConnecte) => return Err(ApiError::NonConnecte),
                Err(_) => {}
            }
        }
        Ok::<_, ApiError>((v, aucun_droit))
    });
    let action = ViewFn::from(|| view! { <a href=navigation::ABSENCES>"Voir les absences"</a> });
    view! {
        <Tuile id="absences-famille" titre="Absences à justifier" largeur=Largeur::Six action=action>
            {move || match a_justifier.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement des absences…"/> }.into_any(),
                Some(Err(ApiError::Interdit(_))) => view! { <p class="texte-attenue">"Absences non disponibles pour votre profil."</p> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| a_justifier.refetch())/> }.into_any(),
                Some(Ok((_, true))) => view! {
                    <p class="texte-attenue">"Votre lien ne vous permet pas de justifier d'absence ; vous pouvez les consulter."</p>
                }
                    .into_any(),
                Some(Ok((v, false))) => {
                    let total: usize = v.iter().map(|(_, n)| n).sum();
                    view! {
                        <p class="accueil-chiffre">
                            <span class="accueil-chiffre__valeur">{total}</span>
                            <span class="accueil-chiffre__libelle">{libelle_a_justifier(total)}</span>
                        </p>
                        <ul class="ui-liste">
                            {v
                                .into_iter()
                                .filter(|(_, n)| *n > 0)
                                .map(|(prenom, n)| view! {
                                    <li class="ui-liste__element">
                                        <span class="ui-liste__principal">
                                            <span class="ui-liste__titre">{prenom}</span>
                                            <span class="ui-liste__detail">{libelle_a_justifier(n)}</span>
                                        </span>
                                    </li>
                                })
                                .collect_view()}
                        </ul>
                    }
                        .into_any()
                }
            }}
        </Tuile>
    }
}
