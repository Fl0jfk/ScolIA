//! Événements métier transverses — socle du Brain.
//!
//! Première implémentation : bus **in-memory** pour tests L0.
//! La persistance Postgres (`metier_event`) viendra en L1 sans changer le contrat public.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

/// Types d'événements connus (alignés sur l'amorce ScolIA `eleve-core/events.ts`).
pub mod types {
    pub const ELEVE_CREATED: &str = "eleve.created";
    pub const ELEVE_STATUS_CHANGED: &str = "eleve.status_changed";
    pub const ELEVE_IDENTITY_CHANGED: &str = "eleve.identity_changed";
    /// Identifiant externe ajouté, confirmé, contesté, révoqué (système et statuts, jamais la valeur).
    pub const ELEVE_IDENTIFIANT_CHANGE: &str = "eleve.identifiant_change";
    /// Proposition déposée / décidée (lot E5) : identifiant, cible, source, noms de champs —
    /// jamais une valeur.
    pub const ELEVE_PROPOSITION_DEPOSEE: &str = "eleve.proposition_deposee";
    pub const ELEVE_PROPOSITION_DECIDEE: &str = "eleve.proposition_decidee";
    /// Paramétrage des champs de l'élève modifié (ELV-26) : noms de champs, jamais de valeur.
    pub const PARAMETRAGE_ELEVE_MODIFIE: &str = "parametrage.eleve_modifie";
    /// Fusion de deux fiches d'élèves et son annulation (ELV-23, ELV-24) : identifiants des deux
    /// fiches et de la fusion, jamais une valeur d'identité.
    pub const ELEVE_FUSIONNE: &str = "eleve.fusionne";
    pub const ELEVE_DEFUSIONNE: &str = "eleve.defusionne";
    /// Deux fiches déclarées distinctes (ELV-33) : identifiants et motif codé.
    pub const ELEVE_NON_DOUBLON_DECLARE: &str = "eleve.non_doublon_declare";
    /// Réintégration d'un ancien élève sur sa fiche (ELV-12).
    pub const ELEVE_REINTEGRE: &str = "eleve.reintegre";
    /// Passage à la majorité (ELV-14, tâche nocturne) et sa veille de 30 jours (ELV-39).
    pub const ELEVE_MAJEUR: &str = "eleve.majeur";
    pub const ELEVE_MAJORITE_PROCHE: &str = "eleve.majorite_proche";
    /// Capacité juridique déclarée ou levée (ELV-13) : nature seulement.
    pub const ELEVE_CAPACITE_CHANGEE: &str = "eleve.capacite_changee";
    /// Décès d'un élève (ELV-34, R-E12) : aucun détail dans la charge utile.
    pub const ELEVE_DECEDE: &str = "eleve.decede";
    /// Confidentialité renforcée posée ou levée (ELV-15) : `pose` / `levee` et identifiant de la
    /// mesure — **jamais** le motif ni la liste des mesures.
    pub const ELEVE_CONFIDENTIALITE_CHANGEE: &str = "eleve.confidentialite_changee";
    /// Consultation d'une fiche protégée par une personne sans lien direct, quand la mesure
    /// `alerte_consultation` est posée (lot E7) : identifiant de l'élève seulement ; l'acteur est
    /// l'auteur de l'événement.
    pub const ELEVE_CONSULTATION_PROTEGEE: &str = "eleve.consultation_protegee";
    /// Coordonnées propres de l'élève modifiées (ELV-17) : type, état et **noms** de champs —
    /// jamais l'adresse ni le numéro.
    pub const ELEVE_COORDONNEES_MODIFIEES: &str = "eleve.coordonnees_modifiees";
    pub const SCOLARITE_MOBILITE_OUVERTE: &str = "scolarite.mobilite_ouverte";
    pub const SCOLARITE_MOBILITE_FERMEE: &str = "scolarite.mobilite_fermee";
    pub const SCOLARITE_MODALITE_CHANGEE: &str = "scolarite.modalite_changee";
    /// Une scolarisation s'ouvre (socle 2, lot E4 : émis par ELV-08 et ELV-12) ;
    /// charge utile : `scolarisation_id`, dates, motif codé — jamais de donnée d'identité.
    pub const SCOLARITE_OPENED: &str = "scolarite.opened";
    pub const SCOLARITE_CLASSE_CHANGED: &str = "scolarite.classe_changed";
    pub const SCOLARITE_MEF_CHANGED: &str = "scolarite.mef_changed";
    pub const SCOLARITE_SITE_CHANGED: &str = "scolarite.site_changed";
    /// Une scolarisation se clôt (ELV-11, ELV-34) ; mêmes règles de charge utile.
    pub const SCOLARITE_CLOSED: &str = "scolarite.closed";
    pub const ELEVE_REGIME_CHANGED: &str = "eleve.regime_changed";
    pub const ELEVE_SORTIE: &str = "eleve.sortie";
    /// Réservé : changement de foyer — émission non branchée (règle produit non validée).
    pub const FOYER_CHANGED: &str = "foyer.changed";
    pub const ELEVE_LIEN_UPDATED: &str = "eleve.lien_updated";
    /// Fait d'absence sur l'élève (une vérité pour absences, cantine, appel…).
    pub const ELEVE_ABSENCE_DECLARED: &str = "eleve.absence_declared";
    pub const ELEVE_ABSENCE_JUSTIF_SOUMIS: &str = "eleve.absence_justif_soumis";
    pub const ELEVE_ABSENCE_JUSTIF_RETRAIT: &str = "eleve.absence_justif_retrait";
    pub const ELEVE_ABSENCE_JUSTIFIED: &str = "eleve.absence_justified";
    pub const ELEVE_ABSENCE_REFUSED: &str = "eleve.absence_refused";
    pub const ELEVE_ABSENCE_ANNULEE_PRESENT: &str = "eleve.absence_annulee_present";
    /// Compteur d'absences actives a atteint le seuil établissement (file VS).
    pub const ELEVE_ABSENCE_SEUIL_ATTEINT: &str = "eleve.absence_seuil_atteint";
    pub const ELEVE_ABSENCE_SIGNAL_EN_COURS: &str = "eleve.absence_signal_en_cours";
    pub const ELEVE_ABSENCE_SIGNAL_CLOS: &str = "eleve.absence_signal_clos";
    pub const ABSENCE_MOTIF_ETAB_CREATED: &str = "absence.motif_etab_created";
    pub const ELEVE_COMPTE_LIE: &str = "eleve.compte_lie";
    pub const ELEVE_COMPTE_REVOQUE: &str = "eleve.compte_revoque";
    pub const ELEVE_COMPTE_INVITE: &str = "eleve.compte_invite";
    pub const ELEVE_OPTION_ADDED: &str = "eleve.option_added";
    pub const ELEVE_DOCUMENT_UPLOADED: &str = "eleve.document_uploaded";
    pub const ELEVE_DOCUMENT_GRANT_REQUESTED: &str = "eleve.document_grant_requested";
    pub const ELEVE_DOCUMENT_GRANT_DECIDED: &str = "eleve.document_grant_decided";
    pub const ELEVE_DISPOSITIF_AJOUTE: &str = "eleve.dispositif_ajoute";
    pub const REPAS_ACTIVITE_CREATED: &str = "repas.activite_created";
    pub const ELEVE_GRILLE_REPAS_POSED: &str = "eleve.grille_repas_posed";
    pub const ELEVE_GRILLE_REPAS_CLOSE: &str = "eleve.grille_repas_close";
    pub const ELEVE_DISPENSE_REPAS_POSED: &str = "eleve.dispense_repas_posed";
    pub const ELEVE_DISPENSE_REPAS_ANNULEE: &str = "eleve.dispense_repas_annulee";
    pub const ELEVE_REPAS_PASSAGE: &str = "eleve.repas_passage";
    pub const BOURSE_TYPE_CREATED: &str = "bourse.type_created";
    pub const ELEVE_BOURSE_AFFECTEE: &str = "eleve.bourse_affectee";
    pub const ELEVE_BOURSE_DECIDEE: &str = "eleve.bourse_decidee";
    pub const REINSCRIPTION_CAMPAGNE_OUVERTE: &str = "reinscription.campagne_ouverte";
    pub const ELEVE_SCOLARITE_ANNEE_ENREGISTREE: &str = "eleve.scolarite_annee_enregistree";
    pub const ELEVE_MONTEE_RAPIDE: &str = "eleve.montee_rapide";
    pub const ELEVE_MONTEE_LOT: &str = "eleve.montee_lot";
    pub const ELEVE_OPTION_N1_CHOISIE: &str = "eleve.option_n1_choisie";
    pub const ELEVE_REINSCRIPTION_CONFIRMEE: &str = "eleve.reinscription_confirmee";
    pub const REINSCRIPTION_EXIGENCE_DOC: &str = "reinscription.exigence_doc";
    pub const REINSCRIPTION_ACOMPTE_CONFIG: &str = "reinscription.acompte_config";
    pub const VS_INCIDENT_TYPE_CREATED: &str = "vs.incident_type_created";
    pub const VS_SANCTION_TYPE_CREATED: &str = "vs.sanction_type_created";
    pub const VS_SANCTION_MOTIF_CREATED: &str = "vs.sanction_motif_created";
    pub const ELEVE_INCIDENT_DECLARED: &str = "eleve.incident_declared";
    pub const ELEVE_SANCTION_PRONONCEE: &str = "eleve.sanction_prononcee";
    pub const ELEVE_SANCTION_ANNULEE: &str = "eleve.sanction_annulee";
    pub const MATIERE_CREATED: &str = "matiere.created";
    pub const NOTE_SAISIE: &str = "note.saisie";
    pub const NOTE_APPRECIATION: &str = "note.appreciation";
    pub const APPEL_OPENED: &str = "appel.opened";
    pub const APPEL_CLOSED: &str = "appel.closed";
    pub const ELEVE_CARNET_ENTREE: &str = "eleve.carnet_entree";
    pub const ELEVE_CARNET_SIGNEE: &str = "eleve.carnet_signee";
    pub const EDT_CRENEAU_CREATED: &str = "edt.creneau_created";
    pub const EDT_EXCEPTION: &str = "edt.exception";
    pub const EDT_CALENDRIER_AB: &str = "edt.calendrier_ab";
    pub const CAHIER_TEXTE: &str = "cahier.texte";
    // Cahier de textes et travail (S2-6, D7).
    pub const CAHIER_SEANCE_RENSEIGNEE: &str = "cahier.seance_renseignee";
    pub const CAHIER_TRAVAIL_DONNE: &str = "cahier.travail_donne";
    pub const CAHIER_TRAVAIL_MODIFIE: &str = "cahier.travail_modifie";
    pub const CAHIER_TRAVAIL_SUPPRIME: &str = "cahier.travail_supprime";
    pub const CAHIER_PIECE_JOINTE: &str = "cahier.piece_jointe";
    pub const CAHIER_TRAVAIL_A_RENDRE: &str = "cahier.travail_a_rendre";
    pub const CAHIER_TRAVAIL_RENDU: &str = "cahier.travail_rendu";
    pub const CAHIER_COPIE_RENDUE: &str = "cahier.copie_rendue";
    pub const CAHIER_VISE: &str = "cahier.vise";
    // Cahier de textes, lot C2 (saisie et publication).
    pub const CAHIER_SEANCE_MODIFIEE: &str = "cahier.seance_modifiee";
    pub const CAHIER_RETIREE: &str = "cahier.retiree";
    pub const CAHIER_PROGRAMME: &str = "cahier.programme";
    pub const CAHIER_CONTRIBUTION_AJOUTEE: &str = "cahier.contribution_ajoutee";
    // Lot C3 : résumé hebdomadaire « la semaine de mon enfant » (décomptes seulement).
    pub const CAHIER_RESUME_HEBDOMADAIRE: &str = "cahier.resume_hebdomadaire";
    // Lot C5 : rendu et retour.
    pub const CAHIER_REDEPOT: &str = "cahier.redepot";
    pub const CAHIER_DEPOT_IMPOSSIBLE_SIGNALE: &str = "cahier.depot_impossible_signale";
    pub const CAHIER_DELAI_DEMANDE: &str = "cahier.delai_demande";
    pub const CAHIER_DELAI_ACCORDE: &str = "cahier.delai_accorde";
    pub const CAHIER_DELAI_DECIDE: &str = "cahier.delai_decide";
    pub const CAHIER_STATUT_CONSTATE: &str = "cahier.statut_constate";
    pub const CAHIER_RETOUR_MODIFIE: &str = "cahier.retour_modifie";
    pub const CAHIER_REMISE_ROUVERTE: &str = "cahier.remise_rouverte";
    pub const CAHIER_COPIE_SIGNALEE: &str = "cahier.copie_signalee";
    pub const CAHIER_GROUPES_COMPOSES: &str = "cahier.groupes_composes";
    pub const CAHIER_COPIES_TELECHARGEES: &str = "cahier.copies_telechargees";
    pub const CAHIER_PROLONGATION_PANNE: &str = "cahier.prolongation_panne";
    pub const CAHIER_RELANCE_NON_RENDUS: &str = "cahier.relance_non_rendus";
    // Lot C6 : continuité.
    pub const CAHIER_REPORT_PROPOSE: &str = "cahier.report_propose";
    pub const CAHIER_REPORT_APPLIQUE: &str = "cahier.report_applique";
    pub const CAHIER_TRAVAIL_ABSENCE_DEPOSE: &str = "cahier.travail_absence_depose";
    pub const CAHIER_SAISIE_RENDUE: &str = "cahier.saisie_rendue";
    pub const CAHIER_COPIES_REATTRIBUEES: &str = "cahier.copies_reattribuees";
    pub const CAHIER_CONTINUITE_OUVERTE: &str = "cahier.continuite_ouverte";
    pub const CAHIER_CONTINUITE_CLOSE: &str = "cahier.continuite_close";
    // Lot C7 : adaptations.
    pub const CAHIER_VARIANTE_ATTRIBUEE: &str = "cahier.variante_attribuee";
    pub const CAHIER_NOTE_ACCOMPAGNEMENT: &str = "cahier.note_accompagnement";
    // Lot C8 : visa, inspection, exports figés, clôture et archive.
    pub const CAHIER_OBSERVATION: &str = "cahier.observation";
    pub const CAHIER_OBSERVATION_REPONSE: &str = "cahier.observation_reponse";
    pub const CAHIER_ACCES_INSPECTION_ACCORDE: &str = "cahier.acces_inspection_accorde";
    pub const CAHIER_ACCES_INSPECTION_PROLONGE: &str = "cahier.acces_inspection_prolonge";
    pub const CAHIER_ACCES_INSPECTION_REVOQUE: &str = "cahier.acces_inspection_revoque";
    pub const CAHIER_CONSULTE_INSPECTION: &str = "cahier.consulte_inspection";
    pub const CAHIER_EXPORT_PRODUIT: &str = "cahier.export_produit";
    pub const CAHIER_ANNEE_FIGEE: &str = "cahier.annee_figee";
    pub const CAHIER_ANNEE_ROUVERTE: &str = "cahier.annee_rouverte";
    pub const CAHIER_REOUVERTURE_DEMANDEE: &str = "cahier.reouverture_demandee";
    pub const CAHIER_ARCHIVE_DECIDEE: &str = "cahier.archive_decidee";
    // Lot C9 : progressions.
    pub const CAHIER_PROGRESSION_PLANIFIEE: &str = "cahier.progression_planifiee";
    pub const CAHIER_COURS_COPIE: &str = "cahier.cours_copie";
    pub const NOTE_PERIODE_CREATED: &str = "note.periode_created";
    pub const NOTE_PERIODE_STATUT: &str = "note.periode_statut";
    pub const NOTE_BULLETIN_PUBLIE: &str = "note.bulletin_publie";
    pub const DEVOIR_CREATED: &str = "devoir.created";
    pub const DEVOIR_DELETED: &str = "devoir.deleted";
    pub const MSG_FIL_OUVERT: &str = "msg.fil_ouvert";
    pub const MSG_MODELE: &str = "msg.modele";
    pub const MSG_FIL_ARCHIVE: &str = "msg.fil_archive";
    pub const MSG_FIL_CLASSER: &str = "msg.fil_classer";
    pub const MSG_ENVOYE: &str = "msg.envoye";
    pub const MSG_ACCUSE: &str = "msg.accuse";
    pub const MSG_PIECE: &str = "msg.piece";
    pub const MSG_DEROGATION_PLAGE: &str = "msg.derogation_plage";
    // Messagerie, socle 16 (§22) : noms `messagerie.*`, charges sans contenu ni sujet (R-M14).
    pub const MESSAGERIE_FIL_OUVERT: &str = "messagerie.fil_ouvert";
    pub const MESSAGERIE_MESSAGE_ENVOYE: &str = "messagerie.message_envoye";
    pub const MESSAGERIE_PARTICIPANT_AJOUTE: &str = "messagerie.participant_ajoute";
    pub const MESSAGERIE_PARTICIPANT_RETIRE: &str = "messagerie.participant_retire";
    pub const MESSAGERIE_FIL_DEDOUBLE: &str = "messagerie.fil_dedouble";
    // Lot M3 (boîtes de service) et lot M4 (écrire mieux) : charges sans contenu ni sujet (R-M14).
    pub const MESSAGERIE_BOITE_CREEE: &str = "messagerie.boite_creee";
    pub const MESSAGERIE_BOITE_MODIFIEE: &str = "messagerie.boite_modifiee";
    pub const MESSAGERIE_FIL_PRIS: &str = "messagerie.fil_pris";
    pub const MESSAGERIE_FIL_TRAITE: &str = "messagerie.fil_traite";
    pub const MESSAGERIE_FIL_TRANSFERE: &str = "messagerie.fil_transfere";
    pub const MESSAGERIE_MESSAGE_TEL_SAISI: &str = "messagerie.message_tel_saisi";
    pub const MESSAGERIE_ENTRANT_DEPOSE: &str = "messagerie.entrant_depose";
    pub const MESSAGERIE_LOT_ENVOYE: &str = "messagerie.lot_envoye";
    pub const MESSAGERIE_MESSAGE_CORRIGE: &str = "messagerie.message_corrige";
    pub const MESSAGERIE_MESSAGE_RETIRE: &str = "messagerie.message_retire";
    pub const MESSAGERIE_FILS_FUSIONNES: &str = "messagerie.fils_fusionnes";
    pub const MESSAGERIE_FIL_SCINDE: &str = "messagerie.fil_scinde";
    // Lots M5 (pièces) et M6 (lecture) : charges sans contenu, sans sujet, sans nom (R-M14, R-M16).
    pub const MESSAGERIE_PIECE_ANALYSEE: &str = "messagerie.piece_analysee";
    pub const MESSAGERIE_CARTE_INSEREE: &str = "messagerie.carte_inseree";
    pub const MESSAGERIE_LU_JUSQUA: &str = "messagerie.lu_jusqua";
    pub const ANNONCE_APERCU: &str = "annonce.apercu";
    pub const ANNONCE_RETIREE: &str = "annonce.retiree";
    pub const LIEN_VERIFIE: &str = "lien.verifie";
    pub const MSG_REGLAGE: &str = "msg.reglage";
    pub const FAC_FACTURE_CREEE: &str = "fac.facture_creee";
    pub const FAC_FACTURE_EMISE: &str = "fac.facture_emise";
    pub const FAC_ENCAISSEMENT: &str = "fac.encaissement";
    pub const FAC_FACTURE_SOLDEE: &str = "fac.facture_soldee";
    pub const FAC_ECHEANCIER_POSE: &str = "fac.echeancier_pose";
    pub const FAC_QUITTANCE_EMISE: &str = "fac.quittance_emise";
    pub const FAC_SEPA_MANDAT: &str = "fac.sepa_mandat";
    pub const FAC_SEPA_PRELEVEMENT: &str = "fac.sepa_prelevement";
    pub const FAC_SEPA_EXPORT: &str = "fac.sepa_export";
    pub const FAC_SEPA_EXECUTE: &str = "fac.sepa_execute";
    pub const FAC_SEPA_REJET: &str = "fac.sepa_rejet";
    pub const FAC_RAPPEL_ENVOYE: &str = "fac.rappel_envoye";
    /// F0 (FD-11) : le compte créancier de l'établissement a changé (IBAN toujours masqué).
    pub const FIN_CREDITEUR_MODIFIE: &str = "fin.crediteur_modifie";
    // Finances reliées (socle 2, lot S2-9).
    pub const FAC_TARIF_DEFINI: &str = "fac.tarif_defini";
    pub const FAC_REMISE_APPLIQUEE: &str = "fac.remise_appliquee";
    pub const FAC_GENERATION_ANNUELLE: &str = "fac.generation_annuelle";
    pub const FAC_LIGNE_AJOUTEE: &str = "fac.ligne_ajoutee";
    pub const FAC_AVOIR_EMIS: &str = "fac.avoir_emis";
    pub const FAC_SEPA_PRENOTIFICATION: &str = "fac.sepa_prenotification";
    pub const FAC_REGLEMENT: &str = "fac.reglement";
    pub const FAC_RELANCE_PROPOSEE: &str = "fac.relance_proposee";
    pub const FAC_RELANCE_ANNULEE: &str = "fac.relance_annulee";
    pub const FAC_EXPORT_COMPTABLE: &str = "fac.export_comptable";
    // ── Socle 24, lot F1 : événements canoniques `facturation.*` (R-F25 : aucun montant, aucun
    // nom, aucun IBAN dans la charge ; les identifiants seulement). Les anciens `fac.*` et
    // `fin.*` ci-dessus restent **lisibles** par alias (`alias_facturation`, `equivalents`) pendant
    // la transition : un consommateur déclaré sur l'un reçoit l'autre. ──
    pub const FACTURATION_BROUILLON_CREE: &str = "facturation.brouillon_cree";
    pub const FACTURATION_PIECE_EMISE: &str = "facturation.piece_emise";
    pub const FACTURATION_ENCAISSEMENT_ENREGISTRE: &str = "facturation.encaissement_enregistre";
    pub const FACTURATION_PIECE_SOLDEE: &str = "facturation.piece_soldee";
    pub const FACTURATION_ECHEANCIER_POSE: &str = "facturation.echeancier_pose";
    pub const FACTURATION_QUITTANCE_EMISE: &str = "facturation.quittance_emise";
    pub const FACTURATION_MANDAT_MODIFIE: &str = "facturation.mandat_modifie";
    pub const FACTURATION_PRELEVEMENT_PREPARE: &str = "facturation.prelevement_prepare";
    pub const FACTURATION_PRELEVEMENTS_EXPORTES: &str = "facturation.prelevements_exportes";
    pub const FACTURATION_PRELEVEMENT_EXECUTE: &str = "facturation.prelevement_execute";
    pub const FACTURATION_PRELEVEMENT_REJETE: &str = "facturation.prelevement_rejete";
    pub const FACTURATION_RAPPEL_ENVOYE: &str = "facturation.rappel_envoye";
    pub const FACTURATION_CREDITEUR_MODIFIE: &str = "facturation.crediteur_modifie";
    pub const FACTURATION_TARIF_DEFINI: &str = "facturation.tarif_defini";
    pub const FACTURATION_REDUCTION_ACCORDEE: &str = "facturation.reduction_accordee";
    pub const FACTURATION_GENERATION_ANNUELLE: &str = "facturation.generation_annuelle";
    pub const FACTURATION_LIGNE_AJOUTEE: &str = "facturation.ligne_ajoutee";
    pub const FACTURATION_AVOIR_EMIS: &str = "facturation.avoir_emis";
    pub const FACTURATION_PRENOTIFICATION_EMISE: &str = "facturation.prenotification_emise";
    pub const FACTURATION_REGLEMENT_ENREGISTRE: &str = "facturation.reglement_enregistre";
    pub const FACTURATION_RELANCE_PROPOSEE: &str = "facturation.relance_proposee";
    pub const FACTURATION_RELANCE_ANNULEE: &str = "facturation.relance_annulee";
    pub const FACTURATION_EXPORT_COMPTABLE: &str = "facturation.export_comptable";
    pub const FACTURATION_PARAMETRES_MODIFIES: &str = "facturation.parametres_modifies";
    pub const FACTURATION_ARTICLE_ENREGISTRE: &str = "facturation.article_enregistre";
    pub const FACTURATION_BAREME_BOURSE_POSE: &str = "facturation.bareme_bourse_pose";
    pub const FACTURATION_ECHEANCIER_TYPE_POSE: &str = "facturation.echeancier_type_pose";
    pub const FACTURATION_MODE_PAIEMENT_MODIFIE: &str = "facturation.mode_paiement_modifie";
    // Lot F2 : comptes débiteurs et payeurs.
    pub const FACTURATION_COMPTE_CREE: &str = "facturation.compte_cree";
    pub const FACTURATION_COMPTE_MODIFIE: &str = "facturation.compte_modifie";
    pub const FACTURATION_REPRISE_COMPTES: &str = "facturation.reprise_comptes";
    pub const FACTURATION_CONSEQUENCES_PROPOSEES: &str = "facturation.consequences_proposees";
    pub const FACTURATION_CHANGEMENT_PAYEUR_APPLIQUE: &str = "facturation.changement_payeur_applique";
    pub const FACTURATION_TACHE_PAYEUR: &str = "facturation.tache_payeur";
    pub const FACTURATION_PRISE_EN_CHARGE: &str = "facturation.prise_en_charge";
    pub const FACTURATION_GEL: &str = "facturation.gel";
    pub const FACTURATION_QUOTIENT_DEFINI: &str = "facturation.quotient_defini";
    pub const FACTURATION_TAUX_TVA_DEFINI: &str = "facturation.taux_tva_defini";
    pub const FACTURATION_IBAN_HISTORISE: &str = "facturation.iban_historise";
    // Lot F3 : catalogue qualifié, grilles votées, simulateur.
    pub const FACTURATION_ARTICLE_QUALIFIE: &str = "facturation.article_qualifie";
    pub const FACTURATION_GRILLE_CREEE: &str = "facturation.grille_creee";
    pub const FACTURATION_GRILLE_PROPOSEE: &str = "facturation.grille_proposee";
    pub const FACTURATION_GRILLE_VOTEE: &str = "facturation.grille_votee";
    pub const FACTURATION_GRILLE_EN_VIGUEUR: &str = "facturation.grille_en_vigueur";
    pub const FACTURATION_GRILLE_PUBLIEE: &str = "facturation.grille_publiee";
    pub const FACTURATION_SIMULATION_CALCULEE: &str = "facturation.simulation_calculee";
    pub const INFIRMERIE_PASSAGE: &str = "infirmerie.passage";
    pub const INFIRMERIE_FICHE_SANTE: &str = "infirmerie.fiche_sante";
    pub const INFIRMERIE_FICHE_SANTE_PDF: &str = "infirmerie.fiche_sante_pdf";
    pub const INFIRMERIE_PAI: &str = "infirmerie.pai";
    pub const INFIRMERIE_PAI_SIGNE: &str = "infirmerie.pai_signe";
    pub const INFIRMERIE_PAI_PIECE: &str = "infirmerie.pai_piece";
    pub const PSY_ENTRETIEN: &str = "psy.entretien";
    pub const PSY_RDV: &str = "psy.rdv";
    pub const PSY_DOSSIER: &str = "psy.dossier";
    pub const PSY_COURRIER: &str = "psy.courrier";
    pub const VS_COMMISSION: &str = "vs.commission";
    pub const VS_COMMISSION_CONVOCATION: &str = "vs.commission_convocation";
    pub const VS_COMMISSION_DECISION: &str = "vs.commission_decision";
    pub const VS_SIGNAL_COMMISSION: &str = "vs.signal_commission";
    /// Mail SMTP sortant (carnet, seuil, courrier psy…).
    pub const MAIL_ENVOYE: &str = "mail.envoye";
    /// SMS sortant (sanctions collège, carnet…).
    pub const SMS_ENVOYE: &str = "sms.envoye";
    /// Notification push (apps natives / web).
    pub const PUSH_ENVOYE: &str = "push.envoye";
    pub const PUSH_DEVICE_REGISTERED: &str = "push.device_registered";
    pub const BANK_RAPPROCHEMENT: &str = "bank.rapprochement";
    pub const OBJECT_STORED: &str = "object.stored";
    pub const EDT_IMPORT: &str = "edt.import";
    pub const SORTIE_AUTORISATION: &str = "sortie.autorisation";
    pub const SORTIE_DEMANDE: &str = "sortie.demande";
    pub const SORTIE_DECISION: &str = "sortie.decision";
    pub const SORTIE_CONTROLE: &str = "sortie.controle";
    pub const INTERNAT_CHAMBRE: &str = "internat.chambre";
    pub const INTERNAT_AFFECTATION: &str = "internat.affectation";
    pub const INTERNAT_PRESENCE: &str = "internat.presence";
    pub const INTERNAT_SUIVI: &str = "internat.suivi";
    /// Zone d'internat créée, modifiée, fermée ou rouverte (socle 41, I1).
    pub const INTERNAT_ZONE: &str = "internat.zone";
    pub const INTERNAT_LIT: &str = "internat.lit";
    /// Séjour d'hébergement ouvert ou clos par le régime (R-IT1) ; rythme versionné (R-IT4).
    pub const INTERNAT_SEJOUR: &str = "internat.sejour";
    pub const INTERNAT_RYTHME: &str = "internat.rythme";
    /// Dérogation de la direction (public de la chambre, interdiction de contact).
    pub const INTERNAT_DEROGATION: &str = "internat.derogation";
    /// Effet d'un changement de régime sur l'hébergement : affectation close, avis au responsable.
    pub const INTERNAT_REGIME_EFFET: &str = "internat.regime_effet";
    pub const INTERNAT_PARAMETRES: &str = "internat.parametres";
    // Socle 41, lot I2 (la nuit). Charges NEUTRES : identifiants, dates, codes et compteurs ; jamais
    // de nom, de motif en clair ni de donnée de santé (R-IT6).
    pub const INTERNAT_NUIT_EXCEPTIONNELLE: &str = "internat.nuit_exceptionnelle";
    pub const INTERNAT_ATTENDU_MODIFIE: &str = "internat.attendu_modifie";
    pub const INTERNAT_APPEL_OUVERT: &str = "internat.appel_ouvert";
    pub const INTERNAT_APPEL_CLOS: &str = "internat.appel_clos";
    pub const INTERNAT_APPEL_VALIDE: &str = "internat.appel_valide";
    pub const INTERNAT_APPEL_CORRIGE: &str = "internat.appel_corrige";
    pub const INTERNAT_APPEL_EN_RETARD: &str = "internat.appel_en_retard";
    pub const INTERNAT_PRESENCE_POINTEE: &str = "internat.presence_pointee";
    pub const INTERNAT_CAS_OUVERT: &str = "internat.cas_ouvert";
    pub const INTERNAT_CAS_ETAPE: &str = "internat.cas_etape";
    pub const INTERNAT_CAS_CLOS: &str = "internat.cas_clos";
    pub const INTERNAT_ABSENCE_NUIT: &str = "internat.absence_nuit";
    pub const INTERNAT_TRANSMISSION: &str = "internat.transmission";
    pub const INTERNAT_EFFECTIF_REPAS: &str = "internat.effectif_repas";
    // Socle 41, lot I3 : absences et sorties. Charges : identifiants, dates, codes ; jamais un nom, un
    // texte libre ni une donnée de santé.
    pub const INTERNAT_ABSENCE_ANNONCEE: &str = "internat.absence_annoncee";
    pub const INTERNAT_ABSENCE_DECIDEE: &str = "internat.absence_decidee";
    pub const INTERNAT_ABSENCE_REGULARISEE: &str = "internat.absence_regularisee";
    pub const INTERNAT_ABSENCE_BLOQUEE: &str = "internat.absence_bloquee";
    pub const INTERNAT_SORTIE_DEMANDEE: &str = "internat.sortie_demandee";
    pub const INTERNAT_SORTIE_DECIDEE: &str = "internat.sortie_decidee";
    pub const INTERNAT_SORTIE_PARTIE: &str = "internat.sortie_partie";
    pub const INTERNAT_SORTIE_RENTREE: &str = "internat.sortie_rentree";
    pub const INTERNAT_RETARD_ANNONCE: &str = "internat.retard_annonce";
    pub const INTERNAT_DEPART_PREVU: &str = "internat.depart_prevu";
    pub const INTERNAT_AUTORISATIONS_DEMANDEES: &str = "internat.autorisations_demandees";
    pub const INTERNAT_AUTORISATION_REVOQUEE: &str = "internat.autorisation_revoquee";
    pub const INTERNAT_CORRESPONDANT: &str = "internat.correspondant";
    /// Tiers autorisé à récupérer l'élève : posé, contesté, tranché, révoqué, repris, bientôt échu. Identifiants et action
    /// seulement : jamais le motif, ni le demandeur, ni l'opposant. Destinataires : tous les titulaires (socle 8).
    pub const INTERNAT_TIERS: &str = "internat.tiers";
    pub const INTERNAT_DESACCORD_TRANCHE: &str = "internat.desaccord_tranche";
    pub const INTERNAT_MAJEUR_INFORME: &str = "internat.majeur_informe";
    // Socle 41, lot I4 : vie quotidienne. Mêmes règles : identifiants, dates, codes ; jamais de texte libre, de nom
    // ni de donnée de santé. Les événements vers les socles 13, 34, 36 et 44 sont émis par le 41 sous son nom ;
    // les socles destinataires les consomment (leurs lots ne sont pas encore livrés).
    pub const INTERNAT_SORTIE_GROUPE_CREEE: &str = "internat.sortie_groupe_creee";
    pub const INTERNAT_SORTIE_GROUPE_DECIDEE: &str = "internat.sortie_groupe_decidee";
    pub const INTERNAT_SORTIE_GROUPE_ANNULEE: &str = "internat.sortie_groupe_annulee";
    pub const INTERNAT_SORTIE_GROUPE_CLOSE: &str = "internat.sortie_groupe_close";
    pub const INTERNAT_SORTIE_GROUPE_AUTORISATION: &str = "internat.sortie_groupe_autorisation";
    pub const INTERNAT_SORTIE_GROUPE_PRESENCE: &str = "internat.sortie_groupe_presence";
    pub const INTERNAT_LIAISON_ECRITE: &str = "internat.liaison_ecrite";
    pub const INTERNAT_LIAISON_MASQUEE: &str = "internat.liaison_masquee";
    pub const INTERNAT_SIGNAL_PP: &str = "internat.signal_pp";
    pub const INTERNAT_EVENEMENT_DECLARE: &str = "internat.evenement_declare";
    pub const INTERNAT_EVENEMENT_COMPLETE: &str = "internat.evenement_complete";
    pub const INTERNAT_EVENEMENT_TRANSMIS: &str = "internat.evenement_transmis";
    pub const INTERNAT_INTERVENTION_DEMANDEE: &str = "internat.intervention_demandee";
    pub const INTERNAT_MALAISE_SIGNALE: &str = "internat.malaise_signale";
    pub const INTERNAT_ALERTE: &str = "internat.alerte";
    pub const INTERNAT_PREOCCUPATION_SIGNALEE: &str = "internat.preoccupation_signalee";
    pub const INTERNAT_VIGILANCE: &str = "internat.vigilance";
    pub const INTERNAT_TEMPS_CREE: &str = "internat.temps_cree";
    pub const INTERNAT_TEMPS_ANNULE: &str = "internat.temps_annule";
    pub const INTERNAT_TEMPS_INSCRIPTION: &str = "internat.temps_inscription";
    pub const INTERNAT_ETUDE_NON_POINTEE: &str = "internat.etude_non_pointee";
    pub const DISPENSE_VS: &str = "vs.dispense";
    pub const DISPENSE_VS_CLOTURE: &str = "vs.dispense_cloture";
    pub const JOURNAL_ENTREE: &str = "journal.entree";
    pub const ALERTE_CPE: &str = "vs.alerte_cpe";
    pub const ALERTE_CPE_CLOTURE: &str = "vs.alerte_cpe_cloture";
    pub const DOC_A_SIGNER: &str = "doc.a_signer";
    pub const DOC_SIGNE: &str = "doc.signe";
    pub const CONSEIL_CLASSE: &str = "notes.conseil_classe";
    pub const CONSEIL_DOSSIER: &str = "notes.conseil_dossier";
    pub const INFO_COMP_CHAMP: &str = "admin.info_comp_champ";
    pub const INFO_COMP_VALEUR: &str = "admin.info_comp_valeur";
    pub const VACANCES: &str = "etab.vacances";
    /// Réservé (audit) : recherche de doublons d'homonymes — émission non branchée.
    pub const HOMONYME_RECHERCHE: &str = "eleve.homonyme_recherche";
    pub const MENU_CANTINE: &str = "repas.menu";
    pub const QUOTIENT_FOYER: &str = "fac.quotient";
    pub const TAUX_TVA: &str = "fac.taux_tva";
    pub const PHOTO_ELEVE: &str = "eleve.photo";
    pub const PARAM_ETAB: &str = "etab.param";
    pub const SALLE: &str = "etab.salle";
    pub const FILIERE: &str = "ins.filiere";
    pub const VOEU: &str = "ins.voeu";
    pub const INSCRIPTION_SUIVI: &str = "ins.suivi";
    pub const IBAN_HISTO: &str = "fac.iban_histo";
    pub const TARIF_CONSO: &str = "repas.tarif_conso";
    pub const ABSENCE_RH: &str = "rh.absence";
    pub const ABSENCE_RH_VALIDEE: &str = "rh.absence_validee";
    pub const EDT_AFFECTATION: &str = "edt.affectation";
    pub const EDITION_VS: &str = "vs.edition";
    pub const ENTREPRISE: &str = "stage.entreprise";
    pub const CONVENTION_STAGE: &str = "stage.convention";
    /// Socle 39, ST0 : changement de statut d'un stage déclaré (identifiants, états, version).
    pub const CONVENTION_STAGE_STATUT: &str = "stage.convention_statut";
    pub const EDT_CONTRAINTE: &str = "edt.contrainte";
    pub const ANNONCE_PUBLIEE: &str = "annonce.publiee";
    /// S2-1 (D-020) : personne (responsable, contact) distincte du compte.
    pub const PERSONNE_CREEE: &str = "personne.creee";
    /// Socle 6 (personnels), lot P1 : agents et engagements. Charges utiles : identifiants, codes,
    /// dates, noms de champs — jamais de nom, de coordonnée ni de motif détaillé.
    pub const PERSONNEL_CREE: &str = "personnel.cree";
    pub const PERSONNEL_ENGAGEMENT_AJOUTE: &str = "personnel.engagement_ajoute";
    pub const PERSONNEL_ENGAGEMENT_MODIFIE: &str = "personnel.engagement_modifie";
    pub const PERSONNEL_ENGAGEMENT_SUSPENDU: &str = "personnel.engagement_suspendu";
    pub const PERSONNEL_ENGAGEMENT_REPRIS: &str = "personnel.engagement_repris";
    pub const PERSONNEL_FONCTIONS_MODIFIEES: &str = "personnel.fonctions_modifiees";
    pub const PERSONNEL_DISCIPLINES_MODIFIEES: &str = "personnel.disciplines_modifiees";
    pub const PERSONNEL_QUALIFICATION_MODIFIEE: &str = "personnel.qualification_modifiee";
    pub const PERSONNEL_PHOTO_MODIFIEE: &str = "personnel.photo_modifiee";
    pub const PERSONNEL_IDENTITE_RESTREINTE: &str = "personnel.identite_restreinte";
    pub const PERSONNEL_FUSIONNE: &str = "personnel.fusionne";
    /// Socle 6, lot P2 : invitations, rattachement d'identité, arrivée, casquettes. Charges
    /// utiles : identifiants et codes — jamais de jeton, d'adresse ni de nom.
    pub const PERSONNEL_INVITE: &str = "personnel.invite";
    pub const PERSONNEL_INVITATIONS_LOT: &str = "personnel.invitations_lot";
    pub const PERSONNEL_ACCES_RATTACHE: &str = "personnel.acces_rattache";
    pub const PERSONNEL_ARRIVE: &str = "personnel.arrive";
    pub const SESSION_CASQUETTE_CHANGEE: &str = "session.casquette_changee";
    /// Socle 6, lot P3 : services d'enseignement et interventions (agrégat `Referentiel`, identifiant
    /// = service). Charges utiles : identifiants, codes, dates — jamais de nom, de motif détaillé.
    pub const SERVICE_CREE: &str = "service.cree";
    pub const SERVICE_MODIFIE: &str = "service.modifie";
    pub const SERVICE_CLOS: &str = "service.clos";
    pub const SERVICE_ANNULE: &str = "service.annule";
    pub const SERVICE_REMPLACE: &str = "service.remplace";
    pub const SERVICE_INTERVENANT_AJOUTE: &str = "service.intervenant_ajoute";
    pub const SERVICE_INTERVENANT_TERMINE: &str = "service.intervenant_termine";
    pub const SERVICE_INTERVENTION_CORRIGEE: &str = "service.intervention_corrigee";
    pub const SERVICE_SUPPLEANCE_OUVERTE: &str = "service.suppleance_ouverte";
    pub const SERVICE_SUPPLEANCE_CLOSE: &str = "service.suppleance_close";
    pub const SERVICE_OUVERT: &str = "service.ouvert";
    pub const SERVICE_VACANT: &str = "service.vacant";
    pub const SERVICE_POURVU: &str = "service.pourvu";
    /// Socle 6, lot P4 : missions (agrégat `Referentiel`, identifiant = mission). Identifiants, codes, dates.
    pub const MISSION_ATTRIBUEE: &str = "mission.attribuee";
    pub const MISSION_TERMINEE: &str = "mission.terminee";
    pub const MISSION_ANNULEE: &str = "mission.annulee";
    pub const MISSION_TYPE_CREE: &str = "mission.type_cree";
    /// Socle 6, lot P5 : responsables de groupes (identifiant = groupe) et équipes constituées (identifiant = équipe).
    pub const GROUPE_RESPONSABLE_AJOUTE: &str = "groupe.responsable_ajoute";
    pub const GROUPE_RESPONSABLE_TERMINE: &str = "groupe.responsable_termine";
    pub const EQUIPE_CREEE: &str = "equipe.creee";
    pub const EQUIPE_MEMBRES_MODIFIES: &str = "equipe.membres_modifies";
    pub const EQUIPE_CLOSE: &str = "equipe.close";
    /// Socle 6, lot P6 : pouvoirs de signature, délégations (agrégat `Personne`, identifiant = délégation ou habilitation).
    pub const SIGNATURE_HABILITATION_MODIFIEE: &str = "signature.habilitation_modifiee";
    pub const DELEGATION_EXPIRATION_PROCHE: &str = "delegation.expiration_proche";
    /// Socle 5 (R-S23, R-P26) : noms que le socle 6 consomme (alignés, pas ceux de la première
    /// conception `annee.ouverte`, `classe.archivee`…). Aucun consommateur dans le lot P1.
    pub const REFERENTIEL_ANNEE_OUVERTE: &str = "referentiel.annee_ouverte";
    pub const REFERENTIEL_ANNEE_CLOTUREE: &str = "referentiel.annee_cloturee";
    pub const REFERENTIEL_CLASSE_ARCHIVEE: &str = "referentiel.classe_archivee";
    pub const REFERENTIEL_GROUPE_FERME: &str = "referentiel.groupe_ferme";
    pub const PERSONNE_COORDONNEES_MODIFIEES: &str = "personne.coordonnees_modifiees";
    pub const PERSONNE_COMPTE_RATTACHE: &str = "personne.compte_rattache";
    pub const PERSONNE_FUSIONNEE: &str = "personne.fusionnee";
    pub const PERSONNE_DEFUSIONNEE: &str = "personne.defusionnee";
    /// Socle 4, lot A2 : identité, préférences et consentements d'une personne (charges utiles :
    /// identifiants, codes, noms de champs ; jamais de valeur).
    pub const PERSONNE_IDENTITE_MODIFIEE: &str = "personne.identite_modifiee";
    pub const PERSONNE_PREFERENCES_MODIFIEES: &str = "personne.preferences_modifiees";
    pub const PERSONNE_CONSENTEMENT_MODIFIE: &str = "personne.consentement_modifie";
    /// S2-1 (B7) : lien de responsabilité suspendu / rétabli (décision de justice).
    pub const LIEN_SUSPENDU: &str = "lien.suspendu";
    pub const LIEN_RETABLI: &str = "lien.retabli";
    pub const ELEVE_CONTACTS_URGENCE_CHANGES: &str = "eleve.contacts_urgence_changes";
    pub const FOYER_PAYEUR_DEFINI: &str = "foyer.payeur_defini";
    pub const RETARD_DECLARE: &str = "vs.retard";
    pub const COMPETENCE_EVALUEE: &str = "note.competence";
    /// S2-3 (D3) : campagnes, préinscriptions, instruction, décision, validation.
    pub const ADMISSION_CAMPAGNE_OUVERTE: &str = "admission.campagne_ouverte";
    pub const ADMISSION_CAMPAGNE_FERMEE: &str = "admission.campagne_fermee";
    pub const ADMISSION_DEPOSEE: &str = "admission.deposee";
    pub const ADMISSION_ETAPE_CHANGEE: &str = "admission.etape_changee";
    pub const ADMISSION_DECIDEE: &str = "admission.decidee";
    pub const ADMISSION_RELANCE_ENVOYEE: &str = "admission.relance_envoyee";
    /// Fait central de la toile (§4.3) : affectation, invitations, accueil…
    pub const INSCRIPTION_VALIDEE: &str = "inscription.validee";
    /// Lot I2 (socle 3) : cycle de vie des dossiers, offre, échéances. Charges utiles : identifiants,
    /// codes, dates ; jamais de nom, d'adresse, d'e-mail ni de motif rédigé.
    pub const ADMISSION_RETIREE: &str = "admission.retiree";
    pub const INSCRIPTION_DESISTEMENT: &str = "inscription.desistement";
    pub const INSCRIPTION_BLOQUE_DESACCORD: &str = "inscription.bloque_desaccord";
    pub const INSCRIPTION_BLOCAGE_LEVE: &str = "inscription.blocage_leve";
    /// Lots I3 et I4 (socle 3) : pièces d'un dossier et accords des responsables. Charges utiles :
    /// identifiants et codes de type de pièce seulement (jamais de nom, d'adresse ni de motif rédigé). Le moteur de
    /// notifications du socle 8 prévient les familles qui ont un élève et des liens ; la famille d'une admission,
    /// sans compte, est prévenue par la file d'envoi des inscriptions.
    pub const INSCRIPTION_PIECE_DEMANDEE: &str = "inscription.piece_demandee";
    pub const INSCRIPTION_PIECE_DEPOSEE: &str = "inscription.piece_deposee";
    pub const INSCRIPTION_PIECE_CONTROLEE: &str = "inscription.piece_controlee";
    pub const INSCRIPTION_PIECES_PURGEES: &str = "inscription.pieces_purgees";
    pub const INSCRIPTION_ACCORD_DEMANDE: &str = "inscription.accord_demande";
    pub const INSCRIPTION_ACCORD_REPONDU: &str = "inscription.accord_repondu";
    pub const INSCRIPTION_VALIDATION_ANNULEE: &str = "inscription.validation_annulee";
    pub const INSCRIPTION_SOLLICITATION_SUSPENDUE: &str = "inscription.sollicitation_suspendue";
    pub const INSCRIPTION_CAMPAGNE_MODIFIEE: &str = "inscription.campagne_modifiee";
    pub const INSCRIPTION_OFFRE_MODIFIEE: &str = "inscription.offre_modifiee";
    pub const INSCRIPTION_ECHEANCE_PROLONGEE: &str = "inscription.echeance_prolongee";
    pub const INSCRIPTION_FERMETURE_DECLAREE: &str = "inscription.fermeture_declaree";
    pub const INSCRIPTION_DOSSIER_CLOS: &str = "inscription.dossier_clos";
    /// Socle 3, lot I5 : décisions publiées à la famille, place proposée, réponse de la famille, séance de
    /// commission clôturée, déport d'un membre du personnel (aucune donnée personnelle dans la charge utile).
    pub const INSCRIPTION_DECISIONS_PUBLIEES: &str = "inscription.decisions_publiees";
    pub const INSCRIPTION_PLACE_PROPOSEE: &str = "inscription.place_proposee";
    pub const INSCRIPTION_REPONSE_FAMILLE: &str = "inscription.reponse_famille";
    pub const INSCRIPTION_COMMISSION_TENUE: &str = "inscription.commission_tenue";
    pub const INSCRIPTION_DEPORT: &str = "inscription.deport";
    /// Socle 2 : création d'élève annulée (ELV-02, commande appelée par INS-68).
    pub const ELEVE_CREATION_ANNULEE: &str = "eleve.creation_annulee";
    pub const SCOLARITE_DEPART_ANNONCE: &str = "scolarite.depart_annonce";
    pub const RESPONSABLE_INVITE: &str = "inscription.responsable_invite";
    pub const INVITATION_ACCEPTEE: &str = "inscription.invitation_acceptee";
    pub const MANDAT_SEPA_A_SIGNER: &str = "fac.sepa_mandat_a_signer";
    /// S2-3 (D23) : import en lot, rejets traités.
    pub const IMPORT_LOT_APPLIQUE: &str = "import.lot_applique";
    pub const IMPORT_REJET_TRAITE: &str = "import.rejet_traite";
    /// Socle 28 (MS0) : lot d'import prévisualisé annulé.
    pub const IMPORT_LOT_ANNULE: &str = "import.lot_annule";
    /// Socle N1 (demandes) : événements du cœur. Charge utile : identifiants, numéro et codes SEULEMENT
    /// (« enveloppe neutre », R-DM27, NJ-86) — jamais l'objet, le texte, un nom ni une adresse.
    pub const DEMANDE_DEPOSEE: &str = "demande.deposee";
    pub const DEMANDE_PRISE: &str = "demande.prise";
    pub const DEMANDE_RELACHEE: &str = "demande.relachee";
    pub const DEMANDE_CLOSE: &str = "demande.close";
    pub const DEMANDE_PURGEE: &str = "demande.purgee";
    /// Lot N1-2 : mêmes règles (enveloppe neutre). La première ouverture par le service (R-DM53), les échanges, la
    /// réaffectation, le transfert, la requalification, la consultation, le retard et la réouverture.
    pub const DEMANDE_VUE: &str = "demande.vue";
    pub const DEMANDE_ECHANGE: &str = "demande.echange";
    pub const DEMANDE_REAFFECTEE: &str = "demande.reaffectee";
    pub const DEMANDE_TRANSFEREE: &str = "demande.transferee";
    pub const DEMANDE_REQUALIFIEE: &str = "demande.requalifiee";
    pub const DEMANDE_CONSULTEE: &str = "demande.consultee";
    pub const DEMANDE_EN_RETARD: &str = "demande.en_retard";
    pub const DEMANDE_ROUVERTE: &str = "demande.rouverte";
    /// Lot N1-3 : HSE (décision, heures transmises, rien déclaré) et reprographie (prête, rappel de retrait). Mêmes
    /// règles : enveloppe neutre, aucun nom, aucun volume, aucun texte.
    pub const DEMANDE_HSE_DECIDEE: &str = "demande.hse_decidee";
    pub const DEMANDE_HSE_TRANSMISE: &str = "demande.hse_transmise";
    pub const DEMANDE_HSE_RIEN_DECLARE: &str = "demande.hse_rien_declare";
    pub const DEMANDE_REPRO_VALIDEE: &str = "demande.repro_validee";
    pub const DEMANDE_REPRO_PRETE: &str = "demande.repro_prete";
    pub const DEMANDE_REPRO_RAPPEL: &str = "demande.repro_rappel";
}

/// Socle 24, lot F1 : correspondance (ancien nom, nom canonique) des événements financiers.
/// Les nouveaux événements sont écrits sous le nom canonique `facturation.*` ; les anciens noms
/// `fac.*` / `fin.*` restent **lisibles** : un consommateur déclaré sur l'un reçoit l'autre
/// (voir `ent_db::outbox`, `Distributeur::enregistrer`).
pub const ALIAS_FACTURATION: &[(&str, &str)] = &[
    ("fac.facture_creee", types::FACTURATION_BROUILLON_CREE),
    ("fac.facture_emise", types::FACTURATION_PIECE_EMISE),
    ("fac.encaissement", types::FACTURATION_ENCAISSEMENT_ENREGISTRE),
    ("fac.facture_soldee", types::FACTURATION_PIECE_SOLDEE),
    ("fac.echeancier_pose", types::FACTURATION_ECHEANCIER_POSE),
    ("fac.quittance_emise", types::FACTURATION_QUITTANCE_EMISE),
    ("fac.sepa_mandat", types::FACTURATION_MANDAT_MODIFIE),
    ("fac.sepa_prelevement", types::FACTURATION_PRELEVEMENT_PREPARE),
    ("fac.sepa_export", types::FACTURATION_PRELEVEMENTS_EXPORTES),
    ("fac.sepa_execute", types::FACTURATION_PRELEVEMENT_EXECUTE),
    ("fac.sepa_rejet", types::FACTURATION_PRELEVEMENT_REJETE),
    ("fac.rappel_envoye", types::FACTURATION_RAPPEL_ENVOYE),
    ("fin.crediteur_modifie", types::FACTURATION_CREDITEUR_MODIFIE),
    ("fac.tarif_defini", types::FACTURATION_TARIF_DEFINI),
    ("fac.remise_appliquee", types::FACTURATION_REDUCTION_ACCORDEE),
    ("fac.generation_annuelle", types::FACTURATION_GENERATION_ANNUELLE),
    ("fac.ligne_ajoutee", types::FACTURATION_LIGNE_AJOUTEE),
    ("fac.avoir_emis", types::FACTURATION_AVOIR_EMIS),
    ("fac.sepa_prenotification", types::FACTURATION_PRENOTIFICATION_EMISE),
    ("fac.reglement", types::FACTURATION_REGLEMENT_ENREGISTRE),
    ("fac.relance_proposee", types::FACTURATION_RELANCE_PROPOSEE),
    ("fac.relance_annulee", types::FACTURATION_RELANCE_ANNULEE),
    ("fac.export_comptable", types::FACTURATION_EXPORT_COMPTABLE),
    ("fac.quotient", types::FACTURATION_QUOTIENT_DEFINI),
    ("fac.taux_tva", types::FACTURATION_TAUX_TVA_DEFINI),
    ("fac.iban_histo", types::FACTURATION_IBAN_HISTORISE),
];

/// Nom canonique d'un type financier : l'ancien nom `fac.*` / `fin.*` donne le nom
/// `facturation.*` ; tout autre type est rendu tel quel.
pub fn canonique_facturation(type_name: &str) -> &str {
    ALIAS_FACTURATION
        .iter()
        .find(|(ancien, _)| *ancien == type_name)
        .map(|(_, canon)| *canon)
        .unwrap_or(type_name)
}

/// Le type et son alias (ancien ou canonique) : ce qu'un consommateur doit écouter pour ne perdre
/// aucun événement pendant la transition. Un type sans alias n'a qu'un élément.
pub fn equivalents_facturation(type_name: &str) -> Vec<&str> {
    let mut v = vec![type_name];
    for (ancien, canon) in ALIAS_FACTURATION {
        if *ancien == type_name {
            v.push(canon);
        } else if *canon == type_name {
            v.push(ancien);
        }
    }
    v
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateKind {
    Eleve,
    Scolarite,
    Foyer,
    Regime,
    Appel,
    Absence,
    Document,
    Repas,
    Bourse,
    Continuum,
    Incident,
    Sanction,
    Carnet,
    Edt,
    CahierTexte,
    NotePeriode,
    Devoir,
    Matiere,
    Note,
    MsgFil,
    Facture,
    Infirmerie,
    Psy,
    Commission,
    Mail,
    Sms,
    Push,
    Bank,
    Object,
    Sortie,
    Internat,
    DispenseVs,
    Journal,
    AlerteCpe,
    DocASigner,
    ConseilClasse,
    InfoComp,
    Vacances,
    MenuCantine,
    Quotient,
    Tva,
    Trombi,
    ParamEtab,
    Salle,
    Filiere,
    Voeu,
    InscriptionSuivi,
    IbanHisto,
    TarifConso,
    AbsenceRh,
    EditionVs,
    Entreprise,
    ConventionStage,
    EdtContrainte,
    Annonce,
    Retard,
    Competence,
    /// Référentiel scolaire (S2-2) : années, classes, périodes, calendrier, séances.
    Referentiel,
    Personne,
    /// Agent d'un établissement et ses engagements (socle 6, lot P1).
    Agent,
    /// Dossier d'admission / de réinscription et campagnes (S2-3).
    Admission,
    /// Lot d'import (S2-3, D23).
    Import,
    /// Sortie ou voyage scolaire collectif (socle 42, lot V0a).
    Voyage,
    /// Événement ou réunion de l'agenda (socle 19, lot AG1). Charge utile : identifiants seulement.
    AgendaEvenement,
    /// Réservation de lieu (socle 19, lot AG3). Charge utile : identifiants seulement.
    AgendaReservation,
    /// Stages : paramètres, périodes, sites, contacts (socle 39, lot ST1). Les stages eux-mêmes
    /// et les organismes gardent `ConventionStage` et `Entreprise`.
    Stage,
    /// Demande du socle N1 (cœur : dépôt, prise, clôture). Charge utile : enveloppe neutre.
    Demande,
    /// Document émis du socle 7 (brique `documents`) : émission, remplacement, annulation, ouverture. Charge utile :
    /// identifiants, nature et sensibilité seulement, jamais de titre ni de contenu.
    Documents,
    /// Espace de travail du socle 32 (lots ET0–ET1) : création, membres, éléments, versions, corbeille, partages.
    /// Charge utile : identifiants et codes seulement, jamais un nom de fichier ni de dossier (E14).
    Espace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetierEvent {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub type_name: String,
    pub aggregate: AggregateKind,
    pub aggregate_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub payload: Value,
    pub actor_user_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MetierEventDraft {
    pub etablissement_id: Uuid,
    pub type_name: String,
    pub aggregate: AggregateKind,
    pub aggregate_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub payload: Value,
    pub actor_user_id: Option<String>,
}

impl MetierEventDraft {
    pub fn into_event(self) -> MetierEvent {
        MetierEvent {
            id: Uuid::new_v4(),
            etablissement_id: self.etablissement_id,
            type_name: self.type_name,
            aggregate: self.aggregate,
            aggregate_id: self.aggregate_id,
            eleve_id: self.eleve_id,
            payload: self.payload,
            actor_user_id: self.actor_user_id,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EventBusError {
    #[error("etablissement_id manquant ou nil")]
    MissingEtablissement,
    #[error("type d'événement vide")]
    EmptyType,
    /// Panne du magasin d'événements (base) : jamais confondue avec une erreur de saisie.
    #[error("erreur d'infrastructure du bus d'événements")]
    Infrastructure,
    /// Chaîne de réactions trop profonde (garde-fou contre les boucles de la toile).
    #[error("profondeur de causalité dépassée")]
    ProfondeurDepassee,
}

/// Handler Brain : réagit à un événement (répercussions métier).
pub trait EventHandler: Send + Sync {
    fn name(&self) -> &str;
    fn on_event(&self, event: &MetierEvent) -> Result<(), EventBusError>;
}

pub trait EventBus: Send + Sync {
    fn publish(&self, draft: MetierEventDraft) -> Result<MetierEvent, EventBusError>;
    fn events_for_eleve(&self, etablissement_id: Uuid, eleve_id: Uuid) -> Vec<MetierEvent>;
    fn all_events(&self) -> Vec<MetierEvent>;
}

/// Bus mémoire + fan-out vers handlers enregistrés.
#[derive(Default)]
pub struct InMemoryEventBus {
    inner: Mutex<Vec<MetierEvent>>,
    handlers: Mutex<Vec<Arc<dyn EventHandler>>>,
}

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        self.handlers
            .lock()
            .expect("event bus handlers lock")
            .push(handler);
    }
}

impl EventBus for InMemoryEventBus {
    fn publish(&self, draft: MetierEventDraft) -> Result<MetierEvent, EventBusError> {
        if draft.etablissement_id.is_nil() {
            return Err(EventBusError::MissingEtablissement);
        }
        if draft.type_name.trim().is_empty() {
            return Err(EventBusError::EmptyType);
        }
        let event = draft.into_event();
        {
            let mut store = self.inner.lock().expect("event bus store lock");
            store.push(event.clone());
        }
        let handlers = self.handlers.lock().expect("event bus handlers lock").clone();
        for handler in handlers {
            handler.on_event(&event)?;
        }
        Ok(event)
    }

    fn events_for_eleve(&self, etablissement_id: Uuid, eleve_id: Uuid) -> Vec<MetierEvent> {
        self.inner
            .lock()
            .expect("event bus store lock")
            .iter()
            .filter(|e| e.etablissement_id == etablissement_id && e.eleve_id == Some(eleve_id))
            .cloned()
            .collect()
    }

    fn all_events(&self) -> Vec<MetierEvent> {
        self.inner.lock().expect("event bus store lock").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingHandler {
        hits: Arc<AtomicUsize>,
    }

    impl EventHandler for CountingHandler {
        fn name(&self) -> &str {
            "counting"
        }

        fn on_event(&self, _event: &MetierEvent) -> Result<(), EventBusError> {
            self.hits.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn publish_stores_and_notifies_handlers() {
        let bus = InMemoryEventBus::new();
        let hits = Arc::new(AtomicUsize::new(0));
        bus.register_handler(Arc::new(CountingHandler {
            hits: Arc::clone(&hits),
        }));

        let etab = Uuid::new_v4();
        let eleve = Uuid::new_v4();
        let event = bus
            .publish(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::ELEVE_CREATED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve,
                eleve_id: Some(eleve),
                payload: json!({ "nom": "DUPONT" }),
                actor_user_id: Some("user-1".into()),
            })
            .expect("publish");

        assert_eq!(event.type_name, types::ELEVE_CREATED);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert_eq!(bus.events_for_eleve(etab, eleve).len(), 1);
    }

    #[test]
    fn reject_nil_etablissement() {
        let bus = InMemoryEventBus::new();
        let err = bus
            .publish(MetierEventDraft {
                etablissement_id: Uuid::nil(),
                type_name: types::ELEVE_CREATED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: Uuid::new_v4(),
                eleve_id: None,
                payload: Value::Null,
                actor_user_id: None,
            })
            .expect_err("must fail");
        assert_eq!(err, EventBusError::MissingEtablissement);
    }

    /// F1 : l'alias `fac.*` ↔ `facturation.*` est une bijection sans doublon, et le nom canonique
    /// suit toujours le motif `facturation.<mot>` (aucun montant ni nom dans un type).
    #[test]
    fn alias_facturation_bijectif_et_canonique() {
        let mut anciens = std::collections::BTreeSet::new();
        let mut canons = std::collections::BTreeSet::new();
        for (ancien, canon) in ALIAS_FACTURATION {
            assert!(ancien.starts_with("fac.") || ancien.starts_with("fin."), "{ancien}");
            assert!(canon.starts_with("facturation."), "{canon}");
            assert!(anciens.insert(*ancien), "ancien en double : {ancien}");
            assert!(canons.insert(*canon), "canonique en double : {canon}");
            assert_eq!(canonique_facturation(ancien), *canon);
            assert_eq!(canonique_facturation(canon), *canon);
            assert_eq!(equivalents_facturation(ancien), vec![*ancien, *canon]);
            assert_eq!(equivalents_facturation(canon), vec![*canon, *ancien]);
        }
        // Un type sans alias n'a qu'un élément.
        assert_eq!(equivalents_facturation(types::ELEVE_CREATED), vec![types::ELEVE_CREATED]);
        assert_eq!(
            equivalents_facturation(types::FACTURATION_GEL),
            vec![types::FACTURATION_GEL]
        );
    }
}
