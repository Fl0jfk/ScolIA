//! Page « Sécurité du compte » : niveau de la session, passkeys, appareils connectés.

use ent_web_api::{ApiError, AppareilSession, PasskeyCompte};
use ent_web_ui::{
    Alerte, Annonce, Badge, Bouton, ChampTexte, Dialogue, EnTetePage, EtatVide, LienBouton, Niveau,
    Squelette, Taille, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::{use_location, use_navigate};

use super::modele::{
    assurance_faible, avertissement_suppression, explication_assurance, horodatage_local,
    libelle_assurance, libelle_client, libelle_obligation, message_action_sensible,
    mot_de_passe_requis, nom_appareil, nom_passkey, trier_appareils, valider_mot_de_passe,
};
use super::{enroler_passkey, EchecEnrolement};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::portee::Portee;
use crate::noyau::session::{self, use_session};

#[component]
pub fn PageSecurite() -> impl IntoView {
    let ctx = use_session();
    let droits = use_droits();
    let annonce = RwSignal::new(String::new());
    let sous_titre =
        Signal::derive(move || ctx.securite.get().map(|m| m.email).unwrap_or_default());
    view! {
        <Title text="Sécurité du compte"/>
        <EnTetePage titre="Sécurité du compte".to_string() sous_titre=sous_titre/>
        <div class="securite">
            <SectionSession/>
            <SectionPasskeys annonce=annonce/>
            <SectionAppareils annonce=annonce/>
            {move || {
                droits
                    .get()
                    .gerer_securite
                    .then(|| {
                        view! {
                            <section class="securite__bloc" aria-labelledby="securite-etab-titre">
                                <h2 id="securite-etab-titre" class="securite__titre">"Établissement"</h2>
                                <p>"Vous pouvez choisir les rôles pour lesquels la passkey est obligatoire."</p>
                                <p>
                                    <LienBouton href=navigation::POLITIQUE_SECURITE variante=Variante::Secondaire>
                                        "Politique de sécurité de l'établissement"
                                    </LienBouton>
                                </p>
                            </section>
                        }
                    })
            }}
            <Annonce message=Signal::derive(move || annonce.get())/>
        </div>
    }
}

/// Niveau de la session courante.
#[component]
fn SectionSession() -> impl IntoView {
    let ctx = use_session();
    view! {
        <section class="securite__bloc" aria-labelledby="securite-session-titre">
            <h2 id="securite-session-titre" class="securite__titre">"Niveau de cette session"</h2>
            {move || match ctx.securite.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement du niveau de la session…"/> }.into_any(),
                Some(m) => {
                    let ton = if assurance_faible(&m.assurance) { TonEtat::Avertissement } else { TonEtat::Ok };
                    view! {
                        <p class="securite__badges">
                            <Badge libelle=format!("Ouverte avec : {}", libelle_assurance(&m.assurance)) ton=ton/>
                            <Badge libelle=libelle_client(&m.client).to_string() ton=TonEtat::Neutre/>
                        </p>
                        <p>{explication_assurance(&m.assurance)}</p>
                        <p>{libelle_obligation(m.passkey_obligatoire, m.passkey_count)}</p>
                    }
                        .into_any()
                }
            }}
        </section>
    }
}

/// Passkeys du compte : liste, ajout, suppression.
#[component]
fn SectionPasskeys(annonce: RwSignal<String>) -> impl IntoView {
    let ctx = use_session();
    let notifications = ent_web_ui::use_notifications();
    let navigate = use_navigate();
    let version = RwSignal::new(0u32);
    let liste = LocalResource::new(move || {
        version.track();
        async { session::client().passkeys().await }
    });

    // --- Ajout
    let nom = RwSignal::new(String::new());
    let mdp = RwSignal::new(String::new());
    let erreur_mdp = RwSignal::new(None::<String>);
    let erreur_ajout = RwSignal::new(None::<String>);
    let en_ajout = RwSignal::new(false);
    let portee = Signal::derive(move || {
        ctx.securite
            .get()
            .map(|m| Portee::depuis(&m.portee))
            .unwrap_or(Portee::Complete)
    });
    let ajouter = {
        let navigate = navigate.clone();
        Callback::new(move |_: ()| {
            if en_ajout.get_untracked() {
                return;
            }
            erreur_ajout.set(None);
            match valider_mot_de_passe(
                &mdp.get_untracked(),
                mot_de_passe_requis(portee.get_untracked()),
            ) {
                Ok(()) => erreur_mdp.set(None),
                Err(m) => {
                    erreur_mdp.set(Some(m.into()));
                    return;
                }
            }
            en_ajout.set(true);
            annonce.set("Validation sur votre appareil en cours…".into());
            let navigate = navigate.clone();
            spawn_local(async move {
                let resultat = enroler_passkey(&nom.get_untracked(), &mdp.get_untracked()).await;
                en_ajout.set(false);
                match resultat {
                    Ok(_) => {
                        nom.set(String::new());
                        mdp.set(String::new());
                        notifications.succes("Passkey ajoutée.");
                        annonce.set("Passkey ajoutée.".into());
                        version.update(|v| *v += 1);
                        ctx.moi.refetch();
                    }
                    Err(e) => {
                        if e.session_perdue() {
                            session::effacer();
                            navigate(
                                &session::adresse_connexion("/web/securite"),
                                Default::default(),
                            );
                            return;
                        }
                        let message = e.message();
                        // Mot de passe refusé : message sous le champ concerné.
                        if matches!(&e, EchecEnrolement::Serveur(s) if message_action_sensible(s) == "Mot de passe actuel incorrect.")
                        {
                            erreur_mdp.set(Some(message.clone()));
                        } else {
                            erreur_ajout.set(Some(message.clone()));
                        }
                        annonce.set(message);
                    }
                }
            });
        })
    };

    // --- Suppression
    let a_supprimer = RwSignal::new(None::<PasskeyCompte>);
    let dialogue_ouvert = RwSignal::new(false);
    let mdp_suppression = RwSignal::new(String::new());
    let erreur_suppression = RwSignal::new(None::<String>);
    let en_suppression = RwSignal::new(false);
    let nb_passkeys = Signal::derive(move || {
        liste
            .get()
            .and_then(Result::ok)
            .map(|l| l.len() as i64)
            .unwrap_or(0)
    });
    let obligatoire =
        Signal::derive(move || ctx.securite.get().is_some_and(|m| m.passkey_obligatoire));
    let demander_suppression = Callback::new(move |p: PasskeyCompte| {
        mdp_suppression.set(String::new());
        erreur_suppression.set(None);
        a_supprimer.set(Some(p));
        dialogue_ouvert.set(true);
    });
    let confirmer_suppression = {
        let navigate = navigate.clone();
        Callback::new(move |_: ()| {
            if en_suppression.get_untracked() {
                return;
            }
            let Some(p) = a_supprimer.get_untracked() else {
                return;
            };
            if let Err(m) = valider_mot_de_passe(&mdp_suppression.get_untracked(), true) {
                erreur_suppression.set(Some(m.into()));
                return;
            }
            erreur_suppression.set(None);
            en_suppression.set(true);
            let navigate = navigate.clone();
            spawn_local(async move {
                let resultat = session::client()
                    .supprimer_passkey(p.id, &mdp_suppression.get_untracked())
                    .await;
                en_suppression.set(false);
                match resultat {
                    Ok(()) => {
                        mdp_suppression.set(String::new());
                        dialogue_ouvert.set(false);
                        notifications.succes("Passkey supprimée.");
                        annonce.set("Passkey supprimée.".into());
                        version.update(|v| *v += 1);
                        // Si la dernière passkey d'un rôle soumis à l'obligation a été retirée,
                        // `/auth/me` passe en « enrôlement » et la coquille redirige.
                        ctx.moi.refetch();
                    }
                    Err(ApiError::NonConnecte) => {
                        session::effacer();
                        navigate(
                            &session::adresse_connexion("/web/securite"),
                            Default::default(),
                        );
                    }
                    Err(ApiError::Introuvable(_)) => {
                        dialogue_ouvert.set(false);
                        annonce.set("Cette passkey n'existe plus.".into());
                        version.update(|v| *v += 1);
                    }
                    Err(e) => erreur_suppression.set(Some(message_action_sensible(&e))),
                }
            });
        })
    };

    view! {
        <section class="securite__bloc" aria-labelledby="securite-passkeys-titre">
            <h2 id="securite-passkeys-titre" class="securite__titre">"Mes passkeys"</h2>
            <p class="securite__aide">
                "Une passkey remplace le mot de passe : vous validez avec l'empreinte, le visage ou le code de votre appareil."
            </p>
            {move || match liste.get() {
                None => view! { <Squelette lignes=2 libelle="Chargement de vos passkeys…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => {
                    view! {
                        <EtatVide
                            titre="Aucune passkey enregistrée"
                            message="Ajoutez-en une ci-dessous pour renforcer la protection de votre compte."
                        />
                    }
                        .into_any()
                }
                Some(Ok(l)) => {
                    view! {
                        <ul class="securite__liste">
                            <For
                                each=move || l.clone()
                                key=|p| (p.id, p.last_used_at)
                                children=move |p| {
                                    let nom_affiche = nom_passkey(&p);
                                    let etiquette = format!("Supprimer la passkey {nom_affiche}");
                                    let cree = horodatage_local(p.created_at);
                                    let utilisee = p
                                        .last_used_at
                                        .map(|d| format!("Dernière utilisation : {}", horodatage_local(d)))
                                        .unwrap_or_else(|| "Jamais utilisée".to_string());
                                    let p2 = p.clone();
                                    view! {
                                        <li class="securite__element">
                                            <div class="securite__texte">
                                                <strong>{nom_affiche}</strong>
                                                <span>{format!("Ajoutée le {cree}")}</span>
                                                <span>{utilisee}</span>
                                            </div>
                                            <Bouton
                                                variante=Variante::Secondaire
                                                taille=Taille::Compacte
                                                libelle_accessible=etiquette
                                                on_click=Callback::new(move |_| demander_suppression.run(p2.clone()))
                                            >
                                                "Supprimer"
                                            </Bouton>
                                        </li>
                                    }
                                }
                            />
                        </ul>
                    }
                        .into_any()
                }
            }}

            <form
                class="securite__formulaire"
                on:submit=move |ev| {
                    ev.prevent_default();
                    ajouter.run(());
                }
                novalidate
            >
                <h3 class="securite__sous-titre">"Ajouter une passkey"</h3>
                <ChampTexte
                    id="ajout-nom-passkey"
                    libelle="Nom de l'appareil"
                    valeur=nom
                    aide="Facultatif. Par exemple : « Téléphone de Marie »."
                    autocomplete="off"
                />
                <ChampTexte
                    id="ajout-mdp"
                    libelle="Mot de passe actuel"
                    valeur=mdp
                    genre="password"
                    requis=true
                    autocomplete="current-password"
                    aide="Demandé pour confirmer que c'est bien vous."
                    erreur=Signal::derive(move || erreur_mdp.get())
                />
                {move || erreur_ajout.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <div>
                    <Bouton soumettre=true variante=Variante::Primaire en_cours=en_ajout>
                        "Ajouter une passkey"
                    </Bouton>
                </div>
            </form>

            <Dialogue ouvert=dialogue_ouvert titre="Supprimer cette passkey ?" id="dialogue-suppression-passkey">
                <p>
                    {move || {
                        a_supprimer
                            .get()
                            .map(|p| format!("La passkey « {} » ne pourra plus servir à vous connecter.", nom_passkey(&p)))
                    }}
                </p>
                {move || {
                    avertissement_suppression(nb_passkeys.get(), obligatoire.get())
                        .map(|m| view! { <Alerte niveau=Niveau::Avertissement titre=m/> })
                }}
                <form
                    class="securite__formulaire"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        confirmer_suppression.run(());
                    }
                    novalidate
                >
                    <ChampTexte
                        id="suppression-mdp"
                        libelle="Mot de passe actuel"
                        valeur=mdp_suppression
                        genre="password"
                        requis=true
                        autocomplete="current-password"
                        erreur=Signal::derive(move || erreur_suppression.get())
                    />
                    <div class="ui-dialogue__actions">
                        <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| dialogue_ouvert.set(false))>
                            "Annuler"
                        </Bouton>
                        <Bouton soumettre=true variante=Variante::Danger en_cours=en_suppression>
                            "Supprimer la passkey"
                        </Bouton>
                    </div>
                </form>
            </Dialogue>
        </section>
    }
}

/// Appareils connectés : liste et déconnexion des autres appareils.
#[component]
fn SectionAppareils(annonce: RwSignal<String>) -> impl IntoView {
    let notifications = ent_web_ui::use_notifications();
    let navigate = use_navigate();
    let location = use_location();
    let version = RwSignal::new(0u32);
    let liste = LocalResource::new(move || {
        version.track();
        async { session::client().appareils().await.map(trier_appareils) }
    });
    let cible = RwSignal::new(None::<AppareilSession>);
    let dialogue_ouvert = RwSignal::new(false);
    let en_cours = RwSignal::new(false);
    let erreur = RwSignal::new(None::<String>);

    let confirmer = Callback::new(move |_: ()| {
        if en_cours.get_untracked() {
            return;
        }
        let Some(a) = cible.get_untracked() else {
            return;
        };
        en_cours.set(true);
        erreur.set(None);
        let navigate = navigate.clone();
        spawn_local(async move {
            let resultat = session::client().deconnecter_appareil(a.id).await;
            en_cours.set(false);
            match resultat {
                Ok(()) | Err(ApiError::Introuvable(_)) => {
                    dialogue_ouvert.set(false);
                    notifications.succes("Appareil déconnecté.");
                    annonce.set("Appareil déconnecté.".into());
                    version.update(|v| *v += 1);
                }
                Err(ApiError::NonConnecte) => {
                    session::effacer();
                    let chemin = session::chemin_complet(
                        &location.pathname.get_untracked(),
                        &location.search.get_untracked(),
                    );
                    navigate(&session::adresse_connexion(&chemin), Default::default());
                }
                Err(e) => erreur.set(Some(e.message_utilisateur())),
            }
        });
    });

    view! {
        <section class="securite__bloc" aria-labelledby="securite-appareils-titre">
            <h2 id="securite-appareils-titre" class="securite__titre">"Mes appareils"</h2>
            <p class="securite__aide">
                "Les appareils et navigateurs actuellement connectés à votre compte dans cet établissement. Déconnectez ceux que vous ne reconnaissez pas."
            </p>
            {move || match liste.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement de vos appareils…"/> }.into_any(),
                Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/> }.into_any(),
                Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucun appareil connecté"/> }.into_any(),
                Some(Ok(l)) => {
                    view! {
                        <ul class="securite__liste">
                            <For
                                each=move || l.clone()
                                key=|a| (a.id, a.last_seen_at)
                                children=move |a| {
                                    let nom_affiche = nom_appareil(&a);
                                    let etiquette = format!("Déconnecter l'appareil {nom_affiche}");
                                    let courante = a.courante;
                                    let a2 = a.clone();
                                    view! {
                                        <li class="securite__element">
                                            <div class="securite__texte">
                                                <strong>
                                                    {nom_affiche}
                                                    {courante.then(|| view! { <Badge libelle="Cet appareil" ton=TonEtat::Ok/> })}
                                                </strong>
                                                <span>
                                                    {format!("{} · ouverte avec : {}", libelle_client(&a.client), libelle_assurance(&a.assurance))}
                                                </span>
                                                <span>{format!("Dernière activité : {}", horodatage_local(a.last_seen_at))}</span>
                                                <span>{format!("Session valable jusqu'au {}", horodatage_local(a.expires_at))}</span>
                                            </div>
                                            {(!courante)
                                                .then(|| {
                                                    view! {
                                                        <Bouton
                                                            variante=Variante::Secondaire
                                                            taille=Taille::Compacte
                                                            libelle_accessible=etiquette
                                                            on_click=Callback::new(move |_| {
                                                                erreur.set(None);
                                                                cible.set(Some(a2.clone()));
                                                                dialogue_ouvert.set(true);
                                                            })
                                                        >
                                                            "Déconnecter"
                                                        </Bouton>
                                                    }
                                                })}
                                        </li>
                                    }
                                }
                            />
                        </ul>
                    }
                        .into_any()
                }
            }}
            <p class="securite__aide">
                "Pour fermer cet appareil, utilisez « Se déconnecter » dans le menu."
            </p>
            <Dialogue ouvert=dialogue_ouvert titre="Déconnecter cet appareil ?" id="dialogue-deconnexion-appareil">
                <p>
                    {move || {
                        cible
                            .get()
                            .map(|a| format!("« {} » devra se reconnecter pour accéder à votre compte.", nom_appareil(&a)))
                    }}
                </p>
                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                <div class="ui-dialogue__actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| dialogue_ouvert.set(false))>
                        "Annuler"
                    </Bouton>
                    <Bouton variante=Variante::Danger en_cours=en_cours on_click=confirmer>
                        "Déconnecter l'appareil"
                    </Bouton>
                </div>
            </Dialogue>
        </section>
    }
}
