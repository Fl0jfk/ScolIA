//! Socle 6, lot P6 — délégations par objet, intérim et pouvoirs de signature, côté HTTP
//! (base locale). Matrice de droits côté serveur, codes HTTP (201, 401, 403, 404, 409, 422), séparation des établissements.

use super::tests_l3::{appel, etat, inscrire};
use super::*;
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn signature_habilitations_interim_et_delegations_matrice_de_droits() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P6: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let chef = inscrire(&app, etab, "direction").await;
    let sec = inscrire(&app, etab, "administratif").await;
    let prof = inscrire(&app, etab, "prof").await;
    let admin = inscrire(&app, etab, "admin").await;
    let (_, annuaire) = appel(&app, "GET", "/personnels/annuaire", Some(&chef.jeton), None).await;
    let agent_sec = annuaire
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["fonction_code"] == "secretaire_scolarite")
        .map(|l| l["agent_id"].as_str().unwrap().to_string())
        .expect("agent sec");
    let corps = json!({"agent_id": agent_sec, "documents": ["certificat_scolarite"]});
    // 401 ; le professeur et le secrétariat n'habilitent pas (403) ; l'administrateur non plus (il n'est pas le chef).
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/signature/habilitations",
            None,
            Some(corps.clone())
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    for j in [&prof.jeton, &sec.jeton, &admin.jeton] {
        assert_eq!(
            appel(
                &app,
                "POST",
                "/personnels/signature/habilitations",
                Some(j),
                Some(corps.clone())
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    // Le chef habilite (201) ; nature invalide : 422 ; le secrétariat lit SES pouvoirs sans droit ; le professeur n'a pas la liste.
    let (st, h) = appel(
        &app,
        "POST",
        "/personnels/signature/habilitations",
        Some(&chef.jeton),
        Some(corps),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{h}");
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/signature/habilitations",
            Some(&chef.jeton),
            Some(json!({"agent_id": agent_sec, "documents": ["*"]}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (st, m) = appel(
        &app,
        "GET",
        "/personnels/moi/signature",
        Some(&sec.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(m.as_array().unwrap().len(), 1);
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/signature/habilitations",
            Some(&prof.jeton),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (st, l) = appel(
        &app,
        "GET",
        "/personnels/signature/habilitations",
        Some(&chef.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(l.as_array().unwrap().len(), 1);
    // Retrait : 200 puis 409.
    let hid = h["id"].as_str().unwrap();
    let uri = format!("/personnels/signature/habilitations/{hid}/retirer");
    assert_eq!(
        appel(&app, "POST", &uri, Some(&chef.jeton), None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        appel(&app, "POST", &uri, Some(&chef.jeton), None).await.0,
        StatusCode::CONFLICT
    );
    // Un autre établissement ne voit rien et ne retire rien (404).
    let autre = inscrire(&app, Uuid::new_v4(), "direction").await;
    assert_eq!(
        appel(&app, "POST", &uri, Some(&autre.jeton), None).await.0,
        StatusCode::NOT_FOUND
    );
    // Intérim : le professeur ne l'organise pas (403) ; fin obligatoire (422 sans `au`) ; le chef l'organise (201).
    let corps = json!({"delegataire_agent_id": agent_sec, "au": "2999-01-01"});
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/delegations/interim",
            Some(&prof.jeton),
            Some(corps)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/delegations/interim",
            Some(&chef.jeton),
            Some(json!({"delegataire_agent_id": agent_sec}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let au = (chrono::Utc::now() + chrono::Duration::days(5))
        .format("%Y-%m-%d")
        .to_string();
    let (st, d) = appel(
        &app,
        "POST",
        "/personnels/delegations/interim",
        Some(&chef.jeton),
        Some(json!({"delegataire_agent_id": agent_sec, "au": au, "motif_code": "absence"})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{d}");
    assert_eq!(d["objet"], "interim_direction");
    // Révocation (route S2-5) : ferme l'habilitation d'intérim.
    let did = d["id"].as_str().unwrap();
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/delegations/{did}/revoquer"),
            Some(&chef.jeton),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, l) = appel(
        &app,
        "GET",
        "/personnels/delegations",
        Some(&chef.jeton),
        None,
    )
    .await;
    assert_eq!(l.as_array().unwrap()[0]["etat"], "revoquee");
}
