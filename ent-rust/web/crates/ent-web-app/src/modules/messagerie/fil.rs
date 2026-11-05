//! Conversation : messages dans l'ordre, pièces jointes, réponse, archivage et classement.
//! L'ouverture de la conversation enregistre l'accusé de lecture côté serveur (`GET /messages/fils/{id}`).

use ent_web_api::{ApiError, DetailFil, MessageFil};
use ent_web_ui::{
    use_notifications, Alerte, Avatar, Badge, Bouton, ChampTexte, Confirmation, Dialogue,
    EnTetePage, Niveau, Selection, Squelette, TonEtat, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::{use_location, use_navigate, use_params_map};
use uuid::Uuid;

use super::fichiers::{enregistrer_telechargement, lire_fichier_choisi, FichierChoisi, Lecture};
use super::modele::{
    arret_definitif, auteur_affiche, en_local, libelle_accuses, libelle_date_heure,
    message_apres_envoi, nom_fichier_sur, ordonner_messages, taille_lisible, valider_dossier,
    valider_message, valider_priorite, IssuePiece, MIME_PIECES, PIECE_MAX_OCTETS,
};
use super::{focaliser, focaliser_si_perdu, traiter_echec};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::non_lus::use_non_lus;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

#[component]
pub fn PageConversation() -> impl IntoView {
    let params = use_params_map();
    let id =
        Memo::new(move |_| params.with(|p| p.get("id").and_then(|s| Uuid::parse_str(&s).ok())));
    let moi = session::use_session().moi;
    let mon_id = Memo::new(move |_| moi.get().and_then(Result::ok).map(|m| m.user_id));
    let non_lus = use_non_lus();

    let detail: RwSignal<Option<DetailFil>> = RwSignal::new(None);
    let erreur: RwSignal<Option<ApiError>> = RwSignal::new(None);

    let charger = Callback::new(move |_: ()| {
        let Some(fil_id) = id.get_untracked() else {
            erreur.set(Some(ApiError::Introuvable("conversation".into())));
            return;
        };
        spawn_local(async move {
            match session::client().detail_fil(fil_id).await {
                Ok(d) => {
                    let premiere_fois = detail.with_untracked(Option::is_none);
                    erreur.set(None);
                    detail.set(Some(d));
                    if premiere_fois {
                        focaliser_si_perdu("contenu");
                    }
                    // L'ouverture a enregistré les accusés : le compteur de la navigation baisse.
                    if let Some(n) = non_lus {
                        n.rafraichir.run(());
                    }
                }
                Err(e) => erreur.set(Some(e)),
            }
        });
    });
    Effect::new(move |_| {
        id.track();
        detail.set(None);
        erreur.set(None);
        charger.run(());
    });

    let titre = move || {
        detail
            .with(|d| d.as_ref().map(|d| d.fil.sujet.clone()))
            .unwrap_or_else(|| "Conversation".to_string())
    };

    view! {
        <Title text=titre/>
        <nav class="fil-ariane" aria-label="Fil d'Ariane">
            <a href=navigation::MESSAGES>"Messages"</a>
            <span aria-hidden="true">" › "</span>
            <span aria-current="page">"Conversation"</span>
        </nav>
        {move || {
            erreur
                .get()
                .map(|e| view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| {
                        erreur.set(None);
                        charger.run(());
                    })/>
                })
        }}
        {move || {
            (detail.with(Option::is_none) && erreur.with(Option::is_none))
                .then(|| view! { <Squelette lignes=4 libelle="Chargement de la conversation…"/> })
        }}
        <Show when=move || detail.with(Option::is_some)>
            <Conversation detail=detail mon_id=mon_id/>
        </Show>
    }
}

#[component]
fn Conversation(detail: RwSignal<Option<DetailFil>>, mon_id: Memo<Option<Uuid>>) -> impl IntoView {
    let droits = use_droits();
    let navigate = use_navigate();
    let location = use_location();
    let notifications = use_notifications();
    let non_lus = use_non_lus();

    let fil = Memo::new(move |_| detail.with(|d| d.as_ref().map(|d| d.fil.clone())));
    let fil_id = move || fil.get().map(|f| f.id);
    let archive = Memo::new(move |_| fil.get().map(|f| f.archive).unwrap_or(false));
    let messages = Memo::new(move |_| {
        detail.with(|d| {
            d.as_ref()
                .map(|d| ordonner_messages(d.messages.clone()))
                .unwrap_or_default()
        })
    });

    // --- Élève concerné : la route du fil ne donne pas son nom --------------------------------
    // Personnel : libellés d'élèves ; famille : ses enfants ; élève : lui-même (pas de nom à chercher).
    let eleve_libelle: RwSignal<Option<String>> =
        RwSignal::new(fil.get_untracked().and_then(|f| f.eleve_label));
    if eleve_libelle.with_untracked(Option::is_none) {
        if let Some(eleve_id) = fil.get_untracked().map(|f| f.eleve_id) {
            let personnel = droits.get_untracked().messagerie_personnel;
            spawn_local(async move {
                let client = session::client();
                let libelle = if personnel {
                    client
                        .messagerie_libelles_eleves(&[eleve_id])
                        .await
                        .ok()
                        .and_then(|l| l.into_iter().find(|x| x.id == eleve_id))
                        .map(|x| x.label)
                } else {
                    client
                        .enfants_famille()
                        .await
                        .ok()
                        .and_then(|l| l.into_iter().find(|e| e.id == eleve_id))
                        .map(|e| format!("{} {}", e.prenom, e.nom))
                };
                eleve_libelle.set(libelle);
            });
        }
    }

    // --- Messages d'erreur d'actions (archivage, classement, téléchargement) -------------
    let erreur_action: RwSignal<Option<String>> = RwSignal::new(None);
    let en_action = RwSignal::new(false);

    // --- Téléchargement d'une pièce jointe ------------------------------------------------
    let telechargement: RwSignal<Option<Uuid>> = RwSignal::new(None);
    let telecharger = Callback::new({
        let navigate = navigate.clone();
        move |(piece_id, nom, mime): (Uuid, String, String)| {
            if telechargement.get_untracked().is_some() {
                return;
            }
            telechargement.set(Some(piece_id));
            erreur_action.set(None);
            let navigate = navigate.clone();
            let chemin = location.pathname.get_untracked();
            spawn_local(async move {
                match session::client().telecharger_piece(piece_id).await {
                    Ok(octets) => {
                        if let Err(m) =
                            enregistrer_telechargement(&nom_fichier_sur(&nom), &mime, &octets)
                        {
                            erreur_action.set(Some(m));
                        }
                    }
                    Err(e) => erreur_action.set(Some(traiter_echec(&e, &navigate, &chemin))),
                }
                telechargement.set(None);
            });
        }
    });

    // --- Archiver -------------------------------------------------------------------------
    let confirmer_archivage = RwSignal::new(false);
    let archiver = Callback::new({
        let navigate = navigate.clone();
        move |_: ()| {
            let Some(fid) = fil_id() else { return };
            if en_action.get_untracked() {
                return;
            }
            en_action.set(true);
            erreur_action.set(None);
            let navigate = navigate.clone();
            let chemin = location.pathname.get_untracked();
            spawn_local(async move {
                match session::client().archiver_fil(fid).await {
                    Ok(_) => {
                        notifications.succes("Conversation archivée.");
                        if let Some(n) = non_lus {
                            n.rafraichir.run(());
                        }
                        navigate(navigation::MESSAGES, Default::default());
                    }
                    Err(e) => {
                        erreur_action.set(Some(traiter_echec(&e, &navigate, &chemin)));
                        focaliser("msg-erreur-action");
                    }
                }
                en_action.set(false);
            });
        }
    });

    // --- Classer (priorité et dossier) ----------------------------------------------------
    let classement_ouvert = RwSignal::new(false);
    let priorite = RwSignal::new("normale".to_string());
    let dossier = RwSignal::new(String::new());
    let erreur_dossier: RwSignal<Option<String>> = RwSignal::new(None);
    let ouvrir_classement = Callback::new(move |_: ()| {
        if let Some(f) = fil.get_untracked() {
            priorite.set(valider_priorite(&f.priorite).to_string());
            dossier.set(f.dossier.unwrap_or_default());
            erreur_dossier.set(None);
            classement_ouvert.set(true);
        }
    });
    let classer = {
        let navigate = navigate.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            let Some(fid) = fil_id() else { return };
            if en_action.get_untracked() {
                return;
            }
            let dossier_valide = match valider_dossier(&dossier.get_untracked()) {
                Ok(d) => d,
                Err(m) => {
                    erreur_dossier.set(Some(m));
                    return;
                }
            };
            erreur_dossier.set(None);
            en_action.set(true);
            let priorite_choisie = valider_priorite(&priorite.get_untracked());
            let navigate = navigate.clone();
            let chemin = location.pathname.get_untracked();
            spawn_local(async move {
                match session::client()
                    .classer_fil(fid, priorite_choisie, dossier_valide.as_deref())
                    .await
                {
                    Ok(classe) => {
                        // La réponse ne porte ni libellé d'élève ni compteur : on ne reprend que le classement.
                        detail.update(|d| {
                            if let Some(d) = d {
                                d.fil.priorite = classe.priorite;
                                d.fil.dossier = classe.dossier;
                            }
                        });
                        classement_ouvert.set(false);
                        notifications.succes("Classement enregistré.");
                    }
                    Err(e) => erreur_dossier.set(Some(traiter_echec(&e, &navigate, &chemin))),
                }
                en_action.set(false);
            });
        }
    };

    // --- Réponse --------------------------------------------------------------------------
    let corps = RwSignal::new(String::new());
    let erreur_corps: RwSignal<Option<String>> = RwSignal::new(None);
    let erreur_envoi: RwSignal<Option<String>> = RwSignal::new(None);
    let erreur_fichier: RwSignal<Option<String>> = RwSignal::new(None);
    let fichier: RwSignal<Option<FichierChoisi>> = RwSignal::new(None);
    let en_envoi = RwSignal::new(false);
    let champ_fichier = NodeRef::<leptos::html::Input>::new();

    let choisir_fichier = move |_| {
        erreur_fichier.set(None);
        let Some(champ) = champ_fichier.get() else {
            return;
        };
        spawn_local(async move {
            match lire_fichier_choisi(&champ).await {
                Lecture::Aucun => fichier.set(None),
                Lecture::Fichier(f) => fichier.set(Some(f)),
                Lecture::Refuse(m) => {
                    fichier.set(None);
                    champ.set_value("");
                    erreur_fichier.set(Some(m));
                }
            }
        });
    };
    let retirer_fichier = Callback::new(move |_: ()| {
        fichier.set(None);
        erreur_fichier.set(None);
        if let Some(champ) = champ_fichier.get_untracked() {
            champ.set_value("");
        }
        focaliser("msg-fichier");
    });

    let envoyer = {
        let navigate = navigate.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            if en_envoi.get_untracked() {
                return;
            }
            let Some(fid) = fil_id() else { return };
            let texte = match valider_message(&corps.get_untracked()) {
                Ok(t) => t,
                Err(m) => {
                    erreur_corps.set(Some(m));
                    focaliser("msg-reponse");
                    return;
                }
            };
            erreur_corps.set(None);
            erreur_envoi.set(None);
            en_envoi.set(true);
            let piece = fichier.get_untracked();
            let navigate = navigate.clone();
            let chemin = location.pathname.get_untracked();
            spawn_local(async move {
                let client = session::client();
                let message = match client.repondre_fil(fid, &texte).await {
                    Ok(m) => m,
                    Err(e) => {
                        // Rien n'est envoyé : le texte reste dans le champ. Sur 401/403/429 on s'arrête là.
                        erreur_envoi.set(Some(traiter_echec(&e, &navigate, &chemin)));
                        en_envoi.set(false);
                        focaliser("msg-erreur-envoi");
                        return;
                    }
                };
                let issue = match piece {
                    None => IssuePiece::SansPiece,
                    Some(p) => match client
                        .joindre_piece(message.id, &p.nom, &p.mime, &p.octets)
                        .await
                    {
                        Ok(_) => IssuePiece::Jointe,
                        Err(e) => IssuePiece::Echec(traiter_echec(&e, &navigate, &chemin)),
                    },
                };
                corps.set(String::new());
                fichier.set(None);
                if let Some(champ) = champ_fichier.get_untracked() {
                    champ.set_value("");
                }
                // Relecture du fil : le message (et sa pièce) y figurent avec leur auteur.
                match client.detail_fil(fid).await {
                    Ok(d) => detail.set(Some(d)),
                    Err(e) if arret_definitif(&e) => {
                        erreur_envoi.set(Some(traiter_echec(&e, &navigate, &chemin)));
                    }
                    Err(_) => {}
                }
                match &issue {
                    IssuePiece::Echec(_) => {
                        notifications.info(message_apres_envoi(&issue));
                    }
                    _ => {
                        notifications.succes(message_apres_envoi(&issue));
                    }
                }
                if let Some(n) = non_lus {
                    n.rafraichir.run(());
                }
                en_envoi.set(false);
                focaliser("msg-reponse");
            });
        }
    };

    view! {
        <EnTetePage
            titre=Signal::derive(move || fil.get().map(|f| f.sujet).unwrap_or_default())
            sous_titre=Signal::derive(move || {
                eleve_libelle
                    .get()
                    .map(|e| format!("À propos de {e}"))
                    .unwrap_or_default()
            })
            compacte=true
        >
            {move || {
                droits.get().messagerie_personnel.then(|| view! {
                    <Bouton variante=Variante::Secondaire on_click=ouvrir_classement>"Classer"</Bouton>
                    {move || {
                        (!archive.get())
                            .then(|| view! {
                                <Bouton
                                    variante=Variante::Secondaire
                                    en_cours=en_action
                                    on_click=Callback::new(move |_| confirmer_archivage.set(true))
                                >
                                    "Archiver"
                                </Bouton>
                            })
                    }}
                })
            }}
        </EnTetePage>

        <p class="msg-badges">
            {move || fil.get().filter(|f| f.est_prioritaire() && droits.get().messagerie_personnel).map(|_| view! { <Badge libelle="Prioritaire" ton=TonEtat::Avertissement/> })}
            {move || archive.get().then(|| view! { <Badge libelle="Archivée"/> })}
            {move || {
                fil.get()
                    .filter(|_| droits.get().messagerie_personnel)
                    .and_then(|f| f.dossier)
                    .map(|d| view! { <Badge libelle=format!("Dossier {d}") ton=TonEtat::Info/> })
            }}
        </p>

        {move || {
            erreur_action
                .get()
                .map(|m| view! {
                    <div id="msg-erreur-action" tabindex="-1">
                        <Alerte niveau=Niveau::Erreur titre=m on_fermer=Callback::new(move |_| erreur_action.set(None))/>
                    </div>
                })
        }}

        <section aria-labelledby="msg-liste-titre" class="msg-section">
            <h2 id="msg-liste-titre" class="ui-sr">"Messages de la conversation"</h2>
            <ol class="msg-fil">
                <For
                    each=move || messages.get()
                    key=|m| (m.id, m.nb_accuses, m.pieces.len())
                    children=move |m| view! { <LigneMessage message=m mon_id=mon_id telechargement=telechargement telecharger=telecharger/> }
                />
            </ol>
        </section>

        <section aria-labelledby="msg-reponse-titre" class="msg-section">
            <form class="msg-reponse carte" on:submit=envoyer novalidate>
                <h2 id="msg-reponse-titre" class="msg-reponse__titre">"Répondre"</h2>
                <ZoneTexte
                    id="msg-reponse"
                    libelle="Votre réponse"
                    valeur=corps
                    requis=true
                    erreur=erreur_corps
                    desactive=en_envoi
                />
                <div class="ui-champ">
                    <label class="ui-champ__libelle" for="msg-fichier">"Pièce jointe (facultative)"</label>
                    <input
                        id="msg-fichier"
                        class="ui-champ__controle"
                        type="file"
                        node_ref=champ_fichier
                        accept=MIME_PIECES.join(",")
                        aria-describedby="msg-fichier-aide"
                        disabled=move || en_envoi.get()
                        on:change=choisir_fichier
                    />
                    <p id="msg-fichier-aide" class="ui-champ__aide">
                        {format!("PDF, JPEG, PNG ou texte, {} au maximum.", taille_lisible(PIECE_MAX_OCTETS as i64))}
                    </p>
                    {move || erreur_fichier.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                    {move || {
                        fichier
                            .get()
                            .map(|f| view! {
                                <p class="msg-fichier-choisi">
                                    <span>{format!("{} ({})", f.nom, taille_lisible(f.octets.len() as i64))}</span>
                                    <Bouton variante=Variante::Discret taille=ent_web_ui::Taille::Compacte on_click=retirer_fichier>
                                        "Retirer"
                                    </Bouton>
                                </p>
                            })
                    }}
                </div>
                {move || {
                    erreur_envoi
                        .get()
                        .map(|m| view! {
                            <div id="msg-erreur-envoi" tabindex="-1">
                                <Alerte niveau=Niveau::Erreur titre=m>
                                    <p>"Votre texte est conservé. Vous pouvez réessayer."</p>
                                </Alerte>
                            </div>
                        })
                }}
                <div class="msg-reponse__actions">
                    <Bouton soumettre=true en_cours=en_envoi>"Envoyer la réponse"</Bouton>
                </div>
            </form>
        </section>

        <Confirmation
            ouvert=confirmer_archivage
            id="msg-archiver"
            titre="Archiver cette conversation ?"
            message="Elle quittera la boîte principale et restera consultable dans la vue « Archivées »."
            libelle_confirmer="Archiver"
            on_confirmer=archiver
        />

        <Dialogue ouvert=classement_ouvert titre="Classer la conversation" id="msg-classer">
            <form class="msg-classer" on:submit=classer novalidate>
                <Selection
                    id="msg-priorite"
                    libelle="Priorité"
                    valeur=priorite
                    options=vec![
                        ("normale".to_string(), "Normale".to_string()),
                        ("haute".to_string(), "Prioritaire".to_string()),
                    ]
                />
                <ChampTexte
                    id="msg-dossier-saisie"
                    libelle="Dossier"
                    valeur=dossier
                    aide="Facultatif. Lettres, chiffres, tirets (par exemple vs, scolarite)."
                    erreur=erreur_dossier
                />
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| classement_ouvert.set(false))>
                        "Annuler"
                    </Bouton>
                    <Bouton soumettre=true en_cours=en_action>"Enregistrer"</Bouton>
                </div>
            </form>
        </Dialogue>
    }
}

#[component]
fn LigneMessage(
    message: MessageFil,
    mon_id: Memo<Option<Uuid>>,
    telechargement: RwSignal<Option<Uuid>>,
    telecharger: Callback<(Uuid, String, String)>,
) -> impl IntoView {
    let mien = mon_id.get_untracked() == Some(message.auteur_user_id);
    let auteur = auteur_affiche(&message, mon_id.get_untracked());
    let date = en_local(&message.created_at)
        .map(|d| libelle_date_heure(d, aujourdhui()))
        .unwrap_or_default();
    let accuse = if mien {
        libelle_accuses(message.nb_accuses)
    } else {
        None
    };
    let pieces = message.pieces.clone();
    view! {
        <li class="msg" class:msg--mien=mien>
            <article class="msg__carte">
                <header class="msg__entete">
                    <Avatar nom=auteur.clone() accent=mien/>
                    <strong class="msg__auteur">{auteur}</strong>
                    <time class="msg__date" datetime=message.created_at.clone()>{date}</time>
                </header>
                <p class="msg__corps">{message.corps.clone()}</p>
                {(!pieces.is_empty()).then(|| view! {
                    <ul class="msg__pieces" aria-label="Pièces jointes">
                        {pieces
                            .into_iter()
                            .map(|p| {
                                let id = p.id;
                                let nom = p.nom.clone();
                                let mime = p.mime_type.clone();
                                let libelle = format!("Télécharger la pièce jointe {}", p.nom);
                                view! {
                                    <li>
                                        <Bouton
                                            variante=Variante::Secondaire
                                            taille=ent_web_ui::Taille::Compacte
                                            libelle_accessible=libelle
                                            en_cours=Signal::derive(move || telechargement.get() == Some(id))
                                            on_click=Callback::new(move |_| telecharger.run((id, nom.clone(), mime.clone())))
                                        >
                                            {format!("{} ({})", p.nom, taille_lisible(p.taille_octets))}
                                        </Bouton>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                })}
                {accuse.map(|a| view! { <p class="msg__accuse">{a}</p> })}
            </article>
        </li>
    }
}
