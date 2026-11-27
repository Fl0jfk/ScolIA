//! Socle 41, lot I4 — **suites 3** (faibles F1, F2, F4, F7 de la contre-revue des suites 2, D-069), testées contre la
//! base locale de test (feature `labo`). Le libellé F6 est testé dans `ent-types` (fonction pure).
//!
//! **Aucun test ne dépend de l'heure réelle** (règle 33) : l'âge d'un événement est posé par la base sur une minute calculée
//! une seule fois ; jours fixes d'avril 2026.
//!
//! | Point | Test(s) |
//! |---|---|
//! | F1 : avis lu, le nombre change → repasse non lu | `s5_avis_regroupe_*` |
//! | F2 : envoi externe déjà parti : jamais réécrit | `s5_avis_regroupe_*` |
//! | F1 (contrepartie) : texte identique → l'avis reste lu | `s5_avis_regroupe_texte_identique_*` |
//! | F4 : le 3e événement est consommé ; `notifier_personne` inchangé | `s5_avis_regroupe_*`, `s5_notifier_personne_*` |
//! | F7 : aucune connexion supplémentaire dans une transaction ouverte | `s5_eleve_cache_au_tiers_*` |
//! | T2 : mêmes variantes `_conn` (visibilité, droit d'établissement, appel de groupe) et gestes de `vie.rs` | `t2_*` |

use super::notifications::{notifier_personne, EnvoiAPlanifier};
use super::tests_internat_i2::{banc, Banc, CPE};
use super::tests_internat_i3::parent;
use super::tests_internat_i4::*;
use super::tests_internat_i4_correctifs::sortie_prete;
use super::tests_internat_i4_passe2::{apres_fin, distribuer};
use super::tests_internat_i4_passe3::proteger;
use super::*;
use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use ent_types::internat_vie::VigilanceSaisieDto;
use uuid::Uuid;

async fn corps_et_lecture(conn: &mut sqlx::PgConnection, source: Uuid, user: Uuid) -> (String, Option<DateTime<Utc>>) {
    sqlx::query_as("SELECT corps, lu_le FROM ent_notification WHERE source_event_id = $1 AND destinataire_user_id = $2")
        .bind(source)
        .bind(user)
        .fetch_one(conn)
        .await
        .unwrap()
}

async fn avis(b: &Banc, user: Uuid) -> Vec<(String, Option<DateTime<Utc>>)> {
    sqlx::query_as(
        "SELECT corps, lu_le FROM ent_notification \
          WHERE etablissement_id = $1 AND kind = 'internat_cas_ouvert' AND destinataire_user_id = $2 ORDER BY created_at",
    )
    .bind(b.etab)
    .bind(user)
    .fetch_all(b.ent.pool_verif())
    .await
    .unwrap()
}

async fn envoi(b: &Banc, user: Uuid, cle: &str) -> (String, String) {
    sqlx::query_as(
        "SELECT corps, statut FROM ent_notification_envoi \
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND cle_dedup = $3 AND canal = 'email'",
    )
    .bind(b.etab)
    .bind(user)
    .bind(cle)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap()
}

/// Un cas ouvert pour `eleve`, daté dans `minute` (posé une fois par la base), puis distribué.
async fn cas_dans_la_minute(b: &Banc, eleve: Uuid, minute: DateTime<Utc>, apres: i64) {
    let (p, _) = parent(b, eleve, true).await;
    let g = sortie_prete(b, &[(eleve, p)]).await;
    depart(b, g.id, vec![eleve]).await;
    assert_eq!(
        b.ent
            .veille_non_rentres_a(b.etab, apres_fin(apres))
            .await
            .unwrap(),
        1
    );
    sqlx::query(
        "UPDATE ent_metier_event SET created_at = $3 \
          WHERE etablissement_id = $1 AND type = 'internat.cas_ouvert' AND eleve_id = $2",
    )
    .bind(b.etab)
    .bind(eleve)
    .bind(minute + chrono::Duration::seconds(10))
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    distribuer(b).await;
}

#[tokio::test]
async fn s5_avis_regroupe_lu_repasse_non_lu_et_envoi_parti_jamais_reecrit() {
    let Some(b) = banc().await else { return };
    let (e1, e2, e3) = (b.a[0], b.a[1], b.a[2]);
    let minute: DateTime<Utc> = sqlx::query_scalar("SELECT date_trunc('minute', now())")
        .fetch_one(b.ent.pool_verif())
        .await
        .unwrap();
    let cle = format!(
        "i4:{}:internat_cas_ouvert:{}",
        b.etab,
        minute.timestamp().div_euclid(60)
    );

    // T3 (faible 4) : le test ne dépend pas de l'état préalable du banc. Chaque banc a son propre établissement de labo
    // (`seed_labo_cast` en crée un neuf ; commentaire corrigé au palier des faibles, lot 3), mais on part quand même d'un
    // état vide pour cette clé, et on le vérifie.
    sqlx::query(
        "DELETE FROM ent_notification_envoi \
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND canal = 'email' AND kind = 'internat_cas_ouvert'",
    )
    .bind(b.etab)
    .bind(b.cpe)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "DELETE FROM ent_notification \
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'internat_cas_ouvert'",
    )
    .bind(b.etab)
    .bind(b.cpe)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    assert!(avis(&b, b.cpe).await.is_empty(), "état de départ : aucun avis");

    // 1er événement : un avis simple, non lu.
    cas_dans_la_minute(&b, e1, minute, 16).await;
    let a1 = avis(&b, b.cpe).await;
    assert_eq!(a1.len(), 1, "{a1:?}");
    assert!(
        a1[0].1.is_none() && !a1[0].0.contains("élèves concernés"),
        "{a1:?}"
    );
    // Un envoi externe existe, `en_attente` (posé ici : les préférences du banc n'en planifient pas forcément).
    sqlx::query(
        "INSERT INTO ent_notification_envoi (id, etablissement_id, destinataire_user_id, canal, kind, titre, corps, \
                source_type, source_id, cle_dedup, statut) \
         VALUES ($1,$2,$3,'email','internat_cas_ouvert','Cas de nuit ouvert',$4,'evenement',$5,$6,'en_attente') \
         ON CONFLICT (etablissement_id, destinataire_user_id, canal, cle_dedup) \
         DO UPDATE SET statut = 'en_attente', corps = EXCLUDED.corps",
    )
    .bind(Uuid::new_v4())
    .bind(b.etab)
    .bind(b.cpe)
    .bind(&a1[0].0)
    .bind(Uuid::new_v4())
    .bind(&cle)
    .execute(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(envoi(&b, b.cpe, &cle).await.1, "en_attente", "départ : l'envoi n'est pas parti");

    // 2e événement : l'avis et l'envoi encore en attente portent « 2 élèves ».
    cas_dans_la_minute(&b, e2, minute, 30).await;
    let a2 = avis(&b, b.cpe).await;
    assert_eq!(a2.len(), 1, "{a2:?}");
    assert!(a2[0].0.contains("2 élèves"), "{a2:?}");
    let (corps_envoi, _) = envoi(&b, b.cpe, &cle).await;
    assert!(
        corps_envoi.contains("2 élèves"),
        "l'envoi en attente est à jour : {corps_envoi}"
    );

    // Le responsable lit l'avis ; l'envoi externe PART (`envoye`).
    sqlx::query("UPDATE ent_notification SET lu_le = $3 WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'internat_cas_ouvert'")
        .bind(b.etab)
        .bind(b.cpe)
        .bind(minute + chrono::Duration::seconds(30))
        .execute(b.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_notification_envoi SET statut = 'envoye', traite_le = $4 WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND cle_dedup = $3")
        .bind(b.etab)
        .bind(b.cpe)
        .bind(&cle)
        .bind(minute + chrono::Duration::seconds(31))
        .execute(b.ent.pool_verif())
        .await
        .unwrap();

    // 3e événement (F4 : il est bien consommé) : l'avis passe à « 3 élèves » ET repasse NON LU (F1) ; l'envoi parti n'est
    // jamais réécrit (F2 : constat documenté, pas d'avis rectificatif) ; toujours un seul avis.
    cas_dans_la_minute(&b, e3, minute, 50).await;
    let a3 = avis(&b, b.cpe).await;
    assert_eq!(a3.len(), 1, "toujours un seul avis : {a3:?}");
    assert!(
        a3[0].0.contains("3 élèves"),
        "le 3e événement est consommé : {a3:?}"
    );
    assert!(
        a3[0].1.is_none(),
        "le nombre a changé : l'avis repasse non lu : {a3:?}"
    );
    let (corps_envoi, statut) = envoi(&b, b.cpe, &cle).await;
    assert_eq!(statut, "envoye");
    assert!(
        corps_envoi.contains("2 élèves") && !corps_envoi.contains("3 élèves"),
        "un envoi parti n'est jamais réécrit : {corps_envoi}"
    );
    // Les trois événements sont bien traités (aucun reste en attente pour ce consommateur).
    let restants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_metier_event m WHERE m.etablissement_id = $1 AND m.type = 'internat.cas_ouvert' \
            AND m.eleve_id = ANY($2) AND NOT EXISTS (SELECT 1 FROM ent_event_traitement t WHERE t.event_id = m.id)",
    )
    .bind(b.etab)
    .bind(vec![e1, e2, e3])
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(restants, 0, "les trois événements sont consommés");
}

/// T3 (faible 3 de I4) : la contrepartie de F1 — un avis regroupé relu avec un texte **identique** reste lu ; seul un texte
/// différent le remet à non lu. Direct sur `notifier_personne_selon` (aucun événement, aucune minute, aucune horloge) et sur
/// un genre d'avis propre au test : aucun effet sur les avis des autres tests du banc.
#[tokio::test]
async fn s5_avis_regroupe_texte_identique_reste_lu_et_texte_different_repasse_non_lu() {
    use super::notifications::notifier_personne_selon;
    let Some(b) = banc().await else { return };
    let source = Uuid::new_v4();
    let avis_a = |corps: &'static str| EnvoiAPlanifier {
        etablissement_id: b.etab,
        user_id: b.cpe,
        kind: "t3_avis_regroupe_identique",
        titre: "Titre",
        corps,
        eleve_id: None,
        notification_id: None,
        source_type: "evenement",
        source_id: source,
        cle_dedup: format!("t3:{source}"),
        droit_requis: None,
        regroupable: false,
        canaux_voulus: &[],
    };
    let mut conn = b.ent.pool_verif().acquire().await.unwrap();
    let maintenant = h(1);
    notifier_personne_selon(&mut conn, avis_a("2 élèves concernés"), source, maintenant, true).await.unwrap();
    sqlx::query("UPDATE ent_notification SET lu_le = $3 WHERE source_event_id = $1 AND destinataire_user_id = $2")
        .bind(source)
        .bind(b.cpe)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .unwrap();
    // Texte identique : l'avis reste lu (rien n'a changé pour la personne).
    notifier_personne_selon(&mut conn, avis_a("2 élèves concernés"), source, maintenant, true).await.unwrap();
    let (corps, lu) = corps_et_lecture(&mut conn, source, b.cpe).await;
    assert_eq!(corps, "2 élèves concernés");
    assert_eq!(lu, Some(maintenant), "texte identique : l'avis reste lu");
    // Texte différent : texte à jour ET non lu (témoin : le test distingue bien les deux cas).
    notifier_personne_selon(&mut conn, avis_a("3 élèves concernés"), source, maintenant, true).await.unwrap();
    let (corps, lu) = corps_et_lecture(&mut conn, source, b.cpe).await;
    assert_eq!(corps, "3 élèves concernés");
    assert!(lu.is_none(), "texte différent : l'avis repasse non lu");
}

#[tokio::test]
async fn s5_notifier_personne_ne_reecrit_jamais_un_avis_existant() {
    let Some(b) = banc().await else { return };
    let source = Uuid::new_v4();
    let envoi_a = |corps: &'static str| EnvoiAPlanifier {
        etablissement_id: b.etab,
        user_id: b.cpe,
        kind: "internat_cas_ouvert",
        titre: "Titre",
        corps,
        eleve_id: None,
        notification_id: None,
        source_type: "evenement",
        source_id: source,
        cle_dedup: format!("s5:{source}"),
        droit_requis: None,
        regroupable: false,
        canaux_voulus: &["email"],
    };
    let mut conn = b.ent.pool_verif().acquire().await.unwrap();
    let maintenant = h(1);
    notifier_personne(&mut conn, envoi_a("premier texte"), source, maintenant)
        .await
        .unwrap();
    notifier_personne(&mut conn, envoi_a("second texte"), source, maintenant)
        .await
        .unwrap();
    let corps: String = sqlx::query_scalar("SELECT corps FROM ent_notification WHERE source_event_id = $1 AND destinataire_user_id = $2")
        .bind(source)
        .bind(b.cpe)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(
        corps, "premier texte",
        "`notifier_personne` : DO NOTHING, jamais de réécriture"
    );
}

// ───────────────────────── F7 : aucune connexion supplémentaire dans une transaction ─────────────────────────

#[tokio::test]
async fn s5_eleve_cache_au_tiers_n_ouvre_pas_de_connexion_dans_une_transaction_ouverte() {
    let Some(b) = banc().await else { return };
    let (libre, protege) = (b.a[0], b.a[1]);
    proteger(&b, protege).await;
    // Pool d'UNE seule connexion : tant que la transaction l'occupe, toute demande d'une seconde connexion attendrait
    // indéfiniment (c'était le cas avant F7). La réponse doit venir sur la connexion de la transaction, et être celle de la
    // version qui passe par le pool.
    let options = (*b.ent.pool_verif().connect_options()).clone();
    let un = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect_with(options)
        .await
        .unwrap();
    let ent1 = PgEnt::from_pool(un.clone());
    for eleve in [libre, protege] {
        let attendu = b
            .ent
            .eleve_cache_au_tiers(b.etab, b.cpe, CPE, eleve)
            .await
            .unwrap();
        let mut tx = crate::tenant_tx::TenantTx::begin(&un, b.etab)
            .await
            .unwrap();
        let rep = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            ent1.eleve_cache_au_tiers_conn(&mut tx, b.etab, b.cpe, CPE, eleve),
        )
        .await
        .expect("aucune connexion supplémentaire attendue (pas de blocage)")
        .unwrap();
        assert_eq!(rep, attendu, "même réponse que la version par le pool");
        tx.rollback().await.unwrap();
    }
}

// ───────────────────── T2 : F7 achevé (exiger_eleve_visible, a_action_etablissement, appel de groupe) ─────────────────────

/// Pool d'UNE connexion, sur la même base : toute connexion demandée en plus de celle de la transaction attendrait.
async fn pool_une_connexion(b: &Banc) -> (sqlx::PgPool, PgEnt) {
    let options = (*b.ent.pool_verif().connect_options()).clone();
    let un = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect_with(options)
        .await
        .unwrap();
    let ent1 = PgEnt::from_pool(un.clone());
    (un, ent1)
}

fn dix_secondes<F: std::future::Future>(f: F) -> tokio::time::Timeout<F> {
    tokio::time::timeout(std::time::Duration::from_secs(10), f)
}

/// Les variantes `_conn` rendent exactement la réponse des variantes par le pool, sur la connexion de la transaction.
#[tokio::test]
async fn t2_visibilite_et_droit_d_etablissement_sur_la_connexion_de_la_transaction() {
    let Some(b) = banc().await else { return };
    let (libre, protege) = (b.a[0], b.a[1]);
    proteger(&b, protege).await;
    let (un, ent1) = pool_une_connexion(&b).await;
    for eleve in [libre, protege] {
        let attendu = b.ent.exiger_eleve_visible(b.etab, b.cpe, CPE, eleve).await.is_ok();
        let mut tx = crate::tenant_tx::TenantTx::begin(&un, b.etab).await.unwrap();
        let rep = dix_secondes(ent1.exiger_eleve_visible_conn(&mut tx, b.etab, b.cpe, CPE, eleve))
            .await
            .expect("aucune connexion supplémentaire attendue")
            .is_ok();
        assert_eq!(rep, attendu, "même verdict (élève protégé : introuvable)");
        tx.rollback().await.unwrap();
    }
    for (acteur, role) in [(b.cpe, CPE), (b.prof, PROF)] {
        for action in [Action::InternatGerer, Action::InternatAppel, Action::InternatEvenementDeclarer] {
            let attendu = b.ent.a_action_etablissement(b.etab, acteur, role, action).await.unwrap();
            let mut tx = crate::tenant_tx::TenantTx::begin(&un, b.etab).await.unwrap();
            let rep = dix_secondes(ent1.a_action_etablissement_conn(&mut tx, b.etab, acteur, role, action))
                .await
                .expect("aucune connexion supplémentaire attendue")
                .unwrap();
            assert_eq!(rep, attendu, "{role} / {action:?}");
            tx.rollback().await.unwrap();
        }
    }
}

/// Droit de faire l'appel d'une sortie : le CPE (droit d'établissement, pas accompagnateur) passe sur la connexion de la
/// transaction, un parent est refusé, et aucune connexion n'est prise au pool.
#[tokio::test]
async fn t2_appel_de_groupe_exiger_sur_la_connexion_de_la_transaction() {
    let Some(b) = banc().await else { return };
    let eleve = b.a[0];
    let (p, _) = parent(&b, eleve, true).await;
    let g = sortie_prete(&b, &[(eleve, p)]).await;
    let (un, ent1) = pool_une_connexion(&b).await;
    let mut tx = crate::tenant_tx::TenantTx::begin(&un, b.etab).await.unwrap();
    dix_secondes(ent1.groupe_appel_exiger(&mut tx, b.etab, b.cpe, CPE, g.id, t0()))
        .await
        .expect("aucune connexion supplémentaire attendue")
        .expect("le CPE a le droit d'établissement");
    let refus = dix_secondes(ent1.groupe_appel_exiger(&mut tx, b.etab, p, "parent", g.id, t0()))
        .await
        .expect("aucune connexion supplémentaire attendue");
    assert!(matches!(refus, Err(EntDbError::Forbidden)), "{refus:?}");
    tx.rollback().await.unwrap();
}

/// Geste de `vie.rs` (lever une vigilance) sur un pool d'une connexion : visibilité contrôlée dans la transaction, élève
/// protégé introuvable pour un tiers, et le geste aboutit sans interblocage pour un élève visible.
#[tokio::test]
async fn t2_lever_une_vigilance_sur_un_pool_d_une_connexion() {
    let Some(b) = banc().await else { return };
    let (libre, protege) = (b.a[0], b.a[1]);
    let saisie = |eleve: Uuid| VigilanceSaisieDto {
        eleve_id: eleve,
        code: "suivi_rapproche".into(),
        referent_user_id: None,
        du: Some(NaiveDate::from_ymd_opt(2026, 4, 14).unwrap()),
        au: Some(NaiveDate::from_ymd_opt(2028, 3, 1).unwrap()),
    };
    let v_libre = b.ent.poser_vigilance_a(b.etab, b.cpe, CPE, &saisie(libre), t0()).await.unwrap();
    let v_protege = b.ent.poser_vigilance_a(b.etab, b.cpe, CPE, &saisie(protege), t0()).await.unwrap();
    proteger(&b, protege).await;
    let (_un, ent1) = pool_une_connexion(&b).await;
    let e = dix_secondes(ent1.lever_vigilance_a(b.etab, b.cpe, CPE, v_protege.id, t0()))
        .await
        .expect("aucun interblocage");
    assert!(matches!(e, Err(EntDbError::Domain(DomainError::NotFound))), "élève protégé introuvable : {e:?}");
    let ok = dix_secondes(ent1.lever_vigilance_a(b.etab, b.cpe, CPE, v_libre.id, t0()))
        .await
        .expect("aucun interblocage")
        .expect("élève visible : vigilance levée");
    assert_eq!(ok.id, v_libre.id);
}

/// T3 (faible 2 de T2) : une variante `_conn` appelée avec la connexion d'un AUTRE établissement est détectée (panique en
/// test/débogage), la bonne connexion passe. Le mode est donné explicitement (`Permissif`) : le test ne dépend pas de
/// `ENT_RLS_MODE`.
#[cfg(debug_assertions)]
#[tokio::test]
async fn t3_le_contrat_des_variantes_conn_detecte_la_connexion_d_un_autre_etablissement() {
    use crate::tenant_tx::{verifier_connexion_etablissement_mode as verifier, RlsMode, TenantTx};
    let Some(b) = banc().await else { return };
    let (un, _ent1) = pool_une_connexion(&b).await;
    // Témoin positif : la connexion porte bien l'établissement demandé.
    let mut tx = TenantTx::begin_mode(&un, b.etab, RlsMode::Permissif).await.unwrap();
    verifier(&mut tx, b.etab, "témoin", RlsMode::Permissif).await;
    tx.rollback().await.unwrap();
    // Autre établissement, puis aucun contexte (mode coupé à l'ouverture) : le contrôle panique, dans les deux cas.
    let autre = Uuid::new_v4();
    for ouverture in [RlsMode::Permissif, RlsMode::Off] {
        let (pool, etab) = (un.clone(), b.etab);
        let r = tokio::spawn(async move {
            let mut tx = TenantTx::begin_mode(&pool, if ouverture == RlsMode::Off { etab } else { autre }, ouverture)
                .await
                .unwrap();
            verifier(&mut tx, etab, "variante _conn", RlsMode::Permissif).await;
        })
        .await;
        let e = r.expect_err("le contrôle doit paniquer");
        assert!(e.is_panic(), "panique attendue ({ouverture:?}) : {e:?}");
    }
    // Mode coupé : aucun contrôle (la variable n'est jamais posée).
    let mut tx = TenantTx::begin_mode(&un, b.etab, RlsMode::Off).await.unwrap();
    verifier(&mut tx, Uuid::new_v4(), "coupé", RlsMode::Off).await;
    tx.rollback().await.unwrap();
}
