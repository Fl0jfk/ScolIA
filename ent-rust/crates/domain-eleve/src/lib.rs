//! Socle élève — **au-dessus de tout** dans l'ENT.
//!
//! L'élève n'est pas un sous-objet de l'administratif. Admin, absences, notes…
//! viennent **piocher** ici (faits liés à `eleve_id`). Pas de fiche élève dupliquée
//! dans un autre « domaine ».
//!
//! Pattern : `plan_*` (invariants + événement) puis apply (repo + bus).
//! Historique daté : [`periode`] — dates d'effet classe / régime / MEF / site.

mod absence;
mod carnet;
mod continuum;
mod document;
mod dossier;
mod invitation;
mod fiche_sante;
mod fiche_sante_pdf;
mod pai;
mod infirmerie;
mod psy;
mod psy_pdf;
mod edt;
mod facturation;
mod facturation_ogec;
mod facturation_pdf;
mod facturation_rappel;
mod facturation_sepa;
mod messagerie;
mod cahier_texte;
mod notes;
mod bulletin_pdf;
mod periode;
mod repas;
mod sanction;
mod commission;
mod sortie;
mod internat;
mod dispense_vs;
mod journal;
mod alerte_cpe;
mod doc_signer;
mod conseil_classe;
mod info_comp;
mod vacances;
mod homonyme;
mod menu_cantine;
mod quotient;
mod tva;
mod trombi;
mod param_etab;
mod salle;
mod filiere;
mod inscription_suivi;
mod ine;
mod lsl_export;
mod onde_export;
mod tarif_conso;
mod notanet_export;
mod parcoursup_export;
mod stage;
mod edt_contrainte;
mod absence_rh;
mod edition_vs;
mod lsun_export;

pub use absence::{
    annee_scolaire_de, corps_alerte_seuil, debut_annee_scolaire, module_absences_voit_absent,
    module_cantine_voit_absent, plan_accepter_justificatif, plan_annuler_vers_present,
    plan_annuler_soumission_justificatif, plan_clore_signal_absence, plan_declare_absence,
    plan_justifier_absence,
    plan_prendre_en_charge_signal, plan_refuser_justificatif, sujet_alerte_seuil, today_utc,
    Absence, AbsenceRepository, AbsenceStatut, CloreSignalAbsence, DecisionAbsence, DeclareAbsence,
    InMemoryAbsenceRepo, JustifierAbsence, PlannedAbsence, PrendreEnChargeSignal, SignalAbsence,
    SignalAbsenceStatut, plan_lier_signal_commission, SEUIL_ABSENCES_DEFAUT,
};
pub use document::{
    grant_est_actif, plan_ajouter_dispositif, plan_decider_grant, plan_demander_grant,
    plan_deposer_document, AjouterDispositif, DeciderGrant, DemanderGrant, DeposerDocument,
    DispositifKind, DocConfidentialite, DocTiroir, DocumentGrant, EleveDispositif, EleveDocument,
    GrantStatus,
};
pub use dossier::{
    age_ans_au, assert_parts_paiement, plan_ajouter_option, plan_update_identite, AjouterOption,
    DossierEleve, DossierSections, EleveOption, LienDossier, OptionNature, UpdateIdentite,
};
pub use invitation::{
    email_compte_eleve_defaut, generer_mot_de_passe_temporaire, plan_inviter_compte_eleve,
    EleveInvitation, InvitationPlanifiee, InvitationStatut, InviterCompteEleve,
};
pub use fiche_sante::{
    plan_upsert_fiche_sante, vue_famille as vue_fiche_sante_famille, FicheSante, UpsertFicheSante,
};
pub use fiche_sante_pdf::{
    plan_resume_fiche_sante, render_resume_fiche_sante_pdf, GenererResumeFicheSante,
    ResumeFicheSante,
};
pub use pai::{
    plan_attacher_ordonnance_pai, plan_signer_pai_famille, plan_upsert_pai,
    vue_famille as vue_pai_famille, AttacherOrdonnancePai, Pai, PaiPiece, PaiPieceMeta,
    SignerPaiFamille, UpsertPai, PAI_PIECE_MAX_OCTETS,
};
pub use infirmerie::{
    plan_creer_passage_infirmerie, vue_famille as vue_passage_infirmerie_famille,
    CreerPassageInfirmerie, PassageInfirmerie, PassageInfirmerieMotif,
};
pub use psy::{
    plan_changer_statut_rdv_psy, plan_creer_entretien_psy, plan_creer_rdv_psy,
    plan_upsert_dossier_psy, vue_dossier_famille, vue_famille as vue_entretien_psy_famille,
    vue_rdv_famille, ChangerStatutRdvPsy, CreerEntretienPsy, CreerRdvPsy, DossierPsy,
    EntretienPsy, EntretienPsyKind, RdvPsy, RdvPsyStatut, UpsertDossierPsy,
};
pub use psy_pdf::{
    plan_courrier_orientation_psy, render_courrier_orientation_pdf, CourrierOrientationPsy,
    GenererCourrierOrientationPsy,
};
pub use edt::{
    creneau_actif_semaine, detect_conflits_edt, grille_edt_defaut, lundi_iso, parse_edt_import_csv,
    parse_edt_pronote_csv, plan_affecter_edt_creneau, plan_configurer_edt_calendrier_ab,
    plan_poser_edt_exception, resolve_pronote_lignes, semaine_ab_au_jour, slot_interdit_soft,
    solve_edt_greedy, solve_edt_greedy_soft, AffecterEdtCreneau, ConfigurerEdtCalendrierAb,
    EdtCalendrierAb, EdtConflict, EdtConflictKind, EdtContrainteSoft, EdtCreneauAffecte,
    EdtDemande, EdtException, EdtExceptionKind, EdtImportLigne, EdtImportParseError,
    EdtImportReport, EdtPronoteLigne, EdtSemaineAb, EdtSlot, EdtSolveReport, PoserEdtException,
};
pub use edition_vs::{
    plan_creer_edition_vs, rendre_edition_vs, CreerEditionVs, EditionVsTemplate, RendreEditionVs,
};
pub use lsun_export::{
    domaine_depuis_matiere, plan_exporter_lsun, render_lsun_csv, ExporterLsun, LigneLsun,
};
pub use sortie::{
    a_cours_autre_site_apres, appliquer_regle_niveau_parent, appliquer_regle_regime_parent,
    evaluer_controle_sortie, evaluer_controle_sortie_avec_contexte, normalize_sortie_regime,
    plan_controle_sortie, plan_controle_sortie_avec_contexte, plan_decider_demande_sortie,
    plan_demander_sortie_exceptionnelle, plan_upsert_autorisation_sortie, plan_upsert_sortie_param,
    plan_upsert_sortie_regle_niveau, plan_upsert_sortie_regle_regime, AutorisationSortie,
    ControleSortie, ControleSortieContexteSite, ControleSortieSource, ControleSortieVerdict,
    DeciderDemandeSortie, DemandeSortieExceptionnelle, DemandeSortieStatut,
    DemanderSortieExceptionnelle, RecapSortieEleve, SortieCreneau, SortieEdtApercu, SortieParamEtab,
    SortieRegime, SortieRegleNiveau, SortieRegleRegime, UpsertAutorisationSortie,
    UpsertSortieParamEtab, UpsertSortieRegleNiveau, UpsertSortieRegleRegime,
};
pub use internat::{
    normalize_regime_internat, plan_affecter_eleve_internat, plan_clore_affectation_internat,
    plan_creer_chambre_internat, plan_creer_suivi_internat, plan_pointer_presence_internat,
    AffectationInternat, AffecterEleveInternat, ChambreGenre, ChambreInternat, CreerChambreInternat,
    CreerSuiviInternat, PointerPresenceInternat, PresenceInternat, PresenceInternatStatut,
    RecapInternatEleve, SuiviInternat,
};
pub use dispense_vs::{
    dispense_vs_couvre, plan_clore_dispense_vs, plan_poser_dispense_vs, DispenseVs, DispenseVsMotif,
    DispenseVsStatut, PoserDispenseVs,
};
pub use journal::{plan_enregistrer_journal, EnregistrerJournal, JournalEntree};
pub use alerte_cpe::{
    plan_clore_alerte_cpe, plan_creer_alerte_cpe, AlerteCpe, AlerteCpeSeverite, AlerteCpeStatut,
    CreerAlerteCpe,
};
pub use doc_signer::{
    plan_annuler_doc_a_signer, plan_publier_doc_a_signer, plan_signer_doc_a_signer, DocASigner,
    DocSignerStatut, PublierDocASigner, SignerDocASigner,
};
pub use conseil_classe::{
    plan_ajouter_dossier_conseil, plan_creer_conseil_classe, plan_decider_dossier_conseil,
    plan_tenir_conseil, AjouterDossierConseil, ConseilClasse, ConseilClasseDossier,
    ConseilClasseStatut, CreerConseilClasse, DeciderDossierConseil, MentionConseil,
};
pub use info_comp::{
    plan_creer_info_comp_champ, plan_upsert_info_comp_valeur, CreerInfoCompChamp, InfoCompChamp,
    InfoCompType, InfoCompValeur, UpsertInfoCompValeur,
};
pub use vacances::{
    jour_est_ferme, plan_creer_periode_vacances, CreerPeriodeVacances, PeriodeVacances,
    VacancesKind,
};
pub use homonyme::{
    normalize_identite, plan_rechercher_homonymes, score_homonyme, HomonymeCandidat,
    RechercherHomonymes,
};
pub use menu_cantine::{plan_upsert_menu_cantine, MenuCantineJour, UpsertMenuCantine};
pub use quotient::{plan_upsert_quotient_foyer, QuotientFoyer, UpsertQuotientFoyer};
pub use tva::{
    ht_depuis_ttc, plan_creer_taux_tva, ttc_depuis_ht, CreerTauxTva, TauxTva,
};
pub use trombi::{plan_poser_photo_eleve, PoserPhotoEleve, TrombiEntree};
pub use param_etab::{plan_upsert_param_etab, ParamEtab, UpsertParamEtab};
pub use salle::{plan_creer_salle, CreerSalle, Salle};
pub use filiere::{
    plan_creer_filiere, plan_poser_voeu, CreerFiliere, Filiere, PoserVoeu, VoeuInscription,
};
pub use inscription_suivi::{
    plan_upsert_inscription_suivi, InscriptionEtapeKind, InscriptionSuivi, UpsertInscriptionSuivi,
};
pub use ine::normalize_ine;
pub use lsl_export::{
    plan_exporter_lsl, render_lsl_csv, ExporterLsl, LigneLsl,
};
pub use onde_export::{
    plan_exporter_onde, render_onde_csv, ExporterOnde, LigneOnde,
};
pub use tarif_conso::{
    plan_creer_tarif_conso, tarif_couvre, CreerTarifConso, TarifConso,
};
pub use notanet_export::{
    plan_exporter_notanet, render_notanet_csv, ExporterNotanet, LigneNotanet,
};
pub use parcoursup_export::{
    plan_exporter_parcoursup, render_parcoursup_csv, ExporterParcoursup, LigneParcoursup,
};
pub use stage::{
    plan_creer_convention_stage, plan_creer_entreprise, ConventionStage, CreerConventionStage,
    CreerEntreprise, Entreprise,
};
pub use edt_contrainte::{
    plan_creer_edt_contrainte, CreerEdtContrainte, EdtContrainte, EdtContrainteKind,
};
pub use absence_rh::{
    plan_declarer_absence_rh, plan_valider_absence_rh, AbsenceRh, DeclarerAbsenceRh,
};
pub use cahier_texte::{
    plan_creer_cahier_texte, CahierTexteEntree, CreerCahierTexte,
};
pub use periode::{
    day_before, en_effectif_au_jour, module_appel_classe_au, plan_cutover, plan_open_initial,
    valeur_au, AttributKind, ChangeAttribut, CutoverOp, ElevePeriode, InMemoryPeriodeRepo,
    PeriodeRepository, PlannedCutover,
};
pub use repas::{
    agreger_volumes_repas, bourse_deductible_facture, compter_jours_facturables, dispense_couvre,
    grille_couvre_jour, inscrit_activite_kind_au_jour,
    inscrit_activite_kind_au_jour_avec_dispenses, plan_affecter_bourse,
    plan_annuler_dispense_repas, plan_creer_bourse_type, plan_creer_repas_activite,
    plan_decider_bourse, plan_enregistrer_passage_badge, plan_exporter_volumes_repas,
    plan_poser_dispense_repas, plan_poser_grille_repas, render_volumes_cuisine_csv, AffecterBourse,
    BourseStatus, BourseType, CreerBourseType, CreerRepasActivite, DeciderBourse, DispenseSource,
    EleveBourse,
    EleveDispenseRepas, EleveGrilleRepas, EnregistrerPassageBadge, ExporterVolumesRepas,
    JoursMasque, PassageRepas, PassageSource, PoserDispenseRepas, PoserGrilleRepas, RepasActivite,
    RepasActiviteKind, VolumeRepasJour,
};
pub use continuum::{
    annee_relative, annees_continuum, assert_vanne_acompte, assert_vannes_documents,
    candidats_lot_montee, codes_documents_manquants, mef_suivant, niveau_depuis_classe_ou_mef,
    niveau_effectif, niveau_suivant, normalize_niveau, plan_choisir_option_n1,
    plan_configurer_acompte_campagne, plan_confirmer_reinscription_parent,
    plan_enregistrer_scolarite_annee, plan_montee_rapide, plan_ouvrir_campagne,
    plan_poser_exigence_doc, scolarite_match_niveau, ChoisirOptionN1, ConfigurerAcompteCampagne,
    ConfirmerReinscription, EnregistrerScolariteAnnee, ExigenceDocReinscription, LotMonteeIgnore,
    LotMonteeResultat, MonteeLot, MonteeRapide, OptionAnnee, OptionAnneeStatut,
    OuvrirCampagneReinscription, PlannedMontee, PoserExigenceDoc, ReinscriptionCampagne,
    ScolariteAnnee, ScolariteAnneeStatut, ScolariteSource,
};
pub use carnet::{
    corps_notif_carnet, plan_creer_carnet_entree, plan_signer_carnet_entree, sujet_notif_carnet,
    CarnetCategorie, CarnetEntree, CreerCarnetEntree, SignerCarnetEntree,
};
pub use facturation::{
    couvrir_echeances, numero_quittance, plan_creer_facture, plan_echeancier_parts_egales,
    plan_emettre_facture, plan_emettre_quittance, plan_encaisser, plan_ligne_cantine_depuis_passages,
    plan_poser_echeancier, reste_a_payer, total_deductions_lignes, ventiler_bourse_sur_echeances,
    CreerFacture, Echeance, EcheanceCmd, EcheanceCouverture, EcheanceVue, Encaissement,
    EncaisserFacture, Facture, FactureLigne, FactureStatut, LigneFactureCmd, MoyenPaiement,
    PoserEcheancier, Quittance, NATURE_CANTINE_BADGE, NATURE_REINSCRIPTION_ACOMPTE,
};
pub use facturation_ogec::{
    plan_exporter_ogec, render_ogec_csv, ExporterOgec, OgecLigneExport,
};
pub use facturation_pdf::{euros_centimes, render_facture_pdf, render_quittance_pdf};
pub use facturation_rappel::{
    corps_rappel, plan_candidats_rappel, plan_creer_rappel, sujet_rappel, RappelCanal,
    RappelCandidat, RappelFacture, RappelKind, RAPPEL_J_MOINS,
};
pub use facturation_sepa::{
    moyen_prelevement, normalize_iban, plan_creer_mandat_sepa, plan_executer_prelevement,
    plan_marquer_exporte, plan_planifier_prelevement, plan_rejeter_prelevement,
    plan_revoquer_mandat, plan_sepa_crediteur, render_sepa_csv, render_sepa_pain008,
    CreerMandatSepa, MandatSepa, MandatSepaStatut, PlanifierPrelevement, PrelevementSepa,
    PrelevementStatut, SepaCrediteur, SepaLigneExport,
};
pub use messagerie::{
    appliquer_modele, enrichir_messages_lues, plan_accuser_lecture, plan_archiver_fil,
    plan_attacher_piece, plan_classer_fil, plan_creer_msg_modele, plan_ouvrir_fil, plan_repondre_fil,
    AccuserLecture, ArchiverFil, AttacherPiece, ClasserFil, ContexteModele, CreerMsgModele,
    Message, MessageLue, MsgAccuse, MsgFil, MsgModele, MsgPiece, MsgPieceMeta, MsgPriorite,
    OuvrirFil, RepondreFil, MSG_PIECE_MAX_OCTETS,
};
pub use notes::{
    moyenne_sur_20, plan_creer_devoir, plan_creer_note_periode, plan_saisir_appreciation,
    Appreciation, CreerDevoir, CreerNotePeriode, Devoir, MoyenneMatiere, NoteLue, NotePeriode,
    NotePeriodeStatut, NotePourMoyenne, SaisirAppreciation,
};
pub use bulletin_pdf::{
    assert_periode_bulletin, plan_bulletin_eleve, render_bulletin_pdf, BulletinEleve,
    LigneBulletinMatiere,
};
pub use sanction::{
    plan_annuler_sanction, plan_creer_incident_type, plan_creer_sanction_motif,
    plan_creer_sanction_type, plan_declarer_incident, plan_prononcer_sanction, AnnulerSanction,
    CreerReferentielVs, DeclarerIncident, Incident, IncidentType, PrononcerSanction, Sanction,
    SanctionMotif, SanctionStatut, SanctionType,
};
pub use commission::{
    plan_convoquer_commission, plan_creer_commission, plan_decider_dossier, solde_points_vs,
    vue_dossier_famille as vue_commission_dossier_famille, CommissionDossier, CommissionStatut,
    CommissionVs, ConvoquerCommission, CreerCommissionVs, DeciderCommissionDossier,
    DecisionCommission, POINTS_VS_DEPART,
};

use ent_events::{
    types, AggregateKind, EventBus, EventBusError, EventHandler, MetierEvent, MetierEventDraft,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EleveStatus {
    Actif,
    Sorti,
    Preinscrit,
}

impl EleveStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Actif => "actif",
            Self::Sorti => "sorti",
            Self::Preinscrit => "preinscrit",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Eleve {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub ine: Option<String>,
    pub sexe: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub numero_badge: Option<String>,
    pub redoublant: bool,
    pub photo_key: Option<String>,
    pub status: EleveStatus,
    /// Projection **au jour courant** (bandeau) — l'historique vit dans `ent_eleve_periode`.
    pub classe: Option<String>,
    pub regime: Option<String>,
    pub mef: Option<String>,
    pub site: Option<String>,
    pub date_entree: Option<NaiveDate>,
    pub date_sortie: Option<NaiveDate>,
    pub motif_sortie: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateEleve {
    pub etablissement_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub ine: Option<String>,
    pub sexe: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub numero_badge: Option<String>,
    pub redoublant: bool,
    pub classe: Option<String>,
    pub regime: Option<String>,
    pub mef: Option<String>,
    pub site: Option<String>,
    pub date_entree: Option<NaiveDate>,
    pub date_sortie: Option<NaiveDate>,
    pub motif_sortie: Option<String>,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChangeClasse {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nouvelle_classe: String,
    pub date_effet: NaiveDate,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChangeRegime {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nouveau_regime: String,
    pub date_effet: NaiveDate,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChangeMef {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nouveau_mef: String,
    pub date_effet: NaiveDate,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChangeSite {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nouveau_site: String,
    pub date_effet: NaiveDate,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChangeStatus {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nouveau_status: EleveStatus,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SortirEleve {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub date_sortie: NaiveDate,
    pub motif_sortie: Option<String>,
    pub actor_user_id: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("nom ou prénom vide")]
    IdentityRequired,
    #[error("élève introuvable")]
    NotFound,
    #[error("mauvais établissement (isolation tenant)")]
    TenantMismatch,
    #[error("classe vide")]
    EmptyClasse,
    #[error("valeur d'attribut vide")]
    EmptyAttributValeur,
    #[error("motif d'absence vide")]
    EmptyAbsenceMotif,
    #[error("signal d'absence invalide")]
    SignalAbsenceInvalide,
    #[error("signal d'absence déjà clos")]
    SignalAbsenceDejaClos,
    #[error("période d'absence invalide (fin < début)")]
    InvalidAbsencePeriod,
    #[error("date d'effet antérieure au début de la période ouverte")]
    DateEffetAvantDebut,
    #[error("aucune période ouverte pour cet attribut")]
    NoOpenPeriode,
    #[error("type de période incohérent")]
    PeriodeKindMismatch,
    #[error("somme des parts payeurs ≠ 100 % pour cet élève")]
    PartsPaiementInvalides,
    #[error("durée de grant invalide")]
    GrantDureeInvalide,
    #[error("grant déjà tranché")]
    GrantDejaTranche,
    #[error("grille repas sans aucun jour actif")]
    GrilleRepasSansJour,
    #[error("dispense repas invalide")]
    DispenseRepasInvalide,
    #[error("dispense repas déjà posée pour ce jour")]
    DispenseRepasDejaPosee,
    #[error("badge repas inconnu")]
    BadgeRepasInconnu,
    #[error("passage badge déjà enregistré")]
    PassageBadgeDejaEnregistre,
    #[error("passage badge refusé (dispense ou hors grille)")]
    PassageBadgeRefuse,
    #[error("bourse déjà tranchée")]
    BourseDejaTranchee,
    #[error("entrée carnet non visible famille")]
    CarnetNonVisibleFamille,
    #[error("entrée carnet déjà signée")]
    CarnetDejaSigne,
    #[error("période de notes : dates invalides")]
    NotePeriodeDatesInvalides,
    #[error("barème ou coefficient de note invalide")]
    NoteBaremeInvalide,
    #[error("période de notes fermée — saisie impossible")]
    NotePeriodeFermee,
    #[error("période fermée — appréciation impossible")]
    NoteAppreciationPeriodeFermee,
    #[error("bulletin réservé aux périodes clôturées")]
    NoteBulletinPeriodeNonFermee,
    #[error("appréciation vide")]
    NoteAppreciationVide,
    #[error("appréciation trop longue")]
    NoteAppreciationTropLongue,
    #[error("devoir introuvable")]
    DevoirIntrouvable,
    #[error("sujet ou message vide")]
    MessageVide,
    #[error("modèle de message invalide")]
    MsgModeleInvalide,
    #[error("priorité ou dossier de messagerie invalide")]
    MsgBoiteInvalide,
    #[error("fil de messagerie introuvable")]
    MsgFilIntrouvable,
    #[error("pièce jointe message invalide (nom, mime ou taille)")]
    MsgPieceInvalide,
    #[error("seule l'auteur du message peut y attacher une pièce")]
    MsgPiecePasAuteur,
    #[error("pièce jointe message introuvable")]
    MsgPieceIntrouvable,
    #[error("compte élève déjà lié — invitation refusée")]
    InvitationCompteDejaLie,
    #[error("élève inactif — invitation refusée")]
    InvitationEleveInactif,
    #[error("e-mail d'invitation invalide")]
    InvitationEmailInvalide,
    #[error("passage infirmière invalide")]
    PassageInfirmerieInvalide,
    #[error("fiche santé invalide")]
    FicheSanteInvalide,
    #[error("PAI invalide")]
    PaiInvalide,
    #[error("PAI inactif — signature impossible")]
    PaiInactif,
    #[error("PAI déjà signé par la famille")]
    PaiDejaSigne,
    #[error("pièce / ordonnance PAI invalide")]
    PaiPieceInvalide,
    #[error("pièce / ordonnance PAI introuvable")]
    PaiPieceIntrouvable,
    #[error("entretien psy invalide")]
    EntretienPsyInvalide,
    #[error("RDV psy invalide")]
    RdvPsyInvalide,
    #[error("dossier psy invalide")]
    DossierPsyInvalide,
    #[error("courrier orientation psy invalide")]
    CourrierPsyInvalide,
    #[error("exception EDT invalide")]
    EdtExceptionInvalide,
    #[error("calendrier EDT A/B invalide (référence doit être un lundi)")]
    EdtCalendrierAbInvalide,
    #[error("cahier de texte invalide")]
    CahierTexteInvalide,
    #[error("année scolaire invalide (attendu YYYY-YYYY+1)")]
    AnneeScolaireInvalide,
    #[error("réinscription : pas de projection N+1 à confirmer")]
    ReinscriptionNonPrete,
    #[error("documents de réinscription manquants : {0}")]
    DocumentsReinscriptionManquants(String),
    #[error("exigence documentaire de réinscription invalide")]
    ExigenceDocInvalide,
    #[error("acompte de réinscription invalide")]
    AcompteReinscriptionInvalide,
    #[error("acompte de réinscription insuffisant (requis {requis} c, encaissé {encaisse} c)")]
    AcompteReinscriptionManquant { requis: i64, encaisse: i64 },
    #[error("période d'export OGEC invalide")]
    OgecPeriodeInvalide,
    #[error("mandat SEPA invalide")]
    SepaMandatInvalide,
    #[error("IBAN SEPA invalide")]
    SepaIbanInvalide,
    #[error("mandat SEPA inactif ou révoqué")]
    SepaMandatInactif,
    #[error("prélèvement SEPA invalide")]
    SepaPrelevementInvalide,
    #[error("rappel d'échéance invalide")]
    RappelInvalide,
    #[error("facture invalide")]
    FactureInvalide,
    #[error("facture sans ligne")]
    FactureSansLigne,
    #[error("facture déjà émise ou soldée")]
    FactureDejaEmise,
    #[error("facture non émise — encaissement impossible")]
    FactureNonEmise,
    #[error("facture déjà soldée")]
    FactureDejaSoldee,
    #[error("encaissement invalide")]
    EncaissementInvalide,
    #[error("encaissement dépasse le reste à payer")]
    EncaissementDepasse,
    #[error("échéancier invalide")]
    EcheancierInvalide,
    #[error("somme des échéances ≠ total facture")]
    EcheancierSommeIncorrecte,
    #[error("échéancier déjà posé sur cette facture")]
    EcheancierDejaPose,
    #[error("quittance invalide")]
    QuittanceInvalide,
    #[error("niveau scolaire inconnu pour montée")]
    NiveauInconnu,
    #[error("pas de niveau suivant (fin de cursus)")]
    NiveauSansSuite,
    #[error("campagne de réinscription fermée")]
    CampagneFermee,
    #[error("durée de sanction invalide")]
    SanctionDureeInvalide,
    #[error("sanction déjà annulée")]
    SanctionDejaAnnulee,
    #[error("commission VS invalide")]
    CommissionVsInvalide,
    #[error("dossier de commission déjà décidé")]
    CommissionDejaDecidee,
    #[error("demande de sortie exceptionnelle invalide")]
    SortieDemandeInvalide,
    #[error("demande de sortie déjà tranchée")]
    SortieDemandeDejaTranchee,
    #[error("règle niveau : choix parent interdit")]
    SortieRegleNiveauInterdit,
    #[error("règle régime : choix parent interdit")]
    SortieRegleRegimeInterdit,
    #[error("régime de sortie invalide (EXT|DP|INT)")]
    SortieRegimeInvalide,
    #[error("chambre internat invalide")]
    InternatChambreInvalide,
    #[error("chambre internat pleine")]
    InternatChambrePleine,
    #[error("élève déjà affecté en internat")]
    InternatDejaAffecte,
    #[error("régime internat requis (INT)")]
    InternatRegimeRequis,
    #[error("affectation internat invalide")]
    InternatAffectationInvalide,
    #[error("affectation internat déjà close")]
    InternatAffectationDejaClose,
    #[error("présence internat invalide")]
    InternatPresenceInvalide,
    #[error("suivi internat invalide")]
    InternatSuiviInvalide,
    #[error("dispense VS invalide")]
    DispenseVsInvalide,
    #[error("dispense VS déjà close")]
    DispenseVsDejaClose,
    #[error("entrée journal invalide")]
    JournalInvalide,
    #[error("alerte CPE invalide")]
    AlerteCpeInvalide,
    #[error("alerte CPE déjà close")]
    AlerteCpeDejaClose,
    #[error("document à signer invalide")]
    DocASignerInvalide,
    #[error("document déjà signé")]
    DocASignerDejaSigne,
    #[error("conseil de classe invalide")]
    ConseilClasseInvalide,
    #[error("dossier de conseil déjà décidé")]
    ConseilDossierDejaDecide,
    #[error("info complémentaire invalide")]
    InfoCompInvalide,
    #[error("période de vacances invalide")]
    VacancesInvalide,
    #[error("recherche homonymes invalide")]
    HomonymeInvalide,
    #[error("menu cantine invalide")]
    MenuCantineInvalide,
    #[error("quotient familial invalide")]
    QuotientInvalide,
    #[error("taux TVA invalide")]
    TvaInvalide,
    #[error("photo / trombi invalide")]
    TrombiInvalide,
    #[error("paramètre établissement invalide")]
    ParamEtabInvalide,
    #[error("salle invalide")]
    SalleInvalide,
    #[error("filière / vœu invalide")]
    FiliereInvalide,
    #[error("suivi inscription invalide")]
    InscriptionSuiviInvalide,
    #[error("INE invalide")]
    IneInvalide,
    #[error("tarif consommation invalide")]
    TarifConsoInvalide,
    #[error("absence RH invalide")]
    AbsenceRhInvalide,
    #[error("affectation EDT invalide")]
    EdtAffectationInvalide,
    #[error("conflit EDT (prof / salle / classe)")]
    EdtConflit,
    #[error("édition VS invalide")]
    EditionVsInvalide,
    #[error("stage / entreprise / convention invalide")]
    StageInvalide,
    #[error("contrainte EDT invalide")]
    EdtContrainteInvalide,
    #[error("type ou motif VS inconnu pour cet établissement")]
    ReferentielVsInconnu,
    #[error(transparent)]
    Event(#[from] EventBusError),
}

#[derive(Debug, Clone)]
pub struct PlannedMutation {
    pub eleve: Eleve,
    pub periodes_initiales: Vec<ElevePeriode>,
    pub event: Option<MetierEventDraft>,
}

#[derive(Debug, Clone)]
pub struct PlannedSortie {
    pub eleve: Eleve,
    /// Veille de la date de sortie — ferme les périodes ouvertes.
    pub close_periodes_au: Option<NaiveDate>,
    pub event: MetierEventDraft,
}

pub fn plan_create(cmd: CreateEleve) -> Result<PlannedMutation, DomainError> {
    let nom = cmd.nom.trim().to_string();
    let prenom = cmd.prenom.trim().to_string();
    if nom.is_empty() || prenom.is_empty() {
        return Err(DomainError::IdentityRequired);
    }
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(EventBusError::MissingEtablissement));
    }

    let date_entree = cmd.date_entree.or_else(|| Some(today_utc()));
    let debut_periodes = date_entree.unwrap_or_else(today_utc);
    let ine = match cmd.ine {
        Some(raw) if !raw.trim().is_empty() => Some(normalize_ine(&raw)?),
        _ => None,
    };

    let eleve = Eleve {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        nom: nom.clone(),
        prenom: prenom.clone(),
        ine,
        sexe: cmd.sexe,
        date_naissance: cmd.date_naissance,
        numero_badge: cmd.numero_badge,
        redoublant: cmd.redoublant,
        photo_key: None,
        status: EleveStatus::Actif,
        classe: cmd.classe.clone(),
        regime: cmd.regime.clone(),
        mef: cmd.mef.clone(),
        site: cmd.site.clone(),
        date_entree,
        date_sortie: cmd.date_sortie,
        motif_sortie: cmd.motif_sortie,
    };

    let mut periodes_initiales = Vec::new();
    if let Some(p) = plan_open_initial(&eleve, AttributKind::Classe, cmd.classe.as_deref().unwrap_or(""), debut_periodes)
    {
        periodes_initiales.push(p);
    }
    if let Some(p) = plan_open_initial(&eleve, AttributKind::Regime, cmd.regime.as_deref().unwrap_or(""), debut_periodes)
    {
        periodes_initiales.push(p);
    }
    if let Some(p) = plan_open_initial(&eleve, AttributKind::Mef, cmd.mef.as_deref().unwrap_or(""), debut_periodes)
    {
        periodes_initiales.push(p);
    }
    if let Some(p) = plan_open_initial(&eleve, AttributKind::Site, cmd.site.as_deref().unwrap_or(""), debut_periodes)
    {
        periodes_initiales.push(p);
    }

    let event = MetierEventDraft {
        etablissement_id: eleve.etablissement_id,
        type_name: types::ELEVE_CREATED.to_string(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: eleve.id,
        eleve_id: Some(eleve.id),
        payload: json!({
            "nom": eleve.nom,
            "prenom": eleve.prenom,
            "classe": eleve.classe,
            "mef": eleve.mef,
            "status": eleve.status.as_str(),
            "date_entree": eleve.date_entree.map(|d| d.to_string()),
        }),
        actor_user_id: cmd.actor_user_id,
    };
    Ok(PlannedMutation {
        eleve,
        periodes_initiales,
        event: Some(event),
    })
}

pub fn plan_change_status(
    current: &Eleve,
    cmd: ChangeStatus,
) -> Result<PlannedMutation, DomainError> {
    if current.etablissement_id != cmd.etablissement_id {
        return Err(DomainError::TenantMismatch);
    }
    let mut eleve = current.clone();
    if eleve.status == cmd.nouveau_status {
        return Ok(PlannedMutation {
            eleve,
            periodes_initiales: Vec::new(),
            event: None,
        });
    }
    let ancien = eleve.status.as_str().to_string();
    eleve.status = cmd.nouveau_status.clone();
    let event = MetierEventDraft {
        etablissement_id: eleve.etablissement_id,
        type_name: types::ELEVE_STATUS_CHANGED.to_string(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: eleve.id,
        eleve_id: Some(eleve.id),
        payload: json!({
            "ancien_status": ancien,
            "nouveau_status": eleve.status.as_str(),
        }),
        actor_user_id: cmd.actor_user_id,
    };
    Ok(PlannedMutation {
        eleve,
        periodes_initiales: Vec::new(),
        event: Some(event),
    })
}

/// Sortie datée : plus d'effectif après la date ; historique conservé.
pub fn plan_sortir(current: &Eleve, cmd: SortirEleve, today: NaiveDate) -> Result<PlannedSortie, DomainError> {
    if current.etablissement_id != cmd.etablissement_id {
        return Err(DomainError::TenantMismatch);
    }
    let mut eleve = current.clone();
    eleve.date_sortie = Some(cmd.date_sortie);
    eleve.motif_sortie = cmd
        .motif_sortie
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());
    if cmd.date_sortie <= today {
        eleve.status = EleveStatus::Sorti;
    }

    // Ferme les périodes ouvertes la veille de la sortie (historique conservé).
    let close_periodes_au = Some(day_before(cmd.date_sortie));

    let event = MetierEventDraft {
        etablissement_id: eleve.etablissement_id,
        type_name: types::ELEVE_SORTIE.to_string(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: eleve.id,
        eleve_id: Some(eleve.id),
        payload: json!({
            "date_sortie": cmd.date_sortie.to_string(),
            "motif_sortie": eleve.motif_sortie,
            "status": eleve.status.as_str(),
        }),
        actor_user_id: cmd.actor_user_id,
    };
    Ok(PlannedSortie {
        eleve,
        close_periodes_au,
        event,
    })
}

pub trait EleveRepository: Send + Sync {
    fn get(&self, etablissement_id: Uuid, eleve_id: Uuid) -> Result<Option<Eleve>, DomainError>;
    fn upsert(&self, eleve: Eleve) -> Result<(), DomainError>;
}

#[derive(Default)]
pub struct InMemoryEleveRepo {
    store: Mutex<HashMap<(Uuid, Uuid), Eleve>>,
}

impl InMemoryEleveRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl EleveRepository for InMemoryEleveRepo {
    fn get(&self, etablissement_id: Uuid, eleve_id: Uuid) -> Result<Option<Eleve>, DomainError> {
        Ok(self
            .store
            .lock()
            .expect("eleve repo lock")
            .get(&(etablissement_id, eleve_id))
            .cloned())
    }

    fn upsert(&self, eleve: Eleve) -> Result<(), DomainError> {
        self.store
            .lock()
            .expect("eleve repo lock")
            .insert((eleve.etablissement_id, eleve.id), eleve);
        Ok(())
    }
}

pub struct EleveService<R, A, P, B>
where
    R: EleveRepository,
    A: AbsenceRepository,
    P: PeriodeRepository,
    B: EventBus,
{
    repo: R,
    absences: A,
    periodes: P,
    bus: B,
}

impl<R, A, P, B> EleveService<R, A, P, B>
where
    R: EleveRepository,
    A: AbsenceRepository,
    P: PeriodeRepository,
    B: EventBus,
{
    pub fn new(repo: R, absences: A, periodes: P, bus: B) -> Self {
        Self {
            repo,
            absences,
            periodes,
            bus,
        }
    }

    pub fn bus(&self) -> &B {
        &self.bus
    }

    pub fn absences(&self) -> &A {
        &self.absences
    }

    pub fn periodes(&self) -> &P {
        &self.periodes
    }

    pub fn create(&self, cmd: CreateEleve) -> Result<Eleve, DomainError> {
        let planned = plan_create(cmd)?;
        self.repo.upsert(planned.eleve.clone())?;
        for p in &planned.periodes_initiales {
            self.periodes
                .apply_cutover(&CutoverOp::OpenFirst { period: p.clone() })?;
        }
        if let Some(event) = planned.event {
            self.bus.publish(event)?;
        }
        Ok(planned.eleve)
    }

    pub fn change_classe(&self, cmd: ChangeClasse) -> Result<Eleve, DomainError> {
        self.change_attribut(ChangeAttribut {
            etablissement_id: cmd.etablissement_id,
            eleve_id: cmd.eleve_id,
            kind: AttributKind::Classe,
            nouvelle_valeur: cmd.nouvelle_classe,
            date_effet: cmd.date_effet,
            actor_user_id: cmd.actor_user_id,
        })
    }

    pub fn change_regime(&self, cmd: ChangeRegime) -> Result<Eleve, DomainError> {
        self.change_attribut(ChangeAttribut {
            etablissement_id: cmd.etablissement_id,
            eleve_id: cmd.eleve_id,
            kind: AttributKind::Regime,
            nouvelle_valeur: cmd.nouveau_regime,
            date_effet: cmd.date_effet,
            actor_user_id: cmd.actor_user_id,
        })
    }

    pub fn change_attribut(&self, cmd: ChangeAttribut) -> Result<Eleve, DomainError> {
        let current = self.require(cmd.etablissement_id, cmd.eleve_id)?;
        let open = self
            .periodes
            .open(cmd.etablissement_id, cmd.eleve_id, cmd.kind)?;
        let planned = plan_cutover(&current, open.as_ref(), cmd, today_utc())?;
        self.periodes.apply_cutover(&planned.op)?;
        self.repo.upsert(planned.eleve.clone())?;
        if let Some(event) = planned.event {
            self.bus.publish(event)?;
        }
        Ok(planned.eleve)
    }

    pub fn change_status(&self, cmd: ChangeStatus) -> Result<Eleve, DomainError> {
        let current = self.require(cmd.etablissement_id, cmd.eleve_id)?;
        let planned = plan_change_status(&current, cmd)?;
        self.repo.upsert(planned.eleve.clone())?;
        if let Some(event) = planned.event {
            self.bus.publish(event)?;
        }
        Ok(planned.eleve)
    }

    pub fn sortir(&self, cmd: SortirEleve) -> Result<Eleve, DomainError> {
        let current = self.require(cmd.etablissement_id, cmd.eleve_id)?;
        let planned = plan_sortir(&current, cmd, today_utc())?;
        if let Some(fin) = planned.close_periodes_au {
            self.periodes.close_open_periodes(
                planned.eleve.etablissement_id,
                planned.eleve.id,
                fin,
            )?;
        }
        self.repo.upsert(planned.eleve.clone())?;
        self.bus.publish(planned.event)?;
        Ok(planned.eleve)
    }

    /// Pose une absence sur l'élève (`ent_absence` + motif, ex. `sejour`).
    pub fn declare_absence(&self, cmd: DeclareAbsence) -> Result<Absence, DomainError> {
        self.require(cmd.etablissement_id, cmd.eleve_id)?;
        let planned = plan_declare_absence(cmd)?;
        self.absences.upsert(planned.absence.clone())?;
        if let Some(event) = planned.event {
            self.bus.publish(event)?;
        }
        Ok(planned.absence)
    }

    pub fn justifier_absence(&self, cmd: JustifierAbsence) -> Result<Absence, DomainError> {
        let current = self
            .absences
            .get(cmd.etablissement_id, cmd.absence_id)?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_justifier_absence(&current, cmd)?;
        self.absences.upsert(planned.absence.clone())?;
        if let Some(event) = planned.event {
            self.bus.publish(event)?;
        }
        Ok(planned.absence)
    }

    fn require(&self, etablissement_id: Uuid, eleve_id: Uuid) -> Result<Eleve, DomainError> {
        match self.repo.get(etablissement_id, eleve_id)? {
            Some(e) if e.etablissement_id == etablissement_id => Ok(e),
            Some(_) => Err(DomainError::TenantMismatch),
            None => Err(DomainError::NotFound),
        }
    }
}

/// Handler Brain d'exemple : journalise les changements de classe.
pub struct ClasseChangedProbe {
    pub seen: Mutex<Vec<String>>,
}

impl ClasseChangedProbe {
    pub fn new() -> Self {
        Self {
            seen: Mutex::new(Vec::new()),
        }
    }

    pub fn classes(&self) -> Vec<String> {
        self.seen.lock().expect("probe lock").clone()
    }
}

impl Default for ClasseChangedProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for ClasseChangedProbe {
    fn name(&self) -> &str {
        "classe_changed_probe"
    }

    fn on_event(&self, event: &MetierEvent) -> Result<(), EventBusError> {
        if event.type_name == types::SCOLARITE_CLASSE_CHANGED {
            let nouvelle = event
                .payload
                .get("nouvelle_valeur")
                .or_else(|| event.payload.get("nouvelle_classe"))
                .and_then(|v| v.as_str());
            if let Some(c) = nouvelle {
                self.seen.lock().expect("probe lock").push(c.to_string());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_events::InMemoryEventBus;
    use std::sync::Arc;

    fn service_with_probe() -> (
        EleveService<InMemoryEleveRepo, InMemoryAbsenceRepo, InMemoryPeriodeRepo, InMemoryEventBus>,
        Arc<ClasseChangedProbe>,
        Uuid,
    ) {
        let bus = InMemoryEventBus::new();
        let probe = Arc::new(ClasseChangedProbe::new());
        bus.register_handler(probe.clone());
        let etab = Uuid::new_v4();
        (
            EleveService::new(
                InMemoryEleveRepo::new(),
                InMemoryAbsenceRepo::new(),
                InMemoryPeriodeRepo::new(),
                bus,
            ),
            probe,
            etab,
        )
    }

    #[test]
    fn create_eleve_emits_created_event() {
        let (svc, _, etab) = service_with_probe();
        let eleve = svc
            .create(CreateEleve {
                etablissement_id: etab,
                nom: " Dupont ".into(),
                prenom: "Léa".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("4B".into()),
                regime: Some("DP".into()),
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: Some("admin".into()),
            })
            .expect("create");

        assert_eq!(eleve.nom, "Dupont");
        assert_eq!(eleve.status, EleveStatus::Actif);
        assert!(eleve.date_entree.is_some());
        let events = svc.bus().events_for_eleve(etab, eleve.id);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].type_name, types::ELEVE_CREATED);
        let periodes = svc.periodes().list(etab, eleve.id).unwrap();
        assert_eq!(periodes.len(), 2);
    }

    #[test]
    fn change_classe_notifies_brain_handler() {
        let (svc, probe, etab) = service_with_probe();
        let eleve = svc
            .create(CreateEleve {
                etablissement_id: etab,
                nom: "Martin".into(),
                prenom: "Tom".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("5A".into()),
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .expect("create");

        svc.change_classe(ChangeClasse {
            etablissement_id: etab,
            eleve_id: eleve.id,
            nouvelle_classe: "4B".into(),
            date_effet: today_utc(),
            actor_user_id: Some("cpe".into()),
        })
        .expect("change classe");

        assert_eq!(probe.classes(), vec!["4B".to_string()]);
    }

    #[test]
    fn reject_empty_identity() {
        let (svc, _, etab) = service_with_probe();
        let err = svc
            .create(CreateEleve {
                etablissement_id: etab,
                nom: "  ".into(),
                prenom: "Léa".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: None,
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .expect_err("must fail");
        assert_eq!(err, DomainError::IdentityRequired);
    }

    #[test]
    fn tenant_isolation_on_get() {
        let (svc, _, etab) = service_with_probe();
        let eleve = svc
            .create(CreateEleve {
                etablissement_id: etab,
                nom: "X".into(),
                prenom: "Y".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: None,
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .expect("create");

        let other = Uuid::new_v4();
        let err = svc
            .change_status(ChangeStatus {
                etablissement_id: other,
                eleve_id: eleve.id,
                nouveau_status: EleveStatus::Sorti,
                actor_user_id: None,
            })
            .expect_err("wrong tenant");
        assert_eq!(err, DomainError::NotFound);
    }

    #[test]
    fn absence_motif_sejour_lue_par_toutes_les_faces() {
        let (svc, _, etab) = service_with_probe();
        let eleve = svc
            .create(CreateEleve {
                etablissement_id: etab,
                nom: "Leo".into(),
                prenom: "Sejour".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("4B".into()),
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .expect("create");

        let debut = today_utc();
        let fin = debut + chrono::Duration::days(2);
        svc.declare_absence(DeclareAbsence {
            etablissement_id: etab,
            eleve_id: eleve.id,
            debut,
            fin,
            motif: "sejour".into(),
            actor_user_id: Some("vie-scolaire".into()),
        })
        .expect("declare");

        assert!(module_absences_voit_absent(svc.absences(), etab, eleve.id, debut).unwrap());
        assert!(module_cantine_voit_absent(svc.absences(), etab, eleve.id, debut).unwrap());
        assert!(module_cantine_voit_absent(svc.absences(), etab, eleve.id, fin).unwrap());

        let events = svc.bus().events_for_eleve(etab, eleve.id);
        assert!(events
            .iter()
            .any(|e| e.type_name == types::ELEVE_ABSENCE_DECLARED));
    }

    #[test]
    fn historique_classe_et_sortie_datée() {
        let (svc, _, etab) = service_with_probe();
        let entree = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let eleve = svc
            .create(CreateEleve {
                etablissement_id: etab,
                nom: "Hist".into(),
                prenom: "Daté".into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: Some("6A".into()),
                regime: Some("DP".into()),
                mef: None,
                site: None,
                date_entree: Some(entree),
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .expect("create");

        let jeudi = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        svc.change_classe(ChangeClasse {
            etablissement_id: etab,
            eleve_id: eleve.id,
            nouvelle_classe: "5B".into(),
            date_effet: jeudi,
            actor_user_id: None,
        })
        .expect("cutover");

        let mardi = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap();
        assert_eq!(
            module_appel_classe_au(svc.periodes(), etab, eleve.id, mardi)
                .unwrap()
                .as_deref(),
            Some("6A")
        );
        assert_eq!(
            module_appel_classe_au(svc.periodes(), etab, eleve.id, jeudi)
                .unwrap()
                .as_deref(),
            Some("5B")
        );

        let sortie = NaiveDate::from_ymd_opt(2026, 12, 20).unwrap();
        let sorti = svc
            .sortir(SortirEleve {
                etablissement_id: etab,
                eleve_id: eleve.id,
                date_sortie: sortie,
                motif_sortie: Some("déménagement".into()),
                actor_user_id: None,
            })
            .expect("sortir");
        assert_eq!(sorti.date_sortie, Some(sortie));
        assert!(!en_effectif_au_jour(&sorti, sortie));
        assert!(en_effectif_au_jour(
            &sorti,
            NaiveDate::from_ymd_opt(2026, 12, 19).unwrap()
        ));
    }
}
