//! Client typé des routes JSON **existantes** d'`ent-api`.
//!
//! Le backend n'est pas modifié : ces types reproduisent le contrat JSON de
//! `ent-api/src/dto.rs` (privé et sérialisation seule côté serveur). À terme, une
//! crate de types partagée (`ent-types`) supprimerait cette copie — évolution
//! backend à valider séparément.
//!
//! Transport :
//! - navigateur (`wasm32`) : `gloo-net` (fetch), chemins relatifs (même origine) ;
//! - natif avec la feature `natif` : `reqwest` (tests d'intégration) ;
//! - natif sans feature (rendu serveur) : non disponible — les données métier sont
//!   chargées côté navigateur, car le jeton de session n'est pas visible du serveur.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod transport;

// ---------------------------------------------------------------------------
// Types (contrat JSON d'ent-api)
// ---------------------------------------------------------------------------

// Types partagés avec ent-api : crate `ent-types` (lot 1). Les noms historiques
// du prototype sont conservés comme alias.
pub use ent_types::appel::{AppelDto as AppelApi, AppelLigneDto as LigneAppelApi};
pub use ent_types::auth::{AuthSessionDto as Session, LoginResponseDto, MeDto as Moi};
pub use ent_types::edt::EdtCreneauDto as Creneau;

/// Issue d'une connexion par mot de passe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Connexion {
    Ouverte(Session),
    /// Le compte a une passkey : le second facteur n'est pas géré par ce prototype
    /// (l'auth n'est pas modifiée dans cette mission).
    PasskeyRequise,
}

/// État d'une ligne tel que l'API le connaît (`present|absent|retard`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EtatApi {
    Present,
    Absent,
    Retard,
}

impl EtatApi {
    pub fn as_str(self) -> &'static str {
        match self {
            EtatApi::Present => "present",
            EtatApi::Absent => "absent",
            EtatApi::Retard => "retard",
        }
    }
}

/// Ce que l'API sait enregistrer. L'écran s'adapte à ces capacités au lieu de
/// promettre un enregistrement qui n'existe pas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capacites {
    /// État « dispensé » sur une ligne d'appel.
    pub dispense: bool,
    /// Commentaire libre par ligne.
    pub commentaire: bool,
}

/// Capacités de l'`ent-api` actuel : ni « dispensé » ni commentaire
/// (`ent_appel_ligne.etat IN ('present','absent','retard')`, pas de colonne commentaire).
pub const CAPACITES_API_ACTUELLE: Capacites = Capacites {
    dispense: false,
    commentaire: false,
};

// ---------------------------------------------------------------------------
// Erreurs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ApiError {
    #[error("session expirée ou absente")]
    NonConnecte,
    #[error("accès refusé : {0}")]
    Interdit(String),
    #[error("introuvable : {0}")]
    Introuvable(String),
    #[error("requête refusée : {0}")]
    Invalide(String),
    #[error("conflit : {0}")]
    Conflit(String),
    #[error("trop de tentatives, réessayez plus tard")]
    TropDeRequetes,
    #[error("erreur serveur ({0}) : {1}")]
    Serveur(u16, String),
    #[error("serveur injoignable : {0}")]
    Reseau(String),
    #[error("réponse illisible : {0}")]
    Decodage(String),
    #[error("appel réseau indisponible dans ce contexte")]
    Indisponible,
}

impl ApiError {
    /// Message destiné à l'utilisateur (français, sans détail technique inutile).
    pub fn message_utilisateur(&self) -> String {
        match self {
            ApiError::NonConnecte => "Votre session a expiré. Reconnectez-vous.".into(),
            ApiError::Interdit(_) => {
                "Vous n'avez pas les droits nécessaires pour cette action.".into()
            }
            ApiError::Introuvable(_) => "Élément introuvable.".into(),
            ApiError::Invalide(m) | ApiError::Conflit(m) => format!("Action refusée : {m}."),
            ApiError::TropDeRequetes => "Trop de tentatives. Patientez avant de réessayer.".into(),
            ApiError::Serveur(..) => "Le serveur a rencontré une erreur. Réessayez.".into(),
            ApiError::Reseau(_) => {
                "Connexion au serveur impossible. Vérifiez le réseau puis réessayez.".into()
            }
            ApiError::Decodage(_) => "Réponse inattendue du serveur.".into(),
            ApiError::Indisponible => "Fonction indisponible ici.".into(),
        }
    }

    /// Traduit un statut HTTP + corps texte (convention d'ent-api : `(StatusCode, String)`).
    pub fn depuis_statut(statut: u16, corps: String) -> ApiError {
        match statut {
            401 => ApiError::NonConnecte,
            403 => ApiError::Interdit(corps),
            404 => ApiError::Introuvable(corps),
            400 | 422 => ApiError::Invalide(corps),
            409 => ApiError::Conflit(corps),
            429 => ApiError::TropDeRequetes,
            s => ApiError::Serveur(s, corps),
        }
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client HTTP. `base` vide = même origine (navigateur).
#[derive(Clone, Debug, Default)]
pub struct ApiClient {
    base: String,
    jeton: Option<String>,
}

pub(crate) enum Methode {
    Get,
    Post,
}

impl ApiClient {
    pub fn new(base: impl Into<String>, jeton: Option<String>) -> Self {
        Self {
            base: base.into(),
            jeton,
        }
    }

    pub fn avec_jeton(&self, jeton: Option<String>) -> Self {
        Self {
            base: self.base.clone(),
            jeton,
        }
    }

    pub fn a_un_jeton(&self) -> bool {
        self.jeton.is_some()
    }

    fn url(&self, chemin: &str) -> String {
        format!("{}{}", self.base, chemin)
    }

    async fn appel<T: serde::de::DeserializeOwned>(
        &self,
        methode: Methode,
        chemin: &str,
        corps: Option<serde_json::Value>,
    ) -> Result<T, ApiError> {
        let (statut, texte) =
            transport::envoyer(methode, &self.url(chemin), self.jeton.as_deref(), corps).await?;
        if (200..300).contains(&statut) {
            let texte = if texte.trim().is_empty() {
                "null".to_string()
            } else {
                texte
            };
            serde_json::from_str(&texte).map_err(|e| ApiError::Decodage(e.to_string()))
        } else {
            Err(ApiError::depuis_statut(statut, texte))
        }
    }

    /// `POST /auth/login` (mot de passe). Comptes de labo sans passkey uniquement.
    pub async fn connexion(
        &self,
        email: &str,
        mot_de_passe: &str,
        etablissement_id: Uuid,
    ) -> Result<Connexion, ApiError> {
        let corps = serde_json::json!({
            "email": email,
            "password": mot_de_passe,
            "etablissement_id": etablissement_id,
        });
        let r: LoginResponseDto = self
            .appel(Methode::Post, "/auth/login", Some(corps))
            .await?;
        match (r.mfa_required, r.session) {
            (false, Some(s)) => Ok(Connexion::Ouverte(s)),
            (true, _) => Ok(Connexion::PasskeyRequise),
            (false, None) => Err(ApiError::Decodage("session absente".into())),
        }
    }

    /// `POST /auth/logout`
    pub async fn deconnexion(&self) -> Result<(), ApiError> {
        let _: serde_json::Value = self.appel(Methode::Post, "/auth/logout", None).await?;
        Ok(())
    }

    /// `GET /auth/me`
    pub async fn moi(&self) -> Result<Moi, ApiError> {
        self.appel(Methode::Get, "/auth/me", None).await
    }

    /// `GET /edt/mes-creneaux?jour=AAAA-MM-JJ`
    pub async fn mes_creneaux(&self, jour: NaiveDate) -> Result<Vec<Creneau>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/edt/mes-creneaux?jour={jour}"),
            None,
        )
        .await
    }

    /// `POST /appels` — ouvre (ou retrouve) l'appel du créneau ce jour-là.
    pub async fn ouvrir_appel(
        &self,
        creneau_id: Uuid,
        jour: NaiveDate,
    ) -> Result<AppelApi, ApiError> {
        let corps = serde_json::json!({ "creneau_id": creneau_id, "jour": jour });
        self.appel(Methode::Post, "/appels", Some(corps)).await
    }

    /// `GET /appels/{id}`
    pub async fn appel_par_id(&self, appel_id: Uuid) -> Result<AppelApi, ApiError> {
        self.appel(Methode::Get, &format!("/appels/{appel_id}"), None)
            .await
    }

    /// `POST /appels/{id}/lignes`
    pub async fn marquer(
        &self,
        appel_id: Uuid,
        eleve_id: Uuid,
        etat: EtatApi,
    ) -> Result<(), ApiError> {
        let corps = serde_json::json!({ "eleve_id": eleve_id, "etat": etat.as_str() });
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                &format!("/appels/{appel_id}/lignes"),
                Some(corps),
            )
            .await?;
        Ok(())
    }

    /// `POST /appels/{id}/clore` — l'appel devient définitif ; les absences
    /// partent vers la vie scolaire (réaction métier côté serveur).
    pub async fn clore(&self, appel_id: Uuid) -> Result<AppelApi, ApiError> {
        self.appel(Methode::Post, &format!("/appels/{appel_id}/clore"), None)
            .await
    }

    /// Accès générique (tests et futurs modules) : `GET` d'un chemin JSON.
    pub async fn get_json(&self, chemin: &str) -> Result<serde_json::Value, ApiError> {
        self.appel(Methode::Get, chemin, None).await
    }

    /// Accès générique : `POST` JSON.
    pub async fn post_json(
        &self,
        chemin: &str,
        corps: serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        self.appel(Methode::Post, chemin, Some(corps)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuts_http_vers_erreurs() {
        assert_eq!(
            ApiError::depuis_statut(401, "x".into()),
            ApiError::NonConnecte
        );
        assert!(matches!(
            ApiError::depuis_statut(403, "x".into()),
            ApiError::Interdit(_)
        ));
        assert!(matches!(
            ApiError::depuis_statut(400, "x".into()),
            ApiError::Invalide(_)
        ));
        assert!(matches!(
            ApiError::depuis_statut(503, "x".into()),
            ApiError::Serveur(503, _)
        ));
    }

    #[test]
    fn decode_appel_du_contrat_ent_api() {
        let json = r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","classe":"4B",
            "jour":"2026-11-02","statut":"ouvert","creneau_id":null,
            "matiere_libelle":"Potions","lignes":[{"eleve_id":"6f1c3c80-0000-4000-8000-000000000002",
            "eleve_label":"POTTER Harry","etat":"present"}]}"#;
        let a: AppelApi = serde_json::from_str(json).unwrap();
        assert_eq!(a.lignes.len(), 1);
        assert!(!a.est_clos());
    }

    #[test]
    fn decode_connexion_aplatie() {
        let json = r#"{"mfa_required":false,"token":"t","user_id":"6f1c3c80-0000-4000-8000-000000000001",
            "email":"a@b","etablissement_id":"6f1c3c80-0000-4000-8000-000000000002","role":"prof",
            "expires_at":"2026-11-03T00:00:00Z"}"#;
        let r: LoginResponseDto = serde_json::from_str(json).unwrap();
        assert!(r.session.is_some());
        let json2 = r#"{"mfa_required":true,"passkey":{"token":"x","options":{},"expires_at":"2026-11-03T00:00:00Z"}}"#;
        let r2: LoginResponseDto = serde_json::from_str(json2).unwrap();
        assert!(r2.mfa_required);
    }
}
