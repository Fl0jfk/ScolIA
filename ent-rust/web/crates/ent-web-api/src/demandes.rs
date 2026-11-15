//! Demandes et flux de travail (socle N1, lot N1-4) : routes `/demandes…` d'`ent-api`.
//! Le serveur décide de tout (droits, 404 neutre, version) ; ce client transporte.

use ent_types::demandes::{
    DecisionHseDto, DemandeFile, DepotDemandeDto, DepotHseDto, EnveloppeHseDto, EtatDemandeDto,
    GesteDto, Guichet, LigneCopiesDto, LigneHseDto, MaDemande, PreteDto, ResultatDecisionHseDto,
    ValidationCopiesDto,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /demandes/guichet` : les démarches proposées à cette personne.
    pub async fn demandes_guichet(&self) -> Result<Guichet, ApiError> {
        self.appel(Methode::Get, "/demandes/guichet", None).await
    }

    /// `POST /demandes` : dépose une demande au nom de la personne connectée.
    pub async fn demandes_deposer(&self, d: &DepotDemandeDto) -> Result<EtatDemandeDto, ApiError> {
        self.appel(Methode::Post, "/demandes", Some(corps(d)?))
            .await
    }

    /// `GET /demandes/mes` : mes demandes (projection du demandeur).
    pub async fn demandes_mes(&self) -> Result<Vec<MaDemande>, ApiError> {
        self.appel(Methode::Get, "/demandes/mes", None).await
    }

    /// `GET /demandes/file` : « Ma file » (personnel).
    pub async fn demandes_ma_file(&self) -> Result<Vec<DemandeFile>, ApiError> {
        self.appel(Methode::Get, "/demandes/file", None).await
    }

    /// `POST /demandes/{id}/prendre`.
    pub async fn demandes_prendre(
        &self,
        id: Uuid,
        version: i32,
    ) -> Result<EtatDemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/demandes/{id}/prendre"),
            Some(corps(&GesteDto { version })?),
        )
        .await
    }

    /// `POST /demandes/{id}/annuler`.
    pub async fn demandes_annuler(
        &self,
        id: Uuid,
        version: i32,
    ) -> Result<EtatDemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/demandes/{id}/annuler"),
            Some(corps(&GesteDto { version })?),
        )
        .await
    }

    /// `POST /demandes/hse` : demande d'HSE (en quarts d'heure).
    pub async fn hse_deposer(&self, d: &DepotHseDto) -> Result<EtatDemandeDto, ApiError> {
        self.appel(Methode::Post, "/demandes/hse", Some(corps(d)?))
            .await
    }

    /// `GET /demandes/hse/mes`.
    pub async fn hse_mes(&self) -> Result<Vec<LigneHseDto>, ApiError> {
        self.appel(Methode::Get, "/demandes/hse/mes", None).await
    }

    /// `GET /demandes/hse/a-decider` (direction).
    pub async fn hse_a_decider(&self) -> Result<Vec<LigneHseDto>, ApiError> {
        self.appel(Methode::Get, "/demandes/hse/a-decider", None)
            .await
    }

    /// `GET /demandes/hse/enveloppe`.
    pub async fn hse_enveloppe(&self) -> Result<EnveloppeHseDto, ApiError> {
        self.appel(Methode::Get, "/demandes/hse/enveloppe", None)
            .await
    }

    /// `POST /demandes/hse/{id}/confirmer` : le collègue confirme une demande déposée en son nom.
    pub async fn hse_confirmer(&self, id: Uuid) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                &format!("/demandes/hse/{id}/confirmer"),
                Some(serde_json::json!({})),
            )
            .await?;
        Ok(())
    }

    /// `POST /demandes/hse/{id}/decider`.
    pub async fn hse_decider(
        &self,
        id: Uuid,
        d: &DecisionHseDto,
    ) -> Result<ResultatDecisionHseDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/demandes/hse/{id}/decider"),
            Some(corps(d)?),
        )
        .await
    }

    /// `GET /demandes/copies/file` : la file d'impression **des sites désignés** au réceptionnaire.
    pub async fn copies_file(&self) -> Result<Vec<LigneCopiesDto>, ApiError> {
        self.appel(Methode::Get, "/demandes/copies/file", None)
            .await
    }

    /// `GET /demandes/copies/mes`.
    pub async fn copies_mes(&self) -> Result<Vec<LigneCopiesDto>, ApiError> {
        self.appel(Methode::Get, "/demandes/copies/mes", None).await
    }

    /// `GET /demandes/copies/a-valider` (direction).
    pub async fn copies_a_valider(&self) -> Result<Vec<LigneCopiesDto>, ApiError> {
        self.appel(Methode::Get, "/demandes/copies/a-valider", None)
            .await
    }

    /// `POST /demandes/copies/{id}/valider`.
    pub async fn copies_valider(
        &self,
        id: Uuid,
        v: &ValidationCopiesDto,
    ) -> Result<EtatDemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/demandes/copies/{id}/valider"),
            Some(corps(v)?),
        )
        .await
    }

    /// `POST /demandes/copies/{id}/impression`.
    pub async fn copies_impression(&self, id: Uuid) -> Result<EtatDemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/demandes/copies/{id}/impression"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `POST /demandes/copies/{id}/prete` : geste authentifié (jamais un lien).
    pub async fn copies_prete(
        &self,
        id: Uuid,
        lieu: Option<Uuid>,
    ) -> Result<EtatDemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/demandes/copies/{id}/prete"),
            Some(corps(&PreteDto { lieu_retrait: lieu })?),
        )
        .await
    }
}
