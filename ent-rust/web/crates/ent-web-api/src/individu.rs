//! Transverse « individu » (migration 279, `docs/socles/TRANSVERSE_INDIVIDU.md`) : routes
//! `/dossier/individu/*` d'`ent-api`, et la recherche de fiches (`GET /dossier/personnes?q=`) qui
//! sert à choisir deux fiches à relier. Le serveur décide de tout (droits, anti-énumération,
//! « jamais par l'individu concerné ») ; ce client transporte.

use ent_types::dossier::PersonneTrouveeDto;
use ent_types::individu::{
    DeclarationIndividuBody, EcarterPropositionBody, LeverLienBody, LienIndividu, MonIndividu,
    PropositionIndividu, RelierFichesBody, ReponseDeclarationBody,
};
use uuid::Uuid;

use crate::communication::encoder_requete;
use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /dossier/individu/moi` : ce que le compte voit de son individu (nombres, liens sans
    /// l'autre fiche, déclarations reçues).
    pub async fn individu_moi(&self) -> Result<MonIndividu, ApiError> {
        self.appel(Methode::Get, "/dossier/individu/moi", None)
            .await
    }

    /// `POST /dossier/individu/declarations` : « ce compte est aussi moi ». Réponse identique que
    /// l'adresse soit connue ou non (anti-énumération).
    pub async fn individu_declarer(&self, email: &str) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                "/dossier/individu/declarations",
                Some(corps(&DeclarationIndividuBody {
                    email: email.to_string(),
                })?),
            )
            .await?;
        Ok(())
    }

    /// `POST /dossier/individu/declarations/{id}/reponse`, depuis le compte cible.
    pub async fn individu_repondre(&self, id: Uuid, confirmer: bool) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                &format!("/dossier/individu/declarations/{id}/reponse"),
                Some(corps(&ReponseDeclarationBody { confirmer })?),
            )
            .await?;
        Ok(())
    }

    /// `GET /dossier/individu/liens` : liens actifs (direction, secrétariat).
    pub async fn individu_liens(&self) -> Result<Vec<LienIndividu>, ApiError> {
        self.appel(Methode::Get, "/dossier/individu/liens", None)
            .await
    }

    /// `POST /dossier/individu/liens` : relie deux fiches.
    pub async fn individu_relier(&self, body: &RelierFichesBody) -> Result<Uuid, ApiError> {
        self.appel(Methode::Post, "/dossier/individu/liens", Some(corps(body)?))
            .await
    }

    /// `POST /dossier/individu/liens/{id}/lever` (direction, jamais l'individu concerné).
    pub async fn individu_lever(&self, id: Uuid, motif_code: &str) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                &format!("/dossier/individu/liens/{id}/lever"),
                Some(corps(&LeverLienBody {
                    motif_code: motif_code.to_string(),
                })?),
            )
            .await?;
        Ok(())
    }

    /// `GET /dossier/individu/propositions` : ressemblances, jamais la coordonnée.
    pub async fn individu_propositions(&self) -> Result<Vec<PropositionIndividu>, ApiError> {
        self.appel(Methode::Get, "/dossier/individu/propositions", None)
            .await
    }

    /// `POST /dossier/individu/propositions/ecarter`.
    pub async fn individu_ecarter(&self, body: &EcarterPropositionBody) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                "/dossier/individu/propositions/ecarter",
                Some(corps(body)?),
            )
            .await?;
        Ok(())
    }

    /// `GET /dossier/personnes?q=` : fiches de l'établissement (au plus 50, deux caractères au
    /// moins ; `personnes.gerer`).
    pub async fn personnes_rechercher(&self, q: &str) -> Result<Vec<PersonneTrouveeDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/dossier/personnes?q={}", encoder_requete(q)),
            None,
        )
        .await
    }
}
