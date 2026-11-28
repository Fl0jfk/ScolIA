//! Socle 27, palier G5 : registre des violations de données. Routes `/rgpd/violations/*` d'`ent-api` ; types
//! d'`ent-types::rgpd_violations`.
//!
//! L'export CSV et les fiches PDF (`…/registre.pdf`, `…/{id}/fiche.pdf`) n'ont pas de méthode : ce sont de simples
//! liens (cookie de session).

use ent_types::rgpd_violations::*;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `POST /rgpd/violations/declarations` : déclarer un incident (tout personnel).
    pub async fn violations_declarer(
        &self,
        b: &DeclarationBody,
    ) -> Result<MaDeclarationDto, ApiError> {
        self.appel(Methode::Post, "/rgpd/violations/declarations", corps(b)?)
            .await
    }

    /// `GET /rgpd/violations/declarations` : mes déclarations (référence et avancement seulement).
    pub async fn violations_mes_declarations(&self) -> Result<Vec<MaDeclarationDto>, ApiError> {
        self.appel(Methode::Get, "/rgpd/violations/declarations", None)
            .await
    }

    /// `GET /rgpd/violations` : le registre, filtré.
    pub async fn violations_registre(
        &self,
        f: &FiltreRegistreQuery,
    ) -> Result<Vec<ViolationLigneDto>, ApiError> {
        let mut q = Vec::new();
        if let Some(a) = f.annee {
            q.push(format!("annee={a}"));
        }
        if let Some(e) = f.etat.as_deref().filter(|e| !e.is_empty()) {
            q.push(format!("etat={e}"));
        }
        if let Some(n) = f.nature_incident.as_deref().filter(|n| !n.is_empty()) {
            q.push(format!("nature_incident={n}"));
        }
        if f.a_notifier == Some(true) {
            q.push("a_notifier=true".into());
        }
        let uri = if q.is_empty() {
            "/rgpd/violations".to_string()
        } else {
            format!("/rgpd/violations?{}", q.join("&"))
        };
        self.appel(Methode::Get, &uri, None).await
    }

    /// `POST /rgpd/violations` : saisie par un gestionnaire (prise de connaissance antérieure possible).
    pub async fn violations_saisir(
        &self,
        b: &SaisieViolationBody,
    ) -> Result<ViolationDto, ApiError> {
        self.appel(Methode::Post, "/rgpd/violations", corps(b)?)
            .await
    }

    /// `GET /rgpd/violations/{id}` : la fiche.
    pub async fn violations_fiche(&self, id: Uuid) -> Result<ViolationDto, ApiError> {
        self.appel(Methode::Get, &format!("/rgpd/violations/{id}"), None)
            .await
    }

    /// `GET /rgpd/violations/{id}/brouillon-cnil` : brouillon de notification, rubrique par rubrique.
    pub async fn violations_brouillon_cnil(&self, id: Uuid) -> Result<BrouillonCnilDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/rgpd/violations/{id}/brouillon-cnil"),
            None,
        )
        .await
    }

    /// `GET /rgpd/violations/{id}/information` : aperçu avant d'informer les personnes.
    pub async fn violations_apercu_information(
        &self,
        id: Uuid,
    ) -> Result<ApercuInformationDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/rgpd/violations/{id}/information"),
            None,
        )
        .await
    }

    async fn violations_ecrire<B: serde::Serialize>(
        &self,
        id: Uuid,
        geste: &str,
        b: &B,
    ) -> Result<ViolationDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/rgpd/violations/{id}/{geste}"),
            corps(b)?,
        )
        .await
    }

    /// `POST …/qualification`.
    pub async fn violations_qualifier(
        &self,
        id: Uuid,
        b: &QualificationViolationBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "qualification", b).await
    }

    /// `POST …/mesures`.
    pub async fn violations_ajouter_mesure(
        &self,
        id: Uuid,
        b: &MesureBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "mesures", b).await
    }

    /// `POST …/avis-dpo`.
    pub async fn violations_avis_dpo(
        &self,
        id: Uuid,
        b: &TexteBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "avis-dpo", b).await
    }

    /// `POST …/risque` (direction).
    pub async fn violations_decider_risque(
        &self,
        id: Uuid,
        b: &RisqueBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "risque", b).await
    }

    /// `POST …/notification-cnil`.
    pub async fn violations_notifier_cnil(
        &self,
        id: Uuid,
        b: &NotificationCnilBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "notification-cnil", b).await
    }

    /// `POST …/notification-complementaire`.
    pub async fn violations_complement_cnil(
        &self,
        id: Uuid,
        b: &ComplementCnilBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "notification-complementaire", b)
            .await
    }

    /// `POST …/non-notification` (direction).
    pub async fn violations_non_notification(
        &self,
        id: Uuid,
        b: &TexteBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "non-notification", b).await
    }

    /// `POST …/personnes` : calcul des personnes concernées.
    pub async fn violations_calculer_personnes(
        &self,
        id: Uuid,
        b: &PersonnesBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "personnes", b).await
    }

    /// `POST …/information` (direction) : confirmée par le nombre de destinataires de l'aperçu.
    pub async fn violations_informer(
        &self,
        id: Uuid,
        b: &InformationBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "information", b).await
    }

    /// `POST …/non-information` (direction).
    pub async fn violations_non_information(
        &self,
        id: Uuid,
        b: &TexteBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "non-information", b).await
    }

    /// `POST …/cloture` : clôture avec retour d'expérience.
    pub async fn violations_clore(
        &self,
        id: Uuid,
        b: &TexteBody,
    ) -> Result<ViolationDto, ApiError> {
        self.violations_ecrire(id, "cloture", b).await
    }
}
