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
//! - ailleurs (400, 409, 422…) le texte de la route est conservé (message d'affichage), borné à 500 caractères,
//!   **sauf** s'il a l'air d'un texte technique (fragment SQL, nom de contrainte, SQLSTATE, chemin de code : voir
//!   [`texte_technique`]) : le message neutre du code le remplace (faibles F1 de la contre-revue MB1) ;
//! - un 503 garde son texte d'affichage (« réessayez ») s'il passe le même filtre ; les autres 5xx ont le message
//!   neutre (`ErreurInterne`) ;
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

/// Marqueurs d'un texte **technique** (base de données, panique, chemin de code) qu'une route aurait relayé tel quel :
/// un tel texte ne sort jamais dans l'enveloppe d'une appli (faible F1, contre-revue MB1). Liste volontairement large :
/// un faux positif ne coûte qu'un message neutre.
const MARQUEURS_TECHNIQUES: &[&str] = &[
    "sqlstate",
    "duplicate key",
    "violates",
    "constraint \"",
    "contrainte \"",
    "contrainte « ",
    "relation \"",
    "column \"",
    "colonne \"",
    "syntax error",
    "error returned from database",
    "erreur de base",
    "sqlx",
    "panicked",
    "deadlock",
    "could not serialize",
    "lock timeout",
    "canceling statement",
    "null value in column",
    "foreign key",
    "unique_violation",
    "::",
    ".rs:",
];

/// Préfixes d'identifiants SQL (`ent_eleve_pkey`, `pg_catalog`) : reconnus seulement en **début de mot** (contre-revue
/// du palier : « changement_etat_civil » ou « paiement_… » contiennent `ent_` sans être un nom de table).
const PREFIXES_IDENTIFIANTS: &[&str] = &["ent_", "pg_"];

/// `true` si le texte d'une route a l'air technique (voir [`MARQUEURS_TECHNIQUES`] et [`PREFIXES_IDENTIFIANTS`]) : il
/// est alors remplacé par le message neutre du code.
pub fn texte_technique(texte: &str) -> bool {
    let bas = texte.to_lowercase();
    MARQUEURS_TECHNIQUES.iter().any(|m| bas.contains(m))
        || PREFIXES_IDENTIFIANTS.iter().any(|p| {
            bas.match_indices(p).any(|(i, _)| {
                bas[..i]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
            })
        })
}

/// Corps déjà au format de l'enveloppe (une route future qui la construit elle-même) : laissé tel quel.
fn deja_enveloppe(corps: &[u8]) -> bool {
    serde_json::from_slice::<EnveloppeErreur>(corps).is_ok()
}

/// Routes `/auth/*` qui ne demandent **aucune** session (connexion, récupération, SSO) : un 401 y refuse des
/// identifiants, même si un en-tête `Authorization` résiduel accompagne la requête.
fn route_sans_session(chemin: &str) -> bool {
    // `/auth/register` n'existe qu'avec le labo : son nom ne doit pas entrer dans le binaire de production
    // (contrôle de séparation L4 de la CI).
    #[cfg(feature = "labo")]
    if chemin == "/auth/register" {
        return true;
    }
    matches!(
        chemin,
        "/auth/login" | "/auth/forgot-password" | "/auth/reset-password"
    ) || chemin.starts_with("/auth/passkey/login/")
        || chemin.starts_with("/auth/oidc/")
        || chemin == "/auth/passkey/recuperation"
}

/// Code d'un 401 sans code explicite (palier MB2, reste de MB1) : `auth.refusee` quand des identifiants sont
/// refusés (route `/auth/*` appelée sans jeton, ou route sans session) ; partout ailleurs, jeton présent compris
/// sur `/auth/me` ou `/auth/password`, c'est `session.expiree` (l'appli purge et reconnecte). Les routes avec
/// session qui vérifient un mot de passe ou une attestation posent elles-mêmes `auth.refusee`
/// (`routes::refus_identifiants`).
fn code_401(route_auth: bool, avec_jeton: bool, sans_session: bool) -> CodeErreur {
    if route_auth && (!avec_jeton || sans_session) {
        CodeErreur::AuthRefusee
    } else {
        CodeErreur::SessionExpiree
    }
}

pub async fn enveloppe_erreur(req: Request, next: Next) -> Response<Body> {
    let appli = requete_appli(req.headers(), req.uri().path());
    let route_auth = req.uri().path().starts_with("/auth/");
    let avec_jeton = req.headers().contains_key(header::AUTHORIZATION);
    let sans_session = route_sans_session(req.uri().path());
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
        if statut == StatusCode::UNAUTHORIZED {
            code_401(route_auth, avec_jeton, sans_session)
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
            .filter(|t| {
                !t.is_empty() && !t.starts_with('{') && !t.starts_with('<') && !texte_technique(t)
            })
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

#[cfg(test)]
mod tests_f1 {
    use super::texte_technique;

    /// Faible F1 (contre-revue MB1) : les fragments SQL et les textes de panique ne passent jamais ; les messages
    /// d'affichage ordinaires passent.
    #[test]
    fn textes_techniques_reconnus_messages_ordinaires_conserves() {
        for t in [
            "duplicate key value violates unique constraint \"ent_sortie_avoir_pkey\"",
            "error returned from database: relation \"ent_x\" does not exist",
            "ERROR: could not serialize access due to concurrent update (SQLSTATE 40001)",
            "null value in column \"eleve_id\" violates not-null constraint",
            "thread 'main' panicked at crates/ent-db/src/lib.rs:12:5",
            "ent_db::finances::Erreur",
            "canceling statement due to lock timeout",
        ] {
            assert!(texte_technique(t), "doit être reconnu technique : {t}");
        }
        for t in [
            "échec simulé 422",
            "La date de fin précède la date de début.",
            // Messages réels d'ent-api (contre-revue du palier) : ils restent affichés.
            "nature : correction ou changement_etat_civil",
            "kind contrainte EDT invalide",
            "Opération concurrente, veuillez réessayer.",
            "Ce créneau est déjà réservé.",
            "Le montant doit être positif.",
        ] {
            assert!(!texte_technique(t), "message d'affichage conservé : {t}");
        }
    }
}

#[cfg(test)]
mod tests_mb2_401 {
    use super::{code_401, route_sans_session};
    use ent_types::erreurs::CodeErreur;

    /// Palier MB2 (reste de MB1, élevé E1 de la contre-revue du lot 2 de MB1 client) : un jeton mort sur `/auth/*`
    /// donne `session.expiree`, pas `auth.refusee` ; les identifiants refusés gardent `auth.refusee`.
    #[test]
    fn code_401_selon_le_jeton_et_la_route() {
        // Connexion sans jeton : identifiants refusés.
        assert_eq!(code_401(true, false, false), CodeErreur::AuthRefusee);
        // Jeton mort sur /auth/me, /auth/password… : session perdue.
        assert_eq!(code_401(true, true, false), CodeErreur::SessionExpiree);
        // Connexion avec un en-tête résiduel : toujours un refus d'identifiants.
        assert_eq!(code_401(true, true, true), CodeErreur::AuthRefusee);
        // Hors /auth/* : toujours session perdue.
        assert_eq!(code_401(false, true, false), CodeErreur::SessionExpiree);
        assert_eq!(code_401(false, false, false), CodeErreur::SessionExpiree);
    }

    #[test]
    fn routes_sans_session() {
        for c in [
            "/auth/login",
            "/auth/passkey/login/start",
            "/auth/passkey/login/finish",
            "/auth/forgot-password",
            "/auth/reset-password",
            "/auth/oidc/callback",
            "/auth/passkey/recuperation",
        ] {
            assert!(route_sans_session(c), "{c}");
        }
        for c in [
            "/auth/me",
            "/auth/password",
            "/auth/sessions",
            "/auth/passkey/register/finish",
            "/auth/passkeys/x",
        ] {
            assert!(!route_sans_session(c), "{c}");
        }
    }
}
