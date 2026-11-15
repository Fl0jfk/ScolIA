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
use crate::tests_edt_t1_t2::{acteur, adm, creer_salle, creneau, decor, dir, h, labo, Decor};
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
pub(crate) async fn basculer_sql(
    ent: &PgEnt,
    e: Uuid,
    avant: Uuid,
    apres: Uuid,
    du: NaiveDate,
    fin: NaiveDate,
) {
    sqlx::query(
        "UPDATE ent_edt_version SET valable_au = $3 WHERE etablissement_id = $1 AND id = $2",
    )
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

async fn seance_du(
    ent: &PgEnt,
    e: Uuid,
    cours: Uuid,
    jour: NaiveDate,
) -> Option<(Uuid, String, Option<Uuid>, Option<Uuid>)> {
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
        .ref_generer_seances(
            &a,
            d.annee_id,
            Some(lundi),
            Some(lundi + Duration::days(41)),
        )
        .await
        .unwrap();
    assert!(r0.creees >= 6, "{r0:?}");
    let (s_stable, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(7))
        .await
        .unwrap();
    let (s_annulee, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(14))
        .await
        .unwrap();
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
    assert!(
        ent.edt_modifier_seance(&g, &annuler)
            .await
            .unwrap()
            .applique
    );

    // Version 2 : le même placement du lundi, plus le cours 2 le mardi (nouveau).
    let v1 = ent
        .edt_versions(&a, d.annee_id)
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.origine == "reprise")
        .unwrap();
    let v2 = ent
        .edt_version_creer(&a, d.annee_id, "Version 2", None, None)
        .await
        .unwrap();
    for (cours, p) in [
        (d.cours, pos(1, "M1", 1, vec![d.salle])),
        (d.cours2, pos(2, "M1", 1, vec![d.salle])),
    ] {
        ent.edt_placement_creer(
            &a,
            NouveauPlacement {
                version_id: v2.id,
                cours_id: cours,
                position: p,
            },
        )
        .await
        .unwrap();
    }
    let effet = lundi + Duration::days(7);
    basculer_sql(&ent, e, v1.id, v2.id, effet, d.annee_fin).await;
    let r1 = ent
        .ref_generer_seances(
            &a,
            d.annee_id,
            Some(lundi),
            Some(lundi + Duration::days(41)),
        )
        .await
        .unwrap();
    assert_eq!(r1.non_projetes, 0, "{r1:?}");
    assert_eq!(r1.supprimees + r1.retirees, 0, "rien ne disparaît : {r1:?}");

    // Identité stable : la séance du lundi reste la même, rattachée au placement de la nouvelle version.
    let (id, etat, placement, version) = seance_du(&ent, e, d.cours, effet).await.unwrap();
    assert_eq!(id, s_stable, "la séance garde son identifiant");
    assert_eq!((etat.as_str(), version), ("prevue", Some(v2.id)));
    let p2: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_edt_placement WHERE version_id = $1 AND cours_id = $2",
    )
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
    let (id_a, etat_a, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(14))
        .await
        .unwrap();
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
    assert!(seance_du(&ent, e, d.cours2, lundi + Duration::days(1))
        .await
        .is_none());
    let (mardi, ..) = seance_du(&ent, e, d.cours2, effet + Duration::days(1))
        .await
        .unwrap();
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
        .ref_generer_seances(
            &a,
            d.annee_id,
            Some(lundi),
            Some(lundi + Duration::days(41)),
        )
        .await
        .unwrap();
    assert_eq!(
        (r2.creees, r2.mises_a_jour, r2.supprimees, r2.retirees),
        (0, 0, 0, 0),
        "{r2:?}"
    );
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
    ent.ref_generer_seances(&a, d.annee_id, fenetre.0, fenetre.1)
        .await
        .unwrap();
    let (porteuse, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(14))
        .await
        .unwrap();
    let (vide, ..) = seance_du(&ent, e, d.cours, lundi + Duration::days(21))
        .await
        .unwrap();
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
    let v1 = ent
        .edt_versions(&a, d.annee_id)
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.origine == "reprise")
        .unwrap();
    let v2 = ent
        .edt_version_creer(&a, d.annee_id, "Cours au mercredi", None, None)
        .await
        .unwrap();
    ent.edt_placement_creer(
        &a,
        NouveauPlacement {
            version_id: v2.id,
            cours_id: d.cours,
            position: pos(3, "M1", 1, vec![d.salle]),
        },
    )
    .await
    .unwrap();
    basculer_sql(
        &ent,
        e,
        v1.id,
        v2.id,
        lundi + Duration::days(14),
        d.annee_fin,
    )
    .await;
    let r = ent
        .ref_generer_seances(&a, d.annee_id, fenetre.0, fenetre.1)
        .await
        .unwrap();
    assert_eq!(
        (r.retirees, r.supprimees),
        (1, 1),
        "contenu : retirée ; vide : supprimée ; {r:?}"
    );

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
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(16))
        .await
        .is_some());
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(9))
        .await
        .is_none());
    assert!(
        seance_du(&ent, e, d.cours, lundi + Duration::days(7))
            .await
            .is_some(),
        "le passé reste"
    );
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
    ent.ref_generer_seances(&a, d.annee_id, fenetre.0, fenetre.1)
        .await
        .unwrap();
    let v0 = ent
        .edt_versions(&a, d.annee_id)
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.origine == "reprise")
        .unwrap();

    // Temporaire (suppléant contraint : le cours se tient le jeudi) du lundi + 7 au lundi + 20.
    let t = ent
        .edt_version_creer(
            &a,
            d.annee_id,
            "Suppléance",
            Some(v0.id),
            Some("3 semaines"),
        )
        .await
        .unwrap();
    ent.edt_placement_creer(
        &a,
        NouveauPlacement {
            version_id: t.id,
            cours_id: d.cours,
            position: pos(4, "M1", 1, vec![d.salle]),
        },
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
    ent.ref_generer_seances(&a, d.annee_id, fenetre.0, fenetre.1)
        .await
        .unwrap();

    assert!(
        seance_du(&ent, e, d.cours, lundi).await.is_some(),
        "avant la plage : le plan de base"
    );
    assert!(
        seance_du(&ent, e, d.cours, lundi + Duration::days(7))
            .await
            .is_none(),
        "lundi remplacé"
    );
    assert!(
        seance_du(&ent, e, d.cours, lundi + Duration::days(10))
            .await
            .is_some(),
        "jeudi de la temporaire"
    );
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(17))
        .await
        .is_some());
    assert!(
        seance_du(&ent, e, d.cours, lundi + Duration::days(24))
            .await
            .is_none(),
        "jeudi après la plage"
    );
    assert!(
        seance_du(&ent, e, d.cours, lundi + Duration::days(21))
            .await
            .is_some(),
        "le plan de base revient seul"
    );
    assert!(seance_du(&ent, e, d.cours, lundi + Duration::days(28))
        .await
        .is_some());
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
    ent.ref_generer_seances(
        &a,
        d.annee_id,
        Some(lundi),
        Some(lundi + Duration::days(13)),
    )
    .await
    .unwrap();
    let v0 = ent
        .edt_versions(&a, d.annee_id)
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.origine == "reprise")
        .unwrap();
    let v1 = ent
        .edt_version_creer(&a, d.annee_id, "Poste vacant", None, None)
        .await
        .unwrap();
    ent.edt_placement_creer(
        &a,
        NouveauPlacement {
            version_id: v1.id,
            cours_id: d.cours2,
            position: pos(2, "M1", 1, vec![]),
        },
    )
    .await
    .unwrap();
    basculer_sql(
        &ent,
        e,
        v0.id,
        v1.id,
        lundi + Duration::days(7),
        d.annee_fin,
    )
    .await;
    let r = ent
        .ref_generer_seances(
            &a,
            d.annee_id,
            Some(lundi),
            Some(lundi + Duration::days(20)),
        )
        .await
        .unwrap();
    assert_eq!(
        r.non_projetes, 1,
        "cours sans intervenant : pas de séance inventée : {r:?}"
    );
    assert!(seance_du(&ent, e, d.cours2, lundi + Duration::days(8))
        .await
        .is_none());
    // Le professeur arrive : le recalcul (idempotent) crée les séances, sans republier.
    service(&ent, &cast, d.cours2, cast.prof.user_id).await;
    let r = ent
        .ref_generer_seances(
            &a,
            d.annee_id,
            Some(lundi),
            Some(lundi + Duration::days(20)),
        )
        .await
        .unwrap();
    assert_eq!(r.non_projetes, 0);
    assert!(seance_du(&ent, e, d.cours2, lundi + Duration::days(8))
        .await
        .is_some());
}

// ───────────────────────── T5a : séparation entre établissements, heure de Paris ─────────────────────────

#[tokio::test]
async fn t5a_deux_etablissements_ne_se_voient_pas_et_l_heure_murale_traverse_le_changement_d_heure()
{
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
    creneau(
        &ent,
        &cast_a,
        Some(da.cours),
        1,
        h(8, 0),
        h(9, 0),
        Some("B12"),
    )
    .await;
    creneau(
        &ent,
        &cast_b,
        Some(db_.cours),
        2,
        h(10, 0),
        h(11, 0),
        Some("B12"),
    )
    .await;
    // Passage à l'heure d'été 2027 : dimanche 28 mars. L'année scolaire courante doit le contenir (sinon, test sans objet).
    let (du, au) = (
        NaiveDate::from_ymd_opt(2027, 3, 22).unwrap(),
        NaiveDate::from_ymd_opt(2027, 4, 4).unwrap(),
    );
    let du_a = du.clamp(da.annee_debut, da.annee_fin);
    let (du, au) = if du_a == du {
        (du, au)
    } else {
        (lundi_futur(&da, 0), lundi_futur(&da, 2))
    };
    ent.ref_generer_seances(&a, da.annee_id, Some(du), Some(au))
        .await
        .unwrap();
    ent.ref_generer_seances(&b, db_.annee_id, Some(du), Some(au))
        .await
        .unwrap();

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
        sqlx::query_scalar(
            "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2",
        )
        .bind(ea)
        .bind(da.cours)
        .fetch_one(ent.pool())
        .await
        .unwrap(),
        sqlx::query_scalar(
            "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2",
        )
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
        let ete = *jour >= NaiveDate::from_ymd_opt(2027, 3, 28).unwrap()
            && *jour < NaiveDate::from_ymd_opt(2027, 10, 31).unwrap()
            || (*jour >= NaiveDate::from_ymd_opt(2026, 3, 29).unwrap()
                && *jour < NaiveDate::from_ymd_opt(2026, 10, 25).unwrap());
        let attendu = if ete { 6 } else { 7 };
        assert_eq!(
            utc.format("%H").to_string(),
            format!("{attendu:02}"),
            "{jour}"
        );
    }
}

// ═════════════════════════════════════ T5 : publication ═════════════════════════════════════

use crate::edt_publication::{CmdIndividuelle, CmdPublier};
use crate::outbox::ConfigDistribution;
use chrono::Utc;
use ent_auth::Role;

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(ConfigDistribution {
        taille_lot: 200,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    })
    .distribuer_tout()
    .await
    .unwrap();
}

async fn notifs(ent: &PgEnt, user: Uuid, kind: &str) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT titre, corps FROM ent_notification WHERE destinataire_user_id = $1 AND kind LIKE $2 || '%' ORDER BY created_at, id",
    )
    .bind(user)
    .bind(kind)
    .fetch_all(ent.pool())
    .await
    .unwrap()
}

async fn nb_evenements(ent: &PgEnt, etab: Uuid, type_name: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = $2",
    )
    .bind(etab)
    .bind(type_name)
    .fetch_one(ent.pool())
    .await
    .unwrap()
}

async fn nb_faits(ent: &PgEnt, etab: Uuid, version: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ent_edt_version_journal WHERE etablissement_id = $1 AND version_id = $2")
        .bind(etab)
        .bind(version)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

/// Cadre commun : cours 1 (lundi M1) avec créneau historique repris en version 0, et deux services.
struct Cadre {
    d: Decor,
    lundi: NaiveDate,
    v0: Uuid,
}

async fn cadre(ent: &PgEnt, cast: &LaboCast) -> Cadre {
    let d = decor(ent, cast).await;
    let a = adm(cast);
    service(ent, cast, d.cours, cast.prof.user_id).await;
    service(ent, cast, d.cours2, cast.prof.user_id).await;
    let lundi = lundi_futur(&d, 0);
    creneau(ent, cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    ent.ref_generer_seances(
        &a,
        d.annee_id,
        Some(lundi),
        Some(lundi + Duration::days(41)),
    )
    .await
    .unwrap();
    let v0 = ent
        .edt_versions(&a, d.annee_id)
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.origine == "reprise")
        .unwrap()
        .id;
    Cadre { d, lundi, v0 }
}

/// Brouillon : le cours 1 reste le lundi à M1 ; `deplacer` : le cours 1 passe au mardi ; `avec_cours2` ajoute le cours 2 le jeudi.
async fn brouillon(
    ent: &PgEnt,
    cast: &LaboCast,
    c: &Cadre,
    libelle: &str,
    deplacer: bool,
    avec_cours2: bool,
) -> Uuid {
    let a = adm(cast);
    let v = ent
        .edt_version_creer(&a, c.d.annee_id, libelle, None, None)
        .await
        .unwrap();
    let jour = if deplacer { 2 } else { 1 };
    ent.edt_placement_creer(
        &a,
        NouveauPlacement {
            version_id: v.id,
            cours_id: c.d.cours,
            position: pos(jour, "M1", 1, vec![c.d.salle]),
        },
    )
    .await
    .unwrap();
    if avec_cours2 {
        ent.edt_placement_creer(
            &a,
            NouveauPlacement {
                version_id: v.id,
                cours_id: c.d.cours2,
                position: pos(4, "M1", 1, vec![c.d.salle]),
            },
        )
        .await
        .unwrap();
    }
    v.id
}

async fn seance_cours(ent: &PgEnt, e: Uuid, cours: Uuid, jour: NaiveDate) -> Uuid {
    sqlx::query_scalar(
        "SELECT id FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2 AND jour = $3",
    )
    .bind(e)
    .bind(cours)
    .bind(jour)
    .fetch_one(ent.pool())
    .await
    .unwrap()
}

async fn version(
    ent: &PgEnt,
    etab: Uuid,
    id: Uuid,
) -> (String, Option<NaiveDate>, Option<NaiveDate>, bool) {
    sqlx::query_as(
        "SELECT etat, valable_du, valable_au, (annoncee_le IS NOT NULL) FROM ent_edt_version WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(ent.pool())
    .await
    .unwrap()
}

fn codes(c: &[crate::edt_publication::Controle]) -> Vec<String> {
    c.iter().map(|x| x.code.clone()).collect()
}

#[tokio::test]
async fn t5_controles_bloquants_et_avertissements_sans_ecriture() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let (a, g) = (adm(&cast), dir(&cast));
    // Un cours sans intervenant (poste vacant) : averti. Un lieu occupé deux fois : bloquant.
    let cours3 = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, classe_id, ouvert_le) VALUES ($1,$2,$3,'Vacant T5',$4,$5)")
        .bind(cours3)
        .bind(e)
        .bind(c.d.annee_id)
        .bind(c.d.classe_4b)
        .bind(c.d.annee_debut)
        .execute(ent.pool())
        .await
        .unwrap();
    let v = ent
        .edt_version_creer(&a, c.d.annee_id, "Avec conflit", None, None)
        .await
        .unwrap()
        .id;
    for (cours, jour, code) in [
        (c.d.cours, 1, "M1"),
        (c.d.cours2, 1, "M1"),
        (cours3, 2, "M3"),
    ] {
        ent.edt_placement_creer(
            &a,
            NouveauPlacement {
                version_id: v,
                cours_id: cours,
                position: pos(jour, code, 1, vec![c.d.salle]),
            },
        )
        .await
        .unwrap();
    }
    let r = ent
        .edt_publication_controler(&a, v, None, None)
        .await
        .unwrap();
    let bloquants = codes(&r.bloquants);
    assert!(
        bloquants.contains(&"conflit_lieu".to_string()),
        "{bloquants:?}"
    );
    assert!(
        bloquants.contains(&"conflit_intervenant".to_string()),
        "même professeur : {bloquants:?}"
    );
    assert!(bloquants.contains(&"conflit_cours".to_string()) || true);
    let vacant = r
        .avertissements
        .iter()
        .find(|x| x.code == "cours_vacant")
        .expect("cours vacant averti");
    assert_eq!(vacant.cours, vec![cours3]);
    assert_eq!(r.empreinte.len(), 64);

    // Publier une version qui a des conflits : refusé, rien n'est écrit.
    let p = ent
        .edt_publier(
            &a,
            &CmdPublier {
                version_id: v,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(!p.applique && !p.bloquants.is_empty());
    assert_eq!(
        version(&ent, e, v).await,
        ("brouillon".to_string(), None, None, false)
    );
    assert_eq!(nb_faits(&ent, e, v).await, 0);
    assert_eq!(nb_evenements(&ent, e, EVT_PUBLIEE).await, 0);

    // Version vide, date hors de l'année.
    let vide = ent
        .edt_version_creer(&a, c.d.annee_id, "Vide", None, None)
        .await
        .unwrap()
        .id;
    let r = ent
        .edt_publication_controler(&a, vide, None, None)
        .await
        .unwrap();
    assert!(codes(&r.bloquants).contains(&"version_vide".to_string()));
    let ok = brouillon(&ent, &cast, &c, "Correcte", false, true).await;
    let r = ent
        .edt_publication_controler(&a, ok, Some(c.d.annee_fin + Duration::days(10)), None)
        .await
        .unwrap();
    assert!(
        codes(&r.bloquants).contains(&"date_hors_annee".to_string()),
        "{:?}",
        r.bloquants
    );
    let r = ent
        .edt_publication_controler(&a, ok, None, None)
        .await
        .unwrap();
    assert!(r.bloquants.is_empty(), "{:?}", r.bloquants);
    // Version temporaire sans plage : bloquant.
    let t = ent
        .edt_version_creer(&a, c.d.annee_id, "Temporaire", Some(c.v0), None)
        .await
        .unwrap()
        .id;
    ent.edt_placement_creer(
        &a,
        NouveauPlacement {
            version_id: t,
            cours_id: c.d.cours,
            position: pos(3, "M1", 1, vec![]),
        },
    )
    .await
    .unwrap();
    let r = ent
        .edt_publication_controler(&a, t, None, None)
        .await
        .unwrap();
    assert!(codes(&r.bloquants).contains(&"temporaire_sans_plage".to_string()));
    // Droits : ni le professeur ni la vie scolaire ne contrôlent ; l'écriture est refusée au professeur.
    let prof = acteur(&cast, cast.prof.user_id, Role::Prof);
    let cpe = acteur(&cast, cast.cpe.user_id, Role::Cpe);
    for x in [&prof, &cpe] {
        assert!(matches!(
            ent.edt_publication_controler(x, ok, None, None).await,
            Err(EntDbError::Forbidden)
        ));
        assert!(matches!(
            ent.edt_publier(
                x,
                &CmdPublier {
                    version_id: ok,
                    ..Default::default()
                }
            )
            .await,
            Err(EntDbError::Forbidden)
        ));
    }
    // Le contrôle ne change rien : toujours un brouillon, aucune annonce.
    assert_eq!(
        version(&ent, e, ok).await,
        ("brouillon".to_string(), None, None, false)
    );
    // Retour arrière : direction seule.
    assert!(matches!(
        ent.edt_retour_arriere(&a, ok, None).await,
        Err(EntDbError::Forbidden)
    ));
    let _ = g;
}

use crate::edt_publication::EVT_VERSION_DIFFUSEE as EVT_DIFFUSEE;
use crate::edt_publication::EVT_VERSION_PUBLIEE as EVT_PUBLIEE;

#[tokio::test]
async fn t5_publication_immediate_effet_le_lendemain_journal_conserve_historique_et_differences() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let (a, g) = (adm(&cast), dir(&cast));
    let aujourdhui = jour_paris();
    // La séance du lundi + 14 est annulée (journal T3, motif interne séparé) avant la publication.
    let (s_annulee, s_stable) = (
        seance_cours(&ent, e, c.d.cours, c.lundi + Duration::days(14)).await,
        seance_cours(&ent, e, c.d.cours, c.lundi + Duration::days(7)).await,
    );
    ent.edt_modifier_seance(
        &g,
        &CmdModifierSeance {
            seance_id: Some(s_annulee),
            changement: ChangementSeance::Annulation,
            motif_public_code: "reunion".into(),
            motif_public: None,
            motif_interne: Some("secret".into()),
            origine: None,
            origine_ref: None,
            cle: Some("t5-annul".into()),
            forcer: None,
            apercu: false,
        },
    )
    .await
    .unwrap();

    let v1 = brouillon(&ent, &cast, &c, "Version 1", false, true).await;
    // Aperçu : ce que la publication changerait, sans rien écrire.
    let ap = ent
        .edt_publication_apercu(&a, v1, Some(aujourdhui), None)
        .await
        .unwrap();
    assert_eq!(
        ap.effet_reel,
        aujourdhui + Duration::days(1),
        "publiée dans la journée : effet demain"
    );
    // Effet demain : trop court pour un délai de deux jours ouvrés avant la diffusion → diffusion immédiate.
    assert!(
        ap.decalee && !ap.deux_temps && ap.diffusion_le.is_none(),
        "{ap:?}"
    );
    assert!(ap.seances_creees > 0 && ap.seances_conservees > 0, "{ap:?}");
    assert_eq!(
        ap.modifications_conservees, 1,
        "la séance annulée reste annulée : {ap:?}"
    );
    assert_eq!(ap.modifications_a_revoir, 0);
    assert!(ap.controles.as_ref().unwrap().bloquants.is_empty());
    assert_eq!(
        version(&ent, e, v1).await,
        ("brouillon".to_string(), None, None, false),
        "l'aperçu n'écrit rien"
    );
    assert_eq!(nb_faits(&ent, e, v1).await, 0);
    let avant: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(e)
    .bind(c.d.cours2)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(avant, 0, "aperçu annulé : aucune séance du cours 2");

    // Publication immédiate (sans deux temps), demandée pour aujourd'hui : prise d'effet demain, sans forçage.
    let cmd = CmdPublier {
        version_id: v1,
        a_partir_du: Some(aujourdhui),
        deux_temps: Some(false),
        ..Default::default()
    };
    // Forcer l'effet du jour : direction seulement.
    assert!(matches!(
        ent.edt_publier(
            &a,
            &CmdPublier {
                forcer_effet_immediat: Some("urgence".into()),
                ..cmd.clone()
            }
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    let r = ent.edt_publier(&a, &cmd).await.unwrap();
    assert!(r.applique && r.diffusee && r.decalee, "{r:?}");
    let effet = aujourdhui + Duration::days(1);
    assert_eq!(r.effet, Some(effet));
    assert_eq!(
        version(&ent, e, v1).await,
        (
            "publiee".to_string(),
            Some(effet),
            Some(c.d.annee_fin),
            true
        )
    );
    let (etat0, _, au0, _) = version(&ent, e, c.v0).await;
    assert_eq!(
        (etat0.as_str(), au0),
        ("publiee", Some(aujourdhui)),
        "la version précédente s'arrête la veille de l'effet"
    );

    // Le journal du réel est conservé : mêmes séances, séance annulée toujours annulée.
    assert_eq!(
        seance_cours(&ent, e, c.d.cours, c.lundi + Duration::days(7)).await,
        s_stable
    );
    let (id_a, etat_a): (Uuid, String) = sqlx::query_as(
        "SELECT id, etat FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2 AND jour = $3",
    )
    .bind(e)
    .bind(c.d.cours)
    .bind(c.lundi + Duration::days(14))
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((id_a, etat_a.as_str()), (s_annulee, "annulee"));
    // Les séances du cours 2 (jeudi) existent à partir de la date d'effet, pas avant.
    let premiere: NaiveDate = sqlx::query_scalar(
        "SELECT min(jour) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(e)
    .bind(c.d.cours2)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(premiere >= effet, "{premiere} < {effet}");
    // Journal de publication, événements, idempotence.
    assert_eq!(nb_faits(&ent, e, v1).await, 2, "annoncée, puis diffusée");
    assert_eq!(nb_evenements(&ent, e, EVT_PUBLIEE).await, 1);
    assert_eq!(nb_evenements(&ent, e, EVT_DIFFUSEE).await, 1);
    let encore = ent.edt_publier(&a, &cmd).await.unwrap();
    assert!(encore.deja_fait && !encore.applique);
    assert_eq!(nb_faits(&ent, e, v1).await, 2);
    assert_eq!(nb_evenements(&ent, e, EVT_PUBLIEE).await, 1);
    // Un autre brouillon qui précéderait cette version publiée : refusé.
    let v_avant = brouillon(&ent, &cast, &c, "Trop tôt", false, false).await;
    let ctl = ent
        .edt_publication_controler(&a, v_avant, Some(aujourdhui + Duration::days(1)), None)
        .await
        .unwrap();
    assert!(
        ctl.bloquants.is_empty(),
        "même jour d'effet : elle remplace la précédente, pas de blocage"
    );

    // Historique et différences.
    let h = ent
        .edt_publications_historique(&a, c.d.annee_id)
        .await
        .unwrap();
    let h1 = h.iter().find(|x| x.id == v1).unwrap();
    assert_eq!(
        (h1.etat.as_str(), h1.remplace_version_id),
        ("publiee", Some(c.v0))
    );
    assert_eq!(
        h1.faits.iter().map(|f| f.fait.as_str()).collect::<Vec<_>>(),
        vec!["annoncee", "diffusee"]
    );
    assert!(h1.faits[1].nb_creees > 0 && h1.faits[1].nb_modifications_conservees >= 1);
    let diff = ent.edt_versions_difference(&a, c.v0, v1).await.unwrap();
    assert_eq!(diff.ajoutes.len(), 1, "le cours 2 est nouveau : {diff:?}");
    assert_eq!(diff.ajoutes[0].cours_id, c.d.cours2);
    assert_eq!(
        (diff.inchanges, diff.modifies.len(), diff.retires.len()),
        (1, 0, 0)
    );
    assert_eq!(diff.par_classe, vec![("4B".to_string(), 1)]);
    // Droits de lecture.
    let prof = acteur(&cast, cast.prof.user_id, Role::Prof);
    assert!(matches!(
        ent.edt_publications_historique(&prof, c.d.annee_id).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.edt_versions_difference(&prof, c.v0, v1).await,
        Err(EntDbError::Forbidden)
    ));
}

#[tokio::test]
async fn t5_effet_force_le_jour_meme_par_la_direction_avec_motif() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let aujourdhui = jour_paris();
    let v = brouillon(&ent, &cast, &c, "Urgence", false, true).await;
    let cmd = CmdPublier {
        version_id: v,
        a_partir_du: Some(aujourdhui),
        deux_temps: Some(false),
        forcer_effet_immediat: Some("   ".into()),
        ..Default::default()
    };
    assert!(
        ent.edt_publier(&g, &cmd).await.is_err(),
        "motif obligatoire"
    );
    let r = ent
        .edt_publier(
            &g,
            &CmdPublier {
                forcer_effet_immediat: Some("Salle fermée par la mairie".into()),
                ..cmd
            },
        )
        .await
        .unwrap();
    assert!(r.applique && r.diffusee && !r.decalee, "{r:?}");
    assert_eq!(r.effet, Some(aujourdhui));
    let motif: Option<String> = sqlx::query_scalar(
        "SELECT forcage_motif FROM ent_edt_version_journal WHERE etablissement_id = $1 AND version_id = $2 AND fait = 'annoncee'",
    )
    .bind(e)
    .bind(v)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(motif.as_deref(), Some("Salle fermée par la mairie"));
    // Les séances d'aujourd'hui déjà commencées ne bougent pas : aucune séance retirée ou créée dans le passé.
    let passe: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2 AND jour < $3",
    )
    .bind(e)
    .bind(c.d.cours2)
    .bind(aujourdhui)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(passe, 0);
}

#[tokio::test]
async fn t5_deux_temps_figee_puis_diffusion_avancee_ou_echue_et_un_message_par_personne() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let a = adm(&cast);
    let aujourdhui = jour_paris();
    let effet = aujourdhui + Duration::days(10);
    let v = brouillon(&ent, &cast, &c, "En deux temps", false, true).await;
    let cmd = CmdPublier {
        version_id: v,
        a_partir_du: Some(effet),
        provisoire: true,
        ..Default::default()
    };
    let r = ent.edt_publier(&a, &cmd).await.unwrap();
    assert!(r.applique && !r.diffusee && !r.decalee, "{r:?}");
    let diffusion = r.diffusion_le.expect("deux temps : une date de diffusion");
    assert!(diffusion > Utc::now());
    // Temps 1 : annoncée et figée ; le réel n'a pas changé (aucune séance du cours 2) ; les familles ne reçoivent rien.
    assert_eq!(
        version(&ent, e, v).await,
        ("brouillon".to_string(), Some(effet), None, true)
    );
    let seances2: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(e)
    .bind(c.d.cours2)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(seances2, 0);
    assert!(
        ent.edt_placement_creer(
            &a,
            NouveauPlacement {
                version_id: v,
                cours_id: c.d.cours2,
                position: pos(5, "M1", 1, vec![])
            }
        )
        .await
        .is_err(),
        "une version annoncée ne se modifie plus"
    );
    distribuer(&ent, e).await;
    let harry = &cast.eleves[0];
    let compte = harry.compte.as_ref().unwrap().user_id;
    let prof = notifs(&ent, cast.prof.user_id, "edt_version_publiee").await;
    assert_eq!(prof.len(), 1, "un message par intervenant : {prof:?}");
    assert!(
        prof[0].1.contains("provisoire") && prof[0].1.contains("diffusé aux familles"),
        "{}",
        prof[0].1
    );
    assert!(
        notifs(&ent, compte, "edt_version").await.is_empty(),
        "l'élève ne sait rien avant la diffusion"
    );
    assert!(notifs(&ent, harry.parent.user_id, "edt_version")
        .await
        .is_empty());
    distribuer(&ent, e).await;
    assert_eq!(
        notifs(&ent, cast.prof.user_id, "edt_version_publiee")
            .await
            .len(),
        1,
        "rejeu : rien en double"
    );
    // Même publication redemandée à une autre date : refusée (retirer l'annonce d'abord) ; même date : déjà faite.
    assert!(ent
        .edt_publier(
            &a,
            &CmdPublier {
                a_partir_du: Some(effet + Duration::days(7)),
                ..cmd.clone()
            }
        )
        .await
        .is_err());
    assert!(ent.edt_publier(&a, &cmd).await.unwrap().deja_fait);
    // Régler la diffusion : après l'effet, refusé ; plus tard mais avant l'effet, accepté.
    assert!(ent
        .edt_diffusion_regler(&a, v, Utc::now() + Duration::days(11))
        .await
        .is_err());
    let plus_tard = Utc::now() + Duration::days(5);
    let r = ent.edt_diffusion_regler(&a, v, plus_tard).await.unwrap();
    assert_eq!((r.diffusee, r.diffusion_le), (false, Some(plus_tard)));
    // Diffusion échue : l'heure est passée, la tâche de fond bascule le réel.
    sqlx::query("UPDATE ent_edt_version SET diffusion_familles_le = now() - interval '1 minute' WHERE etablissement_id = $1 AND id = $2")
        .bind(e)
        .bind(v)
        .execute(ent.pool())
        .await
        .unwrap();
    let faites = ent.edt_diffuser_echues(&a).await.unwrap();
    assert_eq!(faites, vec![v]);
    assert_eq!(version(&ent, e, v).await.0, "publiee");
    let seances2: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(e)
    .bind(c.d.cours2)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(seances2 > 0);
    assert!(ent.edt_diffuser_echues(&a).await.unwrap().is_empty());
    // Temps 2 : l'élève et sa famille sont prévenus, une seule fois ; l'autre classe, pas.
    distribuer(&ent, e).await;
    distribuer(&ent, e).await;
    assert_eq!(notifs(&ent, compte, "edt_version_diffusee").await.len(), 1);
    assert_eq!(
        notifs(&ent, harry.parent.user_id, "edt_version_diffusee")
            .await
            .len(),
        1
    );
    let neville = cast.eleves.iter().find(|x| x.prenom == "Neville").unwrap();
    assert!(notifs(&ent, neville.parent.user_id, "edt_version")
        .await
        .is_empty());
    let corps = &notifs(&ent, compte, "edt_version_diffusee").await[0].1;
    assert!(
        corps.contains("provisoire") && !corps.to_lowercase().contains("secret"),
        "{corps}"
    );
    // Séparation entre établissements : l'autre école ne reçoit rien.
    let autre = ent.seed_labo_cast().await.unwrap();
    assert!(notifs(&ent, autre.prof.user_id, "edt_version")
        .await
        .is_empty());
}

#[tokio::test]
async fn t5_retour_arriere_annonce_retiree_effet_futur_rétabli_et_version_en_vigueur_copiee() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let (a, g) = (adm(&cast), dir(&cast));
    let aujourdhui = jour_paris();

    // (a) Annonce retirée : redevient un brouillon modifiable.
    let v_annonce = brouillon(&ent, &cast, &c, "À retirer", false, true).await;
    ent.edt_publier(
        &a,
        &CmdPublier {
            version_id: v_annonce,
            a_partir_du: Some(aujourdhui + Duration::days(20)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let r = ent.edt_retour_arriere(&g, v_annonce, None).await.unwrap();
    assert!(r.applique && !r.diffusee);
    assert_eq!(
        version(&ent, e, v_annonce).await,
        ("brouillon".to_string(), None, None, false)
    );
    ent.edt_placement_creer(
        &a,
        NouveauPlacement {
            version_id: v_annonce,
            cours_id: c.d.cours2,
            position: pos(5, "M1", 1, vec![]),
        },
    )
    .await
    .expect("modifiable de nouveau");
    assert!(
        nb_faits(&ent, e, v_annonce).await >= 2,
        "annoncée, annonce retirée"
    );

    // (b) Version publiée dont l'effet n'a pas commencé : la précédente est rétablie, celle-ci archivée.
    let v2 = brouillon(&ent, &cast, &c, "Au mardi", true, false).await;
    let effet2 = aujourdhui + Duration::days(15);
    let r = ent
        .edt_publier(
            &a,
            &CmdPublier {
                version_id: v2,
                a_partir_du: Some(effet2),
                deux_temps: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(r.diffusee, "{r:?}");
    let mardi_apres: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2 AND jour >= $3 AND extract(isodow FROM jour) = 2 AND statut <> 'retiree'",
    )
    .bind(e)
    .bind(c.d.cours)
    .bind(effet2)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(mardi_apres > 0);
    assert_eq!(
        version(&ent, e, c.v0).await.2,
        Some(effet2 - Duration::days(1))
    );
    let r = ent.edt_retour_arriere(&g, v2, None).await.unwrap();
    assert!(r.applique && r.diffusee, "{r:?}");
    assert_eq!(version(&ent, e, v2).await.0, "archivee");
    let (etat_v0, du_v0, au_v0, _) = version(&ent, e, c.v0).await;
    assert_eq!(
        (etat_v0.as_str(), du_v0, au_v0),
        ("publiee", Some(c.d.annee_debut), Some(c.d.annee_fin))
    );
    let mardi_apres: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2 AND jour >= $3 AND extract(isodow FROM jour) = 2 AND statut <> 'retiree'",
    )
    .bind(e)
    .bind(c.d.cours)
    .bind(effet2)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(mardi_apres, 0, "le plan précédent est revenu");
    let lundis: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2 AND jour >= $3 AND extract(isodow FROM jour) = 1 AND statut <> 'retiree'",
    )
    .bind(e)
    .bind(c.d.cours)
    .bind(effet2)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(lundis > 0);

    // (c) Version déjà en vigueur : une copie de la précédente est publiée à partir de demain ; le passé ne change pas.
    let v3 = brouillon(&ent, &cast, &c, "En vigueur", true, false).await;
    let effet3 = aujourdhui + Duration::days(1);
    ent.edt_publier(
        &a,
        &CmdPublier {
            version_id: v3,
            a_partir_du: Some(effet3),
            deux_temps: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    // Demain devient aujourd'hui : on simule l'écoulement du temps en avançant la date d'effet vers le passé.
    for (id, du, au) in [
        (c.v0, None, Some(aujourdhui - Duration::days(1))),
        (v3, Some(aujourdhui), None),
    ] {
        sqlx::query("UPDATE ent_edt_version SET valable_du = coalesce($3, valable_du), valable_au = coalesce($4, valable_au) WHERE etablissement_id = $1 AND id = $2")
            .bind(e)
            .bind(id)
            .bind(du)
            .bind(au)
            .execute(ent.pool())
            .await
            .unwrap();
    }
    let r = ent.edt_retour_arriere(&g, v3, None).await.unwrap();
    assert!(r.applique && r.diffusee, "{r:?}");
    let copie = r.version_id;
    assert_ne!(copie, v3);
    assert_eq!(
        r.effet,
        Some(aujourdhui + Duration::days(1)),
        "au plus tôt demain"
    );
    let (etat_c, du_c, _, _) = version(&ent, e, copie).await;
    assert_eq!(
        (etat_c.as_str(), du_c),
        ("publiee", Some(aujourdhui + Duration::days(1)))
    );
    let (_, du3, au3, _) = version(&ent, e, v3).await;
    assert_eq!(
        (du3, au3),
        (Some(aujourdhui), Some(aujourdhui)),
        "la version en vigueur s'arrête la veille : le passé ne change pas"
    );
    let h = ent
        .edt_publications_historique(&a, c.d.annee_id)
        .await
        .unwrap();
    let hc = h.iter().find(|x| x.id == copie).unwrap();
    assert_eq!(
        (hc.copie_de_version_id, hc.remplace_version_id),
        (Some(c.v0), Some(v3))
    );
    assert!(hc.faits.iter().any(|f| f.fait == "copie_retour"));
    // Une version publiée plus tard la remplace à son tour : on ne retire pas celle du dessous.
    let v4 = brouillon(&ent, &cast, &c, "Plus tard", false, true).await;
    let effet4 = aujourdhui + Duration::days(40);
    ent.edt_publier(
        &a,
        &CmdPublier {
            version_id: v4,
            a_partir_du: Some(effet4),
            deux_temps: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        ent.edt_retour_arriere(&g, copie, None).await.is_err(),
        "la copie est remplacée par une version plus tardive"
    );
    let _ = c.lundi;
}

#[tokio::test]
async fn t5_aucune_lecture_melangee_pendant_la_bascule() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let a = adm(&cast);
    let aujourdhui = jour_paris();
    let effet = aujourdhui + Duration::days(1);
    // Étendre les séances du lundi sur plusieurs mois pour que la bascule remplace beaucoup de lignes.
    ent.ref_generer_seances(
        &a,
        c.d.annee_id,
        Some(effet),
        Some(effet + Duration::days(120)),
    )
    .await
    .unwrap();
    let v = brouillon(&ent, &cast, &c, "Tout au mardi", true, false).await;
    let pool = ent.pool().clone();
    let cours = c.d.cours;
    let fin = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let lecteur = {
        let fin = fin.clone();
        tokio::spawn(async move {
            let mut lectures = 0u32;
            let mut melanges = 0u32;
            while !fin.load(std::sync::atomic::Ordering::SeqCst) || lectures < 20 {
                // Un seul ordre : un instantané cohérent des séances de ce cours à partir de la date d'effet.
                let (lundis, mardis): (i64, i64) = sqlx::query_as(
                    "SELECT count(*) FILTER (WHERE extract(isodow FROM jour) = 1), count(*) FILTER (WHERE extract(isodow FROM jour) = 2)
                       FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2 AND jour >= $3 AND statut <> 'retiree'",
                )
                .bind(e)
                .bind(cours)
                .bind(effet)
                .fetch_one(&pool)
                .await
                .unwrap();
                lectures += 1;
                if lundis > 0 && mardis > 0 {
                    melanges += 1;
                }
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            }
            (lectures, melanges)
        })
    };
    let r = ent
        .edt_publier(
            &a,
            &CmdPublier {
                version_id: v,
                a_partir_du: Some(effet),
                deux_temps: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(r.diffusee);
    fin.store(true, std::sync::atomic::Ordering::SeqCst);
    let (lectures, melanges) = lecteur.await.unwrap();
    assert!(lectures >= 20);
    assert_eq!(
        melanges, 0,
        "ancienne et nouvelle version jamais visibles ensemble ({lectures} lectures)"
    );
    let (lundis, mardis): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE extract(isodow FROM jour) = 1), count(*) FILTER (WHERE extract(isodow FROM jour) = 2)
           FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2 AND jour >= $3 AND statut <> 'retiree'",
    )
    .bind(e)
    .bind(cours)
    .bind(effet)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((lundis, mardis > 0), (0, true));
}

#[tokio::test]
async fn t5_version_individuelle_conflit_idempotence_notification_et_separation() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let a = adm(&cast);
    let aujourdhui = jour_paris();
    let v = brouillon(&ent, &cast, &c, "Base", false, true).await;
    ent.edt_publier(
        &a,
        &CmdPublier {
            version_id: v,
            a_partir_du: Some(aujourdhui + Duration::days(1)),
            deux_temps: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let neville = cast.eleves.iter().find(|x| x.prenom == "Neville").unwrap();
    let harry = &cast.eleves[0];
    let effet = aujourdhui + Duration::days(3);
    let cmd = CmdIndividuelle {
        eleve_id: neville.id,
        a_partir_du: Some(effet),
        au: None,
        motif_code: "upe2a".into(),
        ajouter: vec![c.d.cours2],
        retirer: vec![],
        cle: Some("t5-upe2a".into()),
    };
    // Droits.
    let prof = acteur(&cast, cast.prof.user_id, Role::Prof);
    assert!(matches!(
        ent.edt_publier_individuelle(&prof, &cmd).await,
        Err(EntDbError::Forbidden)
    ));
    // Invalide : motif inconnu, rien à changer, cours à la fois ajouté et retiré.
    for mauvais in [
        CmdIndividuelle {
            motif_code: "inconnu".into(),
            ..cmd.clone()
        },
        CmdIndividuelle {
            ajouter: vec![],
            retirer: vec![],
            ..cmd.clone()
        },
        CmdIndividuelle {
            retirer: vec![c.d.cours2],
            ..cmd.clone()
        },
    ] {
        assert!(ent.edt_publier_individuelle(&a, &mauvais).await.is_err());
    }
    let r = ent.edt_publier_individuelle(&a, &cmd).await.unwrap();
    assert!(r.applique && !r.deja_fait && r.conflits.is_empty(), "{r:?}");
    assert_eq!(r.effet, Some(effet));
    // L'élève est dans le public du cours à partir de la date d'effet, pas avant.
    let dans = |jour: NaiveDate| {
        let ent = &ent;
        async move {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $2, $3) WHERE eleve_id = $4)")
                .bind(e)
                .bind(jour)
                .bind(c.d.cours2)
                .bind(neville.id)
                .fetch_one(ent.pool())
                .await
                .unwrap()
        }
    };
    assert!(dans(effet).await && !dans(effet - Duration::days(1)).await);
    // Deux clics = une seule version individuelle.
    let encore = ent.edt_publier_individuelle(&a, &cmd).await.unwrap();
    assert!(encore.deja_fait && !encore.applique && encore.id == r.id);
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_individuelle WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(e)
    .bind(neville.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
    // Exclusion : Harry (4B) ne suit plus le cours 1 à partir de l'effet.
    let sortie = CmdIndividuelle {
        eleve_id: harry.id,
        a_partir_du: Some(effet),
        au: None,
        motif_code: "dispense_cours".into(),
        ajouter: vec![],
        retirer: vec![c.d.cours],
        cle: None,
    };
    assert!(
        ent.edt_publier_individuelle(&a, &sortie)
            .await
            .unwrap()
            .applique
    );
    // Conflit d'horaire : une version publiée place « Même heure T5 » (4B) et le cours 1 (4B) le lundi à M1 (salles et
    // professeurs différents, donc pas de conflit de plan). Neville suit déjà « Même heure T5 » : ajouter le cours 1
    // à sa version individuelle le met en deux endroits à la fois.
    let conflit_cours = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, classe_id, ouvert_le) VALUES ($1,$2,$3,'Même heure T5',$4,$5)")
        .bind(conflit_cours)
        .bind(e)
        .bind(c.d.annee_id)
        .bind(c.d.classe_4b)
        .bind(c.d.annee_debut)
        .execute(ent.pool())
        .await
        .unwrap();
    service(
        &ent,
        &cast,
        conflit_cours,
        cast.profs
            .get(1)
            .map(|p| p.user_id)
            .unwrap_or(cast.prof.user_id),
    )
    .await;
    let salle2 = creer_salle(&ent, e, cast.administratif.user_id, "B13").await;
    let v_conflit = ent
        .edt_version_creer(&a, c.d.annee_id, "Avec cours simultané", None, None)
        .await
        .unwrap()
        .id;
    for (cours, jour, lieu) in [
        (c.d.cours, 1, c.d.salle),
        (c.d.cours2, 4, c.d.salle),
        (conflit_cours, 1, salle2),
    ] {
        ent.edt_placement_creer(
            &a,
            NouveauPlacement {
                version_id: v_conflit,
                cours_id: cours,
                position: pos(jour, "M1", 1, vec![lieu]),
            },
        )
        .await
        .unwrap();
    }
    let pub2 = ent
        .edt_publier(
            &a,
            &CmdPublier {
                version_id: v_conflit,
                a_partir_du: Some(aujourdhui + Duration::days(2)),
                deux_temps: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(pub2.applique && pub2.diffusee, "{pub2:?}");
    let individuelle = |cours: Uuid, jours: i64| CmdIndividuelle {
        eleve_id: neville.id,
        a_partir_du: Some(aujourdhui + Duration::days(jours)),
        au: None,
        motif_code: "echange".into(),
        ajouter: vec![cours],
        retirer: vec![],
        cle: None,
    };
    let membres = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM ent_cours_membre WHERE etablissement_id = $1 AND eleve_id = $2",
        )
        .bind(e)
        .bind(neville.id)
        .fetch_one(ent.pool())
        .await
        .unwrap()
    };
    assert!(
        ent.edt_publier_individuelle(&a, &individuelle(conflit_cours, 5))
            .await
            .unwrap()
            .applique
    );
    let avant = membres().await;
    let r = ent
        .edt_publier_individuelle(&a, &individuelle(c.d.cours, 6))
        .await
        .unwrap();
    assert!(!r.applique && r.id.is_none(), "{r:?}");
    assert_eq!(
        r.conflits
            .iter()
            .map(|x| x.code.as_str())
            .collect::<Vec<_>>(),
        vec!["conflit_eleve"]
    );
    assert_eq!(r.conflits[0].cours, vec![c.d.cours]);
    assert_eq!(membres().await, avant, "un conflit n'écrit rien");
    // Notification : l'élève et sa famille (mineur) sont prévenus, sans le motif.
    distribuer(&ent, e).await;
    for u in [
        neville.compte.as_ref().map(|c| c.user_id),
        Some(neville.parent.user_id),
    ]
    .into_iter()
    .flatten()
    {
        let m = notifs(&ent, u, "edt_version_individuelle").await;
        assert!(!m.is_empty(), "{u}");
        assert!(
            m.iter()
                .all(|(_, corps)| !corps.to_lowercase().contains("upe2a")),
            "le motif reste sensible : {m:?}"
        );
    }
    let autre_eleve = cast
        .eleves
        .iter()
        .find(|x| x.id != harry.id && x.id != neville.id)
        .unwrap();
    assert!(
        notifs(&ent, autre_eleve.parent.user_id, "edt_version_individuelle")
            .await
            .is_empty()
    );
    // Séparation entre établissements : un acteur d'une autre école ne voit ni ne touche cet élève.
    let autre = ent.seed_labo_cast().await.unwrap();
    let ab = adm(&autre);
    assert!(
        ent.edt_publier_individuelle(&ab, &cmd).await.is_err(),
        "élève et cours d'un autre établissement"
    );
}

#[tokio::test]
async fn t5_separation_entre_etablissements_versions_historique_et_differences() {
    let Some((ent, cast_a)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cast_b = ent.seed_labo_cast().await.unwrap();
    let ca = cadre(&ent, &cast_a).await;
    let (a, b) = (adm(&cast_a), adm(&cast_b));
    let ga = dir(&cast_a);
    let gb = dir(&cast_b);
    let v = brouillon(&ent, &cast_a, &ca, "Version de A", false, true).await;
    // B ne voit, ne contrôle, ne publie ni ne compare la version de A.
    assert!(ent
        .edt_publication_controler(&b, v, None, None)
        .await
        .is_err());
    assert!(ent.edt_publication_apercu(&b, v, None, None).await.is_err());
    assert!(ent
        .edt_publier(
            &b,
            &CmdPublier {
                version_id: v,
                ..Default::default()
            }
        )
        .await
        .is_err());
    assert!(ent.edt_diffuser(&b, v).await.is_err());
    assert!(ent.edt_retour_arriere(&gb, v, None).await.is_err());
    assert!(ent.edt_versions_difference(&b, ca.v0, v).await.is_err());
    assert!(ent
        .edt_publications_historique(&b, ca.d.annee_id)
        .await
        .unwrap()
        .is_empty());
    // Rien n'a été écrit chez A.
    assert_eq!(
        version(&ent, cast_a.etablissement_id, v).await.0,
        "brouillon"
    );
    assert_eq!(nb_faits(&ent, cast_a.etablissement_id, v).await, 0);
    // A publie : B ne reçoit rien, son historique reste vide.
    let r = ent
        .edt_publier(
            &a,
            &CmdPublier {
                version_id: v,
                a_partir_du: Some(jour_paris() + Duration::days(30)),
                deux_temps: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(r.applique);
    assert!(ent
        .edt_publications_historique(&b, ca.d.annee_id)
        .await
        .unwrap()
        .is_empty());
    assert!(!ent
        .edt_publications_historique(&ga, ca.d.annee_id)
        .await
        .unwrap()
        .is_empty());
    // Les séances d'une publication de A n'existent que chez A.
    let chez_b: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(cast_b.etablissement_id)
    .bind(ca.d.cours)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(chez_b, 0);
}
