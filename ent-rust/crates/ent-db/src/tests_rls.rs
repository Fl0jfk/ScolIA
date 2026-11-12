//! Socle 1 / C4 — RLS pilote (migration 100) : tests avec un rôle applicatif
//! **non propriétaire**, sans BYPASSRLS, dans la base locale de test. Un
//! superutilisateur contourne toujours la RLS : c'est pourquoi la connexion de
//! test habituelle (propriétaire) sert uniquement à préparer les données.
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use crate::tenant_tx::{RlsMode, TenantTx};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

const ROLE: &str = "ent_rls_app";
const ROLE_PERMISSIF: &str = "ent_rls_app_permissif";

struct Env {
    admin: PgPool,
    app: PgPool,
    /// Rôle non propriétaire sans `app.rls_strict` de rôle (mode par défaut).
    app_permissif: PgPool,
    etab_a: Uuid,
    etab_b: Uuid,
    eleve: Uuid,
    user: Uuid,
    fiche_a: Uuid,
    fiche_b: Uuid,
}

async fn preparer() -> Option<Env> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let admin = PgPoolOptions::new().max_connections(2).connect(&url).await.unwrap();

    // Rôle applicatif : ni propriétaire, ni BYPASSRLS, ni superutilisateur.
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(&admin)
    .await
    .unwrap();
    for sql in [
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ent_fiche_sante, ent_psy_dossier TO {ROLE}"),
        // Fail-closed au niveau du rôle : toute connexion qui oublie TenantTx ne voit rien.
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
        // Rôle applicatif « permissif » (mode par défaut, sans réglage strict de rôle).
        format!(
            "DO $$ BEGIN CREATE ROLE {ROLE_PERMISSIF} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
             EXCEPTION WHEN duplicate_object THEN NULL; END $$"
        ),
        format!("GRANT USAGE ON SCHEMA public TO {ROLE_PERMISSIF}"),
    ] {
        sqlx::query(&sql).execute(&admin).await.unwrap();
    }
    // Rechiffrement (D3) : les deux rôles accèdent à toutes les tables chiffrées.
    for (table, _) in crate::champs_chiffres::CHAMPS_CHIFFRES {
        let sql = format!("GRANT SELECT, UPDATE ON {table} TO {ROLE}, {ROLE_PERMISSIF}");
        sqlx::query(&sql).execute(&admin).await.unwrap();
    }
    // Le rechiffrement liste les établissements dans deux tables sans RLS (contexte système nommé).
    for table in ["ent_etablissement", "ent_membership"] {
        let sql = format!("GRANT SELECT ON {table} TO {ROLE}, {ROLE_PERMISSIF}");
        sqlx::query(&sql).execute(&admin).await.unwrap();
    }
    // Connexion applicative = même base, autre rôle.
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE).unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = PgPoolOptions::new().max_connections(2).connect(u.as_str()).await.unwrap();
    u.set_username(ROLE_PERMISSIF).unwrap();
    let app_permissif = PgPoolOptions::new().max_connections(2).connect(u.as_str()).await.unwrap();

    let (etab_a, etab_b) = (cast.etablissement_id, Uuid::new_v4());
    let (eleve, user) = (cast.eleve_lea_id, cast.direction.user_id);
    let (fiche_a, fiche_b) = (Uuid::new_v4(), Uuid::new_v4());
    // Une fiche par établissement pour le même élève (l'unicité est par établissement).
    sqlx::query("DELETE FROM ent_fiche_sante WHERE eleve_id = $1").bind(eleve).execute(&admin).await.unwrap();
    for (id, etab, note) in [(fiche_a, etab_a, "note A"), (fiche_b, etab_b, "note B")] {
        sqlx::query(
            "INSERT INTO ent_fiche_sante (id, etablissement_id, eleve_id, note_confidentielle, updated_by_user_id) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id).bind(etab).bind(eleve).bind(note).bind(user)
        .execute(&admin).await.unwrap();
    }
    Some(Env { admin, app, app_permissif, etab_a, etab_b, eleve, user, fiche_a, fiche_b })
}

async fn nettoyer(e: &Env) {
    sqlx::query("DELETE FROM ent_fiche_sante WHERE eleve_id = $1")
        .bind(e.eleve)
        .execute(&e.admin)
        .await
        .unwrap();
}

async fn compter(c: &mut sqlx::PgConnection) -> i64 {
    sqlx::query("SELECT count(*) FROM ent_fiche_sante")
        .fetch_one(c)
        .await
        .unwrap()
        .get(0)
}

#[tokio::test]
async fn rls_strict_sans_etablissement_ne_voit_rien() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Connexion « oubli de TenantTx » : variable absente + rôle strict -> 0 ligne.
    let mut c = e.app.acquire().await.unwrap();
    assert_eq!(compter(&mut c).await, 0);
    assert_eq!(
        sqlx::query("SELECT count(*) FROM ent_psy_dossier").fetch_one(&mut *c).await.unwrap().get::<i64, _>(0),
        0
    );
    // Une écriture sans établissement est refusée (WITH CHECK).
    let r = sqlx::query(
        "INSERT INTO ent_fiche_sante (id, etablissement_id, eleve_id, updated_by_user_id) VALUES ($1,$2,$3,$4)",
    )
    .bind(Uuid::new_v4()).bind(e.etab_a).bind(Uuid::new_v4()).bind(e.user)
    .execute(&mut *c).await;
    assert!(r.is_err(), "insertion sans app.etablissement_id doit être refusée");
    // Une variable vide équivaut à absente.
    drop(c);
    let mut tx = e.app.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', '', true)").execute(&mut *tx).await.unwrap();
    assert_eq!(compter(&mut tx).await, 0);
    drop(tx);
    nettoyer(&e).await;
}

#[tokio::test]
async fn rls_strict_etablissement_pose_isole_lecture_ecriture() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Sans filtre applicatif sur etablissement_id : c'est la base qui isole.
    let mut a = TenantTx::begin_mode(&e.app, e.etab_a, RlsMode::Strict).await.unwrap();
    assert_eq!(compter(&mut a).await, 1);
    let ids: Vec<Uuid> = sqlx::query("SELECT id FROM ent_fiche_sante")
        .fetch_all(&mut *a).await.unwrap().iter().map(|r| r.get(0)).collect();
    assert_eq!(ids, vec![e.fiche_a]);
    // Lecture ciblée de la ligne de B : invisible.
    let vu: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_fiche_sante WHERE id = $1")
        .bind(e.fiche_b).fetch_optional(&mut *a).await.unwrap();
    assert!(vu.is_none());
    // Mise à jour de B depuis A : 0 ligne touchée ; de A : 1.
    let n = sqlx::query("UPDATE ent_fiche_sante SET note_confidentielle = 'piratée' WHERE id = $1")
        .bind(e.fiche_b).execute(&mut *a).await.unwrap().rows_affected();
    assert_eq!(n, 0);
    let n = sqlx::query("UPDATE ent_fiche_sante SET allergies = 'noix' WHERE id = $1")
        .bind(e.fiche_a).execute(&mut *a).await.unwrap().rows_affected();
    assert_eq!(n, 1);
    // Suppression de B depuis A : 0 ligne.
    let n = sqlx::query("DELETE FROM ent_fiche_sante WHERE id = $1")
        .bind(e.fiche_b).execute(&mut *a).await.unwrap().rows_affected();
    assert_eq!(n, 0);
    a.commit().await.unwrap();

    // Insertion d'une ligne de B depuis A : refusée (WITH CHECK, 42501).
    let mut a = TenantTx::begin_mode(&e.app, e.etab_a, RlsMode::Strict).await.unwrap();
    let r = sqlx::query(
        "INSERT INTO ent_fiche_sante (id, etablissement_id, eleve_id, updated_by_user_id) VALUES ($1,$2,$3,$4)",
    )
    .bind(Uuid::new_v4()).bind(e.etab_b).bind(e.eleve).bind(e.user)
    .execute(&mut *a).await;
    let err = r.expect_err("insertion inter-établissement refusée");
    assert_eq!(err.as_database_error().and_then(|d| d.code()).as_deref(), Some("42501"));
    drop(a);

    // Symétrique : B voit sa ligne, pas celle de A ; A n'a pas été modifiée par B.
    let mut b = TenantTx::begin_mode(&e.app, e.etab_b, RlsMode::Strict).await.unwrap();
    let notes: Vec<String> = sqlx::query("SELECT note_confidentielle FROM ent_fiche_sante")
        .fetch_all(&mut *b).await.unwrap().iter().map(|r| r.get(0)).collect();
    assert_eq!(notes, vec!["note B".to_string()]);
    drop(b);
    // Vérité terrain côté propriétaire (contourne la RLS) : B intacte, A mise à jour.
    let rows = sqlx::query("SELECT id, note_confidentielle, allergies FROM ent_fiche_sante WHERE eleve_id = $1")
        .bind(e.eleve).fetch_all(&e.admin).await.unwrap();
    assert_eq!(rows.len(), 2);
    for r in rows {
        let (id, note, all): (Uuid, String, String) = (r.get(0), r.get(1), r.get(2));
        if id == e.fiche_b {
            assert_eq!((note.as_str(), all.as_str()), ("note B", ""));
        } else {
            assert_eq!(all, "noix");
        }
    }

    // La variable est locale à la transaction : la connexion rendue au pool est « propre ».
    let mut c = e.app.acquire().await.unwrap();
    assert_eq!(compter(&mut c).await, 0);
    drop(c);
    nettoyer(&e).await;
}

#[tokio::test]
async fn rls_non_strict_sans_variable_reste_permissive() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Mode « permissif » (comportement actuel tant que le mode strict n'est pas posé).
    let mut tx = e.app.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.rls_strict', 'off', true)").execute(&mut *tx).await.unwrap();
    assert!(compter(&mut tx).await >= 2, "permissif : les deux établissements visibles");
    drop(tx);
    // Même en permissif, une variable posée filtre.
    let mut tx = TenantTx::begin_mode(&e.app, e.etab_b, RlsMode::Permissif).await.unwrap();
    sqlx::query("SELECT set_config('app.rls_strict', 'off', true)").execute(&mut *tx).await.unwrap();
    assert_eq!(compter(&mut tx).await, 1);
    drop(tx);
    nettoyer(&e).await;
}

#[tokio::test]
async fn rls_mode_off_ne_pose_rien() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let mut tx = TenantTx::begin_mode(&e.app, e.etab_a, RlsMode::Off).await.unwrap();
    let v: Option<String> = sqlx::query("SELECT current_setting('app.etablissement_id', true)")
        .fetch_one(&mut *tx).await.unwrap().get(0);
    assert!(v.unwrap_or_default().is_empty());
    drop(tx);
    nettoyer(&e).await;
}

#[tokio::test]
async fn rls_politiques_presentes_et_forcees() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let n: i64 = sqlx::query(
        "SELECT count(*) FROM pg_class c JOIN pg_policies p ON p.tablename = c.relname \
         WHERE c.relrowsecurity AND c.relforcerowsecurity AND p.policyname = 'ent_rls_etablissement'",
    )
    .fetch_one(&e.admin).await.unwrap().get(0);
    // 22 tables pilotes (migration 100) + 17 finances (S2-9, 206) + 6 santé/accompagnement (S2-11, 215)
    // + 2 identité versionnée (E2) + 15 des lots E3 et E4 + 2 du socle 5 (S0 archive, S1 membres de cours)
    // + 5 du lot E5 (paramétrage par champ et propositions) + 6 du lot E6 (confidentialité,
    // doublons, fusions) + 9 du socle 6 (P1 : agents, engagements, suspensions, fonctions et
    // catalogue, disciplines, qualifications, contacts d'urgence, identifiants) + 7 du lot S2 du
    // socle 5 (profil, paramètres, journal de structure, bâtiments, lieux, disponibilités,
    // indisponibilités) + 11 du lot S5 (offre, cibles, groupes, règles, incompatibilités,
    // prérequis ; enseignements suivis, historique, reprise, à traiter) + 5 du lot S3 (événements
    // du calendrier, cibles, règle de semaines, semaines, historique des fermetures) + 2 du lot S4
    // (cours, services à traiter à la reprise) + 5 du lot S7 (périodisations et cibles, grilles, cibles
    // et créneaux de grille) + 2 du lot P2
    // (casquettes, invitations) + 18 du socle 21 (V0 : évaluations, conseil, bulletins,
    // compétences, journal des périodes, archive des doublons) + 17 de la messagerie et de la
    // communication (M0a, migration 480 : fils, messages, accusés, pièces, modèles, carnet,
    // annonces, notifications, campagnes, rencontres, formulaires) + 1 du lot P3 (interventions
    // de service) + 1 du registre des objets stockés (migration 500, PRA E5) + 4 du lot D-036
    // (droits de délégation et leurs cibles, résolutions d'inscription, détail des signalements d'assiduité)
    // + 4 du lot P4 (missions, périmètres, types, cibles d'attribution) + 150 des lots RLS 580–587 (élèves et responsables, santé et assiduité, finances, communication,
    // vie scolaire, évaluations, inscriptions, structure et accès : tests_rls_lots.rs) + 2 de la
    // messagerie (M0b, migration 481 : réglage de la plage calme, décisions sur les liens à vérifier)
    // + 9 du socle 42 (lot V0a, migrations 640–642 : sortie, série et exclusions, public, étapes, journal,
    // participants et leur fil, encadrants) + 5 du lot V0b (migration 643 : versions de budget, lignes,
    // prix annoncé figé, rappels, documents émis) + 10 du lot D-036 2 (migrations 662, 665, 666, 667 : semaine type, plages de joignabilité, tables de fusion)
    // + 7 du socle 29 (lot P2, migrations 2580–2582 : index des tâches, destinataires, clés d'idempotence, journal, prises en charge, états personnels, réglages).
    assert_eq!(n, 22 + 17 + 6 + 2 + 15 + 2 + 5 + 6 + 9 + 7 + 11 + 5 + 5 + 2 + 2 + 18 + 17 + 1 + 1 + 4 + 4 + 150 + 2 + 9 + 5 + 10 + 7, "tables avec RLS activée et forcée");
    // Le rôle de test n'est ni propriétaire ni contournement.
    let (bypass, sup): (bool, bool) = sqlx::query_as("SELECT rolbypassrls, rolsuper FROM pg_roles WHERE rolname = $1")
        .bind(ROLE).fetch_one(&e.admin).await.unwrap();
    assert!(!bypass && !sup);
    nettoyer(&e).await;
}

#[tokio::test]
async fn role_applicatif_ne_peut_pas_desactiver_le_trigger_du_journal() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Non propriétaire : ni DDL, ni désactivation du trigger d'ajout seul.
    for sql in [
        "ALTER TABLE ent_journal_modif DISABLE TRIGGER ent_journal_modif_ajout_seul_trg",
        "ALTER TABLE ent_fiche_sante DISABLE ROW LEVEL SECURITY",
        "DROP POLICY ent_rls_etablissement ON ent_fiche_sante",
    ] {
        let err = sqlx::query(sql).execute(&e.app).await.expect_err(sql);
        assert_eq!(err.as_database_error().and_then(|d| d.code()).as_deref(), Some("42501"), "{sql}");
    }
    nettoyer(&e).await;
}

#[tokio::test]
async fn migrations_via_url_proprietaire_dediee() {
    let Some(url) = std::env::var("ENT_DATABASE_URL").ok() else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ent = PgEnt::connect(&url).await.expect("connect");
    // Variable absente / vide : comportement historique.
    std::env::remove_var("ENT_DATABASE_MIGRATION_URL");
    ent.migrate().await.expect("migrate sans URL dédiée");
    std::env::set_var("ENT_DATABASE_MIGRATION_URL", "  ");
    ent.migrate().await.expect("migrate URL vide");
    // URL propriétaire locale : utilisée.
    std::env::set_var("ENT_DATABASE_MIGRATION_URL", &url);
    let ok = ent.migrate().await;
    // Hôte distant refusé (garde-fou), jamais de connexion.
    std::env::set_var("ENT_DATABASE_MIGRATION_URL", "postgres://u:p@51.158.57.80:5432/db");
    let ko = ent.migrate().await;
    std::env::remove_var("ENT_DATABASE_MIGRATION_URL");
    ok.expect("migrate via URL propriétaire");
    assert!(matches!(ko, Err(EntDbError::Url(_))));
}

/// Valeur brute stockée (vérité terrain côté propriétaire, contourne la RLS).
async fn brut_fiche(e: &Env, id: Uuid) -> (String, String) {
    let r = sqlx::query("SELECT note_confidentielle, allergies FROM ent_fiche_sante WHERE id = $1")
        .bind(id)
        .fetch_one(&e.admin)
        .await
        .unwrap();
    (r.get(0), r.get(1))
}

/// Fusion C4 + D3 : les dépôts santé et psy, sous rôle non propriétaire en mode
/// strict, lisent l'id existant, scellent et écrivent dans la même `TenantTx`.
#[tokio::test]
async fn rls_strict_depots_sante_psy_chiffrent_avec_l_id_stocke() {
    use crate::infirmerie_repo::PgInfirmerieRepo;
    use crate::psy_repo::PgPsyRepo;
    use domain_eleve::{DossierPsy, FicheSante};
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    crate::tenant_tx::forcer_mode_test(Some(RlsMode::Strict));
    let sante = PgInfirmerieRepo::new(e.app.clone());
    let maintenant = chrono::Utc::now();
    let fiche = |etab: Uuid, note: &str| FicheSante {
        id: Uuid::new_v4(), // différent de l'id stocké : le dépôt doit reprendre ce dernier
        etablissement_id: etab,
        eleve_id: e.eleve,
        allergies: "arachides".into(),
        traitements: String::new(),
        contre_indications: String::new(),
        pai_actif: false,
        medecin_nom: Some("Dr Martin".into()),
        urgence_tel: None,
        note_confidentielle: note.into(),
        updated_by_user_id: e.user,
        updated_at: maintenant,
    };
    // Mise à jour de la fiche existante de A (pré-lecture sous RLS -> id stocké).
    let f = sante.upsert_fiche(&fiche(e.etab_a, "secret A")).await.expect("upsert A");
    assert_eq!(f.id, e.fiche_a);
    let (note, all) = brut_fiche(&e, e.fiche_a).await;
    assert!(note.starts_with("enc:v1:") && all.starts_with("enc:v1:"), "scellé en base");
    let relu = sante.get_fiche(e.etab_a, e.eleve).await.unwrap().expect("relue");
    assert_eq!((relu.id, relu.note_confidentielle.as_str()), (e.fiche_a, "secret A"));
    // La fiche de B est restée intacte (en clair hérité) ; B la relit avec son id.
    assert_eq!(brut_fiche(&e, e.fiche_b).await.0, "note B");
    let f = sante.upsert_fiche(&fiche(e.etab_b, "secret B")).await.expect("upsert B");
    assert_eq!(f.id, e.fiche_b);
    let relu = sante.get_fiche(e.etab_b, e.eleve).await.unwrap().expect("relue B");
    assert_eq!(relu.note_confidentielle, "secret B");

    // Dossier psy : création puis mise à jour avec un autre id proposé.
    let psy = PgPsyRepo::new(e.app.clone());
    sqlx::query("DELETE FROM ent_psy_dossier WHERE eleve_id = $1")
        .bind(e.eleve).execute(&e.admin).await.unwrap();
    let dossier = |note: &str| DossierPsy {
        id: Uuid::new_v4(),
        etablissement_id: e.etab_a,
        eleve_id: e.eleve,
        axes_suivi: "axes".into(),
        contacts_externes: "CMP".into(),
        note_confidentielle: note.into(),
        suivi_actif: true,
        signale_famille: false,
        updated_by_user_id: e.user,
        updated_at: maintenant,
    };
    let d1 = psy.upsert_dossier(&dossier("n1")).await.expect("création");
    let d2 = psy.upsert_dossier(&dossier("n2")).await.expect("mise à jour");
    assert_eq!(d1.id, d2.id, "id de ligne conservé");
    let relu = psy.get_dossier(e.etab_a, e.eleve).await.unwrap().expect("dossier");
    assert_eq!(relu.note_confidentielle, "n2");
    // Vu de B : rien.
    assert!(psy.get_dossier(e.etab_b, e.eleve).await.unwrap().is_none());
    crate::tenant_tx::forcer_mode_test(None);
    sqlx::query("DELETE FROM ent_psy_dossier WHERE eleve_id = $1")
        .bind(e.eleve).execute(&e.admin).await.unwrap();
    nettoyer(&e).await;
}

/// Rechiffrement au démarrage (D3) sous rôle non propriétaire : tâche transverse nommée
/// (`ContexteSysteme::Rechiffrement`) qui traite **un établissement à la fois**. Il fonctionne
/// donc aussi bien sous un rôle en mode strict qu'en mode permissif (docs/securite/RLS_STRICTE.md §4).
#[tokio::test]
async fn rechiffrement_par_etablissement_sous_rls_strict_et_permissif() {
    use crate::champs_chiffres::{rechiffrer_champs_sensibles, ChampsSensibles};
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let champs = ChampsSensibles::depuis_env();
    // Rôle en strict de rôle : `TenantTx` doit poser le contexte (ENT_RLS_MODE=strict ou permissif en production).
    crate::tenant_tx::forcer_mode_test(Some(RlsMode::Strict));
    // Les établissements de la tâche : la racine `ent_etablissement` (A existe déjà via le jeu de labo).
    for (id, slug) in [(e.etab_a, "rls-a"), (e.etab_b, "rls-b")] {
        sqlx::query("INSERT INTO ent_etablissement (id, slug, nom) VALUES ($1, $2 || '-' || substr($1::text, 1, 8), 'RLS') ON CONFLICT (id) DO NOTHING")
            .bind(id).bind(slug).execute(&e.admin).await.unwrap();
    }
    // Rôle en strict : chaque établissement est traité dans son contexte, rien n'est ignoré.
    let rapport = rechiffrer_champs_sensibles(&e.app, &champs).await.expect("strict");
    assert!(rapport.champs_rechiffres >= 2, "le rôle strict rechiffre les deux établissements : {rapport:?}");
    for (id, etab, clair) in [(e.fiche_a, e.etab_a, "note A"), (e.fiche_b, e.etab_b, "note B")] {
        assert!(brut_fiche(&e, id).await.0.starts_with("enc:v1:"));
        let mut tx = TenantTx::begin_mode(&e.app, etab, RlsMode::Strict).await.unwrap();
        let (stocke,): (String,) = sqlx::query_as("SELECT note_confidentielle FROM ent_fiche_sante WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *tx).await.unwrap();
        drop(tx);
        assert_eq!(champs.ouvrir("ent_fiche_sante", "note_confidentielle", id, &stocke).unwrap(), clair);
    }
    // Idempotent, y compris sous le rôle permissif.
    let rapport = rechiffrer_champs_sensibles(&e.app_permissif, &champs).await.expect("permissif");
    assert_eq!(rapport.detail.iter().filter(|(t, _)| t.starts_with("ent_fiche_sante")).count(), 0);
    crate::tenant_tx::forcer_mode_test(None);
    nettoyer(&e).await;
}
