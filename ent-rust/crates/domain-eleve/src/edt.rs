//! Emploi du temps — exceptions jour J + alternance semaines A/B.
//!
//! Créneau hebdo ; exception nominative pour une date :
//! annulation, remplacement, changement de salle.
//! Semaine A/B : un lundi de référence (semaine A) + filtre `toutes|a|b` sur le créneau.

use crate::DomainError;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Portée d'un créneau sur le cycle A/B.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdtSemaineAb {
    Toutes,
    A,
    B,
}

impl EdtSemaineAb {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Toutes => "toutes",
            Self::A => "a",
            Self::B => "b",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "toutes" | "all" | "" => Some(Self::Toutes),
            "a" => Some(Self::A),
            "b" => Some(Self::B),
            _ => None,
        }
    }
}

/// Lundi ISO de la semaine contenant `jour`.
pub fn lundi_iso(jour: NaiveDate) -> NaiveDate {
    let offset = jour.weekday().num_days_from_monday() as i64;
    jour - Duration::days(offset)
}

/// Semaine A/B au jour J relative au lundi de référence (semaine A).
pub fn semaine_ab_au_jour(reference_lundi: NaiveDate, jour: NaiveDate) -> EdtSemaineAb {
    let lundi = lundi_iso(jour);
    let weeks = (lundi - reference_lundi).num_days().div_euclid(7);
    if weeks.rem_euclid(2) == 0 {
        EdtSemaineAb::A
    } else {
        EdtSemaineAb::B
    }
}

pub fn creneau_actif_semaine(creneau: EdtSemaineAb, jour_ab: EdtSemaineAb) -> bool {
    matches!(creneau, EdtSemaineAb::Toutes) || creneau == jour_ab
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdtCalendrierAb {
    pub etablissement_id: Uuid,
    pub reference_lundi: NaiveDate,
    pub created_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ConfigurerEdtCalendrierAb {
    pub etablissement_id: Uuid,
    pub reference_lundi: NaiveDate,
    pub actor_user_id: Uuid,
}

pub fn plan_configurer_edt_calendrier_ab(
    cmd: ConfigurerEdtCalendrierAb,
) -> Result<EdtCalendrierAb, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.reference_lundi.weekday().num_days_from_monday() != 0 {
        return Err(DomainError::EdtCalendrierAbInvalide);
    }
    Ok(EdtCalendrierAb {
        etablissement_id: cmd.etablissement_id,
        reference_lundi: cmd.reference_lundi,
        created_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdtExceptionKind {
    Annulation,
    Remplacement,
    ChangementSalle,
}

impl EdtExceptionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Annulation => "annulation",
            Self::Remplacement => "remplacement",
            Self::ChangementSalle => "changement_salle",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "annulation" => Some(Self::Annulation),
            "remplacement" => Some(Self::Remplacement),
            "changement_salle" => Some(Self::ChangementSalle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdtException {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub kind: EdtExceptionKind,
    pub remplacant_user_id: Option<Uuid>,
    pub nouvelle_salle: Option<String>,
    pub motif: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PoserEdtException {
    pub etablissement_id: Uuid,
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub kind: EdtExceptionKind,
    pub remplacant_user_id: Option<Uuid>,
    pub nouvelle_salle: Option<String>,
    pub motif: String,
    pub actor_user_id: Uuid,
}

/// Ligne d'import EDT (CSV dry-run / apply).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtImportLigne {
    pub index: usize,
    pub classe: String,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    pub jour_semaine: i16,
    pub heure_debut: chrono::NaiveTime,
    pub heure_fin: chrono::NaiveTime,
    pub salle: Option<String>,
    pub semaine_ab: EdtSemaineAb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdtConflictKind {
    ProfDouble,
    SalleDouble,
    ClasseDouble,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtConflict {
    pub kind: EdtConflictKind,
    pub ligne_a: usize,
    pub ligne_b: usize,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct EdtImportReport {
    pub lines: usize,
    pub ok_count: usize,
    pub conflicts: Vec<EdtConflict>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EdtImportParseError {
    #[error("CSV EDT vide")]
    Empty,
    #[error("colonnes insuffisantes (classe;matiere;prof_user_id;jour;debut;fin[;salle;semaine])")]
    Columns,
    #[error("jour_semaine invalide (1-7)")]
    Jour,
    #[error("heure invalide (HH:MM)")]
    Heure,
    #[error("heure_fin doit être après heure_debut")]
    Plage,
    #[error("prof_user_id invalide")]
    Prof,
    #[error("semaine_ab invalide (toutes|a|b)")]
    Semaine,
    #[error("classe ou matière vide")]
    EmptyField,
    #[error("jour Pronote invalide (Lundi…Dimanche ou 1-7)")]
    JourPronote,
    #[error("enseignant (e-mail) vide")]
    ProfEmail,
}

fn parse_time_hm(s: &str) -> Option<chrono::NaiveTime> {
    let t = s.trim();
    if let Ok(t) = chrono::NaiveTime::parse_from_str(t, "%H:%M") {
        return Some(t);
    }
    chrono::NaiveTime::parse_from_str(t, "%H:%M:%S").ok()
}

fn times_overlap(
    a0: chrono::NaiveTime,
    a1: chrono::NaiveTime,
    b0: chrono::NaiveTime,
    b1: chrono::NaiveTime,
) -> bool {
    a0 < b1 && b0 < a1
}

fn semaines_chevauchent(a: EdtSemaineAb, b: EdtSemaineAb) -> bool {
    matches!(a, EdtSemaineAb::Toutes)
        || matches!(b, EdtSemaineAb::Toutes)
        || a == b
}

/// Ligne Pronote avant résolution e-mail → user_id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtPronoteLigne {
    pub index: usize,
    pub classe: String,
    pub matiere_libelle: String,
    pub prof_email: String,
    pub jour_semaine: i16,
    pub heure_debut: chrono::NaiveTime,
    pub heure_fin: chrono::NaiveTime,
    pub salle: Option<String>,
    pub semaine_ab: EdtSemaineAb,
}

fn parse_jour_pronote(s: &str) -> Option<i16> {
    let t = s.trim().to_ascii_lowercase();
    match t.as_str() {
        "1" | "lundi" | "lun" | "l" => Some(1),
        "2" | "mardi" | "mar" | "ma" => Some(2),
        "3" | "mercredi" | "mer" | "me" => Some(3),
        "4" | "jeudi" | "jeu" | "j" => Some(4),
        "5" | "vendredi" | "ven" | "v" => Some(5),
        "6" | "samedi" | "sam" | "s" => Some(6),
        "7" | "dimanche" | "dim" | "d" => Some(7),
        _ => t.parse().ok().filter(|n| (1..=7).contains(n)),
    }
}

fn parse_time_pronote(s: &str) -> Option<chrono::NaiveTime> {
    let t = s.trim().to_ascii_lowercase().replace('h', ":");
    parse_time_hm(&t)
}

fn parse_freq_pronote(s: &str) -> Option<EdtSemaineAb> {
    match s.trim().to_ascii_lowercase().as_str() {
        "" | "h" | "hebdo" | "hebdomadaire" | "toutes" | "all" | "1" => Some(EdtSemaineAb::Toutes),
        "a" | "s1" | "q1" | "semaine a" | "sem. a" => Some(EdtSemaineAb::A),
        "b" | "s2" | "q2" | "semaine b" | "sem. b" => Some(EdtSemaineAb::B),
        other => EdtSemaineAb::parse(other),
    }
}

/// Parse export Pronote simplifié (`;`) :
/// `Classe;Matière;Enseignant;Jour;Début;Fin[;Salle;Fréquence]`
/// - Enseignant = e-mail ENT
/// - Jour = Lundi…Dimanche ou 1-7
/// - Début/Fin = `08h00` ou `08:00`
/// - Fréquence = H / A|S1 / B|S2
pub fn parse_edt_pronote_csv(raw: &str) -> Result<Vec<EdtPronoteLigne>, EdtImportParseError> {
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(';').map(str::trim).collect();
        if parts.len() < 6 {
            return Err(EdtImportParseError::Columns);
        }
        if i == 0
            && (parts[0].eq_ignore_ascii_case("classe")
                || parts[0].eq_ignore_ascii_case("class"))
        {
            continue;
        }
        let classe = parts[0].to_string();
        let matiere = parts[1].to_string();
        if classe.is_empty() || matiere.is_empty() {
            return Err(EdtImportParseError::EmptyField);
        }
        let prof_email = parts[2].trim().to_ascii_lowercase();
        if prof_email.is_empty() || !prof_email.contains('@') {
            return Err(EdtImportParseError::ProfEmail);
        }
        let jour_semaine =
            parse_jour_pronote(parts[3]).ok_or(EdtImportParseError::JourPronote)?;
        let heure_debut =
            parse_time_pronote(parts[4]).ok_or(EdtImportParseError::Heure)?;
        let heure_fin = parse_time_pronote(parts[5]).ok_or(EdtImportParseError::Heure)?;
        if heure_fin <= heure_debut {
            return Err(EdtImportParseError::Plage);
        }
        let salle = parts.get(6).and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        });
        let semaine_ab = parts
            .get(7)
            .map(|s| parse_freq_pronote(s).ok_or(EdtImportParseError::Semaine))
            .transpose()?
            .unwrap_or(EdtSemaineAb::Toutes);
        out.push(EdtPronoteLigne {
            index: out.len(),
            classe,
            matiere_libelle: matiere,
            prof_email,
            jour_semaine,
            heure_debut,
            heure_fin,
            salle,
            semaine_ab,
        });
    }
    if out.is_empty() {
        return Err(EdtImportParseError::Empty);
    }
    Ok(out)
}

/// Résout les e-mails Pronote → `EdtImportLigne` (callback = lookup user).
pub fn resolve_pronote_lignes(
    lines: &[EdtPronoteLigne],
    mut resolve_email: impl FnMut(&str) -> Option<Uuid>,
) -> Result<Vec<EdtImportLigne>, EdtImportParseError> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let Some(prof_user_id) = resolve_email(&line.prof_email) else {
            return Err(EdtImportParseError::ProfEmail);
        };
        out.push(EdtImportLigne {
            index: line.index,
            classe: line.classe.clone(),
            matiere_libelle: line.matiere_libelle.clone(),
            prof_user_id,
            jour_semaine: line.jour_semaine,
            heure_debut: line.heure_debut,
            heure_fin: line.heure_fin,
            salle: line.salle.clone(),
            semaine_ab: line.semaine_ab,
        });
    }
    Ok(out)
}

/// Parse CSV EDT natif (`;`, en-tête optionnelle).
pub fn parse_edt_import_csv(raw: &str) -> Result<Vec<EdtImportLigne>, EdtImportParseError> {
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(';').map(str::trim).collect();
        if parts.len() < 6 {
            return Err(EdtImportParseError::Columns);
        }
        if i == 0 && parts[0].eq_ignore_ascii_case("classe") {
            continue;
        }
        let classe = parts[0].to_string();
        let matiere = parts[1].to_string();
        if classe.is_empty() || matiere.is_empty() {
            return Err(EdtImportParseError::EmptyField);
        }
        let prof_user_id = Uuid::parse_str(parts[2]).map_err(|_| EdtImportParseError::Prof)?;
        let jour_semaine: i16 = parts[3]
            .parse()
            .map_err(|_| EdtImportParseError::Jour)?;
        if !(1..=7).contains(&jour_semaine) {
            return Err(EdtImportParseError::Jour);
        }
        let heure_debut = parse_time_hm(parts[4]).ok_or(EdtImportParseError::Heure)?;
        let heure_fin = parse_time_hm(parts[5]).ok_or(EdtImportParseError::Heure)?;
        if heure_fin <= heure_debut {
            return Err(EdtImportParseError::Plage);
        }
        let salle = parts.get(6).and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        });
        let semaine_ab = parts
            .get(7)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| EdtSemaineAb::parse(s).ok_or(EdtImportParseError::Semaine))
            .transpose()?
            .unwrap_or(EdtSemaineAb::Toutes);
        out.push(EdtImportLigne {
            index: out.len(),
            classe,
            matiere_libelle: matiere,
            prof_user_id,
            jour_semaine,
            heure_debut,
            heure_fin,
            salle,
            semaine_ab,
        });
    }
    if out.is_empty() {
        return Err(EdtImportParseError::Empty);
    }
    Ok(out)
}

/// Détecte conflits intra-lot + vs créneaux existants (prof / salle / classe).
/// `existing` : tuples (classe, prof, jour, debut, fin, salle, semaine).
pub fn detect_conflits_edt(
    lines: &[EdtImportLigne],
    existing: &[EdtImportLigne],
) -> EdtImportReport {
    let mut conflicts = Vec::new();
    let all: Vec<(bool, &EdtImportLigne)> = existing
        .iter()
        .map(|l| (true, l))
        .chain(lines.iter().map(|l| (false, l)))
        .collect();
    // Compare every pair involving at least one new line.
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            let (ei, a) = all[i];
            let (ej, b) = all[j];
            if ei && ej {
                continue;
            }
            if a.jour_semaine != b.jour_semaine {
                continue;
            }
            if !semaines_chevauchent(a.semaine_ab, b.semaine_ab) {
                continue;
            }
            if !times_overlap(a.heure_debut, a.heure_fin, b.heure_debut, b.heure_fin) {
                continue;
            }
            let la = if ei { usize::MAX } else { a.index };
            let lb = if ej { usize::MAX } else { b.index };
            if a.prof_user_id == b.prof_user_id {
                conflicts.push(EdtConflict {
                    kind: EdtConflictKind::ProfDouble,
                    ligne_a: la,
                    ligne_b: lb,
                    detail: format!("prof {} chevauche", a.prof_user_id),
                });
            }
            if let (Some(sa), Some(sb)) = (&a.salle, &b.salle) {
                if sa.eq_ignore_ascii_case(sb) {
                    conflicts.push(EdtConflict {
                        kind: EdtConflictKind::SalleDouble,
                        ligne_a: la,
                        ligne_b: lb,
                        detail: format!("salle {sa} chevauche"),
                    });
                }
            }
            if a.classe.eq_ignore_ascii_case(&b.classe) {
                conflicts.push(EdtConflict {
                    kind: EdtConflictKind::ClasseDouble,
                    ligne_a: la,
                    ligne_b: lb,
                    detail: format!("classe {} chevauche", a.classe),
                });
            }
        }
    }
    let ok_count = if conflicts.is_empty() {
        lines.len()
    } else {
        0
    };
    EdtImportReport {
        lines: lines.len(),
        ok_count,
        conflicts,
    }
}

/// Affectation structurelle d'un créneau (prof et/ou salle référentiel).
#[derive(Debug, Clone)]
pub struct AffecterEdtCreneau {
    pub etablissement_id: Uuid,
    pub creneau_id: Uuid,
    /// Nouveau titulaire — `None` = inchangé.
    pub prof_user_id: Option<Uuid>,
    /// Code salle (référentiel ou libre) — `None` = inchangé ; `Some("")` = retirer.
    pub salle: Option<String>,
    pub actor_user_id: Uuid,
}

/// Résultat planifié d'une affectation (appliqué sur le créneau existant).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtCreneauAffecte {
    pub creneau_id: Uuid,
    pub etablissement_id: Uuid,
    pub prof_user_id: Uuid,
    pub salle: Option<String>,
}

pub fn plan_affecter_edt_creneau(
    current_prof: Uuid,
    current_salle: Option<&str>,
    cmd: AffecterEdtCreneau,
) -> Result<EdtCreneauAffecte, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.creneau_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.prof_user_id.is_none() && cmd.salle.is_none() {
        return Err(DomainError::EdtAffectationInvalide);
    }
    let prof = match cmd.prof_user_id {
        Some(p) if p.is_nil() => return Err(DomainError::EdtAffectationInvalide),
        Some(p) => p,
        None => current_prof,
    };
    let salle = match cmd.salle {
        None => current_salle.map(|s| s.to_string()),
        Some(s) => {
            let t = s.trim().to_string();
            if t.is_empty() {
                None
            } else if t.len() > 80 {
                return Err(DomainError::EdtAffectationInvalide);
            } else {
                Some(t)
            }
        }
    };
    Ok(EdtCreneauAffecte {
        creneau_id: cmd.creneau_id,
        etablissement_id: cmd.etablissement_id,
        prof_user_id: prof,
        salle,
    })
}

pub fn plan_poser_edt_exception(cmd: PoserEdtException) -> Result<EdtException, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.creneau_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let motif = cmd.motif.trim().to_string();
    if motif.is_empty() || motif.len() > 500 {
        return Err(DomainError::EdtExceptionInvalide);
    }
    let (remplacant, salle) = match cmd.kind {
        EdtExceptionKind::Annulation => {
            if cmd.remplacant_user_id.is_some() || cmd.nouvelle_salle.as_ref().is_some_and(|s| !s.trim().is_empty()) {
                return Err(DomainError::EdtExceptionInvalide);
            }
            (None, None)
        }
        EdtExceptionKind::Remplacement => {
            let Some(r) = cmd.remplacant_user_id.filter(|u| !u.is_nil()) else {
                return Err(DomainError::EdtExceptionInvalide);
            };
            (Some(r), None)
        }
        EdtExceptionKind::ChangementSalle => {
            let s = cmd
                .nouvelle_salle
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_string();
            if s.is_empty() || s.len() > 80 {
                return Err(DomainError::EdtExceptionInvalide);
            }
            (None, Some(s))
        }
    };
    Ok(EdtException {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        creneau_id: cmd.creneau_id,
        jour: cmd.jour,
        kind: cmd.kind,
        remplacant_user_id: remplacant,
        nouvelle_salle: salle,
        motif,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

/// Demande de créneau pour le mini-solveur greedy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtDemande {
    pub classe: String,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    /// Durée en minutes (typ. 55–60).
    pub duree_minutes: i32,
    pub salle: Option<String>,
    pub semaine_ab: EdtSemaineAb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdtSlot {
    pub jour_semaine: i16,
    pub heure_debut: chrono::NaiveTime,
    pub heure_fin: chrono::NaiveTime,
}

#[derive(Debug, Clone, Default)]
pub struct EdtSolveReport {
    pub placed: Vec<EdtImportLigne>,
    pub unplaced: Vec<EdtDemande>,
}

/// Grille standard lun–ven, créneaux 08–12 et 14–17 (1 h).
pub fn grille_edt_defaut() -> Vec<EdtSlot> {
    let mut slots = Vec::new();
    let heures = [
        (8, 9),
        (9, 10),
        (10, 11),
        (11, 12),
        (14, 15),
        (15, 16),
        (16, 17),
    ];
    for jour in 1i16..=5 {
        for (h0, h1) in heures {
            slots.push(EdtSlot {
                jour_semaine: jour,
                heure_debut: chrono::NaiveTime::from_hms_opt(h0, 0, 0).unwrap(),
                heure_fin: chrono::NaiveTime::from_hms_opt(h1, 0, 0).unwrap(),
            });
        }
    }
    slots
}

fn slot_duree_ok(slot: &EdtSlot, duree_minutes: i32) -> bool {
    let mins = (slot.heure_fin - slot.heure_debut).num_minutes() as i32;
    mins >= duree_minutes
}

fn ligne_from_demande_slot(
    index: usize,
    d: &EdtDemande,
    slot: &EdtSlot,
) -> EdtImportLigne {
    let fin = if d.duree_minutes > 0
        && d.duree_minutes < (slot.heure_fin - slot.heure_debut).num_minutes() as i32
    {
        slot.heure_debut + chrono::Duration::minutes(d.duree_minutes as i64)
    } else {
        slot.heure_fin
    };
    EdtImportLigne {
        index,
        classe: d.classe.clone(),
        matiere_libelle: d.matiere_libelle.clone(),
        prof_user_id: d.prof_user_id,
        jour_semaine: slot.jour_semaine,
        heure_debut: slot.heure_debut,
        heure_fin: fin,
        salle: d.salle.clone(),
        semaine_ab: d.semaine_ab,
    }
}

/// Contrainte soft consommée par le greedy (indispo prof / salle).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtContrainteSoft {
    pub kind: &'static str,
    pub prof_user_id: Option<Uuid>,
    pub salle_code: Option<String>,
    pub jour_semaine: Option<i16>,
    pub heure_debut: Option<chrono::NaiveTime>,
    pub heure_fin: Option<chrono::NaiveTime>,
}

fn times_overlap_opt(
    a0: chrono::NaiveTime,
    a1: chrono::NaiveTime,
    b0: Option<chrono::NaiveTime>,
    b1: Option<chrono::NaiveTime>,
) -> bool {
    match (b0, b1) {
        (Some(x0), Some(x1)) => times_overlap(a0, a1, x0, x1),
        _ => true,
    }
}

pub fn slot_interdit_soft(
    dem: &EdtDemande,
    slot: &EdtSlot,
    contraintes: &[EdtContrainteSoft],
) -> bool {
    for c in contraintes {
        if let Some(js) = c.jour_semaine {
            if js != slot.jour_semaine {
                continue;
            }
        }
        if !times_overlap_opt(slot.heure_debut, slot.heure_fin, c.heure_debut, c.heure_fin) {
            continue;
        }
        match c.kind {
            "prof_indispo" => {
                if c.prof_user_id == Some(dem.prof_user_id) {
                    return true;
                }
            }
            "salle_indispo" => {
                if let (Some(cs), Some(ds)) = (&c.salle_code, &dem.salle) {
                    if cs.eq_ignore_ascii_case(ds) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Mini-solveur greedy : place chaque demande sur le premier créneau libre sans conflit.
pub fn solve_edt_greedy(
    demandes: &[EdtDemande],
    slots: &[EdtSlot],
    existing: &[EdtImportLigne],
) -> EdtSolveReport {
    solve_edt_greedy_soft(demandes, slots, existing, &[])
}

/// Même greedy en respectant les contraintes soft (indispos).
pub fn solve_edt_greedy_soft(
    demandes: &[EdtDemande],
    slots: &[EdtSlot],
    existing: &[EdtImportLigne],
    contraintes: &[EdtContrainteSoft],
) -> EdtSolveReport {
    let mut placed: Vec<EdtImportLigne> = existing.to_vec();
    let mut out_placed = Vec::new();
    let mut unplaced = Vec::new();
    for (i, dem) in demandes.iter().enumerate() {
        if dem.classe.trim().is_empty()
            || dem.matiere_libelle.trim().is_empty()
            || dem.prof_user_id.is_nil()
            || dem.duree_minutes <= 0
        {
            unplaced.push(dem.clone());
            continue;
        }
        let mut found = None;
        for slot in slots {
            if !slot_duree_ok(slot, dem.duree_minutes) {
                continue;
            }
            if slot_interdit_soft(dem, slot, contraintes) {
                continue;
            }
            let candidate = ligne_from_demande_slot(i, dem, slot);
            let report = detect_conflits_edt(std::slice::from_ref(&candidate), &placed);
            if report.conflicts.is_empty() {
                found = Some(candidate);
                break;
            }
        }
        match found {
            Some(c) => {
                placed.push(c.clone());
                out_placed.push(c);
            }
            None => unplaced.push(dem.clone()),
        }
    }
    EdtSolveReport {
        placed: out_placed,
        unplaced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remplacement_exige_prof() {
        let err = plan_poser_edt_exception(PoserEdtException {
            etablissement_id: Uuid::new_v4(),
            creneau_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            kind: EdtExceptionKind::Remplacement,
            remplacant_user_id: None,
            nouvelle_salle: None,
            motif: "maladie".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::EdtExceptionInvalide);
    }

    #[test]
    fn affectation_exige_changement() {
        let err = plan_affecter_edt_creneau(
            Uuid::new_v4(),
            Some("A1"),
            AffecterEdtCreneau {
                etablissement_id: Uuid::new_v4(),
                creneau_id: Uuid::new_v4(),
                prof_user_id: None,
                salle: None,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::EdtAffectationInvalide);
    }

    #[test]
    fn affectation_salle_et_prof() {
        let prof = Uuid::new_v4();
        let other = Uuid::new_v4();
        let a = plan_affecter_edt_creneau(
            prof,
            Some("A1"),
            AffecterEdtCreneau {
                etablissement_id: Uuid::new_v4(),
                creneau_id: Uuid::new_v4(),
                prof_user_id: Some(other),
                salle: Some("B12".into()),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(a.prof_user_id, other);
        assert_eq!(a.salle.as_deref(), Some("B12"));
    }

    #[test]
    fn annulation_ok() {
        let e = plan_poser_edt_exception(PoserEdtException {
            etablissement_id: Uuid::new_v4(),
            creneau_id: Uuid::new_v4(),
            jour: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            kind: EdtExceptionKind::Annulation,
            remplacant_user_id: None,
            nouvelle_salle: None,
            motif: "sortie pedagogique".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(e.kind, EdtExceptionKind::Annulation);
    }

    #[test]
    fn calendrier_exige_lundi() {
        let err = plan_configurer_edt_calendrier_ab(ConfigurerEdtCalendrierAb {
            etablissement_id: Uuid::new_v4(),
            reference_lundi: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(), // mardi
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::EdtCalendrierAbInvalide);
    }

    #[test]
    fn semaine_ab_alterne() {
        let ref_lundi = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(); // lundi
        assert_eq!(
            semaine_ab_au_jour(ref_lundi, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()),
            EdtSemaineAb::A
        );
        assert_eq!(
            semaine_ab_au_jour(ref_lundi, NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()),
            EdtSemaineAb::B
        );
        assert!(creneau_actif_semaine(EdtSemaineAb::Toutes, EdtSemaineAb::B));
        assert!(!creneau_actif_semaine(EdtSemaineAb::A, EdtSemaineAb::B));
        assert!(creneau_actif_semaine(EdtSemaineAb::B, EdtSemaineAb::B));
    }

    #[test]
    fn import_detecte_conflit_prof() {
        let pid = Uuid::new_v4();
        let csv = format!(
            "classe;matiere;prof_user_id;jour_semaine;heure_debut;heure_fin;salle;semaine_ab\n\
4A;Maths;{pid};1;08:00;09:00;S1;toutes\n\
4B;Physique;{pid};1;08:30;09:30;S2;toutes\n"
        );
        let lines = parse_edt_import_csv(&csv).unwrap();
        let report = detect_conflits_edt(&lines, &[]);
        assert_eq!(lines.len(), 2);
        assert!(report
            .conflicts
            .iter()
            .any(|c| c.kind == EdtConflictKind::ProfDouble));
    }

    #[test]
    fn import_propre_sans_conflit() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let csv = format!(
            "classe;matiere;prof_user_id;jour_semaine;heure_debut;heure_fin;salle;semaine_ab\n\
4A;Maths;{p1};1;08:00;09:00;S1;toutes\n\
4B;Physique;{p2};1;08:00;09:00;S2;toutes\n"
        );
        let lines = parse_edt_import_csv(&csv).unwrap();
        let report = detect_conflits_edt(&lines, &[]);
        assert!(report.conflicts.is_empty());
        assert_eq!(report.ok_count, 2);
    }

    #[test]
    fn pronote_parse_et_resolve() {
        use chrono::Timelike;
        let pid = Uuid::new_v4();
        let csv = "Classe;Matière;Enseignant;Jour;Début;Fin;Salle;Fréquence\n\
4B;Potions;snape@hogwarts.lab;Lundi;08h00;09h00;Donjon;H\n\
4B;Astronomie;snape@hogwarts.lab;Mardi;16h00;17h00;Tour;S2\n";
        let raw = parse_edt_pronote_csv(csv).unwrap();
        assert_eq!(raw.len(), 2);
        assert_eq!(raw[0].jour_semaine, 1);
        assert_eq!(raw[1].semaine_ab, EdtSemaineAb::B);
        let lines = resolve_pronote_lignes(&raw, |email| {
            if email == "snape@hogwarts.lab" {
                Some(pid)
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(lines[0].prof_user_id, pid);
        assert_eq!(lines[0].heure_debut.hour(), 8);
    }

    #[test]
    fn solveur_greedy_place_deux_demandes() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let demandes = vec![
            EdtDemande {
                classe: "3A".into(),
                matiere_libelle: "Maths".into(),
                prof_user_id: p1,
                duree_minutes: 60,
                salle: Some("S1".into()),
                semaine_ab: EdtSemaineAb::Toutes,
            },
            EdtDemande {
                classe: "3B".into(),
                matiere_libelle: "Histoire".into(),
                prof_user_id: p2,
                duree_minutes: 60,
                salle: Some("S2".into()),
                semaine_ab: EdtSemaineAb::Toutes,
            },
        ];
        let report = solve_edt_greedy(&demandes, &grille_edt_defaut(), &[]);
        assert_eq!(report.placed.len(), 2);
        assert!(report.unplaced.is_empty());
        assert_eq!(report.placed[0].jour_semaine, 1);
    }
}
