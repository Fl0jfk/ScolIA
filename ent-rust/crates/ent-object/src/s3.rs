//! Fournisseur S3-compatible (Scaleway Object Storage, OVHcloud Object Storage, MinIO…).
//!
//! - Signature **AWS SigV4 en-têtes** (`Authorization`), région, endpoint et style de
//!   chemin / hôte virtuel configurables (`S3Config`, partagé avec les liens pré-signés).
//! - Client HTTP : `ureq` (synchrone, rustls). Délais de connexion et totaux bornés ;
//!   deux nouvelles tentatives sur erreur réseau ou 5xx (écritures idempotentes : clé
//!   immuable + même contenu).
//! - Écriture : `x-amz-content-sha256` = empreinte réelle du contenu (le serveur refuse
//!   un corps qui ne correspond pas) et métadonnée `x-amz-meta-sha256` conservée avec
//!   l'objet pour la vérification à la lecture.
//! - **Aucun secret ni clé d'objet dans les messages d'erreur.**

use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::presign::{amz_dates, hex, hmac_raw, S3Config};
use crate::provider::{ObjectHead, StorageProvider, StoredObject};
use crate::ObjectStoreError;

/// Taille maximale lue pour un objet (défense contre un serveur hostile).
const MAX_OBJET: u64 = 256 * 1024 * 1024;
const META_SHA: &str = "x-amz-meta-sha256";

/// Requête signée : URL et en-têtes à poser (hors `Host`, posé par le client).
#[derive(Debug, Clone)]
pub(crate) struct RequeteSignee {
    pub url: String,
    #[cfg(test)]
    pub host: String,
    pub en_tetes: Vec<(String, String)>,
}

/// Signature SigV4 par en-têtes. `extra` : en-têtes supplémentaires signés (noms
/// quelconques ; mis en minuscules). `payload_sha256` : empreinte hex du corps.
pub(crate) fn signer(
    cfg: &S3Config,
    methode: &str,
    key: &str,
    payload_sha256: &str,
    extra: &[(&str, &str)],
    now_unix: i64,
) -> RequeteSignee {
    let (scheme, host, path) = cfg.cible(key);
    let (date, amz_date) = amz_dates(now_unix);
    let mut entetes: Vec<(String, String)> = vec![
        ("host".into(), host.clone()),
        ("x-amz-content-sha256".into(), payload_sha256.into()),
        ("x-amz-date".into(), amz_date.clone()),
    ];
    for (k, v) in extra {
        entetes.push((k.to_ascii_lowercase(), v.trim().to_string()));
    }
    entetes.sort();
    let canoniques: String = entetes.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();
    let signes = entetes
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let requete = format!("{methode}\n{path}\n\n{canoniques}\n{signes}\n{payload_sha256}");
    let scope = format!("{date}/{}/s3/aws4_request", cfg.region);
    let a_signer = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        hex(&Sha256::digest(requete.as_bytes()))
    );
    let k_date = hmac_raw(format!("AWS4{}", cfg.secret()).as_bytes(), date.as_bytes());
    let k_region = hmac_raw(&k_date, cfg.region.as_bytes());
    let k_service = hmac_raw(&k_region, b"s3");
    let k_signature = hmac_raw(&k_service, b"aws4_request");
    let signature = hex(&hmac_raw(&k_signature, a_signer.as_bytes()));
    let autorisation = format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signes}, Signature={signature}",
        cfg.access_key
    );
    let mut en_tetes: Vec<(String, String)> =
        entetes.into_iter().filter(|(k, _)| k != "host").collect();
    en_tetes.push(("authorization".into(), autorisation));
    RequeteSignee {
        url: format!("{scheme}://{host}{path}"),
        #[cfg(test)]
        host,
        en_tetes,
    }
}

pub struct S3Provider {
    cfg: S3Config,
    agent: ureq::Agent,
    tentatives: u32,
}

impl S3Provider {
    pub fn new(cfg: S3Config) -> Self {
        let config = ureq::Agent::config_builder()
            // Les statuts 4xx/5xx sont traités ici (jamais d'erreur d'API opaque).
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(5)))
            .timeout_global(Some(Duration::from_secs(60)))
            .build();
        Self {
            cfg,
            agent: ureq::Agent::new_with_config(config),
            tentatives: 3,
        }
    }

    /// Nombre total de tentatives (≥ 1) ; utile aux tests.
    pub fn avec_tentatives(mut self, n: u32) -> Self {
        self.tentatives = n.max(1);
        self
    }

    fn now() -> i64 {
        crate::presign::now_unix()
    }

    /// Envoie une requête signée ; renvoie `(statut, en-têtes utiles, corps)`.
    fn envoyer(
        &self,
        methode: &str,
        key: &str,
        corps: Option<&[u8]>,
        sha256: &str,
        extra: &[(&str, &str)],
    ) -> Result<Reponse, ObjectStoreError> {
        let mut derniere = ObjectStoreError::Backend("aucune tentative".into());
        for essai in 0..self.tentatives {
            if essai > 0 {
                std::thread::sleep(Duration::from_millis(150 * u64::from(essai)));
            }
            // Signature recalculée à chaque tentative (horodatage frais).
            let req = signer(&self.cfg, methode, key, sha256, extra, Self::now());
            let rep = self.une_requete(methode, &req, corps);
            match rep {
                Ok(r) if r.statut >= 500 => {
                    derniere = ObjectStoreError::Backend(format!("S3 a répondu {}", r.statut));
                }
                Ok(r) => return Ok(r),
                Err(e) => derniere = e,
            }
        }
        Err(derniere)
    }

    fn une_requete(
        &self,
        methode: &str,
        req: &RequeteSignee,
        corps: Option<&[u8]>,
    ) -> Result<Reponse, ObjectStoreError> {
        let reseau =
            |e: ureq::Error| ObjectStoreError::Backend(format!("réseau S3: {}", classe(&e)));
        let resultat = match methode {
            "PUT" => {
                let mut r = self.agent.put(&req.url);
                for (k, v) in &req.en_tetes {
                    r = r.header(k, v);
                }
                r.send(corps.unwrap_or(&[]))
            }
            "HEAD" => {
                let mut r = self.agent.head(&req.url);
                for (k, v) in &req.en_tetes {
                    r = r.header(k, v);
                }
                r.call()
            }
            _ => {
                let mut r = self.agent.get(&req.url);
                for (k, v) in &req.en_tetes {
                    r = r.header(k, v);
                }
                r.call()
            }
        };
        let mut rep = resultat.map_err(reseau)?;
        let statut = rep.status().as_u16();
        let sha = rep
            .headers()
            .get(META_SHA)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let taille_annoncee = rep
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let corps = if methode == "GET" && (200..300).contains(&statut) {
            rep.body_mut()
                .with_config()
                .limit(MAX_OBJET)
                .read_to_vec()
                .map_err(reseau)?
        } else {
            Vec::new()
        };
        Ok(Reponse {
            statut,
            sha256: sha,
            taille_annoncee,
            corps,
        })
    }
}

struct Reponse {
    statut: u16,
    sha256: Option<String>,
    taille_annoncee: Option<u64>,
    corps: Vec<u8>,
}

/// Classe d'erreur sans détail sensible (pas d'URL, pas de clé).
fn classe(e: &ureq::Error) -> &'static str {
    match e {
        ureq::Error::Timeout(_) => "délai dépassé",
        ureq::Error::HostNotFound => "hôte introuvable",
        ureq::Error::ConnectionFailed => "connexion refusée",
        ureq::Error::Io(_) => "erreur d'entrée-sortie",
        ureq::Error::Tls(_) | ureq::Error::Rustls(_) => "erreur TLS",
        _ => "erreur de transport",
    }
}

fn statut_erreur(statut: u16) -> ObjectStoreError {
    ObjectStoreError::Backend(format!("S3 a répondu {statut}"))
}

impl StorageProvider for S3Provider {
    fn nom(&self) -> &'static str {
        "s3"
    }

    fn uri(&self, key: &str) -> String {
        format!("s3://{}/{key}", self.cfg.bucket)
    }

    fn put(&self, key: &str, bytes: &[u8], sha256: &str) -> Result<(), ObjectStoreError> {
        // Immutabilité : l'objet existe-t-il déjà ? (403 d'un `HEAD` sur clé absente est
        // fréquent sans droit de listage : on laisse alors l'écriture trancher.)
        let r = self.envoyer("HEAD", key, None, EMPTY_SHA256, &[])?;
        match r.statut {
            200..=299 => {
                return if r.sha256.as_deref() == Some(sha256)
                    && r.taille_annoncee == Some(bytes.len() as u64)
                {
                    Ok(())
                } else {
                    Err(ObjectStoreError::Immutable)
                };
            }
            403 | 404 => {}
            s => return Err(statut_erreur(s)),
        }
        let r = self.envoyer("PUT", key, Some(bytes), sha256, &[(META_SHA, sha256)])?;
        match r.statut {
            200..=299 => Ok(()),
            s => Err(statut_erreur(s)),
        }
    }

    fn get(&self, key: &str) -> Result<StoredObject, ObjectStoreError> {
        let r = self.envoyer("GET", key, None, EMPTY_SHA256, &[])?;
        match r.statut {
            200..=299 => Ok(StoredObject {
                bytes: r.corps,
                sha256: r.sha256,
            }),
            404 => Err(ObjectStoreError::NotFound(key.to_string())),
            s => Err(statut_erreur(s)),
        }
    }

    fn head(&self, key: &str) -> Result<Option<ObjectHead>, ObjectStoreError> {
        let r = self.envoyer("HEAD", key, None, EMPTY_SHA256, &[])?;
        match r.statut {
            200..=299 => Ok(Some(ObjectHead {
                taille: r.taille_annoncee.ok_or_else(|| {
                    ObjectStoreError::Backend("S3: taille absente de la réponse".into())
                })?,
                sha256: r.sha256,
            })),
            404 => Ok(None),
            s => Err(statut_erreur(s)),
        }
    }
}

/// SHA-256 du corps vide.
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// Vecteur officiel « GET Object » de la documentation AWS SigV4 (en-têtes).
    #[test]
    fn vecteur_aws_get_object() {
        let cfg = S3Config::new(
            "examplebucket",
            "us-east-1",
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            Some("https://s3.amazonaws.com"),
            true,
        )
        .unwrap();
        let r = signer(
            &cfg,
            "GET",
            "test.txt",
            EMPTY_SHA256,
            &[("Range", "bytes=0-9")],
            1_369_353_600, // 2013-05-24T00:00:00Z
        );
        assert_eq!(r.host, "examplebucket.s3.amazonaws.com");
        let auth = &r
            .en_tetes
            .iter()
            .find(|(k, _)| k == "authorization")
            .unwrap()
            .1;
        assert!(
            auth.contains(
                "Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
            ),
            "{auth}"
        );
        assert!(auth.contains("SignedHeaders=host;range;x-amz-content-sha256;x-amz-date"));
    }

    #[test]
    fn style_chemin_et_hote_virtuel() {
        let chemin = S3Config::new(
            "b",
            "fr-par",
            "k",
            "s",
            Some("https://s3.fr-par.scw.cloud"),
            false,
        )
        .unwrap();
        let (_, h, p) = chemin.cible("e/ns/id.pdf");
        assert_eq!(
            (h.as_str(), p.as_str()),
            ("s3.fr-par.scw.cloud", "/b/e/ns/id.pdf")
        );
        let virt = S3Config::new(
            "b",
            "fr-par",
            "k",
            "s",
            Some("https://s3.fr-par.scw.cloud"),
            true,
        )
        .unwrap();
        let (_, h, p) = virt.cible("e/ns/id.pdf");
        assert_eq!(
            (h.as_str(), p.as_str()),
            ("b.s3.fr-par.scw.cloud", "/e/ns/id.pdf")
        );
    }

    /// Mini-serveur S3 : enregistre les requêtes, répond selon un script.
    struct Faux {
        port: u16,
        vues: Arc<Mutex<Vec<String>>>,
    }

    fn faux_serveur(reponses: Vec<String>) -> Faux {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let vues = Arc::new(Mutex::new(Vec::new()));
        let v2 = vues.clone();
        std::thread::spawn(move || {
            for rep in reponses {
                let Ok((mut s, _)) = l.accept() else { return };
                let mut buf = vec![0u8; 65536];
                let n = s.read(&mut buf).unwrap_or(0);
                v2.lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&buf[..n]).to_string());
                let _ = s.write_all(rep.as_bytes());
            }
        });
        Faux { port, vues }
    }

    fn cfg(port: u16) -> S3Config {
        S3Config::new(
            "bkt",
            "fr-par",
            "AKTEST",
            "SECRETTEST",
            Some(&format!("http://127.0.0.1:{port}")),
            false,
        )
        .unwrap()
    }

    fn http(statut: &str, entetes: &str, corps: &str) -> String {
        format!(
            "HTTP/1.1 {statut}\r\n{entetes}Content-Length: {}\r\nConnection: close\r\n\r\n{corps}",
            corps.len()
        )
    }

    #[test]
    fn put_signe_et_remonte_l_echec_serveur() {
        let sha = hex(&Sha256::digest(b"contenu"));
        // HEAD → 404, PUT → 500 (x1 tentative) : l'erreur remonte.
        let f = faux_serveur(vec![
            http("404 Not Found", "", ""),
            http("500 Internal Server Error", "", ""),
        ]);
        let p = S3Provider::new(cfg(f.port)).avec_tentatives(1);
        let e = p.put("e/ns/id.pdf", b"contenu", &sha).unwrap_err();
        assert!(matches!(e, ObjectStoreError::Backend(_)), "{e:?}");
        let vues = f.vues.lock().unwrap();
        let put = vues
            .iter()
            .find(|r| r.starts_with("PUT "))
            .expect("PUT émis");
        let bas = put.to_ascii_lowercase();
        assert!(put.starts_with("PUT /bkt/e/ns/id.pdf "), "{put}");
        assert!(bas.contains("authorization: aws4-hmac-sha256 credential=aktest/"));
        assert!(bas.contains(&format!("x-amz-content-sha256: {sha}")));
        assert!(bas.contains(&format!("x-amz-meta-sha256: {sha}")));
        assert!(
            !put.contains("SECRETTEST"),
            "le secret ne doit jamais être émis"
        );
    }

    #[test]
    fn put_refuse_la_reecriture_differente() {
        let ancien = hex(&Sha256::digest(b"ancien"));
        let nouveau = hex(&Sha256::digest(b"nouveau"));
        let f = faux_serveur(vec![http(
            "200 OK",
            &format!("x-amz-meta-sha256: {ancien}\r\n"),
            "",
        )]);
        // HEAD sans corps : `Content-Length` du faux serveur vaut 0 → taille ≠ 7 : immuable.
        let p = S3Provider::new(cfg(f.port)).avec_tentatives(1);
        let e = p.put("e/ns/id.pdf", b"nouveau", &nouveau).unwrap_err();
        assert!(matches!(e, ObjectStoreError::Immutable), "{e:?}");
    }

    #[test]
    fn get_404_et_5xx_sont_distingues() {
        let f = faux_serveur(vec![http("404 Not Found", "", "")]);
        let p = S3Provider::new(cfg(f.port)).avec_tentatives(1);
        assert!(matches!(
            p.get("a/b/c.pdf"),
            Err(ObjectStoreError::NotFound(_))
        ));
        let f = faux_serveur(vec![http("503 Service Unavailable", "", "")]);
        let p = S3Provider::new(cfg(f.port)).avec_tentatives(1);
        assert!(matches!(
            p.get("a/b/c.pdf"),
            Err(ObjectStoreError::Backend(_))
        ));
    }

    #[test]
    fn get_renvoie_contenu_et_empreinte_conservee() {
        let f = faux_serveur(vec![http(
            "200 OK",
            "x-amz-meta-sha256: abc123\r\n",
            "hello",
        )]);
        let p = S3Provider::new(cfg(f.port)).avec_tentatives(1);
        let o = p.get("a/b/c.pdf").unwrap();
        assert_eq!(o.bytes, b"hello");
        assert_eq!(o.sha256.as_deref(), Some("abc123"));
    }

    #[test]
    fn serveur_injoignable_remonte() {
        // Port fermé : aucune tentative ne réussit.
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        let p = S3Provider::new(cfg(port)).avec_tentatives(2);
        assert!(matches!(
            p.get("a/b/c.pdf"),
            Err(ObjectStoreError::Backend(_))
        ));
    }
}
