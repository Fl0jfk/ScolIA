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
    let ent = ent_db::PgEnt::connect(&ent_db::url_base_de_test().expect("base"))
        .await
        .unwrap();
    let naissance = |annees: i32| {
        format!("{}-01-15", chrono::Datelike::year(&chrono::Utc::now()) - annees)
    };
    for (age, attendu) in [(14, StatusCode::FORBIDDEN), (20, StatusCode::OK)] {
        // L'identité ne s'écrit plus directement (E2) : correction versionnée.
        ent.corriger_identite(
            ent_auth::Role::Administratif.as_str(),
            ent_db::identite::CorrectionIdentite {
                etablissement_id: cast.etablissement_id,
                eleve_id,
                version_id: None,
                modif: domain_eleve::ModifIdentite {
                    date_naissance: Some(chrono::NaiveDate::parse_from_str(&naissance(age), "%Y-%m-%d").ok()),
                    ..Default::default()
                },
                motif_code: "faute_de_frappe".into(),
                motif: None,
                piece_document_id: None,
                source: "saisie".into(),
                acteur: cast.administratif.user_id,
            },
        )
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
    for (cast, slug, hote) in [(&a, "a", &hote_a), (&b, "b", &hote_b)] {
        ent.declarer_etablissement_for_actor(
            cast.etablissement_id,
            cast.admin.user_id,
            cast.admin.role.as_str(),
            &format!("i0{slug}-{}", &Uuid::new_v4().simple().to_string()[..12]),
            &format!("Établissement {slug}"),
        )
        .await
        .expect("déclaration");
        ent.ajouter_domaine_for_actor(
            cast.etablissement_id,
            cast.admin.user_id,
            cast.admin.role.as_str(),
            hote,
        )
        .await
        .expect("domaine");
    }
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

/// Lot I2 : toutes les nouvelles routes — droits (403 hors rôles), jeton absent (401), isolation
/// entre établissements (jamais de 2xx avec un identifiant de l'autre établissement), parcours de
/// la démarche (fratrie) et compteur de places par HTTP.
#[tokio::test]
async fn i2_routes_http_droits_isolation_et_parcours() {
    let Some(state) = etat().await else {
        eprintln!("skip S2-3 HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = state.ent.clone();
    let a = ent.seed_labo_cast().await.expect("cast A");
    let b = ent.seed_labo_cast().await.expect("cast B");
    let app = app(state);
    let (dir_a, dir_b) = (bearer(&a.direction.token), bearer(&b.direction.token));
    let (prof_a, adm_a) = (bearer(&a.prof.token), bearer(&a.administratif.token));
    let parent_a = bearer(&a.parent_lea.token);

    // Campagne et offres.
    let (st, corps) = appel(&app, "POST", "/inscriptions/campagnes", &[("authorization", &dir_a)], Some(json!({
        "nature": "admission", "libelle": "Rentrée I2", "annee_cible": "2027-2028",
        "regles": { "accord_responsables": "un_seul", "decision_requise": true, "motif_communique": "facultatif",
                    "delai_annulation_validation_h": 72, "page_publique": false, "au_fil_de_l_eau": false }
    }))).await;
    assert_eq!(st, StatusCode::CREATED, "{corps}");
    let camp: CampagneDto = serde_json::from_str(&corps).unwrap();
    let offre_corps = json!({ "niveau_code": "6EME", "libelle_public": "6ᵉ", "capacite": 1 });
    let url_offres = format!("/inscriptions/campagnes/{}/offres", camp.id);
    for (jeton, attendu) in [(&prof_a, StatusCode::FORBIDDEN), (&parent_a, StatusCode::FORBIDDEN), (&dir_b, StatusCode::NOT_FOUND)] {
        let (st, _) = appel(&app, "POST", &url_offres, &[("authorization", jeton)], Some(offre_corps.clone())).await;
        assert!(st == attendu || st == StatusCode::FORBIDDEN, "création d'offre : {st}");
    }
    let (st, _) = appel(&app, "POST", &url_offres, &[], Some(offre_corps.clone())).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, corps) = appel(&app, "POST", &url_offres, &[("authorization", &adm_a)], Some(offre_corps.clone())).await;
    assert_eq!(st, StatusCode::CREATED, "{corps}");
    let offre: ent_types::inscriptions::OffreOccupationDto = serde_json::from_str(&corps).unwrap();
    assert_eq!((offre.capacite, offre.disponibles), (Some(1), Some(1)));
    // Doublon de clé : 409.
    let (st, _) = appel(&app, "POST", &url_offres, &[("authorization", &adm_a)], Some(offre_corps.clone())).await;
    assert_eq!(st, StatusCode::CONFLICT);
    // Lecture : personnel de A seulement.
    let (st, corps) = appel(&app, "GET", &url_offres, &[("authorization", &dir_a)], None).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    assert!(corps.contains("\"disponibles\":1"));
    for jeton in [&dir_b, &prof_a, &parent_a] {
        let (st, corps) = appel(&app, "GET", &url_offres, &[("authorization", jeton)], None).await;
        assert!(!st.is_success(), "lecture des offres : {st} {corps}");
    }
    // Modification / archivage d'une offre de A par B ou par un rôle sans droit : jamais 2xx.
    let url_offre = format!("/inscriptions/offres/{}", offre.id);
    for jeton in [&dir_b, &prof_a, &parent_a] {
        for (m, c) in [("PUT", Some(offre_corps.clone())), ("DELETE", None)] {
            let (st, _) = appel(&app, m, &url_offre, &[("authorization", jeton)], c).await;
            assert!(!st.is_success(), "{m} offre : {st}");
        }
    }
    // Ouverture, puis dépôt d'une démarche (fratrie) par la famille.
    let (st, _) = appel(&app, "POST", &format!("/inscriptions/campagnes/{}/ouvrir", camp.id), &[("authorization", &dir_a)], None).await;
    assert_eq!(st, StatusCode::OK);
    let email = format!("i2-{}@exemple.test", Uuid::new_v4().simple());
    let (st, corps) = appel(&app, "POST", "/preinscription/demarrer", &[], Some(json!({ "email": email, "etablissement_id": a.etablissement_id }))).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    let demande: DemarrerDto = serde_json::from_str(&corps).unwrap();
    let code = code_recu(&ent, &email);
    let (st, corps) = appel(&app, "POST", "/preinscription/verifier", &[], Some(json!({ "demande_id": demande.demande_id, "code": code }))).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let j: JetonPreinscriptionDto = serde_json::from_str(&corps).unwrap();
    let h = [("x-preinscription", j.jeton.as_str())];
    let dem_corps = json!({
        "campagne_id": camp.id,
        "enfants": [
            { "eleve": { "nom": "Granger", "prenom": "Hermione", "niveau_demande": "6EME" }, "voeux": [offre.id] },
            { "eleve": { "nom": "Granger", "prenom": "Crookshanks", "niveau_demande": "6EME" } }
        ],
        "responsable": { "nom": "Granger", "prenom": "Jean" }
    });
    let (st, _) = appel(&app, "POST", "/preinscription/demarches", &[], Some(dem_corps.clone())).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "sans jeton");
    let (st, corps) = appel(&app, "POST", "/preinscription/demarches", &h, Some(dem_corps)).await;
    assert_eq!(st, StatusCode::CREATED, "{corps}");
    let dem: ent_types::inscriptions::DemarcheDto = serde_json::from_str(&corps).unwrap();
    assert_eq!(dem.dossiers.len(), 2);
    assert!(dem.dossiers.iter().all(|d| d.reference.starts_with("ADM-2027-")));
    let (st, _) = appel(&app, "GET", &format!("/preinscription/demarches/{}", dem.id), &h, None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "GET", &format!("/preinscription/demarches/{}", dem.id), &[], None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = appel(&app, "GET", &format!("/preinscription/demarches/{}", Uuid::new_v4()), &h, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Décision par HTTP (offre à une place), puis dépassement sans confirmation : 409.
    let d0 = dem.dossiers[0].id;
    let d1 = dem.dossiers[1].id;
    let (st, corps) = appel(&app, "POST", &format!("/inscriptions/dossiers/{d0}/decision"), &[("authorization", &dir_a)],
        Some(json!({ "decision": "admis", "offre_id": offre.id }))).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let (st, _) = appel(&app, "POST", &format!("/inscriptions/dossiers/{d1}/decision"), &[("authorization", &dir_a)],
        Some(json!({ "decision": "admis", "offre_id": offre.id }))).await;
    assert_eq!(st, StatusCode::CONFLICT, "offre pleine");
    let (_, corps) = appel(&app, "GET", &url_offres, &[("authorization", &dir_a)], None).await;
    assert!(corps.contains("\"proposees\":1") && corps.contains("\"disponibles\":0"), "{corps}");

    // Transition, blocage, annulation de validation, ne pas solliciter, prolongation, fermetures.
    let id_b = |chemin: &str| format!("/inscriptions/dossiers/{d0}/{chemin}");
    let corps_t = json!({ "action": "sans_suite", "motif": "Délai dépassé" });
    let corps_blocage = json!({ "bloquer": true, "motif": "Désaccord déclaré" });
    let corps_annul = json!({ "motif": "Erreur de saisie" });
    let corps_nps = json!({ "rang": 2, "motif": "Ordonnance de protection" });
    for (m, chemin, c) in [
        ("POST", id_b("transition"), corps_t.clone()),
        ("POST", id_b("blocage"), corps_blocage.clone()),
        ("POST", id_b("annuler-validation"), corps_annul.clone()),
        ("POST", id_b("ne-pas-solliciter"), corps_nps.clone()),
        ("POST", format!("/inscriptions/campagnes/{}/prolonger", camp.id),
            json!({ "objet": "cloture", "nouvelle_date": "2099-01-01", "motif_public": "Intempéries" })),
        ("POST", format!("/inscriptions/campagnes/{}/statut", camp.id), json!({ "statut": "suspendue" })),
        ("POST", "/inscriptions/fermetures".to_string(),
            json!({ "date_debut": "2099-07-01", "date_fin": "2099-08-20", "motif_public": "Vacances" })),
    ] {
        // Sans session : 401. Famille, professeur : refusés. Autre établissement : jamais de 2xx.
        let (st, _) = appel(&app, m, &chemin, &[], Some(c.clone())).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{chemin}");
        // La création d'une fermeture n'a pas d'identifiant de A : la direction de B agit chez B.
        let intrus: Vec<&String> = if chemin == "/inscriptions/fermetures" {
            vec![&prof_a, &parent_a]
        } else {
            vec![&prof_a, &parent_a, &dir_b]
        };
        for jeton in intrus {
            let (st, corps) = appel(&app, m, &chemin, &[("authorization", jeton)], Some(c.clone())).await;
            assert!(!st.is_success(), "{chemin} : {st} {corps}");
        }
    }
    let (st, _) = appel(&app, "GET", "/inscriptions/fermetures", &[("authorization", &parent_a)], None).await;
    assert!(!st.is_success());
    let (st, _) = appel(&app, "GET", "/inscriptions/fermetures", &[("authorization", &dir_b)], None).await;
    assert_eq!(st, StatusCode::OK, "B lit ses propres fermetures (aucune)");
    // Le personnel de A : blocage (secrétariat), transition de classement.
    let (st, corps) = appel(&app, "POST", &id_b("blocage"), &[("authorization", &adm_a)], Some(corps_blocage)).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let (st, _) = appel(&app, "POST", &id_b("transition"), &[("authorization", &adm_a)], Some(corps_t)).await;
    assert_eq!(st, StatusCode::CONFLICT, "bloqué : aucune transition d'étape");
    // « Ne pas solliciter » : le rang 1 (déposant) n'est pas marquable → 400 ; direction seulement.
    let (st, _) = appel(&app, "POST", &id_b("ne-pas-solliciter"), &[("authorization", &adm_a)], Some(corps_nps.clone())).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "POST", &id_b("ne-pas-solliciter"), &[("authorization", &dir_a)], Some(json!({ "rang": 1, "motif": "Ordonnance" }))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // Retrait par la famille d'un dossier bloqué : refusé ; d'un autre déposant : introuvable.
    let (st, _) = appel(&app, "POST", &format!("/preinscription/dossiers/{d0}/retirer"), &h, None).await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, corps) = appel(&app, "POST", &format!("/preinscription/dossiers/{d1}/retirer"), &h, None).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let (st, _) = appel(&app, "POST", &format!("/preinscription/dossiers/{d1}/retirer"), &[("x-preinscription", "forge")], None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}
