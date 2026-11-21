//! Stockage objet ENT — S3-compatible en production, disque/mémoire en labo et tests.
//!
//! **Production (D-037…, PRA §18.4)** : S3 (Scaleway, OVHcloud…) est le stockage
//! **principal** ; `from_env_strict(false)` refuse de démarrer sans configuration S3
//! complète. Le disque local et la mémoire sont réservés aux tests et au labo
//! (`ENT_ENV` ∈ `lab|labo|dev|local|test`).
//!
//! Principes :
//! - **objets immuables** : clé `{etablissement}/{espace}/{uuid}.{ext}` (UUID seulement,
//!   aucune donnée personnelle — E14), jamais réécrite ; réécrire un contenu différent
//!   est une erreur (`Immutable`) ;
//! - **empreinte SHA-256 et taille** calculées à l'écriture, vérifiées après l'écriture
//!   (relecture des métadonnées) et à chaque lecture ; la base les enregistre
//!   (`ent_objet`) et `get_attendu` les compare aux valeurs de la base ;
//! - **aucun échec ignoré** : toute erreur remonte à l'appelant, est journalisée
//!   (`tracing`, sans contenu) et comptée (`metrics()`) ;
//! - fournisseur abstrait par `StorageProvider` : la logique métier ne dépend d'aucun
//!   fournisseur.
//!
//! Variables d'environnement : voir `ObjectStore::from_vars`.

pub mod presign;
pub mod provider;
pub mod s3;
pub mod upload;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use provider::{DiskProvider, MemoryProvider, StorageProvider};

#[derive(Debug, Error)]
pub enum ObjectStoreError {
    #[error("clé objet invalide")]
    InvalidKey,
    // Message fixe : la clé objet ne doit jamais remonter au client.
    #[error("objet introuvable")]
    NotFound(String),
    #[error("IO: {0}")]
    Io(String),
    /// Réécriture d'une clé existante avec un contenu différent (objets immuables).
    #[error("objet immuable : réécriture refusée")]
    Immutable,
    /// Empreinte ou taille différente de celle attendue (corruption, écriture tronquée).
    #[error("intégrité de l'objet non vérifiée")]
    Integrity,
    /// Le fournisseur de stockage a échoué (réseau, statut S3…) ; message sans secret ni clé.
    #[error("stockage d'objets indisponible: {0}")]
    Backend(String),
    /// Aucun fournisseur utilisable (configuration de production absente ou invalide).
    #[error("stockage d'objets non configuré")]
    Unavailable,
}

/// Résultat d'une écriture vérifiée : à enregistrer en base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectInfo {
    pub key: String,
    pub taille: u64,
    /// SHA-256 hex minuscule.
    pub sha256: String,
}

/// Empreinte et taille attendues (lues en base) pour une lecture contrôlée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectAttendu {
    pub taille: u64,
    pub sha256: String,
}

/// Compteurs du magasin (depuis le démarrage du processus).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StoreMetrics {
    pub ecritures_ok: u64,
    pub ecritures_echec: u64,
    pub lectures_ok: u64,
    pub lectures_echec: u64,
    pub integrite_echec: u64,
    /// Lectures d'objets historiques sans empreinte (non vérifiables).
    pub lectures_sans_empreinte: u64,
    pub head_echec: u64,
}

#[derive(Default)]
struct Compteurs {
    ecritures_ok: AtomicU64,
    ecritures_echec: AtomicU64,
    lectures_ok: AtomicU64,
    lectures_echec: AtomicU64,
    integrite_echec: AtomicU64,
    lectures_sans_empreinte: AtomicU64,
    head_echec: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    S3,
    Disque,
    Memoire,
    Autre,
    Indisponible,
}

const ENVS_NON_PROD: [&str; 5] = ["lab", "labo", "dev", "local", "test"];

#[derive(Clone)]
pub struct ObjectStore {
    provider: Option<Arc<dyn StorageProvider>>,
    kind: BackendKind,
    disk_root: Option<PathBuf>,
    compteurs: Arc<Compteurs>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    presign::hex(&Sha256::digest(bytes))
}

impl ObjectStore {
    fn avec(provider: Arc<dyn StorageProvider>, kind: BackendKind, disk_root: Option<PathBuf>) -> Self {
        Self {
            provider: Some(provider),
            kind,
            disk_root,
            compteurs: Arc::new(Compteurs::default()),
        }
    }

    /// Magasin au-dessus d'un fournisseur quelconque (tests, nouveaux fournisseurs).
    pub fn with_provider(provider: Arc<dyn StorageProvider>) -> Self {
        Self::avec(provider, BackendKind::Autre, None)
    }

    pub fn memory_only() -> Self {
        Self::avec(Arc::new(MemoryProvider::new()), BackendKind::Memoire, None)
    }

    /// Disque local (labo/tests uniquement).
    pub fn disk(root: PathBuf) -> Self {
        Self::avec(
            Arc::new(DiskProvider::new(root.clone())),
            BackendKind::Disque,
            Some(root),
        )
    }

    /// S3-compatible (production).
    pub fn s3(cfg: presign::S3Config) -> Self {
        Self::avec(Arc::new(s3::S3Provider::new(cfg)), BackendKind::S3, None)
    }

    /// Magasin qui refuse toute opération (configuration de production absente).
    pub fn unavailable() -> Self {
        Self {
            provider: None,
            kind: BackendKind::Indisponible,
            disk_root: None,
            compteurs: Arc::new(Compteurs::default()),
        }
    }

    /// Lit l'environnement **sans rien imposer** (tests, labo) : S3 si configuré, sinon
    /// mémoire (`ENT_OBJECT_MEMORY=1`) ou disque. Le binaire de production utilise
    /// `from_env_strict`.
    pub fn from_env() -> Self {
        Self::from_vars(&|k| std::env::var(k).ok(), true).unwrap_or_else(|_| Self::unavailable())
    }

    /// Démarrage de production : hors labo, **S3 obligatoire** (configuration complète),
    /// sinon erreur (le binaire refuse de démarrer). En labo (`non_prod`), repli disque.
    pub fn from_env_strict(non_prod: bool) -> Result<Self, ObjectStoreError> {
        Self::from_vars(&|k| std::env::var(k).ok(), non_prod)
    }

    /// Construction depuis un lecteur de variables (testable sans toucher l'environnement).
    ///
    /// | Variable | Rôle |
    /// |---|---|
    /// | `ENT_OBJECT_BACKEND` | `s3` \| `disk` \| `memory` (défaut : `s3` si configuré, sinon `disk`/`memory` en labo) |
    /// | `ENT_OBJECT_S3_BUCKET`, `_REGION`, `_ENDPOINT`, `_ACCESS_KEY`, `_SECRET_KEY` | S3 |
    /// | `ENT_OBJECT_S3_VIRTUAL_HOST` | `1` : `https://{bucket}.{hôte}` (sinon style chemin) |
    /// | `ENT_OBJECT_DIR` | racine disque (labo) |
    /// | `ENT_OBJECT_MEMORY` | `1` : mémoire (labo, historique) |
    ///
    /// `permissif = false` (production) : `disk`/`memory` refusés, S3 incomplet refusé.
    pub fn from_vars(
        get: &dyn Fn(&str) -> Option<String>,
        permissif: bool,
    ) -> Result<Self, ObjectStoreError> {
        let var = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        let flag = |k: &str| {
            var(k).is_some_and(|v| {
                matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes")
            })
        };
        let s3_cfg = || {
            presign::S3Config::new(
                var("ENT_OBJECT_S3_BUCKET").as_deref().unwrap_or(""),
                var("ENT_OBJECT_S3_REGION").as_deref().unwrap_or("fr-par"),
                var("ENT_OBJECT_S3_ACCESS_KEY").as_deref().unwrap_or(""),
                var("ENT_OBJECT_S3_SECRET_KEY").as_deref().unwrap_or(""),
                var("ENT_OBJECT_S3_ENDPOINT").as_deref(),
                flag("ENT_OBJECT_S3_VIRTUAL_HOST"),
            )
        };
        let demande = var("ENT_OBJECT_BACKEND").map(|v| v.trim().to_ascii_lowercase());
        let choix = match demande.as_deref() {
            Some(c @ ("s3" | "disk" | "memory")) => c.to_string(),
            Some(_) => return Err(ObjectStoreError::Unavailable),
            None if flag("ENT_OBJECT_MEMORY") => "memory".into(),
            None if s3_cfg().is_ok() => "s3".into(),
            None if permissif => "disk".into(),
            None => "s3".into(),
        };
        match choix.as_str() {
            "s3" => {
                let cfg = s3_cfg().map_err(|_| ObjectStoreError::Unavailable)?;
                // Production : jamais de HTTP clair vers le stockage (données d'élèves).
                if !permissif && !cfg.endpoint.starts_with("https://") {
                    return Err(ObjectStoreError::Unavailable);
                }
                Ok(Self::s3(cfg))
            }
            "memory" | "disk" if !permissif => Err(ObjectStoreError::Unavailable),
            "memory" => Ok(Self::memory_only()),
            _ => {
                let root = var("ENT_OBJECT_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(".ent-objects"));
                Ok(Self::disk(root))
            }
        }
    }

    /// `true` si `ENT_ENV` désigne un environnement de labo/test.
    pub fn env_non_prod(valeur: Option<&str>) -> bool {
        valeur
            .map(|e| e.trim().to_ascii_lowercase())
            .is_some_and(|e| ENVS_NON_PROD.contains(&e.as_str()))
    }

    pub fn kind(&self) -> BackendKind {
        self.kind
    }

    pub fn is_memory(&self) -> bool {
        self.kind == BackendKind::Memoire
    }

    pub fn is_disk(&self) -> bool {
        self.kind == BackendKind::Disque
    }

    pub fn is_s3(&self) -> bool {
        self.kind == BackendKind::S3
    }

    pub fn is_available(&self) -> bool {
        self.provider.is_some()
    }

    /// Racine disque (labo) si le fournisseur est le disque.
    pub fn disk_root(&self) -> Option<&Path> {
        self.disk_root.as_deref()
    }

    /// Nom du fournisseur, pour les journaux.
    pub fn provider_name(&self) -> &'static str {
        self.provider.as_ref().map_or("indisponible", |p| p.nom())
    }

    /// URI logique (journalisation ; ne contient jamais de secret).
    pub fn uri(&self, key: &str) -> String {
        match &self.provider {
            Some(p) => p.uri(key),
            None => format!("indisponible://{key}"),
        }
    }

    pub fn metrics(&self) -> StoreMetrics {
        let c = &self.compteurs;
        StoreMetrics {
            ecritures_ok: c.ecritures_ok.load(Ordering::Relaxed),
            ecritures_echec: c.ecritures_echec.load(Ordering::Relaxed),
            lectures_ok: c.lectures_ok.load(Ordering::Relaxed),
            lectures_echec: c.lectures_echec.load(Ordering::Relaxed),
            integrite_echec: c.integrite_echec.load(Ordering::Relaxed),
            lectures_sans_empreinte: c.lectures_sans_empreinte.load(Ordering::Relaxed),
            head_echec: c.head_echec.load(Ordering::Relaxed),
        }
    }

    /// Nombre d'écritures réussies (compat. tests).
    pub fn put_count(&self) -> usize {
        self.compteurs.ecritures_ok.load(Ordering::Relaxed) as usize
    }

    /// Construit une clé sûre pour un artefact métier : `{etab}/{espace}/{id}.{ext}`.
    /// Pour un objet **immuable**, `id` doit être un UUID neuf à chaque version.
    pub fn key_for(etablissement_id: Uuid, namespace: &str, id: Uuid, ext: &str) -> String {
        let ns = sanitize_segment(namespace);
        let ext = sanitize_ext(ext);
        format!("{etablissement_id}/{ns}/{id}.{ext}")
    }

    fn fournisseur(&self) -> Result<&Arc<dyn StorageProvider>, ObjectStoreError> {
        self.provider.as_ref().ok_or_else(|| {
            tracing::error!("stockage d'objets non configuré : opération refusée (fail-closed)");
            ObjectStoreError::Unavailable
        })
    }

    fn compter_echec_lecture(&self, e: &ObjectStoreError) {
        match e {
            ObjectStoreError::NotFound(_) | ObjectStoreError::InvalidKey => {}
            ObjectStoreError::Integrity => {
                self.compteurs.integrite_echec.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.compteurs.lectures_echec.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Écrit un objet **immuable** et vérifie l'écriture (taille et empreinte relues).
    /// Toute erreur remonte ; rien n'est ignoré. Renvoie l'empreinte et la taille à
    /// enregistrer en base.
    pub fn put(&self, key: &str, bytes: &[u8]) -> Result<ObjectInfo, ObjectStoreError> {
        let res = self.put_interne(key, bytes);
        match &res {
            Ok(_) => {
                self.compteurs.ecritures_ok.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                self.compteurs.ecritures_echec.fetch_add(1, Ordering::Relaxed);
                if matches!(e, ObjectStoreError::Integrity) {
                    self.compteurs.integrite_echec.fetch_add(1, Ordering::Relaxed);
                }
                tracing::error!(
                    fournisseur = self.provider_name(),
                    erreur = %e,
                    taille = bytes.len(),
                    "échec d'écriture d'un objet (remonté à l'appelant)"
                );
            }
        }
        res
    }

    fn put_interne(&self, key: &str, bytes: &[u8]) -> Result<ObjectInfo, ObjectStoreError> {
        assert_safe_key(key)?;
        let p = self.fournisseur()?;
        let sha = sha256_hex(bytes);
        p.put(key, bytes, &sha)?;
        // Vérification à l'écriture : l'objet relu a la bonne taille et la bonne empreinte.
        let h = p.head(key)?.ok_or(ObjectStoreError::Integrity)?;
        if h.taille != bytes.len() as u64 || h.sha256.as_deref().is_some_and(|s| s != sha) {
            return Err(ObjectStoreError::Integrity);
        }
        Ok(ObjectInfo {
            key: key.to_string(),
            taille: bytes.len() as u64,
            sha256: sha,
        })
    }

    /// Lit un objet et vérifie son empreinte contre celle conservée avec lui.
    pub fn get(&self, key: &str) -> Result<Vec<u8>, ObjectStoreError> {
        self.get_interne(key, None)
    }

    /// Lit un objet et le compare à l'empreinte et à la taille **enregistrées en base**.
    pub fn get_attendu(&self, key: &str, attendu: &ObjectAttendu) -> Result<Vec<u8>, ObjectStoreError> {
        self.get_interne(key, Some(attendu))
    }

    fn get_interne(
        &self,
        key: &str,
        attendu: Option<&ObjectAttendu>,
    ) -> Result<Vec<u8>, ObjectStoreError> {
        assert_safe_key(key)?;
        let p = self.fournisseur()?;
        let res = p.get(key).and_then(|o| {
            let sha = sha256_hex(&o.bytes);
            if o.sha256.as_deref().is_some_and(|s| s != sha) {
                return Err(ObjectStoreError::Integrity);
            }
            if let Some(a) = attendu {
                if a.sha256 != sha || a.taille != o.bytes.len() as u64 {
                    return Err(ObjectStoreError::Integrity);
                }
            } else if o.sha256.is_none() {
                self.compteurs
                    .lectures_sans_empreinte
                    .fetch_add(1, Ordering::Relaxed);
            }
            Ok(o.bytes)
        });
        match &res {
            Ok(_) => {
                self.compteurs.lectures_ok.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                self.compter_echec_lecture(e);
                if !matches!(e, ObjectStoreError::NotFound(_) | ObjectStoreError::InvalidKey) {
                    tracing::error!(
                        fournisseur = self.provider_name(),
                        erreur = %e,
                        "échec de lecture d'un objet"
                    );
                }
            }
        }
        res
    }

    /// Existence d'un objet ; une panne du fournisseur est une **erreur**, pas « absent ».
    pub fn existe(&self, key: &str) -> Result<bool, ObjectStoreError> {
        assert_safe_key(key)?;
        match self.fournisseur()?.head(key) {
            Ok(h) => Ok(h.is_some()),
            Err(e) => {
                self.compteurs.head_echec.fetch_add(1, Ordering::Relaxed);
                tracing::error!(fournisseur = self.provider_name(), erreur = %e, "échec de vérification d'un objet");
                Err(e)
            }
        }
    }

    /// Compat. : `true` seulement si la clé est valide et l'objet présent. Une panne du
    /// fournisseur est journalisée et comptée (`head_echec`) ; préférer `existe`.
    pub fn exists(&self, key: &str) -> bool {
        self.existe(key).unwrap_or(false)
    }
}

fn sanitize_segment(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn sanitize_ext(s: &str) -> String {
    let t = s.trim().trim_start_matches('.').to_ascii_lowercase();
    if t.is_empty() {
        "bin".into()
    } else {
        sanitize_segment(&t)
    }
}

/// Longueur maximale d'une clé objet.
const MAX_KEY_LEN: usize = 512;

/// Valide une clé objet : relative, sans traversée, sans lettre de lecteur,
/// sans flux NTFS ni caractère de contrôle. Appliquée à tout accès (get/put/exists).
fn assert_safe_key(key: &str) -> Result<(), ObjectStoreError> {
    if key.is_empty() || key.len() > MAX_KEY_LEN {
        return Err(ObjectStoreError::InvalidKey);
    }
    if key.contains("..")
        || key.contains('\\')
        || key.contains(':')
        || key.chars().any(|c| c.is_control())
    {
        return Err(ObjectStoreError::InvalidKey);
    }
    // Segments non vides, ni `.` ni `..` (un `/` initial donne un segment vide).
    if key
        .split('/')
        .any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        return Err(ObjectStoreError::InvalidKey);
    }
    // Défense en profondeur : tous les composants doivent être `Normal`.
    if !Path::new(key)
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
    {
        return Err(ObjectStoreError::InvalidKey);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider::{FaultyProvider, Pannes};
    use std::env;
    use std::fs;
    use std::sync::atomic::Ordering::SeqCst;

    fn cle() -> String {
        ObjectStore::key_for(Uuid::new_v4(), "pai", Uuid::new_v4(), "pdf")
    }

    fn avec_pannes() -> (ObjectStore, Arc<Pannes>) {
        let (f, p) = FaultyProvider::new(Arc::new(MemoryProvider::new()));
        (ObjectStore::with_provider(Arc::new(f)), p)
    }

    #[test]
    fn memory_roundtrip_et_empreinte() {
        let store = ObjectStore::memory_only();
        let key = cle();
        let info = store.put(&key, b"%PDF-1.4").unwrap();
        assert_eq!(info.taille, 8);
        assert_eq!(info.sha256, sha256_hex(b"%PDF-1.4"));
        assert_eq!(store.get(&key).unwrap(), b"%PDF-1.4");
        assert!(store.exists(&key));
        assert_eq!(store.put_count(), 1);
    }

    #[test]
    fn disque_roundtrip_atomique_et_immuable() {
        let dir = env::temp_dir().join(format!("ent-obj-disk-{}", Uuid::new_v4()));
        let store = ObjectStore::disk(dir.clone());
        assert!(store.is_disk());
        let key = cle();
        store.put(&key, b"disk-bytes").unwrap();
        assert_eq!(store.get(&key).unwrap(), b"disk-bytes");
        assert!(dir.join(&key).is_file());
        // Même contenu : idempotent ; contenu différent : refusé, original intact.
        store.put(&key, b"disk-bytes").unwrap();
        assert!(matches!(store.put(&key, b"autre"), Err(ObjectStoreError::Immutable)));
        assert_eq!(store.get(&key).unwrap(), b"disk-bytes");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn immutabilite_en_memoire() {
        let store = ObjectStore::memory_only();
        let key = cle();
        store.put(&key, b"v1").unwrap();
        assert!(matches!(store.put(&key, b"v2"), Err(ObjectStoreError::Immutable)));
        assert_eq!(store.get(&key).unwrap(), b"v1");
        assert_eq!(store.metrics().ecritures_echec, 1);
    }

    #[test]
    fn echec_d_ecriture_remonte_et_est_compte() {
        let (store, pannes) = avec_pannes();
        pannes.echec_ecriture.store(true, SeqCst);
        let key = cle();
        let e = store.put(&key, b"x").unwrap_err();
        assert!(matches!(e, ObjectStoreError::Backend(_)), "{e:?}");
        assert!(!store.exists(&key), "rien d'écrit après un échec");
        let m = store.metrics();
        assert_eq!((m.ecritures_ok, m.ecritures_echec), (0, 1));
    }

    #[test]
    fn ecriture_tronquee_silencieuse_est_detectee() {
        let (store, pannes) = avec_pannes();
        pannes.ecriture_tronquee.store(true, SeqCst);
        let e = store.put(&cle(), b"0123456789").unwrap_err();
        assert!(matches!(e, ObjectStoreError::Integrity), "{e:?}");
        assert_eq!(store.metrics().integrite_echec, 1);
    }

    #[test]
    fn lecture_corrompue_et_panne_de_lecture_remontent() {
        let (store, pannes) = avec_pannes();
        let key = cle();
        store.put(&key, b"contenu").unwrap();
        pannes.lecture_corrompue.store(true, SeqCst);
        assert!(matches!(store.get(&key), Err(ObjectStoreError::Integrity)));
        pannes.lecture_corrompue.store(false, SeqCst);
        assert_eq!(store.get(&key).unwrap(), b"contenu");
        pannes.echec_lecture.store(true, SeqCst);
        assert!(matches!(store.get(&key), Err(ObjectStoreError::Backend(_))));
        let m = store.metrics();
        assert_eq!((m.integrite_echec, m.lectures_echec), (1, 1));
    }

    #[test]
    fn panne_de_verification_n_est_pas_absent() {
        let (store, pannes) = avec_pannes();
        let key = cle();
        store.put(&key, b"x").unwrap();
        pannes.echec_head.store(true, SeqCst);
        assert!(store.existe(&key).is_err());
        assert_eq!(store.metrics().head_echec, 1);
    }

    #[test]
    fn lecture_controlee_contre_la_base() {
        let store = ObjectStore::memory_only();
        let key = cle();
        let info = store.put(&key, b"abc").unwrap();
        let ok = ObjectAttendu { taille: info.taille, sha256: info.sha256.clone() };
        assert_eq!(store.get_attendu(&key, &ok).unwrap(), b"abc");
        let mauvais = ObjectAttendu { taille: 3, sha256: sha256_hex(b"zzz") };
        assert!(matches!(store.get_attendu(&key, &mauvais), Err(ObjectStoreError::Integrity)));
        let mauvaise_taille = ObjectAttendu { taille: 4, sha256: info.sha256 };
        assert!(matches!(store.get_attendu(&key, &mauvaise_taille), Err(ObjectStoreError::Integrity)));
    }

    #[test]
    fn magasin_indisponible_refuse_tout() {
        let store = ObjectStore::unavailable();
        let key = cle();
        assert!(matches!(store.put(&key, b"x"), Err(ObjectStoreError::Unavailable)));
        assert!(matches!(store.get(&key), Err(ObjectStoreError::Unavailable)));
        assert!(store.existe(&key).is_err());
    }

    fn vars(paires: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| paires.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string())
    }

    const S3_OK: &[(&str, &str)] = &[
        ("ENT_OBJECT_S3_BUCKET", "bkt"),
        ("ENT_OBJECT_S3_ACCESS_KEY", "AK"),
        ("ENT_OBJECT_S3_SECRET_KEY", "SK"),
        ("ENT_OBJECT_S3_ENDPOINT", "https://s3.fr-par.scw.cloud"),
    ];

    #[test]
    fn production_exige_s3() {
        // Rien de configuré : refus (le binaire ne démarre pas).
        assert!(ObjectStore::from_vars(&vars(&[]), false).is_err());
        // Disque et mémoire interdits en production, même demandés.
        for b in ["disk", "memory"] {
            let v = move |k: &str| (k == "ENT_OBJECT_BACKEND").then(|| b.to_string());
            assert!(ObjectStore::from_vars(&v, false).is_err(), "{b}");
        }
        let v = |k: &str| (k == "ENT_OBJECT_MEMORY").then(|| "1".to_string());
        assert!(ObjectStore::from_vars(&v, false).is_err());
        // S3 incomplet (pas de clé secrète) : refus.
        assert!(ObjectStore::from_vars(&vars(&[("ENT_OBJECT_S3_BUCKET", "b")]), false).is_err());
        // S3 complet : accepté.
        let s = ObjectStore::from_vars(&vars(S3_OK), false).unwrap();
        assert!(s.is_s3());
        // HTTP clair refusé en production, toléré en labo (MinIO local).
        const HTTP: &[(&str, &str)] = &[
            ("ENT_OBJECT_S3_BUCKET", "bkt"),
            ("ENT_OBJECT_S3_ACCESS_KEY", "AK"),
            ("ENT_OBJECT_S3_SECRET_KEY", "SK"),
            ("ENT_OBJECT_S3_ENDPOINT", "http://127.0.0.1:9000"),
        ];
        assert!(ObjectStore::from_vars(&vars(HTTP), false).is_err());
        assert!(ObjectStore::from_vars(&vars(HTTP), true).unwrap().is_s3());
        assert_eq!(s.uri("a/b.pdf"), "s3://bkt/a/b.pdf");
    }

    #[test]
    fn labo_retombe_sur_le_disque() {
        let s = ObjectStore::from_vars(&vars(&[]), true).unwrap();
        assert!(s.is_disk());
        let m = ObjectStore::from_vars(&vars(&[("ENT_OBJECT_MEMORY", "1")]), true).unwrap();
        assert!(m.is_memory());
        let inconnu = ObjectStore::from_vars(&vars(&[("ENT_OBJECT_BACKEND", "ftp")]), true);
        assert!(inconnu.is_err());
    }

    #[test]
    fn reject_traversal() {
        let store = ObjectStore::memory_only();
        assert!(store.put("../x", b"no").is_err());
    }

    #[test]
    fn reject_unsafe_keys_everywhere() {
        let store = ObjectStore::memory_only();
        let long = "a".repeat(600);
        let bad = [
            "C:/Windows/win.ini",
            "C:foo",
            "\\\\?\\C:\\x",
            "/etc/passwd",
            "a/../b",
            "a//b",
            "./a",
            "a/./b",
            "a:b",
            "a\0b",
            "a\nb",
            "a\\b",
            "",
            "a/",
            long.as_str(),
        ];
        for k in bad {
            assert!(matches!(store.get(k), Err(ObjectStoreError::InvalidKey)), "get {k:?}");
            assert!(matches!(store.put(k, b"x"), Err(ObjectStoreError::InvalidKey)), "put {k:?}");
            assert!(!store.exists(k), "exists {k:?}");
        }
        assert_eq!(store.put_count(), 0);
    }

    #[test]
    fn not_found_ne_divulgue_pas_la_cle() {
        let store = ObjectStore::memory_only();
        let k = ObjectStore::key_for(Uuid::new_v4(), "trombi", Uuid::new_v4(), "jpg");
        let msg = store.get(&k).unwrap_err().to_string();
        assert!(!msg.contains(&k) && !msg.contains("trombi"), "{msg}");
    }

    #[test]
    fn cle_opaque_sans_donnee_personnelle() {
        let etab = Uuid::new_v4();
        let id = Uuid::new_v4();
        let k = ObjectStore::key_for(etab, "documents", id, ".PDF");
        assert_eq!(k, format!("{etab}/documents/{id}.pdf"));
    }
}
