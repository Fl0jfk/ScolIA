//! Tableau de bord de la vie scolaire en panneaux (socle 12, AS14b-bis). Réutilise les routes d'AS6
//! (`ent-api/src/assiduite_verification.rs`) et ajoute les trois du lot (`assiduite_tableau.rs`) : exclus du jour,
//! configuration (heure de bascule, disposition) et réglages. Les droits sont décidés par le serveur, qui répond
//! de la même façon à « interdit » et « inexistant » (R-AS35).

use chrono::{NaiveDate, NaiveTime};
use ent_types::assiduite::{
    AlerteDto, ConfigTableauDto, ExclusDuJourDto, FileDuMatinDto, HeureBasculeBody,
    PreferencePanneauDto, PreferenceTableauBody, ResultatPrevenirDto, TracerContactBody,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /assiduite/file-du-matin` : la file classée du jour (AS6). Sans `jour`, aujourd'hui.
    pub async fn file_du_matin(&self, jour: Option<NaiveDate>) -> Result<FileDuMatinDto, ApiError> {
        let q = jour.map(|j| format!("?jour={j}")).unwrap_or_default();
        self.appel(Methode::Get, &format!("/assiduite/file-du-matin{q}"), None)
            .await
    }

    /// `GET /assiduite/exclus-du-jour` : exclus de cours dont l'arrivée n'est pas confirmée.
    pub async fn exclus_du_jour(&self) -> Result<ExclusDuJourDto, ApiError> {
        self.appel(Methode::Get, "/assiduite/exclus-du-jour", None)
            .await
    }

    /// `GET /assiduite/tableau/configuration` : heure de bascule et disposition de l'utilisateur.
    pub async fn config_tableau(&self) -> Result<ConfigTableauDto, ApiError> {
        self.appel(Methode::Get, "/assiduite/tableau/configuration", None)
            .await
    }

    /// `PUT /assiduite/tableau/heure-bascule` (`assiduite.regle_gerer`).
    pub async fn regler_heure_bascule(&self, heure: NaiveTime) -> Result<NaiveTime, ApiError> {
        let r: HeureBasculeBody = self
            .appel(
                Methode::Put,
                "/assiduite/tableau/heure-bascule",
                Some(corps(&HeureBasculeBody { heure })?),
            )
            .await?;
        Ok(r.heure)
    }

    /// `PUT /assiduite/tableau/preference` : épingle la disposition de l'utilisateur connecté.
    pub async fn poser_preference_tableau(
        &self,
        panneaux: Vec<PreferencePanneauDto>,
    ) -> Result<Vec<PreferencePanneauDto>, ApiError> {
        self.appel(
            Methode::Put,
            "/assiduite/tableau/preference",
            Some(corps(&PreferenceTableauBody { panneaux })?),
        )
        .await
    }

    /// `DELETE /assiduite/tableau/preference` : retour à la disposition automatique.
    pub async fn effacer_preference_tableau(&self) -> Result<(), ApiError> {
        self.appel(Methode::Delete, "/assiduite/tableau/preference", None)
            .await
    }

    /// `POST /assiduite/alertes/{id}/prendre` : « Je prends » (verrou visible de 15 minutes).
    pub async fn alerte_prendre(&self, id: Uuid) -> Result<AlerteDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/assiduite/alertes/{id}/prendre"),
            Some(json!({})),
        )
        .await
    }

    /// `POST /assiduite/alertes/{id}/traiter` : issue codée (liste fermée du serveur).
    pub async fn alerte_traiter(&self, id: Uuid, code: &str) -> Result<AlerteDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/assiduite/alertes/{id}/traiter"),
            Some(json!({ "code": code })),
        )
        .await
    }

    /// `POST /assiduite/eleves/{id}/prevenir-maintenant` : avis urgent aux responsables (raison codée).
    pub async fn prevenir_maintenant(
        &self,
        eleve_id: Uuid,
        raison_code: &str,
    ) -> Result<ResultatPrevenirDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/assiduite/eleves/{eleve_id}/prevenir-maintenant"),
            Some(json!({ "raison_code": raison_code })),
        )
        .await
    }

    /// `POST /assiduite/eleves/{id}/vigilance` (pose) ou `…/vigilance/lever` : drapeau sans motif.
    pub async fn vigilance(&self, eleve_id: Uuid, poser: bool) -> Result<(), ApiError> {
        let fin = if poser { "" } else { "/lever" };
        let _: Value = self
            .appel(
                Methode::Post,
                &format!("/assiduite/eleves/{eleve_id}/vigilance{fin}"),
                Some(json!({})),
            )
            .await?;
        Ok(())
    }

    /// `POST /appels/{id}/arrivees-vs` : la vie scolaire confirme l'arrivée d'un exclu de cours.
    pub async fn confirmer_arrivee_vs(
        &self,
        appel_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/appels/{appel_id}/arrivees-vs"),
            Some(json!({ "eleve_id": eleve_id })),
        )
        .await
    }

    /// `POST /assiduite/appels/a-la-place` : fait l'appel à la place de l'intervenant (raison codée).
    pub async fn appel_a_la_place(
        &self,
        creneau_id: Uuid,
        jour: NaiveDate,
        raison_code: &str,
    ) -> Result<Uuid, ApiError> {
        let r: Value = self
            .appel(
                Methode::Post,
                "/assiduite/appels/a-la-place",
                Some(json!({ "creneau_id": creneau_id, "jour": jour, "raison_code": raison_code })),
            )
            .await?;
        r.get("appel_id")
            .and_then(Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| ApiError::Decodage("appel_id absent".into()))
    }

    /// `POST /assiduite/contacts` : trace un contact avec la famille (la note interne n'est pas envoyée d'ici).
    pub async fn tracer_contact(&self, c: &TracerContactBody) -> Result<(), ApiError> {
        let _: Value = self
            .appel(Methode::Post, "/assiduite/contacts", Some(corps(c)?))
            .await?;
        Ok(())
    }
}
