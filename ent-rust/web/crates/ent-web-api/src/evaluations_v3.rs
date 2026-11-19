//! Évaluations, lot V3 « grille et saisie » : routes `/eva/*` d'`ent-api/src/evaluations_v3.rs`.
//! Le serveur décide des droits et calcule **toutes** les moyennes (moteur unique) ; le client n'envoie que
//! le texte tapé et la version lue de chaque case. Méthodes préfixées `eva3_` (les anciennes routes
//! `/evaluations/*` gardent les leurs).

use ent_types::evaluations_v3::{
    AnnulerEvaluationV3Body, CreerEvaluationV3Body, EnCoursCorrectionBody, EnseignementV3Dto,
    EvaluationColonneDto, ExplicationDto, GrilleV3Dto, ImportApercuDto, ImportBody,
    LigneHistoriqueV3Dto, ModifierEvaluationV3Body, PeriodeV3Dto, PublierV3Body, SaisieLotBody,
    SaisieLotRetourDto, SousMatiereV3Dto,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /eva/enseignements` : les cours accessibles (cours sans matière du référentiel compris).
    pub async fn eva3_enseignements(&self) -> Result<Vec<EnseignementV3Dto>, ApiError> {
        self.appel(Methode::Get, "/eva/enseignements", None).await
    }

    /// `GET /eva/cours/{id}/periodes`.
    pub async fn eva3_periodes(&self, cours: Uuid) -> Result<Vec<PeriodeV3Dto>, ApiError> {
        self.appel(Methode::Get, &format!("/eva/cours/{cours}/periodes"), None)
            .await
    }

    /// `GET /eva/cours/{id}/sous-matieres`.
    pub async fn eva3_sous_matieres(&self, cours: Uuid) -> Result<Vec<SousMatiereV3Dto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/eva/cours/{cours}/sous-matieres"),
            None,
        )
        .await
    }

    /// `GET /eva/grille?cours_id=&periode_id=`.
    pub async fn eva3_grille(&self, cours: Uuid, periode: Uuid) -> Result<GrilleV3Dto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/eva/grille?cours_id={cours}&periode_id={periode}"),
            None,
        )
        .await
    }

    /// `GET /eva/grille/explication` : la moyenne d'un élève, expliquée ligne à ligne.
    pub async fn eva3_explication(
        &self,
        cours: Uuid,
        periode: Uuid,
        eleve: Uuid,
    ) -> Result<ExplicationDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!(
                "/eva/grille/explication?cours_id={cours}&periode_id={periode}&eleve_id={eleve}"
            ),
            None,
        )
        .await
    }

    /// `POST /eva/saisie` : un lot de cases ; chaque case revient avec son issue (écrite, conflit, refusée…).
    pub async fn eva3_saisir(&self, body: &SaisieLotBody) -> Result<SaisieLotRetourDto, ApiError> {
        self.appel(Methode::Post, "/eva/saisie", Some(corps(body)?))
            .await
    }

    /// `POST /eva/evaluations`.
    pub async fn eva3_creer(
        &self,
        body: &CreerEvaluationV3Body,
    ) -> Result<EvaluationColonneDto, ApiError> {
        self.appel(Methode::Post, "/eva/evaluations", Some(corps(body)?))
            .await
    }

    /// `PUT /eva/evaluations/{id}`.
    pub async fn eva3_modifier(
        &self,
        id: Uuid,
        body: &ModifierEvaluationV3Body,
    ) -> Result<EvaluationColonneDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/eva/evaluations/{id}"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /eva/evaluations/{id}/annuler`.
    pub async fn eva3_annuler(
        &self,
        id: Uuid,
        body: &AnnulerEvaluationV3Body,
    ) -> Result<EvaluationColonneDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/eva/evaluations/{id}/annuler"),
            Some(corps(body)?),
        )
        .await
    }

    /// `POST /eva/evaluations/{id}/publication`.
    pub async fn eva3_publication(
        &self,
        id: Uuid,
        body: &PublierV3Body,
    ) -> Result<EvaluationColonneDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/eva/evaluations/{id}/publication"),
            Some(corps(body)?),
        )
        .await
    }

    /// `PUT /eva/evaluations/{id}/en-cours-de-correction`.
    pub async fn eva3_en_cours_de_correction(
        &self,
        id: Uuid,
        en_cours: bool,
    ) -> Result<EvaluationColonneDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/eva/evaluations/{id}/en-cours-de-correction"),
            Some(corps(&EnCoursCorrectionBody { en_cours })?),
        )
        .await
    }

    /// `GET /eva/evaluations/{id}/eleves/{eleve}/historique`.
    pub async fn eva3_historique(
        &self,
        evaluation: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<LigneHistoriqueV3Dto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/eva/evaluations/{evaluation}/eleves/{eleve}/historique"),
            None,
        )
        .await
    }

    /// `POST /eva/import/apercu` : rien n'est écrit.
    pub async fn eva3_apercu_import(&self, body: &ImportBody) -> Result<ImportApercuDto, ApiError> {
        self.appel(Methode::Post, "/eva/import/apercu", Some(corps(body)?))
            .await
    }

    /// `POST /eva/import/confirmer` : avec l'empreinte de l'aperçu.
    pub async fn eva3_confirmer_import(
        &self,
        body: &ImportBody,
    ) -> Result<SaisieLotRetourDto, ApiError> {
        self.appel(Methode::Post, "/eva/import/confirmer", Some(corps(body)?))
            .await
    }
}
