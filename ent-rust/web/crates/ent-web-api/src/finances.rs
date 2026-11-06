//! Service des finances (socle 2, lot S2-9) : routes `/finances/*` et `/scolarite/*`
//! d'`ent-api`. Droits, montants, répartition entre payeurs, émission, prélèvements,
//! rejets et relances sont décidés par le serveur ; ce client transporte.

use ent_types::finances::*;
use uuid::Uuid;

use crate::{transport, ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    pub async fn parametres_finances(&self) -> Result<ParametresFinancesDto, ApiError> {
        self.appel(Methode::Get, "/finances/parametres", None).await
    }

    pub async fn modifier_parametres_finances(
        &self,
        b: &ParametresFinancesBody,
    ) -> Result<ParametresFinancesDto, ApiError> {
        self.appel(Methode::Put, "/finances/parametres", Some(corps(b)?))
            .await
    }

    pub async fn articles_finances(&self) -> Result<Vec<ArticleDto>, ApiError> {
        self.appel(Methode::Get, "/finances/articles", None).await
    }

    pub async fn creer_article(&self, b: &ArticleBody) -> Result<ArticleDto, ApiError> {
        self.appel(Methode::Post, "/finances/articles", Some(corps(b)?))
            .await
    }

    pub async fn tarifs(&self, annee: &str) -> Result<Vec<TarifDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/finances/tarifs?annee={annee}"),
            None,
        )
        .await
    }

    pub async fn creer_tarif(&self, b: &TarifBody) -> Result<TarifDto, ApiError> {
        self.appel(Methode::Post, "/finances/tarifs", Some(corps(b)?))
            .await
    }

    pub async fn generer_annee(
        &self,
        b: &GenerationBody,
    ) -> Result<RapportGenerationDto, ApiError> {
        self.appel(Methode::Post, "/finances/generation", Some(corps(b)?))
            .await
    }

    pub async fn factures_finances(
        &self,
        statut: Option<&str>,
    ) -> Result<Vec<FactureResumeDto>, ApiError> {
        let chemin = match statut {
            Some(s) => format!("/finances/factures?statut={s}"),
            None => "/finances/factures".to_string(),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    pub async fn facture_finances(&self, id: Uuid) -> Result<FactureDetailDto, ApiError> {
        self.appel(Methode::Get, &format!("/finances/factures/{id}"), None)
            .await
    }

    pub async fn emettre_factures(
        &self,
        factures: Vec<Uuid>,
    ) -> Result<RapportEmissionDto, ApiError> {
        self.appel(
            Methode::Post,
            "/finances/factures/emettre",
            Some(corps(&EmissionBody { factures })?),
        )
        .await
    }

    pub async fn creer_avoir(&self, id: Uuid, b: &AvoirBody) -> Result<FactureDetailDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/finances/factures/{id}/avoir"),
            Some(corps(b)?),
        )
        .await
    }

    pub async fn prelevements(
        &self,
        statut: Option<&str>,
    ) -> Result<Vec<PrelevementDto>, ApiError> {
        let chemin = match statut {
            Some(s) => format!("/finances/prelevements?statut={s}"),
            None => "/finances/prelevements".to_string(),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    pub async fn preparer_prelevements(
        &self,
        b: &PreparationBody,
    ) -> Result<RapportPreparationDto, ApiError> {
        self.appel(
            Methode::Post,
            "/finances/prelevements/preparer",
            Some(corps(b)?),
        )
        .await
    }

    /// Fichier de prélèvements (pain.008 XML ou CSV) : contenu brut.
    pub async fn exporter_prelevements(
        &self,
        b: &ExportPrelevementsBody,
    ) -> Result<String, ApiError> {
        let (statut, texte) = transport::envoyer(
            Methode::Post,
            &self.url("/finances/prelevements/export"),
            self.auth(),
            Some(corps(b)?),
        )
        .await?;
        if (200..300).contains(&statut) {
            Ok(texte)
        } else {
            Err(ApiError::depuis_statut(statut, texte))
        }
    }

    pub async fn constater_execution(
        &self,
        b: &ExecutionBody,
    ) -> Result<RapportExecutionDto, ApiError> {
        self.appel(
            Methode::Post,
            "/finances/prelevements/execution",
            Some(corps(b)?),
        )
        .await
    }

    pub async fn traiter_rejet(&self, prelevement: Uuid, code: &str) -> Result<RejetDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/finances/prelevements/{prelevement}/rejet"),
            Some(corps(&RejetBody { code: code.into() })?),
        )
        .await
    }

    pub async fn rejets(&self) -> Result<Vec<RejetDto>, ApiError> {
        self.appel(Methode::Get, "/finances/rejets", None).await
    }

    pub async fn enregistrer_reglement(&self, b: &ReglementBody) -> Result<ReglementDto, ApiError> {
        self.appel(Methode::Post, "/finances/reglements", Some(corps(b)?))
            .await
    }

    pub async fn compte_foyer(&self, foyer: Uuid) -> Result<CompteFoyerDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/finances/foyers/{foyer}/compte"),
            None,
        )
        .await
    }

    pub async fn relances(&self, statut: Option<&str>) -> Result<Vec<RelanceDto>, ApiError> {
        let chemin = match statut {
            Some(s) => format!("/finances/relances?statut={s}"),
            None => "/finances/relances".to_string(),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    pub async fn proposer_relances(&self) -> Result<RapportRelancesDto, ApiError> {
        self.appel(Methode::Post, "/finances/relances/proposer", None)
            .await
    }

    /// Confirmation humaine (`true`) ou annulation d'une relance proposée.
    pub async fn decider_relance(&self, id: Uuid, confirmer: bool) -> Result<RelanceDto, ApiError> {
        let action = if confirmer { "confirmer" } else { "annuler" };
        self.appel(
            Methode::Post,
            &format!("/finances/relances/{id}/{action}"),
            None,
        )
        .await
    }

    pub async fn exports_comptables(&self) -> Result<Vec<ExportComptableDto>, ApiError> {
        self.appel(Methode::Get, "/finances/exports-comptables", None)
            .await
    }

    pub async fn exporter_comptabilite(
        &self,
        b: &ExportComptableBody,
    ) -> Result<ExportComptableDto, ApiError> {
        self.appel(
            Methode::Post,
            "/finances/exports-comptables",
            Some(corps(b)?),
        )
        .await
    }

    /// Espace famille : comptes des foyers dont la personne est payeuse.
    pub async fn finances_famille(&self) -> Result<EspaceFamilleFinancesDto, ApiError> {
        self.appel(Methode::Get, "/finances/famille", None).await
    }
}
