//! Sécurité du compte (socle 1) : portée et niveau de la session, passkeys, appareils
//! connectés, politique de sécurité de l'établissement. Routes d'`ent-api`, inchangées.
//!
//! `ent-types::auth::MeDto` ne porte pas encore `portee`, `assurance`, `client` ni
//! `passkey_obligatoire` (types partagés non modifiés dans cette tranche) : [`MoiSecurite`]
//! décode la réponse complète de `GET /auth/me` ; les champs absents ont une valeur sûre.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode, Moi};

/// Corps du 403 renvoyé à une session « enrôlement » hors des routes autorisées.
pub const ENROLEMENT_REQUIS: &str = "passkey_enrolement_requis";

fn portee_par_defaut() -> String {
    "complete".into()
}

fn assurance_par_defaut() -> String {
    "mot_de_passe".into()
}

fn casquette_par_defaut() -> String {
    "principale".into()
}

fn client_par_defaut() -> String {
    "web".into()
}

/// Réponse complète de `GET /auth/me`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct MoiSecurite {
    pub user_id: Uuid,
    pub email: String,
    pub etablissement_id: Uuid,
    pub role: String,
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
    #[serde(default)]
    pub passkey_count: i64,
    /// `complete` ou `enrolement` (passkey obligatoire à enrôler avant tout).
    #[serde(default = "portee_par_defaut")]
    pub portee: String,
    /// `mot_de_passe`, `passkey` ou `sso`.
    #[serde(default = "assurance_par_defaut")]
    pub assurance: String,
    /// `web` ou `mobile`.
    #[serde(default = "client_par_defaut")]
    pub client: String,
    /// La politique de l'établissement impose la passkey à ce rôle.
    #[serde(default)]
    pub passkey_obligatoire: bool,
    /// Casquette active (`principale` ou `parent`).
    #[serde(default = "casquette_par_defaut")]
    pub casquette: String,
}

impl MoiSecurite {
    /// Identité au format historique (`ent-types`).
    pub fn identite(&self) -> Moi {
        Moi {
            user_id: self.user_id,
            email: self.email.clone(),
            etablissement_id: self.etablissement_id,
            role: self.role.clone(),
            eleve_id: self.eleve_id,
            passkey_count: self.passkey_count,
            portee: self.portee.clone(),
            assurance: self.assurance.clone(),
            client: self.client.clone(),
            passkey_obligatoire: self.passkey_obligatoire,
            casquette: self.casquette.clone(),
        }
    }
}

/// Passkey du compte (`GET /auth/passkeys`) : jamais la clé publique.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PasskeyCompte {
    pub id: Uuid,
    #[serde(default)]
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Session d'un appareil (`GET /auth/sessions`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AppareilSession {
    pub id: Uuid,
    pub client: String,
    pub assurance: String,
    #[serde(default)]
    pub appareil: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// La session qui fait la requête.
    pub courante: bool,
}

/// Politique de sécurité de l'établissement.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolitiqueSecurite {
    /// Rôles pour lesquels la passkey est obligatoire.
    pub passkey_obligatoire_roles: Vec<String>,
}

/// Défi d'enrôlement renvoyé par `POST /auth/passkey/register/start`.
#[derive(Clone, Deserialize)]
pub struct ChallengeEnrolement {
    pub token: String,
    pub options: serde_json::Value,
}

// `Debug` manuel : le défi est à usage unique mais ne doit pas finir dans un journal.
impl std::fmt::Debug for ChallengeEnrolement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChallengeEnrolement")
            .field("token", &"<masqué>")
            .finish_non_exhaustive()
    }
}

/// Temps restant de la session web, lu en base par le relais (`GET /web/session/etat`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct EtatSession {
    /// Secondes avant l'expiration **absolue** (durée maximale, D-011).
    pub expire_dans: u64,
    /// Inactivité maximale en secondes (D-011 : 2 h) : la session est refusée après ce délai
    /// sans requête authentifiée.
    pub inactivite_max: u64,
}

impl EtatSession {
    /// Secondes avant la perte de la session si la personne n'agit plus : le plus proche de
    /// l'expiration absolue et de la fin de l'inactivité.
    pub fn reste_si_inactif(&self) -> u64 {
        self.expire_dans.min(self.inactivite_max)
    }
}

impl ApiClient {
    /// `GET /web/session/etat` (relais seulement). Compte comme une activité : à n'appeler
    /// qu'au chargement et à la demande, jamais en boucle.
    pub async fn etat_session(&self) -> Result<EtatSession, ApiError> {
        self.appel_adresse(Methode::Get, "/web/session/etat", None)
            .await
    }

    /// `GET /auth/me` avec la portée et le niveau d'assurance de la session.
    pub async fn moi_securite(&self) -> Result<MoiSecurite, ApiError> {
        self.appel(Methode::Get, "/auth/me", None).await
    }

    /// `GET /auth/passkeys`.
    pub async fn passkeys(&self) -> Result<Vec<PasskeyCompte>, ApiError> {
        self.appel(Methode::Get, "/auth/passkeys", None).await
    }

    /// `POST /auth/passkey/register/start` : défi d'enrôlement pour la session courante.
    pub async fn passkey_enrolement_debut(&self) -> Result<ChallengeEnrolement, ApiError> {
        self.appel(Methode::Post, "/auth/passkey/register/start", None)
            .await
    }

    /// `POST /auth/passkey/register/finish`. `mot_de_passe` : vide en session « enrôlement »
    /// (le serveur l'accepte alors), exigé sinon.
    pub async fn passkey_enrolement_fin(
        &self,
        jeton_defi: &str,
        credential: serde_json::Value,
        nom: Option<&str>,
        mot_de_passe: &str,
    ) -> Result<PasskeyCompte, ApiError> {
        let corps = serde_json::json!({
            "token": jeton_defi,
            "credential": credential,
            "name": nom.map(str::trim).filter(|n| !n.is_empty()),
            "current_password": mot_de_passe,
        });
        self.appel(Methode::Post, "/auth/passkey/register/finish", Some(corps))
            .await
    }

    /// `DELETE /auth/passkeys/{id}` avec le mot de passe actuel.
    pub async fn supprimer_passkey(&self, id: Uuid, mot_de_passe: &str) -> Result<(), ApiError> {
        let corps = serde_json::json!({ "current_password": mot_de_passe });
        let _: serde_json::Value = self
            .appel(
                Methode::Delete,
                &format!("/auth/passkeys/{id}"),
                Some(corps),
            )
            .await?;
        Ok(())
    }

    /// `GET /auth/sessions` : appareils connectés dans cet établissement.
    pub async fn appareils(&self) -> Result<Vec<AppareilSession>, ApiError> {
        self.appel(Methode::Get, "/auth/sessions", None).await
    }

    /// `DELETE /auth/sessions/{id}` : déconnecte un appareil.
    pub async fn deconnecter_appareil(&self, id: Uuid) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(Methode::Delete, &format!("/auth/sessions/{id}"), None)
            .await?;
        Ok(())
    }

    /// `GET /etablissement/securite` (Admin, Direction).
    pub async fn politique_securite(&self) -> Result<PolitiqueSecurite, ApiError> {
        self.appel(Methode::Get, "/etablissement/securite", None)
            .await
    }

    /// `PUT /etablissement/securite` (Admin, Direction).
    pub async fn definir_politique_securite(
        &self,
        roles: &[String],
    ) -> Result<PolitiqueSecurite, ApiError> {
        let corps = serde_json::json!({ "passkey_obligatoire_roles": roles });
        self.appel(Methode::Put, "/etablissement/securite", Some(corps))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_me_complet() {
        let json = r#"{"user_id":"6f1c3c80-0000-4000-8000-000000000001","email":"a@b",
            "etablissement_id":"6f1c3c80-0000-4000-8000-000000000002","role":"prof",
            "passkey_count":0,"portee":"enrolement","assurance":"mot_de_passe",
            "client":"web","passkey_obligatoire":true}"#;
        let m: MoiSecurite = serde_json::from_str(json).unwrap();
        assert_eq!(m.portee, "enrolement");
        assert!(m.passkey_obligatoire);
        assert_eq!(m.identite().role, "prof");
    }

    #[test]
    fn decode_me_ancien_contrat_valeurs_sures() {
        // Ancien serveur : aucun des champs du socle 1 → session complète, pas d'obligation.
        let json = r#"{"user_id":"6f1c3c80-0000-4000-8000-000000000001","email":"a@b",
            "etablissement_id":"6f1c3c80-0000-4000-8000-000000000002","role":"eleve",
            "passkey_count":2}"#;
        let m: MoiSecurite = serde_json::from_str(json).unwrap();
        assert_eq!(m.portee, "complete");
        assert_eq!(m.assurance, "mot_de_passe");
        assert_eq!(m.client, "web");
        assert!(!m.passkey_obligatoire);
        assert_eq!(m.passkey_count, 2);
    }

    #[test]
    fn decode_appareils_passkeys_politique() {
        let a: Vec<AppareilSession> = serde_json::from_str(
            r#"[{"id":"6f1c3c80-0000-4000-8000-000000000001","client":"web","assurance":"passkey",
            "appareil":null,"created_at":"2026-11-05T08:00:00Z","last_seen_at":"2026-11-05T09:00:00Z",
            "expires_at":"2026-11-05T20:00:00Z","courante":true}]"#,
        )
        .unwrap();
        assert!(a[0].courante && a[0].appareil.is_none());
        let p: Vec<PasskeyCompte> = serde_json::from_str(
            r#"[{"id":"6f1c3c80-0000-4000-8000-000000000001","name":null,
            "created_at":"2026-11-05T08:00:00Z","last_used_at":null}]"#,
        )
        .unwrap();
        assert!(p[0].name.is_none() && p[0].last_used_at.is_none());
        let pol: PolitiqueSecurite =
            serde_json::from_str(r#"{"passkey_obligatoire_roles":["prof","cpe"]}"#).unwrap();
        assert_eq!(pol.passkey_obligatoire_roles, ["prof", "cpe"]);
    }

    #[test]
    fn le_defi_n_apparait_pas_dans_debug() {
        let c = ChallengeEnrolement {
            token: "DEFI-SECRET".into(),
            options: serde_json::json!({}),
        };
        assert!(!format!("{c:?}").contains("DEFI-SECRET"));
    }
}
