//! Sorties et voyages scolaires (socle 42, lot V0a) : liste et frise par mois, création, détail
//! (prochaine étape, commandes d'état, encadrants, participants choisis dans le registre, journal,
//! circulaire) et vue des familles.
//!
//! L'écran n'ajoute **aucune règle** : les boutons d'état viennent du serveur
//! (`SortieDetailDto::commandes`), qui revérifie le droit, l'étape et la version à chaque appel ;
//! un refus (403/409/422) est affiché tel quel. Les familles ne voient que leur enfant.

mod budget;
mod edition;
pub mod modele;
mod modele_edition;
mod modele_programme;
mod modele_serie;
mod programme;
mod serie;

pub use serie::PageNouvelleSerie;

use ent_types::voyages::{
    CandidatsClasseDto, CommandeDto, ParamsCommandeDto, SortieDetailDto, SortieResumeDto,
    VoyageFamilleDto,
};
use ent_web_api::ApiError;
use ent_web_ui::{
    AccesRefuse, Badge, Bouton, Case, ChampTexte, EnTetePage, EtatVide, LienBouton, ResumeErreurs,
    Selection, Squelette, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_params_map};
use uuid::Uuid;

use crate::noyau::coquille::{use_droits, use_droits_moteur};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_heure_paris};
use modele::{
    composer, grouper_par_mois, libelle_destination, libelle_etat, libelle_etat_participant,
    libelle_gravite, libelle_journal, libelle_qui, libelle_role_encadrant, periode, ton_etat,
    ton_gravite, SaisieVoyage, DESTINATIONS, ETATS_FILTRE, FORMES, MOTIFS_SORTIE, NATURES,
    ROLES_ENCADRANT,
};

fn adresse(id: Uuid) -> String {
    format!("{}/{id}", navigation::VOYAGES)
}

// ───────────────────────── Liste ─────────────────────────

/// Écran « Sorties et voyages » : liste pour le personnel, projection famille sinon.
#[component]
pub fn PageVoyages() -> impl IntoView {
    let droits = use_droits();
    let ctx = session::use_session();
    view! {
        <Title text="Sorties et voyages"/>
        <EnTetePage
            surtitre="Vie de l'établissement".to_string()
            titre="Sorties et voyages".to_string()
            sous_titre="Projets, validations, élèves et encadrants d'un même endroit.".to_string()
        />
        {move || match ctx.moi.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des sorties…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| ctx.moi.refetch())/> }.into_any(),
            Some(Ok(_)) if droits.get().personnel => view! { <VuePersonnel/> }.into_any(),
            Some(Ok(_)) => view! { <VueFamille/> }.into_any(),
        }}
    }
}

#[component]
fn VuePersonnel() -> impl IntoView {
    let moteur = use_droits_moteur();
    let q = RwSignal::new(String::new());
    let etat = RwSignal::new(String::new());
    let mes = RwSignal::new(false);
    let a_venir = RwSignal::new(true);
    let liste = LocalResource::new(move || {
        let (q, etat, mes, a_venir) = (q.get(), etat.get(), mes.get(), a_venir.get());
        async move {
            session::client()
                .voyages(&q, Some(etat.as_str()), mes, a_venir)
                .await
        }
    });
    let options_etat: Vec<(String, String)> = ETATS_FILTRE
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    view! {
        <div class="voy-barre">
            {move || moteur.get().a("voyages.creer").then(|| view! {
                <LienBouton href=format!("{}/nouveau", navigation::VOYAGES)>"Nouvelle sortie"</LienBouton>
                <LienBouton href=format!("{}/serie", navigation::VOYAGES) variante=Variante::Secondaire>"Nouvelle série hebdomadaire"</LienBouton>
            })}
        </div>
        <Tableau/>
        <form class="voy-filtres" role="search" on:submit=move |ev| ev.prevent_default()>
            <ChampTexte id="voy-recherche" libelle="Rechercher" genre="search" valeur=q aide="Titre, lieu ou porteur."/>
            <Selection id="voy-etat" libelle="État" valeur=etat options=options_etat/>
            <Case id="voy-mes" libelle="Mes sorties seulement" coche=mes/>
            <Case id="voy-avenir" libelle="À venir seulement" coche=a_venir/>
        </form>
        {move || match liste.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des sorties…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune sortie" message="Aucune sortie ne correspond à ces critères."/>
            }
            .into_any(),
            Some(Ok(l)) => view! { <Frise liste=l/> }.into_any(),
        }}
    }
}

/// Chiffres de la direction et de la comptabilité ; rien n'est affiché sans droit (403 → rien).
#[component]
fn Tableau() -> impl IntoView {
    let moteur = use_droits_moteur();
    let t = LocalResource::new(|| async { session::client().voyages_tableau().await });
    view! {
        {move || {
            let m = moteur.get();
            let autorise = m.a("voyages.valider") || m.a("voyages.autoriser")
                || m.a("voyages.budget_valider") || m.a("voyages.lire");
            if !autorise {
                return ().into_any();
            }
            match t.get() {
                Some(Ok(t)) => view! {
                    <section class="voy-tableau" aria-label="À traiter">
                        <p><strong>{t.a_valider}</strong>" à valider"</p>
                        <p><strong>{t.budget_a_valider}</strong>" budgets à valider"</p>
                        <p><strong>{t.a_autoriser}</strong>" à autoriser"</p>
                        <p><strong>{t.ouvertes}</strong>" ouvertes"</p>
                        <p><strong>{t.pretes}</strong>" prêtes"</p>
                        {(!t.bloquees_proches.is_empty()).then(|| view! {
                            <p class="voy-tableau__alerte" role="status">
                                {format!("{} dossier(s) pas prêt(s) dans les 14 jours.", t.bloquees_proches.len())}
                            </p>
                        })}
                    </section>
                }
                .into_any(),
                _ => ().into_any(),
            }
        }}
    }
}

#[component]
fn Frise(liste: Vec<SortieResumeDto>) -> impl IntoView {
    let groupes: Vec<(String, Vec<SortieResumeDto>)> = grouper_par_mois(&liste)
        .into_iter()
        .map(|(m, v)| (m, v.into_iter().cloned().collect()))
        .collect();
    view! {
        {groupes
            .into_iter()
            .map(|(mois, sorties)| {
                let etiquette = mois.clone();
                view! {
                    <section class="voy-mois" aria-label=etiquette>
                        <h2 class="vs-section-titre">{mois}</h2>
                        <ul class="voy-liste">
                            {sorties.into_iter().map(|s| view! { <CarteSortie s=s/> }).collect_view()}
                        </ul>
                    </section>
                }
            })
            .collect_view()}
    }
}

#[component]
fn CarteSortie(s: SortieResumeDto) -> impl IntoView {
    let href = adresse(s.id);
    let lieu = s
        .destination_libelle
        .clone()
        .unwrap_or_else(|| libelle_destination(&s.destination).to_string());
    view! {
        <li class="voy-carte">
            <div class="voy-carte__tete">
                <a href=href class="voy-carte__titre">{s.titre.clone()}</a>
                <Badge libelle=libelle_etat(&s.etat) ton=ton_etat(&s.etat)/>
            </div>
            <p class="voy-carte__detail">{periode(s.debut, s.fin)}</p>
            <p class="voy-carte__detail">{lieu}</p>
            {s.porteur_nom.clone().map(|p| view! { <p class="voy-carte__detail">{format!("Porteur : {p}")}</p> })}
            {s.nb_inscrits.map(|n| view! { <p class="voy-carte__detail">{format!("{n} élève(s) inscrit(s)")}</p> })}
            {s.est_porteur.then(|| view! { <Badge libelle="Vous la portez" ton=TonEtat::Info/> })}
        </li>
    }
}

// ───────────────────────── Famille ─────────────────────────

#[component]
fn VueFamille() -> impl IntoView {
    let l = LocalResource::new(|| async { session::client().voyages_famille().await });
    view! {
        {move || match l.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des sorties…"/> }.into_any(),
            Some(Err(ApiError::Interdit(_))) => view! { <AccesRefuse message="Cet écran est réservé aux familles et au personnel."/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| l.refetch())/> }.into_any(),
            Some(Ok(v)) if v.is_empty() => view! {
                <EtatVide titre="Aucune sortie" message="Aucune sortie ouverte n'est prévue pour votre enfant."/>
            }
            .into_any(),
            Some(Ok(v)) => view! { <ul class="voy-liste">{v.into_iter().map(|f| view! { <CarteFamille f=f/> }).collect_view()}</ul> }.into_any(),
        }}
    }
}

#[component]
fn CarteFamille(f: VoyageFamilleDto) -> impl IntoView {
    view! {
        <li class="voy-carte">
            <div class="voy-carte__tete">
                <strong class="voy-carte__titre">{f.titre.clone()}</strong>
                <Badge libelle=libelle_etat(&f.etat) ton=ton_etat(&f.etat)/>
            </div>
            <p class="voy-carte__detail">{format!("Pour {}", f.enfant_prenom)}</p>
            <p class="voy-carte__detail">{periode(f.debut, f.fin)}</p>
            <p class="voy-carte__detail">
                {f.destination_libelle.clone().unwrap_or_else(|| libelle_destination(&f.destination).to_string())}
            </p>
            {f.point_rendez_vous.clone().map(|r| view! { <p class="voy-carte__detail">{format!("Rendez-vous : {r}")}</p> })}
        </li>
    }
}

// ───────────────────────── Création ─────────────────────────

#[component]
pub fn PageNouveauVoyage() -> impl IntoView {
    let navigate = use_navigate();
    let classes = LocalResource::new(|| async { session::client().classes(aujourdhui()).await });
    let titre = RwSignal::new(String::new());
    let nature = RwSignal::new("facultative".to_string());
    let forme = RwSignal::new("sans_nuitee".to_string());
    let destination = RwSignal::new("proximite".to_string());
    let pays = RwSignal::new(String::new());
    let lieu = RwSignal::new(String::new());
    let objectifs = RwSignal::new(String::new());
    let date_debut = RwSignal::new(String::new());
    let heure_debut = RwSignal::new("08:00".to_string());
    let date_fin = RwSignal::new(String::new());
    let heure_fin = RwSignal::new("17:00".to_string());
    let effectif = RwSignal::new(String::new());
    let adultes = RwSignal::new(String::new());
    let rdv = RwSignal::new(String::new());
    let car = RwSignal::new(false);
    let choisies = RwSignal::new(Vec::<Uuid>::new());
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let erreur_de = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs
                .get()
                .into_iter()
                .find(|(c, _)| c == champ)
                .map(|(_, m)| m)
        })
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieVoyage {
            titre: titre.get_untracked(),
            nature: nature.get_untracked(),
            forme: forme.get_untracked(),
            destination: destination.get_untracked(),
            pays: pays.get_untracked(),
            lieu: lieu.get_untracked(),
            objectifs: objectifs.get_untracked(),
            date_debut: date_debut.get_untracked(),
            heure_debut: heure_debut.get_untracked(),
            date_fin: date_fin.get_untracked(),
            heure_fin: heure_fin.get_untracked(),
            effectif: effectif.get_untracked(),
            adultes: adultes.get_untracked(),
            point_rendez_vous: rdv.get_untracked(),
            besoin_car: car.get_untracked(),
            classes: choisies.get_untracked(),
        };
        match composer(&saisie) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                let navigate = navigate.clone();
                ecriture.lancer(async move {
                    let cree = session::client().voyage_creer(&corps).await?;
                    navigate(&adresse(cree.id), Default::default());
                    Ok(format!("Sortie « {} » créée en brouillon.", cree.titre))
                });
            }
        }
    };
    let options = |liste: &[(&str, &str)]| -> Vec<(String, String)> {
        liste
            .iter()
            .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
            .collect()
    };
    view! {
        <Title text="Nouvelle sortie"/>
        <EnTetePage
            surtitre="Sorties et voyages".to_string()
            titre="Nouvelle sortie".to_string()
            sous_titre="Vous en serez le porteur. Rien n'est envoyé aux familles à cette étape.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        <form class="voy-formulaire" on:submit=envoyer>
            <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
            <ChampTexte id="voy-titre" libelle="Titre" valeur=titre requis=true erreur=erreur_de("voy-titre")/>
            <Selection id="voy-nature" libelle="Nature" valeur=nature options=options(&NATURES) requis=true erreur=erreur_de("voy-nature")/>
            <Selection id="voy-forme" libelle="Forme" valeur=forme options=options(&FORMES) requis=true erreur=erreur_de("voy-forme")/>
            <Selection id="voy-destination" libelle="Destination" valeur=destination options=options(&DESTINATIONS) requis=true erreur=erreur_de("voy-destination")/>
            {move || (destination.get() == "etranger").then(|| view! {
                <ChampTexte id="voy-pays" libelle="Pays" valeur=pays requis=true erreur=erreur_de("voy-pays")/>
            })}
            <ChampTexte id="voy-lieu" libelle="Lieu" valeur=lieu aide="Nom du lieu ou de la ville."/>
            <ChampTexte id="voy-date-debut" libelle="Date de départ" genre="date" valeur=date_debut requis=true erreur=erreur_de("voy-date-debut")/>
            <ChampTexte id="voy-heure-debut" libelle="Heure de départ" genre="time" valeur=heure_debut requis=true erreur=erreur_de("voy-heure-debut")/>
            <ChampTexte id="voy-date-fin" libelle="Date de retour" genre="date" valeur=date_fin requis=true erreur=erreur_de("voy-date-fin")/>
            <ChampTexte id="voy-heure-fin" libelle="Heure de retour" genre="time" valeur=heure_fin requis=true erreur=erreur_de("voy-heure-fin")/>
            <ChampTexte id="voy-effectif" libelle="Nombre d'élèves prévu" genre="number" valeur=effectif erreur=erreur_de("voy-effectif")/>
            <ChampTexte id="voy-adultes" libelle="Nombre d'adultes prévu" genre="number" valeur=adultes erreur=erreur_de("voy-adultes")/>
            <ChampTexte id="voy-rdv" libelle="Point de rendez-vous" valeur=rdv/>
            <Case id="voy-car" libelle="Un car est nécessaire" coche=car/>
            <ZoneTexte id="voy-objectifs" libelle="Objectifs pédagogiques" valeur=objectifs/>
            <fieldset id="voy-classes" class="voy-classes">
                <legend>"Classes concernées"</legend>
                {move || match classes.get() {
                    None => view! { <Squelette lignes=2 libelle="Chargement des classes…"/> }.into_any(),
                    Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| classes.refetch())/> }.into_any(),
                    Some(Ok(l)) => l
                        .into_iter()
                        .map(|c| {
                            let id = c.id;
                            let coche = RwSignal::new(false);
                            Effect::new(move |_| {
                                let c = coche.get();
                                choisies.update(|v| {
                                    v.retain(|x| *x != id);
                                    if c {
                                        v.push(id);
                                    }
                                });
                            });
                            view! { <Case id=format!("voy-classe-{id}") libelle=c.libelle.clone() coche=coche/> }
                        })
                        .collect_view()
                        .into_any(),
                }}
                {move || erreur_de("voy-classes").get().map(|m| view! { <p class="ui-champ__erreur">{m}</p> })}
            </fieldset>
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Créer le brouillon"</Bouton>
        </form>
    }
}

// ───────────────────────── Détail ─────────────────────────

#[component]
pub fn PageVoyage() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .read()
            .get("id")
            .and_then(|s| Uuid::parse_str(&s).ok())
    };
    let detail = LocalResource::new(move || async move {
        match id() {
            Some(i) => session::client().voyage(i).await,
            None => Err(ApiError::Introuvable("identifiant invalide".into())),
        }
    });
    view! {
        <Title text="Sortie"/>
        {move || match detail.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement de la sortie…"/> }.into_any(),
            Some(Err(ApiError::Introuvable(_))) => view! {
                <EtatVide titre="Sortie introuvable" message="Elle n'existe pas, ou vous n'y avez pas accès.">
                    <LienBouton href=navigation::VOYAGES.to_string() variante=Variante::Secondaire>"Retour à la liste"</LienBouton>
                </EtatVide>
            }
            .into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| detail.refetch())/> }.into_any(),
            Some(Ok(d)) => view! { <Fiche d=d recharger=Callback::new(move |_| detail.refetch())/> }.into_any(),
        }}
    }
}

#[component]
fn Fiche(d: SortieDetailDto, recharger: Callback<()>) -> impl IntoView {
    let s = d.sortie.clone();
    let id = s.id;
    let ecriture = Ecriture::nouveau(recharger);
    let titre = s.titre.clone();
    let publics: Vec<String> = d.publics.iter().map(|p| p.libelle.clone()).collect();
    view! {
        <EnTetePage
            surtitre=format!("Sortie · {}", s.reference)
            titre=titre
            sous_titre=format!("{} · {}", periode(s.debut, s.fin), s.destination_libelle.clone().unwrap_or_else(|| libelle_destination(&s.destination).to_string()))
        />
        <p class="voy-etat">
            <Badge libelle=libelle_etat(&s.etat) ton=ton_etat(&s.etat)/>
            {format!(" Porteur : {}", d.porteur_nom)}
        </p>
        <RetourEcriture ecriture=ecriture/>
        <section class="voy-etape" aria-labelledby="voy-prochaine">
            <h2 id="voy-prochaine" class="vs-section-titre">"Prochaine étape"</h2>
            <p><strong>{libelle_qui(&d.prochaine_etape.qui)}</strong></p>
            <p>{d.prochaine_etape.libelle.clone()}</p>
            {s.motif_decision.clone().map(|m| view! { <p class="voy-note">{format!("Note de la direction : {m}")}</p> })}
        </section>
        <Commandes id=id version=s.version commandes=d.commandes.clone() ecriture=ecriture/>
        <section aria-labelledby="voy-audit">
            <h2 id="voy-audit" class="vs-section-titre">"Points d'attention"</h2>
            {if d.audit.is_empty() {
                view! { <p class="texte-attenue">"Rien à signaler."</p> }.into_any()
            } else {
                view! {
                    <ul class="voy-audit">
                        {d.audit.iter().map(|p| view! {
                            <li><Badge libelle=libelle_gravite(&p.gravite) ton=ton_gravite(&p.gravite)/>{format!(" {}", p.libelle)}</li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>
        <section aria-labelledby="voy-infos">
            <h2 id="voy-infos" class="vs-section-titre">"Le projet"</h2>
            <dl class="voy-infos">
                <dt>"Public"</dt><dd>{if publics.is_empty() { "Aucune classe".to_string() } else { publics.join(", ") }}</dd>
                <dt>"Effectif prévu"</dt><dd>{match s.effectif_eleves_declare { Some(n) => format!("{n} élève(s), {} adulte(s)", s.effectif_adultes_declare.unwrap_or(0)), None => "Non précisé".into() }}</dd>
                <dt>"Inscrits"</dt><dd>{format!("{} (désistés ou retirés : {})", d.nb_inscrits, d.nb_desistes)}</dd>
                <dt>"Rendez-vous"</dt><dd>{s.point_rendez_vous.clone().unwrap_or_else(|| "Non précisé".into())}</dd>
                <dt>"Car"</dt><dd>{if s.besoin_car { "Nécessaire" } else { "Non" }}</dd>
                {s.objectifs.clone().map(|o| view! { <dt>"Objectifs"</dt><dd>{o}</dd> })}
            </dl>
        </section>
        {d.acces.modifier.then(|| view! { <edition::Edition d=d.clone() recharger=recharger/> })}
        <programme::Programme d=d.clone() recharger=recharger/>
        <Encadrants d=d.clone() ecriture=ecriture/>
        <Participants d=d.clone() ecriture=ecriture/>
        <budget::Budget id=id etat=s.etat.clone() version=s.version recharger=recharger/>
        <Circulaire d=d.clone()/>
        <Journal id=id/>
    }
}

/// Boutons d'état proposés par le serveur ; un motif (et une étape pour une réouverture) se demande
/// dans un petit formulaire.
#[component]
fn Commandes(
    id: Uuid,
    version: i32,
    commandes: Vec<CommandeDto>,
    ecriture: Ecriture,
) -> impl IntoView {
    // « Valider le budget » porte sur la version de budget lue : elle se lance depuis la section Budget.
    let commandes: Vec<CommandeDto> = commandes
        .into_iter()
        .filter(|c| c.code != "valider-budget")
        .collect();
    if commandes.is_empty() {
        return ().into_any();
    }
    let choisie = RwSignal::new(None::<CommandeDto>);
    let motif = RwSignal::new(String::new());
    let vers = RwSignal::new(String::new());
    let lancer = move |c: CommandeDto, m: Option<String>, v: Option<String>| {
        ecriture.lancer(async move {
            let params = ParamsCommandeDto {
                version: Some(version),
                motif: m,
                vers: v,
                ..Default::default()
            };
            session::client()
                .voyage_commande(id, &c.code, &params)
                .await?;
            Ok(format!("« {} » : fait.", c.libelle))
        });
    };
    let valider_motif = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(c) = choisie.get_untracked() {
            let m = motif.get_untracked();
            let v = vers.get_untracked();
            lancer(c, Some(m), (!v.is_empty()).then_some(v));
        }
    };
    view! {
        <section aria-labelledby="voy-commandes">
            <h2 id="voy-commandes" class="vs-section-titre">"Actions"</h2>
            <div class="voy-actions">
                {commandes
                    .into_iter()
                    .map(|c| {
                        let danger = c.danger;
                        let libelle = c.libelle.clone();
                        let c2 = c.clone();
                        view! {
                            <Bouton
                                variante=if danger { Variante::Danger } else { Variante::Secondaire }
                                en_cours=ecriture.en_cours
                                on_click=Callback::new(move |_| {
                                    if c2.motif_requis {
                                        motif.set(String::new());
                                        vers.set(c2.vers.first().cloned().unwrap_or_default());
                                        choisie.set(Some(c2.clone()));
                                    } else {
                                        lancer(c2.clone(), None, None);
                                    }
                                })
                            >
                                {libelle}
                            </Bouton>
                        }
                    })
                    .collect_view()}
            </div>
            {move || choisie.get().map(|c| {
                let options: Vec<(String, String)> = c.vers.iter().map(|v| (v.clone(), libelle_etat(v).to_string())).collect();
                view! {
                    <form class="voy-motif" on:submit=valider_motif>
                        <h3>{c.libelle.clone()}</h3>
                        {(!options.is_empty()).then(|| view! {
                            <Selection id="voy-vers" libelle="Revenir à l'étape" valeur=vers options=options.clone() requis=true/>
                        })}
                        <ZoneTexte id="voy-motif" libelle="Motif (obligatoire, conservé au journal)" valeur=motif requis=true/>
                        <Bouton soumettre=true en_cours=ecriture.en_cours>"Confirmer"</Bouton>
                        <Bouton variante=Variante::Discret on_click=Callback::new(move |_| choisie.set(None))>"Annuler"</Bouton>
                    </form>
                }
            })}
        </section>
    }
    .into_any()
}

#[component]
fn Encadrants(d: SortieDetailDto, ecriture: Ecriture) -> impl IntoView {
    let id = d.sortie.id;
    let peut = d.acces.encadrement;
    let agent = RwSignal::new(String::new());
    let role = RwSignal::new("encadrant".to_string());
    let agents = LocalResource::new(|| async { session::client().agents_pour_encadrement().await });
    let options_role: Vec<(String, String)> = ROLES_ENCADRANT
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let ajouter = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let (a, r) = (agent.get_untracked(), role.get_untracked());
        if let Ok(a) = Uuid::parse_str(&a) {
            ecriture.lancer(async move {
                session::client()
                    .voyage_ajouter_encadrant(id, a, &r)
                    .await?;
                Ok("Encadrant ajouté.".to_string())
            });
        }
    };
    view! {
        <section aria-labelledby="voy-encadrants">
            <h2 id="voy-encadrants" class="vs-section-titre">"Encadrants"</h2>
            {if d.encadrants.is_empty() {
                view! { <p class="texte-attenue">"Aucun encadrant choisi."</p> }.into_any()
            } else {
                view! {
                    <ul class="voy-liste voy-liste--simple">
                        {d.encadrants.iter().map(|e| {
                            let eid = e.id;
                            let nom = format!("{} {}", e.prenom.clone().unwrap_or_default(), e.nom).trim().to_string();
                            let nom2 = nom.clone();
                            view! {
                                <li class="voy-ligne">
                                    <span>{nom}" · "{libelle_role_encadrant(&e.role)}</span>
                                    {peut.then(|| view! {
                                        <Bouton
                                            variante=Variante::Discret
                                            libelle_accessible=format!("Retirer {nom2}")
                                            en_cours=ecriture.en_cours
                                            on_click=Callback::new(move |_| ecriture.lancer(async move {
                                                session::client().voyage_retirer_encadrant(id, eid).await?;
                                                Ok("Encadrant retiré.".to_string())
                                            }))
                                        >"Retirer"</Bouton>
                                    })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
            {peut.then(|| view! {
                <form class="voy-formulaire voy-formulaire--ligne" on:submit=ajouter>
                    {move || match agents.get() {
                        Some(Ok(l)) => {
                            let mut options = vec![(String::new(), "Choisir…".to_string())];
                            options.extend(l.into_iter().map(|a| (a.agent_id.to_string(), a.nom_affiche)));
                            view! { <Selection id="voy-agent" libelle="Adulte du personnel" valeur=agent options=options requis=true/> }.into_any()
                        }
                        Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| agents.refetch())/> }.into_any(),
                        None => view! { <Squelette lignes=1 libelle="Chargement du personnel…"/> }.into_any(),
                    }}
                    <Selection id="voy-role" libelle="Rôle" valeur=role options=options_role.clone()/>
                    <Bouton soumettre=true en_cours=ecriture.en_cours>"Ajouter"</Bouton>
                </form>
            })}
        </section>
    }
}

#[component]
fn Participants(d: SortieDetailDto, ecriture: Ecriture) -> impl IntoView {
    let id = d.sortie.id;
    let peut = d.acces.participants;
    let liste =
        LocalResource::new(move || async move { session::client().voyage_participants(id).await });
    let motif = RwSignal::new("autre".to_string());
    let export_erreur = RwSignal::new(None::<String>);
    let options_motif: Vec<(String, String)> = MOTIFS_SORTIE
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    // Recharger la liste après chaque écriture de la fiche : la fiche est recréée à chaque rechargement.
    view! {
        <section aria-labelledby="voy-participants">
            <h2 id="voy-participants" class="vs-section-titre">"Élèves"</h2>
            {peut.then(|| view! {
                <Selection id="voy-motif-sortie" libelle="Motif pour un désistement ou un retrait" valeur=motif options=options_motif.clone()/>
            })}
            {peut.then(|| view! {
                <div class="voy-actions">
                    <Bouton variante=Variante::Secondaire
                        on_click=Callback::new(move |_| {
                            leptos::task::spawn_local(async move {
                                match session::client().voyage_participants_csv(id).await {
                                    Ok(octets) => {
                                        if let Err(m) = crate::noyau::telechargement::enregistrer_telechargement("participants.csv", "text/csv", &octets) {
                                            export_erreur.set(Some(m));
                                        }
                                    }
                                    Err(e) => export_erreur.set(Some(e.message_utilisateur())),
                                }
                            });
                        })>"Exporter la liste (CSV)"</Bouton>
                </div>
                {move || export_erreur.get().map(|m| view! { <ent_web_ui::Alerte niveau=ent_web_ui::Niveau::Erreur titre=m/> })}
            })}
            {move || match liste.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement de la liste…"/> }.into_any(),
                Some(Err(ApiError::Introuvable(_))) => view! { <p class="texte-attenue">"La liste nominative n'est pas accessible avec vos droits."</p> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun élève inscrit" message="Choisissez les élèves dans le registre ci-dessous."/> }.into_any(),
                Some(Ok(l)) => view! {
                    <ul class="voy-liste voy-liste--simple">
                        {l.into_iter().map(|p| {
                            let pid = p.id;
                            let nom = format!("{} {}", p.prenom, p.nom);
                            let actif = !matches!(p.etat.as_str(), "desiste" | "retire");
                            let (n1, n2) = (nom.clone(), nom.clone());
                            view! {
                                <li class="voy-ligne">
                                    <span>{nom}{p.classe.clone().map(|c| format!(" · {c}"))}" · "{libelle_etat_participant(&p.etat)}</span>
                                    {(peut && actif).then(|| view! {
                                        <Bouton variante=Variante::Discret libelle_accessible=format!("Désistement de {n1}") en_cours=ecriture.en_cours
                                            on_click=Callback::new(move |_| {
                                                let m = motif.get_untracked();
                                                ecriture.lancer(async move {
                                                    session::client().voyage_sortir_participant(id, pid, false, &m, None).await?;
                                                    Ok("Désistement enregistré.".to_string())
                                                })
                                            })>"Désistement"</Bouton>
                                        <Bouton variante=Variante::Discret libelle_accessible=format!("Retirer {n2}") en_cours=ecriture.en_cours
                                            on_click=Callback::new(move |_| {
                                                let m = motif.get_untracked();
                                                ecriture.lancer(async move {
                                                    session::client().voyage_sortir_participant(id, pid, true, &m, None).await?;
                                                    Ok("Élève retiré de la liste.".to_string())
                                                })
                                            })>"Retirer"</Bouton>
                                    })}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
            }}
            {peut.then(|| view! { <Registre id=id ecriture=ecriture/> })}
        </section>
    }
}

/// Registre des élèves scolarisés à la date de la sortie, par classe visée (aucune saisie de nom).
#[component]
fn Registre(id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let ouvert = RwSignal::new(false);
    let candidats = LocalResource::new(move || async move {
        if ouvert.get() {
            session::client().voyage_candidats(id, None).await
        } else {
            Ok(Vec::<CandidatsClasseDto>::new())
        }
    });
    let choix = RwSignal::new(Vec::<Uuid>::new());
    let inscrire = move |_| {
        let eleves = choix.get_untracked();
        if eleves.is_empty() {
            return;
        }
        ecriture.lancer(async move {
            let r = session::client()
                .voyage_ajouter_participants(id, &eleves)
                .await?;
            Ok(match (r.ajoutes.len(), r.refuses.len()) {
                (n, 0) => format!("{n} élève(s) inscrit(s)."),
                (n, m) => {
                    format!("{n} élève(s) inscrit(s), {m} refusé(s) (non scolarisés à cette date).")
                }
            })
        });
        choix.set(Vec::new());
    };
    view! {
        <details class="voy-registre" on:toggle=move |_| ouvert.set(true)>
            <summary>"Choisir des élèves dans le registre"</summary>
            {move || match candidats.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement du registre…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| candidats.refetch())/> }.into_any(),
                Some(Ok(groupes)) => view! {
                    {groupes.into_iter().map(|g| view! {
                        <fieldset class="voy-classes">
                            <legend>{g.libelle.clone()}</legend>
                            {g.eleves.into_iter().map(|e| {
                                let eid = e.eleve_id;
                                let coche = RwSignal::new(false);
                                Effect::new(move |_| {
                                    let c = coche.get();
                                    choix.update(|v| {
                                        v.retain(|x| *x != eid);
                                        if c {
                                            v.push(eid);
                                        }
                                    });
                                });
                                let deja = e.deja_inscrit;
                                let libelle = format!("{} {}", e.prenom, e.nom);
                                view! { <Case id=format!("voy-eleve-{eid}") libelle=libelle coche=coche desactive=Signal::derive(move || deja)/> }
                            }).collect_view()}
                        </fieldset>
                    }).collect_view()}
                    <Bouton en_cours=ecriture.en_cours on_click=Callback::new(inscrire)>"Inscrire la sélection"</Bouton>
                }.into_any(),
            }}
        </details>
    }
}

/// Texte de la circulaire aux familles (VOY-170) : réservé au porteur et à ceux qui gèrent ; le serveur
/// refuse tout autre appelant, et avant l'ouverture de la sortie.
#[component]
fn Circulaire(d: SortieDetailDto) -> impl IntoView {
    if !d.acces.modifier || !matches!(d.sortie.etat.as_str(), "ouverte" | "prete") {
        return ().into_any();
    }
    let id = d.sortie.id;
    let texte = RwSignal::new(None::<Result<String, String>>);
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    view! {
        <section aria-labelledby="voy-circulaire">
            <h2 id="voy-circulaire" class="vs-section-titre">"Circulaire aux familles"</h2>
            <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| {
                ecriture.lancer(async move {
                    match session::client().voyage_circulaire(id).await {
                        Ok(c) => {
                            texte.set(Some(Ok(c.corps)));
                            Ok("Circulaire préparée : relisez-la avant tout envoi.".to_string())
                        }
                        Err(e) => Err(e),
                    }
                })
            })>"Préparer le texte"</Bouton>
            <RetourEcriture ecriture=ecriture/>
            {move || texte.get().and_then(Result::ok).map(|t| view! { <pre class="voy-circulaire">{t}</pre> })}
        </section>
    }
    .into_any()
}

#[component]
fn Journal(id: Uuid) -> impl IntoView {
    let j = LocalResource::new(move || async move { session::client().voyage_journal(id).await });
    view! {
        {move || match j.get() {
            Some(Ok(l)) if !l.is_empty() => view! {
                <section aria-labelledby="voy-journal">
                    <h2 id="voy-journal" class="vs-section-titre">"Journal"</h2>
                    <ol class="voy-journal">
                        {l.into_iter().rev().map(|e| view! {
                            <li>
                                <strong>{libelle_journal(&e.action)}</strong>
                                {format!(" · {}", date_heure_paris(e.a))}
                                {e.acteur_nom.clone().map(|n| format!(" · {n}"))}
                                {e.champ.clone().map(|c| format!(" · {c} : {} → {}", e.avant.clone().unwrap_or_default(), e.apres.clone().unwrap_or_default()))}
                                {e.motif.clone().map(|m| format!(" · {m}"))}
                            </li>
                        }).collect_view()}
                    </ol>
                </section>
            }.into_any(),
            _ => ().into_any(),
        }}
    }
}
