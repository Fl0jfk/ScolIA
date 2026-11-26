//! Socle 27, lot G4-1 : traces d'accès. Routes `/acces/*` d'`ent-api` ; types d'`ent-types::acces_traces`.

use ent_types::acces_traces::*;
use ent_types::eleve_synthese::EleveFicheDto;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

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
}
