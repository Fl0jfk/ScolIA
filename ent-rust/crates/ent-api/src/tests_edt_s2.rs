//! Socle 2, lot S2-5 : routes EDT, remplacements, annulation en masse et délégations (HTTP, base
//! locale de labo). Codes de retour, contrat JSON (`ent_types::edt_s2`), droits, isolation entre
//! établissements, notifications des familles.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Datelike, Duration, NaiveDate};
use ent_db::outbox::ConfigDistribution;
use ent_types::edt_s2::{
    AbsenceSeancesDto, CandidatDto, ChangementEdtDto, ConflitsReponseDto, CreneauEnregistreDto,
    DelegationDto, RapportMasseDto, ResultatExceptionDto, SemaineEdtDto,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

async fn etat() -> Option<AppState> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())?;
    let ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    Some(AppState::nouveau(ent))
}

async fn appel(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: &str,
    corps: Option<Value>,
) -> (StatusCode, String) {
    let b = Request::builder()
        .method(methode)
        .uri(uri)
        .header("authorization", format!("Bearer {jeton}"));
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

fn lundi_futur(debut: NaiveDate, fin: NaiveDate) -> NaiveDate {
    let lundi_de = |d: NaiveDate| d - Duration::days(d.weekday().num_days_from_monday() as i64);
    let mut l = lundi_de(domain_eleve::today_utc() + Duration::days(14));
    if l + Duration::days(13) > fin {
        l = lundi_de(fin) - Duration::days(14);
    }
    assert!(l >= debut);
    l
}

fn creneau(classe: &str, prof: Uuid, jour: i16, h: (u32, u32), salle: Option<&str>) -> Value {
    json!({ "classe": classe, "groupe_id": null, "matiere_libelle": "Mathématiques", "prof_user_id": prof,
            "jour_semaine": jour, "heure_debut": format!("{:02}:00:00", h.0), "heure_fin": format!("{:02}:00:00", h.1),
            "salle": salle, "site": "", "semaine_ab": "toutes" })
}

async fn distribuer(state: &AppState, etab: Uuid) {
    let cfg = ConfigDistribution {
        taille_lot: 200,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    };
    state.ent.distributeur(cfg).distribuer_tout().await.unwrap();
}

#[tokio::test]
async fn edt_routes_conflits_remplacement_masse_delegations_et_isolation() {
    let Some(state) = etat().await else {
        eprintln!("skip S2-5 HTTP: ENT_DATABASE_URL");
        return;
    };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let autre = state.ent.seed_labo_cast().await.expect("cast B");
    let annee = state
        .ent
        .referentiel_annee_courante_assuree(cast.etablissement_id)
        .await
        .unwrap();
    let lundi = lundi_futur(annee.date_debut, annee.date_fin);
    let jour = |n: i64| lundi + Duration::days(n - 1);
    let etab = cast.etablissement_id;
    let app = app(state.clone());
    let dir = cast.direction.token.as_str();
    let (absent, libre) = (cast.profs[0].user_id, cast.profs[1].user_id);
    let harry = cast.eleve_lea_id;

    // ── Créneaux : droits, création, conflit 409 détaillé, aperçu, validations ──
    for jeton in [
        cast.prof.token.as_str(),
        cast.parent_lea.token.as_str(),
        cast.compta.token.as_str(),
    ] {
        let (st, _) = appel(
            &app,
            "POST",
            "/edt/admin/creneaux",
            jeton,
            Some(creneau("4B", absent, 1, (14, 15), None)),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN);
        let (st, _) = appel(&app, "GET", "/edt/admin/conflits", jeton, None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let (st, txt) = appel(
        &app,
        "POST",
        "/edt/admin/creneaux",
        dir,
        Some(creneau("4B", absent, 1, (14, 15), None)),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{txt}");
    let cr: CreneauEnregistreDto = serde_json::from_str(&txt).unwrap();
    let cr_id = cr.creneau.expect("créneau créé").id;
    let (st, txt) = appel(
        &app,
        "POST",
        "/edt/admin/creneaux",
        dir,
        Some(creneau("5A", absent, 1, (14, 15), None)),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{txt}");
    let conflit: ConflitsReponseDto = serde_json::from_str(&txt).unwrap();
    assert!(
        conflit.conflits.iter().any(|c| c.nature == "professeur"),
        "{conflit:?}"
    );
    let (st, txt) = appel(
        &app,
        "POST",
        "/edt/admin/creneaux?apercu=true",
        dir,
        Some(creneau("5A", libre, 1, (16, 17), None)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    assert!(
        serde_json::from_str::<CreneauEnregistreDto>(&txt)
            .unwrap()
            .creneau
            .is_none()
    );
    let (st, _) = appel(
        &app,
        "POST",
        "/edt/admin/creneaux",
        dir,
        Some(creneau("ZZ9", libre, 1, (16, 17), None)),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = appel(
        &app,
        "POST",
        "/edt/admin/creneaux",
        dir,
        Some(creneau("4B", Uuid::new_v4(), 2, (16, 17), None)),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(
        &app,
        "POST",
        "/edt/admin/creneaux",
        &autre.direction.token,
        Some(creneau("4B", absent, 2, (16, 17), None)),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "professeur d'un autre établissement"
    );
    // Un second créneau pour la matière, tenu par le remplaçant potentiel (compétent).
    let (st, _) = appel(
        &app,
        "POST",
        "/edt/admin/creneaux",
        dir,
        Some(creneau("5A", libre, 2, (16, 17), None)),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    let (st, txt) = appel(&app, "GET", "/edt/admin/conflits", dir, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        serde_json::from_str::<Value>(&txt)
            .unwrap()
            .as_array()
            .map(Vec::len),
        Some(0)
    );

    // ── Semaine : contrat JSON, cibles, droits ──
    let uri_sem = format!("/edt/semaine?cible=personnel&id={absent}&jour={}", jour(3));
    let (st, txt) = appel(&app, "GET", &uri_sem, dir, None).await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    let sem: SemaineEdtDto = serde_json::from_str(&txt).unwrap();
    assert_eq!(sem.lundi, lundi);
    assert!(
        sem.seances
            .iter()
            .any(|s| s.creneau_id == cr_id && s.statut == "prevue" && s.jour == jour(1))
    );
    let (st, _) = appel(&app, "GET", "/edt/semaine?cible=lune", dir, None).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/edt/semaine?cible=salle&salle=&jour={lundi}"),
        dir,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let parent = cast.parent_lea.token.as_str();
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/edt/semaine?cible=eleve&id={harry}&jour={lundi}"),
        parent,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/edt/semaine?cible=eleve&id={harry}&jour={lundi}"),
        &cast.parent_tom.token,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/edt/semaine?cible=personnel&id={absent}&jour={lundi}"),
        parent,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/edt/semaine?cible=eleve&id={harry}&jour={lundi}"),
        &autre.direction.token,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // ── Absence → séances touchées → remplaçants → remplacement → familles prévenues ──
    let (st, txt) = appel(
        &app,
        "POST",
        "/rh/absences",
        dir,
        Some(json!({
        "user_id": absent, "debut": jour(1), "fin": jour(1), "motif": "Maladie (confidentiel)" })),
    )
    .await;
    assert!(st.is_success(), "{st} {txt}");
    let abs_id: Uuid = serde_json::from_str::<Value>(&txt).unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    // Socle 15 (R-AP46) : saisie par un tiers = « signalée » ; la direction la confirme par un rappel
    // tracé avant de la valider.
    let (st, txt) = appel(
        &app,
        "POST",
        &format!("/personnels/absences/{abs_id}/rappel-confirme"),
        dir,
        Some(json!({})),
    )
    .await;
    assert!(st.is_success(), "{st} {txt}");
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rh/absences/{abs_id}/valider"),
        dir,
        Some(json!({})),
    )
    .await;
    assert!(st.is_success());
    let (st, txt) = appel(
        &app,
        "GET",
        &format!("/edt/absences?du={lundi}&au={}", jour(7)),
        dir,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    let liste: Vec<AbsenceSeancesDto> = serde_json::from_str(&txt).unwrap();
    let ligne = liste
        .iter()
        .find(|l| l.absence.id == abs_id)
        .expect("absence listée");
    assert!(
        ligne
            .seances
            .iter()
            .any(|s| s.creneau_id == cr_id && s.statut == "prevue")
    );
    assert!(
        !txt.contains("confidentiel"),
        "le motif de l'absence ne sort jamais"
    );
    for jeton in [cast.prof.token.as_str(), parent] {
        let (st, _) = appel(
            &app,
            "GET",
            &format!("/edt/absences?du={lundi}&au={}", jour(7)),
            jeton,
            None,
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let (st, txt) = appel(
        &app,
        "GET",
        &format!(
            "/edt/remplacants?creneau_id={cr_id}&jour={}&tous=true",
            jour(1)
        ),
        dir,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    let cands: Vec<CandidatDto> = serde_json::from_str(&txt).unwrap();
    assert!(
        cands.iter().any(|c| c.user_id == libre && c.competent),
        "{cands:?}"
    );
    assert!(cands.iter().all(|c| c.user_id != absent));
    let cmd = json!({ "creneau_id": cr_id, "jour": jour(1), "remplacant_user_id": libre,
                      "absence_rh_id": abs_id, "nouvelle_salle": null });
    let (st, _) = appel(
        &app,
        "POST",
        "/edt/remplacements",
        &cast.prof.token,
        Some(cmd.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, txt) = appel(&app, "POST", "/edt/remplacements", dir, Some(cmd.clone())).await;
    assert_eq!(st, StatusCode::CREATED, "{txt}");
    let r: ResultatExceptionDto = serde_json::from_str(&txt).unwrap();
    assert!(!r.deja_fait && r.delegation_id.is_some());
    let (st, txt) = appel(&app, "POST", "/edt/remplacements", dir, Some(cmd.clone())).await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    assert!(
        serde_json::from_str::<ResultatExceptionDto>(&txt)
            .unwrap()
            .deja_fait
    );
    let mauvais = json!({ "creneau_id": cr_id, "jour": jour(1), "remplacant_user_id": absent,
                          "absence_rh_id": abs_id, "nouvelle_salle": null });
    let (st, _) = appel(&app, "POST", "/edt/remplacements", dir, Some(mauvais)).await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, _) = appel(
        &app,
        "POST",
        "/edt/remplacements",
        &autre.direction.token,
        Some(cmd),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "créneau d'un autre établissement"
    );
    distribuer(&state, etab).await;
    distribuer(&state, etab).await;
    let (st, txt) = appel(&app, "GET", "/notifications", parent, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(txt.matches("Cours remplacé").count(), 1, "{txt}");
    assert!(!txt.contains("Maladie"));
    let (st, txt) = appel(
        &app,
        "GET",
        &format!("/eleves/{harry}/edt/changements?du={lundi}&au={}", jour(7)),
        parent,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    let ch: Vec<ChangementEdtDto> = serde_json::from_str(&txt).unwrap();
    assert!(
        ch.iter()
            .any(|c| c.creneau_id == cr_id && c.nature == "remplacement"),
        "{ch:?}"
    );
    assert!(!txt.contains("Maladie"));
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/eleves/{harry}/edt/changements?du={lundi}&au={}", jour(7)),
        &cast.parent_tom.token,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/eleves/{harry}/edt/changements?du={lundi}&au={}", jour(7)),
        &autre.direction.token,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Lever le remplacement : 204 puis 404 (déjà levé).
    let (st, _) = appel(
        &app,
        "DELETE",
        &format!("/edt/exceptions-levee?creneau_id={cr_id}&jour={}", jour(1)),
        dir,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (st, _) = appel(
        &app,
        "DELETE",
        &format!("/edt/exceptions-levee?creneau_id={cr_id}&jour={}", jour(1)),
        dir,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // ── Annulation en masse : aperçu 200, application 201, répétition 200 (clé), droits ──
    let masse = |apercu: bool| {
        json!({ "du": jour(2), "au": jour(2), "portee": "etablissement", "cibles": [],
        "heure_debut": "15:00:00", "heure_fin": "23:00:00", "motif_public": "Grève", "cle": "greve-http", "apercu": apercu })
    };
    for jeton in [
        cast.administratif.token.as_str(),
        cast.prof.token.as_str(),
        parent,
    ] {
        let (st, _) = appel(
            &app,
            "POST",
            "/edt/annulation-masse",
            jeton,
            Some(masse(true)),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let (st, txt) = appel(
        &app,
        "POST",
        "/edt/annulation-masse",
        dir,
        Some(masse(true)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    let ap: RapportMasseDto = serde_json::from_str(&txt).unwrap();
    assert!(
        ap.apercu && ap.nb_seances == 1 && ap.classes.len() == 1,
        "{ap:?}"
    );
    let (st, txt) = appel(
        &app,
        "POST",
        "/edt/annulation-masse",
        dir,
        Some(masse(false)),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{txt}");
    let (st, txt) = appel(
        &app,
        "POST",
        "/edt/annulation-masse",
        dir,
        Some(masse(false)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    assert!(
        serde_json::from_str::<RapportMasseDto>(&txt)
            .unwrap()
            .deja_applique
    );
    let (st, _) = appel(&app, "POST", "/edt/annulation-masse", dir, Some(json!({
        "du": jour(2), "au": jour(1), "portee": "etablissement", "cibles": [], "heure_debut": null, "heure_fin": null,
        "motif_public": "x", "cle": null, "apercu": true }))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // ── Délégations ──
    let corps = json!({ "delegue_user_id": libre, "droits": [{ "action": "edt.gerer", "portee": "etablissement", "cibles": [] }],
                        "debut": null, "fin": lundi + Duration::days(20), "motif": "Sortie de niveau" });
    for jeton in [
        cast.administratif.token.as_str(),
        cast.prof.token.as_str(),
        parent,
    ] {
        let (st, _) = appel(&app, "POST", "/delegations", jeton, Some(corps.clone())).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
        let (st, _) = appel(&app, "GET", "/delegations", jeton, None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let (st, txt) = appel(&app, "POST", "/delegations", dir, Some(corps.clone())).await;
    assert_eq!(st, StatusCode::CREATED, "{txt}");
    let dl: DelegationDto = serde_json::from_str(&txt).unwrap();
    assert_eq!(
        (dl.etat.as_str(), dl.source.as_str()),
        ("active", "manuelle")
    );
    // Date de fin obligatoire et non délégable → 400.
    let (st, _) = appel(
        &app,
        "POST",
        "/delegations",
        dir,
        Some(json!({ "delegue_user_id": libre, "droits": [], "fin": jour(3) })),
    )
    .await;
    assert!(st.is_client_error());
    let mut sans_fin = corps.clone();
    sans_fin.as_object_mut().unwrap().remove("fin");
    let (st, _) = appel(&app, "POST", "/delegations", dir, Some(sans_fin)).await;
    assert!(
        st == StatusCode::UNPROCESSABLE_ENTITY || st == StatusCode::BAD_REQUEST,
        "{st}"
    );
    let (st, _) = appel(&app, "POST", "/delegations", dir, Some(json!({ "delegue_user_id": libre,
        "droits": [{ "action": "roles.gerer", "portee": "etablissement", "cibles": [] }], "debut": null, "fin": jour(3), "motif": "" }))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // Le délégué voit SA délégation ; la liste de l'autre établissement est vide ; révocation 404 chez lui.
    let delegue_token = cast.profs[1].token.as_str();
    let (st, txt) = appel(&app, "GET", "/delegations/mes", delegue_token, None).await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    assert!(
        serde_json::from_str::<Vec<DelegationDto>>(&txt)
            .unwrap()
            .iter()
            .any(|d| d.id == dl.id)
    );
    let (st, txt) = appel(
        &app,
        "GET",
        "/delegations?en_cours=true",
        &autre.direction.token,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        serde_json::from_str::<Vec<DelegationDto>>(&txt)
            .unwrap()
            .len(),
        0
    );
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/delegations/{}/revoquer", dl.id),
        &autre.direction.token,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Avec la délégation, le délégué gère l'EDT (aperçu de création accepté), puis plus après révocation.
    let (st, _) = appel(
        &app,
        "POST",
        "/edt/admin/creneaux?apercu=true",
        delegue_token,
        Some(creneau("3C", libre, 4, (17, 18), None)),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, txt) = appel(
        &app,
        "POST",
        &format!("/delegations/{}/revoquer", dl.id),
        dir,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    assert_eq!(
        serde_json::from_str::<DelegationDto>(&txt).unwrap().etat,
        "revoquee"
    );
    let (st, _) = appel(
        &app,
        "POST",
        "/edt/admin/creneaux?apercu=true",
        delegue_token,
        Some(creneau("3C", libre, 4, (17, 18), None)),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}
