//! Socle 27, lot G3b : **extrait d'accès**. Contrat JSON des routes `/rgpd/demandes/{id}/extraits/*` et
//! `/rgpd/reglages` d'`ent-api` (`27_RGPD_ARCHIVES.md` §9.4, §12.2).
//!
//! Le **contenu** d'un extrait n'est jamais stocké : [`ExtraitDto`] ne porte que sa structure, ses empreintes et les
//! décisions humaines ; [`ContenuExtraitDto`] est reconstruit à la lecture, tracée avant d'être servie (R-RG8).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Une section d'extrait, sans contenu.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionExtraitDto {
    pub fournisseur: String,
    pub socle: String,
    /// `ok`, `echec`, `vide`, `non_couverte`, `exclue`.
    pub statut: String,
    pub motif_code: Option<String>,
    pub lignes: i32,
    /// Décision motivée de la direction de remettre malgré une section incomplète (R-RG18).
    pub remise_incomplete_motif: Option<String>,
}

/// Un extrait (structure seulement).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtraitDto {
    pub id: Uuid,
    pub demande_id: Uuid,
    pub sujet_type: String,
    pub sujet_id: Uuid,
    pub version: i32,
    pub assemble_le: DateTime<Utc>,
    pub complet: bool,
    pub perime: bool,
    pub perime_motif: Option<String>,
    pub nb_sections: i32,
    pub nb_lignes: i32,
    pub empreinte: String,
    pub sections: Vec<SectionExtraitDto>,
    pub soumis_dpo: bool,
    pub avis_dpo_code: Option<String>,
    pub avis_dpo_texte: Option<String>,
    /// Complet, ou chaque section incomplète décidée par la direction.
    pub remissible_en_l_etat: bool,
    /// Marquages restant à décider : aucune remise tant qu'il en reste.
    pub masquages_a_decider: i32,
}

/// Un élément marqué à relire.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarquageDto {
    pub id: Uuid,
    pub fournisseur: String,
    pub table: String,
    pub ligne_id: Option<Uuid>,
    pub rang: i32,
    pub colonne: Option<String>,
    /// Raison visible du lecteur : `a_arbitrer` pour une protection quand il n'a pas `droits.arbitrer` (R-RG46).
    pub raison: String,
    /// `sys` (règle déterministe) ou `agent`.
    pub propose_par: String,
    /// `a_decider`, `masque`, `leve`.
    pub decision: String,
    pub motif: Option<String>,
    pub decide_le: Option<DateTime<Utc>>,
    /// Sa levée demande `droits.arbitrer`.
    pub levee_reservee_direction: bool,
}

/// Une ligne de contenu : `(colonne, valeur)` triées par colonne.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneExtraitDto {
    pub id: Option<Uuid>,
    pub valeurs: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableExtraitDto {
    pub nom: String,
    pub lignes: Vec<LigneExtraitDto>,
}

/// Ce qui est écarté d'une section, avec sa raison.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExclusionDto {
    pub table: String,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionContenuDto {
    pub fournisseur: String,
    pub socle: String,
    pub statut: String,
    pub motif_code: Option<String>,
    pub tables: Vec<TableExtraitDto>,
    pub exclusions: Vec<ExclusionDto>,
    pub lignes: usize,
}

/// Le contenu reconstruit d'un extrait, pour la relecture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContenuExtraitDto {
    pub sujet_type: String,
    pub sujet_id: Uuid,
    pub periode_du: Option<NaiveDate>,
    pub periode_au: Option<NaiveDate>,
    pub sections: Vec<SectionContenuDto>,
    pub complet: bool,
    pub empreinte: String,
    pub nb_lignes: usize,
    pub destinataires_internes: Vec<String>,
    pub destinataires_externes: Vec<String>,
    pub soustraitants: Vec<String>,
    pub marquages: Vec<MarquageDto>,
}

// ─────────────────────────────── Entrées ───────────────────────────────

/// Assemblage : un seul sujet, ou tous quand les deux champs sont absents.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssemblerBody {
    #[serde(default)]
    pub sujet_type: Option<String>,
    #[serde(default)]
    pub sujet_id: Option<Uuid>,
}

/// Ajout d'un sujet (fratrie, RGP-302).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SujetBody {
    pub sujet_type: String,
    pub sujet_id: Uuid,
}

/// Décision de remise d'une section incomplète (R-RG18) : un motif écrit par section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionSectionBody {
    pub fournisseur: String,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArbitrageIncompletBody {
    pub decisions: Vec<DecisionSectionBody>,
}

/// Avis du DPO sur l'extrait soumis : `favorable` ou `reserves`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvisDpoBody {
    pub avis: String,
    pub texte: String,
}

/// Décision sur un marquage : `masque` ou `leve` (avec motif).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionMasquageBody {
    pub decision: String,
    #[serde(default)]
    pub motif: Option<String>,
}

/// Masquage ajouté à la main.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AjoutMasquageBody {
    pub fournisseur: String,
    pub table: String,
    #[serde(default)]
    pub ligne_id: Option<Uuid>,
    #[serde(default)]
    pub colonne: Option<String>,
    pub motif: String,
}

/// Réglages RGPD de l'établissement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglageRgpdDto {
    pub plafond_depots_publics_jour: i32,
    pub extrait_disponible_jours: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglageRgpdBody {
    /// `plafond_depots_publics_jour` ou `extrait_disponible_jours`.
    pub reglage: String,
    pub valeur: i32,
}
