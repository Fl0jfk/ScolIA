//! Enveloppe d'erreur codée (socle 11, lot MB1, `docs/socles/11_APPLICATIONS_MOBILES.md` §5.3).
//!
//! **Une couche commune**, pas une réécriture route par route : les routes continuent de renvoyer `(StatusCode, String)`.
//! Quand la requête vient d'une appli (en-tête `X-ScolIA-Appli`) ou vise une route `/mobile/*`, toute réponse d'erreur
//! (statut ≥ 400) est convertie en `{ "erreur": { code, message, champs, reessayer_apres_s, trace } }` ; le web garde
//! exactement ses réponses actuelles.
//!
//! Règles :
//! - le `code` vient de l'en-tête interne `x-scolia-code` quand une route le pose ([`avec_code`]), sinon du statut
//!   ([`CodeErreur::par_defaut`]) ; l'en-tête interne n'est jamais servi ;
//! - pour 401, 403, 404 et 500 le message est **toujours** le message neutre du code : le texte d'origine d'une route
//!   (qui peut dire « vous êtes visé par une interdiction » ou « cet élève existe mais… ») n'est jamais reproduit
//!   (pas d'oracle d'existence, pas de cause d'un refus d'accès à une personne) ;
//! - ailleurs (400, 409, 422…) le texte de la route est conservé (message d'affichage), borné à 500 caractères ;
//! - `trace` = identifiant de requête (`x-request-id`, corrélé au journal) : jamais une donnée personnelle.

use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, HeaderName, HeaderValue, Response, StatusCode};
use axum::middleware::Next;
use ent_types::erreurs::{CodeErreur, EnveloppeErreur};

/// En-tête de requête déclarant l'appli (`android/1.4.2 (build 142)`) : base du mode « enveloppe » et de l'exigence
/// d'`Idempotency-Key`.
pub const EN_TETE_APPLI: &str = "x-scolia-appli";
/// En-tête **interne** (réponse → couche) : code précis choisi par la route. Retiré avant l'envoi.
pub const EN_TETE_CODE: &str = "x-scolia-code";

const MESSAGE_MAX: usize = 500;
const CORPS_LU_MAX: usize = 16 * 1024;

/// `true` si la requête déclare une appli (`X-ScolIA-Appli` non vide) : seule condition qui rend `Idempotency-Key`
/// obligatoire sur une écriture authentifiée.
pub fn declare_appli(en_tetes: &axum::http::HeaderMap) -> bool {
    en_tetes
        .get(EN_TETE_APPLI)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| !v.trim().is_empty())
}

/// `true` si la réponse d'erreur doit être servie sous forme d'enveloppe : appli déclarée **ou** route `/mobile/*`.
pub fn requete_appli(en_tetes: &axum::http::HeaderMap, chemin: &str) -> bool {
    declare_appli(en_tetes) || chemin == "/mobile" || chemin.starts_with("/mobile/")
}

/// Réponse d'erreur portant un code précis (la couche l'enveloppe pour les applis ; le web reçoit le texte seul).
pub fn avec_code(code: CodeErreur, message: impl Into<String>) -> Response<Body> {
    let statut = StatusCode::from_u16(code.statut_http()).unwrap_or(StatusCode::BAD_REQUEST);
    let mut res = Response::new(Body::from(message.into()));
    *res.status_mut() = statut;
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    res.headers_mut().insert(
        HeaderName::from_static(EN_TETE_CODE),
        HeaderValue::from_static(code.as_str()),
    );
    res
}

/// Identifiant de requête (`trace`) ; `inconnue` s'il manque ou s'il n'a pas un format sûr.
fn trace_de(req: &Request) -> String {
    req.headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| crate::http_garde::request_id_sur(v))
        .unwrap_or("inconnue")
        .to_string()
}

/// Corps déjà au format de l'enveloppe (une route future qui la construit elle-même) : laissé tel quel.
fn deja_enveloppe(corps: &[u8]) -> bool {
    serde_json::from_slice::<EnveloppeErreur>(corps).is_ok()
}

pub async fn enveloppe_erreur(req: Request, next: Next) -> Response<Body> {
    let appli = requete_appli(req.headers(), req.uri().path());
    let route_auth = req.uri().path().starts_with("/auth/");
    let trace = trace_de(&req);
    let mut res = next.run(req).await;

    // L'en-tête interne ne sort jamais, mobile ou non.
    let explicite = res
        .headers_mut()
        .remove(EN_TETE_CODE)
        .and_then(|v| v.to_str().ok().and_then(CodeErreur::depuis_texte));
    let statut = res.status();
    if !appli || statut.as_u16() < 400 {
        return res;
    }

    let (mut parts, body) = res.into_parts();
    let brut = axum::body::to_bytes(body, CORPS_LU_MAX)
        .await
        .unwrap_or_default();
    if explicite.is_none() && deja_enveloppe(&brut) {
        return Response::from_parts(parts, Body::from(brut));
    }

    let code = explicite.unwrap_or_else(|| {
        if statut == StatusCode::UNAUTHORIZED && route_auth {
            CodeErreur::AuthRefusee
        } else {
            CodeErreur::par_defaut(statut.as_u16())
        }
    });
    let message = if code.message_toujours_neutre() {
        code.message_neutre().to_string()
    } else {
        std::str::from_utf8(&brut)
            .ok()
            .map(str::trim)
            .filter(|t| !t.is_empty() && !t.starts_with('{') && !t.starts_with('<'))
            .map(|t| t.chars().take(MESSAGE_MAX).collect::<String>())
            .unwrap_or_else(|| code.message_neutre().to_string())
    };
    let reessayer = parts
        .headers
        .get(header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u32>().ok())
        .or(match code {
            CodeErreur::IdempotenceEnCours => Some(1),
            _ => None,
        });
    let mut env = EnveloppeErreur::nouvelle(code, message, trace);
    env.erreur.reessayer_apres_s = reessayer;
    let json = serde_json::to_vec(&env).unwrap_or_default();

    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    Response::from_parts(parts, Body::from(json))
}
