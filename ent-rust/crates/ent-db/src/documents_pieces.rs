//! Socle 7, lot D5 : **pièces justificatives** — catalogue, demandes, dépôt (photos → PDF, PDF désarmé, quarantaine,
//! antivirus), contrôle en rafale, réutilisation, « ce qui manque », port de reprise
//! (`docs/socles/07_DOCUMENTS.md` §11, §17, §19.1 n° 7 à 10, §21).
//!
//! Règles du lot, toutes appliquées ici (et, pour les plus graves, redoublées par la base : migration 3704) :
//! * **demander le minimum** : le catalogue est **livré par ScolIA** et fermé ; le type « autre » exige un motif affiché
//!   à la famille et refuse les pièces qu'aucun établissement ne peut demander (titre de séjour, carte Vitale, casier
//!   judiciaire, document religieux, photo d'un domicile…) ;
//! * **ne jamais redemander ce qu'on a** : avant toute demande, le service cherche une pièce **valable** au dossier (même
//!   sujet, ou même foyer pour une pièce de portée « foyer ») et la **propose** ; la famille ou l'agent confirme
//!   (`reutiliser_piece_for_actor`), rien n'est réutilisé en silence ;
//! * **quarantaine** : tout fichier déposé est d'abord **hors d'atteinte** (`en_quarantaine`, aucun fichier servi, aucun
//!   lien possible) ; l'analyse se fait **ensuite** (`analyser_piece`) : antivirus (client ClamAV maison, `ent-object`),
//!   puis conversion (photos assemblées en un seul PDF sans métadonnées ; PDF désarmé : `ent-pdf::piece`). **Antivirus
//!   indisponible → la pièce reste en quarantaine** (fermé par défaut) ; infecté → `infectee`, jamais servie ; forme
//!   refusée → `rejetee` ;
//! * **un humain décide** : accepter, refuser avec motif type + commentaire, ou « sans objet » ; contrôle **en rafale** ;
//!   l'accueil ne contrôle que les pièces d'identité ; les types C3 exigent en plus le droit de leur domaine ;
//! * **ce qu'on ne garde pas** : une pièce d'identité n'est **vue** que le temps du contrôle (`copie_gardee = false`) :
//!   ses fichiers sont marqués supprimés dans le registre des objets dès la décision ;
//! * **une pièce déposée par erreur** est retirée par son déposant tant qu'elle n'est pas contrôlée ;
//! * **jamais l'original** : seul le fichier servi (désarmé) sort, après quarantaine levée, à qui a le droit ; chaque
//!   lecture laisse une ligne au journal.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::{Datelike, Months, NaiveDate};
use ent_auth::{Action, Role};
use ent_events::MetierEvent;
use ent_object::upload::{AnalyseurAntivirus, ContexteAnalyse, TypeFichier, UsageFichier, VerdictAnalyse};
use ent_pdf::piece::{desarmer_pdf, images_en_pdf, ErreurPiece};
use serde_json::json;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::documents::{Domaine, Sensibilite};
use crate::documents_coffre::{jour_etab, liens_autorite, majeurs_parmi, resoudre_titulaires};
use crate::documents_repo::{journaliser, publier, texte};
use crate::documents_zip::{lire_zip_stored, EcrivainZip};
use crate::ref_commun::{db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

pub const EVT_PIECE_DEMANDEE: &str = "documents.piece_demandee";
pub const EVT_PIECE_DEPOSEE: &str = "documents.depose";
pub const EVT_PIECE_REFUSEE_ANALYSE: &str = "documents.refuse_analyse";
pub const EVT_PIECE_DECIDEE: &str = "documents.piece_decidee";

/// Taille maximale d'un fichier déposé, et de l'ensemble d'un dépôt.
pub const FICHIER_MAX: usize = 15 * 1024 * 1024;
pub const DEPOT_MAX: usize = 45 * 1024 * 1024;
/// Dépôts par compte et par 24 h (anti-abus : une famille qui dépose en rafale n'inonde pas le secrétariat).
pub const DEPOTS_PAR_JOUR: i64 = 40;
/// Contrôles par appel en rafale.
pub const RAFALE_MAX: usize = 100;

fn refus(nature: RefusMetier, msg: &'static str) -> EntDbError {
    EntDbError::Refus(nature, msg)
}

// ───────────────────────── Catalogue (§11.2) ─────────────────────────

/// Ce que couvre une pièce : l'élève seul, tout son foyer (les frères et sœurs de même foyer), une personne, un agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortePiece {
    Foyer,
    Eleve,
    Personne,
    Agent,
}

impl PortePiece {
    pub fn code(self) -> &'static str {
        match self {
            Self::Foyer => "foyer",
            Self::Eleve => "eleve",
            Self::Personne => "personne",
            Self::Agent => "agent",
        }
    }
}

/// Durée de validité à partir de la décision d'acceptation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Validite {
    Permanente,
    /// N mois (justificatif de domicile : 3).
    Mois(u32),
    /// Jusqu'au 31 août.
    AnneeScolaire,
    /// Jusqu'au 31 décembre.
    AnneeCivile,
    /// Jusqu'à nouvelle décision (jugement) : pas de date.
    JusquaDecision,
    /// La date vient de la pièce (certificat médical, notification MDPH) : le contrôleur la saisit.
    Variable,
}

#[derive(Debug, Clone, Copy)]
pub struct PieceTypeDecl {
    pub code: &'static str,
    pub libelle: &'static str,
    pub portee: PortePiece,
    pub validite: Validite,
    pub sensibilite: Sensibilite,
    pub domaine: Domaine,
    /// `false` : vue seulement, jamais conservée (pièce d'identité, §8.6).
    pub copie_gardee: bool,
    /// Conseil affiché au dépôt.
    pub conseil: &'static str,
    /// Peut être demandée par un agent ou une brique (le type technique de la reprise ne l'est pas).
    pub demandable: bool,
    /// L'accueil peut contrôler ce type (identité seulement, §18.2 n° 15).
    pub accueil_controle: bool,
}

const fn t(
    code: &'static str,
    libelle: &'static str,
    portee: PortePiece,
    validite: Validite,
    sensibilite: Sensibilite,
    domaine: Domaine,
    conseil: &'static str,
) -> PieceTypeDecl {
    PieceTypeDecl { code, libelle, portee, validite, sensibilite, domaine, copie_gardee: true, conseil, demandable: true, accueil_controle: false }
}

use Domaine as D;
use PortePiece as P;
use Sensibilite::{C1, C2, C3};
use Validite as V;

/// Le catalogue des pièces que ScolIA sait demander. **Fermé** : un établissement ne peut pas y ajouter ce qui est interdit
/// (§11.2), il active ou non les types et peut utiliser « autre » (motivé).
pub const CATALOGUE: &[PieceTypeDecl] = &[
    t("justificatif_domicile", "Justificatif de domicile", P::Foyer, V::Mois(3), C2, D::Scolarite,
      "Facture d'énergie, d'eau, de téléphone fixe, quittance de loyer ou avis d'imposition."),
    t("livret_famille", "Livret de famille (pages utiles)", P::Foyer, V::Permanente, C2, D::Scolarite,
      "Seules les pages des parents et de l'enfant concerné."),
    t("acte_naissance", "Acte de naissance", P::Eleve, V::Permanente, C2, D::Scolarite, ""),
    t("decision_justice", "Décision de justice (autorité, résidence, protection)", P::Eleve, V::JusquaDecision, C3, D::Justice,
      "La première page et le dispositif suffisent."),
    PieceTypeDecl {
        copie_gardee: false,
        accueil_controle: true,
        ..t("piece_identite", "Pièce d'identité", P::Personne, V::Permanente, C2, D::Scolarite,
            "Elle n'est vue que le temps du contrôle : ScolIA n'en garde pas de copie.")
    },
    t("attestation_assurance", "Attestation d'assurance scolaire", P::Eleve, V::AnneeScolaire, C2, D::Scolarite, ""),
    t("photo_identite", "Photo d'identité", P::Eleve, V::Mois(12), C2, D::Scolarite,
      "Le consentement à l'affichage se donne séparément."),
    t("exeat", "Certificat de radiation de l'établissement d'origine (exeat)", P::Eleve, V::Permanente, C2, D::Scolarite, ""),
    t("bulletins_anterieurs", "Bulletins ou dossier scolaire antérieurs", P::Eleve, V::Permanente, C2, D::Scolarite, ""),
    t("quotient_familial", "Attestation de quotient familial ou de ressources", P::Foyer, V::AnneeCivile, C2, D::Financier, ""),
    t("avis_imposition", "Avis d'imposition (bourses)", P::Foyer, V::AnneeCivile, C2, D::Financier,
      "Seule la première page est nécessaire."),
    t("certificat_medical", "Certificat médical, ordonnance, protocole", P::Eleve, V::Variable, C3, D::Sante,
      "Proposé seulement si le module santé est actif."),
    t("notification_mdph", "Notification MDPH, plan d'accompagnement", P::Eleve, V::Variable, C3, D::Sante, ""),
    t("attestation_rc_entreprise", "Attestation de responsabilité civile de l'entreprise", P::Eleve, V::Variable, C1, D::Scolarite, ""),
    t("piece_rh", "Pièce RH (diplôme, contrat, attestation)", P::Agent, V::Variable, C2, D::Rh, ""),
    t("autorisation_travail", "Autorisation de travail d'un salarié étranger", P::Agent, V::Variable, C2, D::Rh, ""),
    t("attestation_hebergement", "Attestation d'hébergement", P::Foyer, V::Mois(3), C2, D::Scolarite,
      "Pas de pièce d'identité de l'hébergeant."),
    t("attestation_domiciliation", "Attestation de domiciliation (organisme agréé)", P::Foyer, V::Mois(12), C2, D::Scolarite, ""),
    t("justificatif_absence", "Justificatif d'absence", P::Eleve, V::AnneeScolaire, C2, D::Scolarite, "Le motif suffit."),
    t("certificat_mairie", "Certificat d'inscription délivré par la mairie", P::Eleve, V::Permanente, C2, D::Scolarite, ""),
    // « Autre » : motif obligatoire, jamais pour ce que le catalogue interdit.
    t("autre", "Autre pièce (motif exigé)", P::Eleve, V::Variable, C2, D::Scolarite,
      "Le motif est affiché à la famille et apparaît dans la revue du DPO."),
    // Type technique de la reprise des demandes (N1-5) : pièces jointes d'une demande de l'ancienne application.
    PieceTypeDecl {
        demandable: false,
        ..t("piece_de_demande", "Pièce jointe d'une demande (reprise)", P::Personne, V::Permanente, C2, D::Scolarite, "")
    },
];

pub fn type_piece(code: &str) -> Option<&'static PieceTypeDecl> {
    CATALOGUE.iter().find(|t| t.code == code)
}

/// Mots qui, dans le motif d'une pièce « autre », signalent une pièce qu'aucun établissement ne peut demander (§11.2).
/// Comparaison sans accents ni majuscules.
const INTERDITS: &[&str] = &[
    "titre de sejour", "carte de sejour", "visa", "regularite", "situation administrative", "carte vitale",
    "securite sociale", "releve d'identite bancaire", "rib", "iban", "casier judiciaire", "bulletin n3",
    "religi", "baptem", "culte", "photo du domicile", "photos du domicile", "photo de la maison",
];

fn sans_accents(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'À' | 'Â' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' => 'e',
            'î' | 'ï' | 'Î' => 'i',
            'ô' | 'ö' | 'Ô' => 'o',
            'ù' | 'û' | 'ü' | 'Ù' => 'u',
            'ç' | 'Ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// Le motif d'une pièce « autre » demande-t-il quelque chose que le catalogue interdit ?
pub fn motif_interdit(motif: &str) -> bool {
    let m = sans_accents(motif);
    INTERDITS.iter().any(|i| {
        if i.len() <= 3 {
            // Mots courts (« rib », « visa ») : mot entier seulement.
            m.split(|c: char| !c.is_alphanumeric()).any(|w| w == *i)
        } else {
            m.contains(i)
        }
    })
}

/// Date de fin de validité d'une pièce acceptée le `jour` (§11.2). `fournie` : date lue sur la pièce par le contrôleur.
pub fn fin_de_validite(
    decl: &PieceTypeDecl,
    jour: NaiveDate,
    fournie: Option<NaiveDate>,
) -> Result<Option<NaiveDate>, EntDbError> {
    if fournie.is_some_and(|f| f < jour) {
        return Err(refus(RefusMetier::Conflit, "la date de validité est déjà passée : refuser la pièce (périmée)"));
    }
    let calculee = match decl.validite {
        Validite::Permanente | Validite::JusquaDecision => None,
        Validite::Mois(n) => jour.checked_add_months(Months::new(n)),
        Validite::AnneeScolaire => {
            let a = if jour.month() >= 9 { jour.year() + 1 } else { jour.year() };
            NaiveDate::from_ymd_opt(a, 8, 31)
        }
        Validite::AnneeCivile => NaiveDate::from_ymd_opt(jour.year(), 12, 31),
        Validite::Variable => {
            if fournie.is_none() {
                return Err(refus(RefusMetier::Invalide, "la date de validité de cette pièce est à saisir"));
            }
            None
        }
    };
    Ok(match (calculee, fournie) {
        (Some(c), Some(f)) => Some(c.min(f)),
        (Some(c), None) => Some(c),
        (None, f) => f,
    })
}

// ───────────────────────── Types publiés ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SujetPiece {
    Eleve(Uuid),
    Personne(Uuid),
    Agent(Uuid),
}

impl SujetPiece {
    fn colonnes(self) -> (Option<Uuid>, Option<Uuid>, Option<Uuid>) {
        match self {
            Self::Eleve(e) => (Some(e), None, None),
            Self::Personne(p) => (None, Some(p), None),
            Self::Agent(g) => (None, None, Some(g)),
        }
    }

    fn eleve(self) -> Option<Uuid> {
        match self {
            Self::Eleve(e) => Some(e),
            _ => None,
        }
    }

    /// Le sujet convient-il à la portée du type ? (« autre » : élève, personne ou agent.)
    fn convient(self, decl: &PieceTypeDecl) -> bool {
        if decl.code == "autre" {
            return true;
        }
        matches!(
            (decl.portee, self),
            (PortePiece::Foyer | PortePiece::Eleve, Self::Eleve(_))
                | (PortePiece::Personne, Self::Personne(_))
                | (PortePiece::Agent, Self::Agent(_))
        )
    }
}

#[derive(Debug, Clone)]
pub struct DemandePiece {
    pub type_code: String,
    pub sujet: SujetPiece,
    pub motif_affiche: String,
    pub echeance: Option<NaiveDate>,
    pub obligatoire: bool,
    /// La brique qui demande (`inscriptions`, `voyages`, `bourses`…).
    pub brique: String,
    pub reference_externe: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct DemandeCreee {
    pub demande_id: Uuid,
    /// Une demande ouverte existait déjà pour ce type et ce sujet : rendue telle quelle.
    pub deja_ouverte: bool,
    /// Une pièce **valable** est déjà au dossier : à proposer, jamais à utiliser en silence.
    pub proposition_piece_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct DepotPiece {
    pub type_code: String,
    pub sujet: SujetPiece,
    pub demande_id: Option<Uuid>,
    /// Exigé pour le type « autre », affiché à la famille.
    pub motif_autre: Option<String>,
    /// Un PDF, **ou** une à cinquante photos (JPEG, PNG) dans l'ordre des pages.
    pub fichiers: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieceDeposee {
    pub piece_id: Uuid,
    pub etat: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultatAnalyse {
    /// Pièce reçue, en attente de contrôle humain.
    Recue,
    /// L'antivirus n'a pas répondu : la pièce reste en quarantaine, on réessaiera.
    EnQuarantaine,
    Infectee,
    Rejetee(String),
    /// Déjà analysée (ou introuvable) : rien à faire.
    DejaTraitee,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RapportAnalyse {
    pub recues: usize,
    pub en_quarantaine: usize,
    pub infectees: usize,
    pub rejetees: usize,
}

#[derive(Debug, Clone)]
pub enum ControleDecision {
    /// `valable_jusqu_au` : date lue sur la pièce (obligatoire pour les types à validité variable).
    Accepter { valable_jusqu_au: Option<NaiveDate> },
    /// Motif type : `illisible`, `incomplet`, `perime`, `ne_correspond_pas`, `mauvais_document`, `autre`.
    Refuser { motif: String, commentaire: Option<String> },
    SansObjet { commentaire: String },
}

#[derive(Debug, Clone)]
pub struct ResultatControle {
    pub piece_id: Uuid,
    pub ok: bool,
    pub erreur: Option<String>,
}

/// Une pièce à contrôler, avec ce que la démarche attend et ce que le dossier dit (§11.5).
#[derive(Debug, Clone)]
pub struct PieceAControler {
    pub piece_id: Uuid,
    pub type_code: String,
    pub libelle: String,
    pub sujet_nom: String,
    pub deposee_le: chrono::DateTime<chrono::Utc>,
    pub nb_pages: i32,
    /// Ce que le désarmement a neutralisé (information du contrôleur).
    pub desarmement: Option<String>,
    pub demande_id: Option<Uuid>,
    /// « Ce que la démarche attend » : le motif affiché de la demande.
    pub attendu: Option<String>,
    pub motif_autre: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatManque {
    AFournir,
    EnAnalyse,
    RecueEnAttente,
    Refusee,
    /// Acceptée, mais sa validité est passée : à renouveler.
    Perimee,
}

#[derive(Debug, Clone)]
pub struct LigneManque {
    pub demande_id: Uuid,
    pub type_code: String,
    pub libelle: String,
    pub obligatoire: bool,
    pub echeance: Option<NaiveDate>,
    pub motif_affiche: String,
    pub etat: EtatManque,
    pub piece_id: Option<Uuid>,
    /// Motif type du refus, ou `analyse` quand le fichier a été refusé à l'analyse.
    pub refus_motif: Option<String>,
    /// Une pièce valable existe déjà au dossier : à proposer.
    pub proposition_piece_id: Option<Uuid>,
    pub perimee_le: Option<NaiveDate>,
    pub conseil: &'static str,
}

#[derive(Debug, Clone)]
pub struct PieceLue {
    pub piece_id: Uuid,
    pub type_code: String,
    pub etat: String,
    pub nb_pages: i32,
    pub empreinte_sha256: String,
    pub octets: Vec<u8>,
}

// ───────────────────────── Aides ─────────────────────────

fn type_de_fichier(b: &[u8]) -> Result<TypeFichier, EntDbError> {
    if b.starts_with(b"%PDF-") {
        Ok(TypeFichier::Pdf)
    } else if b.starts_with(&[0xff, 0xd8, 0xff]) {
        Ok(TypeFichier::Jpeg)
    } else if b.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Ok(TypeFichier::Png)
    } else {
        Err(refus(
            RefusMetier::Invalide,
            "format non accepté : déposez un PDF ou une photo (JPEG, PNG)",
        ))
    }
}

fn message_erreur_piece(e: &ErreurPiece) -> String {
    e.to_string().chars().take(200).collect()
}

/// Les élèves du même foyer que `eleve` (liens portant le même `foyer_id`), l'élève compris.
async fn eleves_du_foyer(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<Vec<Uuid>, EntDbError> {
    let l: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT l2.eleve_id FROM ent_eleve_lien l1
           JOIN ent_eleve_lien l2 ON l2.etablissement_id = l1.etablissement_id AND l2.foyer_id = l1.foyer_id
          WHERE l1.etablissement_id = $1 AND l1.eleve_id = $2 AND l1.foyer_id IS NOT NULL
            AND l1.suspendu_le IS NULL AND l2.suspendu_le IS NULL",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    let mut v: Vec<Uuid> = l.into_iter().map(|x| x.0).collect();
    if !v.contains(&eleve) {
        v.push(eleve);
    }
    Ok(v)
}

/// Une pièce **valable** déjà au dossier du sujet (ou de son foyer) : acceptée, non périmée.
async fn piece_valable(
    conn: &mut PgConnection,
    etab: Uuid,
    decl: &PieceTypeDecl,
    sujet: SujetPiece,
    jour: NaiveDate,
) -> Result<Option<Uuid>, EntDbError> {
    let eleves: Vec<Uuid> = match (decl.portee, sujet) {
        (PortePiece::Foyer, SujetPiece::Eleve(e)) => eleves_du_foyer(&mut *conn, etab, e).await?,
        (_, SujetPiece::Eleve(e)) => vec![e],
        _ => Vec::new(),
    };
    let (_, p, g) = sujet.colonnes();
    let r: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_doc_piece
          WHERE etablissement_id = $1 AND type_code = $2 AND etat = 'acceptee'
            AND (valable_jusqu_au IS NULL OR valable_jusqu_au >= $3)
            AND (eleve_id = ANY($4) OR personne_id = $5 OR agent_id = $6)
          ORDER BY controle_le DESC, id LIMIT 1",
    )
    .bind(etab)
    .bind(decl.code)
    .bind(jour)
    .bind(&eleves)
    .bind(p)
    .bind(g)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.map(|x| x.0))
}

async fn decision(
    conn: &mut PgConnection,
    etab: Uuid,
    piece: Uuid,
    demande: Option<Uuid>,
    decision: &str,
    motif: Option<&str>,
    commentaire: Option<&str>,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_doc_piece_decision (id, etablissement_id, piece_id, demande_id, decision, motif_code, commentaire, acteur_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(piece)
    .bind(demande)
    .bind(decision)
    .bind(motif)
    .bind(commentaire)
    .bind(acteur)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct LignePiece {
    id: Uuid,
    type_code: String,
    eleve_id: Option<Uuid>,
    personne_id: Option<Uuid>,
    agent_id: Option<Uuid>,
    demande_id: Option<Uuid>,
    deposant_user_id: Uuid,
    etat: String,
    cle_objet_original: String,
    sha256_original: String,
    mime_original: String,
    nb_fichiers_origine: i32,
    cle_objet_servi: Option<String>,
    sha256_servi: Option<String>,
    nb_pages: Option<i32>,
    analyse_tentatives: i32,
}

const COLONNES_PIECE: &str = "id, type_code, eleve_id, personne_id, agent_id, demande_id, deposant_user_id, etat, cle_objet_original,
    sha256_original, mime_original, nb_fichiers_origine, cle_objet_servi, sha256_servi, nb_pages, analyse_tentatives";

impl LignePiece {
    fn sujet(&self) -> Option<SujetPiece> {
        match (self.eleve_id, self.personne_id, self.agent_id) {
            (Some(e), _, _) => Some(SujetPiece::Eleve(e)),
            (_, Some(p), _) => Some(SujetPiece::Personne(p)),
            (_, _, Some(g)) => Some(SujetPiece::Agent(g)),
            _ => None,
        }
    }
}

async fn piece_par_id(conn: &mut PgConnection, etab: Uuid, id: Uuid, verrou: bool) -> Result<LignePiece, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES_PIECE} FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as::<_, LignePiece>(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

fn systeme(etab: Uuid) -> Acteur<'static> {
    Acteur { etablissement_id: etab, user_id: Uuid::nil(), role: "systeme" }
}

// ───────────────────────── Service ─────────────────────────

impl PgEnt {
    /// Le droit **de domaine** d'un type de pièce (C3 : santé, justice ; financier ; RH) en plus du droit de la fonction.
    async fn droit_du_type(&self, a: &Acteur<'_>, decl: &PieceTypeDecl, eleve: Option<Uuid>) -> Result<bool, EntDbError> {
        if decl.sensibilite >= Sensibilite::C3 || matches!(decl.domaine, Domaine::Financier | Domaine::Rh) {
            return self.doc_peut_sur_sujet(a, decl.domaine.action_lecture(), eleve).await;
        }
        Ok(true)
    }

    /// Peut-on **contrôler** cette pièce ? `documents.pieces.controler`, l'accueil pour l'identité seulement, le droit du
    /// domaine pour les types sensibles, le droit sur l'élève.
    async fn peut_controler(&self, a: &Acteur<'_>, decl: &PieceTypeDecl, eleve: Option<Uuid>) -> Result<bool, EntDbError> {
        if Role::parse(a.role).is_none_or(|r| !r.is_staff()) {
            return Ok(false);
        }
        if a.role == "accueil" && !decl.accueil_controle {
            return Ok(false);
        }
        let ok = match eleve {
            Some(e) => {
                self.autoriser_eleve(a.etablissement_id, a.user_id, a.role, Action::DocumentsPiecesControler, e).await?
            }
            None => {
                self.a_action_etablissement(a.etablissement_id, a.user_id, a.role, Action::DocumentsPiecesControler)
                    .await?
            }
        };
        Ok(ok && self.droit_du_type(a, decl, eleve).await?)
    }

    /// La famille (ou l'élève majeur, ou la personne elle-même) a-t-elle la main sur ce sujet ? Titulaire de l'autorité
    /// parentale au lien actif et non suspendu ; l'élève majeur pour lui-même ; la personne pour elle-même.
    async fn famille_a_la_main(
        &self,
        conn: &mut PgConnection,
        a: &Acteur<'_>,
        sujet: SujetPiece,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let t = resoudre_titulaires(&mut *conn, a.etablissement_id, a.user_id).await?;
        Ok(match sujet {
            SujetPiece::Personne(p) => t.personnes.contains(&p),
            SujetPiece::Agent(g) => t.agents.contains(&g),
            SujetPiece::Eleve(e) => {
                if t.eleves.contains(&e) {
                    majeurs_parmi(&mut *conn, a.etablissement_id, &[e], jour).await?.contains(&e)
                } else {
                    liens_autorite(&mut *conn, a.etablissement_id, &t.personnes, jour)
                        .await?
                        .iter()
                        .any(|(_, el)| *el == e)
                }
            }
        })
    }

    // ───────────── Demander ─────────────

    /// **Demande** une pièce (`demander_pieces`, §11.3) : droit `documents.pieces.demander` (et celui du domaine pour un
    /// type sensible), type du catalogue, sujet cohérent avec sa portée. Idempotent : une demande ouverte existante est
    /// rendue telle quelle. Cherche d'abord une pièce **valable** au dossier et la **propose**.
    pub async fn demander_piece_for_actor(&self, a: &Acteur<'_>, d: DemandePiece) -> Result<DemandeCreee, EntDbError> {
        let decl = type_piece(&d.type_code).filter(|t| t.demandable).ok_or_else(invalide)?;
        let motif = texte(&d.motif_affiche, 3, 300)?;
        let brique = d.brique.trim().to_string();
        if brique.is_empty() || brique.len() > 40 || !brique.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(invalide());
        }
        if !d.sujet.convient(decl) {
            return Err(refus(RefusMetier::Conflit, "ce type de pièce ne se demande pas pour ce sujet"));
        }
        if decl.code == "autre" && motif_interdit(&motif) {
            return Err(refus(RefusMetier::Conflit, "cette pièce ne peut être demandée par aucun établissement"));
        }
        self.exiger_action_documents(a, Action::DocumentsPiecesDemander).await?;
        if !self.doc_peut_sur_sujet(a, Action::DocumentsPiecesDemander, d.sujet.eleve()).await?
            || !self.droit_du_type(a, decl, d.sujet.eleve()).await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        let (e, p, g) = d.sujet.colonnes();
        if decl.code != "autre" {
            let existante: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_doc_piece_demande
                  WHERE etablissement_id = $1 AND type_code = $2 AND etat = 'ouverte'
                    AND COALESCE(eleve_id, personne_id, agent_id) = COALESCE($3, $4, $5)",
            )
            .bind(a.etablissement_id)
            .bind(decl.code)
            .bind(e)
            .bind(p)
            .bind(g)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((id,)) = existante {
                let prop = piece_valable(&mut tx, a.etablissement_id, decl, d.sujet, jour).await?;
                tx.commit().await.map_err(db)?;
                return Ok(DemandeCreee { demande_id: id, deja_ouverte: true, proposition_piece_id: prop });
            }
        }
        let proposition = piece_valable(&mut tx, a.etablissement_id, decl, d.sujet, jour).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_doc_piece_demande
                 (id, etablissement_id, type_code, eleve_id, personne_id, agent_id, portee, motif_affiche, echeance,
                  obligatoire, brique_demandeuse, reference_externe, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(decl.code)
        .bind(e)
        .bind(p)
        .bind(g)
        .bind(decl.portee.code())
        .bind(&motif)
        .bind(d.echeance)
        .bind(d.obligatoire)
        .bind(&brique)
        .bind(d.reference_externe)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, a, None, None, "piece_demandee", (None, None), Some(decl.code), None).await?;
        let ev = publier(
            &mut tx,
            a,
            EVT_PIECE_DEMANDEE,
            id,
            d.sujet.eleve(),
            json!({ "type": decl.code, "brique": brique, "propose": proposition.is_some() }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(DemandeCreee { demande_id: id, deja_ouverte: false, proposition_piece_id: proposition })
    }

    /// Annule une demande ouverte (motif obligatoire).
    pub async fn annuler_demande_piece_for_actor(
        &self,
        a: &Acteur<'_>,
        demande_id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsPiecesDemander).await?;
        let motif = texte(motif, 3, 500)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let r: Option<(String, Option<Uuid>)> = sqlx::query_as(
            "UPDATE ent_doc_piece_demande SET etat = 'annulee', cloturee_le = now(), cloture_motif = $3
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'ouverte' RETURNING type_code, eleve_id",
        )
        .bind(a.etablissement_id)
        .bind(demande_id)
        .bind(&motif)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((type_code, eleve)) = r else {
            return Err(refus(RefusMetier::Conflit, "demande introuvable ou déjà close"));
        };
        let decl = type_piece(&type_code);
        if let Some(decl) = decl {
            if !self.doc_peut_sur_sujet(a, Action::DocumentsPiecesDemander, eleve).await?
                || !self.droit_du_type(a, decl, eleve).await?
            {
                return Err(EntDbError::Forbidden);
            }
        }
        journaliser(&mut tx, a, None, None, "piece_demande_annulee", (None, None), Some(&type_code), Some(&motif)).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────── Déposer ─────────────

    /// **Dépose** une pièce (§11.3). La famille : titulaire de l'autorité au lien actif, élève majeur, personne pour
    /// elle-même. L'agent : `documents.deposer` sur l'élève (guichet, scan). Contrôle de forme **à l'entrée** (type réel
    /// par signature, taille, nombre) puis **quarantaine** : rien n'est servi, l'analyse suit (`analyser_piece`).
    pub async fn deposer_piece_for_actor(&self, a: &Acteur<'_>, d: DepotPiece) -> Result<PieceDeposee, EntDbError> {
        let decl = type_piece(&d.type_code).ok_or_else(invalide)?;
        if !d.sujet.convient(decl) {
            return Err(refus(RefusMetier::Conflit, "ce type de pièce ne se dépose pas pour ce sujet"));
        }
        let motif_autre = match (decl.code == "autre", d.motif_autre.as_deref()) {
            (true, Some(m)) => {
                let m = texte(m, 3, 300)?;
                if motif_interdit(&m) {
                    return Err(refus(RefusMetier::Conflit, "cette pièce ne peut être demandée ni conservée par un établissement"));
                }
                Some(m)
            }
            (true, None) => return Err(refus(RefusMetier::Invalide, "le motif est obligatoire pour une autre pièce")),
            (false, _) => None,
        };
        if d.fichiers.is_empty() || d.fichiers.len() > ent_pdf::piece::PAGES_MAX {
            return Err(refus(RefusMetier::Invalide, "un PDF, ou de une à cinquante photos"));
        }
        let mut total = 0usize;
        let mut types = Vec::new();
        for f in &d.fichiers {
            if f.is_empty() || f.len() > FICHIER_MAX {
                return Err(refus(RefusMetier::Invalide, "fichier vide ou trop volumineux (15 Mio au plus)"));
            }
            total += f.len();
            types.push(type_de_fichier(f)?);
        }
        if total > DEPOT_MAX {
            return Err(refus(RefusMetier::Invalide, "dépôt trop volumineux (45 Mio au plus)"));
        }
        if types.contains(&TypeFichier::Pdf) && d.fichiers.len() > 1 {
            return Err(refus(RefusMetier::Invalide, "un seul PDF à la fois, ou plusieurs photos : pas les deux"));
        }
        // Original scellé : le fichier tel quel, ou l'archive des photos reçues.
        let (original, mime) = if d.fichiers.len() == 1 {
            (
                d.fichiers[0].clone(),
                match types[0] {
                    TypeFichier::Pdf => "application/pdf",
                    TypeFichier::Jpeg => "image/jpeg",
                    _ => "image/png",
                },
            )
        } else {
            let mut z = EcrivainZip::nouveau();
            for (i, (f, t)) in d.fichiers.iter().zip(&types).enumerate() {
                let ext = if *t == TypeFichier::Jpeg { "jpg" } else { "png" };
                z.ajouter(&format!("{:03}.{ext}", i + 1), f).map_err(|_| invalide())?;
            }
            (z.terminer(), "application/zip")
        };
        let staff = Role::parse(a.role).is_some_and(|r| r.is_staff());
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        if staff {
            match d.sujet.eleve() {
                Some(e) => {
                    if !self.doc_peut_sur_sujet(a, Action::DocumentsDeposer, Some(e)).await? {
                        return Err(EntDbError::Forbidden);
                    }
                }
                None => {
                    self.exiger_action_documents(a, Action::DocumentsDeposer).await?;
                }
            }
            if !self.droit_du_type(a, decl, d.sujet.eleve()).await? {
                return Err(EntDbError::Forbidden);
            }
        } else if !self.famille_a_la_main(&mut tx, a, d.sujet, jour).await? {
            return Err(EntDbError::Forbidden);
        }
        // Le dépôt répond à une demande ouverte de même type et de même sujet (ou de son foyer).
        if let Some(did) = d.demande_id {
            let dem: Option<(String, Option<Uuid>, Option<Uuid>, Option<Uuid>, String)> = sqlx::query_as(
                "SELECT type_code, eleve_id, personne_id, agent_id, etat FROM ent_doc_piece_demande
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(did)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some((tc, de, dp, dg, etat)) = dem else { return Err(introuvable()) };
            let ok_sujet = match d.sujet {
                SujetPiece::Eleve(e) => {
                    de == Some(e) || (decl.portee == PortePiece::Foyer && de.is_some() && eleves_du_foyer(&mut tx, a.etablissement_id, e).await?.contains(&de.unwrap_or_default()))
                }
                SujetPiece::Personne(p) => dp == Some(p),
                SujetPiece::Agent(g) => dg == Some(g),
            };
            if tc != decl.code || !ok_sujet || etat != "ouverte" {
                return Err(refus(RefusMetier::Conflit, "cette demande ne correspond pas à ce dépôt ou n'est plus ouverte"));
            }
        }
        let (recents,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_doc_piece WHERE etablissement_id = $1 AND deposant_user_id = $2 AND deposee_le > now() - interval '24 hours'",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if recents >= DEPOTS_PAR_JOUR {
            return Err(refus(RefusMetier::Conflit, "trop de dépôts aujourd'hui : réessayez demain ou rendez-vous au secrétariat"));
        }
        let id = Uuid::new_v4();
        let ext = match mime {
            "application/pdf" => "pdf",
            "image/jpeg" => "jpg",
            "image/png" => "png",
            _ => "zip",
        };
        let cle = ent_object::ObjectStore::key_for(a.etablissement_id, "pieces-quarantaine", id, ext);
        let info = self.objet_put(&cle, &original).await?;
        crate::objets::enregistrer_objet(&mut *tx, a.etablissement_id, &info, Some(mime)).await?;
        let (e, p, g) = d.sujet.colonnes();
        sqlx::query(
            "INSERT INTO ent_doc_piece
                 (id, etablissement_id, type_code, eleve_id, personne_id, agent_id, demande_id, motif_autre,
                  deposant_user_id, deposant_role, cle_objet_original, sha256_original, taille_originale, mime_original,
                  nb_fichiers_origine)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(decl.code)
        .bind(e)
        .bind(p)
        .bind(g)
        .bind(d.demande_id)
        .bind(&motif_autre)
        .bind(a.user_id)
        .bind(a.role)
        .bind(&info.key)
        .bind(&info.sha256)
        .bind(info.taille as i64)
        .bind(mime)
        .bind(d.fichiers.len() as i32)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        decision(&mut tx, a.etablissement_id, id, d.demande_id, "deposee", None, None, a.user_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(PieceDeposee { piece_id: id, etat: "en_quarantaine".into() })
    }

    /// Retire une pièce **déposée par erreur** : son déposant, tant qu'elle n'est pas contrôlée (quarantaine ou reçue).
    pub async fn retirer_depot_for_actor(&self, a: &Acteur<'_>, piece_id: Uuid) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let pc = piece_par_id(&mut tx, a.etablissement_id, piece_id, true).await?;
        if pc.deposant_user_id != a.user_id {
            // Aucun indice sur l'existence : « introuvable » pour qui n'est pas le déposant.
            return Err(introuvable());
        }
        if !matches!(pc.etat.as_str(), "en_quarantaine" | "recue") {
            return Err(refus(RefusMetier::Conflit, "la pièce est déjà contrôlée : elle ne se retire plus"));
        }
        sqlx::query("UPDATE ent_doc_piece SET etat = 'retiree', maj_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(piece_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        decision(&mut tx, a.etablissement_id, piece_id, pc.demande_id, "retiree", None, None, a.user_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────── Analyser (tâche de fond) ─────────────

    /// **Analyse** une pièce en quarantaine (§11.4) : antivirus, puis conversion. Appelée par la tâche de fond (jamais
    /// dans la requête de dépôt). L'antivirus **indisponible** laisse la pièce en quarantaine : rien n'est servi.
    pub async fn analyser_piece(
        &self,
        etab: Uuid,
        piece_id: Uuid,
        antivirus: Arc<dyn AnalyseurAntivirus>,
    ) -> Result<ResultatAnalyse, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let pc = piece_par_id(&mut tx, etab, piece_id, false).await?;
        tx.commit().await.map_err(db)?;
        if pc.etat != "en_quarantaine" {
            return Ok(ResultatAnalyse::DejaTraitee);
        }
        let original = self
            .lire_objet_empreinte(etab, &pc.cle_objet_original, Some(&pc.sha256_original))
            .await?;
        // Les fichiers reçus (un seul, ou ceux de l'archive de photos).
        let fichiers: Vec<Vec<u8>> = if pc.mime_original == "application/zip" {
            match lire_zip_stored(&original) {
                Ok(l) => l.into_iter().map(|(_, o)| o).collect(),
                Err(_) => return self.conclure_rejet(etab, &pc, "archive de photos illisible").await,
            }
        } else {
            vec![original]
        };
        // 1. Antivirus : chaque fichier reçu, dans un fil bloquant (client réseau synchrone). Fermé par défaut.
        let a2 = antivirus.clone();
        let a_scanner = fichiers.clone();
        let verdict = tokio::task::spawn_blocking(move || {
            for f in &a_scanner {
                let type_fichier = type_de_fichier(f).unwrap_or(TypeFichier::Pdf);
                let v = a2.analyser(&ContexteAnalyse { usage: UsageFichier::Document, type_fichier, nom: "piece", octets: f });
                if v != VerdictAnalyse::Sain {
                    return v;
                }
            }
            VerdictAnalyse::Sain
        })
        .await
        .map_err(|_| refus(RefusMetier::Conflit, "analyse interrompue"))?;
        let sys = systeme(etab);
        match verdict {
            VerdictAnalyse::Indisponible => {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                sqlx::query(
                    "UPDATE ent_doc_piece SET analyse_tentatives = analyse_tentatives + 1, derniere_analyse_le = now(),
                            analyse_motif = 'antivirus indisponible', maj_le = now()
                      WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_quarantaine'",
                )
                .bind(etab)
                .bind(piece_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                if pc.analyse_tentatives == 0 {
                    decision(&mut tx, etab, piece_id, pc.demande_id, "analyse_indisponible", None, None, sys.user_id).await?;
                }
                tx.commit().await.map_err(db)?;
                Ok(ResultatAnalyse::EnQuarantaine)
            }
            VerdictAnalyse::Infecte(nom) => {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                let r = sqlx::query(
                    "UPDATE ent_doc_piece SET etat = 'infectee', analyse_motif = $3, analyse_tentatives = analyse_tentatives + 1,
                            derniere_analyse_le = now(), maj_le = now()
                      WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_quarantaine'",
                )
                .bind(etab)
                .bind(piece_id)
                .bind(format!("menace : {}", nom.chars().take(80).collect::<String>()))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                if r.rows_affected() == 1 {
                    decision(&mut tx, etab, piece_id, pc.demande_id, "infectee", Some("antivirus"), None, sys.user_id).await?;
                    let ev = publier(
                        &mut tx,
                        &sys,
                        EVT_PIECE_REFUSEE_ANALYSE,
                        piece_id,
                        pc.eleve_id,
                        json!({ "type": pc.type_code, "motif": "infectee" }),
                    )
                    .await?;
                    tx.commit().await.map_err(db)?;
                    self.diffuser_apres_commit(&[ev]);
                } else {
                    tx.commit().await.map_err(db)?;
                }
                Ok(ResultatAnalyse::Infectee)
            }
            VerdictAnalyse::Sain => self.convertir_et_lever_quarantaine(etab, &pc, fichiers).await,
        }
    }

    async fn conclure_rejet(&self, etab: Uuid, pc: &LignePiece, motif: &str) -> Result<ResultatAnalyse, EntDbError> {
        let sys = systeme(etab);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = sqlx::query(
            "UPDATE ent_doc_piece SET etat = 'rejetee', analyse_motif = $3, analyse_tentatives = analyse_tentatives + 1,
                    derniere_analyse_le = now(), maj_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_quarantaine'",
        )
        .bind(etab)
        .bind(pc.id)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 1 {
            decision(&mut tx, etab, pc.id, pc.demande_id, "rejetee", Some("forme"), Some(motif), sys.user_id).await?;
            let ev = publier(
                &mut tx,
                &sys,
                EVT_PIECE_REFUSEE_ANALYSE,
                pc.id,
                pc.eleve_id,
                json!({ "type": pc.type_code, "motif": "forme" }),
            )
            .await?;
            tx.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&[ev]);
        } else {
            tx.commit().await.map_err(db)?;
        }
        Ok(ResultatAnalyse::Rejetee(motif.to_string()))
    }

    async fn convertir_et_lever_quarantaine(
        &self,
        etab: Uuid,
        pc: &LignePiece,
        fichiers: Vec<Vec<u8>>,
    ) -> Result<ResultatAnalyse, EntDbError> {
        // 2. Désarmement / normalisation : un PDF désarmé, ou des photos assemblées en un seul PDF sans métadonnées.
        let converti = if pc.mime_original == "application/pdf" {
            desarmer_pdf(&fichiers[0]).map(|d| (d.octets, d.nb_pages, d.neutralises.join(", ")))
        } else {
            let refs: Vec<&[u8]> = fichiers.iter().map(Vec::as_slice).collect();
            images_en_pdf(&refs).map(|p| (p.pdf, p.nb_pages, String::new()))
        };
        let (pdf, nb_pages, desarme) = match converti {
            Ok(x) => x,
            Err(e) => return self.conclure_rejet(etab, pc, &message_erreur_piece(&e)).await,
        };
        let cle = ent_object::ObjectStore::key_for(etab, "pieces", Uuid::new_v4(), "pdf");
        let info = self.objet_put(&cle, &pdf).await?;
        let sys = systeme(etab);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        crate::objets::enregistrer_objet(&mut *tx, etab, &info, Some("application/pdf")).await?;
        // Le seul passage « en quarantaine → reçue » : le fichier servi s'écrit ici, une fois (garde de la base).
        let r = sqlx::query(
            "UPDATE ent_doc_piece SET etat = 'recue', cle_objet_servi = $3, sha256_servi = $4, nb_pages = $5,
                    desarmement = NULLIF($6, ''), analyse_motif = NULL, analyse_tentatives = analyse_tentatives + 1,
                    derniere_analyse_le = now(), maj_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_quarantaine'",
        )
        .bind(etab)
        .bind(pc.id)
        .bind(&info.key)
        .bind(&info.sha256)
        .bind(nb_pages as i32)
        .bind(&desarme)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() != 1 {
            tx.commit().await.map_err(db)?;
            return Ok(ResultatAnalyse::DejaTraitee);
        }
        decision(&mut tx, etab, pc.id, pc.demande_id, "analysee_saine", None, None, sys.user_id).await?;
        let ev = publier(
            &mut tx,
            &sys,
            EVT_PIECE_DEPOSEE,
            pc.id,
            pc.eleve_id,
            json!({ "type": pc.type_code, "demande_id": pc.demande_id, "pages": nb_pages }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ResultatAnalyse::Recue)
    }

    /// Analyse jusqu'à `limite` pièces en quarantaine (les plus anciennes d'abord) : la tâche de fond de l'établissement.
    pub async fn analyser_pieces_en_attente(
        &self,
        etab: Uuid,
        antivirus: Arc<dyn AnalyseurAntivirus>,
        limite: i64,
    ) -> Result<RapportAnalyse, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_doc_piece WHERE etablissement_id = $1 AND etat = 'en_quarantaine' ORDER BY deposee_le, id LIMIT $2",
        )
        .bind(etab)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut r = RapportAnalyse::default();
        for (id,) in ids {
            match self.analyser_piece(etab, id, antivirus.clone()).await? {
                ResultatAnalyse::Recue => r.recues += 1,
                ResultatAnalyse::EnQuarantaine => r.en_quarantaine += 1,
                ResultatAnalyse::Infectee => r.infectees += 1,
                ResultatAnalyse::Rejetee(_) => r.rejetees += 1,
                ResultatAnalyse::DejaTraitee => {}
            }
        }
        Ok(r)
    }

    // ───────────── Lire (jamais l'original, jamais hors quarantaine levée) ─────────────

    /// Lit le fichier **servi** d'une pièce (version désarmée) : jamais l'original, jamais tant qu'elle est en
    /// quarantaine, infectée ou rejetée. La famille lit ce qu'elle a déposé pour son sujet ; le personnel, avec le droit
    /// de contrôle ou de lecture du domaine ; une copie non conservée (identité) ne se relit plus. Une lecture laisse une
    /// ligne au journal.
    pub async fn lire_piece_justificative_for_actor(&self, a: &Acteur<'_>, piece_id: Uuid) -> Result<PieceLue, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let pc = piece_par_id(&mut tx, a.etablissement_id, piece_id, false).await?;
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        let decl = type_piece(&pc.type_code).ok_or_else(introuvable)?;
        let sujet = pc.sujet().ok_or_else(introuvable)?;
        let staff = Role::parse(a.role).is_some_and(|r| r.is_staff());
        let autorise = if staff {
            self.peut_controler(a, decl, sujet.eleve()).await?
                || (self.doc_peut_sur_sujet(a, decl.domaine.action_lecture(), sujet.eleve()).await?
                    && self.droit_du_type(a, decl, sujet.eleve()).await?)
        } else {
            self.famille_a_la_main(&mut tx, a, sujet, jour).await?
        };
        if !autorise {
            return Err(introuvable());
        }
        // Quarantaine : aucun fichier n'existe côté servi (CHECK de la base) ; on le dit sans détail.
        let (Some(cle), Some(sha)) = (pc.cle_objet_servi.clone(), pc.sha256_servi.clone()) else {
            return Err(introuvable());
        };
        if !matches!(pc.etat.as_str(), "recue" | "acceptee" | "refusee" | "sans_objet") {
            return Err(introuvable());
        }
        let supprime: Option<(bool,)> = sqlx::query_as(
            "SELECT supprime_le IS NOT NULL FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2",
        )
        .bind(a.etablissement_id)
        .bind(&cle)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if supprime.is_none_or(|s| s.0) {
            return Err(refus(RefusMetier::Conflit, "la copie de cette pièce n'est pas conservée"));
        }
        if decl.sensibilite >= Sensibilite::C2 {
            journaliser(&mut tx, a, None, None, "piece_consultee", (None, None), Some(&piece_id.to_string()), None).await?;
        }
        tx.commit().await.map_err(db)?;
        let octets = self.lire_objet_empreinte(a.etablissement_id, &cle, Some(&sha)).await?;
        Ok(PieceLue {
            piece_id,
            type_code: pc.type_code,
            etat: pc.etat,
            nb_pages: pc.nb_pages.unwrap_or(0),
            empreinte_sha256: sha,
            octets,
        })
    }

    // ───────────── Contrôler ─────────────

    /// La file de contrôle (pièces **reçues**, les plus anciennes d'abord), restreinte à ce que l'acteur peut contrôler,
    /// avec ce que la démarche attend (§11.5).
    pub async fn pieces_a_controler_for_actor(
        &self,
        a: &Acteur<'_>,
        limite: i64,
    ) -> Result<Vec<PieceAControler>, EntDbError> {
        let r = Role::parse(a.role).ok_or(EntDbError::Forbidden)?;
        if !r.is_staff()
            || !self
                .a_action_etablissement(a.etablissement_id, a.user_id, a.role, Action::DocumentsPiecesControler)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let lignes = sqlx::query(
            "SELECT p.id, p.type_code, p.eleve_id, p.deposee_le, p.nb_pages, p.desarmement, p.demande_id, p.motif_autre,
                    d.motif_affiche,
                    coalesce(
                      (SELECT btrim(e.prenom || ' ' || e.nom) FROM ent_eleve e WHERE e.etablissement_id = p.etablissement_id AND e.id = p.eleve_id),
                      (SELECT btrim(coalesce(x.prenom || ' ', '') || x.nom) FROM ent_personne x WHERE x.etablissement_id = p.etablissement_id AND x.id = p.personne_id),
                      (SELECT btrim(coalesce(x.prenom || ' ', '') || x.nom) FROM ent_agent ag JOIN ent_personne x ON x.etablissement_id = ag.etablissement_id AND x.id = ag.personne_id
                        WHERE ag.etablissement_id = p.etablissement_id AND ag.id = p.agent_id), '?') AS nom
               FROM ent_doc_piece p
               LEFT JOIN ent_doc_piece_demande d ON d.etablissement_id = p.etablissement_id AND d.id = p.demande_id
              WHERE p.etablissement_id = $1 AND p.etat = 'recue'
              ORDER BY p.deposee_le, p.id LIMIT $2",
        )
        .bind(a.etablissement_id)
        .bind(limite.clamp(1, 500) * 4)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut sortie = Vec::new();
        for l in lignes {
            let tc: String = l.get("type_code");
            let Some(decl) = type_piece(&tc) else { continue };
            let eleve: Option<Uuid> = l.get("eleve_id");
            if !self.peut_controler(a, decl, eleve).await? {
                continue;
            }
            sortie.push(PieceAControler {
                piece_id: l.get("id"),
                libelle: decl.libelle.to_string(),
                type_code: tc,
                sujet_nom: l.get("nom"),
                deposee_le: l.get("deposee_le"),
                nb_pages: l.get::<Option<i32>, _>("nb_pages").unwrap_or(0),
                desarmement: l.get("desarmement"),
                demande_id: l.get("demande_id"),
                attendu: l.get("motif_affiche"),
                motif_autre: l.get("motif_autre"),
            });
            if sortie.len() as i64 >= limite.clamp(1, 500) {
                break;
            }
        }
        Ok(sortie)
    }

    /// **Contrôle** une pièce reçue : accepter, refuser (motif type), déclarer sans objet. Une seule décision (la base
    /// refuse la seconde) ; l'acceptation satisfait les demandes ouvertes du même type pour le même sujet **ou le même
    /// foyer** ; l'identité n'est pas conservée.
    pub async fn controler_piece_for_actor(
        &self,
        a: &Acteur<'_>,
        piece_id: Uuid,
        decision_prise: ControleDecision,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let pc = piece_par_id(&mut tx, a.etablissement_id, piece_id, true).await?;
        let decl = type_piece(&pc.type_code).ok_or_else(introuvable)?;
        let sujet = pc.sujet().ok_or_else(introuvable)?;
        if !self.peut_controler(a, decl, sujet.eleve()).await? {
            return Err(EntDbError::Forbidden);
        }
        if pc.etat != "recue" {
            return Err(refus(RefusMetier::Conflit, "cette pièce n'est pas à contrôler (déjà décidée, en analyse ou retirée)"));
        }
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        let (code, motif, commentaire, valable): (&str, Option<String>, Option<String>, Option<NaiveDate>) = match &decision_prise {
            ControleDecision::Accepter { valable_jusqu_au } => {
                ("acceptee", None, None, fin_de_validite(decl, jour, *valable_jusqu_au)?)
            }
            ControleDecision::Refuser { motif, commentaire } => {
                if !["illisible", "incomplet", "perime", "ne_correspond_pas", "mauvais_document", "autre"].contains(&motif.as_str()) {
                    return Err(invalide());
                }
                let c = match commentaire.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
                    Some(c) => Some(texte(c, 1, 500)?),
                    None if motif == "autre" => return Err(refus(RefusMetier::Invalide, "un commentaire est exigé pour le motif « autre »")),
                    None => None,
                };
                ("refusee", Some(motif.clone()), c, None)
            }
            ControleDecision::SansObjet { commentaire } => ("sans_objet", None, Some(texte(commentaire, 3, 500)?), None),
        };
        sqlx::query(
            "UPDATE ent_doc_piece SET etat = $3, controle_par = $4, controle_le = now(), valable_jusqu_au = $5,
                    refus_motif = $6, commentaire = $7, maj_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'recue'",
        )
        .bind(a.etablissement_id)
        .bind(piece_id)
        .bind(code)
        .bind(a.user_id)
        .bind(valable)
        .bind(&motif)
        .bind(&commentaire)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        decision(&mut tx, a.etablissement_id, piece_id, pc.demande_id, code, motif.as_deref(), commentaire.as_deref(), a.user_id).await?;
        let mut demandes_satisfaites = 0usize;
        if code == "acceptee" {
            let eleves: Vec<Uuid> = match (decl.portee, sujet) {
                (PortePiece::Foyer, SujetPiece::Eleve(e)) => eleves_du_foyer(&mut tx, a.etablissement_id, e).await?,
                (_, SujetPiece::Eleve(e)) => vec![e],
                _ => Vec::new(),
            };
            let (_, p, g) = sujet.colonnes();
            let faites = sqlx::query(
                "UPDATE ent_doc_piece_demande SET etat = 'satisfaite', piece_id = $2, cloturee_le = now()
                  WHERE etablissement_id = $1 AND etat = 'ouverte' AND type_code = $3
                    AND (eleve_id = ANY($4) OR personne_id = $5 OR agent_id = $6)",
            )
            .bind(a.etablissement_id)
            .bind(piece_id)
            .bind(decl.code)
            .bind(&eleves)
            .bind(p)
            .bind(g)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            demandes_satisfaites = faites.rows_affected() as usize;
        }
        // Ce qu'on ne garde pas : l'identité n'est vue que le temps du contrôle.
        if !decl.copie_gardee {
            sqlx::query(
                "UPDATE ent_objet SET supprime_le = now()
                  WHERE etablissement_id = $1 AND cle_objet IN ($2, $3) AND supprime_le IS NULL",
            )
            .bind(a.etablissement_id)
            .bind(&pc.cle_objet_original)
            .bind(pc.cle_objet_servi.as_deref().unwrap_or(""))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let ev: MetierEvent = publier(
            &mut tx,
            a,
            EVT_PIECE_DECIDEE,
            piece_id,
            sujet.eleve(),
            json!({ "type": decl.code, "decision": code, "motif": motif, "demandes": demandes_satisfaites }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    /// **Contrôle en rafale** : une décision par pièce, indépendantes (une pièce refusée ou déjà décidée n'arrête pas les
    /// autres). Au plus 100 par appel.
    pub async fn controle_en_rafale_for_actor(
        &self,
        a: &Acteur<'_>,
        decisions: Vec<(Uuid, ControleDecision)>,
    ) -> Result<Vec<ResultatControle>, EntDbError> {
        if decisions.len() > RAFALE_MAX {
            return Err(refus(RefusMetier::Invalide, "trop de décisions en une fois (100 au plus)"));
        }
        let mut sortie = Vec::with_capacity(decisions.len());
        let mut vus = HashSet::new();
        for (id, d) in decisions {
            if !vus.insert(id) {
                sortie.push(ResultatControle { piece_id: id, ok: false, erreur: Some("pièce en double dans la rafale".into()) });
                continue;
            }
            match self.controler_piece_for_actor(a, id, d).await {
                Ok(()) => sortie.push(ResultatControle { piece_id: id, ok: true, erreur: None }),
                Err(EntDbError::Refus(_, m)) => sortie.push(ResultatControle { piece_id: id, ok: false, erreur: Some(m.to_string()) }),
                Err(EntDbError::Forbidden) => sortie.push(ResultatControle { piece_id: id, ok: false, erreur: Some("non autorisé".into()) }),
                Err(e) if crate::documents_coffre::est_introuvable(&e) => {
                    sortie.push(ResultatControle { piece_id: id, ok: false, erreur: Some("pièce introuvable".into()) })
                }
                Err(e) => return Err(e),
            }
        }
        Ok(sortie)
    }

    // ───────────── Réutiliser ─────────────

    /// **Réutilise** une pièce déjà au dossier pour une demande ouverte (§11.3) : la famille (lien actif sur le sujet) ou
    /// l'agent (`documents.pieces.demander`) **confirme**. La pièce doit être acceptée, valable aujourd'hui, du même type et
    /// du même sujet — ou du même foyer pour une pièce de portée « foyer ». Aucun nouveau fichier n'est créé.
    pub async fn reutiliser_piece_for_actor(
        &self,
        a: &Acteur<'_>,
        demande_id: Uuid,
        piece_id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let dem: Option<(String, Option<Uuid>, Option<Uuid>, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT type_code, eleve_id, personne_id, agent_id, etat FROM ent_doc_piece_demande
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(demande_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (tc, de, dp, dg, etat) = dem.ok_or_else(introuvable)?;
        let decl = type_piece(&tc).ok_or_else(introuvable)?;
        let sujet = match (de, dp, dg) {
            (Some(e), _, _) => SujetPiece::Eleve(e),
            (_, Some(p), _) => SujetPiece::Personne(p),
            (_, _, Some(g)) => SujetPiece::Agent(g),
            _ => return Err(introuvable()),
        };
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        let staff = Role::parse(a.role).is_some_and(|r| r.is_staff());
        let autorise = if staff {
            self.doc_peut_sur_sujet(a, Action::DocumentsPiecesDemander, sujet.eleve()).await?
                && self.droit_du_type(a, decl, sujet.eleve()).await?
        } else {
            self.famille_a_la_main(&mut tx, a, sujet, jour).await?
        };
        if !autorise {
            return Err(introuvable());
        }
        if etat != "ouverte" {
            return Err(refus(RefusMetier::Conflit, "cette demande n'est plus ouverte"));
        }
        let candidate = piece_valable(&mut tx, a.etablissement_id, decl, sujet, jour).await?;
        // La pièce choisie doit être l'une des pièces valables du dossier (pas n'importe laquelle).
        let ok: Option<(Uuid,)> = {
            let eleves: Vec<Uuid> = match (decl.portee, sujet) {
                (PortePiece::Foyer, SujetPiece::Eleve(e)) => eleves_du_foyer(&mut tx, a.etablissement_id, e).await?,
                (_, SujetPiece::Eleve(e)) => vec![e],
                _ => Vec::new(),
            };
            let (_, p, g) = sujet.colonnes();
            sqlx::query_as(
                "SELECT id FROM ent_doc_piece
                  WHERE etablissement_id = $1 AND id = $2 AND type_code = $3 AND etat = 'acceptee'
                    AND (valable_jusqu_au IS NULL OR valable_jusqu_au >= $4)
                    AND (eleve_id = ANY($5) OR personne_id = $6 OR agent_id = $7)",
            )
            .bind(a.etablissement_id)
            .bind(piece_id)
            .bind(decl.code)
            .bind(jour)
            .bind(&eleves)
            .bind(p)
            .bind(g)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
        };
        let _ = candidate;
        if ok.is_none() {
            return Err(refus(RefusMetier::Conflit, "cette pièce n'est pas valable pour cette demande (type, sujet, validité)"));
        }
        sqlx::query(
            "UPDATE ent_doc_piece_demande SET etat = 'satisfaite', piece_id = $3, cloturee_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'ouverte'",
        )
        .bind(a.etablissement_id)
        .bind(demande_id)
        .bind(piece_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        decision(&mut tx, a.etablissement_id, piece_id, Some(demande_id), "reutilisee", None, None, a.user_id).await?;
        let ev = publier(
            &mut tx,
            a,
            EVT_PIECE_DECIDEE,
            piece_id,
            sujet.eleve(),
            json!({ "type": decl.code, "decision": "reutilisee", "demandes": 1 }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    // ───────────── Ce qui manque ─────────────

    /// **« Ce qui manque »** (§21) pour un élève, une personne ou un agent : les demandes ouvertes avec leur état
    /// (à fournir, en cours d'analyse, reçue en attente de vérification, refusée avec motif), les pièces acceptées dont la
    /// validité est passée (à renouveler) et, pour chaque manque, la pièce déjà au dossier qu'on peut **proposer**.
    /// Famille : titulaire au lien actif ; personnel : `documents.pieces.demander` (les types sensibles exigent le droit de
    /// leur domaine).
    pub async fn ce_qui_manque_for_actor(&self, a: &Acteur<'_>, sujet: SujetPiece) -> Result<Vec<LigneManque>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        let staff = Role::parse(a.role).is_some_and(|r| r.is_staff());
        if staff {
            if !self.doc_peut_sur_sujet(a, Action::DocumentsPiecesDemander, sujet.eleve()).await? {
                return Err(EntDbError::Forbidden);
            }
        } else if !self.famille_a_la_main(&mut tx, a, sujet, jour).await? {
            return Err(EntDbError::Forbidden);
        }
        let (e, p, g) = sujet.colonnes();
        let demandes = sqlx::query(
            "SELECT d.id, d.type_code, d.obligatoire, d.echeance, d.motif_affiche, d.etat, d.piece_id,
                    (SELECT x.id FROM ent_doc_piece x WHERE x.etablissement_id = d.etablissement_id AND x.demande_id = d.id
                      ORDER BY x.deposee_le DESC, x.id LIMIT 1) AS derniere
               FROM ent_doc_piece_demande d
              WHERE d.etablissement_id = $1 AND d.etat IN ('ouverte', 'satisfaite')
                AND (d.eleve_id IS NOT DISTINCT FROM $2 AND $2 IS NOT NULL OR d.personne_id IS NOT DISTINCT FROM $3 AND $3 IS NOT NULL
                     OR d.agent_id IS NOT DISTINCT FROM $4 AND $4 IS NOT NULL)
              ORDER BY d.echeance NULLS LAST, d.cree_le, d.id",
        )
        .bind(a.etablissement_id)
        .bind(e)
        .bind(p)
        .bind(g)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        for d in demandes {
            let tc: String = d.get("type_code");
            let Some(decl) = type_piece(&tc) else { continue };
            if staff && !self.droit_du_type(a, decl, sujet.eleve()).await? {
                continue;
            }
            let etat_d: String = d.get("etat");
            let demande_id: Uuid = d.get("id");
            let base = |etat, piece, refus, prop, perimee| LigneManque {
                demande_id,
                type_code: tc.clone(),
                libelle: decl.libelle.to_string(),
                obligatoire: d.get("obligatoire"),
                echeance: d.get("echeance"),
                motif_affiche: d.get("motif_affiche"),
                etat,
                piece_id: piece,
                refus_motif: refus,
                proposition_piece_id: prop,
                perimee_le: perimee,
                conseil: decl.conseil,
            };
            if etat_d == "satisfaite" {
                // Acceptée : ne manque que si sa validité est passée.
                let Some(pid) = d.get::<Option<Uuid>, _>("piece_id") else { continue };
                let v: Option<(Option<NaiveDate>,)> = sqlx::query_as(
                    "SELECT valable_jusqu_au FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(pid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                if let Some((Some(fin),)) = v {
                    if fin < jour {
                        let prop = piece_valable(&mut tx, a.etablissement_id, decl, sujet, jour).await?;
                        sortie.push(base(EtatManque::Perimee, Some(pid), None, prop, Some(fin)));
                    }
                }
                continue;
            }
            let derniere: Option<Uuid> = d.get("derniere");
            let (etat, piece, refus_motif) = match derniere {
                None => (EtatManque::AFournir, None, None),
                Some(pid) => {
                    let pc: Option<(String, Option<String>)> = sqlx::query_as(
                        "SELECT etat, refus_motif FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(a.etablissement_id)
                    .bind(pid)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
                    match pc.as_ref().map(|x| x.0.as_str()) {
                        Some("en_quarantaine") => (EtatManque::EnAnalyse, Some(pid), None),
                        Some("recue") => (EtatManque::RecueEnAttente, Some(pid), None),
                        Some("refusee") => (EtatManque::Refusee, Some(pid), pc.and_then(|x| x.1)),
                        Some("infectee" | "rejetee") => (EtatManque::AFournir, Some(pid), Some("analyse".to_string())),
                        _ => (EtatManque::AFournir, None, None),
                    }
                }
            };
            let prop = if matches!(etat, EtatManque::AFournir | EtatManque::Refusee) {
                let sj = match (decl.portee, sujet) {
                    (_, s) => s,
                };
                piece_valable(&mut tx, a.etablissement_id, decl, sj, jour).await?
            } else {
                None
            };
            sortie.push(base(etat, piece, refus_motif, prop, None));
        }
        tx.commit().await.map_err(db)?;
        Ok(sortie)
    }

    /// L'élève est-il à jour de ses pièces **obligatoires** ? (Brique demandeuse : « dossier complet ».)
    pub async fn dossier_complet_pieces(&self, a: &Acteur<'_>, sujet: SujetPiece) -> Result<bool, EntDbError> {
        Ok(!self
            .ce_qui_manque_for_actor(a, sujet)
            .await?
            .iter()
            .any(|l| l.obligatoire && l.etat != EtatManque::RecueEnAttente && l.etat != EtatManque::EnAnalyse))
    }
}

// ───────────────────────── Port de reprise N1-5 ─────────────────────────

use crate::demandes_reprise::{DocumentRepris, PieceSource, PortPiecesReprise};

/// Une pièce de l'ancienne application à copier au socle 7, avec ses octets et la personne qui l'avait déposée.
#[derive(Debug, Clone)]
pub struct PieceARepriser {
    pub origine_id: String,
    pub personne_id: Uuid,
    pub octets: Vec<u8>,
}

/// Réponses du port de reprise, préparées d'avance (le port est **synchrone et sans accès base**, comme les autres ports
/// de N1) : `origine_id` → document créé. Une pièce absente de la table n'a pas pu être reprise (infectée, refusée à
/// l'analyse, illisible, antivirus indisponible) : la reprise la **signale** au rapport, jamais en silence.
#[derive(Debug, Clone, Default)]
pub struct PiecesReprises {
    reprises: std::collections::HashMap<String, DocumentRepris>,
}

impl PiecesReprises {
    pub fn nombre(&self) -> usize {
        self.reprises.len()
    }
}

impl PortPiecesReprise for PiecesReprises {
    fn copier(&self, _etab: Uuid, piece: &PieceSource<'_>) -> Option<DocumentRepris> {
        self.reprises.get(piece.origine_id).copied()
    }
}

impl PgEnt {
    /// Copie les pièces de l'ancienne application **par le même chemin que les dépôts** (quarantaine, antivirus,
    /// désarmement) et rend l'adaptateur à passer à la reprise N1-5. Idempotent : une pièce déjà reprise (même
    /// empreinte, même personne) est retrouvée, pas dupliquée. Seules les pièces **saines** entrent dans la table.
    pub async fn preparer_reprise_pieces(
        &self,
        etab: Uuid,
        pieces: Vec<PieceARepriser>,
        antivirus: Arc<dyn AnalyseurAntivirus>,
    ) -> Result<PiecesReprises, EntDbError> {
        let sys = systeme(etab);
        let mut sortie = PiecesReprises::default();
        for p in pieces {
            let Ok(kind) = type_de_fichier(&p.octets) else { continue };
            let sha = ent_object::sha256_hex(&p.octets);
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let deja: Option<(Uuid, Option<i32>)> = sqlx::query_as(
                "SELECT id, nb_pages FROM ent_doc_piece WHERE etablissement_id = $1 AND personne_id = $2
                    AND type_code = 'piece_de_demande' AND sha256_original = $3 AND etat = 'acceptee' LIMIT 1",
            )
            .bind(etab)
            .bind(p.personne_id)
            .bind(&sha)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((id, pages)) = deja {
                tx.commit().await.map_err(db)?;
                sortie.reprises.insert(p.origine_id, DocumentRepris { document_id: id, pages });
                continue;
            }
            let id = Uuid::new_v4();
            let (mime, ext) = match kind {
                TypeFichier::Pdf => ("application/pdf", "pdf"),
                TypeFichier::Jpeg => ("image/jpeg", "jpg"),
                _ => ("image/png", "png"),
            };
            let cle = ent_object::ObjectStore::key_for(etab, "pieces-quarantaine", id, ext);
            let info = self.objet_put(&cle, &p.octets).await?;
            crate::objets::enregistrer_objet(&mut *tx, etab, &info, Some(mime)).await?;
            sqlx::query(
                "INSERT INTO ent_doc_piece
                     (id, etablissement_id, type_code, personne_id, deposant_user_id, deposant_role, cle_objet_original,
                      sha256_original, taille_originale, mime_original)
                 VALUES ($1,$2,'piece_de_demande',$3,$4,'systeme',$5,$6,$7,$8)",
            )
            .bind(id)
            .bind(etab)
            .bind(p.personne_id)
            .bind(sys.user_id)
            .bind(&info.key)
            .bind(&info.sha256)
            .bind(info.taille as i64)
            .bind(mime)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            decision(&mut tx, etab, id, None, "deposee", Some("reprise"), None, sys.user_id).await?;
            tx.commit().await.map_err(db)?;
            if self.analyser_piece(etab, id, antivirus.clone()).await? != ResultatAnalyse::Recue {
                continue;
            }
            // Pièce reprise : acceptée d'office (elle a déjà servi dans l'ancienne application), sans contrôle humain.
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query(
                "UPDATE ent_doc_piece SET etat = 'acceptee', controle_par = $3, controle_le = now(), commentaire = 'reprise de l''ancienne application', maj_le = now()
                  WHERE etablissement_id = $1 AND id = $2 AND etat = 'recue'",
            )
            .bind(etab)
            .bind(id)
            .bind(sys.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            decision(&mut tx, etab, id, None, "acceptee", Some("reprise"), None, sys.user_id).await?;
            let (pages,): (Option<i32>,) =
                sqlx::query_as("SELECT nb_pages FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
            tx.commit().await.map_err(db)?;
            sortie.reprises.insert(p.origine_id, DocumentRepris { document_id: id, pages });
        }
        Ok(sortie)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn catalogue_coherent() {
        let mut vus = HashSet::new();
        for t in CATALOGUE {
            assert!(vus.insert(t.code), "code en double : {}", t.code);
            assert!(t.code.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{}", t.code);
            assert!(!t.libelle.is_empty());
            // Les types sensibles sont rangés dans le domaine qui porte leur droit.
            if t.sensibilite >= Sensibilite::C3 {
                assert!(matches!(t.domaine, Domaine::Sante | Domaine::Justice | Domaine::Social), "{}", t.code);
            }
        }
        // Ce que le catalogue ne contient pas et qu'un établissement ne peut pas ajouter (§11.2).
        for interdit in ["titre_sejour", "carte_vitale", "rib", "casier_judiciaire", "document_religieux", "photo_domicile"] {
            assert!(type_piece(interdit).is_none(), "{interdit} ne doit pas exister");
        }
        // L'identité n'est jamais conservée ; seul le type technique de reprise n'est pas demandable.
        assert!(!type_piece("piece_identite").unwrap().copie_gardee);
        assert!(CATALOGUE.iter().filter(|t| !t.copie_gardee).count() == 1);
        assert_eq!(CATALOGUE.iter().filter(|t| !t.demandable).map(|t| t.code).collect::<Vec<_>>(), ["piece_de_demande"]);
        // Seule l'identité se contrôle à l'accueil.
        assert_eq!(CATALOGUE.iter().filter(|t| t.accueil_controle).map(|t| t.code).collect::<Vec<_>>(), ["piece_identite"]);
        // Le domicile est de portée foyer, valable 3 mois.
        let dom = type_piece("justificatif_domicile").unwrap();
        assert_eq!((dom.portee, dom.validite), (PortePiece::Foyer, Validite::Mois(3)));
    }

    #[test]
    fn motifs_interdits() {
        for m in [
            "Copie du titre de séjour",
            "CARTE VITALE",
            "relevé d'identité bancaire",
            "Votre RIB",
            "casier judiciaire (bulletin n°3)",
            "certificat de baptême",
            "photos du domicile",
            "attestation de régularité de séjour",
            "Visa en cours",
        ] {
            assert!(motif_interdit(m), "{m}");
        }
        for m in ["Attestation de stage", "Autorisation de sortie", "Ribambelle de mots", "Avis de la médecine scolaire"] {
            assert!(!motif_interdit(m), "{m}");
        }
    }

    #[test]
    fn validites() {
        let dom = type_piece("justificatif_domicile").unwrap();
        assert_eq!(fin_de_validite(dom, d(2026, 11, 15), None).unwrap(), Some(d(2027, 2, 15)));
        assert_eq!(fin_de_validite(dom, d(2026, 11, 30), None).unwrap(), Some(d(2027, 2, 28)), "fin de mois");
        let acte = type_piece("acte_naissance").unwrap();
        assert_eq!(fin_de_validite(acte, d(2026, 11, 15), None).unwrap(), None);
        let ass = type_piece("attestation_assurance").unwrap();
        assert_eq!(fin_de_validite(ass, d(2026, 11, 15), None).unwrap(), Some(d(2027, 8, 31)));
        assert_eq!(fin_de_validite(ass, d(2026, 3, 1), None).unwrap(), Some(d(2026, 8, 31)));
        assert_eq!(fin_de_validite(ass, d(2026, 9, 1), None).unwrap(), Some(d(2027, 8, 31)));
        let qf = type_piece("quotient_familial").unwrap();
        assert_eq!(fin_de_validite(qf, d(2026, 11, 15), None).unwrap(), Some(d(2026, 12, 31)));
        let med = type_piece("certificat_medical").unwrap();
        assert!(fin_de_validite(med, d(2026, 11, 15), None).is_err(), "date à saisir");
        assert_eq!(fin_de_validite(med, d(2026, 11, 15), Some(d(2027, 6, 30))).unwrap(), Some(d(2027, 6, 30)));
        // La date lue sur la pièce peut raccourcir, jamais allonger ; une date passée se refuse.
        assert_eq!(fin_de_validite(dom, d(2026, 11, 15), Some(d(2026, 12, 1))).unwrap(), Some(d(2026, 12, 1)));
        assert_eq!(fin_de_validite(dom, d(2026, 11, 15), Some(d(2030, 1, 1))).unwrap(), Some(d(2027, 2, 15)));
        assert!(fin_de_validite(dom, d(2026, 11, 15), Some(d(2026, 11, 14))).is_err());
    }

    #[test]
    fn sujet_et_portee() {
        let u = Uuid::new_v4();
        let dom = type_piece("justificatif_domicile").unwrap();
        assert!(SujetPiece::Eleve(u).convient(dom) && !SujetPiece::Personne(u).convient(dom));
        let id = type_piece("piece_identite").unwrap();
        assert!(SujetPiece::Personne(u).convient(id) && !SujetPiece::Eleve(u).convient(id));
        let rh = type_piece("piece_rh").unwrap();
        assert!(SujetPiece::Agent(u).convient(rh));
        let autre = type_piece("autre").unwrap();
        assert!(SujetPiece::Agent(u).convient(autre) && SujetPiece::Eleve(u).convient(autre));
    }

    #[test]
    fn formats_a_l_entree() {
        assert_eq!(type_de_fichier(b"%PDF-1.4 x").unwrap(), TypeFichier::Pdf);
        assert_eq!(type_de_fichier(&[0xff, 0xd8, 0xff, 0xe0]).unwrap(), TypeFichier::Jpeg);
        assert_eq!(type_de_fichier(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]).unwrap(), TypeFichier::Png);
        for mauvais in [&b"MZ\x90\x00"[..], b"<html>", b"#!/bin/sh", b"PK\x03\x04", b"GIF89a", b"RIFF....WEBP", b""] {
            assert!(type_de_fichier(mauvais).is_err());
        }
    }
}
