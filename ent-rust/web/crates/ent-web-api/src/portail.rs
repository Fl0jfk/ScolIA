//! Portail (socle 29, lot P1) : la page « Aujourd'hui » composée par le serveur.
//! Route d'`ent-api/src/portail.rs` ; les droits sont appliqués par le serveur, qui ne renvoie
//! que les tuiles permises (jamais de tuile 403).

use ent_types::portail::PortailAujourdhui;

use crate::{ApiClient, ApiError, Methode};

impl ApiClient {
    /// `GET /portail/aujourdhui` : l'accueil de la personne connectée, en une requête.
    pub async fn portail_aujourdhui(&self) -> Result<PortailAujourdhui, ApiError> {
        self.appel(Methode::Get, "/portail/aujourdhui", None).await
    }
}
