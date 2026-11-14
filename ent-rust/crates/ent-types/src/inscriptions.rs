//! Admissions, préinscriptions, inscriptions et réinscriptions (socle 2, lot S2-3 ;
//! D-014). Routes `ent-api/src/inscriptions.rs` (préfixes `/inscriptions`,
//! `/preinscription`, `/imports`). Aucune logique métier : l'autorité est `ent-db`.
//!
//! États (chaînes) :
//! - campagne : `"brouillon"`, `"planifiee"`, `"ouverte"`, `"suspendue"`, `"fermee"`,
//!   `"archivee"` ; nature `"admission"`, `"affectes"`, `"reinscription"` ou
//!   `"en_cours_annee"` ;
//! - dossier : **`etat`** (lot I2, `03_INSCRIPTIONS.md` §7 : table de transitions dans
//!   `ent-db::inscriptions_etats`) et, pour les anciens lecteurs, `statut` : `"brouillon"`,
//!   `"deposee"`, `"en_instruction"`, `"incomplet"`, `"liste_attente"`, `"admis"`, `"refuse"`,
//!   `"validee"`, `"desistement"` ;
//! - pièce : `"attendue"`, `"deposee"`, `"acceptee"`, `"refusee"` ;
//! - ligne d'import : `"a_creer"`, `"existant"`, `"rejet"`, `"creee"`, `"ignoree"`.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Campagnes (INS-01, INS-06)
// ---------------------------------------------------------------------------

/// Pièce demandée par une campagne.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceCampagneDto {
    pub code: String,
    pub libelle: String,
    pub obligatoire: bool,
}

/// Niveau ouvert par une campagne, avec sa capacité (places) facultative.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NiveauCampagneDto {
    pub code: String,
    pub capacite: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampagneDto {
    pub id: Uuid,
    pub nature: String,
    pub libelle: String,
    pub annee_cible: String,
    pub statut: String,
    pub date_ouverture: Option<NaiveDate>,
    pub date_cloture: Option<NaiveDate>,
    pub message_famille: Option<String>,
    pub niveaux: Vec<NiveauCampagneDto>,
    pub pieces: Vec<PieceCampagneDto>,
    /// Nombre de dossiers par statut (personnel seulement ; vide pour les familles).
    #[serde(default)]
    pub compteurs: Vec<CompteurStatutDto>,
    /// Année du référentiel (lot I2) ; absente tant que le référentiel ne la connaît pas.
    #[serde(default)]
    pub annee_id: Option<Uuid>,
    /// Version de la campagne : chaque modification en crée une (journal).
    #[serde(default)]
    pub version: i32,
    #[serde(default)]
    pub regles: ReglesCampagneDto,
    /// Offres et places (personnel seulement ; vide pour les familles).
    #[serde(default)]
    pub offres: Vec<OffreOccupationDto>,
}

/// Règles paramétrables d'une campagne (`03_INSCRIPTIONS.md` §8.1), dans les bornes du logiciel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglesCampagneDto {
    /// `"un_seul"` ou `"tous_les_titulaires"` (accord des responsables).
    pub accord_responsables: String,
    pub decision_requise: bool,
    /// `"jamais"`, `"facultatif"`, `"toujours"`.
    pub motif_communique: String,
    /// `"tacite"`, `"explicite"`, `"explicite_avec_acompte"` (réinscription).
    #[serde(default)]
    pub mode_reinscription: Option<String>,
    #[serde(default)]
    pub date_limite_famille: Option<NaiveDate>,
    #[serde(default)]
    pub date_cloture_depot: Option<NaiveDate>,
    /// Délai de réponse de la famille, 2 à 30 jours.
    #[serde(default)]
    pub delai_reponse_jours: Option<i16>,
    /// Délai pendant lequel une validation erronée peut être annulée, 0 à 168 heures.
    pub delai_annulation_validation_h: i16,
    pub page_publique: bool,
    pub au_fil_de_l_eau: bool,
}

impl Default for ReglesCampagneDto {
    fn default() -> Self {
        Self {
            accord_responsables: "un_seul".into(),
            decision_requise: true,
            motif_communique: "facultatif".into(),
            mode_reinscription: None,
            date_limite_famille: None,
            date_cloture_depot: None,
            delai_reponse_jours: None,
            delai_annulation_validation_h: 72,
            page_publique: false,
            au_fil_de_l_eau: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompteurStatutDto {
    pub statut: String,
    pub nombre: i64,
}

/// Corps de `POST /inscriptions/campagnes` et `PUT /inscriptions/campagnes/{id}`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampagneBody {
    pub nature: String,
    pub libelle: String,
    pub annee_cible: String,
    #[serde(default)]
    pub date_ouverture: Option<NaiveDate>,
    #[serde(default)]
    pub date_cloture: Option<NaiveDate>,
    #[serde(default)]
    pub message_famille: Option<String>,
    #[serde(default)]
    pub niveaux: Vec<NiveauCampagneDto>,
    #[serde(default)]
    pub pieces: Vec<PieceCampagneDto>,
    /// Règles de la campagne ; absentes = valeurs actuelles (création : valeurs par défaut de la
    /// nature).
    #[serde(default)]
    pub regles: Option<ReglesCampagneDto>,
    /// Motif d'une modification après création (journal des versions).
    #[serde(default)]
    pub motif_modification: Option<String>,
}

/// Résultat de l'ouverture d'une campagne (réinscription : dossiers préparés).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OuvertureCampagneDto {
    pub campagne: CampagneDto,
    pub dossiers_prepares: i64,
}

// ---------------------------------------------------------------------------
// Dossiers
// ---------------------------------------------------------------------------

/// Élève tel que saisi dans le formulaire (copie propre à l'établissement).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveFormulaireDto {
    pub nom: String,
    pub prenom: String,
    #[serde(default)]
    pub date_naissance: Option<NaiveDate>,
    #[serde(default)]
    pub sexe: Option<String>,
    #[serde(default)]
    pub ine: Option<String>,
    #[serde(default)]
    pub niveau_demande: Option<String>,
    #[serde(default)]
    pub regime_demande: Option<String>,
    #[serde(default)]
    pub etablissement_origine: Option<String>,
}

/// Responsable tel que saisi (rang 1 = déposant, rang 2 = second parent).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponsableFormulaireDto {
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
    /// `"mere"`, `"pere"`, `"tuteur"`, `"autre"`.
    #[serde(default)]
    pub lien: Option<String>,
    /// Second parent : même foyer que le déposant ? `false` = parents séparés
    /// (une fiche par parent, e-mail obligatoire, D-014).
    #[serde(default = "vrai")]
    pub meme_foyer: bool,
    #[serde(default = "vrai")]
    pub autorite_parentale: bool,
    /// Champs réservés du contrat `DeclarationResponsables` v1 (socle 4, lot A0 bis). Jamais
    /// rendus à la famille.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserve: Option<ReserveResponsableDto>,
}

/// Champs réservés d'un responsable déclaré (tous facultatifs). Une adresse à protéger n'est
/// pas enregistrée en clair ; « ne pas solliciter » bloque toute invitation, demande d'accord
/// et relance vers cette personne.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReserveResponsableDto {
    #[serde(default)]
    pub adresse_a_proteger: bool,
    #[serde(default)]
    pub ne_pas_solliciter: bool,
    /// Référence de la pièce de protection ; entrée seulement (jamais renvoyée).
    #[serde(default, skip_serializing)]
    pub ne_pas_solliciter_ref: Option<String>,
    #[serde(default)]
    pub langue: Option<String>,
    #[serde(default)]
    pub besoin_interprete: bool,
    #[serde(default)]
    pub organisme: Option<String>,
    #[serde(default)]
    pub nature_code: Option<String>,
}

fn vrai() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponsableDossierDto {
    pub rang: i16,
    pub formulaire: ResponsableFormulaireDto,
    /// Invitation envoyée à la validation : `"envoyee"`, `"acceptee"`, … (`None` : aucune).
    pub invitation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceDossierDto {
    pub code: String,
    pub libelle: String,
    pub obligatoire: bool,
    pub statut: String,
    pub nom_fichier: Option<String>,
    pub motif_refus: Option<String>,
}

/// Entrée du suivi d'un dossier (changement d'étape, commentaire, pièce, relance).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiviDossierDto {
    pub quand: DateTime<Utc>,
    pub kind: String,
    pub statut_apres: Option<String>,
    pub texte: Option<String>,
    pub visible_famille: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DossierResumeDto {
    pub id: Uuid,
    pub campagne_id: Uuid,
    pub nature: String,
    pub statut: String,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub niveau_demande: Option<String>,
    pub eleve_id: Option<Uuid>,
    pub depose_le: Option<DateTime<Utc>>,
    pub pieces_manquantes: i64,
    /// État (§7) et numéro lisible du dossier (lot I2).
    #[serde(default)]
    pub etat: String,
    #[serde(default)]
    pub reference: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DossierDto {
    pub id: Uuid,
    pub campagne_id: Uuid,
    pub campagne_libelle: String,
    pub nature: String,
    pub statut: String,
    pub eleve: EleveFormulaireDto,
    pub eleve_id: Option<Uuid>,
    pub classe_cible_id: Option<Uuid>,
    pub responsables: Vec<ResponsableDossierDto>,
    pub pieces: Vec<PieceDossierDto>,
    pub suivi: Vec<SuiviDossierDto>,
    /// Motif de décision (refus, liste d'attente), visible de la famille.
    pub decision_motif: Option<String>,
    pub depose_le: Option<DateTime<Utc>>,
    pub valide_le: Option<DateTime<Utc>>,
    pub reutilise_identite: bool,
    /// Actions possibles pour le lecteur (codes stables).
    pub actions: Vec<String>,
    /// État du dossier (§7) ; `statut` n'en est que la vue historique.
    #[serde(default)]
    pub etat: String,
    /// Numéro lisible donné à la famille (`ADM-2026-0142`).
    #[serde(default)]
    pub reference: String,
    /// Démarche familiale (fratrie : plusieurs dossiers pour une démarche).
    #[serde(default)]
    pub demarche_id: Option<Uuid>,
    /// Prochaine échéance de la famille (calculée, décalée par les fermetures).
    #[serde(default)]
    pub echeance_famille: Option<NaiveDate>,
    #[serde(default)]
    pub rang_attente: Option<i32>,
    /// Fermeture du secrétariat qui suspend les délais (message affiché à la famille).
    #[serde(default)]
    pub fermeture: Option<FermetureDto>,
}

/// Corps de `POST /inscriptions/dossiers/{id}/statut` (instruction).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatutBody {
    pub statut: String,
    #[serde(default)]
    pub motif: Option<String>,
}

/// Corps de `POST /inscriptions/dossiers/{id}/pieces/{code}` (personnel).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceDecisionBody {
    /// `"acceptee"` ou `"refusee"` (motif obligatoire).
    pub statut: String,
    #[serde(default)]
    pub motif: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentaireBody {
    pub texte: String,
    #[serde(default)]
    pub visible_famille: bool,
}

/// Corps de `POST /inscriptions/dossiers/{id}/decision` (INS-04).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionBody {
    /// `"admis"`, `"liste_attente"`, `"refuse"`.
    pub decision: String,
    #[serde(default)]
    pub motif: Option<String>,
}

/// Corps de `POST /inscriptions/dossiers/{id}/decision` (INS-04), forme complète : la forme
/// `{decision, motif}` de [`DecisionBody`] reste valide (mêmes champs, les autres facultatifs).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionAvanceeBody {
    /// `"admis"`, `"liste_attente"`, `"refuse"`.
    pub decision: String,
    #[serde(default)]
    pub motif: Option<String>,
    /// Motif communiqué à la famille (selon la règle de la campagne).
    #[serde(default)]
    pub motif_communique: Option<String>,
    /// Offre sur laquelle porte la décision ; à défaut, le premier vœu du dossier.
    #[serde(default)]
    pub offre_id: Option<Uuid>,
    /// Admettre au-delà de la capacité : confirmé et tracé (jamais en silence).
    #[serde(default)]
    pub confirmer_depassement: bool,
}

/// Corps de `POST /inscriptions/dossiers/{id}/valider` (INS-05).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationBody {
    /// Classe choisie dans la liste : **arrivée en cours d'année seulement** (classe de l'année
    /// courante). Avant la rentrée, la validation n'affecte personne (socle 5).
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub regime: Option<String>,
    /// Niveau de l'inscription annuelle prévue (à défaut : le niveau demandé du dossier).
    #[serde(default)]
    pub niveau: Option<String>,
    /// Parcours (option, section, filière) de l'inscription annuelle prévue.
    #[serde(default)]
    pub parcours: Option<String>,
    /// Choix humains de rapprochement (I0, §2.2 pts 1-3). Obligatoire pour tout
    /// responsable ayant des correspondances possibles (`GET …/correspondances`).
    #[serde(default)]
    pub responsables: Vec<ResolutionResponsable>,
    /// Obligatoire si une fiche élève correspond déjà (homonyme exact ou INE).
    #[serde(default)]
    pub eleve: Option<ChoixEleve>,
}

/// Décision de la personne de l'établissement pour un responsable déclaré.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionResponsable {
    pub responsable_id: Uuid,
    pub personne: ChoixPersonne,
    /// Obligatoire quand la personne choisie appartient déjà à un foyer.
    #[serde(default)]
    pub foyer: Option<ChoixFoyer>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "choix", rename_all = "snake_case")]
pub enum ChoixPersonne {
    /// Créer une nouvelle fiche (les correspondances sont écartées en connaissance de cause).
    Nouvelle,
    /// Rattacher à une fiche existante de l'établissement.
    Existante { personne_id: Uuid },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "choix", rename_all = "snake_case")]
pub enum ChoixFoyer {
    Nouveau,
    Existant { foyer_id: Uuid },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "choix", rename_all = "snake_case")]
pub enum ChoixEleve {
    /// Homonyme différent : créer une nouvelle fiche (refusé si l'INE est identique).
    Nouveau,
    /// Ancien élève qui revient : rattacher le dossier à la fiche existante.
    /// `confirme_responsables_existants` : obligatoire si la fiche a déjà des responsables
    /// liés (le dossier ne doit pas donner accès à un élève déjà suivi par d'autres).
    Existant {
        eleve_id: Uuid,
        #[serde(default)]
        confirme_responsables_existants: bool,
    },
}

/// `GET /inscriptions/dossiers/{id}/correspondances` : de quoi décider à la validation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrespondancesDto {
    pub eleves: Vec<EleveCandidatDto>,
    pub responsables: Vec<ResponsableCorrespondancesDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveCandidatDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub date_naissance: Option<NaiveDate>,
    pub statut: String,
    pub ine_identique: bool,
    /// Responsables déjà liés à cette fiche (à vérifier avant tout rattachement).
    pub responsables_lies: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponsableCorrespondancesDto {
    pub responsable_id: Uuid,
    pub rang: i16,
    pub candidats: Vec<PersonneCandidateDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonneCandidateDto {
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub email_masque: Option<String>,
    /// `"email"` (même adresse) ou `"nom"` (même nom et prénom).
    pub raison: String,
    pub a_un_compte: bool,
    pub foyers: Vec<FoyerCandidatDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoyerCandidatDto {
    pub foyer_id: Uuid,
    pub libelle: String,
}

/// Droits attribués à un lien de responsabilité selon son type (paramétrage établissement).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametreLienDto {
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    #[serde(default)]
    pub peut_autoriser_sorties: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationDto {
    pub dossier_id: Uuid,
    pub eleve_id: Uuid,
    pub evenement_id: Uuid,
    /// Inscription annuelle **prévue** créée par la validation (socle 2).
    #[serde(default)]
    pub inscription_annuelle_id: Option<Uuid>,
    /// `true` seulement pour une arrivée en cours d'année avec une classe choisie.
    #[serde(default)]
    pub affecte_en_classe: bool,
}

// ---------------------------------------------------------------------------
// Préinscription par la famille (INS-02, D-014)
// ---------------------------------------------------------------------------

/// `POST /preinscription/demarrer` : réponse **identique** que le compte existe ou non.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemarrerBody {
    pub email: String,
    /// Ignoré quand l'établissement est imposé par l'adresse (D-012).
    #[serde(default)]
    pub etablissement_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemarrerDto {
    pub demande_id: Uuid,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifierBody {
    pub demande_id: Uuid,
    pub code: String,
}

/// Jeton de préinscription (en-tête `X-Preinscription`), court et révocable.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JetonPreinscriptionDto {
    pub jeton: String,
    pub expire_le: DateTime<Utc>,
}

impl std::fmt::Debug for JetonPreinscriptionDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JetonPreinscriptionDto")
            .field("jeton", &"[masqué]")
            .field("expire_le", &self.expire_le)
            .finish()
    }
}

/// Informations proposées au pré-remplissage (profil choisi par la personne).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfilDto {
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub civilite: Option<String>,
    pub telephone: Option<String>,
    pub adresse: Option<String>,
}

/// `GET /preinscription/moi` (après vérification seulement).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EspacePreinscriptionDto {
    pub email: String,
    /// Personne déjà connue de l'ENT : « Nous avons déjà vos informations. Les utiliser ? »
    pub connu: bool,
    pub profil: Option<ProfilDto>,
    pub campagnes: Vec<CampagneDto>,
    pub dossiers: Vec<DossierResumeDto>,
    /// Message de transparence (D-014), toujours affiché.
    pub message_confidentialite: String,
}

/// `POST /preinscription/dossiers`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreinscriptionBody {
    pub campagne_id: Uuid,
    /// « Utiliser mes informations ? » : oui → pré-remplissage accepté (tracé).
    #[serde(default)]
    pub utiliser_mes_infos: bool,
    /// « Enregistrer ces informations dans votre profil ? » (non par défaut).
    #[serde(default)]
    pub enregistrer_profil: bool,
    pub eleve: EleveFormulaireDto,
    pub responsable: ResponsableFormulaireDto,
    #[serde(default)]
    pub second_parent: Option<ResponsableFormulaireDto>,
}

/// `POST /preinscription/dossiers/{id}/pieces/{code}` (déclaration par la famille).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceDeposeeBody {
    pub nom_fichier: String,
}

/// IBAN proposé à la reprise, **masqué** (`FR76 •••• 1234`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IbanProposeDto {
    pub proposition_id: Uuid,
    pub iban_masque: String,
    pub titulaire_initiales: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordonneesBancairesDto {
    /// Propositions de reprise (vide si l'identité n'a pas été réutilisée).
    pub propositions: Vec<IbanProposeDto>,
    /// Mandat en attente de signature ou signé.
    pub mandat: Option<MandatDemandeDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MandatDemandeDto {
    pub id: Uuid,
    pub iban_masque: String,
    pub source: String,
    pub statut: String,
    pub rum: Option<String>,
}

/// `POST /preinscription/dossiers/{id}/mandat` : reprise d'une proposition ou saisie.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MandatBody {
    #[serde(default)]
    pub proposition_id: Option<Uuid>,
    #[serde(default)]
    pub iban: Option<String>,
    #[serde(default)]
    pub bic: Option<String>,
    #[serde(default)]
    pub titulaire: Option<String>,
}

impl std::fmt::Debug for MandatBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MandatBody")
            .field("proposition_id", &self.proposition_id)
            .field("iban", &self.iban.as_ref().map(|_| "[masqué]"))
            .finish()
    }
}

/// `POST /inscriptions/invitations/accepter` (lien reçu par e-mail).
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitationBody {
    pub jeton: String,
}

impl std::fmt::Debug for InvitationBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvitationBody { jeton: [masqué] }")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitationAccepteeDto {
    /// `"compte_rattache"` (identité existante) ou `"compte_cree"`.
    pub resultat: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Réinscription par la famille (INS-07, INS-08)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationReinscriptionBody {
    #[serde(default)]
    pub regime: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepartBody {
    pub motif: String,
    #[serde(default)]
    pub date_depart: Option<NaiveDate>,
}

// ---------------------------------------------------------------------------
// Import (IOP-01 format ScolIA documenté, IOP-04 rejets)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportBody {
    pub nom_fichier: String,
    /// Contenu CSV (UTF-8, séparateur `;`, en-tête obligatoire) — format `scolia_eleves_v1`.
    pub contenu: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneImportDto {
    pub id: Uuid,
    pub numero: i32,
    pub statut: String,
    pub motifs: Vec<String>,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub eleve_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LotImportDto {
    pub id: Uuid,
    pub format: String,
    pub nom_fichier: String,
    pub statut: String,
    pub nb_lignes: i32,
    pub nb_a_creer: i32,
    pub nb_existantes: i32,
    pub nb_rejets: i32,
    pub nb_creees: i32,
    pub lignes: Vec<LigneImportDto>,
}

/// `POST /imports/lignes/{id}/traiter` : `"ignorer"` ou `"corriger"` (champs remplacés).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraiterLigneBody {
    pub action: String,
    #[serde(default)]
    pub corrections: std::collections::BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Offre, capacités et compteur de places (lot I2, §8.2)
// ---------------------------------------------------------------------------

/// Corps de `POST /inscriptions/campagnes/{id}/offres` et `PUT /inscriptions/offres/{id}`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffreBody {
    /// Code du niveau (`6EME`…) ; rapproché du référentiel quand il existe.
    pub niveau_code: String,
    #[serde(default)]
    pub parcours_code: Option<String>,
    /// `"externe"`, `"demi_pension"`, `"interne"`.
    #[serde(default)]
    pub regime: Option<String>,
    pub libelle_public: String,
    /// `None` = capacité non limitée.
    #[serde(default)]
    pub capacite: Option<i32>,
    #[serde(default)]
    pub places_reservees_reinscription: Option<bool>,
    #[serde(default)]
    pub ouverte_candidature: Option<bool>,
    #[serde(default)]
    pub conditions_acces: Option<String>,
    /// Motif d'un changement de capacité (trace).
    #[serde(default)]
    pub motif: Option<String>,
    /// Unités d'hébergement (offre de régime interne, R-I15).
    #[serde(default)]
    pub unites: Vec<UniteHebergementDto>,
}

/// Unité d'hébergement d'une offre de régime interne (dortoir, chambre adaptée…).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniteHebergementDto {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub libelle: String,
    pub capacite: i32,
    #[serde(default)]
    pub conditions_acces: Option<String>,
    /// Places prises (vœux retenus rattachés à l'unité) — lecture seule.
    #[serde(default)]
    pub occupees: i64,
}

/// Une offre avec son compteur de places (calculé à chaque lecture, jamais saisi).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffreOccupationDto {
    pub id: Uuid,
    pub campagne_id: Uuid,
    pub niveau_code: String,
    pub niveau_id: Option<Uuid>,
    pub parcours_code: Option<String>,
    pub regime: Option<String>,
    pub libelle_public: String,
    pub capacite: Option<i32>,
    pub places_reservees_reinscription: bool,
    pub ouverte_candidature: bool,
    pub conditions_acces: Option<String>,
    /// Candidatures en cours d'instruction (informatif, ne consomme pas de place).
    pub demandes: i64,
    /// Admis en attente de la réponse de la famille.
    pub proposees: i64,
    pub confirmees: i64,
    pub validees: i64,
    /// Sur liste d'attente (ne consomme pas de place).
    pub liste_attente: i64,
    /// Réinscrits qui gardent une place (offre jumelle d'une campagne de réinscription).
    pub reserve_reinscription: i64,
    /// `capacité − réserve − proposées − confirmées − validées` ; négatif = dépassement assumé ;
    /// `None` quand la capacité n'est pas limitée.
    pub disponibles: Option<i64>,
    #[serde(default)]
    pub unites: Vec<UniteHebergementDto>,
}

// ---------------------------------------------------------------------------
// Démarches familiales, fratries, déclarations (lot I2, §8.3 et §8.4)
// ---------------------------------------------------------------------------

/// Un enfant d'une démarche : l'élève du formulaire et ses vœux (offres classées).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnfantDemarcheBody {
    pub eleve: EleveFormulaireDto,
    /// Offres souhaitées, par ordre de préférence (cinq au plus).
    #[serde(default)]
    pub voeux: Vec<Uuid>,
}

/// Responsable supplémentaire (au-delà des deux premiers) : tuteur, beau-parent, ASE…
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutreResponsableBody {
    pub responsable: ResponsableFormulaireDto,
    /// Positions (à partir de 0) des enfants concernés dans la démarche ; vide = tous.
    #[serde(default)]
    pub enfants: Vec<usize>,
}

/// `POST /preinscription/demarches` : une démarche pour un ou plusieurs enfants (fratrie native),
/// une seule déclaration des responsables.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemarcheBody {
    pub campagne_id: Uuid,
    #[serde(default)]
    pub utiliser_mes_infos: bool,
    #[serde(default)]
    pub enregistrer_profil: bool,
    /// Un à six enfants.
    pub enfants: Vec<EnfantDemarcheBody>,
    pub responsable: ResponsableFormulaireDto,
    #[serde(default)]
    pub second_parent: Option<ResponsableFormulaireDto>,
    /// Responsables supplémentaires (quatre au plus).
    #[serde(default)]
    pub autres_responsables: Vec<AutreResponsableBody>,
}

/// Déclaration d'un responsable dans une démarche (sans valeur scellée).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarationResponsableDto {
    pub ordre: i16,
    pub est_deposant: bool,
    pub nom: String,
    pub prenom: Option<String>,
    pub lien_code: String,
    pub autorite_parentale: String,
    pub foyer_cle: String,
    pub pour_enfants: Vec<Uuid>,
    /// Aucune sollicitation (invitation, accord, relance) ne lui est envoyée.
    pub ne_pas_solliciter: bool,
    /// Rempli à la validation seulement (retour de la commande du socle 4).
    pub personne_id: Option<Uuid>,
}

/// Vue d'une démarche : ses dossiers et ses déclarations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemarcheDto {
    pub id: Uuid,
    pub dossiers: Vec<DossierResumeDto>,
    pub declarations: Vec<DeclarationResponsableDto>,
}

// ---------------------------------------------------------------------------
// Transitions, blocage, annulation de validation, ne pas solliciter (lot I2)
// ---------------------------------------------------------------------------

/// `POST /inscriptions/dossiers/{id}/transition` : changements d'état hors instruction et hors
/// décision. `action` : `sans_suite`, `annuler`, `desister`, `reprendre` (depuis « sans suite »),
/// `non_presente`, `reaffecte`, `non_reinscription_etablissement`, `sans_reponse`,
/// `reouvrir` (depuis « sans réponse »), `en_attente_passage`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionBody {
    pub action: String,
    /// Motif interne obligatoire (3 à 1000 caractères).
    pub motif: String,
}

/// `POST /inscriptions/dossiers/{id}/blocage` : poser (désaccord des responsables) ou lever.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlocageBody {
    /// `true` : poser le blocage ; `false` : le lever (retour à l'état d'avant).
    pub bloquer: bool,
    pub motif: String,
}

/// `POST /inscriptions/dossiers/{id}/annuler-validation` (INS-68 ★).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnulationValidationBody {
    pub motif: String,
}

/// Résultat d'une annulation de validation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnulationValidationDto {
    pub dossier_id: Uuid,
    pub etat: String,
    /// Fiche créée par la validation, archivée (ELV-02) ; `false` si l'élève existait déjà.
    pub eleve_archive: bool,
    pub inscription_annuelle_retiree: bool,
    pub liens_suspendus: i64,
    pub invitations_revoquees: i64,
}

/// `POST /inscriptions/dossiers/{id}/ne-pas-solliciter` (INS-57 ★, direction).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NePasSolliciterBody {
    /// Rang (1 à 6) du responsable concerné dans la démarche.
    pub rang: i16,
    /// Motif scellé (danger, ordonnance de protection, décision de justice) : jamais renvoyé.
    pub motif: String,
}

/// Résultat : nombre d'envois, d'invitations et de demandes retirés.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NePasSolliciterDto {
    pub envois_annules: i64,
    pub invitations_annulees: i64,
}

// ---------------------------------------------------------------------------
// Échéances et fermetures (INS-79, INS-80)
// ---------------------------------------------------------------------------

/// `POST /inscriptions/campagnes/{id}/prolonger` (INS-79 ★).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProlongationBody {
    /// `"date_limite_famille"`, `"cloture"` ou `"echeance_dossiers"`.
    pub objet: String,
    pub nouvelle_date: NaiveDate,
    /// Motif court, communiqué aux familles.
    pub motif_public: String,
    /// `echeance_dossiers` : sélection de dossiers ; vide = tous les dossiers ouverts.
    #[serde(default)]
    pub dossiers: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProlongationDto {
    pub id: Uuid,
    pub objet: String,
    pub ancienne_date: Option<NaiveDate>,
    pub nouvelle_date: NaiveDate,
    pub nb_dossiers: i64,
    pub familles_prevenues: i64,
}

/// `POST /inscriptions/fermetures` (INS-80).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FermetureBody {
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    /// Jours après la réouverture avant lesquels une famille n'est ni échue ni relancée.
    #[serde(default)]
    pub report_jours: Option<i16>,
    pub motif_public: String,
    #[serde(default)]
    pub contact_urgence: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FermetureDto {
    pub id: Uuid,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub report_jours: i16,
    pub motif_public: String,
    pub contact_urgence: Option<String>,
    pub echeances_reportees: i64,
}
