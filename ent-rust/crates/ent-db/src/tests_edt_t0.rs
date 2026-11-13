//! Socle 10 (emploi du temps), lot T0 : correctifs urgents D1, D2, D3, D4, D8, D9, D12, D19
//! (`docs/socles/10_EMPLOI_DU_TEMPS.md` §2.2, §23.1). Base locale de test (feature `labo`).
//!
//! Chaque défaut a son test « rouge avant, vert après » : régénération avec jour férié → cahier
//! conservé ; professeur d'une autre langue → 403 sur l'EDT de l'élève (quatre routes) ; aucun motif
//! interne pour un professeur ; RLS posée sur les tables EDT ; professeur importé membre de
//! l'établissement ; collègue réduit à sa présence.

use super::*;
use crate::edt_admin::CibleEdt;
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use domain_eleve::today_utc;
use ent_auth::Role;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

fn acteur(cast: &LaboCast, user: Uuid, role: Role) -> Acteur<'static> {
    Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: user,
        role: role.as_str(),
    }
}

fn adm(cast: &LaboCast) -> Acteur<'static> {
    acteur(cast, cast.administratif.user_id, Role::Administratif)
}

// ───────────────────────── D1 : le cahier survit à la régénération ─────────────────────────

#[tokio::test]
async fn d1_regeneration_apres_jour_ferie_conserve_le_cahier() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let a = adm(&cast);
    let annee = ent.referentiel_annee_courante_assuree(e).await.unwrap();
    ent.ref_amorcer_grille(&a).await.unwrap();
    let mut lundi = today_utc().clamp(annee.date_debut, annee.date_fin);
    lundi += Duration::days((8 - lundi.weekday().number_from_monday() as i64) % 7);
    let (du, au) = (lundi, lundi + Duration::days(27));
    let creneau = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id,
                jour_semaine, heure_debut, heure_fin, salle)
         VALUES ($1,$2,'4B','Maths',$3,1,$4,$5,'B12')",
    )
    .bind(creneau)
    .bind(e)
    .bind(cast.prof.user_id)
    .bind(NaiveTime::from_hms_opt(8, 0, 0).unwrap())
    .bind(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
    .execute(ent.pool())
    .await
    .unwrap();
    ent.ref_generer_seances(&a, annee.id, Some(du), Some(au))
        .await
        .unwrap();

    let jour_ferie = du + Duration::days(14);
    let seance_id = |jour: NaiveDate| {
        let pool = ent.pool().clone();
        async move {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM ent_edt_seance WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3",
            )
            .bind(e)
            .bind(creneau)
            .bind(jour)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    let porteuse = seance_id(jour_ferie).await;
    let vide = seance_id(du + Duration::days(21)).await;
    // Le professeur a rempli le cahier de la séance du jour futur férié et donné un travail.
    sqlx::query(
        "INSERT INTO ent_seance_contenu (id, etablissement_id, seance_id, auteur_user_id, contenu)
         VALUES ($1,$2,$3,$4,'Chapitre 3 : les fractions')",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(porteuse)
    .bind(cast.prof.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    let classe_4b: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_classe WHERE etablissement_id = $1 AND annee_id = $2 AND code = '4B'",
    )
    .bind(e)
    .bind(annee.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let travail = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_travail (id, etablissement_id, seance_id, classe_id, matiere_libelle,
                auteur_user_id, titre, consigne, donne_le, pour_le)
         VALUES ($1,$2,$3,$4,'Maths',$5,'Exercices','Page 42',$6,$7)",
    )
    .bind(travail)
    .bind(e)
    .bind(porteuse)
    .bind(classe_4b)
    .bind(cast.prof.user_id)
    .bind(du)
    .bind(jour_ferie)
    .execute(ent.pool())
    .await
    .unwrap();

    // Défense en profondeur : même un DELETE direct est refusé tant que la séance porte quelque chose.
    let err = sqlx::query("DELETE FROM ent_edt_seance WHERE id = $1")
        .bind(porteuse)
        .execute(ent.pool())
        .await;
    assert!(err.is_err(), "la clé étrangère doit être RESTRICT");

    // La direction déclare le jour férié (et un second jour sans rien dessus).
    ent.ref_ajouter_jours(&a, jour_ferie, jour_ferie, "ferie", "Jour férié", None)
        .await
        .unwrap();
    ent.ref_ajouter_jours(
        &a,
        du + Duration::days(21),
        du + Duration::days(21),
        "ferie",
        "Pont",
        None,
    )
    .await
    .unwrap();
    let r = ent
        .ref_generer_seances(&a, annee.id, Some(du), Some(au))
        .await
        .unwrap();
    assert_eq!(r.retirees, 1, "{r:?}");
    assert_eq!(r.supprimees, 1, "{r:?}");

    // La séance porteuse existe toujours, retirée ; son contenu et le travail sont intacts.
    let (statut, retiree_le): (String, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT statut, retiree_le FROM ent_edt_seance WHERE id = $1")
            .bind(porteuse)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(statut, "retiree");
    assert!(retiree_le.is_some());
    let contenus: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_seance_contenu WHERE seance_id = $1")
            .bind(porteuse)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(contenus, 1, "le cahier a été effacé par la régénération");
    let sid: Option<Uuid> = sqlx::query_scalar("SELECT seance_id FROM ent_travail WHERE id = $1")
        .bind(travail)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(sid, Some(porteuse), "le travail a perdu sa séance");
    let encore: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance WHERE id = $1")
        .bind(vide)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(encore, 0, "la séance vide, elle, est supprimée comme avant");

    // Une séance retirée n'apparaît plus dans les lectures de l'emploi du temps.
    let liste = ent
        .ref_seances(&a, du, au, Some(classe_4b), None)
        .await
        .unwrap();
    assert!(liste.iter().all(|s| s.id != porteuse));

    // Idempotent : une seconde régénération ne retire ni ne supprime rien de plus.
    let r2 = ent
        .ref_generer_seances(&a, annee.id, Some(du), Some(au))
        .await
        .unwrap();
    assert_eq!((r2.creees, r2.retirees, r2.supprimees), (0, 0, 0), "{r2:?}");

    // La levée du jour férié rétablit la MÊME séance (identifiant stable), cahier toujours là.
    ent.ref_retirer_jour(&a, jour_ferie, None).await.unwrap();
    ent.ref_generer_seances(&a, annee.id, Some(du), Some(au))
        .await
        .unwrap();
    let (statut, retiree_le): (String, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT statut, retiree_le FROM ent_edt_seance WHERE id = $1")
            .bind(porteuse)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(statut, "prevue");
    assert!(retiree_le.is_none());
}

// ───────────────────────── D2 : D-022 sur l'EDT d'un élève ─────────────────────────

#[tokio::test]
async fn d2_professeur_hors_perimetre_refuse_sur_les_routes_edt_eleve() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let jour = today_utc();
    // Rogue enseigne en 4B ; Draco est en 5A : hors périmètre. Harry (4B) est dans le périmètre.
    let rogue = acteur(&cast, cast.prof.user_id, Role::Prof);
    assert!(ent
        .prof_suit_eleve(e, cast.prof.user_id, cast.eleve_lea_id)
        .await
        .unwrap());
    assert!(!ent
        .prof_suit_eleve(e, cast.prof.user_id, cast.eleve_tom_id)
        .await
        .unwrap());
    let (du, au) = (jour, jour + Duration::days(6));

    // 1. /edt/semaine?cible=eleve
    let r = ent
        .edt_semaine(&rogue, CibleEdt::Eleve(cast.eleve_tom_id), jour)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    assert!(ent
        .edt_semaine(&rogue, CibleEdt::Eleve(cast.eleve_lea_id), jour)
        .await
        .is_ok());
    // 2. /eleves/{id}/edt (et 3. .ics, qui passe par la même lecture du jour)
    let r = ent
        .list_edt_eleve_jour_for_actor(e, "prof", cast.prof.user_id, cast.eleve_tom_id, jour)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    assert!(ent
        .list_edt_eleve_jour_for_actor(e, "prof", cast.prof.user_id, cast.eleve_lea_id, jour)
        .await
        .is_ok());
    // 4. changements de l'élève
    let r = ent.edt_changements(&rogue, cast.eleve_tom_id, du, au).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    assert!(ent
        .edt_changements(&rogue, cast.eleve_lea_id, du, au)
        .await
        .is_ok());

    // La direction (périmètre d'établissement) lit toujours l'EDT de n'importe quel élève.
    let dir = acteur(&cast, cast.direction.user_id, Role::Direction);
    assert!(ent
        .edt_semaine(&dir, CibleEdt::Eleve(cast.eleve_tom_id), jour)
        .await
        .is_ok());
    assert!(ent
        .edt_changements(&dir, cast.eleve_tom_id, du, au)
        .await
        .is_ok());
}

// ───────────────────────── D3 : aucun motif interne pour un professeur ─────────────────────────

#[tokio::test]
async fn d3_motifs_internes_reserves_a_la_direction() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let jour = today_utc();
    sqlx::query(
        "INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, motif, created_by_user_id)
         VALUES ($1,$2,$3,$4,'annulation','Maladie grave de M. Rogue',$5)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(cast.creneau_rogue_4b)
    .bind(jour)
    .bind(cast.direction.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    let absence = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_absence_rh (id, etablissement_id, user_id, debut, fin, motif, statut, created_by_user_id)
         VALUES ($1,$2,$3,$4,$4,'Chimiothérapie','validee',$5)",
    )
    .bind(absence)
    .bind(e)
    .bind(cast.prof.user_id)
    .bind(jour)
    .bind(cast.direction.user_id)
    .execute(ent.pool())
    .await
    .unwrap();

    // Exceptions du jour : le professeur (et le CPE, qui gère l'EDT) ne reçoivent aucun motif.
    for role in ["prof", "cpe", "administratif"] {
        let l = ent.list_edt_exceptions_jour(e, role, jour).await.unwrap();
        assert!(
            l.iter().any(|x| x.creneau_id == cast.creneau_rogue_4b),
            "{role}"
        );
        assert!(
            l.iter().all(|x| x.motif.is_empty()),
            "{role} lit un motif interne : {l:?}"
        );
    }
    let l = ent
        .list_edt_exceptions_jour(e, "direction", jour)
        .await
        .unwrap();
    assert!(l.iter().any(|x| x.motif.contains("Maladie")));

    // Absences de personnel (socle 15, A0 : AR-01) : un collègue (professeur) ne voit AUCUNE ligne d'autrui
    // — la route n'est plus ouverte à tout le personnel ; la direction lit le motif ; l'agent concerné
    // retrouve le motif de SES absences seulement. Une seule règle : le cercle habilité.
    let autre_prof = cast
        .profs
        .iter()
        .find(|p| p.user_id != cast.prof.user_id)
        .unwrap();
    let l = ent
        .list_absences_rh_for_actor(e, autre_prof.user_id, "prof", jour, jour)
        .await
        .unwrap();
    assert!(l.iter().all(|a| a.id != absence), "{l:?}");
    assert!(l.iter().all(|a| a.motif.is_empty() || a.motif == "Absence"), "{l:?}");
    let l = ent
        .list_absences_rh_for_actor(e, cast.direction.user_id, "direction", jour, jour)
        .await
        .unwrap();
    assert!(l
        .iter()
        .any(|a| a.id == absence && a.motif == "Chimiothérapie"));
    let l = ent
        .list_absences_rh_for_actor(e, cast.prof.user_id, "prof", jour, jour)
        .await
        .unwrap();
    assert!(l
        .iter()
        .any(|a| a.id == absence && a.motif == "Chimiothérapie"));
}

// ───────────────────────── D4 : RLS posée sur les tables EDT ─────────────────────────

#[tokio::test]
async fn d4_rls_posee_sur_les_tables_edt_et_salles() {
    let Some((ent, _cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    for table in [
        "ent_edt_creneau",
        "ent_edt_seance",
        "ent_edt_exception",
        "ent_edt_deplacement",
        "ent_edt_annulation_masse",
        "ent_edt_calendrier_ab",
        "ent_edt_contrainte",
        "ent_edt_import",
        "ent_salle",
        "ent_absence_rh",
        "ent_delegation",
    ] {
        let (rls, force, politique): (bool, bool, i64) = sqlx::query_as(
            "SELECT c.relrowsecurity, c.relforcerowsecurity,
                    (SELECT count(*) FROM pg_policies p
                      WHERE p.tablename = c.relname AND p.policyname = 'ent_rls_etablissement')
               FROM pg_class c WHERE c.relname = $1 AND c.relnamespace = 'public'::regnamespace",
        )
        .bind(table)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        assert!(
            rls && force && politique == 1,
            "{table} : RLS {rls}, FORCE {force}, politiques {politique}"
        );
    }
}

// ───────────────────────── D8 : jour de l'établissement ─────────────────────────

#[tokio::test]
async fn d8_aujourdhui_est_le_jour_local_de_l_etablissement() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    sqlx::query(
        "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, 'Indian/Reunion')
         ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = 'Indian/Reunion'",
    )
    .bind(e)
    .execute(ent.pool())
    .await
    .unwrap();
    // La Réunion (UTC+4) à 23 h UTC : il est déjà le lendemain sur place.
    let local: NaiveDate =
        sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, '2026-11-11T23:00:00Z')")
            .bind(e)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(local, NaiveDate::from_ymd_opt(2026, 11, 12).unwrap());
    // Le service s'appuie sur cette même fonction.
    let attendu: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)")
        .bind(e)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(ent.aujourdhui_etablissement(e).await.unwrap(), attendu);
}

// ───────────────────────── D9 : le professeur importé est membre ─────────────────────────

#[tokio::test]
async fn d9_import_refuse_un_professeur_d_un_autre_etablissement() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let autre = ent
        .seed_labo_cast()
        .await
        .expect("seed autre établissement");
    let e = cast.etablissement_id;
    let csv = |prof: Uuid| {
        format!("classe;matiere;prof_user_id;jour;debut;fin\n4B;Maths;{prof};2;10:00;11:00\n")
    };

    // Professeur d'un AUTRE établissement : refusé, même en simulation, sans fuite d'existence.
    let r = ent
        .importer_edt_csv_format_for_actor(
            e,
            "direction",
            cast.direction.user_id,
            &csv(autre.prof.user_id),
            true,
            "native",
        )
        .await;
    assert!(r.is_err(), "professeur étranger accepté");
    let r = ent
        .importer_edt_csv_format_for_actor(
            e,
            "direction",
            cast.direction.user_id,
            &csv(Uuid::new_v4()),
            true,
            "native",
        )
        .await;
    assert!(r.is_err(), "identifiant inconnu accepté");
    // Professeur membre : accepté.
    let ok = ent
        .importer_edt_csv_format_for_actor(
            e,
            "direction",
            cast.direction.user_id,
            &csv(cast.prof.user_id),
            true,
            "native",
        )
        .await
        .unwrap();
    assert_eq!(ok.lines, 1);
}

// ───────────────────────── D19 : un collègue n'est vu que par sa présence ─────────────────────────

#[tokio::test]
async fn d19_collegue_reduit_a_sa_presence() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let jour = today_utc();
    let mcg: Uuid = sqlx::query_scalar("SELECT prof_user_id FROM ent_edt_creneau WHERE id = $1")
        .bind(cast.creneau_mcg_5a)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_ne!(mcg, cast.prof.user_id);

    let rogue = acteur(&cast, cast.prof.user_id, Role::Prof);
    let semaine = ent
        .edt_semaine(&rogue, CibleEdt::Personnel(mcg), jour)
        .await
        .unwrap();
    assert!(
        !semaine.seances.is_empty(),
        "la présence du collègue doit rester visible"
    );
    for s in &semaine.seances {
        assert!(s.classe.is_empty() && s.matiere_libelle.is_empty(), "{s:?}");
        assert!(
            s.salle.is_none() && s.prof_nom.is_none() && s.classe_id.is_none(),
            "{s:?}"
        );
        assert!(s.prof_user_id.is_nil() && s.creneau_id.is_nil(), "{s:?}");
    }
    // Son propre emploi du temps reste détaillé.
    let moi = ent
        .edt_semaine(&rogue, CibleEdt::Personnel(cast.prof.user_id), jour)
        .await
        .unwrap();
    assert!(moi.seances.iter().all(|s| !s.classe.is_empty()));
    // La direction lit le détail du collègue.
    let dir = acteur(&cast, cast.direction.user_id, Role::Direction);
    let detail = ent
        .edt_semaine(&dir, CibleEdt::Personnel(mcg), jour)
        .await
        .unwrap();
    assert!(detail.seances.iter().any(|s| s.classe == "5A"));
    let _ = e;
}
