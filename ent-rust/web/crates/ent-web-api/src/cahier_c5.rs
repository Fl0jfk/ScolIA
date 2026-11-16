//! Cahier de textes, lot C5 : rendu et retour. Routes `/cahier/c5/*` d'`ent-api` ; types d'`ent-types::cahier_c5`.
//! Les fichiers partent en base64 (même convention que C2) ; ils se lisent par un lien court émis après contrôle des droits.

use chrono::{DateTime, Utc};
use ent_types::cahier_c5::*;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /cahier/c5/travaux/{id}/remise` : la situation d'un élève (ou de son enfant) devant un travail à rendre.
    pub async fn cdt_remise(
        &self,
        travail: Uuid,
        eleve: Option<Uuid>,
    ) -> Result<RemiseLueDto, ApiError> {
        let e = eleve.map(|e| format!("?eleve_id={e}")).unwrap_or_default();
        self.appel(
            Methode::Get,
            &format!("/cahier/c5/travaux/{travail}/remise{e}"),
            None,
        )
        .await
    }

    /// `PUT .../brouillon` : prépare la remise (seul le dépôt compte).
    pub async fn cdt_brouillon(
        &self,
        travail: Uuid,
        b: &DepotBody,
    ) -> Result<RemiseLueDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/cahier/c5/travaux/{travail}/brouillon"),
            corps(b)?,
        )
        .await
    }

    /// `POST .../depot` : dépôt immuable avec accusé.
    pub async fn cdt_deposer(&self, travail: Uuid, b: &DepotBody) -> Result<AccuseDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/travaux/{travail}/depot"),
            corps(b)?,
        )
        .await
    }

    /// `POST .../signalement` : « je n'arrive pas à déposer ».
    pub async fn cdt_depot_signaler(
        &self,
        travail: Uuid,
        message: Option<String>,
    ) -> Result<DateTime<Utc>, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/travaux/{travail}/signalement"),
            corps(&SignalementBody { message })?,
        )
        .await
    }

    /// `POST .../delai` : demande de délai (élève, ou responsable pour son enfant).
    pub async fn cdt_delai_demander(
        &self,
        travail: Uuid,
        eleve: Option<Uuid>,
        message: Option<String>,
    ) -> Result<DelaiLuDto, ApiError> {
        let e = eleve.map(|e| format!("?eleve_id={e}")).unwrap_or_default();
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/travaux/{travail}/delai{e}"),
            corps(&DemandeDelaiBody { message })?,
        )
        .await
    }

    /// `GET .../groupes` : groupes actifs d'un travail (professeur en saisie, ou élève du public).
    pub async fn cdt_groupes(&self, travail: Uuid) -> Result<Vec<GroupeDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c5/travaux/{travail}/groupes"),
            None,
        )
        .await
    }

    /// `POST .../groupes/rejoindre` : l'élève crée un groupe ou rejoint celui d'un camarade.
    pub async fn cdt_groupe_rejoindre(
        &self,
        travail: Uuid,
        b: &GroupeLibreBody,
    ) -> Result<GroupeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/travaux/{travail}/groupes/rejoindre"),
            corps(b)?,
        )
        .await
    }

    /// `GET .../suivi` : le tableau de suivi (professeur en saisie).
    pub async fn cdt_suivi(&self, travail: Uuid) -> Result<SuiviC5Dto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c5/travaux/{travail}/suivi"),
            None,
        )
        .await
    }

    /// `POST .../constats` : constats en lot.
    pub async fn cdt_constater(&self, travail: Uuid, b: &ConstatsBody) -> Result<usize, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/travaux/{travail}/constats"),
            corps(b)?,
        )
        .await
    }

    /// `POST .../prolongations` : délai accordé à des élèves.
    pub async fn cdt_prolonger(
        &self,
        travail: Uuid,
        b: &ProlongationBody,
    ) -> Result<usize, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/travaux/{travail}/prolongations"),
            corps(b)?,
        )
        .await
    }

    /// `PUT .../cloture` : clôture du ramassage.
    pub async fn cdt_cloture(
        &self,
        travail: Uuid,
        b: &ClotureBody,
    ) -> Result<DateTime<Utc>, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/cahier/c5/travaux/{travail}/cloture"),
            corps(b)?,
        )
        .await
    }

    /// `POST .../relance` : aperçu, puis envoi confirmé.
    pub async fn cdt_relancer(
        &self,
        travail: Uuid,
        b: &RelanceBody,
    ) -> Result<RelanceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/travaux/{travail}/relance"),
            corps(b)?,
        )
        .await
    }

    /// `POST .../retours/lot` : rendre toutes les copies corrigées.
    pub async fn cdt_retours_lot(&self, travail: Uuid) -> Result<LotRenduDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/travaux/{travail}/retours/lot"),
            None,
        )
        .await
    }

    /// `GET .../copies` : copies à télécharger, nommées « NOM Prénom – titre ».
    pub async fn cdt_copies(&self, travail: Uuid) -> Result<Vec<CopieATelechargerDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c5/travaux/{travail}/copies"),
            None,
        )
        .await
    }

    /// `GET /cahier/c5/remises/{id}/retour` : le retour (brouillon compris).
    pub async fn cdt_retour_lire(&self, remise: Uuid) -> Result<Option<RetourProfDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c5/remises/{remise}/retour"),
            None,
        )
        .await
    }

    /// `PUT /cahier/c5/remises/{id}/retour` : brouillon, rendu ou correction (nouvelle version).
    pub async fn cdt_retour_enregistrer(
        &self,
        remise: Uuid,
        b: &RetourC5Body,
    ) -> Result<RetourProfDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/cahier/c5/remises/{remise}/retour"),
            corps(b)?,
        )
        .await
    }

    /// `POST /cahier/c5/remises/{id}/rouvrir` : « à refaire ».
    pub async fn cdt_remise_rouvrir(&self, remise: Uuid, b: &RouvrirBody) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/remises/{remise}/rouvrir"),
            corps(b)?,
        )
        .await
    }

    /// `GET .../delais` : demandes de délai en attente.
    pub async fn cdt_delais_attente(
        &self,
        travail: Uuid,
    ) -> Result<Vec<DelaiAttenteDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/c5/travaux/{travail}/delais"),
            None,
        )
        .await
    }

    /// `POST /cahier/c5/delais/{id}/decision` : accepter (avec échéance) ou refuser.
    pub async fn cdt_delai_decider(
        &self,
        demande: Uuid,
        b: &DelaiDecisionBody,
    ) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/delais/{demande}/decision"),
            corps(b)?,
        )
        .await
    }

    /// `POST /cahier/c5/pieces/{id}/lien` : lien court vers une pièce, après contrôle des droits.
    pub async fn cdt_piece_lien(&self, piece: Uuid) -> Result<LienPieceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/c5/pieces/{piece}/lien"),
            None,
        )
        .await
    }

    /// Adresse absolue d'un lien court.
    pub fn adresse_lien_c5(&self, lien: &LienPieceDto) -> String {
        if lien.url.starts_with('/') {
            self.url(&lien.url)
        } else {
            lien.url.clone()
        }
    }
}
