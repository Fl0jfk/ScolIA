//! Absences des personnels (socle 15, lots A0–A1) : types JSON des routes
//! `ent-api/src/absences_personnels.rs` (préfixe `/personnels/absences`).
//! Conception : `docs/socles/15_ABSENCES_PERSONNELS.md`. Aucune logique métier.
//!
//! Deux niveaux de lecture, **deux types distincts** (R-AP3, §23.4 point 3) :
//! * [`AbsenceOrganisationDto`] : ce que voient ceux qui organisent la journée. Il n'a **aucun champ de
//!   nature, de traitement ni de décision** : la projection « organisation » est une structure
//!   différente, pas un filtre d'affichage ; un test parcourt le JSON pour le vérifier ;
//! * [`AbsenceDto`] : le cercle RH (l'agent lui-même, direction, secrétariat, circuit), avec la nature.
//!
//! Les codes (état, traitement, nature, employeur…) restent des chaînes : la liste fermée vit dans
//! `ent-db` et dans la base.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Une plage d'une absence discontinue (ABP-115).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlageBody {
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
}

/// `POST /personnels/absences` (ABP-100, 101, 106 à 109, 114 à 117). Sans `agent_id` : l'agent connecté.
/// Les dates sont **inclusives** et en jour de l'établissement ; sans heures : journée entière.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeclarerAbsenceBody {
    #[serde(default)]
    pub agent_id: Option<Uuid>,
    /// Code d'une nature active et permise pour l'engagement ; absent = « je préciserai » (`inconnue`).
    #[serde(default)]
    pub nature_code: Option<String>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
    /// Absence discontinue : si présent, remplace `debut` / `fin` (qui deviennent l'enveloppe).
    #[serde(default)]
    pub plages: Vec<PlageBody>,
    /// 200 caractères, natures non sensibles seulement (R-AP1) ; refusé sinon.
    #[serde(default)]
    pub texte_libre: Option<String>,
    /// « Je prolongerai » : la fin est une estimation.
    #[serde(default)]
    pub fin_provisoire: bool,
    /// Préférence de traitement exprimée par l'agent (ABP-107) : proposée au valideur, jamais imposée.
    #[serde(default)]
    pub preference_traitement: Option<String>,
    /// Double engagement : limite la déclaration à cet engagement (ABP-106).
    #[serde(default)]
    pub engagement_id: Option<Uuid>,
}

/// `POST /personnels/absences/signalements` (ABP-102, 103) : **sans nature**, aucun effet extérieur tant
/// qu'elle n'est pas confirmée (R-AP46).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignalerAbsenceBody {
    pub agent_id: Uuid,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
}

/// Corps commun des décisions (valider, refuser, rouvrir, annuler, traiter, clore).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionBody {
    /// Valider : traitement des heures (§9.3) ; imposé par la nature si elle est non discrétionnaire.
    #[serde(default)]
    pub traitement: Option<String>,
    /// Code court (`erreur_saisie`, `non_conforme`, …) ; jamais un diagnostic.
    #[serde(default)]
    pub raison_code: Option<String>,
    /// Note de 1000 caractères, visible des seuls instructeurs ; contrôlée comme un texte sortant (R-AP41).
    #[serde(default)]
    pub note: Option<String>,
    /// Traiter : voie de transmission (`rectorat`, `dsden`, `paie`, `comptabilite`, `autre`).
    #[serde(default)]
    pub voie_transmission: Option<String>,
}

/// `POST /personnels/absences/{id}/requalifier` (ABP-128).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequalifierBody {
    pub nature_code: String,
    #[serde(default)]
    pub raison_code: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// `POST /personnels/absences/{id}/confirmer` (ABP-112) : l'agent confirme une absence signalée et peut
/// en préciser la nature (ABP-117 : « je préciserai plus tard » devient une nature).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmerBody {
    #[serde(default)]
    pub nature_code: Option<String>,
}

/// `POST /personnels/absences/{id}/arret-recu` (ABP-131) : l'instructeur note la réception de l'arrêt
/// (échéance de 48 h, R-AP16 ; ScolIA ne sanctionne pas).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArretRecuBody {
    /// Sans identifiant : le dernier arrêt non encore reçu.
    #[serde(default)]
    pub arret_id: Option<Uuid>,
}

/// `POST /personnels/absences/{id}/prolonger` (ABP-110) ou `/ecourter` (ABP-111) : nouvelle dernière
/// journée (inclusive).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangerFinBody {
    pub fin: NaiveDate,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
}

/// `POST /personnels/absences/{id}/decision-externe` (ABP-288, 289) : l'employeur décide, l'établissement
/// enregistre.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionExterneBody {
    /// `rectorat`, `dsden`, `employeur_mutualisateur`, `autre`.
    pub employeur: String,
    /// `accordee`, `refusee`, `requalifiee`.
    pub decision: String,
    /// Requise si `requalifiee`.
    #[serde(default)]
    pub nature_code: Option<String>,
    pub le: NaiveDate,
}

/// `POST /personnels/absences/circuit-regles` (ABP-259 : règle de valideur ou d'instructeur).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreerRegleCircuitBody {
    /// `etat`, `ogec` ou `tous`.
    pub employeur: String,
    #[serde(default)]
    pub famille: Option<String>,
    /// `valideur`, `instructeur`, `informe`.
    pub role: String,
    pub agent_id: Uuid,
    #[serde(default)]
    pub rang: Option<i16>,
}

/// Une ligne du circuit figé d'une absence (ABP-120).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitLigneDto {
    pub role: String,
    pub agent_id: Uuid,
    pub rang: i16,
    /// `regle`, `responsable_fonctionnel`, `defaut`, `delegation`.
    pub source: String,
}

/// Une absence vue par le **cercle RH** : l'agent lui-même, la direction, le secrétariat. La nature n'y
/// figure que si l'appelant a le droit de la lire (`nature_code` absent sinon).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbsenceDto {
    pub id: Uuid,
    pub agent_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_label: Option<String>,
    pub etat: String,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    pub fin_provisoire: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nature_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nature_libelle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texte_libre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traitement: Option<String>,
    /// `interne` : la direction décide ; `externe` : l'établissement prend en compte, l'employeur décide.
    pub decision: String,
    /// `agent`, `rappel`, ou absent (signalée, non confirmée).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmee_par: Option<String>,
    /// L'absence regroupe un second engagement (double engagement, R-AP4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absence_liee_id: Option<Uuid>,
    #[serde(default)]
    pub circuit: Vec<CircuitLigneDto>,
}

/// Une absence vue par **ceux qui organisent** (R-AP3) : **aucun champ de nature**, de traitement, de
/// décision, de texte libre ni de circuit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceOrganisationDto {
    pub id: Uuid,
    pub agent_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_label: Option<String>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// `true` : confirmée par l'agent ou par un rappel tracé ; `false` : signalée seulement, à ne pas
    /// annoncer à l'extérieur (R-AP46).
    pub confirmee: bool,
}

/// Une réponse de déclaration : une absence par engagement touché, et les chevauchements à confirmer
/// (proposition seulement, jamais un blocage : AD-06, ABP-108).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeclarationDto {
    pub absences: Vec<AbsenceDto>,
    #[serde(default)]
    pub chevauchements: Vec<Uuid>,
}

/// Une nature proposée à l'agent pour son engagement (liste courte, dans ses mots).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatureDto {
    pub code: String,
    pub libelle: String,
    pub famille: String,
    /// Le texte libre est permis pour cette nature (jamais pour une nature sensible).
    pub texte_libre_permis: bool,
    pub piece_attendue: String,
}

/// Un fait du journal d'une absence (ABP-134) : cercle RH seulement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalAbsenceDto {
    pub le: DateTime<Utc>,
    pub action: String,
    pub auteur_user_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colonne: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valeur_avant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valeur_apres: Option<String>,
}

/// Une règle de circuit (paramétrage).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegleCircuitDto {
    pub id: Uuid,
    pub employeur: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub famille: Option<String>,
    pub role: String,
    pub agent_id: Uuid,
    pub rang: i16,
}

/// **Port « absences validées non remplacées »** (socles 10 et 19, D-058) : une séance dont l'enseignant
/// est absent (absence validée ou prise en compte) et qui n'a pas de remplaçant. Aucun motif, aucune
/// nature, aucun nom d'agent (donnée personnelle) : seulement ce qu'il faut pour dire « cours de 4ᵉ B
/// supprimé » et libérer la salle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceSansEnseignantDto {
    pub absence_id: Uuid,
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub classe: String,
    pub matiere_libelle: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salle: Option<String>,
    /// `non_remplacee` (aucun remplaçant nommé) ou `annulee` (la séance est supprimée).
    pub statut: String,
}

// ───────────────────────── Lot A2 : pièces, fil, conservation ─────────────────────────
// Ces DTO ne sont lisibles que du cercle RH (l'agent, le circuit, l'instructeur, la direction) : aucun ne
// porte un nom de fichier (minimisation) ni, hors fil, un texte libre. Aucun DTO du niveau « organisation »
// n'en dérive.

/// Dépôt d'une pièce : le fichier est envoyé en base64 comme les pièces du cahier de textes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeposerPieceBody {
    /// `arret` | `justificatif` | `convocation` | `autre`.
    pub type_piece: String,
    pub fichier: crate::cahier::FichierDepotBody,
    /// Pour un arrêt : l'agent confirme n'avoir déposé que les volets 2 et 3, jamais le volet 1 (R-08).
    #[serde(default)]
    pub volet_1_exclu: bool,
}

/// Une pièce d'une absence (sans nom de fichier).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceAbsenceDto {
    pub id: Uuid,
    pub absence_id: Uuid,
    pub type_piece: String,
    /// `deposee` | `retiree` | `purgee`.
    pub etat: String,
    pub taille_octets: i64,
    pub mime: String,
    pub deposee_par_user_id: Uuid,
    pub deposee_le: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retiree_le: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purge_prevue_le: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purgee_le: Option<DateTime<Utc>>,
}

/// Lien pré-signé court vers une pièce (5 minutes au plus), émis après contrôle du droit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LienPieceDto {
    pub url: String,
    pub expires_at: i64,
}

/// Qui a lu une pièce, et quand (ABP-144 : l'agent voit les lectures de SES pièces).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LecturePieceDto {
    pub piece_id: Uuid,
    pub lecteur_user_id: Uuid,
    pub le: DateTime<Utc>,
}

/// Un message du fil de l'absence (ajout seul, 4 000 caractères, jamais de pièce jointe).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageAbsenceDto {
    pub id: Uuid,
    pub auteur_user_id: Uuid,
    /// `agent` | `valideur` | `instructeur` | `direction` (jamais saisi par le client).
    pub role_affiche: String,
    pub texte: String,
    pub le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PosterMessageBody {
    pub texte: String,
}

/// Gel de conservation (contentieux) : suspend la purge de l'absence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GelDto {
    pub id: Uuid,
    pub absence_id: Uuid,
    pub motif: String,
    pub du: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub au: Option<NaiveDate>,
    pub pose_par_user_id: Uuid,
    pub pose_le: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leve_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoserGelBody {
    pub motif: String,
    pub du: NaiveDate,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

/// Purge d'une pièce à la demande (ABP-161) : un **code** de motif, jamais un texte libre.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurgerPieceBody {
    /// `demande_agent` | `erreur_depot` | `contentieux_clos` | `autre`.
    pub motif_code: String,
}

/// Une purge à venir (ABP-162) : la pièce, sa date, et si un gel la suspend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurgeAVenirDto {
    pub absence_id: Uuid,
    pub piece_id: Uuid,
    pub type_piece: String,
    pub etat: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purge_prevue_le: Option<NaiveDate>,
    pub gelee: bool,
}

/// Résultat d'un passage de la purge programmée (ABP-160).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BilanPurgeDto {
    pub dates_calculees: u32,
    pub pieces_purgees: u32,
    pub messages_supprimes: u32,
    /// Pièces dont la date est échue mais suspendue par un gel.
    pub pieces_gelees: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfidentialiteBody {
    pub renforcee: bool,
}

/// Une échéance à suivre par l'instructeur (ABP-147) : arrêt non encore reçu, pièce attendue manquante.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcheanceAbsenceDto {
    pub absence_id: Uuid,
    /// `arret_a_recevoir` | `piece_manquante`.
    pub genre: String,
    pub echeance_le: DateTime<Utc>,
    /// Nombre de relances déjà faites (plafonné par S-1502).
    pub relances: u32,
}

// ───────────────────────────── Lot A3 : impact, tableaux, files ─────────────────────────────
// Niveau « organisation » (§18.1) : AUCUN de ces DTO ne porte la nature, le motif, un traitement, une
// décision ni une pièce. L'agent n'y est nommé que dans le tableau, et jamais pour une absence en
// confidentialité renforcée vue d'un profil qui n'y a pas droit.

/// La séance (socle 10) touchée par une absence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceImpactDto {
    pub seance_id: Uuid,
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub classe: String,
    pub matiere_libelle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salle: Option<String>,
}

/// Un impact : ce qu'une absence touche, et où en est son traitement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactDto {
    pub id: Uuid,
    pub absence_id: Uuid,
    /// `seance` (poste et engagement : ports des autres socles, non branchés).
    pub type_impact: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seance: Option<SeanceImpactDto>,
    pub debut: DateTime<Utc>,
    pub fin: DateTime<Utc>,
    /// `a_traiter` · `propose` · `sollicite` · `remplace` · `autre_cours` · `deplace` · `continuite` · `annule`
    /// · `supplee` · `sans_objet` · `non_assure` (§10.2).
    pub etat: String,
    /// CAP-46 : la séance a disparu d'une nouvelle version d'EDT alors qu'une réponse y était posée.
    pub a_revoir: bool,
}

/// Compte d'impacts par état (tableau et tableau de bord).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompteursImpactDto {
    pub a_traiter: u32,
    pub propose: u32,
    pub sollicite: u32,
    pub remplace: u32,
    pub autre_cours: u32,
    pub deplace: u32,
    pub continuite: u32,
    pub annule: u32,
    pub supplee: u32,
    pub sans_objet: u32,
    pub non_assure: u32,
}

impl CompteursImpactDto {
    pub fn ajouter(&mut self, etat: &str) {
        match etat {
            "a_traiter" => self.a_traiter += 1,
            "propose" => self.propose += 1,
            "sollicite" => self.sollicite += 1,
            "remplace" => self.remplace += 1,
            "autre_cours" => self.autre_cours += 1,
            "deplace" => self.deplace += 1,
            "continuite" => self.continuite += 1,
            "annule" => self.annule += 1,
            "supplee" => self.supplee += 1,
            "sans_objet" => self.sans_objet += 1,
            "non_assure" => self.non_assure += 1,
            _ => {}
        }
    }
}

/// Un impact dans le tableau, avec l'agent (masqué en confidentialité renforcée).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactTableauDto {
    pub impact: ImpactDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_label: Option<String>,
}

/// Une heure du tableau : les séances qui commencent à cette heure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneTableauDto {
    pub heure_debut: NaiveTime,
    pub impacts: Vec<ImpactTableauDto>,
}

/// Tableau « ce matin » / « demain » (§10.4) : absents du jour, impacts par heure et par état.
/// Les effectifs attendus, les salles libres et les familles prévenues viennent des lots A4 et du socle 10.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableauJourDto {
    pub jour: NaiveDate,
    pub absents: Vec<AbsenceOrganisationDto>,
    pub lignes: Vec<LigneTableauDto>,
    pub compteurs: CompteursImpactDto,
}

/// Compteurs du tableau de bord (ABP-174).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompteursTableauDeBordDto {
    pub a_valider: u32,
    pub a_traiter: u32,
    pub seances_a_couvrir_aujourdhui: u32,
}

/// « Prochaine étape » d'une absence : qui doit agir, et quoi (ABP-172).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProchaineEtapeDto {
    /// `agent` | `valideur` | `instructeur` | `employeur` | `personne`.
    pub acteur: String,
    /// Code court : `confirmer` · `valider` · `instruire` · `traiter` · `decision_externe` · `clore` · `rien`.
    pub action: String,
}

/// Présence d'un agent (ABP-155) : jamais la nature ni le lieu.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceDto {
    pub agent_id: Uuid,
    /// `oui` | `non` | `inconnu`.
    pub statut: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdtReferenceBody {
    pub actif: bool,
}

/// Interrupteur « EDT de référence » (R-AP45, ABP-272) : conditionne tout effet vers les familles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdtReferenceDto {
    pub actif: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pose_le: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pose_par_user_id: Option<Uuid>,
}

/// Bilan d'un recalcul d'impacts (tâche planifiée, ABP-151).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BilanImpactDto {
    pub absences: u32,
    pub impacts: u32,
    pub a_revoir: u32,
}
