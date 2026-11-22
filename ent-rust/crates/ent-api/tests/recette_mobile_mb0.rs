//! Socle 11, lot MB0 : séparation HTTP des correctifs mobiles (DS1 à DS4, MEN-11), contre le
//! routeur réel et le service réel (PostgreSQL local de test).
//!
//! Lancement : voir `recette_securite_3.rs` (mêmes variables ; sans base : SKIP).

#![allow(dead_code)]

#[path = "separation/harness.rs"]
mod harness;

use axum::http::StatusCode;
use harness::*;
use serde_json::json;

use Acteur::*;

macro_rules! monde_ou_skip {
    () => {
        match monde() {
            Some(m) => m,
            None => return eprintln!("SKIP : pas de base de test"),
        }
    };
}

/// MEN-11 : un client qui écrit `"client":"mobile"` avec un mot de passe reçoit la durée web.
#[test]
fn mobile_declare_avec_mot_de_passe_recoit_la_duree_web() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let email = m.a.emails.get(&Prof).expect("prof");
        let l = m
            .appel(
                "POST",
                "/auth/login",
                None,
                Some(&json!({
                    "email": email, "password": m.a.mot_de_passe,
                    "etablissement_id": m.a.objets.etab, "client": "mobile"
                })),
            )
            .await;
        assert_eq!(l.statut, StatusCode::OK, "{}", l.texte);
        // F-7 : durée accordée calculée côté SQL (écart entre deux colonnes de la ligne), sans heure réelle.
        let url =
            std::env::var("ENT_DATABASE_URL").expect("ENT_DATABASE_URL (la base de test du monde)");
        let pool = sqlx::PgPool::connect(&url)
            .await
            .expect("connexion de vérification");
        let (duree_max_13h,): (bool,) = sqlx::query_as(
            "SELECT s.expires_at - s.created_at <= interval '13 hours'
               FROM ent_session s JOIN ent_user u ON u.id = s.user_id
              WHERE u.email = $1 ORDER BY s.created_at DESC LIMIT 1",
        )
        .bind(email)
        .fetch_one(&pool)
        .await
        .expect("session ouverte");
        assert!(duree_max_13h, "session longue sans appareil rattaché");
        let jeton = l.json["token"].as_str().expect("jeton");
        let moi = m.appel("GET", "/auth/me", Some(jeton), None).await;
        assert_eq!(moi.statut, StatusCode::OK);
        assert_eq!(
            moi.json["client"], "web",
            "le client déclaré n'est jamais retenu sans preuve"
        );
    });
}

/// DS3 / DS4 : la réponse ne renvoie pas le jeton ; la désinscription est réservée au propriétaire ;
/// DS1 : la déconnexion coupe l'inscription.
#[test]
fn inscription_push_sans_jeton_en_reponse_et_desinscription() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let secret = format!("jeton-secret-{}", uuid::Uuid::new_v4());
        let reg = m
            .appel(
                "POST",
                "/devices/push",
                Some(m.jeton(Prof)),
                Some(&json!({"platform": "android", "token": secret})),
            )
            .await;
        assert_eq!(reg.statut, StatusCode::CREATED, "{}", reg.texte);
        assert!(
            !reg.texte.contains(&secret),
            "le jeton ne doit jamais être renvoyé : {}",
            reg.texte
        );
        let id = reg.json["id"].as_str().expect("id").to_string();
        // Un autre acteur ne peut pas désinscrire cet appareil.
        let autre = m
            .appel(
                "DELETE",
                &format!("/devices/push/{id}"),
                Some(m.jeton(Parent)),
                None,
            )
            .await;
        assert_eq!(autre.statut, StatusCode::NOT_FOUND);
        let sans = m
            .appel("DELETE", &format!("/devices/push/{id}"), None, None)
            .await;
        assert!(sans.est_refus());
        // Le propriétaire désinscrit ; une seconde fois : introuvable.
        let ok = m
            .appel(
                "DELETE",
                &format!("/devices/push/{id}"),
                Some(m.jeton(Prof)),
                None,
            )
            .await;
        assert_eq!(ok.statut, StatusCode::NO_CONTENT);
        let encore = m
            .appel(
                "DELETE",
                &format!("/devices/push/{id}"),
                Some(m.jeton(Prof)),
                None,
            )
            .await;
        assert_eq!(encore.statut, StatusCode::NOT_FOUND);
    });
}

/// F-3 : le contrôle de session précède la validation du corps. Une session révoquée donne 401 même avec
/// un corps invalide (plateforme inconnue, jeton vide) ; avec une session vivante, le corps invalide garde
/// son erreur (jamais 500).
#[test]
fn session_revoquee_donne_401_avant_la_validation_du_corps() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let email = m.a.emails.get(&Prof).expect("prof");
        let l = m
            .appel(
                "POST",
                "/auth/login",
                None,
                Some(&json!({"email": email, "password": m.a.mot_de_passe, "etablissement_id": m.a.objets.etab})),
            )
            .await;
        assert_eq!(l.statut, StatusCode::OK, "{}", l.texte);
        let jeton = l.json["token"].as_str().expect("jeton").to_string();
        for corps in [
            json!({"platform": "inconnue", "token": "abc"}),
            json!({"platform": "android", "token": ""}),
            json!({"platform": "inconnue", "token": ""}),
        ] {
            let vivante = m.appel("POST", "/devices/push", Some(&jeton), Some(&corps)).await;
            assert!(
                vivante.statut.is_client_error() && vivante.statut != StatusCode::UNAUTHORIZED,
                "session vivante, corps invalide : {} {}",
                vivante.statut,
                vivante.texte
            );
        }
        let sortie = m.appel("POST", "/auth/logout", Some(&jeton), None).await;
        assert!(sortie.est_succes(), "{}", sortie.texte);
        for corps in [
            json!({"platform": "inconnue", "token": "abc"}),
            json!({"platform": "android", "token": ""}),
            json!({"platform": "inconnue", "token": ""}),
        ] {
            let morte = m.appel("POST", "/devices/push", Some(&jeton), Some(&corps)).await;
            assert_eq!(morte.statut, StatusCode::UNAUTHORIZED, "session morte : {}", morte.texte);
        }
    });
}
