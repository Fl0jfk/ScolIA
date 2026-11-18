//! Socle 21, lot V1 « modèle et reprise » : tests sur base (feature `labo`). Aucune horloge réelle :
//! l'année de ces tests est fictive (2031-2032) et toutes les dates sont fixes. Chaque scénario part
//! d'un établissement neuf du labo ; les préparations passent par `pool_verif()` (propriétaire : le
//! rôle applicatif strict ne voit rien hors contexte d'établissement).

use super::*;
use crate::evaluations_v1::*;
use crate::tests_evaluations::labo;
use chrono::NaiveDate;
use domain_eleve::evaluation_moteur::{Comptage, ReglesMoteur, StatutCalcul};
use domain_eleve::CreateEleve;
use sqlx::Row;
use uuid::Uuid;

pub(crate) fn d(a: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, j).unwrap()
}

pub(crate) const PROF: &str = "prof";
pub(crate) const DIR: &str = "direction";

pub(crate) struct W {
    pub(crate) etab: Uuid,
    pub(crate) annee: Uuid,
    /// T1 : 2031-09-01 → 2031-12-20, arrêt des notes le 2031-12-05 (périodisation par défaut).
    pub(crate) t1: Uuid,
    /// T2 : 2031-12-21 → 2032-03-31.
    pub(crate) t2: Uuid,
    pub(crate) classe_a: Uuid,
    pub(crate) classe_d: Uuid,
    pub(crate) maths: Uuid,
    pub(crate) francais: Uuid,
    /// Cours de maths de la classe A, tenu par le professeur du labo.
    pub(crate) cours_maths: Uuid,
    /// Cours de français de la classe A, tenu par un autre professeur.
    pub(crate) cours_fr: Uuid,
    pub(crate) prof: Uuid,
    pub(crate) direction: Uuid,
    /// Élèves de la classe A (trois), puis de la classe D (deux).
    pub(crate) eleves_a: Vec<Uuid>,
    pub(crate) eleves_d: Vec<Uuid>,
}

pub(crate) async fn exec(ent: &PgEnt, sql: &str, binds: Vec<Bind>) {
    let mut q = sqlx::query(sql);
    for b in binds {
        q = match b {
            Bind::U(u) => q.bind(u),
            Bind::T(t) => q.bind(t),
            Bind::D(x) => q.bind(x),
        };
    }
    q.execute(ent.pool_verif())
        .await
        .unwrap_or_else(|e| panic!("{sql} : {e}"));
}

pub(crate) enum Bind {
    U(Uuid),
    T(String),
    D(NaiveDate),
}

pub(crate) fn u(x: Uuid) -> Bind {
    Bind::U(x)
}
pub(crate) fn t(x: &str) -> Bind {
    Bind::T(x.to_string())
}
pub(crate) fn dt(x: NaiveDate) -> Bind {
    Bind::D(x)
}

pub(crate) async fn creer_eleve(ent: &PgEnt, etab: Uuid, nom: &str) -> Uuid {
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: nom.into(),
        prenom: "Test".into(),
        ine: None,
        sexe: None,
        date_naissance: None,
        numero_badge: None,
        redoublant: false,
        classe: None,
        regime: None,
        mef: None,
        site: None,
        date_entree: None,
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
    .id
}

pub(crate) async fn affecter(
    ent: &PgEnt,
    w_etab: Uuid,
    annee: Uuid,
    eleve: Uuid,
    classe: Uuid,
    debut: NaiveDate,
) {
    exec(
        ent,
        "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut, source)
         VALUES ($1, $2, $3, $4, $5, $6, 'saisie')",
        vec![u(Uuid::new_v4()), u(w_etab), u(annee), u(eleve), u(classe), dt(debut)],
    )
    .await;
}

pub(crate) async fn monde_v1(ent: &PgEnt, cast: &LaboCast) -> W {
    let etab = cast.etablissement_id;
    let annee = Uuid::new_v4();
    exec(
        ent,
        "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
         VALUES ($1, $2, '2031-2032', '2031-09-01', '2032-06-30', false)",
        vec![u(annee), u(etab)],
    )
    .await;
    let suffixe = Uuid::new_v4().simple().to_string()[..6].to_string();
    let mut classes = Vec::new();
    for code in ["A", "D"] {
        let c = Uuid::new_v4();
        exec(
            ent,
            "INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle) VALUES ($1, $2, $3, $4, $4)",
            vec![u(c), u(etab), u(annee), t(&format!("{code}{suffixe}"))],
        )
        .await;
        classes.push(c);
    }
    let mut ids_periodes = Vec::new();
    for (code, debut, fin, arret, ordre) in [
        (
            "T1",
            d(2031, 9, 1),
            d(2031, 12, 20),
            Some(d(2031, 12, 5)),
            1i16,
        ),
        (
            "T2",
            d(2031, 12, 21),
            d(2032, 3, 31),
            Some(d(2032, 3, 20)),
            2i16,
        ),
    ] {
        let p = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_periode (id, etablissement_id, annee_id, niveau_id, code, libelle, kind, ordre,
                                      date_debut, date_fin, date_arret_notes)
             VALUES ($1, $2, $3, NULL, $4, $4, 'trimestre', $5, $6, $7, $8)",
        )
        .bind(p)
        .bind(etab)
        .bind(annee)
        .bind(code)
        .bind(ordre)
        .bind(debut)
        .bind(fin)
        .bind(arret)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        ids_periodes.push(p);
    }
    let mut matieres = Vec::new();
    for (code, lib) in [("M", "Mathématiques"), ("F", "Français")] {
        let m = Uuid::new_v4();
        exec(
            ent,
            "INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, $4)",
            vec![u(m), u(etab), t(&format!("{code}{suffixe}")), t(lib)],
        )
        .await;
        matieres.push(m);
    }
    let autre_prof = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != cast.prof.user_id)
        .expect("second professeur");
    let mut cours = Vec::new();
    for (prof, matiere) in [(cast.prof.user_id, matieres[0]), (autre_prof, matieres[1])] {
        exec(
            ent,
            "INSERT INTO ent_service_enseignement (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle, classe_id)
             VALUES ($1, $2, $3, $4, $5, '', $6)",
            vec![u(Uuid::new_v4()), u(etab), u(annee), u(prof), u(matiere), u(classes[0])],
        )
        .await;
        let c: Uuid = sqlx::query_scalar(
            "SELECT cours_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND prof_user_id = $2
                AND matiere_id = $3 AND classe_id = $4",
        )
        .bind(etab)
        .bind(prof)
        .bind(matiere)
        .bind(classes[0])
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        cours.push(c);
    }
    let mut eleves_a = Vec::new();
    for nom in ["Abbott", "Bones", "Corner"] {
        let e = creer_eleve(ent, etab, nom).await;
        affecter(ent, etab, annee, e, classes[0], d(2031, 9, 1)).await;
        eleves_a.push(e);
    }
    let mut eleves_d = Vec::new();
    for nom in ["Davies", "Entwhistle"] {
        let e = creer_eleve(ent, etab, nom).await;
        affecter(ent, etab, annee, e, classes[1], d(2031, 9, 1)).await;
        eleves_d.push(e);
    }
    W {
        etab,
        annee,
        t1: ids_periodes[0],
        t2: ids_periodes[1],
        classe_a: classes[0],
        classe_d: classes[1],
        maths: matieres[0],
        francais: matieres[1],
        cours_maths: cours[0],
        cours_fr: cours[1],
        prof: cast.prof.user_id,
        direction: cast.direction.user_id,
        eleves_a,
        eleves_d,
    }
}

/// Insère une évaluation (le service de création est celui du lot V3 : ici, le fait en base).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn evaluation(
    ent: &PgEnt,
    w: &W,
    cours: Uuid,
    date: NaiveDate,
    titre: &str,
    bareme: f64,
    coef: f64,
    comptage: Comptage,
    publiee: bool,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO eva_evaluation (id, etablissement_id, annee_id, cours_id, date_eval, titre, bareme, coef,
                                     comptage, publication, publiee_le, publiee_par, cree_par)
         VALUES ($1, $2, $3, $4, $5, $6, $7::float8::numeric, $8::float8::numeric, $9, 'explicite',
                 CASE WHEN $10 THEN '2031-10-01T08:00:00Z'::timestamptz END, CASE WHEN $10 THEN $11 END, $11)",
    )
    .bind(id)
    .bind(w.etab)
    .bind(w.annee)
    .bind(cours)
    .bind(date)
    .bind(titre)
    .bind(bareme)
    .bind(coef)
    .bind(comptage.as_str())
    .bind(publiee)
    .bind(w.prof)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

pub(crate) fn note(ev: Uuid, eleve: Uuid, valeur_m: i64) -> EcrireResultat {
    EcrireResultat {
        evaluation_id: ev,
        eleve_id: eleve,
        statut: StatutCalcul::Note,
        valeur_m: Some(valeur_m),
        motif: None,
        commentaire_eleve: None,
        source: SourceResultat::Saisie,
    }
}

pub(crate) fn statut(ev: Uuid, eleve: Uuid, s: StatutCalcul) -> EcrireResultat {
    EcrireResultat {
        evaluation_id: ev,
        eleve_id: eleve,
        statut: s,
        valeur_m: None,
        motif: None,
        commentaire_eleve: None,
        source: SourceResultat::Saisie,
    }
}

pub(crate) async fn arreter(ent: &PgEnt, w: &W, periode: Uuid, classe: Uuid) {
    ent.eva_v1_changer_etat_periode_for_actor(
        w.etab,
        w.direction,
        DIR,
        periode,
        classe,
        "notes_arretees",
        None,
        Some(d(2031, 12, 5)),
    )
    .await
    .unwrap();
}

// ───────────────────────── Structure : tables, RLS, immuabilité ─────────────────────────

#[tokio::test]
async fn un_resultat_se_remplace_jamais_ne_se_supprime_et_laisse_son_historique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS 1",
        20.0,
        2.0,
        Comptage::Normal,
        false,
    )
    .await;
    let a = w.eleves_a[0];
    let r1 = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 12_500))
        .await
        .unwrap();
    assert_eq!((r1.version, r1.correction), (1, false));
    let r2 = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 14_000))
        .await
        .unwrap();
    assert_eq!(
        r2.resultat_id, r1.resultat_id,
        "un seul résultat par (évaluation, élève) : D8"
    );
    assert_eq!(r2.version, 2);
    // Vider une cellule = « absent », jamais une suppression physique (D7) ; l'ancienne valeur reste.
    ent.eva_v1_ecrire_resultat_for_actor(
        w.etab,
        w.prof,
        PROF,
        &statut(ev, a, StatutCalcul::Absent),
    )
    .await
    .unwrap();
    let h = ent
        .eva_v1_historique_resultat_for_actor(w.etab, w.direction, DIR, r1.resultat_id)
        .await
        .unwrap();
    assert_eq!(h.len(), 3);
    assert_eq!(
        (h[0].statut_avant.clone(), h[0].valeur_apres_m),
        (None, Some(12_500))
    );
    assert_eq!(
        (h[1].valeur_avant_m, h[1].valeur_apres_m),
        (Some(12_500), Some(14_000))
    );
    assert_eq!(
        (h[2].statut_apres.as_str(), h[2].valeur_avant_m),
        ("absent", Some(14_000))
    );
    assert!(h.iter().all(|l| l.auteur == Some(w.prof) && !l.correction));

    // Les gardes sont en base : même le propriétaire ne supprime ni ne réécrit un historique.
    let p = ent.pool_verif();
    assert!(sqlx::query("DELETE FROM eva_resultat WHERE id = $1")
        .bind(r1.resultat_id)
        .execute(p)
        .await
        .is_err());
    assert!(
        sqlx::query("DELETE FROM eva_resultat_historique WHERE resultat_id = $1")
            .bind(r1.resultat_id)
            .execute(p)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE eva_resultat_historique SET motif = 'x' WHERE resultat_id = $1")
            .bind(r1.resultat_id)
            .execute(p)
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM eva_evaluation WHERE id = $1")
        .bind(ev)
        .execute(p)
        .await
        .is_err());
    // Un statut non numérique ne porte jamais de valeur : une absence n'est jamais un zéro (R-V5).
    assert!(
        sqlx::query("UPDATE eva_resultat SET valeur = 0 WHERE id = $1")
            .bind(r1.resultat_id)
            .execute(p)
            .await
            .is_err()
    );
    let (statut_final, valeur): (String, Option<String>) =
        sqlx::query_as("SELECT statut, valeur::text FROM eva_resultat WHERE id = $1")
            .bind(r1.resultat_id)
            .fetch_one(p)
            .await
            .unwrap();
    assert_eq!((statut_final.as_str(), valeur), ("absent", None));
}

#[tokio::test]
async fn saisie_refusee_hors_bareme_hors_pas_et_pour_un_eleve_non_concerne() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS 1",
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await;
    let a = w.eleves_a[0];
    // 23 sur 20 : « vérifiez le barème ».
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 23_000))
        .await
        .is_err());
    // Une note avec statut non numérique, ou une absence avec valeur : incohérent.
    let mut mauvais = note(ev, a, 5_000);
    mauvais.statut = StatutCalcul::Absent;
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &mauvais)
        .await
        .is_err());
    // `non_concerne` se calcule, il ne se saisit pas.
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(
            w.etab,
            w.prof,
            PROF,
            &statut(ev, a, StatutCalcul::NonConcerne)
        )
        .await
        .is_err());
    // Un élève de la classe D n'est pas dans le public du cours de maths de la classe A : non concerné (R-V2).
    let hors = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, w.eleves_d[0], 10_000))
        .await;
    assert!(hors.is_err(), "élève hors du public du cours");
    // Le pas de saisie de l'établissement s'applique : demi-point refuse 13,3.
    let mut regles = ReglesMoteur::default();
    regles.affichage.precision = domain_eleve::evaluation_moteur::Precision::DemiPoint;
    ent.eva_v1_definir_regles_for_actor(w.etab, w.direction, DIR, w.annee, &regles, 500)
        .await
        .unwrap();
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 13_300))
        .await
        .is_err());
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 13_500))
        .await
        .unwrap();
    // Un autre professeur (celui de français) ne note pas en maths.
    let autre = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .unwrap();
    assert!(matches!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, autre, PROF, &note(ev, a, 11_000))
            .await,
        Err(EntDbError::Forbidden)
    ));
}

#[tokio::test]
async fn machine_d_etats_de_periode_un_cran_a_la_fois() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let change = |cible: &'static str| {
        let (ent, etab, dir, p, c) = (&ent, w.etab, w.direction, w.t1, w.classe_a);
        async move {
            ent.eva_v1_changer_etat_periode_for_actor(
                etab,
                dir,
                DIR,
                p,
                c,
                cible,
                None,
                Some(d(2031, 12, 5)),
            )
            .await
        }
    };
    // Pas de saut : ouverte → conseil_tenu, notes_arretees → close sont refusés ; un état inconnu aussi.
    assert!(change("conseil_tenu").await.is_err());
    assert!(change("inconnu").await.is_err());
    change("notes_arretees").await.unwrap();
    assert!(change("close").await.is_err());
    assert!(change("conseil_tenu").await.is_err());
    change("appreciations_closes").await.unwrap();
    change("conseil_tenu").await.unwrap();
    change("close").await.unwrap();
    // Plus rien n'avance après « close ».
    assert!(
        change("close").await.is_ok(),
        "rester dans le même état est sans effet"
    );
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM eva_periode_transition WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3",
    )
    .bind(w.etab)
    .bind(w.t1)
    .bind(w.classe_a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        n, 4,
        "une ligne par transition, aucune pour un état inchangé"
    );
}

#[tokio::test]
async fn reouverture_d_une_periode_close_exige_un_motif_renforce_et_laisse_des_traces() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    ent.eva_v1_definir_regles_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.annee,
        &ReglesMoteur::default(),
        10,
    )
    .await
    .unwrap();
    for cible in [
        "notes_arretees",
        "appreciations_closes",
        "conseil_tenu",
        "close",
    ] {
        ent.eva_v1_changer_etat_periode_for_actor(
            w.etab,
            w.direction,
            DIR,
            w.t1,
            w.classe_a,
            cible,
            None,
            Some(d(2031, 12, 5)),
        )
        .await
        .unwrap();
    }
    for motif in [None, Some(""), Some("ab"), Some("neuf cara")] {
        assert!(
            ent.eva_v1_changer_etat_periode_for_actor(
                w.etab,
                w.direction,
                DIR,
                w.t1,
                w.classe_a,
                "ouverte",
                motif,
                None
            )
            .await
            .is_err(),
            "motif {motif:?}"
        );
    }
    // Un administratif, un professeur ne rouvrent pas (D2) ; la direction, avec un vrai motif, oui.
    for (user, role) in [
        (cast.administratif.user_id, "administratif"),
        (w.prof, PROF),
    ] {
        assert!(matches!(
            ent.eva_v1_changer_etat_periode_for_actor(
                w.etab,
                user,
                role,
                w.t1,
                w.classe_a,
                "ouverte",
                Some("erreur de report des notes"),
                None
            )
            .await,
            Err(EntDbError::Forbidden)
        ));
    }
    ent.eva_v1_changer_etat_periode_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.t1,
        w.classe_a,
        "ouverte",
        Some("erreur de report des notes"),
        None,
    )
    .await
    .unwrap();
    let transitions: Vec<(String, String, bool, Option<String>, String)> = sqlx::query_as(
        "SELECT etat_avant, etat_apres, reouverture, motif, origine FROM eva_periode_transition
          WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3 ORDER BY le, id",
    )
    .bind(w.etab)
    .bind(w.t1)
    .bind(w.classe_a)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(transitions.len(), 5);
    let derniere = transitions.last().unwrap();
    assert_eq!(
        (derniere.0.as_str(), derniere.1.as_str(), derniere.2),
        ("close", "ouverte", true)
    );
    assert_eq!(derniere.3.as_deref(), Some("erreur de report des notes"));
    // Le journal est en ajout seul.
    assert!(
        sqlx::query("DELETE FROM eva_periode_transition WHERE etablissement_id = $1")
            .bind(w.etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // Le premier arrêt a figé les règles de l'année (R-V21) : l'échelle ne change plus.
    let regles = ReglesMoteur {
        echelle_m: 10_000,
        ..ReglesMoteur::default()
    };
    assert!(ent
        .eva_v1_definir_regles_for_actor(w.etab, w.direction, DIR, w.annee, &regles, 10)
        .await
        .is_err());
}

#[tokio::test]
async fn apres_l_arret_seule_la_direction_corrige_avec_un_motif_et_le_journal_le_dit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 11, 12),
        "DS 2",
        20.0,
        2.0,
        Comptage::Normal,
        true,
    )
    .await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let r = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 9_960))
        .await
        .unwrap();
    // `b` est « à rattraper » avant l'arrêt de la période.
    ent.eva_v1_ecrire_resultat_for_actor(
        w.etab,
        w.prof,
        PROF,
        &statut(ev, b, StatutCalcul::ARattraper),
    )
    .await
    .unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    // Le professeur ne saisit plus (R-V10) ; la direction non plus sans motif.
    assert!(matches!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 11_000))
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev, a, 10_040))
        .await
        .is_err());
    let mut corr = note(ev, a, 10_040);
    corr.motif = Some("erreur de report de la copie".into());
    let c = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &corr)
        .await
        .unwrap();
    assert!(c.correction);
    // Même motif répété pour une seconde correction : il vaut pour cette écriture, pas pour la suivante.
    let mut corr2 = note(ev, a, 10_050);
    corr2.motif = Some("erreur de report de la copie".into());
    assert!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &corr2)
            .await
            .unwrap()
            .correction
    );
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev, a, 10_060))
        .await
        .is_err());
    let h = ent
        .eva_v1_historique_resultat_for_actor(w.etab, w.direction, DIR, r.resultat_id)
        .await
        .unwrap();
    assert_eq!(h.len(), 3);
    assert!(!h[0].correction && h[1].correction && h[2].correction);
    assert_eq!(h[1].motif.as_deref(), Some("erreur de report de la copie"));
    assert_eq!(
        (h[1].valeur_avant_m, h[1].valeur_apres_m, h[1].auteur),
        (Some(9_960), Some(10_040), Some(w.direction))
    );
    // Rattrapage autorisé (R-V10) : le professeur peut noter un élève à rattraper, sans motif.
    assert!(matches!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, b, 11_000))
            .await,
        Err(EntDbError::Forbidden)
    ));
    ent.eva_v1_autoriser_rattrapage_for_actor(
        w.etab,
        w.direction,
        DIR,
        ev,
        b,
        d(2099, 12, 12),
        "absent justifié",
    )
    .await
    .unwrap();
    let ok = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, b, 11_000))
        .await
        .unwrap();
    assert!(!ok.correction, "rattrapage autorisé : saisie ordinaire");
    // Le professeur ne peut pas autoriser lui-même.
    assert!(matches!(
        ent.eva_v1_autoriser_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            ev,
            w.eleves_a[2],
            d(2099, 12, 12),
            "moi-même"
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
}

// ───────────────────────── Période par élève (R-V29) ─────────────────────────

#[tokio::test]
async fn ce65_une_evaluation_deux_classes_arretees_a_des_dates_differentes() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // Un groupe transverse réunit des élèves des classes A et D : un seul cours, deux classes.
    let groupe = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle) VALUES ($1, $2, $3, 'SPE', 'Spécialité')",
        vec![u(groupe), u(w.etab), u(w.annee)],
    )
    .await;
    exec(
        &ent,
        "INSERT INTO ent_service_enseignement (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle, groupe_id)
         VALUES ($1, $2, $3, $4, $5, '', $6)",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.annee), u(w.prof), u(w.francais), u(groupe)],
    )
    .await;
    let cours_groupe: Uuid = sqlx::query_scalar(
        "SELECT cours_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND groupe_id = $2",
    )
    .bind(w.etab)
    .bind(groupe)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let (ea, ed) = (w.eleves_a[0], w.eleves_d[0]);
    for e in [ea, ed] {
        exec(
            &ent,
            "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, source)
             VALUES ($1, $2, $3, $4, $5, '2031-09-01', 'saisie')",
            vec![u(Uuid::new_v4()), u(w.etab), u(w.annee), u(e), u(groupe)],
        )
        .await;
    }
    // La classe A est arrêtée (5 décembre) ; la classe D ne l'est pas. Devoir du 1ᵉʳ décembre.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let ev = evaluation(
        &ent,
        &w,
        cours_groupe,
        d(2031, 12, 1),
        "DS commun",
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await;
    let pa = ent
        .eva_v1_periode_eleve(w.etab, ev, ea)
        .await
        .unwrap()
        .expect("période de l'élève de A");
    let pd = ent
        .eva_v1_periode_eleve(w.etab, ev, ed)
        .await
        .unwrap()
        .expect("période de l'élève de D");
    assert_eq!(
        (pa.classe_id, pa.periode_id, pa.etat.as_str()),
        (w.classe_a, w.t1, "notes_arretees")
    );
    assert_eq!(
        (pd.classe_id, pd.periode_id, pd.etat.as_str()),
        (w.classe_d, w.t1, "ouverte")
    );
    // La grille verrouille cellule par cellule.
    let grille = ent
        .eva_v1_grille_eleves_for_actor(w.etab, w.prof, PROF, ev)
        .await
        .unwrap();
    assert_eq!(grille.len(), 2);
    let cel = |e: Uuid| grille.iter().find(|c| c.eleve_id == e).unwrap();
    assert!(!cel(ea).saisie_ouverte && cel(ed).saisie_ouverte);
    assert_eq!(cel(ea).etat.as_deref(), Some("notes_arretees"));
    // Saisie : refusée (prof) pour la cellule arrêtée, acceptée pour l'autre.
    assert!(matches!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, ea, 12_000))
            .await,
        Err(EntDbError::Forbidden)
    ));
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, ed, 12_000))
        .await
        .unwrap();
    // `eva_evaluation.periode_id` n'est qu'indicative : la renseigner à tort n'y change rien.
    exec(
        &ent,
        "UPDATE eva_evaluation SET periode_id = $2 WHERE id = $1",
        vec![u(ev), u(w.t2)],
    )
    .await;
    let pa2 = ent
        .eva_v1_periode_eleve(w.etab, ev, ea)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        pa2.periode_id, w.t1,
        "la période d'un élève ne vient jamais de la période indicative"
    );
}

#[tokio::test]
async fn ce66_option_multi_niveaux_trimestres_pour_les_uns_semestres_pour_les_autres() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // La classe D passe en semestres (périodisation propre, cible = la classe).
    let pz = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_periodisation (id, etablissement_id, annee_id, code, libelle, usage) VALUES ($1, $2, $3, 'SEM', 'Semestres', 'evaluation')",
        vec![u(pz), u(w.etab), u(w.annee)],
    )
    .await;
    exec(
        &ent,
        "INSERT INTO ent_periodisation_cible (id, etablissement_id, annee_id, usage, periodisation_id, classe_id)
         VALUES ($1, $2, $3, 'evaluation', $4, $5)",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.annee), u(pz), u(w.classe_d)],
    )
    .await;
    let s1 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_periode (id, etablissement_id, annee_id, periodisation_id, code, libelle, kind, ordre,
                                  date_debut, date_fin, date_arret_notes, origine)
         VALUES ($1, $2, $3, $4, 'S1', 'Semestre 1', 'semestre', 1, '2031-09-01', '2032-01-31', '2032-01-20', 'service')",
    )
    .bind(s1)
    .bind(w.etab)
    .bind(w.annee)
    .bind(pz)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    // Un enseignement multi-niveaux : cours de groupe, élèves des classes A (trimestres) et D (semestres).
    let groupe = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle) VALUES ($1, $2, $3, 'DGEMC', 'DGEMC')",
        vec![u(groupe), u(w.etab), u(w.annee)],
    )
    .await;
    exec(
        &ent,
        "INSERT INTO ent_service_enseignement (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle, groupe_id)
         VALUES ($1, $2, $3, $4, $5, '', $6)",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.annee), u(w.prof), u(w.francais), u(groupe)],
    )
    .await;
    let cours: Uuid = sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND groupe_id = $2")
        .bind(w.etab)
        .bind(groupe)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let (ea, ed) = (w.eleves_a[0], w.eleves_d[0]);
    for e in [ea, ed] {
        exec(
            &ent,
            "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, source) VALUES ($1, $2, $3, $4, $5, '2031-09-01', 'saisie')",
            vec![u(Uuid::new_v4()), u(w.etab), u(w.annee), u(e), u(groupe)],
        )
        .await;
    }
    let ev = evaluation(
        &ent,
        &w,
        cours,
        d(2032, 1, 10),
        "Oral",
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await;
    let pa = ent
        .eva_v1_periode_eleve(w.etab, ev, ea)
        .await
        .unwrap()
        .unwrap();
    let pd = ent
        .eva_v1_periode_eleve(w.etab, ev, ed)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        pa.periode_id, w.t2,
        "le 10 janvier est au T2 pour la classe en trimestres"
    );
    assert_eq!(pd.periode_id, s1, "et au S1 pour la classe en semestres");
    // La grille affiche les deux.
    let grille = ent
        .eva_v1_grille_eleves_for_actor(w.etab, w.prof, PROF, ev)
        .await
        .unwrap();
    let periodes: std::collections::BTreeSet<Option<Uuid>> =
        grille.iter().map(|c| c.periode_id).collect();
    assert_eq!(periodes.len(), 2);
    // Une date hors de toute période de la classe : aucune période (pas de valeur inventée).
    let hors = evaluation(
        &ent,
        &w,
        cours,
        d(2032, 5, 10),
        "Tard",
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await;
    assert!(
        ent.eva_v1_periode_eleve(w.etab, hors, ed)
            .await
            .unwrap()
            .is_none(),
        "S1 est finie au 31 janvier, pas de S2"
    );
}

#[tokio::test]
async fn aucune_fonction_de_controle_ne_lit_la_periode_indicative_de_l_evaluation() {
    let Some((ent, _cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let fautes: Vec<(String,)> = sqlx::query_as(
        "SELECT p.proname::text FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
          WHERE n.nspname = 'public' AND (p.proname LIKE 'eva\\_%' OR p.proname LIKE 'ent\\_trg\\_eva\\_%')
            AND p.prosrc ~* '(eva_evaluation|ev)\\.periode_id'",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(
        fautes.is_empty(),
        "lecture de eva_evaluation.periode_id dans : {fautes:?}"
    );
}

// ───────────────────────── Règles de calcul paramétrables ─────────────────────────

#[tokio::test]
async fn les_regles_de_calcul_se_posent_se_relisent_et_s_historisent() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // Sans ligne : les défauts recommandés.
    let defaut = ent.eva_v1_regles(w.etab, w.annee).await.unwrap();
    assert_eq!(defaut.regles, ReglesMoteur::default());
    assert_eq!((defaut.version, defaut.figees), (0, false));
    use domain_eleve::evaluation_moteur::*;
    let r = ReglesMoteur {
        ponderation: PonderationNotes::PointsCumules,
        non_rendu: TraitementNonRendu::Zero,
        bonus: TraitementBonus::PointsAuDessusDe { seuil_m: 10_000 },
        sous_matieres: MethodeSousMatieres::ToutesLesNotes,
        generale: PonderationGenerale::NonPonderee,
        annuelle: MethodeAnnuelle::ToutesLesEvaluations,
        mediane: MedianePaire::Superieure,
        seuil_effectif_stats: 8,
        plafonner_a_l_echelle: false,
        affichage: ArrondiRegle::new(Precision::Dixieme, ModeArrondi::DemiPair),
        arrondis: EtapesArrondi {
            note_ramenee: Some(ArrondiRegle::new(
                Precision::Centieme,
                ModeArrondi::DemiHaut,
            )),
            moyenne_matiere: Some(ArrondiRegle::new(Precision::Quart, ModeArrondi::Superieur)),
            ..EtapesArrondi::default()
        },
        ..ReglesMoteur::default()
    };
    let pose = ent
        .eva_v1_definir_regles_for_actor(w.etab, w.direction, DIR, w.annee, &r, 500)
        .await
        .unwrap();
    assert_eq!(pose.regles, r, "relecture fidèle de chaque réglage");
    assert_eq!((pose.pas_saisie_m, pose.version), (500, 1));
    let mut r2 = r;
    r2.generale = PonderationGenerale::Ponderee;
    let v2 = ent
        .eva_v1_definir_regles_for_actor(w.etab, w.direction, DIR, w.annee, &r2, 500)
        .await
        .unwrap();
    assert_eq!(v2.version, 2);
    let histo: Vec<(i32, String)> = sqlx::query_as(
        "SELECT version, generale FROM eva_reglage_historique WHERE etablissement_id = $1 ORDER BY version",
    )
    .bind(w.etab)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        histo,
        vec![(1, "non_ponderee".to_string()), (2, "ponderee".to_string())]
    );
    // Le professeur ne règle rien ; une échelle absurde est refusée.
    assert!(matches!(
        ent.eva_v1_definir_regles_for_actor(w.etab, w.prof, PROF, w.annee, &r, 10)
            .await,
        Err(EntDbError::Forbidden)
    ));
    let mut mauvaise = r;
    mauvaise.echelle_m = 0;
    assert!(ent
        .eva_v1_definir_regles_for_actor(w.etab, w.direction, DIR, w.annee, &mauvaise, 10)
        .await
        .is_err());
    // Un jeu de règles ne se supprime pas.
    assert!(
        sqlx::query("DELETE FROM eva_reglage WHERE etablissement_id = $1")
            .bind(w.etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
}

// ───────────────────────── Publication, élève protégé, événements neutres ─────────────────────────

#[tokio::test]
async fn une_note_non_publiee_n_est_visible_ni_de_la_famille_ni_de_l_eleve() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    for (user, voit) in [
        (cast.parent_lea.user_id, true),
        (cast.parent_tom.user_id, false),
    ] {
        ent.link_parent(crate::foyer_repo::CreateLien {
            etablissement_id: w.etab,
            eleve_id: a,
            user_id: user,
            foyer_id: None,
            lien: "responsable".into(),
            peut_voir_notes: voit,
            peut_voir_vs: true,
            peut_justifier_absence: false,
            peut_autoriser_sorties: false,
            est_payeur: false,
            part_paiement: None,
        })
        .await
        .unwrap();
    }
    let prive = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "Privé",
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await;
    let publie = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 17),
        "Publié",
        20.0,
        1.0,
        Comptage::Normal,
        true,
    )
    .await;
    for ev in [prive, publie] {
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 15_000))
            .await
            .unwrap();
    }
    // La note privée du professeur n'est jamais dans la vue.
    exec(
        &ent,
        "UPDATE eva_resultat SET note_privee = 'à surveiller' WHERE evaluation_id = $1",
        vec![u(publie)],
    )
    .await;
    let vues = ent
        .eva_v1_resultats_publies_for_actor(w.etab, cast.parent_lea.user_id, "parent", a)
        .await
        .unwrap();
    assert_eq!(vues.len(), 1);
    assert_eq!(
        (vues[0].evaluation_id, vues[0].valeur_m, vues[0].bareme_m),
        (publie, Some(15_000), Some(20_000))
    );
    let colonnes: Vec<(String,)> = sqlx::query_as(
        "SELECT column_name::text FROM information_schema.columns WHERE table_name = 'eva_resultat_publie'",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(colonnes
        .iter()
        .all(|(c,)| c != "note_privee" && c != "saisi_par" && c != "source"));
    // Parent privé du droit aux notes ; personne sans lien : refusés.
    assert!(matches!(
        ent.eva_v1_resultats_publies_for_actor(w.etab, cast.parent_tom.user_id, "parent", a)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v1_resultats_publies_for_actor(
            w.etab,
            cast.parent_lea.user_id,
            "parent",
            w.eleves_a[1]
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    // Retrait de publication : la note disparaît de la vue sans disparaître de la base.
    exec(
        &ent,
        "UPDATE eva_evaluation SET publiee_le = NULL, publiee_par = NULL WHERE id = $1",
        vec![u(publie)],
    )
    .await;
    assert!(ent
        .eva_v1_resultats_publies_for_actor(w.etab, cast.parent_lea.user_id, "parent", a)
        .await
        .unwrap()
        .is_empty());
    // Le moteur côté famille ne compte que ce qui est publié ; côté personnel, tout.
    let famille = ent
        .eva_v1_moyennes_eleve_periode(w.etab, a, w.t1, true)
        .await
        .unwrap();
    assert!(famille.generale.exacte.is_none());
    let personnel = ent
        .eva_v1_moyennes_eleve_periode(w.etab, a, w.t1, false)
        .await
        .unwrap();
    assert_eq!(personnel.generale.affichee_m, Some(15_000));
}

#[tokio::test]
async fn eleve_protege_introuvable_pour_un_tiers_et_absent_de_la_grille() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS",
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await;
    let protege = w.eleves_a[2];
    exec(
        &ent,
        "INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, niveau, motif_scelle, du, decide_par)
         VALUES ($1, $2, $3, 'renforcee', 'enc:v1:mesure', '2020-01-01', $4)",
        vec![u(Uuid::new_v4()), u(w.etab), u(protege), u(w.direction)],
    )
    .await;
    // Un membre du personnel sans lien direct avec l'élève (ici le secrétariat) ne le voit pas dans la
    // grille et ne peut pas écrire à son nom : la réponse est celle d'un élève absent.
    let tiers = (cast.administratif.user_id, "administratif");
    let grille = ent
        .eva_v1_grille_eleves_for_actor(w.etab, tiers.0, tiers.1, ev)
        .await
        .unwrap();
    assert!(grille.iter().all(|c| c.eleve_id != protege));
    assert_eq!(grille.len(), 2);
    assert!(matches!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, tiers.0, tiers.1, &note(ev, protege, 10_000))
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Pour un élève ordinaire, le même acteur reçoit un refus de droit, pas « introuvable ».
    assert!(matches!(
        ent.eva_v1_ecrire_resultat_for_actor(
            w.etab,
            tiers.0,
            tiers.1,
            &note(ev, w.eleves_a[0], 10_000)
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    // Même le professeur du cours ne voit pas l'élève protégé (seul G6 le voit : la direction).
    assert_eq!(
        ent.eva_v1_grille_eleves_for_actor(w.etab, w.prof, PROF, ev)
            .await
            .unwrap()
            .len(),
        2
    );
    // La direction (accès aux protégés) le voit et écrit.
    let grille_dir = ent
        .eva_v1_grille_eleves_for_actor(w.etab, w.direction, DIR, ev)
        .await
        .unwrap();
    assert_eq!(grille_dir.len(), 3);
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev, protege, 10_000))
        .await
        .unwrap();
}

#[tokio::test]
async fn les_evenements_de_v1_sont_neutres_aucune_note_aucune_moyenne_aucun_nom() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 11, 12),
        "DS",
        20.0,
        1.0,
        Comptage::Normal,
        true,
    )
    .await;
    let a = w.eleves_a[0];
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 13_380))
        .await
        .unwrap();
    ent.eva_v1_definir_regles_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.annee,
        &ReglesMoteur::default(),
        10,
    )
    .await
    .unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let mut corr = note(ev, a, 13_880);
    corr.motif = Some("erreur de report très confidentielle".into());
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &corr)
        .await
        .unwrap();
    ent.eva_v1_changer_etat_periode_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.t1,
        w.classe_a,
        "ouverte",
        Some("réouverture de test"),
        None,
    )
    .await
    .unwrap();
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    let evenements: Vec<(String, String)> = sqlx::query_as(
        "SELECT type, payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'evaluations.%' ORDER BY created_at, id",
    )
    .bind(w.etab)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let types: Vec<&str> = evenements.iter().map(|(t, _)| t.as_str()).collect();
    for attendu in [
        "evaluations.resultats_saisis",
        "evaluations.regles_modifiees",
        "evaluations.notes_arretees",
        "evaluations.corrigee",
        "evaluations.periode_rouverte",
        "evaluations.reprise_terminee",
    ] {
        assert!(
            types.contains(&attendu),
            "événement {attendu} manquant : {types:?}"
        );
    }
    const CLES_AUTORISEES: [&str; 6] = [
        "evaluation_id",
        "resultat_id",
        "version",
        "annee_id",
        "periode_id",
        "classe_id",
    ];
    for (type_, charge) in &evenements {
        let v: serde_json::Value = serde_json::from_str(charge).unwrap();
        let obj = v.as_object().expect("charge en objet");
        for cle in obj.keys() {
            assert!(
                CLES_AUTORISEES.contains(&cle.as_str()) || cle == "execution_id",
                "{type_} : clé « {cle} » non neutre"
            );
        }
        // Ni la valeur de la note, ni le motif, ni un nom, ni le statut ne sont dans la charge.
        for interdit in [
            "13.38",
            "13380",
            "13.88",
            "13880",
            "confidentielle",
            "Abbott",
            "Test",
            "absent",
            "valeur",
            "moyenne",
        ] {
            assert!(
                !charge.contains(interdit),
                "{type_} : « {interdit} » dans la charge"
            );
        }
    }
}

#[tokio::test]
async fn rls_sur_les_tables_eva_isole_les_etablissements() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS",
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, w.eleves_a[0], 11_000))
        .await
        .unwrap();
    ent.eva_v1_definir_regles_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.annee,
        &ReglesMoteur::default(),
        10,
    )
    .await
    .unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    const ROLE: &str = "ent_eva_v1_app";
    let tables = [
        "eva_reglage",
        "eva_reglage_historique",
        "eva_coefficient_matiere",
        "eva_coefficient_sous_matiere",
        "eva_nature",
        "eva_evaluation_commune",
        "eva_evaluation",
        "eva_evaluation_restriction",
        "eva_bareme_item",
        "eva_periode_etat",
        "eva_periode_transition",
        "eva_rattrapage_autorise",
        "eva_instantane",
        "eva_instantane_ligne",
        "eva_resultat",
        "eva_resultat_historique",
        "eva_resultat_item",
        "eva_moyenne_statut",
        "eva_appreciation",
        "eva_appreciation_historique",
        "eva_banque_phrase",
        "eva_correction_demande",
        "eva_revision_demande",
        "eva_consultation",
        "eva_resultat_externe",
        "eva_enseignement_externe",
        "eva_moyenne_externe",
        "eva_reprise_execution",
        "eva_reprise_compte",
        "eva_reprise_ligne",
    ];
    for sql in [
        format!("DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; EXCEPTION WHEN duplicate_object THEN NULL; END $$"),
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT ON {} TO {ROLE}", tables.join(", ")),
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(ent.pool_verif()).await.unwrap();
    }
    let mut url = url::Url::parse(&url).unwrap();
    url.set_username(ROLE).unwrap();
    url.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(url.as_str())
        .await
        .unwrap();
    for table in [
        "eva_reglage",
        "eva_evaluation",
        "eva_resultat",
        "eva_resultat_historique",
        "eva_periode_etat",
        "eva_periode_transition",
    ] {
        let compter = |etab_pose: Option<Uuid>| {
            let app = app.clone();
            async move {
                let mut tx = app.begin().await.unwrap();
                if let Some(e) = etab_pose {
                    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
                        .bind(e.to_string())
                        .execute(&mut *tx)
                        .await
                        .unwrap();
                }
                let n: i64 = sqlx::query(&format!("SELECT count(*) FROM {table}"))
                    .fetch_one(&mut *tx)
                    .await
                    .unwrap()
                    .get(0);
                n
            }
        };
        assert!(
            compter(Some(w.etab)).await >= 1,
            "{table} : l'établissement voit ses lignes"
        );
        assert_eq!(
            compter(Some(Uuid::new_v4())).await,
            0,
            "{table} : un autre établissement ne voit rien"
        );
        assert_eq!(
            compter(None).await,
            0,
            "{table} : strict sans établissement posé : aucune ligne"
        );
    }
    // Un autre établissement du labo ne lit pas non plus ces résultats par l'API.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let vue: Vec<CelluleGrille> = ent
        .eva_v1_grille_eleves_for_actor(autre.etablissement_id, autre.direction.user_id, DIR, ev)
        .await
        .unwrap_or_default();
    assert!(
        vue.is_empty(),
        "une évaluation d'un autre établissement est introuvable"
    );
}

// ───────────────────────── Reprise M2 ─────────────────────────

struct Ancien {
    note_periode: Uuid,
    devoirs: Vec<Uuid>,
}

/// Données « historiques » écrites directement dans les anciennes tables (l'ancien écrivain est
/// fermé ou migré : on rejoue ce qu'il a laissé), dates fixes.
async fn ancien_monde(ent: &PgEnt, w: &W) -> Ancien {
    let np = Uuid::new_v4();
    exec(
        ent,
        "INSERT INTO ent_note_periode (id, etablissement_id, code, libelle, date_debut, date_fin, statut, periode_id)
         VALUES ($1, $2, $3, 'Trimestre 1', '2031-09-01', '2031-12-20', 'ouverte', $4)",
        vec![u(np), u(w.etab), t(&format!("T1-{}", &np.simple().to_string()[..6])), u(w.t1)],
    )
    .await;
    let service_maths: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(w.etab)
    .bind(w.cours_maths)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let classe_code: String =
        sqlx::query_scalar("SELECT code FROM ent_classe WHERE etablissement_id = $1 AND id = $2")
            .bind(w.etab)
            .bind(w.classe_a)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    let mut devoirs = Vec::new();
    // (titre, barème, coef, date, service ?, publication explicite, publié ?)
    for (titre, sur, coef, date, avec_service, explicite, publie) in [
        ("DS 1", 20.0_f64, 2.0_f64, d(2031, 10, 10), true, true, true),
        ("Interro", 10.0, 1.0, d(2031, 10, 17), true, true, true),
        ("DM", 20.0, 1.0, d(2031, 10, 24), true, false, false),
        ("Hors service", 20.0, 1.0, d(2031, 11, 5), false, true, true),
    ] {
        let dv = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_devoir (id, etablissement_id, periode_id, matiere_id, classe, classe_id, service_id, titre,
                                     date_devoir, coef, sur, created_by_user_id, publication_explicite, publie_le, publie_par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                     CASE WHEN $14 THEN '2031-11-20T10:00:00Z'::timestamptz END, CASE WHEN $14 THEN $12 END)",
        )
        .bind(dv)
        .bind(w.etab)
        .bind(np)
        .bind(w.maths)
        .bind(&classe_code)
        .bind(w.classe_a)
        .bind(avec_service.then_some(service_maths))
        .bind(titre)
        .bind(date)
        .bind(coef)
        .bind(sur)
        .bind(w.prof)
        .bind(explicite)
        .bind(publie)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        devoirs.push(dv);
    }
    Ancien {
        note_periode: np,
        devoirs,
    }
}

async fn ancienne_note(
    ent: &PgEnt,
    w: &W,
    devoir: Uuid,
    eleve: Uuid,
    valeur: f64,
    sur: f64,
    statut: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, sur, date_note, auteur_user_id, devoir_id, statut)
         VALUES ($1, $2, $3, $4, $5, $6, '2031-10-10', $7, $8, $9)",
    )
    .bind(id)
    .bind(w.etab)
    .bind(eleve)
    .bind(w.maths)
    .bind(valeur)
    .bind(sur)
    .bind(w.prof)
    .bind(devoir)
    .bind(statut)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn reprise_idempotente_rapport_et_invariant() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // Le semis du labo a lui-même des évaluations historiques : on les absorbe d'abord, pour que le
    // rapport qui suit ne parle que des données de ce scénario.
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    let anc = ancien_monde(&ent, &w).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let n_ok = ancienne_note(&ent, &w, anc.devoirs[0], a, 14.0, 20.0, "note").await;
    let n_corrigee = ancienne_note(&ent, &w, anc.devoirs[0], b, 12.0, 20.0, "note").await;
    ancienne_note(&ent, &w, anc.devoirs[1], a, 7.0, 10.0, "note").await;
    ancienne_note(&ent, &w, anc.devoirs[1], b, 0.0, 10.0, "absent").await;
    ancienne_note(&ent, &w, anc.devoirs[1], c, 0.0, 10.0, "dispense").await;
    // Une note à quatre décimales : reprise à trois décimales (14,4996 → 14,500), écart listé. (Un simple
    // bruit binaire de DOUBLE, comme 14,499999999999998, se lit 14,5 sans écart.)
    let n_arrondi = ancienne_note(&ent, &w, anc.devoirs[2], a, 14.4996, 20.0, "note").await;
    // Une note au-delà du barème : reprise, signalée, ignorée par le moteur comme par l'ancien.
    let n_hors = ancienne_note(&ent, &w, anc.devoirs[3], c, 25.0, 20.0, "note").await;
    // Une note sans devoir (voie historique fermée) : listée, non reprise.
    let n_sans = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, sur, date_note, auteur_user_id, statut)
         VALUES ($1, $2, $3, $4, 9, 20, '2031-10-11', $5, 'note')",
    )
    .bind(n_sans)
    .bind(w.etab)
    .bind(a)
    .bind(w.maths)
    .bind(w.prof)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    // Une correction journalisée (12 → 13) : qui, quand, avant, après, motif sont conservés.
    exec(
        &ent,
        "UPDATE ent_note SET valeur = 13 WHERE id = $1",
        vec![u(n_corrigee)],
    )
    .await;
    exec(
        &ent,
        "INSERT INTO ent_note_correction (id, etablissement_id, note_id, eleve_id, periode_id, ancien_statut, ancienne_valeur,
                                          nouveau_statut, nouvelle_valeur, motif, corrige_par_user_id, corrige_le)
         VALUES ($1, $2, $3, $4, $5, 'note', 12, 'note', 13, 'oubli d''un point', $6, '2031-11-25T09:00:00Z')",
        vec![u(Uuid::new_v4()), u(w.etab), u(n_corrigee), u(b), u(anc.note_periode), u(w.direction)],
    )
    .await;
    // Un devoir sans service ni classe : refusé, jamais inventé.
    let orphelin = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_devoir (id, etablissement_id, periode_id, matiere_id, classe, titre, date_devoir, coef, sur)
         VALUES ($1, $2, $3, $4, '', 'Orphelin', '2031-11-06', 1, 20)",
        vec![u(orphelin), u(w.etab), u(anc.note_periode), u(w.maths)],
    )
    .await;
    // Une appréciation de matière et une générale ; règles et coefficient de l'ancien monde.
    exec(
        &ent,
        "INSERT INTO ent_note_appreciation (id, etablissement_id, periode_id, eleve_id, matiere_id, texte, auteur_user_id)
         VALUES ($1, $2, $3, $4, $5, 'Bon trimestre.', $6), ($7, $2, $3, $4, NULL, 'Élève sérieux.', $6)",
        vec![u(Uuid::new_v4()), u(w.etab), u(anc.note_periode), u(a), u(w.maths), u(w.prof), u(Uuid::new_v4())],
    )
    .await;
    exec(
        &ent,
        "INSERT INTO ent_eva_regle (etablissement_id, arrondi) VALUES ($1, 'dixieme')
         ON CONFLICT (etablissement_id) DO UPDATE SET arrondi = 'dixieme'",
        vec![u(w.etab)],
    )
    .await;
    exec(
        &ent,
        "INSERT INTO ent_eva_coef_matiere (id, etablissement_id, classe_id, matiere_id, coef) VALUES ($1, $2, $3, $4, 3)",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.classe_a), u(w.maths)],
    )
    .await;
    // Une période fermée : l'état de chaque classe de la périodisation devient « appréciations closes ».
    exec(
        &ent,
        "UPDATE ent_note_periode SET statut = 'fermee' WHERE id = $1",
        vec![u(anc.note_periode)],
    )
    .await;

    let rapport = ent
        .eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    for c in &rapport.comptes {
        assert_eq!(
            c.lues,
            c.creees + c.inchangees + c.refusees + c.ignorees,
            "invariant du rapport pour {}",
            c.objet
        );
    }
    let cpt = |o: &str| rapport.compte(o).unwrap_or_else(|| panic!("compte {o}"));
    assert_eq!(cpt("devoir").creees, 4, "quatre évaluations reprises");
    assert!(cpt("devoir").refusees >= 1, "le devoir orphelin est refusé");
    assert_eq!(
        cpt("note").creees,
        7,
        "sept résultats repris (la huitième note n'a pas de devoir)"
    );
    assert!(cpt("note_sans_devoir").ignorees >= 1);
    assert_eq!(cpt("correction").creees, 1);
    assert_eq!(cpt("appreciation").creees, 2);
    assert!(cpt("reglage").creees >= 1);
    assert_eq!(cpt("coefficient").creees, 1);
    assert!(
        cpt("periode").creees >= 2,
        "un état par classe de la périodisation (A et D)"
    );
    let a_la_ligne = |objet: &str, id: Uuid, code: &str| {
        rapport
            .lignes
            .iter()
            .any(|l| l.objet == objet && l.source_id == id && l.code == code)
    };
    assert!(a_la_ligne("devoir", orphelin, "public_introuvable"));
    assert!(a_la_ligne("note_sans_devoir", n_sans, "note_sans_devoir"));
    assert!(a_la_ligne("note", n_arrondi, "arrondi_millieme"));
    assert!(a_la_ligne("note", n_hors, "hors_bareme"));
    // Le rapport ne porte ni nom, ni note, ni texte d'appréciation.
    for l in &rapport.lignes {
        let d = l.detail.clone().unwrap_or_default();
        assert!(!d.contains("Abbott") && !d.contains("sérieux"), "{d}");
    }

    // Valeurs reprises : décimales exactes, jamais de dérive binaire.
    let reprise: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
        "SELECT id, statut, valeur::text FROM eva_resultat WHERE etablissement_id = $1 ORDER BY id",
    )
    .bind(w.etab)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let val = |id: Uuid| reprise.iter().find(|r| r.0 == id).unwrap().2.clone();
    assert_eq!(val(n_ok).as_deref(), Some("14.000"));
    assert_eq!(val(n_arrondi).as_deref(), Some("14.500"));
    // L'historique de la correction : 12 (origine) → 13 (correction motivée, par la direction).
    let h: Vec<(i32, Option<String>, Option<String>, bool, Option<String>, Option<Uuid>)> = sqlx::query_as(
        "SELECT version, valeur_avant::text, valeur_apres::text, correction, motif, auteur
           FROM eva_resultat_historique WHERE etablissement_id = $1 AND resultat_id = $2 ORDER BY version",
    )
    .bind(w.etab)
    .bind(n_corrigee)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(h.len(), 2);
    assert_eq!(
        (h[0].1.clone(), h[0].2.as_deref(), h[0].3),
        (None, Some("12.000"), false)
    );
    assert_eq!(
        (h[1].1.as_deref(), h[1].2.as_deref(), h[1].3),
        (Some("12.000"), Some("13.000"), true)
    );
    assert_eq!(
        (h[1].4.as_deref(), h[1].5),
        (Some("oubli d'un point"), Some(w.direction))
    );
    // États de période : « fermée » → « appréciations closes » pour les classes de la périodisation.
    let etats: Vec<(String,)> = sqlx::query_as(
        "SELECT etat FROM eva_periode_etat WHERE etablissement_id = $1 AND periode_id = $2",
    )
    .bind(w.etab)
    .bind(w.t1)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(etats.len(), 2, "classes A et D");
    assert!(etats.iter().all(|(e,)| e == "appreciations_closes"));
    // Règle d'arrondi et coefficient repris ; jamais « absent compte zéro ».
    let regles = ent.eva_v1_regles(w.etab, w.annee).await.unwrap();
    assert_eq!(
        regles.regles.affichage.precision,
        domain_eleve::evaluation_moteur::Precision::Dixieme
    );

    // Idempotence : une seconde reprise ne crée rien et ne perd rien.
    let encore = ent
        .eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    assert_ne!(encore.execution_id, rapport.execution_id);
    for objet in [
        "reglage",
        "coefficient",
        "devoir",
        "note",
        "correction",
        "appreciation",
        "periode",
    ] {
        let c = encore.compte(objet).unwrap();
        assert_eq!(c.creees, 0, "{objet} : rien de recréé");
        assert_eq!(c.lues, c.creees + c.inchangees + c.refusees + c.ignorees);
    }
    assert!(encore.compte("devoir").unwrap().inchangees >= 4);
    assert!(encore.compte("note").unwrap().inchangees >= 7);
    // L'ancienne table n'a pas bougé (rien n'est supprimé ni modifié).
    let (n_anc,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_note WHERE etablissement_id = $1 AND eleve_id = ANY($2)",
    )
    .bind(w.etab)
    .bind(&w.eleves_a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n_anc, 8);
    // Dérive : l'ancien écrivain change une note après la reprise → listée, jamais écrasée.
    exec(
        &ent,
        "UPDATE ent_note SET valeur = 16 WHERE id = $1",
        vec![u(n_ok)],
    )
    .await;
    let derive = ent
        .eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    assert!(derive
        .lignes
        .iter()
        .any(|l| l.source_id == n_ok && l.code == "derive_depuis_la_reprise"));
    assert_eq!(val(n_ok).as_deref(), Some("14.000"));
    // Droit : un professeur ne lance pas la reprise.
    assert!(matches!(
        ent.eva_v1_reprendre_for_actor(w.etab, w.prof, PROF).await,
        Err(EntDbError::Forbidden)
    ));
}

/// Jeu « historique » riche : maths (4 devoirs) et français (3 devoirs), trois élèves, statuts variés,
/// coefficients de matière par classe. Rend l'identifiant de la période de notes.
async fn ancien_riche(ent: &PgEnt, w: &W) -> Ancien {
    let anc = ancien_monde(ent, w).await;
    let service_fr: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(w.etab)
    .bind(w.cours_fr)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let classe_code: String =
        sqlx::query_scalar("SELECT code FROM ent_classe WHERE etablissement_id = $1 AND id = $2")
            .bind(w.etab)
            .bind(w.classe_a)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    let mut dev_fr = Vec::new();
    for (titre, sur, coef) in [
        ("Dictée", 20.0_f64, 1.0_f64),
        ("Rédaction", 40.0, 2.0),
        ("Oral", 10.0, 1.5),
    ] {
        let dv = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_devoir (id, etablissement_id, periode_id, matiere_id, classe, classe_id, service_id, titre,
                                     date_devoir, coef, sur, created_by_user_id, publication_explicite)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, '2031-11-10', $9, $10, $11, false)",
        )
        .bind(dv)
        .bind(w.etab)
        .bind(anc.note_periode)
        .bind(w.francais)
        .bind(&classe_code)
        .bind(w.classe_a)
        .bind(service_fr)
        .bind(titre)
        .bind(coef)
        .bind(sur)
        .bind(w.prof)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        dev_fr.push(dv);
    }
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    // Maths : DS 1 (20, coef 2), Interro (10), DM (20), Hors service (20).
    let m = &anc.devoirs;
    for (dv, e, v, sur, st) in [
        (m[0], a, 14.0, 20.0, "note"),
        (m[0], b, 8.5, 20.0, "note"),
        (m[0], c, 0.0, 20.0, "absent"),
        (m[1], a, 7.0, 10.0, "note"),
        (m[1], b, 9.0, 10.0, "note"),
        (m[1], c, 6.5, 10.0, "note"),
        (m[2], a, 14.499_999_999_999_998, 20.0, "note"),
        (m[2], b, 0.0, 20.0, "a_rattraper"),
        (m[2], c, 11.25, 20.0, "note"),
        (m[3], a, 12.0, 20.0, "note"),
        (m[3], b, 25.0, 20.0, "note"),
        (m[3], c, 0.0, 20.0, "non_note"),
        // Français : une matière sur trois barèmes (20, 40, 10), un élève dispensé de tout.
        (dev_fr[0], a, 13.0, 20.0, "note"),
        (dev_fr[0], b, 10.0, 20.0, "note"),
        (dev_fr[0], c, 0.0, 20.0, "dispense"),
        (dev_fr[1], a, 31.0, 40.0, "note"),
        (dev_fr[1], b, 22.5, 40.0, "note"),
        (dev_fr[1], c, 0.0, 40.0, "dispense"),
        (dev_fr[2], a, 8.0, 10.0, "note"),
        (dev_fr[2], b, 4.5, 10.0, "note"),
        (dev_fr[2], c, 0.0, 10.0, "dispense"),
    ] {
        let matiere = if dev_fr.contains(&dv) {
            w.francais
        } else {
            w.maths
        };
        sqlx::query(
            "INSERT INTO ent_note (id, etablissement_id, eleve_id, matiere_id, valeur, sur, date_note, auteur_user_id, devoir_id, statut)
             VALUES ($1, $2, $3, $4, $5, $6, '2031-11-10', $7, $8, $9)",
        )
        .bind(Uuid::new_v4())
        .bind(w.etab)
        .bind(e)
        .bind(matiere)
        .bind(v)
        .bind(sur)
        .bind(w.prof)
        .bind(dv)
        .bind(st)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    }
    exec(
        ent,
        "INSERT INTO ent_eva_coef_matiere (id, etablissement_id, classe_id, matiere_id, coef) VALUES ($1, $2, $3, $4, 3), ($5, $2, $3, $6, 2)",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.classe_a), u(w.maths), u(Uuid::new_v4()), u(w.francais)],
    )
    .await;
    exec(
        ent,
        "INSERT INTO ent_note_appreciation (id, etablissement_id, periode_id, eleve_id, matiere_id, texte, auteur_user_id)
         VALUES ($1, $2, $3, $4, $5, 'Bon trimestre.', $6)",
        vec![u(Uuid::new_v4()), u(w.etab), u(anc.note_periode), u(a), u(w.maths), u(w.prof)],
    )
    .await;
    anc
}

/// Le test d'or : les moyennes tombent à l'identique avant et après la reprise (§22, M2). « Avant » :
/// le lecteur historique (`eva_calculer_classe`, moteur de V0) sur les anciennes tables ; « après » :
/// le moteur étendu de V2 sur `eva_*`.
#[tokio::test]
async fn test_d_or_les_moyennes_sont_identiques_avant_et_apres_la_reprise() {
    use domain_eleve::evaluation::Arrondi;
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let anc = ancien_riche(&ent, &w).await;
    let avant = ent
        .eva_calculer_classe(w.etab, w.classe_a, anc.note_periode, false)
        .await
        .unwrap();
    let rapport = ent
        .eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    assert!(rapport.compte("note").unwrap().creees >= 21);
    let mut compares = 0;
    for &e in &w.eleves_a {
        let apres = ent
            .eva_v1_moyennes_eleve_periode(w.etab, e, w.t1, false)
            .await
            .unwrap();
        for m in [w.maths, w.francais] {
            let ancienne = avant
                .moyennes
                .get(&(e, m))
                .map(|x| (x.arrondie(Arrondi::Centieme) * 100.0).round() as i64);
            let nouvelle = apres
                .matieres
                .iter()
                .find(|x| x.matiere_id == m)
                .and_then(|x| x.moyenne.affichee_m)
                .map(|x| x / 10);
            assert_eq!(
                ancienne, nouvelle,
                "moyenne de matière, élève {e}, matière {m}"
            );
            if ancienne.is_some() {
                compares += 1;
            }
        }
        let g_anc = avant
            .generales
            .get(&e)
            .map(|x| (x.arrondie(Arrondi::Centieme) * 100.0).round() as i64);
        let g_nouv = apres.generale.affichee_m.map(|x| x / 10);
        assert_eq!(g_anc, g_nouv, "moyenne générale de {e}");
        // Les coefficients de matière repris sont ceux de l'ancienne table (3 et 2).
        let coefs: Vec<i64> = apres.matieres.iter().map(|x| x.coef_m).collect();
        assert!(coefs.contains(&3000) && coefs.contains(&2000), "{coefs:?}");
    }
    assert!(
        compares >= 5,
        "le jeu doit produire des moyennes à comparer ({compares})"
    );
    // Exemple lisible : Hannah, maths = (14×2 + 14×1 + 14,5×1 + 12×1) / 5.
    let hannah = ent
        .eva_v1_moyennes_eleve_periode(w.etab, w.eleves_a[0], w.t1, false)
        .await
        .unwrap();
    let maths = hannah
        .matieres
        .iter()
        .find(|x| x.matiere_id == w.maths)
        .unwrap();
    assert_eq!(maths.moyenne.nb_comptees(), 4);
    assert_eq!(maths.moyenne.affichee_m, Some(13_700));
    // Et l'explication dit pourquoi, ligne à ligne.
    let texte = maths.moyenne.explication_fr(&hannah.regles);
    assert_eq!(texte.last().unwrap(), "Moyenne : 13,7 sur 20");
    assert!(texte
        .iter()
        .any(|l| l.contains("Interro") && l.contains("7 sur 10") && l.contains("soit 14 sur 20")));
}

#[tokio::test]
async fn vues_de_compatibilite_rendent_la_forme_des_anciennes_tables() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let anc = ancien_riche(&ent, &w).await;
    exec(
        &ent,
        "UPDATE ent_note_periode SET statut = 'partielle_appreciations' WHERE id = $1",
        vec![u(anc.note_periode)],
    )
    .await;
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    let p = ent.pool_verif();
    // Résultats : mêmes identifiants, élèves, évaluations, statuts, valeurs (à trois décimales).
    let ancien: Vec<(Uuid, Uuid, Uuid, String, String)> = sqlx::query_as(
        "SELECT id, eleve_id, devoir_id, statut, round(valeur::numeric, 3)::text FROM ent_note
          WHERE etablissement_id = $1 AND devoir_id IS NOT NULL AND eleve_id = ANY($2) ORDER BY id",
    )
    .bind(w.etab)
    .bind(&w.eleves_a)
    .fetch_all(p)
    .await
    .unwrap();
    let vue: Vec<(Uuid, Uuid, Uuid, String, String)> = sqlx::query_as(
        "SELECT id, eleve_id, devoir_id, statut, round(valeur::numeric, 3)::text FROM ent_note_v
          WHERE etablissement_id = $1 AND eleve_id = ANY($2) ORDER BY id",
    )
    .bind(w.etab)
    .bind(&w.eleves_a)
    .fetch_all(p)
    .await
    .unwrap();
    assert_eq!(ancien.len(), 21);
    assert_eq!(ancien, vue);
    // Évaluations : titre, barème, coefficient, date, matière, et surtout la VISIBILITÉ (règle ancienne :
    // visible si la publication n'est pas explicite ou si elle a eu lieu).
    let ancien_dv: Vec<(Uuid, String, f64, f64, NaiveDate, Uuid, bool)> = sqlx::query_as(
        "SELECT id, titre, coef, sur, date_devoir, matiere_id, (NOT publication_explicite OR publie_le IS NOT NULL)
           FROM ent_devoir WHERE etablissement_id = $1 AND classe_id = $2 ORDER BY id",
    )
    .bind(w.etab)
    .bind(w.classe_a)
    .fetch_all(p)
    .await
    .unwrap();
    let vue_dv: Vec<(Uuid, String, f64, f64, NaiveDate, Uuid, bool)> = sqlx::query_as(
        "SELECT id, titre, coef, sur, date_devoir, matiere_id, (NOT publication_explicite OR publie_le IS NOT NULL)
           FROM ent_devoir_v WHERE etablissement_id = $1 AND classe_id = $2 ORDER BY id",
    )
    .bind(w.etab)
    .bind(w.classe_a)
    .fetch_all(p)
    .await
    .unwrap();
    assert_eq!(ancien_dv, vue_dv);
    // Période : le statut ancien est relu depuis les états par classe.
    let (statut_vue,): (String,) =
        sqlx::query_as("SELECT statut FROM ent_note_periode_v WHERE id = $1")
            .bind(anc.note_periode)
            .fetch_one(p)
            .await
            .unwrap();
    assert_eq!(statut_vue, "partielle_appreciations");
    let (cloturees,): (bool,) =
        sqlx::query_as("SELECT appreciations_closes FROM ent_note_periode_v WHERE id = $1")
            .bind(anc.note_periode)
            .fetch_one(p)
            .await
            .unwrap();
    assert!(!cloturees, "notes arrêtées seulement : appréciations pas closes");
    // Appréciation (M5) : validée, mais non montrée tant que l'état de la période ne clôt pas les appréciations.
    let (cachees,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_note_appreciation_v WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(w.etab)
    .bind(w.eleves_a[0])
    .fetch_one(p)
    .await
    .unwrap();
    assert_eq!(cachees, 0, "appréciation validée mais période pas close : cachée");
    for classe in [w.classe_a, w.classe_d] {
        ent.eva_v1_changer_etat_periode_for_actor(
            w.etab,
            w.direction,
            DIR,
            w.t1,
            classe,
            "appreciations_closes",
            None,
            None,
        )
        .await
        .unwrap();
    }
    let (statut_vue,): (String,) =
        sqlx::query_as("SELECT statut FROM ent_note_periode_v WHERE id = $1")
            .bind(anc.note_periode)
            .fetch_one(p)
            .await
            .unwrap();
    assert_eq!(statut_vue, "fermee");
    let (texte, matiere): (String, Option<Uuid>) =
        sqlx::query_as("SELECT texte, matiere_id FROM ent_note_appreciation_v WHERE etablissement_id = $1 AND eleve_id = $2")
            .bind(w.etab)
            .bind(w.eleves_a[0])
            .fetch_one(p)
            .await
            .unwrap();
    assert_eq!((texte.as_str(), matiere), ("Bon trimestre.", Some(w.maths)));
    // Publication intégrée à `ent_note_v` (M5) : une évaluation dépubliée disparaît de la vue.
    let (avant,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_note_v WHERE etablissement_id = $1")
        .bind(w.etab)
        .fetch_one(p)
        .await
        .unwrap();
    exec(
        &ent,
        "UPDATE eva_evaluation SET publiee_le = NULL WHERE id = $1",
        vec![u(anc.devoirs[0])],
    )
    .await;
    let (apres,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_note_v WHERE etablissement_id = $1")
        .bind(w.etab)
        .fetch_one(p)
        .await
        .unwrap();
    assert!(apres < avant, "les notes d'une évaluation non publiée ne sont plus dans la vue");
    // Les vues sont en lecture seule.
    assert!(
        sqlx::query("DELETE FROM ent_note_v WHERE etablissement_id = $1")
            .bind(w.etab)
            .execute(p)
            .await
            .is_err()
    );
}
