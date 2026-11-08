//! Identifiants externes de l'élève (socle 2, lot E3) : INE, identifiant SIECLE, badge…
//! Routes `ent-api` sous `/eleves/{id}/identifiants…`. Aucune logique métier.
//! Les statuts et provenances sont des chaînes : `declare`, `a_verifier`, `confirme`,
//! `conteste`, `revoque` ; `saisie`, `document_famille`, `ancien_etablissement`, …

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AjouterIdentifiantBody {
    /// `INE`, `SIECLE_ELEVE`, `ONDE`, `AAF_JOINTURE`, `BADGE`, `ANCIEN_LOGICIEL`.
    pub systeme: String,
    pub valeur: String,
    /// À la main : `saisie` (défaut), `document_famille`, `ancien_etablissement`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valide_du: Option<NaiveDate>,
    /// Pièce : document administratif ou familial de cet élève.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preuve_document_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmerIdentifiantBody {
    /// Ex. `verification_manuelle`, `import_aaf`.
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotifIdentifiantBody {
    pub motif: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BadgePerduBody {
    /// Nouveau badge, actif aussitôt ; absent = badge retiré seulement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nouveau_badge: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifiantDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub systeme: String,
    pub valeur: String,
    pub statut: String,
    pub provenance: String,
    pub preuve_document_id: Option<Uuid>,
    pub valide_du: NaiveDate,
    pub valide_au: Option<NaiveDate>,
    pub confirme_le: Option<DateTime<Utc>>,
    pub confirme_par_source: Option<String>,
    pub motif: Option<String>,
    pub enregistre_le: DateTime<Utc>,
    /// Un export national n'utilise que les identifiants confirmés.
    pub utilisable_pour_export: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatIdentifiantDto {
    pub identifiant: IdentifiantDto,
    /// Valeur déjà portée par un autre élève de l'établissement : enregistrée `conteste`.
    pub collision: bool,
    pub autre_eleve_id: Option<Uuid>,
    pub remplace: Option<Uuid>,
    /// L'élève portait déjà cette valeur : rien n'a été écrit.
    pub existant: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifiantConsteDto {
    pub identifiant_id: Uuid,
    pub eleve_id: Uuid,
    pub systeme: String,
    pub motif: Option<String>,
    pub autre_eleve_id: Option<Uuid>,
}
