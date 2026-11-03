//! **PROTOTYPE ISOLÉ** (D-016) — session web par cookie `HttpOnly`, **sans modifier**
//! `ent-auth` ni `ent-api`.
//!
//! Principe : une **passerelle** placée devant le routeur `ent_api::app` existant.
//! - `POST /web/session/connexion` (mot de passe), `/web/session/passkey/debut` et
//!   `/web/session/passkey/fin` appellent les routes de connexion existantes, puis
//!   **posent le jeton dans un cookie** `HttpOnly; Secure; SameSite=Lax` et le
//!   **retirent du corps** : le JavaScript de la page ne le voit jamais.
//! - `/web/api/…` relaie vers l'API existante en transformant le cookie en
//!   `Authorization: Bearer` (ent-api ne lit que cet en-tête, inchangé).
//! - `POST /web/session/deconnexion` révoque la session en base (`/auth/logout`) et
//!   efface le cookie.
//! - Protection CSRF sur toutes les requêtes qui modifient : origine autorisée **et**
//!   en-tête `X-ENT-CSRF: 1`. Elle s'applique aussi à la connexion (« login CSRF »).
//! - Durées web (D-011) : inactivité et durée maximale, contrôlées par la passerelle.
//!   **Limite du prototype** : ce suivi est en mémoire (perdu au redémarrage, donc
//!   déconnexion de tous). Une version réelle le stockerait en base (`last_seen_at`).
//!
//! Le cookie n'est accepté **que** sous `/web/…` : les routes existantes (`/auth`,
//! `/eleves`, l'ancienne interface `/app`, les applis mobiles en Bearer) l'ignorent.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::{to_bytes, Body};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::Router;
use sha2::{Digest, Sha256};
use tower::ServiceExt;

/// En-tête anti-CSRF exigé sur les requêtes qui modifient.
pub const EN_TETE_CSRF: &str = "x-ent-csrf";
const LIMITE_CORPS: usize = 2 * 1024 * 1024;

/// Routes d'`ent-api` qui **émettent un jeton de session** : interdites au relais
/// `/web/api`, sinon le jeton arriverait dans le JavaScript de la page.
const ROUTES_EMETTRICES: &[&str] = &[
    "/auth/login",
    "/auth/register",
    "/auth/passkey/login/",
    "/auth/oidc/",
];

#[derive(Clone, Debug)]
pub struct Config {
    /// Origines publiques autorisées (D-012 : une par établissement), ex. `https://lpnb.scolia.fr`.
    pub origines: Vec<String>,
    /// `Secure` + préfixe `__Host-`. `false` uniquement pour un essai sans HTTPS ni localhost.
    pub secure: bool,
    /// Durée maximale d'une session web (D-011 : 12 h).
    pub duree_max: Duration,
    /// Inactivité maximale (D-011 : 2 h).
    pub inactivite: Duration,
}

impl Config {
    pub fn nom_cookie(&self) -> &'static str {
        if self.secure {
            "__Host-ent_session"
        } else {
            "ent_session"
        }
    }
}

struct Suivi {
    cree: Instant,
    vu: Instant,
}

enum EtatSuivi {
    Valide,
    Expiree,
    Inconnue,
}

#[derive(Clone)]
struct Etat {
    api: Router,
    config: Arc<Config>,
    /// Clé : SHA-256 du jeton (le jeton lui-même n'est pas conservé).
    suivis: Arc<Mutex<HashMap<String, Suivi>>>,
}

impl Etat {
    fn verifier(&self, jeton: &str) -> EtatSuivi {
        let maintenant = Instant::now();
        let mut suivis = self.suivis.lock().unwrap();
        let cle = empreinte(jeton);
        match suivis.get_mut(&cle) {
            None => EtatSuivi::Inconnue,
            Some(s)
                if maintenant.duration_since(s.cree) > self.config.duree_max
                    || maintenant.duration_since(s.vu) > self.config.inactivite =>
            {
                suivis.remove(&cle);
                EtatSuivi::Expiree
            }
            Some(s) => {
                s.vu = maintenant;
                EtatSuivi::Valide
            }
        }
    }

    fn oublier(&self, jeton: &str) {
        self.suivis.lock().unwrap().remove(&empreinte(jeton));
    }
}

fn empreinte(jeton: &str) -> String {
    Sha256::digest(jeton.as_bytes())
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect()
}

/// Passerelle de session web, à fusionner **avant** le routeur `api` :
/// `passerelle(api.clone(), config).merge(api)`.
pub fn passerelle(api: Router, config: Config) -> Router {
    let etat = Etat {
        api,
        config: Arc::new(config),
        suivis: Arc::new(Mutex::new(HashMap::new())),
    };
    Router::new()
        .route("/web/session/connexion", post(connexion))
        .route("/web/session/passkey/debut", post(passkey_debut))
        .route("/web/session/passkey/fin", post(passkey_fin))
        .route("/web/session/deconnexion", post(deconnexion))
        .route("/web/api/{*chemin}", any(relayer))
        .with_state(etat)
}

const CSP_DEMO: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
connect-src 'self'; img-src 'self' data:; frame-ancestors 'self'; base-uri 'none'; form-action 'self'";

/// Page de démonstration (essais navigateur : multi-onglets, passkey), servie avec une
/// CSP stricte : aucun script en ligne.
pub fn page_demo() -> Router {
    fn fichier(contenu: &'static str, type_mime: &'static str) -> Response {
        let mut r = Response::new(Body::from(contenu));
        let h = r.headers_mut();
        h.insert(header::CONTENT_TYPE, HeaderValue::from_static(type_mime));
        h.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP_DEMO),
        );
        h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        h.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
        r
    }
    Router::new()
        .route(
            "/web/_session",
            get(|| async {
                fichier(
                    include_str!("../static/session.html"),
                    "text/html; charset=utf-8",
                )
            }),
        )
        .route(
            "/web/_session.js",
            get(|| async {
                fichier(
                    include_str!("../static/session.js"),
                    "text/javascript; charset=utf-8",
                )
            }),
        )
}

// ---------------------------------------------------------------------------
// Utilitaires

fn texte(code: StatusCode, message: &'static str) -> Response {
    let mut r = (code, message).into_response();
    sans_cache(r.headers_mut());
    r
}

fn sans_cache(h: &mut HeaderMap) {
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(header::VARY, HeaderValue::from_static("Cookie"));
}

fn methode_sure(m: &Method) -> bool {
    matches!(*m, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Contrôle anti-CSRF : origine autorisée (ou, sans `Origin`, `Sec-Fetch-Site:
/// same-origin`) **et** en-tête `X-ENT-CSRF: 1`.
fn garde_csrf(config: &Config, h: &HeaderMap) -> Result<(), &'static str> {
    let origine_ok = match h.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        Some(o) => config.origines.iter().any(|a| a == o),
        None => h
            .get("sec-fetch-site")
            .is_some_and(|v| v.as_bytes() == b"same-origin"),
    };
    if !origine_ok {
        return Err("origine refusée");
    }
    if h.get(EN_TETE_CSRF).map(|v| v.as_bytes()) != Some(b"1") {
        return Err("en-tête X-ENT-CSRF manquant");
    }
    Ok(())
}

pub fn lire_cookie(h: &HeaderMap, nom: &str) -> Option<String> {
    h.get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|p| p.trim().split_once('='))
        .find(|(n, v)| *n == nom && !v.is_empty())
        .map(|(_, v)| v.to_string())
}

fn cookie(config: &Config, valeur: &str, max_age: u64) -> HeaderValue {
    let secure = if config.secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{}={valeur}; HttpOnly; Path=/; SameSite=Lax; Max-Age={max_age}{secure}",
        config.nom_cookie()
    ))
    .expect("cookie valide")
}

fn effacer_cookie(config: &Config, h: &mut HeaderMap) {
    h.append(header::SET_COOKIE, cookie(config, "", 0));
}

/// Appelle l'API existante (en mémoire, sans réseau) en gardant méthode, en-têtes utiles,
/// corps et adresse du client (utilisée par l'anti-force brute d'ent-api).
async fn transmettre(etat: &Etat, req: Request, chemin: &str, jeton: Option<&str>) -> Response {
    let (parties, corps) = req.into_parts();
    let mut nouvelle = Request::builder()
        .method(parties.method.clone())
        .uri(chemin)
        .body(corps)
        .expect("requête interne");
    for (nom, valeur) in parties.headers.iter() {
        if nom != header::COOKIE && nom != header::AUTHORIZATION && nom != header::HOST {
            nouvelle.headers_mut().append(nom.clone(), valeur.clone());
        }
    }
    if let Some(j) = jeton {
        nouvelle.headers_mut().insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {j}")).expect("jeton ASCII"),
        );
    }
    if let Some(ci) = parties.extensions.get::<ConnectInfo<SocketAddr>>() {
        nouvelle.extensions_mut().insert(*ci);
    }
    let mut rep = etat
        .api
        .clone()
        .oneshot(nouvelle)
        .await
        .unwrap_or_else(|e| match e {});
    sans_cache(rep.headers_mut());
    rep
}

/// Réponse de connexion réussie d'ent-api → cookie posé, jeton retiré du corps.
async fn poser_session(etat: &Etat, rep: Response) -> Response {
    if rep.status() != StatusCode::OK {
        return rep;
    }
    let (mut parties, corps) = rep.into_parts();
    let Ok(octets) = to_bytes(corps, LIMITE_CORPS).await else {
        return texte(StatusCode::BAD_GATEWAY, "réponse illisible");
    };
    let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(&octets) else {
        return Response::from_parts(parties, Body::from(octets));
    };
    let jeton = json
        .as_object_mut()
        .filter(|o| o.get("mfa_required").and_then(|v| v.as_bool()) != Some(true))
        .and_then(|o| o.remove("token"))
        .and_then(|t| t.as_str().map(str::to_string));
    let Some(jeton) = jeton else {
        // Second facteur demandé : pas de session, le défi passkey est renvoyé tel quel.
        return Response::from_parts(parties, Body::from(octets));
    };
    // Max-Age ≤ durée maximale web et ≤ expiration réelle de la session en base.
    let reste_serveur = json
        .get("expires_at")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| (d.timestamp() - chrono::Utc::now().timestamp()).max(0) as u64)
        .unwrap_or(0);
    let max_age = reste_serveur.min(etat.config.duree_max.as_secs());
    let maintenant = Instant::now();
    etat.suivis.lock().unwrap().insert(
        empreinte(&jeton),
        Suivi {
            cree: maintenant,
            vu: maintenant,
        },
    );
    let corps = serde_json::to_vec(&json).expect("JSON");
    parties.headers.remove(header::CONTENT_LENGTH);
    parties
        .headers
        .append(header::SET_COOKIE, cookie(&etat.config, &jeton, max_age));
    Response::from_parts(parties, Body::from(corps))
}

// ---------------------------------------------------------------------------
// Routes

async fn connexion(State(etat): State<Etat>, req: Request) -> Response {
    if let Err(m) = garde_csrf(&etat.config, req.headers()) {
        return texte(StatusCode::FORBIDDEN, m);
    }
    let rep = transmettre(&etat, req, "/auth/login", None).await;
    poser_session(&etat, rep).await
}

async fn passkey_debut(State(etat): State<Etat>, req: Request) -> Response {
    if let Err(m) = garde_csrf(&etat.config, req.headers()) {
        return texte(StatusCode::FORBIDDEN, m);
    }
    transmettre(&etat, req, "/auth/passkey/login/start", None).await
}

async fn passkey_fin(State(etat): State<Etat>, req: Request) -> Response {
    if let Err(m) = garde_csrf(&etat.config, req.headers()) {
        return texte(StatusCode::FORBIDDEN, m);
    }
    let rep = transmettre(&etat, req, "/auth/passkey/login/finish", None).await;
    poser_session(&etat, rep).await
}

async fn deconnexion(State(etat): State<Etat>, req: Request) -> Response {
    if let Err(m) = garde_csrf(&etat.config, req.headers()) {
        return texte(StatusCode::FORBIDDEN, m);
    }
    if let Some(jeton) = lire_cookie(req.headers(), etat.config.nom_cookie()) {
        etat.oublier(&jeton);
        // Révocation en base (route existante) ; un jeton déjà invalide est sans importance.
        let _ = transmettre(&etat, req, "/auth/logout", Some(&jeton)).await;
    }
    let mut r = StatusCode::NO_CONTENT.into_response();
    sans_cache(r.headers_mut());
    effacer_cookie(&etat.config, r.headers_mut());
    r
}

async fn relayer(State(etat): State<Etat>, req: Request) -> Response {
    let chemin = req
        .uri()
        .path()
        .strip_prefix("/web/api")
        .unwrap_or("/")
        .to_string();
    if ROUTES_EMETTRICES.iter().any(|r| chemin.starts_with(r)) {
        return texte(
            StatusCode::FORBIDDEN,
            "route de connexion : utiliser /web/session/…",
        );
    }
    let nom = etat.config.nom_cookie();
    let Some(jeton) = lire_cookie(req.headers(), nom) else {
        return texte(StatusCode::UNAUTHORIZED, "non connecté");
    };
    match etat.verifier(&jeton) {
        EtatSuivi::Valide => {}
        etat_suivi => {
            if matches!(etat_suivi, EtatSuivi::Expiree) {
                // Durée web dépassée : la session est aussi révoquée en base.
                let logout = Request::builder()
                    .method(Method::POST)
                    .uri("/auth/logout")
                    .body(Body::empty())
                    .unwrap();
                let _ = transmettre(&etat, logout, "/auth/logout", Some(&jeton)).await;
            }
            let mut r = texte(StatusCode::UNAUTHORIZED, "session expirée");
            effacer_cookie(&etat.config, r.headers_mut());
            return r;
        }
    }
    if !methode_sure(req.method()) {
        if let Err(m) = garde_csrf(&etat.config, req.headers()) {
            return texte(StatusCode::FORBIDDEN, m);
        }
    }
    let cible = match req.uri().query() {
        Some(q) => format!("{chemin}?{q}"),
        None => chemin,
    };
    let mut rep = transmettre(&etat, req, &cible, Some(&jeton)).await;
    if rep.status() == StatusCode::UNAUTHORIZED {
        // Session révoquée ou expirée côté serveur : on efface aussi le cookie.
        etat.oublier(&jeton);
        effacer_cookie(&etat.config, rep.headers_mut());
    }
    rep
}
