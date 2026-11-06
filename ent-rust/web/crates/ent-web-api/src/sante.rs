//! Santé et accompagnement (socle 2, lot S2-11) : routes d'`ent-api/src/sante.rs`. Les droits
//! (`sante.*`, `accompagnement.gerer`, `amenagements.lire`, `contraintes.lire`,
//! `sante.urgence_lire`) sont vérifiés par le serveur ; ce client n'en suppose aucun.

use chrono::NaiveDate;
use ent_types::sante::{
    AeshDto, AffecterAeshBody, AmenagementDto, AmenagementsEleveDto, CatalogueSanteDto,
    CloturePassageBody, CompteRenduBody, ContraintePublieeDto, ContraintesEleveDto,
    DefinirAmenagementBody, DispositifARevoirDto, DispositifDto, DossierAccompagnementDto,
    EnregistrerPaiBody, EnregistrerProtocoleBody, FicheUrgenceDto, NouveauPassageBody,
    OuvrirDispositifBody, PaiARevoirDto, PaiCompletDto, PassageSanteDto, PlanifierReunionBody,
    ProtocoleSanteDto, PublierContrainteBody, ReunionEquipeDto, ReviserDispositifBody,
    SignerPaiBody,
};
use serde_json::json;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /sante/catalogue` : contraintes et aménagements normalisés, issues, parties du PAI.
    pub async fn catalogue_sante(&self) -> Result<CatalogueSanteDto, ApiError> {
        self.appel(Methode::Get, "/sante/catalogue", None).await
    }

    // ── Infirmerie ──

    /// `GET /sante/passages?jour=` : tableau du jour de l'infirmerie.
    pub async fn passages_du_jour(
        &self,
        jour: NaiveDate,
    ) -> Result<Vec<PassageSanteDto>, ApiError> {
        self.appel(Methode::Get, &format!("/sante/passages?jour={jour}"), None)
            .await
    }

    /// `GET /sante/passages?eleve_id=` : passages d'un élève (la famille ne reçoit que les visibles).
    pub async fn passages_eleve(&self, eleve: Uuid) -> Result<Vec<PassageSanteDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/sante/passages?eleve_id={eleve}"),
            None,
        )
        .await
    }

    /// `POST /sante/passages`.
    pub async fn creer_passage_sante(
        &self,
        body: &NouveauPassageBody,
    ) -> Result<PassageSanteDto, ApiError> {
        self.appel(Methode::Post, "/sante/passages", Some(corps(body)?))
            .await
    }

    /// `POST /sante/passages/{id}/cloturer`.
    pub async fn cloturer_passage(
        &self,
        passage: Uuid,
        body: &CloturePassageBody,
    ) -> Result<PassageSanteDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/sante/passages/{passage}/cloturer"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /sante/passages/{id}/informer-famille` : notification sans détail médical.
    pub async fn informer_famille_passage(
        &self,
        passage: Uuid,
    ) -> Result<PassageSanteDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/sante/passages/{passage}/informer-famille"),
            Some(json!({})),
        )
        .await
    }

    /// `GET /sante/pai-a-reviser?horizon=`.
    pub async fn pai_a_reviser(&self, horizon: i32) -> Result<Vec<PaiARevoirDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/sante/pai-a-reviser?horizon={horizon}"),
            None,
        )
        .await
    }

    /// `GET /eleves/{id}/pai-complet`.
    pub async fn pai_complet(&self, eleve: Uuid) -> Result<Option<PaiCompletDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}/pai-complet"), None)
            .await
    }

    /// `PUT /eleves/{id}/pai-complet` : toute écriture invalide les signatures.
    pub async fn enregistrer_pai(
        &self,
        eleve: Uuid,
        body: &EnregistrerPaiBody,
    ) -> Result<Option<PaiCompletDto>, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/eleves/{eleve}/pai-complet"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /eleves/{id}/pai/signatures`.
    pub async fn signer_pai(
        &self,
        eleve: Uuid,
        body: &SignerPaiBody,
    ) -> Result<PaiCompletDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/eleves/{eleve}/pai/signatures"),
            Some(corps(body)?),
        )
        .await
    }

    /// `GET /eleves/{id}/fiche-urgence` (consultation journalisée par le serveur).
    pub async fn fiche_urgence(&self, eleve: Uuid) -> Result<FicheUrgenceDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/eleves/{eleve}/fiche-urgence"),
            None,
        )
        .await
    }

    /// `GET /sante/protocoles`.
    pub async fn protocoles_sante(&self) -> Result<Vec<ProtocoleSanteDto>, ApiError> {
        self.appel(Methode::Get, "/sante/protocoles", None).await
    }

    /// `PUT /sante/protocoles`.
    pub async fn enregistrer_protocole(
        &self,
        body: &EnregistrerProtocoleBody,
    ) -> Result<ProtocoleSanteDto, ApiError> {
        self.appel(Methode::Put, "/sante/protocoles", Some(corps(body)?))
            .await
    }

    // ── Contraintes publiées ──

    /// `GET /sante/contraintes?audience=` : `restauration`, `enseignants` ou `sorties`.
    pub async fn contraintes_liste(
        &self,
        audience: &str,
    ) -> Result<Vec<ContraintesEleveDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/sante/contraintes?audience={audience}"),
            None,
        )
        .await
    }

    /// `GET /eleves/{id}/contraintes` (personnel de santé : toutes ; sinon `audience` obligatoire).
    pub async fn contraintes_eleve(
        &self,
        eleve: Uuid,
        audience: Option<&str>,
    ) -> Result<Vec<ContraintePublieeDto>, ApiError> {
        let q = audience
            .map(|a| format!("?audience={a}"))
            .unwrap_or_default();
        self.appel(
            Methode::Get,
            &format!("/eleves/{eleve}/contraintes{q}"),
            None,
        )
        .await
    }

    /// `POST /eleves/{id}/contraintes`.
    pub async fn publier_contrainte(
        &self,
        eleve: Uuid,
        body: &PublierContrainteBody,
    ) -> Result<ContraintePublieeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/eleves/{eleve}/contraintes"),
            Some(corps(body)?),
        )
        .await
    }

    /// `DELETE /sante/contraintes/{id}`.
    pub async fn retirer_contrainte(&self, id: Uuid) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(Methode::Delete, &format!("/sante/contraintes/{id}"), None)
            .await?;
        Ok(())
    }

    // ── Accompagnement ──

    /// `GET /accompagnement/mes-amenagements` : élèves du professeur, aménagements seulement.
    pub async fn mes_amenagements(&self) -> Result<Vec<AmenagementsEleveDto>, ApiError> {
        self.appel(Methode::Get, "/accompagnement/mes-amenagements", None)
            .await
    }

    /// `GET /accompagnement/a-reviser?horizon=`.
    pub async fn dispositifs_a_reviser(
        &self,
        horizon: i32,
    ) -> Result<Vec<DispositifARevoirDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/accompagnement/a-reviser?horizon={horizon}"),
            None,
        )
        .await
    }

    /// `GET /eleves/{id}/accompagnement`.
    pub async fn dossier_accompagnement(
        &self,
        eleve: Uuid,
    ) -> Result<DossierAccompagnementDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/eleves/{eleve}/accompagnement"),
            None,
        )
        .await
    }

    /// `GET /eleves/{id}/amenagements` (professeur de l'élève, gestion, famille liée, élève).
    pub async fn amenagements_eleve(&self, eleve: Uuid) -> Result<Vec<AmenagementDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}/amenagements"), None)
            .await
    }

    /// `POST /eleves/{id}/dispositifs`.
    pub async fn ouvrir_dispositif(
        &self,
        eleve: Uuid,
        body: &OuvrirDispositifBody,
    ) -> Result<DispositifDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/eleves/{eleve}/dispositifs"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /accompagnement/dispositifs/{id}/reviser` (réviser l'échéance ou clore).
    pub async fn reviser_dispositif(
        &self,
        dispositif: Uuid,
        body: &ReviserDispositifBody,
    ) -> Result<DispositifDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/accompagnement/dispositifs/{dispositif}/reviser"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /eleves/{id}/amenagements`.
    pub async fn definir_amenagement(
        &self,
        eleve: Uuid,
        body: &DefinirAmenagementBody,
    ) -> Result<AmenagementDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/eleves/{eleve}/amenagements"),
            Some(corps(body)?),
        )
        .await
    }

    /// `DELETE /accompagnement/amenagements/{id}`.
    pub async fn retirer_amenagement(&self, id: Uuid) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Delete,
                &format!("/accompagnement/amenagements/{id}"),
                None,
            )
            .await?;
        Ok(())
    }

    /// `POST /eleves/{id}/aesh`.
    pub async fn affecter_aesh(
        &self,
        eleve: Uuid,
        body: &AffecterAeshBody,
    ) -> Result<AeshDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/eleves/{eleve}/aesh"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /eleves/{id}/reunions`.
    pub async fn planifier_reunion(
        &self,
        eleve: Uuid,
        body: &PlanifierReunionBody,
    ) -> Result<ReunionEquipeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/eleves/{eleve}/reunions"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /accompagnement/reunions/{id}/compte-rendu`.
    pub async fn consigner_reunion(
        &self,
        reunion: Uuid,
        compte_rendu: &str,
    ) -> Result<(), ApiError> {
        let c = corps(&CompteRenduBody {
            compte_rendu: compte_rendu.to_string(),
        })?;
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                &format!("/accompagnement/reunions/{reunion}/compte-rendu"),
                Some(c),
            )
            .await?;
        Ok(())
    }
}
