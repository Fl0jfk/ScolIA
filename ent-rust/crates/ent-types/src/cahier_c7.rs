//! Cahier de textes, lot C7 : **adaptations** (`20_CAHIER_DE_TEXTES.md` §13) — variantes, aménagements appliqués, AESH,
//! réglages d'affichage de l'élève.
//!
//! Contrat JSON des routes `/cahier/c7/*` d'`ent-api`. Règle d'or (R-C8) : **aucun motif** n'est stocké ni servi ; l'élève voit
//! **sa** version comme la seule, un camarade ne détecte jamais qu'une variante existe (aucune route d'élève ne porte
//! d'indicateur, de compteur ni de libellé de variante). Les aménagements ne sont que des **faits utiles** d'un catalogue fermé
//! (pastilles) : jamais un diagnostic, un intitulé de plan (PAP, PPS), ni la nature d'un dispositif.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───────────────────────────── variantes (professeur en saisie) ─────────────────────────────

/// Variante d'un travail : consigne, durée et échéance propres à des élèves nommés. Le libellé est **interne** au professeur.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VarianteBody {
    pub libelle_interne: String,
    pub consigne: String,
    #[serde(default)]
    pub duree_estimee_min: Option<i16>,
    #[serde(default)]
    pub echeance_jour: Option<NaiveDate>,
    #[serde(default)]
    pub echeance_heure: Option<NaiveTime>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VarianteDto {
    pub id: Uuid,
    pub travail_id: Uuid,
    pub libelle_interne: String,
    pub consigne: String,
    pub duree_estimee_min: Option<i16>,
    pub echeance_le: Option<DateTime<Utc>>,
    pub version: i32,
    pub eleves: Vec<EleveVarianteDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveVarianteDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributionBody {
    pub eleve_ids: Vec<Uuid>,
}

// ───────────────────────────── aménagements appliqués ─────────────────────────────

/// Pastilles d'un cours pour le professeur : un fait utile par pastille, jamais un diagnostic ni un intitulé de plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PastillesCoursDto {
    pub cours_id: Uuid,
    pub eleves: Vec<ElevePastillesDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElevePastillesDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub pastilles: Vec<String>,
}

/// Ce que « appliquer les aménagements » ferait pour un travail : aperçu d'abord (rien n'est écrit).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationAmenagementsDto {
    /// Élèves dont l'échéance effective serait majorée (ou l'a été) : sans la raison, seulement le nouvel instant.
    pub eleves: Vec<EcheanceMajoreeDto>,
    pub appliques: i32,
    /// `true` après l'application ; `false` pour un aperçu.
    pub applique: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcheanceMajoreeDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub echeance_le: DateTime<Utc>,
    pub echeance_jour: NaiveDate,
    pub echeance_heure: String,
}

// ───────────────────────────── AESH ─────────────────────────────

/// Élève accompagné à la date par l'AESH connecté.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveAccompagneDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
}

/// Note d'accompagnement de l'AESH, lue des professeurs du cours et du PP (« consigne reformulée en classe »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteAccompagnementBody {
    #[serde(default)]
    pub travail_id: Option<Uuid>,
    #[serde(default)]
    pub entree_id: Option<Uuid>,
    pub eleve_id: Uuid,
    pub texte: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteAccompagnementDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub texte: String,
    pub auteur: String,
    pub cree_le: DateTime<Utc>,
}

// ───────────────────────────── réglages d'affichage de l'élève ─────────────────────────────

pub const TAILLES_POLICE: [&str; 3] = ["normale", "grande", "tres_grande"];
pub const INTERLIGNES: [&str; 2] = ["normal", "large"];

/// Réglages d'affichage : accessibilité de l'élève, sans intervention du professeur. Jamais lus par un adulte.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffichageDto {
    pub taille_police: String,
    pub interligne: String,
    pub lecture_audio: bool,
    pub animations_reduites: bool,
}

impl Default for AffichageDto {
    fn default() -> Self {
        Self {
            taille_police: "normale".into(),
            interligne: "normal".into(),
            lecture_audio: false,
            animations_reduites: false,
        }
    }
}
