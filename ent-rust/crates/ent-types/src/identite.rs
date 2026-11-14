//! Identité versionnée de l'élève (socle 2, lot E2) : corrections, changements d'état
//! civil, noms d'usage et historique. Routes `ent-api` sous `/eleves/{id}/identite/…`.
//! Aucune logique métier. Les états restent des chaînes (`nature` : `correction`,
//! `changement_etat_civil`, `usage`, `creation`, `reprise`, `import`, `fusion`).
//!
//! Champ facultatif : absent = inchangé ; `null` = retiré ; valeur = remplacé.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

/// Distingue « champ absent » (`None`) de « champ à `null` » (`Some(None)`).
fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// Valeurs d'identité à modifier (toutes facultatives).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampsIdentiteBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nom_naissance: Option<String>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub nom_usage: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prenoms: Option<Vec<String>>,
    /// Rang (à partir de 1) du prénom usuel parmi les prénoms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rang_prenom_usuel: Option<i16>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub prenom_usage: Option<Option<String>>,
    /// `"M"` ou `"F"`.
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub sexe: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub date_naissance: Option<Option<NaiveDate>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub naissance_commune: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub naissance_commune_cog: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub naissance_pays_cog: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub naissance_lieu_etranger: Option<Option<String>>,
    /// `jour`, `mois`, `annee` ou `inconnue` (« inconnue » efface la date).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_naissance_precision: Option<String>,
    /// `declaree`, `justifiee`, `presumee` ou `contestee`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_naissance_statut: Option<String>,
    /// Graphie d'origine (écriture non latine).
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub nom_ecriture_origine: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub prenoms_ecriture_origine: Option<Option<String>>,
}

/// `POST /eleves/{id}/identite/corrections` (ELV-03) : la valeur précédente était
/// fausse depuis le début.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrigerIdentiteBody {
    /// Version corrigée ; à défaut, la version courante.
    #[serde(default)]
    pub version_id: Option<Uuid>,
    pub champs: ChampsIdentiteBody,
    /// Code du motif (obligatoire) : `faute_de_frappe`, `casse_accents`,
    /// `erreur_de_transcription`, `d_apres_piece`, `erreur_d_import` ou `autre`.
    pub motif_code: String,
    /// Texte libre (500 caractères au plus), obligatoire seulement pour `autre`.
    #[serde(default)]
    pub motif: Option<String>,
    /// Document de l'élève (déposé par la route des documents), facultatif.
    #[serde(default)]
    pub piece_document_id: Option<Uuid>,
    /// `saisie` par défaut.
    #[serde(default)]
    pub source: Option<String>,
}

/// `POST /eleves/{id}/identite/changements` (ELV-04) : la personne a changé à une date.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangerEtatCivilBody {
    pub champs: ChampsIdentiteBody,
    pub effet_du: NaiveDate,
    /// Code du motif (obligatoire) : `decision_etat_civil`, `adoption`, `changement_prenom`,
    /// `changement_mention_sexe`, `reconnaissance`, `protection` ou `autre`. Adoption,
    /// changement de prénom ou de mention de sexe et protection rendent la version sensible.
    pub motif_code: String,
    #[serde(default)]
    pub motif: Option<String>,
    /// Obligatoire : acte, jugement, livret de famille… (document privé de l'élève).
    pub piece_document_id: Uuid,
    #[serde(default)]
    pub source: Option<String>,
}

/// `POST /eleves/{id}/identite/usages` (ELV-05) : nom ou prénom d'usage.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinirUsageBody {
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub nom_usage: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub prenom_usage: Option<Option<String>>,
    #[serde(default)]
    pub effet_du: Option<NaiveDate>,
    /// `demande_famille`, `demande_eleve` ou `autre`.
    #[serde(default)]
    pub motif_code: Option<String>,
    #[serde(default)]
    pub motif: Option<String>,
    #[serde(default)]
    pub piece_document_id: Option<Uuid>,
    #[serde(default)]
    pub source: Option<String>,
}

/// Une version d'identité (ligne d'historique).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentiteVersionDto {
    pub id: Uuid,
    pub nom_naissance: String,
    pub nom_usage: Option<String>,
    pub prenoms: Vec<String>,
    pub rang_prenom_usuel: i16,
    pub prenom_usage: Option<String>,
    pub sexe: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub naissance_commune: Option<String>,
    pub naissance_commune_cog: Option<String>,
    pub naissance_pays_cog: Option<String>,
    pub naissance_lieu_etranger: Option<String>,
    pub date_naissance_precision: String,
    pub date_naissance_statut: String,
    pub nom_ecriture_origine: Option<String>,
    pub prenoms_ecriture_origine: Option<String>,
    pub nature: String,
    pub effet_du: NaiveDate,
    /// Version corrigée par celle-ci.
    pub remplace_version_id: Option<Uuid>,
    /// Version qui a corrigé celle-ci (absent si elle n'est pas visible du lecteur).
    pub remplacee_par: Option<Uuid>,
    pub motif_code: Option<String>,
    pub motif: Option<String>,
    /// À ouvrir par `POST /documents/{id}/lien` (URL pré-signée courte).
    pub piece_document_id: Option<Uuid>,
    pub source: String,
    pub sensible: bool,
    pub enregistre_le: DateTime<Utc>,
    pub enregistre_par: Option<Uuid>,
}

/// Réponse d'une commande : version créée et NOMS des champs modifiés.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatIdentiteDto {
    pub version: IdentiteVersionDto,
    pub champs: Vec<String>,
}
