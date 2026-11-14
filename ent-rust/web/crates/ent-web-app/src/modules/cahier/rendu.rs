//! Écran C5 « Rendre un travail » (`/web/cahier/rendu/:id`) : la situation d'un élève devant un travail à rendre — échéance
//! effective (« délai accordé jusqu'au… » sans motif), brouillon, dépôt avec accusé, redépôt, retour du professeur, demande
//! de délai, « je n'arrive pas à déposer ». Pour un responsable habilité (`?eleve=`) : la même situation en lecture seule, sans
//! les dépôts (il ne lit que la copie annotée rendue, R-C37). La coche « fait » n'apparaît jamais ici (outil de l'élève).

use ent_web_api::cahier_c5_types::{
    AccuseDto, DepotBody, GroupeLibreBody, PieceC5Dto, RemiseLueDto, RetourLuDto,
};
use ent_web_api::depot_fichier;
use ent_web_ui::{
    Alerte, Annonce, Badge, Bouton, EnTetePage, EtatVide, Niveau, Squelette, Taille, TonEtat,
    Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::{use_params_map, use_query_map};
use uuid::Uuid;

use super::fichiers::{lire_fichier_choisi, FichierChoisi, Lecture};
use super::modele::libelle_taille;
use super::modele_c5::{
    conseil_depot_refuse, libelle_statut, message_accuse, phrase_echeance, ton_statut,
    valider_depot,
};
use super::gestionnaire_echec;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::{date_longue, date_heure_paris};

pub const ADRESSE_RENDU: &str = "/web/cahier/rendu";

#[component]
pub fn PageRendu() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let id = Memo::new(move |_| {
        params
            .with(|p| p.get("id"))
            .and_then(|s| s.parse::<Uuid>().ok())
    });
    let eleve = Memo::new(move |_| {
        query
            .with(|q| q.get("eleve"))
            .and_then(|s| s.parse::<Uuid>().ok())
    });
    view! {
        <Title text="Rendre un travail"/>
        {move || match id.get() {
            None => view! { <EtatVide titre="Travail introuvable" message="L'adresse de cette page est incorrecte."/> }.into_any(),
            Some(id) => view! { <Rendu travail=id eleve=eleve.get()/> }.into_any(),
        }}
    }
}

#[component]
fn Rendu(travail: Uuid, eleve: Option<Uuid>) -> impl IntoView {
    let remise = LocalResource::new(move || async move { session::client().cdt_remise(travail, eleve).await });
    view! {
        {move || match remise.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement de votre travail…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| remise.refetch())/> }.into_any(),
            Some(Ok(r)) => view! { <Contenu remise=r eleve=eleve on_change=Callback::new(move |_| remise.refetch())/> }.into_any(),
        }}
    }
}

#[component]
fn Contenu(remise: RemiseLueDto, eleve: Option<Uuid>, on_change: Callback<()>) -> impl IntoView {
    let famille = eleve.is_some();
    let statut = remise.statut.clone().unwrap_or_else(|| "a_faire".into());
    let jour = date_longue(remise.echeance_jour);
    let phrase = phrase_echeance(&remise, &jour);
    let titre = remise.titre.clone();
    let erreur = RwSignal::new(None::<String>);
    view! {
        <EnTetePage surtitre="Cahier de textes".to_string() titre=titre sous_titre=phrase>
            <Badge libelle=libelle_statut(&statut) ton=ton_statut(&statut)/>
        </EnTetePage>
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        {remise.retour.clone().map(|r| view! { <Retour retour=r erreur=erreur/> })}
        {(!famille).then(|| view! { <Depots remise=remise.clone() erreur=erreur on_change=on_change/> })}
        {famille.then(|| view! { <p class="cdt-resume">"Le dépôt est fait par l'élève. Vous voyez ici l'état du travail et, une fois rendu, le retour du professeur."</p> })}
        {remise.demande_delai.clone().map(|d| {
            let etat = match d.etat.as_str() {
                "acceptee" => "Votre demande de délai a été acceptée.",
                "refusee" => "Votre demande de délai a été refusée.",
                _ => "Votre demande de délai est en attente.",
            };
            view! {
                <section class="cdt-seance" aria-label="Demande de délai">
                    <p>{etat}</p>
                    {d.reponse.map(|r| view! { <p class="cdt-travail__detail">{r}</p> })}
                </section>
            }
        })}
        <Demandes remise=remise.clone() eleve=eleve erreur=erreur on_change=on_change/>
    }
}

/// Retour du professeur : commentaire, grille, note seulement si le professeur l'a choisi, copie annotée.
#[component]
fn Retour(retour: RetourLuDto, erreur: RwSignal<Option<String>>) -> impl IntoView {
    let mis_a_jour = retour.modifie.then(|| {
        retour
            .rendu_le
            .map(|d| format!("Retour mis à jour — {}", date_heure_paris(d)))
            .unwrap_or_else(|| "Retour mis à jour".into())
    });
    view! {
        <section class="cdt-seance" aria-label="Retour du professeur">
            <h2 class="cdt-travail__titre">"Retour du professeur"</h2>
            {mis_a_jour.map(|m| view! { <p class="cdt-travail__detail">{m}</p> })}
            {(!retour.commentaire.is_empty()).then(|| view! { <p class="cdt-copie__mot">{retour.commentaire.clone()}</p> })}
            {(!retour.criteres.is_empty()).then(|| view! {
                <ul class="cdt-liste" aria-label="Grille">
                    {retour.criteres.iter().map(|c| {
                        let niveau = match c.niveau.as_deref() {
                            Some("non_atteint") => "non atteint",
                            Some("partiel") => "partiellement atteint",
                            Some("atteint") => "atteint",
                            Some("depasse") => "dépassé",
                            _ => "",
                        };
                        view! { <li class="cdt-travail__detail">{format!("{} : {niveau}", c.libelle)}</li> }
                    }).collect_view()}
                </ul>
            })}
            {retour.note_valeur.zip(retour.note_sur).map(|(v, s)| view! {
                <p class="cdt-travail__detail">{format!("Note : {v} / {s}")}</p>
            })}
            <Pieces pieces=retour.pieces.clone() erreur=erreur/>
        </section>
    }
}

#[component]
fn Pieces(pieces: Vec<PieceC5Dto>, erreur: RwSignal<Option<String>>) -> impl IntoView {
    view! {
        <ul class="cdt-pieces">
            {pieces.into_iter().map(|p| {
                let pid = p.id;
                let etiquette = format!("{} ({})", p.nom, libelle_taille(p.taille));
                let aria = format!("Télécharger {}", p.nom);
                view! {
                    <li>
                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=aria
                            on_click=Callback::new(move |_| ouvrir_piece_c5(pid, erreur))>
                            {etiquette}
                        </Bouton>
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}

/// Ouvre une pièce par un lien court émis après contrôle des droits (le client ne connaît jamais de clé de stockage).
pub(crate) fn ouvrir_piece_c5(piece: Uuid, erreur: RwSignal<Option<String>>) {
    let echec = gestionnaire_echec(erreur);
    erreur.set(None);
    spawn_local(async move {
        let client = session::client();
        match client.cdt_piece_lien(piece).await {
            Ok(lien) => {
                if let Err(m) = super::fichiers::ouvrir_adresse(&client.adresse_lien_c5(&lien)) {
                    erreur.set(Some(m));
                }
            }
            Err(e) => echec.run(e),
        }
    });
}

/// Brouillon, dépôt et accusés (élève seulement).
#[component]
fn Depots(remise: RemiseLueDto, erreur: RwSignal<Option<String>>, on_change: Callback<()>) -> impl IntoView {
    let travail = remise.travail_id;
    let texte = RwSignal::new(remise.brouillon.as_ref().and_then(|b| b.texte.clone()).unwrap_or_default());
    let commentaire = RwSignal::new(remise.brouillon.as_ref().and_then(|b| b.commentaire.clone()).unwrap_or_default());
    let fichiers = RwSignal::new(Vec::<FichierChoisi>::new());
    let err_texte = RwSignal::new(None::<String>);
    let err_fichier = RwSignal::new(None::<String>);
    let annonce = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let champ = NodeRef::<leptos::html::Input>::new();
    let nb_brouillon = remise.brouillon.as_ref().map(|b| b.pieces.len()).unwrap_or(0);
    let peut = remise.peut_deposer;
    let raison = remise.raison_fermeture.clone();
    let echec = gestionnaire_echec(erreur);
    let formats = remise.formats.clone();

    let choisir = move |_| {
        let Some(input) = champ.get() else { return };
        spawn_local(async move {
            match lire_fichier_choisi(&input).await {
                Lecture::Aucun => {}
                Lecture::Fichier(f) => {
                    err_fichier.set(None);
                    fichiers.update(|l| l.push(f));
                    input.set_value("");
                }
                Lecture::Refuse(m) => {
                    input.set_value("");
                    err_fichier.set(Some(m));
                }
            }
        });
    };
    let corps = move || DepotBody {
        texte: Some(texte.get_untracked()).filter(|t| !t.trim().is_empty()),
        commentaire: Some(commentaire.get_untracked()).filter(|t| !t.trim().is_empty()),
        fichiers: fichiers
            .get_untracked()
            .iter()
            .map(|f| depot_fichier(&f.nom, &f.mime, &f.octets))
            .collect(),
    };
    let brouillon = move |_| {
        if en_cours.get_untracked() {
            return;
        }
        erreur.set(None);
        let c = corps();
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client().cdt_brouillon(travail, &c).await;
            en_cours.set(false);
            match r {
                Ok(_) => {
                    annonce.set("Brouillon enregistré : seul le dépôt compte.".into());
                    fichiers.set(vec![]);
                    on_change.run(());
                }
                Err(e) => echec.run(e),
            }
        });
    };
    let deposer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        erreur.set(None);
        if let Err(m) = valider_depot(&texte.get_untracked(), fichiers.get_untracked().len(), nb_brouillon) {
            err_texte.set(Some(m));
            return;
        }
        err_texte.set(None);
        let c = corps();
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client().cdt_deposer(travail, &c).await;
            en_cours.set(false);
            match r {
                Ok(a) => {
                    annonce.set(message_accuse(a.numero, a.en_retard, a.nb_fichiers));
                    fichiers.set(vec![]);
                    texte.set(String::new());
                    on_change.run(());
                }
                Err(e) => {
                    echec.run(e);
                    err_fichier.set(Some(conseil_depot_refuse().to_string()));
                }
            }
        });
    };
    view! {
        <Annonce message=Signal::derive(move || annonce.get())/>
        {move || (!annonce.get().is_empty()).then(|| view! { <Alerte niveau=Niveau::Succes titre=annonce.get()/> })}
        {remise.groupe.clone().map(|g| {
            let membres = g.membres.iter().map(|m| format!("{} {}", m.prenom, m.nom)).collect::<Vec<_>>().join(", ");
            view! { <p class="cdt-resume">{format!("Remise du groupe « {} » : {membres}. Un dépôt vaut pour tout le groupe.", g.libelle)}</p> }
        })}
        {(remise.remise_par_groupe && remise.groupe.is_none()).then(|| view! { <InscriptionGroupe travail=travail erreur=erreur on_change=on_change/> })}
        {(!peut).then(|| view! { <Alerte niveau=Niveau::Info titre=raison.clone().unwrap_or_else(|| "Le dépôt est fermé.".into())/> })}
        {peut.then(|| view! {
            <section class="cdt-seance" aria-label="Déposer votre travail">
                <h2 class="cdt-travail__titre">"Votre dépôt"</h2>
                <form class="cdt-form" on:submit=deposer novalidate=true>
                    <ZoneTexte id="cdt-rendu-texte" libelle="Texte (facultatif si vous joignez un fichier)" valeur=texte erreur=err_texte/>
                    <ZoneTexte id="cdt-rendu-commentaire" libelle="Mot pour votre professeur (facultatif)" valeur=commentaire/>
                    <div class="ui-champ">
                        <label class="ui-champ__libelle" for="cdt-rendu-fichier">"Ajouter un fichier"</label>
                        <input id="cdt-rendu-fichier" class="ui-champ__controle" type="file" node_ref=champ
                            aria-describedby="cdt-rendu-fichier-aide" on:change=choisir/>
                        <p id="cdt-rendu-fichier-aide" class="ui-champ__aide">
                            {if formats.is_empty() { "PDF, image ou document. 10 Mo au maximum. Le contenu est contrôlé par le serveur.".to_string() } else { format!("Formats acceptés : {}. 10 Mo au maximum.", formats.join(", ")) }}
                        </p>
                        {move || err_fichier.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                        <ul class="cdt-liste" aria-label="Fichiers à déposer">
                            {move || fichiers.get().iter().enumerate().map(|(i, f)| {
                                let aria = format!("Retirer {}", f.nom);
                                view! {
                                    <li class="cdt-travail__detail">
                                        {format!("{} ({})", f.nom, libelle_taille(f.octets.len() as i64))}" "
                                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte libelle_accessible=aria
                                            on_click=Callback::new(move |_| fichiers.update(|l| { if i < l.len() { l.remove(i); } }))>
                                            "Retirer"
                                        </Bouton>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    </div>
                    <div class="cdt-actions">
                        <Bouton soumettre=true en_cours=en_cours>"Déposer"</Bouton>
                        <Bouton variante=Variante::Secondaire en_cours=en_cours on_click=Callback::new(brouillon)>"Enregistrer le brouillon"</Bouton>
                    </div>
                </form>
            </section>
        })}
        {remise.brouillon.as_ref().map(|b| view! {
            <section class="cdt-seance" aria-label="Fichiers du brouillon">
                <p class="cdt-travail__detail">"Fichiers en attente (ils rejoindront votre dépôt) :"</p>
                <Pieces pieces=b.pieces.clone() erreur=erreur/>
            </section>
        })}
        <ListeAccuses accuses=remise.depots.clone() erreur=erreur/>
    }
}

/// Dépôts déjà faits, du plus récent au plus ancien : accusé (numéro, heure, empreinte), fichiers, « déposé par … ».
#[component]
fn ListeAccuses(accuses: Vec<AccuseDto>, erreur: RwSignal<Option<String>>) -> impl IntoView {
    if accuses.is_empty() {
        return ().into_any();
    }
    view! {
        <section class="cdt-seance" aria-label="Vos dépôts">
            <h2 class="cdt-travail__titre">"Vos dépôts"</h2>
            <ul class="cdt-liste">
                {accuses.into_iter().map(|a| {
                    let en_tete = format!(
                        "Dépôt n° {} — {} à {}{}{}",
                        a.numero,
                        date_longue(a.recu_jour),
                        a.recu_heure.format("%H h %M"),
                        if a.courant { " (dépôt courant)" } else { "" },
                        a.depose_par.as_ref().map(|p| format!(" — déposé par {p}")).unwrap_or_default(),
                    );
                    let empreinte = format!("Empreinte {}", a.empreinte.chars().take(12).collect::<String>());
                    view! {
                        <li class="cdt-travail">
                            <p class="cdt-travail__titre">{en_tete}</p>
                            {a.en_retard.then(|| view! { <Badge libelle="En retard" ton=TonEtat::Avertissement/> })}
                            <p class="cdt-travail__detail">{empreinte}</p>
                            {a.texte.map(|t| view! { <p class="cdt-copie__mot">{t}</p> })}
                            <Pieces pieces=a.pieces erreur=erreur/>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </section>
    }
    .into_any()
}

/// Inscription libre dans un groupe : créer un groupe ou rejoindre celui d'un camarade (taille fixée par le professeur).
#[component]
fn InscriptionGroupe(travail: Uuid, erreur: RwSignal<Option<String>>, on_change: Callback<()>) -> impl IntoView {
    let libelle = RwSignal::new(String::new());
    let echec = gestionnaire_echec(erreur);
    let groupes = LocalResource::new(move || async move { session::client().cdt_groupes(travail).await });
    let rejoindre = move |b: GroupeLibreBody| {
        spawn_local(async move {
            match session::client().cdt_groupe_rejoindre(travail, &b).await {
                Ok(_) => on_change.run(()),
                Err(e) => echec.run(e),
            }
        });
    };
    view! {
        <section class="cdt-seance" aria-label="Groupes">
            <h2 class="cdt-travail__titre">"Ce travail se rend par groupe"</h2>
            {move || match groupes.get() {
                Some(Ok(l)) => view! {
                    <ul class="cdt-liste">
                        {l.into_iter().map(|g| {
                            let id = g.id;
                            let membres = g.membres.iter().map(|m| format!("{} {}", m.prenom, m.nom)).collect::<Vec<_>>().join(", ");
                            let complet = g.complet;
                            let aria = format!("Rejoindre le groupe {}", g.libelle);
                            view! {
                                <li class="cdt-travail__detail">
                                    {format!("{} : {membres}", g.libelle)}" "
                                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte desactive=complet libelle_accessible=aria
                                        on_click=Callback::new(move |_| rejoindre(GroupeLibreBody { groupe_id: Some(id), libelle: None }))>
                                        {if complet { "Complet" } else { "Rejoindre" }}
                                    </Bouton>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }.into_any(),
                _ => ().into_any(),
            }}
            <div class="cdt-form">
                <div class="ui-champ">
                    <label class="ui-champ__libelle" for="cdt-groupe-libelle">"Créer un groupe"</label>
                    <input id="cdt-groupe-libelle" class="ui-champ__controle" type="text" prop:value=move || libelle.get()
                        on:input=move |ev| libelle.set(event_target_value(&ev))/>
                </div>
                <Bouton on_click=Callback::new(move |_| {
                    let l = libelle.get_untracked();
                    if !l.trim().is_empty() {
                        rejoindre(GroupeLibreBody { groupe_id: None, libelle: Some(l) });
                    }
                })>"Créer et rejoindre"</Bouton>
            </div>
        </section>
    }
}

/// « Je n'arrive pas à déposer » et demande de délai : visibles tant que l'échéance n'est pas close pour l'élève ; un
/// responsable peut demander un délai pour son enfant mais ne dépose jamais.
#[component]
fn Demandes(remise: RemiseLueDto, eleve: Option<Uuid>, erreur: RwSignal<Option<String>>, on_change: Callback<()>) -> impl IntoView {
    let travail = remise.travail_id;
    let message = RwSignal::new(String::new());
    let annonce = RwSignal::new(String::new());
    let echec = gestionnaire_echec(erreur);
    let famille = eleve.is_some();
    let deja_demande = remise.demande_delai.as_ref().is_some_and(|d| d.etat == "ouverte");
    if remise.rendu != "en_ligne" {
        return ().into_any();
    }
    let signaler = move |_| {
        spawn_local(async move {
            let m = Some(message.get_untracked()).filter(|t| !t.trim().is_empty());
            match session::client().cdt_depot_signaler(travail, m).await {
                Ok(_) => annonce.set("Votre professeur est prévenu, avec l'heure de votre signalement.".into()),
                Err(e) => echec.run(e),
            }
        });
    };
    let demander = move |_| {
        spawn_local(async move {
            let m = Some(message.get_untracked()).filter(|t| !t.trim().is_empty());
            match session::client().cdt_delai_demander(travail, eleve, m).await {
                Ok(_) => {
                    annonce.set("Votre demande de délai est transmise au professeur.".into());
                    on_change.run(());
                }
                Err(e) => echec.run(e),
            }
        });
    };
    view! {
        <section class="cdt-seance" aria-label="Une difficulté ?">
            <h2 class="cdt-travail__titre">"Une difficulté ?"</h2>
            <Annonce message=Signal::derive(move || annonce.get())/>
            <ZoneTexte id="cdt-rendu-difficulte" libelle="Message pour le professeur (facultatif)" valeur=message/>
            <div class="cdt-actions">
                {(!famille).then(|| view! {
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(signaler)>"Je n'arrive pas à déposer"</Bouton>
                })}
                <Bouton variante=Variante::Secondaire desactive=deja_demande on_click=Callback::new(demander)>"Demander un délai"</Bouton>
            </div>
            {move || (!annonce.get().is_empty()).then(|| view! { <p class="cdt-travail__detail">{annonce.get()}</p> })}
        </section>
    }
    .into_any()
}
