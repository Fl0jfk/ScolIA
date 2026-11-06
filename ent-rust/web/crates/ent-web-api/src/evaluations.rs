//! Évaluations, conseils de classe, bulletins et livrets (socle 2, lot S2-7).
//! Routes `/evaluations/*` d'`ent-api/src/evaluations.rs`. Les droits (`notes.saisir`,
//! `conseils.gerer`, `bulletins.publier`, `livrets.exporter`…) et **tous les calculs de
//! moyennes** sont faits par le serveur : le client n'affiche que ce qu'il reçoit.

use ent_types::evaluations::{
    AppreciationBody, AppreciationDto, AvisVoeuBody, BulletinDetailDto, BulletinDto, ConseilDto,
    CreerConseilBody, CreerDevoirGrilleBody, DecisionConseilBody, DecisionConseilDto,
    DevoirGrilleDto, ExportLivretDto, GenererBulletinsBody, GrilleDto, PeriodeNotesDto,
    PreparationConseilDto, PublierBulletinsBody, RapportBulletinsDto, RapportSaisieDto,
    ReleveEleveDto, SaisieGrilleBody, ServiceEvaluationDto, VoeuAvisDto, VueClasseDto,
};
use serde_json::json;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /evaluations/services` : services que la personne peut évaluer ou consulter.
    pub async fn services_evaluables(&self) -> Result<Vec<ServiceEvaluationDto>, ApiError> {
        self.appel(Methode::Get, "/evaluations/services", None)
            .await
    }

    /// `GET /evaluations/periodes`.
    pub async fn periodes_notes(&self) -> Result<Vec<PeriodeNotesDto>, ApiError> {
        self.appel(Methode::Get, "/evaluations/periodes", None)
            .await
    }

    /// `PUT /evaluations/periodes/{id}/statut` (`ouverte`, `partielle_appreciations`, `fermee`).
    pub async fn changer_statut_periode(
        &self,
        periode: Uuid,
        statut: &str,
    ) -> Result<PeriodeNotesDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/evaluations/periodes/{periode}/statut"),
            Some(json!({ "statut": statut })),
        )
        .await
    }

    /// `GET /evaluations/grille?service_id=&periode_id=`.
    pub async fn grille_notes(&self, service: Uuid, periode: Uuid) -> Result<GrilleDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/evaluations/grille?service_id={service}&periode_id={periode}"),
            None,
        )
        .await
    }

    /// `POST /evaluations/devoirs`.
    pub async fn creer_devoir_grille(
        &self,
        body: &CreerDevoirGrilleBody,
    ) -> Result<DevoirGrilleDto, ApiError> {
        self.appel(Methode::Post, "/evaluations/devoirs", Some(corps(body)?))
            .await
    }

    /// `PUT /evaluations/devoirs/{id}/resultats` : seulement les cellules modifiées.
    pub async fn saisir_resultats(
        &self,
        devoir: Uuid,
        body: &SaisieGrilleBody,
    ) -> Result<RapportSaisieDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/evaluations/devoirs/{devoir}/resultats"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /evaluations/devoirs/{id}/publier`.
    pub async fn publier_devoir(&self, devoir: Uuid) -> Result<DevoirGrilleDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/evaluations/devoirs/{devoir}/publier"),
            Some(json!({})),
        )
        .await
    }

    /// `GET /evaluations/classes/{id}?periode_id=`.
    pub async fn vue_classe(&self, classe: Uuid, periode: Uuid) -> Result<VueClasseDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/evaluations/classes/{classe}?periode_id={periode}"),
            None,
        )
        .await
    }

    /// `GET /evaluations/eleves/{id}/releve?periode_id=` : évaluations publiées (familles).
    pub async fn releve_eleve(
        &self,
        eleve: Uuid,
        periode: Uuid,
    ) -> Result<ReleveEleveDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/evaluations/eleves/{eleve}/releve?periode_id={periode}"),
            None,
        )
        .await
    }

    /// `POST /evaluations/appreciations`.
    pub async fn saisir_appreciation(
        &self,
        body: &AppreciationBody,
    ) -> Result<AppreciationDto, ApiError> {
        self.appel(
            Methode::Post,
            "/evaluations/appreciations",
            Some(corps(body)?),
        )
        .await
    }

    /// `GET /evaluations/conseils`.
    pub async fn conseils_evaluation(&self) -> Result<Vec<ConseilDto>, ApiError> {
        self.appel(Methode::Get, "/evaluations/conseils", None)
            .await
    }

    /// `POST /evaluations/conseils`.
    pub async fn creer_conseil(&self, body: &CreerConseilBody) -> Result<ConseilDto, ApiError> {
        self.appel(Methode::Post, "/evaluations/conseils", Some(corps(body)?))
            .await
    }

    /// `GET /evaluations/conseils/{id}/preparation`.
    pub async fn preparation_conseil(
        &self,
        conseil: Uuid,
    ) -> Result<PreparationConseilDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/evaluations/conseils/{conseil}/preparation"),
            None,
        )
        .await
    }

    /// `PUT /evaluations/conseils/{id}/decisions`.
    pub async fn decider_conseil(
        &self,
        conseil: Uuid,
        body: &DecisionConseilBody,
    ) -> Result<DecisionConseilDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/evaluations/conseils/{conseil}/decisions"),
            Some(corps(body)?),
        )
        .await
    }

    /// `PUT /evaluations/conseils/{id}/avis-voeux`.
    pub async fn avis_voeu(
        &self,
        conseil: Uuid,
        body: &AvisVoeuBody,
    ) -> Result<VoeuAvisDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/evaluations/conseils/{conseil}/avis-voeux"),
            Some(corps(body)?),
        )
        .await
    }

    /// `GET /evaluations/eleves/{id}/orientation`.
    pub async fn orientation_eleve(&self, eleve: Uuid) -> Result<Vec<VoeuAvisDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/evaluations/eleves/{eleve}/orientation"),
            None,
        )
        .await
    }

    /// `POST /evaluations/avis/{id}/reponse` (`accord` | `desaccord`).
    pub async fn repondre_avis(&self, avis: Uuid, reponse: &str) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/evaluations/avis/{avis}/reponse"),
            Some(json!({ "reponse": reponse })),
        )
        .await
    }

    /// `GET /evaluations/bulletins?classe_id=&periode_id=`.
    pub async fn bulletins_classe(
        &self,
        classe: Uuid,
        periode: Uuid,
    ) -> Result<Vec<BulletinDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/evaluations/bulletins?classe_id={classe}&periode_id={periode}"),
            None,
        )
        .await
    }

    /// `POST /evaluations/bulletins/generer`.
    pub async fn generer_bulletins(
        &self,
        body: &GenererBulletinsBody,
    ) -> Result<RapportBulletinsDto, ApiError> {
        self.appel(
            Methode::Post,
            "/evaluations/bulletins/generer",
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /evaluations/bulletins/publier`.
    pub async fn publier_bulletins(
        &self,
        body: &PublierBulletinsBody,
    ) -> Result<RapportBulletinsDto, ApiError> {
        self.appel(
            Methode::Post,
            "/evaluations/bulletins/publier",
            Some(corps(body)?),
        )
        .await
    }

    /// `GET /evaluations/eleves/{id}/bulletins` (familles : version publiée seulement).
    pub async fn bulletins_eleve(&self, eleve: Uuid) -> Result<Vec<BulletinDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/evaluations/eleves/{eleve}/bulletins"),
            None,
        )
        .await
    }

    /// `GET /evaluations/bulletins/{id}`.
    pub async fn bulletin(&self, id: Uuid) -> Result<BulletinDetailDto, ApiError> {
        self.appel(Methode::Get, &format!("/evaluations/bulletins/{id}"), None)
            .await
    }

    /// `GET /evaluations/bulletins/{id}/pdf` : octets du PDF.
    pub async fn bulletin_pdf(&self, id: Uuid) -> Result<Vec<u8>, ApiError> {
        let (statut, octets) = crate::transport::envoyer_octets(
            &self.url(&format!("/evaluations/bulletins/{id}/pdf")),
            self.auth(),
        )
        .await?;
        if (200..300).contains(&statut) {
            Ok(octets)
        } else {
            Err(ApiError::depuis_statut(
                statut,
                String::from_utf8_lossy(&octets).into_owned(),
            ))
        }
    }

    /// `POST /evaluations/exports` (`lsu` | `lsl`).
    pub async fn exporter_livret(
        &self,
        format: &str,
        classe: Uuid,
        periode: Uuid,
    ) -> Result<ExportLivretDto, ApiError> {
        self.appel(
            Methode::Post,
            "/evaluations/exports",
            Some(json!({ "format": format, "classe_id": classe, "periode_id": periode })),
        )
        .await
    }
}
