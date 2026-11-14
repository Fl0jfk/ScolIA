//! Socle 2, lot S2-9 : routes HTTP des finances (base locale de labo).
//! Codes de retour, droits (moteur D-017), isolation entre établissements,
//! espace famille (IBAN masqué), paiement en ligne désactivé, fichier SEPA.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use ent_types::finances::{
    ArticleDto, EspaceFamilleFinancesDto, EtatPaiementEnLigneDto, ParametresFinancesDto,
    RapportGenerationDto,
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
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, String) {
    let mut b = Request::builder().method(methode).uri(uri);
    if let Some(t) = jeton {
        b = b.header("authorization", format!("Bearer {t}"));
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

#[tokio::test]
async fn finances_http_droits_isolation_famille() {
    let Some(state) = etat().await else {
        eprintln!("skip S2-9 HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = state.ent.clone();
    let cast = ent.seed_labo_cast().await.expect("cast");
    let autre = ent.seed_labo_cast().await.expect("cast B");
    let app = app(state);
    let compta = cast.compta.token.as_str();
    let direction = cast.direction.token.as_str();
    let prof = cast.prof.token.as_str();
    let parent = cast.parent_lea.token.as_str();

    // Sans session : 401 ; prof : 403 ; comptabilité : 200.
    assert_eq!(
        appel(&app, "GET", "/finances/parametres", None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        appel(&app, "GET", "/finances/parametres", Some(prof), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let (st, corps) = appel(&app, "GET", "/finances/parametres", Some(compta), None).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let p: ParametresFinancesDto = serde_json::from_str(&corps).unwrap();
    assert_eq!((p.mois_debut, p.nb_mois, p.paliers.len()), (9, 10, 3));

    // Paramètres : IBAN du créancier jamais renvoyé en clair.
    let (st, corps) = appel(
        &app,
        "PUT",
        "/finances/parametres",
        Some(direction),
        Some(
            json!({ "crediteur_iban": "FR1420041010050500013M02606", "jours_prenotification": 10 }),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    assert!(
        !corps.contains("20041010050500013"),
        "IBAN masqué : {corps}"
    );
    let (st, _) = appel(
        &app,
        "PUT",
        "/finances/parametres",
        Some(compta),
        Some(json!({ "jour_echeance": 31 })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Article : 201, doublon 409, catégorie inconnue 400, prof 403.
    let art = json!({ "code": "SCOL", "libelle": "Scolarité", "categorie": "scolarite" });
    let (st, corps) = appel(
        &app,
        "POST",
        "/finances/articles",
        Some(compta),
        Some(art.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{corps}");
    let a: ArticleDto = serde_json::from_str(&corps).unwrap();
    assert_eq!(
        appel(
            &app,
            "POST",
            "/finances/articles",
            Some(compta),
            Some(art.clone())
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            "/finances/articles",
            Some(compta),
            Some(json!({ "code": "X", "libelle": "X", "categorie": "luxe" }))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        appel(&app, "POST", "/finances/articles", Some(prof), Some(art))
            .await
            .0,
        StatusCode::FORBIDDEN
    );

    // Tarif : 201 ; année mal formée : 400 ; article d'un autre établissement : 404.
    let tarif = json!({ "annee_scolaire": "2030-2031", "article_id": a.id, "montant_annuel_centimes": 100000 });
    assert_eq!(
        appel(&app, "POST", "/finances/tarifs", Some(compta), Some(tarif))
            .await
            .0,
        StatusCode::CREATED
    );
    assert_eq!(
        appel(&app, "POST", "/finances/tarifs", Some(compta), Some(json!({ "annee_scolaire": "2030", "article_id": a.id, "montant_annuel_centimes": 1 }))).await.0,
        StatusCode::BAD_REQUEST
    );
    let compta_b = autre.compta.token.as_str();
    assert_eq!(
        appel(&app, "POST", "/finances/tarifs", Some(compta_b), Some(json!({ "annee_scolaire": "2030-2031", "article_id": a.id, "montant_annuel_centimes": 1 }))).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, corps) = appel(&app, "GET", "/finances/articles", Some(compta_b), None).await;
    assert!(
        !corps.contains(&a.id.to_string()),
        "articles de A invisibles depuis B"
    );

    // Génération (aperçu) : 200, rien d'écrit.
    let (st, corps) = appel(
        &app,
        "POST",
        "/finances/generation",
        Some(compta),
        Some(json!({ "annee_scolaire": "2030-2031", "apercu": true })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let g: RapportGenerationDto = serde_json::from_str(&corps).unwrap();
    assert!(g.apercu);

    // Facture inconnue : 404 ; préparation trop proche : 400 ; export sans prélèvement : 409.
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/finances/factures/{}", Uuid::new_v4()),
            Some(compta),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let demain = (domain_eleve::horloge::jour_paris() + chrono::Duration::days(1)).to_string();
    assert_eq!(
        appel(
            &app,
            "POST",
            "/finances/prelevements/preparer",
            Some(compta),
            Some(json!({ "date_execution": demain }))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    // pain.008 sans créancier complet : 400 ; CSV sans prélèvement pré-notifié : 409.
    assert_eq!(
        appel(
            &app,
            "POST",
            "/finances/prelevements/export",
            Some(compta),
            Some(json!({ "date_execution": "2030-09-30" }))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            "/finances/prelevements/export",
            Some(compta),
            Some(json!({ "date_execution": "2030-09-30", "format": "csv" }))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // Rejet d'un prélèvement inconnu : 404 ; code inconnu : 400.
    let inconnu = format!("/finances/prelevements/{}/rejet", Uuid::new_v4());
    assert_eq!(
        appel(
            &app,
            "POST",
            &inconnu,
            Some(compta),
            Some(json!({ "code": "AM04" }))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &inconnu,
            Some(compta),
            Some(json!({ "code": "??" }))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    // Relances, exports : lecture 200 ; prof 403.
    assert_eq!(
        appel(&app, "GET", "/finances/relances", Some(compta), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        appel(&app, "GET", "/finances/relances", Some(prof), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            "/finances/relances/proposer",
            Some(prof),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            "/finances/exports-comptables",
            Some(compta),
            None
        )
        .await
        .0,
        StatusCode::OK
    );

    // Espace famille : 200, IBAN masqués, paiement en ligne désactivé (intention : 409).
    let (st, corps) = appel(&app, "GET", "/finances/famille", Some(parent), None).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let fam: EspaceFamilleFinancesDto = serde_json::from_str(&corps).unwrap();
    assert!(fam
        .comptes
        .iter()
        .flat_map(|c| &c.mandats)
        .all(|m| m.iban_masque.contains("••••")));
    assert!(fam
        .comptes
        .iter()
        .flat_map(|c| &c.factures)
        .all(|f| f.statut != "brouillon"));
    let (st, corps) = appel(
        &app,
        "GET",
        "/finances/paiement-en-ligne",
        Some(parent),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let e: EtatPaiementEnLigneDto = serde_json::from_str(&corps).unwrap();
    assert!(!e.actif && e.prestataire.is_none());
    let (st, _) = appel(
        &app,
        "POST",
        "/finances/paiement-en-ligne/intention",
        Some(parent),
        Some(json!({ "facture_id": Uuid::new_v4(), "montant_centimes": 100 })),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    // Un parent n'accède pas aux routes de gestion.
    assert_eq!(
        appel(&app, "GET", "/finances/factures", Some(parent), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/finances/foyers/{}/compte", cast.foyer_lea_id),
            Some(parent),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Régime : direction 200 puis 409 (même régime) ; comptabilité 403 ; autre établissement 404.
    let dir = cast.direction.token.as_str();
    let eleve = cast.eleve_tom_id;
    let regime = json!({ "regime": "interne", "date_effet": "2030-10-01" });
    let uri = format!("/scolarite/eleves/{eleve}/regime");
    assert_eq!(
        appel(&app, "POST", &uri, Some(compta), Some(regime.clone()))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &uri,
            Some(autre.direction.token.as_str()),
            Some(regime.clone())
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (st, corps) = appel(&app, "POST", &uri, Some(dir), Some(regime.clone())).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    assert_eq!(
        appel(&app, "POST", &uri, Some(dir), Some(regime)).await.0,
        StatusCode::CONFLICT
    );
}
