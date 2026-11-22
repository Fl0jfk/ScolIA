//! Évaluations, lot V5 « parcours » (socle 21) : résultats externes, enseignements suivis hors de l'établissement et
//! parcours d'un élève dans une période. Compile pour `wasm32` (aucune dépendance serveur).
//!
//! Principes :
//! * **une note appartient à l'élève et à la date** (règle 6) : un changement de classe, de groupe ou d'option ne fait
//!   ni disparaître ni compter deux fois une note ;
//! * **un résultat externe ne compte pas par défaut** (CE-02) : le compter dans UN cours est une décision tracée
//!   (auteur, date, motif), distincte de sa publication à la famille ; un retrait est motivé ; rien ne se modifie ;
//! * **la famille ne voit que ce qui est publié** (règle 1), sans auteur ni motif ;
//! * **aucune donnée de santé ni motif d'absence** dans le parcours (« absent » ne dit jamais pourquoi).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Origines d'un résultat externe (liste fermée, identique à la contrainte de la base).
pub const ORIGINES_EXTERNES: [&str; 5] = ["etablissement", "cned", "etranger", "reprise", "autre"];
/// Organismes d'un enseignement suivi hors de l'établissement.
pub const ORGANISMES_EXTERNES: [&str; 5] = [
    "cned",
    "etablissement",
    "conservatoire",
    "etranger",
    "autre",
];

/// Déclarer un résultat d'un autre établissement, du CNED ou de l'étranger. Naît **non compté**, **non publié**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarerResultatExterneBody {
    pub eleve_id: Uuid,
    pub matiere_id: Uuid,
    pub origine: String,
    /// Intitulé libre (« DS de maths, lycée d'origine »), 160 caractères au plus.
    pub libelle: Option<String>,
    /// Date de l'évaluation d'origine : elle range le résultat dans une période (obligatoire sans `periode_id`).
    pub date_resultat: Option<NaiveDate>,
    pub periode_id: Option<Uuid>,
    /// Valeur et barème en millièmes de point (`15500` sur `20000`).
    pub valeur_m: i64,
    pub bareme_m: i64,
    /// Coefficient en millièmes (`1000` : coefficient 1) ; 1000 par défaut.
    #[serde(default)]
    pub coef_m: Option<i64>,
    /// Pièce du dossier (socle 7), si le relevé a été déposé.
    pub document_id: Option<Uuid>,
}

/// Décider de compter le résultat dans la moyenne d'UN cours (geste explicite et tracé).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompterExterneBody {
    pub cours_id: Uuid,
    /// 3 à 500 caractères.
    pub motif: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetirerExterneBody {
    /// 3 à 500 caractères.
    pub motif: String,
}

/// Un résultat externe tel que le voit le **personnel**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatExterneDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub matiere_id: Uuid,
    pub matiere_libelle: String,
    pub origine: String,
    pub libelle: Option<String>,
    pub date_resultat: Option<NaiveDate>,
    pub periode_id: Option<Uuid>,
    pub valeur_m: i64,
    pub bareme_m: i64,
    pub coef_m: i64,
    /// Cours dans la moyenne duquel le résultat compte (`None` : ne compte pas).
    pub compte_dans_cours_id: Option<Uuid>,
    pub compte_motif: Option<String>,
    pub publiee: bool,
    pub retiree: bool,
    pub motif_retrait: Option<String>,
}

/// Un résultat externe tel que le voit la **famille** ou l'élève : publié et compté seulement, ni auteur ni motif.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatExterneFamilleDto {
    pub id: Uuid,
    pub matiere_libelle: String,
    pub libelle: Option<String>,
    pub date_resultat: Option<NaiveDate>,
    pub valeur_m: i64,
    pub bareme_m: i64,
    /// Mention neutre : « évalué par un autre organisme ».
    pub mention: String,
}

/// Mention neutre affichée pour tout résultat ou moyenne venus d'ailleurs (CE-06).
pub const MENTION_AUTRE_ORGANISME: &str = "évalué par un autre organisme";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarerEnseignementExterneBody {
    pub eleve_id: Uuid,
    pub matiere_id: Uuid,
    pub organisme: String,
    pub organisme_libelle: Option<String>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub referent: Option<String>,
    /// La moyenne externe vaut moyenne de matière (R-V38) : décision de la direction, non modifiable (redéclarer).
    pub compte_moyenne_generale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnseignementExterneDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub matiere_id: Uuid,
    pub organisme: String,
    pub organisme_libelle: Option<String>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub compte_moyenne_generale: bool,
    pub moyennes: Vec<MoyenneExterneDto>,
}

/// Saisir (ou corriger, avec journal) la moyenne d'un enseignement externe pour une période.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaisirMoyenneExterneBody {
    pub periode_id: Uuid,
    /// `moyenne`, `en_attente` ou `dispense`.
    pub statut: String,
    /// En millièmes ; présente si et seulement si le statut est `moyenne`.
    pub valeur_m: Option<i64>,
    /// En millièmes ; 20000 par défaut.
    #[serde(default)]
    pub bareme_m: Option<i64>,
    pub appreciation_transcrite: Option<String>,
    pub document_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoyenneExterneDto {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub statut: String,
    pub valeur_m: Option<i64>,
    pub bareme_m: i64,
    pub appreciation_transcrite: Option<String>,
}

// ───────────────────────────── Parcours d'un élève dans une période ─────────────────────────────

/// Un passage dans une classe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentClasseDto {
    pub classe_id: Uuid,
    pub classe_code: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
}

/// Un enseignement suivi dans la période, avec ses dates (option abandonnée, groupe changé, arrivée tardive).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnseignementSuiviDto {
    pub cours_id: Uuid,
    pub matiere_libelle: String,
    /// Classe d'origine du cours (code), pour dire d'où vient chaque note (CE-09, CE-11).
    pub classe_code: Option<String>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    /// « abandonné le … » : le suivi s'arrête dans la période alors que l'élève reste dans l'établissement (CE-13).
    pub abandonne_le: Option<NaiveDate>,
}

/// Une évaluation d'un cours de l'élève dans la période, et la place qu'elle y tient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationParcoursDto {
    pub evaluation_id: Uuid,
    pub titre: String,
    pub date_eval: NaiveDate,
    pub matiere_libelle: String,
    pub classe_code: Option<String>,
    /// `false` : l'élève n'était pas dans le public du cours à cette date (arrivé après, déjà parti, option quittée) :
    /// `non_concerne`, jamais un zéro ni une absence (CE-01).
    pub concerne: bool,
    /// Statut du résultat s'il existe (`note`, `absent`, `dispense`…) ; la valeur n'est pas dans le parcours.
    pub statut: Option<String>,
}

/// Le parcours d'un élève dans une période, pour le **personnel** (jamais la famille, jamais un motif).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParcoursPeriodeDto {
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    /// Classe de rattachement pour la période : la seule qui produit son bulletin (D15).
    pub classe_rattachement_id: Option<Uuid>,
    pub classe_rattachement_code: Option<String>,
    pub segments: Vec<SegmentClasseDto>,
    /// « arrivé le … » : première affectation postérieure au début de la période (CE-01).
    pub arrive_le: Option<NaiveDate>,
    /// Date de sortie de l'établissement dans la période (CE-03).
    pub parti_le: Option<NaiveDate>,
    pub enseignements: Vec<EnseignementSuiviDto>,
    pub evaluations: Vec<EvaluationParcoursDto>,
    pub resultats_externes: Vec<ResultatExterneDto>,
}
