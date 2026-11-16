//! Écran enseignant : cahier de textes de la semaine (contenu des séances, travail à donner)
//! et vue par classe (travaux à venir, alerte de charge).

use chrono::NaiveDate;
use ent_web_api::cahier_types::{ChargeDto, SeanceCahierDto, TravailDto};
use ent_web_api::{ApiError, EnseignementApi};
use ent_web_ui::{
    use_notifications, AccesRefuse, Alerte, Badge, Bouton, Case, ChampTexte, Confirmation,
    EnTetePage, EtatVide, GroupeChoix, Icone, LienBouton, Niveau, NomIcone, Squelette, Taille,
    TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use super::gestionnaire_echec;
use super::modele::{
    classes_des_seances, fin_de_semaine, libelle_duree, libelle_limite, libelle_semaine,
    libelle_type, nb_a_renseigner, seances_par_jour, semaine_demandee, semaine_voisine,
    valider_contenu, valider_travail, Erreurs, SaisieTravail,
};
use crate::composants_metier::classes::{enseignement_du_creneau, libelle_enseignement};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue, heure_courte};

/// Horizon de la vue par classe (jours à partir du lundi affiché).
const HORIZON_JOURS: i64 = 28;

#[component]
pub fn PageCahier() -> impl IntoView {
    let droits = use_droits();
    view! {
        <Title text="Cahier de textes"/>
        {move || {
            if droits.get().cahier_ecrire {
                view! { <ContenuCahier/> }.into_any()
            } else {
                view! { <AccesRefuse message="Le cahier de textes est réservé aux enseignants."/> }.into_any()
            }
        }}
    }
}

#[component]
fn ContenuCahier() -> impl IntoView {
    let query = use_query_map();
    let lundi =
        Memo::new(move |_| semaine_demandee(query.with(|q| q.get("semaine")), aujourdhui()));
    let seances = LocalResource::new(move || {
        let l = lundi.get();
        async move { session::client().cahier_seances(l, fin_de_semaine(l)).await }
    });
    // Liste unique « mes enseignements » (classes et groupes) : effectif de chaque séance.
    let enseignements = LocalResource::new(move || {
        let l = lundi.get();
        async move {
            session::client()
                .mes_enseignements(l)
                .await
                .unwrap_or_default()
        }
    });
    let adresse = move |decalage: i64| {
        format!(
            "{}?semaine={}",
            navigation::CAHIER,
            semaine_voisine(lundi.get(), decalage)
        )
    };
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre=Signal::derive(move || libelle_semaine(lundi.get()))
        >
            <LienBouton href=super::cours::ADRESSE_COURS variante=Variante::Secondaire taille=Taille::Compacte>
                "Cahier du cours"
            </LienBouton>
            <LienBouton href=super::ADRESSE_CHARGE variante=Variante::Secondaire taille=Taille::Compacte>
                "Charge des classes"
            </LienBouton>
            <nav class="edt-semaines" aria-label="Changer de semaine">
                <a class="ui-btn ui-btn--secondaire ui-btn--compact" href=move || adresse(-1) aria-label="Semaine précédente">
                    <Icone nom=NomIcone::ChevronGauche petite=true/>
                </a>
                <LienBouton href=navigation::CAHIER variante=Variante::Secondaire taille=Taille::Compacte>
                    "Cette semaine"
                </LienBouton>
                <a class="ui-btn ui-btn--secondaire ui-btn--compact" href=move || adresse(1) aria-label="Semaine suivante">
                    <Icone nom=NomIcone::ChevronDroite petite=true/>
                </a>
            </nav>
        </EnTetePage>
        {move || match seances.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement du cahier de textes…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| seances.refetch())/> }.into_any(),
            Some(Ok(l)) => {
                let rafraichir = Callback::new(move |_| seances.refetch());
                view! { <SemaineSeances seances=l enseignements=enseignements.get().unwrap_or_default() lundi=lundi.get() on_change=rafraichir/> }.into_any()
            }
        }}
    }
}

#[component]
fn SemaineSeances(
    seances: Vec<SeanceCahierDto>,
    enseignements: Vec<EnseignementApi>,
    lundi: NaiveDate,
    on_change: Callback<()>,
) -> impl IntoView {
    if seances.is_empty() {
        return view! {
            <EtatVide
                titre="Aucune séance cette semaine"
                message="Les séances datées sont générées depuis l'emploi du temps par l'administration."
            />
        }
        .into_any();
    }
    let a_renseigner = nb_a_renseigner(&seances);
    let classes = classes_des_seances(&seances);
    let jours = seances_par_jour(seances);
    view! {
        <p class="cdt-resume" role="status">
            {if a_renseigner == 0 {
                "Toutes les séances de la semaine sont renseignées.".to_string()
            } else {
                format!("{a_renseigner} séance{} à renseigner.", if a_renseigner > 1 { "s" } else { "" })
            }}
        </p>
        <div class="cdt-semaine">
            {jours
                .into_iter()
                .map(|(jour, liste)| {
                    let id_titre = format!("cdt-jour-{jour}");
                    let pour = id_titre.clone();
                    view! {
                        <section class="cdt-jour" aria-labelledby=pour>
                            <h2 id=id_titre class="cdt-jour__titre">{date_longue(jour)}</h2>
                            <ul class="cdt-liste">
                                {liste
                                    .into_iter()
                                    .map(|s| {
                                        let effectif = enseignement_du_creneau(&enseignements, &s.classe, &s.matiere_libelle)
                                            .map(libelle_enseignement);
                                        view! { <li><CarteSeance seance=s effectif=effectif on_change=on_change/></li> }
                                    })
                                    .collect_view()}
                            </ul>
                        </section>
                    }
                })
                .collect_view()}
        </div>
        <section class="cdt-classes" aria-labelledby="cdt-classes-titre">
            <h2 id="cdt-classes-titre" class="vs-section-titre">"Travail à venir par classe"</h2>
            {classes
                .into_iter()
                .map(|(id, code)| view! { <SectionClasse classe_id=id code=code lundi=lundi/> })
                .collect_view()}
        </section>
    }
    .into_any()
}

#[component]
fn CarteSeance(
    seance: SeanceCahierDto,
    /// Libellé de l'enseignement (groupe ou classe) avec effectif, si identifié sans ambiguïté.
    effectif: Option<String>,
    on_change: Callback<()>,
) -> impl IntoView {
    let id = seance.seance_id;
    let annulee = seance.statut == "annulee";
    let renseignee = seance.contenu.is_some();
    let titre = format!(
        "{} – {} · {} · {}",
        heure_courte(&seance.heure_debut),
        heure_courte(&seance.heure_fin),
        seance.matiere_libelle,
        seance.classe
    );
    let (badge, ton) = if annulee {
        ("Séance annulée", TonEtat::Neutre)
    } else if renseignee {
        ("Renseignée", TonEtat::Ok)
    } else {
        ("À renseigner", TonEtat::Avertissement)
    };
    let apercu = seance
        .contenu
        .as_ref()
        .map(|c| c.contenu.clone())
        .filter(|c| !c.is_empty());
    let nb_travaux = seance.nb_travaux;
    let jour = seance.jour;
    let classe_id = seance.classe_id;
    view! {
        <article class="cdt-seance" class:cdt-seance--annulee=annulee>
            <header class="cdt-seance__entete">
                <h3 class="cdt-seance__titre">{titre}</h3>
                <Badge libelle=badge ton=ton/>
            </header>
            {effectif.map(|e| view! { <p class="cdt-seance__detail">{e}</p> })}
            {apercu.map(|a| view! { <p class="cdt-seance__apercu">{a}</p> })}
            {(nb_travaux > 0).then(|| view! {
                <p class="cdt-seance__detail">
                    {format!("{nb_travaux} travail{} donné{}.", if nb_travaux > 1 { "x" } else { "" }, if nb_travaux > 1 { "s" } else { "" })}
                </p>
            })}
            {(!annulee).then(|| view! {
                <details class="cdt-seance__saisie">
                    <summary>{if renseignee { "Modifier le contenu" } else { "Renseigner la séance" }}</summary>
                    <FormulaireContenu seance=seance.clone() on_change=on_change/>
                </details>
                <details class="cdt-seance__saisie">
                    <summary>"Donner un travail"</summary>
                    <FormulaireTravail seance_id=id classe_id=classe_id jour=jour on_change=on_change/>
                </details>
            })}
        </article>
    }
}

#[component]
fn FormulaireContenu(seance: SeanceCahierDto, on_change: Callback<()>) -> impl IntoView {
    let id = seance.seance_id;
    let notifications = use_notifications();
    let contenu = RwSignal::new(
        seance
            .contenu
            .as_ref()
            .map(|c| c.contenu.clone())
            .unwrap_or_default(),
    );
    let ressources = RwSignal::new(
        seance
            .contenu
            .as_ref()
            .map(|c| c.ressources.clone())
            .unwrap_or_default(),
    );
    let err_contenu = RwSignal::new(None::<String>);
    let err_ressources = RwSignal::new(None::<String>);
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let echec = gestionnaire_echec(erreur);
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        erreur.set(None);
        match valider_contenu(&contenu.get_untracked(), &ressources.get_untracked()) {
            Err(es) => poser_erreurs(
                &es,
                &[("contenu", err_contenu), ("ressources", err_ressources)],
            ),
            Ok((c, r)) => {
                err_contenu.set(None);
                err_ressources.set(None);
                en_cours.set(true);
                spawn_local(async move {
                    let res = session::client().cahier_renseigner_seance(id, &c, &r).await;
                    en_cours.set(false);
                    match res {
                        Ok(_) => {
                            notifications.succes("Contenu de la séance enregistré.");
                            on_change.run(());
                        }
                        Err(e) => echec.run(e),
                    }
                });
            }
        }
    };
    let pref = format!("cdt-c-{id}");
    view! {
        <form class="cdt-form" on:submit=envoyer novalidate=true>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            <ZoneTexte id=format!("{pref}-contenu") libelle="Contenu de la séance" valeur=contenu erreur=err_contenu requis=true/>
            <ZoneTexte id=format!("{pref}-ressources") libelle="Ressources (facultatif)" valeur=ressources erreur=err_ressources
                aide="Liens, pages du manuel… Les fichiers se joignent depuis la fiche de la séance."/>
            <Bouton soumettre=true en_cours=en_cours>"Enregistrer"</Bouton>
        </form>
    }
}

fn poser_erreurs(es: &Erreurs, cibles: &[(&'static str, RwSignal<Option<String>>)]) {
    for (champ, signal) in cibles {
        signal.set(es.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()));
    }
}

#[component]
fn FormulaireTravail(
    seance_id: Uuid,
    classe_id: Option<Uuid>,
    jour: NaiveDate,
    on_change: Callback<()>,
) -> impl IntoView {
    let notifications = use_notifications();
    let type_travail = RwSignal::new("a_faire".to_string());
    let titre = RwSignal::new(String::new());
    let consigne = RwSignal::new(String::new());
    let pour_le = RwSignal::new(String::new());
    let duree = RwSignal::new(String::new());
    let limite = RwSignal::new(String::new());
    let formats = RwSignal::new(String::new());
    let retard = RwSignal::new(true);
    let (e_titre, e_consigne, e_pour_le, e_duree, e_limite) = (
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
    );
    let erreur = RwSignal::new(None::<String>);
    let alerte_charge = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let echec = gestionnaire_echec(erreur);
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        erreur.set(None);
        alerte_charge.set(None);
        let saisie = SaisieTravail {
            type_travail: type_travail.get_untracked(),
            titre: titre.get_untracked(),
            consigne: consigne.get_untracked(),
            pour_le: pour_le.get_untracked(),
            duree_min: duree.get_untracked(),
            limite: limite.get_untracked(),
            formats: formats.get_untracked(),
            retard_accepte: retard.get_untracked(),
        };
        match valider_travail(&saisie, Some(seance_id), classe_id, aujourdhui().max(jour)) {
            Err(es) => poser_erreurs(
                &es,
                &[
                    ("titre", e_titre),
                    ("consigne", e_consigne),
                    ("pour-le", e_pour_le),
                    ("duree", e_duree),
                    ("limite", e_limite),
                ],
            ),
            Ok(corps) => {
                for s in [e_titre, e_consigne, e_pour_le, e_duree, e_limite] {
                    s.set(None);
                }
                en_cours.set(true);
                spawn_local(async move {
                    let res = session::client().cahier_donner_travail(&corps).await;
                    en_cours.set(false);
                    match res {
                        Ok(cree) => {
                            notifications.succes("Travail publié : les élèves et leurs responsables seront prévenus.");
                            if cree.charge.surcharge {
                                alerte_charge.set(Some(format!(
                                    "Charge élevée le {} : {} de travail estimé, {} contrôle(s).",
                                    date_longue(cree.charge.jour),
                                    libelle_duree(cree.charge.minutes),
                                    cree.charge.nb_controles
                                )));
                            }
                            titre.set(String::new());
                            consigne.set(String::new());
                            on_change.run(());
                        }
                        Err(e) => echec.run(e),
                    }
                });
            }
        }
    };
    let pref = format!("cdt-t-{seance_id}");
    let a_rendre = Memo::new(move |_| type_travail.get() == "a_rendre");
    view! {
        <form class="cdt-form" on:submit=envoyer novalidate=true>
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            {move || alerte_charge.get().map(|m| view! { <Alerte niveau=Niveau::Avertissement titre=m/> })}
            <GroupeChoix
                nom=format!("{pref}-type")
                legende="Type de travail"
                valeur=type_travail
                options=vec![
                    ("a_faire".to_string(), "À faire".to_string()),
                    ("a_rendre".to_string(), "À rendre en ligne".to_string()),
                    ("controle".to_string(), "Contrôle annoncé".to_string()),
                ]
            />
            <ChampTexte id=format!("{pref}-titre") libelle="Titre" valeur=titre erreur=e_titre requis=true/>
            <ZoneTexte id=format!("{pref}-consigne") libelle="Consigne" valeur=consigne erreur=e_consigne requis=true/>
            <ChampTexte id=format!("{pref}-pour-le") libelle="Pour le" genre="date" valeur=pour_le erreur=e_pour_le requis=true/>
            <ChampTexte id=format!("{pref}-duree") libelle="Durée estimée (minutes)" genre="number" inputmode="numeric"
                valeur=duree erreur=e_duree aide="Facultative. Sert à repérer les jours trop chargés pour la classe."/>
            {move || a_rendre.get().then(|| view! {
                <ChampTexte id=format!("{pref}-limite") libelle="Date limite de dépôt" genre="date" valeur=limite erreur=e_limite
                    aide="Facultative : sans date, le dépôt reste ouvert jusqu'à la fin du jour d'échéance."/>
                <ChampTexte id=format!("{pref}-formats") libelle="Formats acceptés" valeur=formats
                    aide="Facultatif, séparés par des virgules (pdf, docx, jpg…). Vide : tous les formats autorisés."/>
                <Case id=format!("{pref}-retard") libelle="Accepter les dépôts après la date limite (marqués en retard)" coche=retard/>
            })}
            <Bouton soumettre=true en_cours=en_cours>"Publier le travail"</Bouton>
        </form>
    }
}

// ───────────────────────────── Vue par classe ─────────────────────────────

type DonneesClasse = Result<(Vec<TravailDto>, Option<ChargeDto>), ApiError>;

#[component]
fn SectionClasse(classe_id: Uuid, code: String, lundi: NaiveDate) -> impl IntoView {
    let au = lundi + chrono::Duration::days(HORIZON_JOURS);
    let donnees = LocalResource::new(move || async move {
        let client = session::client();
        let travaux = client
            .cahier_travaux(Some(classe_id), None, lundi, au)
            .await?;
        // La charge est une aide : un refus ou une panne ne bloque pas la liste.
        let charge = client.cahier_charge(classe_id, lundi, au).await.ok();
        Ok::<_, ApiError>((travaux, charge))
    });
    let id_titre = format!("cdt-classe-{classe_id}");
    let pour = id_titre.clone();
    view! {
        <section class="cdt-classe" aria-labelledby=pour>
            <h3 id=id_titre class="cdt-classe__titre">{format!("Classe {code}")}</h3>
            {move || match donnees.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des travaux…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| donnees.refetch())/> }.into_any(),
                Some(Ok(d)) => view! { <TravauxClasse donnees=Ok(d) on_change=Callback::new(move |_| donnees.refetch())/> }.into_any(),
            }}
        </section>
    }
}

#[component]
fn TravauxClasse(donnees: DonneesClasse, on_change: Callback<()>) -> impl IntoView {
    let Ok((travaux, charge)) = donnees else {
        return ().into_any();
    };
    let jours_charges: Vec<_> = charge
        .map(|c| c.jours.into_iter().filter(|j| j.surcharge).collect())
        .unwrap_or_default();
    if travaux.is_empty() {
        return view! { <p class="cdt-vide">"Aucun travail prévu sur les quatre prochaines semaines."</p> }.into_any();
    }
    view! {
        {(!jours_charges.is_empty()).then(|| view! {
            <Alerte niveau=Niveau::Avertissement titre="Jours très chargés pour cette classe">
                <ul class="cdt-surcharge">
                    {jours_charges
                        .into_iter()
                        .map(|j| view! {
                            <li>
                                {format!("{} : {} de travail estimé, {} contrôle(s), {} travaux.",
                                    date_longue(j.jour), libelle_duree(j.minutes), j.nb_controles, j.nb_travaux)}
                            </li>
                        })
                        .collect_view()}
                </ul>
            </Alerte>
        })}
        <ul class="cdt-liste">
            {travaux
                .into_iter()
                .map(|t| view! { <li><LigneTravail travail=t on_change=on_change/></li> })
                .collect_view()}
        </ul>
    }
    .into_any()
}

#[component]
fn LigneTravail(travail: TravailDto, on_change: Callback<()>) -> impl IntoView {
    let notifications = use_notifications();
    let erreur = RwSignal::new(None::<String>);
    let confirmer = RwSignal::new(false);
    let echec = gestionnaire_echec(erreur);
    let id = travail.id;
    let supprimer = Callback::new(move |_| {
        spawn_local(async move {
            match session::client().cahier_supprimer_travail(id).await {
                Ok(()) => {
                    notifications.succes("Travail supprimé.");
                    on_change.run(());
                }
                Err(e) => echec.run(e),
            }
        });
    });
    let ton = if travail.type_travail == "controle" {
        TonEtat::Avertissement
    } else {
        TonEtat::Info
    };
    let limite = libelle_limite(&travail);
    let duree = travail.duree_estimee_min.map(libelle_duree);
    let lien_copies = format!("{}/travaux/{}", navigation::CAHIER, id);
    let a_rendre = travail.rendu_en_ligne;
    let dialogue = format!("cdt-sup-{id}");
    view! {
        <article class="cdt-travail">
            {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            <header class="cdt-travail__entete">
                <h4 class="cdt-travail__titre">{format!("{} — {}", travail.matiere_libelle, travail.titre)}</h4>
                <Badge libelle=libelle_type(&travail.type_travail) ton=ton/>
            </header>
            <p class="cdt-travail__detail">
                {format!("Pour le {}", date_longue(travail.pour_le))}
                {duree.map(|d| format!(" · {d}"))}
                {limite.map(|l| format!(" · {l}"))}
            </p>
            <div class="cdt-travail__actions">
                {a_rendre.then(|| view! {
                    <LienBouton href=lien_copies variante=Variante::Secondaire taille=Taille::Compacte>"Voir les copies"</LienBouton>
                })}
                <Bouton variante=Variante::Discret taille=Taille::Compacte on_click=Callback::new(move |_| confirmer.set(true))>
                    "Supprimer"
                </Bouton>
            </div>
            <Confirmation
                ouvert=confirmer
                id=dialogue
                titre="Supprimer ce travail ?"
                message="Les élèves ne le verront plus. L'historique de la modification est conservé."
                libelle_confirmer="Supprimer"
                danger=true
                on_confirmer=supprimer
            />
        </article>
    }
}
