//! Le semis de labo est relatif à l'année scolaire du jour de référence injecté (règle 33) : il ne
//! contient plus de date en dur et se rejoue à n'importe quelle rentrée (le 1ᵉʳ septembre 2027 le
//! faisait échouer). Le jour injecté ne déplace pas l'horloge réelle : l'année « courante » du référentiel reste
//! celle du jour réel ; ce test borne donc le semis seul (aucune année antérieure parasite en préparation). Contre la base locale de test (feature `labo`).

use super::*;
use chrono::NaiveDate;
use domain_eleve::{annee_scolaire_de, debut_annee_scolaire};

fn jour(a: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, j).unwrap()
}

#[tokio::test]
async fn le_semis_se_rejoue_a_n_importe_quelle_date_de_reference() {
    let Some(url) = crate::url_guard::url_labo_ou_echec() else {
        return;
    };
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    // Rentrée, 20 décembre (fin du T1), 15 juin, rentrée 2027 (cassait), rentrée 2030.
    for j in [
        jour(2026, 9, 1),
        jour(2026, 12, 20),
        jour(2027, 6, 15),
        jour(2027, 9, 1),
        jour(2030, 9, 1),
    ] {
        let cast = ent
            .seed_labo_cast_au(j)
            .await
            .unwrap_or_else(|e| panic!("semis au {j} : {e:?}"));
        let rentree = debut_annee_scolaire(j);
        let annees: Vec<(String, String, bool)> = sqlx::query_as(
            "SELECT libelle, statut, date_fin < $2 AS anterieure
               FROM ent_annee_scolaire WHERE etablissement_id = $1 ORDER BY date_debut",
        )
        .bind(cast.etablissement_id)
        .bind(rentree)
        .fetch_all(ent.pool())
        .await
        .unwrap();
        assert!(
            annees.iter().any(|(l, _, _)| *l == annee_scolaire_de(j)),
            "au {j} : l'année {} manque ({annees:?})",
            annee_scolaire_de(j)
        );
        assert!(
            annees
                .iter()
                .filter(|(_, _, ant)| *ant)
                .all(|(_, s, _)| s != "preparation"),
            "au {j} : une année antérieure est restée « préparation » ({annees:?})"
        );
    }
}
