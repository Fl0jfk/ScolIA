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
    pub const INSCRIPTION_VALIDATION_ANNULEE: &str = "inscription.validation_annulee";
    pub const INSCRIPTION_SOLLICITATION_SUSPENDUE: &str = "inscription.sollicitation_suspendue";
    pub const INSCRIPTION_CAMPAGNE_MODIFIEE: &str = "inscription.campagne_modifiee";
    pub const INSCRIPTION_OFFRE_MODIFIEE: &str = "inscription.offre_modifiee";
    pub const INSCRIPTION_ECHEANCE_PROLONGEE: &str = "inscription.echeance_prolongee";
    pub const INSCRIPTION_FERMETURE_DECLAREE: &str = "inscription.fermeture_declaree";
    pub const INSCRIPTION_DOSSIER_CLOS: &str = "inscription.dossier_clos";
    /// Socle 2 : création d'élève annulée (ELV-02, commande appelée par INS-68).
    pub const ELEVE_CREATION_ANNULEE: &str = "eleve.creation_annulee";
    pub const SCOLARITE_DEPART_ANNONCE: &str = "scolarite.depart_annonce";
    pub const RESPONSABLE_INVITE: &str = "inscription.responsable_invite";
    pub const INVITATION_ACCEPTEE: &str = "inscription.invitation_acceptee";
    pub const MANDAT_SEPA_A_SIGNER: &str = "fac.sepa_mandat_a_signer";
    /// S2-3 (D23) : import en lot, rejets traités.
    pub const IMPORT_LOT_APPLIQUE: &str = "import.lot_applique";
    pub const IMPORT_REJET_TRAITE: &str = "import.rejet_traite";
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
}
