//! Transport HTTP du SSO — trait injectable + implémentation réelle (reqwest).

use std::time::Duration;

use async_trait::async_trait;

use crate::ssrf::{cible_redirection, resoudre_controle, valider_url, UrlPolicy};
use crate::OidcError;

/// Accès réseau minimal dont le SSO a besoin (découverte, JWKS, échange de
/// code). Isolé pour que la logique soit testable sans réseau.
#[async_trait]
pub trait OidcTransport: Send + Sync {
    async fn get_json(&self, url: &str) -> Result<serde_json::Value, OidcError>;
    async fn post_form_json(
        &self,
        url: &str,
        form: &[(String, String)],
    ) -> Result<serde_json::Value, OidcError>;
}

/// Redirections suivies au maximum (chaque cible est revalidée).
const MAX_REDIRECTS: usize = 3;
/// Taille maximale d'une réponse (découverte, JWKS, tokens) : 1 Mio par défaut,
/// surchargeable via `ENT_OIDC_MAX_BYTES`.
const DEFAULT_MAX_BYTES: usize = 1024 * 1024;

/// Implémentation réelle : reqwest + rustls, délais courts (un IdP lent ne doit
/// pas immobiliser un worker), garde SSRF ([`UrlPolicy`]), redirections suivies
/// à la main, taille de réponse bornée.
///
/// Un client est construit **par requête** : l'adresse IP validée après
/// résolution DNS est épinglée (`resolve_to_addrs`), ce qui exclut le DNS
/// rebinding entre le contrôle et la connexion.
pub struct HttpOidcTransport {
    timeout: Duration,
    policy: UrlPolicy,
    max_bytes: usize,
}

impl HttpOidcTransport {
    /// Politique **stricte** par défaut ; voir [`HttpOidcTransport::with_policy`].
    pub fn new(timeout_secs: u64) -> Result<Self, OidcError> {
        Ok(Self::with_policy(timeout_secs, UrlPolicy::strict()))
    }

    pub fn with_policy(timeout_secs: u64, policy: UrlPolicy) -> Self {
        Self {
            timeout: Duration::from_secs(timeout_secs.max(1)),
            policy,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }

    /// Borne de taille de réponse (tests).
    pub fn with_max_bytes(mut self, max: usize) -> Self {
        self.max_bytes = max;
        self
    }

    pub fn policy(&self) -> UrlPolicy {
        self.policy
    }

    /// Délai par défaut (10 s) — surchargeable via `ENT_OIDC_TIMEOUT_SECS`.
    /// Politique d'URL : stricte, sauf `ENT_ENV` explicitement labo/test.
    pub fn from_env() -> Self {
        let secs = std::env::var("ENT_OIDC_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|s| *s > 0)
            .unwrap_or(10);
        let mut t = Self::with_policy(secs, UrlPolicy::from_env());
        if let Some(m) = std::env::var("ENT_OIDC_MAX_BYTES")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|m| *m >= 1024)
        {
            t.max_bytes = m;
        }
        t
    }

    async fn lire(&self, mut res: reqwest::Response) -> Result<serde_json::Value, OidcError> {
        let status = res.status();
        if !status.is_success() {
            return Err(OidcError::Http {
                status: status.as_u16(),
            });
        }
        if res
            .content_length()
            .is_some_and(|l| l > self.max_bytes as u64)
        {
            return Err(OidcError::Discovery("réponse trop volumineuse".into()));
        }
        let mut buf: Vec<u8> = Vec::new();
        while let Some(chunk) = res
            .chunk()
            .await
            .map_err(|e| OidcError::Network(e.to_string()))?
        {
            if buf.len() + chunk.len() > self.max_bytes {
                return Err(OidcError::Discovery("réponse trop volumineuse".into()));
            }
            buf.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&buf)
            .map_err(|_| OidcError::Discovery(format!("réponse non JSON ({} octets)", buf.len())))
    }

    async fn envoyer(
        &self,
        url: &str,
        form: Option<&[(String, String)]>,
    ) -> Result<serde_json::Value, OidcError> {
        let mut courante = valider_url(url, self.policy)?;
        for _ in 0..=MAX_REDIRECTS {
            let addrs = resoudre_controle(&courante, self.policy, self.timeout).await?;
            let mut b = reqwest::Client::builder()
                .timeout(self.timeout)
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("ScolIA-ENT/1.0 (+oidc)");
            if let Some(url::Host::Domain(d)) = courante.host() {
                b = b.resolve_to_addrs(d, &addrs);
            }
            let client = b.build().map_err(|e| OidcError::Network(e.to_string()))?;
            let req = match form {
                None => client.get(courante.clone()),
                Some(f) => client.post(courante.clone()).form(f),
            };
            let res = req
                .header("accept", "application/json")
                .send()
                .await
                .map_err(|e| OidcError::Network(e.to_string()))?;
            if res.status().is_redirection() {
                if form.is_some() {
                    return Err(OidcError::Discovery(
                        "redirection refusée sur une requête POST".into(),
                    ));
                }
                let loc = res
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| OidcError::Discovery("redirection sans Location".into()))?
                    .to_string();
                courante = cible_redirection(&courante, &loc, self.policy)?;
                continue;
            }
            return self.lire(res).await;
        }
        Err(OidcError::Discovery("trop de redirections".into()))
    }
}

#[async_trait]
impl OidcTransport for HttpOidcTransport {
    async fn get_json(&self, url: &str) -> Result<serde_json::Value, OidcError> {
        self.envoyer(url, None).await
    }

    async fn post_form_json(
        &self,
        url: &str,
        form: &[(String, String)],
    ) -> Result<serde_json::Value, OidcError> {
        self.envoyer(url, Some(form)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Mini-serveur HTTP : renvoie `reponses[i]` (octets bruts) à la i-ᵉ connexion.
    async fn serveur(reponses: Vec<String>) -> (String, tokio::task::JoinHandle<usize>) {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        let h = tokio::spawn(async move {
            let mut n = 0;
            for r in reponses {
                let Ok((mut sock, _)) = l.accept().await else { break };
                let mut b = [0u8; 2048];
                let _ = sock.read(&mut b).await;
                let _ = sock.write_all(r.as_bytes()).await;
                let _ = sock.shutdown().await;
                n += 1;
            }
            n
        });
        (base, h)
    }

    fn ok_json(corps: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{corps}",
            corps.len()
        )
    }

    fn permissif() -> HttpOidcTransport {
        HttpOidcTransport::with_policy(5, UrlPolicy::permissive())
    }

    #[tokio::test]
    async fn strict_refuse_http_et_cibles_internes_sans_connexion() {
        let (base, h) = serveur(vec![ok_json("{}")]).await;
        let t = HttpOidcTransport::with_policy(5, UrlPolicy::strict());
        for u in [
            base.clone(),
            "https://127.0.0.1/".to_string(),
            "https://169.254.169.254/latest/meta-data/".to_string(),
            "https://[::1]/".to_string(),
        ] {
            assert!(matches!(t.get_json(&u).await, Err(OidcError::Discovery(_))), "{u}");
            assert!(matches!(
                t.post_form_json(&u, &[]).await,
                Err(OidcError::Discovery(_))
            ));
        }
        h.abort(); // le serveur n'a jamais été contacté
    }

    #[tokio::test]
    async fn permissif_lit_le_json() {
        let (base, _h) = serveur(vec![ok_json("{\"a\":1}")]).await;
        assert_eq!(permissif().get_json(&base).await.unwrap()["a"], 1);
    }

    #[tokio::test]
    async fn redirection_suivie_puis_boucle_bornee() {
        let (base, _h) = serveur(vec![]).await; // juste pour une adresse libre
        let _ = base;
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        let redir = format!(
            "HTTP/1.1 302 Found\r\nlocation: http://{addr}/suite\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
        );
        let h = tokio::spawn(async move {
            let mut n = 0;
            // 1 redirection suivie (OK), puis boucle infinie de redirections.
            loop {
                let Ok((mut sock, _)) = l.accept().await else { break };
                let mut b = [0u8; 2048];
                let _ = sock.read(&mut b).await;
                let rep = redir.clone();
                let _ = sock.write_all(rep.as_bytes()).await;
                let _ = sock.shutdown().await;
                n += 1;
            }
            n
        });
        let err = permissif().get_json(&format!("http://{addr}/")).await.unwrap_err();
        assert!(matches!(err, OidcError::Discovery(m) if m.contains("trop de redirections")));
        h.abort();
    }

    #[tokio::test]
    async fn redirection_sur_post_refusee() {
        let (base, _h) = serveur(vec![
            "HTTP/1.1 307 Temporary Redirect\r\nlocation: /x\r\ncontent-length: 0\r\nconnection: close\r\n\r\n".to_string(),
        ])
        .await;
        let err = permissif().post_form_json(&base, &[]).await.unwrap_err();
        assert!(matches!(err, OidcError::Discovery(m) if m.contains("POST")));
    }

    #[tokio::test]
    async fn reponse_trop_volumineuse_refusee() {
        let gros = format!("{{\"x\":\"{}\"}}", "a".repeat(5000));
        let (base, _h) = serveur(vec![ok_json(&gros), ok_json(&gros)]).await;
        let t = permissif().with_max_bytes(1024);
        let err = t.get_json(&base).await.unwrap_err();
        assert!(matches!(err, OidcError::Discovery(m) if m.contains("volumineuse")));
        // Même sans Content-Length (flux borné par chunks).
        let sans_cl = format!("HTTP/1.1 200 OK\r\nconnection: close\r\n\r\n{gros}");
        let (base, _h) = serveur(vec![sans_cl]).await;
        let err = t.get_json(&base).await.unwrap_err();
        assert!(matches!(err, OidcError::Discovery(m) if m.contains("volumineuse")));
    }

    #[tokio::test]
    async fn delai_depasse_est_une_erreur_reseau() {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        let _h = tokio::spawn(async move {
            let (_s, _) = l.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        let t = HttpOidcTransport::with_policy(1, UrlPolicy::permissive());
        let err = t.get_json(&format!("http://{addr}/")).await.unwrap_err();
        assert!(matches!(err, OidcError::Network(_)));
    }
}
