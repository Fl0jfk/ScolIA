//! Fournisseurs de stockage d'objets : le trait et les implémentations hors réseau.
//!
//! La logique métier ne connaît que `ObjectStore` (lib.rs) ; il délègue à un
//! `StorageProvider`. Contrat commun (testé pour chaque implémentation) :
//!
//! - **objets immuables** : écrire une clé qui existe avec un contenu *différent* est
//!   une erreur (`Immutable`) ; réécrire le même contenu est accepté (idempotent) ;
//! - l'empreinte SHA-256 (hex) est transmise à l'écriture et conservée avec l'objet ;
//! - **aucun échec silencieux** : toute erreur remonte.
//!
//! Implémentations : `MemoryProvider` (tests), `DiskProvider` (labo), `S3Provider`
//! (production, voir `s3.rs`), `FaultyProvider` (tests de panne).

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::ObjectStoreError;

/// Métadonnées d'un objet telles que le fournisseur les relit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectHead {
    pub taille: u64,
    /// Empreinte SHA-256 (hex minuscule) conservée à l'écriture ; `None` pour un objet
    /// historique écrit sans empreinte.
    pub sha256: Option<String>,
}

/// Contenu relu + empreinte conservée avec l'objet.
#[derive(Debug, Clone)]
pub struct StoredObject {
    pub bytes: Vec<u8>,
    pub sha256: Option<String>,
}

pub trait StorageProvider: Send + Sync {
    /// Nom court pour les journaux et métriques (`s3`, `disque`, `memoire`…).
    fn nom(&self) -> &'static str;
    /// URI logique, sans secret (journalisation).
    fn uri(&self, key: &str) -> String;
    /// Écrit l'objet. `sha256` est l'empreinte hex des `bytes` (déjà calculée).
    /// Clé existante + contenu différent → `Immutable` ; même contenu → `Ok`.
    fn put(&self, key: &str, bytes: &[u8], sha256: &str) -> Result<(), ObjectStoreError>;
    fn get(&self, key: &str) -> Result<StoredObject, ObjectStoreError>;
    /// `Ok(None)` si l'objet n'existe pas ; toute autre difficulté est une erreur.
    fn head(&self, key: &str) -> Result<Option<ObjectHead>, ObjectStoreError>;
    /// Supprime **physiquement** l'objet (purge, socle 7 D9). Idempotent : une clé absente est un succès.
    /// Un fournisseur qui ne sait pas supprimer répond une erreur (jamais un faux succès).
    fn delete(&self, _key: &str) -> Result<(), ObjectStoreError> {
        Err(ObjectStoreError::Backend(
            "suppression non prise en charge par ce fournisseur".into(),
        ))
    }
}

// ───────────────────────────── Mémoire ─────────────────────────────

#[derive(Default)]
pub struct MemoryProvider {
    objets: Mutex<HashMap<String, (Vec<u8>, String)>>,
}

impl MemoryProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.objets.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn verrou<T>(e: std::sync::PoisonError<T>) -> ObjectStoreError {
    ObjectStoreError::Io(format!("verrou empoisonné: {e}"))
}

impl StorageProvider for MemoryProvider {
    fn nom(&self) -> &'static str {
        "memoire"
    }

    fn uri(&self, key: &str) -> String {
        format!("mem://{key}")
    }

    fn put(&self, key: &str, bytes: &[u8], sha256: &str) -> Result<(), ObjectStoreError> {
        let mut g = self.objets.lock().map_err(verrou)?;
        if let Some((_, deja)) = g.get(key) {
            return if deja == sha256 {
                Ok(())
            } else {
                Err(ObjectStoreError::Immutable)
            };
        }
        g.insert(key.to_string(), (bytes.to_vec(), sha256.to_string()));
        Ok(())
    }

    fn get(&self, key: &str) -> Result<StoredObject, ObjectStoreError> {
        let g = self.objets.lock().map_err(verrou)?;
        g.get(key)
            .map(|(b, h)| StoredObject {
                bytes: b.clone(),
                sha256: Some(h.clone()),
            })
            .ok_or_else(|| ObjectStoreError::NotFound(key.to_string()))
    }

    fn head(&self, key: &str) -> Result<Option<ObjectHead>, ObjectStoreError> {
        let g = self.objets.lock().map_err(verrou)?;
        Ok(g.get(key).map(|(b, h)| ObjectHead {
            taille: b.len() as u64,
            sha256: Some(h.clone()),
        }))
    }

    fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        self.objets.lock().map_err(verrou)?.remove(key);
        Ok(())
    }
}

// ───────────────────────────── Disque (labo) ─────────────────────────────

/// Disque local : **labo et tests uniquement** (jamais en production : `from_env_strict`
/// le refuse). Écriture atomique (fichier temporaire + renommage), empreinte dans un
/// fichier voisin `{clé}.sha256`.
pub struct DiskProvider {
    root: PathBuf,
}

impl DiskProvider {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    fn chemin(&self, key: &str) -> Result<PathBuf, ObjectStoreError> {
        // Le suffixe `.sha256` est réservé aux fichiers d'empreinte.
        if key.ends_with(".sha256") || key.contains(".tmp-") {
            return Err(ObjectStoreError::InvalidKey);
        }
        Ok(self.root.join(key))
    }
}

fn io(e: std::io::Error) -> ObjectStoreError {
    ObjectStoreError::Io(e.kind().to_string())
}

fn ecrire_atomique(path: &std::path::Path, bytes: &[u8]) -> Result<(), ObjectStoreError> {
    let tmp = path.with_extension(format!("tmp-{}", Uuid::new_v4().simple()));
    let res = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res.map_err(io)
}

impl StorageProvider for DiskProvider {
    fn nom(&self) -> &'static str {
        "disque"
    }

    fn uri(&self, key: &str) -> String {
        format!("file://{key}")
    }

    fn put(&self, key: &str, bytes: &[u8], sha256: &str) -> Result<(), ObjectStoreError> {
        let path = self.chemin(key)?;
        if let Some(h) = self.head(key)? {
            return if h.sha256.as_deref() == Some(sha256) && h.taille == bytes.len() as u64 {
                Ok(())
            } else {
                Err(ObjectStoreError::Immutable)
            };
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        // Empreinte d'abord : un objet sans empreinte ne doit pas apparaître.
        let mut sidecar = path.clone().into_os_string();
        sidecar.push(".sha256");
        ecrire_atomique(&PathBuf::from(sidecar), sha256.as_bytes())?;
        ecrire_atomique(&path, bytes)
    }

    fn get(&self, key: &str) -> Result<StoredObject, ObjectStoreError> {
        let path = self.chemin(key)?;
        let bytes = fs::read(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ObjectStoreError::NotFound(key.to_string()),
            _ => io(e),
        })?;
        let mut sidecar = path.into_os_string();
        sidecar.push(".sha256");
        let sha256 = fs::read_to_string(PathBuf::from(sidecar))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        Ok(StoredObject { bytes, sha256 })
    }

    fn head(&self, key: &str) -> Result<Option<ObjectHead>, ObjectStoreError> {
        match self.get(key) {
            Ok(o) => Ok(Some(ObjectHead {
                taille: o.bytes.len() as u64,
                sha256: o.sha256,
            })),
            Err(ObjectStoreError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        let path = self.chemin(key)?;
        let mut sidecar = path.clone().into_os_string();
        sidecar.push(".sha256");
        for f in [path, PathBuf::from(sidecar)] {
            match fs::remove_file(&f) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io(e)),
            }
        }
        Ok(())
    }
}

// ───────────────────────────── Pannes simulées (tests) ─────────────────────────────

/// Enveloppe de test : injecte des pannes pour vérifier qu'elles **remontent**.
/// Les drapeaux se changent à chaud (`Arc` partagé avec le test).
#[derive(Default)]
pub struct Pannes {
    /// `put` renvoie une erreur.
    pub echec_ecriture: AtomicBool,
    /// `put` « réussit » mais l'objet stocké est tronqué (perte silencieuse simulée).
    pub ecriture_tronquee: AtomicBool,
    /// `get` renvoie une erreur.
    pub echec_lecture: AtomicBool,
    /// `get` renvoie des octets altérés avec l'empreinte d'origine (corruption au repos).
    pub lecture_corrompue: AtomicBool,
    /// `head` renvoie une erreur.
    pub echec_head: AtomicBool,
    /// `delete` renvoie une erreur.
    pub echec_suppression: AtomicBool,
}

pub struct FaultyProvider {
    interne: Arc<dyn StorageProvider>,
    pub pannes: Arc<Pannes>,
}

impl FaultyProvider {
    pub fn new(interne: Arc<dyn StorageProvider>) -> (Self, Arc<Pannes>) {
        let pannes = Arc::new(Pannes::default());
        (
            Self {
                interne,
                pannes: pannes.clone(),
            },
            pannes,
        )
    }
}

fn actif(b: &AtomicBool) -> bool {
    b.load(Ordering::SeqCst)
}

impl StorageProvider for FaultyProvider {
    fn nom(&self) -> &'static str {
        "panne-simulee"
    }

    fn uri(&self, key: &str) -> String {
        self.interne.uri(key)
    }

    fn put(&self, key: &str, bytes: &[u8], sha256: &str) -> Result<(), ObjectStoreError> {
        if actif(&self.pannes.echec_ecriture) {
            return Err(ObjectStoreError::Backend("panne simulée (écriture)".into()));
        }
        if actif(&self.pannes.ecriture_tronquee) {
            let coupe = &bytes[..bytes.len() / 2];
            return self.interne.put(key, coupe, sha256);
        }
        self.interne.put(key, bytes, sha256)
    }

    fn get(&self, key: &str) -> Result<StoredObject, ObjectStoreError> {
        if actif(&self.pannes.echec_lecture) {
            return Err(ObjectStoreError::Backend("panne simulée (lecture)".into()));
        }
        let mut o = self.interne.get(key)?;
        if actif(&self.pannes.lecture_corrompue) {
            if let Some(b) = o.bytes.first_mut() {
                *b ^= 0xFF;
            } else {
                o.bytes.push(1);
            }
        }
        Ok(o)
    }

    fn head(&self, key: &str) -> Result<Option<ObjectHead>, ObjectStoreError> {
        if actif(&self.pannes.echec_head) {
            return Err(ObjectStoreError::Backend("panne simulée (head)".into()));
        }
        self.interne.head(key)
    }

    fn delete(&self, key: &str) -> Result<(), ObjectStoreError> {
        if actif(&self.pannes.echec_suppression) {
            return Err(ObjectStoreError::Backend("panne simulée (suppression)".into()));
        }
        self.interne.delete(key)
    }
}

#[cfg(test)]
mod tests_suppression {
    use super::*;

    fn sha(b: &[u8]) -> String {
        crate::sha256_hex(b)
    }

    #[test]
    fn memoire_supprime_et_tolere_l_absent() {
        let m = MemoryProvider::new();
        m.put("e/a/b.bin", b"x", &sha(b"x")).unwrap();
        m.delete("e/a/b.bin").unwrap();
        assert!(m.head("e/a/b.bin").unwrap().is_none());
        m.delete("e/a/b.bin").unwrap();
    }

    #[test]
    fn disque_supprime_objet_et_empreinte() {
        let dir = std::env::temp_dir().join(format!("ent-del-{}", std::process::id()));
        let d = DiskProvider::new(dir.clone());
        d.put("e/a/b.bin", b"xyz", &sha(b"xyz")).unwrap();
        d.delete("e/a/b.bin").unwrap();
        assert!(d.head("e/a/b.bin").unwrap().is_none());
        assert!(!dir.join("e/a/b.bin.sha256").exists());
        d.delete("e/a/b.bin").unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn panne_de_suppression_remonte_et_defaut_refuse() {
        let (f, pannes) = FaultyProvider::new(Arc::new(MemoryProvider::new()));
        f.put("e/a/b.bin", b"x", &sha(b"x")).unwrap();
        pannes.echec_suppression.store(true, Ordering::SeqCst);
        assert!(f.delete("e/a/b.bin").is_err());
        assert!(f.head("e/a/b.bin").unwrap().is_some(), "rien n'a été supprimé");
        pannes.echec_suppression.store(false, Ordering::SeqCst);
        f.delete("e/a/b.bin").unwrap();
        // Un fournisseur qui ne sait pas supprimer ne ment pas.
        struct Muet;
        impl StorageProvider for Muet {
            fn nom(&self) -> &'static str {
                "muet"
            }
            fn uri(&self, k: &str) -> String {
                k.into()
            }
            fn put(&self, _: &str, _: &[u8], _: &str) -> Result<(), ObjectStoreError> {
                Ok(())
            }
            fn get(&self, k: &str) -> Result<StoredObject, ObjectStoreError> {
                Err(ObjectStoreError::NotFound(k.into()))
            }
            fn head(&self, _: &str) -> Result<Option<ObjectHead>, ObjectStoreError> {
                Ok(None)
            }
        }
        assert!(Muet.delete("e/a/b.bin").is_err());
    }
}
