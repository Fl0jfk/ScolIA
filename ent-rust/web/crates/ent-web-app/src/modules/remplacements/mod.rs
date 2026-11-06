//! Absences de personnel et remplacements (socle 2, S2-5) : pour chaque absence validée, les
//! séances touchées à traiter (proposer un remplaçant, annuler, rétablir) ; annulation en masse
//! avec aperçu obligatoire. Routes `/edt/absences`, `/edt/remplacants`, `/edt/remplacements`,
//! `/edt/annulations`, `/edt/exceptions-levee`, `/edt/annulation-masse`, `/rh/absences/{id}/valider`.
//!
//! Les droits (`edt.gerer`, `edt.annuler_masse`) sont décidés par le serveur : l'écran masque les
//! boutons d'écriture selon les droits effectifs du moteur et affiche le refus (403) tel quel.
//! Chaque remplacement ou annulation prévient les élèves et leurs familles : la confirmation le
//! rappelle explicitement.

pub mod modele;

use chrono::NaiveDate;
use ent_types::edt_s2::{AbsenceSeancesDto, RapportMasseDto, SeanceToucheeDto};
use ent_web_api::{ApiError, ClasseApi};
use ent_web_ui::{
    AccesRefuse, Alerte, Annonce, Badge, Bouton, Case, ChampTexte, Dialogue, EnTetePage, EtatVide,
    GroupeChoix, Niveau, ResumeErreurs, Squelette, Taille, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use crate::composants_metier::confirmation_detaillee::ConfirmationDetaillee;
use crate::noyau::coquille::{use_droits, use_droits_moteur};
use crate::noyau::ecriture::{message_echec, Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;
use modele::{
    apercu_valide, cle_idempotence, cmd_remplacement, composer_masse, libelle_classe_touchee,
    libelle_cours, libelle_dates, libelle_etat_seance, libelle_horaire, libelle_statut_absence,
    message_annulation, message_application, message_confirmation_masse, message_remplacement,
    message_retablissement, nom_candidat, options_candidats, periode_par_defaut, peut_retablir,
    peut_traiter, phrase_seance, resume_rapport, resume_traitement, signature, ton_statut_absence,
    total_a_traiter, trier_candidats, valider_periode, EtatSeance, SaisieMasse, PORTEE_CLASSES,
    PORTEE_ETABLISSEMENT, RAPPEL_PREVENUS,
};

#[component]
pub fn PageRemplacements() -> impl IntoView {
    let droits = use_droits();
    let ctx = session::use_session();
    view! {
        <Title text="Absences et remplacements"/>
        <EnTetePage
            surtitre="Emploi du temps".to_string()
            titre="Absences et remplacements".to_string()
            sous_titre="Absences du personnel, séances à traiter et annulations en masse.".to_string()
        />
        {move || match ctx.moi.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des absences…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| ctx.moi.refetch())/> }.into_any(),
            Some(Ok(_)) if !droits.get().personnel => {
                view! { <AccesRefuse message="Cet écran est réservé au personnel de l'établissement."/> }.into_any()
            }
            Some(Ok(_)) => view! { <Contenu/> }.into_any(),
        }}
    }
}

/// Actions proposées sur une séance touchée ; chaque rappel reçoit l'identifiant de l'absence.
#[derive(Clone, Copy)]
struct Actions {
    proposer: Callback<(Uuid, SeanceToucheeDto)>,
    annuler: Callback<(Uuid, SeanceToucheeDto)>,
    retablir: Callback<(Uuid, SeanceToucheeDto)>,
    valider: Callback<(Uuid, String)>,
    peut_ecrire: Signal<bool>,
}

type Absences = Result<Option<Vec<AbsenceSeancesDto>>, ApiError>;

#[component]
fn Contenu() -> impl IntoView {
    let moteur = use_droits_moteur();
    // Période posée après l'hydratation (jamais la date du jour dans le HTML du serveur).
    let periode = RwSignal::new(None::<(NaiveDate, NaiveDate)>);
    let du = RwSignal::new(String::new());
    let au = RwSignal::new(String::new());
    let erreur_periode = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let (d, a) = periode_par_defaut(aujourdhui());
        du.set(d.to_string());
        au.set(a.to_string());
        periode.set(Some((d, a)));
    });
    let absences: LocalResource<Absences> = LocalResource::new(move || {
        let p = periode.get();
        async move {
            match p {
                Some((d, a)) => session::client().absences_edt(d, a).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let recharger = Callback::new(move |_| absences.refetch());
    let ecriture = Ecriture::nouveau(recharger);

    // Séance visée par la boîte de dialogue en cours : (absence, séance).
    let cible = RwSignal::new(None::<(Uuid, SeanceToucheeDto)>);
    let ouvert_candidats = RwSignal::new(false);
    let ouvert_remplacement = RwSignal::new(false);
    let ouvert_annulation = RwSignal::new(false);
    let ouvert_retablissement = RwSignal::new(false);
    let ouvert_validation = RwSignal::new(false);
    let a_valider = RwSignal::new(None::<(Uuid, String)>);
    let tous = RwSignal::new(false);
    let choix = RwSignal::new(String::new());
    // Remplaçant retenu au moment de « Désigner » : (identifiant, nom).
    let retenu = RwSignal::new(None::<(String, String)>);

    let candidats = LocalResource::new(move || {
        let c = cible.get();
        let t = tous.get();
        let ouvert = ouvert_candidats.get();
        async move {
            match (c, ouvert) {
                (Some((_, s)), true) => session::client()
                    .remplacants(s.creneau_id, s.jour, t)
                    .await
                    .map(|l| Some(trier_candidats(l))),
                _ => Ok(None),
            }
        }
    });
    // Changer l'option « tous » change la liste : l'ancien choix n'a plus de sens.
    Effect::new(move |_| {
        tous.track();
        choix.set(String::new());
    });

    let actions = Actions {
        proposer: Callback::new(move |(a, s): (Uuid, SeanceToucheeDto)| {
            cible.set(Some((a, s)));
            tous.set(false);
            choix.set(String::new());
            ouvert_candidats.set(true);
        }),
        annuler: Callback::new(move |(a, s): (Uuid, SeanceToucheeDto)| {
            cible.set(Some((a, s)));
            ouvert_annulation.set(true);
        }),
        retablir: Callback::new(move |(a, s): (Uuid, SeanceToucheeDto)| {
            cible.set(Some((a, s)));
            ouvert_retablissement.set(true);
        }),
        valider: Callback::new(move |c: (Uuid, String)| {
            a_valider.set(Some(c));
            ouvert_validation.set(true);
        }),
        peut_ecrire: Signal::derive(move || moteur.get().peut_gerer_edt()),
    };

    let designer = Callback::new(move |_| {
        let id = choix.get_untracked();
        let nom = candidats
            .get_untracked()
            .and_then(Result::ok)
            .flatten()
            .and_then(|l| l.into_iter().find(|c| c.user_id.to_string() == id))
            .map(|c| nom_candidat(&c));
        if let Some(nom) = nom {
            retenu.set(Some((id, nom)));
            ouvert_candidats.set(false);
            ouvert_remplacement.set(true);
        }
    });
    let confirmer_remplacement = Callback::new(move |_| {
        if let (Some((absence, s)), Some((id, nom))) =
            (cible.get_untracked(), retenu.get_untracked())
        {
            if let Some(cmd) = cmd_remplacement(absence, &s, &id) {
                ecriture.lancer(async move {
                    let r = session::client().organiser_remplacement(&cmd).await?;
                    Ok(if r.deja_fait {
                        "Ce remplacement était déjà enregistré.".to_string()
                    } else {
                        format!(
                            "{nom} remplacera ce cours. Les élèves et les familles sont prévenus."
                        )
                    })
                });
            }
        }
    });
    let confirmer_annulation = Callback::new(move |_| {
        if let Some((absence, s)) = cible.get_untracked() {
            ecriture.lancer(async move {
                let r = session::client()
                    .annuler_seance(s.creneau_id, s.jour, Some(absence))
                    .await?;
                Ok(if r.deja_fait {
                    "Ce cours était déjà annulé.".to_string()
                } else {
                    "Cours annulé. Les élèves et les familles sont prévenus.".to_string()
                })
            });
        }
    });
    let confirmer_retablissement = Callback::new(move |_| {
        if let Some((_, s)) = cible.get_untracked() {
            ecriture.lancer(async move {
                session::client()
                    .lever_exception(s.creneau_id, s.jour)
                    .await?;
                Ok("Cours rétabli. Les élèves et les familles sont prévenus.".to_string())
            });
        }
    });
    let confirmer_validation = Callback::new(move |_| {
        if let Some((id, nom)) = a_valider.get_untracked() {
            ecriture.lancer(async move {
                session::client().valider_absence_rh(id).await?;
                Ok(format!(
                    "Absence de {nom} validée : ses séances peuvent être traitées."
                ))
            });
        }
    });

    let phrase = move |f: fn(&SeanceToucheeDto) -> String| {
        Signal::derive(move || cible.get().map(|(_, s)| f(&s)).unwrap_or_default())
    };
    let message_rempl = Signal::derive(move || match (cible.get(), retenu.get()) {
        (Some((_, s)), Some((_, nom))) => message_remplacement(&s, &nom),
        _ => String::new(),
    });
    let message_valid = Signal::derive(move || match a_valider.get() {
        Some((_, nom)) => format!(
            "Valider l'absence de {nom} ? Ses séances touchées pourront alors être traitées. \
             Les motifs d'absence ne sont jamais communiqués aux élèves ni aux familles."
        ),
        None => String::new(),
    });
    let appliquer =
        Callback::new(
            move |_| match valider_periode(&du.get_untracked(), &au.get_untracked()) {
                Ok(p) => {
                    erreur_periode.set(None);
                    periode.set(Some(p));
                }
                Err(m) => erreur_periode.set(Some(m)),
            },
        );
    let annonce = Signal::derive(move || match absences.get() {
        Some(Ok(Some(l))) => format!(
            "{} absence(s), {} séance(s) à traiter.",
            l.len(),
            total_a_traiter(&l)
        ),
        _ => String::new(),
    });

    view! {
        <Annonce message=annonce/>
        <RetourEcriture ecriture=ecriture/>
        <form
            class="rpl-periode"
            on:submit=move |ev| {
                ev.prevent_default();
                appliquer.run(());
            }
        >
            <ChampTexte id="rpl-du" libelle="Du" genre="date" valeur=du requis=true erreur=Signal::derive(move || erreur_periode.get())/>
            <ChampTexte id="rpl-au" libelle="Au" genre="date" valeur=au requis=true/>
            <Bouton soumettre=true>"Afficher"</Bouton>
        </form>
        <section class="rpl-section" aria-labelledby="rpl-titre-absences">
            <h2 id="rpl-titre-absences" class="vs-section-titre">"Absences du personnel"</h2>
            {move || match absences.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des absences…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| absences.refetch())/> }.into_any(),
                Some(Ok(None)) => view! { <Squelette lignes=3 libelle="Chargement des absences…"/> }.into_any(),
                Some(Ok(Some(l))) if l.is_empty() => view! {
                    <EtatVide
                        titre="Aucune absence sur la période"
                        message="Aucune absence de personnel n'a été signalée ou validée sur ces dates."
                    />
                }
                .into_any(),
                Some(Ok(Some(l))) => view! {
                    <ul class="rpl-liste">
                        {l.into_iter().map(|a| view! { <CarteAbsence absence=a actions=actions/> }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
        </section>
        {move || moteur.get().peut_annuler_masse().then(|| view! { <AnnulationMasse recharger=recharger/> })}

        <Dialogue ouvert=ouvert_candidats titre="Proposer un remplaçant" id="rpl-candidats">
            <p>{move || cible.get().map(|(_, s)| phrase_seance(&s)).unwrap_or_default()}</p>
            <Case
                id="rpl-tous"
                libelle="Afficher aussi les enseignants qui n'enseignent pas cette matière"
                coche=tous
            />
            {move || match candidats.get() {
                None => view! { <Squelette lignes=3 libelle="Recherche des remplaçants disponibles…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| candidats.refetch())/> }.into_any(),
                Some(Ok(None)) => ().into_any(),
                Some(Ok(Some(l))) if l.is_empty() => view! {
                    <EtatVide
                        titre="Aucun enseignant disponible"
                        message="Personne n'est libre sans conflit sur ce créneau. Cochez l'option ci-dessus pour élargir, ou annulez le cours."
                    />
                }
                .into_any(),
                Some(Ok(Some(l))) => view! {
                    <GroupeChoix
                        nom="rpl-remplacant"
                        legende="Enseignants disponibles"
                        valeur=choix
                        options=options_candidats(&l)
                    />
                }
                .into_any(),
            }}
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouvert_candidats.set(false))>
                    "Fermer"
                </Bouton>
                <Bouton
                    desactive=Signal::derive(move || choix.get().is_empty())
                    on_click=designer
                >
                    "Désigner ce remplaçant"
                </Bouton>
            </div>
        </Dialogue>
        <ConfirmationDetaillee
            ouvert=ouvert_remplacement
            id="rpl-confirmer-remplacement"
            titre="Confirmer le remplacement"
            message=message_rempl
            rappel=Signal::derive(|| RAPPEL_PREVENUS.to_string())
            libelle_confirmer="Confirmer le remplacement"
            on_confirmer=confirmer_remplacement
        />
        <ConfirmationDetaillee
            ouvert=ouvert_annulation
            id="rpl-confirmer-annulation"
            titre="Annuler le cours"
            message=phrase(message_annulation)
            rappel=Signal::derive(|| RAPPEL_PREVENUS.to_string())
            libelle_confirmer="Annuler le cours"
            danger=true
            on_confirmer=confirmer_annulation
        />
        <ConfirmationDetaillee
            ouvert=ouvert_retablissement
            id="rpl-confirmer-retablissement"
            titre="Rétablir le cours"
            message=phrase(message_retablissement)
            rappel=Signal::derive(|| "Les élèves et leurs familles seront prévenus que le cours est rétabli.".to_string())
            libelle_confirmer="Rétablir le cours"
            on_confirmer=confirmer_retablissement
        />
        <ConfirmationDetaillee
            ouvert=ouvert_validation
            id="rpl-confirmer-validation"
            titre="Valider l'absence"
            message=message_valid
            libelle_confirmer="Valider l'absence"
            on_confirmer=confirmer_validation
        />
    }
}

#[component]
fn CarteAbsence(absence: AbsenceSeancesDto, actions: Actions) -> impl IntoView {
    let id = absence.absence.id;
    let nom = absence
        .absence
        .nom
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "Personnel".to_string());
    let validee = absence.absence.statut == "validee";
    let statut = absence.absence.statut.clone();
    let resume = resume_traitement(&absence);
    let dates = libelle_dates(absence.absence.debut, absence.absence.fin);
    let nom_valider = nom.clone();
    let libelle_valider = format!("Valider l'absence de {nom}");
    let seances = absence.seances.clone();
    view! {
        <li class="rpl-carte">
            <div class="rpl-carte__tete">
                <strong>{nom}</strong>
                <Badge libelle=libelle_statut_absence(&statut) ton=ton_statut_absence(&statut)/>
            </div>
            <p class="rpl-carte__detail">{dates}</p>
            <p class="rpl-carte__resume">{resume}</p>
            {(!validee).then(|| view! {
                <div class="rpl-carte__actions">
                    {move || actions.peut_ecrire.get().then(|| {
                        let nom = nom_valider.clone();
                        view! {
                            <Bouton
                                taille=Taille::Compacte
                                libelle_accessible=libelle_valider.clone()
                                on_click=Callback::new(move |_| actions.valider.run((id, nom.clone())))
                            >
                                "Valider l'absence"
                            </Bouton>
                        }
                    })}
                </div>
            })}
            {(validee && !seances.is_empty()).then(|| view! {
                <ul class="rpl-seances">
                    {seances
                        .into_iter()
                        .map(|s| view! { <LigneSeance absence=id seance=s actions=actions/> })
                        .collect_view()}
                </ul>
            })}
        </li>
    }
}

#[component]
fn LigneSeance(absence: Uuid, seance: SeanceToucheeDto, actions: Actions) -> impl IntoView {
    let etat = libelle_etat_seance(&seance);
    let ton = EtatSeance::depuis(&seance.statut).ton();
    let a_traiter = peut_traiter(&seance);
    let retablissable = peut_retablir(&seance);
    let contexte = format!("{} le {}", libelle_cours(&seance), libelle_horaire(&seance));
    let remplacant_de_l_absent = seance.role_absent == "remplacant";
    let (s1, s2, s3) = (seance.clone(), seance.clone(), seance.clone());
    let (c1, c2, c3) = (contexte.clone(), contexte.clone(), contexte.clone());
    view! {
        <li class="rpl-seance">
            <p class="rpl-seance__horaire">{libelle_horaire(&seance)}</p>
            <p class="rpl-seance__cours">{libelle_cours(&seance)}</p>
            {remplacant_de_l_absent.then(|| view! {
                <p class="rpl-seance__detail">"L'absent assurait ce cours comme remplaçant."</p>
            })}
            <p class="rpl-seance__etat"><Badge libelle=etat ton=ton/></p>
            {move || actions.peut_ecrire.get().then(|| view! {
                <div class="rpl-seance__actions">
                    {a_traiter.then(|| {
                        let (s1, s2) = (s1.clone(), s2.clone());
                        view! {
                            <Bouton
                                taille=Taille::Compacte
                                libelle_accessible=format!("Proposer un remplaçant pour {c1}")
                                on_click=Callback::new(move |_| actions.proposer.run((absence, s1.clone())))
                            >
                                "Proposer un remplaçant"
                            </Bouton>
                            <Bouton
                                taille=Taille::Compacte
                                variante=Variante::Danger
                                libelle_accessible=format!("Annuler le cours : {c2}")
                                on_click=Callback::new(move |_| actions.annuler.run((absence, s2.clone())))
                            >
                                "Annuler le cours"
                            </Bouton>
                        }
                    })}
                    {retablissable.then(|| {
                        let s3 = s3.clone();
                        view! {
                            <Bouton
                                taille=Taille::Compacte
                                variante=Variante::Secondaire
                                libelle_accessible=format!("Rétablir le cours : {c3}")
                                on_click=Callback::new(move |_| actions.retablir.run((absence, s3.clone())))
                            >
                                "Rétablir"
                            </Bouton>
                        }
                    })}
                </div>
            })}
        </li>
    }
}

// ---------------------------------------------------------------------------
// Annulation en masse (edt.annuler_masse)
// ---------------------------------------------------------------------------

/// Aperçu en cours : signature de la saisie prévisualisée, corps envoyé et rapport.
type Apercu = (
    String,
    ent_types::edt_s2::CmdAnnulationMasseDto,
    RapportMasseDto,
);

#[component]
fn AnnulationMasse(recharger: Callback<()>) -> impl IntoView {
    let du = RwSignal::new(String::new());
    let au = RwSignal::new(String::new());
    let portee = RwSignal::new(PORTEE_ETABLISSEMENT.to_string());
    let classes_choisies = RwSignal::new(Vec::<Uuid>::new());
    let heure_debut = RwSignal::new(String::new());
    let heure_fin = RwSignal::new(String::new());
    let motif = RwSignal::new(String::new());
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let apercu = RwSignal::new(None::<Apercu>);
    let en_apercu = RwSignal::new(false);
    let erreur_apercu = RwSignal::new(None::<String>);
    let ouvert = RwSignal::new(false);

    let saisie = Memo::new(move |_| SaisieMasse {
        du: du.get(),
        au: au.get(),
        portee: portee.get(),
        classes: classes_choisies.get(),
        heure_debut: heure_debut.get(),
        heure_fin: heure_fin.get(),
        motif: motif.get(),
    });
    // Toute modification de la saisie invalide l'aperçu : seule la saisie prévisualisée s'applique.
    let valide = Memo::new(move |_| {
        let s = saisie.get();
        apercu.with(|a| apercu_valide(a.as_ref().map(|x| x.0.as_str()), &s))
    });
    let obsolete = Memo::new(move |_| apercu.with(Option::is_some) && !valide.get());

    let ecriture = Ecriture::nouveau(Callback::new(move |_| {
        apercu.set(None);
        recharger.run(());
    }));

    let classes = LocalResource::new(move || {
        let p = portee.get();
        async move {
            if p == PORTEE_CLASSES {
                let mut l = session::client().classes(aujourdhui()).await?;
                l.sort_by(|a, b| a.code.cmp(&b.code));
                Ok::<Vec<ClasseApi>, ApiError>(l)
            } else {
                Ok(Vec::new())
            }
        }
    });

    let previsualiser = Callback::new(move |_| {
        if en_apercu.get_untracked() {
            return;
        }
        let s = saisie.get_untracked();
        match composer_masse(&s, true) {
            Err(e) => {
                erreurs.set(e);
                apercu.set(None);
            }
            Ok(cmd) => {
                erreurs.set(Vec::new());
                erreur_apercu.set(None);
                en_apercu.set(true);
                let sig = signature(&s);
                spawn_local(async move {
                    match session::client().annulation_masse(&cmd).await {
                        Ok(r) => apercu.set(Some((sig, cmd, r))),
                        Err(e) => {
                            apercu.set(None);
                            erreur_apercu.set(Some(message_echec(e)));
                        }
                    }
                    en_apercu.set(false);
                });
            }
        }
    });
    let confirmer = Callback::new(move |_| {
        // Ne s'applique que ce qui a été prévisualisé, avec la clé d'idempotence de ces paramètres.
        if !valide.get_untracked() {
            return;
        }
        if let Some((_, mut cmd, _)) = apercu.get_untracked() {
            cmd.apercu = false;
            cmd.cle = Some(cle_idempotence(&cmd));
            ecriture.lancer(async move {
                let r = session::client().annulation_masse(&cmd).await?;
                Ok(message_application(&r))
            });
        }
    });
    let message = Signal::derive(move || {
        apercu
            .with(|a| {
                a.as_ref()
                    .map(|(_, cmd, r)| message_confirmation_masse(cmd, r))
            })
            .unwrap_or_default()
    });
    let rappel = Signal::derive(move || {
        format!(
            "{RAPPEL_PREVENUS} Le motif public sera lu par les familles : « {} ».",
            motif.get().trim()
        )
    });
    let erreur_de = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs
                .get()
                .into_iter()
                .find(|(c, _)| c == champ)
                .map(|(_, m)| m)
        })
    };

    view! {
        <section class="rpl-section" aria-labelledby="rpl-titre-masse">
            <h2 id="rpl-titre-masse" class="vs-section-titre">"Annulation en masse"</h2>
            <p class="texte-attenue">
                "Annule tous les cours d'une période (grève, fermeture exceptionnelle…). Un aperçu est obligatoire avant de confirmer ; "
                "toute modification du formulaire le rend caduc."
            </p>
            <RetourEcriture ecriture=ecriture/>
            <form
                class="rpl-formulaire"
                on:submit=move |ev| {
                    ev.prevent_default();
                    previsualiser.run(());
                }
            >
                <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
                <ChampTexte id="rpl-masse-du" libelle="Du" genre="date" valeur=du requis=true erreur=erreur_de("rpl-masse-du")/>
                <ChampTexte id="rpl-masse-au" libelle="Au (inclus)" genre="date" valeur=au requis=true erreur=erreur_de("rpl-masse-au")/>
                <GroupeChoix
                    nom="rpl-masse-portee"
                    legende="Cours concernés"
                    valeur=portee
                    options=vec![
                        (PORTEE_ETABLISSEMENT.to_string(), "Tout l'établissement".to_string()),
                        (PORTEE_CLASSES.to_string(), "Certaines classes".to_string()),
                    ]
                />
                {move || (portee.get() == PORTEE_CLASSES).then(|| view! {
                    <fieldset id="rpl-masse-classes" class="rpl-classes">
                        <legend>"Classes concernées"</legend>
                        {move || match classes.get() {
                            None => view! { <Squelette lignes=2 libelle="Chargement des classes…"/> }.into_any(),
                            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| classes.refetch())/> }.into_any(),
                            Some(Ok(l)) if l.is_empty() => view! { <p>"Aucune classe."</p> }.into_any(),
                            Some(Ok(l)) => l
                                .into_iter()
                                .map(|c| {
                                    let (id, pour) = (c.id, format!("rpl-classe-{}", c.id));
                                    let libelle = if c.libelle.trim().is_empty() || c.libelle == c.code {
                                        c.code.clone()
                                    } else {
                                        format!("{} — {}", c.code, c.libelle.trim())
                                    };
                                    let pour_label = pour.clone();
                                    view! {
                                        <label class="ui-case" for=pour_label>
                                            <input
                                                id=pour
                                                type="checkbox"
                                                prop:checked=move || classes_choisies.with(|v| v.contains(&id))
                                                on:change=move |ev| {
                                                    let coche = event_target_checked(&ev);
                                                    classes_choisies.update(|v| {
                                                        v.retain(|x| *x != id);
                                                        if coche {
                                                            v.push(id);
                                                        }
                                                    });
                                                }
                                            />
                                            <span>{libelle}</span>
                                        </label>
                                    }
                                })
                                .collect_view()
                                .into_any(),
                        }}
                        {move || erreur_de("rpl-masse-classes").get().map(|m| view! { <p class="ui-champ__erreur">{m}</p> })}
                    </fieldset>
                })}
                <ChampTexte
                    id="rpl-masse-hd"
                    libelle="À partir de (facultatif)"
                    genre="time"
                    valeur=heure_debut
                    aide="Laissez vide pour toute la journée."
                    erreur=erreur_de("rpl-masse-hd")
                />
                <ChampTexte
                    id="rpl-masse-hf"
                    libelle="Jusqu'à (facultatif)"
                    genre="time"
                    valeur=heure_fin
                    erreur=erreur_de("rpl-masse-hf")
                />
                <ZoneTexte
                    id="rpl-masse-motif"
                    libelle="Motif public"
                    valeur=motif
                    requis=true
                    erreur=erreur_de("rpl-masse-motif")
                />
                <p class="texte-attenue">
                    "Ce motif sera lu par les élèves et les familles : n'indiquez aucune information personnelle."
                </p>
                <Bouton soumettre=true variante=Variante::Secondaire en_cours=en_apercu>"Prévisualiser"</Bouton>
            </form>
            {move || erreur_apercu.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {move || obsolete.get().then(|| view! {
                <Alerte niveau=Niveau::Avertissement titre="Le formulaire a changé depuis l'aperçu.">
                    <p>"Relancez l'aperçu pour pouvoir confirmer."</p>
                </Alerte>
            })}
            {move || {
                valide
                    .get()
                    .then(|| apercu.get())
                    .flatten()
                    .map(|(_, _, r)| view! { <ApercuMasse rapport=r/> })
            }}
            {move || (valide.get() && apercu.with(|a| a.as_ref().is_some_and(|x| x.2.nb_seances > 0))).then(|| view! {
                <div class="rpl-confirmation">
                    <Bouton
                        variante=Variante::Danger
                        en_cours=ecriture.en_cours
                        on_click=Callback::new(move |_| ouvert.set(true))
                    >
                        "Confirmer l'annulation"
                    </Bouton>
                </div>
            })}
        </section>
        <ConfirmationDetaillee
            ouvert=ouvert
            id="rpl-confirmer-masse"
            titre="Confirmer l'annulation en masse"
            message=message
            rappel=rappel
            libelle_confirmer="Annuler ces cours"
            danger=true
            on_confirmer=confirmer
        />
    }
}

#[component]
fn ApercuMasse(rapport: RapportMasseDto) -> impl IntoView {
    let resume = resume_rapport(&rapport);
    let vide = rapport.nb_seances == 0;
    view! {
        <div class="rpl-apercu" role="status">
            <h3>"Aperçu"</h3>
            <p>{if vide { format!("Aucun cours à annuler : {resume}") } else { format!("Cette annulation touchera {resume}") }}</p>
            {(!rapport.classes.is_empty()).then(|| view! {
                <ul class="rpl-apercu__classes">
                    {rapport
                        .classes
                        .iter()
                        .map(|c| view! { <li>{libelle_classe_touchee(c)}</li> })
                        .collect_view()}
                </ul>
            })}
            {(rapport.deja_applique).then(|| view! {
                <p><Badge libelle="Déjà enregistrée" ton=TonEtat::Info/></p>
            })}
        </div>
    }
}
