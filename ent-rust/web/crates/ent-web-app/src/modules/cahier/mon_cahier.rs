//! Écran du lot C3 : **le cahier de l'élève et de la famille**. Un seul écran, quatre onglets, lisible à 375 px :
//! l'agenda de la semaine (travaux, évaluations annoncées, sorties), « ce que j'ai manqué » (séances où l'élève était
//! absent), la semaine en un coup d'œil, et l'abonnement au calendrier personnel.
//!
//! Route `/web/cahier/agenda?onglet=…&semaine=AAAA-MM-JJ`. L'élève coche « fait » (outil personnel, jamais montré à la
//! famille) ; le responsable lit l'agenda de chaque enfant lié, sans coche, avec les statuts constatés. Droits, public du
//! cours, plage calme et statuts sont décidés par le serveur ; l'écran affiche ce que l'API renvoie et ses refus.

use ent_web_api::cahier_c3_types::{AbonnementCreeDto, AgendaLigneDto, GENRE_TRAVAIL};
use ent_web_api::Enfant;
use ent_web_ui::{
    AccesRefuse, Alerte, Annonce, Badge, Bouton, Case, EnTetePage, EtatVide, Icone, LienBouton,
    Niveau, NomIcone, OptionSegment, Segmente, Selection, Squelette, Taille, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use super::gestionnaire_echec;
use super::modele_c3::{
    fin_de_semaine, grouper_par_jour, libelle_duree, libelle_genre, libelle_semaine,
    libelle_statut, periode_manque, semaine_demandee, semaine_voisine, synthese, ton_genre, Onglet,
};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue, heure_courte};

/// Adresse de l'écran.
pub const ADRESSE_MON_CAHIER: &str = "/web/cahier/agenda";

#[component]
pub fn PageMonCahier() -> impl IntoView {
    let droits = use_droits();
    view! {
        <Title text="Mon cahier"/>
        {move || {
            let d = droits.get();
            if d.agenda && d.portail_famille {
                view! { <VueFamille/> }.into_any()
            } else if d.agenda {
                view! { <ContenuCahier eleve=None peut_cocher=true/> }.into_any()
            } else {
                view! { <AccesRefuse message="Cet écran est réservé aux élèves et aux familles."/> }.into_any()
            }
        }}
    }
}

fn nom_complet(e: &Enfant) -> String {
    format!("{} {}", e.prenom.trim(), e.nom.trim())
        .trim()
        .to_string()
}

#[component]
fn VueFamille() -> impl IntoView {
    let enfants = LocalResource::new(|| async { session::client().enfants_famille().await });
    view! {
        {move || match enfants.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de vos enfants…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| enfants.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucun enfant rattaché" message="Aucun enfant n'est lié à votre compte. Contactez le secrétariat de l'établissement."/>
            }.into_any(),
            Some(Ok(l)) => view! { <ChoixEnfant enfants=l/> }.into_any(),
        }}
    }
}

#[component]
fn ChoixEnfant(enfants: Vec<Enfant>) -> impl IntoView {
    let premier = enfants
        .first()
        .map(|e| e.id.to_string())
        .unwrap_or_default();
    let choix = RwSignal::new(premier);
    let options: Vec<(String, String)> = enfants
        .iter()
        .map(|e| (e.id.to_string(), nom_complet(e)))
        .collect();
    let selecteur = (enfants.len() > 1).then(|| {
        if enfants.len() <= 3 {
            let o = options
                .iter()
                .map(|(v, l)| OptionSegment::new(v.clone(), l.clone()))
                .collect();
            view! { <Segmente nom="cdt-enfant" legende="Enfant" options=o valeur=choix/> }
                .into_any()
        } else {
            view! { <Selection id="cdt-enfant" libelle="Enfant" valeur=choix options=options/> }
                .into_any()
        }
    });
    view! {
        {selecteur.map(|s| view! { <div class="abs-enfants">{s}</div> })}
        {move || {
            let id = choix.get().parse::<Uuid>().ok();
            // Nouvelle instance par enfant : rien ne se mélange d'un enfant à l'autre.
            id.map(|id| view! { <ContenuCahier eleve=Some(id) peut_cocher=false/> })
        }}
    }
}

fn adresse(onglet: Onglet, lundi: chrono::NaiveDate) -> String {
    format!(
        "{ADRESSE_MON_CAHIER}?onglet={}&semaine={lundi}",
        onglet.code()
    )
}

#[component]
fn ContenuCahier(eleve: Option<Uuid>, peut_cocher: bool) -> impl IntoView {
    let query = use_query_map();
    let onglet = Memo::new(move |_| Onglet::depuis(query.with(|q| q.get("onglet")).as_deref()));
    let lundi =
        Memo::new(move |_| semaine_demandee(query.with(|q| q.get("semaine")), aujourdhui()));
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre=Signal::derive(move || libelle_semaine(lundi.get()))
        >
            <nav class="edt-semaines" aria-label="Changer de semaine">
                <a class="ui-btn ui-btn--secondaire ui-btn--compact"
                   href=move || adresse(onglet.get(), semaine_voisine(lundi.get(), -1))
                   aria-label="Semaine précédente">
                    <Icone nom=NomIcone::ChevronGauche petite=true/>
                </a>
                <LienBouton href=ADRESSE_MON_CAHIER variante=Variante::Secondaire taille=Taille::Compacte>
                    "Cette semaine"
                </LienBouton>
                <a class="ui-btn ui-btn--secondaire ui-btn--compact"
                   href=move || adresse(onglet.get(), semaine_voisine(lundi.get(), 1))
                   aria-label="Semaine suivante">
                    <Icone nom=NomIcone::ChevronDroite petite=true/>
                </a>
            </nav>
        </EnTetePage>
        <nav class="cdt-onglets" aria-label="Rubriques du cahier">
            {Onglet::TOUS
                .into_iter()
                .map(|o| {
                    view! {
                        <a class="ui-btn ui-btn--compact"
                           class:ui-btn--secondaire=move || onglet.get() != o
                           href=move || adresse(o, lundi.get())
                           aria-current=move || (onglet.get() == o).then_some("page")>
                            {o.libelle()}
                        </a>
                    }
                })
                .collect_view()}
        </nav>
        {move || match onglet.get() {
            Onglet::Agenda => view! { <VueAgenda eleve=eleve lundi=lundi.get() peut_cocher=peut_cocher/> }.into_any(),
            Onglet::Manque => view! { <VueManque eleve=eleve/> }.into_any(),
            Onglet::Semaine => view! { <VueSemaine eleve=eleve lundi=lundi.get()/> }.into_any(),
            Onglet::Calendrier => view! { <VueCalendrier eleve=eleve/> }.into_any(),
        }}
    }
}

// ───────────────────────── Agenda ─────────────────────────

#[component]
fn VueAgenda(eleve: Option<Uuid>, lundi: chrono::NaiveDate, peut_cocher: bool) -> impl IntoView {
    let agenda = LocalResource::new(move || async move {
        session::client()
            .cdt_agenda_eleve(eleve, lundi, fin_de_semaine(lundi))
            .await
    });
    view! {
        {move || match agenda.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement de l'agenda…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| agenda.refetch())/> }.into_any(),
            Some(Ok(a)) if a.lignes.is_empty() => view! {
                <EtatVide titre="Rien cette semaine" message="Aucun travail, évaluation ni sortie n'est annoncé pour cette semaine."/>
            }.into_any(),
            Some(Ok(a)) => view! { <SemaineAgenda lignes=a.lignes peut_cocher=peut_cocher/> }.into_any(),
        }}
    }
}

#[component]
fn SemaineAgenda(lignes: Vec<AgendaLigneDto>, peut_cocher: bool) -> impl IntoView {
    let phrase = synthese(&lignes);
    let jours = grouper_par_jour(lignes);
    view! {
        <p class="cdt-resume" role="status">{phrase}</p>
        <div class="cdt-semaine">
            {jours
                .into_iter()
                .map(|j| {
                    let id_titre = format!("cdt-c3-{}", j.jour);
                    let pour = id_titre.clone();
                    let total = (j.minutes > 0).then(|| format!("Temps estimé : {}", libelle_duree(j.minutes)));
                    view! {
                        <section class="cdt-jour" aria-labelledby=pour>
                            <h2 id=id_titre class="cdt-jour__titre">
                                {format!("Pour le {}", date_longue(j.jour))}
                                {total.map(|t| view! { <span class="cdt-jour__total">{t}</span> })}
                            </h2>
                            <ul class="cdt-liste">
                                {j.lignes
                                    .into_iter()
                                    .map(|l| view! { <li><LigneAgenda ligne=l peut_cocher=peut_cocher/></li> })
                                    .collect_view()}
                            </ul>
                        </section>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn LigneAgenda(ligne: AgendaLigneDto, peut_cocher: bool) -> impl IntoView {
    let erreur = RwSignal::new(None::<String>);
    let annonce = RwSignal::new(String::new());
    let echec = gestionnaire_echec(erreur);
    let id = ligne.id;
    let est_travail = ligne.genre == GENRE_TRAVAIL;
    // La coche n'existe que pour l'élève (le serveur ne la sert pas à une famille).
    let fait = RwSignal::new(ligne.fait.unwrap_or(false));
    let cochable = peut_cocher && est_travail && ligne.fait.is_some();
    let ignorer = StoredValue::new(false);
    Effect::new(move |precedent: Option<bool>| {
        let coche = fait.get();
        if let Some(avant) = precedent {
            if avant != coche {
                if ignorer.get_value() {
                    ignorer.set_value(false);
                } else {
                    spawn_local(async move {
                        match session::client().cdt_cocher(id, coche).await {
                            Ok(()) => annonce.set(if coche {
                                "Marqué comme fait.".into()
                            } else {
                                "Marqué comme à faire.".into()
                            }),
                            Err(e) => {
                                ignorer.set_value(true);
                                fait.set(!coche);
                                echec.run(e);
                            }
                        }
                    });
                }
            }
        }
        coche
    });
    let statut = libelle_statut(ligne.statut.as_deref());
    let heure = ligne
        .heure
        .as_deref()
        .map(|h| format!("avant {}", heure_courte(h)));
    let duree = ligne
        .duree_estimee_min
        .map(|d| format!("Durée estimée : {}", libelle_duree(i32::from(d))));
    let auteur = ligne.auteur.clone();
    let detail = [duree, heure, auteur]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    let pref = format!("cdt-c3-fait-{id}");
    let evaluation = ligne.genre != GENRE_TRAVAIL;
    view! {
        <article class="cdt-travail" class:cdt-travail--controle=evaluation>
            <Annonce message=Signal::derive(move || annonce.get())/>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            <header class="cdt-travail__entete">
                <h3 class="cdt-travail__titre">{format!("{} — {}", ligne.matiere, ligne.titre)}</h3>
                <span class="cdt-travail__badges">
                    <Badge libelle=libelle_genre(&ligne.genre) ton=ton_genre(&ligne.genre)/>
                    {statut.map(|(l, t)| view! { <Badge libelle=l ton=t/> })}
                </span>
            </header>
            {(!detail.is_empty()).then(|| view! { <p class="cdt-travail__detail">{detail}</p> })}
            {ligne.echeance_prolongee.then(|| view! { <p class="cdt-travail__detail">"Échéance prolongée pour cet élève."</p> })}
            {ligne.modifie_le.map(|_| view! { <p class="cdt-travail__detail">"Modifié depuis sa publication (version " {ligne.version} ")."</p> })}
            {ligne.consigne.clone().filter(|c| !c.trim().is_empty()).map(|c| view! {
                <details class="cdt-seance__saisie">
                    <summary>"Voir la consigne"</summary>
                    <p class="cdt-travail__consigne">{c}</p>
                </details>
            })}
            {cochable.then(|| view! { <Case id=pref.clone() libelle="J'ai terminé ce travail" coche=fait/> })}
        </article>
    }
}

// ───────────────────────── Ce que j'ai manqué ─────────────────────────

#[component]
fn VueManque(eleve: Option<Uuid>) -> impl IntoView {
    let manque = LocalResource::new(move || async move {
        let (du, au) = periode_manque(aujourdhui());
        session::client().cdt_manque(eleve, du, au).await
    });
    view! {
        <p class="cdt-resume">"Le contenu des séances des 14 derniers jours où l'absence a été constatée."</p>
        {move || match manque.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des séances manquées…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| manque.refetch())/> }.into_any(),
            Some(Ok(m)) if m.seances.is_empty() => view! {
                <EtatVide titre="Rien à rattraper" message="Aucune séance manquée avec un contenu publié sur cette période."/>
            }.into_any(),
            Some(Ok(m)) => view! {
                <ul class="cdt-liste">
                    {m.seances
                        .into_iter()
                        .map(|s| {
                            let notions = s.notions.clone();
                            view! {
                                <li>
                                    <article class="cdt-seance">
                                        <header class="cdt-seance__entete">
                                            <h2 class="cdt-seance__titre">
                                                {format!("{} — {}", s.matiere, date_longue(s.jour))}
                                            </h2>
                                            <span class="cdt-seance__detail">
                                                {format!("{} – {}", heure_courte(&s.heure_debut), heure_courte(&s.heure_fin))}
                                                {s.auteur.clone().map(|a| format!(" · {a}"))}
                                            </span>
                                        </header>
                                        {s.titre.clone().map(|t| view! { <p class="cdt-seance__detail">{t}</p> })}
                                        {s.contenu.clone().map(|c| view! { <p class="cdt-seance__apercu">{c}</p> })}
                                        {(!notions.is_empty()).then(|| view! {
                                            <p class="cdt-seance__detail">{format!("Notions : {}", notions.join(", "))}</p>
                                        })}
                                        {(!s.travaux.is_empty()).then(|| view! {
                                            <ul class="cdt-liste">
                                                {s.travaux
                                                    .iter()
                                                    .map(|t| view! {
                                                        <li>
                                                            <p class="cdt-travail__titre">{t.titre.clone()}</p>
                                                            <p class="cdt-travail__consigne">{t.consigne.clone()}</p>
                                                            <p class="cdt-travail__detail">
                                                                {format!("Pour le {}", date_longue(t.echeance_jour))}
                                                            </p>
                                                        </li>
                                                    })
                                                    .collect_view()}
                                            </ul>
                                        })}
                                    </article>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            }.into_any(),
        }}
    }
}

// ───────────────────────── La semaine ─────────────────────────

#[component]
fn VueSemaine(eleve: Option<Uuid>, lundi: chrono::NaiveDate) -> impl IntoView {
    let resume =
        LocalResource::new(move || async move { session::client().cdt_resume(eleve, lundi).await });
    view! {
        {move || match resume.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de la semaine…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| resume.refetch())/> }.into_any(),
            Some(Ok(r)) => {
                let duree = (r.duree_estimee_min > 0).then(|| format!("Temps de travail estimé : {}.", libelle_duree(r.duree_estimee_min)));
                view! {
                    <ul class="cdt-liste" aria-label="Décompte de la semaine">
                        <li class="cdt-resume">{format!("{} travail(aux) à rendre ou à faire", r.nb_travaux)}</li>
                        <li class="cdt-resume">{format!("{} évaluation(s) annoncée(s)", r.nb_evaluations)}</li>
                        <li class="cdt-resume">{format!("{} sortie(s) ou épreuve(s)", r.nb_autres)}</li>
                    </ul>
                    {duree.map(|d| view! { <p class="cdt-resume">{d}</p> })}
                    <SemaineAgenda lignes=r.lignes peut_cocher=false/>
                }.into_any()
            }
        }}
    }
}

// ───────────────────────── Calendrier ─────────────────────────

#[cfg(target_arch = "wasm32")]
fn origine_site() -> String {
    // Origine du site lue sur l'adresse de base du document (aucune API supplémentaire nécessaire).
    let base = leptos::prelude::document()
        .base_uri()
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut parties = base.splitn(4, '/');
    match (parties.next(), parties.next(), parties.next()) {
        (Some(schema), Some(""), Some(hote)) => format!("{schema}//{hote}"),
        _ => String::new(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn origine_site() -> String {
    String::new()
}

#[component]
fn VueCalendrier(eleve: Option<Uuid>) -> impl IntoView {
    let abonnements = LocalResource::new(|| async { session::client().cdt_abonnements().await });
    let erreur = RwSignal::new(None::<String>);
    let cree = RwSignal::new(None::<AbonnementCreeDto>);
    let occupe = RwSignal::new(false);
    let echec = gestionnaire_echec(erreur);
    let creer = Callback::new(move |_: ()| {
        if occupe.get_untracked() {
            return;
        }
        occupe.set(true);
        erreur.set(None);
        spawn_local(async move {
            match session::client().cdt_abonnement_creer(eleve).await {
                Ok(a) => {
                    cree.set(Some(a));
                    abonnements.refetch();
                }
                Err(e) => echec.run(e),
            }
            occupe.set(false);
        });
    });
    let revoquer = Callback::new(move |id: Uuid| {
        erreur.set(None);
        spawn_local(async move {
            match session::client().cdt_abonnement_revoquer(id).await {
                Ok(()) => {
                    cree.set(None);
                    abonnements.refetch();
                }
                Err(e) => echec.run(e),
            }
        });
    });
    view! {
        <p class="cdt-resume">
            "Ajoutez vos travaux et évaluations à l'agenda de votre téléphone ou de votre ordinateur. "
            "Le calendrier ne contient ni consigne ni nom d'enseignant. Ne partagez pas son adresse : "
            "elle donne accès à cet agenda tant qu'elle n'est pas révoquée."
        </p>
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        {move || cree.get().map(|c| {
            let url = super::modele_c3::adresse_abonnement(&origine_site(), &c.chemin);
            match url {
                Some(u) => view! {
                    <div class="cdt-form">
                        <Alerte niveau=Niveau::Info titre="Copiez cette adresse maintenant : elle ne sera plus affichée.".to_string()/>
                        <label for="cdt-c3-adresse">"Adresse d'abonnement"</label>
                        <input id="cdt-c3-adresse" class="ui-champ" type="text" readonly=true prop:value=u/>
                    </div>
                }.into_any(),
                None => view! { <Alerte niveau=Niveau::Erreur titre="Adresse d'abonnement illisible.".to_string()/> }.into_any(),
            }
        })}
        <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| creer.run(())) desactive=Signal::derive(move || occupe.get())>
            "Créer un abonnement"
        </Bouton>
        {move || match abonnements.get() {
            None => view! { <Squelette lignes=2 libelle="Chargement des abonnements…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| abonnements.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <p class="cdt-vide">"Aucun abonnement actif."</p> }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="cdt-liste" aria-label="Abonnements actifs">
                    {l.into_iter().map(|a| {
                        let id = a.id;
                        view! {
                            <li class="cdt-travail">
                                <p class="cdt-travail__detail">{format!("Abonnement créé le {}", date_longue(a.cree_jour))}</p>
                                <Bouton variante=Variante::Secondaire taille=Taille::Compacte on_click=Callback::new(move |_| revoquer.run(id))>
                                    "Révoquer"
                                </Bouton>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            }.into_any(),
        }}
    }
}
