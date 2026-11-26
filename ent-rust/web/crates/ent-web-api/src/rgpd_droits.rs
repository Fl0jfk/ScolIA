//! Socle 27, lot G3a : guichet des demandes de droits. Routes `/rgpd/*` d'`ent-api` ; types
//! d'`ent-types::rgpd_droits`.
//!
//! Le dépôt **sans compte** (`rgpd_deposer_public`) est appelé depuis une page publique, sans session.

use ent_types::rgpd_droits::*;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /rgpd/guichet/referentiel` : vocabulaire, grille d'identité, modèles de réponse.
    pub async fn rgpd_referentiel(&self) -> Result<ReferentielGuichetDto, ApiError> {
        self.appel(Methode::Get, "/rgpd/guichet/referentiel", None)
            .await
    }

    /// `GET /rgpd/demandes` : la file du guichet (métadonnées seulement).
    pub async fn rgpd_file(&self, closes: bool) -> Result<TableauDemandesDto, ApiError> {
        let uri = if closes {
            "/rgpd/demandes?closes=true"
        } else {
            "/rgpd/demandes"
        };
        self.appel(Methode::Get, uri, None).await
    }

    /// `GET /rgpd/demandes/{id}` : la fiche, journal compris.
    pub async fn rgpd_demande(&self, id: Uuid) -> Result<DemandeDto, ApiError> {
        self.appel(Methode::Get, &format!("/rgpd/demandes/{id}"), None)
            .await
    }

    /// `POST /rgpd/demandes` : saisie d'une demande reçue par courrier, courriel ou au guichet.
    pub async fn rgpd_saisir(&self, b: &SaisieBody) -> Result<DemandeDto, ApiError> {
        self.appel(Methode::Post, "/rgpd/demandes", corps(b)?).await
    }

    /// `POST /rgpd/demandes/{id}/qualification`.
    pub async fn rgpd_qualifier(
        &self,
        id: Uuid,
        b: &QualificationBody,
    ) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/qualification"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/identite` : vérification tracée (la pièce est détruite à la clôture).
    pub async fn rgpd_verifier_identite(
        &self,
        id: Uuid,
        b: &IdentiteBody,
    ) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/identite"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/identite/precision`.
    pub async fn rgpd_demander_identite(
        &self,
        id: Uuid,
        b: &PrecisionIdentiteBody,
    ) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/identite/precision"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/affectation`.
    pub async fn rgpd_affecter(
        &self,
        id: Uuid,
        b: &AffectationBody,
    ) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/affectation"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/prolongation` (direction).
    pub async fn rgpd_prolonger(
        &self,
        id: Uuid,
        b: &ProlongationBody,
    ) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/prolongation"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/droits`.
    pub async fn rgpd_ajouter_droit(
        &self,
        id: Uuid,
        b: &DroitBody,
    ) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/droits"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/decision`.
    pub async fn rgpd_decider(&self, id: Uuid, b: &DecisionBody) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/decision"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/titulaires` (élève mineur : information des titulaires).
    pub async fn rgpd_titulaires(
        &self,
        id: Uuid,
        b: &TitulairesBody,
    ) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/titulaires"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/cloture`.
    pub async fn rgpd_clore(&self, id: Uuid, b: &ClotureBody) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/cloture"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/refus` (direction).
    pub async fn rgpd_refuser(&self, id: Uuid, b: &MotifBody) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/refus"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/reouverture` (direction).
    pub async fn rgpd_rouvrir(&self, id: Uuid, b: &MotifBody) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/reouverture"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/identite/arbitrage` : la direction tranche une qualité non compatible.
    pub async fn rgpd_arbitrer_identite(
        &self,
        id: Uuid,
        b: &ArbitrageIdentiteBody,
    ) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/identite/arbitrage"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/{id}/rapprochement` : rapproche le sujet et/ou la fiche du demandeur.
    pub async fn rgpd_rapprocher(
        &self,
        id: Uuid,
        b: &RapprochementBody,
    ) -> Result<DemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/demandes/{id}/rapprochement"),
            corps(b)?,
        )
        .await
    }

    /// `POST /rgpd/demandes/sans-compte` : dépôt **public**, sans session. La réponse est invariable.
    pub async fn rgpd_deposer_public(&self, b: &DepotPublicBody) -> Result<DepotPublicDto, ApiError> {
        self.appel(Methode::Post, "/rgpd/demandes/sans-compte", corps(b)?)
            .await
    }

    /// `POST /rgpd/mes-demandes` : dépôt par le demandeur connecté.
    pub async fn rgpd_deposer(&self, b: &DepotBody) -> Result<DemandeDto, ApiError> {
        self.appel(Methode::Post, "/rgpd/mes-demandes", corps(b)?)
            .await
    }

    /// `GET /rgpd/mes-demandes` : suivi de ses propres demandes.
    pub async fn rgpd_mes_demandes(&self) -> Result<Vec<SuiviDemandeDto>, ApiError> {
        self.appel(Methode::Get, "/rgpd/mes-demandes", None).await
    }
}
