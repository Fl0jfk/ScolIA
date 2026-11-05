//! Garde SSRF du SSO : les URL de découverte / JWKS / token sont fournies par un
//! administrateur d'établissement ; le serveur ne doit pas devenir un relais vers
//! le réseau interne ni vers les métadonnées cloud.
//!
//! Règles (mode « strict », obligatoire hors labo/test) :
//! - schéma `https` uniquement, sans identifiants (`user:pass@`) ;
//! - toute adresse résolue (DNS **ou** littérale) loopback / privée / link-local
//!   (dont 169.254.169.254) / CGNAT / multicast / non spécifiée / documentation
//!   est refusée — si **une seule** des adresses résolues est interdite, l'hôte est refusé ;
//! - l'adresse validée est celle utilisée pour la connexion (pas de nouvelle
//!   résolution : contre le DNS rebinding) ;
//! - redirections suivies à la main (max. 3), chaque cible revalidée, pas de
//!   rétrogradation https → http.
//!
//! En labo/test (`UrlPolicy::permissive()`), http et les adresses privées sont permis.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use url::Url;

use crate::OidcError;

/// Valeurs de `ENT_ENV` désignant un environnement non productif (miroir de
/// `ent_auth::ENVS_NON_PROD`, dont la parité est testée dans `ent-api`).
pub const ENVS_NON_PROD: [&str; 5] = ["lab", "labo", "dev", "local", "test"];

/// Politique appliquée aux URL sortantes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UrlPolicy {
    pub require_https: bool,
    pub forbid_private: bool,
}

impl UrlPolicy {
    /// Production : https obligatoire, cibles internes refusées.
    pub const fn strict() -> Self {
        Self {
            require_https: true,
            forbid_private: true,
        }
    }
    /// Labo / test uniquement.
    pub const fn permissive() -> Self {
        Self {
            require_https: false,
            forbid_private: false,
        }
    }
    /// Stricte sauf si `ENT_ENV` désigne explicitement un environnement non productif.
    pub fn from_env() -> Self {
        let non_prod = std::env::var("ENT_ENV")
            .ok()
            .map(|e| e.trim().to_ascii_lowercase())
            .is_some_and(|v| ENVS_NON_PROD.contains(&v.as_str()));
        if non_prod {
            Self::permissive()
        } else {
            Self::strict()
        }
    }
}

fn refuse(msg: impl Into<String>) -> OidcError {
    OidcError::Discovery(msg.into())
}

fn v4_interdit(a: Ipv4Addr) -> bool {
    let o = a.octets();
    a.is_loopback()
        || a.is_private()
        || a.is_link_local()
        || a.is_unspecified()
        || a.is_broadcast()
        || a.is_multicast()
        || a.is_documentation()
        || o[0] == 0 // 0.0.0.0/8
        || (o[0] == 100 && (o[1] & 0xC0) == 64) // 100.64.0.0/10 (CGNAT, métadonnées Alibaba)
        || (o[0] == 192 && o[1] == 0 && o[2] == 0) // 192.0.0.0/24
        || (o[0] == 198 && (o[1] & 0xFE) == 18) // 198.18.0.0/15
        || o[0] >= 240 // réservé / classe E
}

/// `true` si l'adresse ne doit jamais être jointe par le serveur (mode strict).
pub fn ip_interdite(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => v4_interdit(a),
        IpAddr::V6(a) => v6_interdit(a),
    }
}

fn v6_interdit(a: Ipv6Addr) -> bool {
    if a.is_loopback() || a.is_unspecified() || a.is_multicast() {
        return true;
    }
    let s = a.segments();
    // IPv4 mappée (::ffff:a.b.c.d) ou compatible (::a.b.c.d) → on juge l'IPv4 embarquée.
    if let Some(v4) = a.to_ipv4_mapped() {
        return v4_interdit(v4);
    }
    if s[..6] == [0, 0, 0, 0, 0, 0] {
        return true; // ::/96 (inclut ::1 et IPv4-compatible)
    }
    let embarque =
        |hi: u16, lo: u16| Ipv4Addr::new((hi >> 8) as u8, hi as u8, (lo >> 8) as u8, lo as u8);
    // NAT64 64:ff9b::/96
    if s[0] == 0x64 && s[1] == 0xff9b && s[2..6] == [0, 0, 0, 0] {
        return v4_interdit(embarque(s[6], s[7]));
    }
    // 6to4 2002::/16
    if s[0] == 0x2002 {
        return v4_interdit(embarque(s[1], s[2]));
    }
    (s[0] & 0xFE00) == 0xFC00 // fc00::/7 unique-local
        || (s[0] & 0xFFC0) == 0xFE80 // fe80::/10 link-local
        || (s[0] & 0xFFC0) == 0xFEC0 // fec0::/10 site-local (obsolète)
        || (s[0] == 0x2001 && s[1] == 0x0db8) // documentation
        || (s[0] == 0x0100 && s[1..4] == [0, 0, 0]) // 100::/64 discard
}

/// Valide la forme d'une URL sortante (schéma, identifiants, hôte). Ne résout pas le DNS.
pub fn valider_url(url: &str, policy: UrlPolicy) -> Result<Url, OidcError> {
    let u = Url::parse(url).map_err(|_| refuse("URL illisible"))?;
    match u.scheme() {
        "https" => {}
        "http" if !policy.require_https => {}
        _ => return Err(refuse("URL refusée : https obligatoire")),
    }
    if !u.username().is_empty() || u.password().is_some() {
        return Err(refuse("URL refusée : identifiants intégrés"));
    }
    if u.host().is_none() {
        return Err(refuse("URL refusée : hôte manquant"));
    }
    if policy.forbid_private {
        if let Some(url::Host::Ipv4(a)) = u.host() {
            if v4_interdit(a) {
                return Err(refuse("URL refusée : adresse interne"));
            }
        }
        if let Some(url::Host::Ipv6(a)) = u.host() {
            if v6_interdit(a) {
                return Err(refuse("URL refusée : adresse interne"));
            }
        }
        if let Some(url::Host::Domain(d)) = u.host() {
            let d = d.trim_end_matches('.').to_ascii_lowercase();
            if d == "localhost"
                || d.ends_with(".localhost")
                || d.ends_with(".internal")
                || d.ends_with(".local")
            {
                return Err(refuse("URL refusée : nom interne"));
            }
        }
    }
    Ok(u)
}

/// Valide l'URL puis résout l'hôte et vérifie **toutes** les adresses. Renvoie les
/// adresses à utiliser pour la connexion (épinglées).
pub async fn resoudre_controle(
    url: &Url,
    policy: UrlPolicy,
    delai: Duration,
) -> Result<Vec<SocketAddr>, OidcError> {
    let host = url.host().ok_or_else(|| refuse("hôte manquant"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| refuse("port inconnu"))?;
    let addrs: Vec<SocketAddr> = match host {
        url::Host::Ipv4(a) => vec![SocketAddr::new(IpAddr::V4(a), port)],
        url::Host::Ipv6(a) => vec![SocketAddr::new(IpAddr::V6(a), port)],
        url::Host::Domain(d) => tokio::time::timeout(delai, tokio::net::lookup_host((d, port)))
            .await
            .map_err(|_| OidcError::Network("résolution DNS : délai dépassé".into()))?
            .map_err(|_| OidcError::Network("résolution DNS impossible".into()))?
            .collect(),
    };
    if addrs.is_empty() {
        return Err(OidcError::Network("aucune adresse pour l'hôte".into()));
    }
    if policy.forbid_private && addrs.iter().any(|a| ip_interdite(a.ip())) {
        return Err(refuse(
            "URL refusée : l'hôte résout vers une adresse interne",
        ));
    }
    Ok(addrs)
}

/// Cible d'une redirection : relative à l'URL courante, revalidée, sans rétrogradation.
pub fn cible_redirection(
    courante: &Url,
    location: &str,
    policy: UrlPolicy,
) -> Result<Url, OidcError> {
    let cible = courante
        .join(location)
        .map_err(|_| refuse("redirection illisible"))?;
    if courante.scheme() == "https" && cible.scheme() != "https" {
        return Err(refuse("redirection refusée : rétrogradation https → http"));
    }
    valider_url(cible.as_str(), policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn adresses_internes_refusees() {
        for a in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "100.100.100.200",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "192.0.2.1",
            "198.18.0.1",
            "240.0.0.1",
            "::1",
            "::",
            "fe80::1",
            "fc00::1",
            "fd12:3456::1",
            "ff02::1",
            "::ffff:127.0.0.1",
            "::ffff:169.254.169.254",
            "::ffff:10.0.0.1",
            "64:ff9b::a9fe:a9fe",
            "2002:a9fe:a9fe::1",
            "2001:db8::1",
        ] {
            assert!(ip_interdite(ip(a)), "{a} doit être interdite");
        }
    }

    #[test]
    fn adresses_publiques_acceptees() {
        for a in [
            "8.8.8.8",
            "51.158.57.80",
            "172.32.0.1",
            "100.128.0.1",
            "2606:4700:4700::1111",
            "::ffff:8.8.8.8",
        ] {
            assert!(!ip_interdite(ip(a)), "{a} doit être permise");
        }
    }

    #[test]
    fn url_https_obligatoire_en_strict() {
        let s = UrlPolicy::strict();
        assert!(valider_url("https://idp.example.org/x", s).is_ok());
        for u in [
            "http://idp.example.org/x",
            "ftp://idp.example.org/",
            "file:///etc/passwd",
            "https://user:pw@idp.example.org/",
            "https://169.254.169.254/latest/meta-data",
            "https://[::1]/",
            "https://127.0.0.1:8443/",
            "https://localhost/",
            "https://metadata.internal/",
            "https://[::ffff:169.254.169.254]/",
            "pas une url",
        ] {
            assert!(valider_url(u, s).is_err(), "{u} doit être refusée");
        }
        assert!(valider_url("http://127.0.0.1:8080/", UrlPolicy::permissive()).is_ok());
    }

    #[tokio::test]
    async fn resolution_controlee() {
        let d = Duration::from_secs(2);
        let strict = UrlPolicy::strict();
        // Littéral : pas de DNS.
        let u = Url::parse("https://8.8.8.8/").unwrap();
        assert_eq!(
            resoudre_controle(&u, strict, d).await.unwrap()[0].port(),
            443
        );
        // Hôte qui résout en loopback (via /etc/hosts) : refusé même si le nom est « public ».
        let u = Url::parse("https://localhost./").unwrap();
        assert!(valider_url(u.as_str(), strict).is_err());
        let u = Url::parse("https://localhost/").unwrap();
        let perm = UrlPolicy {
            require_https: true,
            forbid_private: false,
        };
        let addrs = resoudre_controle(&u, perm, d).await.unwrap();
        assert!(addrs.iter().all(|a| a.ip().is_loopback()));
        // Le contrôle post-DNS refuse un domaine résolvant en interne (strict, sans filtre de nom).
        assert!(resoudre_controle(&u, strict, d).await.is_err());
    }

    #[test]
    fn redirections_controlees() {
        let s = UrlPolicy::strict();
        let base = Url::parse("https://idp.example.org/a/b").unwrap();
        assert_eq!(
            cible_redirection(&base, "/c", s).unwrap().as_str(),
            "https://idp.example.org/c"
        );
        assert!(
            cible_redirection(&base, "http://idp.example.org/c", s).is_err(),
            "downgrade"
        );
        assert!(cible_redirection(&base, "https://169.254.169.254/", s).is_err());
        assert!(cible_redirection(&base, "https://10.0.0.5/", s).is_err());
        assert!(cible_redirection(&base, "https://[fd00::1]/", s).is_err());
    }
}
