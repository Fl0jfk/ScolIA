//! Absences des personnels (socle 15, écrans A11d à A11f) : client des routes d'`ent-api` pour les heures
//! constatées, les états mensuels, la paie de l'OGEC, la DSN, le remplacement de courte durée (RCD) et le
//! paramétrage. Routes `/personnels/heures`, `/personnels/etats-heures`, `/personnels/paie`, `/personnels/rcd`,
//! `/personnels/absences/{parametres,edt-reference,circuit-regles}` (`absences_personnels_heures.rs`,
//! `absences_personnels_rcd.rs`, `absences_personnels_impact.rs`, `absences_personnels.rs`). Les types sont ceux
//! d'`ent-types`.
//!
//! Aucun droit n'est décidé ici : le serveur répond 403 / 404 hors du cercle (§23, périmètre d'employeur).
//! Les méthodes portent le préfixe `absp_` (comme `absences_personnels.rs`).

use chrono::NaiveDate;
use ent_types::absences_personnels::{
    AttesterBody, BilanPaieDto, BilanValidationDto, CompteRenduRcdBody, CompteRenduRcdDto,
    CompteurRcdDto, ContesterHeureBody, CorrigerHeureBody, CreerRegleCircuitBody, DsnSignaleBody,
    DsnSuiviDto, EdtReferenceBody, EdtReferenceDto, ElementPaieDto, EngagementRcdBody,
    EngagementRcdDto, EtatAnnuelDto, EtatMensuelDto, FigerEtatBody, FiltreHeures, HeureDto,
    MesHeuresDto, ModifierEngagementRcdBody, ModifierParametresAbsencesBody, ParametresAbsencesDto,
    PlanRcdBody, PlanRcdDto, PreparerPaieBody, PreparerRemonteeBody, ProcedurePlanRcdBody,
    RegleCircuitDto, RegularisationDto, RemonteeRcdDto, SaisirHeureBody, ValiderHeuresBody,
    VivierRcdDto,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

/// Premier jour du mois au format de l'API (`AAAA-MM-JJ`).
fn jour(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Chaîne de requête des filtres d'heures (paramètres absents omis).
pub(crate) fn requete_filtre_heures(f: &FiltreHeures) -> String {
    let mut p: Vec<String> = Vec::new();
    if let Some(a) = f.agent_id {
        p.push(format!("agent_id={a}"));
    }
    if let Some(m) = f.mois {
        p.push(format!("mois={}", jour(m)));
    }
    if let Some(n) = &f.nature {
        p.push(format!("nature={n}"));
    }
    if let Some(e) = &f.etat {
        p.push(format!("etat={e}"));
    }
    if p.is_empty() {
        String::new()
    } else {
        format!("?{}", p.join("&"))
    }
}

impl ApiClient {
    async fn absp_csv(&self, chemin: &str) -> Result<Vec<u8>, ApiError> {
        let (statut, octets) =
            crate::transport::envoyer_octets(&self.url(chemin), self.auth()).await?;
        if (200..300).contains(&statut) {
            Ok(octets)
        } else {
            Err(ApiError::depuis_statut(
                statut,
                String::from_utf8_lossy(&octets).into_owned(),
            ))
        }
    }

    // ───────────────────────── A11d : heures constatées ─────────────────────────

    /// `GET /personnels/heures/moi` : mes heures, et les séances remplacées sans heure (à attester).
    pub async fn absp_mes_heures(&self, mois: Option<NaiveDate>) -> Result<MesHeuresDto, ApiError> {
        let q = mois
            .map(|m| format!("?mois={}", jour(m)))
            .unwrap_or_default();
        self.appel(Methode::Get, &format!("/personnels/heures/moi{q}"), None)
            .await
    }

    /// `GET /personnels/heures/moi/annuel` : mon état annuel.
    pub async fn absp_mon_etat_annuel(&self) -> Result<EtatAnnuelDto, ApiError> {
        self.appel(Methode::Get, "/personnels/heures/moi/annuel", None)
            .await
    }

    /// `GET /personnels/heures/annuel/{agent}` : état annuel d'un agent (`heures.lire`).
    pub async fn absp_etat_annuel_agent(&self, agent: Uuid) -> Result<EtatAnnuelDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/personnels/heures/annuel/{agent}"),
            None,
        )
        .await
    }

    /// `GET /personnels/heures` : heures du périmètre d'employeur du lecteur.
    pub async fn absp_heures(&self, filtre: &FiltreHeures) -> Result<Vec<HeureDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/personnels/heures{}", requete_filtre_heures(filtre)),
            None,
        )
        .await
    }

    /// `POST /personnels/heures` : heure sans séance (HSE, mission, vacation, service non fait).
    pub async fn absp_saisir_heure(&self, body: &SaisirHeureBody) -> Result<HeureDto, ApiError> {
        self.appel(Methode::Post, "/personnels/heures", corps(body)?)
            .await
    }

    /// `POST /personnels/heures/attestation` : le remplaçant atteste une séance assurée sans appel.
    pub async fn absp_attester(&self, seance_id: Uuid) -> Result<HeureDto, ApiError> {
        self.appel(
            Methode::Post,
            "/personnels/heures/attestation",
            corps(&AttesterBody { seance_id })?,
        )
        .await
    }

    /// `POST /personnels/heures/validation` : valider les heures d'un mois (ou d'un agent).
    pub async fn absp_valider_heures(
        &self,
        body: &ValiderHeuresBody,
    ) -> Result<BilanValidationDto, ApiError> {
        self.appel(Methode::Post, "/personnels/heures/validation", corps(body)?)
            .await
    }

    /// `POST /personnels/heures/{id}/contestation`.
    pub async fn absp_contester_heure(
        &self,
        id: Uuid,
        body: &ContesterHeureBody,
    ) -> Result<HeureDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/heures/{id}/contestation"),
            corps(body)?,
        )
        .await
    }

    /// `POST /personnels/heures/{id}/correction` : heure non figée.
    pub async fn absp_corriger_heure(
        &self,
        id: Uuid,
        body: &CorrigerHeureBody,
    ) -> Result<HeureDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/heures/{id}/correction"),
            corps(body)?,
        )
        .await
    }

    /// `POST /personnels/heures/{id}/regularisation` : heure figée ou exportée (jamais une réécriture).
    pub async fn absp_regulariser_heure(
        &self,
        id: Uuid,
        body: &CorrigerHeureBody,
    ) -> Result<RegularisationDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/heures/{id}/regularisation"),
            corps(body)?,
        )
        .await
    }

    // ───────────────────────── A11d : états mensuels ─────────────────────────

    /// `POST /personnels/etats-heures` : figer l'état d'un employeur pour un mois terminé.
    pub async fn absp_figer_etat(&self, body: &FigerEtatBody) -> Result<EtatMensuelDto, ApiError> {
        self.appel(Methode::Post, "/personnels/etats-heures", corps(body)?)
            .await
    }

    /// `GET /personnels/etats-heures/{id}`.
    pub async fn absp_etat_mensuel(&self, id: Uuid) -> Result<EtatMensuelDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/personnels/etats-heures/{id}"),
            None,
        )
        .await
    }

    /// `POST /personnels/etats-heures/{id}/signature` : signature du chef d'établissement.
    pub async fn absp_signer_etat(&self, id: Uuid) -> Result<EtatMensuelDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/etats-heures/{id}/signature"),
            None,
        )
        .await
    }

    /// `GET /personnels/etats-heures/{id}/export.csv` (tracé par le serveur).
    pub async fn absp_etat_csv(&self, id: Uuid) -> Result<Vec<u8>, ApiError> {
        self.absp_csv(&format!("/personnels/etats-heures/{id}/export.csv"))
            .await
    }

    // ───────────────────────── A11d : paie de l'OGEC et DSN ─────────────────────────

    /// `GET /personnels/paie/elements?mois=` : éléments variables d'un mois.
    pub async fn absp_elements_paie(
        &self,
        mois: NaiveDate,
    ) -> Result<Vec<ElementPaieDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/personnels/paie/elements?mois={}", jour(mois)),
            None,
        )
        .await
    }

    /// `POST /personnels/paie/elements` : préparer les éléments d'un mois.
    pub async fn absp_preparer_paie(&self, mois: NaiveDate) -> Result<BilanPaieDto, ApiError> {
        self.appel(
            Methode::Post,
            "/personnels/paie/elements",
            corps(&PreparerPaieBody { mois })?,
        )
        .await
    }

    /// `GET /personnels/paie/export.csv?mois=` : les éléments « préparés » deviennent « envoyés ».
    pub async fn absp_paie_csv(&self, mois: NaiveDate) -> Result<Vec<u8>, ApiError> {
        self.absp_csv(&format!("/personnels/paie/export.csv?mois={}", jour(mois)))
            .await
    }

    /// `GET /personnels/paie/dsn` : arrêts à signaler en DSN dans les cinq jours.
    pub async fn absp_dsn(&self) -> Result<Vec<DsnSuiviDto>, ApiError> {
        self.appel(Methode::Get, "/personnels/paie/dsn", None).await
    }

    /// `POST /personnels/paie/dsn/{absence}/signale` : « signalé en DSN le … ».
    pub async fn absp_dsn_signale(
        &self,
        absence: Uuid,
        le: NaiveDate,
    ) -> Result<DsnSuiviDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/paie/dsn/{absence}/signale"),
            corps(&DsnSignaleBody { le })?,
        )
        .await
    }

    // ───────────────────────── A11e : plan RCD ─────────────────────────

    /// `GET /personnels/rcd/plans` : toutes les versions du plan (direction).
    pub async fn absp_plans_rcd(&self) -> Result<Vec<PlanRcdDto>, ApiError> {
        self.appel(Methode::Get, "/personnels/rcd/plans", None)
            .await
    }

    /// `GET /personnels/rcd/plan-courant` : le plan vivant de l'année, lisible de tout le personnel.
    pub async fn absp_plan_courant(&self) -> Result<Option<PlanRcdDto>, ApiError> {
        self.appel(Methode::Get, "/personnels/rcd/plan-courant", None)
            .await
    }

    /// `POST /personnels/rcd/plans` : créer le plan de l'année (trame).
    pub async fn absp_creer_plan(&self, body: &PlanRcdBody) -> Result<PlanRcdDto, ApiError> {
        self.appel(Methode::Post, "/personnels/rcd/plans", corps(body)?)
            .await
    }

    /// `PUT /personnels/rcd/plans/{id}` : modifier un brouillon.
    pub async fn absp_modifier_plan(
        &self,
        id: Uuid,
        body: &PlanRcdBody,
    ) -> Result<PlanRcdDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/personnels/rcd/plans/{id}"),
            corps(body)?,
        )
        .await
    }

    /// `POST /personnels/rcd/plans/{id}/version` : nouvelle version (l'ancienne est archivée).
    pub async fn absp_versionner_plan(
        &self,
        id: Uuid,
        body: &PlanRcdBody,
    ) -> Result<PlanRcdDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/rcd/plans/{id}/version"),
            corps(body)?,
        )
        .await
    }

    /// `POST /personnels/rcd/plans/{id}/procedure` : avis du conseil pédagogique, présentation, transmission.
    pub async fn absp_procedure_plan(
        &self,
        id: Uuid,
        body: &ProcedurePlanRcdBody,
    ) -> Result<PlanRcdDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/rcd/plans/{id}/procedure"),
            corps(body)?,
        )
        .await
    }

    /// `POST /personnels/rcd/plans/{id}/campagne` : lancer la campagne de volontariat.
    pub async fn absp_campagne_rcd(&self, id: Uuid) -> Result<PlanRcdDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/rcd/plans/{id}/campagne"),
            None,
        )
        .await
    }

    /// `POST /personnels/rcd/plans/{id}/comptes-rendus` : compte rendu d'une période.
    pub async fn absp_compte_rendu_rcd(
        &self,
        id: Uuid,
        body: &CompteRenduRcdBody,
    ) -> Result<CompteRenduRcdDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/rcd/plans/{id}/comptes-rendus"),
            corps(body)?,
        )
        .await
    }

    // ───────────────────────── A11e : volontaires et compteurs ─────────────────────────

    /// `POST /personnels/rcd/engagements` : s'engager (soi-même, ou un collègue avec `rcd.gerer`).
    pub async fn absp_engager_rcd(
        &self,
        body: &EngagementRcdBody,
    ) -> Result<EngagementRcdDto, ApiError> {
        self.appel(Methode::Post, "/personnels/rcd/engagements", corps(body)?)
            .await
    }

    /// `PUT /personnels/rcd/engagements/{id}` : modifier, ou mettre fin à date.
    pub async fn absp_modifier_engagement_rcd(
        &self,
        id: Uuid,
        body: &ModifierEngagementRcdBody,
    ) -> Result<EngagementRcdDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/personnels/rcd/engagements/{id}"),
            corps(body)?,
        )
        .await
    }

    /// `GET /personnels/rcd/mon-compteur` : mon compteur de volontaire (absent si je ne suis pas engagé).
    pub async fn absp_mon_compteur_rcd(&self) -> Result<Option<CompteurRcdDto>, ApiError> {
        self.appel(Methode::Get, "/personnels/rcd/mon-compteur", None)
            .await
    }

    /// `GET /personnels/rcd/vivier` : le vivier et les compteurs (direction, adjoint).
    pub async fn absp_vivier_rcd(&self) -> Result<VivierRcdDto, ApiError> {
        self.appel(Methode::Get, "/personnels/rcd/vivier", None)
            .await
    }

    // ───────────────────────── A11e : remontée hebdomadaire sans nom ─────────────────────────

    /// `GET /personnels/rcd/remontees` : les dernières semaines (la plus récente d'abord).
    pub async fn absp_remontees_rcd(&self) -> Result<Vec<RemonteeRcdDto>, ApiError> {
        self.appel(Methode::Get, "/personnels/rcd/remontees", None)
            .await
    }

    /// `POST /personnels/rcd/remontees` : préparer une semaine (lundi ; absente : la semaine passée).
    pub async fn absp_preparer_remontee_rcd(
        &self,
        semaine: Option<NaiveDate>,
    ) -> Result<RemonteeRcdDto, ApiError> {
        self.appel(
            Methode::Post,
            "/personnels/rcd/remontees",
            corps(&PreparerRemonteeBody { semaine })?,
        )
        .await
    }

    /// `POST /personnels/rcd/remontees/{id}/{controle|rejet|transmission}`.
    pub async fn absp_suite_remontee_rcd(
        &self,
        id: Uuid,
        suite: SuiteRemontee,
    ) -> Result<RemonteeRcdDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/personnels/rcd/remontees/{id}/{}", suite.segment()),
            None,
        )
        .await
    }

    /// `GET /personnels/rcd/remontees/{id}/export.csv` (une remontée non contrôlée ne s'exporte pas).
    pub async fn absp_remontee_csv(&self, id: Uuid) -> Result<Vec<u8>, ApiError> {
        self.absp_csv(&format!("/personnels/rcd/remontees/{id}/export.csv"))
            .await
    }

    // ───────────────────────── A11f : paramétrage ─────────────────────────

    /// `GET /personnels/absences/parametres` : seuil de la mention « répond rarement » (direction, organisateur).
    pub async fn absp_parametres(&self) -> Result<ParametresAbsencesDto, ApiError> {
        self.appel(Methode::Get, "/personnels/absences/parametres", None)
            .await
    }

    /// `PUT /personnels/absences/parametres` : règle le seuil, de 1 à 20 (droit `parametrer`).
    pub async fn absp_definir_parametres(
        &self,
        seuil_sans_reponse_min: i16,
    ) -> Result<ParametresAbsencesDto, ApiError> {
        self.appel(
            Methode::Put,
            "/personnels/absences/parametres",
            corps(&ModifierParametresAbsencesBody {
                seuil_sans_reponse_min,
            })?,
        )
        .await
    }

    /// `GET /personnels/absences/edt-reference` : l'emploi du temps est-il celui de référence ?
    pub async fn absp_edt_reference(&self) -> Result<EdtReferenceDto, ApiError> {
        self.appel(Methode::Get, "/personnels/absences/edt-reference", None)
            .await
    }

    /// `PUT /personnels/absences/edt-reference` : pose ou retire l'interrupteur (droit `parametrer`).
    pub async fn absp_definir_edt_reference(
        &self,
        actif: bool,
    ) -> Result<EdtReferenceDto, ApiError> {
        self.appel(
            Methode::Put,
            "/personnels/absences/edt-reference",
            corps(&EdtReferenceBody { actif })?,
        )
        .await
    }

    /// `GET /personnels/absences/circuit-regles` : règles de valideurs, instructeurs et informés.
    pub async fn absp_regles_circuit(&self) -> Result<Vec<RegleCircuitDto>, ApiError> {
        self.appel(Methode::Get, "/personnels/absences/circuit-regles", None)
            .await
    }

    /// `POST /personnels/absences/circuit-regles` : ajoute une règle.
    pub async fn absp_creer_regle_circuit(
        &self,
        body: &CreerRegleCircuitBody,
    ) -> Result<RegleCircuitDto, ApiError> {
        self.appel(
            Methode::Post,
            "/personnels/absences/circuit-regles",
            corps(body)?,
        )
        .await
    }

    /// `DELETE /personnels/absences/circuit-regles/{id}` : retire une règle (désactivée, jamais supprimée).
    pub async fn absp_retirer_regle_circuit(&self, id: Uuid) -> Result<(), ApiError> {
        self.appel(
            Methode::Delete,
            &format!("/personnels/absences/circuit-regles/{id}"),
            None,
        )
        .await
    }
}

/// Étape suivante d'une remontée RCD : le segment d'adresse correspondant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuiteRemontee {
    Controle,
    Rejet,
    Transmission,
}

impl SuiteRemontee {
    pub fn segment(self) -> &'static str {
        match self {
            Self::Controle => "controle",
            Self::Rejet => "rejet",
            Self::Transmission => "transmission",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_requete_des_filtres_omet_les_parametres_absents() {
        assert_eq!(requete_filtre_heures(&FiltreHeures::default()), "");
        let f = FiltreHeures {
            mois: NaiveDate::from_ymd_opt(2026, 10, 1),
            etat: Some("contestee".into()),
            ..Default::default()
        };
        assert_eq!(requete_filtre_heures(&f), "?mois=2026-10-01&etat=contestee");
    }

    #[test]
    fn les_segments_de_remontee_suivent_les_routes_du_serveur() {
        assert_eq!(SuiteRemontee::Controle.segment(), "controle");
        assert_eq!(SuiteRemontee::Rejet.segment(), "rejet");
        assert_eq!(SuiteRemontee::Transmission.segment(), "transmission");
    }
}
