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

    // ───────────── Remise, « Mes données », renvois (lot G3b-3) ─────────────

    /// `POST …/extraits/{extrait}/remise/defi` : défi de passkey récente du gestionnaire.
    pub async fn rgpd_remise_defi(
        &self,
        demande: Uuid,
        extrait: Uuid,
    ) -> Result<DefiDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{demande}/extraits/{extrait}/remise/defi"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `POST …/extraits/{extrait}/remise` : remet l'extrait (en ligne ou en main propre).
    pub async fn rgpd_remettre(
        &self,
        demande: Uuid,
        extrait: Uuid,
        b: &RemettreBody,
    ) -> Result<RemiseDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{demande}/extraits/{extrait}/remise"),
            corps(b)?,
        )
        .await
    }

    /// `GET /rgpd/mes-donnees` : les extraits remis en ligne au compte connecté.
    pub async fn rgpd_mes_extraits(&self) -> Result<Vec<MonExtraitDto>, ApiError> {
        self.appel(Methode::Get, "/rgpd/mes-donnees", None).await
    }

    /// `POST /rgpd/mes-donnees/extraits/{extrait}/defi` : défi de passkey du demandeur.
    pub async fn rgpd_telechargement_defi(&self, extrait: Uuid) -> Result<DefiDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/mes-donnees/extraits/{extrait}/defi"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `POST /rgpd/mes-donnees/fichiers/{fichier}/telecharger` : le fichier, après la passkey du demandeur.
    pub async fn rgpd_telecharger(
        &self,
        fichier: Uuid,
        b: &TelechargerBody,
    ) -> Result<FichierTelechargeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/mes-donnees/fichiers/{fichier}/telecharger"),
            corps(b)?,
        )
        .await
    }

    /// `GET …/renvois` et `POST …/renvois` : renvois tracés d'une demande.
    pub async fn rgpd_renvois(&self, demande: Uuid) -> Result<Vec<RenvoiDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/rgpd/demandes/{demande}/renvois"),
            None,
        )
        .await
    }

    pub async fn rgpd_renvoi_tracer(
        &self,
        demande: Uuid,
        b: &RenvoiBody,
    ) -> Result<RenvoiDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{demande}/renvois"),
            corps(b)?,
        )
        .await
    }

    /// `GET …/pieces-identite` : pièces d'identité du coffre à choisir (Pf7).
    pub async fn rgpd_pieces_identite(
        &self,
        demande: Uuid,
    ) -> Result<Vec<PieceChoisissableDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/rgpd/demandes/{demande}/pieces-identite"),
            None,
        )
        .await
    }
}
