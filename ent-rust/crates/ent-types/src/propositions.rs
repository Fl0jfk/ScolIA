//! Propositions de modification de la fiche de l'élève et paramétrage des champs (socle 2,
//! lot E5). Routes `ent-api` sous `/eleves/propositions…`, `/eleves/{id}/propositions…` et
//! `/eleves/parametrage…`. Aucune logique métier.
//!
//! Les valeurs proposées sont des valeurs JSON : texte (nom, lieu, identifiant), date
//! `AAAA-MM-JJ`, liste de prénoms, `M`/`F`, ou `null` pour retirer un champ facultatif.
//! La valeur actuelle de la fiche n'est jamais montrée à la famille.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// `POST /eleves/{id}/propositions` (ELV-18 famille et élève majeur, ELV-40 personnel).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeposerPropositionBody {
    /// Champ du catalogue → valeur proposée (`GET /eleves/{id}/propositions/champs`).
    pub champs: BTreeMap<String, Value>,
    /// Pièce justificative : document de cet élève (exigée pour l'état civil).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piece_document_id: Option<Uuid>,
    /// Précision libre, 500 caractères au plus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commentaire: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChampPropositionDto {
    pub champ: String,
    /// Valeur de la fiche au dépôt (absente pour la famille).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuel: Option<Value>,
    pub propose: Value,
    /// `en_attente`, `acceptee`, `refusee` ou `obsolete`.
    pub decision: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropositionDto {
    pub id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub cible: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origine: Option<String>,
    /// Compte déposant (secrétariat seulement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depose_par: Option<Uuid>,
    pub piece_document_id: Option<Uuid>,
    pub commentaire: Option<String>,
    /// `en_attente`, `acceptee`, `acceptee_partiellement`, `refusee`, `expiree`,
    /// `appliquee_auto`, `obsolete`.
    pub statut: String,
    /// Règle automatique qui l'a appliquée, le cas échéant (secrétariat seulement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regle_auto: Option<String>,
    /// Motif codé de la décision : visible du déposant.
    pub decision_code: Option<String>,
    pub decision_motif: Option<String>,
    pub decide_le: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
    pub expire_le: Option<DateTime<Utc>>,
    pub champs: Vec<ChampPropositionDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RaisonNonAutoDto {
    pub champ: String,
    /// `aucune_regle`, `champ_sensible`, `champ_verrouille`, `source_ne_fait_pas_foi`,
    /// `conflit_avec_la_fiche`, `application_impossible`…
    pub raison: String,
}

/// Réponse d'un dépôt.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ResultatDepotDto {
    pub en_attente: Option<PropositionDto>,
    pub appliquee_auto: Option<PropositionDto>,
    /// Champs identiques à la fiche, écartés.
    pub champs_identiques: Vec<String>,
    pub raisons_non_auto: Vec<RaisonNonAutoDto>,
}

/// `POST /eleves/propositions/{id}/decision` (ELV-19) : les champs acceptés ; tout le reste
/// est refusé.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionPropositionBody {
    #[serde(default)]
    pub champs_acceptes: Vec<String>,
    /// `correction` ou `changement_etat_civil` (propositions d'une personne).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nature: Option<String>,
    /// Motif codé de la version (voir les motifs de correction et de changement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piece_document_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effet_du: Option<NaiveDate>,
    /// Obligatoire si un champ est refusé : `a_corriger_dans_la_source`,
    /// `piece_insuffisante`, `valeur_incorrecte`, `deja_traite`, `hors_perimetre`, `autre`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refus_code: Option<String>,
    /// Visible du déposant ; obligatoire pour `autre`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refus_motif: Option<String>,
}

/// Un champ que la famille peut signaler.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampSignalableDto {
    pub code: String,
    pub libelle: String,
    pub piece_requise: bool,
    pub sensible: bool,
}

// ───────────────────────── Paramétrage ─────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampParametreDto {
    pub code: String,
    pub libelle: String,
    pub visible: bool,
    /// `creation`, `inscription`, `export_bee`, `export_gar`.
    pub obligatoire_a: Vec<String>,
    /// `non`, `proposer`, `modifier`.
    pub saisie_famille: String,
    /// `scolia`, `siecle`, `onde`, `reprise`, `famille`.
    pub source_qui_fait_foi: String,
    /// `libre`, `par_proposition`, `interdite`.
    pub modification_locale: String,
    pub verrouille: bool,
    /// Une ligne existe pour ce champ (sinon : valeur par défaut du catalogue).
    pub regle: bool,
    /// Sources pour lesquelles une règle d'application automatique est active.
    pub regles_auto: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfilPropose {
    pub code: String,
    pub libelle: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametrageEleveDto {
    /// Profil appliqué (`personnalise` après un réglage manuel), absent si jamais appliqué.
    pub profil: Option<String>,
    pub profils_proposes: Vec<ProfilPropose>,
    pub champs: Vec<ChampParametreDto>,
}

/// `PUT /eleves/parametrage/champs/{champ}` : modification partielle.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierChampBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obligatoire_a: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saisie_famille: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_qui_fait_foi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modification_locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verrouille: Option<bool>,
}

/// `POST /eleves/parametrage/profil`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliquerProfilBody {
    /// `public`, `prive_sous_contrat`, `prive_sous_contrat_logiciel_agree`,
    /// `hors_contrat`, `premier_degre`.
    pub profil: String,
}

/// `PUT /eleves/parametrage/regles-auto`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegleAutoBody {
    pub champ: String,
    /// `import`, `educonnect` ou `systeme`.
    pub source: String,
    pub actif: bool,
}
