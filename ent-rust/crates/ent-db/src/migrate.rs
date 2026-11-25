use sqlx::PgPool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MigrateError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    /// Une migration déjà appliquée a changé : son empreinte ne correspond plus à celle du registre.
    #[error("migration « {nom} » modifiée depuis son application (empreinte en base {en_base}, dans le fichier {fichier}) : une migration appliquée ne se modifie plus, ajouter une nouvelle migration")]
    FichierModifie { nom: String, en_base: String, fichier: String },
}

/// Une migration : (nom du fichier sans extension, texte). Le nom est la clé du registre des migrations
/// appliquées (`ent_migration_appliquee`).
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
    m!("295_eleve_identite_jour_etablissement"),
    m!("296_eleve_jour_etablissement"),
    m!("297_eleve_horodatage_jour_etablissement"),
    m!("298_eleve_protection_photo_coordonnees"),
    m!("299_eleve_indicateurs"),
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
    m!("342_eleve_enseignement_jour_etablissement"),
    m!("343_eleve_enseignement_horodatage_jour_etablissement"),
    // 339 (socle 5, S6) est appliquée ICI, après 341 : elle repose la liste des objets du journal de structure.
    m!("339_structure_groupes"),
    m!("350_acces_etat"),
    m!("351_attributions_datees"),
    m!("352_agents"),
    m!("353_agents_fonctions"),
    m!("354_services_interventions"),
    m!("355_missions"),
    m!("356_groupes_responsables_dates"),
    m!("357_casquettes_invitations"),
    m!("358_equipes"),
    m!("359_delegations_signature"),
    m!("361_agent_identifiants"),
    m!("362_agent_sujet"),
    m!("370_edt_creneau_groupe"),
    m!("375_msg_fil_prive"),
    m!("376_sepa_unicite_encaissement"),
    m!("377_edt_exception_absence_etab"),
    m!("440_edt_t0_correctifs"),
    m!("441_edt_t1_versions"),
    m!("442_edt_t1_placements"),
    m!("443_edt_t2_seance_enrichie"),
    m!("444_edt_t2_seance_lieux_intervenants"),
    m!("445_edt_t3_modifications"),
    m!("447_edt_t4_localisation_journal"),
    m!("448_edt_t5a_materialisation"),
    m!("449_edt_t5_publication"),
    m!("460_eva_v0_correctifs"),
    m!("480_msg_m0a_fuites"),
    m!("481_msg_m0b_securite_fiabilite"),
    m!("482_msg_m1_participants"),
    m!("483_msg_m1_reprise"),
    m!("484_msg_m2_matrice"),
    m!("485_msg_m3_boites"),
    m!("486_msg_m3_traitement"),
    m!("487_msg_m4_ecrire_mieux"),
    m!("500_objet_registre"),
    m!("520_finances_f0_immuabilite"),
    // Socle 24, lot F1 (plage de repli 520–539) : journal financier en ajout seul.
    m!("521_finances_f1_journal"),
    // Lot F2 : comptes débiteurs, désignations datées, prises en charge, tâches, reprise MIG-4. Référence
    // `ent_personne` (120, 260), `ent_foyer` (007, 260), `ent_eleve` (482) : placée après eux, avant les lots RLS 58x.
    m!("522_finances_f2_comptes_payeurs"),
    m!("523_finances_f3_catalogue_grilles"),
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
    m!("452_edt_rls_permissive"),
    m!("453_edt_t3_rls_permissive"),
    m!("454_edt_t4_rls_permissive"),
    m!("455_edt_t5_rls_permissive"),
    m!("600_rls_stricte_adhesion_contexte"),
    m!("601_pieces_vers_objets"),
    m!("640_sortie_coeur"),
    m!("641_sortie_participants"),
    m!("642_sortie_encadrants"),
    m!("643_sortie_budget"),
    m!("644_sortie_prestataires"),
    m!("645_sortie_restauration"),
    m!("646_sortie_journal_voyage"),
    m!("647_sortie_reprise"),
    m!("648_sortie_exigences"),
    m!("649_sortie_transmission"),
    m!("650_sortie_argent_suivi"),
    m!("651_sortie_argent_bordereaux"),
    m!("652_sortie_contournement_devis_code"),
    m!("653_sortie_argent_aides_reports"),
    m!("661_d036_corrections_lot1"),
    m!("662_d036_profil_jours_ouvres"),
    m!("663_d036_parametre_type"),
    m!("664_d036_import_ligne_typee"),
    m!("665_d036_personne_plage_joignable"),
    m!("666_d036_eleve_fusion_normalisee"),
    m!("667_d036_personne_fusion_normalisee"),
    m!("668_d036_retrait_colonnes_lot1"),
    m!("669_d036_retrait_colonnes_lot2"),
    m!("2240_assiduite_as0"),
    m!("2241_assiduite_as1_referentiels"),
    m!("2242_assiduite_as2_appel_v2"),
    m!("2243_assiduite_as2_situations"),
    m!("2244_assiduite_as3_episodes"),
    m!("2245_assiduite_as3_justificatifs_demandes"),
    m!("2246_assiduite_as4_decompte"),
    m!("2300_absp_a0_correctifs"),
    m!("446_edt_t3_reprise"),
    m!("2301_absp_referentiel"),
    m!("2302_absp_absence"),
    m!("2303_absp_pieces_fil"),
    m!("2304_absp_impact"),
    m!("2305_absp_sollicitation"),
    m!("2306_absp_suppleance"),
    m!("2307_absp_rcd"),
    m!("2308_absp_heures"),
    m!("2309_absp_rattrapage"),
    m!("2310_absp_paie_dsn"),
    m!("2312_absp_personas_circuit_arrets"),
    m!("2315_absp_parametres"),
    // Socle 18 (formulaires, autorisations, consentements), plage 2360–2379 : lots FO0 (2360) et FO1.
    m!("2360_formulaires_fo0_correctifs"),
    m!("2361_acc_objets_catalogue_usages"),
    m!("2362_acc_versions_decisions"),
    m!("2363_acc_journal"),
    // Lots FO2 et FO3 : formulaires (2364), cibles et soumissions (2365), désaccords et objets par source (2366).
    m!("2364_frm_formulaires"),
    m!("2365_frm_cibles_soumissions"),
    m!("2366_acc_desaccords_sources"),
    m!("2380_agenda_ag0_rencontres"),
    m!("2381_agenda_evenement"),
    m!("2386_agenda_reservation_lieu"),
    m!("2400_cdt_c0_correctifs"),
    m!("2401_cdt_reglages_natures"),
    m!("2402_cdt_entrees"),
    m!("2403_cdt_travaux"),
    m!("2404_cdt_pieces_liens"),
    m!("2405_cdt_remises"),
    m!("2406_cdt_retours"),
    m!("2409_cdt_reports_continuites"),
    m!("2410_cdt_visas_charge"),
    m!("2411_cdt_archives"),
    m!("2412_cdt_reprise"),
    m!("2413_cdt_c2_saisie_publication"),
    m!("2414_cdt_c3_agenda_cahier_famille"),
    m!("2415_cdt_c4_charge"),
    m!("2416_cdt_c5_rendu_retour"),
    m!("2417_cdt_c6_continuite"),
    m!("2418_cdt_c7_adaptations"),
    // Socle 20, C8 et C9 : premiers numéros libres de la plage 2400–2419 (2407 et 2408, jamais utilisés) ; placés après 2418
    // car elles s'appuient sur 2410 (visa) et 2411 (archive). Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("2407_cdt_c8_visa_inspection_archive"),
    m!("2408_cdt_c9_progressions"),
    // Socle 27, G0 : TRUNCATE interdit sur le journal en ajout seul (RG-D02).
    m!("2540_rgpd_g0_journal_truncate"),
    // Socle 27, G1 : règles de conservation par établissement, DPO (peut être absent), journal en ajout seul.
    m!("2541_rgpd_g1_conservation"),
    // Socle 27, G2 : registre généré, écarts, mentions, documents ; sous-traitants.
    m!("2542_rgpd_g2_registre"),
    m!("2543_rgpd_g2_soustraitants"),
    // Socle 28, lot MS0 : annulation réelle des lots d'import prévisualisés.
    m!("2560_ms0_import_annulation"),
    // Socle 28, lot R2 (MS4 c) : exécutions de reprise, correspondance des identifiants, zone de transit.
    m!("2564_ms4_reprise_executions"),
    m!("2565_ms4_reprise_correspondance"),
    m!("2566_ms4_zone_reprise"),
    // Socle 28, lot R3 (MS4 d) : rapprochement, file « à décider », décisions tracées, synonymes validés.
    m!("2567_ms4_reprise_rapprochement"),
    // Socle 29 (portails, à-faire unifié), plage 2580–2599 : lot P2. 2587 (actions D-017) et 2588
    // (reprise initiale) sont libérées : actions dans le code, reprise par le rapprochement.
    m!("2580_afaire_index"),
    m!("2581_afaire_journal_prises"),
    m!("2582_afaire_reglages"),
    // Lot P5 : compteur de recherches (volatile, aucun texte de requête).
    m!("2585_recherche_compteur"),
    m!("2780_stage_st0_correctifs"),
    m!("2781_stage_st1a_parametres_organismes"),
    m!("2782_stage_st1a_periodes"),
    m!("2820_internat_i0_presence_faits"),
    m!("2821_internat_i0_regime_parametres"),
    m!("2822_internat_i1_zones_chambres_lits"),
    m!("2823_internat_i1_sejours_affectations"),
    // Socle 41, lot I2 (la nuit) : attendu calculé, appels, cas et procédures, transmission, rappels,
    // journal des lectures. Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("2824_internat_i2_la_nuit"),
    // Suites du lot I2 (contre-revue de sécurité) : date de sortie, sorties scolaires, note de clôture d'un cas.
    m!("2825_internat_i2_suites"),
    // Socle 41, lot I3 (absences et sorties) : absences annoncées, sorties typées, correspondants, appel
    // « soir » complémentaire, attendu qui lit les absences et les sorties. Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("2826_internat_i3_absences_sorties"),
    m!("2827_internat_i3_suites"),
    // Socle 42, lot V3 : lien séjour ↔ internat ; référence `ent_internat_retrait` (2824), d'où l'ordre APRÈS le socle 41 (I3).
    // Elle redéfinit `ent_internat_attendu_calcule` : la version de 2829 (I4) en reprend le texte et y ajoute sa branche, d'où
    // l'ordre 654 AVANT 2828-2829 (sans quoi 654 écraserait la branche des sorties de groupe).
    m!("654_sortie_internat_lien"),
    // Socle 41, lot I4 (vie quotidienne) : sorties de groupe (2828) ; liaison codée, événements de nuit, temps du soir,
    // préoccupation (2829, qui reprend l'attendu de 654). Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("2828_internat_i4_sorties_groupe"),
    m!("2829_internat_i4_vie_quotidienne"),
    // Socle N1 (demandes et flux de travail), plage 3040–3059 : lot N1-0 (cœur, projections).
    m!("3040_demandes_n1_0_coeur"),
    m!("3041_demandes_n1_0_droits"),
    // Lot N1-1 : files, membres calculés, routage, repli RH.
    m!("3042_demandes_n1_1_files"),
    m!("3043_demandes_n1_1_routage"),
    // Lot N1-2 : échanges, pièces (références au socle 7), abonnés ; consultations, liens, personnes visées.
    m!("3044_demandes_n1_2_echanges"),
    m!("3045_demandes_n1_2_consultations_liens"),
    // Lot N1-3 : HSE (en heures, validées par la direction, D-061) et reprographie (file par site).
    m!("3046_demandes_n1_3_hse"),
    m!("3047_demandes_n1_3_reprographie"),
    // Lot N1-5 : reprise des données du Next.js (R9), puis gardes de la bascule (guichet, rapprochement, HSE hors ScolIA).
    m!("3048_demandes_n1_5_reprise"),
    m!("3049_demandes_n1_5_bascule"),
    m!("3700_documents_identite_natures"),
    m!("3701_documents_documents_versions"),
    m!("3702_documents_compteurs"),
    m!("3703_documents_mises_a_disposition"),
    m!("3704_documents_pieces"),
    m!("3705_documents_signatures"),
    // Socle 8 (notifications), plage 3800–3899 : lot N1 (brique et contrat).
    m!("3800_ntf_parametres"),
    m!("3801_ntf_avis"),
    m!("3802_ntf_remises"),
    // Lot N3 (préférences et temps).
    m!("3803_ntf_preferences"),
    // Lot N2 (destinataires = personnes).
    m!("3808_ntf_gels_liens_a_prevenir"),
    // Socle 12, lot AS5 (plage 2247–2255) : ces migrations référencent `ent_ntf_avis` (socle 8, 3801) ; elles sont
    // donc appliquées APRÈS les migrations 38xx (l'ordre d'application est celui de ce tableau, pas le numéro).
    // Leur numéro reste sous 3808 : `MIGRATION_TAG` (plus haut numéro du répertoire) est inchangé.
    m!("2247_assiduite_as5_relance_contact"),
    m!("2248_assiduite_as5_avis_demi_journee"),
    m!("2249_assiduite_as5_courrier"),
    // Socle 12, lot AS6 (plage 2250–2255) : 2250 à 2252 ne dépendent pas du socle 8 ; 2253 référence `ent_ntf_avis` et
    // modifie le registre d'AS5 (2248). Tous restent sous 3808 : `MIGRATION_TAG` est inchangé (règle 28).
    m!("2250_assiduite_as6_alerte"),
    m!("2251_assiduite_as6_relance_appel"),
    m!("2252_assiduite_as6_substitution"),
    m!("2253_assiduite_as6_prevenir_maintenant"),
    // Socle 12, lot AS14b-bis (2254, 2255) : réglage de l'heure de bascule et disposition du tableau de bord ; sous 3808.
    m!("2254_assiduite_as14b_bis_reglage_bascule"),
    m!("2255_assiduite_as14b_bis_preference"),
    // Socle 3, lots I3 et I4 (plage 310–329) : règles de pièces, fichier des pièces (sujet « dossier » du socle 7,
    // d'où l'ordre APRÈS 3704), accords des responsables. Sous 3808 : `MIGRATION_TAG` est inchangé.
    m!("310_inscription_piece_regle"),
    m!("311_inscription_piece_fichier"),
    m!("312_inscription_accord"),
    // Socle 3, lot I5 (plage 313–329) : décisions en ajout seul, avis, commissions, liste d'attente, propositions, conditions.
    m!("313_inscription_decision_i5"),
    // Passe 3 du lot I5 (314) : décision caduque, séquence de l'ordre d'attente, réparation de la chaîne, gardes à
    // `search_path` fixé. 313 est versée et ne change pas (règle 25). Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("314_inscription_decision_i5_passe3"),
    // Socle 32 (espaces de travail), lots ET0 et ET1 (plage 2640–2643) : ces migrations référencent `ent_personne`
    // (120), `ent_objet` (500, index unique de 2402) et `ent_rls_poser` (580) ; elles sont appliquées après eux.
    // Leur numéro reste sous 3808 : `MIGRATION_TAG` (plus haut numéro du répertoire) est inchangé.
    m!("2640_esp_et0_quotas"),
    m!("2641_esp_espaces_membres"),
    m!("2642_esp_dossiers_elements_versions"),
    m!("2643_esp_partages_journal"),
    // Socle 18, lot FO3 (2367) : dossier de preuve par la brique de signature (tables du socle 7, 3705) ; appliquée après
    // elles, numéro sous 3808 donc `MIGRATION_TAG` inchangé (règle 28).
    m!("2367_acc_dossier_preuve"),
    // Socle 2, lot E9 (plage 3200–3219) : pastilles R-E9, statut jamais NULL (F3), contraintes validées (F5). Appliquée
    // en dernier (elle remplace des fonctions de 299 et lit des tables de plusieurs socles) ; sous 3808, donc
    // `MIGRATION_TAG` inchangé (règle 28).
    m!("3200_eleve_e9_pastilles_statut_contraintes"),
    // Socle 11, lot MB0 (plage 2220–2239) : appareils push liés à la session, révocation en cascade (DS1, DS3) ; sous 3808.
    m!("2220_mob_mb0_appareils_push"),
    // Messagerie, lots M5 et M6 (plage 488–493 du repli 480–499) : 488 référence `ent_doc_document` (socle 7, 3701) et
    // modifie `ent_msg_piece` (038, 601) ; appliquées donc APRÈS 3701. Sous 3808 : `MIGRATION_TAG` est inchangé (règle 28).
    m!("488_msg_m5_pieces"),
    m!("489_msg_m6_lecture"),
    // Correctifs de la revue de sécurité de M5-M6 (490 : annulation d'un message différé, journal d'ouverture des liens).
    m!("490_msg_correctifs_revue"),
    // Correctifs de la revue des suites M12 (491) : annulation d'un différé corrigé (versions), curseur du flux en `xid8`.
    // Sous 3808 : `MIGRATION_TAG` est inchangé (règle 28).
    m!("491_msg_annulation_versions_flux_xid"),
    // Passe 2 de la contre-revue E1 (492) : garde d'annulation à jeton de transaction et `search_path` fixé. Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("492_msg_garde_annulation_jeton"),
    // Socle 2 (3201) : un seul jour de l'établissement (réglage officiel du fuseau d'abord) ; sous 3808.
    m!("3201_jour_etablissement_unique"),
    // Socle 21 (évaluations), lot V1 « modèle et reprise » (plage 461–479 ; 460 = V0) : règles de calcul, évaluations,
    // résultats (historique en ajout seul, aucune suppression), états de période (machine d'états), appréciations,
    // demandes, enseignements externes, reprise M2 et vues de compatibilité. Elles référencent `ent_cours` (341),
    // `ent_periodisation_pour_classe` (335), `ent_jour_etablissement` (3201) et `ent_rls_poser` (580) : appliquées en
    // dernier. Sous 3808 : `MIGRATION_TAG` est inchangé (règle 28). 469 exécute la reprise (une fois par établissement).
    m!("461_eva_v1_reglages_coefficients"),
    m!("462_eva_v1_evaluations"),
    m!("463_eva_v1_periodes"),
    m!("464_eva_v1_resultats"),
    m!("465_eva_v1_appreciations"),
    m!("466_eva_v1_demandes_externes"),
    m!("467_eva_v1_reprise"),
    m!("468_eva_v1_vues_compat"),
    m!("469_eva_v1_reprise_executee"),
    // Correctifs de la revue du lot V1 (F9) : les cours « de reprise » ne figurent plus dans les publics.
    m!("470_eva_v1_cours_reprise_hors_publics"),
    // Lot V3 « grille et saisie » : correctifs F7/F8 de la reprise, retrait de publication, journaux.
    m!("471_eva_v3_grille_saisie"),
    // Lot V3b « les lecteurs passent sur eva_* » (M2-M3 du plan) : `ent_devoir` et `ent_note` deviennent des vues en lecture
    // seule alimentées par `eva_*` (anciennes tables renommées en archives), reprise corrigée (M2), statut de période gelé.
    m!("472_eva_v3b_lecteurs_sur_eva"),
    // Correctifs de la revue de sécurité de V3 qui touchent la base : attestation du droit de correction (F2), première
    // publication d'une évaluation (F4).
    m!("473_eva_v3b_revue_securite"),
    // Lot V3c : revue de sécurité de V3b (auteur obligatoire sur un résultat, vue `ent_note` sans 0 de substitution).
    m!("474_eva_v3c_revue_securite"),
    // Lot V4 « absences, dispenses, aménagements, rattrapages » : ports 12 (lecture), 34 et AESH (simulation), 35 (lecture),
    // convocations de rattrapage et de remplacement, marque « aménagé » posée par la base.
    m!("475_eva_v4_absences_dispenses_rattrapages"),
    // Lot V3d : revue de sécurité de V4 (F-1) : TRUNCATE de `eva_dispense_port`, table AESH en ajout seul.
    m!("476_eva_v3d_revue_securite_v4"),
    // Lots V3e + V5 « parcours » : motif conservé, fonction unique de période, prolongation d'échéance tracée, résultats
    // externes (décision de compter, publication, retrait) et moyennes externes gardés en base.
    m!("477_eva_v3e_v5_parcours"),
    // Contre-revue de V5 : une correction d'une moyenne externe publiée la remet en « non publiée », retrait de publication
    // tracé, seule une moyenne se publie (en base).
    m!("478_eva_v5_contre_revue_publication"),
    // Chantier transverse T1 (plage 690–699, sécurité transverse) : `search_path` fixé sur toutes les fonctions de `public`
    // (une table temporaire homonyme ne trompe plus aucune garde). **Règle des lots suivants** : toute migration nouvelle
    // s'ajoute APRÈS 690 (jamais avant : une base déjà migrée ne rejouerait pas 690 après elle), et chacun de ses
    // `CREATE OR REPLACE FUNCTION` porte `SET search_path = pg_catalog, public, pg_temp` (la garde `garde_search_path`
    // l'exige sur base neuve et après `rejouer_tout`). Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("690_t1_search_path_fonctions"),
    // Suites d'I4 (passe 3) : contraintes nommées (motif du participant, acte des faits) ; 2828-2829 sont versées.
    // Règle 38 : APRÈS 690 (aucune fonction créée ici). `MIGRATION_TAG` inchangé (3808).
    m!("2830_internat_i4_suites"),
    // Socle 21, suites V5 passe 3 (plage d'extension 700–719) : compteur `version` des moyennes externes (jeton de
    // compare-et-écrit de la publication et du retrait). APRÈS 690 (règle 38) ; sa fonction porte son `search_path`.
    m!("700_eva_v5_suites_jeton_version"),
    // Passe 4 du lot I5 (315) : rang d'attente calculé en lecture (`rang_attente` en lecture seule), clé du verrou
    // consultatif par offre, origine d'un vœu. 313 et 314 sont versées et ne changent pas (règle 25). Règle 38 : en fin
    // de tableau, après toutes les migrations versées ; ses fonctions portent leur `search_path`. Sous 3808 :
    // `MIGRATION_TAG` inchangé.
    m!("315_inscription_decision_i5_passe4"),
    // Suites E1 (493, plage 482–499 du socle 16, **après 690**, règle 38) : un fil dont le seul message est annulé n'est plus
    // lisible de la famille (F-6) ; `ent_msg_annulation_ouvrir` reçoit l'auteur, devient `SECURITY DEFINER`, et la table de
    // garde n'est plus inscriptible que par le propriétaire. Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("493_msg_fil_vide_et_garde_proprietaire"),
    // Suites I5 (316) : sentinelle en base de l'ordre des verrous (déclencheur qui exige le verrou consultatif de l'offre),
    // source unique du rang (liste ordonnée d'une offre). Règle 38 : en fin de tableau ; chaque fonction porte son
    // `search_path`. 313 à 315 sont versées et ne changent pas. Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("316_inscription_i5_suites_sentinelle"),
    // Socle 42, V3 suites (plage 640–659) : instant d'annulation d'une sortie, retrait « séjour » à source unique (M-1),
    // règle « jamais pour son propre enfant » étendue à tout pouvoir sur l'argent (M-3), cohérences de base (F-d).
    // 653 et 654 sont versées et ne changent pas (règle 25). Règle 38 : en fin de tableau ; ses fonctions portent leur
    // `search_path` (la fonction d'attendu, SQL inlinée, a ses relations qualifiées `public.`). `MIGRATION_TAG` inchangé.
    m!("655_sortie_voyages_v3_suites"),
    // Socle 24, suites du lot F3 (contre-revue, D-069 ; réserve 531–537 de la plage 520–539) : égalité de date de
    // vigueur refusée, contributeurs du second regard, publication d'une version close, année d'un tarif versionné
    // figée, historique des calculs supprimé seulement avec sa simulation. 520 à 523 sont versées et ne changent pas.
    // Règle 38 : en fin de tableau ; ses fonctions portent leur `search_path`. `MIGRATION_TAG` inchangé.
    m!("531_finances_f3_suites"),
    // Suites E1 (passe 4, 494, plage 482–499 du socle 16, **fin de tableau**, règle 38) : `ent_msg_annulation_ouvrir` n'est plus
    // exécutable par `PUBLIC` (M1) ; aucune fonction recréée. Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("494_msg_garde_jeton_droit_execution"),
    // Socle 20, suites C8/C9 : dernier numéro libre de la plage 2400–2419. `REVOKE EXECUTE` sur `ent_cdt_reprise`, gel d'année
    // qui échoue fermé (F3), tables filles du cahier classées au gel (F2). Règle 38 : en fin de tableau ; ses fonctions
    // portent leur `search_path`. Sous 3808 : `MIGRATION_TAG` inchangé.
    m!("2419_cdt_suites_gel_filles"),
    // Suites I5 (passe 6, 317, **fin de tableau**, règle 38) : sentinelle de 316 par instruction (une vérification par offre
    // distincte) et `ENABLE ALWAYS TRIGGER` ; liste d'attente sans appel de fonction par dossier de l'établissement. 313 à 316
    // sont versées et ne changent pas (règle 25) ; ses fonctions portent leur `search_path`. `MIGRATION_TAG` inchangé.
    m!("317_inscription_i5_suites_passe6"),
    // Socle 42, V3b suites (656, plage 640–659) : instant de passage en « rentrée » (`rentree_le`), contournement tracé de
    // la règle « sortie entière » par une seconde personne libre, auteur de l'annulation d'un acompte, attendu de l'internat
    // sans branche directe redondante. 640–655 sont versées et ne changent pas (règle 25). Règle 38 : en fin de tableau ;
    // ses fonctions portent leur `search_path` (l'attendu, SQL inlinée, a ses relations qualifiées `public.`).
    // `MIGRATION_TAG` inchangé.
    m!("656_sortie_voyages_v3b_suites"),
    // Suites I5 (passe 7, 318, **fin de tableau**, règle 38) : la sentinelle par instruction lit l'offre de la liste d'un dossier
    // par jointure (plus d'appel de fonction par ligne). 313 à 317 sont versées et ne changent pas (règle 25) ; sa fonction
    // porte son `search_path`. `MIGRATION_TAG` inchangé.
    m!("318_inscription_i5_suites_passe7"),
    // Socle 24, suites 3 du lot F3 (D-069 ; réserve 532–537 de la plage 520–539) : nature de tâche « régularisation de
    // changement de régime non facturée » (liée à l'événement : `evenement_id`, index d'unicité recréé) et fin de tarif
    // datée d'un article. 520 à 523 et 531 sont versées et ne
    // changent pas (règle 25). Règle 38 : en fin de tableau ; aucune fonction. `MIGRATION_TAG` inchangé.
    m!("532_finances_f3_suites3"),
];

/// Tag de la **dernière** migration du socle — exposé par `/health` pour qu'un
/// appelant (orchestrateur, PRA, support) sache quel schéma tourne réellement.
/// Le test `migration_tag_suit_la_derniere_migration` échoue si on ajoute un
/// fichier sans mettre à jour cette constante.
pub const MIGRATION_TAG: &str = "3808_ntf_gels_liens_a_prevenir";

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
    if result.is_ok() {
        // Un signal visible, pas un simple avertissement de migration : une contrainte que la reprise n'a pas pu
        // valider reste NOT VALID tant que les données ne sont pas corrigées.
        let _ = contraintes_non_validees(pool).await;
        // Idem pour la sentinelle des verrous d'offre (I5) : une restauration `pg_restore --disable-triggers` la remet à
        // l'état « origine » et lui fait perdre `ENABLE ALWAYS`.
        let _ = sentinelles_non_toujours_actives(pool).await;
    }

    let unlock = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(ENT_MIGRATE_LOCK)
        .execute(&mut *lock_conn)
        .await;
    drop(lock_conn);
    result?;
    unlock?;
    Ok(())
}

/// Contraintes dont la validation est attendue (posées NOT VALID par une migration, validées après reprise).
pub const CONTRAINTES_A_VALIDER: [&str; 3] = [
    "ent_eleve_confidentialite_regles_chk",
    "ent_eleve_statut_chk",
    // 478 : posée NOT VALID, validée dans la même migration après dépublication des lignes publiées hors « moyenne ».
    "eva_moyenne_externe_publication_statut_chk",
];

/// Noms des contraintes de `CONTRAINTES_A_VALIDER` qui sont encore NOT VALID ; chacune est **journalisée en
/// erreur** (au démarrage) avec son nom : les écritures futures sont contrôlées, mais des lignes existantes
/// restent hors règle et demandent une correction.
pub async fn contraintes_non_validees(pool: &PgPool) -> Result<Vec<String>, MigrateError> {
    let noms: Vec<(String,)> = sqlx::query_as(
        "SELECT conname::text FROM pg_constraint WHERE NOT convalidated AND conname = ANY($1) ORDER BY conname",
    )
    .bind(&CONTRAINTES_A_VALIDER[..])
    .fetch_all(pool)
    .await?;
    let noms: Vec<String> = noms.into_iter().map(|(n,)| n).collect();
    for n in &noms {
        tracing::error!(contrainte = %n, "contrainte NOT VALID : des lignes existantes sont hors règle, à corriger puis à valider");
    }
    Ok(noms)
}

/// Déclencheurs de la sentinelle des verrous d'offre (317, fonctions de 318) attendus en `ENABLE ALWAYS`.
pub const SENTINELLES_ATTENDUES: [&str; 7] = [
    "ent_inscription_sentinelle_decision_del_trg",
    "ent_inscription_sentinelle_decision_ins_trg",
    "ent_inscription_sentinelle_decision_upd_trg",
    "ent_inscription_sentinelle_offre_upd_trg",
    "ent_inscription_sentinelle_proposition_del_trg",
    "ent_inscription_sentinelle_proposition_ins_trg",
    "ent_inscription_sentinelle_proposition_upd_trg",
];

/// Vérification de santé au démarrage (I5, passe 7, moyen A) : déclencheurs `ent_inscription_sentinelle_*` dont l'état n'est pas
/// `ENABLE ALWAYS` (`tgenabled <> 'A'`) : suspendus (`DISABLE`) ou remis à « origine » par une restauration
/// (`pg_restore --disable-triggers` finit par `ENABLE TRIGGER ALL`). Chacun est **journalisé en erreur**, sans bloquer le démarrage
/// (la sentinelle est un filet, pas un contrôle de sécurité). Procédure : `docs/socles/03_INSCRIPTIONS_I5_LIVRAISON.md`.
pub async fn sentinelles_non_toujours_actives<'e, E: sqlx::PgExecutor<'e>>(ex: E) -> Result<Vec<String>, MigrateError> {
    // Contre-revue du pilote (passe 7) : la liste ATTENDUE est explicite, un déclencheur absent (supprimé, omis par une
    // restauration) est signalé (`absent`) comme un déclencheur non ALWAYS.
    let noms: Vec<(String, String)> = sqlx::query_as(
        "SELECT a.nom, COALESCE(t.tgenabled::text, 'absent')
           FROM unnest($1::text[]) AS a(nom)
           LEFT JOIN pg_catalog.pg_trigger t ON t.tgname = a.nom AND NOT t.tgisinternal
          WHERE t.tgenabled IS DISTINCT FROM 'A'
          ORDER BY 1",
    )
    .bind(&SENTINELLES_ATTENDUES[..])
    .fetch_all(ex)
    .await?;
    for (n, etat) in &noms {
        tracing::error!(
            declencheur = %n, etat = %etat,
            "sentinelle des verrous d'offre absente ou non ENABLE ALWAYS : la recréer ou rétablir `ALTER TABLE … ENABLE ALWAYS TRIGGER …` (voir la procédure de restauration)"
        );
    }
    Ok(noms.into_iter().map(|(n, _)| n).collect())
}

/// Registre des migrations appliquées : `(nom, empreinte SHA-256 du fichier, date)`. Créé par le lanceur
/// lui-même (il doit exister avant la première migration) ; classé « Technique » au registre des tables.
const CREER_REGISTRE: &str = "CREATE TABLE IF NOT EXISTS ent_migration_appliquee (
    nom TEXT PRIMARY KEY CHECK (nom ~ '^[0-9]{3,4}_[a-z0-9_]+$' AND length(nom) <= 120),
    empreinte TEXT NOT NULL CHECK (empreinte ~ '^[0-9a-f]{64}$'),
    appliquee_le TIMESTAMPTZ NOT NULL DEFAULT now())";

/// Empreinte SHA-256 (hexadécimal) du texte d'un fichier de migration.
pub(crate) fn empreinte(sql: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(sql.as_bytes()).iter().map(|o| format!("{o:02x}")).collect()
}

/// Applique, dans l'ordre, les migrations **pas encore** enregistrées : une transaction par fichier, un
/// fichier appliqué une seule fois. Avant toute écriture, l'empreinte de chaque fichier déjà appliqué est
/// vérifiée : un fichier modifié est refusé (`MigrateError::FichierModifie`). Une base sans registre
/// (neuve, ou antérieure au registre) applique tout dans l'ordre ; les fichiers sont idempotents.
async fn apply_all(pool: &PgPool) -> Result<(), MigrateError> {
    sqlx::query(CREER_REGISTRE).execute(pool).await?;
    let appliquees: Vec<(String, String)> =
        sqlx::query_as("SELECT nom, empreinte FROM ent_migration_appliquee").fetch_all(pool).await?;
    for (nom, sql) in MIGRATIONS {
        if let Some((_, en_base)) = appliquees.iter().find(|(n, _)| n == nom) {
            let fichier = empreinte(sql);
            if *en_base != fichier {
                return Err(MigrateError::FichierModifie { nom: (*nom).to_string(), en_base: en_base.clone(), fichier });
            }
        }
    }
    for (nom, sql) in MIGRATIONS {
        if appliquees.iter().any(|(n, _)| n == nom) {
            continue;
        }
        appliquer_dans_une_transaction(pool, nom, sql).await?;
        tracing::info!(migration = %nom, "migration appliquée");
    }
    Ok(())
}

/// Tous les ordres d'un fichier + l'inscription au registre, dans **une** transaction : un échec n'en
/// laisse aucune trace (ni demi-fichier, ni inscription).
pub(crate) async fn appliquer_dans_une_transaction(pool: &PgPool, nom: &str, sql: &str) -> Result<(), MigrateError> {
    let mut tx = pool.begin().await?;
    for stmt in ordres(sql) {
        sqlx::query(&stmt).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO ent_migration_appliquee (nom, empreinte) VALUES ($1, $2)")
        .bind(nom)
        .bind(empreinte(sql))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Tests : applique les migrations dans l'ordre **jusqu'à** `dernier` (inclus), **sans** registre ni
/// transaction — reproduit une base antérieure au registre, migrée jusqu'à une version donnée.
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
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

/// Gardes de rejeu (test seulement) : ordres exécutés **juste avant** le rejeu d'un fichier, dans `rejouer_tout`.
///
/// Un rejeu rejoue 644 après 648. Or 644 recrée `ent_sortie_document_type_v0c_chk` (6 types) quand elle est
/// absente, et 648 l'a supprimée en posant `ent_sortie_document_type_v1_chk` : sur une base qui contient déjà un
/// document de type V1 (`ordre_mission`, `autorisation_adulte`, `liste_nominative`), l'ADD de 644 échouait. La garde
/// pose une contrainte factice du même nom, la garde de 644 saute alors son bloc, et 648 la supprime ensuite.
/// Les migrations ne sont pas modifiées (règle 25) ; la production n'exécute jamais ce tableau.
///
/// **Tout futur couple « migration qui recrée une contrainte qu'une migration plus récente remplace » doit être
/// ajouté ici** (fichier concerné, ordre SQL idempotent, cause).
/// Retire les déclencheurs de la sentinelle des verrous d'offre sur la table des décisions (gardes de 304, 313 et 314).
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
const RETIRER_SENTINELLE_DECISIONS: &str = "DO $$
    DECLARE
        t TEXT;
    BEGIN
        IF to_regclass('ent_inscription_decision') IS NOT NULL THEN
            FOR t IN SELECT tgname FROM pg_trigger
                      WHERE tgrelid = 'ent_inscription_decision'::regclass AND NOT tgisinternal
                        AND tgname LIKE 'ent\\_inscription\\_sentinelle\\_%'
            LOOP
                EXECUTE format('DROP TRIGGER %I ON ent_inscription_decision', t);
            END LOOP;
        END IF;
    END $$";

#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
const GARDES_DE_REJEU: &[(&str, &str)] = &[
    // 304 rejoue sa reprise des décisions (`INSERT INTO ent_inscription_decision … SELECT` pour chaque dossier admis sans
    // décision) ; au rejeu, la sentinelle de 316/317 (verrou de l'offre exigé pour toute écriture de décision) existe déjà et
    // refuserait cet INSERT de reprise, qui ne tient aucun verrou. Comme pour une restauration de données (procédure
    // documentée : suspendre explicitement les déclencheurs de la sentinelle le temps de la reprise), on retire ici les
    // déclencheurs de la sentinelle sur la table des décisions ; 316 puis 317, rejouées plus loin dans le même rejeu, les
    // recréent (316 par ligne, 317 par instruction, `ENABLE ALWAYS`). Une base qui contient un dossier admis sans décision
    // rejoue ainsi 304 sans erreur.
    (
        "304_inscription_demarche_dossier",
        RETIRER_SENTINELLE_DECISIONS,
    ),
    // 313 et 314 rejouent aussi des écritures sur les décisions (reprise de `publiee_le`, chaînage `remplace_decision_id` sur
    // un historique non chaîné) sans verrou d'offre : même garde (passe 7, faible 1). 316, 317 et 318 recréent la sentinelle.
    ("313_inscription_decision_i5", RETIRER_SENTINELLE_DECISIONS),
    ("314_inscription_decision_i5_passe3", RETIRER_SENTINELLE_DECISIONS),
    (
        "644_sortie_prestataires",
        "DO $$ BEGIN
        IF to_regclass('ent_sortie_document') IS NOT NULL
           AND NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ent_sortie_document_type_v0c_chk') THEN
            ALTER TABLE ent_sortie_document ADD CONSTRAINT ent_sortie_document_type_v0c_chk CHECK (true);
        END IF;
    END $$",
    ),
    // 2824 recrée l'index unique d'appel sans complément que 2826 remplace (un second appel « soir » complémentaire,
    // M-1, est permis) : sur une base qui contient un tel appel, le rejeu de 2824 échouerait (doublon). Un index
    // provisoire de même nom, non unique, fait ignorer la création (`IF NOT EXISTS`) ; 2826 le supprime de nouveau.
    (
        "2824_internat_i2_la_nuit",
        "DO $$ BEGIN
        IF to_regclass('ent_internat_appel') IS NOT NULL
           AND to_regclass('ent_internat_appel_perimetre_uq') IS NULL THEN
            CREATE INDEX ent_internat_appel_perimetre_uq ON ent_internat_appel (etablissement_id, nuit);
        END IF;
    END $$",
    ),
    // 2412 rejoue la reprise de l'ancien cahier (`SELECT … FROM ent_cdt_reprise(NULL)`) ; en installation elle passe
    // avant 2407, mais au rejeu le gel des années closes (2407) existe déjà et refuserait, à raison, ses INSERT dans une
    // année figée : le gel n'a AUCUNE exception (une exception par pile d'appel ou par drapeau est contournable). On
    // retire donc ses déclencheurs juste avant 2412 ; 2407, rejouée plus loin dans le même rejeu, les recrée.
    (
        "2412_cdt_reprise",
        "DO $$
    DECLARE
        t TEXT;
    BEGIN
        FOREACH t IN ARRAY ARRAY['ent_cdt_entree', 'ent_cdt_entree_version', 'ent_cdt_entree_notion', 'ent_cdt_travail',
                                 'ent_cdt_travail_version', 'ent_cdt_travail_cible', 'ent_cdt_travail_format', 'ent_cdt_variante',
                                 'ent_cdt_variante_eleve', 'ent_cdt_contribution', 'ent_cdt_contribution_version', 'ent_cdt_lien',
                                 'ent_cdt_piece_jointe', 'ent_cdt_report']
        LOOP
            IF to_regclass(t) IS NOT NULL THEN
                EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', t || '_annee_figee', t);
            END IF;
        END LOOP;
    END $$",
    ),
];

/// Tests : rejoue **tous** les fichiers (sans passer par le registre) — simule une reprise d'anciennes
/// données (« legacy » insérées après coup) que les fichiers de reprise savent absorber.
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
pub(crate) async fn rejouer_tout(pool: &PgPool) -> Result<(), MigrateError> {
    rejouer_liste(pool, MIGRATIONS).await
}

/// Tests : `rejouer_tout` sur une liste donnée (le test d'échec de rejeu y ajoute un fichier qui échoue).
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
pub(crate) async fn rejouer_liste(pool: &PgPool, liste: &[(&str, &str)]) -> Result<(), MigrateError> {
    // Ces fichiers (reprises du lot D-036) lisent des colonnes retirées par 668 et 669 : on ne les rejoue
    // pas (il n'y a plus de ligne « héritée » à reprendre, les colonnes n'existent plus).
    const LISENT_UNE_COLONNE_RETIREE: &[&str] = &[
        "560_delegation_droits_normalises",
        "561_inscription_resolution",
        "562_inscription_scolarite_actuelle",
        "563_signalement_assiduite_detail",
        "662_d036_profil_jours_ouvres",
        "663_d036_parametre_type",
        "664_d036_import_ligne_typee",
        "665_d036_personne_plage_joignable",
        "666_d036_eleve_fusion_normalisee",
        "667_d036_personne_fusion_normalisee",
        // 469 appelle `eva_reprendre_tout()` : le rejeu de 467 vient de le REDÉFINIR dans sa version d'origine, qui lit
        // `ent_devoir`/`ent_note` (des vues sur `eva_*` depuis 472) et laisse un curseur ouvert sur `eva_resultat`
        // pendant la reprise (PostgreSQL refuse alors l'`ALTER TABLE` : « being used by active queries »). La reprise
        // est rejouée par le bloc final de 472, avec les fonctions de 472 (qui lisent les archives).
        "469_eva_v1_reprise_executee",
    ];
    // Connexion détachée : fermée en fin de fonction, ce qui libère le verrou de session même en cas d'erreur.
    let mut verrou = pool.acquire().await?.detach();
    // 7770042 : verrou des préparations de rôles (GRANT sur le catalogue), cf. `tests_rls_stricte`.
    sqlx::query("SELECT pg_advisory_lock(7770042)").execute(&mut verrou).await?;
    sqlx::query("SELECT pg_advisory_lock($1)").bind(CLE_REJEU).execute(&mut verrou).await?;
    let resultat: Result<(), MigrateError> = async {
        for (nom, sql) in liste {
            if LISENT_UNE_COLONNE_RETIREE.contains(nom) {
                continue;
            }
            for (fichier, garde) in GARDES_DE_REJEU {
                if fichier == nom {
                    sqlx::query(garde).execute(&mut verrou).await?;
                }
            }
            // Un test voisin qui travaille sur les mêmes tables pendant le rejeu peut faire de lui la victime d'un
            // verrou mortel (40P01) ou d'un conflit de sérialisation (40001) : les fichiers sont idempotents, on
            // rejoue le fichier (cinq tentatives au plus, attente croissante).
            let mut tentative = 0u64;
            loop {
                match apply_sql_file_rejeu(pool, sql).await {
                    Err(MigrateError::Sqlx(sqlx::Error::Database(e)))
                        if tentative < 4 && e.code().is_some_and(|c| c.starts_with("40")) =>
                    {
                        tentative += 1;
                        tokio::time::sleep(std::time::Duration::from_millis(200 * tentative)).await;
                    }
                    r => break r?,
                }
            }
        }
        Ok(())
    }
    .await;
    // Moyen C (passe 7) : un rejeu qui échoue après une garde (304, 313, 314 retirent la sentinelle) ne laisse **jamais** la
    // sentinelle absente pour les tests suivants : 316, 317 et 318 sont rejouées (elles la recréent), l'erreur d'origine est rendue.
    if resultat.is_err() {
        restaurer_sentinelle(pool).await;
    }
    resultat
}

/// Tests : recrée la sentinelle des verrous d'offre (316, 317, 318 dans l'ordre), appelée quand un rejeu échoue. Un échec ici est
/// journalisé et ne masque pas l'erreur d'origine.
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
pub(crate) async fn restaurer_sentinelle(pool: &PgPool) {
    for fichier in ["316_inscription_i5_suites_sentinelle", "317_inscription_i5_suites_passe6", "318_inscription_i5_suites_passe7"] {
        let (_, sql) = MIGRATIONS.iter().find(|(n, _)| *n == fichier).expect("migration de la sentinelle");
        // Contre-revue du pilote (passe 7) : mêmes nouvelles tentatives que le rejeu (40P01, 40001) ; un échec définitif est
        // écrit sur la sortie d'erreur (les tests n'ont pas d'abonné `tracing`).
        let mut tentative = 0u64;
        loop {
            match apply_sql_file_rejeu(pool, sql).await {
                Err(MigrateError::Sqlx(sqlx::Error::Database(e)))
                    if tentative < 4 && e.code().is_some_and(|c| c.starts_with("40")) =>
                {
                    tentative += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(200 * tentative)).await;
                }
                Err(e) => {
                    eprintln!("RESTAURATION DE LA SENTINELLE EN ÉCHEC ({fichier}) : {e}");
                    tracing::error!(migration = fichier, erreur = %e, "restauration de la sentinelle : échec");
                    break;
                }
                Ok(()) => break,
            }
        }
    }
}

/// Tests (chantier T1) : remet les clauses `search_path` en rejouant **690 seule**, sous les verrous des rejeux. D'autres
/// tests rejouent des fichiers de migration isolés dans la base commune, et chaque `CREATE OR REPLACE FUNCTION` efface la
/// clause ; les preuves de T1 (table temporaire homonyme sous le rôle applicatif) veulent l'état d'une installation.
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
pub(crate) async fn remettre_search_path(pool: &PgPool) -> Result<(), MigrateError> {
    let (_, sql) = MIGRATIONS
        .iter()
        .find(|(nom, _)| *nom == "690_t1_search_path_fonctions")
        .expect("690 est dans le tableau des migrations");
    let mut verrou = pool.acquire().await?.detach();
    sqlx::query("SELECT pg_advisory_lock(7770042)").execute(&mut verrou).await?;
    sqlx::query("SELECT pg_advisory_lock($1)").bind(CLE_REJEU).execute(&mut verrou).await?;
    let mut tentative = 0u64;
    loop {
        match apply_sql_file_rejeu(pool, sql).await {
            Err(MigrateError::Sqlx(sqlx::Error::Database(e)))
                if tentative < 4 && e.code().is_some_and(|c| c.starts_with("40")) =>
            {
                tentative += 1;
                tokio::time::sleep(std::time::Duration::from_millis(200 * tentative)).await;
            }
            r => break r,
        }
    }
}

/// Verrou consultatif entre les rejeux de migrations (exclusif) et les tests qui vérifient un déclencheur ou une
/// politique recréés par ces rejeux (partagé). Le rejeu est fait instruction par instruction : un `DROP TRIGGER`
/// suivi de son `CREATE` laisse un instant la table sans garde, et un test voisin qui vérifie cette garde
/// (ex. immuabilité de `ent_objet`, migration 500) voyait alors l'écriture acceptée. Deux rejeux en parallèle sont
/// aussi sérialisés.
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
const CLE_REJEU: i64 = 7770043;

#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
pub(crate) struct GardeRejeu(sqlx::PgConnection);

/// Prend le verrou **partagé** : aucun rejeu ne tourne tant que la garde est tenue. À libérer avec `liberer()`.
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
pub(crate) async fn garde_rejeu_partagee(pool: &PgPool) -> GardeRejeu {
    let mut c = pool.acquire().await.expect("connexion").detach();
    sqlx::query("SELECT pg_advisory_lock_shared($1)").bind(CLE_REJEU).execute(&mut c).await.expect("verrou");
    GardeRejeu(c)
}

#[cfg(test)]
impl GardeRejeu {
    #[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
    pub(crate) async fn liberer(mut self) {
        let _ = sqlx::query("SELECT pg_advisory_unlock_shared($1)").bind(CLE_REJEU).execute(&mut self.0).await;
    }
}

/// Ordres d'un fichier : commentaires retirés **avant** le découpage sur `;` (sinon un `;` dans un
/// commentaire deviendrait une « requête »), blocs `$$ … $$` préservés.
pub(crate) fn ordres(sql: &str) -> Vec<String> {
    let sans_commentaires: String = sql
        .lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    split_statements(&sans_commentaires)
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Applique un fichier ordre par ordre, sans transaction ni registre (tests seulement).
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
async fn apply_sql_file(pool: &PgPool, sql: &str) -> Result<(), MigrateError> {
    for stmt in ordres(sql) {
        sqlx::query(&stmt).execute(pool).await?;
    }
    Ok(())
}

/// Rejeu : comme `apply_sql_file`, mais un ordre qui vise `ent_note` ou `ent_devoir` comme **table** (colonne, index,
/// contrainte, suppression) est ignoré s'il échoue parce que ce sont désormais des **vues** de compatibilité en
/// lecture seule sur `eva_*` (472 : les anciennes tables sont des archives). Ces ordres appartiennent à des fichiers
/// anciens (008, 022, 111, 180, 460, 2402) dont la reprise ne porte plus sur ces objets ; tout autre échec reste fatal.
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
async fn apply_sql_file_rejeu(pool: &PgPool, sql: &str) -> Result<(), MigrateError> {
    for stmt in ordres(sql) {
        if let Err(e) = sqlx::query(&stmt).execute(pool).await {
            let code = match &e {
                sqlx::Error::Database(d) => d.code().map(|c| c.to_string()).unwrap_or_default(),
                _ => String::new(),
            };
            if rejeu_tolere(&code, &stmt) {
                continue;
            }
            return Err(e.into());
        }
    }
    Ok(())
}

/// L'unique ordre de 460 qui échoue au rejeu en `42601` : l'archivage des doublons de notes (`SELECT n.*` face à
/// `ent_note_doublon_archive`, dont la forme ne suit plus celle de la vue). Texte **exact**, espaces normalisés.
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
const ORDRE_460_DOUBLONS: &str = "WITH classees AS ( SELECT n.id, first_value(n.id) OVER w AS garde FROM ent_note n LEFT JOIN (SELECT DISTINCT note_id FROM ent_note_correction) c ON c.note_id = n.id WHERE n.devoir_id IS NOT NULL WINDOW w AS (PARTITION BY n.etablissement_id, n.devoir_id, n.eleve_id ORDER BY (c.note_id IS NOT NULL) DESC, n.created_at DESC, n.id DESC) ), en_trop AS ( SELECT id, garde FROM classees WHERE id <> garde ), archivees AS ( INSERT INTO ent_note_doublon_archive SELECT n.*, e.garde, now() FROM ent_note n JOIN en_trop e ON e.id = n.id RETURNING id ) DELETE FROM ent_note WHERE id IN (SELECT id FROM archivees)";

/// Le rejeu tolère-t-il cet échec ? **Liste fermée** (V3c, M5) : avant, trois SQLSTATE (42809, 55000, 42601) étaient tolérés
/// pour tout ordre citant `ent_note` ou `ent_devoir`, ce qui aurait masqué une vraie faute de syntaxe dans une migration
/// future qui cite ces noms. Désormais :
/// * `42809` (« n'est pas une table ») : seulement une instruction qui cite `ent_note` ou `ent_devoir` comme identifiant
///   entier (colonne, index, contrainte, déclencheur sur ce qui est devenu une vue) ;
/// * `55000` : seulement l'appel exact `SELECT count(*) FROM ent_rattacher_classes()` (fonction de 112 redéfinie par le rejeu) ;
/// * `42601` : seulement l'ordre exact de 460 (`ORDRE_460_DOUBLONS`) ;
/// * tout autre code, ou tout autre ordre : fatal.
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
fn rejeu_tolere(code: &str, stmt: &str) -> bool {
    let normalise = stmt.split_whitespace().collect::<Vec<_>>().join(" ");
    match code {
        "42809" => vise_ent_note_ou_devoir(stmt),
        "55000" => normalise == "SELECT count(*) FROM ent_rattacher_classes()",
        "42601" => normalise == ORDRE_460_DOUBLONS,
        _ => false,
    }
}

/// Vrai si l'ordre cite `ent_note` ou `ent_devoir` comme identifiant entier (pas `ent_note_periode`, `ent_devoir_archive`…).
#[cfg(test)]
#[cfg_attr(not(all(test, feature = "labo")), allow(dead_code))] // utilisé seulement par des tests réservés à la feature labo
fn vise_ent_note_ou_devoir(stmt: &str) -> bool {
    let est_id = |c: char| c.is_ascii_alphanumeric() || c == '_';
    ["ent_note", "ent_devoir"].iter().any(|nom| {
        stmt.match_indices(nom).any(|(i, _)| {
            let avant = stmt[..i].chars().next_back();
            let apres = stmt[i + nom.len()..].chars().next();
            !avant.is_some_and(est_id) && !apres.is_some_and(est_id)
        })
    })
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
    #[test]
    fn le_rejeu_ne_tolere_que_les_ordres_exacts_attendus() {
        use super::{rejeu_tolere, ORDRE_460_DOUBLONS};
        // L'ordre exact de 460 est toléré en 42601 (espaces et retours à la ligne indifférents)…
        assert!(rejeu_tolere("42601", ORDRE_460_DOUBLONS));
        assert!(rejeu_tolere("42601", &ORDRE_460_DOUBLONS.replace(' ', "\n  ")));
        // …mais pas une autre faute de syntaxe qui cite `ent_note` (c'était toléré avant V3c).
        assert!(!rejeu_tolere("42601", "SELECT n.* FROM ent_note n WHERE n.etablissement_id = $1 AND"));
        assert!(!rejeu_tolere("42601", &format!("{ORDRE_460_DOUBLONS} -- modifié")));
        assert!(!rejeu_tolere("42601", "INSERT INTO ent_note_doublon_archive SELECT n.* FROM ent_note n WHERE n.etablissement_id = $1"));
        // 55000 : l'appel exact, pas un autre appel ni un ordre qui cite la vue.
        assert!(rejeu_tolere("55000", "SELECT count(*) FROM ent_rattacher_classes()"));
        assert!(!rejeu_tolere("55000", "UPDATE ent_note SET valeur = 1 WHERE etablissement_id = $1"));
        assert!(!rejeu_tolere("55000", "SELECT count(*) FROM ent_autre_fonction()"));
        // 42809 : DDL qui cite la vue comme identifiant entier ; pas `ent_note_periode`, ni un ordre qui ne la cite pas.
        assert!(rejeu_tolere("42809", "CREATE INDEX IF NOT EXISTS ent_note_eleve_idx ON ent_note (etablissement_id)"));
        assert!(rejeu_tolere("42809", "ALTER TABLE ent_devoir ADD COLUMN IF NOT EXISTS x INT"));
        assert!(!rejeu_tolere("42809", "ALTER TABLE ent_note_periode ADD COLUMN x INT"));
        // Tout autre code reste fatal, même pour un ordre qui cite ent_note.
        for code in ["42P01", "23505", "42703", "22P02", ""] {
            assert!(!rejeu_tolere(code, "ALTER TABLE ent_note ADD COLUMN IF NOT EXISTS statut TEXT"));
        }
    }

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
        // Tri NUMÉRIQUE sur le préfixe : les plages à quatre chiffres (2820–2839, socle 41)
        // dépassent celles à trois chiffres, ce que le tri alphabétique ne voit pas.
        noms.sort_by_key(|n| {
            (
                n.split('_').next().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0),
                n.clone(),
            )
        });
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

    /// Règle 38 (T1) : l'ordre des migrations **versées sur `ENT`** est figé. Une base déjà migrée n'applique une migration
    /// nouvelle qu'à la suite de celles qu'elle a déjà : insérée au milieu du tableau, elle passerait après 690 sur une base
    /// migrée mais avant sur une base neuve (fonctions sans `search_path`, ordre de rejeu faux). Toute migration nouvelle
    /// s'ajoute donc À LA FIN du tableau `MIGRATIONS`. La référence est `migrations_versees.txt`, **préfixe obligatoire** du
    /// tableau ; le pilote y ajoute, au versement, les migrations du lot versé (dans l'ordre du tableau).
    #[test]
    fn l_ordre_des_migrations_versees_est_fige_regle_38() {
        let figees: Vec<&str> = include_str!("migrations_versees.txt").lines().filter(|l| !l.trim().is_empty()).collect();
        let tableau: Vec<&str> = MIGRATIONS.iter().map(|(n, _)| *n).collect();
        let regle = "Règle 38 (docs/securite/T1_SEARCH_PATH.md, PILOTAGE) : une migration nouvelle s'ajoute À LA FIN du tableau \
                     MIGRATIONS, après toutes les migrations versées (migrations_versees.txt), jamais au milieu. Une base déjà \
                     migrée l'appliquerait dans un autre ordre qu'une base neuve (et après 690 : fonctions sans `search_path`). ";
        assert!(figees.contains(&"690_t1_search_path_fonctions"), "{regle}la liste de référence doit contenir 690");
        assert!(tableau.len() >= figees.len(), "{regle}des migrations versées ont disparu du tableau");
        for (i, attendue) in figees.iter().enumerate() {
            assert_eq!(
                tableau[i], *attendue,
                "{regle}position {i} : `{}` trouvée là où la migration versée `{attendue}` est attendue (insertion, déplacement ou retrait)",
                tableau[i]
            );
        }
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
