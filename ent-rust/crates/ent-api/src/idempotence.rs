//! `Idempotency-Key` générique (socle 11, lot MB1, `docs/socles/11_APPLICATIONS_MOBILES.md` §5.2 et §5.5).
//!
//! Couche commune posée **à l'intérieur** de la résolution de session ([`crate::auth::contexte_droits`]) : elle lit la
//! session déjà résolue, jamais ses propres droits. Sur toute écriture authentifiée (`POST`, `PUT`, `PATCH`,
//! `DELETE`) :
//!
//! | `Idempotency-Key` | requête d'appli (`X-ScolIA-Appli`) | requête web (une route `/mobile/*` sans cet en-tête compte comme le web) |
//! |---|---|---|
//! | absente | `400 idempotence.requise` | exécutée normalement |
//! | invalide (pas un UUID) | `400 idempotence.cle_invalide` | `400 idempotence.cle_invalide` |
//! | présente | registre d'idempotence (24 h) | registre d'idempotence (24 h) |
//!
//! Avec le registre (`ent_idempotence_cle`, migration 2227, cloisonné par établissement **et** session) : même clé +
//! même empreinte (méthode, chemin, corps) → réponse rejouée (`Idempotent-Replayed: true`) ; même clé + autre
//! empreinte → `409 idempotence.conflit` ; deux requêtes concurrentes de même clé → une seule s'exécute, l'autre attend
//! sa réponse (≤ 10 s) puis la rejoue, sinon `409 idempotence.en_cours`.
//!
//! Règles prudentes (D-069, `docs/socles/11_MB1_SERVEUR_LIVRAISON.md`) : une requête **sans session** (connexion,
//! rattachement) n'est pas concernée (rien à cloisonner) ; seules les réponses de succès (2xx, 3xx) sont conservées —
//! un refus ou une erreur libère la clé (un nouvel essai s'exécute pour de bon, et ne rejoue jamais un refus de droit
//! devenu faux) ; une réponse de plus de 256 Kio ou non textuelle est marquée non rejouable (un rejeu reçoit
//! `409 idempotence.non_rejouable`, jamais une seconde exécution).

use crate::enveloppe::{avec_code, declare_appli};
use crate::AppState;
use axum::body::{Body, HttpBody};
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, Response, StatusCode};
use axum::middleware::Next;
use ent_db::idempotence::{
    empreinte_requete, ReponseConservee, RevendicationCle, IDEMPOTENCE_CORPS_MAX,
};
use ent_types::erreurs::CodeErreur;
use uuid::Uuid;

/// Attente maximale d'une requête dont la clé est en cours chez une autre requête.
const ATTENTE_MAX_MS: u64 = 10_000;
const PAS_ATTENTE_MS: u64 = 50;

fn est_ecriture(m: &Method) -> bool {
    matches!(
        *m,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

fn rejouer(r: ReponseConservee) -> Response<Body> {
    let statut = StatusCode::from_u16(r.statut_http).unwrap_or(StatusCode::OK);
    let mut res = Response::new(Body::from(r.corps.unwrap_or_default()));
    *res.status_mut() = statut;
    if let Some(Ok(v)) = r.type_contenu.as_deref().map(HeaderValue::from_str) {
        res.headers_mut().insert(header::CONTENT_TYPE, v);
    }
    if let Some(Ok(v)) = r.emplacement.as_deref().map(HeaderValue::from_str) {
        res.headers_mut().insert(header::LOCATION, v);
    }
    res.headers_mut()
        .insert("idempotent-replayed", HeaderValue::from_static("true"));
    res
}

pub async fn idempotence(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response<Body> {
    if !est_ecriture(req.method()) {
        return next.run(req).await;
    }
    // Sans session résolue (connexion, rattachement, jeton invalide) : rien à cloisonner, la route répond seule.
    let Some(info) = req
        .extensions()
        .get::<crate::auth::SessionPreResolue>()
        .map(|s| s.0.clone())
    else {
        return next.run(req).await;
    };
    let appli = declare_appli(req.headers());
    let cle_brute = req
        .headers()
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string());
    let Some(cle_brute) = cle_brute else {
        if appli {
            return avec_code(
                CodeErreur::IdempotenceRequise,
                "L'en-tête Idempotency-Key est obligatoire sur une écriture.",
            );
        }
        return next.run(req).await;
    };
    let Some(cle) = (cle_brute.len() == 36)
        .then(|| Uuid::parse_str(&cle_brute).ok())
        .flatten()
    else {
        return avec_code(
            CodeErreur::IdempotenceCleInvalide,
            "L'en-tête Idempotency-Key doit être un UUID.",
        );
    };

    let (parts, body) = req.into_parts();
    let Ok(octets) = axum::body::to_bytes(body, crate::max_body_bytes()).await else {
        return avec_code(
            CodeErreur::CorpsTropGros,
            "Le contenu envoyé est trop volumineux.",
        );
    };
    let chemin = parts
        .uri
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| parts.uri.path().to_string());
    let empreinte = empreinte_requete(parts.method.as_str(), &chemin, &octets);

    let (etab, session) = (info.etablissement_id, info.session_id);
    let mut attendu_ms = 0;
    loop {
        let maintenant = (state.horloge)();
        match state
            .ent
            .idem_revendiquer(etab, session, cle, &empreinte, maintenant)
            .await
        {
            Ok(RevendicationCle::Nouvelle) => break,
            Ok(RevendicationCle::Rejeu(r)) => return rejouer(r),
            Ok(RevendicationCle::Conflit) => {
                return avec_code(
                    CodeErreur::IdempotenceConflit,
                    "Cet identifiant d'idempotence a déjà servi pour une autre requête.",
                )
            }
            Ok(RevendicationCle::NonRejouable) => {
                return avec_code(
                    CodeErreur::IdempotenceNonRejouable,
                    "Cette opération a déjà été traitée ; sa réponse n'est plus disponible.",
                )
            }
            Ok(RevendicationCle::EnCours) => {
                if attendu_ms >= ATTENTE_MAX_MS {
                    let mut res = avec_code(
                        CodeErreur::IdempotenceEnCours,
                        "Cette opération est déjà en cours de traitement.",
                    );
                    res.headers_mut()
                        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
                    return res;
                }
                tokio::time::sleep(std::time::Duration::from_millis(PAS_ATTENTE_MS)).await;
                attendu_ms += PAS_ATTENTE_MS;
            }
            Err(e) => {
                tracing::error!(error = %e, "registre d'idempotence indisponible");
                return avec_code(CodeErreur::ErreurInterne, "Erreur interne.");
            }
        }
    }

    let res = next
        .run(Request::from_parts(parts, Body::from(octets)))
        .await;
    let statut = res.status();
    if !(statut.is_success() || statut.is_redirection()) {
        if let Err(e) = state.ent.idem_abandonner(etab, session, cle).await {
            tracing::error!(error = %e, "libération d'une clé d'idempotence impossible");
        }
        return res;
    }

    let (res_parts, res_body) = res.into_parts();
    let stockable = res_body
        .size_hint()
        .upper()
        .is_some_and(|n| n <= IDEMPOTENCE_CORPS_MAX as u64);
    let (corps_octets, rendu_body) = if stockable {
        match axum::body::to_bytes(res_body, IDEMPOTENCE_CORPS_MAX).await {
            Ok(b) => (Some(b.clone()), Body::from(b)),
            // Ne devrait pas arriver (taille connue) : réponse perdue, on libère la clé.
            Err(_) => {
                let _ = state.ent.idem_abandonner(etab, session, cle).await;
                return avec_code(CodeErreur::ErreurInterne, "Erreur interne.");
            }
        }
    } else {
        (None, res_body)
    };
    let texte = corps_octets
        .as_ref()
        .and_then(|b| String::from_utf8(b.to_vec()).ok());
    let rejouable = stockable && texte.is_some();
    let conservee = ReponseConservee {
        statut_http: statut.as_u16(),
        type_contenu: res_parts
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string),
        emplacement: res_parts
            .headers
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string),
        corps: if rejouable { texte } else { None },
        rejouable,
    };
    if let Err(e) = state
        .ent
        .idem_terminer(etab, session, cle, &conservee, (state.horloge)())
        .await
    {
        tracing::error!(error = %e, "conservation d'une réponse d'idempotence impossible");
    }
    Response::from_parts(res_parts, rendu_body)
}
