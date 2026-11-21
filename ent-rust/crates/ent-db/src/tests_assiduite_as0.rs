//! Socle 12, lot AS0 (correctifs urgents) : DA-01 à DA-03, DA-05, DA-06, DA-07, DA-22.
//! Réf. : `docs/socles/12_ASSIDUITE.md` §2.2, §23-24. Sans `ENT_DATABASE_URL` : ignorés par
//! absence de base (le test échoue en CI, qui fournit Postgres).

use super::*;
use crate::appel_repo::LigneEtat;
use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};
use domain_eleve::{today_utc, DeclareAbsence, DeclarerRetard};
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { e.install_brain_p0(); }).await;
    Some((ent, cast))
}

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

/// Jour ouvré (lun-ven) à J+`n` de l'établissement, à partir de J+150 (hors jeu de labo).
fn jour_ouvre(n: i64) -> NaiveDate {
    let mut d = today_utc() + Duration::days(150);
    let mut k = 0;
    loop {
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            if k == n {
                return d;
            }
            k += 1;
        }
        d += Duration::days(1);
    }
}

/// Appel de 4B (Rogue) le `jour`, `lea` marqué absent, clôturé. Renvoie l'id de l'appel.
async fn appel_lea_absent(ent: &PgEnt, cast: &LaboCast, jour: NaiveDate, debut: (u32, u32), fin: (u32, u32)) -> Uuid {
    let etab = cast.etablissement_id;
    let c = ent
        .seed_creneau_du_jour(etab, "4B", "AS0", cast.prof.user_id, jour, t(debut.0, debut.1), t(fin.0, fin.1), None)
        .await
        .expect("créneau");
    let (appel, _) = ent
        .ouvrir_appel(etab, c.id, jour, cast.prof.user_id, Role::Prof.as_str())
        .await
        .expect("ouvrir");
    ent.marquer_appel_ligne(etab, appel.id, cast.prof.user_id, "prof", cast.eleve_lea_id, LigneEtat::Absent)
        .await
        .expect("marquer");
    ent.clore_appel(etab, appel.id, cast.prof.user_id, "prof", None).await.expect("clore");
    appel.id
}

async fn compter(ent: &PgEnt, sql: &str, a: Uuid, b: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql).bind(a).bind(b).fetch_one(ent.pool_verif()).await.unwrap();
    n
}

/// Une absence `a_traiter` de `lea` le `jour`, déclarée par le CPE (pas par un professeur).
async fn absence_existante(ent: &PgEnt, cast: &LaboCast, jour: NaiveDate) -> Uuid {
    ent.declare_absence(DeclareAbsence {
        etablissement_id: cast.etablissement_id,
        eleve_id: cast.eleve_lea_id,
        debut: jour,
        fin: jour,
        motif: "maladie".into(),
        actor_user_id: Some(cast.cpe.user_id.to_string()),
    })
    .await
    .expect("absence")
    .id
}

// ─── DA-01 : un professeur ne déclare pas d'absence (R-AS24) ────────────────

#[test]
fn da01_le_role_prof_n_a_pas_le_droit_de_declarer_ni_de_remettre_present() {
    assert!(!Role::Prof.can_declare_absence_staff());
    assert!(!Role::Prof.can_annuler_absence_vers_present());
    for r in [Role::Cpe, Role::Direction, Role::Administratif, Role::Admin] {
        assert!(r.can_declare_absence_staff() && r.can_annuler_absence_vers_present(), "{r:?}");
    }
    // Il garde l'appel de sa séance.
    assert!(Role::Prof.can_faire_appel());
}

#[tokio::test]
async fn da01_professeur_ne_declare_pas_d_absence_meme_pour_son_eleve() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (rogue, mcg) = (cast.profs[0].user_id, cast.profs[1].user_id);
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    for (p, e) in [(rogue, harry), (rogue, draco), (mcg, draco), (mcg, harry)] {
        assert!(!ent.actor_can_declare_absence(etab, p, "prof", e).await.unwrap());
    }
    assert!(ent.actor_can_declare_absence(etab, cast.cpe.user_id, "cpe", draco).await.unwrap());
}

// ─── DA-02 : un professeur n'efface pas une absence ────────────────────────

#[tokio::test]
async fn da02_professeur_ne_remet_pas_present_une_absence() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let id = absence_existante(&ent, &cast, jour_ouvre(0)).await;
    for prof in [cast.profs[0].user_id, cast.profs[1].user_id] {
        let r = ent
            .annuler_absence_vers_present_for_actor(etab, prof, "prof", id, Some("erreur".into()))
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    }
    let statut: (String,) = sqlx::query_as("SELECT statut FROM ent_absence WHERE id = $1")
        .bind(id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(statut.0, "a_traiter", "l'absence est intacte");
    // La vie scolaire le peut.
    ent.annuler_absence_vers_present_for_actor(etab, cast.cpe.user_id, "cpe", id, Some("erreur d'appel".into()))
        .await
        .unwrap();
}

// ─── DA-03 : retard borné (périmètre, jour) ────────────────────────────────

fn retard(etab: Uuid, eleve: Uuid, jour: NaiveDate, auteur: Uuid) -> DeclarerRetard {
    DeclarerRetard {
        etablissement_id: etab,
        eleve_id: eleve,
        jour,
        minutes: Some(5),
        motif: None,
        source_appel_id: None,
        actor_user_id: Some(auteur),
    }
}

#[tokio::test]
async fn da03_retard_du_professeur_limite_a_ses_eleves_et_au_jour_meme() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let auj = ent.aujourdhui_etablissement(etab).await.unwrap();

    // Hors de ses groupes : refusé.
    let r = ent.declarer_retard_for_actor("prof", retard(etab, draco, auj, rogue)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Veille et lendemain : refusés, même pour son élève.
    for j in [auj - Duration::days(1), auj + Duration::days(1)] {
        let r = ent.declarer_retard_for_actor("prof", retard(etab, harry, j, rogue)).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{j} : {r:?}");
    }
    // Son élève, le jour même : accepté.
    ent.declarer_retard_for_actor("prof", retard(etab, harry, auj, rogue)).await.unwrap();
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_retard WHERE etablissement_id = $1 AND eleve_id = $2", etab, harry).await,
        1
    );
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_retard WHERE etablissement_id = $1 AND eleve_id = $2", etab, draco).await,
        0
    );
}

#[tokio::test]
async fn da03_aucun_retard_a_une_date_future_meme_pour_la_vie_scolaire() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let auj = ent.aujourdhui_etablissement(etab).await.unwrap();
    let r = ent
        .declarer_retard_for_actor("cpe", retard(etab, cast.eleve_lea_id, auj + Duration::days(3), cast.cpe.user_id))
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // La vie scolaire garde la saisie rétroactive (retard constaté à l'accueil).
    ent.declarer_retard_for_actor("cpe", retard(etab, cast.eleve_lea_id, auj - Duration::days(1), cast.cpe.user_id))
        .await
        .unwrap();
}

// ─── DA-05 : une journée d'absence = une absence ───────────────────────────

#[tokio::test]
async fn da05_deux_appels_le_meme_jour_donnent_une_seule_absence() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let jour = jour_ouvre(1);
    let a1 = appel_lea_absent(&ent, &cast, jour, (8, 0), (9, 0)).await;
    let a2 = appel_lea_absent(&ent, &cast, jour, (9, 0), (10, 0)).await;
    let a3 = appel_lea_absent(&ent, &cast, jour, (10, 0), (11, 0)).await;
    let ce_jour = |j: NaiveDate| format!("SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 AND debut = '{j}'");
    assert_eq!(compter(&ent, &ce_jour(jour), etab, lea).await, 1, "une seule absence pour la journée");
    // Les trois séances sont rattachées à cette absence.
    let (n, d): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(DISTINCT absence_id) FROM ent_absence_seance
          WHERE etablissement_id = $1 AND appel_id = ANY($2)",
    )
    .bind(etab)
    .bind(vec![a1, a2, a3])
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((n, d), (3, 1));
    // Un seul événement « absence déclarée » (donc un seul avis immédiat) pour la journée.
    let (absence,): (Uuid,) = sqlx::query_as("SELECT absence_id FROM ent_absence_seance WHERE appel_id = $1")
        .bind(a1).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'eleve.absence_declared'", etab, absence).await,
        1
    );
    // Un autre jour : une nouvelle absence.
    let autre = jour_ouvre(2);
    appel_lea_absent(&ent, &cast, autre, (8, 0), (9, 0)).await;
    assert_eq!(compter(&ent, &ce_jour(autre), etab, lea).await, 1);
}

#[tokio::test]
async fn da05_l_appel_se_rattache_a_l_absence_deja_declaree() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let jour = jour_ouvre(3);
    let existante = absence_existante(&ent, &cast, jour).await;
    let appel = appel_lea_absent(&ent, &cast, jour, (8, 0), (9, 0)).await;
    assert_eq!(
        compter(&ent, &format!("SELECT count(*) FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 AND debut = '{jour}'"), etab, cast.eleve_lea_id).await,
        1
    );
    let (id,): (Uuid,) = sqlx::query_as("SELECT absence_id FROM ent_absence_seance WHERE appel_id = $1")
        .bind(appel).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(id, existante);
}

// ─── DA-06 : auteur de la validation et de la modification ──────────────────

#[tokio::test]
async fn da06_l_appel_garde_l_auteur_de_la_validation_et_des_modifications() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let appel = appel_lea_absent(&ent, &cast, jour_ouvre(4), (8, 0), (9, 0)).await;
    let (par, le): (Option<Uuid>, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT valide_par, valide_le FROM ent_appel WHERE id = $1")
            .bind(appel).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(par, Some(cast.prof.user_id));
    assert!(le.is_some());
    let (par, le): (Option<Uuid>, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "SELECT modifie_par, modifie_le FROM ent_appel_ligne WHERE appel_id = $1 AND eleve_id = $2",
    )
    .bind(appel).bind(cast.eleve_lea_id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(par, Some(cast.prof.user_id));
    assert!(le.is_some());
    let _ = etab;
}

// ─── DA-07 : le seuil annuel n'écrit plus aux familles ─────────────────────

#[tokio::test]
async fn da07_le_seuil_annuel_ne_cree_ni_fil_ni_courrier_aux_familles() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let fils_avant = compter(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = 'msg.fil_ouvert' AND $2 = $2", etab, lea).await;
    for n in 0..11 {
        appel_lea_absent(&ent, &cast, jour_ouvre(10 + n), (8, 0), (9, 0)).await;
    }
    // Le signal interne existe (alerte pour la vie scolaire)…
    let (signaux, fils): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(fil_alerte_id) FROM ent_absence_signal
          WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(etab).bind(lea).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(signaux >= 1, "signal interne conservé");
    // … mais aucun fil de messagerie ne part vers la famille.
    assert_eq!(fils, 0, "aucun fil d'alerte famille");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = 'msg.fil_ouvert' AND $2 = $2", etab, lea).await,
        fils_avant
    );
}

// ─── DA-22 : déclarations en double ────────────────────────────────────────

#[tokio::test]
async fn da22_trois_declarations_identiques_n_en_font_qu_une() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    let mk = || DeclarerAbsence {
        etablissement_id: etab,
        eleve_id: lea,
        debut_jour: jour,
        debut_heure: None,
        fin_jour: jour,
        fin_heure: None,
        motif: "maladie".into(),
        commentaire: None,
        declarant_nom: Some("Mme Granger".into()),
        declarant_qualite: Some("mère".into()),
        piece_document_id: None,
    };
    let cmds = [mk(), mk(), mk()];
    let mut ids = Vec::new();
    for c in cmds {
        ids.push(ent.declarer_absence_accueil(cast.administratif.user_id, "administratif", c).await.unwrap().id);
    }
    assert!(ids.iter().all(|i| *i == ids[0]), "même déclaration renvoyée");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_absence_declaration WHERE etablissement_id = $1 AND eleve_id = $2", etab, lea).await,
        1
    );
    // Une période différente reste une autre déclaration.
    let mut autre = mk();
    autre.fin_jour = jour + Duration::days(1);
    let d = ent.declarer_absence_accueil(cast.administratif.user_id, "administratif", autre).await.unwrap();
    assert_ne!(d.id, ids[0]);
}
