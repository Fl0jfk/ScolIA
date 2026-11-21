//! Discipline et assiduité (socle 2, lot S2-4) : punitions, sanctions (lecture), procédures,
//! signalements, décompte. Routes d'`ent-api/src/vie_scolaire.rs`. Les droits
//! (`punitions.donner`, `discipline.gerer`, `assiduite.signaler`…) sont vérifiés par le serveur.

use chrono::NaiveDate;
use ent_types::discipline::{
    DecompteMoisDto, DonnerPunitionBody, EffacementRapportDto, EngagerProcedureBody, ProcedureDto,
    PunitionDto, RegleAssiduiteDto, SanctionDto, SignalementDto, TypeVsDto,
};
use serde_json::json;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /vs/punition-types`.
    pub async fn types_punition(&self) -> Result<Vec<TypeVsDto>, ApiError> {
        self.appel(Methode::Get, "/vs/punition-types", None).await
    }

    /// `POST /vs/punition-types/amorcer` : types usuels (idempotent).
    pub async fn amorcer_types_punition(&self) -> Result<Vec<TypeVsDto>, ApiError> {
        self.appel(Methode::Post, "/vs/punition-types/amorcer", Some(json!({})))
            .await
    }

    /// `GET /eleves/{id}/punitions`.
    pub async fn punitions_eleve(&self, eleve: Uuid) -> Result<Vec<PunitionDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}/punitions"), None)
            .await
    }

    /// `POST /eleves/{id}/punitions`.
    pub async fn donner_punition(
        &self,
        eleve: Uuid,
        body: &DonnerPunitionBody,
    ) -> Result<PunitionDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/eleves/{eleve}/punitions"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /vs/punitions/{id}/clore` (`effectuee` | `annulee`).
    pub async fn clore_punition(&self, id: Uuid, statut: &str) -> Result<PunitionDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/vs/punitions/{id}/clore"),
            Some(json!({ "statut": statut })),
        )
        .await
    }

    /// `GET /eleves/{id}/sanctions` : les sanctions, distinctes des punitions.
    pub async fn sanctions_eleve(&self, eleve: Uuid) -> Result<Vec<SanctionDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}/sanctions"), None)
            .await
    }

    /// `GET /eleves/{id}/procedures`.
    pub async fn procedures_eleve(&self, eleve: Uuid) -> Result<Vec<ProcedureDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}/procedures"), None)
            .await
    }

    /// `POST /vs/procedures`.
    pub async fn engager_procedure(
        &self,
        body: &EngagerProcedureBody,
    ) -> Result<ProcedureDto, ApiError> {
        self.appel(Methode::Post, "/vs/procedures", Some(corps(body)?))
            .await
    }

    /// `POST /vs/procedures/{id}/observations`.
    pub async fn recueillir_observations(
        &self,
        id: Uuid,
        le: NaiveDate,
    ) -> Result<ProcedureDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/vs/procedures/{id}/observations"),
            Some(json!({ "le": le })),
        )
        .await
    }

    /// `POST /vs/procedures/{id}/convoquer` : convocation du conseil de discipline.
    pub async fn convoquer_conseil(
        &self,
        id: Uuid,
        convocation_envoyee_le: NaiveDate,
        conseil_le: NaiveDate,
    ) -> Result<ProcedureDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/vs/procedures/{id}/convoquer"),
            Some(json!({ "convocation_envoyee_le": convocation_envoyee_le, "conseil_le": conseil_le })),
        )
        .await
    }

    /// `POST /vs/procedures/{id}/decider` : décision (texte libre, daté).
    pub async fn decider_procedure(
        &self,
        id: Uuid,
        decision: &str,
        decision_le: NaiveDate,
    ) -> Result<ProcedureDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/vs/procedures/{id}/decider"),
            Some(json!({ "decision": decision, "decision_le": decision_le })),
        )
        .await
    }

    /// `POST /vs/procedures/{id}/classer` : classement sans suite.
    pub async fn classer_procedure(&self, id: Uuid) -> Result<ProcedureDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/vs/procedures/{id}/classer"),
            Some(json!({})),
        )
        .await
    }

    /// `GET /vs/decompte?mois=` : demi-journées par élève.
    pub async fn decompte_mensuel(&self, mois: NaiveDate) -> Result<DecompteMoisDto, ApiError> {
        self.appel(Methode::Get, &format!("/vs/decompte?mois={mois}"), None)
            .await
    }

    /// `GET /vs/regle-assiduite`.
    pub async fn regle_assiduite(&self) -> Result<RegleAssiduiteDto, ApiError> {
        self.appel(Methode::Get, "/vs/regle-assiduite", None).await
    }

    /// `GET /vs/signalements?mois=`.
    pub async fn signalements(&self, mois: NaiveDate) -> Result<Vec<SignalementDto>, ApiError> {
        self.appel(Methode::Get, &format!("/vs/signalements?mois={mois}"), None)
            .await
    }

    /// `POST /vs/signalements/proposer` : le CPE propose les élèves au seuil du mois.
    pub async fn proposer_signalements(
        &self,
        mois: NaiveDate,
    ) -> Result<Vec<SignalementDto>, ApiError> {
        self.appel(
            Methode::Post,
            "/vs/signalements/proposer",
            Some(json!({ "mois": mois })),
        )
        .await
    }

    /// `POST /vs/signalements/{id}/decider` (`valider` | `refuser` | `emettre`).
    pub async fn decider_signalement(
        &self,
        id: Uuid,
        action: &str,
    ) -> Result<SignalementDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/vs/signalements/{id}/decider"),
            Some(json!({ "action": action })),
        )
        .await
    }

    /// `POST /vs/sanctions/effacer?jour=` : efface les sanctions échues (idempotent).
    pub async fn effacer_sanctions(
        &self,
        jour: NaiveDate,
    ) -> Result<EffacementRapportDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/vs/sanctions/effacer?jour={jour}"),
            Some(json!({})),
        )
        .await
    }
}
