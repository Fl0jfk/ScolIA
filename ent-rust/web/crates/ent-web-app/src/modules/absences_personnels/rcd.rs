//! Remplacement de courte durée, RCD (A11e, ABP-213, 220 à 231) : plan annuel **versionné** (avis du conseil
//! pédagogique, présentation, transmission au recteur, comptes rendus), volontaires et compteurs, remontée
//! hebdomadaire **sans nom** (R-02) à contrôler puis laisser partir.
//!
//! Le serveur juge : `rcd.gerer` (plan, vivier), `rcd.remontee` (remontées). Un volontaire ne voit que **son**
//! compteur et sa position par rapport à la médiane, sans nommer personne (ABP-213). Aucune nature d'absence
//! n'est lue : la remontée ne porte que la discipline, le niveau, la catégorie nationale du motif et le mode de
//! continuité. Le flux vers SI-RCD n'est pas branché (Q-AP3) : l'export CSV est le tableau équivalent.

use chrono::NaiveDate;
use ent_types::absences_personnels::{
    CompteRenduRcdBody, CompteurRcdDto, EngagementRcdDto, ModifierEngagementRcdBody, PlanRcdDto,
    RemonteeRcdDto,
};
use ent_web_api::SuiteRemontee;
use ent_web_ui::{
    Alerte, Badge, Bouton, Case, ChampTexte, Dialogue, EtatVide, Niveau, OptionSegment, Segmente,
    Selection, Squelette, Taille, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::commun::{charger_noms, lancer_export, GardePersonnel};
use super::modele_heures::Noms;
use super::modele_rcd::{
    actions_remontee, campagne_lancable, composer_engagement, composer_periode, composer_plan,
    composer_procedure, comptes_rendus_manquants, libelle_categorie_nationale, libelle_etat_plan,
    libelle_etat_remontee, libelle_heures, libelle_instance, libelle_mode_continuite,
    libelle_position, libelle_semaine, libelle_statut_engagement, lignes_par_discipline, lundi_de,
    part_consommee, plan_modifiable_en_place, saisie_depuis_plan, semaine_passee,
    taux_remplacement, ton_etat_plan, ton_etat_remontee, totaux_remontee, SaisieEngagement,
    SaisiePlan, SaisieProcedure, INSTANCES, STATUTS_ENGAGEMENT,
};
use crate::composants_metier::confirmation_detaillee::ConfirmationDetaillee;
use crate::noyau::coquille::use_droits_moteur;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue};

#[component]
pub fn PageRcd() -> impl IntoView {
    view! {
        <Title text="Remplacement de courte durée"/>
        <GardePersonnel
            titre="Remplacement de courte durée"
            sous_titre="Le plan de l'année, les volontaires et la remontée de la semaine."
        >
            <Contenu/>
        </GardePersonnel>
    }
}

fn jour_texte(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

#[component]
fn Contenu() -> impl IntoView {
    let moteur = use_droits_moteur();
    let auj = aujourdhui();
    let section = RwSignal::new("plan".to_string());
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let noms = charger_noms();
    let options = move || {
        let m = moteur.get();
        let mut o = vec![
            OptionSegment::new("plan", "Plan de l'année"),
            OptionSegment::new("engagement", "Mon engagement"),
        ];
        if m.a("personnels.rcd.gerer") {
            o.push(OptionSegment::new("vivier", "Volontaires"));
        }
        if m.a("personnels.rcd.remontee") {
            o.push(OptionSegment::new("remontee", "Remontée de la semaine"));
        }
        o
    };
    view! {
        <RetourEcriture ecriture=ecriture/>
        <div class="absp-onglets">
            {move || view! {
                <Segmente nom="absp-section-rcd" legende="Section" options=options() valeur=section/>
            }}
        </div>
        {move || match section.get().as_str() {
            "engagement" => view! { <SectionEngagement version=version noms=noms ecriture=ecriture auj=auj/> }.into_any(),
            "vivier" if moteur.get().a("personnels.rcd.gerer") => view! { <SectionVivier version=version noms=noms ecriture=ecriture auj=auj/> }.into_any(),
            "remontee" if moteur.get().a("personnels.rcd.remontee") => view! { <SectionRemontee version=version ecriture=ecriture auj=auj/> }.into_any(),
            _ => view! { <SectionPlan version=version ecriture=ecriture auj=auj/> }.into_any(),
        }}
    }
}

// ───────────────────────────── Plan ─────────────────────────────

#[component]
fn SectionPlan(version: RwSignal<u32>, ecriture: Ecriture, auj: NaiveDate) -> impl IntoView {
    let moteur = use_droits_moteur();
    let courant = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_plan_courant().await }
    });
    let ouvert_creation = RwSignal::new(false);
    view! {
        {move || match courant.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement du plan…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| courant.refetch())/> }.into_any(),
            Some(Ok(None)) => {
                let gere = moteur.get().a("personnels.rcd.gerer");
                view! {
                    <EtatVide
                        titre="Pas encore de plan pour cette année"
                        message="Le plan de remplacement de courte durée est établi chaque année par le chef d'établissement."
                    />
                    {gere.then(|| view! {
                        <p class="absp-actions">
                            <Bouton on_click=Callback::new(move |_| ouvert_creation.set(true))>"Créer le plan de l'année"</Bouton>
                        </p>
                        <DialoguePlan mode=ModePlan::Creer plan=None ouvert=ouvert_creation ecriture=ecriture/>
                    })}
                }
                .into_any()
            }
            Some(Ok(Some(p))) => view! { <CartePlan p=p ecriture=ecriture auj=auj/> }.into_any(),
        }}
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModePlan {
    Creer,
    Modifier,
    Version,
}

#[component]
fn CartePlan(p: PlanRcdDto, ecriture: Ecriture, auj: NaiveDate) -> impl IntoView {
    let moteur = use_droits_moteur();
    let gere = move || moteur.get().a("personnels.rcd.gerer");
    let id = p.id;
    let ouvert_plan = RwSignal::new(false);
    let ouvert_procedure = RwSignal::new(false);
    let ouvert_campagne = RwSignal::new(false);
    let ouvert_cr = RwSignal::new(false);
    let en_place = plan_modifiable_en_place(&p);
    let manquants = comptes_rendus_manquants(&p);
    let lancable = campagne_lancable(&p);
    let mode = if en_place {
        ModePlan::Modifier
    } else {
        ModePlan::Version
    };
    view! {
        <section class="absp-section" aria-labelledby="absp-plan">
            <div class="absp-carte__tete">
                <h2 id="absp-plan" class="vs-section-titre">{format!("Plan de l'année, version {}", p.version)}</h2>
                <Badge libelle=libelle_etat_plan(&p.etat) ton=ton_etat_plan(&p.etat)/>
            </div>
            <dl class="absp-carte">
                <div class="absp-ligne"><dt>"Délai de prévenance d'un volontaire"</dt><dd>{format!("{} h", p.delai_prevenance_heures)}</dd></div>
                <div class="absp-ligne"><dt>"Continuité numérique encadrée"</dt><dd>{if p.continuite_numerique { "Oui" } else { "Non" }}</dd></div>
                <div class="absp-ligne"><dt>"Avis du conseil pédagogique"</dt><dd>{p.avis_cp_le.map_or("Non renseigné".to_string(), date_longue)}</dd></div>
                <div class="absp-ligne">
                    <dt>"Présentation"</dt>
                    <dd>{match (p.presente_le, p.instance.as_deref()) {
                        (Some(d), Some(i)) => format!("{} — {}", date_longue(d), libelle_instance(i)),
                        (Some(d), None) => date_longue(d),
                        _ => "Non renseignée".to_string(),
                    }}</dd>
                </div>
                <div class="absp-ligne"><dt>"Transmission au recteur"</dt><dd>{p.transmis_le.map_or("Non renseignée".to_string(), date_longue)}</dd></div>
                <div class="absp-ligne">
                    <dt>"Campagne de volontariat"</dt>
                    <dd>{p.campagne_lancee_le.map_or("Pas lancée".to_string(), |d| format!("Lancée le {}", crate::noyau::temps::date_heure_paris(d)))}</dd>
                </div>
                <div class="absp-ligne"><dt>"Comptes rendus produits"</dt><dd>{p.comptes_rendus}</dd></div>
            </dl>
            {p.objectifs.clone().map(|o| view! {
                <div class="absp-carte"><strong>"Objectifs"</strong><p class="absp-message">{o}</p></div>
            })}
            {(manquants > 0 && p.etat != "brouillon").then(|| view! {
                <Alerte
                    niveau=Niveau::Info
                    titre=format!("{manquants} compte(s) rendu(s) à produire cette année : la règle en exige au moins deux.")
                />
            })}
            {move || gere().then(|| view! {
                <p class="absp-actions">
                    <Bouton on_click=Callback::new(move |_| ouvert_plan.set(true))>
                        {if en_place { "Modifier le brouillon" } else { "Créer une nouvelle version" }}
                    </Bouton>
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert_procedure.set(true))>"Dates de procédure"</Bouton>
                    {lancable.then(|| view! {
                        <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert_campagne.set(true))>"Lancer la campagne de volontariat"</Bouton>
                    })}
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert_cr.set(true))>"Produire un compte rendu"</Bouton>
                </p>
            })}
            <DialoguePlan mode=mode plan=Some(p.clone()) ouvert=ouvert_plan ecriture=ecriture/>
            <DialogueProcedure plan=p.clone() ouvert=ouvert_procedure ecriture=ecriture/>
            <DialogueCompteRendu id=id ouvert=ouvert_cr ecriture=ecriture auj=auj/>
            <ConfirmationDetaillee
                ouvert=ouvert_campagne
                id="absp-dlg-campagne"
                titre="Lancer la campagne de volontariat"
                message="Les enseignants sont invités à se porter volontaires pour le remplacement de courte durée, avec leurs vœux d'emploi du temps."
                rappel=Signal::derive(|| "La campagne ne se lance qu'une fois par version du plan.".to_string())
                libelle_confirmer="Lancer la campagne"
                on_confirmer=Callback::new(move |_| {
                    ecriture.lancer(async move {
                        session::client().absp_campagne_rcd(id).await?;
                        Ok("Campagne de volontariat lancée.".to_string())
                    });
                })
            />
        </section>
        {move || gere().then(|| view! { <Versions ecriture=ecriture/> })}
    }
}

/// Historique des versions du plan (ABP-221) : l'ancienne version reste lisible, archivée.
#[component]
fn Versions(ecriture: Ecriture) -> impl IntoView {
    let plans = LocalResource::new(move || {
        let _ = ecriture.annonce.get();
        async { session::client().absp_plans_rcd().await }
    });
    view! {
        {move || match plans.get() {
            Some(Ok(v)) if v.len() > 1 => view! {
                <section class="absp-section" aria-labelledby="absp-versions">
                    <h2 id="absp-versions" class="vs-section-titre">"Versions du plan"</h2>
                    <ul class="absp-liste absp-liste--serree">
                        {v.into_iter().map(|p| view! {
                            <li class="absp-ligne">
                                <span>{format!("Version {} — créée le {}", p.version, crate::noyau::temps::date_heure_paris(p.cree_le))}</span>
                                <Badge libelle=libelle_etat_plan(&p.etat) ton=ton_etat_plan(&p.etat)/>
                            </li>
                        }).collect_view()}
                    </ul>
                </section>
            }
            .into_any(),
            _ => ().into_any(),
        }}
    }
}

#[component]
fn DialoguePlan(
    mode: ModePlan,
    plan: Option<PlanRcdDto>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let depart = plan.as_ref().map(saisie_depuis_plan).unwrap_or_default();
    let objectifs = RwSignal::new(depart.objectifs);
    let delai = RwSignal::new(depart.delai);
    let continuite = RwSignal::new(depart.continuite_numerique);
    let erreur = RwSignal::new(None::<String>);
    let referent = plan.as_ref().and_then(|p| p.referent_agent_id);
    let plan_id = plan.as_ref().map(|p| p.id);
    let (titre, bouton, message) = match mode {
        ModePlan::Creer => (
            "Créer le plan de l'année",
            "Créer le plan",
            "Plan créé (brouillon).",
        ),
        ModePlan::Modifier => (
            "Modifier le brouillon du plan",
            "Enregistrer",
            "Brouillon enregistré.",
        ),
        ModePlan::Version => (
            "Nouvelle version du plan",
            "Créer la version",
            "Nouvelle version créée : l'ancienne est archivée.",
        ),
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisiePlan {
            objectifs: objectifs.get_untracked(),
            delai: delai.get_untracked(),
            continuite_numerique: continuite.get_untracked(),
        };
        let mut corps = match composer_plan(&saisie) {
            Ok(c) => c,
            Err(m) => {
                erreur.set(Some(m));
                return;
            }
        };
        corps.referent_agent_id = referent;
        erreur.set(None);
        ouvert.set(false);
        ecriture.lancer(async move {
            match (mode, plan_id) {
                (ModePlan::Modifier, Some(id)) => {
                    session::client().absp_modifier_plan(id, &corps).await?
                }
                (ModePlan::Version, Some(id)) => {
                    session::client().absp_versionner_plan(id, &corps).await?
                }
                _ => session::client().absp_creer_plan(&corps).await?,
            };
            Ok(message.to_string())
        });
    };
    let prefixe = match mode {
        ModePlan::Creer => "absp-plan-creer",
        ModePlan::Modifier => "absp-plan-modifier",
        ModePlan::Version => "absp-plan-version",
    };
    view! {
        <Dialogue ouvert=ouvert titre=titre id=format!("{prefixe}-dlg")>
            <form class="absp-formulaire" on:submit=envoyer>
                <ZoneTexte
                    id=format!("{prefixe}-objectifs")
                    libelle="Objectifs du plan"
                    valeur=objectifs
                    aide="1 000 caractères au plus."
                />
                <ChampTexte
                    id=format!("{prefixe}-delai")
                    libelle="Délai de prévenance (heures)"
                    valeur=delai
                    genre="number"
                    inputmode="numeric"
                    aide="De 0 à 168 heures ; 24 par défaut. Un candidat sollicité hors délai n'est proposé que sur accord."
                    erreur=erreur
                />
                <Case id=format!("{prefixe}-numerique") libelle="Continuité numérique encadrée prévue" coche=continuite/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>{bouton}</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
fn DialogueProcedure(
    plan: PlanRcdDto,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let avis = RwSignal::new(plan.avis_cp_le.map(jour_texte).unwrap_or_default());
    let presente = RwSignal::new(plan.presente_le.map(jour_texte).unwrap_or_default());
    let instance = RwSignal::new(plan.instance.clone().unwrap_or_default());
    let transmis = RwSignal::new(plan.transmis_le.map(jour_texte).unwrap_or_default());
    let erreur = RwSignal::new(None::<String>);
    let id = plan.id;
    let mut options: Vec<(String, String)> =
        vec![(String::new(), "Choisir l'instance".to_string())];
    options.extend(
        INSTANCES
            .iter()
            .map(|(c, l)| ((*c).to_string(), (*l).to_string())),
    );
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieProcedure {
            avis_cp_le: avis.get_untracked(),
            presente_le: presente.get_untracked(),
            instance: instance.get_untracked(),
            transmis_le: transmis.get_untracked(),
        };
        let corps = match composer_procedure(&saisie) {
            Ok(c) => c,
            Err(m) => {
                erreur.set(Some(m));
                return;
            }
        };
        erreur.set(None);
        ouvert.set(false);
        ecriture.lancer(async move {
            session::client().absp_procedure_plan(id, &corps).await?;
            Ok("Dates de procédure enregistrées.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Dates de procédure du plan" id="absp-plan-procedure-dlg">
            <form class="absp-formulaire" on:submit=envoyer>
                <p class="texte-attenue">
                    "Avis du conseil pédagogique, présentation au conseil d'administration (ou à l'instance de l'établissement dans le privé), transmission au recteur."
                </p>
                <ChampTexte id="absp-proc-avis" libelle="Avis du conseil pédagogique le" valeur=avis genre="date"/>
                <ChampTexte id="absp-proc-presente" libelle="Présenté le" valeur=presente genre="date"/>
                <Selection id="absp-proc-instance" libelle="Instance de présentation" valeur=instance options=options/>
                <ChampTexte id="absp-proc-transmis" libelle="Transmis au recteur le" valeur=transmis genre="date" erreur=erreur/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
fn DialogueCompteRendu(
    id: Uuid,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
    auj: NaiveDate,
) -> impl IntoView {
    let du = RwSignal::new(String::new());
    let au = RwSignal::new(jour_texte(auj));
    let erreur = RwSignal::new(None::<String>);
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let (d, a) = match composer_periode(&du.get_untracked(), &au.get_untracked()) {
            Ok(p) => p,
            Err(m) => {
                erreur.set(Some(m));
                return;
            }
        };
        erreur.set(None);
        ouvert.set(false);
        ecriture.lancer(async move {
            let r = session::client()
                .absp_compte_rendu_rcd(id, &CompteRenduRcdBody { periode_du: d, periode_au: a })
                .await?;
            Ok(format!(
                "Compte rendu produit : {} prévues, {} d'absence, {} remplacées (sur {} remontée(s)).",
                libelle_heures(r.heures_prevues),
                libelle_heures(r.heures_absence),
                libelle_heures(r.heures_remplacees),
                r.remontees
            ))
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Produire un compte rendu" id="absp-plan-cr-dlg">
            <form class="absp-formulaire" on:submit=envoyer>
                <p class="texte-attenue">
                    "Le compte rendu totalise les remontées de la période (heures prévues, d'absence, remplacées). Il en faut au moins deux par an."
                </p>
                <ChampTexte id="absp-cr-du" libelle="Du" valeur=du genre="date" requis=true/>
                <ChampTexte id="absp-cr-au" libelle="Au" valeur=au genre="date" requis=true erreur=erreur/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Produire"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

// ───────────────────────────── Mon engagement ─────────────────────────────

fn options_statut() -> Vec<(String, String)> {
    STATUTS_ENGAGEMENT
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect()
}

#[component]
fn SectionEngagement(
    version: RwSignal<u32>,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
    auj: NaiveDate,
) -> impl IntoView {
    let compteur = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_mon_compteur_rcd().await }
    });
    let ouvert_engager = RwSignal::new(false);
    view! {
        {move || match compteur.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de votre compteur…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| compteur.refetch())/> }.into_any(),
            Some(Ok(None)) => view! {
                <EtatVide
                    titre="Vous n'êtes pas volontaire"
                    message="Le remplacement de courte durée repose sur des volontaires, sur des créneaux fixes d'au moins une heure par semaine. Votre accord vaut : vous pouvez le retirer."
                />
                <p class="absp-actions">
                    <Bouton on_click=Callback::new(move |_| ouvert_engager.set(true))>"Me porter volontaire"</Bouton>
                </p>
                <DialogueEngagement soi_meme=true noms=noms ouvert=ouvert_engager ecriture=ecriture auj=auj/>
            }
            .into_any(),
            Some(Ok(Some(c))) => view! { <CarteCompteur c=c ecriture=ecriture soi=true noms=noms/> }.into_any(),
        }}
    }
}

#[component]
fn CarteCompteur(
    c: CompteurRcdDto,
    ecriture: Ecriture,
    soi: bool,
    noms: LocalResource<Noms>,
) -> impl IntoView {
    let e: EngagementRcdDto = c.engagement.clone();
    let id = e.id;
    let agent = e.agent_id;
    let ouvert_fin = RwSignal::new(false);
    let ouvert_modif = RwSignal::new(false);
    let part = part_consommee(c.heures_acceptees, e.volume_heures);
    let actif = e.etat == "actif";
    let titre = if soi {
        "Mon engagement".to_string()
    } else {
        String::new()
    };
    view! {
        <section class="absp-carte" aria-label=if soi { "Mon engagement" } else { "Compteur d'un volontaire" }>
            <div class="absp-carte__tete">
                {if soi {
                    view! { <h2 class="vs-section-titre">{titre}</h2> }.into_any()
                } else {
                    view! { <strong class="absp-carte__titre">{move || noms.get().unwrap_or_default().nom(agent)}</strong> }.into_any()
                }}
                <Badge libelle=if actif { "Actif" } else { "Terminé" } ton=if actif { ent_web_ui::TonEtat::Ok } else { ent_web_ui::TonEtat::Neutre }/>
            </div>
            <p class="absp-carte__detail">
                {format!(
                    "{} · {} — du {} au {}",
                    libelle_heures(e.volume_heures),
                    libelle_statut_engagement(&e.statut),
                    date_longue(e.du),
                    date_longue(e.au)
                )}
            </p>
            <p class="absp-resume">
                {format!(
                    "Acceptées : {} · faites : {} · restantes : {}",
                    libelle_heures(c.heures_acceptees),
                    libelle_heures(c.heures_faites),
                    libelle_heures(c.heures_restantes)
                )}
            </p>
            <meter class="absp-jauge" min="0" max="100" value=part aria-label=format!("Volume engagé consommé : {part} %")>
                {format!("{part} %")}
            </meter>
            <p class="absp-carte__detail">
                {format!(
                    "{} sollicitation(s) · {} acceptée(s) · {} refusée(s) dont {} pour un motif légitime.",
                    c.sollicitees, c.acceptees, c.refusees, c.refusees_motif_legitime
                )}
            </p>
            {c.volume_atteint.then(|| view! {
                <Alerte niveau=Niveau::Info titre="Le volume engagé est atteint ou dépassé : c'est une alerte, pas un blocage — votre accord vaut."/>
            })}
            {soi.then(|| view! { <p class="absp-carte__detail">{libelle_position(&c.position)}</p> })}
            {actif.then(|| view! {
                <p class="absp-carte__actions">
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert_modif.set(true))>"Modifier le volume"</Bouton>
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert_fin.set(true))>"Mettre fin"</Bouton>
                </p>
            })}
            <DialogueModifierEngagement id=id volume=e.volume_heures statut=e.statut.clone() ouvert=ouvert_modif ecriture=ecriture/>
            <ConfirmationDetaillee
                ouvert=ouvert_fin
                id=format!("absp-dlg-fin-engagement-{id}")
                titre="Mettre fin à l'engagement"
                message="L'engagement sort du vivier : on ne vous sollicitera plus pour le remplacement de courte durée. Les remplacements déjà acceptés restent dus."
                libelle_confirmer="Mettre fin"
                on_confirmer=Callback::new(move |_| {
                    ecriture.lancer(async move {
                        session::client()
                            .absp_modifier_engagement_rcd(id, &ModifierEngagementRcdBody { terminer: true, ..Default::default() })
                            .await?;
                        Ok("Engagement terminé.".to_string())
                    });
                })
            />
        </section>
    }
}

#[component]
fn DialogueModifierEngagement(
    id: Uuid,
    volume: f64,
    statut: String,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let vol = RwSignal::new(format!("{volume}").replace('.', ","));
    let stat = RwSignal::new(statut);
    let erreur = RwSignal::new(None::<String>);
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Ok(v) = vol.get_untracked().trim().replace(',', ".").parse::<f64>() else {
            erreur.set(Some("Le volume annuel est un nombre d'heures.".into()));
            return;
        };
        if !(v.is_finite() && v > 0.0 && v <= 500.0) {
            erreur.set(Some(
                "Le volume annuel va de plus de 0 à 500 heures.".into(),
            ));
            return;
        }
        erreur.set(None);
        ouvert.set(false);
        let corps = ModifierEngagementRcdBody {
            volume_heures: Some((v * 10.0).round() / 10.0),
            statut: Some(stat.get_untracked()),
            ..Default::default()
        };
        ecriture.lancer(async move {
            session::client()
                .absp_modifier_engagement_rcd(id, &corps)
                .await?;
            Ok("Engagement modifié.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Modifier l'engagement" id=format!("absp-dlg-modif-engagement-{id}")>
            <form class="absp-formulaire" on:submit=envoyer>
                <ChampTexte id=format!("absp-modif-vol-{id}") libelle="Volume annuel (heures)" valeur=vol inputmode="decimal" erreur=erreur autocomplete="off"/>
                <Selection id=format!("absp-modif-statut-{id}") libelle="Statut" valeur=stat options=options_statut()/>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

/// S'engager (soi-même) ou engager un collègue avec son accord (`rcd.gerer`).
#[component]
fn DialogueEngagement(
    soi_meme: bool,
    noms: LocalResource<Noms>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
    auj: NaiveDate,
) -> impl IntoView {
    let agent = RwSignal::new(String::new());
    let volume = RwSignal::new(String::new());
    let statut = RwSignal::new("pacte".to_string());
    let du = RwSignal::new(jour_texte(auj));
    let au = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieEngagement {
            agent: if soi_meme {
                String::new()
            } else {
                agent.get_untracked()
            },
            volume: volume.get_untracked(),
            statut: statut.get_untracked(),
            du: du.get_untracked(),
            au: au.get_untracked(),
        };
        if !soi_meme && saisie.agent.trim().is_empty() {
            erreur.set(Some("Choisissez l'agent engagé.".into()));
            return;
        }
        let corps = match composer_engagement(&saisie) {
            Ok(c) => c,
            Err(m) => {
                erreur.set(Some(m));
                return;
            }
        };
        erreur.set(None);
        ouvert.set(false);
        ecriture.lancer(async move {
            session::client().absp_engager_rcd(&corps).await?;
            Ok(if soi_meme {
                "Vous êtes volontaire. Vos créneaux se tiennent dans l'emploi du temps.".to_string()
            } else {
                "Volontaire engagé.".to_string()
            })
        });
    };
    let (titre, id) = if soi_meme {
        ("Me porter volontaire", "absp-dlg-engager-moi")
    } else {
        ("Engager un volontaire", "absp-dlg-engager-autre")
    };
    view! {
        <Dialogue ouvert=ouvert titre=titre id=id>
            <form class="absp-formulaire" on:submit=envoyer>
                {(!soi_meme).then(|| view! {
                    <p class="texte-attenue">"À faire avec l'accord de la personne : un volontariat ne s'impose pas."</p>
                    {move || {
                        let mut o = vec![(String::new(), "Choisir un agent".to_string())];
                        o.extend(noms.get().unwrap_or_default().options());
                        view! { <Selection id="absp-engager-agent" libelle="Agent" valeur=agent options=o requis=true/> }
                    }}
                })}
                <ChampTexte id=format!("{id}-volume") libelle="Volume annuel (heures)" valeur=volume inputmode="decimal" requis=true autocomplete="off"/>
                <Selection id=format!("{id}-statut") libelle="Statut" valeur=statut options=options_statut()/>
                <div class="absp-dates">
                    <ChampTexte id=format!("{id}-du") libelle="Du" valeur=du genre="date" requis=true/>
                    <ChampTexte id=format!("{id}-au") libelle="Au" valeur=au genre="date" requis=true erreur=erreur/>
                </div>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

// ───────────────────────────── Vivier ─────────────────────────────

#[component]
fn SectionVivier(
    version: RwSignal<u32>,
    noms: LocalResource<Noms>,
    ecriture: Ecriture,
    auj: NaiveDate,
) -> impl IntoView {
    let vivier = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_vivier_rcd().await }
    });
    let ouvert = RwSignal::new(false);
    view! {
        {move || match vivier.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement du vivier…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| vivier.refetch())/> }.into_any(),
            Some(Ok(v)) => view! {
                <section class="absp-section" aria-labelledby="absp-vivier">
                    <h2 id="absp-vivier" class="vs-section-titre">"Les volontaires de l'année"</h2>
                    <ul class="absp-compteurs" aria-label="Chiffres du vivier">
                        <li><Badge libelle=format!("{} volontaire(s)", v.volontaires)/></li>
                        <li><Badge libelle=format!("Volume engagé : {}", libelle_heures(v.volume_engage))/></li>
                        <li><Badge libelle=format!("Heures acceptées : {}", libelle_heures(v.heures_acceptees))/></li>
                    </ul>
                    <p class="absp-actions">
                        <Bouton on_click=Callback::new(move |_| ouvert.set(true))>"Engager un volontaire"</Bouton>
                    </p>
                    {if v.compteurs.is_empty() {
                        view! {
                            <EtatVide titre="Aucun volontaire" message="Lancez la campagne de volontariat depuis le plan de l'année."/>
                        }
                        .into_any()
                    } else {
                        view! {
                            <ul class="absp-liste">
                                {v.compteurs.into_iter().map(|c| view! {
                                    <li><CarteCompteur c=c ecriture=ecriture soi=false noms=noms/></li>
                                }).collect_view()}
                            </ul>
                        }
                        .into_any()
                    }}
                </section>
            }
            .into_any(),
        }}
        <DialogueEngagement soi_meme=false noms=noms ouvert=ouvert ecriture=ecriture auj=auj/>
    }
}

// ───────────────────────────── Remontée hebdomadaire ─────────────────────────────

#[component]
fn SectionRemontee(version: RwSignal<u32>, ecriture: Ecriture, auj: NaiveDate) -> impl IntoView {
    let liste = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_remontees_rcd().await }
    });
    let semaine = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    let preparer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let t = semaine.get_untracked();
        let choisie = if t.trim().is_empty() {
            None
        } else {
            match NaiveDate::parse_from_str(t.trim(), "%Y-%m-%d") {
                Ok(d) => Some(lundi_de(d)),
                Err(_) => {
                    erreur.set(Some("Date de semaine invalide.".into()));
                    return;
                }
            }
        };
        erreur.set(None);
        ecriture.lancer(async move {
            let r = session::client()
                .absp_preparer_remontee_rcd(choisie)
                .await?;
            Ok(format!(
                "Remontée préparée : semaine {}.",
                libelle_semaine(r.semaine)
            ))
        });
    };
    view! {
        <section class="absp-section" aria-labelledby="absp-remontee">
            <h2 id="absp-remontee" class="vs-section-titre">"Remontée hebdomadaire"</h2>
            <p class="texte-attenue">
                "Aucun nom, aucune nature fine : seulement la discipline, le niveau, la catégorie nationale du motif et le mode de continuité. Contrôlez, puis laissez partir. Le flux vers le ministère n'est pas encore branché : l'export CSV est le tableau équivalent."
            </p>
            <form class="absp-barre" on:submit=preparer>
                <ChampTexte
                    id="absp-remontee-semaine"
                    libelle="Semaine à préparer (un jour de la semaine)"
                    valeur=semaine
                    genre="date"
                    aide=format!("Vide : la semaine passée ({}).", libelle_semaine(semaine_passee(auj)))
                    erreur=erreur
                />
                <Bouton soumettre=true>"Préparer la remontée"</Bouton>
            </form>
            {move || match liste.get() {
                None => view! { <Squelette lignes=4 libelle="Chargement des remontées…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(v)) if v.is_empty() => view! {
                    <EtatVide titre="Aucune remontée" message="Préparez la semaine passée pour la contrôler."/>
                }
                .into_any(),
                Some(Ok(v)) => view! {
                    <ul class="absp-liste">
                        {v.into_iter().map(|r| view! { <li><CarteRemontee r=r ecriture=ecriture/></li> }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn CarteRemontee(r: RemonteeRcdDto, ecriture: Ecriture) -> impl IntoView {
    let id = r.id;
    let actions = actions_remontee(&r.etat);
    let (prevues, absence, remplacees) = totaux_remontee(&r);
    let taux = taux_remplacement(absence, remplacees);
    let semaine = libelle_semaine(r.semaine);
    let ouvert_transmission = RwSignal::new(false);
    let ouvert_rejet = RwSignal::new(false);
    let erreur_export = RwSignal::new(None::<String>);
    let groupes: Vec<(
        String,
        Vec<ent_types::absences_personnels::LigneRemonteeRcdDto>,
    )> = lignes_par_discipline(&r)
        .into_iter()
        .map(|(d, l)| (d, l.into_iter().cloned().collect()))
        .collect();
    let message_transmission = format!("Laisser partir la remontée {semaine} ?");
    let message_rejet = format!("Rejeter la remontée {semaine} ?");
    let suivre = move |suite: SuiteRemontee, message: &'static str| {
        ecriture.lancer(async move {
            session::client().absp_suite_remontee_rcd(id, suite).await?;
            Ok(message.to_string())
        });
    };
    view! {
        <section class="absp-carte" aria-label=format!("Remontée {semaine}")>
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{format!("Semaine {semaine}")}</strong>
                <Badge libelle=libelle_etat_remontee(&r.etat) ton=ton_etat_remontee(&r.etat)/>
            </div>
            <p class="absp-resume">
                {format!(
                    "Heures prévues : {} · d'absence déclarées : {} · remplacées : {}{}",
                    libelle_heures(prevues),
                    libelle_heures(absence),
                    libelle_heures(remplacees),
                    taux.map(|t| format!(" ({t} %)")).unwrap_or_default()
                )}
            </p>
            {groupes.is_empty().then(|| view! { <p class="absp-carte__detail">"Aucune absence à remonter cette semaine."</p> })}
            {(!groupes.is_empty()).then(|| view! {
                <details>
                    <summary>"Voir le détail par discipline"</summary>
                    {groupes.into_iter().map(|(d, lignes)| view! {
                        <h3 class="absp-sous-titre">{d}</h3>
                        <ul class="absp-liste absp-liste--serree">
                            {lignes.into_iter().map(|l| view! {
                                <li class="absp-ligne">
                                    <span>
                                        {format!(
                                            "{} · {} · {}",
                                            l.niveau_code,
                                            libelle_categorie_nationale(&l.categorie_nationale),
                                            libelle_mode_continuite(&l.mode_continuite)
                                        )}
                                    </span>
                                    <span>
                                        {format!(
                                            "{} prévues · {} d'absence · {} remplacées",
                                            libelle_heures(l.heures_prevues),
                                            libelle_heures(l.heures_absence),
                                            libelle_heures(l.heures_remplacees)
                                        )}
                                    </span>
                                </li>
                            }).collect_view()}
                        </ul>
                    }).collect_view()}
                </details>
            })}
            <p class="absp-carte__actions">
                {actions.controler.then(|| view! {
                    <Bouton taille=Taille::Compacte on_click=Callback::new(move |_| suivre(SuiteRemontee::Controle, "Remontée contrôlée."))>"Marquer contrôlée"</Bouton>
                })}
                {actions.transmettre.then(|| view! {
                    <Bouton taille=Taille::Compacte on_click=Callback::new(move |_| ouvert_transmission.set(true))>"Laisser partir"</Bouton>
                })}
                {actions.exporter.then(|| view! {
                    <Bouton taille=Taille::Compacte variante=Variante::Secondaire
                        on_click=Callback::new(move |_| {
                            lancer_export(
                                async move { session::client().absp_remontee_csv(id).await },
                                "remontee-rcd.csv",
                                erreur_export,
                                ecriture,
                            );
                        })>
                        "Exporter (CSV)"
                    </Bouton>
                })}
                {actions.rejeter.then(|| view! {
                    <Bouton taille=Taille::Compacte variante=Variante::Discret on_click=Callback::new(move |_| ouvert_rejet.set(true))>"Rejeter"</Bouton>
                })}
            </p>
            {move || erreur_export.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            <ConfirmationDetaillee
                ouvert=ouvert_transmission
                id=format!("absp-dlg-transmission-{id}")
                titre="Laisser partir la remontée"
                message=Signal::derive(move || message_transmission.clone())
                rappel=Signal::derive(|| "Elle ne contient aucun nom. Le flux vers le ministère n'est pas branché : exportez le tableau pour le transmettre.".to_string())
                libelle_confirmer="Laisser partir"
                on_confirmer=Callback::new(move |_| suivre(SuiteRemontee::Transmission, "Remontée marquée transmise."))
            />
            <ConfirmationDetaillee
                ouvert=ouvert_rejet
                id=format!("absp-dlg-rejet-{id}")
                titre="Rejeter la remontée"
                message=Signal::derive(move || message_rejet.clone())
                rappel=Signal::derive(|| "Elle ne partira pas ; préparez-la de nouveau après correction des données.".to_string())
                libelle_confirmer="Rejeter"
                danger=true
                on_confirmer=Callback::new(move |_| suivre(SuiteRemontee::Rejet, "Remontée rejetée."))
            />
        </section>
    }
}
