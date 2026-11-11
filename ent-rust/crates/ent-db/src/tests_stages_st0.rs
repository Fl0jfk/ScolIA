//! Socle 39, lot ST0 : correctifs RS-01 à RS-10 du module des stages
//! (`docs/socles/39_STAGES.md` §2.6). Tests contre la base locale de labo ; sans
//! `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use chrono::{Datelike, NaiveDate};
use domain_eleve::{CreerConventionStage, CreerEntreprise, DomainError, StageErreur};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast, sqlx::PgPool)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    Some((ent, cast, pool))
}

/// Jour de novembre de l'année scolaire courante : toujours dans l'année (paramétrée ou civile).
fn nov(jour: u32) -> NaiveDate {
    let auj = domain_eleve::horloge::jour_paris();
    let an = if auj.month() >= 9 { auj.year() } else { auj.year() - 1 };
    NaiveDate::from_ymd_opt(an, 11, jour).unwrap()
}

/// SIRET valide et unique à chaque appel (clé de Luhn calculée).
fn siret_unique() -> String {
    let base: String = Uuid::new_v4()
        .as_u128()
        .to_string()
        .chars()
        .take(13)
        .collect::<String>();
    let base = format!("{base:0<13}");
    (0..10)
        .map(|c| format!("{base}{c}"))
        .find(|s| domain_eleve::siret_cle_valide(s))
        .expect("une clé existe")
}

fn cmd_entreprise(cast: &LaboCast, acteur: Uuid, siret: Option<String>) -> CreerEntreprise {
    CreerEntreprise {
        etablissement_id: cast.etablissement_id,
        raison_sociale: "Weasleys' Wizard Wheezes".into(),
        siret,
        adresse: "93 Diagon Alley".into(),
        actor_user_id: acteur,
    }
}

fn cmd_stage(
    cast: &LaboCast,
    eleve: Uuid,
    entreprise: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
    acteur: Uuid,
) -> CreerConventionStage {
    CreerConventionStage {
        etablissement_id: cast.etablissement_id,
        eleve_id: eleve,
        entreprise_id: entreprise,
        debut,
        fin,
        tuteur_nom: "Fred Weasley".into(),
        actor_user_id: acteur,
    }
}

fn est_refus(r: &Result<impl std::fmt::Debug, EntDbError>, motif: StageErreur) -> bool {
    matches!(r, Err(EntDbError::Refus(_, m)) if *m == motif.message())
}

/// RS-01 : un stage créé est `declaree` (jamais `active`) et ne vaut pas convention signée ;
/// les anciennes lignes `active` sont converties ; aucune ligne ne peut rester `active`.
#[tokio::test]
async fn rs01_stage_declare_n_est_pas_une_convention_signee() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let admini = &cast.administratif;
    let org = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, None))
        .await
        .expect("organisme");
    let c = ent
        .creer_convention_stage_for_actor(
            "administratif",
            cmd_stage(&cast, cast.eleve_lea_id, org.id, nov(2), nov(6), admini.user_id),
        )
        .await
        .expect("stage");
    assert_eq!(c.statut, "declaree");
    assert!(!c.convention_signee());
    assert_eq!(c.mention(), Some("convention non signée dans ScolIA"));
    let (statut,): (String,) = sqlx::query_as("SELECT statut FROM ent_convention_stage WHERE id = $1")
        .bind(c.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(statut, "declaree");
    // La base refuse l'ancien mot : « active » n'existe plus.
    let r = sqlx::query("UPDATE ent_convention_stage SET statut = 'active' WHERE id = $1")
        .bind(c.id)
        .execute(&pool)
        .await;
    assert!(r.is_err(), "le statut `active` ne doit plus être accepté");
    // Rejeu de la conversion (migration rejouée à chaque démarrage) : sans effet sur `declaree`.
    ent.migrate().await.expect("rejeu");
    let relu = ent
        .list_conventions_eleve_for_actor(cast.etablissement_id, "administratif", admini.user_id, cast.eleve_lea_id)
        .await
        .expect("lecture");
    assert!(relu.iter().all(|x| x.statut == "declaree" && !x.convention_signee()));
}

/// RS-02 : droits D-017 `stages.*` ; le CPE ne crée plus ; le professeur ne crée rien ; lecture
/// bornée aux élèves suivis.
#[tokio::test]
async fn rs02_droits_par_action_et_cpe_ne_cree_plus() {
    let Some((ent, cast, _pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let admini = &cast.administratif;
    // Professeur : ni organisme ni stage.
    let r = ent
        .creer_entreprise_for_actor("prof", cmd_entreprise(&cast, cast.prof.user_id, None))
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let org = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, None))
        .await
        .expect("organisme");
    for (role, user) in [("prof", cast.prof.user_id), ("cpe", cast.cpe.user_id), ("compta", cast.compta.user_id)] {
        let r = ent
            .creer_convention_stage_for_actor(
                role,
                cmd_stage(&cast, cast.eleve_lea_id, org.id, nov(2), nov(6), user),
            )
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} ne crée pas de stage : {r:?}");
        let r = ent.list_entreprises_for_actor(cast.etablissement_id, user, role).await;
        // Le carnet d'organismes (organisme, adresse) n'est pas ouvert au professeur, au CPE ni à
        // la comptabilité (§21.2) : `stages.organismes.gerer` ou `stages.lire` à l'établissement.
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    let c = ent
        .creer_convention_stage_for_actor(
            "administratif",
            cmd_stage(&cast, cast.eleve_lea_id, org.id, nov(2), nov(6), admini.user_id),
        )
        .await
        .expect("le secrétariat déclare");
    // Lecture : le CPE n'a pas stages.lire (projection « en stage » au lot ST1).
    let r = ent
        .list_conventions_eleve_for_actor(cast.etablissement_id, "cpe", cast.cpe.user_id, cast.eleve_lea_id)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Direction : oui.
    let r = ent
        .list_conventions_eleve_for_actor(cast.etablissement_id, "direction", cast.direction.user_id, cast.eleve_lea_id)
        .await
        .expect("direction");
    assert!(r.iter().any(|x| x.id == c.id));
    // Professeur : son élève (4B, Harry) oui ; un élève d'une autre classe (5A, Draco) non.
    let r = ent
        .list_conventions_eleve_for_actor(cast.etablissement_id, "prof", cast.prof.user_id, cast.eleve_lea_id)
        .await;
    assert!(r.is_ok(), "{r:?}");
    let r = ent
        .list_conventions_eleve_for_actor(cast.etablissement_id, "prof", cast.prof.user_id, cast.eleve_tom_id)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Changer le statut : droit stages.preparer.
    for (role, user) in [("prof", cast.prof.user_id), ("cpe", cast.cpe.user_id)] {
        let r = ent
            .changer_statut_convention_stage_for_actor(cast.etablissement_id, role, user, c.id, 1, "annulee")
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
}

/// RS-03 : lecture famille par le lien — droit du lien, lien actif, interdiction, majorité.
#[tokio::test]
async fn rs03_lecture_famille_par_le_lien() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let admini = &cast.administratif;
    let org = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, None))
        .await
        .unwrap();
    for eleve in [cast.eleve_lea_id, cast.eleve_tom_id] {
        ent.creer_convention_stage_for_actor(
            "administratif",
            cmd_stage(&cast, eleve, org.id, nov(2), nov(6), admini.user_id),
        )
        .await
        .unwrap();
    }
    let lecture = |user: Uuid, eleve: Uuid| {
        let ent = &ent;
        async move { ent.list_conventions_eleve_for_actor(etab, "parent", user, eleve).await }
    };
    // Parent avec le droit du lien : oui.
    assert!(lecture(cast.parent_lea.user_id, cast.eleve_lea_id).await.is_ok());
    // Parent dont le lien ne porte pas le droit : 403 (Draco : `peut_voir_vs` décoché).
    let r = lecture(cast.parent_tom.user_id, cast.eleve_tom_id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Parent d'un autre enfant : 403.
    let r = lecture(cast.parent_lea.user_id, cast.eleve_tom_id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");

    // Interdiction active de communication visant cette personne : 403.
    let (personne,): (Uuid,) = sqlx::query_as(
        "SELECT personne_id FROM ent_eleve_lien_actif WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
    )
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(cast.parent_lea.user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let interdiction = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,$4, ARRAY['communication'], 'decision_justice', 'Ne rien communiquer', CURRENT_DATE - 1, $5)",
    )
    .bind(interdiction)
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(personne)
    .bind(cast.direction.user_id)
    .execute(&pool)
    .await
    .unwrap();
    let r = lecture(cast.parent_lea.user_id, cast.eleve_lea_id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "interdiction : {r:?}");
    // Interdiction levée : le droit revient (seul l'objet « communication » compte).
    sqlx::query("UPDATE ent_eleve_interdiction SET levee_le = now(), levee_par = $2 WHERE id = $1")
        .bind(interdiction)
        .bind(cast.direction.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(lecture(cast.parent_lea.user_id, cast.eleve_lea_id).await.is_ok());

    // Lien suspendu : 403.
    sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = now() WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(cast.eleve_lea_id)
        .bind(cast.parent_lea.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let r = lecture(cast.parent_lea.user_id, cast.eleve_lea_id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "lien suspendu : {r:?}");
    sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = NULL WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(cast.eleve_lea_id)
        .bind(cast.parent_lea.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(lecture(cast.parent_lea.user_id, cast.eleve_lea_id).await.is_ok());

    // Élève majeur : ses parents ne lisent rien (partage décidé par l'élève, absent ici).
    sqlx::query("UPDATE ent_eleve SET date_naissance = DATE '2000-01-01' WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(cast.eleve_lea_id)
        .execute(&pool)
        .await
        .unwrap();
    let r = lecture(cast.parent_lea.user_id, cast.eleve_lea_id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "élève majeur : {r:?}");
}

/// RS-04 et RS-05 : l'événement s'écrit dans la transaction du fait ; pas d'événement si
/// l'écriture échoue ; charge utile sans nom de tuteur.
#[tokio::test]
async fn rs04_rs05_evenement_dans_la_transaction_sans_donnee_de_tuteur() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let admini = &cast.administratif;
    let compte = |t: &'static str| {
        let pool = pool.clone();
        async move {
            let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = $2")
                .bind(etab)
                .bind(t)
                .fetch_one(&pool)
                .await
                .unwrap();
            n
        }
    };
    let org = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, Some(siret_unique())))
        .await
        .unwrap();
    assert_eq!(compte("stage.entreprise").await, 1);
    let c = ent
        .creer_convention_stage_for_actor(
            "administratif",
            cmd_stage(&cast, cast.eleve_lea_id, org.id, nov(2), nov(6), admini.user_id),
        )
        .await
        .unwrap();
    assert_eq!(compte("stage.convention").await, 1);
    let (payload,): (String,) = sqlx::query_as(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'stage.convention' AND aggregate_id = $2",
    )
    .bind(etab)
    .bind(c.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!payload.contains("Fred") && !payload.contains("Weasley"), "RS-05 : {payload}");
    assert!(payload.contains(&org.id.to_string()) && payload.contains("declaree"));
    let (payload_org,): (String,) = sqlx::query_as(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'stage.entreprise' AND aggregate_id = $2",
    )
    .bind(etab)
    .bind(org.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!payload_org.contains("Wizard") && !payload_org.contains(&org.siret.clone().unwrap()), "{payload_org}");
    // Écriture refusée (chevauchement) : aucun événement ne reste.
    let r = ent
        .creer_convention_stage_for_actor(
            "administratif",
            cmd_stage(&cast, cast.eleve_lea_id, org.id, nov(4), nov(10), admini.user_id),
        )
        .await;
    assert!(est_refus(&r, StageErreur::Chevauchement), "{r:?}");
    assert_eq!(compte("stage.convention").await, 1, "un fait refusé ne laisse pas d'événement");
    // SIRET en double : idem pour l'organisme.
    let r = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, org.siret.clone()))
        .await;
    assert!(est_refus(&r, StageErreur::SiretDoublon), "{r:?}");
    assert_eq!(compte("stage.entreprise").await, 1);
    // Changement de statut : événement dans la même transaction, sans donnée de personne.
    ent.changer_statut_convention_stage_for_actor(etab, "administratif", admini.user_id, c.id, 1, "cloturee")
        .await
        .unwrap();
    assert_eq!(compte("stage.convention_statut").await, 1);
    // Écriture qui échoue en base APRÈS le premier ordre (SQL direct dans une transaction) :
    // l'événement et le fait disparaissent ensemble.
    let mut tx = crate::tenant_tx::TenantTx::begin(&pool, etab).await.unwrap();
    {
        use crate::outbox::Publier;
        let conn: &mut sqlx::PgConnection = &mut tx;
        conn.publier(ent_events::MetierEventDraft {
            etablissement_id: etab,
            type_name: "stage.convention".into(),
            aggregate: ent_events::AggregateKind::ConventionStage,
            aggregate_id: Uuid::new_v4(),
            eleve_id: None,
            payload: serde_json::json!({}),
            actor_user_id: None,
        })
        .await
        .unwrap();
    }
    tx.rollback().await.unwrap();
    assert_eq!(compte("stage.convention").await, 1, "l'annulation retire aussi l'événement");
}

/// RS-06 : clés composées en RESTRICT, version et horodatage de modification.
#[tokio::test]
async fn rs06_rs07_cles_composees_restrict_et_version() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let autre = ent.seed_labo_cast().await.expect("autre établissement");
    let etab = cast.etablissement_id;
    let admini = &cast.administratif;
    let org = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, None))
        .await
        .unwrap();
    let c = ent
        .creer_convention_stage_for_actor(
            "administratif",
            cmd_stage(&cast, cast.eleve_lea_id, org.id, nov(2), nov(6), admini.user_id),
        )
        .await
        .unwrap();
    assert_eq!((c.version, c.modifie_le >= c.created_at), (1, true));

    // Élève d'un autre établissement : refusé par la clé composée.
    let r = sqlx::query(
        "INSERT INTO ent_convention_stage (id, etablissement_id, eleve_id, entreprise_id, debut, fin, tuteur_nom, created_by_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,'X',$7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(autre.eleve_lea_id)
    .bind(org.id)
    .bind(nov(10))
    .bind(nov(12))
    .bind(admini.user_id)
    .execute(&pool)
    .await;
    assert!(r.is_err(), "élève d'un autre établissement accepté");
    // Organisme d'un autre établissement : idem.
    let org_autre = ent
        .creer_entreprise_for_actor(
            "administratif",
            CreerEntreprise {
                etablissement_id: autre.etablissement_id,
                raison_sociale: "Autre".into(),
                siret: None,
                adresse: String::new(),
                actor_user_id: autre.administratif.user_id,
            },
        )
        .await
        .unwrap();
    let r = sqlx::query(
        "INSERT INTO ent_convention_stage (id, etablissement_id, eleve_id, entreprise_id, debut, fin, tuteur_nom, created_by_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,'X',$7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cast.eleve_tom_id)
    .bind(org_autre.id)
    .bind(nov(10))
    .bind(nov(12))
    .bind(admini.user_id)
    .execute(&pool)
    .await;
    assert!(r.is_err(), "organisme d'un autre établissement accepté");
    // Via le service : l'organisme d'un autre établissement est introuvable.
    let r = ent
        .creer_convention_stage_for_actor(
            "administratif",
            cmd_stage(&cast, cast.eleve_tom_id, org_autre.id, nov(10), nov(12), admini.user_id),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
    // RESTRICT : ni l'organisme ni l'élève ne se suppriment tant qu'un stage les référence.
    let r = sqlx::query("DELETE FROM ent_entreprise WHERE id = $1").bind(org.id).execute(&pool).await;
    assert!(r.is_err(), "RESTRICT organisme");
    let r = sqlx::query("DELETE FROM ent_eleve WHERE id = $1").bind(cast.eleve_lea_id).execute(&pool).await;
    assert!(r.is_err(), "RESTRICT élève");

    // RS-07 : verrou optimiste.
    let r = ent
        .changer_statut_convention_stage_for_actor(etab, "administratif", admini.user_id, c.id, 7, "annulee")
        .await;
    assert!(est_refus(&r, StageErreur::ConflitDeVersion), "{r:?}");
    let ok = ent
        .changer_statut_convention_stage_for_actor(etab, "administratif", admini.user_id, c.id, 1, "annulee")
        .await
        .expect("statut");
    assert_eq!((ok.statut.as_str(), ok.version), ("annulee", 2));
    assert!(ok.modifie_le >= c.modifie_le);
    // Un stage annulé ne redevient pas déclaré.
    let r = ent
        .changer_statut_convention_stage_for_actor(etab, "administratif", admini.user_id, c.id, 2, "declaree")
        .await;
    assert!(est_refus(&r, StageErreur::StatutInvalide), "{r:?}");
    // Un autre établissement ne voit pas le stage.
    let r = ent
        .changer_statut_convention_stage_for_actor(
            autre.etablissement_id,
            "administratif",
            autre.administratif.user_id,
            c.id,
            2,
            "cloturee",
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
    let r = ent
        .list_conventions_eleve_for_actor(autre.etablissement_id, "administratif", autre.administratif.user_id, cast.eleve_lea_id)
        .await;
    assert!(r.map(|l| l.is_empty()).unwrap_or(true), "cloisonnement d'établissement");
}

/// RS-08 : clé du SIRET, unicité par établissement, rapport de doublons.
#[tokio::test]
async fn rs08_siret_cle_et_unicite() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let autre = ent.seed_labo_cast().await.expect("autre établissement");
    let admini = &cast.administratif;
    let r = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, Some("12345678901234".into())))
        .await;
    assert!(est_refus(&r, StageErreur::SiretInvalide), "{r:?}");
    let s = siret_unique();
    let a = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, Some(s.clone())))
        .await
        .expect("SIRET valide");
    let r = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, Some(s.clone())))
        .await;
    assert!(est_refus(&r, StageErreur::SiretDoublon), "{r:?}");
    // Même SIRET dans un autre établissement : permis (carnets distincts).
    ent.creer_entreprise_for_actor(
        "administratif",
        CreerEntreprise {
            etablissement_id: autre.etablissement_id,
            raison_sociale: "Même enseigne".into(),
            siret: Some(s.clone()),
            adresse: String::new(),
            actor_user_id: autre.administratif.user_id,
        },
    )
    .await
    .expect("autre établissement");
    // Sans SIRET : plusieurs organismes permis.
    for _ in 0..2 {
        ent.creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, None))
            .await
            .expect("sans SIRET");
    }
    // La base applique aussi la clé et l'unicité (course ou écriture hors service).
    let r = sqlx::query(
        "INSERT INTO ent_entreprise (id, etablissement_id, raison_sociale, siret, created_by_user_id) VALUES ($1,$2,'Doublon',$3,$4)",
    )
    .bind(Uuid::new_v4())
    .bind(cast.etablissement_id)
    .bind(&s)
    .bind(admini.user_id)
    .execute(&pool)
    .await;
    assert!(r.is_err(), "unicité (établissement, SIRET) en base");
    let r = sqlx::query(
        "INSERT INTO ent_entreprise (id, etablissement_id, raison_sociale, siret, created_by_user_id) VALUES ($1,$2,'Faux',$3,$4)",
    )
    .bind(Uuid::new_v4())
    .bind(cast.etablissement_id)
    .bind("12345678901234")
    .bind(admini.user_id)
    .execute(&pool)
    .await;
    assert!(r.is_err(), "clé du SIRET en base");
    // La fonction SQL est identique au code Rust.
    for (siret, attendu) in [("73282932000074", true), ("73282932000075", false), ("35600000000079", true), ("35600000000078", false)] {
        let (ok,): (bool,) = sqlx::query_as("SELECT ent_siret_cle_valide($1)").bind(siret).fetch_one(&pool).await.unwrap();
        assert_eq!(ok, attendu, "{siret}");
        assert_eq!(domain_eleve::siret_cle_valide(siret), attendu, "{siret} (Rust)");
    }
    // Rapport de doublons : vide ici (l'unicité est posée), la fonction existe et se lit.
    let doublons: Vec<(Uuid, String, i64)> = sqlx::query_as("SELECT etablissement_id, siret, nombre FROM ent_stage_siret_doublons()")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(doublons.iter().all(|(e, _, _)| *e != cast.etablissement_id));
    let _ = a;
}

/// RS-09 : pas de chevauchement de deux stages d'un élève ; dates dans l'année scolaire.
#[tokio::test]
async fn rs09_chevauchement_et_annee_scolaire() {
    let Some((ent, cast, pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let admini = &cast.administratif;
    let org = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, None))
        .await
        .unwrap();
    let nouveau = |debut, fin| cmd_stage(&cast, cast.eleve_lea_id, org.id, debut, fin, admini.user_id);
    let premier = ent.creer_convention_stage_for_actor("administratif", nouveau(nov(9), nov(13))).await.expect("premier");
    // Recouvrement partiel, inclusion, même jour : refusés.
    for (d, f) in [(nov(12), nov(16)), (nov(10), nov(11)), (nov(5), nov(9)), (nov(13), nov(13))] {
        let r = ent.creer_convention_stage_for_actor("administratif", nouveau(d, f)).await;
        assert!(est_refus(&r, StageErreur::Chevauchement), "{d}..{f} : {r:?}");
    }
    // Un autre élève, ou des dates contiguës sans recouvrement : permis.
    ent.creer_convention_stage_for_actor("administratif", cmd_stage(&cast, cast.eleve_tom_id, org.id, nov(9), nov(13), admini.user_id))
        .await
        .expect("autre élève");
    ent.creer_convention_stage_for_actor("administratif", nouveau(nov(16), nov(20))).await.expect("contigu");
    // Un stage annulé ne bloque plus.
    ent.changer_statut_convention_stage_for_actor(etab, "administratif", admini.user_id, premier.id, 1, "annulee")
        .await
        .unwrap();
    ent.creer_convention_stage_for_actor("administratif", nouveau(nov(9), nov(13))).await.expect("après annulation");
    // La base refuse aussi le chevauchement (course entre deux requêtes).
    let r = sqlx::query(
        "INSERT INTO ent_convention_stage (id, etablissement_id, eleve_id, entreprise_id, debut, fin, tuteur_nom, created_by_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,'X',$7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cast.eleve_lea_id)
    .bind(org.id)
    .bind(nov(10))
    .bind(nov(11))
    .bind(admini.user_id)
    .execute(&pool)
    .await;
    assert!(r.is_err(), "exclusion btree_gist");
    // Dates hors année scolaire : à cheval sur deux années (31 août → 1er septembre).
    let auj = domain_eleve::horloge::jour_paris();
    let an = if auj.month() >= 9 { auj.year() } else { auj.year() - 1 };
    let r = ent
        .creer_convention_stage_for_actor(
            "administratif",
            cmd_stage(
                &cast,
                cast.eleve_tom_id,
                org.id,
                NaiveDate::from_ymd_opt(an + 1, 8, 28).unwrap(),
                NaiveDate::from_ymd_opt(an + 1, 9, 4).unwrap(),
                admini.user_id,
            ),
        )
        .await;
    assert!(est_refus(&r, StageErreur::DatesHorsAnnee), "{r:?}");
}

/// RS-10 : chaque cause de refus a son propre message (l'interface sait dire ce qui ne va pas).
#[tokio::test]
async fn rs10_erreurs_typees_distinctes() {
    let Some((ent, cast, _pool)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let admini = &cast.administratif;
    let org = ent
        .creer_entreprise_for_actor("administratif", cmd_entreprise(&cast, admini.user_id, None))
        .await
        .unwrap();
    let mut messages = std::collections::HashSet::new();
    let mut note = |r: Result<(), EntDbError>, motif: StageErreur| {
        assert!(
            matches!(&r, Err(EntDbError::Refus(_, m)) if *m == motif.message())
                || matches!(&r, Err(EntDbError::Domain(DomainError::Stage(e))) if *e == motif),
            "{motif:?} : {r:?}"
        );
        messages.insert(motif.code());
    };
    let mut c = cmd_entreprise(&cast, admini.user_id, Some("12345678901234".into()));
    note(ent.creer_entreprise_for_actor("administratif", c.clone()).await.map(|_| ()), StageErreur::SiretInvalide);
    c.siret = None;
    c.raison_sociale = "  ".into();
    note(ent.creer_entreprise_for_actor("administratif", c).await.map(|_| ()), StageErreur::RaisonSocialeInvalide);
    let mut s = cmd_stage(&cast, cast.eleve_lea_id, org.id, nov(9), nov(13), admini.user_id);
    s.tuteur_nom = String::new();
    note(ent.creer_convention_stage_for_actor("administratif", s.clone()).await.map(|_| ()), StageErreur::TuteurManquant);
    s.tuteur_nom = "Fred".into();
    s.fin = nov(2);
    note(ent.creer_convention_stage_for_actor("administratif", s).await.map(|_| ()), StageErreur::DatesInvalides);
    assert_eq!(messages.len(), 4);
}
