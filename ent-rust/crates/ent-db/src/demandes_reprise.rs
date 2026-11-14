//! Socle N1 (« Demandes et flux de travail »), lot N1-5 : **reprise des données du Next.js (R9)**.
//!
//! Conception : `docs/socles/N1_DEMANDES.md` §29 (et §37, §38, D-061). Ce module lit un **export anonymisé** (format
//! décrit dans `docs/socles/N1_5_LIVRAISON.md`, version 1) et le verse dans les tables `ent_dem_*`. **Jamais** une
//! connexion à la base ni au stockage du Next.js : l'outil ne voit que le texte qu'on lui donne.
//!
//! | Source Next.js | Ce que fait la reprise |
//! |---|---|
//! | `request` ouvertes | reprise intégrale : nature déduite de la route, file par le routage de N1-1, prise en charge reprise si l'agent est retrouvé, historique en faits `reprise`, commentaires **internes** |
//! | `request` closes | archive lecture seule jusqu'à `purge_at` (issue `traitee`, aucun avis) |
//! | `request_attr` | valeur typée si le champ est déclaré par la nature, sinon **note interne** ; jamais de champ créé |
//! | demandeur | table de passage des comptes ; sinon **personne de reprise** « à rapprocher » par le secrétariat ; adresse **masquée** ; jamais de rattachement par e-mail (NJ-84, NJ-88) |
//! | `requests-routing`, `requests-org` | **proposition** (files, natures, désignations datées, adresses non retrouvées), jamais appliquée seule |
//! | HSE de l'année | `en_attente` → `en_validation` (décision à refaire) ; `acceptee` → close **sans aucune séance** (CDM-50) ; refusée, annulée → archive |
//! | photocopies | demandes **ouvertes** seulement ; le reste est perdu (T0 : perdable) |
//!
//! Propriétés : **rejouable** (une ligne de correspondance par objet source : rejouer le même export ne crée rien de
//! plus), **aucun avis** envoyé (R-DM49 : un seul avis de reprise par personne, le jour d'ouverture), **aucune heure**
//! créée au socle 15 (CDM-50 : le compteur du lot est contraint à 0 en base et un déclencheur interdit toute séance sur
//! une HSE reprise comme « transmise hors ScolIA »), **jour d'établissement** partout (jamais le jour du serveur).

use crate::champs_chiffres::ChampsSensibles;
use crate::demandes::RefusDemande;
use crate::demandes::{
    ajouter_fait, echeance_ouvree, inserer_valeur, nature, personne_du_user, ChampType,
    Confidentialite, FaitNouveau, NatureDecl, Urgence, Valeur, ValeurSaisie,
};
use crate::demandes_files::{charger_graphe, jour_metier, journaliser};
use crate::demandes_routage::router_demande_existante;
use crate::service::PgEnt;
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use ent_auth::{Action, Role};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgConnection;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uuid::Uuid;

/// Nom du format d'export attendu.
pub const FORMAT_EXPORT: &str = "scolia-reprise-nextjs";
/// Version du format d'export attendue.
pub const VERSION_EXPORT: u32 = 1;
/// Taille maximale d'un export lu (octets) : un export plus gros est refusé avant tout traitement.
pub const EXPORT_MAX_OCTETS: usize = 64 * 1024 * 1024;

const PHRASE_LECTURE_SEULE: &str =
    "Le guichet est en lecture seule pour quelques instants (bascule ou maintenance) : appelez l'accueil.";
const NOM_PERSONNE_EXTERNE: &str = "Demandeur externe non vérifié (reprise)";
const NOM_PERSONNE_REPRISE: &str = "Ancienne application (reprise)";

// ───────────────────────── Format de l'export (version 1) ─────────────────────────

/// Correspondance de la **table de passage des comptes** (socle 28, mise en service) : compte du Next.js → compte Rust.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ComptePassage {
    pub user_ref: String,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AttrNext {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CommentaireNext {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub at: String,
    #[serde(default)]
    pub by_name: String,
    #[serde(default)]
    pub by_user_id: Option<String>,
    #[serde(default)]
    pub body: String,
    /// Absent ou faux : le commentaire est repris **interne** (jamais visible du demandeur par défaut).
    #[serde(default)]
    pub visible_demandeur: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HistoriqueNext {
    #[serde(default)]
    pub at: String,
    #[serde(default)]
    pub action: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PieceNext {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub s3_key: String,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub comment_id: Option<String>,
}

/// Une ligne de la table `request` du Next.js, avec ses lignes liées.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RequeteNext {
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requester_user_id: Option<String>,
    #[serde(default)]
    pub requester_email: String,
    #[serde(default)]
    pub assigned_unit: String,
    #[serde(default)]
    pub assigned_email: String,
    #[serde(default)]
    pub assigned_route_id: Option<String>,
    #[serde(default)]
    pub purge_at: Option<String>,
    #[serde(default)]
    pub attrs: Vec<AttrNext>,
    #[serde(default)]
    pub comments: Vec<CommentaireNext>,
    #[serde(default)]
    pub history: Vec<HistoriqueNext>,
    #[serde(default)]
    pub attachments: Vec<PieceNext>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActeurNext {
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
}

/// Une demande d'HSE (`demandes-hse/index.json`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HseNext {
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub created_by: ActeurNext,
    #[serde(default)]
    pub resume_demande: String,
    #[serde(default)]
    pub motif: String,
    #[serde(default)]
    pub nombre_heures: Option<f64>,
    #[serde(default)]
    pub classe: String,
    #[serde(default)]
    pub details: String,
    #[serde(default)]
    pub decided_by: Option<ActeurNext>,
    #[serde(default)]
    pub decided_at: Option<String>,
    #[serde(default)]
    pub acceptance_pdf_path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPhotocopieNext {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub content_type: Option<String>,
}

/// Une demande de photocopies (`photocopies-couleur/index.json`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotocopieNext {
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub status: String,
    /// `COULEUR` (défaut des anciennes demandes) ou `NOIR_BLANC`.
    #[serde(default)]
    pub type_impression: Option<String>,
    #[serde(default)]
    pub created_by: ActeurNext,
    #[serde(default)]
    pub motif: String,
    #[serde(default)]
    pub classes_ou_matiere: String,
    #[serde(default)]
    pub nombre_photocopies: i64,
    #[serde(default)]
    pub documents: Vec<DocumentPhotocopieNext>,
    #[serde(default)]
    pub decided_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TacheRoutageNext {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RoutageNext {
    #[serde(default)]
    pub tasks: Vec<TacheRoutageNext>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UniteNext {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub manager_emails: Vec<String>,
    #[serde(default)]
    pub member_emails: Vec<String>,
    #[serde(default)]
    pub task_ids: Vec<String>,
    #[serde(default = "vrai")]
    pub active: bool,
}

fn vrai() -> bool {
    true
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OrgNext {
    #[serde(default)]
    pub units: Vec<UniteNext>,
}

/// L'export anonymisé complet (version 1).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ExportNext {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub comptes: Vec<ComptePassage>,
    #[serde(default)]
    pub requests: Vec<RequeteNext>,
    #[serde(default)]
    pub routing: Option<RoutageNext>,
    #[serde(default)]
    pub org: Option<OrgNext>,
    #[serde(default)]
    pub hse: Vec<HseNext>,
    #[serde(default)]
    pub photocopies: Vec<PhotocopieNext>,
}

// ───────────────────────── Ports et rapport ─────────────────────────

/// Une pièce de l'ancien stockage à copier au socle 7.
#[derive(Debug, Clone, Copy)]
pub struct PieceSource<'a> {
    pub origine_id: &'a str,
    pub nom_fichier: &'a str,
    pub cle_stockage: &'a str,
    pub type_contenu: Option<&'a str>,
}

/// Un document créé au socle 7 par la copie.
#[derive(Debug, Clone, Copy)]
pub struct DocumentRepris {
    pub document_id: Uuid,
    pub pages: Option<i32>,
}

/// Ce que la reprise demande au **socle 7** (documents) : copier une pièce de l'ancien stockage (antivirus, type
/// contrôlé). Instantané sans accès base, comme les autres ports de N1. `None` : pièce **non reprise** (elle reste
/// signalée au rapport : le critère « 0 pièce illisible » n'est atteint que quand le socle 7 est branché).
pub trait PortPiecesReprise: Send + Sync {
    fn copier(&self, etab: Uuid, piece: &PieceSource<'_>) -> Option<DocumentRepris>;
}

/// Socle 7 absent : aucune pièce n'est copiée, toutes sont signalées.
#[derive(Debug, Clone, Copy, Default)]
pub struct PiecesAbsentes;

impl PortPiecesReprise for PiecesAbsentes {
    fn copier(&self, _: Uuid, _: &PieceSource<'_>) -> Option<DocumentRepris> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeReprise {
    /// Répétition (Noël, 15 janvier) : mêmes gestes, sur une base de répétition.
    Repetition,
    /// Bascule : exige le guichet de ScolIA en lecture seule.
    Bascule,
}

impl ModeReprise {
    fn code(self) -> &'static str {
        match self {
            Self::Repetition => "repetition",
            Self::Bascule => "bascule",
        }
    }
}

/// Un comptage du rapport : par source, décision et raison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LigneRapport {
    pub source: String,
    pub decision: String,
    pub raison: Option<String>,
    pub nb: i64,
}

/// Une demande reprise, pour la relecture par échantillon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EchantillonReprise {
    pub source_id: String,
    pub numero: String,
    pub nature_code: String,
    pub statut: String,
}

/// Le rapport d'une passe (DEM-343). Aucune donnée personnelle : des comptes, des codes, des numéros.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RapportReprise {
    pub lot_id: Uuid,
    pub empreinte_export: String,
    pub rejoue: bool,
    pub comptages: Vec<LigneRapport>,
    /// Critère go/no-go : 100 % des demandes ouvertes reprises et attribuées à une file.
    pub requetes_ouvertes: i64,
    pub requetes_ouvertes_reprises: i64,
    /// Critère go/no-go : 0 heure HSE créée au socle 15 par la reprise.
    pub heures_creees_au_15: i32,
    /// Pièces qui n'ont pas pu être copiées au socle 7 (« 0 pièce illisible »).
    pub pieces_non_reprises: i64,
    pub demandeurs_a_rapprocher: i64,
    pub propositions: i64,
    pub echantillon: Vec<EchantillonReprise>,
    pub duree_ms: i64,
}

impl RapportReprise {
    /// Le critère go/no-go de §29.2 est-il rempli ? (La relecture humaine des 20 demandes et la lisibilité des pièces
    /// restent à faire par une personne.)
    pub fn go_no_go(&self) -> bool {
        self.requetes_ouvertes == self.requetes_ouvertes_reprises
            && self.heures_creees_au_15 == 0
            && self.pieces_non_reprises == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DemandeurARapprocher {
    pub source_ref: String,
    pub repere_masque: Option<String>,
    pub etat: String,
    pub nb_demandes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EtatGuichet {
    pub lecture_seule: bool,
    pub raison: Option<String>,
}

// ───────────────────────── Fonctions pures ─────────────────────────

/// Horodatage RFC 3339, ou jour seul (midi UTC : jamais un changement de jour d'établissement).
pub fn horodatage(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    if let Ok(n) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(n.and_utc());
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(12, 0, 0))
        .map(|n| n.and_utc())
}

/// « j***@e***.fr » : de quoi reconnaître, jamais de quoi écrire.
pub fn masquer_adresse(a: &str) -> Option<String> {
    let a = a.trim();
    let (local, domaine) = a.split_once('@')?;
    let l = local.chars().next()?;
    let d = domaine.chars().next()?;
    let tld = domaine
        .rsplit_once('.')
        .map(|(_, t)| t)
        .filter(|t| !t.is_empty() && t.len() <= 10);
    Some(match tld {
        Some(t) => format!("{l}***@{d}***.{}", t.to_lowercase()),
        None => format!("{l}***@{d}***"),
    })
}

fn empreinte_hex(octets: &[u8]) -> String {
    Sha256::digest(octets)
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect()
}

/// Code `[a-z0-9_]{1,60}` tiré d'un libellé source (jamais d'un texte libre dans un fait).
fn code_depuis(s: &str, par_defaut: &str) -> String {
    let mut c = String::new();
    for ch in s.trim().chars().flat_map(char::to_lowercase) {
        let ch = match ch {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            x => x,
        };
        if ch.is_ascii_alphanumeric() {
            c.push(ch);
        } else if !c.ends_with('_') && !c.is_empty() {
            c.push('_');
        }
        if c.len() >= 60 {
            break;
        }
    }
    let c = c.trim_matches('_').to_string();
    if c.is_empty() {
        par_defaut.to_string()
    } else {
        c
    }
}

/// Nature déduite de la route du Next.js (§29.1). Renvoie la nature et, pour la corbeille et les routes inconnues,
/// l'obligation de passer par la **file de tri**.
pub fn nature_depuis_route(route: &str) -> (&'static str, bool) {
    let r = route.trim().to_lowercase();
    if r.starts_with("maintenance") {
        ("maintenance.intervention", false)
    } else if r.starts_with("comptabilite") {
        ("comptabilite.demande_interne", false)
    } else if r.starts_with("admin") || r.starts_with("accueil") {
        ("secretariat.demande_interne", false)
    } else if r.starts_with("rh") {
        ("rh.demande_agent", false)
    } else if r.starts_with("cpe_") || r.starts_with("vie_scolaire") {
        ("vie_scolaire.demande_interne", false)
    } else {
        // `corbeille` et tout ce qu'on ne reconnaît pas : file de tri, un humain décide (jamais perdu).
        ("secretariat.demande_interne", true)
    }
}

// ───────────────────────── Guichet en lecture seule (DEM-349) ─────────────────────────

/// Le guichet de l'établissement est-il en lecture seule ? Lu à chaque écriture de demande (dépôt et gestes).
pub(crate) async fn guichet_lecture_seule(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<bool, RefusDemande> {
    let r: Option<(bool,)> =
        sqlx::query_as("SELECT lecture_seule FROM ent_dem_guichet WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_optional(conn)
            .await?;
    Ok(r.is_some_and(|(b,)| b))
}

/// Refus propre quand le guichet est en lecture seule : une phrase claire, aucune demande créée ni modifiée.
pub(crate) async fn refuser_si_lecture_seule(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<(), RefusDemande> {
    if guichet_lecture_seule(conn, etab).await? {
        return Err(RefusDemande::Conflit(PHRASE_LECTURE_SEULE));
    }
    Ok(())
}

// ───────────────────────── Contexte d'une passe ─────────────────────────

struct Ctx<'a> {
    etab: Uuid,
    lot: Uuid,
    jour: NaiveDate,
    annee_id: Uuid,
    annee_debut: NaiveDate,
    annee_fin: NaiveDate,
    comptes: HashMap<String, Uuid>,
    pieces: &'a dyn PortPiecesReprise,
}

/// Issue du traitement d'un objet source.
struct Sortie {
    decision: &'static str,
    raison: Option<&'static str>,
    cible: Option<Uuid>,
}

impl Sortie {
    fn reprise(cible: Uuid) -> Self {
        Self {
            decision: "reprise",
            raison: None,
            cible: Some(cible),
        }
    }
    fn archive(cible: Uuid, raison: Option<&'static str>) -> Self {
        Self {
            decision: "archive",
            raison,
            cible: Some(cible),
        }
    }
    fn ignoree(raison: &'static str) -> Self {
        Self {
            decision: "ignoree",
            raison: Some(raison),
            cible: None,
        }
    }
    fn a_verifier(raison: &'static str) -> Self {
        Self {
            decision: "a_verifier",
            raison: Some(raison),
            cible: None,
        }
    }
}

#[derive(Debug, Default)]
struct Compteurs {
    rejoue: bool,
}

impl PgEnt {
    /// Lance une passe de reprise sur un **export anonymisé** (texte JSON, format version 1) et rend son rapport.
    ///
    /// Réservé à `demandes.reprise` (administrateur). En mode **bascule**, le guichet de ScolIA doit être en lecture
    /// seule. Rejouable : un même export relu ne crée aucune ligne de plus. Aucun avis n'est envoyé.
    pub async fn reprendre_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        mode: ModeReprise,
        export: &str,
        pieces: &dyn PortPiecesReprise,
    ) -> Result<RapportReprise, RefusDemande> {
        let debut = std::time::Instant::now();
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !role.is_staff()
            || !self
                .a_action_etablissement(etab, user_id, role.as_str(), Action::DemandesReprise)
                .await?
        {
            return Err(RefusDemande::Interdit);
        }
        if export.len() > EXPORT_MAX_OCTETS {
            return Err(RefusDemande::Invalide("L'export est trop volumineux."));
        }
        let donnees: ExportNext = serde_json::from_str(export).map_err(|_| {
            RefusDemande::Invalide("L'export n'est pas lisible : format attendu non respecté.")
        })?;
        if donnees.format != FORMAT_EXPORT || donnees.version != VERSION_EXPORT {
            return Err(RefusDemande::Invalide(
                "L'export n'est pas au format attendu (version 1).",
            ));
        }
        let empreinte = empreinte_hex(export.as_bytes());

        // Le jour et l'année sont ceux de l'ÉTABLISSEMENT, lus une fois pour toute la passe.
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if mode == ModeReprise::Bascule && !guichet_lecture_seule(&mut tx, etab).await? {
            return Err(RefusDemande::Conflit(
                "Mettez d'abord le guichet en lecture seule avant la reprise de bascule.",
            ));
        }
        let jour = jour_metier(&mut tx, etab).await?;
        let annee: Option<(Uuid, NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT id, date_debut, date_fin FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND (courante OR (date_debut <= $2 AND date_fin >= $2))
              ORDER BY courante DESC, date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((annee_id, annee_debut, annee_fin)) = annee else {
            tracing::error!("reprise : aucune année scolaire pour numéroter");
            return Err(RefusDemande::Indisponible);
        };
        let (rejoue,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_dem_reprise_lot WHERE etablissement_id = $1 AND empreinte_export = $2)",
        )
        .bind(etab)
        .bind(&empreinte)
        .fetch_one(&mut *tx)
        .await?;
        let lot = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_dem_reprise_lot (id, etablissement_id, mode, empreinte_export, jour_export, lance_par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(lot)
        .bind(etab)
        .bind(mode.code())
        .bind(&empreinte)
        .bind(jour)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        journaliser(
            &mut tx,
            etab,
            "reprise",
            mode.code(),
            None,
            Some("lot_ouvert"),
            user_id,
        )
        .await?;
        tx.commit().await?;

        let mut comptes = HashMap::new();
        for c in &donnees.comptes {
            if let Some(u) = c.user_id {
                comptes.insert(c.user_ref.clone(), u);
            }
        }
        let ctx = Ctx {
            etab,
            lot,
            jour,
            annee_id,
            annee_debut,
            annee_fin,
            comptes,
            pieces,
        };
        let mut cpt = Compteurs { rejoue };

        for r in &donnees.requests {
            self.passe_objet(&ctx, "request", &r.id, &mut cpt, |tx, ctx, _| {
                Box::pin(reprendre_requete(tx, ctx, r))
            })
            .await?;
        }
        for h in &donnees.hse {
            self.passe_objet(&ctx, "demandes_hse", &h.id, &mut cpt, |tx, ctx, _| {
                Box::pin(reprendre_hse(tx, ctx, h))
            })
            .await?;
        }
        for p in &donnees.photocopies {
            self.passe_objet(&ctx, "photocopies", &p.id, &mut cpt, |tx, ctx, _| {
                Box::pin(reprendre_photocopie(tx, ctx, p))
            })
            .await?;
        }
        self.proposer_organisation(&ctx, donnees.routing.as_ref(), donnees.org.as_ref())
            .await?;

        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query("UPDATE ent_dem_reprise_lot SET termine_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(lot)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let mut rapport = self.rapport_du_lot(etab, lot).await?;
        rapport.rejoue = cpt.rejoue;
        rapport.duree_ms = i64::try_from(debut.elapsed().as_millis()).unwrap_or(i64::MAX);
        Ok(rapport)
    }

    /// Un objet source, dans **sa propre transaction** : une erreur sur un objet ne bloque pas les autres, elle le
    /// classe « à vérifier » (le détail va au journal technique, jamais au rapport).
    async fn passe_objet<'a, F>(
        &self,
        ctx: &Ctx<'a>,
        source: &'static str,
        source_id: &str,
        cpt: &mut Compteurs,
        f: F,
    ) -> Result<(), RefusDemande>
    where
        F: for<'t> FnOnce(
            &'t mut PgConnection,
            &'t Ctx<'a>,
            &'t mut Compteurs,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Sortie, RefusDemande>> + Send + 't>,
        >,
    {
        if source_id.trim().is_empty() || source_id.len() > 200 {
            // Pas d'identifiant exploitable : on ne peut pas rejouer proprement, on le dit sans écrire.
            tracing::warn!(
                source,
                "reprise : objet source sans identifiant exploitable"
            );
            return Ok(());
        }
        let mut tx = TenantTx::begin(self.pool(), ctx.etab).await?;
        let existant: Option<(String, Option<Uuid>)> = sqlx::query_as(
            "SELECT decision, cible_id FROM ent_dem_reprise_correspondance
              WHERE etablissement_id = $1 AND source = $2 AND source_id = $3 FOR UPDATE",
        )
        .bind(ctx.etab)
        .bind(source)
        .bind(source_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((decision, _)) = &existant {
            if decision != "a_verifier" {
                // Rejouer : rien n'est recréé. On note seulement le passage.
                sqlx::query(
                    "UPDATE ent_dem_reprise_correspondance SET lot_reprise_id = $4, passages = passages + 1, maj_le = now()
                      WHERE etablissement_id = $1 AND source = $2 AND source_id = $3",
                )
                .bind(ctx.etab)
                .bind(source)
                .bind(source_id)
                .bind(ctx.lot)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                return Ok(());
            }
        }
        let resultat = f(&mut tx, ctx, cpt).await;
        match resultat {
            Ok(sortie) => {
                noter(&mut tx, ctx, source, source_id, &sortie).await?;
                tx.commit().await?;
            }
            Err(e) => {
                tracing::error!(source, erreur = %e, "reprise : objet non repris (erreur) ; classé à vérifier");
                drop(tx);
                let mut tx = TenantTx::begin(self.pool(), ctx.etab).await?;
                noter(
                    &mut tx,
                    ctx,
                    source,
                    source_id,
                    &Sortie::a_verifier("erreur_technique"),
                )
                .await?;
                tx.commit().await?;
            }
        }
        Ok(())
    }

    /// Le rapport d'un lot : comptages par source, décision et raison, et les deux critères go/no-go (§29.2).
    pub async fn rapport_du_lot(
        &self,
        etab: Uuid,
        lot: Uuid,
    ) -> Result<RapportReprise, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let lignes: Vec<(String, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT source, decision, raison_code, count(*) FROM ent_dem_reprise_correspondance
              WHERE etablissement_id = $1 AND lot_reprise_id = $2
              GROUP BY source, decision, raison_code ORDER BY source, decision, raison_code NULLS FIRST",
        )
        .bind(etab)
        .bind(lot)
        .fetch_all(&mut *tx)
        .await?;
        let (empreinte, heures): (String, i32) = sqlx::query_as(
            "SELECT empreinte_export, heures_creees_au_15 FROM ent_dem_reprise_lot WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lot)
        .fetch_one(&mut *tx)
        .await?;
        // Les demandes ouvertes de la source sont celles qui sont (ou devaient être) reprises (un statut inconnu compte :
        // il bloque le go) ; les archives (closes) et les closes sans date de purge ne comptent pas.
        let (ouvertes, reprises): (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE c.raison_code IS DISTINCT FROM 'close_archivee'
                                      AND c.raison_code IS DISTINCT FROM 'purge_at_absente'
                                      AND c.decision IN ('reprise', 'a_verifier')),
                    count(*) FILTER (WHERE c.decision = 'reprise')
               FROM ent_dem_reprise_correspondance c
              WHERE c.etablissement_id = $1 AND c.lot_reprise_id = $2 AND c.source = 'request'",
        )
        .bind(etab)
        .bind(lot)
        .fetch_one(&mut *tx)
        .await?;
        let (pieces_non,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_dem_reprise_correspondance
              WHERE etablissement_id = $1 AND lot_reprise_id = $2 AND source IN ('request_piece', 'hse_piece', 'photocopies_piece')
                AND decision <> 'reprise'",
        )
        .bind(etab)
        .bind(lot)
        .fetch_one(&mut *tx)
        .await?;
        let (a_rapprocher,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_dem_reprise_demandeur WHERE etablissement_id = $1 AND etat = 'a_rapprocher'",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await?;
        let (propositions,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_dem_reprise_proposition WHERE etablissement_id = $1 AND lot_reprise_id = $2",
        )
        .bind(etab)
        .bind(lot)
        .fetch_one(&mut *tx)
        .await?;
        let echantillon: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT c.source_id, d.numero, d.nature_code, d.statut
               FROM ent_dem_reprise_correspondance c
               JOIN ent_dem_demande d ON d.etablissement_id = c.etablissement_id AND d.id = c.cible_id
              WHERE c.etablissement_id = $1 AND c.lot_reprise_id = $2 AND c.decision = 'reprise'
              ORDER BY md5(c.source_id || $2::text) LIMIT 20",
        )
        .bind(etab)
        .bind(lot)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(RapportReprise {
            lot_id: lot,
            empreinte_export: empreinte,
            rejoue: false,
            comptages: lignes
                .into_iter()
                .map(|(source, decision, raison, nb)| LigneRapport {
                    source,
                    decision,
                    raison,
                    nb,
                })
                .collect(),
            requetes_ouvertes: ouvertes,
            requetes_ouvertes_reprises: reprises,
            heures_creees_au_15: heures,
            pieces_non_reprises: pieces_non,
            demandeurs_a_rapprocher: a_rapprocher,
            propositions,
            echantillon: echantillon
                .into_iter()
                .map(
                    |(source_id, numero, nature_code, statut)| EchantillonReprise {
                        source_id,
                        numero,
                        nature_code,
                        statut,
                    },
                )
                .collect(),
            duree_ms: 0,
        })
    }

    /// Rapport du dernier lot de l'établissement (DEM-343), pour l'administrateur.
    pub async fn rapport_reprise_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<RapportReprise, RefusDemande> {
        self.controle_reprise(etab, user_id, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let dernier: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_dem_reprise_lot WHERE etablissement_id = $1 ORDER BY lance_le DESC, id LIMIT 1",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        let Some((lot,)) = dernier else {
            return Err(RefusDemande::Introuvable);
        };
        self.rapport_du_lot(etab, lot).await
    }

    async fn controle_reprise(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<(), RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !role.is_staff()
            || !self
                .a_action_etablissement(etab, user_id, role.as_str(), Action::DemandesReprise)
                .await?
        {
            return Err(RefusDemande::Interdit);
        }
        Ok(())
    }

    // ───────────── Guichet ─────────────

    /// Met le guichet de ScolIA en lecture seule (ou le rouvre) : dépôts et gestes refusés proprement (DEM-349).
    /// Tracé au journal des réglages. Le geste côté Next.js est celui de Florian, hors de ce dépôt.
    pub async fn regler_guichet_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        lecture_seule: bool,
        raison: &str,
    ) -> Result<EtatGuichet, RefusDemande> {
        self.controle_reprise(etab, user_id, role).await?;
        if !matches!(raison, "bascule" | "incident" | "maintenance") {
            return Err(RefusDemande::Invalide(
                "La raison du mode lecture seule est inconnue.",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let avant = guichet_lecture_seule(&mut tx, etab).await?;
        sqlx::query(
            "INSERT INTO ent_dem_guichet (etablissement_id, lecture_seule, raison, depuis_le, maj_par_user_id)
             VALUES ($1, $2, $3, CASE WHEN $2 THEN now() END, $4)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET lecture_seule = EXCLUDED.lecture_seule, raison = EXCLUDED.raison,
                    depuis_le = CASE WHEN EXCLUDED.lecture_seule THEN COALESCE(
                        CASE WHEN ent_dem_guichet.lecture_seule THEN ent_dem_guichet.depuis_le END, now()) END,
                    maj_le = now(), maj_par_user_id = EXCLUDED.maj_par_user_id",
        )
        .bind(etab)
        .bind(lecture_seule)
        .bind(raison)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        let dit = |b: bool| if b { "lecture_seule" } else { "ouvert" };
        journaliser(
            &mut tx,
            etab,
            "guichet",
            raison,
            Some(dit(avant)),
            Some(dit(lecture_seule)),
            user_id,
        )
        .await?;
        tx.commit().await?;
        self.etat_guichet(etab).await
    }

    /// État du guichet (DEM-365) : de quoi afficher « le guichet est en lecture seule, appelez l'accueil ».
    pub async fn etat_guichet(&self, etab: Uuid) -> Result<EtatGuichet, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r: Option<(bool, String)> = sqlx::query_as(
            "SELECT lecture_seule, raison FROM ent_dem_guichet WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(match r {
            Some((true, raison)) => EtatGuichet {
                lecture_seule: true,
                raison: Some(raison),
            },
            _ => EtatGuichet {
                lecture_seule: false,
                raison: None,
            },
        })
    }

    // ───────────── Demandeurs à rapprocher ─────────────

    /// La liste « demandeurs à rapprocher » du secrétariat : adresse **masquée**, nombre de demandes concernées.
    pub async fn demandeurs_a_rapprocher_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<Vec<DemandeurARapprocher>, RefusDemande> {
        self.controle_reprise(etab, user_id, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let l: Vec<(String, Option<String>, String, i64)> = sqlx::query_as(
            "SELECT r.source_ref, r.repere_masque, r.etat,
                    (SELECT count(*) FROM ent_dem_demande d
                      WHERE d.etablissement_id = r.etablissement_id AND d.demandeur_personne_id = r.personne_id
                        AND d.canal_depot = 'reprise')
               FROM ent_dem_reprise_demandeur r
              WHERE r.etablissement_id = $1 AND r.etat IN ('a_rapprocher', 'externe_non_verifie')
              ORDER BY r.source_ref",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(l.into_iter()
            .map(
                |(source_ref, repere_masque, etat, nb_demandes)| DemandeurARapprocher {
                    source_ref,
                    repere_masque,
                    etat,
                    nb_demandes,
                },
            )
            .collect())
    }

    /// Le secrétariat rapproche un demandeur de reprise de **la personne qu'il a retrouvée** : ses demandes reprises
    /// lui sont alors attachées. Jamais d'e-mail, jamais automatique (NJ-84, NJ-88). Tracé (fait par demande).
    pub async fn rapprocher_demandeur_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        source_ref: &str,
        personne_id: Uuid,
    ) -> Result<u64, RefusDemande> {
        self.controle_reprise(etab, user_id, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let ligne: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT personne_id, etat FROM ent_dem_reprise_demandeur
              WHERE etablissement_id = $1 AND source_ref = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(source_ref)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((placeholder, etat)) = ligne else {
            return Err(RefusDemande::Introuvable);
        };
        if !matches!(etat.as_str(), "a_rapprocher" | "externe_non_verifie")
            || placeholder == personne_id
        {
            return Err(RefusDemande::Conflit("Ce demandeur est déjà rapproché."));
        }
        // La personne choisie existe dans CET établissement et n'est pas fusionnée ailleurs.
        let existe: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND id = $2 AND fusionnee_dans IS NULL AND source <> 'reprise'",
        )
        .bind(etab)
        .bind(personne_id)
        .fetch_optional(&mut *tx)
        .await?;
        if existe.is_none() {
            return Err(RefusDemande::Invalide(
                "La personne choisie est introuvable.",
            ));
        }
        sqlx::query("SELECT set_config('ent.dem_rapprochement', 'on', true)")
            .execute(&mut *tx)
            .await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_dem_demande
              WHERE etablissement_id = $1 AND demandeur_personne_id = $2 AND canal_depot = 'reprise' AND purgee_le IS NULL
              ORDER BY id FOR UPDATE",
        )
        .bind(etab)
        .bind(placeholder)
        .fetch_all(&mut *tx)
        .await?;
        for (id,) in &ids {
            sqlx::query(
                "UPDATE ent_dem_demande SET demandeur_personne_id = $3, version = version + 1
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(personne_id)
            .execute(&mut *tx)
            .await?;
            let mut f = FaitNouveau::nouveau(*id, "rapprochement");
            f.auteur_personne = Some(moi);
            f.motif = Some("demandeur_rapproche");
            ajouter_fait(&mut tx, etab, f).await?;
        }
        sqlx::query(
            "UPDATE ent_dem_reprise_demandeur SET etat = 'rapproche', personne_id = $3, rapproche_par_user_id = $4, rapproche_le = now()
              WHERE etablissement_id = $1 AND source_ref = $2",
        )
        .bind(etab)
        .bind(source_ref)
        .bind(personne_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ids.len() as u64)
    }

    // ───────────── Organisation (proposition) ─────────────

    /// §29.1 : unités → files, tâches → natures, responsables et membres (adresses) → agents retrouvés en
    /// **désignations datées** ; adresses non retrouvées listées (masquées). **Jamais appliqué** : une proposition.
    async fn proposer_organisation(
        &self,
        ctx: &Ctx<'_>,
        routing: Option<&RoutageNext>,
        org: Option<&OrgNext>,
    ) -> Result<(), RefusDemande> {
        let (Some(org), routing) = (org, routing) else {
            return Ok(());
        };
        let etiquettes: BTreeMap<&str, &str> = routing
            .map(|r| {
                r.tasks
                    .iter()
                    .map(|t| (t.id.as_str(), t.label.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        let mut tx = TenantTx::begin(self.pool(), ctx.etab).await?;
        for u in org
            .units
            .iter()
            .filter(|u| u.active && !u.id.trim().is_empty())
        {
            let code_file = code_depuis(&u.id, "service");
            proposer(
                &mut tx,
                ctx,
                "file",
                &u.id,
                Some(&u.label),
                None,
                Some(&code_file),
                None,
                false,
                None,
                None,
            )
            .await?;
            for t in &u.task_ids {
                let (nature_code, _) = nature_depuis_route(t);
                let cle = format!("{}|{}", u.id, t);
                let libelle = etiquettes.get(t.as_str()).copied().unwrap_or(t.as_str());
                proposer(
                    &mut tx,
                    ctx,
                    "nature",
                    &cle,
                    Some(libelle),
                    Some(nature_code),
                    Some(&code_file),
                    None,
                    false,
                    None,
                    None,
                )
                .await?;
            }
            let responsables: BTreeSet<String> = u
                .manager_emails
                .iter()
                .map(|e| e.trim().to_lowercase())
                .collect();
            let tous: BTreeSet<String> = u
                .manager_emails
                .iter()
                .chain(u.member_emails.iter())
                .map(|e| e.trim().to_lowercase())
                .filter(|e| !e.is_empty())
                .collect();
            for email in tous {
                match agent_par_email(&mut tx, ctx.etab, &email).await? {
                    Some(agent) => {
                        let cle = format!("{}|{}", u.id, agent);
                        proposer(
                            &mut tx,
                            ctx,
                            "designation",
                            &cle,
                            Some(&u.label),
                            None,
                            Some(&code_file),
                            Some(agent),
                            responsables.contains(&email),
                            Some(ctx.jour),
                            None,
                        )
                        .await?;
                    }
                    None => {
                        let Some(masque) = masquer_adresse(&email) else {
                            continue;
                        };
                        let cle = format!("{}|{}", u.id, masque);
                        proposer(
                            &mut tx,
                            ctx,
                            "adresse_non_retrouvee",
                            &cle,
                            Some(&u.label),
                            None,
                            Some(&code_file),
                            None,
                            false,
                            None,
                            Some(&masque),
                        )
                        .await?;
                    }
                }
            }
            noter(
                &mut tx,
                ctx,
                "org",
                &u.id,
                &Sortie::a_verifier("proposition_a_relire"),
            )
            .await?;
        }
        if let Some(r) = routing {
            for t in r.tasks.iter().filter(|t| !t.id.trim().is_empty()) {
                noter(
                    &mut tx,
                    ctx,
                    "routing",
                    &t.id,
                    &Sortie::a_verifier("proposition_a_relire"),
                )
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
async fn proposer(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    genre: &str,
    cle: &str,
    libelle: Option<&str>,
    nature_code: Option<&str>,
    file_code: Option<&str>,
    agent: Option<Uuid>,
    responsable: bool,
    du: Option<NaiveDate>,
    repere_masque: Option<&str>,
) -> Result<(), RefusDemande> {
    let libelle = libelle.map(|l| l.chars().take(200).collect::<String>());
    sqlx::query(
        "INSERT INTO ent_dem_reprise_proposition (id, etablissement_id, lot_reprise_id, genre, cle, libelle, nature_code,
                file_code, agent_id, responsable, du, repere_masque)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
         ON CONFLICT (etablissement_id, genre, cle) DO UPDATE
            SET lot_reprise_id = EXCLUDED.lot_reprise_id, libelle = EXCLUDED.libelle, agent_id = EXCLUDED.agent_id,
                responsable = EXCLUDED.responsable
          WHERE ent_dem_reprise_proposition.statut = 'proposee'",
    )
    .bind(Uuid::new_v4())
    .bind(ctx.etab)
    .bind(ctx.lot)
    .bind(genre)
    .bind(cle.chars().take(200).collect::<String>())
    .bind(libelle)
    .bind(nature_code)
    .bind(file_code)
    .bind(agent)
    .bind(responsable)
    .bind(du)
    .bind(repere_masque)
    .execute(conn)
    .await?;
    Ok(())
}

/// Un agent retrouvé par l'adresse de sa fiche de personne (adresse exacte, sans casse). Réservé aux **personnels** :
/// jamais utilisé pour un demandeur (NJ-84, NJ-88).
async fn agent_par_email(
    conn: &mut PgConnection,
    etab: Uuid,
    email: &str,
) -> Result<Option<Uuid>, RefusDemande> {
    let e = email.trim();
    if e.is_empty() {
        return Ok(None);
    }
    let r: Option<(Uuid,)> = sqlx::query_as(
        "SELECT a.id FROM ent_agent a
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND lower(btrim(p.email)) = lower($2) AND p.fusionnee_dans IS NULL
          ORDER BY a.cree_le, a.id LIMIT 1",
    )
    .bind(etab)
    .bind(e)
    .fetch_optional(conn)
    .await?;
    Ok(r.map(|(i,)| i))
}

/// Écrit (ou met à jour) la correspondance d'un objet source.
async fn noter(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    source: &str,
    source_id: &str,
    s: &Sortie,
) -> Result<(), RefusDemande> {
    sqlx::query(
        "INSERT INTO ent_dem_reprise_correspondance (id, etablissement_id, source, source_id, cible_id, decision, raison_code,
                lot_reprise_id, premier_lot_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
         ON CONFLICT (etablissement_id, source, source_id) DO UPDATE
            SET cible_id = EXCLUDED.cible_id, decision = EXCLUDED.decision, raison_code = EXCLUDED.raison_code,
                lot_reprise_id = EXCLUDED.lot_reprise_id,
                passages = ent_dem_reprise_correspondance.passages + 1, maj_le = now()",
    )
    .bind(Uuid::new_v4())
    .bind(ctx.etab)
    .bind(source)
    .bind(source_id)
    .bind(s.cible)
    .bind(s.decision)
    .bind(s.raison)
    .bind(ctx.lot)
    .execute(conn)
    .await?;
    Ok(())
}

// ───────────────────────── Demandeur et personnes de reprise ─────────────────────────

/// La personne « Ancienne application (reprise) » : auteur technique des messages dont l'auteur n'est pas retrouvé.
async fn personne_reprise(conn: &mut PgConnection, etab: Uuid) -> Result<Uuid, RefusDemande> {
    let r: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND source = 'reprise' AND nom = $2 AND user_id IS NULL
          ORDER BY created_at, id LIMIT 1",
    )
    .bind(etab)
    .bind(NOM_PERSONNE_REPRISE)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some((id,)) = r {
        return Ok(id);
    }
    creer_personne_reprise(conn, etab, NOM_PERSONNE_REPRISE).await
}

async fn creer_personne_reprise(
    conn: &mut PgConnection,
    etab: Uuid,
    nom: &str,
) -> Result<Uuid, RefusDemande> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom, source) VALUES ($1, $2, $3, 'reprise')")
        .bind(id)
        .bind(etab)
        .bind(nom)
        .execute(conn)
        .await?;
    Ok(id)
}

/// Le demandeur d'une demande reprise. Compte retrouvé par la **table de passage** ; sinon une personne de reprise
/// « à rapprocher » (adresse masquée). **Jamais** de recherche par e-mail (NJ-84, NJ-88).
async fn demandeur_de_reprise(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    user_ref: Option<&str>,
    email: &str,
    id_source: &str,
) -> Result<Uuid, RefusDemande> {
    let user_ref = user_ref.map(str::trim).filter(|s| !s.is_empty());
    if let Some(r) = user_ref {
        if let Some(user) = ctx.comptes.get(r) {
            if let Ok(p) = personne_du_user(conn, ctx.etab, *user).await {
                return Ok(p);
            }
        }
    }
    let masque = masquer_adresse(email);
    let source_ref = match (user_ref, email.trim()) {
        (Some(r), _) => format!("user:{r}"),
        (None, e) if !e.is_empty() => {
            format!("ext:{}", &empreinte_hex(e.to_lowercase().as_bytes())[..16])
        }
        _ => format!("anonyme:{id_source}"),
    };
    let existant: Option<(Uuid,)> = sqlx::query_as(
        "SELECT personne_id FROM ent_dem_reprise_demandeur WHERE etablissement_id = $1 AND source_ref = $2",
    )
    .bind(ctx.etab)
    .bind(&source_ref)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some((p,)) = existant {
        return Ok(p);
    }
    let personne = creer_personne_reprise(conn, ctx.etab, NOM_PERSONNE_EXTERNE).await?;
    sqlx::query(
        "INSERT INTO ent_dem_reprise_demandeur (id, etablissement_id, source_ref, personne_id, etat, repere_masque)
         VALUES ($1, $2, $3, $4, 'a_rapprocher', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(ctx.etab)
    .bind(&source_ref)
    .bind(personne)
    .bind(masque)
    .execute(conn)
    .await?;
    Ok(personne)
}

// ───────────────────────── Outils communs aux reprises ─────────────────────────

fn scelle(decl: &NatureDecl) -> bool {
    !matches!(
        decl.confidentialite,
        Confidentialite::C0 | Confidentialite::C1
    )
}

fn tronquer(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

/// Jour d'établissement d'un horodatage (jamais une conversion de date du serveur).
async fn jour_de(
    conn: &mut PgConnection,
    etab: Uuid,
    t: DateTime<Utc>,
) -> Result<NaiveDate, RefusDemande> {
    let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(etab)
        .bind(t)
        .fetch_one(conn)
        .await?;
    Ok(j)
}

/// Résultat du routage d'une demande reprise : la file, le site et les délais.
struct Destination {
    file_id: Uuid,
    site_id: Option<Uuid>,
    service: String,
    delai_prise: u16,
    delai_traitement: u16,
    motif: &'static str,
    ref_id: Option<Uuid>,
}

async fn destination(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    demandeur: Uuid,
    nature_code: &str,
    tri: bool,
) -> Result<Option<Destination>, RefusDemande> {
    let decl = nature(nature_code).ok_or(RefusDemande::Indisponible)?;
    if tri {
        // Corbeille ou route inconnue : file de tri (§29.1). Jamais perdue, un humain décide.
        let g = charger_graphe(conn, ctx.etab, ctx.jour).await?;
        let tri = g
            .files
            .values()
            .find(|f| f.type_ == "tri" && f.site_id.is_none())
            .or_else(|| g.files.values().find(|f| f.type_ == "tri"));
        return Ok(tri.map(|f| Destination {
            file_id: f.id,
            site_id: f.site_id,
            service: f.service_calendrier.clone(),
            delai_prise: decl.delai_prise_jours,
            delai_traitement: decl.delai_traitement_jours,
            motif: "tri_du_site",
            ref_id: None,
        }));
    }
    let r = router_demande_existante(
        conn,
        ctx.etab,
        ctx.jour,
        Some(demandeur),
        nature_code,
        None,
        None,
        Urgence::Normale,
        &[],
    )
    .await?;
    Ok(r.resolution.map(|res| Destination {
        file_id: res.file.id,
        site_id: res.file.site_id,
        service: res.file.service_calendrier.clone(),
        delai_prise: res.delai_prise_jours,
        delai_traitement: res.delai_traitement_jours,
        motif: res.motif,
        ref_id: res.ref_id,
    }))
}

async fn numero_suivant(conn: &mut PgConnection, ctx: &Ctx<'_>) -> Result<String, RefusDemande> {
    let (n,): (i32,) = sqlx::query_as("SELECT ent_dem_numero_suivant($1, $2)")
        .bind(ctx.etab)
        .bind(ctx.annee_id)
        .fetch_one(conn)
        .await?;
    use chrono::Datelike;
    Ok(format!("D{}-{n:05}", ctx.annee_debut.year()))
}

/// Écrit la demande reprise (canal « reprise », déposant absent) et son fait d'origine. Aucun événement n'est publié :
/// R-DM49, un seul avis de reprise par personne, le jour d'ouverture.
#[allow(clippy::too_many_arguments)]
async fn inserer_demande(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    id: Uuid,
    decl: &NatureDecl,
    demandeur: Uuid,
    dest: &Destination,
    objet: &str,
    texte: Option<&str>,
    statut: &str,
    issue: Option<&str>,
    decision: Option<&str>,
    depose_le: Option<DateTime<Utc>>,
    clos_le: Option<DateTime<Utc>>,
    conservation: Option<NaiveDate>,
    agent: Option<Uuid>,
) -> Result<String, RefusDemande> {
    let numero = numero_suivant(conn, ctx).await?;
    let sceller = scelle(decl);
    let texte_stocke = match texte {
        Some(t) if sceller => Some(
            ChampsSensibles::depuis_env()
                .sceller("ent_dem_demande", "texte", id, t)
                .map_err(|_| RefusDemande::Indisponible)?,
        ),
        autre => autre.map(str::to_string),
    };
    let objet = if objet.trim().is_empty() {
        "(sans objet)".to_string()
    } else {
        tronquer(objet, 140)
    };
    sqlx::query(
        "INSERT INTO ent_dem_demande (id, etablissement_id, numero, annee_id, nature_code, nature_version, statut, issue,
                confidentialite, urgence, demandeur_personne_id, deposant_personne_id, canal_depot, site_id, objet, texte,
                texte_scelle, file_id, attribuee_a_agent_id, attribuee_le, depose_le, clos_le, decision, conservation_jusqu_au)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'normale', $10, NULL, 'reprise', $11, $12, $13, $14, $15, $16,
                 CASE WHEN $16::uuid IS NULL THEN NULL ELSE COALESCE($17, now()) END, $17, $18, $19, $20)",
    )
    .bind(id)
    .bind(ctx.etab)
    .bind(&numero)
    .bind(ctx.annee_id)
    .bind(decl.code)
    .bind(decl.version)
    .bind(statut)
    .bind(issue)
    .bind(decl.confidentialite.code())
    .bind(demandeur)
    .bind(dest.site_id)
    .bind(objet)
    .bind(texte_stocke.as_deref())
    .bind(sceller && texte_stocke.is_some())
    .bind(dest.file_id)
    .bind(agent)
    .bind(depose_le)
    .bind(clos_le)
    .bind(decision)
    .bind(conservation)
    .execute(&mut *conn)
    .await?;
    let mut f = FaitNouveau::nouveau(id, "reprise");
    f.auteur_tache = Some("reprise");
    f.statut_apres = Some(if statut == "prise" { "deposee" } else { statut });
    f.file_apres = Some(dest.file_id);
    f.motif = Some("reprise_ancienne_application");
    ajouter_fait(conn, ctx.etab, f).await?;
    // La raison du routage reste un fait lisible des membres de la file, jamais du demandeur (§10.1).
    let mut r = FaitNouveau::nouveau(id, "routage");
    r.auteur_tache = Some("routage_deterministe");
    r.file_apres = Some(dest.file_id);
    r.motif = Some(dest.motif);
    r.ref_id = dest.ref_id;
    ajouter_fait(conn, ctx.etab, r).await?;
    Ok(numero)
}

/// Échéances d'une demande reprise encore ouverte : le décompte repart du jour de la reprise (jours ouvrés du service).
async fn echeances_reprise(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    id: Uuid,
    dest: &Destination,
    avec_prise: bool,
) -> Result<(), RefusDemande> {
    let delai_traitement = dest.delai_traitement.max(dest.delai_prise);
    let mut a_poser = vec![("traitement", delai_traitement)];
    if avec_prise {
        a_poser.push(("prise_en_charge", dest.delai_prise));
    }
    for (type_, jours) in a_poser {
        let due = echeance_ouvree(conn, ctx.etab, ctx.jour, jours, &dest.service).await;
        sqlx::query(
            "INSERT INTO ent_dem_echeance (id, etablissement_id, demande_id, type, due_le)
             VALUES ($1, $2, $3, $4,
                     ($5::date + time '23:59:59') AT TIME ZONE COALESCE(
                        (SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $2), 'Europe/Paris'))",
        )
        .bind(Uuid::new_v4())
        .bind(ctx.etab)
        .bind(id)
        .bind(type_)
        .bind(due)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Message de reprise (interne par défaut). Texte scellé dès C2+, comme tout message de la demande.
#[allow(clippy::too_many_arguments)]
async fn inserer_message(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    demande: Uuid,
    decl: &NatureDecl,
    auteur: Uuid,
    visibilite: &str,
    texte: &str,
    cree_le: Option<DateTime<Utc>>,
) -> Result<Uuid, RefusDemande> {
    let id = Uuid::new_v4();
    let texte = tronquer(texte, 29_000);
    if texte.is_empty() {
        return Err(RefusDemande::Invalide("message vide"));
    }
    let (stocke, scelle_) = if scelle(decl) {
        (
            ChampsSensibles::depuis_env()
                .sceller("ent_dem_message", "texte", id, &texte)
                .map_err(|_| RefusDemande::Indisponible)?,
            true,
        )
    } else {
        (texte, false)
    };
    sqlx::query(
        "INSERT INTO ent_dem_message (id, etablissement_id, demande_id, auteur_personne_id, visibilite, genre, texte, texte_scelle, cree_le)
         VALUES ($1, $2, $3, $4, $5, 'echange', $6, $7, COALESCE($8, now()))",
    )
    .bind(id)
    .bind(ctx.etab)
    .bind(demande)
    .bind(auteur)
    .bind(visibilite)
    .bind(stocke)
    .bind(scelle_)
    .bind(cree_le)
    .execute(conn)
    .await?;
    Ok(id)
}

/// Copie une pièce au socle 7 par le port ; la correspondance dit ce qui n'a pas pu l'être.
#[allow(clippy::too_many_arguments)]
async fn reprendre_piece(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    source: &'static str,
    demande: Uuid,
    p: &PieceSource<'_>,
    role: &str,
    visibilite: &str,
    depose_par: Uuid,
) -> Result<Option<DocumentRepris>, RefusDemande> {
    let cle = format!("{}/{}", demande, p.origine_id);
    let existant: Option<(String,)> = sqlx::query_as(
        "SELECT decision FROM ent_dem_reprise_correspondance WHERE etablissement_id = $1 AND source = $2 AND source_id = $3",
    )
    .bind(ctx.etab)
    .bind(source)
    .bind(&cle)
    .fetch_optional(&mut *conn)
    .await?;
    if matches!(existant, Some((ref d,)) if d == "reprise") {
        return Ok(None);
    }
    match ctx.pieces.copier(ctx.etab, p) {
        Some(doc) => {
            if role == "fichier_a_imprimer" {
                sqlx::query(
                    "INSERT INTO ent_dem_repro_document (id, etablissement_id, demande_id, document_id, pages, depose_par_personne_id)
                     VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (etablissement_id, demande_id, document_id) DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(ctx.etab)
                .bind(demande)
                .bind(doc.document_id)
                .bind(doc.pages.unwrap_or(1).clamp(1, 2000))
                .bind(depose_par)
                .execute(&mut *conn)
                .await?;
            }
            sqlx::query(
                "INSERT INTO ent_dem_piece (id, etablissement_id, demande_id, document_id, role, visibilite, depose_par_personne_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (etablissement_id, demande_id, document_id) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(ctx.etab)
            .bind(demande)
            .bind(doc.document_id)
            .bind(role)
            .bind(visibilite)
            .bind(depose_par)
            .execute(&mut *conn)
            .await?;
            noter(conn, ctx, source, &cle, &Sortie::reprise(doc.document_id)).await?;
            Ok(Some(doc))
        }
        None => {
            noter(
                conn,
                ctx,
                source,
                &cle,
                &Sortie::a_verifier("socle_7_absent_ou_piece_illisible"),
            )
            .await?;
            Ok(None)
        }
    }
}

// ───────────────────────── Reprise d'une demande (request) ─────────────────────────

/// Valeur typée d'un attribut du Next.js quand le champ est déclaré par la nature (§29.1) ; sinon `None`.
fn valeur_typee(decl: &NatureDecl, a: &AttrNext) -> Option<ValeurSaisie> {
    let chemin = a.path.trim();
    let champ = decl.champs.iter().find(|c| c.code == chemin)?;
    let v = a.value.trim();
    let valeur = match champ.type_ {
        ChampType::Texte { max } => Valeur::Texte(tronquer(v, max.min(2000))),
        ChampType::Entier { min, max } => {
            Valeur::Entier(v.parse::<i64>().ok().filter(|n| (min..=max).contains(n))?)
        }
        ChampType::Bool => Valeur::Bool(match v.to_lowercase().as_str() {
            "true" | "oui" | "1" => true,
            "false" | "non" | "0" => false,
            _ => return None,
        }),
        ChampType::Liste(l) => Valeur::Code(l.iter().find(|x| **x == v)?.to_string()),
        ChampType::Date => Valeur::Date(NaiveDate::parse_from_str(v, "%Y-%m-%d").ok()?),
        _ => return None,
    };
    Some(ValeurSaisie {
        champ: champ.code.to_string(),
        rang: 0,
        valeur,
    })
}

async fn reprendre_requete(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    r: &RequeteNext,
) -> Result<Sortie, RefusDemande> {
    let ouverte = matches!(r.status.as_str(), "NOUVELLE" | "EN_COURS" | "EN_ATTENTE");
    let close = r.status == "TERMINEE";
    if !ouverte && !close {
        return Ok(Sortie::a_verifier("statut_inconnu"));
    }
    let purge = r.purge_at.as_deref().and_then(horodatage);
    let mut conservation = None;
    if close {
        // Archive jusqu'à `purge_at` : pas de prolongation. Sans date, on ne devine rien (à vérifier) ; déjà échue :
        // inutile de la reprendre.
        let Some(p) = purge else {
            return Ok(Sortie::a_verifier("purge_at_absente"));
        };
        let j = jour_de(conn, ctx.etab, p).await?;
        if j <= ctx.jour {
            return Ok(Sortie::ignoree("purge_deja_echue"));
        }
        conservation = Some(j);
    }
    let route = r
        .assigned_route_id
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&r.assigned_unit);
    let (nature_code, tri) = nature_depuis_route(route);
    let decl = nature(nature_code).ok_or(RefusDemande::Indisponible)?;
    let demandeur = demandeur_de_reprise(
        conn,
        ctx,
        r.requester_user_id.as_deref(),
        &r.requester_email,
        &r.id,
    )
    .await?;
    let Some(dest) = destination(conn, ctx, demandeur, nature_code, tri).await? else {
        return Ok(Sortie::a_verifier("aucune_file"));
    };
    // Prise en charge reprise si l'agent est retrouvé ; sinon la demande est remise en file (jamais « perdue »).
    let agent = if r.status == "EN_COURS" {
        agent_par_email(conn, ctx.etab, &r.assigned_email).await?
    } else {
        None
    };
    let statut = if close {
        "close"
    } else {
        match r.status.as_str() {
            "EN_ATTENTE" => "en_attente_tiers",
            _ if agent.is_some() => "prise",
            _ => "deposee",
        }
    };
    let depose = horodatage(&r.created_at);
    let cloture = close
        .then(|| horodatage(&r.updated_at).or(depose))
        .flatten();
    let id = Uuid::new_v4();
    // Le texte libre : dans la demande si la nature en accepte, sinon dans une note interne.
    let description = r.description.trim();
    let (texte, note_description) = match decl.texte_max {
        Some(max) if !description.is_empty() => {
            (Some(tronquer(description, max.min(20_000))), None)
        }
        None if !description.is_empty() => (None, Some(description)),
        _ => (None, None),
    };
    let numero = inserer_demande(
        conn,
        ctx,
        id,
        decl,
        demandeur,
        &dest,
        &r.subject,
        texte.as_deref(),
        statut,
        close.then_some("traitee"),
        None,
        depose,
        cloture,
        conservation,
        agent,
    )
    .await?;
    let _ = numero;
    if let Some(a) = agent {
        let mut f = FaitNouveau::nouveau(id, "prise");
        f.auteur_tache = Some("reprise");
        f.statut_avant = Some("deposee");
        f.statut_apres = Some("prise");
        f.agent_apres = Some(a);
        f.motif = Some("reprise_ancienne_application");
        ajouter_fait(conn, ctx.etab, f).await?;
    }
    // Historique : un fait `reprise` par ligne (code seulement, jamais le texte), dans l'ordre chronologique.
    let mut histo: Vec<&HistoriqueNext> = r.history.iter().collect();
    histo.sort_by_key(|h| horodatage(&h.at));
    for h in histo.into_iter().take(200) {
        let code = code_depuis(&h.action, "historique_repris");
        let mut f = FaitNouveau::nouveau(id, "reprise_historique");
        f.auteur_tache = Some("reprise");
        f.motif = Some(&code);
        ajouter_fait(conn, ctx.etab, f).await?;
    }
    if close {
        let mut f = FaitNouveau::nouveau(id, "cloture");
        f.auteur_tache = Some("reprise");
        f.statut_avant = Some("deposee");
        f.statut_apres = Some("close");
        f.motif = Some("reprise_archive");
        ajouter_fait(conn, ctx.etab, f).await?;
    } else {
        echeances_reprise(conn, ctx, id, &dest, statut == "deposee").await?;
    }
    // Valeurs typées, et le reste en une note interne.
    let mut notes: Vec<String> = Vec::new();
    let mut vues: BTreeSet<String> = BTreeSet::new();
    for a in &r.attrs {
        match valeur_typee(decl, a) {
            Some(v) if vues.insert(v.champ.clone()) => {
                inserer_valeur(conn, ctx.etab, id, &v).await?
            }
            _ => {
                if !a.path.trim().is_empty() && !a.value.trim().is_empty() {
                    notes.push(format!(
                        "{} : {}",
                        tronquer(&a.path, 80),
                        tronquer(&a.value, 300)
                    ));
                }
            }
        }
    }
    let technique = personne_reprise(conn, ctx.etab).await?;
    if let Some(d) = note_description {
        inserer_message(
            conn,
            ctx,
            id,
            decl,
            technique,
            "interne",
            &format!("Description d'origine : {d}"),
            depose,
        )
        .await?;
    }
    if !notes.is_empty() {
        let corps = format!(
            "Note de reprise (champs de l'ancienne application) : {}",
            notes.join(" ; ")
        );
        inserer_message(conn, ctx, id, decl, technique, "interne", &corps, depose).await?;
    }
    for c in r.comments.iter().take(500) {
        if c.body.trim().is_empty() {
            continue;
        }
        let auteur = match c.by_user_id.as_deref().and_then(|u| ctx.comptes.get(u)) {
            Some(user) => personne_du_user(conn, ctx.etab, *user)
                .await
                .unwrap_or(technique),
            None => technique,
        };
        // Un commentaire n'est visible du demandeur que si l'export le dit ; le reste est interne.
        let visibilite = if c.visible_demandeur {
            "demandeur"
        } else {
            "interne"
        };
        let corps = format!("[{}] {}", tronquer(&c.by_name, 80), c.body.trim());
        inserer_message(
            conn,
            ctx,
            id,
            decl,
            auteur,
            visibilite,
            &corps,
            horodatage(&c.at),
        )
        .await?;
    }
    for p in r.attachments.iter().take(100) {
        let visibilite = if p.comment_id.as_deref().is_none_or(str::is_empty) {
            "demandeur"
        } else {
            "interne"
        };
        let depose_par = if visibilite == "demandeur" {
            demandeur
        } else {
            technique
        };
        let src = PieceSource {
            origine_id: &p.id,
            nom_fichier: &p.file_name,
            cle_stockage: &p.s3_key,
            type_contenu: p.content_type.as_deref(),
        };
        reprendre_piece(
            conn,
            ctx,
            "request_piece",
            id,
            &src,
            "annexe",
            visibilite,
            depose_par,
        )
        .await?;
    }
    Ok(if close {
        Sortie::archive(id, Some("close_archivee"))
    } else {
        Sortie::reprise(id)
    })
}

// ───────────────────────── Reprise d'une HSE ─────────────────────────

async fn reprendre_hse(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    h: &HseNext,
) -> Result<Sortie, RefusDemande> {
    let creee = horodatage(&h.created_at);
    // Année en cours seulement (T0) : une demande d'une année précédente n'est pas reprise.
    if let Some(c) = creee {
        if jour_de(conn, ctx.etab, c).await? < ctx.annee_debut {
            return Ok(Sortie::ignoree("annee_precedente"));
        }
    }
    if !matches!(
        h.status.as_str(),
        "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE" | "ANNULEE"
    ) {
        return Ok(Sortie::a_verifier("statut_inconnu"));
    }
    let Some(heures) = h.nombre_heures.filter(|n| n.is_finite() && *n > 0.0) else {
        return Ok(Sortie::a_verifier("volume_absent"));
    };
    let quarts = (heures * 4.0).round();
    if !(1.0..=1600.0).contains(&quarts) {
        return Ok(Sortie::a_verifier("volume_invalide"));
    }
    let quarts = quarts as i32;
    // Le bénéficiaire est l'AGENT du demandeur : table de passage, à défaut l'adresse de sa fiche (personnels seulement).
    let user_de_passage = ctx.comptes.get(h.created_by.user_id.trim()).copied();
    let mut demandeur = None;
    if let Some(u) = user_de_passage {
        demandeur = personne_du_user(conn, ctx.etab, u).await.ok();
    }
    let agent: Option<(Uuid, Uuid)> = match demandeur {
        Some(p) => {
            sqlx::query_as("SELECT id, personne_id FROM ent_agent WHERE etablissement_id = $1 AND personne_id = $2 ORDER BY cree_le LIMIT 1")
                .bind(ctx.etab)
                .bind(p)
                .fetch_optional(&mut *conn)
                .await?
        }
        None => match agent_par_email(conn, ctx.etab, &h.created_by.email).await? {
            Some(a) => {
                sqlx::query_as("SELECT id, personne_id FROM ent_agent WHERE etablissement_id = $1 AND id = $2")
                    .bind(ctx.etab)
                    .bind(a)
                    .fetch_optional(&mut *conn)
                    .await?
            }
            None => None,
        },
    };
    let Some((agent_id, personne_agent)) = agent else {
        return Ok(Sortie::a_verifier("agent_introuvable"));
    };
    let decl = nature("rh.hse").ok_or(RefusDemande::Indisponible)?;
    let Some(dest) = destination(conn, ctx, personne_agent, "rh.hse", false).await? else {
        return Ok(Sortie::a_verifier("aucune_file"));
    };
    let ouverte = h.status == "EN_ATTENTE";
    let (statut, issue, decision): (&str, Option<&str>, Option<&str>) = match h.status.as_str() {
        "EN_ATTENTE" => ("en_validation", None, None),
        "ACCEPTEE" => ("close", Some("accordee"), Some("accordee")),
        "REFUSEE" => ("close", Some("refusee"), Some("refusee")),
        _ => ("annulee", Some("annulee_par_demandeur"), None),
    };
    let decide_le = h.decided_at.as_deref().and_then(horodatage);
    let cloture = (!ouverte)
        .then(|| decide_le.or_else(|| horodatage(&h.updated_at)).or(creee))
        .flatten();
    let id = Uuid::new_v4();
    let texte = {
        let t = format!("{} {}", h.motif.trim(), h.details.trim());
        let t = t.trim();
        (!t.is_empty()).then(|| tronquer(t, decl.texte_max.unwrap_or(2000).min(20_000)))
    };
    inserer_demande(
        conn,
        ctx,
        id,
        decl,
        personne_agent,
        &dest,
        &h.resume_demande,
        texte.as_deref(),
        statut,
        issue,
        decision,
        creee,
        cloture,
        None,
        None,
    )
    .await?;
    // Période : du jour de la demande (au plus tôt le début de l'année) à la fin de l'année scolaire.
    let du = match creee {
        Some(c) => jour_de(conn, ctx.etab, c).await?.max(ctx.annee_debut),
        None => ctx.annee_debut,
    };
    // La décision n'est écrite que si son auteur est retrouvé : on n'attribue jamais une décision à quelqu'un d'autre.
    let decideur: Option<Uuid> = match h
        .decided_by
        .as_ref()
        .and_then(|d| ctx.comptes.get(d.user_id.trim()))
    {
        Some(u) if !ouverte && h.status != "ANNULEE" => {
            personne_du_user(conn, ctx.etab, *u).await.ok()
        }
        _ => None,
    };
    let decide = decideur.filter(|d| *d != personne_agent && cloture.is_some());
    sqlx::query(
        "INSERT INTO ent_dem_hse (etablissement_id, demande_id, annee_id, site_id, beneficiaire_agent_id, motif, type_heure,
                volume_demande_quarts, volume_accorde_quarts, periode_du, periode_au, lieu_libelle, decision, decide_par_personne_id, decide_le)
         VALUES ($1, $2, $3, $4, $5, 'autre', 'hse_autre', $6, $7, $8, $9, NULL, $10, $11, $12)",
    )
    .bind(ctx.etab)
    .bind(id)
    .bind(ctx.annee_id)
    .bind(dest.site_id)
    .bind(agent_id)
    .bind(quarts)
    .bind(if decide.is_some() && h.status == "ACCEPTEE" { Some(quarts) } else { None })
    .bind(du)
    .bind(ctx.annee_fin.max(du))
    .bind(decide.and(decision))
    .bind(decide)
    .bind(decide.and(cloture))
    .execute(&mut *conn)
    .await?;
    if let (Some(d), Some(dec)) = (decide, decision) {
        sqlx::query(
            "INSERT INTO ent_dem_hse_decision (id, etablissement_id, demande_id, rang, decision, volume_quarts, motif_code,
                    avenant, depassement_enveloppe, decide_par_personne_id, decide_le)
             VALUES ($1, $2, $3, 0, $4, $5, 'reprise_ancienne_application', false, false, $6, COALESCE($7, now()))",
        )
        .bind(Uuid::new_v4())
        .bind(ctx.etab)
        .bind(id)
        .bind(dec)
        .bind(if dec == "refusee" { 0 } else { quarts })
        .bind(d)
        .bind(cloture)
        .execute(&mut *conn)
        .await?;
    }
    if !h.classe.trim().is_empty() || !h.resume_demande.trim().is_empty() {
        let technique = personne_reprise(conn, ctx.etab).await?;
        if !h.classe.trim().is_empty() {
            inserer_message(
                conn,
                ctx,
                id,
                decl,
                technique,
                "interne",
                &format!(
                    "Classe indiquée dans l'ancienne application : {}",
                    tronquer(&h.classe, 200)
                ),
                creee,
            )
            .await?;
        }
    }
    let mut f = FaitNouveau::nouveau(
        id,
        if ouverte {
            "reprise_hse_en_attente"
        } else {
            "reprise_hse_decidee"
        },
    );
    f.auteur_tache = Some("reprise");
    f.statut_avant = Some("deposee");
    f.statut_apres = Some(statut);
    f.motif = Some(match h.status.as_str() {
        "ACCEPTEE" => "hse_transmise_hors_scolia",
        "EN_ATTENTE" => "decision_a_refaire",
        _ => "reprise_archive",
    });
    ajouter_fait(conn, ctx.etab, f).await?;
    if ouverte {
        echeances_reprise(conn, ctx, id, &dest, true).await?;
    }
    // Le PDF d'acceptation est une pièce historique du coffre de l'agent (socle 7) : copié par le port, sinon signalé.
    if let Some(chemin) = h
        .acceptance_pdf_path
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        let technique = personne_reprise(conn, ctx.etab).await?;
        let src = PieceSource {
            origine_id: &h.id,
            nom_fichier: "acceptation.pdf",
            cle_stockage: chemin,
            type_contenu: Some("application/pdf"),
        };
        reprendre_piece(
            conn,
            ctx,
            "hse_piece",
            id,
            &src,
            "accord",
            "interne",
            technique,
        )
        .await?;
    }
    Ok(match h.status.as_str() {
        "EN_ATTENTE" => Sortie::reprise(id),
        // CDM-50 : déjà envoyée au rectorat depuis le Next.js ; aucune séance ne sera jamais possible (déclencheur 3049).
        "ACCEPTEE" => Sortie::archive(id, Some("hse_transmise_hors_scolia")),
        _ => Sortie::archive(id, Some("close_archivee")),
    })
}

// ───────────────────────── Reprise d'une demande de photocopies ─────────────────────────

async fn reprendre_photocopie(
    conn: &mut PgConnection,
    ctx: &Ctx<'_>,
    p: &PhotocopieNext,
) -> Result<Sortie, RefusDemande> {
    // Ouvertes seulement : en attente, acceptées non prêtes (T0 : le reste est perdable).
    if !matches!(p.status.as_str(), "EN_ATTENTE" | "ACCEPTEE") {
        return Ok(Sortie::ignoree("non_ouverte"));
    }
    if p.nombre_photocopies < 1 || p.nombre_photocopies > 1_000_000 {
        return Ok(Sortie::a_verifier("quantite_invalide"));
    }
    let couleur = !p
        .type_impression
        .as_deref()
        .is_some_and(|t| t.eq_ignore_ascii_case("NOIR_BLANC"));
    let decl = nature("reprographie.tirage").ok_or(RefusDemande::Indisponible)?;
    let demandeur = demandeur_de_reprise(
        conn,
        ctx,
        Some(p.created_by.user_id.as_str()),
        &p.created_by.email,
        &p.id,
    )
    .await?;
    let Some(dest) = destination(conn, ctx, demandeur, "reprographie.tirage", false).await? else {
        return Ok(Sortie::a_verifier("aucune_file"));
    };
    // Le site de la demande : celui de la file, à défaut l'unique site de l'établissement. Sinon, on ne devine pas.
    let site = match dest.site_id {
        Some(s) => s,
        None => {
            let sites: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_site WHERE etablissement_id = $1 ORDER BY id LIMIT 2",
            )
            .bind(ctx.etab)
            .fetch_all(&mut *conn)
            .await?;
            match sites.as_slice() {
                [(s,)] => *s,
                _ => return Ok(Sortie::a_verifier("site_introuvable")),
            }
        }
    };
    let dest = Destination {
        site_id: Some(site),
        ..dest
    };
    let creee = horodatage(&p.created_at);
    let decidee = p.decided_at.as_deref().and_then(horodatage);
    let id = Uuid::new_v4();
    let validation = couleur;
    let (statut, etat) = if p.status == "EN_ATTENTE" && validation {
        ("en_validation", "a_valider")
    } else {
        ("deposee", "a_imprimer")
    };
    let objet = if p.motif.trim().is_empty() {
        "Photocopies".to_string()
    } else {
        p.motif.clone()
    };
    let texte =
        (!p.classes_ou_matiere.trim().is_empty()).then(|| tronquer(&p.classes_ou_matiere, 2000));
    inserer_demande(
        conn,
        ctx,
        id,
        decl,
        demandeur,
        &dest,
        &objet,
        texte.as_deref().filter(|_| decl.texte_max.is_some()),
        statut,
        None,
        None,
        creee,
        None,
        None,
        None,
    )
    .await?;
    let copies = i32::try_from(p.nombre_photocopies).unwrap_or(1_000_000);
    let exemplaires = copies.clamp(1, 5000);
    sqlx::query(
        "INSERT INTO ent_dem_repro (etablissement_id, demande_id, site_id, couleur, exemplaires_total, pages_total, copies_total,
                pour_le, etat, validation_requise, valide_le)
         VALUES ($1, $2, $3, $4, $5, 1, $6, $7, $8, $9,
                 CASE WHEN $9 AND $8 <> 'a_valider' THEN COALESCE($10, now()) END)",
    )
    .bind(ctx.etab)
    .bind(id)
    .bind(site)
    .bind(couleur)
    .bind(exemplaires)
    .bind(copies)
    .bind(ctx.jour)
    .bind(etat)
    .bind(validation)
    .bind(decidee.or(creee))
    .execute(&mut *conn)
    .await?;
    let mut f = FaitNouveau::nouveau(id, "reprise_photocopies");
    f.auteur_tache = Some("reprise");
    f.statut_apres = Some(statut);
    f.motif = Some("reprise_ancienne_application");
    ajouter_fait(conn, ctx.etab, f).await?;
    echeances_reprise(conn, ctx, id, &dest, true).await?;
    let technique = personne_reprise(conn, ctx.etab).await?;
    inserer_message(
        conn, ctx, id, decl, technique, "interne",
        "Note de reprise : le nombre de pages n'existe pas dans l'ancienne application (1 page comptée par défaut).", creee,
    )
    .await?;
    for d in p.documents.iter().take(5) {
        let src = PieceSource {
            origine_id: &d.key,
            nom_fichier: &d.file_name,
            cle_stockage: &d.key,
            type_contenu: d.content_type.as_deref(),
        };
        reprendre_piece(
            conn,
            ctx,
            "photocopies_piece",
            id,
            &src,
            "fichier_a_imprimer",
            "demandeur",
            demandeur,
        )
        .await?;
    }
    Ok(Sortie::reprise(id))
}
