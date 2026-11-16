//! Cahier de textes, lot C6 : **continuité** (`20_CAHIER_DE_TEXTES.md` §8.4, §12).
//!
//! Contrat JSON des routes `/cahier/c6/*` d'`ent-api`. Propositions de report quand une séance change, travail en
//! l'absence du professeur, remplaçant, suppléant, poste vacant, continuité d'un élève (absence longue, exclusion
//! temporaire), fermeture. Aucun motif d'absence ni d'exclusion n'est servi : un professeur voit « élève en continuité à
//! distance », jamais pourquoi. Jours et heures de l'établissement.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const REPORT_ENTREE: &str = "entree";
pub const REPORT_TRAVAIL: &str = "travail";

pub const CONTINUITE_ABSENCE_LONGUE: &str = "absence_longue";
pub const CONTINUITE_EXCLUSION: &str = "exclusion";

pub const CONTEXTE_ABSENCE: &str = "absence_professeur";
pub const CONTEXTE_DISTANCE: &str = "distance";
pub const CONTEXTE_VACANT: &str = "poste_vacant";

/// Séance (jour et heure de l'établissement) d'origine ou d'arrivée d'un report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceReportDto {
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: String,
}

/// Proposition de report (§8.4) : jamais d'échéance avancée sans accord, rien ne disparaît.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDto {
    pub id: Uuid,
    pub cours_id: Uuid,
    /// `entree` ou `travail`.
    pub objet: String,
    pub objet_id: Uuid,
    pub titre: String,
    pub origine: SeanceReportDto,
    /// Séance d'arrivée proposée : la prochaine du cours ; absente s'il n'y en a pas (à décider).
    pub vers: Option<SeanceReportDto>,
    /// `proposee`, `appliquee`, `refusee`, `appliquee_auto`.
    pub etat: String,
    /// Reporter ce travail avancerait l'échéance : confirmation explicite requise.
    pub avance_l_echeance: bool,
    pub traite_le: Option<DateTime<Utc>>,
}

/// L'écran « vos séances annulées » : propositions groupées par jour (un écran, une grève = un lot).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportsDto {
    pub propositions: Vec<ReportDto>,
    pub nb_ouvertes: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDecisionBody {
    /// `accepter` ou `refuser`.
    pub decision: String,
    /// Autre séance d'arrivée que celle proposée (même cours).
    #[serde(default)]
    pub vers_seance_id: Option<Uuid>,
    /// Confirmer qu'une échéance soit avancée (sinon refus).
    #[serde(default)]
    pub confirmer_avance: bool,
    #[serde(default)]
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportsLotBody {
    pub ids: Vec<Uuid>,
    pub decision: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportsLotDto {
    pub appliques: i32,
    pub refuses: i32,
    /// Propositions laissées en attente (séance d'arrivée absente, échéance qui serait avancée, accès) avec la raison.
    pub ignores: Vec<ReportIgnoreDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportIgnoreDto {
    pub id: Uuid,
    pub raison: String,
}

// ───────────────────────────── continuité d'un élève ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuiteBody {
    pub eleve_id: Uuid,
    pub du: NaiveDate,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    pub cours_ids: Vec<Uuid>,
    /// `absence_longue` (PP, CPE) ou `exclusion` (vie scolaire, direction).
    pub nature: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuiteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub cours_ids: Vec<Uuid>,
    /// Nature visible de la vie scolaire seulement ; un professeur ne la voit jamais.
    pub nature: Option<String>,
    pub close: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuiteCloreBody {
    pub au: NaiveDate,
}

/// Bandeau d'un cours : élèves en continuité à distance (sans motif) et poste vacant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BandeauCoursDto {
    pub cours_id: Uuid,
    pub eleves_en_continuite: Vec<EleveContinuiteDto>,
    /// « Professeur en cours de nomination » : aucun intervenant en saisie ; un cahier vide n'est pas « non tenu ».
    pub poste_vacant: bool,
    pub suppleance_en_cours: bool,
    pub copies_a_decider: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveContinuiteDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
}

// ───────────────────────────── absence, fermeture, poste vacant ─────────────────────────────

/// Travail déposé pour **une séance** : en l'absence du professeur (« travail en autonomie »), à distance (fermeture) ou
/// pour un poste vacant. Publié à l'heure de la séance ; l'échéance est la séance suivante du cours.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailSeanceBody {
    pub titre: String,
    pub consigne: String,
    /// `absence_professeur` ou `distance`.
    pub contexte: String,
    #[serde(default)]
    pub duree_estimee_min: Option<i16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailSeanceDto {
    pub travail_id: Uuid,
    pub seance_id: Uuid,
    pub visible_a: Option<DateTime<Utc>>,
    pub contexte: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VacantsDto {
    pub cours: Vec<CoursVacantDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoursVacantDto {
    pub cours_id: Uuid,
    pub libelle: String,
    pub cible: String,
}

// ───────────────────────────── remplaçant, suppléant ─────────────────────────────

/// Vue du remplaçant d'une séance : le cahier du cours, la liste des élèves avec les **seuls** aménagements utiles
/// (pastilles) ; ni copies ni retours.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemplacementVueDto {
    pub seance: SeanceReportDto,
    pub cours_id: Uuid,
    pub entrees: Vec<EntreeRemplacementDto>,
    pub eleves: Vec<ElevePastilleDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntreeRemplacementDto {
    pub seance: SeanceReportDto,
    pub titre: Option<String>,
    pub contenu: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElevePastilleDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    /// Faits utiles (« temps majoré », « version allégée »…) : jamais un diagnostic ni un intitulé de plan.
    pub pastilles: Vec<String>,
}

/// Écran d'accueil d'un suppléant : « où en était le cours ».
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuppleanceAccueilDto {
    pub cours_id: Uuid,
    /// Dernières entrées publiées.
    pub entrees: Vec<EntreeRemplacementDto>,
    pub travaux_en_cours: Vec<TravailEnCoursDto>,
    pub copies_en_attente: i32,
    /// La saisie est ouverte (prise de poste atteinte) ; sinon lecture seule (CDT-255).
    pub saisie_ouverte: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailEnCoursDto {
    pub id: Uuid,
    pub titre: String,
    pub echeance_jour: NaiveDate,
    pub nb_remises: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopiesDecisionBody {
    /// `je_corrige`, `titulaire_corrigera` ou `rendues_sans_correction`.
    pub decision: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopiesDecisionDto {
    pub decision: String,
    pub nb_copies: i32,
}

/// Travail donné par un remplaçant pour une séance du titulaire : à garder, modifier ou retirer (CDT-258).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailDeRemplacantDto {
    pub travail_id: Uuid,
    pub titre: String,
    pub seance: SeanceReportDto,
    pub donne_par: String,
}
