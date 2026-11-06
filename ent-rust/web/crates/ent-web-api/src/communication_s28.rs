//! Communication complète (socle 2, lot S2-8) : préférences de notification, campagnes,
//! rencontres parents-professeurs, formulaires et consentements. Routes d'`ent-api/src/
//! communication_s28.rs`. Les droits (`campagnes.lancer`, `rencontres.gerer`,
//! `formulaires.gerer`…) et les liens des familles sont vérifiés par le serveur.

use ent_types::communication::{
    ApercuCampagneDto, CampagneDto, ConfirmationLancementBody, ConsentementEleveDto,
    EnseignantClasseDto, EnseignantCreneauxDto, EnvoiDto, EtatConsentementDto, FormulaireDto,
    FormulaireFamilleDto, LigneBilanDto, LignePlanningDto, NouveauFormulaireBody,
    NouveauxCreneauxBody, NouvelleCampagneBody, NouvelleRencontreBody, PlafondsDto, PreferencesDto,
    RencontreDto, RencontreFamilleDto, ReponseFormulaireBody, ResultatsFormulaireDto,
};
use serde_json::json;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    // ----- Notifications -------------------------------------------------------

    /// `GET /notifications/preferences`.
    pub async fn preferences_notification(&self) -> Result<PreferencesDto, ApiError> {
        self.appel(Methode::Get, "/notifications/preferences", None)
            .await
    }

    /// `PUT /notifications/preferences`.
    pub async fn definir_preferences_notification(
        &self,
        p: &PreferencesDto,
    ) -> Result<PreferencesDto, ApiError> {
        self.appel(Methode::Put, "/notifications/preferences", Some(corps(p)?))
            .await
    }

    /// `GET /notifications/envois` : mes derniers envois (traçabilité).
    pub async fn mes_envois_notification(&self) -> Result<Vec<EnvoiDto>, ApiError> {
        self.appel(Methode::Get, "/notifications/envois", None)
            .await
    }

    /// `GET /notifications/plafonds` (`notifications.gerer`).
    pub async fn plafonds_notification(&self) -> Result<PlafondsDto, ApiError> {
        self.appel(Methode::Get, "/notifications/plafonds", None)
            .await
    }

    /// `PUT /notifications/plafonds`.
    pub async fn definir_plafonds_notification(
        &self,
        p: &PlafondsDto,
    ) -> Result<PlafondsDto, ApiError> {
        self.appel(Methode::Put, "/notifications/plafonds", Some(corps(p)?))
            .await
    }

    // ----- Campagnes -----------------------------------------------------------

    /// `GET /campagnes`.
    pub async fn campagnes(&self) -> Result<Vec<CampagneDto>, ApiError> {
        self.appel(Methode::Get, "/campagnes", None).await
    }

    /// `POST /campagnes` : brouillon.
    pub async fn creer_campagne_information(&self, b: &NouvelleCampagneBody) -> Result<CampagneDto, ApiError> {
        self.appel(Methode::Post, "/campagnes", Some(corps(b)?))
            .await
    }

    /// `POST /campagnes/{id}/apercu` : calcule les destinataires (n'envoie rien).
    pub async fn apercu_campagne(&self, id: Uuid) -> Result<ApercuCampagneDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/campagnes/{id}/apercu"),
            Some(json!({})),
        )
        .await
    }

    /// `POST /campagnes/{id}/lancer` : exige l'empreinte de l'aperçu confirmé.
    pub async fn lancer_campagne(
        &self,
        id: Uuid,
        b: &ConfirmationLancementBody,
    ) -> Result<CampagneDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/campagnes/{id}/lancer"),
            Some(corps(b)?),
        )
        .await
    }

    /// `POST /campagnes/{id}/annuler`.
    pub async fn annuler_campagne(&self, id: Uuid) -> Result<CampagneDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/campagnes/{id}/annuler"),
            Some(json!({})),
        )
        .await
    }

    /// `GET /campagnes/{id}/bilan` : envois par canal et statut.
    pub async fn bilan_campagne(&self, id: Uuid) -> Result<Vec<LigneBilanDto>, ApiError> {
        self.appel(Methode::Get, &format!("/campagnes/{id}/bilan"), None)
            .await
    }

    // ----- Rencontres ----------------------------------------------------------

    /// `GET /rencontres` (organisateur).
    pub async fn rencontres(&self) -> Result<Vec<RencontreDto>, ApiError> {
        self.appel(Methode::Get, "/rencontres", None).await
    }

    /// `POST /rencontres`.
    pub async fn creer_rencontre(
        &self,
        b: &NouvelleRencontreBody,
    ) -> Result<RencontreDto, ApiError> {
        self.appel(Methode::Post, "/rencontres", Some(corps(b)?))
            .await
    }

    /// `GET /rencontres/classes/{id}/enseignants`.
    pub async fn enseignants_de_classe(
        &self,
        classe: Uuid,
    ) -> Result<Vec<EnseignantClasseDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/rencontres/classes/{classe}/enseignants"),
            None,
        )
        .await
    }

    /// `POST /rencontres/{id}/creneaux`.
    pub async fn ajouter_creneaux(
        &self,
        id: Uuid,
        b: &NouveauxCreneauxBody,
    ) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Post,
                &format!("/rencontres/{id}/creneaux"),
                Some(corps(b)?),
            )
            .await?;
        Ok(())
    }

    /// `POST /rencontres/{id}/ouvrir` (prévient les familles).
    pub async fn ouvrir_rencontre(&self, id: Uuid) -> Result<RencontreDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rencontres/{id}/ouvrir"),
            Some(json!({})),
        )
        .await
    }

    /// `POST /rencontres/{id}/cloturer`.
    pub async fn cloturer_rencontre(&self, id: Uuid) -> Result<RencontreDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rencontres/{id}/cloturer"),
            Some(json!({})),
        )
        .await
    }

    /// `GET /rencontres/{id}/planning` (organisateur).
    pub async fn planning_rencontre(&self, id: Uuid) -> Result<Vec<LignePlanningDto>, ApiError> {
        self.appel(Methode::Get, &format!("/rencontres/{id}/planning"), None)
            .await
    }

    /// `GET /rencontres/mes-creneaux` (enseignant).
    pub async fn mes_creneaux_rencontre(&self) -> Result<Vec<LignePlanningDto>, ApiError> {
        self.appel(Methode::Get, "/rencontres/mes-creneaux", None)
            .await
    }

    /// `GET /famille/rencontres`.
    pub async fn rencontres_famille(&self) -> Result<Vec<RencontreFamilleDto>, ApiError> {
        self.appel(Methode::Get, "/famille/rencontres", None).await
    }

    /// `GET /famille/rencontres/{id}/enfants/{eleve}/creneaux`.
    pub async fn creneaux_famille(
        &self,
        rencontre: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<EnseignantCreneauxDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/famille/rencontres/{rencontre}/enfants/{eleve}/creneaux"),
            None,
        )
        .await
    }

    /// `POST /rencontres/creneaux/{id}/reserver`.
    pub async fn reserver_creneau(
        &self,
        creneau: Uuid,
        eleve: Uuid,
    ) -> Result<LignePlanningDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rencontres/creneaux/{creneau}/reserver"),
            Some(json!({ "eleve_id": eleve })),
        )
        .await
    }

    /// `POST /rencontres/creneaux/{id}/annuler`.
    pub async fn annuler_reservation(&self, creneau: Uuid) -> Result<LignePlanningDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rencontres/creneaux/{creneau}/annuler"),
            Some(json!({})),
        )
        .await
    }

    // ----- Formulaires et consentements ------------------------------------------

    /// `GET /formulaires` (gestion).
    pub async fn formulaires(&self) -> Result<Vec<FormulaireDto>, ApiError> {
        self.appel(Methode::Get, "/formulaires", None).await
    }

    /// `POST /formulaires` : brouillon.
    pub async fn creer_formulaire(
        &self,
        b: &NouveauFormulaireBody,
    ) -> Result<FormulaireDto, ApiError> {
        self.appel(Methode::Post, "/formulaires", Some(corps(b)?))
            .await
    }

    /// `POST /formulaires/{id}/publier`.
    pub async fn publier_formulaire(&self, id: Uuid) -> Result<FormulaireDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/formulaires/{id}/publier"),
            Some(json!({})),
        )
        .await
    }

    /// `POST /formulaires/{id}/archiver`.
    pub async fn archiver_formulaire(&self, id: Uuid) -> Result<FormulaireDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/formulaires/{id}/archiver"),
            Some(json!({})),
        )
        .await
    }

    /// `GET /formulaires/{id}/resultats` : comptages, jamais d'identité.
    pub async fn resultats_formulaire(&self, id: Uuid) -> Result<ResultatsFormulaireDto, ApiError> {
        self.appel(Methode::Get, &format!("/formulaires/{id}/resultats"), None)
            .await
    }

    /// `GET /formulaires/mes` : formulaires à traiter pour mes enfants.
    pub async fn formulaires_famille(&self) -> Result<Vec<FormulaireFamilleDto>, ApiError> {
        self.appel(Methode::Get, "/formulaires/mes", None).await
    }

    /// `POST /formulaires/{id}/repondre`.
    pub async fn repondre_formulaire(
        &self,
        id: Uuid,
        b: &ReponseFormulaireBody,
    ) -> Result<EtatConsentementDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/formulaires/{id}/repondre"),
            Some(corps(b)?),
        )
        .await
    }

    /// `GET /consentements/{objet}/classes/{classe}` : qui consent (un seul objet).
    pub async fn consentements_classe(
        &self,
        objet: &str,
        classe: Uuid,
    ) -> Result<Vec<ConsentementEleveDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/consentements/{objet}/classes/{classe}"),
            None,
        )
        .await
    }
}
