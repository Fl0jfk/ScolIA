//! Socle 27, **lot G4-1** : traces d'accès séparées (migration 2546) et page « Consultations du dossier ».
//! Aucune heure réelle dans les assertions (règle 33) : chaque date est injectée. Seule la reprise de la migration
//! (`now()` en SQL) lit l'horloge de la base, pour décider quelles anciennes traces entrent dans la durée de conservation.

use crate::acces_traces::{MOTIFS_PAR_DEFAUT, MOTIF_REQUIS};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::tests_rgpd_g3b2::{labo, parents_separes, personne_de};
use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

const PROF: &str = "prof";

fn le(a: i32, m: u32, j: u32, h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(a, m, j, h, 0, 0).unwrap()
}

/// L'agent (compte) a **cette** fonction du `du` au `au` ; ses autres fonctions sont annulées (date fixe).
async fn poser_fonction(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    code: &str,
    du: &str,
    au: Option<&str>,
) {
    let p = ent.pool_verif();
    let personne = personne_de(ent, etab, user).await;
    let agent: Uuid = match sqlx::query_scalar(
        "SELECT id FROM ent_agent WHERE etablissement_id = $1 AND personne_id = $2",
    )
    .bind(etab)
    .bind(personne)
    .fetch_optional(p)
    .await
    .unwrap()
    {
        Some(a) => a,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO ent_agent (id, etablissement_id, personne_id, source) VALUES ($1, $2, $3, 'saisie')")
                .bind(id)
                .bind(etab)
                .bind(personne)
                .execute(p)
                .await
                .unwrap();
            id
        }
    };
    sqlx::query(
        "INSERT INTO ent_agent_fonction (id, etablissement_id, agent_id, fonction_id, principale, du, au)
         SELECT gen_random_uuid(), $1, $2, f.id, true, $3::date, $5::date FROM ent_fonction f
          WHERE f.code = $4 AND f.etablissement_id IS NULL",
    )
    .bind(etab)
    .bind(agent)
    .bind(du)
    .bind(code)
    .bind(au)
    .execute(p)
    .await
    .unwrap();
}

async fn annuler_fonctions(ent: &PgEnt, etab: Uuid, user: Uuid) {
    let personne = personne_de(ent, etab, user).await;
    sqlx::query(
        "UPDATE ent_agent_fonction SET annule_le = '2000-01-01T00:00:00Z'
          WHERE etablissement_id = $1 AND annule_le IS NULL
            AND agent_id IN (SELECT id FROM ent_agent WHERE etablissement_id = $1 AND personne_id = $2)",
    )
    .bind(etab)
    .bind(personne)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

async fn trace_de(
    ent: &PgEnt,
    id: Uuid,
) -> Option<(
    String,
    String,
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<String>,
)> {
    sqlx::query_as(
        "SELECT fonction_code, service_code, nature, section, action, hors_perimetre, perimetre_temporaire, motif_code
           FROM acc_trace WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(ent.pool_verif())
    .await
    .unwrap()
}

async fn derniere_trace(ent: &PgEnt, etab: Uuid, acteur: Uuid, action: &str) -> Option<Uuid> {
    sqlx::query_scalar(
        "SELECT id FROM acc_trace WHERE etablissement_id = $1 AND acteur_compte_id = $2 AND action = $3
          ORDER BY le DESC LIMIT 1",
    )
    .bind(etab)
    .bind(acteur)
    .bind(action)
    .fetch_optional(ent.pool_verif())
    .await
    .unwrap()
}

// ───────────────────────────── Écriture ─────────────────────────────

#[tokio::test]
async fn g4_trace_ecrite_dans_la_transaction_avec_le_meme_identifiant_que_le_journal() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        Some(cast.eleve_lea_id),
        "identifiants",
        "Consultation des identifiants",
        false,
        le(2026, 10, 15, 9),
    )
    .await
    .unwrap();
    let (jid,): (Uuid,) = sqlx::query_as(
        "SELECT id FROM ent_journal_modif WHERE etablissement_id = $1 AND actor_user_id = $2 AND action = 'identifiants'",
    )
    .bind(etab)
    .bind(rogue)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let t = trace_de(&ent, jid)
        .await
        .expect("la trace partage l'identifiant du journal (double écriture)");
    assert_eq!(
        (t.2.as_str(), t.3.as_str(), t.4.as_str()),
        ("consultation", "identite", "identifiants")
    );
    assert!(
        !t.5 && t.6.is_none() && t.7.is_none(),
        "dans le périmètre, aucun motif"
    );
    // Le journal actuel continue d'être écrit (double écriture jusqu'à la décision 2559).
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'consultation'")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(n >= 1);
}

#[tokio::test]
async fn g4_echec_de_la_trace_refuse_la_lecture_et_n_ecrit_rien() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    let p = ent.pool_verif();
    // Déclencheur de test : refuse l'insertion des seules lignes marquées par ce test (la base est partagée).
    sqlx::query(
        "CREATE OR REPLACE FUNCTION g4_test_refus() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN IF NEW.action = 'g4_refus_test' THEN RAISE EXCEPTION 'trace refusée (test)' USING ERRCODE = '23514'; END IF;
               RETURN NEW; END $$",
    )
    .execute(p)
    .await
    .unwrap();
    sqlx::query("CREATE OR REPLACE TRIGGER g4_test_refus_trg BEFORE INSERT ON acc_trace FOR EACH ROW EXECUTE FUNCTION g4_test_refus()")
        .execute(p)
        .await
        .unwrap();
    let r = ent
        .tracer_consultation_a(
            etab,
            rogue,
            PROF,
            Some(cast.eleve_lea_id),
            "g4_refus_test",
            "Lecture refusée",
            false,
            le(2026, 10, 15, 9),
        )
        .await;
    sqlx::query("DROP TRIGGER IF EXISTS g4_test_refus_trg ON acc_trace")
        .execute(p)
        .await
        .unwrap();
    assert!(
        r.is_err(),
        "un échec d'écriture de la trace refuse la lecture (R-RG8)"
    );
    let (j, t): (i64, i64) = (
        sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND action = 'g4_refus_test'")
            .bind(etab)
            .fetch_one(p)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT count(*) FROM acc_trace WHERE etablissement_id = $1 AND action = 'g4_refus_test'")
            .bind(etab)
            .fetch_one(p)
            .await
            .unwrap(),
    );
    assert_eq!((j, t), (0, 0), "même transaction : ni journal ni trace");
}

#[tokio::test]
async fn g4_fonction_et_service_figes_apres_un_changement_de_poste() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    annuler_fonctions(&ent, etab, rogue).await;
    poser_fonction(
        &ent,
        etab,
        rogue,
        "enseignant",
        "2026-09-01",
        Some("2026-10-31"),
    )
    .await;
    poser_fonction(&ent, etab, rogue, "cpe", "2026-11-01", None).await;
    let eleve = Some(cast.eleve_lea_id);
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        eleve,
        "scolarite",
        "Scolarité (octobre)",
        false,
        le(2026, 10, 15, 9),
    )
    .await
    .unwrap();
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        eleve,
        "coordonnees",
        "Coordonnées (novembre)",
        false,
        le(2026, 11, 20, 9),
    )
    .await
    .unwrap();
    let avant = derniere_trace(&ent, etab, rogue, "scolarite")
        .await
        .unwrap();
    let apres = derniere_trace(&ent, etab, rogue, "coordonnees")
        .await
        .unwrap();
    let (a, b) = (
        trace_de(&ent, avant).await.unwrap(),
        trace_de(&ent, apres).await.unwrap(),
    );
    assert_eq!(
        a.0, "enseignant",
        "la consultation d'octobre garde la fonction d'alors"
    );
    assert_eq!(a.1, "enseignement");
    assert_eq!(b.0, "cpe");
    assert_eq!(b.1, "education");
    // Un changement de poste ultérieur ne réécrit rien (ajout seul).
    annuler_fonctions(&ent, etab, rogue).await;
    poser_fonction(&ent, etab, rogue, "comptable", "2026-12-01", None).await;
    assert_eq!(trace_de(&ent, avant).await.unwrap().0, "enseignant");
}

// ───────────────────────────── Partitions ─────────────────────────────

#[tokio::test]
async fn g4_partition_du_mois_suivant_creee_rls_active_sur_la_partition_interrogee_directement() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    // Mois courant et deux suivants, créés à l'avance pour une date injectée.
    ent.acces_partitions_assurer_a(le(2030, 5, 10, 8))
        .await
        .unwrap();
    for nom in ["acc_trace_203005", "acc_trace_203006", "acc_trace_203007"] {
        let existe: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
            .bind(format!("public.{nom}"))
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
        assert!(existe, "{nom}");
    }
    // Une trace du mois suivant tombe dans SA partition.
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        Some(cast.eleve_lea_id),
        "scolarite",
        "Scolarité",
        false,
        le(2030, 6, 15, 9),
    )
    .await
    .unwrap();
    let (partition,): (String,) = sqlx::query_as(
        "SELECT tableoid::regclass::text FROM acc_trace WHERE etablissement_id = $1 AND le = $2",
    )
    .bind(etab)
    .bind(le(2030, 6, 15, 9))
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(partition, "acc_trace_203006");
    // La RLS de la partition interrogée directement est vérifiée avec un rôle non propriétaire en mode strict :
    // `tests_rls::rls_partition_de_traces_interrogee_directement`.
    // Ajout seul, sur le parent comme sur la partition ; TRUNCATE aussi.
    for sql in [
        "UPDATE acc_trace SET action = 'y' WHERE etablissement_id = $1",
        "DELETE FROM acc_trace_203006 WHERE etablissement_id = $1",
        "DELETE FROM acc_trace WHERE etablissement_id = $1",
    ] {
        let r = sqlx::query(sql).bind(etab).execute(ent.pool_verif()).await;
        assert!(r.is_err(), "{sql}");
    }
    for sql in ["TRUNCATE acc_trace_203006", "TRUNCATE acc_trace"] {
        let r = sqlx::query(sql).execute(ent.pool_verif()).await;
        assert!(r.is_err(), "{sql}");
    }
    // Une date ancienne crée sa partition à la volée (pas d'insertion refusée faute de partition).
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        Some(cast.eleve_lea_id),
        "scolarite",
        "Scolarité",
        false,
        le(2020, 2, 3, 9),
    )
    .await
    .unwrap();
    let ancien: bool =
        sqlx::query_scalar("SELECT to_regclass('public.acc_trace_202002') IS NOT NULL")
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(ancien);
}

// ───────────────────────────── Reprise ─────────────────────────────

#[tokio::test]
async fn g4_reprise_des_anciennes_traces_par_copie_nombre_egal_sans_doublon_et_journal_intact() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    let p = ent.pool_verif();
    // Quatre lignes de journal « consultation » plus anciennes que cette session (la durée est lue par la base).
    for (i, action) in [
        "etat_civil",
        "coordonnees",
        "pastilles_sensibles",
        "sante_dossier",
    ]
    .iter()
    .enumerate()
    {
        sqlx::query(
            "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle, payload, visible_famille,
                                             actor_user_id, created_at)
             VALUES ($1, $2, $3, 'consultation', $4, 'ancienne', jsonb_build_object('role', 'prof'), false, $5,
                     now() - make_interval(days => 10 + $6::int))",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(cast.eleve_lea_id)
        .bind(*action)
        .bind(rogue)
        .bind(i32::try_from(i).unwrap())
        .execute(p)
        .await
        .unwrap();
    }
    let avant: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_one(p)
            .await
            .unwrap();
    // La migration se rejoue (idempotente) : la reprise copie les nouvelles lignes, deux fois de suite sans doublon.
    for _ in 0..2 {
        sqlx::raw_sql(include_str!(
            "../../../migrations/2546_rgpd_g4_traces_acces.sql"
        ))
        .execute(p)
        .await
        .unwrap();
    }
    let (journal, traces, communs): (i64, i64, i64) = (
        sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1 AND domaine = 'consultation' AND actor_user_id IS NOT NULL")
            .bind(etab).fetch_one(p).await.unwrap(),
        sqlx::query_scalar("SELECT count(*) FROM acc_trace WHERE etablissement_id = $1").bind(etab).fetch_one(p).await.unwrap(),
        sqlx::query_scalar("SELECT count(*) FROM acc_trace t JOIN ent_journal_modif j ON j.id = t.id WHERE t.etablissement_id = $1")
            .bind(etab).fetch_one(p).await.unwrap(),
    );
    assert_eq!(
        journal, traces,
        "nombre égal : tout le journal de consultation est repris"
    );
    assert_eq!(
        communs, traces,
        "chaque trace reprise garde l'identifiant de sa ligne de journal"
    );
    let apres: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_journal_modif WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_one(p)
            .await
            .unwrap();
    assert_eq!(
        avant, apres,
        "rien n'est supprimé du journal (2559 reste réservée)"
    );
    // Classement : la santé n'est pas affichable ; les sections protégées portent un code générique.
    let (sante, protegee): (bool, bool) = (
        sqlx::query_scalar("SELECT affichable FROM acc_trace WHERE etablissement_id = $1 AND action = 'sante_dossier'").bind(etab).fetch_one(p).await.unwrap(),
        sqlx::query_scalar("SELECT protegee FROM acc_trace WHERE etablissement_id = $1 AND action = 'pastilles_sensibles'").bind(etab).fetch_one(p).await.unwrap(),
    );
    assert!(!sante, "R-RG29");
    assert!(protegee);
    let section: String = sqlx::query_scalar("SELECT section FROM acc_trace WHERE etablissement_id = $1 AND action = 'pastilles_sensibles'").bind(etab).fetch_one(p).await.unwrap();
    assert_eq!(section, "section_protegee");
}

// ───────────────────────────── Hors périmètre ─────────────────────────────

#[tokio::test]
async fn g4_hors_perimetre_motif_exige_puis_lecture_servie_et_tracee_avec_le_motif() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id; // enseigne en 4B : Draco (5A) est hors de son périmètre
    let draco = cast.eleve_tom_id;
    let t = le(2026, 10, 15, 9);
    // Dans son périmètre : aucun motif demandé.
    let harry = cast.eleve_lea_id;
    ent.get_eleve_vue_motive_for_actor(etab, rogue, PROF, harry, None, true, t)
        .await
        .unwrap()
        .unwrap();
    // Hors périmètre : « motif requis », et aucune trace.
    let r = ent
        .get_eleve_vue_motive_for_actor(etab, rogue, PROF, draco, None, true, t)
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(m)) if m == MOTIF_REQUIS),
        "{r:?}"
    );
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM acc_trace WHERE etablissement_id = $1 AND sujet_eleve_id = $2",
    )
    .bind(etab)
    .bind(draco)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0, "le refus ne laisse aucune trace");
    // Un motif hors liste est refusé ; un motif de la liste est servi et tracé.
    let r = ent
        .get_eleve_vue_motive_for_actor(etab, rogue, PROF, draco, Some("pour_voir"), true, t)
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    ent.get_eleve_vue_motive_for_actor(etab, rogue, PROF, draco, Some("urgence"), true, t)
        .await
        .unwrap()
        .unwrap();
    let (hors, motif): (bool, Option<String>) = sqlx::query_as(
        "SELECT hors_perimetre, motif_code FROM acc_trace WHERE etablissement_id = $1 AND sujet_eleve_id = $2 AND acteur_compte_id = $3",
    )
    .bind(etab).bind(draco).bind(rogue).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(hors);
    assert_eq!(motif.as_deref(), Some("urgence"));
    // La famille voit le motif (libellé), jamais le nom de l'agent.
    let page = ent
        .acces_consultations_a(etab, cast.parent_tom.user_id, "parent", t)
        .await
        .unwrap();
    let ligne = &page.dossiers[0].jours[0].lignes[0];
    assert_eq!(ligne.motif.as_deref(), Some("Urgence"));
    assert!(!ligne.fonction.contains("Rogue") && !ligne.service.contains("Rogue"));
}

#[tokio::test]
async fn g4_remplacant_dans_son_perimetre_temporaire_aucun_motif_demande() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let jour = chrono::NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
    let titulaire = cast.profs[1].user_id; // enseigne en 5A (Draco)
    let remplacant = cast.profs[3].user_id;
    let creneau = ent
        .seed_creneau_du_jour(
            etab,
            "5A",
            "Histoire",
            titulaire,
            jour,
            chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            chrono::NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            None,
        )
        .await
        .unwrap();
    // Hors périmètre sans remplacement ce jour-là : motif exigé.
    let t = le(2026, 10, 14, 12);
    let r = ent
        .acces_exiger_motif_fiche_a(etab, remplacant, PROF, cast.eleve_tom_id, None, t)
        .await;
    assert!(
        matches!(r, Err(EntDbError::ConflitDossier(m)) if m == MOTIF_REQUIS),
        "{r:?}"
    );
    sqlx::query(
        "INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, remplacant_user_id, motif, created_by_user_id)
         VALUES ($1, $2, $3, $4, 'remplacement', $5, 'G4', $6)",
    )
    .bind(Uuid::new_v4()).bind(etab).bind(creneau.id).bind(jour).bind(remplacant).bind(titulaire)
    .execute(ent.pool_verif()).await.unwrap();
    // Couvert par le remplacement : pas « hors périmètre », pas de motif ; la trace le dit.
    ent.acces_exiger_motif_fiche_a(etab, remplacant, PROF, cast.eleve_tom_id, None, t)
        .await
        .unwrap();
    ent.tracer_consultation_a(
        etab,
        remplacant,
        PROF,
        Some(cast.eleve_tom_id),
        "scolarite",
        "Scolarité",
        false,
        t,
    )
    .await
    .unwrap();
    let (hors, temp, motif): (bool, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT hors_perimetre, perimetre_temporaire, motif_code FROM acc_trace WHERE etablissement_id = $1 AND acteur_compte_id = $2 ORDER BY le DESC LIMIT 1",
    )
    .bind(etab).bind(remplacant).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(!hors);
    assert_eq!(temp.as_deref(), Some("remplacement"));
    assert!(motif.is_none());
    // Le lendemain, plus de remplacement : de nouveau hors périmètre.
    let r = ent
        .acces_exiger_motif_fiche_a(
            etab,
            remplacant,
            PROF,
            cast.eleve_tom_id,
            None,
            le(2026, 10, 15, 12),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
}

#[tokio::test]
async fn g4_conseil_de_classe_et_sortie_sont_des_perimetres_temporaires() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let agent = cast.profs[0].user_id; // 4B ; Draco est en 5A
    let draco = cast.eleve_tom_id;
    let p = ent.pool_verif();
    let t = le(2026, 10, 16, 12);
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let jour = chrono::NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
    let hors = ent
        .perimetre_acteur(&mut tx, etab, agent, PROF, draco, jour)
        .await
        .unwrap();
    assert_eq!(hors, crate::acces_traces::Perimetre::Hors);
    drop(tx);
    sqlx::query(
        "INSERT INTO ent_conseil_classe (id, etablissement_id, classe, periode_libelle, jour, heure, statut, created_by_user_id)
         VALUES ($1, $2, '5A', 'T1', $3, '17:00', 'planifie', $4)",
    )
    .bind(Uuid::new_v4()).bind(etab).bind(jour).bind(agent)
    .execute(p).await.unwrap();
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let r = ent
        .perimetre_acteur(&mut tx, etab, agent, PROF, draco, jour)
        .await
        .unwrap();
    assert_eq!(
        r,
        crate::acces_traces::Perimetre::Temporaire("conseil_classe")
    );
    let lendemain = chrono::NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();
    let r = ent
        .perimetre_acteur(&mut tx, etab, agent, PROF, draco, lendemain)
        .await
        .unwrap();
    assert_eq!(
        r,
        crate::acces_traces::Perimetre::Hors,
        "le conseil ne couvre que son jour"
    );
    drop(tx);
    let _ = t;
}

// ───────────────────────────── Page « Consultations du dossier » ─────────────────────────────

#[tokio::test]
async fn g4_page_famille_fonction_et_service_sans_nom_regroupee_par_jour_sans_sante_ni_conformite()
{
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    let draco = Some(cast.eleve_tom_id);
    annuler_fonctions(&ent, etab, rogue).await;
    poser_fonction(&ent, etab, rogue, "enseignant", "2026-01-01", None).await;
    let motif = Some("suivi_pedagogique");
    // Deux jours ; une lecture hors périmètre motivée (fiche), une santé, une lecture de la conformité, une liste.
    ent.tracer_consultation_motive_a(
        etab,
        rogue,
        PROF,
        draco,
        "scolarite",
        "Scolarité",
        motif,
        le(2026, 11, 3, 9),
    )
    .await
    .unwrap();
    ent.tracer_consultation_motive_a(
        etab,
        rogue,
        PROF,
        draco,
        "identifiants",
        "Identifiants",
        motif,
        le(2026, 11, 3, 14),
    )
    .await
    .unwrap();
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        draco,
        "scolarite",
        "Scolarité",
        false,
        le(2026, 11, 4, 9),
    )
    .await
    .unwrap();
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        draco,
        "sante_dossier",
        "Santé",
        false,
        le(2026, 11, 4, 10),
    )
    .await
    .unwrap();
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        draco,
        "extrait_droits",
        "Extrait",
        false,
        le(2026, 11, 4, 11),
    )
    .await
    .unwrap();
    let page = ent
        .acces_consultations_a(etab, cast.parent_tom.user_id, "parent", le(2026, 11, 20, 9))
        .await
        .unwrap();
    assert_eq!(
        page.dossiers.len(),
        1,
        "un dossier par enfant dont le parent est titulaire"
    );
    let d = &page.dossiers[0];
    assert_eq!(d.sujet_type, "eleve");
    assert_eq!(d.total, 3, "ni santé (R-RG29) ni lecture de la conformité");
    assert_eq!(d.jours.len(), 2);
    assert!(
        d.jours[0].jour > d.jours[1].jour,
        "les jours les plus récents d'abord"
    );
    let l = &d.jours[0].lignes[0];
    assert_eq!(l.fonction, "Enseignant");
    assert_eq!(l.service, "Enseignement");
    assert_eq!(l.section, "Scolarité");
    assert_eq!(l.nature, "Consultation");
    // Le nom de l'agent n'existe nulle part dans la page (§11.3, S-549).
    let tout = format!("{page:?}");
    assert!(
        !tout.contains("Rogue") && !tout.contains(&rogue.to_string()),
        "aucun nom ni identifiant d'agent"
    );
    // Une famille qui n'est pas titulaire de Draco ne voit rien de lui.
    let autre = ent
        .acces_consultations_a(etab, cast.parent_lea.user_id, "parent", le(2026, 11, 20, 9))
        .await
        .unwrap();
    assert!(autre
        .dossiers
        .iter()
        .all(|d| d.sujet_id != cast.eleve_tom_id));
}

#[tokio::test]
async fn g4_page_famille_seulement_les_dates_ou_le_parent_est_titulaire() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    let draco = Some(cast.eleve_tom_id);
    sqlx::query("UPDATE ent_eleve_lien SET date_debut = '2026-11-10' WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(cast.eleve_tom_id).bind(cast.parent_tom.user_id)
        .execute(ent.pool_verif()).await.unwrap();
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        draco,
        "scolarite",
        "Avant",
        false,
        le(2026, 11, 5, 9),
    )
    .await
    .unwrap();
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        draco,
        "scolarite",
        "Après",
        false,
        le(2026, 11, 12, 9),
    )
    .await
    .unwrap();
    let page = ent
        .acces_consultations_a(etab, cast.parent_tom.user_id, "parent", le(2026, 11, 20, 9))
        .await
        .unwrap();
    let nb: usize = page
        .dossiers
        .iter()
        .flat_map(|d| &d.jours)
        .map(|j| j.lignes.len())
        .sum();
    assert_eq!(
        nb, 1,
        "la consultation du 5 novembre précède la titularité du parent : non visible"
    );
}

#[tokio::test]
async fn g4_parent_vise_par_la_protection_ne_voit_pas_la_trace_coordonnees_protegees() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let _autre = parents_separes(&ent, &cast, true).await; // parent_tom : protégé ; parent_lea : l'autre parent
    let rogue = cast.profs[0].user_id;
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        Some(cast.eleve_lea_id),
        "coordonnees",
        "Coordonnées",
        false,
        le(2026, 11, 3, 9),
    )
    .await
    .unwrap();
    ent.tracer_consultation_a(
        etab,
        rogue,
        PROF,
        Some(cast.eleve_lea_id),
        "scolarite",
        "Scolarité",
        false,
        le(2026, 11, 3, 10),
    )
    .await
    .unwrap();
    let s: String = sqlx::query_scalar(
        "SELECT section FROM acc_trace WHERE etablissement_id = $1 AND action = 'coordonnees'",
    )
    .bind(etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(s, "coordonnees_protegees", "code générique");
    let t = le(2026, 11, 20, 9);
    let vise = ent
        .acces_consultations_a(etab, cast.parent_lea.user_id, "parent", t)
        .await
        .unwrap();
    let lignes: Vec<&crate::acces_traces::LigneConsultation> = vise
        .dossiers
        .iter()
        .filter(|d| d.sujet_id == cast.eleve_lea_id)
        .flat_map(|d| d.jours.iter().flat_map(|j| &j.lignes))
        .collect();
    assert_eq!(lignes.len(), 1, "le parent visé ne voit que la scolarité");
    assert_eq!(lignes[0].section, "Scolarité");
    let protege = ent
        .acces_consultations_a(etab, cast.parent_tom.user_id, "parent", t)
        .await
        .unwrap();
    let n: usize = protege
        .dossiers
        .iter()
        .filter(|d| d.sujet_id == cast.eleve_lea_id)
        .flat_map(|d| d.jours.iter().flat_map(|j| &j.lignes))
        .count();
    assert_eq!(
        n, 2,
        "la personne protégée voit ses propres traces, sous le code générique"
    );
}

#[tokio::test]
async fn g4_support_de_l_editeur_sans_fonction_avec_la_mention_et_dossier_de_l_agent() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let p = ent.pool_verif();
    ent.acces_partitions_assurer_a(le(2026, 11, 1, 9))
        .await
        .unwrap();
    // Ligne de support de l'éditeur sur Draco : fonction et service ne sont pas montrés.
    sqlx::query(
        "INSERT INTO acc_trace (id, etablissement_id, le, acteur_compte_id, acteur_role, fonction_code, fonction_libelle,
                                service_code, service_libelle, nature, section, action, sujet_eleve_id, support_editeur,
                                support_intervention_le)
         VALUES ($1, $2, $3, $1, 'support', 'support', 'Technicien secret', 'support', 'Équipe secrète', 'consultation', 'dossier',
                 'support', $4, true, '2026-11-12')",
    )
    .bind(Uuid::new_v4()).bind(etab).bind(le(2026, 11, 13, 9)).bind(cast.eleve_tom_id)
    .execute(p).await.unwrap();
    let page = ent
        .acces_consultations_a(etab, cast.parent_tom.user_id, "parent", le(2026, 11, 20, 9))
        .await
        .unwrap();
    let l = &page.dossiers[0].jours[0].lignes[0];
    assert_eq!(
        l.mention.as_deref(),
        Some("Support ScolIA, intervention autorisée le 12/11/2026")
    );
    let tout = format!("{page:?}");
    assert!(
        !tout.contains("secret"),
        "ni la fonction ni le service du support : {tout}"
    );
    // Dossier de l'agent : sa propre page, et seulement la sienne.
    let rogue = cast.profs[0].user_id;
    let sa_personne = personne_de(&ent, etab, rogue).await;
    sqlx::query(
        "INSERT INTO acc_trace (id, etablissement_id, le, acteur_compte_id, acteur_role, fonction_code, fonction_libelle,
                                service_code, service_libelle, nature, section, action, sujet_personne_id)
         VALUES ($1, $2, $3, $4, 'administratif', 'secretaire_direction', 'Secrétaire de direction', 'administration',
                 'Administration', 'consultation', 'dossier_agent', 'dossier_agent', $5)",
    )
    .bind(Uuid::new_v4()).bind(etab).bind(le(2026, 11, 9, 9)).bind(cast.administratif.user_id).bind(sa_personne)
    .execute(p).await.unwrap();
    let mien = ent
        .acces_consultations_a(etab, rogue, PROF, le(2026, 11, 20, 9))
        .await
        .unwrap();
    assert_eq!(mien.dossiers.len(), 1);
    assert_eq!(mien.dossiers[0].sujet_type, "agent");
    assert_eq!(mien.dossiers[0].total, 1);
    assert_eq!(
        mien.dossiers[0].jours[0].lignes[0].fonction,
        "Secrétaire de direction"
    );
    let autre = ent
        .acces_consultations_a(etab, cast.profs[1].user_id, PROF, le(2026, 11, 20, 9))
        .await
        .unwrap();
    assert_eq!(
        autre.dossiers[0].total, 0,
        "le dossier d'un autre agent n'est pas montré"
    );
}

#[tokio::test]
async fn g4_cloisonnement_entre_etablissements() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    ent.tracer_consultation_a(
        etab,
        cast.profs[0].user_id,
        PROF,
        Some(cast.eleve_lea_id),
        "scolarite",
        "Scolarité",
        false,
        le(2026, 11, 3, 9),
    )
    .await
    .unwrap();
    // La page d'un compte ne montre jamais les traces d'un autre établissement : un identifiant d'élève étranger.
    let page = ent
        .acces_consultations_a(
            Uuid::new_v4(),
            cast.parent_lea.user_id,
            "parent",
            le(2026, 11, 20, 9),
        )
        .await
        .unwrap();
    assert!(page.dossiers.is_empty());
}

// ───────────────────────────── Motifs (S-542) ─────────────────────────────

#[tokio::test]
async fn g4_motifs_par_defaut_renommes_ou_desactives_par_l_etablissement() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    assert_eq!(
        ent.acces_motifs(etab).await.unwrap().len(),
        MOTIFS_PAR_DEFAUT.len()
    );
    let p = ent.pool_verif();
    sqlx::query("INSERT INTO acc_motif (etablissement_id, code, libelle, rang, actif) VALUES ($1, 'urgence', 'Urgence médicale', 0, true), ($1, 'autre_motif', 'Autre', 9, false)")
        .bind(etab).execute(p).await.unwrap();
    let m = ent.acces_motifs(etab).await.unwrap();
    assert!(m
        .iter()
        .any(|x| x.code == "urgence" && x.libelle == "Urgence médicale"));
    assert!(m.iter().all(|x| x.code != "autre_motif"), "motif désactivé");
    // Un motif désactivé n'est plus accepté.
    let r = ent
        .get_eleve_vue_motive_for_actor(
            etab,
            cast.profs[0].user_id,
            PROF,
            cast.eleve_tom_id,
            Some("autre_motif"),
            true,
            le(2026, 10, 15, 9),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
}
