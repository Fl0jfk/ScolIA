//! Socle 27, palier G5 : **registre des violations de données**. Contrat JSON des routes `/rgpd/violations/*`
//! d'`ent-api` (`27_RGPD_ARCHIVES.md` §8.7, §14.10, §18.2 point 5).
//!
//! Deux publics, deux contrats :
//! * le **personnel qui déclare** ([`DeclarationBody`], [`MaDeclarationDto`]) : trois champs, puis la référence et
//!   l'avancement, rien d'autre (R-RG32, CL27-45) ;
//! * les **gestionnaires** (direction, DPO, référent sécurité) : registre ([`ViolationLigneDto`]) et fiche
//!   ([`ViolationDto`]) avec horloge, qualification, risque proposé et décidé, notifications, mesures, chronologie.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Délai de notification à la CNIL, en heures (art. 33.1).
pub const DELAI_CNIL_HEURES: i64 = 72;

// ─────────────────────────────── Entrées ───────────────────────────────

/// Déclaration en trente secondes (RGP-240). Seul `intitule` est obligatoire.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarationBody {
    /// Ce qui s'est passé, en une phrase.
    pub intitule: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Depuis quand (instant du constat), si on le sait.
    #[serde(default)]
    pub constatee_le: Option<DateTime<Utc>>,
    /// Incident de sécurité sans atteinte aux données personnelles (RGP-259).
    #[serde(default)]
    pub incident_securite: bool,
}

/// Saisie par un gestionnaire : la prise de connaissance peut précéder la saisie (jamais la suivre).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaisieViolationBody {
    #[serde(flatten)]
    pub declaration: DeclarationBody,
    #[serde(default)]
    pub connue_le: Option<DateTime<Utc>>,
}

/// Qualification (RGP-243).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationViolationBody {
    /// `violation` ou `incident_securite`.
    pub nature_incident: String,
    #[serde(default)]
    pub confidentialite: bool,
    #[serde(default)]
    pub integrite: bool,
    #[serde(default)]
    pub disponibilite: bool,
    /// Codes du catalogue de conservation (`scol.registre`, `eva.note`, `sante.dossier`…).
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub personnes_nombre: Option<i32>,
    #[serde(default = "vrai")]
    pub personnes_estime: bool,
    #[serde(default)]
    pub personnes_mineurs: bool,
    #[serde(default)]
    pub consequences: Option<String>,
}

fn vrai() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MesureBody {
    /// `prise` ou `envisagee`.
    pub nature: String,
    pub texte: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TexteBody {
    pub texte: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RisqueBody {
    /// `faible`, `avere` ou `eleve`.
    pub risque: String,
    pub motif: String,
}

/// Notification faite par l'établissement sur le téléservice de la CNIL (ScolIA ne transmet rien).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationCnilBody {
    pub notifiee_le: DateTime<Utc>,
    pub reference_cnil: String,
    /// Exigé au-delà de 72 h (art. 33.1) ; ignoré dans le délai.
    #[serde(default)]
    pub retard_motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplementCnilBody {
    pub notifiee_le: DateTime<Utc>,
    pub reference_cnil: String,
    pub resume: String,
}

/// Calcul des personnes concernées (RGP-244) : exactement une origine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "origine", rename_all = "snake_case")]
pub enum PersonnesBody {
    /// Élèves affectés à la classe (liste de classe divulguée).
    Classe { classe_id: Uuid },
    /// Élèves désignés un par un.
    Eleves { eleve_ids: Vec<Uuid> },
    /// Comptes désignés un par un (identifiants de personnels).
    Comptes { compte_ids: Vec<Uuid> },
}

/// Information des personnes (RGP-252) : le message et la **confirmation par le nombre** de destinataires de l'aperçu.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformationBody {
    pub message: String,
    pub confirmation: i64,
}

/// Filtre du registre (paramètres de requête).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiltreRegistreQuery {
    #[serde(default)]
    pub annee: Option<i32>,
    #[serde(default)]
    pub etat: Option<String>,
    #[serde(default)]
    pub nature_incident: Option<String>,
    #[serde(default)]
    pub a_notifier: Option<bool>,
}

// ─────────────────────────────── Sorties ───────────────────────────────

/// Ce que voit le déclarant (R-RG32).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaDeclarationDto {
    pub id: Uuid,
    pub reference: String,
    pub intitule: String,
    pub connue_le: DateTime<Utc>,
    /// `recue`, `en_cours` ou `close`.
    pub avancement: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MesureDto {
    pub nature: String,
    pub texte: String,
    pub le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationCnilDto {
    pub rang: i32,
    pub nature: String,
    pub notifiee_le: DateTime<Utc>,
    pub reference_cnil: String,
    pub resume: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtapeViolationDto {
    pub etape: String,
    pub texte: Option<String>,
    pub par_role: Option<String>,
    pub le: DateTime<Utc>,
}

/// Fiche d'une violation (gestionnaires).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViolationDto {
    pub id: Uuid,
    pub reference: String,
    pub nature_incident: String,
    pub origine: String,
    pub plateforme: bool,
    pub source_code: Option<String>,
    pub declaree_par_compte_id: Option<Uuid>,
    pub intitule: String,
    pub description: Option<String>,
    pub constatee_le: Option<DateTime<Utc>>,
    pub connue_le: DateTime<Utc>,
    pub echeance_cnil_le: DateTime<Utc>,
    pub heures_restantes: i64,
    pub en_retard: bool,
    pub confidentialite: bool,
    pub integrite: bool,
    pub disponibilite: bool,
    pub categories: Vec<String>,
    pub personnes_nombre: Option<i32>,
    pub personnes_estime: bool,
    pub personnes_mineurs: bool,
    pub consequences: Option<String>,
    pub qualifiee_le: Option<DateTime<Utc>>,
    pub risque_propose: Option<String>,
    pub raisons_proposition: Vec<String>,
    pub avis_dpo: Option<String>,
    pub avis_dpo_le: Option<DateTime<Utc>>,
    pub risque: Option<String>,
    pub risque_motif: Option<String>,
    pub risque_decide_le: Option<DateTime<Utc>>,
    pub notifiee_cnil_le: Option<DateTime<Utc>>,
    pub reference_cnil: Option<String>,
    pub retard_motif: Option<String>,
    pub non_notification_motif: Option<String>,
    pub personnes_informees_le: Option<DateTime<Utc>>,
    pub personnes_informees_nombre: Option<i32>,
    pub personnes_non_information_motif: Option<String>,
    pub etat: String,
    pub close_le: Option<DateTime<Utc>>,
    pub retour_experience: Option<String>,
    pub revue_le: Option<NaiveDate>,
    pub mesures: Vec<MesureDto>,
    pub notifications: Vec<NotificationCnilDto>,
    pub chronologie: Vec<EtapeViolationDto>,
    pub manque_pour_clore: Vec<String>,
    /// Violation de la plateforme propagée (RGP-255) : les seuls éléments de cet établissement.
    #[serde(default)]
    pub plateforme_element: Option<ElementPlateformeDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementPlateformeDto {
    pub reference: String,
    pub intitule: String,
    pub description: String,
    pub connue_editeur_le: DateTime<Utc>,
    pub informe_le: DateTime<Utc>,
    pub elements: String,
    pub personnes_nombre: Option<i32>,
    pub couche_compte: bool,
    pub notifiee_cnil_par_editeur_le: Option<DateTime<Utc>>,
}

/// Une ligne du registre (RGP-258).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViolationLigneDto {
    pub id: Uuid,
    pub reference: String,
    pub nature_incident: String,
    pub origine: String,
    pub intitule: String,
    pub connue_le: DateTime<Utc>,
    pub echeance_cnil_le: DateTime<Utc>,
    pub heures_restantes: i64,
    pub en_retard: bool,
    pub risque: Option<String>,
    pub notifiee_cnil_le: Option<DateTime<Utc>>,
    pub non_notifiee: bool,
    pub personnes_nombre: Option<i32>,
    pub etat: String,
    pub close_le: Option<DateTime<Utc>>,
}

/// Aperçu avant l'information des personnes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuInformationDto {
    pub personnes: i64,
    pub destinataires: i64,
    pub personnes_sans_destinataire: i64,
    pub mineurs: i64,
    pub deja_informees: bool,
}

/// Brouillon de notification à la CNIL (RGP-247), rubrique par rubrique de l'art. 33.3, à recopier sur le
/// téléservice. Les rubriques non renseignées le disent (R-RG11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrouillonCnilDto {
    pub reference: String,
    pub rubriques: Vec<RubriqueDto>,
    /// Rubriques encore à compléter avant de notifier.
    pub a_completer: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RubriqueDto {
    pub titre: String,
    pub texte: String,
}

// ─────────────────────────────── Libellés ───────────────────────────────

pub fn libelle_risque(r: &str) -> &'static str {
    match r {
        "faible" => "faible",
        "avere" => "avéré",
        "eleve" => "élevé",
        _ => "inconnu",
    }
}

pub fn libelle_etat(e: &str) -> &'static str {
    match e {
        "ouverte" => "à qualifier",
        "qualifiee" => "qualifiée",
        "notifiee" => "notifiée à la CNIL",
        "close" => "close",
        _ => "inconnu",
    }
}

pub fn libelle_avancement(a: &str) -> &'static str {
    match a {
        "recue" => "reçue, en attente de traitement",
        "en_cours" => "en cours de traitement",
        "close" => "traitée",
        _ => "inconnu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_minimale_se_lit_avec_le_seul_intitule() {
        let d: DeclarationBody = serde_json::from_str(r#"{"intitule":"clé USB perdue"}"#).unwrap();
        assert_eq!(d.intitule, "clé USB perdue");
        assert!(!d.incident_securite);
        assert!(d.description.is_none());
    }

    #[test]
    fn saisie_aplatit_la_declaration() {
        let s: SaisieViolationBody =
            serde_json::from_str(r#"{"intitule":"appel reçu","connue_le":"2027-03-01T08:00:00Z"}"#).unwrap();
        assert_eq!(s.declaration.intitule, "appel reçu");
        assert!(s.connue_le.is_some());
    }

    #[test]
    fn personnes_se_lisent_par_origine() {
        let p: PersonnesBody =
            serde_json::from_str(r#"{"origine":"classe","classe_id":"00000000-0000-0000-0000-000000000001"}"#).unwrap();
        assert!(matches!(p, PersonnesBody::Classe { .. }));
        assert!(serde_json::from_str::<PersonnesBody>(r#"{"origine":"tous"}"#).is_err());
    }

    #[test]
    fn libelles() {
        assert_eq!(libelle_risque("avere"), "avéré");
        assert_eq!(libelle_etat("notifiee"), "notifiée à la CNIL");
        assert_eq!(libelle_avancement("en_cours"), "en cours de traitement");
    }
}
