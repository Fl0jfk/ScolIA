//! Session web par cookie `HttpOnly` (A6, D-011) — relais devant le routeur `ent_api::app`
//! existant, **sans modifier** `ent-auth` ni `ent-api`. Intégré au serveur de production
//! `ent-web-server` ; sert aussi de prototype autonome (`proto-session-cookie`).
//!
//! Principe :
//! - `POST /web/session/connexion` (mot de passe), `/web/session/passkey/debut` et
//!   `/web/session/passkey/fin` appellent les routes de connexion existantes, puis
//!   **posent le jeton dans un cookie** `HttpOnly; Secure; SameSite=Lax` et le
//!   **retirent du corps** : le JavaScript de la page ne le voit jamais. Le relais impose
//!   `client = web` (un navigateur ne peut pas réclamer une session mobile de 90 jours).
//! - `/web/api/…` relaie vers l'API existante en transformant le cookie en
//!   `Authorization: Bearer` (ent-api ne lit que cet en-tête, inchangé).
//! - `POST /web/session/deconnexion` révoque la session en base (`/auth/logout`) et
//!   efface le cookie.
//! - `GET /web/session/etat` : temps restant de la session (lu en base), pour l'alerte
//!   avant expiration.
//! - Protection CSRF sur toutes les requêtes qui modifient : origine autorisée **et**
//!   en-tête `X-ENT-CSRF: 1`. Elle s'applique aussi à la connexion (« login CSRF »).
//! - **Inactivité et durée maximale : la base fait foi** (`ent_session.last_seen_at`,
//!   `inactivite_max_secs`, `expires_at`, appliqués par `resolve_bearer`). Le relais ne
//!   garde **aucun état** : redémarrage et multi-instance sans effet. Un 401 d'ent-api
//!   efface le cookie.
//! - Après toute **élévation de session** (enrôlement d'une passkey obligatoire : la session
//!   passe de 15 min à 12 h), le cookie est ré-émis avec la durée lue en base (E3).
//!
//! Le cookie n'est accepté **que** sous `/web/…` : les routes existantes (`/auth`,
//! `/eleves`, l'ancienne interface `/app`, les applis mobiles en Bearer) l'ignorent.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::Router;
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
    /// Plafond de `Max-Age` du cookie (D-011 : 12 h). La durée réelle est celle de la base
    /// (`ENT_SESSION_WEB_MAX_HEURES`) ; le cookie ne dépasse jamais ce plafond.
    pub duree_max: Duration,
    /// Inactivité annoncée au navigateur par `/web/session/etat` (D-011 : 2 h). Information
    /// seulement : c'est la base (`ENT_SESSION_WEB_INACTIVITE_MINUTES`) qui l'applique.
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

#[derive(Clone)]
struct Etat {
    api: Router,
    config: Arc<Config>,
}

/// Passerelle de session web, à fusionner **avant** le routeur `api` :
/// `passerelle(api.clone(), config).merge(api)`.
pub fn passerelle(api: Router, config: Config) -> Router {
    let etat = Etat {
        api,
        config: Arc::new(config),
    };
    Router::new()
        .route("/web/session/connexion", post(connexion))
        .route("/web/session/passkey/debut", post(passkey_debut))
        .route("/web/session/passkey/fin", post(passkey_fin))
        .route("/web/session/deconnexion", post(deconnexion))
        .route("/web/session/etat", get(etat_session))
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

/// Secondes restantes avant `expires_at` (0 si dépassée ou illisible).
fn reste_avant(expires_at: &str) -> u64 {
    chrono::DateTime::parse_from_rfc3339(expires_at)
        .map(|d| (d.timestamp() - chrono::Utc::now().timestamp()).max(0) as u64)
        .unwrap_or(0)
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
    // Max-Age ≤ plafond web et ≤ expiration réelle de la session en base.
    let max_age = json
        .get("expires_at")
        .and_then(|v| v.as_str())
        .map_or(0, reste_avant)
        .min(etat.config.duree_max.as_secs());
    let corps = serde_json::to_vec(&json).expect("JSON");
    parties.headers.remove(header::CONTENT_LENGTH);
    parties
        .headers
        .append(header::SET_COOKIE, cookie(&etat.config, &jeton, max_age));
    Response::from_parts(parties, Body::from(corps))
}

/// Ligne « session courante » de `GET /auth/sessions` (lue en base par ent-api).
struct SessionCourante {
    /// Secondes avant l'expiration absolue (`expires_at`), 0 si déjà dépassée.
    expire_dans: u64,
    client: String,
}

/// Interroge la base (via `GET /auth/sessions`, route existante) sur la session du jeton.
/// `Err(statut)` : refus d'ent-api (401 session invalide, 403 session d'enrôlement…).
async fn session_courante(etat: &Etat, jeton: &str) -> Result<SessionCourante, StatusCode> {
    let req = Request::builder()
        .method(Method::GET)
        .uri("/auth/sessions")
        .body(Body::empty())
        .expect("requête interne");
    let rep = transmettre(etat, req, "/auth/sessions", Some(jeton)).await;
    if rep.status() != StatusCode::OK {
        return Err(rep.status());
    }
    let octets = to_bytes(rep.into_body(), LIMITE_CORPS)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let liste: Vec<serde_json::Value> =
        serde_json::from_slice(&octets).map_err(|_| StatusCode::BAD_GATEWAY)?;
    let courante = liste
        .iter()
        .find(|s| s.get("courante").and_then(|v| v.as_bool()) == Some(true))
        .ok_or(StatusCode::BAD_GATEWAY)?;
    let expires_at = courante
        .get("expires_at")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_GATEWAY)?;
    Ok(SessionCourante {
        expire_dans: reste_avant(expires_at),
        client: courante
            .get("client")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
    })
}

/// Routes d'ent-api qui **changent la durée ou la portée** de la session courante : après un
/// succès, le cookie est ré-émis avec la durée lue en base (sinon il garderait celle de la
/// connexion : 15 min après l'enrôlement d'une passkey obligatoire, alors que la session
/// passe à 12 h).
const ROUTES_ELEVATION: &[&str] = &["/auth/passkey/register/finish"];

// ---------------------------------------------------------------------------
// Routes

/// Force `client = web` dans le corps JSON d'une demande de connexion : la durée de session
/// (12 h / 2 h d'inactivité) ne doit pas dépendre de ce que le navigateur écrit.
async fn imposer_client_web(req: Request) -> Request {
    let (mut parties, corps) = req.into_parts();
    let Ok(octets) = to_bytes(corps, LIMITE_CORPS).await else {
        return Request::from_parts(parties, Body::empty());
    };
    let nouveau = match serde_json::from_slice::<serde_json::Value>(&octets) {
        Ok(serde_json::Value::Object(mut o)) => {
            o.insert("client".into(), "web".into());
            serde_json::to_vec(&o).unwrap_or_else(|_| octets.to_vec())
        }
        _ => octets.to_vec(),
    };
    parties.headers.remove(header::CONTENT_LENGTH);
    Request::from_parts(parties, Body::from(nouveau))
}

async fn connexion(State(etat): State<Etat>, req: Request) -> Response {
    if let Err(m) = garde_csrf(&etat.config, req.headers()) {
        return texte(StatusCode::FORBIDDEN, m);
    }
    let req = imposer_client_web(req).await;
    let rep = transmettre(&etat, req, "/auth/login", None).await;
    poser_session(&etat, rep).await
}

async fn passkey_debut(State(etat): State<Etat>, req: Request) -> Response {
    if let Err(m) = garde_csrf(&etat.config, req.headers()) {
        return texte(StatusCode::FORBIDDEN, m);
    }
    let req = imposer_client_web(req).await;
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
        // Révocation en base (route existante) ; un jeton déjà invalide est sans importance.
        let _ = transmettre(&etat, req, "/auth/logout", Some(&jeton)).await;
    }
    let mut r = StatusCode::NO_CONTENT.into_response();
    sans_cache(r.headers_mut());
    effacer_cookie(&etat.config, r.headers_mut());
    r
}

/// `GET /web/session/etat` : `{ "expire_dans": s, "inactivite_max": s, "client": "web" }`.
/// `expire_dans` = temps restant avant l'**expiration absolue** (base). L'appel compte
/// comme une activité (comme toute requête authentifiée) : le navigateur ne l'utilise
/// qu'au chargement et à la demande, jamais en boucle.
async fn etat_session(State(etat): State<Etat>, req: Request) -> Response {
    let Some(jeton) = lire_cookie(req.headers(), etat.config.nom_cookie()) else {
        return texte(StatusCode::UNAUTHORIZED, "non connecté");
    };
    match session_courante(&etat, &jeton).await {
        Ok(s) => {
            let mut r = axum::Json(serde_json::json!({
                "expire_dans": s.expire_dans,
                "inactivite_max": etat.config.inactivite.as_secs(),
                "client": s.client,
            }))
            .into_response();
            sans_cache(r.headers_mut());
            r
        }
        Err(StatusCode::UNAUTHORIZED) => session_refusee(&etat),
        Err(StatusCode::FORBIDDEN) => texte(StatusCode::FORBIDDEN, "état indisponible"),
        Err(_) => texte(StatusCode::BAD_GATEWAY, "état illisible"),
    }
}

/// 401 « session expirée » + effacement du cookie (la base a refusé le jeton).
fn session_refusee(etat: &Etat) -> Response {
    let mut r = texte(StatusCode::UNAUTHORIZED, "session expirée");
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
    let sure = methode_sure(req.method());
    if !sure {
        if let Err(m) = garde_csrf(&etat.config, req.headers()) {
            return texte(StatusCode::FORBIDDEN, m);
        }
    }
    let elevation = !sure && ROUTES_ELEVATION.iter().any(|r| chemin == *r);
    let cible = match req.uri().query() {
        Some(q) => format!("{chemin}?{q}"),
        None => chemin,
    };
    // Inactivité, durée maximale, révocation : c'est ent-api (donc la base) qui tranche.
    let mut rep = transmettre(&etat, req, &cible, Some(&jeton)).await;
    if rep.status() == StatusCode::UNAUTHORIZED {
        // Session expirée, inactive ou révoquée : on efface aussi le cookie.
        effacer_cookie(&etat.config, rep.headers_mut());
    } else if elevation && rep.status().is_success() {
        // Session élevée : cookie ré-émis avec la nouvelle expiration (lue en base).
        if let Ok(s) = session_courante(&etat, &jeton).await {
            let max_age = s.expire_dans.min(etat.config.duree_max.as_secs());
            rep.headers_mut()
                .append(header::SET_COOKIE, cookie(&etat.config, &jeton, max_age));
        }
    }
    rep
}
