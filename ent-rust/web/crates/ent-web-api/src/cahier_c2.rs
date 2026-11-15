//! Cahier de textes, lot C2 : saisie et publication. Routes `/cahier/c2/*` d'`ent-api` ; types
//! d'`ent-types::cahier_c2`. Le serveur donne les jours et heures de l'établissement : aucune conversion ici.

use chrono::NaiveDate;
use ent_types::cahier_c2::{
    ContributionBody, ContributionDto, CopieSeanceBody, CopieVersBody, CoursSaisieDto, EntreeBody,
    EntreeDto, ModifierTravailC2Body, NatureTravailDto, RetraitC2Body, SeanceDuCoursDto,
    TravailC2Body, TravailC2Dto, TravailModifieDto, TravauxCreesDto, VersionDto, VersionTravailDto,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /cahier/c2/cours` : les cours où l'on écrit aujourd'hui.
    pub async fn cdt_cours(&self) -> Result<Vec<CoursSaisieDto>, ApiError> {
        self.appel(Methode::Get, "/cahier/c2/cours", None).await
    }

    /// `GET /cahier/c2/natures` : natures de travail proposées, avec leur durée.
    pub async fn cdt_natures(&self) -> Result<Vec<NatureTravailDto>, ApiError> {
        self.appel(Methode::Get, "/cahier/c2/natures", None).await
    }

    /// `GET /cahier/c2/cours/{id}/seances` : séances d'un cours avec entrée et travaux (écran 1).
    pub async fn cdt_seances_du_cours(
        &self,
        cours_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceDuCoursDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c2/cours/{cours_id}/seances?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `PUT /cahier/c2/cours/{id}/seances/{seance}/entree` : crée ou modifie l'entrée.
    pub async fn cdt_entree_enregistrer(
        &self,
        cours_id: Uuid,
        seance_id: Uuid,
        b: &EntreeBody,
    ) -> Result<EntreeDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/cahier/c2/cours/{cours_id}/seances/{seance_id}/entree"),
            corps(b)?,
        )
        .await
    }

    /// `GET /cahier/c2/entrees/{id}/versions` : versions précédentes d'une entrée.
    pub async fn cdt_entree_versions(&self, id: Uuid) -> Result<Vec<VersionDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c2/entrees/{id}/versions"),
            None,
        )
        .await
    }

    /// `POST /cahier/c2/entrees/{id}/retrait` : retire l'entrée de la diffusion (motif obligatoire).
    pub async fn cdt_entree_retirer(&self, id: Uuid, motif: &str) -> Result<EntreeDto, ApiError> {
        let b = RetraitC2Body {
            motif: motif.to_string(),
        };
        self.appel(
            Methode::Post,
            &format!("/cahier/c2/entrees/{id}/retrait"),
            corps(&b)?,
        )
        .await
    }

    /// `POST /cahier/c2/entrees/{id}/copies` : copie l'entrée vers d'autres séances (brouillons).
    pub async fn cdt_entree_copier(
        &self,
        id: Uuid,
        vers: Vec<CopieVersBody>,
    ) -> Result<Vec<EntreeDto>, ApiError> {
        let b = CopieSeanceBody { vers };
        self.appel(
            Methode::Post,
            &format!("/cahier/c2/entrees/{id}/copies"),
            corps(&b)?,
        )
        .await
    }

    /// `POST /cahier/c2/entrees/{id}/contributions` : contribution signée d'un co-intervenant.
    pub async fn cdt_contribuer(
        &self,
        id: Uuid,
        contenu: &str,
    ) -> Result<ContributionDto, ApiError> {
        let b = ContributionBody {
            contenu: contenu.to_string(),
        };
        self.appel(
            Methode::Post,
            &format!("/cahier/c2/entrees/{id}/contributions"),
            corps(&b)?,
        )
        .await
    }

    /// `POST /cahier/c2/travaux` : donne un travail à un ou plusieurs cours.
    pub async fn cdt_travaux_creer(&self, b: &TravailC2Body) -> Result<TravauxCreesDto, ApiError> {
        self.appel(Methode::Post, "/cahier/c2/travaux", corps(b)?)
            .await
    }

    /// `PUT /cahier/c2/travaux/{id}` : modifie un travail (avec ou sans avis).
    pub async fn cdt_travail_modifier(
        &self,
        id: Uuid,
        b: &ModifierTravailC2Body,
    ) -> Result<TravailModifieDto, ApiError> {
        self.appel(Methode::Put, &format!("/cahier/c2/travaux/{id}"), corps(b)?)
            .await
    }

    /// `GET /cahier/c2/travaux/{id}` : un travail.
    pub async fn cdt_travail(&self, id: Uuid) -> Result<TravailC2Dto, ApiError> {
        self.appel(Methode::Get, &format!("/cahier/c2/travaux/{id}"), None)
            .await
    }

    /// `GET /cahier/c2/travaux/{id}/versions` : versions précédentes d'un travail.
    pub async fn cdt_travail_versions(&self, id: Uuid) -> Result<Vec<VersionTravailDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c2/travaux/{id}/versions"),
            None,
        )
        .await
    }

    /// `POST /cahier/c2/travaux/{id}/retrait` : retire le travail de la diffusion (motif obligatoire).
    pub async fn cdt_travail_retirer(
        &self,
        id: Uuid,
        motif: &str,
    ) -> Result<TravailC2Dto, ApiError> {
        let b = RetraitC2Body {
            motif: motif.to_string(),
        };
        self.appel(
            Methode::Post,
            &format!("/cahier/c2/travaux/{id}/retrait"),
            corps(&b)?,
        )
        .await
    }
}
