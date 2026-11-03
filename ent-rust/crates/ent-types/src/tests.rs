//! Aller-retour JSON sur des exemples au format d'`ent-api`.
//! Le contrôle contre le vrai backend est dans `ent-web-server/tests/contre_backend.rs`.

use crate::appel::*;
use crate::auth::*;
use crate::edt::*;

fn aller_retour<T: serde::Serialize + serde::de::DeserializeOwned>(json: &str) {
    let attendu: serde_json::Value = serde_json::from_str(json).unwrap();
    let t: T = serde_json::from_str(json).unwrap();
    let obtenu = serde_json::to_value(&t).unwrap();
    assert_eq!(obtenu, attendu, "le JSON doit être reproduit à l'identique");
}

#[test]
fn appel_et_lignes() {
    aller_retour::<AppelDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","classe":"4B","jour":"2026-11-02",
            "statut":"ouvert","creneau_id":"6f1c3c80-0000-4000-8000-000000000003","matiere_libelle":"Potions",
            "lignes":[{"eleve_id":"6f1c3c80-0000-4000-8000-000000000002","eleve_label":"Harry Potter","etat":"present"},
                      {"eleve_id":"6f1c3c80-0000-4000-8000-000000000004","etat":"absent"}]}"#,
    );
    // Champs facultatifs à null : conservés tels quels (pas de skip côté ent-api).
    aller_retour::<AppelDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","classe":"4B","jour":"2026-11-02",
            "statut":"clos","creneau_id":null,"matiere_libelle":null,"lignes":[]}"#,
    );
    let a: AppelDto = serde_json::from_str(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","classe":"4B","jour":"2026-11-02",
            "statut":"clos","creneau_id":null,"matiere_libelle":null,"lignes":[]}"#,
    )
    .unwrap();
    assert!(a.est_clos());
    aller_retour::<OuvrirAppelBody>(
        r#"{"creneau_id":"6f1c3c80-0000-4000-8000-000000000003","jour":"2026-11-02"}"#,
    );
    aller_retour::<MarquerAppelLigneBody>(
        r#"{"eleve_id":"6f1c3c80-0000-4000-8000-000000000002","etat":"retard"}"#,
    );
}

#[test]
fn creneau() {
    aller_retour::<EdtCreneauDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000003","classe":"4B","matiere_libelle":"Potions",
            "prof_user_id":"6f1c3c80-0000-4000-8000-000000000005","prof_label":"S. Rogue","jour_semaine":1,
            "heure_debut":"08:00:00","heure_fin":"09:00:00","salle":"Donjons","semaine_ab":"toutes"}"#,
    );
    aller_retour::<EdtCreneauDto>(
        r#"{"id":"6f1c3c80-0000-4000-8000-000000000003","classe":"4B","matiere_libelle":"Potions",
            "prof_user_id":"6f1c3c80-0000-4000-8000-000000000005","jour_semaine":1,
            "heure_debut":"08:00:00","heure_fin":"09:00:00","salle":null,"semaine_ab":"A"}"#,
    );
}

#[test]
fn session_aplatie_et_defi_passkey() {
    aller_retour::<LoginResponseDto>(
        r#"{"mfa_required":false,"token":"t","user_id":"6f1c3c80-0000-4000-8000-000000000001",
            "email":"a@exemple.test","etablissement_id":"6f1c3c80-0000-4000-8000-000000000002",
            "role":"prof","expires_at":"2026-11-03T08:00:00Z"}"#,
    );
    aller_retour::<LoginResponseDto>(
        r#"{"mfa_required":true,"passkey":{"token":"x","options":{"publicKey":{}},"expires_at":"2026-11-03T08:00:00Z"}}"#,
    );
    aller_retour::<MeDto>(
        r#"{"user_id":"6f1c3c80-0000-4000-8000-000000000001","email":"a@exemple.test",
            "etablissement_id":"6f1c3c80-0000-4000-8000-000000000002","role":"eleve",
            "eleve_id":"6f1c3c80-0000-4000-8000-000000000009","passkey_count":0}"#,
    );
}

#[test]
fn secrets_masques_dans_debug() {
    let b = LoginBody {
        email: "a@exemple.test".into(),
        password: "ne-pas-afficher".into(),
        etablissement_id: uuid::Uuid::nil(),
    };
    assert!(!format!("{b:?}").contains("ne-pas-afficher"));
    let s: LoginResponseDto = serde_json::from_str(
        r#"{"mfa_required":false,"token":"jeton-secret","user_id":"6f1c3c80-0000-4000-8000-000000000001",
            "email":"a@exemple.test","etablissement_id":"6f1c3c80-0000-4000-8000-000000000002",
            "role":"prof","expires_at":"2026-11-03T08:00:00Z"}"#,
    )
    .unwrap();
    assert!(!format!("{s:?}").contains("jeton-secret"));
}
