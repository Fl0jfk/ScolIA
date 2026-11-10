//! Socle 21, lot V0 : routes historiques fermées (410), réouverture d'une période avec motif
//! et journal, lectures soumises à publication (HTTP, base locale de labo).

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn etat() -> Option<AppState> {
    let url = std::env::var("ENT_DATABASE_URL").ok().filter(|u| !u.trim().is_empty())?;
    let ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    Some(AppState::nouveau(ent))
}

async fn appel(app: &axum::Router, methode: &str, uri: &str, jeton: &str, corps: Option<Value>) -> (StatusCode, Value) {
    let b = Request::builder().method(methode).uri(uri).header("authorization", format!("Bearer {jeton}"));
    let req = match corps {
        Some(c) => b
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&c).unwrap()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let st = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (st, v)
}

#[tokio::test]
async fn routes_historiques_d_ecriture_fermees_et_periode_inchangee() {
    let Some(state) = etat().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let etab = cast.etablissement_id;
    let t = chrono::Utc::now().date_naive();
    let periode = state
        .ent
        .creer_note_periode(domain_eleve::CreerNotePeriode {
            etablissement_id: etab,
            code: format!("V0{}", &uuid::Uuid::new_v4().simple().to_string()[..6]),
            libelle: "Période V0".into(),
            date_debut: t - Duration::days(5),
            date_fin: t + Duration::days(30),
            actor_user_id: None,
        })
        .await
        .unwrap();
    let matiere = state
        .ent
        .create_matiere(
            ent_db::CreateMatiere { etablissement_id: etab, code: format!("V{}", &uuid::Uuid::new_v4().simple().to_string()[..5]), libelle: "V0".into(), source_nomenclature: None },
            None,
        )
        .await
        .unwrap();
    let app = app(state.clone());
    let eleve = cast.eleve_lea_id;
    let jour = t.to_string();

    // D2 : l'ancienne route de statut est fermée pour tous (administratif, direction, admin).
    for jeton in [&cast.administratif.token, &cast.direction.token, &cast.admin.token] {
        let (st, corps) = appel(&app, "POST", &format!("/note-periodes/{}/statut", periode.id), jeton, Some(json!({"statut": "fermee"}))).await;
        assert_eq!(st, StatusCode::GONE, "{corps}");
    }
    let (_, liste) = appel(&app, "GET", "/evaluations/periodes", &cast.direction.token, None).await;
    let p = liste.as_array().unwrap().iter().find(|p| p["id"] == json!(periode.id)).unwrap();
    assert_eq!(p["statut"], "ouverte", "la route fermée ne change rien");

    // D1 : saisie historique d'une note (même hors matière, même par un professeur).
    let (st, _) = appel(&app, "POST", &format!("/eleves/{eleve}/notes"), &cast.prof.token,
        Some(json!({"matiere_id": matiere.id, "valeur": 12.0, "date_note": jour}))).await;
    assert_eq!(st, StatusCode::GONE);
    // D5 : bulletin historique publié par un professeur ou par la direction.
    for jeton in [&cast.prof.token, &cast.direction.token] {
        let (st, _) = appel(&app, "POST", &format!("/eleves/{eleve}/bulletins"), jeton, Some(json!({"periode_id": periode.id}))).await;
        assert_eq!(st, StatusCode::GONE);
    }
    // D4 : décision de conseil historique.
    let (st, _) = appel(&app, "POST", &format!("/conseils-classe/dossiers/{}/decider", uuid::Uuid::new_v4()), &cast.prof.token,
        Some(json!({"appreciation": "x", "mention": "felicitations"}))).await;
    assert_eq!(st, StatusCode::GONE);
    // Aucune note n'a été créée.
    let (st, notes) = appel(&app, "GET", &format!("/eleves/{eleve}/notes"), &cast.direction.token, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(notes.as_array().unwrap().iter().all(|n| n["libelle"].as_str() != Some("DS1")));
    // Sans session : 401, jamais 410 (la fermeture ne révèle rien à un anonyme).
    let res = app.clone().oneshot(Request::builder().method("POST").uri(format!("/eleves/{eleve}/notes"))
        .header("content-type", "application/json").body(Body::from("{}")).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn reouverture_de_periode_http_motif_obligatoire_et_journal() {
    let Some(state) = etat().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let t = chrono::Utc::now().date_naive();
    let periode = state
        .ent
        .creer_note_periode(domain_eleve::CreerNotePeriode {
            etablissement_id: cast.etablissement_id,
            code: format!("W{}", &uuid::Uuid::new_v4().simple().to_string()[..6]),
            libelle: "Période W".into(),
            date_debut: t - Duration::days(5),
            date_fin: t + Duration::days(30),
            actor_user_id: None,
        })
        .await
        .unwrap();
    let app = app(state);
    let url = format!("/evaluations/periodes/{}/statut", periode.id);
    let dir = cast.direction.token.as_str();
    let (st, _) = appel(&app, "PUT", &url, dir, Some(json!({"statut": "fermee"}))).await;
    assert_eq!(st, StatusCode::OK);
    // L'administratif ne rouvre pas (403), même avec un motif.
    let (st, _) = appel(&app, "PUT", &url, &cast.administratif.token, Some(json!({"statut": "ouverte", "motif": "je rouvre"}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // La direction sans motif : refus 4xx ; la période reste fermée.
    let (st, _) = appel(&app, "PUT", &url, dir, Some(json!({"statut": "ouverte"}))).await;
    assert!(st.is_client_error(), "{st}");
    // Avec motif : accepté et journalisé.
    let (st, corps) = appel(&app, "PUT", &url, dir, Some(json!({"statut": "ouverte", "motif": "Note de maths mal saisie"}))).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    assert_eq!(corps["statut"], "ouverte");
    let (st, j) = appel(&app, "GET", &format!("/evaluations/periodes/{}/journal", periode.id), dir, None).await;
    assert_eq!(st, StatusCode::OK);
    let l = j.as_array().unwrap();
    assert_eq!(l.len(), 2);
    assert_eq!(l[0]["motif"], "Note de maths mal saisie");
    assert_eq!((l[0]["statut_avant"].as_str(), l[0]["statut_apres"].as_str()), (Some("fermee"), Some("ouverte")));
    // Le journal est réservé à la direction (ni administratif, ni professeur, ni famille).
    for jeton in [&cast.administratif.token, &cast.prof.token, &cast.parent_lea.token] {
        let (st, _) = appel(&app, "GET", &format!("/evaluations/periodes/{}/journal", periode.id), jeton, None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
}
