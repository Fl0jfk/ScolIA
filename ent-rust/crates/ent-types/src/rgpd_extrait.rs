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

// ─────────────── Remise, téléchargement, clôture, renvois (lot G3b-3) ───────────────

/// Défi de passkey récente : `options` est à jouer tel quel par le navigateur (`navigator.credentials.get`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DefiDto {
    pub defi_id: Uuid,
    pub options: serde_json::Value,
    pub intention: String,
    pub empreinte: String,
}

/// Remise : `mode` = `en_ligne` ou `main_propre` (émargement obligatoire dans ce cas).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RemettreBody {
    pub mode: String,
    pub defi_id: Uuid,
    pub credential: serde_json::Value,
    #[serde(default)]
    pub emargement_nom: Option<String>,
    #[serde(default)]
    pub emargement_piece_vue: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FichierRemisDto {
    pub id: Uuid,
    pub nom: String,
    pub type_fichier: String,
    pub octets: i32,
    pub empreinte: String,
    pub disponible_jusqu_le: DateTime<Utc>,
    pub purge: bool,
    pub telechargements: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemiseDto {
    pub id: Uuid,
    pub extrait_id: Uuid,
    pub mode: String,
    pub remis_le: DateTime<Utc>,
    pub empreinte_extrait: String,
    pub fichiers: Vec<FichierRemisDto>,
    /// Remise en main propre : le PDF à imprimer (base64), rendu une seule fois.
    #[serde(default)]
    pub pdf_base64: Option<String>,
}

/// « Mes données » : un extrait remis en ligne au compte connecté.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonExtraitDto {
    pub demande_reference: String,
    pub remise: RemiseDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TelechargerBody {
    pub defi_id: Uuid,
    pub credential: serde_json::Value,
}

/// Renvoi tracé : `rectification`, `retrait_consentement` ou `cada_tiers`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenvoiBody {
    pub nature: String,
    pub cible: String,
    pub texte: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenvoiDto {
    pub id: Uuid,
    pub nature: String,
    pub cible: String,
    pub texte: String,
    pub cree_le: DateTime<Utc>,
}

/// Ce que la clôture a consigné : éléments remis **par empreinte** et refus motivés.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClotureDto {
    pub elements: serde_json::Value,
    pub refus: serde_json::Value,
}

/// Pf7 : une pièce d'identité du coffre proposée au guichet (aucun nom, aucun contenu).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceChoisissableDto {
    pub id: Uuid,
    pub deposee_le: DateTime<Utc>,
    pub etat: String,
    pub pour_cette_demande: bool,
}

/// Fichier servi au demandeur authentifié : octets en base64url **sans remplissage**, jamais mis en cache.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FichierTelechargeDto {
    pub nom: String,
    pub type_fichier: String,
    pub contenu_base64url: String,
}
