//! Emploi du temps S2-5 : contrat JSON des routes `/edt/semaine`, `/edt/admin/*`, `/edt/absences`,
//! `/edt/remplacants`, `/edt/remplacements`, `/edt/annulations`, `/edt/deplacements`,
//! `/edt/annulation-masse`, `/eleves/{id}/edt/changements` et `/delegations*`.
//! Le JSON est celui des structures d'`ent-db` (champs inconnus ignorés) ; un test contre le vrai
//! backend vérifie le décodage. Aucune logique métier.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───── Semaine ─────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceSemaineDto {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub classe: String,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    #[serde(default)]
    pub prof_nom: Option<String>,
    #[serde(default)]
    pub remplacant_user_id: Option<Uuid>,
    #[serde(default)]
    pub remplacant_nom: Option<String>,
    #[serde(default)]
    pub salle: Option<String>,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    /// `prevue`, `annulee`, `remplacee`, `deplacee` (cours d'origine), `ajoutee` (cours déplacé).
    pub statut: String,
    #[serde(default)]
    pub deplace_vers: Option<NaiveDate>,
    #[serde(default)]
    pub deplace_depuis: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JourFermeDto {
    pub jour: NaiveDate,
    pub libelle: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemaineEdtDto {
    pub lundi: NaiveDate,
    /// `A`, `B` ou absent (aucun calendrier d'alternance).
    #[serde(default)]
    pub semaine_ab: Option<String>,
    #[serde(default)]
    pub jours_fermes: Vec<JourFermeDto>,
    #[serde(default)]
    pub seances: Vec<SeanceSemaineDto>,
}

// ───── Créneaux et conflits ─────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreneauAdminDto {
    pub id: Uuid,
    pub classe: String,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe: Option<String>,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    #[serde(default)]
    pub prof_nom: Option<String>,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default)]
    pub salle: Option<String>,
    #[serde(default)]
    pub site: String,
    pub semaine_ab: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreneauSaisieDto {
    pub classe: String,
    pub groupe_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub salle: Option<String>,
    pub site: String,
    /// `toutes`, `a`, `b`.
    pub semaine_ab: String,
}

/// `professeur`, `salle`, `classe`, `indisponibilite_professeur`, `indisponibilite_salle`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflitEdtDto {
    pub nature: String,
    #[serde(default)]
    pub creneau_id: Option<Uuid>,
    #[serde(default)]
    pub autre_creneau_id: Option<Uuid>,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default)]
    pub detail: String,
}

/// Corps de la réponse 409 d'un créneau en conflit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflitsReponseDto {
    pub message: String,
    pub conflits: Vec<ConflitEdtDto>,
}

/// Réponse 200/201 d'un enregistrement de créneau (`creneau` absent en aperçu).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreneauEnregistreDto {
    #[serde(default)]
    pub creneau: Option<CreneauAdminDto>,
}

// ───── Absences et remplacements ─────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceVueDto {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(default)]
    pub nom: Option<String>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    /// `signalee` ou `validee`.
    pub statut: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceToucheeDto {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub classe: String,
    pub classe_id: Uuid,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default)]
    pub salle: Option<String>,
    /// `prevue` (à traiter), `annulee`, `remplacee`, `deplacee`.
    pub statut: String,
    /// `titulaire` ou `remplacant`.
    pub role_absent: String,
    #[serde(default)]
    pub remplacant_user_id: Option<Uuid>,
    #[serde(default)]
    pub remplacant_nom: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceSeancesDto {
    pub absence: AbsenceVueDto,
    #[serde(default)]
    pub seances: Vec<SeanceToucheeDto>,
    pub a_traiter: usize,
    pub du: NaiveDate,
    pub au: NaiveDate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatDto {
    pub user_id: Uuid,
    #[serde(default)]
    pub nom: Option<String>,
    pub competent: bool,
    pub seances_ce_jour: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmdRemplacementDto {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub remplacant_user_id: Uuid,
    pub absence_rh_id: Option<Uuid>,
    pub nouvelle_salle: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatExceptionDto {
    pub exception_id: Uuid,
    pub deja_fait: bool,
    #[serde(default)]
    pub delegation_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmdDeplacementDto {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub nouveau_jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub salle: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmdAnnulationMasseDto {
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// `etablissement`, `niveau` ou `classes`.
    pub portee: String,
    pub cibles: Vec<Uuid>,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
    pub motif_public: String,
    pub cle: Option<String>,
    pub apercu: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClasseToucheeDto {
    pub classe_id: Uuid,
    pub classe: String,
    pub nb_seances: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportMasseDto {
    pub apercu: bool,
    pub deja_applique: bool,
    #[serde(default)]
    pub lot_id: Option<Uuid>,
    pub nb_seances: usize,
    pub nb_classes: usize,
    #[serde(default)]
    pub classes: Vec<ClasseToucheeDto>,
    pub deja_annulees: usize,
}

// ───── Vue élève / famille ─────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangementEdtDto {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    /// `annulation`, `remplacement`, `changement_salle`, `deplacement`.
    pub nature: String,
    pub matiere_libelle: String,
    pub classe: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default)]
    pub nouvelle_salle: Option<String>,
    #[serde(default)]
    pub remplacant_nom: Option<String>,
    #[serde(default)]
    pub nouveau_jour: Option<NaiveDate>,
    #[serde(default)]
    pub nouvelle_heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub nouvelle_heure_fin: Option<NaiveTime>,
    #[serde(default)]
    pub motif_public: Option<String>,
    pub enregistre_le: DateTime<Utc>,
}

// ───── Délégations ─────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroitDelegueDto {
    pub action: String,
    pub portee: String,
    #[serde(default)]
    pub cibles: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationDto {
    pub id: Uuid,
    pub delegue_user_id: Uuid,
    #[serde(default)]
    pub delegue_nom: Option<String>,
    pub delegant_user_id: Uuid,
    #[serde(default)]
    pub delegant_nom: Option<String>,
    pub droits: Vec<DroitDelegueDto>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    #[serde(default)]
    pub motif: String,
    /// `manuelle` ou `remplacement`.
    pub source: String,
    /// `a_venir`, `active`, `expiree`, `revoquee`.
    pub etat: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationSaisieDto {
    pub delegue_user_id: Uuid,
    pub droits: Vec<DroitDelegueDto>,
    pub debut: Option<NaiveDate>,
    pub fin: NaiveDate,
    pub motif: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exemples figés : le JSON produit par `ent-db` reste décodable (champs optionnels absents).
    #[test]
    fn semaine_minimale_et_conflits() {
        let s: SemaineEdtDto = serde_json::from_str(
            r#"{"lundi":"2026-11-09","semaine_ab":null,"jours_fermes":[],"seances":[
              {"creneau_id":"00000000-0000-4000-8000-000000000001","jour":"2026-11-09","classe":"4B",
               "matiere_libelle":"Maths","prof_user_id":"00000000-0000-4000-8000-000000000002",
               "heure_debut":"14:00:00","heure_fin":"15:00:00","statut":"prevue"}]}"#,
        )
        .unwrap();
        assert_eq!(s.seances.len(), 1);
        assert!(s.seances[0].remplacant_nom.is_none());
        let c: ConflitsReponseDto = serde_json::from_str(
            r#"{"message":"conflit d'emploi du temps","conflits":[{"nature":"professeur",
               "creneau_id":null,"autre_creneau_id":null,"jour_semaine":1,
               "heure_debut":"08:00:00","heure_fin":"09:00:00","detail":"même professeur"}]}"#,
        )
        .unwrap();
        assert_eq!(c.conflits[0].nature, "professeur");
    }

    #[test]
    fn delegation_aller_retour() {
        let d = DelegationSaisieDto {
            delegue_user_id: Uuid::nil(),
            droits: vec![DroitDelegueDto {
                action: "edt.gerer".into(),
                portee: "etablissement".into(),
                cibles: vec![],
            }],
            debut: None,
            fin: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
            motif: "Sortie".into(),
        };
        let j = serde_json::to_string(&d).unwrap();
        assert_eq!(serde_json::from_str::<DelegationSaisieDto>(&j).unwrap(), d);
    }
}
