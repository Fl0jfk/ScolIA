use crate::absence_repo::PgAbsenceRepo;
use crate::appel_repo::{Appel, AppelLigne, AppelStatut, LigneEtat, PgAppelRepo};
use crate::annonce_repo::PgAnnonceRepo;
use crate::retard_repo::PgRetardRepo;
use crate::competence_repo::{EvaluationLue, PgCompetenceRepo};
use crate::auth_repo::{
    AssuranceSession, AuthUser, ClientSession, OptionsSession, PgAuthRepo, PorteeSession,
    SessionInfo, StaffMember,
};
use crate::passkey_repo::{ChallengeRow, CounterOutcome, InsertPasskey, PasskeyRow, PgPasskeyRepo};
use crate::oidc_repo::{
    OidcProviderPublic, OidcProviderRow, OidcProviderUpsert, OidcStateInsert, PgOidcRepo,
};
use crate::carnet_repo::PgCarnetRepo;
use crate::cahier_texte_repo::PgCahierTexteRepo;
use crate::edt_repo::{jour_semaine_iso, EdtCreneau, PgEdtRepo};
use crate::eleve_compte_repo::{EleveCompte, PgEleveCompteRepo};
use crate::eleve_repo::PgEleveRepo;
use crate::tenant_tx::TenantTx;
use crate::event_store::PgEventStore;
use crate::foyer_repo::{CreateLien, EleveLien, LienDossierFoyer, PgFoyerRepo, UpdateLien};
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
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
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
    plan_annuler_soumission_justificatif, plan_confirmer_reinscription_parent,
    plan_enregistrer_scolarite_annee, plan_justifier_absence,
    plan_montee_rapide, plan_ouvrir_campagne, plan_poser_dispense_repas, plan_poser_exigence_doc,
    plan_poser_grille_repas, plan_refuser_justificatif, plan_sortir, plan_update_identite,
    today_utc, Absence, AbsenceStatut,
    AffecterBourse, AjouterDispositif, AjouterOption, AttributKind, BourseType, ChangeAttribut,
    ChangeClasse, ChangeMef, ChangeRegime, ChangeSite, ChangeStatus, ChoisirOptionN1,
    ConfigurerAcompteCampagne, CreateEleve, CreerBourseType, CreerRepasActivite,
    ConfirmerReinscription, DeciderBourse, DeciderGrant, DecisionAbsence, DeclareAbsence,
    DemanderGrant, DeposerDocument, DocConfidentialite, DocTiroir, DocumentGrant, DomainError,
    DispenseSource, DossierEleve, DossierSections, Eleve, EleveBourse, EleveDispenseRepas,
    EleveDispositif, EleveDocument, EleveGrilleRepas, EleveOption, ElevePeriode, EleveStatus,
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
    FactureStatut, InviterCompteEleve, MandatSepa, Message, MessageLue, MsgFil, MsgModele, MsgPriorite,
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
    plan_attacher_ordonnance_pai, plan_resume_fiche_sante,
    plan_upsert_dossier_psy, plan_upsert_fiche_sante,
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
    plan_exporter_lsun, render_lsun_csv, ExporterLsun, LigneLsun,
    plan_creer_convention_stage, plan_creer_entreprise, ConventionStage, CreerConventionStage,
    CreerEntreprise, Entreprise,
    plan_creer_edt_contrainte, CreerEdtContrainte, EdtContrainte,
};
use domain_eleve::{plan_creer_annonce, Annonce, AudienceAnnonce, CreerAnnonce};
use domain_eleve::{plan_declarer_retard, DeclarerRetard, NiveauIdentite, Retard};
use domain_eleve::{
    plan_creer_competence_item, plan_evaluer_competence, render_livret_competences_pdf,
    CompetenceItem, CreerCompetenceItem, EvaluerCompetence, EvaluationCompetence,
    LigneLivretCompetence, LivretCompetences, NiveauMaitrise,
};
use ent_auth::{
    generate_challenge_token, Action, Portee, generate_reset_token, generate_temp_password, hash_token, AuthError,
    Passkey, PasskeyAuthentication, PasskeyEngine, PasskeyError, PasskeyRegistration,
    PublicKeyCredential, RegisterPublicKeyCredential, Role, SecretBox, SecretBoxError,
};
use ent_brain::BrainP0;
use crate::outbox::{Causalite, ConfigDistribution, Distributeur, Publier};
use crate::reactions::{consommateurs_standard, NotificationInterne};
use ent_events::{types, AggregateKind, EventBusError, EventHandler, MetierEvent, MetierEventDraft};
use ent_mail::{MailAttachment, MailReceipt, MailSendStatus, MailService, OutgoingMail};
use ent_sms::{OutgoingSms, SmsService};
use ent_push::{OutgoingPush, PushPlatform, PushService};
use ent_bank::{plan_action_ligne, BankConnector, BankMatchAction, BankParseError};
use ent_object::presign::{Disposition, LinkSpec, PresignError, Presigner};
use ent_object::upload::{ErreurFichier, PolitiqueFichiers, UsageFichier};
use ent_object::{ObjectStore, ObjectStoreError};
use ent_oidc::{
    build_authorization_url, discover, exchange_authorization_code, generate_pkce, random_token,
    valider_url, HttpOidcTransport, OidcDiscovery, OidcError, OidcProviderConfig, OidcTransport,
    UrlPolicy,
};
use std::sync::{Arc, RwLock};
use serde_json::json;
use crate::copilote::{self, ReponseCopilote};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

/// Commande de provisionnement d'un compte personnel (staff).
#[derive(Debug, Clone)]
pub struct ProvisionnerPersonnel {
    pub email: String,
    pub display_name: Option<String>,
    pub role: String,
}
/// Issue d'une réinitialisation de mot de passe par un tiers habilité.
///
/// Socle 6, lot P0 : plus aucun mot de passe temporaire n'est produit ni renvoyé, quel que soit le
/// profil de la cible. Un lien à usage unique est envoyé à l'adresse du compte.
#[derive(Clone)]
pub enum ReinitialisationMdp {
    /// Un lien a été envoyé, aucun secret n'est renvoyé.
    LienEnvoye,
}

/// Famille d'opérations de gestion de compte par un tiers (hiérarchies distinctes).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperationCompte {
    Email,
    Identifiants,
    /// Fermer, suspendre, rouvrir un accès ou en programmer la fin (socle 6, lot P0).
    Acces,
}

/// Empreinte SHA-256 tronquée d'une adresse (pseudonymisation pour le journal).
fn empreinte_email(email: &str) -> String {
    use sha2::{Digest, Sha256};
    let norm = email.trim().to_ascii_lowercase();
    Sha256::digest(norm.as_bytes())
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Catégories Unicode Cf (format : bidi U+202A-202E/2066-2069, largeur nulle
/// U+200B-200F, U+2060-2064, BOM U+FEFF, tags…) et séparateurs de ligne /
/// paragraphe Zl/Zp : invisibles ou trompeurs dans un libellé affiché.
fn est_format_unicode(c: char) -> bool {
    matches!(c as u32,
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x0890..=0x0891 | 0x08E2
        | 0x180E | 0x200B..=0x200F | 0x2028..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F
        | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x110CD | 0x13430..=0x1343F
        | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0001 | 0xE0020..=0xE007F)
}

/// Nom de passkey : trim, 64 caractères maximum, pas de caractère de contrôle
/// ni de format Unicode (Cf).
fn nettoyer_nom_passkey(name: Option<String>) -> Result<Option<String>, EntDbError> {
    let Some(n) = name else { return Ok(None) };
    let n = n.trim();
    if n.is_empty() {
        return Ok(None);
    }
    if n.chars().count() > 64 || n.chars().any(|c| c.is_control() || est_format_unicode(c)) {
        return Err(EntDbError::PasskeyNomInvalide);
    }
    Ok(Some(n.to_string()))
}


/// Résultat d'un provisionnement staff. Aucun secret : la personne reçoit une invitation par
/// e-mail (lien à usage unique) et choisit elle-même son mot de passe / enrôle sa passkey.
#[derive(Clone)]
pub struct PersonnelProvisionne {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    pub invitation_envoyee: bool,
}

/// Durée de vie d'un challenge passkey (minutes). Court : c'est une fenêtre
/// d'authentification, pas une session. Configurable via
/// `ENT_PASSKEY_CHALLENGE_MINUTES` (5 par défaut).
fn passkey_challenge_minutes() -> i64 {
    std::env::var("ENT_PASSKEY_CHALLENGE_MINUTES")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|m| *m > 0)
        .unwrap_or(5)
}

/// Durée de vie d'un parcours SSO (minutes). Assez large pour une saisie de
/// mot de passe chez l'IdP, assez court pour limiter le rejeu.
/// `ENT_OIDC_STATE_MINUTES` (10 par défaut).
fn oidc_state_minutes() -> i64 {
    std::env::var("ENT_OIDC_STATE_MINUTES")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|m| *m > 0)
        .unwrap_or(10)
}

/// URL de retour enregistrée chez l'IdP. Dérivée de l'URL publique de l'ENT
/// (`ENT_PUBLIC_BASE_URL`, la même que celle des liens e-mail ; repli
/// `http://localhost:8088` en dev). Surchargeable par fournisseur (colonne
/// `ent_oidc_provider.redirect_uri`) quand l'IdP impose une URL enregistrée.
fn oidc_default_redirect_uri() -> String {
    let base = std::env::var("ENT_PUBLIC_BASE_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "http://localhost:8088".to_string());
    format!("{base}/auth/oidc/callback")
}

/// Lien court d'accès à un fichier privé (réponse des routes `…/lien`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LienFichier {
    pub url: String,
    /// Expiration, secondes Unix (UTC).
    pub expires_at: i64,
}

/// Nom de fichier affiché et type MIME d'un document (partagé téléchargement / lien).
pub fn nom_fichier_document(doc: &EleveDocument) -> (String, String) {
    let mime = doc
        .mime_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    let base = doc.titre.replace('/', "_").replace('\\', "_");
    let ext = match ent_object::upload::TypeFichier::depuis_mime(mime) {
        Some(t) => format!(".{}", t.extension()),
        None => String::new(),
    };
    let nom = if base.contains('.') {
        base
    } else {
        format!("{base}{ext}")
    };
    (nom, mime.to_string())
}
/// Contrôles de démarrage fail-closed (hors labo/test).
struct RpConfigCheck;

impl RpConfigCheck {
    fn verifier() -> Result<(), EntDbError> {
        PasskeyEngine::from_env()?;
        SecretBox::from_env()?;
        Ok(())
    }
}

/// Contexte (AAD) du chiffrement du secret client OIDC.
const OIDC_SECRET_AAD: &[u8] = b"ent_oidc_provider.client_secret";

/// Nature d'un refus métier explicite (voir [`EntDbError::Refus`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusMetier {
    /// Donnée invalide (HTTP 422).
    Invalide,
    /// État incompatible avec l'action (HTTP 409).
    Conflit,
    /// Action refusée par une règle métier (HTTP 403).
    Interdit,
}

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
    #[error(transparent)]
    Passkey(#[from] PasskeyError),
    #[error(transparent)]
    Oidc(#[from] OidcError),
    #[error(transparent)]
    Secrets(#[from] SecretBoxError),
    #[error("parcours SSO inconnu, expiré ou déjà utilisé")]
    OidcStateInvalide,
    #[error("aucun compte local rattaché à cette identité SSO (création par un administrateur requise)")]
    OidcCompteInconnu,
    #[error("fournisseur SSO introuvable ou désactivé")]
    OidcProviderInconnu,
    /// 2ᵉ facteur (passkey) requis pour ce compte
    #[error("2ᵉ facteur (passkey) requis pour ce compte")]
    PasskeyRequis,
    #[error("accès refusé pour ce rôle")]
    Forbidden,
    /// Saisie refusée par une règle du dossier élève (message sans donnée personnelle).
    #[error("valeur invalide : {0}")]
    SaisieInvalide(&'static str),
    /// Opération du dossier élève incompatible avec l'état actuel.
    #[error("opération en conflit : {0}")]
    ConflitDossier(&'static str),
    /// Même conflit, avec un détail construit (liste de domaines, de types d'événements) : le
    /// message est destiné à l'utilisateur et ne contient aucune donnée personnelle.
    #[error("opération en conflit : {0}")]
    ConflitDossierDetail(String),
    /// Refus métier explicite (S2-6) : le message est destiné à l'utilisateur.
    #[error("{1}")]
    Refus(RefusMetier, &'static str),
    /// Mot de passe actuel absent ou incorrect (re-vérification d'une action sensible).
    #[error("mot de passe actuel requis ou incorrect")]
    MotDePasseActuelInvalide,
    /// Envoi d'un lien impossible : la messagerie n'est pas prête (aucun secret n'est produit).
    #[error("envoi impossible : la messagerie n'est pas configurée")]
    EnvoiImpossible,
    /// Lien de récupération de passkey inconnu, déjà utilisé, annulé ou expiré.
    #[error("lien de récupération invalide ou expiré")]
    RecuperationLienInvalide,
    /// Finalisation demandée avant la fin du délai de sécurité (D-018).
    #[error("délai de sécurité en cours")]
    RecuperationDelaiEnCours(DateTime<Utc>),
    /// Lien de changement d'e-mail inconnu, déjà utilisé, annulé ou expiré (RF-01).
    #[error("lien de changement d'adresse invalide ou expiré")]
    EmailLienInvalide,
    /// Plafond de passkeys atteint pour ce compte.
    #[error("limite de passkeys atteinte ({0} maximum)")]
    PasskeyPlafond(i64),
    /// Nom de passkey invalide (trop long ou caractères de contrôle).
    #[error("nom de passkey invalide")]
    PasskeyNomInvalide,
    #[error("inventaire de sauvegarde d'un autre établissement refusé")]
    InventairePorteeEtrangere,
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
    #[error("fichier absent pour ce document")]
    FichierAbsent,
    /// Liens courts de fichiers non émissibles (secret de signature absent ou invalide : fail-closed).
    #[error("liens de fichiers indisponibles")]
    LiensIndisponibles,
    /// D2 : fichier déposé refusé (type réel, taille, nom, fichier actif, antivirus).
    #[error("{0}")]
    Fichier(ErreurFichier),
    #[error("import EDT refusé : conflits détectés")]
    EdtImportConflicts,
}

/// Issue d'une tentative de connexion par mot de passe : soit une session est
/// ouverte, soit un 2ᵉ facteur (passkey) est exigé avant toute session.
pub enum LoginOutcome {
    Session(SessionInfo),
    PasskeyRequired {
        token: String,
        options: serde_json::Value,
        expires_at: DateTime<Utc>,
    },
}

/// Une sonde chiffrée prête à être publiée en jauge (`/metrics`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SondeMetrique {
    pub nom: String,
    pub etiquettes: Vec<(String, String)>,
    pub valeur: i64,
}

impl SondeMetrique {
    pub fn jauge(nom: &str, etiquettes: &[(&str, &str)], valeur: i64) -> Self {
        Self {
            nom: nom.to_string(),
            etiquettes: etiquettes
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
            valeur,
        }
    }
}

/// Paramètres de création / mise à jour d'un fournisseur OIDC d'établissement.
#[derive(Clone)]
pub struct OidcProviderParams {
    pub slug: String,
    pub libelle: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    /// `None` → `openid email profile`.
    pub scopes: Option<String>,
    /// `None` → `{ENT_PUBLIC_BASE_URL}/auth/oidc/callback`.
    pub redirect_uri: Option<String>,
}

/// Résumé d'un ingest relevé bancaire (rapprochement SEPA).
#[derive(Debug, Clone, Default)]
pub struct BankRapprochementSummary {    pub lines: usize,
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

/// Vrai si `key` est un segment « simple » (alphanumérique, `-`, `_`), non vide.
fn is_simple_segment(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Clé objet d'une pièce de la forme `{etab}/{ns}/{id}.{ext}` appartenant bien à (etab, id).
fn object_key_is_owned(etab: Uuid, ns: &str, id: Uuid, key: &str) -> bool {
    let segs: Vec<&str> = key.split('/').collect();
    let [e, n, file] = segs.as_slice() else {
        return false;
    };
    let Some((fid, ext)) = file.split_once('.') else {
        return false;
    };
    *e == etab.to_string()
        && *n == ns
        && fid == id.to_string()
        && is_simple_segment(ext)
}

/// Photo d'élève : `{etab}/trombi/{eleve_id}.{ext}` uniquement.
pub fn photo_storage_key_is_owned(eleve: &Eleve, key: &str) -> bool {
    object_key_is_owned(eleve.etablissement_id, "trombi", eleve.id, key)
}

/// Pièce PAI : `{etab}/pai/{piece.id}.{ext}` uniquement.
pub fn pai_storage_key_is_owned(piece: &PaiPiece, key: &str) -> bool {
    object_key_is_owned(piece.etablissement_id, "pai", piece.id, key)
}


/// Vérifie que la clé objet stockée APPARTIENT bien à ce document (même établissement,
/// même document/élève). Toute autre forme n'est jamais lue dans le magasin d'objets.
///
/// Formes acceptées :
/// - `{etab}/documents/{doc.id}.{pdf|png|jpg|txt|bin}` (dépôt avec contenu) ;
/// - `{etab}/bulletin|livret-competences/{uuid}.pdf-{doc.eleve_id}` (bulletins, livrets) ;
/// - `reinscription/{etab}/{doc.eleve_id}/{CODE}.pdf` (réinscription sans contenu) ;
/// - `lab/{8 premiers hex de l'établissement}/{nom}.pdf` (seeds de labo).
pub fn document_storage_key_is_owned(doc: &EleveDocument, key: &str) -> bool {
    let etab = doc.etablissement_id;
    let segs: Vec<&str> = key.split('/').collect();
    match segs.as_slice() {
        [e, "documents", file] => {
            let Some((id, ext)) = file.split_once('.') else {
                return false;
            };
            *e == etab.to_string()
                && id == doc.id.to_string()
                && matches!(ext, "pdf" | "png" | "jpg" | "txt" | "bin")
        }
        [e, ns @ ("bulletin" | "livret-competences"), file] => {
            let _ = ns;
            let Some((periode, eleve)) = file.split_once(".pdf-") else {
                return false;
            };
            *e == etab.to_string()
                && Uuid::parse_str(periode).is_ok()
                && eleve == doc.eleve_id.to_string()
        }
        ["reinscription", e, el, file] => {
            let Some(code) = file.strip_suffix(".pdf") else {
                return false;
            };
            *e == etab.to_string() && *el == doc.eleve_id.to_string() && is_simple_segment(code)
        }
        ["lab", short, file] => {
            let Some(name) = file.strip_suffix(".pdf") else {
                return false;
            };
            let expected = etab.simple().to_string();
            *short == &expected[..8] && is_simple_segment(name)
        }
        _ => false,
    }
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
    annonces: PgAnnonceRepo,
    retards: PgRetardRepo,
    competences: PgCompetenceRepo,
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
    presigner: Presigner,
    /// D2 : politique des fichiers déposés (signatures, tailles, antivirus).
    uploads: PolitiqueFichiers,
    auth: PgAuthRepo,
    passkeys: PgPasskeyRepo,
    passkey_engine: Option<PasskeyEngine>,
    /// Coffre des secrets au repos (`ENT_SECRETS_KEY`). `None` seulement si la clé
    /// est absente/invalide hors labo/test : `connect` refuse alors de démarrer.
    secrets: Option<SecretBox>,
    recup: crate::recuperation_repo::PgRecuperationRepo,
    email_change: crate::email_change_repo::PgEmailChangeRepo,
    oidc: PgOidcRepo,
    oidc_transport: RwLock<Arc<dyn OidcTransport>>,
    events: PgEventStore,
    brain: Option<BrainP0>,
    /// Un envoi `DryRun` compte comme réussi (labo / tests uniquement).
    mail_dry_run_accepte: bool,
}

/// `ENT_DATABASE_MIGRATION_URL` (non vide) : URL du rôle propriétaire pour les migrations.
fn migration_url_from_env() -> Option<String> {
    std::env::var("ENT_DATABASE_MIGRATION_URL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl PgEnt {
    /// Connexion **uniquement** si l'URL est localhost / 127.0.0.1.
    pub async fn connect(database_url: &str) -> Result<Self, EntDbError> {
        assert_local_postgres_url(database_url)?;
        // Fail-closed hors labo/test (socle 1, A5/A7) : configuration WebAuthn
        // (rp_id + origines https) et clé de chiffrement des secrets obligatoires.
        if !ent_auth::env_non_prod() {
            RpConfigCheck::verifier()?;
        }
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
            annonces: PgAnnonceRepo::new(pool.clone()),
            retards: PgRetardRepo::new(pool.clone()),
            competences: PgCompetenceRepo::new(pool.clone()),
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
            presigner: Presigner::from_env(),
            uploads: PolitiqueFichiers::depuis_env(),
            auth: PgAuthRepo::new(pool.clone()),
            passkeys: PgPasskeyRepo::new(pool.clone()),
            passkey_engine: match PasskeyEngine::from_env() {
                Ok(e) => Some(e),
                Err(err) => {
                    tracing::warn!(error = %err, "WebAuthn indisponible : passkeys désactivées (fail-closed)");
                    None
                }
            },
            secrets: SecretBox::from_env().ok(),
            recup: crate::recuperation_repo::PgRecuperationRepo::new(pool.clone()),
            email_change: crate::email_change_repo::PgEmailChangeRepo::new(pool.clone()),
            oidc: PgOidcRepo::new(pool.clone()),
            oidc_transport: RwLock::new(Arc::new(HttpOidcTransport::from_env())),
            events: PgEventStore::new(pool.clone()),
            pool,
            brain: None,
            mail_dry_run_accepte: cfg!(feature = "labo"),
        }
    }

    /// Lab : force dry-run SMTP / SMS / push + banque + objet mémoire.
    pub fn install_mail_dry_run(&mut self) {
        self.mailer = MailService::dry_run_only();
        // Mode lab/test explicite : un reçu `DryRun` vaut envoi réussi.
        self.mail_dry_run_accepte = true;
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

    /// Tests (labo uniquement) : messagerie « prête » mais qui n'envoie rien de
    /// confirmé. `live = false` → reçus `DryRun` ; `live = true` → tentative SMTP
    /// vers un port local fermé (reçu `Failed`). `accepter_dry_run = false`
    /// reproduit le comportement de production (un `DryRun` n'est pas un envoi).
    #[cfg(feature = "labo")]
    pub fn installer_mailer_de_test(&mut self, live: bool, accepter_dry_run: bool) {
        self.mailer = MailService::with_config(
            ent_mail::SmtpConfig {
                from: "ent-test@example.test".into(),
                auth_user: "ent-test".into(),
                pass: "x".into(),
                host: "127.0.0.1".into(),
                port: 1,
                secure: false,
            },
            live,
        );
        self.mail_dry_run_accepte = accepter_dry_run;
    }

    /// Expédie les messages d'inscription en file (après validation des transactions
    /// métier). Appelée par la tâche de fond de l'API ; les tests l'appellent à la main.
    pub async fn expedier_envois_inscriptions(&self, limite: i64) -> Result<usize, EntDbError> {
        Ok(crate::inscriptions_reactions::expedier_envois(
            &self.pool,
            &self.mailer,
            self.mail_dry_run_accepte,
            limite,
        )
        .await?)
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

    /// Émetteur de liens courts (jetons HMAC locaux ou URL SigV4).
    pub fn presigner(&self) -> &Presigner {
        &self.presigner
    }

    /// D2 : remplace la politique des fichiers déposés (tests, branchement d'un antivirus).
    pub fn installer_politique_fichiers(&mut self, politique: PolitiqueFichiers) {
        self.uploads = politique;
    }

    /// D2 : politique courante (tailles maximales, analyseur).
    pub fn politique_fichiers(&self) -> &PolitiqueFichiers {
        &self.uploads
    }

    /// Remplace l'émetteur de liens (tests / démarrage explicite).
    pub fn installer_presigner(&mut self, presigner: Presigner) {
        self.presigner = presigner;
    }

    pub async fn set_user_telephone(
        &self,
        user_id: Uuid,
        telephone: &str,
    ) -> Result<(), EntDbError> {
        Ok(self.auth.set_telephone(user_id, telephone).await?)
    }

    pub async fn set_user_email(
        &self,
        user_id: Uuid,
        email: &str,
    ) -> Result<(), EntDbError> {
        Ok(self.auth.set_email(user_id, email).await?)
    }

    /// Applique les migrations. Si `ENT_DATABASE_MIGRATION_URL` est définie (rôle
    /// **propriétaire** des tables, C4), elles passent par une connexion dédiée,
    /// fermée ensuite : le rôle applicatif (`ENT_DATABASE_URL`) n'a alors aucun
    /// droit DDL. Sinon : comportement historique (migrations via le pool applicatif).
    pub async fn migrate(&self) -> Result<(), EntDbError> {
        match migration_url_from_env() {
            Some(url) => {
                assert_local_postgres_url(&url)?;
                let owner_pool = PgPoolOptions::new().max_connections(2).connect(&url).await?;
                let r = apply_socle_migrations(&owner_pool).await;
                owner_pool.close().await;
                r?;
            }
            None => apply_socle_migrations(&self.pool).await?,
        }
        Ok(())
    }

    /// Sonde de disponibilité (readiness) : la base répond.
    pub async fn ping(&self) -> Result<(), EntDbError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    /// Pool sous-jacent (usage interne au crate : PRA, sondes, migrateurs).
    pub(crate) fn auth_repo(&self) -> &PgAuthRepo {
        &self.auth
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Après commit : informe les observateurs locaux des événements publiés par outbox.
    pub(crate) fn diffuser_apres_commit(&self, events: &[ent_events::MetierEvent]) {
        self.events.diffuser_locaux(events);
    }

    /// Limiteur de débit (seaux en base, partagés entre instances).
    pub fn limiteur_debit(&self) -> crate::RateLimiter {
        crate::RateLimiter::new(self.pool.clone())
    }

    /// Sondes chiffrées exposées par `/metrics` (jauges). Une seule requête par
    /// famille : volontairement peu coûteuses pour un scrape toutes les 15 s.
    ///
    /// Aucune donnée nominative : ce sont des **compteurs d'exploitation**
    /// (pool, sessions actives, volumétrie du outbox, passkeys enrôlées).
    pub async fn sondes_metriques(&self) -> Result<Vec<SondeMetrique>, EntDbError> {
        let mut sondes = vec![
            SondeMetrique::jauge(
                "ent_db_pool_connexions",
                &[("etat", "total")],
                self.pool.size() as i64,
            ),
            SondeMetrique::jauge(
                "ent_db_pool_connexions",
                &[("etat", "inactives")],
                self.pool.num_idle() as i64,
            ),
            SondeMetrique::jauge(
                "ent_db_pool_connexions",
                &[("etat", "maximum")],
                self.pool.options().get_max_connections() as i64,
            ),
        ];

        let sessions = sqlx::query(
            "SELECT count(*)::bigint AS n FROM ent_session
             WHERE revoked_at IS NULL AND expires_at > now()",
        )
        .fetch_one(&self.pool)
        .await?
        .get::<i64, _>("n");
        sondes.push(SondeMetrique::jauge("ent_sessions_actives", &[], sessions));

        let passkeys =
            sqlx::query("SELECT count(*)::bigint AS n FROM ent_passkey_credential")
                .fetch_one(&self.pool)
                .await?
                .get::<i64, _>("n");
        sondes.push(SondeMetrique::jauge("ent_passkeys_total", &[], passkeys));

        let eleves = sqlx::query("SELECT count(*)::bigint AS n FROM ent_eleve")
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("n");
        sondes.push(SondeMetrique::jauge("ent_eleves_total", &[], eleves));

        let etablissements =
            sqlx::query("SELECT count(DISTINCT etablissement_id)::bigint AS n FROM ent_membership")
                .fetch_one(&self.pool)
                .await?
                .get::<i64, _>("n");
        sondes.push(SondeMetrique::jauge(
            "ent_etablissements_total",
            &[],
            etablissements,
        ));

        // Outbox courrier par statut : `dry_run` = jamais parti (pas de SMTP
        // configuré ou envoi désactivé) — c'est la jauge d'alerte principale.
        for ligne in sqlx::query(
            "SELECT status AS statut, count(*)::bigint AS n
             FROM ent_mail_outbox GROUP BY status ORDER BY status",
        )
        .fetch_all(&self.pool)
        .await?
        {
            let statut: String = ligne.get("statut");
            sondes.push(SondeMetrique::jauge(
                "ent_mails_outbox",
                &[("statut", statut.as_str())],
                ligne.get::<i64, _>("n"),
            ));
        }

        Ok(sondes)
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

    /// Distributeur de l'outbox (socle 2, S2-0) avec les consommateurs standard
    /// (réactions entre domaines). Le lancer avec `Arc::new(d).lancer()`.
    pub fn distributeur(&self, config: ConfigDistribution) -> Distributeur {
        let mut d = Distributeur::new(self.pool.clone(), config);
        for c in consommateurs_standard() {
            d.enregistrer(c).expect("consommateurs standard aux noms uniques");
        }
        // S2-3 : réactions à `inscription.validee` (envois par la messagerie de l'ENT).
        for c in crate::inscriptions_reactions::consommateurs_inscriptions(
            self.mailer.clone(),
            self.mail_dry_run_accepte,
        ) {
            d.enregistrer(c).expect("consommateurs d'inscription aux noms uniques");
        }
        // S2-9 : facturation par événements (inscription, régime, sortie, absence).
        for c in crate::finances_reactions::consommateurs_finances() {
            d.enregistrer(c).expect("consommateur de facturation au nom unique");
        }
        d
    }

    /// Notifications internes de l'utilisateur connecté (plus récentes d'abord),
    /// strictement dans son établissement.
    pub async fn mes_notifications(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        limite: i64,
    ) -> Result<Vec<NotificationInterne>, EntDbError> {
        Ok(crate::reactions::notifications_de(&self.pool, etablissement_id, user_id, limite).await?)
    }

    pub fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        self.events.register_handler(handler);
    }

    /// Notifie les parents liés par e-mail (dry-run ou live). Best-effort.
    ///
    /// Anti-doublon **par source** (`source_id`, `kind`) : un même geste
    /// (signal, carnet, facture…) ne notifie qu'une fois. Pour une diffusion à
    /// plusieurs familles partageant une source, voir `notify_parents_mail_broadcast`.
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
        self.envoyer_mail_parents(
            etablissement_id,
            eleve_id,
            kind,
            subject,
            body_text,
            source_aggregate,
            source_id,
            actor_user_id,
            attachments,
            None,
        )
        .await
    }

    /// Facture, quittance : seulement les **payeurs du foyer concerné** (D-014, RP1-11),
    /// jamais les responsables d'un autre foyer du même élève.
    #[allow(clippy::too_many_arguments)]
    async fn notify_payeurs_foyer_mail(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
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
            .emails_payeurs_foyer(etablissement_id, foyer_id)
            .await?;
        self.envoyer_mail_parents(
            etablissement_id,
            eleve_id,
            kind,
            subject,
            body_text,
            source_aggregate,
            source_id,
            actor_user_id,
            attachments,
            Some(emails),
        )
        .await
    }

    /// Diffusion : anti-doublon **par destinataire** (élève). Plusieurs familles
    /// peuvent partager la même `source_id` (ex. annonce établissement) sans se
    /// bloquer mutuellement.
    #[allow(clippy::too_many_arguments)]
    async fn notify_parents_mail_broadcast(
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
        if self
            .mail_outbox
            .count_by_source_kind_eleve(etablissement_id, source_id, kind, eleve_id)
            .await?
            > 0
        {
            return Ok(0);
        }
        self.envoyer_mail_parents(
            etablissement_id,
            eleve_id,
            kind,
            subject,
            body_text,
            source_aggregate,
            source_id,
            actor_user_id,
            attachments,
            None,
        )
        .await
    }

    /// Corps commun : envoie à tous les responsables liés à l'élève et journalise.
    #[allow(clippy::too_many_arguments)]
    async fn envoyer_mail_parents(
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
        destinataires: Option<Vec<String>>,
    ) -> Result<usize, EntDbError> {
        let emails = match destinataires {
            Some(d) => d,
            None => {
                self.foyers
                    .emails_parents_eleve(etablissement_id, eleve_id)
                    .await?
            }
        };
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
                    tracing::error!(canal = "mail", kind = %kind, error = %e, "envoi échoué");
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
        // Revue finale socle 1 : l'historique des envois (adresses des familles, sujets
        // nominatifs) est réservé à `audit.lire` ; avant, tout personnel (professeur,
        // comptable, infirmerie…) le lisait.
        Self::exiger_etablissement(role, Action::AuditLire)?;
        let lim = limit.clamp(1, 200);
        Ok(self.mail_outbox.list_recent(etablissement_id, lim).await?)
    }

    pub async fn list_sms_recents_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        limit: i64,
    ) -> Result<Vec<crate::sms_repo::SmsOutboxRow>, EntDbError> {
        // Revue finale socle 1 : téléphones des familles et corps nominatifs des SMS
        // (sanctions, absences) → `audit.lire`, ou la vie scolaire (CPE) qui les envoie.
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if parsed != Role::Cpe && !Self::peut_etablissement(role, Action::AuditLire) {
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
                    tracing::error!(canal = "sms", kind = %kind, error = %e, "envoi échoué");
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
    /// Push aux responsables liés à l'élève. Anti-doublon **par source**.
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
        self.envoyer_push_parents(
            etablissement_id,
            eleve_id,
            kind,
            title,
            body,
            source_aggregate,
            source_id,
            actor_user_id,
        )
        .await
    }

    /// Diffusion : anti-doublon **par destinataire** (élève) — plusieurs familles
    /// peuvent partager la même `source_id` (ex. annonce établissement).
    #[allow(clippy::too_many_arguments)]
    async fn notify_parents_push_broadcast(
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
            .count_by_source_kind_eleve(etablissement_id, source_id, kind, eleve_id)
            .await?
            > 0
        {
            return Ok(0);
        }
        self.envoyer_push_parents(
            etablissement_id,
            eleve_id,
            kind,
            title,
            body,
            source_aggregate,
            source_id,
            actor_user_id,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn envoyer_push_parents(
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
        self.envoyer_push_parents_vers(
            etablissement_id,
            eleve_id,
            kind,
            title,
            body,
            source_aggregate,
            source_id,
            actor_user_id,
            None,
        )
        .await
    }

    /// Notification d'un fil de messagerie : un fil privé ou de foyer (D-014, RP1-11)
    /// ne notifie que ses destinataires, jamais les autres responsables de l'élève.
    #[allow(clippy::too_many_arguments)]
    async fn notify_parents_push_fil(
        &self,
        fil: &MsgFil,
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
            .count_by_source_kind(fil.etablissement_id, source_id, kind)
            .await?
            > 0
        {
            return Ok(0);
        }
        let seulement = self
            .messages
            .destinataires_famille_fil(fil.etablissement_id, fil.id)
            .await?;
        self.envoyer_push_parents_vers(
            fil.etablissement_id,
            fil.eleve_id,
            kind,
            title,
            body,
            source_aggregate,
            source_id,
            actor_user_id,
            seulement,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn envoyer_push_parents_vers(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        kind: &str,
        title: &str,
        body: &str,
        source_aggregate: &str,
        source_id: Uuid,
        actor_user_id: Option<Uuid>,
        seulement: Option<Vec<Uuid>>,
    ) -> Result<usize, EntDbError> {
        // Liens ACTIFS seulement (S2-1) : un responsable suspendu n'est plus notifié.
        let liens = self
            .foyers
            .list_liens_eleve_actifs(etablissement_id, eleve_id)
            .await?;
        let user_ids: Vec<Uuid> = liens
            .iter()
            .map(|l| l.user_id)
            .filter(|u| seulement.as_ref().is_none_or(|s| s.contains(u)))
            .collect();
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
                    tracing::error!(canal = "push", kind = %kind, error = %e, "envoi échoué");
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

    /// Push aux membres staff (CPE / direction / admin) avec device enregistré. Best-effort.
    async fn notify_staff_push(
        &self,
        etablissement_id: Uuid,
        eleve_id: Option<Uuid>,
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
        let user_ids = self
            .auth
            .list_user_ids_with_roles(
                etablissement_id,
                &["cpe", "direction", "admin", "administratif"],
            )
            .await?;
        let devices = self
            .push_devices
            .list_active_for_users(etablissement_id, &user_ids)
            .await?;
        let mut n = 0usize;
        for device in devices {
            if actor_user_id == Some(device.user_id) {
                continue;
            }
            let platform = PushPlatform::parse(&device.platform).unwrap_or(PushPlatform::Ios);
            let data_json = match eleve_id {
                Some(eid) => Some(format!(
                    "{{\"kind\":\"{kind}\",\"eleve_id\":\"{eid}\"}}"
                )),
                None => Some(format!("{{\"kind\":\"{kind}\"}}")),
            };
            let receipt = match self
                .push
                .send(OutgoingPush {
                    token: device.token.clone(),
                    platform,
                    title: title.to_string(),
                    body: body.to_string(),
                    data_json,
                })
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(canal = "push staff", kind = %kind, error = %e, "envoi échoué");
                    continue;
                }
            };
            let outbox_id = self
                .push_devices
                .insert_outbox(InsertPushOutbox {
                    etablissement_id,
                    eleve_id,
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
                    eleve_id,
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
                        "audience": "staff",
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
        self.login_with_ip(email, password, etablissement_id, None)
            .await
    }

    /// Connexion avec l'IP cliente (garde anti-force brute par IP, en plus du
    /// verrou par compte). `client_ip` ne vient que d'un proxy de confiance.
    ///
    /// **Ne contourne pas** le 2ᵉ facteur : pour un compte doté de passkeys,
    /// cette méthode échoue avec [`EntDbError::PasskeyRequis`] — utilisez
    /// [`PgEnt::login_start_with_ip`]. Les appelants internes (seed, tests) ne
    /// manipulent que des comptes sans passkey.
    pub async fn login_with_ip(
        &self,
        email: &str,
        password: &str,
        etablissement_id: Uuid,
        client_ip: Option<&str>,
    ) -> Result<SessionInfo, EntDbError> {
        match self
            .login_start_with_ip(email, password, etablissement_id, client_ip)
            .await?
        {
            LoginOutcome::Session(session) => Ok(session),
            LoginOutcome::PasskeyRequired { .. } => Err(EntDbError::PasskeyRequis),
        }
    }

    /// Connexion *complète* par mot de passe : émet une session **ou** signale
    /// qu'une passkey est exigée (2ᵉ facteur) pour ce compte.
    pub async fn login_start_with_ip(
        &self,
        email: &str,
        password: &str,
        etablissement_id: Uuid,
        client_ip: Option<&str>,
    ) -> Result<LoginOutcome, EntDbError> {
        self.login_start_client(
            email,
            password,
            etablissement_id,
            client_ip,
            ClientSession::Web,
            None,
        )
        .await
    }

    /// Connexion par mot de passe pour un type de client (D-011 : web / mobile).
    /// Sans passkey : session complète, ou « enrôlement seulement » si la
    /// politique de l'établissement impose la passkey à ce rôle (D-009).
    pub async fn login_start_client(
        &self,
        email: &str,
        password: &str,
        etablissement_id: Uuid,
        client_ip: Option<&str>,
        client: ClientSession,
        appareil: Option<String>,
    ) -> Result<LoginOutcome, EntDbError> {
        let identity = match self
            .auth
            .verify_credentials(email, password, etablissement_id, client_ip)
            .await
        {
            Ok(identity) => identity,
            Err(e) => {
                self.tracer_echec_connexion(etablissement_id, email, client_ip.is_some(), &e)
                    .await;
                // C2 : un non-membre reçoit la même réponse que de mauvais
                // identifiants (pas d'énumération) ; le journal garde la cause.
                let e = match e {
                    AuthError::NotMember => AuthError::InvalidCredentials,
                    autre => autre,
                };
                return Err(e.into());
            }
        };
        let passkeys = self
            .passkeys
            .list_passkeys(identity.user_id)
            .await?;
        if passkeys.is_empty() {
            let portee = if self
                .passkey_obligatoire(etablissement_id, &identity.role)
                .await?
            {
                PorteeSession::Enrolement
            } else {
                PorteeSession::Complete
            };
            let session = self
                .auth
                .issue_session(
                    identity.user_id,
                    etablissement_id,
                    &identity.email,
                    &identity.role,
                    &OptionsSession {
                        client,
                        assurance: AssuranceSession::MotDePasse,
                        portee,
                        appareil,
                        ip: client_ip.map(str::to_string),
                    },
                )
                .await?;
            self.audit_acces(
                etablissement_id,
                Some(identity.user_id),
                &identity.email,
                &identity.role,
                "connexion",
                "Connexion à l'ENT",
            )
            .await;
            return Ok(LoginOutcome::Session(session));
        }

        // 2ᵉ facteur : on n'ouvre pas de session tant que la passkey n'a pas été
        // vérifiée. Le challenge est à usage unique et expire vite.
        let (token, options, expires_at) = self
            .open_authentication_challenge(etablissement_id, identity.user_id, &passkeys, client)
            .await?;
        self.audit_acces(
            etablissement_id,
            Some(identity.user_id),
            &identity.email,
            &identity.role,
            "connexion_2fa",
            "Mot de passe validé — passkey exigée",
        )
        .await;
        Ok(LoginOutcome::PasskeyRequired {
            token,
            options,
            expires_at,
        })
    }

    /// Moteur WebAuthn du déploiement (RP ID / origine). Absent seulement si la
    /// configuration est invalide : l'inscription comme la connexion par
    /// passkey échouent alors explicitement (fail closed).
    pub(crate) fn passkey_engine(&self) -> Result<&PasskeyEngine, EntDbError> {
        self.passkey_engine
            .as_ref()
            .ok_or(EntDbError::Passkey(PasskeyError::Config))
    }

    /// Coffre des secrets au repos : absent seulement hors labo/test sans clé valide.
    fn secrets(&self) -> Result<&SecretBox, EntDbError> {
        self.secrets
            .as_ref()
            .ok_or(EntDbError::Secrets(SecretBoxError::KeyMissing))
    }

    /// Nombre de passkeys d'un compte (information de sécurité pour l'UI).
    pub async fn count_passkeys_for_user(&self, user_id: Uuid) -> Result<i64, EntDbError> {
        Ok(self.passkeys.count_for_user(user_id).await?)
    }

    /// Ouvre (et persiste) un challenge d'authentification pour les passkeys
    /// fournies. Renvoie le token opaque remis au client, les options WebAuthn
    /// et l'expiration.
    async fn open_authentication_challenge(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        passkeys: &[Passkey],
        client: ClientSession,
    ) -> Result<(String, serde_json::Value, DateTime<Utc>), EntDbError> {
        let engine = self.passkey_engine()?;
        let (rcr, state) = engine.start_authentication(passkeys)?;
        let state_json = serde_json::to_value(&state)
            .map_err(|_| EntDbError::Passkey(PasskeyError::BadState))?;
        let options = serde_json::to_value(&rcr).unwrap_or(serde_json::Value::Null);
        let (raw_token, token_hash) = generate_challenge_token();
        let expires_at =
            Utc::now() + chrono::Duration::minutes(passkey_challenge_minutes());
        self.passkeys.prune_expired_challenges().await?;
        self.passkeys
            .insert_challenge(
                etablissement_id,
                user_id,
                "authenticate",
                &state_json,
                &token_hash,
                expires_at,
                client.as_str(),
            )
            .await?;
        Ok((raw_token, options, expires_at))
    }

    /// Démarre l'enrôlement d'une passkey pour l'acteur connecté. Renvoie le
    /// token de corrélation et les options à passer au navigateur.
    pub async fn passkey_register_start_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<(String, serde_json::Value), EntDbError> {
        if !self.auth.is_member(etablissement_id, user_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let (email, display_name) = self
            .auth
            .user_profile(user_id)
            .await?
            .ok_or(EntDbError::Forbidden)?;
        let display = display_name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| email.clone());
        self.verifier_plafond_passkeys(user_id).await?;
        let exclude = self
            .passkeys
            .list_credential_ids(user_id)
            .await?;
        let engine = self.passkey_engine()?;
        let (ccr, state) = engine.start_registration(user_id, &email, &display, exclude)?;
        let state_json = serde_json::to_value(&state)
            .map_err(|_| EntDbError::Passkey(PasskeyError::BadState))?;
        let options = serde_json::to_value(&ccr).unwrap_or(serde_json::Value::Null);
        let (raw_token, token_hash) = generate_challenge_token();
        let expires_at =
            Utc::now() + chrono::Duration::minutes(passkey_challenge_minutes());
        self.passkeys.prune_expired_challenges().await?;
        self.passkeys
            .insert_challenge(
                etablissement_id,
                user_id,
                "register",
                &state_json,
                &token_hash,
                expires_at,
                ClientSession::Web.as_str(),
            )
            .await?;
        Ok((raw_token, options))
    }

    /// Finalise l'enrôlement : vérifie l'attestation puis persiste la passkey.
    pub async fn passkey_register_finish_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        token: &str,
        credential: &RegisterPublicKeyCredential,
        name: Option<String>,
    ) -> Result<PasskeyRow, EntDbError> {
        let name = nettoyer_nom_passkey(name)?;
        self.verifier_plafond_passkeys(user_id).await?;
        let token_hash = hash_token(token);
        let challenge = self
            .passkeys
            .take_challenge(&token_hash, "register")
            .await?
            .ok_or(EntDbError::Passkey(PasskeyError::RegisterStart))?;
        // Le challenge doit appartenir au même compte / établissement : un
        // token volé ne peut pas enrôler une passkey sur un autre compte.
        if challenge.user_id != user_id || challenge.etablissement_id != etablissement_id {
            return Err(EntDbError::Forbidden);
        }
        let state: PasskeyRegistration = serde_json::from_value(challenge.state.clone())
            .map_err(|_| EntDbError::Passkey(PasskeyError::BadState))?;
        let engine = self.passkey_engine()?;
        let passkey = engine.finish_registration(credential, &state)?;
        match self
            .passkeys
            .insert_credential_plafonne(
                etablissement_id,
                user_id,
                &passkey,
                name.as_deref(),
                Self::PASSKEYS_MAX,
            )
            .await?
        {
            InsertPasskey::Stored => {}
            // Plafond vérifié sous verrou : les enrôlements concurrents ne le dépassent pas.
            InsertPasskey::Plafond => {
                return Err(EntDbError::PasskeyPlafond(Self::PASSKEYS_MAX))
            }
            // Même appareil déjà enrôlé (identifiant de credential connu).
            InsertPasskey::Duplicate => {
                return Err(EntDbError::Passkey(PasskeyError::RegisterFinish))
            }
        }
        let email = self
            .auth
            .resolve_user_email(user_id)
            .await
            .unwrap_or_default();
        self.audit_acces(
            etablissement_id,
            Some(user_id),
            &email,
            "",
            "passkey_enrolement",
            "Enrôlement d'une passkey",
        )
        .await;
        // Prévenir la personne (best-effort) : une passkey inconnue d'elle est
        // un signe de compromission.
        if self.mailer.is_ready() && !email.is_empty() {
            let _ = self
                .mailer
                .send(OutgoingMail {
                    to: email.clone(),
                    subject: "Une passkey a été ajoutée à votre compte ENT".into(),
                    body_text: "Bonjour,\n\nUne nouvelle passkey vient d'être ajoutée à votre compte ENT. Si vous n'êtes pas à l'origine de cet ajout, contactez immédiatement l'établissement : elle permet de se connecter à votre place.\n".into(),
                    attachments: vec![],
                })
                .await;
        }
        let rows = self
            .passkeys
            .list_rows(user_id)
            .await?;
        rows.into_iter()
            .last()
            .ok_or(EntDbError::Passkey(PasskeyError::RegisterFinish))
    }

    /// Passkeys d'un compte (métadonnées, jamais la matière cryptographique).
    pub async fn list_passkeys_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<PasskeyRow>, EntDbError> {
        if !self.auth.is_member(etablissement_id, user_id).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.passkeys.list_rows(user_id).await?)
    }

    /// Plafond de passkeys par compte.
    pub const PASSKEYS_MAX: i64 = 10;

    async fn verifier_plafond_passkeys(&self, user_id: Uuid) -> Result<(), EntDbError> {
        if self.passkeys.count_for_user(user_id).await? >= Self::PASSKEYS_MAX {
            return Err(EntDbError::PasskeyPlafond(Self::PASSKEYS_MAX));
        }
        Ok(())
    }

    /// Re-vérifie le mot de passe actuel d'un utilisateur déjà authentifié
    /// (action sensible). **Ne touche jamais au compteur de connexion** : un
    /// mauvais mot de passe n'incrémente que le compteur dédié de
    /// ré-authentification (`reauth:<user>`, 5 échecs / 15 min par défaut →
    /// `TooManyAttempts`/429 sur ces seules routes). Une session volée ne
    /// permet donc ni la force brute ni le verrouillage du login du propriétaire.
    pub async fn verifier_mot_de_passe_actuel(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        password: &str,
    ) -> Result<(), EntDbError> {
        if password.is_empty() {
            return Err(EntDbError::MotDePasseActuelInvalide);
        }
        if !self.auth.is_member(etablissement_id, user_id).await? {
            return Err(EntDbError::Forbidden);
        }
        if self.auth.verify_current_password(user_id, password).await? {
            Ok(())
        } else {
            Err(EntDbError::MotDePasseActuelInvalide)
        }
    }

    /// Enrôlement passkey (point d'entrée HTTP) : exige le mot de passe actuel,
    /// puis révoque les **autres** sessions du compte (la session courante est
    /// conservée).
    #[allow(clippy::too_many_arguments)]
    ///
    /// Session « enrôlement » (obligation D-009) : elle vient d'être ouverte par
    /// une authentification complète (mot de passe ou SSO) et ne sert qu'à cela ;
    /// le mot de passe actuel n'est alors pas redemandé (compte SSO sans mot de
    /// passe). La session est ensuite élevée en session complète.
    #[allow(clippy::too_many_arguments)]
    pub async fn passkey_register_finish_session(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        keep_session_id: Uuid,
        portee: PorteeSession,
        assurance: AssuranceSession,
        current_password: &str,
        token: &str,
        credential: &RegisterPublicKeyCredential,
        name: Option<String>,
    ) -> Result<PasskeyRow, EntDbError> {
        // Revue socle 1 (E2) : seul un compte entré par SSO (souvent sans mot de
        // passe) enrôle sans le redemander. Après une connexion par mot de passe,
        // on le redemande : un jeton d'enrôlement volé ne suffit pas à poser une
        // passkey de l'attaquant.
        if portee == PorteeSession::Enrolement
            && assurance == AssuranceSession::Sso
            && current_password.is_empty()
        {
            if !self.auth.is_member(etablissement_id, user_id).await? {
                return Err(EntDbError::Forbidden);
            }
        } else {
            self.verifier_mot_de_passe_actuel(etablissement_id, user_id, current_password)
                .await?;
        }
        let row = self
            .passkey_register_finish_for_actor(etablissement_id, user_id, token, credential, name)
            .await?;
        if portee == PorteeSession::Enrolement {
            self.auth
                .completer_session_apres_enrolement(keep_session_id)
                .await?;
        }
        let _ = self
            .auth
            .revoke_other_sessions_for_user(user_id, Some(keep_session_id))
            .await?;
        Ok(row)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn passkey_register_finish_verified(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        keep_session_id: Uuid,
        current_password: &str,
        token: &str,
        credential: &RegisterPublicKeyCredential,
        name: Option<String>,
    ) -> Result<PasskeyRow, EntDbError> {
        self.verifier_mot_de_passe_actuel(etablissement_id, user_id, current_password)
            .await?;
        let row = self
            .passkey_register_finish_for_actor(etablissement_id, user_id, token, credential, name)
            .await?;
        let _ = self
            .auth
            .revoke_other_sessions_for_user(user_id, Some(keep_session_id))
            .await?;
        Ok(row)
    }

    /// Retire une passkey du compte connecté (peut désactiver son 2ᵉ facteur).
    /// Exige le mot de passe actuel ; révoque ensuite les autres sessions.
    pub async fn delete_passkey_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        keep_session_id: Uuid,
        current_password: &str,
        passkey_id: Uuid,
    ) -> Result<bool, EntDbError> {
        self.verifier_mot_de_passe_actuel(etablissement_id, user_id, current_password)
            .await?;
        let removed = self
            .passkeys
            .delete_credential(user_id, passkey_id)
            .await?;
        if removed {
            let _ = self
                .auth
                .revoke_other_sessions_for_user(user_id, Some(keep_session_id))
                .await?;
            let email = self
                .auth
                .resolve_user_email(user_id)
                .await
                .unwrap_or_default();
            self.audit_acces(
                etablissement_id,
                Some(user_id),
                &email,
                "",
                "passkey_suppression",
                "Suppression d'une passkey",
            )
            .await;
        }
        Ok(removed)
    }

    /// Démarre une connexion **sans mot de passe** par passkey à partir de
    /// l'e-mail. `None` si l'utilisateur n'existe pas, n'appartient pas à
    /// l'établissement ou n'a aucune passkey : la réponse HTTP reste générique
    /// pour ne pas révéler l'existence d'un compte.
    pub async fn passkey_login_start(
        &self,
        etablissement_id: Uuid,
        email: &str,
    ) -> Result<Option<(String, serde_json::Value, DateTime<Utc>)>, EntDbError> {
        self.passkey_login_start_client(etablissement_id, email, ClientSession::Web)
            .await
    }

    /// Variante avec type de client (web / mobile, D-011).
    pub async fn passkey_login_start_client(
        &self,
        etablissement_id: Uuid,
        email: &str,
        client: ClientSession,
    ) -> Result<Option<(String, serde_json::Value, DateTime<Utc>)>, EntDbError> {
        let Some(user_id) = self.auth.find_user_id_by_email(email).await? else {
            return Ok(None);
        };
        if !self.auth.is_member(etablissement_id, user_id).await? {
            return Ok(None);
        }
        let passkeys = self.passkeys.list_passkeys(user_id).await?;
        if passkeys.is_empty() {
            return Ok(None);
        }
        let (token, options, expires_at) = self
            .open_authentication_challenge(etablissement_id, user_id, &passkeys, client)
            .await?;
        Ok(Some((token, options, expires_at)))
    }

    /// Challenge **factice** de même forme qu'un vrai (jeton opaque, options
    /// WebAuthn, expiration) pour répondre à `passkey_login_start` quand le
    /// compte n'existe pas, n'est pas membre ou n'a pas de passkey : la réponse
    /// HTTP est ainsi indiscernable (anti-énumération). Rien n'est persisté : le
    /// jeton renvoyé est refusé comme n'importe quel jeton inconnu à l'étape
    /// `finish`.
    pub fn passkey_challenge_factice(&self, email: &str) -> (String, serde_json::Value, DateTime<Utc>) {
        fn b64url(octets: &[u8]) -> String {
            const A: &[u8; 64] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
            let mut s = String::with_capacity(octets.len() * 4 / 3 + 2);
            for bloc in octets.chunks(3) {
                let n = bloc.iter().enumerate().fold(0u32, |a, (i, b)| a | (u32::from(*b) << (16 - 8 * i)));
                for i in 0..=bloc.len() {
                    s.push(A[((n >> (18 - 6 * i)) & 63) as usize] as char);
                }
            }
            s
        }
        fn aleatoire_32() -> [u8; 32] {
            let mut o = [0u8; 32];
            o[..16].copy_from_slice(Uuid::new_v4().as_bytes());
            o[16..].copy_from_slice(Uuid::new_v4().as_bytes());
            o
        }
        let (jeton, _) = generate_challenge_token();
        let (rp_id, _, _) = ent_auth::rp_config_from_env();
        // RF-07 : le nombre d'`allowCredentials` d'un vrai défi est le nombre de passkeys
        // du compte (stable). Le factice en annonce 1 à 3, fixé par l'adresse via un secret
        // de processus : stable d'un appel à l'autre, sans lien avec l'existence du compte.
        static SEL: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
        let sel = SEL.get_or_init(aleatoire_32);
        let empreinte = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(sel);
            h.update(email.trim().to_ascii_lowercase().as_bytes());
            h.finalize()
        };
        let nb = match empreinte[0] {
            0..=203 => 1,
            204..=243 => 2,
            _ => 3,
        };
        let identifiants: Vec<serde_json::Value> = (0..nb)
            .map(|_| json!({ "type": "public-key", "id": b64url(&aleatoire_32()) }))
            .collect();
        let options = json!({
            "publicKey": {
                "challenge": b64url(&aleatoire_32()),
                "timeout": 300000,
                "rpId": rp_id,
                "allowCredentials": identifiants,
                // Revue finale socle 1 : même valeur que le vrai défi (webauthn-rs exige
                // la vérification de l'utilisateur) ; « preferred » trahissait le factice.
                "userVerification": "required",
            }
        });
        let expires_at = Utc::now() + chrono::Duration::minutes(passkey_challenge_minutes());
        (jeton, options, expires_at)
    }

    /// Finalise une connexion par passkey (2ᵉ facteur **ou** sans mot de passe) :
    /// vérifie l'assertion, met à jour le compteur d'usage, ouvre la session.
    pub async fn passkey_login_finish(
        &self,
        token: &str,
        credential: &PublicKeyCredential,
    ) -> Result<SessionInfo, EntDbError> {
        let token_hash = hash_token(token);
        let challenge = self
            .passkeys
            .take_challenge(&token_hash, "authenticate")
            .await?
            .ok_or(EntDbError::Passkey(PasskeyError::AuthStart))?;
        let state: PasskeyAuthentication = serde_json::from_value(challenge.state.clone())
            .map_err(|_| EntDbError::Passkey(PasskeyError::BadState))?;
        let passkeys = self
            .passkeys
            .list_passkeys(challenge.user_id)
            .await?;
        if passkeys.is_empty() {
            return Err(EntDbError::Passkey(PasskeyError::AuthFinish));
        }
        let engine = self.passkey_engine()?;
        let result = match engine.finish_authentication(credential, &state) {
            Ok(r) => r,
            Err(PasskeyError::PossibleCompromise) => {
                self.signaler_compteur_regresse(&challenge, "refusé par la vérification WebAuthn")
                    .await;
                return Err(EntDbError::Passkey(PasskeyError::PossibleCompromise));
            }
            Err(e) => return Err(e.into()),
        };
        // Compteur d'usage : mise à jour atomique (verrou de ligne) du seul
        // credential utilisé ; un compteur qui ne progresse pas est refusé.
        match self.passkeys.apply_authentication(&result).await? {
            CounterOutcome::Regressed { stored, received } => {
                self.signaler_compteur_regresse(
                    &challenge,
                    &format!("compteur reçu {received} ≤ compteur enregistré {stored}"),
                )
                .await;
                return Err(EntDbError::Passkey(PasskeyError::PossibleCompromise));
            }
            CounterOutcome::Unknown => {
                return Err(EntDbError::Passkey(PasskeyError::AuthFinish));
            }
            CounterOutcome::Updated | CounterOutcome::Unchanged => {}
        }
        let identity = self
            .auth
            .user_profile(challenge.user_id)
            .await?
            .ok_or(EntDbError::Passkey(PasskeyError::AuthFinish))?;
        let role = self
            .auth
            .role_in(challenge.etablissement_id, challenge.user_id)
            .await?
            .ok_or(EntDbError::Forbidden)?;
        let session = self
            .auth
            .issue_session(
                challenge.user_id,
                challenge.etablissement_id,
                &identity.0,
                &role,
                &OptionsSession {
                    client: ClientSession::parse_client(Some(&challenge.client)),
                    assurance: AssuranceSession::Passkey,
                    portee: PorteeSession::Complete,
                    appareil: None,
                    ip: None,
                },
            )
            .await?;
        self.audit_acces(
            challenge.etablissement_id,
            Some(challenge.user_id),
            &identity.0,
            &role,
            "connexion_passkey",
            "Connexion par passkey",
        )
        .await;
        // D-018 : une connexion réussie par passkey pendant le délai de
        // sécurité annule toute récupération en cours.
        self.annuler_recuperation_passkey_si_active(challenge.user_id, &identity.0, "connexion_passkey")
            .await;
        Ok(session)
    }

    /// Journalise (journal d'accès + traces) un compteur de signature régressé :
    /// la clé a peut-être été clonée. Aucun secret dans le message.
    async fn signaler_compteur_regresse(&self, challenge: &ChallengeRow, detail: &str) {
        tracing::warn!(
            user_id = %challenge.user_id,
            etablissement_id = %challenge.etablissement_id,
            detail,
            "passkey : compteur de signature régressé (clonage possible), connexion refusée"
        );
        let email = self
            .auth
            .user_profile(challenge.user_id)
            .await
            .ok()
            .flatten()
            .map(|p| p.0)
            .unwrap_or_default();
        let role = self
            .auth
            .role_in(challenge.etablissement_id, challenge.user_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        self.audit_acces(
            challenge.etablissement_id,
            Some(challenge.user_id),
            &email,
            &role,
            "passkey_compteur_regresse",
            &format!("Connexion passkey refusée : {detail} (clonage possible)"),
        )
        .await;
    }

    /// Récupération après perte d'appareil : Direction / Admin retire toutes
    /// les passkeys d'un membre de **son** établissement. Même hiérarchie que
    /// la réinitialisation de mot de passe (pas soi-même ; la Direction ne
    /// touche ni un Admin ni une autre Direction). Le compte retombe en simple
    /// mot de passe, **toutes ses sessions sont révoquées**, puis la personne
    /// réenrôle une passkey. Le journal porte l'acteur réel et la cible.
    pub async fn reset_passkeys_admin_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        target_user_id: Uuid,
    ) -> Result<u64, EntDbError> {
        let target = self
            .cible_compte_autorisee(
                etablissement_id,
                actor_user_id,
                actor_role,
                target_user_id,
                OperationCompte::Identifiants,
            )
            .await?;
        let removed = self
            .passkeys
            .delete_all_for_user(target_user_id)
            .await?;
        let sessions = self.auth.revoke_all_for_user(target_user_id).await?;
        self.auth.invalidate_pending_resets(target_user_id).await?;
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "passkey_reset_admin",
            "Réinitialisation des passkeys (récupération)",
            json!({
                "cible_user_id": target_user_id.to_string(),
                "cible_role": target.as_str(),
                "passkeys_retirees": removed,
                "sessions_revoquees": sessions,
            }),
        )
        .await;
        Ok(removed)
    }

    // ---- Récupération de passkey en libre-service (D-018) -------------------

    /// Journalise un événement de récupération **dans chaque établissement** de
    /// la personne. Le payload ne contient ni e-mail en clair (empreinte) ni
    /// aucune information sur les autres établissements (D-013).
    async fn audit_recuperation_passkey(
        &self,
        user_id: Uuid,
        email: &str,
        action: &str,
        libelle: &str,
        extra: serde_json::Value,
    ) {
        for (etab, role) in self
            .auth
            .list_memberships_for_user(user_id)
            .await
            .unwrap_or_default()
        {
            let mut payload = json!({ "empreinte": empreinte_email(email) });
            if let (Some(o), Some(e)) = (payload.as_object_mut(), extra.as_object()) {
                for (k, v) in e {
                    o.insert(k.clone(), v.clone());
                }
            }
            self.audit_compte(etab, user_id, &role, action, libelle, payload)
                .await;
        }
    }

    /// Annule une éventuelle récupération active (connexion par passkey réussie).
    /// Best-effort : n'échoue jamais la connexion.
    async fn annuler_recuperation_passkey_si_active(
        &self,
        user_id: Uuid,
        email: &str,
        motif: &str,
    ) {
        match self.recup.annuler_pour_user(user_id).await {
            Ok(n) if n > 0 => {
                self.audit_recuperation_passkey(
                    user_id,
                    email,
                    "passkey_recuperation_annulee",
                    "Récupération de passkey annulée",
                    json!({ "motif": motif }),
                )
                .await;
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("annulation de récupération de passkey : {e}"),
        }
    }

    /// Étape 1 — demande publique. Crée une demande et envoie le lien de
    /// confirmation **seulement si** le compte existe, appartient à
    /// l'établissement et a au moins une passkey. Renvoie `true` si un e-mail
    /// a été envoyé (tests) ; l'appelant HTTP répond 202 quoi qu'il arrive.
    pub async fn demander_recuperation_passkey(
        &self,
        email: &str,
        etablissement_id: Uuid,
        public_base_url: &str,
    ) -> Result<bool, EntDbError> {
        let Ok(Some(user_id)) = self.auth.find_user_id_by_email(email).await else {
            return Ok(false);
        };
        if !self.auth.is_member(etablissement_id, user_id).await.unwrap_or(false)
            || self.passkeys.count_for_user(user_id).await? == 0
            || self.email_change.est_gele(user_id).await?
        {
            return Ok(false);
        }
        if public_base_url.trim().is_empty() || !self.mailer.is_ready() {
            tracing::warn!("récupération de passkey : messagerie ou URL publique absente");
            return Ok(false);
        }
        let (raw, hashed) = generate_reset_token();
        let Some(id) = self.recup.creer(user_id, etablissement_id, &hashed).await? else {
            return Ok(false); // déjà une récupération en attente : on ne l'écrase pas
        };
        let to = self.auth.resolve_user_email(user_id).await?;
        let lien = format!(
            "{}/recuperation-passkey/confirmer?token={raw}",
            public_base_url.trim_end_matches('/')
        );
        let envoi = self
            .mailer
            .send(OutgoingMail {
                to,
                subject: "Récupération de votre passkey ENT".into(),
                body_text: format!(
                    "Bonjour,\n\nVous avez demandé à récupérer l'accès à votre compte après la perte de votre passkey. \
Pour confirmer (lien valable {} minutes), ouvrez :\n{lien}\n\n\
Un délai de sécurité de {} h s'appliquera ensuite avant tout changement : vous pourrez annuler pendant ce délai.\n\n\
Si vous n'êtes pas à l'origine de cette demande, ignorez ce message : rien ne sera modifié.\n",
                    crate::recuperation_repo::CONFIRMATION_TTL_MINUTES,
                    crate::recuperation_repo::delai_securite_heures(),
                ),
                attachments: vec![],
            })
            .await;
        match envoi {
            Ok(r) if self.envoi_reussi(&r) => Ok(true),
            _ => {
                let _ = self.recup.abandonner(id).await;
                tracing::warn!("récupération de passkey : lien non envoyé");
                Ok(false)
            }
        }
    }

    /// Étape 2 — confirmation par le lien : démarre le délai de sécurité,
    /// envoie à la personne l'alerte (lien d'annulation + lien de finalisation)
    /// et journalise dans chaque établissement. Renvoie la fin du délai.
    pub async fn confirmer_recuperation_passkey(
        &self,
        token: &str,
        public_base_url: &str,
    ) -> Result<DateTime<Utc>, EntDbError> {
        if public_base_url.trim().is_empty() || !self.mailer.is_ready() {
            return Err(EntDbError::EnvoiImpossible);
        }
        let (annul_raw, annul_hash) = generate_reset_token();
        let (final_raw, final_hash) = generate_reset_token();
        let delai = crate::recuperation_repo::delai_securite_heures();
        let demande = self
            .recup
            .confirmer(&hash_token(token.trim()), &annul_hash, &final_hash, delai)
            .await?
            .ok_or(EntDbError::RecuperationLienInvalide)?;
        let to = self.auth.resolve_user_email(demande.user_id).await?;
        let base = public_base_url.trim_end_matches('/');
        let quand = demande.effective_at.format("%d/%m/%Y à %H:%M UTC");
        let envoi = self
            .mailer
            .send(OutgoingMail {
                to: to.clone(),
                subject: "Récupération de passkey en cours — annulez si ce n'est pas vous".into(),
                body_text: format!(
                    "Bonjour,\n\nUne récupération de votre passkey est en cours. Ce n'est pas vous ? \
Annulez-la immédiatement :\n{base}/recuperation-passkey/annuler?token={annul_raw}\n\n\
Si c'est bien vous : à partir du {quand}, ouvrez ce lien pour supprimer vos passkeys et vous reconnecter \
par mot de passe (toutes vos sessions seront fermées ; si vous n'avez pas de mot de passe connu, utilisez \
« mot de passe oublié » ensuite) :\n{base}/recuperation-passkey/finaliser?token={final_raw}\n\n\
Ce dernier lien reste valable {} h après cette date. Une connexion réussie par passkey annule aussi la récupération.\n",
                    crate::recuperation_repo::FINALISATION_FENETRE_HEURES,
                ),
                attachments: vec![],
            })
            .await;
        if !matches!(&envoi, Ok(r) if self.envoi_reussi(r)) {
            let _ = self.recup.abandonner(demande.id).await;
            return Err(EntDbError::EnvoiImpossible);
        }
        self.audit_recuperation_passkey(
            demande.user_id,
            &to,
            "passkey_recuperation_en_attente",
            "Récupération de passkey demandée (délai de sécurité en cours)",
            json!({ "demande_id": demande.id.to_string(), "effective_at": demande.effective_at.to_rfc3339() }),
        )
        .await;
        Ok(demande.effective_at)
    }

    /// Annulation par le lien d'annulation (jeton distinct, usage unique).
    pub async fn annuler_recuperation_passkey(&self, token: &str) -> Result<(), EntDbError> {
        let user_id = self
            .recup
            .annuler_par_jeton(&hash_token(token.trim()))
            .await?
            .ok_or(EntDbError::RecuperationLienInvalide)?;
        let email = self.auth.resolve_user_email(user_id).await.unwrap_or_default();
        self.audit_recuperation_passkey(
            user_id,
            &email,
            "passkey_recuperation_annulee",
            "Récupération de passkey annulée",
            json!({ "motif": "lien_annulation" }),
        )
        .await;
        Ok(())
    }

    /// Étape finale (vérifiée paresseusement) : si le délai est écoulé, supprime
    /// les passkeys de la personne et révoque toutes ses sessions. Avant la fin
    /// du délai : `RecuperationDelaiEnCours` (le jeton n'est pas consommé).
    pub async fn finaliser_recuperation_passkey(&self, token: &str) -> Result<(), EntDbError> {
        match self.recup.finaliser(&hash_token(token.trim())).await? {
            crate::recuperation_repo::Finalisation::Invalide => {
                Err(EntDbError::RecuperationLienInvalide)
            }
            crate::recuperation_repo::Finalisation::DelaiEnCours(t) => {
                Err(EntDbError::RecuperationDelaiEnCours(t))
            }
            crate::recuperation_repo::Finalisation::Terminee {
                user_id,
                passkeys,
                sessions,
            } => {
                let email = self.auth.resolve_user_email(user_id).await.unwrap_or_default();
                self.audit_recuperation_passkey(
                    user_id,
                    &email,
                    "passkey_recuperation_finalisee",
                    "Passkeys supprimées par récupération en libre-service",
                    json!({ "passkeys_retirees": passkeys, "sessions_revoquees": sessions }),
                )
                .await;
                Ok(())
            }
        }
    }

    /// Purge les demandes de récupération closes/périmées (maintenance boot).
    pub async fn purger_recuperations_passkey(&self) -> Result<u64, EntDbError> {
        Ok(self.recup.purger().await?)
    }

    /// Autorise (ou refuse) la gestion des identifiants d'un compte par un
    /// tiers et renvoie le rôle de la cible dans l'établissement de l'acteur.
    ///
    /// L'identité (e-mail, mot de passe, passkeys) est **globale** : si la
    /// cible a une appartenance dans un autre établissement (tout rôle),
    /// l'acteur n'a aucun droit dessus (cloisonnement) → refus.
    pub(crate) async fn cible_compte_autorisee(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        target_user_id: Uuid,
        op: OperationCompte,
    ) -> Result<Role, EntDbError> {
        let actor = Role::parse(actor_role).ok_or(EntDbError::Forbidden)?;
        let permis_acteur = match op {
            OperationCompte::Email => {
                matches!(actor, Role::Admin | Role::Direction | Role::Administratif)
            }
            // Socle 6 : l'ancien contrôle de profil (Admin, Direction) est l'action
            // `personnels.acces.gerer` (même modèle par défaut) ; la hiérarchie ci-dessous reste.
            OperationCompte::Identifiants | OperationCompte::Acces => {
                self.a_action_etablissement(
                    etablissement_id,
                    actor_user_id,
                    actor_role,
                    Action::PersonnelsAccesGerer,
                )
                .await?
            }
        };
        if !permis_acteur {
            return Err(EntDbError::Forbidden);
        }
        if op != OperationCompte::Email && target_user_id == actor_user_id {
            return Err(EntDbError::Forbidden);
        }
        let memberships = self
            .auth
            .list_memberships_for_user(target_user_id)
            .await?;
        // 403 uniforme (jamais de 404 : on ne révèle pas l'appartenance).
        let Some(role_ici) = memberships
            .iter()
            .find(|(etab, _)| *etab == etablissement_id)
            .map(|(_, r)| r.clone())
        else {
            return Err(EntDbError::Forbidden);
        };
        let target = Role::parse(&role_ici).ok_or(EntDbError::Forbidden)?;
        // Identité globale : dès que la cible appartient à un AUTRE
        // établissement (quel que soit son rôle), aucun tiers n'a de droit
        // dessus ; seul le propriétaire agit sur son compte.
        // Exception : l'état d'un accès ne concerne que SON établissement (fermer l'accès ici ne
        // touche ni l'identité ni les autres établissements de la personne), et seulement un
        // personnel (les familles et les élèves relèvent des socles 3 et 4).
        if op == OperationCompte::Acces {
            if !target.is_staff() {
                return Err(EntDbError::Forbidden);
            }
        } else if memberships.iter().any(|(etab, _)| *etab != etablissement_id) {
            return Err(EntDbError::Forbidden);
        }
        let ok = match (op, actor) {
            // RF-01 : jamais d'un pair ; un niveau strictement inférieur seulement.
            // L'Administratif ne touche qu'aux comptes famille / élève (aucun compte de
            // personnel : c'est la voie de prise de compte décrite par la revue).
            (OperationCompte::Email, Role::Admin) => target != Role::Admin,
            (OperationCompte::Email, Role::Direction) => {
                !matches!(target, Role::Admin | Role::Direction)
            }
            (OperationCompte::Email, Role::Administratif) => {
                matches!(target, Role::Parent | Role::Eleve)
            }
            (_, Role::Admin) => true,
            // Tout autre détenteur de l'action (Direction, ou rôle paramétré) : jamais sur un pair
            // ni sur un niveau supérieur (anti-escalade).
            (OperationCompte::Identifiants | OperationCompte::Acces, _) => {
                !matches!(target, Role::Admin | Role::Direction)
            }
            _ => false,
        };
        if !ok {
            return Err(EntDbError::Forbidden);
        }
        Ok(target)
    }

    /// Journal d'une action de gestion de compte : **acteur réel**, jamais
    /// d'adresse en clair (le payload ne contient que des identifiants et des
    /// empreintes). Best-effort comme `audit_acces`.
    pub(crate) async fn audit_compte(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        action: &str,
        libelle: &str,
        mut payload: serde_json::Value,
    ) {
        if let Some(o) = payload.as_object_mut() {
            o.insert("role".into(), json!(actor_role));
        }
        if let Err(e) = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id,
                eleve_id: None,
                domaine: "auth".into(),
                action: action.into(),
                libelle: libelle.into(),
                payload,
                visible_famille: false,
                actor_user_id: Some(actor_user_id),
            })
            .await
        {
            tracing::error!(action, error = %e, "échec d'écriture du journal d'audit");
        }
    }

    // ---- SSO OIDC ----------------------------------------------------------

    /// Transport SSO courant (HTTP en production, simulé dans les tests).
    fn oidc_transport(&self) -> Arc<dyn OidcTransport> {
        self.oidc_transport
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Remplace le transport SSO (tests / labo). Prend `&self` : le PgEnt est
    /// partagé derrière un `Arc` par l'API.
    pub fn set_oidc_transport(&self, transport: Arc<dyn OidcTransport>) {
        *self
            .oidc_transport
            .write()
            .unwrap_or_else(|e| e.into_inner()) = transport;
    }

    /// Déclare (ou met à jour) un fournisseur SSO pour l'établissement.
    ///
    /// La configuration est **validée par découverte réelle** de l'IdP avant
    /// d'être enregistrée : issuer injoignable ou document incomplet → refus
    /// explicite, pas de fournisseur à moitié configuré. Réservé à
    /// Direction / Admin.
    pub async fn upsert_oidc_provider_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_role: &str,
        params: OidcProviderParams,
    ) -> Result<OidcProviderRow, EntDbError> {
        let actor = Role::parse(actor_role).ok_or(EntDbError::Forbidden)?;
        if !matches!(actor, Role::Admin | Role::Direction) {
            return Err(EntDbError::Forbidden);
        }
        let slug = params.slug.trim().to_lowercase();
        if slug.is_empty()
            || params.libelle.trim().is_empty()
            || params.issuer.trim().is_empty()
            || params.client_id.trim().is_empty()
            || params.client_secret.is_empty()
        {
            return Err(EntDbError::Oidc(OidcError::Discovery(
                "slug, libellé, issuer, client_id et client_secret sont requis".into(),
            )));
        }
        let issuer = params.issuer.trim().trim_end_matches('/').to_string();
        // Garde SSRF (A7) : https obligatoire hors labo/test, cibles internes refusées.
        // Le transport HTTP réel revalide chaque requête (DNS résolu, redirections).
        let policy = UrlPolicy::from_env();
        valider_url(&issuer, policy)?;
        let transport = self.oidc_transport();
        let discovery = discover(transport.as_ref(), &issuer).await?;
        for endpoint in [
            &discovery.authorization_endpoint,
            &discovery.token_endpoint,
            &discovery.jwks_uri,
        ] {
            valider_url(endpoint, policy)?;
        }
        let redirect_uri = params
            .redirect_uri
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty())
            .unwrap_or_else(oidc_default_redirect_uri);
        if policy.require_https && !redirect_uri.starts_with("https://") {
            return Err(EntDbError::Oidc(OidcError::Discovery(
                "redirect_uri : https obligatoire hors labo/test".into(),
            )));
        }
        let secret_chiffre = self.secrets()?.seal(&params.client_secret, OIDC_SECRET_AAD)?;
        let scopes = params
            .scopes
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "openid email profile".to_string());
        self.oidc
            .upsert_provider(OidcProviderUpsert {
                etablissement_id,
                slug: &slug,
                libelle: params.libelle.trim(),
                issuer: &discovery.issuer,
                client_id: params.client_id.trim(),
                client_secret: &secret_chiffre,
                scopes: &scopes,
                redirect_uri: &redirect_uri,
                authorization_endpoint: &discovery.authorization_endpoint,
                token_endpoint: &discovery.token_endpoint,
                jwks_uri: &discovery.jwks_uri,
            })
            .await?;
        self.audit_acces(
            etablissement_id,
            None,
            "",
            actor_role,
            "oidc_provider_config",
            &format!("Configuration SSO « {slug} » ({issuer})"),
        )
        .await;
        self.oidc
            .get_provider(etablissement_id, &slug)
            .await?
            .ok_or(EntDbError::OidcProviderInconnu)
    }

    /// Fournisseurs configurés (staff — la direction voit client_id et issuer).
    pub async fn list_oidc_providers_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_role: &str,
    ) -> Result<Vec<OidcProviderRow>, EntDbError> {
        let actor = Role::parse(actor_role).ok_or(EntDbError::Forbidden)?;
        if !actor.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.oidc.list_providers(etablissement_id).await?)
    }

    /// Fournisseurs actifs exposés à la page de connexion (aucune donnée
    /// sensible : slug + libellé seulement).
    pub async fn list_oidc_providers_public(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<OidcProviderPublic>, EntDbError> {
        Ok(self.oidc.list_public(etablissement_id).await?)
    }

    pub async fn delete_oidc_provider_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_role: &str,
        slug: &str,
    ) -> Result<bool, EntDbError> {
        let actor = Role::parse(actor_role).ok_or(EntDbError::Forbidden)?;
        if !matches!(actor, Role::Admin | Role::Direction) {
            return Err(EntDbError::Forbidden);
        }
        let removed = self
            .oidc
            .delete_provider(etablissement_id, &slug.trim().to_lowercase())
            .await?;
        if removed {
            self.audit_acces(
                etablissement_id,
                None,
                "",
                actor_role,
                "oidc_provider_suppression",
                &format!("Suppression du fournisseur SSO « {slug} »"),
            )
            .await;
        }
        Ok(removed)
    }

    /// Secret client en clair (mémoire seulement). Un secret hérité en clair est lu
    /// tel quel puis rechiffré aussitôt (au mieux : l'échec n'empêche pas la connexion).
    async fn oidc_secret_clair(&self, provider: &OidcProviderRow) -> Result<String, EntDbError> {
        let secrets = self.secrets()?;
        let clair = secrets.open(&provider.client_secret, OIDC_SECRET_AAD)?;
        if !ent_auth::is_sealed(&provider.client_secret) {
            if let Ok(sealed) = secrets.seal(&clair, OIDC_SECRET_AAD) {
                if self
                    .oidc
                    .set_client_secret_raw(provider.id, &sealed)
                    .await
                    .is_ok()
                {
                    tracing::info!(provider = %provider.slug, "secret client OIDC hérité rechiffré");
                }
            }
        }
        Ok(clair)
    }

    /// D3 : rechiffre toutes les valeurs encore en clair des champs sensibles
    /// (IBAN/BIC/titulaire, fiche santé, dossier psy) — idempotent, reprenable.
    /// Appelé au démarrage ; relançable à la main (voir `docs/ent-rust-prod-runbook.md`).
    pub async fn rechiffrer_champs_sensibles(
        &self,
    ) -> Result<crate::champs_chiffres::RapportRechiffrement, EntDbError> {
        // Fail-closed : sans clé valide, aucune opération.
        self.secrets()?;
        Ok(crate::champs_chiffres::rechiffrer_champs_sensibles(
            &self.pool,
            &crate::champs_chiffres::ChampsSensibles::depuis_env(),
        )
        .await?)
    }

    /// Rechiffre tous les secrets clients OIDC encore en clair (appel au démarrage).
    /// Renvoie le nombre de secrets rechiffrés.
    pub async fn rechiffrer_secrets_oidc(&self) -> Result<u64, EntDbError> {
        let secrets = self.secrets()?;
        let mut n = 0;
        for (id, stored) in self.oidc.list_plaintext_secret_ids().await? {
            let sealed = secrets.seal(&stored, OIDC_SECRET_AAD)?;
            self.oidc.set_client_secret_raw(id, &sealed).await?;
            n += 1;
        }
        Ok(n)
    }

    /// Démarre un parcours SSO : renvoie l'URL d'autorisation de l'IdP (le
    /// `state` OIDC est notre jeton opaque, consommé au retour).
    ///
    /// Aucune écriture de session ici : l'utilisateur n'est pas encore
    /// authentifié. Renvoie `(url, secret_de_liaison)` : l'appelant HTTP pose le
    /// secret dans un cookie `__Host-` court ; seul son hash est stocké avec le
    /// `state`, et `oidc_complete` l'exige (anti login-CSRF).
    pub async fn oidc_start(
        &self,
        etablissement_id: Uuid,
        slug: &str,
    ) -> Result<(String, String), EntDbError> {
        let provider = self
            .oidc
            .get_provider(etablissement_id, &slug.trim().to_lowercase())
            .await?
            .filter(|p| p.actif)
            .ok_or(EntDbError::OidcProviderInconnu)?;

        let discovery = OidcDiscovery {
            issuer: provider.issuer.clone(),
            authorization_endpoint: provider.authorization_endpoint.clone(),
            token_endpoint: provider.token_endpoint.clone(),
            jwks_uri: provider.jwks_uri.clone(),
            id_token_signing_alg_values_supported: vec!["RS256".to_string()],
        };
        let config = OidcProviderConfig {
            issuer: provider.issuer.clone(),
            client_id: provider.client_id.clone(),
            // Inutile pour l'URL d'autorisation : jamais déchiffré au `start`.
            client_secret: String::new(),
            scopes: provider
                .scopes
                .split_whitespace()
                .map(str::to_string)
                .collect(),
            redirect_uri: provider.redirect_uri.clone(),
        };

        let state_token = random_token();
        let binding_secret = random_token();
        let (code_verifier, code_challenge) = generate_pkce();
        let nonce = random_token();
        let url = build_authorization_url(
            &discovery,
            &config,
            &state_token,
            &nonce,
            &code_challenge,
        );

        let expires_at = Utc::now() + chrono::Duration::minutes(oidc_state_minutes());
        self.oidc.prune_expired_states().await?;
        self.oidc
            .insert_state(OidcStateInsert {
                token_hash: &hash_token(&state_token),
                provider_id: provider.id,
                etablissement_id,
                code_verifier: &code_verifier,
                nonce: &nonce,
                redirect_uri: &provider.redirect_uri,
                binding_hash: &hash_token(&binding_secret),
                expires_at,
            })
            .await?;
        Ok((url, binding_secret))
    }

    /// Termine le parcours SSO : vérifie l'`id_token` puis résout le **compte
    /// local**. Aucune création automatique : une identité SSO inconnue est
    /// refusée (un administrateur rattache le compte au préalable).
    ///
    /// Le SSO ne **contourne pas** un 2ᵉ facteur déjà enrôlé : si le compte a
    /// ≥ 1 passkey, une assertion est exigée après l'IdP (`PasskeyRequired`) —
    /// sinon il suffirait de choisir « SSO » pour esquiver sa passkey.
    pub async fn oidc_complete(
        &self,
        code: &str,
        state_token: &str,
        browser_binding: &str,
    ) -> Result<LoginOutcome, EntDbError> {
        // Le state est consommé d'abord (usage unique, même en cas de refus ensuite).
        let state = self
            .oidc
            .take_state(&hash_token(state_token))
            .await?
            .ok_or(EntDbError::OidcStateInvalide)?;
        // Liaison au navigateur qui a démarré le parcours (cookie `__Host-`) : un
        // `code` + `state` rejoués depuis un autre navigateur sont refusés avant
        // tout échange réseau. Comparaison en temps constant.
        let lie = match (state.binding_hash.as_deref(), browser_binding.trim()) {
            (Some(attendu), fourni) if !fourni.is_empty() => {
                ent_auth::ct_eq(&hash_token(fourni), attendu)
            }
            _ => false,
        };
        if !lie {
            tracing::warn!("SSO : retour refusé, state non lié à ce navigateur");
            return Err(EntDbError::OidcStateInvalide);
        }
        let provider = self
            .oidc
            .get_provider_by_id(state.provider_id)
            .await?
            .ok_or(EntDbError::OidcProviderInconnu)?;

        let discovery = OidcDiscovery {
            issuer: provider.issuer.clone(),
            authorization_endpoint: provider.authorization_endpoint.clone(),
            token_endpoint: provider.token_endpoint.clone(),
            jwks_uri: provider.jwks_uri.clone(),
            id_token_signing_alg_values_supported: vec!["RS256".to_string()],
        };
        let config = OidcProviderConfig {
            issuer: provider.issuer.clone(),
            client_id: provider.client_id.clone(),
            client_secret: self.oidc_secret_clair(&provider).await?,
            scopes: provider
                .scopes
                .split_whitespace()
                .map(str::to_string)
                .collect(),
            redirect_uri: state.redirect_uri.clone(),
        };
        let identity = exchange_authorization_code(
            self.oidc_transport().as_ref(),
            &discovery,
            &config,
            code,
            &state.code_verifier,
            &state.nonce,
            Utc::now().timestamp(),
        )
        .await?;

        // Résolution du compte local : d'abord le rattachement stable (sub),
        // puis — à la **première** connexion — l'e-mail si un membre de
        // l'établissement correspond. Sinon : refus (pas de provisioning
        // implicite, donc pas de rôle inventé).
        let user_id = match self
            .oidc
            .find_identity(provider.id, &identity.subject)
            .await?
        {
            Some(user_id) => user_id,
            None => {
                // Rattachement par e-mail : l'IdP doit AFFIRMER que l'e-mail est vérifié
                // (`email_verified == true`). Absent ou faux → refus : sinon un compte
                // IdP à e-mail non vérifié pourrait usurper un compte local.
                if identity.email_verified != Some(true) {
                    tracing::warn!(provider = %provider.slug, "SSO : e-mail non vérifié, rattachement refusé");
                    return Err(EntDbError::Oidc(OidcError::EmailNotVerified));
                }
                let email = identity
                    .email
                    .clone()
                    .ok_or(EntDbError::OidcCompteInconnu)?;
                let candidate = self
                    .auth
                    .find_user_id_by_email(&email)
                    .await?
                    .ok_or(EntDbError::OidcCompteInconnu)?;
                // RF-01 : pas de rattachement par e-mail pendant le gel qui suit un
                // changement d'adresse fait par un tiers.
                if self.email_change.est_gele(candidate).await? {
                    return Err(EntDbError::OidcCompteInconnu);
                }
                self.oidc
                    .link_identity(provider.id, &identity.subject, candidate)
                    .await?;
                candidate
            }
        };

        // Non-membre : même réponse que « compte inconnu » (pas d'oracle
        // d'appartenance inter-établissements, C1/D-013).
        if !self.auth.is_member(state.etablissement_id, user_id).await? {
            return Err(EntDbError::OidcCompteInconnu);
        }
        let role = self
            .auth
            .role_in(state.etablissement_id, user_id)
            .await?
            .ok_or(EntDbError::OidcCompteInconnu)?;
        let email = self
            .auth
            .resolve_user_email(user_id)
            .await
            .unwrap_or_default();

        // Un 2ᵉ facteur déjà enrôlé reste obligatoire après le SSO : on ne
        // laisse pas un compte à passkey esquiver sa clé en passant par l'IdP.
        let passkeys = self
            .passkeys
            .list_passkeys(user_id)
            .await?;
        if !passkeys.is_empty() {
            let (token, options, expires_at) = self
                .open_authentication_challenge(
                    state.etablissement_id,
                    user_id,
                    &passkeys,
                    ClientSession::Web,
                )
                .await?;
            self.audit_acces(
                state.etablissement_id,
                Some(user_id),
                &email,
                &role,
                "connexion_sso_2fa",
                &format!("SSO « {} » validé — passkey exigée", provider.slug),
            )
            .await;
            return Ok(LoginOutcome::PasskeyRequired {
                token,
                options,
                expires_at,
            });
        }

        let portee = if self
            .passkey_obligatoire(state.etablissement_id, &role)
            .await?
        {
            PorteeSession::Enrolement
        } else {
            PorteeSession::Complete
        };
        let session = self
            .auth
            .issue_session(
                user_id,
                state.etablissement_id,
                &email,
                &role,
                &OptionsSession {
                    client: ClientSession::Web,
                    assurance: AssuranceSession::Sso,
                    portee,
                    appareil: None,
                    ip: None,
                },
            )
            .await?;
        self.audit_acces(
            state.etablissement_id,
            Some(user_id),
            &email,
            &role,
            "connexion_sso",
            &format!("Connexion SSO via « {} »", provider.slug),
        )
        .await;
        Ok(LoginOutcome::Session(session))
    }

    /// Provisionne un compte **personnel** (staff) dans l'établissement de la
    /// session. La création libre étant fermée en prod, c'est le chemin
    /// d'ouverture de comptes staff — réservé à Direction / Admin. Seul un
    /// Admin peut créer un Admin / une Direction.
    ///
    /// Socle 6, lot P0 : **plus de mot de passe temporaire**. Le compte est créé avec un secret
    /// aléatoire que personne ne connaît (ni renvoyé, ni journalisé), et la personne reçoit une
    /// invitation par e-mail : un lien à usage unique (7 jours) pour choisir son mot de passe, puis
    /// la politique « passkey obligatoire » (D-009) l'enrôle à la première connexion. Messagerie
    /// non prête ou URL publique absente → `EnvoiImpossible` **avant** toute création de compte.
    pub async fn provisionner_personnel_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        cmd: ProvisionnerPersonnel,
        public_base_url: &str,
    ) -> Result<PersonnelProvisionne, EntDbError> {
        let actor = Role::parse(actor_role).ok_or(EntDbError::Forbidden)?;
        // Socle 6 : l'ancien contrôle de profil (Admin, Direction) est l'action `personnels.gerer`
        // (même modèle par défaut, test d'oracle) : un rôle paramétré peut l'accorder à d'autres.
        if !self
            .a_action_etablissement(etablissement_id, actor_user_id, actor_role, Action::PersonnelsGerer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let target = Role::parse(&cmd.role).ok_or(EntDbError::Forbidden)?;
        if !target.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        if matches!(target, Role::Admin | Role::Direction) && actor != Role::Admin {
            return Err(EntDbError::Forbidden);
        }
        // Conflit dans CET établissement : explicite (409), même messagerie à l'arrêt.
        if self
            .auth
            .email_membre_de(etablissement_id, &cmd.email)
            .await?
        {
            return Err(AuthError::EmailTaken.into());
        }
        if public_base_url.trim().is_empty() || !self.mailer.is_ready() {
            return Err(EntDbError::EnvoiImpossible);
        }

        // Secret aléatoire jamais divulgué : il ne sert qu'à satisfaire le schéma (colonne non
        // nulle) ; la personne définit le sien par le lien d'invitation.
        let secret_inconnu = generate_temp_password();
        let user = match self
            .auth
            .register(
                &cmd.email,
                &secret_inconnu,
                cmd.display_name.as_deref(),
                etablissement_id,
                target.as_str(),
            )
            .await
        {
            Ok(u) => u,
            Err(AuthError::EmailTaken) => {
                // Adresse déjà connue. Membre de CET établissement : le
                // conflit est légitime (409). Connue ailleurs : réponse
                // **indiscernable d'un succès** (C1/D-013, pas d'oracle
                // d'existence inter-établissements) ; aucun compte n'est
                // créé ni modifié et aucun message n'est envoyé. Le rattachement
                // passera par une invitation acceptée par la personne (lot P2).
                if self
                    .auth
                    .email_membre_de(etablissement_id, &cmd.email)
                    .await?
                {
                    return Err(AuthError::EmailTaken.into());
                }
                // Même coût CPU qu'une création (Argon2id).
                let _ = ent_auth::hash_password(&secret_inconnu);
                let email = ent_auth::normalize_email(&cmd.email)?;
                let _ = self
                    .enregistrer_journal_interne(EnregistrerJournal {
                        etablissement_id,
                        eleve_id: None,
                        domaine: "etablissement".into(),
                        action: "personnel_provisionne_adresse_connue".into(),
                        libelle: "Provisionnement d'une adresse déjà utilisée ailleurs : \
                                  aucun compte créé (invitation à concevoir)"
                            .into(),
                        payload: json!({
                            "role": target.as_str(),
                            "par": actor_user_id.to_string(),
                        }),
                        visible_famille: false,
                        actor_user_id: Some(actor_user_id),
                    })
                    .await;
                return Ok(PersonnelProvisionne {
                    user_id: Uuid::new_v4(),
                    email,
                    role: target.as_str().to_string(),
                    invitation_envoyee: true,
                });
            }
            Err(e) => return Err(e.into()),
        };

        let _ = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id,
                eleve_id: None,
                domaine: "etablissement".into(),
                action: "personnel_provisionne".into(),
                libelle: format!(
                    "Compte personnel provisionné : {} ({})",
                    user.email,
                    target.as_str()
                ),
                payload: json!({
                    "email": user.email,
                    "role": target.as_str(),
                    "par": actor_user_id.to_string(),
                }),
                visible_famille: false,
                actor_user_id: Some(actor_user_id),
            })
            .await;

        // Invitation : lien à usage unique, 7 jours. Si l'envoi échoue malgré le contrôle
        // préalable, le compte existe sans que personne n'en connaisse le secret : l'acteur
        // renvoie l'invitation par la réinitialisation du mot de passe.
        self.envoyer_lien_reinitialisation(
            user.id,
            public_base_url,
            7 * 24 * 60,
            "Invitation à rejoindre l'ENT",
            "Un compte vient de vous être ouvert sur l'ENT de votre établissement.\nPour choisir votre mot de passe et activer votre accès, ouvrez ce lien (valable 7 jours)",
        )
        .await?;
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "invitation_personnel_envoyee",
            "Invitation par e-mail envoyée à un nouveau membre du personnel",
            json!({ "cible_user_id": user.id.to_string(), "cible_role": target.as_str() }),
        )
        .await;

        Ok(PersonnelProvisionne {
            user_id: user.id,
            email: user.email,
            role: target.as_str().to_string(),
            invitation_envoyee: true,
        })
    }

    /// Réinitialisation du mot de passe d'un compte par un tiers habilité
    /// (Direction / Admin, hiérarchie : la Direction ne touche ni un Admin ni
    /// une autre Direction ; jamais soi-même, voir `POST /auth/password`).
    ///
    /// Socle 6, lot P0 : **aucun secret n'est produit ni renvoyé**, pour aucun profil. Un lien de
    /// réinitialisation à usage unique (30 minutes) est envoyé à l'adresse du compte. Pour un
    /// personnel, les sessions de la cible sont en plus révoquées (mesure conservatoire). Messagerie
    /// non prête → `EnvoiImpossible`.
    pub async fn reinitialiser_mot_de_passe_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        target_user_id: Uuid,
        public_base_url: &str,
    ) -> Result<ReinitialisationMdp, EntDbError> {
        let target = self
            .cible_compte_autorisee(
                etablissement_id,
                actor_user_id,
                actor_role,
                target_user_id,
                OperationCompte::Identifiants,
            )
            .await?;

        // Le lien serait refusé pendant le gel qui suit un changement d'e-mail par un tiers : on
        // le dit à l'acteur plutôt que d'envoyer un lien inopérant.
        if self.email_change.est_gele(target_user_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let sessions = if target.is_staff() {
            self.auth.revoke_all_for_user(target_user_id).await?
        } else {
            0
        };
        self.envoyer_lien_reinitialisation(
            target_user_id,
            public_base_url,
            30,
            "Réinitialisation de votre mot de passe ENT",
            "Un membre de l'établissement a demandé la réinitialisation de votre mot de passe.\nPour en choisir un nouveau, ouvrez ce lien (valable 30 minutes)",
        )
        .await?;
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "lien_reinitialisation_envoye",
            "Lien de réinitialisation envoyé",
            json!({
                "cible_user_id": target_user_id.to_string(),
                "cible_role": target.as_str(),
                "sessions_revoquees": sessions,
            }),
        )
        .await;
        Ok(ReinitialisationMdp::LienEnvoye)
    }

    /// Un reçu vaut « envoyé » seulement si le statut est `Sent` ; `DryRun`
    /// n'est accepté qu'en build `labo` ou après `install_mail_dry_run` (tests),
    /// jamais en production : sinon un échec / une config absente produirait un
    /// faux « lien envoyé ».
    /// Même règle pour un SMS (lot A2 : code de vérification d'un mobile).
    pub(crate) fn sms_envoi_reussi(&self, receipt: &ent_sms::SmsReceipt) -> bool {
        match receipt.status {
            ent_sms::SmsSendStatus::Sent => true,
            ent_sms::SmsSendStatus::DryRun => self.mail_dry_run_accepte,
            ent_sms::SmsSendStatus::SkippedNoConfig | ent_sms::SmsSendStatus::Failed => false,
        }
    }

    pub(crate) fn envoi_reussi(&self, receipt: &MailReceipt) -> bool {
        match receipt.status {
            MailSendStatus::Sent => true,
            MailSendStatus::DryRun => self.mail_dry_run_accepte,
            MailSendStatus::SkippedNoConfig | MailSendStatus::Failed => false,
        }
    }

    /// Crée un jeton de réinitialisation à usage unique et envoie le lien.
    /// Strict : URL publique absente (`public_base_url` vide), messagerie non
    /// prête ou envoi non confirmé → `EnvoiImpossible`. Dans le dernier cas le
    /// jeton créé est invalidé (aucun lien orphelin actif).
    pub(crate) async fn envoyer_lien_reinitialisation(
        &self,
        user_id: Uuid,
        public_base_url: &str,
        ttl_minutes: i64,
        sujet: &str,
        intro: &str,
    ) -> Result<(), EntDbError> {
        if public_base_url.trim().is_empty() || !self.mailer.is_ready() {
            return Err(EntDbError::EnvoiImpossible);
        }
        let to = self.auth.resolve_user_email(user_id).await?;
        let (raw, hashed) = generate_reset_token();
        let expires_at = Utc::now() + chrono::Duration::minutes(ttl_minutes);
        self.auth
            .create_password_reset(user_id, &hashed, expires_at)
            .await?;
        let lien = format!(
            "{}/reset-mot-de-passe?token={}",
            public_base_url.trim_end_matches('/'),
            raw
        );
        let envoi = self
            .mailer
            .send(OutgoingMail {
                to,
                subject: sujet.into(),
                body_text: format!(
                    "Bonjour,\n\n{intro} :\n{lien}\n\nSi vous n'êtes pas concerné(e), ignorez ce message.\n"
                ),
                attachments: vec![],
            })
            .await;
        match envoi {
            Ok(r) if self.envoi_reussi(&r) => Ok(()),
            _ => {
                let _ = self.auth.invalidate_pending_resets(user_id).await;
                Err(EntDbError::EnvoiImpossible)
            }
        }
    }

    /// Purge les sessions expirées (maintenance au démarrage du conteneur).
    pub async fn purge_expired_sessions(&self) -> Result<u64, EntDbError> {
        Ok(self.auth.purge_expired_sessions().await?)
    }

    pub async fn resolve_session(&self, authorization: &str) -> Result<SessionInfo, EntDbError> {
        match self.auth.resolve_bearer(authorization).await {
            Ok(s) => Ok(s),
            Err(AuthError::InvalidSession) => {
                // `auth_repo` ramène aussi une panne base à « session invalide » :
                // on sonde la base pour ne pas déconnecter tout le monde pendant
                // une panne (500 au lieu de 401).
                self.ping().await?;
                Err(AuthError::InvalidSession.into())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn logout(&self, authorization: &str) -> Result<(), EntDbError> {
        // Résoudre avant révocation pour pouvoir tracer l'accès.
        let session = self.auth.resolve_bearer(authorization).await.ok();
        self.auth.revoke_bearer(authorization).await?;
        if let Some(s) = session {
            self.audit_acces(
                s.etablissement_id,
                Some(s.user_id),
                &s.email,
                &s.role,
                "deconnexion",
                "Déconnexion de l'ENT",
            )
            .await;
        }
        Ok(())
    }

    /// Changement de mot de passe par l'utilisateur lui-même. Les autres
    /// sessions de cet utilisateur sont révoquées (l'appelant garde la sienne).
    pub async fn change_password(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        keep_session_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), EntDbError> {
        self.auth
            .change_password(user_id, current_password, new_password)
            .await?;
        let revoked = self
            .auth
            .revoke_other_sessions_for_user(user_id, Some(keep_session_id))
            .await?;
        let email = self
            .auth
            .resolve_user_email(user_id)
            .await
            .unwrap_or_default();
        self.audit_acces(
            etablissement_id,
            Some(user_id),
            &email,
            "",
            "mot_de_passe_change",
            "Mot de passe modifié",
        )
        .await;
        let _ = revoked;
        Ok(())
    }

    /// Purge les jetons de réinitialisation expirés / consommés (maintenance boot).
    pub async fn purge_expired_password_resets(&self) -> Result<u64, EntDbError> {
        Ok(self.auth.prune_password_resets().await?)
    }

    /// Demande de réinitialisation de mot de passe : génère un jeton à usage
    /// unique (TTL 30 min) et envoie un lien par e-mail (dry-run ou SMTP live).
    ///
    /// Renvoie `true` si un compte correspondait (utile aux tests), mais
    /// **l'appelant HTTP ne doit pas divulguer ce booléen** : un e-mail inconnu
    /// ne produit ni jeton ni erreur (anti-énumération).
    pub async fn demander_reinitialisation_mot_de_passe(
        &self,
        email: &str,
        public_base_url: &str,
    ) -> Result<bool, EntDbError> {
        let Ok(Some(user_id)) = self.auth.find_user_id_by_email(email).await else {
            return Ok(false);
        };
        // RF-01 : gel de 72 h après un changement d'e-mail par un tiers (réponse neutre).
        if self.email_change.est_gele(user_id).await? {
            return Ok(false);
        }
        // Réponse HTTP neutre (202) quoi qu'il arrive : si l'URL publique est
        // absente, la messagerie non prête ou l'envoi non confirmé, aucun jeton
        // ne reste actif ; on journalise seulement (côté serveur).
        if let Err(e) = self
            .envoyer_lien_reinitialisation(
                user_id,
                public_base_url,
                30,
                "Réinitialisation de votre mot de passe ENT",
                "Pour choisir un nouveau mot de passe, ouvrez ce lien (valable 30 minutes)",
            )
            .await
        {
            tracing::warn!("mot de passe oublié : lien non envoyé ({e})");
        }

        // Audit best-effort pour chaque établissement de l'utilisateur.
        for (etab, role) in self
            .auth
            .list_memberships_for_user(user_id)
            .await
            .unwrap_or_default()
        {
            self.audit_acces(
                etab,
                Some(user_id),
                email,
                &role,
                "reset_demande",
                "Réinitialisation de mot de passe demandée",
            )
            .await;
        }
        Ok(true)
    }

    /// Applique une réinitialisation par jeton : vérifie le jeton (usage unique,
    /// non expiré), change le mot de passe et **révoque toutes les sessions**.
    pub async fn reinitialiser_mot_de_passe_par_token(
        &self,
        token: &str,
        new_password: &str,
    ) -> Result<(), EntDbError> {
        let hashed = hash_token(token.trim());
        let user_id = self
            .auth
            .consume_password_reset(&hashed, new_password)
            .await?;
        let email = self
            .auth
            .resolve_user_email(user_id)
            .await
            .unwrap_or_default();
        for (etab, role) in self
            .auth
            .list_memberships_for_user(user_id)
            .await
            .unwrap_or_default()
        {
            self.audit_acces(
                etab,
                Some(user_id),
                &email,
                &role,
                "mot_de_passe_reinitialise_token",
                "Mot de passe réinitialisé par lien",
            )
            .await;
        }
        Ok(())
    }

    /// Test-only : crée un jeton de réinitialisation et renvoie le jeton **brut**
    /// (jamais exposé par l'API HTTP). Réservé aux tests L1.
    #[cfg(all(test, feature = "labo"))]
    pub(crate) async fn reset_token_for_test(
        &self,
        email: &str,
    ) -> Result<Option<String>, EntDbError> {
        let Some(user_id) = self.auth.find_user_id_by_email(email).await? else {
            return Ok(None);
        };
        let (raw, hashed) = generate_reset_token();
        let expires_at = Utc::now() + chrono::Duration::minutes(30);
        self.auth
            .create_password_reset(user_id, &hashed, expires_at)
            .await?;
        Ok(Some(raw))
    }

    /// Trace un accès dans le journal (`domaine = auth`). Best-effort : un échec
    /// d'audit ne doit jamais faire échouer l'action métier.
    async fn audit_acces(
        &self,
        etablissement_id: Uuid,
        user_id: Option<Uuid>,
        email: &str,
        role: &str,
        action: &str,
        libelle: &str,
    ) {
        if let Err(e) = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id,
                eleve_id: None,
                domaine: "auth".into(),
                action: action.into(),
                libelle: libelle.into(),
                payload: json!({ "email": email, "role": role }),
                visible_famille: false,
                actor_user_id: user_id,
            })
            .await
        {
            // Ne fait jamais échouer l'action métier, mais ne se tait plus.
            tracing::error!(action, error = %e, "échec d'écriture du journal d'audit");
        }
    }

    /// Trace un accès sensible (consultation, téléchargement, export) dans
    /// `ent_journal_modif`. Contenu minimal : acteur, rôle, action, cible —
    /// **jamais** le contenu consulté. Best-effort : un échec d'écriture est
    /// journalisé (`tracing`) mais ne fait pas échouer la requête métier.
    pub async fn tracer_acces_sensible(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        domaine: &str,
        action: &str,
        libelle: &str,
    ) {
        if let Err(e) = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id,
                eleve_id,
                domaine: domaine.into(),
                action: action.into(),
                libelle: libelle.into(),
                payload: json!({ "role": role }),
                visible_famille: false,
                actor_user_id: Some(actor_user_id),
            })
            .await
        {
            tracing::error!(domaine, action, error = %e, "échec d'écriture de la trace d'accès sensible");
        }
    }

    /// Trace un échec de connexion ou un verrouillage. L'e-mail n'est **pas**
    /// écrit en clair : empreinte SHA-256 tronquée (pseudonymisation — permet de
    /// corréler les tentatives sur un même compte sans exposer l'adresse ni la
    /// frappe d'un mot de passe saisi par erreur dans le champ e-mail). Les
    /// verrous sont dédupliqués de façon atomique (un par compte et par 5 min)
    /// et limités aux membres de l'établissement, pour qu'un client anonyme ne puisse
    /// pas inonder le journal (ajout seul). SHA-256 sans secret serveur : l'empreinte
    /// est une pseudonymisation, pas une protection contre une attaque par dictionnaire
    /// sur des adresses connues.
    async fn tracer_echec_connexion(
        &self,
        etablissement_id: Uuid,
        email: &str,
        client_ip_connue: bool,
        err: &AuthError,
    ) {
        use sha2::{Digest, Sha256};
        let (action, libelle) = match err {
            AuthError::TooManyAttempts => ("connexion_verrou", "Connexion refusée : compte ou IP verrouillé"),
            AuthError::InvalidCredentials | AuthError::NotMember => {
                ("connexion_echec", "Échec de connexion")
            }
            _ => return,
        };
        if etablissement_id.is_nil() {
            return;
        }
        let email_norm = email.trim().to_ascii_lowercase();
        let digest = Sha256::digest(email_norm.as_bytes());
        let empreinte: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
        let payload = json!({ "empreinte": empreinte, "ip_connue": client_ip_connue });
        // Une seule transaction : verrou advisory par (établissement, action,
        // empreinte) puis INSERT … SELECT gardé. L'établissement n'a pas de table
        // dédiée : il « existe » s'il a au moins un membership, et l'identité doit
        // y être membre. Un client anonyme ne peut donc écrire que pour de vrais
        // comptes d'un vrai établissement, au plus une ligne par 5 minutes chacun.
        let res: Result<(), sqlx::Error> = async {
            let mut tx = self.pool.begin().await?;
            let cle = format!("{etablissement_id}:{action}:{empreinte}");
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
                .bind(&cle)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                r#"
                INSERT INTO ent_journal_modif (
                    id, etablissement_id, eleve_id, domaine, action, libelle,
                    payload, visible_famille, actor_user_id, created_at
                )
                SELECT $1, $2, NULL, 'auth', $3, $4, $5, false, NULL, now()
                WHERE EXISTS (
                    SELECT 1 FROM ent_membership m
                    JOIN ent_user u ON u.id = m.user_id
                    WHERE m.etablissement_id = $2 AND u.email = $6
                )
                AND NOT EXISTS (
                    SELECT 1 FROM ent_journal_modif j
                    WHERE j.etablissement_id = $2 AND j.domaine = 'auth' AND j.action = $3
                      AND j.payload->>'empreinte' = $7
                      AND j.created_at > now() - interval '5 minutes'
                )
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(etablissement_id)
            .bind(action)
            .bind(libelle)
            .bind(&payload)
            .bind(&email_norm)
            .bind(&empreinte)
            .execute(&mut *tx)
            .await?;
            tx.commit().await
        }
        .await;
        if let Err(e) = res {
            tracing::warn!(action, error = %e, "échec d'écriture de la trace de connexion");
        }
    }

    // --- Consultations sensibles tracées (domaine `consultation`) -----------
    // Chaque lecture autorisée de données de santé / psy laisse une ligne :
    // qui (acteur, rôle), quoi (action), sur quel élève. Pas de contenu.

    pub async fn get_fiche_sante_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<FicheSante>, EntDbError> {
        let r = self
            .get_fiche_sante_for_actor_sans_trace(etablissement_id, actor_user_id, role, eleve_id)
            .await?;
        self.tracer_acces_sensible(
            etablissement_id,
            actor_user_id,
            role,
            Some(eleve_id),
            "consultation",
            "fiche_sante",
            "Consultation de la fiche santé",
        )
        .await;
        Ok(r)
    }

    pub async fn get_pai_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<Pai>, EntDbError> {
        let r = self
            .get_pai_for_actor_sans_trace(etablissement_id, actor_user_id, role, eleve_id)
            .await?;
        self.tracer_acces_sensible(
            etablissement_id,
            actor_user_id,
            role,
            Some(eleve_id),
            "consultation",
            "pai",
            "Consultation du PAI",
        )
        .await;
        Ok(r)
    }

    pub async fn telecharger_pai_piece_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<PaiPiece, EntDbError> {
        let piece = self
            .telecharger_pai_piece_for_actor_sans_trace(
                etablissement_id,
                actor_user_id,
                role,
                piece_id,
            )
            .await?;
        self.tracer_acces_sensible(
            etablissement_id,
            actor_user_id,
            role,
            Some(piece.eleve_id),
            "consultation",
            "pai_piece",
            "Téléchargement d'une pièce du PAI",
        )
        .await;
        Ok(piece)
    }

    pub async fn list_passages_infirmerie_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<PassageInfirmerie>, EntDbError> {
        let r = self
            .list_passages_infirmerie_for_actor_sans_trace(
                etablissement_id,
                actor_user_id,
                role,
                eleve_id,
                jour,
            )
            .await?;
        self.tracer_acces_sensible(
            etablissement_id,
            actor_user_id,
            role,
            eleve_id,
            "consultation",
            "passages_infirmerie",
            "Consultation des passages à l'infirmerie",
        )
        .await;
        Ok(r)
    }

    pub async fn list_entretiens_psy_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<EntretienPsy>, EntDbError> {
        let r = self
            .list_entretiens_psy_for_actor_sans_trace(
                etablissement_id,
                actor_user_id,
                role,
                eleve_id,
                jour,
            )
            .await?;
        self.tracer_acces_sensible(
            etablissement_id,
            actor_user_id,
            role,
            eleve_id,
            "consultation",
            "entretiens_psy",
            "Consultation des entretiens psychologue",
        )
        .await;
        Ok(r)
    }

    pub async fn get_dossier_psy_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<DossierPsy>, EntDbError> {
        let r = self
            .get_dossier_psy_for_actor_sans_trace(etablissement_id, actor_user_id, role, eleve_id)
            .await?;
        self.tracer_acces_sensible(
            etablissement_id,
            actor_user_id,
            role,
            Some(eleve_id),
            "consultation",
            "dossier_psy",
            "Consultation du dossier psychologue",
        )
        .await;
        Ok(r)
    }

    /// Crée un élève : fait + périodes initiales + journal + événement dans UNE
    /// transaction (E0). Une erreur à n'importe quelle étape défait tout ; aucun
    /// `let _ =` sur le journal. Aucun état civil dans l'événement ni le journal.
    pub async fn create_eleve(&self, cmd: CreateEleve) -> Result<Eleve, EntDbError> {
        let actor = cmd.actor_user_id.clone();
        let planned = plan_create(cmd)?;
        let etab = planned.eleve.etablissement_id;
        let mut tx = TenantTx::begin(&self.pool, etab).await?;
        // Auteur de la version d'identité initiale (écrite par la base à l'insertion).
        if let Some(a) = actor.as_deref().filter(|a| Uuid::parse_str(a).is_ok()) {
            sqlx::query("SELECT set_config('ent.acteur_id', $1, true)")
                .bind(a)
                .execute(&mut *tx)
                .await?;
        }
        PgEleveRepo::upsert_sur(&mut tx, &planned.eleve).await?;
        for p in &planned.periodes_initiales {
            PgPeriodeRepo::ouvrir_premiere_sur(&mut tx, p).await?;
        }
        journal_eleve_tx(
            &mut tx,
            etab,
            planned.eleve.id,
            "eleve_cree",
            "Dossier élève créé",
            json!({ "eleve_id": planned.eleve.id }),
            &actor,
        )
        .await?;
        if let Some(event) = planned.event {
            tx.publier(event).await?;
        }
        tx.commit()
            .await
            ?;
        Ok(planned.eleve)
    }

    pub async fn change_classe(&self, cmd: ChangeClasse) -> Result<Eleve, EntDbError> {
        let (etab, eleve_id, date_effet) = (cmd.etablissement_id, cmd.eleve_id, cmd.date_effet);
        let classe = cmd.nouvelle_classe.clone();
        let eleve = self
            .change_attribut(ChangeAttribut {
                etablissement_id: cmd.etablissement_id,
                eleve_id: cmd.eleve_id,
                kind: AttributKind::Classe,
                nouvelle_valeur: cmd.nouvelle_classe,
                date_effet: cmd.date_effet,
                actor_user_id: cmd.actor_user_id,
            })
            .await?;
        // Référentiel scolaire (domaine propriétaire) : le champ texte reste la
        // compatibilité, l'affectation le suit. Un échec de synchronisation ne défait
        // pas un changement de classe déjà enregistré : il est journalisé, et
        // l'amorçage (idempotent) le signalera comme divergence.
        if let Err(e) = self
            .sync_affectation_classe(etab, eleve_id, &classe, date_effet)
            .await
        {
            tracing::warn!(error = %e, "synchronisation du référentiel après changement de classe");
        }
        Ok(eleve)
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

    /// Modifie l'identité : lecture verrouillée + écriture + journal + événement
    /// dans UNE transaction (E0). L'événement et le journal ne portent que les
    /// NOMS des champs modifiés, jamais leurs valeurs (minimisation).
    pub async fn update_identite(&self, cmd: UpdateIdentite) -> Result<Eleve, EntDbError> {
        let etab = cmd.etablissement_id;
        let actor = cmd.actor_user_id.clone();
        let mut tx = TenantTx::begin(&self.pool, etab).await?;
        let current = PgEleveRepo::get_pour_maj(&mut tx, etab, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let (eleve, changed) = plan_update_identite(&current, cmd)?;
        if !changed {
            return Ok(eleve);
        }
        let acteur = actor.as_deref().and_then(|s| Uuid::parse_str(s).ok());
        // Nom, prénom, sexe, date de naissance : version d'identité (correction), pas de
        // colonne écrasée. Elle publie et journalise elle-même (noms de champs seulement).
        PgEnt::correction_heritee(&mut tx, &current, &eleve, acteur).await?;
        let champs: Vec<&'static str> = champs_identite_modifies(&current, &eleve)
            .into_iter()
            .filter(|c| !matches!(*c, "nom" | "prenom" | "sexe" | "date_naissance"))
            .collect();
        PgEleveRepo::upsert_sur(&mut tx, &eleve).await?;
        if !champs.is_empty() {
            tx.publier(MetierEventDraft {
                etablissement_id: eleve.etablissement_id,
                type_name: types::ELEVE_IDENTITY_CHANGED.to_string(),
                aggregate: AggregateKind::Eleve,
                aggregate_id: eleve.id,
                eleve_id: Some(eleve.id),
                payload: json!({ "champs": champs }),
                actor_user_id: actor.clone(),
            })
            .await?;
            journal_eleve_tx(
                &mut tx,
                eleve.etablissement_id,
                eleve.id,
                "identite_modifiee",
                "Identité de l'élève modifiée",
                json!({ "champs": champs }),
                &actor,
            )
            .await?;
        }
        tx.commit()
            .await
            ?;
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
        niveau: NiveauIdentite,
    ) -> Result<Vec<Eleve>, EntDbError> {
        // Le badge n'est interrogeable que si le niveau l'expose (pas d'oracle).
        self.search_eleves_brut(etablissement_id, q, limit, niveau.voit_badge())
            .await
    }

    pub(crate) async fn search_eleves_brut(
        &self,
        etablissement_id: Uuid,
        q: &str,
        limit: i64,
        avec_badge: bool,
    ) -> Result<Vec<Eleve>, EntDbError> {
        Ok(self
            .eleves
            .search(etablissement_id, q, limit, avec_badge)
            .await?)
    }

    /// D-022 : libellés d'élèves ; un professeur n'obtient que ceux de son périmètre.
    pub async fn eleve_labels_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, EntDbError> {
        let mut ids = ids.to_vec();
        if Role::parse(role) == Some(Role::Prof) {
            if let Some(ok) = self
                .eleves_autorises(etablissement_id, actor_user_id, role, Action::DossierLire)
                .await?
            {
                ids.retain(|i| ok.contains(i));
            }
        }
        Ok(self.eleve_labels(etablissement_id, &ids).await?.into_iter().collect())
    }

    /// D-022 : pour un professeur, ne garde que les élèves qu'il suit (groupes compris).
    pub async fn restreindre_eleves_du_professeur(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleves: Vec<Eleve>,
    ) -> Result<Vec<Eleve>, EntDbError> {
        if Role::parse(role) != Some(Role::Prof) {
            return Ok(eleves);
        }
        match self
            .eleves_autorises(etablissement_id, actor_user_id, role, Action::DossierLire)
            .await?
        {
            None => Ok(eleves),
            Some(ok) => Ok(eleves.into_iter().filter(|e| ok.contains(&e.id)).collect()),
        }
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

    /// Libellés des utilisateurs **membres de l'établissement** (D-013) : un
    /// identifiant d'un autre établissement n'a aucun libellé.
    pub async fn user_labels(
        &self,
        etablissement_id: Uuid,
        user_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, EntDbError> {
        Ok(self.auth.user_labels(etablissement_id, user_ids).await?)
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
            let brut = self
                .foyers
                .list_liens_dossier(etablissement_id, eleve_id)
                .await?;
            // RP2-01 : même vue A1 que `GET /dossier/eleves/{id}/responsables` (aucune seconde règle).
            let admin = self
                .lecture_dossier_admin(etablissement_id, actor_user_id, role, eleve_id)
                .await?;
            crate::dossier_personnes::minimiser_liens_dossier(brut, &admin.responsables)
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

    pub(crate) async fn dossier_sections_for_actor(
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
        if Self::role_can_manage_eleves(role)
            || parsed == Role::Admin
            || parsed == Role::Direction
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
        if parsed == Role::Compta {
            // Minimisation (L2) : gestion/facturation uniquement — ni notes, ni vie
            // scolaire (sanctions, incidents, carnet), ni documents, dispositifs, options.
            return Ok(DossierSections {
                identite: true,
                scolarite: true,
                famille: true,
                options: false,
                notes: false,
                vs: false,
                documents: false,
                dispositifs: false,
                repas: true,
                bourses: true,
                continuum: false,
            });
        }
        // Socle 1 (B1) : hors de ses élèves, un professeur ne voit que l'identité
        // réduite et la scolarité (pas de notes, vie scolaire, famille, dispositifs).
        // D-017 : le droit « dossier.lire » (et sa portée) décide pour tout personnel.
        if parsed.is_staff()
            && !self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::DossierLire, eleve_id)
                .await?
        {
            return Ok(DossierSections {
                identite: true,
                scolarite: true,
                famille: false,
                options: true,
                notes: false,
                vs: false,
                documents: false,
                dispositifs: false,
                repas: false,
                bourses: false,
                continuum: false,
            });
        }
        if parsed.can_read_all_eleves() {
            let notes = self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::NotesLire, eleve_id)
                .await?;
            let vs = self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::AbsencesLire, eleve_id)
                .await?;
            // Prof / CPE / infirmière : docs filtrés (grants) ; bourses = finance → non.
            return Ok(DossierSections {
                identite: true,
                scolarite: true,
                famille: matches!(parsed, Role::Cpe | Role::Administratif | Role::Prof),
                options: true,
                notes,
                vs,
                documents: true,
                // RP1-20 : un professeur lit les aménagements (S2-11), jamais le type de
                // dispositif (PAP, PPS, PPRE, ULIS : donnée de santé / handicap).
                dispositifs: parsed != Role::Prof,
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
        let kind = cmd.kind.as_str();
        let kind_enum = cmd.kind;
        let nouvelle_valeur = cmd.nouvelle_valeur.clone();
        let date_effet = cmd.date_effet;
        let actor = cmd.actor_user_id.clone();
        let etab = cmd.etablissement_id;
        let eleve_id = cmd.eleve_id;
        let planned = plan_cutover(&current, open.as_ref(), cmd, today_utc())?;
        let changed = match kind_enum {
            AttributKind::Classe => current.classe != planned.eleve.classe,
            AttributKind::Regime => current.regime != planned.eleve.regime,
            AttributKind::Mef => current.mef != planned.eleve.mef,
            AttributKind::Site => current.site != planned.eleve.site,
        };
        self.periodes.apply_cutover(etab, &planned.op).await?;
        self.eleves.upsert(&planned.eleve).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        // Traçabilité : toute bascule d'attribut daté laisse une entrée journal.
        if changed {
            let _ = self
                .enregistrer_journal_interne(EnregistrerJournal {
                    etablissement_id: etab,
                    eleve_id: Some(eleve_id),
                    domaine: "scolarite".into(),
                    action: format!("{kind}_change"),
                    libelle: format!("Changement {kind} → {nouvelle_valeur} au {date_effet}"),
                    payload: json!({
                        "kind": kind,
                        "nouvelle_valeur": nouvelle_valeur,
                        "date_effet": date_effet.to_string(),
                    }),
                    visible_famille: true,
                    actor_user_id: actor.and_then(|s| Uuid::parse_str(&s).ok()),
                })
                .await;
        }
        Ok(planned.eleve)
    }

    pub async fn change_status(&self, cmd: ChangeStatus) -> Result<Eleve, EntDbError> {
        let current = self
            .eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let nouveau = cmd.nouveau_status.as_str().to_string();
        let actor = cmd.actor_user_id.clone();
        let etab = cmd.etablissement_id;
        let eleve_id = cmd.eleve_id;
        let planned = plan_change_status(&current, cmd)?;
        self.eleves.upsert(&planned.eleve).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        let _ = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id: etab,
                eleve_id: Some(eleve_id),
                domaine: "scolarite".into(),
                action: "statut_change".into(),
                libelle: format!("Statut élève → {nouveau}"),
                payload: json!({ "statut": nouveau }),
                visible_famille: true,
                actor_user_id: actor.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
            })
            .await;
        // Une bascule de statut vers `Sorti` est une sortie : mêmes répercussions
        // transverses que le geste dédié `sortir` (une seule vérité pour l'état).
        if current.status != EleveStatus::Sorti && planned.eleve.status == EleveStatus::Sorti {
            let date_sortie = planned.eleve.date_sortie.unwrap_or_else(today_utc);
            self.appliquer_repercussions_sortie(etab, eleve_id, date_sortie, actor)
                .await?;
        }
        Ok(planned.eleve)
    }

    pub async fn sortir(&self, cmd: SortirEleve) -> Result<Eleve, EntDbError> {
        let current = self
            .eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_sortir(&current, cmd.clone(), today_utc())?;
        let etab = planned.eleve.etablissement_id;
        let eleve_id = planned.eleve.id;
        let actor_user_id = cmd.actor_user_id.clone();
        if let Some(fin) = planned.close_periodes_au {
            self.periodes
                .close_open_periodes(planned.eleve.etablissement_id, planned.eleve.id, fin)
                .await?;
        }
        self.eleves.upsert(&planned.eleve).await?;
        self.events.publish(planned.event).await?;

        // Sortie effective : répercussions transverses (cantine, internat, accès, docs).
        if planned.eleve.status == EleveStatus::Sorti {
            let date_sortie = planned.eleve.date_sortie.unwrap_or_else(today_utc);
            self.appliquer_repercussions_sortie(etab, eleve_id, date_sortie, actor_user_id.clone())
                .await?;
        }
        // Traçabilité : la sortie est un acte administratif journalisé.
        let _ = self
            .enregistrer_journal_interne(EnregistrerJournal {
                etablissement_id: etab,
                eleve_id: Some(eleve_id),
                domaine: "administratif".into(),
                action: "eleve_sorti".into(),
                libelle: match planned.eleve.date_sortie {
                    Some(d) => format!("Sortie élève au {d}"),
                    None => "Sortie élève".to_string(),
                },
                payload: json!({
                    "date_sortie": planned.eleve.date_sortie.map(|d| d.to_string()),
                    "motif_sortie": planned.eleve.motif_sortie,
                    "statut": planned.eleve.status.as_str(),
                }),
                visible_famille: true,
                actor_user_id: actor_user_id.and_then(|s| Uuid::parse_str(&s).ok()),
            })
            .await;
        Ok(planned.eleve)
    }

    /// Répercussions transverses d'une **sortie effective** (statut `Sorti`).
    ///
    /// Un élève qui quitte l'établissement doit être retiré de tous les flux
    /// vivants, **quel que soit le geste** qui a produit la sortie (`sortir` daté
    /// ou bascule de statut) :
    /// - cantine : grille repas fermée la veille de la sortie ;
    /// - internat : affectation de chambre close la veille de la sortie ;
    /// - accès : compte élève délié + sessions applicatives révoquées ;
    /// - documents à signer en attente annulés.
    ///
    /// Chaque étape est idempotente (les dépôts renvoient `0` / `None` quand il n'y
    /// a rien à fermer), mais l'appelant ne déclenche la cascade que sur une
    /// **transition** vers `Sorti`, pour ne pas republier d'événements à chaque
    /// écriture ultérieure.
    pub(crate) async fn appliquer_repercussions_sortie(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
        date_sortie: NaiveDate,
        actor_user_id: Option<String>,
    ) -> Result<(), EntDbError> {
        let veille = date_sortie - chrono::Duration::days(1);

        // Scolarité : les périodes ouvertes sont closes la veille de la sortie
        // (historique + effectif daté conservés). Idempotent (ne touche que les
        // périodes encore ouvertes).
        self.periodes
            .close_open_periodes(etab, eleve_id, veille)
            .await?;

        // Cantine : plus de consommable après la veille de la sortie.
        let grilles_closes = self.repas.close_grilles_eleve(etab, eleve_id, veille).await?;
        if grilles_closes > 0 {
            self.events
                .publish(MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types::ELEVE_GRILLE_REPAS_CLOSE.to_string(),
                    aggregate: AggregateKind::Eleve,
                    aggregate_id: eleve_id,
                    eleve_id: Some(eleve_id),
                    payload: json!({
                        "date_fin": veille.to_string(),
                        "grilles_closes": grilles_closes,
                    }),
                    actor_user_id: actor_user_id.clone(),
                })
                .await?;
        }

        // Internat : l'hébergement s'arrête la veille de la sortie (au plus tôt
        // le premier jour d'affectation, si celle-ci démarre le jour même).
        if let Some(affectation) = self
            .internat
            .get_affectation_active(etab, eleve_id, date_sortie)
            .await?
        {
            let fin_internat = veille.max(affectation.debut);
            let closed = plan_clore_affectation_internat(affectation, etab, fin_internat)?;
            self.internat.update_affectation_fin(&closed).await?;
            self.events
                .publish(MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types::INTERNAT_AFFECTATION.to_string(),
                    aggregate: AggregateKind::Internat,
                    aggregate_id: closed.id,
                    eleve_id: Some(eleve_id),
                    payload: json!({
                        "fin": fin_internat.to_string(),
                        "cloture": true,
                        "motif": "sortie_eleve",
                    }),
                    actor_user_id: actor_user_id.clone(),
                })
                .await?;
        }

        // Accès applicatif élève : délien + sessions révoquées.
        if let Some(user_id) = self.eleve_comptes.unlink_eleve(etab, eleve_id).await? {
            let sessions = self.auth.revoke_all_for_user(user_id).await.unwrap_or(0);
            self.events
                .publish(MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types::ELEVE_COMPTE_REVOQUE.to_string(),
                    aggregate: AggregateKind::Eleve,
                    aggregate_id: eleve_id,
                    eleve_id: Some(eleve_id),
                    payload: json!({
                        "user_id": user_id.to_string(),
                        "sessions_revoquees": sessions,
                    }),
                    actor_user_id: None,
                })
                .await?;
        }

        // Documents à signer en attente : plus de sens une fois l'élève parti.
        let docs_annules = self.docs_signer.annuler_pending_eleve(etab, eleve_id).await?;
        if docs_annules > 0 {
            self.events
                .publish(MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types::DOC_A_SIGNER.to_string(),
                    aggregate: AggregateKind::Eleve,
                    aggregate_id: eleve_id,
                    eleve_id: Some(eleve_id),
                    payload: json!({
                        "action": "annule_sortie",
                        "docs_annules": docs_annules,
                    }),
                    actor_user_id: actor_user_id.clone(),
                })
                .await?;
        }

        Ok(())
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
        let actor_user_id = cmd.actor_user_id;
        let planned = plan_justifier_absence(&current, cmd)?;
        self.absences.upsert(&planned.absence).await?;
        if let Some(event) = planned.event {
            self.events.publish(event).await?;
        }
        let absence = planned.absence;
        let eleve = self
            .eleves
            .get(absence.etablissement_id, absence.eleve_id)
            .await?;
        let (prenom, nom) = eleve
            .as_ref()
            .map(|e| (e.prenom.as_str(), e.nom.as_str()))
            .unwrap_or(("Élève", ""));
        let sujet = format!("Justificatif d'absence — {prenom} {nom}");
        let corps = format!(
            "{prenom} {nom} : un justificatif d'absence a été déposé ({motif}). À traiter dans la file VS.",
            motif = absence
                .motif_justif
                .as_deref()
                .unwrap_or("motif")
        );
        let _ = self
            .notify_staff_push(
                absence.etablissement_id,
                Some(absence.eleve_id),
                "absence_justif_soumise",
                &sujet,
                &corps,
                "absence",
                absence.id,
                Some(actor_user_id),
            )
            .await;
        Ok(absence)
    }

    pub async fn annuler_soumission_justificatif(
        &self,
        cmd: JustifierAbsence,
    ) -> Result<Absence, EntDbError> {
        let current = self
            .absences
            .get(cmd.etablissement_id, cmd.absence_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_annuler_soumission_justificatif(&current, cmd)?;
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

    pub async fn annuler_soumission_justificatif_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        absence_id: Uuid,
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
        self.annuler_soumission_justificatif(JustifierAbsence {
            etablissement_id,
            absence_id,
            motif_justif: String::new(),
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

    /// Justifier une absence (socle 1, B3) : vie scolaire (`can_trancher_justificatif`),
    /// responsable lié avec le droit de justifier, ou l'élève **majeur** lui-même.
    /// Un professeur constate l'absence (appel) mais ne la justifie pas.
    pub async fn actor_can_justifier_absence(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(false);
        };
        if parsed.can_trancher_justificatif() {
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
            return Ok(linked == Some(eleve_id)
                && self
                    .eleve_est_majeur(etablissement_id, eleve_id, today_utc())
                    .await?);
        }
        Ok(false)
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
        // D-017 : « absences.lire » et sa portée (professeur : ses classes par défaut).
        if parsed.is_staff() {
            return self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::AbsencesLire, eleve_id)
                .await;
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

    /// Socle 1 (B1) : garde unique du périmètre professeur. Pour un professeur,
    /// l'élève doit être l'un des siens ([`Self::prof_suit_eleve`]) ; sans effet
    /// pour les autres rôles (leur propre contrôle s'applique ensuite).
    pub async fn exiger_perimetre_prof(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        if Role::parse(role).is_some_and(|r| r.is_staff())
            && !self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::SuiviEleve, eleve_id)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    /// D-022 : les exports officiels (LSL, LSU/LSUN, Notanet, Parcoursup, ONDE) sont
    /// réservés aux administratifs. Action `exports.officiels` à l'établissement entier ;
    /// absente du modèle « professeur » (un rôle libre peut l'accorder).
    pub async fn exiger_export_officiel(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etablissement_id, actor_user_id, role, Action::ExportsOfficiels)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// RP1-03 / RP1-07 (D-022) : élèves qu'un personnel peut **suivre** (`suivi_eleve`).
    /// `None` = aucune restriction (rôles « établissement ») ; `Some(ids)` = seulement ceux-là
    /// (professeur : ses classes et ses groupes ; comptabilité : aucun). Listes de classe,
    /// effectif et trombinoscope s'y bornent. Non-personnel : `None` (leurs routes ont leur règle).
    pub async fn eleves_suivis_restreints(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Option<std::collections::HashSet<Uuid>>, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Ok(None);
        }
        self.eleves_autorises(etablissement_id, actor_user_id, role, Action::SuiviEleve)
            .await
    }

    /// RF-04 (d à g) : le périmètre « mes classes » appliqué à une CLASSE entière. Un
    /// professeur (ou tout rôle dont `suivi_eleve` est borné) n'agit que sur les classes
    /// où il suit au moins un élève ; direction et rôles « établissement » gardent leur
    /// portée. `Ok(None)` = aucune restriction ; sinon l'ensemble des classes autorisées.
    pub async fn classes_dans_perimetre(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Option<std::collections::HashSet<String>>, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Ok(None);
        }
        let Some(eleves) = self
            .eleves_autorises(etablissement_id, actor_user_id, role, Action::SuiviEleve)
            .await?
        else {
            return Ok(None);
        };
        let ids: Vec<Uuid> = eleves.into_iter().collect();
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT classe FROM ent_eleve
              WHERE etablissement_id = $1 AND id = ANY($2) AND classe IS NOT NULL",
        )
        .bind(etablissement_id)
        .bind(&ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(Some(rows.into_iter().map(|(c,)| c).collect()))
    }

    /// Refuse si `classe` est hors du périmètre de l'acteur (voir `classes_dans_perimetre`).
    pub async fn exiger_classe_dans_perimetre(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        classe: &str,
    ) -> Result<(), EntDbError> {
        match self
            .classes_dans_perimetre(etablissement_id, actor_user_id, role)
            .await?
        {
            None => Ok(()),
            Some(cl) if cl.contains(classe.trim()) => Ok(()),
            Some(_) => Err(EntDbError::Forbidden),
        }
    }

    /// Peut savoir si l'élève est absent un jour donné : personnel, parent lié
    /// (statut seul, sans motif), élève lui-même.
    pub async fn actor_can_know_absent(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(false);
        };
        if parsed == Role::Prof {
            return self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::AbsencesLire, eleve_id)
                .await;
        }
        if parsed.is_staff() {
            return Ok(true);
        }
        if parsed == Role::Parent {
            return Ok(self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?
                .is_some());
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
        if !Self::role_can_manage_eleves(role) {
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
        // Profil inconnu : refus même pour sa propre fiche.
        Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let self_edit = actor_user_id == user_id;
        if !Self::role_can_manage_eleves(role) && !self_edit {
            return Err(EntDbError::Forbidden);
        }
        if !self.auth.is_member(etablissement_id, user_id).await? {
            return Err(DomainError::NotFound.into());
        }
        // Revue finale socle 1 (D-013) : le téléphone est porté par l'identité globale ;
        // un tiers d'un établissement ne le modifie pas pour une personne aussi membre
        // d'un autre établissement (ses SMS y partiraient vers le numéro choisi). Même
        // règle que `cible_compte_autorisee` pour l'e-mail.
        if !self_edit
            && self
                .auth
                .list_memberships_for_user(user_id)
                .await?
                .iter()
                .any(|(etab, _)| *etab != etablissement_id)
        {
            return Err(EntDbError::Forbidden);
        }
        self.set_user_telephone(user_id, telephone).await
    }

    /// Changement d'adresse e-mail (identité globale, donc strictement encadré) — RF-01.
    ///
    /// * soi-même : uniquement avec le mot de passe actuel (`current_password`) ;
    /// * tiers, **niveau strictement inférieur** (jamais un pair) : Admin → tout sauf
    ///   Admin ; Direction → tout sauf Admin et Direction ; Administratif → Parent, Élève.
    ///
    /// Le changement n'est **pas immédiat** : un lien de confirmation part vers la NOUVELLE
    /// adresse, l'ANCIENNE reçoit une alerte avec un lien d'annulation (qui rétablit
    /// l'ancienne adresse jusqu'à 72 h après la confirmation). Messagerie ou URL publique
    /// absente → `EnvoiImpossible`, rien n'est modifié. Un changement demandé par un tiers
    /// annule les récupérations de passkey en cours et gèle 72 h la réinitialisation de
    /// mot de passe et la récupération de passkey.
    pub async fn set_user_email_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        user_id: Uuid,
        email: &str,
        current_password: Option<&str>,
        public_base_url: &str,
    ) -> Result<(), EntDbError> {
        let tiers = actor_user_id != user_id;
        let target_role = if !tiers {
            if !self.auth.is_member(etablissement_id, user_id).await? {
                return Err(EntDbError::Forbidden);
            }
            self.verifier_mot_de_passe_actuel(
                etablissement_id,
                user_id,
                current_password.unwrap_or(""),
            )
            .await?;
            role.to_string()
        } else {
            self.cible_compte_autorisee(
                etablissement_id,
                actor_user_id,
                role,
                user_id,
                OperationCompte::Email,
            )
            .await?
            .as_str()
            .to_string()
        };
        if public_base_url.trim().is_empty() || !self.mailer.is_ready() {
            return Err(EntDbError::EnvoiImpossible);
        }
        // Plafond : pas de rafale de demandes (gel prolongé, courriels vers une adresse
        // arbitraire) sur une même cible.
        if self.email_change.demandes_recentes(user_id).await? >= 3 {
            return Err(AuthError::TooManyAttempts.into());
        }

        let ancien = self.auth.resolve_user_email(user_id).await?;
        let nouveau = ent_auth::normalize_email(email)?;
        if self.auth.find_user_id_by_email(&nouveau).await?.is_some() {
            return Err(AuthError::EmailTaken.into());
        }
        let (confirm_raw, confirm_hash) = generate_reset_token();
        let (annul_raw, annul_hash) = generate_reset_token();
        let id = self
            .email_change
            .creer(
                user_id,
                etablissement_id,
                actor_user_id,
                tiers,
                &ancien,
                &nouveau,
                &confirm_hash,
                &annul_hash,
            )
            .await?;
        let base = public_base_url.trim_end_matches('/');
        let envoi = self
            .mailer
            .send(OutgoingMail {
                to: nouveau.clone(),
                subject: "Confirmez votre nouvelle adresse e-mail ENT".into(),
                body_text: format!(
                    "Bonjour,\n\nCette adresse a été indiquée comme nouvelle adresse de connexion d'un compte ENT. \
Pour confirmer (lien valable {} minutes), ouvrez :\n{base}/email/confirmer?token={confirm_raw}\n\n\
Sans confirmation, rien ne sera modifié. Si vous n'êtes pas concerné(e), ignorez ce message.\n",
                    crate::email_change_repo::CONFIRMATION_TTL_MINUTES,
                ),
                attachments: vec![],
            })
            .await;
        if !matches!(&envoi, Ok(r) if self.envoi_reussi(r)) {
            let _ = self.email_change.abandonner(id).await;
            return Err(EntDbError::EnvoiImpossible);
        }
        if tiers {
            // Dès la demande : plus aucune récupération en cours ni lien de mot de passe actif.
            self.email_change.geler_et_annuler_recuperations(user_id).await?;
        }
        // Alerte de l'ancienne adresse avec lien d'annulation (best-effort).
        if ancien != nouveau {
            let _ = self
                .mailer
                .send(OutgoingMail {
                    to: ancien,
                    subject: "Changement d'adresse e-mail demandé sur votre compte ENT".into(),
                    body_text: format!(
                        "Bonjour,\n\nUn changement de l'adresse e-mail de votre compte ENT a été demandé. \
Ce n'est pas vous ? Annulez-le immédiatement (le lien reste actif {} h, y compris si le changement a déjà \
été confirmé : l'ancienne adresse sera alors rétablie) :\n{base}/email/annuler?token={annul_raw}\n\n\
Sans action de votre part, l'adresse changera dès que la nouvelle aura été confirmée.\n",
                        crate::email_change_repo::GEL_HEURES,
                    ),
                    attachments: vec![],
                })
                .await;
        }
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            role,
            "email_changement_demande",
            "Changement d'adresse e-mail d'un compte demandé (confirmation en attente)",
            json!({
                "cible_user_id": user_id.to_string(),
                "cible_role": target_role,
                "tiers": tiers,
                "ancienne_empreinte": empreinte_email(&self.auth.resolve_user_email(user_id).await?),
                "nouvelle_empreinte": empreinte_email(&nouveau),
            }),
        )
        .await;
        Ok(())
    }

    /// Confirmation par le lien reçu à la nouvelle adresse (public, le jeton fait foi) :
    /// applique le changement, révoque les sessions, annule récupérations et liens de mot
    /// de passe ; gel de 72 h si le changement vient d'un tiers.
    pub async fn confirmer_changement_email(&self, token: &str) -> Result<(), EntDbError> {
        let c = self
            .email_change
            .confirmer(&hash_token(token.trim()))
            .await?
            .ok_or(EntDbError::EmailLienInvalide)?;
        self.audit_compte(
            c.etablissement_id,
            c.actor_user_id,
            "",
            "email_modifie",
            "Adresse e-mail d'un compte modifiée (confirmée par la nouvelle adresse)",
            json!({
                "cible_user_id": c.user_id.to_string(),
                "tiers": c.tiers,
                "ancienne_empreinte": empreinte_email(&c.ancien_email),
                "nouvelle_empreinte": empreinte_email(&c.nouveau_email),
                "sessions_revoquees": c.sessions_revoquees,
            }),
        )
        .await;
        if self.mailer.is_ready() && c.ancien_email != c.nouveau_email {
            let _ = self
                .mailer
                .send(OutgoingMail {
                    to: c.ancien_email,
                    subject: "L'adresse e-mail de votre compte ENT a changé".into(),
                    body_text: "Bonjour,\n\nL'adresse e-mail de votre compte ENT vient d'être modifiée et toutes vos sessions ont été fermées. Si vous n'êtes pas à l'origine de ce changement, utilisez le lien d'annulation du message précédent ou contactez immédiatement l'établissement.\n".into(),
                    attachments: vec![],
                })
                .await;
        }
        Ok(())
    }

    /// Annulation par le lien reçu à l'ancienne adresse (public, le jeton fait foi) :
    /// annule la demande, ou rétablit l'ancienne adresse (gel de 72 h maintenu).
    pub async fn annuler_changement_email(&self, token: &str) -> Result<(), EntDbError> {
        let c = self
            .email_change
            .annuler(&hash_token(token.trim()))
            .await?
            .ok_or(EntDbError::EmailLienInvalide)?;
        self.audit_compte(
            c.etablissement_id,
            c.actor_user_id,
            "",
            if c.retabli { "email_retabli" } else { "email_changement_annule" },
            "Changement d'adresse e-mail annulé par l'ancienne adresse",
            json!({
                "cible_user_id": c.user_id.to_string(),
                "retabli": c.retabli,
                "sessions_revoquees": c.sessions_revoquees,
            }),
        )
        .await;
        Ok(())
    }

    /// Purge les demandes de changement d'e-mail anciennes (maintenance boot).
    pub async fn purger_changements_email(&self) -> Result<u64, EntDbError> {
        Ok(self.email_change.purger().await?)
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
        public_base_url: &str,
        lien: &str,
        peut_voir_notes: bool,
        peut_voir_vs: bool,
        peut_justifier_absence: bool,
        peut_autoriser_sorties: bool,
        est_payeur: bool,
        part_paiement: Option<i16>,
    ) -> Result<(EleveLien, Option<bool>), EntDbError> {
        if !Self::role_can_manage_eleves(role) {
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

        // RP2-03 : une personne déjà connue pour cet e-mail est réutilisée (jamais de seconde fiche)
        // et sa marque « ne pas solliciter » est respectée : ni compte ouvert, ni activation.
        let existante = self.personne_par_email(etablissement_id, email).await?;
        if existante.is_some_and(|p| p.ne_pas_solliciter) {
            return Err(EntDbError::ConflitDossier(
                "cette personne ne doit pas être sollicitée : aucun compte n'est ouvert par ce chemin ; \
                 utiliser l'ajout de responsable du dossier (lien sans compte)",
            ));
        }
        let mut compte_cree = false;
        let user_id = if let Some(uid) = self
            .auth
            .find_member_id_by_email(etablissement_id, email)
            .await?
        {
            uid
        } else if let Some(uid) = existante.and_then(|p| p.user_id) {
            uid
        } else {
            // Jamais de mot de passe choisi par un tiers : le compte reçoit un
            // secret aléatoire que personne ne connaît ; le responsable définit
            // le sien via le lien d'activation (flux « mot de passe oublié »).
            let pwd_alea = ent_auth::generate_temp_password();
            let created = self
                .register_user(
                    email,
                    pwd_alea.as_str(),
                    display_name,
                    etablissement_id,
                    Role::Parent.as_str(),
                )
                .await?;
            compte_cree = true;
            created.id
        };

        if let Some(tel) = telephone {
            let t = tel.trim();
            if !t.is_empty() {
                self.set_user_telephone(user_id, t).await?;
            }
        }
        if let Some(p) = existante.filter(|p| p.user_id.is_none()) {
            self.rattacher_compte_a_la_fiche(etablissement_id, p.id, user_id)
                .await?;
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
        // Nouveau compte : lien d'activation tenté automatiquement. Jamais
        // d'erreur ici (le rattachement est déjà fait) ; l'issue est renvoyée :
        // `Some(true)` envoyé, `Some(false)` non envoyé (URL publique absente,
        // messagerie indisponible ou en échec : jeton invalidé, le parent
        // utilisera « mot de passe oublié »), `None` si aucun compte créé.
        let activation = if compte_cree {
            Some(
                self.envoyer_lien_reinitialisation(
                    user_id,
                    public_base_url,
                    72 * 60,
                    "Activation de votre compte ENT",
                    "Un compte ENT vient d'être créé pour vous. Pour choisir votre mot de passe, ouvrez ce lien (valable 72 heures)",
                )
                .await
                .is_ok(),
            )
        } else {
            None
        };
        Ok((saved, activation))
    }

    pub async fn get_foyer_detail_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        foyer_id: Uuid,
    ) -> Result<Option<(Uuid, Option<String>, Vec<LienDossierFoyer>)>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(Self::role_can_manage_eleves(role) || matches!(parsed, Role::Compta | Role::Cpe)) {
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
        if !(Self::role_can_manage_eleves(role) || matches!(parsed, Role::Compta | Role::Cpe)) {
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
        if !Self::role_can_manage_eleves(role) {
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
        if !Self::role_can_manage_eleves(role) {
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
        if !Self::role_can_manage_eleves(role) {
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

    pub(crate) fn infirmerie_repo(&self) -> &PgInfirmerieRepo {
        &self.infirmerie
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
        if parsed.is_staff() {
            // E6 (§7.10) : une fiche absorbée par une fusion redirige vers la fiche conservée ;
            // les droits sont évalués sur la fiche qui répond.
            let eleve_id = self.eleve_effectif(etablissement_id, eleve_id).await?;
            // E1 : l'annuaire (désignation) est une action du catalogue, avec sa portée.
            if !self
                .autoriser_eleve(
                    etablissement_id,
                    actor_user_id,
                    role,
                    Action::ElevesAnnuaire,
                    eleve_id,
                )
                .await?
            {
                return Ok(None);
            }
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
        Self::peut_etablissement(role, Action::ElevesGerer)
    }

    /// Sauvegardes / PRA : seule la Direction (ou un admin plateforme) peut
    /// lire un inventaire d'intégrité ou vérifier une restauration.
    pub fn role_can_sauvegarde(role: &str) -> bool {
        matches!(Role::parse(role), Some(Role::Admin) | Some(Role::Direction))
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
        Self::peut_etablissement(role, Action::MatieresGerer)
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
            if linked != Some(eleve_id) {
                return Ok(false);
            }
            // RF-22 : un élève mineur ne déclare pas sa propre absence (cf. B3).
            return self
                .eleve_est_majeur(etablissement_id, eleve_id, today_utc())
                .await;
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
        // D-017 : « notes.lire » et sa portée (professeur : ses classes par défaut).
        if parsed.is_staff() {
            return self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::NotesLire, eleve_id)
                .await;
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

    pub async fn update_matiere_libelle(
        &self,
        etablissement_id: Uuid,
        matiere_id: Uuid,
        libelle: String,
        actor_user_id: Option<String>,
    ) -> Result<Matiere, EntDbError> {
        let libelle = libelle.trim().to_string();
        if libelle.is_empty() {
            return Err(EntDbError::MatiereInvalide);
        }
        let updated = self
            .matieres
            .update_libelle(etablissement_id, matiere_id, &libelle)
            .await?;
        let matiere = updated.ok_or(EntDbError::Domain(DomainError::NotFound))?;
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
                    "updated": true,
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
        // Revue finale socle 1 : même portée que la saisie de note (`notes.saisir`,
        // professeur : ses élèves) ; avant, tout professeur écrivait l'appréciation de
        // n'importe quel élève de l'établissement.
        if !self
            .autoriser_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::NotesSaisir,
                cmd.eleve_id,
            )
            .await?
        {
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
        if !(Self::role_can_manage_eleves(role) || parsed.can_saisir_notes() || parsed == Role::Cpe) {
            return Err(EntDbError::Forbidden);
        }
        // RF-04 (e) : un professeur ne publie que le bulletin d'un élève qu'il suit.
        self.exiger_perimetre_prof(etablissement_id, actor_user_id, role, eleve_id)
            .await?;
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

        // Répercussion famille : un bulletin publié est communiqué aux responsables.
        let _ = self
            .notify_parents_mail(
                etablissement_id,
                eleve_id,
                "bulletin",
                &format!(
                    "Bulletin {} — {} {}",
                    periode.libelle, eleve.prenom, eleve.nom
                ),
                &format!(
                    "Le bulletin {} de {} {} est disponible dans le portail famille.",
                    periode.libelle, eleve.prenom, eleve.nom
                ),
                "document",
                doc.id,
                Some(actor_user_id),
                Vec::new(),
            )
            .await;
        let _ = self
            .notify_parents_push(
                etablissement_id,
                eleve_id,
                "bulletin",
                "Nouveau bulletin disponible",
                &format!("Bulletin {} de {}", periode.libelle, eleve.prenom),
                "document",
                doc.id,
                Some(actor_user_id),
            )
            .await;
        Ok(doc)
    }

    // ---- Compétences / LSU ---------------------------------------------------

    /// Seed le référentiel de compétences (socle commun) + évaluations T1 d'un élève.
    /// Défaut **éditable** (données établissement), pas une règle nationale figée.
    pub async fn seed_competences_socle(
        &self,
        etablissement_id: Uuid,
        periode_id: Uuid,
        eleve_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<(), EntDbError> {
        const SOCLE: &[(&str, &str, &str)] = &[
            ("D1", "D1.1", "Comprendre et s'exprimer à l'oral"),
            ("D1", "D1.2", "Comprendre et s'exprimer à l'écrit"),
            ("D2", "D2.1", "Organiser son travail personnel"),
            ("D2", "D2.2", "Chercher et traiter l'information"),
            ("D3", "D3.1", "Faire preuve de responsabilité et d'initiative"),
            ("D3", "D3.2", "Respecter autrui et les règles de la vie commune"),
            ("D4", "D4.1", "Démarrer une démarche scientifique"),
            ("D4", "D4.2", "Comprendre les systèmes techniques"),
            ("D5", "D5.1", "Se repérer dans le temps et l'espace"),
            ("D5", "D5.2", "Formuler une opinion argumentée"),
        ];
        for (ordre, (domaine, code, libelle)) in SOCLE.iter().enumerate() {
            let item = plan_creer_competence_item(CreerCompetenceItem {
                etablissement_id,
                domaine: (*domaine).to_string(),
                code: (*code).to_string(),
                libelle: (*libelle).to_string(),
                ordre: Some(ordre as i32),
            })?;
            self.competences.upsert_item(&item).await?;
        }
        let items = self.competences.list_items(etablissement_id, false).await?;
        for (i, item) in items.iter().enumerate() {
            let niveau = match i % 4 {
                0 => NiveauMaitrise::TresBonne,
                1 | 2 => NiveauMaitrise::Satisfaisante,
                _ => NiveauMaitrise::Fragile,
            };
            let evaluation = EvaluationCompetence {
                id: Uuid::new_v4(),
                etablissement_id,
                eleve_id,
                item_id: item.id,
                periode_id,
                niveau,
                commentaire: (i == 0)
                    .then(|| "Participation orale remarquée en Potions.".to_string()),
                auteur_user_id: actor_user_id,
            };
            self.competences.upsert_evaluation(&evaluation).await?;
        }
        Ok(())
    }

    /// Crée / met à jour un item du référentiel de compétences (staff pédagogique).
    pub async fn creer_competence_item_for_actor(
        &self,
        role: &str,
        cmd: CreerCompetenceItem,
    ) -> Result<CompetenceItem, EntDbError> {
        if !(Self::role_can_manage_matieres(role) || Self::role_can_manage_eleves(role)) {
            return Err(EntDbError::Forbidden);
        }
        let item = plan_creer_competence_item(cmd)?;
        self.competences.upsert_item(&item).await?;
        Ok(item)
    }

    /// Référentiel de compétences de l'établissement (staff notes).
    pub async fn list_competences_items_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        inclure_inactifs: bool,
    ) -> Result<Vec<CompetenceItem>, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.can_read_notes_staff()) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .competences
            .list_items(etablissement_id, inclure_inactifs)
            .await?)
    }

    /// Évalue (ou ré-évalue) un item du socle pour un élève sur une période.
    pub async fn evaluer_competence_for_actor(
        &self,
        role: &str,
        cmd: EvaluerCompetence,
    ) -> Result<EvaluationCompetence, EntDbError> {
        // D-017 : « competences.evaluer » et sa portée.
        if !Role::parse(role).is_some_and(|r| r.is_staff())
            || !self
                .autoriser_eleve(
                    cmd.etablissement_id,
                    cmd.actor_user_id,
                    role,
                    Action::CompetencesEvaluer,
                    cmd.eleve_id,
                )
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let periode = self
            .notes
            .get_periode(cmd.etablissement_id, cmd.periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !periode.statut.autorise_appreciations() {
            return Err(DomainError::NoteAppreciationPeriodeFermee.into());
        }
        if self
            .competences
            .get_item(cmd.etablissement_id, cmd.item_id)
            .await?
            .is_none()
        {
            return Err(DomainError::CompetenceInvalide.into());
        }
        let planned = plan_evaluer_competence(cmd)?;
        self.competences.upsert_evaluation(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::COMPETENCE_EVALUEE.to_string(),
                aggregate: AggregateKind::Competence,
                aggregate_id: planned.item_id,
                eleve_id: Some(planned.eleve_id),
                payload: json!({
                    "periode_id": planned.periode_id,
                    "niveau": planned.niveau.as_str(),
                    "code_lsu": planned.niveau.code_lsu(),
                }),
                actor_user_id: Some(planned.auteur_user_id.to_string()),
            })
            .await?;
        Ok(planned)
    }

    /// Évaluations de compétences d'un élève (staff ou famille liée).
    pub async fn list_evaluations_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        periode_id: Option<Uuid>,
    ) -> Result<Vec<EvaluationLue>, EntDbError> {
        if !self
            .actor_can_read_notes(etablissement_id, user_id, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        // Isolation tenant : un staff d'un autre établissement ne lit pas cet élève.
        self.eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        Ok(self
            .competences
            .list_evaluations_eleve(etablissement_id, eleve_id, periode_id)
            .await?)
    }

    /// Publie le **livret de compétences** (PDF) — déposé côté famille + notifié.
    /// Refuse un livret vide : on n'officialise pas un document sans évaluation.
    pub async fn publier_livret_competences_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        periode_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<EleveDocument, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let gestion = Self::role_can_manage_eleves(role) || parsed == Role::Cpe;
        if !gestion
            && !(parsed.is_staff()
                && self
                    .autoriser_eleve(
                        etablissement_id,
                        actor_user_id,
                        role,
                        Action::CompetencesEvaluer,
                        eleve_id,
                    )
                    .await?)
        {
            return Err(EntDbError::Forbidden);
        }
        let eleve = self
            .eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let periode = self
            .notes
            .get_periode(etablissement_id, periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let evals = self
            .competences
            .list_evaluations_eleve(etablissement_id, eleve_id, Some(periode_id))
            .await?;
        if evals.is_empty() {
            return Err(DomainError::CompetenceInvalide.into());
        }
        let lignes: Vec<LigneLivretCompetence> = evals
            .iter()
            .map(|e| LigneLivretCompetence {
                domaine: e.domaine.clone(),
                item: e.item_libelle.clone(),
                niveau: Some(e.niveau),
                commentaire: e.commentaire.clone(),
            })
            .collect();
        let livret = LivretCompetences {
            eleve_prenom: eleve.prenom.clone(),
            eleve_nom: eleve.nom.clone(),
            classe: eleve.classe.clone(),
            periode_libelle: periode.libelle.clone(),
            periode_code: periode.code.clone(),
            date_document: today_utc(),
            lignes,
        };
        let bytes = render_livret_competences_pdf(&livret);
        let key = format!(
            "{}-{eleve_id}",
            ObjectStore::key_for(etablissement_id, "livret-competences", periode_id, "pdf")
        );
        self.objects.put(&key, &bytes)?;
        let doc = self
            .deposer_document(DeposerDocument {
                etablissement_id,
                eleve_id,
                tiroir: DocTiroir::Famille,
                titre: format!(
                    "Livret de compétences {} — {} {}",
                    periode.libelle, eleve.prenom, eleve.nom
                ),
                code: Some(format!("LIVRET_{}", periode.code)),
                mime_type: Some("application/pdf".into()),
                storage_key: Some(key.clone()),
                confidentialite: DocConfidentialite::Standard,
                actor_user_id,
            })
            .await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::COMPETENCE_EVALUEE.to_string(),
                aggregate: AggregateKind::Competence,
                aggregate_id: periode_id,
                eleve_id: Some(eleve_id),
                payload: json!({
                    "document_id": doc.id,
                    "storage_key": key,
                    "octets": bytes.len(),
                    "publication": "livret",
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        let _ = self
            .notify_parents_mail(
                etablissement_id,
                eleve_id,
                "livret_competences",
                &format!("Livret de compétences {} disponible", periode.libelle),
                &format!(
                    "Le livret de compétences {} de {} {} est disponible dans le portail famille.",
                    periode.libelle, eleve.prenom, eleve.nom
                ),
                "document",
                doc.id,
                Some(actor_user_id),
                Vec::new(),
            )
            .await;
        let _ = self
            .notify_parents_push(
                etablissement_id,
                eleve_id,
                "livret_competences",
                "Nouveau livret de compétences",
                &format!("Livret {} de {}", periode.libelle, eleve.prenom),
                "document",
                doc.id,
                Some(actor_user_id),
            )
            .await;
        Ok(doc)
    }

    /// Création d'un devoir par un acteur : rôle de saisie de notes et classe dans son
    /// périmètre (RF-04 g).
    pub async fn creer_devoir_for_actor(
        &self,
        role: &str,
        cmd: CreerDevoir,
    ) -> Result<Devoir, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_saisir_notes() {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_classe_dans_perimetre(cmd.etablissement_id, cmd.actor_user_id, role, &cmd.classe)
            .await?;
        self.creer_devoir(cmd).await
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

    pub async fn supprimer_devoir(
        &self,
        etablissement_id: Uuid,
        devoir_id: Uuid,
    ) -> Result<Devoir, EntDbError> {
        let devoir = self
            .notes
            .get_devoir(etablissement_id, devoir_id)
            .await?
            .ok_or(DomainError::DevoirIntrouvable)?;
        let periode = self
            .notes
            .get_periode(etablissement_id, devoir.periode_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !periode.statut.autorise_saisie_notes() {
            return Err(DomainError::NotePeriodeFermee.into());
        }
        let n_notes = self
            .notes
            .count_notes_devoir(etablissement_id, devoir_id)
            .await?;
        if n_notes > 0 {
            return Err(DomainError::DevoirNotesLiees.into());
        }
        let deleted = self
            .notes
            .delete_devoir(etablissement_id, devoir_id)
            .await?;
        if !deleted {
            return Err(DomainError::DevoirIntrouvable.into());
        }
        self.events
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: types::DEVOIR_DELETED.to_string(),
                aggregate: AggregateKind::Devoir,
                aggregate_id: devoir.id,
                eleve_id: None,
                payload: json!({
                    "periode_id": devoir.periode_id,
                    "matiere_id": devoir.matiere_id,
                    "classe": devoir.classe,
                    "titre": devoir.titre,
                }),
                actor_user_id: devoir.created_by_user_id.map(|u| u.to_string()),
            })
            .await?;
        Ok(devoir)
    }

    pub async fn supprimer_devoir_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        actor_user_id: Uuid,
        devoir_id: Uuid,
    ) -> Result<Devoir, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.can_saisir_notes() {
            return Err(EntDbError::Forbidden);
        }
        // RF-04 (g) : un professeur ne supprime que les devoirs de ses classes.
        let devoir = self
            .notes
            .get_devoir(etablissement_id, devoir_id)
            .await?
            .ok_or(DomainError::DevoirIntrouvable)?;
        self.exiger_classe_dans_perimetre(etablissement_id, actor_user_id, role, &devoir.classe)
            .await?;
        self.supprimer_devoir(etablissement_id, devoir_id).await
    }

    pub async fn get_devoir(
        &self,
        etablissement_id: Uuid,
        devoir_id: Uuid,
    ) -> Result<Option<Devoir>, EntDbError> {
        Ok(self.notes.get_devoir(etablissement_id, devoir_id).await?)
    }

    /// Liste des devoirs bornée au périmètre de l'acteur (RF-04 g) : un professeur ne
    /// voit que les devoirs de ses classes. Les routes utilisent cette variante.
    pub async fn list_devoirs_pour_acteur(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        periode_id: Uuid,
        classe: Option<&str>,
    ) -> Result<Vec<Devoir>, EntDbError> {
        let mut devoirs = self
            .list_devoirs_for_actor(etablissement_id, role, periode_id, classe)
            .await?;
        if let Some(cl) = self
            .classes_dans_perimetre(etablissement_id, actor_user_id, role)
            .await?
        {
            devoirs.retain(|d| cl.contains(d.classe.trim()));
        }
        Ok(devoirs)
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
            || Self::role_can_manage_eleves(role)
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

    /// Saisie d'une note par un acteur (socle 1, B1) : un professeur ne note que
    /// ses élèves ; les autres rôles autorisés à saisir gardent l'établissement.
    pub async fn saisir_note_for_actor(
        &self,
        role: &str,
        cmd: SaisirNote,
    ) -> Result<Note, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        // D-017 : « notes.saisir » et sa portée (professeur : ses classes par défaut).
        if !parsed.is_staff()
            || !self
                .autoriser_eleve(
                    cmd.etablissement_id,
                    cmd.auteur_user_id,
                    role,
                    Action::NotesSaisir,
                    cmd.eleve_id,
                )
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.saisir_note(cmd).await
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
            // S2-7 : un devoir de grille (rattaché à un service) se saisit par la grille,
            // qui contrôle le service, l'effectif et le verrou.
            let (grille,): (bool,) = sqlx::query_as(
                "SELECT service_id IS NOT NULL FROM ent_devoir WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(cmd.etablissement_id)
            .bind(devoir_id)
            .fetch_one(&self.pool)
            .await?;
            if grille {
                return Err(EntDbError::ConflitDossier("devoir de grille : saisir depuis la grille"));
            }
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

    /// Règle d'accès à un appel (ouverture, lecture, pointage, clôture) :
    /// titulaire du créneau (sans remplaçant ce jour), remplaçant désigné par
    /// l'exception du jour, ou rôle de supervision.
    fn autoriser_acteur_appel(
        creneau: &EdtCreneau,
        exception: Option<&EdtException>,
        acteur_user_id: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        let remplacant = exception.and_then(|e| e.remplacant_user_id);
        let est_remplacant = remplacant.is_some_and(|r| r == acteur_user_id);
        let est_titulaire = creneau.prof_user_id == acteur_user_id && remplacant.is_none();
        if est_titulaire || est_remplacant || Self::role_peut_ouvrir_appel_remplacant(role) {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Contrôle d'accès à un appel **existant** (même règle que l'ouverture).
    /// Un appel sans créneau (historique) n'est accessible qu'à son auteur
    /// ou à la supervision.
    async fn autoriser_acces_appel(
        &self,
        appel: &Appel,
        acteur_user_id: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        if !Self::role_can_faire_appel(role) {
            return Err(EntDbError::Forbidden);
        }
        let superviseur = Self::role_peut_ouvrir_appel_remplacant(role);
        let Some(creneau_id) = appel.creneau_id else {
            return if appel.auteur_user_id == acteur_user_id || superviseur {
                Ok(())
            } else {
                Err(EntDbError::Forbidden)
            };
        };
        let Some(creneau) = self.edt.get(appel.etablissement_id, creneau_id).await? else {
            return if superviseur {
                Ok(())
            } else {
                Err(EntDbError::Forbidden)
            };
        };
        let exception = self
            .edt
            .get_exception(appel.etablissement_id, creneau_id, appel.jour)
            .await?;
        Self::autoriser_acteur_appel(&creneau, exception.as_ref(), acteur_user_id, role)
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
        Self::peut_etablissement(role, Action::EdtGerer)
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
                // Socle 1 (C1) : le remplaçant doit être membre de l'établissement.
                if !self.auth.is_member(cmd.etablissement_id, rid).await? {
                    return Err(DomainError::EdtExceptionInvalide.into());
                }
            }
        }
        let actor = cmd.actor_user_id;
        let ex = plan_poser_edt_exception(cmd)?;
        // S2-5 : l'exception et son événement entrent dans l'outbox dans la même transaction
        // (élèves et familles prévenus par `d4.notifications_edt`). Le motif libre n'est plus
        // recopié dans l'événement.
        let map = |e: sqlx::Error| EntDbError::Domain(crate::pg_err::map_sqlx("edt_repo", e));
        let mut tx = self.pool().begin().await.map_err(map)?;
        sqlx::query(
            "INSERT INTO ent_edt_exception (
                id, etablissement_id, creneau_id, jour, kind, remplacant_user_id, nouvelle_salle,
                motif, created_by_user_id, created_at, nature
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$5)
             ON CONFLICT (etablissement_id, creneau_id, jour) DO UPDATE SET
                kind = EXCLUDED.kind, remplacant_user_id = EXCLUDED.remplacant_user_id,
                nouvelle_salle = EXCLUDED.nouvelle_salle, motif = EXCLUDED.motif,
                created_by_user_id = EXCLUDED.created_by_user_id, created_at = EXCLUDED.created_at,
                lot_id = NULL, absence_rh_id = NULL, nature = EXCLUDED.nature
             RETURNING id",
        )
        .bind(ex.id)
        .bind(ex.etablissement_id)
        .bind(ex.creneau_id)
        .bind(ex.jour)
        .bind(ex.kind.as_str())
        .bind(ex.remplacant_user_id)
        .bind(ex.nouvelle_salle.as_deref())
        .bind(&ex.motif)
        .bind(ex.created_by_user_id)
        .bind(ex.created_at)
        .fetch_one(&mut *tx)
        .await
        .map_err(map)?;
        let ev = {
            use crate::outbox::Publier;
            let conn: &mut sqlx::PgConnection = &mut tx;
            let type_name = if ex.kind == EdtExceptionKind::Remplacement {
                "edt.remplacement_organise"
            } else {
                types::EDT_EXCEPTION
            };
            conn.publier(MetierEventDraft {
                etablissement_id: ex.etablissement_id,
                type_name: type_name.to_string(),
                aggregate: AggregateKind::Edt,
                aggregate_id: ex.creneau_id,
                eleve_id: None,
                payload: json!({
                    "exception_id": ex.id,
                    "creneau_id": ex.creneau_id,
                    "jour": ex.jour.to_string(),
                    "nature": ex.kind.as_str(),
                    "remplacant_user_id": ex.remplacant_user_id,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?
        };
        tx.commit().await.map_err(map)?;
        self.diffuser_apres_commit(&[ev]);
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
        // Élève : même surface « mon EDT du jour » que /eleves/{id}/edt (pas l’appel).
        if Role::parse(role) == Some(Role::Eleve) {
            let Some(eleve_id) = self.eleve_id_for_compte(etablissement_id, user_id).await? else {
                return Err(EntDbError::Forbidden);
            };
            return self
                .list_edt_eleve_jour_for_actor(
                    etablissement_id,
                    role,
                    user_id,
                    eleve_id,
                    jour,
                )
                .await;
        }
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
    /// D-022 : élèves d'un appel. Créneau rattaché à un groupe (LV2, option…) → les
    /// membres du groupe ; sinon la classe. Un professeur (ni supervision) ne fait l'appel
    /// que des élèves qu'il suit (`eleves_suivis_par_prof`) : une classe mixte ne lui
    /// livre jamais les élèves de l'autre groupe.
    async fn effectif_appel(
        &self,
        creneau: &EdtCreneau,
        jour: NaiveDate,
        acteur_user_id: Uuid,
        role: &str,
    ) -> Result<Vec<Eleve>, EntDbError> {
        let (groupe,): (Option<Uuid>,) = sqlx::query_as(
            "SELECT groupe_id FROM ent_edt_creneau WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(creneau.etablissement_id)
        .bind(creneau.id)
        .fetch_one(&self.pool)
        .await?;
        let mut effectif = self
            .eleves
            .list_en_classe_au(creneau.etablissement_id, &creneau.classe, jour)
            .await?;
        if let Some(g) = groupe {
            let membres: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT a.eleve_id FROM ent_affectation a
                  WHERE a.etablissement_id = $1 AND a.groupe_id = $2
                    AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)",
            )
            .bind(creneau.etablissement_id)
            .bind(g)
            .bind(jour)
            .fetch_all(&self.pool)
            .await?;
            let membres: std::collections::HashSet<Uuid> = membres.into_iter().map(|(i,)| i).collect();
            effectif.retain(|e| membres.contains(&e.id));
        }
        if !Self::role_peut_ouvrir_appel_remplacant(role) {
            let suivis: std::collections::HashSet<Uuid> = self
                .eleves_suivis_par_prof(creneau.etablissement_id, acteur_user_id, jour)
                .await?
                .into_iter()
                .collect();
            effectif.retain(|e| suivis.contains(&e.id));
        }
        Ok(effectif)
    }

    /// D-022 : rattache (ou détache, `None`) un créneau à un groupe du référentiel.
    pub async fn rattacher_creneau_groupe_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        creneau_id: Uuid,
        groupe_id: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        if !Self::role_can_manage_edt(role) {
            return Err(EntDbError::Forbidden);
        }
        if let Some(g) = groupe_id {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_groupe WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etablissement_id)
            .bind(g)
            .fetch_one(&self.pool)
            .await?;
            if !ok {
                return Err(DomainError::NotFound.into());
            }
        }
        let r = sqlx::query(
            "UPDATE ent_edt_creneau SET groupe_id = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(creneau_id)
        .bind(groupe_id)
        .execute(&self.pool)
        .await?;
        if r.rows_affected() == 0 {
            return Err(DomainError::NotFound.into());
        }
        Ok(())
    }

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
        Self::autoriser_acteur_appel(&creneau, exception.as_ref(), auteur_user_id, role)?;

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

        let mut effectif = self
            .effectif_appel(&creneau, jour, auteur_user_id, role)
            .await?;
        // D-022 (recette personas 1, RP1-24) : l'appel d'un cours de groupe ne porte que sur
        // les élèves affectés au groupe à la date, jamais sur toute la classe.
        let groupe: Option<Uuid> = sqlx::query_scalar(
            "SELECT groupe_id FROM ent_edt_creneau WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(creneau.id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();
        if let Some(groupe_id) = groupe {
            let membres: Vec<Uuid> = sqlx::query_scalar(
                "SELECT eleve_id FROM ent_affectation
                  WHERE etablissement_id = $1 AND groupe_id = $2
                    AND date_debut <= $3 AND (date_fin IS NULL OR date_fin >= $3)",
            )
            .bind(etablissement_id)
            .bind(groupe_id)
            .bind(jour)
            .fetch_all(&self.pool)
            .await?;
            effectif.retain(|e| membres.contains(&e.id));
        }

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
        acteur_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        etat: LigneEtat,
    ) -> Result<(), EntDbError> {
        let appel = self
            .appels
            .get(etablissement_id, appel_id)
            .await?
            .ok_or(EntDbError::AppelInvalide)?;
        self.autoriser_acces_appel(&appel, acteur_user_id, role)
            .await?;
        if appel.statut != AppelStatut::Ouvert {
            return Err(EntDbError::AppelInvalide);
        }
        // Contrôle du statut + écriture sous verrou de ligne (sérialisé avec la clôture).
        // Seul « appel absent / non ouvert / ligne inexistante » devient `AppelInvalide` ;
        // une panne base ou un deadlock reste une erreur serveur (`EntDbError::Sqlx`).
        let ecrit = self
            .appels
            .set_etat_si_ouvert(etablissement_id, appel_id, eleve_id, &etat)
            .await?;
        if !ecrit {
            return Err(EntDbError::AppelInvalide);
        }
        Ok(())
    }

    /// Clôture → Brain `EnsureAbsence` pour chaque absent → `ent_absence` (VS / cantine).
    pub async fn clore_appel(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        acteur_user_id: Uuid,
        role: &str,
        actor_user_id: Option<String>,
    ) -> Result<Appel, EntDbError> {
        let appel = self
            .appels
            .get(etablissement_id, appel_id)
            .await?
            .ok_or(EntDbError::AppelInvalide)?;
        self.autoriser_acces_appel(&appel, acteur_user_id, role)
            .await?;
        if appel.statut != AppelStatut::Ouvert {
            return Err(EntDbError::AppelInvalide);
        }
        // Socle 2 (S2-0) : une seule transaction pour le fait et ses événements.
        // Clôture (lignes lues sous verrou), absences, retards, `appel.closed` et
        // `eleve.absence_declared` / `vs.retard` dans l'outbox : tout est validé
        // ensemble, ou rien (aucun événement pour un fait annulé, aucun fait sans
        // son événement). Les réactions des autres domaines (notification des
        // responsables…) passent par le distributeur de l'outbox.
        let mut tx = self.pool.begin().await?;
        let lignes = self
            .appels
            .clore_avec_lignes_tx(&mut tx, etablissement_id, appel_id)
            .await?
            .ok_or(EntDbError::AppelInvalide)?;
        let absents: Vec<Uuid> = lignes
            .iter()
            .filter(|l| l.etat == LigneEtat::Absent)
            .map(|l| l.eleve_id)
            .collect();
        let retards: Vec<Uuid> = lignes
            .iter()
            .filter(|l| l.etat == LigneEtat::Retard)
            .map(|l| l.eleve_id)
            .collect();

        let evt_appel = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types::APPEL_CLOSED.to_string(),
                aggregate: AggregateKind::Appel,
                aggregate_id: appel_id,
                eleve_id: None,
                payload: json!({
                    "classe": appel.classe,
                    "jour": appel.jour.to_string(),
                    "absents": absents.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                    "retards": retards.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                }),
                actor_user_id: actor_user_id.clone(),
            })
            .await?;
        let suite = Causalite::suite_de(&evt_appel, Causalite::default());
        let mut publies = vec![evt_appel];

        // Intra-domaine (D5) : les absents deviennent des absences canoniques
        // (`ent_absence`, une vérité pour VS / cantine / famille).
        for &eleve_id in &absents {
            let absence = Absence {
                id: Uuid::new_v4(),
                etablissement_id,
                eleve_id,
                debut: appel.jour,
                fin: appel.jour,
                motif: "appel".into(),
                statut: AbsenceStatut::ATraiter,
                motif_justif: None,
                justifie_par_user_id: None,
                decision_note: None,
                decision_par_user_id: None,
            };
            self.absences.upsert_tx(&mut tx, &absence).await?;
            // S2-4 (D-020) : l'absence est aussi enregistrée à la séance et rapprochée
            // des déclarations (accueil, famille) — même transaction.
            let prevenue = self
                .enregistrer_absence_seance_tx(
                    &mut tx,
                    etablissement_id,
                    eleve_id,
                    absence.id,
                    appel_id,
                    appel.creneau_id,
                    appel.jour,
                )
                .await?;
            publies.push(
                tx.publier_avec(
                    MetierEventDraft {
                        etablissement_id,
                        type_name: types::ELEVE_ABSENCE_DECLARED.to_string(),
                        aggregate: AggregateKind::Absence,
                        aggregate_id: absence.id,
                        eleve_id: Some(eleve_id),
                        payload: json!({
                            "debut": absence.debut.to_string(),
                            "fin": absence.fin.to_string(),
                            "statut": absence.statut.as_str(),
                            "source": "appel",
                            "appel_id": appel_id,
                            "prevenue": prevenue,
                        }),
                        actor_user_id: actor_user_id.clone(),
                    },
                    suite,
                )
                .await?,
            );
        }

        // Les retards pointés à l'appel deviennent un fait VS **canonique**
        // (persistant, distinct de l'absence) — sinon l'info disparaît avec l'appel.
        for eleve_id in retards {
            let r = plan_declarer_retard(DeclarerRetard {
                etablissement_id,
                eleve_id,
                jour: appel.jour,
                minutes: None,
                motif: None,
                source_appel_id: Some(appel_id),
                actor_user_id: actor_user_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
            })?;
            self.retards.insert_tx(&mut tx, &r).await?;
            publies.push(
                tx.publier_avec(
                    MetierEventDraft {
                        etablissement_id: r.etablissement_id,
                        type_name: types::RETARD_DECLARE.to_string(),
                        aggregate: AggregateKind::Retard,
                        aggregate_id: r.id,
                        eleve_id: Some(r.eleve_id),
                        payload: json!({
                            "jour": r.jour.to_string(),
                            "minutes": r.minutes,
                            "motif": r.motif,
                            "source_appel_id": r.source_appel_id,
                        }),
                        actor_user_id: r.auteur_user_id.map(|u| u.to_string()),
                    },
                    suite,
                )
                .await?,
            );
        }
        tx.commit().await?;
        self.events.diffuser_locaux(&publies);

        // Encore hors transaction (migration progressive, cf. SOCLE2_DOMAINES §6 S2-0) :
        // compteur annuel → signal de seuil → fil d'alerte et mail famille.
        for &eleve_id in &absents {
            self.apres_absence_ecrite(etablissement_id, eleve_id, appel.jour)
                .await?;
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
        acteur_user_id: Uuid,
        role: &str,
    ) -> Result<Option<(Appel, Vec<AppelLigne>)>, EntDbError> {
        let Some(appel) = self.appels.get(etablissement_id, appel_id).await? else {
            return Ok(None);
        };
        self.autoriser_acces_appel(&appel, acteur_user_id, role)
            .await?;
        let lignes = self.appels.list_lignes(etablissement_id, appel_id).await?;
        Ok(Some((appel, lignes)))
    }

    /// Charge un élève **si** l'acteur est autorisé à lire son dossier : staff,
    /// parent lié, ou l'élève lui-même. Refus sinon (isolation tenant + liens).
    async fn charger_eleve_si_autorise(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Eleve, EntDbError> {
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id).await?;
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = parsed.can_read_all_eleves() || Self::role_can_publier_annonces(role);
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
            .ok_or(DomainError::NotFound.into())
    }

    /// Déclare un retard (fait VS **canonique**, distinct de l'absence).
    /// Persistant : contrairement au simple état d'appel, il reste dans
    /// `ent_retard` (historique, compteur, famille).
    pub async fn declarer_retard_for_actor(
        &self,
        role: &str,
        cmd: DeclarerRetard,
    ) -> Result<Retard, EntDbError> {
        if !Self::role_can_faire_appel(role) {
            return Err(EntDbError::Forbidden);
        }
        self.eleves
            .get(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let planned = plan_declarer_retard(cmd)?;
        self.enregistrer_retard(&planned).await?;
        Ok(planned)
    }

    /// Écrit un retard + publie `vs.retard` (idempotent côté table).
    async fn enregistrer_retard(&self, r: &Retard) -> Result<(), EntDbError> {
        self.retards.insert(r).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: r.etablissement_id,
                type_name: types::RETARD_DECLARE.to_string(),
                aggregate: AggregateKind::Retard,
                aggregate_id: r.id,
                eleve_id: Some(r.eleve_id),
                payload: json!({
                    "jour": r.jour.to_string(),
                    "minutes": r.minutes,
                    "motif": r.motif,
                    "source_appel_id": r.source_appel_id,
                }),
                actor_user_id: r.auteur_user_id.map(|u| u.to_string()),
            })
            .await?;
        Ok(())
    }

    /// Historique des retards d'un élève (staff ou famille liée).
    pub async fn list_retards_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Retard>, EntDbError> {
        self.charger_eleve_si_autorise(etablissement_id, user_id, role, eleve_id)
            .await?;
        Ok(self
            .retards
            .list_eleve(etablissement_id, eleve_id, limit)
            .await?)
    }

    /// Nombre de retards d'un élève depuis une date (compteur vie scolaire).
    pub async fn count_retards_eleve_depuis(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        depuis: NaiveDate,
    ) -> Result<i64, EntDbError> {
        Ok(self
            .retards
            .count_eleve_depuis(etablissement_id, eleve_id, depuis)
            .await?)
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
        // Socle 2 (S2-0) : décision et événement dans la même transaction (outbox).
        let mut tx = self.pool.begin().await?;
        self.absences.upsert_tx(&mut tx, &planned.absence).await?;
        let mut publies = Vec::new();
        if let Some(event) = planned.event {
            publies.push(tx.publier(event).await?);
        }
        tx.commit().await?;
        self.events.diffuser_locaux(&publies);
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
        // Socle 2 (S2-0) : décision et événement dans la même transaction (outbox).
        let mut tx = self.pool.begin().await?;
        self.absences.upsert_tx(&mut tx, &planned.absence).await?;
        let mut publies = Vec::new();
        if let Some(event) = planned.event {
            publies.push(tx.publier(event).await?);
        }
        tx.commit().await?;
        self.events.diffuser_locaux(&publies);
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
        if !Self::role_can_manage_eleves(role)
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
        if !Self::role_can_manage_eleves(role)
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
        if !Self::role_can_manage_eleves(role)
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
        if !Self::role_can_manage_eleves(role)
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

    // ───── R2 (D-017) : domaines sensibles, droits via le moteur ─────

    /// Le personnel `role` a-t-il `action` sur cet élève ? Familles et élèves :
    /// jamais (garde-fou 1) — leurs règles relationnelles restent à l'appelant.
    pub(crate) async fn staff_eleve(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        action: Action,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Ok(false);
        }
        self.autoriser_eleve(etablissement_id, user_id, role, action, eleve_id)
            .await
    }

    /// Le personnel a-t-il `action` avec au moins une portée (pré-filtre des
    /// routes et des listes ; le contrôle par élève reste à `autoriser_eleve`) ?
    pub async fn a_action_quelconque(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        action: Action,
    ) -> Result<bool, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Ok(false);
        }
        Ok(self
            .droits_effectifs(etablissement_id, user_id, role)
            .await?
            .iter()
            .any(|d| d.action == action))
    }

    /// Variante d'`a_action_quelconque` qui échoue en `Forbidden`.
    pub async fn exiger_action_quelconque(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        action: Action,
    ) -> Result<(), EntDbError> {
        if self
            .a_action_quelconque(etablissement_id, user_id, role, action)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Droit d'établissement du seul profil de base (sans rôle paramétré) :
    /// utilisé par les anciens points d'entrée sans identité d'acteur (seed, lab).
    fn modele_a_etablissement(role: &str, action: Action) -> bool {
        Role::parse(role).is_some_and(|r| {
            ent_auth::modele_par_defaut(r)
                .iter()
                .any(|(a, p)| *a == action && *p == Portee::Etablissement)
        })
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
        let declare = cmd.mime_type.clone();
        let titre = cmd.titre.clone();
        let mut doc = plan_deposer_document(cmd)?;
        if let Some(bytes) = contenu {
            // D2 : le type retenu est celui des octets, jamais celui du client.
            let valide = self
                .uploads
                .valider(UsageFichier::Document, &titre, declare.as_deref(), bytes)
                .map_err(EntDbError::Fichier)?;
            doc.mime_type = Some(valide.mime.to_string());
            let ext = valide.extension;
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
        // Chemin public : un client ne choisit jamais la clé objet. Elle n'est posée
        // que par le serveur, si du contenu est écrit (voir `deposer_document_with_contenu`).
        cmd.storage_key = None;
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::DocumentsDeposer,
                cmd.eleve_id,
            )
            .await?
        {
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

    /// Contrôle d'accès commun au téléchargement ET à l'émission d'un lien :
    /// le document existe dans l'établissement et l'acteur peut le lire.
    async fn document_lisible_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        document_id: Uuid,
    ) -> Result<EleveDocument, EntDbError> {
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
        Ok(doc)
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
            .document_lisible_for_actor(etablissement_id, actor_user_id, role, document_id)
            .await?;
        let Some(key) = doc.storage_key.as_ref() else {
            return Err(EntDbError::FichierAbsent);
        };
        let stub = |doc: &EleveDocument| {
            // Lab : métadonnée sans octets (ancien dépôt) → stub lisible
            // (uniquement les métadonnées du document lui-même, déjà autorisées).
            format!(
                "Document ENT (lab) — {}\nTiroir: {}\nCode: {}\n",
                doc.titre,
                doc.tiroir.as_str(),
                doc.code.as_deref().unwrap_or("—")
            )
            .into_bytes()
        };
        // La clé stockée doit appartenir à CE document ; sinon le magasin n'est pas lu.
        if !document_storage_key_is_owned(&doc, key) {
            tracing::warn!(
                document_id = %doc.id,
                etablissement_id = %doc.etablissement_id,
                key_len = key.len(),
                "storage_key non conforme : lecture du magasin refusée (stub)"
            );
            let bytes = stub(&doc);
            return Ok((doc, bytes));
        }
        match self.objects.get(key) {
            Ok(bytes) => Ok((doc, bytes)),
            Err(ObjectStoreError::NotFound(_)) => {
                let bytes = stub(&doc);
                Ok((doc, bytes))
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

    /// Demandes d'accès en attente, limitées aux élèves sur lesquels l'acteur a
    /// `documents.gerer_droits` (D-017, R2).
    pub async fn list_grants_pending_pour_acteur(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Vec<DocumentGrant>, EntDbError> {
        self.exiger_action_quelconque(
            etablissement_id,
            actor_user_id,
            role,
            Action::DocumentsGererDroits,
        )
        .await?;
        let autorises = self
            .eleves_autorises(
                etablissement_id,
                actor_user_id,
                role,
                Action::DocumentsGererDroits,
            )
            .await?;
        let tous = self.documents.list_grants_pending(etablissement_id).await?;
        let Some(autorises) = autorises else {
            return Ok(tous);
        };
        let mut out = Vec::new();
        for g in tous {
            if let Some(doc) = self
                .documents
                .get_document(etablissement_id, g.document_id)
                .await?
            {
                if autorises.contains(&doc.eleve_id) {
                    out.push(g);
                }
            }
        }
        Ok(out)
    }

    /// Ancien point d'entrée sans identité d'acteur : profil de base seulement
    /// (seed et labo). Les routes utilisent `list_grants_pending_pour_acteur`.
    pub async fn list_grants_pending_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<Vec<DocumentGrant>, EntDbError> {
        if !Self::modele_a_etablissement(role, Action::DocumentsGererDroits) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.documents.list_grants_pending(etablissement_id).await?)
    }

    pub async fn latest_grant_etat_for_actor_doc(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        document_id: Uuid,
    ) -> Result<Option<String>, EntDbError> {
        Ok(self
            .documents
            .latest_grant_status_for_user_doc(etablissement_id, document_id, actor_user_id)
            .await?)
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
            // RF-15 (D-017) : le droit « dossier.lire » et sa portée décident ;
            // un professeur ne liste que les documents de ses élèves.
            if !self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::DossierLire, eleve_id)
                .await?
            {
                return Err(EntDbError::Forbidden);
            }
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
            // D-014 : le document famille d'un autre foyer n'apparaît même pas.
            if !accessible
                && parsed == Role::Parent
                && doc.tiroir == DocTiroir::Famille
                && !self
                    .foyers
                    .document_famille_lisible(
                        etablissement_id,
                        doc.eleve_id,
                        doc.created_by_user_id,
                        actor_user_id,
                    )
                    .await?
            {
                continue;
            }
            if !accessible {
                doc.storage_key = None;
                doc.mime_type = None;
                // RF-15 : le titre d'un document restreint/santé n'est pas révélé.
                if doc.confidentialite != DocConfidentialite::Standard {
                    doc.titre = "Document à accès restreint".to_string();
                }
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
        self.exiger_action_quelconque(
            cmd.etablissement_id,
            cmd.actor_user_id,
            role,
            Action::DocumentsGererDroits,
        )
        .await?;
        let grant = self
            .documents
            .get_grant(cmd.etablissement_id, cmd.grant_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let doc = self
            .documents
            .get_document(cmd.etablissement_id, grant.document_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::DocumentsGererDroits,
                doc.eleve_id,
            )
            .await?
        {
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
        if parsed.is_staff() {
            if self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::DocumentsLire, doc.eleve_id)
                .await?
            {
                return Ok(true);
            }
            if doc.confidentialite == DocConfidentialite::Sante
                && self
                    .autoriser_eleve(
                        etablissement_id,
                        actor_user_id,
                        role,
                        Action::DocumentsLireSante,
                        doc.eleve_id,
                    )
                    .await?
            {
                return Ok(true);
            }
        }
        // Parent : tiroir famille + standard seulement (sans grant).
        if parsed == Role::Parent
            && doc.tiroir == DocTiroir::Famille
            && doc.confidentialite == DocConfidentialite::Standard
        {
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, doc.eleve_id)
                .await?;
            if lien.is_none() {
                return Ok(false);
            }
            // D-014 (RP1-11) : seulement les documents de son propre foyer.
            return Ok(self
                .foyers
                .document_famille_lisible(
                    etablissement_id,
                    doc.eleve_id,
                    doc.created_by_user_id,
                    actor_user_id,
                )
                .await?);
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
        Self::peut_etablissement(role, Action::RepasGerer)
    }

    pub fn role_can_manage_bourses(role: &str) -> bool {
        Self::peut_etablissement(role, Action::BoursesGerer)
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
            Role::Eleve => {
                // Portail élève : prévenance cantine sur son propre dossier (même source « parent » facturation).
                if cmd.source != DispenseSource::Parent {
                    return Err(EntDbError::Forbidden);
                }
                let linked = self
                    .eleve_comptes
                    .eleve_id_for_user(cmd.etablissement_id, actor_id)
                    .await?;
                if linked != Some(cmd.eleve_id) {
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
            Role::Eleve => {
                if current.source != DispenseSource::Parent {
                    return Err(EntDbError::Forbidden);
                }
                let linked = self
                    .eleve_comptes
                    .eleve_id_for_user(etablissement_id, actor_user_id)
                    .await?;
                if linked != Some(current.eleve_id) {
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
        Self::peut_etablissement(role, Action::RepasScanner)
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
        let staff = Self::role_can_scan_repas(role) || Self::role_can_manage_eleves(role);
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
        if !Self::role_can_lire_facturation(role) {
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
            || Self::role_can_lire_facturation(role)
            || Self::role_can_manage_eleves(role)
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
        let actor_uuid = actor.as_deref().and_then(|s| Uuid::parse_str(s).ok());
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

        // Répercussion famille : la décision de bourse est notifiée aux responsables.
        let eleve = self.eleves.get(b.etablissement_id, b.eleve_id).await?;
        let (prenom, nom) = eleve
            .as_ref()
            .map(|e| (e.prenom.as_str(), e.nom.as_str()))
            .unwrap_or(("l'élève", ""));
        let statut_lbl = match b.status {
            domain_eleve::BourseStatus::Acceptee => "acceptée",
            domain_eleve::BourseStatus::Refusee => "refusée",
            domain_eleve::BourseStatus::Demandee => "enregistrée",
            domain_eleve::BourseStatus::Cloturee => "clôturée",
        };
        let sujet = format!("Décision bourse — {prenom} {nom}");
        let corps = format!(
            "La demande de bourse de {prenom} {nom} est {statut_lbl}. Détail dans le portail famille."
        );
        let kind = format!("bourse_{}", b.status.as_str());
        let _ = self
            .notify_parents_mail(
                b.etablissement_id,
                b.eleve_id,
                &kind,
                &sujet,
                &corps,
                "bourse",
                b.id,
                actor_uuid,
                Vec::new(),
            )
            .await;
        let _ = self
            .notify_parents_push(
                b.etablissement_id,
                b.eleve_id,
                &kind,
                "Décision bourse",
                &corps,
                "bourse",
                b.id,
                actor_uuid,
            )
            .await;
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
            .search(cmd.etablissement_id, "", 2000, true)
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
        let eleves = self.eleves.search(etablissement_id, "", 2000, true).await?;
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
        mime_type: Option<&str>,
        contenu: Option<&[u8]>,
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
        let mime = mime_type
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("application/pdf");
        let cmd = DeposerDocument {
            etablissement_id,
            eleve_id,
            tiroir: exigence.tiroir,
            titre: if titre.trim().is_empty() {
                exigence.libelle.clone()
            } else {
                titre.to_string()
            },
            code: Some(code_norm),
            mime_type: Some(mime.into()),
            storage_key: if contenu.is_some() {
                None
            } else {
                Some(format!(
                    "reinscription/{etablissement_id}/{eleve_id}/{}.pdf",
                    exigence.code
                ))
            },
            confidentialite: DocConfidentialite::Standard,
            actor_user_id: parent_user_id,
        };
        if contenu.is_some() {
            self.deposer_document_with_contenu(cmd, contenu).await
        } else {
            self.deposer_document(cmd).await
        }
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
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::IncidentsDeclarer,
                cmd.eleve_id,
            )
            .await?
        {
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
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::SanctionsPrononcer,
                cmd.eleve_id,
            )
            .await?
        {
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
        if !self
            .a_action_etablissement(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::SanctionsPrononcer,
            )
            .await?
        {
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
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::SanctionsPrononcer,
                cmd.eleve_id,
            )
            .await?
        {
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
        self.exiger_action_quelconque(
            cmd.etablissement_id,
            cmd.actor_user_id,
            role,
            Action::SanctionsPrononcer,
        )
        .await?;
        let current = self
            .sanctions
            .get_commission_dossier(cmd.etablissement_id, cmd.dossier_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::SanctionsPrononcer,
                current.eleve_id,
            )
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
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

    /// Commissions du jour : exige `sanctions.prononcer` sur tout l'établissement.
    pub async fn list_commissions_pour_acteur(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<CommissionVs>, EntDbError> {
        if !self
            .a_action_etablissement(etablissement_id, actor_user_id, role, Action::SanctionsPrononcer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .sanctions
            .list_commissions_jour(etablissement_id, jour, 50)
            .await?)
    }

    /// Ancien point d'entrée sans identité d'acteur : profil de base seulement
    /// (seed et labo). Les routes utilisent `list_commissions_pour_acteur`.
    pub async fn list_commissions_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<CommissionVs>, EntDbError> {
        if !Self::modele_a_etablissement(role, Action::SanctionsPrononcer) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .sanctions
            .list_commissions_jour(etablissement_id, jour, 50)
            .await?)
    }

    /// Dossiers d'une commission, limités aux élèves couverts par `sanctions.prononcer`.
    pub async fn list_dossiers_commission_pour_acteur(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        commission_id: Uuid,
    ) -> Result<Vec<CommissionDossier>, EntDbError> {
        self.exiger_action_quelconque(
            etablissement_id,
            actor_user_id,
            role,
            Action::SanctionsPrononcer,
        )
        .await?;
        let autorises = self
            .eleves_autorises(etablissement_id, actor_user_id, role, Action::SanctionsPrononcer)
            .await?;
        let mut list = self
            .sanctions
            .list_dossiers_commission(etablissement_id, commission_id)
            .await?;
        if let Some(autorises) = autorises {
            list.retain(|d| autorises.contains(&d.eleve_id));
        }
        Ok(list)
    }

    pub async fn list_dossiers_commission_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        commission_id: Uuid,
    ) -> Result<Vec<CommissionDossier>, EntDbError> {
        if !Self::modele_a_etablissement(role, Action::SanctionsPrononcer) {
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
        let staff = self
            .staff_eleve(etablissement_id, actor_user_id, role, Action::SanctionsPrononcer, eleve_id)
            .await?;
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
        let staff = self
            .staff_eleve(etablissement_id, actor_user_id, role, Action::SanctionsPrononcer, eleve_id)
            .await?;
        if !staff {
            if parsed != Role::Parent {
                return Err(EntDbError::Forbidden);
            }
            let lien = self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?;
            if !lien.map(|l| l.peut_voir_vs).unwrap_or(false) {
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
        self.exiger_action_quelconque(
            cmd.etablissement_id,
            cmd.actor_user_id,
            role,
            Action::SanctionsPrononcer,
        )
        .await?;
        let sanction = self
            .sanctions
            .get_sanction(cmd.etablissement_id, cmd.sanction_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::SanctionsPrononcer,
                sanction.eleve_id,
            )
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.annuler_sanction(cmd).await
    }

    pub fn role_can_gerer_sorties(role: &str) -> bool {
        Self::peut_etablissement(role, Action::SortiesGerer)
    }

    pub fn role_can_controle_sortie(role: &str) -> bool {
        Self::peut_etablissement(role, Action::SortiesControler)
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
            // Revue finale socle 1 : le droit coché « autoriser les sorties » du lien
            // est exigé (il n'était lu que par l'ancienne UI).
            if !self
                .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
                .await?
                .is_some_and(|l| l.peut_autoriser_sorties)
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
        self.exiger_perimetre_prof(etablissement_id, actor_user_id, role, eleve_id).await?;
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
            r if Self::role_can_gerer_sorties(role) || r.can_read_all_eleves() => {}
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
                // Revue finale socle 1 : droit coché « autoriser les sorties » exigé.
                if !self
                    .lien_parent_eleve(cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id)
                    .await?
                    .is_some_and(|l| l.peut_autoriser_sorties)
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
            _ if Self::role_can_gerer_sorties(role) => {}
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
        Self::peut_etablissement(role, Action::InternatGerer)
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
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id).await?;
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

    pub(crate) async fn enregistrer_journal_interne(
        &self,
        cmd: EnregistrerJournal,
    ) -> Result<JournalEntree, EntDbError> {
        let planned = plan_enregistrer_journal(cmd)?;
        if !self.journal.insert(&planned).await? {
            // Élève d'un autre établissement : ni trace ni événement.
            return Ok(planned);
        }
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
        let staff = parsed.can_read_all_eleves() || Self::role_can_manage_eleves(role);
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parsed.is_staff()
            && !self
                .autoriser_eleve(etablissement_id, user_id, role, Action::JournalLire, eleve_id)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && !self
                .lien_parent_eleve(etablissement_id, user_id, eleve_id)
                .await?
                .map(|l| l.peut_voir_vs)
                .unwrap_or(false)
        {
            // Le journal expose des faits de vie scolaire : exige `peut_voir_vs`.
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
        let mut entrees = self
            .journal
            .list_eleve(etablissement_id, eleve_id, famille_only, lim)
            .await?;
        // A1 : conflits d'intérêts et interdictions ne se lisent pas dans le journal d'un élève
        // (ils décrivent des données F3/F4) ; la direction les voit dans ses tableaux dédiés.
        if !famille_only
            && !self
                .a_action_etablissement(etablissement_id, user_id, role, Action::ResponsablesProtegeLire)
                .await?
        {
            entrees.retain(|e| {
                e.domaine != "conflit_interet"
                    && !e.action.starts_with("conflit_interet")
                    && !e.action.starts_with("interdiction_")
            });
        }
        // RP1-20 : les traces de consultation sensible (« Consultation du PAI », « …dossier
        // psychologue ») sont du journal d'audit : `audit.lire` seulement, pas le personnel
        // qui lit simplement le journal de vie scolaire.
        if parsed.is_staff()
            && !self
                .a_action_etablissement(etablissement_id, user_id, role, Action::AuditLire)
                .await?
        {
            entrees.retain(|e| e.domaine != "consultation");
        }
        Ok(entrees)
    }

    /// Journal d'audit filtrable par domaine (ex. `auth`) — staff habilité.
    pub async fn list_journal_domaine_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        domaine: &str,
        limit: i64,
    ) -> Result<Vec<JournalEntree>, EntDbError> {
        // D-017 R2 : journal d'audit (dont les consultations sensibles) = `audit.lire`.
        if !self
            .a_action_etablissement(etablissement_id, actor_user_id, role, Action::AuditLire)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let lim = if limit <= 0 { 50 } else { limit.min(500) };
        Ok(self
            .journal
            .list_domaine(etablissement_id, domaine, lim)
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
            || Self::role_can_manage_eleves(role)
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
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id).await?;
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
        Self::peut_etablissement(role, Action::DocsSignerPublier)
    }

    pub fn role_can_publier_annonces(role: &str) -> bool {
        Self::peut_etablissement(role, Action::AnnoncesPublier)
    }

    /// Publie une annonce de l'établissement. Répercussions transverses :
    /// événement `annonce.publiee`, entrée journal, et — si l'audience touche
    /// les familles — notification mail + push **de chaque famille ciblée**
    /// (anti-doublon par destinataire, cf. `notify_parents_*_broadcast`).
    pub async fn publier_annonce_for_actor(
        &self,
        role: &str,
        cmd: CreerAnnonce,
    ) -> Result<Annonce, EntDbError> {
        if !Self::role_can_publier_annonces(role) {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_creer_annonce(cmd)?;
        self.annonces.insert(&planned).await?;
        self.events
            .publish(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::ANNONCE_PUBLIEE.to_string(),
                aggregate: AggregateKind::Annonce,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "titre": planned.titre,
                    "audience": planned.audience.as_str(),
                    "cible_classe": planned.cible_classe,
                }),
                actor_user_id: Some(planned.auteur_user_id.to_string()),
            })
            .await?;
        self.enregistrer_journal_interne(EnregistrerJournal {
            etablissement_id: planned.etablissement_id,
            eleve_id: None,
            domaine: "communication".into(),
            action: "annonce_publiee".into(),
            libelle: format!(
                "Annonce « {} » diffusée ({})",
                planned.titre,
                planned.audience.as_str()
            ),
            payload: json!({
                "annonce_id": planned.id,
                "audience": planned.audience.as_str(),
                "cible_classe": planned.cible_classe,
            }),
            visible_famille: planned.audience.touche_parents(),
            actor_user_id: Some(planned.auteur_user_id),
        })
        .await?;

        if planned.audience.touche_parents() {
            self.diffuser_annonce_aux_familles(&planned).await;
        }
        Ok(planned)
    }

    /// Projette une annonce vers les familles ciblées : un mail + un push par
    /// élève (donc par foyer), sans jamais bloquer le geste si un envoi échoue.
    async fn diffuser_annonce_aux_familles(&self, a: &Annonce) {
        let jour = chrono::Utc::now().date_naive();
        let eleves = match a.audience {
            AudienceAnnonce::Classe => {
                let classe = a.cible_classe.clone().unwrap_or_default();
                self.eleves
                    .list_en_classe_au(a.etablissement_id, &classe, jour)
                    .await
                    .unwrap_or_default()
            }
            _ => self
                .eleves
                .list_actifs_au(a.etablissement_id, jour)
                .await
                .unwrap_or_default(),
        };
        let subject = format!("Annonce — {}", a.titre);
        let push_body: String = a.corps.chars().take(140).collect();
        for e in eleves {
            let _ = self
                .notify_parents_mail_broadcast(
                    a.etablissement_id,
                    e.id,
                    "annonce",
                    &subject,
                    &a.corps,
                    "annonce",
                    a.id,
                    Some(a.auteur_user_id),
                    Vec::new(),
                )
                .await;
            let _ = self
                .notify_parents_push_broadcast(
                    a.etablissement_id,
                    e.id,
                    "annonce",
                    &subject,
                    &push_body,
                    "annonce",
                    a.id,
                    Some(a.auteur_user_id),
                )
                .await;
        }
    }

    /// Annonces de l'établissement (vue staff / communication).
    pub async fn list_annonces_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        limit: i64,
    ) -> Result<Vec<Annonce>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if matches!(parsed, Role::Parent | Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self.annonces.list(etablissement_id, limit).await?)
    }

    /// Annonces visibles par la famille d'un élève (ciblage classe respecté).
    /// Le lien à l'élève est vérifié (parent) comme pour les autres vues famille.
    pub async fn list_annonces_famille_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<Annonce>, EntDbError> {
        let eleve = self
            .charger_eleve_si_autorise(etablissement_id, user_id, role, eleve_id)
            .await?;
        Ok(self
            .annonces
            .list_visibles_famille(etablissement_id, eleve.classe.as_deref(), 100)
            .await?)
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

        // Répercussion famille : un document à signer est une action requise → notifier.
        let _ = self
            .notify_parents_mail(
                planned.etablissement_id,
                planned.eleve_id,
                "doc_a_signer",
                &format!("Document à signer : {}", planned.titre),
                &format!(
                    "Un document est à signer dans le portail famille : « {} ».",
                    planned.titre
                ),
                "doc_a_signer",
                planned.id,
                Some(planned.publie_par_user_id),
                Vec::new(),
            )
            .await;
        let _ = self
            .notify_parents_push(
                planned.etablissement_id,
                planned.eleve_id,
                "doc_a_signer",
                "Document à signer",
                &planned.titre,
                "doc_a_signer",
                planned.id,
                Some(planned.publie_par_user_id),
            )
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
            // Recette personas 1 (RP1-10) : un élève mineur ne signe pas à la place de ses
            // responsables ; seul l'élève majeur signe lui-même.
            if !self
                .eleve_est_majeur(cmd.etablissement_id, current.eleve_id, today_utc())
                .await?
            {
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
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id).await?;
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
        Self::peut_etablissement(role, Action::ConseilsGerer)
    }

    pub async fn creer_conseil_classe_for_actor(
        &self,
        role: &str,
        cmd: CreerConseilClasse,
    ) -> Result<ConseilClasse, EntDbError> {
        if !Self::role_can_gerer_conseil_classe(role) {
            return Err(EntDbError::Forbidden);
        }
        // RF-04 (f) : un professeur n'agit que sur les classes qu'il enseigne.
        self.exiger_classe_dans_perimetre(cmd.etablissement_id, cmd.actor_user_id, role, &cmd.classe)
            .await?;
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
        self.exiger_classe_dans_perimetre(etablissement_id, actor_user_id, role, &current.classe)
            .await?;
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
        let conseil = self
            .conseils
            .get_conseil(cmd.etablissement_id, cmd.conseil_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.exiger_classe_dans_perimetre(
            cmd.etablissement_id,
            cmd.actor_user_id,
            role,
            &conseil.classe,
        )
        .await?;
        self.exiger_perimetre_prof(cmd.etablissement_id, cmd.actor_user_id, role, cmd.eleve_id)
            .await?;
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
        // RF-04 (f) : mention et envoi aux familles seulement pour un élève suivi.
        self.exiger_perimetre_prof(cmd.etablissement_id, cmd.actor_user_id, role, current.eleve_id)
            .await?;
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

            // Répercussion famille : une décision de conseil de classe publiée
            // (visible_famille) est communiquée aux responsables.
            let mention_lbl = decided.mention.map(|m| m.as_str()).unwrap_or("décision");
            let eleve = self
                .eleves
                .get(decided.etablissement_id, decided.eleve_id)
                .await?;
            let (prenom, nom) = eleve
                .as_ref()
                .map(|e| (e.prenom.as_str(), e.nom.as_str()))
                .unwrap_or(("l'élève", ""));
            let sujet = format!("Conseil de classe — {prenom} {nom}");
            let corps = format!(
                "Le conseil de classe de {prenom} {nom} a rendu sa décision ({mention_lbl}). Détail dans le portail famille."
            );
            let _ = self
                .notify_parents_mail(
                    decided.etablissement_id,
                    decided.eleve_id,
                    "conseil_decision",
                    &sujet,
                    &corps,
                    "conseil_classe",
                    decided.id,
                    decided.decided_by_user_id,
                    Vec::new(),
                )
                .await;
            let _ = self
                .notify_parents_push(
                    decided.etablissement_id,
                    decided.eleve_id,
                    "conseil_decision",
                    "Décision conseil de classe",
                    &corps,
                    "conseil_classe",
                    decided.id,
                    decided.decided_by_user_id,
                )
                .await;
        }
        Ok(decided)
    }

    pub async fn list_conseils_classe_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        classe: Option<&str>,
    ) -> Result<Vec<ConseilClasse>, EntDbError> {
        if !Self::role_can_gerer_conseil_classe(role) {
            return Err(EntDbError::Forbidden);
        }
        let mut conseils = self.conseils.list_conseils(etablissement_id, classe).await?;
        if let Some(cl) = self
            .classes_dans_perimetre(etablissement_id, actor_user_id, role)
            .await?
        {
            conseils.retain(|c| cl.contains(c.classe.trim()));
        }
        Ok(conseils)
    }

    pub async fn list_dossiers_conseil_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        conseil_id: Uuid,
    ) -> Result<Vec<ConseilClasseDossier>, EntDbError> {
        if !Self::role_can_gerer_conseil_classe(role) {
            return Err(EntDbError::Forbidden);
        }
        let conseil = self
            .conseils
            .get_conseil(etablissement_id, conseil_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        self.exiger_classe_dans_perimetre(etablissement_id, actor_user_id, role, &conseil.classe)
            .await?;
        let dossiers = self
            .conseils
            .list_dossiers_conseil(etablissement_id, conseil_id)
            .await?;
        // D-022 : un professeur de groupe ne voit que les dossiers des élèves de son groupe.
        // Pour tout rôle (délégations, rôles libres à portée limitée compris).
        if let Some(ok) = self
            .eleves_autorises(etablissement_id, actor_user_id, role, Action::SuiviEleve)
            .await?
        {
            return Ok(dossiers.into_iter().filter(|d| ok.contains(&d.eleve_id)).collect());
        }
        Ok(dossiers)
    }

    pub async fn list_conseils_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<ConseilClasseDossier>, EntDbError> {
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id).await?;
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
        if !Self::role_can_manage_eleves(role) {
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
        if !(Self::role_can_manage_eleves(role) || parsed.can_read_all_eleves()) {
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
        let staff = Self::role_can_manage_eleves(role) || Self::role_can_gerer_alertes_cpe(role);
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
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id).await?;
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_manage_eleves(role) || parsed.can_read_all_eleves();
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
        if !Self::role_can_manage_eleves(role) {
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
        if !Self::role_can_manage_eleves(role) {
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
        actor_user_id: Uuid,
        role: &str,
        cmd: RechercherHomonymes,
    ) -> Result<Vec<HomonymeCandidat>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !Self::role_can_manage_eleves(role) && !parsed.can_read_all_eleves() {
            return Err(EntDbError::Forbidden);
        }
        // E0/E1 : minimisation par groupes. Un lecteur qui ne voit pas l'état civil
        // d'un candidat ne reçoit pas sa date de naissance, et la date qu'il fournirait
        // n'entre pas dans le score de ce candidat (sinon le score serait un oracle de
        // la date de naissance des élèves).
        let profil = self
            .profil_champs(etablissement_id, actor_user_id, role)
            .await?;
        let voit_naissance = |id: Uuid| profil.pour(id).ouvert(domain_eleve::GroupeChamps::EtatCivil);
        let date_naissance = cmd.date_naissance;
        let (nom, prenom) = plan_rechercher_homonymes(RechercherHomonymes {
            etablissement_id,
            nom: cmd.nom,
            prenom: cmd.prenom,
            date_naissance,
        })?;
        // Le badge n'est interrogeable que s'il est visible (pas d'oracle).
        let candidats = self
            .eleves
            .search(etablissement_id, &nom, 50, profil.badge_interrogeable())
            .await?;
        let mut out: Vec<HomonymeCandidat> = candidats
            .into_iter()
            .filter(|e| profil.voit_designation(e.id))
            .filter_map(|e| {
                let voit = voit_naissance(e.id);
                let score = score_homonyme(
                    &nom,
                    &prenom,
                    if voit { date_naissance } else { None },
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
                    date_naissance: if voit { e.date_naissance } else { None },
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
            Self::role_can_lire_facturation(role) || Self::role_can_manage_bourses(role);
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
        if !(Self::role_can_lire_facturation(role) || parsed.can_read_all_eleves()) {
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
        if !(Self::role_can_lire_facturation(role) || parsed.can_read_all_eleves()) {
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
        if Self::role_can_manage_eleves(role) {
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
        // D2 : signature binaire, liste blanche « photo », taille maximale configurable.
        let valide = self
            .uploads
            .valider(UsageFichier::Photo, "photo", Some(content_type), bytes)
            .map_err(EntDbError::Fichier)?;
        let ext = plan_poser_photo_eleve(PoserPhotoEleve {
            etablissement_id,
            eleve_id,
            content_type: valide.mime.to_string(),
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
        actor_user_id: Uuid,
        role: &str,
        classe: &str,
        jour: NaiveDate,
    ) -> Result<Vec<TrombiEntree>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !(Self::role_can_manage_eleves(role)
            || parsed.can_read_all_eleves()
            || parsed.can_faire_appel())
        {
            return Err(EntDbError::Forbidden);
        }
        let eleves = self
            .eleves
            .list_en_classe_au(etablissement_id, classe, jour)
            .await?;
        // D-022 : un professeur ne voit que les élèves de ses groupes dans une classe mixte.
        let eleves = self
            .restreindre_eleves_du_professeur(etablissement_id, actor_user_id, role, eleves)
            .await?;
        // S2-8 (D24) : un refus du droit à l'image masque la photo.
        let ids: Vec<Uuid> = eleves.iter().map(|e| e.id).collect();
        let masques = self
            .eleves_ayant_refuse(
                etablissement_id,
                &ids,
                crate::formulaires::OBJET_DROIT_IMAGE,
            )
            .await?;
        Ok(eleves
            .into_iter()
            .map(|mut e| {
                if masques.contains(&e.id) {
                    e.photo_key = None;
                }
                e
            })
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
        let Some(key) = self
            .photo_cle_for_actor(etablissement_id, user_id, role, eleve_id)
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(self.objects.get(&key)?))
    }

    /// Contrôle d'accès commun au téléchargement ET à l'émission d'un lien de photo :
    /// retourne la clé objet **vérifiée comme appartenant à cet élève**, ou `None`.
    async fn photo_cle_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<String>, EntDbError> {
        // RP1-04 : la photo d'un élève n'est lue que par le personnel qui le suit.
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id)
            .await?;
        let eleve = self
            .get_eleve_for_actor(etablissement_id, user_id, role, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let Some(key) = eleve.photo_key.as_ref() else {
            return Ok(None);
        };
        // S2-8 (D24) : photo masquée si le droit à l'image est refusé.
        if !self
            .eleves_ayant_refuse(
                etablissement_id,
                &[eleve.id],
                crate::formulaires::OBJET_DROIT_IMAGE,
            )
            .await?
            .is_empty()
        {
            return Ok(None);
        }
        // Clé non conforme (autre élève / établissement) : jamais lue, comme « absente ».
        if !photo_storage_key_is_owned(&eleve, key) {
            tracing::warn!(
                eleve_id = %eleve.id,
                etablissement_id = %eleve.etablissement_id,
                key_len = key.len(),
                "photo_key non conforme : lecture du magasin refusée"
            );
            return Ok(None);
        }
        Ok(Some(key.clone()))
    }

    pub async fn upsert_param_etab_for_actor(
        &self,
        role: &str,
        cmd: UpsertParamEtab,
    ) -> Result<ParamEtab, EntDbError> {
        if !Self::role_can_manage_eleves(role) {
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
        if !Self::role_can_manage_edt(role) && !Self::role_can_manage_eleves(role) {
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
        if !Self::role_can_manage_eleves(role) {
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
        let staff = Self::role_can_manage_eleves(role) || parsed.can_manage_continuum();
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
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id).await?;
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_manage_eleves(role) || parsed.can_read_all_eleves();
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

    pub async fn retirer_voeu_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        eleve_id: Uuid,
        voeu_id: Uuid,
    ) -> Result<(), EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_manage_eleves(role) || parsed.can_manage_continuum();
        let parent = matches!(parsed, Role::Parent);
        let eleve = matches!(parsed, Role::Eleve);
        if !staff && !parent && !eleve {
            return Err(EntDbError::Forbidden);
        }
        if parent
            && self
                .lien_parent_eleve(etablissement_id, actor_user_id, eleve_id)
                .await?
                .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        if eleve {
            let linked = self
                .eleve_comptes
                .eleve_id_for_user(etablissement_id, actor_user_id)
                .await?;
            if linked != Some(eleve_id) {
                return Err(EntDbError::Forbidden);
            }
        }
        self.eleves
            .get(etablissement_id, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let deleted = self
            .filieres
            .delete_voeu(etablissement_id, eleve_id, voeu_id)
            .await?;
        if !deleted {
            return Err(DomainError::NotFound.into());
        }
        Ok(())
    }

    pub async fn upsert_inscription_suivi_for_actor(
        &self,
        role: &str,
        cmd: UpsertInscriptionSuivi,
    ) -> Result<InscriptionSuivi, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !Self::role_can_manage_eleves(role) && !parsed.can_manage_continuum() {
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
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id).await?;
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = Self::role_can_manage_eleves(role) || parsed.can_read_all_eleves();
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
        if !Self::role_can_manage_eleves(role) {
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
        actor_user_id: Uuid,
        cmd: ExporterLsl,
    ) -> Result<String, EntDbError> {
        let cmd = plan_exporter_lsl(cmd)?;
        self.exiger_export_officiel(cmd.etablissement_id, actor_user_id, role)
            .await?;
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
        // RF-04 (d) : un professeur n'exporte que les classes qu'il enseigne.
        self.exiger_classe_dans_perimetre(cmd.etablissement_id, actor_user_id, role, classe)
            .await?;
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
        actor_user_id: Uuid,
        cmd: ExporterOnde,
    ) -> Result<String, EntDbError> {
        let cmd = plan_exporter_onde(cmd)?;
        self.exiger_export_officiel(cmd.etablissement_id, actor_user_id, role)
            .await?;
        let eleves = if let Some(ref classe) = cmd.classe {
            self.eleves
                .list_en_classe_au(cmd.etablissement_id, classe, cmd.jour_reference)
                .await?
        } else {
            // Sans filtre classe : tous les élèves actifs au jour (via search large).
            self.eleves
                .search(cmd.etablissement_id, "", 500, true)
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
            || Self::role_can_lire_facturation(role)
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
        actor_user_id: Uuid,
        cmd: ExporterNotanet,
        jour: NaiveDate,
    ) -> Result<String, EntDbError> {
        let cmd = plan_exporter_notanet(cmd)?;
        self.exiger_export_officiel(cmd.etablissement_id, actor_user_id, role)
            .await?;
        // RF-04 (d) : un professeur n'exporte que les classes qu'il enseigne.
        self.exiger_classe_dans_perimetre(cmd.etablissement_id, actor_user_id, role, &cmd.classe)
            .await?;
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
        actor_user_id: Uuid,
        cmd: ExporterParcoursup,
        jour: NaiveDate,
    ) -> Result<String, EntDbError> {
        let cmd = plan_exporter_parcoursup(cmd)?;
        self.exiger_export_officiel(cmd.etablissement_id, actor_user_id, role)
            .await?;
        // RF-04 (d) : un professeur n'exporte que les classes qu'il enseigne.
        self.exiger_classe_dans_perimetre(cmd.etablissement_id, actor_user_id, role, &cmd.classe)
            .await?;
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
            || Self::role_can_manage_eleves(role)
            || matches!(parsed, Role::Direction | Role::Admin);
        if !self_or_rh {
            return Err(EntDbError::Forbidden);
        }
        // Socle 1 (C1) : la personne absente doit être membre de l'établissement.
        if !self.auth.is_member(cmd.etablissement_id, cmd.user_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let planned = plan_declarer_absence_rh(cmd)?;
        // S2-5 (RH-02) : l'absence et son événement entrent dans l'outbox dans la même
        // transaction. Le motif (texte libre, parfois médical) n'est jamais dans l'événement.
        let mut tx = self.pool().begin().await.map_err(|e| {
            EntDbError::Domain(crate::pg_err::map_sqlx("absence_rh", e))
        })?;
        sqlx::query(
            "INSERT INTO ent_absence_rh (
                id, etablissement_id, user_id, debut, fin, motif, statut,
                created_by_user_id, created_at
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(planned.id)
        .bind(planned.etablissement_id)
        .bind(planned.user_id)
        .bind(planned.debut)
        .bind(planned.fin)
        .bind(&planned.motif)
        .bind(&planned.statut)
        .bind(planned.created_by_user_id)
        .bind(planned.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| EntDbError::Domain(crate::pg_err::map_sqlx("absence_rh", e)))?;
        let ev = {
            use crate::outbox::Publier;
            let conn: &mut sqlx::PgConnection = &mut tx;
            conn.publier(MetierEventDraft {
                etablissement_id: planned.etablissement_id,
                type_name: types::ABSENCE_RH.to_string(),
                aggregate: AggregateKind::AbsenceRh,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "user_id": planned.user_id,
                    "debut": planned.debut.to_string(),
                    "fin": planned.fin.to_string(),
                }),
                actor_user_id: Some(planned.created_by_user_id.to_string()),
            })
            .await?
        };
        tx.commit()
            .await
            .map_err(|e| EntDbError::Domain(crate::pg_err::map_sqlx("absence_rh", e)))?;
        self.diffuser_apres_commit(&[ev]);
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
        if !Self::role_can_manage_eleves(role) && !matches!(parsed, Role::Direction | Role::Admin) {
            return Err(EntDbError::Forbidden);
        }
        let current = self
            .absences_rh
            .get(etablissement_id, absence_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if current.statut == "validee" {
            return Ok(current); // idempotent : pas de second événement
        }
        let planned = plan_valider_absence_rh(current, actor_user_id)?;
        // S2-5 (RH-03) : validation et événement `rh.absence_validee` dans la même transaction.
        let mut tx = self.pool().begin().await.map_err(|e| {
            EntDbError::Domain(crate::pg_err::map_sqlx("absence_rh", e))
        })?;
        sqlx::query(
            "UPDATE ent_absence_rh SET statut = $3, created_by_user_id = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(planned.etablissement_id)
        .bind(planned.id)
        .bind(&planned.statut)
        .bind(planned.created_by_user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| EntDbError::Domain(crate::pg_err::map_sqlx("absence_rh", e)))?;
        let ev = {
            use crate::outbox::Publier;
            let conn: &mut sqlx::PgConnection = &mut tx;
            conn.publier(MetierEventDraft {
                etablissement_id,
                type_name: types::ABSENCE_RH_VALIDEE.to_string(),
                aggregate: AggregateKind::AbsenceRh,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "statut": planned.statut,
                    "user_id": planned.user_id,
                    "debut": planned.debut.to_string(),
                    "fin": planned.fin.to_string(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?
        };
        tx.commit()
            .await
            .map_err(|e| EntDbError::Domain(crate::pg_err::map_sqlx("absence_rh", e)))?;
        self.diffuser_apres_commit(&[ev]);
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

        // Le professeur affecté est fourni par le client : il doit être membre
        // de CET établissement (C6, D-013).
        if let Some(prof) = cmd.prof_user_id {
            if !self.auth.is_member(cmd.etablissement_id, prof).await? {
                return Err(DomainError::NotFound.into());
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
        if !Self::role_can_manage_eleves(role) && !matches!(parsed, Role::Cpe | Role::Direction | Role::Admin)
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
        if !Self::role_can_manage_eleves(role)
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
        if !Self::role_can_manage_eleves(role)
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
        self.exiger_perimetre_prof(etablissement_id, actor_user_id, role, eleve_id).await?;
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
        actor_user_id: Uuid,
        cmd: ExporterLsun,
    ) -> Result<String, EntDbError> {
        let cmd = plan_exporter_lsun(cmd)?;
        self.exiger_export_officiel(cmd.etablissement_id, actor_user_id, role)
            .await?;
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
        // RF-04 (d) : un professeur n'exporte que les classes qu'il enseigne.
        self.exiger_classe_dans_perimetre(cmd.etablissement_id, actor_user_id, role, classe)
            .await?;
        let eleves = self
            .eleves
            .list_en_classe_au(cmd.etablissement_id, classe, cmd.jour_reference)
            .await?;
        let eleve_ids: Vec<Uuid> = eleves.iter().map(|e| e.id).collect();
        // Le LSU ne se devine plus depuis une appréciation : on lit les évaluations
        // de compétences réellement saisies. Un élève sans évaluation sort une ligne
        // « non évalué » (domaine/item/niveau vides) — on n'invente aucun niveau.
        let evals = self
            .competences
            .list_evaluations_classe(cmd.etablissement_id, &eleve_ids, cmd.periode_id)
            .await?;
        let mut lignes = Vec::new();
        for e in &eleves {
            let mine: Vec<&EvaluationLue> = evals
                .iter()
                .filter(|(eid, _)| *eid == e.id)
                .map(|(_, l)| l)
                .collect();
            if mine.is_empty() {
                lignes.push(LigneLsun {
                    ine: e.ine.clone().unwrap_or_default(),
                    nom: e.nom.clone(),
                    prenom: e.prenom.clone(),
                    classe: e.classe.clone().unwrap_or_default(),
                    domaine: String::new(),
                    item: String::new(),
                    niveau: String::new(),
                    periode: periode.code.clone(),
                });
                continue;
            }
            for l in mine {
                lignes.push(LigneLsun {
                    ine: e.ine.clone().unwrap_or_default(),
                    nom: e.nom.clone(),
                    prenom: e.prenom.clone(),
                    classe: e.classe.clone().unwrap_or_default(),
                    domaine: l.domaine.clone(),
                    item: l.item_libelle.clone(),
                    niveau: l.niveau.code_lsu().to_string(),
                    periode: periode.code.clone(),
                });
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
        self.exiger_perimetre_prof(etablissement_id, user_id, role, eleve_id).await?;
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
        .map(|mut l: Vec<DispenseVs>| {
            // RP1-20 : le professeur et le reste du personnel non soignant voient la
            // dispense (dates, motif général) mais pas le commentaire libre, qui porte
            // le motif médical.
            if parsed.is_staff() && !matches!(parsed, Role::Admin | Role::Direction | Role::Cpe | Role::Infirmerie | Role::Psy) {
                for d in &mut l {
                    d.commentaire.clear();
                    if d.motif == domain_eleve::DispenseVsMotif::Medical {
                        d.motif = domain_eleve::DispenseVsMotif::Autre;
                    }
                }
            }
            l
        })
    }

    pub async fn list_dispenses_vs_jour_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<DispenseVs>, EntDbError> {
        if Self::role_can_gerer_dispenses_vs(role) {
            return Ok(self
                .dispenses_vs
                .list_actives_au_jour(etablissement_id, jour)
                .await?);
        }
        if !Role::parse(role)
            .map(|r| r.can_faire_appel() || r.can_lire_vs_absences())
            .unwrap_or(false)
        {
            return Err(EntDbError::Forbidden);
        }
        // Revue finale socle 1 : hors gestionnaires des dispenses, la liste (motif
        // médical, commentaire) est bornée aux élèves suivis (comme la route par élève).
        let toutes = self
            .dispenses_vs
            .list_actives_au_jour(etablissement_id, jour)
            .await?;
        match self
            .eleves_autorises(etablissement_id, actor_user_id, role, Action::SuiviEleve)
            .await?
        {
            None => Ok(toutes),
            Some(ids) => Ok(toutes.into_iter().filter(|d| ids.contains(&d.eleve_id)).collect()),
        }
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

    pub async fn creer_passage_infirmerie_for_actor(
        &self,
        role: &str,
        cmd: CreerPassageInfirmerie,
    ) -> Result<PassageInfirmerie, EntDbError> {
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::SanteSaisir,
                cmd.eleve_id,
            )
            .await?
        {
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
                // Minimisation (S2-11) : ni motif, ni soins dans l'événement.
                payload: json!({
                    "jour": passage.jour.to_string(),
                    "visible_famille": passage.visible_famille,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(passage)
    }

    async fn list_passages_infirmerie_for_actor_sans_trace(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<PassageInfirmerie>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff_saisie = match eleve_id {
            Some(eid) => {
                self.staff_eleve(etablissement_id, actor_user_id, role, Action::SanteLire, eid)
                    .await?
            }
            None => {
                self.a_action_quelconque(etablissement_id, actor_user_id, role, Action::SanteLire)
                    .await?
            }
        };
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

        // Listes du jour : limitées aux élèves couverts par la portée de `sante.lire`.
        if eleve_id.is_none() && staff_saisie {
            if let Some(ens) = self
                .eleves_autorises(etablissement_id, actor_user_id, role, Action::SanteLire)
                .await?
            {
                list.retain(|p| ens.contains(&p.eleve_id));
            }
        }
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
        let staff = self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::SanteSaisir,
                cmd.eleve_id,
            )
            .await?;
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
                // Minimisation (S2-11) : aucune donnée de santé dans l'événement.
                payload: json!({
                    "pai_actif": fiche.pai_actif,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(fiche)
    }

    async fn get_fiche_sante_for_actor_sans_trace(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<FicheSante>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = self
            .staff_eleve(etablissement_id, actor_user_id, role, Action::SanteLire, eleve_id)
            .await?;
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

    /// PAI (création ou mise à jour). Délègue au module santé (S2-11) : une seule
    /// transaction (PAI chiffré, version, contraintes dérivées, événement minimal).
    pub async fn upsert_pai_for_actor(
        &self,
        role: &str,
        cmd: UpsertPai,
    ) -> Result<Pai, EntDbError> {
        self.enregistrer_pai_for_actor(role, cmd, None).await
    }

    pub(crate) async fn get_pai_for_actor_sans_trace(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<Pai>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = self
            .staff_eleve(etablissement_id, actor_user_id, role, Action::SanteLire, eleve_id)
            .await?;
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

    /// Signature du PAI par la famille (version courante). Délègue au module santé.
    pub async fn signer_pai_famille_for_actor(
        &self,
        role: &str,
        cmd: SignerPaiFamille,
    ) -> Result<Pai, EntDbError> {
        let complet = self
            .signer_pai_for_actor(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                cmd.eleve_id,
                "famille",
                None,
            )
            .await?;
        Ok(vue_pai_famille(&complet.pai))
    }

    pub async fn attacher_ordonnance_pai_for_actor(
        &self,
        role: &str,
        cmd: AttacherOrdonnancePai,
    ) -> Result<PaiPiece, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff = self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::SanteSaisir,
                cmd.eleve_id,
            )
            .await?;
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
        let pai = self
            .infirmerie
            .get_pai(cmd.etablissement_id, cmd.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        let actor = cmd.actor_user_id;
        // D2 : signature binaire + liste blanche PAI (PDF, PNG, JPEG) + nom assaini.
        let mut cmd = cmd;
        let valide = self
            .uploads
            .valider(UsageFichier::PiecePai, &cmd.nom, Some(&cmd.mime_type), &cmd.contenu)
            .map_err(EntDbError::Fichier)?;
        cmd.nom = valide.nom.clone();
        cmd.mime_type = valide.mime.to_string();
        let mut piece = plan_attacher_ordonnance_pai(&pai, cmd)?;
        let ext = valide.extension;
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

    /// Contrôle d'accès commun au téléchargement ET à l'émission d'un lien de pièce PAI
    /// (sans lire le contenu) : staff / parent lié + PAI actif.
    async fn pai_piece_autorisee_sans_trace(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<PaiPiece, EntDbError> {
        let piece = self
            .infirmerie
            .get_pai_piece(etablissement_id, piece_id)
            .await?
            .ok_or(DomainError::PaiPieceIntrouvable)?;
        // Réutilise le contrôle d'accès get_pai (staff / parent lié + actif).
        let _ = self
            .get_pai_for_actor_sans_trace(etablissement_id, actor_user_id, role, piece.eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        Ok(piece)
    }

    async fn telecharger_pai_piece_for_actor_sans_trace(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<PaiPiece, EntDbError> {
        let mut piece = self
            .pai_piece_autorisee_sans_trace(etablissement_id, actor_user_id, role, piece_id)
            .await?;
        if let Some(ref key) = piece.storage_key {
            // Clé non conforme : jamais lue dans le magasin (traité comme sans contenu).
            if pai_storage_key_is_owned(&piece, key) {
                piece.contenu = self.objects.get(key)?;
            }
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
        let staff = self
            .staff_eleve(etablissement_id, actor_user_id, role, Action::SanteLire, eleve_id)
            .await?;
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
                    "octets": bytes.len(),
                }),
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(bytes)
    }

    pub async fn creer_entretien_psy_for_actor(
        &self,
        role: &str,
        cmd: CreerEntretienPsy,
    ) -> Result<EntretienPsy, EntDbError> {
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::PsySaisir,
                cmd.eleve_id,
            )
            .await?
        {
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

    async fn list_entretiens_psy_for_actor_sans_trace(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<EntretienPsy>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let staff_psy = match eleve_id {
            Some(eid) => {
                self.staff_eleve(etablissement_id, actor_user_id, role, Action::PsyLire, eid)
                    .await?
            }
            None => {
                self.a_action_quelconque(etablissement_id, actor_user_id, role, Action::PsyLire)
                    .await?
            }
        };
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

        // Listes du jour : limitées aux élèves couverts par la portée de `psy.lire`.
        if eleve_id.is_none() && staff_psy {
            if let Some(ens) = self
                .eleves_autorises(etablissement_id, actor_user_id, role, Action::PsyLire)
                .await?
            {
                list.retain(|x| ens.contains(&x.eleve_id));
            }
        }
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
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::PsySaisir,
                cmd.eleve_id,
            )
            .await?
        {
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
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(dossier)
    }

    async fn get_dossier_psy_for_actor_sans_trace(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<DossierPsy>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if self
            .staff_eleve(etablissement_id, actor_user_id, role, Action::PsyLire, eleve_id)
            .await?
        {
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
        let staff_psy = self
            .staff_eleve(etablissement_id, actor_user_id, role, Action::PsyLire, eleve_id)
            .await?;
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
        if !self
            .staff_eleve(etablissement_id, actor_user_id, role, Action::PsySaisir, eleve_id)
            .await?
        {
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
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::PsySaisir,
                cmd.eleve_id,
            )
            .await?
        {
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
        self.exiger_action_quelconque(
            cmd.etablissement_id,
            cmd.actor_user_id,
            role,
            Action::PsySaisir,
        )
        .await?;
        let current = self
            .psy
            .get_rdv(cmd.etablissement_id, cmd.rdv_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !self
            .staff_eleve(
                cmd.etablissement_id,
                cmd.actor_user_id,
                role,
                Action::PsySaisir,
                current.eleve_id,
            )
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
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
        let staff_psy = match eleve_id {
            Some(eid) => {
                self.staff_eleve(etablissement_id, actor_user_id, role, Action::PsyLire, eid)
                    .await?
            }
            None => {
                self.a_action_quelconque(etablissement_id, actor_user_id, role, Action::PsyLire)
                    .await?
            }
        };
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

        // Listes du jour : limitées aux élèves couverts par la portée de `psy.lire`.
        if eleve_id.is_none() && staff_psy {
            if let Some(ens) = self
                .eleves_autorises(etablissement_id, actor_user_id, role, Action::PsyLire)
                .await?
            {
                list.retain(|x| ens.contains(&x.eleve_id));
            }
        }
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
        // RF-04 (g) : un professeur n'écrit que dans le cahier de ses classes.
        self.exiger_classe_dans_perimetre(cmd.etablissement_id, cmd.actor_user_id, role, &cmd.classe)
            .await?;
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
        // D-017 : « messagerie.eleve » et sa portée (professeur : ses classes).
        if parsed.is_staff() {
            return self
                .autoriser_eleve(etablissement_id, actor_user_id, role, Action::MessagerieEleve, eleve_id)
                .await;
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

    /// Accès à un fil existant (socle 1, B2) : règle de l'élève du fil. Pour un
    /// professeur, l'élève doit être **encore** l'un des siens : avoir participé
    /// au fil ne suffit pas (revue E6 : changement de classe, fin de remplacement).
    pub async fn actor_can_access_fil(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        fil: &MsgFil,
    ) -> Result<bool, EntDbError> {
        if !self
            .actor_can_access_msg_fil(etablissement_id, actor_user_id, role, fil.eleve_id)
            .await?
        {
            return Ok(false);
        }
        // D-014 (RP1-11) : un compte famille ne lit que les fils auxquels il participe.
        if matches!(Role::parse(role), Some(Role::Parent | Role::Eleve)) {
            return Ok(self
                .messages
                .fils_lisibles_famille(etablissement_id, actor_user_id, &[fil.id])
                .await?
                .contains(&fil.id));
        }
        Ok(true)
    }

    /// Ne garde que les fils lisibles par un compte famille (D-014, RP1-11).
    async fn filtrer_fils_famille(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        fils: Vec<MsgFil>,
    ) -> Result<Vec<MsgFil>, EntDbError> {
        let ids: Vec<Uuid> = fils.iter().map(|f| f.id).collect();
        let ok = self
            .messages
            .fils_lisibles_famille(etablissement_id, user_id, &ids)
            .await?;
        Ok(fils.into_iter().filter(|f| ok.contains(&f.id)).collect())
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
        // D-014 (RP1-11) : un fil ouvert par un responsable reste privé entre lui et
        // l'établissement ; un fil ouvert par l'établissement reste visible de tous les
        // responsables de l'élève.
        let prive = (Role::parse(role) == Some(Role::Parent)).then_some(fil.created_by_user_id);
        self.messages.insert_fil_portee(&fil, prive, None).await?;
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
            .notify_parents_push_fil(
                &fil,
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
            .actor_can_access_fil(cmd.etablissement_id, cmd.actor_user_id, role, &fil)
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
            .notify_parents_push_fil(
                &fil,
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
        // Socle 1 (B2) : un professeur ne voit que les fils de ses élèves (revue E6),
        // jamais toute la boîte de l'établissement.
        let perimetre = if parsed.is_staff() {
            self.eleves_autorises(etablissement_id, actor_user_id, role, Action::MessagerieEleve)
                .await?
        } else {
            None
        };
        if let Some(suivis) = perimetre.filter(|s| !s.is_empty() || Self::role_can_messagerie_staff(role)) {
            let mut fils = if let Some(eid) = eleve_id {
                if suivis.contains(&eid) {
                    self.messages.list_fils_eleve(etablissement_id, eid).await?
                } else {
                    Vec::new()
                }
            } else {
                let ids: Vec<Uuid> = suivis.into_iter().collect();
                self.messages
                    .list_fils_eleves(etablissement_id, &ids)
                    .await?
            };
            let dossier = dossier.map(str::trim).filter(|d| !d.is_empty());
            fils.retain(|f| {
                (include_archives || eleve_id.is_some() || !f.archive)
                    && dossier.is_none_or(|d| f.dossier.as_deref() == Some(d))
            });
            fils.sort_by(|a, b| {
                (a.priorite != MsgPriorite::Haute)
                    .cmp(&(b.priorite != MsgPriorite::Haute))
                    .then(b.last_message_at.cmp(&a.last_message_at))
            });
            fils.truncate(50);
            return Ok(fils);
        }
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
                let fils = self.messages.list_fils_eleve(etablissement_id, eid).await?;
                return self
                    .filtrer_fils_famille(etablissement_id, actor_user_id, fils)
                    .await;
            }
            let fils = self
                .messages
                .list_fils_eleves(etablissement_id, &lies)
                .await?;
            return self
                .filtrer_fils_famille(etablissement_id, actor_user_id, fils)
                .await;
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
            let fils = self
                .messages
                .list_fils_eleve(etablissement_id, linked)
                .await?;
            return self
                .filtrer_fils_famille(etablissement_id, actor_user_id, fils)
                .await;
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
            .actor_can_access_fil(etablissement_id, actor_user_id, role, &fil)
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
            .actor_can_access_fil(cmd.etablissement_id, cmd.actor_user_id, role, &fil)
            .await?;
        if !can {
            return Err(EntDbError::Forbidden);
        }
        // D2 : signature binaire + liste blanche + nom assaini (le contenu reste en base, P0).
        let mut cmd = cmd;
        let valide = self
            .uploads
            .valider(UsageFichier::PieceMessage, &cmd.nom, Some(&cmd.mime_type), &cmd.contenu)
            .map_err(EntDbError::Fichier)?;
        cmd.nom = valide.nom;
        cmd.mime_type = valide.mime.to_string();
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
            .actor_can_access_fil(etablissement_id, actor_user_id, role, &fil)
            .await?;
        if !can {
            return Err(EntDbError::Forbidden);
        }
        Ok(piece)
    }

    // ───────────── Liens courts d'accès aux fichiers (URL pré-signées) ─────────────
    //
    // Chaque émission passe par EXACTEMENT le même contrôle d'accès que le
    // téléchargement correspondant (fonctions partagées ci-dessus) ; le client ne
    // fournit jamais de clé de stockage.

    fn emettre_lien(
        &self,
        key: &str,
        user_id: Uuid,
        etablissement_id: Uuid,
        disposition: Disposition,
        filename: &str,
        mime: &str,
    ) -> Result<LienFichier, EntDbError> {
        let link = self
            .presigner
            .issue(
                &LinkSpec {
                    key: key.to_string(),
                    user_id,
                    etablissement_id,
                    disposition,
                    filename: filename.to_string(),
                    mime: mime.to_string(),
                },
                ent_object::presign::now_unix(),
            )
            .map_err(|e| match e {
                PresignError::Disabled => EntDbError::LiensIndisponibles,
                // Clé hors établissement / hostile : traitée comme fichier absent.
                _ => EntDbError::FichierAbsent,
            })?;
        Ok(LienFichier {
            url: link.url,
            expires_at: link.expires_at,
        })
    }

    /// Copie dans le magasin d'objets un contenu stocké en base (pièces de messagerie,
    /// pièces PAI historiques) pour pouvoir le servir par lien court. Idempotent.
    fn refleter_dans_magasin(&self, key: &str, bytes: &[u8]) -> Result<(), EntDbError> {
        if !self.objects.exists(key) {
            self.objects.put(key, bytes)?;
        }
        Ok(())
    }

    /// Lien court vers le fichier d'un document (mêmes droits que le téléchargement).
    pub async fn lien_document_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        document_id: Uuid,
    ) -> Result<LienFichier, EntDbError> {
        if !self.presigner.is_enabled() {
            return Err(EntDbError::LiensIndisponibles);
        }
        let doc = self
            .document_lisible_for_actor(etablissement_id, actor_user_id, role, document_id)
            .await?;
        let key = doc
            .storage_key
            .as_deref()
            .filter(|k| document_storage_key_is_owned(&doc, k) && self.objects.exists(k))
            .ok_or(EntDbError::FichierAbsent)?;
        let (nom, mime) = nom_fichier_document(&doc);
        let lien = self.emettre_lien(
            key,
            actor_user_id,
            etablissement_id,
            Disposition::Inline,
            &nom,
            &mime,
        )?;
        self.tracer_acces_sensible(
            etablissement_id,
            actor_user_id,
            role,
            Some(doc.eleve_id),
            "telechargement",
            "lien_document",
            &format!("Lien court vers le document {document_id}"),
        )
        .await;
        Ok(lien)
    }

    /// Lien court vers la photo d'un élève (mêmes droits que `GET /eleves/{id}/photo`).
    /// `None` : pas de photo.
    pub async fn lien_photo_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<LienFichier>, EntDbError> {
        if !self.presigner.is_enabled() {
            return Err(EntDbError::LiensIndisponibles);
        }
        let Some(key) = self
            .photo_cle_for_actor(etablissement_id, actor_user_id, role, eleve_id)
            .await?
        else {
            return Ok(None);
        };
        if !self.objects.exists(&key) {
            return Ok(None);
        }
        // La route de téléchargement sert toujours « image/jpeg ».
        Ok(Some(self.emettre_lien(
            &key,
            actor_user_id,
            etablissement_id,
            Disposition::Inline,
            "photo.jpg",
            "image/jpeg",
        )?))
    }

    /// Lien court vers une pièce du PAI (mêmes droits que le téléchargement ; trace d'accès).
    pub async fn lien_pai_piece_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<LienFichier, EntDbError> {
        if !self.presigner.is_enabled() {
            return Err(EntDbError::LiensIndisponibles);
        }
        let piece = self
            .pai_piece_autorisee_sans_trace(etablissement_id, actor_user_id, role, piece_id)
            .await?;
        let key = match piece.storage_key.as_deref() {
            Some(k) if pai_storage_key_is_owned(&piece, k) => k.to_string(),
            // Pièce historique stockée en base : recopiée sous une clé conforme.
            _ if !piece.contenu.is_empty() => {
                let ext = piece.nom.rsplit_once('.').map(|(_, e)| e).unwrap_or("bin");
                let k = ObjectStore::key_for(piece.etablissement_id, "pai", piece.id, ext);
                self.refleter_dans_magasin(&k, &piece.contenu)?;
                k
            }
            _ => return Err(EntDbError::FichierAbsent),
        };
        if !self.objects.exists(&key) {
            return Err(EntDbError::FichierAbsent);
        }
        let lien = self.emettre_lien(
            &key,
            actor_user_id,
            etablissement_id,
            Disposition::Attachment,
            &piece.nom,
            &piece.mime_type,
        )?;
        self.tracer_acces_sensible(
            etablissement_id,
            actor_user_id,
            role,
            Some(piece.eleve_id),
            "consultation",
            "lien_pai_piece",
            "Lien court vers une pièce du PAI",
        )
        .await;
        Ok(lien)
    }

    /// Lien court vers une pièce jointe de messagerie (mêmes droits que le téléchargement).
    /// Le contenu, stocké en base, est recopié dans le magasin d'objets (idempotent).
    pub async fn lien_piece_message_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        piece_id: Uuid,
    ) -> Result<LienFichier, EntDbError> {
        if !self.presigner.is_enabled() {
            return Err(EntDbError::LiensIndisponibles);
        }
        let piece = self
            .telecharger_piece_for_actor(etablissement_id, actor_user_id, role, piece_id)
            .await?;
        if piece.contenu.is_empty() {
            return Err(EntDbError::FichierAbsent);
        }
        let ext = piece.nom.rsplit_once('.').map(|(_, e)| e).unwrap_or("bin");
        let key = ObjectStore::key_for(piece.etablissement_id, "messages", piece.id, ext);
        self.refleter_dans_magasin(&key, &piece.contenu)?;
        let lien = self.emettre_lien(
            &key,
            actor_user_id,
            etablissement_id,
            Disposition::Attachment,
            &piece.nom,
            &piece.mime_type,
        )?;
        self.tracer_acces_sensible(
            etablissement_id,
            actor_user_id,
            role,
            None,
            "telechargement",
            "lien_piece_message",
            &format!("Lien court vers la pièce jointe {piece_id}"),
        )
        .await;
        Ok(lien)
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
        if !self
            .actor_can_access_msg_fil(etablissement_id, actor_user_id, role, eleve_id)
            .await?
        {
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
        if !self
            .actor_can_access_fil(cmd.etablissement_id, cmd.actor_user_id, role, &fil)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
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
        if !self
            .actor_can_access_fil(cmd.etablissement_id, cmd.actor_user_id, role, &fil)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
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
        Self::peut_etablissement(role, Action::FacturationGerer)
    }

    /// Consulter la facturation : `facturation.lire`, ou `facturation.gerer` qui l'inclut.
    pub fn role_can_lire_facturation(role: &str) -> bool {
        Self::peut_etablissement(role, Action::FacturationLire)
    }

    /// Foyers d'un parent pour la **facturation** : seuls les foyers dont il est
    /// **payeur** (`ent_eleve_lien.est_payeur`). Un responsable sans charge de
    /// paiement (parent séparé non payeur, tiers lié) ne voit ni les factures,
    /// ni l'historique bancaire, ni les mandats SEPA, et ne peut ni créer ni
    /// révoquer de mandat (RS3-01 : le chemin historique `/facturation/*` ne
    /// filtrait pas `est_payeur`, contrairement au chemin `/finances/*`).
    async fn foyer_ids_parent(
        &self,
        etablissement_id: Uuid,
        parent_user_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let liens = self
            .foyers
            .liens_for_parent(etablissement_id, parent_user_id)
            .await?;
        let mut ids: Vec<Uuid> = liens
            .into_iter()
            .filter(|l| l.est_payeur)
            .filter_map(|l| l.foyer_id)
            .collect();
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
        if Self::role_can_lire_facturation(role) {
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
        // Une ligne ne peut viser qu'un élève de l'établissement (C6, D-013).
        for l in &lignes {
            if let Some(eid) = l.eleve_id {
                self.eleves
                    .get(facture.etablissement_id, eid)
                    .await?
                    .ok_or(DomainError::NotFound)?;
            }
        }
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

        // Répercussion famille : une facture émise est notifiée aux responsables.
        if let Some(eleve_id) = self
            .facturation
            .first_eleve_id_foyer(emise.etablissement_id, emise.foyer_id)
            .await?
        {
            let _ = self
                .notify_payeurs_foyer_mail(
                    emise.etablissement_id,
                    emise.foyer_id,
                    eleve_id,
                    "facture",
                    &format!("Facture {}", emise.numero),
                    &format!(
                        "Votre facture {} d'un montant de {} est disponible dans le portail famille (rubrique Facturation).",
                        emise.numero,
                        domain_eleve::euros_centimes(emise.total_centimes)
                    ),
                    "facture",
                    emise.id,
                    Some(actor_user_id),
                    Vec::new(),
                )
                .await;
        }
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
        if Self::role_can_lire_facturation(role) {
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

    pub async fn list_factures_famille_enriched(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Vec<(Facture, i64, i64)>, EntDbError> {
        let list = self
            .list_factures_for_actor(etablissement_id, actor_user_id, role)
            .await?;
        let mut out = Vec::with_capacity(list.len());
        for f in list {
            let encaisse = self
                .facturation
                .sum_encaissements(etablissement_id, f.id)
                .await?;
            let reste = reste_a_payer(f.total_centimes, encaisse);
            out.push((f, encaisse, reste));
        }
        Ok(out)
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
        if Self::role_can_lire_facturation(role) {
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
        // Répercussion famille : le paiement est confirmé aux responsables, avec
        // la quittance PDF en pièce jointe.
        if let Some(eleve_id) = self
            .facturation
            .first_eleve_id_foyer(facture.etablissement_id, facture.foyer_id)
            .await?
        {
            let pdf = domain_eleve::render_quittance_pdf(&quittance, &facture.numero);
            let _ = self
                .notify_payeurs_foyer_mail(
                    facture.etablissement_id,
                    facture.foyer_id,
                    eleve_id,
                    "quittance",
                    &format!(
                        "Quittance {} — facture {}",
                        quittance.numero, facture.numero
                    ),
                    &format!(
                        "Nous avons reçu votre paiement de {} (facture {}). Quittance ci-jointe.",
                        domain_eleve::euros_centimes(quittance.montant_centimes),
                        facture.numero
                    ),
                    "quittance",
                    quittance.id,
                    enc.created_by_user_id,
                    vec![MailAttachment {
                        filename: format!("quittance-{}.pdf", quittance.numero),
                        mime: "application/pdf".into(),
                        bytes: pdf,
                    }],
                )
                .await;
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
                    // D3 : le journal d'événements ne porte jamais l'IBAN complet.
                    "iban": domain_eleve::masquer_iban(&histo.iban),
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
        if Self::role_can_lire_facturation(role) {
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
        if Self::role_can_lire_facturation(role) {
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
                    // D-014 (RP1-11) : la relance n'est lisible que des payeurs du foyer.
                    self.messages
                        .insert_fil_portee(&fil, None, Some(cand.foyer_id))
                        .await?;
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
        if !Self::role_can_lire_facturation(role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(self
            .facturation
            .list_rappels_etab(etablissement_id, 80)
            .await?)
    }

    /// Copilote — répond à une question du personnel en **lecture seule**.
    ///
    /// Déterministe : intention → requête sur les tables centrales, toujours
    /// filtrée par `etablissement_id`. Le rôle est vérifié **par intention** ;
    /// une question interdite renvoie une réponse `refusee = true` (pas d'erreur
    /// HTTP) pour que l'UI l'affiche simplement.
    pub async fn copilote_repondre(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        question: &str,
    ) -> Result<ReponseCopilote, EntDbError> {
        let r = Role::parse(role).unwrap_or(Role::Parent);
        let jour = today_utc();
        let reponse = match copilote::analyser(question) {
            copilote::Intention::Capacites => ReponseCopilote::new(
                "capacites",
                question,
                "Je peux répondre à :",
                json!({ "capacites": copilote::CAPACITES }),
            ),
            copilote::Intention::Inconnu => ReponseCopilote::new(
                "inconnu",
                question,
                "Je ne comprends pas encore cette question. Essayez par exemple :",
                json!({ "capacites": copilote::CAPACITES }),
            ),
            copilote::Intention::Effectif { classe } => {
                if !r.can_read_effectif() {
                    ReponseCopilote::refus("effectif", question)
                } else {
                    match classe {
                        Some(c) => {
                            let n = self
                                .eleves_en_classe_au(etablissement_id, &c, jour)
                                .await?
                                .len();
                            ReponseCopilote::new(
                                "effectif",
                                question,
                                format!("Effectif {c} : {n} élève(s) au {jour}."),
                                json!({ "classe": c, "effectif": n, "au": jour.to_string() }),
                            )
                        }
                        None => {
                            let n = self.eleves.count_actifs_au(etablissement_id, jour).await?;
                            ReponseCopilote::new(
                                "effectif",
                                question,
                                format!("Effectif total : {n} élève(s) actif(s) au {jour}."),
                                json!({ "effectif": n, "au": jour.to_string() }),
                            )
                        }
                    }
                }
            }
            copilote::Intention::AbsencesNonJustifiees { classe, jours } => {
                if !r.can_lire_vs_absences() {
                    ReponseCopilote::refus("absences_non_justifiees", question)
                } else {
                    let depuis = jour - chrono::Duration::days(i64::from(jours));
                    let rows = self
                        .absences
                        .count_a_traiter_par_eleve_depuis(
                            etablissement_id,
                            depuis,
                            classe.as_deref(),
                            jour,
                        )
                        .await?;
                    // Revue finale socle 1 : même portée que `absences.lire` (professeur :
                    // ses élèves) ; avant, le copilote listait tout l'établissement.
                    let rows = match self
                        .eleves_autorises(etablissement_id, actor_user_id, role, Action::AbsencesLire)
                        .await?
                    {
                        None => rows,
                        Some(ids) => rows.into_iter().filter(|r| ids.contains(&r.0)).collect(),
                    };
                    let total: i64 = rows.iter().map(|(_, _, _, n)| *n).sum();
                    let contexte = match &classe {
                        Some(c) => format!("classe {c}, "),
                        None => String::new(),
                    };
                    let detail = rows
                        .iter()
                        .take(5)
                        .map(|(_, nom, prenom, n)| format!("{prenom} {nom} ({n})"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let eleves: Vec<_> = rows
                        .iter()
                        .map(|(id, nom, prenom, n)| {
                            json!({ "eleve_id": id, "nom": nom, "prenom": prenom, "total": n })
                        })
                        .collect();
                    let mut msg = format!(
                        "Absences non justifiées ({contexte}{jours} derniers jours, depuis {depuis}) : {total} au total chez {} élève(s).",
                        rows.len()
                    );
                    if !detail.is_empty() {
                        msg.push_str(&format!(" Détail : {detail}."));
                    }
                    ReponseCopilote::new(
                        "absences_non_justifiees",
                        question,
                        msg,
                        json!({
                            "classe": classe,
                            "jours": jours,
                            "depuis": depuis.to_string(),
                            "total": total,
                            "eleves": eleves,
                        }),
                    )
                }
            }
            copilote::Intention::Impayes => {
                if !Self::role_can_lire_facturation(role) {
                    ReponseCopilote::refus("impayes", question)
                } else {
                    let list = self
                        .list_factures_famille_enriched(etablissement_id, actor_user_id, role)
                        .await?;
                    let mut nb = 0i64;
                    let mut reste = 0i64;
                    for (f, _encaisse, reste_facture) in &list {
                        if f.statut != domain_eleve::FactureStatut::Soldee && *reste_facture > 0 {
                            nb += 1;
                            reste += *reste_facture;
                        }
                    }
                    ReponseCopilote::new(
                        "impayes",
                        question,
                        format!(
                            "Factures non soldées : {nb} facture(s), reste dû {}.",
                            copilote::format_eur(reste)
                        ),
                        json!({ "factures_non_soldees": nb, "reste_du_centimes": reste }),
                    )
                }
            }
            copilote::Intention::SansCompte => {
                if !Self::role_can_manage_eleves(role) {
                    ReponseCopilote::refus("sans_compte", question)
                } else {
                    let n = self
                        .list_eleves_sans_compte_for_actor(etablissement_id, role)
                        .await?
                        .len();
                    ReponseCopilote::new(
                        "sans_compte",
                        question,
                        format!("{n} élève(s) actif(s) sans compte ENT."),
                        json!({ "sans_compte": n }),
                    )
                }
            }
            copilote::Intention::AlertesCpe => {
                if !r.can_gerer_alertes_cpe() {
                    ReponseCopilote::refus("alertes_cpe", question)
                } else {
                    let n = self
                        .list_alertes_cpe_ouvertes_for_actor(etablissement_id, role)
                        .await?
                        .len();
                    ReponseCopilote::new(
                        "alertes_cpe",
                        question,
                        format!("{n} alerte(s) CPE ouverte(s)."),
                        json!({ "alertes_cpe": n }),
                    )
                }
            }
        };
        Ok(reponse)
    }
}

fn cand_corps(c: &domain_eleve::RappelCandidat) -> String {
    domain_eleve::corps_rappel(c)
}

impl std::fmt::Debug for PersonnelProvisionne {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersonnelProvisionne")
            .field("user_id", &self.user_id)
            .field("role", &self.role)
            .field("email", &"[masqué]")
            .field("invitation_envoyee", &self.invitation_envoyee)
            .finish()
    }
}

impl std::fmt::Debug for OidcProviderParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidcProviderParams")
            .field("slug", &self.slug)
            .field("issuer", &self.issuer)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[masqué]")
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for LoginOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginOutcome::Session(s) => f.debug_tuple("Session").field(s).finish(),
            LoginOutcome::PasskeyRequired { expires_at, .. } => f
                .debug_struct("PasskeyRequired")
                .field("token", &"[masqué]")
                .field("options", &"[masqué]")
                .field("expires_at", expires_at)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests_cles_objet_photo_pai {
    use super::*;

    fn eleve(etab: Uuid, id: Uuid) -> Eleve {
        Eleve {
            id,
            etablissement_id: etab,
            nom: "N".into(),
            prenom: "P".into(),
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: false,
            photo_key: None,
            status: domain_eleve::EleveStatus::Actif,
            classe: None,
            regime: None,
            mef: None,
            site: None,
            date_entree: None,
            date_sortie: None,
            motif_sortie: None,
        }
    }

    #[test]
    fn photo_cle_conforme_seulement() {
        let (etab, id) = (Uuid::new_v4(), Uuid::new_v4());
        let e = eleve(etab, id);
        assert!(photo_storage_key_is_owned(&e, &format!("{etab}/trombi/{id}.jpg")));
        assert!(!photo_storage_key_is_owned(
            &e,
            &format!("{etab}/trombi/{}.jpg", Uuid::new_v4())
        ));
        assert!(!photo_storage_key_is_owned(
            &e,
            &format!("{}/trombi/{id}.jpg", Uuid::new_v4())
        ));
        assert!(!photo_storage_key_is_owned(&e, &format!("{etab}/documents/{id}.jpg")));
        assert!(!photo_storage_key_is_owned(&e, &format!("{etab}/trombi/../{id}.jpg")));
        assert!(!photo_storage_key_is_owned(&e, "autre/secret.pdf"));
    }

    #[test]
    fn pai_cle_conforme_seulement() {
        let (etab, id) = (Uuid::new_v4(), Uuid::new_v4());
        let piece = PaiPiece {
            id,
            etablissement_id: etab,
            eleve_id: Uuid::new_v4(),
            pai_id: Uuid::new_v4(),
            nom: "x".into(),
            mime_type: "application/pdf".into(),
            taille_octets: 1,
            contenu: Vec::new(),
            storage_key: None,
            created_by_user_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };
        assert!(pai_storage_key_is_owned(&piece, &format!("{etab}/pai/{id}.pdf")));
        assert!(!pai_storage_key_is_owned(
            &piece,
            &format!("{etab}/pai/{}.pdf", Uuid::new_v4())
        ));
        assert!(!pai_storage_key_is_owned(&piece, &format!("{etab}/documents/{id}.pdf")));
        assert!(!pai_storage_key_is_owned(&piece, "lab/abcd1234/x.pdf"));
    }
}


/// Noms (jamais les valeurs) des champs d'identité qui diffèrent.
fn champs_identite_modifies(avant: &Eleve, apres: &Eleve) -> Vec<&'static str> {
    let mut c = Vec::new();
    if avant.nom != apres.nom {
        c.push("nom");
    }
    if avant.prenom != apres.prenom {
        c.push("prenom");
    }
    if avant.ine != apres.ine {
        c.push("ine");
    }
    if avant.sexe != apres.sexe {
        c.push("sexe");
    }
    if avant.date_naissance != apres.date_naissance {
        c.push("date_naissance");
    }
    if avant.numero_badge != apres.numero_badge {
        c.push("numero_badge");
    }
    if avant.redoublant != apres.redoublant {
        c.push("redoublant");
    }
    if avant.photo_key != apres.photo_key {
        c.push("photo");
    }
    if avant.date_entree != apres.date_entree {
        c.push("date_entree");
    }
    c
}

/// Entrée de journal (domaine « administratif ») écrite dans la transaction du
/// fait métier : une erreur remonte et défait la transaction.
async fn journal_eleve_tx(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    action: &str,
    libelle: &str,
    payload: serde_json::Value,
    actor: &Option<String>,
) -> Result<(), EntDbError> {
    let acteur = actor.as_deref().and_then(|s| Uuid::parse_str(s).ok());
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,$3,'administratif',$4,$5,$6,true,$7, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve_id)
    .bind(action)
    .bind(libelle)
    .bind(payload)
    .bind(acteur)
    .execute(conn)
    .await?;
    Ok(())
}
