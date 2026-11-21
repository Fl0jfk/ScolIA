//! Délégations de droits datées (socle 2, S2-5, ADM-04) : « mes délégations » pour tout le
//! personnel ; liste, octroi et révocation pour qui détient `delegations.gerer` (administration
//! et direction par défaut). La date de fin est obligatoire ; les droits se choisissent dans une
//! liste fermée d'actions sûres. Le serveur reste l'autorité : on ne délègue que ce que l'on
//! détient, et un refus (403) est affiché tel quel.

pub mod modele;

use ent_types::edt_s2::{DelegationDto, DelegationSaisieDto};
use ent_web_api::ApiError;
use ent_web_ui::{
    AccesRefuse, Badge, Bouton, Case, ChampTexte, EnTetePage, EtatVide, ResumeErreurs, Selection,
    Squelette, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::confirmation_detaillee::ConfirmationDetaillee;
use crate::noyau::coquille::{use_droits, use_droits_moteur};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use modele::{
    composer, libelle_droits, libelle_etat, libelle_periode, libelle_source, nb_actives,
    peut_revoquer, resume_octroi, ton_etat, trier, SaisieDelegation, ACTIONS_DELEGABLES,
};

#[component]
pub fn PageDelegations() -> impl IntoView {
    let droits = use_droits();
    let moteur = use_droits_moteur();
    let ctx = session::use_session();
    view! {
        <Title text="Délégations"/>
        <EnTetePage
            surtitre="Administration".to_string()
            titre="Délégations de droits".to_string()
            sous_titre="Des droits confiés à une personne pour une durée limitée.".to_string()
        />
        {move || match ctx.moi.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des délégations…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| ctx.moi.refetch())/> }.into_any(),
            Some(Ok(_)) if !droits.get().personnel => {
                view! { <AccesRefuse message="Les délégations sont réservées au personnel de l'établissement."/> }.into_any()
            }
            Some(Ok(_)) => view! {
                <MesDelegations/>
                {move || moteur.get().peut_gerer_delegations().then(|| view! { <Gestion/> })}
            }
            .into_any(),
        }}
    }
}

// ---------------------------------------------------------------------------
// Mes délégations
// ---------------------------------------------------------------------------

#[component]
fn MesDelegations() -> impl IntoView {
    let mes = LocalResource::new(|| async { session::client().mes_delegations().await.map(trier) });
    view! {
        <section class="dlg-section" aria-labelledby="dlg-mes">
            <h2 id="dlg-mes" class="vs-section-titre">"Mes délégations"</h2>
            {move || match mes.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement de vos délégations…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| mes.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucune délégation reçue" message="Personne ne vous a confié de droits supplémentaires."/>
                }
                .into_any(),
                Some(Ok(l)) => view! { <ListeDelegations liste=l recues=true revoquer=None/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn ListeDelegations(
    liste: Vec<DelegationDto>,
    /// Vrai : délégations reçues (on montre qui les a accordées) ; faux : accordées (on montre le bénéficiaire).
    recues: bool,
    revoquer: Option<Callback<(Uuid, String)>>,
) -> impl IntoView {
    view! {
        <ul class="dlg-liste">
            {liste
                .into_iter()
                .map(|d| {
                    let personne = if recues {
                        format!("Accordée par {}", d.delegant_nom.clone().unwrap_or_else(|| "la direction".into()))
                    } else {
                        d.delegue_nom.clone().unwrap_or_else(|| "Personne inconnue".into())
                    };
                    let nom_delegue = d.delegue_nom.clone().unwrap_or_else(|| "cette personne".into());
                    let id = d.id;
                    let peut = revoquer.is_some() && peut_revoquer(&d);
                    view! {
                        <li class="dlg-carte">
                            <div class="dlg-carte__tete">
                                <strong>{personne}</strong>
                                <Badge libelle=libelle_etat(&d.etat) ton=ton_etat(&d.etat)/>
                            </div>
                            <p class="dlg-carte__droits">{libelle_droits(&d)}</p>
                            <p class="dlg-carte__detail">{libelle_periode(&d)}</p>
                            <p class="dlg-carte__detail">{libelle_source(&d.source)}</p>
                            {(!d.motif.trim().is_empty()).then(|| view! {
                                <p class="dlg-carte__detail">{format!("Motif : {}", d.motif.trim())}</p>
                            })}
                            {revoquer.filter(|_| peut).map(|cb| view! {
                                <div class="dlg-carte__actions">
                                    <Bouton
                                        variante=Variante::Danger
                                        taille=ent_web_ui::Taille::Compacte
                                        libelle_accessible=format!("Révoquer la délégation de {nom_delegue}")
                                        on_click=Callback::new({
                                            let nom = nom_delegue.clone();
                                            move |_| cb.run((id, nom.clone()))
                                        })
                                    >
                                        "Révoquer"
                                    </Bouton>
                                </div>
                            })}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}

// ---------------------------------------------------------------------------
// Gestion (delegations.gerer)
// ---------------------------------------------------------------------------

#[component]
fn Gestion() -> impl IntoView {
    let liste =
        LocalResource::new(|| async { session::client().delegations(false).await.map(trier) });
    // Compteur incrémenté à chaque octroi réussi : le formulaire se vide alors.
    let version = RwSignal::new(0u32);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        version.update(|v| *v += 1);
        liste.refetch();
    }));
    let cible = RwSignal::new(None::<(Uuid, String)>);
    let ouvert_revocation = RwSignal::new(false);
    let revoquer = Callback::new(move |c: (Uuid, String)| {
        cible.set(Some(c));
        ouvert_revocation.set(true);
    });
    let confirmer_revocation = Callback::new(move |_| {
        if let Some((id, nom)) = cible.get_untracked() {
            ecriture.lancer(async move {
                session::client().revoquer_delegation(id).await?;
                Ok(format!("Délégation de {nom} révoquée."))
            });
        }
    });
    let message = Signal::derive(move || match cible.get() {
        Some((_, nom)) => {
            format!("Révoquer la délégation de {nom} ? Les droits confiés cessent immédiatement.")
        }
        None => String::new(),
    });
    view! {
        <RetourEcriture ecriture=ecriture/>
        <section class="dlg-section" aria-labelledby="dlg-etab">
            <h2 id="dlg-etab" class="vs-section-titre">"Délégations de l'établissement"</h2>
            {move || match liste.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des délégations…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucune délégation" message="Accordez des droits à une personne avec le formulaire ci-dessous."/>
                }
                .into_any(),
                Some(Ok(l)) => {
                    let actives = nb_actives(&l);
                    view! {
                        <p class="texte-attenue" role="status">
                            {match actives {
                                0 => "Aucune délégation en cours.".to_string(),
                                1 => "1 délégation en cours.".to_string(),
                                n => format!("{n} délégations en cours."),
                            }}
                        </p>
                        <ListeDelegations liste=l recues=false revoquer=Some(revoquer)/>
                    }
                    .into_any()
                }
            }}
        </section>
        <ConfirmationDetaillee
            ouvert=ouvert_revocation
            id="dlg-revoquer"
            titre="Révoquer la délégation"
            message=message
            libelle_confirmer="Révoquer"
            danger=true
            on_confirmer=confirmer_revocation
        />
        <FormulaireOctroi ecriture=ecriture version=version/>
    }
}

#[component]
fn FormulaireOctroi(ecriture: Ecriture, version: RwSignal<u32>) -> impl IntoView {
    let personnel = LocalResource::new(|| async {
        let mut p = session::client().personnel("").await?;
        p.sort_by(|a, b| a.label.cmp(&b.label));
        Ok::<_, ApiError>(p)
    });
    let delegue = RwSignal::new(String::new());
    let debut = RwSignal::new(String::new());
    let fin = RwSignal::new(String::new());
    let motif = RwSignal::new(String::new());
    let cases: Vec<(&'static str, &'static str, RwSignal<bool>)> = ACTIONS_DELEGABLES
        .iter()
        .map(|(c, l)| (*c, *l, RwSignal::new(false)))
        .collect();
    let cases_lues = cases.clone();
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let a_confirmer = RwSignal::new(None::<(DelegationSaisieDto, String)>);
    let ouvert = RwSignal::new(false);

    // Octroi réussi : le formulaire est vidé.
    {
        let cases = cases.clone();
        Effect::new(move |_| {
            version.track();
            delegue.set(String::new());
            debut.set(String::new());
            fin.set(String::new());
            motif.set(String::new());
            for (_, _, c) in &cases {
                c.set(false);
            }
        });
    }

    let erreur_de = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs
                .get()
                .into_iter()
                .find(|(c, _)| c == champ)
                .map(|(_, m)| m)
        })
    };
    let preparer = Callback::new(move |_| {
        let saisie = SaisieDelegation {
            delegue: delegue.get_untracked(),
            actions: cases_lues
                .iter()
                .filter(|(_, _, c)| c.get_untracked())
                .map(|(code, _, _)| (*code).to_string())
                .collect(),
            debut: debut.get_untracked(),
            fin: fin.get_untracked(),
            motif: motif.get_untracked(),
        };
        match composer(&saisie, aujourdhui()) {
            Ok(corps) => {
                erreurs.set(Vec::new());
                let nom = personnel
                    .get_untracked()
                    .and_then(Result::ok)
                    .and_then(|p| {
                        p.into_iter()
                            .find(|x| x.user_id == corps.delegue_user_id)
                            .map(|x| x.label)
                    })
                    .unwrap_or_else(|| "Cette personne".into());
                a_confirmer.set(Some((corps, nom)));
                ouvert.set(true);
            }
            Err(e) => erreurs.set(e),
        }
    });
    let confirmer = Callback::new(move |_| {
        if let Some((corps, nom)) = a_confirmer.get_untracked() {
            ecriture.lancer(async move {
                session::client().accorder_delegation(&corps).await?;
                Ok(format!("Délégation accordée à {nom}."))
            });
        }
    });
    let message = Signal::derive(move || match a_confirmer.get() {
        Some((corps, nom)) => resume_octroi(&nom, &corps),
        None => String::new(),
    });
    view! {
        <section class="dlg-section" aria-labelledby="dlg-octroi">
            <h2 id="dlg-octroi" class="vs-section-titre">"Accorder une délégation"</h2>
            <form
                class="dlg-formulaire"
                on:submit=move |ev| {
                    ev.prevent_default();
                    preparer.run(());
                }
            >
                <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
                {move || match personnel.get() {
                    None => view! { <Squelette lignes=1 libelle="Chargement du personnel…"/> }.into_any(),
                    Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| personnel.refetch())/> }.into_any(),
                    Some(Ok(p)) => {
                        let mut options = vec![(String::new(), "Choisir…".to_string())];
                        options.extend(p.into_iter().map(|x| (x.user_id.to_string(), x.label)));
                        view! {
                            <Selection
                                id="dlg-personne"
                                libelle="Personne qui reçoit les droits"
                                valeur=delegue
                                options=options
                                requis=true
                                erreur=erreur_de("dlg-personne")
                            />
                        }
                        .into_any()
                    }
                }}
                <fieldset id="dlg-actions" class="dlg-droits">
                    <legend>"Droits délégués (sur tout l'établissement)"</legend>
                    {cases
                        .iter()
                        .map(|(code, libelle, coche)| view! {
                            <Case id=format!("dlg-action-{code}") libelle=*libelle coche=*coche/>
                        })
                        .collect_view()}
                    {move || erreur_de("dlg-actions").get().map(|m| view! { <p class="ui-champ__erreur">{m}</p> })}
                </fieldset>
                <ChampTexte
                    id="dlg-debut"
                    libelle="Début"
                    genre="date"
                    valeur=debut
                    aide="Facultatif : aujourd'hui par défaut."
                    erreur=erreur_de("dlg-debut")
                />
                <ChampTexte
                    id="dlg-fin"
                    libelle="Fin (incluse)"
                    genre="date"
                    valeur=fin
                    requis=true
                    aide="Obligatoire : une délégation dure 366 jours au plus."
                    erreur=erreur_de("dlg-fin")
                />
                <ZoneTexte
                    id="dlg-motif"
                    libelle="Motif"
                    valeur=motif
                    requis=true
                    erreur=erreur_de("dlg-motif")
                />
                <Bouton soumettre=true en_cours=ecriture.en_cours>"Accorder la délégation"</Bouton>
            </form>
        </section>
        <ConfirmationDetaillee
            ouvert=ouvert
            id="dlg-confirmer-octroi"
            titre="Accorder cette délégation ?"
            message=message
            rappel=Signal::derive(|| "Les droits s'appliquent dès le début de la période et cessent après la date de fin.".to_string())
            libelle_confirmer="Accorder"
            on_confirmer=confirmer
        />
    }
}
