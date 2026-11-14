//! Écran C5 « Suivi des rendus » (`/web/cahier/suivi/:id`) : tableau dense du travail à rendre — statut de chaque élève,
//! dernier dépôt, retour — avec les actions du professeur : constats en lot (jamais une note), délai accordé, clôture du
//! ramassage, relance avec confirmation, retours en brouillon puis rendus en un geste, copies à télécharger. Réservé à
//! l'intervenant en saisie sur le cours : le serveur répond 403 à tout autre.
//!
//! Charte : tableau dense pour la saisie, un seul accent bleu, rien de décoratif.

use ent_web_api::cahier_c5_types::{
    CritereDto, RelanceBody, RetourC5Body, SuiviC5Dto,
};
use ent_web_api::depot_fichier;
use ent_web_ui::{
    AccesRefuse, Alerte, Badge, Bouton, Case, ChampTexte, EnTetePage, EtatVide, LienBouton, Niveau,
    Selection, Squelette, Taille, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;

use super::fichiers::{lire_fichier_choisi, FichierChoisi, Lecture};
use super::modele_c5::{
    corps_cloture, corps_constats, corps_prolongation, filtrer_suivi, libelle_retour, libelle_statut,
    nb_selections_masquees, resume_comptes, ton_statut, FiltreSuivi, MOTIFS,
};
use super::rendu::ouvrir_piece_c5;
use crate::noyau::coquille::use_droits;
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;

pub const ADRESSE_SUIVI: &str = "/web/cahier/suivi";

#[component]
pub fn PageSuiviRendus() -> impl IntoView {
    let droits = use_droits();
    let params = use_params_map();
    let id = Memo::new(move |_| {
        params
            .with(|p| p.get("id"))
            .and_then(|s| s.parse::<Uuid>().ok())
    });
    view! {
        <Title text="Suivi des rendus"/>
        {move || {
            if !droits.get().cahier_ecrire {
                return view! { <AccesRefuse message="Le suivi des rendus est réservé aux enseignants du cours."/> }.into_any();
            }
            match id.get() {
                None => view! { <EtatVide titre="Travail introuvable" message="L'adresse de cette page est incorrecte."/> }.into_any(),
                Some(id) => view! { <Suivi travail=id/> }.into_any(),
            }
        }}
    }
}

#[component]
fn Suivi(travail: Uuid) -> impl IntoView {
    let suivi = LocalResource::new(move || async move { session::client().cdt_suivi(travail).await });
    view! {
        {move || match suivi.get() {
            None => view! { <Squelette lignes=8 libelle="Chargement du suivi…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| suivi.refetch())/> }.into_any(),
            Some(Ok(s)) => view! { <Tableau suivi=s on_change=Callback::new(move |_| suivi.refetch())/> }.into_any(),
        }}
    }
}

#[component]
fn Tableau(suivi: SuiviC5Dto, on_change: Callback<()>) -> impl IntoView {
    let travail = suivi.travail_id;
    let ecriture = Ecriture::nouveau(on_change);
    let filtre = RwSignal::new(FiltreSuivi::Tous);
    let lignes = StoredValue::new(suivi.lignes.clone());
    // Un signal de sélection par ligne : cocher une ligne ne redessine que sa case.
    let selection: Vec<(Uuid, RwSignal<bool>)> = suivi.lignes.iter().map(|l| (l.eleve_id, RwSignal::new(false))).collect();
    let selection = StoredValue::new(selection);
    let corrige = RwSignal::new(None::<Uuid>);
    let statut_constat = RwSignal::new("excuse".to_string());
    let jour = RwSignal::new(String::new());
    let heure = RwSignal::new("08:00".to_string());
    let motif = RwSignal::new("amenagement".to_string());
    let cloture_jour = RwSignal::new(String::new());
    let cloture_heure = RwSignal::new("23:59".to_string());
    let apercu_relance = RwSignal::new(None::<String>);
    let aux_familles = RwSignal::new(false);
    let copies_a_lister = RwSignal::new(Vec::<(Uuid, String)>::new());
    let erreur_piece = RwSignal::new(None::<String>);

    let affichees = Memo::new(move |_| filtrer_suivi(&lignes.get_value(), filtre.get()));
    let ids_affichees = move || affichees.get().iter().map(|l| l.eleve_id).collect::<Vec<_>>();
    let ids_selectionnes = move || {
        selection
            .get_value()
            .iter()
            .filter(|(_, s)| s.get_untracked())
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
    };
    let masquees = Memo::new(move |_| {
        let sel: Vec<Uuid> = selection.get_value().iter().filter(|(_, s)| s.get()).map(|(id, _)| *id).collect();
        nb_selections_masquees(&sel, &ids_affichees())
    });

    let constater = move |_| {
        let Some(c) = corps_constats(&ids_selectionnes(), &ids_affichees(), &statut_constat.get_untracked()) else {
            ecriture.erreur.set(Some("Cochez au moins un élève affiché.".into()));
            return;
        };
        ecriture.lancer(async move {
            let n = session::client().cdt_constater(travail, &c).await?;
            Ok(format!("{n} constat(s) enregistré(s)."))
        });
    };
    let prolonger = move |_| {
        let Some(c) = corps_prolongation(ids_selectionnes(), &jour.get_untracked(), &heure.get_untracked(), &motif.get_untracked()) else {
            ecriture.erreur.set(Some("Cochez des élèves et indiquez le jour, l'heure et le motif.".into()));
            return;
        };
        ecriture.lancer(async move {
            let n = session::client().cdt_prolonger(travail, &c).await?;
            Ok(format!("Délai accordé à {n} élève(s)."))
        });
    };
    let fixer_cloture = move |_| {
        let Some(c) = corps_cloture(&cloture_jour.get_untracked(), &cloture_heure.get_untracked()) else {
            ecriture.erreur.set(Some("Indiquez le jour de la clôture du ramassage.".into()));
            return;
        };
        ecriture.lancer(async move {
            session::client().cdt_cloture(travail, &c).await?;
            Ok("Clôture du ramassage enregistrée.".into())
        });
    };
    let rendre_en_lot = move |_| {
        ecriture.lancer(async move {
            let r = session::client().cdt_retours_lot(travail).await?;
            Ok(format!("{} copie(s) rendue(s) : les élèves et leurs familles sont prévenus.", r.rendus))
        });
    };
    // Relance : un aperçu d'abord (rien n'est envoyé), puis la confirmation explicite.
    let previsualiser = move |_| {
        spawn_local(async move {
            let b = RelanceBody { eleve_ids: vec![], aux_familles: aux_familles.get_untracked(), confirmer: false };
            match session::client().cdt_relancer(travail, &b).await {
                Ok(r) => apercu_relance.set(Some(format!(
                    "{} destinataire(s), {} déjà relancé(s) aujourd'hui. Message : « {} »",
                    r.destinataires, r.deja_relances, r.texte
                ))),
                Err(e) => ecriture.erreur.set(Some(e.message_utilisateur())),
            }
        });
    };
    let envoyer_relance = move |_| {
        let aux = aux_familles.get_untracked();
        apercu_relance.set(None);
        ecriture.lancer(async move {
            let b = RelanceBody { eleve_ids: vec![], aux_familles: aux, confirmer: true };
            let r = session::client().cdt_relancer(travail, &b).await?;
            Ok(format!("Relance envoyée à {} élève(s).", r.destinataires))
        });
    };
    let lister_copies = move |_| {
        spawn_local(async move {
            match session::client().cdt_copies(travail).await {
                Ok(l) => copies_a_lister.set(l.into_iter().map(|c| (c.piece_id, c.nom_archive)).collect()),
                Err(e) => ecriture.erreur.set(Some(e.message_utilisateur())),
            }
        });
    };

    let options_motif: Vec<(String, String)> = MOTIFS.iter().map(|(c, l)| (c.to_string(), l.to_string())).collect();
    let titre = suivi.titre.clone();
    let sous_titre = format!("Clôture du ramassage : {}", date_heure_paris(suivi.cloture_ramassage_le));
    let resume = resume_comptes(&suivi.comptes);
    let par_groupe = suivi.remise_par_groupe;
    let vide = suivi.lignes.is_empty();
    view! {
        <EnTetePage surtitre="Suivi des rendus".to_string() titre=titre sous_titre=sous_titre>
            <LienBouton href=navigation::CAHIER variante=Variante::Secondaire taille=Taille::Compacte>"Retour au cahier"</LienBouton>
        </EnTetePage>
        <p class="cdt-resume" role="status">{resume}</p>
        <RetourEcriture ecriture=ecriture/>
        {vide.then(|| view! { <EtatVide titre="Aucun élève concerné" message="Aucun élève n'est dans le public de ce travail."/> })}
        {(!vide).then(|| view! {
            <section class="cdt-outils" aria-label="Actions sur la sélection">
                <div class="cdt-actions" role="group" aria-label="Filtrer le tableau">
                    {[(FiltreSuivi::Tous, "Tous"), (FiltreSuivi::ARelancer, "À relancer"), (FiltreSuivi::ACorriger, "À corriger"), (FiltreSuivi::Signales, "Signalements et demandes")]
                        .into_iter()
                        .map(|(f, l)| view! {
                            <Bouton variante=Variante::Secondaire taille=Taille::Compacte on_click=Callback::new(move |_| filtre.set(f))>{l}</Bouton>
                        })
                        .collect_view()}
                </div>
                {move || (masquees.get() > 0).then(|| view! {
                    <p class="cdt-travail__detail" role="status">{format!("{} sélection(s) masquée(s) par le filtre : elles ne seront pas touchées.", masquees.get())}</p>
                })}
                <div class="cdt-form">
                    <Selection id="suivi-constat" libelle="Constater (fait, jamais une note)" valeur=statut_constat
                        options=vec![("rendu_papier".to_string(), "Rendu sur papier".to_string()), ("excuse".to_string(), "Excusé".to_string()),
                                     ("dispense".to_string(), "Dispensé".to_string()), ("non_rendu".to_string(), "Non rendu (après la clôture)".to_string())]/>
                    <Bouton on_click=Callback::new(constater) en_cours=ecriture.en_cours>"Appliquer aux élèves cochés"</Bouton>
                </div>
                <div class="cdt-form">
                    <ChampTexte id="suivi-jour" libelle="Délai accordé jusqu'au" valeur=jour genre="date"/>
                    <ChampTexte id="suivi-heure" libelle="À" valeur=heure genre="time"/>
                    <Selection id="suivi-motif" libelle="Motif (interne, jamais montré à l'élève)" valeur=motif
                        options=options_motif.clone()/>
                    <Bouton on_click=Callback::new(prolonger) en_cours=ecriture.en_cours>"Accorder le délai"</Bouton>
                </div>
                <div class="cdt-form">
                    <ChampTexte id="suivi-cloture-jour" libelle="Clôture du ramassage le" valeur=cloture_jour genre="date"/>
                    <ChampTexte id="suivi-cloture-heure" libelle="À" valeur=cloture_heure genre="time"/>
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(fixer_cloture) en_cours=ecriture.en_cours>"Fixer la clôture"</Bouton>
                </div>
                <div class="cdt-actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(rendre_en_lot) en_cours=ecriture.en_cours>"Rendre toutes les copies corrigées"</Bouton>
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(lister_copies)>"Copies à télécharger"</Bouton>
                </div>
                <div class="cdt-form">
                    <Case id="suivi-relance-familles" libelle="Prévenir aussi les familles" coche=aux_familles/>
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(previsualiser)>"Relancer les non-rendus (aperçu)"</Bouton>
                    {move || apercu_relance.get().map(|a| view! {
                        <Alerte niveau=Niveau::Info titre=a/>
                        <Bouton on_click=Callback::new(envoyer_relance) en_cours=ecriture.en_cours>"Confirmer l'envoi"</Bouton>
                    })}
                </div>
                {move || erreur_piece.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                {move || {
                    let l = copies_a_lister.get();
                    (!l.is_empty()).then(|| view! {
                        <ul class="cdt-pieces" aria-label="Copies">
                            {l.into_iter().map(|(pid, nom)| {
                                let aria = format!("Télécharger {nom}");
                                view! {
                                    <li><Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=aria
                                        on_click=Callback::new(move |_| ouvrir_piece_c5(pid, erreur_piece))>{nom}</Bouton></li>
                                }
                            }).collect_view()}
                        </ul>
                    })
                }}
            </section>
            <div class="cdt-tableau-defilant">
                <table class="cdt-tableau">
                    <caption class="ui-sr">"Rendus des élèves"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Sélection"</th>
                            <th scope="col">"Élève"</th>
                            {par_groupe.then(|| view! { <th scope="col">"Groupe"</th> })}
                            <th scope="col">"Statut"</th>
                            <th scope="col">"Dernier dépôt"</th>
                            <th scope="col">"Retour"</th>
                            <th scope="col">"Signalements"</th>
                            <th scope="col">"Action"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For each=move || affichees.get() key=|l| l.eleve_id let(l)>
                            {
                                let coche = selection.get_value().iter().find(|(id, _)| *id == l.eleve_id).map(|(_, s)| *s).unwrap_or_else(|| RwSignal::new(false));
                                let statut = l.statut.clone().unwrap_or_else(|| "attendu".into());
                                let nom = format!("{} {}", l.nom, l.prenom);
                                let nom_case = nom.clone();
                                let dernier = l.dernier_depot_le.map(date_heure_paris).unwrap_or_else(|| "—".into());
                                let retour = libelle_retour(&l.retour);
                                let remise = l.remise_id;
                                let eleve = l.eleve_id;
                                let mut avis = Vec::new();
                                if l.signalement_le.is_some() { avis.push("difficulté signalée"); }
                                if l.delai_demande_id.is_some() { avis.push("délai demandé"); }
                                if l.prolonge { avis.push("délai accordé"); }
                                if l.constat_propose.is_some() { avis.push("non rendu à constater"); }
                                let avis = avis.join(" · ");
                                let aria = format!("Corriger la copie de {nom}");
                                view! {
                                    <tr>
                                        <td><Case id=format!("suivi-sel-{eleve}") libelle=format!("Sélectionner {nom_case}") coche=coche/></td>
                                        <th scope="row">{nom}</th>
                                        {par_groupe.then(|| view! { <td>{l.groupe.clone().unwrap_or_default()}</td> })}
                                        <td><Badge libelle=libelle_statut(&statut) ton=ton_statut(&statut)/></td>
                                        <td>{dernier}</td>
                                        <td>{retour}</td>
                                        <td>{avis}</td>
                                        <td>
                                            {remise.map(|r| view! {
                                                <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=aria
                                                    on_click=Callback::new(move |_| corrige.set(Some(r)))>"Corriger"</Bouton>
                                            })}
                                        </td>
                                    </tr>
                                }
                            }
                        </For>
                    </tbody>
                </table>
            </div>
            {move || corrige.get().map(|r| view! { <PanneauRetour remise=r on_fermer=Callback::new(move |_| corrige.set(None)) on_change=on_change/> })}
        })}
    }
}

/// Retour d'une copie : commentaire, copie annotée facultative, brouillon puis rendu ; correction = nouvelle version.
#[component]
fn PanneauRetour(remise: Uuid, on_fermer: Callback<()>, on_change: Callback<()>) -> impl IntoView {
    let ecriture = Ecriture::nouveau(on_change);
    let commentaire = RwSignal::new(String::new());
    let fichier = RwSignal::new(None::<FichierChoisi>);
    let err_fichier = RwSignal::new(None::<String>);
    let avertissements = RwSignal::new(Vec::<String>::new());
    let champ = NodeRef::<leptos::html::Input>::new();
    let courant = LocalResource::new(move || async move { session::client().cdt_retour_lire(remise).await });
    Effect::new(move |_| {
        if let Some(Ok(Some(r))) = courant.get() {
            commentaire.set(r.commentaire.clone());
        }
    });
    let choisir = move |_| {
        let Some(input) = champ.get() else { return };
        spawn_local(async move {
            match lire_fichier_choisi(&input).await {
                Lecture::Aucun => fichier.set(None),
                Lecture::Fichier(f) => {
                    err_fichier.set(None);
                    fichier.set(Some(f));
                }
                Lecture::Refuse(m) => {
                    fichier.set(None);
                    input.set_value("");
                    err_fichier.set(Some(m));
                }
            }
        });
    };
    let enregistrer = move |rendre: bool| {
        let texte = commentaire.get_untracked();
        let f = fichier.get_untracked();
        ecriture.lancer(async move {
            let b = RetourC5Body {
                commentaire: texte,
                criteres: Vec::<CritereDto>::new(),
                montrer_note: false,
                note_id: None,
                fichiers: f.iter().map(|f| depot_fichier(&f.nom, &f.mime, &f.octets)).collect(),
                rendre,
            };
            let r = session::client().cdt_retour_enregistrer(remise, &b).await?;
            avertissements.set(r.avertissements.clone());
            Ok(if rendre { "Retour rendu : l'élève et sa famille seront prévenus.".to_string() } else { "Brouillon du retour enregistré.".to_string() })
        });
    };
    view! {
        <section class="cdt-seance" aria-label="Retour sur la copie">
            <h2 class="cdt-travail__titre">"Retour sur la copie"</h2>
            <RetourEcriture ecriture=ecriture/>
            {move || avertissements.get().into_iter().map(|a| view! { <Alerte niveau=Niveau::Avertissement titre=a/> }).collect_view()}
            {move || match courant.get() {
                Some(Ok(Some(r))) => {
                    let etat = if r.etat == "rendu" { format!("Retour rendu (version {}) : toute correction crée une nouvelle version.", r.version) } else { "Retour en brouillon.".into() };
                    view! { <p class="cdt-travail__detail">{etat}</p> }.into_any()
                }
                _ => ().into_any(),
            }}
            <ZoneTexte id="cdt-retour-commentaire" libelle="Commentaire" valeur=commentaire/>
            <div class="ui-champ">
                <label class="ui-champ__libelle" for="cdt-retour-fichier">"Copie annotée (facultative)"</label>
                <input id="cdt-retour-fichier" class="ui-champ__controle" type="file" node_ref=champ on:change=choisir/>
                {move || err_fichier.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                {move || fichier.get().map(|f| view! { <p class="ui-champ__aide">{format!("Fichier choisi : {}", f.nom)}</p> })}
            </div>
            <div class="cdt-actions">
                <Bouton variante=Variante::Secondaire en_cours=ecriture.en_cours on_click=Callback::new(move |_| enregistrer(false))>"Enregistrer en brouillon"</Bouton>
                <Bouton en_cours=ecriture.en_cours on_click=Callback::new(move |_| enregistrer(true))>"Rendre la copie"</Bouton>
                <Bouton variante=Variante::Secondaire on_click=on_fermer>"Fermer"</Bouton>
            </div>
            <p class="cdt-travail__detail">
                "La note se saisit dans le module Évaluations ; elle n'apparaît ici que si vous la liez et choisissez de la montrer."
            </p>
        </section>
    }
}
