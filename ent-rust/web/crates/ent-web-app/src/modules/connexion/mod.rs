//! Connexion : passkey (sans mot de passe), ou mot de passe puis passkey si le compte
//! en a une (second facteur). Routes d'authentification existantes, **inchangées**.

pub mod webauthn;

use ent_web_api::{ApiError, Connexion, PasskeyChallengeDto, Session};
use ent_web_ui::{Alerte, Bouton, ChampTexte, Icone, Niveau, NomIcone, Variante};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_query_map};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::noyau::session;

/// Configuration publique de la page de connexion (aucun secret).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametresConnexion {
    /// Établissement proposé par défaut (labo local). À terme : déduit du sous-domaine (D-012).
    pub etablissement_id: Option<Uuid>,
    /// Vrai hors production : affiche le bandeau « environnement de test ».
    pub labo: bool,
}

/// Fonction serveur : exécutée sur le serveur, appelée en HTTP par le navigateur
/// (et directement pendant le rendu serveur).
#[server]
pub async fn config_connexion() -> Result<ParametresConnexion, ServerFnError> {
    let etablissement_id = std::env::var("ENT_WEB_ETABLISSEMENT_ID")
        .ok()
        .and_then(|v| v.trim().parse().ok());
    let env = std::env::var("ENT_ENV").unwrap_or_default();
    Ok(ParametresConnexion {
        etablissement_id,
        labo: env_non_productif(&env),
    })
}

/// Valeurs de `ENT_ENV` désignant un environnement explicitement non productif
/// (même règle fail-closed que `gate_open` côté `ent-api`).
#[cfg(any(feature = "ssr", test))]
pub const ENVS_NON_PROD: [&str; 5] = ["lab", "labo", "dev", "local", "test"];

/// Vrai seulement si `ENT_ENV` (trim, insensible à la casse) est un environnement
/// non productif explicite ; absent, vide, `prod` ou inconnu → faux.
#[cfg(any(feature = "ssr", test))]
fn env_non_productif(env: &str) -> bool {
    ENVS_NON_PROD.contains(&env.trim().to_ascii_lowercase().as_str())
}

#[cfg(test)]
mod tests_env {
    use super::env_non_productif;

    #[test]
    fn bandeau_labo_fail_closed() {
        assert!(env_non_productif(" LAB "));
        assert!(env_non_productif("dev"));
        assert!(!env_non_productif(""));
        assert!(!env_non_productif("prod"));
        assert!(!env_non_productif("production"));
        assert!(!env_non_productif("inconnu"));
    }
}

/// Message d'erreur de connexion, sans révéler si le compte existe.
fn message_erreur(e: &ApiError) -> String {
    match e {
        ApiError::NonConnecte | ApiError::Interdit(_) => {
            "Adresse e-mail ou mot de passe incorrect.".into()
        }
        ApiError::TropDeRequetes => {
            "Trop de tentatives. Patientez quelques minutes avant de réessayer.".into()
        }
        autre => autre.message_utilisateur(),
    }
}

#[component]
pub fn PageConnexion() -> impl IntoView {
    let navigate = use_navigate();
    let query = use_query_map();
    let config = Resource::new(|| (), |_| config_connexion());

    let email = RwSignal::new(String::new());
    let mot_de_passe = RwSignal::new(String::new());
    let etablissement = RwSignal::new(String::new());
    let modifier_etab = RwSignal::new(false);
    let en_cours = RwSignal::new(false);
    let erreur = RwSignal::new(None::<String>);
    let erreur_email = RwSignal::new(None::<String>);
    // Défi du second facteur, conservé pour « Réessayer ».
    let second_facteur = RwSignal::new(None::<PasskeyChallengeDto>);

    // Préremplit l'établissement depuis l'URL (?etab=) ou la configuration serveur.
    Effect::new(move |_| {
        if !etablissement.get_untracked().is_empty() {
            return;
        }
        if let Some(e) = query.with(|q| q.get("etab")) {
            etablissement.set(e);
        } else if let Some(Ok(c)) = config.get() {
            if let Some(id) = c.etablissement_id {
                etablissement.set(id.to_string());
            }
        }
    });

    let info_fin = move || {
        query.with(|q| match q.get("fin").as_deref() {
            Some("deconnexion") => Some("Vous êtes déconnecté."),
            Some("autre-onglet") => Some("Vous avez été déconnecté depuis un autre onglet."),
            Some("expiree-saisie") => Some(
                "Votre session a expiré pendant la saisie : les modifications non enregistrées ont été perdues. Reconnectez-vous puis refaites-les.",
            ),
            _ => None,
        })
    };

    let ouvrir_session = {
        let navigate = navigate.clone();
        move |s: Session| {
            // Stockage bloqué : sans jeton conservé, la page /web renverrait ici en boucle.
            if session::enregistrer(&s.token).is_err() {
                mot_de_passe.set(String::new());
                erreur.set(Some(session::StockageIndisponible::MESSAGE.into()));
                return;
            }
            mot_de_passe.set(String::new());
            let retour = session::retour_sur(query.with_untracked(|q| q.get("retour")));
            navigate(&retour, Default::default());
        }
    };

    let etab_valide = move || -> Option<Uuid> {
        match etablissement.get_untracked().trim().parse::<Uuid>() {
            Ok(id) => Some(id),
            Err(_) => {
                modifier_etab.set(true);
                erreur.set(Some(
                    "Indiquez l'identifiant de votre établissement.".into(),
                ));
                None
            }
        }
    };

    // Signature du défi par l'appareil, puis ouverture de la session.
    let finir_passkey = {
        let ouvrir_session = ouvrir_session.clone();
        move |defi: PasskeyChallengeDto| {
            let ouvrir_session = ouvrir_session.clone();
            en_cours.set(true);
            erreur.set(None);
            spawn_local(async move {
                let resultat = match webauthn::signer_defi(&defi.options).await {
                    Ok(assertion) => session::client()
                        .connexion_passkey_fin(&defi.token, assertion)
                        .await
                        .map_err(|e| match e {
                            ApiError::NonConnecte | ApiError::Interdit(_) => {
                                "La passkey n'a pas été reconnue.".to_string()
                            }
                            autre => autre.message_utilisateur(),
                        }),
                    Err(m) => Err(format!("{}.", capitaliser(&m))),
                };
                en_cours.set(false);
                match resultat {
                    Ok(s) => {
                        second_facteur.set(None);
                        ouvrir_session(s);
                    }
                    Err(m) => erreur.set(Some(m)),
                }
            });
        }
    };

    let connexion_passkey = {
        let finir_passkey = finir_passkey.clone();
        move |_| {
            if en_cours.get_untracked() {
                return;
            }
            let adresse = email.get_untracked().trim().to_string();
            if adresse.is_empty() {
                erreur_email.set(Some(
                    "Saisissez votre adresse e-mail pour utiliser votre passkey.".into(),
                ));
                return;
            }
            erreur_email.set(None);
            let Some(etab) = etab_valide() else { return };
            let finir_passkey = finir_passkey.clone();
            en_cours.set(true);
            erreur.set(None);
            spawn_local(async move {
                match session::client()
                    .connexion_passkey_debut(&adresse, etab)
                    .await
                {
                    Ok(defi) => finir_passkey(defi),
                    Err(e) => {
                        en_cours.set(false);
                        erreur.set(Some(match e {
                            ApiError::NonConnecte => {
                                "Aucune passkey n'est enregistrée pour ce compte. Utilisez votre mot de passe."
                                    .into()
                            }
                            autre => message_erreur(&autre),
                        }));
                    }
                }
            });
        }
    };

    let soumettre = {
        let ouvrir_session = ouvrir_session.clone();
        let finir_passkey = finir_passkey.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            if en_cours.get_untracked() {
                return;
            }
            let Some(etab) = etab_valide() else { return };
            erreur.set(None);
            erreur_email.set(None);
            en_cours.set(true);
            let ouvrir_session = ouvrir_session.clone();
            let finir_passkey = finir_passkey.clone();
            spawn_local(async move {
                let resultat = session::client()
                    .connexion(
                        email.get_untracked().trim(),
                        &mot_de_passe.get_untracked(),
                        etab,
                    )
                    .await;
                en_cours.set(false);
                match resultat {
                    Ok(Connexion::Ouverte(s)) => ouvrir_session(s),
                    Ok(Connexion::PasskeyRequise(defi)) => {
                        second_facteur.set(Some((*defi).clone()));
                        finir_passkey(*defi);
                    }
                    Err(e) => erreur.set(Some(message_erreur(&e))),
                }
            });
        }
    };

    let reessayer_second_facteur = {
        let finir_passkey = finir_passkey.clone();
        Callback::new(move |_| {
            if let Some(d) = second_facteur.get_untracked() {
                finir_passkey(d);
            }
        })
    };

    let etab_prerempli =
        move || !modifier_etab.get() && etablissement.with(|e| e.trim().parse::<Uuid>().is_ok());

    view! {
        <Title text="Connexion"/>
        <main id="contenu" class="connexion">
            <div class="connexion__marque">
                <span class="connexion__logo"><Icone nom=NomIcone::Ecole/></span>
                <p class="connexion__etab">
                    <Suspense fallback=|| "ScolIA">
                        {move || {
                            if config.get().and_then(Result::ok).is_some_and(|c| c.labo) {
                                "Environnement de test"
                            } else {
                                "ScolIA"
                            }
                        }}
                    </Suspense>
                </p>
                <h1>"Connexion à l'ENT"</h1>
            </div>

            <div class="connexion__carte">
                <Suspense fallback=|| ()>
                    {move || {
                        config
                            .get()
                            .and_then(Result::ok)
                            .filter(|c| c.labo)
                            .map(|_| {
                                view! {
                                    <p class="bandeau-labo">
                                        "Données fictives uniquement."
                                    </p>
                                }
                            })
                    }}
                </Suspense>
                {move || info_fin().map(|m| view! { <Alerte niveau=Niveau::Info titre=m/> })}

                <form class="connexion__formulaire" on:submit=soumettre novalidate>
                    <ChampTexte
                        id="email"
                        libelle="Adresse e-mail"
                        valeur=email
                        genre="email"
                        requis=true
                        autocomplete="username webauthn"
                        erreur=Signal::derive(move || erreur_email.get())
                    />

                    <Bouton
                        variante=Variante::Sombre
                        en_cours=Signal::derive(move || en_cours.get())
                        on_click=Callback::new(connexion_passkey)
                    >
                        <Icone nom=NomIcone::Passkey/>
                        "Se connecter avec une passkey"
                    </Bouton>
                    <p class="connexion__aide">"Empreinte, visage ou code de votre appareil."</p>

                    <div class="connexion__separateur" aria-hidden="true"><span>"ou"</span></div>

                    <ChampTexte
                        id="mdp"
                        libelle="Mot de passe"
                        valeur=mot_de_passe
                        genre="password"
                        autocomplete="current-password"
                    />
                    <Show
                        when=etab_prerempli
                        fallback=move || {
                            view! {
                                // Transitoire : l'établissement sera déduit de l'adresse du site (D-012).
                                <ChampTexte id="etab" libelle="Établissement" valeur=etablissement
                                    aide="Identifiant fourni par votre établissement."/>
                            }
                        }
                    >
                        <p class="connexion__etab-choisi">
                            "Établissement de test présélectionné. "
                            <button type="button" class="connexion__lien" on:click=move |_| modifier_etab.set(true)>
                                "Modifier"
                            </button>
                        </p>
                    </Show>

                    {move || {
                        erreur
                            .get()
                            .map(|m| {
                                view! {
                                    <Alerte niveau=Niveau::Erreur titre=m>
                                        {move || {
                                            second_facteur
                                                .get()
                                                .is_some()
                                                .then(|| {
                                                    view! {
                                                        <p>
                                                            <Bouton variante=Variante::Secondaire on_click=reessayer_second_facteur>
                                                                "Réessayer avec ma passkey"
                                                            </Bouton>
                                                        </p>
                                                    }
                                                })
                                        }}
                                    </Alerte>
                                }
                            })
                    }}
                    {move || {
                        (second_facteur.get().is_some() && en_cours.get())
                            .then(|| {
                                view! {
                                    <Alerte niveau=Niveau::Info titre="Second facteur : validez avec votre passkey."/>
                                }
                            })
                    }}

                    <Bouton soumettre=true variante=Variante::Primaire en_cours=Signal::derive(move || en_cours.get())>
                        "Continuer"
                    </Bouton>
                </form>
            </div>
            <p class="connexion__pied">
                "Pas encore de compte ? Votre établissement vous envoie une invitation par e-mail."
            </p>
        </main>
    }
}

fn capitaliser(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|p| p.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}
