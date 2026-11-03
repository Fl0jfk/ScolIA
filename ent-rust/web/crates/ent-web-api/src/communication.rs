//! Service communication (lecture seule) : fils de messages et annonces.
//!
//! Types de lecture : seulement les champs utilisés par l'accueil (les autres
//! champs du JSON d'`ent-api` sont ignorés). À déplacer dans `ent-types` lors du
//! lot qui migrera ces DTO côté `ent-api` (décision séparée, D-016).

use serde::Deserialize;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

/// Résumé d'un fil de messages (`MsgFilDto` d'ent-api, champs utiles).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FilMessages {
    pub id: Uuid,
    #[serde(default)]
    pub eleve_label: Option<String>,
    pub sujet: String,
    pub last_message_at: String,
    #[serde(default)]
    pub archive: bool,
    #[serde(default)]
    pub nb_non_lus: i64,
}

/// Annonce d'établissement (`AnnonceDto` d'ent-api, champs utiles).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Annonce {
    pub id: Uuid,
    pub titre: String,
    pub corps: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ApiClient {
    /// `GET /messages/fils` : fils visibles par la personne connectée (filtrés par le serveur).
    pub async fn fils_messages(&self) -> Result<Vec<FilMessages>, ApiError> {
        self.appel(Methode::Get, "/messages/fils", None).await
    }

    /// `GET /annonces` : annonces de l'établissement visibles selon le rôle.
    pub async fn annonces(&self) -> Result<Vec<Annonce>, ApiError> {
        self.appel(Methode::Get, "/annonces", None).await
    }
}
