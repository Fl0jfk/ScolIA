//! Socle 2 (l'élève), lot E6 — registre `TABLES_ELEVE` : TOUTE table qui porte `eleve_id`
//! et ce que devient chacune de ses lignes quand deux fiches d'élèves sont fusionnées (§7.10
//! de `docs/socles/02_ELEVE.md`).
//!
//! Même esprit que `TABLES_CLASSE` (référentiel) et que la garde SQL statique : un **test de
//! garde** (`ent-db`, `tests_eleve_fusion`) lit `information_schema` et **échoue** si une
//! table qui porte `eleve_id` n'est pas déclarée ici (ou si une table déclarée n'existe plus,
//! ou si la clé primaire déclarée n'est pas la vraie). Aucune table nouvelle n'échappe donc à
//! la fusion : l'auteur d'une migration doit choisir une stratégie.
//!
//! Deux stratégies seulement, volontairement :
//! * [`Strategie::Deplacer`] — toutes les lignes passent à l'élève conservé. Si une ligne de
//!   l'absorbé entre en conflit d'unicité avec une ligne du conservé (même année d'inscription,
//!   même INE, scolarisations qui se chevauchent…), la ligne **du conservé l'emporte** et celle
//!   de l'absorbé reste sur l'absorbé, listée dans l'aperçu et dans le registre de la fusion.
//! * [`Strategie::Reste`] — la table est un **historique** ou un **état propre à la fiche** :
//!   ses lignes restent attachées à l'élève absorbé (lisibles via la fusion, jamais perdues).
//!
//! Ce module est pur (aucune base de données) : le déplacement est dans `ent-db::eleve_fusion`.

/// Délai pendant lequel une fusion s'annule (Q-E16 : 30 jours, comme les personnes).
pub const DELAI_DEFUSION_JOURS: i64 = 30;

/// Ce que devient une table lors d'une fusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategie {
    /// Les lignes passent à l'élève conservé ; en cas de conflit d'unicité, le conservé l'emporte.
    Deplacer,
    /// Les lignes restent sur l'élève absorbé (historique ou état propre à la fiche).
    Reste,
}

/// Une table qui porte `eleve_id`.
#[derive(Debug, Clone, Copy)]
pub struct TableEleve {
    pub table: &'static str,
    /// Colonnes de la clé primaire réelle (vérifiée contre la base par le test de garde).
    pub cle: &'static [&'static str],
    pub strategie: Strategie,
    /// Pourquoi la table reste sur l'absorbé (vide pour `Deplacer`).
    pub raison: &'static str,
}

impl TableEleve {
    /// Colonnes qui identifient une ligne **indépendamment de l'élève** (la clé sans `eleve_id`).
    pub fn cle_hors_eleve(&self) -> Vec<&'static str> {
        self.cle.iter().copied().filter(|c| *c != "eleve_id").collect()
    }
}

const fn d(table: &'static str, cle: &'static [&'static str]) -> TableEleve {
    TableEleve { table, cle, strategie: Strategie::Deplacer, raison: "" }
}

const fn r(table: &'static str, cle: &'static [&'static str], raison: &'static str) -> TableEleve {
    TableEleve { table, cle, strategie: Strategie::Reste, raison }
}

/// Registre de toutes les tables qui portent `eleve_id` (ordre alphabétique).
pub const TABLES_ELEVE: &[TableEleve] = &[
    d("ent_absence", &["id"]),
    d("ent_absence_declaration", &["id"]),
    d("ent_absence_seance", &["id"]),
    d("ent_absence_signal", &["id"]),
    d("ent_aesh_affectation", &["id"]),
    r("ent_afaire", &["id"], "index des tâches du socle 29 : projection reconstructible, jamais une source de vérité ; le rapprochement ré-attache chaque tâche à l'élève conservé à partir de son propriétaire"),
    d("ent_affectation", &["id"]),
    r("ent_affectation_supprimee", &["archive_id"], "archive des affectations supprimées : trace en ajout seul, reste sur la fiche où la suppression a eu lieu"),
    d("ent_alerte_cpe", &["id"]),
    d("ent_amenagement", &["id"]),
    d("ent_appel_ligne", &["id"]),
    r("ent_appel_ligne_histo", &["id"], "historique des lignes d'appel en ajout seul (déclencheur) : trace du geste, reste sur la fiche où il a eu lieu"),
    d("ent_asd_absence", &["id"]),
    d("ent_asd_absence_seance", &["id"]),
    d("ent_asd_alerte", &["id"]),
    r("ent_asd_avis_demi_journee", &["id"], "registre des avis d'absence par demi-journée : l'avis (socle 8) et son sort restent sur la fiche pour laquelle ils ont été émis"),
    r("ent_asd_contact", &["id"], "contact tracé avec la famille, en ajout seul (déclencheur) : reste sur la fiche concernée"),
    r("ent_asd_courrier", &["id"], "courrier postal préparé, décidé puis posté : champs d'origine figés (déclencheur), reste sur la fiche concernée"),
    d("ent_asd_demande", &["id"]),
    r("ent_asd_demi_journee", &["etablissement_id", "eleve_id", "jour", "periode"], "matérialisation recalculable du décompte : les épisodes suivent l'élève conservé, le recalcul reconstruit ses lignes"),
    r("ent_asd_mois_arrete", &["etablissement_id", "eleve_id", "mois"], "décompte figé d'un mois arrêté, en ajout seul : reste sur la fiche pour laquelle il a été arrêté"),
    r("ent_asd_prevenir", &["id"], "décision « prévenir maintenant » en ajout seul (déclencheur) : trace datée de l'alerte, reste sur la fiche concernée"),
    r("ent_asd_rectification", &["id"], "rectification postérieure à l'arrêt, en ajout seul (déclencheur) : trace datée, reste sur la fiche concernée"),
    d("ent_asd_relance", &["id"]),
    d("ent_asd_situation", &["id"]),
    r("ent_asd_vigilance", &["id"], "drapeau de vigilance posé par une personne (un seul vivant par élève, déclencheur) : décision sur la fiche, ne suit pas la fusion"),
    d("ent_bilan_cycle", &["id"]),
    r("ent_bulletin", &["id"], "bulletin émis : document figé (contenu et élève immuables)"),
    d("ent_campagne_destinataire", &["campagne_id", "user_id"]),
    d("ent_carnet_entree", &["id"]),
    d("ent_cdt_abonnement", &["id"]),
    r("ent_cdt_affichage", &["eleve_id"], "réglages d'affichage de l'élève (une ligne par élève) : outil personnel, reste sur la fiche conservée"),
    d("ent_cdt_amenagement_simule", &["id"]),
    d("ent_cdt_coche", &["travail_id", "eleve_id"]),
    d("ent_cdt_continuite", &["id"]),
    d("ent_cdt_delai_demande", &["id"]),
    d("ent_cdt_depot_signalement", &["id"]),
    r("ent_cdt_echeance_eleve", &["id"], "prolongation individuelle en ajout seul (R-C9) : la ligne ne se réécrit pas, elle reste sur la fiche où elle a été accordée"),
    r("ent_cdt_groupe_membre", &["groupe_id", "eleve_id", "depuis"], "appartenance datée à un groupe de remise : historique propre à la fiche (garde d'unicité du groupe actif)"),
    d("ent_cdt_note_accompagnement", &["id"]),
    d("ent_cdt_relance", &["id"]),
    r("ent_cdt_remise", &["id"], "remise et dépôts immuables d'un élève (R-C9, une remise par élève et par travail) : l'œuvre reste sur la fiche de son auteur"),
    r("ent_cdt_resume_envoi", &["eleve_id", "semaine_debut"], "trace d'envoi du résumé hebdomadaire (un seul par élève et par semaine) : état propre à la fiche, jamais rejoué pour l'élève conservé"),
    r("ent_cdt_statut", &["id"], "constat du professeur en ajout seul (R-C12) : reste sur la fiche concernée"),
    d("ent_cdt_travail_cible", &["travail_id", "eleve_id"]),
    d("ent_cdt_variante_eleve", &["variante_id", "eleve_id"]),
    d("ent_competence_evaluation", &["id"]),
    d("ent_conflit_interet", &["id"]),
    d("ent_conseil_classe_dossier", &["id"]),
    d("ent_contrainte_publiee", &["id"]),
    d("ent_convention_stage", &["id"]),
    r("ent_cours_membre", &["id"], "inclusion ou exclusion datée d'un cours : ligne immuable (seule l'annulation est permise), elle reste sur la fiche pour laquelle elle a été décidée"),
    d("ent_creneau_rdv", &["id"]),
    r("ent_dem_demande", &["id"], "demande du socle N1 : preuve en ajout seul (faits chaînés), identité et version gardées par la base ; la demande reste rattachée à la fiche sur laquelle elle a été déposée"),
    d("ent_dispense_vs", &["id"]),
    d("ent_doc_a_signer", &["id"]),
    r("ent_doc_document", &["id"], "document émis du socle 7 : identité, numéro et sujet immuables (déclencheur), le document reste rattaché à la fiche pour laquelle il a été émis ; le rattachement à la fiche conservée est une commande de classement tracée (documents.classer), jamais une réécriture"),
    r("ent_doc_mise_a_disposition", &["id"], "mise à disposition d'un exemplaire : titulaire figé par la base (seuls les accusés et le retrait s'ajoutent), elle reste sur la fiche d'origine ; le coffre de l'élève conservé se reconstruit par le classement des documents"),
    r("ent_doc_piece", &["id"], "pièce justificative du socle 7 : élève, déposant, original et empreinte immuables (déclencheur) et décisions en ajout seul, la pièce reste rattachée à la fiche pour laquelle elle a été déposée ; l'élève conservé redemande ou réutilise la pièce par la démarche (réutilisation confirmée), jamais par une réécriture"),
    r("ent_doc_piece_demande", &["id"], "demande de pièce du socle 7 : sujet immuable (déclencheur), la demande reste sur la fiche pour laquelle elle a été ouverte ; l'élève conservé reçoit ses propres demandes"),
    r("ent_doc_signataire", &["id"], "signataire d'un circuit de signature du socle 7 : identité, rôle et élève immuables (déclencheur), la signature chaînée est en ajout seul ; la ligne reste sur la fiche de l'élève pour laquelle le circuit a été lancé, jamais réécrite (la preuve garde ce qui a été signé, par qui)"),
    r("ent_edt_individuelle", &["id"], "version individuelle d'emploi du temps (socle 10, T5) : fait daté de la décision, dont l'effet est porté par `ent_cours_membre` (lui aussi immuable et laissé sur la fiche) ; elle reste sur la fiche pour laquelle elle a été décidée"),
    r("ent_edt_localisation_journal", &["id"], "journal en ajout seul (déclencheur) des lectures « où est cet élève » : trace de qui a regardé, reste sur la fiche consultée"),
    d("ent_eleve_bourse", &["id"]),
    d("ent_eleve_capacite", &["id"]),
    d("ent_eleve_compte", &["etablissement_id", "eleve_id"]),
    d("ent_eleve_confidentialite", &["id"]),
    d("ent_eleve_contact_urgence", &["id"]),
    r("ent_eleve_coordonnee", &["id"], "coordonnées propres de l'élève (E7) : lignes à états gardés par la base (valeur immuable, retrait daté), personnelles à la fiche — l'élève conservé garde et vérifie les siennes ; celles de l'absorbé restent sur sa fiche"),
    r("ent_eleve_coordonnee_verification", &["id"], "code de vérification d'une coordonnée (empreinte seule, quinze minutes) : état éphémère propre à la coordonnée, jamais déplacé"),
    d("ent_eleve_dispense_repas", &["id"]),
    d("ent_eleve_dispositif", &["id"]),
    d("ent_eleve_document", &["id"]),
    d("ent_eleve_enseignement", &["id"]),
    r("ent_eleve_enseignement_a_traiter", &["id"], "diagnostic de la reprise des options en enseignements suivis"),
    d("ent_eleve_grille_repas", &["id"]),
    d("ent_eleve_identifiant", &["id"]),
    r("ent_eleve_identite", &["id"], "versions d'identité immuables : l'historique d'état civil reste sur la fiche absorbée"),
    r("ent_eleve_identite_a_traiter", &["id"], "diagnostic de la reprise des identités"),
    r("ent_eleve_identite_scellee", &["id"], "valeurs scellées des versions d'identité sensibles (E7, ajout seul) : suivent leur version, qui reste sur la fiche absorbée"),
    r("ent_eleve_indicateur", &["etablissement_id", "eleve_id", "code"], "pastilles de l'élève (E8) : projection recalculable depuis les tables sources, reconstruite pour la fiche conservée par l'événement `eleve.fusionne`"),
    d("ent_eleve_inscription", &["id"]),
    r("ent_eleve_inscription_a_traiter", &["id"], "diagnostic de la reprise des inscriptions"),
    d("ent_eleve_interdiction", &["id"]),
    d("ent_eleve_invitation", &["id"]),
    d("ent_eleve_lien", &["id"]),
    r("ent_eleve_majorite_evenement", &["eleve_id", "sorte"], "marqueurs de la tâche nocturne, propres à chaque fiche"),
    d("ent_eleve_mobilite", &["id"]),
    d("ent_eleve_modalite", &["id"]),
    d("ent_eleve_option", &["id"]),
    r("ent_eleve_option_a_traiter", &["id"], "diagnostic de la reprise des options"),
    d("ent_eleve_option_annee", &["id"]),
    d("ent_eleve_option_groupe", &["source", "option_id"]),
    r("ent_eleve_orphelin", &["id"], "diagnostic des clés composées"),
    d("ent_eleve_periode", &["id"]),
    r("ent_eleve_photo", &["id"], "photos de l'élève (E7) : objets immuables du stockage, une photo validée au plus par fiche (index unique) ; la photo de l'absorbé reste sur sa fiche, l'élève conservé garde la sienne"),
    r("ent_eleve_photo_refus", &["id"], "refus de la photo administrative (E7) : décision de la famille propre à la fiche, jamais transposée à l'autre fiche"),
    r("ent_eleve_proposition", &["id"], "propositions déposées sur la fiche absorbée (immuables une fois décidées)"),
    d("ent_eleve_scolarisation", &["id"]),
    d("ent_eleve_scolarite_annee", &["id"]),
    r("ent_eleve_scolarite_ecart", &["id"], "diagnostic du statut dérivé"),
    d("ent_equipe_reunion", &["id"]),
    r("ent_fac_ligne", &["id"], "ligne de facture : pièce comptable immuable une fois émise (déclencheur de la migration 520) et écriture réservée au service de facturation (R-F1, socle 24) ; elle reste sur la fiche facturée, la fusion ne réécrit jamais une pièce"),
    d("ent_fiche_sante", &["id"]),
    r("ent_fin_consequence", &["id"], "conséquence d'un changement de payeur : historique financier proposé puis décidé par la comptabilité, il reste daté sur la fiche où il a été calculé"),
    r("ent_fin_designation", &["id"], "désignation de payeur datée : historique de qui a payé quoi et quand (R-F4), jamais déplacée ni réécrite ; la fusion reprend les payeurs de l'élève conservé par une nouvelle désignation, pas par une copie"),
    r("ent_fin_journal", &["id"], "journal financier en ajout seul (R-F27) : trace des gestes, conservée telle quelle"),
    r("ent_fin_prise_en_charge", &["id"], "prise en charge d'un organisme : engagement daté et référencé, il reste sur la fiche pour laquelle il a été accordé"),
    r("ent_fin_reduction_eleve", &["id"], "réduction accordée à un élève : décision datée de la comptabilité, non transférée à un autre élève sans décision ; écriture réservée au service de facturation (R-F1)"),
    r("ent_fin_tache", &["id"], "tâche du secrétariat ou de la comptabilité : trace de travail, elle reste sur la fiche concernée"),
    d("ent_formulaire_reponse", &["id"]),
    r("ent_frm_cible", &["id"], "cible résolue d'un formulaire du socle 18 (audience figée à la publication) : une ligne par élève, jamais réécrite, elle reste datée sur la fiche d'origine ; l'élève conservé garde ses propres cibles"),
    r("ent_frm_soumission", &["id"], "réponse envoyée à un formulaire du socle 18 : ajout seul (déclencheur), preuve de ce qu'un répondant a envoyé pour cet élève, elle reste sur la fiche pour laquelle elle a été donnée"),
    d("ent_import_ligne", &["id"]),
    d("ent_incident", &["id"]),
    d("ent_infirmerie_passage", &["id"]),
    d("ent_info_comp_valeur", &["id"]),
    d("ent_inscription_dossier", &["id"]),
    d("ent_inscription_lien_attente", &["id"]),
    d("ent_inscription_resolution", &["id"]),
    d("ent_inscription_suivi", &["id"]),
    r("ent_internat_absence", &["id"], "absence d'internat annoncée par la famille (statut, décision, régularisation) : engagement daté et tracé, jamais supprimé, il reste sur la fiche concernée"),
    d("ent_internat_affectation", &["id"]),
    r("ent_internat_appel_attendu", &["appel_id", "eleve_id"], "liste figée d'un appel de nuit (ajout seul) : ce qu'on croyait à l'heure de l'appel, elle reste sur la fiche pour laquelle elle a été figée"),
    r("ent_internat_cas", &["id"], "cas de nuit (non retrouvé, retour tardif) : procédure, étapes et issue datées, jamais supprimés, ils restent sur la fiche concernée"),
    r("ent_internat_correspondant", &["id"], "correspondant d'internat désigné par la famille (personne, jours, suspension) : désignation datée qui reste sur la fiche pour laquelle elle a été faite"),
    r("ent_internat_derogation", &["id"], "dérogation de la direction en ajout seul : décision datée qui reste sur la fiche pour laquelle elle a été prise"),
    r("ent_internat_i3_fait", &["id"], "faits d'absence, de sortie, de correspondant et de requalification de contact en ajout seul : historique daté qui reste sur la fiche concernée"),
    r("ent_internat_lecture_sensible", &["id"], "journal des lectures sensibles de l'internat en ajout seul : trace d'accès, elle reste sur la fiche lue"),
    r("ent_internat_nuit_exceptionnelle", &["id"], "nuit exceptionnelle datée (élève accueilli une nuit de plus) : décision qui reste sur la fiche pour laquelle elle a été prise"),
    r("ent_internat_perte_droit_fait", &["id"], "faits de perte de droit sur un lien en ajout seul : date immuable de la perte, elle reste sur la fiche concernée"),
    d("ent_internat_presence", &["id"]),
    r("ent_internat_presence_fait", &["id"], "faits de présence en ajout seul (R-IT13) : l'historique de la nuit reste sur la fiche où il a été écrit"),
    r("ent_internat_retrait", &["id"], "retrait de l'attendu de nuit, motif codé, annulable seulement : historique qui reste sur la fiche concernée"),
    r("ent_internat_sejour", &["id"], "séjour d'hébergement daté, sans chevauchement par élève : reste sur la fiche absorbée (le séjour de la fiche gardée se resynchronise sur le régime)"),
    r("ent_internat_sortie", &["id"], "sortie individuelle de l'internat (week-end, mercredi, soirée, correspondant) : engagement daté, départ et retour contrôlés, jamais supprimé, il reste sur la fiche concernée"),
    d("ent_internat_suivi", &["id"]),
    r("ent_internat_tiers_autorise", &["id"], "tiers autorisé à récupérer l'élève (posé par le personnel à la demande écrite d'un titulaire, daté, révocable) : décision tracée qui reste sur la fiche pour laquelle elle a été prise"),
    r("ent_journal_modif", &["id"], "journal d'audit en ajout seul : l'historique reste sur la fiche où il a été écrit"),
    d("ent_lien_attente", &["id"]),
    r("ent_mail_outbox", &["id"], "file d'envoi : messages déjà adressés"),
    r("ent_metier_event", &["id"], "événements déjà publiés : historique immuable"),
    d("ent_msg_brouillon", &["id"]),
    d("ent_msg_fil", &["id"]),
    d("ent_msg_fil_eleve", &["fil_id", "eleve_id"]),
    d("ent_msg_lot_eleve", &["lot_id", "eleve_id"]),
    d("ent_msg_participant", &["id"]),
    d("ent_note_appreciation", &["id"]),
    d("ent_note_archive", &["id"]),
    d("ent_note_correction", &["id"]),
    d("ent_note_doublon_archive", &["id"]),
    d("ent_notification", &["id"]),
    d("ent_notification_envoi", &["id"]),
    d("ent_ntf_avis", &["id"]),
    d("ent_orientation_avis", &["id"]),
    d("ent_pai", &["id"]),
    d("ent_pai_piece", &["id"]),
    d("ent_pai_signature", &["id"]),
    d("ent_personne", &["id"]),
    r("ent_personne_fusion_contact_supprime", &["fusion_id", "id"], "copie d'un contact d'urgence supprimé par une fusion de personnes : trace de restauration, reste sur la fiche où la fusion a eu lieu"),
    r("ent_personne_fusion_lien_supprime", &["fusion_id", "id"], "copie d'un lien supprimé par une fusion de personnes : trace de restauration, reste sur la fiche où la fusion a eu lieu"),
    d("ent_procedure_disciplinaire", &["id"]),
    d("ent_psy_dossier", &["id"]),
    d("ent_psy_entretien", &["id"]),
    d("ent_psy_rdv", &["id"]),
    d("ent_punition", &["id"]),
    r("ent_push_outbox", &["id"], "file d'envoi : notifications déjà adressées"),
    r("ent_rdv_fait", &["id"], "fil de faits d'un rendez-vous en ajout seul : reste sur la fiche où le fait a eu lieu"),
    d("ent_repas_passage", &["id"]),
    d("ent_retard", &["id"]),
    d("ent_sanction", &["id"]),
    d("ent_signalement_assiduite", &["id"]),
    r("ent_sms_outbox", &["id"], "file d'envoi : SMS déjà adressés"),
    d("ent_sortie_autorisation", &["id"]),
    d("ent_sortie_controle", &["id"]),
    d("ent_sortie_demande", &["id"]),
    d("ent_sortie_participant", &["id"]),
    d("ent_sortie_publication_photo_eleve", &["id"]),
    d("ent_sortie_reprise_participant", &["id"]),
    d("ent_stage_periode_public", &["id"]),
    d("ent_travail_eleve", &["travail_id", "eleve_id"]),
    d("ent_travail_remise", &["id"]),
    d("ent_voeu_inscription", &["id"]),
    d("ent_vs_commission_dossier", &["id"]),
    r("eva_appreciation", &["id"], "appréciation versionnée du socle 21 : élève immuable (déclencheur de 465) ; les appréciations restent sur la fiche où elles ont été saisies (lisibles via la fusion) ; le rapprochement pédagogique vers l'élève conservé est une suite du lot V3"),
    r("eva_consultation", &["id"], "journal en ajout seul des consultations de résultats : trace de qui a regardé, reste sur la fiche consultée"),
    r("eva_dispense_port", &["id"], "dispense ou inaptitude datée, SANS motif (port 34 en simulation, socle 21 V4) : décision rattachée à cette fiche, elle reste sur la fiche où elle a été déclarée (lisible via la fusion) ; le rapprochement pédagogique vers l'élève conservé est une suite du lot V3"),
    r("eva_enseignement_externe", &["id"], "enseignement suivi hors de l'établissement (socle 21) : déclaré pour cette fiche, il suit ses résultats externes, qui restent sur la fiche où ils ont été saisis (lisibles via la fusion) ; le rapprochement pédagogique vers l'élève conservé est une suite du lot V3"),
    r("eva_evaluation_restriction", &["evaluation_id", "eleve_id"], "public restreint d'une évaluation : cohérent avec les résultats de la fiche, qui restent sur la fiche où ils ont été saisis (lisibles via la fusion) ; le rapprochement pédagogique vers l'élève conservé est une suite du lot V3"),
    r("eva_instantane_ligne", &["id"], "ligne d'un instantané de période figé (socle 21) : document gelé, jamais réécrit"),
    r("eva_moyenne_statut", &["id"], "statut de moyenne (non évalué, dispensé…) attaché aux résultats de la fiche, qui restent sur la fiche où ils ont été saisis (lisibles via la fusion) ; le rapprochement pédagogique vers l'élève conservé est une suite du lot V3"),
    r("eva_rattrapage", &["id"], "convocation de rattrapage ou de remplacement (socle 21 V4) : élève immuable (déclencheur de 475), états en machine à états ; les convocations restent sur la fiche où elles ont été faites (lisibles via la fusion) ; le rapprochement pédagogique vers l'élève conservé est une suite du lot V3"),
    r("eva_rattrapage_autorise", &["id"], "autorisation de rattrapage versionnée, liée aux résultats de la fiche, qui restent sur la fiche où ils ont été saisis (lisibles via la fusion) ; le rapprochement pédagogique vers l'élève conservé est une suite du lot V3"),
    r("eva_rattrapage_historique", &["id"], "historique en ajout seul des convocations de rattrapage : trace, reste sur la fiche d'origine"),
    r("eva_resultat", &["id"], "résultat d'évaluation du socle 21 : élève immuable (déclencheur de 464), historique en ajout seul ; les résultats restent sur la fiche où ils ont été saisis (lisibles via la fusion) ; le rapprochement pédagogique vers l'élève conservé est une suite du lot V3"),
    r("eva_resultat_externe", &["id"], "résultat obtenu hors de l'établissement : déclaré pour cette fiche, les résultats restent sur la fiche où ils ont été saisis (lisibles via la fusion) ; le rapprochement pédagogique vers l'élève conservé est une suite du lot V3"),
    r("eva_resultat_historique", &["id"], "historique en ajout seul des corrections de résultats : trace, reste sur la fiche d'origine"),
];

/// Domaine métier d'une table, pour l'aperçu (libellé que le lecteur comprend, pas le nom
/// technique) ; les domaines protégés sont regroupés pour ne rien révéler de plus que ce que
/// l'acteur a le droit de savoir (voir [`domaine_protege`]).
pub fn domaine_fusion(table: &str) -> &'static str {
    const REGLES: &[(&str, &str)] = &[
        ("ent_psy_", "Suivi et santé (protégé)"),
        ("ent_fiche_sante", "Suivi et santé (protégé)"),
        ("ent_pai", "Suivi et santé (protégé)"),
        ("ent_infirmerie", "Suivi et santé (protégé)"),
        ("ent_amenagement", "Suivi et santé (protégé)"),
        ("ent_aesh", "Suivi et santé (protégé)"),
        ("ent_eleve_dispositif", "Suivi et santé (protégé)"),
        ("ent_absence", "Absences et retards"),
        ("ent_appel", "Absences et retards"),
        ("ent_asd_", "Absences et retards"),
        ("ent_retard", "Absences et retards"),
        ("ent_signalement_assiduite", "Absences et retards"),
        ("ent_alerte_cpe", "Absences et retards"),
        ("ent_sortie", "Sorties"),
        ("ent_afaire", "Démarches à faire"),
        ("ent_note", "Évaluations"),
        ("eva_", "Évaluations"),
        ("ent_competence", "Évaluations"),
        ("ent_bulletin", "Évaluations"),
        ("ent_bilan", "Évaluations"),
        ("ent_orientation", "Évaluations"),
        ("ent_conseil", "Évaluations"),
        ("ent_travail", "Travail à faire"),
        ("ent_cdt_", "Travail à faire"),
        ("ent_sanction", "Discipline"),
        ("ent_punition", "Discipline"),
        ("ent_incident", "Discipline"),
        ("ent_procedure", "Discipline"),
        ("ent_dispense_vs", "Discipline"),
        ("ent_carnet", "Carnet de liaison"),
        ("ent_vs_", "Vie scolaire"),
        ("ent_fac_", "Finances et restauration"),
        ("ent_fin_", "Finances et restauration"),
        ("ent_eleve_bourse", "Finances et restauration"),
        ("ent_eleve_grille_repas", "Finances et restauration"),
        ("ent_eleve_dispense_repas", "Finances et restauration"),
        ("ent_repas", "Finances et restauration"),
        ("ent_internat", "Internat"),
        ("ent_msg", "Messages et notifications"),
        ("ent_notification", "Messages et notifications"),
        ("ent_ntf_avis", "Messages et notifications"),
        ("ent_mail", "Messages et notifications"),
        ("ent_push", "Messages et notifications"),
        ("ent_sms", "Messages et notifications"),
        ("ent_campagne", "Messages et notifications"),
        ("ent_formulaire", "Messages et notifications"),
        ("ent_frm_cible", "Messages et notifications"),
        ("ent_frm_soumission", "Messages et notifications"),
        ("ent_creneau_rdv", "Rencontres"),
        ("ent_rdv_fait", "Rencontres"),
        ("ent_equipe_reunion", "Rencontres"),
        ("ent_inscription", "Inscriptions"),
        ("ent_voeu", "Inscriptions"),
        ("ent_lien_attente", "Inscriptions"),
        ("ent_import", "Inscriptions"),
        ("ent_convention_stage", "Stages"),
        ("ent_stage_periode_public", "Stages"),
        ("ent_sortie_participant", "Sorties et voyages"),
        ("ent_dem_demande", "Demandes"),
        ("ent_doc_a_signer", "Documents"),
        ("ent_doc_document", "Documents"),
        ("ent_doc_mise_a_disposition", "Documents"),
        ("ent_doc_piece", "Documents"),
        ("ent_doc_signataire", "Documents"),
        ("ent_eleve_document", "Documents"),
        ("ent_eleve_identite", "Identité"),
        ("ent_eleve_identifiant", "Identité"),
        ("ent_eleve_proposition", "Identité"),
        ("ent_eleve_confidentialite", "Identité"),
        ("ent_eleve_coordonnee", "Identité"),
        ("ent_eleve_photo", "Identité"),
        ("ent_eleve_indicateur", "Identité"),
        ("ent_eleve_lien", "Responsables et contacts"),
        ("ent_eleve_contact", "Responsables et contacts"),
        ("ent_eleve_interdiction", "Responsables et contacts"),
        ("ent_personne", "Responsables et contacts"),
        ("ent_conflit_interet", "Responsables et contacts"),
        ("ent_eleve_scolar", "Scolarité"),
        ("ent_eleve_inscription", "Scolarité"),
        ("ent_eleve_option", "Scolarité"),
        ("ent_eleve_enseignement", "Scolarité"),
        ("ent_eleve_periode", "Scolarité"),
        ("ent_eleve_mobilite", "Scolarité"),
        ("ent_eleve_modalite", "Scolarité"),
        ("ent_eleve_capacite", "Scolarité"),
        ("ent_eleve_majorite", "Scolarité"),
        ("ent_affectation", "Scolarité"),
        ("ent_cours_membre", "Scolarité"),
        ("ent_eleve_compte", "Compte de l'élève"),
        ("ent_eleve_invitation", "Compte de l'élève"),
        ("ent_info_comp", "Informations complémentaires"),
        ("ent_eleve_orphelin", "Journal et événements"),
        ("ent_journal", "Journal et événements"),
        ("ent_metier_event", "Journal et événements"),
        ("ent_contrainte", "Emploi du temps"),
        ("ent_edt_individuelle", "Emploi du temps"),
        ("ent_edt_localisation", "Emploi du temps"),
        ("ent_cours_membre", "Emploi du temps"),
    ];
    REGLES
        .iter()
        .find(|(prefixe, _)| table.starts_with(prefixe))
        .map(|(_, d)| *d)
        .unwrap_or("Autres données")
}

/// Domaine dont le détail ne se montre pas (santé, suivi) : l'aperçu n'en donne qu'un total.
pub fn domaine_protege(table: &str) -> bool {
    domaine_fusion(table) == "Suivi et santé (protégé)"
}

/// La déclaration d'une table du registre.
pub fn table_eleve(nom: &str) -> Option<&'static TableEleve> {
    TABLES_ELEVE.iter().find(|t| t.table == nom)
}

/// Tables dont les lignes passent à l'élève conservé, dans l'ordre du registre.
pub fn tables_a_deplacer() -> impl Iterator<Item = &'static TableEleve> {
    TABLES_ELEVE.iter().filter(|t| t.strategie == Strategie::Deplacer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registre_sans_doublon_et_trie() {
        let noms: Vec<_> = TABLES_ELEVE.iter().map(|t| t.table).collect();
        let uniques: HashSet<_> = noms.iter().collect();
        assert_eq!(noms.len(), uniques.len(), "une table déclarée deux fois");
        let mut tri = noms.clone();
        tri.sort_unstable();
        assert_eq!(noms, tri, "registre à garder en ordre alphabétique");
    }

    #[test]
    fn chaque_table_a_une_cle_et_les_reste_une_raison() {
        for t in TABLES_ELEVE {
            assert!(!t.cle.is_empty(), "{} : clé primaire vide", t.table);
            assert_eq!(
                t.strategie == Strategie::Reste,
                !t.raison.is_empty(),
                "{} : une table qui reste sur l'absorbé doit dire pourquoi (et seulement celles-là)",
                t.table
            );
        }
    }

    #[test]
    fn historiques_immuables_restent_sur_l_absorbe() {
        for t in ["ent_journal_modif", "ent_metier_event", "ent_eleve_identite", "ent_bulletin"] {
            assert_eq!(table_eleve(t).unwrap().strategie, Strategie::Reste, "{t}");
        }
        for t in ["ent_note_archive", "ent_absence", "ent_eleve_inscription", "ent_eleve_identifiant"] {
            assert_eq!(table_eleve(t).unwrap().strategie, Strategie::Deplacer, "{t}");
        }
    }

    #[test]
    fn domaines_lisibles_et_proteges() {
        assert_eq!(domaine_fusion("ent_note_archive"), "Évaluations");
        assert_eq!(domaine_fusion("ent_eleve_inscription"), "Scolarité");
        // Les tables d'admission (campagnes, voeux) sont « Inscriptions » ; l'inscription annuelle de l'élève fait partie de sa scolarité.
        assert_eq!(domaine_fusion("ent_voeu"), "Inscriptions");
        assert!(domaine_protege("ent_psy_entretien"));
        assert!(domaine_protege("ent_pai_signature"));
        assert!(domaine_protege("ent_fiche_sante"));
        assert!(!domaine_protege("ent_absence"));
        // Toute table du registre a un domaine nommé (pas de « Autres données » par oubli).
        let sans: Vec<_> = TABLES_ELEVE.iter().filter(|t| domaine_fusion(t.table) == "Autres données").map(|t| t.table).collect();
        assert!(sans.is_empty(), "tables sans domaine : {sans:?}");
    }

    #[test]
    fn cle_hors_eleve() {
        assert_eq!(table_eleve("ent_travail_eleve").unwrap().cle_hors_eleve(), vec!["travail_id"]);
        assert_eq!(table_eleve("ent_note_archive").unwrap().cle_hors_eleve(), vec!["id"]);
        assert_eq!(
            table_eleve("ent_eleve_compte").unwrap().cle_hors_eleve(),
            vec!["etablissement_id"]
        );
    }
}
