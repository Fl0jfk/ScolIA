//! Socle 10 (emploi du temps), lots T5a (matérialisation depuis les placements) et T5 (publication).
//! Base locale de test (feature `labo`). Chaque test crée son propre établissement.
//!
//! T5a : les séances viennent des placements de la version en vigueur ; identité stable d'une version à l'autre ;
//! journal du réel (T3) conservé ; version temporaire puis retour du plan de base ; cours vacant non projeté ;
//! séparation entre établissements ; heure de Paris à travers le changement d'heure.

#![allow(clippy::too_many_arguments)]

use super::*;
use crate::edt_modif::{ChangementSeance, CmdModifierSeance};
use crate::edt_versions::{NouveauPlacement, PositionPlacement};
use crate::tests_edt_t1_t2::{adm, creneau, decor, dir, h, labo, Decor};
use chrono::{Datelike, Duration, NaiveDate};
use domain_eleve::horloge::jour_paris;
use uuid::Uuid;

/// Un lundi à venir (dans l'année courante), `n` semaines après le prochain.
pub(crate) fn lundi_futur(d: &Decor, n: i64) -> NaiveDate {
    let base = jour_paris().clamp(d.annee_debut, d.annee_fin);
    base + Duration::days((8 - base.weekday().number_from_monday() as i64) % 7 + 7 * n + 14)
}

/// Service ouvert (titulaire) sur un cours : c'est lui qui donne l'intervenant aux séances d'un placement.
pub(crate) async fn service(ent: &PgEnt, cast: &LaboCast, cours: Uuid, prof: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_service_enseignement (id, etablissement_id, annee_id, prof_user_id, matiere_libelle, cours_id)
         SELECT $1, $2, c.annee_id, $3, c.libelle, c.id FROM ent_cours c WHERE c.id = $4",
    )
    .bind(id)
    .bind(cast.etablissement_id)
    .bind(prof)
    .bind(cours)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

pub(crate) fn pos(jour: i16, code: &str, duree: i16, lieux: Vec<Uuid>) -> PositionPlacement {
    PositionPlacement {
        jour_semaine: jour,
        creneau_debut_code: code.into(),
        duree_creneaux: duree,
        semaine_type: "toutes".into(),
        lieux,
    }
}

/// Bascule « à la main » d'une version brouillon (la publication de T5 est testée plus bas) : la version en vigueur est
/// tronquée à la veille, la nouvelle devient publiée à partir de `du`.
pub(crate) async fn basculer_sql(ent: &PgEnt, e: Uuid, avant: Uuid, apres: Uuid, du: NaiveDate, fin: NaiveDate) {
    sqlx::query("UPDATE ent_edt_version SET valable_au = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(e)
        .bind(avant)
        .bind(du - Duration::days(1))
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE ent_edt_version SET etat = 'publiee', valable_du = $3, valable_au = $4, publiee_le = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(e)
    .bind(apres)
    .bind(du)
    .bind(fin)
    .execute(ent.pool())
    .await
    .unwrap();
}

async fn seance_du(ent: &PgEnt, e: Uuid, cours: Uuid, jour: NaiveDate) -> Option<(Uuid, String, Option<Uuid>, Option<Uuid>)> {
    sqlx::query_as(
        "SELECT id, etat, placement_id, version_id FROM ent_edt_seance
          WHERE etablissement_id = $1 AND cours_id = $2 AND jour = $3 AND statut <> 'retiree'",
    )
    .bind(e)
    .bind(cours)
    .bind(jour)
    .fetch_optional(ent.pool())
    .await
    .unwrap()
}

// ───────────────────────── T5a : identité stable, journal conservé, cours vacant ─────────────────────────

#[tokio::test]
async fn t5a_republier_garde_les_identifiants_et_le_journal_et_ajoute_les_nouvelles_seances() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let d = decor(&ent, &cast).await;
    let (a, g) = (adm(&cast), dir(&cast));
    service(&ent, &cast, d.cours, cast.prof.user_id).await;
    service(&ent, &cast, d.cours2, cast.prof.user_id).await;
    let lundi = lundi_futur(&d, 0);
    creneau(&ent, &cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    let r0 = ent
        .ref_generer_seances(&a, d.annee_id, Some(lundi), Some(lundi + Duration::days(41)))
        .await
        .unwrap();
    assert!(r0.creees >= 6, "{r0:?}");
    let (s_stable, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(7)).await.unwrap();
    let (s_annulee, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(14)).await.unwrap();
    // Le journal du réel (T3) : une séance annulée (motif interne séparé).
    let mut annuler = CmdModifierSeance {
        seance_id: Some(s_annulee),
        changement: ChangementSeance::Annulation,
        motif_public_code: "reunion".to_string(),
        motif_public: Some("Réunion pédagogique".to_string()),
        motif_interne: Some("confidentiel".to_string()),
        origine: None,
        origine_ref: None,
        cle: Some("t5a-annul".to_string()),
        forcer: None,
        apercu: false,
    };
    annuler.apercu = false;
    assert!(ent.edt_modifier_seance(&g, &annuler).await.unwrap().applique);

    // Version 2 : le même placement du lundi, plus le cours 2 le mardi (nouveau).
    let v1 = ent.edt_versions(&a, d.annee_id).await.unwrap().into_iter().find(|v| v.origine == "reprise").unwrap();
    let v2 = ent.edt_version_creer(&a, d.annee_id, "Version 2", None, None).await.unwrap();
    for (cours, p) in [(d.cours, pos(1, "M1", 1, vec![d.salle])), (d.cours2, pos(2, "M1", 1, vec![d.salle]))] {
        ent.edt_placement_creer(&a, NouveauPlacement { version_id: v2.id, cours_id: cours, position: p })
            .await
            .unwrap();
    }
    let effet = lundi + Duration::days(7);
    basculer_sql(&ent, e, v1.id, v2.id, effet, d.annee_fin).await;
    let r1 = ent
        .ref_generer_seances(&a, d.annee_id, Some(lundi), Some(lundi + Duration::days(41)))
        .await
        .unwrap();
    assert_eq!(r1.non_projetes, 0, "{r1:?}");
    assert_eq!(r1.supprimees + r1.retirees, 0, "rien ne disparaît : {r1:?}");

    // Identité stable : la séance du lundi reste la même, rattachée au placement de la nouvelle version.
    let (id, etat, placement, version) = seance_du(&ent, e, d.cours, effet).await.unwrap();
    assert_eq!(id, s_stable, "la séance garde son identifiant");
    assert_eq!((etat.as_str(), version), ("prevue", Some(v2.id)));
    let p2: Uuid = sqlx::query_scalar("SELECT id FROM ent_edt_placement WHERE version_id = $1 AND cours_id = $2")
        .bind(v2.id)
        .bind(d.cours)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(placement, Some(p2));
    // Avant la date d'effet : la séance reste celle de la version précédente.
    let (_, _, _, avant) = seance_du(&ent, e, d.cours, lundi).await.unwrap();
    assert_eq!(avant, Some(v1.id));
    // Le journal du réel est conservé : même séance, toujours annulée, modification vivante.
    let (id_a, etat_a, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(14)).await.unwrap();
    assert_eq!((id_a, etat_a.as_str()), (s_annulee, "annulee"));
    let vivantes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_modification WHERE etablissement_id = $1 AND seance_id = $2 AND levee_le IS NULL",
    )
    .bind(e)
    .bind(s_annulee)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(vivantes, 1);
    // Le cours 2 n'a de séances qu'à partir de la date d'effet, avec un créneau projeté.
    assert!(seance_du(&ent, e, d.cours2, lundi + Duration::days(1)).await.is_none());
    let (mardi, ..) = seance_du(&ent, e, d.cours2, effet + Duration::days(1)).await.unwrap();
    let projete: bool = sqlx::query_scalar(
        "SELECT c.projete FROM ent_edt_seance s JOIN ent_edt_creneau c ON c.id = s.creneau_id WHERE s.id = $1",
    )
    .bind(mardi)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(projete);
    // Recalculer ne change rien (idempotence).
    let r2 = ent
        .ref_generer_seances(&a, d.annee_id, Some(lundi), Some(lundi + Duration::days(41)))
        .await
        .unwrap();
    assert_eq!((r2.creees, r2.mises_a_jour, r2.supprimees, r2.retirees), (0, 0, 0, 0), "{r2:?}");
    // La reprise n'importe pas le créneau projeté dans l'ancienne version.
    let rep = ent.edt_reprendre_creneaux(&a, d.annee_id).await;
    if let Ok(rep) = rep {
        assert_eq!(rep.placements_crees, 0, "{rep:?}");
    }
}

#[tokio::test]
async fn t5a_changement_de_position_retire_l_ancienne_seance_sans_perdre_son_contenu() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let d = decor(&ent, &cast).await;
    let a = adm(&cast);
    service(&ent, &cast, d.cours, cast.prof.user_id).await;
    let lundi = lundi_futur(&d, 0);
    creneau(&ent, &cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    let fenetre = (Some(lundi), Some(lundi + Duration::days(27)));
    ent.ref_generer_seances(&a, d.annee_id, fenetre.0, fenetre.1).await.unwrap();
    let (porteuse, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(14)).await.unwrap();
    let (vide, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(21)).await.unwrap();
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

    // Nouvelle version : le cours passe du lundi au mercredi à partir du lundi + 14.
    let v1 = ent.edt_versions(&a, d.annee_id).await.unwrap().into_iter().find(|v| v.origine == "reprise").unwrap();
    let v2 = ent.edt_version_creer(&a, d.annee_id, "Cours au mercredi", None, None).await.unwrap();
    ent.edt_placement_creer(
        &a,
        NouveauPlacement { version_id: v2.id, cours_id: d.cours, position: pos(3, "M1", 1, vec![d.salle]) },
    )
    .await
    .unwrap();
    basculer_sql(&ent, e, v1.id, v2.id, lundi + Duration::days(14), d.annee_fin).await;
    let r = ent.ref_generer_seances(&a, d.annee_id, fenetre.0, fenetre.1).await.unwrap();
    assert_eq!((r.retirees, r.supprimees), (1, 1), "contenu : retirée ; vide : supprimée ; {r:?}");

    // La séance qui porte du contenu est retirée (jamais supprimée) ; la vide a disparu.
    let (etat, motif): (String, Option<String>) =
        sqlx::query_as("SELECT etat, retiree_motif FROM ent_edt_seance WHERE id = $1")
            .bind(porteuse)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(etat, "retiree");
    assert!(motif.is_some());
    let reste: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance WHERE id = $1")
        .bind(vide)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(reste, 0);
    // Les nouvelles séances du mercredi existent à partir de la date d'effet, pas avant.
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(16)).await.is_some());
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(9)).await.is_none());
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(7)).await.is_some(), "le passé reste");
}

#[tokio::test]
async fn t5a_version_temporaire_remplace_la_base_puis_la_base_revient() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let d = decor(&ent, &cast).await;
    let a = adm(&cast);
    service(&ent, &cast, d.cours, cast.prof.user_id).await;
    let lundi = lundi_futur(&d, 0);
    creneau(&ent, &cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    let fenetre = (Some(lundi), Some(lundi + Duration::days(34)));
    ent.ref_generer_seances(&a, d.annee_id, fenetre.0, fenetre.1).await.unwrap();
    let v0 = ent.edt_versions(&a, d.annee_id).await.unwrap().into_iter().find(|v| v.origine == "reprise").unwrap();

    // Temporaire (suppléant contraint : le cours se tient le jeudi) du lundi + 7 au lundi + 20.
    let t = ent.edt_version_creer(&a, d.annee_id, "Suppléance", Some(v0.id), Some("3 semaines")).await.unwrap();
    ent.edt_placement_creer(
        &a,
        NouveauPlacement { version_id: t.id, cours_id: d.cours, position: pos(4, "M1", 1, vec![d.salle]) },
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_edt_version_perimetre (version_id, etablissement_id, cours_id) VALUES ($1,$2,$3)")
        .bind(t.id)
        .bind(e)
        .bind(d.cours)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE ent_edt_version SET etat = 'publiee', valable_du = $3, valable_au = $4, publiee_le = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(e)
    .bind(t.id)
    .bind(lundi + Duration::days(7))
    .bind(lundi + Duration::days(20))
    .execute(ent.pool())
    .await
    .unwrap();
    ent.ref_generer_seances(&a, d.annee_id, fenetre.0, fenetre.1).await.unwrap();

    assert!(seance_du(&ent, e, d.cours, lundi).await.is_some(), "avant la plage : le plan de base");
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(7)).await.is_none(), "lundi remplacé");
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(10)).await.is_some(), "jeudi de la temporaire");
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(17)).await.is_some());
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(24)).await.is_none(), "jeudi après la plage");
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(21)).await.is_some(), "le plan de base revient seul");
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(28)).await.is_some());
}

#[tokio::test]
async fn t5a_cours_vacant_non_projete_puis_projete_quand_un_intervenant_arrive() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let d = decor(&ent, &cast).await;
    let a = adm(&cast);
    let lundi = lundi_futur(&d, 0);
    creneau(&ent, &cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    ent.ref_generer_seances(&a, d.annee_id, Some(lundi), Some(lundi + Duration::days(13))).await.unwrap();
    let v0 = ent.edt_versions(&a, d.annee_id).await.unwrap().into_iter().find(|v| v.origine == "reprise").unwrap();
    let v1 = ent.edt_version_creer(&a, d.annee_id, "Poste vacant", None, None).await.unwrap();
    ent.edt_placement_creer(
        &a,
        NouveauPlacement { version_id: v1.id, cours_id: d.cours2, position: pos(2, "M1", 1, vec![]) },
    )
    .await
    .unwrap();
    basculer_sql(&ent, e, v0.id, v1.id, lundi + Duration::days(7), d.annee_fin).await;
    let r = ent.ref_generer_seances(&a, d.annee_id, Some(lundi), Some(lundi + Duration::days(20))).await.unwrap();
    assert_eq!(r.non_projetes, 1, "cours sans intervenant : pas de séance inventée : {r:?}");
    assert!(seance_du(&ent, e, d.cours2, lundi + Duration::days(8)).await.is_none());
    // Le professeur arrive : le recalcul (idempotent) crée les séances, sans republier.
    service(&ent, &cast, d.cours2, cast.prof.user_id).await;
    let r = ent.ref_generer_seances(&a, d.annee_id, Some(lundi), Some(lundi + Duration::days(20))).await.unwrap();
    assert_eq!(r.non_projetes, 0);
    assert!(seance_du(&ent, e, d.cours2, lundi + Duration::days(8)).await.is_some());
}

// ───────────────────────── T5a : séparation entre établissements, heure de Paris ─────────────────────────

#[tokio::test]
async fn t5a_deux_etablissements_ne_se_voient_pas_et_l_heure_murale_traverse_le_changement_d_heure() {
    let Some((ent, cast_a)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cast_b = ent.seed_labo_cast().await.unwrap();
    let (ea, eb) = (cast_a.etablissement_id, cast_b.etablissement_id);
    let da = decor(&ent, &cast_a).await;
    let db_ = decor(&ent, &cast_b).await;
    service(&ent, &cast_a, da.cours, cast_a.prof.user_id).await;
    service(&ent, &cast_b, db_.cours, cast_b.prof.user_id).await;
    let (a, b) = (adm(&cast_a), adm(&cast_b));
    creneau(&ent, &cast_a, Some(da.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    creneau(&ent, &cast_b, Some(db_.cours), 2, h(10, 0), h(11, 0), Some("B12")).await;
    // Passage à l'heure d'été 2027 : dimanche 28 mars. L'année scolaire courante doit le contenir (sinon, test sans objet).
    let (du, au) = (NaiveDate::from_ymd_opt(2027, 3, 22).unwrap(), NaiveDate::from_ymd_opt(2027, 4, 4).unwrap());
    let du_a = du.clamp(da.annee_debut, da.annee_fin);
    let (du, au) = if du_a == du { (du, au) } else { (lundi_futur(&da, 0), lundi_futur(&da, 2)) };
    ent.ref_generer_seances(&a, da.annee_id, Some(du), Some(au)).await.unwrap();
    ent.ref_generer_seances(&b, db_.annee_id, Some(du), Some(au)).await.unwrap();

    // Chaque établissement n'a que ses séances, issues de ses placements.
    let croise: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance s
           JOIN ent_edt_placement p ON p.id = s.placement_id
          WHERE s.etablissement_id IN ($1,$2) AND s.etablissement_id <> p.etablissement_id",
    )
    .bind(ea)
    .bind(eb)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(croise, 0);
    let (na, nb): (i64, i64) = (
        sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2")
            .bind(ea)
            .bind(da.cours)
            .fetch_one(ent.pool())
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2")
            .bind(eb)
            .bind(da.cours)
            .fetch_one(ent.pool())
            .await
            .unwrap(),
    );
    assert!(na >= 2 && nb == 0, "{na} {nb}");
    // Heure murale stable (8 h) ; l'instant UTC suit le fuseau de Paris (7 h UTC en hiver, 6 h UTC en été).
    let lignes: Vec<(NaiveDate, chrono::NaiveTime, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT jour, heure_debut, debut_utc FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2 ORDER BY jour",
    )
    .bind(ea)
    .bind(da.cours)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    for (jour, debut, utc) in &lignes {
        assert_eq!(*debut, h(8, 0));
        let ete = *jour >= NaiveDate::from_ymd_opt(2027, 3, 28).unwrap() && *jour < NaiveDate::from_ymd_opt(2027, 10, 31).unwrap()
            || (*jour >= NaiveDate::from_ymd_opt(2026, 3, 29).unwrap() && *jour < NaiveDate::from_ymd_opt(2026, 10, 25).unwrap());
        let attendu = if ete { 6 } else { 7 };
        assert_eq!(utc.format("%H").to_string(), format!("{attendu:02}"), "{jour}");
    }
}

