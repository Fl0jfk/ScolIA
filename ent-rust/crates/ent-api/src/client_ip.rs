//! IP cliente fiable pour les gardes anti-abus (verrou de connexion, limitation
//! de débit).
//!
//! Principe : la **seule** source non falsifiable est l'adresse de la socket
//! (`ConnectInfo<SocketAddr>`, requiert `into_make_service_with_connect_info`
//! au lancement). `X-Forwarded-For` n'est lu que si `ENT_TRUST_PROXY` déclare
//! un ou plusieurs proxys de confiance, et on prend alors l'entrée de **droite**
//! correspondante (celle ajoutée par notre proxy), jamais la première (celle
//! que le client écrit lui-même).
//!
//! `ENT_TRUST_PROXY` :
//! - absent, `0`, `false`, `off` : aucun proxy, IP de la socket (défaut sûr) ;
//! - `1`/`true`/`yes`/`on` : un proxy de confiance (compatibilité) ;
//! - un entier `N` : `N` proxys de confiance en chaîne devant l'application
//!   (l'IP cliente est la N-ième entrée en partant de la droite de `X-Forwarded-For`) ;
//! - une liste de CIDR / IP séparés par des virgules (`10.0.0.0/8,fd00::/8`) :
//!   on remonte la chaîne de droite à gauche tant que l'adresse appartient à un
//!   proxy de confiance ; la première adresse hors liste est le client. Si la
//!   socket elle-même n'est pas dans la liste, l'en-tête est ignoré.
//!
//! En IPv6, la clé est le préfixe `/64` (un client dispose d'au moins un /64 :
//! sans cela, il changerait d'adresse à chaque requête pour éluder les quotas).

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};

/// Politique de confiance envers les proxys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfianceProxy {
    Aucune,
    /// Nombre de proxys de confiance devant l'application.
    Sauts(usize),
    Cidrs(Vec<Cidr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    reseau: IpAddr,
    prefixe: u8,
}

impl Cidr {
    fn analyser(s: &str) -> Option<Cidr> {
        let s = s.trim();
        let (ip, pref) = match s.split_once('/') {
            Some((ip, p)) => (
                ip.trim().parse::<IpAddr>().ok()?,
                Some(p.trim().parse::<u8>().ok()?),
            ),
            None => (s.parse::<IpAddr>().ok()?, None),
        };
        let ip = canonique(ip);
        let max = if ip.is_ipv4() { 32 } else { 128 };
        let prefixe = pref.unwrap_or(max);
        if prefixe > max {
            return None;
        }
        Some(Cidr {
            reseau: ip,
            prefixe,
        })
    }

    fn contient(&self, ip: IpAddr) -> bool {
        match (self.reseau, canonique(ip)) {
            (IpAddr::V4(r), IpAddr::V4(a)) => {
                masque_eq(u32::from(r) as u128, u32::from(a) as u128, 32, self.prefixe)
            }
            (IpAddr::V6(r), IpAddr::V6(a)) => {
                masque_eq(u128::from(r), u128::from(a), 128, self.prefixe)
            }
            _ => false,
        }
    }
}

fn masque_eq(a: u128, b: u128, largeur: u32, prefixe: u8) -> bool {
    if prefixe == 0 {
        return true;
    }
    let decalage = largeur - u32::from(prefixe);
    (a >> decalage) == (b >> decalage)
}

/// IPv4 mappée en IPv6 (`::ffff:a.b.c.d`) → IPv4 : une seule clé par client.
fn canonique(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        v4 => v4,
    }
}

impl ConfianceProxy {
    /// Analyse la valeur de `ENT_TRUST_PROXY`. Toute valeur illisible donne
    /// `Aucune` (échec sûr : on ne fait pas confiance à l'en-tête).
    pub fn analyser(valeur: &str) -> ConfianceProxy {
        let v = valeur.trim();
        let bas = v.to_ascii_lowercase();
        match bas.as_str() {
            "" | "0" | "false" | "off" | "no" | "non" | "none" => return ConfianceProxy::Aucune,
            "1" | "true" | "yes" | "on" | "oui" => return ConfianceProxy::Sauts(1),
            _ => {}
        }
        if let Ok(n) = bas.parse::<usize>() {
            return if (1..=16).contains(&n) {
                ConfianceProxy::Sauts(n)
            } else {
                ConfianceProxy::Aucune
            };
        }
        let mut cidrs = Vec::new();
        for part in v.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            match Cidr::analyser(part) {
                Some(c) => cidrs.push(c),
                None => {
                    tracing::warn!("ENT_TRUST_PROXY : entrée « {part} » invalide, ignorée");
                }
            }
        }
        if cidrs.is_empty() {
            ConfianceProxy::Aucune
        } else {
            ConfianceProxy::Cidrs(cidrs)
        }
    }

    pub fn depuis_env() -> ConfianceProxy {
        match std::env::var("ENT_TRUST_PROXY") {
            Ok(v) => Self::analyser(&v),
            Err(_) => ConfianceProxy::Aucune,
        }
    }
}

/// Analyse une entrée de `X-Forwarded-For` : IP seule, `ip:port` ou `[v6]:port`.
fn analyser_entree(s: &str) -> Option<IpAddr> {
    let s = s.trim();
    if let Ok(ip) = s.parse::<IpAddr>() {
        return Some(ip);
    }
    s.parse::<SocketAddr>().ok().map(|sa| sa.ip())
}

/// Résout l'IP cliente à partir de l'adresse de la socket et des valeurs de
/// `X-Forwarded-For` (déjà concaténées, dans l'ordre). Fonction pure, testée.
///
/// Sans adresse de socket, aucune IP n'est fiable (`None`).
pub fn resoudre_ip(
    pair: Option<IpAddr>,
    xff: Option<&str>,
    confiance: &ConfianceProxy,
) -> Option<IpAddr> {
    let pair = canonique(pair?);
    let entrees: Vec<&str> = match xff {
        Some(v) if v.len() <= 2048 => v
            .split(',')
            .map(str::trim)
            .filter(|e| !e.is_empty())
            .collect(),
        _ => Vec::new(),
    };
    let client = match confiance {
        ConfianceProxy::Aucune => pair,
        ConfianceProxy::Sauts(n) => {
            // chaîne = entrées ++ [socket] ; le client est `n` rangs avant la socket.
            // Soit entrées[len - n] (le proxy de droite a ajouté l'IP qu'il voyait).
            if entrees.len() < *n {
                pair
            } else {
                analyser_entree(entrees[entrees.len() - n])
                    .map(canonique)
                    .unwrap_or(pair)
            }
        }
        ConfianceProxy::Cidrs(cidrs) => {
            let approuve = |ip: IpAddr| cidrs.iter().any(|c| c.contient(ip));
            if !approuve(pair) {
                pair
            } else {
                let mut resultat = None;
                let mut toutes_approuvees = true;
                for e in entrees.iter().rev() {
                    match analyser_entree(e) {
                        // Entrée illisible : l'en-tête est altéré, on ne s'y fie plus.
                        None => {
                            resultat = Some(pair);
                            toutes_approuvees = false;
                            break;
                        }
                        Some(ip) if approuve(ip) => continue,
                        Some(ip) => {
                            resultat = Some(canonique(ip));
                            toutes_approuvees = false;
                            break;
                        }
                    }
                }
                match resultat {
                    Some(ip) => ip,
                    None if toutes_approuvees => entrees
                        .first()
                        .and_then(|e| analyser_entree(e))
                        .map(canonique)
                        .unwrap_or(pair),
                    None => pair,
                }
            }
        }
    };
    Some(client)
}

/// Clé textuelle d'une IP pour les gardes : IPv4 telle quelle, IPv6 réduite à
/// son préfixe `/64` (`2001:db8:0:1::/64`). Longueur ≤ 45.
pub fn cle_ip(ip: IpAddr) -> String {
    match canonique(ip) {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            let s = v6.segments();
            let base = Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0);
            format!("{base}/64")
        }
    }
}

/// IP cliente fiable de la requête (`None` seulement sans `ConnectInfo`, c.-à-d.
/// hors serveur réel : tests unitaires par `oneshot`).
#[derive(Debug, Clone, Default)]
pub struct ClientIp(pub Option<String>);

impl ClientIp {
    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// Dérivée des parties d'une requête (utilisable hors extracteur).
    pub fn depuis_parts(parts: &Parts) -> ClientIp {
        let pair = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip());
        let mut xff = String::new();
        for v in parts.headers.get_all("x-forwarded-for") {
            match v.to_str() {
                Ok(s) => {
                    if !xff.is_empty() {
                        xff.push(',');
                    }
                    xff.push_str(s);
                }
                // En-tête non ASCII : altéré, ignoré en bloc.
                Err(_) => {
                    xff.clear();
                    xff.push_str(&"x".repeat(4096));
                    break;
                }
            }
        }
        let xff = if xff.is_empty() {
            None
        } else {
            Some(xff.as_str())
        };
        ClientIp(resoudre_ip(pair, xff, &ConfianceProxy::depuis_env()).map(cle_ip))
    }
}

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(ClientIp::depuis_parts(parts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn analyse_de_la_variable() {
        assert_eq!(ConfianceProxy::analyser(""), ConfianceProxy::Aucune);
        assert_eq!(ConfianceProxy::analyser("0"), ConfianceProxy::Aucune);
        assert_eq!(ConfianceProxy::analyser("off"), ConfianceProxy::Aucune);
        assert_eq!(ConfianceProxy::analyser("1"), ConfianceProxy::Sauts(1));
        assert_eq!(ConfianceProxy::analyser("true"), ConfianceProxy::Sauts(1));
        assert_eq!(ConfianceProxy::analyser("2"), ConfianceProxy::Sauts(2));
        assert_eq!(ConfianceProxy::analyser("9999"), ConfianceProxy::Aucune);
        assert_eq!(
            ConfianceProxy::analyser("n'importe quoi"),
            ConfianceProxy::Aucune
        );
        match ConfianceProxy::analyser("10.0.0.0/8, fd00::/8, 192.168.1.5") {
            ConfianceProxy::Cidrs(c) => assert_eq!(c.len(), 3),
            autre => panic!("{autre:?}"),
        }
        // Préfixe hors bornes ignoré ; liste vide → aucune confiance.
        assert_eq!(
            ConfianceProxy::analyser("10.0.0.0/40"),
            ConfianceProxy::Aucune
        );
    }

    #[test]
    fn sans_confiance_xff_est_ignore() {
        let r = resoudre_ip(
            Some(ip("203.0.113.9")),
            Some("1.2.3.4"),
            &ConfianceProxy::Aucune,
        );
        assert_eq!(r, Some(ip("203.0.113.9")));
        // Pas de socket : pas d'IP fiable.
        assert_eq!(
            resoudre_ip(None, Some("1.2.3.4"), &ConfianceProxy::Sauts(1)),
            None
        );
    }

    #[test]
    fn un_proxy_prend_l_entree_de_droite_pas_la_premiere() {
        // Le client écrit « 6.6.6.6 » ; notre proxy ajoute l'IP réellement vue.
        let r = resoudre_ip(
            Some(ip("10.0.0.1")),
            Some("6.6.6.6, 198.51.100.7"),
            &ConfianceProxy::Sauts(1),
        );
        assert_eq!(
            r,
            Some(ip("198.51.100.7")),
            "usurpation en tête de chaîne sans effet"
        );
    }

    #[test]
    fn plusieurs_proxys_en_chaine() {
        // client, CDN vu par le LB ; socket = LB.
        let r = resoudre_ip(
            Some(ip("10.0.0.2")),
            Some("6.6.6.6, 198.51.100.7, 172.16.0.9"),
            &ConfianceProxy::Sauts(2),
        );
        assert_eq!(r, Some(ip("198.51.100.7")));
        // En-têtes multiples concaténés se comportent comme un seul.
        let r = resoudre_ip(
            Some(ip("10.0.0.2")),
            Some("198.51.100.7,172.16.0.9"),
            &ConfianceProxy::Sauts(2),
        );
        assert_eq!(r, Some(ip("198.51.100.7")));
    }

    #[test]
    fn chaine_trop_courte_ou_invalide_retombe_sur_la_socket() {
        let s = Some(ip("10.0.0.1"));
        // Chaîne vide / absente.
        assert_eq!(resoudre_ip(s, None, &ConfianceProxy::Sauts(1)), s);
        assert_eq!(resoudre_ip(s, Some(""), &ConfianceProxy::Sauts(1)), s);
        assert_eq!(resoudre_ip(s, Some(" , ,"), &ConfianceProxy::Sauts(1)), s);
        // Moins d'entrées que de proxys déclarés.
        assert_eq!(
            resoudre_ip(s, Some("198.51.100.7"), &ConfianceProxy::Sauts(2)),
            s
        );
        // Valeur invalide à la position du client.
        assert_eq!(
            resoudre_ip(s, Some("pas-une-ip"), &ConfianceProxy::Sauts(1)),
            s
        );
        assert_eq!(
            resoudre_ip(s, Some("1.2.3.4, <script>"), &ConfianceProxy::Sauts(1)),
            s
        );
        // En-tête démesuré.
        let enorme = "1.1.1.1,".repeat(500);
        assert_eq!(resoudre_ip(s, Some(&enorme), &ConfianceProxy::Sauts(1)), s);
    }

    #[test]
    fn ipv6_et_ipv4_mappee() {
        let r = resoudre_ip(
            Some(ip("::ffff:10.0.0.1")),
            Some("2001:db8:aaaa:bbbb:1:2:3:4"),
            &ConfianceProxy::Sauts(1),
        );
        assert_eq!(r, Some(ip("2001:db8:aaaa:bbbb:1:2:3:4")));
        // `ip:port` et `[v6]:port` (certains proxys ajoutent le port).
        assert_eq!(
            resoudre_ip(
                Some(ip("10.0.0.1")),
                Some("198.51.100.7:51234"),
                &ConfianceProxy::Sauts(1)
            ),
            Some(ip("198.51.100.7"))
        );
        assert_eq!(
            resoudre_ip(
                Some(ip("10.0.0.1")),
                Some("[2001:db8::1]:443"),
                &ConfianceProxy::Sauts(1)
            ),
            Some(ip("2001:db8::1"))
        );
        // Clé : /64 pour IPv6, une seule clé pour IPv4 mappée.
        assert_eq!(
            cle_ip(ip("2001:db8:aaaa:bbbb:1:2:3:4")),
            "2001:db8:aaaa:bbbb::/64"
        );
        assert_eq!(
            cle_ip(ip("2001:db8:aaaa:bbbb:ffff::9")),
            "2001:db8:aaaa:bbbb::/64"
        );
        assert_eq!(cle_ip(ip("::ffff:203.0.113.9")), "203.0.113.9");
        assert!(cle_ip(ip("ffff:ffff:ffff:ffff::")).len() <= 45);
    }

    #[test]
    fn cidr_remonte_la_chaine_de_droite_a_gauche() {
        let conf = ConfianceProxy::analyser("10.0.0.0/8,fd00::/8");
        // Le client forge « 10.9.9.9 » en tête : ignoré car la première adresse
        // non approuvée en partant de la droite est 198.51.100.7.
        let r = resoudre_ip(
            Some(ip("10.0.0.1")),
            Some("10.9.9.9, 198.51.100.7, 10.0.0.5"),
            &conf,
        );
        assert_eq!(r, Some(ip("198.51.100.7")));
        // Socket hors liste : l'en-tête est ignoré en bloc.
        let r = resoudre_ip(Some(ip("203.0.113.50")), Some("1.2.3.4"), &conf);
        assert_eq!(r, Some(ip("203.0.113.50")));
        // Entrée illisible dans la chaîne : repli sur la socket.
        let r = resoudre_ip(Some(ip("10.0.0.1")), Some("1.2.3.4, zzz"), &conf);
        assert_eq!(r, Some(ip("10.0.0.1")));
        // Tout approuvé : l'entrée la plus à gauche.
        let r = resoudre_ip(Some(ip("10.0.0.1")), Some("10.1.1.1, 10.2.2.2"), &conf);
        assert_eq!(r, Some(ip("10.1.1.1")));
        // IPv6 de confiance.
        let r = resoudre_ip(Some(ip("fd00::1")), Some("2001:db8::7"), &conf);
        assert_eq!(r, Some(ip("2001:db8::7")));
    }
}
