//! Client **RFC 3161** (horodatage qualifié) écrit à la main, sans dépendance nouvelle (D-062) : requête
//! `TimeStampReq` en DER (SHA-256), envoi HTTP (`application/timestamp-query`, authentification Basic facultative),
//! lecture minimale du `TimeStampResp` (statut, jeton rendu tel quel). Socle 7, lot D6.
//!
//! Le jeton est conservé **opaque** : sa validation cryptographique relève d'un outil tiers (OpenSSL, DSS). Ici on
//! contrôle seulement que le statut est « accordé » et que le jeton porte bien l'empreinte et le nonce envoyés.

use std::time::Duration;

use base64::Engine;

/// OID SHA-256 (2.16.840.1.101.3.4.2.1) et paramètre NULL, sous forme d'`AlgorithmIdentifier`.
const ALGO_SHA256: [u8; 15] = [
    0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05, 0x00,
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurRfc3161 {
    #[error("service d'horodatage indisponible : {0}")]
    Indisponible(String),
    #[error("horodatage refusé par le service (statut {0})")]
    Refuse(i64),
    #[error("réponse d'horodatage illisible : {0}")]
    Reponse(&'static str),
}

/// Requête `TimeStampReq` (version 1, SHA-256, `certReq` vrai). Le nonce a son bit de poids fort à zéro (entier positif).
pub fn requete_der(empreinte: &[u8; 32], nonce: [u8; 8]) -> Vec<u8> {
    let mut nonce = nonce;
    nonce[0] &= 0x7f;
    let mut imprint = Vec::with_capacity(49);
    imprint.extend_from_slice(&ALGO_SHA256);
    imprint.extend_from_slice(&[0x04, 0x20]);
    imprint.extend_from_slice(empreinte);
    let mut corps = vec![0x02, 0x01, 0x01, 0x30, imprint.len() as u8];
    corps.extend_from_slice(&imprint);
    corps.extend_from_slice(&[0x02, 0x08]);
    corps.extend_from_slice(&nonce);
    corps.extend_from_slice(&[0x01, 0x01, 0xff]);
    let mut out = vec![0x30, corps.len() as u8];
    out.extend_from_slice(&corps);
    out
}

/// Lit un en-tête DER : (étiquette, longueur du contenu, taille de l'en-tête).
fn entete(b: &[u8]) -> Option<(u8, usize, usize)> {
    let tag = *b.first()?;
    let l = *b.get(1)?;
    if l < 0x80 {
        return Some((tag, l as usize, 2));
    }
    let n = (l & 0x7f) as usize;
    if n == 0 || n > 4 || b.len() < 2 + n {
        return None;
    }
    let mut len = 0usize;
    for &x in &b[2..2 + n] {
        len = (len << 8) | x as usize;
    }
    Some((tag, len, 2 + n))
}

fn contient(h: &[u8], a: &[u8]) -> bool {
    !a.is_empty() && h.windows(a.len()).any(|w| w == a)
}

/// Lit un `TimeStampResp` : rend le jeton (`TimeStampToken`, DER) si le statut est 0 ou 1 et s'il porte l'empreinte
/// et le nonce de la requête.
pub fn lire_reponse(
    reponse: &[u8],
    empreinte: &[u8; 32],
    nonce: [u8; 8],
) -> Result<Vec<u8>, ErreurRfc3161> {
    let mut nonce = nonce;
    nonce[0] &= 0x7f;
    let mal = ErreurRfc3161::Reponse;
    let (t, l, h) = entete(reponse).ok_or(mal("en-tête"))?;
    if t != 0x30 || h + l > reponse.len() {
        return Err(mal("enveloppe"));
    }
    let corps = &reponse[h..h + l];
    let (t, li, hi) = entete(corps).ok_or(mal("statut"))?;
    if t != 0x30 || hi + li > corps.len() {
        return Err(mal("PKIStatusInfo"));
    }
    let info = &corps[hi..hi + li];
    let (t, ls, hs) = entete(info).ok_or(mal("entier de statut"))?;
    if t != 0x02 || ls != 1 || hs + ls > info.len() {
        return Err(mal("entier de statut"));
    }
    let statut = i64::from(info[hs]);
    if statut > 1 {
        return Err(ErreurRfc3161::Refuse(statut));
    }
    let jeton = &corps[hi + li..];
    if jeton.is_empty() {
        return Err(mal("jeton absent"));
    }
    if !contient(jeton, empreinte) {
        return Err(mal("le jeton ne porte pas l'empreinte envoyée"));
    }
    if !contient(jeton, &nonce) {
        return Err(mal("le jeton ne porte pas le nonce envoyé"));
    }
    Ok(jeton.to_vec())
}

/// **M6** : l'adresse du service ne doit jamais viser le réseau interne (SSRF). HTTPS seul, aucune identité dans
/// l'adresse, et **toutes** les adresses IP obtenues par la résolution DNS doivent être publiques (ni bouclage, ni
/// privée, ni lien local, ni CGNAT, ni multicast, ni non spécifiée). La vérification se refait à chaque envoi.
pub fn adresse_publique(url: &str) -> Result<(), String> {
    use std::net::{IpAddr, ToSocketAddrs};
    let reste = url.strip_prefix("https://").ok_or("HTTPS exigé")?;
    let autorite = reste.split(['/', '?', '#']).next().unwrap_or("");
    if autorite.is_empty() || autorite.contains('@') || autorite.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("adresse invalide".into());
    }
    let (hote, port) = if let Some(v6) = autorite.strip_prefix('[') {
        let (h, apres) = v6.split_once(']').ok_or("adresse invalide")?;
        (h.to_string(), apres.strip_prefix(':').map(str::to_string))
    } else {
        match autorite.rsplit_once(':') {
            Some((h, p)) if !h.contains(':') => (h.to_string(), Some(p.to_string())),
            _ => (autorite.to_string(), None),
        }
    };
    let port: u16 = match port {
        Some(p) => p.parse().map_err(|_| "port invalide")?,
        None => 443,
    };
    let ips: Vec<IpAddr> = match hote.parse::<IpAddr>() {
        Ok(ip) => vec![ip],
        Err(_) => (hote.as_str(), port)
            .to_socket_addrs()
            .map_err(|_| "résolution DNS impossible")?
            .map(|a| a.ip())
            .collect(),
    };
    if ips.is_empty() {
        return Err("aucune adresse".into());
    }
    if let Some(ip) = ips.iter().find(|ip| !ip_publique(ip)) {
        return Err(format!("adresse non publique refusée ({ip})"));
    }
    Ok(())
}

fn ip_publique(ip: &std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v) => {
            let o = v.octets();
            !(v.is_loopback()
                || v.is_private()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_broadcast()
                || v.is_multicast()
                || v.is_documentation()
                || (o[0] == 100 && (64..=127).contains(&o[1]))
                || o[0] == 0
                || o[0] >= 240)
        }
        IpAddr::V6(v) => {
            if let Some(v4) = v.to_ipv4_mapped() {
                return ip_publique(&IpAddr::V4(v4));
            }
            let s = v.segments();
            !(v.is_loopback()
                || v.is_unspecified()
                || v.is_multicast()
                || (s[0] & 0xfe00) == 0xfc00
                || (s[0] & 0xffc0) == 0xfe80)
        }
    }
}

/// Client HTTP : un service RFC 3161 (adresse, identifiants facultatifs).
#[derive(Debug, Clone)]
pub struct ClientRfc3161 {
    url: String,
    basic: Option<String>,
    /// Les tests (faux serveur local en HTTP) désactivent la garde réseau ; jamais en production.
    garde_reseau: bool,
}

impl ClientRfc3161 {
    pub fn new(url: &str, identifiant: Option<&str>, secret: Option<&str>) -> Self {
        let basic = identifiant.map(|i| {
            let brut = format!("{i}:{}", secret.unwrap_or(""));
            format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(brut)
            )
        });
        Self {
            url: url.to_string(),
            basic,
            garde_reseau: true,
        }
    }

    /// Sans la garde d'adresse (HTTPS public seulement) : **réservé aux tests** avec un faux serveur local.
    #[doc(hidden)]
    pub fn sans_garde_reseau(mut self) -> Self {
        self.garde_reseau = false;
        self
    }

    /// Demande un jeton pour l'empreinte ; `nonce` vient de l'appelant (aléatoire).
    pub fn horodater(&self, empreinte: &[u8; 32], nonce: [u8; 8]) -> Result<Vec<u8>, ErreurRfc3161> {
        if self.garde_reseau {
            adresse_publique(&self.url).map_err(ErreurRfc3161::Indisponible)?;
        }
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            // Aucune redirection : une redirection pourrait mener vers le réseau interne.
            .max_redirects(0)
            .timeout_connect(Some(Duration::from_secs(5)))
            .timeout_global(Some(Duration::from_secs(20)))
            .build();
        let agent = ureq::Agent::new_with_config(config);
        let req = requete_der(empreinte, nonce);
        let mut r = agent
            .post(&self.url)
            .header("Content-Type", "application/timestamp-query");
        if let Some(b) = &self.basic {
            r = r.header("Authorization", b);
        }
        let mut rep = r
            .send(&req[..])
            .map_err(|e| ErreurRfc3161::Indisponible(format!("{e}").chars().take(120).collect()))?;
        let statut = rep.status().as_u16();
        if statut != 200 {
            return Err(ErreurRfc3161::Indisponible(format!("HTTP {statut}")));
        }
        let corps = rep
            .body_mut()
            .with_config()
            .limit(64 * 1024)
            .read_to_vec()
            .map_err(|_| ErreurRfc3161::Indisponible("lecture de la réponse".into()))?;
        lire_reponse(&corps, empreinte, nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    const H: [u8; 32] = [7; 32];
    const N: [u8; 8] = [0x81, 1, 2, 3, 4, 5, 6, 7];

    fn reponse(statut: u8, avec_jeton: bool) -> Vec<u8> {
        let info = vec![0x30, 0x03, 0x02, 0x01, statut];
        let mut corps = info;
        if avec_jeton {
            let mut j = vec![0x30, 0x30];
            j.extend_from_slice(&H);
            j.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10]);
            let mut j2 = vec![0x30, (j.len() - 2 + 8) as u8];
            j2.extend_from_slice(&j[2..]);
            j2.extend_from_slice(&[0x01, 1, 2, 3, 4, 5, 6, 7]);
            corps.extend_from_slice(&j2);
        }
        let mut out = vec![0x30, corps.len() as u8];
        out.extend_from_slice(&corps);
        out
    }

    #[test]
    fn requete_bien_formee() {
        let r = requete_der(&H, N);
        assert_eq!(r[0], 0x30);
        assert_eq!(r[1] as usize, r.len() - 2);
        assert_eq!(&r[2..5], &[0x02, 0x01, 0x01]);
        assert!(contient(&r, &H));
        // nonce positif
        assert!(contient(&r, &[0x01, 1, 2, 3, 4, 5, 6, 7]));
        assert_eq!(&r[r.len() - 3..], &[0x01, 0x01, 0xff]);
    }

    #[test]
    fn reponse_accordee_refusee_et_incoherente() {
        assert!(lire_reponse(&reponse(0, true), &H, N).is_ok());
        assert_eq!(
            lire_reponse(&reponse(2, false), &H, N),
            Err(ErreurRfc3161::Refuse(2))
        );
        assert!(lire_reponse(&reponse(0, false), &H, N).is_err());
        assert!(lire_reponse(&reponse(0, true), &[9; 32], N).is_err());
        assert!(lire_reponse(b"\x30", &H, N).is_err());
    }

    #[test]
    fn adresses_privees_refusees_publiques_admises() {
        for u in [
            "http://tsa.exemple/",
            "https://127.0.0.1/",
            "https://10.1.2.3/tsr",
            "https://192.168.0.5:8443/",
            "https://172.16.0.1/",
            "https://169.254.169.254/latest",
            "https://100.64.0.1/",
            "https://0.0.0.0/",
            "https://[::1]/",
            "https://[fe80::1]/",
            "https://[fd00::1]/",
            "https://[::ffff:127.0.0.1]/",
            "https://user@93.184.216.34/",
            "https://",
            "https://localhost/",
        ] {
            assert!(adresse_publique(u).is_err(), "{u} devrait être refusée");
        }
        for u in ["https://93.184.216.34/tsr", "https://[2606:2800:220:1:248:1893:25c8:1946]:8443/x"] {
            assert!(adresse_publique(u).is_ok(), "{u} devrait être admise");
        }
    }

    #[test]
    fn client_refuse_une_adresse_privee_sans_rien_envoyer() {
        let c = ClientRfc3161::new("https://127.0.0.1:1/tsa", None, None);
        assert!(matches!(c.horodater(&H, N), Err(ErreurRfc3161::Indisponible(m)) if m.contains("non publique")));
    }

    #[test]
    fn client_http_contre_un_faux_serveur() {
        let ecoute = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = ecoute.local_addr().unwrap().port();
        let corps = reponse(0, true);
        let fil = std::thread::spawn(move || {
            let (mut s, _) = ecoute.accept().unwrap();
            let mut tampon = [0u8; 4096];
            let n = s.read(&mut tampon).unwrap();
            let vu = String::from_utf8_lossy(&tampon[..n]).to_lowercase();
            let ok = vu.contains("application/timestamp-query") && vu.contains("authorization: basic");
            let tete = format!(
                "HTTP/1.1 {} X\r\nContent-Type: application/timestamp-reply\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                if ok { 200 } else { 400 },
                corps.len()
            );
            s.write_all(tete.as_bytes()).unwrap();
            s.write_all(&corps).unwrap();
        });
        let c = ClientRfc3161::new(&format!("http://127.0.0.1:{port}/tsa"), Some("u"), Some("p")).sans_garde_reseau();
        let jeton = c.horodater(&H, N).expect("jeton");
        assert!(contient(&jeton, &H));
        fil.join().unwrap();
    }
}
