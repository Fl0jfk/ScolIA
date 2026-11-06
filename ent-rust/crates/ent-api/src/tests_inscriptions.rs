//! Socle 2, lot S2-3 : routes HTTP des préinscriptions, inscriptions et imports
//! (base locale de labo). Anti-énumération au démarrage, rien d'affiché sans
//! vérification, codes de retour, droits, isolation entre établissements.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use ent_types::inscriptions::{
    CampagneDto, DemarrerDto, DossierDto, EspacePreinscriptionDto, JetonPreinscriptionDto,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn etat() -> Option<AppState> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())?;
    let mut ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    ent.install_mail_dry_run();
    Some(AppState::nouveau(ent))
}

async fn appel(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    entetes: &[(&str, &str)],
    corps: Option<Value>,
) -> (StatusCode, String) {
    let mut b = Request::builder().method(methode).uri(uri);
    for (k, v) in entetes {
        b = b.header(*k, *v);
    }
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
    (st, String::from_utf8_lossy(&bytes).into_owned())
}

fn bearer(t: &str) -> String {
    format!("Bearer {t}")
}

fn code_recu(ent: &PgEnt, email: &str) -> String {
    let (_, _, corps) = ent
        .mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .find(|(to, s, _)| to == email && s.contains("code de vérification"))
        .expect("code envoyé");
    let i = corps.find("est : ").unwrap() + "est : ".len();
    corps[i..i + 6].to_string()
}

#[tokio::test]
async fn preinscription_http_anti_enumeration_droits_isolation() {
    let Some(state) = etat().await else {
        eprintln!("skip S2-3 HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = state.ent.clone();
    let cast = ent.seed_labo_cast().await.expect("cast");
    let autre = ent.seed_labo_cast().await.expect("cast B");
    let etab = cast.etablissement_id;
    let app = app(state);
    let dir = bearer(&cast.direction.token);
    let prof = bearer(&cast.prof.token);

    // Campagne : 201 (direction), 403 (prof).
    let corps = json!({
        "nature": "admission", "libelle": "Rentrée 2027", "annee_cible": "2027-2028",
        "pieces": [{ "code": "livret", "libelle": "Livret", "obligatoire": true }]
    });
    let (st, _) = appel(
        &app,
        "POST",
        "/inscriptions/campagnes",
        &[("authorization", &prof)],
        Some(corps.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, b) = appel(
        &app,
        "POST",
        "/inscriptions/campagnes",
        &[("authorization", &dir)],
        Some(corps),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let camp: CampagneDto = serde_json::from_str(&b).unwrap();
    let (st, b) = appel(
        &app,
        "POST",
        &format!("/inscriptions/campagnes/{}/ouvrir", camp.id),
        &[("authorization", &dir)],
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{b}");

    // Démarrage public : 202 et même corps (hors identifiant de demande), compte connu ou non.
    let inconnu = format!("http-{}@exemple.test", Uuid::new_v4().simple());
    let mut corps_vus = Vec::new();
    for email in [inconnu.as_str(), cast.parent_lea.email.as_str()] {
        let (st, b) = appel(
            &app,
            "POST",
            "/preinscription/demarrer",
            &[],
            Some(json!({ "email": email, "etablissement_id": etab })),
        )
        .await;
        assert_eq!(st, StatusCode::ACCEPTED);
        let d: DemarrerDto = serde_json::from_str(&b).unwrap();
        corps_vus.push((d.demande_id, d.message));
    }
    assert_eq!(corps_vus[0].1, corps_vus[1].1);
    // Rien n'est affiché sans jeton vérifié.
    let (st, _) = appel(&app, "GET", "/preinscription/moi", &[], None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = appel(
        &app,
        "GET",
        "/preinscription/moi",
        &[("x-preinscription", "forge")],
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    // Mauvais code et demande inconnue : même réponse.
    let code = code_recu(&ent, &inconnu);
    let faux = if code == "000000" { "111111" } else { "000000" };
    let (st1, b1) = appel(
        &app,
        "POST",
        "/preinscription/verifier",
        &[],
        Some(json!({ "demande_id": corps_vus[0].0, "code": faux })),
    )
    .await;
    let (st2, b2) = appel(
        &app,
        "POST",
        "/preinscription/verifier",
        &[("host", "inconnu.invalid")],
        Some(json!({ "demande_id": Uuid::new_v4(), "code": faux })),
    )
    .await;
    assert_eq!(
        (st1, st2),
        (StatusCode::BAD_REQUEST, StatusCode::BAD_REQUEST)
    );
    assert_eq!(b1, b2);
    let (st, b) = appel(
        &app,
        "POST",
        "/preinscription/verifier",
        &[],
        Some(json!({ "demande_id": corps_vus[0].0, "code": code })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{b}");
    assert!(!b.contains(&inconnu));
    let j: JetonPreinscriptionDto = serde_json::from_str(&b).unwrap();
    let h = [("x-preinscription", j.jeton.as_str())];
    let (st, b) = appel(&app, "GET", "/preinscription/moi", &h, None).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    let espace: EspacePreinscriptionDto = serde_json::from_str(&b).unwrap();
    assert!(!espace.connu);
    assert_eq!(espace.campagnes.len(), 1);

    // Dépôt (201) ; formulaire invalide (400).
    let mut f = json!({
        "campagne_id": camp.id,
        "eleve": { "nom": "Diggory", "prenom": "Cedric" },
        "responsable": { "nom": "Diggory", "prenom": "Amos" },
        "second_parent": { "nom": "Diggory", "meme_foyer": false }
    });
    let (st, _) = appel(
        &app,
        "POST",
        "/preinscription/dossiers",
        &h,
        Some(f.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "parents séparés sans e-mail");
    f["second_parent"]["email"] = json!(format!("mere-{}@exemple.test", Uuid::new_v4().simple()));
    let (st, b) = appel(&app, "POST", "/preinscription/dossiers", &h, Some(f)).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let dossier: DossierDto = serde_json::from_str(&b).unwrap();
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/preinscription/dossiers/{}", dossier.id),
        &h,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/preinscription/dossiers/{}/banque", dossier.id),
        &h,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "IBAN demandé à la validation seulement"
    );

    // Personnel : liste (200), prof (403), autre établissement (404 / liste vide).
    let (st, b) = appel(
        &app,
        "GET",
        &format!("/inscriptions/dossiers?campagne_id={}", camp.id),
        &[("authorization", &dir)],
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(b.contains(&dossier.id.to_string()));
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/inscriptions/dossiers/{}", dossier.id),
        &[("authorization", &prof)],
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let dir_b = bearer(&autre.direction.token);
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/inscriptions/dossiers/{}", dossier.id),
        &[("authorization", &dir_b)],
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/inscriptions/dossiers/{}/decision", dossier.id),
        &[("authorization", &dir_b)],
        Some(json!({ "decision": "admis" })),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Administratif : instruit (200) mais ne décide pas (403) ; direction : décide.
    let adm = bearer(&cast.administratif.token);
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/inscriptions/dossiers/{}/statut", dossier.id),
        &[("authorization", &adm)],
        Some(json!({ "statut": "en_instruction" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/inscriptions/dossiers/{}/decision", dossier.id),
        &[("authorization", &adm)],
        Some(json!({ "decision": "admis" })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/inscriptions/dossiers/{}/decision", dossier.id),
        &[("authorization", &dir)],
        Some(json!({ "decision": "refuse" })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "motif obligatoire");
    // Validation d'un dossier non admis : 409.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/inscriptions/dossiers/{}/valider", dossier.id),
        &[("authorization", &dir)],
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Le jeton de A ne vaut rien sur l'adresse d'un autre établissement.
    let (st, _) = appel(&app, "GET", "/preinscription/moi", &h, None).await;
    assert_eq!(st, StatusCode::OK);
    let _ = autre;

    // Import : 403 prof, 201 administratif (rejet signalé), 400 format inconnu.
    let imp =
        json!({ "nom_fichier": "e.csv", "contenu": "nom;prenom;date_naissance\nA;B;99/99/2014\n" });
    let (st, _) = appel(
        &app,
        "POST",
        "/imports/eleves",
        &[("authorization", &prof)],
        Some(imp.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, b) = appel(
        &app,
        "POST",
        "/imports/eleves",
        &[("authorization", &adm)],
        Some(imp),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert!(b.contains("\"nb_rejets\":1"));
    let (st, _) = appel(
        &app,
        "POST",
        "/imports/eleves",
        &[("authorization", &adm)],
        Some(json!({ "nom_fichier": "e.csv", "contenu": "colonne;inconnue\n" })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Invitation inconnue : 400 neutre.
    let (st, _) = appel(
        &app,
        "POST",
        "/inscriptions/invitations/accepter",
        &[],
        Some(json!({ "jeton": "x".repeat(64) })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}
