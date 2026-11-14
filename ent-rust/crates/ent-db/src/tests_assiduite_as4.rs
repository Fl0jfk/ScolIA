//! Socle 12, lot AS4 (décompte versionné). Réf. : `docs/socles/12_ASSIDUITE.md` §13.1, §13.2, §22.6, R-AS15 à R-AS17.
//! Critères de fin : « tout recalculer ne change rien » ; un changement de règle en janvier laisse septembre
//! inchangé (règle datée, jamais rétroactive) ; des vacances déclarées donnent 0 demi-journée.
//! Chaque test travaille dans un établissement neuf (jeu de labo) et ne vérifie que ses propres lignes ; les
//! jours « passés » sont des **dates fixes** insérées directement (aucune dépendance à l'heure réelle).
//! Sans `ENT_DATABASE_URL` : ignorés par absence de base (le test échoue en CI, qui fournit Postgres).

use super::*;
use crate::ref_commun::Acteur;
use crate::structure_calendrier::EvenementSaisie;
use chrono::{Duration, NaiveDate, NaiveTime};
use domain_eleve::DeclareAbsence;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn d(a: i32, m: u32, j: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, j).unwrap()
}

fn interdit<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

async fn aujourdhui(ent: &PgEnt, cast: &LaboCast) -> NaiveDate {
    ent.aujourdhui_etablissement(cast.etablissement_id)
        .await
        .unwrap()
}

async fn nombre(ent: &PgEnt, sql: &str, a: Uuid, b: Uuid) -> i64 {
    let (c,): (i64,) = sqlx::query_as(sql)
        .bind(a)
        .bind(b)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    c
}

fn saisie(etat: &str) -> SaisieLigne {
    SaisieLigne {
        etat: etat.into(),
        minutes: None,
        lieu_code: None,
    }
}

/// Appel de 4B sur `[debut, fin)` : les `absents` sont absents, les autres présents.
async fn appel(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    debut: (u32, u32),
    fin: (u32, u32),
    absents: &[Uuid],
) {
    let etab = cast.etablissement_id;
    let c = ent
        .seed_creneau_du_jour(
            etab,
            "4B",
            "AS4",
            cast.prof.user_id,
            jour,
            t(debut.0, debut.1),
            t(fin.0, fin.1),
            None,
        )
        .await
        .expect("créneau")
        .id;
    let (a, _) = ent
        .ouvrir_appel(etab, c, jour, cast.prof.user_id, "prof")
        .await
        .expect("ouvrir");
    for e in absents {
        ent.pointer_ligne_v2(
            etab,
            a.id,
            *e,
            cast.prof.user_id,
            "prof",
            saisie("absent"),
            "web",
        )
        .await
        .expect("pointer");
    }
    ent.valider_appel_v2(
        etab,
        a.id,
        cast.prof.user_id,
        "prof",
        ValidationAppel {
            confirme_non_pointes: false,
            canal: None,
        },
    )
    .await
    .expect("valider");
}

fn lea(cast: &LaboCast) -> Uuid {
    cast.eleve_lea_id
}

fn qualifier(q: &str, version: i32) -> QualifierEpisode {
    QualifierEpisode {
        qualification: q.into(),
        version,
        nature_code: None,
        motif_id: None,
        libelle_famille_code: None,
        note_interne: None,
        proposition_id: None,
    }
}

/// Épisode inséré directement pour une date fixe passée : l'établissement de labo ne rejoue pas l'histoire.
async fn episode_sql(
    ent: &PgEnt,
    cast: &LaboCast,
    eleve: Uuid,
    jour: NaiveDate,
    debut: NaiveTime,
    fin: NaiveTime,
    qualification: &str,
) -> Uuid {
    let etab = cast.etablissement_id;
    let id = Uuid::new_v4();
    let etat = if qualification == "en_attente" {
        "a_qualifier"
    } else {
        "qualifie"
    };
    sqlx::query(
        "INSERT INTO ent_asd_absence (id, etablissement_id, eleve_id, jour_debut, jour_fin, etat, qualification, source)
         VALUES ($1,$2,$3,$4,$4,$5,$6,'reprise')",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve)
    .bind(jour)
    .bind(etat)
    .bind(qualification)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_asd_absence_seance (id, etablissement_id, absence_id, eleve_id, jour, kind, heure_debut, heure_fin, source)
         VALUES ($1,$2,$3,$4,$5,'seance',$6,$7,'reprise')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(id)
    .bind(eleve)
    .bind(jour)
    .bind(debut)
    .bind(fin)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

/// Année scolaire de labo (hors de l'année courante du jeu de base) pour fixer le calendrier d'un test.
async fn annee_fixe(
    ent: &PgEnt,
    cast: &LaboCast,
    libelle: &str,
    du: NaiveDate,
    au: NaiveDate,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
         VALUES ($1,$2,$3,$4,$5,false)",
    )
    .bind(id)
    .bind(cast.etablissement_id)
    .bind(libelle)
    .bind(du)
    .bind(au)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

async fn regle_sql(
    ent: &PgEnt,
    cast: &LaboCast,
    valide_du: NaiveDate,
    definition: &str,
    seuil: i32,
    seuil_signalement: i32,
) {
    sqlx::query(
        "INSERT INTO ent_asd_regle (id, etablissement_id, valide_du, definition_demi_journee, seuil_minutes,
                                    seuil_signalement_demi_journees)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(cast.etablissement_id)
    .bind(valide_du)
    .bind(definition)
    .bind(seuil)
    .bind(seuil_signalement)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

async fn calcule(
    ent: &PgEnt,
    cast: &LaboCast,
    eleve: Uuid,
    du: NaiveDate,
    au: NaiveDate,
) -> Vec<DemiJourneeCalculee> {
    ent.calculer_demi_journees(cast.etablissement_id, Some(eleve), du, au)
        .await
        .expect("calcul")
}

fn comptees(v: &[DemiJourneeCalculee]) -> (usize, usize) {
    (
        v.iter().filter(|x| x.compte).count(),
        v.iter().filter(|x| x.non_justifiee).count(),
    )
}

async fn instantane(ent: &PgEnt, cast: &LaboCast, eleve: Uuid) -> Vec<String> {
    let l: Vec<(String,)> = sqlx::query_as(
        "SELECT d::text FROM ent_asd_demi_journee d WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY jour, periode",
    )
    .bind(cast.etablissement_id)
    .bind(eleve)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    l.into_iter().map(|(s,)| s).collect()
}

/// Les élèves de labo entrent à la rentrée courante : pour rejouer des dates passées, la scolarisation
/// remonte (hors garde du déclencheur, par `session_replication_role`, comme un import d'historique).
async fn scolarises_depuis(ent: &PgEnt, cast: &LaboCast, depuis: NaiveDate) {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_eleve_scolarisation SET date_entree = $2 WHERE etablissement_id = $1")
        .bind(cast.etablissement_id)
        .bind(depuis)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

/// L'appel ne s'ouvre que pour le jour métier courant : les parcours « appel » se jouent aujourd'hui, quel
/// que soit le jour de la semaine où le test tourne. Le calendrier est donc rendu ouvert tous les jours
/// (profil de l'établissement) et couvert par une année de labo qui contient aujourd'hui.
async fn monde_du_jour(ent: &PgEnt, cast: &LaboCast) -> NaiveDate {
    let jour = aujourdhui(ent, cast).await;
    annee_fixe(
        ent,
        cast,
        "AS4-du-jour",
        jour - Duration::days(30),
        jour + Duration::days(30),
    )
    .await;
    scolarises_depuis(ent, cast, jour - Duration::days(400)).await;
    let tous = serde_json::json!({
        "lun": ["matin", "apres_midi"], "mar": ["matin", "apres_midi"], "mer": ["matin", "apres_midi"],
        "jeu": ["matin", "apres_midi"], "ven": ["matin", "apres_midi"], "sam": ["matin", "apres_midi"],
        "dim": ["matin", "apres_midi"]
    });
    ent.structure_profil_definir(
        &dir(cast),
        crate::structure_etab::ProfilSaisie {
            nom_officiel: "Collège de labo AS4".into(),
            nom_usage: None,
            logo_document_id: None,
            uai: None,
            secteur: "public".into(),
            contrat_type: None,
            degres: vec!["second".into()],
            types: vec!["college".into()],
            academie_code: None,
            departement_code: Some("75".into()),
            zone_calendrier: None,
            regime_local: None,
            tutelle_libelle: None,
            adresse: None,
            telephone: None,
            courriel_contact: None,
            jours_ouvres: Some(tous),
            version_attendue: None,
        },
    )
    .await
    .expect("profil de l'établissement");
    jour
}

fn dir(cast: &LaboCast) -> Acteur<'static> {
    Acteur {
        etablissement_id: cast.etablissement_id,
        user_id: cast.direction.user_id,
        role: "direction",
    }
}

// ── Critère : le décompte lit les épisodes (limite laissée par AS3) ─────────────────────────────

#[tokio::test]
async fn as4_jour_mixte_chaque_episode_garde_son_etat() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let jour = monde_du_jour(&ent, &cast).await;
    let lea = lea(&cast);
    // Absent 8 h–9 h, présent 9 h–10 h, absent 10 h–10 h 20 : deux épisodes dans la même demi-journée.
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    appel(&ent, &cast, jour, (9, 0), (10, 0), &[]).await;
    appel(&ent, &cast, jour, (10, 0), (10, 20), &[lea]).await;
    let eps = ent
        .episodes_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            "cpe",
            FiltreEpisodes {
                eleve_id: Some(lea),
                etat: Some("tous".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(eps.len(), 2);
    let long = eps
        .iter()
        .find(|e| e.seances.iter().any(|s| s.heure_fin == Some(t(9, 0))))
        .expect("épisode 8 h");
    let court = eps.iter().find(|e| e.id != long.id).expect("épisode 10 h");
    // Avant toute qualification : 80 minutes, 80 non justifiées → la demi-journée compte au décompte.
    let v = calcule(&ent, &cast, lea, jour, jour).await;
    assert_eq!(v.len(), 1);
    assert_eq!(
        (
            v[0].minutes_manquees,
            v[0].minutes_non_justifiees,
            v[0].non_justifiee
        ),
        (80, 80, true)
    );
    // L'épisode de 8 h est excusé, celui de 10 h reste en attente : seules 20 minutes restent non justifiées,
    // sous le seuil de 30 : la demi-journée compte mais n'est plus « non justifiée ». L'ancien statut projeté
    // aurait dit « refusée » pour tout le jour et compté la demi-journée au décompte réglementaire.
    ent.qualifier_episode_for_actor(
        cast.etablissement_id,
        cast.cpe.user_id,
        "cpe",
        long.id,
        qualifier("excuse_valable", long.version),
    )
    .await
    .unwrap();
    let v = calcule(&ent, &cast, lea, jour, jour).await;
    assert_eq!(
        (v[0].minutes_manquees, v[0].minutes_non_justifiees),
        (80, 20)
    );
    assert!(
        v[0].compte && !v[0].non_justifiee,
        "20 min non justifiées < 30 : comptée, pas au décompte réglementaire"
    );
    let legacy: (String,) = sqlx::query_as("SELECT statut FROM ent_absence WHERE etablissement_id = $1 AND eleve_id = $2 AND debut = $3")
        .bind(cast.etablissement_id)
        .bind(lea)
        .bind(jour)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_ne!(
        legacy.0, "justifiee",
        "l'ancien statut ne dit pas la même chose : il n'est plus la source"
    );
    // Les deux épisodes valables : comptée, justifiée.
    ent.qualifier_episode_for_actor(
        cast.etablissement_id,
        cast.cpe.user_id,
        "cpe",
        court.id,
        qualifier("non_valable", court.version),
    )
    .await
    .unwrap();
    let v = calcule(&ent, &cast, lea, jour, jour).await;
    assert!(
        v[0].compte && !v[0].non_justifiee,
        "épisode de 10 h non valable, mais 20 min < seuil"
    );
    // La lecture publique (API de la vie scolaire) suit les épisodes, pas l'ancien statut ; le jour du test
    // est le seul jour que ce test regarde (le jeu de labo porte d'autres absences dans le mois).
    let lu = ent
        .demi_journees_eleve_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            "cpe",
            lea,
            jour,
            jour,
        )
        .await
        .unwrap();
    assert_eq!(lu.len(), 1);
    assert!(lu[0].comptee && !lu[0].non_justifiee && lu[0].raison == "compte");
    assert!(
        lu[0].explication.contains("compte, justifiée"),
        "{}",
        lu[0].explication
    );
    let mois = ent
        .decompte_mensuel_for_actor(cast.etablissement_id, cast.cpe.user_id, "cpe", jour)
        .await
        .unwrap();
    assert!(!mois.arrete);
}

// ── Critère : tout recalculer ne change rien ────────────────────────────────────────────────────

#[tokio::test]
async fn as4_recalculer_tout_ne_change_rien() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (lea, hermione) = (lea(&cast), cast.eleve_tom_id);
    annee_fixe(&ent, &cast, "AS4-2024", d(2024, 9, 1), d(2025, 7, 4)).await;
    scolarises_depuis(&ent, &cast, d(2020, 1, 1)).await;
    for (e, j, a, b, q) in [
        (lea, d(2024, 10, 8), t(8, 0), t(9, 0), "non_valable"),
        (lea, d(2024, 10, 10), t(13, 30), t(15, 0), "excuse_valable"),
        (hermione, d(2024, 10, 8), t(8, 0), t(8, 20), "en_attente"),
        (
            hermione,
            d(2024, 10, 15),
            t(11, 30),
            t(12, 30),
            "non_valable",
        ), // à cheval sur la coupure
    ] {
        episode_sql(&ent, &cast, e, j, a, b, q).await;
    }
    let (du, au) = (d(2024, 10, 1), d(2024, 10, 31));
    let r1 = ent
        .recalculer_decompte(etab, None, du, au, "recalcul", None)
        .await
        .unwrap();
    assert_eq!(
        r1.lignes_total, 5,
        "Harry 2 demi-journées, Draco 3 (la séance à cheval en fait deux)"
    );
    assert!(r1.lignes_modifiees >= 5);
    let (compte_l,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_asd_demi_journee WHERE etablissement_id = $1 AND eleve_id = $2 AND compte")
        .bind(etab)
        .bind(lea)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(
        compte_l, 2,
        "les deux demi-journées de Harry comptent (calendrier ouvert, élève scolarisé)"
    );
    let (avant_l, avant_h) = (
        instantane(&ent, &cast, lea).await,
        instantane(&ent, &cast, hermione).await,
    );
    assert_eq!((avant_l.len(), avant_h.len()), (2, 3));
    // Rejouer le calcul, plusieurs fois : aucune ligne écrite, aucune ligne changée (même `calcule_le`).
    for _ in 0..3 {
        let r = ent
            .recalculer_decompte(etab, None, du, au, "recalcul", None)
            .await
            .unwrap();
        assert_eq!(
            (r.lignes_modifiees, r.lignes_supprimees, r.lignes_total),
            (0, 0, 5)
        );
        assert_eq!(r.rectifications, 0);
    }
    assert_eq!(instantane(&ent, &cast, lea).await, avant_l);
    assert_eq!(instantane(&ent, &cast, hermione).await, avant_h);
    // Un seul élève recalculé seul : mêmes lignes (le recalcul d'un élève ne touche pas les autres).
    let r = ent
        .recalculer_decompte(etab, Some(lea), du, au, "recalcul", None)
        .await
        .unwrap();
    assert_eq!((r.lignes_modifiees, r.lignes_supprimees), (0, 0));
    assert_eq!(instantane(&ent, &cast, hermione).await, avant_h);
    // Une requalification change vraiment le résultat : alors seulement les lignes bougent, et le retour à
    // l'état initial redonne exactement le même instantané.
    sqlx::query("UPDATE ent_asd_absence SET qualification = 'excuse_valable' WHERE etablissement_id = $1 AND eleve_id = $2 AND jour_debut = $3")
        .bind(etab)
        .bind(lea)
        .bind(d(2024, 10, 8))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let r = ent
        .recalculer_decompte(etab, None, du, au, "recalcul", None)
        .await
        .unwrap();
    assert_eq!(r.lignes_modifiees, 1);
    sqlx::query("UPDATE ent_asd_absence SET qualification = 'non_valable' WHERE etablissement_id = $1 AND eleve_id = $2 AND jour_debut = $3")
        .bind(etab)
        .bind(lea)
        .bind(d(2024, 10, 8))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    ent.recalculer_decompte(etab, None, du, au, "recalcul", None)
        .await
        .unwrap();
    let apres: Vec<String> = instantane(&ent, &cast, lea).await;
    assert_eq!(apres.len(), avant_l.len());
}

// ── Critère : règle datée, jamais rétroactive ───────────────────────────────────────────────────

#[tokio::test]
async fn as4_changement_de_regle_en_janvier_laisse_septembre_inchange() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = lea(&cast);
    annee_fixe(&ent, &cast, "AS4-2025", d(2025, 9, 1), d(2026, 7, 4)).await;
    scolarises_depuis(&ent, &cast, d(2020, 1, 1)).await;
    // Septembre : 30 minutes ; à partir du 5 janvier : « toute la demi-journée » ou 120 minutes.
    regle_sql(&ent, &cast, d(2025, 9, 1), "minutes", 30, 4).await;
    regle_sql(&ent, &cast, d(2026, 1, 5), "minutes", 120, 6).await;
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2025, 9, 9),
        t(8, 0),
        t(8, 45),
        "non_valable",
    )
    .await;
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2026, 1, 13),
        t(8, 0),
        t(8, 45),
        "non_valable",
    )
    .await;
    // La même absence de 45 minutes compte en septembre (45 ≥ 30) et pas en janvier (45 < 120).
    let sept = calcule(&ent, &cast, lea, d(2025, 9, 1), d(2025, 9, 30)).await;
    let janv = calcule(&ent, &cast, lea, d(2026, 1, 1), d(2026, 1, 31)).await;
    assert_eq!(comptees(&sept), (1, 1));
    assert_eq!(comptees(&janv), (0, 0));
    assert_eq!(
        sept[0].regle_valide_du,
        d(2025, 9, 1),
        "la règle appliquée est celle DU JOUR"
    );
    assert_eq!(janv[0].regle_valide_du, d(2026, 1, 5));
    assert_eq!(janv[0].raison, "sous_le_seuil");
    // Poser (ou avoir posé) la règle de janvier ne change pas septembre, ni après un recalcul complet.
    ent.recalculer_decompte(etab, None, d(2025, 9, 1), d(2026, 1, 31), "recalcul", None)
        .await
        .unwrap();
    let sept_apres = calcule(&ent, &cast, lea, d(2025, 9, 1), d(2025, 9, 30)).await;
    assert_eq!(sept_apres, sept);
    // Arrêt de septembre (le jour métier de labo est postérieur) : le décompte est figé avec la règle de septembre.
    let arret = ent
        .arreter_mois_for_actor(etab, cast.direction.user_id, "direction", d(2025, 9, 1))
        .await
        .unwrap();
    assert!(arret.arrete_maintenant);
    let ligne = arret
        .eleves
        .iter()
        .find(|e| e.eleve_id == lea)
        .expect("Harry");
    assert_eq!(
        (
            ligne.demi_journees,
            ligne.non_justifiees,
            ligne.seuil_signalement
        ),
        (1, 1, 4)
    );
    assert_eq!(ligne.regle_valide_du, d(2025, 9, 1));
    // La règle de septembre est figée : plus aucune modification, même directe (déclencheur SQL).
    let figee = sqlx::query(
        "UPDATE ent_asd_regle SET seuil_minutes = 1 WHERE etablissement_id = $1 AND valide_du = $2",
    )
    .bind(etab)
    .bind(d(2025, 9, 1))
    .execute(ent.pool_verif())
    .await;
    assert!(figee.is_err(), "règle figée par un mois arrêté");
    // La règle de janvier n'est pas figée : aucun mois arrêté ne l'a utilisée.
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_asd_regle WHERE etablissement_id = $1 AND figee_le IS NULL",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1);
    // Le décompte lu de septembre est le décompte figé ; celui de janvier reste vivant.
    let (cpe, role) = (cast.cpe.user_id, "cpe");
    let m = ent
        .decompte_mensuel_for_actor(etab, cpe, role, d(2025, 9, 15))
        .await
        .unwrap();
    assert!(m.arrete && m.seuil_signalement == 4);
    let m = ent
        .decompte_mensuel_for_actor(etab, cpe, role, d(2026, 1, 15))
        .await
        .unwrap();
    assert!(!m.arrete && m.seuil_signalement == 6);
}

// ── Critère : des vacances déclarées donnent 0 demi-journée ─────────────────────────────────────

#[tokio::test]
async fn as4_vacances_declarees_donnent_zero_demi_journee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = lea(&cast);
    let annee = annee_fixe(&ent, &cast, "AS4-2023", d(2023, 9, 1), d(2024, 7, 4)).await;
    scolarises_depuis(&ent, &cast, d(2020, 1, 1)).await;
    // Vacances de la Toussaint déclarées par la direction (calendrier du socle 5).
    ent.structure_calendrier_evenement_creer(
        &dir(&cast),
        EvenementSaisie {
            annee_id: annee,
            type_evenement: "vacances".into(),
            libelle: "Toussaint".into(),
            du: d(2023, 10, 23),
            au: d(2023, 11, 3),
            du_demi: None,
            au_demi: None,
            effet: None,
            cibles: vec![],
            publie_familles: false,
            publie_personnels: false,
            motif: None,
            jour_reference: None,
        },
    )
    .await
    .expect("vacances");
    // Une absence « déclarée » pendant les vacances (ancien circuit, à la journée) et une séance isolée.
    let decl = ent
        .declare_absence(DeclareAbsence {
            etablissement_id: etab,
            eleve_id: lea,
            debut: d(2023, 10, 23),
            fin: d(2023, 10, 27),
            motif: "maladie".into(),
            actor_user_id: None,
        })
        .await
        .unwrap();
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2023, 10, 30),
        t(8, 0),
        t(10, 0),
        "non_valable",
    )
    .await;
    // Un jour hors vacances, pour contrôle : le mardi 17 octobre compte.
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2023, 10, 17),
        t(8, 0),
        t(10, 0),
        "non_valable",
    )
    .await;
    let v = calcule(&ent, &cast, lea, d(2023, 10, 1), d(2023, 11, 30)).await;
    let vacances: Vec<_> = v
        .iter()
        .filter(|x| x.jour >= d(2023, 10, 23) && x.jour <= d(2023, 11, 3))
        .collect();
    assert!(
        !vacances.is_empty(),
        "les absences déclarées en vacances sont lues, avec leur raison"
    );
    assert!(vacances
        .iter()
        .all(|x| !x.compte && !x.non_justifiee && x.raison == "sans_cours"));
    assert_eq!(
        comptees(&v),
        (1, 1),
        "seule la demi-journée du 17 octobre compte"
    );
    // L'ancienne reprise à la journée (DA-14) a donné 5 jours × 2 demi-journées : toutes à 0.
    assert_eq!(
        vacances
            .iter()
            .filter(|x| x.jour <= d(2023, 10, 27))
            .count(),
        10
    );
    let _ = decl;
    // Explication : la raison est lisible.
    let e = expliquer(vacances[0]);
    assert!(e.contains("aucun cours selon le calendrier"), "{e}");
    // Un mercredi après-midi sans cours (profil de l'établissement) : le matin compte, pas l'après-midi.
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2023, 10, 18),
        t(8, 0),
        t(9, 0),
        "non_valable",
    )
    .await;
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2023, 10, 18),
        t(14, 0),
        t(15, 0),
        "non_valable",
    )
    .await;
    let v = calcule(&ent, &cast, lea, d(2023, 10, 18), d(2023, 10, 18)).await;
    assert_eq!(v.len(), 2);
    let (matin, apres) = (&v[0], &v[1]);
    assert!(matin.compte, "mercredi matin : cours");
    assert_eq!(apres.compte, apres.raison == "compte");
    // Hors de l'année scolaire (été) : rien ne compte non plus.
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2024, 7, 20),
        t(8, 0),
        t(9, 0),
        "non_valable",
    )
    .await;
    let v = calcule(&ent, &cast, lea, d(2024, 7, 20), d(2024, 7, 20)).await;
    assert_eq!((v[0].compte, v[0].raison), (false, "hors_annee"));
}

// ── Mois arrêté : figé, rectifications à part ───────────────────────────────────────────────────

#[tokio::test]
async fn as4_arret_du_mois_fige_et_les_rectifications_sont_a_part() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (lea, autre) = (lea(&cast), cast.eleve_tom_id);
    annee_fixe(&ent, &cast, "AS4-2022", d(2022, 9, 1), d(2023, 7, 4)).await;
    scolarises_depuis(&ent, &cast, d(2020, 1, 1)).await;
    regle_sql(&ent, &cast, d(2022, 9, 1), "une_seance", 30, 3).await;
    // Harry : trois demi-journées non justifiées en novembre 2022 (seuil 3 atteint) ; Draco : une.
    let mut eps = Vec::new();
    for j in [d(2022, 11, 7), d(2022, 11, 8), d(2022, 11, 9)] {
        eps.push(episode_sql(&ent, &cast, lea, j, t(8, 0), t(9, 0), "non_valable").await);
    }
    episode_sql(
        &ent,
        &cast,
        autre,
        d(2022, 11, 7),
        t(8, 0),
        t(9, 0),
        "non_valable",
    )
    .await;
    let (du, au) = (d(2022, 11, 1), d(2022, 11, 30));
    // Un mois non terminé ne s'arrête pas ; la date doit être le 1er du mois.
    let futur = bornes_futur(&ent, &cast).await;
    assert!(ent
        .arreter_mois_for_actor(etab, cast.direction.user_id, "direction", futur)
        .await
        .is_err());
    assert!(ent
        .arreter_mois_for_actor(etab, cast.direction.user_id, "direction", d(2022, 11, 15))
        .await
        .is_err());
    // Seule la direction arrête (le CPE lit le décompte, il ne fige pas la règle).
    assert!(interdit(
        ent.arreter_mois_for_actor(etab, cast.cpe.user_id, "cpe", du)
            .await
    ));
    assert!(interdit(
        ent.arreter_mois_for_actor(etab, cast.prof.user_id, "prof", du)
            .await
    ));
    let a = ent
        .arreter_mois_for_actor(etab, cast.direction.user_id, "direction", du)
        .await
        .unwrap();
    assert!(a.arrete_maintenant);
    let h = a.eleves.iter().find(|e| e.eleve_id == lea).unwrap();
    assert_eq!(
        (h.demi_journees, h.non_justifiees, h.seuil_signalement),
        (3, 3, 3)
    );
    // Rejouer l'arrêt (action ou tâche système) n'écrit rien : même résultat, jamais réécrit.
    let b = ent.arreter_mois(etab, du, None).await.unwrap();
    assert!(!b.arrete_maintenant);
    assert_eq!(b.eleves.len(), a.eleves.len());
    let (clotures,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_asd_mois_cloture WHERE etablissement_id = $1 AND mois = $2",
    )
    .bind(etab)
    .bind(du)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(clotures, 1);
    // Le mois figé est en ajout seul.
    let maj = sqlx::query(
        "UPDATE ent_asd_mois_arrete SET non_justifiees = 0 WHERE etablissement_id = $1",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await;
    assert!(maj.is_err(), "ent_asd_mois_arrete est en ajout seul");
    assert!(
        sqlx::query("DELETE FROM ent_asd_mois_cloture WHERE etablissement_id = $1")
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // Un recalcul sans changement ne produit aucune rectification.
    let r = ent
        .recalculer_decompte(etab, None, du, au, "recalcul", None)
        .await
        .unwrap();
    assert_eq!(r.rectifications, 0);
    // Requalification après l'arrêt : un des trois épisodes devient valable. Le mois figé ne bouge pas ;
    // la rectification est inscrite à part, et touche le signalement (3 → 2 sous le seuil de 3).
    sqlx::query("UPDATE ent_asd_absence SET qualification = 'excuse_valable' WHERE id = $1")
        .bind(eps[0])
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let r = ent
        .recalculer_decompte(etab, Some(lea), du, au, "requalification", Some(eps[0]))
        .await
        .unwrap();
    assert_eq!(r.rectifications, 1);
    let figes = ent.mois_arrete(etab, du).await.unwrap().unwrap();
    let h = figes.iter().find(|e| e.eleve_id == lea).unwrap();
    assert_eq!(
        (h.demi_journees, h.non_justifiees),
        (3, 3),
        "le mois arrêté reste tel qu'il a été arrêté"
    );
    let m = ent
        .decompte_mensuel_for_actor(etab, cast.cpe.user_id, "cpe", du)
        .await
        .unwrap();
    let l = m.eleves.iter().find(|e| e.eleve_id == lea).unwrap();
    assert_eq!(
        (l.demi_journees_non_justifiees, l.seuil_atteint, m.arrete),
        (3, true, true)
    );
    let rects = ent
        .rectifications_for_actor(etab, cast.cpe.user_id, "cpe", du)
        .await
        .unwrap();
    assert_eq!(rects.len(), 1);
    let rc = &rects[0];
    assert_eq!(
        (
            rc.eleve_id,
            rc.avant_non_justifiees,
            rc.apres_non_justifiees
        ),
        (lea, 3, 2)
    );
    assert_eq!(
        (rc.cause.as_str(), rc.cause_ref_id, rc.touche_signalement),
        ("requalification", Some(eps[0]), true)
    );
    // Idempotent : même état, pas de seconde rectification ; un second changement s'ajoute à la suite.
    let r = ent
        .recalculer_decompte(etab, Some(lea), du, au, "requalification", Some(eps[0]))
        .await
        .unwrap();
    assert_eq!(r.rectifications, 0);
    sqlx::query("UPDATE ent_asd_absence SET qualification = 'excuse_valable' WHERE id = $1")
        .bind(eps[1])
        .execute(ent.pool_verif())
        .await
        .unwrap();
    ent.recalculer_decompte(etab, None, du, au, "requalification", Some(eps[1]))
        .await
        .unwrap();
    let rects = ent
        .rectifications_for_actor(etab, cast.direction.user_id, "direction", du)
        .await
        .unwrap();
    assert_eq!(rects.len(), 2);
    assert_eq!(
        (
            rects[1].avant_non_justifiees,
            rects[1].apres_non_justifiees,
            rects[1].touche_signalement
        ),
        (2, 1, false)
    );
    // « Vue par » : posée une fois, jamais réécrite ; le professeur n'y accède pas.
    assert!(interdit(
        ent.rectifications_for_actor(etab, cast.prof.user_id, "prof", du)
            .await
    ));
    ent.rectification_vue_for_actor(etab, cast.direction.user_id, "direction", rects[0].id)
        .await
        .unwrap();
    assert!(ent
        .rectification_vue_for_actor(etab, cast.direction.user_id, "direction", rects[0].id)
        .await
        .is_err());
    let alteree =
        sqlx::query("UPDATE ent_asd_rectification SET apres_non_justifiees = 9 WHERE id = $1")
            .bind(rects[0].id)
            .execute(ent.pool_verif())
            .await;
    assert!(alteree.is_err(), "une rectification ne se modifie pas");
    // Matérialisation vivante : le recalcul a mis à jour les lignes (elles ne sont pas figées, le mois l'est).
    assert_eq!(
        nombre(&ent, "SELECT count(*) FROM ent_asd_demi_journee WHERE etablissement_id = $1 AND eleve_id = $2 AND compte AND NOT non_justifiee", etab, lea).await,
        2
    );
    // Isolation : un autre établissement ne voit ni le mois ni les rectifications.
    let autre_etab = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    assert!(ent
        .rectifications_for_actor(
            autre_etab.etablissement_id,
            autre_etab.cpe.user_id,
            "cpe",
            du
        )
        .await
        .unwrap()
        .is_empty());
    assert!(ent
        .mois_arrete(autre_etab.etablissement_id, du)
        .await
        .unwrap()
        .is_none());
}

async fn bornes_futur(ent: &PgEnt, cast: &LaboCast) -> NaiveDate {
    let j = aujourdhui(ent, cast).await;
    crate::assiduite::bornes_mois(j).0
}

// ── Calendrier, fuseau et coupure ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn as4_la_coupure_de_la_grille_prime_sur_le_repli_de_la_regle() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = lea(&cast);
    annee_fixe(&ent, &cast, "AS4-2021", d(2021, 9, 1), d(2022, 7, 4)).await;
    scolarises_depuis(&ent, &cast, d(2020, 1, 1)).await;
    // Séance 11 h 30–12 h 30 : avec la coupure de la règle (12 h), 30 minutes le matin et 30 l'après-midi.
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2021, 10, 5),
        t(11, 30),
        t(12, 30),
        "non_valable",
    )
    .await;
    let v = calcule(&ent, &cast, lea, d(2021, 10, 5), d(2021, 10, 5)).await;
    assert_eq!(v.len(), 2);
    assert!(v
        .iter()
        .all(|x| x.coupure_source == "regle" && x.minutes_manquees == 30));
    // Une grille du socle 5 avec une coupure à 12 h 30 s'applique à la classe de l'élève : tout le matin.
    sqlx::query("UPDATE ent_asd_regle SET heure_coupure = '12:00' WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let niveau: Option<Uuid> = sqlx::query_scalar(
        "SELECT c.niveau_id FROM ent_classe c JOIN ent_eleve e ON e.classe_id = c.id AND e.etablissement_id = c.etablissement_id
          WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(etab)
    .bind(lea)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let _ = niveau;
    // La grille se pose sur l'établissement entier (aucun site ni niveau ciblé) : nouvelle version datée.
    let g = Uuid::new_v4();
    let mut tx = ent.pool_verif().begin().await.unwrap();
    // Grille déjà commencée (valable depuis 2021) : la saisie passe hors garde, comme une reprise d'historique.
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_grille (id, etablissement_id, code, libelle, valable_du, coupure_demi_journee, origine)
         VALUES ($1,$2,'G-AS4','Grille AS4','2021-09-01','12:30','service')",
    )
    .bind(g)
    .bind(etab)
    .execute(&mut *tx)
    .await
    .expect("grille");
    sqlx::query("INSERT INTO ent_grille_cible (id, etablissement_id, grille_id) VALUES ($1,$2,$3)")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(g)
        .execute(&mut *tx)
        .await
        .expect("cible");
    tx.commit().await.unwrap();
    let v = calcule(&ent, &cast, lea, d(2021, 10, 5), d(2021, 10, 5)).await;
    assert_eq!(
        v.len(),
        1,
        "coupure 12 h 30 : la séance est tout entière le matin"
    );
    assert_eq!((v[0].coupure_source, v[0].minutes_manquees), ("grille", 60));
}

#[tokio::test]
async fn as4_le_jour_metier_est_celui_de_l_etablissement_pour_l_arret() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    // Le mois courant n'est jamais arrêtable ; le mois précédent l'est dès que le jour métier est le 1er.
    let jour = aujourdhui(&ent, &cast).await;
    let courant = crate::assiduite::bornes_mois(jour).0;
    assert!(ent.arreter_mois(etab, courant, None).await.is_err());
    let precedent = crate::assiduite::bornes_mois(courant - Duration::days(1)).0;
    let a = ent.arreter_mois(etab, precedent, None).await.unwrap();
    assert!(
        a.arrete_maintenant && a.eleves.is_empty(),
        "mois sans absence : arrêté vide, 0 demi-journée"
    );
    let a = ent.arreter_mois(etab, precedent, None).await.unwrap();
    assert!(!a.arrete_maintenant);
    // Le tick système arrête les mois terminés des trois derniers mois, une fois chacun.
    let n = ent.arreter_mois_systeme(etab).await.unwrap();
    assert_eq!(n, 2, "les deux autres mois des trois derniers");
    assert_eq!(ent.arreter_mois_systeme(etab).await.unwrap(), 0);
}

#[tokio::test]
async fn as4_explication_et_droits_de_lecture() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let jour = monde_du_jour(&ent, &cast).await;
    let lea = lea(&cast);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    appel(&ent, &cast, jour, (9, 0), (10, 0), &[lea]).await;
    // La vie scolaire et la famille de l'enfant lisent l'explication ; une autre famille, non.
    let l = ent
        .decompte_eleve_explique_for_actor(etab, cast.cpe.user_id, "cpe", lea, jour, jour)
        .await
        .unwrap();
    assert_eq!(l.len(), 1);
    assert!(
        l[0].1.contains("compte, non justifiée") && l[0].1.contains("2 séances manquées (120 min)"),
        "{}",
        l[0].1
    );
    assert!(
        l[0].1.contains("version du"),
        "la version de la règle est affichée : {}",
        l[0].1
    );
    let parent = ent
        .decompte_eleve_explique_for_actor(etab, cast.parent_lea.user_id, "parent", lea, jour, jour)
        .await;
    assert!(parent.is_ok(), "{parent:?}");
    assert!(interdit(
        ent.decompte_eleve_explique_for_actor(
            etab,
            cast.parent_tom.user_id,
            "parent",
            lea,
            jour,
            jour
        )
        .await
    ));
    // Aucun motif ni note dans l'explication (R-AS19, R-AS22) : elle ne sort que de champs codés.
    assert!(!l[0].1.to_lowercase().contains("maladie"));
    // Période invalide ou trop longue : refusée.
    assert!(ent
        .decompte_eleve_explique_for_actor(
            etab,
            cast.cpe.user_id,
            "cpe",
            lea,
            jour,
            jour - Duration::days(1)
        )
        .await
        .is_err());
    assert!(ent
        .decompte_eleve_explique_for_actor(
            etab,
            cast.cpe.user_id,
            "cpe",
            lea,
            jour,
            jour + Duration::days(500)
        )
        .await
        .is_err());
}

// ── Scolarisation : hors de l'établissement, rien ne compte (DA-22) ─────────────────────────────

#[tokio::test]
async fn as4_un_eleve_hors_scolarisation_ne_compte_pas() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let lea = lea(&cast);
    annee_fixe(&ent, &cast, "AS4-2020", d(2020, 9, 1), d(2021, 7, 4)).await;
    // Les élèves de labo entrent le 1er septembre 2026 : une absence antérieure n'est pas une absence.
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2020, 10, 6),
        t(8, 0),
        t(10, 0),
        "non_valable",
    )
    .await;
    let v = calcule(&ent, &cast, lea, d(2020, 10, 6), d(2020, 10, 6)).await;
    assert_eq!(v.len(), 1);
    assert_eq!((v[0].compte, v[0].raison), (false, "hors_scolarisation"));
    assert!(expliquer(&v[0]).contains("pas scolarisé"));
    // Scolarisé depuis 2020, parti le 1er octobre 2020 (premier jour hors établissement) : toujours hors.
    scolarises_depuis(&ent, &cast, d(2020, 1, 1)).await;
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_eleve_scolarisation SET date_sortie = '2020-10-06' WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(cast.etablissement_id)
        .bind(lea)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let v = calcule(&ent, &cast, lea, d(2020, 10, 6), d(2020, 10, 6)).await;
    assert_eq!(
        v[0].raison, "hors_scolarisation",
        "la date de sortie est le premier jour hors établissement"
    );
    episode_sql(
        &ent,
        &cast,
        lea,
        d(2020, 10, 5),
        t(8, 0),
        t(10, 0),
        "non_valable",
    )
    .await;
    let v = calcule(&ent, &cast, lea, d(2020, 10, 5), d(2020, 10, 5)).await;
    assert!(
        v[0].compte && v[0].non_justifiee,
        "la veille du départ, l'élève est encore scolarisé"
    );
}

// ── Rectification déclenchée par la qualification, sans recalcul manuel ─────────────────────────

#[tokio::test]
async fn as4_requalifier_apres_l_arret_inscrit_la_rectification() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let lea = lea(&cast);
    annee_fixe(&ent, &cast, "AS4-2019", d(2019, 9, 1), d(2020, 7, 4)).await;
    scolarises_depuis(&ent, &cast, d(2019, 1, 1)).await;
    let ep = episode_sql(
        &ent,
        &cast,
        lea,
        d(2019, 11, 5),
        t(8, 0),
        t(10, 0),
        "en_attente",
    )
    .await;
    let a = ent
        .arreter_mois_for_actor(etab, cast.direction.user_id, "direction", d(2019, 11, 1))
        .await
        .unwrap();
    let l = a.eleves.iter().find(|e| e.eleve_id == lea).expect("Harry");
    assert_eq!(
        (l.demi_journees, l.non_justifiees),
        (1, 1),
        "en attente de qualification : non justifiée jusqu'à décision"
    );
    assert!(ent
        .rectifications_for_actor(etab, cast.cpe.user_id, "cpe", d(2019, 11, 1))
        .await
        .unwrap()
        .is_empty());
    // La vie scolaire qualifie l'épisode après l'arrêt : la rectification s'inscrit d'elle-même.
    ent.qualifier_episode_for_actor(
        etab,
        cast.cpe.user_id,
        "cpe",
        ep,
        qualifier("excuse_valable", 1),
    )
    .await
    .unwrap();
    let r = ent
        .rectifications_for_actor(etab, cast.cpe.user_id, "cpe", d(2019, 11, 1))
        .await
        .unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(
        (
            r[0].avant_non_justifiees,
            r[0].apres_non_justifiees,
            r[0].avant_demi_journees,
            r[0].apres_demi_journees
        ),
        (1, 0, 1, 1)
    );
    assert_eq!(
        (r[0].cause.as_str(), r[0].cause_ref_id),
        ("requalification", Some(ep))
    );
    // Le mois arrêté n'a pas bougé, et la requalification suivante (retour en arrière) s'ajoute à la suite.
    let m = ent
        .decompte_mensuel_for_actor(etab, cast.cpe.user_id, "cpe", d(2019, 11, 1))
        .await
        .unwrap();
    assert!(m.arrete);
    assert_eq!(
        m.eleves
            .iter()
            .find(|e| e.eleve_id == lea)
            .map(|e| e.demi_journees_non_justifiees),
        Some(1)
    );
    let (version,): (i32,) = sqlx::query_as("SELECT version FROM ent_asd_absence WHERE id = $1")
        .bind(ep)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    ent.requalifier_episode_for_actor(
        etab,
        cast.cpe.user_id,
        "cpe",
        ep,
        RequalifierEpisode {
            qualification: "non_valable".into(),
            version,
            nature_code: None,
            libelle_famille_code: None,
            note_interne: None,
            motif_code: "erreur_de_saisie".into(),
        },
    )
    .await
    .expect("requalification");
    let r = ent
        .rectifications_for_actor(etab, cast.cpe.user_id, "cpe", d(2019, 11, 1))
        .await
        .unwrap();
    assert_eq!(r.len(), 2);
    assert_eq!(
        (r[1].avant_non_justifiees, r[1].apres_non_justifiees),
        (0, 1)
    );
}
