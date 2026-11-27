//! Vie scolaire (personnel) : « Absences à traiter », tableau de bord du jour, détail d'une
//! absence avec décision confirmée, signaux d'absence répétée.
//!
//! Routes existantes d'`ent-api` : `GET /vs/absences?jour=`, `POST /absences/{id}/accepter-
//! justificatif|refuser-justificatif|annuler-present`, `GET /vs/signaux`, `POST /vs/signaux/
//! {id}/prendre|clore`, `GET /eleves/{id}` (classe), `GET /eleves/{id}/absences|retards`
//! (historique). Les droits et les règles métier restent décidés par le serveur.
//!
//! Limite de l'API : `GET /vs/absences` ne renvoie que les absences **à traiter** du jour.
//! Les absences décidées pendant la session restent affichées avec leur nouveau statut ;
//! l'historique complet d'un élève est dans le détail d'une absence.

mod detail;
pub mod modele;
mod signaux;

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use ent_web_api::{Absence, ApiError};
use ent_web_ui::{
    AccesRefuse, Annonce, Badge, Bouton, Chargement, EnTetePage, EtatVide, FiltrePastille,
    Recherche, Selection, Squelette, Statistique, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_query_map};
use uuid::Uuid;

use crate::composants_metier::absences::{
    a_un_justificatif, eleves_absents, jour_demande, libelle_periode, nom_eleve, Compteurs,
    StatutAbsence,
};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::temps::{aujourdhui, date_longue};
use crate::noyau::{reconnexion, session};
use modele::{classes_connues, filtrer, fusionner, trier, Filtre};

/// Réaction commune à l'échec d'une action : session expirée → retour à la connexion
/// (après effacement de la session) ; sinon le message est affiché là où l'action a eu lieu.
/// Aucune requête n'est relancée automatiquement.
pub(crate) fn gestionnaire_echec(erreur: RwSignal<Option<String>>) -> Callback<ApiError> {
    Callback::new(move |e: ApiError| match e {
        ApiError::NonConnecte => {
            reconnexion::signaler_session_perdue();
            erreur.set(Some(reconnexion::MESSAGE_SESSION_EXPIREE.into()));
        }
        autre => erreur.set(Some(autre.message_utilisateur())),
    })
}

#[component]
pub fn PageVieScolaire() -> impl IntoView {
    let droits = use_droits();
    let ctx = session::use_session();
    view! {
        <Title text="Vie scolaire"/>
        {move || match ctx.moi.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement de la vie scolaire…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| ctx.moi.refetch())/> }.into_any(),
            Some(Ok(_)) if droits.get().lire_vs_absences => view! { <EcranVieScolaire/> }.into_any(),
            Some(Ok(_)) => view! {
                <EnTetePage titre="Vie scolaire".to_string()/>
                <AccesRefuse message="Les absences de l'établissement ne sont pas accessibles avec votre profil."/>
            }
                .into_any(),
        }}
    }
}

#[component]
fn EcranVieScolaire() -> impl IntoView {
    let query = use_query_map();
    let navigate = use_navigate();
    let jour_brut =
        Memo::new(move |_| query.with(|q| jour_demande(q.get("jour").as_deref(), aujourdhui())));
    let jour: Memo<NaiveDate> = Memo::new(move |_| jour_brut.get().0);

    let absences = LocalResource::new(move || {
        let j = jour.get();
        async move { session::client().absences_a_traiter(j).await }
    });
    let signaux = LocalResource::new(|| async { session::client().signaux_absence().await });
    let liste_signaux = RwSignal::new(Vec::<ent_web_api::SignalAbsence>::new());

    // Décisions prises pendant la session (l'API ne relist plus ces absences).
    let decidees = RwSignal::new(Vec::<Absence>::new());
    Effect::new(move |_| {
        jour.track();
        decidees.set(Vec::new());
    });
    let annonce = RwSignal::new(String::new());

    let toutes = Memo::new(move |_| match absences.get() {
        Some(Ok(l)) => decidees.with(|d| trier(fusionner(l, d, jour.get()))),
        _ => Vec::new(),
    });
    let compteurs = Memo::new(move |_| toutes.with(|t| Compteurs::de(t)));
    let absents = Memo::new(move |_| toutes.with(|t| eleves_absents(t)));

    // Classes : une requête par élève (la liste du serveur ne les contient pas), jamais
    // deux fois pour le même élève, en arrière-plan (l'écran n'attend pas).
    let classes = RwSignal::new(HashMap::<Uuid, String>::new());
    let essayes = StoredValue::new(HashSet::<Uuid>::new());
    let ids = Memo::new(move |_| {
        let mut v: Vec<Uuid> = toutes.with(|t| t.iter().map(|a| a.eleve_id).collect());
        v.sort();
        v.dedup();
        v
    });
    Effect::new(move |_| {
        let manquants: Vec<Uuid> = ids
            .get()
            .into_iter()
            .filter(|i| !essayes.with_value(|e| e.contains(i)))
            .collect();
        if manquants.is_empty() {
            return;
        }
        essayes.update_value(|e| e.extend(manquants.iter().copied()));
        spawn_local(async move {
            let client = session::client();
            for id in manquants {
                match client.fiche_eleve(id).await {
                    Ok(f) => {
                        if let Some(c) = f.classe.filter(|c| !c.trim().is_empty()) {
                            classes.update(|m| {
                                m.insert(id, c);
                            });
                        }
                    }
                    // Plus de session, de droits ou de quota : inutile d'insister.
                    Err(
                        ApiError::NonConnecte | ApiError::Interdit(_) | ApiError::TropDeRequetes,
                    ) => break,
                    Err(_) => {}
                }
            }
        });
    });
    let liste_classes = Memo::new(move |_| classes.with(classes_connues));

    // Filtres.
    let statut_filtre = RwSignal::new(None::<StatutAbsence>);
    let classe_filtre = RwSignal::new(String::new());
    let recherche = RwSignal::new(String::new());
    let visibles = Memo::new(move |_| {
        let filtre = Filtre {
            statut: statut_filtre.get(),
            classe: Some(classe_filtre.get()).filter(|c| !c.is_empty()),
            recherche: recherche.get(),
        };
        toutes.with(|t| classes.with(|c| filtrer(t, c, &filtre)))
    });
    let a_filtrer = Signal::derive(move || {
        statut_filtre.get().is_some()
            || !classe_filtre.get().is_empty()
            || !recherche.get().trim().is_empty()
    });

    // Détail.
    let selection = RwSignal::new(None::<Uuid>);
    let detail_ouvert = RwSignal::new(false);
    let examiner = Callback::new(move |id: Uuid| {
        selection.set(Some(id));
        detail_ouvert.set(true);
    });

    let changer_jour = move |ev: leptos::ev::Event| {
        if let Ok(j) = event_target_value(&ev).parse::<NaiveDate>() {
            navigate(
                &format!("{}?jour={j}", navigation::VIE_SCOLAIRE),
                Default::default(),
            );
        }
    };
    let sous_titre = Signal::derive(move || {
        let j = jour.get();
        if j == aujourdhui() {
            "Absences relevées par les appels clos et déclarées par les familles, à traiter aujourd'hui."
                .to_string()
        } else {
            "Absences à traiter ce jour-là.".to_string()
        }
    });

    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>
        <EnTetePage
            surtitre=Signal::derive(move || date_longue(jour.get()))
            titre="Vie scolaire".to_string()
            sous_titre=sous_titre
        >
            <label class="appel-jour">
                <span class="ui-sr">"Jour affiché"</span>
                <input
                    id="jour"
                    type="date"
                    prop:value=move || jour.get().to_string()
                    on:change=changer_jour
                />
            </label>
        </EnTetePage>

        {move || {
            jour_brut
                .get()
                .1
                .then(|| {
                    view! {
                        <ent_web_ui::Alerte
                            niveau=ent_web_ui::Niveau::Avertissement
                            titre="La date demandée dans l'adresse est invalide."
                        >
                            <p>{format!("Affichage du jour courant ({}).", date_longue(jour.get()))}</p>
                        </ent_web_ui::Alerte>
                    }
                })
        }}

        <section class="vs-stats" aria-label="Tableau de bord du jour">
            <Statistique
                libelle="Élèves absents"
                valeur=Signal::derive(move || chargee(absences, absents.get()))
                ton=TonEtat::Mal
            />
            <Statistique
                libelle="Justificatifs en attente"
                valeur=Signal::derive(move || chargee(absences, compteurs.get().en_attente))
                ton=TonEtat::Avertissement
            />
            <Statistique
                libelle="Non justifiées"
                valeur=Signal::derive(move || chargee(absences, compteurs.get().non_justifiees))
            />
            <Statistique
                libelle="Absences répétées"
                valeur=Signal::derive(move || match signaux.get() {
                    Some(Ok(_)) => liste_signaux.with(Vec::len).to_string(),
                    _ => "–".to_string(),
                })
            />
        </section>
        <p class="vs-note">
            "Les retards sont consultables dans le détail de chaque absence (historique de l'élève) : "
            "le serveur ne fournit pas encore de liste des retards du jour."
        </p>

        {move || match absences.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement des absences…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| absences.refetch())/>
            }
                .into_any(),
            Some(Ok(_)) => view! {
                <section class="ui-tableau-cadre" aria-label="Absences du jour">
                    <div class="ui-tableau-outils" role="group" aria-label="Filtres">
                        <Recherche id="vs-recherche" libelle="Rechercher un élève" valeur=recherche placeholder="Rechercher un élève"/>
                        {move || {
                            let l = liste_classes.get();
                            if l.is_empty() {
                                view! { <span class="vs-classes-attente"><Chargement libelle="Classes…"/></span> }.into_any()
                            } else {
                                let mut options = vec![(String::new(), "Toutes les classes".to_string())];
                                options.extend(l.into_iter().map(|c| (c.clone(), c)));
                                view! {
                                    <div class="vs-filtre-classe">
                                        <Selection id="vs-classe" libelle="Classe" valeur=classe_filtre options=options/>
                                    </div>
                                }
                                    .into_any()
                            }
                        }}
                        <div class="appel-pastilles" role="group" aria-label="Filtrer par statut">
                            <FiltrePastille
                                libelle="Tous"
                                nombre=Signal::derive(move || toutes.with(Vec::len))
                                actif=Signal::derive(move || statut_filtre.get().is_none())
                                teinte="tous"
                                on_click=Callback::new(move |_| statut_filtre.set(None))
                            />
                            {StatutAbsence::TOUS
                                .into_iter()
                                .map(|s| {
                                    view! {
                                        <FiltrePastille
                                            libelle=s.libelle()
                                            nombre=Signal::derive(move || compteurs.get().pour(s))
                                            actif=Signal::derive(move || statut_filtre.get() == Some(s))
                                            teinte=s.code()
                                            on_click=Callback::new(move |_| {
                                                statut_filtre.update(|f| *f = if *f == Some(s) { None } else { Some(s) })
                                            })
                                        />
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div>
                    <div class="ui-tableau-defile">
                        <table class="ui-tableau vs-tableau">
                            <caption class="ui-sr">"Absences du jour"</caption>
                            <thead>
                                <tr>
                                    <th scope="col">"Élève"</th>
                                    <th scope="col">"Période"</th>
                                    <th scope="col">"Justification"</th>
                                    <th scope="col">"Statut"</th>
                                    <th scope="col"><span class="ui-sr">"Action"</span></th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || visibles.get()
                                    key=|a| (a.id, a.statut.clone(), a.motif_justif.clone())
                                    children=move |a| view! { <LigneAbsence absence=a classes=classes on_examiner=examiner/> }
                                />
                            </tbody>
                        </table>
                    </div>
                    {move || {
                        visibles
                            .with(Vec::is_empty)
                            .then(|| {
                                if toutes.with(Vec::is_empty) {
                                    view! {
                                        <EtatVide
                                            titre="Aucune absence à traiter ce jour"
                                            message="Les absences issues des appels clos apparaissent ici dès la clôture de l'appel."
                                        />
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <EtatVide titre="Aucune absence ne correspond" message="Modifiez la recherche ou les filtres.">
                                            <Show when=move || a_filtrer.get()>
                                                <Bouton
                                                    variante=Variante::Secondaire
                                                    on_click=Callback::new(move |_| {
                                                        statut_filtre.set(None);
                                                        classe_filtre.set(String::new());
                                                        recherche.set(String::new());
                                                    })
                                                >
                                                    "Effacer les filtres"
                                                </Bouton>
                                            </Show>
                                        </EtatVide>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                    <p class="appel-aide">
                        "Cette liste reprend les absences encore à traiter ce jour ; celles que vous décidez ici restent affichées avec leur nouveau statut jusqu'au changement de jour."
                    </p>
                </section>
            }
                .into_any(),
        }}

        <signaux::SectionSignaux signaux=signaux liste=liste_signaux annonce=annonce/>
        <detail::DetailAbsence
            selection=selection
            ouvert=detail_ouvert
            toutes=toutes
            classes=classes
            decidees=decidees
            annonce=annonce
            absences=absences
        />
    }
}

/// Valeur d'un compteur, « – » tant que les absences ne sont pas chargées.
fn chargee(absences: LocalResource<Result<Vec<Absence>, ApiError>>, n: usize) -> String {
    match absences.get() {
        Some(Ok(_)) => n.to_string(),
        _ => "–".to_string(),
    }
}

#[component]
fn LigneAbsence(
    absence: Absence,
    classes: RwSignal<HashMap<Uuid, String>>,
    on_examiner: Callback<Uuid>,
) -> impl IntoView {
    let statut = StatutAbsence::depuis(&absence);
    let nom = nom_eleve(&absence);
    let id = absence.id;
    let eleve_id = absence.eleve_id;
    let periode = libelle_periode(absence.debut, absence.fin);
    let justification = if a_un_justificatif(&absence) {
        absence.motif_justif.clone().unwrap_or_default()
    } else {
        "Aucune".to_string()
    };
    let libelle_bouton = format!("Examiner l'absence de {nom}");
    view! {
        <tr class="vs-ligne" data-statut=statut.code()>
            <th scope="row" class="vs-ligne__eleve">
                <span class="vs-ligne__nom">{nom}</span>
                <span class="vs-ligne__classe">
                    {move || classes.with(|c| c.get(&eleve_id).cloned()).unwrap_or_default()}
                </span>
            </th>
            <td class="vs-ligne__periode" data-libelle="Période">{periode}</td>
            <td class="vs-ligne__justification" data-libelle="Justification">{justification}</td>
            <td class="vs-ligne__statut"><Badge libelle=statut.libelle() ton=statut.ton()/></td>
            <td class="vs-ligne__action">
                <Bouton
                    variante=Variante::Secondaire
                    taille=Taille::Compacte
                    libelle_accessible=libelle_bouton
                    on_click=Callback::new(move |_| on_examiner.run(id))
                >
                    "Examiner"
                </Bouton>
            </td>
        </tr>
    }
}
