//! Communication complète (socle 2, lot S2-8) : préférences de notification, campagnes,
//! rencontres parents-professeurs, formulaires et consentements. JSON des structures
//! d'`ent-db` (champs inconnus ignorés). Aucune logique métier.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Notifications
// ---------------------------------------------------------------------------

/// `GET/PUT /notifications/preferences`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferencesDto {
    pub email: bool,
    pub sms: bool,
    pub push: bool,
    /// `immediat` | `quotidien`.
    pub resume: String,
    /// Heures calmes, en minutes depuis minuit (heure de Paris).
    #[serde(default)]
    pub calme_debut: Option<u16>,
    #[serde(default)]
    pub calme_fin: Option<u16>,
}

/// `GET /notifications/envois` : envois de la personne (sans adresse ni numéro).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvoiDto {
    pub id: Uuid,
    pub canal: String,
    pub kind: String,
    pub titre: String,
    pub statut: String,
    #[serde(default)]
    pub motif: Option<String>,
    pub planifie_pour: DateTime<Utc>,
    #[serde(default)]
    pub traite_le: Option<DateTime<Utc>>,
}

/// `GET/PUT /notifications/plafonds`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlafondsDto {
    pub sms_par_jour: i32,
    pub sms_par_personne_jour: i32,
    pub sms_par_campagne: i32,
    pub campagnes_par_jour: i32,
}

// ---------------------------------------------------------------------------
// Campagnes
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampagneDto {
    pub id: Uuid,
    pub titre: String,
    pub corps: String,
    pub canaux: Vec<String>,
    /// `etablissement` | `niveau` | `classe` | `groupe`.
    pub audience: String,
    #[serde(default)]
    pub cible_ids: Vec<Uuid>,
    /// `brouillon` | `apercu` | `lancee` | `annulee`.
    pub statut: String,
    #[serde(default)]
    pub nb_destinataires: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Corps de `POST /campagnes`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NouvelleCampagneBody {
    pub titre: String,
    pub corps: String,
    pub canaux: Vec<String>,
    pub audience: String,
    #[serde(default)]
    pub cible_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestinataireApercuDto {
    pub nom: String,
    pub nb_eleves: i32,
    pub canaux: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EstimationSmsDto {
    pub destinataires: usize,
    pub segments_par_message: usize,
    pub credits_estimes: usize,
    pub plafond_campagne: i32,
    pub plafond_jour_restant: i64,
    pub depasse: bool,
    pub tronque: bool,
}

/// `POST /campagnes/{id}/apercu`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuCampagneDto {
    pub campagne_id: Uuid,
    pub nb_destinataires: usize,
    pub nb_eleves: usize,
    pub par_canal: BTreeMap<String, usize>,
    pub sans_canal_externe: usize,
    pub exclus_hors_droits: usize,
    #[serde(default)]
    pub sms: Option<EstimationSmsDto>,
    pub empreinte: String,
    pub destinataires: Vec<DestinataireApercuDto>,
    pub tronque: bool,
}

/// Corps de `POST /campagnes/{id}/lancer` : l'empreinte de l'aperçu confirmé.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationLancementBody {
    pub empreinte: String,
    pub nb_destinataires: usize,
    #[serde(default)]
    pub confirme_sms: bool,
}

/// `GET /campagnes/{id}/bilan`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneBilanDto {
    pub canal: String,
    pub statut: String,
    pub nombre: i64,
}

/// Cible possible d'une audience (classe, niveau…), pour les listes de choix.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnseignantClasseDto {
    pub user_id: Uuid,
    pub nom: String,
    pub matieres: String,
}

// ---------------------------------------------------------------------------
// Rencontres parents-professeurs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RencontreDto {
    pub id: Uuid,
    pub titre: String,
    pub classe_id: Uuid,
    pub classe_libelle: String,
    /// `brouillon` | `ouverte` | `close`.
    pub statut: String,
    pub nb_creneaux: i64,
    pub nb_reserves: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NouvelleRencontreBody {
    pub titre: String,
    pub classe_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NouveauxCreneauxBody {
    pub enseignant_user_id: Uuid,
    pub debut: DateTime<Utc>,
    pub duree_minutes: i32,
    pub nombre: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LignePlanningDto {
    pub creneau_id: Uuid,
    pub rencontre_id: Uuid,
    pub enseignant_user_id: Uuid,
    #[serde(default)]
    pub enseignant_nom: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// `libre` | `reserve`.
    pub statut: String,
    #[serde(default)]
    pub eleve_prenom: Option<String>,
    #[serde(default)]
    pub eleve_nom: Option<String>,
    #[serde(default)]
    pub responsable_nom: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RencontreFamilleDto {
    pub rencontre_id: Uuid,
    pub titre: String,
    pub classe_libelle: String,
    pub eleve_id: Uuid,
    pub eleve_prenom: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreneauVuDto {
    pub id: Uuid,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// `libre` | `reserve_par_moi` | `indisponible`.
    pub etat: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnseignantCreneauxDto {
    pub enseignant_user_id: Uuid,
    pub nom: String,
    pub matieres: String,
    pub creneaux: Vec<CreneauVuDto>,
}

// ---------------------------------------------------------------------------
// Formulaires et consentements
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaireDto {
    pub id: Uuid,
    /// `consentement` | `autorisation` | `sondage`.
    #[serde(rename = "type")]
    pub type_formulaire: String,
    #[serde(default)]
    pub objet: Option<String>,
    pub version: i32,
    pub titre: String,
    pub texte: String,
    /// `brouillon` | `publie` | `archive`.
    pub statut: String,
    pub audience: String,
    #[serde(default)]
    pub cible_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NouveauFormulaireBody {
    #[serde(rename = "type")]
    pub type_formulaire: String,
    #[serde(default)]
    pub objet: Option<String>,
    pub titre: String,
    pub texte: String,
    pub audience: String,
    #[serde(default)]
    pub cible_ids: Vec<Uuid>,
}

/// `GET /formulaires/mes` : un formulaire à traiter pour un enfant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaireFamilleDto {
    pub formulaire_id: Uuid,
    #[serde(rename = "type")]
    pub type_formulaire: String,
    #[serde(default)]
    pub objet: Option<String>,
    pub version: i32,
    pub titre: String,
    pub texte: String,
    pub eleve_id: Uuid,
    pub eleve_prenom: String,
    #[serde(default)]
    pub ma_decision: Option<String>,
    #[serde(default)]
    pub etat: Option<String>,
    pub a_repondre: bool,
}

/// Corps de `POST /formulaires/{id}/repondre`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReponseFormulaireBody {
    pub eleve_id: Uuid,
    /// `accepte` | `refuse` | `retire`.
    pub decision: String,
    #[serde(default)]
    pub commentaire: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatConsentementDto {
    pub eleve_id: Uuid,
    pub objet: String,
    /// `accepte` | `refuse` | `aucune`.
    pub decision: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentementEleveDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub decision: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatsFormulaireDto {
    pub cibles: usize,
    pub accepte: usize,
    pub refuse: usize,
    pub sans_reponse: usize,
}
