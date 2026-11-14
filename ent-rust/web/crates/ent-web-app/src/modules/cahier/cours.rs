//! Écran 1 du lot C2 : le **cahier du cours** (entrées par séance, brouillons, programmation, plage calme,
//! modification avec ou sans avis, historique, retrait, copie, travaux de la séance).
//!
//! Route `/web/cahier/cours?cours=<id>&semaine=<date>`. Aucune règle métier ici : droits à la date, plage calme,
//! versions et avis sont décidés par le serveur ; l'écran formate ce qu'il annonce (`modele_c2`).

use chrono::NaiveDate;
use ent_web_api::cahier_c2_types::{
    CopieVersBody, CoursSaisieDto, EntreeDto, SeanceDuCoursDto, TravailC2Dto, TravailResumeDto,
    VersionDto,
};
use ent_web_ui::{
    AccesRefuse, Alerte, Badge, Bouton, Case, ChampTexte, Confirmation, EnTetePage, EtatVide,
    GroupeChoix, Icone, LienBouton, Niveau, NomIcone, Selection, Squelette, Taille, Variante,
    ZoneTexte,
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use super::modele::{fin_de_semaine, libelle_semaine, semaine_demandee, semaine_voisine, Erreurs};
use super::modele_c2::{
    etat_entree, etat_travail, heure_en_clair, libelle_visibilite, valider_entree,
    valider_modification, OrigineTravail, SaisieEntree, SaisieModification, RAPPEL_PLAGE_CALME,
};
use crate::noyau::coquille::use_droits;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{aujourdhui, date_longue, heure_courte};

/// Adresse de l'écran 1.
pub const ADRESSE_COURS: &str = "/web/cahier/cours";
/// Adresse de l'écran 2 (donner un travail).
pub const ADRESSE_TRAVAIL: &str = "/web/cahier/travail";

#[component]
pub fn PageCahierCours() -> impl IntoView {
    let droits = use_droits();
    view! {
        <Title text="Cahier du cours"/>
        {move || {
            if droits.get().cahier_ecrire {
                view! { <ContenuCours/> }.into_any()
            } else {
                view! { <AccesRefuse message="Le cahier du cours est réservé aux enseignants."/> }.into_any()
            }
        }}
    }
}

fn poser(es: &Erreurs, cibles: &[(&'static str, RwSignal<Option<String>>)]) {
    for (champ, signal) in cibles {
        signal.set(es.iter().find(|(c, _)| c == champ).map(|(_, m)| m.clone()));
    }
}

#[component]
fn ContenuCours() -> impl IntoView {
    let query = use_query_map();
    let cours_demande = Memo::new(move |_| {
        query
            .with(|q| q.get("cours"))
            .and_then(|c| Uuid::parse_str(c.trim()).ok())
    });
    let lundi =
        Memo::new(move |_| semaine_demandee(query.with(|q| q.get("semaine")), aujourdhui()));
    let cours = LocalResource::new(|| async { session::client().cdt_cours().await });
    view! {
        <EnTetePage
            surtitre="Cahier de textes".to_string()
            titre="Cahier du cours".to_string()
            sous_titre="Une entrée par séance : brouillon, publication ou programmation. Les familles ne voient rien avant la publication.".to_string()
        />
        {move || match cours.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement de vos cours…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| cours.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucun cours à renseigner" message="Vous n'êtes intervenant en saisie sur aucun cours aujourd'hui."/>
            }.into_any(),
            Some(Ok(l)) => {
                let choisi = cours_demande.get().filter(|c| l.iter().any(|x| x.cours_id == *c)).unwrap_or(l[0].cours_id);
                view! { <ChoixCours cours=l choisi=choisi lundi=lundi/> }.into_any()
            }
        }}
    }
}

#[component]
fn ChoixCours(cours: Vec<CoursSaisieDto>, choisi: Uuid, lundi: Memo<NaiveDate>) -> impl IntoView {
    let adresse = move |id: Uuid, l: NaiveDate| format!("{ADRESSE_COURS}?cours={id}&semaine={l}");
    let semaine = move |d: i64| adresse(choisi, semaine_voisine(lundi.get(), d));
    view! {
        <nav class="cdt-cours-choix" aria-label="Mes cours">
            <ul class="cdt-liste">
                {cours
                    .into_iter()
                    .map(|c| {
                        let est = c.cours_id == choisi;
                        let id_cours = c.cours_id;
                        let href = move || adresse(id_cours, lundi.get());
                        view! {
                            <li>
                                <a class="ui-btn ui-btn--compact" class:ui-btn--secondaire=!est href=href aria-current=if est { Some("page") } else { None }>
                                    {format!("{} · {}", c.libelle, c.cible)}
                                </a>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </nav>
        <nav class="edt-semaines" aria-label="Changer de semaine">
            <a class="ui-btn ui-btn--secondaire ui-btn--compact" href=move || semaine(-1) aria-label="Semaine précédente">
                <Icone nom=NomIcone::ChevronGauche petite=true/>
            </a>
            <span class="cdt-semaine-titre">{move || libelle_semaine(lundi.get())}</span>
            <a class="ui-btn ui-btn--secondaire ui-btn--compact" href=move || semaine(1) aria-label="Semaine suivante">
                <Icone nom=NomIcone::ChevronDroite petite=true/>
            </a>
        </nav>
        <SeancesDuCours cours_id=choisi lundi=lundi/>
    }
}

#[component]
fn SeancesDuCours(cours_id: Uuid, lundi: Memo<NaiveDate>) -> impl IntoView {
    let seances = LocalResource::new(move || {
        let l = lundi.get();
        async move {
            session::client()
                .cdt_seances_du_cours(cours_id, l, fin_de_semaine(l))
                .await
        }
    });
    let recharger = Callback::new(move |_| seances.refetch());
    let ecriture = Ecriture::nouveau(recharger);
    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || match seances.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des séances…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| seances.refetch())/> }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune séance cette semaine" message="Les séances datées viennent de l'emploi du temps."/>
            }.into_any(),
            Some(Ok(l)) => {
                let autres: Vec<(String, String)> = l
                    .iter()
                    .map(|s| (s.seance_id.to_string(), format!("{} {}", date_longue(s.jour), heure_courte(&s.heure_debut))))
                    .collect();
                view! {
                    <ul class="cdt-liste">
                        {l
                            .into_iter()
                            .map(|s| view! { <li><CarteSeanceCours cours_id=cours_id seance=s autres=autres.clone() ecriture=ecriture/></li> })
                            .collect_view()}
                    </ul>
                }.into_any()
            }
        }}
    }
}

#[component]
fn CarteSeanceCours(
    cours_id: Uuid,
    seance: SeanceDuCoursDto,
    /// Séances de la période (copie vers une autre séance du cours).
    autres: Vec<(String, String)>,
    ecriture: Ecriture,
) -> impl IntoView {
    let annulee = seance.statut == "annulee";
    let titre = format!(
        "{} · {} – {}",
        date_longue(seance.jour),
        heure_courte(&seance.heure_debut),
        heure_courte(&seance.heure_fin)
    );
    let seance_id = seance.seance_id;
    let entree = seance.entree.clone();
    let (badge, ton) = match (&entree, annulee) {
        (_, true) => ("Séance annulée", ent_web_ui::TonEtat::Neutre),
        (Some(e), _) => etat_entree(&e.etat),
        (None, _) => ("À renseigner", ent_web_ui::TonEtat::Avertissement),
    };
    let visibilite = entree.as_ref().and_then(libelle_visibilite);
    let differee = entree.as_ref().is_some_and(|e| e.differee_par_plage_calme);
    let apercu = entree.as_ref().map(|e| e.contenu.clone());
    let retro = entree
        .as_ref()
        .and_then(|e| e.renseignee_le)
        .map(|_| "Saisie après la séance (renseignée après coup).");
    let donnes = seance.travaux_donnes.clone();
    let attendus = seance.travaux_attendus.clone();
    let autres_ici: Vec<(String, String)> = autres
        .into_iter()
        .filter(|(id, _)| *id != seance_id.to_string())
        .collect();
    view! {
        <article class="cdt-seance" class:cdt-seance--annulee=annulee>
            <header class="cdt-seance__entete">
                <h3 class="cdt-seance__titre">{titre}</h3>
                <Badge libelle=badge ton=ton/>
            </header>
            {visibilite.map(|v| view! { <p class="cdt-seance__detail" role="status">{v}</p> })}
            {differee.then(|| view! { <Alerte niveau=Niveau::Info titre=RAPPEL_PLAGE_CALME.to_string()/> })}
            {retro.map(|r| view! { <p class="cdt-seance__detail">{r}</p> })}
            {entree.as_ref().and_then(|e| e.titre.clone()).map(|t| view! { <p class="cdt-seance__detail"><strong>{t}</strong></p> })}
            {apercu.map(|a| view! { <p class="cdt-seance__apercu">{a}</p> })}
            {entree.as_ref().map(|e| e.contributions.iter().map(|c| view! {
                <p class="cdt-seance__detail">{format!("{} : {}", c.auteur, c.contenu)}</p>
            }).collect_view())}
            <ListeTravaux titre="Travail donné" travaux=donnes ecriture=ecriture/>
            <ListeTravaux titre="Travail attendu" travaux=attendus ecriture=ecriture/>
            {(!annulee).then(|| view! {
                <details class="cdt-seance__saisie">
                    <summary>{if entree.is_some() { "Modifier l'entrée" } else { "Renseigner la séance" }}</summary>
                    <FormulaireEntree cours_id=cours_id seance_id=seance_id entree=entree.clone() ecriture=ecriture/>
                </details>
                <LienBouton href=format!("{ADRESSE_TRAVAIL}?cours={cours_id}&seance={seance_id}") variante=Variante::Secondaire taille=Taille::Compacte>
                    "Donner un travail"
                </LienBouton>
            })}
            {entree.map(|e| view! { <ActionsEntree entree=e autres=autres_ici ecriture=ecriture/> })}
        </article>
    }
}

#[component]
fn FormulaireEntree(
    cours_id: Uuid,
    seance_id: Uuid,
    entree: Option<EntreeDto>,
    ecriture: Ecriture,
) -> impl IntoView {
    let publiee = entree.as_ref().is_some_and(|e| e.etat == "publiee");
    let titre = RwSignal::new(
        entree
            .as_ref()
            .and_then(|e| e.titre.clone())
            .unwrap_or_default(),
    );
    let contenu = RwSignal::new(
        entree
            .as_ref()
            .map(|e| e.contenu.clone())
            .unwrap_or_default(),
    );
    let notions = RwSignal::new(
        entree
            .as_ref()
            .map(|e| e.notions.join("\n"))
            .unwrap_or_default(),
    );
    // Une entrée déjà publiée le reste : on ne propose pas « brouillon ».
    let publication = RwSignal::new("maintenant".to_string());
    let jour = RwSignal::new(String::new());
    let heure = RwSignal::new(String::new());
    let sans_avis = RwSignal::new(false);
    let motif = RwSignal::new(String::new());
    let (e_contenu, e_titre, e_notions, e_jour, e_heure) = (
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
    );
    let options: Vec<(String, String)> = if publiee {
        vec![("maintenant".into(), "Enregistrer".into())]
    } else {
        vec![
            (
                "brouillon".into(),
                "Brouillon (invisible des élèves)".into(),
            ),
            ("maintenant".into(), "Publier maintenant".into()),
            ("programmer".into(), "Programmer la publication".into()),
        ]
    };
    let pref = format!("cdt-e-{seance_id}");
    let (pref_pub, pref_av) = (pref.clone(), pref.clone());
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieEntree {
            titre: titre.get_untracked(),
            contenu: contenu.get_untracked(),
            notions: notions.get_untracked(),
            publication: publication.get_untracked(),
            jour: jour.get_untracked(),
            heure: heure.get_untracked(),
            sans_avis: sans_avis.get_untracked(),
            motif: motif.get_untracked(),
        };
        match valider_entree(&saisie) {
            Err(es) => poser(
                &es,
                &[
                    ("contenu", e_contenu),
                    ("titre", e_titre),
                    ("notions", e_notions),
                    ("jour", e_jour),
                    ("heure", e_heure),
                ],
            ),
            Ok(corps) => {
                for s in [e_contenu, e_titre, e_notions, e_jour, e_heure] {
                    s.set(None);
                }
                ecriture.lancer(async move {
                    session::client()
                        .cdt_entree_enregistrer(cours_id, seance_id, &corps)
                        .await?;
                    Ok("Entrée du cahier enregistrée.".to_string())
                });
            }
        }
    };
    view! {
        <form class="cdt-form" on:submit=envoyer novalidate=true>
            <ChampTexte id=format!("{pref}-titre") libelle="Titre (facultatif)" valeur=titre erreur=e_titre/>
            <ZoneTexte id=format!("{pref}-contenu") libelle="Contenu de la séance" valeur=contenu erreur=e_contenu requis=true
                aide="Texte simple : titres, listes, liens https. Aucun nom d'élève."/>
            <ZoneTexte id=format!("{pref}-notions") libelle="Notions travaillées (une par ligne)" valeur=notions erreur=e_notions/>
            <GroupeChoix nom=format!("{pref}-publication") legende="Publication" valeur=publication options=options/>
            {move || (publication.get() == "programmer").then(|| view! {
                <ChampTexte id=format!("{pref_pub}-jour") libelle="Jour de publication" valeur=jour genre="date" erreur=e_jour requis=true/>
                <ChampTexte id=format!("{pref_pub}-heure") libelle="Heure (heure de l'établissement, 7 h par défaut)" valeur=heure genre="time" erreur=e_heure/>
            })}
            {publiee.then(|| view! {
                <Case id=format!("{pref_av}-sans-avis") libelle="Modification mineure : ne pas prévenir les élèves (la mention « modifié le » reste visible)" coche=sans_avis/>
                <ChampTexte id=format!("{pref_av}-motif") libelle="Motif de la modification (facultatif)" valeur=motif/>
            })}
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer"</Bouton>
        </form>
    }
}

#[component]
fn ActionsEntree(
    entree: EntreeDto,
    autres: Vec<(String, String)>,
    ecriture: Ecriture,
) -> impl IntoView {
    let id = entree.id;
    let retiree = entree.etat == "retiree";
    let versions = RwSignal::new(None::<Vec<VersionDto>>);
    let erreur = RwSignal::new(None::<String>);
    let voir = move |_| {
        erreur.set(None);
        leptos::task::spawn_local(async move {
            match session::client().cdt_entree_versions(id).await {
                Ok(v) => versions.set(Some(v)),
                Err(e) => erreur.set(Some(e.message_utilisateur())),
            }
        });
    };
    let motif = RwSignal::new(String::new());
    let confirmer = RwSignal::new(false);
    let destination = RwSignal::new(String::new());
    let copier = Callback::new(move |_| {
        let Ok(seance_id) = Uuid::parse_str(destination.get_untracked().trim()) else {
            return;
        };
        let cours_id = entree.cours_id;
        ecriture.lancer(async move {
            session::client()
                .cdt_entree_copier(
                    id,
                    vec![CopieVersBody {
                        cours_id,
                        seance_id,
                    }],
                )
                .await?;
            Ok("Entrée copiée en brouillon.".to_string())
        });
    });
    let retirer = Callback::new(move |_| {
        let m = motif.get_untracked();
        ecriture.lancer(async move {
            session::client().cdt_entree_retirer(id, &m).await?;
            Ok("Entrée retirée de la diffusion.".to_string())
        });
    });
    let pref = format!("cdt-a-{id}");
    let mut options_copie = vec![(String::new(), "Choisir une séance…".to_string())];
    options_copie.extend(autres.clone());
    view! {
        <div class="cdt-actions">
            <details class="cdt-seance__saisie">
                <summary on:click=voir>"Historique des versions"</summary>
                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                {move || versions.get().map(|v| if v.is_empty() {
                    view! { <p class="cdt-seance__detail">"Aucune modification depuis la première version."</p> }.into_any()
                } else {
                    view! {
                        <ol class="cdt-versions">
                            {v.into_iter().map(|x| view! {
                                <li>
                                    <strong>{format!("Version {}", x.version)}</strong>
                                    {format!(" · {} · {}", x.auteur, if x.sans_avis { "modification sans avis" } else { "avec avis" })}
                                    {x.motif.map(|m| format!(" · {m}"))}
                                    <p class="cdt-seance__apercu">{x.contenu}</p>
                                </li>
                            }).collect_view()}
                        </ol>
                    }.into_any()
                })}
            </details>
            {(!autres.is_empty() && !retiree).then(|| view! {
                <details class="cdt-seance__saisie">
                    <summary>"Copier vers une autre séance du cours"</summary>
                    <Selection id=format!("{pref}-copie") libelle="Séance de destination" valeur=destination options=options_copie/>
                    <Bouton variante=Variante::Secondaire on_click=copier>"Copier en brouillon"</Bouton>
                </details>
            })}
            {(!retiree).then(|| view! {
                <details class="cdt-seance__saisie">
                    <summary>"Retirer de la diffusion"</summary>
                    <ChampTexte id=format!("{pref}-motif") libelle="Motif du retrait (obligatoire)" valeur=motif/>
                    <Bouton variante=Variante::Danger on_click=Callback::new(move |_| confirmer.set(true))>"Retirer…"</Bouton>
                    <Confirmation
                        ouvert=confirmer
                        id=format!("{pref}-conf")
                        titre="Retirer cette entrée ?"
                        message="Les versions sont gardées ; les élèves qui l'avaient lue verront « retiré »."
                        libelle_confirmer="Retirer"
                        danger=true
                        on_confirmer=retirer
                    />
                </details>
            })}
        </div>
    }
}

#[component]
fn ListeTravaux(
    titre: &'static str,
    travaux: Vec<TravailResumeDto>,
    ecriture: Ecriture,
) -> impl IntoView {
    if travaux.is_empty() {
        return ().into_any();
    }
    view! {
        <section class="cdt-travaux" aria-label=titre>
            <h4 class="cdt-seance__detail">{titre}</h4>
            <ul class="cdt-liste">
                {travaux
                    .into_iter()
                    .map(|t| {
                        let (etat, ton) = etat_travail(&t.etat);
                        let modifiable = t.etat != "retire";
                        let id = t.id;
                        view! {
                            <li>
                                <strong>{t.titre.clone()}</strong>
                                {format!(" · {} · pour le {} à {}", t.nature, date_longue(t.echeance_jour).to_lowercase(), heure_en_clair(&t.echeance_heure))}
                                {(t.public == "restreint").then_some(" · public restreint")}
                                " "
                                <Badge libelle=etat ton=ton/>
                                {modifiable.then(|| view! { <details class="cdt-seance__saisie"><summary>"Modifier ou retirer"</summary><FormulaireTravail id=id ecriture=ecriture/></details> })}
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </section>
    }
    .into_any()
}

/// Modification d'un travail (chargé à l'ouverture) : avec ou sans avis, confirmation d'une échéance avancée, retrait.
#[component]
fn FormulaireTravail(id: Uuid, ecriture: Ecriture) -> impl IntoView {
    let travail =
        LocalResource::new(move || async move { session::client().cdt_travail(id).await });
    view! {
        {move || match travail.get() {
            None => view! { <Squelette lignes=2 libelle="Chargement du travail…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| travail.refetch())/> }.into_any(),
            Some(Ok(t)) => view! { <EditionTravail travail=t ecriture=ecriture/> }.into_any(),
        }}
    }
}

#[component]
fn EditionTravail(travail: TravailC2Dto, ecriture: Ecriture) -> impl IntoView {
    let id = travail.id;
    let origine = OrigineTravail {
        titre: travail.titre.clone(),
        consigne: travail.consigne.clone(),
        jour: travail.echeance_jour,
        heure: travail.echeance_heure.clone(),
    };
    let titre = RwSignal::new(travail.titre.clone());
    let consigne = RwSignal::new(travail.consigne.clone());
    let jour = RwSignal::new(travail.echeance_jour.to_string());
    let heure = RwSignal::new(travail.echeance_heure.clone());
    let sans_avis = RwSignal::new(false);
    let motif = RwSignal::new(String::new());
    let (e_titre, e_consigne, e_jour, e_heure) = (
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
        RwSignal::new(None::<String>),
    );
    let confirmer_avance = RwSignal::new(false);
    let en_attente = RwSignal::new(None::<ent_web_api::cahier_c2_types::ModifierTravailC2Body>);
    let publie = travail.etat == "publie";
    let envoyer_corps = move |mut corps: ent_web_api::cahier_c2_types::ModifierTravailC2Body,
                              confirme: bool| {
        corps.confirme_echeance_avancee = confirme;
        ecriture.lancer(async move {
            let r = session::client().cdt_travail_modifier(id, &corps).await?;
            Ok(if r.avis_emis {
                "Travail modifié : élèves et familles prévenus.".to_string()
            } else {
                "Travail modifié sans avis (la mention « modifié le » reste visible).".to_string()
            })
        });
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieModification {
            titre: titre.get_untracked(),
            consigne: consigne.get_untracked(),
            jour: jour.get_untracked(),
            heure: heure.get_untracked(),
            sans_avis: sans_avis.get_untracked(),
            motif: motif.get_untracked(),
        };
        match valider_modification(&saisie, &origine) {
            Err(es) => poser(
                &es,
                &[
                    ("titre", e_titre),
                    ("consigne", e_consigne),
                    ("jour", e_jour),
                    ("heure", e_heure),
                ],
            ),
            Ok((corps, avancee)) => {
                for s in [e_titre, e_consigne, e_jour, e_heure] {
                    s.set(None);
                }
                if avancee && publie {
                    en_attente.set(Some(corps));
                    confirmer_avance.set(true);
                } else {
                    envoyer_corps(corps, false);
                }
            }
        }
    };
    let confirmer = Callback::new(move |_| {
        if let Some(c) = en_attente.get_untracked() {
            envoyer_corps(c, true);
        }
    });
    let motif_retrait = RwSignal::new(String::new());
    let confirmer_retrait = RwSignal::new(false);
    let retirer = Callback::new(move |_| {
        let m = motif_retrait.get_untracked();
        ecriture.lancer(async move {
            session::client().cdt_travail_retirer(id, &m).await?;
            Ok(
                "Travail retiré de la diffusion (les copies déjà rendues restent à l'élève)."
                    .to_string(),
            )
        });
    });
    let pref = format!("cdt-t-{id}");
    view! {
        <form class="cdt-form" on:submit=envoyer novalidate=true>
            <ChampTexte id=format!("{pref}-titre") libelle="Titre" valeur=titre erreur=e_titre requis=true/>
            <ZoneTexte id=format!("{pref}-consigne") libelle="Consigne" valeur=consigne erreur=e_consigne requis=true/>
            <ChampTexte id=format!("{pref}-jour") libelle="Échéance : jour" valeur=jour genre="date" erreur=e_jour/>
            <ChampTexte id=format!("{pref}-heure") libelle="Échéance : heure" valeur=heure genre="time" erreur=e_heure/>
            {publie.then(|| view! {
                <Case id=format!("{pref}-sans-avis") libelle="Modification mineure, sans avis (impossible pour une échéance avancée)" coche=sans_avis/>
                <ChampTexte id=format!("{pref}-motif") libelle="Motif (facultatif)" valeur=motif/>
            })}
            <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer la modification"</Bouton>
        </form>
        <Confirmation
            ouvert=confirmer_avance
            id=format!("{pref}-avance")
            titre="Avancer l'échéance ?"
            message="Les élèves et leurs familles seront toujours prévenus d'une échéance avancée."
            libelle_confirmer="Avancer et prévenir"
            on_confirmer=confirmer
        />
        <ChampTexte id=format!("{pref}-motif-retrait") libelle="Motif du retrait (obligatoire)" valeur=motif_retrait/>
        <Bouton variante=Variante::Danger on_click=Callback::new(move |_| confirmer_retrait.set(true))>"Retirer de la diffusion…"</Bouton>
        <Confirmation
            ouvert=confirmer_retrait
            id=format!("{pref}-retrait")
            titre="Retirer ce travail ?"
            message="Le travail n'est plus diffusé ; les copies déjà rendues restent à l'élève."
            libelle_confirmer="Retirer"
            danger=true
            on_confirmer=retirer
        />
    }
}
