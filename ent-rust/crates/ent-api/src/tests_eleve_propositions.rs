//! Socle 2 (l'élève), lot E5 — propositions et paramétrage : parcours HTTP (base locale).
//! Établissement neuf par test : aucun réglage ne déborde sur les autres suites.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use domain_eleve::CreateEleve;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";

async fn appel(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: &str,
    corps: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(methode)
        .uri(uri)
        .header("authorization", format!("Bearer {jeton}"));
    let corps = match corps {
        Some(c) => {
            b = b.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&c).unwrap())
        }
        None => Body::empty(),
    };
    let r = app.clone().oneshot(b.body(corps).unwrap()).await.unwrap();
    let statut = r.status();
    let octets = http_body_util::BodyExt::collect(r.into_body()).await.unwrap().to_bytes();
    let v = serde_json::from_slice(&octets).unwrap_or(Value::Null);
    (statut, v)
}

async fn inscrire(app: &axum::Router, etab: Uuid, role: &str) -> (String, Uuid) {
    let email = format!("e5-{role}-{}@ent.local", Uuid::new_v4().simple());
    let post = |uri: &str, corps: Value| {
        let req = Request::builder()
            .method("POST")
            .uri(uri.to_string())
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&corps).unwrap()))
            .unwrap();
        app.clone().oneshot(req)
    };
    let r = post(
        "/auth/register",
        json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role}),
    )
    .await
    .unwrap();
    assert_eq!(r.status(), StatusCode::CREATED, "register {role}");
    let r = post(
        "/auth/login",
        json!({"email": email, "password": MDP, "etablissement_id": etab}),
    )
    .await
    .unwrap();
    assert_eq!(r.status(), StatusCode::OK, "login {role}");
    let octets = http_body_util::BodyExt::collect(r.into_body()).await.unwrap().to_bytes();
    let b: Value = serde_json::from_slice(&octets).unwrap();
    (
        b["token"].as_str().unwrap().to_string(),
        Uuid::parse_str(b["user_id"].as_str().unwrap()).unwrap(),
    )
}

#[tokio::test]
async fn e5_parcours_http_parametrage_signalement_et_decision() {
    let Some(url) = std::env::var("ENT_DATABASE_URL").ok().filter(|u| !u.trim().is_empty()) else {
        eprintln!("skip E5 HTTP: ENT_DATABASE_URL");
        return;
    };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let state = AppState::nouveau(ent);
    let ent = state.ent.clone();
    let app = app(state);
    let etab = Uuid::new_v4();
    let (admin, _) = inscrire(&app, etab, "admin").await;
    let (adm, _) = inscrire(&app, etab, "administratif").await;
    let (prof, _) = inscrire(&app, etab, "prof").await;
    let (cpe, _) = inscrire(&app, etab, "cpe").await;
    let (parent, _) = inscrire(&app, etab, "parent").await;

    let eleve = ent
        .create_eleve(CreateEleve {
            etablissement_id: etab,
            nom: "Dupont".into(),
            prenom: "Zélie".into(),
            ine: None,
            sexe: Some("F".into()),
            date_naissance: chrono::NaiveDate::from_ymd_opt(2011, 5, 3),
            numero_badge: None,
            redoublant: false,
            classe: Some("6A".into()),
            regime: None,
            mef: None,
            site: None,
            date_entree: None,
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: None,
        })
        .await
        .unwrap();
    let eid = eleve.id;

    // Paramétrage : l'administrateur seulement (eleves.parametrer).
    let (s, v) = appel(&app, "GET", "/eleves/parametrage", &admin, None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert!(v["profil"].is_null());
    assert_eq!(v["champs"].as_array().unwrap().len(), 13);
    assert_eq!(v["profils_proposes"].as_array().unwrap().len(), 5);
    for (jeton, nom) in [(&adm, "administratif"), (&prof, "prof"), (&parent, "parent")] {
        let (s, _) = appel(&app, "GET", "/eleves/parametrage", jeton, None).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{nom}");
        let (s, _) = appel(&app, "POST", "/eleves/parametrage/profil", jeton, Some(json!({"profil": "public"}))).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{nom}");
    }
    // Entrées invalides : 400 ; bornes du logiciel : 400.
    let (s, _) = appel(&app, "POST", "/eleves/parametrage/profil", &admin, Some(json!({"profil": "inconnu"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = appel(&app, "PUT", "/eleves/parametrage/champs/ine", &admin, Some(json!({"saisie_famille": "proposer"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "l'INE n'est jamais proposable par la famille");
    let (s, _) = appel(&app, "PUT", "/eleves/parametrage/champs/nom_naissance", &admin, Some(json!({"modification_locale": "n_importe_quoi"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Profil public : SIECLE fait foi, l'état civil n'est plus modifiable localement.
    let (s, v) = appel(&app, "POST", "/eleves/parametrage/profil", &admin, Some(json!({"profil": "public"}))).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["profil"], "public");
    let nom = v["champs"].as_array().unwrap().iter().find(|c| c["code"] == "nom_naissance").unwrap().clone();
    assert_eq!(nom["source_qui_fait_foi"], "siecle");
    assert_eq!(nom["modification_locale"], "interdite");
    let (s, v) = appel(
        &app,
        "POST",
        &format!("/eleves/{eid}/identite/corrections"),
        &adm,
        Some(json!({"champs": {"nom_naissance": "Durand"}, "motif_code": "faute_de_frappe"})),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "correction directe interdite sous SIECLE : {v}");

    // Signalement du personnel : une proposition, jamais une modification. Un professeur
    // ne signale que les élèves de ses classes et groupes (D-022) : celui-ci n'en fait pas partie.
    let (s, _) = appel(&app, "POST", &format!("/eleves/{eid}/propositions"), &prof, Some(json!({"champs": {"nom_naissance": "Dupond"}}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "professeur hors de ses classes");
    let (s, v) = appel(
        &app,
        "POST",
        &format!("/eleves/{eid}/propositions"),
        &cpe,
        Some(json!({"champs": {"nom_naissance": "Dupond"}})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{v}");
    let pid = v["en_attente"]["id"].as_str().unwrap().to_string();
    assert_eq!(v["en_attente"]["source"], "personnel");
    // Le dépôt ne donne aucun droit de lecture : le CPE (sans `eleves.propositions.traiter`)
    // ne voit pas la valeur de la fiche dans la réponse.
    assert!(v["en_attente"]["champs"][0]["actuel"].is_null(), "{v}");
    assert!(v["raisons_non_auto"].as_array().unwrap().is_empty());
    // Le secrétariat, lui, voit la valeur actuelle.
    let (s, v) = appel(&app, "GET", &format!("/eleves/propositions/{pid}"), &adm, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["champs"][0]["actuel"], "Dupont");
    // Champ inconnu / lot vide : 400.
    let (s, _) = appel(&app, "POST", &format!("/eleves/{eid}/propositions"), &cpe, Some(json!({"champs": {"inconnu": "x"}}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = appel(&app, "POST", &format!("/eleves/{eid}/propositions"), &cpe, Some(json!({"champs": {}}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // File : le secrétariat la voit ; le professeur et le parent non.
    let (s, v) = appel(&app, "GET", "/eleves/propositions?statut=en_attente", &adm, None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(v.as_array().unwrap().iter().any(|p| p["id"] == pid.as_str()));
    for jeton in [&prof, &cpe, &parent] {
        let (s, _) = appel(&app, "GET", "/eleves/propositions", jeton, None).await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        let (s, _) = appel(&app, "GET", &format!("/eleves/propositions/{pid}"), jeton, None).await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        let (s, _) = appel(&app, "POST", &format!("/eleves/propositions/{pid}/decision"), jeton, Some(json!({"champs_acceptes": ["nom_naissance"]}))).await;
        assert_eq!(s, StatusCode::FORBIDDEN);
    }
    // Un parent sans lien actif ne dépose ni ne lit rien « en famille ».
    let (s, _) = appel(&app, "POST", &format!("/eleves/{eid}/propositions"), &parent, Some(json!({"champs": {"nom_naissance": "X"}}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = appel(&app, "GET", &format!("/eleves/{eid}/propositions"), &parent, None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = appel(&app, "GET", &format!("/eleves/{eid}/propositions/champs"), &parent, None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    // « import » n'est pas une nature qu'une personne choisit.
    let (s, _) = appel(&app, "POST", &format!("/eleves/propositions/{pid}/decision"), &adm, Some(json!({"champs_acceptes": ["nom_naissance"], "nature": "import"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    // Accepter un champ géré par SIECLE : refusé ; refuser avec le motif codé : accepté.
    let (s, v) = appel(
        &app,
        "POST",
        &format!("/eleves/propositions/{pid}/decision"),
        &adm,
        Some(json!({"champs_acceptes": ["nom_naissance"], "nature": "correction", "motif_code": "d_apres_piece"})),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "{v}");
    let (s, v) = appel(
        &app,
        "POST",
        &format!("/eleves/propositions/{pid}/decision"),
        &adm,
        Some(json!({"champs_acceptes": [], "refus_code": "a_corriger_dans_la_source"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["statut"], "refusee");
    assert_eq!(v["decision_code"], "a_corriger_dans_la_source");
    let (s, _) = appel(&app, "POST", &format!("/eleves/propositions/{pid}/decision"), &adm, Some(json!({"champs_acceptes": [], "refus_code": "autre", "refus_motif": "x"}))).await;
    assert_eq!(s, StatusCode::CONFLICT, "déjà décidée");

    // Import SIECLE : règle explicite du profil → appliqué automatiquement, visible en file.
    let r = ent
        .deposer_proposition_systeme(ent_db::propositions::DepotProposition {
            etablissement_id: etab,
            eleve_id: eid,
            cible: domain_eleve::CibleProposition::Identite,
            source: domain_eleve::SourceProposition::Import,
            origine: Some(domain_eleve::OrigineProposition::Siecle),
            source_ref: Some(Uuid::new_v4()),
            champs: vec![("nom_naissance".into(), json!("Dupond"))],
            piece_document_id: None,
            commentaire: None,
            expire_le: None,
            depose_par: None,
        })
        .await
        .unwrap();
    assert!(r.appliquee_auto.is_some());
    let (s, v) = appel(&app, "GET", &format!("/eleves/propositions/{}", r.appliquee_auto.unwrap().entete.id), &adm, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["statut"], "appliquee_auto");
    assert_eq!(v["regle_auto"], "regle_auto:import");

    // Règle automatique : retirée par l'administrateur, elle ne s'applique plus.
    let (s, _) = appel(&app, "PUT", "/eleves/parametrage/regles-auto", &admin, Some(json!({"champ": "nom_naissance", "source": "import", "actif": false}))).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = appel(&app, "PUT", "/eleves/parametrage/regles-auto", &admin, Some(json!({"champ": "prenom_usage", "source": "import", "actif": true}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "un champ sensible n'a jamais de règle automatique");
    let (s, _) = appel(&app, "PUT", "/eleves/parametrage/regles-auto", &adm, Some(json!({"champ": "nom_naissance", "source": "import", "actif": true}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}
