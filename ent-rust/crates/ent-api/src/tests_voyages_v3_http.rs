//! Socle 42, lot V3 « suites » (HTTP) : les routes d'argent de V3 (aides, reports, demandes, internat, retour anticipé,
//! consigne du motif) répondent 201, 202, 403, 404 ou 422 comme le contrat le dit. Aucune règle ici : elles sont dans
//! `ent-db` (tests de `tests_voyages_v3.rs`). Base locale de labo (cast Harry Potter).

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn appel(
    app: &axum::Router,
    m: &str,
    uri: &str,
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(m).uri(uri);
    if let Some(j) = jeton {
        b = b.header("authorization", format!("Bearer {j}"));
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
    let v = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (st, v)
}

fn cle() -> String {
    format!("http-{}", Uuid::new_v4().simple())
}

#[tokio::test]
async fn routes_d_argent_de_v3_repondent_201_202_403_404_422() {
    let Some(url) = super::tests_l3::url_locale() else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let autre = state.ent.seed_labo_cast().await.expect("cast B");
    let etab = cast.etablissement_id;
    let annee = state
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap();
    let classe = state
        .ent
        .referentiel_classe_par_code(etab, annee.id, "4B")
        .await
        .unwrap()
        .expect("classe 4B du cast");
    let app = app(state.clone());
    let (dir, prof, compta, parent) = (
        cast.direction.token.as_str(),
        cast.prof.token.as_str(),
        cast.compta.token.as_str(),
        cast.parent_lea.token.as_str(),
    );
    let b_dir = autre.direction.token.as_str();
    let fantome = Uuid::new_v4();

    // Session obligatoire.
    for (m, u) in [
        ("GET", format!("/voyages/{fantome}/argent/aides")),
        ("POST", format!("/voyages/{fantome}/argent/aides")),
        (
            "POST",
            format!("/voyages/{fantome}/argent/aides/{fantome}/annulation"),
        ),
        ("POST", format!("/voyages/{fantome}/argent/reports")),
        (
            "POST",
            format!("/voyages/{fantome}/argent/reports/{fantome}/annulation"),
        ),
        ("GET", format!("/voyages/{fantome}/argent/demandes")),
        (
            "POST",
            format!("/voyages/{fantome}/argent/demandes/{fantome}/validation"),
        ),
        (
            "POST",
            format!("/voyages/{fantome}/argent/demandes/{fantome}/refus"),
        ),
        ("GET", format!("/voyages/{fantome}/argent/consigne-motif")),
        ("GET", format!("/voyages/{fantome}/argent/contournements")),
        ("POST", format!("/voyages/{fantome}/argent/contournements")),
        (
            "POST",
            format!("/voyages/{fantome}/argent/contournements/{fantome}/decision"),
        ),
        (
            "POST",
            format!("/voyages/{fantome}/argent/contournements/{fantome}/revocation"),
        ),
        ("POST", format!("/voyages/{fantome}/internat/synchroniser")),
        (
            "POST",
            format!("/voyages/{fantome}/participants/{fantome}/retour-anticipe"),
        ),
    ] {
        assert_eq!(
            appel(&app, m, &u, None, None).await.0,
            StatusCode::UNAUTHORIZED,
            "{m} {u}"
        );
    }

    // Une sortie ouverte, deux participants d'un même foyer payeur, une part de 500 € chacun (mise en place en base : le
    // calcul des parts n'est pas l'objet de ce test).
    let debut = chrono::Utc::now() + chrono::Duration::days(3);
    let (st, cree) = appel(
        &app,
        "POST",
        "/voyages",
        Some(prof),
        Some(json!({
            "titre": "Sortie au muséum", "nature": "facultative", "forme": "sans_nuitee",
            "destination": "proximite", "destination_libelle": "Muséum",
            "debut": debut.to_rfc3339(), "fin": (debut + chrono::Duration::hours(6)).to_rfc3339(),
            "classes": [classe.id], "effectif_eleves_declare": 2, "point_rendez_vous": "Grille",
        })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{cree}");
    let id: Uuid = cree["id"].as_str().unwrap().parse().unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query("UPDATE ent_sortie SET etat = 'ouverte' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let compte_id: Uuid =
        sqlx::query_scalar("SELECT ent_fin_compte_du_foyer($1, $2, NULL, 'manuel')")
            .bind(etab)
            .bind(cast.foyer_lea_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let mut participants = Vec::new();
    for eleve in [cast.eleve_lea_id, cast.eleve_tom_id] {
        let p = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_sortie_participant (id, etablissement_id, sortie_id, eleve_id, etat) VALUES ($1, $2, $3, $4, 'inscrit')")
            .bind(p)
            .bind(etab)
            .bind(id)
            .bind(eleve)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ent_sortie_part (id, etablissement_id, sortie_id, participant_id, compte_id, prix_rang, montant_centimes, part_bps, jour_calcul)
             VALUES ($1, $2, $3, $4, $5, 1, 50000, 10000, DATE '2031-03-12')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(p)
        .bind(compte_id)
        .execute(&pool)
        .await
        .unwrap();
        participants.push(p);
    }
    let (p_lea, p_tom) = (participants[0], participants[1]);
    let aide = |p: Uuid, montant: &str| {
        json!({
            "participant_id": p, "compte_id": compte_id, "source": "apel", "montant": montant,
            "reference": "DEC-1", "motif": "Décision de la commission", "cle": cle(),
        })
    };

    // 201 : aide appliquée sous le seuil (150 €). 202 : au-delà, elle attend une seconde personne.
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/argent/aides"),
        Some(compta),
        Some(aide(p_lea, "100")),
    )
    .await;
    assert_eq!(
        (st, d["statut"].as_str()),
        (StatusCode::CREATED, Some("appliquee")),
        "{d}"
    );
    let aide_id = d["aide"]["id"].as_str().expect("aide écrite").to_string();
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/argent/aides"),
        Some(compta),
        Some(aide(p_tom, "200")),
    )
    .await;
    assert_eq!(
        (st, d["statut"].as_str(), d["aide"].is_null()),
        (StatusCode::ACCEPTED, Some("en_attente"), true),
        "{d}"
    );
    let demande_tom = d["demande_id"].as_str().unwrap().to_string();
    // Le cumul compte (M-2) : 100 + 60 € sur le même participant et le même payeur dépassent le seuil.
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/argent/aides"),
        Some(compta),
        Some(aide(p_lea, "60")),
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED, "{d}");
    let demande_lea = d["demande_id"].as_str().unwrap().to_string();
    // 422 : montant illisible, source inconnue, clé trop courte, motif vide.
    for mauvais in [
        json!({"participant_id": p_lea, "compte_id": compte_id, "source": "apel", "montant": "abc", "motif": "x", "cle": cle()}),
        json!({"participant_id": p_lea, "compte_id": compte_id, "source": "inconnue", "montant": "10", "motif": "x", "cle": cle()}),
        json!({"participant_id": p_lea, "compte_id": compte_id, "source": "apel", "montant": "10", "motif": "x", "cle": "court"}),
        json!({"participant_id": p_lea, "compte_id": compte_id, "source": "apel", "montant": "10", "motif": "  ", "cle": cle()}),
    ] {
        let (st, d) = appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/aides"),
            Some(compta),
            Some(mauvais),
        )
        .await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{st} {d}");
    }
    // 403 : un professeur (porteur, sans le droit d'argent) ; 404 : une famille, une sortie inconnue, un autre établissement.
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/aides"),
            Some(prof),
            Some(aide(p_lea, "10"))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/aides"),
            Some(parent),
            Some(aide(p_lea, "10"))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{fantome}/argent/aides"),
            Some(compta),
            Some(aide(p_lea, "10"))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/aides"),
            Some(b_dir),
            Some(aide(p_lea, "10"))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // F-a : un professeur sans droit reçoit 403 AVANT qu'on regarde si la sortie existe.
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{fantome}/argent/aides"),
            Some(prof),
            Some(aide(p_lea, "10"))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Lecture : 200 pour la comptabilité, 404 pour la famille.
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/voyages/{id}/argent/aides"),
            Some(compta),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/voyages/{id}/argent/aides"),
            Some(parent),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // Demandes : liste (200), le demandeur ne se valide pas (403), une autre personne valide (201), refus motivé (200).
    let (st, liste) = appel(
        &app,
        "GET",
        &format!("/voyages/{id}/argent/demandes"),
        Some(compta),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(liste.as_array().unwrap().len(), 3);
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/demandes/{demande_tom}/validation"),
            Some(compta),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/demandes/{demande_tom}/validation"),
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
            "POST",
            &format!("/voyages/{id}/argent/demandes/{demande_tom}/validation"),
            Some(dir),
            None
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/demandes/{demande_tom}/validation"),
            Some(dir),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT,
        "une demande tranchée ne se retranche pas"
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/demandes/{fantome}/validation"),
            Some(dir),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/demandes/{demande_lea}/refus"),
            Some(dir),
            Some(json!({"motif": ""}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/demandes/{demande_lea}/refus"),
            Some(dir),
            Some(json!({"motif": "Hors critères"}))
        )
        .await
        .0,
        StatusCode::OK
    );
    // Annulation d'une aide : 201 sous le seuil ; 409 si déjà annulée ; 404 inconnue ; 403 sans droit.
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/aides/{aide_id}/annulation"),
            Some(prof),
            Some(json!({"motif": "Erreur"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/aides/{aide_id}/annulation"),
            Some(compta),
            Some(json!({"motif": "Erreur de saisie"}))
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/aides/{aide_id}/annulation"),
            Some(compta),
            Some(json!({"motif": "Erreur de saisie"}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/aides/{fantome}/annulation"),
            Some(compta),
            Some(json!({"motif": "Erreur"}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // Reports : 201 sous le seuil, 202 au-delà (cumul), 422 (même participant, montant), 403, 404.
    let (st, v) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/argent/versements"),
        Some(compta),
        Some(json!({"participant_id": p_lea, "compte_id": compte_id, "moyen": "cheque", "montant": "300"})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{v}");
    let report = |de: Uuid, vers: Uuid, montant: &str| {
        json!({
            "de_participant_id": de, "vers_participant_id": vers, "compte_id": compte_id, "montant": montant,
            "motif": "Le frère prend la place", "cle": cle(),
        })
    };
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/argent/reports"),
        Some(compta),
        Some(report(p_lea, p_tom, "100")),
    )
    .await;
    assert_eq!(
        (st, d["statut"].as_str()),
        (StatusCode::CREATED, Some("appliquee")),
        "{d}"
    );
    let report_id = d["report"]["id"]
        .as_str()
        .expect("report écrit")
        .to_string();
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/argent/reports"),
        Some(compta),
        Some(report(p_lea, p_tom, "60")),
    )
    .await;
    assert_eq!(
        (st, d["statut"].as_str()),
        (StatusCode::ACCEPTED, Some("en_attente")),
        "100 + 60 € cumulés : {d}"
    );
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/argent/reports"),
        Some(compta),
        Some(report(p_lea, p_lea, "10")),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{st} {d}");
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/argent/reports"),
        Some(compta),
        Some(report(p_lea, p_tom, "x")),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{st} {d}");
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/reports"),
            Some(prof),
            Some(report(p_lea, p_tom, "10"))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/reports"),
            Some(parent),
            Some(report(p_lea, p_tom, "10"))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{fantome}/argent/reports"),
            Some(compta),
            Some(report(p_lea, p_tom, "10"))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/voyages/{id}/argent/reports"),
            Some(compta),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/reports/{report_id}/annulation"),
            Some(prof),
            Some(json!({"motif": "Erreur"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/reports/{report_id}/annulation"),
            Some(compta),
            Some(json!({"motif": "Erreur de saisie"}))
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/argent/reports/{fantome}/annulation"),
            Some(compta),
            Some(json!({"motif": "Erreur"}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // Consigne du motif : 200 pour le personnel, 404 pour la famille.
    let (st, c) = appel(
        &app,
        "GET",
        &format!("/voyages/{id}/argent/consigne-motif"),
        Some(compta),
        None,
    )
    .await;
    assert_eq!(
        (st, c["motif_obligatoire"].as_bool()),
        (StatusCode::OK, Some(true))
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/voyages/{id}/argent/consigne-motif"),
            Some(parent),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // V3b (Moyen 3) : contournements tracés. Liste : 200 pour la comptabilité, 403 pour le porteur sans droit d'argent,
    // 404 pour la famille ; demande d'une personne libre : 409 (aucun contournement nécessaire) ; action inconnue : 422 ;
    // décision d'une demande inconnue : 404.
    let url_cont = format!("/voyages/{id}/argent/contournements");
    let (st, l) = appel(&app, "GET", &url_cont, Some(compta), None).await;
    assert_eq!((st, l.is_array()), (StatusCode::OK, true), "{l}");
    assert_eq!(
        appel(&app, "GET", &url_cont, Some(prof), None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(&app, "GET", &url_cont, Some(parent), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &url_cont,
            Some(compta),
            Some(json!({"action": "acompte_creer", "motif": "Seule présente",
                        "montant_max": "100", "prestataire_id": fantome}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // Moyen 1 : une demande sans sa portée (plafond, prestataire) est refusée : pas d'autorisation « en blanc » (422).
    assert_eq!(
        appel(
            &app,
            "POST",
            &url_cont,
            Some(compta),
            Some(json!({"action": "acompte_creer", "motif": "Sans portée"}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &url_cont,
            Some(compta),
            Some(json!({"action": "tout_faire", "motif": "Motif"}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("{url_cont}/{fantome}/decision"),
            Some(dir),
            Some(json!({"autoriser": true}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // Suites V3b, 2 : révocation d'une autorisation inconnue : 404 ; sans droit d'argent : 403 ; famille : 404.
    let url_rev = format!("{url_cont}/{fantome}/revocation");
    assert_eq!(
        appel(&app, "POST", &url_rev, Some(compta), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(&app, "POST", &url_rev, Some(prof), None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(&app, "POST", &url_rev, Some(parent), None).await.0,
        StatusCode::NOT_FOUND
    );

    // Internat : synchronisation (200 pour le porteur, 403 sans droit, 404 famille ou sortie inconnue).
    let (st, r) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/internat/synchroniser"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert!(r["poses"].is_number() && r["annules"].is_number());
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/internat/synchroniser"),
            Some(parent),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{fantome}/internat/synchroniser"),
            Some(dir),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/internat/synchroniser"),
            Some(b_dir),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/internat/synchroniser"),
            Some(compta),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    // Retour anticipé : 409 tant que le participant n'est pas parti ; 422 (motif inconnu, retour dans le futur) ; 200 ;
    // 409 la seconde fois ; 404 famille et sortie inconnue ; 403 sans droit.
    let rentre = |motif: &str| json!({"motif_code": motif});
    let url_retour = format!("/voyages/{id}/participants/{p_lea}/retour-anticipe");
    assert_eq!(
        appel(&app, "POST", &url_retour, Some(prof), Some(rentre("sante")))
            .await
            .0,
        StatusCode::CONFLICT
    );
    // Le séjour est en cours : parti hier, de retour demain (marges de jours, aucune dépendance à l'heure de la journée).
    sqlx::query("UPDATE ent_sortie SET etat = 'en_cours', debut = now() - interval '1 day', fin = now() + interval '2 days' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_sortie_participant SET etat = 'parti' WHERE id = $1")
        .bind(p_lea)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        appel(
            &app,
            "POST",
            &url_retour,
            Some(prof),
            Some(rentre("inconnu"))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let futur = (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339();
    assert_eq!(
        appel(
            &app,
            "POST",
            &url_retour,
            Some(prof),
            Some(json!({"motif_code": "sante", "retour_le": futur}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY,
        "F-b : un retour dans le futur est refusé"
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &url_retour,
            Some(compta),
            Some(rentre("sante"))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &url_retour,
            Some(parent),
            Some(rentre("sante"))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{fantome}/participants/{p_lea}/retour-anticipe"),
            Some(dir),
            Some(rentre("sante"))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (st, r) = appel(&app, "POST", &url_retour, Some(prof), Some(rentre("sante"))).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!(
        appel(&app, "POST", &url_retour, Some(prof), Some(rentre("sante")))
            .await
            .0,
        StatusCode::CONFLICT
    );
}
