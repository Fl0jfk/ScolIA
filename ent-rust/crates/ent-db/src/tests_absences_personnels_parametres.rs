//! Socle 15, lot A11f — seuil réglable de la mention « répond rarement aux sollicitations » (migration 2315),
//! contre la base locale de test (feature `labo`). Sans `ENT_DATABASE_URL` : ignorés en local et **en échec en CI**.
//!
//! Un établissement neuf par test. Le seuil se règle par établissement (droit `parametrer`), se lit de qui
//! organise les remplacements, jamais d'un professeur ; une valeur hors de 1 à 20 est refusée ; le réglage d'un
//! établissement n'atteint pas un autre ; chaque changement est journalisé.

use super::*;
use ent_types::absences_personnels::*;
use uuid::Uuid;

const MDP: &str = "MotDePasse-A15-Aa9!";

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("a15p-{role}-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(&email, MDP, Some(&format!("Nom {role}")), etab, role)
        .await
        .expect("compte")
        .id
}

fn body(v: i16) -> ModifierParametresAbsencesBody {
    ModifierParametresAbsencesBody { seuil_sans_reponse_min: v }
}

/// Défaut, réglage, bornes, droits, journal et cloisonnement du seuil.
#[tokio::test]
async fn a11f_le_seuil_se_regle_par_etablissement_avec_le_droit_parametrer() {
    let Some(ent) = connecter().await else {
        eprintln!("skip : ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    let admin = compte(&ent, etab, "admin").await;
    let sec = compte(&ent, etab, "administratif").await;
    let cpe = compte(&ent, etab, "cpe").await;
    let prof = compte(&ent, etab, "prof").await;
    let accueil = compte(&ent, etab, "accueil").await;

    // Sans réglage : la valeur d'origine, non « réglée ».
    let p = ent.parametres_absences_for_actor(etab, dir, "direction").await.unwrap();
    assert_eq!((p.seuil_sans_reponse_min, p.seuil_sans_reponse_defaut, p.regle), (3, 3, false));
    assert!(p.modifie_le.is_none());
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert_eq!(crate::absences_personnels::impact::seuil_sans_reponse_tx(&mut tx, etab).await.unwrap(), 3);
    drop(tx);

    // Lecture : la direction et qui organise ; jamais un professeur, l'accueil ni le secrétariat sans le droit.
    assert!(ent.parametres_absences_for_actor(etab, cpe, "cpe").await.is_ok(), "le CPE organise");
    for (u, r) in [(prof, "prof"), (accueil, "accueil")] {
        let e = ent.parametres_absences_for_actor(etab, u, r).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "lecture {r} : {e:?}");
    }

    // Écriture : seulement le droit `parametrer` (direction, administration) ; les autres sont refusés.
    for (u, r) in [(prof, "prof"), (cpe, "cpe"), (accueil, "accueil"), (sec, "administratif")] {
        let e = ent.definir_parametres_absences_for_actor(etab, u, r, &body(5)).await.unwrap_err();
        assert!(matches!(e, EntDbError::Forbidden), "écriture {r} : {e:?}");
    }
    assert_eq!(
        ent.parametres_absences_for_actor(etab, dir, "direction").await.unwrap().seuil_sans_reponse_min,
        3,
        "un refus ne change rien"
    );

    // Bornes : de 1 à 20.
    for v in [0, -1, 21, 100] {
        let e = ent.definir_parametres_absences_for_actor(etab, dir, "direction", &body(v)).await.unwrap_err();
        assert!(matches!(e, EntDbError::Refus(..)), "{v} : {e:?}");
    }

    // Réglage par la direction, puis par l'administration ; relu par le CPE.
    let p = ent.definir_parametres_absences_for_actor(etab, dir, "direction", &body(5)).await.unwrap();
    assert_eq!((p.seuil_sans_reponse_min, p.regle), (5, true));
    assert!(p.modifie_le.is_some());
    assert_eq!(ent.parametres_absences_for_actor(etab, cpe, "cpe").await.unwrap().seuil_sans_reponse_min, 5);
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert_eq!(crate::absences_personnels::impact::seuil_sans_reponse_tx(&mut tx, etab).await.unwrap(), 5);
    drop(tx);
    let p = ent.definir_parametres_absences_for_actor(etab, admin, "admin", &body(2)).await.unwrap();
    assert_eq!(p.seuil_sans_reponse_min, 2);
    // Rejouer la même valeur ne journalise rien de plus ; revenir à 3 (la valeur d'origine) est « non réglé ».
    ent.definir_parametres_absences_for_actor(etab, admin, "admin", &body(2)).await.unwrap();
    let p = ent.definir_parametres_absences_for_actor(etab, dir, "direction", &body(3)).await.unwrap();
    assert_eq!((p.seuil_sans_reponse_min, p.regle), (3, false));

    // Journal : un fait par changement réel (3 → 5, 5 → 2, 2 → 3), valeurs avant et après, jamais autre chose.
    let lignes: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT action, valeur_avant, valeur_apres FROM ent_absp_journal
          WHERE etablissement_id = $1 AND objet = 'param' AND colonne = 'seuil_sans_reponse_min' ORDER BY le, id",
    )
    .bind(etab)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(lignes.len(), 3, "{lignes:?}");
    assert!(lignes.iter().all(|(a, _, _)| a == "seuil_sans_reponse"));
    let mut paires: Vec<(String, String)> = lignes
        .into_iter()
        .map(|(_, a, b)| (a.unwrap_or_default(), b.unwrap_or_default()))
        .collect();
    paires.sort();
    assert_eq!(
        paires,
        vec![("2".into(), "3".into()), ("3".into(), "5".into()), ("5".into(), "2".into())]
    );

    // Cloisonnement : un autre établissement garde la valeur d'origine, et ne lit rien du premier.
    let autre = Uuid::new_v4();
    let dir_b = compte(&ent, autre, "direction").await;
    ent.definir_parametres_absences_for_actor(etab, dir, "direction", &body(9)).await.unwrap();
    let pb = ent.parametres_absences_for_actor(autre, dir_b, "direction").await.unwrap();
    assert_eq!((pb.seuil_sans_reponse_min, pb.regle), (3, false), "l'autre établissement n'est pas touché");
    ent.definir_parametres_absences_for_actor(autre, dir_b, "direction", &body(7)).await.unwrap();
    assert_eq!(ent.parametres_absences_for_actor(etab, dir, "direction").await.unwrap().seuil_sans_reponse_min, 9);
    // (Un compte ne règle que l'établissement de sa session : la route prend l'établissement de la session, jamais
    // du corps ; ce point est joué contre le vrai backend dans `ent-web-server/tests/absences_personnels_hrcd.rs`.)
}

/// Une ligne créée pour un autre réglage (l'interrupteur « EDT de référence ») ne fait pas passer le seuil pour réglé.
#[tokio::test]
async fn a11f_un_autre_reglage_ne_change_pas_le_seuil() {
    let Some(ent) = connecter().await else {
        eprintln!("skip : ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let dir = compte(&ent, etab, "direction").await;
    sqlx::query(
        "INSERT INTO ent_absp_param (etablissement_id, edt_reference, modifie_par_user_id) VALUES ($1, false, $2)",
    )
    .bind(etab)
    .bind(dir)
    .execute(ent.pool())
    .await
    .unwrap();
    let p = ent.parametres_absences_for_actor(etab, dir, "direction").await.unwrap();
    assert_eq!((p.seuil_sans_reponse_min, p.regle), (3, false));
    assert!(p.modifie_le.is_none());
}
