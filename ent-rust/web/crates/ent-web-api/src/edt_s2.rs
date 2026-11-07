//! Emploi du temps S2-5 : semaine, changements vus par les familles, administration des
//! créneaux, absences et remplacements, annulation en masse, délégations, absences de personnel.
//! Routes de `ent-api/src/edt_s2.rs` ; les types JSON sont ceux d'`ent-types::edt_s2`.
//! Les droits (`edt.gerer`, `edt.annuler_masse`, `delegations.gerer`) sont décidés par le serveur.

use chrono::NaiveDate;
use ent_types::edt_s2::{
    AbsenceSeancesDto, CandidatDto, ChangementEdtDto, CmdAnnulationMasseDto, CmdDeplacementDto,
    CmdRemplacementDto, ConflitEdtDto, ConflitsReponseDto, CreneauAdminDto, CreneauEnregistreDto,
    CreneauSaisieDto, DelegationDto, DelegationSaisieDto, RapportMasseDto, ResultatExceptionDto,
    SemaineEdtDto,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::communication::encoder_requete;
use crate::{ApiClient, ApiError, Methode};

/// Ce dont on affiche la semaine (`GET /edt/semaine?cible=…`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CibleSemaine {
    /// Soi-même (cible omise).
    Moi,
    /// Un autre membre du personnel (lecture réservée au personnel).
    Personnel(Uuid),
    Classe(Uuid),
    /// Code de la salle.
    Salle(String),
    /// Élève : lui-même ou un enfant lié (famille).
    Eleve(Uuid),
}

/// Chemin de `GET /edt/semaine` pour une cible et un jour (le serveur ramène au lundi).
pub fn chemin_semaine(cible: &CibleSemaine, jour: NaiveDate) -> String {
    match cible {
        CibleSemaine::Moi => format!("/edt/semaine?jour={jour}"),
        CibleSemaine::Personnel(id) => format!("/edt/semaine?cible=personnel&id={id}&jour={jour}"),
        CibleSemaine::Classe(id) => format!("/edt/semaine?cible=classe&id={id}&jour={jour}"),
        CibleSemaine::Salle(code) => format!(
            "/edt/semaine?cible=salle&salle={}&jour={jour}",
            encoder_requete(code.trim())
        ),
        CibleSemaine::Eleve(id) => format!("/edt/semaine?cible=eleve&id={id}&jour={jour}"),
    }
}

/// Salle de `GET /salles` (types propres à cette route : pas encore dans `ent-types`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SalleApi {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub capacite: Option<i32>,
    #[serde(default)]
    pub site: Option<String>,
}

/// Absence de personnel (`/rh/absences`) : le motif est une donnée sensible, affichée seulement
/// à qui gère les absences.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AbsenceRhApi {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(default)]
    pub user_label: Option<String>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    #[serde(default)]
    pub motif: String,
    pub statut: String,
}

/// Corps de `POST /rh/absences`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DeclarationAbsenceRh {
    /// Absent (soi-même si omis).
    pub user_id: Option<Uuid>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
}

/// Conflits d'un créneau refusé (409 de `POST/PUT /edt/admin/creneaux`) ; `None` pour toute
/// autre erreur (le corps d'un 409 est alors un texte simple).
pub fn conflits_de(erreur: &ApiError) -> Option<ConflitsReponseDto> {
    match erreur {
        ApiError::Conflit(corps) => serde_json::from_str(corps).ok(),
        _ => None,
    }
}

fn corps<T: Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    // ───── Semaine et changements ─────

    /// `GET /edt/semaine` : séances de la semaine, statuts, jours fermés, semaine A/B.
    pub async fn edt_semaine(
        &self,
        cible: &CibleSemaine,
        jour: NaiveDate,
    ) -> Result<SemaineEdtDto, ApiError> {
        self.appel(Methode::Get, &chemin_semaine(cible, jour), None)
            .await
    }

    /// `GET /eleves/{id}/edt/changements?du=&au=` : changements vus par l'élève et sa famille
    /// (jamais de motif libre).
    pub async fn changements_edt(
        &self,
        eleve: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<ChangementEdtDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/eleves/{eleve}/edt/changements?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /salles` : salles de l'établissement.
    pub async fn salles(&self) -> Result<Vec<SalleApi>, ApiError> {
        self.appel(Methode::Get, "/salles", None).await
    }

    // ───── Administration des créneaux ─────

    /// `GET /edt/admin/creneaux` (filtres facultatifs : classe, professeur, salle).
    pub async fn creneaux_admin(
        &self,
        classe: Option<&str>,
        prof: Option<Uuid>,
        salle: Option<&str>,
    ) -> Result<Vec<CreneauAdminDto>, ApiError> {
        let mut params = Vec::new();
        if let Some(c) = classe.filter(|c| !c.trim().is_empty()) {
            params.push(format!("classe={}", encoder_requete(c.trim())));
        }
        if let Some(p) = prof {
            params.push(format!("prof={p}"));
        }
        if let Some(s) = salle.filter(|s| !s.trim().is_empty()) {
            params.push(format!("salle={}", encoder_requete(s.trim())));
        }
        let chemin = if params.is_empty() {
            "/edt/admin/creneaux".to_string()
        } else {
            format!("/edt/admin/creneaux?{}", params.join("&"))
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `POST /edt/admin/creneaux?apercu=` : 409 avec la liste des conflits (voir [`conflits_de`]).
    pub async fn creer_creneau(
        &self,
        saisie: &CreneauSaisieDto,
        apercu: bool,
    ) -> Result<CreneauEnregistreDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/edt/admin/creneaux?apercu={apercu}"),
            Some(corps(saisie)?),
        )
        .await
    }

    /// `PUT /edt/admin/creneaux/{id}?apercu=`.
    pub async fn modifier_creneau(
        &self,
        id: Uuid,
        saisie: &CreneauSaisieDto,
        apercu: bool,
    ) -> Result<CreneauEnregistreDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/edt/admin/creneaux/{id}?apercu={apercu}"),
            Some(corps(saisie)?),
        )
        .await
    }

    /// `DELETE /edt/admin/creneaux/{id}` (409 s'il existe des séances).
    pub async fn supprimer_creneau(&self, id: Uuid) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(Methode::Delete, &format!("/edt/admin/creneaux/{id}"), None)
            .await?;
        Ok(())
    }

    /// `GET /edt/admin/conflits` : conflits de l'emploi du temps en place.
    pub async fn conflits_edt(&self) -> Result<Vec<ConflitEdtDto>, ApiError> {
        self.appel(Methode::Get, "/edt/admin/conflits", None).await
    }

    /// `PUT /edt/admin/salles/{id}`.
    pub async fn modifier_salle(
        &self,
        id: Uuid,
        libelle: &str,
        capacite: Option<i32>,
        actif: bool,
    ) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Put,
                &format!("/edt/admin/salles/{id}"),
                Some(json!({ "libelle": libelle, "capacite": capacite, "actif": actif })),
            )
            .await?;
        Ok(())
    }

    // ───── Absences et remplacements ─────

    /// `GET /edt/absences?du=&au=` (62 jours au plus) : absences de personnel et séances touchées.
    pub async fn absences_edt(
        &self,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<AbsenceSeancesDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/edt/absences?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /edt/absences/{id}/seances`.
    pub async fn absence_seances(&self, id: Uuid) -> Result<AbsenceSeancesDto, ApiError> {
        self.appel(Methode::Get, &format!("/edt/absences/{id}/seances"), None)
            .await
    }

    /// `GET /edt/remplacants` : disponibles et sans conflit, compétents d'abord ; `tous` ajoute
    /// les enseignants non compétents.
    pub async fn remplacants(
        &self,
        creneau: Uuid,
        jour: NaiveDate,
        tous: bool,
    ) -> Result<Vec<CandidatDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/edt/remplacants?creneau_id={creneau}&jour={jour}&tous={tous}"),
            None,
        )
        .await
    }

    /// `POST /edt/remplacements` : 201, ou 200 si le remplacement existait déjà (`deja_fait`).
    pub async fn organiser_remplacement(
        &self,
        cmd: &CmdRemplacementDto,
    ) -> Result<ResultatExceptionDto, ApiError> {
        self.appel(Methode::Post, "/edt/remplacements", Some(corps(cmd)?))
            .await
    }

    /// `POST /edt/annulations`.
    pub async fn annuler_seance(
        &self,
        creneau: Uuid,
        jour: NaiveDate,
        absence_rh: Option<Uuid>,
    ) -> Result<ResultatExceptionDto, ApiError> {
        self.appel(
            Methode::Post,
            "/edt/annulations",
            Some(json!({ "creneau_id": creneau, "jour": jour, "absence_rh_id": absence_rh })),
        )
        .await
    }

    /// `POST /edt/changements-salle`.
    pub async fn changer_salle(
        &self,
        creneau: Uuid,
        jour: NaiveDate,
        salle: &str,
    ) -> Result<ResultatExceptionDto, ApiError> {
        self.appel(
            Methode::Post,
            "/edt/changements-salle",
            Some(json!({ "creneau_id": creneau, "jour": jour, "salle": salle })),
        )
        .await
    }

    /// `POST /edt/deplacements`.
    pub async fn deplacer_seance(
        &self,
        cmd: &CmdDeplacementDto,
    ) -> Result<ResultatExceptionDto, ApiError> {
        self.appel(Methode::Post, "/edt/deplacements", Some(corps(cmd)?))
            .await
    }

    /// `DELETE /edt/exceptions-levee` : rétablit la séance (404 si rien à rétablir).
    pub async fn lever_exception(&self, creneau: Uuid, jour: NaiveDate) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Delete,
                &format!("/edt/exceptions-levee?creneau_id={creneau}&jour={jour}"),
                None,
            )
            .await?;
        Ok(())
    }

    /// `POST /edt/annulation-masse` : aperçu (`cmd.apercu`) ou application avec sa clé d'idempotence.
    pub async fn annulation_masse(
        &self,
        cmd: &CmdAnnulationMasseDto,
    ) -> Result<RapportMasseDto, ApiError> {
        self.appel(Methode::Post, "/edt/annulation-masse", Some(corps(cmd)?))
            .await
    }

    // ───── Absences de personnel (RH-02, RH-03) ─────

    /// `GET /rh/absences?debut=&fin=`.
    pub async fn absences_rh(
        &self,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<AbsenceRhApi>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/rh/absences?debut={debut}&fin={fin}"),
            None,
        )
        .await
    }

    /// `POST /rh/absences` : déclare une absence (soi-même, ou un autre si l'on gère le personnel).
    pub async fn declarer_absence_rh(
        &self,
        declaration: &DeclarationAbsenceRh,
    ) -> Result<AbsenceRhApi, ApiError> {
        self.appel(Methode::Post, "/rh/absences", Some(corps(declaration)?))
            .await
    }

    /// `POST /rh/absences/{id}/valider` (idempotent).
    pub async fn valider_absence_rh(&self, id: Uuid) -> Result<AbsenceRhApi, ApiError> {
        self.appel(Methode::Post, &format!("/rh/absences/{id}/valider"), None)
            .await
    }

    // ───── Délégations ─────

    /// `GET /delegations?en_cours=` (droit `delegations.gerer`).
    pub async fn delegations(&self, en_cours: bool) -> Result<Vec<DelegationDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/delegations?en_cours={en_cours}"),
            None,
        )
        .await
    }

    /// `GET /delegations/mes` : délégations reçues par la personne connectée.
    pub async fn mes_delegations(&self) -> Result<Vec<DelegationDto>, ApiError> {
        self.appel(Methode::Get, "/delegations/mes", None).await
    }

    /// `POST /delegations` (201).
    pub async fn accorder_delegation(
        &self,
        saisie: &DelegationSaisieDto,
    ) -> Result<DelegationDto, ApiError> {
        self.appel(Methode::Post, "/delegations", Some(corps(saisie)?))
            .await
    }

    /// `POST /delegations/{id}/revoquer`.
    pub async fn revoquer_delegation(&self, id: Uuid) -> Result<DelegationDto, ApiError> {
        self.appel(Methode::Post, &format!("/delegations/{id}/revoquer"), None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jour() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 9).unwrap()
    }

    #[test]
    fn chemins_de_la_semaine() {
        let id = Uuid::nil();
        assert_eq!(
            chemin_semaine(&CibleSemaine::Moi, jour()),
            "/edt/semaine?jour=2026-11-09"
        );
        assert_eq!(
            chemin_semaine(&CibleSemaine::Classe(id), jour()),
            format!("/edt/semaine?cible=classe&id={id}&jour=2026-11-09")
        );
        assert_eq!(
            chemin_semaine(&CibleSemaine::Eleve(id), jour()),
            format!("/edt/semaine?cible=eleve&id={id}&jour=2026-11-09")
        );
        assert_eq!(
            chemin_semaine(&CibleSemaine::Personnel(id), jour()),
            format!("/edt/semaine?cible=personnel&id={id}&jour=2026-11-09")
        );
        // Le code de salle est encodé (espaces, accents, séparateurs).
        let c = chemin_semaine(&CibleSemaine::Salle("B 12&x=1".into()), jour());
        assert!(c.starts_with("/edt/semaine?cible=salle&salle="), "{c}");
        assert!(!c.contains(' ') && !c.contains("&x=1"), "{c}");
    }

    #[test]
    fn conflits_lus_dans_un_409() {
        let corps = r#"{"message":"conflit d'emploi du temps","conflits":[{"nature":"salle",
            "creneau_id":null,"autre_creneau_id":null,"jour_semaine":2,
            "heure_debut":"08:00:00","heure_fin":"09:00:00","detail":"B12"}]}"#;
        let c = conflits_de(&ApiError::Conflit(corps.into())).expect("409 décodé");
        assert_eq!(c.conflits.len(), 1);
        assert!(conflits_de(&ApiError::Conflit("des séances existent".into())).is_none());
        assert!(conflits_de(&ApiError::Introuvable("x".into())).is_none());
    }

    #[test]
    fn absence_rh_decodee_sans_motif_obligatoire() {
        let a: AbsenceRhApi = serde_json::from_str(
            r#"{"id":"6f1c3c80-0000-4000-8000-000000000001",
            "user_id":"6f1c3c80-0000-4000-8000-000000000002","debut":"2026-11-09",
            "fin":"2026-11-10","statut":"signalee"}"#,
        )
        .unwrap();
        assert_eq!(a.statut, "signalee");
        assert!(a.motif.is_empty());
    }
}
