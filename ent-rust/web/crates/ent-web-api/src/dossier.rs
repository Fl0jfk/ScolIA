//! Service dossier élève et personnes (socle 2, lot S2-1) : routes `/dossier/*`
//! d'`ent-api`. Les droits, la minimisation et les règles (parts = 100 %, motif
//! de suspension…) restent décidés par le serveur ; ce client transporte.

use ent_types::dossier::{
    ContactBody, ContactUrgenceDto, EntreeChronologieDto, FicheEleveDto, LienBody, PayeurPartDto,
    PayeursBody, PersonneBody, PersonneDto, ResponsableDto, SuspensionBody,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /dossier/eleves/{id}/fiche` : fiche 360° vue par la personne connectée.
    pub async fn fiche_dossier(&self, eleve_id: Uuid) -> Result<FicheEleveDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/dossier/eleves/{eleve_id}/fiche"),
            None,
        )
        .await
    }

    /// `GET /dossier/eleves/{id}/chronologie?limite=`.
    pub async fn chronologie_eleve(
        &self,
        eleve_id: Uuid,
        limite: usize,
    ) -> Result<Vec<EntreeChronologieDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/dossier/eleves/{eleve_id}/chronologie?limite={limite}"),
            None,
        )
        .await
    }

    /// `POST /dossier/personnes` : nouvelle personne (avec ou sans e-mail).
    pub async fn creer_personne(&self, p: &PersonneBody) -> Result<PersonneDto, ApiError> {
        self.appel(Methode::Post, "/dossier/personnes", Some(corps(p)?))
            .await
    }

    /// `POST /dossier/eleves/{id}/responsables` : lier une personne à l'élève.
    pub async fn lier_responsable(
        &self,
        eleve_id: Uuid,
        lien: &LienBody,
    ) -> Result<ResponsableDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/dossier/eleves/{eleve_id}/responsables"),
            Some(corps(lien)?),
        )
        .await
    }

    /// `POST /dossier/liens/{id}/suspendre` (motif obligatoire).
    pub async fn suspendre_lien(
        &self,
        lien_id: Uuid,
        motif: String,
        reference: Option<String>,
    ) -> Result<ResponsableDto, ApiError> {
        let c = corps(&SuspensionBody { motif, reference })?;
        self.appel(
            Methode::Post,
            &format!("/dossier/liens/{lien_id}/suspendre"),
            Some(c),
        )
        .await
    }

    /// `POST /dossier/liens/{id}/retablir`.
    pub async fn retablir_lien(&self, lien_id: Uuid) -> Result<ResponsableDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/dossier/liens/{lien_id}/retablir"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `PUT /dossier/eleves/{id}/payeurs` : liste complète des payeurs.
    pub async fn definir_payeurs(
        &self,
        eleve_id: Uuid,
        payeurs: Vec<(Uuid, i16)>,
    ) -> Result<Vec<ResponsableDto>, ApiError> {
        let c = corps(&PayeursBody {
            payeurs: payeurs
                .into_iter()
                .map(|(lien_id, part)| PayeurPartDto { lien_id, part })
                .collect(),
        })?;
        self.appel(
            Methode::Put,
            &format!("/dossier/eleves/{eleve_id}/payeurs"),
            Some(c),
        )
        .await
    }

    /// `POST /dossier/eleves/{id}/contacts` : contact d'urgence (proposition si famille).
    pub async fn ajouter_contact(
        &self,
        eleve_id: Uuid,
        contact: &ContactBody,
    ) -> Result<ContactUrgenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/dossier/eleves/{eleve_id}/contacts"),
            Some(corps(contact)?),
        )
        .await
    }

    /// `POST /dossier/contacts/{id}/valider`.
    pub async fn valider_contact(&self, contact_id: Uuid) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/dossier/contacts/{contact_id}/valider"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `DELETE /dossier/contacts/{id}`.
    pub async fn retirer_contact(&self, contact_id: Uuid) -> Result<(), ApiError> {
        self.appel(
            Methode::Delete,
            &format!("/dossier/contacts/{contact_id}"),
            None,
        )
        .await
    }
}
