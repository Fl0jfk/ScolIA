//! Observabilité HTTP : middleware de comptage et endpoint `/metrics`.
//!
//! Pourquoi un middleware maison plutôt qu'`axum-prometheus` : l'entrepôt de
//! métriques est déjà dans `ent-obs` (sans dépendance) et le point d'attention
//! ici est la **cardinalité** : l'étiquette `route` vient de `MatchedPath`
//! (le gabarit `/auth/passkeys/{id}`), jamais du chemin brut — sinon chaque
//! identifiant d'élève créerait une série.

use crate::AppState;
use axum::extract::{MatchedPath, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ent_obs::{Genre, Metrics};

/// Politique d'accès à `/metrics`.
///
/// - `Jeton(t)` : `ENT_METRICS_TOKEN` défini → `Authorization: Bearer t` exigé ;
/// - `Ouvert` : `ENT_METRICS=1` (sans jeton) → accès libre (scrape local) ;
/// - `Desactive` : défaut → 404 (aucune surface d'observabilité exposée).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricsAcces {
    Desactive,
    Ouvert,
    Jeton(String),
}

impl MetricsAcces {
    pub fn depuis_env() -> Self {
        if let Ok(jeton) = std::env::var("ENT_METRICS_TOKEN") {
            let jeton = jeton.trim().to_string();
            if !jeton.is_empty() {
                return Self::Jeton(jeton);
            }
        }
        match std::env::var("ENT_METRICS")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "1" | "true" | "oui" | "yes" | "on" => Self::Ouvert,
            _ => Self::Desactive,
        }
    }
}

/// Déclare les familles (aide + genre) dès le démarrage : l'export `/metrics`
/// porte alors `# HELP` / `# TYPE` même avant la première requête.
pub fn installer_familles(obs: &Metrics) {
    obs.declarer(
        "ent_http_requetes_total",
        Genre::Compteur,
        "Requêtes HTTP traitées, par méthode, gabarit de route et statut",
    );
    obs.declarer(
        "ent_http_duree_secondes",
        Genre::Histogramme,
        "Durée de traitement des requêtes HTTP en secondes",
    );
    obs.declarer(
        "ent_http_requetes_en_cours",
        Genre::Jauge,
        "Requêtes HTTP en cours de traitement",
    );
    obs.declarer(
        "ent_db_pool_connexions",
        Genre::Jauge,
        "Connexions Postgres du pool par état",
    );
    obs.declarer(
        "ent_sessions_actives",
        Genre::Jauge,
        "Sessions ENT valides (non révoquées, non expirées)",
    );
    obs.declarer(
        "ent_passkeys_total",
        Genre::Jauge,
        "Passkeys (2ᵉ facteur) enrôlées",
    );
    obs.declarer(
        "ent_eleves_total",
        Genre::Jauge,
        "Élèves en base (tous statuts)",
    );
    obs.declarer(
        "ent_etablissements_total",
        Genre::Jauge,
        "Établissements ayant au moins une adhésion",
    );
    obs.declarer(
        "ent_mails_outbox",
        Genre::Jauge,
        "Courriers du outbox par statut (dry_run = jamais parti)",
    );
    obs.declarer(
        "ent_sondes_erreur",
        Genre::Jauge,
        "1 si la collecte des sondes base a échoué au dernier scrape",
    );
    obs.declarer(
        "ent_info",
        Genre::Jauge,
        "Version, SHA git et tag de migration du binaire",
    );
}

/// Middleware : compte chaque requête (statut + latence) sur son gabarit.
pub async fn suivre_requetes(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let obs = state.obs.clone();
    let methode = req.method().as_str().to_string();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "inconnu".to_string());

    obs.jauge_ajouter("ent_http_requetes_en_cours", &[], 1);
    let mesure = ent_obs::Mesure::demarrer();
    let reponse = next.run(req).await;
    obs.jauge_ajouter("ent_http_requetes_en_cours", &[], -1);

    let statut = reponse.status().as_u16().to_string();
    obs.compteur_incr(
        "ent_http_requetes",
        &[
            ("methode", methode.as_str()),
            ("route", route.as_str()),
            ("statut", statut.as_str()),
        ],
    );
    obs.histogramme_observer(
        "ent_http_duree_secondes",
        &[("methode", methode.as_str()), ("route", route.as_str())],
        mesure.ecoule_secondes(),
    );
    reponse
}

/// `GET /metrics` — format texte Prometheus (`version=0.0.4`).
pub async fn metrics_handler(State(state): State<AppState>, entetes: HeaderMap) -> Response {
    match &state.metrics_acces {
        MetricsAcces::Desactive => StatusCode::NOT_FOUND.into_response(),
        MetricsAcces::Ouvert => rendre(&state).await,
        MetricsAcces::Jeton(attendu) => {
            let fourni = entetes
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .unwrap_or("")
                .trim();
            if !jeton_correspond(attendu, fourni) {
                return (
                    StatusCode::UNAUTHORIZED,
                    [(header::WWW_AUTHENTICATE, "Bearer")],
                    "jeton de métriques requis\n",
                )
                    .into_response();
            }
            rendre(&state).await
        }
    }
}

async fn rendre(state: &AppState) -> Response {
    let obs = &state.obs;
    obs.jauge_definir(
        "ent_info",
        &[
            ("version", crate::VERSION),
            ("git_sha", crate::GIT_SHA),
            ("migration_tag", ent_db::MIGRATION_TAG),
        ],
        1,
    );

    match state.ent.sondes_metriques().await {
        Ok(sondes) => {
            obs.jauge_definir("ent_sondes_erreur", &[], 0);
            for sonde in sondes {
                let paires: Vec<(&str, &str)> = sonde
                    .etiquettes
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                obs.jauge_definir(&sonde.nom, &paires, sonde.valeur);
            }
        }
        Err(e) => {
            // Le scrape reste servi (l'alerte porte sur `ent_sondes_erreur`).
            obs.jauge_definir("ent_sondes_erreur", &[], 1);
            tracing::warn!("sondes métriques indisponibles : {e}");
        }
    }

    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        obs.rendre(),
    )
        .into_response()
}

/// Comparaison à temps constant (évite de fuiter la longueur / le préfixe du
/// jeton par le temps de réponse).
fn jeton_correspond(attendu: &str, fourni: &str) -> bool {
    let (a, b) = (attendu.as_bytes(), fourni.as_bytes());
    let mut diff = (a.len() ^ b.len()) as u64;
    let max = a.len().max(b.len());
    for i in 0..max {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= (x ^ y) as u64;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jeton_temps_constant() {
        assert!(jeton_correspond("secret", "secret"));
        assert!(!jeton_correspond("secret", "secreu"));
        assert!(!jeton_correspond("secret", "secret-plus-long"));
        assert!(!jeton_correspond("secret", ""));
        assert!(!jeton_correspond("", "x"));
        assert!(jeton_correspond("", ""));
    }

    #[test]
    fn familles_declarees_sans_conflit() {
        let obs = Metrics::new();
        installer_familles(&obs);
        obs.compteur_incr("ent_http_requetes", &[("methode", "GET")]);
        obs.jauge_definir("ent_sessions_actives", &[], 1);
        let texte = obs.rendre();
        assert!(
            texte.contains("# TYPE ent_http_requetes_total counter"),
            "{texte}"
        );
        assert!(
            texte.contains("# TYPE ent_sessions_actives gauge"),
            "{texte}"
        );
        assert!(
            texte.contains("# HELP ent_http_requetes_total Requêtes HTTP traitées"),
            "{texte}"
        );
    }
}
