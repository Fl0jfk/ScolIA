//! Réglages des absences des personnels (A11f, ABP-260, 262, 272) : seuil de la mention « répond rarement aux
//! sollicitations » (par établissement), emploi du temps de référence, règles de circuit (valideurs,
//! instructeurs, informés).
//!
//! Écriture : droit `parametrer` (direction, administration), jugé par le serveur. L'organisateur des
//! remplacements **lit** le seuil, sans pouvoir le changer. La mention n'est servie qu'à l'organisateur, sans
//! chiffre, jamais au professeur concerné (R-AP2). Les autres réglages (S-1500 à S-1517) restent fixés par le
//! service : cet écran ne prétend pas les régler.

use ent_types::absences_personnels::RegleCircuitDto;
use ent_web_ui::{
    Badge, Bouton, ChampTexte, Dialogue, EtatVide, Selection, Squelette, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;

use super::commun::{charger_noms, GardePersonnel};
use super::modele_heures::Noms;
use super::modele_parametrage::{
    analyser_seuil, composer_regle, libelle_employeur_circuit, libelle_famille_circuit,
    phrase_seuil, regles_par_role, SaisieRegle, EMPLOYEURS_CIRCUIT, FAMILLES_CIRCUIT,
    ROLES_CIRCUIT, SEUIL_MAX, SEUIL_MIN,
};
use crate::composants_metier::confirmation_detaillee::ConfirmationDetaillee;
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;

#[component]
pub fn PageParametrageAbsences() -> impl IntoView {
    view! {
        <Title text="Réglages des absences"/>
        <GardePersonnel
            titre="Réglages des absences"
            sous_titre="Le seuil de la mention « répond rarement », l'emploi du temps de référence et le circuit de validation."
        >
            <Contenu/>
        </GardePersonnel>
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let noms = charger_noms();
    view! {
        <RetourEcriture ecriture=ecriture/>
        <SectionSeuil version=version ecriture=ecriture/>
        <SectionEdtReference version=version ecriture=ecriture/>
        <SectionCircuit version=version ecriture=ecriture noms=noms/>
    }
}

// ───────────────────────────── Seuil de la mention ─────────────────────────────

#[component]
fn SectionSeuil(version: RwSignal<u32>, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let parametres = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_parametres().await }
    });
    let saisie = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    // Le champ prend la valeur en vigueur à chaque chargement.
    Effect::new(move |_| {
        if let Some(Ok(p)) = parametres.get() {
            saisie.set(p.seuil_sans_reponse_min.to_string());
        }
    });
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let seuil = match analyser_seuil(&saisie.get_untracked()) {
            Ok(s) => s,
            Err(m) => {
                erreur.set(Some(m));
                return;
            }
        };
        erreur.set(None);
        ecriture.lancer(async move {
            let p = session::client().absp_definir_parametres(seuil).await?;
            Ok(format!(
                "Seuil réglé à {} demande(s) restée(s) sans réponse.",
                p.seuil_sans_reponse_min
            ))
        });
    };
    view! {
        <section class="absp-section" aria-labelledby="absp-seuil">
            <h2 id="absp-seuil" class="vs-section-titre">"Mention « répond rarement aux sollicitations »"</h2>
            <p class="texte-attenue">
                "Quand vous choisissez un remplaçant, une mention neutre, sans chiffre, signale un collègue qui laisse souvent les demandes sans réponse. Elle n'est montrée qu'à la personne qui organise les remplacements, jamais au collègue concerné ni aux autres professeurs."
            </p>
            {move || match parametres.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement du réglage…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| parametres.refetch())/> }.into_any(),
                Some(Ok(p)) => {
                    let peut = moteur.get().a("personnels.absences.parametrer");
                    view! {
                        <p class="absp-resume">{phrase_seuil(p.seuil_sans_reponse_min)}</p>
                        <p class="absp-carte__detail">
                            {if p.regle {
                                format!(
                                    "Réglé par l'établissement{}. Valeur d'origine : {}.",
                                    p.modifie_le.map(|d| format!(" le {}", date_heure_paris(d))).unwrap_or_default(),
                                    p.seuil_sans_reponse_defaut
                                )
                            } else {
                                format!("Valeur d'origine ({}), jamais modifiée par l'établissement.", p.seuil_sans_reponse_defaut)
                            }}
                        </p>
                        {if peut {
                            view! {
                                <form class="absp-barre" on:submit=envoyer>
                                    <ChampTexte
                                        id="absp-seuil-valeur"
                                        libelle="Seuil (demandes restées sans réponse)"
                                        valeur=saisie
                                        genre="number"
                                        inputmode="numeric"
                                        aide=format!("De {SEUIL_MIN} à {SEUIL_MAX}. Plus le seuil est bas, plus la mention est fréquente.")
                                        erreur=erreur
                                        autocomplete="off"
                                    />
                                    <Bouton soumettre=true>"Enregistrer le seuil"</Bouton>
                                </form>
                            }
                            .into_any()
                        } else {
                            view! { <p class="texte-attenue">"Le seuil est réglé par la direction."</p> }.into_any()
                        }}
                    }
                    .into_any()
                }
            }}
        </section>
    }
}

// ───────────────────────────── Emploi du temps de référence ─────────────────────────────

#[component]
fn SectionEdtReference(version: RwSignal<u32>, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let etat = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_edt_reference().await }
    });
    let ouvert = RwSignal::new(false);
    view! {
        <section class="absp-section" aria-labelledby="absp-edt-ref">
            <h2 id="absp-edt-ref" class="vs-section-titre">"Emploi du temps de référence"</h2>
            <p class="texte-attenue">
                "Tant qu'il est éteint, aucune absence ne produit d'effet vers les familles et les élèves, et aucune sortie anticipée n'est proposée : l'emploi du temps officiel peut rester celui d'un autre outil."
            </p>
            {move || match etat.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement du réglage…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| etat.refetch())/> }.into_any(),
                Some(Ok(e)) => {
                    let actif = e.actif;
                    let peut = moteur.get().a("personnels.absences.parametrer");
                    view! {
                        <p class="absp-carte__tete">
                            <Badge
                                libelle=if actif { "ScolIA est l'emploi du temps de référence" } else { "Éteint" }
                                ton=if actif { TonEtat::Ok } else { TonEtat::Neutre }
                            />
                            {e.pose_le.map(|d| view! { <span class="absp-carte__detail">{format!("Posé le {}", date_heure_paris(d))}</span> })}
                        </p>
                        {peut.then(|| view! {
                            <p class="absp-actions">
                                <Bouton variante=if actif { Variante::Secondaire } else { Variante::Primaire }
                                    on_click=Callback::new(move |_| ouvert.set(true))>
                                    {if actif { "Éteindre" } else { "Allumer" }}
                                </Bouton>
                            </p>
                            <ConfirmationDetaillee
                                ouvert=ouvert
                                id="absp-dlg-edt-reference"
                                titre="Emploi du temps de référence"
                                message=if actif {
                                    "Éteindre : les familles et les élèves ne seront plus prévenus des remplacements."
                                } else {
                                    "Allumer : les remplacements et annulations seront communiqués aux familles et aux élèves."
                                }
                                rappel=Signal::derive(move || if actif {
                                    "Les séances déjà modifiées restent telles quelles.".to_string()
                                } else {
                                    "Le serveur refuse si aucune séance n'est prévue dans les 14 prochains jours.".to_string()
                                })
                                libelle_confirmer=if actif { "Éteindre" } else { "Allumer" }
                                on_confirmer=Callback::new(move |_| {
                                    ecriture.lancer(async move {
                                        let r = session::client().absp_definir_edt_reference(!actif).await?;
                                        Ok(if r.actif { "Emploi du temps de référence allumé.".to_string() } else { "Emploi du temps de référence éteint.".to_string() })
                                    });
                                })
                            />
                        })}
                    }
                    .into_any()
                }
            }}
        </section>
    }
}

// ───────────────────────────── Circuit de validation ─────────────────────────────

#[component]
fn SectionCircuit(
    version: RwSignal<u32>,
    ecriture: Ecriture,
    noms: LocalResource<Noms>,
) -> impl IntoView {
    let moteur = use_droits_moteur();
    let regles = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_regles_circuit().await }
    });
    let ouvert_ajout = RwSignal::new(false);
    view! {
        <section class="absp-section" aria-labelledby="absp-circuit">
            <h2 id="absp-circuit" class="vs-section-titre">"Circuit de validation"</h2>
            <p class="texte-attenue">
                "Qui valide, instruit ou est informé d'une absence, selon les personnels et la famille de nature. Sans règle, le chef d'établissement valide ; le secrétariat (agents de l'État) ou la comptabilité (OGEC) instruit. Les absences déjà déclarées gardent leur circuit."
            </p>
            {move || moteur.get().a("personnels.absences.parametrer").then(|| view! {
                <p class="absp-actions">
                    <Bouton on_click=Callback::new(move |_| ouvert_ajout.set(true))>"Ajouter une règle"</Bouton>
                </p>
            })}
            {move || match regles.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des règles…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| regles.refetch())/> }.into_any(),
                Some(Ok(v)) if v.is_empty() => view! {
                    <EtatVide titre="Aucune règle propre à l'établissement" message="Le circuit par défaut s'applique."/>
                }
                .into_any(),
                Some(Ok(v)) => view! {
                    {regles_par_role(&v).into_iter().map(|(titre, g)| view! {
                        <h3 class="absp-sous-titre">{titre}</h3>
                        <ul class="absp-liste">
                            {g.into_iter().map(|r| view! { <CarteRegle r=r noms=noms ecriture=ecriture/> }).collect_view()}
                        </ul>
                    }).collect_view()}
                }
                .into_any(),
            }}
            <DialogueRegle ouvert=ouvert_ajout noms=noms ecriture=ecriture/>
        </section>
    }
}

#[component]
fn CarteRegle(r: RegleCircuitDto, noms: LocalResource<Noms>, ecriture: Ecriture) -> impl IntoView {
    let moteur = use_droits_moteur();
    let id = r.id;
    let agent = r.agent_id;
    let ouvert = RwSignal::new(false);
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{move || noms.get().unwrap_or_default().nom(agent)}</strong>
                <Badge libelle=format!("Rang {}", r.rang)/>
            </div>
            <p class="absp-carte__detail">
                {format!("{} · {}", libelle_employeur_circuit(&r.employeur), libelle_famille_circuit(r.famille.as_deref()))}
            </p>
            {move || moteur.get().a("personnels.absences.parametrer").then(|| view! {
                <p class="absp-carte__actions">
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire
                        libelle_accessible=format!("Retirer la règle de {}", noms.get().unwrap_or_default().nom(agent))
                        on_click=Callback::new(move |_| ouvert.set(true))>
                        "Retirer"
                    </Bouton>
                </p>
            })}
            <ConfirmationDetaillee
                ouvert=ouvert
                id=format!("absp-dlg-retirer-regle-{id}")
                titre="Retirer la règle de circuit"
                message="Cette règle ne s'appliquera plus aux nouvelles absences."
                rappel=Signal::derive(|| "Les absences déjà déclarées gardent leur circuit.".to_string())
                libelle_confirmer="Retirer"
                danger=true
                on_confirmer=Callback::new(move |_| {
                    ecriture.lancer(async move {
                        session::client().absp_retirer_regle_circuit(id).await?;
                        Ok("Règle retirée.".to_string())
                    });
                })
            />
        </li>
    }
}

#[component]
fn DialogueRegle(
    ouvert: RwSignal<bool>,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
) -> impl IntoView {
    let employeur = RwSignal::new("tous".to_string());
    let famille = RwSignal::new(String::new());
    let role = RwSignal::new("valideur".to_string());
    let agent = RwSignal::new(String::new());
    let rang = RwSignal::new("1".to_string());
    let erreur = RwSignal::new(None::<String>);
    let options_employeur: Vec<(String, String)> = EMPLOYEURS_CIRCUIT
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let options_famille: Vec<(String, String)> = FAMILLES_CIRCUIT
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let options_role: Vec<(String, String)> = ROLES_CIRCUIT
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect();
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieRegle {
            employeur: employeur.get_untracked(),
            famille: famille.get_untracked(),
            role: role.get_untracked(),
            agent: agent.get_untracked(),
            rang: rang.get_untracked(),
        };
        let corps = match composer_regle(&saisie) {
            Ok(c) => c,
            Err(m) => {
                erreur.set(Some(m));
                return;
            }
        };
        erreur.set(None);
        ouvert.set(false);
        ecriture.lancer(async move {
            session::client().absp_creer_regle_circuit(&corps).await?;
            Ok("Règle ajoutée.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Ajouter une règle de circuit" id="absp-dlg-regle">
            <form class="absp-formulaire" on:submit=envoyer>
                <Selection id="absp-regle-role" libelle="Rôle" valeur=role options=options_role/>
                <Selection id="absp-regle-employeur" libelle="Personnels concernés" valeur=employeur options=options_employeur/>
                <Selection id="absp-regle-famille" libelle="Famille de nature" valeur=famille options=options_famille/>
                {move || {
                    let mut o = vec![(String::new(), "Choisir un agent".to_string())];
                    o.extend(noms.get().unwrap_or_default().options());
                    view! { <Selection id="absp-regle-agent" libelle="Agent" valeur=agent options=o requis=true/> }
                }}
                <ChampTexte
                    id="absp-regle-rang"
                    libelle="Rang"
                    valeur=rang
                    genre="number"
                    inputmode="numeric"
                    aide="De 1 à 5 : le rang 1 est sollicité d'abord."
                    erreur=erreur
                />
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Ajouter"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}
