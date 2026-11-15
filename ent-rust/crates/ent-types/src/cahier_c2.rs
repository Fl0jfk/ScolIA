//! Cahier de textes, lot C2 : saisie et publication (entrées par cours et par séance, travaux).
//!
//! Contrat JSON des routes `/cahier/c2/*` d'`ent-api`. Le serveur donne les jours et heures **de
//! l'établissement** (`jour`, `heure_debut`, `echeance_jour`…) : le client ne convertit aucun instant UTC
//! et n'envoie jamais d'heure UTC saisie à la main (R-C5, `FRONTEND_LEPTOS.md` §13).

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Cours dans lequel l'acteur peut écrire à la date du jour (R-C2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoursSaisieDto {
    pub cours_id: Uuid,
    pub libelle: String,
    /// Code de la classe ou du groupe public du cours.
    pub cible: String,
}

/// Nature de travail (préréglage de l'établissement, §9.1) avec sa durée proposée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatureTravailDto {
    pub code: String,
    pub libelle: String,
    pub duree_defaut_min: Option<i16>,
}

/// Contribution signée d'un co-intervenant (CDT-125).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionDto {
    pub id: Uuid,
    pub auteur: String,
    pub contenu: String,
    pub version: i32,
    pub cree_le: DateTime<Utc>,
}

/// Entrée du cahier d'un cours pour une séance (états `brouillon`, `programmee`, `publiee`, `retiree`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntreeDto {
    pub id: Uuid,
    pub cours_id: Uuid,
    pub seance_id: Uuid,
    pub titre: Option<String>,
    pub contenu: String,
    pub notions: Vec<String>,
    pub etat: String,
    /// Publication demandée ou effective.
    pub publie_a: Option<DateTime<Utc>>,
    /// Début de visibilité des élèves et des familles (plage calme appliquée, R-C5).
    pub visible_a: Option<DateTime<Utc>>,
    /// Jour et heure de l'établissement de `visible_a`, lisibles tels quels (« visible lundi à 7 h »).
    pub visible_jour: Option<NaiveDate>,
    pub visible_heure: Option<String>,
    /// Vrai si la publication est différée par la plage calme (rappel du droit à la déconnexion à l'auteur).
    pub differee_par_plage_calme: bool,
    pub auteur: String,
    pub version: i32,
    pub modifie_le: DateTime<Utc>,
    /// Saisie après la séance : date de saisie (« renseignée le … », CDT-130).
    pub renseignee_le: Option<DateTime<Utc>>,
    pub retire_motif: Option<String>,
    pub copie_de_id: Option<Uuid>,
    pub contributions: Vec<ContributionDto>,
}

/// Version précédente lisible (R-C29 : versions visibles des lecteurs d'une entrée publiée).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionDto {
    pub version: i32,
    pub titre: Option<String>,
    pub contenu: String,
    pub etat: String,
    pub auteur: String,
    pub motif: Option<String>,
    /// La modification qui a remplacé cette version n'a pas prévenu les lecteurs.
    pub sans_avis: bool,
    pub cree_le: DateTime<Utc>,
}

/// Résumé d'un travail dans la vue d'un cours.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailResumeDto {
    pub id: Uuid,
    pub titre: String,
    pub nature: String,
    pub etat: String,
    pub echeance_jour: NaiveDate,
    pub echeance_heure: String,
    pub public: String,
}

/// Séance datée d'un cours avec son entrée (vide tant que rien n'est écrit) et ses travaux (écran 1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceDuCoursDto {
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: String,
    pub heure_fin: String,
    pub statut: String,
    pub entree: Option<EntreeDto>,
    pub travaux_donnes: Vec<TravailResumeDto>,
    pub travaux_attendus: Vec<TravailResumeDto>,
}

/// `publication` : `brouillon`, `maintenant`, `programmer` (avec `publie_a`, sinon l'heure par défaut du réglage).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntreeBody {
    #[serde(default)]
    pub titre: Option<String>,
    pub contenu: String,
    #[serde(default)]
    pub notions: Vec<String>,
    pub publication: String,
    #[serde(default)]
    pub publie_a: Option<DateTime<Utc>>,
    /// Programmation à l'heure de l'établissement (jour, et heure : 7 h par défaut) : le client n'envoie jamais d'UTC.
    #[serde(default)]
    pub publie_jour: Option<NaiveDate>,
    #[serde(default)]
    pub publie_heure: Option<NaiveTime>,
    /// Modification mineure : l'avis est omis, la mention « modifié le » et les versions restent lisibles (R-C29).
    #[serde(default)]
    pub sans_avis: bool,
    /// Motif de la modification d'une entrée publiée (facultatif).
    #[serde(default)]
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetraitC2Body {
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopieSeanceBody {
    pub vers: Vec<CopieVersBody>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopieVersBody {
    pub cours_id: Uuid,
    pub seance_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionBody {
    pub contenu: String,
}

/// Travail d'un cours, tel que saisi, relu et modifié (écran 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailC2Dto {
    pub id: Uuid,
    pub cours_id: Uuid,
    /// Travail donné à plusieurs cours : identifiant du premier travail de l'ensemble.
    pub relie_id: Option<Uuid>,
    pub seance_donnee_id: Option<Uuid>,
    pub donne_le: NaiveDate,
    pub echeance_mode: String,
    pub echeance_seance_id: Option<Uuid>,
    pub echeance_le: DateTime<Utc>,
    pub echeance_jour: NaiveDate,
    pub echeance_heure: String,
    pub nature: String,
    pub titre: String,
    pub consigne: String,
    pub duree_estimee_min: Option<i16>,
    pub rendu: String,
    pub retard_accepte: bool,
    pub formats: Vec<String>,
    /// `cours` ou `restreint` (liste d'élèves du cours).
    pub public: String,
    pub nb_eleves_cibles: i64,
    pub ia_autorisee: String,
    pub etat: String,
    pub publie_a: Option<DateTime<Utc>>,
    pub visible_a: Option<DateTime<Utc>>,
    pub differee_par_plage_calme: bool,
    pub auteur: String,
    pub version: i32,
    pub modifie_le: DateTime<Utc>,
    pub retire_motif: Option<String>,
}

/// Création : un travail par cours, reliés (CDT-133).
///
/// Échéance : exactement l'une de `echeance_seance_id` (un seul cours), `echeance_prochaine_seance`
/// (la prochaine séance de chaque cours), ou `echeance_jour` (+ `echeance_heure` locale, 19 h par défaut).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailC2Body {
    pub cours_ids: Vec<Uuid>,
    #[serde(default)]
    pub seance_donnee_id: Option<Uuid>,
    #[serde(default)]
    pub echeance_seance_id: Option<Uuid>,
    #[serde(default)]
    pub echeance_prochaine_seance: bool,
    #[serde(default)]
    pub echeance_jour: Option<NaiveDate>,
    #[serde(default)]
    pub echeance_heure: Option<NaiveTime>,
    pub nature: String,
    pub titre: String,
    pub consigne: String,
    #[serde(default)]
    pub duree_estimee_min: Option<i16>,
    /// `aucun`, `en_ligne`, `en_classe`.
    #[serde(default = "rendu_aucun")]
    pub rendu: String,
    #[serde(default = "vrai")]
    pub retard_accepte: bool,
    #[serde(default)]
    pub formats: Vec<String>,
    /// Non vide : public restreint à ces élèves (qui doivent tous appartenir au cours, R-C25).
    #[serde(default)]
    pub eleve_ids: Vec<Uuid>,
    /// `non` (défaut), `recherche`, `relecture`, `libre`.
    #[serde(default = "ia_non")]
    pub ia_autorisee: String,
    pub publication: String,
    #[serde(default)]
    pub publie_a: Option<DateTime<Utc>>,
    #[serde(default)]
    pub publie_jour: Option<NaiveDate>,
    #[serde(default)]
    pub publie_heure: Option<NaiveTime>,
}

fn rendu_aucun() -> String {
    "aucun".into()
}
fn vrai() -> bool {
    true
}
fn ia_non() -> String {
    "non".into()
}

/// Modification d'un travail non retiré ; les champs absents ne changent pas.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ModifierTravailC2Body {
    #[serde(default)]
    pub titre: Option<String>,
    #[serde(default)]
    pub consigne: Option<String>,
    #[serde(default)]
    pub duree_estimee_min: Option<i16>,
    #[serde(default)]
    pub nature: Option<String>,
    #[serde(default)]
    pub rendu: Option<String>,
    #[serde(default)]
    pub echeance_seance_id: Option<Uuid>,
    #[serde(default)]
    pub echeance_jour: Option<NaiveDate>,
    #[serde(default)]
    pub echeance_heure: Option<NaiveTime>,
    /// Confirmation explicite d'une échéance **avancée** (toujours prévenante, §9.1).
    #[serde(default)]
    pub confirme_echeance_avancee: bool,
    /// Modification mineure : pas d'avis (jamais pour une échéance avancée).
    #[serde(default)]
    pub sans_avis: bool,
    #[serde(default)]
    pub motif: Option<String>,
    /// Publier un brouillon ou reprogrammer : `maintenant`, `programmer`, ou absent (état inchangé).
    #[serde(default)]
    pub publication: Option<String>,
    #[serde(default)]
    pub publie_a: Option<DateTime<Utc>>,
    #[serde(default)]
    pub publie_jour: Option<NaiveDate>,
    #[serde(default)]
    pub publie_heure: Option<NaiveTime>,
}

/// Réponse à la création : travaux créés et avertissements non bloquants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravauxCreesDto {
    pub travaux: Vec<TravailC2Dto>,
    /// Codes : `donne_moins_de_24h`, `differe_par_plage_calme`.
    pub avertissements: Vec<String>,
}

/// Réponse à une modification : le travail, et si l'avis a été émis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailModifieDto {
    pub travail: TravailC2Dto,
    pub avis_emis: bool,
    pub echeance_avancee: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionTravailDto {
    pub version: i32,
    pub titre: String,
    pub consigne: String,
    pub duree_estimee_min: Option<i16>,
    pub echeance_le: Option<DateTime<Utc>>,
    pub etat: String,
    pub auteur: String,
    pub motif: Option<String>,
    pub sans_avis: bool,
    pub cree_le: DateTime<Utc>,
}
