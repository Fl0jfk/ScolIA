//! Passkeys (WebAuthn) — 2ᵉ facteur **et** connexion sans mot de passe.
//!
//! Choix structurant : l'ENT ne s'appuie **que** sur WebAuthn (passkey /
//! Touch ID / Face ID / clé de sécurité). Pas de TOTP, pas d'application
//! d'authentification tierce : le secret ne quitte jamais l'appareil de
//! l'utilisateur et la vérification est asymétrique (aucun secret partagé
//! serveur).
//!
//! Ce module isole la mécanique cryptographique (challenge, attestation,
//! assertion, compteur d'usage) de la persistance : il ne touche pas à
//! Postgres. La façade `ent-db` stocke les états de challenge et les
//! credentials.

use thiserror::Error;
use uuid::Uuid;
use webauthn_rs::prelude::WebauthnBuilder;

pub use webauthn_rs::prelude::{
    AuthenticationResult, CreationChallengeResponse, CredentialID, Passkey, PasskeyAuthentication,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Url,
};

/// Nom de la « partie » affiché par le navigateur lors de l'enrôlement (ex.
/// « ScolIA ENT »). Configurable par déploiement.
pub const DEFAULT_RP_NAME: &str = "ScolIA ENT";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PasskeyError {
    #[error("configuration WebAuthn invalide (rp_id / rp_origin)")]
    Config,
    #[error("démarrage de l'enrôlement de passkey impossible")]
    RegisterStart,
    #[error("enrôlement de passkey refusé par le vérificateur")]
    RegisterFinish,
    #[error("démarrage de l'authentification par passkey impossible")]
    AuthStart,
    #[error("authentification par passkey refusée")]
    AuthFinish,
    #[error("vérification utilisateur absente (biométrie / code refusé)")]
    UserVerificationRequired,
    #[error("état de passkey illisible")]
    BadState,
    /// Compteur de signature non croissant : la clé a probablement été clonée.
    #[error("authentification par passkey refusée : compteur de signature régressé (clonage possible)")]
    PossibleCompromise,
    /// Configuration WebAuthn refusée au démarrage (message explicite pour l'opérateur).
    #[error("configuration WebAuthn invalide : {0}")]
    ConfigInvalide(String),
}

/// Origine (RP ID + origine complète) lue depuis l'environnement, avec des
/// valeurs de repli sûres pour le développement local.
///
/// - `ENT_RP_ID`   : domaine sans schéma (défaut `localhost`).
/// - `ENT_RP_ORIGIN` : origine complète (défaut `http://localhost:3000`).
/// - `ENT_RP_NAME` : libellé affiché (défaut [`DEFAULT_RP_NAME`]).
pub fn rp_config_from_env() -> (String, String, String) {
    let rp_id = std::env::var("ENT_RP_ID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "localhost".to_string());
    let rp_origin = std::env::var("ENT_RP_ORIGIN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "http://localhost:3000".to_string());
    let rp_name = std::env::var("ENT_RP_NAME")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_RP_NAME.to_string());
    (rp_id, rp_origin, rp_name)
}

/// Configuration WebAuthn résolue et validée (A5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpConfig {
    pub rp_id: String,
    /// Première origine = origine principale ; les suivantes sont autorisées en plus
    /// (autres sous-domaines, origines natives `android:apk-key-hash:…`).
    pub origins: Vec<Url>,
    pub rp_name: String,
    pub allow_subdomains: bool,
}

fn cfg_err(msg: impl Into<String>) -> PasskeyError {
    PasskeyError::ConfigInvalide(msg.into())
}

/// Origine native Android : `android:apk-key-hash:<base64url>` (forme exacte).
fn is_android_origin(u: &Url) -> bool {
    u.scheme() == "android"
        && u.path().strip_prefix("apk-key-hash:").is_some_and(|h| {
            !h.is_empty()
                && h.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'+' || b == b'/' || b == b'=')
        })
        && u.query().is_none()
        && u.fragment().is_none()
}

impl RpConfig {
    /// Résout la configuration depuis des variables (fonction `get`) — pure, testable.
    ///
    /// - `ENT_RP_ID`, `ENT_RP_ORIGIN`, `ENT_RP_ORIGINS` (liste séparée par des virgules,
    ///   s'ajoute à `ENT_RP_ORIGIN`), `ENT_RP_NAME`, `ENT_RP_ALLOW_SUBDOMAINS` (`1`/`true`).
    /// - **Hors labo/test** (`non_prod = false`) : `ENT_RP_ID` obligatoire (nom de domaine
    ///   enregistrable, ni `localhost` ni IP), au moins une origine, **toutes** les origines
    ///   web en `https` et sous le `rp_id` (hôte égal ou sous-domaine), sans chemin / requête.
    ///   Les origines natives Android sont acceptées. Sinon : erreur explicite au démarrage.
    /// - En labo/test : replis `localhost` / `http://localhost:3000` comme avant.
    pub fn resolve(
        get: &dyn Fn(&str) -> Option<String>,
        non_prod: bool,
    ) -> Result<Self, PasskeyError> {
        let var = |k: &str| get(k).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let rp_name = var("ENT_RP_NAME").unwrap_or_else(|| DEFAULT_RP_NAME.to_string());
        let rp_id = match var("ENT_RP_ID") {
            Some(v) => v.to_ascii_lowercase(),
            None if non_prod => "localhost".to_string(),
            None => {
                return Err(cfg_err(
                    "ENT_RP_ID est obligatoire hors labo/test (ex. scolia.fr) ; \
                     refus de démarrer avec « localhost »",
                ))
            }
        };
        if rp_id.contains(['/', ':', '@', ' ', '?', '#']) || rp_id.starts_with('.') || rp_id.ends_with('.')
        {
            return Err(cfg_err("ENT_RP_ID doit être un nom de domaine sans schéma ni port"));
        }
        if !non_prod {
            let est_ip = rp_id.parse::<std::net::IpAddr>().is_ok();
            if rp_id == "localhost" || est_ip || !rp_id.contains('.') {
                return Err(cfg_err(
                    "ENT_RP_ID doit être un domaine public (ni localhost, ni IP, ni nom sans point)",
                ));
            }
        }

        let mut raw: Vec<String> = Vec::new();
        if let Some(o) = var("ENT_RP_ORIGIN") {
            raw.push(o);
        }
        if let Some(list) = var("ENT_RP_ORIGINS") {
            raw.extend(
                list.split(',')
                    .map(|o| o.trim().to_string())
                    .filter(|o| !o.is_empty()),
            );
        }
        if raw.is_empty() {
            if non_prod {
                raw.push("http://localhost:3000".to_string());
            } else {
                return Err(cfg_err(
                    "au moins une origine https est obligatoire hors labo/test \
                     (ENT_RP_ORIGIN ou ENT_RP_ORIGINS, ex. https://scolia.fr)",
                ));
            }
        }
        let mut origins: Vec<Url> = Vec::new();
        for o in &raw {
            let url = Url::parse(o).map_err(|_| cfg_err(format!("origine illisible : {o}")))?;
            if is_android_origin(&url) {
                // Origine native : comparée exactement par webauthn-rs.
            } else {
                if !matches!(url.scheme(), "http" | "https") {
                    return Err(cfg_err(format!("schéma d'origine non pris en charge : {o}")));
                }
                if url.scheme() == "http" && !non_prod {
                    return Err(cfg_err(format!("origine http refusée hors labo/test : {o}")));
                }
                if url.path() != "/"
                    || url.query().is_some()
                    || url.fragment().is_some()
                    || !url.username().is_empty()
                    || url.password().is_some()
                {
                    return Err(cfg_err(format!(
                        "une origine ne porte ni chemin, ni requête, ni identifiants : {o}"
                    )));
                }
                let host = url.domain().ok_or_else(|| cfg_err(format!("origine sans nom de domaine : {o}")))?;
                if host != rp_id && !host.ends_with(&format!(".{rp_id}")) {
                    return Err(cfg_err(format!(
                        "l'origine {o} n'est pas sous le rp_id « {rp_id} »"
                    )));
                }
            }
            if !origins.contains(&url) {
                origins.push(url);
            }
        }
        if !non_prod && !origins.iter().any(|u| u.scheme() == "https") {
            return Err(cfg_err("au moins une origine https est obligatoire hors labo/test"));
        }
        // L'origine principale doit être une origine web (le builder la valide face au rp_id).
        if let Some(i) = origins.iter().position(|u| u.scheme() != "android") {
            origins.swap(0, i);
        } else {
            return Err(cfg_err("au moins une origine web est requise (en plus des origines natives)"));
        }
        let allow_subdomains = var("ENT_RP_ALLOW_SUBDOMAINS")
            .is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "oui"));
        Ok(Self {
            rp_id,
            origins,
            rp_name,
            allow_subdomains,
        })
    }

    /// Depuis les variables d'environnement du processus et `ENT_ENV`.
    pub fn from_env() -> Result<Self, PasskeyError> {
        Self::resolve(
            &|k| std::env::var(k).ok(),
            crate::env::env_non_prod(),
        )
    }
}

/// Moteur WebAuthn — construit une fois par processus à partir de la config
/// d'établissement/déploiement.
pub struct PasskeyEngine {
    webauthn: webauthn_rs::Webauthn,
}

impl PasskeyEngine {
    /// Moteur à origine unique (tests, labo).
    pub fn new(rp_id: &str, rp_origin: &str, rp_name: &str) -> Result<Self, PasskeyError> {
        let origin = Url::parse(rp_origin).map_err(|_| PasskeyError::Config)?;
        let webauthn = WebauthnBuilder::new(rp_id, &origin)
            .map_err(|_| PasskeyError::Config)?
            .rp_name(rp_name)
            .build()
            .map_err(|_| PasskeyError::Config)?;
        Ok(Self { webauthn })
    }

    /// Moteur depuis une configuration validée (origines multiples, sous-domaines).
    pub fn from_config(cfg: &RpConfig) -> Result<Self, PasskeyError> {
        let first = cfg.origins.first().ok_or(PasskeyError::Config)?;
        let mut b = WebauthnBuilder::new(&cfg.rp_id, first)
            .map_err(|_| PasskeyError::Config)?
            .rp_name(&cfg.rp_name)
            .allow_subdomains(cfg.allow_subdomains);
        for extra in cfg.origins.iter().skip(1) {
            b = b.append_allowed_origin(extra);
        }
        Ok(Self {
            webauthn: b.build().map_err(|_| PasskeyError::Config)?,
        })
    }

    /// Construit le moteur depuis l'environnement ([`RpConfig::from_env`]) :
    /// fail-closed hors labo/test (voir [`RpConfig::resolve`]).
    pub fn from_env() -> Result<Self, PasskeyError> {
        Self::from_config(&RpConfig::from_env()?)
    }

    /// Ouvre une session d'enrôlement. `exclude` liste les credentials déjà
    /// connus pour éviter qu'un même appareil s'enrôle deux fois.
    pub fn start_registration(
        &self,
        user_id: Uuid,
        user_name: &str,
        user_display_name: &str,
        exclude: Vec<CredentialID>,
    ) -> Result<(CreationChallengeResponse, PasskeyRegistration), PasskeyError> {
        self.webauthn
            .start_passkey_registration(user_id, user_name, user_display_name, Some(exclude))
            .map_err(|_| PasskeyError::RegisterStart)
    }

    /// Vérifie l'attestation et produit la `Passkey` à persister.
    pub fn finish_registration(
        &self,
        credential: &RegisterPublicKeyCredential,
        state: &PasskeyRegistration,
    ) -> Result<Passkey, PasskeyError> {
        self.webauthn
            .finish_passkey_registration(credential, state)
            .map_err(|_| PasskeyError::RegisterFinish)
    }

    /// Ouvre une session d'authentification à partir des passkeys d'un compte.
    pub fn start_authentication(
        &self,
        passkeys: &[Passkey],
    ) -> Result<(RequestChallengeResponse, PasskeyAuthentication), PasskeyError> {
        self.webauthn
            .start_passkey_authentication(passkeys)
            .map_err(|_| PasskeyError::AuthStart)
    }

    /// Vérifie l'assertion. La vérification utilisateur (biométrie / code) est
    /// **exigée** : elle est ce qui distingue un 2ᵉ facteur d'un simple
    /// porteur d'appareil.
    pub fn finish_authentication(
        &self,
        credential: &PublicKeyCredential,
        state: &PasskeyAuthentication,
    ) -> Result<AuthenticationResult, PasskeyError> {
        let res = self
            .webauthn
            .finish_passkey_authentication(credential, state)
            .map_err(|e| match e {
                webauthn_rs::prelude::WebauthnError::CredentialPossibleCompromise => {
                    PasskeyError::PossibleCompromise
                }
                _ => PasskeyError::AuthFinish,
            })?;
        if !res.user_verified() {
            return Err(PasskeyError::UserVerificationRequired);
        }
        Ok(res)
    }

    /// Rejoue le compteur d'usage d'une passkey après une authentification
    /// réussie. `Ok(true)` = à persister ; `Ok(false)` = rien à écrire ;
    /// `Err(AuthFinish)` = le résultat ne concerne pas ce credential (erreur
    /// **non avalée**). La régression du compteur (clonage) est déjà refusée
    /// par `finish_authentication` ([`PasskeyError::PossibleCompromise`]).
    pub fn apply_counter(
        passkey: &mut Passkey,
        res: &AuthenticationResult,
    ) -> Result<bool, PasskeyError> {
        passkey
            .update_credential(res)
            .ok_or(PasskeyError::AuthFinish)
    }
}

impl std::fmt::Debug for PasskeyEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasskeyEngine").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dev_par_defaut_acceptee() {
        // http://localhost est une origine « secure context » : indispensable
        // pour tester les passkeys en local sans TLS.
        let engine = PasskeyEngine::new("localhost", "http://localhost:3000", DEFAULT_RP_NAME);
        assert!(engine.is_ok(), "origine localhost doit être acceptée");
    }

    #[test]
    fn origine_invalide_refusee() {
        assert_eq!(
            PasskeyEngine::new("localhost", "pas-une-url", DEFAULT_RP_NAME).unwrap_err(),
            PasskeyError::Config
        );
    }

    #[test]
    fn rp_config_env_repli() {
        // Les variables d'environnement ne sont pas posées dans ce test :
        // on doit retomber sur les valeurs de dev.
        std::env::remove_var("ENT_RP_ID");
        std::env::remove_var("ENT_RP_ORIGIN");
        std::env::remove_var("ENT_RP_NAME");
        let (id, origin, name) = rp_config_from_env();
        assert_eq!(id, "localhost");
        assert_eq!(origin, "http://localhost:3000");
        assert_eq!(name, DEFAULT_RP_NAME);
    }

    #[test]
    fn demarrage_enrolement_produit_un_challenge() {
        let engine = PasskeyEngine::new("localhost", "http://localhost:3000", DEFAULT_RP_NAME)
            .expect("engine");
        let uid = Uuid::new_v4();
        let (ccr, _state) = engine
            .start_registration(uid, "claire@ent.local", "Claire", vec![])
            .expect("start registration");
        // Le challenge porte un identifiant de credential utilisateur et un
        // challenge non vide, prêts à être sérialisés vers le navigateur.
        assert_eq!(ccr.public_key.user.id.as_slice(), uid.as_bytes());
        assert!(!ccr.public_key.challenge.is_empty());
    }

    #[test]
    fn enrolement_avec_charge_invalide_refuse() {
        // Un enrôlement fabriqué ne peut pas valider un état de challenge
        // réel : la vérification échoue proprement (pas de panic).
        let engine = PasskeyEngine::new("localhost", "http://localhost:3000", DEFAULT_RP_NAME)
            .expect("engine");
        let uid = Uuid::new_v4();
        let (ccr, state) = engine
            .start_registration(uid, "claire@ent.local", "Claire", vec![])
            .expect("start registration");
        let fake: RegisterPublicKeyCredential = serde_json::from_value(serde_json::json!({
            "id": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "rawId": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "type": "public-key",
            "response": {
                "attestationObject": "AAAAAA",
                "clientDataJSON": "AAAAAA"
            }
        }))
        .expect("fake credential json");
        assert_eq!(
            engine.finish_registration(&fake, &state).unwrap_err(),
            PasskeyError::RegisterFinish
        );
        // Le state reste inutilisable pour un second essai (non consommé) :
        // la persistance à usage unique est de la responsabilité de l'appelant.
        assert!(!ccr.public_key.challenge.is_empty());
    }

    // ---- A5 : configuration RP stricte ------------------------------------

    fn cfg(vars: &[(&str, &str)], non_prod: bool) -> Result<RpConfig, PasskeyError> {
        let m: std::collections::HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        RpConfig::resolve(&|k| m.get(k).cloned(), non_prod)
    }

    fn msg(r: Result<RpConfig, PasskeyError>) -> String {
        match r {
            Err(PasskeyError::ConfigInvalide(m)) => m,
            other => panic!("ConfigInvalide attendue, obtenu {other:?}"),
        }
    }

    #[test]
    fn prod_sans_rp_id_refuse_de_demarrer() {
        assert!(msg(cfg(&[("ENT_RP_ORIGIN", "https://scolia.fr")], false)).contains("ENT_RP_ID"));
        assert!(msg(cfg(&[], false)).contains("ENT_RP_ID"));
        // Pas de repli silencieux sur localhost.
        assert!(msg(cfg(&[("ENT_RP_ID", "localhost"), ("ENT_RP_ORIGIN", "https://localhost")], false))
            .contains("domaine public"));
        assert!(msg(cfg(&[("ENT_RP_ID", "10.0.0.1"), ("ENT_RP_ORIGIN", "https://10.0.0.1")], false))
            .contains("domaine public"));
    }

    #[test]
    fn prod_exige_une_origine_https() {
        assert!(msg(cfg(&[("ENT_RP_ID", "scolia.fr")], false)).contains("origine https"));
        assert!(msg(cfg(&[("ENT_RP_ID", "scolia.fr"), ("ENT_RP_ORIGIN", "http://scolia.fr")], false))
            .contains("http refusée"));
        // Une seule origine http parmi plusieurs https est aussi refusée.
        assert!(msg(cfg(
            &[
                ("ENT_RP_ID", "scolia.fr"),
                ("ENT_RP_ORIGIN", "https://scolia.fr"),
                ("ENT_RP_ORIGINS", "http://lpnb.scolia.fr"),
            ],
            false
        ))
        .contains("http refusée"));
    }

    #[test]
    fn origine_hors_rp_id_ou_avec_chemin_refusee() {
        assert!(msg(cfg(&[("ENT_RP_ID", "scolia.fr"), ("ENT_RP_ORIGIN", "https://evil.example")], false))
            .contains("pas sous le rp_id"));
        assert!(msg(cfg(&[("ENT_RP_ID", "scolia.fr"), ("ENT_RP_ORIGIN", "https://notscolia.fr")], false))
            .contains("pas sous le rp_id"));
        assert!(msg(cfg(&[("ENT_RP_ID", "scolia.fr"), ("ENT_RP_ORIGIN", "https://scolia.fr/login")], false))
            .contains("chemin"));
    }

    #[test]
    fn prod_valide_liste_dorigines_et_sous_domaines() {
        let c = cfg(
            &[
                ("ENT_RP_ID", "Scolia.fr"),
                ("ENT_RP_ORIGINS", "https://lpnb.scolia.fr, https://scolia.fr,android:apk-key-hash:abc-_123"),
            ],
            false,
        )
        .expect("config valide");
        assert_eq!(c.rp_id, "scolia.fr");
        assert_eq!(c.origins.len(), 3);
        assert!(!c.allow_subdomains);
        let engine = PasskeyEngine::from_config(&c).expect("moteur");
        let uid = Uuid::new_v4();
        assert!(engine.start_registration(uid, "a@b.fr", "A", vec![]).is_ok());
        let c2 = cfg(
            &[("ENT_RP_ID", "scolia.fr"), ("ENT_RP_ORIGIN", "https://scolia.fr"), ("ENT_RP_ALLOW_SUBDOMAINS", "1")],
            false,
        )
        .unwrap();
        assert!(c2.allow_subdomains);
    }

    #[test]
    fn origine_native_android_forme_stricte() {
        assert!(msg(cfg(
            &[("ENT_RP_ID", "scolia.fr"), ("ENT_RP_ORIGIN", "https://scolia.fr"), ("ENT_RP_ORIGINS", "android:autre:xyz")],
            false
        ))
        .contains("schéma"));
    }

    #[test]
    fn labo_garde_les_replis_de_dev() {
        let c = cfg(&[], true).expect("dev");
        assert_eq!(c.rp_id, "localhost");
        assert_eq!(c.origins[0].as_str(), "http://localhost:3000/");
        assert!(PasskeyEngine::from_config(&c).is_ok());
    }

    /// Les origines additionnelles et les sous-domaines sont réellement acceptés / refusés
    /// par la vérification d'une assertion (SoftPasskey, de bout en bout).
    #[test]
    fn origines_multiples_et_sous_domaines_verifies_de_bout_en_bout() {
        use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
        use webauthn_authenticator_rs::softpasskey::SoftPasskey;
        let ok_pour = |cfg_vars: &[(&str, &str)], origin: &str| -> bool {
            let engine = PasskeyEngine::from_config(&cfg(cfg_vars, false).unwrap()).unwrap();
            let (ccr, state) = engine
                .start_registration(Uuid::new_v4(), "a@b.fr", "A", vec![])
                .unwrap();
            let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
            // Le client (authenticator) refuse déjà une origine étrangère au rp_id,
            // comme un navigateur : c'est aussi un refus.
            match dev.do_registration(Url::parse(origin).unwrap(), ccr) {
                Ok(reg) => engine.finish_registration(&reg, &state).is_ok(),
                Err(_) => false,
            }
        };
        let base = [("ENT_RP_ID", "scolia.fr"), ("ENT_RP_ORIGIN", "https://scolia.fr")];
        assert!(ok_pour(&base, "https://scolia.fr"));
        assert!(!ok_pour(&base, "https://lpnb.scolia.fr"), "sous-domaine refusé par défaut");
        assert!(!ok_pour(&base, "https://evil.example"));
        let liste = [
            ("ENT_RP_ID", "scolia.fr"),
            ("ENT_RP_ORIGIN", "https://scolia.fr"),
            ("ENT_RP_ORIGINS", "https://lpnb.scolia.fr"),
        ];
        assert!(ok_pour(&liste, "https://lpnb.scolia.fr"), "origine listée");
        assert!(!ok_pour(&liste, "https://autre.scolia.fr"), "origine non listée");
        let sous = [
            ("ENT_RP_ID", "scolia.fr"),
            ("ENT_RP_ORIGIN", "https://scolia.fr"),
            ("ENT_RP_ALLOW_SUBDOMAINS", "1"),
        ];
        assert!(ok_pour(&sous, "https://autre.scolia.fr"), "sous-domaine autorisé");
        assert!(!ok_pour(&sous, "https://scolia.fr.evil.example"));
        assert!(!ok_pour(&sous, "http://autre.scolia.fr"), "http refusé");
    }

    // ---- A8 : régression du compteur de signature --------------------------

    #[test]
    fn compteur_regresse_refuse_par_webauthn_rs_et_distingue() {
        use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
        use webauthn_authenticator_rs::softpasskey::SoftPasskey;
        let origin = Url::parse("http://localhost:3000").unwrap();
        let engine = PasskeyEngine::new("localhost", origin.as_str(), DEFAULT_RP_NAME).unwrap();
        let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let (ccr, st) = engine
            .start_registration(Uuid::new_v4(), "a@b.fr", "A", vec![])
            .unwrap();
        let reg = dev.do_registration(origin.clone(), ccr).unwrap();
        let mut pk = engine.finish_registration(&reg, &st).unwrap();

        let mut authentifier = |pk: &Passkey| {
            let (rcr, st) = engine.start_authentication(std::slice::from_ref(pk)).unwrap();
            let cred = dev.do_authentication(origin.clone(), rcr).unwrap();
            engine.finish_authentication(&cred, &st)
        };
        // Deux connexions normales : le compteur progresse et s'applique.
        for _ in 0..2 {
            let res = authentifier(&pk).expect("connexion normale");
            assert_eq!(PasskeyEngine::apply_counter(&mut pk, &res), Ok(true));
        }
        // Un clone a déjà fait monter le compteur côté serveur : l'original « recule ».
        let mut v = serde_json::to_value(&pk).unwrap();
        let slot = v.pointer_mut("/cred/counter").expect("compteur sérialisé");
        *slot = serde_json::json!(1000);
        let pk_clone: Passkey = serde_json::from_value(v).unwrap();
        assert_eq!(
            authentifier(&pk_clone).unwrap_err(),
            PasskeyError::PossibleCompromise
        );
    }

    #[test]
    fn apply_counter_nest_plus_avale_pour_un_autre_credential() {
        use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
        use webauthn_authenticator_rs::softpasskey::SoftPasskey;
        let origin = Url::parse("http://localhost:3000").unwrap();
        let engine = PasskeyEngine::new("localhost", origin.as_str(), DEFAULT_RP_NAME).unwrap();
        let mut enrol = || {
            let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
            let (ccr, st) = engine.start_registration(Uuid::new_v4(), "a@b.fr", "A", vec![]).unwrap();
            let reg = dev.do_registration(origin.clone(), ccr).unwrap();
            (dev, engine.finish_registration(&reg, &st).unwrap())
        };
        let (mut dev1, pk1) = enrol();
        let (_dev2, mut pk2) = enrol();
        let (rcr, st) = engine.start_authentication(std::slice::from_ref(&pk1)).unwrap();
        let cred = dev1.do_authentication(origin.clone(), rcr).unwrap();
        let res = engine.finish_authentication(&cred, &st).unwrap();
        assert_eq!(PasskeyEngine::apply_counter(&mut pk2, &res), Err(PasskeyError::AuthFinish));
    }
}
