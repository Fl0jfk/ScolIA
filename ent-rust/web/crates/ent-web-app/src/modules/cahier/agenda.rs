//! Écran élève et famille : travail à faire, échéances, contrôles annoncés, rendu en ligne.
//!
//! L'élève agit sur lui-même (marquer fait, rendre) ; le responsable consulte l'agenda de
//! chaque enfant lié, sans pouvoir rendre ni marquer. Ce qui est visible (classe, groupe) est
//! décidé par le serveur.

use chrono::Utc;
use ent_web_api::cahier_types::{AgendaItemDto, PieceCahierDto};
use ent_web_api::{depot_fichier, Enfant};
use ent_web_ui::{
    use_notifications, AccesRefuse, Alerte, Annonce, Badge, Bouton, Case, EnTetePage, EtatVide,
    Icone, LienBouton, Niveau, NomIcone, OptionSegment, Segmente, Selection, Squelette, Taille,
    TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use super::fichiers::{lire_fichier_choisi, FichierChoisi, Lecture};
use super::modele::{
    etat_travail, fin_de_semaine, jours_agenda, libelle_duree, libelle_limite, libelle_semaine,
    libelle_taille, libelle_type, nb_a_faire, peut_rendre, semaine_demandee, semaine_voisine,
    valider_commentaire_eleve,
};
use super::{gestionnaire_echec, ouvrir_piece};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};

#[component]
pub fn PageAgenda() -> impl IntoView {
    let droits = use_droits();
    view! {
        <Title text="Travail à faire"/>
        {move || {
            let d = droits.get();
            if d.agenda && d.portail_famille {
                view! { <VueFamille/> }.into_any()
            } else if d.agenda {
                view! { <ContenuAgenda eleve=None peut_agir=true/> }.into_any()
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
            // Nouvelle instance par enfant : l'état (semaine, saisies) ne se mélange pas.
            id.map(|id| view! { <ContenuAgenda eleve=Some(id) peut_agir=false/> })
        }}
    }
}

#[component]
fn ContenuAgenda(eleve: Option<Uuid>, peut_agir: bool) -> impl IntoView {
    let query = use_query_map();
    let lundi =
        Memo::new(move |_| semaine_demandee(query.with(|q| q.get("semaine")), aujourdhui()));
    let agenda = LocalResource::new(move || {
        let l = lundi.get();
        async move {
            session::client()
                .cahier_agenda(eleve, l, fin_de_semaine(l))
                .await
        }
    });
    let adresse = move |decalage: i64| {
        format!(
            "{}?semaine={}",
            navigation::AGENDA,
            semaine_voisine(lundi.get(), decalage)
        )
    };
    view! {
        <EnTetePage
            surtitre="Travail à faire".to_string()
            titre=Signal::derive(move || libelle_semaine(lundi.get()))
        >
            <nav class="edt-semaines" aria-label="Changer de semaine">
                <a class="ui-btn ui-btn--secondaire ui-btn--compact" href=move || adresse(-1) aria-label="Semaine précédente">
                    <Icone nom=NomIcone::ChevronGauche petite=true/>
                </a>
                <LienBouton href=navigation::AGENDA variante=Variante::Secondaire taille=Taille::Compacte>
                    "Cette semaine"
                </LienBouton>
                <a class="ui-btn ui-btn--secondaire ui-btn--compact" href=move || adresse(1) aria-label="Semaine suivante">
                    <Icone nom=NomIcone::ChevronDroite petite=true/>
                </a>
            </nav>
        </EnTetePage>
        {move || match agenda.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement du travail à faire…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| agenda.refetch())/> }.into_any(),
            Some(Ok(items)) => view! { <SemaineAgenda items=items peut_agir=peut_agir on_change=Callback::new(move |_| agenda.refetch())/> }.into_any(),
        }}
    }
}

#[component]
fn SemaineAgenda(
    items: Vec<AgendaItemDto>,
    peut_agir: bool,
    on_change: Callback<()>,
) -> impl IntoView {
    if items.is_empty() {
        return view! {
            <EtatVide titre="Rien à faire cette semaine" message="Aucun travail ni contrôle n'est annoncé pour cette semaine."/>
        }
        .into_any();
    }
    let maintenant = Utc::now();
    let a_faire = nb_a_faire(&items, maintenant);
    let jours = jours_agenda(items);
    view! {
        <p class="cdt-resume" role="status">
            {if a_faire == 0 {
                "Tout est fait pour cette semaine.".to_string()
            } else {
                format!("{a_faire} travail{} encore à faire.", if a_faire > 1 { "x" } else { "" })
            }}
        </p>
        <div class="cdt-semaine">
            {jours
                .into_iter()
                .map(|j| {
                    let id_titre = format!("cdt-ag-{}", j.jour);
                    let pour = id_titre.clone();
                    let total = (j.minutes > 0).then(|| format!("Temps estimé : {}", libelle_duree(j.minutes)));
                    view! {
                        <section class="cdt-jour" aria-labelledby=pour>
                            <h2 id=id_titre class="cdt-jour__titre">
                                {format!("Pour le {}", date_longue(j.jour))}
                                {total.map(|t| view! { <span class="cdt-jour__total">{t}</span> })}
                            </h2>
                            <ul class="cdt-liste">
                                {j.items
                                    .into_iter()
                                    .map(|i| view! { <li><CarteTravail item=i peut_agir=peut_agir on_change=on_change/></li> })
                                    .collect_view()}
                            </ul>
                        </section>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

#[component]
fn CarteTravail(item: AgendaItemDto, peut_agir: bool, on_change: Callback<()>) -> impl IntoView {
    let maintenant = Utc::now();
    let etat = etat_travail(&item, maintenant);
    let t = item.travail.clone();
    let id = t.id;
    let erreur = RwSignal::new(None::<String>);
    let annonce = RwSignal::new(String::new());
    let notifications = use_notifications();
    let echec = gestionnaire_echec(erreur);
    let fait = RwSignal::new(item.fait);
    let pref = format!("cdt-fait-{id}");

    // L'élève qui ouvre la consigne la « voit » : une modification ultérieure le préviendra.
    let ouvert = move |ev: leptos::ev::Event| {
        if !peut_agir {
            return;
        }
        let ouvert = event_target::<web_sys::HtmlElement>(&ev)
            .get_attribute("open")
            .is_some();
        if ouvert {
            spawn_local(async move {
                let _ = session::client().cahier_travail(id).await;
            });
        }
    };
    // L'envoi suit la case ; en cas d'échec la case revient à son état sans renvoyer de requête.
    let ignorer = StoredValue::new(false);
    Effect::new(move |precedent: Option<bool>| {
        let coche = fait.get();
        if let Some(avant) = precedent {
            if avant != coche {
                if ignorer.get_value() {
                    ignorer.set_value(false);
                } else {
                    spawn_local(async move {
                        match session::client().cahier_marquer_fait(id, coche).await {
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
    let peut_marquer = peut_agir && !t.rendu_en_ligne;
    let peut_rendre_maintenant = peut_agir && peut_rendre(&item, maintenant);
    let limite = libelle_limite(&t);
    let duree = t.duree_estimee_min.map(libelle_duree);
    let ton_type = match t.type_travail.as_str() {
        "controle" => TonEtat::Avertissement,
        _ => TonEtat::Neutre,
    };
    let remise = item.remise.clone();
    view! {
        <article class="cdt-travail" class:cdt-travail--controle=t.type_travail == "controle">
            <Annonce message=Signal::derive(move || annonce.get())/>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            <header class="cdt-travail__entete">
                <h3 class="cdt-travail__titre">{format!("{} — {}", t.matiere_libelle, t.titre)}</h3>
                <span class="cdt-travail__badges">
                    <Badge libelle=libelle_type(&t.type_travail) ton=ton_type/>
                    <Badge libelle=etat.libelle() ton=etat.ton()/>
                </span>
            </header>
            <p class="cdt-travail__detail">
                {duree.map(|d| format!("Durée estimée : {d}"))}
                {limite.map(|l| format!(" · {l}"))}
            </p>
            <details class="cdt-seance__saisie" on:toggle=ouvert>
                <summary>"Voir la consigne"</summary>
                <p class="cdt-travail__consigne">{t.consigne.clone()}</p>
                <PiecesTravail pieces=t.pieces.clone() erreur=erreur/>
            </details>
            {peut_marquer.then(|| view! {
                <Case id=pref.clone() libelle="J'ai terminé ce travail" coche=fait/>
            })}
            {remise.map(|r| view! { <ResumeRemise remise=r erreur=erreur/> })}
            {peut_rendre_maintenant.then(|| view! { <FormulaireRendu travail_id=id notifications_ok=notifications erreur=erreur on_change=on_change/> })}
        </article>
    }
}

#[component]
fn PiecesTravail(pieces: Vec<PieceCahierDto>, erreur: RwSignal<Option<String>>) -> impl IntoView {
    if pieces.is_empty() {
        return ().into_any();
    }
    view! {
        <ul class="cdt-pieces">
            {pieces
                .into_iter()
                .map(|p| {
                    let pid = p.id;
                    let aria = format!("Télécharger {}", p.nom);
                    view! {
                        <li>
                            <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=aria
                                on_click=Callback::new(move |_| ouvrir_piece(pid, erreur))>
                                {format!("{} ({})", p.nom, libelle_taille(p.taille_octets))}
                            </Bouton>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

#[component]
fn ResumeRemise(
    remise: ent_web_api::cahier_types::RemiseDto,
    erreur: RwSignal<Option<String>>,
) -> impl IntoView {
    let retour = remise.retour_commentaire.clone();
    let annotations: Vec<PieceCahierDto> = remise
        .pieces
        .iter()
        .filter(|p| p.genre == "annotation")
        .cloned()
        .collect();
    let copies = remise.pieces.iter().filter(|p| p.genre == "copie").count();
    view! {
        <div class="cdt-remise">
            <p class="cdt-travail__detail">
                {format!("Rendu le {} ({} fichier{})", remise.soumise_le.format("%d/%m/%Y"), copies, if copies > 1 { "s" } else { "" })}
            </p>
            {retour.map(|r| view! {
                <Alerte niveau=Niveau::Succes titre="Retour de l'enseignant">
                    <p>{r}</p>
                    <PiecesTravail pieces=annotations erreur=erreur/>
                </Alerte>
            })}
        </div>
    }
}

#[component]
fn FormulaireRendu(
    travail_id: Uuid,
    notifications_ok: ent_web_ui::Notifications,
    erreur: RwSignal<Option<String>>,
    on_change: Callback<()>,
) -> impl IntoView {
    let commentaire = RwSignal::new(String::new());
    let err_commentaire = RwSignal::new(None::<String>);
    let fichier = RwSignal::new(None::<FichierChoisi>);
    let err_fichier = RwSignal::new(None::<String>);
    let champ = NodeRef::<leptos::html::Input>::new();
    let en_cours = RwSignal::new(false);
    let echec = gestionnaire_echec(erreur);
    let choisir = move |_| {
        let Some(input) = champ.get() else { return };
        spawn_local(async move {
            match lire_fichier_choisi(&input).await {
                Lecture::Aucun => fichier.set(None),
                Lecture::Fichier(f) => {
                    err_fichier.set(None);
                    fichier.set(Some(f));
                }
                Lecture::Refuse(m) => {
                    fichier.set(None);
                    input.set_value("");
                    err_fichier.set(Some(m));
                }
            }
        });
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        erreur.set(None);
        let texte = match valider_commentaire_eleve(&commentaire.get_untracked()) {
            Ok(t) => t,
            Err(m) => {
                err_commentaire.set(Some(m));
                return;
            }
        };
        err_commentaire.set(None);
        let Some(f) = fichier.get_untracked() else {
            err_fichier.set(Some("Choisissez le fichier à rendre.".into()));
            return;
        };
        let depot = depot_fichier(&f.nom, &f.mime, &f.octets);
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client()
                .cahier_rendre(travail_id, &texte, Some(depot))
                .await;
            en_cours.set(false);
            match r {
                Ok(_) => {
                    notifications_ok.succes("Travail rendu : votre enseignant est prévenu.");
                    on_change.run(());
                }
                Err(e) => echec.run(e),
            }
        });
    };
    let pref = format!("cdt-rendu-{travail_id}");
    view! {
        <details class="cdt-seance__saisie">
            <summary>"Rendre mon travail"</summary>
            <form class="cdt-form" on:submit=envoyer novalidate=true>
                <div class="ui-champ">
                    <label class="ui-champ__libelle" for=format!("{pref}-fichier")>"Fichier à rendre"<span class="ui-champ__requis" aria-hidden="true">"*"</span></label>
                    <input id=format!("{pref}-fichier") class="ui-champ__controle" type="file" node_ref=champ
                        aria-describedby=format!("{pref}-fichier-aide") on:change=choisir/>
                    <p id=format!("{pref}-fichier-aide") class="ui-champ__aide">"10 Mo au maximum. PDF, image, texte ou document bureautique sans macro."</p>
                    {move || err_fichier.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                    {move || fichier.get().map(|f| view! { <p class="ui-champ__aide">{format!("Fichier choisi : {}", f.nom)}</p> })}
                </div>
                <ZoneTexte id=format!("{pref}-commentaire") libelle="Mot pour l'enseignant (facultatif)" valeur=commentaire erreur=err_commentaire/>
                <Bouton soumettre=true en_cours=en_cours>"Rendre"</Bouton>
            </form>
        </details>
    }
}
