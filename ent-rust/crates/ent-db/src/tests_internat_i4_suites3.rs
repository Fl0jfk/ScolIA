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
//! | F4 : le 3e événement est consommé ; `notifier_personne` inchangé | `s5_avis_regroupe_*`, `s5_notifier_personne_*` |
//! | F7 : aucune connexion supplémentaire dans une transaction ouverte | `s5_eleve_cache_au_tiers_*` |

use super::notifications::{notifier_personne, EnvoiAPlanifier};
use super::tests_internat_i2::{banc, Banc, CPE};
use super::tests_internat_i3::parent;
use super::tests_internat_i4::*;
use super::tests_internat_i4_correctifs::sortie_prete;
use super::tests_internat_i4_passe2::{apres_fin, distribuer};
use super::tests_internat_i4_passe3::proteger;
use super::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

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
         ON CONFLICT (etablissement_id, destinataire_user_id, canal, cle_dedup) DO NOTHING",
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
