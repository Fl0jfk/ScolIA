//! Feuille d'appel : saisie, sélection multiple, validation et résultat.
//!
//! Logique inchangée depuis le prototype (modèle `modele.rs` et ses tests) :
//! statuts saisis localement, puis à la validation envoi ligne par ligne
//! (`POST /appels/{id}/lignes`) et clôture (`POST /appels/{id}/clore`).
//! Réactivité fine : chaque élève a ses propres signaux ; changer un statut ne
//! redessine que sa ligne et les compteurs, même avec plusieurs centaines d'élèves.

use chrono::NaiveDate;
use ent_web_api::{ApiError, Capacites, CAPACITES_API_ACTUELLE};
use ent_web_ui::{
    Alerte, Annonce, Avatar, BarreAction, Bouton, Dialogue, EtatVide, FiltrePastille, Niveau,
    Recherche, Squelette, Statistique, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::wasm_bindgen::JsCast;
use leptos_router::hooks::{use_location, use_navigate};
use uuid::Uuid;

use super::modele::{
    arret_apres, changements, cibles_action_groupee, message_arret, selection_masquee, visibles,
    Arret, Bilan, Blocage, Changement, Compteurs, EleveAppel, Feuille, Filtre, Saisie, Statut,
};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

/// Signaux d'une ligne.
#[derive(Clone, Copy)]
struct LigneVue {
    statut: RwSignal<Statut>,
    commentaire: RwSignal<String>,
    selection: RwSignal<bool>,
}

#[component]
pub fn FeuilleAppel(creneau_id: Uuid, jour: NaiveDate) -> impl IntoView {
    let dernier_bilan = RwSignal::new(None::<(Bilan, String)>);
    let echec = RwSignal::new(None::<EchecEnvoi>);
    // `ouvrir_appel` est idempotent : il crée l'appel au premier accès puis le renvoie.
    let feuille = LocalResource::new(move || async move {
        session::client()
            .ouvrir_appel(creneau_id, jour)
            .await
            .map(|a| Feuille::depuis_api(&a))
    });

    view! {
        {move || {
            dernier_bilan
                .get()
                .map(|(b, heure)| {
                    view! {
                        <Alerte niveau=Niveau::Succes titre=format!("Appel enregistré à {heure}")>
                            <p>
                                {format!(
                                    "{} modification(s) envoyée(s). L'appel est clos ; les absences et retards ont été transmis à la vie scolaire.",
                                    b.envoyes,
                                )}
                            </p>
                        </Alerte>
                    }
                })
        }}
        {move || match feuille.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement de la liste des élèves…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| feuille.refetch())/> }.into_any(),
            Some(Ok(f)) if f.eleves.is_empty() => {
                view! { <EtatVide titre="Aucun élève inscrit" message="Aucun élève n'est rattaché à cette classe à cette date."/> }.into_any()
            }
            Some(Ok(f)) => {
                let on_termine = Callback::new(move |b: Bilan| {
                    let heure = chrono::Local::now().format("%H:%M").to_string();
                    echec.set(None);
                    dernier_bilan.set(Some((b, heure)));
                    feuille.refetch();
                });
                // Échec partiel : on relit l'état enregistré (les lignes envoyées y figurent).
                let on_incomplet = Callback::new(move |_| feuille.refetch());
                view! { <EcranSaisie feuille=f capacites=CAPACITES_API_ACTUELLE echec=echec on_termine=on_termine on_incomplet=on_incomplet/> }.into_any()
            }
        }}
    }
}

/// État de l'enregistrement.
#[derive(Clone, Debug, PartialEq)]
enum Envoi {
    Inactif,
    EnCours { fait: usize, total: usize },
}

/// Échec (partiel ou total) du dernier enregistrement. Porté par `FeuilleAppel` : il
/// survit au rechargement de l'état enregistré qui suit.
#[derive(Clone, Debug, PartialEq)]
struct EchecEnvoi {
    message: String,
    echecs: Vec<(String, String)>,
}

#[component]
fn EcranSaisie(
    feuille: Feuille,
    capacites: Capacites,
    echec: RwSignal<Option<EchecEnvoi>>,
    on_termine: Callback<Bilan>,
    on_incomplet: Callback<()>,
) -> impl IntoView {
    let navigate = use_navigate();
    let location = use_location();
    let close = feuille.close;
    let appel_id = feuille.appel_id;
    let eleves = StoredValue::new(feuille.eleves.clone());
    let lignes: Vec<LigneVue> = feuille
        .eleves
        .iter()
        .map(|e| LigneVue {
            statut: RwSignal::new(e.statut_enregistre),
            commentaire: RwSignal::new(String::new()),
            selection: RwSignal::new(false),
        })
        .collect();
    let lignes = StoredValue::new(lignes);

    let recherche = RwSignal::new(String::new());
    let filtre_statut = RwSignal::new(None::<Statut>);
    let annonce = RwSignal::new(String::new());
    let envoi = RwSignal::new(Envoi::Inactif);
    let blocages = RwSignal::new(Vec::<Blocage>::new());
    let confirmation = RwSignal::new(false);
    let a_valider = RwSignal::new(Vec::<Changement>::new());

    let statuts = Memo::new(move |_| {
        lignes.with_value(|l| l.iter().map(|x| x.statut.get()).collect::<Vec<_>>())
    });
    let compteurs = Memo::new(move |_| Compteurs::de(statuts.get()));
    let indices_visibles = Memo::new(move |_| {
        let filtre = Filtre {
            recherche: recherche.get(),
            statut: filtre_statut.get(),
        };
        eleves.with_value(|e| statuts.with(|s| visibles(e, s, &filtre)))
    });
    let selection = Memo::new(move |_| {
        lignes.with_value(|l| l.iter().map(|x| x.selection.get()).collect::<Vec<_>>())
    });
    // Seules les lignes sélectionnées ET affichées sont concernées par une action groupée.
    let nb_selection = Memo::new(move |_| {
        selection.with(|s| indices_visibles.with(|v| cibles_action_groupee(s, v).len()))
    });
    let nb_masquees =
        Memo::new(move |_| selection.with(|s| indices_visibles.with(|v| selection_masquee(s, v))));
    let nb_modifications = Memo::new(move |_| {
        eleves.with_value(|e| {
            lignes.with_value(|l| {
                e.iter()
                    .zip(l)
                    .filter(|(e, l)| {
                        l.statut.get() != e.statut_enregistre
                            || !l.commentaire.get().trim().is_empty()
                    })
                    .count()
            })
        })
    });

    let appliquer_lot = move |s: Statut| {
        let cibles = selection.with_untracked(|sel| {
            indices_visibles.with_untracked(|v| cibles_action_groupee(sel, v))
        });
        let n = cibles.len();
        lignes.with_value(|l| {
            for i in cibles {
                l[i].statut.set(s);
                l[i].selection.set(false);
            }
        });
        annonce.set(format!(
            "{n} élève(s) marqué(s) {}",
            s.libelle().to_lowercase()
        ));
    };
    let tout_selectionner = move |valeur: bool| {
        let visibles = indices_visibles.get_untracked();
        lignes.with_value(|l| {
            for i in visibles {
                l[i].selection.set(valeur);
            }
        });
    };

    let saisies = move || -> Vec<Saisie> {
        eleves.with_value(|e| {
            lignes.with_value(|l| {
                e.iter()
                    .zip(l)
                    .map(|(e, l)| Saisie {
                        eleve_id: e.eleve_id,
                        statut: l.statut.get_untracked(),
                        commentaire: l.commentaire.get_untracked(),
                    })
                    .collect()
            })
        })
    };

    let preparer = move || {
        let s = saisies();
        match eleves.with_value(|e| changements(e, &s, capacites)) {
            Ok(c) => {
                blocages.set(vec![]);
                a_valider.set(c);
                confirmation.set(true);
            }
            Err(b) => blocages.set(b),
        }
    };

    let enregistrer = move || {
        let liste = a_valider.get_untracked();
        confirmation.set(false);
        echec.set(None);
        let total = liste.len();
        envoi.set(Envoi::EnCours { fait: 0, total });
        let chemin = session::chemin_complet(
            &location.pathname.get_untracked(),
            &location.search.get_untracked(),
        );
        let navigate = navigate.clone();
        spawn_local(async move {
            let client = session::client();
            let mut echecs = Vec::new();
            let mut arret = None;
            let mut tentees = 0;
            for (i, c) in liste.iter().enumerate() {
                tentees = i + 1;
                if let Err(e) = client.marquer(appel_id, c.eleve_id, c.etat).await {
                    if let Some(a) = arret_apres(&e) {
                        arret = Some(a);
                        break;
                    }
                    echecs.push((c.nom.clone(), e.message_utilisateur()));
                }
                envoi.set(Envoi::EnCours { fait: i + 1, total });
            }
            if let Some(a) = arret {
                envoi.set(Envoi::Inactif);
                if a == Arret::SessionExpiree {
                    // La saisie non envoyée ne peut pas être gardée (l'écran disparaît) :
                    // la page de connexion en informe explicitement.
                    session::effacer();
                    navigate(
                        &session::adresse_connexion_saisie_perdue(&chemin),
                        Default::default(),
                    );
                    return;
                }
                echec.set(Some(EchecEnvoi {
                    message: message_arret(a, total - tentees),
                    echecs,
                }));
                on_incomplet.run(());
                return;
            }
            if !echecs.is_empty() {
                envoi.set(Envoi::Inactif);
                echec.set(Some(EchecEnvoi {
                    message:
                        "Certaines lignes n'ont pas été enregistrées. L'appel n'a pas été clos."
                            .into(),
                    echecs,
                }));
                on_incomplet.run(());
                return;
            }
            match client.clore(appel_id).await {
                Ok(_) => {
                    envoi.set(Envoi::Inactif);
                    on_termine.run(Bilan {
                        envoyes: total,
                        echecs: vec![],
                        clos: true,
                    });
                }
                Err(e) => {
                    envoi.set(Envoi::Inactif);
                    if e == ApiError::NonConnecte {
                        session::effacer();
                        navigate(
                            &session::adresse_connexion_saisie_perdue(&chemin),
                            Default::default(),
                        );
                        return;
                    }
                    echec.set(Some(EchecEnvoi {
                        message: format!(
                            "Les lignes sont enregistrées mais l'appel n'a pas pu être clos : {}",
                            e.message_utilisateur()
                        ),
                        echecs: vec![],
                    }));
                    on_incomplet.run(());
                }
            }
        });
    };

    let en_cours = Signal::derive(move || matches!(envoi.get(), Envoi::EnCours { .. }));
    let total = feuille.eleves.len();
    let compter = move |s: Statut| Signal::derive(move || compteurs.get().pour(s).to_string());

    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>

        {close
            .then(|| {
                view! {
                    <Alerte niveau=Niveau::Info titre="Cet appel est clos.">
                        <p>"Il est consultable mais ne peut plus être modifié depuis cet écran."</p>
                    </Alerte>
                }
            })}

        <div class="appel-stats">
            <Statistique libelle="Élèves" valeur=Signal::derive(move || compteurs.get().total().to_string())/>
            <Statistique libelle="Présents" valeur=compter(Statut::Present) ton=TonEtat::Ok/>
            <Statistique libelle="Absents" valeur=compter(Statut::Absent) ton=TonEtat::Mal/>
            <Statistique libelle="Retards" valeur=compter(Statut::Retard) ton=TonEtat::Avertissement/>
        </div>

        <section class="ui-tableau-cadre" aria-label="Liste des élèves">
            <div class="ui-tableau-outils" role="group" aria-label="Filtres">
                <Recherche id="recherche" libelle="Rechercher un élève" valeur=recherche placeholder="Rechercher un élève"/>
                <div class="appel-pastilles" role="group" aria-label="Filtrer par statut">
                    <FiltrePastille
                        libelle="Tous"
                        nombre=Signal::derive(move || compteurs.get().total())
                        actif=Signal::derive(move || filtre_statut.get().is_none())
                        teinte="tous"
                        on_click=Callback::new(move |_| filtre_statut.set(None))
                    />
                    {Statut::TOUS
                        .into_iter()
                        .filter(|s| s.disponible(capacites))
                        .map(|s| {
                            view! {
                                <FiltrePastille
                                    libelle=s.libelle_pluriel()
                                    nombre=Signal::derive(move || compteurs.get().pour(s))
                                    actif=Signal::derive(move || filtre_statut.get() == Some(s))
                                    teinte=s.code()
                                    on_click=Callback::new(move |_| {
                                        filtre_statut.update(|f| *f = if *f == Some(s) { None } else { Some(s) })
                                    })
                                />
                            }
                        })
                        .collect_view()}
                </div>
            </div>

            <Show when=move || { nb_selection.get() > 0 && !close }>
                <div class="appel-lot" role="region" aria-label="Actions sur la sélection">
                    <span class="appel-lot__compte">{move || format!("{} sélectionné(s)", nb_selection.get())}</span>
                    <Show when=move || { nb_masquees.get() > 0 }>
                        <span class="appel-lot__masques">
                            {move || format!("{} autre(s) sélection(s) masquée(s) par le filtre : non concernée(s)", nb_masquees.get())}
                        </span>
                    </Show>
                    {Statut::TOUS
                        .into_iter()
                        .filter(|s| s.disponible(capacites))
                        .map(|s| {
                            view! {
                                <Bouton variante=Variante::Secondaire taille=ent_web_ui::Taille::Compacte on_click=Callback::new(move |_| appliquer_lot(s))>
                                    {format!("Marquer {}", s.libelle().to_lowercase())}
                                </Bouton>
                            }
                        })
                        .collect_view()}
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
                                    disabled=close
                                    prop:checked=move || {
                                        let v = indices_visibles.get();
                                        !v.is_empty() && lignes.with_value(|l| v.iter().all(|i| l[*i].selection.get()))
                                    }
                                    on:change=move |ev| tout_selectionner(event_target_checked(&ev))
                                />
                            </th>
                            <th scope="col">"Élève"</th>
                            <th scope="col">"Statut"</th>
                            {capacites.commentaire.then(|| view! { <th scope="col">"Commentaire"</th> })}
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || indices_visibles.get()
                            key=|i| *i
                            children=move |i| {
                                let eleve = eleves.with_value(|e| e[i].clone());
                                let ligne = lignes.with_value(|l| l[i]);
                                view! { <LigneEleve eleve=eleve ligne=ligne capacites=capacites close=close annonce=annonce/> }
                            }
                        />
                    </tbody>
                </table>
            </div>
            <Show when=move || indices_visibles.with(|v| v.is_empty())>
                <EtatVide titre="Aucun élève ne correspond" message="Modifiez la recherche ou le filtre."/>
            </Show>
            <p class="appel-aide" id="aide-clavier">
                "Raccourcis sur une ligne : P présent, A absent, R retard · Espace coche la ligne. Tous les élèves sont présents par défaut."
            </p>
        </section>

        {(!capacites.dispense || !capacites.commentaire)
            .then(|| {
                view! {
                    <p class="appel-note">
                        "« Dispensé » et les commentaires ne sont pas encore enregistrés par le serveur : ils ne sont pas proposés."
                    </p>
                }
            })}

        {move || {
            let b = blocages.get();
            (!b.is_empty())
                .then(|| {
                    view! {
                        <Alerte niveau=Niveau::Avertissement titre="Certaines saisies ne peuvent pas encore être enregistrées">
                            <ul>
                                {b
                                    .into_iter()
                                    .map(|x| match x {
                                        Blocage::StatutNonPrisEnCharge { nom, statut } => {
                                            view! { <li>{format!("{nom} : « {} » n'est pas encore géré par le serveur.", statut.libelle())}</li> }
                                        }
                                        Blocage::CommentaireNonPrisEnCharge { nom } => {
                                            view! { <li>{format!("{nom} : les commentaires ne sont pas encore enregistrés.")}</li> }
                                        }
                                    })
                                    .collect_view()}
                            </ul>
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

        {(!close)
            .then(|| {
                view! {
                    <BarreAction libelle="Validation de l'appel">
                        <p class="ui-barre-action__texte" aria-live="polite">
                            {move || match envoi.get() {
                                Envoi::EnCours { fait, total } => format!("Enregistrement… {fait}/{total}"),
                                _ => {
                                    let c = compteurs.get();
                                    match nb_modifications.get() {
                                        0 => format!("{} présent(s) sur {}. Aucune modification.", c.present, c.total()),
                                        n => format!("{} absent(s), {} retard(s) · {n} modification(s) non enregistrée(s)", c.absent, c.retard),
                                    }
                                }
                            }}
                        </p>
                        <Bouton desactive=en_cours on_click=Callback::new(move |_| preparer())>
                            "Valider l'appel"
                        </Bouton>
                    </BarreAction>
                }
            })}

        <Dialogue ouvert=confirmation titre="Confirmer l'appel" id="confirmer-appel">
            <div class="dialogue__corps">
                <p>
                    {move || {
                        let c = compteurs.get();
                        format!(
                            "{} présent(s), {} absent(s), {} retard(s) sur {}.",
                            c.present, c.absent, c.retard, c.total(),
                        )
                    }}
                </p>
                {move || {
                    let noms: Vec<String> = eleves.with_value(|e| {
                        lignes.with_value(|l| {
                            e.iter()
                                .zip(l)
                                .filter(|(_, l)| matches!(l.statut.get(), Statut::Absent | Statut::Retard))
                                .map(|(e, l)| format!("{} — {}", e.nom_affiche, l.statut.get().libelle()))
                                .collect()
                        })
                    });
                    (!noms.is_empty())
                        .then(|| {
                            view! {
                                <ul class="dialogue__liste">
                                    {noms.into_iter().map(|n| view! { <li>{n}</li> }).collect_view()}
                                </ul>
                            }
                        })
                }}
                <p class="dialogue__avert">
                    "Une fois validé, l'appel est clos et les absences sont transmises à la vie scolaire."
                </p>
            </div>
            <div class="dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| confirmation.set(false))>
                    "Annuler"
                </Bouton>
                <Bouton on_click=Callback::new(move |_| enregistrer())>"Confirmer et clore l'appel"</Bouton>
            </div>
        </Dialogue>
    }
}

#[component]
fn LigneEleve(
    eleve: EleveAppel,
    ligne: LigneVue,
    capacites: Capacites,
    close: bool,
    annonce: RwSignal<String>,
) -> impl IntoView {
    let enregistre = eleve.statut_enregistre;
    let nom = eleve.nom_affiche.clone();
    let groupe = format!("st-{}", eleve.eleve_id.simple());
    let nom_annonce = nom.clone();
    let choisir = move |s: Statut| {
        if close || !s.disponible(capacites) {
            return;
        }
        ligne.statut.set(s);
        annonce.set(format!("{nom_annonce} : {}", s.libelle()));
    };
    let choisir_clavier = choisir.clone();

    let au_clavier = move |ev: leptos::ev::KeyboardEvent| {
        // Pas de raccourci pendant la saisie d'un commentaire.
        let dans_texte = event_target::<web_sys::HtmlElement>(&ev)
            .dyn_into::<web_sys::HtmlInputElement>()
            .map(|i| i.type_() == "text")
            .unwrap_or(false);
        if dans_texte || ev.ctrl_key() || ev.alt_key() || ev.meta_key() {
            return;
        }
        let touche = ev.key();
        if touche == " " {
            // Sur un bouton radio, Espace sélectionne déjà le statut : on ne touche qu'au reste.
            let sur_radio = event_target::<web_sys::HtmlInputElement>(&ev).type_() == "radio";
            if !sur_radio && !close {
                ev.prevent_default();
                ligne.selection.update(|v| *v = !*v);
            }
            return;
        }
        if let Some(s) = touche
            .chars()
            .next()
            .filter(|_| touche.chars().count() == 1)
            .and_then(Statut::depuis_touche)
        {
            ev.prevent_default();
            choisir_clavier(s);
        }
    };

    view! {
        <tr
            class="appel-ligne"
            class:appel-ligne--modifiee=move || ligne.statut.get() != enregistre
            class:appel-ligne--selection=move || ligne.selection.get()
            data-statut=move || ligne.statut.get().code()
            on:keydown=au_clavier
        >
            <td class="appel-tableau__selection">
                <input
                    type="checkbox"
                    aria-label=format!("Sélectionner {nom}")
                    disabled=close
                    prop:checked=move || ligne.selection.get()
                    on:change=move |ev| ligne.selection.set(event_target_checked(&ev))
                />
            </td>
            <th scope="row" class="appel-ligne__eleve">
                <span class="appel-eleve">
                    <Avatar nom=nom.clone()/>
                    <span class="appel-eleve__nom">{nom.clone()}</span>
                    <Show when=move || ligne.statut.get() != enregistre>
                        <span class="appel-eleve__modif">
                            <span aria-hidden="true">"●"</span>
                            <span class="ui-sr">" (modifié)"</span>
                        </span>
                    </Show>
                </span>
            </th>
            <td class="appel-ligne__statut">
                <fieldset class="ui-segmente appel-statuts" disabled=close>
                    <legend>{format!("Statut de {nom}")}</legend>
                    {Statut::TOUS
                        .into_iter()
                        .filter(|s| s.disponible(capacites))
                        .map(|s| {
                            let choisir = choisir.clone();
                            view! {
                                <label class=format!("ui-segmente__option appel-statut appel-statut--{}", s.code())>
                                    <input
                                        type="radio"
                                        name=groupe.clone()
                                        value=s.code()
                                        aria-keyshortcuts=s.touche().to_string()
                                        prop:checked=move || ligne.statut.get() == s
                                        on:change=move |_| choisir(s)
                                    />
                                    <span>{s.libelle()}</span>
                                </label>
                            }
                        })
                        .collect_view()}
                </fieldset>
            </td>
            {capacites.commentaire.then(|| {
                view! {
                    <td class="appel-ligne__commentaire">
                        <input
                            type="text"
                            aria-label=format!("Commentaire pour {nom}")
                            placeholder="Commentaire (facultatif)"
                            disabled=close
                            prop:value=move || ligne.commentaire.get()
                            on:input=move |ev| ligne.commentaire.set(event_target_value(&ev))
                        />
                    </td>
                }
            })}
        </tr>
    }
}
