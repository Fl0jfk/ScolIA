//! Socle 27, lot G4-1 : traces d'accès. Routes `/acces/*` d'`ent-api` ; types d'`ent-types::acces_traces`.

use ent_types::acces_traces::*;
use ent_types::eleve_synthese::EleveFicheDto;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /acces/consultations` : « Consultations du dossier » de la personne connectée (famille, élève majeur,
    /// agent) : fonction et service, jamais le nom de l'agent.
    pub async fn acces_consultations(&self) -> Result<ConsultationsDossierDto, ApiError> {
        self.appel(Methode::Get, "/acces/consultations", None).await
    }

    /// `GET /acces/motifs` : liste des motifs d'une consultation hors périmètre (S-542).
    pub async fn acces_motifs(&self) -> Result<Vec<MotifConsultationDto>, ApiError> {
        self.appel(Methode::Get, "/acces/motifs", None).await
    }

    /// `GET /eleves/{id}?motif=` : ouverture d'une fiche hors du périmètre de l'agent, avec le motif choisi (RGP-191).
    /// Les codes de motif ne contiennent que `[a-z0-9_]` : aucun encodage n'est nécessaire.
    pub async fn eleve_fiche_motive(
        &self,
        eleve: Uuid,
        motif: &str,
    ) -> Result<EleveFicheDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/eleves/{eleve}?motif={motif}"),
            None,
        )
        .await
    }

    // ── Lot G4-2 : réglages, contrôles internes, alertes, traces d'un sujet ──

    /// `GET /acces/reglages` : réglages d'accès (valeur, défaut, bornes) et historique des réglages RGPD.
    pub async fn acces_reglages(&self) -> Result<ReglagesAccesDto, ApiError> {
        self.appel(Methode::Get, "/acces/reglages", None).await
    }

    /// `POST /acces/reglages` : modifie un réglage (`conformite.gerer`).
    pub async fn acces_reglage_modifier(
        &self,
        body: &ReglageAccesBody,
    ) -> Result<ReglagesAccesDto, ApiError> {
        self.appel(Methode::Post, "/acces/reglages", corps(body)?)
            .await
    }

    /// `GET /acces/controles` : les contrôles internes (aucun total par agent, aucun classement).
    pub async fn acces_controles(&self) -> Result<Vec<ControleDto>, ApiError> {
        self.appel(Methode::Get, "/acces/controles", None).await
    }

    /// `POST /acces/controles/defi` : défi de passkey récente, lié au contrôle demandé.
    pub async fn acces_controle_defi(
        &self,
        body: &ControleDemandeBody,
    ) -> Result<ControleDefiDto, ApiError> {
        self.appel(Methode::Post, "/acces/controles/defi", corps(body)?)
            .await
    }

    /// `POST /acces/controles/ouvrir` : ouvre le contrôle (`acces.controle_ouvrir`, passkey récente).
    pub async fn acces_controle_ouvrir(
        &self,
        body: &ControleOuvrirBody,
    ) -> Result<ControleDto, ApiError> {
        self.appel(Methode::Post, "/acces/controles/ouvrir", corps(body)?)
            .await
    }

    /// `GET /acces/controles/{id}/traces` : consultations de l'agent contrôlé (contrôle ouvert seulement ; lecture tracée).
    pub async fn acces_controle_traces(&self, id: Uuid) -> Result<TracesControleDto, ApiError> {
        self.appel(Methode::Get, &format!("/acces/controles/{id}/traces"), None)
            .await
    }

    /// `POST /acces/controles/{id}/clore`.
    pub async fn acces_controle_clore(
        &self,
        id: Uuid,
        body: &ControleCloreBody,
    ) -> Result<ControleDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/acces/controles/{id}/clore"),
            corps(body)?,
        )
        .await
    }

    /// `GET /acces/alertes` : alertes d'accès (à traiter d'abord).
    pub async fn acces_alertes(&self) -> Result<Vec<AlerteAccesDto>, ApiError> {
        self.appel(Methode::Get, "/acces/alertes", None).await
    }

    /// `POST /acces/alertes/{id}/suite`.
    pub async fn acces_alerte_suite(
        &self,
        id: Uuid,
        body: &AlerteSuiteBody,
    ) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/acces/alertes/{id}/suite"),
            corps(body)?,
        )
        .await
    }

    /// `GET /acces/traces/demandes/{id}` : traces des sujets d'une demande ouverte (lecture tracée).
    pub async fn acces_traces_demande(&self, id: Uuid) -> Result<Vec<TracesSujetDto>, ApiError> {
        self.appel(Methode::Get, &format!("/acces/traces/demandes/{id}"), None)
            .await
    }

    /// `POST /acces/differer` : diffère l'affichage d'une consultation liée à un signalement (`droits.arbitrer`).
    pub async fn acces_differer(&self, body: &DifferBody) -> Result<(), ApiError> {
        self.appel(Methode::Post, "/acces/differer", corps(body)?)
            .await
    }

    /// `POST /acces/demandes/{id}/nom-agent` : communique le nom d'un agent pour une demande (`droits.arbitrer`).
    pub async fn acces_nom_agent(
        &self,
        demande: Uuid,
        body: &NomAgentBody,
    ) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/acces/demandes/{demande}/nom-agent"),
            corps(body)?,
        )
        .await
    }
}
