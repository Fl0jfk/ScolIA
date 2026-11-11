//! Socle 10 (emploi du temps), lots T1 (versions et placements) et T2 (séances et ports).
//! Base locale de test (feature `labo`). Voir `docs/socles/10_EMPLOI_DU_TEMPS.md` §21, §22, §23.
//!
//! T1 : reprise rejouable des créneaux (un créneau sans cours est listé, jamais deviné ; hors grille
//! signalé ; salle inconnue comptée), brouillons (conflits en avertissement, version publiée
//! intouchable, version temporaire rattachée à une base), droits.
//! T2 : séance rattachée à son placement et à sa version (identité stable), instants dans le fuseau de
//! l'établissement, « recalculer ne change rien », ports (séances, séance, en cours, occupation, plages
//! libres, salles libérées D-058, cours supprimés ou modifiés) et portée D-022.

use super::*;
use crate::edt_ports::{CibleSeances, RaisonSalleLibre, RessourcePlan};
use crate::edt_versions::{NatureConflitPlan, NouveauPlacement, PositionPlacement};
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

fn dir(cast: &LaboCast) -> Acteur<'static> {
    acteur(cast, cast.direction.user_id, Role::Direction)
}

fn adm(cast: &LaboCast) -> Acteur<'static> {
    acteur(cast, cast.administratif.user_id, Role::Administratif)
}

fn h(a: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(a, m, 0).unwrap()
}

/// Salle de test (une salle est toujours rattachée à un site de son établissement, migration 337).
async fn creer_salle(ent: &PgEnt, etab: Uuid, user: Uuid, code: &str) -> Uuid {
    sqlx::query(
        "INSERT INTO ent_site (id, etablissement_id, code, libelle) VALUES ($1,$2,'edt-t12','Site EDT T1T2')
         ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .execute(ent.pool())
    .await
    .unwrap();
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_salle (id, etablissement_id, code, libelle, created_by_user_id, site)
         VALUES ($1,$2,$3,$3,$4,'edt-t12')",
    )
    .bind(id)
    .bind(etab)
    .bind(code)
    .bind(user)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

/// Cours de la classe 4B (ouvert depuis le début de l'année) et salle de test.
struct Decor {
    annee_id: Uuid,
    annee_debut: NaiveDate,
    annee_fin: NaiveDate,
    classe_4b: Uuid,
    cours: Uuid,
    cours2: Uuid,
    salle: Uuid,
}

async fn decor(ent: &PgEnt, cast: &LaboCast) -> Decor {
    let e = cast.etablissement_id;
    let annee = ent.referentiel_annee_courante_assuree(e).await.unwrap();
    ent.ref_amorcer_grille(&adm(cast)).await.unwrap();
    let classe_4b: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_classe WHERE etablissement_id = $1 AND annee_id = $2 AND code = '4B'",
    )
    .bind(e)
    .bind(annee.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    let mut ids = Vec::new();
    for libelle in ["Maths T1T2", "Français T1T2"] {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, classe_id, ouvert_le)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(e)
        .bind(annee.id)
        .bind(libelle)
        .bind(classe_4b)
        .bind(annee.date_debut)
        .execute(ent.pool())
        .await
        .unwrap();
        ids.push(id);
    }
    let salle = creer_salle(ent, e, cast.administratif.user_id, "B12").await;
    Decor {
        annee_id: annee.id,
        annee_debut: annee.date_debut,
        annee_fin: annee.date_fin,
        classe_4b,
        cours: ids[0],
        cours2: ids[1],
        salle,
    }
}

async fn creneau(
    ent: &PgEnt,
    cast: &LaboCast,
    cours: Option<Uuid>,
    jour: i16,
    d: NaiveTime,
    f: NaiveTime,
    salle: Option<&str>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine,
                heure_debut, heure_fin, salle, cours_id)
         VALUES ($1,$2,'4B','Maths',$3,$4,$5,$6,$7,$8)",
    )
    .bind(id)
    .bind(cast.etablissement_id)
    .bind(cast.prof.user_id)
    .bind(jour)
    .bind(d)
    .bind(f)
    .bind(salle)
    .bind(cours)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

// ───────────────────────── T1 : reprise des créneaux ─────────────────────────

#[tokio::test]
async fn t1_reprise_des_creneaux_rejouable_sans_deviner() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let d = decor(&ent, &cast).await;
    let a = dir(&cast);
    let cr_a = creneau(&ent, &cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    let cr_b = creneau(&ent, &cast, Some(d.cours), 2, h(8, 0), h(10, 0), None).await; // double
    let cr_c = creneau(
        &ent,
        &cast,
        Some(d.cours),
        3,
        h(8, 20),
        h(9, 10),
        Some("XX99"),
    )
    .await; // hors grille, salle inconnue
    let cr_d = creneau(&ent, &cast, None, 4, h(8, 0), h(9, 0), None).await; // sans cours

    let r = ent.edt_reprendre_creneaux(&a, d.annee_id).await.unwrap();
    assert!(r.version_creee);
    assert!(r.placements_crees >= 3, "{r:?}");
    assert!(
        r.sans_cours.iter().any(|c| c.creneau_id == cr_d),
        "le créneau sans cours doit être listé"
    );
    assert!(!r.sans_cours.iter().any(|c| c.creneau_id == cr_a), "{r:?}");
    assert!(r.hors_grille >= 1 && r.sans_lieu >= 1, "{r:?}");

    let versions = ent.edt_versions(&a, d.annee_id).await.unwrap();
    let v = versions.iter().find(|v| v.id == r.version_id).unwrap();
    assert_eq!(
        (v.etat.as_str(), v.nature.as_str(), v.origine.as_str()),
        ("publiee", "base", "reprise")
    );
    assert_eq!(
        (v.valable_du, v.valable_au),
        (Some(d.annee_debut), Some(d.annee_fin))
    );

    let pl = ent.edt_placements(&a, r.version_id).await.unwrap();
    let place = |c: Uuid| pl.iter().find(|p| p.creneau_id == Some(c)).cloned();
    let pa = place(cr_a).expect("placement du créneau A");
    assert_eq!(
        (
            pa.jour_semaine,
            pa.creneau_debut_code.as_deref(),
            pa.duree_creneaux
        ),
        (1, Some("M1"), Some(1))
    );
    assert_eq!(pa.lieux, vec![d.salle]);
    assert_eq!(pa.cours_id, d.cours);
    let pb = place(cr_b).unwrap();
    assert_eq!(
        (pb.creneau_debut_code.as_deref(), pb.duree_creneaux),
        (Some("M1"), Some(2))
    );
    let pc = place(cr_c).unwrap();
    assert!(
        pc.creneau_debut_code.is_none() && pc.heure_debut_libre == Some(h(8, 20)),
        "{pc:?}"
    );
    assert!(pc.lieux.is_empty());
    assert!(place(cr_d).is_none(), "jamais de placement deviné");
    // Besoin du cours = somme des créneaux (1 h + 2 h + 50 min).
    let heures: f64 = sqlx::query_scalar(
        "SELECT heures_hebdo::float8 FROM ent_edt_besoin WHERE version_id = $1 AND cours_id = $2",
    )
    .bind(r.version_id)
    .bind(d.cours)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!((heures - 3.83).abs() < 0.011, "{heures}");

    // Rejouable : rien ne change.
    let r2 = ent.edt_reprendre_creneaux(&a, d.annee_id).await.unwrap();
    assert!(!r2.version_creee);
    assert_eq!(
        (
            r2.placements_crees,
            r2.placements_mis_a_jour,
            r2.placements_retires
        ),
        (0, 0, 0),
        "{r2:?}"
    );
    assert!(r2.placements_inchanges >= 3);
    assert_eq!(r2.version_id, r.version_id);

    // Un créneau modifié met à jour son placement (même identifiant), un créneau supprimé le retire.
    sqlx::query(
        "UPDATE ent_edt_creneau SET heure_debut = '09:00', heure_fin = '10:00' WHERE id = $1",
    )
    .bind(cr_a)
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query("DELETE FROM ent_edt_creneau WHERE id = $1")
        .bind(cr_b)
        .execute(ent.pool())
        .await
        .unwrap();
    let r3 = ent.edt_reprendre_creneaux(&a, d.annee_id).await.unwrap();
    assert_eq!(
        (r3.placements_mis_a_jour, r3.placements_retires),
        (1, 1),
        "{r3:?}"
    );
    let pl = ent.edt_placements(&a, r.version_id).await.unwrap();
    let pa2 = pl.iter().find(|p| p.creneau_id == Some(cr_a)).unwrap();
    assert_eq!(pa2.id, pa.id, "identité stable du placement");
    assert_eq!(pa2.creneau_debut_code.as_deref(), Some("M2"));
    assert!(!pl.iter().any(|p| p.id == pb.id));

    // Droits : le professeur et la famille ne reprennent rien.
    let prof = acteur(&cast, cast.prof.user_id, Role::Prof);
    assert!(matches!(
        ent.edt_reprendre_creneaux(&prof, d.annee_id).await,
        Err(EntDbError::Forbidden)
    ));
    let parent = acteur(&cast, cast.parent_lea.user_id, Role::Parent);
    assert!(matches!(
        ent.edt_versions(&parent, d.annee_id).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.edt_placements(&parent, r.version_id).await,
        Err(EntDbError::Forbidden)
    ));
    // Isolation : un autre établissement ne voit pas la version.
    let autre = ent.seed_labo_cast().await.unwrap();
    let autre_a = dir(&autre);
    let annee_autre = ent
        .referentiel_annee_courante_assuree(autre.etablissement_id)
        .await
        .unwrap();
    assert!(ent
        .edt_versions(&autre_a, annee_autre.id)
        .await
        .unwrap()
        .is_empty());
    assert!(ent
        .edt_placements(&autre_a, r.version_id)
        .await
        .unwrap()
        .is_empty());
}

// ───────────────────────── T1 : brouillons, conflits, version temporaire ─────────────────────────

#[tokio::test]
async fn t1_brouillon_conflits_publiee_intouchable_et_temporaire() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let d = decor(&ent, &cast).await;
    let a = dir(&cast);
    let prof = acteur(&cast, cast.prof.user_id, Role::Prof);
    creneau(&ent, &cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    let reprise = ent.edt_reprendre_creneaux(&a, d.annee_id).await.unwrap();

    // Droits.
    assert!(matches!(
        ent.edt_version_creer(&prof, d.annee_id, "Brouillon", None, None)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(
        ent.edt_version_creer(&a, d.annee_id, "   ", None, None)
            .await
            .is_err(),
        "libellé vide"
    );

    let v = ent
        .edt_version_creer(&a, d.annee_id, "Brouillon 2027", None, None)
        .await
        .unwrap();
    assert_eq!(
        (v.etat.as_str(), v.nature.as_str(), v.origine.as_str()),
        ("brouillon", "base", "saisie")
    );
    assert!(v.valable_du.is_none() && v.publiee_le.is_none());

    let pos = |jour, code: &str, duree, lieux: Vec<Uuid>| PositionPlacement {
        jour_semaine: jour,
        creneau_debut_code: code.into(),
        duree_creneaux: duree,
        semaine_type: "toutes".into(),
        lieux,
    };
    let p1 = ent
        .edt_placement_creer(
            &a,
            NouveauPlacement {
                version_id: v.id,
                cours_id: d.cours,
                position: pos(2, "M1", 2, vec![d.salle]),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (p1.origine.as_str(), p1.duree_creneaux, p1.lieux.clone()),
        ("main", Some(2), vec![d.salle])
    );

    // Entrées invalides : code inconnu, durée qui sort de la journée, lieu d'un autre établissement.
    let nouveau = |p: PositionPlacement| NouveauPlacement {
        version_id: v.id,
        cours_id: d.cours2,
        position: p,
    };
    assert!(ent
        .edt_placement_creer(&a, nouveau(pos(2, "Z9", 1, vec![])))
        .await
        .is_err());
    assert!(ent
        .edt_placement_creer(&a, nouveau(pos(2, "S4", 3, vec![])))
        .await
        .is_err());
    assert!(ent
        .edt_placement_creer(&a, nouveau(pos(8, "M1", 1, vec![])))
        .await
        .is_err());
    let autre = ent.seed_labo_cast().await.unwrap();
    let salle_etrangere = creer_salle(
        &ent,
        autre.etablissement_id,
        autre.administratif.user_id,
        "Z1",
    )
    .await;
    assert!(ent
        .edt_placement_creer(&a, nouveau(pos(2, "M1", 1, vec![salle_etrangere])))
        .await
        .is_err());

    // Un autre cours au même créneau et dans la même salle : accepté (brouillon), conflit signalé.
    let p2 = ent
        .edt_placement_creer(&a, nouveau(pos(2, "M2", 1, vec![d.salle])))
        .await
        .unwrap();
    let conflits = ent.edt_version_conflits(&a, v.id).await.unwrap();
    assert!(
        conflits.iter().any(|c| c.nature == NatureConflitPlan::Lieu
            && [c.placement_a, c.placement_b].contains(&p1.id)
            && [c.placement_a, c.placement_b].contains(&p2.id)),
        "{conflits:?}"
    );
    // On le déplace en M4 : le conflit disparaît ; placement verrouillé : plus déplaçable.
    let p2 = ent
        .edt_placement_deplacer(&a, p2.id, pos(2, "M4", 1, vec![d.salle]))
        .await
        .unwrap();
    assert_eq!(p2.creneau_debut_code.as_deref(), Some("M4"));
    assert!(ent.edt_version_conflits(&a, v.id).await.unwrap().is_empty());
    sqlx::query("UPDATE ent_edt_placement SET verrouille = true WHERE id = $1")
        .bind(p2.id)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(ent
        .edt_placement_deplacer(&a, p2.id, pos(3, "M1", 1, vec![]))
        .await
        .is_err());
    assert!(ent.edt_placement_retirer(&a, p2.id).await.is_err());
    // Retrait d'un placement libre : la pièce et le besoin orphelins disparaissent avec lui.
    ent.edt_placement_retirer(&a, p1.id).await.unwrap();
    assert_eq!(ent.edt_placements(&a, v.id).await.unwrap().len(), 1);
    let besoins: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_edt_besoin WHERE version_id = $1")
            .bind(v.id)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(
        besoins, 1,
        "le besoin du cours sans placement a disparu, celui du cours 2 reste"
    );

    // Une version publiée (la reprise) ne se modifie pas : brouillon à part.
    let sur_reprise = NouveauPlacement {
        version_id: reprise.version_id,
        cours_id: d.cours,
        position: pos(5, "M1", 1, vec![]),
    };
    assert!(matches!(
        ent.edt_placement_creer(&a, sur_reprise).await,
        Err(EntDbError::Domain(_))
    ));
    let pl = ent.edt_placements(&a, reprise.version_id).await.unwrap();
    assert!(ent.edt_placement_retirer(&a, pl[0].id).await.is_err());
    assert!(ent
        .edt_version_abandonner(&a, reprise.version_id)
        .await
        .is_err());

    // Version temporaire (§8.4) : rattachée à une version de BASE de la même année, jamais à une temporaire.
    let t = ent
        .edt_version_creer(
            &a,
            d.annee_id,
            "Suppléant contraint",
            Some(reprise.version_id),
            Some("novembre–janvier"),
        )
        .await
        .unwrap();
    assert_eq!(
        (t.nature.as_str(), t.base_version_id),
        ("temporaire", Some(reprise.version_id))
    );
    assert!(ent
        .edt_version_creer(&a, d.annee_id, "Sur une temporaire", Some(t.id), None)
        .await
        .is_err());
    assert!(ent
        .edt_version_creer(&a, d.annee_id, "Base inconnue", Some(Uuid::new_v4()), None)
        .await
        .is_err());
    // Une seule version de base publiée par date : une seconde base publiée sur la même plage est refusée par la base.
    let double = sqlx::query(
        "INSERT INTO ent_edt_version (id, etablissement_id, annee_id, libelle, nature, etat, valable_du, valable_au, publiee_le)
         VALUES ($1,$2,$3,'Concurrente','base','publiee',$4,$5,now())",
    )
    .bind(Uuid::new_v4())
    .bind(cast.etablissement_id)
    .bind(d.annee_id)
    .bind(d.annee_debut + Duration::days(10))
    .bind(d.annee_fin)
    .execute(ent.pool())
    .await;
    assert!(
        double.is_err(),
        "deux versions de base publiées se chevauchent"
    );
    // Abandon d'un brouillon : une fois, puis refus.
    ent.edt_version_abandonner(&a, t.id).await.unwrap();
    assert!(ent.edt_version_abandonner(&a, t.id).await.is_err());
}

// ───────────────────────── T2 : séances rattachées, ports, salles libérées ─────────────────────────

#[tokio::test]
async fn t2_seances_rattachees_stables_et_ports() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let d = decor(&ent, &cast).await;
    let a = adm(&cast);
    let g = dir(&cast);

    // Un lundi M1 en salle B12, sur quatre semaines (dans l'année courante).
    let mut lundi = today_utc().clamp(d.annee_debut, d.annee_fin);
    lundi += Duration::days((8 - lundi.weekday().number_from_monday() as i64) % 7);
    let (du, au) = (lundi, lundi + Duration::days(27));
    let cr = creneau(&ent, &cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    let r = ent
        .ref_generer_seances(&a, d.annee_id, Some(du), Some(au))
        .await
        .unwrap();
    assert!(r.creees >= 4 && r.rattachees >= 4, "{r:?}");

    #[derive(sqlx::FromRow, Debug, PartialEq, Clone)]
    struct Ligne {
        id: Uuid,
        jour: NaiveDate,
        placement_id: Option<Uuid>,
        version_id: Option<Uuid>,
        cours_id: Option<Uuid>,
        creneau_code: Option<String>,
        debut_utc: Option<chrono::DateTime<chrono::Utc>>,
        fin_utc: Option<chrono::DateTime<chrono::Utc>>,
        etat: String,
        origine: String,
        statut: String,
        salle: Option<String>,
    }
    let instantane = || {
        let pool = ent.pool().clone();
        async move {
            sqlx::query_as::<_, Ligne>(
                "SELECT id, jour, placement_id, version_id, cours_id, creneau_code, debut_utc, fin_utc, etat, origine,
                        statut, salle FROM ent_edt_seance WHERE etablissement_id = $1 AND creneau_id = $2 ORDER BY jour",
            )
            .bind(e)
            .bind(cr)
            .fetch_all(&pool)
            .await
            .unwrap()
        }
    };
    let avant = instantane().await;
    assert_eq!(avant.len(), 4);
    for s in &avant {
        assert!(s.placement_id.is_some() && s.version_id.is_some(), "{s:?}");
        assert_eq!(
            (s.cours_id, s.creneau_code.as_deref()),
            (Some(d.cours), Some("M1"))
        );
        assert_eq!((s.etat.as_str(), s.origine.as_str()), ("prevue", "plan"));
    }
    // Instants : l'heure murale 08:00 d'Europe/Paris, quelle que soit la saison.
    let (h_local, h_fin): (NaiveTime, NaiveTime) = sqlx::query_as(
        "SELECT (debut_utc AT TIME ZONE 'Europe/Paris')::time, (fin_utc AT TIME ZONE 'Europe/Paris')::time
           FROM ent_edt_seance WHERE id = $1",
    )
    .bind(avant[0].id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((h_local, h_fin), (h(8, 0), h(9, 0)));

    // « Recalculer tout ne change rien » : mêmes identifiants, mêmes valeurs, aucune création.
    let r2 = ent
        .ref_generer_seances(&a, d.annee_id, Some(du), Some(au))
        .await
        .unwrap();
    assert_eq!(
        (
            r2.creees,
            r2.mises_a_jour,
            r2.supprimees,
            r2.retirees,
            r2.rattachees
        ),
        (0, 0, 0, 0, 0),
        "{r2:?}"
    );
    assert_eq!(instantane().await, avant);
    // Republier la reprise (plan identique) ne change aucun identifiant de séance.
    ent.edt_reprendre_creneaux(&g, d.annee_id).await.unwrap();
    ent.ref_generer_seances(&a, d.annee_id, Some(du), Some(au))
        .await
        .unwrap();
    assert_eq!(instantane().await, avant);

    // Lieux : la salle B12 est un lieu de chaque séance (source de l'occupation, D-058).
    let nb_lieux: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance_lieu l JOIN ent_edt_seance s ON s.id = l.seance_id
          WHERE s.creneau_id = $1 AND l.lieu_id = $2 AND l.principal",
    )
    .bind(cr)
    .bind(d.salle)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(nb_lieux, 4);

    // ── Ports ──
    let jour1 = avant[0].jour;
    let seances = ent
        .edt_seances(&g, CibleSeances::Classe(d.classe_4b), du, au)
        .await
        .unwrap();
    assert!(seances.iter().filter(|s| s.creneau_id == cr).count() == 4);
    assert!(seances.iter().all(|s| s.etat != "retiree"));
    let un = seances.iter().find(|s| s.id == avant[0].id).unwrap();
    assert_eq!(
        (un.cours_id, un.lieux.clone()),
        (Some(d.cours), vec![d.salle])
    );
    // Élève de la classe : sa séance sort ; le professeur d'une autre classe n'a pas le droit (D-022).
    let sem_eleve = ent
        .edt_seances(&g, CibleSeances::Eleve(cast.eleve_lea_id), du, au)
        .await
        .unwrap();
    assert!(
        sem_eleve.iter().any(|s| s.id == avant[0].id),
        "Harry (4B) doit avoir le cours"
    );
    let rogue = acteur(&cast, cast.prof.user_id, Role::Prof);
    assert!(matches!(
        ent.edt_seances(&rogue, CibleSeances::Eleve(cast.eleve_tom_id), du, au)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .edt_seances(&rogue, CibleSeances::Eleve(cast.eleve_lea_id), du, au)
        .await
        .is_ok());
    // Un professeur lit les siennes, pas celles d'un collègue ; la famille n'a pas ce port.
    let autre_prof = cast
        .profs
        .iter()
        .find(|p| p.user_id != cast.prof.user_id)
        .unwrap();
    assert!(ent
        .edt_seances(&rogue, CibleSeances::Intervenant(cast.prof.user_id), du, au)
        .await
        .is_ok());
    assert!(matches!(
        ent.edt_seances(
            &rogue,
            CibleSeances::Intervenant(autre_prof.user_id),
            du,
            au
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    let parent = acteur(&cast, cast.parent_lea.user_id, Role::Parent);
    assert!(matches!(
        ent.edt_seances(&parent, CibleSeances::Classe(d.classe_4b), du, au)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .edt_seances(
            &g,
            CibleSeances::Classe(d.classe_4b),
            du,
            du + Duration::days(63)
        )
        .await
        .is_err());
    // seance(id) : le titulaire et la direction ; un collègue non.
    assert!(ent.edt_seance(&rogue, avant[0].id).await.is_ok());
    assert!(ent.edt_seance(&g, avant[0].id).await.is_ok());
    let autre = acteur(&cast, autre_prof.user_id, Role::Prof);
    assert!(matches!(
        ent.edt_seance(&autre, avant[0].id).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.edt_seance(&g, Uuid::new_v4()).await,
        Err(EntDbError::Forbidden)
    ));
    // « Maintenant » : pendant la séance, pour l'élève ; hors séance, rien.
    let pendant = avant[0].debut_utc.unwrap() + Duration::minutes(10);
    let en_cours = ent
        .edt_seance_en_cours(&g, cast.eleve_lea_id, pendant)
        .await
        .unwrap();
    assert!(en_cours.is_some_and(|s| s.id == avant[0].id));
    assert!(ent
        .edt_seance_en_cours(
            &g,
            cast.eleve_lea_id,
            avant[0].fin_utc.unwrap() + Duration::minutes(1)
        )
        .await
        .unwrap()
        .is_none_or(|s| s.id != avant[0].id));
    // Occupation du lieu (anonymisée), occupation à l'instant, et plages libres.
    let occ = ent
        .edt_occupation_lieu(&adm(&cast), d.salle, du, au)
        .await
        .unwrap();
    assert_eq!(occ.len(), 4);
    assert!(ent
        .edt_occupe(&g, d.salle, jour1, h(8, 30), h(9, 30))
        .await
        .unwrap());
    assert!(
        !ent.edt_occupe(&g, d.salle, jour1, h(9, 0), h(10, 0))
            .await
            .unwrap(),
        "plages contiguës"
    );
    let libres = ent
        .edt_plages_libres(&g, RessourcePlan::Lieu(d.salle), jour1, jour1, 55)
        .await
        .unwrap();
    assert!(
        libres
            .iter()
            .all(|p| !(p.jour == jour1 && p.debut < h(9, 0) && p.fin > h(8, 0))),
        "la plage de 8 h à 9 h est occupée : {libres:?}"
    );
    assert!(
        libres.iter().any(|p| p.jour == jour1 && p.debut == h(9, 0)),
        "{libres:?}"
    );
    assert!(ent
        .edt_plages_libres(&g, RessourcePlan::Lieu(d.salle), jour1, jour1, 1)
        .await
        .is_err());
    assert!(matches!(
        ent.edt_plages_libres(
            &rogue,
            RessourcePlan::Intervenant(autre_prof.user_id),
            jour1,
            jour1,
            55
        )
        .await,
        Err(EntDbError::Forbidden)
    ));

    // ── D-058 : le cours supprimé libère la salle ; le remplaçant nommé la reprend ──
    let s2 = avant[1].clone();
    sqlx::query("UPDATE ent_edt_seance SET statut = 'annulee' WHERE id = $1")
        .bind(s2.id)
        .execute(ent.pool())
        .await
        .unwrap();
    // Motif interne d'une exception : n'apparaît dans aucun port.
    sqlx::query(
        "INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, motif, created_by_user_id)
         VALUES ($1,$2,$3,$4,'annulation','Maladie confidentielle',$5)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(cr)
    .bind(s2.jour)
    .bind(cast.direction.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    let liberees = ent.edt_salles_liberees(&adm(&cast), du, au).await.unwrap();
    let l = liberees
        .iter()
        .find(|l| l.seance_id == s2.id)
        .expect("salle libérée par le cours annulé");
    assert_eq!(
        (l.lieu_id, l.raison, l.classe.as_str()),
        (d.salle, RaisonSalleLibre::CoursAnnule, "4B")
    );
    let json = serde_json::to_string(&(
        &liberees,
        ent.edt_cours_supprimes_ou_modifies(&g, du, au)
            .await
            .unwrap(),
    ))
    .unwrap();
    assert!(
        !json.contains("Maladie"),
        "le motif interne ne sort d'aucun port : {json}"
    );
    let changes = ent
        .edt_cours_supprimes_ou_modifies(&adm(&cast), du, au)
        .await
        .unwrap();
    assert!(changes
        .iter()
        .any(|c| c.seance_id == s2.id && c.etat == "annulee"));
    // L'annulée n'occupe plus la salle ; la créneau est libre à l'instant.
    assert!(!ent
        .edt_occupe(&g, d.salle, s2.jour, h(8, 0), h(9, 0))
        .await
        .unwrap());
    assert_eq!(
        ent.edt_occupation_lieu(&g, d.salle, du, au)
            .await
            .unwrap()
            .len(),
        3
    );
    // Un remplaçant est nommé : le cours est rétabli (`modifiee`), la salle n'est plus libérée.
    sqlx::query(
        "UPDATE ent_edt_seance SET statut = 'remplacee', remplacant_user_id = $2 WHERE id = $1",
    )
    .bind(s2.id)
    .bind(autre_prof.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(!ent
        .edt_salles_liberees(&g, du, au)
        .await
        .unwrap()
        .iter()
        .any(|l| l.seance_id == s2.id));
    assert!(ent
        .edt_occupe(&g, d.salle, s2.jour, h(8, 0), h(9, 0))
        .await
        .unwrap());
    // Un jour de travaux sur la salle : indisponible, donc occupée et non libre.
    sqlx::query(
        "INSERT INTO ent_salle_indisponibilite (id, etablissement_id, salle_id, du, au, motif_code, par)
         VALUES ($1,$2,$3,$4::date + time '12:00', $4::date + time '18:00', 'travaux', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(d.salle)
    .bind(avant[2].jour)
    .bind(cast.administratif.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    assert!(ent
        .edt_occupe(&g, d.salle, avant[2].jour, h(13, 0), h(14, 0))
        .await
        .unwrap());
    let libres = ent
        .edt_plages_libres(
            &g,
            RessourcePlan::Lieu(d.salle),
            avant[2].jour,
            avant[2].jour,
            55,
        )
        .await
        .unwrap();
    assert!(
        libres
            .iter()
            .all(|p| p.fin <= h(12, 0) || p.debut >= h(18, 0)),
        "{libres:?}"
    );

    // Recalcul de la fenêtre glissante : idempotent.
    ent.edt_recalculer_fenetre(&a, d.annee_id, Some(au))
        .await
        .unwrap();
    let apres = instantane().await;
    assert_eq!(
        apres.iter().map(|s| s.id).collect::<Vec<_>>(),
        avant.iter().map(|s| s.id).collect::<Vec<_>>()
    );
}
