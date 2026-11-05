//! Connexion : passkey (sans mot de passe), ou mot de passe puis passkey si le compte
//! en a une (second facteur). Routes d'authentification existantes, **inchangées**.

// WebAuthn partagé (connexion et enrôlement) : vit dans le noyau.
pub use crate::noyau::webauthn;

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

/// Message générique quand le second facteur est abandonné.
const MESSAGE_SECOND_FACTEUR_ECHOUE: &str =
    "La vérification par passkey a échoué. Saisissez à nouveau votre mot de passe.";

/// Échec du second facteur, selon l'endroit où il survient.
#[derive(Clone, Debug)]
enum EchecSecondFacteur {
    /// Avant tout envoi au serveur (annulation, refus ou délai de l'appareil) : message
    /// déjà formulé.
    Navigateur(String),
    /// Après envoi : refus (401, 400, 429…) ou erreur réseau. Le défi est à usage unique
    /// côté serveur : on ne sait pas s'il est consommé, on ne le rejoue pas.
    Serveur(ApiError),
}

/// Suite à donner à un échec du second facteur (avec le message à afficher).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Suite {
    /// Même défi, bouton « Réessayer avec ma passkey ».
    Reessayer(String),
    /// Défi inutilisable : retour au mot de passe.
    Abandonner(String),
}

/// Décision unique (utilisée par l'écran et testée) : seul un échec local avant envoi
/// autorise à rejouer le défi. Aucune horloge du poste n'intervient : si le défi a
/// expiré côté serveur, c'est le serveur qui le refusera, et on abandonnera alors.
fn suite_echec_second_facteur(echec: &EchecSecondFacteur) -> Suite {
    match echec {
        EchecSecondFacteur::Navigateur(m) => Suite::Reessayer(m.clone()),
        EchecSecondFacteur::Serveur(ApiError::TropDeRequetes) => Suite::Abandonner(
            "Trop de tentatives. Patientez quelques minutes puis saisissez à nouveau votre mot de passe."
                .into(),
        ),
        EchecSecondFacteur::Serveur(ApiError::Reseau(_) | ApiError::Indisponible) => {
            Suite::Abandonner(
                "Le serveur est injoignable. Vérifiez la connexion puis saisissez à nouveau votre mot de passe."
                    .into(),
            )
        }
        EchecSecondFacteur::Serveur(_) => Suite::Abandonner(MESSAGE_SECOND_FACTEUR_ECHOUE.into()),
    }
}

/// Événements qui font évoluer le défi de second facteur conservé.
enum EvenementSecondFacteur<'a> {
    /// Nouvelle soumission, connexion passkey directe ou changement d'e-mail.
    NouvelleTentative,
    /// Le serveur demande un second facteur.
    DefiRecu(PasskeyChallengeDto),
    /// Connexion réussie.
    Reussite,
    Echec(&'a Suite),
}

/// Transition pure du défi conservé (`None` = aucun second facteur en attente).
fn second_facteur_suivant(
    courant: Option<PasskeyChallengeDto>,
    ev: EvenementSecondFacteur<'_>,
) -> Option<PasskeyChallengeDto> {
    match ev {
        EvenementSecondFacteur::NouvelleTentative | EvenementSecondFacteur::Reussite => None,
        EvenementSecondFacteur::DefiRecu(d) => Some(d),
        EvenementSecondFacteur::Echec(Suite::Reessayer(_)) => courant,
        EvenementSecondFacteur::Echec(Suite::Abandonner(_)) => None,
    }
}

/// Applique un événement au signal du défi.
fn appliquer(
    second_facteur: RwSignal<Option<PasskeyChallengeDto>>,
    ev: EvenementSecondFacteur<'_>,
) {
    second_facteur.update(|c| *c = second_facteur_suivant(c.take(), ev));
}

/// Second facteur abandonné : mot de passe vidé et focalisé, e-mail conservé.
fn revenir_au_mot_de_passe(mot_de_passe: RwSignal<String>) {
    mot_de_passe.set(String::new());
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::JsCast;
        if let Some(c) = document()
            .get_element_by_id("mdp")
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = c.focus();
        }
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

    // Adresse modifiée pendant qu'un second facteur est en attente : le défi appartient
    // à l'ancien compte, plus de « Réessayer » (et le message qui l'accompagnait part).
    Effect::new(move |precedent: Option<String>| {
        let courant = email.get();
        if precedent.is_some_and(|p| p != courant) && second_facteur.get_untracked().is_some() {
            appliquer(second_facteur, EvenementSecondFacteur::NouvelleTentative);
            erreur.set(None);
        }
        courant
    });

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
            Some("locale") => Some(session::MESSAGE_DECONNEXION_LOCALE),
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
        // `second` : le défi vient d'un mot de passe valide (second facteur) ; il est à
        // usage unique et sa suite est décidée par `suite_echec_second_facteur`.
        move |defi: PasskeyChallengeDto, second: bool| {
            let ouvrir_session = ouvrir_session.clone();
            erreur.set(None);
            en_cours.set(true);
            spawn_local(async move {
                let resultat = match webauthn::signer_defi(&defi.options).await {
                    Ok(assertion) => session::client()
                        .connexion_passkey_fin(&defi.token, assertion)
                        .await
                        .map_err(EchecSecondFacteur::Serveur),
                    Err(m) => Err(EchecSecondFacteur::Navigateur(format!(
                        "{}.",
                        capitaliser(&m)
                    ))),
                };
                en_cours.set(false);
                match resultat {
                    Ok(s) => {
                        appliquer(second_facteur, EvenementSecondFacteur::Reussite);
                        ouvrir_session(s);
                    }
                    Err(echec) if second => {
                        let suite = suite_echec_second_facteur(&echec);
                        appliquer(second_facteur, EvenementSecondFacteur::Echec(&suite));
                        match suite {
                            Suite::Reessayer(m) => erreur.set(Some(m)),
                            Suite::Abandonner(m) => {
                                revenir_au_mot_de_passe(mot_de_passe);
                                erreur.set(Some(m));
                            }
                        }
                    }
                    // Connexion directe par passkey : un nouvel essai repart de zéro.
                    Err(EchecSecondFacteur::Navigateur(m)) => erreur.set(Some(m)),
                    Err(EchecSecondFacteur::Serveur(
                        ApiError::NonConnecte | ApiError::Interdit(_),
                    )) => erreur.set(Some("La passkey n'a pas été reconnue.".into())),
                    Err(EchecSecondFacteur::Serveur(e)) => {
                        erreur.set(Some(e.message_utilisateur()))
                    }
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
            appliquer(second_facteur, EvenementSecondFacteur::NouvelleTentative);
            en_cours.set(true);
            erreur.set(None);
            spawn_local(async move {
                match session::client()
                    .connexion_passkey_debut(&adresse, etab)
                    .await
                {
                    Ok(defi) => finir_passkey(defi, false),
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
            appliquer(second_facteur, EvenementSecondFacteur::NouvelleTentative);
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
                        appliquer(
                            second_facteur,
                            EvenementSecondFacteur::DefiRecu((*defi).clone()),
                        );
                        finir_passkey(*defi, true);
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
                finir_passkey(d, true);
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

#[cfg(test)]
mod tests_second_facteur {
    use super::*;

    fn defi() -> PasskeyChallengeDto {
        PasskeyChallengeDto {
            token: "t".into(),
            options: serde_json::json!({}),
            expires_at: chrono::Utc::now(),
        }
    }

    fn local() -> EchecSecondFacteur {
        EchecSecondFacteur::Navigateur("Annulée.".into())
    }

    #[test]
    fn echec_local_reessaie_avec_le_meme_message() {
        assert_eq!(
            suite_echec_second_facteur(&local()),
            Suite::Reessayer("Annulée.".into())
        );
    }

    #[test]
    fn horloge_du_poste_sans_effet_sur_la_decision() {
        // Défi « expiré » selon l'horloge locale (expires_at = maintenant) : un échec
        // local propose toujours de réessayer et conserve le même défi ; le serveur
        // tranchera.
        let d = defi();
        let suite = suite_echec_second_facteur(&local());
        assert!(matches!(suite, Suite::Reessayer(_)));
        assert_eq!(
            second_facteur_suivant(Some(d.clone()), EvenementSecondFacteur::Echec(&suite)),
            Some(d)
        );
    }

    #[test]
    fn refus_serveur_abandonne_avec_message_generique() {
        for e in [
            ApiError::NonConnecte,
            ApiError::Interdit("x".into()),
            ApiError::Invalide("x".into()),
            ApiError::Serveur(500, "x".into()),
        ] {
            assert_eq!(
                suite_echec_second_facteur(&EchecSecondFacteur::Serveur(e)),
                Suite::Abandonner(MESSAGE_SECOND_FACTEUR_ECHOUE.into())
            );
        }
    }

    #[test]
    fn limitation_et_reseau_ont_un_message_precis() {
        let Suite::Abandonner(m) =
            suite_echec_second_facteur(&EchecSecondFacteur::Serveur(ApiError::TropDeRequetes))
        else {
            panic!("abandon attendu")
        };
        assert!(m.starts_with("Trop de tentatives. Patientez quelques minutes puis saisissez"));
        let Suite::Abandonner(m) = suite_echec_second_facteur(&EchecSecondFacteur::Serveur(
            ApiError::Reseau("hors ligne".into()),
        )) else {
            panic!("abandon attendu")
        };
        assert!(m.starts_with("Le serveur est injoignable."));
    }

    #[test]
    fn reussite_vide_le_defi() {
        assert_eq!(
            second_facteur_suivant(Some(defi()), EvenementSecondFacteur::Reussite),
            None
        );
    }

    #[test]
    fn abandon_vide_le_defi() {
        let suite = suite_echec_second_facteur(&EchecSecondFacteur::Serveur(ApiError::NonConnecte));
        assert_eq!(
            second_facteur_suivant(Some(defi()), EvenementSecondFacteur::Echec(&suite)),
            None
        );
    }

    #[test]
    fn nouvelle_tentative_ou_changement_d_email_reinitialise() {
        assert_eq!(
            second_facteur_suivant(Some(defi()), EvenementSecondFacteur::NouvelleTentative),
            None
        );
        let d = defi();
        assert_eq!(
            second_facteur_suivant(None, EvenementSecondFacteur::DefiRecu(d.clone())),
            Some(d)
        );
    }
}
