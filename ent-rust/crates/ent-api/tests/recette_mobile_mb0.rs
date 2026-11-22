//! Socle 11, lot MB0 : séparation HTTP des correctifs mobiles (DS1 à DS4, MEN-11), contre le
//! routeur réel et le service réel (PostgreSQL local de test).
//!
//! Lancement : voir `recette_securite_3.rs` (mêmes variables ; sans base : SKIP).

#![allow(dead_code)]

#[path = "separation/harness.rs"]
mod harness;

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
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
        let fin: DateTime<Utc> = l.json["expires_at"]
            .as_str()
            .expect("expires_at")
            .parse()
            .expect("date");
        assert!(
            fin - Utc::now() <= Duration::hours(13),
            "session longue sans appareil rattaché : {fin}"
        );
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
