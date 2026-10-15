//! Stockage objet ENT — filesystem local, mémoire lab, stub / live S3-compatible.
//!
//! - Racine : `ENT_OBJECT_DIR` ou `./.ent-objects` (créée à la volée).
//! - Clé : `{etablissement_id}/{namespace}/{id}` — jamais de path traversal.
//! - Dry-run mémoire si `ENT_OBJECT_MEMORY=1`.
//! - Stub S3 si `ENT_OBJECT_S3=1` (+ `ENT_OBJECT_S3_BUCKET`) : URI `s3://bucket/key`,
//!   contenu en mémoire (pas d'appel réseau).
//! - Live S3-path si `ENT_OBJECT_S3_LIVE=1` : persistance disque sous
//!   `{root}/{bucket}/{key}` + URI `s3://…` (compatible layout bucket).
//!   HTTP PUT optionnel si `ENT_OBJECT_S3_ENDPOINT` (ex. `http://127.0.0.1:9000`)
//!   + `ENT_OBJECT_S3_HTTP=1` — PUT path-style `{endpoint}/{bucket}/{key}`.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ObjectStoreError {
    #[error("clé objet invalide")]
    InvalidKey,
    #[error("objet introuvable: {0}")]
    NotFound(String),
    #[error("IO: {0}")]
    Io(String),
}

#[derive(Clone)]
pub struct ObjectStore {
    root: PathBuf,
    memory_only: bool,
    s3_stub: bool,
    s3_live: bool,
    s3_bucket: String,
    s3_endpoint: Option<String>,
    mem: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    put_count: Arc<AtomicUsize>,
}

impl ObjectStore {
    pub fn from_env() -> Self {
        let memory_only = env_flag("ENT_OBJECT_MEMORY");
        let s3_stub = env_flag("ENT_OBJECT_S3") && !env_flag("ENT_OBJECT_S3_LIVE");
        let s3_live = env_flag("ENT_OBJECT_S3_LIVE");
        let s3_bucket = std::env::var("ENT_OBJECT_S3_BUCKET")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "scola-ent".into());
        let s3_endpoint = std::env::var("ENT_OBJECT_S3_ENDPOINT")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let root = std::env::var("ENT_OBJECT_DIR")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                if s3_live {
                    PathBuf::from(".ent-objects-s3")
                } else {
                    PathBuf::from(".ent-objects")
                }
            });
        Self {
            root,
            memory_only: memory_only || s3_stub,
            s3_stub,
            s3_live,
            s3_bucket: sanitize_segment(&s3_bucket),
            s3_endpoint,
            mem: Arc::new(Mutex::new(HashMap::new())),
            put_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn memory_only() -> Self {
        Self {
            root: PathBuf::from(".ent-objects-unused"),
            memory_only: true,
            s3_stub: false,
            s3_live: false,
            s3_bucket: "unused".into(),
            s3_endpoint: None,
            mem: Arc::new(Mutex::new(HashMap::new())),
            put_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Stub S3 : même sémantique mémoire + URI `s3://bucket/key` (pas de réseau).
    pub fn s3_stub(bucket: &str) -> Self {
        let bucket = sanitize_segment(bucket);
        Self {
            root: PathBuf::from(".ent-objects-s3-stub"),
            memory_only: true,
            s3_stub: true,
            s3_live: false,
            s3_bucket: if bucket.is_empty() {
                "scola-ent".into()
            } else {
                bucket
            },
            s3_endpoint: None,
            mem: Arc::new(Mutex::new(HashMap::new())),
            put_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Live path : écrit sous `{root}/{bucket}/{key}` avec URI S3.
    pub fn s3_live(bucket: &str, root: PathBuf) -> Self {
        let bucket = sanitize_segment(bucket);
        Self {
            root,
            memory_only: false,
            s3_stub: false,
            s3_live: true,
            s3_bucket: if bucket.is_empty() {
                "scola-ent".into()
            } else {
                bucket
            },
            s3_endpoint: None,
            mem: Arc::new(Mutex::new(HashMap::new())),
            put_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn is_memory(&self) -> bool {
        self.memory_only
    }

    pub fn is_s3_stub(&self) -> bool {
        self.s3_stub
    }

    pub fn is_s3_live(&self) -> bool {
        self.s3_live
    }

    pub fn s3_bucket(&self) -> &str {
        &self.s3_bucket
    }

    pub fn s3_endpoint(&self) -> Option<&str> {
        self.s3_endpoint.as_deref()
    }

    pub fn put_count(&self) -> usize {
        self.put_count.load(Ordering::Relaxed)
    }

    /// URI logique S3 (stub ou live) pour journalisation.
    pub fn s3_uri(&self, key: &str) -> String {
        format!("s3://{}/{}", self.s3_bucket, key)
    }

    /// Construit une clé sûre pour un artefact métier.
    pub fn key_for(etablissement_id: Uuid, namespace: &str, id: Uuid, ext: &str) -> String {
        let ns = sanitize_segment(namespace);
        let ext = sanitize_ext(ext);
        format!("{etablissement_id}/{ns}/{id}.{ext}")
    }

    fn disk_path(&self, key: &str) -> PathBuf {
        if self.s3_live {
            self.root.join(&self.s3_bucket).join(key)
        } else {
            self.root.join(key)
        }
    }

    pub fn put(&self, key: &str, bytes: &[u8]) -> Result<(), ObjectStoreError> {
        assert_safe_key(key)?;
        if self.memory_only {
            self.mem
                .lock()
                .map_err(|e| ObjectStoreError::Io(e.to_string()))?
                .insert(key.to_string(), bytes.to_vec());
            self.put_count.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        let path = self.disk_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ObjectStoreError::Io(e.to_string()))?;
        }
        fs::write(&path, bytes).map_err(|e| ObjectStoreError::Io(e.to_string()))?;
        self.put_count.fetch_add(1, Ordering::Relaxed);
        if self.s3_live && env_flag("ENT_OBJECT_S3_HTTP") {
            if let Some(ep) = &self.s3_endpoint {
                // Best-effort : disque local reste source de vérité si HTTP échoue.
                let _ = http_put_object(ep, &self.s3_bucket, key, bytes);
            }
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Vec<u8>, ObjectStoreError> {
        assert_safe_key(key)?;
        if self.memory_only {
            return self
                .mem
                .lock()
                .map_err(|e| ObjectStoreError::Io(e.to_string()))?
                .get(key)
                .cloned()
                .ok_or_else(|| ObjectStoreError::NotFound(key.to_string()));
        }
        let path = self.disk_path(key);
        fs::read(&path).map_err(|_| ObjectStoreError::NotFound(key.to_string()))
    }

    pub fn exists(&self, key: &str) -> bool {
        self.get(key).is_ok()
    }

    pub fn memory_len(&self) -> usize {
        self.mem.lock().map(|g| g.len()).unwrap_or(0)
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

fn sanitize_ext(s: &str) -> String {
    let t = s.trim().trim_start_matches('.').to_ascii_lowercase();
    if t.is_empty() {
        "bin".into()
    } else {
        sanitize_segment(&t)
    }
}

fn assert_safe_key(key: &str) -> Result<(), ObjectStoreError> {
    if key.is_empty() || key.contains("..") || key.starts_with('/') || key.starts_with('\\') {
        return Err(ObjectStoreError::InvalidKey);
    }
    if key.contains('\\') {
        return Err(ObjectStoreError::InvalidKey);
    }
    Ok(())
}

fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let t = v.trim();
            t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

/// PUT HTTP path-style minimal (pas de signature AWS) — lab / MinIO sans auth.
fn http_put_object(
    endpoint: &str,
    bucket: &str,
    key: &str,
    body: &[u8],
) -> Result<(), ObjectStoreError> {
    let endpoint = endpoint.trim().trim_end_matches('/');
    let without_scheme = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("https://"))
        .unwrap_or(endpoint);
    let (host_port, _) = without_scheme
        .split_once('/')
        .unwrap_or((without_scheme, ""));
    let addr = if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{host_port}:80")
    };
    let path = format!("/{bucket}/{key}");
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| ObjectStoreError::Io(format!("s3 http connect {addr}: {e}")))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .ok();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .ok();
    let req = format!(
        "PUT {path} HTTP/1.1\r\nHost: {host_port}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(req.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|e| ObjectStoreError::Io(format!("s3 http write: {e}")))?;
    let mut buf = [0u8; 128];
    let _ = stream.read(&mut buf);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn memory_roundtrip() {
        let store = ObjectStore::memory_only();
        let id = Uuid::new_v4();
        let etab = Uuid::new_v4();
        let key = ObjectStore::key_for(etab, "pai", id, "pdf");
        store.put(&key, b"%PDF-1.4").unwrap();
        assert_eq!(store.get(&key).unwrap(), b"%PDF-1.4");
        assert!(store.exists(&key));
        assert_eq!(store.put_count(), 1);
    }

    #[test]
    fn s3_stub_uri() {
        let store = ObjectStore::s3_stub("scola-lab");
        assert!(store.is_s3_stub());
        assert!(!store.is_s3_live());
        let key = ObjectStore::key_for(Uuid::new_v4(), "pai", Uuid::new_v4(), "pdf");
        store.put(&key, b"x").unwrap();
        assert!(store.s3_uri(&key).starts_with("s3://scola-lab/"));
        assert_eq!(store.get(&key).unwrap(), b"x");
    }

    #[test]
    fn s3_live_disk_roundtrip() {
        let dir = env::temp_dir().join(format!("ent-obj-live-{}", Uuid::new_v4()));
        let store = ObjectStore::s3_live("scola-lab", dir.clone());
        assert!(store.is_s3_live());
        assert!(!store.is_s3_stub());
        let key = ObjectStore::key_for(Uuid::new_v4(), "pai", Uuid::new_v4(), "pdf");
        store.put(&key, b"live-bytes").unwrap();
        assert_eq!(store.get(&key).unwrap(), b"live-bytes");
        let on_disk = dir.join("scola-lab").join(&key);
        assert!(on_disk.is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reject_traversal() {
        let store = ObjectStore::memory_only();
        assert!(store.put("../x", b"no").is_err());
    }
}
