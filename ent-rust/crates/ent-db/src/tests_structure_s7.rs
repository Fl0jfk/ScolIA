//! Socle 5, lot S7 : périodisations (résolution par classe, un lycée en semestres et un collège en
//! trimestres dans le même établissement), grille horaire versionnée (gel du passé, coupure des
//! demi-journées lue par le port), droits, cloisonnement (isolation entre établissements, RLS
//! stricte), journal. Base locale de test (feature `labo`) ; sans `ENT_DATABASE_URL`, ignorés en
//! local et **en échec en CI**. Chaque scénario travaille dans un **établissement neuf**.

use super::*;
use crate::ref_commun::Acteur;
use crate::seed::LaboPersona;
use crate::structure_grilles::{
    structure as port_grille, CibleGrilleSaisie, CreneauS7Saisie, DemiJournee, GrilleModif,
    GrilleSaisie,
};
use crate::structure_periodes::{
    structure as port_periode, CibleSaisie, PeriodeModif, PeriodisationSaisie,
};
use crate::tenant_tx::{RlsMode, TenantTx};
use crate::tests_d022_bis::labo;
use chrono::{Duration, NaiveDate, NaiveTime};
use domain_eleve::DomainError;
use uuid::Uuid;

fn acteur_de(etab: Uuid, p: &LaboPersona) -> Acteur<'static> {
    Acteur {
        etablissement_id: etab,
        user_id: p.user_id,
        role: p.role.as_str(),
    }
}

fn sfx() -> String {
    Uuid::new_v4().simple().to_string()[..6].to_uppercase()
}

fn aujourdhui() -> NaiveDate {
    domain_eleve::horloge::jour_paris()
}

fn h(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn est_forbidden<T>(r: &Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn est_domaine<T>(r: &Result<T, EntDbError>, d: DomainError) -> bool {
    matches!(r, Err(EntDbError::Domain(e)) if *e == d)
}

/// Établissement neuf avec une année qui a commencé il y a 60 jours.
async fn etab_neuf(ent: &PgEnt) -> (Uuid, Uuid) {
    let (etab, annee) = (Uuid::new_v4(), Uuid::new_v4());
    let t = aujourdhui();
    sqlx::query(
        "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
         VALUES ($1,$2,'N',$3,$4,true)",
    )
    .bind(annee)
    .bind(etab)
    .bind(t - Duration::days(60))
    .bind(t + Duration::days(200))
    .execute(ent.pool())
    .await
    .unwrap();
    (etab, annee)
}

async fn niveau(ent: &PgEnt, etab: Uuid, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_niveau (id, etablissement_id, code, libelle) VALUES ($1,$2,$3,$3)",
    )
    .bind(id)
    .bind(etab)
    .bind(code)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn mef(ent: &PgEnt, etab: Uuid, niveau: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let code = format!("{:0>11}", sfx());
    sqlx::query(
        "INSERT INTO ent_mef (id, etablissement_id, code_mef, niveau_id, libelle_long) VALUES ($1,$2,$3,$4,'MEF')",
    )
    .bind(id)
    .bind(etab)
    .bind(code)
    .bind(niveau)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

async fn classe(
    ent: &PgEnt,
    etab: Uuid,
    annee: Uuid,
    code: &str,
    niveau: Option<Uuid>,
    mef: Option<Uuid>,
    site: Option<Uuid>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle, niveau_id, mef_id, site_id)
         VALUES ($1,$2,$3,$4,$4,$5,$6,$7)",
    )
    .bind(id)
    .bind(etab)
    .bind(annee)
    .bind(code)
    .bind(niveau)
    .bind(mef)
    .bind(site)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

fn modele(code: &str, m: &str, cibles: Vec<CibleSaisie>) -> PeriodisationSaisie {
    PeriodisationSaisie {
        code: code.into(),
        libelle: format!("Périodisation {code}"),
        usage: "evaluation".into(),
        modele: Some(m.into()),
        periodes: None,
        cibles,
    }
}

fn cible_niveau(n: Uuid) -> CibleSaisie {
    CibleSaisie {
        niveau_id: Some(n),
        mef_id: None,
        classe_id: None,
        defaut: false,
    }
}

fn cible_defaut() -> CibleSaisie {
    CibleSaisie {
        niveau_id: None,
        mef_id: None,
        classe_id: None,
        defaut: true,
    }
}

async fn codes_de_classe(ent: &PgEnt, etab: Uuid, classe: Uuid) -> Vec<String> {
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let ps = port_periode::periodes_de_classe(&mut t, etab, classe)
        .await
        .unwrap();
    t.commit().await.unwrap();
    ps.into_iter().map(|p| p.code).collect()
}

// ───────────────────────── Périodisations ─────────────────────────

#[tokio::test]
async fn lycee_en_semestres_et_college_en_trimestres_dans_le_meme_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let (sixieme, seconde, troisieme) = (
        niveau(&ent, etab, "6EME").await,
        niveau(&ent, etab, "2NDE").await,
        niveau(&ent, etab, "3EME").await,
    );
    let (c6, c2, c3) = (
        classe(&ent, etab, annee, "6A", Some(sixieme), None, None).await,
        classe(&ent, etab, annee, "2A", Some(seconde), None, None).await,
        classe(&ent, etab, annee, "3A", Some(troisieme), None, None).await,
    );

    // Sans aucune périodisation : rien n'est résolu, le contrôle liste les trois classes.
    assert!(codes_de_classe(&ent, etab, c6).await.is_empty());
    let ctl = ent
        .structure_periodisation_controle(&dir, annee)
        .await
        .unwrap();
    assert!(!ctl.ok);
    assert_eq!(ctl.classes_sans_periodisation.len(), 3);

    // Collège en trimestres (niveau 6e), lycée en semestres (niveau 2nde).
    let tri = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele("COLLEGE", "trimestres", vec![cible_niveau(sixieme)]),
        )
        .await
        .unwrap();
    assert_eq!(tri.periodes.len(), 3);
    let sem = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele("LYCEE", "semestres", vec![cible_niveau(seconde)]),
        )
        .await
        .unwrap();
    assert_eq!(sem.periodes.len(), 2);
    assert_eq!(codes_de_classe(&ent, etab, c6).await, ["T1", "T2", "T3"]);
    assert_eq!(codes_de_classe(&ent, etab, c2).await, ["S1", "S2"]);
    // La 3e n'a pas de cible : elle n'hérite ni du collège ni du lycée.
    assert!(codes_de_classe(&ent, etab, c3).await.is_empty());
    let ctl = ent
        .structure_periodisation_controle(&dir, annee)
        .await
        .unwrap();
    assert_eq!(ctl.classes_sans_periodisation.len(), 1);
    assert_eq!(ctl.classes_sans_periodisation[0].code, "3A");

    // Une périodisation par défaut recueille la 3e, sans toucher aux deux autres.
    ent.structure_periodisation_creer(
        &dir,
        annee,
        modele("DEFAUT", "trimestres", vec![cible_defaut()]),
    )
    .await
    .unwrap();
    assert_eq!(codes_de_classe(&ent, etab, c3).await, ["T1", "T2", "T3"]);
    assert_eq!(codes_de_classe(&ent, etab, c2).await, ["S1", "S2"]);
    assert!(
        ent.structure_periodisation_controle(&dir, annee)
            .await
            .unwrap()
            .ok
    );

    // Le port rend la période à une date, et la périodisation d'une classe est unique.
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let p = port_periode::periode_a_la_date(&mut t, etab, c2, aujourdhui())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p.code, "S1");
    assert!(
        port_periode::periode_a_la_date(&mut t, etab, c2, aujourdhui() + Duration::days(400))
            .await
            .unwrap()
            .is_none()
    );
    let pz = port_periode::periodisation_de_classe(&mut t, etab, c6)
        .await
        .unwrap();
    assert_eq!(pz, Some(tri.id));
    t.commit().await.unwrap();
}

#[tokio::test]
async fn la_cible_la_plus_precise_gagne_classe_puis_mef_puis_niveau_puis_defaut() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let n = niveau(&ent, etab, "2NDE").await;
    let m = mef(&ent, etab, n).await;
    // Classe 2A : niveau 2nde + MEF ; classe 2B : niveau 2nde sans MEF ; classe 2C : MEF seul (niveau via le MEF).
    let (c_a, c_b, c_c) = (
        classe(&ent, etab, annee, "2A", Some(n), Some(m), None).await,
        classe(&ent, etab, annee, "2B", Some(n), None, None).await,
        classe(&ent, etab, annee, "2C", None, Some(m), None).await,
    );
    let defaut = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele("DEF", "trimestres", vec![cible_defaut()]),
        )
        .await
        .unwrap();
    let niv = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele("NIV", "semestres", vec![cible_niveau(n)]),
        )
        .await
        .unwrap();
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    for c in [c_a, c_b, c_c] {
        assert_eq!(
            port_periode::periodisation_de_classe(&mut t, etab, c)
                .await
                .unwrap(),
            Some(niv.id)
        );
    }
    t.commit().await.unwrap();

    // Le MEF l'emporte sur le niveau (2A et 2C), pas sur 2B qui n'a pas ce MEF.
    let par_mef = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            PeriodisationSaisie {
                code: "MEF".into(),
                libelle: "MEF".into(),
                usage: "evaluation".into(),
                modele: Some("trimestres".into()),
                periodes: None,
                cibles: vec![CibleSaisie {
                    niveau_id: None,
                    mef_id: Some(m),
                    classe_id: None,
                    defaut: false,
                }],
            },
        )
        .await
        .unwrap();
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert_eq!(
        port_periode::periodisation_de_classe(&mut t, etab, c_a)
            .await
            .unwrap(),
        Some(par_mef.id)
    );
    assert_eq!(
        port_periode::periodisation_de_classe(&mut t, etab, c_c)
            .await
            .unwrap(),
        Some(par_mef.id)
    );
    assert_eq!(
        port_periode::periodisation_de_classe(&mut t, etab, c_b)
            .await
            .unwrap(),
        Some(niv.id)
    );
    t.commit().await.unwrap();

    // La classe l'emporte sur tout.
    let par_classe = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            PeriodisationSaisie {
                code: "CLA".into(),
                libelle: "Classe".into(),
                usage: "evaluation".into(),
                modele: Some("semestres".into()),
                periodes: None,
                cibles: vec![CibleSaisie {
                    niveau_id: None,
                    mef_id: None,
                    classe_id: Some(c_a),
                    defaut: false,
                }],
            },
        )
        .await
        .unwrap();
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert_eq!(
        port_periode::periodisation_de_classe(&mut t, etab, c_a)
            .await
            .unwrap(),
        Some(par_classe.id)
    );
    t.commit().await.unwrap();

    // Une cible n'a qu'UNE périodisation d'évaluation par année (409) ; « autre » n'est pas résolue.
    let doublon = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele("NIV2", "trimestres", vec![cible_niveau(n)]),
        )
        .await;
    assert!(est_domaine(&doublon, DomainError::Conflit), "{doublon:?}");
    let autre = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            PeriodisationSaisie {
                code: "PROJET".into(),
                libelle: "Projets".into(),
                usage: "autre".into(),
                modele: None,
                periodes: Some(vec![crate::structure_periodes::PeriodeS7Saisie {
                    code: "P1".into(),
                    libelle: "Projet 1".into(),
                    kind: "periode".into(),
                    date_debut: aujourdhui(),
                    date_fin: aujourdhui() + Duration::days(30),
                    date_arret_notes: None,
                    date_conseil: None,
                }]),
                cibles: vec![cible_niveau(n)],
            },
        )
        .await
        .unwrap();
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert_ne!(
        port_periode::periodisation_de_classe(&mut t, etab, c_b)
            .await
            .unwrap(),
        Some(autre.id)
    );
    t.commit().await.unwrap();

    // Archivée, la périodisation de classe laisse la classe retomber sur la cible suivante (MEF).
    ent.structure_periodisation_archiver(&dir, par_classe.id)
        .await
        .unwrap();
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert_eq!(
        port_periode::periodisation_de_classe(&mut t, etab, c_a)
            .await
            .unwrap(),
        Some(par_mef.id)
    );
    t.commit().await.unwrap();
    // Le défaut existe toujours pour une classe hors niveau.
    assert!(defaut.cibles[0].defaut);
}

#[tokio::test]
async fn validation_des_periodes_et_des_cibles() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let t0 = aujourdhui();
    let (debut, fin) = (t0 - Duration::days(60), t0 + Duration::days(200));
    let p = |code: &str, d: NaiveDate, f: NaiveDate| crate::structure_periodes::PeriodeS7Saisie {
        code: code.into(),
        libelle: code.into(),
        kind: "trimestre".into(),
        date_debut: d,
        date_fin: f,
        date_arret_notes: None,
        date_conseil: None,
    };
    let saisie = |periodes| PeriodisationSaisie {
        code: format!("V{}", sfx()),
        libelle: "V".into(),
        usage: "evaluation".into(),
        modele: None,
        periodes: Some(periodes),
        cibles: vec![],
    };
    // Trou, chevauchement, début qui n'est pas celui de l'année, débordement : 400.
    let milieu = debut + Duration::days(130);
    for mauvaises in [
        vec![
            p("A", debut, milieu),
            p("B", milieu + Duration::days(2), fin),
        ], // trou
        vec![p("A", debut, milieu), p("B", milieu, fin)], // chevauchement
        vec![p("A", debut + Duration::days(1), fin)],     // début tardif
        vec![p("A", debut, fin + Duration::days(1))],     // déborde de l'année
    ] {
        let r = ent
            .structure_periodisation_creer(&dir, annee, saisie(mauvaises))
            .await;
        assert!(est_domaine(&r, DomainError::ValeurInvalide), "{r:?}");
    }
    // Un modèle ET des périodes, ou ni l'un ni l'autre : refusé.
    let r = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            PeriodisationSaisie {
                modele: Some("trimestres".into()),
                ..saisie(vec![p("A", debut, fin)])
            },
        )
        .await;
    assert!(est_domaine(&r, DomainError::ValeurInvalide), "{r:?}");
    // Cible d'un autre établissement : introuvable.
    let (autre_etab, _) = etab_neuf(&ent).await;
    let n_autre = niveau(&ent, autre_etab, "6EME").await;
    let r = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele("X", "trimestres", vec![cible_niveau(n_autre)]),
        )
        .await;
    assert!(est_domaine(&r, DomainError::NotFound), "{r:?}");
    // Deux champs de cible à la fois : refusé.
    let r = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele(
                "Y",
                "trimestres",
                vec![CibleSaisie {
                    niveau_id: Some(Uuid::new_v4()),
                    defaut: true,
                    mef_id: None,
                    classe_id: None,
                }],
            ),
        )
        .await;
    assert!(est_domaine(&r, DomainError::ValeurInvalide), "{r:?}");
    // Code en double dans l'année : 409.
    ent.structure_periodisation_creer(&dir, annee, modele("DUP", "trimestres", vec![]))
        .await
        .unwrap();
    let r = ent
        .structure_periodisation_creer(&dir, annee, modele("dup", "semestres", vec![]))
        .await;
    assert!(est_domaine(&r, DomainError::Conflit), "{r:?}");
}

#[tokio::test]
async fn modifier_une_periode_en_cours_est_reserve_a_la_direction_et_decale_la_voisine() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let (dir, adm) = (
        acteur_de(etab, &cast.direction),
        acteur_de(etab, &cast.administratif),
    );
    let n = niveau(&ent, etab, "4EME").await;
    let cl = classe(&ent, etab, annee, "4A", Some(n), None, None).await;
    // L'administratif peut créer un découpage (STR-31, « Dir, Adm »).
    let pz = ent
        .structure_periodisation_creer(
            &adm,
            annee,
            modele("TRI", "trimestres", vec![cible_niveau(n)]),
        )
        .await
        .unwrap();
    let t1 = pz.periodes[0].clone();
    let t2 = pz.periodes[1].clone();
    let t3 = pz.periodes[2].clone();
    assert!(
        t1.date_debut <= aujourdhui(),
        "T1 doit être commencée dans ce scénario"
    );

    // T1 est commencée : l'administratif ne peut pas en changer les dates (★) ni l'arrêt des notes.
    let nouvelle_fin = t1.date_fin + Duration::days(5);
    let r = ent
        .structure_periode_modifier(
            &adm,
            t1.id,
            PeriodeModif {
                date_fin: Some(nouvelle_fin),
                ajuster_voisines: true,
                ..Default::default()
            },
        )
        .await;
    assert!(est_forbidden(&r), "{r:?}");
    // T3 n'a pas commencé : l'administratif peut y poser une date de conseil.
    let conseil = t3.date_debut + Duration::days(40);
    let v = ent
        .structure_periode_modifier(
            &adm,
            t3.id,
            PeriodeModif {
                date_conseil: Some(Some(conseil)),
                ajuster_voisines: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(v.periodes[2].date_conseil, Some(conseil));

    // La direction décale la fin de T1 : T2 commence le lendemain, la couverture reste exacte.
    let v = ent
        .structure_periode_modifier(
            &dir,
            t1.id,
            PeriodeModif {
                date_fin: Some(nouvelle_fin),
                ajuster_voisines: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(v.periodes[0].date_fin, nouvelle_fin);
    assert_eq!(v.periodes[1].date_debut, nouvelle_fin + Duration::days(1));
    assert_eq!(v.periodes[1].date_fin, t2.date_fin);
    assert!(
        ent.structure_periodisation_controle(&dir, annee)
            .await
            .unwrap()
            .ok
    );

    // Sans ajustement des voisines, le trou est refusé (la couverture est une règle).
    let r = ent
        .structure_periode_modifier(
            &dir,
            t1.id,
            PeriodeModif {
                date_fin: Some(nouvelle_fin + Duration::days(3)),
                ajuster_voisines: false,
                ..Default::default()
            },
        )
        .await;
    assert!(
        est_domaine(&r, DomainError::ValeurInvalide) || est_domaine(&r, DomainError::Conflit),
        "{r:?}"
    );

    // Arrêt des notes : le port le lit pour la classe (socle 21), la date d'arrêt est incluse.
    let arret = aujourdhui() + Duration::days(2);
    ent.structure_periode_modifier(
        &dir,
        t1.id,
        PeriodeModif {
            date_arret_notes: Some(Some(arret)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert!(
        port_periode::saisie_notes_ouverte(&mut t, etab, cl, t1.id, arret)
            .await
            .unwrap()
    );
    assert!(!port_periode::saisie_notes_ouverte(
        &mut t,
        etab,
        cl,
        t1.id,
        arret + Duration::days(1)
    )
    .await
    .unwrap());
    // Une période d'une autre périodisation n'est jamais « de la classe ».
    let autre = ent
        .structure_periodisation_creer(&dir, annee, modele("AUTRE", "semestres", vec![]))
        .await
        .unwrap();
    assert!(
        !port_periode::saisie_notes_ouverte(&mut t, etab, cl, autre.periodes[0].id, arret)
            .await
            .unwrap()
    );
    t.commit().await.unwrap();

    // Journal en ajout seul : les modifications sont tracées avec avant / après.
    let n_journal: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_structure_journal WHERE etablissement_id = $1 AND objet = 'periode'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(n_journal >= 3, "{n_journal}");
    let r =
        sqlx::query("UPDATE ent_structure_journal SET action = 'x' WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool())
            .await;
    assert!(r.is_err(), "le journal est en ajout seul");
}

#[tokio::test]
async fn periode_utilisee_par_les_notes_ne_se_retire_pas_et_la_periodisation_ne_s_archive_pas() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let pz = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele("DEF", "trimestres", vec![cible_defaut()]),
        )
        .await
        .unwrap();
    let t2 = pz.periodes[1].clone();
    // La saisie des notes s'appuie sur T2 (le rapatriement la retrouve par son code).
    sqlx::query("INSERT INTO ent_note_periode (id, etablissement_id, code, libelle, date_debut, date_fin) VALUES ($1,$2,'T2','Trimestre 2',$3,$4)")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(t2.date_debut)
        .bind(t2.date_fin)
        .execute(ent.pool())
        .await
        .unwrap();
    let liee: Option<Uuid> = sqlx::query_scalar(
        "SELECT periode_id FROM ent_note_periode WHERE etablissement_id = $1 AND code = 'T2'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        liee,
        Some(t2.id),
        "la période de notes est rattachée à la période de la périodisation"
    );
    let sans_t2: Vec<_> = pz
        .periodes
        .iter()
        .filter(|p| p.code != "T2")
        .map(|p| crate::structure_periodes::PeriodeS7Saisie {
            code: p.code.clone(),
            libelle: p.libelle.clone(),
            kind: p.kind.clone(),
            date_debut: p.date_debut,
            date_fin: p.date_fin,
            date_arret_notes: None,
            date_conseil: None,
        })
        .collect();
    let r = ent
        .structure_periodisation_remplacer_periodes(&dir, pz.id, sans_t2)
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(_))), "{r:?}");
    let r = ent.structure_periodisation_archiver(&dir, pz.id).await;
    assert!(est_domaine(&r, DomainError::Conflit), "{r:?}");
}

#[tokio::test]
async fn anciens_ecrivains_de_periodes_restent_rattaches_a_la_source_unique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let n = niveau(&ent, etab, "5EME").await;
    let t0 = aujourdhui();
    // Insertion « à l'ancienne » (sans périodisation) pour un niveau puis pour l'établissement.
    for (niv, code) in [(Some(n), "T1"), (None, "S1")] {
        sqlx::query(
            "INSERT INTO ent_periode (id, etablissement_id, annee_id, niveau_id, code, libelle, kind, ordre, date_debut, date_fin)
             VALUES ($1,$2,$3,$4,$5,$5,'periode',1,$6,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(annee)
        .bind(niv)
        .bind(code)
        .bind(t0 - Duration::days(60))
        .bind(t0 + Duration::days(200))
        .execute(ent.pool())
        .await
        .unwrap();
    }
    let vues = ent
        .structure_periodisations(&dir, annee, false)
        .await
        .unwrap();
    assert_eq!(
        vues.len(),
        2,
        "une périodisation « reprise » par cible : {vues:?}"
    );
    assert!(vues
        .iter()
        .all(|v| v.origine == "reprise" && v.cibles.len() == 1));
    let niv_pz = vues
        .iter()
        .find(|v| v.cibles[0].niveau_id == Some(n))
        .unwrap();
    assert_eq!(niv_pz.periodes[0].code, "T1");
    // Une deuxième insertion pour le même niveau réutilise la même périodisation.
    sqlx::query(
        "INSERT INTO ent_periode (id, etablissement_id, annee_id, niveau_id, code, libelle, kind, ordre, date_debut, date_fin)
         VALUES ($1,$2,$3,$4,'T9','T9','periode',2,$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(annee)
    .bind(n)
    .bind(t0 + Duration::days(300))
    .bind(t0 + Duration::days(301))
    .execute(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        ent.structure_periodisations(&dir, annee, false)
            .await
            .unwrap()
            .len(),
        2
    );

    // L'ancienne route (niveau) lit et écrit la MÊME périodisation que le nouveau service.
    let cls = classe(&ent, etab, annee, "5A", Some(n), None, None).await;
    let via_ancienne = ent.ref_periodes(&dir, annee, Some(n)).await.unwrap();
    assert!(via_ancienne.iter().any(|p| p.code == "T1"));
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert_eq!(
        port_periode::periodisation_de_classe(&mut t, etab, cls)
            .await
            .unwrap(),
        Some(niv_pz.id)
    );
    t.commit().await.unwrap();
}

#[tokio::test]
async fn ancienne_route_de_periodes_alimente_la_periodisation_par_defaut() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let t0 = aujourdhui();
    let (debut, fin) = (t0 - Duration::days(60), t0 + Duration::days(200));
    let coupe = debut + Duration::days(120);
    let ps = vec![
        crate::referentiel_structure::PeriodeSaisie {
            code: "S1".into(),
            libelle: "Semestre 1".into(),
            kind: "semestre".into(),
            date_debut: debut,
            date_fin: coupe,
        },
        crate::referentiel_structure::PeriodeSaisie {
            code: "S2".into(),
            libelle: "Semestre 2".into(),
            kind: "semestre".into(),
            date_debut: coupe + Duration::days(1),
            date_fin: fin,
        },
    ];
    ent.ref_definir_periodes(&dir, annee, None, &ps)
        .await
        .unwrap();
    let c = classe(&ent, etab, annee, "1A", None, None, None).await;
    assert_eq!(codes_de_classe(&ent, etab, c).await, ["S1", "S2"]);
    // Rejouée : idempotente, toujours une seule périodisation par défaut.
    ent.ref_definir_periodes(&dir, annee, None, &ps)
        .await
        .unwrap();
    assert_eq!(
        ent.structure_periodisations(&dir, annee, false)
            .await
            .unwrap()
            .len(),
        1
    );
    // Le nouveau service voit et peut compléter cette périodisation (arrêt des notes).
    let v = ent
        .structure_periodisations(&dir, annee, false)
        .await
        .unwrap()
        .remove(0);
    ent.structure_periode_modifier(
        &dir,
        v.periodes[0].id,
        PeriodeModif {
            date_arret_notes: Some(Some(coupe)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    // Si le défaut est partagé par plusieurs cibles, la route historique refuse de le réécrire.
    let n = niveau(&ent, etab, "6EME").await;
    ent.structure_periodisation_assigner(&dir, v.id, vec![cible_defaut(), cible_niveau(n)])
        .await
        .unwrap();
    let r = ent.ref_definir_periodes(&dir, annee, None, &ps).await;
    assert!(
        r.is_err(),
        "périodisation partagée : la route historique ne la réécrit pas"
    );
}

#[tokio::test]
async fn chevauchement_refuse_par_la_base() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let pz = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele("DEF", "trimestres", vec![cible_defaut()]),
        )
        .await
        .unwrap();
    let p1 = &pz.periodes[0];
    let r = sqlx::query(
        "INSERT INTO ent_periode (id, etablissement_id, annee_id, periodisation_id, code, libelle, kind, ordre, date_debut, date_fin, origine)
         VALUES ($1,$2,$3,$4,'CHEV','chev','periode',9,$5,$6,'service')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(annee)
    .bind(pz.id)
    .bind(p1.date_debut + Duration::days(1))
    .bind(p1.date_debut + Duration::days(5))
    .execute(ent.pool())
    .await;
    assert!(
        r.is_err(),
        "le chevauchement doit être refusé par la contrainte d'exclusion"
    );
    // Hors année : refusé aussi (déclencheur), et une période ne change jamais de périodisation.
    let r = sqlx::query(
        "UPDATE ent_periode SET date_fin = date_fin + 2000 WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(pz.periodes[2].id)
    .execute(ent.pool())
    .await;
    assert!(r.is_err());
    let autre = ent
        .structure_periodisation_creer(&dir, annee, modele("AUTRE", "semestres", vec![]))
        .await
        .unwrap();
    let r = sqlx::query(
        "UPDATE ent_periode SET periodisation_id = $3 WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(p1.id)
    .bind(autre.id)
    .execute(ent.pool())
    .await;
    assert!(r.is_err());
}

// ───────────────────────── Grilles ─────────────────────────

fn cr(code: &str, d: NaiveTime, f: NaiveTime, nature: &str) -> CreneauS7Saisie {
    CreneauS7Saisie {
        code: code.into(),
        libelle: code.into(),
        heure_debut: d,
        heure_fin: f,
        nature: nature.into(),
    }
}

fn journee() -> Vec<CreneauS7Saisie> {
    vec![
        cr("M1", h(8, 0), h(8, 55), "cours"),
        cr("M2", h(8, 55), h(9, 50), "cours"),
        cr("R1", h(9, 50), h(10, 5), "recreation"),
        cr("M3", h(10, 5), h(11, 0), "cours"),
        cr("M4", h(11, 0), h(11, 55), "cours"),
        cr("PM", h(11, 55), h(13, 5), "pause_meridienne"),
        cr("S1", h(13, 5), h(14, 0), "cours"),
        cr("S2", h(14, 0), h(14, 55), "cours"),
    ]
}

fn grille(
    code: &str,
    du: NaiveDate,
    creneaux: Vec<CreneauS7Saisie>,
    cibles: Vec<CibleGrilleSaisie>,
) -> GrilleSaisie {
    GrilleSaisie {
        code: code.into(),
        libelle: format!("Grille {code}"),
        valable_du: du,
        coupure_demi_journee: None,
        motif: None,
        creneaux,
        cibles,
    }
}

async fn site(ent: &PgEnt, etab: Uuid, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_site (id, etablissement_id, code, libelle) VALUES ($1,$2,$3,$3)")
        .bind(id)
        .bind(etab)
        .bind(code)
        .execute(ent.pool())
        .await
        .unwrap();
    id
}

async fn grille_du_jour(
    ent: &PgEnt,
    etab: Uuid,
    jour: NaiveDate,
    site: Option<Uuid>,
    niveau: Option<Uuid>,
) -> Option<port_grille::GrilleResolue> {
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let g = port_grille::grille(&mut t, etab, jour, site, niveau)
        .await
        .unwrap();
    t.commit().await.unwrap();
    g
}

#[tokio::test]
async fn grille_versionnee_sans_toucher_au_passe_et_coupure_lue_par_le_port() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, _) = etab_neuf(&ent).await;
    let (dir, adm) = (
        acteur_de(etab, &cast.direction),
        acteur_de(etab, &cast.administratif),
    );
    let t0 = aujourdhui();

    // Version 1 d'aujourd'hui (administratif autorisé, STR-34 « Dir, Adm »), coupure par défaut 12 h 30.
    let v1 = ent
        .structure_grille_versionner(&adm, grille("STD", t0, journee(), vec![]))
        .await
        .unwrap();
    assert_eq!(v1.coupure_demi_journee, h(12, 30));
    assert_eq!(v1.valable_au, None);
    assert_eq!(v1.creneaux.len(), 8);

    // Le passé ne se réécrit pas.
    let r = ent
        .structure_grille_versionner(
            &adm,
            grille("STD", t0 - Duration::days(1), journee(), vec![]),
        )
        .await;
    assert!(est_domaine(&r, DomainError::ValeurInvalide), "{r:?}");
    // Une version qui commence déjà ce jour-là : on la remplace, on n'en crée pas une autre.
    let r = ent
        .structure_grille_versionner(&adm, grille("STD", t0, journee(), vec![]))
        .await;
    assert!(est_domaine(&r, DomainError::Conflit), "{r:?}");

    // Version 2 dans 10 jours : journée plus courte. La version 1 est close la veille.
    let courte = vec![
        cr("M1", h(8, 30), h(9, 25), "cours"),
        cr("M2", h(9, 25), h(10, 20), "cours"),
        cr("PM", h(12, 0), h(13, 0), "pause_meridienne"),
        cr("S1", h(13, 30), h(14, 25), "cours"),
    ];
    let jour2 = t0 + Duration::days(10);
    let v2 = ent
        .structure_grille_versionner(&adm, grille("STD", jour2, courte, vec![]))
        .await
        .unwrap();
    assert_eq!(v2.coupure_demi_journee, h(12, 30));
    let anciennes = ent.structure_grilles(&adm).await.unwrap();
    let v1_relue = anciennes.iter().find(|g| g.id == v1.id).unwrap();
    assert_eq!(v1_relue.valable_au, Some(jour2 - Duration::days(1)));

    // Le port répond par date : v1 jusqu'à la veille, v2 ensuite ; avant aujourd'hui, rien.
    assert_eq!(
        grille_du_jour(&ent, etab, t0, None, None).await.unwrap().id,
        v1.id
    );
    assert_eq!(
        grille_du_jour(&ent, etab, jour2 - Duration::days(1), None, None)
            .await
            .unwrap()
            .id,
        v1.id
    );
    assert_eq!(
        grille_du_jour(&ent, etab, jour2, None, None)
            .await
            .unwrap()
            .id,
        v2.id
    );
    assert!(
        grille_du_jour(&ent, etab, t0 - Duration::days(1), None, None)
            .await
            .is_none()
    );

    // Coupure : réservée à la direction ; jamais au milieu d'un cours ; datée (nouvelle version).
    let r = ent
        .structure_grille_coupure(&adm, "STD", t0, h(12, 15), None)
        .await;
    assert!(est_forbidden(&r), "{r:?}");
    let r = ent
        .structure_grille_coupure(&dir, "STD", t0, h(11, 30), None)
        .await; // dans M4
    assert!(est_domaine(&r, DomainError::ValeurInvalide), "{r:?}");
    let jour3 = t0 + Duration::days(3);
    let v3 = ent
        .structure_grille_coupure(&dir, "STD", jour3, h(12, 15), Some("pause plus tôt"))
        .await
        .unwrap();
    assert_ne!(v3.id, v1.id);
    assert_eq!(v3.coupure_demi_journee, h(12, 15));
    assert_eq!(
        v3.creneaux.len(),
        v1.creneaux.len(),
        "mêmes créneaux que la version qu'elle relaie"
    );
    // Avant le changement : coupure 12 h 30 ; à partir de jour3 : 12 h 15. Le port répond « matin » /
    // « après-midi » selon la version en vigueur ce jour-là (décompte du socle 12).
    let demi = |jour, heure| {
        let ent = &ent;
        async move {
            let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
            let r = port_grille::demi_journee(&mut t, etab, jour, None, None, heure)
                .await
                .unwrap();
            t.commit().await.unwrap();
            r
        }
    };
    assert_eq!(demi(t0, h(12, 20)).await, Some(DemiJournee::Matin));
    assert_eq!(demi(jour3, h(12, 20)).await, Some(DemiJournee::ApresMidi));
    assert_eq!(
        demi(t0 - Duration::days(30), h(9, 0)).await,
        None,
        "pas de grille avant la première version"
    );
    // La version 2 (dans 10 jours) a été découpée par la version 3 ? Non : v3 relaie v1, v2 reste suivante.
    let toutes = ent.structure_grilles(&dir).await.unwrap();
    let v1_apres = toutes.iter().find(|g| g.id == v1.id).unwrap();
    assert_eq!(v1_apres.valable_au, Some(jour3 - Duration::days(1)));
    assert_eq!(
        toutes.iter().find(|g| g.id == v3.id).unwrap().valable_au,
        Some(jour2 - Duration::days(1))
    );
    assert_eq!(
        toutes.iter().find(|g| g.id == v2.id).unwrap().valable_au,
        None
    );

    // Supprimer la version 3 (future) rend sa période à la version 1.
    ent.structure_grille_supprimer(&adm, v3.id).await.unwrap();
    let toutes = ent.structure_grilles(&dir).await.unwrap();
    assert_eq!(
        toutes.iter().find(|g| g.id == v1.id).unwrap().valable_au,
        Some(jour2 - Duration::days(1))
    );

    // Modifier une version future sur place (créneaux et cibles).
    let r = ent
        .structure_grille_remplacer(
            &adm,
            v2.id,
            GrilleModif {
                libelle: "Courte bis".into(),
                coupure_demi_journee: Some(h(12, 45)),
                motif: None,
                creneaux: journee(),
                cibles: vec![],
            },
        )
        .await
        .unwrap();
    assert_eq!(r.coupure_demi_journee, h(12, 45));
    assert_eq!(r.creneaux.len(), 8);

    // Journal : chaque écriture est tracée.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_structure_journal WHERE etablissement_id = $1 AND objet = 'grille'")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(n >= 5, "{n}");
}

#[tokio::test]
async fn version_commencee_gelee_par_la_base() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, _) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let t0 = aujourdhui();
    let v1 = ent
        .structure_grille_versionner(&dir, grille("STD", t0, journee(), vec![]))
        .await
        .unwrap();
    // On « vieillit » la version d'une semaine (simulation du temps qui passe) : elle a commencé.
    sqlx::query("UPDATE ent_grille SET valable_du = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(v1.id)
        .bind(t0 - Duration::days(7))
        .execute(ent.pool())
        .await
        .unwrap();
    // Gelée : libellé, coupure, créneaux, cibles, suppression — tout est refusé par la base…
    for sql in [
        "UPDATE ent_grille SET libelle = 'x' WHERE id = $1",
        "UPDATE ent_grille SET coupure_demi_journee = '12:00' WHERE id = $1",
        "DELETE FROM ent_grille WHERE id = $1",
        "UPDATE ent_grille_creneau SET libelle = 'x' WHERE grille_id = $1",
        "DELETE FROM ent_grille_creneau WHERE grille_id = $1",
        "DELETE FROM ent_grille_cible WHERE grille_id = $1",
    ] {
        let r = sqlx::query(sql).bind(v1.id).execute(ent.pool()).await;
        assert!(r.is_err(), "{sql} devrait être refusé");
    }
    let r = sqlx::query(
        "INSERT INTO ent_grille_creneau (id, etablissement_id, grille_id, code, libelle, heure_debut, heure_fin, nature, ordre)
         VALUES ($1,$2,$3,'Z','Z','16:00','17:00','cours',20)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(v1.id)
    .execute(ent.pool())
    .await;
    assert!(r.is_err());
    // … et par le service (409), qui propose de versionner.
    let r = ent
        .structure_grille_remplacer(
            &dir,
            v1.id,
            GrilleModif {
                libelle: "x".into(),
                coupure_demi_journee: None,
                motif: None,
                creneaux: journee(),
                cibles: vec![],
            },
        )
        .await;
    assert!(est_domaine(&r, DomainError::Conflit), "{r:?}");
    assert!(est_domaine(
        &ent.structure_grille_supprimer(&dir, v1.id).await,
        DomainError::Conflit
    ));
    // Seule sa date de fin bouge : une nouvelle version la clôt la veille.
    let v2 = ent
        .structure_grille_versionner(
            &dir,
            grille("STD", t0, vec![cr("A", h(9, 0), h(10, 0), "cours")], vec![]),
        )
        .await
        .unwrap();
    let vues = ent.structure_grilles(&dir).await.unwrap();
    assert_eq!(
        vues.iter().find(|g| g.id == v1.id).unwrap().valable_au,
        Some(t0 - Duration::days(1))
    );
    assert_eq!(
        vues.iter().find(|g| g.id == v2.id).unwrap().valable_au,
        None
    );
    assert!(vues.iter().find(|g| g.id == v1.id).unwrap().commencee);
    assert_eq!(
        vues.iter().find(|g| g.id == v1.id).unwrap().creneaux.len(),
        8,
        "le passé garde ses 8 créneaux"
    );
}

#[tokio::test]
async fn grille_par_site_niveau_et_jour_la_cible_la_plus_precise_gagne() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let t0 = aujourdhui();
    let (s_college, s_lycee) = (
        site(&ent, etab, "COLLEGE").await,
        site(&ent, etab, "LYCEE").await,
    );
    let sixieme = niveau(&ent, etab, "6EME").await;
    let cl = classe(
        &ent,
        etab,
        annee,
        "6A",
        Some(sixieme),
        None,
        Some(s_college),
    )
    .await;

    let tout = ent
        .structure_grille_versionner(&dir, grille("BASE", t0, journee(), vec![]))
        .await
        .unwrap();
    let college = ent
        .structure_grille_versionner(
            &dir,
            grille(
                "COLLEGE",
                t0,
                vec![
                    cr("A", h(8, 0), h(9, 0), "cours"),
                    cr("B", h(9, 0), h(10, 0), "cours"),
                ],
                vec![CibleGrilleSaisie {
                    site_id: Some(s_college),
                    niveau_id: None,
                    jours: None,
                }],
            ),
        )
        .await
        .unwrap();
    let sixieme_g = ent
        .structure_grille_versionner(
            &dir,
            grille(
                "SIXIEME",
                t0,
                vec![cr("A", h(8, 30), h(9, 30), "cours")],
                vec![CibleGrilleSaisie {
                    site_id: None,
                    niveau_id: Some(sixieme),
                    jours: None,
                }],
            ),
        )
        .await
        .unwrap();
    // Le mercredi, matin seulement, pour tout le monde.
    let mercredi = ent
        .structure_grille_versionner(
            &dir,
            grille(
                "MERCREDI",
                t0,
                vec![cr("A", h(8, 0), h(9, 0), "cours")],
                vec![CibleGrilleSaisie {
                    site_id: None,
                    niveau_id: None,
                    jours: Some(vec![3]),
                }],
            ),
        )
        .await
        .unwrap();

    // Un lundi quelconque dans les 7 prochains jours, et le mercredi suivant.
    let mut lundi = t0;
    while chrono::Datelike::weekday(&lundi) != chrono::Weekday::Mon {
        lundi += Duration::days(1);
    }
    let mercredi_j = lundi + Duration::days(2);
    // Lycée sans niveau particulier : la grille de tout l'établissement.
    assert_eq!(
        grille_du_jour(&ent, etab, lundi, Some(s_lycee), None)
            .await
            .unwrap()
            .id,
        tout.id
    );
    // Collège : la grille du site.
    assert_eq!(
        grille_du_jour(&ent, etab, lundi, Some(s_college), None)
            .await
            .unwrap()
            .id,
        college.id
    );
    // 6e (niveau) sur le site du collège : la cible « niveau » (plus précise que « site » seul ? non :
    // niveau > site dans l'ordre de spécificité, cf. migration 336).
    assert_eq!(
        grille_du_jour(&ent, etab, lundi, Some(s_college), Some(sixieme))
            .await
            .unwrap()
            .id,
        sixieme_g.id
    );
    // Le mercredi : la cible qui précise un jour l'emporte à spécificité égale (tout l'établissement).
    assert_eq!(
        grille_du_jour(&ent, etab, mercredi_j, Some(s_lycee), None)
            .await
            .unwrap()
            .id,
        mercredi.id
    );
    // Le port « classe » déduit site et niveau de la classe.
    let mut t = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let g = port_grille::grille_de_classe(&mut t, etab, cl, lundi)
        .await
        .unwrap()
        .unwrap();
    t.commit().await.unwrap();
    assert_eq!(g.id, sixieme_g.id);

    // Deux grilles de codes différents sur la même cible et les mêmes jours : ambigu, refusé (409).
    let r = ent
        .structure_grille_versionner(
            &dir,
            grille(
                "CONCURRENTE",
                t0,
                journee(),
                vec![CibleGrilleSaisie {
                    site_id: Some(s_college),
                    niveau_id: None,
                    jours: None,
                }],
            ),
        )
        .await;
    assert!(est_domaine(&r, DomainError::Conflit), "{r:?}");
    // Même cible mais jours disjoints : accepté.
    ent.structure_grille_versionner(
        &dir,
        grille(
            "SAMEDI",
            t0,
            journee(),
            vec![CibleGrilleSaisie {
                site_id: Some(s_college),
                niveau_id: None,
                jours: Some(vec![6]),
            }],
        ),
    )
    .await
    .unwrap();
    // Site ou niveau d'un autre établissement : introuvable.
    let (autre, _) = etab_neuf(&ent).await;
    let s_autre = site(&ent, autre, "AUTRE").await;
    let r = ent
        .structure_grille_versionner(
            &dir,
            grille(
                "X",
                t0,
                journee(),
                vec![CibleGrilleSaisie {
                    site_id: Some(s_autre),
                    niveau_id: None,
                    jours: None,
                }],
            ),
        )
        .await;
    assert!(est_domaine(&r, DomainError::NotFound), "{r:?}");
}

#[tokio::test]
async fn ancienne_grille_miroitee_et_nouvelle_grille_projetee_pour_les_seances() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, _) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let t0 = aujourdhui();

    // Nouvelle écriture → l'ancienne table plate (lue par la génération des séances) suit,
    // avec la demi-journée déduite de la coupure.
    ent.structure_grille_versionner(&dir, grille("STD", t0, journee(), vec![]))
        .await
        .unwrap();
    let anciennes = ent.ref_grille(&dir, None).await.unwrap();
    assert_eq!(
        anciennes.len(),
        6,
        "seuls les cours sont projetés (pas la récréation ni la pause)"
    );
    assert_eq!(
        anciennes
            .iter()
            .filter(|c| c.demi_journee == "matin")
            .count(),
        4
    );
    assert_eq!(
        anciennes
            .iter()
            .filter(|c| c.demi_journee == "apres_midi")
            .count(),
        2
    );

    // Ancienne écriture → le modèle versionné la voit (version « REPRISE » d'aujourd'hui).
    let legacy = vec![
        crate::referentiel_structure::CreneauGrilleSaisie {
            code: "M1".into(),
            libelle: "M1".into(),
            demi_journee: "matin".into(),
            heure_debut: h(8, 0),
            heure_fin: h(9, 0),
        },
        crate::referentiel_structure::CreneauGrilleSaisie {
            code: "S1".into(),
            libelle: "S1".into(),
            demi_journee: "apres_midi".into(),
            heure_debut: h(14, 0),
            heure_fin: h(15, 0),
        },
    ];
    let (etab2, _) = etab_neuf(&ent).await;
    let dir2 = acteur_de(etab2, &cast.direction);
    ent.ref_definir_grille(&dir2, None, &legacy).await.unwrap();
    let vues = ent.structure_grilles(&dir2).await.unwrap();
    assert_eq!(vues.len(), 1);
    assert_eq!(vues[0].code, "REPRISE-ETAB");
    assert_eq!(vues[0].creneaux.len(), 2);
    let g = grille_du_jour(&ent, etab2, t0, None, None).await.unwrap();
    assert_eq!(g.demi_journee(h(8, 30)), DemiJournee::Matin);
    assert_eq!(g.demi_journee(h(14, 30)), DemiJournee::ApresMidi);
    assert_eq!(g.creneau_a(h(14, 30)).unwrap().code, "S1");
    // Rejouée le même jour : une seule version, remplacée.
    ent.ref_definir_grille(&dir2, None, &legacy[..1])
        .await
        .unwrap();
    let vues = ent.structure_grilles(&dir2).await.unwrap();
    assert_eq!(vues.len(), 1);
    assert_eq!(vues[0].creneaux.len(), 1);
}

// ───────────────────────── Droits, isolation, RLS ─────────────────────────

#[tokio::test]
async fn matrice_de_droits_periodes_et_grilles() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, annee) = etab_neuf(&ent).await;
    let dir = acteur_de(etab, &cast.direction);
    let t0 = aujourdhui();
    let pz = ent
        .structure_periodisation_creer(
            &dir,
            annee,
            modele("DEF", "trimestres", vec![cible_defaut()]),
        )
        .await
        .unwrap();
    let g = ent
        .structure_grille_versionner(&dir, grille("STD", t0, journee(), vec![]))
        .await
        .unwrap();

    // Lecture : tout le personnel ; écriture : direction, administratif, administrateur. Les
    // familles et les élèves n'ont aucun accès. Comptabilité, professeur et CPE : lecture seule.
    for (nom, p) in [
        ("prof", &cast.prof),
        ("cpe", &cast.cpe),
        ("compta", &cast.compta),
    ] {
        let a = acteur_de(etab, p);
        assert!(
            ent.structure_periodisations(&a, annee, false).await.is_ok(),
            "{nom} lit"
        );
        assert!(
            ent.structure_periodisation(&a, pz.id).await.is_ok(),
            "{nom} lit"
        );
        assert!(ent.structure_grilles(&a).await.is_ok(), "{nom} lit");
        assert!(
            ent.structure_grille_a_la_date(&a, t0, None, None)
                .await
                .is_ok(),
            "{nom} lit"
        );
        let w = ent
            .structure_periodisation_creer(
                &a,
                annee,
                modele(&format!("W{nom}"), "trimestres", vec![]),
            )
            .await;
        assert!(est_forbidden(&w), "{nom} n'écrit pas : {w:?}");
        assert!(
            est_forbidden(
                &ent.structure_periodisation_assigner(&a, pz.id, vec![])
                    .await
            ),
            "{nom}"
        );
        assert!(
            est_forbidden(&ent.structure_periodisation_archiver(&a, pz.id).await),
            "{nom}"
        );
        assert!(
            est_forbidden(
                &ent.structure_grille_versionner(
                    &a,
                    grille("W", t0 + Duration::days(1), journee(), vec![])
                )
                .await
            ),
            "{nom}"
        );
        assert!(
            est_forbidden(&ent.structure_grille_supprimer(&a, g.id).await),
            "{nom}"
        );
        assert!(
            est_forbidden(
                &ent.structure_grille_coupure(&a, "STD", t0, h(12, 0), None)
                    .await
            ),
            "{nom}"
        );
    }
    for (nom, p) in [("parent", &cast.parent_lea)] {
        let a = acteur_de(etab, p);
        assert!(
            est_forbidden(&ent.structure_periodisations(&a, annee, false).await),
            "{nom} ne lit pas"
        );
        assert!(
            est_forbidden(&ent.structure_grilles(&a).await),
            "{nom} ne lit pas"
        );
        assert!(
            est_forbidden(&ent.structure_grille_a_la_date(&a, t0, None, None).await),
            "{nom} ne lit pas"
        );
        assert!(
            est_forbidden(&ent.structure_periodes_de_classe(&a, Uuid::new_v4()).await),
            "{nom} ne lit pas"
        );
    }
    // Administratif : écrit des périodes futures et des grilles, mais pas la coupure.
    let adm = acteur_de(etab, &cast.administratif);
    ent.structure_grille_versionner(
        &adm,
        grille(
            "ADM",
            t0 + Duration::days(1),
            journee(),
            vec![CibleGrilleSaisie {
                site_id: None,
                niveau_id: None,
                jours: Some(vec![6]),
            }],
        ),
    )
    .await
    .unwrap();
    assert!(est_forbidden(
        &ent.structure_grille_coupure(&adm, "STD", t0, h(12, 0), None)
            .await
    ));
    // Administrateur : écrit aussi, y compris la coupure.
    let admin = acteur_de(etab, &cast.admin);
    ent.structure_grille_coupure(&admin, "STD", t0, h(12, 0), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn isolation_entre_etablissements_periodes_et_grilles() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ((e1, a1), (e2, a2)) = (etab_neuf(&ent).await, etab_neuf(&ent).await);
    let (d1, d2) = (
        acteur_de(e1, &cast.direction),
        acteur_de(e2, &cast.direction),
    );
    let t0 = aujourdhui();
    let n1 = niveau(&ent, e1, "6EME").await;
    let c1 = classe(&ent, e1, a1, "6A", Some(n1), None, None).await;
    let pz1 = ent
        .structure_periodisation_creer(&d1, a1, modele("DEF", "trimestres", vec![cible_niveau(n1)]))
        .await
        .unwrap();
    let g1 = ent
        .structure_grille_versionner(&d1, grille("STD", t0, journee(), vec![]))
        .await
        .unwrap();

    // L'établissement 2 ne lit rien de l'établissement 1 : « introuvable », jamais « interdit ».
    for r in [
        ent.structure_periodisation(&d2, pz1.id).await.map(|_| ()),
        ent.structure_periodisation_assigner(&d2, pz1.id, vec![])
            .await
            .map(|_| ()),
        ent.structure_periodisation_archiver(&d2, pz1.id)
            .await
            .map(|_| ()),
        ent.structure_periodisation_remplacer_periodes(&d2, pz1.id, vec![])
            .await
            .map(|_| ()),
        ent.structure_periode_modifier(&d2, pz1.periodes[0].id, PeriodeModif::default())
            .await
            .map(|_| ()),
        ent.structure_periodisation_modeles(&d2, a1)
            .await
            .map(|_| ()),
        ent.structure_periodisation_controle(&d2, a1)
            .await
            .map(|_| ()),
        ent.structure_grille(&d2, g1.id).await.map(|_| ()),
        ent.structure_grille_supprimer(&d2, g1.id).await,
        ent.structure_grille_remplacer(
            &d2,
            g1.id,
            GrilleModif {
                libelle: "x".into(),
                coupure_demi_journee: None,
                motif: None,
                creneaux: journee(),
                cibles: vec![],
            },
        )
        .await
        .map(|_| ()),
        ent.structure_grille_coupure(&d2, "STD", t0, h(12, 0), None)
            .await
            .map(|_| ()),
    ] {
        assert!(est_domaine(&r, DomainError::NotFound), "{r:?}");
    }
    // Les listes de l'établissement 2 sont vides ; aucune ligne de 1 ne fuit.
    assert!(ent
        .structure_periodisations(&d2, a2, true)
        .await
        .unwrap()
        .is_empty());
    assert!(ent.structure_grilles(&d2).await.unwrap().is_empty());
    // La classe de 1 n'existe pas pour 2 : le port ne résout rien, la route répond une liste vide.
    assert!(codes_de_classe(&ent, e2, c1).await.is_empty());
    let mut t = TenantTx::begin(ent.pool(), e2).await.unwrap();
    assert!(port_grille::grille_de_classe(&mut t, e2, c1, t0)
        .await
        .is_err());
    t.commit().await.unwrap();
    // Même avec des identifiants valides de 1, une cible dans 2 est refusée.
    let r = ent
        .structure_periodisation_creer(
            &d2,
            a2,
            modele("PIRATE", "trimestres", vec![cible_niveau(n1)]),
        )
        .await;
    assert!(est_domaine(&r, DomainError::NotFound), "{r:?}");
    // Et la base refuse les clés composites croisées (établissement 2, année de l'établissement 1).
    let r = sqlx::query("INSERT INTO ent_periodisation (id, etablissement_id, annee_id, code, libelle) VALUES ($1,$2,$3,'X','X')")
        .bind(Uuid::new_v4())
        .bind(e2)
        .bind(a1)
        .execute(ent.pool())
        .await;
    assert!(r.is_err(), "clé composite (année, établissement)");
    let r = sqlx::query(
        "INSERT INTO ent_grille_cible (id, etablissement_id, grille_id) VALUES ($1,$2,$3)",
    )
    .bind(Uuid::new_v4())
    .bind(e2)
    .bind(g1.id)
    .execute(ent.pool())
    .await;
    assert!(r.is_err(), "clé composite (grille, établissement)");
}

#[tokio::test]
async fn rls_strict_tables_du_lot_s7() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    let admin = ent.pool().clone();
    const ROLE: &str = "ent_rls_app";
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(&admin)
    .await
    .unwrap();
    let tables = "ent_periodisation, ent_periodisation_cible, ent_grille, ent_grille_cible, \
                  ent_grille_creneau, ent_structure_journal";
    for sql in [
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON {tables} TO {ROLE}"),
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(&admin).await.unwrap();
    }
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE).unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(u.as_str())
        .await
        .unwrap();

    let ((e1, a1), (e2, a2)) = (etab_neuf(&ent).await, etab_neuf(&ent).await);
    let t0 = aujourdhui();
    for (e, a) in [(e1, a1), (e2, a2)] {
        let d = acteur_de(e, &cast.direction);
        ent.structure_periodisation_creer(&d, a, modele("DEF", "trimestres", vec![cible_defaut()]))
            .await
            .unwrap();
        ent.structure_grille_versionner(&d, grille("STD", t0, journee(), vec![]))
            .await
            .unwrap();
    }
    for (table, attendu) in [
        ("ent_periodisation", 1i64),
        ("ent_periodisation_cible", 1),
        ("ent_grille", 1),
        ("ent_grille_cible", 1),
        ("ent_grille_creneau", 8),
    ] {
        let mut t = TenantTx::begin_mode(&app, e1, RlsMode::Strict)
            .await
            .unwrap();
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&mut *t)
            .await
            .unwrap();
        assert_eq!(
            n, attendu,
            "{table} : seules les lignes de l'établissement courant sont visibles"
        );
        // Écrire dans l'autre établissement est refusé par la politique.
        t.commit().await.unwrap();
    }
    let mut t = TenantTx::begin_mode(&app, e1, RlsMode::Strict)
        .await
        .unwrap();
    let r = sqlx::query("INSERT INTO ent_periodisation (id, etablissement_id, annee_id, code, libelle) VALUES ($1,$2,$3,'P','P')")
        .bind(Uuid::new_v4())
        .bind(e2)
        .bind(a2)
        .execute(&mut *t)
        .await;
    assert!(
        r.is_err(),
        "écriture dans un autre établissement refusée par la RLS"
    );
    drop(t);
    // Sans contexte d'établissement en mode strict : aucune ligne.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_grille")
        .fetch_one(&app)
        .await
        .unwrap();
    assert_eq!(n, 0);
}
