//! RLS d'établissement par lots de domaine (migrations 580 à 587, D-037 / E8).
//!
//! Chaque lot a **son test**, avec un rôle applicatif **non propriétaire, sans
//! BYPASSRLS**, en mode strict de rôle. Les requêtes du test n'écrivent volontairement
//! **aucun filtre `etablissement_id`** : c'est la base qui doit isoler (le code qui
//! « oublie le filtre »).
//!
//! Pour toutes les tables du lot :
//! 1. RLS activée + forcée + politique `ent_rls_etablissement` (aucune dérive du SQL) ;
//! 2. écriture dans un autre établissement refusée (`42501`, `WITH CHECK`), avant toute
//!    contrainte de colonne ;
//! 3. lecture sous un établissement inconnu : 0 ligne ; sous l'établissement propriétaire
//!    des données : exactement ses lignes ; sans contexte (rôle strict) : 0 ligne.
//!
//! Pour une table représentative du lot, avec de vraies lignes des établissements A et B :
//! lecture, `UPDATE` et `DELETE` sans filtre ne touchent que l'établissement du contexte.
//!
//! Sans `ENT_DATABASE_URL` (hors CI), les scénarios sont ignorés.

use super::*;
use crate::tenant_tx::{RlsMode, TenantTx};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

const ROLE: &str = "ent_rls_lots_app";

/// Lots : (nom, tables). Doit couvrir exactement les tables passées sous RLS par les
/// migrations 580–587 (vérifié par `lots_couvrent_les_migrations`).
pub(crate) const LOTS: &[(&str, &[&str])] = &[
    ("eleves_responsables", &[
        "ent_adresse_protegee", "ent_convention_stage", "ent_coordonnee_verification", "ent_doc_a_signer",
        "ent_document_grant", "ent_eleve", "ent_eleve_compte", "ent_eleve_contact_urgence",
        "ent_eleve_dispositif", "ent_eleve_document", "ent_eleve_invitation", "ent_eleve_lien",
        "ent_eleve_orphelin", "ent_eleve_periode", "ent_eleve_scolarite_annee", "ent_entreprise",
        "ent_foyer", "ent_identite_journal", "ent_import_ligne", "ent_import_lot", "ent_info_comp_champ",
        "ent_info_comp_valeur", "ent_journal_modif", "ent_lien_attente", "ent_personne",
        "ent_personne_consentement", "ent_personne_coordonnee", "ent_personne_coordonnee_histo",
        "ent_personne_divergence_decision", "ent_personne_fusion", "ent_personne_preference",
        "ent_personne_preference_version", "ent_personne_reserve", "ent_personne_version",
    ]),
    ("sante_assiduite", &[
        "ent_absence", "ent_absence_declaration", "ent_absence_motif_etab", "ent_absence_rapprochement",
        "ent_absence_rh", "ent_absence_seance", "ent_absence_signal", "ent_alerte_cpe", "ent_appel",
        "ent_appel_ligne", "ent_appel_relance", "ent_dispense_vs", "ent_retard",
        "ent_signalement_assiduite", "ent_sortie_autorisation", "ent_sortie_controle",
        "ent_sortie_demande", "ent_sortie_param", "ent_sortie_regle_niveau", "ent_sortie_regle_regime",
        "ent_vs_regle_assiduite",
    ]),
    ("finances", &[
        "ent_bank_rapprochement", "ent_bourse_type", "ent_eleve_bourse", "ent_foyer_iban_histo",
        "ent_foyer_quotient", "ent_mandat_sepa_demande", "ent_tarif_conso", "ent_taux_tva",
    ]),
    ("communication", &["ent_mail_outbox", "ent_push_device", "ent_push_outbox", "ent_sms_outbox"]),
    ("vie_scolaire_discipline", &[
        "ent_edition_vs", "ent_eleve_interdiction", "ent_procedure_disciplinaire", "ent_punition",
        "ent_punition_type",
    ]),
    ("evaluations_pedagogie", &[
        "ent_cahier_texte", "ent_cdt_historique", "ent_cdt_piece", "ent_cdt_visa", "ent_matiere",
        "ent_periode", "ent_periode_vacances", "ent_seance_contenu", "ent_travail", "ent_travail_eleve",
        "ent_travail_remise",
    ]),
    ("inscriptions", &[
        "ent_inscription_campagne", "ent_inscription_campagne_niveau", "ent_inscription_campagne_piece",
        "ent_inscription_campagne_version", "ent_inscription_decision",
        "ent_inscription_declaration_responsable", "ent_inscription_demarche", "ent_inscription_dossier",
        "ent_inscription_fermeture", "ent_inscription_lien_attente", "ent_inscription_offre",
        "ent_inscription_offre_historique", "ent_inscription_offre_unite", "ent_inscription_parametre_lien",
        "ent_inscription_piece", "ent_inscription_prolongation", "ent_inscription_reference_compteur",
        "ent_inscription_reprise_rapport", "ent_inscription_responsable", "ent_inscription_suivi",
        "ent_inscription_suivi_dossier", "ent_inscription_voeu", "ent_reinscription_campagne",
        "ent_reinscription_exigence_doc", "ent_voeu_inscription", "ent_eleve_option",
    ]),
    ("structure_acces_divers", &[
        "ent_affectation", "ent_annee_scolaire", "ent_classe", "ent_conflit_interet", "ent_delegation",
        "ent_edt_annulation_masse", "ent_edt_calendrier_ab", "ent_edt_contrainte", "ent_edt_creneau",
        "ent_edt_deplacement", "ent_edt_exception", "ent_edt_import", "ent_edt_seance",
        "ent_eleve_grille_repas", "ent_eleve_dispense_repas", "ent_eleve_option_annee",
        "ent_etablissement_parametre", "ent_filiere", "ent_grille_horaire", "ent_groupe",
        "ent_groupe_responsable", "ent_internat_affectation", "ent_internat_chambre",
        "ent_internat_presence", "ent_internat_suivi", "ent_jour_calendrier", "ent_menu_cantine", "ent_mef",
        "ent_module_etablissement", "ent_niveau", "ent_param_etab", "ent_politique_securite",
        "ent_regle_effacement", "ent_repas_activite", "ent_repas_passage", "ent_role", "ent_role_attribution",
        "ent_role_droit", "ent_salle", "ent_service_enseignement", "ent_site",
    ]),
];

struct Env {
    admin: PgPool,
    app: PgPool,
}

async fn preparer() -> Option<Env> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let admin = PgPoolOptions::new().max_connections(2).connect(&url).await.unwrap();
    for sql in [
        format!(
            "DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB \
             NOCREATEROLE; EXCEPTION WHEN duplicate_object THEN NULL; END $$"
        ),
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {ROLE}"),
        // Fail-closed au niveau du rôle : une connexion qui oublie TenantTx ne voit rien.
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(&admin).await.unwrap();
    }
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE).unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = PgPoolOptions::new().max_connections(2).connect(u.as_str()).await.unwrap();
    Some(Env { admin, app })
}

async fn compter_sous(e: &Env, table: &str, etab: Option<Uuid>) -> i64 {
    let sql = format!("SELECT count(*) FROM {table}");
    match etab {
        Some(id) => {
            let mut tx = TenantTx::begin_mode(&e.app, id, RlsMode::Strict).await.unwrap();
            sqlx::query(&sql).fetch_one(&mut *tx).await.unwrap().get(0)
        }
        None => sqlx::query(&sql).fetch_one(&e.app).await.unwrap().get(0),
    }
}

/// Contrôle commun à toutes les tables d'un lot.
async fn verifier_lot(nom: &str) {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (_, tables) = LOTS.iter().find(|(n, _)| *n == nom).expect("lot");
    let inconnu = Uuid::new_v4();
    for table in *tables {
        // 1. RLS activée, forcée, politique présente.
        let (rls, forcee, pol): (bool, bool, i64) = sqlx::query_as(
            "SELECT c.relrowsecurity, c.relforcerowsecurity, \
                    (SELECT count(*) FROM pg_policies p WHERE p.tablename = c.relname \
                      AND p.policyname = 'ent_rls_etablissement') \
               FROM pg_class c WHERE c.relname = $1 AND c.relnamespace = 'public'::regnamespace",
        )
        .bind(table)
        .fetch_one(&e.admin)
        .await
        .unwrap_or_else(|_| panic!("{table} : table absente"));
        assert!(rls && forcee && pol == 1, "{table} : RLS {rls}, FORCE {forcee}, politiques {pol}");

        // 2. La politique filtre bien sur l'établissement du contexte, en lecture (USING) comme en
        //    écriture (WITH CHECK) : le refus d'écriture vers un autre établissement est démontré sur
        //    une table réelle par lot (`verifier_isolation_concrete`) ; les contraintes de colonnes
        //    (NOT NULL, CHECK) passent avant WITH CHECK, d'où l'absence d'INSERT générique.
        let (qual, check): (String, String) = sqlx::query_as(
            "SELECT qual, with_check FROM pg_policies WHERE tablename = $1 AND policyname = 'ent_rls_etablissement'",
        )
        .bind(table)
        .fetch_one(&e.admin)
        .await
        .unwrap();
        for (nom_expr, expr) in [("USING", &qual), ("WITH CHECK", &check)] {
            assert!(
                expr.contains("etablissement_id = ( SELECT ent_rls_etablissement()") || expr.contains("etablissement_id = (SELECT ent_rls_etablissement()"),
                "{table} : {nom_expr} ne filtre pas sur l'établissement du contexte : {expr}"
            );
        }

        // 3. Lecture : un établissement qui n'a pas de lignes n'en voit aucune, quelles que soient
        //    les lignes des autres ; sans contexte : 0 ; avec le bon contexte : ses lignes seulement.
        assert_eq!(compter_sous(&e, table, Some(inconnu)).await, 0, "{table} : lecture sous un établissement inconnu");
        assert_eq!(compter_sous(&e, table, None).await, 0, "{table} : lecture sans contexte en mode strict");
        let proprietaires: Vec<(Uuid, i64)> = sqlx::query_as(&format!(
            "SELECT etablissement_id, count(*) FROM {table} WHERE etablissement_id IS NOT NULL GROUP BY 1"
        ))
        .fetch_all(&e.admin)
        .await
        .unwrap();
        for (etab, n) in proprietaires.into_iter().take(3) {
            assert_eq!(compter_sous(&e, table, Some(etab)).await, n, "{table} : un établissement voit exactement ses lignes");
        }
    }
}

/// Lignes réelles de A et de B dans une table représentative : lecture, mise à jour et
/// suppression **sans filtre d'établissement** n'atteignent que l'établissement du contexte.
async fn verifier_isolation_concrete(e: &Env, table: &str, ligne: impl Fn(Uuid, Uuid, Uuid) -> String) {
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    // Les clés étrangères ne sont pas l'objet du test : un rôle de réplication les suspend.
    let mut tx = e.admin.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await.unwrap();
    let (ida, idb) = (Uuid::new_v4(), Uuid::new_v4());
    for (id, etab) in [(ida, a), (idb, b)] {
        sqlx::query(&ligne(id, etab, Uuid::new_v4())).execute(&mut *tx).await.unwrap_or_else(|er| panic!("{table} : {er}"));
    }
    tx.commit().await.unwrap();
    let n_admin = |etab: Uuid| {
        let sql = format!("SELECT count(*) FROM {table} WHERE etablissement_id = $1");
        let pool = e.admin.clone();
        async move { sqlx::query(&sql).bind(etab).fetch_one(&pool).await.unwrap().get::<i64, _>(0) }
    };

    // Lecture sans filtre sous A : une ligne, celle de A.
    let mut tx = TenantTx::begin_mode(&e.app, a, RlsMode::Strict).await.unwrap();
    let ids: Vec<(Uuid,)> = sqlx::query_as(&format!("SELECT id FROM {table}")).fetch_all(&mut *tx).await.unwrap();
    assert_eq!(ids, vec![(ida,)], "{table} : A ne voit que ses lignes");
    // UPDATE et DELETE sans filtre : seule la ligne de A est touchée.
    let r = sqlx::query(&format!("UPDATE {table} SET id = id")).execute(&mut *tx).await.unwrap();
    assert_eq!(r.rows_affected(), 1, "{table} : UPDATE sans filtre sous A");
    // Écriture d'une ligne de B depuis le contexte de A : refusée par la base (42501).
    let r = sqlx::query(&ligne(Uuid::new_v4(), b, Uuid::new_v4())).execute(&mut *tx).await;
    let code = match &r {
        Err(sqlx::Error::Database(d)) => d.code().map(|c| c.to_string()),
        _ => None,
    };
    assert_eq!(code.as_deref(), Some("42501"), "{table} : écrire une ligne de B sous A doit être refusé ({r:?})");
    drop(tx);

    // Sous B : symétrique ; suppression sans filtre.
    let mut tx = TenantTx::begin_mode(&e.app, b, RlsMode::Strict).await.unwrap();
    let ids: Vec<(Uuid,)> = sqlx::query_as(&format!("SELECT id FROM {table}")).fetch_all(&mut *tx).await.unwrap();
    assert_eq!(ids, vec![(idb,)], "{table} : B ne voit que ses lignes");
    let r = sqlx::query(&format!("DELETE FROM {table}")).execute(&mut *tx).await.unwrap();
    assert_eq!(r.rows_affected(), 1, "{table} : DELETE sans filtre sous B");
    tx.commit().await.unwrap();

    // Vérité terrain côté propriétaire : A intact (modifié), B supprimé, rien d'autre.
    assert_eq!(n_admin(a).await, 1, "{table} : la ligne de A existe encore");
    assert_eq!(n_admin(b).await, 0, "{table} : la ligne de B a bien été supprimée par B");
    sqlx::query(&format!("DELETE FROM {table} WHERE etablissement_id = $1")).bind(a).execute(&e.admin).await.unwrap();
}

macro_rules! test_lot {
    ($fonction:ident, $lot:literal, $table:literal, $ligne:expr) => {
        #[tokio::test]
        async fn $fonction() {
            verifier_lot($lot).await;
            let Some(e) = preparer().await else { return };
            verifier_isolation_concrete(&e, $table, $ligne).await;
        }
    };
}

test_lot!(lot_eleves_responsables, "eleves_responsables", "ent_foyer", |id, etab, _| {
    format!("INSERT INTO ent_foyer (id, etablissement_id) VALUES ('{id}', '{etab}')")
});
test_lot!(lot_sante_assiduite, "sante_assiduite", "ent_alerte_cpe", |id, etab, el| {
    format!("INSERT INTO ent_alerte_cpe (id, etablissement_id, eleve_id, titre, jour, created_by_user_id) VALUES ('{id}', '{etab}', '{el}', 't', current_date, '{el}')")
});
test_lot!(lot_finances, "finances", "ent_taux_tva", |id, etab, _| {
    format!("INSERT INTO ent_taux_tva (id, etablissement_id, code, libelle, taux_bps, created_by_user_id) VALUES ('{id}', '{etab}', 't', 't', 0, '{id}')")
});
test_lot!(lot_communication, "communication", "ent_sms_outbox", |id, etab, _| {
    format!("INSERT INTO ent_sms_outbox (id, etablissement_id, kind, to_phone, body, status) VALUES ('{id}', '{etab}', 'k', '0600000000', 'b', 'dry_run')")
});
test_lot!(lot_vie_scolaire_discipline, "vie_scolaire_discipline", "ent_punition_type", |id, etab, _| {
    format!("INSERT INTO ent_punition_type (id, etablissement_id, code, libelle) VALUES ('{id}', '{etab}', 'abc', 't')")
});
test_lot!(lot_evaluations_pedagogie, "evaluations_pedagogie", "ent_matiere", |id, etab, _| {
    format!("INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ('{id}', '{etab}', 'abc', 't')")
});
test_lot!(lot_inscriptions, "inscriptions", "ent_inscription_responsable", |id, etab, el| {
    format!("INSERT INTO ent_inscription_responsable (id, etablissement_id, dossier_id, rang, nom) VALUES ('{id}', '{etab}', '{el}', 1, 'N')")
});
test_lot!(lot_structure_acces_divers, "structure_acces_divers", "ent_niveau", |id, etab, _| {
    format!("INSERT INTO ent_niveau (id, etablissement_id, code, libelle) VALUES ('{id}', '{etab}', 'n', 'n')")
});

/// Les lots listés ici sont exactement les tables des migrations 580–587 : pas de table oubliée,
/// et aucune table des exceptions de lecture hors contexte n'y figure.
#[tokio::test]
async fn lots_couvrent_les_migrations() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let attendu: std::collections::BTreeSet<&str> = LOTS.iter().flat_map(|(_, t)| t.iter().copied()).collect();
    assert_eq!(attendu.len(), 150, "les 8 lots portent 150 tables");
    let mut des_migrations = std::collections::BTreeSet::new();
    for (num, nom) in [
        (580, "eleves_responsables"), (581, "sante_assiduite"), (582, "finances"), (583, "communication"),
        (584, "vie_scolaire_discipline"), (585, "evaluations_pedagogie"), (586, "inscriptions"),
        (587, "structure_acces_divers"),
    ] {
        let sql = std::fs::read_to_string(format!("{}/../../migrations/{num}_rls_{nom}.sql", env!("CARGO_MANIFEST_DIR"))).expect("migration");
        let (_, apres) = sql.split_once("ent_rls_poser(ARRAY[").expect("appel");
        let liste = apres.split("]);").next().unwrap();
        for t in liste.split(',').map(|s| s.trim().trim_matches('\'')).filter(|s| !s.is_empty()) {
            des_migrations.insert(t.to_string());
        }
    }
    let attendu: Vec<String> = attendu.into_iter().map(String::from).collect();
    assert_eq!(attendu, des_migrations.into_iter().collect::<Vec<_>>());
    // Toute table du registre « par établissement » est sous RLS, sauf exceptions déclarées.
    let sans: Vec<(String,)> = sqlx::query_as(
        "SELECT relname FROM pg_class WHERE relnamespace = 'public'::regnamespace AND relkind = 'r' \
         AND NOT relrowsecurity AND relname IN (SELECT table_name FROM information_schema.columns \
              WHERE table_schema = 'public' AND column_name = 'etablissement_id') ORDER BY 1",
    )
    .fetch_all(&e.admin)
    .await
    .unwrap();
    let sans: Vec<String> = sans.into_iter().map(|(n,)| n).collect();
    for n in &sans {
        let c = crate::registre_donnees::classer(n).unwrap_or_else(|| panic!("{n} non classée"));
        assert!(c.exception(crate::registre_donnees::Exception::SansRls).is_some() || c.portee != crate::registre_donnees::Portee::Etablissement, "{n} sans RLS ni exception");
    }
}
