//! Contrat mobile de « Mes autres comptes » (transverse individu) : les trois routes qu'appellent les
//! applis sont jouées contre le **vrai** backend (`ent_api::app`), en Bearer avec l'en-tête des applis,
//! sur un établissement neuf. Les réponses, normalisées (identifiants, dates, adresses tirées au hasard),
//! sont comparées aux fixtures `ent-rust/contrat/mobile/individu/*.json`, que décodent les cœurs
//! Kotlin (`IndividuTest`) et Swift (`IndividuTests`) avec la vraie méthode du client.
//!
//! Dossier à part de `contrat/mobile/` (banc MB2, `tests_contrat_mobile.rs`) pour ne pas toucher les
//! fichiers que modifient en parallèle d'autres branches : à fondre dans le banc une fois celles-ci fusionnées.
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test individu_contrat_mobile -- --test-threads=1
//! Régénérer : `ENT_CONTRAT_ECRIRE=1` devant la même commande.
//!
//! Sans `ENT_DATABASE_URL`, le test est ignoré (message « SKIP »). Aucun envoi réel.
#![cfg(feature = "labo")]

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use ent_web_api::{ApiClient, Connexion};
use serde_json::Value;
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";
const APPLI: &str = "android/1.0.0 (build 1)";

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent (sourcer scripts/env.sh)");
        return None;
    };
    let parsed = ent_db::assert_local_postgres_url(&url)
        .unwrap_or_else(|e| panic!("base de test locale uniquement : {e}"));
    let nom_base = parsed.path().trim_start_matches('/').to_ascii_lowercase();
    assert!(
        ["test", "proto", "lab"]
            .iter()
            .any(|m| nom_base.contains(m)),
        "le nom de la base doit contenir test, proto ou lab (refusé : {nom_base})"
    );
    let (app, _) = ent_web_server::routeur_complet()
        .await
        .expect("démarrage ent-api + web");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    Some(format!("http://{addr}"))
}

/// Un compte neuf de l'établissement : son jeton (Bearer, comme une appli) et son adresse.
async fn inscrire(base: &str, etab: Uuid, role: &str) -> (String, String) {
    let email = format!("contrat-{role}-{}@ent.local", Uuid::new_v4().simple());
    ApiClient::new(base, None)
        .post_json(
            "/auth/register",
            serde_json::json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role}),
        )
        .await
        .unwrap_or_else(|e| panic!("inscription {role} : {e:?}"));
    match ApiClient::new(base, None)
        .connexion(&email, MDP, etab)
        .await
        .unwrap_or_else(|e| panic!("connexion {role} : {e:?}"))
    {
        Connexion::Ouverte(s) => (s.token, email),
        Connexion::PasskeyRequise(_) => panic!("passkey requise pour {role}"),
    }
}

/// Une fiche de responsable, rattachée au compte du jeton (`PUT /dossier/personnes/{id}/compte`).
async fn fiche_du_compte(base: &str, direction: &str, jeton: &str) {
    let (_, me) = appel(base, jeton, "GET", "/auth/me", None).await;
    let user: Value = serde_json::from_str(&me).unwrap();
    let (s, p) = appel(
        base,
        direction,
        "POST",
        "/dossier/personnes",
        Some(serde_json::json!({ "nom": "Contrat", "prenom": "Mobile" })),
    )
    .await;
    assert!(s == 200 || s == 201, "fiche : {s} {p}");
    let id = serde_json::from_str::<Value>(&p).unwrap()["id"].as_str().unwrap().to_string();
    let (s, c) = appel(
        base,
        direction,
        "PUT",
        &format!("/dossier/personnes/{id}/compte"),
        Some(serde_json::json!({ "user_id": user["user_id"] })),
    )
    .await;
    assert_eq!(s, 200, "rattachement : {c}");
}

/// Une requête comme l'envoie une appli : Bearer, `X-ScolIA-Appli`, `Idempotency-Key` sur une écriture.
async fn appel(
    base: &str,
    jeton: &str,
    methode: &str,
    chemin: &str,
    corps: Option<Value>,
) -> (u16, String) {
    let c = reqwest::Client::new();
    let mut r = match methode {
        "GET" => c.get(format!("{base}{chemin}")),
        "POST" => c
            .post(format!("{base}{chemin}"))
            .header("Idempotency-Key", Uuid::new_v4().to_string()),
        "PUT" => c
            .put(format!("{base}{chemin}"))
            .header("Idempotency-Key", Uuid::new_v4().to_string()),
        m => panic!("méthode non jouée : {m}"),
    }
    .bearer_auth(jeton)
    .header("X-ScolIA-Appli", APPLI)
    .header("Accept", "application/json");
    if let Some(b) = corps {
        r = r.json(&b);
    }
    let rep = r.send().await.expect("requête");
    let statut = rep.status().as_u16();
    (statut, rep.text().await.unwrap())
}

/// Remplace ce qui change d'une exécution à l'autre, en gardant la **forme** (type, format des dates).
fn normaliser(v: &mut Value, ids: &mut BTreeMap<String, String>, noms: &[String]) {
    match v {
        Value::Object(o) => o.values_mut().for_each(|x| normaliser(x, ids, noms)),
        Value::Array(a) => a.iter_mut().for_each(|x| normaliser(x, ids, noms)),
        Value::String(s) => {
            if Uuid::parse_str(s).is_ok() {
                let n = ids.len() + 1;
                *s = ids
                    .entry(s.clone())
                    .or_insert_with(|| format!("00000000-0000-4000-8000-{n:012}"))
                    .clone();
            } else if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
                // Valeur fixe ; seule la présence d'une fraction de seconde est gardée (chrono en écrit 3, 6 ou 9
                // chiffres selon la valeur : en garder le nombre rendrait le test instable).
                *s = if s.contains('.') {
                    "2027-03-08T07:15:30.111111Z".to_string()
                } else {
                    "2027-03-08T07:15:30Z".to_string()
                };
            } else if noms.iter().any(|n| s.contains(n.as_str())) {
                *s = "Compte déclarant".to_string();
            }
        }
        _ => {}
    }
}

fn dossier() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../contrat/mobile/individu")
}

/// Compare (ou écrit, `ENT_CONTRAT_ECRIRE=1`) la réponse normalisée à la fixture.
fn verifier(nom: &str, corps: &str, noms: &[String], erreurs: &mut Vec<String>) {
    let mut v: Value = serde_json::from_str(corps)
        .unwrap_or_else(|e| panic!("{nom} : réponse non JSON ({e}) : {corps}"));
    normaliser(&mut v, &mut BTreeMap::new(), noms);
    let chemin = dossier().join(format!("{nom}.json"));
    let texte = serde_json::to_string_pretty(&v).unwrap() + "\n";
    if std::env::var("ENT_CONTRAT_ECRIRE").as_deref() == Ok("1") {
        std::fs::create_dir_all(dossier()).unwrap();
        std::fs::write(&chemin, texte).unwrap();
        return;
    }
    match std::fs::read_to_string(&chemin) {
        Ok(attendu) if attendu == texte => {}
        Ok(attendu) => erreurs.push(format!(
            "{nom} : la réponse du serveur a changé de forme.\n--- fixture\n{attendu}\n+++ serveur\n{texte}"
        )),
        Err(e) => erreurs.push(format!("{nom} : fixture absente ({e})")),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn routes_mes_autres_comptes_de_l_appli() {
    let Some(base) = demarrer().await else { return };
    let etab = Uuid::new_v4();
    // La confirmation exige une fiche par compte : trois responsables, chacun rattaché à sa fiche par la direction.
    let (direction, _) = inscrire(&base, etab, "direction").await;
    let (cible, email_cible) = inscrire(&base, etab, "parent").await;
    let (premier, email_premier) = inscrire(&base, etab, "parent").await;
    let (second, email_second) = inscrire(&base, etab, "parent").await;
    for jeton in [&cible, &premier, &second] {
        fiche_du_compte(&base, &direction, jeton).await;
    }
    // Un compte sans nom s'affiche « Un autre compte » ; si le serveur se mettait à montrer l'adresse (tirée au
    // hasard), elle serait normalisée, et la fixture le montrerait.
    let noms: Vec<String> = [&email_premier, &email_second]
        .iter()
        .map(|e| e.split('@').next().unwrap().to_string())
        .collect();
    let mut erreurs = Vec::new();

    // Déclarer : 202, corps vide (l'appli ne lit rien), même réponse pour une adresse inconnue.
    for email in [email_cible.as_str(), "inconnu@nulle-part.example"] {
        let (s, c) = appel(
            &base,
            &premier,
            "POST",
            "/dossier/individu/declarations",
            Some(serde_json::json!({ "email": email })),
        )
        .await;
        assert_eq!((s, c.as_str()), (202, ""), "déclaration vers {email}");
    }

    // Confirmer : l'identifiant du lien posé.
    let (_, moi) = appel(&base, &cible, "GET", "/dossier/individu/moi", None).await;
    let moi: Value = serde_json::from_str(&moi).unwrap();
    let d1 = moi["declarations_recues"][0]["id"].as_str().unwrap().to_string();
    let (s, c) = appel(
        &base,
        &cible,
        "POST",
        &format!("/dossier/individu/declarations/{d1}/reponse"),
        Some(serde_json::json!({ "confirmer": true })),
    )
    .await;
    assert_eq!(s, 200, "{c}");
    verifier("reponse_confirmer", &c, &noms, &mut erreurs);

    // Une seconde demande reste en attente : « moi » porte un lien et une demande.
    let (s, _) = appel(
        &base,
        &second,
        "POST",
        "/dossier/individu/declarations",
        Some(serde_json::json!({ "email": email_cible })),
    )
    .await;
    assert_eq!(s, 202);
    let (s, c) = appel(&base, &cible, "GET", "/dossier/individu/moi", None).await;
    assert_eq!(s, 200, "{c}");
    let v: Value = serde_json::from_str(&c).unwrap();
    assert_eq!(v["autres_comptes"], 1, "{c}");
    assert_eq!(v["liens"].as_array().unwrap().len(), 1, "{c}");
    assert_eq!(v["declarations_recues"].as_array().unwrap().len(), 1, "{c}");
    verifier("moi", &c, &noms, &mut erreurs);

    // Refuser : `null`.
    let d2 = v["declarations_recues"][0]["id"].as_str().unwrap().to_string();
    let (s, c) = appel(
        &base,
        &cible,
        "POST",
        &format!("/dossier/individu/declarations/{d2}/reponse"),
        Some(serde_json::json!({ "confirmer": false })),
    )
    .await;
    assert_eq!(s, 200, "{c}");
    verifier("reponse_refuser", &c, &noms, &mut erreurs);

    // Le second déclarant, refusé, ne voit ni la demande ni le refus (anti-énumération) : il n'est relié à personne.
    let (_, c) = appel(&base, &second, "GET", "/dossier/individu/moi", None).await;
    let v: Value = serde_json::from_str(&c).unwrap();
    assert_eq!(v["autres_comptes"], 0, "{c}");

    // Chaque route jouée est dans la liste blanche du dossier.
    let routes = std::fs::read_to_string(dossier().join("routes.txt")).expect("routes.txt");
    for r in [
        "GET /dossier/individu/moi",
        "POST /dossier/individu/declarations",
        "POST /dossier/individu/declarations/{id}/reponse",
    ] {
        assert!(routes.lines().any(|l| l.trim() == r), "{r} absente de routes.txt");
    }

    assert!(erreurs.is_empty(), "{}", erreurs.join("\n\n"));
}
