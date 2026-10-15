use crate::absence_repo::PgAbsenceRepo;
use crate::appel_repo::{Appel, AppelLigne, AppelStatut, LigneEtat, PgAppelRepo};
use crate::auth_repo::{AuthUser, PgAuthRepo, SessionInfo, StaffMember};
use crate::carnet_repo::PgCarnetRepo;
use crate::cahier_texte_repo::PgCahierTexteRepo;
use crate::edt_repo::{jour_semaine_iso, EdtCreneau, PgEdtRepo};
use crate::eleve_compte_repo::{EleveCompte, PgEleveCompteRepo};
use crate::eleve_repo::PgEleveRepo;
use crate::event_store::PgEventStore;
use crate::foyer_repo::{CreateLien, EleveLien, LienDossierFoyer, PgFoyerRepo, UpdateLien};
use crate::seed::LABO_PASSWORD;
use crate::mail_repo::{InsertMailOutbox, MailOutboxRow, PgMailRepo};
use crate::sms_repo::{InsertSmsOutbox, PgSmsRepo};
use crate::push_repo::{InsertPushOutbox, PgPushRepo, PushDevice, RegisterPushDevice};
use crate::bank_repo::{BankRapprochementRow, BankRepo};
use crate::matiere_repo::{CreateMatiere, Matiere, PgMatiereRepo};
use crate::migrate::apply_socle_migrations;
use crate::continuum_repo::PgContinuumRepo;
use crate::document_repo::PgDocumentRepo;
use crate::motif_repo::{AbsenceMotif, PgMotifRepo};
use crate::fac_repo::PgFacRepo;
use crate::infirmerie_repo::PgInfirmerieRepo;
use crate::psy_repo::PgPsyRepo;
use crate::msg_repo::PgMsgRepo;
use crate::note_repo::{Note, PgNoteRepo, SaisirNote};
use crate::option_repo::PgOptionRepo;
use crate::periode_repo::PgPeriodeRepo;
use crate::repas_repo::PgRepasRepo;
use crate::sanction_repo::PgSanctionRepo;
use crate::sortie_repo::PgSortieRepo;
use crate::internat_repo::PgInternatRepo;
use crate::dispense_vs_repo::PgDispenseVsRepo;
use crate::journal_repo::PgJournalRepo;
use crate::alerte_cpe_repo::PgAlerteCpeRepo;
use crate::doc_signer_repo::PgDocSignerRepo;
use crate::conseil_classe_repo::PgConseilClasseRepo;
use crate::info_comp_repo::PgInfoCompRepo;
use crate::vacances_repo::PgVacancesRepo;
use crate::menu_cantine_repo::PgMenuCantineRepo;
use crate::quotient_repo::PgQuotientRepo;
use crate::tva_repo::PgTvaRepo;
use crate::param_etab_repo::PgParamEtabRepo;
use crate::salle_repo::PgSalleRepo;
use crate::filiere_repo::PgFiliereRepo;
use crate::inscription_repo::PgInscriptionRepo;
use crate::iban_histo_repo::{IbanHisto, PgIbanHistoRepo};
use crate::tarif_conso_repo::PgTarifConsoRepo;
use crate::absence_rh_repo::PgAbsenceRhRepo;
use crate::edition_vs_repo::PgEditionVsRepo;
use crate::stage_repo::PgStageRepo;
use crate::edt_contrainte_repo::PgEdtContrainteRepo;
use crate::signal_repo::PgSignalRepo;
use crate::url_guard::{assert_local_postgres_url, UrlGuardError};
use chrono::{NaiveDate, NaiveTime, Utc};
use domain_eleve::{
    age_ans_au, annee_relative, annee_scolaire_de, annees_continuum, bourse_deductible_facture,
    assert_vanne_acompte, assert_vannes_documents, candidats_lot_montee,
    agreger_volumes_repas, codes_documents_manquants, compter_jours_facturables,
    debut_annee_scolaire, inscrit_activite_kind_au_jour_avec_dispenses,
    niveau_depuis_classe_ou_mef, normalize_niveau, plan_exporter_volumes_repas,
    render_volumes_cuisine_csv,
    plan_accepter_justificatif, plan_affecter_bourse, plan_ajouter_dispositif, plan_ajouter_option,
    plan_annuler_dispense_repas, plan_annuler_vers_present, plan_change_status,
    plan_clore_signal_absence, plan_lier_signal_commission, plan_prendre_en_charge_signal,
    plan_choisir_option_n1, plan_configurer_acompte_campagne, plan_create, plan_creer_bourse_type,
    plan_creer_repas_activite, plan_cutover,     plan_decider_bourse, plan_decider_grant, plan_enregistrer_passage_badge,
    plan_declare_absence, plan_demander_grant, plan_deposer_document,
    plan_confirmer_reinscription_parent, plan_enregistrer_scolarite_annee, plan_justifier_absence,
    plan_montee_rapide, plan_ouvrir_campagne, plan_poser_dispense_repas, plan_poser_exigence_doc,
    plan_poser_grille_repas, plan_refuser_justificatif, plan_sortir, plan_update_identite,
    today_utc, Absence, AbsenceStatut,
    AffecterBourse, AjouterDispositif, AjouterOption, AttributKind, BourseType, ChangeAttribut,
    ChangeClasse, ChangeMef, ChangeRegime, ChangeSite, ChangeStatus, ChoisirOptionN1,
    ConfigurerAcompteCampagne, CreateEleve, CreerBourseType, CreerRepasActivite,
    ConfirmerReinscription, CutoverOp, DeciderBourse, DeciderGrant, DecisionAbsence, DeclareAbsence,
    DemanderGrant, DeposerDocument, DocConfidentialite, DocTiroir, DocumentGrant, DomainError,
    DispenseSource, DossierEleve, DossierSections, Eleve, EleveBourse, EleveDispenseRepas,
    EleveDispositif, EleveDocument, EleveGrilleRepas, EleveOption, ElevePeriode,
    EnregistrerScolariteAnnee, ExigenceDocReinscription, ExporterVolumesRepas, JustifierAbsence,
    LotMonteeIgnore, LotMonteeResultat, MonteeLot, MonteeRapide, OptionAnnee, OptionNature,
    CloreSignalAbsence, EnregistrerPassageBadge, OuvrirCampagneReinscription, PassageRepas,
    PassageSource, PoserDispenseRepas, PoserExigenceDoc, PoserGrilleRepas, PrendreEnChargeSignal,
    SignalAbsence, SignalAbsenceStatut, VolumeRepasJour,
    ReinscriptionCampagne, RepasActivite, RepasActiviteKind, ScolariteAnnee, ScolariteAnneeStatut,
    ScolariteSource, SortirEleve, UpdateIdentite, SEUIL_ABSENCES_DEFAUT, AnnulerSanction,
    CommissionDossier, CommissionVs, ConvoquerCommission, CreerCommissionVs,
    DeciderCommissionDossier,
    CarnetEntree, CreerCarnetEntree, CreerReferentielVs, DeclarerIncident, Incident, IncidentType,
    PrononcerSanction, Sanction, SanctionMotif, SanctionType, SignerCarnetEntree,
    plan_annuler_sanction, plan_convoquer_commission, plan_creer_carnet_entree,
    plan_creer_commission, plan_creer_incident_type, plan_decider_dossier,
    plan_creer_sanction_motif, plan_creer_sanction_type, plan_declarer_incident,
    plan_prononcer_sanction, plan_signer_carnet_entree, solde_points_vs,
    vue_commission_dossier_famille,
    plan_creer_devoir, plan_creer_note_periode, plan_saisir_appreciation, moyenne_sur_20,
    plan_bulletin_eleve, render_bulletin_pdf,
    Appreciation, CreerDevoir, CreerNotePeriode, Devoir, MoyenneMatiere, NoteLue, NotePeriode,
    NotePeriodeStatut, SaisirAppreciation,
    couvrir_echeances, plan_creer_facture, plan_creer_mandat_sepa, plan_echeancier_parts_egales,
    plan_ligne_cantine_depuis_passages,
    plan_emettre_facture, plan_emettre_quittance, plan_encaisser, plan_executer_prelevement,
    plan_candidats_rappel, plan_creer_rappel, plan_exporter_ogec, plan_marquer_exporte,
    enrichir_messages_lues, plan_accuser_lecture, plan_attacher_piece, plan_inviter_compte_eleve,
    plan_ouvrir_fil, plan_planifier_prelevement, plan_poser_echeancier, plan_rejeter_prelevement,
    plan_repondre_fil, plan_revoquer_mandat, render_ogec_csv, render_sepa_csv, render_sepa_pain008,
    corps_alerte_seuil, corps_notif_carnet, reste_a_payer, sujet_alerte_seuil,
    sujet_notif_carnet, sujet_rappel, total_deductions_lignes, ventiler_bourse_sur_echeances,
    AccuserLecture, AttacherPiece, ArchiverFil, CahierTexteEntree, ChangerStatutRdvPsy, ClasserFil,
    ConfigurerEdtCalendrierAb, ContexteModele, CreerCahierTexte, CreerEntretienPsy, CreerFacture,
    CreerMandatSepa, CreerMsgModele, CreerPassageInfirmerie, CreerRdvPsy, DossierPsy, Echeance,
    EcheanceVue, EdtCalendrierAb, EdtException, EdtExceptionKind, EdtSemaineAb, EleveInvitation,
    FicheSante, Encaissement, EncaisserFacture, EntretienPsy, ExporterOgec, Facture, FactureLigne,
    FactureStatut, InviterCompteEleve, MandatSepa, Message, MessageLue, MsgFil, MsgModele,
    MsgPiece, MsgPieceMeta, MoyenPaiement, OuvrirFil, PassageInfirmerie, PlanifierPrelevement,
    PoserEcheancier, PoserEdtException, PrelevementSepa, Quittance, RappelCanal, RappelFacture,
    RdvPsy, RepondreFil, SepaCrediteur, UpsertDossierPsy, UpsertFicheSante, NATURE_CANTINE_BADGE,
    appliquer_modele, creneau_actif_semaine, plan_archiver_fil, plan_changer_statut_rdv_psy,
    plan_classer_fil, plan_configurer_edt_calendrier_ab, plan_courrier_orientation_psy,
    plan_creer_cahier_texte, plan_creer_entretien_psy, plan_creer_msg_modele,
    plan_creer_passage_infirmerie, plan_creer_rdv_psy, plan_poser_edt_exception,
    detect_conflits_edt, grille_edt_defaut, parse_edt_import_csv, parse_edt_pronote_csv,
    solve_edt_greedy_soft, EdtContrainteSoft, EdtDemande, EdtImportLigne, EdtImportParseError,
    EdtImportReport, EdtSolveReport,
    plan_attacher_ordonnance_pai, plan_resume_fiche_sante, plan_signer_pai_famille,
    plan_upsert_dossier_psy, plan_upsert_fiche_sante, plan_upsert_pai,
    render_courrier_orientation_pdf, render_resume_fiche_sante_pdf, semaine_ab_au_jour,
    vue_dossier_famille, vue_entretien_psy_famille, vue_fiche_sante_famille, vue_pai_famille,
    vue_passage_infirmerie_famille, vue_rdv_famille, AttacherOrdonnancePai,
    GenererCourrierOrientationPsy, GenererResumeFicheSante, Pai, PaiPiece, PaiPieceMeta,
    SignerPaiFamille, UpsertPai,
    appliquer_regle_niveau_parent, appliquer_regle_regime_parent, normalize_sortie_regime,
    plan_controle_sortie_avec_contexte, plan_decider_demande_sortie,
    plan_demander_sortie_exceptionnelle, plan_upsert_autorisation_sortie, plan_upsert_sortie_param,
    plan_upsert_sortie_regle_niveau, plan_upsert_sortie_regle_regime, AutorisationSortie,
    ControleSortie, ControleSortieContexteSite, DeciderDemandeSortie,
    DemandeSortieExceptionnelle, DemanderSortieExceptionnelle, RecapSortieEleve, SortieCreneau,
    SortieEdtApercu, SortieParamEtab, SortieRegleNiveau, SortieRegleRegime,
    UpsertAutorisationSortie, UpsertSortieParamEtab, UpsertSortieRegleNiveau,
    UpsertSortieRegleRegime,
    plan_affecter_eleve_internat, plan_clore_affectation_internat, plan_creer_chambre_internat,
    plan_creer_suivi_internat, plan_pointer_presence_internat, AffectationInternat,
    AffecterEleveInternat, ChambreInternat, CreerChambreInternat, CreerSuiviInternat,
    PointerPresenceInternat, PresenceInternat, RecapInternatEleve, SuiviInternat,
    dispense_vs_couvre, plan_clore_dispense_vs, plan_poser_dispense_vs, DispenseVs, PoserDispenseVs,
    plan_enregistrer_journal, EnregistrerJournal, JournalEntree,
    plan_clore_alerte_cpe, plan_creer_alerte_cpe, AlerteCpe, CreerAlerteCpe,
    plan_annuler_doc_a_signer, plan_publier_doc_a_signer, plan_signer_doc_a_signer, DocASigner,
    PublierDocASigner, SignerDocASigner,
    plan_ajouter_dossier_conseil, plan_creer_conseil_classe, plan_decider_dossier_conseil,
    plan_tenir_conseil, AjouterDossierConseil, ConseilClasse, ConseilClasseDossier,
    CreerConseilClasse, DeciderDossierConseil,
    plan_creer_info_comp_champ, plan_upsert_info_comp_valeur, CreerInfoCompChamp, InfoCompChamp,
    InfoCompValeur, UpsertInfoCompValeur,
    jour_est_ferme, plan_creer_periode_vacances, CreerPeriodeVacances, PeriodeVacances,
    plan_rechercher_homonymes, score_homonyme, HomonymeCandidat, RechercherHomonymes,
    plan_upsert_menu_cantine, MenuCantineJour, UpsertMenuCantine,
    plan_upsert_quotient_foyer, QuotientFoyer, UpsertQuotientFoyer,
    plan_creer_taux_tva, ttc_depuis_ht, CreerTauxTva, TauxTva,
    plan_poser_photo_eleve, PoserPhotoEleve, TrombiEntree,
    plan_upsert_param_etab, ParamEtab, UpsertParamEtab,
    plan_creer_salle, CreerSalle, Salle,
    plan_creer_filiere, plan_poser_voeu, CreerFiliere, Filiere, PoserVoeu, VoeuInscription,
    plan_upsert_inscription_suivi, InscriptionSuivi, UpsertInscriptionSuivi,
    plan_exporter_lsl, render_lsl_csv, ExporterLsl, LigneLsl,
    plan_exporter_onde, render_onde_csv, ExporterOnde, LigneOnde,
    plan_creer_tarif_conso, CreerTarifConso, TarifConso,
    plan_exporter_notanet, render_notanet_csv, ExporterNotanet, LigneNotanet,
    plan_exporter_parcoursup, render_parcoursup_csv, ExporterParcoursup, LigneParcoursup,
    plan_declarer_absence_rh, plan_valider_absence_rh, AbsenceRh, DeclarerAbsenceRh,
    plan_affecter_edt_creneau, AffecterEdtCreneau,
    plan_creer_edition_vs, rendre_edition_vs, CreerEditionVs, EditionVsTemplate, RendreEditionVs,
    domaine_depuis_matiere, plan_exporter_lsun, render_lsun_csv, ExporterLsun, LigneLsun,
    plan_creer_convention_stage, plan_creer_entreprise, ConventionStage, CreerConventionStage,
    CreerEntreprise, Entreprise,
    plan_creer_edt_contrainte, CreerEdtContrainte, EdtContrainte,
};
use ent_auth::{AuthError, Role};
use ent_brain::{react, BrainAction, BrainP0};
use ent_events::{types, AggregateKind, EventBusError, EventHandler, MetierEvent, MetierEventDraft};
use ent_mail::{MailAttachment, MailService, OutgoingMail};
use ent_sms::{OutgoingSms, SmsService};
use ent_push::{OutgoingPush, PushPlatform, PushService};
use ent_bank::{plan_action_ligne, BankConnector, BankMatchAction, BankParseError};
use ent_object::{ObjectStore, ObjectStoreError};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum EntDbError {
    #[error(transparent)]
    Url(#[from] UrlGuardError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Event(#[from] EventBusError),
    #[error(transparent)]
    Migrate(#[from] crate::migrate::MigrateError),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error("accès refusé pour ce rôle")]
    Forbidden,
    #[error("code ou libellé matière vide")]
    MatiereInvalide,
    #[error("note hors barème")]
    NoteInvalide,
    #[error("appel déjà clos ou introuvable")]
    AppelInvalide,
    #[error("motif d'absence inconnu pour cet établissement")]
    MotifInconnu,
    #[error("code motif invalide")]
    MotifInvalide,
    #[error(transparent)]
    Bank(#[from] BankParseError),
    #[error(transparent)]
    Object(#[from] ObjectStoreError),
    #[error(transparent)]
    EdtImport(#[from] EdtImportParseError),
    #[error("import EDT refusé : conflits détectés")]
    EdtImportConflicts,
}

/// Résumé d'un ingest relevé bancaire (rapprochement SEPA).
#[derive(Debug, Clone, Default)]
pub struct BankRapprochementSummary {
    pub lines: usize,
    pub executed: usize,
    pub rejected: usize,
    pub unmatched: usize,
    pub errors: usize,
}

/// Enfant vu depuis le portail famille (lien + flags).
#[derive(Debug, Clone)]
pub struct EnfantFamille {
    pub eleve: Eleve,
    pub lien: String,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
}

/// Façade async Postgres — auth + noyau élève (+ périodes + absences) + events + Brain.
pub struct PgEnt {
    pool: PgPool,
    eleves: PgEleveRepo,
    absences: PgAbsenceRepo,
    periodes: PgPeriodeRepo,
    foyers: PgFoyerRepo,
    matieres: PgMatiereRepo,
    notes: PgNoteRepo,
    appels: PgAppelRepo,
    motifs: PgMotifRepo,
    signaux: PgSignalRepo,
    eleve_comptes: PgEleveCompteRepo,
    options: PgOptionRepo,
    documents: PgDocumentRepo,
    repas: PgRepasRepo,
    continuum: PgContinuumRepo,
    sanctions: PgSanctionRepo,
    sorties: PgSortieRepo,
    internat: PgInternatRepo,
    dispenses_vs: PgDispenseVsRepo,
    journal: PgJournalRepo,
    alertes_cpe: PgAlerteCpeRepo,
    docs_signer: PgDocSignerRepo,
    conseils: PgConseilClasseRepo,
    info_comp: PgInfoCompRepo,
    vacances: PgVacancesRepo,
    menus: PgMenuCantineRepo,
    quotients: PgQuotientRepo,
    tva: PgTvaRepo,
    params: PgParamEtabRepo,
    salles: PgSalleRepo,
    filieres: PgFiliereRepo,
    inscriptions: PgInscriptionRepo,
    iban_histo: PgIbanHistoRepo,
    tarifs: PgTarifConsoRepo,
    absences_rh: PgAbsenceRhRepo,
    editions_vs: PgEditionVsRepo,
    stages: PgStageRepo,
    edt_contraintes: PgEdtContrainteRepo,
    carnet: PgCarnetRepo,
    cahier_texte: PgCahierTexteRepo,
    infirmerie: PgInfirmerieRepo,
    psy: PgPsyRepo,
    edt: PgEdtRepo,
    messages: PgMsgRepo,
    facturation: PgFacRepo,
    mail_outbox: PgMailRepo,
    mailer: MailService,
    sms_outbox: PgSmsRepo,
    sms: SmsService,
    push_devices: PgPushRepo,
    push: PushService,
    bank_repo: BankRepo,
    bank: BankConnector,
    objects: ObjectStore,
    auth: PgAuthRepo,
    events: PgEventStore,
    brain: Option<BrainP0>,
}

impl PgEnt {
    /// Connexion **uniquement** si l'URL est localhost / 127.0.0.1.
    pub async fn connect(database_url: &str) -> Result<Self, EntDbError> {
        assert_local_postgres_url(database_url)?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self::from_pool(pool))
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self {
            eleves: PgEleveRepo::new(pool.clone()),
            absences: PgAbsenceRepo::new(pool.clone()),
            periodes: PgPeriodeRepo::new(pool.clone()),
            foyers: PgFoyerRepo::new(pool.clone()),
            matieres: PgMatiereRepo::new(pool.clone()),
            notes: PgNoteRepo::new(pool.clone()),
            appels: PgAppelRepo::new(pool.clone()),
            motifs: PgMotifRepo::new(pool.clone()),
            signaux: PgSignalRepo::new(pool.clone()),
            eleve_comptes: PgEleveCompteRepo::new(pool.clone()),
            options: PgOptionRepo::new(pool.clone()),
            documents: PgDocumentRepo::new(pool.clone()),
            repas: PgRepasRepo::new(pool.clone()),
            continuum: PgContinuumRepo::new(pool.clone()),
            sanctions: PgSanctionRepo::new(pool.clone()),
            sorties: PgSortieRepo::new(pool.clone()),
            internat: PgInternatRepo::new(pool.clone()),
            dispenses_vs: PgDispenseVsRepo::new(pool.clone()),
            journal: PgJournalRepo::new(pool.clone()),
            alertes_cpe: PgAlerteCpeRepo::new(pool.clone()),
            docs_signer: PgDocSignerRepo::new(pool.clone()),
            conseils: PgConseilClasseRepo::new(pool.clone()),
            info_comp: PgInfoCompRepo::new(pool.clone()),
            vacances: PgVacancesRepo::new(pool.clone()),
            menus: PgMenuCantineRepo::new(pool.clone()),
            quotients: PgQuotientRepo::new(pool.clone()),
            tva: PgTvaRepo::new(pool.clone()),
            params: PgParamEtabRepo::new(pool.clone()),
            salles: PgSalleRepo::new(pool.clone()),
            filieres: PgFiliereRepo::new(pool.clone()),
            inscriptions: PgInscriptionRepo::new(pool.clone()),
            iban_histo: PgIbanHistoRepo::new(pool.clone()),
            tarifs: PgTarifConsoRepo::new(pool.clone()),
            absences_rh: PgAbsenceRhRepo::new(pool.clone()),
            editions_vs: PgEditionVsRepo::new(pool.clone()),
            stages: PgStageRepo::new(pool.clone()),
            edt_contraintes: PgEdtContrainteRepo::new(pool.clone()),
            carnet: PgCarnetRepo::new(pool.clone()),
            cahier_texte: PgCahierTexteRepo::new(pool.clone()),
            infirmerie: PgInfirmerieRepo::new(pool.clone()),
            psy: PgPsyRepo::new(pool.clone()),
            edt: PgEdtRepo::new(pool.clone()),
            messages: PgMsgRepo::new(pool.clone()),
            facturation: PgFacRepo::new(pool.clone()),
            mail_outbox: PgMailRepo::new(pool.clone()),
            mailer: MailService::from_env(),
            sms_outbox: PgSmsRepo::new(pool.clone()),
            sms: SmsService::from_env(),
            push_devices: PgPushRepo::new(pool.clone()),
            push: PushService::from_env(),
            bank_repo: BankRepo::new(pool.clone()),
            bank: BankConnector::from_env(),
            objects: ObjectStore::from_env(),
            auth: PgAuthRepo::new(pool.clone()),
            events: PgEventStore::new(pool.clone()),
            pool,
            brain: None,
        }
    }

    /// Lab : force dry-run SMTP / SMS / push + banque + objet mémoire.
    pub fn install_mail_dry_run(&mut self) {
        self.mailer = MailService::dry_run_only();
        self.sms = SmsService::dry_run_only();
        self.push = PushService::dry_run_only();
        self.bank = BankConnector::dry_run_only();
        // Respecte ENT_OBJECT_S3 / ENT_OBJECT_S3_LIVE / ENT_OBJECT_MEMORY / ENT_OBJECT_DIR.
        self.objects = ObjectStore::from_env();
        if !self.objects.is_memory()
            && !self.objects.is_s3_stub()
            && !self.objects.is_s3_live()
        {
            self.objects = ObjectStore::memory_only();
        }
    }

    pub fn mailer(&self) -> &MailService {
        &self.mailer
    }

    pub fn sms(&self) -> &SmsService {
        &self.sms
    }

    pub fn push(&self) -> &PushService {
        &self.push
    }

    pub fn bank(&self) -> &BankConnector {
        &self.bank
    }

    pub fn objects(&self) -> &ObjectStore {
        &self.objects
    }

    pub async fn set_user_telephone(
        &self,
        user_id: Uuid,
        telephone: &str,
    ) -> Result<(), EntDbError> {
        Ok(self.auth.set_telephone(user_id, telephone).await?)
    }

    pub async fn migrate(&self) -> Result<(), EntDbError> {
        apply_socle_migrations(&self.pool).await?;
        Ok(())
    }

    pub fn install_brain_p0(&mut self) -> &BrainP0 {
        let brain = BrainP0::install();
        self.events.register_handler(brain.runtime.clone());
        self.brain = Some(brain);
        self.brain.as_ref().expect("brain just installed")
    }

    pub fn brain(&self) -> Option<&BrainP0> {
        self.brain.as_ref()
    }

    pub fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        self.events.register_handler(handler);
    }

    /// Notifie les parents liés par e-mail (dry-run ou live). Best-effort.
    async fn notify_parents_mail(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        kind: &str,
        subject: &str,
        body_text: &str,
        source_aggregate: &str,
        source_id: Uuid,
        actor_user_id: Option<Uuid>,
        attachments: Vec<MailAttachment>,
    ) -> Result<usize, EntDbError> {
        if !self.mailer.is_ready() {
            return Ok(0);
        }
        // Anti-doublon : déjà journalisé pour cette source + kind → skip.
        if self
            .mail_outbox
            .count_by_source_kind(etablissement_id, source_id, kind)
            .await?
            > 0
        {
            return Ok(0);
        }
        let emails = self
            .foyers
            .emails_parents_eleve(etablissement_id, eleve_id)
            .await?;
        let mut n = 0usize;
        for to in emails {
            let receipt = match self
                .mailer
                .send(OutgoingMail {
                    to: to.clone(),
                    subject: subject.to_string(),
                    body_text: body_text.to_string(),
                    attachments: attachments.clone(),
                })
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[ent-db] mail send error ({kind}): {e}");
                    continue;
                }
            };
            let row = self
                .mail_outbox
                .insert(InsertMailOutbox {
                    etablissement_id,
                    eleve_id: Some(eleve_id),
                    kind: kind.to_string(),
                    to_email: to,
                    subject: subject.to_string(),
                    body_text: body_text.to_string(),
                    status: receipt.status,
                    error: receipt.error.clone(),
                    source_aggregate: Some(source_aggregate.to_string()),
                    source_id: Some(source_id),
                    message_id: receipt.message_id.clone(),
                })
                .await?;
            let _ = self
                .events
                .publish(MetierEventDraft {
                    etablissement_id,
                    type_name: types::MAIL_ENVOYE.to_string(),
                    aggregate: AggregateKind::Mail,
                    aggregate_id: row.id,
                    eleve_id: Some(eleve_id),
                    payload: json!({
                        "kind": kind,
                        "status": row.status,
                        "to": row.to_email,
                        "subject": subject,
                        "source_aggregate": source_aggregate,
                        "source_id": source_id,
                    }),
                    actor_user_id: actor_user_id.map(|u| u.to_string()),
                })
                .await;
            n += 1;
        }
        Ok(n)
    }

    pub async fn count_mails_kind(
        &self,
        etablissement_id: Uuid,
        kind: &str,
    ) -> Result<i64, EntDbError> {
        Ok(self.mail_outbox.count_by_kind(etablissement_id, kind).await?)
    }

    pub async fn list_mails_recents(
        &self,
        etablissement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MailOutboxRow>, EntDbError> {
        Ok(self.mail_outbox.list_recent(etablissement_id, limit).await?)
    }

    pub async fn list_mails_recents_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        limit: i64,
    ) -> Result<Vec<MailOutboxRow>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let lim = limit.clamp(1, 200);
        Ok(self.mail_outbox.list_recent(etablissement_id, lim).await?)
    }

    pub async fn list_sms_recents_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        limit: i64,
    ) -> Result<Vec<crate::sms_repo::SmsOutboxRow>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let lim = limit.clamp(1, 200);
        Ok(self.sms_outbox.list_recent(etablissement_id, lim).await?)
    }

    /// SMS aux parents liés (dry-run ou live). Best-effort.
    async fn notify_parents_sms(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        kind: &str,
        body: &str,
        source_aggregate: &str,
        source_id: Uuid,
        actor_user_id: Option<Uuid>,
    ) -> Result<usize, EntDbError> {
        if !self.sms.is_ready() {
            return Ok(0);
        }
        if self
            .sms_outbox
            .count_by_source_kind(etablissement_id, source_id, kind)
            .await?
            > 0
        {
            return Ok(0);
        }
        let phones = self
            .foyers
            .telephones_parents_eleve(etablissement_id, eleve_id)
            .await?;
        let mut n = 0usize;
        for to in phones {
            let receipt = match self
                .sms
                .send(OutgoingSms {
                    to: to.clone(),
                    body: body.to_string(),
                })
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[ent-db] sms send error ({kind}): {e}");
                    continue;
                }
            };
            let row = self
                .sms_outbox
                .insert(InsertSmsOutbox {
                    etablissement_id,
                    eleve_id: Some(eleve_id),
                    kind: kind.to_string(),
                    to_phone: receipt.to.clone(),
                    body: body.to_string(),
                    status: receipt.status,
                    error: receipt.error.clone(),
                    source_aggregate: Some(source_aggregate.to_string()),
                    source_id: Some(source_id),
                    message_id: receipt.message_id.clone(),
                })
                .await?;
            let _ = self
                .events
                .publish(MetierEventDraft {
                    etablissement_id,
                    type_name: types::SMS_ENVOYE.to_string(),
                    aggregate: AggregateKind::Sms,
                    aggregate_id: row.id,
                    eleve_id: Some(eleve_id),
                    payload: json!({
                        "kind": kind,
                        "status": row.status,
                        "to": row.to_phone,
                        "source_aggregate": source_aggregate,
                        "source_id": source_id,
                    }),
                    actor_user_id: actor_user_id.map(|u| u.to_string()),
                })
                .await;
            n += 1;
        }
        Ok(n)
    }

    pub async fn count_sms_kind(
        &self,
        etablissement_id: Uuid,
        kind: &str,
    ) -> Result<i64, EntDbError> {
        Ok(self.sms_outbox.count_by_kind(etablissement_id, kind).await?)
    }

    pub async fn register_push_device_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        platform: &str,
        token: &str,
        app_id: Option<String>,
    ) -> Result<PushDevice, EntDbError> {
        let platform = PushPlatform::parse(platform).ok_or(EntDbError::Forbidden)?;
        let device = self
            .push_devices
            .upsert_device(RegisterPushDevice {
                etablissement_id,
                user_id,
                platform,
                token: token.to_string(),
                app_id,
            })
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::PUSH_DEVICE_REGISTERED.to_string(),
                aggregate: AggregateKind::Push,
                aggregate_id: device.id,
                eleve_id: None,
                payload: json!({
                    "platform": device.platform,
                    "user_id": user_id,
                }),
                actor_user_id: Some(user_id.to_string()),
            })
            .await?;
        Ok(device)
    }

    /// Push aux parents liés qui ont un device enregistré. Best-effort.
    async fn notify_parents_push(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        kind: &str,
        title: &str,
        body: &str,
        source_aggregate: &str,
        source_id: Uuid,
        actor_user_id: Option<Uuid>,
    ) -> Result<usize, EntDbError> {
        if !self.push.is_ready() {
            return Ok(0);
        }
        if self
            .push_devices
            .count_by_source_kind(etablissement_id, source_id, kind)
            .await?
            > 0
        {
            return Ok(0);
        }
        let liens = self
            .foyers
            .list_liens_eleve(etablissement_id, eleve_id)
            .await?;
        let user_ids: Vec<Uuid> = liens.iter().map(|l| l.user_id).collect();
        let devices = self
            .push_devices
            .list_active_for_users(etablissement_id, &user_ids)
            .await?;
        let mut n = 0usize;
        for device in devices {
            // Ne pas notifier l'auteur du geste (ex. parent qui ouvre un fil).
            if actor_user_id == Some(device.user_id) {
                continue;
            }
            let platform = PushPlatform::parse(&device.platform).unwrap_or(PushPlatform::Ios);
            let receipt = match self
                .push
                .send(OutgoingPush {
                    token: device.token.clone(),
                    platform,
                    title: title.to_string(),
                    body: body.to_string(),
                    data_json: Some(format!(
                        "{{\"kind\":\"{kind}\",\"eleve_id\":\"{eleve_id}\"}}"
                    )),
                })
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[ent-db] push send error ({kind}): {e}");
                    continue;
                }
            };
            let outbox_id = self
                .push_devices
                .insert_outbox(InsertPushOutbox {
                    etablissement_id,
                    eleve_id: Some(eleve_id),
                    user_id: Some(device.user_id),
                    kind: kind.to_string(),
                    title: title.to_string(),
                    body: body.to_string(),
                    platform: Some(device.platform.clone()),
                    token: Some(device.token.clone()),
                    status: receipt.status,
                    error: receipt.error.clone(),
                    source_aggregate: Some(source_aggregate.to_string()),
                    source_id: Some(source_id),
                    message_id: receipt.message_id.clone(),
                })
                .await?;
            let _ = self
                .events
                .publish(MetierEventDraft {
                    etablissement_id,
                    type_name: types::PUSH_ENVOYE.to_string(),
                    aggregate: AggregateKind::Push,
                    aggregate_id: outbox_id,
                    eleve_id: Some(eleve_id),
                    payload: json!({
                        "kind": kind,
                        "status": match receipt.status {
                            ent_push::PushSendStatus::DryRun => "dry_run",
                            ent_push::PushSendStatus::Sent => "sent",
                            ent_push::PushSendStatus::SkippedNoConfig => "skipped_no_config",
                            ent_push::PushSendStatus::Failed => "failed",
                        },
                        "platform": device.platform,
                        "source_id": source_id,
                    }),
                    actor_user_id: actor_user_id.map(|u| u.to_string()),
                })
                .await;
            n += 1;
        }
        Ok(n)
    }

    pub async fn count_push_kind(
        &self,
        etablissement_id: Uuid,
        kind: &str,
    ) -> Result<i64, EntDbError> {
        Ok(self
            .push_devices
            .count_by_kind(etablissement_id, kind)
            .await?)
    }

    pub async fn register_user(
        &self,
        email: &str,
        password: &str,
        display_name: Option<&str>,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<AuthUser, EntDbError> {
        Ok(self
            .auth
            .register(email, password, display_name, etablissement_id, role)
            .await?)
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
        etablissement_id: Uuid,
    ) -> Result<SessionInfo, EntDbError> {
        Ok(self.auth.login(email, password, etablissement_id).await?)
    }

    pub async fn resolve_session(&self, authorization: &str) -> Result<SessionInfo, EntDbError> {
        Ok(self.auth.resolve_bearer(authorization).await?)
    }

    pub async fn logout(&self, authorization: &str) -> Result<(), EntDbError> {
        Ok(self.auth.revoke_bearer(authorization).await?)
    }

    pub async fn create_eleve(&self, cmd: CreateEleve) -> Result<Eleve, EntDbError> {
        let planned = plan_create(cmd)?;
        self.eleves.upsert(&planned.eleve).await?;
        for p in &planned.periodes_initiales {
            self.periodes
                .apply_cutover(&CutoverOp::OpenFirst { period: p.clone() })
                .await?;
        }
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        Ok(planned.eleve)
    }

    pub async fn change_classe(&self, cmd: ChangeClasse) -> Result<Eleve, EntDbError> {
        self.change_attribut(ChangeAttribut {
            etablissement_id: cmd.etablissement_id,
            eleve_id: cmd.eleve_id,
            kind: AttributKind::Classe,
            nouvelle_valeur: cmd.nouvelle_classe,
            date_effet: cmd.date_effet,
            actor_user_id: cmd.actor_user_id,
        })
        .await
    }

    pub async fn change_regime(&self, cmd: ChangeRegime) -> Result<Eleve, EntDbError> {
        self.change_attribut(ChangeAttribut {
            etablissement_id: cmd.etablissement_id,
            eleve_id: cmd.eleve_id,
            kind: AttributKind::Regime,
            nouvelle_valeur: cmd.nouveau_regime,
            date_effet: cmd.date_effet,
            actor_user_id: cmd.actor_user_id,
        })
        .await
    }

    pub async fn change_mef(&self, cmd: ChangeMef) -> Result<Eleve, EntDbError> {
        self.change_attribut(ChangeAttribut {
            etablissement_id: cmd.etablissement_id,
            eleve_id: cmd.eleve_id,
            kind: AttributKind::Mef,
            nouvelle_valeur: cmd.nouveau_mef,
            date_effet: cmd.date_effet,
            actor_user_id: cmd.actor_user_id,
        })
        .await
    }

    pub async fn change_site(&self, cmd: ChangeSite) -> Result<Eleve, EntDbError> {
        self.change_attribut(ChangeAttribut {
            etablissement_id: cmd.etablissement_id,
            eleve_id: cmd.eleve_id,
            kind: AttributKind::Site,
            nouvelle_valeur: cmd.nouveau_site,
            date_effet: cmd.date_effet,
            actor_user_id: cmd.actor_user_id,
        })
        .await
    }

    pub async fn update_identite(&self, cmd: UpdateIdentite) -> Result<Eleve, EntDbError> {
        let current = self
            .eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id.clone();
        let (eleve, changed) = plan_update_identite(&current, cmd)?;
        if !changed {
            return Ok(eleve);
        }
        self.eleves.upsert(&eleve).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: eleve.etablissement_id,
                type_name: types::ELEVE_IDENTITY_CHANGED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve.id,
                eleve_id: Some(eleve.id),
                payload: json!({
                    "nom": eleve.nom,
                    "prenom": eleve.prenom,
                    "sexe": eleve.sexe,
                    "date_naissance": eleve.date_naissance.map(|d| d.to_string()),
                    "numero_badge": eleve.numero_badge,
                    "redoublant": eleve.redoublant,
                    "date_entree": eleve.date_entree.map(|d| d.to_string()),
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(eleve)
    }

    pub async fn ajouter_option(&self, cmd: AjouterOption) -> Result<EleveOption, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id.clone();
        let opt = plan_ajouter_option(cmd)?;
        self.options.upsert(&opt).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: opt.etablissement_id,
                type_name: types::ELEVE_OPTION_ADDED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: opt.eleve_id,
                eleve_id: Some(opt.eleve_id),
                payload: json!({
                    "nature": opt.nature.as_str(),
                    "code": opt.code,
                    "libelle": opt.libelle,
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(opt)
    }

    pub async fn search_eleves(
        &self,
        etablissement_id: Uuid,
        q: &str,
        limit: i64,
    ) -> Result<Vec<Eleve>, EntDbError> {
        Ok(self.eleves.search(etablissement_id, q, limit).await?)
    }

    pub async fn search_staff(
        &self,
        etablissement_id: Uuid,
        q: &str,
        limit: i64,
    ) -> Result<Vec<StaffMember>, EntDbError> {
        Ok(self.auth.search_staff(etablissement_id, q, limit).await?)
    }

    pub async fn get_staff_member(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<StaffMember>, EntDbError> {
        Ok(self.auth.get_staff_member(etablissement_id, user_id).await?)
    }

    pub async fn user_labels(
        &self,
        user_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, EntDbError> {
        Ok(self.auth.user_labels(user_ids).await?)
    }

    pub async fn document_labels(
        &self,
        etablissement_id: Uuid,
        document_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, EntDbError> {
        Ok(self
            .documents
            .document_labels(etablissement_id, document_ids)
            .await?)
    }

    pub async fn edt_creneau_labels(
        &self,
        etablissement_id: Uuid,
        creneau_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, EntDbError> {
        Ok(self
            .edt
            .creneau_labels(etablissement_id, creneau_ids)
            .await?)
    }

    pub async fn eleve_labels(
        &self,
        etablissement_id: Uuid,
        eleve_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, EntDbError> {
        Ok(self.eleves.labels(etablissement_id, eleve_ids).await?)
    }

    pub async fn entreprise_labels(
        &self,
        etablissement_id: Uuid,
        entreprise_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, EntDbError> {
        Ok(self
            .stages
            .entreprise_labels(etablissement_id, entreprise_ids)
            .await?)
    }

    /// Vue dossier complète filtrée selon le rôle (module monté avant le lab).
    pub async fn get_dossier_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<DossierEleve>, EntDbError> {
        let Some(eleve) = self
            .get_eleve_for_actor(etablissement_id, actor_user_id, role, eleve_id)
            .await?
        else {
            return Ok(None);
        };
        let sections = self
            .dossier_sections_for_actor(etablissement_id, actor_user_id, role, eleve_id)
            .await?;

        let periodes = if sections.scolarite {
            self.periodes.list(etablissement_id, eleve_id).await?
        } else {
            Vec::new()
        };
        let options = if sections.options {
            self.options.list_for_eleve(etablissement_id, eleve_id).await?
        } else {
            Vec::new()
        };
        let liens = if sections.famille {
            self.foyers
                .list_liens_dossier(etablissement_id, eleve_id)
                .await?
        } else {
            Vec::new()
        };
        let documents = if sections.documents {
            self.documents_visibles_for_actor(
                etablissement_id,
                actor_user_id,
                role,
                eleve_id,
            )
            .await?
        } else {
            Vec::new()
        };
        let dispositifs = if sections.dispositifs {
            self.documents
                .list_dispositifs(etablissement_id, eleve_id)
                .await?
        } else {
            Vec::new()
        };
        let grille_repas = if sections.repas {
            self.repas
                .list_grilles_eleve(etablissement_id, eleve_id)
                .await?
        } else {
            Vec::new()
        };
        let dispenses_repas = if sections.repas {
            self.repas
                .list_dispenses_eleve(etablissement_id, eleve_id, true)
                .await?
        } else {
            Vec::new()
        };
        let bourses = if sections.bourses {
            self.repas
                .list_bourses_eleve(etablissement_id, eleve_id)
                .await?
        } else {
            Vec::new()
        };
        let (continuum, options_continuum) = if sections.continuum {
            let (nm1, n, np1) = annees_continuum(jour);
            let annees = [nm1.as_str(), n.as_str(), np1.as_str()];
            let cont = self
                .continuum
                .list_scolarites_continuum(etablissement_id, eleve_id, &annees)
                .await?;
            let opts = self
                .continuum
                .list_options_continuum(etablissement_id, eleve_id, &annees)
                .await?;
            (cont, opts)
        } else {
            (Vec::new(), Vec::new())
        };
        let (sanctions, incidents, carnet) = if sections.vs {
            let famille_only = Role::parse(role)
                .map(|r| matches!(r, Role::Parent | Role::Eleve))
                .unwrap_or(false);
            (
                self.sanctions
                    .list_sanctions_eleve(etablissement_id, eleve_id)
                    .await?,
                self.sanctions
                    .list_incidents_eleve(etablissement_id, eleve_id)
                    .await?,
                self.carnet
                    .list_for_eleve(etablissement_id, eleve_id, famille_only)
                    .await?,
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };
        let notes = if sections.notes {
            let matieres = self.matieres.list(etablissement_id).await?;
            let mat_by_id: std::collections::HashMap<Uuid, String> =
                matieres.into_iter().map(|m| (m.id, m.libelle)).collect();
            self.notes
                .list_for_eleve(etablissement_id, eleve_id)
                .await?
                .into_iter()
                .map(|n| NoteLue {
                    id: n.id,
                    matiere_id: n.matiere_id,
                    matiere_libelle: mat_by_id.get(&n.matiere_id).cloned(),
                    valeur: n.valeur,
                    sur: n.sur,
                    date_note: n.date_note,
                    libelle: n.libelle,
                    devoir_id: n.devoir_id,
                    periode_id: n.periode_id,
                })
                .collect()
        } else {
            Vec::new()
        };
        let age_ans = eleve.date_naissance.map(|dn| age_ans_au(dn, jour));

        Ok(Some(DossierEleve {
            eleve,
            age_ans,
            periodes,
            options,
            liens,
            documents,
            dispositifs,
            grille_repas,
            dispenses_repas,
            bourses,
            continuum,
            options_continuum,
            sanctions,
            incidents,
            carnet,
            notes,
            sections,
        }))
    }

    async fn dossier_sections_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<DossierSections, EntDbError> {
        let none = DossierSections {
            identite: false,
            scolarite: false,
            famille: false,
            options: false,
            notes: false,
            vs: false,
            documents: false,
            dispositifs: false,
            repas: false,
            bourses: false,
            continuum: false,
        };
        let Some(parsed) = Role::parse(role) else {
            return Ok(none);
        };
        if parsed.can_manage_eleves()
            || parsed == Role::Admin
            || parsed == Role::Direction
            || parsed == Role::Compta
        {
            return Ok(DossierSections {
                identite: true,
                scolarite: true,
                famille: true,
                options: true,
                notes: true,
                vs: true,
                documents: true,
                dispositifs: true,
                repas: true,
                bourses: true,
                continuum: true,
            });
        }
        if parsed.can_read_all_eleves() {
            let notes = parsed.can_read_notes_staff();
            let vs = parsed.can_lire_vs_absences();
            // Prof / CPE / infirmière : docs filtrés (grants) ; bourses = finance → non.
            return Ok(DossierSections {
                identite: true,
                scolarite: true,
                famille: matches!(parsed, Role::Cpe | Role::Administratif | Role::Prof),
                options: true,
                notes,
                vs,
                documents: true,
                dispositifs: true,
                repas: true,
                bourses: false,
                continuum: true,
            });
        }
        if parsed == Role::Parent {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            let Some(l) = lien else {
                return Ok(none);
            };
            return Ok(DossierSections {
                identite: true,
                scolarite: true,
                famille: true,
                options: l.peut_voir_notes,
                notes: l.peut_voir_notes,
                vs: l.peut_voir_vs,
                documents: true,
                dispositifs: false,
                repas: true,
                bourses: true,
                continuum: true,
            });
        }
        if parsed == Role::Eleve {
            return Ok(DossierSections {
                identite: true,
                scolarite: true,
                famille: false,
                options: true,
                notes: true,
                vs: false,
                documents: true,
                dispositifs: false,
                repas: true,
                bourses: false,
                continuum: true,
            });
        }
        Ok(none)
    }

    pub async fn change_attribut(&self, cmd: ChangeAttribut) -> Result<Eleve, EntDbError> {
        let current = self
            .eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let open = self
            .periodes
            .open(cmd.etablissement_id, cmd.eleve_id, cmd.kind)
            .await?;
        let planned = plan_cutover(&current, open.as_ref(), cmd, today_utc())?;
        self.periodes.apply_cutover(&planned.op).await?;
        self.eleves.upsert(&planned.eleve).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        Ok(planned.eleve)
    }

    pub async fn change_status(&self, cmd: ChangeStatus) -> Result<Eleve, EntDbError> {
        let current = self
            .eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_change_status(&current, cmd)?;
        self.eleves.upsert(&planned.eleve).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        Ok(planned.eleve)
    }

    pub async fn sortir(&self, cmd: SortirEleve) -> Result<Eleve, EntDbError> {
        let current = self
            .eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_sortir(&current, cmd, today_utc())?;
        if let Some(fin) = planned.close_periodes_au {
            self.periodes
                .close_open_periodes(planned.eleve.etablissement_id, planned.eleve.id, fin)
                .await?;
        }
        self.eleves.upsert(&planned.eleve).await?;
        self.events.publish(planned.event).await?;
        Ok(planned.eleve)
    }

    pub async fn get_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<Eleve>, EntDbError> {
        Ok(self.eleves.get(etablissement_id, eleve_id).await?)
    }

    pub async fn list_periodes(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<ElevePeriode>, EntDbError> {
        Ok(self.periodes.list(etablissement_id, eleve_id).await?)
    }

    /// Effectif = SELECT daté sur `ent_eleve` + `ent_eleve_periode` (zéro doublon).
    pub async fn eleves_en_classe_au(
        &self,
        etablissement_id: Uuid,
        classe: &str,
        jour: NaiveDate,
    ) -> Result<Vec<Eleve>, EntDbError> {
        Ok(self
            .eleves
            .list_en_classe_au(etablissement_id, classe, jour)
            .await?)
    }

    pub async fn declare_absence(&self, cmd: DeclareAbsence) -> Result<Absence, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !self
            .motifs
            .est_connu(cmd.etablissement_id, cmd.motif.trim())
            .await?
        {
            return Err(EntDbError::MotifInconnu);
        }
        let planned = plan_declare_absence(cmd)?;
        self.absences.upsert(&planned.absence).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        self.apres_absence_ecrite(
            planned.absence.etablissement_id,
            planned.absence.eleve_id,
            planned.absence.debut,
        )
        .await?;
        Ok(planned.absence)
    }

    pub async fn justifier_absence(
        &self,
        cmd: JustifierAbsence,
    ) -> Result<Absence, EntDbError> {
        let current = self
            .absences
            .get(cmd.etablissement_id, cmd.absence_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_justifier_absence(&current, cmd)?;
        self.absences.upsert(&planned.absence).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        Ok(planned.absence)
    }

    /// Authz + justif : charge l'absence, vérifie le droit sur l'élève, applique.
    pub async fn justifier_absence_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        absence_id: Uuid,
        motif_justif: String,
    ) -> Result<Absence, EntDbError> {
        let current = self
            .absences
            .get(etablissement_id, absence_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let allowed = self
            .actor_can_justifier_absence(
                etablissement_id,
                actor_user_id,
                role,
                current.eleve_id,
            )
            .await?;
        if !allowed {
            return Err(EntDbError::Forbidden);
        }
        self.justifier_absence(JustifierAbsence {
            etablissement_id,
            absence_id,
            motif_justif,
            actor_user_id,
        })
        .await
    }

    pub async fn list_absences_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Absence>, EntDbError> {
        Ok(self
            .absences
            .list_for_eleve(etablissement_id, eleve_id)
            .await?)
    }

    pub async fn list_absences_a_traiter(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Absence>, EntDbError> {
        Ok(self
            .absences
            .list_a_traiter_jour(etablissement_id, jour)
            .await?)
    }

    /// Parent lié avec flag justif, ou staff absences.
    pub async fn actor_can_justifier_absence(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        self.actor_can_declare_absence(etablissement_id, actor_user_id, role, eleve_id)
            .await
    }

    pub async fn actor_can_read_absences(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(false);
        };
        if parsed.can_lire_vs_absences() {
            return Ok(true);
        }
        if parsed == Role::Parent {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            return Ok(lien.map(|l| l.peut_voir_vs).unwrap_or(false));
        }
        if parsed == Role::Eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            return Ok(linked == Some(eleve_id));
        }
        Ok(false)
    }

    /// Lecture partagée (module absences, cantine, …) — même `ent_absence`.
    pub async fn eleve_est_absent(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        Ok(self
            .absences
            .est_absent(etablissement_id, eleve_id, jour)
            .await?)
    }

    pub async fn events_for_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<MetierEvent>, EntDbError> {
        Ok(self
            .events
            .events_for_eleve(etablissement_id, eleve_id)
            .await?)
    }

    pub async fn create_foyer(
        &self,
        etablissement_id: Uuid,
        libelle: Option<&str>,
    ) -> Result<Uuid, EntDbError> {
        Ok(self.foyers.create_foyer(etablissement_id, libelle).await?)
    }

    pub async fn create_foyer_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        libelle: Option<&str>,
    ) -> Result<Uuid, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        self.create_foyer(etablissement_id, libelle).await
    }

    pub async fn set_user_telephone_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        user_id: Uuid,
        telephone: &str,
    ) -> Result<(), EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let self_edit = actor_user_id == user_id;
        if !parsed.can_manage_eleves() && !self_edit {
            return Err(EntDbError::Forbidden);
        }
        if !self.auth.is_member(etablissement_id, user_id).await? {
            return Err(DomainError::NotFound.into());
        }
        self.set_user_telephone(user_id, telephone).await
    }

    /// Crée (ou retrouve) un compte parent et le rattache à un élève sur ce foyer.
    pub async fn rattacher_responsable_foyer_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        foyer_id: Uuid,
        eleve_id: Uuid,
        email: &str,
        display_name: Option<&str>,
        telephone: Option<&str>,
        password: Option<&str>,
        lien: &str,
        peut_voir_notes: bool,
        peut_voir_vs: bool,
        peut_justifier_absence: bool,
        peut_autoriser_sorties: bool,
        est_payeur: bool,
        part_paiement: Option<i16>,
    ) -> Result<EleveLien, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        if self
            .foyers
            .get_foyer(etablissement_id, foyer_id)
            .await?
            .is_none()
        {
            return Err(DomainError::NotFound.into());
        }
        let Some(eleve) = self.eleves.get(etablissement_id, eleve_id).await? else {
            return Err(DomainError::NotFound.into());
        };
        let _ = eleve;

        let user_id = if let Some(uid) = self
            .auth
            .find_member_id_by_email(etablissement_id, email)
            .await?
        {
            uid
        } else {
            let pwd = password.unwrap_or(LABO_PASSWORD);
            let created = self
                .register_user(
                    email,
                    pwd,
                    display_name,
                    etablissement_id,
                    Role::Parent.as_str(),
                )
                .await?;
            created.id
        };

        if let Some(tel) = telephone {
            let t = tel.trim();
            if !t.is_empty() {
                self.set_user_telephone(user_id, t).await?;
            }
        }

        let saved = self
            .foyers
            .link_parent(CreateLien {
                etablissement_id,
                eleve_id,
                user_id,
                foyer_id: Some(foyer_id),
                lien: lien.to_string(),
                peut_voir_notes,
                peut_voir_vs,
                peut_justifier_absence,
                peut_autoriser_sorties,
                est_payeur,
                part_paiement,
            })
            .await?;

        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ELEVE_LIEN_UPDATED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve_id,
                eleve_id: Some(eleve_id),
                payload: serde_json::json!({
                    "created": true,
                    "lien_id": saved.id,
                    "user_id": saved.user_id,
                    "foyer_id": foyer_id,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn get_foyer_detail_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        foyer_id: Uuid,
    ) -> Result<Option<(Uuid, Option<String>, Vec<LienDossierFoyer>)>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(parsed.can_manage_eleves() || parsed.can_read_all_eleves() || parsed == Role::Compta)
        {
            return Err(EntDbError::Forbidden);
        }
        let Some((id, libelle)) = self.foyers.get_foyer(etablissement_id, foyer_id).await? else {
            return Ok(None);
        };
        let liens = self.foyers.list_liens_foyer(etablissement_id, foyer_id).await?;
        Ok(Some((id, libelle, liens)))
    }

    pub async fn list_foyers_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        q: Option<&str>,
        limit: i64,
    ) -> Result<Vec<(Uuid, Option<String>, i64)>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(parsed.can_manage_eleves() || parsed.can_read_all_eleves() || parsed == Role::Compta)
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .foyers
            .list_foyers(etablissement_id, q, limit.clamp(1, 200))
            .await?)
    }

    pub async fn update_lien_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        lien_id: Uuid,
        cmd: UpdateLien,
        actor_user_id: Uuid,
    ) -> Result<EleveLien, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        let saved = self
            .foyers
            .update_lien(etablissement_id, lien_id, cmd)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ELEVE_LIEN_UPDATED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: saved.eleve_id,
                eleve_id: Some(saved.eleve_id),
                payload: serde_json::json!({
                    "lien_id": saved.id,
                    "user_id": saved.user_id,
                    "est_payeur": saved.est_payeur,
                    "part_paiement": saved.part_paiement,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn update_liens_eleve_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        cmds: Vec<(Uuid, UpdateLien)>,
        actor_user_id: Uuid,
    ) -> Result<Vec<EleveLien>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        let saved = self
            .foyers
            .update_liens_eleve(etablissement_id, eleve_id, cmds)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ELEVE_LIEN_UPDATED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve_id,
                eleve_id: Some(eleve_id),
                payload: serde_json::json!({
                    "batch": true,
                    "liens": saved.iter().map(|l| serde_json::json!({
                        "lien_id": l.id,
                        "user_id": l.user_id,
                        "est_payeur": l.est_payeur,
                        "part_paiement": l.part_paiement,
                    })).collect::<Vec<_>>(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn delete_lien_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        lien_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.delete_lien_eleve_for_actor(role, etablissement_id, None, lien_id, actor_user_id)
            .await
    }

    pub async fn delete_lien_eleve_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        eleve_id: Option<Uuid>,
        lien_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<(), EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        let before = self
            .foyers
            .get_lien(etablissement_id, lien_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if let Some(eid) = eleve_id {
            if before.eleve_id != eid {
                return Err(DomainError::NotFound.into());
            }
        }
        self.foyers
            .delete_lien(etablissement_id, lien_id)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ELEVE_LIEN_UPDATED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: before.eleve_id,
                eleve_id: Some(before.eleve_id),
                payload: serde_json::json!({
                    "deleted": true,
                    "lien_id": lien_id,
                    "user_id": before.user_id,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(())
    }

    pub async fn link_parent(&self, cmd: CreateLien) -> Result<EleveLien, EntDbError> {
        Ok(self.foyers.link_parent(cmd).await?)
    }

    pub async fn lien_parent_eleve(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<EleveLien>, EntDbError> {
        Ok(self
            .foyers
            .lien_parent_eleve(etablissement_id, user_id, eleve_id)
            .await?)
    }

    pub async fn eleves_lies(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        Ok(self.foyers.eleves_lies(etablissement_id, user_id).await?)
    }

    /// Portail famille : enfants liés au responsable + flags du lien.
    pub async fn mes_enfants_for_parent(
        &self,
        etablissement_id: Uuid,
        parent_user_id: Uuid,
        role: &str,
    ) -> Result<Vec<EnfantFamille>, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Err(EntDbError::Forbidden);
        };
        if parsed != Role::Parent {
            return Err(EntDbError::Forbidden);
        }
        let liens = self
            .foyers
            .liens_for_parent(etablissement_id, parent_user_id)
            .await?;
        let mut out = Vec::with_capacity(liens.len());
        for lien in liens {
            let Some(eleve) = self.eleves.get(etablissement_id, lien.eleve_id).await? else {
                continue;
            };
            out.push(EnfantFamille {
                eleve,
                lien: lien.lien,
                peut_voir_notes: lien.peut_voir_notes,
                peut_voir_vs: lien.peut_voir_vs,
                peut_justifier_absence: lien.peut_justifier_absence,
                peut_autoriser_sorties: lien.peut_autoriser_sorties,
            });
        }
        Ok(out)
    }

    /// Absences d'un enfant encore sans motif parent (statut à traiter).
    pub async fn absences_a_justifier_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Absence>, EntDbError> {
        let all = self
            .absences
            .list_for_eleve(etablissement_id, eleve_id)
            .await?;
        Ok(all
            .into_iter()
            .filter(|a| a.statut == AbsenceStatut::ATraiter && a.motif_justif.is_none())
            .collect())
    }

    /// Entrées carnet visibles famille non encore signées.
    pub async fn carnet_a_signer_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<CarnetEntree>, EntDbError> {
        let list = self
            .carnet
            .list_for_eleve(etablissement_id, eleve_id, true)
            .await?;
        Ok(list.into_iter().filter(|e| e.signe_at.is_none()).collect())
    }

    /// Lecture élève selon le rôle (staff large vs parent lié uniquement).
    pub async fn get_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<Eleve>, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Err(EntDbError::Forbidden);
        };
        if parsed.can_read_all_eleves() {
            return self.get_eleve(etablissement_id, eleve_id).await;
        }
        if parsed == Role::Parent {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            if lien.is_none() {
                return Ok(None);
            }
            return self.get_eleve(etablissement_id, eleve_id).await;
        }
        if parsed == Role::Eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Ok(None);
            }
            return self.get_eleve(etablissement_id, eleve_id).await;
        }
        Err(EntDbError::Forbidden)
    }

    pub fn role_can_manage_eleves(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_manage_eleves())
            .unwrap_or(false)
    }

    pub fn role_can_declare_absence_staff(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_declare_absence_staff())
            .unwrap_or(false)
    }

    pub fn role_can_read_effectif(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_read_effectif())
            .unwrap_or(false)
    }

    pub fn role_can_manage_matieres(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_manage_matieres())
            .unwrap_or(false)
    }

    pub fn role_can_saisir_notes(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_saisir_notes())
            .unwrap_or(false)
    }

    /// Parent lié avec flag justif, ou staff absences.
    pub async fn actor_can_declare_absence(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(false);
        };
        if parsed.can_declare_absence_staff() {
            return Ok(true);
        }
        if parsed == Role::Parent {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            return Ok(lien.map(|l| l.peut_justifier_absence).unwrap_or(false));
        }
        if parsed == Role::Eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            return Ok(linked == Some(eleve_id));
        }
        Ok(false)
    }

    pub async fn actor_can_read_notes(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(false);
        };
        if parsed.can_read_notes_staff() {
            return Ok(true);
        }
        if parsed == Role::Parent {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            return Ok(lien.map(|l| l.peut_voir_notes).unwrap_or(false));
        }
        if parsed == Role::Eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            return Ok(linked == Some(eleve_id));
        }
        Ok(false)
    }

    pub async fn create_matiere(
        &self,
        cmd: CreateMatiere,
        actor_user_id: Option<String>,
    ) -> Result<Matiere, EntDbError> {
        let code = cmd.code.trim().to_string();
        let libelle = cmd.libelle.trim().to_string();
        if code.is_empty() || libelle.is_empty() {
            return Err(EntDbError::MatiereInvalide);
        }
        let matiere = Matiere {
            id: Uuid::new_v4(),
            etablissement_id: cmd.etablissement_id,
            code,
            libelle,
            source_nomenclature: cmd.source_nomenclature,
        };
        self.matieres.insert(&matiere).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: matiere.etablissement_id,
                type_name: types::MATIERE_CREATED.to_string(),
                aggregate: AggregateKind::Matiere,
                aggregate_id: matiere.id,
                eleve_id: None,
                payload: json!({
                    "code": matiere.code,
                    "libelle": matiere.libelle,
                    "source_nomenclature": matiere.source_nomenclature,
                }),
                actor_user_id,
            })
            .await?;
        Ok(matiere)
    }

    pub async fn list_matieres(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<Matiere>, EntDbError> {
        Ok(self.matieres.list(etablissement_id).await?)
    }

    pub async fn creer_note_periode(
        &self,
        cmd: CreerNotePeriode,
    ) -> Result<NotePeriode, EntDbError> {
        let periode = plan_creer_note_periode(cmd)?;
        self.notes.insert_periode(&periode).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: periode.etablissement_id,
                type_name: types::NOTE_PERIODE_CREATED.to_string(),
                aggregate: AggregateKind::NotePeriode,
                aggregate_id: periode.id,
                eleve_id: None,
                payload: json!({
                    "code": periode.code,
                    "libelle": periode.libelle,
                    "date_debut": periode.date_debut.to_string(),
                    "date_fin": periode.date_fin.to_string(),
                    "statut": periode.statut.as_str(),
                }),
                actor_user_id: None,
            })
            .await?;
        Ok(periode)
    }

    pub async fn set_note_periode_statut(
        &self,
        etablissement_id: Uuid,
        periode_id: Uuid,
        statut: NotePeriodeStatut,
        actor_user_id: Option<String>,
    ) -> Result<NotePeriode, EntDbError> {
        self.notes
            .set_periode_statut(etablissement_id, periode_id, statut)
            .await?;
        let periode = self
            .notes
            .get_periode(etablissement_id, periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::NOTE_PERIODE_STATUT.to_string(),
                aggregate: AggregateKind::NotePeriode,
                aggregate_id: periode_id,
                eleve_id: None,
                payload: json!({ "statut": statut.as_str() }),
                actor_user_id,
            })
            .await?;
        Ok(periode)
    }

    pub async fn list_note_periodes(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<NotePeriode>, EntDbError> {
        Ok(self.notes.list_periodes(etablissement_id).await?)
    }

    pub async fn saisir_appreciation_for_actor(
        &self,
        role: &str,
        cmd: SaisirAppreciation,
    ) -> Result<Appreciation, EntDbError> {
        if !Self::role_can_saisir_notes(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if let Some(mid) = cmd.matiere_id {
            self.matieres
                .get(cmd.etablissement_id, mid)
                .await?
                .ok_or(DomainError::NotFound)?;
        }
        let periode = self
            .notes
            .get_periode(cmd.etablissement_id, cmd.periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_saisir_appreciation(&periode, cmd)?;
        let saved = self.notes.upsert_appreciation(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::NOTE_APPRECIATION.to_string(),
                aggregate: AggregateKind::Note,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({
                    "periode_id": saved.periode_id,
                    "matiere_id": saved.matiere_id,
                    "generale": saved.matiere_id.is_none(),
                }),
                actor_user_id: Some(saved.auteur_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn list_appreciations_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        periode_id: Option<Uuid>,
    ) -> Result<Vec<Appreciation>, EntDbError> {
        if !self
            .actor_can_read_notes(etablissement_id, actor_user_id, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .notes
            .list_appreciations_eleve(etablissement_id, eleve_id, periode_id)
            .await?)
    }

    /// Période fermée → PDF bulletin → dépôt document famille (un cerveau).
    pub async fn publier_bulletin_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        periode_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<EleveDocument, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(parsed.can_manage_eleves() || parsed.can_saisir_notes() || parsed == Role::Cpe) {
            return Err(EntDbError::Forbidden);
        }
        let periode = self
            .notes
            .get_periode(etablissement_id, periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let moyennes = self
            .moyennes_eleve_periode(etablissement_id, eleve_id, periode_id)
            .await?;
        let matieres = self.list_matieres(etablissement_id).await?;
        let labels: std::collections::HashMap<Uuid, String> = matieres
            .into_iter()
            .map(|m| (m.id, m.libelle))
            .collect();
        let moyennes_l: Vec<(MoyenneMatiere, String)> = moyennes
            .into_iter()
            .map(|m| {
                let lib = labels
                    .get(&m.matiere_id)
                    .cloned()
                    .unwrap_or_else(|| m.matiere_id.to_string());
                (m, lib)
            })
            .collect();
        let apps = self
            .notes
            .list_appreciations_eleve(etablissement_id, eleve_id, Some(periode_id))
            .await?;
        let bulletin = plan_bulletin_eleve(
            &periode,
            &eleve.prenom,
            &eleve.nom,
            eleve.classe.as_deref(),
            &moyennes_l,
            &apps,
            today_utc(),
        )?;
        let bytes = render_bulletin_pdf(&bulletin);
        let key = ObjectStore::key_for(etablissement_id, "bulletin", periode_id, "pdf");
        // Clé stable période+élève : namespace inclut eleve_id via suffixe.
        let key = format!("{key}-{eleve_id}");
        self.objects.put(&key, &bytes)?;
        let doc = self
            .deposer_document(DeposerDocument {
                etablissement_id,
                eleve_id,
                // Publication famille / Lite — même règle docs standard tiroir famille.
                tiroir: DocTiroir::Famille,
                titre: format!(
                    "Bulletin {} — {} {}",
                    periode.libelle, eleve.prenom, eleve.nom
                ),
                code: Some(format!("BULLETIN_{}", periode.code)),
                mime_type: Some("application/pdf".into()),
                storage_key: Some(key.clone()),
                confidentialite: DocConfidentialite::Standard,
                actor_user_id,
            })
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::NOTE_BULLETIN_PUBLIE.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve_id,
                eleve_id: Some(eleve_id),
                payload: json!({
                    "periode_id": periode_id,
                    "document_id": doc.id,
                    "storage_key": key,
                    "octets": bytes.len(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(doc)
    }

    pub async fn creer_devoir(&self, cmd: CreerDevoir) -> Result<Devoir, EntDbError> {
        let periode = self
            .notes
            .get_periode(cmd.etablissement_id, cmd.periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !periode.statut.autorise_saisie_notes() {
            return Err(DomainError::NotePeriodeFermee.into());
        }
        self.matieres
            .get(cmd.etablissement_id, cmd.matiere_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let devoir = plan_creer_devoir(cmd)?;
        self.notes.insert_devoir(&devoir).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: devoir.etablissement_id,
                type_name: types::DEVOIR_CREATED.to_string(),
                aggregate: AggregateKind::Devoir,
                aggregate_id: devoir.id,
                eleve_id: None,
                payload: json!({
                    "periode_id": devoir.periode_id,
                    "matiere_id": devoir.matiere_id,
                    "classe": devoir.classe,
                    "titre": devoir.titre,
                    "coef": devoir.coef,
                    "sur": devoir.sur,
                }),
                actor_user_id: devoir.created_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        Ok(devoir)
    }

    pub async fn get_devoir(
        &self,
        etablissement_id: Uuid,
        devoir_id: Uuid,
    ) -> Result<Option<Devoir>, EntDbError> {
        Ok(self.notes.get_devoir(etablissement_id, devoir_id).await?)
    }

    pub async fn list_devoirs_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        periode_id: Uuid,
        classe: Option<&str>,
    ) -> Result<Vec<Devoir>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(parsed.can_saisir_notes()
            || parsed.can_manage_eleves()
            || parsed.can_read_all_eleves())
        {
            return Err(EntDbError::Forbidden);
        }
        self.notes
            .get_periode(etablissement_id, periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        Ok(self
            .notes
            .list_devoirs_periode(etablissement_id, periode_id, classe)
            .await?)
    }

    /// Devoirs de la classe de l'élève (lecture famille / face élève).
    pub async fn list_devoirs_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        periode_id: Uuid,
    ) -> Result<Vec<Devoir>, EntDbError> {
        if !self
            .actor_can_read_notes(etablissement_id, actor_user_id, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.notes
            .get_periode(etablissement_id, periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let Some(classe) = eleve
            .classe
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Ok(Vec::new());
        };
        Ok(self
            .notes
            .list_devoirs_periode(etablissement_id, periode_id, Some(classe))
            .await?)
    }

    pub async fn saisir_note(&self, cmd: SaisirNote) -> Result<Note, EntDbError> {
        if cmd.sur <= 0.0 || cmd.valeur < 0.0 || cmd.valeur > cmd.sur {
            return Err(EntDbError::NoteInvalide);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.matieres
            .get(cmd.etablissement_id, cmd.matiere_id)
            .await?
            .ok_or(DomainError::NotFound)?;

        let mut periode_id = cmd.periode_id;
        let mut matiere_id = cmd.matiere_id;
        let mut sur = cmd.sur;
        let mut libelle = cmd.libelle.clone();

        if let Some(devoir_id) = cmd.devoir_id {
            let devoir = self
                .notes
                .get_devoir(cmd.etablissement_id, devoir_id)
                .await?
                .ok_or(DomainError::DevoirIntrouvable)?;
            periode_id = Some(devoir.periode_id);
            matiere_id = devoir.matiere_id;
            sur = devoir.sur;
            if libelle.is_none() {
                libelle = Some(devoir.titre.clone());
            }
        }

        if let Some(pid) = periode_id {
            let periode = self
                .notes
                .get_periode(cmd.etablissement_id, pid)
                .await?
                .ok_or(DomainError::NotFound)?;
            if !periode.statut.autorise_saisie_notes() {
                return Err(DomainError::NotePeriodeFermee.into());
            }
        }

        let note = Note {
            id: Uuid::new_v4(),
            etablissement_id: cmd.etablissement_id,
            eleve_id: cmd.eleve_id,
            matiere_id,
            valeur: cmd.valeur,
            sur,
            date_note: cmd.date_note,
            auteur_user_id: cmd.auteur_user_id,
            libelle,
            devoir_id: cmd.devoir_id,
            periode_id,
        };
        self.notes.insert(&note).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: note.etablissement_id,
                type_name: types::NOTE_SAISIE.to_string(),
                aggregate: AggregateKind::Note,
                aggregate_id: note.id,
                eleve_id: Some(note.eleve_id),
                payload: json!({
                    "matiere_id": note.matiere_id,
                    "valeur": note.valeur,
                    "sur": note.sur,
                    "date_note": note.date_note.to_string(),
                    "devoir_id": note.devoir_id,
                    "periode_id": note.periode_id,
                }),
                actor_user_id: Some(note.auteur_user_id.to_string()),
            })
            .await?;
        Ok(note)
    }

    pub async fn notes_for_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Note>, EntDbError> {
        Ok(self.notes.list_for_eleve(etablissement_id, eleve_id).await?)
    }

    pub async fn moyenne_matiere(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        periode_id: Uuid,
        matiere_id: Uuid,
    ) -> Result<Option<MoyenneMatiere>, EntDbError> {
        let rows = self
            .notes
            .list_for_moyenne(etablissement_id, eleve_id, periode_id, matiere_id)
            .await?;
        Ok(moyenne_sur_20(&rows).map(|m| MoyenneMatiere {
            matiere_id,
            periode_id,
            moyenne_sur_20: (m * 100.0).round() / 100.0,
        }))
    }

    pub async fn moyennes_eleve_periode(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        periode_id: Uuid,
    ) -> Result<Vec<MoyenneMatiere>, EntDbError> {
        let notes = self.notes.list_for_eleve(etablissement_id, eleve_id).await?;
        let mut matiere_ids: Vec<Uuid> = notes
            .iter()
            .filter(|n| n.periode_id == Some(periode_id))
            .map(|n| n.matiere_id)
            .collect();
        matiere_ids.sort();
        matiere_ids.dedup();
        let mut out = Vec::new();
        for mid in matiere_ids {
            if let Some(m) = self
                .moyenne_matiere(etablissement_id, eleve_id, periode_id, mid)
                .await?
            {
                out.push(m);
            }
        }
        Ok(out)
    }

    pub fn role_can_faire_appel(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_faire_appel())
            .unwrap_or(false)
    }

    /// Staff hors titulaire qui peut ouvrir un appel (remplacement / CPE / direction).
    fn role_peut_ouvrir_appel_remplacant(role: &str) -> bool {
        Role::parse(role)
            .map(|r| {
                matches!(
                    r,
                    Role::Admin | Role::Direction | Role::Administratif | Role::Cpe
                )
            })
            .unwrap_or(false)
    }

    pub async fn creer_edt_creneau(&self, c: EdtCreneau) -> Result<EdtCreneau, EntDbError> {
        if c.classe.trim().is_empty() || c.matiere_libelle.trim().is_empty() {
            return Err(EntDbError::AppelInvalide);
        }
        if !(1..=7).contains(&c.jour_semaine) || c.heure_fin <= c.heure_debut {
            return Err(EntDbError::AppelInvalide);
        }
        self.edt.insert(&c).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: c.etablissement_id,
                type_name: types::EDT_CRENEAU_CREATED.to_string(),
                aggregate: AggregateKind::Edt,
                aggregate_id: c.id,
                eleve_id: None,
                payload: json!({
                    "classe": c.classe,
                    "matiere": c.matiere_libelle,
                    "prof_user_id": c.prof_user_id.to_string(),
                    "jour_semaine": c.jour_semaine,
                    "semaine_ab": c.semaine_ab.as_str(),
                }),
                actor_user_id: None,
            })
            .await?;
        Ok(c)
    }

    /// Filtre A/B : sans calendrier, seuls les créneaux `toutes` restent visibles.
    async fn filtrer_creneaux_semaine_ab(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
        list: Vec<EdtCreneau>,
    ) -> Result<Vec<EdtCreneau>, EntDbError> {
        let cal = self.edt.get_calendrier_ab(etablissement_id).await?;
        let Some(cal) = cal else {
            return Ok(list
                .into_iter()
                .filter(|c| c.semaine_ab == EdtSemaineAb::Toutes)
                .collect());
        };
        let ab = semaine_ab_au_jour(cal.reference_lundi, jour);
        Ok(list
            .into_iter()
            .filter(|c| creneau_actif_semaine(c.semaine_ab, ab))
            .collect())
    }

    pub async fn list_edt_creneaux_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<EdtCreneau>, EntDbError> {
        let list = self.edt.list_pour_jour(etablissement_id, jour).await?;
        self.filtrer_creneaux_semaine_ab(etablissement_id, jour, list)
            .await
    }

    /// EDT classe de l'élève au jour J — staff ou parent lié.
    pub async fn list_edt_eleve_jour_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<EdtCreneau>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.is_staff();
        if !staff && !matches!(parsed, Role::Parent | Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        if matches!(parsed, Role::Parent)
            && self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if matches!(parsed, Role::Eleve) {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let classe = eleve.classe.ok_or(DomainError::NotFound)?;
        let list = self
            .list_edt_creneaux_jour(etablissement_id, jour)
            .await?
            .into_iter()
            .filter(|c| c.classe.eq_ignore_ascii_case(&classe))
            .collect();
        Ok(list)
    }

    pub async fn configurer_edt_calendrier_ab_for_actor(
        &self,
        role: &str,
        cmd: ConfigurerEdtCalendrierAb,
    ) -> Result<EdtCalendrierAb, EntDbError> {
        if !Self::role_can_manage_edt(role) {
            return Err(EntDbError::Forbidden);
        }
        let cal = plan_configurer_edt_calendrier_ab(cmd)?;
        self.edt.upsert_calendrier_ab(&cal).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: cal.etablissement_id,
                type_name: types::EDT_CALENDRIER_AB.to_string(),
                aggregate: AggregateKind::Edt,
                aggregate_id: cal.etablissement_id,
                eleve_id: None,
                payload: json!({
                    "reference_lundi": cal.reference_lundi.to_string(),
                }),
                actor_user_id: Some(cal.created_by_user_id.to_string()),
            })
            .await?;
        Ok(cal)
    }

    pub async fn get_edt_calendrier_ab(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Option<EdtCalendrierAb>, EntDbError> {
        if !Self::role_can_manage_edt(role) && !Self::role_can_faire_appel(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.edt.get_calendrier_ab(etablissement_id).await?)
    }

    pub fn role_can_manage_edt(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_manage_edt())
            .unwrap_or(false)
    }

    pub async fn poser_edt_exception_for_actor(
        &self,
        role: &str,
        cmd: PoserEdtException,
    ) -> Result<EdtException, EntDbError> {
        if !Self::role_can_manage_edt(role) {
            return Err(EntDbError::Forbidden);
        }
        let creneau = self
            .edt
            .get(cmd.etablissement_id, cmd.creneau_id)
            .await?
            .ok_or(EntDbError::AppelInvalide)?;
        if creneau.jour_semaine != jour_semaine_iso(cmd.jour) {
            return Err(DomainError::EdtExceptionInvalide.into());
        }
        if cmd.kind == EdtExceptionKind::Remplacement {
            if let Some(rid) = cmd.remplacant_user_id {
                if rid == creneau.prof_user_id {
                    return Err(DomainError::EdtExceptionInvalide.into());
                }
            }
        }
        let actor = cmd.actor_user_id;
        let ex = plan_poser_edt_exception(cmd)?;
        self.edt.upsert_exception(&ex).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: ex.etablissement_id,
                type_name: types::EDT_EXCEPTION.to_string(),
                aggregate: AggregateKind::Edt,
                aggregate_id: ex.creneau_id,
                eleve_id: None,
                payload: json!({
                    "exception_id": ex.id,
                    "kind": ex.kind.as_str(),
                    "jour": ex.jour.to_string(),
                    "remplacant_user_id": ex.remplacant_user_id,
                    "motif": ex.motif,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(ex)
    }

    /// Import CSV EDT — `format` = `native` (UUID prof) ou `pronote` (e-mail enseignant).
    /// dry_run = détection conflits sans écriture ; apply = crée les créneaux.
    pub async fn importer_edt_csv_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        csv: &str,
        dry_run: bool,
    ) -> Result<EdtImportReport, EntDbError> {
        self.importer_edt_csv_format_for_actor(
            etablissement_id,
            role,
            actor_user_id,
            csv,
            dry_run,
            "native",
        )
        .await
    }

    pub async fn importer_edt_csv_format_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        csv: &str,
        dry_run: bool,
        format: &str,
    ) -> Result<EdtImportReport, EntDbError> {
        if !Self::role_can_manage_edt(role) {
            return Err(EntDbError::Forbidden);
        }
        let format = format.trim().to_ascii_lowercase();
        let lines = if matches!(format.as_str(), "pronote" | "pronote_csv" | "export_pronote") {
            let raw = parse_edt_pronote_csv(csv)?;
            let mut cache: std::collections::HashMap<String, Option<Uuid>> =
                std::collections::HashMap::new();
            let mut resolved = Vec::with_capacity(raw.len());
            for line in &raw {
                let uid = if let Some(cached) = cache.get(&line.prof_email) {
                    *cached
                } else {
                    let found = self
                        .auth
                        .find_member_id_by_email(etablissement_id, &line.prof_email)
                        .await
                        .map_err(EntDbError::Auth)?;
                    cache.insert(line.prof_email.clone(), found);
                    found
                };
                let Some(prof_user_id) = uid else {
                    return Err(EdtImportParseError::ProfEmail.into());
                };
                resolved.push(EdtImportLigne {
                    index: line.index,
                    classe: line.classe.clone(),
                    matiere_libelle: line.matiere_libelle.clone(),
                    prof_user_id,
                    jour_semaine: line.jour_semaine,
                    heure_debut: line.heure_debut,
                    heure_fin: line.heure_fin,
                    salle: line.salle.clone(),
                    semaine_ab: line.semaine_ab,
                });
            }
            resolved
        } else {
            parse_edt_import_csv(csv)?
        };
        let existing_creneaux = self.edt.list_all(etablissement_id).await?;
        let existing: Vec<EdtImportLigne> = existing_creneaux
            .iter()
            .map(|c| EdtImportLigne {
                index: usize::MAX,
                classe: c.classe.clone(),
                matiere_libelle: c.matiere_libelle.clone(),
                prof_user_id: c.prof_user_id,
                jour_semaine: c.jour_semaine,
                heure_debut: c.heure_debut,
                heure_fin: c.heure_fin,
                salle: c.salle.clone(),
                semaine_ab: c.semaine_ab,
            })
            .collect();
        let report = detect_conflits_edt(&lines, &existing);
        let mut applied = 0usize;
        if !dry_run {
            if !report.conflicts.is_empty() {
                return Err(EntDbError::EdtImportConflicts);
            }
            for line in &lines {
                let c = EdtCreneau {
                    id: Uuid::new_v4(),
                    etablissement_id,
                    classe: line.classe.clone(),
                    matiere_id: None,
                    matiere_libelle: line.matiere_libelle.clone(),
                    prof_user_id: line.prof_user_id,
                    jour_semaine: line.jour_semaine,
                    heure_debut: line.heure_debut,
                    heure_fin: line.heure_fin,
                    salle: line.salle.clone(),
                    site: String::new(),
                    semaine_ab: line.semaine_ab,
                };
                self.edt.insert(&c).await?;
                applied += 1;
            }
        }
        let import_id = Uuid::new_v4();
        let report_json = json!({
            "lines": report.lines,
            "ok_count": if dry_run { report.ok_count } else { applied },
            "conflicts": report.conflicts.iter().map(|c| json!({
                "kind": match c.kind {
                    domain_eleve::EdtConflictKind::ProfDouble => "prof_double",
                    domain_eleve::EdtConflictKind::SalleDouble => "salle_double",
                    domain_eleve::EdtConflictKind::ClasseDouble => "classe_double",
                },
                "ligne_a": c.ligne_a,
                "ligne_b": c.ligne_b,
                "detail": c.detail,
            })).collect::<Vec<_>>(),
            "dry_run": dry_run,
            "applied": applied,
            "format": format,
        });
        self.edt
            .insert_import_journal(
                import_id,
                etablissement_id,
                dry_run,
                report.lines as i32,
                applied as i32,
                report.conflicts.len() as i32,
                report_json,
                actor_user_id,
            )
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::EDT_IMPORT.to_string(),
                aggregate: AggregateKind::Edt,
                aggregate_id: import_id,
                eleve_id: None,
                payload: json!({
                    "dry_run": dry_run,
                    "lines": report.lines,
                    "conflicts": report.conflicts.len(),
                    "applied": applied,
                    "format": format,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        let mut out = report;
        if !dry_run {
            out.ok_count = applied;
        }
        Ok(out)
    }

    pub async fn count_edt_imports(
        &self,
        etablissement_id: Uuid,
    ) -> Result<i64, EntDbError> {
        Ok(self.edt.count_imports(etablissement_id).await?)
    }

    /// Mini-solveur greedy : place des demandes sur la grille défaut (ou slots fournis).
    /// `apply=true` crée les créneaux placés (refuse s'il reste des unplaced).
    pub async fn resoudre_edt_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        demandes: Vec<EdtDemande>,
        dry_run: bool,
    ) -> Result<EdtSolveReport, EntDbError> {
        if !Self::role_can_manage_edt(role) {
            return Err(EntDbError::Forbidden);
        }
        if demandes.is_empty() {
            return Err(EdtImportParseError::Empty.into());
        }
        let existing_creneaux = self.edt.list_all(etablissement_id).await?;
        let existing: Vec<EdtImportLigne> = existing_creneaux
            .iter()
            .map(|c| EdtImportLigne {
                index: usize::MAX,
                classe: c.classe.clone(),
                matiere_libelle: c.matiere_libelle.clone(),
                prof_user_id: c.prof_user_id,
                jour_semaine: c.jour_semaine,
                heure_debut: c.heure_debut,
                heure_fin: c.heure_fin,
                salle: c.salle.clone(),
                semaine_ab: c.semaine_ab,
            })
            .collect();
        let slots = grille_edt_defaut();
        let soft: Vec<EdtContrainteSoft> = self
            .edt_contraintes
            .list(etablissement_id)
            .await?
            .into_iter()
            .map(|c| EdtContrainteSoft {
                kind: c.kind.as_str(),
                prof_user_id: c.prof_user_id,
                salle_code: c.salle_code,
                jour_semaine: c.jour_semaine,
                heure_debut: c.heure_debut,
                heure_fin: c.heure_fin,
            })
            .collect();
        let report = solve_edt_greedy_soft(&demandes, &slots, &existing, &soft);
        if !dry_run {
            if !report.unplaced.is_empty() {
                return Err(EntDbError::EdtImportConflicts);
            }
            for line in &report.placed {
                let c = EdtCreneau {
                    id: Uuid::new_v4(),
                    etablissement_id,
                    classe: line.classe.clone(),
                    matiere_id: None,
                    matiere_libelle: line.matiere_libelle.clone(),
                    prof_user_id: line.prof_user_id,
                    jour_semaine: line.jour_semaine,
                    heure_debut: line.heure_debut,
                    heure_fin: line.heure_fin,
                    salle: line.salle.clone(),
                    site: String::new(),
                    semaine_ab: line.semaine_ab,
                };
                self.edt.insert(&c).await?;
            }
        }
        let import_id = Uuid::new_v4();
        let report_json = json!({
            "placed": report.placed.len(),
            "unplaced": report.unplaced.len(),
            "dry_run": dry_run,
            "format": "solver_greedy",
        });
        self.edt
            .insert_import_journal(
                import_id,
                etablissement_id,
                dry_run,
                demandes.len() as i32,
                if dry_run { 0 } else { report.placed.len() as i32 },
                report.unplaced.len() as i32,
                report_json,
                actor_user_id,
            )
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::EDT_IMPORT.to_string(),
                aggregate: AggregateKind::Edt,
                aggregate_id: import_id,
                eleve_id: None,
                payload: json!({
                    "format": "solver_greedy",
                    "dry_run": dry_run,
                    "placed": report.placed.len(),
                    "unplaced": report.unplaced.len(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(report)
    }

    pub async fn list_edt_exceptions_jour(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<EdtException>, EntDbError> {
        if !Self::role_can_manage_edt(role) && !Self::role_can_faire_appel(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.edt.list_exceptions_jour(etablissement_id, jour).await?)
    }

    /// Créneaux du jour en tenant compte des exceptions (annulation filtrée).
    pub async fn list_mes_creneaux(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<EdtCreneau>, EntDbError> {
        if !Self::role_can_faire_appel(role) {
            return Err(EntDbError::Forbidden);
        }
        let exceptions = self.edt.list_exceptions_jour(etablissement_id, jour).await?;
        let annules: std::collections::HashSet<Uuid> = exceptions
            .iter()
            .filter(|e| e.kind == EdtExceptionKind::Annulation)
            .map(|e| e.creneau_id)
            .collect();
        let remplacements: std::collections::HashMap<Uuid, Uuid> = exceptions
            .iter()
            .filter(|e| e.kind == EdtExceptionKind::Remplacement)
            .filter_map(|e| e.remplacant_user_id.map(|r| (e.creneau_id, r)))
            .collect();

        let parsed = Role::parse(role);
        let mut list = if parsed == Some(Role::Prof) {
            let mut own = self
                .edt
                .list_pour_prof_jour(etablissement_id, user_id, jour)
                .await?;
            // Titulaire remplacé ce jour → créneau retiré de sa liste.
            own.retain(|c| match remplacements.get(&c.id) {
                Some(r) if *r != user_id => false,
                _ => true,
            });
            // Remplaçant nominatif → créneau ajouté.
            let tous = self.edt.list_pour_jour(etablissement_id, jour).await?;
            for c in tous {
                if remplacements.get(&c.id) == Some(&user_id) && !own.iter().any(|x| x.id == c.id)
                {
                    own.push(c);
                }
            }
            own
        } else {
            self.edt.list_pour_jour(etablissement_id, jour).await?
        };
        list.retain(|c| !annules.contains(&c.id));
        // Appliquer salle effective
        for c in &mut list {
            if let Some(ex) = exceptions.iter().find(|e| {
                e.creneau_id == c.id && e.kind == EdtExceptionKind::ChangementSalle
            }) {
                if let Some(ref s) = ex.nouvelle_salle {
                    c.salle = Some(s.clone());
                }
            }
        }
        self.filtrer_creneaux_semaine_ab(etablissement_id, jour, list)
            .await
    }

    /// Ouvre l'appel d'un **créneau** au jour J (effectif = dates d'effet).
    /// Titulaire, remplaçant nominatif, ou staff VS.
    pub async fn ouvrir_appel(
        &self,
        etablissement_id: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
        auteur_user_id: Uuid,
        role: &str,
    ) -> Result<(Appel, Vec<AppelLigne>), EntDbError> {
        if !Self::role_can_faire_appel(role) {
            return Err(EntDbError::Forbidden);
        }
        let creneau = self
            .edt
            .get(etablissement_id, creneau_id)
            .await?
            .ok_or(EntDbError::AppelInvalide)?;
        if creneau.jour_semaine != jour_semaine_iso(jour) {
            return Err(EntDbError::AppelInvalide);
        }
        // Créneau hors semaine A/B → pas d'appel.
        let filtered = self
            .filtrer_creneaux_semaine_ab(etablissement_id, jour, vec![creneau.clone()])
            .await?;
        if filtered.is_empty() {
            return Err(EntDbError::AppelInvalide);
        }
        let exception = self
            .edt
            .get_exception(etablissement_id, creneau_id, jour)
            .await?;
        if exception
            .as_ref()
            .is_some_and(|e| e.kind == EdtExceptionKind::Annulation)
        {
            return Err(EntDbError::AppelInvalide);
        }
        let est_remplacant = exception
            .as_ref()
            .and_then(|e| e.remplacant_user_id)
            .is_some_and(|r| r == auteur_user_id);
        let est_titulaire = creneau.prof_user_id == auteur_user_id
            && exception
                .as_ref()
                .and_then(|e| e.remplacant_user_id)
                .is_none();
        if !est_titulaire && !est_remplacant && !Self::role_peut_ouvrir_appel_remplacant(role) {
            return Err(EntDbError::Forbidden);
        }

        if let Some(existing) = self
            .appels
            .get_by_creneau_jour(etablissement_id, creneau_id, jour)
            .await?
        {
            let lignes = self
                .appels
                .list_lignes(etablissement_id, existing.id)
                .await?;
            return Ok((existing, lignes));
        }

        let effectif = self
            .eleves
            .list_en_classe_au(etablissement_id, &creneau.classe, jour)
            .await?;

        let appel = Appel {
            id: Uuid::new_v4(),
            etablissement_id,
            classe: creneau.classe.clone(),
            jour,
            auteur_user_id,
            statut: AppelStatut::Ouvert,
            creneau_id: Some(creneau.id),
            matiere_libelle: Some(creneau.matiere_libelle.clone()),
        };
        self.appels.insert_appel(&appel).await?;

        let mut lignes = Vec::new();
        for e in &effectif {
            let ligne = AppelLigne {
                id: Uuid::new_v4(),
                etablissement_id,
                appel_id: appel.id,
                eleve_id: e.id,
                etat: LigneEtat::Present,
            };
            self.appels.insert_ligne(&ligne).await?;
            lignes.push(ligne);
        }

        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::APPEL_OPENED.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel.id,
                eleve_id: None,
                payload: json!({
                    "classe": appel.classe,
                    "jour": jour.to_string(),
                    "creneau_id": creneau.id.to_string(),
                    "matiere": creneau.matiere_libelle,
                    "effectif": lignes.len(),
                }),
                actor_user_id: Some(auteur_user_id.to_string()),
            })
            .await?;

        Ok((appel, lignes))
    }

    /// Helper seed / tests : crée un créneau du jour courant pour un prof.
    pub async fn seed_creneau_du_jour(
        &self,
        etablissement_id: Uuid,
        classe: &str,
        matiere_libelle: &str,
        prof_user_id: Uuid,
        jour: NaiveDate,
        heure_debut: NaiveTime,
        heure_fin: NaiveTime,
        salle: Option<&str>,
    ) -> Result<EdtCreneau, EntDbError> {
        self.creer_edt_creneau(EdtCreneau {
            id: Uuid::new_v4(),
            etablissement_id,
            classe: classe.into(),
            matiere_id: None,
            matiere_libelle: matiere_libelle.into(),
            prof_user_id,
            jour_semaine: jour_semaine_iso(jour),
            heure_debut,
            heure_fin,
            salle: salle.map(|s| s.into()),
            site: String::new(),
            semaine_ab: EdtSemaineAb::Toutes,
        })
        .await
    }

    pub async fn marquer_appel_ligne(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        etat: LigneEtat,
    ) -> Result<(), EntDbError> {
        let appel = self
            .appels
            .get(etablissement_id, appel_id)
            .await?
            .ok_or(EntDbError::AppelInvalide)?;
        if appel.statut != AppelStatut::Ouvert {
            return Err(EntDbError::AppelInvalide);
        }
        self.appels
            .set_etat(etablissement_id, appel_id, eleve_id, &etat)
            .await
            .map_err(|_| EntDbError::AppelInvalide)?;
        Ok(())
    }

    /// Clôture → Brain `EnsureAbsence` pour chaque absent → `ent_absence` (VS / cantine).
    pub async fn clore_appel(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        actor_user_id: Option<String>,
    ) -> Result<Appel, EntDbError> {
        let appel = self
            .appels
            .get(etablissement_id, appel_id)
            .await?
            .ok_or(EntDbError::AppelInvalide)?;
        if appel.statut != AppelStatut::Ouvert {
            return Err(EntDbError::AppelInvalide);
        }
        let lignes = self.appels.list_lignes(etablissement_id, appel_id).await?;
        let absents: Vec<Uuid> = lignes
            .iter()
            .filter(|l| l.etat == LigneEtat::Absent)
            .map(|l| l.eleve_id)
            .collect();

        self.appels
            .clore(etablissement_id, appel_id)
            .await
            .map_err(|_| EntDbError::AppelInvalide)?;

        let event = self
            .events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::APPEL_CLOSED.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel_id,
                eleve_id: None,
                payload: json!({
                    "classe": appel.classe,
                    "jour": appel.jour.to_string(),
                    "absents": absents.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                }),
                actor_user_id,
            })
            .await?;

        for action in react(&event) {
            self.apply_brain_action(action).await?;
        }

        Ok(Appel {
            statut: AppelStatut::Clos,
            ..appel
        })
    }

    pub async fn get_appel(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
    ) -> Result<Option<(Appel, Vec<AppelLigne>)>, EntDbError> {
        let Some(appel) = self.appels.get(etablissement_id, appel_id).await? else {
            return Ok(None);
        };
        let lignes = self.appels.list_lignes(etablissement_id, appel_id).await?;
        Ok(Some((appel, lignes)))
    }

    async fn apply_brain_action(&self, action: BrainAction) -> Result<(), EntDbError> {
        match action {
            BrainAction::EnsureAbsence {
                etablissement_id,
                eleve_id,
                motif,
                debut,
                fin,
            } => {
                let debut = NaiveDate::parse_from_str(&debut, "%Y-%m-%d")
                    .map_err(|_| EntDbError::AppelInvalide)?;
                let fin = NaiveDate::parse_from_str(&fin, "%Y-%m-%d")
                    .map_err(|_| EntDbError::AppelInvalide)?;
                // Idempotent : si déjà absent ce jour (même motif), on réécrit une ligne.
                let absence = Absence {
                    id: Uuid::new_v4(),
                    etablissement_id,
                    eleve_id,
                    debut,
                    fin,
                    motif,
                    statut: AbsenceStatut::ATraiter,
                    motif_justif: None,
                    justifie_par_user_id: None,
                    decision_note: None,
                    decision_par_user_id: None,
                };
                self.absences.upsert(&absence).await?;
                self.apres_absence_ecrite(etablissement_id, eleve_id, debut)
                    .await?;
            }
        }
        Ok(())
    }

    /// Après toute écriture d'absence active : compteur année scolaire → signal VS si seuil.
    async fn apres_absence_ecrite(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<(), EntDbError> {
        let depuis = debut_annee_scolaire(jour);
        let annee = annee_scolaire_de(jour);
        let n = self
            .absences
            .count_actives_depuis(etablissement_id, eleve_id, depuis)
            .await?;
        let seuil = SEUIL_ABSENCES_DEFAUT;
        if n < seuil {
            return Ok(());
        }
        let signal = self
            .signaux
            .upsert_ouvert(
                etablissement_id,
                eleve_id,
                &annee,
                seuil as i32,
                n as i32,
            )
            .await?;
        if !signal.statut.est_actif() {
            return Ok(());
        }

        // Alerte famille (messagerie) — une seule fois par signal.
        let mut fil_alerte_id = signal.fil_alerte_id;
        if fil_alerte_id.is_none() {
            if let Some(acteur) = self
                .auth
                .first_member_with_roles(
                    etablissement_id,
                    &["cpe", "direction", "admin", "administratif"],
                )
                .await
                .ok()
                .flatten()
            {
                let eleve = self.eleves.get(etablissement_id, eleve_id).await?;
                let (prenom, nom) = eleve
                    .as_ref()
                    .map(|e| (e.prenom.as_str(), e.nom.as_str()))
                    .unwrap_or(("l'élève", ""));
                let sujet = sujet_alerte_seuil(n as i32, seuil as i32);
                let corps = corps_alerte_seuil(
                    prenom,
                    nom,
                    n as i32,
                    seuil as i32,
                    &annee,
                );
                let (fil, msg) = plan_ouvrir_fil(OuvrirFil {
                    etablissement_id,
                    eleve_id,
                    sujet: sujet.clone(),
                    premier_message: corps.clone(),
                    actor_user_id: acteur,
                })?;
                self.messages.insert_fil(&fil).await?;
                self.messages.insert_message(&msg).await?;
                if self
                    .signaux
                    .set_fil_alerte(etablissement_id, signal.id, fil.id)
                    .await?
                {
                    fil_alerte_id = Some(fil.id);
                    self.events
                        .publish(MetierEventDraft {
                            etablissement_id,
                            type_name: types::MSG_FIL_OUVERT.to_string(),
                            aggregate: AggregateKind::MsgFil,
                            aggregate_id: fil.id,
                            eleve_id: Some(eleve_id),
                            payload: json!({
                                "sujet": fil.sujet,
                                "source": "absence_seuil",
                                "signal_id": signal.id,
                                "compteur": n,
                                "seuil": seuil,
                            }),
                            actor_user_id: Some(acteur.to_string()),
                        })
                        .await?;
                }
                let _ = self
                    .notify_parents_mail(
                        etablissement_id,
                        eleve_id,
                        "absence_seuil",
                        &sujet,
                        &corps,
                        "absence_signal",
                        signal.id,
                        Some(acteur),
                        vec![],
                    )
                    .await;
                let _ = self
                    .notify_parents_push(
                        etablissement_id,
                        eleve_id,
                        "absence_seuil",
                        &sujet,
                        "Seuil d'absences atteint — consultez le portail famille.",
                        "absence_signal",
                        signal.id,
                        Some(acteur),
                    )
                    .await;
            }
        }

        if signal.statut == SignalAbsenceStatut::Ouvert {
            self.events
                .publish(MetierEventDraft {
                    etablissement_id,
                    type_name: types::ELEVE_ABSENCE_SEUIL_ATTEINT.to_string(),
                    aggregate: AggregateKind::Absence,
                    aggregate_id: signal.id,
                    eleve_id: Some(eleve_id),
                    payload: json!({
                        "annee_scolaire": annee,
                        "seuil": seuil,
                        "compteur": n,
                        "fil_alerte_id": fil_alerte_id,
                    }),
                    actor_user_id: None,
                })
                .await?;
        }
        Ok(())
    }

    pub fn role_can_trancher_justificatif(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_trancher_justificatif())
            .unwrap_or(false)
    }

    pub fn role_can_annuler_vers_present(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_annuler_absence_vers_present())
            .unwrap_or(false)
    }

    pub async fn accepter_justificatif(
        &self,
        cmd: DecisionAbsence,
    ) -> Result<Absence, EntDbError> {
        let current = self
            .absences
            .get(cmd.etablissement_id, cmd.absence_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_accepter_justificatif(&current, cmd)?;
        self.absences.upsert(&planned.absence).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        Ok(planned.absence)
    }

    pub async fn refuser_justificatif(
        &self,
        cmd: DecisionAbsence,
    ) -> Result<Absence, EntDbError> {
        let current = self
            .absences
            .get(cmd.etablissement_id, cmd.absence_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_refuser_justificatif(&current, cmd)?;
        self.absences.upsert(&planned.absence).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        Ok(planned.absence)
    }

    pub async fn annuler_absence_vers_present(
        &self,
        cmd: DecisionAbsence,
    ) -> Result<Absence, EntDbError> {
        let current = self
            .absences
            .get(cmd.etablissement_id, cmd.absence_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_annuler_vers_present(&current, cmd)?;
        self.absences.upsert(&planned.absence).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        Ok(planned.absence)
    }

    pub async fn accepter_justificatif_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        absence_id: Uuid,
        note: Option<String>,
    ) -> Result<Absence, EntDbError> {
        if !Self::role_can_trancher_justificatif(role) {
            return Err(EntDbError::Forbidden);
        }
        self.accepter_justificatif(DecisionAbsence {
            etablissement_id,
            absence_id,
            actor_user_id,
            note,
        })
        .await
    }

    pub async fn refuser_justificatif_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        absence_id: Uuid,
        note: Option<String>,
    ) -> Result<Absence, EntDbError> {
        if !Self::role_can_trancher_justificatif(role) {
            return Err(EntDbError::Forbidden);
        }
        self.refuser_justificatif(DecisionAbsence {
            etablissement_id,
            absence_id,
            actor_user_id,
            note,
        })
        .await
    }

    pub async fn annuler_absence_vers_present_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        absence_id: Uuid,
        note: Option<String>,
    ) -> Result<Absence, EntDbError> {
        if !Self::role_can_annuler_vers_present(role) {
            return Err(EntDbError::Forbidden);
        }
        self.annuler_absence_vers_present(DecisionAbsence {
            etablissement_id,
            absence_id,
            actor_user_id,
            note,
        })
        .await
    }

    pub async fn count_absences_depuis(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        depuis: NaiveDate,
    ) -> Result<i64, EntDbError> {
        Ok(self
            .absences
            .count_actives_depuis(etablissement_id, eleve_id, depuis)
            .await?)
    }

    pub async fn list_signaux_absences_ouverts(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<SignalAbsence>, EntDbError> {
        Ok(self.signaux.list_actifs(etablissement_id).await?)
    }

    pub async fn signal_absence_ouvert(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<SignalAbsence>, EntDbError> {
        Ok(self
            .signaux
            .get_ouvert(etablissement_id, eleve_id, &annee_scolaire_de(jour))
            .await?)
    }

    pub fn role_can_gerer_signaux_vs(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_trancher_justificatif())
            .unwrap_or(false)
    }

    pub async fn prendre_en_charge_signal_for_actor(
        &self,
        etablissement_id: Uuid,
        signal_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<SignalAbsence, EntDbError> {
        if !Self::role_can_gerer_signaux_vs(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .signaux
            .get(etablissement_id, signal_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let updated = plan_prendre_en_charge_signal(
            &current,
            &PrendreEnChargeSignal {
                etablissement_id,
                signal_id,
                actor_user_id,
            },
        )?;
        self.signaux.update_workflow(&updated).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ELEVE_ABSENCE_SIGNAL_EN_COURS.to_string(),
                aggregate: AggregateKind::Absence,
                aggregate_id: updated.id,
                eleve_id: Some(updated.eleve_id),
                payload: json!({
                    "compteur": updated.compteur,
                    "seuil": updated.seuil,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(updated)
    }

    pub async fn clore_signal_absence_for_actor(
        &self,
        etablissement_id: Uuid,
        signal_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        note: String,
    ) -> Result<SignalAbsence, EntDbError> {
        if !Self::role_can_gerer_signaux_vs(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .signaux
            .get(etablissement_id, signal_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let updated = plan_clore_signal_absence(
            &current,
            &CloreSignalAbsence {
                etablissement_id,
                signal_id,
                note,
                actor_user_id,
            },
        )?;
        self.signaux.update_workflow(&updated).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ELEVE_ABSENCE_SIGNAL_CLOS.to_string(),
                aggregate: AggregateKind::Absence,
                aggregate_id: updated.id,
                eleve_id: Some(updated.eleve_id),
                payload: json!({
                    "compteur": updated.compteur,
                    "note_vs": updated.note_vs,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(updated)
    }

    /// CPE : crée (ou réutilise) une commission VS et convoque l'élève du signal seuil.
    /// Lie `signal.commission_dossier_id` — idempotent si déjà lié.
    pub async fn convoquer_commission_depuis_signal_for_actor(
        &self,
        etablissement_id: Uuid,
        signal_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        jour: NaiveDate,
        heure: chrono::NaiveTime,
    ) -> Result<(CommissionVs, CommissionDossier, SignalAbsence), EntDbError> {
        if !Self::role_can_gerer_signaux_vs(role) {
            return Err(EntDbError::Forbidden);
        }
        let mut signal = self
            .signaux
            .get(etablissement_id, signal_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !signal.statut.est_actif() {
            return Err(DomainError::SignalAbsenceDejaClos.into());
        }

        // Déjà lié → retourne commission + dossier existants.
        if let Some(dossier_id) = signal.commission_dossier_id {
            let dossier = self
                .sanctions
                .get_commission_dossier(etablissement_id, dossier_id)
                .await?
                .ok_or(DomainError::NotFound)?;
            let commission = self
                .sanctions
                .get_commission(etablissement_id, dossier.commission_id)
                .await?
                .ok_or(DomainError::NotFound)?;
            return Ok((commission, dossier, signal));
        }

        // Prise en charge automatique si encore ouvert.
        if signal.statut == SignalAbsenceStatut::Ouvert {
            signal = plan_prendre_en_charge_signal(
                &signal,
                &PrendreEnChargeSignal {
                    etablissement_id,
                    signal_id,
                    actor_user_id,
                },
            )?;
            self.signaux.update_workflow(&signal).await?;
        }

        let eleve = self
            .eleves
            .get(etablissement_id, signal.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let objet = format!(
            "Absences répétées — {} {} ({}/{})",
            eleve.prenom, eleve.nom, signal.compteur, signal.seuil
        );
        let commission = self
            .creer_commission_for_actor(
                role,
                CreerCommissionVs {
                    etablissement_id,
                    jour,
                    heure,
                    objet,
                    lieu: "Bureau CPE".into(),
                    actor_user_id,
                },
            )
            .await?;
        let motifs = format!(
            "Seuil d'absences atteint : {} absences (seuil {}) — année {}.",
            signal.compteur, signal.seuil, signal.annee_scolaire
        );
        let dossier = self
            .convoquer_commission_for_actor(
                role,
                ConvoquerCommission {
                    etablissement_id,
                    commission_id: commission.id,
                    eleve_id: signal.eleve_id,
                    motifs,
                    note_interne: format!("Signal VS {}", signal.id),
                    sanction_id: None,
                    incident_id: None,
                    signale_famille: true,
                    actor_user_id,
                },
            )
            .await?;
        let linked = plan_lier_signal_commission(&signal, dossier.id)?;
        let posed = self
            .signaux
            .set_commission_dossier(etablissement_id, signal.id, dossier.id)
            .await?;
        if !posed && linked.commission_dossier_id != Some(dossier.id) {
            return Err(DomainError::SignalAbsenceInvalide.into());
        }
        let signal = self
            .signaux
            .get(etablissement_id, signal_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::VS_SIGNAL_COMMISSION.to_string(),
                aggregate: AggregateKind::Commission,
                aggregate_id: dossier.id,
                eleve_id: Some(signal.eleve_id),
                payload: json!({
                    "signal_id": signal.id,
                    "commission_id": commission.id,
                    "dossier_id": dossier.id,
                    "compteur": signal.compteur,
                    "seuil": signal.seuil,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok((commission, dossier, signal))
    }

    pub async fn list_motifs_absence(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<AbsenceMotif>, EntDbError> {
        Ok(self.motifs.list_disponibles(etablissement_id).await?)
    }

    pub async fn list_motifs_absence_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<AbsenceMotif>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() && parsed != Role::Parent {
            return Err(EntDbError::Forbidden);
        }
        self.list_motifs_absence(etablissement_id).await
    }

    pub async fn create_motif_absence_etab(
        &self,
        etablissement_id: Uuid,
        code: &str,
        libelle: &str,
        actor_user_id: Option<String>,
    ) -> Result<AbsenceMotif, EntDbError> {
        let code = code.trim().to_ascii_lowercase();
        let libelle = libelle.trim().to_string();
        if code.len() < 2
            || code.len() > 40
            || !code
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            || libelle.is_empty()
        {
            return Err(EntDbError::MotifInvalide);
        }
        let motif = self
            .motifs
            .create_etab(etablissement_id, &code, &libelle)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ABSENCE_MOTIF_ETAB_CREATED.to_string(),
                aggregate: AggregateKind::Absence,
                aggregate_id: etablissement_id,
                eleve_id: None,
                payload: json!({ "code": motif.code, "libelle": motif.libelle }),
                actor_user_id,
            })
            .await?;
        Ok(motif)
    }

    pub async fn create_motif_absence_etab_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        code: &str,
        libelle: &str,
        actor_user_id: Uuid,
    ) -> Result<AbsenceMotif, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves()
            && !matches!(parsed, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe)
        {
            return Err(EntDbError::Forbidden);
        }
        self.create_motif_absence_etab(
            etablissement_id,
            code,
            libelle,
            Some(actor_user_id.to_string()),
        )
        .await
    }

    pub async fn link_eleve_compte(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        user_id: Uuid,
    ) -> Result<EleveCompte, EntDbError> {
        self.eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let compte = self
            .eleve_comptes
            .link(etablissement_id, eleve_id, user_id)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ELEVE_COMPTE_LIE.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve_id,
                eleve_id: Some(eleve_id),
                payload: json!({ "user_id": user_id.to_string() }),
                actor_user_id: Some(user_id.to_string()),
            })
            .await?;
        Ok(compte)
    }

    /// Résultat d'un provisionnement : invitation + e-mail + mdp temporaire (une fois).
    pub async fn inviter_compte_eleve_for_actor(
        &self,
        role: &str,
        cmd: InviterCompteEleve,
    ) -> Result<(EleveInvitation, String, String), EntDbError> {
        if !Role::parse(role)
            .map(|r| r.can_manage_eleves())
            .unwrap_or(false)
        {
            return Err(EntDbError::Forbidden);
        }
        let eleve = self
            .eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let deja = self
            .eleve_comptes
            .has_compte(cmd.etablissement_id, cmd.eleve_id)
            .await?;
        let mut plan = plan_inviter_compte_eleve(&eleve, deja, cmd)?;
        let user = self
            .register_user(
                &plan.email,
                &plan.mot_de_passe_temporaire,
                Some(&plan.display_name),
                plan.invitation.etablissement_id,
                Role::Eleve.as_str(),
            )
            .await?;
        self.link_eleve_compte(
            plan.invitation.etablissement_id,
            plan.invitation.eleve_id,
            user.id,
        )
        .await?;
        plan.invitation.activated_user_id = Some(user.id);
        self.eleve_comptes
            .insert_invitation(&plan.invitation)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: plan.invitation.etablissement_id,
                type_name: types::ELEVE_COMPTE_INVITE.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: plan.invitation.eleve_id,
                eleve_id: Some(plan.invitation.eleve_id),
                payload: json!({
                    "invitation_id": plan.invitation.id,
                    "email": plan.email,
                    "user_id": user.id,
                }),
                actor_user_id: Some(plan.invitation.invited_by_user_id.to_string()),
            })
            .await?;
        Ok((
            plan.invitation,
            plan.email,
            plan.mot_de_passe_temporaire,
        ))
    }

    pub async fn list_eleves_sans_compte_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<Eleve>, EntDbError> {
        if !Role::parse(role)
            .map(|r| r.can_manage_eleves())
            .unwrap_or(false)
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .eleve_comptes
            .list_eleves_sans_compte(etablissement_id, 100)
            .await?)
    }

    /// Provisionne tous les élèves actifs sans compte (lot).
    pub async fn provisionner_comptes_manquants_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Vec<(EleveInvitation, String, String)>, EntDbError> {
        let sans = self
            .list_eleves_sans_compte_for_actor(etablissement_id, role)
            .await?;
        let mut out = Vec::new();
        for e in sans {
            let res = self
                .inviter_compte_eleve_for_actor(
                    role,
                    InviterCompteEleve {
                        etablissement_id,
                        eleve_id: e.id,
                        email: None,
                        actor_user_id,
                    },
                )
                .await?;
            out.push(res);
        }
        Ok(out)
    }

    pub async fn list_invitations_compte_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<EleveInvitation>, EntDbError> {
        if !Role::parse(role)
            .map(|r| r.can_manage_eleves())
            .unwrap_or(false)
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .eleve_comptes
            .list_invitations(etablissement_id, 50)
            .await?)
    }

    pub async fn eleve_id_for_compte(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Uuid>, EntDbError> {
        Ok(self
            .eleve_comptes
            .eleve_id_for_user(etablissement_id, user_id)
            .await?)
    }

    pub fn role_can_manage_documents(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_manage_documents())
            .unwrap_or(false)
    }

    pub async fn deposer_document(
        &self,
        cmd: DeposerDocument,
    ) -> Result<EleveDocument, EntDbError> {
        self.deposer_document_with_contenu(cmd, None).await
    }

    /// Dépose un document ; si `contenu` est fourni, écrit l’objet et pose `storage_key`.
    pub async fn deposer_document_with_contenu(
        &self,
        cmd: DeposerDocument,
        contenu: Option<&[u8]>,
    ) -> Result<EleveDocument, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let mut doc = plan_deposer_document(cmd)?;
        if let Some(bytes) = contenu {
            let ext = match doc.mime_type.as_deref() {
                Some("application/pdf") => "pdf",
                Some("image/png") => "png",
                Some("image/jpeg") | Some("image/jpg") => "jpg",
                Some("text/plain") => "txt",
                _ => "bin",
            };
            let key = ObjectStore::key_for(doc.etablissement_id, "documents", doc.id, ext);
            self.objects.put(&key, bytes)?;
            doc.storage_key = Some(key);
        }
        self.documents.upsert_document(&doc).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: doc.etablissement_id,
                type_name: types::ELEVE_DOCUMENT_UPLOADED.to_string(),
                aggregate: AggregateKind::Document,
                aggregate_id: doc.id,
                eleve_id: Some(doc.eleve_id),
                payload: json!({
                    "tiroir": doc.tiroir.as_str(),
                    "titre": doc.titre,
                    "confidentialite": doc.confidentialite.as_str(),
                    "storage_key": doc.storage_key,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(doc)
    }

    pub async fn deposer_document_for_actor(
        &self,
        role: &str,
        mut cmd: DeposerDocument,
        contenu: Option<&[u8]>,
    ) -> Result<EleveDocument, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if Self::role_can_manage_documents(role) {
            return self.deposer_document_with_contenu(cmd, contenu).await;
        }
        if matches!(parsed, Role::Parent) {
            if self
                .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
                .await?
                .is_none()
            {
                return Err(EntDbError::Forbidden);
            }
            // Famille : dépôt limité au tiroir famille, confidentialité standard.
            cmd.tiroir = DocTiroir::Famille;
            cmd.confidentialite = DocConfidentialite::Standard;
            return self.deposer_document_with_contenu(cmd, contenu).await;
        }
        if matches!(parsed, Role::Eleve) {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(cmd.etablissement_id, cmd.actor_user_id)
                .await?;
            if linked != Some(cmd.eleve_id) {
                return Err(EntDbError::Forbidden);
            }
            cmd.tiroir = DocTiroir::Famille;
            cmd.confidentialite = DocConfidentialite::Standard;
            return self.deposer_document_with_contenu(cmd, contenu).await;
        }
        Err(EntDbError::Forbidden)
    }

    /// Télécharge le fichier d’un document si l’acteur peut le lire.
    pub async fn telecharger_document_bytes_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        document_id: Uuid,
    ) -> Result<(EleveDocument, Vec<u8>), EntDbError> {
        let doc = self
            .documents
            .get_document(etablissement_id, document_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !self
            .actor_can_read_document(etablissement_id, actor_user_id, role, &doc)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let Some(key) = doc.storage_key.as_ref() else {
            return Err(DomainError::NotFound.into());
        };
        match self.objects.get(key) {
            Ok(bytes) => Ok((doc, bytes)),
            Err(ObjectStoreError::NotFound(_)) => {
                // Lab : métadonnée sans octets (ancien dépôt) → stub lisible.
                let stub = format!(
                    "Document ENT (lab) — {}\nTiroir: {}\nCode: {}\n",
                    doc.titre,
                    doc.tiroir.as_str(),
                    doc.code.as_deref().unwrap_or("—")
                );
                Ok((doc, stub.into_bytes()))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn demander_grant_document(
        &self,
        cmd: DemanderGrant,
    ) -> Result<DocumentGrant, EntDbError> {
        self.documents
            .get_document(cmd.etablissement_id, cmd.document_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let grant = plan_demander_grant(cmd)?;
        self.documents.upsert_grant(&grant).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: grant.etablissement_id,
                type_name: types::ELEVE_DOCUMENT_GRANT_REQUESTED.to_string(),
                aggregate: AggregateKind::Document,
                aggregate_id: grant.document_id,
                eleve_id: None,
                payload: json!({
                    "grant_id": grant.id.to_string(),
                    "grantee_user_id": grant.grantee_user_id.to_string(),
                    "duration_days": grant.duration_days,
                }),
                actor_user_id: Some(grant.grantee_user_id.to_string()),
            })
            .await?;
        Ok(grant)
    }

    /// Staff / parent / élève : demande un accès pour soi-même sur un document verrouillé.
    pub async fn demander_grant_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        document_id: Uuid,
        actor_user_id: Uuid,
        duration_days: i32,
        note: Option<String>,
    ) -> Result<DocumentGrant, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let doc = self
            .documents
            .get_document(etablissement_id, document_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        match parsed {
            r if r.is_staff() => {}
            Role::Parent => {
                if self
                    .lien_parent_eleve(etablissement_id, actor_user_id, doc.eleve_id)
                    .await?
                    .is_none()
                {
                    return Err(EntDbError::Forbidden);
                }
            }
            Role::Eleve => {
                let linked = self
                    .eleve_comptes
                    .eleve_id_for_user(etablissement_id, actor_user_id)
                    .await?;
                if linked != Some(doc.eleve_id) {
                    return Err(EntDbError::Forbidden);
                }
            }
            _ => return Err(EntDbError::Forbidden),
        }
        self.demander_grant_document(DemanderGrant {
            etablissement_id,
            document_id,
            grantee_user_id: actor_user_id,
            duration_days,
            note,
        })
        .await
    }

    pub async fn list_grants_pending_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<DocumentGrant>, EntDbError> {
        if !Self::role_can_manage_documents(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.documents.list_grants_pending(etablissement_id).await?)
    }

    /// Métadonnées documents : tous les docs du dossier + flag `accessible` (staff / parent / élève).
    pub async fn list_documents_meta_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<(EleveDocument, bool)>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if parsed.is_staff() {
            self.eleves
                .get(etablissement_id, eleve_id)
                .await?
                .ok_or(DomainError::NotFound)?;
        } else if matches!(parsed, Role::Parent) {
            if self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?
                .is_none()
            {
                return Err(EntDbError::Forbidden);
            }
        } else if matches!(parsed, Role::Eleve) {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        } else {
            return Err(EntDbError::Forbidden);
        }
        let all = self
            .documents
            .list_documents_eleve(etablissement_id, eleve_id)
            .await?;
        let mut out = Vec::new();
        for mut doc in all {
            let accessible = self
                .actor_can_read_document(etablissement_id, actor_user_id, role, &doc)
                .await?;
            if !accessible {
                doc.storage_key = None;
                doc.mime_type = None;
            }
            out.push((doc, accessible));
        }
        Ok(out)
    }

    pub async fn decider_grant_document(
        &self,
        cmd: DeciderGrant,
    ) -> Result<DocumentGrant, EntDbError> {
        let current = self
            .documents
            .get_grant(cmd.etablissement_id, cmd.grant_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let accepter = cmd.accepter;
        let decided = plan_decider_grant(&current, cmd)?;
        self.documents.upsert_grant(&decided).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: decided.etablissement_id,
                type_name: types::ELEVE_DOCUMENT_GRANT_DECIDED.to_string(),
                aggregate: AggregateKind::Document,
                aggregate_id: decided.document_id,
                eleve_id: None,
                payload: json!({
                    "grant_id": decided.id.to_string(),
                    "status": decided.status.as_str(),
                    "accepter": accepter,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(decided)
    }

    pub async fn decider_grant_for_actor(
        &self,
        role: &str,
        cmd: DeciderGrant,
    ) -> Result<DocumentGrant, EntDbError> {
        if !Self::role_can_manage_documents(role) {
            return Err(EntDbError::Forbidden);
        }
        self.decider_grant_document(cmd).await
    }

    pub async fn ajouter_dispositif(
        &self,
        cmd: AjouterDispositif,
    ) -> Result<EleveDispositif, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if let Some(doc_id) = cmd.document_id {
            let doc = self
                .documents
                .get_document(cmd.etablissement_id, doc_id)
                .await?
                .ok_or(DomainError::NotFound)?;
            if doc.eleve_id != cmd.eleve_id {
                return Err(EntDbError::Domain(DomainError::NotFound));
            }
        }
        let actor = cmd.actor_user_id.clone();
        let d = plan_ajouter_dispositif(cmd)?;
        self.documents.upsert_dispositif(&d).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: d.etablissement_id,
                type_name: types::ELEVE_DISPOSITIF_AJOUTE.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: d.eleve_id,
                eleve_id: Some(d.eleve_id),
                payload: json!({
                    "kind": d.kind.as_str(),
                    "document_id": d.document_id.map(|id| id.to_string()),
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(d)
    }

    pub async fn actor_can_read_document(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        doc: &EleveDocument,
    ) -> Result<bool, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(false);
        };
        if parsed.can_manage_documents() {
            return Ok(true);
        }
        if doc.confidentialite == DocConfidentialite::Sante && parsed.can_read_sante_docs() {
            return Ok(true);
        }
        // Parent : tiroir famille + standard seulement (sans grant).
        if parsed == Role::Parent
            && doc.tiroir == DocTiroir::Famille
            && doc.confidentialite == DocConfidentialite::Standard
        {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, doc.eleve_id)
                .await?;
            return Ok(lien.is_some());
        }
        // Élève : docs famille standard sur soi.
        if parsed == Role::Eleve
            && doc.tiroir == DocTiroir::Famille
            && doc.confidentialite == DocConfidentialite::Standard
        {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            return Ok(linked == Some(doc.eleve_id));
        }
        // Sinon : grant actif obligatoire (prof / CPE / etc. — circuit PAP).
        let actifs = self
            .documents
            .list_grants_actifs_user_doc(
                etablissement_id,
                doc.id,
                actor_user_id,
                Utc::now(),
            )
            .await?;
        Ok(!actifs.is_empty())
    }

    async fn documents_visibles_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<EleveDocument>, EntDbError> {
        let all = self
            .documents
            .list_documents_eleve(etablissement_id, eleve_id)
            .await?;
        let mut out = Vec::new();
        for doc in all {
            if self
                .actor_can_read_document(etablissement_id, actor_user_id, role, &doc)
                .await?
            {
                out.push(doc);
            }
        }
        Ok(out)
    }

    pub fn role_can_manage_repas(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_manage_repas())
            .unwrap_or(false)
    }

    pub fn role_can_manage_bourses(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_manage_bourses())
            .unwrap_or(false)
    }

    pub async fn creer_repas_activite(
        &self,
        cmd: CreerRepasActivite,
    ) -> Result<RepasActivite, EntDbError> {
        let actor = cmd.actor_user_id.clone();
        let act = plan_creer_repas_activite(cmd)?;
        self.repas.upsert_activite(&act).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: act.etablissement_id,
                type_name: types::REPAS_ACTIVITE_CREATED.to_string(),
                aggregate: AggregateKind::Repas,
                aggregate_id: act.id,
                eleve_id: None,
                payload: json!({
                    "code": act.code,
                    "kind": act.kind.as_str(),
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(act)
    }

    pub async fn creer_repas_activite_for_actor(
        &self,
        role: &str,
        cmd: CreerRepasActivite,
    ) -> Result<RepasActivite, EntDbError> {
        if !Self::role_can_manage_repas(role) {
            return Err(EntDbError::Forbidden);
        }
        self.creer_repas_activite(cmd).await
    }

    pub async fn list_repas_activites(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<RepasActivite>, EntDbError> {
        Ok(self.repas.list_activites(etablissement_id).await?)
    }

    pub async fn poser_grille_repas(
        &self,
        cmd: PoserGrilleRepas,
    ) -> Result<EleveGrilleRepas, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.repas
            .get_activite(cmd.etablissement_id, cmd.activite_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id.clone();
        let grille = plan_poser_grille_repas(cmd)?;
        self.repas.upsert_grille(&grille).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: grille.etablissement_id,
                type_name: types::ELEVE_GRILLE_REPAS_POSED.to_string(),
                aggregate: AggregateKind::Repas,
                aggregate_id: grille.id,
                eleve_id: Some(grille.eleve_id),
                payload: json!({
                    "activite_id": grille.activite_id.to_string(),
                    "date_debut": grille.date_debut.to_string(),
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(grille)
    }

    pub async fn poser_grille_repas_for_actor(
        &self,
        role: &str,
        cmd: PoserGrilleRepas,
    ) -> Result<EleveGrilleRepas, EntDbError> {
        if !Self::role_can_manage_repas(role) {
            return Err(EntDbError::Forbidden);
        }
        self.poser_grille_repas(cmd).await
    }

    /// Cantine : inscrit midi ce jour, sans dispense, ET pas absent (fait partagé `ent_absence`).
    pub async fn eleve_inscrit_midi_present(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let grilles = self
            .repas
            .list_grilles_eleve(etablissement_id, eleve_id)
            .await?;
        let activites = self.repas.list_activites(etablissement_id).await?;
        let dispenses = self
            .repas
            .list_dispenses_periode(etablissement_id, eleve_id, jour, jour)
            .await?;
        if !inscrit_activite_kind_au_jour_avec_dispenses(
            &grilles,
            &activites,
            &dispenses,
            RepasActiviteKind::Midi,
            jour,
        ) {
            return Ok(false);
        }
        let absent = self.eleve_est_absent(etablissement_id, eleve_id, jour).await?;
        Ok(!absent)
    }

    pub async fn poser_dispense_repas(
        &self,
        cmd: PoserDispenseRepas,
    ) -> Result<EleveDispenseRepas, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let act = self
            .repas
            .get_activite(cmd.etablissement_id, cmd.activite_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !act.actif {
            return Err(DomainError::DispenseRepasInvalide.into());
        }
        let grilles = self
            .repas
            .list_grilles_eleve(cmd.etablissement_id, cmd.eleve_id)
            .await?;
        let covered = grilles.iter().any(|g| {
            g.activite_id == cmd.activite_id
                && domain_eleve::grille_couvre_jour(g, cmd.jour)
        });
        if !covered {
            return Err(DomainError::DispenseRepasInvalide.into());
        }
        let actor = cmd.actor_user_id.clone();
        let dispense = plan_poser_dispense_repas(cmd)?;
        self.repas.insert_dispense(&dispense).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: dispense.etablissement_id,
                type_name: types::ELEVE_DISPENSE_REPAS_POSED.to_string(),
                aggregate: AggregateKind::Repas,
                aggregate_id: dispense.id,
                eleve_id: Some(dispense.eleve_id),
                payload: json!({
                    "activite_id": dispense.activite_id.to_string(),
                    "jour": dispense.jour.to_string(),
                    "source": dispense.source.as_str(),
                    "motif": dispense.motif,
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(dispense)
    }

    /// Parent (prévenance) ou staff repas / admini.
    pub async fn poser_dispense_repas_for_actor(
        &self,
        role: &str,
        cmd: PoserDispenseRepas,
    ) -> Result<EleveDispenseRepas, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let actor_id = cmd
            .actor_user_id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or(EntDbError::Forbidden)?;
        match parsed {
            Role::Parent => {
                if cmd.source != DispenseSource::Parent {
                    return Err(EntDbError::Forbidden);
                }
                let lien = self
                    .lien_parent_eleve(cmd.etablissement_id, actor_id, cmd.eleve_id)
                    .await?;
                if lien.is_none() {
                    return Err(EntDbError::Forbidden);
                }
            }
            _ if Self::role_can_manage_repas(role) => {
                if cmd.source != DispenseSource::Staff {
                    return Err(EntDbError::Forbidden);
                }
            }
            _ => return Err(EntDbError::Forbidden),
        }
        self.poser_dispense_repas(cmd).await
    }

    pub async fn annuler_dispense_repas_for_actor(
        &self,
        etablissement_id: Uuid,
        dispense_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<EleveDispenseRepas, EntDbError> {
        let current = self
            .repas
            .get_dispense(etablissement_id, dispense_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        match parsed {
            Role::Parent => {
                if current.source != DispenseSource::Parent {
                    return Err(EntDbError::Forbidden);
                }
                let lien = self
                    .lien_parent_eleve(etablissement_id, actor_user_id, current.eleve_id)
                    .await?;
                if lien.is_none() {
                    return Err(EntDbError::Forbidden);
                }
            }
            _ if Self::role_can_manage_repas(role) => {}
            _ => return Err(EntDbError::Forbidden),
        }
        let annulee = plan_annuler_dispense_repas(&current, etablissement_id, dispense_id)?;
        self.repas.update_dispense_annulee(&annulee).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ELEVE_DISPENSE_REPAS_ANNULEE.to_string(),
                aggregate: AggregateKind::Repas,
                aggregate_id: annulee.id,
                eleve_id: Some(annulee.eleve_id),
                payload: json!({
                    "activite_id": annulee.activite_id.to_string(),
                    "jour": annulee.jour.to_string(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(annulee)
    }

    /// Volume cantine facturable (jours grille − dispenses) sur une période.
    pub async fn compter_repas_facturables_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        kind: RepasActiviteKind,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<u32, EntDbError> {
        let grilles = self
            .repas
            .list_grilles_eleve(etablissement_id, eleve_id)
            .await?;
        let activites = self.repas.list_activites(etablissement_id).await?;
        let dispenses = self
            .repas
            .list_dispenses_periode(etablissement_id, eleve_id, debut, fin)
            .await?;
        Ok(compter_jours_facturables(
            &grilles, &activites, &dispenses, kind, debut, fin,
        )?)
    }

    pub async fn list_dispenses_repas_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<EleveDispenseRepas>, EntDbError> {
        Ok(self
            .repas
            .list_dispenses_eleve(etablissement_id, eleve_id, true)
            .await?)
    }

    /// Effectifs cuisine prévus (grille − prévenances) par jour × activité.
    pub async fn volumes_repas_periode(
        &self,
        cmd: ExporterVolumesRepas,
    ) -> Result<Vec<VolumeRepasJour>, EntDbError> {
        plan_exporter_volumes_repas(&cmd)?;
        let grilles = self.repas.list_grilles_etab(cmd.etablissement_id).await?;
        let activites = self.repas.list_activites(cmd.etablissement_id).await?;
        let dispenses = self
            .repas
            .list_dispenses_etab_periode(cmd.etablissement_id, cmd.debut, cmd.fin)
            .await?;
        Ok(agreger_volumes_repas(
            &grilles,
            &activites,
            &dispenses,
            cmd.debut,
            cmd.fin,
            cmd.kind,
        )?)
    }

    pub async fn export_volumes_cuisine_for_actor(
        &self,
        role: &str,
        cmd: ExporterVolumesRepas,
    ) -> Result<(Vec<VolumeRepasJour>, Vec<u8>), EntDbError> {
        if !Self::role_can_manage_repas(role) && !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let lignes = self.volumes_repas_periode(cmd).await?;
        let csv = render_volumes_cuisine_csv(&lignes);
        Ok((lignes, csv))
    }

    pub fn role_can_scan_repas(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_scan_repas())
            .unwrap_or(false)
    }

    /// Pointage badge cantine : résout l'élève, contrôle grille/dispense, enregistre le passage.
    pub async fn scan_badge_repas_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        badge: &str,
        kind: RepasActiviteKind,
        jour: NaiveDate,
        actor_user_id: Uuid,
    ) -> Result<(PassageRepas, domain_eleve::Eleve), EntDbError> {
        if !Self::role_can_scan_repas(role) {
            return Err(EntDbError::Forbidden);
        }
        let eleve = self
            .eleves
            .get_by_badge(etablissement_id, badge)
            .await?
            .ok_or(DomainError::BadgeRepasInconnu)?;
        let activites = self.repas.list_activites(etablissement_id).await?;
        let act = activites
            .iter()
            .find(|a| a.actif && a.kind == kind)
            .ok_or(DomainError::NotFound)?;
        let grilles = self
            .repas
            .list_grilles_eleve(etablissement_id, eleve.id)
            .await?;
        let dispenses = self
            .repas
            .list_dispenses_periode(etablissement_id, eleve.id, jour, jour)
            .await?;
        let deja = self
            .repas
            .has_passage_actif(etablissement_id, eleve.id, act.id, jour)
            .await?;
        let passage = plan_enregistrer_passage_badge(
            EnregistrerPassageBadge {
                etablissement_id,
                eleve_id: eleve.id,
                activite_id: act.id,
                jour,
                badge_lu: badge.to_string(),
                source: PassageSource::Badge,
                actor_user_id: Some(actor_user_id.to_string()),
            },
            &grilles,
            &dispenses,
            deja,
        )?;
        self.repas.insert_passage(&passage).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ELEVE_REPAS_PASSAGE.to_string(),
                aggregate: AggregateKind::Repas,
                aggregate_id: passage.id,
                eleve_id: Some(eleve.id),
                payload: json!({
                    "activite_id": act.id.to_string(),
                    "kind": kind.as_str(),
                    "jour": jour.to_string(),
                    "badge": passage.badge_lu,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok((passage, eleve))
    }

    pub async fn list_passages_repas_jour_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<PassageRepas>, EntDbError> {
        if !Self::role_can_scan_repas(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.repas.list_passages_jour(etablissement_id, jour).await?)
    }

    /// Historique passages badge d'un élève (staff cantine / parent lié).
    pub async fn list_passages_repas_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        eleve_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<PassageRepas>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_scan_repas(role) || parsed.can_manage_eleves();
        if !staff {
            if matches!(parsed, Role::Parent) {
                if self
                    .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                    .await?
                    .is_none()
                {
                    return Err(EntDbError::Forbidden);
                }
            } else if matches!(parsed, Role::Eleve) {
                let linked = self
                    .eleve_comptes
                    .eleve_id_for_user(etablissement_id, actor_user_id)
                    .await?;
                if linked != Some(eleve_id) {
                    return Err(EntDbError::Forbidden);
                }
            } else {
                return Err(EntDbError::Forbidden);
            }
        }
        if fin < debut {
            return Err(DomainError::NotFound.into());
        }
        Ok(self
            .repas
            .list_passages_eleve_periode(etablissement_id, eleve_id, debut, fin)
            .await?)
    }

    /// Aperçu + création facture cantine basée sur les passages badge d'un élève.
    pub async fn apercu_cantine_badge(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
        kind: RepasActiviteKind,
    ) -> Result<(u32, Option<Uuid>), EntDbError> {
        let activites = self.repas.list_activites(etablissement_id).await?;
        let act_id = activites
            .iter()
            .find(|a| a.actif && a.kind == kind)
            .map(|a| a.id);
        let n = self
            .repas
            .count_passages_eleve_periode(etablissement_id, eleve_id, debut, fin, act_id)
            .await?;
        Ok((n, act_id))
    }

    pub async fn creer_facture_cantine_badge_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        foyer_id: Uuid,
        eleve_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
        prix_unitaire_centimes: i64,
        numero: String,
        actor_user_id: Uuid,
        kind: RepasActiviteKind,
    ) -> Result<(Facture, Vec<FactureLigne>, u32), EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let (nb, _) = self
            .apercu_cantine_badge(etablissement_id, eleve_id, debut, fin, kind)
            .await?;
        let ligne = plan_ligne_cantine_depuis_passages(
            eleve_id,
            nb,
            prix_unitaire_centimes,
            Some(format!(
                "Cantine {}→{} ({} passage(s) badge)",
                debut.format("%d/%m"),
                fin.format("%d/%m"),
                nb
            )),
        )?;
        let (facture, lignes) = self
            .creer_facture_for_actor(
                role,
                CreerFacture {
                    etablissement_id,
                    foyer_id,
                    numero,
                    libelle: format!("Cantine badge {} → {}", debut, fin),
                    nature: Some(NATURE_CANTINE_BADGE.into()),
                    annee_scolaire: Some(annee_scolaire_de(debut)),
                    lignes: vec![ligne],
                    actor_user_id,
                },
            )
            .await?;
        Ok((facture, lignes, nb))
    }

    /// Preview public pour l'UI (authz facturation).
    pub async fn apercu_cantine_badge_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
        kind: RepasActiviteKind,
    ) -> Result<u32, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let (n, _) = self
            .apercu_cantine_badge(etablissement_id, eleve_id, debut, fin, kind)
            .await?;
        Ok(n)
    }

    pub async fn list_bourse_types(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<BourseType>, EntDbError> {
        Ok(self.repas.list_bourse_types(etablissement_id).await?)
    }

    pub async fn list_bourse_types_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<BourseType>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(Self::role_can_manage_bourses(role)
            || Self::role_can_manage_facturation(role)
            || parsed.can_manage_eleves()
            || parsed.can_read_all_eleves())
        {
            return Err(EntDbError::Forbidden);
        }
        self.list_bourse_types(etablissement_id).await
    }

    pub async fn creer_bourse_type(
        &self,
        cmd: CreerBourseType,
    ) -> Result<BourseType, EntDbError> {
        let actor = cmd.actor_user_id.clone();
        let t = plan_creer_bourse_type(cmd)?;
        self.repas.upsert_bourse_type(&t).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: t.etablissement_id,
                type_name: types::BOURSE_TYPE_CREATED.to_string(),
                aggregate: AggregateKind::Bourse,
                aggregate_id: t.id,
                eleve_id: None,
                payload: json!({ "code": t.code, "libelle": t.libelle }),
                actor_user_id: actor,
            })
            .await?;
        Ok(t)
    }

    pub async fn creer_bourse_type_for_actor(
        &self,
        role: &str,
        cmd: CreerBourseType,
    ) -> Result<BourseType, EntDbError> {
        if !Self::role_can_manage_bourses(role) {
            return Err(EntDbError::Forbidden);
        }
        self.creer_bourse_type(cmd).await
    }

    pub async fn affecter_bourse(&self, cmd: AffecterBourse) -> Result<EleveBourse, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.repas
            .get_bourse_type(cmd.etablissement_id, cmd.bourse_type_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id.clone();
        let b = plan_affecter_bourse(cmd)?;
        self.repas.upsert_bourse(&b).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: b.etablissement_id,
                type_name: types::ELEVE_BOURSE_AFFECTEE.to_string(),
                aggregate: AggregateKind::Bourse,
                aggregate_id: b.id,
                eleve_id: Some(b.eleve_id),
                payload: json!({
                    "echelon": b.echelon,
                    "avec_procuration": b.avec_procuration,
                    "status": b.status.as_str(),
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(b)
    }

    pub async fn affecter_bourse_for_actor(
        &self,
        role: &str,
        cmd: AffecterBourse,
    ) -> Result<EleveBourse, EntDbError> {
        if !Self::role_can_manage_bourses(role) {
            return Err(EntDbError::Forbidden);
        }
        self.affecter_bourse(cmd).await
    }

    pub async fn decider_bourse(&self, cmd: DeciderBourse) -> Result<EleveBourse, EntDbError> {
        let current = self
            .repas
            .get_bourse(cmd.etablissement_id, cmd.bourse_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id.clone();
        let b = plan_decider_bourse(&current, cmd)?;
        self.repas.upsert_bourse(&b).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: b.etablissement_id,
                type_name: types::ELEVE_BOURSE_DECIDEE.to_string(),
                aggregate: AggregateKind::Bourse,
                aggregate_id: b.id,
                eleve_id: Some(b.eleve_id),
                payload: json!({
                    "status": b.status.as_str(),
                    "avec_procuration": b.avec_procuration,
                    "deductible": bourse_deductible_facture(&b, today_utc()),
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(b)
    }

    pub async fn decider_bourse_for_actor(
        &self,
        role: &str,
        cmd: DeciderBourse,
    ) -> Result<EleveBourse, EntDbError> {
        if !Self::role_can_manage_bourses(role) {
            return Err(EntDbError::Forbidden);
        }
        self.decider_bourse(cmd).await
    }

    pub async fn bourses_deductibles_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<EleveBourse>, EntDbError> {
        let all = self.repas.list_bourses_eleve(etablissement_id, eleve_id).await?;
        Ok(all
            .into_iter()
            .filter(|b| bourse_deductible_facture(b, jour))
            .collect())
    }

    pub fn role_can_manage_continuum(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_manage_continuum())
            .unwrap_or(false)
    }

    pub async fn ouvrir_campagne_reinscription(
        &self,
        cmd: OuvrirCampagneReinscription,
    ) -> Result<ReinscriptionCampagne, EntDbError> {
        let actor = cmd.actor_user_id.clone();
        let planned = plan_ouvrir_campagne(cmd)?;
        // Upsert : si campagne déjà connue, on la réouvre avec le même id métier.
        let campagne = if let Some(existing) = self
            .continuum
            .get_campagne(planned.etablissement_id, &planned.annee_cible)
            .await?
        {
            ReinscriptionCampagne {
                id: existing.id,
                etablissement_id: planned.etablissement_id,
                annee_cible: planned.annee_cible,
                ouverte: true,
                acompte_centimes: existing.acompte_centimes,
            }
        } else {
            planned
        };
        self.continuum.upsert_campagne(&campagne).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: campagne.etablissement_id,
                type_name: types::REINSCRIPTION_CAMPAGNE_OUVERTE.to_string(),
                aggregate: AggregateKind::Continuum,
                aggregate_id: campagne.id,
                eleve_id: None,
                payload: json!({ "annee_cible": campagne.annee_cible }),
                actor_user_id: actor,
            })
            .await?;
        Ok(campagne)
    }

    pub async fn ouvrir_campagne_reinscription_for_actor(
        &self,
        role: &str,
        cmd: OuvrirCampagneReinscription,
    ) -> Result<ReinscriptionCampagne, EntDbError> {
        if !Self::role_can_manage_continuum(role) {
            return Err(EntDbError::Forbidden);
        }
        self.ouvrir_campagne_reinscription(cmd).await
    }

    pub async fn configurer_acompte_campagne(
        &self,
        cmd: ConfigurerAcompteCampagne,
    ) -> Result<ReinscriptionCampagne, EntDbError> {
        let actor = cmd.actor_user_id.clone();
        let existing = self
            .continuum
            .get_campagne(cmd.etablissement_id, &cmd.annee_cible)
            .await?
            .ok_or(DomainError::NotFound)?;
        let updated = plan_configurer_acompte_campagne(&existing, cmd)?;
        self.continuum.upsert_campagne(&updated).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: updated.etablissement_id,
                type_name: types::REINSCRIPTION_ACOMPTE_CONFIG.to_string(),
                aggregate: AggregateKind::Continuum,
                aggregate_id: updated.id,
                eleve_id: None,
                payload: json!({
                    "annee_cible": updated.annee_cible,
                    "acompte_centimes": updated.acompte_centimes,
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(updated)
    }

    pub async fn configurer_acompte_campagne_for_actor(
        &self,
        role: &str,
        cmd: ConfigurerAcompteCampagne,
    ) -> Result<ReinscriptionCampagne, EntDbError> {
        if !Self::role_can_manage_continuum(role) {
            return Err(EntDbError::Forbidden);
        }
        self.configurer_acompte_campagne(cmd).await
    }

    pub async fn encaisse_acompte_reinscription(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<i64, EntDbError> {
        Ok(self
            .facturation
            .sum_encaisse_acompte_reinscription(etablissement_id, foyer_id, annee_scolaire)
            .await?)
    }

    /// Foyer payeur principal de l'élève (premier lien avec foyer_id).
    pub async fn foyer_principal_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<Uuid>, EntDbError> {
        let liens = self.foyers.list_liens_eleve(etablissement_id, eleve_id).await?;
        Ok(liens.into_iter().find_map(|l| l.foyer_id))
    }

    pub async fn enregistrer_scolarite_annee(
        &self,
        cmd: EnregistrerScolariteAnnee,
    ) -> Result<ScolariteAnnee, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id.clone();
        let mut s = plan_enregistrer_scolarite_annee(cmd)?;
        if let Some(existing) = self
            .continuum
            .get_scolarite(s.etablissement_id, s.eleve_id, &s.annee_scolaire)
            .await?
        {
            s.id = existing.id;
        }
        self.continuum.upsert_scolarite(&s).await?;
        // Relire pour id stable si conflit.
        let saved = self
            .continuum
            .get_scolarite(s.etablissement_id, s.eleve_id, &s.annee_scolaire)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::ELEVE_SCOLARITE_ANNEE_ENREGISTREE.to_string(),
                aggregate: AggregateKind::Continuum,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({
                    "annee_scolaire": saved.annee_scolaire,
                    "statut": saved.statut.as_str(),
                    "niveau": saved.niveau,
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(saved)
    }

    /// Synchronise la scolarité « courante » depuis la projection élève (continuité fiche).
    pub async fn sync_scolarite_courante(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
        actor_user_id: Option<String>,
    ) -> Result<ScolariteAnnee, EntDbError> {
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let annee = annee_scolaire_de(jour);
        let niveau = niveau_depuis_classe_ou_mef(eleve.classe.as_deref(), eleve.mef.as_deref());
        self.enregistrer_scolarite_annee(EnregistrerScolariteAnnee {
            etablissement_id,
            eleve_id,
            annee_scolaire: annee,
            niveau,
            mef: eleve.mef.clone(),
            classe: eleve.classe.clone(),
            regime: eleve.regime.clone(),
            statut: ScolariteAnneeStatut::Courante,
            source: ScolariteSource::SyncCourante,
            passage_juridique: true,
            redoublement_possible: false,
            actor_user_id,
        })
        .await
    }

    pub async fn montee_rapide(
        &self,
        cmd: MonteeRapide,
        jour: NaiveDate,
        exiger_campagne: bool,
    ) -> Result<ScolariteAnnee, EntDbError> {
        let annee_n1 = annee_relative(&cmd.annee_courante, 1)?;
        if exiger_campagne {
            let camp = self
                .continuum
                .get_campagne(cmd.etablissement_id, &annee_n1)
                .await?;
            if !camp.map(|c| c.ouverte).unwrap_or(false) {
                return Err(DomainError::CampagneFermee.into());
            }
        }
        let courante = self
            .continuum
            .get_scolarite(cmd.etablissement_id, cmd.eleve_id, &cmd.annee_courante)
            .await?
            .ok_or(DomainError::NotFound)?;
        let opts_n = self
            .options
            .list_for_eleve(cmd.etablissement_id, cmd.eleve_id)
            .await?;
        let tuples: Vec<(OptionNature, String, String)> = opts_n
            .into_iter()
            .map(|o| (o.nature, o.code, o.libelle))
            .collect();
        let actor = cmd.actor_user_id.clone();
        let planned = plan_montee_rapide(&courante, &tuples, cmd, jour)?;
        let mut scolarite = planned.scolarite_n1;
        if let Some(existing) = self
            .continuum
            .get_scolarite(
                scolarite.etablissement_id,
                scolarite.eleve_id,
                &scolarite.annee_scolaire,
            )
            .await?
        {
            scolarite.id = existing.id;
        }
        self.continuum.upsert_scolarite(&scolarite).await?;
        for opt in &planned.options_proposees {
            self.continuum.upsert_option_annee(opt).await?;
        }
        let saved = self
            .continuum
            .get_scolarite(
                scolarite.etablissement_id,
                scolarite.eleve_id,
                &scolarite.annee_scolaire,
            )
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::ELEVE_MONTEE_RAPIDE.to_string(),
                aggregate: AggregateKind::Continuum,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({
                    "annee_scolaire": saved.annee_scolaire,
                    "niveau": saved.niveau,
                    "passage_juridique": saved.passage_juridique,
                    "redoublement_possible": saved.redoublement_possible,
                    "options_proposees": planned.options_proposees.len(),
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(saved)
    }

    pub async fn montee_rapide_for_actor(
        &self,
        role: &str,
        cmd: MonteeRapide,
        jour: NaiveDate,
    ) -> Result<ScolariteAnnee, EntDbError> {
        if !Self::role_can_manage_continuum(role) {
            return Err(EntDbError::Forbidden);
        }
        self.montee_rapide(cmd, jour, true).await
    }

    /// Lot montée N→N+1 : toutes les scolarités courantes du niveau source.
    /// Idempotent : déjà projeté / réinscrit → ignoré (pas de double geste).
    pub async fn montee_lot(
        &self,
        cmd: MonteeLot,
        jour: NaiveDate,
        exiger_campagne: bool,
    ) -> Result<LotMonteeResultat, EntDbError> {
        let niveau_want = normalize_niveau(&cmd.niveau_source);
        if niveau_want.is_empty() {
            return Err(DomainError::NiveauInconnu.into());
        }
        let annee_n1 = annee_relative(&cmd.annee_courante, 1)?;
        if exiger_campagne {
            let camp = self
                .continuum
                .get_campagne(cmd.etablissement_id, &annee_n1)
                .await?;
            if !camp.map(|c| c.ouverte).unwrap_or(false) {
                return Err(DomainError::CampagneFermee.into());
            }
        }

        // Assure une scolarité courante pour chaque élève du niveau (classe / MEF).
        let eleves = self
            .eleves
            .search(cmd.etablissement_id, "", 2000)
            .await?;
        for e in &eleves {
            let niv = niveau_depuis_classe_ou_mef(e.classe.as_deref(), e.mef.as_deref())
                .map(|n| normalize_niveau(&n));
            if niv.as_deref() != Some(niveau_want.as_str()) {
                continue;
            }
            let existing = self
                .continuum
                .get_scolarite(cmd.etablissement_id, e.id, &cmd.annee_courante)
                .await?;
            if existing.is_none() {
                self.sync_scolarite_courante(
                    cmd.etablissement_id,
                    e.id,
                    jour,
                    cmd.actor_user_id.clone(),
                )
                .await?;
            }
        }

        let courantes = self
            .continuum
            .list_scolarites_annee(cmd.etablissement_id, &cmd.annee_courante)
            .await?;
        let (niveau_source, niveau_cible, candidats) =
            candidats_lot_montee(&courantes, &niveau_want)?;

        let actor = cmd.actor_user_id.clone();
        let mut montes = Vec::new();
        let mut ignores = Vec::new();

        for c in candidats {
            if let Some(proj) = self
                .continuum
                .get_scolarite(cmd.etablissement_id, c.eleve_id, &annee_n1)
                .await?
            {
                if matches!(
                    proj.statut,
                    ScolariteAnneeStatut::Projetee | ScolariteAnneeStatut::Reinscription
                ) {
                    ignores.push(LotMonteeIgnore {
                        eleve_id: c.eleve_id,
                        motif: "deja_projetee".into(),
                    });
                    continue;
                }
            }
            match self
                .montee_rapide(
                    MonteeRapide {
                        etablissement_id: cmd.etablissement_id,
                        eleve_id: c.eleve_id,
                        annee_courante: cmd.annee_courante.clone(),
                        actor_user_id: actor.clone(),
                    },
                    jour,
                    false, // campagne déjà vérifiée pour le lot
                )
                .await
            {
                Ok(s) => montes.push(s.eleve_id),
                Err(EntDbError::Domain(DomainError::NiveauSansSuite)) => {
                    ignores.push(LotMonteeIgnore {
                        eleve_id: c.eleve_id,
                        motif: "niveau_sans_suite".into(),
                    });
                }
                Err(EntDbError::Domain(DomainError::NiveauInconnu)) => {
                    ignores.push(LotMonteeIgnore {
                        eleve_id: c.eleve_id,
                        motif: "niveau_inconnu".into(),
                    });
                }
                Err(e) => {
                    ignores.push(LotMonteeIgnore {
                        eleve_id: c.eleve_id,
                        motif: format!("erreur:{e}"),
                    });
                }
            }
        }

        let resultat = LotMonteeResultat {
            niveau_source,
            niveau_cible,
            annee_courante: cmd.annee_courante.clone(),
            annee_cible: annee_n1,
            montes,
            ignores,
        };

        self.events
            .publish(MetierEventDraft {
                etablissement_id: cmd.etablissement_id,
                type_name: types::ELEVE_MONTEE_LOT.to_string(),
                aggregate: AggregateKind::Continuum,
                aggregate_id: cmd.etablissement_id,
                eleve_id: None,
                payload: json!({
                    "niveau_source": resultat.niveau_source,
                    "niveau_cible": resultat.niveau_cible,
                    "annee_courante": resultat.annee_courante,
                    "annee_cible": resultat.annee_cible,
                    "montes": resultat.montes.len(),
                    "ignores": resultat.ignores.len(),
                }),
                actor_user_id: actor,
            })
            .await?;

        Ok(resultat)
    }

    pub async fn montee_lot_for_actor(
        &self,
        role: &str,
        cmd: MonteeLot,
        jour: NaiveDate,
    ) -> Result<LotMonteeResultat, EntDbError> {
        if !Self::role_can_manage_continuum(role) {
            return Err(EntDbError::Forbidden);
        }
        self.montee_lot(cmd, jour, true).await
    }

    /// Aperçu des candidats d'un lot (sans écrire).
    pub async fn candidats_montee_lot(
        &self,
        etablissement_id: Uuid,
        annee_courante: &str,
        niveau_source: &str,
        jour: NaiveDate,
    ) -> Result<(String, String, Vec<ScolariteAnnee>), EntDbError> {
        let niveau_want = normalize_niveau(niveau_source);
        if niveau_want.is_empty() {
            return Err(DomainError::NiveauInconnu.into());
        }
        let eleves = self.eleves.search(etablissement_id, "", 2000).await?;
        for e in &eleves {
            let niv = niveau_depuis_classe_ou_mef(e.classe.as_deref(), e.mef.as_deref())
                .map(|n| normalize_niveau(&n));
            if niv.as_deref() != Some(niveau_want.as_str()) {
                continue;
            }
            if self
                .continuum
                .get_scolarite(etablissement_id, e.id, annee_courante)
                .await?
                .is_none()
            {
                self.sync_scolarite_courante(etablissement_id, e.id, jour, None)
                    .await?;
            }
        }
        let courantes = self
            .continuum
            .list_scolarites_annee(etablissement_id, annee_courante)
            .await?;
        let (src, cible, refs) = candidats_lot_montee(&courantes, &niveau_want)?;
        Ok((src, cible, refs.into_iter().cloned().collect()))
    }

    pub async fn choisir_option_n1(
        &self,
        cmd: ChoisirOptionN1,
        exiger_campagne: bool,
    ) -> Result<OptionAnnee, EntDbError> {
        if exiger_campagne {
            let camp = self
                .continuum
                .get_campagne(cmd.etablissement_id, &cmd.annee_scolaire)
                .await?;
            if !camp.map(|c| c.ouverte).unwrap_or(false) {
                return Err(DomainError::CampagneFermee.into());
            }
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id.clone();
        let opt = plan_choisir_option_n1(cmd)?;
        self.continuum.upsert_option_annee(&opt).await?;
        let saved = self
            .continuum
            .list_options_annee(opt.etablissement_id, opt.eleve_id, &opt.annee_scolaire)
            .await?
            .into_iter()
            .find(|o| o.code == opt.code && o.nature == opt.nature)
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::ELEVE_OPTION_N1_CHOISIE.to_string(),
                aggregate: AggregateKind::Continuum,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({
                    "annee_scolaire": saved.annee_scolaire,
                    "code": saved.code,
                    "statut": saved.statut.as_str(),
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(saved)
    }

    /// Parent lié : choisir / retenir une option académique N+1 si campagne ouverte.
    pub async fn choisir_option_n1_for_parent(
        &self,
        etablissement_id: Uuid,
        parent_user_id: Uuid,
        cmd: ChoisirOptionN1,
    ) -> Result<OptionAnnee, EntDbError> {
        if cmd.etablissement_id != etablissement_id {
            return Err(DomainError::TenantMismatch.into());
        }
        let lien = self
            .lien_parent_eleve(etablissement_id, parent_user_id, cmd.eleve_id)
            .await?;
        if lien.is_none() {
            return Err(EntDbError::Forbidden);
        }
        self.choisir_option_n1(cmd, true).await
    }

    pub async fn get_campagne_reinscription(
        &self,
        etablissement_id: Uuid,
        annee_cible: &str,
    ) -> Result<Option<ReinscriptionCampagne>, EntDbError> {
        Ok(self
            .continuum
            .get_campagne(etablissement_id, annee_cible)
            .await?)
    }

    pub async fn options_continuum_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Vec<OptionAnnee>, EntDbError> {
        Ok(self
            .continuum
            .list_options_annee(etablissement_id, eleve_id, annee_scolaire)
            .await?)
    }

    pub async fn scolarite_eleve_annee(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Option<ScolariteAnnee>, EntDbError> {
        Ok(self
            .continuum
            .get_scolarite(etablissement_id, eleve_id, annee_scolaire)
            .await?)
    }

    pub async fn poser_exigence_doc(
        &self,
        cmd: PoserExigenceDoc,
    ) -> Result<ExigenceDocReinscription, EntDbError> {
        let actor = cmd.actor_user_id.clone();
        let ex = plan_poser_exigence_doc(cmd)?;
        self.continuum.upsert_exigence_doc(&ex).await?;
        let saved = self
            .continuum
            .list_exigences_doc(ex.etablissement_id, &ex.annee_cible)
            .await?
            .into_iter()
            .find(|e| e.code == ex.code)
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::REINSCRIPTION_EXIGENCE_DOC.to_string(),
                aggregate: AggregateKind::Continuum,
                aggregate_id: saved.id,
                eleve_id: None,
                payload: json!({
                    "annee_cible": saved.annee_cible,
                    "code": saved.code,
                    "obligatoire": saved.obligatoire,
                }),
                actor_user_id: actor,
            })
            .await?;
        Ok(saved)
    }

    pub async fn poser_exigence_doc_for_actor(
        &self,
        role: &str,
        cmd: PoserExigenceDoc,
    ) -> Result<ExigenceDocReinscription, EntDbError> {
        if !Self::role_can_manage_continuum(role) {
            return Err(EntDbError::Forbidden);
        }
        self.poser_exigence_doc(cmd).await
    }

    pub async fn list_exigences_doc(
        &self,
        etablissement_id: Uuid,
        annee_cible: &str,
    ) -> Result<Vec<ExigenceDocReinscription>, EntDbError> {
        Ok(self
            .continuum
            .list_exigences_doc(etablissement_id, annee_cible)
            .await?)
    }

    /// Parent dépose un document famille pour satisfaire une vanne réinscription.
    pub async fn deposer_doc_reinscription_for_parent(
        &self,
        etablissement_id: Uuid,
        parent_user_id: Uuid,
        eleve_id: Uuid,
        annee_cible: &str,
        code: &str,
        titre: &str,
    ) -> Result<EleveDocument, EntDbError> {
        let lien = self
            .lien_parent_eleve(etablissement_id, parent_user_id, eleve_id)
            .await?;
        if lien.is_none() {
            return Err(EntDbError::Forbidden);
        }
        let camp = self
            .continuum
            .get_campagne(etablissement_id, annee_cible)
            .await?;
        if !camp.map(|c| c.ouverte).unwrap_or(false) {
            return Err(DomainError::CampagneFermee.into());
        }
        let code_norm = code.trim().to_ascii_uppercase();
        let exigences = self
            .continuum
            .list_exigences_doc(etablissement_id, annee_cible)
            .await?;
        let exigence = exigences
            .iter()
            .find(|e| e.code == code_norm)
            .ok_or(DomainError::ExigenceDocInvalide)?;
        self.deposer_document(DeposerDocument {
            etablissement_id,
            eleve_id,
            tiroir: exigence.tiroir,
            titre: if titre.trim().is_empty() {
                exigence.libelle.clone()
            } else {
                titre.to_string()
            },
            code: Some(code_norm),
            mime_type: Some("application/pdf".into()),
            storage_key: Some(format!(
                "reinscription/{etablissement_id}/{eleve_id}/{}.pdf",
                exigence.code
            )),
            confidentialite: DocConfidentialite::Standard,
            actor_user_id: parent_user_id,
        })
        .await
    }

    /// Parent confirme la réinscription N+1 (statut → reinscription).
    pub async fn confirmer_reinscription_for_parent(
        &self,
        etablissement_id: Uuid,
        parent_user_id: Uuid,
        cmd: ConfirmerReinscription,
    ) -> Result<ScolariteAnnee, EntDbError> {
        if cmd.etablissement_id != etablissement_id {
            return Err(DomainError::TenantMismatch.into());
        }
        let lien = self
            .lien_parent_eleve(etablissement_id, parent_user_id, cmd.eleve_id)
            .await?;
        if lien.is_none() {
            return Err(EntDbError::Forbidden);
        }
        let camp = self
            .continuum
            .get_campagne(etablissement_id, &cmd.annee_scolaire)
            .await?
            .ok_or(DomainError::CampagneFermee)?;
        if !camp.ouverte {
            return Err(DomainError::CampagneFermee.into());
        }
        let projection = self
            .continuum
            .get_scolarite(etablissement_id, cmd.eleve_id, &cmd.annee_scolaire)
            .await?
            .ok_or(DomainError::ReinscriptionNonPrete)?;
        let exigences = self
            .continuum
            .list_exigences_doc(etablissement_id, &cmd.annee_scolaire)
            .await?;
        let docs = self
            .documents
            .list_documents_eleve(etablissement_id, cmd.eleve_id)
            .await?;
        assert_vannes_documents(&exigences, &docs)?;
        let foyer_id = self
            .foyer_principal_eleve(etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let encaisse_acompte = self
            .encaisse_acompte_reinscription(etablissement_id, foyer_id, &cmd.annee_scolaire)
            .await?;
        assert_vanne_acompte(camp.acompte_centimes, encaisse_acompte)?;
        let actor = cmd.actor_user_id.clone();
        let updated = plan_confirmer_reinscription_parent(&projection, cmd)?;
        self.continuum.upsert_scolarite(&updated).await?;
        let saved = self
            .continuum
            .get_scolarite(etablissement_id, updated.eleve_id, &updated.annee_scolaire)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::ELEVE_REINSCRIPTION_CONFIRMEE.to_string(),
                aggregate: AggregateKind::Continuum,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({
                    "annee_scolaire": saved.annee_scolaire,
                    "niveau": saved.niveau,
                    "statut": saved.statut.as_str(),
                }),
                actor_user_id: actor.clone(),
            })
            .await?;

        // Accusé SMTP famille (confirmation reçue).
        let eleve = self
            .eleves
            .get(etablissement_id, saved.eleve_id)
            .await?;
        let (prenom, nom) = eleve
            .as_ref()
            .map(|e| (e.prenom.as_str(), e.nom.as_str()))
            .unwrap_or(("l'élève", ""));
        let niveau = saved.niveau.as_deref().unwrap_or("niveau à confirmer");
        let sujet = format!(
            "Réinscription confirmée — {prenom} {nom} ({})",
            saved.annee_scolaire
        );
        let corps = format!(
            "Bonjour,\n\n\
             Nous avons bien enregistré la confirmation de réinscription de {prenom} {nom} \
             pour l'année scolaire {} (projection {niveau}).\n\n\
             Ce message confirme la réception des documents et de l'acompte requis. \
             Le passage administratif définitif sera traité par l'établissement.\n\n\
             Cordialement,\nL'établissement",
            saved.annee_scolaire
        );
        let actor_uuid = actor
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok());
        let _ = self
            .notify_parents_mail(
                etablissement_id,
                saved.eleve_id,
                "reinscription_confirmee",
                &sujet,
                &corps,
                "continuum",
                saved.id,
                actor_uuid,
                vec![],
            )
            .await;
        // Accusé push (inclut le parent qui confirme — pas de skip actor).
        let _ = self
            .notify_parents_push(
                etablissement_id,
                saved.eleve_id,
                "reinscription_confirmee",
                &sujet,
                "Réinscription bien enregistrée — consultez le portail famille.",
                "continuum",
                saved.id,
                None,
            )
            .await;

        Ok(saved)
    }

    /// État des vannes docs pour un élève / année cible.
    pub async fn vannes_docs_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annee_cible: &str,
    ) -> Result<(Vec<ExigenceDocReinscription>, Vec<String>), EntDbError> {
        let exigences = self
            .continuum
            .list_exigences_doc(etablissement_id, annee_cible)
            .await?;
        let docs = self
            .documents
            .list_documents_eleve(etablissement_id, eleve_id)
            .await?;
        let manquants = codes_documents_manquants(&exigences, &docs);
        Ok((exigences, manquants))
    }

    pub fn role_can_manage_vs_referentiels(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_manage_vs_referentiels())
            .unwrap_or(false)
    }

    pub fn role_can_saisir_sanctions(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_saisir_sanctions())
            .unwrap_or(false)
    }

    pub fn role_can_declarer_incident(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_declarer_incident())
            .unwrap_or(false)
    }

    pub async fn creer_incident_type(
        &self,
        cmd: CreerReferentielVs,
    ) -> Result<IncidentType, EntDbError> {
        let actor = cmd.actor_user_id.clone();
        let t = plan_creer_incident_type(cmd)?;
        self.sanctions.upsert_incident_type(&t).await?;
        let saved = self
            .sanctions
            .get_incident_type_by_code(t.etablissement_id, &t.code)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::VS_INCIDENT_TYPE_CREATED.to_string(),
                aggregate: AggregateKind::Incident,
                aggregate_id: saved.id,
                eleve_id: None,
                payload: json!({ "code": saved.code }),
                actor_user_id: actor,
            })
            .await?;
        Ok(saved)
    }

    pub async fn creer_incident_type_for_actor(
        &self,
        role: &str,
        cmd: CreerReferentielVs,
    ) -> Result<IncidentType, EntDbError> {
        if !Self::role_can_manage_vs_referentiels(role) {
            return Err(EntDbError::Forbidden);
        }
        self.creer_incident_type(cmd).await
    }

    pub async fn creer_sanction_type(
        &self,
        cmd: CreerReferentielVs,
    ) -> Result<SanctionType, EntDbError> {
        let actor = cmd.actor_user_id.clone();
        let t = plan_creer_sanction_type(cmd)?;
        self.sanctions.upsert_sanction_type(&t).await?;
        let saved = self
            .sanctions
            .get_sanction_type_by_code(t.etablissement_id, &t.code)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::VS_SANCTION_TYPE_CREATED.to_string(),
                aggregate: AggregateKind::Sanction,
                aggregate_id: saved.id,
                eleve_id: None,
                payload: json!({ "code": saved.code }),
                actor_user_id: actor,
            })
            .await?;
        Ok(saved)
    }

    pub async fn creer_sanction_type_for_actor(
        &self,
        role: &str,
        cmd: CreerReferentielVs,
    ) -> Result<SanctionType, EntDbError> {
        if !Self::role_can_manage_vs_referentiels(role) {
            return Err(EntDbError::Forbidden);
        }
        self.creer_sanction_type(cmd).await
    }

    pub async fn creer_sanction_motif(
        &self,
        cmd: CreerReferentielVs,
    ) -> Result<SanctionMotif, EntDbError> {
        let actor = cmd.actor_user_id.clone();
        let m = plan_creer_sanction_motif(cmd)?;
        self.sanctions.upsert_sanction_motif(&m).await?;
        let saved = self
            .sanctions
            .get_sanction_motif_by_code(m.etablissement_id, &m.code)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::VS_SANCTION_MOTIF_CREATED.to_string(),
                aggregate: AggregateKind::Sanction,
                aggregate_id: saved.id,
                eleve_id: None,
                payload: json!({ "code": saved.code }),
                actor_user_id: actor,
            })
            .await?;
        Ok(saved)
    }

    pub async fn creer_sanction_motif_for_actor(
        &self,
        role: &str,
        cmd: CreerReferentielVs,
    ) -> Result<SanctionMotif, EntDbError> {
        if !Self::role_can_manage_vs_referentiels(role) {
            return Err(EntDbError::Forbidden);
        }
        self.creer_sanction_motif(cmd).await
    }

    pub async fn list_sanction_types(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<SanctionType>, EntDbError> {
        Ok(self.sanctions.list_sanction_types(etablissement_id).await?)
    }

    pub async fn list_incident_types(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<IncidentType>, EntDbError> {
        Ok(self.sanctions.list_incident_types(etablissement_id).await?)
    }

    pub async fn list_sanction_motifs(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<SanctionMotif>, EntDbError> {
        Ok(self.sanctions.list_sanction_motifs(etablissement_id).await?)
    }

    pub async fn declarer_incident(&self, cmd: DeclarerIncident) -> Result<Incident, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let typ = self
            .sanctions
            .get_incident_type(cmd.etablissement_id, cmd.type_id)
            .await?
            .ok_or(DomainError::ReferentielVsInconnu)?;
        if !typ.actif {
            return Err(DomainError::ReferentielVsInconnu.into());
        }
        let actor = cmd.actor_user_id;
        let incident = plan_declarer_incident(cmd)?;
        self.sanctions.upsert_incident(&incident).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: incident.etablissement_id,
                type_name: types::ELEVE_INCIDENT_DECLARED.to_string(),
                aggregate: AggregateKind::Incident,
                aggregate_id: incident.id,
                eleve_id: Some(incident.eleve_id),
                payload: json!({
                    "type_id": incident.type_id.to_string(),
                    "jour": incident.jour.to_string(),
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(incident)
    }

    pub async fn declarer_incident_for_actor(
        &self,
        role: &str,
        cmd: DeclarerIncident,
    ) -> Result<Incident, EntDbError> {
        if !Self::role_can_declarer_incident(role) {
            return Err(EntDbError::Forbidden);
        }
        self.declarer_incident(cmd).await
    }

    pub async fn prononcer_sanction(
        &self,
        cmd: PrononcerSanction,
    ) -> Result<Sanction, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let typ = self
            .sanctions
            .get_sanction_type(cmd.etablissement_id, cmd.type_id)
            .await?
            .ok_or(DomainError::ReferentielVsInconnu)?;
        if !typ.actif {
            return Err(DomainError::ReferentielVsInconnu.into());
        }
        if let Some(mid) = cmd.motif_id {
            let motif = self
                .sanctions
                .get_sanction_motif(cmd.etablissement_id, mid)
                .await?
                .ok_or(DomainError::ReferentielVsInconnu)?;
            if !motif.actif {
                return Err(DomainError::ReferentielVsInconnu.into());
            }
        }
        if let Some(iid) = cmd.incident_id {
            let inc = self
                .sanctions
                .get_incident(cmd.etablissement_id, iid)
                .await?
                .ok_or(DomainError::NotFound)?;
            if inc.eleve_id != cmd.eleve_id {
                return Err(DomainError::NotFound.into());
            }
        }
        let actor = cmd.actor_user_id;
        let type_libelle = typ.libelle.clone();
        let sanction = plan_prononcer_sanction(cmd)?;
        self.sanctions.upsert_sanction(&sanction).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: sanction.etablissement_id,
                type_name: types::ELEVE_SANCTION_PRONONCEE.to_string(),
                aggregate: AggregateKind::Sanction,
                aggregate_id: sanction.id,
                eleve_id: Some(sanction.eleve_id),
                payload: json!({
                    "type_id": sanction.type_id.to_string(),
                    "duree_minutes": sanction.duree_minutes,
                    "statut": sanction.statut.as_str(),
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;

        // SMS auto collège → famille (dry-run lab / live si ENT_SMS_LIVE).
        let eleve = self
            .eleves
            .get(sanction.etablissement_id, sanction.eleve_id)
            .await?;
        let (prenom, nom) = eleve
            .as_ref()
            .map(|e| (e.prenom.as_str(), e.nom.as_str()))
            .unwrap_or(("l'élève", ""));
        let duree = sanction
            .duree_minutes
            .map(|m| format!(" ({m} min)"))
            .unwrap_or_default();
        let sms_body = format!(
            "Vie scolaire: sanction « {type_libelle} »{duree} pour {prenom} {nom} le {}. Consultez le portail famille.",
            sanction.jour
        );
        let _ = self
            .notify_parents_sms(
                sanction.etablissement_id,
                sanction.eleve_id,
                "sanction",
                &sms_body,
                "sanction",
                sanction.id,
                Some(actor),
            )
            .await;

        Ok(sanction)
    }

    pub async fn prononcer_sanction_for_actor(
        &self,
        role: &str,
        cmd: PrononcerSanction,
    ) -> Result<Sanction, EntDbError> {
        if !Self::role_can_saisir_sanctions(role) {
            return Err(EntDbError::Forbidden);
        }
        self.prononcer_sanction(cmd).await
    }

    /// Liste les sanctions d’un élève si l’acteur a la section VS du dossier.
    pub async fn list_sanctions_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<Sanction>, EntDbError> {
        let sections = self
            .dossier_sections_for_actor(etablissement_id, actor_user_id, role, eleve_id)
            .await?;
        if !sections.vs {
            return Ok(Vec::new());
        }
        Ok(self
            .sanctions
            .list_sanctions_eleve(etablissement_id, eleve_id)
            .await?)
    }

    pub async fn list_incidents_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<Incident>, EntDbError> {
        let sections = self
            .dossier_sections_for_actor(etablissement_id, actor_user_id, role, eleve_id)
            .await?;
        if !sections.vs {
            return Ok(Vec::new());
        }
        Ok(self
            .sanctions
            .list_incidents_eleve(etablissement_id, eleve_id)
            .await?)
    }

    pub async fn creer_commission_for_actor(
        &self,
        role: &str,
        cmd: CreerCommissionVs,
    ) -> Result<CommissionVs, EntDbError> {
        if !Self::role_can_saisir_sanctions(role) {
            return Err(EntDbError::Forbidden);
        }
        let actor = cmd.actor_user_id;
        let c = plan_creer_commission(cmd)?;
        self.sanctions.insert_commission(&c).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: c.etablissement_id,
                type_name: types::VS_COMMISSION.to_string(),
                aggregate: AggregateKind::Commission,
                aggregate_id: c.id,
                eleve_id: None,
                payload: json!({
                    "jour": c.jour.to_string(),
                    "objet": c.objet,
                    "statut": c.statut.as_str(),
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(c)
    }

    pub async fn convoquer_commission_for_actor(
        &self,
        role: &str,
        cmd: ConvoquerCommission,
    ) -> Result<CommissionDossier, EntDbError> {
        if !Self::role_can_saisir_sanctions(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let commission = self
            .sanctions
            .get_commission(cmd.etablissement_id, cmd.commission_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let dossier = plan_convoquer_commission(&commission, cmd)?;
        self.sanctions.insert_commission_dossier(&dossier).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: dossier.etablissement_id,
                type_name: types::VS_COMMISSION_CONVOCATION.to_string(),
                aggregate: AggregateKind::Commission,
                aggregate_id: dossier.id,
                eleve_id: Some(dossier.eleve_id),
                payload: json!({
                    "commission_id": dossier.commission_id,
                    "signale_famille": dossier.signale_famille,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(dossier)
    }

    pub async fn decider_commission_dossier_for_actor(
        &self,
        role: &str,
        cmd: DeciderCommissionDossier,
    ) -> Result<CommissionDossier, EntDbError> {
        if !Self::role_can_saisir_sanctions(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .sanctions
            .get_commission_dossier(cmd.etablissement_id, cmd.dossier_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let commission = self
            .sanctions
            .get_commission(cmd.etablissement_id, current.commission_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let (dossier, tenue) = plan_decider_dossier(&current, &commission, &cmd)?;
        self.sanctions.update_commission_dossier(&dossier).await?;
        self.sanctions.update_commission_statut(&tenue).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: dossier.etablissement_id,
                type_name: types::VS_COMMISSION_DECISION.to_string(),
                aggregate: AggregateKind::Commission,
                aggregate_id: dossier.id,
                eleve_id: Some(dossier.eleve_id),
                payload: json!({
                    "decision": cmd.decision.as_str(),
                    "points_retires": dossier.points_retires,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(dossier)
    }

    pub async fn list_commissions_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<CommissionVs>, EntDbError> {
        if !Self::role_can_saisir_sanctions(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .sanctions
            .list_commissions_jour(etablissement_id, jour, 50)
            .await?)
    }

    pub async fn list_dossiers_commission_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        commission_id: Uuid,
    ) -> Result<Vec<CommissionDossier>, EntDbError> {
        if !Self::role_can_saisir_sanctions(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .sanctions
            .list_dossiers_commission(etablissement_id, commission_id)
            .await?)
    }

    pub async fn list_commissions_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<CommissionDossier>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_saisir_sanctions();
        if !staff {
            if parsed != Role::Parent {
                return Err(EntDbError::Forbidden);
            }
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            if lien.is_none() {
                return Err(EntDbError::Forbidden);
            }
        }
        let mut list = self
            .sanctions
            .list_dossiers_eleve(etablissement_id, eleve_id, 50)
            .await?;
        if !staff {
            list = list
                .into_iter()
                .filter(|d| d.signale_famille)
                .map(|d| vue_commission_dossier_famille(&d))
                .collect();
        }
        Ok(list)
    }

    pub async fn solde_points_vs_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<i32, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_saisir_sanctions();
        if !staff {
            if parsed != Role::Parent {
                return Err(EntDbError::Forbidden);
            }
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            if lien.is_none() {
                return Err(EntDbError::Forbidden);
            }
        }
        let sum = self
            .sanctions
            .sum_points_retires_eleve(etablissement_id, eleve_id)
            .await?;
        Ok(solde_points_vs(sum))
    }

    pub async fn annuler_sanction(&self, cmd: AnnulerSanction) -> Result<Sanction, EntDbError> {
        let current = self
            .sanctions
            .get_sanction(cmd.etablissement_id, cmd.sanction_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let cancelled = plan_annuler_sanction(&current, cmd)?;
        self.sanctions
            .mark_sanction_annulee(cancelled.etablissement_id, cancelled.id, actor)
            .await?;
        let saved = self
            .sanctions
            .get_sanction(cancelled.etablissement_id, cancelled.id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::ELEVE_SANCTION_ANNULEE.to_string(),
                aggregate: AggregateKind::Sanction,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({ "statut": saved.statut.as_str() }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn annuler_sanction_for_actor(
        &self,
        role: &str,
        cmd: AnnulerSanction,
    ) -> Result<Sanction, EntDbError> {
        if !Self::role_can_saisir_sanctions(role) {
            return Err(EntDbError::Forbidden);
        }
        self.annuler_sanction(cmd).await
    }

    pub fn role_can_gerer_sorties(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_gerer_sorties())
            .unwrap_or(false)
    }

    pub fn role_can_controle_sortie(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_controle_sortie())
            .unwrap_or(false)
    }

    /// Consentement permanent parent, ou forbid établissement (staff).
    pub async fn upsert_autorisation_sortie_for_actor(
        &self,
        role: &str,
        cmd: UpsertAutorisationSortie,
    ) -> Result<AutorisationSortie, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let is_parent = matches!(parsed, Role::Parent);
        let is_staff = Self::role_can_gerer_sorties(role);
        if !is_parent && !is_staff {
            return Err(EntDbError::Forbidden);
        }
        if is_parent {
            if cmd.interdit_etablissement {
                return Err(EntDbError::Forbidden);
            }
            if self
                .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
                .await?
                .is_none()
            {
                return Err(EntDbError::Forbidden);
            }
            // Parent ne peut pas lever un forbid déjà posé.
            if let Some(cur) = self
                .sorties
                .get_autorisation(cmd.etablissement_id, cmd.eleve_id)
                .await?
            {
                if cur.interdit_etablissement {
                    return Err(EntDbError::Forbidden);
                }
            }
        }
        let mut planned = plan_upsert_autorisation_sortie(cmd)?;
        if is_parent && !planned.interdit_etablissement {
            let eleve = self
                .eleves
                .get(planned.etablissement_id, planned.eleve_id)
                .await?
                .ok_or(DomainError::NotFound)?;
            if let Some(niveau) =
                niveau_depuis_classe_ou_mef(eleve.classe.as_deref(), eleve.mef.as_deref())
            {
                let regle = self
                    .sorties
                    .get_regle_niveau(planned.etablissement_id, &normalize_niveau(&niveau))
                    .await?;
                planned = appliquer_regle_niveau_parent(planned, regle.as_ref())?;
            }
            if let Some(reg) = eleve.regime.as_deref() {
                let code = normalize_sortie_regime(reg);
                let regle = self
                    .sorties
                    .get_regle_regime(planned.etablissement_id, &code)
                    .await?;
                planned = appliquer_regle_regime_parent(planned, regle.as_ref())?;
            }
        }
        self.sorties.upsert_autorisation(&planned).await?;
        let saved = self
            .sorties
            .get_autorisation(planned.etablissement_id, planned.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::SORTIE_AUTORISATION.to_string(),
                aggregate: AggregateKind::Sortie,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({
                    "matin": saved.matin,
                    "midi": saved.midi,
                    "soir": saved.soir,
                    "interdit_etablissement": saved.interdit_etablissement,
                }),
                actor_user_id: Some(saved.updated_by_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn upsert_sortie_regle_niveau_for_actor(
        &self,
        role: &str,
        cmd: UpsertSortieRegleNiveau,
    ) -> Result<SortieRegleNiveau, EntDbError> {
        if !Self::role_can_gerer_sorties(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_upsert_sortie_regle_niveau(cmd)?;
        self.sorties.upsert_regle_niveau(&planned).await?;
        let saved = self
            .sorties
            .get_regle_niveau(planned.etablissement_id, &planned.niveau)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::SORTIE_AUTORISATION.to_string(),
                aggregate: AggregateKind::Sortie,
                aggregate_id: saved.id,
                eleve_id: None,
                payload: json!({
                    "regle_niveau": saved.niveau,
                    "choix_parent_interdit": saved.choix_parent_interdit,
                    "matin": saved.matin_autorise,
                    "midi": saved.midi_autorise,
                    "soir": saved.soir_autorise,
                }),
                actor_user_id: Some(saved.updated_by_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn list_sortie_regles_niveau_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<SortieRegleNiveau>, EntDbError> {
        if !Self::role_can_gerer_sorties(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.sorties.list_regles_niveau(etablissement_id).await?)
    }

    pub async fn upsert_sortie_regle_regime_for_actor(
        &self,
        role: &str,
        cmd: UpsertSortieRegleRegime,
    ) -> Result<SortieRegleRegime, EntDbError> {
        if !Self::role_can_gerer_sorties(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_upsert_sortie_regle_regime(cmd)?;
        self.sorties.upsert_regle_regime(&planned).await?;
        let saved = self
            .sorties
            .get_regle_regime(planned.etablissement_id, planned.regime.as_str())
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::SORTIE_AUTORISATION.to_string(),
                aggregate: AggregateKind::Sortie,
                aggregate_id: saved.id,
                eleve_id: None,
                payload: json!({
                    "regle_regime": saved.regime.as_str(),
                    "choix_parent_interdit": saved.choix_parent_interdit,
                    "matin": saved.matin_autorise,
                    "midi": saved.midi_autorise,
                    "soir": saved.soir_autorise,
                }),
                actor_user_id: Some(saved.updated_by_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn list_sortie_regles_regime_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<SortieRegleRegime>, EntDbError> {
        if !Self::role_can_gerer_sorties(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.sorties.list_regles_regime(etablissement_id).await?)
    }

    pub async fn get_recap_sortie_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<RecapSortieEleve, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        match parsed {
            Role::Parent => {
                if self
                    .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                    .await?
                    .is_none()
                {
                    return Err(EntDbError::Forbidden);
                }
            }
            Role::Eleve => {
                let linked = self
                    .eleve_comptes
                    .eleve_id_for_user(etablissement_id, actor_user_id)
                    .await?;
                if linked != Some(eleve_id) {
                    return Err(EntDbError::Forbidden);
                }
            }
            r if r.can_gerer_sorties() || r.can_read_all_eleves() => {}
            _ => return Err(EntDbError::Forbidden),
        }
        let autorisation = self
            .sorties
            .get_autorisation(etablissement_id, eleve_id)
            .await?;
        let demandes_ouvertes = self
            .sorties
            .list_demandes_eleve(etablissement_id, eleve_id, true)
            .await?;
        Ok(RecapSortieEleve {
            eleve_id,
            autorisation,
            demandes_ouvertes,
        })
    }

    pub async fn demander_sortie_exceptionnelle_for_actor(
        &self,
        role: &str,
        cmd: DemanderSortieExceptionnelle,
    ) -> Result<DemandeSortieExceptionnelle, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        match parsed {
            Role::Parent => {
                if self
                    .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
                    .await?
                    .is_none()
                {
                    return Err(EntDbError::Forbidden);
                }
            }
            Role::Eleve => {
                let linked = self
                    .eleve_comptes
                    .eleve_id_for_user(cmd.etablissement_id, cmd.actor_user_id)
                    .await?;
                if linked != Some(cmd.eleve_id) {
                    return Err(EntDbError::Forbidden);
                }
            }
            r if r.can_gerer_sorties() => {}
            _ => return Err(EntDbError::Forbidden),
        }
        if let Some(auth) = self
            .sorties
            .get_autorisation(cmd.etablissement_id, cmd.eleve_id)
            .await?
        {
            if auth.interdit_etablissement {
                return Err(EntDbError::Domain(DomainError::SortieDemandeInvalide));
            }
        }
        let dem = plan_demander_sortie_exceptionnelle(cmd)?;
        self.sorties.insert_demande(&dem).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: dem.etablissement_id,
                type_name: types::SORTIE_DEMANDE.to_string(),
                aggregate: AggregateKind::Sortie,
                aggregate_id: dem.id,
                eleve_id: Some(dem.eleve_id),
                payload: json!({
                    "jour": dem.jour.to_string(),
                    "creneau": dem.creneau.as_str(),
                    "motif": dem.motif,
                    "statut": dem.statut.as_str(),
                }),
                actor_user_id: Some(dem.demande_par_user_id.to_string()),
            })
            .await?;
        Ok(dem)
    }

    pub async fn decider_demande_sortie_for_actor(
        &self,
        role: &str,
        cmd: DeciderDemandeSortie,
    ) -> Result<DemandeSortieExceptionnelle, EntDbError> {
        if !Self::role_can_gerer_sorties(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .sorties
            .get_demande(cmd.etablissement_id, cmd.demande_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let decided = plan_decider_demande_sortie(&current, cmd)?;
        self.sorties.update_demande(&decided).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: decided.etablissement_id,
                type_name: types::SORTIE_DECISION.to_string(),
                aggregate: AggregateKind::Sortie,
                aggregate_id: decided.id,
                eleve_id: Some(decided.eleve_id),
                payload: json!({
                    "statut": decided.statut.as_str(),
                    "creneau": decided.creneau.as_str(),
                    "jour": decided.jour.to_string(),
                }),
                actor_user_id: decided.decided_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        Ok(decided)
    }

    pub async fn list_demandes_sortie_en_attente_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<DemandeSortieExceptionnelle>, EntDbError> {
        if !Self::role_can_gerer_sorties(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .sorties
            .list_demandes_en_attente(etablissement_id)
            .await?)
    }

    pub async fn upsert_sortie_param_for_actor(
        &self,
        role: &str,
        cmd: UpsertSortieParamEtab,
    ) -> Result<SortieParamEtab, EntDbError> {
        if !Self::role_can_gerer_sorties(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_upsert_sortie_param(cmd)?;
        self.sorties.upsert_param(&planned).await?;
        Ok(self
            .sorties
            .get_param(planned.etablissement_id)
            .await?
            .ok_or(DomainError::NotFound)?)
    }

    pub async fn get_sortie_param_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Option<SortieParamEtab>, EntDbError> {
        if !Self::role_can_gerer_sorties(role) && !Self::role_can_controle_sortie(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.sorties.get_param(etablissement_id).await?)
    }

    /// Contrôle badge portail : évalue + journalise (permanent / exception / autre site EDT).
    pub async fn controle_sortie_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
        creneau: SortieCreneau,
        site_portail: Option<String>,
        maintenant: Option<NaiveTime>,
    ) -> Result<ControleSortie, EntDbError> {
        if !Self::role_can_controle_sortie(role) {
            return Err(EntDbError::Forbidden);
        }
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let auth = self
            .sorties
            .get_autorisation(etablissement_id, eleve_id)
            .await?;
        let demandes = self
            .sorties
            .list_demandes_jour(etablissement_id, eleve_id, jour)
            .await?;
        let param = self.sorties.get_param(etablissement_id).await?;
        let mut ctx = ControleSortieContexteSite {
            autoriser_changement_site: param
                .as_ref()
                .map(|p| p.autoriser_changement_site)
                .unwrap_or(false),
            site_portail: site_portail.filter(|s| !s.trim().is_empty()),
            maintenant,
            creneaux_apres: Vec::new(),
        };
        if ctx.autoriser_changement_site {
            if let Some(classe) = eleve.classe.as_deref() {
                let tous = self.edt.list_pour_jour(etablissement_id, jour).await?;
                ctx.creneaux_apres = tous
                    .into_iter()
                    .filter(|c| c.classe == classe && !c.site.trim().is_empty())
                    .map(|c| SortieEdtApercu {
                        heure_debut: c.heure_debut,
                        site: c.site,
                    })
                    .collect();
            }
        }
        let ctrl = plan_controle_sortie_avec_contexte(
            etablissement_id,
            eleve_id,
            jour,
            creneau,
            auth.as_ref(),
            &demandes,
            actor_user_id,
            &ctx,
        )?;
        self.sorties.insert_controle(&ctrl).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::SORTIE_CONTROLE.to_string(),
                aggregate: AggregateKind::Sortie,
                aggregate_id: ctrl.id,
                eleve_id: Some(eleve_id),
                payload: json!({
                    "verdict": ctrl.verdict.as_str(),
                    "source": ctrl.source.as_str(),
                    "creneau": ctrl.creneau.as_str(),
                    "motif": ctrl.motif,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(ctrl)
    }

    pub async fn list_controles_sortie_jour_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<ControleSortie>, EntDbError> {
        if !Self::role_can_controle_sortie(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.sorties.list_controles_jour(etablissement_id, jour).await?)
    }

    pub fn role_can_gerer_internat(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_gerer_internat())
            .unwrap_or(false)
    }

    pub async fn creer_chambre_internat_for_actor(
        &self,
        role: &str,
        cmd: CreerChambreInternat,
    ) -> Result<ChambreInternat, EntDbError> {
        if !Self::role_can_gerer_internat(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_chambre_internat(cmd)?;
        self.internat.insert_chambre(&planned).await?;
        let saved = self
            .internat
            .get_chambre(planned.etablissement_id, planned.id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::INTERNAT_CHAMBRE.to_string(),
                aggregate: AggregateKind::Internat,
                aggregate_id: saved.id,
                eleve_id: None,
                payload: json!({
                    "code": saved.code,
                    "batiment": saved.batiment,
                    "capacite": saved.capacite,
                    "genre": saved.genre.as_str(),
                }),
                actor_user_id: Some(saved.created_by_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn list_chambres_internat_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<ChambreInternat>, EntDbError> {
        if !Self::role_can_gerer_internat(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.internat.list_chambres(etablissement_id, true).await?)
    }

    pub async fn affecter_eleve_internat_for_actor(
        &self,
        role: &str,
        cmd: AffecterEleveInternat,
    ) -> Result<AffectationInternat, EntDbError> {
        if !Self::role_can_gerer_internat(role) {
            return Err(EntDbError::Forbidden);
        }
        let eleve = self
            .eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let chambre = self
            .internat
            .get_chambre(cmd.etablissement_id, cmd.chambre_id)
            .await?
            .ok_or(DomainError::InternatChambreInvalide)?;
        let occupants = self
            .internat
            .count_occupants_actifs(cmd.etablissement_id, cmd.chambre_id, cmd.debut)
            .await?;
        let deja = self
            .internat
            .has_affectation_ouverte(cmd.etablissement_id, cmd.eleve_id)
            .await?;
        let planned = plan_affecter_eleve_internat(
            cmd,
            &chambre,
            eleve.regime.as_deref(),
            occupants,
            deja,
        )?;
        self.internat.insert_affectation(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::INTERNAT_AFFECTATION.to_string(),
                aggregate: AggregateKind::Internat,
                aggregate_id: planned.id,
                eleve_id: Some(planned.eleve_id),
                payload: json!({
                    "chambre_id": planned.chambre_id,
                    "debut": planned.debut.to_string(),
                    "lit": planned.lit,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn clore_affectation_internat_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        affectation_id: Uuid,
        fin: NaiveDate,
        actor_user_id: Uuid,
    ) -> Result<AffectationInternat, EntDbError> {
        if !Self::role_can_gerer_internat(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .internat
            .get_affectation(etablissement_id, affectation_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let closed = plan_clore_affectation_internat(current, etablissement_id, fin)?;
        self.internat.update_affectation_fin(&closed).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::INTERNAT_AFFECTATION.to_string(),
                aggregate: AggregateKind::Internat,
                aggregate_id: closed.id,
                eleve_id: Some(closed.eleve_id),
                payload: json!({
                    "fin": fin.to_string(),
                    "cloture": true,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(closed)
    }

    pub async fn pointer_presence_internat_for_actor(
        &self,
        role: &str,
        cmd: PointerPresenceInternat,
    ) -> Result<PresenceInternat, EntDbError> {
        if !Self::role_can_gerer_internat(role) {
            return Err(EntDbError::Forbidden);
        }
        let aff = self
            .internat
            .get_affectation_active(cmd.etablissement_id, cmd.eleve_id, cmd.jour)
            .await?
            .ok_or(DomainError::InternatAffectationInvalide)?;
        let planned = plan_pointer_presence_internat(cmd, &aff)?;
        self.internat.upsert_presence(&planned).await?;
        let saved = self
            .internat
            .get_presence(planned.etablissement_id, planned.eleve_id, planned.jour)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::INTERNAT_PRESENCE.to_string(),
                aggregate: AggregateKind::Internat,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({
                    "jour": saved.jour.to_string(),
                    "statut": saved.statut.as_str(),
                }),
                actor_user_id: Some(saved.actor_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn list_presences_internat_jour_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<PresenceInternat>, EntDbError> {
        if !Self::role_can_gerer_internat(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.internat.list_presences_jour(etablissement_id, jour).await?)
    }

    pub async fn list_affectations_internat_jour_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<AffectationInternat>, EntDbError> {
        if !Self::role_can_gerer_internat(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .internat
            .list_affectations_actives(etablissement_id, jour)
            .await?)
    }

    pub async fn count_msg_non_lus_par_fils(
        &self,
        etablissement_id: Uuid,
        lecteur_user_id: Uuid,
        fil_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, i64>, EntDbError> {
        Ok(self
            .messages
            .count_non_lus_par_fils(etablissement_id, lecteur_user_id, fil_ids)
            .await?)
    }

    pub async fn creer_suivi_internat_for_actor(
        &self,
        role: &str,
        cmd: CreerSuiviInternat,
    ) -> Result<SuiviInternat, EntDbError> {
        if !Self::role_can_gerer_internat(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_creer_suivi_internat(cmd)?;
        self.internat.insert_suivi(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::INTERNAT_SUIVI.to_string(),
                aggregate: AggregateKind::Internat,
                aggregate_id: planned.id,
                eleve_id: Some(planned.eleve_id),
                payload: json!({
                    "jour": planned.jour.to_string(),
                    "visible_famille": planned.visible_famille,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn get_recap_internat_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<RecapInternatEleve, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_gerer_internat(role) || parsed.can_read_all_eleves();
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        self.eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let affectation = self
            .internat
            .get_affectation_active(etablissement_id, eleve_id, jour)
            .await?;
        let chambre = if let Some(ref a) = affectation {
            self.internat
                .get_chambre(etablissement_id, a.chambre_id)
                .await?
        } else {
            None
        };
        let presence_du_jour = self
            .internat
            .get_presence(etablissement_id, eleve_id, jour)
            .await?;
        let suivis_famille = self
            .internat
            .list_suivis_eleve(etablissement_id, eleve_id, !staff)
            .await?;
        Ok(RecapInternatEleve {
            affectation,
            chambre,
            presence_du_jour,
            suivis_famille,
        })
    }

    pub fn role_can_gerer_dispenses_vs(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_gerer_dispenses_vs())
            .unwrap_or(false)
    }

    pub async fn poser_dispense_vs_for_actor(
        &self,
        role: &str,
        cmd: PoserDispenseVs,
    ) -> Result<DispenseVs, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_gerer_dispenses_vs(role);
        let parent = matches!(parsed, Role::Parent);
        if !staff && !parent {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_poser_dispense_vs(cmd)?;
        self.dispenses_vs.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::DISPENSE_VS.to_string(),
                aggregate: AggregateKind::DispenseVs,
                aggregate_id: planned.id,
                eleve_id: Some(planned.eleve_id),
                payload: json!({
                    "motif": planned.motif.as_str(),
                    "debut": planned.debut.to_string(),
                    "fin": planned.fin.to_string(),
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        let _ = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id: planned.etablissement_id,
                eleve_id: Some(planned.eleve_id),
                domaine: "vs".into(),
                action: "dispense_posee".into(),
                libelle: format!(
                    "Dispense {} du {} au {}",
                    planned.motif.as_str(),
                    planned.debut,
                    planned.fin
                ),
                payload: json!({ "dispense_id": planned.id, "motif": planned.motif.as_str() }),
                visible_famille: true,
                actor_user_id: Some(planned.created_by_user_id),
            })
            .await;
        Ok(planned)
    }

    pub async fn clore_dispense_vs_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        dispense_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<DispenseVs, EntDbError> {
        if !Self::role_can_gerer_dispenses_vs(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .dispenses_vs
            .get(etablissement_id, dispense_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let closed = plan_clore_dispense_vs(current, etablissement_id, actor_user_id)?;
        self.dispenses_vs.update_cloture(&closed).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::DISPENSE_VS_CLOTURE.to_string(),
                aggregate: AggregateKind::DispenseVs,
                aggregate_id: closed.id,
                eleve_id: Some(closed.eleve_id),
                payload: json!({ "statut": closed.statut.as_str() }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        let _ = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id,
                eleve_id: Some(closed.eleve_id),
                domaine: "vs".into(),
                action: "dispense_cloturee".into(),
                libelle: format!("Clôture dispense {}", closed.motif.as_str()),
                payload: json!({ "dispense_id": closed.id }),
                visible_famille: true,
                actor_user_id: Some(actor_user_id),
            })
            .await;
        Ok(closed)
    }

    async fn enregistrer_journal_interne(
        &self,
        cmd: EnregistrerJournal,
    ) -> Result<JournalEntree, EntDbError> {
        let planned = plan_enregistrer_journal(cmd)?;
        self.journal.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::JOURNAL_ENTREE.to_string(),
                aggregate: AggregateKind::Journal,
                aggregate_id: planned.id,
                eleve_id: planned.eleve_id,
                payload: json!({
                    "domaine": planned.domaine,
                    "action": planned.action,
                    "libelle": planned.libelle,
                }),
                actor_user_id: planned.actor_user_id.map(|u| u.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_journal_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        limit: i64,
    ) -> Result<Vec<JournalEntree>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_read_all_eleves() || parsed.can_manage_eleves();
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        // Parent / élève : uniquement les entrées `visible_famille`.
        let famille_only = parent || eleve;
        let lim = if limit <= 0 { 50 } else { limit.min(200) };
        Ok(self
            .journal
            .list_eleve(etablissement_id, eleve_id, famille_only, lim)
            .await?)
    }

    pub async fn list_repas_activites_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<RepasActivite>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(Self::role_can_manage_repas(role)
            || parsed.can_read_all_eleves()
            || parsed.can_manage_eleves()
            || matches!(parsed, Role::Parent | Role::Eleve))
        {
            return Err(EntDbError::Forbidden);
        }
        self.list_repas_activites(etablissement_id).await
    }

    pub async fn list_grilles_repas_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<EleveGrilleRepas>, EntDbError> {
        let dossier = self
            .get_dossier_for_actor(
                etablissement_id,
                actor_user_id,
                role,
                eleve_id,
                today_utc(),
            )
            .await?
            .ok_or(EntDbError::Forbidden)?;
        if !dossier.sections.repas {
            return Err(EntDbError::Forbidden);
        }
        Ok(dossier.grille_repas)
    }

    pub fn role_can_gerer_alertes_cpe(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_gerer_alertes_cpe())
            .unwrap_or(false)
    }

    pub async fn creer_alerte_cpe_for_actor(
        &self,
        role: &str,
        cmd: CreerAlerteCpe,
    ) -> Result<AlerteCpe, EntDbError> {
        if !Self::role_can_gerer_alertes_cpe(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_creer_alerte_cpe(cmd)?;
        self.alertes_cpe.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::ALERTE_CPE.to_string(),
                aggregate: AggregateKind::AlerteCpe,
                aggregate_id: planned.id,
                eleve_id: Some(planned.eleve_id),
                payload: json!({
                    "titre": planned.titre,
                    "severite": planned.severite.as_str(),
                    "visible_famille": planned.visible_famille,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        let _ = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id: planned.etablissement_id,
                eleve_id: Some(planned.eleve_id),
                domaine: "vs".into(),
                action: "alerte_cpe".into(),
                libelle: format!("Alerte CPE : {}", planned.titre),
                payload: json!({
                    "alerte_id": planned.id,
                    "severite": planned.severite.as_str(),
                }),
                visible_famille: planned.visible_famille,
                actor_user_id: Some(planned.created_by_user_id),
            })
            .await;
        Ok(planned)
    }

    pub async fn clore_alerte_cpe_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        alerte_id: Uuid,
        actor_user_id: Uuid,
        note: String,
    ) -> Result<AlerteCpe, EntDbError> {
        if !Self::role_can_gerer_alertes_cpe(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .alertes_cpe
            .get(etablissement_id, alerte_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let closed = plan_clore_alerte_cpe(current, etablissement_id, actor_user_id, note)?;
        self.alertes_cpe.update_cloture(&closed).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ALERTE_CPE_CLOTURE.to_string(),
                aggregate: AggregateKind::AlerteCpe,
                aggregate_id: closed.id,
                eleve_id: Some(closed.eleve_id),
                payload: json!({ "cloture_note": closed.cloture_note }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(closed)
    }

    pub async fn list_alertes_cpe_ouvertes_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<AlerteCpe>, EntDbError> {
        if !Self::role_can_gerer_alertes_cpe(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.alertes_cpe.list_ouvertes(etablissement_id).await?)
    }

    pub async fn list_alertes_cpe_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<AlerteCpe>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_gerer_alertes_cpe(role) || parsed.can_read_all_eleves();
        let parent = matches!(parsed, Role::Parent);
        if !staff && !parent {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .alertes_cpe
            .list_eleve(etablissement_id, eleve_id, parent)
            .await?)
    }

    pub fn role_can_publier_docs_a_signer(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_publier_docs_a_signer())
            .unwrap_or(false)
    }

    pub async fn publier_doc_a_signer_for_actor(
        &self,
        role: &str,
        cmd: PublierDocASigner,
    ) -> Result<DocASigner, EntDbError> {
        if !Self::role_can_publier_docs_a_signer(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_publier_doc_a_signer(cmd)?;
        self.docs_signer.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::DOC_A_SIGNER.to_string(),
                aggregate: AggregateKind::DocASigner,
                aggregate_id: planned.id,
                eleve_id: Some(planned.eleve_id),
                payload: json!({ "code": planned.code, "titre": planned.titre }),
                actor_user_id: Some(planned.publie_par_user_id.to_string()),
            })
            .await?;
        let _ = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id: planned.etablissement_id,
                eleve_id: Some(planned.eleve_id),
                domaine: "docs".into(),
                action: "doc_publie".into(),
                libelle: format!("Document à signer : {}", planned.titre),
                payload: json!({ "doc_id": planned.id, "code": planned.code }),
                visible_famille: true,
                actor_user_id: Some(planned.publie_par_user_id),
            })
            .await;
        Ok(planned)
    }

    pub async fn signer_doc_a_signer_for_actor(
        &self,
        role: &str,
        cmd: SignerDocASigner,
    ) -> Result<DocASigner, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !matches!(parsed, Role::Parent | Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .docs_signer
            .get(cmd.etablissement_id, cmd.doc_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if matches!(parsed, Role::Parent) {
            if self
                .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, current.eleve_id)
                .await?
                .is_none()
            {
                return Err(EntDbError::Forbidden);
            }
        } else if matches!(parsed, Role::Eleve) {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(cmd.etablissement_id, cmd.actor_user_id)
                .await?;
            if linked != Some(current.eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        let signed = plan_signer_doc_a_signer(current, cmd)?;
        self.docs_signer.update_signature(&signed).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: signed.etablissement_id,
                type_name: types::DOC_SIGNE.to_string(),
                aggregate: AggregateKind::DocASigner,
                aggregate_id: signed.id,
                eleve_id: Some(signed.eleve_id),
                payload: json!({ "code": signed.code }),
                actor_user_id: signed.signe_par_user_id.map(|u| u.to_string()),
            })
            .await?;
        Ok(signed)
    }

    pub async fn annuler_doc_a_signer_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        doc_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<DocASigner, EntDbError> {
        if !Self::role_can_publier_docs_a_signer(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .docs_signer
            .get(etablissement_id, doc_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let cancelled = plan_annuler_doc_a_signer(current, etablissement_id)?;
        self.docs_signer.update_statut(&cancelled).await?;
        let _ = actor_user_id;
        Ok(cancelled)
    }

    pub async fn list_docs_a_signer_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        a_signer_seulement: bool,
    ) -> Result<Vec<DocASigner>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_publier_docs_a_signer(role) || parsed.can_read_all_eleves();
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        Ok(self
            .docs_signer
            .list_eleve(etablissement_id, eleve_id, a_signer_seulement)
            .await?)
    }

    pub async fn list_docs_a_signer_etab_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<DocASigner>, EntDbError> {
        if !Self::role_can_publier_docs_a_signer(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.docs_signer.list_a_signer_etab(etablissement_id).await?)
    }

    pub fn role_can_gerer_conseil_classe(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_gerer_conseil_classe())
            .unwrap_or(false)
    }

    pub async fn creer_conseil_classe_for_actor(
        &self,
        role: &str,
        cmd: CreerConseilClasse,
    ) -> Result<ConseilClasse, EntDbError> {
        if !Self::role_can_gerer_conseil_classe(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_conseil_classe(cmd)?;
        self.conseils.insert_conseil(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::CONSEIL_CLASSE.to_string(),
                aggregate: AggregateKind::ConseilClasse,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "classe": planned.classe,
                    "periode": planned.periode_libelle,
                    "jour": planned.jour.to_string(),
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn tenir_conseil_classe_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        conseil_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<ConseilClasse, EntDbError> {
        if !Self::role_can_gerer_conseil_classe(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .conseils
            .get_conseil(etablissement_id, conseil_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let held = plan_tenir_conseil(current, etablissement_id)?;
        self.conseils.update_statut(&held).await?;
        let _ = actor_user_id;
        Ok(held)
    }

    pub async fn ajouter_dossier_conseil_for_actor(
        &self,
        role: &str,
        cmd: AjouterDossierConseil,
    ) -> Result<ConseilClasseDossier, EntDbError> {
        if !Self::role_can_gerer_conseil_classe(role) {
            return Err(EntDbError::Forbidden);
        }
        self.conseils
            .get_conseil(cmd.etablissement_id, cmd.conseil_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_ajouter_dossier_conseil(cmd)?;
        self.conseils.insert_dossier(&planned).await?;
        Ok(planned)
    }

    pub async fn decider_dossier_conseil_for_actor(
        &self,
        role: &str,
        cmd: DeciderDossierConseil,
    ) -> Result<ConseilClasseDossier, EntDbError> {
        if !Self::role_can_gerer_conseil_classe(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .conseils
            .get_dossier(cmd.etablissement_id, cmd.dossier_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let decided = plan_decider_dossier_conseil(current, cmd)?;
        self.conseils.update_dossier(&decided).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: decided.etablissement_id,
                type_name: types::CONSEIL_DOSSIER.to_string(),
                aggregate: AggregateKind::ConseilClasse,
                aggregate_id: decided.id,
                eleve_id: Some(decided.eleve_id),
                payload: json!({
                    "mention": decided.mention.map(|m| m.as_str()),
                    "visible_famille": decided.visible_famille,
                }),
                actor_user_id: decided.decided_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        if decided.visible_famille {
            let _ = self
                .enregistrer_journal_interne(EnregistrerJournal {
                    etablissement_id: decided.etablissement_id,
                    eleve_id: Some(decided.eleve_id),
                    domaine: "notes".into(),
                    action: "conseil_decision".into(),
                    libelle: format!(
                        "Conseil de classe — mention {}",
                        decided
                            .mention
                            .map(|m| m.as_str())
                            .unwrap_or("aucune")
                    ),
                    payload: json!({ "dossier_id": decided.id }),
                    visible_famille: true,
                    actor_user_id: decided.decided_by_user_id,
                })
                .await;
        }
        Ok(decided)
    }

    pub async fn list_conseils_classe_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        classe: Option<&str>,
    ) -> Result<Vec<ConseilClasse>, EntDbError> {
        if !Self::role_can_gerer_conseil_classe(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.conseils.list_conseils(etablissement_id, classe).await?)
    }

    pub async fn list_dossiers_conseil_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        conseil_id: Uuid,
    ) -> Result<Vec<ConseilClasseDossier>, EntDbError> {
        if !Self::role_can_gerer_conseil_classe(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .conseils
            .list_dossiers_conseil(etablissement_id, conseil_id)
            .await?)
    }

    pub async fn list_conseils_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<ConseilClasseDossier>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_gerer_conseil_classe(role) || parsed.can_read_all_eleves();
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let own = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, user_id)
                .await?;
            if own != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        let famille_seulement = parent || eleve;
        Ok(self
            .conseils
            .list_dossiers_eleve(etablissement_id, eleve_id, famille_seulement)
            .await?)
    }

    pub async fn creer_info_comp_champ_for_actor(
        &self,
        role: &str,
        cmd: CreerInfoCompChamp,
    ) -> Result<InfoCompChamp, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_info_comp_champ(cmd)?;
        self.info_comp.insert_champ(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::INFO_COMP_CHAMP.to_string(),
                aggregate: AggregateKind::InfoComp,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({ "code": planned.code, "type": planned.type_valeur.as_str() }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_info_comp_champs_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<InfoCompChamp>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(parsed.can_manage_eleves() || parsed.can_read_all_eleves()) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.info_comp.list_champs(etablissement_id, true).await?)
    }

    pub async fn upsert_info_comp_valeur_for_actor(
        &self,
        role: &str,
        cmd: UpsertInfoCompValeur,
    ) -> Result<InfoCompValeur, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_manage_eleves() || Self::role_can_gerer_alertes_cpe(role);
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        let champ = self
            .info_comp
            .get_champ(cmd.etablissement_id, cmd.champ_id)
            .await?
            .ok_or(DomainError::InfoCompInvalide)?;
        if parent || eleve {
            if !champ.visible_famille {
                return Err(EntDbError::Forbidden);
            }
        }
        if parent
            && self
                .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(cmd.etablissement_id, cmd.actor_user_id)
                .await?;
            if linked != Some(cmd.eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_upsert_info_comp_valeur(cmd, &champ)?;
        self.info_comp.upsert_valeur(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::INFO_COMP_VALEUR.to_string(),
                aggregate: AggregateKind::InfoComp,
                aggregate_id: planned.id,
                eleve_id: Some(planned.eleve_id),
                payload: json!({ "champ_id": planned.champ_id }),
                actor_user_id: Some(planned.updated_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_info_comp_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<(Vec<InfoCompChamp>, Vec<InfoCompValeur>), EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_manage_eleves() || parsed.can_read_all_eleves();
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        let mut champs = self.info_comp.list_champs(etablissement_id, true).await?;
        if parent || eleve {
            champs.retain(|c| c.visible_famille);
        }
        let valeurs = self
            .info_comp
            .list_valeurs_eleve(etablissement_id, eleve_id)
            .await?;
        let valeurs = if parent || eleve {
            let ids: std::collections::HashSet<_> = champs.iter().map(|c| c.id).collect();
            valeurs
                .into_iter()
                .filter(|v| ids.contains(&v.champ_id))
                .collect()
        } else {
            valeurs
        };
        Ok((champs, valeurs))
    }

    pub async fn creer_periode_vacances_for_actor(
        &self,
        role: &str,
        cmd: CreerPeriodeVacances,
    ) -> Result<PeriodeVacances, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_periode_vacances(cmd)?;
        self.vacances.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::VACANCES.to_string(),
                aggregate: AggregateKind::Vacances,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "libelle": planned.libelle,
                    "kind": planned.kind.as_str(),
                    "debut": planned.debut.to_string(),
                    "fin": planned.fin.to_string(),
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_periodes_vacances_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<PeriodeVacances>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(parsed.is_staff() || matches!(parsed, Role::Parent | Role::Eleve)) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.vacances.list(etablissement_id).await?)
    }

    pub async fn jour_ferme_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(parsed.is_staff() || matches!(parsed, Role::Parent | Role::Eleve)) {
            return Err(EntDbError::Forbidden);
        }
        let periodes = self.vacances.list_couvrant(etablissement_id, jour).await?;
        Ok(jour_est_ferme(&periodes, jour))
    }

    pub async fn supprimer_periode_vacances_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        periode_id: Uuid,
    ) -> Result<(), EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        if !self.vacances.delete(etablissement_id, periode_id).await? {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        Ok(())
    }

    pub async fn rechercher_homonymes_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        cmd: RechercherHomonymes,
    ) -> Result<Vec<HomonymeCandidat>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() && !parsed.can_read_all_eleves() {
            return Err(EntDbError::Forbidden);
        }
        let date_naissance = cmd.date_naissance;
        let (nom, prenom) = plan_rechercher_homonymes(RechercherHomonymes {
            etablissement_id,
            nom: cmd.nom,
            prenom: cmd.prenom,
            date_naissance,
        })?;
        let candidats = self.eleves.search(etablissement_id, &nom, 50).await?;
        let mut out: Vec<HomonymeCandidat> = candidats
            .into_iter()
            .filter_map(|e| {
                let score = score_homonyme(
                    &nom,
                    &prenom,
                    date_naissance,
                    &e.nom,
                    &e.prenom,
                    e.date_naissance,
                );
                if score < 50 {
                    return None;
                }
                Some(HomonymeCandidat {
                    eleve_id: e.id,
                    nom: e.nom,
                    prenom: e.prenom,
                    date_naissance: e.date_naissance,
                    classe: e.classe,
                    score,
                })
            })
            .collect();
        out.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.nom.cmp(&b.nom)));
        Ok(out)
    }

    pub async fn upsert_menu_cantine_for_actor(
        &self,
        role: &str,
        cmd: UpsertMenuCantine,
    ) -> Result<MenuCantineJour, EntDbError> {
        if !Self::role_can_manage_repas(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_upsert_menu_cantine(cmd)?;
        self.menus.upsert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::MENU_CANTINE.to_string(),
                aggregate: AggregateKind::MenuCantine,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "jour": planned.jour.to_string(),
                    "plat": planned.plat,
                    "publie": planned.publie,
                }),
                actor_user_id: Some(planned.updated_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_menus_cantine_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<MenuCantineJour>, EntDbError> {
        let _ = user_id;
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_manage_repas(role) || parsed.can_read_all_eleves();
        let famille = matches!(parsed, Role::Parent | Role::Eleve);
        if !staff && !famille {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .menus
            .list_periode(etablissement_id, debut, fin, famille)
            .await?)
    }

    pub async fn upsert_quotient_foyer_for_actor(
        &self,
        role: &str,
        cmd: UpsertQuotientFoyer,
    ) -> Result<QuotientFoyer, EntDbError> {
        if !Self::role_can_manage_facturation(role) && !Self::role_can_manage_bourses(role) {
            return Err(EntDbError::Forbidden);
        }
        if !self
            .facturation
            .foyer_exists(cmd.etablissement_id, cmd.foyer_id)
            .await?
        {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        let planned = plan_upsert_quotient_foyer(cmd)?;
        let saved = self.quotients.upsert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::QUOTIENT_FOYER.to_string(),
                aggregate: AggregateKind::Quotient,
                aggregate_id: saved.id,
                eleve_id: None,
                payload: json!({
                    "foyer_id": saved.foyer_id,
                    "annee_scolaire": saved.annee_scolaire,
                    "quotient_centimes": saved.quotient_centimes,
                    "parts_x100": saved.parts_x100,
                }),
                actor_user_id: Some(saved.updated_by_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn get_quotient_foyer_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        foyer_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Option<QuotientFoyer>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff =
            Self::role_can_manage_facturation(role) || Self::role_can_manage_bourses(role);
        let parent = matches!(parsed, Role::Parent);
        if !staff && !parent {
            return Err(EntDbError::Forbidden);
        }
        if parent {
            let foyers = self.foyer_ids_parent(etablissement_id, user_id).await?;
            if !foyers.contains(&foyer_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        Ok(self
            .quotients
            .get(etablissement_id, foyer_id, annee_scolaire)
            .await?)
    }

    pub async fn creer_taux_tva_for_actor(
        &self,
        role: &str,
        cmd: CreerTauxTva,
    ) -> Result<TauxTva, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_taux_tva(cmd)?;
        self.tva.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::TAUX_TVA.to_string(),
                aggregate: AggregateKind::Tva,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "code": planned.code,
                    "taux_bps": planned.taux_bps,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_taux_tva_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<TauxTva>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(Self::role_can_manage_facturation(role) || parsed.can_read_all_eleves()) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.tva.list(etablissement_id, true).await?)
    }

    pub async fn calculer_ttc_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        ht_centimes: i64,
        taux_bps: i32,
    ) -> Result<i64, EntDbError> {
        let _ = etablissement_id;
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(Self::role_can_manage_facturation(role) || parsed.can_read_all_eleves()) {
            return Err(EntDbError::Forbidden);
        }
        if !(0..=10000).contains(&taux_bps) {
            return Err(EntDbError::Domain(DomainError::TvaInvalide));
        }
        Ok(ttc_depuis_ht(ht_centimes, taux_bps))
    }

    pub async fn poser_photo_eleve_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        actor_user_id: Uuid,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<Eleve, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if parsed.can_manage_eleves() {
            // staff manage
        } else if matches!(parsed, Role::Parent) {
            if self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?
                .is_none()
            {
                return Err(EntDbError::Forbidden);
            }
        } else {
            return Err(EntDbError::Forbidden);
        }
        let ext = plan_poser_photo_eleve(PoserPhotoEleve {
            etablissement_id,
            eleve_id,
            content_type: content_type.to_string(),
            bytes_len: bytes.len(),
            actor_user_id,
        })?;
        self.eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let key = ObjectStore::key_for(etablissement_id, "trombi", eleve_id, &ext);
        self.objects.put(&key, bytes)?;
        let eleve = self
            .update_identite(UpdateIdentite {
                etablissement_id,
                eleve_id,
                nom: None,
                prenom: None,
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: None,
                photo_key: Some(Some(key.clone())),
                date_entree: None,
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::PHOTO_ELEVE.to_string(),
                aggregate: AggregateKind::Trombi,
                aggregate_id: eleve_id,
                eleve_id: Some(eleve_id),
                payload: json!({ "photo_key": key }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(eleve)
    }

    pub async fn list_trombi_classe_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        classe: &str,
        jour: NaiveDate,
    ) -> Result<Vec<TrombiEntree>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(parsed.can_manage_eleves()
            || parsed.can_read_all_eleves()
            || parsed.can_faire_appel())
        {
            return Err(EntDbError::Forbidden);
        }
        let eleves = self
            .eleves
            .list_en_classe_au(etablissement_id, classe, jour)
            .await?;
        Ok(eleves
            .into_iter()
            .map(|e| TrombiEntree {
                eleve_id: e.id,
                nom: e.nom,
                prenom: e.prenom,
                classe: e.classe,
                photo_key: e.photo_key,
            })
            .collect())
    }

    pub async fn get_photo_eleve_bytes_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<Vec<u8>>, EntDbError> {
        let eleve = self
            .get_eleve_for_actor(etablissement_id, user_id, role, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let Some(key) = eleve.photo_key.as_ref() else {
            return Ok(None);
        };
        Ok(Some(self.objects.get(key)?))
    }

    pub async fn upsert_param_etab_for_actor(
        &self,
        role: &str,
        cmd: UpsertParamEtab,
    ) -> Result<ParamEtab, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_upsert_param_etab(cmd)?;
        let saved = self.params.upsert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::PARAM_ETAB.to_string(),
                aggregate: AggregateKind::ParamEtab,
                aggregate_id: saved.id,
                eleve_id: None,
                payload: json!({ "cle": saved.cle, "valeur": saved.valeur }),
                actor_user_id: Some(saved.updated_by_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn list_params_etab_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<ParamEtab>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.params.list(etablissement_id).await?)
    }

    pub async fn get_param_etab_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        cle: &str,
    ) -> Result<Option<ParamEtab>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let cle = cle.trim().to_ascii_lowercase();
        Ok(self.params.get(etablissement_id, &cle).await?)
    }

    pub async fn creer_salle_for_actor(
        &self,
        role: &str,
        cmd: CreerSalle,
    ) -> Result<Salle, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_edt() && !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_salle(cmd)?;
        self.salles.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::SALLE.to_string(),
                aggregate: AggregateKind::Salle,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "code": planned.code,
                    "libelle": planned.libelle,
                    "site": planned.site,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_salles_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<Salle>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.salles.list(etablissement_id, true).await?)
    }

    pub async fn creer_filiere_for_actor(
        &self,
        role: &str,
        cmd: CreerFiliere,
    ) -> Result<Filiere, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_filiere(cmd)?;
        self.filieres.insert_filiere(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::FILIERE.to_string(),
                aggregate: AggregateKind::Filiere,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "code": planned.code,
                    "libelle": planned.libelle,
                    "mef": planned.mef,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_filieres_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<Filiere>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(parsed.is_staff() || matches!(parsed, Role::Parent | Role::Eleve)) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.filieres.list_filieres(etablissement_id, true).await?)
    }

    pub async fn poser_voeu_for_actor(
        &self,
        role: &str,
        cmd: PoserVoeu,
    ) -> Result<VoeuInscription, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_manage_eleves() || parsed.can_manage_continuum();
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(cmd.etablissement_id, cmd.actor_user_id)
                .await?;
            if linked != Some(cmd.eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.filieres
            .get_filiere(cmd.etablissement_id, cmd.filiere_id)
            .await?
            .ok_or(DomainError::FiliereInvalide)?;
        let planned = plan_poser_voeu(cmd)?;
        let saved = self.filieres.upsert_voeu(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::VOEU.to_string(),
                aggregate: AggregateKind::Voeu,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({
                    "filiere_id": saved.filiere_id,
                    "rang": saved.rang,
                    "annee_scolaire": saved.annee_scolaire,
                }),
                actor_user_id: Some(saved.created_by_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn list_voeux_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Vec<VoeuInscription>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_manage_eleves() || parsed.can_read_all_eleves();
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        Ok(self
            .filieres
            .list_voeux_eleve(etablissement_id, eleve_id, annee_scolaire)
            .await?)
    }

    pub async fn upsert_inscription_suivi_for_actor(
        &self,
        role: &str,
        cmd: UpsertInscriptionSuivi,
    ) -> Result<InscriptionSuivi, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() && !parsed.can_manage_continuum() {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_upsert_inscription_suivi(cmd)?;
        let saved = self.inscriptions.upsert_suivi(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: saved.etablissement_id,
                type_name: types::INSCRIPTION_SUIVI.to_string(),
                aggregate: AggregateKind::InscriptionSuivi,
                aggregate_id: saved.id,
                eleve_id: Some(saved.eleve_id),
                payload: json!({
                    "etape": saved.etape.as_str(),
                    "motif_code": saved.motif_code,
                    "annee_scolaire": saved.annee_scolaire,
                }),
                actor_user_id: Some(saved.updated_by_user_id.to_string()),
            })
            .await?;
        Ok(saved)
    }

    pub async fn get_inscription_suivi_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Option<InscriptionSuivi>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_manage_eleves() || parsed.can_read_all_eleves();
        let parent = matches!(parsed, Role::Parent);
        if !staff && !parent {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .inscriptions
            .get_suivi(etablissement_id, eleve_id, annee_scolaire)
            .await?)
    }

    pub async fn list_inscriptions_par_etape_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        annee_scolaire: &str,
        etape: &str,
    ) -> Result<Vec<InscriptionSuivi>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .inscriptions
            .list_par_etape(etablissement_id, annee_scolaire, etape)
            .await?)
    }

    pub async fn exporter_lsl_csv_for_actor(
        &self,
        role: &str,
        cmd: ExporterLsl,
    ) -> Result<String, EntDbError> {
        if !Self::role_can_saisir_notes(role) && !Role::parse(role).is_some_and(|r| r.can_manage_eleves()) {
            return Err(EntDbError::Forbidden);
        }
        let cmd = plan_exporter_lsl(cmd)?;
        let periode = self
            .notes
            .get_periode(cmd.etablissement_id, cmd.periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let classe = cmd
            .classe
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or(DomainError::NotFound)?;
        let eleves = self
            .eleves
            .list_en_classe_au(cmd.etablissement_id, classe, cmd.jour_reference)
            .await?;
        let matieres = self.matieres.list(cmd.etablissement_id).await?;
        let matiere_code = |id: Uuid| -> String {
            matieres
                .iter()
                .find(|m| m.id == id)
                .map(|m| m.code.clone())
                .unwrap_or_else(|| id.to_string())
        };
        let mut lignes = Vec::new();
        for e in &eleves {
            let apps = self
                .notes
                .list_appreciations_eleve(cmd.etablissement_id, e.id, Some(cmd.periode_id))
                .await?;
            if apps.is_empty() {
                lignes.push(LigneLsl {
                    ine: e.ine.clone().unwrap_or_default(),
                    nom: e.nom.clone(),
                    prenom: e.prenom.clone(),
                    classe: e.classe.clone().unwrap_or_default(),
                    matiere_code: String::new(),
                    note_sur_20: None,
                    appreciation: String::new(),
                    periode: periode.code.clone(),
                });
            } else {
                for a in apps {
                    lignes.push(LigneLsl {
                        ine: e.ine.clone().unwrap_or_default(),
                        nom: e.nom.clone(),
                        prenom: e.prenom.clone(),
                        classe: e.classe.clone().unwrap_or_default(),
                        matiere_code: a
                            .matiere_id
                            .map(matiere_code)
                            .unwrap_or_else(|| "GENERALE".into()),
                        note_sur_20: None,
                        appreciation: a.texte,
                        periode: periode.code.clone(),
                    });
                }
            }
        }
        Ok(render_lsl_csv(&lignes))
    }

    pub async fn exporter_onde_csv_for_actor(
        &self,
        role: &str,
        cmd: ExporterOnde,
    ) -> Result<String, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() && !parsed.can_manage_continuum() {
            return Err(EntDbError::Forbidden);
        }
        let cmd = plan_exporter_onde(cmd)?;
        let eleves = if let Some(ref classe) = cmd.classe {
            self.eleves
                .list_en_classe_au(cmd.etablissement_id, classe, cmd.jour_reference)
                .await?
        } else {
            // Sans filtre classe : tous les élèves actifs au jour (via search large).
            self.eleves
                .search(cmd.etablissement_id, "", 500)
                .await?
        };
        let lignes: Vec<LigneOnde> = eleves
            .into_iter()
            .map(|e| LigneOnde {
                ine: e.ine.unwrap_or_default(),
                nom: e.nom,
                prenom: e.prenom,
                date_naissance: e
                    .date_naissance
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                sexe: e.sexe.unwrap_or_default(),
                classe: e.classe.unwrap_or_default(),
                mef: e.mef.unwrap_or_default(),
            })
            .collect();
        Ok(render_onde_csv(&lignes))
    }

    pub async fn creer_tarif_conso_for_actor(
        &self,
        role: &str,
        cmd: CreerTarifConso,
    ) -> Result<TarifConso, EntDbError> {
        if !Self::role_can_manage_repas(role) && !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_tarif_conso(cmd)?;
        self.tarifs.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::TARIF_CONSO.to_string(),
                aggregate: AggregateKind::TarifConso,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "code": planned.code,
                    "montant_centimes": planned.montant_centimes,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_tarifs_conso_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<TarifConso>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(Self::role_can_manage_repas(role)
            || Self::role_can_manage_facturation(role)
            || parsed.is_staff())
        {
            return Err(EntDbError::Forbidden);
        }
        if let Some(j) = jour {
            Ok(self.tarifs.list_au_jour(etablissement_id, j).await?)
        } else {
            Ok(self.tarifs.list(etablissement_id, true).await?)
        }
    }

    pub async fn exporter_notanet_csv_for_actor(
        &self,
        role: &str,
        cmd: ExporterNotanet,
        jour: NaiveDate,
    ) -> Result<String, EntDbError> {
        if !Self::role_can_saisir_notes(role)
            && !Role::parse(role).is_some_and(|r| r.can_manage_eleves())
        {
            return Err(EntDbError::Forbidden);
        }
        let cmd = plan_exporter_notanet(cmd)?;
        self.notes
            .get_periode(cmd.etablissement_id, cmd.periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let eleves = self
            .eleves
            .list_en_classe_au(cmd.etablissement_id, &cmd.classe, jour)
            .await?;
        let matieres = self.matieres.list(cmd.etablissement_id).await?;
        let mut lignes = Vec::new();
        for e in &eleves {
            let notes = self
                .notes
                .list_for_eleve(cmd.etablissement_id, e.id)
                .await?;
            for n in notes
                .into_iter()
                .filter(|n| n.periode_id == Some(cmd.periode_id))
            {
                let matiere = matieres
                    .iter()
                    .find(|m| m.id == n.matiere_id)
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "?".into());
                let note_20 = if n.sur > 0.0 {
                    n.valeur * 20.0 / n.sur
                } else {
                    n.valeur
                };
                lignes.push(LigneNotanet {
                    ine: e.ine.clone().unwrap_or_default(),
                    nom: e.nom.clone(),
                    prenom: e.prenom.clone(),
                    matiere,
                    note_sur_20: note_20,
                });
            }
        }
        Ok(render_notanet_csv(&lignes))
    }

    pub async fn exporter_parcoursup_csv_for_actor(
        &self,
        role: &str,
        cmd: ExporterParcoursup,
        jour: NaiveDate,
    ) -> Result<String, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.can_manage_eleves() || r.can_saisir_notes()) {
            return Err(EntDbError::Forbidden);
        }
        let cmd = plan_exporter_parcoursup(cmd)?;
        let eleves = self
            .eleves
            .list_en_classe_au(cmd.etablissement_id, &cmd.classe, jour)
            .await?;
        let filieres = self
            .filieres
            .list_filieres(cmd.etablissement_id, true)
            .await?;
        let filiere_lib = |id: Uuid| -> String {
            filieres
                .iter()
                .find(|f| f.id == id)
                .map(|f| f.libelle.clone())
                .unwrap_or_else(|| id.to_string())
        };
        let mut lignes = Vec::new();
        for e in &eleves {
            let voeux = self
                .filieres
                .list_voeux_eleve(cmd.etablissement_id, e.id, &cmd.annee_scolaire)
                .await?;
            if voeux.is_empty() {
                lignes.push(LigneParcoursup {
                    ine: e.ine.clone().unwrap_or_default(),
                    nom: e.nom.clone(),
                    prenom: e.prenom.clone(),
                    classe: e.classe.clone().unwrap_or_default(),
                    formation_souhaitee: String::new(),
                    rang_voeu: 0,
                    moyenne_generale: None,
                });
            } else {
                for v in voeux {
                    lignes.push(LigneParcoursup {
                        ine: e.ine.clone().unwrap_or_default(),
                        nom: e.nom.clone(),
                        prenom: e.prenom.clone(),
                        classe: e.classe.clone().unwrap_or_default(),
                        formation_souhaitee: filiere_lib(v.filiere_id),
                        rang_voeu: i32::from(v.rang),
                        moyenne_generale: None,
                    });
                }
            }
        }
        Ok(render_parcoursup_csv(&lignes))
    }

    pub async fn declarer_absence_rh_for_actor(
        &self,
        role: &str,
        cmd: DeclarerAbsenceRh,
    ) -> Result<AbsenceRh, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        // Soi-même ou admin RH.
        let self_or_rh = cmd.user_id == cmd.actor_user_id
            || parsed.can_manage_eleves()
            || matches!(parsed, Role::Direction | Role::Admin);
        if !self_or_rh {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_declarer_absence_rh(cmd)?;
        self.absences_rh.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::ABSENCE_RH.to_string(),
                aggregate: AggregateKind::AbsenceRh,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "user_id": planned.user_id,
                    "debut": planned.debut.to_string(),
                    "fin": planned.fin.to_string(),
                    "motif": planned.motif,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn valider_absence_rh_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        absence_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<AbsenceRh, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() && !matches!(parsed, Role::Direction | Role::Admin) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .absences_rh
            .get(etablissement_id, absence_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_valider_absence_rh(current, actor_user_id)?;
        self.absences_rh.update_statut(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::ABSENCE_RH_VALIDEE.to_string(),
                aggregate: AggregateKind::AbsenceRh,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({ "statut": planned.statut }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_absences_rh_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<AbsenceRh>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .absences_rh
            .list_periode(etablissement_id, debut, fin)
            .await?)
    }

    /// Affecte prof et/ou salle sur un créneau (conflits prof/salle/classe refusés).
    pub async fn affecter_edt_creneau_for_actor(
        &self,
        role: &str,
        cmd: AffecterEdtCreneau,
        salle_id: Option<Uuid>,
    ) -> Result<EdtCreneau, EntDbError> {
        if !Self::role_can_manage_edt(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .edt
            .get(cmd.etablissement_id, cmd.creneau_id)
            .await?
            .ok_or(DomainError::NotFound)?;

        let actor = cmd.actor_user_id;
        let mut salle_override = cmd.salle.clone();
        if let Some(sid) = salle_id {
            let salle = self
                .salles
                .get(cmd.etablissement_id, sid)
                .await?
                .ok_or(DomainError::SalleInvalide)?;
            if !salle.actif {
                return Err(DomainError::SalleInvalide.into());
            }
            salle_override = Some(salle.code);
        } else if let Some(code) = salle_override
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            // Si le code existe au référentiel, normaliser (casse / libellé code).
            if let Some(salle) = self
                .salles
                .get_by_code(cmd.etablissement_id, &code)
                .await?
            {
                if !salle.actif {
                    return Err(DomainError::SalleInvalide.into());
                }
                salle_override = Some(salle.code);
            }
        }

        let planned = plan_affecter_edt_creneau(
            current.prof_user_id,
            current.salle.as_deref(),
            AffecterEdtCreneau {
                salle: salle_override,
                ..cmd
            },
        )?;

        let candidate = EdtImportLigne {
            index: 0,
            classe: current.classe.clone(),
            matiere_libelle: current.matiere_libelle.clone(),
            prof_user_id: planned.prof_user_id,
            jour_semaine: current.jour_semaine,
            heure_debut: current.heure_debut,
            heure_fin: current.heure_fin,
            salle: planned.salle.clone(),
            semaine_ab: current.semaine_ab,
        };
        let others: Vec<EdtImportLigne> = self
            .edt
            .list_all(planned.etablissement_id)
            .await?
            .into_iter()
            .filter(|c| c.id != planned.creneau_id)
            .map(|c| EdtImportLigne {
                index: usize::MAX,
                classe: c.classe,
                matiere_libelle: c.matiere_libelle,
                prof_user_id: c.prof_user_id,
                jour_semaine: c.jour_semaine,
                heure_debut: c.heure_debut,
                heure_fin: c.heure_fin,
                salle: c.salle,
                semaine_ab: c.semaine_ab,
            })
            .collect();
        let report = detect_conflits_edt(&[candidate], &others);
        if !report.conflicts.is_empty() {
            return Err(DomainError::EdtConflit.into());
        }

        self.edt
            .update_affectation(
                planned.etablissement_id,
                planned.creneau_id,
                planned.prof_user_id,
                planned.salle.as_deref(),
            )
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::EDT_AFFECTATION.to_string(),
                aggregate: AggregateKind::Edt,
                aggregate_id: planned.creneau_id,
                eleve_id: None,
                payload: json!({
                    "prof_user_id": planned.prof_user_id,
                    "salle": planned.salle,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        self.edt
            .get(planned.etablissement_id, planned.creneau_id)
            .await?
            .ok_or(DomainError::NotFound.into())
    }

    pub async fn creer_edition_vs_for_actor(
        &self,
        role: &str,
        cmd: CreerEditionVs,
    ) -> Result<EditionVsTemplate, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves() && !matches!(parsed, Role::Cpe | Role::Direction | Role::Admin)
        {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_edition_vs(cmd)?;
        self.editions_vs.upsert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::EDITION_VS.to_string(),
                aggregate: AggregateKind::EditionVs,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({ "code": planned.code, "titre": planned.titre }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_editions_vs_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<EditionVsTemplate>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.editions_vs.list(etablissement_id).await?)
    }

    pub async fn rendre_edition_vs_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        template_id: Uuid,
        eleve_id: Uuid,
        date: NaiveDate,
    ) -> Result<String, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let tpl = self
            .editions_vs
            .get(etablissement_id, template_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.rendre_edition_corps(etablissement_id, &tpl.corps, eleve_id, date)
            .await
    }

    pub async fn rendre_edition_vs_par_code_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        code: &str,
        eleve_id: Uuid,
        date: NaiveDate,
    ) -> Result<String, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let code = code.trim().to_ascii_uppercase();
        let tpl = self
            .editions_vs
            .get_by_code(etablissement_id, &code)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.rendre_edition_corps(etablissement_id, &tpl.corps, eleve_id, date)
            .await
    }

    async fn rendre_edition_corps(
        &self,
        etablissement_id: Uuid,
        corps: &str,
        eleve_id: Uuid,
        date: NaiveDate,
    ) -> Result<String, EntDbError> {
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        Ok(rendre_edition_vs(
            corps,
            &RendreEditionVs {
                nom: eleve.nom,
                prenom: eleve.prenom,
                classe: eleve.classe.unwrap_or_default(),
                date: date.to_string(),
            },
        ))
    }

    pub async fn creer_edt_contrainte_for_actor(
        &self,
        role: &str,
        cmd: CreerEdtContrainte,
    ) -> Result<EdtContrainte, EntDbError> {
        if !Self::role_can_manage_edt(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_edt_contrainte(cmd)?;
        self.edt_contraintes.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::EDT_CONTRAINTE.to_string(),
                aggregate: AggregateKind::EdtContrainte,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "kind": planned.kind.as_str(),
                    "prof_user_id": planned.prof_user_id,
                    "salle_code": planned.salle_code,
                    "detail": planned.detail,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_edt_contraintes_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<EdtContrainte>, EntDbError> {
        if !Self::role_can_manage_edt(role)
            && !Role::parse(role).is_some_and(|r| r.is_staff())
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.edt_contraintes.list(etablissement_id).await?)
    }

    pub async fn creer_entreprise_for_actor(
        &self,
        role: &str,
        cmd: CreerEntreprise,
    ) -> Result<Entreprise, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves()
            && !matches!(parsed, Role::Administratif | Role::Direction | Role::Admin)
        {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_entreprise(cmd)?;
        self.stages.insert_entreprise(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::ENTREPRISE.to_string(),
                aggregate: AggregateKind::Entreprise,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "raison_sociale": planned.raison_sociale,
                    "siret": planned.siret,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_entreprises_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<Entreprise>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.stages.list_entreprises(etablissement_id).await?)
    }

    pub async fn creer_convention_stage_for_actor(
        &self,
        role: &str,
        cmd: CreerConventionStage,
    ) -> Result<ConventionStage, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_manage_eleves()
            && !matches!(parsed, Role::Administratif | Role::Direction | Role::Admin | Role::Cpe)
        {
            return Err(EntDbError::Forbidden);
        }
        self.stages
            .get_entreprise(cmd.etablissement_id, cmd.entreprise_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_creer_convention_stage(cmd)?;
        self.stages.insert_convention(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::CONVENTION_STAGE.to_string(),
                aggregate: AggregateKind::ConventionStage,
                aggregate_id: planned.id,
                eleve_id: Some(planned.eleve_id),
                payload: json!({
                    "entreprise_id": planned.entreprise_id,
                    "debut": planned.debut.to_string(),
                    "fin": planned.fin.to_string(),
                    "tuteur": planned.tuteur_nom,
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    pub async fn list_conventions_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<ConventionStage>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if parsed.is_staff() {
            return Ok(self
                .stages
                .list_conventions_eleve(etablissement_id, eleve_id)
                .await?);
        }
        if matches!(parsed, Role::Parent) {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            if lien.is_none() {
                return Err(EntDbError::Forbidden);
            }
            return Ok(self
                .stages
                .list_conventions_eleve(etablissement_id, eleve_id)
                .await?);
        }
        if matches!(parsed, Role::Eleve) {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
            return Ok(self
                .stages
                .list_conventions_eleve(etablissement_id, eleve_id)
                .await?);
        }
        Err(EntDbError::Forbidden)
    }

    pub async fn exporter_lsun_csv_for_actor(
        &self,
        role: &str,
        cmd: ExporterLsun,
    ) -> Result<String, EntDbError> {
        if !Self::role_can_saisir_notes(role)
            && !Role::parse(role).is_some_and(|r| r.can_manage_eleves())
        {
            return Err(EntDbError::Forbidden);
        }
        let cmd = plan_exporter_lsun(cmd)?;
        let periode = self
            .notes
            .get_periode(cmd.etablissement_id, cmd.periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let classe = cmd
            .classe
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or(DomainError::NotFound)?;
        let eleves = self
            .eleves
            .list_en_classe_au(cmd.etablissement_id, classe, cmd.jour_reference)
            .await?;
        let matieres = self.matieres.list(cmd.etablissement_id).await?;
        let matiere_label = |id: Option<Uuid>| -> String {
            id.and_then(|mid| matieres.iter().find(|m| m.id == mid).map(|m| m.code.clone()))
                .unwrap_or_else(|| "GENERALE".into())
        };
        let mut lignes = Vec::new();
        for e in &eleves {
            let apps = self
                .notes
                .list_appreciations_eleve(cmd.etablissement_id, e.id, Some(cmd.periode_id))
                .await?;
            if apps.is_empty() {
                lignes.push(LigneLsun {
                    ine: e.ine.clone().unwrap_or_default(),
                    nom: e.nom.clone(),
                    prenom: e.prenom.clone(),
                    classe: e.classe.clone().unwrap_or_default(),
                    domaine: "D1".into(),
                    item: String::new(),
                    niveau: String::new(),
                    periode: periode.code.clone(),
                });
            } else {
                for a in apps {
                    let item = matiere_label(a.matiere_id);
                    let domaine = domaine_depuis_matiere(&item);
                    let niveau = if a.texte.to_ascii_lowercase().contains("maîtrise")
                        || a.texte.to_ascii_lowercase().contains("maitrise")
                    {
                        "MF".into()
                    } else if a.texte.to_ascii_lowercase().contains("fragile") {
                        "PA".into()
                    } else {
                        "A".into()
                    };
                    lignes.push(LigneLsun {
                        ine: e.ine.clone().unwrap_or_default(),
                        nom: e.nom.clone(),
                        prenom: e.prenom.clone(),
                        classe: e.classe.clone().unwrap_or_default(),
                        domaine,
                        item,
                        niveau,
                        periode: periode.code.clone(),
                    });
                }
            }
        }
        Ok(render_lsun_csv(&lignes))
    }

    pub async fn list_dispenses_vs_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        actives_seulement: bool,
    ) -> Result<Vec<DispenseVs>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_gerer_dispenses_vs(role)
            || parsed.can_read_all_eleves()
            || parsed.can_faire_appel();
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        Ok(self
            .dispenses_vs
            .list_eleve(etablissement_id, eleve_id, actives_seulement)
            .await?)
    }

    pub async fn list_dispenses_vs_jour_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<DispenseVs>, EntDbError> {
        if !Self::role_can_gerer_dispenses_vs(role)
            && !Role::parse(role)
                .map(|r| r.can_faire_appel() || r.can_lire_vs_absences())
                .unwrap_or(false)
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .dispenses_vs
            .list_actives_au_jour(etablissement_id, jour)
            .await?)
    }

    /// Helper appel / EDT : dispense active couvrant la matière ce jour.
    pub async fn dispense_vs_pour_appel(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
        matiere_libelle: Option<&str>,
    ) -> Result<Option<DispenseVs>, EntDbError> {
        let list = self
            .dispenses_vs
            .list_eleve(etablissement_id, eleve_id, true)
            .await?;
        Ok(dispense_vs_couvre(&list, jour, matiere_libelle).cloned())
    }

    pub fn role_can_ecrire_carnet(role: &str) -> bool {
        Role::parse(role)
            .map(|r| {
                matches!(
                    r,
                    Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Prof
                )
            })
            .unwrap_or(false)
    }

    pub fn role_can_saisir_infirmerie(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_saisir_infirmerie())
            .unwrap_or(false)
    }

    pub async fn creer_passage_infirmerie_for_actor(
        &self,
        role: &str,
        cmd: CreerPassageInfirmerie,
    ) -> Result<PassageInfirmerie, EntDbError> {
        if !Self::role_can_saisir_infirmerie(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let passage = plan_creer_passage_infirmerie(cmd)?;
        self.infirmerie.insert(&passage).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: passage.etablissement_id,
                type_name: types::INFIRMERIE_PASSAGE.to_string(),
                aggregate: AggregateKind::Infirmerie,
                aggregate_id: passage.id,
                eleve_id: Some(passage.eleve_id),
                payload: json!({
                    "motif": passage.motif.as_str(),
                    "jour": passage.jour.to_string(),
                    "visible_famille": passage.visible_famille,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(passage)
    }

    pub async fn list_passages_infirmerie_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<PassageInfirmerie>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff_saisie = parsed.can_saisir_infirmerie();
        let is_parent = parsed == Role::Parent;
        let is_eleve = parsed == Role::Eleve;

        let mut list = if let Some(eid) = eleve_id {
            if staff_saisie {
                // ok
            } else if is_parent {
                let lien = self
                    .lien_parent_eleve(etablissement_id, actor_user_id, eid)
                    .await?;
                if lien.is_none() {
                    return Err(EntDbError::Forbidden);
                }
            } else if is_eleve {
                let linked = self
                    .eleve_comptes
                    .eleve_id_for_user(etablissement_id, actor_user_id)
                    .await?;
                if linked != Some(eid) {
                    return Err(EntDbError::Forbidden);
                }
            } else {
                return Err(EntDbError::Forbidden);
            }
            self.infirmerie
                .list_eleve(etablissement_id, eid, 50)
                .await?
        } else if let Some(j) = jour {
            if !staff_saisie {
                return Err(EntDbError::Forbidden);
            }
            self.infirmerie.list_jour(etablissement_id, j, 100).await?
        } else if staff_saisie {
            self.infirmerie
                .list_jour(etablissement_id, today_utc(), 100)
                .await?
        } else {
            return Err(EntDbError::Forbidden);
        };

        if is_parent || is_eleve {
            list = list
                .into_iter()
                .filter(|p| p.visible_famille)
                .map(|p| vue_passage_infirmerie_famille(&p))
                .collect();
        }
        Ok(list)
    }

    pub async fn upsert_fiche_sante_for_actor(
        &self,
        role: &str,
        cmd: UpsertFicheSante,
    ) -> Result<FicheSante, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_saisir_infirmerie(role);
        let parent = matches!(parsed, Role::Parent);
        if !staff && !parent {
            return Err(EntDbError::Forbidden);
        }
        if parent {
            if self
                .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
                .await?
                .is_none()
            {
                return Err(EntDbError::Forbidden);
            }
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let mut cmd = cmd;
        if parent {
            // Famille : ne touche pas note confidentielle / PAI clinique.
            let existing = self
                .infirmerie
                .get_fiche(cmd.etablissement_id, cmd.eleve_id)
                .await?;
            cmd.note_confidentielle = existing
                .as_ref()
                .map(|f| f.note_confidentielle.clone())
                .unwrap_or_default();
            cmd.pai_actif = existing.map(|f| f.pai_actif).unwrap_or(false);
        }
        let actor = cmd.actor_user_id;
        let planned = plan_upsert_fiche_sante(cmd)?;
        let fiche = self.infirmerie.upsert_fiche(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: fiche.etablissement_id,
                type_name: types::INFIRMERIE_FICHE_SANTE.to_string(),
                aggregate: AggregateKind::Infirmerie,
                aggregate_id: fiche.id,
                eleve_id: Some(fiche.eleve_id),
                payload: json!({
                    "pai_actif": fiche.pai_actif,
                    "allergies": fiche.allergies,
                    "traitements": fiche.traitements,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(fiche)
    }

    pub async fn get_fiche_sante_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<FicheSante>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_saisir_infirmerie();
        if staff {
            return Ok(self.infirmerie.get_fiche(etablissement_id, eleve_id).await?);
        }
        if parsed == Role::Parent {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            if lien.is_none() {
                return Err(EntDbError::Forbidden);
            }
            return Ok(self
                .infirmerie
                .get_fiche(etablissement_id, eleve_id)
                .await?
                .map(|f| vue_fiche_sante_famille(&f)));
        }
        if parsed == Role::Eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
            return Ok(self
                .infirmerie
                .get_fiche(etablissement_id, eleve_id)
                .await?
                .map(|f| vue_fiche_sante_famille(&f)));
        }
        Err(EntDbError::Forbidden)
    }

    pub async fn upsert_pai_for_actor(
        &self,
        role: &str,
        cmd: UpsertPai,
    ) -> Result<Pai, EntDbError> {
        if !Self::role_can_saisir_infirmerie(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let etab = cmd.etablissement_id;
        let eleve_id = cmd.eleve_id;
        let actif = cmd.actif;
        let planned = plan_upsert_pai(cmd)?;
        let pai = self.infirmerie.upsert_pai(&planned).await?;
        // Aligne le flag fiche santé si une fiche existe déjà.
        if let Some(mut fiche) = self.infirmerie.get_fiche(etab, eleve_id).await? {
            if fiche.pai_actif != actif {
                fiche.pai_actif = actif;
                fiche.updated_by_user_id = actor;
                fiche.updated_at = Utc::now();
                let _ = self.infirmerie.upsert_fiche(&fiche).await?;
            }
        }
        self.events
            .publish(MetierEventDraft {
                etablissement_id: pai.etablissement_id,
                type_name: types::INFIRMERIE_PAI.to_string(),
                aggregate: AggregateKind::Infirmerie,
                aggregate_id: pai.id,
                eleve_id: Some(pai.eleve_id),
                payload: json!({
                    "motif": pai.motif,
                    "actif": pai.actif,
                    "date_debut": pai.date_debut.to_string(),
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(pai)
    }

    pub async fn get_pai_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<Pai>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_saisir_infirmerie();
        if staff {
            return Ok(self.infirmerie.get_pai(etablissement_id, eleve_id).await?);
        }
        if parsed == Role::Parent {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            if lien.is_none() {
                return Err(EntDbError::Forbidden);
            }
            return Ok(self
                .infirmerie
                .get_pai(etablissement_id, eleve_id)
                .await?
                .filter(|p| p.actif)
                .map(|p| vue_pai_famille(&p)));
        }
        if parsed == Role::Eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
            return Ok(self
                .infirmerie
                .get_pai(etablissement_id, eleve_id)
                .await?
                .filter(|p| p.actif)
                .map(|p| vue_pai_famille(&p)));
        }
        Err(EntDbError::Forbidden)
    }

    pub async fn signer_pai_famille_for_actor(
        &self,
        role: &str,
        cmd: SignerPaiFamille,
    ) -> Result<Pai, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if parsed != Role::Parent {
            return Err(EntDbError::Forbidden);
        }
        let lien = self
            .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
            .await?;
        if lien.is_none() {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .infirmerie
            .get_pai(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let updated = plan_signer_pai_famille(&current, &cmd)?;
        let pai = self.infirmerie.update_pai_signature(&updated).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: pai.etablissement_id,
                type_name: types::INFIRMERIE_PAI_SIGNE.to_string(),
                aggregate: AggregateKind::Infirmerie,
                aggregate_id: pai.id,
                eleve_id: Some(pai.eleve_id),
                payload: json!({
                    "motif": pai.motif,
                    "signe_famille": true,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(vue_pai_famille(&pai))
    }

    pub async fn attacher_ordonnance_pai_for_actor(
        &self,
        role: &str,
        cmd: AttacherOrdonnancePai,
    ) -> Result<PaiPiece, EntDbError> {
        if !Self::role_can_saisir_infirmerie(role) {
            return Err(EntDbError::Forbidden);
        }
        let pai = self
            .infirmerie
            .get_pai(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let mut piece = plan_attacher_ordonnance_pai(&pai, cmd)?;
        let ext = piece
            .nom
            .rsplit('.')
            .next()
            .filter(|e| e.len() <= 8)
            .unwrap_or("bin");
        let key = ObjectStore::key_for(piece.etablissement_id, "pai", piece.id, ext);
        self.objects.put(&key, &piece.contenu)?;
        piece.storage_key = Some(key.clone());
        // Contenu hors base — récupéré via object store au téléchargement.
        piece.contenu = Vec::new();
        self.infirmerie.insert_pai_piece(&piece).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: piece.etablissement_id,
                type_name: types::INFIRMERIE_PAI_PIECE.to_string(),
                aggregate: AggregateKind::Infirmerie,
                aggregate_id: piece.id,
                eleve_id: Some(piece.eleve_id),
                payload: json!({
                    "pai_id": piece.pai_id,
                    "nom": piece.nom,
                    "mime_type": piece.mime_type,
                    "taille_octets": piece.taille_octets,
                    "storage_key": key,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: piece.etablissement_id,
                type_name: types::OBJECT_STORED.to_string(),
                aggregate: AggregateKind::Object,
                aggregate_id: piece.id,
                eleve_id: Some(piece.eleve_id),
                payload: json!({
                    "namespace": "pai",
                    "storage_key": key,
                    "taille_octets": piece.taille_octets,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(piece)
    }

    pub async fn list_pai_pieces_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<PaiPieceMeta>, EntDbError> {
        let pai = self
            .get_pai_for_actor(etablissement_id, actor_user_id, role, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let pieces = self
            .infirmerie
            .list_pai_pieces(etablissement_id, pai.id)
            .await?;
        Ok(pieces.iter().map(PaiPieceMeta::from).collect())
    }

    pub async fn telecharger_pai_piece_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<PaiPiece, EntDbError> {
        let mut piece = self
            .infirmerie
            .get_pai_piece(etablissement_id, piece_id)
            .await?
            .ok_or(DomainError::PaiPieceIntrouvable)?;
        // Réutilise le contrôle d'accès get_pai (staff / parent lié + actif).
        let _ = self
            .get_pai_for_actor(etablissement_id, actor_user_id, role, piece.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if let Some(ref key) = piece.storage_key {
            piece.contenu = self.objects.get(key)?;
        }
        Ok(piece)
    }

    /// PDF résumé santé — staff infirmerie / direction ; parent lié (sans note).
    pub async fn resume_fiche_sante_pdf_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<u8>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_saisir_infirmerie();
        if !staff {
            if parsed == Role::Eleve {
                let linked = self
                    .eleve_comptes
                    .eleve_id_for_user(etablissement_id, actor_user_id)
                    .await?;
                if linked != Some(eleve_id) {
                    return Err(EntDbError::Forbidden);
                }
            } else if parsed == Role::Parent {
                let lien = self
                    .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                    .await?;
                if lien.is_none() {
                    return Err(EntDbError::Forbidden);
                }
            } else {
                return Err(EntDbError::Forbidden);
            }
        }
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let fiche = self
            .infirmerie
            .get_fiche(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let resume = plan_resume_fiche_sante(
            &fiche,
            GenererResumeFicheSante {
                eleve_nom: eleve.nom.clone(),
                eleve_prenom: eleve.prenom.clone(),
                classe: eleve.classe.clone(),
                date_document: today_utc(),
            },
        )?;
        let bytes = render_resume_fiche_sante_pdf(&resume);
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::INFIRMERIE_FICHE_SANTE_PDF.to_string(),
                aggregate: AggregateKind::Infirmerie,
                aggregate_id: fiche.id,
                eleve_id: Some(eleve_id),
                payload: json!({
                    "pai_actif": resume.pai_actif,
                    "octets": bytes.len(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(bytes)
    }

    pub fn role_can_saisir_psy(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_saisir_psy())
            .unwrap_or(false)
    }

    pub async fn creer_entretien_psy_for_actor(
        &self,
        role: &str,
        cmd: CreerEntretienPsy,
    ) -> Result<EntretienPsy, EntDbError> {
        if !Self::role_can_saisir_psy(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let entretien = plan_creer_entretien_psy(cmd)?;
        self.psy.insert(&entretien).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: entretien.etablissement_id,
                type_name: types::PSY_ENTRETIEN.to_string(),
                aggregate: AggregateKind::Psy,
                aggregate_id: entretien.id,
                eleve_id: Some(entretien.eleve_id),
                payload: json!({
                    "kind": entretien.kind.as_str(),
                    "jour": entretien.jour.to_string(),
                    "duree_minutes": entretien.duree_minutes,
                    "signale_famille": entretien.signale_famille,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(entretien)
    }

    pub async fn list_entretiens_psy_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<EntretienPsy>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff_psy = parsed.can_saisir_psy();
        let is_parent = parsed == Role::Parent;

        let mut list = if let Some(eid) = eleve_id {
            if staff_psy {
                // ok
            } else if is_parent {
                let lien = self
                    .lien_parent_eleve(etablissement_id, actor_user_id, eid)
                    .await?;
                if lien.is_none() {
                    return Err(EntDbError::Forbidden);
                }
            } else {
                // CPE / prof / infirmière : pas d'accès aux notes psy
                return Err(EntDbError::Forbidden);
            }
            self.psy.list_eleve(etablissement_id, eid, 50).await?
        } else if let Some(j) = jour {
            if !staff_psy {
                return Err(EntDbError::Forbidden);
            }
            self.psy.list_jour(etablissement_id, j, 100).await?
        } else if staff_psy {
            self.psy
                .list_jour(etablissement_id, today_utc(), 100)
                .await?
        } else {
            return Err(EntDbError::Forbidden);
        };

        if is_parent {
            list = list
                .into_iter()
                .filter(|e| e.signale_famille)
                .map(|e| vue_entretien_psy_famille(&e))
                .collect();
        }
        Ok(list)
    }

    pub async fn upsert_dossier_psy_for_actor(
        &self,
        role: &str,
        cmd: UpsertDossierPsy,
    ) -> Result<DossierPsy, EntDbError> {
        if !Self::role_can_saisir_psy(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let planned = plan_upsert_dossier_psy(cmd)?;
        let dossier = self.psy.upsert_dossier(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: dossier.etablissement_id,
                type_name: types::PSY_DOSSIER.to_string(),
                aggregate: AggregateKind::Psy,
                aggregate_id: dossier.id,
                eleve_id: Some(dossier.eleve_id),
                payload: json!({
                    "suivi_actif": dossier.suivi_actif,
                    "signale_famille": dossier.signale_famille,
                    "axes_suivi": dossier.axes_suivi,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(dossier)
    }

    pub async fn get_dossier_psy_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<DossierPsy>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if parsed.can_saisir_psy() {
            return Ok(self.psy.get_dossier(etablissement_id, eleve_id).await?);
        }
        if parsed == Role::Parent {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            if lien.is_none() {
                return Err(EntDbError::Forbidden);
            }
            return Ok(self
                .psy
                .get_dossier(etablissement_id, eleve_id)
                .await?
                .filter(|d| d.signale_famille)
                .map(|d| vue_dossier_famille(&d)));
        }
        Err(EntDbError::Forbidden)
    }

    async fn build_courrier_orientation_psy_pdf(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<(Vec<u8>, String, String), EntDbError> {
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let dossier = self
            .psy
            .get_dossier(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let courrier = plan_courrier_orientation_psy(
            &dossier,
            GenererCourrierOrientationPsy {
                eleve_nom: eleve.nom.clone(),
                eleve_prenom: eleve.prenom.clone(),
                classe: eleve.classe.clone(),
                date_courrier: today_utc(),
            },
        )?;
        let bytes = render_courrier_orientation_pdf(&courrier);
        let sujet = format!(
            "Courrier d'orientation — {} {}",
            eleve.prenom, eleve.nom
        );
        let corps = format!(
            "Bonjour,\n\nVeuillez trouver ci-joint le courrier d'orientation \
             concernant {} {} ({}).\n\nCet envoi est généré par l'établissement.",
            eleve.prenom, eleve.nom, eleve.classe.as_deref().unwrap_or("classe")
        );
        Ok((bytes, sujet, corps))
    }

    /// PDF courrier orientation — staff psy toujours ; parent si dossier signalé.
    pub async fn courrier_orientation_psy_pdf_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<u8>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff_psy = parsed.can_saisir_psy();
        if !staff_psy {
            if parsed != Role::Parent {
                return Err(EntDbError::Forbidden);
            }
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            if lien.is_none() {
                return Err(EntDbError::Forbidden);
            }
        }
        let dossier = self
            .psy
            .get_dossier(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !staff_psy && !dossier.signale_famille {
            return Err(EntDbError::Forbidden);
        }
        let (bytes, _, _) = self
            .build_courrier_orientation_psy_pdf(etablissement_id, eleve_id)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::PSY_COURRIER.to_string(),
                aggregate: AggregateKind::Psy,
                aggregate_id: dossier.id,
                eleve_id: Some(eleve_id),
                payload: json!({
                    "kind": "orientation",
                    "canal": "pdf",
                    "octets": bytes.len(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(bytes)
    }

    /// Envoie le courrier d'orientation PDF aux parents (SMTP dry-run / live).
    pub async fn envoyer_courrier_orientation_psy_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<usize, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_saisir_psy() {
            return Err(EntDbError::Forbidden);
        }
        let dossier = self
            .psy
            .get_dossier(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let (bytes, sujet, corps) = self
            .build_courrier_orientation_psy_pdf(etablissement_id, eleve_id)
            .await?;
        let n = self
            .notify_parents_mail(
                etablissement_id,
                eleve_id,
                "courrier_psy",
                &sujet,
                &corps,
                "psy_dossier",
                dossier.id,
                Some(actor_user_id),
                vec![MailAttachment {
                    filename: format!("courrier-orientation-{eleve_id}.pdf"),
                    mime: "application/pdf".into(),
                    bytes,
                }],
            )
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::PSY_COURRIER.to_string(),
                aggregate: AggregateKind::Psy,
                aggregate_id: dossier.id,
                eleve_id: Some(eleve_id),
                payload: json!({
                    "kind": "orientation",
                    "canal": "smtp",
                    "mails": n,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(n)
    }

    pub async fn creer_rdv_psy_for_actor(
        &self,
        role: &str,
        cmd: CreerRdvPsy,
    ) -> Result<RdvPsy, EntDbError> {
        if !Self::role_can_saisir_psy(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let rdv = plan_creer_rdv_psy(cmd)?;
        self.psy.insert_rdv(&rdv).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: rdv.etablissement_id,
                type_name: types::PSY_RDV.to_string(),
                aggregate: AggregateKind::Psy,
                aggregate_id: rdv.id,
                eleve_id: Some(rdv.eleve_id),
                payload: json!({
                    "kind": rdv.kind.as_str(),
                    "jour": rdv.jour.to_string(),
                    "heure_debut": rdv.heure_debut.format("%H:%M").to_string(),
                    "heure_fin": rdv.heure_fin.format("%H:%M").to_string(),
                    "statut": rdv.statut.as_str(),
                    "signale_famille": rdv.signale_famille,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(rdv)
    }

    pub async fn changer_statut_rdv_psy_for_actor(
        &self,
        role: &str,
        cmd: ChangerStatutRdvPsy,
    ) -> Result<RdvPsy, EntDbError> {
        if !Self::role_can_saisir_psy(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .psy
            .get_rdv(cmd.etablissement_id, cmd.rdv_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let updated = plan_changer_statut_rdv_psy(&current, &cmd)?;
        self.psy.update_rdv_statut(&updated).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: updated.etablissement_id,
                type_name: types::PSY_RDV.to_string(),
                aggregate: AggregateKind::Psy,
                aggregate_id: updated.id,
                eleve_id: Some(updated.eleve_id),
                payload: json!({
                    "statut": updated.statut.as_str(),
                    "action": "statut",
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(updated)
    }

    pub async fn list_rdvs_psy_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<RdvPsy>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff_psy = parsed.can_saisir_psy();
        let is_parent = parsed == Role::Parent;

        let mut list = if let Some(eid) = eleve_id {
            if staff_psy {
                // ok
            } else if is_parent {
                let lien = self
                    .lien_parent_eleve(etablissement_id, actor_user_id, eid)
                    .await?;
                if lien.is_none() {
                    return Err(EntDbError::Forbidden);
                }
            } else {
                return Err(EntDbError::Forbidden);
            }
            self.psy.list_rdv_eleve(etablissement_id, eid, 50).await?
        } else if let Some(j) = jour {
            if !staff_psy {
                return Err(EntDbError::Forbidden);
            }
            self.psy.list_rdv_jour(etablissement_id, j, 100).await?
        } else if staff_psy {
            self.psy
                .list_rdv_jour(etablissement_id, today_utc(), 100)
                .await?
        } else {
            return Err(EntDbError::Forbidden);
        };

        if is_parent {
            list = list
                .into_iter()
                .filter(|r| r.signale_famille)
                .map(|r| vue_rdv_famille(&r))
                .collect();
        }
        Ok(list)
    }

    pub async fn creer_carnet_entree(
        &self,
        cmd: CreerCarnetEntree,
    ) -> Result<CarnetEntree, EntDbError> {
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        let mut entree = plan_creer_carnet_entree(cmd)?;
        self.carnet.insert(&entree).await?;

        // Notif famille via messagerie si entrée visible.
        if entree.visible_famille {
            let (fil, msg) = plan_ouvrir_fil(OuvrirFil {
                etablissement_id: entree.etablissement_id,
                eleve_id: entree.eleve_id,
                sujet: sujet_notif_carnet(&entree.titre),
                premier_message: corps_notif_carnet(
                    entree.categorie.as_str(),
                    &entree.titre,
                    &entree.corps,
                ),
                actor_user_id: actor,
            })?;
            self.messages.insert_fil(&fil).await?;
            self.messages.insert_message(&msg).await?;
            if self
                .carnet
                .set_fil_notif(entree.etablissement_id, entree.id, fil.id)
                .await?
            {
                entree.fil_notif_id = Some(fil.id);
                self.events
                    .publish(MetierEventDraft {
                        etablissement_id: entree.etablissement_id,
                        type_name: types::MSG_FIL_OUVERT.to_string(),
                        aggregate: AggregateKind::MsgFil,
                        aggregate_id: fil.id,
                        eleve_id: Some(entree.eleve_id),
                        payload: json!({
                            "sujet": fil.sujet,
                            "source": "carnet",
                            "carnet_entree_id": entree.id,
                        }),
                        actor_user_id: Some(actor.to_string()),
                    })
                    .await?;
            }

            // SMTP + SMS famille (dry-run lab / live si ENT_MAIL_LIVE / ENT_SMS_LIVE).
            let sujet = sujet_notif_carnet(&entree.titre);
            let corps = corps_notif_carnet(
                entree.categorie.as_str(),
                &entree.titre,
                &entree.corps,
            );
            let _ = self
                .notify_parents_mail(
                    entree.etablissement_id,
                    entree.eleve_id,
                    "carnet",
                    &sujet,
                    &corps,
                    "carnet",
                    entree.id,
                    Some(actor),
                    vec![],
                )
                .await;
            let sms_body = format!(
                "Carnet de liaison: {} — {}. Merci de consulter et signer sur le portail famille.",
                entree.categorie.as_str(),
                entree.titre
            );
            let _ = self
                .notify_parents_sms(
                    entree.etablissement_id,
                    entree.eleve_id,
                    "carnet",
                    &sms_body,
                    "carnet",
                    entree.id,
                    Some(actor),
                )
                .await;
            let _ = self
                .notify_parents_push(
                    entree.etablissement_id,
                    entree.eleve_id,
                    "carnet",
                    &sujet,
                    &entree.titre,
                    "carnet",
                    entree.id,
                    Some(actor),
                )
                .await;
        }

        self.events
            .publish(MetierEventDraft {
                etablissement_id: entree.etablissement_id,
                type_name: types::ELEVE_CARNET_ENTREE.to_string(),
                aggregate: AggregateKind::Carnet,
                aggregate_id: entree.id,
                eleve_id: Some(entree.eleve_id),
                payload: json!({
                    "categorie": entree.categorie.as_str(),
                    "visible_famille": entree.visible_famille,
                    "titre": entree.titre,
                    "fil_notif_id": entree.fil_notif_id,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(entree)
    }

    pub fn role_can_ecrire_cahier_texte(role: &str) -> bool {
        Role::parse(role)
            .map(|r| {
                matches!(
                    r,
                    Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Prof
                )
            })
            .unwrap_or(false)
    }

    pub async fn creer_cahier_texte_for_actor(
        &self,
        role: &str,
        cmd: CreerCahierTexte,
    ) -> Result<CahierTexteEntree, EntDbError> {
        if !Self::role_can_ecrire_cahier_texte(role) {
            return Err(EntDbError::Forbidden);
        }
        let entree = plan_creer_cahier_texte(cmd)?;
        self.cahier_texte.insert(&entree).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: entree.etablissement_id,
                type_name: types::CAHIER_TEXTE.to_string(),
                aggregate: AggregateKind::CahierTexte,
                aggregate_id: entree.id,
                eleve_id: None,
                payload: json!({
                    "classe": entree.classe,
                    "matiere": entree.matiere_libelle,
                    "titre": entree.titre,
                    "pour_le": entree.pour_le.to_string(),
                }),
                actor_user_id: Some(entree.created_by_user_id.to_string()),
            })
            .await?;
        Ok(entree)
    }

    /// Staff : filtre classe. Parent / élève : classe de l'élève (accès vérifié).
    pub async fn list_cahier_texte_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        classe: Option<String>,
        eleve_id: Option<Uuid>,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<CahierTexteEntree>, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Err(EntDbError::Forbidden);
        };
        let classe_cible = if Self::role_can_ecrire_cahier_texte(role) {
            classe
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .ok_or(EntDbError::Forbidden)?
        } else if parsed == Role::Parent || parsed == Role::Eleve {
            let eid = if let Some(eid) = eleve_id {
                eid
            } else if parsed == Role::Eleve {
                self.eleve_comptes
                    .eleve_id_for_user(etablissement_id, actor_user_id)
                    .await?
                    .ok_or(EntDbError::Forbidden)?
            } else {
                return Err(EntDbError::Forbidden);
            };
            let eleve = self
                .get_eleve_for_actor(etablissement_id, actor_user_id, role, eid)
                .await?
                .ok_or(EntDbError::Forbidden)?;
            eleve.classe.ok_or(DomainError::CahierTexteInvalide)?
        } else {
            return Err(EntDbError::Forbidden);
        };
        Ok(self
            .cahier_texte
            .list_classe(etablissement_id, &classe_cible, from, to)
            .await?)
    }

    pub async fn creer_carnet_entree_for_actor(
        &self,
        role: &str,
        cmd: CreerCarnetEntree,
    ) -> Result<CarnetEntree, EntDbError> {
        if !Self::role_can_ecrire_carnet(role) {
            return Err(EntDbError::Forbidden);
        }
        self.creer_carnet_entree(cmd).await
    }

    pub async fn list_carnet_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<CarnetEntree>, EntDbError> {
        let allowed = self
            .actor_can_read_absences(etablissement_id, actor_user_id, role, eleve_id)
            .await?;
        if !allowed {
            return Err(EntDbError::Forbidden);
        }
        let famille_only = Role::parse(role)
            .map(|r| matches!(r, Role::Parent | Role::Eleve))
            .unwrap_or(false);
        Ok(self
            .carnet
            .list_for_eleve(etablissement_id, eleve_id, famille_only)
            .await?)
    }

    pub async fn signer_carnet_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        entree_id: Uuid,
    ) -> Result<CarnetEntree, EntDbError> {
        let current = self
            .carnet
            .get(etablissement_id, entree_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let can = self
            .actor_can_read_absences(
                etablissement_id,
                actor_user_id,
                role,
                current.eleve_id,
            )
            .await?;
        if !can || !Role::parse(role).map(|r| r == Role::Parent).unwrap_or(false) {
            return Err(EntDbError::Forbidden);
        }
        let signed = plan_signer_carnet_entree(
            &current,
            SignerCarnetEntree {
                etablissement_id,
                entree_id,
                actor_user_id,
            },
        )?;
        self.carnet.update_signature(&signed).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: signed.etablissement_id,
                type_name: types::ELEVE_CARNET_SIGNEE.to_string(),
                aggregate: AggregateKind::Carnet,
                aggregate_id: signed.id,
                eleve_id: Some(signed.eleve_id),
                payload: json!({ "signe_par": actor_user_id.to_string() }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(signed)
    }

    pub fn role_can_messagerie_staff(role: &str) -> bool {
        Role::parse(role)
            .map(|r| {
                matches!(
                    r,
                    Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Prof
                )
            })
            .unwrap_or(false)
    }

    pub async fn actor_can_access_msg_fil(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(false);
        };
        if Self::role_can_messagerie_staff(role) {
            return Ok(true);
        }
        if parsed == Role::Parent {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            return Ok(lien.is_some());
        }
        if parsed == Role::Eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            return Ok(linked == Some(eleve_id));
        }
        Ok(false)
    }

    pub async fn ouvrir_fil_for_actor(
        &self,
        role: &str,
        cmd: OuvrirFil,
    ) -> Result<(MsgFil, Message), EntDbError> {
        let can = self
            .actor_can_access_msg_fil(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                cmd.eleve_id,
            )
            .await?;
        // Parent peut aussi ouvrir un fil (écrit à l'école).
        if !can {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let (fil, msg) = plan_ouvrir_fil(cmd)?;
        self.messages.insert_fil(&fil).await?;
        self.messages.insert_message(&msg).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: fil.etablissement_id,
                type_name: types::MSG_FIL_OUVERT.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: fil.id,
                eleve_id: Some(fil.eleve_id),
                payload: json!({
                    "sujet": fil.sujet,
                    "message_id": msg.id,
                }),
                actor_user_id: Some(fil.created_by_user_id.to_string()),
            })
            .await?;
        let _ = self
            .notify_parents_push(
                fil.etablissement_id,
                fil.eleve_id,
                "messagerie",
                &fil.sujet,
                "Nouveau message — ouvrez le portail famille.",
                "msg_fil",
                fil.id,
                Some(fil.created_by_user_id),
            )
            .await;
        Ok((fil, msg))
    }

    pub async fn repondre_fil_for_actor(
        &self,
        role: &str,
        cmd: RepondreFil,
    ) -> Result<Message, EntDbError> {
        let fil = self
            .messages
            .get_fil(cmd.etablissement_id, cmd.fil_id)
            .await?
            .ok_or(DomainError::MsgFilIntrouvable)?;
        let can = self
            .actor_can_access_msg_fil(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                fil.eleve_id,
            )
            .await?;
        if !can {
            return Err(EntDbError::Forbidden);
        }
        let (msg, at) = plan_repondre_fil(&fil, cmd)?;
        self.messages.insert_message(&msg).await?;
        self.messages
            .touch_fil_last(msg.etablissement_id, msg.fil_id, at)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: msg.etablissement_id,
                type_name: types::MSG_ENVOYE.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: msg.fil_id,
                eleve_id: Some(fil.eleve_id),
                payload: json!({
                    "message_id": msg.id,
                    "auteur_user_id": msg.auteur_user_id,
                }),
                actor_user_id: Some(msg.auteur_user_id.to_string()),
            })
            .await?;
        // Push famille si staff répond (pas si le parent se répond à lui-même).
        let _ = self
            .notify_parents_push(
                fil.etablissement_id,
                fil.eleve_id,
                "messagerie_reponse",
                &fil.sujet,
                "Nouvelle réponse — ouvrez la messagerie.",
                "msg_message",
                msg.id,
                Some(msg.auteur_user_id),
            )
            .await;
        Ok(msg)
    }

    pub async fn list_fils_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        include_archives: bool,
        dossier: Option<&str>,
    ) -> Result<Vec<MsgFil>, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Err(EntDbError::Forbidden);
        };
        if Self::role_can_messagerie_staff(role) {
            return if let Some(eid) = eleve_id {
                Ok(self.messages.list_fils_eleve(etablissement_id, eid).await?)
            } else {
                Ok(self
                    .messages
                    .list_fils_etab(etablissement_id, 50, include_archives, dossier)
                    .await?)
            };
        }
        if parsed == Role::Parent {
            let lies = self.eleves_lies(etablissement_id, actor_user_id).await?;
            if let Some(eid) = eleve_id {
                if !lies.contains(&eid) {
                    return Err(EntDbError::Forbidden);
                }
                return Ok(self.messages.list_fils_eleve(etablissement_id, eid).await?);
            }
            return Ok(self
                .messages
                .list_fils_eleves(etablissement_id, &lies)
                .await?);
        }
        if parsed == Role::Eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?
                .ok_or(EntDbError::Forbidden)?;
            if let Some(eid) = eleve_id {
                if eid != linked {
                    return Err(EntDbError::Forbidden);
                }
            }
            return Ok(self
                .messages
                .list_fils_eleve(etablissement_id, linked)
                .await?);
        }
        Err(EntDbError::Forbidden)
    }

    pub async fn list_messages_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        fil_id: Uuid,
    ) -> Result<(MsgFil, Vec<MessageLue>), EntDbError> {
        let fil = self
            .messages
            .get_fil(etablissement_id, fil_id)
            .await?
            .ok_or(DomainError::MsgFilIntrouvable)?;
        let can = self
            .actor_can_access_msg_fil(etablissement_id, actor_user_id, role, fil.eleve_id)
            .await?;
        if !can {
            return Err(EntDbError::Forbidden);
        }
        let msgs = self.messages.list_messages(etablissement_id, fil_id).await?;
        let nouveaux = plan_accuser_lecture(
            &fil,
            &msgs,
            &AccuserLecture {
                etablissement_id,
                fil_id,
                actor_user_id,
            },
        )?;
        let inserted = self.messages.upsert_accuses(&nouveaux).await?;
        if inserted > 0 {
            self.events
                .publish(MetierEventDraft {
                    etablissement_id,
                    type_name: types::MSG_ACCUSE.to_string(),
                    aggregate: AggregateKind::MsgFil,
                    aggregate_id: fil_id,
                    eleve_id: Some(fil.eleve_id),
                    payload: json!({
                        "lecteur_user_id": actor_user_id,
                        "nb_nouveaux": inserted,
                    }),
                    actor_user_id: Some(actor_user_id.to_string()),
                })
                .await?;
        }
        let accuses = self
            .messages
            .list_accuses_fil(etablissement_id, fil_id)
            .await?;
        let pieces = self
            .messages
            .list_pieces_meta_fil(etablissement_id, fil_id)
            .await?;
        let lues = enrichir_messages_lues(&msgs, &accuses, &pieces, actor_user_id);
        Ok((fil, lues))
    }

    pub async fn attacher_piece_for_actor(
        &self,
        role: &str,
        cmd: AttacherPiece,
    ) -> Result<MsgPieceMeta, EntDbError> {
        let msg = self
            .messages
            .get_message(cmd.etablissement_id, cmd.message_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let fil = self
            .messages
            .get_fil(cmd.etablissement_id, msg.fil_id)
            .await?
            .ok_or(DomainError::MsgFilIntrouvable)?;
        let can = self
            .actor_can_access_msg_fil(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                fil.eleve_id,
            )
            .await?;
        if !can {
            return Err(EntDbError::Forbidden);
        }
        let piece = plan_attacher_piece(&msg, cmd)?;
        self.messages.insert_piece(&piece).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: piece.etablissement_id,
                type_name: types::MSG_PIECE.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: piece.fil_id,
                eleve_id: Some(fil.eleve_id),
                payload: json!({
                    "piece_id": piece.id,
                    "message_id": piece.message_id,
                    "nom": piece.nom,
                    "taille_octets": piece.taille_octets,
                }),
                actor_user_id: Some(piece.created_by_user_id.to_string()),
            })
            .await?;
        Ok(MsgPieceMeta::from(&piece))
    }

    pub async fn telecharger_piece_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<MsgPiece, EntDbError> {
        let piece = self
            .messages
            .get_piece(etablissement_id, piece_id)
            .await?
            .ok_or(DomainError::MsgPieceIntrouvable)?;
        let fil = self
            .messages
            .get_fil(etablissement_id, piece.fil_id)
            .await?
            .ok_or(DomainError::MsgFilIntrouvable)?;
        let can = self
            .actor_can_access_msg_fil(etablissement_id, actor_user_id, role, fil.eleve_id)
            .await?;
        if !can {
            return Err(EntDbError::Forbidden);
        }
        Ok(piece)
    }

    pub async fn creer_msg_modele_for_actor(
        &self,
        role: &str,
        cmd: CreerMsgModele,
    ) -> Result<MsgModele, EntDbError> {
        if !Self::role_can_messagerie_staff(role) {
            return Err(EntDbError::Forbidden);
        }
        let modele = plan_creer_msg_modele(cmd)?;
        self.messages.insert_modele(&modele).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: modele.etablissement_id,
                type_name: types::MSG_MODELE.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: modele.id,
                eleve_id: None,
                payload: json!({
                    "titre": modele.titre,
                    "sujet": modele.sujet,
                }),
                actor_user_id: Some(modele.created_by_user_id.to_string()),
            })
            .await?;
        Ok(modele)
    }

    pub async fn list_msg_modeles_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actifs_seulement: bool,
    ) -> Result<Vec<MsgModele>, EntDbError> {
        if !Self::role_can_messagerie_staff(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .messages
            .list_modeles(etablissement_id, actifs_seulement)
            .await?)
    }

    /// Ouvre un fil depuis un modèle staff — placeholders `{prenom|nom|classe}`.
    pub async fn ouvrir_fil_depuis_modele_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        modele_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<(MsgFil, Message), EntDbError> {
        if !Self::role_can_messagerie_staff(role) {
            return Err(EntDbError::Forbidden);
        }
        let modele = self
            .messages
            .get_modele(etablissement_id, modele_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !modele.actif {
            return Err(DomainError::MsgModeleInvalide.into());
        }
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let (sujet, corps) = appliquer_modele(
            &modele,
            &ContexteModele {
                prenom: eleve.prenom.clone(),
                nom: eleve.nom.clone(),
                classe: eleve.classe.clone().unwrap_or_default(),
            },
        );
        let (mut fil, msg) = plan_ouvrir_fil(OuvrirFil {
            etablissement_id,
            eleve_id,
            sujet,
            premier_message: corps,
            actor_user_id,
        })?;
        fil.modele_id = Some(modele.id);
        self.messages.insert_fil(&fil).await?;
        self.messages.insert_message(&msg).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: fil.etablissement_id,
                type_name: types::MSG_FIL_OUVERT.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: fil.id,
                eleve_id: Some(fil.eleve_id),
                payload: json!({
                    "sujet": fil.sujet,
                    "message_id": msg.id,
                    "modele_id": modele.id,
                }),
                actor_user_id: Some(fil.created_by_user_id.to_string()),
            })
            .await?;
        Ok((fil, msg))
    }

    pub async fn archiver_fil_for_actor(
        &self,
        role: &str,
        cmd: ArchiverFil,
    ) -> Result<MsgFil, EntDbError> {
        if !Self::role_can_messagerie_staff(role) {
            return Err(EntDbError::Forbidden);
        }
        let fil = self
            .messages
            .get_fil(cmd.etablissement_id, cmd.fil_id)
            .await?
            .ok_or(DomainError::MsgFilIntrouvable)?;
        let archived = plan_archiver_fil(&fil, &cmd)?;
        if !fil.archive {
            self.messages
                .set_fil_archive(
                    archived.etablissement_id,
                    archived.id,
                    true,
                    Some(Utc::now()),
                )
                .await?;
            self.events
                .publish(MetierEventDraft {
                    etablissement_id: archived.etablissement_id,
                    type_name: types::MSG_FIL_ARCHIVE.to_string(),
                    aggregate: AggregateKind::MsgFil,
                    aggregate_id: archived.id,
                    eleve_id: Some(archived.eleve_id),
                    payload: json!({ "archive": true }),
                    actor_user_id: Some(cmd.actor_user_id.to_string()),
                })
                .await?;
        }
        Ok(archived)
    }

    pub async fn classer_fil_for_actor(
        &self,
        role: &str,
        cmd: ClasserFil,
    ) -> Result<MsgFil, EntDbError> {
        if !Self::role_can_messagerie_staff(role) {
            return Err(EntDbError::Forbidden);
        }
        let fil = self
            .messages
            .get_fil(cmd.etablissement_id, cmd.fil_id)
            .await?
            .ok_or(DomainError::MsgFilIntrouvable)?;
        let actor = cmd.actor_user_id;
        let classed = plan_classer_fil(&fil, &cmd)?;
        self.messages
            .set_fil_boite(
                classed.etablissement_id,
                classed.id,
                classed.priorite,
                classed.dossier.as_deref(),
            )
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: classed.etablissement_id,
                type_name: types::MSG_FIL_CLASSER.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: classed.id,
                eleve_id: Some(classed.eleve_id),
                payload: json!({
                    "priorite": classed.priorite.as_str(),
                    "dossier": classed.dossier,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(classed)
    }

    pub fn role_can_manage_facturation(role: &str) -> bool {
        Role::parse(role)
            .map(|r| r.can_manage_facturation())
            .unwrap_or(false)
    }

    async fn foyer_ids_parent(
        &self,
        etablissement_id: Uuid,
        parent_user_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let liens = self
            .foyers
            .liens_for_parent(etablissement_id, parent_user_id)
            .await?;
        let mut ids: Vec<Uuid> = liens.into_iter().filter_map(|l| l.foyer_id).collect();
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    pub async fn actor_can_access_facture(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        facture: &Facture,
    ) -> Result<bool, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(false);
        };
        if Self::role_can_manage_facturation(role) {
            return Ok(true);
        }
        if parsed == Role::Parent {
            // Parents ne voient que les factures émises / soldées de leurs foyers.
            if facture.statut == FactureStatut::Brouillon {
                return Ok(false);
            }
            let foyers = self.foyer_ids_parent(etablissement_id, actor_user_id).await?;
            return Ok(foyers.contains(&facture.foyer_id));
        }
        Ok(false)
    }

    pub async fn creer_facture_for_actor(
        &self,
        role: &str,
        cmd: CreerFacture,
    ) -> Result<(Facture, Vec<FactureLigne>), EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        if !self
            .facturation
            .foyer_exists(cmd.etablissement_id, cmd.foyer_id)
            .await?
        {
            return Err(DomainError::NotFound.into());
        }
        let (facture, lignes) = plan_creer_facture(cmd)?;
        self.facturation
            .insert_facture(&facture, &lignes)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: facture.etablissement_id,
                type_name: types::FAC_FACTURE_CREEE.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: facture.id,
                eleve_id: None,
                payload: json!({
                    "foyer_id": facture.foyer_id,
                    "numero": facture.numero,
                    "total_centimes": facture.total_centimes,
                    "lignes": lignes.len(),
                }),
                actor_user_id: facture.created_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        Ok((facture, lignes))
    }

    pub async fn emettre_facture_for_actor(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Facture, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .facturation
            .get_facture(etablissement_id, facture_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let emise = plan_emettre_facture(&current)?;
        self.facturation.update_facture_emise(&emise).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: emise.etablissement_id,
                type_name: types::FAC_FACTURE_EMISE.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: emise.id,
                eleve_id: None,
                payload: json!({
                    "foyer_id": emise.foyer_id,
                    "numero": emise.numero,
                    "total_centimes": emise.total_centimes,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(emise)
    }

    pub async fn list_factures_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Vec<Facture>, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Err(EntDbError::Forbidden);
        };
        if Self::role_can_manage_facturation(role) {
            return Ok(self
                .facturation
                .list_factures_etab(etablissement_id, 80)
                .await?);
        }
        if parsed == Role::Parent {
            let foyers = self.foyer_ids_parent(etablissement_id, actor_user_id).await?;
            return Ok(self
                .facturation
                .list_factures_foyers(etablissement_id, &foyers, true)
                .await?);
        }
        Err(EntDbError::Forbidden)
    }

    pub async fn get_facture_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        facture_id: Uuid,
    ) -> Result<
        (
            Facture,
            Vec<FactureLigne>,
            Vec<Encaissement>,
            i64,
            Vec<EcheanceVue>,
            Vec<Quittance>,
            Vec<PrelevementSepa>,
        ),
        EntDbError,
    > {
        let facture = self
            .facturation
            .get_facture(etablissement_id, facture_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let can = self
            .actor_can_access_facture(etablissement_id, actor_user_id, role, &facture)
            .await?;
        if !can {
            return Err(EntDbError::Forbidden);
        }
        let lignes = self
            .facturation
            .list_lignes(etablissement_id, facture_id)
            .await?;
        let encaissements = self
            .facturation
            .list_encaissements(etablissement_id, facture_id)
            .await?;
        let encaisse = self
            .facturation
            .sum_encaissements(etablissement_id, facture_id)
            .await?;
        let echeances = self
            .facturation
            .list_echeances(etablissement_id, facture_id)
            .await?;
        let echeances_vues = couvrir_echeances(&echeances, encaisse, today_utc());
        let quittances = self
            .facturation
            .list_quittances_facture(etablissement_id, facture_id)
            .await?;
        let prelevements = self
            .facturation
            .list_sepa_prelevements_facture(etablissement_id, facture_id)
            .await?;
        Ok((
            facture,
            lignes,
            encaissements,
            encaisse,
            echeances_vues,
            quittances,
            prelevements,
        ))
    }

    pub async fn list_quittances_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Vec<Quittance>, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Err(EntDbError::Forbidden);
        };
        if Self::role_can_manage_facturation(role) {
            // Staff : via détail facture ; listing global = tous foyers non exposé ici.
            return Err(EntDbError::Forbidden);
        }
        if parsed == Role::Parent {
            let foyers = self.foyer_ids_parent(etablissement_id, actor_user_id).await?;
            return Ok(self
                .facturation
                .list_quittances_foyers(etablissement_id, &foyers)
                .await?);
        }
        Err(EntDbError::Forbidden)
    }

    pub async fn get_quittance_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        quittance_id: Uuid,
    ) -> Result<Quittance, EntDbError> {
        let q = self
            .facturation
            .get_quittance(etablissement_id, quittance_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let facture = self
            .facturation
            .get_facture(etablissement_id, q.facture_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let can = self
            .actor_can_access_facture(etablissement_id, actor_user_id, role, &facture)
            .await?;
        if !can {
            return Err(EntDbError::Forbidden);
        }
        Ok(q)
    }

    pub async fn poser_echeancier_for_actor(
        &self,
        role: &str,
        cmd: PoserEcheancier,
    ) -> Result<Vec<Echeance>, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let facture = self
            .facturation
            .get_facture(cmd.etablissement_id, cmd.facture_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let existing = self
            .facturation
            .count_echeances(cmd.etablissement_id, cmd.facture_id)
            .await?;
        if existing > 0 {
            return Err(DomainError::EcheancierDejaPose.into());
        }
        let mut echeances = plan_poser_echeancier(&facture, cmd)?;
        let lignes = self
            .facturation
            .list_lignes(facture.etablissement_id, facture.id)
            .await?;
        let bourse = total_deductions_lignes(&lignes);
        ventiler_bourse_sur_echeances(&mut echeances, bourse)?;
        self.facturation.insert_echeances(&echeances).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: facture.etablissement_id,
                type_name: types::FAC_ECHEANCIER_POSE.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: facture.id,
                eleve_id: None,
                payload: json!({
                    "numero": facture.numero,
                    "nb": echeances.len(),
                    "total_centimes": facture.total_centimes,
                    "bourse_ventilee_centimes": bourse,
                }),
                actor_user_id: None,
            })
            .await?;
        Ok(echeances)
    }

    pub async fn poser_echeancier_parts_for_actor(
        &self,
        etablissement_id: Uuid,
        facture_id: Uuid,
        role: &str,
        dates: Vec<NaiveDate>,
    ) -> Result<Vec<Echeance>, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let facture = self
            .facturation
            .get_facture(etablissement_id, facture_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let existing = self
            .facturation
            .count_echeances(etablissement_id, facture_id)
            .await?;
        if existing > 0 {
            return Err(DomainError::EcheancierDejaPose.into());
        }
        let mut echeances = plan_echeancier_parts_egales(&facture, dates)?;
        let lignes = self
            .facturation
            .list_lignes(etablissement_id, facture_id)
            .await?;
        let bourse = total_deductions_lignes(&lignes);
        ventiler_bourse_sur_echeances(&mut echeances, bourse)?;
        self.facturation.insert_echeances(&echeances).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: facture.etablissement_id,
                type_name: types::FAC_ECHEANCIER_POSE.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: facture.id,
                eleve_id: None,
                payload: json!({
                    "numero": facture.numero,
                    "nb": echeances.len(),
                    "mode": "parts_egales",
                    "bourse_ventilee_centimes": bourse,
                }),
                actor_user_id: None,
            })
            .await?;
        Ok(echeances)
    }

    pub async fn encaisser_facture_for_actor(
        &self,
        role: &str,
        cmd: EncaisserFacture,
    ) -> Result<(Encaissement, Facture, Quittance), EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .facturation
            .get_facture(cmd.etablissement_id, cmd.facture_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let deja = self
            .facturation
            .sum_encaissements(cmd.etablissement_id, cmd.facture_id)
            .await?;
        let ouvert = self
            .facturation
            .sum_prelevements_ouverts(cmd.etablissement_id, cmd.facture_id)
            .await?;
        let engage = deja.saturating_add(ouvert);
        self.encaisser_avec_engage(role, cmd, &current, deja, engage)
            .await
    }

    async fn encaisser_avec_engage(
        &self,
        _role: &str,
        cmd: EncaisserFacture,
        current: &Facture,
        deja: i64,
        engage: i64,
    ) -> Result<(Encaissement, Facture, Quittance), EntDbError> {
        let (enc, facture) = plan_encaisser(current, engage, cmd)?;
        self.facturation
            .insert_encaissement_and_maybe_solde(&enc, &facture)
            .await?;
        let rang = self
            .facturation
            .count_quittances_facture(enc.etablissement_id, enc.facture_id)
            .await?
            + 1;
        let quittance = plan_emettre_quittance(&facture, &enc, rang as i32)?;
        self.facturation.insert_quittance(&quittance).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: enc.etablissement_id,
                type_name: types::FAC_ENCAISSEMENT.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: enc.facture_id,
                eleve_id: None,
                payload: json!({
                    "encaissement_id": enc.id,
                    "montant_centimes": enc.montant_centimes,
                    "moyen": enc.moyen.as_str(),
                    "deja_plus_ce": deja.saturating_add(enc.montant_centimes),
                    "reste": reste_a_payer(
                        facture.total_centimes,
                        deja.saturating_add(enc.montant_centimes)
                    ),
                    "quittance_id": quittance.id,
                    "quittance_numero": quittance.numero,
                }),
                actor_user_id: enc.created_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: quittance.etablissement_id,
                type_name: types::FAC_QUITTANCE_EMISE.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: quittance.facture_id,
                eleve_id: None,
                payload: json!({
                    "quittance_id": quittance.id,
                    "numero": quittance.numero,
                    "montant_centimes": quittance.montant_centimes,
                    "encaissement_id": quittance.encaissement_id,
                }),
                actor_user_id: quittance.created_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        if facture.statut == FactureStatut::Soldee {
            self.events
                .publish(MetierEventDraft {
                    etablissement_id: facture.etablissement_id,
                    type_name: types::FAC_FACTURE_SOLDEE.to_string(),
                    aggregate: AggregateKind::Facture,
                    aggregate_id: facture.id,
                    eleve_id: None,
                    payload: json!({
                        "numero": facture.numero,
                        "total_centimes": facture.total_centimes,
                    }),
                    actor_user_id: enc.created_by_user_id.map(|u| u.to_string()),
                })
                .await?;
        }
        Ok((enc, facture, quittance))
    }

    /// Export journal recettes OGEC (CSV `;`, UTF-8 BOM) — rôles compta uniquement.
    pub async fn export_ogec_for_actor(
        &self,
        role: &str,
        cmd: ExporterOgec,
    ) -> Result<Vec<u8>, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        plan_exporter_ogec(&cmd)?;
        let lignes = self
            .facturation
            .list_encaissements_periode(cmd.etablissement_id, cmd.debut, cmd.fin)
            .await?;
        Ok(render_ogec_csv(&lignes))
    }

    pub async fn creer_mandat_sepa_for_actor(
        &self,
        role: &str,
        actor_user_id: Uuid,
        cmd: CreerMandatSepa,
    ) -> Result<MandatSepa, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if Self::role_can_manage_facturation(role) {
            // staff factu
        } else if matches!(parsed, Role::Parent) {
            let foyers = self
                .foyer_ids_parent(cmd.etablissement_id, actor_user_id)
                .await?;
            if !foyers.contains(&cmd.foyer_id) {
                return Err(EntDbError::Forbidden);
            }
        } else {
            return Err(EntDbError::Forbidden);
        }
        if !self
            .facturation
            .foyer_exists(cmd.etablissement_id, cmd.foyer_id)
            .await?
        {
            return Err(DomainError::NotFound.into());
        }
        if self
            .facturation
            .get_sepa_mandat_actif_foyer(cmd.etablissement_id, cmd.foyer_id)
            .await?
            .is_some()
        {
            return Err(DomainError::SepaMandatInvalide.into());
        }
        let mandat = plan_creer_mandat_sepa(cmd)?;
        self.facturation.insert_sepa_mandat(&mandat).await?;
        let histo = IbanHisto {
            id: Uuid::new_v4(),
            etablissement_id: mandat.etablissement_id,
            foyer_id: mandat.foyer_id,
            iban: mandat.iban.clone(),
            bic: mandat.bic.clone(),
            titulaire: mandat.titulaire.clone(),
            mandat_id: Some(mandat.id),
            created_by_user_id: mandat.created_by_user_id,
            created_at: Utc::now(),
        };
        self.iban_histo.insert(&histo).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: mandat.etablissement_id,
                type_name: types::FAC_SEPA_MANDAT.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: mandat.id,
                eleve_id: None,
                payload: json!({
                    "foyer_id": mandat.foyer_id,
                    "rum": mandat.rum,
                    "statut": mandat.statut.as_str(),
                }),
                actor_user_id: mandat.created_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: histo.etablissement_id,
                type_name: types::IBAN_HISTO.to_string(),
                aggregate: AggregateKind::IbanHisto,
                aggregate_id: histo.id,
                eleve_id: None,
                payload: json!({
                    "foyer_id": histo.foyer_id,
                    "iban": histo.iban,
                    "mandat_id": histo.mandat_id,
                }),
                actor_user_id: histo.created_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        Ok(mandat)
    }

    pub async fn list_iban_histo_foyer_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        foyer_id: Uuid,
        limit: i64,
    ) -> Result<Vec<IbanHisto>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if Self::role_can_manage_facturation(role) {
            // ok
        } else if matches!(parsed, Role::Parent) {
            let foyers = self
                .foyer_ids_parent(etablissement_id, actor_user_id)
                .await?;
            if !foyers.contains(&foyer_id) {
                return Err(EntDbError::Forbidden);
            }
        } else {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .iban_histo
            .list_foyer(etablissement_id, foyer_id, limit.clamp(1, 100))
            .await?)
    }

    pub async fn list_mandats_sepa_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Vec<MandatSepa>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let all = self
            .facturation
            .list_sepa_mandats_etab(etablissement_id, 80)
            .await?;
        if Self::role_can_manage_facturation(role) {
            return Ok(all);
        }
        if matches!(parsed, Role::Parent) {
            let foyers = self
                .foyer_ids_parent(etablissement_id, actor_user_id)
                .await?;
            return Ok(all
                .into_iter()
                .filter(|m| foyers.contains(&m.foyer_id))
                .collect());
        }
        Err(EntDbError::Forbidden)
    }

    pub async fn revoquer_mandat_sepa_for_actor(
        &self,
        etablissement_id: Uuid,
        mandat_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
    ) -> Result<MandatSepa, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let current = self
            .facturation
            .get_sepa_mandat(etablissement_id, mandat_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if Self::role_can_manage_facturation(role) {
            // ok
        } else if matches!(parsed, Role::Parent) {
            let foyers = self
                .foyer_ids_parent(etablissement_id, actor_user_id)
                .await?;
            if !foyers.contains(&current.foyer_id) {
                return Err(EntDbError::Forbidden);
            }
        } else {
            return Err(EntDbError::Forbidden);
        }
        let revoke = plan_revoquer_mandat(&current)?;
        self.facturation
            .update_sepa_mandat_statut(&revoke)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: revoke.etablissement_id,
                type_name: types::FAC_SEPA_MANDAT.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: revoke.id,
                eleve_id: None,
                payload: json!({
                    "foyer_id": revoke.foyer_id,
                    "rum": revoke.rum,
                    "statut": revoke.statut.as_str(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(revoke)
    }

    pub async fn planifier_prelevement_for_actor(
        &self,
        role: &str,
        cmd: PlanifierPrelevement,
    ) -> Result<PrelevementSepa, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let facture = self
            .facturation
            .get_facture(cmd.etablissement_id, cmd.facture_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let mandat = self
            .facturation
            .get_sepa_mandat(cmd.etablissement_id, cmd.mandat_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let deja = self
            .facturation
            .sum_encaissements(cmd.etablissement_id, cmd.facture_id)
            .await?;
        let ouvert = self
            .facturation
            .sum_prelevements_ouverts(cmd.etablissement_id, cmd.facture_id)
            .await?;
        let prev = plan_planifier_prelevement(&facture, &mandat, deja, ouvert, cmd)?;
        self.facturation.insert_sepa_prelevement(&prev).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: prev.etablissement_id,
                type_name: types::FAC_SEPA_PRELEVEMENT.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: prev.facture_id,
                eleve_id: None,
                payload: json!({
                    "prelevement_id": prev.id,
                    "montant_centimes": prev.montant_centimes,
                    "date_execution": prev.date_execution,
                    "end_to_end_id": prev.end_to_end_id,
                }),
                actor_user_id: prev.created_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        Ok(prev)
    }

    pub async fn export_sepa_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        debut: NaiveDate,
        fin: NaiveDate,
        actor_user_id: Uuid,
        format: &str,
        crediteur: Option<SepaCrediteur>,
    ) -> Result<(Vec<u8>, &'static str), EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        if fin < debut || (fin - debut).num_days() > 366 {
            return Err(DomainError::SepaPrelevementInvalide.into());
        }
        let format = format.trim().to_ascii_lowercase();
        let is_pain = matches!(format.as_str(), "pain008" | "pain.008" | "xml");
        if is_pain && crediteur.is_none() {
            return Err(DomainError::SepaMandatInvalide.into());
        }
        let lignes = self
            .facturation
            .list_sepa_prelevements_a_exporter(etablissement_id, debut, fin)
            .await?;
        if lignes.is_empty() {
            return Err(DomainError::SepaPrelevementInvalide.into());
        }

        let mut seq_map = std::collections::HashMap::new();
        if is_pain {
            for l in &lignes {
                if seq_map.contains_key(&l.prelevement.mandat_id) {
                    continue;
                }
                let hist = self
                    .facturation
                    .count_sepa_historique_mandat(
                        etablissement_id,
                        l.prelevement.mandat_id,
                    )
                    .await?;
                seq_map.insert(
                    l.prelevement.mandat_id,
                    if hist == 0 { "FRST" } else { "RCUR" },
                );
            }
        }

        for ligne in &lignes {
            let marked = plan_marquer_exporte(&ligne.prelevement)?;
            self.facturation
                .update_sepa_prelevement(&marked)
                .await?;
        }

        let (bytes, content_kind) = if is_pain {
            let cdtr = crediteur.expect("crediteur checked");
            let msg_id = format!(
                "SDD-{}",
                &Uuid::new_v4().simple().to_string()[..16]
            );
            let xml = render_sepa_pain008(&cdtr, &lignes, &msg_id, Utc::now(), &seq_map)?;
            (xml, "pain008")
        } else {
            (render_sepa_csv(&lignes), "csv")
        };

        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::FAC_SEPA_EXPORT.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: etablissement_id,
                eleve_id: None,
                payload: json!({
                    "debut": debut,
                    "fin": fin,
                    "nb": lignes.len(),
                    "format": content_kind,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok((bytes, content_kind))
    }

    pub async fn rejeter_prelevement_for_actor(
        &self,
        etablissement_id: Uuid,
        prelevement_id: Uuid,
        role: &str,
        motif: &str,
        actor_user_id: Uuid,
    ) -> Result<PrelevementSepa, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .facturation
            .get_sepa_prelevement(etablissement_id, prelevement_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let rejected = plan_rejeter_prelevement(&current, motif)?;
        self.facturation
            .update_sepa_prelevement(&rejected)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::FAC_SEPA_REJET.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: rejected.facture_id,
                eleve_id: None,
                payload: json!({
                    "prelevement_id": rejected.id,
                    "statut": rejected.statut.as_str(),
                    "motif_rejet": rejected.motif_rejet,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(rejected)
    }

    /// Ingest relevé bancaire CSV → rapproche + exécute / rejette les prélèvements SEPA.
    /// Dry-run banque = pas d'API PSD2 ; les écritures métier s'appliquent quand même.
    pub async fn rapprocher_releve_banque_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        csv: &str,
    ) -> Result<BankRapprochementSummary, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        if !self.bank.is_ready() {
            return Err(EntDbError::Forbidden);
        }
        let lines = self.bank.ingest_csv(csv)?;
        let mut summary = BankRapprochementSummary::default();
        summary.lines = lines.len();
        for line in &lines {
            let planned = plan_action_ligne(line);
            let prev = self
                .facturation
                .get_sepa_prelevement_by_e2e(etablissement_id, &line.end_to_end_id)
                .await?;
            let Some(prev) = prev else {
                let row = BankRapprochementRow {
                    id: Uuid::new_v4(),
                    etablissement_id,
                    end_to_end_id: line.end_to_end_id.clone(),
                    montant_centimes: line.montant_centimes,
                    result: line.result.as_str().to_string(),
                    motif: line.motif.clone(),
                    prelevement_id: None,
                    action: "skip".into(),
                    status: "unmatched".into(),
                    error: Some("prelevement introuvable".into()),
                    created_at: Utc::now(),
                };
                self.bank_repo.insert_rapprochement(&row).await?;
                summary.unmatched += 1;
                continue;
            };
            if prev.montant_centimes != line.montant_centimes {
                let row = BankRapprochementRow {
                    id: Uuid::new_v4(),
                    etablissement_id,
                    end_to_end_id: line.end_to_end_id.clone(),
                    montant_centimes: line.montant_centimes,
                    result: line.result.as_str().to_string(),
                    motif: line.motif.clone(),
                    prelevement_id: Some(prev.id),
                    action: "skip".into(),
                    status: "error".into(),
                    error: Some(format!(
                        "montant mismatch: banque={} sepa={}",
                        line.montant_centimes, prev.montant_centimes
                    )),
                    created_at: Utc::now(),
                };
                self.bank_repo.insert_rapprochement(&row).await?;
                summary.errors += 1;
                continue;
            }
            let apply_result = match planned.action {
                BankMatchAction::Execute => {
                    self.executer_prelevement_for_actor(
                        etablissement_id,
                        prev.id,
                        role,
                        actor_user_id,
                    )
                    .await
                    .map(|_| ())
                }
                BankMatchAction::Reject => {
                    let motif = line
                        .motif
                        .clone()
                        .unwrap_or_else(|| "REJET_BANQUE".into());
                    self.rejeter_prelevement_for_actor(
                        etablissement_id,
                        prev.id,
                        role,
                        &motif,
                        actor_user_id,
                    )
                    .await
                    .map(|_| ())
                }
            };
            let (action, status, error) = match apply_result {
                Ok(()) => {
                    match planned.action {
                        BankMatchAction::Execute => summary.executed += 1,
                        BankMatchAction::Reject => summary.rejected += 1,
                    }
                    (
                        match planned.action {
                            BankMatchAction::Execute => "execute",
                            BankMatchAction::Reject => "reject",
                        }
                        .to_string(),
                        "applied".to_string(),
                        None,
                    )
                }
                Err(e) => {
                    summary.errors += 1;
                    (
                        match planned.action {
                            BankMatchAction::Execute => "execute",
                            BankMatchAction::Reject => "reject",
                        }
                        .to_string(),
                        "error".to_string(),
                        Some(e.to_string()),
                    )
                }
            };
            let row = BankRapprochementRow {
                id: Uuid::new_v4(),
                etablissement_id,
                end_to_end_id: line.end_to_end_id.clone(),
                montant_centimes: line.montant_centimes,
                result: line.result.as_str().to_string(),
                motif: line.motif.clone(),
                prelevement_id: Some(prev.id),
                action,
                status,
                error,
                created_at: Utc::now(),
            };
            self.bank_repo.insert_rapprochement(&row).await?;
        }
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::BANK_RAPPROCHEMENT.to_string(),
                aggregate: AggregateKind::Bank,
                aggregate_id: etablissement_id,
                eleve_id: None,
                payload: json!({
                    "lines": summary.lines,
                    "executed": summary.executed,
                    "rejected": summary.rejected,
                    "unmatched": summary.unmatched,
                    "errors": summary.errors,
                    "bank_live": self.bank.is_live(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(summary)
    }

    pub async fn count_bank_rapprochements(
        &self,
        etablissement_id: Uuid,
    ) -> Result<i64, EntDbError> {
        Ok(self.bank_repo.count_rapprochements(etablissement_id).await?)
    }

    pub async fn count_bank_rapprochements_status(
        &self,
        etablissement_id: Uuid,
        status: &str,
    ) -> Result<i64, EntDbError> {
        Ok(self
            .bank_repo
            .count_by_status(etablissement_id, status)
            .await?)
    }

    pub async fn executer_prelevement_for_actor(
        &self,
        etablissement_id: Uuid,
        prelevement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
    ) -> Result<(PrelevementSepa, Encaissement, Facture, Quittance), EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .facturation
            .get_sepa_prelevement(etablissement_id, prelevement_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let facture = self
            .facturation
            .get_facture(etablissement_id, current.facture_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let deja = self
            .facturation
            .sum_encaissements(etablissement_id, current.facture_id)
            .await?;
        let ouvert = self
            .facturation
            .sum_prelevements_ouverts(etablissement_id, current.facture_id)
            .await?;
        // Ce prélèvement quitte la réserve ouverte en devenant encaissement.
        let autres_ouverts = ouvert.saturating_sub(current.montant_centimes);
        let engage = deja.saturating_add(autres_ouverts);
        let (enc, facture, quittance) = self
            .encaisser_avec_engage(
                role,
                EncaisserFacture {
                    etablissement_id,
                    facture_id: current.facture_id,
                    montant_centimes: current.montant_centimes,
                    moyen: MoyenPaiement::Prelevement,
                    reference: Some(current.end_to_end_id.clone()),
                    actor_user_id,
                },
                &facture,
                deja,
                engage,
            )
            .await?;
        let executed = plan_executer_prelevement(&current, enc.id)?;
        self.facturation
            .update_sepa_prelevement(&executed)
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::FAC_SEPA_EXECUTE.to_string(),
                aggregate: AggregateKind::Facture,
                aggregate_id: executed.facture_id,
                eleve_id: None,
                payload: json!({
                    "prelevement_id": executed.id,
                    "encaissement_id": enc.id,
                    "quittance_id": quittance.id,
                    "montant_centimes": executed.montant_centimes,
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok((executed, enc, facture, quittance))
    }

    /// Lance les rappels J-3 / en retard pour les factures émises (idempotent).
    pub async fn lancer_rappels_echeances_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        today: NaiveDate,
    ) -> Result<Vec<RappelFacture>, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        let factures = self
            .facturation
            .list_factures_emises(etablissement_id, 200)
            .await?;
        let mut envoyes = Vec::new();
        for facture in factures {
            let echeances = self
                .facturation
                .list_echeances(etablissement_id, facture.id)
                .await?;
            if echeances.is_empty() {
                continue;
            }
            let encaisse = self
                .facturation
                .sum_encaissements(etablissement_id, facture.id)
                .await?;
            let candidats = plan_candidats_rappel(&facture, &echeances, encaisse, today);
            for cand in candidats {
                let mut rappel =
                    plan_creer_rappel(&cand, RappelCanal::Interne, actor_user_id, None)?;
                let inserted = self.facturation.insert_rappel(&rappel).await?;
                if !inserted {
                    continue;
                }
                let eleve_id = self
                    .facturation
                    .first_eleve_id_foyer(etablissement_id, cand.foyer_id)
                    .await?;
                if let Some(eleve_id) = eleve_id {
                    let (fil, msg) = plan_ouvrir_fil(OuvrirFil {
                        etablissement_id,
                        eleve_id,
                        sujet: sujet_rappel(&cand),
                        premier_message: cand_corps(&cand),
                        actor_user_id,
                    })?;
                    self.messages.insert_fil(&fil).await?;
                    self.messages.insert_message(&msg).await?;
                    self.facturation
                        .update_rappel_fil(
                            etablissement_id,
                            rappel.id,
                            fil.id,
                            RappelCanal::Message,
                        )
                        .await?;
                    rappel.fil_id = Some(fil.id);
                    rappel.canal = RappelCanal::Message;
                    self.events
                        .publish(MetierEventDraft {
                            etablissement_id,
                            type_name: types::MSG_FIL_OUVERT.to_string(),
                            aggregate: AggregateKind::MsgFil,
                            aggregate_id: fil.id,
                            eleve_id: Some(eleve_id),
                            payload: json!({
                                "sujet": fil.sujet,
                                "source": "fac_rappel",
                                "echeance_id": cand.echeance_id,
                            }),
                            actor_user_id: Some(actor_user_id.to_string()),
                        })
                        .await?;
                }
                self.events
                    .publish(MetierEventDraft {
                        etablissement_id,
                        type_name: types::FAC_RAPPEL_ENVOYE.to_string(),
                        aggregate: AggregateKind::Facture,
                        aggregate_id: rappel.facture_id,
                        eleve_id,
                        payload: json!({
                            "rappel_id": rappel.id,
                            "echeance_id": rappel.echeance_id,
                            "kind": rappel.kind.as_str(),
                            "canal": rappel.canal.as_str(),
                            "fil_id": rappel.fil_id,
                        }),
                        actor_user_id: Some(actor_user_id.to_string()),
                    })
                    .await?;
                envoyes.push(rappel);
            }
        }
        Ok(envoyes)
    }

    pub async fn list_rappels_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<RappelFacture>, EntDbError> {
        if !Self::role_can_manage_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .facturation
            .list_rappels_etab(etablissement_id, 80)
            .await?)
    }
}

fn cand_corps(c: &domain_eleve::RappelCandidat) -> String {
    domain_eleve::corps_rappel(c)
}
