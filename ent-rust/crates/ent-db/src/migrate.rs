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
    include_str!("../../../migrations/084_auth_guard.sql"),
    include_str!("../../../migrations/085_auth_reset.sql"),
    include_str!("../../../migrations/086_annonce.sql"),
    include_str!("../../../migrations/087_retard.sql"),
    include_str!("../../../migrations/088_competence.sql"),
    include_str!("../../../migrations/089_passkey.sql"),
    include_str!("../../../migrations/090_oidc.sql"),
    include_str!("../../../migrations/091_journal_ajout_seul.sql"),
    include_str!("../../../migrations/092_referentiel_scolaire.sql"),
    include_str!("../../../migrations/093_passkey_par_personne.sql"),
    include_str!("../../../migrations/094_limitation_debit.sql"),
    include_str!("../../../migrations/095_session_assurance.sql"),
    include_str!("../../../migrations/096_etablissement_domaines.sql"),
    include_str!("../../../migrations/097_recuperation_passkey.sql"),
    include_str!("../../../migrations/098_roles_parametrables.sql"),
    include_str!("../../../migrations/099_oidc_securite.sql"),
    include_str!("../../../migrations/100_rls_pilote.sql"),
    include_str!("../../../migrations/102_outbox.sql"),
    include_str!("../../../migrations/110_referentiel_structure.sql"),
    include_str!("../../../migrations/111_classe_id.sql"),
    include_str!("../../../migrations/112_classe_id_rattachement.sql"),
    include_str!("../../../migrations/120_personne.sql"),
    include_str!("../../../migrations/121_lien_autorite.sql"),
    include_str!("../../../migrations/122_contact_urgence.sql"),
    include_str!("../../../migrations/123_personne_fusion.sql"),
    include_str!("../../../migrations/130_vs_absence_seance.sql"),
    include_str!("../../../migrations/131_vs_declaration.sql"),
    include_str!("../../../migrations/132_vs_verification_appel.sql"),
    include_str!("../../../migrations/133_vs_punition.sql"),
    include_str!("../../../migrations/134_vs_discipline.sql"),
    include_str!("../../../migrations/140_inscription_campagne.sql"),
    include_str!("../../../migrations/141_inscription_dossier.sql"),
    include_str!("../../../migrations/142_preinscription_verification.sql"),
    include_str!("../../../migrations/143_mandat_sepa_demande.sql"),
    include_str!("../../../migrations/144_import_lot.sql"),
    include_str!("../../../migrations/145_inscription_invitation.sql"),
    include_str!("../../../migrations/150_cahier_textes.sql"),
    include_str!("../../../migrations/170_edt_admin.sql"),
    include_str!("../../../migrations/171_edt_remplacement.sql"),
    include_str!("../../../migrations/172_delegation.sql"),
    include_str!("../../../migrations/180_eva_grille.sql"),
    include_str!("../../../migrations/181_eva_conseil.sql"),
    include_str!("../../../migrations/182_eva_bulletin.sql"),
    include_str!("../../../migrations/190_notifications.sql"),
    include_str!("../../../migrations/191_campagnes.sql"),
    include_str!("../../../migrations/192_rencontres.sql"),
    include_str!("../../../migrations/193_formulaires.sql"),
    include_str!("../../../migrations/200_fin_parametres_tarifs.sql"),
    include_str!("../../../migrations/201_fin_sources.sql"),
    include_str!("../../../migrations/202_fin_sepa.sql"),
    include_str!("../../../migrations/203_fin_reglements_lettrage.sql"),
    include_str!("../../../migrations/204_fin_relances.sql"),
    include_str!("../../../migrations/205_fin_export_comptable.sql"),
    include_str!("../../../migrations/206_fin_rls.sql"),
    include_str!("../../../migrations/210_sante_pai_complet.sql"),
    include_str!("../../../migrations/211_sante_contrainte.sql"),
    include_str!("../../../migrations/212_accompagnement.sql"),
    include_str!("../../../migrations/213_infirmerie_passage_suite.sql"),
    include_str!("../../../migrations/214_sante_protocole.sql"),
    include_str!("../../../migrations/215_sante_rls.sql"),
    include_str!("../../../migrations/216_module_sante.sql"),
    include_str!("../../../migrations/240_eleve_cles_composees.sql"),
    include_str!("../../../migrations/250_inscriptions_i0.sql"),
    include_str!("../../../migrations/255_email_confirmation.sql"),
    include_str!("../../../migrations/260_administratif_a0.sql"),
    include_str!("../../../migrations/265_etablissement_fuseau.sql"),
    include_str!("../../../migrations/266_personne_reserve.sql"),
    include_str!("../../../migrations/267_adresse_protegee.sql"),
    include_str!("../../../migrations/268_lien_confirmation_voie.sql"),
    include_str!("../../../migrations/269_inscription_responsable_reserve.sql"),
    include_str!("../../../migrations/270_conflit_interet.sql"),
    include_str!("../../../migrations/271_eleve_interdiction.sql"),
    include_str!("../../../migrations/272_contact_declarant.sql"),
    include_str!("../../../migrations/280_eleve_identite_versionnee.sql"),
    include_str!("../../../migrations/281_eleve_identifiants.sql"),
    include_str!("../../../migrations/282_eleve_numero.sql"),
    include_str!("../../../migrations/283_eleve_scolarisation.sql"),
    include_str!("../../../migrations/284_eleve_inscription.sql"),
    include_str!("../../../migrations/285_eleve_capacite_majorite.sql"),
    include_str!("../../../migrations/286_eleve_options.sql"),
    include_str!("../../../migrations/300_inscription_annuelle_prevue.sql"),
    include_str!("../../../migrations/301_inscriptions_i1_dossier.sql"),
    include_str!("../../../migrations/330_structure_garde_fous.sql"),
    include_str!("../../../migrations/331_cours_membres.sql"),
    include_str!("../../../migrations/350_acces_etat.sql"),
    include_str!("../../../migrations/351_attributions_datees.sql"),
    include_str!("../../../migrations/352_agents.sql"),
    include_str!("../../../migrations/353_agents_fonctions.sql"),
    include_str!("../../../migrations/357_casquettes_invitations.sql"),
    include_str!("../../../migrations/361_agent_identifiants.sql"),
    include_str!("../../../migrations/370_edt_creneau_groupe.sql"),
];

/// Tag de la **dernière** migration du socle — exposé par `/health` pour qu'un
/// appelant (orchestrateur, PRA, support) sache quel schéma tourne réellement.
/// Le test `migration_tag_suit_la_derniere_migration` échoue si on ajoute un
/// fichier sans mettre à jour cette constante.
pub const MIGRATION_TAG: &str = "370_edt_creneau_groupe";

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

    for stmt in split_statements(&without_comments) {
        let cleaned = stmt.trim();
        if cleaned.is_empty() {
            continue;
        }
        sqlx::query(cleaned).execute(pool).await?;
    }
    Ok(())
}

/// Découpe sur `;` **hors** blocs `$$ … $$` (corps de fonction plpgsql).
pub(crate) fn split_statements(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut dans_dollar = false;
    let mut rest = sql;
    while let Some(c) = rest.chars().next() {
        if rest.starts_with("$$") {
            dans_dollar = !dans_dollar;
            cur.push_str("$$");
            rest = &rest[2..];
            continue;
        }
        if c == ';' && !dans_dollar {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
        rest = &rest[c.len_utf8()..];
    }
    out.push(cur);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Garde anti-dérive : le tag annoncé par `/health` suit le dernier fichier
    /// de `migrations/`. Ajouter `092_xxx.sql` sans bumper `MIGRATION_TAG` fait
    /// échouer ce test (le tag est une preuve de schéma, il ne doit pas mentir).
    #[test]
    fn migration_tag_suit_la_derniere_migration() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/../../../migrations");
        let mut noms: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("lecture {}: {e}", dir.display()))
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".sql"))
            .collect();
        noms.sort();
        let dernier = noms.last().expect("au moins une migration");
        let dernier = dernier.trim_end_matches(".sql");
        assert_eq!(
            dernier, MIGRATION_TAG,
            "MIGRATION_TAG doit pointer la dernière migration ({dernier})"
        );

        // Tous les fichiers sont compilés (aucune migration oubliée dans MIGRATIONS).
        assert_eq!(
            MIGRATIONS.len(),
            noms.len(),
            "chaque fichier de migrations/ doit être dans MIGRATIONS"
        );
    }
}
