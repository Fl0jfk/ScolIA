use sqlx::PgPool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MigrateError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    /// `ent_migration_figee` nomme un fichier qui n'existe pas dans `MIGRATIONS` : on ne fige pas à l'aveugle.
    #[error("ent_migration_figee : fichier de migration inconnu « {0} » (on ne fige qu'un fichier existant)")]
    FigeInconnu(String),
}

/// Une migration : (nom du fichier sans extension, texte). Le nom permet au lanceur de **sauter** les
/// fichiers « figés » (table `ent_migration_figee`, migration 670).
macro_rules! m {
    ($nom:literal) => {
        ($nom, include_str!(concat!("../../../migrations/", $nom, ".sql")))
    };
}

pub(crate) const MIGRATIONS: &[(&str, &str)] = &[
    m!("001_ent_socle"),
    m!("002_drop_effectif_doublon"),
    m!("003_ent_absence"),
    m!("004_absence_reshape"),
    m!("005_ent_auth"),
    m!("006_ent_eleve_periodes"),
    m!("007_ent_foyer_liens"),
    m!("008_ent_matiere_note"),
    m!("009_ent_appel"),
    m!("010_absence_justif"),
    m!("011_absence_decisions"),
    m!("012_absence_signal"),
    m!("013_absence_motif_etab"),
    m!("014_eleve_compte"),
    m!("015_dossier_eleve"),
    m!("016_eleve_documents"),
    m!("017_eleve_repas_bourses"),
    m!("018_eleve_continuum"),
    m!("019_vs_sanctions"),
    m!("020_vs_carnet"),
    m!("021_edt_creneau_appel"),
    m!("022_note_periode_devoir"),
    m!("023_messagerie"),
    m!("024_facturation"),
    m!("025_encaissements"),
    m!("026_echeances"),
    m!("027_quittances"),
    m!("028_reinscription_doc_exigences"),
    m!("029_reinscription_acompte"),
    m!("030_sepa"),
    m!("031_sepa_rejet_motif"),
    m!("032_fac_rappels"),
    m!("033_echeance_bourse"),
    m!("034_repas_dispense"),
    m!("035_absence_signal_workflow"),
    m!("036_repas_passage_badge"),
    m!("037_msg_accuse"),
    m!("038_msg_piece"),
    m!("039_eleve_invitation"),
    m!("040_infirmerie_passage"),
    m!("041_psy_entretien"),
    m!("042_edt_exception"),
    m!("043_absence_signal_alerte_famille"),
    m!("044_carnet_notif_famille"),
    m!("045_fiche_sante"),
    m!("046_psy_rdv"),
    m!("047_msg_modele_archive"),
    m!("048_edt_semaine_ab"),
    m!("049_cahier_texte"),
    m!("050_msg_priorite_dossier"),
    m!("051_psy_dossier"),
    m!("052_pai"),
    m!("053_pai_piece"),
    m!("054_vs_commission"),
    m!("055_mail_outbox"),
    m!("056_note_appreciation"),
    m!("057_sms_telephone"),
    m!("058_push_devices"),
    m!("059_bank_object"),
    m!("060_edt_import"),
    m!("061_signal_commission"),
    m!("062_sortie_autorisation"),
    m!("063_sortie_regle_niveau"),
    m!("064_sortie_regle_regime"),
    m!("065_sortie_site_edt"),
    m!("066_internat"),
    m!("067_dispense_vs"),
    m!("068_journal_modif"),
    m!("069_alerte_cpe"),
    m!("070_doc_a_signer"),
    m!("071_conseil_classe"),
    m!("072_info_comp"),
    m!("073_vacances"),
    m!("074_menu_cantine"),
    m!("075_quotient_tva"),
    m!("076_param_salle"),
    m!("077_filiere_voeu"),
    m!("078_inscription_iban"),
    m!("079_tarif_conso"),
    m!("080_absence_rh"),
    m!("081_edition_vs"),
    m!("082_stage"),
    m!("083_edt_contrainte"),
    m!("084_auth_guard"),
    m!("085_auth_reset"),
    m!("086_annonce"),
    m!("087_retard"),
    m!("088_competence"),
    m!("089_passkey"),
    m!("090_oidc"),
    m!("091_journal_ajout_seul"),
    m!("092_referentiel_scolaire"),
    m!("093_passkey_par_personne"),
    m!("094_limitation_debit"),
    m!("095_session_assurance"),
    m!("096_etablissement_domaines"),
    m!("097_recuperation_passkey"),
    m!("098_roles_parametrables"),
    m!("099_oidc_securite"),
    m!("100_rls_pilote"),
    m!("102_outbox"),
    m!("110_referentiel_structure"),
    m!("111_classe_id"),
    m!("112_classe_id_rattachement"),
    m!("120_personne"),
    m!("121_lien_autorite"),
    m!("122_contact_urgence"),
    m!("123_personne_fusion"),
    m!("130_vs_absence_seance"),
    m!("131_vs_declaration"),
    m!("132_vs_verification_appel"),
    m!("133_vs_punition"),
    m!("134_vs_discipline"),
    m!("140_inscription_campagne"),
    m!("141_inscription_dossier"),
    m!("142_preinscription_verification"),
    m!("143_mandat_sepa_demande"),
    m!("144_import_lot"),
    m!("145_inscription_invitation"),
    m!("150_cahier_textes"),
    m!("170_edt_admin"),
    m!("171_edt_remplacement"),
    m!("172_delegation"),
    m!("180_eva_grille"),
    m!("181_eva_conseil"),
    m!("182_eva_bulletin"),
    m!("190_notifications"),
    m!("191_campagnes"),
    m!("192_rencontres"),
    m!("193_formulaires"),
    m!("200_fin_parametres_tarifs"),
    m!("201_fin_sources"),
    m!("202_fin_sepa"),
    m!("203_fin_reglements_lettrage"),
    m!("204_fin_relances"),
    m!("205_fin_export_comptable"),
    m!("206_fin_rls"),
    m!("210_sante_pai_complet"),
    m!("211_sante_contrainte"),
    m!("212_accompagnement"),
    m!("213_infirmerie_passage_suite"),
    m!("214_sante_protocole"),
    m!("215_sante_rls"),
    m!("216_module_sante"),
    m!("240_eleve_cles_composees"),
    m!("250_inscriptions_i0"),
    m!("255_email_confirmation"),
    m!("260_administratif_a0"),
    m!("265_etablissement_fuseau"),
    m!("266_personne_reserve"),
    m!("267_adresse_protegee"),
    m!("268_lien_confirmation_voie"),
    m!("269_inscription_responsable_reserve"),
    m!("270_conflit_interet"),
    m!("271_eleve_interdiction"),
    m!("272_contact_declarant"),
    m!("273_personne_coordonnee"),
    m!("274_personne_coordonnee_projection"),
    m!("275_personne_verification_preferences"),
    m!("276_personne_version_divergence"),
    m!("280_eleve_identite_versionnee"),
    m!("281_eleve_identifiants"),
    m!("282_eleve_numero"),
    m!("283_eleve_scolarisation"),
    m!("284_eleve_inscription"),
    m!("285_eleve_capacite_majorite"),
    m!("286_eleve_options"),
    m!("287_eleve_param_champ"),
    m!("288_eleve_proposition"),
    m!("289_eleve_proposition_garde"),
    m!("290_eleve_recherche"),
    m!("291_eleve_doublon"),
    m!("292_eleve_fusion"),
    m!("293_eleve_fusion_gardes"),
    m!("294_eleve_fusion_securite"),
    m!("300_inscription_annuelle_prevue"),
    m!("301_inscriptions_i1_dossier"),
    m!("302_inscription_campagne_modele"),
    m!("303_inscription_offre"),
    m!("304_inscription_demarche_dossier"),
    m!("305_inscription_declaration_responsable"),
    m!("306_inscription_offre_reprise_places"),
    m!("307_inscription_fermeture_prolongation"),
    m!("308_inscription_validation_annulee"),
    m!("309_inscription_envoi_information"),
    m!("330_structure_garde_fous"),
    m!("331_cours_membres"),
    m!("332_structure_profil_parametres"),
    m!("333_structure_calendrier"),
    m!("334_structure_fermeture"),
    m!("335_structure_periodes"),
    m!("336_structure_grilles"),
    m!("337_structure_lieux"),
    m!("338_structure_offre"),
    m!("340_eleve_enseignement"),
    m!("341_cours"),
    m!("350_acces_etat"),
    m!("351_attributions_datees"),
    m!("352_agents"),
    m!("353_agents_fonctions"),
    m!("354_services_interventions"),
    m!("355_missions"),
    m!("357_casquettes_invitations"),
    m!("361_agent_identifiants"),
    m!("362_agent_sujet"),
    m!("370_edt_creneau_groupe"),
    m!("375_msg_fil_prive"),
    m!("376_sepa_unicite_encaissement"),
    m!("377_edt_exception_absence_etab"),
    m!("460_eva_v0_correctifs"),
    m!("480_msg_m0a_fuites"),
    m!("481_msg_m0b_securite_fiabilite"),
    m!("500_objet_registre"),
    m!("520_finances_f0_immuabilite"),
    m!("560_delegation_droits_normalises"),
    m!("561_inscription_resolution"),
    m!("562_inscription_scolarite_actuelle"),
    m!("563_signalement_assiduite_detail"),
    m!("580_rls_eleves_responsables"),
    m!("581_rls_sante_assiduite"),
    m!("582_rls_finances"),
    m!("583_rls_communication"),
    m!("584_rls_vie_scolaire_discipline"),
    m!("585_rls_evaluations_pedagogie"),
    m!("586_rls_inscriptions"),
    m!("587_rls_structure_acces_divers"),
    m!("600_rls_stricte_adhesion_contexte"),
    m!("601_pieces_vers_objets"),
    m!("640_sortie_coeur"),
    m!("641_sortie_participants"),
    m!("642_sortie_encadrants"),
    m!("643_sortie_budget"),
    m!("661_d036_corrections_lot1"),
    m!("662_d036_profil_jours_ouvres"),
    m!("663_d036_parametre_type"),
    m!("664_d036_import_ligne_typee"),
    m!("665_d036_personne_plage_joignable"),
    m!("666_d036_eleve_fusion_normalisee"),
    m!("667_d036_personne_fusion_normalisee"),
    m!("668_d036_retrait_colonnes_lot1"),
    m!("669_d036_retrait_colonnes_lot2"),
    m!("670_d036_migrations_figees"),
];

/// Tag de la **dernière** migration du socle — exposé par `/health` pour qu'un
/// appelant (orchestrateur, PRA, support) sache quel schéma tourne réellement.
/// Le test `migration_tag_suit_la_derniere_migration` échoue si on ajoute un
/// fichier sans mettre à jour cette constante.
pub const MIGRATION_TAG: &str = "670_d036_migrations_figees";

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
    let figees = fichiers_figes(pool).await?;
    for (nom, sql) in MIGRATIONS {
        // Fichier figé : sa reprise a été faite et ses colonnes sources ont été retirées (D-036,
        // migrations 668–670). Le rejouer échouerait, ou ressusciterait une colonne retirée.
        if figees.iter().any(|f| f == nom) {
            tracing::info!(migration = %nom, "migration figée : sautée (D-036, ent_migration_figee)");
            continue;
        }
        apply_sql_file(pool, sql).await?;
    }
    Ok(())
}

/// Noms des fichiers de migration figés (liste vide tant que la migration 670 n'a pas tourné).
async fn fichiers_figes(pool: &PgPool) -> Result<Vec<String>, MigrateError> {
    let existe: bool = sqlx::query_scalar("SELECT to_regclass('public.ent_migration_figee') IS NOT NULL")
        .fetch_one(pool)
        .await?;
    if !existe {
        return Ok(Vec::new());
    }
    let noms: Vec<String> = sqlx::query_scalar("SELECT nom FROM ent_migration_figee").fetch_all(pool).await?;
    // On ne fige qu'un fichier qui existe : un nom inconnu (faute de frappe, fichier retiré) est une erreur.
    if let Some(inconnu) = noms.iter().find(|n| !MIGRATIONS.iter().any(|(m, _)| m == n)) {
        return Err(MigrateError::FigeInconnu(inconnu.clone()));
    }
    Ok(noms)
}

/// Tests : applique les migrations dans l'ordre **jusqu'à** `dernier` (inclus), sans figeage — reproduit
/// une base migrée jusqu'à une version donnée avant la mise à niveau.
#[cfg(test)]
pub(crate) async fn appliquer_jusqu_a(pool: &PgPool, dernier: &str) -> Result<(), MigrateError> {
    let mut trouve = false;
    for (nom, sql) in MIGRATIONS {
        apply_sql_file(pool, sql).await?;
        if *nom == dernier {
            trouve = true;
            break;
        }
    }
    assert!(trouve, "migration {dernier} inconnue");
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

    /// D-036 : colonnes JSON/JSONB déclarées. `(table, colonne, catégorie, justification)`.
    /// Catégories : `exception` (permise par D-036) et `dette` (à normaliser, lot suivant ; voir
    /// `docs/D036_INVENTAIRE_JSON.md`). Les 13 colonnes normalisées (lots 1 et 2) ont été retirées par
    /// les migrations 668 et 669 : le test retranche les `DROP COLUMN` du schéma net, elles ne figurent
    /// donc plus ici. Toute nouvelle colonne JSONB hors de cette liste fait échouer
    /// `aucune_colonne_json_hors_liste_d036`.
    const COLONNES_JSON_D036: &[(&str, &str, &str, &str)] = &[
        ("ent_metier_event", "payload", "exception", "charge utile d'événement (file d'envoi / journal figé)"),
        ("ent_edt_import", "report_json", "exception", "rapport d'import figé, jamais relu comme source"),
        ("ent_journal_modif", "payload", "exception", "journal d'audit en ajout seul"),
        ("ent_passkey_credential", "passkey", "exception", "état opaque webauthn-rs"),
        ("ent_passkey_challenge", "state", "exception", "état opaque webauthn-rs (transitoire)"),
                ("ent_cdt_historique", "instantane", "exception", "instantané d'historique du cahier de textes"),
        ("ent_bulletin", "contenu", "exception", "copie figée d'un bulletin publié"),
        ("ent_eleve_orphelin", "ligne", "exception", "ligne mise en quarantaine, instantané figé"),
        ("ent_inscription_campagne_version", "instantane", "exception", "instantané d'historique de campagne"),
        ("ent_structure_journal", "avant", "exception", "journal de structure en ajout seul (valeur avant)"),
        ("ent_structure_journal", "apres", "exception", "journal de structure en ajout seul (valeur après)"),
        ("ent_eleve_fusion", "resume", "exception", "registre de fusion en ajout seul (résumé figé)"),
        ("ent_inscription_campagne", "regle_relance", "dette", "laissée : refonte prévue au socle finances 24-25 (lot F0 en cours)"),
    ];

    /// Colonnes JSON/JSONB déclarées dans les migrations (`CREATE TABLE` / `ADD COLUMN`).
    fn colonnes_json_des_migrations() -> Vec<(String, String)> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/../../../migrations");
        let mut fichiers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("lecture {}: {e}", dir.display()))
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "sql"))
            .collect();
        fichiers.sort();
        let mut trouvees = Vec::new();
        // Colonnes retirées par `ALTER TABLE … DROP COLUMN` (migrations de retrait D-036) : le schéma net
        // ne les contient plus, elles ne doivent donc plus figurer dans la liste déclarée.
        let mut retirees: Vec<(String, String)> = Vec::new();
        for f in fichiers {
            let texte = std::fs::read_to_string(&f).unwrap();
            let mut table: Option<String> = None;
            let mut dans_corps = false;
            for ligne in texte.lines() {
                let l = ligne.split("--").next().unwrap_or("").trim_end().to_lowercase();
                if l.trim().is_empty() {
                    continue;
                }
                // Corps de fonction ou bloc DO (`$$ … $$`) : pas de déclaration de colonne.
                let n = l.matches("$$").count();
                if dans_corps {
                    if n % 2 == 1 {
                        dans_corps = false;
                    }
                    continue;
                }
                if n % 2 == 1 {
                    dans_corps = true;
                    continue;
                }
                let mots: Vec<&str> = l
                    .split(|c: char| c.is_whitespace() || c == '(' || c == ',')
                    .filter(|m| !m.is_empty())
                    .collect();
                if mots.len() >= 3 && (mots[0] == "create" || mots[0] == "alter") && mots[1] == "table" {
                    let mut i = 2;
                    if mots.get(i) == Some(&"if") {
                        i += 3; // if not exists
                    }
                    if mots.get(i) == Some(&"only") {
                        i += 1;
                    }
                    table = mots.get(i).map(|t| t.trim_matches('"').to_string());
                }
                if let Some(t) = &table {
                    if let Some(p) = mots.iter().position(|m| *m == "drop") {
                        if mots.get(p + 1) == Some(&"column") {
                            let mut i = p + 2;
                            if mots.get(i) == Some(&"if") {
                                i += 2; // if exists
                            }
                            if let Some(c) = mots.get(i) {
                                retirees.push((t.clone(), c.trim_end_matches(';').trim_matches('"').to_string()));
                            }
                            if l.trim_end().ends_with(';') {
                                table = None;
                            }
                            continue;
                        }
                    }
                    let type_json = |m: &str| matches!(m.trim_end_matches(';'), "json" | "jsonb");
                    // `col jsonb …` en tête de ligne, ou `add column [if not exists] col jsonb`.
                    let mut candidat: Option<&str> = None;
                    if mots.len() >= 2 && type_json(mots[1]) {
                        candidat = Some(mots[0]);
                    }
                    if let Some(p) = mots.iter().position(|m| *m == "column") {
                        let mut i = p + 1;
                        if mots.get(i) == Some(&"if") {
                            i += 3;
                        }
                        if mots.get(i + 1).is_some_and(|m| type_json(m)) {
                            candidat = Some(mots[i]);
                        }
                    }
                    if let Some(c) = candidat {
                        if !matches!(c, "alter" | "add" | "column" | "returns" | "return") {
                            trouvees.push((t.clone(), c.trim_matches('"').to_string()));
                        }
                    }
                }
                if l.trim_end().ends_with(';') {
                    table = None;
                }
            }
        }
        trouvees.retain(|c| !retirees.contains(c));
        trouvees.sort();
        trouvees.dedup();
        trouvees
    }

    /// D-036 : aucune colonne JSON/JSONB ne peut apparaître sans être déclarée (et justifiée)
    /// dans `COLONNES_JSON_D036` ; la liste ne garde pas non plus d'entrée disparue.
    #[test]
    fn aucune_colonne_json_hors_liste_d036() {
        let trouvees = colonnes_json_des_migrations();
        let declarees: Vec<(String, String)> = COLONNES_JSON_D036
            .iter()
            .map(|(t, c, _, _)| (t.to_string(), c.to_string()))
            .collect();
        let inconnues: Vec<_> = trouvees.iter().filter(|c| !declarees.contains(c)).collect();
        assert!(
            inconnues.is_empty(),
            "colonne(s) JSON/JSONB non déclarée(s) (D-036 : normaliser en tables typées, ou déclarer \
             l'exception dans COLONNES_JSON_D036 et docs/socles/MIGRATIONS.md) : {inconnues:?}"
        );
        let disparues: Vec<_> = declarees.iter().filter(|c| !trouvees.contains(c)).collect();
        assert!(disparues.is_empty(), "entrée(s) obsolète(s) dans COLONNES_JSON_D036 : {disparues:?}");
        for (t, c, cat, just) in COLONNES_JSON_D036 {
            assert!(["exception", "dette"].contains(cat), "{t}.{c} : catégorie {cat}");
            assert!(!just.is_empty(), "{t}.{c} : justification manquante");
        }
    }
}
