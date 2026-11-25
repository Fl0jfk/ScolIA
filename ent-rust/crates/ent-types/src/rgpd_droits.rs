//! Socle 27, lot G3a : **guichet des demandes de droits**. Contrat JSON des routes `/rgpd/demandes/*`
//! d'`ent-api` (`27_RGPD_ARCHIVES.md` §9, §12.1, §14.3).
//!
//! Deux publics, deux contrats :
//! * le **guichet** (direction, secrétariat habilité) lit la file ([`DemandeLigneDto`], métadonnées
//!   seulement, R-RG32) et la fiche ([`DemandeDto`], journal compris) ;
//! * le **demandeur** lit son seul suivi ([`SuiviDemandeDto`]) : états lisibles, aucune information
//!   interne (gestionnaire, notes, journal).
//!
//! Le dépôt **sans compte** ne rend aucun identifiant : seule une phrase invariable, qui ne dit jamais si
//! l'établissement détient des données sur la personne (R-RG42).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─────────────────────────────── Entrées ───────────────────────────────

/// Dépôt par un demandeur connecté (RGP-140).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotBody {
    /// `soi_meme`, `eleve_mineur`, `titulaire_autorite`, `representant_majeur_protege`, `mandataire`,
    /// `ancien_eleve`, `personnel`. Les qualités qui exigent une pièce (`tiers`, `parent_sans_autorite`)
    /// passent par le guichet.
    pub qualite: String,
    /// `eleve`, `personne` ou `agent`.
    pub sujet_type: String,
    pub sujet_id: Option<Uuid>,
    pub droits: Vec<String>,
    #[serde(default)]
    pub perimetre_texte: Option<String>,
    #[serde(default)]
    pub perimetre_du: Option<NaiveDate>,
    #[serde(default)]
    pub perimetre_au: Option<NaiveDate>,
}

/// Dépôt **sans compte** (RGP-142) : route publique, débit limité.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotPublicBody {
    pub nom: String,
    /// Moyen de contact donné par le déposant. Il n'est **jamais** utilisé pour répondre (R-RG42) : il aide
    /// le guichet à rapprocher la demande d'un dossier.
    pub contact: String,
    /// `soi_meme`, `ancien_eleve` ou `titulaire_autorite`.
    pub qualite: String,
    /// Nom de la personne concernée, tel que le déposant l'écrit.
    pub personne_concernee: String,
    pub droits: Vec<String>,
    #[serde(default)]
    pub perimetre_texte: Option<String>,
}

/// Réponse **invariable** d'un dépôt sans compte : aucune confirmation, aucun identifiant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotPublicDto {
    pub message: String,
}

/// Saisie d'une demande reçue par courrier, courriel, au guichet, ou transmise par le DPO ou la CNIL
/// (RGP-143).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaisieBody {
    /// `courrier`, `courriel`, `guichet`, `dpo` ou `cnil`.
    pub canal: String,
    /// Date de **réception** (date du courrier) : elle fait courir le délai légal.
    pub recue_le: NaiveDate,
    #[serde(default)]
    pub demandeur_compte_id: Option<Uuid>,
    #[serde(default)]
    pub demandeur_personne_id: Option<Uuid>,
    #[serde(default)]
    pub demandeur_nom: Option<String>,
    #[serde(default)]
    pub demandeur_contact: Option<String>,
    pub qualite: String,
    /// `eleve`, `personne`, `agent` ou `inconnu`.
    pub sujet_type: String,
    #[serde(default)]
    pub sujet_id: Option<Uuid>,
    #[serde(default)]
    pub personne_concernee: Option<String>,
    pub droits: Vec<String>,
    #[serde(default)]
    pub perimetre_texte: Option<String>,
    #[serde(default)]
    pub perimetre_du: Option<NaiveDate>,
    #[serde(default)]
    pub perimetre_au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationBody {
    /// `rgpd`, `cada` ou `document`.
    pub qualification: String,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentiteBody {
    /// `session_forte`, `code_courriel`, `piece` ou `mandat`.
    pub mode: String,
    pub reussie: bool,
    #[serde(default)]
    pub note: Option<String>,
    /// Référence du document du coffre consulté. Il est **détruit à la clôture** (R-RG16).
    #[serde(default)]
    pub piece_document_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrecisionIdentiteBody {
    /// Ce qui est demandé au demandeur, en une phrase.
    pub demande: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectationBody {
    /// `None` retire l'affectation.
    #[serde(default)]
    pub gestionnaire_personne_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProlongationBody {
    pub jusqu_au: NaiveDate,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroitBody {
    pub droit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionBody {
    pub droit: String,
    /// `accorde`, `partiel`, `refuse` ou `sans_objet`.
    pub decision: String,
    #[serde(default)]
    pub motif_code: Option<String>,
    #[serde(default)]
    pub motif_texte: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitulairesBody {
    pub informer: bool,
    #[serde(default)]
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClotureBody {
    pub reponse_resume: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotifBody {
    pub motif: String,
}

// ─────────────────────────────── Sorties ───────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroitDecisionDto {
    pub droit: String,
    pub decision: String,
    pub motif_code: Option<String>,
    pub motif_texte: Option<String>,
    pub decide_le: Option<DateTime<Utc>>,
}

/// Une vérification d'identité. **Sans la pièce** : seule la mention « vérifiée le, par » subsiste après la
/// clôture (R-RG16).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentiteDto {
    pub mode: String,
    pub resultat: String,
    pub qualite_constatee: Option<String>,
    pub qualite_note: Option<String>,
    pub verifie_le: Option<DateTime<Utc>>,
    pub piece_detenue: bool,
    pub piece_detruite_le: Option<DateTime<Utc>>,
}

/// Une transition. Le détail ne porte **jamais** de donnée d'élève (E14).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalDto {
    pub evenement: String,
    pub detail: Option<String>,
    pub par_role: Option<String>,
    pub le: DateTime<Utc>,
}

/// Fiche d'une demande, pour le guichet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemandeDto {
    pub id: Uuid,
    pub reference: String,
    pub canal: String,
    pub recue_le: NaiveDate,
    pub saisie_le: DateTime<Utc>,
    pub saisie_tardive: bool,
    pub echeance_le: NaiveDate,
    pub prolongee_jusqu_au: Option<NaiveDate>,
    pub prolongation_motif: Option<String>,
    pub echeance_effective: NaiveDate,
    pub jours_restants: i64,
    pub en_retard: bool,
    pub qualite: String,
    pub demandeur_nom: Option<String>,
    pub demandeur_a_un_compte: bool,
    pub sujet_type: String,
    pub sujet_id: Option<Uuid>,
    pub personne_concernee: Option<String>,
    pub qualification: String,
    pub qualification_motif: Option<String>,
    pub perimetre_texte: Option<String>,
    pub perimetre_du: Option<NaiveDate>,
    pub perimetre_au: Option<NaiveDate>,
    pub etat: String,
    pub gestionnaire_personne_id: Option<Uuid>,
    pub identite_demandee_le: Option<NaiveDate>,
    pub identite_fournie_le: Option<NaiveDate>,
    /// Jours d'attente d'une précision d'identité. **Comptés, non déduits** de l'échéance (§9.2).
    pub identite_attente_jours: i64,
    pub reouvertures: i32,
    pub close_le: Option<DateTime<Utc>>,
    pub reponse_resume: Option<String>,
    pub refus_motif: Option<String>,
    pub titulaires_informes_le: Option<DateTime<Utc>>,
    pub titulaires_non_informes_motif: Option<String>,
    pub droits: Vec<DroitDecisionDto>,
    pub identites: Vec<IdentiteDto>,
    pub journal: Vec<JournalDto>,
    pub piece_a_detruire: bool,
}

/// Ligne de la file. **Métadonnées seulement** : aucun nom, aucun texte de périmètre (R-RG32).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemandeLigneDto {
    pub id: Uuid,
    pub reference: String,
    pub canal: String,
    pub recue_le: NaiveDate,
    pub echeance_effective: NaiveDate,
    pub jours_restants: i64,
    pub en_retard: bool,
    pub etat: String,
    pub qualification: String,
    pub qualite: String,
    pub sujet_type: String,
    pub gestionnaire_personne_id: Option<Uuid>,
    pub droits: Vec<String>,
    pub saisie_tardive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableauDemandesDto {
    /// Jour de l'établissement qui sert de référence aux échéances.
    pub jour: NaiveDate,
    pub lignes: Vec<DemandeLigneDto>,
    pub ouvertes: usize,
    pub en_retard: usize,
    pub echeance_proche: usize,
    pub identite_a_verifier: usize,
    pub a_qualifier: usize,
    pub closes: usize,
}

/// Suivi côté demandeur (RGP-155).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiviDemandeDto {
    pub id: Uuid,
    pub reference: String,
    pub recue_le: NaiveDate,
    pub echeance_effective: NaiveDate,
    pub etat: String,
    /// Phrase prête à afficher.
    pub avancement: String,
    pub droits: Vec<String>,
    pub prolongee_jusqu_au: Option<NaiveDate>,
    pub prolongation_motif: Option<String>,
    pub close_le: Option<DateTime<Utc>>,
    pub reponse_resume: Option<String>,
}

/// Une étape de la grille de vérification d'identité (RGP-307).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtapeVerificationDto {
    pub code: String,
    pub libelle: String,
    pub geste: String,
    pub obligatoire: bool,
}

/// Un modèle de réponse éditable (RGP-304).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeleReponseDto {
    pub code: String,
    pub libelle: String,
    pub droit: Option<String>,
    pub motif_code: String,
    pub texte: String,
}

/// Référentiel du guichet : vocabulaire, grille d'identité, modèles de réponse.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferentielGuichetDto {
    pub droits: Vec<String>,
    pub qualites: Vec<String>,
    pub canaux_guichet: Vec<String>,
    pub grille_identite: Vec<EtapeVerificationDto>,
    pub modeles_reponse: Vec<ModeleReponseDto>,
    /// Jours avant échéance auxquels les personnes habilitées sont alertées (S-544).
    pub alerte_echeance_jours: Vec<i64>,
    /// La vérification d'identité suspend-elle le délai légal ? Faux par défaut (§9.2).
    pub identite_suspend_delai: bool,
}
