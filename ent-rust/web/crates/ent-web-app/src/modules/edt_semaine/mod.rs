//! EDT de la semaine (socle 2, S2-5) : personnel (soi, classe, salle, autre membre du personnel),
//! élève et famille (par enfant). Route `GET /edt/semaine` : séances avec leur statut (annulé,
//! remplacé, déplacé), jours fermés et semaine A/B. Lecture seule ; le serveur décide des droits
//! (le personnel lit classes, salles et collègues ; l'élève et la famille, l'élève lié seulement).

pub mod modele;

use chrono::NaiveDate;
use ent_types::edt_s2::SemaineEdtDto;
use ent_web_api::{ApiError, CibleSemaine, Enfant, Moi};
use ent_web_ui::{
    AccesRefuse, Annonce, Badge, Bouton, EnTetePage, EtatVide, Icone, NomIcone, Selection,
    Squelette, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::choix_enfant::{enfant_choisi, ChoixEnfant};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use modele::{
    cible_depuis, libelle_semaine, libelle_semaine_ab, lundi_de, nb_changements,
    regrouper_par_jour, semaine_voisine, JourAffiche, Vue,
};

#[component]
pub fn PageSemaineEdt() -> impl IntoView {
    let droits = use_droits();
    let ctx = session::use_session();
    view! {
        <Title text="Emploi du temps de la semaine"/>
        <EnTetePage
            surtitre="Emploi du temps".to_string()
            titre="Emploi du temps de la semaine".to_string()
            sous_titre="Cours de la semaine, avec les annulations, remplacements et déplacements.".to_string()
        />
        {move || match ctx.moi.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de l'emploi du temps…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| ctx.moi.refetch())/> }.into_any(),
            Some(Ok(moi)) => {
                let d = droits.get();
                if d.personnel {
                    view! { <VuePersonnel/> }.into_any()
                } else if d.portail_famille {
                    view! { <VueFamille/> }.into_any()
                } else if d.mes_absences {
                    view! { <VueEleve moi=moi/> }.into_any()
                } else {
                    view! { <AccesRefuse message="Cet écran n'est pas disponible pour votre profil."/> }.into_any()
                }
            }
        }}
    }
}

// ---------------------------------------------------------------------------
// Personnel
// ---------------------------------------------------------------------------

/// Options (valeur, libellé) de la liste de choix selon la vue ; la première est « Choisir… ».
async fn options_de(vue: Vue) -> Result<Vec<(String, String)>, ApiError> {
    let client = session::client();
    let mut options = vec![(String::new(), "Choisir…".to_string())];
    match vue {
        Vue::Moi => return Ok(Vec::new()),
        Vue::Classe => {
            let mut classes = client.classes(aujourdhui()).await?;
            classes.sort_by(|a, b| a.code.cmp(&b.code));
            options.extend(classes.into_iter().map(|c| {
                let libelle = if c.libelle.trim().is_empty() || c.libelle == c.code {
                    c.code.clone()
                } else {
                    format!("{} — {}", c.code, c.libelle.trim())
                };
                (c.id.to_string(), libelle)
            }));
        }
        Vue::Salle => {
            let mut salles = client.salles().await?;
            salles.sort_by(|a, b| a.code.cmp(&b.code));
            options.extend(salles.into_iter().map(|s| {
                let libelle = if s.libelle.trim().is_empty() || s.libelle == s.code {
                    s.code.clone()
                } else {
                    format!("{} — {}", s.code, s.libelle.trim())
                };
                (s.code, libelle)
            }));
        }
        Vue::Personnel => {
            let mut personnel = client.personnel("").await?;
            personnel.sort_by(|a, b| a.label.cmp(&b.label));
            options.extend(
                personnel
                    .into_iter()
                    .map(|p| (p.user_id.to_string(), p.label)),
            );
        }
    }
    Ok(options)
}

#[component]
fn VuePersonnel() -> impl IntoView {
    let vue = RwSignal::new(Vue::Moi.code().to_string());
    let choix = RwSignal::new(String::new());
    let vue_courante = Memo::new(move |_| Vue::depuis_code(&vue.get()));
    // Changer de vue efface le choix précédent (une classe n'est pas une salle).
    Effect::new(move |_| {
        vue_courante.track();
        choix.set(String::new());
    });
    let options = LocalResource::new(move || {
        let v = vue_courante.get();
        async move { options_de(v).await }
    });
    let cible = Signal::derive(move || cible_depuis(vue_courante.get(), &choix.get()));
    let montre_professeur = Signal::derive(move || vue_courante.get().montre_le_professeur());
    let vues: Vec<(String, String)> = Vue::TOUTES
        .into_iter()
        .map(|v| (v.code().to_string(), v.libelle().to_string()))
        .collect();
    view! {
        <div class="edts-choix">
            <Selection id="edt-vue" libelle="Afficher" valeur=vue options=vues/>
            {move || {
                let v = vue_courante.get();
                v.libelle_choix().map(|etiquette| match options.get() {
                    None => view! { <Squelette lignes=1 libelle="Chargement de la liste…"/> }.into_any(),
                    Some(Err(e)) => view! {
                        <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| options.refetch())/>
                    }
                    .into_any(),
                    Some(Ok(o)) => view! { <Selection id="edt-choix" libelle=etiquette valeur=choix options=o/> }.into_any(),
                })
            }}
        </div>
        <SemaineAffichee cible=cible montre_professeur=montre_professeur/>
    }
}

// ---------------------------------------------------------------------------
// Famille et élève
// ---------------------------------------------------------------------------

#[component]
fn VueFamille() -> impl IntoView {
    let enfants = LocalResource::new(|| async { session::client().enfants_famille().await });
    view! {
        {move || match enfants.get() {
            None => view! { <Squelette lignes=2 libelle="Chargement de vos enfants…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| enfants.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide
                    titre="Aucun enfant rattaché"
                    message="Aucun enfant n'est lié à votre compte. Contactez le secrétariat de l'établissement."
                />
            }
            .into_any(),
            Some(Ok(l)) => view! { <SemaineEnfants enfants=l/> }.into_any(),
        }}
    }
}

#[component]
fn SemaineEnfants(enfants: Vec<Enfant>) -> impl IntoView {
    let liste = StoredValue::new(enfants.clone());
    let choisi = RwSignal::new(
        enfant_choisi(&enfants, None)
            .map(|i| i.to_string())
            .unwrap_or_default(),
    );
    let cible = Signal::derive(move || {
        let demande = choisi.get().parse::<Uuid>().ok();
        liste
            .with_value(|l| enfant_choisi(l, demande))
            .map(CibleSemaine::Eleve)
    });
    view! {
        <ChoixEnfant enfants=enfants choisi=choisi/>
        <SemaineAffichee cible=cible montre_professeur=Signal::derive(|| true)/>
        <p class="texte-attenue edts-aide">
            <a href=navigation::CHANGEMENTS_EDT>"Voir les changements à venir"</a>
        </p>
    }
}

#[component]
fn VueEleve(moi: Moi) -> impl IntoView {
    match moi.eleve_id {
        Some(eleve) => view! {
            <SemaineAffichee
                cible=Signal::derive(move || Some(CibleSemaine::Eleve(eleve)))
                montre_professeur=Signal::derive(|| true)
            />
        <p class="texte-attenue edts-aide">
            <a href=navigation::CHANGEMENTS_EDT>"Voir les changements à venir"</a>
        </p>
        }
        .into_any(),
        None => view! {
            <EtatVide
                titre="Compte non lié à une fiche élève"
                message="Votre emploi du temps ne peut pas être affiché. Contactez le secrétariat."
            />
        }
        .into_any(),
    }
}

// ---------------------------------------------------------------------------
// Semaine
// ---------------------------------------------------------------------------

type Donnees = Result<Option<SemaineEdtDto>, ApiError>;

#[component]
fn SemaineAffichee(
    cible: Signal<Option<CibleSemaine>>,
    montre_professeur: Signal<bool>,
) -> impl IntoView {
    // La date du jour n'est posée qu'après l'hydratation (jamais dans le HTML du serveur).
    let lundi = RwSignal::new(None::<NaiveDate>);
    Effect::new(move |_| lundi.set(Some(lundi_de(aujourdhui()))));
    let donnees: LocalResource<Donnees> = LocalResource::new(move || {
        let c = cible.get();
        let l = lundi.get();
        async move {
            match (c, l) {
                (Some(c), Some(l)) => session::client().edt_semaine(&c, l).await.map(Some),
                _ => Ok(None),
            }
        }
    });
    let decaler = move |decalage: i64| {
        lundi.update(|l| {
            if let Some(nouveau) = l.and_then(|x| semaine_voisine(x, decalage)) {
                *l = Some(nouveau);
            }
        });
    };
    let titre_semaine = Signal::derive(move || {
        lundi
            .get()
            .map(libelle_semaine)
            .unwrap_or_else(|| "Semaine".to_string())
    });
    let annonce = Signal::derive(move || match (donnees.get(), lundi.get()) {
        (Some(Ok(Some(s))), Some(l)) => {
            let n = nb_changements(&s);
            let changements = match n {
                0 => "aucun changement".to_string(),
                1 => "1 changement".to_string(),
                n => format!("{n} changements"),
            };
            format!("{} : {changements}.", libelle_semaine(l))
        }
        _ => String::new(),
    });
    view! {
        <Annonce message=annonce/>
        <nav class="edts-navigation" aria-label="Changer de semaine">
            <Bouton
                variante=Variante::Secondaire
                taille=Taille::Compacte
                libelle_accessible="Semaine précédente"
                on_click=Callback::new(move |_| decaler(-1))
            >
                <Icone nom=NomIcone::ChevronGauche petite=true/>
            </Bouton>
            <Bouton
                variante=Variante::Secondaire
                taille=Taille::Compacte
                on_click=Callback::new(move |_| lundi.set(Some(lundi_de(aujourdhui()))))
            >
                "Aujourd'hui"
            </Bouton>
            <Bouton
                variante=Variante::Secondaire
                taille=Taille::Compacte
                libelle_accessible="Semaine suivante"
                on_click=Callback::new(move |_| decaler(1))
            >
                <Icone nom=NomIcone::ChevronDroite petite=true/>
            </Bouton>
            <h2 class="edts-semaine-titre">{move || titre_semaine.get()}</h2>
            {move || {
                donnees
                    .get()
                    .and_then(Result::ok)
                    .flatten()
                    .and_then(|s| libelle_semaine_ab(s.semaine_ab.as_deref()))
                    .map(|l| view! { <Badge libelle=l ton=TonEtat::Info/> })
            }}
        </nav>
        {move || match donnees.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement de la semaine…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| donnees.refetch())/> }.into_any(),
            Some(Ok(None)) if cible.get().is_none() => view! {
                <EtatVide titre="Faites un choix" message="Choisissez ce dont vous voulez voir l'emploi du temps."/>
            }
            .into_any(),
            Some(Ok(None)) => view! { <Squelette lignes=5 libelle="Chargement de la semaine…"/> }.into_any(),
            Some(Ok(Some(s))) => {
                let jours = regrouper_par_jour(&s, montre_professeur.get_untracked());
                view! { <JoursDeLaSemaine jours=jours/> }.into_any()
            }
        }}
    }
}

#[component]
fn JoursDeLaSemaine(jours: Vec<JourAffiche>) -> impl IntoView {
    let auj = aujourdhui();
    view! {
        <div class="edts-semaine">
            {jours
                .into_iter()
                .map(|j| {
                    let id = format!("edts-jour-{}", j.jour);
                    let titre_id = id.clone();
                    let est_auj = j.jour == auj;
                    let nb = j.nb_cours();
                    let resume = match (&j.ferme, nb) {
                        (Some(_), _) => String::new(),
                        (None, 0) if j.seances.is_empty() => "Aucun cours".to_string(),
                        (None, 0) => "Aucun cours n'a lieu".to_string(),
                        (None, 1) => "1 cours".to_string(),
                        (None, n) => format!("{n} cours"),
                    };
                    view! {
                        <section class="edts-jour" class:edts-jour--aujourdhui=est_auj aria-labelledby=titre_id>
                            <h3 id=id class="edts-jour__titre">
                                {j.titre.clone()}
                                {est_auj.then(|| view! { <Badge libelle="Aujourd'hui" ton=TonEtat::Info/> })}
                            </h3>
                            {j.ferme.clone().map(|f| view! {
                                <p class="edts-jour__ferme">
                                    <Badge libelle="Fermé" ton=TonEtat::Avertissement/>
                                    " "
                                    {if f.is_empty() { "Établissement fermé".to_string() } else { f }}
                                </p>
                            })}
                            {(!resume.is_empty()).then(|| view! { <p class="edts-jour__resume">{resume}</p> })}
                            {(!j.seances.is_empty()).then(|| view! {
                                <ul class="edts-seances">
                                    {j.seances
                                        .iter()
                                        .map(|s| {
                                            let annule = s.statut.n_a_pas_lieu();
                                            view! {
                                                <li class="edts-seance" class:edts-seance--annulee=annule>
                                                    <p class="edts-seance__heure">{s.horaire.clone()}</p>
                                                    <p class="edts-seance__titre">{s.matiere.clone()}</p>
                                                    <p class="edts-seance__detail">{s.detail.clone()}</p>
                                                    {s.mention.clone().map(|m| view! {
                                                        <p class="edts-seance__mention"><Badge libelle=m ton=s.statut.ton()/></p>
                                                    })}
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            })}
                        </section>
                    }
                })
                .collect_view()}
        </div>
    }
}
