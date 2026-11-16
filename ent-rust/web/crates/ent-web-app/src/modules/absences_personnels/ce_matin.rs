//! « Ce matin » et « demain » (ABP-165 à 169, 175, 185) : pour l'adjoint, la direction et la vie scolaire,
//! qui manque, quelles séances attendent une réponse, quels remplaçants proposer **avec leurs raisons**,
//! solliciter, désigner en urgence, lancer un appel à volontaires, prendre en charge les élèves, ou
//! appliquer un plan de réponses en lot.
//!
//! Niveau « organisation » : aucun DTO de cet écran ne porte la nature ni le motif d'une absence (R-AP3).
//! Une absence signalée mais non confirmée n'est jamais annoncée à l'extérieur : l'écran la marque.
//! Le serveur reste juge des droits (`edt.gerer` + lecture de l'organisation) : les boutons d'action sont
//! masqués sans `edt.gerer`, et un refus (403) est affiché.

use std::collections::HashSet;

use chrono::{Duration, NaiveDate};
use ent_types::absences_personnels::{ImpactTableauDto, PropositionDto, TableauJourDto};
use ent_web_ui::{
    Alerte, Badge, Bouton, Case, ChampTexte, Dialogue, EtatVide, Niveau, OptionSegment, Segmente,
    Squelette, Statistique, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use super::commun::GardePersonnel;
use super::modele::{
    a_couvrir, arret_sur_erreur, composer_plan, compteurs_non_nuls, impacts_planifiables,
    jour_dans_fenetre, libelle_etat_impact, libelle_seance, lignes_applicables,
    lignes_avec_urgence, nom_agent, nom_proposition, raisons_proposition, reponse_attendue,
    reponse_levable, ton_etat_impact, LignePlan,
};
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{message_echec, Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};

#[component]
pub fn PageCeMatin() -> impl IntoView {
    view! {
        <Title text="Ce matin"/>
        <GardePersonnel
            titre="Remplacements du jour"
            sous_titre="Qui manque, quelles séances attendent une réponse, qui peut remplacer."
        >
            <Contenu/>
        </GardePersonnel>
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let choix = RwSignal::new("aujourdhui".to_string());
    let autre = RwSignal::new(String::new());
    // Ce contenu n'est jamais rendu côté serveur (la garde attend la session, chargée par le navigateur) : le
    // jour du navigateur peut être lu à la création, aucune date n'entre dans le HTML du serveur. (Un `Effect`
    // posant le jour après coup laissait la première requête partir sans jour, sans jamais être relancée.)
    let reference = RwSignal::new(Some(aujourdhui()));
    let jour = Memo::new(move |_| {
        let r = reference.get()?;
        match choix.get().as_str() {
            "demain" => Some(r + Duration::days(1)),
            "date" => NaiveDate::parse_from_str(autre.get().trim(), "%Y-%m-%d").ok(),
            _ => Some(r),
        }
    });
    let version = RwSignal::new(0u32);
    // Le bilan du plan survit au rechargement du tableau.
    let bilan_plan = RwSignal::new(None::<BilanPlan>);
    // Le serveur refuse un jour hors de la veille à +14 jours : on le dit sans l'appeler.
    let hors_fenetre = Memo::new(move |_| match (jour.get(), reference.get()) {
        (Some(j), Some(r)) => !jour_dans_fenetre(j, r),
        _ => false,
    });
    let tableau = LocalResource::new(move || {
        let _ = version.get();
        let j = jour.get().filter(|_| !hors_fenetre.get_untracked());
        async move {
            match j {
                Some(j) => session::client().absp_tableau(j).await.map(Some),
                None => Ok(None),
            }
        }
    });
    view! {
        <div class="absp-barre">
            <Segmente
                nom="absp-jour"
                legende="Jour"
                valeur=choix
                options=vec![
                    OptionSegment::new("aujourdhui", "Aujourd'hui"),
                    OptionSegment::new("demain", "Demain"),
                    OptionSegment::new("date", "Autre date"),
                ]
            />
            {move || (choix.get() == "date").then(|| view! {
                <ChampTexte id="absp-jour-date" libelle="Date" genre="date" valeur=autre/>
            })}
            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| version.update(|v| *v += 1))>
                "Actualiser"
            </Bouton>
        </div>
        {move || bilan_plan.get().map(|b| view! { <BilanPlanVue b=b/> })}
        {move || match tableau.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement du tableau…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| tableau.refetch())/> }.into_any(),
            Some(Ok(None)) if hors_fenetre.get() => view! {
                <EtatVide
                    titre="Jour hors de la fenêtre"
                    message="Le tableau couvre la veille, aujourd'hui et les 14 jours suivants. Pour une absence plus lointaine, ouvrez sa fiche."
                />
            }
            .into_any(),
            Some(Ok(None)) => view! { <EtatVide titre="Date invalide" message="Choisissez un jour valide."/> }.into_any(),
            Some(Ok(Some(t))) => view! { <Tableau t=t version=version bilan=bilan_plan/> }.into_any(),
        }}
    }
}

/// Cible du panneau « répondre à cette séance ».
type Cible = Option<ImpactTableauDto>;

#[component]
fn Tableau(
    t: TableauJourDto,
    version: RwSignal<u32>,
    bilan: RwSignal<Option<BilanPlan>>,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let cible = RwSignal::new(None::<ImpactTableauDto>);
    let ouvert_panneau = RwSignal::new(false);
    let ouvert_plan = RwSignal::new(false);
    let selection = RwSignal::new(HashSet::<Uuid>::new());

    let jour = t.jour;
    let couvrir = a_couvrir(&t);
    let ton_couvrir = if couvrir > 0 {
        TonEtat::Mal
    } else {
        TonEtat::Ok
    };
    let compteurs = compteurs_non_nuls(&t);
    let absents = t.absents.clone();
    let planifiables: Vec<(Uuid, String, chrono::NaiveTime)> = impacts_planifiables(&t)
        .into_iter()
        .map(|i| {
            let h = i
                .impact
                .seance
                .as_ref()
                .map(|s| s.heure_debut)
                .unwrap_or_default();
            (i.impact.id, libelle_seance(&i.impact), h)
        })
        .collect();
    let planifiables = StoredValue::new(planifiables);
    let lignes = t.lignes.clone();
    let urgences: Vec<usize> = lignes_avec_urgence(&t).iter().map(|(_, n)| *n).collect();

    let ouvrir_reponse = Callback::new(move |i: ImpactTableauDto| {
        cible.set(Some(i));
        ouvert_panneau.set(true);
    });
    let lever = Callback::new(move |id: Uuid| {
        ecriture.lancer(async move {
            session::client().absp_lever(id).await?;
            Ok("Réponse levée : la séance est de nouveau à traiter.".to_string())
        });
    });
    let peut_agir = move || moteur.get().peut_gerer_edt();

    view! {
        <RetourEcriture ecriture=ecriture/>
        <p class="absp-jour" role="status">{date_longue(jour)}</p>
        <div class="absp-stats">
            <Statistique libelle="Séances à couvrir" valeur=Signal::derive(move || couvrir.to_string()) ton=ton_couvrir/>
            <Statistique libelle="Absents connus" valeur=Signal::derive({ let n = absents.len(); move || n.to_string() })/>
        </div>
        {(!compteurs.is_empty()).then(|| view! {
            <ul class="absp-compteurs" aria-label="Séances par état">
                {compteurs.iter().map(|(e, l, n)| view! {
                    <li><Badge libelle=format!("{l} : {n}") ton=ton_etat_impact(e)/></li>
                }).collect_view()}
            </ul>
        })}

        <section class="absp-section" aria-labelledby="absp-absents">
            <h2 id="absp-absents" class="vs-section-titre">"Absents"</h2>
            {if absents.is_empty() {
                view! { <p class="texte-attenue">"Aucun absent connu ce jour."</p> }.into_any()
            } else {
                view! {
                    <ul class="absp-liste absp-liste--serree">
                        {absents.iter().map(|a| view! {
                            <li class="absp-ligne">
                                <span>{nom_agent(&a.agent_label)}</span>
                                {(!a.confirmee).then(|| view! {
                                    <span class="texte-attenue">"Signalée, pas encore confirmée : à ne pas annoncer"</span>
                                })}
                            </li>
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            }}
            <p class="texte-attenue">"Vue d'organisation : ni le motif ni la nature des absences n'apparaissent ici."</p>
        </section>

        {move || peut_agir().then(|| view! {
            <p class="absp-lot" role="group" aria-label="Plan de remplacements">
                <Bouton
                    variante=Variante::Secondaire
                    desactive=Signal::derive(move || selection.get().is_empty())
                    on_click=Callback::new(move |_| ouvert_plan.set(true))
                >
                    {move || format!("Préparer un plan pour la sélection ({})", selection.get().len())}
                </Bouton>
                <span class="texte-attenue">"Cochez des séances, vérifiez l'aperçu, puis appliquez : une demande par séance."</span>
            </p>
        })}

        <section class="absp-section" aria-labelledby="absp-heures">
            <h2 id="absp-heures" class="vs-section-titre">"Heure par heure"</h2>
            {if lignes.is_empty() {
                view! { <EtatVide titre="Aucune séance touchée" message="Aucune absence ne touche de séance ce jour."/> }.into_any()
            } else {
                view! {
                    <div>
                        {lignes.into_iter().zip(urgences).map(|(l, n)| {
                            let titre_id = format!("absp-h-{}", l.heure_debut.format("%H%M"));
                            let reference = titre_id.clone();
                            view! {
                                <section class="absp-heure" aria-labelledby=reference>
                                    <h3 id=titre_id class="absp-sous-titre">
                                        {l.heure_debut.format("%H:%M").to_string()}
                                        {(n > 0).then(|| view! { <span class="texte-attenue">{format!(" · {n} à couvrir")}</span> })}
                                    </h3>
                                    <ul class="absp-liste">
                                        {l.impacts.into_iter().map(|i| view! {
                                            <CarteImpact
                                                i=i
                                                selection=selection
                                                peut_agir=Signal::derive(peut_agir)
                                                ouvrir=ouvrir_reponse
                                                lever=lever
                                            />
                                        }).collect_view()}
                                    </ul>
                                </section>
                            }
                        }).collect_view()}
                    </div>
                }
                .into_any()
            }}
        </section>

        <PanneauReponse cible=cible ouvert=ouvert_panneau ecriture=ecriture/>
        <PanneauPlan
            ouvert=ouvert_plan
            selection=selection
            planifiables=planifiables
            version=version
            bilan=bilan
        />
    }
}

#[component]
fn CarteImpact(
    i: ImpactTableauDto,
    selection: RwSignal<HashSet<Uuid>>,
    #[prop(into)] peut_agir: Signal<bool>,
    ouvrir: Callback<ImpactTableauDto>,
    lever: Callback<Uuid>,
) -> impl IntoView {
    let id = i.impact.id;
    let etat = i.impact.etat.clone();
    let attend = reponse_attendue(&etat);
    let levable = reponse_levable(&etat);
    let seance = libelle_seance(&i.impact);
    let agent = nom_agent(&i.agent_label);
    let coche = RwSignal::new(selection.get_untracked().contains(&id));
    Effect::new(move |_| {
        let c = coche.get();
        selection.update(|s| {
            if c {
                s.insert(id);
            } else {
                s.remove(&id);
            }
        });
    });
    Effect::new(move |_| {
        let dedans = selection.with(|s| s.contains(&id));
        if coche.get_untracked() != dedans {
            coche.set(dedans);
        }
    });
    let pour_ouvrir = i.clone();
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{seance.clone()}</strong>
                <Badge libelle=libelle_etat_impact(&etat) ton=ton_etat_impact(&etat)/>
            </div>
            <p class="absp-carte__detail">{format!("Absent : {agent}")}</p>
            {i.impact.a_revoir.then(|| view! {
                <p class="absp-carte__detail">"À revoir : l'emploi du temps a changé depuis la réponse."</p>
            })}
            {move || peut_agir.get().then(|| {
                let pour_ouvrir = pour_ouvrir.clone();
                let seance_a = seance.clone();
                view! {
                    <p class="absp-carte__actions">
                        {attend.then(|| view! {
                            <Case id=format!("absp-plan-{id}") libelle=format!("Inclure dans le plan : {seance_a}") coche=coche/>
                        })}
                        {attend.then(|| {
                            let p = pour_ouvrir.clone();
                            view! {
                                <Bouton
                                    taille=Taille::Compacte
                                    libelle_accessible=format!("Répondre à la séance : {seance_a}")
                                    on_click=Callback::new(move |_| ouvrir.run(p.clone()))
                                >
                                    "Répondre"
                                </Bouton>
                            }
                        })}
                        {levable.then(|| view! {
                            <Bouton
                                variante=Variante::Secondaire
                                taille=Taille::Compacte
                                libelle_accessible=format!("Lever la réponse : {seance_a}")
                                on_click=Callback::new(move |_| lever.run(id))
                            >
                                "Lever la réponse"
                            </Bouton>
                        })}
                    </p>
                }
            })}
        </li>
    }
}

// ───────────────────────────── Répondre à une séance ─────────────────────────────

#[component]
fn PanneauReponse(
    cible: RwSignal<Cible>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let urgence = RwSignal::new(false);
    let sortie = RwSignal::new(false);
    let volontaires = RwSignal::new(HashSet::<Uuid>::new());
    // Les propositions ne sont chargées que panneau ouvert, pour la séance choisie.
    let propositions = LocalResource::new(move || {
        let ouvert_maintenant = ouvert.get();
        let c = cible.get();
        async move {
            match (ouvert_maintenant, c) {
                (true, Some(i)) => session::client()
                    .absp_propositions(i.impact.id)
                    .await
                    .map(Some),
                _ => Ok(None),
            }
        }
    });
    Effect::new(move |_| {
        if !ouvert.get() {
            urgence.set(false);
            sortie.set(false);
            volontaires.set(HashSet::new());
        }
    });
    let impact_id = move || cible.get_untracked().map(|i| i.impact.id);
    let solliciter = Callback::new(move |agent: Uuid| {
        let Some(id) = impact_id() else { return };
        let designer = urgence.get_untracked();
        ouvert.set(false);
        ecriture.lancer(async move {
            let c = session::client();
            if designer {
                c.absp_designer(id, agent).await?;
                Ok("Remplaçant désigné : il doit accuser réception.".to_string())
            } else {
                c.absp_solliciter(id, agent).await?;
                Ok("Demande envoyée : réponse attendue.".to_string())
            }
        });
    });
    let lancer_appel = move |_| {
        let Some(id) = impact_id() else { return };
        let agents: Vec<Uuid> = volontaires.get_untracked().into_iter().collect();
        if agents.is_empty() {
            return;
        }
        ouvert.set(false);
        ecriture.lancer(async move {
            let n = session::client()
                .absp_appel_volontaires(id, agents)
                .await?
                .len();
            Ok(format!(
                "Appel envoyé à {n} volontaire(s) : le premier qui accepte est retenu."
            ))
        });
    };
    let annuler_seance = move |_| {
        let Some(id) = impact_id() else { return };
        let s = sortie.get_untracked();
        ouvert.set(false);
        ecriture.lancer(async move {
            session::client().absp_annuler_seance(id, s).await?;
            Ok("Séance annulée : les élèves sont pris en charge.".to_string())
        });
    };
    let sans_objet = move |_| {
        let Some(id) = impact_id() else { return };
        ouvert.set(false);
        ecriture.lancer(async move {
            session::client().absp_sans_objet(id).await?;
            Ok("Séance marquée sans objet.".to_string())
        });
    };

    view! {
        <Dialogue ouvert=ouvert titre="Répondre à cette séance" id="absp-dlg-reponse">
            <p class="absp-carte__detail">
                {move || cible.get().map(|i| libelle_seance(&i.impact)).unwrap_or_default()}
            </p>
            <h3 class="absp-sous-titre">"Remplaçants possibles"</h3>
            <p class="texte-attenue">
                "Classés selon des critères affichés : même discipline, charge du jour, remplacements déjà faits. Aucune donnée de santé, d'âge ni d'ancienneté n'entre dans le classement."
            </p>
            <p class="texte-attenue">
                "Le remplacement n'est appliqué à l'emploi du temps que lorsque l'absence est validée."
            </p>
            <Case id="absp-urgence" libelle="Urgence : désigner sans demander (la personne devra accuser réception)" coche=urgence/>
            {move || match propositions.get() {
                None => view! { <Squelette lignes=3 libelle="Recherche des remplaçants libres…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <Alerte niveau=Niveau::Erreur titre=message_echec(e)/>
                }
                .into_any(),
                Some(Ok(None)) => ().into_any(),
                Some(Ok(Some(v))) if v.is_empty() => view! {
                    <p>"Aucun collègue n'est libre sur cette plage. Vous pouvez annuler la séance et prendre en charge les élèves."</p>
                }
                .into_any(),
                Some(Ok(Some(v))) => view! {
                    <ul class="absp-liste">
                        {v.into_iter().map(|p| view! {
                            <LigneProposition p=p urgence=urgence volontaires=volontaires agir=solliciter/>
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
            <p class="absp-actions">
                <Bouton
                    variante=Variante::Secondaire
                    desactive=Signal::derive(move || volontaires.get().is_empty())
                    on_click=Callback::new(move |()| lancer_appel(()))
                >
                    {move || format!("Lancer un appel à volontaires ({})", volontaires.get().len())}
                </Bouton>
            </p>
            <h3 class="absp-sous-titre">"Autres réponses"</h3>
            <Case id="absp-sortie" libelle="Autoriser la sortie anticipée des élèves (seulement si les familles ont été prévenues)" coche=sortie/>
            <p class="absp-actions">
                <Bouton variante=Variante::Danger on_click=Callback::new(move |()| annuler_seance(()))>
                    "Annuler la séance, élèves pris en charge"
                </Bouton>
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |()| sans_objet(()))>
                    "Marquer sans objet"
                </Bouton>
            </p>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Fermer"</Bouton>
            </div>
        </Dialogue>
    }
}

#[component]
fn LigneProposition(
    p: PropositionDto,
    urgence: RwSignal<bool>,
    volontaires: RwSignal<HashSet<Uuid>>,
    agir: Callback<Uuid>,
) -> impl IntoView {
    let id = p.agent_id;
    let nom = nom_proposition(&p);
    let raisons = raisons_proposition(&p);
    let coche = RwSignal::new(volontaires.get_untracked().contains(&id));
    Effect::new(move |_| {
        let c = coche.get();
        volontaires.update(|s| {
            if c {
                s.insert(id);
            } else {
                s.remove(&id);
            }
        });
    });
    Effect::new(move |_| {
        let dedans = volontaires.with(|s| s.contains(&id));
        if coche.get_untracked() != dedans {
            coche.set(dedans);
        }
    });
    let nom_bouton = nom.clone();
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{nom.clone()}</strong>
            </div>
            <ul class="absp-raisons" aria-label=format!("Pourquoi {nom} est proposé")>
                {raisons.into_iter().map(|r| view! { <li>{r}</li> }).collect_view()}
            </ul>
            <p class="absp-carte__actions">
                <Bouton
                    variante=Variante::Primaire
                    taille=Taille::Compacte
                    libelle_accessible=format!("Demander à {nom_bouton}")
                    on_click=Callback::new(move |_| agir.run(id))
                >
                    {move || if urgence.get() { "Désigner" } else { "Solliciter" }}
                </Bouton>
                <Case id=format!("absp-volontaire-{id}") libelle=format!("Appel à volontaires : {nom_bouton}") coche=coche/>
            </p>
        </li>
    }
}

// ───────────────────────────── Plan en lot ─────────────────────────────

#[derive(Clone)]
struct LigneApercu {
    ligne: LignePlan,
    retenu: RwSignal<bool>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct BilanPlan {
    envoyees: usize,
    echecs: Vec<(String, String)>,
    arret: Option<String>,
    restantes: usize,
}

#[component]
fn PanneauPlan(
    ouvert: RwSignal<bool>,
    selection: RwSignal<HashSet<Uuid>>,
    planifiables: StoredValue<Vec<(Uuid, String, chrono::NaiveTime)>>,
    version: RwSignal<u32>,
    bilan: RwSignal<Option<BilanPlan>>,
) -> impl IntoView {
    let apercu = RwSignal::new(None::<Vec<LigneApercu>>);
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    // Génération de l'aperçu : une boucle périmée n'écrit jamais son résultat.
    let generation = StoredValue::new(0u32);

    // À l'ouverture : une demande de propositions par séance cochée (arrêt sur 401, 403, 429).
    Effect::new(move |_| {
        generation.update_value(|g| *g += 1);
        let cette_generation = generation.get_value();
        apercu.set(None);
        erreur.set(None);
        if !ouvert.get() {
            return;
        }
        let choisis: Vec<(Uuid, String)> = planifiables.with_value(|p| {
            p.iter()
                .filter(|(i, _, _)| selection.with_untracked(|s| s.contains(i)))
                .map(|(i, l, _)| (*i, l.clone()))
                .collect()
        });
        bilan.set(None);
        spawn_local(async move {
            let client = session::client();
            let mut reponses: Vec<(Uuid, Vec<PropositionDto>)> = Vec::new();
            for (id, _) in &choisis {
                match client.absp_propositions(*id).await {
                    Ok(v) => reponses.push((*id, v)),
                    Err(e) => {
                        let stop = arret_sur_erreur(&e);
                        if stop {
                            if generation.get_value() == cette_generation {
                                erreur.set(Some(message_echec(e)));
                            }
                            return;
                        }
                        reponses.push((*id, Vec::new()));
                    }
                }
            }
            if generation.get_value() != cette_generation {
                return;
            }
            let plan = composer_plan(&choisis, &reponses);
            apercu.set(Some(
                plan.into_iter()
                    .map(|l| {
                        let retenu = RwSignal::new(l.candidat_id.is_some());
                        LigneApercu { ligne: l, retenu }
                    })
                    .collect(),
            ));
        });
    });

    let appliquer = move |_| {
        if en_cours.get_untracked() {
            return;
        }
        let Some(lignes) = apercu.get_untracked() else {
            return;
        };
        let retenues: Vec<LignePlan> = lignes
            .iter()
            .filter(|l| l.retenu.get_untracked())
            .map(|l| l.ligne.clone())
            .collect();
        let heures: Vec<(Uuid, chrono::NaiveTime)> =
            planifiables.with_value(|p| p.iter().map(|(i, _, h)| (*i, *h)).collect());
        let a_faire = lignes_applicables(&retenues, &heures);
        // Lignes retenues mais écartées (même remplaçant à la même heure) : dites dans le bilan.
        let ecartees: Vec<String> = retenues
            .iter()
            .filter(|r| !a_faire.iter().any(|a| a.impact_id == r.impact_id))
            .map(|r| r.seance.clone())
            .collect();
        en_cours.set(true);
        spawn_local(async move {
            let client = session::client();
            let mut b = BilanPlan::default();
            for s in ecartees {
                b.echecs.push((
                    s,
                    "le même remplaçant est déjà demandé à cette heure : à traiter à la main"
                        .to_string(),
                ));
            }
            let total = a_faire.len();
            for (rang, l) in a_faire.iter().enumerate() {
                let Some(agent) = l.candidat_id else { continue };
                match client.absp_solliciter(l.impact_id, agent).await {
                    Ok(_) => b.envoyees += 1,
                    Err(e) => {
                        let stop = arret_sur_erreur(&e);
                        let m = message_echec(e);
                        if stop {
                            b.arret = Some(m);
                            b.restantes = total - rang;
                            break;
                        }
                        b.echecs.push((l.seance.clone(), m));
                    }
                }
            }
            selection.set(HashSet::new());
            bilan.set(Some(b));
            en_cours.set(false);
            ouvert.set(false);
            version.update(|v| *v += 1);
        });
    };

    view! {
        <Dialogue ouvert=ouvert titre="Plan de remplacements" id="absp-dlg-plan">
            <p class="texte-attenue">
                "Pour chaque séance cochée, la première proposition du serveur. Décochez ce que vous ne voulez pas : rien n'est envoyé avant « Appliquer »."
            </p>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {move || match apercu.get() {
                None if erreur.get().is_none() => view! { <Squelette lignes=3 libelle="Préparation du plan…"/> }.into_any(),
                None => ().into_any(),
                Some(l) => view! {
                    <ul class="absp-liste">
                        {l.into_iter().map(|x| {
                            let sans = x.ligne.candidat_id.is_none();
                            let nom = x.ligne.candidat_nom.clone().unwrap_or_default();
                            let seance = x.ligne.seance.clone();
                            view! {
                                <li class="absp-carte">
                                    <strong class="absp-carte__titre">{seance.clone()}</strong>
                                    {if sans {
                                        view! { <p class="absp-carte__detail">"Aucun remplaçant libre : à traiter à la main."</p> }.into_any()
                                    } else {
                                        view! {
                                            <Case
                                                id=format!("absp-plan-ligne-{}", x.ligne.impact_id)
                                                libelle=format!("Demander à {nom}")
                                                coche=x.retenu
                                            />
                                        }
                                        .into_any()
                                    }}
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Fermer"</Bouton>
                <Bouton
                    desactive=Signal::derive(move || apercu.get().is_none())
                    en_cours=en_cours
                    on_click=Callback::new(move |()| appliquer(()))
                >
                    "Appliquer le plan"
                </Bouton>
            </div>
        </Dialogue>
    }
}

#[component]
fn BilanPlanVue(b: BilanPlan) -> impl IntoView {
    let niveau = if b.arret.is_some() || !b.echecs.is_empty() {
        Niveau::Avertissement
    } else {
        Niveau::Succes
    };
    view! {
        <Alerte niveau=niveau titre=format!("{} demande(s) envoyée(s).", b.envoyees)>
            {b.arret.clone().map(|m| view! { <p>{format!("Le plan s'est arrêté : {m} {} séance(s) n'ont pas été traitées.", b.restantes)}</p> })}
            {(!b.echecs.is_empty()).then(|| view! {
                <ul>{b.echecs.iter().map(|(s, m)| view! { <li>{format!("{s} : {m}")}</li> }).collect_view()}</ul>
            })}
        </Alerte>
    }
}
