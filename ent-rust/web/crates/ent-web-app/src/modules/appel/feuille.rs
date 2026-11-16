//! Feuille d'appel v2 (socle 12, AS14a) : pointage (présent, absent, retard), « Appel fait »,
//! corrections avec motif et historique, séance non assurée.
//!
//! Flux : ouverture idempotente (`POST /appels`), lecture v2 pré-remplie (`GET /appels/{id}/v2` :
//! situations connues, libellés courts sans motif), saisie locale, puis à « Appel fait » envoi **des
//! seules lignes modifiées** (`POST /appels/{id}/v2/lignes`) et validation (`…/valider`). En cas d'échec
//! partiel, l'appel n'est pas validé et les échecs sont listés. Corrections après validation :
//! `…/v2/corrections` avec la version lue (conflit signalé). Qui peut corriger, dans quel délai : le
//! serveur décide ; l'écran traite le refus.
//! Réactivité fine : un jeu de signaux par ligne.

use chrono::NaiveDate;
use ent_types::assiduite::{CorrigerLigneBody, HistoLigneDto, IncoherenceV2Dto};
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Annonce, Avatar, Badge, BarreAction, Bouton, ChampTexte, Dialogue, EtatVide,
    FiltrePastille, Niveau, Recherche, Selection, Squelette, Statistique, TonEtat, Variante,
    ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::wasm_bindgen::JsCast;
use uuid::Uuid;

use super::modele::{arret_apres, message_arret, Arret};
use super::modele::{cibles_action_groupee, selection_masquee};
use super::modele_v2::{
    confirmation_non_pointes_requise, envois, est_modifiee, etat_affiche, libelle_canal,
    libelle_incoherence, libelle_lieu, libelle_transition, minutes_valides, phrase_ligne, resume,
    Choix, Compteurs, EleveV2, EtatAppel, EtatLigne, FeuilleV2, SaisieV2, MAX_MOTIF_TEXTE,
    MOTIFS_CORRECTION,
};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::temps::date_heure_paris;
use crate::noyau::{ecriture::message_echec, reconnexion, session};

/// Signaux d'une ligne.
#[derive(Clone, Copy)]
struct LigneVue {
    choix: RwSignal<Option<Choix>>,
    minutes: RwSignal<String>,
    selection: RwSignal<bool>,
}

/// Échec (partiel ou total) du dernier enregistrement ; survit au rechargement qui suit.
#[derive(Clone, Debug, PartialEq)]
struct EchecEnvoi {
    message: String,
    echecs: Vec<(String, String)>,
}

#[component]
pub fn FeuilleAppel(creneau_id: Uuid, jour: NaiveDate) -> impl IntoView {
    let dernier = RwSignal::new(None::<String>);
    let echec = RwSignal::new(None::<EchecEnvoi>);
    // `ouvrir_appel` est idempotent : il crée l'appel au premier accès puis le renvoie ; la lecture
    // v2 apporte le pré-remplissage (situations connues) et le récapitulatif.
    let feuille = LocalResource::new(move || async move {
        let ouvert = session::client().ouvrir_appel(creneau_id, jour).await?;
        let a = session::client().appel_v2(ouvert.id).await?;
        Ok::<_, ApiError>(FeuilleV2::depuis_api(&a))
    });

    view! {
        {move || {
            dernier
                .get()
                .map(|heure| {
                    view! {
                        <Alerte niveau=Niveau::Succes titre=format!("Appel fait à {heure}")>
                            <p>
                                "Les absences et retards sont transmis à la vie scolaire ; les familles sont prévenues selon les règles de l'établissement."
                            </p>
                        </Alerte>
                    }
                })
        }}
        {move || match feuille.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement de la liste des élèves…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| feuille.refetch())/> }.into_any(),
            Some(Ok(f)) if f.eleves.is_empty() => {
                view! { <EtatVide titre="Aucun élève inscrit" message="Aucun élève n'est rattaché à cette séance à cette date."/> }.into_any()
            }
            Some(Ok(f)) => {
                let recharger = Callback::new(move |_| feuille.refetch());
                let fait = Callback::new(move |_| {
                    dernier.set(Some(date_heure_paris(chrono::Utc::now())));
                    echec.set(None);
                    feuille.refetch();
                });
                view! { <EcranFeuille feuille=f echec=echec on_fait=fait recharger=recharger/> }.into_any()
            }
        }}
    }
}

#[component]
fn EcranFeuille(
    feuille: FeuilleV2,
    echec: RwSignal<Option<EchecEnvoi>>,
    on_fait: Callback<()>,
    recharger: Callback<()>,
) -> impl IntoView {
    let etat = feuille.etat;
    let en_cours_appel = etat == EtatAppel::EnCours;
    let appel_id = feuille.appel_id;
    let total = feuille.eleves.len();
    let eleves = StoredValue::new(feuille.eleves.clone());
    let lignes: Vec<LigneVue> = feuille
        .eleves
        .iter()
        .map(|e| LigneVue {
            choix: RwSignal::new(None),
            minutes: RwSignal::new(e.minutes.map(|m| m.to_string()).unwrap_or_default()),
            selection: RwSignal::new(false),
        })
        .collect();
    let lignes = StoredValue::new(lignes);

    let recherche = RwSignal::new(String::new());
    let filtre = RwSignal::new(None::<Choix>);
    let annonce = RwSignal::new(String::new());
    let envoi = RwSignal::new(None::<(usize, usize)>);
    let blocages = RwSignal::new(Vec::<String>::new());
    let confirmation = RwSignal::new(false);

    let etats = Memo::new(move |_| {
        eleves.with_value(|e| {
            lignes.with_value(|l| {
                e.iter()
                    .zip(l)
                    .map(|(e, l)| {
                        etat_affiche(
                            e,
                            &SaisieV2 {
                                eleve_id: e.eleve_id,
                                choix: l.choix.get(),
                                minutes: String::new(),
                            },
                        )
                    })
                    .collect::<Vec<EtatLigne>>()
            })
        })
    });
    let compteurs = Memo::new(move |_| Compteurs::de(etats.get()));
    let indices_visibles = Memo::new(move |_| {
        let r = recherche.get();
        let f = filtre.get();
        eleves.with_value(|e| {
            etats.with(|s| {
                e.iter()
                    .zip(s)
                    .enumerate()
                    .filter(|(_, (e, s))| {
                        f.is_none_or(|c| s.choix() == Some(c))
                            && (r.trim().is_empty()
                                || super::modele::correspond(&e.cle_recherche, &r))
                    })
                    .map(|(i, _)| i)
                    .collect::<Vec<usize>>()
            })
        })
    });
    let selection = Memo::new(move |_| {
        lignes.with_value(|l| l.iter().map(|x| x.selection.get()).collect::<Vec<_>>())
    });
    let nb_selection = Memo::new(move |_| {
        selection.with(|s| indices_visibles.with(|v| cibles_action_groupee(s, v).len()))
    });
    let nb_masquees =
        Memo::new(move |_| selection.with(|s| indices_visibles.with(|v| selection_masquee(s, v))));

    let saisies = move || -> Vec<SaisieV2> {
        eleves.with_value(|e| {
            lignes.with_value(|l| {
                e.iter()
                    .zip(l)
                    .map(|(e, l)| SaisieV2 {
                        eleve_id: e.eleve_id,
                        choix: l.choix.get_untracked(),
                        minutes: l.minutes.get_untracked(),
                    })
                    .collect()
            })
        })
    };
    let nb_modifications = Memo::new(move |_| {
        eleves.with_value(|e| {
            lignes.with_value(|l| {
                e.iter()
                    .zip(l)
                    .filter(|(e, l)| {
                        est_modifiee(
                            e,
                            &SaisieV2 {
                                eleve_id: e.eleve_id,
                                choix: l.choix.get(),
                                minutes: l.minutes.get(),
                            },
                        )
                    })
                    .count()
            })
        })
    });

    let appliquer_lot = move |c: Choix| {
        let cibles = selection
            .with_untracked(|s| indices_visibles.with_untracked(|v| cibles_action_groupee(s, v)));
        let n = cibles.len();
        lignes.with_value(|l| {
            for i in cibles {
                l[i].choix.set(Some(c));
                l[i].selection.set(false);
            }
        });
        annonce.set(format!(
            "{n} élève(s) marqué(s) {}",
            c.libelle().to_lowercase()
        ));
    };
    let tout_selectionner = move |valeur: bool| {
        let v = indices_visibles.get_untracked();
        lignes.with_value(|l| {
            for i in v {
                l[i].selection.set(valeur);
            }
        });
    };

    let preparer = move || {
        let s = saisies();
        match eleves.with_value(|e| envois(e, &s)) {
            Ok(_) => {
                blocages.set(vec![]);
                confirmation.set(true);
            }
            Err(b) => blocages.set(
                b.into_iter()
                    .map(|b| format!("{} : {}", b.nom, b.message))
                    .collect(),
            ),
        }
    };

    let enregistrer = move || {
        let s = saisies();
        let Ok(liste) = eleves.with_value(|e| envois(e, &s)) else {
            return;
        };
        let non_pointes = eleves.with_value(|e| confirmation_non_pointes_requise(&resume(e, &s)));
        confirmation.set(false);
        echec.set(None);
        let total_envois = liste.len();
        envoi.set(Some((0, total_envois)));
        spawn_local(async move {
            let client = session::client();
            let mut echecs = Vec::new();
            let mut arret = None;
            let mut tentees = 0;
            for (i, e) in liste.iter().enumerate() {
                tentees = i + 1;
                if let Err(err) = client.pointer_ligne(appel_id, &e.corps).await {
                    if let Some(a) = arret_apres(&err) {
                        arret = Some(a);
                        break;
                    }
                    echecs.push((e.nom.clone(), err.message_utilisateur()));
                }
                envoi.set(Some((i + 1, total_envois)));
            }
            if let Some(a) = arret {
                envoi.set(None);
                if a == Arret::SessionExpiree {
                    reconnexion::signaler_session_perdue();
                }
                echec.set(Some(EchecEnvoi {
                    message: message_arret(a, total_envois - tentees),
                    echecs,
                }));
                recharger.run(());
                return;
            }
            if !echecs.is_empty() {
                envoi.set(None);
                echec.set(Some(EchecEnvoi {
                    message:
                        "Certaines lignes n'ont pas été enregistrées. L'appel n'a pas été validé."
                            .into(),
                    echecs,
                }));
                recharger.run(());
                return;
            }
            match client.valider_appel_v2(appel_id, non_pointes).await {
                Ok(_) => {
                    envoi.set(None);
                    on_fait.run(());
                }
                Err(err) => {
                    envoi.set(None);
                    echec.set(Some(EchecEnvoi {
                        message: format!(
                            "Les lignes sont enregistrées mais l'appel n'a pas pu être validé : {}",
                            message_echec(err)
                        ),
                        echecs: vec![],
                    }));
                    recharger.run(());
                }
            }
        });
    };

    // Corrections (après « Appel fait ») et historique : un seul dialogue chacun, ligne ciblée.
    let cible = RwSignal::new(None::<usize>);
    let corr_ouvert = RwSignal::new(false);
    let corr_choix = RwSignal::new("present".to_string());
    let corr_minutes = RwSignal::new(String::new());
    let corr_motif = RwSignal::new("erreur_saisie".to_string());
    let corr_texte = RwSignal::new(String::new());
    let corr_erreur = RwSignal::new(None::<String>);
    let corr_en_cours = RwSignal::new(false);
    let histo_ouvert = RwSignal::new(false);
    let histo = RwSignal::new(None::<Result<Vec<HistoLigneDto>, String>>);

    let ouvrir_correction = Callback::new(move |i: usize| {
        let e = eleves.with_value(|e| e[i].clone());
        cible.set(Some(i));
        corr_choix.set(
            e.etat
                .choix()
                .map(|c| c.code().to_string())
                .unwrap_or_else(|| "present".into()),
        );
        corr_minutes.set(e.minutes.map(|m| m.to_string()).unwrap_or_default());
        corr_motif.set("erreur_saisie".into());
        corr_texte.set(String::new());
        corr_erreur.set(None);
        corr_ouvert.set(true);
    });
    let ouvrir_historique = Callback::new(move |i: usize| {
        let e = eleves.with_value(|e| e[i].clone());
        cible.set(Some(i));
        histo.set(None);
        histo_ouvert.set(true);
        spawn_local(async move {
            let r = session::client()
                .historique_ligne(appel_id, e.eleve_id)
                .await
                .map_err(message_echec);
            histo.set(Some(r));
        });
    });
    let envoyer_correction = move || {
        let Some(i) = cible.get_untracked() else {
            return;
        };
        let e = eleves.with_value(|e| e[i].clone());
        let Some(choix) = Choix::depuis_code(&corr_choix.get_untracked()) else {
            return;
        };
        let minutes = if choix == Choix::Retard {
            match minutes_valides(&corr_minutes.get_untracked()) {
                Ok(m) => Some(m),
                Err(m) => {
                    corr_erreur.set(Some(m.to_string()));
                    return;
                }
            }
        } else {
            None
        };
        let texte = corr_texte.get_untracked().trim().to_string();
        if texte.chars().count() > MAX_MOTIF_TEXTE {
            corr_erreur.set(Some(format!(
                "Le commentaire ne dépasse pas {MAX_MOTIF_TEXTE} caractères."
            )));
            return;
        }
        let corps = CorrigerLigneBody {
            eleve_id: e.eleve_id,
            etat: choix.code().to_string(),
            minutes,
            lieu_code: None,
            motif_code: Some(corr_motif.get_untracked()),
            motif_texte: (!texte.is_empty()).then_some(texte),
            version_base: Some(e.version),
        };
        corr_en_cours.set(true);
        corr_erreur.set(None);
        spawn_local(async move {
            let r = session::client().corriger_ligne(appel_id, &corps).await;
            corr_en_cours.set(false);
            match r {
                Ok(_) => {
                    corr_ouvert.set(false);
                    annonce.set(format!("Correction enregistrée pour {}", e.nom));
                    recharger.run(());
                }
                Err(err) => corr_erreur.set(Some(message_correction(&err))),
            }
        });
    };

    let options_motifs: Vec<(String, String)> = MOTIFS_CORRECTION
        .iter()
        .map(|(c, l)| (c.to_string(), l.to_string()))
        .collect();
    let options_etats: Vec<(String, String)> = Choix::TOUS
        .iter()
        .map(|c| (c.code().to_string(), c.libelle().to_string()))
        .collect();
    let incoherences = LocalResource::new(move || async move {
        if etat == EtatAppel::Fait {
            session::client().incoherences_appel(appel_id).await
        } else {
            Ok(Vec::<IncoherenceV2Dto>::new())
        }
    });

    let en_cours = Signal::derive(move || envoi.get().is_some());
    let compter = move |c: Choix| Signal::derive(move || compteurs.get().pour(c).to_string());
    let lecture_seule = !en_cours_appel;

    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>

        {(etat == EtatAppel::NonAssuree)
            .then(|| {
                view! {
                    <Alerte niveau=Niveau::Avertissement titre="Séance non assurée">
                        <p>
                            "Cette séance a été annulée : l'appel ne compte plus. Les absences et retards qu'il avait créés sont retirés, et les familles déjà prévenues reçoivent un démenti."
                        </p>
                    </Alerte>
                }
            })}
        {(etat == EtatAppel::Fait)
            .then(|| {
                view! {
                    <Alerte niveau=Niveau::Info titre="L'appel est fait.">
                        <p>
                            "Vous pouvez corriger une ligne le jour même, avec un motif ; la vie scolaire peut corriger ensuite. Chaque correction est conservée dans l'historique."
                        </p>
                    </Alerte>
                }
            })}
        {move || {
            incoherences
                .get()
                .and_then(Result::ok)
                .filter(|l| !l.is_empty())
                .map(|l| {
                    let noms: Vec<(String, &'static str)> = eleves.with_value(|e| {
                        l.iter()
                            .map(|i| {
                                let nom = e
                                    .iter()
                                    .find(|x| x.eleve_id == i.eleve_id)
                                    .map(|x| x.nom.clone())
                                    .unwrap_or_else(|| "Élève".into());
                                (nom, libelle_incoherence(&i.code))
                            })
                            .collect()
                    });
                    view! {
                        <Alerte niveau=Niveau::Avertissement titre="À vérifier après l'appel">
                            <ul>
                                {noms.into_iter().map(|(n, m)| view! { <li>{format!("{n} : {m}")}</li> }).collect_view()}
                            </ul>
                        </Alerte>
                    }
                })
        }}

        <div class="appel-stats">
            <Statistique libelle="Élèves" valeur=Signal::derive(move || compteurs.get().total().to_string())/>
            <Statistique libelle="Présents" valeur=compter(Choix::Present) ton=TonEtat::Ok/>
            <Statistique libelle="Absents" valeur=compter(Choix::Absent) ton=TonEtat::Mal/>
            <Statistique libelle="Retards" valeur=compter(Choix::Retard) ton=TonEtat::Avertissement/>
            <Statistique libelle="Ailleurs" valeur=Signal::derive(move || compteurs.get().ailleurs.to_string())/>
        </div>

        <section class="ui-tableau-cadre" aria-label="Liste des élèves">
            <div class="ui-tableau-outils" role="group" aria-label="Filtres">
                <Recherche id="recherche" libelle="Rechercher un élève" valeur=recherche placeholder="Rechercher un élève"/>
                <div class="appel-pastilles" role="group" aria-label="Filtrer par statut">
                    <FiltrePastille
                        libelle="Tous"
                        nombre=Signal::derive(move || compteurs.get().total())
                        actif=Signal::derive(move || filtre.get().is_none())
                        teinte="tous"
                        on_click=Callback::new(move |_| filtre.set(None))
                    />
                    {Choix::TOUS
                        .into_iter()
                        .map(|c| {
                            view! {
                                <FiltrePastille
                                    libelle=c.libelle_pluriel()
                                    nombre=Signal::derive(move || compteurs.get().pour(c))
                                    actif=Signal::derive(move || filtre.get() == Some(c))
                                    teinte=c.code()
                                    on_click=Callback::new(move |_| {
                                        filtre.update(|f| *f = if *f == Some(c) { None } else { Some(c) })
                                    })
                                />
                            }
                        })
                        .collect_view()}
                </div>
            </div>

            <Show when=move || { nb_selection.get() > 0 && !lecture_seule }>
                <div class="appel-lot" role="region" aria-label="Actions sur la sélection">
                    <span class="appel-lot__compte">{move || format!("{} sélectionné(s)", nb_selection.get())}</span>
                    <Show when=move || { nb_masquees.get() > 0 }>
                        <span class="appel-lot__masques">
                            {move || format!("{} autre(s) sélection(s) masquée(s) par le filtre : non concernée(s)", nb_masquees.get())}
                        </span>
                    </Show>
                    <Bouton variante=Variante::Secondaire taille=ent_web_ui::Taille::Compacte on_click=Callback::new(move |_| appliquer_lot(Choix::Present))>
                        "Marquer présents"
                    </Bouton>
                    <Bouton variante=Variante::Secondaire taille=ent_web_ui::Taille::Compacte on_click=Callback::new(move |_| appliquer_lot(Choix::Absent))>
                        "Marquer absents"
                    </Bouton>
                    <Bouton variante=Variante::Discret taille=ent_web_ui::Taille::Compacte on_click=Callback::new(move |_| tout_selectionner(false))>
                        "Désélectionner"
                    </Bouton>
                </div>
            </Show>

            <div class="ui-tableau-defile">
                <table class="ui-tableau appel-tableau" aria-describedby="aide-clavier">
                    <caption class="ui-sr">{format!("Liste des {total} élèves de la séance")}</caption>
                    <thead>
                        <tr>
                            <th scope="col" class="appel-tableau__selection">
                                <input
                                    type="checkbox"
                                    aria-label="Sélectionner tous les élèves affichés"
                                    disabled=lecture_seule
                                    prop:checked=move || {
                                        let v = indices_visibles.get();
                                        !v.is_empty() && lignes.with_value(|l| v.iter().all(|i| l[*i].selection.get()))
                                    }
                                    on:change=move |ev| tout_selectionner(event_target_checked(&ev))
                                />
                            </th>
                            <th scope="col">"Élève"</th>
                            <th scope="col">"Statut"</th>
                            <th scope="col">{if lecture_seule { "Actions" } else { "Retard" }}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || indices_visibles.get()
                            key=|i| *i
                            children=move |i| {
                                let eleve = eleves.with_value(|e| e[i].clone());
                                let ligne = lignes.with_value(|l| l[i]);
                                view! {
                                    <LigneEleve
                                        index=i
                                        eleve=eleve
                                        ligne=ligne
                                        etat=etat
                                        annonce=annonce
                                        on_corriger=ouvrir_correction
                                        on_historique=ouvrir_historique
                                    />
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>
            <Show when=move || indices_visibles.with(|v| v.is_empty())>
                <EtatVide titre="Aucun élève ne correspond" message="Modifiez la recherche ou le filtre."/>
            </Show>
            <p class="appel-aide" id="aide-clavier">
                {if lecture_seule {
                    "Appel en lecture : utilisez « Corriger » pour modifier une ligne."
                } else {
                    "Raccourcis sur une ligne : P présent, A absent, R retard · Espace coche la ligne. Les élèves déjà prévenus ou localisés sont pré-remplis."
                }}
            </p>
        </section>

        {move || {
            let b = blocages.get();
            (!b.is_empty())
                .then(|| {
                    view! {
                        <Alerte niveau=Niveau::Avertissement titre="Corrigez avant de valider">
                            <ul>{b.into_iter().map(|x| view! { <li>{x}</li> }).collect_view()}</ul>
                        </Alerte>
                    }
                })
        }}
        {move || {
            echec
                .get()
                .map(|EchecEnvoi { message, echecs }| {
                    view! {
                        <Alerte niveau=Niveau::Erreur titre=message>
                            <ul>
                                {echecs.into_iter().map(|(n, m)| view! { <li>{format!("{n} : {m}")}</li> }).collect_view()}
                            </ul>
                        </Alerte>
                    }
                })
        }}

        {en_cours_appel
            .then(|| {
                view! {
                    <BarreAction libelle="Validation de l'appel">
                        <p class="ui-barre-action__texte" aria-live="polite">
                            {move || match envoi.get() {
                                Some((fait, total)) => format!("Enregistrement… {fait}/{total}"),
                                None => {
                                    let c = compteurs.get();
                                    match nb_modifications.get() {
                                        0 => format!("{} présent(s) sur {}. Aucune modification.", c.presents, c.total()),
                                        n => format!("{} absent(s), {} retard(s) · {n} modification(s) non enregistrée(s)", c.absents, c.retards),
                                    }
                                }
                            }}
                        </p>
                        <Bouton desactive=en_cours on_click=Callback::new(move |_| preparer())>
                            "Appel fait"
                        </Bouton>
                    </BarreAction>
                }
            })}

        <Dialogue ouvert=confirmation titre="Valider l'appel" id="confirmer-appel">
            <div class="dialogue__corps">
                {move || {
                    let s = saisies();
                    let r = eleves.with_value(|e| resume(e, &s));
                    view! {
                        <p>
                            {format!(
                                "{} présent(s), {} absent(s), {} retard(s), {} ailleurs.",
                                r.presents, r.absents.len(), r.retards.len(), r.ailleurs,
                            )}
                        </p>
                        {(!r.absents.is_empty() || !r.retards.is_empty())
                            .then(|| {
                                let mut l: Vec<String> = r.absents.iter().map(|n| format!("{n} — absent")).collect();
                                l.extend(r.retards.iter().map(|n| format!("{n} — retard")));
                                view! { <ul class="dialogue__liste">{l.into_iter().map(|n| view! { <li>{n}</li> }).collect_view()}</ul> }
                            })}
                        {(!r.non_pointes.is_empty())
                            .then(|| {
                                view! {
                                    <p class="dialogue__avert">
                                        {format!(
                                            "{} élève(s) n'ont pas été pointés et seront marqués absents : {}.",
                                            r.non_pointes.len(),
                                            r.non_pointes.join(", "),
                                        )}
                                    </p>
                                }
                            })}
                    }
                }}
                <p class="dialogue__avert">
                    "Une fois l'appel fait, les absences sont transmises à la vie scolaire. Vous pourrez corriger une ligne le jour même."
                </p>
            </div>
            <div class="dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| confirmation.set(false))>"Annuler"</Bouton>
                <Bouton on_click=Callback::new(move |_| enregistrer())>"Confirmer : appel fait"</Bouton>
            </div>
        </Dialogue>

        <Dialogue ouvert=corr_ouvert titre="Corriger une ligne" id="corriger-ligne">
            <div class="dialogue__corps">
                <p>
                    {move || {
                        cible
                            .get()
                            .map(|i| eleves.with_value(|e| e[i].nom.clone()))
                            .unwrap_or_default()
                    }}
                </p>
                <Selection
                    id="corr-etat"
                    libelle="Nouvel état"
                    valeur=corr_choix
                    options=options_etats.clone()
                />
                <Show when=move || corr_choix.get() == "retard">
                    <ChampTexte
                        id="corr-minutes"
                        libelle="Minutes de retard"
                        valeur=corr_minutes
                        genre="number"
                        inputmode="numeric"
                    />
                </Show>
                <Selection
                    id="corr-motif"
                    libelle="Motif de la correction"
                    valeur=corr_motif
                    options=options_motifs.clone()
                />
                <ZoneTexte
                    id="corr-texte"
                    libelle="Précision (facultatif, jamais de motif de santé ou de famille)"
                    valeur=corr_texte
                />
                {move || corr_erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
            </div>
            <div class="dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| corr_ouvert.set(false))>"Annuler"</Bouton>
                <Bouton en_cours=corr_en_cours on_click=Callback::new(move |_| envoyer_correction())>"Enregistrer la correction"</Bouton>
            </div>
        </Dialogue>

        <Dialogue ouvert=histo_ouvert titre="Historique de la ligne" id="historique-ligne">
            <div class="dialogue__corps">
                {move || match histo.get() {
                    None => view! { <p>"Chargement…"</p> }.into_any(),
                    Some(Err(m)) => view! { <Alerte niveau=Niveau::Erreur titre=m/> }.into_any(),
                    Some(Ok(l)) if l.is_empty() => view! { <p>"Aucun changement enregistré."</p> }.into_any(),
                    Some(Ok(l)) => view! {
                        <ol class="appel-histo">
                            {l.into_iter().map(|h| {
                                let mut t = libelle_transition(h.etat_avant.as_deref(), &h.etat_apres);
                                if let Some(m) = h.minutes { t.push_str(&format!(" ({m} min)")); }
                                if let Some(l) = h.lieu_code.as_deref() { t.push_str(&format!(" {}", libelle_lieu(l))); }
                                let motif = h.motif_code.as_deref().and_then(|c| {
                                    MOTIFS_CORRECTION.iter().find(|(k, _)| *k == c).map(|(_, l)| *l)
                                });
                                view! {
                                    <li>
                                        <strong>{t}</strong>
                                        {format!(" · {} · {}", date_heure_paris(h.cree_le), libelle_canal(&h.canal))}
                                        {motif.map(|m| format!(" · {m}"))}
                                        {h.motif_texte.map(|m| format!(" · « {m} »"))}
                                    </li>
                                }
                            }).collect_view()}
                        </ol>
                    }.into_any(),
                }}
            </div>
            <div class="dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| histo_ouvert.set(false))>"Fermer"</Bouton>
            </div>
        </Dialogue>
    }
}

/// Message d'une correction refusée : conflit de version, délai dépassé, droit absent.
fn message_correction(e: &ApiError) -> String {
    match e {
        ApiError::Conflit(_) => {
            "Cette ligne a été modifiée entre-temps. Fermez, relisez la feuille, puis recommencez.".into()
        }
        ApiError::Interdit(_) => {
            "Correction refusée : seul l'intervenant peut corriger, le jour même ; ensuite, la vie scolaire.".into()
        }
        autre => autre.message_utilisateur(),
    }
}

#[component]
fn LigneEleve(
    index: usize,
    eleve: EleveV2,
    ligne: LigneVue,
    etat: EtatAppel,
    annonce: RwSignal<String>,
    on_corriger: Callback<usize>,
    on_historique: Callback<usize>,
) -> impl IntoView {
    let lecture = etat != EtatAppel::EnCours;
    let serveur = eleve.etat;
    let nom = eleve.nom.clone();
    let groupe = format!("st-{}", eleve.eleve_id.simple());
    let id_minutes = format!("min-{}", eleve.eleve_id.simple());
    let nom_annonce = nom.clone();
    let eleve_phrase = eleve.clone();
    let eleve_modif = eleve.clone();

    let choisir = move |c: Choix| {
        if lecture {
            return;
        }
        ligne.choix.set(Some(c));
        annonce.set(format!("{nom_annonce} : {}", c.libelle()));
    };
    let choisir_clavier = choisir.clone();
    let au_clavier = move |ev: leptos::ev::KeyboardEvent| {
        // Pas de raccourci pendant la saisie des minutes ou d'un texte.
        let dans_texte = event_target::<web_sys::HtmlElement>(&ev)
            .dyn_into::<web_sys::HtmlInputElement>()
            .map(|i| matches!(i.type_().as_str(), "text" | "number"))
            .unwrap_or(false);
        if dans_texte || ev.ctrl_key() || ev.alt_key() || ev.meta_key() {
            return;
        }
        let touche = ev.key();
        if touche == " " {
            let sur_radio = event_target::<web_sys::HtmlInputElement>(&ev).type_() == "radio";
            if !sur_radio && !lecture {
                ev.prevent_default();
                ligne.selection.update(|v| *v = !*v);
            }
            return;
        }
        if let Some(c) = touche
            .chars()
            .next()
            .filter(|_| touche.chars().count() == 1)
            .and_then(Choix::depuis_touche)
        {
            ev.prevent_default();
            choisir_clavier(c);
        }
    };

    let affiche = Memo::new(move |_| {
        etat_affiche(
            &eleve_phrase,
            &SaisieV2 {
                eleve_id: eleve_phrase.eleve_id,
                choix: ligne.choix.get(),
                minutes: String::new(),
            },
        )
    });
    let modifiee = move || {
        est_modifiee(
            &eleve_modif,
            &SaisieV2 {
                eleve_id: eleve_modif.eleve_id,
                choix: ligne.choix.get(),
                minutes: ligne.minutes.get(),
            },
        )
    };
    let badge = eleve.libelle_court.clone().map(|c| {
        let texte = match (serveur, eleve.lieu.as_deref()) {
            (EtatLigne::Localise, Some(l)) => format!("{c} ({})", libelle_lieu(l)),
            _ => c,
        };
        view! { <Badge libelle=texte ton=TonEtat::Info/> }
    });
    let phrase_eleve = eleve.clone();
    let phrase = move || {
        let e = affiche.get();
        let m = if e == EtatLigne::Retard {
            minutes_valides(&ligne.minutes.get()).ok()
        } else {
            None
        };
        phrase_ligne(&phrase_eleve, e, m)
    };
    let eleve_minutes = eleve.clone();

    view! {
        <tr
            class="appel-ligne"
            class:appel-ligne--modifiee=modifiee.clone()
            class:appel-ligne--selection=move || ligne.selection.get()
            data-statut=move || affiche.get().code()
            on:keydown=au_clavier
        >
            <td class="appel-tableau__selection">
                <input
                    type="checkbox"
                    aria-label=format!("Sélectionner {nom}")
                    disabled=lecture
                    prop:checked=move || ligne.selection.get()
                    on:change=move |ev| ligne.selection.set(event_target_checked(&ev))
                />
            </td>
            <th scope="row" class="appel-ligne__eleve">
                <span class="appel-eleve">
                    <Avatar nom=nom.clone()/>
                    <span class="appel-eleve__nom">{nom.clone()}</span>
                    {badge}
                    <Show when=modifiee.clone()>
                        <span class="appel-eleve__modif">
                            <span aria-hidden="true">"●"</span>
                            <span class="ui-sr">" (modifié)"</span>
                        </span>
                    </Show>
                    <span class="ui-sr">{phrase}</span>
                </span>
            </th>
            <td class="appel-ligne__statut">
                <fieldset class="ui-segmente appel-statuts" disabled=lecture>
                    <legend>{format!("Statut de {nom}")}</legend>
                    {Choix::TOUS
                        .into_iter()
                        .map(|c| {
                            let choisir = choisir.clone();
                            view! {
                                <label class=format!("ui-segmente__option appel-statut appel-statut--{}", c.code())>
                                    <input
                                        type="radio"
                                        name=groupe.clone()
                                        value=c.code()
                                        aria-keyshortcuts=c.touche().to_string()
                                        prop:checked=move || affiche.get().choix() == Some(c)
                                        on:change=move |_| choisir(c)
                                    />
                                    <span>{c.libelle()}</span>
                                </label>
                            }
                        })
                        .collect_view()}
                </fieldset>
            </td>
            <td class="appel-ligne__commentaire">
                {if lecture {
                    view! {
                        <span class="appel-actions">
                            <Bouton variante=Variante::Discret taille=ent_web_ui::Taille::Compacte libelle_accessible=format!("Corriger {}", eleve.nom) on_click=Callback::new(move |_| on_corriger.run(index))>
                                "Corriger"
                            </Bouton>
                            <Bouton variante=Variante::Discret taille=ent_web_ui::Taille::Compacte libelle_accessible=format!("Historique de {}", eleve.nom) on_click=Callback::new(move |_| on_historique.run(index))>
                                "Historique"
                            </Bouton>
                        </span>
                    }
                        .into_any()
                } else {
                    let libelle_min = format!("Minutes de retard de {}", eleve_minutes.nom);
                    view! {
                        <Show when=move || affiche.get() == EtatLigne::Retard>
                            <input
                                id=id_minutes.clone()
                                type="number"
                                inputmode="numeric"
                                min="1"
                                max="600"
                                class="appel-minutes"
                                aria-label=libelle_min.clone()
                                placeholder="min"
                                prop:value=move || ligne.minutes.get()
                                on:input=move |ev| ligne.minutes.set(event_target_value(&ev))
                            />
                        </Show>
                    }
                        .into_any()
                }}
            </td>
        </tr>
    }
}
