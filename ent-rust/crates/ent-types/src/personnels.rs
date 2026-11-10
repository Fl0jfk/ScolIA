//! Personnels (socle 6, lot P1) : agents, engagements, fonctions, disciplines, qualifications.
//! Routes `ent-api/src/personnels.rs` (préfixe `/personnels`). Aucune logique métier.
//!
//! Les codes (nature d'engagement, employeur, motifs, familles de fonctions) restent des chaînes :
//! la liste fermée vit dans `ent-db` (`personnels.rs`, vérifiée aussi par la base).
//! **Aucun nom, aucune coordonnée, aucun motif détaillé** ne figure dans les événements ni le
//! journal ; ces types sont les seuls à porter une identité, et seulement vers un lecteur habilité.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn vrai() -> bool {
    true
}

/// Identité saisie d'une nouvelle personne (socle 4) qui deviendra agent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonneAgentBody {
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
    pub adresse: Option<String>,
}

/// Corps de `POST /personnels/agents` (PRS-01) : une personne existante OU une nouvelle identité.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreerAgentBody {
    /// Personne déjà connue de l'établissement (professeur-parent, responsable, contact).
    #[serde(default)]
    pub personne_id: Option<Uuid>,
    /// Nouvelle identité : l'établissement cherche d'abord les correspondances.
    #[serde(default)]
    pub personne: Option<PersonneAgentBody>,
    /// Confirme la création d'une nouvelle personne malgré des correspondances proposées.
    #[serde(default)]
    pub confirmer_nouvelle: bool,
    #[serde(default)]
    pub matricule_interne: Option<String>,
    /// Fonction principale (code du catalogue) ; facultative à la création.
    #[serde(default)]
    pub fonction_code: Option<String>,
    #[serde(default)]
    pub personnel_mineur: bool,
}

/// Personne qui pourrait être celle que l'on veut créer (e-mail masqué, jamais de coordonnée).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrespondanceAgentDto {
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub email_masque: Option<String>,
    /// `email` (candidat, jamais une preuve) ou `nom`.
    pub raison: String,
    pub a_un_compte: bool,
    /// La personne est déjà agent de cet établissement.
    pub est_agent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCreeDto {
    pub agent_id: Uuid,
    pub personne_id: Uuid,
    /// Codes : `personne_est_eleve` (R-P5), `personne_responsable` (casquettes parent et personnel).
    #[serde(default)]
    pub avertissements: Vec<String>,
}

/// Réponse 409 de la création quand des correspondances doivent être tranchées.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrespondancesAConfirmerDto {
    pub code: String,
    pub correspondances: Vec<CorrespondanceAgentDto>,
}

/// Corps de `PUT /personnels/agents/{id}/identite` (PRS-02) : identité de la personne (socle 4).
pub type ModifierIdentiteBody = PersonneAgentBody;

/// Engagement à créer (PRS-03).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EngagementBody {
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    pub nature: String,
    pub employeur_type: String,
    #[serde(default)]
    pub employeur_organisme_id: Option<Uuid>,
    #[serde(default)]
    pub quotite_pct: Option<f64>,
    #[serde(default)]
    pub jours_travailles: Option<Vec<i16>>,
    #[serde(default)]
    pub heures_dues_hebdo: Option<f64>,
    #[serde(default)]
    pub motif_arrivee: Option<String>,
    #[serde(default)]
    pub referent_agent_id: Option<Uuid>,
    #[serde(default)]
    pub remplace_agent_id: Option<Uuid>,
}

/// Modification d'un engagement (PRS-04) : remplace les champs modifiables.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModifierEngagementBody {
    pub nature: String,
    pub employeur_type: String,
    #[serde(default)]
    pub employeur_organisme_id: Option<Uuid>,
    #[serde(default)]
    pub quotite_pct: Option<f64>,
    #[serde(default)]
    pub jours_travailles: Option<Vec<i16>>,
    #[serde(default)]
    pub heures_dues_hebdo: Option<f64>,
    #[serde(default)]
    pub referent_agent_id: Option<Uuid>,
    /// Prise de fonctions effective (PRS-116) ; jamais avant le début, jamais dans le futur.
    #[serde(default)]
    pub installe_le: Option<NaiveDate>,
}

/// Prolongation : nouvelle date de fin (`null` = sans fin).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProlongerEngagementBody {
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

/// Fin d'engagement : date et motif **codé** (jamais de texte).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminerEngagementBody {
    pub au: NaiveDate,
    pub motif_fin: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuspensionDto {
    pub id: Uuid,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    /// Motif de la suspension : lu avec `personnels.lire` seulement (sinon `null`).
    pub motif_code: Option<String>,
    pub acces_suspendu: bool,
    pub levee: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngagementDto {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub nature: String,
    pub employeur_type: String,
    pub employeur_organisme_id: Option<Uuid>,
    pub quotite_pct: Option<f64>,
    pub jours_travailles: Option<Vec<i16>>,
    pub heures_dues_hebdo: Option<f64>,
    pub motif_arrivee: Option<String>,
    pub motif_fin: Option<String>,
    pub referent_agent_id: Option<Uuid>,
    pub remplace_agent_id: Option<Uuid>,
    pub installe_le: Option<NaiveDate>,
    /// Suspendu à la date de lecture (disponibilité, détachement, congé parental).
    pub suspendu: bool,
    #[serde(default)]
    pub suspensions: Vec<SuspensionDto>,
    /// Alertes de qualité : `nature_a_preciser`, `meme_employeur_simultane`, `fin_a_saisir`.
    #[serde(default)]
    pub alertes: Vec<String>,
}

/// Résultat d'une écriture d'engagement : l'engagement et ses avertissements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngagementEcritDto {
    pub engagement: EngagementDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuspendreEngagementBody {
    pub du: NaiveDate,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    /// `disponibilite`, `detachement`, `conge_parental`, `conge_long`, `formation`, `autre`.
    pub motif_code: String,
    /// Par défaut l'accès est suspendu pendant la suspension (R-P4).
    #[serde(default = "vrai")]
    pub acces_suspendu: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReprendreEngagementBody {
    /// Premier jour de reprise : la suspension se termine la veille (rien à recréer, pas d'invitation).
    pub date_reprise: NaiveDate,
}

// ---- Fonctions, disciplines, qualifications ------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FonctionCatalogueDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub famille: String,
    pub profil_propose: Option<String>,
    /// `true` : fonction créée par l'établissement ; `false` : fournie par ScolIA.
    pub propre_etablissement: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreerFonctionBody {
    pub code: String,
    pub libelle: String,
    pub famille: String,
}

/// PRS-05 : ajouter une fonction à un agent (code du catalogue, fourni ou propre à l'établissement).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentFonctionBody {
    pub fonction_code: String,
    #[serde(default)]
    pub principale: bool,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentFonctionDto {
    pub id: Uuid,
    pub fonction_code: String,
    pub libelle: String,
    pub famille: String,
    pub principale: bool,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminerLigneBody {
    pub au: NaiveDate,
}

/// PRS-06 : discipline de poste (matière du référentiel et/ou code national).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDisciplineBody {
    #[serde(default)]
    pub matiere_id: Option<Uuid>,
    #[serde(default)]
    pub code_discipline_poste: Option<String>,
    #[serde(default)]
    pub principale: bool,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDisciplineDto {
    pub id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub code_discipline_poste: Option<String>,
    pub principale: bool,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
}

/// PRS-07 : qualification (secourisme, encadrement, permis…).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationBody {
    pub code: String,
    #[serde(default)]
    pub precision: Option<String>,
    #[serde(default)]
    pub obtenue_le: Option<NaiveDate>,
    #[serde(default)]
    pub valable_jusqu_au: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationDto {
    pub id: Uuid,
    pub code: String,
    pub precision: Option<String>,
    pub obtenue_le: Option<NaiveDate>,
    pub valable_jusqu_au: Option<NaiveDate>,
    /// Expirée à la date de lecture.
    pub expiree: bool,
}

// ---- Choix de l'agent ---------------------------------------------------------------------------

/// PRS-08 : contact d'urgence, facultatif, sur décision de l'agent ; lu par la direction seule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactUrgenceAgentBody {
    pub ordre: i16,
    pub nom: String,
    #[serde(default)]
    pub prenom: Option<String>,
    #[serde(default)]
    pub lien: Option<String>,
    pub telephone: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactUrgenceAgentDto {
    pub id: Uuid,
    pub ordre: i16,
    pub nom: String,
    pub prenom: Option<String>,
    pub lien: Option<String>,
    pub telephone: String,
}

/// PRS-09 : visibilité de la photo choisie par l'agent (le fichier relève du socle 7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoAgentBody {
    #[serde(default)]
    pub visible_collegues: bool,
    #[serde(default)]
    pub visible_eleves: bool,
    #[serde(default)]
    pub visible_familles: bool,
}

/// PRS-10 : pause des notifications jusqu'à une date (`null` = reprendre). Avis de sécurité maintenus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseNotificationsBody {
    #[serde(default)]
    pub jusqu_au: Option<NaiveDate>,
}

/// PRS-11 : identité restreinte (direction) — `motif` : `menace`, `protection_judiciaire`, `autre`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentiteRestreinteBody {
    pub restreinte: bool,
    #[serde(default)]
    pub motif: Option<String>,
}

/// PRS-12 : signaler un personnel mineur.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonnelMineurBody {
    pub personnel_mineur: bool,
}

/// PRS-13 : fusion de deux agents en double (après la fusion des personnes du socle 4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FusionnerAgentsBody {
    pub source_agent_id: Uuid,
    pub cible_agent_id: Uuid,
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FusionAgentsDto {
    pub source_agent_id: Uuid,
    pub cible_agent_id: Uuid,
    pub engagements_deplaces: i64,
}

/// Identifiant externe d'un agent (STS, AAF, guichet Agents, GAR, emploi du temps…).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifiantAgentBody {
    pub systeme: String,
    pub valeur: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifiantAgentDto {
    pub id: Uuid,
    pub systeme: String,
    pub valeur: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
}

// ---- Lectures ---------------------------------------------------------------------------------

/// Ligne de l'annuaire interne. Jamais de téléphone ; l'adresse e-mail n'est rendue qu'à
/// l'annuaire interne (jamais aux familles ni aux élèves).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentResumeDto {
    pub agent_id: Uuid,
    pub nom_affiche: String,
    pub fonction_code: Option<String>,
    pub fonction_libelle: Option<String>,
    pub famille: Option<String>,
    /// `prevu`, `actif`, `suspendu`, `parti`.
    pub etat: String,
    pub email: Option<String>,
    /// Compte de connexion rattaché (lecteurs internes seulement).
    pub user_id: Option<Uuid>,
    pub identite_restreinte: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentiteAgentDto {
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub civilite: Option<String>,
    pub email: Option<String>,
}

/// Fiche d'organisation d'un agent (`personnels.lire`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FicheAgentDto {
    pub agent_id: Uuid,
    pub identite: IdentiteAgentDto,
    pub etat: String,
    pub source: String,
    pub matricule_interne: Option<String>,
    pub personnel_mineur: bool,
    pub identite_restreinte: bool,
    /// Motif de l'identité restreinte : `personnels.confidentiel.gerer` seulement.
    pub identite_restreinte_motif: Option<String>,
    pub identite_a_separer: bool,
    pub notifications_en_pause_jusqu_au: Option<NaiveDate>,
    pub photo_visible_collegues: bool,
    pub photo_visible_eleves: bool,
    pub photo_visible_familles: bool,
    pub engagements: Vec<EngagementDto>,
    pub fonctions: Vec<AgentFonctionDto>,
    pub disciplines: Vec<AgentDisciplineDto>,
    pub qualifications: Vec<QualificationDto>,
    pub identifiants: Vec<IdentifiantAgentDto>,
    /// `null` sans `personnels.personnel.lire` (et hors de l'agent lui-même).
    pub contacts_urgence: Option<Vec<ContactUrgenceAgentDto>>,
}

// ---------------------------------------------------------------------------------------------
// Lot P2 : invitations, accueil, casquettes (PRS-20 à PRS-29)
// ---------------------------------------------------------------------------------------------
//
// **Aucun jeton ne figure dans ces types, sauf `InvitationQrDto::lien`** : le lien d'une
// invitation par QR code est le contenu du QR, affiché une seule fois sur l'écran authentifié du
// secrétariat qui vient de l'émettre. Aucune lecture ultérieure (liste, relance, annulation) ne le
// renvoie ; la base ne conserve que son empreinte.

/// Corps de `POST /personnels/agents/{id}/invitations` (PRS-20, PRS-21).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviterAgentBody {
    /// `email` (lien valable 7 jours) ou `qr` (valable 10 minutes, affiché au guichet).
    pub canal: String,
    /// Profil de base que l'accès recevra (`prof`, `cpe`, `accueil`, `infirmerie`, `psy`,
    /// `administratif`, `direction`, `admin`) ; la hiérarchie des comptes s'applique.
    pub profil: String,
    /// Adresse d'envoi (canal e-mail) ; à défaut, l'adresse de la fiche de la personne.
    #[serde(default)]
    pub email: Option<String>,
}

/// Réponse d'une invitation par e-mail : jamais de jeton, jamais l'adresse en clair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitationDto {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub canal: String,
    pub profil: String,
    /// `en_attente`, `expiree`, `utilisee`, `annulee`.
    pub etat: String,
    pub email_masque: Option<String>,
    pub cree_le: chrono::DateTime<chrono::Utc>,
    pub expire_le: chrono::DateTime<chrono::Utc>,
    pub utilisee_le: Option<chrono::DateTime<chrono::Utc>>,
    pub annulee_le: Option<chrono::DateTime<chrono::Utc>>,
    /// L'invitation a rattaché une identité qui existait déjà.
    pub rattachement_identite_existante: bool,
}

/// Réponse de l'émission d'une invitation par QR code : seule réponse qui porte le lien.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitationQrDto {
    pub invitation: InvitationDto,
    /// Contenu du QR code (à afficher, jamais à journaliser). Valable 10 minutes, usage unique.
    pub lien: String,
}

/// Corps de `POST /personnels/invitations/lot` (PRS-22).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitationsLotBody {
    pub agent_ids: Vec<Uuid>,
    pub profil: String,
    /// `true` : aperçu seul (aucun envoi, aucune écriture).
    #[serde(default)]
    pub apercu: bool,
}

/// Une ligne du lot : `envoyee`, `a_envoyer` (aperçu), `adresse_manquante`, `deja_un_acces`,
/// `non_autorisee`, `introuvable`, `echec_envoi`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitationLotLigneDto {
    pub agent_id: Uuid,
    pub resultat: String,
}

/// Aperçu de l'invitation pour la personne qui détient le lien (public : le jeton est la preuve).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitationApercuDto {
    pub etablissement_id: Uuid,
    pub canal: String,
    pub profil: String,
    pub expire_le: chrono::DateTime<chrono::Utc>,
    /// Une identité existe déjà : se connecter (passkey) puis confirmer le rattachement.
    pub identite_existante: bool,
    /// L'engagement commence plus tard : l'accès sera réduit jusqu'à la date d'arrivée.
    pub avant_arrivee: bool,
}

/// Corps de `POST /invitations/personnel/consulter`.
#[derive(Clone, Deserialize)]
pub struct InvitationJetonBody {
    pub token: String,
}

/// Corps de `POST /invitations/personnel/accepter`. Le mot de passe n'est jamais journalisé.
#[derive(Clone, Deserialize)]
pub struct AccepterInvitationBody {
    pub token: String,
    /// Nouvelle identité : mot de passe choisi par la personne.
    #[serde(default)]
    pub mot_de_passe: Option<String>,
    /// Nouvelle identité, invitation par QR code (sans adresse mémorisée).
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub nom_affiche: Option<String>,
    /// Identité existante : confirmation explicite du rattachement (jamais implicite).
    #[serde(default)]
    pub confirmer_rattachement: bool,
}

impl std::fmt::Debug for AccepterInvitationBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccepterInvitationBody")
            .field("confirmer_rattachement", &self.confirmer_rattachement)
            .finish_non_exhaustive()
    }
}

/// Issue de l'acceptation. Aucune session n'est ouverte ici : la personne se connecte ensuite
/// (mot de passe puis enrôlement de la passkey, ou passkey) ; l'enrôlement est exigé pour la
/// casquette personnel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitationAccepteeDto {
    /// `compte_cree` ou `identite_rattachee`.
    pub resultat: String,
    pub etablissement_id: Uuid,
    /// `true` : la connexion doit enrôler une passkey avant tout accès.
    pub passkey_a_enroler: bool,
    /// `true` : l'accès est réduit jusqu'à la date d'arrivée.
    pub avant_arrivee: bool,
    /// Casquettes ouvertes sur ce compte dans l'établissement.
    pub casquettes: Vec<String>,
}

/// Une casquette ouverte pour la session (PRS-29).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasquetteDto {
    /// `principale` ou `parent`.
    pub code: String,
    /// Libellé neutre : `Espace professeur`, `Espace parent`…
    pub libelle: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasquettesDto {
    pub casquettes: Vec<CasquetteDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasculerCasquetteBody {
    pub casquette: String,
}

/// « Préparer mon arrivée » (PRS-25) : aucune donnée d'élève.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparerArriveeDto {
    pub avant_arrivee: bool,
    pub arrivee_le: Option<NaiveDate>,
    pub collegues: Vec<AgentResumeDto>,
}
