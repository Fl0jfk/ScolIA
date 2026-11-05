//! Vie scolaire : absences, justificatifs, signaux d'absence répétée, retards, famille.
//! Copie fidèle (champs et attributs serde) de `ent-api/src/dto.rs`, sans logique métier.
//!
//! Les statuts restent des chaînes (`"a_traiter"`, `"justifiee"`, `"refusee"`,
//! `"annulee_present"` ; signaux : `"ouvert"`, `"en_cours"`, `"clos"`) : leur
//! interprétation d'affichage est dans `ent-web-app` (`composants_metier::absences`).
//!
//! Types de **lecture partielle** (les autres champs du JSON sont ignorés) :
//! [`EleveFicheDto`] (`GET /eleves/{id}`) et [`FamilleResumeDto`]
//! (`GET /famille/enfants/{id}/resume`).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Absence d'un élève (période en jours), avec la justification éventuelle.
/// `eleve_label` n'est fourni que par les routes enrichies (`/vs/absences`, résumé famille).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceDto {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub statut: String,
    pub motif_justif: Option<String>,
    pub decision_note: Option<String>,
}

/// Corps de `POST /absences/{id}/justifier`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JustifierAbsenceBody {
    pub motif_justif: String,
}

/// Corps de `POST /absences/{id}/accepter-justificatif|refuser-justificatif|annuler-present`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionAbsenceBody {
    pub note: Option<String>,
}

/// Signal d'absence répétée (seuil atteint).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalAbsenceDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub annee_scolaire: String,
    pub seuil: i32,
    pub compteur: i32,
    pub statut: String,
    pub note_vs: Option<String>,
    pub traite_par_user_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traite_par_label: Option<String>,
    pub fil_alerte_id: Option<Uuid>,
    pub commission_dossier_id: Option<Uuid>,
}

/// Corps de `POST /vs/signaux/{id}/clore`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloreSignalBody {
    pub note: String,
}

/// Retard d'un élève (`GET /eleves/{id}/retards`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetardDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_appel_id: Option<Uuid>,
}

/// Motif d'absence proposé (`GET /motifs-absence`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceMotifDto {
    pub code: String,
    pub libelle: String,
    pub etablissement_id: Option<Uuid>,
}

/// Enfant d'une famille et droits du lien (`GET /famille/enfants`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnfantFamilleDto {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub status: String,
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
}

/// Résumé famille d'un enfant : lecture partielle (absences à justifier seulement).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilleResumeDto {
    pub eleve_id: Uuid,
    pub absences_a_justifier: Vec<AbsenceDto>,
}

/// Fiche élève : lecture partielle de `GET /eleves/{id}` (identité et classe).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveFicheDto {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
}
