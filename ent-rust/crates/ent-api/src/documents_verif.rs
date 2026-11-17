//! Routes **publiques sans session** de la vérification des documents (socle 7, lot D3 ; `07_DOCUMENTS.md` §9.7, §17.7).
//!
//! * `GET  /verifier/{code}` — vérification **par code** (page publique de l'établissement de l'adresse, D-012) ;
//! * `POST /verifier` — vérification **par dépôt** du PDF reçu (corps = octets du fichier, 10 Mio au plus, calcul de
//!   l'empreinte sans décodage, **rien n'est conservé**).
//!
//! Garde-fous : l'établissement vient **de l'adresse** (jamais du client) ; une adresse inconnue répond 404 ; un code
//! mal formé, inexistant ou d'un autre établissement donne **la même réponse** 200 `{"resultat":"inconnu"}` ; les
//! réponses portent `noindex` et `no-store` ; le débit est limité par adresse IP (`anti_abus::regle`) ; les données
//! sont celles du mode de vérification de la nature (jamais d'adresse, de classe ni de contenu). Toute nouvelle route
//! publique de ce fichier doit rester dans la liste `PUBLIQUES` de la matrice de séparation.

use axum::body::{to_bytes, Body};
use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_db::documents_natures::{EtatVerifie, ReponseDepot, ReponseVerification, DEPOT_MAX_OCTETS};
use serde_json::{json, Value};

use crate::routes::map_db;
use crate::tenant::TenantHote;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/verifier/{code}", get(verifier_code))
        .route(
            "/verifier",
            post(verifier_depot).layer(axum::extract::DefaultBodyLimit::max(DEPOT_MAX_OCTETS + 1)),
        )
}

/// En-têtes communs : jamais indexé, jamais mis en cache.
fn publique(corps: Value) -> Response {
    let mut r = Json(corps).into_response();
    let h = r.headers_mut();
    h.insert(
        "x-robots-tag",
        HeaderValue::from_static("noindex, nofollow"),
    );
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    r
}

fn etat(e: &EtatVerifie) -> Value {
    match e {
        EtatVerifie::Valide => json!({ "code": "valide" }),
        EtatVerifie::Remplace { le } => json!({ "code": "remplace", "le": le.to_string() }),
        EtatVerifie::Annule { le } => json!({ "code": "annule", "le": le.to_string() }),
    }
}

fn inconnu() -> Value {
    json!({ "resultat": "inconnu" })
}

fn hote_inconnu() -> (StatusCode, String) {
    (
        StatusCode::NOT_FOUND,
        "établissement inconnu pour cette adresse".to_string(),
    )
}

async fn verifier_code(
    State(s): State<AppState>,
    tenant: TenantHote,
    Path(code): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let etab = tenant.etablissement_id.ok_or_else(hote_inconnu)?;
    // Un code lu sur un document ne dépasse jamais 40 caractères (séparateurs compris) : au-delà, même réponse.
    let rep = if code.chars().count() > 40 {
        ReponseVerification::Inconnu
    } else {
        s.ent
            .verifier_code_public(etab, &code)
            .await
            .map_err(map_db)?
    };
    Ok(publique(match rep {
        ReponseVerification::Inconnu => inconnu(),
        ReponseVerification::Authentique {
            emetteur,
            emis_le,
            etat: e,
        } => json!({
            "resultat": "authentique", "emetteur": emetteur, "emis_le": emis_le.to_string(), "etat": etat(&e),
        }),
        ReponseVerification::Complete {
            emetteur,
            nature_libelle,
            emis_le,
            etat: e,
            empreinte_sha256,
            titulaire,
        } => json!({
            "resultat": "complete", "emetteur": emetteur, "nature": nature_libelle, "emis_le": emis_le.to_string(),
            "etat": etat(&e), "empreinte_sha256": empreinte_sha256, "titulaire": titulaire,
        }),
    }))
}

async fn verifier_depot(
    State(s): State<AppState>,
    tenant: TenantHote,
    corps: Body,
) -> Result<Response, (StatusCode, String)> {
    let etab = tenant.etablissement_id.ok_or_else(hote_inconnu)?;
    let octets = to_bytes(corps, DEPOT_MAX_OCTETS).await.map_err(|_| {
        (
            StatusCode::PAYLOAD_TOO_LARGE,
            "fichier trop volumineux (10 Mio au plus)".to_string(),
        )
    })?;
    let rep = s
        .ent
        .verifier_par_depot(etab, &octets)
        .await
        .map_err(map_db)?;
    drop(octets);
    Ok(publique(match rep {
        ReponseDepot::Different => json!({ "resultat": "different" }),
        ReponseDepot::IdentiqueSigne {
            emetteur,
            emis_le,
            etat: e,
            nb_signatures,
            complet_le,
            dossier_de_preuve,
        } => json!({
            "resultat": if dossier_de_preuve { "identique_dossier_de_preuve" } else { "identique_signe" },
            "emetteur": emetteur, "emis_le": emis_le.to_string(), "etat": etat(&e),
            "nb_signatures": nb_signatures, "complet_le": complet_le.to_string(),
        }),
        ReponseDepot::Identique {
            emetteur,
            emis_le,
            etat: e,
        } => json!({
            "resultat": "identique", "emetteur": emetteur, "emis_le": emis_le.to_string(), "etat": etat(&e),
        }),
    }))
}
