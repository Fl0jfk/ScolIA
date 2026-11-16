//! « Mes heures » (ABP-200, 201, 202, 219) : les heures constatées de l'agent connecté, mois par mois et
//! par nature avec leur état, **ce qui manque** (séance remplacée terminée sans appel : à attester) et la
//! contestation d'un geste.
//!
//! Règle R-AP28 : l'agent ne **déclare** jamais une heure. Une heure naît d'un fait (appel fait, attestation
//! d'une séance que le serveur dit assurée). Ici on ne peut qu'attester une séance listée par le serveur et
//! contester une heure existante. Aucune nature d'absence n'est lue : seulement des heures du remplaçant.

use ent_types::absences_personnels::{ContesterHeureBody, HeureDto, SeanceAAttesterDto};
use ent_web_ui::{
    Alerte, Badge, Bouton, Dialogue, EtatVide, Niveau, Selection, Squelette, Taille, Variante,
    ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use super::commun::GardePersonnel;
use super::modele_heures::{
    duree_effective, libelle_duree, libelle_etat_heure, libelle_mois, libelle_nature_heure,
    libelle_preuve, nb_a_regler, par_mois, peut_contester, ton_etat_heure, totaux_par_nature,
    RAISONS_HEURE,
};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_longue;

#[component]
pub fn PageMesHeures() -> impl IntoView {
    view! {
        <Title text="Mes heures"/>
        <GardePersonnel
            titre="Mes heures"
            sous_titre="Les heures constatées pour vous, mois par mois, et ce qui manque."
        >
            <Contenu/>
        </GardePersonnel>
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| version.update(|v| *v += 1)));
    let liste = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_mes_heures(None).await }
    });
    let annuel = LocalResource::new(move || {
        let _ = version.get();
        async { session::client().absp_mon_etat_annuel().await }
    });
    let a_attester: RwSignal<Option<SeanceAAttesterDto>> = RwSignal::new(None);
    let ouvert_attestation = RwSignal::new(false);
    let a_contester: RwSignal<Option<HeureDto>> = RwSignal::new(None);
    let ouvert_contestation = RwSignal::new(false);
    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || match annuel.get() {
            Some(Ok(a)) if a.total_minutes > 0 => view! {
                <p class="absp-resume" role="status">
                    {format!("Cette année scolaire : {} constatées pour vous.", libelle_duree(a.total_minutes))}
                </p>
            }
            .into_any(),
            _ => ().into_any(),
        }}
        {move || match liste.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de vos heures…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
            Some(Ok(m)) => {
                let manque = m.a_attester.clone();
                let heures = m.heures.clone();
                let contestees = nb_a_regler(&heures);
                view! {
                    {(!manque.is_empty()).then(|| view! {
                        <section class="absp-section" aria-labelledby="absp-attester">
                            <h2 id="absp-attester" class="vs-section-titre">"Séances remplacées, sans heure"</h2>
                            <p class="texte-attenue">
                                "L'appel n'a pas été fait sur ces séances que vous avez remplacées. Si vous les avez bien assurées, attestez-le : l'heure est alors constatée."
                            </p>
                            <ul class="absp-liste">
                                {manque.into_iter().map(|s| view! {
                                    <CarteAAttester s=s choix=a_attester ouvert=ouvert_attestation/>
                                }).collect_view()}
                            </ul>
                        </section>
                    })}
                    {(contestees > 0).then(|| view! {
                        <Alerte
                            niveau=Niveau::Avertissement
                            titre=format!("{contestees} heure(s) contestée(s) en attente d'une correction par la direction ou le secrétariat.")
                        />
                    })}
                    {if heures.is_empty() {
                        view! {
                            <EtatVide
                                titre="Aucune heure constatée pour le moment"
                                message="Une heure apparaît ici quand une séance que vous avez remplacée est assurée (appel fait), ou quand la direction saisit une mission ou une vacation."
                            />
                        }
                        .into_any()
                    } else {
                        par_mois(&heures)
                            .into_iter()
                            .map(|(mois, g)| view! { <SectionMois mois=mois heures=g choix=a_contester ouvert=ouvert_contestation/> })
                            .collect_view()
                            .into_any()
                    }}
                }
                .into_any()
            }
        }}
        <DialogueAttestation choix=a_attester ouvert=ouvert_attestation ecriture=ecriture/>
        <DialogueContestation choix=a_contester ouvert=ouvert_contestation ecriture=ecriture/>
    }
}

fn libelle_a_attester(s: &SeanceAAttesterDto) -> String {
    format!(
        "{} · {}–{} · {}",
        date_longue(s.le),
        s.heure_debut.format("%H:%M"),
        s.heure_fin.format("%H:%M"),
        s.matiere
    )
}

#[component]
fn CarteAAttester(
    s: SeanceAAttesterDto,
    choix: RwSignal<Option<SeanceAAttesterDto>>,
    ouvert: RwSignal<bool>,
) -> impl IntoView {
    let resume = libelle_a_attester(&s);
    let aria = format!("Attester la séance : {resume}");
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{resume}</strong>
                <Badge libelle="Sans appel" ton=ent_web_ui::TonEtat::Avertissement/>
            </div>
            <p class="absp-carte__actions">
                <Bouton
                    taille=Taille::Compacte
                    libelle_accessible=aria
                    on_click=Callback::new(move |_| {
                        choix.set(Some(s.clone()));
                        ouvert.set(true);
                    })
                >
                    "J'ai assuré cette séance"
                </Bouton>
            </p>
        </li>
    }
}

#[component]
fn SectionMois(
    mois: chrono::NaiveDate,
    heures: Vec<HeureDto>,
    choix: RwSignal<Option<HeureDto>>,
    ouvert: RwSignal<bool>,
) -> impl IntoView {
    let id = format!("absp-mois-{}", mois.format("%Y-%m"));
    let id_section = id.clone();
    let totaux = totaux_par_nature(&heures);
    view! {
        <section class="absp-section" aria-labelledby=id_section>
            <h2 id=id class="vs-section-titre">{libelle_mois(mois)}</h2>
            <ul class="absp-compteurs" aria-label="Total par nature">
                {totaux.into_iter().map(|(n, m)| view! {
                    <li><Badge libelle=format!("{} : {}", libelle_nature_heure(n), libelle_duree(m))/></li>
                }).collect_view()}
            </ul>
            <ul class="absp-liste">
                {heures.into_iter().map(|h| view! { <CarteHeure h=h choix=choix ouvert=ouvert/> }).collect_view()}
            </ul>
        </section>
    }
}

#[component]
fn CarteHeure(
    h: HeureDto,
    choix: RwSignal<Option<HeureDto>>,
    ouvert: RwSignal<bool>,
) -> impl IntoView {
    let resume = format!(
        "{} · {}",
        date_longue(h.le),
        libelle_nature_heure(&h.nature)
    );
    let effective = duree_effective(&h);
    let regul = (h.regularisation_minutes != 0).then(|| {
        format!(
            "dont régularisation : {}",
            libelle_duree(i64::from(h.regularisation_minutes))
        )
    });
    let contestable = peut_contester(&h.etat);
    let aria = format!("Contester l'heure : {resume}");
    let h_choix = h.clone();
    view! {
        <li class="absp-carte">
            <div class="absp-carte__tete">
                <strong class="absp-carte__titre">{resume}</strong>
                <Badge libelle=libelle_etat_heure(&h.etat) ton=ton_etat_heure(&h.etat)/>
            </div>
            <p class="absp-carte__detail">
                {format!("{} · {}", libelle_duree(effective), libelle_preuve(&h.preuve))}
            </p>
            {regul.map(|r| view! { <p class="absp-carte__detail">{r}</p> })}
            {(h.etat == "contestee").then(|| view! {
                <p class="absp-carte__detail">"Contestée : en attente d'une correction."</p>
            })}
            {(h.etat == "figee" || h.etat == "exportee").then(|| view! {
                <p class="absp-carte__detail">
                    "Cette heure est figée : une erreur se corrige par une régularisation sur l'état suivant. Écrivez à la direction ou au secrétariat."
                </p>
            })}
            {contestable.then(|| view! {
                <p class="absp-carte__actions">
                    <Bouton
                        variante=Variante::Secondaire
                        taille=Taille::Compacte
                        libelle_accessible=aria
                        on_click=Callback::new(move |_| {
                            choix.set(Some(h_choix.clone()));
                            ouvert.set(true);
                        })
                    >
                        "Contester"
                    </Bouton>
                </p>
            })}
        </li>
    }
}

/// Attestation d'une séance assurée sans appel (ABP-202) : un fait attesté, donc confirmé.
#[component]
fn DialogueAttestation(
    choix: RwSignal<Option<SeanceAAttesterDto>>,
    ouvert: RwSignal<bool>,
    ecriture: Ecriture,
) -> impl IntoView {
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(s) = choix.get_untracked() else {
            return;
        };
        ouvert.set(false);
        let id: Uuid = s.seance_id;
        ecriture.lancer(async move {
            session::client().absp_attester(id).await?;
            Ok("Séance attestée : l'heure est constatée.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Attester la séance" id="absp-dlg-attester">
            <form class="absp-formulaire" on:submit=envoyer>
                <p>
                    {move || choix.get().map(|s| format!("Vous attestez avoir assuré : {}.", libelle_a_attester(&s)))}
                </p>
                <p class="texte-attenue">
                    "L'heure sera constatée. Une fausse attestation se corrige par la direction, avec une trace."
                </p>
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"J'atteste"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

/// Contestation d'une heure (ABP-201) : motif codé, note courte sans information de santé.
#[component]
fn DialogueContestation(
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
        let corps = ContesterHeureBody {
            raison_code: raison.get_untracked(),
            note: (!n.trim().is_empty()).then(|| n.trim().to_string()),
        };
        ouvert.set(false);
        note.set(String::new());
        ecriture.lancer(async move {
            session::client().absp_contester_heure(h.id, &corps).await?;
            Ok("Heure contestée : la direction ou le secrétariat la corrigera.".to_string())
        });
    };
    view! {
        <Dialogue ouvert=ouvert titre="Contester une heure" id="absp-dlg-contester">
            <form class="absp-formulaire" on:submit=envoyer>
                <p>
                    {move || choix.get().map(|h| format!(
                        "{} · {} · {}",
                        date_longue(h.le),
                        libelle_nature_heure(&h.nature),
                        libelle_duree(duree_effective(&h))
                    ))}
                </p>
                <Selection id="absp-contester-raison" libelle="Pourquoi ?" valeur=raison options=options/>
                <ZoneTexte
                    id="absp-contester-note"
                    libelle="Précision (facultatif)"
                    valeur=note
                    aide="500 caractères au plus. Aucune information de santé."
                />
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert.set(false))>"Retour"</Bouton>
                    <Bouton soumettre=true>"Contester"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}
