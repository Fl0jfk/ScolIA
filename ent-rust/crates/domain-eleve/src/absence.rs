//! Absence = fait de l'élève dans **ent_absence**.
//!
//! Soumission parent ≠ décision VS : un motif peut arriver sans changer le statut.
//! Les issues (justifiée / refusée / annulée-présent) existent parce que le labo
//! et le métier les ont croisées — pas parce qu'un exemple oral les imposait.

use crate::DomainError;
use chrono::{NaiveDate, Utc};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbsenceStatut {
    ATraiter,
    Justifiee,
    Refusee,
    AnnuleePresent,
}

impl AbsenceStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ATraiter => "a_traiter",
            Self::Justifiee => "justifiee",
            Self::Refusee => "refusee",
            Self::AnnuleePresent => "annulee_present",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "justifiee" => Self::Justifiee,
            "refusee" => Self::Refusee,
            "annulee_present" => Self::AnnuleePresent,
            _ => Self::ATraiter,
        }
    }

    /// Compte encore comme absent pour appel / cantine / effectif jour.
    pub fn compte_comme_absent(self) -> bool {
        !matches!(self, Self::AnnuleePresent)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Absence {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub statut: AbsenceStatut,
    pub motif_justif: Option<String>,
    pub justifie_par_user_id: Option<Uuid>,
    pub decision_note: Option<String>,
    pub decision_par_user_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct DeclareAbsence {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JustifierAbsence {
    pub etablissement_id: Uuid,
    pub absence_id: Uuid,
    pub motif_justif: String,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct DecisionAbsence {
    pub etablissement_id: Uuid,
    pub absence_id: Uuid,
    pub actor_user_id: Uuid,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlannedAbsence {
    pub absence: Absence,
    pub event: Option<MetierEventDraft>,
}

pub fn plan_declare_absence(cmd: DeclareAbsence) -> Result<PlannedAbsence, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let motif = cmd.motif.trim().to_string();
    if motif.is_empty() {
        return Err(DomainError::EmptyAbsenceMotif);
    }
    if cmd.fin < cmd.debut {
        return Err(DomainError::InvalidAbsencePeriod);
    }

    let absence = Absence {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        debut: cmd.debut,
        fin: cmd.fin,
        motif: motif.clone(),
        statut: AbsenceStatut::ATraiter,
        motif_justif: None,
        justifie_par_user_id: None,
        decision_note: None,
        decision_par_user_id: None,
    };
    let event = MetierEventDraft {
        etablissement_id: absence.etablissement_id,
        type_name: types::ELEVE_ABSENCE_DECLARED.to_string(),
        aggregate: AggregateKind::Absence,
        aggregate_id: absence.id,
        eleve_id: Some(absence.eleve_id),
        payload: json!({
            "debut": absence.debut.to_string(),
            "fin": absence.fin.to_string(),
            "motif": motif,
            "statut": absence.statut.as_str(),
        }),
        actor_user_id: cmd.actor_user_id,
    };
    Ok(PlannedAbsence {
        absence,
        event: Some(event),
    })
}

/// Parent soumet un motif — la VS tranche ensuite (accepter / refuser).
pub fn plan_justifier_absence(
    current: &Absence,
    cmd: JustifierAbsence,
) -> Result<PlannedAbsence, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.absence_id {
        return Err(DomainError::NotFound);
    }
    if current.statut != AbsenceStatut::ATraiter {
        return Err(DomainError::InvalidAbsencePeriod);
    }
    let motif_justif = cmd.motif_justif.trim().to_string();
    if motif_justif.is_empty() {
        return Err(DomainError::EmptyAbsenceMotif);
    }

    let mut absence = current.clone();
    absence.motif_justif = Some(motif_justif.clone());
    absence.justifie_par_user_id = Some(cmd.actor_user_id);

    let event = MetierEventDraft {
        etablissement_id: absence.etablissement_id,
        type_name: types::ELEVE_ABSENCE_JUSTIF_SOUMIS.to_string(),
        aggregate: AggregateKind::Absence,
        aggregate_id: absence.id,
        eleve_id: Some(absence.eleve_id),
        payload: json!({
            "motif_justif": motif_justif,
            "statut": absence.statut.as_str(),
        }),
        actor_user_id: Some(cmd.actor_user_id.to_string()),
    };
    Ok(PlannedAbsence {
        absence,
        event: Some(event),
    })
}

pub fn plan_accepter_justificatif(
    current: &Absence,
    cmd: DecisionAbsence,
) -> Result<PlannedAbsence, DomainError> {
    decide(current, cmd, AbsenceStatut::Justifiee, types::ELEVE_ABSENCE_JUSTIFIED)
}

pub fn plan_refuser_justificatif(
    current: &Absence,
    cmd: DecisionAbsence,
) -> Result<PlannedAbsence, DomainError> {
    decide(current, cmd, AbsenceStatut::Refusee, types::ELEVE_ABSENCE_REFUSED)
}

pub fn plan_annuler_vers_present(
    current: &Absence,
    cmd: DecisionAbsence,
) -> Result<PlannedAbsence, DomainError> {
    decide(
        current,
        cmd,
        AbsenceStatut::AnnuleePresent,
        types::ELEVE_ABSENCE_ANNULEE_PRESENT,
    )
}

fn decide(
    current: &Absence,
    cmd: DecisionAbsence,
    nouveau: AbsenceStatut,
    type_name: &str,
) -> Result<PlannedAbsence, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.absence_id {
        return Err(DomainError::NotFound);
    }
    if current.statut == AbsenceStatut::AnnuleePresent {
        return Ok(PlannedAbsence {
            absence: current.clone(),
            event: None,
        });
    }
    let mut absence = current.clone();
    absence.statut = nouveau;
    absence.decision_note = cmd.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    absence.decision_par_user_id = Some(cmd.actor_user_id);

    let event = MetierEventDraft {
        etablissement_id: absence.etablissement_id,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Absence,
        aggregate_id: absence.id,
        eleve_id: Some(absence.eleve_id),
        payload: json!({
            "statut": absence.statut.as_str(),
            "decision_note": absence.decision_note,
        }),
        actor_user_id: Some(cmd.actor_user_id.to_string()),
    };
    Ok(PlannedAbsence {
        absence,
        event: Some(event),
    })
}

pub fn today_utc() -> NaiveDate {
    Utc::now().date_naive()
}

/// Année scolaire FR : 1er septembre → 31 août. Ex. `2026-2027`.
pub fn annee_scolaire_de(jour: NaiveDate) -> String {
    use chrono::Datelike;
    let y = if jour.month() >= 9 {
        jour.year()
    } else {
        jour.year() - 1
    };
    format!("{y}-{}", y + 1)
}

pub fn debut_annee_scolaire(jour: NaiveDate) -> NaiveDate {
    use chrono::Datelike;
    let y = if jour.month() >= 9 {
        jour.year()
    } else {
        jour.year() - 1
    };
    NaiveDate::from_ymd_opt(y, 9, 1).expect("1er sept")
}

/// Seuil par défaut avant signal VS (découvert en labo sur volumes réels).
pub const SEUIL_ABSENCES_DEFAUT: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalAbsenceStatut {
    Ouvert,
    EnCours,
    Clos,
}

impl SignalAbsenceStatut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ouvert => "ouvert",
            Self::EnCours => "en_cours",
            Self::Clos => "clos",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ouvert" => Some(Self::Ouvert),
            "en_cours" => Some(Self::EnCours),
            "clos" => Some(Self::Clos),
            _ => None,
        }
    }

    pub fn est_actif(self) -> bool {
        matches!(self, Self::Ouvert | Self::EnCours)
    }
}

/// Signal VS absences répétées (file établissement).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignalAbsence {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub annee_scolaire: String,
    pub seuil: i32,
    pub compteur: i32,
    pub statut: SignalAbsenceStatut,
    pub note_vs: Option<String>,
    pub traite_par_user_id: Option<Uuid>,
    /// Fil messagerie ouvert une fois pour prévenir la famille (+ SMTP outbox).
    pub fil_alerte_id: Option<Uuid>,
    /// Dossier de commission VS lié (convocation absences répétées).
    pub commission_dossier_id: Option<Uuid>,
}

/// Sujet du fil d'alerte famille au seuil.
pub fn sujet_alerte_seuil(compteur: i32, seuil: i32) -> String {
    format!("Absences répétées — seuil atteint ({compteur}/{seuil})")
}

/// Corps du message d'alerte famille au seuil.
pub fn corps_alerte_seuil(
    prenom: &str,
    nom: &str,
    compteur: i32,
    seuil: i32,
    annee_scolaire: &str,
) -> String {
    format!(
        "Bonjour,\n\n\
         Le suivi de vie scolaire signale que {prenom} {nom} a atteint le seuil \
         d'absences de l'établissement ({compteur} absences, seuil {seuil}) \
         pour l'année scolaire {annee_scolaire}.\n\n\
         Merci de prendre contact avec le CPE. Ce message est généré automatiquement \
         dans la messagerie de l'établissement."
    )
}

#[derive(Debug, Clone)]
pub struct PrendreEnChargeSignal {
    pub etablissement_id: Uuid,
    pub signal_id: Uuid,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CloreSignalAbsence {
    pub etablissement_id: Uuid,
    pub signal_id: Uuid,
    pub note: String,
    pub actor_user_id: Uuid,
}

/// CPE / VS prend le signal en charge (ouvert → en_cours).
pub fn plan_prendre_en_charge_signal(
    current: &SignalAbsence,
    cmd: &PrendreEnChargeSignal,
) -> Result<SignalAbsence, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.signal_id {
        return Err(DomainError::NotFound);
    }
    match current.statut {
        SignalAbsenceStatut::Clos => Err(DomainError::SignalAbsenceDejaClos),
        SignalAbsenceStatut::EnCours => Ok(current.clone()),
        SignalAbsenceStatut::Ouvert => {
            let mut s = current.clone();
            s.statut = SignalAbsenceStatut::EnCours;
            s.traite_par_user_id = Some(cmd.actor_user_id);
            Ok(s)
        }
    }
}

/// Clôture le signal (ouvert|en_cours → clos) avec note VS obligatoire.
pub fn plan_clore_signal_absence(
    current: &SignalAbsence,
    cmd: &CloreSignalAbsence,
) -> Result<SignalAbsence, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.id != cmd.signal_id {
        return Err(DomainError::NotFound);
    }
    if current.statut == SignalAbsenceStatut::Clos {
        return Err(DomainError::SignalAbsenceDejaClos);
    }
    let note = cmd.note.trim().to_string();
    if note.is_empty() {
        return Err(DomainError::SignalAbsenceInvalide);
    }
    let mut s = current.clone();
    s.statut = SignalAbsenceStatut::Clos;
    s.note_vs = Some(note);
    s.traite_par_user_id = Some(cmd.actor_user_id);
    Ok(s)
}

/// Attache un dossier de commission au signal (idempotent si déjà le même).
pub fn plan_lier_signal_commission(
    current: &SignalAbsence,
    dossier_id: Uuid,
) -> Result<SignalAbsence, DomainError> {
    if !current.statut.est_actif() {
        return Err(DomainError::SignalAbsenceDejaClos);
    }
    if dossier_id.is_nil() {
        return Err(DomainError::SignalAbsenceInvalide);
    }
    if let Some(existing) = current.commission_dossier_id {
        if existing == dossier_id {
            return Ok(current.clone());
        }
        return Err(DomainError::SignalAbsenceInvalide);
    }
    let mut s = current.clone();
    s.commission_dossier_id = Some(dossier_id);
    Ok(s)
}

pub trait AbsenceRepository: Send + Sync {
    fn upsert(&self, absence: Absence) -> Result<(), DomainError>;
    fn get(
        &self,
        etablissement_id: Uuid,
        absence_id: Uuid,
    ) -> Result<Option<Absence>, DomainError>;
    fn est_absent(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, DomainError>;
    fn list_for_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Absence>, DomainError>;
}

#[derive(Default)]
pub struct InMemoryAbsenceRepo {
    store: std::sync::Mutex<Vec<Absence>>,
}

impl InMemoryAbsenceRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AbsenceRepository for InMemoryAbsenceRepo {
    fn upsert(&self, absence: Absence) -> Result<(), DomainError> {
        let mut g = self.store.lock().expect("absence lock");
        g.retain(|a| a.id != absence.id);
        g.push(absence);
        Ok(())
    }

    fn get(
        &self,
        etablissement_id: Uuid,
        absence_id: Uuid,
    ) -> Result<Option<Absence>, DomainError> {
        Ok(self
            .store
            .lock()
            .expect("absence lock")
            .iter()
            .find(|a| a.etablissement_id == etablissement_id && a.id == absence_id)
            .cloned())
    }

    fn est_absent(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, DomainError> {
        Ok(self.store.lock().expect("absence lock").iter().any(|a| {
            a.etablissement_id == etablissement_id
                && a.eleve_id == eleve_id
                && a.debut <= jour
                && jour <= a.fin
                && a.statut.compte_comme_absent()
        }))
    }

    fn list_for_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Absence>, DomainError> {
        let mut rows: Vec<_> = self
            .store
            .lock()
            .expect("absence lock")
            .iter()
            .filter(|a| a.etablissement_id == etablissement_id && a.eleve_id == eleve_id)
            .cloned()
            .collect();
        rows.sort_by_key(|a| a.debut);
        Ok(rows)
    }
}

pub fn module_absences_voit_absent<A: AbsenceRepository>(
    repo: &A,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
) -> Result<bool, DomainError> {
    repo.est_absent(etablissement_id, eleve_id, jour)
}

pub fn module_cantine_voit_absent<A: AbsenceRepository>(
    repo: &A,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
) -> Result<bool, DomainError> {
    repo.est_absent(etablissement_id, eleve_id, jour)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soumission_parent_ne_tranche_pas() {
        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let parent = Uuid::new_v4();
        let declared = plan_declare_absence(DeclareAbsence {
            etablissement_id: etab,
            eleve_id: eleve,
            debut: today_utc(),
            fin: today_utc(),
            motif: "appel".into(),
            actor_user_id: None,
        })
        .unwrap();
        let soumise = plan_justifier_absence(
            &declared.absence,
            JustifierAbsence {
                etablissement_id: etab,
                absence_id: declared.absence.id,
                motif_justif: "maladie".into(),
                actor_user_id: parent,
            },
        )
        .unwrap();
        assert_eq!(soumise.absence.statut, AbsenceStatut::ATraiter);
        assert_eq!(soumise.absence.motif_justif.as_deref(), Some("maladie"));
    }

    #[test]
    fn annulee_ne_compte_plus_comme_absent() {
        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let actor = Uuid::new_v4();
        let declared = plan_declare_absence(DeclareAbsence {
            etablissement_id: etab,
            eleve_id: eleve,
            debut: today_utc(),
            fin: today_utc(),
            motif: "appel".into(),
            actor_user_id: None,
        })
        .unwrap();
        let annulee = plan_annuler_vers_present(
            &declared.absence,
            DecisionAbsence {
                etablissement_id: etab,
                absence_id: declared.absence.id,
                actor_user_id: actor,
                note: Some("erreur".into()),
            },
        )
        .unwrap();
        assert!(!annulee.absence.statut.compte_comme_absent());
        let repo = InMemoryAbsenceRepo::new();
        repo.upsert(annulee.absence).unwrap();
        assert!(!repo.est_absent(etab, eleve, today_utc()).unwrap());
    }

    #[test]
    fn signal_prise_en_charge_puis_cloture() {
        let sig = SignalAbsence {
            id: Uuid::new_v4(),
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            annee_scolaire: "2026-2027".into(),
            seuil: 10,
            compteur: 12,
            statut: SignalAbsenceStatut::Ouvert,
            note_vs: None,
            traite_par_user_id: None,
            fil_alerte_id: None,
            commission_dossier_id: None,
        };
        assert!(sujet_alerte_seuil(12, 10).contains("12/10"));
        assert!(corps_alerte_seuil("Paul", "Longbottom", 12, 10, "2026-2027")
            .contains("Paul Longbottom"));
        let actor = Uuid::new_v4();
        let en_cours = plan_prendre_en_charge_signal(
            &sig,
            &PrendreEnChargeSignal {
                etablissement_id: sig.etablissement_id,
                signal_id: sig.id,
                actor_user_id: actor,
            },
        )
        .unwrap();
        assert_eq!(en_cours.statut, SignalAbsenceStatut::EnCours);
        let clos = plan_clore_signal_absence(
            &en_cours,
            &CloreSignalAbsence {
                etablissement_id: sig.etablissement_id,
                signal_id: sig.id,
                note: "Entretien famille réalisé".into(),
                actor_user_id: actor,
            },
        )
        .unwrap();
        assert_eq!(clos.statut, SignalAbsenceStatut::Clos);
        assert_eq!(clos.note_vs.as_deref(), Some("Entretien famille réalisé"));
        assert!(plan_clore_signal_absence(
            &clos,
            &CloreSignalAbsence {
                etablissement_id: sig.etablissement_id,
                signal_id: sig.id,
                note: "encore".into(),
                actor_user_id: actor,
            },
        )
        .is_err());
    }
}
