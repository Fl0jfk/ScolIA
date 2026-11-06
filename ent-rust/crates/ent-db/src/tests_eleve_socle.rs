//! Socle 2 (l'élève), lot E0 : correctifs. Scénarios contre la base locale de test
//! (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés.

use super::*;
use domain_eleve::{CreateEleve, RechercherHomonymes, UpdateIdentite};
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn nouvel_eleve(etab: Uuid, nom: &str, acteur: Uuid) -> CreateEleve {
    CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: "Zélie".into(),
        ine: None,
        sexe: Some("F".into()),
        date_naissance: chrono::NaiveDate::from_ymd_opt(2010, 3, 14),
        numero_badge: None,
        redoublant: false,
        classe: Some("6A".into()),
        regime: None,
        mef: None,
        site: None,
        date_entree: None,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: Some(acteur.to_string()),
    }
}

fn unique(prefixe: &str) -> String {
    format!("{prefixe}{}", &Uuid::new_v4().simple().to_string()[..8])
}

async fn un_i64(ent: &PgEnt, sql: &str, a: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql).bind(a).fetch_one(ent.pool()).await.unwrap();
    n
}

/// E0 : la recherche d'homonymes ne donne la date de naissance qu'aux profils qui
/// y ont droit, et la date fournie par un profil réduit n'entre pas dans le score.
#[tokio::test]
async fn homonymes_minimises_selon_le_profil() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let nom = unique("Homon");
    let e = ent
        .create_eleve(nouvel_eleve(etab, &nom, cast.administratif.user_id))
        .await
        .unwrap();
    let cherche = |date| RechercherHomonymes {
        etablissement_id: etab,
        nom: nom.clone(),
        prenom: "Zélie".into(),
        date_naissance: date,
    };
    let date = e.date_naissance;
    // Gestionnaire : date visible.
    let c = ent
        .rechercher_homonymes_for_actor(etab, Role::Administratif.as_str(), cherche(date))
        .await
        .unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].date_naissance, date);
    let score_complet = c[0].score;
    // CPE (niveau réduit) : aucune date en sortie, et la date fournie est ignorée
    // (même score avec ou sans date : pas d'oracle de date de naissance).
    for fournie in [date, chrono::NaiveDate::from_ymd_opt(1999, 1, 1), None] {
        let c = ent
            .rechercher_homonymes_for_actor(etab, Role::Cpe.as_str(), cherche(fournie))
            .await
            .unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].date_naissance, None, "date de naissance exposée au CPE");
        assert!(c[0].score <= score_complet);
    }
    let sans = ent
        .rechercher_homonymes_for_actor(etab, Role::Cpe.as_str(), cherche(None))
        .await
        .unwrap()[0]
        .score;
    let avec = ent
        .rechercher_homonymes_for_actor(etab, Role::Cpe.as_str(), cherche(date))
        .await
        .unwrap()[0]
        .score;
    assert_eq!(sans, avec);
    // Infirmerie (niveau santé) : voit la date.
    let c = ent
        .rechercher_homonymes_for_actor(etab, Role::Infirmerie.as_str(), cherche(date))
        .await
        .unwrap();
    assert_eq!(c[0].date_naissance, date);
}

/// E0 : création = élève + période + journal + événement, sans état civil dans
/// l'événement ni le journal.
#[tokio::test]
async fn creation_ecrit_journal_et_evenement_sans_etat_civil() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let nom = unique("Creat");
    let e = ent
        .create_eleve(nouvel_eleve(etab, &nom, cast.administratif.user_id))
        .await
        .unwrap();
    let (payload,): (serde_json::Value,) = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.created'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let texte = payload.to_string();
    assert!(!texte.contains(&nom) && !texte.contains("Zélie"), "état civil dans l'événement : {texte}");
    assert_eq!(
        un_i64(
            &ent,
            "SELECT count(*) FROM ent_journal_modif WHERE eleve_id = $1 AND action = 'eleve_cree'",
            e.id
        )
        .await,
        1
    );
    assert_eq!(
        un_i64(&ent, "SELECT count(*) FROM ent_eleve_periode WHERE eleve_id = $1", e.id).await,
        1
    );
}

/// E0 : si le journal échoue, rien n'est créé (plus de `let _ =` sur le journal) —
/// ni élève, ni période, ni événement.
#[tokio::test]
async fn creation_defaite_si_le_journal_echoue() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let nom = unique("Rollb");
    sqlx::query(
        "CREATE OR REPLACE FUNCTION ent_test_refuse_journal() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN IF NEW.action IN ('eleve_cree','identite_modifiee') THEN
           RAISE EXCEPTION 'journal refusé (test)'; END IF; RETURN NEW; END $$",
    )
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query("DROP TRIGGER IF EXISTS ent_test_refuse_journal ON ent_journal_modif")
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "CREATE TRIGGER ent_test_refuse_journal BEFORE INSERT ON ent_journal_modif
         FOR EACH ROW EXECUTE FUNCTION ent_test_refuse_journal()",
    )
    .execute(ent.pool())
    .await
    .unwrap();
    let resultat = ent
        .create_eleve(nouvel_eleve(etab, &nom, cast.administratif.user_id))
        .await;
    let (restant,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve WHERE nom = $1")
        .bind(&nom)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    // La mise à jour d'identité doit, elle aussi, échouer sans effet.
    let existant = cast.eleve_lea_id;
    let nom_de = |ent: &PgEnt| {
        let pool = ent.pool().clone();
        async move {
            let (n,): (String,) = sqlx::query_as("SELECT nom FROM ent_eleve WHERE id = $1")
                .bind(existant)
                .fetch_one(&pool)
                .await
                .unwrap();
            n
        }
    };
    let avant = nom_de(&ent).await;
    let modif = ent
        .update_identite(UpdateIdentite {
            etablissement_id: etab,
            eleve_id: existant,
            nom: Some(unique("Chg")),
            prenom: None,
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: None,
            photo_key: None,
            date_entree: None,
            actor_user_id: Some(cast.administratif.user_id.to_string()),
        })
        .await;
    let apres = nom_de(&ent).await;
    // Nettoyage avant toute assertion (le trigger est global).
    sqlx::query("DROP TRIGGER IF EXISTS ent_test_refuse_journal ON ent_journal_modif")
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(resultat.is_err(), "la création doit échouer si le journal échoue");
    assert_eq!(restant, 0, "élève créé malgré l'échec du journal");
    assert!(modif.is_err(), "la modification doit échouer si le journal échoue");
    assert_eq!(avant, apres, "identité modifiée malgré l'échec du journal");
}

/// E0 : la modification d'identité ne publie et ne journalise que des NOMS de champs.
#[tokio::test]
async fn identite_evenement_et_journal_sans_valeurs() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let e = ent
        .create_eleve(nouvel_eleve(etab, &unique("Ident"), cast.administratif.user_id))
        .await
        .unwrap();
    let nouveau = unique("Nouveau");
    ent.update_identite(UpdateIdentite {
        etablissement_id: etab,
        eleve_id: e.id,
        nom: Some(nouveau.clone()),
        prenom: None,
        ine: None,
        sexe: None,
        date_naissance: Some(chrono::NaiveDate::from_ymd_opt(2009, 12, 1)),
        numero_badge: None,
        redoublant: None,
        photo_key: None,
        date_entree: None,
        actor_user_id: Some(cast.administratif.user_id.to_string()),
    })
    .await
    .unwrap();
    let (payload,): (serde_json::Value,) = sqlx::query_as(
        "SELECT payload FROM ent_metier_event WHERE eleve_id = $1 AND type = 'eleve.identity_changed'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(payload["champs"], serde_json::json!(["nom", "date_naissance"]));
    let (journal, libelle): (serde_json::Value, String) = sqlx::query_as(
        "SELECT payload, libelle FROM ent_journal_modif WHERE eleve_id = $1 AND action = 'identite_modifiee'",
    )
    .bind(e.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    for t in [payload.to_string(), journal.to_string(), libelle] {
        assert!(!t.contains(&nouveau) && !t.contains("2009"), "valeur d'identité en clair : {t}");
    }
}

/// E0/M1 : les données ne peuvent être rattachées qu'à un élève du MÊME établissement.
#[tokio::test]
async fn cles_composees_refusent_l_autre_etablissement_et_l_inconnu() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let sql = "INSERT INTO ent_eleve_periode (id, etablissement_id, eleve_id, kind, valeur, date_debut)
               VALUES (gen_random_uuid(), $1, $2, 'classe', '6A', now()::date)";
    // Élève de l'établissement, mais avec l'identifiant d'un autre établissement.
    let croise = sqlx::query(sql)
        .bind(Uuid::new_v4())
        .bind(cast.eleve_lea_id)
        .execute(ent.pool())
        .await;
    assert!(croise.is_err(), "rattachement inter-établissements accepté");
    // Élève inexistant.
    let inconnu = sqlx::query(sql)
        .bind(cast.etablissement_id)
        .bind(Uuid::new_v4())
        .execute(ent.pool())
        .await;
    assert!(inconnu.is_err(), "rattachement à un élève inexistant accepté");
    // Aucun orphelin non traité n'est signalé sur la base de test.
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_orphelin WHERE traite_le IS NULL")
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
}
