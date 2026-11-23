//! Chantier transverse T1 — **preuves** : une table temporaire homonyme, créée par une session du rôle applicatif,
//! ne désarme aucune garde (docs/securite/T1_SEARCH_PATH.md §6).
//!
//! Chaque preuve a trois temps, dans des transactions toujours annulées :
//! 1. **sans masque** : la garde refuse l'écriture avec son message exact (référence) ;
//! 2. **avec masque, sous le rôle applicatif** : table temporaire homonyme, remplie de lignes qui feraient passer la
//!    garde si elle les lisait ; l'écriture reste refusée, avec le MÊME message ;
//! 3. **témoin** (la preuve prouve quelque chose) : sous le propriétaire, la clause `search_path` des fonctions de la
//!    garde est retirée dans la transaction (état d'avant T1) ; le même masque désarme alors la garde et l'écriture passe.
//!
//! Le rôle applicatif est le rôle de test non propriétaire, sans BYPASSRLS, `app.rls_strict = on`
//! (`tests_rls_stricte::ROLE_STRICT`, celui de `ent_app` en production). Les autres tests qui créent des tables
//! temporaires (`tests_cahier`, `tests_inscriptions_i5_passe3`, `tests_internat_i3`) tournent sous le rôle du harnais
//! (propriétaire en mode permissif, rôle strict en mode strict) : voir le document pour la conséquence d'un
//! `REVOKE TEMP … FROM PUBLIC` sur ces tests.

use super::tests_internat_i2::banc;
use super::tests_internat_i3::parent;
use super::tests_internat_i4::{accepter, brouillon, depart, repondre, soumettre};
use super::tests_messagerie_m1::{ecrit, labo, role};
use super::tests_messagerie_m5::fil_de_lily;
use super::tests_messagerie_m6::dernier_message;
use super::*;
use ent_auth::Role;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

/// Pool du rôle applicatif de test (non propriétaire, sans BYPASSRLS, RLS stricte).
async fn pool_applicatif(proprietaire: &PgPool) -> PgPool {
    crate::tests_rls_stricte::assurer_role_strict(proprietaire).await;
    let url = crate::url_guard::url_labo_ou_echec().expect("ENT_DATABASE_URL");
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(crate::tests_rls_stricte::ROLE_STRICT)
        .unwrap();
    u.set_password(Some("rls")).unwrap();
    PgPoolOptions::new()
        .max_connections(2)
        .connect(u.as_str())
        .await
        .unwrap()
}

/// `(code SQLSTATE, message)` d'un refus de la base.
type Refus = (String, String);

/// Exécute `preparation` puis `ecriture` dans une transaction **toujours annulée**, avec l'établissement posé comme
/// le fait `TenantTx`. Rend le nombre de lignes touchées, ou le refus de la base.
async fn tenter(
    pool: &PgPool,
    etab: Uuid,
    preparation: &[String],
    ecriture: &str,
) -> Result<u64, Refus> {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    for ordre in preparation {
        sqlx::query(ordre)
            .execute(&mut *tx)
            .await
            .unwrap_or_else(|e| panic!("préparation « {ordre} » : {e}"));
    }
    let r = sqlx::query(ecriture).execute(&mut *tx).await;
    tx.rollback().await.unwrap();
    match r {
        Ok(x) => Ok(x.rows_affected()),
        Err(e) => {
            let d = e
                .as_database_error()
                .unwrap_or_else(|| panic!("erreur hors base : {e}"));
            Err((
                d.code().map(|c| c.to_string()).unwrap_or_default(),
                d.message().to_string(),
            ))
        }
    }
}

/// Retire, dans la transaction, la clause `search_path` des fonctions nommées : l'état d'avant T1.
fn desarmer(fonctions: &[&str]) -> Vec<String> {
    fonctions
        .iter()
        .map(|f| format!("ALTER FUNCTION public.{f} RESET search_path"))
        .collect()
}

/// Les trois temps d'une preuve (voir l'en-tête du fichier).
struct Preuve<'a> {
    nom: &'a str,
    etab: Uuid,
    /// Fonctions (signatures) que la garde appelle, dont la clause est retirée pour le témoin.
    fonctions: &'a [&'a str],
    /// Table(s) temporaire(s) homonyme(s) et leur remplissage « qui ferait passer la garde ».
    masque: Vec<String>,
    ecriture: String,
    code: &'a str,
    message: &'a str,
    /// La garde qualifie-t-elle elle-même ses relations (`public.`) ? Alors, même sans clause, le masque est sans effet :
    /// le témoin vérifie ce **refus** au lieu d'un désarmement.
    qualifiee: bool,
}

async fn prouver(proprietaire: &PgPool, p: Preuve<'_>) {
    // L'état d'une installation : d'autres tests rejouent des fichiers de migration isolés dans la base commune
    // (chaque `CREATE OR REPLACE` efface la clause) ; la garde du catalogue, elle, vérifie une base neuve.
    crate::migrate::remettre_search_path(proprietaire)
        .await
        .expect("690");
    let app = pool_applicatif(proprietaire).await;
    let nom = p.nom;
    // Précondition : le rôle applicatif a bien le privilège TEMP (le défaut de PostgreSQL) — sinon la preuve serait vide.
    let (temp,): (bool,) =
        sqlx::query_as("SELECT has_database_privilege(current_user, current_database(), 'TEMP')")
            .fetch_one(&app)
            .await
            .unwrap();
    assert!(temp, "{nom} : le rôle applicatif de test doit pouvoir créer des tables temporaires (sinon rien n'est prouvé)");

    // 1. Sans masque : la garde refuse, message exact.
    let r = tenter(&app, p.etab, &[], &p.ecriture).await;
    assert_eq!(
        r.as_ref().map_err(|e| (e.0.as_str(), e.1.as_str())),
        Err((p.code, p.message)),
        "{nom} : référence sans masque"
    );

    // 2. Avec masque, sous le rôle applicatif : le même refus, mot pour mot.
    let r = tenter(&app, p.etab, &p.masque, &p.ecriture).await;
    assert_eq!(
        r.as_ref().map_err(|e| (e.0.as_str(), e.1.as_str())),
        Err((p.code, p.message)),
        "{nom} : une table temporaire homonyme a désarmé la garde"
    );

    // 3. Témoin : sans la clause (état d'avant T1), le même masque désarme la garde.
    let mut prep = desarmer(p.fonctions);
    prep.extend(p.masque.iter().cloned());
    let r = tenter(proprietaire, p.etab, &prep, &p.ecriture).await;
    if p.qualifiee {
        assert_eq!(
            r.as_ref().map_err(|e| (e.0.as_str(), e.1.as_str())),
            Err((p.code, p.message)),
            "{nom} : garde qualifiée — même sans `search_path`, la table temporaire ne doit rien désarmer"
        );
    } else {
        assert!(
            r.is_ok(),
            "{nom} : témoin — sans `search_path` la table temporaire devait désarmer la garde (sinon la preuve ne prouve rien) : {r:?}"
        );
    }
}

/// Gel du cahier de textes (2407) : une année dont le cahier est clos ne se clôt pas deux fois. Le masque : un journal
/// de clôture temporaire où le cahier a été rouvert, et une année temporaire non close — la garde, si elle les lisait,
/// croirait le cahier ouvert.
#[tokio::test]
async fn t1_gel_du_cahier_une_table_temporaire_ne_le_desarme_pas() {
    let Some((ent, _)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Établissement propre à ce test (aucune autre suite ne clôt son année).
    let cast = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let autre = cast.profs[0].user_id;
    let annee = ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap()
        .id;
    sqlx::query("INSERT INTO ent_cdt_cloture (id, etablissement_id, annee_id, action, par_user_id) VALUES (gen_random_uuid(), $1, $2, 'cloture', $3)")
        .bind(etab)
        .bind(annee)
        .bind(dir)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let fige: bool = sqlx::query_scalar("SELECT ent_cdt_annee_gelee($1, $2)")
        .bind(etab)
        .bind(annee)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(fige, "précondition : le cahier de l'année est clos");
    let masque = vec![
        "CREATE TEMP TABLE ent_cdt_cloture (LIKE public.ent_cdt_cloture INCLUDING DEFAULTS) ON COMMIT DROP".to_string(),
        "CREATE TEMP TABLE ent_annee_scolaire (LIKE public.ent_annee_scolaire) ON COMMIT DROP".to_string(),
        // Une réouverture plus récente que la clôture : le cahier paraîtrait ouvert.
        format!(
            "INSERT INTO pg_temp.ent_cdt_cloture (id, etablissement_id, annee_id, action, motif, par_user_id, demandeur_user_id, cree_le) VALUES \
             (gen_random_uuid(), '{etab}', '{annee}', 'cloture', NULL, '{dir}', NULL, now() - interval '2 hours'), \
             (gen_random_uuid(), '{etab}', '{annee}', 'reouverture', 'Réouverture de complaisance posée dans la table temporaire', '{autre}', '{dir}', now() - interval '1 hour')"
        ),
        // L'année, non close au socle 5.
        format!("INSERT INTO pg_temp.ent_annee_scolaire SELECT * FROM public.ent_annee_scolaire WHERE id = '{annee}'"),
        "UPDATE pg_temp.ent_annee_scolaire SET cloturee_le = NULL".to_string(),
    ];
    prouver(
        ent.pool_verif(),
        Preuve {
            nom: "gel du cahier (journal de clôture)",
            etab,
            fonctions: &["ent_cdt_cloture_coherence()", "ent_cdt_cahier_fige(uuid, uuid)", "ent_cdt_annee_gelee(uuid, uuid)"],
            masque,
            ecriture: format!(
                "INSERT INTO public.ent_cdt_cloture (id, etablissement_id, annee_id, action, par_user_id) VALUES (gen_random_uuid(), '{etab}', '{annee}', 'cloture', '{dir}')"
            ),
            code: "23514",
            message: "journal de clôture : le cahier de cette année est déjà clôturé",
            qualifiee: false,
        },
    )
    .await;
}

/// Garde d'annulation de la messagerie (490/491) : les versions d'un message DÉJÀ REMIS restent en ajout seul, même
/// avec le réglage d'annulation posé. Le masque : un `ent_msg` temporaire où ce message est encore différé (remise
/// future) — la garde, si elle le lisait, autoriserait la suppression.
#[tokio::test]
async fn t1_messagerie_la_garde_d_annulation_n_est_pas_trompee_par_une_table_temporaire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "T1 ajout seul").await;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Remis").await);
    let remis = dernier_message(&ent, etab, fil, dir).await.id;
    ent.corriger_message_fil_for_actor(etab, dir, role(Role::Direction), remis, "Remis, corrigé")
        .await
        .expect("correction");
    let versions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_msg_version WHERE message_id = $1")
            .bind(remis)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(
        versions >= 1,
        "précondition : le message remis a des versions"
    );
    let masque = vec![
        format!("SELECT set_config('ent.msg_annulation', '{remis}', true)"),
        "CREATE TEMP TABLE ent_msg (LIKE public.ent_msg) ON COMMIT DROP".to_string(),
        format!("INSERT INTO pg_temp.ent_msg SELECT * FROM public.ent_msg WHERE id = '{remis}'"),
        // Dans la table temporaire, le message n'est pas encore remis : remise prévue demain.
        "UPDATE pg_temp.ent_msg SET remise_prevue_le = now() + interval '1 day'".to_string(),
    ];
    prouver(
        ent.pool_verif(),
        Preuve {
            nom: "messagerie (annulation)",
            etab,
            fonctions: &["ent_msg_ajout_seul_sauf_annulation()"],
            masque,
            ecriture: format!("DELETE FROM public.ent_msg_version WHERE message_id = '{remis}'"),
            code: "23514",
            message: "ent_msg_version : ajout seul (ni modification ni suppression)",
            // 492 (passe 2 de la contre-revue E1) a recréé la garde avec ses relations qualifiées `public.` et un jeton
            // de transaction : la table temporaire ne la trompe pas, même sans clause.
            qualifiee: true,
        },
    )
    .await;
    // Le message et ses versions sont intacts (les transactions ont été annulées).
    let apres: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_msg_version WHERE message_id = $1")
            .bind(remis)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(apres, versions, "aucune version supprimée");
}

/// Garde de l'internat (2828) : une sortie de groupe dont des élèves sont partis ne s'annule pas. Le masque : une table
/// `ent_internat_sortie_groupe_participant` temporaire VIDE — la garde, si elle la lisait, ne verrait aucun élève parti.
#[tokio::test]
async fn t1_internat_la_garde_d_une_sortie_de_groupe_n_est_pas_trompee_par_une_table_temporaire() {
    let Some(b) = banc().await else { return };
    let e = b.a[0];
    let (p, _) = parent(&b, e, true).await;
    let g = soumettre(&b, brouillon(&b, vec![b.prof], vec![e]).await.id).await;
    repondre(&b, p, g.id, e, true).await.unwrap();
    accepter(&b, g.id).await;
    depart(&b, g.id, vec![e]).await;
    let partis: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_internat_sortie_groupe_participant WHERE sortie_groupe_id = $1 AND etat = 'parti'",
    )
    .bind(g.id)
    .fetch_one(b.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(partis, 1, "précondition : un élève est parti");
    prouver(
        b.ent.pool_verif(),
        Preuve {
            nom: "internat (sortie de groupe)",
            etab: b.etab,
            fonctions: &["ent_internat_sortie_groupe_garde()"],
            masque: vec!["CREATE TEMP TABLE ent_internat_sortie_groupe_participant (LIKE public.ent_internat_sortie_groupe_participant) ON COMMIT DROP".to_string()],
            ecriture: format!(
                "UPDATE public.ent_internat_sortie_groupe SET statut = 'annulee', annule_motif_code = 'meteo' WHERE id = '{}'",
                g.id
            ),
            code: "42501",
            message: "sortie de groupe : des élèves sont partis, l'annulation est refusée",
            qualifiee: false,
        },
    )
    .await;
    let statut: String =
        sqlx::query_scalar("SELECT statut FROM ent_internat_sortie_groupe WHERE id = $1")
            .bind(g.id)
            .fetch_one(b.ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(statut, "acceptee", "la sortie n'a pas été annulée");
}
