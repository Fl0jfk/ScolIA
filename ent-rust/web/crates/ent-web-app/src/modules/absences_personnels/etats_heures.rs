//! Heures à valider et états mensuels (ABP-203 à 208, 214, 218, 219) : la direction, le secrétariat (agents de
//! l'État) et la comptabilité (OGEC) valident les heures d'un mois, règlent les contestations, **figent** l'état
//! mensuel, le font **signer** par le chef d'établissement, l'exportent, puis **régularisent** après export.
//!
//! Le serveur juge le droit par action **et** le périmètre d'employeur. Une heure figée ne se réécrit jamais :
//! l'écran ne propose que la régularisation (D-027 E). Aucune nature d'absence n'est lue ici.

use chrono::NaiveDate;
use ent_types::absences_personnels::{
    CorrigerHeureBody, EtatMensuelDto, FigerEtatBody, FiltreHeures, HeureDto, SaisirHeureBody,
    ValiderHeuresBody,
};
use ent_web_ui::{
    Alerte, Badge, Bouton, ChampTexte, Dialogue, EtatVide, Niveau, OptionSegment, Segmente,
    Selection, Squelette, Taille, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::commun::{charger_noms, lancer_export, GardePersonnel};
use super::modele_heures::{
    analyser_duree, derniers_mois, duree_effective, employeurs_offerts, etats_des_heures,
    libelle_duree, libelle_employeur, libelle_etat_heure, libelle_etat_mensuel, libelle_mois,
    libelle_nature_heure, libelle_preuve, mois_termine, nb_a_regler, nb_a_valider, nom_correspond,
    peut_contester, peut_corriger, peut_regulariser, phrase_bilan_validation, ton_etat_heure,
    ton_etat_mensuel, totaux_par_agent, totaux_par_nature, Noms, NATURES_SAISIE, RAISONS_HEURE,
};
use super::paie::{SectionDsn, SectionPaie};
use crate::composants_metier::confirmation_detaillee::ConfirmationDetaillee;
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};

#[component]
pub fn PageEtatsHeures() -> impl IntoView {
    view! {
        <Title text="Heures et états mensuels"/>
        <GardePersonnel
            titre="Heures et états mensuels"
            sous_titre="Valider les heures d'un mois, figer l'état, le faire signer, l'exporter, régulariser."
        >
            <Contenu/>
        </GardePersonnel>
    }
}

fn texte_mois(m: NaiveDate) -> String {
    m.format("%Y-%m-%d").to_string()
}

#[component]
fn Contenu() -> impl IntoView {
    let ctx = session::use_session();
    let role = ctx
        .moi
        .get_untracked()
        .and_then(Result::ok)
        .map(|m| m.role)
        .unwrap_or_default();
    let offerts = employeurs_offerts(&role);
    // Jour lu **avant** la création des ressources (FRONTEND_LEPTOS.md §14).
    let auj = aujourdhui();
    let mois_possibles = derniers_mois(auj, 12);
    let employeur = RwSignal::new(
        offerts
            .first()
            .map_or_else(|| "etat".to_string(), |e| (*e).to_string()),
    );
    let mois_choisi = RwSignal::new(texte_mois(mois_possibles.get(1).copied().unwrap_or(auj)));
    let section = RwSignal::new("heures".to_string());
    let recherche = RwSignal::new(String::new());
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let noms = charger_noms();
    let mois_actuel =
        move || NaiveDate::parse_from_str(&mois_choisi.get(), "%Y-%m-%d").unwrap_or(auj);
    let heures = LocalResource::new(move || {
        let _ = version.get();
        let filtre = FiltreHeures {
            mois: Some(mois_actuel()),
            ..Default::default()
        };
        async move { session::client().absp_heures(&filtre).await }
    });

    let options_employeur: Vec<(String, String)> = offerts
        .iter()
        .map(|e| ((*e).to_string(), libelle_employeur(e).to_string()))
        .collect();
    let options_mois: Vec<(String, String)> = mois_possibles
        .iter()
        .map(|m| (texte_mois(*m), libelle_mois(*m)))
        .collect();
    let ogec = move || employeur.get() == "ogec";
    let options_section = move || {
        let mut o = vec![
            OptionSegment::new("heures", "Heures du mois"),
            OptionSegment::new("etat", "État mensuel"),
        ];
        if ogec() {
            o.push(OptionSegment::new("paie", "Paie de l'OGEC"));
            o.push(OptionSegment::new("dsn", "DSN"));
        }
        o
    };
    view! {
        <RetourEcriture ecriture=ecriture/>
        {if offerts.is_empty() {
            view! {
                <Alerte
                    niveau=Niveau::Info
                    titre="Les états d'heures sont réservés à la direction, au secrétariat (agents de l'État) et à la comptabilité (salariés de l'OGEC). Vos propres heures sont dans « Mes heures »."
                />
            }
            .into_any()
        } else {
            view! {
                <div class="absp-barre">
                    <Selection id="absp-employeur" libelle="Personnels" valeur=employeur options=options_employeur/>
                    <Selection id="absp-mois" libelle="Mois" valeur=mois_choisi options=options_mois/>
                </div>
                <div class="absp-onglets">
                    {move || view! {
                        <Segmente nom="absp-section-etats" legende="Section" options=options_section() valeur=section/>
                    }}
                </div>
                {move || match section.get().as_str() {
                    "etat" => view! { <SectionEtat heures=heures employeur=employeur mois=Signal::derive(mois_actuel) noms=noms ecriture=ecriture auj=auj/> }.into_any(),
                    "paie" if ogec() => view! { <SectionPaie mois=Signal::derive(mois_actuel) noms=noms ecriture=ecriture/> }.into_any(),
                    "dsn" if ogec() => view! { <SectionDsn noms=noms ecriture=ecriture auj=auj/> }.into_any(),
                    _ => view! { <SectionHeures heures=heures employeur=employeur recherche=recherche mois=Signal::derive(mois_actuel) noms=noms ecriture=ecriture/> }.into_any(),
                }}
            }
            .into_any()
        }}
    }
}

// ───────────────────────────── Heures du mois ─────────────────────────────

#[component]
fn SectionHeures(
    heures: LocalResource<Result<Vec<HeureDto>, ent_web_api::ApiError>>,
    employeur: RwSignal<String>,
    recherche: RwSignal<String>,
    mois: Signal<NaiveDate>,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let peut_valider = move || moteur.get().a("personnels.heures.valider");
    let ouvert_valider = RwSignal::new(false);
    let ouvert_saisie = RwSignal::new(false);
    let a_corriger: RwSignal<Option<HeureDto>> = RwSignal::new(None);
    let ouvert_correction = RwSignal::new(false);
    let a_regulariser: RwSignal<Option<HeureDto>> = RwSignal::new(None);
    let ouvert_regularisation = RwSignal::new(false);
    let a_contester: RwSignal<Option<HeureDto>> = RwSignal::new(None);
    let ouvert_contestation = RwSignal::new(false);
    view! {
        {move || match heures.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des heures…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| heures.refetch())/> }.into_any(),
            Some(Ok(toutes)) => {
                let propres: Vec<HeureDto> = toutes.into_iter().filter(|h| h.employeur == employeur.get()).collect();
                let a_valider = nb_a_valider(&propres);
                let contestees = nb_a_regler(&propres);
                let totaux = totaux_par_nature(&propres);
                let n = propres.len();
                let liste = StoredValue::new(propres);
                view! {
                    <p class="absp-barre">
                        {peut_valider().then(|| view! {
                            <Bouton
                                libelle_accessible=format!("Valider les {a_valider} heures constatées du mois")
                                desactive=a_valider == 0
                                on_click=Callback::new(move |_| ouvert_valider.set(true))
                            >
                                {format!("Valider les heures constatées ({a_valider})")}
                            </Bouton>
                        })}
                        {peut_valider().then(|| view! {
                            <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert_saisie.set(true))>
                                "Saisir une heure sans séance"
                            </Bouton>
                        })}
                    </p>
                    <ul class="absp-compteurs" aria-label="Total par nature">
                        {totaux.into_iter().map(|(nat, m)| view! {
                            <li><Badge libelle=format!("{} : {}", libelle_nature_heure(nat), libelle_duree(m))/></li>
                        }).collect_view()}
                    </ul>
                    {(contestees > 0).then(|| view! {
                        <Alerte
                            niveau=Niveau::Avertissement
                            titre=format!("{contestees} heure(s) contestée(s) à régler avant de figer l'état : corrigez-les (la correction règle la contestation).")
                        />
                    })}
                    <div class="absp-barre">
                        <ChampTexte id="absp-heures-recherche" libelle="Filtrer par nom" valeur=recherche genre="search" autocomplete="off"/>
                    </div>
                    {if n == 0 {
                        view! {
                            <EtatVide
                                titre="Aucune heure ce mois-ci"
                                message="Les heures apparaissent quand une séance remplacée est assurée, ou quand une mission, une vacation ou une HSE est saisie."
                            />
                        }
                        .into_any()
                    } else {
                        view! {
                            <ul class="absp-liste">
                                {move || {
                                    let q = recherche.get();
                                    let nm = noms.get().unwrap_or_default();
                                    liste.get_value().into_iter()
                                        .filter(|h| nom_correspond(&nm.nom(h.agent_id), &q))
                                        .map(|h| {
                                            let nom = nm.nom(h.agent_id);
                                            view! {
                                                <LigneHeure h=h nom=nom peut_gerer=peut_valider()
                                                    corriger=a_corriger ouvert_corriger=ouvert_correction
                                                    regulariser=a_regulariser ouvert_regulariser=ouvert_regularisation
                                                    contester=a_contester ouvert_contester=ouvert_contestation/>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </ul>
                        }
                        .into_any()
                    }}
                    <ConfirmationDetaillee
                        ouvert=ouvert_valider
                        id="absp-dlg-valider-heures"
                        titre="Valider les heures du mois"
                        message=Signal::derive(move || format!(
                            "Valider les {a_valider} heures constatées de {} ?",
                            libelle_mois(mois.get())
                        ))
                        rappel=Signal::derive(|| "Vos propres heures sont laissées à une autre personne ; les heures contestées restent à régler.".to_string())
                        libelle_confirmer="Valider"
                        on_confirmer=Callback::new(move |_| {
                            let corps = ValiderHeuresBody { mois: mois.get_untracked(), agent_id: None };
                            ecriture.lancer(async move {
                                let b = session::client().absp_valider_heures(&corps).await?;
                                Ok(phrase_bilan_validation(&b))
                            });
                        })
                    />
                }
                .into_any()
            }
        }}
        <DialogueSaisie ouvert=ouvert_saisie noms=noms ecriture=ecriture/>
        <DialogueDuree regul=false choix=a_corriger ouvert=ouvert_correction ecriture=ecriture/>
        <DialogueDuree regul=true choix=a_regulariser ouvert=ouvert_regularisation ecriture=ecriture/>
        <DialogueContestationGestion choix=a_contester ouvert=ouvert_contestation ecriture=ecriture/>
    }
}

#[allow(clippy::too_many_arguments)]
#[component]
fn LigneHeure(
    h: HeureDto,
    nom: String,
    peut_gerer: bool,
    corriger: RwSignal<Option<HeureDto>>,
    ouvert_corriger: RwSignal<bool>,
    regulariser: RwSignal<Option<HeureDto>>,
    ouvert_regulariser: RwSignal<bool>,
    contester: RwSignal<Option<HeureDto>>,
    ouvert_contester: RwSignal<bool>,
) -> impl IntoView {
    let resume = format!(
        "{} · {} · {}",
        nom,
        date_longue(h.le),
        libelle_nature_heure(&h.nature)
    );
    let regul = (h.regularisation_minutes != 0).then(|| {
        format!(
            "dont régularisation : {}",
            libelle_duree(i64::from(h.regularisation_minutes))
        )
    });
    let (hc, hr, hx) = (h.clone(), h.clone(), h.clone());
    let (r1, r2, r3) = (resume.clone(), resume.clone(), resume.clone());
    let corrigeable = peut_corriger(&h.etat);
    let regularisable = peut_regulariser(&h.etat);
    let contestable = peut_contester(&h.etat);
    let libelle_correction = if h.etat == "contestee" {
        "Régler la contestation"
    } else {
        "Corriger"
    };
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{resume}</strong>
                <Badge libelle=libelle_etat_heure(&h.etat) ton=ton_etat_heure(&h.etat)/>
            </div>
            <p class="absp-carte__detail">
                {format!("{} · {}", libelle_duree(duree_effective(&h)), libelle_preuve(&h.preuve))}
            </p>
            {regul.map(|r| view! { <p class="absp-carte__detail">{r}</p> })}
            {peut_gerer.then(|| view! {
                <p class="absp-carte__actions">
                    {corrigeable.then(|| view! {
                        <Bouton taille=Taille::Compacte libelle_accessible=format!("Corriger l'heure : {r1}")
                            on_click=Callback::new(move |_| { corriger.set(Some(hc.clone())); ouvert_corriger.set(true); })>
                            {libelle_correction}
                        </Bouton>
                    })}
                    {regularisable.then(|| view! {
                        <Bouton taille=Taille::Compacte variante=Variante::Secondaire libelle_accessible=format!("Régulariser l'heure : {r2}")
                            on_click=Callback::new(move |_| { regulariser.set(Some(hr.clone())); ouvert_regulariser.set(true); })>
                            "Régulariser"
                        </Bouton>
                    })}
                    {contestable.then(|| view! {
                        <Bouton taille=Taille::Compacte variante=Variante::Discret libelle_accessible=format!("Contester l'heure : {r3}")
                            on_click=Callback::new(move |_| { contester.set(Some(hx.clone())); ouvert_contester.set(true); })>
                            "Contester"
                        </Bouton>
                    })}
                </p>
            })}
        </li>
    }
}

/// Correction (heure non figée) ou régularisation (heure figée ou exportée) : durée demandée, motif codé.
/// `0` annule l'heure. Un titre de `Dialogue` est statique : une boîte par action (FRONTEND_LEPTOS.md §15).
#[component]
fn DialogueDuree(
    regul: bool,
    choix: RwSignal<Option<HeureDto>>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let prefixe = if regul { "absp-regul" } else { "absp-corr" };
    let duree = RwSignal::new(String::new());
    let raison = RwSignal::new(RAISONS_HEURE[0].0.to_string());
    let note = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let options: Vec<(String, String)> = RAISONS_HEURE
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    // La boîte s'ouvre sur la durée actuelle.
    Effect::new(move |_| {
        if let Some(h) = choix.get() {
            let m = duree_effective(&h);
            duree.set(libelle_duree(m).replace(' ', ""));
            erreur.set(None);
        }
    });
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(h) = choix.get_untracked() else {
            return;
        };
        let texte = duree.get_untracked();
        // « 0 » annule l'heure (règle du serveur) ; sinon 1 minute à 24 heures.
        let minutes = if texte.trim() == "0" || texte.trim() == "0min" {
            Ok(0)
        } else {
            analyser_duree(&texte)
        };
        let minutes = match minutes {
            Ok(m) => m,
            Err(m) => {
                erreur.set(Some(m.to_string()));
                return;
            }
        };
        let n = note.get_untracked();
        let corps = CorrigerHeureBody {
            duree_minutes: minutes,
            raison_code: raison.get_untracked(),
            note: (!n.trim().is_empty()).then(|| n.trim().to_string()),
        };
        ouvert.set(false);
        note.set(String::new());
        ecriture.lancer(async move {
            if regul {
                let r = session::client()
                    .absp_regulariser_heure(h.id, &corps)
                    .await?;
                Ok(format!(
                    "Régularisation posée ({}) : elle figurera sur l'état suivant.",
                    libelle_duree(i64::from(r.delta_minutes))
                ))
            } else {
                session::client().absp_corriger_heure(h.id, &corps).await?;
                Ok("Heure corrigée et validée.".to_string())
            }
        });
    };
    let titre = if regul {
        "Régulariser une heure"
    } else {
        "Corriger une heure"
    };
    let aide = if regul {
        "L'heure est figée dans un état : elle n'est pas réécrite. L'écart est reporté sur l'état suivant."
    } else {
        "La durée corrigée remplace celle de l'heure, qui est alors validée."
    };
    view! {
        <Dialogue ouvert=ouvert titre=titre id=format!("{prefixe}-dlg")>
            <form class="absp-formulaire" on:submit=envoyer>
                <p>
                    {move || choix.get().map(|h| format!(
                        "{} · {} · actuellement {}",
                        date_longue(h.le),
                        libelle_nature_heure(&h.nature),
                        libelle_duree(duree_effective(&h))
                    ))}
                </p>
                <ChampTexte
                    id=format!("{prefixe}-duree")
                    libelle="Durée juste"
                    valeur=duree
                    aide="Par exemple 1 h 30 ou 45 min. 0 annule l'heure."
                    erreur=erreur
                    autocomplete="off"
                />
                <Selection id=format!("{prefixe}-raison") libelle="Pourquoi ?" valeur=raison options=options/>
                <ZoneTexte id=format!("{prefixe}-note") libelle="Précision (facultatif)" valeur=note aide="500 caractères au plus."/>
                <p class="texte-attenue">{aide}</p>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>{if regul { "Régulariser" } else { "Corriger" }}</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

/// Contestation par un gestionnaire (ABP-201 : « l'agent ou l'adjoint »).
#[component]
fn DialogueContestationGestion(
    choix: RwSignal<Option<HeureDto>>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let raison = RwSignal::new(RAISONS_HEURE[0].0.to_string());
    let note = RwSignal::new(String::new());
    let options: Vec<(String, String)> = RAISONS_HEURE
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(h) = choix.get_untracked() else {
            return;
        };
        let n = note.get_untracked();
        let corps = ent_types::absences_personnels::ContesterHeureBody {
            raison_code: raison.get_untracked(),
            note: (!n.trim().is_empty()).then(|| n.trim().to_string()),
        };
        ouvert.set(false);
        note.set(String::new());
        ecriture.lancer(async move {
            session::client().absp_contester_heure(h.id, &corps).await?;
            Ok("Heure contestée : elle est à régler avant de figer l'état.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Contester une heure" id="absp-gest-contester-dlg">
            <form class="absp-formulaire" on:submit=envoyer>
                <Selection id="absp-gest-contester-raison" libelle="Pourquoi ?" valeur=raison options=options/>
                <ZoneTexte id="absp-gest-contester-note" libelle="Précision (facultatif)" valeur=note aide="500 caractères au plus."/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Contester"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

/// Saisie d'une heure **sans séance** (HSE hors séance, mission Pacte, vacation, service non fait) : jamais un
/// remplacement (R-AP28), jamais dans le futur, jamais pour soi-même (le serveur refuse).
#[component]
fn DialogueSaisie(
    ouvert: RwSignal<bool>,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
) -> impl IntoView {
    let agent = RwSignal::new(String::new());
    let nature = RwSignal::new(NATURES_SAISIE[0].to_string());
    let duree = RwSignal::new(String::new());
    let le = RwSignal::new(String::new());
    let erreur_duree = RwSignal::new(None::<String>);
    let erreur_date = RwSignal::new(None::<String>);
    let erreur_agent = RwSignal::new(None::<String>);
    let options_nature: Vec<(String, String)> = NATURES_SAISIE
        .iter()
        .map(|n| ((*n).to_string(), libelle_nature_heure(n).to_string()))
        .collect();
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        erreur_duree.set(None);
        erreur_date.set(None);
        erreur_agent.set(None);
        let Ok(agent_id) = Uuid::parse_str(agent.get_untracked().trim()) else {
            erreur_agent.set(Some("Choisissez un agent dans la liste.".into()));
            return;
        };
        let minutes = match analyser_duree(&duree.get_untracked()) {
            Ok(m) => m,
            Err(m) => {
                erreur_duree.set(Some(m.to_string()));
                return;
            }
        };
        let Ok(jour) = NaiveDate::parse_from_str(le.get_untracked().trim(), "%Y-%m-%d") else {
            erreur_date.set(Some("Indiquez la date de l'heure.".into()));
            return;
        };
        let corps = SaisirHeureBody {
            agent_id,
            nature: nature.get_untracked(),
            duree_minutes: minutes,
            le: jour,
        };
        ouvert.set(false);
        duree.set(String::new());
        ecriture.lancer(async move {
            session::client().absp_saisir_heure(&corps).await?;
            Ok("Heure saisie : elle est constatée, à valider.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Saisir une heure sans séance" id="absp-dlg-saisie">
            <form class="absp-formulaire" on:submit=envoyer>
                <p class="texte-attenue">
                    "Le remplacement de courte durée et le rattrapage dû ne se saisissent pas : ils naissent d'une séance assurée."
                </p>
                {move || {
                    let mut o = vec![(String::new(), "Choisir un agent".to_string())];
                    o.extend(noms.get().unwrap_or_default().options());
                    view! { <Selection id="absp-saisie-agent" libelle="Agent" valeur=agent options=o erreur=erreur_agent requis=true/> }
                }}
                <Selection id="absp-saisie-nature" libelle="Nature" valeur=nature options=options_nature/>
                <ChampTexte id="absp-saisie-duree" libelle="Durée" valeur=duree aide="Par exemple 1 h 30 ou 45 min." erreur=erreur_duree requis=true autocomplete="off"/>
                <ChampTexte id="absp-saisie-date" libelle="Jour" valeur=le genre="date" erreur=erreur_date requis=true/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Saisir"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

// ───────────────────────────── État mensuel ─────────────────────────────

#[component]
fn SectionEtat(
    heures: LocalResource<Result<Vec<HeureDto>, ent_web_api::ApiError>>,
    employeur: RwSignal<String>,
    mois: Signal<NaiveDate>,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
    auj: NaiveDate,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    view! {
        {move || match heures.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de l'état…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| heures.refetch())/> }.into_any(),
            Some(Ok(toutes)) => {
                let propres: Vec<HeureDto> = toutes.into_iter().filter(|h| h.employeur == employeur.get()).collect();
                let etats = etats_des_heures(&propres);
                let contestees = nb_a_regler(&propres);
                let a_valider = nb_a_valider(&propres);
                let m = mois.get();
                let emp = employeur.get();
                if let Some(id) = etats.first().copied() {
                    view! { <EtatFige id=id noms=noms ecriture=ecriture peut_signer=moteur.get().a("personnels.heures.signer") peut_exporter=moteur.get().a("personnels.heures.exporter")/> }.into_any()
                } else {
                    let fin = mois_termine(m, auj);
                    let peut_figer = moteur.get().a("personnels.heures.valider") || moteur.get().a("personnels.heures.exporter");
                    view! {
                        <section class="absp-section" aria-labelledby="absp-etat-aucun">
                            <h2 id="absp-etat-aucun" class="vs-section-titre">{format!("{} — {}", libelle_mois(m), libelle_employeur(&emp))}</h2>
                            <p>"L'état de ce mois n'est pas encore figé."</p>
                            {(!fin).then(|| view! {
                                <Alerte niveau=Niveau::Info titre="Le mois n'est pas terminé : l'état ne se fige qu'après son dernier jour."/>
                            })}
                            {(fin && contestees > 0).then(|| view! {
                                <Alerte niveau=Niveau::Avertissement titre=format!("{contestees} heure(s) contestée(s) : réglez-les (section « Heures du mois ») avant de figer.")/>
                            })}
                            {(fin && a_valider > 0).then(|| view! {
                                <Alerte niveau=Niveau::Info titre=format!("{a_valider} heure(s) encore à valider : validez-les avant de figer, sinon elles ne figureront pas dans cet état.")/>
                            })}
                            {(fin && peut_figer).then(|| view! { <BoutonFiger mois=m employeur=emp.clone() ecriture=ecriture/> })}
                        </section>
                    }
                    .into_any()
                }
            }
        }}
    }
}

#[component]
fn BoutonFiger(mois: NaiveDate, employeur: String, ecriture: Ecriture) -> impl IntoView {
    let ouvert = RwSignal::new(false);
    let emp = employeur.clone();
    let message = format!(
        "Figer l'état de {} pour {} ?",
        libelle_mois(mois),
        libelle_employeur(&employeur)
    );
    view! {
        <p class="absp-actions">
            <Bouton on_click=Callback::new(move |_| ouvert.set(true))>"Figer l'état du mois"</Bouton>
        </p>
        <ConfirmationDetaillee
            ouvert=ouvert
            id="absp-dlg-figer"
            titre="Figer l'état mensuel"
            message=Signal::derive(move || message.clone())
            rappel=Signal::derive(|| "Les heures validées sont figées : elles ne se réécrivent plus. Une erreur se corrige ensuite par une régularisation sur l'état suivant.".to_string())
            libelle_confirmer="Figer"
            on_confirmer=Callback::new(move |_| {
                let corps = FigerEtatBody { mois, employeur: emp.clone() };
                ecriture.lancer(async move {
                    session::client().absp_figer_etat(&corps).await?;
                    Ok("État figé : il attend la signature du chef d'établissement.".to_string())
                });
            })
        />
    }
}

#[component]
fn EtatFige(
    id: Uuid,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
    peut_signer: bool,
    peut_exporter: bool,
) -> impl IntoView {
    let etat = LocalResource::new(move || {
        // Relu après chaque écriture : la signature change l'état.
        let _ = ecriture.annonce.get();
        async move { session::client().absp_etat_mensuel(id).await }
    });
    view! {
        {move || match etat.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de l'état…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| etat.refetch())/> }.into_any(),
            Some(Ok(e)) => view! { <ContenuEtat e=e noms=noms ecriture=ecriture peut_signer=peut_signer peut_exporter=peut_exporter/> }.into_any(),
        }}
    }
}

#[component]
fn ContenuEtat(
    e: EtatMensuelDto,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
    peut_signer: bool,
    peut_exporter: bool,
) -> impl IntoView {
    let id = e.id;
    let ouvert_signature = RwSignal::new(false);
    let erreur_export = RwSignal::new(None::<String>);
    let signable = super::modele_heures::peut_signer(&e);
    let titre = format!(
        "{} — {}",
        libelle_mois(e.mois),
        libelle_employeur(&e.employeur)
    );
    let message_signature = format!("Signer l'état de {} ?", libelle_mois(e.mois));
    let par_agent = totaux_par_agent(&e.lignes);
    let signe_le = e.signe_le.map(crate::noyau::temps::date_heure_paris);
    let nb_regul = e
        .lignes
        .iter()
        .filter(|l| l.regularisation_de_id.is_some())
        .count();
    view! {
        <section class="absp-section" aria-labelledby="absp-etat-titre">
            <div class="absp-carte__tete">
                <h2 id="absp-etat-titre" class="vs-section-titre">{titre}</h2>
                <Badge libelle=libelle_etat_mensuel(&e.etat) ton=ton_etat_mensuel(&e.etat)/>
            </div>
            <p class="absp-resume">
                {format!("Total : {} pour {} agent(s).", libelle_duree(e.total_minutes), par_agent.len())}
            </p>
            {signe_le.map(|d| view! { <p class="absp-carte__detail">{format!("Signé le {d}.")}</p> })}
            {(nb_regul > 0).then(|| view! {
                <p class="absp-carte__detail">
                    {format!("Dont {nb_regul} ligne(s) de régularisation d'heures déjà exportées.")}
                </p>
            })}
            <p class="absp-actions">
                {(signable && peut_signer).then(|| view! {
                    <Bouton on_click=Callback::new(move |_| ouvert_signature.set(true))>"Signer l'état"</Bouton>
                })}
                {peut_exporter.then(|| view! {
                    <Bouton variante=Variante::Secondaire
                        on_click=Callback::new(move |_| {
                            lancer_export(
                                async move { session::client().absp_etat_csv(id).await },
                                "etat-heures.csv",
                                erreur_export,
                                ecriture,
                            );
                        })>
                        "Exporter (CSV)"
                    </Bouton>
                })}
            </p>
            {move || erreur_export.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {(signable && !peut_signer).then(|| view! {
                <p class="texte-attenue">"La signature est réservée au chef d'établissement."</p>
            })}
            <ul class="absp-liste absp-liste--serree">
                {par_agent.into_iter().map(|(agent, minutes, regul)| view! {
                    <li class="absp-ligne">
                        <span>
                            {move || noms.get().unwrap_or_default().nom(agent)}
                            {regul.then_some(" (avec régularisation)")}
                        </span>
                        <strong>{libelle_duree(minutes)}</strong>
                    </li>
                }).collect_view()}
            </ul>
            <ConfirmationDetaillee
                ouvert=ouvert_signature
                id="absp-dlg-signer"
                titre="Signer l'état mensuel"
                message=Signal::derive(move || message_signature.clone())
                rappel=Signal::derive(|| "La signature engage le chef d'établissement ; l'état ne change plus ensuite.".to_string())
                libelle_confirmer="Signer"
                on_confirmer=Callback::new(move |_| {
                    ecriture.lancer(async move {
                        session::client().absp_signer_etat(id).await?;
                        Ok("État signé.".to_string())
                    });
                })
            />
        </section>
    }
}
