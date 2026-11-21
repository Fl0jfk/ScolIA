//! Recette personas 1 (2026-11-07) : tests d'attaque issus de docs/recette/RECETTE_PERSONAS_1.md
//! contre la base locale de test (feature `labo`). Sans `ENT_DATABASE_URL`, ignorés.

use super::*;
use uuid::Uuid;

const MDP: &str = "Recette-Personas-Pass-1!";

async fn labo() -> Option<(PgEnt, LaboCast, sqlx::PgPool)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    Some((ent, cast, pool))
}

/// Inscrit un professeur sans aucune classe ni groupe.
async fn prof_sans_classe(ent: &PgEnt, etab: Uuid) -> Uuid {
    let email = format!("rp1-prof-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(&email, MDP, None, etab, "prof")
        .await
        .expect("inscription prof")
        .id
}

/// RP1-C01 / D-022 : un professeur n'exporte jamais (LSL, LSUN, Notanet, Parcoursup) ;
/// un administratif exporte.
#[tokio::test]
async fn rp1_c01_professeur_n_exporte_jamais() {
    let Some((ent, cast, _pool)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let classe = cast.eleves[0].classe.clone();
    let periode = ent.list_note_periodes(etab).await.unwrap()[0].id;
    let jour = domain_eleve::today_utc();
    let sans_classe = prof_sans_classe(&ent, etab).await;
    let prof_classe = cast.profs[0].user_id;
    assert!(ent
        .prof_suit_eleve(etab, prof_classe, cast.eleves[0].id)
        .await
        .unwrap());

    let lsl = || domain_eleve::ExporterLsl {
        etablissement_id: etab,
        periode_id: periode,
        classe: Some(classe.clone()),
        jour_reference: jour,
    };
    let lsun = || domain_eleve::ExporterLsun {
        etablissement_id: etab,
        periode_id: periode,
        classe: Some(classe.clone()),
        jour_reference: jour,
    };
    let notanet = || domain_eleve::ExporterNotanet {
        etablissement_id: etab,
        periode_id: periode,
        classe: classe.clone(),
    };
    let parcoursup = || domain_eleve::ExporterParcoursup {
        etablissement_id: etab,
        annee_scolaire: "2026-2027".into(),
        classe: classe.clone(),
    };

    for (nom, prof) in [
        ("prof de la classe", prof_classe),
        ("prof sans classe", sans_classe),
    ] {
        let r = ent.exporter_lsl_csv_for_actor("prof", prof, lsl()).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "LSL {nom} : {r:?}");
        let r = ent.exporter_lsun_csv_for_actor("prof", prof, lsun()).await;
        assert!(
            matches!(r, Err(EntDbError::Forbidden)),
            "LSUN {nom} : {r:?}"
        );
        let r = ent
            .exporter_notanet_csv_for_actor("prof", prof, notanet(), jour)
            .await;
        assert!(
            matches!(r, Err(EntDbError::Forbidden)),
            "Notanet {nom} : {r:?}"
        );
        let r = ent
            .exporter_parcoursup_csv_for_actor("prof", prof, parcoursup(), jour)
            .await;
        assert!(
            matches!(r, Err(EntDbError::Forbidden)),
            "Parcoursup {nom} : {r:?}"
        );
    }

    let adm = cast.administratif.user_id;
    let r = ent
        .exporter_lsl_csv_for_actor("administratif", adm, lsl())
        .await;
    assert!(
        !matches!(r, Err(EntDbError::Forbidden)),
        "LSL administratif : {r:?}"
    );
    let r = ent
        .exporter_lsun_csv_for_actor("administratif", adm, lsun())
        .await;
    assert!(
        !matches!(r, Err(EntDbError::Forbidden)),
        "LSUN administratif : {r:?}"
    );
    let r = ent
        .exporter_notanet_csv_for_actor("administratif", adm, notanet(), jour)
        .await;
    assert!(
        !matches!(r, Err(EntDbError::Forbidden)),
        "Notanet administratif : {r:?}"
    );
    let r = ent
        .exporter_parcoursup_csv_for_actor("administratif", adm, parcoursup(), jour)
        .await;
    assert!(
        !matches!(r, Err(EntDbError::Forbidden)),
        "Parcoursup administratif : {r:?}"
    );
}

/// RP1-C09 / RF-22 : un élève mineur ne déclare pas sa propre absence ; un majeur le peut.
#[tokio::test]
async fn rp1_c09_eleve_mineur_ne_declare_pas_son_absence() {
    let Some((ent, cast, pool)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let harry = &cast.eleves[0];
    let compte = harry.compte.as_ref().expect("compte élève de Harry");
    let r = ent
        .actor_can_declare_absence(etab, compte.user_id, "eleve", harry.id)
        .await
        .unwrap();
    assert!(
        !r,
        "un élève mineur ne doit pas pouvoir déclarer son absence"
    );

    // Un élève majeur (date de naissance posée en base) reste accepté.
    let majeur = cast
        .eleves
        .iter()
        .skip(1)
        .find(|e| e.compte.is_some())
        .expect("autre élève avec compte");
    let majeur_compte = majeur.compte.as_ref().unwrap();
    let ancienne: Option<chrono::NaiveDate> =
        sqlx::query_scalar("SELECT date_naissance FROM ent_eleve WHERE id = $1")
            .bind(majeur.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    // E2 : l'identité ne s'écrit en direct que via la projection (`ent.identite_projection`).
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.identite_projection', 'on', true)").execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE ent_eleve SET date_naissance = DATE '2000-01-01' WHERE id = $1")
        .bind(majeur.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let r = ent
        .actor_can_declare_absence(etab, majeur_compte.user_id, "eleve", majeur.id)
        .await;
    // Restaure la date avant toute assertion.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.identite_projection', 'on', true)").execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE ent_eleve SET date_naissance = $2 WHERE id = $1")
        .bind(majeur.id)
        .bind(ancienne)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(r.unwrap(), "un élève majeur déclare sa propre absence");
    // Et jamais pour un autre élève.
    let r = ent
        .actor_can_declare_absence(etab, majeur_compte.user_id, "eleve", harry.id)
        .await
        .unwrap();
    assert!(!r);
}

/// RP1-C05 / RF-15 : un professeur qui ne suit pas l'élève ne liste pas ses documents ;
/// celui qui le suit ne voit pas le titre d'un document restreint inaccessible ; la
/// direction voit les titres.
#[tokio::test]
async fn rp1_c05_documents_hors_perimetre_et_titre_restreint() {
    let Some((ent, cast, _pool)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let harry = cast.eleves[0].id;
    let titre = format!("PAP confidentiel {}", Uuid::new_v4().simple());
    let doc = ent
        .deposer_document(domain_eleve::DeposerDocument {
            etablissement_id: etab,
            eleve_id: harry,
            tiroir: domain_eleve::DocTiroir::Scolaire,
            titre: titre.clone(),
            code: None,
            mime_type: Some("application/pdf".into()),
            storage_key: Some(format!("lab/rp1/{}.pdf", Uuid::new_v4().simple())),
            confidentialite: domain_eleve::DocConfidentialite::Restreint,
            actor_user_id: cast.direction.user_id,
        })
        .await
        .expect("dépôt document");

    // Prof sans classe : refusé.
    let sans_classe = prof_sans_classe(&ent, etab).await;
    let r = ent
        .list_documents_meta_eleve_for_actor(etab, sans_classe, "prof", harry)
        .await;
    assert!(
        matches!(r, Err(EntDbError::Forbidden)),
        "prof sans classe : {r:?}"
    );

    // Prof qui suit Harry : liste visible, titre restreint masqué, id conservé.
    let suit = cast.profs[0].user_id;
    assert!(ent.prof_suit_eleve(etab, suit, harry).await.unwrap());
    let liste = ent
        .list_documents_meta_eleve_for_actor(etab, suit, "prof", harry)
        .await
        .expect("prof qui suit l'élève");
    let (d, accessible) = liste
        .iter()
        .find(|(d, _)| d.id == doc.id)
        .expect("document présent (id conservé)");
    assert!(!accessible);
    assert_eq!(d.titre, "Document à accès restreint");
    for (d, accessible) in &liste {
        assert!(
            *accessible
                || d.confidentialite == domain_eleve::DocConfidentialite::Standard
                || d.titre == "Document à accès restreint",
            "titre fuité : {}",
            d.titre
        );
    }

    // Direction : titres en clair.
    let liste = ent
        .list_documents_meta_eleve_for_actor(etab, cast.direction.user_id, "direction", harry)
        .await
        .expect("direction");
    let (d, accessible) = liste.iter().find(|(d, _)| d.id == doc.id).unwrap();
    assert!(accessible);
    assert_eq!(d.titre, titre);
}
