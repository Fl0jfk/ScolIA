//! Service des admissions, préinscriptions, inscriptions, réinscriptions et imports
//! (socle 2, lot S2-3) : routes `/inscriptions/*`, `/preinscription/*`, `/imports/*`
//! d'`ent-api`. Les droits, la vérification de l'e-mail, les transitions et les effets
//! d'une validation restent décidés par le serveur ; ce client transporte.
//!
//! Deux modes :
//! - personnel et responsables connectés : méthodes de [`ApiClient`] (cookie ou Bearer) ;
//! - famille en préinscription (sans compte) : [`ClientPreinscription`], appels directs
//!   (même origine) avec le jeton remis après vérification, **gardé en mémoire seulement**.

use ent_types::inscriptions::*;
use uuid::Uuid;

use crate::{transport, ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    pub async fn campagnes_inscription(&self) -> Result<Vec<CampagneDto>, ApiError> {
        self.appel(Methode::Get, "/inscriptions/campagnes", None)
            .await
    }

    pub async fn creer_campagne(&self, b: &CampagneBody) -> Result<CampagneDto, ApiError> {
        self.appel(Methode::Post, "/inscriptions/campagnes", Some(corps(b)?))
            .await
    }

    pub async fn ouvrir_campagne(&self, id: Uuid) -> Result<OuvertureCampagneDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/campagnes/{id}/ouvrir"),
            None,
        )
        .await
    }

    pub async fn fermer_campagne(&self, id: Uuid) -> Result<CampagneDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/campagnes/{id}/fermer"),
            None,
        )
        .await
    }

    pub async fn dossiers_inscription(
        &self,
        campagne_id: Option<Uuid>,
    ) -> Result<Vec<DossierResumeDto>, ApiError> {
        let chemin = match campagne_id {
            Some(c) => format!("/inscriptions/dossiers?campagne_id={c}"),
            None => "/inscriptions/dossiers".to_string(),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    pub async fn dossier_inscription(&self, id: Uuid) -> Result<DossierDto, ApiError> {
        self.appel(Methode::Get, &format!("/inscriptions/dossiers/{id}"), None)
            .await
    }

    pub async fn changer_statut_dossier(
        &self,
        id: Uuid,
        b: &StatutBody,
    ) -> Result<DossierDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/dossiers/{id}/statut"),
            Some(corps(b)?),
        )
        .await
    }

    pub async fn decider_piece(
        &self,
        id: Uuid,
        code: &str,
        b: &PieceDecisionBody,
    ) -> Result<DossierDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/dossiers/{id}/pieces/{code}"),
            Some(corps(b)?),
        )
        .await
    }

    pub async fn commenter_dossier(
        &self,
        id: Uuid,
        b: &CommentaireBody,
    ) -> Result<DossierDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/dossiers/{id}/commentaires"),
            Some(corps(b)?),
        )
        .await
    }

    pub async fn relancer_dossier(&self, id: Uuid) -> Result<serde_json::Value, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/dossiers/{id}/relancer"),
            None,
        )
        .await
    }

    pub async fn decider_dossier(
        &self,
        id: Uuid,
        b: &DecisionBody,
    ) -> Result<DossierDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/dossiers/{id}/decision"),
            Some(corps(b)?),
        )
        .await
    }

    pub async fn valider_dossier(
        &self,
        id: Uuid,
        b: &ValidationBody,
    ) -> Result<ValidationDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/dossiers/{id}/valider"),
            Some(corps(b)?),
        )
        .await
    }

    /// Responsable connecté : dossiers de réinscription de ses enfants.
    pub async fn mes_reinscriptions(&self) -> Result<Vec<DossierDto>, ApiError> {
        self.appel(Methode::Get, "/inscriptions/famille/reinscriptions", None)
            .await
    }

    pub async fn confirmer_reinscription(
        &self,
        id: Uuid,
        b: &ConfirmationReinscriptionBody,
    ) -> Result<DossierDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/famille/reinscriptions/{id}/confirmer"),
            Some(corps(b)?),
        )
        .await
    }

    pub async fn annoncer_depart(&self, id: Uuid, b: &DepartBody) -> Result<DossierDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/inscriptions/famille/reinscriptions/{id}/depart"),
            Some(corps(b)?),
        )
        .await
    }

    pub async fn previsualiser_import(&self, b: &ImportBody) -> Result<LotImportDto, ApiError> {
        self.appel(Methode::Post, "/imports/eleves", Some(corps(b)?))
            .await
    }

    pub async fn appliquer_import(&self, lot_id: Uuid) -> Result<LotImportDto, ApiError> {
        self.appel(Methode::Post, &format!("/imports/{lot_id}/appliquer"), None)
            .await
    }
}

/// Client de la famille en préinscription : routes publiques puis jeton vérifié.
#[derive(Clone, Default)]
pub struct ClientPreinscription {
    base: String,
    jeton: Option<String>,
}

impl std::fmt::Debug for ClientPreinscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientPreinscription")
            .field("base", &self.base)
            .field("jeton", &self.jeton.as_ref().map(|_| "<masqué>"))
            .finish()
    }
}

impl ClientPreinscription {
    /// `base` vide = même origine (navigateur).
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            jeton: None,
        }
    }

    pub fn avec_jeton(&self, jeton: String) -> Self {
        Self {
            base: self.base.clone(),
            jeton: Some(jeton),
        }
    }

    pub fn est_verifie(&self) -> bool {
        self.jeton.is_some()
    }

    async fn appel<T: serde::de::DeserializeOwned>(
        &self,
        methode: Methode,
        chemin: &str,
        corps: Option<serde_json::Value>,
    ) -> Result<T, ApiError> {
        let auth = match self.jeton.as_deref() {
            Some(j) => transport::Auth::Preinscription(j),
            None => transport::Auth::Aucun,
        };
        let (statut, texte) =
            transport::envoyer(methode, &format!("{}{chemin}", self.base), auth, corps).await?;
        if (200..300).contains(&statut) {
            let texte = if texte.trim().is_empty() {
                "null".into()
            } else {
                texte
            };
            serde_json::from_str(&texte).map_err(|e| ApiError::Decodage(e.to_string()))
        } else {
            Err(ApiError::depuis_statut(statut, texte))
        }
    }

    /// Étape 1 (publique) : réponse identique que l'adresse soit connue ou non.
    /// `etablissement_id` : ignoré quand l'adresse du site désigne l'établissement (D-012).
    pub async fn demarrer(
        &self,
        email: &str,
        etablissement_id: Option<Uuid>,
    ) -> Result<DemarrerDto, ApiError> {
        let b = DemarrerBody {
            email: email.into(),
            etablissement_id,
        };
        self.appel(Methode::Post, "/preinscription/demarrer", Some(corps(&b)?))
            .await
    }

    /// Étape 2 : code reçu → client vérifié (jeton en mémoire).
    pub async fn verifier(&self, demande_id: Uuid, code: &str) -> Result<Self, ApiError> {
        let b = VerifierBody {
            demande_id,
            code: code.into(),
        };
        let j: JetonPreinscriptionDto = self
            .appel(Methode::Post, "/preinscription/verifier", Some(corps(&b)?))
            .await?;
        Ok(self.avec_jeton(j.jeton))
    }

    pub async fn espace(&self) -> Result<EspacePreinscriptionDto, ApiError> {
        self.appel(Methode::Get, "/preinscription/moi", None).await
    }

    pub async fn deposer(&self, b: &PreinscriptionBody) -> Result<DossierDto, ApiError> {
        self.appel(Methode::Post, "/preinscription/dossiers", Some(corps(b)?))
            .await
    }

    pub async fn dossier(&self, id: Uuid) -> Result<DossierDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/preinscription/dossiers/{id}"),
            None,
        )
        .await
    }

    pub async fn deposer_piece(
        &self,
        id: Uuid,
        code: &str,
        nom_fichier: &str,
    ) -> Result<DossierDto, ApiError> {
        let b = PieceDeposeeBody {
            nom_fichier: nom_fichier.into(),
        };
        self.appel(
            Methode::Post,
            &format!("/preinscription/dossiers/{id}/pieces/{code}"),
            Some(corps(&b)?),
        )
        .await
    }

    pub async fn coordonnees_bancaires(
        &self,
        id: Uuid,
    ) -> Result<CoordonneesBancairesDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/preinscription/dossiers/{id}/banque"),
            None,
        )
        .await
    }

    pub async fn demander_mandat(
        &self,
        id: Uuid,
        b: &MandatBody,
    ) -> Result<MandatDemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/preinscription/dossiers/{id}/mandat"),
            Some(corps(b)?),
        )
        .await
    }

    pub async fn signer_mandat(&self, id: Uuid) -> Result<MandatDemandeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/preinscription/dossiers/{id}/mandat/signer"),
            None,
        )
        .await
    }

    /// Invitation reçue par e-mail (publique).
    pub async fn accepter_invitation(
        &self,
        jeton: &str,
    ) -> Result<InvitationAccepteeDto, ApiError> {
        let b = InvitationBody {
            jeton: jeton.into(),
        };
        self.appel(
            Methode::Post,
            "/inscriptions/invitations/accepter",
            Some(corps(&b)?),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_jeton_de_preinscription_n_apparait_pas_dans_debug() {
        let c = ClientPreinscription::new("http://x").avec_jeton("SECRET-PREINS".into());
        assert!(c.est_verifie());
        assert!(!format!("{c:?}").contains("SECRET-PREINS"));
        assert!(!ClientPreinscription::new("").est_verifie());
    }
}
