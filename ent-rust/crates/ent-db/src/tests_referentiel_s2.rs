//! S2-2 contre la base locale de test (feature `labo`). Sans `ENT_DATABASE_URL`, ignorés.
//!
//! Couvre : migration de rattachement idempotente et comparaison avant/après, cohérence
//! texte ↔ `classe_id` (double écriture), génération des séances (vacances exclues),
//! bascule d'année (préparation, reconduction, montée, ouverture, clôture), périodes,
//! grille, isolation entre établissements, droits par le moteur et événements outbox.

use super::*;
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use domain_eleve::{today_utc, DomainError};
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn adm(cast: &LaboCast) -> Acteur<'static> {
    Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.administratif.user_id,
        role: Role::Administratif.as_str(),
    }
}

fn dir(cast: &LaboCast) -> Acteur<'static> {
    Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.direction.user_id,
        role: Role::Direction.as_str(),
    }
}

fn prof(cast: &LaboCast) -> Acteur<'static> {
    Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.prof.user_id,
        role: Role::Prof.as_str(),
    }
}

fn est(e: &EntDbError, d: DomainError) -> bool {
    matches!(e, EntDbError::Domain(x) if *x == d)
}

fn d(a: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, j).unwrap()
}

/// Nombre d'événements publiés (journal) d'un type pour l'établissement.
async fn nb_evenements(ent: &PgEnt, etab: Uuid, type_name: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = $2")
        .bind(etab)
        .bind(type_name)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

/// Empreinte des textes d'une table (ordre stable) : doit rester identique après rattachement.
async fn empreinte_textes(ent: &PgEnt, etab: Uuid, table: &str, col: &str, extra: &str) -> String {
    let sql = format!(
        "SELECT coalesce(string_agg(coalesce(t.{col}, '<null>'), '|' ORDER BY t.id::text), '')
           FROM {table} t WHERE t.etablissement_id = $1 {extra}"
    );
    sqlx::query_scalar(&sql).bind(etab).fetch_one(ent.pool()).await.unwrap()
}

/// Données couvrant les 9 tables à `classe` texte, en plus du seed (élèves, EDT).
async fn semer_donnees_historiques(ent: &PgEnt, c: &LaboCast) {
    let e = c.etablissement_id;
    let auteur = c.prof.user_id;
    let p = ent.pool();
    let jour = today_utc();
    sqlx::query("INSERT INTO ent_appel (id, etablissement_id, classe, jour, auteur_user_id) VALUES ($1,$2,'4B',$3,$4)")
        .bind(Uuid::new_v4()).bind(e).bind(jour).bind(auteur).execute(p).await.unwrap();
    sqlx::query("INSERT INTO ent_appel (id, etablissement_id, classe, jour, auteur_user_id) VALUES ($1,$2,'3C',$3,$4)")
        .bind(Uuid::new_v4()).bind(e).bind(jour - Duration::days(400)).bind(auteur).execute(p).await.unwrap();
    sqlx::query("INSERT INTO ent_cahier_texte (id, etablissement_id, classe, matiere_libelle, titre, consigne, pour_le, cree_le, created_by_user_id)
                 VALUES ($1,$2,'4B','Maths','Exos','p.12',$3,$3,$4)")
        .bind(Uuid::new_v4()).bind(e).bind(jour).bind(auteur).execute(p).await.unwrap();
    sqlx::query("INSERT INTO ent_conseil_classe (id, etablissement_id, classe, periode_libelle, jour, heure, created_by_user_id)
                 VALUES ($1,$2,'5A','T1',$3,'18:00',$4)")
        .bind(Uuid::new_v4()).bind(e).bind(jour).bind(auteur).execute(p).await.unwrap();
    sqlx::query("INSERT INTO ent_annonce (id, etablissement_id, titre, corps, audience, cible_classe, auteur_user_id)
                 VALUES ($1,$2,'Sortie','Bonjour','classe','4B',$3)")
        .bind(Uuid::new_v4()).bind(e).bind(auteur).execute(p).await.unwrap();
    sqlx::query("INSERT INTO ent_eleve_scolarite_annee (id, etablissement_id, eleve_id, annee_scolaire, classe, statut)
                 VALUES ($1,$2,$3,'2024-2025','5A','historique')")
        .bind(Uuid::new_v4()).bind(e).bind(c.eleve_lea_id).execute(p).await.unwrap();
    sqlx::query("INSERT INTO ent_eleve_periode (id, etablissement_id, eleve_id, kind, valeur, date_debut, date_fin)
                 VALUES ($1,$2,$3,'classe','5A',$4,$5)")
        .bind(Uuid::new_v4()).bind(e).bind(c.eleve_lea_id).bind(d(2024, 9, 2)).bind(d(2025, 6, 30)).execute(p).await.unwrap();
    // Devoir : période de notes (rapatriée dans ent_periode par trigger) + matière.
    let periode = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_note_periode (id, etablissement_id, code, libelle, date_debut, date_fin)
                 VALUES ($1,$2,'T9','Trimestre 9',$3,$4)")
        .bind(periode).bind(e).bind(d(jour.year(), 9, 1)).bind(d(jour.year(), 12, 20)).execute(p).await.unwrap();
    let matiere = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1,$2,'MATHS',$3)")
        .bind(matiere).bind(e).bind("Maths").execute(p).await.unwrap();
    sqlx::query("INSERT INTO ent_devoir (id, etablissement_id, periode_id, matiere_id, classe, titre, date_devoir, created_by_user_id)
                 VALUES ($1,$2,$3,$4,'4B','DS1',$5,$6)")
        .bind(Uuid::new_v4()).bind(e).bind(periode).bind(matiere).bind(jour).bind(auteur).execute(p).await.unwrap();
}

#[tokio::test]
async fn rattachement_idempotent_et_comparaison_avant_apres() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    semer_donnees_historiques(&ent, &cast).await;

    // Double écriture : les écritures « nouvelles » (triggers) sont déjà cohérentes.
    for c in ent.ref_comparer_classe_id(e).await.unwrap() {
        assert!(c.est_saine(), "double écriture incohérente : {c:?}");
    }

    // « Avant » : état hérité simulé (classe_id vide partout, triggers contournés).
    let mut tx = ent.pool().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await.unwrap();
    for (table, _, extra) in TABLES_CLASSE {
        let extra = extra.replace("t.", "");
        sqlx::query(&format!("UPDATE {table} SET classe_id = NULL WHERE etablissement_id = $1 {extra}"))
            .bind(e)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();

    let mut textes_avant = Vec::new();
    for (table, col, extra) in TABLES_CLASSE {
        textes_avant.push(empreinte_textes(&ent, e, table, col, extra).await);
    }
    let avant = ent.ref_comparer_classe_id(e).await.unwrap();
    for c in &avant {
        assert_eq!(c.avec_classe_id, 0, "{c:?}");
        assert_eq!(c.sans_classe_id, c.avec_texte, "{c:?}");
    }
    // Chaque table a au moins une ligne à rattacher (les 9 sont couvertes).
    for c in &avant {
        assert!(c.avec_texte > 0, "table sans donnée de test : {}", c.table);
    }

    // Rattachement : relancé, il rattache tout ; texte inchangé.
    let rapport = ent.ref_rattacher_classes(&adm(&cast)).await.unwrap();
    for (table, n) in &rapport {
        let att = avant.iter().find(|c| &c.table == table).unwrap();
        assert_eq!(*n, att.avec_texte, "{table}");
    }
    let apres = ent.ref_comparer_classe_id(e).await.unwrap();
    for (a, b) in avant.iter().zip(&apres) {
        assert_eq!(b.avec_texte, a.avec_texte, "le texte ne bouge pas : {}", a.table);
        assert_eq!(b.avec_classe_id, a.avec_texte, "{b:?}");
        assert!(b.est_saine(), "{b:?}");
    }
    for ((table, col, extra), avant_t) in TABLES_CLASSE.iter().zip(&textes_avant) {
        assert_eq!(&empreinte_textes(&ent, e, table, col, extra).await, avant_t, "{table}");
    }

    // Idempotent : une seconde passe ne change plus rien.
    let n_classes: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_classe WHERE etablissement_id = $1")
        .bind(e).fetch_one(ent.pool()).await.unwrap();
    let rapport2 = ent.ref_rattacher_classes(&adm(&cast)).await.unwrap();
    assert!(rapport2.iter().all(|(_, n)| *n == 0), "{rapport2:?}");
    let n_classes2: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_classe WHERE etablissement_id = $1")
        .bind(e).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n_classes, n_classes2);
    assert_eq!(ent.ref_comparer_classe_id(e).await.unwrap(), apres);

    // Années : les lignes d'une année passée sont rattachées à la classe de CETTE année.
    let (annee_appel,): (String,) = sqlx::query_as(
        "SELECT y.libelle FROM ent_appel a JOIN ent_classe c ON c.id = a.classe_id
           JOIN ent_annee_scolaire y ON y.id = c.annee_id
          WHERE a.etablissement_id = $1 AND a.classe = '3C'",
    )
    .bind(e).fetch_one(ent.pool()).await.unwrap();
    let j = today_utc() - Duration::days(400);
    let y = if j.month() >= 9 { j.year() } else { j.year() - 1 };
    assert_eq!(annee_appel, format!("{y}-{}", y + 1));
    let (annee_hist,): (String,) = sqlx::query_as(
        "SELECT y.libelle FROM ent_eleve_scolarite_annee s JOIN ent_classe c ON c.id = s.classe_id
           JOIN ent_annee_scolaire y ON y.id = c.annee_id
          WHERE s.etablissement_id = $1 AND s.annee_scolaire = '2024-2025'",
    )
    .bind(e).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(annee_hist, "2024-2025");
}

#[tokio::test]
async fn double_ecriture_texte_et_classe_id() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let autre = ent.seed_labo_cast().await.unwrap();

    // Texte seul : classe_id résolu (classe créée si besoin).
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_appel (id, etablissement_id, classe, jour, auteur_user_id) VALUES ($1,$2,'6Z',$3,$4)")
        .bind(id).bind(e).bind(today_utc()).bind(cast.prof.user_id)
        .execute(ent.pool()).await.unwrap();
    let (cid, code): (Uuid, String) = sqlx::query_as(
        "SELECT c.id, c.code FROM ent_appel a JOIN ent_classe c ON c.id = a.classe_id WHERE a.id = $1",
    ).bind(id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(code, "6Z");

    // classe_id seul fait foi : le texte s'aligne.
    let id2 = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_appel (id, etablissement_id, classe, classe_id, jour, auteur_user_id) VALUES ($1,$2,'AUTRE',$3,$4,$5)")
        .bind(id2).bind(e).bind(cid).bind(today_utc() + Duration::days(1)).bind(cast.prof.user_id)
        .execute(ent.pool()).await.unwrap();
    let (texte,): (String,) = sqlx::query_as("SELECT classe FROM ent_appel WHERE id = $1")
        .bind(id2).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(texte, "6Z");

    // Changement du texte seul : l'identifiant suit.
    sqlx::query("UPDATE ent_appel SET classe = '6Y' WHERE id = $1").bind(id).execute(ent.pool()).await.unwrap();
    let (code,): (String,) = sqlx::query_as(
        "SELECT c.code FROM ent_appel a JOIN ent_classe c ON c.id = a.classe_id WHERE a.id = $1",
    ).bind(id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(code, "6Y");

    // Classe d'un autre établissement : refusée.
    let classe_autre: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_classe WHERE etablissement_id = $1 LIMIT 1",
    ).bind(autre.etablissement_id).fetch_one(ent.pool()).await.unwrap();
    let r = sqlx::query("UPDATE ent_appel SET classe_id = $2 WHERE id = $1")
        .bind(id).bind(classe_autre).execute(ent.pool()).await;
    assert!(r.is_err(), "classe_id d'un autre établissement accepté");

    // Le flux historique `change_classe` reste cohérent des deux côtés.
    for c in ent.ref_comparer_classe_id(e).await.unwrap() {
        assert!(c.est_saine(), "{c:?}");
    }
}

#[tokio::test]
async fn seances_generees_vacances_exclues_et_idempotence() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let a = adm(&cast);
    let annee = ent.referentiel_annee_courante_assuree(e).await.unwrap();
    ent.ref_amorcer_grille(&a).await.unwrap();

    // Un lundi 08:00 de la classe 4B, ancré dans l'année courante.
    let mut lundi = today_utc().clamp(annee.date_debut, annee.date_fin);
    lundi += Duration::days((8 - lundi.weekday().number_from_monday() as i64) % 7);
    let du = lundi;
    let au = lundi + Duration::days(27); // 4 lundis
    let creneau = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin, salle)
                 VALUES ($1,$2,'4B','Maths',$3,1,$4,$5,'B12')")
        .bind(creneau).bind(e).bind(cast.prof.user_id)
        .bind(NaiveTime::from_hms_opt(8, 0, 0).unwrap()).bind(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
        .execute(ent.pool()).await.unwrap();
    let nb = |ent: &PgEnt| {
        let pool = ent.pool().clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND creneau_id = $2")
                .bind(e).bind(creneau).fetch_one(&pool).await.unwrap()
        }
    };

    let r = ent.ref_generer_seances(&a, annee.id, Some(du), Some(au)).await.unwrap();
    assert!(r.creees >= 4, "{r:?}");
    assert_eq!(nb(&ent).await, 4);
    let (code, demi): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT grille_code, demi_journee FROM ent_edt_seance WHERE creneau_id = $1 LIMIT 1",
    ).bind(creneau).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((code.as_deref(), demi.as_deref()), (Some("M1"), Some("matin")));

    // Idempotent : seconde génération, rien de nouveau.
    let r2 = ent.ref_generer_seances(&a, annee.id, Some(du), Some(au)).await.unwrap();
    assert_eq!((r2.creees, r2.mises_a_jour, r2.supprimees), (0, 0, 0), "{r2:?}");
    assert_eq!(nb(&ent).await, 4);

    // Vacances sur le 2e lundi : sa séance disparaît ; le calendrier unifié les voit.
    let lundi2 = du + Duration::days(7);
    ent.ref_creer_vacances(&a, "Toussaint", "vacances", lundi2, lundi2 + Duration::days(4)).await.unwrap();
    let cal = ent.ref_calendrier(&a, du, au).await.unwrap();
    assert!(cal.iter().any(|j| j.jour == lundi2 && j.origine == "vacances"));
    let r3 = ent.ref_generer_seances(&a, annee.id, Some(du), Some(au)).await.unwrap();
    assert!(r3.supprimees >= 1, "{r3:?}");
    assert_eq!(nb(&ent).await, 3);
    let present: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance WHERE creneau_id = $1 AND jour = $2")
        .bind(creneau).bind(lundi2).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(present, 0, "séance générée pendant les vacances");

    // Journée fermée (jour calendrier) sur le 3e lundi.
    let lundi3 = du + Duration::days(14);
    assert_eq!(ent.ref_ajouter_jours(&a, lundi3, lundi3, "banalise", "Journée pédagogique", None).await.unwrap(), 1);
    // Idempotent : le même jour une 2e fois n'ajoute rien.
    assert_eq!(ent.ref_ajouter_jours(&a, lundi3, lundi3, "banalise", "Journée pédagogique", None).await.unwrap(), 0);
    ent.ref_generer_seances(&a, annee.id, Some(du), Some(au)).await.unwrap();
    assert_eq!(nb(&ent).await, 2);

    // Réouverture du jour : la séance revient.
    ent.ref_retirer_jour(&a, lundi3, None).await.unwrap();
    let r4 = ent.ref_generer_seances(&a, annee.id, Some(du), Some(au)).await.unwrap();
    assert!(r4.creees >= 1, "{r4:?}");

    // Exception d'EDT (annulation) répercutée ; séance saisie à la main jamais écrasée.
    let lundi4 = du + Duration::days(21);
    sqlx::query("INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, motif, created_by_user_id)
                 VALUES ($1,$2,$3,$4,'annulation','Absence prof',$5)")
        .bind(Uuid::new_v4()).bind(e).bind(creneau).bind(lundi4).bind(cast.administratif.user_id)
        .execute(ent.pool()).await.unwrap();
    ent.ref_generer_seances(&a, annee.id, Some(du), Some(au)).await.unwrap();
    let statut: String = sqlx::query_scalar("SELECT statut FROM ent_edt_seance WHERE creneau_id = $1 AND jour = $2")
        .bind(creneau).bind(lundi4).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(statut, "annulee");
    sqlx::query("UPDATE ent_edt_seance SET source = 'manuel', salle = 'Gymnase' WHERE creneau_id = $1 AND jour = $2")
        .bind(creneau).bind(du).execute(ent.pool()).await.unwrap();
    ent.ref_generer_seances(&a, annee.id, Some(du), Some(au)).await.unwrap();
    let salle: Option<String> = sqlx::query_scalar("SELECT salle FROM ent_edt_seance WHERE creneau_id = $1 AND jour = $2")
        .bind(creneau).bind(du).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(salle.as_deref(), Some("Gymnase"));

    // Lecture par classe ; événements publiés dans l'outbox.
    let c4b: Uuid = sqlx::query_scalar("SELECT id FROM ent_classe WHERE etablissement_id = $1 AND annee_id = $2 AND code = '4B'")
        .bind(e).bind(annee.id).fetch_one(ent.pool()).await.unwrap();
    let liste = ent.ref_seances(&a, du, au, Some(c4b), None).await.unwrap();
    assert_eq!(liste.iter().filter(|s| s.creneau_id == creneau).count(), 3); // 4 lundis moins la semaine de vacances
    assert!(nb_evenements(&ent, e, "edt.seances_generees").await >= 5);
    assert!(nb_evenements(&ent, e, "referentiel.vacances_creees").await >= 1);

    // Un enseignant (profil de base prof) n'a pas la gestion du référentiel.
    let err = ent.ref_generer_seances(&prof(&cast), annee.id, Some(du), Some(au)).await.unwrap_err();
    assert!(matches!(err, EntDbError::Forbidden), "{err:?}");
    assert!(ent.ref_seances(&prof(&cast), du, au, None, None).await.is_ok());
}

#[tokio::test]
async fn bascule_d_annee_complete() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let direction = dir(&cast);
    let annee = ent.referentiel_annee_courante_assuree(e).await.unwrap();
    let a_n = ent.ref_annees(&direction).await.unwrap().into_iter().find(|x| x.id == annee.id).unwrap();
    assert_eq!(a_n.statut, "courante");

    // Périodes de l'année courante (REF-04) : trou refusé, couverture exacte acceptée.
    let p = |code: &str, du: NaiveDate, au: NaiveDate| PeriodeSaisie {
        code: code.into(), libelle: code.into(), kind: "trimestre".into(), date_debut: du, date_fin: au,
    };
    let trou = vec![p("T1", annee.date_debut, d(annee.date_debut.year(), 11, 29)), p("T2", d(annee.date_debut.year(), 12, 5), annee.date_fin)];
    let err = ent.ref_definir_periodes(&direction, annee.id, None, &trou).await.unwrap_err();
    assert!(est(&err, DomainError::ValeurInvalide), "{err:?}");
    let y = annee.date_debut.year();
    let bonnes = vec![
        p("T1", annee.date_debut, d(y, 11, 30)),
        p("T2", d(y, 12, 1), d(y + 1, 2, 28)),
        p("T3", d(y + 1, 3, 1), annee.date_fin),
    ];
    let ps = ent.ref_definir_periodes(&direction, annee.id, None, &bonnes).await.unwrap();
    assert_eq!(ps.len(), 3);
    // Redéfinition idempotente (mêmes codes) : mêmes identifiants conservés.
    let ps2 = ent.ref_definir_periodes(&direction, annee.id, None, &bonnes).await.unwrap();
    assert_eq!(ps.iter().map(|x| x.id).collect::<Vec<_>>(), ps2.iter().map(|x| x.id).collect::<Vec<_>>());

    // Niveaux nationaux + site + classe rattachée.
    let (crees, _) = ent.ref_amorcer_niveaux(&direction).await.unwrap();
    assert_eq!(crees, NIVEAUX_NATIONAUX.len() as i64);
    assert_eq!(ent.ref_amorcer_niveaux(&direction).await.unwrap().0, 0);
    let site = ent.ref_creer_site(&direction, "PRINC", "Site principal", None, None).await.unwrap();
    let c4b: Uuid = sqlx::query_scalar("SELECT id FROM ent_classe WHERE etablissement_id = $1 AND annee_id = $2 AND code = '4B'")
        .bind(e).bind(annee.id).fetch_one(ent.pool()).await.unwrap();
    let niveau4 = ent.ref_niveaux(&direction).await.unwrap().into_iter().find(|n| n.code == "4EME").unwrap();
    ent.ref_modifier_classe(&direction, c4b, None, Some(site.id), Some(niveau4.id), None, None).await.unwrap();
    // 4B a déjà son niveau lié par l'amorçage (texte « 4e ») ou par la modification ci-dessus.
    let (n_id, s_id): (Option<Uuid>, Option<Uuid>) = sqlx::query_as("SELECT niveau_id, site_id FROM ent_classe WHERE id = $1")
        .bind(c4b).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((n_id, s_id), (Some(niveau4.id), Some(site.id)));
    // Archiver une classe qui a des élèves : refusé.
    let err = ent.ref_modifier_classe(&direction, c4b, None, None, None, None, Some(true)).await.unwrap_err();
    assert!(est(&err, DomainError::Conflit), "{err:?}");

    // Préparation de l'année suivante : reconduction des structures, idempotente.
    let (suivante, r1) = ent.ref_preparer_annee_suivante(&direction, annee.id, true).await.unwrap();
    assert_eq!(suivante.statut, "preparation");
    assert_eq!(suivante.date_debut, d(y + 1, 9, 1));
    assert!(r1.classes_creees >= 2 && r1.periodes_creees == 3, "{r1:?}");
    assert!(r1.services_crees >= 1, "{r1:?}");
    let (suivante2, r2) = ent.ref_preparer_annee_suivante(&direction, annee.id, true).await.unwrap();
    assert_eq!(suivante2.id, suivante.id);
    assert_eq!((r2.classes_creees, r2.groupes_crees, r2.periodes_creees, r2.services_crees), (0, 0, 0, 0), "{r2:?}");
    let p_suiv = ent.ref_periodes(&direction, suivante.id, None).await.unwrap();
    assert_eq!(p_suiv[0].date_debut, d(y + 1, 9, 1));
    assert!(verifier_couverture(
        &p_suiv.iter().map(|x| PeriodeSaisie { code: x.code.clone(), libelle: x.libelle.clone(), kind: x.kind.clone(), date_debut: x.date_debut, date_fin: x.date_fin }).collect::<Vec<_>>(),
        suivante.date_debut, suivante.date_fin).is_ok());

    // Les dates ne peuvent chevaucher une année existante (REF-01).
    let err = ent.ref_creer_annee(&direction, "Chevauche", d(y + 1, 1, 1), d(y + 2, 6, 30)).await.unwrap_err();
    assert!(est(&err, DomainError::Conflit), "{err:?}");

    // Montée : 4B → 3C (aperçu puis application), idempotente, sans écraser les conflits.
    let classes_s = ent.referentiel_classes(Role::Direction.as_str(), e, Some(suivante.id), today_utc().max(suivante.date_debut)).await.unwrap();
    let c4b_s = classes_s.iter().find(|c| c.code == "4B").unwrap().id;
    let eleves_4b: i64 = sqlx::query_scalar("SELECT count(DISTINCT eleve_id) FROM ent_affectation WHERE etablissement_id = $1 AND annee_id = $2 AND classe_id = $3 AND date_fin IS NULL")
        .bind(e).bind(annee.id).bind(c4b).fetch_one(ent.pool()).await.unwrap();
    assert!(eleves_4b > 0);
    let apercu = ent.ref_monter(&direction, annee.id, suivante.id, &[(c4b, c4b_s)], true).await.unwrap();
    assert!(apercu.apercu && apercu.a_creer == eleves_4b, "{apercu:?}");
    let nb_aff: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_affectation WHERE etablissement_id = $1 AND annee_id = $2")
        .bind(e).bind(suivante.id).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(nb_aff, 0, "l'aperçu n'écrit rien");
    let reel = ent.ref_monter(&direction, annee.id, suivante.id, &[(c4b, c4b_s)], false).await.unwrap();
    assert_eq!(reel.a_creer, eleves_4b);
    let encore = ent.ref_monter(&direction, annee.id, suivante.id, &[(c4b, c4b_s)], false).await.unwrap();
    assert_eq!((encore.a_creer, encore.deja_conformes), (0, eleves_4b), "{encore:?}");
    // Un élève déjà affecté ailleurs dans l'année cible = conflit, jamais écrasé.
    let c5a_s = classes_s.iter().find(|c| c.code == "5A").unwrap().id;
    let conflit = ent.ref_affecter_lot(&direction, suivante.id, &[(cast.eleve_lea_id, c5a_s), (cast.eleve_tom_id, c5a_s)], false, "lot").await.unwrap();
    assert_eq!(conflit.a_creer + conflit.deja_conformes + conflit.conflits.len() as i64, 2, "{conflit:?}");
    // Classe d'une autre année : refusée en bloc.
    let err = ent.ref_affecter_lot(&direction, suivante.id, &[(cast.eleve_lea_id, c4b)], false, "lot").await.unwrap_err();
    assert!(est(&err, DomainError::NotFound), "{err:?}");

    // Ouverture : l'ancienne courante est clôturée, une seule courante ; la classe courante bascule.
    let ouv = ent.ref_ouvrir_annee(&direction, suivante.id).await.unwrap();
    assert_eq!(ouv.annee_cloturee_id, Some(annee.id));
    assert!(ouv.eleves_synchronises >= eleves_4b);
    let courantes: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante")
        .bind(e).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(courantes, 1);
    let statuts = ent.ref_annees(&direction).await.unwrap();
    assert_eq!(statuts.iter().find(|x| x.id == annee.id).unwrap().statut, "cloturee");
    assert_eq!(statuts.iter().find(|x| x.id == suivante.id).unwrap().statut, "courante");
    let (classe_el, cid_el): (Option<String>, Option<Uuid>) = sqlx::query_as("SELECT el.classe, el.classe_id FROM ent_eleve el JOIN ent_affectation af ON af.eleve_id = el.id AND af.classe_id = $2 WHERE el.etablissement_id = $1 LIMIT 1")
        .bind(e).bind(c4b_s).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((classe_el.as_deref(), cid_el), (Some("4B"), Some(c4b_s)));
    for c in ent.ref_comparer_classe_id(e).await.unwrap() {
        assert!(c.est_saine(), "{c:?}");
    }

    // L'ancienne année est en lecture seule.
    let err = ent.referentiel_creer_classe(Role::Direction.as_str(), e, Some(annee.id), "NEW", None, None).await.unwrap_err();
    assert!(est(&err, DomainError::Conflit), "{err:?}");
    let err = ent.ref_definir_periodes(&direction, annee.id, None, &bonnes).await.unwrap_err();
    assert!(est(&err, DomainError::Conflit), "{err:?}");
    let err = ent.ref_cloturer_annee(&direction, annee.id, false).await.unwrap_err();
    assert!(est(&err, DomainError::Conflit), "déjà clôturée : {err:?}");
    let err = ent.ref_ouvrir_annee(&direction, annee.id).await.unwrap_err();
    assert!(est(&err, DomainError::Conflit), "{err:?}");

    // Clôture de la courante : bloquée tant qu'une période de notes est ouverte (sauf forcer).
    let np: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_note_periode np JOIN ent_periode p ON p.id = np.periode_id WHERE np.etablissement_id = $1 AND p.annee_id = $2 AND np.statut <> 'fermee'")
        .bind(e).bind(suivante.id).fetch_one(ent.pool()).await.unwrap();
    let r = ent.ref_cloturer_annee(&direction, suivante.id, false).await;
    if np > 0 { assert!(r.is_err()); } else { assert!(r.is_ok()); }
    // Événements publiés.
    for t in ["referentiel.annee_creee", "referentiel.annee_ouverte", "referentiel.annee_cloturee",
              "referentiel.structures_reconduites", "referentiel.affectations_lot", "referentiel.periodes_definies"] {
        assert!(nb_evenements(&ent, e, t).await >= 1, "événement {t} absent");
    }
}

#[tokio::test]
async fn periodes_notes_rapatriees_dans_le_referentiel() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    semer_donnees_historiques(&ent, &cast).await;
    let e = cast.etablissement_id;
    let (periode_id, code): (Option<Uuid>, String) = sqlx::query_as("SELECT periode_id, code FROM ent_note_periode WHERE etablissement_id = $1 AND code = 'T9'")
        .bind(e).fetch_one(ent.pool()).await.unwrap();
    let pid = periode_id.expect("ent_note_periode.periode_id renseigné par trigger");
    let (c, kind): (String, String) = sqlx::query_as("SELECT code, kind FROM ent_periode WHERE id = $1 AND etablissement_id = $2")
        .bind(pid).bind(e).fetch_one(ent.pool()).await.unwrap();
    assert_eq!((c, kind.as_str()), (code, "trimestre"));
    // Rejouer le rapatriement ne duplique rien.
    sqlx::query("UPDATE ent_note_periode SET periode_id = NULL WHERE etablissement_id = $1 AND code = 'T9'").bind(e).execute(ent.pool()).await.unwrap();
    sqlx::query("UPDATE ent_note_periode SET periode_id = ent_periode_pour_note(etablissement_id, code, libelle, date_debut, date_fin) WHERE etablissement_id = $1 AND code = 'T9' AND periode_id IS NULL")
        .bind(e).execute(ent.pool()).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_periode WHERE etablissement_id = $1 AND code = 'T9'").bind(e).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn isolation_droits_et_import_mef() {
    let Some((ent, a)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let b = ent.seed_labo_cast().await.expect("second établissement");
    let (aa, ab) = (adm(&a), adm(&b));

    // Droits par le moteur : famille et élève refusés partout ; prof en lecture seule.
    let parent = Acteur { etablissement_id: a.etablissement_id, user_id: a.parent_lea.user_id, role: Role::Parent.as_str() };
    assert!(matches!(ent.ref_annees(&parent).await.unwrap_err(), EntDbError::Forbidden));
    assert!(matches!(ent.ref_comparaison_classe_id(&parent).await.unwrap_err(), EntDbError::Forbidden));
    assert!(ent.ref_annees(&prof(&a)).await.is_ok());
    assert!(matches!(ent.ref_creer_site(&prof(&a), "X", "X", None, None).await.unwrap_err(), EntDbError::Forbidden));
    assert!(matches!(ent.ref_amorcer_niveaux(&prof(&a)).await.unwrap_err(), EntDbError::Forbidden));
    // Un acteur d'un établissement ne peut pas se prévaloir d'un autre : le droit est lu
    // pour l'établissement de l'acteur (un prof de A n'a rien dans B).
    let faux = Acteur { etablissement_id: b.etablissement_id, user_id: a.prof.user_id, role: Role::Prof.as_str() };
    assert!(ent.ref_annees(&faux).await.is_ok(), "lecture prof du profil de base");
    assert!(matches!(ent.ref_creer_site(&faux, "Y", "Y", None, None).await.unwrap_err(), EntDbError::Forbidden));

    // Sites : cloisonnés.
    let sa = ent.ref_creer_site(&aa, "S1", "Site A", None, None).await.unwrap();
    let sb = ent.ref_creer_site(&ab, "S1", "Site B", None, None).await.unwrap();
    assert_ne!(sa.id, sb.id, "même code, deux établissements : permis");
    assert!(ent.ref_sites(&aa).await.unwrap().iter().all(|s| s.id != sb.id));
    let err = ent.ref_creer_site(&aa, "S1", "doublon", None, None).await.unwrap_err();
    assert!(est(&err, DomainError::Conflit), "{err:?}");
    let err = ent.ref_maj_site(&aa, sb.id, Some("pirate"), None, None).await.unwrap_err();
    assert!(est(&err, DomainError::NotFound), "{err:?}");

    // Classe de B rattachée à un site de A : introuvable ; modifier une classe de B depuis A : introuvable.
    let classe_b: Uuid = sqlx::query_scalar("SELECT id FROM ent_classe WHERE etablissement_id = $1 LIMIT 1")
        .bind(b.etablissement_id).fetch_one(ent.pool()).await.unwrap();
    let err = ent.ref_modifier_classe(&aa, classe_b, Some("x"), None, None, None, None).await.unwrap_err();
    assert!(est(&err, DomainError::NotFound), "{err:?}");
    let classe_a: Uuid = sqlx::query_scalar("SELECT id FROM ent_classe WHERE etablissement_id = $1 LIMIT 1")
        .bind(a.etablissement_id).fetch_one(ent.pool()).await.unwrap();
    let err = ent.ref_modifier_classe(&aa, classe_a, None, Some(sb.id), None, None, None).await.unwrap_err();
    assert!(est(&err, DomainError::NotFound), "site d'un autre établissement : {err:?}");
    // Séances / grille / calendrier : lecture limitée à l'établissement.
    ent.ref_amorcer_grille(&ab).await.unwrap();
    assert!(ent.ref_grille(&aa, None).await.unwrap().is_empty());
    assert_eq!(ent.ref_grille(&ab, None).await.unwrap().len(), 8);
    // EDT-03 : service d'un autre établissement refusé.
    let creneau = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin) VALUES ($1,$2,'4B','Maths',$3,2,'08:00','09:00')")
        .bind(creneau).bind(a.etablissement_id).bind(a.prof.user_id).execute(ent.pool()).await.unwrap();
    let service_b: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ent_service_enseignement WHERE etablissement_id = $1 LIMIT 1")
        .bind(b.etablissement_id).fetch_optional(ent.pool()).await.unwrap();
    if let Some(sid) = service_b {
        let err = ent.ref_lier_creneau_service(&aa, creneau, Some(sid)).await.unwrap_err();
        assert!(est(&err, DomainError::NotFound), "{err:?}");
    }
    let service_a: Option<Uuid> = sqlx::query_scalar("SELECT s.id FROM ent_service_enseignement s JOIN ent_classe c ON c.id = s.classe_id WHERE s.etablissement_id = $1 AND s.prof_user_id = $2 AND c.code = '4B' LIMIT 1")
        .bind(a.etablissement_id).bind(a.prof.user_id).fetch_optional(ent.pool()).await.unwrap();
    if let Some(sid) = service_a {
        ent.ref_lier_creneau_service(&aa, creneau, Some(sid)).await.unwrap();
        let lie: Option<Uuid> = sqlx::query_scalar("SELECT service_id FROM ent_edt_creneau WHERE id = $1").bind(creneau).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(lie, Some(sid));
    }

    // Import MEF : création, mise à jour idempotente, rejets détaillés.
    ent.ref_amorcer_niveaux(&aa).await.unwrap();
    let ligne = |code: &str, niveau: Option<&str>| MefImport {
        code_mef: code.into(), mefstat11: Some(code.into()), niveau_code: niveau.map(Into::into),
        libelle_long: "4EME".into(), libelle_edition: None, code_formation_diplome: Some("23020001".into()),
        date_ouverture: None, date_fermeture: None,
    };
    let lignes = vec![ligne("10210002110", Some("4EME")), ligne("COURT", None), ligne("10310002110", Some("INCONNU"))];
    let r1 = ent.ref_importer_mefs(&aa, &lignes, "import_siecle").await.unwrap();
    assert_eq!((r1.crees, r1.mis_a_jour, r1.rejets.len()), (1, 0, 2), "{r1:?}");
    let r2 = ent.ref_importer_mefs(&aa, &lignes, "import_siecle").await.unwrap();
    assert_eq!((r2.crees, r2.mis_a_jour), (0, 1), "{r2:?}");
    assert_eq!(ent.ref_mefs(&aa).await.unwrap().len(), 1);
    assert!(ent.ref_mefs(&ab).await.unwrap().is_empty());

    // Droit paramétré : un rôle personnalisé « referentiel.gerer » (D-017) ouvre la gestion à
    // un profil de base qui ne l'avait pas (ici un professeur).
    use ent_auth::{Action, Portee};
    let role = ent.creer_role_for_actor(
        a.etablissement_id, a.admin.user_id, Role::Admin.as_str(),
        RoleSaisie {
            nom: "Gestionnaire du référentiel".into(), description: None, modele: None,
            droits: vec![Droit { action: Action::ReferentielGerer, portee: Portee::Etablissement, cibles: vec![] }],
        },
    ).await;
    let role = role.expect("création du rôle");
    {
        ent.attribuer_role_for_actor(a.etablissement_id, a.admin.user_id, Role::Admin.as_str(), role.id, a.prof.user_id, None)
            .await
            .expect("attribution");
        let site = ent.ref_creer_site(&prof(&a), "PROF", "Créé par un prof habilité", None, None).await;
        assert!(site.is_ok(), "{site:?}");
    }
}
