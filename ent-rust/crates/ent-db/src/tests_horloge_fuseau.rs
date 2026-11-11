//! Fuseau de l'établissement et minuit de Paris : la règle Rust (`domain_eleve::horloge`) doit
//! coïncider avec la base (`ent_aujourdhui_etab`, fuseau du profil d'établissement, S2) à toute
//! heure, en particulier entre 22 h et 0 h UTC (la date UTC diffère alors de celle de Paris).
//! Contre la base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ils sont ignorés.

use super::*;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::horloge::{date_paris, jour_paris, jour_paris_a, Fixe};

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

/// La règle Rust = la base de fuseaux de PostgreSQL, toutes les 30 minutes sur trois ans
/// (heures d'été et d'hiver, bascules de mars et d'octobre, changements d'année).
#[tokio::test]
async fn jour_de_paris_rust_egal_jour_de_la_base_sur_trois_ans() {
    let Some((ent, cast)) = labo().await else { return };
    let debut: DateTime<Utc> = "2025-01-01T00:00:00Z".parse().unwrap();
    let fin: DateTime<Utc> = "2028-01-01T00:00:00Z".parse().unwrap();
    let lignes: Vec<(DateTime<Utc>, NaiveDate)> = sqlx::query_as(
        "SELECT t, ent_aujourdhui_etab($1, t)
           FROM generate_series($2::timestamptz, $3::timestamptz, interval '30 minutes') AS t",
    )
    .bind(cast.etablissement_id)
    .bind(debut)
    .bind(fin)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(lignes.len() > 50_000);
    for (t, base) in lignes {
        assert_eq!(date_paris(t), base, "écart Rust/base à {t}");
    }
}

/// Scénario 23 h 30 UTC : la base et le Rust donnent le lendemain (Paris), pas la date UTC ;
/// un établissement réglé sur un fuseau retardataire garde, lui, la date UTC.
#[tokio::test]
async fn scenario_23h30_utc_jour_de_l_etablissement() {
    let Some((ent, cast)) = labo().await else { return };
    let etab = cast.etablissement_id;
    let instant: DateTime<Utc> = "2026-11-10T23:30:00Z".parse().unwrap();
    let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(j, NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
    assert_eq!(j, jour_paris_a(&Fixe(instant)));
    assert_ne!(j, instant.date_naive(), "la date UTC n'est pas le jour de l'établissement");
    // Même instant, étab. en Guyane (UTC−3) : 20 h 30 le 10 novembre. (Calcul pur côté base.)
    let (guyane,): (NaiveDate,) =
        sqlx::query_as("SELECT (($1::timestamptz) AT TIME ZONE 'America/Cayenne')::date")
            .bind(instant)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(guyane, instant.date_naive());
}

/// Les ports de l'établissement et le Rust donnent le même jour courant (tolérance de
/// l'instant de lecture : on encadre l'appel de la base par deux lectures de l'horloge).
#[tokio::test]
async fn jour_courant_base_et_rust_concordent() {
    let Some((ent, cast)) = labo().await else { return };
    let avant = jour_paris();
    let mut conn = ent.pool().acquire().await.unwrap();
    let base = crate::structure_etab::structure::aujourdhui(&mut conn, cast.etablissement_id)
        .await
        .unwrap();
    let apres = jour_paris();
    assert!(base == avant || base == apres, "base {base}, Rust {avant}/{apres}");
    assert!(apres - avant <= Duration::days(1));
}
