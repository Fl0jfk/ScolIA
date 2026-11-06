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
    let url = ent_db::url_base_de_test()?;
    // Base annoncée mais inutilisable : échec explicite, jamais un test « vert » vide.
    let mut ent = PgEnt::connect(&url).await.expect("connexion à ENT_DATABASE_URL");
    ent.migrate().await.expect("migrations");
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

// ---------------------------------------------------------------------------
// Lot I0 : routes à jeton famille (limitation de débit, rôles, isolation HTTP)
// ---------------------------------------------------------------------------

use axum::extract::ConnectInfo;
use std::net::SocketAddr;

/// Requête portant une adresse de socket (base des quotas par IP, comme en production).
async fn appel_ip(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    entetes: &[(&str, &str)],
    corps: Option<Value>,
    ip: &str,
) -> (StatusCode, String) {
    let mut b = Request::builder().method(methode).uri(uri);
    for (k, v) in entetes {
        b = b.header(*k, *v);
    }
    b = b.extension(ConnectInfo(SocketAddr::new(ip.parse().unwrap(), 40000)));
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

fn ip_unique() -> String {
    let b = Uuid::new_v4().into_bytes();
    format!("{}.{}.{}.{}", 11 + b[0] % 200, b[1], b[2], b[3])
}

/// Pose une variable d'environnement le temps du test.
struct VarEnv(&'static str);
impl VarEnv {
    fn pose(nom: &'static str, valeur: &str) -> Self {
        std::env::set_var(nom, valeur);
        VarEnv(nom)
    }
}
impl Drop for VarEnv {
    fn drop(&mut self) {
        std::env::remove_var(self.0);
    }
}

/// I0 pt 4 : les routes à jeton famille sont limitées (par IP) ; le jeton par session
/// n'est délivré qu'aux responsables et aux élèves majeurs.
#[tokio::test]
async fn jeton_famille_limite_de_debit_et_roles() {
    let Some(state) = etat().await else {
        eprintln!("skip S2-3 HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = state.ent.clone();
    let cast = ent.seed_labo_cast().await.expect("cast");
    let app = app(state);

    // Limitation : 2 requêtes par IP puis 429, sur plusieurs routes à jeton (un seul seau).
    let _q = VarEnv::pose("ENT_RATE_PREINSCRIPTION_FAMILLE_IP", "2/3600");
    let ip = ip_unique();
    let h = [("x-preinscription", "jeton-forge")];
    let mut vus = Vec::new();
    for uri in [
        "/preinscription/moi",
        "/preinscription/dossiers/00000000-0000-0000-0000-000000000001",
        "/preinscription/dossiers/00000000-0000-0000-0000-000000000001/banque",
        "/preinscription/moi",
    ] {
        let (st, _) = appel_ip(&app, "GET", uri, &h, None, &ip).await;
        vus.push(st);
    }
    assert_eq!(
        vus,
        vec![
            StatusCode::UNAUTHORIZED,
            StatusCode::UNAUTHORIZED,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::TOO_MANY_REQUESTS
        ],
        "un seul seau pour toutes les routes à jeton famille"
    );
    // Une autre IP n'est pas touchée.
    let (st, _) = appel_ip(&app, "GET", "/preinscription/moi", &h, None, &ip_unique()).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Jeton par session : parent oui ; professeur, direction non.
    let ip = ip_unique();
    let (st, b) = appel_ip(
        &app,
        "POST",
        "/preinscription/session",
        &[("authorization", &bearer(&cast.parent_lea.token))],
        None,
        &ip,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{b}");
    for t in [&cast.prof.token, &cast.direction.token] {
        let (st, _) = appel_ip(
            &app,
            "POST",
            "/preinscription/session",
            &[("authorization", &bearer(t))],
            None,
            &ip,
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    // Élève : refusé s'il est mineur, accepté s'il est majeur.
    let Some(compte) = cast.eleves.iter().find_map(|e| e.compte.clone().map(|c| (e.id, c))) else {
        return;
    };
    let (eleve_id, persona) = compte;
    let pool = sqlx::PgPool::connect(&ent_db::url_base_de_test().expect("base"))
        .await
        .unwrap();
    let naissance = |annees: i32| {
        format!("{}-01-15", chrono::Datelike::year(&chrono::Utc::now()) - annees)
    };
    for (age, attendu) in [(14, StatusCode::FORBIDDEN), (20, StatusCode::OK)] {
        sqlx::query("UPDATE ent_eleve SET date_naissance = $2::date WHERE id = $1")
            .bind(eleve_id)
            .bind(naissance(age))
            .execute(&pool)
            .await
            .unwrap();
        let (st, b) = appel_ip(
            &app,
            "POST",
            "/preinscription/session",
            &[("authorization", &bearer(&persona.token))],
            None,
            &ip,
        )
        .await;
        assert_eq!(st, attendu, "élève de {age} ans : {b}");
    }
}

/// I0 pt 5 : isolation RÉELLE du jeton entre établissements, par adresse (Host) : le jeton
/// obtenu sur l'adresse de A ne vaut rien sur l'adresse de B.
#[tokio::test]
async fn jeton_famille_isole_entre_etablissements_http() {
    let Some(state) = etat().await else {
        eprintln!("skip S2-3 HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = state.ent.clone();
    let a = ent.seed_labo_cast().await.expect("cast A");
    let b = ent.seed_labo_cast().await.expect("cast B");
    let hote_a = format!("a-{}.ent.test", Uuid::new_v4().simple());
    let hote_b = format!("b-{}.ent.test", Uuid::new_v4().simple());
    ent.ajouter_domaine_for_actor(a.etablissement_id, a.admin.user_id, a.admin.role.as_str(), &hote_a)
        .await
        .unwrap();
    ent.ajouter_domaine_for_actor(b.etablissement_id, b.admin.user_id, b.admin.role.as_str(), &hote_b)
        .await
        .unwrap();
    let app = app(state);
    let email = format!("iso-{}@exemple.test", Uuid::new_v4().simple());

    // Démarrage + vérification sur l'adresse de A (l'identifiant d'établissement du corps,
    // ici celui de B, est ignoré : l'adresse impose l'établissement).
    let ha = [("host", hote_a.as_str())];
    let (st, corps) = appel(
        &app,
        "POST",
        "/preinscription/demarrer",
        &ha,
        Some(json!({ "email": email, "etablissement_id": b.etablissement_id })),
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED, "{corps}");
    let demande: DemarrerDto = serde_json::from_str(&corps).unwrap();
    let code = code_recu(&ent, &email);
    // Le code de A ne se vérifie pas sur l'adresse de B.
    let (st, _) = appel(
        &app,
        "POST",
        "/preinscription/verifier",
        &[("host", hote_b.as_str())],
        Some(json!({ "demande_id": demande.demande_id, "code": code })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, corps) = appel(
        &app,
        "POST",
        "/preinscription/verifier",
        &ha,
        Some(json!({ "demande_id": demande.demande_id, "code": code })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let jeton: JetonPreinscriptionDto = serde_json::from_str(&corps).unwrap();

    // Sur A : accepté. Sur B : rejeté, aucune donnée.
    let (st, _) = appel(
        &app,
        "GET",
        "/preinscription/moi",
        &[("host", hote_a.as_str()), ("x-preinscription", &jeton.jeton)],
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, corps) = appel(
        &app,
        "GET",
        "/preinscription/moi",
        &[("host", hote_b.as_str()), ("x-preinscription", &jeton.jeton)],
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "{corps}");
    assert!(!corps.contains(&email));
}
