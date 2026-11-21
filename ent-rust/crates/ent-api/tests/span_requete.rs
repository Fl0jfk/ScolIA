//! Span de requête : binaire de test **séparé** (processus dédié) car la capture
//! de journal dépend du cache d'intérêt des points d'instrumentation `tracing`,
//! que des tests concurrents dans le même processus peuvent figer à « jamais ».
//! Sans base ni feature `labo`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ent_api::{app, AppState};
use ent_db::PgEnt;
use serde_json::Value;
use tower::ServiceExt;

/// Capture de journal : le span de requête porte méthode, gabarit de route et
/// identifiant de requête, jamais la query (jeton de réinitialisation).
#[tokio::test]
async fn span_de_requete_sans_query_ni_uri_brute() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct Tampon(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Tampon {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for Tampon {
        type Writer = Tampon;
        fn make_writer(&'a self) -> Tampon {
            self.clone()
        }
    }

    let tampon = Tampon::default();
    let abonne = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(tampon.clone())
        .finish();
    let _garde = tracing::subscriber::set_default(abonne);

    // Pas de base nécessaire : /health ne touche pas Postgres.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://u:p@127.0.0.1:1/aucune")
        .unwrap();
    let app = app(AppState::nouveau(PgEnt::from_pool(pool)));
    let res = app
        .oneshot(
            Request::builder()
                .uri("/health?token=SECRET-DE-RESET&q=Nom-Eleve")
                .header("x-request-id", "req-span-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let sortie = String::from_utf8(tampon.0.lock().unwrap().clone()).unwrap();
    assert!(!sortie.contains("SECRET-DE-RESET"), "query dans le journal : {sortie}");
    assert!(!sortie.contains("Nom-Eleve"), "query dans le journal : {sortie}");
    let ligne = sortie
        .lines()
        .find(|l| l.contains("finished processing request"))
        .unwrap_or_else(|| panic!("pas de ligne de fin de requête : {sortie}"));
    let v: Value = serde_json::from_str(ligne).unwrap();
    assert_eq!(v["span"]["methode"], "GET");
    assert_eq!(v["span"]["route"], "/health");
    assert_eq!(v["span"]["request_id"], "req-span-1");
    assert!(v["status"].is_number() || v["fields"]["status"].is_number() || ligne.contains("status"));
}
