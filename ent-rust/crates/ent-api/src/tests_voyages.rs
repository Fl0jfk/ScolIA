//! Socle 42, lot V0a (HTTP) : routes `/voyages/*` — session obligatoire, matrice de droits,
//! cloisonnement entre établissements, commandes d'état, projections, export, et les deux défauts
//! du module Next.js corrigés (circulaire et envoi aux prestataires sans contrôle de droits).
//! Base locale de labo (cast Harry Potter).

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
    let (st, _, v) = appel_brut(app, m, uri, jeton, corps).await;
    (st, v)
}

async fn appel_brut(
    app: &axum::Router,
    m: &str,
    uri: &str,
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
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
    let h = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (st, h, v)
}

#[tokio::test]
async fn routes_voyages_droits_cloisonnement_commandes_et_defauts_nextjs() {
    let Some(url) = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
    else {
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
    let (dir, adm, prof, compta, parent) = (
        cast.direction.token.as_str(),
        cast.administratif.token.as_str(),
        cast.prof.token.as_str(),
        cast.compta.token.as_str(),
        cast.parent_lea.token.as_str(),
    );
    let b_dir = autre.direction.token.as_str();
    let eleves_4b: Vec<Uuid> = cast
        .eleves
        .iter()
        .filter(|e| e.classe == "4B")
        .map(|e| e.id)
        .collect();
    assert!(eleves_4b.len() >= 2);

    // Session obligatoire sur toutes les routes.
    let fantome = Uuid::new_v4();
    for (m, u) in [
        ("GET", "/voyages".to_string()),
        ("POST", "/voyages".to_string()),
        ("GET", "/voyages/tableau".to_string()),
        ("GET", "/voyages/famille".to_string()),
        ("GET", format!("/voyages/{fantome}")),
        ("POST", format!("/voyages/{fantome}/commandes/soumettre")),
        ("POST", format!("/voyages/{fantome}/circulaire")),
        ("GET", format!("/voyages/{fantome}/participants.csv")),
    ] {
        let (st, _) = appel(&app, m, &u, None, None).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }

    // Les familles n'ont ni liste, ni création ; seule la projection famille leur est ouverte.
    assert_eq!(
        appel(&app, "GET", "/voyages", Some(parent), None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(&app, "GET", "/voyages/famille", Some(parent), None)
            .await
            .0,
        StatusCode::OK
    );

    // Création par un professeur (devient porteur) ; valeurs invalides : 4xx.
    let debut = chrono::Utc::now() + chrono::Duration::days(3);
    let corps = json!({
        "titre": "Sortie au muséum", "nature": "facultative", "forme": "sans_nuitee",
        "destination": "proximite", "destination_libelle": "Muséum",
        "debut": debut.to_rfc3339(), "fin": (debut + chrono::Duration::hours(6)).to_rfc3339(),
        "classes": [classe.id], "effectif_eleves_declare": 2, "point_rendez_vous": "Grille",
    });
    for mauvais in [
        json!({"titre": "", "nature": "facultative", "forme": "sans_nuitee", "destination": "proximite",
               "debut": debut.to_rfc3339(), "fin": (debut + chrono::Duration::hours(2)).to_rfc3339()}),
        json!({"titre": "x", "nature": "bizarre", "forme": "sans_nuitee", "destination": "proximite",
               "debut": debut.to_rfc3339(), "fin": (debut + chrono::Duration::hours(2)).to_rfc3339()}),
        json!({"titre": "x", "nature": "facultative", "forme": "sans_nuitee", "destination": "etranger",
               "debut": debut.to_rfc3339(), "fin": (debut + chrono::Duration::hours(2)).to_rfc3339()}),
    ] {
        let (st, _) = appel(&app, "POST", "/voyages", Some(prof), Some(mauvais)).await;
        assert!(st.is_client_error(), "{st}");
    }
    let (st, cree) = appel(&app, "POST", "/voyages", Some(prof), Some(corps.clone())).await;
    assert_eq!(st, StatusCode::CREATED, "{cree}");
    let id = cree["id"].as_str().unwrap().to_string();
    assert_eq!(cree["etat"], "brouillon");
    // Les champs internes ne sortent pas de l'API.
    assert!(
        cree.get("cree_par_user_id").is_none() && cree.get("liste_confirmee_par_user_id").is_none()
    );
    // Le JSON se désérialise dans les types partagés du frontend.
    let _: ent_types::voyages::SortieDto = serde_json::from_value(cree.clone()).expect("SortieDto");

    // Cloisonnement : l'établissement B ne voit rien et ne peut rien faire.
    assert_eq!(
        appel(&app, "GET", &format!("/voyages/{id}"), Some(b_dir), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (st, liste_b) = appel(&app, "GET", "/voyages", Some(b_dir), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(liste_b
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["id"] != id.as_str()));
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/commandes/valider-pedagogie"),
            Some(b_dir),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // Un autre membre du personnel (la CPE) : introuvable (pas de fuite d'existence).
    let cpe = cast.cpe.token.as_str();
    assert_eq!(
        appel(&app, "GET", &format!("/voyages/{id}"), Some(cpe), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    // Soumission refusée : aucun encadrant. Puis ajout d'un encadrant par l'annuaire du personnel.
    let (st, rep) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/commandes/soumettre"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{rep}");
    let (st, annuaire) = appel(&app, "GET", "/personnels/annuaire", Some(prof), None).await;
    assert_eq!(st, StatusCode::OK, "{annuaire}");
    let agent = annuaire
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["user_id"] == cast.cpe.user_id.to_string())
        .map(|a| a["agent_id"].as_str().unwrap().to_string())
        .expect("la CPE est dans l'annuaire");
    let (st, enc) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/encadrants"),
        Some(prof),
        Some(json!({"agent_id": agent, "role": "responsable"})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{enc}");
    let _: ent_types::voyages::EncadrantDto = serde_json::from_value(enc).expect("EncadrantDto");
    // La CPE, désormais encadrante, lit le détail mais n'exporte pas la liste et ne modifie pas.
    assert_eq!(
        appel(&app, "GET", &format!("/voyages/{id}"), Some(cpe), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/voyages/{id}/participants.csv"),
            Some(cpe),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (st, _) = appel(
        &app,
        "PUT",
        &format!("/voyages/{id}"),
        Some(cpe),
        Some({
            let mut c = corps.clone();
            c["version"] = json!(1);
            c
        }),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Commandes : le porteur soumet ; il ne valide pas sa propre pédagogie ; la direction oui.
    let (st, s) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/commandes/soumettre"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{s}");
    assert_eq!(s["etat"], "soumise");
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/commandes/valider-pedagogie"),
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
            &format!("/voyages/{id}/commandes/inconnue"),
            Some(dir),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // Mauvaise étape : conflit.
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/commandes/autoriser"),
            Some(dir),
            Some(json!({"version": 1}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // Verrou optimiste : une version périmée est refusée.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/commandes/valider-pedagogie"),
        Some(dir),
        Some(json!({"version": 1})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, s) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/commandes/valider-pedagogie"),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(
        (st, s["etat"].as_str()),
        (StatusCode::OK, Some("preparation"))
    );
    let (st, s) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/commandes/passer-finances"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(
        (st, s["etat"].as_str()),
        (StatusCode::OK, Some("a_autoriser")),
        "{s}"
    );

    // Circulaire : refusée avant l'autorisation, même au porteur ; refusée à un tiers, à la
    // comptabilité et à une famille ; la direction autorise.
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/circulaire"),
            Some(prof),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (_, courant) = appel(&app, "GET", &format!("/voyages/{id}"), Some(dir), None).await;
    let ver = courant["sortie"]["version"].clone();
    let (st, s) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/commandes/autoriser"),
        Some(dir),
        Some(json!({"version": ver})),
    )
    .await;
    assert_eq!(
        (st, s["etat"].as_str()),
        (StatusCode::OK, Some("ouverte")),
        "{s}"
    );
    for (jeton, nom) in [
        (compta, "compta"),
        (parent, "parent"),
        (cast.profs[1].token.as_str(), "prof tiers"),
    ] {
        let (st, _) = appel(
            &app,
            "POST",
            &format!("/voyages/{id}/circulaire"),
            Some(jeton),
            None,
        )
        .await;
        assert!(
            st == StatusCode::FORBIDDEN || st == StatusCode::NOT_FOUND,
            "circulaire {nom} : {st}"
        );
    }
    let (st, circ) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/circulaire"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{circ}");
    let circ: ent_types::voyages::CirculaireDto =
        serde_json::from_value(circ).expect("CirculaireDto");
    assert!(circ.corps.contains("Sortie au muséum") && circ.corps.contains("Grille"));
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/voyages/{id}/circulaire"),
            Some(adm),
            None
        )
        .await
        .0,
        StatusCode::OK
    );

    // Participants : choix dans le registre ; élèves d'un autre établissement refusés.
    let (st, cand) = appel(
        &app,
        "GET",
        &format!("/voyages/{id}/candidats"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{cand}");
    let cand: Vec<ent_types::voyages::CandidatsClasseDto> =
        serde_json::from_value(cand).expect("CandidatsClasseDto");
    assert!(cand[0].eleves.len() >= 2);
    let (st, r) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/participants"),
        Some(prof),
        Some(json!({"eleve_ids": [eleves_4b[0], eleves_4b[1], autre.eleve_lea_id]})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    let r: ent_types::voyages::ResultatAjoutDto =
        serde_json::from_value(r).expect("ResultatAjoutDto");
    assert_eq!((r.ajoutes.len(), r.refuses), (2, vec![autre.eleve_lea_id]));
    // Liste confirmée (sans nuitée : pas de dépôt/reprise exigés) → prête.
    let (st, s) = appel(
        &app,
        "POST",
        &format!("/voyages/{id}/commandes/confirmer-liste"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(
        (st, s["etat"].as_str()),
        (StatusCode::OK, Some("prete")),
        "{s}"
    );

    // Export : porteur oui ; en-têtes sûrs ; journalisé.
    let (st, h, csv) = appel_brut(
        &app,
        "GET",
        &format!("/voyages/{id}/participants.csv"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(h["content-type"].to_str().unwrap().starts_with("text/csv"));
    assert_eq!(h["cache-control"], "no-store");
    assert!(csv.as_str().unwrap().contains("Nom;Prénom;Classe"));
    let (st, journal) = appel(
        &app,
        "GET",
        &format!("/voyages/{id}/journal"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let journal: Vec<ent_types::voyages::JournalDto> =
        serde_json::from_value(journal).expect("JournalDto");
    assert!(journal.iter().any(|j| j.action == "liste_exportee"));
    assert!(journal
        .iter()
        .any(|j| j.action == "autorisee" && j.etat_apres.as_deref() == Some("ouverte")));

    // Projections : détail typé, droits calculés, prochaine étape ; la famille de Harry voit sa sortie.
    let (st, d) = appel(&app, "GET", &format!("/voyages/{id}"), Some(prof), None).await;
    assert_eq!(st, StatusCode::OK);
    let d: ent_types::voyages::SortieDetailDto =
        serde_json::from_value(d).expect("SortieDetailDto");
    assert!(d.acces.porteur && d.acces.modifier && !d.acces.valider);
    assert_eq!(d.nb_inscrits, 2);
    let (_, d_dir) = appel(&app, "GET", &format!("/voyages/{id}"), Some(dir), None).await;
    assert_eq!(d_dir["acces"]["valider"], true);
    let (st, fam) = appel(&app, "GET", "/voyages/famille", Some(parent), None).await;
    assert_eq!(st, StatusCode::OK);
    let fam: Vec<ent_types::voyages::VoyageFamilleDto> =
        serde_json::from_value(fam).expect("VoyageFamilleDto");
    assert!(fam.iter().all(|f| f.titre == "Sortie au muséum"));
    assert_eq!(
        appel(&app, "GET", &format!("/voyages/{id}"), Some(parent), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    // Tableau de la direction et de la comptabilité ; refusé au professeur.
    for jeton in [dir, compta] {
        let (st, t) = appel(&app, "GET", "/voyages/tableau", Some(jeton), None).await;
        assert_eq!(st, StatusCode::OK, "{t}");
        let _: ent_types::voyages::TableauDirectionDto =
            serde_json::from_value(t).expect("TableauDirectionDto");
    }
    assert_eq!(
        appel(&app, "GET", "/voyages/tableau", Some(prof), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );

    // Liste : le professeur tiers voit la sortie ouverte, sous forme minimale.
    let (st, l) = appel(
        &app,
        "GET",
        "/voyages?q=muséum",
        Some(cast.profs[1].token.as_str()),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let l: Vec<ent_types::voyages::SortieResumeDto> =
        serde_json::from_value(l).expect("SortieResumeDto");
    let vue = l
        .iter()
        .find(|s| s.id.to_string() == id)
        .expect("sortie visible du personnel");
    assert!(!vue.acces_detail && vue.porteur_nom.is_none());
}
