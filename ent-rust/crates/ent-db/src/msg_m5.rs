//! Messagerie, lot M5 (`docs/socles/16_MESSAGERIE.md` §12, §26 ; R-M31 à R-M33 ; CM-57 à CM-60, D16) : pièces jointes.
//!
//! * **Quarantaine** : une pièce déposée naît `en_quarantaine`. Son **original** (`storage_key`) est scellé et
//!   jamais servi ; elle n'est servie qu'une fois **analysée** (antivirus du socle 7, `ent-object::clamav`, fermé par
//!   défaut) puis **normalisée** (PDF désarmé, JPEG sans EXIF ni GPS, PNG sans blocs annexes : D-005, `ent-pdf`).
//!   Sans démon antivirus, rien ne sort de quarantaine (jamais d'antivirus « factice » ni de repli).
//! * **Stockage objet** : jamais de `BYTEA` ; la reprise des pièces historiques réutilise `reprise_bytea.rs`
//!   (idempotente, rejouable) et y ajoute l'analyse a posteriori et un rapport ([`PgEnt::reprendre_et_analyser_pieces_messagerie`]).
//! * **Téléchargement** : droits relus **au moment du téléchargement**, lien pré-signé de 60 s seulement après contrôle,
//!   jamais d'URL durable ; un participant retiré du fil ne télécharge plus rien ; **404 uniforme** pour qui n'a pas le
//!   droit de lire le fil (aucun indice sur l'existence d'une pièce) ; la pièce d'un message retiré est conservée mais
//!   plus servie (R-M32) ; chaque lien et chaque téléchargement laissent une ligne dans `ent_msg_piece_acces`.
//! * **Carte « document »** (R-M33) : un document du coffre (socle 7) se joint par carte, jamais par fichier ; le droit
//!   **du lecteur** est relu à chaque lecture (auprès du socle 7).

#![allow(clippy::too_many_arguments)]

use std::sync::Arc;

use domain_eleve::{DomainError, MsgPiece};
use ent_auth::Role;
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_object::presign::Disposition;
use ent_object::upload::{AnalyseurAntivirus, ContexteAnalyse, TypeFichier, UsageFichier, VerdictAnalyse};
use ent_object::ObjectStore;
use serde::Serialize;
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::documents::Domaine;
use crate::documents_coffre::FiltreCoffre;
use crate::objets::enregistrer_objet;
use crate::ref_commun::Acteur;
use crate::reprise_bytea::{OptionsReprise, RapportReprise, TablePiece};
use crate::service::{EntDbError, LienFichier, PgEnt, RefusMetier};
use crate::tenant_tx::{lire_par_etablissement, ContexteSysteme, TenantTx};

/// Espace des fichiers **servis** dans le magasin d'objets (distinct de `messages/` qui garde les originaux).
pub const ESPACE_SERVI: &str = "messages-servis";
/// Plus de cartes « document » par message que de pièces (§12 point 3 : 5 par message).
pub const CARTES_MAX_PAR_MESSAGE: i64 = 5;
/// Indisponibilités consécutives de l'antivirus après lesquelles une passe s'arrête (un démon éteint ne coûte pas
/// trente secondes par pièce).
pub const INDISPONIBLES_AVANT_ARRET: usize = 3;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

/// 404 uniforme : la pièce, le message ou le fil n'existent pas **pour ce compte** (aucun indice).
fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::MsgPieceIntrouvable)
}

/// Durée maximale d'un lien vers une pièce de messagerie, quelle que soit `ENT_OBJECT_LINK_TTL_SECS` (M8).
pub const DUREE_LIEN_PIECE_SECS: u64 = 60;

/// Clé d'un fichier servi : `{établissement}/messages-servis/{uuid}.{ext}` (jamais le nom du fichier).
pub fn cle_servie_est_a_moi(etab: Uuid, cle: &str) -> bool {
    let prefixe = format!("{etab}/{ESPACE_SERVI}/");
    match cle.strip_prefix(&prefixe) {
        Some(reste) => {
            !reste.is_empty() && !reste.contains('/') && !reste.contains("..") && reste.len() <= 64
        }
        None => false,
    }
}

// ───────────────────────── Normalisation (D-005) ─────────────────────────

/// Blocs PNG conservés : le rendu (`IHDR`, `PLTE`, `tRNS`, `IDAT`, `IEND`) et la colorimétrie / la densité.
/// Tout le reste (texte, `eXIf`, `tIME`, profil ICC, blocs privés) est jeté : aucune métadonnée ne quitte l'établissement.
const BLOCS_PNG_CONSERVES: [&[u8; 4]; 10] = [
    b"IHDR", b"PLTE", b"tRNS", b"IDAT", b"IEND", b"gAMA", b"sRGB", b"cHRM", b"pHYs", b"sBIT",
];

/// PNG sans blocs annexes. Refuse un PNG mal formé (longueur incohérente, `IHDR` pas en tête, `IEND` absent).
pub fn png_nettoye(entree: &[u8]) -> Result<Vec<u8>, &'static str> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if entree.len() < 8 + 12 || entree[..8] != SIGNATURE {
        return Err("image PNG illisible");
    }
    let mut sortie = SIGNATURE.to_vec();
    let mut pos = 8usize;
    let mut premier = true;
    let mut fini = false;
    while pos < entree.len() {
        if pos + 12 > entree.len() {
            return Err("image PNG tronquée");
        }
        let longueur = u32::from_be_bytes([entree[pos], entree[pos + 1], entree[pos + 2], entree[pos + 3]]) as usize;
        let fin = pos
            .checked_add(12)
            .and_then(|x| x.checked_add(longueur))
            .ok_or("image PNG illisible")?;
        if fin > entree.len() {
            return Err("image PNG tronquée");
        }
        let genre: [u8; 4] = [entree[pos + 4], entree[pos + 5], entree[pos + 6], entree[pos + 7]];
        if premier && &genre != b"IHDR" {
            return Err("image PNG illisible");
        }
        premier = false;
        if BLOCS_PNG_CONSERVES.iter().any(|b| **b == genre) {
            sortie.extend_from_slice(&entree[pos..fin]);
        }
        pos = fin;
        if &genre == b"IEND" {
            fini = true;
            break;
        }
    }
    if !fini {
        return Err("image PNG tronquée");
    }
    Ok(sortie)
}

/// Ce qu'une analyse décide pour une pièce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueAnalyse {
    /// Sortie de quarantaine : le fichier servi est écrit.
    Saine,
    /// L'antivirus ne répond pas : la pièce reste en quarantaine (réessai à la prochaine passe).
    EnQuarantaine,
    Infectee,
    /// Refusée pour sa forme (PDF protégé ou non inspectable, format actif, image illisible).
    Rejetee(String),
    /// Une autre passe l'avait déjà traitée.
    DejaTraitee,
    /// Pas encore d'objet : le contenu est encore en base (la reprise doit passer d'abord).
    AReprendre,
}

/// Bilan d'une passe d'analyse.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BilanAnalysePieces {
    pub saines: u64,
    pub en_quarantaine: u64,
    pub infectees: u64,
    pub rejetees: u64,
    pub a_reprendre: u64,
    /// Pièces dont l'analyse a échoué (stockage indisponible, objet absent) : laissées en quarantaine, réessayées à
    /// la passe suivante ; une pièce en échec ne bloque jamais les autres.
    pub en_echec: u64,
    /// La passe s'est arrêtée parce que l'antivirus ne répondait pas.
    pub antivirus_indisponible: bool,
}

/// Durée de conservation de l'original d'une pièce de messagerie **infectée** (catégorie `msg.piece_refusee`).
pub const RETENTION_MSG_INFECTEES_JOURS: i64 = 7;
/// Durée de conservation de l'original d'une pièce de messagerie **rejetée** (forme, protégée, active).
pub const RETENTION_MSG_REJETEES_JOURS: i64 = 30;

fn jours_depuis_env(nom: &str, defaut: i64) -> i64 {
    std::env::var(nom)
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|j| (0..=365).contains(j))
        .unwrap_or(defaut)
}

/// `(infectées, rejetées)` : `ENT_MSG_PIECES_INFECTEES_RETENTION_JOURS` et `ENT_MSG_PIECES_REJETEES_RETENTION_JOURS`
/// (0 à 365 jours), sinon 7 et 30 jours.
pub fn retention_pieces_msg_depuis_env() -> (i64, i64) {
    (
        jours_depuis_env("ENT_MSG_PIECES_INFECTEES_RETENTION_JOURS", RETENTION_MSG_INFECTEES_JOURS),
        jours_depuis_env("ENT_MSG_PIECES_REJETEES_RETENTION_JOURS", RETENTION_MSG_REJETEES_JOURS),
    )
}

/// Bilan de la purge des pièces de messagerie refusées.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BilanPurgeMsg {
    pub purges: usize,
    pub echecs: usize,
}

/// Rapport de la reprise des pièces historiques et de leur analyse a posteriori (§26).
#[derive(Debug, Clone)]
pub struct RapportRepriseM5 {
    /// Rapport de la reprise `BYTEA` → objet (copies, vérifications, échecs, restantes).
    pub reprise: RapportReprise,
    pub analyse: BilanAnalysePieces,
    /// Pièces encore en quarantaine à la fin (analyse impossible ou pas encore faite).
    pub restent_en_quarantaine: i64,
}

#[derive(Debug, FromRow)]
struct LignePiece {
    fil_id: Uuid,
    message_id: Uuid,
    nom: String,
    mime_type: String,
    etat: String,
    created_at: chrono::DateTime<chrono::Utc>,
    storage_key: Option<String>,
    sha256: Option<String>,
    cle_objet_servi: Option<String>,
    sha256_servi: Option<String>,
    mime_servi: Option<String>,
    taille_servie: Option<i32>,
    retire: bool,
    created_by_user_id: Uuid,
}

const COLONNES_PIECE: &str = "p.fil_id, p.message_id, p.nom, p.mime_type, p.etat, p.created_at, p.storage_key, p.sha256,
    p.cle_objet_servi, p.sha256_servi, p.mime_servi, p.taille_servie,
    (m.retire_le IS NOT NULL) AS retire, p.created_by_user_id";

/// Ce qui se sert d'une pièce saine.
struct PieceAServir {
    id: Uuid,
    fil_id: Uuid,
    message_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    nom: String,
    mime: String,
    taille: i32,
    cle: String,
    sha256: String,
    created_by_user_id: Uuid,
}

impl PgEnt {
    async fn ligne_piece(&self, etab: Uuid, piece_id: Uuid) -> Result<Option<LignePiece>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l = sqlx::query_as::<_, LignePiece>(&format!(
            "SELECT {COLONNES_PIECE}
               FROM ent_msg_piece p
               JOIN ent_msg m ON m.etablissement_id = p.etablissement_id AND m.id = p.message_id
              WHERE p.etablissement_id = $1 AND p.id = $2"
        ))
        .bind(etab)
        .bind(piece_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l)
    }

    /// Contrôle commun du téléchargement et du lien (une seule porte, R-M2) : le fil est lisible **maintenant** par
    /// ce compte (un participant retiré n'y accède plus), le message est remis à ce lecteur et n'est pas retiré,
    /// puis la pièce est saine. Tout refus de droit répond « introuvable » (404 uniforme) ; seul un participant
    /// légitime apprend l'état de la pièce.
    async fn piece_a_servir(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<PieceAServir, EntDbError> {
        let ligne = self.ligne_piece(etab, piece_id).await?.ok_or_else(introuvable)?;
        let fil = self
            .messages
            .get_fil(etab, ligne.fil_id)
            .await?
            .ok_or_else(introuvable)?;
        if !self.actor_can_access_fil(etab, user, role, &fil).await? {
            return Err(introuvable());
        }
        if !self.messages.message_remis_pour(etab, ligne.message_id, user).await? {
            return Err(introuvable());
        }
        // R-M32 : la pièce d'un message retiré est conservée, plus servie.
        if ligne.retire {
            return Err(introuvable());
        }
        match ligne.etat.as_str() {
            "saine" => {}
            "en_quarantaine" => {
                return Err(EntDbError::Refus(RefusMetier::Conflit, "pièce en cours d'analyse"));
            }
            _ => return Err(EntDbError::Refus(RefusMetier::Conflit, "pièce refusée")),
        }
        let (Some(cle), Some(sha), Some(mime), Some(taille)) = (
            ligne.cle_objet_servi,
            ligne.sha256_servi,
            ligne.mime_servi,
            ligne.taille_servie,
        ) else {
            return Err(EntDbError::FichierAbsent);
        };
        if !cle_servie_est_a_moi(etab, &cle) {
            return Err(EntDbError::FichierAbsent);
        }
        Ok(PieceAServir {
            id: piece_id,
            fil_id: ligne.fil_id,
            message_id: ligne.message_id,
            created_at: ligne.created_at,
            nom: ligne.nom,
            mime,
            taille,
            cle,
            sha256: sha,
            created_by_user_id: ligne.created_by_user_id,
        })
    }

    async fn journaliser_acces_piece(
        &self,
        etab: Uuid,
        user: Uuid,
        piece: &PieceAServir,
        via: &str,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_msg_piece_acces (id, etablissement_id, piece_id, fil_id, user_id, via)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(piece.id)
        .bind(piece.fil_id)
        .bind(user)
        .bind(via)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Téléchargement direct d'une pièce **saine** : octets du fichier **servi**, empreinte relue deux fois
    /// (`ent_objet` et la ligne). Journalisé.
    pub async fn telecharger_piece_for_actor(
        &self,
        etab: Uuid,
        actor_user_id: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<MsgPiece, EntDbError> {
        let p = self.piece_a_servir(etab, actor_user_id, role, piece_id).await?;
        let octets = self.lire_objet_empreinte(etab, &p.cle, Some(&p.sha256)).await?;
        self.journaliser_acces_piece(etab, actor_user_id, &p, "telechargement")
            .await?;
        Ok(MsgPiece {
            id: p.id,
            etablissement_id: etab,
            fil_id: p.fil_id,
            message_id: p.message_id,
            nom: p.nom,
            mime_type: p.mime,
            taille_octets: p.taille,
            contenu: octets,
            storage_key: Some(p.cle),
            sha256: Some(p.sha256),
            created_by_user_id: p.created_by_user_id,
            created_at: p.created_at,
            etat: "saine".into(),
        })
    }

    /// **Ouverture** d'un lien (`GET /fichiers/{jeton}`) vers un fichier servi de messagerie : les droits sont **relus**
    /// à cet instant (un participant retiré entre-temps reçoit 404) et l'ouverture est journalisée à côté de
    /// l'émission. Une clé qui n'est pas celle d'un fichier servi de messagerie n'est pas concernée (`Ok(false)`).
    pub async fn controler_ouverture_piece_message(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        cle: &str,
    ) -> Result<bool, EntDbError> {
        if !cle_servie_est_a_moi(etab, cle) {
            return Ok(false);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_msg_piece WHERE etablissement_id = $1 AND cle_objet_servi = $2 LIMIT 1",
        )
        .bind(etab)
        .bind(cle)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let piece_id = id.ok_or_else(introuvable)?;
        let p = self.piece_a_servir(etab, user, role, piece_id).await?;
        self.journaliser_acces_piece(etab, user, &p, "ouverture").await?;
        Ok(true)
    }

    /// Lien (60 s au plus, `Content-Disposition: attachment`) vers le fichier **servi** d'une pièce saine, émis
    /// seulement après le contrôle ci-dessus. Journalisé (C2 : trace d'accès sensible du socle 1, en plus de la ligne
    /// de la brique). Le client ne fournit jamais de clé de stockage.
    pub async fn lien_piece_message_for_actor(
        &self,
        etab: Uuid,
        actor_user_id: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<LienFichier, EntDbError> {
        // M8 : jamais d'URL SigV4 directe pour la messagerie, même en mode S3 : le lien passe par le point de
        // téléchargement de l'application (jeton lié à l'utilisateur, relu à l'ouverture), 60 s au plus.
        if !self.presigner().peut_servir_par_application() {
            return Err(EntDbError::LiensIndisponibles);
        }
        let p = self.piece_a_servir(etab, actor_user_id, role, piece_id).await?;
        if !self.objet_existe(&p.cle).await? {
            return Err(EntDbError::FichierAbsent);
        }
        let lien = self.emettre_lien_application_borne(
            &p.cle,
            actor_user_id,
            etab,
            Disposition::Attachment,
            &p.nom,
            &p.mime,
            DUREE_LIEN_PIECE_SECS,
        )?;
        self.journaliser_acces_piece(etab, actor_user_id, &p, "lien").await?;
        self.tracer_acces_sensible(
            etab,
            actor_user_id,
            role,
            None,
            "telechargement",
            "lien_piece_message",
            &format!("Lien court vers la pièce jointe {piece_id}"),
        )
        .await?;
        Ok(lien)
    }

    // ───────────────────────── Analyse (tâche de fond) ─────────────────────────

    /// Analyse **une** pièce en quarantaine : antivirus sur l'original, validation de forme relue, normalisation,
    /// écriture du fichier servi (une seule fois, garde de la base). Idempotente.
    pub async fn analyser_piece_message(
        &self,
        etab: Uuid,
        piece_id: Uuid,
        antivirus: Arc<dyn AnalyseurAntivirus>,
    ) -> Result<IssueAnalyse, EntDbError> {
        let Some(l) = self.ligne_piece(etab, piece_id).await? else {
            return Ok(IssueAnalyse::DejaTraitee);
        };
        if l.etat != "en_quarantaine" {
            return Ok(IssueAnalyse::DejaTraitee);
        }
        let (Some(cle), Some(sha)) = (l.storage_key.as_deref(), l.sha256.as_deref()) else {
            return Ok(IssueAnalyse::AReprendre);
        };
        // L'original est sous `{etab}/messages/` : un autre établissement ou un autre espace est refusé. (Les jumeaux d'un
        // fil dédoublé partagent l'original de la pièce copiée : la clé n'est donc pas liée à l'identifiant de la ligne.)
        if !cle.starts_with(&format!("{etab}/messages/")) || cle.contains("..") {
            return self.conclure_piece(etab, piece_id, "rejetee", "clé d'objet inattendue").await;
        }
        let original = self.lire_objet_empreinte(etab, cle, Some(sha)).await?;
        // 1. Antivirus (client réseau synchrone : fil bloquant). Fermé par défaut : indisponible → quarantaine.
        let type_fichier = detecter_type(&original);
        let a_scanner = original.clone();
        let av = antivirus.clone();
        let verdict = tokio::task::spawn_blocking(move || {
            av.analyser(&ContexteAnalyse {
                usage: UsageFichier::PieceMessage,
                type_fichier: type_fichier.unwrap_or(TypeFichier::Pdf),
                nom: "piece",
                octets: &a_scanner,
            })
        })
        .await
        .map_err(|_| EntDbError::Refus(RefusMetier::Conflit, "analyse interrompue"))?;
        match verdict {
            VerdictAnalyse::Indisponible => {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                sqlx::query(
                    "UPDATE ent_msg_piece SET analyse_tentatives = analyse_tentatives + 1,
                            derniere_analyse_le = now(), analyse_motif = 'antivirus indisponible'
                      WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_quarantaine'",
                )
                .bind(etab)
                .bind(piece_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                tx.commit().await.map_err(db)?;
                return Ok(IssueAnalyse::EnQuarantaine);
            }
            VerdictAnalyse::Infecte(nom) => {
                let motif = format!("menace : {}", nom.chars().take(80).collect::<String>());
                return self.conclure_piece(etab, piece_id, "infectee", &motif).await;
            }
            VerdictAnalyse::Sain => {}
        }
        // 2. Forme : la validation du dépôt est relue sur l'original (une pièce reprise de l'historique n'a pas
        //    forcément passé la politique actuelle : macros, SVG actif, type annoncé).
        let valide = match self
            .uploads
            .valider(UsageFichier::PieceMessage, &l.nom, Some(&l.mime_type), &original)
        {
            Ok(v) => v,
            Err(e) => {
                let motif: String = e.to_string().chars().take(200).collect();
                return self.conclure_piece(etab, piece_id, "rejetee", &motif).await;
            }
        };
        // 3. Normalisation (D-005) : PDF désarmé, JPEG sans EXIF/GPS, PNG sans blocs annexes ; le reste est servi tel
        //    quel (texte, bureautique ouverte sans macro), toujours en téléchargement (`attachment`).
        let (servi, mime, ext): (Vec<u8>, &'static str, &'static str) = match valide.type_fichier {
            TypeFichier::Pdf => match ent_pdf::piece::desarmer_pdf(&original) {
                Ok(d) => (d.octets, valide.mime, valide.extension),
                Err(e) => {
                    let motif: String = e.to_string().chars().take(200).collect();
                    return self.conclure_piece(etab, piece_id, "rejetee", &motif).await;
                }
            },
            // F3 : sans EXIF ni GPS, mais l'**orientation** est gardée sous la forme d'un segment minimal (la seule balise
            // d'orientation) : la photo s'affiche à l'endroit, sans décodage ni réencodage des pixels (D-062, comme E9).
            TypeFichier::Jpeg => match crate::photo_nettoyage::sans_metadonnees("image/jpeg", &original) {
                Ok(j) => (j, valide.mime, valide.extension),
                Err(m) => return self.conclure_piece(etab, piece_id, "rejetee", m).await,
            },
            TypeFichier::Png => match png_nettoye(&original) {
                Ok(o) => (o, valide.mime, valide.extension),
                Err(m) => return self.conclure_piece(etab, piece_id, "rejetee", m).await,
            },
            _ => (original.clone(), valide.mime, valide.extension),
        };
        let cle_servie = ObjectStore::key_for(etab, ESPACE_SERVI, Uuid::new_v4(), ext);
        let info = self.objet_put(&cle_servie, &servi).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        enregistrer_objet(&mut *tx, etab, &info, Some(mime)).await?;
        // Le seul passage « en quarantaine → saine » : le fichier servi s'écrit ici, une fois (garde de la base).
        let r = sqlx::query(
            "UPDATE ent_msg_piece SET etat = 'saine', cle_objet_servi = $3, sha256_servi = $4, mime_servi = $5,
                    taille_servie = $6, analyse_motif = NULL, analyse_tentatives = analyse_tentatives + 1,
                    derniere_analyse_le = now(), analysee_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_quarantaine'",
        )
        .bind(etab)
        .bind(piece_id)
        .bind(&info.key)
        .bind(&info.sha256)
        .bind(mime)
        .bind(servi.len() as i32)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        if r.rows_affected() != 1 {
            return Ok(IssueAnalyse::DejaTraitee);
        }
        self.publier_piece_analysee(etab, piece_id, l.fil_id, "saine").await?;
        Ok(IssueAnalyse::Saine)
    }

    /// Conclut une analyse par un état terminal **refusé** (`infectee` ou `rejetee`). Le motif reste en base, jamais
    /// dans l'événement ni dans la réponse (l'auteur voit « pièce refusée (sécurité) »).
    async fn conclure_piece(
        &self,
        etab: Uuid,
        piece_id: Uuid,
        etat: &'static str,
        motif: &str,
    ) -> Result<IssueAnalyse, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let fil: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_msg_piece SET etat = $3, analyse_motif = $4, analyse_tentatives = analyse_tentatives + 1,
                    derniere_analyse_le = now(), analysee_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_quarantaine'
          RETURNING fil_id",
        )
        .bind(etab)
        .bind(piece_id)
        .bind(etat)
        .bind(motif.chars().take(200).collect::<String>())
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let Some((fil_id,)) = fil else {
            return Ok(IssueAnalyse::DejaTraitee);
        };
        self.publier_piece_analysee(etab, piece_id, fil_id, etat).await?;
        Ok(if etat == "infectee" {
            IssueAnalyse::Infectee
        } else {
            IssueAnalyse::Rejetee(motif.chars().take(200).collect())
        })
    }

    /// Événement neutre : identifiants et état, **ni nom de fichier, ni motif, ni nom de menace** (R-M14).
    async fn publier_piece_analysee(
        &self,
        etab: Uuid,
        piece_id: Uuid,
        fil_id: Uuid,
        etat: &str,
    ) -> Result<(), EntDbError> {
        self.events
            .publish(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::MESSAGERIE_PIECE_ANALYSEE.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: fil_id,
                // Jamais l'élève : le fil peut être une boîte C3 (R-M11).
                eleve_id: None,
                payload: json!({ "piece_id": piece_id, "etat": etat }),
                actor_user_id: None,
            })
            .await?;
        Ok(())
    }

    /// Analyse jusqu'à `limite` pièces en quarantaine d'un établissement (les plus anciennes d'abord). S'arrête après
    /// [`INDISPONIBLES_AVANT_ARRET`] indisponibilités de l'antivirus de suite.
    pub async fn analyser_pieces_messagerie_etablissement(
        &self,
        etab: Uuid,
        antivirus: Arc<dyn AnalyseurAntivirus>,
        limite: i64,
    ) -> Result<BilanAnalysePieces, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_msg_piece WHERE etablissement_id = $1 AND etat = 'en_quarantaine'
              ORDER BY created_at, id LIMIT $2",
        )
        .bind(etab)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut bilan = BilanAnalysePieces::default();
        let mut indisponibles = 0usize;
        for (id,) in ids {
            let issue = match self.analyser_piece_message(etab, id, antivirus.clone()).await {
                Ok(i) => i,
                Err(e) => {
                    // Ni nom ni clé dans le journal : l'identifiant de la pièce et la cause seulement.
                    tracing::warn!(piece = %id, error = %e, "analyse d'une pièce de messagerie en échec");
                    bilan.en_echec += 1;
                    continue;
                }
            };
            match issue {
                IssueAnalyse::Saine => {
                    bilan.saines += 1;
                    indisponibles = 0;
                }
                IssueAnalyse::EnQuarantaine => {
                    bilan.en_quarantaine += 1;
                    indisponibles += 1;
                    if indisponibles >= INDISPONIBLES_AVANT_ARRET {
                        bilan.antivirus_indisponible = true;
                        break;
                    }
                }
                IssueAnalyse::Infectee => bilan.infectees += 1,
                IssueAnalyse::Rejetee(_) => bilan.rejetees += 1,
                IssueAnalyse::AReprendre => bilan.a_reprendre += 1,
                IssueAnalyse::DejaTraitee => {}
            }
        }
        Ok(bilan)
    }

    /// Une passe de la tâche de fond, pour tous les établissements qui ont des pièces en quarantaine. Appelée par
    /// l'ordonnanceur de `ent-api` quand `CLAMAV_ADDR` est configurée (fermé par défaut).
    pub async fn analyser_pieces_messagerie_tache(
        &self,
        antivirus: Arc<dyn AnalyseurAntivirus>,
        limite: i64,
    ) -> Result<BilanAnalysePieces, EntDbError> {
        let etabs: Vec<(Uuid,)> = lire_par_etablissement(
            self.pool(),
            ContexteSysteme::JobPlanifie("pieces_messagerie_analyse"),
            "SELECT DISTINCT etablissement_id FROM ent_msg_piece WHERE etat = 'en_quarantaine'",
        )
        .await?;
        let mut total = BilanAnalysePieces::default();
        for (etab,) in etabs {
            // F9 : une erreur chez un établissement n'interrompt pas la passe pour les autres.
            let b = match self
                .analyser_pieces_messagerie_etablissement(etab, antivirus.clone(), limite)
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(etablissement = %etab, error = %e, "analyse des pièces de messagerie en échec pour un établissement");
                    total.en_echec += 1;
                    continue;
                }
            };
            total.saines += b.saines;
            total.en_quarantaine += b.en_quarantaine;
            total.infectees += b.infectees;
            total.rejetees += b.rejetees;
            total.a_reprendre += b.a_reprendre;
            total.en_echec += b.en_echec;
            if b.antivirus_indisponible {
                total.antivirus_indisponible = true;
                break;
            }
        }
        Ok(total)
    }

    /// Reprise des pièces `BYTEA` historiques de la messagerie **et** analyse a posteriori (§26 : « état `saine` après
    /// analyse »). Idempotente et rejouable : la reprise (`reprise_bytea.rs`) ne refait que le reste ; l'analyse ne
    /// traite que ce qui est encore en quarantaine. Avec `options.simulation`, rien n'est écrit (ni objet, ni ligne,
    /// ni analyse). Le rapport ne porte ni nom de fichier ni clé d'objet.
    pub async fn reprendre_et_analyser_pieces_messagerie(
        &self,
        etab: Uuid,
        options: &OptionsReprise,
        antivirus: Option<Arc<dyn AnalyseurAntivirus>>,
        limite_analyse: i64,
    ) -> Result<RapportRepriseM5, EntDbError> {
        let reprise = self
            .reprendre_pieces_etablissement(TablePiece::Messagerie, etab, options)
            .await?;
        let analyse = match (options.simulation, antivirus) {
            (false, Some(av)) => {
                self.analyser_pieces_messagerie_etablissement(etab, av, limite_analyse)
                    .await?
            }
            _ => BilanAnalysePieces::default(),
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (restent,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_msg_piece WHERE etablissement_id = $1 AND etat = 'en_quarantaine'",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(RapportRepriseM5 { reprise, analyse, restent_en_quarantaine: restent })
    }

    // ───────────────────────── Purge des pièces refusées (M5) ─────────────────────────

    /// Une passe de la purge des originaux de pièces **refusées** de la messagerie, pour tous les établissements :
    /// `infectee` après `jours_infectees` jours, `rejetee` après `jours_rejetees` jours (catégorie de conservation
    /// `msg.piece_refusee`, réglable par `ENT_MSG_PIECES_INFECTEES_RETENTION_JOURS` et
    /// `ENT_MSG_PIECES_REJETEES_RETENTION_JOURS`). Une erreur chez un établissement n'arrête pas les autres.
    pub async fn purger_pieces_messagerie_refusees_tache(
        &self,
        jours_infectees: i64,
        jours_rejetees: i64,
    ) -> Result<BilanPurgeMsg, EntDbError> {
        let etabs: Vec<(Uuid,)> = lire_par_etablissement(
            self.pool(),
            ContexteSysteme::JobPlanifie("pieces_messagerie_purge"),
            "SELECT DISTINCT p.etablissement_id FROM ent_msg_piece p
               JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.cle_objet = p.storage_key
              WHERE p.etat IN ('infectee', 'rejetee') AND o.supprime_le IS NULL",
        )
        .await?;
        let mut bilan = BilanPurgeMsg::default();
        for (etab,) in etabs {
            match self
                .purger_pieces_messagerie_refusees(etab, jours_infectees, jours_rejetees)
                .await
            {
                Ok(b) => {
                    bilan.purges += b.purges;
                    bilan.echecs += b.echecs;
                }
                Err(e) => {
                    tracing::warn!(etablissement = %etab, error = %e, "purge des pièces de messagerie refusées en échec");
                    bilan.echecs += 1;
                }
            }
        }
        Ok(bilan)
    }

    /// Détruit les originaux des pièces refusées d'un établissement dont l'analyse est assez ancienne. L'objet est
    /// détruit au magasin **avant** d'être marqué supprimé au registre (une panne entre les deux se rattrape à la passe
    /// suivante : suppression idempotente). Un objet encore référencé par une autre pièce (jumeaux d'un fil dédoublé)
    /// n'est jamais détruit.
    pub async fn purger_pieces_messagerie_refusees(
        &self,
        etab: Uuid,
        jours_infectees: i64,
        jours_rejetees: i64,
    ) -> Result<BilanPurgeMsg, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a_purger: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT p.id, p.storage_key, p.etat
               FROM ent_msg_piece p
               JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.cle_objet = p.storage_key
              WHERE p.etablissement_id = $1 AND o.supprime_le IS NULL
                AND ((p.etat = 'infectee' AND coalesce(p.analysee_le, p.derniere_analyse_le) <= now() - make_interval(days => $2::int))
                  OR (p.etat = 'rejetee'  AND coalesce(p.analysee_le, p.derniere_analyse_le) <= now() - make_interval(days => $3::int)))
                AND NOT EXISTS (SELECT 1 FROM ent_msg_piece q
                                 WHERE q.etablissement_id = p.etablissement_id AND q.id <> p.id
                                   AND (q.storage_key = p.storage_key OR q.cle_objet_servi = p.storage_key)
                                   AND q.etat NOT IN ('infectee', 'rejetee'))
              ORDER BY coalesce(p.analysee_le, p.derniere_analyse_le), p.id
              LIMIT 200",
        )
        .bind(etab)
        .bind(jours_infectees.clamp(0, 3650) as i32)
        .bind(jours_rejetees.clamp(0, 3650) as i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut bilan = BilanPurgeMsg::default();
        for (piece_id, cle, etat) in a_purger {
            if let Err(e) = self.objet_supprimer(&cle).await {
                // Ni la clé ni aucun nom dans le journal d'exploitation : l'identifiant de la pièce suffit.
                tracing::warn!(piece = %piece_id, erreur = %e, "purge d'une pièce de messagerie refusée : destruction impossible");
                bilan.echecs += 1;
                continue;
            }
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let r = sqlx::query(
                "UPDATE ent_objet SET supprime_le = now()
                  WHERE etablissement_id = $1 AND cle_objet = $2 AND supprime_le IS NULL",
            )
            .bind(etab)
            .bind(&cle)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            if r.rows_affected() == 1 {
                tracing::info!(piece = %piece_id, etat = %etat, "original d'une pièce de messagerie refusée détruit");
                bilan.purges += 1;
            }
        }
        Ok(bilan)
    }

    // ───────────────────────── Carte « document » (R-M33) ─────────────────────────

    /// Le droit **de ce lecteur** sur un document du socle 7, **sans effet de bord** (ni trace, ni accusé, ni octet
    /// lu) : un titulaire le retrouve dans son coffre (droit du jour : lien, autorité parentale, majorité), un
    /// personnel par le droit de lecture du domaine du document sur son sujet.
    async fn document_lisible_par(&self, a: &Acteur<'_>, document_id: Uuid) -> Result<Option<String>, EntDbError> {
        if let Some(ligne) = self
            .coffre_for_actor(a, &FiltreCoffre::default())
            .await?
            .into_iter()
            .find(|l| l.document_id == document_id)
        {
            return Ok(Some(ligne.titre));
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let doc: Option<(String, Option<Uuid>, String, String, String)> = sqlx::query_as(
            "SELECT domaine, eleve_id, sensibilite, etat, titre FROM ent_doc_document
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let Some((domaine, eleve, sensibilite, etat, titre)) = doc else {
            return Ok(None);
        };
        if sensibilite == "C4" || matches!(etat.as_str(), "brouillon" | "annule" | "elimine") {
            return Ok(None);
        }
        let Some(domaine) = Domaine::parse(&domaine) else {
            return Ok(None);
        };
        Ok(self
            .doc_peut_sur_sujet(a, domaine.action_lecture(), eleve)
            .await?
            .then_some(titre))
    }

    /// L'élève à porter par un événement du fil : jamais pour un fil C3.
    pub(crate) async fn eleve_pour_evenement(&self, fil: &domain_eleve::MsgFil) -> Result<Option<Uuid>, EntDbError> {
        Ok(if self.fil_est_c3(fil.etablissement_id, fil.id).await? { None } else { Some(fil.eleve_id) })
    }

    /// Détruit des objets de messagerie au magasin **puis** les marque supprimés au registre (`ent_objet`). Au mieux :
    /// un échec est journalisé (sans clé ni nom), l'objet reste alors au registre « non supprimé » et la purge
    /// périodique le reprendra. Un objet référencé par une autre pièce n'est jamais détruit.
    pub(crate) async fn detruire_objets_messagerie(&self, etab: Uuid, cles: &[String]) {
        for cle in cles {
            let encore: Result<(i64,), sqlx::Error> = async {
                let mut tx = TenantTx::begin(self.pool(), etab).await?;
                let r = sqlx::query_as(
                    "SELECT count(*) FROM ent_msg_piece
                      WHERE etablissement_id = $1 AND (storage_key = $2 OR cle_objet_servi = $2)",
                )
                .bind(etab)
                .bind(cle)
                .fetch_one(&mut *tx)
                .await;
                tx.commit().await?;
                r
            }
            .await;
            if !matches!(encore, Ok((0,))) {
                continue;
            }
            if let Err(e) = self.objet_supprimer(cle).await {
                tracing::warn!(erreur = %e, "pièce de messagerie : destruction de l'objet impossible");
                continue;
            }
            let r: Result<(), sqlx::Error> = async {
                let mut tx = TenantTx::begin(self.pool(), etab).await?;
                sqlx::query(
                    "UPDATE ent_objet SET supprime_le = now()
                      WHERE etablissement_id = $1 AND cle_objet = $2 AND supprime_le IS NULL",
                )
                .bind(etab)
                .bind(cle)
                .execute(&mut *tx)
                .await?;
                tx.commit().await
            }
            .await;
            if let Err(e) = r {
                tracing::warn!(erreur = %e, "pièce de messagerie : objet détruit mais registre non mis à jour");
            }
        }
    }

    /// Le fil est-il celui d'une boîte **C3** (infirmerie, psychologue, assistante sociale) ? Aucun événement d'un tel
    /// fil ne porte l'élève ni un lecteur (M1, R-M14, R-M22).
    pub(crate) async fn fil_est_c3(&self, etab: Uuid, fil_id: Uuid) -> Result<bool, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Option<(String,)> = sqlx::query_as(
            "SELECT sensibilite FROM ent_msg_fil WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(fil_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        Ok(r.is_some_and(|(s,)| s == "C3"))
    }

    /// Joint un **document du coffre** à un message par carte (CM-60) : jamais le fichier. L'auteur du message doit
    /// lui-même pouvoir lire le document ; les destinataires le liront selon **leurs** droits, relus à chaque lecture.
    pub async fn joindre_carte_document_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        message_id: Uuid,
        document_id: Uuid,
    ) -> Result<CarteDocument, EntDbError> {
        let msg = self.messages.get_message(etab, message_id).await?.ok_or_else(introuvable)?;
        let fil = self.messages.get_fil(etab, msg.fil_id).await?.ok_or_else(introuvable)?;
        if !self.actor_can_access_fil(etab, user, role, &fil).await? {
            return Err(introuvable());
        }
        if msg.auteur_user_id != user {
            return Err(EntDbError::Forbidden);
        }
        let acteur = Acteur { etablissement_id: etab, user_id: user, role };
        let Some(titre) = self.document_lisible_par(&acteur, document_id).await? else {
            // Même réponse que pour un document inexistant : aucun indice.
            return Err(introuvable());
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (nb,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_msg_carte_document WHERE etablissement_id = $1 AND message_id = $2",
        )
        .bind(etab)
        .bind(message_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if nb >= CARTES_MAX_PAR_MESSAGE {
            return Err(EntDbError::SaisieInvalide("5 documents au plus par message"));
        }
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_msg_carte_document (id, etablissement_id, fil_id, message_id, document_id, ajoutee_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (message_id, document_id) DO NOTHING",
        )
        .bind(id)
        .bind(etab)
        .bind(msg.fil_id)
        .bind(message_id)
        .bind(document_id)
        .bind(user)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("ce document est déjà joint à ce message"));
        }
        tx.commit().await.map_err(db)?;
        let c3 = self.fil_est_c3(etab, msg.fil_id).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::MESSAGERIE_CARTE_INSEREE.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: msg.fil_id,
                // Fil C3 : jamais l'élève (motif de M3).
                eleve_id: if c3 { None } else { Some(fil.eleve_id) },
                // Socle propriétaire et objet : jamais le titre ni la nature du document (R-M14).
                payload: json!({ "message_id": message_id, "socle": "documents", "objet_id": document_id }),
                actor_user_id: Some(user.to_string()),
            })
            .await?;
        Ok(CarteDocument {
            id,
            message_id,
            document_id,
            lisible: true,
            titre: Some(titre),
        })
    }

    /// Les cartes « document » d'un fil **telles que ce lecteur les voit** : le droit est relu **maintenant**. Un
    /// lecteur sans droit voit « document non disponible » (ni titre, ni nature) ; une carte d'un message retiré ou
    /// encore différé n'est pas servie.
    pub async fn cartes_du_fil_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil_id: Uuid,
    ) -> Result<Vec<CarteDocument>, EntDbError> {
        let fil = self.messages.get_fil(etab, fil_id).await?.ok_or_else(introuvable)?;
        if !self.actor_can_access_fil(etab, user, role, &fil).await? {
            return Err(introuvable());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cartes: Vec<(Uuid, Uuid, Uuid)> = sqlx::query_as(
            "SELECT c.id, c.message_id, c.document_id
               FROM ent_msg_carte_document c
               JOIN ent_msg m ON m.etablissement_id = c.etablissement_id AND m.id = c.message_id
              WHERE c.etablissement_id = $1 AND c.fil_id = $2 AND m.retire_le IS NULL
                AND (m.remise_prevue_le IS NULL OR m.remise_prevue_le <= now() OR m.auteur_user_id = $3)
              ORDER BY c.cree_le, c.id",
        )
        .bind(etab)
        .bind(fil_id)
        .bind(user)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let acteur = Acteur { etablissement_id: etab, user_id: user, role };
        let mut vues = Vec::with_capacity(cartes.len());
        for (id, message_id, document_id) in cartes {
            let titre = self.document_lisible_par(&acteur, document_id).await?;
            vues.push(CarteDocument { id, message_id, document_id, lisible: titre.is_some(), titre });
        }
        Ok(vues)
    }

    /// Ouvre le document d'une carte avec **les droits du lecteur** (coffre ou lecture du personnel) : la lecture, la
    /// trace et l'accusé de première ouverture sont ceux du socle 7. Sans droit : « introuvable ».
    pub async fn ouvrir_carte_document_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        carte_id: Uuid,
    ) -> Result<CarteOuverte, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let c: Option<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT fil_id, document_id FROM ent_msg_carte_document WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(carte_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let (fil_id, document_id) = c.ok_or_else(introuvable)?;
        // Le lecteur doit toujours lire le fil (un participant retiré n'ouvre plus rien) …
        let cartes = self.cartes_du_fil_for_actor(etab, user, role, fil_id).await?;
        // … et la carte doit être servie (message ni retiré ni différé) et lisible par lui.
        let Some(carte) = cartes.iter().find(|x| x.id == carte_id && x.lisible) else {
            return Err(introuvable());
        };
        let titre = carte.titre.clone().unwrap_or_default();
        let acteur = Acteur { etablissement_id: etab, user_id: user, role };
        let d = self.lire_document_for_actor(&acteur, document_id, None).await?;
        // F5 : type et nom selon le document **réel** (signature des octets), jamais un PDF supposé.
        let (mime, ext) = match detecter_type(&d.octets) {
            Some(t) => (t.mime(), t.extension()),
            None => ("application/octet-stream", "bin"),
        };
        let base: String = titre
            .chars()
            .map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') { c } else { '_' })
            .collect::<String>()
            .trim()
            .chars()
            .take(80)
            .collect();
        let base = if base.is_empty() { "document".to_string() } else { base };
        Ok(CarteOuverte { nom: format!("{base}.{ext}"), mime, octets: d.octets })
    }
}

/// Document ouvert par une carte : octets, type et nom **réels** (F5).
#[derive(Debug, Clone)]
pub struct CarteOuverte {
    pub octets: Vec<u8>,
    pub nom: String,
    pub mime: &'static str,
}

/// Carte « document » d'un fil, vue par un lecteur.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CarteDocument {
    pub id: Uuid,
    pub message_id: Uuid,
    pub document_id: Uuid,
    /// Faux : « document non disponible » (le droit du lecteur ne le couvre pas, ou plus).
    pub lisible: bool,
    /// Titre, **seulement** si `lisible`.
    pub titre: Option<String>,
}

/// Type réel d'un fichier par sa signature (jamais le type annoncé).
fn detecter_type(octets: &[u8]) -> Option<TypeFichier> {
    crate::documents_pieces::type_de_fichier(octets).ok()
}

/// Le rôle d'un compte est-il du personnel ? (un personnel ne dépose pas un document du coffre en fichier, CM-60).
pub(crate) fn est_personnel(role: &str) -> bool {
    Role::parse(role).is_some_and(|r| r.is_staff())
}
