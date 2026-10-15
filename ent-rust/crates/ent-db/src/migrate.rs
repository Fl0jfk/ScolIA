use sqlx::PgPool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MigrateError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

const MIGRATIONS: &[&str] = &[
    include_str!("../../../migrations/001_ent_socle.sql"),
    include_str!("../../../migrations/002_drop_effectif_doublon.sql"),
    include_str!("../../../migrations/003_ent_absence.sql"),
    include_str!("../../../migrations/004_absence_reshape.sql"),
    include_str!("../../../migrations/005_ent_auth.sql"),
    include_str!("../../../migrations/006_ent_eleve_periodes.sql"),
    include_str!("../../../migrations/007_ent_foyer_liens.sql"),
    include_str!("../../../migrations/008_ent_matiere_note.sql"),
    include_str!("../../../migrations/009_ent_appel.sql"),
    include_str!("../../../migrations/010_absence_justif.sql"),
    include_str!("../../../migrations/011_absence_decisions.sql"),
    include_str!("../../../migrations/012_absence_signal.sql"),
    include_str!("../../../migrations/013_absence_motif_etab.sql"),
    include_str!("../../../migrations/014_eleve_compte.sql"),
    include_str!("../../../migrations/015_dossier_eleve.sql"),
    include_str!("../../../migrations/016_eleve_documents.sql"),
    include_str!("../../../migrations/017_eleve_repas_bourses.sql"),
    include_str!("../../../migrations/018_eleve_continuum.sql"),
    include_str!("../../../migrations/019_vs_sanctions.sql"),
    include_str!("../../../migrations/020_vs_carnet.sql"),
    include_str!("../../../migrations/021_edt_creneau_appel.sql"),
    include_str!("../../../migrations/022_note_periode_devoir.sql"),
    include_str!("../../../migrations/023_messagerie.sql"),
    include_str!("../../../migrations/024_facturation.sql"),
    include_str!("../../../migrations/025_encaissements.sql"),
    include_str!("../../../migrations/026_echeances.sql"),
    include_str!("../../../migrations/027_quittances.sql"),
    include_str!("../../../migrations/028_reinscription_doc_exigences.sql"),
    include_str!("../../../migrations/029_reinscription_acompte.sql"),
    include_str!("../../../migrations/030_sepa.sql"),
    include_str!("../../../migrations/031_sepa_rejet_motif.sql"),
    include_str!("../../../migrations/032_fac_rappels.sql"),
    include_str!("../../../migrations/033_echeance_bourse.sql"),
    include_str!("../../../migrations/034_repas_dispense.sql"),
    include_str!("../../../migrations/035_absence_signal_workflow.sql"),
    include_str!("../../../migrations/036_repas_passage_badge.sql"),
    include_str!("../../../migrations/037_msg_accuse.sql"),
    include_str!("../../../migrations/038_msg_piece.sql"),
    include_str!("../../../migrations/039_eleve_invitation.sql"),
    include_str!("../../../migrations/040_infirmerie_passage.sql"),
    include_str!("../../../migrations/041_psy_entretien.sql"),
    include_str!("../../../migrations/042_edt_exception.sql"),
    include_str!("../../../migrations/043_absence_signal_alerte_famille.sql"),
    include_str!("../../../migrations/044_carnet_notif_famille.sql"),
    include_str!("../../../migrations/045_fiche_sante.sql"),
    include_str!("../../../migrations/046_psy_rdv.sql"),
    include_str!("../../../migrations/047_msg_modele_archive.sql"),
    include_str!("../../../migrations/048_edt_semaine_ab.sql"),
    include_str!("../../../migrations/049_cahier_texte.sql"),
    include_str!("../../../migrations/050_msg_priorite_dossier.sql"),
    include_str!("../../../migrations/051_psy_dossier.sql"),
    include_str!("../../../migrations/052_pai.sql"),
    include_str!("../../../migrations/053_pai_piece.sql"),
    include_str!("../../../migrations/054_vs_commission.sql"),
    include_str!("../../../migrations/055_mail_outbox.sql"),
    include_str!("../../../migrations/056_note_appreciation.sql"),
    include_str!("../../../migrations/057_sms_telephone.sql"),
    include_str!("../../../migrations/058_push_devices.sql"),
    include_str!("../../../migrations/059_bank_object.sql"),
    include_str!("../../../migrations/060_edt_import.sql"),
    include_str!("../../../migrations/061_signal_commission.sql"),
    include_str!("../../../migrations/062_sortie_autorisation.sql"),
    include_str!("../../../migrations/063_sortie_regle_niveau.sql"),
    include_str!("../../../migrations/064_sortie_regle_regime.sql"),
    include_str!("../../../migrations/065_sortie_site_edt.sql"),
    include_str!("../../../migrations/066_internat.sql"),
    include_str!("../../../migrations/067_dispense_vs.sql"),
    include_str!("../../../migrations/068_journal_modif.sql"),
    include_str!("../../../migrations/069_alerte_cpe.sql"),
    include_str!("../../../migrations/070_doc_a_signer.sql"),
    include_str!("../../../migrations/071_conseil_classe.sql"),
    include_str!("../../../migrations/072_info_comp.sql"),
    include_str!("../../../migrations/073_vacances.sql"),
    include_str!("../../../migrations/074_menu_cantine.sql"),
    include_str!("../../../migrations/075_quotient_tva.sql"),
    include_str!("../../../migrations/076_param_salle.sql"),
    include_str!("../../../migrations/077_filiere_voeu.sql"),
    include_str!("../../../migrations/078_inscription_iban.sql"),
    include_str!("../../../migrations/079_tarif_conso.sql"),
    include_str!("../../../migrations/080_absence_rh.sql"),
    include_str!("../../../migrations/081_edition_vs.sql"),
    include_str!("../../../migrations/082_stage.sql"),
    include_str!("../../../migrations/083_edt_contrainte.sql"),
];

/// Clé advisory dédiée au chantier ENT (évite les courses CREATE TABLE en tests parallèles).
const ENT_MIGRATE_LOCK: i64 = 0x454E_545F_4D49_4752; // "ENT_MIGR"

pub async fn apply_socle_migrations(pool: &PgPool) -> Result<(), MigrateError> {
    // Le verrou advisory est lié à la *session* Postgres : lock + unlock
    // doivent passer sur la même connexion (sinon le pool laisse un ExclusiveLock
    // orphelin et bloque le prochain migrate — lab / ent-api concurrent).
    let mut lock_conn = pool.acquire().await?;
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(ENT_MIGRATE_LOCK)
        .execute(&mut *lock_conn)
        .await?;

    let result = apply_all(pool).await;

    let unlock = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(ENT_MIGRATE_LOCK)
        .execute(&mut *lock_conn)
        .await;
    drop(lock_conn);
    result?;
    unlock?;
    Ok(())
}

async fn apply_all(pool: &PgPool) -> Result<(), MigrateError> {
    for sql in MIGRATIONS {
        apply_sql_file(pool, sql).await?;
    }
    Ok(())
}

async fn apply_sql_file(pool: &PgPool, sql: &str) -> Result<(), MigrateError> {
    // Retirer les commentaires **avant** de découper sur `;`
    // (sinon un `;` dans un commentaire devient une « requête »).
    let without_comments: String = sql
        .lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");

    for stmt in without_comments.split(';') {
        let cleaned = stmt.trim();
        if cleaned.is_empty() {
            continue;
        }
        sqlx::query(cleaned).execute(pool).await?;
    }
    Ok(())
}
