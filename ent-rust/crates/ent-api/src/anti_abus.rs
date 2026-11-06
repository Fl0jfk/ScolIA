//! Limitation de débit des routes d'authentification **publiques**.
//!
//! Une couche unique, appliquée à l'application entière, ne s'intéresse qu'aux
//! gabarits listés dans [`regle`] (via `MatchedPath`). Pour chacune :
//! - un seau par **IP cliente fiable** (voir `client_ip`), ignoré si l'IP est
//!   inconnue (hors serveur réel) ;
//! - éventuellement un seau par **identifiant normalisé** lu dans le corps JSON
//!   (e-mail en minuscules, ou jeton) — jamais sur la connexion ni le démarrage
//!   passkey : un tiers pourrait sinon épuiser le seau de sa victime et lui
//!   interdire de se connecter (le verrou de connexion par (compte, IP) et le
//!   ralentissement par compte couvrent ces cas).
//!
//! Dépassement : `429 Too Many Requests` + `Retry-After`, message neutre
//! identique pour toutes les routes (aucune information sur l'existence du
//! compte). Les quotas sont des seaux à jetons en base (partagés entre
//! instances), configurables par variables d'environnement :
//! `ENT_RATE_<ROUTE>_IP` et `ENT_RATE_<ROUTE>_ID`, au format `max/secondes`
//! (`off` pour désactiver ce seau) ; `ENT_RATE_LIMIT=off` coupe tout.

use crate::client_ip::ClientIp;
use crate::AppState;
use axum::body::{to_bytes, Body};
use axum::extract::{MatchedPath, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use ent_db::{Decision, Quota};

/// Corps JSON lu au plus pour extraire l'identifiant (les corps d'authentification
/// sont minuscules ; au-delà, la requête est refusée avant le handler).
const CORPS_MAX: usize = 64 * 1024;

pub const MESSAGE_LIMITE: &str = "Trop de requêtes — réessayez plus tard.";

/// Règle d'une route : nom (variables d'environnement), quotas par défaut,
/// champ JSON servant d'identifiant.
pub struct Regle {
    pub nom: &'static str,
    pub ip: Option<Quota>,
    pub id: Option<Quota>,
    pub champ_id: Option<&'static str>,
}

/// Règle applicable à un gabarit de route, `None` si non limitée.
pub fn regle(gabarit: &str) -> Option<Regle> {
    let r = |nom, ip, id, champ_id| Regle {
        nom,
        ip,
        id,
        champ_id,
    };
    Some(match gabarit {
        // Connexion : le plafond par IP protège le CPU (Argon2) ; pas de seau par
        // compte (déni de service d'une victime), voir verrou (compte, IP).
        "/auth/login" => r("LOGIN", Some(Quota::new(120, 60)), None, None),
        "/auth/forgot-password" => r(
            "FORGOT_PASSWORD",
            Some(Quota::new(10, 900)),
            Some(Quota::new(3, 3600)),
            Some("email"),
        ),
        "/auth/reset-password" => r(
            "RESET_PASSWORD",
            Some(Quota::new(20, 900)),
            Some(Quota::new(10, 900)),
            Some("token"),
        ),
        // Récupération de passkey (D-018) : demande = envoi d'e-mail (seau par IP
        // et par e-mail, strict) ; les jetons portent 256 bits, seau par IP/jeton.
        "/auth/passkey/recuperation" => r(
            "PASSKEY_RECUP",
            Some(Quota::new(5, 900)),
            Some(Quota::new(2, 3600)),
            Some("email"),
        ),
        "/auth/passkey/recuperation/confirmer" => r(
            "PASSKEY_RECUP_CONFIRMER",
            Some(Quota::new(20, 900)),
            Some(Quota::new(10, 900)),
            Some("token"),
        ),
        "/auth/passkey/recuperation/annuler" => r(
            "PASSKEY_RECUP_ANNULER",
            Some(Quota::new(20, 900)),
            Some(Quota::new(10, 900)),
            Some("token"),
        ),
        "/auth/passkey/recuperation/finaliser" => r(
            "PASSKEY_RECUP_FINALISER",
            Some(Quota::new(20, 900)),
            Some(Quota::new(10, 900)),
            Some("token"),
        ),
        "/auth/passkey/login/start" => {
            r("PASSKEY_LOGIN_START", Some(Quota::new(30, 300)), None, None)
        }
        "/auth/passkey/login/finish" => r(
            "PASSKEY_LOGIN_FINISH",
            Some(Quota::new(60, 300)),
            None,
            None,
        ),
        // Préinscription (S2-3, D-014) : la demande envoie un code par e-mail
        // (seau par IP et par e-mail, strict) ; la vérification est bornée par
        // demande (5 essais en base) et par IP ; invitation : jeton de 256 bits.
        "/preinscription/demarrer" => r(
            "PREINSCRIPTION_DEMARRER",
            Some(Quota::new(10, 900)),
            Some(Quota::new(3, 3600)),
            Some("email"),
        ),
        "/preinscription/verifier" => r(
            "PREINSCRIPTION_VERIFIER",
            Some(Quota::new(30, 900)),
            Some(Quota::new(10, 900)),
            Some("demande_id"),
        ),
        // Espace famille à jeton (I0, §2.2 pt 12) : mêmes plafonds par IP pour toutes
        // les routes à jeton (un seul seau) ; la délivrance d'un jeton par session est
        // plus stricte. Le jeton porte 256 bits : pas de seau par identifiant.
        "/preinscription/session" => r(
            "PREINSCRIPTION_SESSION",
            Some(Quota::new(20, 900)),
            None,
            None,
        ),
        "/preinscription/moi"
        | "/preinscription/dossiers"
        | "/preinscription/dossiers/{id}"
        | "/preinscription/dossiers/{id}/pieces/{code}"
        | "/preinscription/dossiers/{id}/banque"
        | "/preinscription/dossiers/{id}/mandat"
        | "/preinscription/dossiers/{id}/mandat/signer" => r(
            "PREINSCRIPTION_FAMILLE",
            Some(Quota::new(120, 900)),
            None,
            None,
        ),
        "/inscriptions/invitations/refuser" => r(
            "INVITATION_REFUSER",
            Some(Quota::new(20, 900)),
            Some(Quota::new(10, 900)),
            Some("jeton"),
        ),
        "/inscriptions/invitations/accepter" => r(
            "INVITATION_ACCEPTER",
            Some(Quota::new(20, 900)),
            Some(Quota::new(10, 900)),
            Some("jeton"),
        ),
        "/auth/oidc/{slug}/start" => r("OIDC_START", Some(Quota::new(30, 300)), None, None),
        _ => return None,
    })
}

fn desactive() -> bool {
    matches!(
        std::env::var("ENT_RATE_LIMIT")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "off" | "0" | "false" | "non"
    )
}

fn reponse_limite(retry_after_secs: u64) -> Response {
    let mut res = Response::new(Body::from(MESSAGE_LIMITE));
    *res.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    let h = res.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    if let Ok(v) = HeaderValue::from_str(&retry_after_secs.to_string()) {
        h.insert(header::RETRY_AFTER, v);
    }
    res
}

/// Couche axum : contrôle les quotas avant le handler.
pub async fn limiter_debit(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let gabarit = match req.extensions().get::<MatchedPath>() {
        Some(m) => m.as_str().to_string(),
        None => return next.run(req).await,
    };
    let Some(regle) = regle(&gabarit) else {
        return next.run(req).await;
    };
    if desactive() {
        return next.run(req).await;
    }
    let quota_ip = Quota::depuis_env(&format!("ENT_RATE_{}_IP", regle.nom), regle.ip);
    let quota_id = Quota::depuis_env(&format!("ENT_RATE_{}_ID", regle.nom), regle.id);
    let route = regle.nom.to_ascii_lowercase();
    let limiteur = state.ent.limiteur_debit();

    let (parts, body) = req.into_parts();
    let ip = ClientIp::depuis_parts(&parts);

    if let (Some(q), Some(ip)) = (quota_ip, ip.as_deref()) {
        if let Decision::Limite { retry_after_secs } = limiteur.verifier(&route, "ip", ip, q).await
        {
            return reponse_limite(retry_after_secs);
        }
    }

    // Seau par identifiant : lecture bornée du corps JSON, puis corps rétabli.
    let mut body = body;
    if let (Some(q), Some(champ)) = (quota_id, regle.champ_id) {
        let octets = match to_bytes(body, CORPS_MAX).await {
            Ok(b) => b,
            Err(_) => {
                let mut res = Response::new(Body::from("Requête trop volumineuse."));
                *res.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;
                return res;
            }
        };
        let identifiant = serde_json::from_slice::<serde_json::Value>(&octets)
            .ok()
            .and_then(|v| v.get(champ).and_then(|c| c.as_str().map(str::to_string)));
        body = Body::from(octets);
        if let Some(id) = identifiant.filter(|s| !s.trim().is_empty()) {
            if let Decision::Limite { retry_after_secs } =
                limiteur.verifier(&route, "id", &id, q).await
            {
                return reponse_limite(retry_after_secs);
            }
        }
    }

    next.run(Request::from_parts(parts, body)).await
}
