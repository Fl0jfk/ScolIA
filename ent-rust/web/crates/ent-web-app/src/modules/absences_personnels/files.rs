//! Files de traitement des absences (ABP-170, 171, 174, 147, 124) : « à valider » (valideur), « à traiter »
//! (instructeur : secrétariat, comptabilité), compteurs, échéances (arrêt sous 48 h, pièces attendues) et
//! validation **en lot** des seules natures non sensibles (convocations, formations).
//!
//! Le serveur compose chaque file selon le circuit de la personne connectée : une file vide n'est pas une
//! erreur. Un lot n'est jamais une écriture groupée : une absence après l'autre, arrêt sur 401, 403 ou 429,
//! échecs listés (FRONTEND_LEPTOS §5).

use std::collections::HashSet;

use ent_types::absences_personnels::{AbsenceDto, DecisionBody};
use ent_web_api::ActionDecision;
use ent_web_ui::{
    Alerte, Badge, Bouton, Case, ChampTexte, EtatVide, Niveau, OptionSegment, Segmente, Squelette,
    Statistique, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use super::commun::GardePersonnel;
use super::modele::{
    arret_sur_erreur, eligible_lot, filtrer_file, libelle_echeance, libelle_etat, libelle_periode,
    nom_agent, ton_etat, trier_file,
};
use crate::composants_metier::confirmation_detaillee::ConfirmationDetaillee;
use crate::noyau::ecriture::message_echec;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;

#[component]
pub fn PageFilesAbsences() -> impl IntoView {
    view! {
        <Title text="Files des absences"/>
        <GardePersonnel
            titre="Absences à instruire"
            sous_titre="Les absences qui attendent votre décision ou votre traitement."
        >
            <Contenu/>
        </GardePersonnel>
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let file = RwSignal::new("a_valider".to_string());
    let recherche = RwSignal::new(String::new());
    let version = RwSignal::new(0u32);
    // Le bilan d'un lot survit au rechargement de la liste.
    let bilan = RwSignal::new(None::<BilanLot>);
    let compteurs = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_compteurs().await }
    });
    view! {
        {move || compteurs.get().and_then(Result::ok).map(|c| {
            let ton_valider = if c.a_valider > 0 { TonEtat::Avertissement } else { TonEtat::Ok };
            let ton_traiter = if c.a_traiter > 0 { TonEtat::Avertissement } else { TonEtat::Ok };
            let ton_couvrir = if c.seances_a_couvrir_aujourdhui > 0 { TonEtat::Mal } else { TonEtat::Ok };
            view! {
                <div class="absp-stats">
                    <Statistique libelle="À valider" valeur=Signal::derive(move || c.a_valider.to_string()) ton=ton_valider/>
                    <Statistique libelle="À traiter" valeur=Signal::derive(move || c.a_traiter.to_string()) ton=ton_traiter/>
                    <Statistique
                        libelle="Séances à couvrir aujourd'hui"
                        valeur=Signal::derive(move || c.seances_a_couvrir_aujourdhui.to_string())
                        ton=ton_couvrir
                    />
                </div>
            }
        })}
        <Segmente
            nom="absp-file"
            legende="File"
            valeur=file
            options=vec![
                OptionSegment::new("a_valider", "À valider"),
                OptionSegment::new("a_traiter", "À traiter"),
                OptionSegment::new("echeances", "Échéances"),
            ]
        />
        {move || bilan.get().map(|b| view! { <BilanLotVue b=b/> })}
        {move || match file.get().as_str() {
            "echeances" => view! { <Echeances/> }.into_any(),
            "a_traiter" => view! { <File traiter=true recherche=recherche version=version bilan=bilan/> }.into_any(),
            _ => view! { <File traiter=false recherche=recherche version=version bilan=bilan/> }.into_any(),
        }}
    }
}

/// Bilan d'un lot : validées, échecs (nom, message), arrêt éventuel.
#[derive(Clone, Debug, Default)]
pub(super) struct BilanLot {
    validees: usize,
    echecs: Vec<(String, String)>,
    arret: Option<String>,
    restantes: usize,
}

#[component]
fn File(
    traiter: bool,
    recherche: RwSignal<String>,
    version: RwSignal<u32>,
    bilan: RwSignal<Option<BilanLot>>,
) -> impl IntoView {
    let liste = LocalResource::new(move || {
        let _ = version.get();
        async move {
            if traiter {
                session::client().absp_a_traiter().await
            } else {
                session::client().absp_a_valider().await
            }
        }
    });
    view! {
        {move || match liste.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de la file…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            Some(Ok(v)) if v.is_empty() => view! {
                <EtatVide
                    titre=if traiter { "Rien à traiter" } else { "Rien à valider" }
                    message="Les nouvelles absences apparaîtront ici dès qu'elles seront déclarées."
                />
            }
            .into_any(),
            Some(Ok(v)) => view! { <ListeFile v=v traiter=traiter recherche=recherche version=version bilan=bilan/> }.into_any(),
        }}
    }
}

#[component]
fn ListeFile(
    v: Vec<AbsenceDto>,
    traiter: bool,
    recherche: RwSignal<String>,
    version: RwSignal<u32>,
    bilan: RwSignal<Option<BilanLot>>,
) -> impl IntoView {
    let toutes = StoredValue::new(trier_file(v));
    let selection = RwSignal::new(HashSet::<Uuid>::new());
    let ouvert = RwSignal::new(false);
    let en_cours = RwSignal::new(false);

    let visibles = Memo::new(move |_| toutes.with_value(|t| filtrer_file(t, &recherche.get())));
    // Seules les lignes sélectionnées ET affichées sont envoyées (FRONTEND_LEPTOS §6) ; on compte le reste.
    let choisies = Memo::new(move |_| {
        let sel = selection.get();
        visibles
            .get()
            .into_iter()
            .filter(|a| sel.contains(&a.id) && eligible_lot(a))
            .collect::<Vec<_>>()
    });
    let masquees = Memo::new(move |_| {
        let affichees: HashSet<Uuid> = visibles.get().iter().map(|a| a.id).collect();
        selection
            .get()
            .iter()
            .filter(|i| !affichees.contains(i))
            .count()
    });
    let eligibles = Memo::new(move |_| visibles.get().iter().filter(|a| eligible_lot(a)).count());

    let lancer = Callback::new(move |_| {
        if en_cours.get_untracked() {
            return;
        }
        let lot = choisies.get_untracked();
        bilan.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let mut b = BilanLot::default();
            let total = lot.len();
            let client = session::client();
            for (rang, a) in lot.iter().enumerate() {
                let corps = DecisionBody {
                    traitement: Some("sans_incidence".into()),
                    ..Default::default()
                };
                match client
                    .absp_decider(a.id, ActionDecision::Valider, &corps)
                    .await
                {
                    Ok(_) => b.validees += 1,
                    Err(e) => {
                        let stop = arret_sur_erreur(&e);
                        let message = message_echec(e);
                        if stop {
                            b.arret = Some(message);
                            b.restantes = total - rang;
                            break;
                        }
                        b.echecs.push((nom_agent(&a.agent_label), message));
                    }
                }
            }
            selection.set(HashSet::new());
            bilan.set(Some(b));
            en_cours.set(false);
            version.update(|x| *x += 1);
        });
    });

    view! {
        <div class="absp-barre">
            <ChampTexte id="absp-recherche" libelle="Filtrer par nom" genre="search" valeur=recherche/>
        </div>
        {(!traiter).then(|| view! {
            <p class="absp-lot" role="group" aria-label="Validation en lot">
                <Bouton
                    variante=Variante::Secondaire
                    desactive=Signal::derive(move || choisies.get().is_empty() || en_cours.get())
                    en_cours=en_cours
                    on_click=Callback::new(move |_| ouvert.set(true))
                >
                    {move || format!("Valider la sélection ({})", choisies.get().len())}
                </Bouton>
                <span class="texte-attenue" role="status">
                    {move || {
                        let m = masquees.get();
                        let base = format!("{} absence(s) peuvent être validées en lot (convocations et formations).", eligibles.get());
                        if m > 0 { format!("{base} {m} sélection(s) masquée(s) par le filtre : elles ne sont pas envoyées.") } else { base }
                    }}
                </span>
            </p>
        })}
        <ul class="absp-liste">
            <For
                each=move || visibles.get()
                key=|a| (a.id, a.etat.clone())
                children=move |a| view! { <LigneFile a=a selection=selection lot=!traiter/> }
            />
        </ul>
        {move || visibles.get().is_empty().then(|| view! {
            <p class="texte-attenue" role="status">"Aucune absence ne correspond à ce nom."</p>
        })}
        <ConfirmationDetaillee
            ouvert=ouvert
            id="absp-dlg-lot"
            titre="Valider les absences sélectionnées ?"
            message=Signal::derive(move || {
                format!(
                    "{} absence(s) de nature non sensible (convocation, formation) seront validées sans incidence sur les heures, une par une.",
                    choisies.get().len()
                )
            })
            rappel=Signal::derive(|| "Chaque personne est prévenue. Une absence que le serveur refuse reste dans la file, avec son motif de refus.".to_string())
            libelle_confirmer="Valider"
            on_confirmer=lancer
        />
    }
}

#[component]
fn LigneFile(a: AbsenceDto, selection: RwSignal<HashSet<Uuid>>, lot: bool) -> impl IntoView {
    let id = a.id;
    let eligible = lot && eligible_lot(&a);
    let periode = libelle_periode(a.debut, a.fin);
    let nom = nom_agent(&a.agent_label);
    let href = format!("/web/personnel/absences/{id}");
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
    // Un lot vidé (après envoi) décoche les cases.
    Effect::new(move |_| {
        let dedans = selection.with(|s| s.contains(&id));
        if coche.get_untracked() != dedans {
            coche.set(dedans);
        }
    });
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{nom.clone()}</strong>
                <Badge libelle=libelle_etat(&a.etat) ton=ton_etat(&a.etat)/>
            </div>
            <p class="absp-carte__detail">{periode.clone()}</p>
            {a.nature_libelle.clone().map(|n| view! { <p class="absp-carte__detail">{n}</p> })}
            <p class="absp-carte__actions">
                {eligible.then(|| view! {
                    <Case id=format!("absp-lot-{id}") libelle=format!("Inclure {nom} dans le lot") coche=coche/>
                })}
                <a class="absp-lien" href=href aria-label=format!("Ouvrir l'absence de {} : {periode}", nom_agent(&a.agent_label))>
                    "Ouvrir"
                </a>
            </p>
        </li>
    }
}

#[component]
fn BilanLotVue(b: BilanLot) -> impl IntoView {
    let niveau = if b.arret.is_some() || !b.echecs.is_empty() {
        Niveau::Avertissement
    } else {
        Niveau::Succes
    };
    let titre = format!(
        "{} absence(s) validée(s){}.",
        b.validees,
        if b.echecs.is_empty() && b.arret.is_none() {
            String::new()
        } else {
            format!(
                ", {} échec(s)",
                b.echecs.len() + usize::from(b.arret.is_some())
            )
        }
    );
    view! {
        <Alerte niveau=niveau titre=titre>
            {b.arret.clone().map(|m| view! {
                <p>{format!("Le lot s'est arrêté : {m} {} absence(s) n'ont pas été traitées.", b.restantes)}</p>
            })}
            {(!b.echecs.is_empty()).then(|| view! {
                <ul>
                    {b.echecs.iter().map(|(n, m)| view! { <li>{format!("{n} : {m}")}</li> }).collect_view()}
                </ul>
            })}
        </Alerte>
    }
}

#[component]
fn Echeances() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().absp_echeances().await });
    view! {
        {move || match liste.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des échéances…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            Some(Ok(v)) if v.is_empty() => view! {
                <EtatVide titre="Aucune échéance" message="Aucun arrêt à recevoir ni pièce attendue pour le moment."/>
            }
            .into_any(),
            Some(Ok(mut v)) => {
                v.sort_by_key(|e| e.echeance_le);
                let maintenant = chrono::Utc::now();
                view! {
                    <ul class="absp-liste">
                        {v.into_iter().map(|e| {
                            let retard = e.echeance_le < maintenant;
                            view! {
                                <li class="absp-carte">
                                    <div class="absp-carte__tete">
                                        <strong class="absp-carte__titre">{libelle_echeance(&e.genre)}</strong>
                                        {retard.then(|| view! { <Badge libelle="Échéance dépassée" ton=TonEtat::Mal/> })}
                                    </div>
                                    <p class="absp-carte__detail">{format!("Avant le {}", date_heure_paris(e.echeance_le))}</p>
                                    {(e.relances > 0).then(|| view! { <p class="absp-carte__detail">{format!("{} relance(s) envoyée(s)", e.relances)}</p> })}
                                    <p class="absp-carte__actions">
                                        <a class="absp-lien" href=format!("/web/personnel/absences/{}", e.absence_id)>"Ouvrir l'absence"</a>
                                    </p>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }
                .into_any()
            }
        }}
    }
}
