//! Dossier élève et personnes (socle 2, lot S2-1) : fiche 360°, responsables,
//! contacts d'urgence, payeurs, chronologie, fusion de personnes.
//! Routes `ent-api/src/dossier.rs` (préfixe `/dossier`). Aucune logique métier.
//!
//! Les états restent des chaînes : garde (`"exclusive"`, `"alternee"`,
//! `"aucune"`), statut de contact (`"valide"`, `"propose"`), domaine de
//! chronologie (`"dossier"`, `"scolarite"`, `"vie_scolaire"`, `"notes"`…).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Personne (responsable, contact, élève majeur), distincte du compte.
/// Coordonnées absentes quand le lecteur n'a pas à les connaître.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonneDto {
    pub id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub civilite: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub telephone_2: Option<String>,
    pub adresse: Option<String>,
    /// La personne a un compte de connexion (identifiant non exposé).
    pub a_compte: bool,
    /// Fiche représentant l'élève majeur lui-même.
    pub represente_eleve: bool,
    pub fusionnee_dans: Option<Uuid>,
}

/// Corps de `POST /dossier/personnes` et `PUT /dossier/personnes/{id}`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonneBody {
    pub nom: String,
    #[serde(default)]
    pub prenom: Option<String>,
    #[serde(default)]
    pub civilite: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub telephone: Option<String>,
    #[serde(default)]
    pub telephone_2: Option<String>,
    #[serde(default)]
    pub adresse: Option<String>,
}

/// Lien de responsabilité personne ↔ élève.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponsableDto {
    pub lien_id: Uuid,
    pub eleve_id: Uuid,
    pub personne: PersonneDto,
    pub foyer_id: Option<Uuid>,
    pub foyer_libelle: Option<String>,
    pub lien: String,
    pub autorite_parentale: bool,
    pub garde: Option<String>,
    pub date_debut: Option<NaiveDate>,
    pub date_fin: Option<NaiveDate>,
    pub suspendu_le: Option<DateTime<Utc>>,
    /// Réservé au personnel habilité (jamais envoyé aux familles).
    pub suspension_motif: Option<String>,
    pub suspension_reference: Option<String>,
    /// Ouvre des accès aujourd'hui (ni suspendu, ni hors de ses dates).
    pub actif: bool,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

/// Corps de `POST /dossier/eleves/{id}/responsables` (avec `personne_id`)
/// et de `PUT /dossier/liens/{id}` (`personne_id` ignoré).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LienBody {
    #[serde(default)]
    pub personne_id: Option<Uuid>,
    #[serde(default)]
    pub foyer_id: Option<Uuid>,
    pub lien: String,
    #[serde(default)]
    pub autorite_parentale: bool,
    #[serde(default)]
    pub garde: Option<String>,
    #[serde(default)]
    pub date_debut: Option<NaiveDate>,
    #[serde(default)]
    pub date_fin: Option<NaiveDate>,
    #[serde(default)]
    pub peut_voir_notes: bool,
    #[serde(default)]
    pub peut_voir_vs: bool,
    #[serde(default)]
    pub peut_justifier_absence: bool,
    #[serde(default)]
    pub peut_autoriser_sorties: bool,
}

/// Corps de `POST /dossier/eleves/{id}/responsables/ajout` : commande unique « ajouter un
/// responsable » (socle 4, lot A0). Soit `nouvelle_personne` (la personne est créée avec son lien
/// dans une seule transaction), soit `lien.personne_id` (personne existante) ; jamais les deux.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AjoutResponsableBody {
    #[serde(default)]
    pub nouvelle_personne: Option<PersonneBody>,
    pub lien: LienBody,
}

/// Réponse `202 Accepted` quand la personne désignée **a un compte** : le lien est en attente de
/// sa confirmation, il n'ouvre aucun accès.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LienEnAttenteDto {
    pub en_attente: bool,
    pub attente_id: Uuid,
}

/// Lien en attente vu par la personne concernée (minimisé : prénom et initiale de l'élève).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonLienEnAttenteDto {
    pub id: Uuid,
    pub eleve_prenom: String,
    pub eleve_initiale: String,
    pub lien: String,
    pub demande_le: DateTime<Utc>,
}

/// Corps de `POST /dossier/mes-liens-en-attente/{id}/reponse`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReponseLienBody {
    pub accepter: bool,
}

/// Lien en attente d'un élève, vu du personnel : de quoi le confirmer au guichet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LienEnAttenteEleveDto {
    pub attente_id: Uuid,
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub a_un_compte: bool,
    pub lien: String,
    /// `saisie` ou `inscription`.
    pub source: String,
    pub demande_le: DateTime<Utc>,
}

/// Corps de `POST /dossier/liens-en-attente/{id}/confirmer-guichet` : la pièce d'identité vue
/// au guichet. Seul le type est enregistré (jamais la copie ni le numéro).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationGuichetBody {
    /// `carte_identite`, `passeport`, `titre_sejour`, `permis_conduire`, `livret_famille`, `autre`.
    pub piece: String,
}

/// Réponse de la confirmation au guichet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationGuichetDto {
    /// Lien réel créé (`None` si la personne était déjà liée à l'élève).
    pub lien_id: Option<Uuid>,
}

/// Corps de `POST /dossier/personnes/{id}/ne-pas-solliciter` (direction).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NePasSolliciterBody {
    pub actif: bool,
    /// Référence de la pièce de protection (scellée en base, jamais renvoyée).
    #[serde(default)]
    pub reference: Option<String>,
}

/// Adresse à protéger mise de côté, vue de la direction (sans l'adresse).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdresseParqueeDto {
    pub id: Uuid,
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub origine: String,
    pub created_at: DateTime<Utc>,
}

/// « Ne pas solliciter » déclaré par une famille, à confirmer par la direction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectionADeclarerDto {
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub declaree_le: Option<DateTime<Utc>>,
}

/// `GET /dossier/alertes-protection` : alertes d'information de la direction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertesProtectionDto {
    pub adresses_a_traiter: Vec<AdresseParqueeDto>,
    pub ne_pas_solliciter_a_confirmer: Vec<ProtectionADeclarerDto>,
}

/// Réponse de `POST /dossier/adresses-protegees/{id}/ouvrir` (consultation journalisée).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdresseProtegeeDto {
    pub adresse: String,
}

/// Corps de `POST /dossier/liens/{id}/suspendre`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuspensionBody {
    pub motif: String,
    #[serde(default)]
    pub reference: Option<String>,
}

/// Une part de paiement (pourcentage entier).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayeurPartDto {
    pub lien_id: Uuid,
    pub part: i16,
}

/// Corps de `PUT /dossier/eleves/{id}/payeurs` : liste complète (vide = aucun payeur).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayeursBody {
    pub payeurs: Vec<PayeurPartDto>,
}

/// Contact d'urgence / personne autorisée à récupérer l'élève.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactUrgenceDto {
    pub id: Uuid,
    pub personne: PersonneDto,
    pub relation: String,
    pub ordre: i16,
    pub peut_recuperer: bool,
    pub statut: String,
    pub commentaire: Option<String>,
}

/// Corps de `POST /dossier/eleves/{id}/contacts` : `personne_id` OU `nouvelle_personne`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactBody {
    #[serde(default)]
    pub personne_id: Option<Uuid>,
    #[serde(default)]
    pub nouvelle_personne: Option<PersonneBody>,
    pub relation: String,
    #[serde(default = "ordre_defaut")]
    pub ordre: i16,
    #[serde(default)]
    pub peut_recuperer: bool,
    #[serde(default)]
    pub commentaire: Option<String>,
}

fn ordre_defaut() -> i16 {
    1
}

/// Action métier proposée depuis la fiche (code du catalogue `ent_auth::Action`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionFicheDto {
    pub code: String,
    pub libelle: String,
}

/// Sections du dossier visibles par le lecteur (calculées par le serveur).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionsDto {
    pub identite: bool,
    pub scolarite: bool,
    pub famille: bool,
    pub options: bool,
    pub notes: bool,
    pub vs: bool,
    pub documents: bool,
    pub dispositifs: bool,
    pub repas: bool,
    pub bourses: bool,
    pub continuum: bool,
}

/// Nombre d'éléments visibles dans chaque section existante.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeSectionsDto {
    pub notes: u32,
    pub sanctions: u32,
    pub incidents: u32,
    pub carnet: u32,
    pub documents: u32,
    pub options: u32,
    pub dispositifs: u32,
    pub bourses: u32,
}

/// `GET /dossier/eleves/{id}/fiche` : fiche élève 360°.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FicheEleveDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub regime: Option<String>,
    pub statut: String,
    pub ine: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub age_ans: Option<u32>,
    pub majeur: bool,
    pub date_entree: Option<NaiveDate>,
    pub date_sortie: Option<NaiveDate>,
    pub sections: SectionsDto,
    pub resume: ResumeSectionsDto,
    pub responsables: Vec<ResponsableDto>,
    pub contacts: Vec<ContactUrgenceDto>,
    pub actions: Vec<ActionFicheDto>,
    pub alertes: Vec<String>,
    /// Groupes de champs masqués pour ce lecteur, avec la raison (E1).
    #[serde(default)]
    pub masques: Vec<MasqueChampsDto>,
}

/// Un groupe de champs masqué et pourquoi (jamais de masquage muet).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasqueChampsDto {
    pub groupe: String,
    /// `droit_manquant`, `hors_portee` ou `reserve_au_personnel`.
    pub raison: String,
    /// Action du catalogue qui ouvrirait le groupe (absente pour une réserve absolue).
    #[serde(default)]
    pub action: Option<String>,
    pub message: String,
}

/// `GET /dossier/eleves/{id}/chronologie` : une entrée (titre neutre).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntreeChronologieDto {
    pub quand: DateTime<Utc>,
    /// `"journal"` ou `"evenement"`.
    pub source: String,
    pub domaine: String,
    pub titre: String,
}

/// `GET /dossier/personnes?q=` : personne trouvée.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonneTrouveeDto {
    pub personne: PersonneDto,
    pub nb_liens: i64,
}

/// Corps de `PUT /dossier/personnes/{id}/compte`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompteBody {
    pub user_id: Uuid,
}

/// Corps de `POST /dossier/fusions`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FusionBody {
    pub source_id: Uuid,
    pub cible_id: Uuid,
    pub motif: String,
}

/// Fusion de personnes (annulable 30 jours).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FusionDto {
    pub id: Uuid,
    pub source_id: Uuid,
    pub cible_id: Uuid,
    pub motif: String,
    pub fusionne_le: DateTime<Utc>,
    pub annule_le: Option<DateTime<Utc>>,
}
