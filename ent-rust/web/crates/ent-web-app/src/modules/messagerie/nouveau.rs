//! Nouveau message : le personnel écrit à propos d'un élève (recherche d'élève, modèles) ;
//! une famille écrit à l'établissement à propos de son enfant ; un élève écrit à propos de
//! lui-même. Le message n'est envoyé qu'après relecture et confirmation.

use ent_web_api::{ApiError, EleveTrouve, EnfantFamille, ModeleMessage, Moi};
use ent_web_ui::{
    use_notifications, AccesRefuse, Alerte, Bouton, ChampTexte, Dialogue, EnTetePage, EtatVide,
    Niveau, Selection, Squelette, Taille, Variante, ZoneTexte,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_navigate;
use uuid::Uuid;

use super::fichiers::{lire_fichier_choisi, FichierChoisi, Lecture};
use super::modele::{
    libelle_eleve, message_apres_envoi, taille_lisible, valider_message, valider_sujet, IssuePiece,
    MIME_PIECES, PIECE_MAX_OCTETS,
};
use super::{focaliser, traiter_echec};
use crate::noyau::droits::{droits_du_role, Droits};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::non_lus::use_non_lus;
use crate::noyau::session;

/// Qui écrit, et donc comment l'élève concerné est désigné.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Personnel : recherche d'un élève, modèles.
    Personnel,
    /// Famille : choix parmi ses enfants.
    Famille,
    /// Élève : à propos de lui-même.
    Eleve(Uuid),
}

fn mode_de(moi: &Moi, droits: Droits) -> Option<Mode> {
    if droits.messagerie_personnel {
        Some(Mode::Personnel)
    } else if let Some(eleve) = moi.eleve_id.filter(|_| droits.messagerie) {
        Some(Mode::Eleve(eleve))
    } else if droits.messagerie {
        Some(Mode::Famille)
    } else {
        None
    }
}

#[component]
pub fn PageNouveauMessage() -> impl IntoView {
    let moi = session::use_session().moi;
    view! {
        <Title text="Nouveau message"/>
        <nav class="fil-ariane" aria-label="Fil d'Ariane">
            <a href=navigation::MESSAGES>"Messages"</a>
            <span aria-hidden="true">" › "</span>
            <span aria-current="page">"Nouveau message"</span>
        </nav>
        <EnTetePage titre="Nouveau message".to_string() compacte=true/>
        {move || match moi.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement du formulaire…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| moi.refetch())/> }.into_any(),
            Some(Ok(m)) => match mode_de(&m, droits_du_role(&m.role)) {
                Some(mode) => view! { <Formulaire mode=mode/> }.into_any(),
                None => view! { <AccesRefuse message="Votre profil n'a pas accès à la messagerie."/> }.into_any(),
            },
        }}
    }
}

#[component]
fn Formulaire(mode: Mode) -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let non_lus = use_non_lus();

    // Élève concerné : (identifiant, libellé).
    let eleve: RwSignal<Option<(Uuid, String)>> = RwSignal::new(match mode {
        Mode::Eleve(id) => Some((id, "vous".to_string())),
        _ => None,
    });
    let modeles: RwSignal<Vec<ModeleMessage>> = RwSignal::new(Vec::new());
    let modele_choisi = RwSignal::new(String::new());
    let sujet = RwSignal::new(String::new());
    let corps = RwSignal::new(String::new());
    let fichier: RwSignal<Option<FichierChoisi>> = RwSignal::new(None);

    let erreur_eleve: RwSignal<Option<String>> = RwSignal::new(None);
    let erreur_sujet: RwSignal<Option<String>> = RwSignal::new(None);
    let erreur_corps: RwSignal<Option<String>> = RwSignal::new(None);
    let erreur_fichier: RwSignal<Option<String>> = RwSignal::new(None);
    let erreur_envoi: RwSignal<Option<String>> = RwSignal::new(None);
    let en_envoi = RwSignal::new(false);
    let confirmation = RwSignal::new(false);
    let champ_fichier = NodeRef::<leptos::html::Input>::new();

    // Modèles (personnel) : le serveur remplace {prenom}, {nom}, {classe} à l'envoi.
    if mode == Mode::Personnel {
        spawn_local(async move {
            if let Ok(liste) = session::client().modeles_messages().await {
                modeles.set(liste.into_iter().filter(|m| m.actif).collect());
            }
        });
    }
    let modele_actif = Memo::new(move |_| {
        let id = modele_choisi.get();
        modeles.with(|l| l.iter().find(|m| m.id.to_string() == id).cloned())
    });
    Effect::new(move |precedent: Option<bool>| {
        let m = modele_actif.get();
        match &m {
            Some(m) => {
                sujet.set(m.sujet.clone());
                corps.set(m.corps.clone());
            }
            None if precedent == Some(true) => {
                sujet.set(String::new());
                corps.set(String::new());
            }
            None => {}
        }
        m.is_some()
    });
    let saisie_figee = Signal::derive(move || modele_actif.with(Option::is_some) || en_envoi.get());

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

    // Relecture : contrôle des champs puis demande de confirmation.
    let relire = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_envoi.get_untracked() {
            return;
        }
        let mut premier: Option<&'static str> = None;
        if eleve.with_untracked(Option::is_none) {
            erreur_eleve.set(Some(
                match mode {
                    Mode::Personnel => "Choisissez l'élève concerné.",
                    _ => "Choisissez l'enfant concerné.",
                }
                .to_string(),
            ));
            premier = Some(match mode {
                Mode::Personnel => "msg-eleve-q",
                _ => "msg-enfant",
            });
        } else {
            erreur_eleve.set(None);
        }
        // Avec un modèle, objet et texte viennent du serveur : rien à saisir.
        if modele_actif.with_untracked(Option::is_none) {
            match valider_sujet(&sujet.get_untracked()) {
                Ok(_) => erreur_sujet.set(None),
                Err(m) => {
                    erreur_sujet.set(Some(m));
                    premier = premier.or(Some("msg-sujet"));
                }
            }
            match valider_message(&corps.get_untracked()) {
                Ok(_) => erreur_corps.set(None),
                Err(m) => {
                    erreur_corps.set(Some(m));
                    premier = premier.or(Some("msg-corps"));
                }
            }
        }
        match premier {
            Some(id) => focaliser(id),
            None => confirmation.set(true),
        }
    };

    // Envoi après confirmation : le fil, puis la pièce jointe sur son premier message.
    let envoyer = {
        let navigate = navigate.clone();
        Callback::new(move |_: ()| {
            if en_envoi.get_untracked() {
                return;
            }
            let Some((eleve_id, _)) = eleve.get_untracked() else {
                return;
            };
            let modele = modele_actif.get_untracked();
            let (sujet_ok, corps_ok) = match (
                &modele,
                valider_sujet(&sujet.get_untracked()),
                valider_message(&corps.get_untracked()),
            ) {
                (None, Ok(s), Ok(c)) => (s, c),
                (Some(_), _, _) => (String::new(), String::new()),
                _ => return,
            };
            confirmation.set(false);
            en_envoi.set(true);
            erreur_envoi.set(None);
            let piece = fichier.get_untracked();
            let navigate = navigate.clone();
            spawn_local(async move {
                let client = session::client();
                let cree = match &modele {
                    Some(m) => client.ouvrir_fil_depuis_modele(m.id, eleve_id).await,
                    None => {
                        client
                            .ouvrir_fil_messages(eleve_id, &sujet_ok, &corps_ok)
                            .await
                    }
                };
                let detail = match cree {
                    Ok(d) => d,
                    Err(e) => {
                        // Rien n'est parti : les saisies restent. 401/403/429 : on s'arrête, sans rien enchaîner.
                        erreur_envoi.set(Some(traiter_echec(&e)));
                        en_envoi.set(false);
                        focaliser("msg-erreur-envoi");
                        return;
                    }
                };
                let issue = match (piece, detail.messages.first()) {
                    (None, _) => IssuePiece::SansPiece,
                    (Some(p), Some(premier)) => match client
                        .joindre_piece(premier.id, &p.nom, &p.mime, &p.octets)
                        .await
                    {
                        Ok(_) => IssuePiece::Jointe,
                        Err(e) => IssuePiece::Echec(traiter_echec(&e)),
                    },
                    (Some(_), None) => IssuePiece::Echec("message introuvable".into()),
                };
                match &issue {
                    IssuePiece::Echec(_) => notifications.info(message_apres_envoi(&issue)),
                    _ => notifications.succes(message_apres_envoi(&issue)),
                };
                if let Some(n) = non_lus {
                    n.rafraichir.run(());
                }
                en_envoi.set(false);
                navigate(
                    &format!("{}/{}", navigation::MESSAGES, detail.fil.id),
                    Default::default(),
                );
            });
        })
    };

    let destinataires = match mode {
        Mode::Personnel => "La famille de l'élève et le personnel autorisé de la messagerie.",
        Mode::Famille | Mode::Eleve(_) => "L'équipe de l'établissement autorisée à la messagerie.",
    };

    view! {
        {match mode {
            Mode::Personnel => view! { <ChoixEleveRecherche eleve=eleve erreur=erreur_eleve/> }.into_any(),
            Mode::Famille => view! { <ChoixEnfant eleve=eleve erreur=erreur_eleve/> }.into_any(),
            Mode::Eleve(_) => view! {
                <p class="msg-concerne">"Ce message concerne : vous."</p>
            }
                .into_any(),
        }}

        <form class="msg-nouveau carte" on:submit=relire novalidate>
            {move || {
                (mode == Mode::Personnel && !modeles.with(Vec::is_empty))
                    .then(|| {
                        let mut options = vec![(String::new(), "Message libre".to_string())];
                        options.extend(modeles.get().into_iter().map(|m| (m.id.to_string(), m.titre)));
                        view! {
                            <Selection
                                id="msg-modele"
                                libelle="Modèle de message"
                                valeur=modele_choisi
                                options=options
                                aide="Les mentions {prenom}, {nom} et {classe} du modèle sont remplacées à l'envoi."
                                desactive=en_envoi
                            />
                        }
                    })
            }}
            <ChampTexte
                id="msg-sujet"
                libelle="Objet"
                valeur=sujet
                requis=true
                erreur=erreur_sujet
                desactive=saisie_figee
                autocomplete="off"
            />
            <ZoneTexte
                id="msg-corps"
                libelle="Message"
                valeur=corps
                requis=true
                erreur=erreur_corps
                desactive=saisie_figee
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
                                <Bouton variante=Variante::Discret taille=Taille::Compacte on_click=retirer_fichier>
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
                                <p>"Rien n'a été envoyé. Vos saisies sont conservées : vous pouvez réessayer."</p>
                            </Alerte>
                        </div>
                    })
            }}
            <div class="msg-nouveau__actions">
                <Bouton soumettre=true en_cours=en_envoi>"Relire et envoyer"</Bouton>
            </div>
        </form>

        <Dialogue ouvert=confirmation titre="Envoyer ce message ?" id="msg-confirmation">
            <dl class="msg-recap">
                <dt>"À propos de"</dt>
                <dd>{move || eleve.get().map(|(_, l)| l).unwrap_or_default()}</dd>
                <dt>"Visible par"</dt>
                <dd>{destinataires}</dd>
                <dt>"Objet"</dt>
                <dd>{move || sujet.get()}</dd>
                <dt>"Message"</dt>
                <dd class="msg-recap__corps">{move || corps.get()}</dd>
                {move || fichier.get().map(|f| view! {
                    <dt>"Pièce jointe"</dt>
                    <dd>{f.nom}</dd>
                })}
            </dl>
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| confirmation.set(false))>
                    "Modifier"
                </Bouton>
                <Bouton on_click=envoyer>"Envoyer le message"</Bouton>
            </div>
        </Dialogue>
    }
}

/// Famille : choix de l'enfant concerné parmi `/famille/enfants`.
#[component]
fn ChoixEnfant(
    eleve: RwSignal<Option<(Uuid, String)>>,
    erreur: RwSignal<Option<String>>,
) -> impl IntoView {
    let enfants = LocalResource::new(|| async { session::client().enfants_famille().await });
    let choix = RwSignal::new(String::new());
    Effect::new(move |_| {
        let id = choix.get();
        let trouve = enfants
            .get()
            .and_then(Result::ok)
            .and_then(|l| l.into_iter().find(|e| e.id.to_string() == id));
        eleve.set(trouve.map(|e| (e.id, libelle_enfant(&e))));
        if eleve.with_untracked(Option::is_some) {
            erreur.set(None);
        }
    });
    view! {
        <section class="msg-eleve carte" aria-labelledby="msg-enfant-titre">
            <h2 id="msg-enfant-titre" class="msg-eleve__titre">"Enfant concerné"</h2>
            {move || match enfants.get() {
                None => view! { <Squelette lignes=1 libelle="Chargement de vos enfants…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| enfants.refetch())/> }.into_any(),
                Some(Ok(liste)) if liste.is_empty() => view! {
                    <EtatVide titre="Aucun enfant rattaché" message="Votre compte n'est lié à aucun élève : contactez l'établissement."/>
                }
                    .into_any(),
                Some(Ok(liste)) if liste.len() == 1 => {
                    let seul = liste.into_iter().next().expect("un enfant");
                    let libelle = libelle_enfant(&seul);
                    choix.set(seul.id.to_string());
                    view! { <p class="msg-concerne">{format!("Ce message concerne : {libelle}.")}</p> }.into_any()
                }
                Some(Ok(liste)) => {
                    let mut options = vec![(String::new(), "Choisir un enfant".to_string())];
                    options.extend(liste.iter().map(|e| (e.id.to_string(), libelle_enfant(e))));
                    view! {
                        <Selection id="msg-enfant" libelle="Enfant" valeur=choix options=options requis=true erreur=erreur/>
                    }
                        .into_any()
                }
            }}
        </section>
    }
}

fn libelle_enfant(e: &EnfantFamille) -> String {
    libelle_eleve(&e.nom, &e.prenom, e.classe.as_deref())
}

/// Personnel : recherche d'un élève (`/eleves/search`), 50 résultats au plus côté serveur.
#[component]
fn ChoixEleveRecherche(
    eleve: RwSignal<Option<(Uuid, String)>>,
    erreur: RwSignal<Option<String>>,
) -> impl IntoView {
    let requete = RwSignal::new(String::new());
    let erreur_recherche: RwSignal<Option<String>> = RwSignal::new(None);
    let resultats: RwSignal<Option<Vec<EleveTrouve>>> = RwSignal::new(None);
    let en_cours = RwSignal::new(false);

    let chercher = Callback::new(move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        let texte = requete.get_untracked();
        if texte.trim().chars().count() < 2 {
            erreur_recherche.set(Some("Saisissez au moins 2 caractères.".into()));
            focaliser("msg-eleve-q");
            return;
        }
        erreur_recherche.set(None);
        en_cours.set(true);
        spawn_local(async move {
            match session::client().messagerie_chercher_eleves(&texte).await {
                Ok(liste) => resultats.set(Some(liste)),
                Err(e) => {
                    resultats.set(None);
                    erreur_recherche.set(Some(match e {
                        ApiError::Interdit(_) => {
                            "La recherche d'élèves n'est pas autorisée pour votre profil.".into()
                        }
                        e => traiter_echec(&e),
                    }));
                }
            }
            en_cours.set(false);
        });
    });
    let choisir = Callback::new(move |e: EleveTrouve| {
        let libelle = libelle_eleve(&e.nom, &e.prenom, e.classe.as_deref());
        eleve.set(Some((e.id, libelle)));
        erreur.set(None);
        resultats.set(None);
        focaliser("msg-sujet");
    });
    let changer = Callback::new(move |_: ()| {
        eleve.set(None);
        focaliser("msg-eleve-q");
    });

    view! {
        <section class="msg-eleve carte" aria-labelledby="msg-eleve-titre">
            <h2 id="msg-eleve-titre" class="msg-eleve__titre">"Élève concerné"</h2>
            {move || match eleve.get() {
                Some((_, libelle)) => view! {
                    <p class="msg-concerne">
                        <span>"Ce message concerne : "<strong>{libelle}</strong></span>
                        <Bouton variante=Variante::Discret taille=Taille::Compacte on_click=changer>"Changer d'élève"</Bouton>
                    </p>
                }
                    .into_any(),
                None => view! {
                    <form class="msg-eleve__recherche" role="search" on:submit=move |ev| chercher.run(ev) novalidate>
                        <ChampTexte
                            id="msg-eleve-q"
                            libelle="Rechercher un élève (nom, prénom ou classe)"
                            valeur=requete
                            genre="search"
                            erreur=Signal::derive(move || erreur_recherche.get().or_else(|| erreur.get()))
                            autocomplete="off"
                        />
                        <Bouton soumettre=true en_cours=en_cours>"Rechercher"</Bouton>
                    </form>
                    {move || resultats.get().map(|liste| {
                        if liste.is_empty() {
                            view! { <p class="texte-attenue" role="status">"Aucun élève ne correspond à cette recherche."</p> }.into_any()
                        } else {
                            view! {
                                <p class="ui-sr" role="status">{format!("{} élève(s) trouvé(s)", liste.len())}</p>
                                <ul class="msg-eleves" aria-label="Élèves trouvés">
                                    {liste
                                        .into_iter()
                                        .map(|e| {
                                            let libelle = libelle_eleve(&e.nom, &e.prenom, e.classe.as_deref());
                                            view! {
                                                <li>
                                                    <Bouton
                                                        variante=Variante::Secondaire
                                                        libelle_accessible=format!("Choisir {libelle}")
                                                        on_click=Callback::new(move |_| choisir.run(e.clone()))
                                                    >
                                                        {libelle}
                                                    </Bouton>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }
                    })}
                }
                    .into_any(),
            }}
        </section>
    }
}
