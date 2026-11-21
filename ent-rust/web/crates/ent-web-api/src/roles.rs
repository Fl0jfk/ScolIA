//! Rôles paramétrables (D-017) et récupération de passkey (D-018) : routes `/roles/*`,
//! `/referentiel/groupes/{id}/responsables`, `/personnel/search`, `/auth/passkey/recuperation/*`
//! d'`ent-api`, inchangées. Le serveur reste l'autorité (droit `roles.gerer`, anti-escalade).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

/// Action du catalogue (`GET /roles/catalogue`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ActionCatalogue {
    pub code: String,
    pub libelle: String,
    /// Portées admises pour cette action (les actions d'administration : établissement seul).
    pub portees: Vec<String>,
}

/// Portée du catalogue.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PorteeCatalogue {
    pub code: String,
    pub libelle: String,
}

/// Réponse de `GET /roles/catalogue`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct CatalogueRoles {
    pub actions: Vec<ActionCatalogue>,
    pub portees: Vec<PorteeCatalogue>,
}

/// Droit d'un rôle : action × portée (+ classes ou groupes pour la portée « désignés »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroitApi {
    pub action: String,
    pub portee: String,
    #[serde(default)]
    pub cibles: Vec<Uuid>,
}

/// Rôle de l'établissement (`GET /roles`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct RoleApi {
    pub id: Uuid,
    pub nom: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Profil historique dont le rôle est issu (informatif).
    #[serde(default)]
    pub modele: Option<String>,
    #[serde(default)]
    pub droits: Vec<DroitApi>,
    /// Personnes à qui le rôle est attribué.
    #[serde(default)]
    pub membres: Vec<Uuid>,
}

/// Corps de `POST /roles` et `PUT /roles/{id}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SaisieRole {
    pub nom: String,
    pub description: Option<String>,
    pub modele: Option<String>,
    pub droits: Vec<DroitApi>,
}

/// Membre du personnel (`GET /personnel/search`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PersonnelApi {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    pub label: String,
}

/// Fin du délai de sécurité renvoyée à la confirmation d'une récupération.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ConfirmationRecuperation {
    pub effective_at: DateTime<Utc>,
}

impl ApiClient {
    /// `GET /roles/catalogue` : actions et portées (tout le personnel).
    pub async fn catalogue_roles(&self) -> Result<CatalogueRoles, ApiError> {
        self.appel(Methode::Get, "/roles/catalogue", None).await
    }

    /// `GET /roles` : rôles de l'établissement avec leurs droits et membres.
    pub async fn roles(&self) -> Result<Vec<RoleApi>, ApiError> {
        self.appel(Methode::Get, "/roles", None).await
    }

    /// `GET /roles/mes-droits` : droits effectifs de la personne connectée.
    pub async fn mes_droits(&self) -> Result<Vec<DroitApi>, ApiError> {
        self.appel(Methode::Get, "/roles/mes-droits", None).await
    }

    /// `POST /roles`.
    pub async fn creer_role(&self, saisie: &SaisieRole) -> Result<RoleApi, ApiError> {
        let corps = serde_json::to_value(saisie).map_err(|e| ApiError::Decodage(e.to_string()))?;
        self.appel(Methode::Post, "/roles", Some(corps)).await
    }

    /// `PUT /roles/{id}`.
    pub async fn modifier_role(&self, id: Uuid, saisie: &SaisieRole) -> Result<RoleApi, ApiError> {
        let corps = serde_json::to_value(saisie).map_err(|e| ApiError::Decodage(e.to_string()))?;
        self.appel(Methode::Put, &format!("/roles/{id}"), Some(corps))
            .await
    }

    /// `DELETE /roles/{id}`.
    pub async fn supprimer_role(&self, id: Uuid) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(Methode::Delete, &format!("/roles/{id}"), None)
            .await?;
        Ok(())
    }

    /// `PUT /roles/{id}/membres/{user_id}` avec une date de fin facultative.
    pub async fn attribuer_role(
        &self,
        id: Uuid,
        user_id: Uuid,
        fin: Option<NaiveDate>,
    ) -> Result<(), ApiError> {
        let corps = serde_json::json!({ "fin": fin });
        let _: serde_json::Value = self
            .appel(
                Methode::Put,
                &format!("/roles/{id}/membres/{user_id}"),
                Some(corps),
            )
            .await?;
        Ok(())
    }

    /// `DELETE /roles/{id}/membres/{user_id}`.
    pub async fn retirer_role(&self, id: Uuid, user_id: Uuid) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Delete,
                &format!("/roles/{id}/membres/{user_id}"),
                None,
            )
            .await?;
        Ok(())
    }

    /// `PUT /referentiel/groupes/{id}/responsables` : **remplace** la liste des responsables.
    pub async fn definir_responsables_groupe(
        &self,
        groupe_id: Uuid,
        responsables: &[Uuid],
    ) -> Result<Vec<Uuid>, ApiError> {
        let corps = serde_json::json!({ "responsables": responsables });
        self.appel(
            Methode::Put,
            &format!("/referentiel/groupes/{groupe_id}/responsables"),
            Some(corps),
        )
        .await
    }

    /// `GET /personnel/search?q=` : personnel de l'établissement (100 résultats au plus).
    pub async fn personnel(&self, recherche: &str) -> Result<Vec<PersonnelApi>, ApiError> {
        let q = crate::encoder_requete(recherche);
        self.appel(Methode::Get, &format!("/personnel/search?q={q}"), None)
            .await
    }

    // --- Récupération de passkey (D-018), routes publiques -----------------------------

    /// `POST /auth/passkey/recuperation` : réponse **toujours** 202 (anti-énumération).
    pub async fn demander_recuperation_passkey(
        &self,
        email: &str,
        etablissement_id: Uuid,
    ) -> Result<(), ApiError> {
        let corps = serde_json::json!({ "email": email, "etablissement_id": etablissement_id });
        let _: serde_json::Value = self
            .appel(Methode::Post, "/auth/passkey/recuperation", Some(corps))
            .await?;
        Ok(())
    }

    /// `POST /auth/passkey/recuperation/confirmer` : démarre le délai de sécurité.
    pub async fn confirmer_recuperation_passkey(
        &self,
        jeton: &str,
    ) -> Result<ConfirmationRecuperation, ApiError> {
        let corps = serde_json::json!({ "token": jeton });
        self.appel(
            Methode::Post,
            "/auth/passkey/recuperation/confirmer",
            Some(corps),
        )
        .await
    }

    /// `POST /auth/passkey/recuperation/annuler`.
    pub async fn annuler_recuperation_passkey(&self, jeton: &str) -> Result<(), ApiError> {
        let corps = serde_json::json!({ "token": jeton });
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                "/auth/passkey/recuperation/annuler",
                Some(corps),
            )
            .await?;
        Ok(())
    }

    /// `POST /auth/passkey/recuperation/finaliser` : après le délai (409 avant).
    pub async fn finaliser_recuperation_passkey(&self, jeton: &str) -> Result<(), ApiError> {
        let corps = serde_json::json!({ "token": jeton });
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                "/auth/passkey/recuperation/finaliser",
                Some(corps),
            )
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_catalogue_et_roles() {
        let c: CatalogueRoles = serde_json::from_str(
            r#"{"actions":[{"code":"notes.lire","libelle":"Lire les notes",
            "portees":["etablissement","mes_classes"]}],
            "portees":[{"code":"etablissement","libelle":"Tout l'établissement"}]}"#,
        )
        .unwrap();
        assert_eq!(c.actions[0].portees.len(), 2);
        let r: Vec<RoleApi> = serde_json::from_str(
            r#"[{"id":"6f1c3c80-0000-4000-8000-000000000001","nom":"Théâtre",
            "description":null,"modele":null,
            "droits":[{"action":"absences.lire","portee":"mes_groupes","cibles":[]}],
            "membres":["6f1c3c80-0000-4000-8000-000000000009"]}]"#,
        )
        .unwrap();
        assert_eq!(r[0].droits[0].portee, "mes_groupes");
        assert_eq!(r[0].membres.len(), 1);
    }

    #[test]
    fn droit_sans_cibles_decode_et_serialise() {
        let d: DroitApi = serde_json::from_str(r#"{"action":"a","portee":"moi"}"#).unwrap();
        assert!(d.cibles.is_empty());
        let s = serde_json::to_value(&d).unwrap();
        assert_eq!(s["cibles"], serde_json::json!([]));
    }

    #[test]
    fn saisie_serialise_au_contrat_du_serveur() {
        let s = SaisieRole {
            nom: "Accueil".into(),
            description: None,
            modele: None,
            droits: vec![DroitApi {
                action: "absences.lire".into(),
                portee: "etablissement".into(),
                cibles: vec![],
            }],
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["nom"], "Accueil");
        assert!(v["description"].is_null());
        assert_eq!(v["droits"][0]["portee"], "etablissement");
    }

    #[test]
    fn decode_confirmation_et_personnel() {
        let c: ConfirmationRecuperation =
            serde_json::from_str(r#"{"effective_at":"2026-11-06T09:00:00Z"}"#).unwrap();
        assert_eq!(c.effective_at.to_rfc3339(), "2026-11-06T09:00:00+00:00");
        let p: Vec<PersonnelApi> = serde_json::from_str(
            r#"[{"user_id":"6f1c3c80-0000-4000-8000-000000000001","email":"a@b.fr",
            "role":"prof","label":"A B","telephone":null}]"#,
        )
        .unwrap();
        assert_eq!(p[0].label, "A B");
    }
}
