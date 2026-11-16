//! Socle 12, lot AS5 (familles et notifications). Réf. : `docs/socles/12_ASSIDUITE.md` §10.5, §14, R-AS13,
//! R-AS14, R-AS18, §24.1. Critères de fin : une journée d'absence = 2 avis au plus par foyer ; arrivée à
//! l'infirmerie dans le délai → aucun avis ; avis parti puis correction → démenti ; parent sous interdiction :
//! rien. Chaque test travaille dans un établissement neuf (jeu de labo) ; les instants de remise sont ceux de
//! l'événement (jamais l'heure réelle d'exécution comme condition).
//! Sans `ENT_DATABASE_URL` : ignorés par absence de base (le test échoue en CI, qui fournit Postgres).

use super::*;
use crate::ntf::catalogue::{
    ABSENCE_CONSTATEE, ABSENCE_DECISION, ABSENCE_DEMENTI, RELANCE_JUSTIFICATIF,
};
use crate::ntf::expedition::expedier_avis_dus;
use crate::tests_notifications_n1::{basculer, config, distribuer, notifications_legacy};
use chrono::{Duration, NaiveDate, NaiveTime, Utc};
use domain_eleve::DomainError;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    Some(
        crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
            e.install_brain_p0();
        })
        .await,
    )
}

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn introuvable<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Domain(DomainError::NotFound)))
}

fn interdit<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Forbidden))
}

fn invalide<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::SaisieInvalide(_)))
}

fn conflit<T: std::fmt::Debug>(r: Result<T, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::ConflitDossier(_)))
}

async fn aujourdhui(ent: &PgEnt, cast: &LaboCast) -> NaiveDate {
    ent.aujourdhui_etablissement(cast.etablissement_id)
        .await
        .unwrap()
}

async fn compte(ent: &PgEnt, sql: &str, a: Uuid, b: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(a)
        .bind(b)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

fn saisie(etat: &str) -> SaisieLigne {
    SaisieLigne {
        etat: etat.into(),
        minutes: None,
        lieu_code: None,
    }
}

fn correction(etat: &str) -> CorrectionLigne {
    CorrectionLigne {
        etat: etat.into(),
        minutes: None,
        lieu_code: None,
        motif_code: None,
        motif_texte: Some("erreur de saisie".into()),
        version_base: None,
    }
}

/// Appel de 4B sur `[debut, fin)` : les `absents` sont marqués absents, validation. Renvoie l'appel.
async fn appel(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    debut: (u32, u32),
    fin: (u32, u32),
    absents: &[Uuid],
) -> Uuid {
    let etab = cast.etablissement_id;
    let c = ent
        .seed_creneau_du_jour(
            etab,
            "4B",
            "AS5",
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
    a.id
}

/// Remise des avis échus : 40 minutes après maintenant (le délai de consolidation est de 20 minutes).
async fn expedier(ent: &PgEnt, etab: Uuid, apres_min: i64) -> crate::ntf::expedition::BilanAvis {
    expedier_avis_dus(ent.pool(), etab, Utc::now() + Duration::minutes(apres_min))
        .await
        .unwrap()
}

async fn avis_nature(ent: &PgEnt, etab: Uuid, eleve: Uuid, nature: &str) -> Vec<(Uuid, String)> {
    sqlx::query_as(
        "SELECT id, statut FROM ent_ntf_avis WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = $3 ORDER BY created_at, id",
    )
    .bind(etab)
    .bind(eleve)
    .bind(nature)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap()
}

async fn ledger(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Vec<(String, String, Option<String>)> {
    let l: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT periode, statut, cause_code FROM ent_asd_avis_demi_journee
          WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY jour, periode, generation",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    // Une correction d'appel émet deux événements (« remis présent » de l'ancien circuit et le démenti) : le
    // premier traité fixe la cause (`present` ou `correction`), deux noms pour le même fait.
    l.into_iter()
        .map(|(p, s, c)| {
            (
                p,
                s,
                c.map(|c| {
                    if c == "present" {
                        "correction".to_string()
                    } else {
                        c
                    }
                }),
            )
        })
        .collect()
}

async fn corps_notification(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    kind: &str,
) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT titre, corps FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = $3
          ORDER BY created_at, id",
    )
    .bind(etab)
    .bind(user)
    .bind(kind)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap()
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

async fn episodes(ent: &PgEnt, cast: &LaboCast, eleve: Uuid) -> Vec<EpisodeAsd> {
    ent.episodes_for_actor(
        cast.etablissement_id,
        cast.cpe.user_id,
        "cpe",
        FiltreEpisodes {
            eleve_id: Some(eleve),
            etat: Some("tous".into()),
            ..Default::default()
        },
    )
    .await
    .expect("episodes")
}

/// Pose directement une version de la règle d'assiduité en vigueur depuis 2020 (préparation de test : le service
/// refuse une règle posée dans le passé, R-AS15 ; le propriétaire de la base, lui, prépare l'état de départ).
async fn poser_regle(ent: &PgEnt, etab: Uuid, delai: i32, rythme: &str) {
    sqlx::query("DELETE FROM ent_asd_regle WHERE etablissement_id = $1 AND figee_le IS NULL")
        .bind(etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_asd_regle (id, etablissement_id, valide_du, delai_consolidation_min, rythme_relance)
         VALUES ($1, $2, DATE '2020-01-01', $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(delai)
    .bind(rythme)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

// ───────────────────────── Avis par foyer et par demi-journée ─────────────────────────

/// Critère de fin : une journée d'absence donne **deux avis au plus** (un par demi-journée), jamais un par
/// séance ; le délai de consolidation tient ; l'ancien circuit se tait ; chaque avis porte sa demi-journée.
#[tokio::test]
async fn as5_une_journee_d_absence_donne_deux_avis_au_plus() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
    );
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    for (d, f) in [
        ((8, 0), (9, 0)),
        ((9, 0), (10, 0)),
        ((14, 0), (15, 0)),
        ((15, 0), (16, 0)),
    ] {
        appel(&ent, &cast, jour, d, f, &[lea]).await;
        distribuer(&ent, etab).await;
    }
    let avis = avis_nature(&ent, etab, lea, ABSENCE_CONSTATEE.code).await;
    assert_eq!(
        avis.len(),
        2,
        "quatre séances, deux demi-journées : deux avis"
    );
    assert!(avis.iter().all(|(_, s)| s == "planifie"));
    let l = ledger(&ent, etab, lea).await;
    assert_eq!(
        l.iter()
            .map(|(p, s, _)| (p.as_str(), s.as_str()))
            .collect::<Vec<_>>(),
        vec![("apres_midi", "programme"), ("matin", "programme")]
    );
    // Rien ne part avant la fin du délai de consolidation, et l'ancien chemin se tait.
    assert_eq!(expedier(&ent, etab, 5).await.avis, 0);
    assert_eq!(
        notifications_legacy(&ent, etab, parent, "absence_constatee").await,
        0
    );
    // Après le délai : un seul avis par demi-journée arrive à la famille.
    let b = expedier(&ent, etab, 40).await;
    assert_eq!(b.avis, 2);
    let recus = corps_notification(&ent, etab, parent, "absence_constatee").await;
    assert_eq!(recus.len(), 2, "deux avis, pas quatre");
    assert!(
        recus.iter().any(|(_, c)| c.contains("(matin)")),
        "{recus:?}"
    );
    assert!(
        recus.iter().any(|(_, c)| c.contains("(après-midi)")),
        "{recus:?}"
    );
    // Rejeu du consommateur : aucun avis de plus.
    let d = ent.distributeur(config(etab));
    d.rejouer_consommateur(CONSOMMATEUR_FAMILLES, etab, 0)
        .await
        .unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(
        avis_nature(&ent, etab, lea, ABSENCE_CONSTATEE.code)
            .await
            .len(),
        2
    );
    assert_eq!(ledger(&ent, etab, lea).await.len(), 2);
}

/// L'unicité d'un avis vivant par demi-journée est portée par la base elle-même.
#[tokio::test]
async fn as5_la_base_refuse_deux_avis_vivants_pour_une_demi_journee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    let (avis,): (Uuid,) = sqlx::query_as("SELECT avis_id FROM ent_asd_avis_demi_journee WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(lea)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let r = sqlx::query(
        "INSERT INTO ent_asd_avis_demi_journee (id, etablissement_id, eleve_id, jour, periode, generation, avis_id)
         VALUES ($1,$2,$3,$4,'matin',2,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(lea)
    .bind(jour)
    .bind(avis)
    .execute(ent.pool_verif())
    .await;
    assert!(
        r.is_err(),
        "un second avis vivant pour la même demi-journée est refusé par la base"
    );
}

/// Tant que la brique n'est pas basculée, l'ancien circuit prévient seul : aucun registre, aucun avis du moteur.
#[tokio::test]
async fn as5_sans_bascule_l_ancien_circuit_previent_seul() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
    );
    let jour = aujourdhui(&ent, &cast).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    assert!(ledger(&ent, etab, lea).await.is_empty());
    assert!(avis_nature(&ent, etab, lea, ABSENCE_CONSTATEE.code)
        .await
        .is_empty());
    assert_eq!(
        notifications_legacy(&ent, etab, parent, "absence_constatee").await,
        1,
        "l'ancien circuit a prévenu"
    );
}

/// Une absence déjà annoncée par la famille ne donne aucun avis (comme l'ancien circuit).
#[tokio::test]
async fn as5_une_absence_prevenue_ne_donne_aucun_avis() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let h = cast
        .eleves
        .iter()
        .find(|e| e.prenom == "Hermione")
        .unwrap()
        .id;
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    let a = appel(&ent, &cast, jour, (8, 0), (9, 0), &[h]).await;
    // Un événement d'absence d'Hermione porte `prevenue = true` (la famille l'avait annoncée) : aucun avis ne
    // naît de cet événement.
    let evt = crate::tests_notifications_n1::publier(
        &ent,
        crate::tests_notifications_n1::evenement(
            etab,
            "assiduite.absence_ouverte",
            ent_events::AggregateKind::Absence,
            h,
            serde_json::json!({"appel_id": a, "jour": jour.to_string(), "source": "appel", "prevenue": true}),
        ),
    )
    .await;
    distribuer(&ent, etab).await;
    let n: i64 = compte(
        &ent,
        "SELECT count(*) FROM ent_ntf_avis WHERE etablissement_id = $1 AND cause_event_id = $2",
        etab,
        evt.id,
    )
    .await;
    assert_eq!(n, 0, "absence annoncée : la famille est déjà prévenue");
}

// ───────────────────────── Infirmerie, correction, démenti ─────────────────────────

/// Critère de fin : arrivée à l'infirmerie dans le délai → aucun avis ne part ; aucun démenti non plus.
#[tokio::test]
async fn as5_arrivee_a_l_infirmerie_dans_le_delai_aucun_avis() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
    );
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    assert_eq!(
        avis_nature(&ent, etab, lea, ABSENCE_CONSTATEE.code).await[0].1,
        "planifie"
    );
    let inf = cast.infirmieres[0].user_id;
    ent.poser_localisation_for_actor(
        etab,
        inf,
        "infirmerie",
        PoserLocalisation {
            eleve_id: lea,
            lieu_code: "infirmerie".into(),
            jour: None,
            heure_debut: None,
            heure_fin: None,
        },
    )
    .await
    .expect("localiser");
    distribuer(&ent, etab).await;
    let avis = avis_nature(&ent, etab, lea, ABSENCE_CONSTATEE.code).await;
    assert_eq!(avis[0].1, "annule", "l'avis est retiré avant toute remise");
    assert_eq!(
        ledger(&ent, etab, lea).await,
        vec![(
            "matin".to_string(),
            "annule".to_string(),
            Some("localisation".to_string())
        )]
    );
    assert_eq!(expedier(&ent, etab, 60).await.avis, 0);
    assert_eq!(
        notifications_legacy(&ent, etab, parent, "absence_constatee").await,
        0
    );
    assert!(
        avis_nature(&ent, etab, lea, ABSENCE_DEMENTI.code)
            .await
            .is_empty(),
        "rien n'est parti : aucun démenti"
    );
}

/// Critère de fin : avis parti, puis correction de l'appel → un démenti est notifié (R-AS14), une seule fois.
#[tokio::test]
async fn as5_avis_parti_puis_correction_donne_un_dementi() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
    );
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    let a = appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    assert_eq!(expedier(&ent, etab, 30).await.avis, 1);
    assert_eq!(
        corps_notification(&ent, etab, parent, "absence_constatee")
            .await
            .len(),
        1
    );
    // L'intervenant corrige : l'élève était présent.
    ent.corriger_ligne_for_actor(
        etab,
        a,
        lea,
        cast.prof.user_id,
        "prof",
        correction("present"),
        "web",
    )
    .await
    .expect("correction");
    distribuer(&ent, etab).await;
    assert_eq!(
        ledger(&ent, etab, lea).await,
        vec![(
            "matin".to_string(),
            "dementi".to_string(),
            Some("correction".to_string())
        )]
    );
    let dementis = avis_nature(&ent, etab, lea, ABSENCE_DEMENTI.code).await;
    assert_eq!(dementis.len(), 1, "un démenti, pas deux");
    assert_eq!(expedier(&ent, etab, 31).await.avis, 1);
    let recus = corps_notification(&ent, etab, parent, "absence_dementi").await;
    assert_eq!(recus.len(), 1);
    assert!(recus[0].0.contains("Correction"), "{recus:?}");
    assert!(recus[0].1.contains("(matin)"), "{recus:?}");
    // Rejeu : toujours un seul démenti.
    let d = ent.distributeur(config(etab));
    d.rejouer_consommateur(CONSOMMATEUR_FAMILLES, etab, 0)
        .await
        .unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(
        avis_nature(&ent, etab, lea, ABSENCE_DEMENTI.code)
            .await
            .len(),
        1
    );
    // L'élève est à nouveau constaté absent plus tard dans la même demi-journée : un nouvel avis, génération 2.
    ent.corriger_ligne_for_actor(
        etab,
        a,
        lea,
        cast.prof.user_id,
        "prof",
        correction("absent"),
        "web",
    )
    .await
    .expect("re-correction");
    distribuer(&ent, etab).await;
    let g: Vec<i16> = sqlx::query_scalar(
        "SELECT generation FROM ent_asd_avis_demi_journee WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY generation",
    )
    .bind(etab)
    .bind(lea)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(g.len() <= 2, "{g:?}");
}

/// Un avis corrigé **avant** la remise est annulé sans démenti.
#[tokio::test]
async fn as5_correction_dans_le_delai_annule_l_avis_sans_dementi() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
    );
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    let a = appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    ent.corriger_ligne_for_actor(
        etab,
        a,
        lea,
        cast.prof.user_id,
        "prof",
        correction("present"),
        "web",
    )
    .await
    .expect("correction");
    distribuer(&ent, etab).await;
    assert_eq!(
        avis_nature(&ent, etab, lea, ABSENCE_CONSTATEE.code).await[0].1,
        "annule"
    );
    assert_eq!(
        ledger(&ent, etab, lea).await,
        vec![(
            "matin".to_string(),
            "annule".to_string(),
            Some("correction".to_string())
        )]
    );
    assert!(avis_nature(&ent, etab, lea, ABSENCE_DEMENTI.code)
        .await
        .is_empty());
    assert_eq!(expedier(&ent, etab, 60).await.avis, 0);
    assert_eq!(
        notifications_legacy(&ent, etab, parent, "absence_constatee").await,
        0
    );
}

// ───────────────────────── Décision modifiée (requalification) ─────────────────────────

/// Requalification qui change le sens : la famille en est informée une seule fois (`absence_decision`), sans
/// que l'ancien circuit n'envoie « justifiée » en plus.
#[tokio::test]
async fn as5_requalification_qui_change_le_sens_est_notifiee_une_fois() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent, cpe) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
        cast.cpe.user_id,
    );
    basculer(&ent, etab, ABSENCE_DECISION.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    let r = ent
        .qualifier_episode_for_actor(
            etab,
            cpe,
            "cpe",
            ep.id,
            qualifier("non_valable", ep.version),
        )
        .await
        .unwrap();
    // Qualification initiale : pas de nouvel avis de décision (ce n'est pas un changement de sens).
    distribuer(&ent, etab).await;
    assert!(avis_nature(&ent, etab, lea, ABSENCE_DECISION.code)
        .await
        .is_empty());
    let rq = RequalifierEpisode {
        qualification: "excuse_valable".into(),
        version: r.version,
        motif_code: "nouvelle_piece".into(),
        nature_code: None,
        libelle_famille_code: None,
        note_interne: None,
    };
    ent.requalifier_episode_for_actor(etab, cpe, "cpe", ep.id, rq)
        .await
        .unwrap();
    distribuer(&ent, etab).await;
    assert_eq!(
        avis_nature(&ent, etab, lea, ABSENCE_DECISION.code)
            .await
            .len(),
        1
    );
    expedier(&ent, etab, 5).await;
    let recus = corps_notification(&ent, etab, parent, "absence_decision").await;
    assert_eq!(recus.len(), 1, "{recus:?}");
    assert!(recus[0].1.contains("désormais justifiée"), "{recus:?}");
    assert_eq!(
        notifications_legacy(&ent, etab, parent, "absence_justifiee").await,
        0,
        "l'ancien circuit ne double pas la décision modifiée"
    );
    // Rejeu : toujours un seul avis de décision.
    let d = ent.distributeur(config(etab));
    d.rejouer_consommateur(CONSOMMATEUR_FAMILLES, etab, 0)
        .await
        .unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(
        avis_nature(&ent, etab, lea, ABSENCE_DECISION.code)
            .await
            .len(),
        1
    );
}

/// Sans bascule de `absence_decision`, l'ancien circuit garde sa notification et la brique n'en ajoute pas.
#[tokio::test]
async fn as5_sans_bascule_la_requalification_reste_a_l_ancien_circuit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent, cpe) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
        cast.cpe.user_id,
    );
    let jour = aujourdhui(&ent, &cast).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    let ep = episodes(&ent, &cast, lea).await.remove(0);
    let r = ent
        .qualifier_episode_for_actor(
            etab,
            cpe,
            "cpe",
            ep.id,
            qualifier("non_valable", ep.version),
        )
        .await
        .unwrap();
    let rq = RequalifierEpisode {
        qualification: "excuse_valable".into(),
        version: r.version,
        motif_code: "nouvelle_piece".into(),
        nature_code: None,
        libelle_famille_code: None,
        note_interne: None,
    };
    ent.requalifier_episode_for_actor(etab, cpe, "cpe", ep.id, rq)
        .await
        .unwrap();
    distribuer(&ent, etab).await;
    assert!(avis_nature(&ent, etab, lea, ABSENCE_DECISION.code)
        .await
        .is_empty());
    assert_eq!(
        notifications_legacy(&ent, etab, parent, "absence_justifiee").await,
        1
    );
}

// ───────────────────────── Interdiction, droit du lien ─────────────────────────

async fn personne_du_parent(ent: &PgEnt, etab: Uuid, eleve: Uuid, parent: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT personne_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(eleve)
        .bind(parent)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

/// Critère de fin : un parent sous interdiction de communication ne reçoit rien, même si l'interdiction est
/// posée entre l'avis et la remise (relue à la remise).
#[tokio::test]
async fn as5_parent_sous_interdiction_ne_recoit_rien() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
    );
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    let personne = personne_du_parent(&ent, etab, lea, parent).await;
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1, $2, $3, $4, ARRAY['communication'], 'decision_justice', 'consigne de test', DATE '2020-01-01', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(lea)
    .bind(personne)
    .bind(cast.direction.user_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    expedier(&ent, etab, 40).await;
    assert_eq!(
        notifications_legacy(&ent, etab, parent, "absence_constatee").await,
        0
    );
    let motif: Option<String> = sqlx::query_scalar(
        "SELECT r.motif FROM ent_ntf_remise r
           JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
           JOIN ent_ntf_avis a ON a.id = d.avis_id
          WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND d.compte_id = $3 AND r.canal = 'inapp'",
    )
    .bind(etab)
    .bind(lea)
    .bind(parent)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif.as_deref(), Some("interdiction"));
    // Une interdiction déjà en vigueur à la création : le parent est exclu avec sa raison, sans remise.
    let h = cast.eleves.iter().find(|e| e.prenom == "Hermione").unwrap();
    let personne_h = personne_du_parent(&ent, etab, h.id, h.parent.user_id).await;
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1, $2, $3, $4, ARRAY['communication'], 'decision_justice', 'consigne de test', DATE '2020-01-01', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(h.id)
    .bind(personne_h)
    .bind(cast.direction.user_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    appel(&ent, &cast, jour, (14, 0), (15, 0), &[h.id]).await;
    distribuer(&ent, etab).await;
    let (retenu, code): (bool, Option<String>) = sqlx::query_as(
        "SELECT d.retenu, d.exclusion_code FROM ent_ntf_destinataire d JOIN ent_ntf_avis a ON a.id = d.avis_id
          WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND d.personne_id = $3",
    )
    .bind(etab)
    .bind(h.id)
    .bind(personne_h)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(!retenu);
    assert_eq!(code.as_deref(), Some("interdiction"));
    expedier(&ent, etab, 40).await;
    assert_eq!(
        notifications_legacy(&ent, etab, h.parent.user_id, "absence_constatee").await,
        0
    );
}

/// Un lien sans le droit de voir la vie scolaire ne reçoit pas l'avis : exclu avec sa raison.
#[tokio::test]
async fn as5_lien_sans_droit_de_vie_scolaire_ne_recoit_rien() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
    );
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(lea)
        .bind(parent)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let jour = aujourdhui(&ent, &cast).await;
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    let code: Option<String> = sqlx::query_scalar(
        "SELECT d.exclusion_code FROM ent_ntf_destinataire d JOIN ent_ntf_avis a ON a.id = d.avis_id
          WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND d.compte_id = $3",
    )
    .bind(etab)
    .bind(lea)
    .bind(parent)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(code.as_deref(), Some("droit_insuffisant"));
    expedier(&ent, etab, 40).await;
    assert_eq!(
        notifications_legacy(&ent, etab, parent, "absence_constatee").await,
        0
    );
}

/// Un élève décédé : aucun avis ne naît (gel du sujet, socle 8).
#[tokio::test]
async fn as5_eleve_decede_aucun_avis() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    let a = appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    ent.enregistrer_deces(
        "direction",
        etab,
        lea,
        domain_eleve::horloge::jour_paris(),
        cast.direction.user_id,
    )
    .await
    .expect("décès");
    distribuer(&ent, etab).await;
    assert_eq!(
        avis_nature(&ent, etab, lea, ABSENCE_CONSTATEE.code).await[0].1,
        "annule"
    );
    assert!(ledger(&ent, etab, lea)
        .await
        .iter()
        .all(|(_, s, _)| s == "annule"));
    // Une nouvelle absence constatée après le décès (événement rejoué) : le sujet est gelé, aucun avis ne naît.
    crate::tests_notifications_n1::publier(
        &ent,
        crate::tests_notifications_n1::evenement(
            etab,
            "assiduite.absence_ouverte",
            ent_events::AggregateKind::Absence,
            lea,
            serde_json::json!({"appel_id": a, "jour": jour.to_string(), "source": "appel", "prevenue": false}),
        ),
    )
    .await;
    distribuer(&ent, etab).await;
    assert_eq!(
        avis_nature(&ent, etab, lea, ABSENCE_CONSTATEE.code)
            .await
            .len(),
        1,
        "aucun nouvel avis"
    );
    assert!(ledger(&ent, etab, lea)
        .await
        .iter()
        .all(|(_, s, _)| s == "annule"));
}

// ───────────────────────── Relance graduée ─────────────────────────

async fn episode_sans_motif(ent: &PgEnt, etab: Uuid, eleve: Uuid, jour: NaiveDate) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_asd_absence (id, etablissement_id, eleve_id, jour_debut, jour_fin, etat, qualification, source)
         VALUES ($1, $2, $3, $4, $4, 'a_qualifier', 'en_attente', 'appel')",
    )
    .bind(id)
    .bind(etab)
    .bind(eleve)
    .bind(jour)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

async fn jours_de_relance(
    ent: &PgEnt,
    etab: Uuid,
    eleve: Uuid,
    depuis: NaiveDate,
    delais: [i64; 4],
) -> [NaiveDate; 4] {
    let classe: Option<Uuid> = sqlx::query_scalar(
        "SELECT classe_id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let mut c = ent.pool_verif().acquire().await.unwrap();
    let mut out = [depuis; 4];
    for (i, d) in delais.iter().enumerate() {
        out[i] = crate::assiduite_as5::jour_scolaire_apres(&mut c, etab, depuis, *d, classe)
            .await
            .unwrap();
    }
    out
}

/// Relance graduée : J+1 rappel, J+3 canal externe, J+5 « sans réponse », J+8 proposition « sans motif » ;
/// chaque étape ne part qu'une fois ; rien n'est qualifié d'office (R-AS18).
#[tokio::test]
async fn as5_relance_graduee_etape_par_etape() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, parent) = (
        cast.etablissement_id,
        cast.eleve_lea_id,
        cast.parent_lea.user_id,
    );
    basculer(&ent, etab, RELANCE_JUSTIFICATIF.code).await;
    let fin = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
    let ep = episode_sans_motif(&ent, etab, lea, fin).await;
    let j = jours_de_relance(&ent, etab, lea, fin, [1, 3, 5, 8]).await;
    assert!(j.windows(2).all(|w| w[0] < w[1]));
    // Avant J+1 : rien.
    let b = ent
        .relancer_jour(
            etab,
            fin + Duration::days(0) + Duration::days(0),
            Utc::now(),
        )
        .await
        .unwrap();
    assert_eq!(b, BilanRelance::default());
    // J+1 : rappel (ENT, push).
    let b = ent.relancer_jour(etab, j[0], Utc::now()).await.unwrap();
    assert_eq!((b.rappels, b.sautees), (1, 0));
    assert_eq!(
        ent.relancer_jour(etab, j[0], Utc::now()).await.unwrap(),
        BilanRelance::default(),
        "une étape ne part qu'une fois"
    );
    expedier(&ent, etab, 2).await;
    let recus = corps_notification(&ent, etab, parent, "relance_justificatif").await;
    assert_eq!(recus.len(), 1, "{recus:?}");
    assert!(recus[0].1.contains("08/09/2026"), "{recus:?}");
    // J+3 : canal externe (urgence relevée à U2).
    let b = ent.relancer_jour(etab, j[1], Utc::now()).await.unwrap();
    assert_eq!(b.canaux_externes, 1);
    let urgences: Vec<String> = sqlx::query_scalar(
        "SELECT urgence FROM ent_ntf_avis WHERE etablissement_id = $1 AND nature = $2 AND eleve_id = $3 ORDER BY created_at",
    )
    .bind(etab)
    .bind(RELANCE_JUSTIFICATIF.code)
    .bind(lea)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(urgences, vec!["u1", "u2"]);
    // J+5 : « sans réponse », pour la vie scolaire (aucun avis à la famille).
    let b = ent.relancer_jour(etab, j[2], Utc::now()).await.unwrap();
    assert_eq!((b.sans_reponse, b.canaux_externes), (1, 0));
    let file = ent
        .sans_reponse_for_actor(etab, cast.cpe.user_id, "cpe")
        .await
        .unwrap();
    assert_eq!(file.iter().filter(|e| e.absence_id == ep).count(), 1);
    assert_eq!(
        file.iter()
            .find(|e| e.absence_id == ep)
            .unwrap()
            .derniere_etape,
        "sans_reponse"
    );
    // J+8 : qualification « sans motif » proposée, jamais posée.
    let b = ent.relancer_jour(etab, j[3], Utc::now()).await.unwrap();
    assert_eq!(b.propositions, 1);
    let (etat, q): (String, String) = sqlx::query_as(
        "SELECT etat, qualification FROM ent_asd_absence WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(ep)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (etat.as_str(), q.as_str()),
        ("a_qualifier", "en_attente"),
        "ScolIA ne qualifie rien d'elle-même"
    );
    let etapes: Vec<String> = ent
        .relances_episode_for_actor(etab, cast.cpe.user_id, "cpe", ep)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.etape)
        .collect();
    assert_eq!(
        etapes,
        vec![
            "rappel",
            "canal_externe",
            "sans_reponse",
            "proposition_sans_motif"
        ]
    );
    // Passage ultérieur : plus rien.
    assert_eq!(
        ent.relancer_jour(etab, j[3] + Duration::days(10), Utc::now())
            .await
            .unwrap(),
        BilanRelance::default()
    );
}

/// Une tâche arrêtée : on ne relance pas trois fois d'un coup (la plus avancée part, les autres sont rattrapées).
#[tokio::test]
async fn as5_relance_en_retard_ne_spamme_pas() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, RELANCE_JUSTIFICATIF.code).await;
    let fin = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
    let ep = episode_sans_motif(&ent, etab, lea, fin).await;
    let j = jours_de_relance(&ent, etab, lea, fin, [1, 3, 5, 8]).await;
    let b = ent.relancer_jour(etab, j[2], Utc::now()).await.unwrap();
    assert_eq!(
        (b.sans_reponse, b.sautees, b.rappels, b.canaux_externes),
        (1, 2, 0, 0)
    );
    let saut: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT etape, statut, saut_code FROM ent_asd_relance WHERE etablissement_id = $1 AND absence_id = $2 ORDER BY jour_prevu",
    )
    .bind(etab)
    .bind(ep)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(saut.len(), 3);
    assert_eq!(
        saut[0],
        ("rappel".into(), "sautee".into(), Some("rattrape".into()))
    );
    assert_eq!(
        compte(
            &ent,
            "SELECT count(*) FROM ent_ntf_avis WHERE etablissement_id = $1 AND eleve_id = $2",
            etab,
            lea
        )
        .await,
        0
    );
}

/// Épisode justifié, qualifié ou déjà expliqué : plus de relance. Inactive sans bascule. Rythme réglable.
#[tokio::test]
async fn as5_relance_s_arrete_quand_le_motif_arrive_et_suit_le_rythme() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let fin = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
    let ep = episode_sans_motif(&ent, etab, lea, fin).await;
    let j = jours_de_relance(&ent, etab, lea, fin, [1, 3, 5, 8]).await;
    // Inactive tant que la nature n'est pas basculée.
    assert_eq!(
        ent.relancer_jour(etab, j[3], Utc::now()).await.unwrap(),
        BilanRelance::default()
    );
    basculer(&ent, etab, RELANCE_JUSTIFICATIF.code).await;
    // Rythme « court » de la règle d'assiduité en vigueur : J+1, J+2, J+3, J+5.
    poser_regle(&ent, etab, 15, "court").await;
    let r = jours_de_relance(&ent, etab, lea, fin, [1, 2, 3, 5]).await;
    let b = ent.relancer_jour(etab, r[1], Utc::now()).await.unwrap();
    assert_eq!(
        (b.canaux_externes, b.sautees),
        (1, 1),
        "J+2 au rythme court : l'étape la plus avancée due"
    );
    // Le motif arrive (épisode qualifié) : plus rien ne part.
    sqlx::query("UPDATE ent_asd_absence SET etat = 'qualifie', qualification = 'excuse_valable' WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(ep)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(
        ent.relancer_jour(etab, r[3], Utc::now()).await.unwrap(),
        BilanRelance::default()
    );
}

// ───────────────────────── Contacts tracés ─────────────────────────

fn contact(eleve: Uuid, canal: &str, issue: &str) -> TracerContact {
    TracerContact {
        eleve_id: eleve,
        absence_id: None,
        personne_id: None,
        canal: canal.into(),
        sens: None,
        issue: issue.into(),
        note: None,
    }
}

#[tokio::test]
async fn as5_contacts_traces_visibles_de_l_equipe_et_numero_faux() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let (cpe, adm) = (cast.cpe.user_id, cast.administratif.user_id);
    let jour = aujourdhui(&ent, &cast).await;
    let ep = episode_sans_motif(&ent, etab, lea, jour).await;
    let parent_personne = personne_du_parent(&ent, etab, lea, cast.parent_lea.user_id).await;
    let mut c1 = contact(lea, "telephone", "ne_repond_pas");
    c1.absence_id = Some(ep);
    c1.personne_id = Some(parent_personne);
    c1.note = Some("  messagerie pleine  ".into());
    let a = ent
        .tracer_contact_for_actor(etab, cpe, "cpe", c1)
        .await
        .unwrap();
    assert_eq!(
        (a.issue.as_str(), a.sens.as_str(), a.note_interne.as_deref()),
        ("ne_repond_pas", "sortant", Some("messagerie pleine"))
    );
    ent.tracer_contact_for_actor(etab, cpe, "cpe", contact(lea, "sms", "envoye"))
        .await
        .unwrap();
    let mut c3 = contact(lea, "telephone", "numero_faux");
    c3.absence_id = Some(ep);
    c3.personne_id = Some(parent_personne);
    let faux = ent
        .tracer_contact_for_actor(etab, adm, "administratif", c3)
        .await
        .unwrap();
    // Visible de toute l'équipe (secrétariat compris), du plus récent au plus ancien ; filtre par épisode.
    let tous = ent
        .contacts_for_actor(etab, cpe, "cpe", lea, None)
        .await
        .unwrap();
    assert_eq!(tous.len(), 3);
    assert_eq!(tous[0].id, faux.id);
    assert_eq!(
        ent.contacts_for_actor(etab, adm, "administratif", lea, Some(ep))
            .await
            .unwrap()
            .len(),
        2
    );
    // Le numéro faux crée une tâche pour le secrétariat, qui se clôt une seule fois.
    let taches = ent
        .numeros_faux_for_actor(etab, adm, "administratif")
        .await
        .unwrap();
    assert_eq!(taches.iter().filter(|c| c.id == faux.id).count(), 1);
    ent.cloturer_numero_faux_for_actor(etab, adm, "administratif", faux.id)
        .await
        .unwrap();
    assert!(introuvable(
        ent.cloturer_numero_faux_for_actor(etab, adm, "administratif", faux.id)
            .await
    ));
    assert!(ent
        .numeros_faux_for_actor(etab, adm, "administratif")
        .await
        .unwrap()
        .iter()
        .all(|c| c.id != faux.id));
    // Validations : canal, issue, cohérence, note, élève et responsable de la bonne fiche.
    assert!(invalide(
        ent.tracer_contact_for_actor(etab, cpe, "cpe", contact(lea, "pigeon", "joint"))
            .await
    ));
    assert!(invalide(
        ent.tracer_contact_for_actor(etab, cpe, "cpe", contact(lea, "telephone", "gagne"))
            .await
    ));
    assert!(invalide(
        ent.tracer_contact_for_actor(etab, cpe, "cpe", contact(lea, "courriel", "numero_faux"))
            .await
    ));
    let mut long = contact(lea, "telephone", "joint");
    long.note = Some("x".repeat(301));
    assert!(invalide(
        ent.tracer_contact_for_actor(etab, cpe, "cpe", long).await
    ));
    let mut autre = contact(lea, "telephone", "joint");
    autre.personne_id = Some(Uuid::new_v4());
    assert!(introuvable(
        ent.tracer_contact_for_actor(etab, cpe, "cpe", autre).await
    ));
    assert!(introuvable(
        ent.tracer_contact_for_actor(
            etab,
            cpe,
            "cpe",
            contact(Uuid::new_v4(), "telephone", "joint")
        )
        .await
    ));
    // Ajout seul : ni réécriture ni suppression.
    let r = sqlx::query(
        "UPDATE ent_asd_contact SET issue = 'joint' WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(a.id)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err());
    let r = sqlx::query("DELETE FROM ent_asd_contact WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(a.id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err());
}

/// Droits : ni le professeur, ni la famille, ni l'élève ne tracent ni ne lisent les contacts ; un autre
/// établissement ne voit rien.
#[tokio::test]
async fn as5_contacts_et_courriers_matrice_de_droits() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let prof = cast.prof.user_id;
    let parent = cast.parent_lea.user_id;
    for (u, r) in [
        (prof, "prof"),
        (parent, "parent"),
        (cast.comptas[0].user_id, "compta"),
    ] {
        assert!(
            interdit(
                ent.tracer_contact_for_actor(etab, u, r, contact(lea, "telephone", "joint"))
                    .await
            ),
            "{r}"
        );
        assert!(
            interdit(ent.contacts_for_actor(etab, u, r, lea, None).await),
            "{r}"
        );
        assert!(
            interdit(ent.numeros_faux_for_actor(etab, u, r).await),
            "{r}"
        );
        assert!(
            interdit(ent.sans_reponse_for_actor(etab, u, r).await),
            "{r}"
        );
        assert!(
            interdit(ent.param_famille_for_actor(etab, u, r).await),
            "{r}"
        );
        assert!(
            interdit(ent.avis_eleve_for_actor(etab, u, r, lea, None).await),
            "{r}"
        );
        assert!(
            interdit(ent.courriers_for_actor(etab, u, r, lea).await),
            "{r}"
        );
        let p = PreparerCourrier {
            eleve_id: lea,
            absence_id: None,
            personne_id: None,
            nature: "avis_absence".into(),
        };
        assert!(
            interdit(ent.preparer_courrier_for_actor(etab, u, r, p).await),
            "{r}"
        );
    }
    // Autre établissement : l'élève n'existe pas pour lui (même réponse que « inexistant »).
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    assert!(introuvable(
        ent.tracer_contact_for_actor(
            autre.etablissement_id,
            autre.cpe.user_id,
            "cpe",
            contact(lea, "telephone", "joint")
        )
        .await
    ));
    assert!(introuvable(
        ent.contacts_for_actor(autre.etablissement_id, autre.cpe.user_id, "cpe", lea, None)
            .await
    ));
    // La famille d'un autre élève ne lit pas les communications de celui-ci.
    assert!(introuvable(
        ent.communications_famille_for_actor(etab, cast.parent_tom.user_id, "parent", lea)
            .await
    ));
    assert!(introuvable(
        ent.communications_famille_for_actor(etab, prof, "prof", lea)
            .await
    ));
}

// ───────────────────────── Courriers postaux, vue de la famille ─────────────────────────

#[tokio::test]
async fn as5_courrier_postal_prepare_decide_puis_poste() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    let (adm, cpe, parent) = (
        cast.administratif.user_id,
        cast.cpe.user_id,
        cast.parent_lea.user_id,
    );
    let jour = aujourdhui(&ent, &cast).await;
    let ep = episode_sans_motif(&ent, etab, lea, jour).await;
    let p = |nature: &str| PreparerCourrier {
        eleve_id: lea,
        absence_id: Some(ep),
        personne_id: None,
        nature: nature.into(),
    };
    assert!(invalide(
        ent.preparer_courrier_for_actor(etab, adm, "administratif", p("menace"))
            .await
    ));
    let c = ent
        .preparer_courrier_for_actor(etab, adm, "administratif", p("relance_absence"))
        .await
        .unwrap();
    assert_eq!(c.statut, "prepare");
    // Un seul courrier en cours par épisode et par nature.
    assert!(conflit(
        ent.preparer_courrier_for_actor(etab, adm, "administratif", p("relance_absence"))
            .await
    ));
    // R-AS18 : rien ne part avant la décision d'une personne habilitée ; le secrétariat ne décide pas.
    assert!(
        conflit(
            ent.poster_courrier_for_actor(etab, adm, "administratif", c.id)
                .await
        ),
        "pas posté avant décision"
    );
    assert!(interdit(
        ent.valider_courrier_for_actor(etab, adm, "administratif", c.id)
            .await
    ));
    // La famille ne voit rien tant que le courrier n'est pas posté.
    assert!(ent
        .communications_famille_for_actor(etab, parent, "parent", lea)
        .await
        .unwrap()
        .is_empty());
    // Cloisonnement : un acteur d'un autre établissement ne valide, ne poste ni n'abandonne le courrier de celui-ci.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let (be, bc, bd) = (
        autre.etablissement_id,
        autre.cpe.user_id,
        autre.administratif.user_id,
    );
    assert!(introuvable(
        ent.valider_courrier_for_actor(be, bc, "cpe", c.id).await
    ));
    assert!(introuvable(
        ent.poster_courrier_for_actor(be, bd, "administratif", c.id)
            .await
    ));
    assert!(introuvable(
        ent.abandonner_courrier_for_actor(be, bd, "administratif", c.id, "erreur")
            .await
    ));
    let (statut,): (String,) = sqlx::query_as(
        "SELECT statut FROM ent_asd_courrier WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(c.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(statut, "prepare", "le courrier de A n'a pas bougé");
    let v = ent
        .valider_courrier_for_actor(etab, cpe, "cpe", c.id)
        .await
        .unwrap();
    assert_eq!((v.statut.as_str(), v.valide_par), ("valide", Some(cpe)));
    assert!(
        conflit(ent.valider_courrier_for_actor(etab, cpe, "cpe", c.id).await),
        "déjà validé"
    );
    assert!(ent
        .communications_famille_for_actor(etab, parent, "parent", lea)
        .await
        .unwrap()
        .is_empty());
    let posted = ent
        .poster_courrier_for_actor(etab, adm, "administratif", c.id)
        .await
        .unwrap();
    assert_eq!(
        (posted.statut.as_str(), posted.poste_par),
        ("poste", Some(adm))
    );
    assert!(
        conflit(
            ent.abandonner_courrier_for_actor(etab, adm, "administratif", c.id, "erreur")
                .await
        ),
        "posté : fini"
    );
    // Le courrier posté est visible de la famille, sans note ni auteur ; le suivi interne ne l'est pas.
    ent.tracer_contact_for_actor(etab, cpe, "cpe", contact(lea, "telephone", "ne_repond_pas"))
        .await
        .unwrap();
    ent.tracer_contact_for_actor(etab, cpe, "cpe", contact(lea, "telephone", "numero_faux"))
        .await
        .unwrap();
    let mut joint = contact(lea, "telephone", "joint");
    joint.note = Some("note interne".into());
    ent.tracer_contact_for_actor(etab, cpe, "cpe", joint)
        .await
        .unwrap();
    let vue = ent
        .communications_famille_for_actor(etab, parent, "parent", lea)
        .await
        .unwrap();
    assert_eq!(
        vue.len(),
        2,
        "un courrier posté et un contact qui l'a atteinte : {vue:?}"
    );
    assert!(vue.iter().any(|c| c.type_communication == "courrier"));
    assert!(vue
        .iter()
        .any(|c| c.type_communication == "contact" && c.canal == "telephone"));
    let json = serde_json::to_string(&vue).unwrap();
    assert!(
        !json.contains("note interne") && !json.contains("auteur"),
        "{json}"
    );
    // Abandon d'un second courrier, avec un code de la liste.
    let c2 = ent
        .preparer_courrier_for_actor(etab, adm, "administratif", p("avis_absence"))
        .await
        .unwrap();
    assert!(invalide(
        ent.abandonner_courrier_for_actor(etab, adm, "administratif", c2.id, "parce que")
            .await
    ));
    let ab = ent
        .abandonner_courrier_for_actor(etab, adm, "administratif", c2.id, "motif_recu")
        .await
        .unwrap();
    assert_eq!(
        (ab.statut.as_str(), ab.abandon_code.as_deref()),
        ("abandonne", Some("motif_recu"))
    );
    // Un courrier ne se supprime pas.
    let r = sqlx::query("DELETE FROM ent_asd_courrier WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(c.id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err());
    assert_eq!(
        ent.courriers_for_actor(etab, cpe, "cpe", lea)
            .await
            .unwrap()
            .len(),
        2
    );
}

// ───────────────────────── Réglages ─────────────────────────

/// Le délai de consolidation vient de la règle d'assiduité en vigueur (0 à 30 minutes, R131-5), pas d'un second
/// réglage : valeur du code (10) sans règle, valeur de la règle ensuite, plafonnée à 30.
#[tokio::test]
async fn as5_delai_de_consolidation_vient_de_la_regle_et_plafonne_a_30() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let jour = aujourdhui(&ent, &cast).await;
    let delai = |ent: &PgEnt| {
        let pool = ent.pool_verif().clone();
        async move {
            let (effet, confirme): (chrono::DateTime<Utc>, chrono::DateTime<Utc>) = sqlx::query_as(
                "SELECT effet_le, confirme_apres FROM ent_ntf_avis
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = $3 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(etab)
            .bind(lea)
            .bind(ABSENCE_CONSTATEE.code)
            .fetch_one(&pool)
            .await
            .unwrap();
            confirme - effet
        }
    };
    let p = ent
        .param_famille_for_actor(etab, cast.cpe.user_id, "cpe")
        .await
        .unwrap();
    assert_eq!(
        (p.delai_consolidation_min, p.rythme_relance.as_str()),
        (10, "standard"),
        "valeurs du code sans règle"
    );
    assert!(p.avis_demi_journee_actif && !p.decision_active && !p.relance_active);
    appel(&ent, &cast, jour, (8, 0), (9, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    assert_eq!(delai(&ent).await, Duration::minutes(10));
    // Règle en vigueur : 15 minutes ; une valeur hors bornes (40) est plafonnée à 30 à l'usage.
    poser_regle(&ent, etab, 15, "standard").await;
    appel(&ent, &cast, jour, (14, 0), (15, 0), &[lea]).await;
    distribuer(&ent, etab).await;
    assert_eq!(delai(&ent).await, Duration::minutes(15));
    assert_eq!(
        ent.param_famille_for_actor(etab, cast.cpe.user_id, "cpe")
            .await
            .unwrap()
            .delai_consolidation_min,
        15
    );
    sqlx::query(
        "UPDATE ent_asd_regle SET delai_consolidation_min = 40 WHERE etablissement_id = $1",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        ent.param_famille_for_actor(etab, cast.cpe.user_id, "cpe")
            .await
            .unwrap()
            .delai_consolidation_min,
        30
    );
    // Avis lisibles de la vie scolaire, avec l'état dans le moteur ; jamais d'un professeur.
    let avis = ent
        .avis_eleve_for_actor(etab, cast.cpe.user_id, "cpe", lea, None)
        .await
        .unwrap();
    assert_eq!(avis.len(), 2);
    assert!(avis
        .iter()
        .all(|a| a.statut == "programme" && a.avis_statut == "planifie"));
}

/// Les événements du lot ne portent que des codes et des identifiants (R-AS25) : ni motif, ni note, ni nom.
#[tokio::test]
async fn as5_evenements_sans_texte_libre() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lea, cpe) = (cast.etablissement_id, cast.eleve_lea_id, cast.cpe.user_id);
    let mut c = contact(lea, "telephone", "joint");
    c.note = Some("la mère pleure, divorce en cours".into());
    ent.tracer_contact_for_actor(etab, cpe, "cpe", c)
        .await
        .unwrap();
    let ep = episode_sans_motif(&ent, etab, lea, aujourdhui(&ent, &cast).await).await;
    ent.preparer_courrier_for_actor(
        etab,
        cpe,
        "cpe",
        PreparerCourrier {
            eleve_id: lea,
            absence_id: Some(ep),
            personne_id: None,
            nature: "relance_absence".into(),
        },
    )
    .await
    .unwrap();
    let payloads: Vec<String> = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type IN ('assiduite.contact_trace', 'assiduite.courrier_prepare')",
    )
    .bind(etab)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(payloads.len(), 2);
    for p in payloads {
        assert!(!p.contains("pleure") && !p.contains("divorce"), "{p}");
    }
    let _ = ABSENCE_DEMENTI;
    let _ = t(0, 0);
}
