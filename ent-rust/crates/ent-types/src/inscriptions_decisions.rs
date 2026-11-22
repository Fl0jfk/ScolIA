//! Socle 3, lot I5 — types d'échange des décisions, avis, commissions, liste d'attente, propositions de place et
//! conditions de confirmation (`docs/socles/03_INSCRIPTIONS.md` §7.1, §8.7, §11.4, §11.5).
//!
//! Les états restent des chaînes (`"admis"`, `"designee"`…) comme dans le reste du module. Le **motif interne**,
//! les avis et le rang des autres candidats ne figurent que dans les types du personnel ; l'espace de la famille
//! n'a que [`DecisionFamilleDto`], qui ne contient rien d'un autre responsable ni d'un autre candidat.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Corps de `POST /inscriptions/dossiers/{id}/decisions` et `…/decisions/revoir` (INS-04, INS-33, INS-65).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionCompleteBody {
    /// `admis`, `admis_sous_reserve`, `liste_attente`, `refuse` ou `ajourne`.
    pub sens: String,
    /// Offre retenue (vœu) ; à défaut, le premier vœu du dossier.
    #[serde(default)]
    pub offre_id: Option<Uuid>,
    /// Condition de la réserve (obligatoire pour `admis_sous_reserve`).
    #[serde(default)]
    pub reserve: Option<String>,
    /// Motif **interne**, obligatoire (3 à 1000 caractères) : il ne quitte jamais le personnel.
    pub motif_interne: String,
    /// Motif communiqué à la famille (règle de la campagne : jamais, facultatif, toujours pour un refus).
    #[serde(default)]
    pub motif_communique: Option<String>,
    /// Admettre au-delà de la capacité : confirmé et tracé, jamais en silence.
    #[serde(default)]
    pub confirmer_depassement: bool,
    /// Offre alternative (INS-65) : la décision porte sur une autre offre que le vœu demandé.
    #[serde(default)]
    pub alternative: bool,
    /// Instant de publication à la famille. Absent : publication immédiate. Présent : la décision est prise mais
    /// invisible jusqu'à cet instant (INS-62).
    #[serde(default)]
    pub publier_le: Option<DateTime<Utc>>,
    /// Confirmation d'une publication programmée entre 20 h et 8 h (heure de l'établissement) ou un dimanche.
    #[serde(default)]
    pub confirmer_plage_calme: bool,
    /// Admettre directement **alors que la dernière place est déjà désignée** au premier de la liste d'attente :
    /// confirmation explicite, avec son motif (`motif_passer_devant`), tracée sur les deux dossiers (D-069 M5).
    #[serde(default)]
    pub confirmer_passer_devant: bool,
    #[serde(default)]
    pub motif_passer_devant: Option<String>,
}

/// Une décision (historique complet, personnel seulement).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionDto {
    pub id: Uuid,
    pub dossier_id: Uuid,
    pub sens: String,
    pub offre_id: Option<Uuid>,
    pub reserve: Option<String>,
    pub motif_interne: String,
    pub motif_communique: Option<String>,
    pub depassement_capacite: bool,
    pub alternative: bool,
    pub remplace_decision_id: Option<Uuid>,
    pub commission_id: Option<Uuid>,
    pub origine: String,
    pub decide_par: Option<Uuid>,
    pub decide_le: DateTime<Utc>,
    pub publication_prevue_le: Option<DateTime<Utc>>,
    pub publiee_le: Option<DateTime<Utc>>,
}

/// Corps de `POST /inscriptions/decisions/{id}/publier` et `…/commissions/{id}/publier`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublierBody {
    /// Absent : publier maintenant. Présent : programmer.
    #[serde(default)]
    pub publier_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub confirmer_plage_calme: bool,
}

/// Résultat d'une publication : combien de décisions, combien de familles prévenues.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationDto {
    pub publiees: i64,
    pub programmees: i64,
    pub familles_prevenues: i64,
}

/// Projet de courrier (jamais envoyé tel quel : la direction le relit, le publie par l'espace de la famille).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourrierDto {
    pub decision_id: Uuid,
    pub objet: String,
    pub corps: String,
}

// ───────────────────────────── Avis et déport ─────────────────────────────

/// Corps de `POST /inscriptions/dossiers/{id}/avis` (INS-31).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvisBody {
    /// `entretien`, `commission` ou `pedagogique`.
    pub contexte: String,
    /// `favorable`, `reserve`, `defavorable` ou `sans_avis`.
    pub sens: String,
    /// 3 à 2000 caractères. Interne, mais à rédiger comme si la famille pouvait le lire (droit d'accès).
    pub texte: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvisDto {
    pub id: Uuid,
    pub dossier_id: Uuid,
    pub auteur_id: Uuid,
    pub contexte: String,
    pub sens: String,
    pub texte: String,
    pub cree_le: DateTime<Utc>,
    /// L'auteur s'est déporté de ce dossier depuis : l'avis est **marqué** (D-069 règle 2), la direction le lit comme tel.
    #[serde(default)]
    pub auteur_deporte: bool,
}

/// Corps de `POST /inscriptions/dossiers/{id}/deport` (INS-63).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeportBody {
    pub motif: String,
}

/// Réponse de `GET /inscriptions/dossiers/{id}/deport` : le déport est proposé, jamais imposé.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeportEtatDto {
    /// Le lecteur s'est déjà déporté de ce dossier.
    pub deporte: bool,
    /// Le lecteur partage une adresse avec un responsable ou le déposant du dossier : déport suggéré.
    pub conflit_possible: bool,
}

// ───────────────────────────── Commissions ─────────────────────────────

/// Corps de `POST /inscriptions/commissions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissionBody {
    pub campagne_id: Uuid,
    pub intitule: String,
    pub seance_le: DateTime<Utc>,
    #[serde(default)]
    pub membres: Vec<Uuid>,
    #[serde(default)]
    pub dossier_ids: Vec<Uuid>,
}

/// Une place restante en direct (INS-32) : la capacité se lit à chaque appel, jamais à la création.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacesOffreDto {
    pub offre_id: Uuid,
    pub libelle: String,
    pub capacite: Option<i64>,
    pub disponibles: Option<i64>,
    pub liste_attente: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissionDossierDto {
    pub dossier_id: Uuid,
    pub reference: String,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub niveau_demande: Option<String>,
    pub etat: String,
    /// Dernier sens décidé, le cas échéant.
    pub derniere_decision: Option<String>,
    pub examine_le: Option<DateTime<Utc>>,
    /// Nombre d'avis internes (le texte ne se lit que par la direction, dossier par dossier).
    pub nb_avis: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissionDto {
    pub id: Uuid,
    pub campagne_id: Uuid,
    pub intitule: String,
    pub seance_le: DateTime<Utc>,
    pub statut: String,
    pub membres: Vec<Uuid>,
    pub dossiers: Vec<CommissionDossierDto>,
    pub places: Vec<PlacesOffreDto>,
    pub pv_texte: Option<String>,
}

/// Corps de `POST /inscriptions/commissions/{id}/dossiers` (ajout ou retrait).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissionDossiersBody {
    #[serde(default)]
    pub ajouter: Vec<Uuid>,
    #[serde(default)]
    pub retirer: Vec<Uuid>,
}

/// Une décision d'un lot de commission.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionEnLotItem {
    pub dossier_id: Uuid,
    #[serde(flatten)]
    pub decision: DecisionCompleteBody,
}

/// Corps de `POST /inscriptions/commissions/{id}/decisions` : chaque dossier est traité dans sa propre
/// transaction ; un échec n'annule pas les autres et figure au récapitulatif.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionsEnLotBody {
    pub decisions: Vec<DecisionEnLotItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionEnLotResultat {
    pub dossier_id: Uuid,
    pub ok: bool,
    pub decision_id: Option<Uuid>,
    pub erreur: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecapitulatifLotDto {
    pub resultats: Vec<DecisionEnLotResultat>,
    pub reussies: i64,
    pub refusees: i64,
}

/// Corps de `POST /inscriptions/commissions/{id}/cloturer`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissionClotureBody {
    /// Procès-verbal (texte ; le document signé relève du socle 7).
    #[serde(default)]
    pub pv_texte: Option<String>,
}

// ───────────────────────────── Liste d'attente et propositions ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttenteLigneDto {
    pub dossier_id: Uuid,
    pub reference: String,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    /// Position calculée (1, 2, 3…) : sans trou, même après un départ.
    pub position: i64,
    /// Proposition désignée (rien n'est parti) ou envoyée.
    pub proposition_id: Option<Uuid>,
    pub proposition_statut: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListeAttenteDto {
    pub offre_id: Uuid,
    pub libelle: String,
    pub capacite: Option<i64>,
    pub disponibles: Option<i64>,
    pub lignes: Vec<AttenteLigneDto>,
}

/// Une proposition de place (personnel).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionDto {
    pub id: Uuid,
    pub dossier_id: Uuid,
    pub reference: String,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub offre_id: Uuid,
    pub origine: String,
    pub statut: String,
    pub rang_au_moment: Option<i32>,
    pub designee_le: DateTime<Utc>,
    pub envoyee_le: Option<DateTime<Utc>>,
    pub repondre_avant: Option<NaiveDate>,
    pub repondu_le: Option<DateTime<Utc>>,
    /// Délai dépassé sans réponse : l'expiration se constate (jamais automatique).
    pub expiree_a_constater: bool,
    pub motif: Option<String>,
}

/// Corps de `POST /inscriptions/propositions/{id}/envoyer` : une personne confirme l'envoi.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvoyerPropositionBody {
    /// Doit être vrai : l'envoi est un acte humain explicite.
    #[serde(default)]
    pub confirmer: bool,
    /// Place disponible = 0 : admettre quand même, confirmé et tracé.
    #[serde(default)]
    pub confirmer_depassement: bool,
    /// Motif interne de la décision d'admission (facultatif : un motif par défaut décrit la liste d'attente).
    #[serde(default)]
    pub motif_interne: Option<String>,
    #[serde(default)]
    pub motif_communique: Option<String>,
}

/// Corps de `POST /inscriptions/propositions/{id}/ecarter` : passer son tour, motif obligatoire.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotifBody {
    pub motif: String,
}

// ───────────────────────────── Conditions de confirmation ─────────────────────────────

/// Corps de `PUT /inscriptions/campagnes/{id}/conditions`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionsCampagneBody {
    /// Parmi `reponse_famille`, `contrat_signe`, `frais_payes`, `acompte_paye`,
    /// `situation_financiere_examinee`, `passage`.
    pub codes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionDto {
    pub code: String,
    pub libelle: String,
    pub remplie: bool,
    /// `famille`, `personnel`, `evenement`, `levee`, ou `calculee` (accords, pièces bloquantes).
    pub source: Option<String>,
    pub remplie_le: Option<DateTime<Utc>>,
    /// Motif d'une levée (personnel seulement).
    pub motif: Option<String>,
    /// Pourquoi une condition calculée n'est pas remplie (sans nommer qui, ni quel responsable).
    pub detail: Option<String>,
}

/// Corps de `POST /inscriptions/dossiers/{id}/conditions/{code}/remplir`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemplirConditionBody {
    #[serde(default)]
    pub motif: Option<String>,
    /// Événement ou objet qui l'a remplie (paiement, signature…).
    #[serde(default)]
    pub source_ref: Option<Uuid>,
}

/// État des conditions d'un dossier : ce qui reste, et si le dossier peut être confirmé.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionsDossierDto {
    pub conditions: Vec<ConditionDto>,
    pub toutes_remplies: bool,
    /// Le dossier est passé (ou resté) `confirme` à cette évaluation.
    pub confirme: bool,
}

// ───────────────────────────── Famille ─────────────────────────────

/// Ce que la famille lit de la décision, **une fois publiée** : jamais le motif interne, jamais un avis, jamais
/// la position des autres candidats, jamais une information sur un autre responsable.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionFamilleDto {
    /// Une décision est publiée.
    pub publiee: bool,
    pub sens: Option<String>,
    pub offre: Option<String>,
    pub motif_communique: Option<String>,
    pub reserve: Option<String>,
    pub publiee_le: Option<DateTime<Utc>>,
    /// Position du seul dossier de la famille sur la liste d'attente.
    pub position_attente: Option<i64>,
    /// Proposition de place à laquelle répondre.
    pub proposition: Option<PropositionFamilleDto>,
    pub conditions: Vec<ConditionDto>,
    /// `repondre`, `deposer_piece`…
    pub actions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionFamilleDto {
    pub id: Uuid,
    pub statut: String,
    pub repondre_avant: Option<NaiveDate>,
}

/// Corps de `POST /preinscription/dossiers/{id}/proposition/repondre`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReponsePropositionBody {
    /// `acceptee` ou `declinee`.
    pub reponse: String,
}
