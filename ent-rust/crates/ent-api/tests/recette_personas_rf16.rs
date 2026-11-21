//! Recette personas 1 (2026-11-07) : test d'attaque issu de
//! docs/recette/RECETTE_PERSONAS_1.md (C21 / RF-16).
//!
//! La direction suspend le lien d'un père séparé (ordonnance de protection) :
//! un professeur de la classe de l'élève ne doit connaître ni le motif, ni la
//! référence de la décision, ni qui paie. Appel du vrai routeur en process
//! sur une base PostgreSQL locale de test.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const MOTIF: &str = "ordonnance de protection";
const REFERENCE: &str = "JAF 2026/123";

fn garde_base() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent");
        return None;
    };
    if url.trim().is_empty() {
        eprintln!("SKIP : ENT_DATABASE_URL vide");
        return None;
    }
    let parsed = ent_db::assert_local_postgres_url(&url)
        .unwrap_or_else(|e| panic!("base de test locale uniquement : {e}"));
    let nom = parsed.path().trim_start_matches('/').to_ascii_lowercase();
    assert!(
        ["test", "proto", "lab"].iter().any(|m| nom.contains(m)),
        "le nom de la base doit contenir test, proto ou lab (refusé : {nom})"
    );
    Some(url)
}

struct Rep {
    statut: StatusCode,
    texte: String,
    json: Value,
}

async fn appel(
    routeur: &axum::Router,
    methode: &str,
    chemin: &str,
    jeton: Option<&str>,
    corps: Option<&Value>,
) -> Rep {
    let mut b = Request::builder().method(methode).uri(chemin);
    if let Some(j) = jeton {
        b = b.header("authorization", format!("Bearer {j}"));
    }
    let body = match corps {
        Some(c) => {
            b = b.header("content-type", "application/json");
            Body::from(c.to_string())
        }
        None => Body::empty(),
    };
    let res = routeur
        .clone()
        .oneshot(b.body(body).expect("requête"))
        .await
        .expect("routeur infaillible");
    let statut = res.status();
    let octets = res.into_body().collect().await.expect("corps").to_bytes();
    Rep {
        statut,
        texte: String::from_utf8_lossy(&octets).into_owned(),
        json: serde_json::from_slice(&octets).unwrap_or(Value::Null),
    }
}

fn uuid(v: &Value) -> Uuid {
    Uuid::parse_str(v.as_str().expect("uuid attendu")).expect("uuid valide")
}

async fn connexion(routeur: &axum::Router, etab: Uuid, mdp: &str, email: &str) -> String {
    let l = appel(
        routeur,
        "POST",
        "/auth/login",
        None,
        Some(&json!({"email": email, "password": mdp, "etablissement_id": etab})),
    )
    .await;
    assert_eq!(l.statut, StatusCode::OK, "connexion {email} : {}", l.statut);
    l.json["token"].as_str().expect("jeton").to_string()
}

#[test]
fn rp1_rf16_professeur_ne_voit_ni_motif_judiciaire_ni_payeurs() {
    if garde_base().is_none() {
        return;
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime de test");
    rt.block_on(async {
        let (state, _bind) = ent_api::boot_from_env()
            .await
            .unwrap_or_else(|e| panic!("démarrage ent-api : {e}"));
        let r = ent_api::app(state);

        let b = appel(&r, "POST", "/labo/bootstrap", None, None).await;
        if b.statut == StatusCode::FORBIDDEN || b.statut == StatusCode::NOT_FOUND {
            eprintln!("SKIP : labo fermé (feature `labo` et ENT_ENV=lab requis)");
            return;
        }
        assert_eq!(b.statut, StatusCode::OK, "bootstrap labo");
        let etab = uuid(&b.json["etablissement_id"]);
        let mdp = b.json["password"]
            .as_str()
            .expect("mot de passe")
            .to_string();
        let email_de = |label: &str| -> String {
            b.json["personnel"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["label"] == label)
                .unwrap_or_else(|| panic!("{label} absent du labo"))["email"]
                .as_str()
                .unwrap()
                .to_string()
        };
        let harry = b.json["eleves"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["prenom"] == "Harry")
            .expect("Harry");
        let eleve = uuid(&harry["id"]);

        let dir = connexion(&r, etab, &mdp, &email_de("Direction")).await;
        let prof = connexion(&r, etab, &mdp, &email_de("Prof 1")).await;

        // La direction ajoute un père séparé (dans un nouveau foyer) puis suspend son lien.
        let foyer = appel(
            &r,
            "POST",
            "/foyers",
            Some(&dir),
            Some(&json!({"libelle": "Foyer du père"})),
        )
        .await;
        assert_eq!(
            foyer.statut,
            StatusCode::CREATED,
            "création foyer : {}",
            foyer.texte
        );
        let foyer_id = uuid(&foyer.json["id"]);
        let email_pere = format!("pere-{}@ent.lab", Uuid::new_v4().simple());
        let lien = appel(
            &r,
            "POST",
            &format!("/foyers/{foyer_id}/liens"),
            Some(&dir),
            Some(
                &json!({"eleve_id": eleve, "email": email_pere, "display_name": "James Potter",
                         "lien": "père", "est_payeur": false}),
            ),
        )
        .await;
        assert!(
            lien.statut.is_success(),
            "rattachement du père : {} {}",
            lien.statut,
            lien.texte
        );

        let resp = appel(
            &r,
            "GET",
            &format!("/dossier/eleves/{eleve}/responsables"),
            Some(&dir),
            None,
        )
        .await;
        assert_eq!(resp.statut, StatusCode::OK, "responsables (direction)");
        let lien_id = resp
            .json
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["personne"]["email"] == email_pere.as_str())
            .map(|x| uuid(&x["lien_id"]))
            .expect("lien du père introuvable");
        let s = appel(
            &r,
            "POST",
            &format!("/dossier/liens/{lien_id}/suspendre"),
            Some(&dir),
            Some(&json!({"motif": MOTIF, "reference": REFERENCE})),
        )
        .await;
        assert_eq!(s.statut, StatusCode::OK, "suspension : {}", s.texte);

        // La direction voit toujours la décision.
        let fd = appel(
            &r,
            "GET",
            &format!("/dossier/eleves/{eleve}/fiche"),
            Some(&dir),
            None,
        )
        .await;
        assert_eq!(fd.statut, StatusCode::OK);
        assert!(
            fd.texte.contains(MOTIF) && fd.texte.contains(REFERENCE),
            "la direction doit voir motif et référence : {}",
            fd.texte
        );

        // Le professeur de la classe ne voit ni la décision judiciaire ni les payeurs.
        let fp = appel(
            &r,
            "GET",
            &format!("/dossier/eleves/{eleve}/fiche"),
            Some(&prof),
            None,
        )
        .await;
        assert_eq!(
            fp.statut,
            StatusCode::OK,
            "fiche (professeur) : {}",
            fp.texte
        );
        for (nom, rep) in [
            ("fiche", fp),
            (
                "responsables",
                appel(
                    &r,
                    "GET",
                    &format!("/dossier/eleves/{eleve}/responsables"),
                    Some(&prof),
                    None,
                )
                .await,
            ),
        ] {
            if nom == "responsables" && !rep.statut.is_success() {
                continue; // refus : rien ne fuit
            }
            assert!(
                !rep.texte.contains(MOTIF),
                "{nom} : motif judiciaire exposé au professeur"
            );
            assert!(
                !rep.texte.contains(REFERENCE),
                "{nom} : référence judiciaire exposée au professeur"
            );
            assert!(
                !rep.texte.contains("\"est_payeur\":true"),
                "{nom} : payeur exposé au professeur"
            );
            assert!(
                !rep.texte.contains("James Potter") || !rep.texte.contains(&email_pere),
                "{nom} : lien suspendu exposé au professeur"
            );
        }
    });
}
