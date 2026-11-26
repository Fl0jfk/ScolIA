//! Socle 27, lot G3b : extrait d'accès. Routes `/rgpd/demandes/{id}/extraits/*` et `/rgpd/reglages` d'`ent-api` ;
//! types d'`ent-types::rgpd_extrait`.

use ent_types::rgpd_extrait::*;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /rgpd/demandes/{id}/extraits` : structure des extraits, sans contenu.
    pub async fn rgpd_extraits(&self, demande: Uuid) -> Result<Vec<ExtraitDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/rgpd/demandes/{demande}/extraits"),
            None,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/extraits` : assemble une nouvelle version.
    pub async fn rgpd_extrait_assembler(
        &self,
        demande: Uuid,
        b: &AssemblerBody,
    ) -> Result<Vec<ExtraitDto>, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{demande}/extraits"),
            corps(b)?,
        )
        .await
    }

    /// `GET …/extraits/{extrait}/contenu` : lecture tracée du contenu, pour la relecture.
    pub async fn rgpd_extrait_contenu(
        &self,
        demande: Uuid,
        extrait: Uuid,
    ) -> Result<ContenuExtraitDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/rgpd/demandes/{demande}/extraits/{extrait}/contenu"),
            None,
        )
        .await
    }

    /// `POST …/masquages/{id}` : accepte (`masque`) ou lève (`leve`, motif) un marquage.
    pub async fn rgpd_masquage_decider(
        &self,
        demande: Uuid,
        masquage: Uuid,
        b: &DecisionMasquageBody,
    ) -> Result<Vec<MarquageDto>, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{demande}/masquages/{masquage}"),
            corps(b)?,
        )
        .await
    }

    /// `POST …/extraits/{extrait}/dpo` : soumet l'extrait au DPO.
    pub async fn rgpd_extrait_soumettre_dpo(
        &self,
        demande: Uuid,
        extrait: Uuid,
    ) -> Result<ExtraitDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{demande}/extraits/{extrait}/dpo"),
            Some(serde_json::json!({})),
        )
        .await
    }
}
