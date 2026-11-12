use crate::auth::EntSession;
use crate::tenant::TenantHote;
use crate::ical::{render_edt_calendar, EdtOccurrence};
use crate::dto::{
    AbsenceDto, AbsentDto, AnnonceDto, AppelDto, AppelLigneDto, CampagneDto, CarnetEntreeDto, ChangeClasseBody,
    ChangeMefBody, ChangeRegimeBody, ChangeSiteBody, ChoisirOptionBody, CreateEleveBody,
    CreateMatiereBody, UpdateMatiereBody, CreerAnnonceBody, CreerCarnetBody,
    CreerDevoirBody, CreerFactureBody, CreerNotePeriodeBody, DeclareAbsenceBody, DevoirDto,
    DeclarerRetardBody, RetardDto,
    CreerCompetenceItemBody, CompetenceItemDto, EvaluerCompetenceBody, EvaluationCompetenceDto,
    EcheanceDto, EcheancierPartsBody, EdtCreneauDto, EdtExceptionDto, VersDto, EffectifDto, EleveDto,
    EncaissementDto,
    EncaisserBody, EnfantFamilleDto, FamilleResumeDto, FactureDetailDto, FactureDto,
    FactureFamilleListItemDto,
    DecisionAbsenceBody, FactureLigneDto, FilDetailDto, JustifierAbsenceBody, MarquerAppelLigneBody,
    MatiereDto,
    CandidatsMonteeLotDto, ConfigurerAcompteBody, CreerMandatSepaBody, DeposerDocReinscriptionBody,
    AttacherPieceBody, MessageEnvoyeDto, ReglagePlageCalmeBody, RetirerAnnonceBody, VerifierLienBody,
    ChangerStatutRdvPsyBody, CompteProvisionneDto, CreerEntretienPsyBody,
    CommissionDto, CommissionDossierDto, ConvoquerCommissionBody, CreerCommissionBody,
    CreerMsgModeleBody, CreerPassageInfirmerieBody, CreerRdvPsyBody, DeciderCommissionBody,
    DeclarerIncidentBody, DossierPsyDto, FicheSanteDto, IncidentDto, PaiDto, PaiPieceDto,
    CreerReferentielVsBody, PrononcerSanctionBody, ReferentielVsDto, SanctionDto, SoldePointsDto,
    EntretienPsyDto, ExigenceDocDto, InvitationCompteDto, InviterCompteBody, LotMonteeResultatDto,
    MandatSepaDto, MessageDto, MonteeLotBody, MonteeRapideBody, MoyenneMatiereDto, MsgFilDto,
    MsgModeleDto, MsgPieceDto, NoteDto, NotePeriodeDto, OptionAnneeDto, OuvrirAppelBody,
    OuvrirCampagneBody, OuvrirFilBody, OuvrirFilDepuisModeleBody, PassageInfirmerieDto, PeriodeDto,
    PlanifierPrelevementBody, PoserEcheancierBody, RdvPsyDto, UpsertDossierPsyBody,
    UpsertFicheSanteBody, UpsertPaiBody,
    CahierTexteDto, ClasserFilBody, ConfigurerEdtCalendrierAbBody, CreerCahierTexteBody,
    EdtCalendrierAbDto, PoserDispenseRepasBody, PoserEdtExceptionBody, PoserExigenceDocBody,
    PrelevementSepaDto,
    QuittanceDto, RappelFactureDto, ReinscriptionEnfantDto, ReinscriptionFamilleDto,
    RejeterPrelevementBody, RepondreFilBody,
    AffecterBourseBody, AppreciationDto, BourseTypeDto, CantineBadgeApercuDto,
    CantineBadgeFactureBody, CreerBourseTypeBody, CreerRepasActiviteBody, DeciderBourseBody,
    EleveBourseDto, GrilleRepasDto, PassageRepasDto, PoserGrilleRepasBody, RepasActiviteDto,
    PublierBulletinBody, SaisirAppreciationBody,
    ScolariteAnneeDto, ScanBadgeRepasBody, SignalAbsenceDto, SortirBody,
    VolumeRepasDto, CloreSignalBody, DispenseRepasDto,
    AutorisationSortieDto, ControleSortieBody, ControleSortieDto, DeciderSortieBody,
    DemandeSortieDto, DemanderSortieBody, RecapSortieDto, SortieRegleNiveauDto,
    SortieParamDto, SortieRegleRegimeDto, UpsertAutorisationSortieBody, UpsertSortieParamBody,
    UpsertSortieRegleBody, UpsertSortieRegleRegimeBody,
    AffectationInternatDto, AffecterInternatBody, ChambreInternatDto, CloreAffectationInternatBody,
    CreerChambreInternatBody, CreerSuiviInternatBody, PointerPresenceInternatBody,
    PresenceInternatDto, RecapInternatDto, SuiviInternatDto,
    DispenseVsDto, JournalEntreeDto, PoserDispenseVsBody,
    AlerteCpeDto, CloreAlerteCpeBody, CreerAlerteCpeBody,
    AjouterDossierConseilBody, ConseilClasseDossierDto, ConseilClasseDto, CreerConseilClasseBody,
    DocASignerDto, PublierDocASignerBody,
    CreerInfoCompChampBody, InfoCompChampDto, InfoCompEleveDto, InfoCompValeurDto,
    UpsertInfoCompValeurBody,
    CreerPeriodeVacancesBody, HomonymeCandidatDto, JourFermeDto, MenuCantineDto,
    PeriodeVacancesDto, RechercherHomonymesBody, UpsertMenuCantineBody,
    AbsenceMotifDto, CalculTtcDto, CreerAbsenceMotifBody, CreerTauxTvaBody, DeciderGrantBody,
    DemanderGrantBody, DeposerDocumentBody, DocumentGrantDto, DocumentMetaDto, QuotientFoyerDto,
    TauxTvaDto, TrombiEntreeDto, UpsertQuotientBody, CreerSalleBody, ParamEtabDto, SalleDto,
    UpsertParamEtabBody,
    CreerFiliereBody, FiliereDto, MailOutboxDto, PoserVoeuBody, VoeuDto,
    IbanHistoDto, InscriptionSuiviDto, UpsertInscriptionSuiviBody, SmsOutboxDto,
    CreerTarifConsoBody, TarifConsoDto, AbsenceRhDto, DeclarerAbsenceRhBody,
    AffecterEdtCreneauBody, CreerEditionVsBody, EditionVsDto, EditionVsRenduDto,
    RendreEditionVsBody, ConventionStageDto, CreerConventionStageBody, CreerEntrepriseBody,
    EntrepriseDto, CreerEdtContrainteBody, EdtContrainteDto,
    CreateFoyerBody, EleveLienDto, FoyerDetailDto, FoyerLienDto, FoyerListItemDto,
    RattacherResponsableFoyerBody, SetEmailBody, SetTelephoneBody, UpdateLienBody, UpdateLiensEleveBody,
    UpdateIdentiteBody,
};
use crate::client_ip::ClientIp;
use crate::ui;
use crate::AppState;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use chrono::{NaiveDate, NaiveTime, Utc};
use domain_eleve::{
    annee_scolaire_de, annees_continuum, render_facture_pdf, render_quittance_pdf, reste_a_payer,
    today_utc, CarnetCategorie, ChangeClasse, ChangeMef, ChangeRegime, ChangeSite, ChoisirOptionN1,
    ConfigurerAcompteCampagne, ConfirmerReinscription, CreateEleve, CreerAnnonce, CreerCarnetEntree,
    CreerDevoir,
    CreerFacture, CreerMandatSepa, CreerNotePeriode, DeclareAbsence, DeclarerRetard, DocTiroir, EcheanceCmd,
    SaisirAppreciation,
    EncaisserFacture, ExporterOgec, LigneFactureCmd, MonteeLot, MonteeRapide, MoyenPaiement,
    OptionNature, OuvrirCampagneReinscription, OuvrirFil, PlanifierPrelevement,
    ArchiverFil, AttacherPiece, ClasserFil, ConfigurerEdtCalendrierAb,
    CreerCahierTexte, CreerEntretienPsy, CreerMsgModele, CreerPassageInfirmerie,
    DeposerDocument, DispenseSource, DocConfidentialite, EdtExceptionKind,
    EntretienPsyKind, ExporterVolumesRepas, InviterCompteEleve,
    AffecterBourse, ChangerStatutRdvPsy, CreerBourseType, CreerRdvPsy, CreerRepasActivite,
    DeciderBourse, JoursMasque, MsgPriorite, PassageInfirmerieMotif, PoserDispenseRepas,
    PoserGrilleRepas,
    AnnulerSanction, AttacherOrdonnancePai, ConvoquerCommission, CreerCommissionVs,
    CreerReferentielVs, DeciderCommissionDossier, DeciderGrant, DeclarerIncident, DecisionCommission,
    DeciderDemandeSortie,
    DemanderSortieExceptionnelle, PoserEcheancier, PrononcerSanction,
    PoserEdtException, RdvPsyStatut, SignerPaiFamille, SortieCreneau, UpsertAutorisationSortie,
    UpsertSortieParamEtab, UpsertSortieRegleNiveau, UpsertSortieRegleRegime, UpsertDossierPsy,
    UpsertFicheSante, UpsertPai, PoserExigenceDoc, RepondreFil, RepasActiviteKind,
    ScolariteAnneeStatut, SortirEleve, POINTS_VS_DEPART,
    AffecterEleveInternat, CreerChambreInternat, CreerSuiviInternat, PointerPresenceInternat,
    PoserDispenseVs, CreerAlerteCpe,
    AjouterDossierConseil, CreerConseilClasse, PublierDocASigner,
    SignerDocASigner, CreerInfoCompChamp, UpsertInfoCompValeur,
    CreerPeriodeVacances, RechercherHomonymes, UpsertMenuCantine,
    CreerTauxTva, UpsertQuotientFoyer, CreerSalle, UpsertParamEtab,
    CreerFiliere, PoserVoeu, UpsertInscriptionSuivi, ExporterLsl, ExporterOnde,
    CreerTarifConso, ExporterNotanet, DeclarerAbsenceRh, AffecterEdtCreneau,
    CreerEditionVs, ExporterLsun, ExporterParcoursup, CreerConventionStage, CreerEntreprise,
    CreerEdtContrainte, EdtContrainteKind, UpdateIdentite,
    CreerCompetenceItem, EvaluerCompetence,
};
use base64::Engine;
use ent_db::{Appel, AppelLigne, CreateMatiere, LigneEtat};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    // D2 : plafonds de corps par usage (base64 inclus) alignés sur les tailles maximales
    // configurables (`ENT_UPLOAD_MAX_*`) ; la taille réelle est recontrôlée par le service.
    let up = ent_db::upload::PolitiqueFichiers::depuis_env();
    let lim = |u| DefaultBodyLimit::max(up.limite_corps_base64(u));
    let r = Router::new()
        .route("/", get(ui_page))
        .route("/login", get(ui_page))
        .route("/mot-de-passe-oublie", get(ui_page))
        .route("/reset-mot-de-passe", get(ui_page))
        .route("/app", get(ui_page))
        .route("/app/{*rest}", get(ui_page))
        .route("/famille", get(ui_page))
        .route("/famille/{*rest}", get(ui_page))
        .route("/eleve", get(ui_page))
        .route("/eleve/{*rest}", get(ui_page))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(crate::obs::metrics_handler))
        .route("/pra/inventaire", get(pra_inventaire))
        .route("/pra/inventaire/verifier", post(pra_verifier_inventaire))
        .route("/auth/login", post(login))
        .route("/auth/etablissement", get(etablissement_de_l_hote))
        .route("/etablissement/fiche", put(put_etablissement_fiche))
        .route("/etablissement/domaines", get(list_etablissement_domaines).post(post_etablissement_domaine))
        .route("/etablissement/domaines/{domaine}", delete(delete_etablissement_domaine))
        .route("/auth/passkey/register/start", post(passkey_register_start))
        .route(
            "/auth/passkey/register/finish",
            post(passkey_register_finish),
        )
        .route("/auth/passkey/login/start", post(passkey_login_start))
        .route("/auth/passkey/login/finish", post(passkey_login_finish))
        .route("/auth/passkeys", get(list_passkeys))
        .route("/auth/passkeys/{id}", delete(delete_passkey))
        .route("/auth/users/{id}/passkeys", delete(reset_passkeys_admin))
        .route("/auth/oidc/callback", get(ui_page))
        .route("/auth/oidc/public", get(oidc_public_providers))
        .route("/auth/oidc/providers", get(list_oidc_providers).put(upsert_oidc_provider))
        .route(
            "/auth/oidc/providers/{slug}",
            delete(delete_oidc_provider),
        )
        .route("/auth/oidc/{slug}/start", post(oidc_start))
        .route("/auth/oidc/complete", post(oidc_complete))
        .route("/auth/logout", post(logout))
        .route("/auth/password", post(change_password))
        .route("/auth/forgot-password", post(forgot_password))
        .route("/auth/reset-password", post(reset_password))
        .route("/auth/email/confirmer", post(email_confirmer))
        .route("/auth/email/annuler", post(email_annuler))
        .route("/auth/passkey/recuperation", post(recuperation_passkey_demander))
        .route(
            "/auth/passkey/recuperation/confirmer",
            post(recuperation_passkey_confirmer),
        )
        .route(
            "/auth/passkey/recuperation/annuler",
            post(recuperation_passkey_annuler),
        )
        .route(
            "/auth/passkey/recuperation/finaliser",
            post(recuperation_passkey_finaliser),
        )
        .route("/auth/me", get(me))
        .route("/auth/sessions", get(list_sessions_appareils))
        .route("/auth/sessions/{id}", delete(revoquer_session_appareil))
        .route(
            "/etablissement/securite",
            get(get_politique_securite).put(put_politique_securite),
        )
        .route("/audit/journal", get(list_audit_journal))
        .route("/copilote/ask", post(copilote_ask))
        .route("/copilote/capacites", get(copilote_capacites))
        .route("/devices/push", post(register_push_device))
        .route("/eleves", post(create_eleve))
        .route("/eleves/search", get(search_eleves))
        .route("/eleves/labels", get(eleve_labels_batch))
        .route("/personnel/search", get(search_personnel))
        .route("/personnel/{id}", get(get_personnel))
        .route("/etablissement/personnel", post(provisionner_personnel))
        .route(
            "/etablissement/personnel/{id}/reset-mot-de-passe",
            post(reinitialiser_mot_de_passe),
        )
        .route(
            "/etablissement/personnel/{id}/acces/fermer",
            post(acces_fermer),
        )
        .route(
            "/etablissement/personnel/{id}/acces/suspendre",
            post(acces_suspendre),
        )
        .route(
            "/etablissement/personnel/{id}/acces/rouvrir",
            post(acces_rouvrir),
        )
        .route(
            "/etablissement/personnel/{id}/acces/fin",
            put(acces_programmer_fin),
        )
        .route("/eleves/homonymes", post(rechercher_homonymes))
        .route("/eleves/sans-compte", get(list_eleves_sans_compte))
        .route(
            "/eleves/comptes/provisionner-lot",
            post(provisionner_comptes_lot),
        )
        .route("/eleves/comptes/invitations", get(list_invitations_compte))
        .route("/eleves/{id}/compte/inviter", post(inviter_compte_eleve))
        .route("/eleves/{id}", get(get_eleve))
        .route(
            "/eleves/{id}/identite",
            axum::routing::put(update_identite_eleve),
        )
        .route(
            "/eleves/{id}/identite/corrections",
            post(corriger_identite_eleve),
        )
        .route(
            "/eleves/{id}/identite/changements",
            post(changer_etat_civil_eleve),
        )
        .route("/eleves/{id}/identite/usages", post(definir_usage_eleve))
        .route(
            "/eleves/{id}/identite/historique",
            get(historique_identite_eleve),
        )
        // Socle 2, lot E3 : identifiants externes (INE…) et badge.
        .route(
            "/eleves/{id}/identifiants",
            get(lister_identifiants_eleve).post(ajouter_identifiant_eleve),
        )
        .route(
            "/eleves/{id}/identifiants/{identifiant_id}/confirmer",
            post(confirmer_identifiant_eleve),
        )
        .route(
            "/eleves/{id}/identifiants/{identifiant_id}/contester",
            post(contester_identifiant_eleve),
        )
        .route(
            "/eleves/{id}/identifiants/{identifiant_id}/revoquer",
            post(revoquer_identifiant_eleve),
        )
        .route("/eleves/{id}/badge/perdu", post(badge_perdu_eleve))
        .route(
            "/eleves/identifiants/contestes",
            get(lister_identifiants_contestes),
        )
        // Socle 2, lot E4 : scolarité (scolarisations, inscriptions, changements datés).
        .route("/eleves/{id}/scolarite", get(scolarite_eleve))
        .route("/eleves/{id}/scolarisations", post(ouvrir_scolarisation_eleve))
        .route("/eleves/{id}/scolarisations/reintegrer", post(reintegrer_eleve_route))
        .route("/eleves/{id}/scolarisations/cloturer", post(cloturer_scolarisation_eleve))
        .route("/eleves/{id}/deces", post(deces_eleve))
        .route(
            "/eleves/{id}/inscriptions/{annee_id}",
            axum::routing::put(inscription_annuelle_eleve),
        )
        .route("/eleves/{id}/scolarite/classe", post(changer_classe_eleve))
        .route("/eleves/{id}/scolarite/options", post(changer_options_eleve))
        .route("/eleves/{id}/scolarite/regime", post(changer_regime_eleve))
        .route("/eleves/{id}/scolarite/site", post(changer_site_eleve))
        .route("/eleves/{id}/capacites", post(declarer_capacite_eleve))
        .route("/eleves/{id}/capacites/{capacite_id}/lever", post(lever_capacite_eleve))
        .route("/eleves/{id}/mobilites", post(ouvrir_mobilite_eleve))
        .route("/eleves/{id}/mobilites/{mobilite_id}/fermer", post(fermer_mobilite_eleve))
        .route("/eleves/{id}/modalites", post(enregistrer_modalite_eleve))
        .route("/eleves/{id}/modalites/{modalite_id}/terminer", post(terminer_modalite_eleve))
        .route("/eleves/{id}/dossier", get(get_dossier))
        .route(
            "/eleves/{id}/documents",
            get(list_documents_meta_eleve)
                .post(deposer_document_eleve)
                .layer(lim(ent_db::upload::UsageFichier::Document)),
        )
        .route("/documents/{id}/fichier", get(telecharger_document_fichier))
        .route("/documents/{id}/lien", post(lien_document_fichier))
        // Mode local des liens courts : route PUBLIQUE, le jeton signé fait foi.
        .route("/fichiers/{jeton}", get(servir_fichier_signe))
        .route("/documents/{id}/grants", post(demander_grant_document))
        .route("/documents/grants/pending", get(list_grants_pending))
        .route("/documents/grants/{id}/decider", post(decider_grant_document))
        .route(
            "/eleves/{id}/repas/dispenses",
            get(list_dispenses_repas).post(poser_dispense_repas),
        )
        .route(
            "/eleves/{id}/repas/dispenses/{dispense_id}/annuler",
            post(annuler_dispense_repas),
        )
        .route(
            "/eleves/{id}/repas/grilles",
            get(list_grilles_repas).post(poser_grille_repas),
        )
        .route("/eleves/{id}/repas/volume", get(volume_repas_facturable))
        .route("/eleves/{id}/repas/passages", get(list_passages_repas_eleve))
        .route(
            "/repas/activites",
            get(list_repas_activites).post(creer_repas_activite),
        )
        .route(
            "/bourses/types",
            get(list_bourse_types).post(creer_bourse_type),
        )
        .route("/eleves/{id}/bourses", post(affecter_bourse))
        .route("/bourses/{id}/decider", post(decider_bourse))
        .route("/eleves/{id}/classe", post(change_classe))
        .route("/eleves/{id}/regime", post(change_regime))
        .route("/eleves/{id}/mef", post(change_mef))
        .route("/eleves/{id}/site", post(change_site))
        .route("/eleves/{id}/sortie", post(sortir))
        .route("/eleves/{id}/periodes", get(list_periodes))
        .route(
            "/eleves/{id}/absences",
            get(list_absences).post(declare_absence),
        )
        .route("/eleves/{id}/absent", get(eleve_absent))
        .route("/absences/{id}/justifier", post(justifier_absence))
        .route(
            "/absences/{id}/annuler-justificatif",
            post(annuler_soumission_justificatif),
        )
        .route(
            "/absences/{id}/accepter-justificatif",
            post(accepter_justificatif),
        )
        .route(
            "/absences/{id}/refuser-justificatif",
            post(refuser_justificatif),
        )
        .route(
            "/absences/{id}/annuler-present",
            post(annuler_absence_present),
        )
        .route("/vs/absences", get(list_absences_a_traiter))
        .route("/vs/signaux", get(list_signaux_vs))
        .route("/vs/signaux/{id}/prendre", post(prendre_signal_vs))
        .route("/vs/signaux/{id}/clore", post(clore_signal_vs))
        .route(
            "/vs/signaux/{id}/commission",
            post(convoquer_commission_depuis_signal),
        )
        .route(
            "/vs/commissions",
            get(list_commissions).post(creer_commission),
        )
        .route(
            "/vs/commissions/{id}/dossiers",
            get(list_dossiers_commission).post(convoquer_commission),
        )
        .route(
            "/vs/commissions/dossiers/{id}/decider",
            post(decider_commission_dossier),
        )
        .route(
            "/eleves/{id}/commissions",
            get(list_commissions_eleve),
        )
        .route("/eleves/{id}/points-vs", get(solde_points_vs))
        .route(
            "/vs/incident-types",
            get(list_incident_types).post(creer_incident_type),
        )
        .route(
            "/vs/sanction-types",
            get(list_sanction_types).post(creer_sanction_type),
        )
        .route(
            "/vs/sanction-motifs",
            get(list_sanction_motifs).post(creer_sanction_motif),
        )
        .route(
            "/eleves/{id}/incidents",
            get(list_incidents_eleve).post(declarer_incident),
        )
        .route(
            "/eleves/{id}/sanctions",
            get(list_sanctions_eleve).post(prononcer_sanction),
        )
        .route("/vs/sanctions/{id}/annuler", post(annuler_sanction))
        .route(
            "/eleves/{id}/sorties",
            get(get_recap_sortie).put(upsert_autorisation_sortie),
        )
        .route(
            "/eleves/{id}/sorties/demandes",
            post(demander_sortie_exceptionnelle),
        )
        .route(
            "/vs/sorties/demandes",
            get(list_demandes_sortie_en_attente),
        )
        .route(
            "/vs/sorties/demandes/{id}/decider",
            post(decider_demande_sortie),
        )
        .route("/vs/sorties/controle", post(controle_sortie_badge))
        .route("/vs/sorties/controles", get(list_controles_sortie))
        .route(
            "/vs/sorties/regles",
            get(list_sortie_regles).post(upsert_sortie_regle),
        )
        .route(
            "/vs/sorties/regles-regime",
            get(list_sortie_regles_regime).post(upsert_sortie_regle_regime),
        )
        .route(
            "/vs/sorties/param",
            get(get_sortie_param).put(upsert_sortie_param),
        )
        .route(
            "/vs/internat/chambres",
            get(list_chambres_internat).post(creer_chambre_internat),
        )
        .route(
            "/vs/internat/affectations",
            get(list_affectations_internat).post(affecter_eleve_internat),
        )
        .route(
            "/vs/internat/affectations/{id}/clore",
            post(clore_affectation_internat),
        )
        .route(
            "/vs/internat/presences",
            get(list_presences_internat).post(pointer_presence_internat),
        )
        .route(
            "/vs/internat/chambres/{id}",
            axum::routing::patch(modifier_chambre_internat),
        )
        .route(
            "/vs/internat/presences/historique",
            get(historique_presence_internat),
        )
        .route(
            "/eleves/{id}/internat",
            get(get_recap_internat),
        )
        .route(
            "/eleves/{id}/internat/suivis",
            post(creer_suivi_internat),
        )
        .route(
            "/eleves/{id}/dispenses-vs",
            get(list_dispenses_vs).post(poser_dispense_vs),
        )
        .route(
            "/vs/dispenses/{id}/clore",
            post(clore_dispense_vs),
        )
        .route("/vs/dispenses", get(list_dispenses_vs_jour))
        .route("/eleves/{id}/journal", get(list_journal_eleve))
        .route(
            "/eleves/{id}/alertes-cpe",
            get(list_alertes_cpe_eleve).post(creer_alerte_cpe),
        )
        .route("/vs/alertes-cpe", get(list_alertes_cpe_ouvertes))
        .route(
            "/vs/alertes-cpe/{id}/clore",
            post(clore_alerte_cpe),
        )
        .route(
            "/eleves/{id}/docs-a-signer",
            get(list_docs_a_signer_eleve).post(publier_doc_a_signer),
        )
        .route("/docs-a-signer/{id}/signer", post(signer_doc_a_signer))
        .route("/docs-a-signer/{id}/annuler", post(annuler_doc_a_signer))
        .route("/vs/docs-a-signer", get(list_docs_a_signer_etab))
        .route("/annonces", get(list_annonces).post(publier_annonce))
        .route("/annonces/{id}/confirmer", post(confirmer_annonce))
        .route("/annonces/{id}/retirer", post(retirer_annonce))
        .route("/notifications", get(mes_notifications))
        .route("/eleves/{id}/annonces", get(list_annonces_eleve))
        .route(
            "/eleves/{id}/retards",
            get(list_retards_eleve).post(declarer_retard),
        )
        .route(
            "/competences",
            get(list_competences_items).post(creer_competence_item),
        )
        .route(
            "/eleves/{id}/competences",
            get(list_competences_eleve).post(evaluer_competence),
        )
        .route(
            "/eleves/{id}/livret-competences",
            post(publier_livret_competences),
        )
        .route(
            "/conseils-classe",
            get(list_conseils_classe).post(creer_conseil_classe),
        )
        .route("/conseils-classe/{id}/tenir", post(tenir_conseil_classe))
        .route(
            "/conseils-classe/{id}/dossiers",
            get(list_dossiers_conseil).post(ajouter_dossier_conseil),
        )
        .route(
            "/conseils-classe/dossiers/{id}/decider",
            post(decider_dossier_conseil),
        )
        .route("/eleves/{id}/conseils", get(list_conseils_eleve))
        .route(
            "/info-comp/champs",
            get(list_info_comp_champs).post(creer_info_comp_champ),
        )
        .route(
            "/eleves/{id}/info-comp",
            get(list_info_comp_eleve).put(upsert_info_comp_valeur),
        )
        .route(
            "/vacances",
            get(list_periodes_vacances).post(creer_periode_vacances),
        )
        .route("/vacances/jour-ferme", get(jour_ferme))
        .route("/vacances/{id}/supprimer", post(supprimer_periode_vacances))
        .route(
            "/menus-cantine",
            get(list_menus_cantine).post(upsert_menu_cantine),
        )
        .route(
            "/tarifs-conso",
            get(list_tarifs_conso).post(creer_tarif_conso),
        )
        .route("/rh/absences", get(list_absences_rh).post(declarer_absence_rh))
        .route("/rh/absences/{id}/valider", post(valider_absence_rh))
        .route("/foyers", get(list_foyers).post(create_foyer))
        .route("/foyers/{id}", get(get_foyer_detail))
        .route(
            "/foyers/{id}/quotient",
            get(get_quotient_foyer).put(upsert_quotient_foyer),
        )
        .route("/foyers/{id}/liens", post(rattacher_responsable_foyer))
        .route("/users/{id}/telephone", put(set_user_telephone))
        .route("/users/{id}/email", put(set_user_email))
        .route(
            "/eleves/{id}/liens/{lien_id}",
            put(update_lien_eleve).delete(delete_lien_eleve),
        )
        .route(
            "/eleves/{id}/liens",
            put(update_liens_eleve_batch),
        )
        .route("/taux-tva", get(list_taux_tva).post(creer_taux_tva))
        .route("/taux-tva/calcul-ttc", get(calcul_ttc))
        .route("/classes/{classe}/trombi", get(list_trombi_classe))
        .route(
            "/eleves/{id}/photo",
            get(get_photo_eleve)
                .post(poser_photo_eleve)
                .layer(lim(ent_db::upload::UsageFichier::Photo)),
        )
        .route("/eleves/{id}/photo/lien", post(lien_photo_eleve))
        .route(
            "/etab/params",
            get(list_params_etab).post(upsert_param_etab),
        )
        .route("/etab/params/{cle}", get(get_param_etab))
        .route("/salles", get(list_salles).post(creer_salle))
        .route(
            "/motifs-absence",
            get(list_motifs_absence).post(creer_motif_absence),
        )
        .route("/filieres", get(list_filieres).post(creer_filiere))
        .route(
            "/eleves/{id}/voeux",
            get(list_voeux_eleve).post(poser_voeu),
        )
        .route("/eleves/{id}/voeux/{voeu_id}", delete(retirer_voeu))
        .route("/mails", get(list_mails))
        .route("/sms", get(list_sms))
        .route(
            "/eleves/{id}/inscription-suivi",
            get(get_inscription_suivi).put(upsert_inscription_suivi),
        )
        .route("/inscriptions/par-etape", get(list_inscriptions_par_etape))
        .route("/foyers/{id}/iban-histo", get(list_iban_histo))
        .route("/exports/lsl", get(export_lsl))
        .route("/exports/lsun", get(export_lsun))
        .route("/exports/onde", get(export_onde))
        .route("/exports/notanet", get(export_notanet))
        .route("/exports/parcoursup", get(export_parcoursup))
        .route("/editions-vs", get(list_editions_vs).post(creer_edition_vs))
        .route(
            "/editions-vs/par-code/{code}/rendre",
            post(rendre_edition_vs_par_code),
        )
        .route("/editions-vs/{id}/rendre", post(rendre_edition_vs))
        .route(
            "/entreprises",
            get(list_entreprises).post(creer_entreprise),
        )
        .route("/conventions-stage", post(creer_convention_stage))
        .route(
            "/eleves/{id}/conventions-stage",
            get(list_conventions_eleve),
        )
        .route("/eleves/{id}/notes", get(list_notes).post(saisir_note))
        .route("/eleves/{id}/bulletins", post(publier_bulletin))
        .route(
            "/eleves/{id}/appreciations",
            get(list_appreciations).post(saisir_appreciation),
        )
        .route("/eleves/{id}/moyennes", get(list_moyennes))
        .route("/eleves/{id}/devoirs", get(list_devoirs_eleve))
        .route("/note-periodes", get(list_note_periodes).post(creer_note_periode))
        .route("/note-periodes/{id}/statut", post(set_note_periode_statut))
        .route("/devoirs", get(list_devoirs).post(creer_devoir))
        .route("/devoirs/{id}", delete(supprimer_devoir))
        .route("/classes/{classe}/effectif", get(effectif_classe))
        .route("/matieres", get(list_matieres).post(create_matiere))
        .route("/matieres/{id}", patch(update_matiere))
        .route("/appels", post(ouvrir_appel))
        .route("/appels/{id}", get(get_appel))
        .route("/appels/{id}/lignes", post(marquer_appel_ligne))
        .route("/appels/{id}/clore", post(clore_appel))
        .route("/edt/creneaux", get(list_edt_creneaux))
        .route(
            "/edt/creneaux/{id}/affectation",
            post(affecter_edt_creneau),
        )
        .route("/eleves/{id}/edt", get(list_edt_eleve))
        .route("/eleves/{id}/edt.ics", get(export_edt_eleve_ics))
        .route("/edt/mes-creneaux", get(list_mes_creneaux))
        .route("/edt/mes-creneaux.ics", get(export_mes_creneaux_ics))
        .route(
            "/edt/calendrier-ab",
            get(get_edt_calendrier_ab).put(configurer_edt_calendrier_ab),
        )
        .route(
            "/edt/exceptions",
            get(list_edt_exceptions).post(poser_edt_exception),
        )
        .route("/edt/import", post(importer_edt_csv))
        .route("/edt/resoudre", post(resoudre_edt))
        .route(
            "/edt/contraintes",
            get(list_edt_contraintes).post(creer_edt_contrainte),
        )
        .route(
            "/cahier-texte",
            get(list_cahier_texte).post(creer_cahier_texte),
        )
        .route(
            "/eleves/{id}/carnet",
            get(list_carnet).post(creer_carnet),
        )
        .route("/carnet/{id}/signer", post(signer_carnet))
        .route("/famille/enfants", get(list_mes_enfants))
        .route("/famille/enfants/{id}/resume", get(famille_resume_enfant))
        .route(
            "/infirmerie/passages",
            get(list_passages_infirmerie).post(creer_passage_infirmerie),
        )
        .route(
            "/eleves/{id}/fiche-sante",
            get(get_fiche_sante).put(upsert_fiche_sante),
        )
        .route(
            "/eleves/{id}/fiche-sante.pdf",
            get(pdf_resume_fiche_sante),
        )
        .route(
            "/eleves/{id}/pai",
            get(get_pai).put(upsert_pai),
        )
        .route("/eleves/{id}/pai/signer", post(signer_pai_famille))
        .route(
            "/eleves/{id}/pai/pieces",
            get(list_pai_pieces)
                .post(attacher_ordonnance_pai)
                .layer(lim(ent_db::upload::UsageFichier::PiecePai)),
        )
        .route("/pai/pieces/{id}", get(telecharger_pai_piece))
        .route("/pai/pieces/{id}/lien", post(lien_pai_piece))
        .route(
            "/psy/entretiens",
            get(list_entretiens_psy).post(creer_entretien_psy),
        )
        .route("/psy/rdvs", get(list_rdvs_psy).post(creer_rdv_psy))
        .route("/psy/rdvs/{id}/statut", post(changer_statut_rdv_psy))
        .route(
            "/eleves/{id}/dossier-psy",
            get(get_dossier_psy).put(upsert_dossier_psy),
        )
        .route(
            "/eleves/{id}/dossier-psy/courrier.pdf",
            get(pdf_courrier_orientation_psy),
        )
        .route(
            "/eleves/{id}/dossier-psy/courrier/envoyer",
            post(envoyer_courrier_orientation_psy),
        )
        .route("/messages/fils", get(list_fils).post(ouvrir_fil))
        .route("/messages/fils/depuis-modele", post(ouvrir_fil_depuis_modele))
        .route("/messages/fils/{id}", get(get_fil))
        .route("/messages/fils/{id}/repondre", post(repondre_fil))
        .route("/messages/fils/{id}/archiver", post(archiver_fil))
        .route("/messages/fils/{id}/desarchiver", post(desarchiver_fil))
        .route("/messages/non-traites", get(list_fils_non_traites))
        .route("/messages/{id}/annuler", post(annuler_message_differe))
        .route(
            "/messagerie/reglage",
            get(get_reglage_plage_calme).put(put_reglage_plage_calme),
        )
        .route("/messagerie/liens-a-verifier", get(list_liens_a_verifier))
        .route("/messagerie/liens/{id}/verification", post(verifier_lien_messagerie))
        .route("/messages/fils/{id}/classer", post(classer_fil))
        .route("/messages/modeles", get(list_msg_modeles).post(creer_msg_modele))
        .route(
            "/messages/{id}/pieces",
            post(attacher_piece).layer(lim(ent_db::upload::UsageFichier::PieceMessage)),
        )
        .route("/messages/pieces/{id}", get(telecharger_piece))
        .route("/messages/pieces/{id}/lien", post(lien_piece_message))
        .route(
            "/reinscription/campagne",
            get(get_campagne).post(ouvrir_campagne),
        )
        .route(
            "/reinscription/campagne/acompte",
            post(configurer_acompte_campagne),
        )
        .route("/reinscription/montee", post(montee_rapide))
        .route("/reinscription/montee/lot", post(montee_lot))
        .route("/reinscription/montee/candidats", get(candidats_montee_lot))
        .route(
            "/reinscription/exigences",
            get(list_exigences_doc).post(poser_exigence_doc),
        )
        .route("/reinscription/famille", get(famille_reinscription))
        .route(
            "/reinscription/famille/{id}/options",
            post(famille_retenir_option),
        )
        .route(
            "/reinscription/famille/{id}/documents",
            post(famille_deposer_doc_reinscription)
                .layer(lim(ent_db::upload::UsageFichier::Document)),
        )
        .route(
            "/reinscription/famille/{id}/confirmer",
            post(famille_confirmer_reinscription),
        )
        .route(
            "/facturation/factures",
            get(list_factures).post(creer_facture),
        )
        .route("/facturation/factures/{id}", get(get_facture))
        .route("/facturation/factures/{id}/pdf", get(pdf_facture))
        .route("/facturation/factures/{id}/emettre", post(emettre_facture))
        .route(
            "/facturation/factures/{id}/encaisser",
            post(encaisser_facture),
        )
        .route(
            "/facturation/factures/{id}/echeancier",
            post(poser_echeancier),
        )
        .route(
            "/facturation/factures/{id}/echeancier/parts",
            post(poser_echeancier_parts),
        )
        .route("/facturation/famille", get(list_factures_famille))
        .route("/facturation/famille/quittances", get(list_quittances_famille))
        .route("/facturation/quittances/{id}", get(get_quittance))
        .route("/facturation/quittances/{id}/pdf", get(pdf_quittance))
        .route("/facturation/export/ogec", get(export_ogec))
        .route("/repas/volumes", get(list_volumes_repas))
        .route("/repas/volumes/export", get(export_volumes_cuisine))
        .route("/repas/passages", get(list_passages_repas))
        .route("/repas/passages/scan", post(scan_badge_repas))
        .route(
            "/facturation/cantine/badge",
            get(apercu_cantine_badge).post(creer_facture_cantine_badge),
        )
        .route(
            "/facturation/sepa/mandats",
            get(list_sepa_mandats).post(creer_sepa_mandat),
        )
        .route(
            "/facturation/sepa/mandats/{id}/revoquer",
            post(revoquer_sepa_mandat),
        )
        .route(
            "/facturation/factures/{id}/sepa/prelever",
            post(planifier_sepa_prelevement),
        )
        // F0 (FD-03) : POST — l'export change l'état des prélèvements ; jamais en GET.
        .route("/facturation/sepa/export", post(export_sepa))
        .route(
            "/facturation/sepa/prelevements/{id}/executer",
            post(executer_sepa_prelevement),
        )
        .route(
            "/facturation/sepa/prelevements/{id}/rejeter",
            post(rejeter_sepa_prelevement),
        )
        .route(
            "/facturation/banque/rapprocher",
            post(rapprocher_releve_banque),
        )
        .route(
            "/facturation/rappels",
            get(list_rappels).post(lancer_rappels),
        );
    // Référentiel scolaire (module séparé : `referentiel.rs`).
    let r = r.merge(crate::referentiel::routes());
    let r = r.merge(crate::referentiel_s2::routes());
    // Socle 5, lot S2 : profil, paramètres, sites, salles, lieux (`structure.rs`).
    let r = r.merge(crate::structure::routes());
    // Socle 5, lot S5 : matières, offre d'enseignements, enseignements suivis (`structure_offre.rs`).
    let r = r.merge(crate::structure_offre::routes());
    // Socle 5, lot S3 : calendrier, semaines A/B, fermetures exceptionnelles (`structure_calendrier.rs`).
    let r = r.merge(crate::structure_calendrier::routes());
    // Socle 5, lot S7 : périodisations et grille horaire versionnée (`structure_periodes.rs`).
    let r = r.merge(crate::structure_periodes::routes());
    // Socle 5, lot S4 : l'objet « cours » et les postes vacants (`structure_cours.rs`).
    let r = r.merge(crate::structure_cours::routes());
    // Socle 42, lot V0a : sorties et voyages scolaires collectifs (`voyages.rs`).
    let r = r.merge(crate::voyages::routes());
    let r = r.merge(crate::internat::routes());
    let r = r.merge(crate::cahier::routes());
    let r = r.merge(crate::roles::routes());
    // Dossier élève et personnes (socle 2, S2-1 : `dossier.rs`).
    let r = r.merge(crate::dossier::routes());
    // Vie scolaire complète (S2-4, module séparé : `vie_scolaire.rs`).
    let r = r.merge(crate::vie_scolaire::routes());
    let r = r.merge(crate::portail::routes());
    // Admissions, préinscriptions, inscriptions, imports (socle 2, S2-3 : `inscriptions.rs`).
    let r = r.merge(crate::inscriptions::routes());
    // EDT, remplacements, délégations (S2-5, module séparé : `edt_s2.rs`).
    let r = r.merge(crate::edt_s2::routes());
    // Socle 2, lot S2-7 : évaluations, conseils, bulletins, livrets.
    let r = r.merge(crate::evaluations::routes());
    // Socle 2, lot E5 : propositions et paramétrage des champs (`eleve_propositions.rs`).
    let r = r.merge(crate::eleve_propositions::routes());
    // Socle 2, lot E6 : doublons et fusion de fiches (`eleve_fusion.rs`).
    let r = r.merge(crate::eleve_fusion::routes());
    // Communication complète (S2-8, module séparé : `communication_s28.rs`).
    let r = r.merge(crate::communication_s28::routes());
    // Finances reliées (socle 2, S2-9 : `finances.rs`).
    let r = r.merge(crate::finances::routes());
    // Santé et accompagnement (S2-11, module séparé : `sante.rs`).
    let r = r.merge(crate::sante::routes());
    // Personnels : agents, engagements, fonctions, annuaire (socle 6, lot P1 : `personnels.rs`).
    let r = r.merge(crate::personnels::routes());
    // Services d'enseignement et interventions (socle 6, lot P3 : `personnels_services.rs`).
    let r = r.merge(crate::personnels_services::routes());
    // Missions et périmètre (socle 6, lot P4 : `personnels_missions.rs`).
    let r = r.merge(crate::personnels_missions::routes());
    #[cfg(feature = "labo")]
    let r = r
        .route("/labo/bootstrap", post(labo_bootstrap))
        .route("/auth/register", post(register));
    r
}

async fn ui_page() -> impl axum::response::IntoResponse {
    // Revue finale socle 1 : CSP à empreinte du script unique de l'ancienne UI.
    let (page, csp) = ui::page_et_csp();
    (
        [(header::CONTENT_SECURITY_POLICY, csp)],
        axum::response::Html(page),
    )
}

#[derive(Debug, Serialize)]
struct PersonnelDto {
    user_id: Uuid,
    email: String,
    role: String,
    label: String,
}

/// Routes de labo (`POST /labo/bootstrap`) : compilées SEULEMENT avec la
/// feature `labo` (L4). Sans elle, ni le code ni `LABO_PASSWORD` ne sont dans le binaire.
#[cfg(feature = "labo")]
mod labo_routes {
use super::*;
use ent_db::LABO_PASSWORD;

#[derive(Debug, Serialize)]
struct LaboPersonaDto {
    label: String,
    email: String,
    role: String,
}

#[derive(Debug, Serialize)]
struct LaboEleveDto {
    id: Uuid,
    nom: String,
    prenom: String,
    classe: String,
    foyer_id: Uuid,
}

#[derive(Debug, Serialize)]
struct LaboPersonnelDto {
    user_id: Uuid,
    email: String,
    role: String,
    label: String,
}

debug_masque!(LaboBootstrapDto { etablissement_id } masque { password });

#[derive(Serialize)]
pub(super) struct LaboBootstrapDto {
    etablissement_id: Uuid,
    password: String,
    personas: Vec<LaboPersonaDto>,
    eleves: Vec<LaboEleveDto>,
    personnel: Vec<LaboPersonnelDto>,
    foyer_lea_id: Uuid,
    foyer_tom_id: Uuid,
    /// Molly Weasley — parent commun Ron + Ginny (seed audit).
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_weasley: Option<LaboPersonaDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eleve_ron_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eleve_ginny_id: Option<Uuid>,
}

/// Seed une école dense (local only) et renvoie les comptes pour l'UI labo.
pub(super) async fn labo_bootstrap(
    State(state): State<AppState>,
) -> Result<Json<LaboBootstrapDto>, (StatusCode, String)> {
    if !labo_open() {
        return Err((
            StatusCode::FORBIDDEN,
            "labo désactivé sur cet environnement".into(),
        ));
    }
    let cast = state
        .ent
        .seed_labo_cast()
        .await
        .map_err(|e| erreur_interne(&e))?;

    let mut personas = vec![
        LaboPersonaDto {
            label: "Admin - Albus Dumbledore".into(),
            email: cast.admin.email.clone(),
            role: cast.admin.role.as_str().into(),
        },
        LaboPersonaDto {
            label: "Direction - Albus Dumbledore".into(),
            email: cast.direction.email.clone(),
            role: cast.direction.role.as_str().into(),
        },
        LaboPersonaDto {
            label: "Administratif - Argus Rusard".into(),
            email: cast.administratif.email.clone(),
            role: cast.administratif.role.as_str().into(),
        },
        LaboPersonaDto {
            label: "CPE - Remus Lupin".into(),
            email: cast.cpe.email.clone(),
            role: cast.cpe.role.as_str().into(),
        },
        LaboPersonaDto {
            label: "Prof - Severus Rogue".into(),
            email: cast.prof.email.clone(),
            role: cast.prof.role.as_str().into(),
        },
        LaboPersonaDto {
            label: "Compta - Horace Slughorn".into(),
            email: cast.compta.email.clone(),
            role: cast.compta.role.as_str().into(),
        },
        LaboPersonaDto {
            label: "Parent - Lily Potter (Harry)".into(),
            email: cast.parent_lea.email.clone(),
            role: cast.parent_lea.role.as_str().into(),
        },
        LaboPersonaDto {
            label: "Parent - Narcissa Malefoy (Draco)".into(),
            email: cast.parent_tom.email.clone(),
            role: cast.parent_tom.role.as_str().into(),
        },
    ];
    if let Some(ref molly) = cast.parent_weasley {
        personas.push(LaboPersonaDto {
            label: "Parent - Molly Weasley (Ron + Ginny)".into(),
            email: molly.email.clone(),
            role: molly.role.as_str().into(),
        });
    }
    if let Some(inf) = cast.infirmieres.first() {
        personas.push(LaboPersonaDto {
            label: "Infirmerie - Poppy Pomfresh".into(),
            email: inf.email.clone(),
            role: inf.role.as_str().into(),
        });
    }
    if let Some(psy) = cast.psys.first() {
        personas.push(LaboPersonaDto {
            label: "Psy - Sybille Trelawney".into(),
            email: psy.email.clone(),
            role: psy.role.as_str().into(),
        });
    }
    if let Some(c) = cast.eleves[0].compte.as_ref() {
        personas.push(LaboPersonaDto {
            label: "Eleve - Harry Potter".into(),
            email: c.email.clone(),
            role: c.role.as_str().into(),
        });
    }

    let eleves = cast
        .eleves
        .iter()
        .map(|e| LaboEleveDto {
            id: e.id,
            nom: e.nom.clone(),
            prenom: e.prenom.clone(),
            classe: e.classe.clone(),
            foyer_id: e.foyer_id,
        })
        .collect();

    let mut personnel: Vec<LaboPersonnelDto> = Vec::new();
    let mut push_pers = |p: &ent_db::LaboPersona, label: &str| {
        personnel.push(LaboPersonnelDto {
            user_id: p.user_id,
            email: p.email.clone(),
            role: p.role.as_str().into(),
            label: label.to_string(),
        });
    };
    push_pers(&cast.admin, "Admin");
    push_pers(&cast.direction, "Direction");
    push_pers(&cast.administratif, "Administratif");
    push_pers(&cast.compta, "Compta");
    push_pers(&cast.cpe, "CPE");
    for (i, p) in cast.profs.iter().enumerate() {
        push_pers(p, &format!("Prof {}", i + 1));
    }
    for (i, p) in cast.cpes.iter().enumerate().skip(1) {
        push_pers(p, &format!("CPE {}", i + 1));
    }
    for (i, p) in cast.infirmieres.iter().enumerate() {
        push_pers(p, &format!("Infirmerie {}", i + 1));
    }
    for (i, p) in cast.psys.iter().enumerate() {
        push_pers(p, &format!("Psy {}", i + 1));
    }

    let parent_weasley = cast.parent_weasley.as_ref().map(|p| LaboPersonaDto {
        label: "Parent - Molly Weasley (Ron + Ginny)".into(),
        email: p.email.clone(),
        role: p.role.as_str().into(),
    });

    Ok(Json(LaboBootstrapDto {
        etablissement_id: cast.etablissement_id,
        password: LABO_PASSWORD.to_string(),
        personas,
        eleves,
        personnel,
        foyer_lea_id: cast.foyer_lea_id,
        foyer_tom_id: cast.foyer_tom_id,
        parent_weasley,
        eleve_ron_id: cast.eleve_ron_id,
        eleve_ginny_id: cast.eleve_ginny_id,
    }))
}
}
#[cfg(feature = "labo")]
use labo_routes::labo_bootstrap;

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

async fn search_eleves(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<EleveDto>>, (StatusCode, String)> {
    session.require_staff()?;
    let needle = q.q.unwrap_or_default();
    // E1 : groupes de champs ouverts pour CE lecteur (annuaire avec sa portée) ; le
    // badge n'est interrogeable que si les identifiants sont visibles partout.
    let rows = state
        .ent
        .search_eleves_pour_acteur(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &needle,
            50,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(rows.iter().map(EleveDto::depuis_vue).collect()))
}

#[derive(Debug, Deserialize)]
struct EleveLabelsQuery {
    /// Liste d'UUID séparés par des virgules.
    ids: Option<String>,
}

#[derive(Debug, Serialize)]
struct EleveLabelRow {
    id: Uuid,
    label: String,
}

async fn eleve_labels_batch(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<EleveLabelsQuery>,
) -> Result<Json<Vec<EleveLabelRow>>, (StatusCode, String)> {
    session.require_staff()?;
    let ids: Vec<Uuid> = q
        .ids
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| Uuid::parse_str(s.trim()).ok())
        .collect();
    let map = state
        .ent
        .eleve_labels_for_actor(session.etablissement_id, session.user_id, &session.role, &ids)
        .await
        .map_err(map_db)?;
    Ok(Json(
        map.into_iter()
            .map(|(id, label)| EleveLabelRow { id, label })
            .collect(),
    ))
}

fn personnel_dto(p: ent_db::StaffMember) -> PersonnelDto {
    let label = p
        .display_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| p.email.clone());
    PersonnelDto {
        user_id: p.user_id,
        email: p.email,
        role: p.role,
        label,
    }
}

async fn search_personnel(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<PersonnelDto>>, (StatusCode, String)> {
    if !session
        .role_parsed()
        .map(|r| r.is_staff())
        .unwrap_or(false)
    {
        return Err((StatusCode::FORBIDDEN, "réservé au personnel".into()));
    }
    let needle = q.q.unwrap_or_default();
    let rows = state
        .ent
        .search_staff(session.etablissement_id, &needle, 100)
        .await
        .map_err(|e| erreur_interne(&e))?;
    Ok(Json(rows.into_iter().map(personnel_dto).collect()))
}

#[derive(Debug, Deserialize)]
struct ProvisionnerPersonnelBody {
    email: String,
    role: String,
    display_name: Option<String>,
}

#[derive(Serialize)]
struct PersonnelProvisionneDto {
    user_id: Uuid,
    email: String,
    role: String,
    /// Toujours `true` : le compte est créé sans mot de passe utilisable et la personne reçoit un
    /// lien d'invitation à usage unique par e-mail. Aucun secret n'est renvoyé (socle 6 P0).
    invitation_envoyee: bool,
}

/// Provisionne un compte personnel (staff) dans l'établissement de la session.
/// Réservé à Direction / Admin (Admin seul peut créer Admin / Direction).
async fn provisionner_personnel(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<ProvisionnerPersonnelBody>,
) -> Result<(StatusCode, Json<PersonnelProvisionneDto>), (StatusCode, String)> {
    let p = state
        .ent
        .provisionner_personnel_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            ent_db::ProvisionnerPersonnel {
                email: body.email,
                display_name: body.display_name,
                role: body.role,
            },
            &base_url_publique(),
        )
        .await
        .map_err(|e| map_compte_sensible(&e).unwrap_or_else(|| map_auth(e)))?;
    Ok((
        StatusCode::CREATED,
        Json(PersonnelProvisionneDto {
            user_id: p.user_id,
            email: p.email,
            role: p.role,
            invitation_envoyee: p.invitation_envoyee,
        }),
    ))
}

#[derive(Serialize)]
struct MotDePasseReinitialiseDto {
    user_id: Uuid,
    /// `true` : un lien de réinitialisation à usage unique a été envoyé à l'adresse du compte.
    /// Aucun secret n'est jamais renvoyé (socle 6 P0, fin du mot de passe temporaire).
    lien_envoye: bool,
}

/// Erreurs « mot de passe actuel » / envoi / plafond / nom de passkey, mappées
/// avant les heuristiques génériques. `None` : laisser la fonction appelante décider.
pub(crate) fn map_compte_sensible(err: &ent_db::EntDbError) -> Option<(StatusCode, String)> {
    match err {
        ent_db::EntDbError::MotDePasseActuelInvalide => {
            Some((StatusCode::FORBIDDEN, err.to_string()))
        }
        ent_db::EntDbError::EnvoiImpossible => {
            Some((StatusCode::SERVICE_UNAVAILABLE, err.to_string()))
        }
        ent_db::EntDbError::PasskeyPlafond(_) => Some((StatusCode::CONFLICT, err.to_string())),
        ent_db::EntDbError::PasskeyNomInvalide => {
            Some((StatusCode::BAD_REQUEST, err.to_string()))
        }
        ent_db::EntDbError::Auth(ent_auth::AuthError::TooManyAttempts) => {
            Some((StatusCode::TOO_MANY_REQUESTS, err.to_string()))
        }
        _ => None,
    }
}

/// URL publique pour les liens envoyés par e-mail. Hors feature `labo`, son
/// absence renvoie une chaîne vide : le service refuse alors d'émettre un jeton
/// (`EnvoiImpossible`, 503) au lieu de produire un lien vers localhost.
pub(crate) fn base_url_publique() -> String {
    match std::env::var("ENT_PUBLIC_BASE_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ if cfg!(feature = "labo") => "http://localhost:8088".into(),
        _ => String::new(),
    }
}

/// Réinitialise le mot de passe d'un compte (Direction / Admin) : un lien à usage unique est envoyé
/// à l'adresse du compte (202). Aucun mot de passe n'est produit ni renvoyé, quel que soit le profil
/// de la cible ; ses sessions sont révoquées pour un personnel.
async fn reinitialiser_mot_de_passe(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<MotDePasseReinitialiseDto>), (StatusCode, String)> {
    state
        .ent
        .reinitialiser_mot_de_passe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &base_url_publique(),
        )
        .await
        .map_err(|e| map_compte_sensible(&e).unwrap_or_else(|| map_db(e)))?;
    Ok((
        StatusCode::ACCEPTED,
        Json(MotDePasseReinitialiseDto {
            user_id: id,
            lien_envoye: true,
        }),
    ))
}

#[derive(Debug, Deserialize)]
struct MotifAccesBody {
    motif: String,
}

#[derive(Debug, Deserialize)]
struct FinAccesBody {
    /// Jour de fin (AAAA-MM-JJ), l'accès se ferme le soir de ce jour ; `null` retire la date.
    date_fin: Option<chrono::NaiveDate>,
}

#[derive(Serialize)]
struct AccesEtatDto {
    user_id: Uuid,
    etat: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    date_fin: Option<chrono::NaiveDate>,
}

/// Ferme l'accès d'un membre du personnel (départ). Admin / Direction, jamais soi-même ; le
/// dernier administrateur d'un établissement ne peut pas être fermé (409).
async fn acces_fermer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<MotifAccesBody>,
) -> Result<Json<AccesEtatDto>, (StatusCode, String)> {
    let etat = state
        .ent
        .acces_fermer_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &body.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(AccesEtatDto { user_id: id, etat: etat.as_str(), date_fin: None }))
}

/// Suspend l'accès d'un membre du personnel (mesure conservatoire, congé) ; réversible.
async fn acces_suspendre(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<MotifAccesBody>,
) -> Result<Json<AccesEtatDto>, (StatusCode, String)> {
    let etat = state
        .ent
        .acces_suspendre_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &body.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(AccesEtatDto { user_id: id, etat: etat.as_str(), date_fin: None }))
}

/// Rouvre un accès suspendu ou fermé.
async fn acces_rouvrir(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<AccesEtatDto>, (StatusCode, String)> {
    let etat = state
        .ent
        .acces_rouvrir_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(AccesEtatDto { user_id: id, etat: etat.as_str(), date_fin: None }))
}

/// Programme (ou retire) la date de fin d'un accès.
async fn acces_programmer_fin(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<FinAccesBody>,
) -> Result<Json<AccesEtatDto>, (StatusCode, String)> {
    let fin = state
        .ent
        .acces_programmer_fin_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            body.date_fin,
        )
        .await
        .map_err(map_db)?;
    let etat = state
        .ent
        .acces_etat(session.etablissement_id, id)
        .await
        .map_err(map_db)?
        .map_or("ouvert", |a| a.etat.as_str());
    Ok(Json(AccesEtatDto { user_id: id, etat, date_fin: fin }))
}

async fn get_personnel(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<PersonnelDto>, (StatusCode, String)> {
    if !session
        .role_parsed()
        .map(|r| r.is_staff())
        .unwrap_or(false)
    {
        return Err((StatusCode::FORBIDDEN, "réservé au personnel".into()));
    }
    let p = state
        .ent
        .get_staff_member(session.etablissement_id, id)
        .await
        .map_err(|e| erreur_interne(&e))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "adulte introuvable".into()))?;
    Ok(Json(personnel_dto(p)))
}

async fn list_eleves_sans_compte(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<EleveDto>>, (StatusCode, String)> {
    session.require_manage_eleves()?;
    let rows = state
        .ent
        .list_eleves_sans_compte_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(rows.iter().map(EleveDto::from).collect()))
}

async fn inviter_compte_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<InviterCompteBody>,
) -> Result<(StatusCode, Json<CompteProvisionneDto>), (StatusCode, String)> {
    session.require_manage_eleves()?;
    let (inv, email, mdp) = state
        .ent
        .inviter_compte_eleve_for_actor(
            &session.role,
            InviterCompteEleve {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                email: body.email,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(CompteProvisionneDto::from_parts(&inv, &email, &mdp)),
    ))
}

async fn provisionner_comptes_lot(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<(StatusCode, Json<Vec<CompteProvisionneDto>>), (StatusCode, String)> {
    session.require_manage_eleves()?;
    let lot = state
        .ent
        .provisionner_comptes_manquants_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
        )
        .await
        .map_err(map_db)?;
    let dtos = lot
        .iter()
        .map(|(inv, email, mdp)| CompteProvisionneDto::from_parts(inv, email, mdp))
        .collect();
    Ok((StatusCode::CREATED, Json(dtos)))
}

async fn list_invitations_compte(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<InvitationCompteDto>>, (StatusCode, String)> {
    session.require_manage_eleves()?;
    let list = state
        .ent
        .list_invitations_compte_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_invitations(
            &state,
            session.etablissement_id,
            list.iter().map(InvitationCompteDto::from).collect(),
        )
        .await?,
    ))
}

async fn get_dossier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut dossier = state
        .ent
        .get_dossier_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            today_utc(),
        )
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "dossier inaccessible".into()))?;
    let sanctions = enrich_sanctions(
        &state,
        session.etablissement_id,
        dossier.sanctions.iter().map(SanctionDto::from).collect(),
    )
    .await?;
    let incidents = enrich_incidents(
        &state,
        session.etablissement_id,
        dossier.incidents.iter().map(IncidentDto::from).collect(),
    )
    .await?;
    let carnet = enrich_carnet(
        &state,
        session.etablissement_id,
        dossier.carnet.iter().map(CarnetEntreeDto::from).collect(),
    )
    .await?;
    let grille_repas = enrich_grilles_repas(
        &state,
        session.etablissement_id,
        dossier
            .grille_repas
            .iter()
            .map(GrilleRepasDto::from)
            .collect(),
    )
    .await?;
    let dispenses_repas = enrich_dispenses_repas(
        &state,
        session.etablissement_id,
        dossier
            .dispenses_repas
            .iter()
            .map(DispenseRepasDto::from)
            .collect(),
    )
    .await?;
    let bourses = enrich_bourses(
        &state,
        session.etablissement_id,
        dossier.bourses.iter().map(EleveBourseDto::from).collect(),
    )
    .await?;
    // Les clés objet restent internes au serveur : jamais dans les réponses.
    for d in dossier.documents.iter_mut() {
        d.storage_key = None;
    }
    dossier.eleve.photo_key = None;
    // E1 : groupes ouverts pour ce lecteur et cet élève (point unique de minimisation).
    let groupes = state
        .ent
        .profil_champs(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?
        .pour(dossier.eleve.id);
    dossier.minimise_groupes(&groupes);
    let mut value = serde_json::to_value(&dossier)
        .map_err(|e| erreur_interne(&e))?;
    if !groupes.ouvert(domain_eleve::GroupeChamps::Scolarite) {
        // Donnée retirée = `null` (« non renseigné »), jamais un `false` trompeur.
        if let Some(e) = value.get_mut("eleve").and_then(|e| e.as_object_mut()) {
            e.insert("redoublant".into(), serde_json::Value::Null);
        }
        if let Some(cont) = value.get_mut("continuum").and_then(|c| c.as_array_mut()) {
            for a in cont.iter_mut().filter_map(|a| a.as_object_mut()) {
                a.insert("redoublement_possible".into(), serde_json::Value::Null);
            }
        }
    }
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "sanctions".into(),
            serde_json::to_value(sanctions)
                .map_err(|e| erreur_interne(&e))?,
        );
        obj.insert(
            "incidents".into(),
            serde_json::to_value(incidents)
                .map_err(|e| erreur_interne(&e))?,
        );
        obj.insert(
            "carnet".into(),
            serde_json::to_value(carnet)
                .map_err(|e| erreur_interne(&e))?,
        );
        obj.insert(
            "grille_repas".into(),
            serde_json::to_value(grille_repas)
                .map_err(|e| erreur_interne(&e))?,
        );
        obj.insert(
            "dispenses_repas".into(),
            serde_json::to_value(dispenses_repas)
                .map_err(|e| erreur_interne(&e))?,
        );
        obj.insert(
            "bourses".into(),
            serde_json::to_value(bourses)
                .map_err(|e| erreur_interne(&e))?,
        );
    }
    Ok(Json(value))
}

async fn list_documents_meta_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DocumentMetaDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_documents_meta_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    let mut out = Vec::with_capacity(list.len());
    for (d, accessible) in list {
        let grant_etat = if accessible {
            None
        } else {
            state
                .ent
                .latest_grant_etat_for_actor_doc(
                    session.etablissement_id,
                    session.user_id,
                    d.id,
                )
                .await
                .ok()
                .flatten()
        };
        out.push(DocumentMetaDto {
            id: d.id,
            eleve_id: d.eleve_id,
            titre: d.titre,
            tiroir: d.tiroir.as_str().to_string(),
            confidentialite: d.confidentialite.as_str().to_string(),
            source: d.source,
            code: d.code,
            accessible,
            grant_etat,
        });
    }
    Ok(Json(out))
}

async fn deposer_document_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeposerDocumentBody>,
) -> Result<(StatusCode, Json<DocumentMetaDto>), (StatusCode, String)> {
    let tiroir = DocTiroir::parse(&body.tiroir).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "tiroir invalide (scolaire|administratif|sante|famille|autre)".into(),
        )
    })?;
    let confidentialite = DocConfidentialite::parse(&body.confidentialite).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "confidentialite invalide (standard|restreint|sante)".into(),
        )
    })?;
    let contenu = if let Some(b64) = body.contenu_base64.as_ref() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".into()))?;
        Some(bytes)
    } else {
        None
    };
    let doc = state
        .ent
        .deposer_document_for_actor(
            &session.role,
            DeposerDocument {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                tiroir,
                titre: body.titre,
                code: body.code,
                mime_type: body.mime_type,
                storage_key: None,
                confidentialite,
                actor_user_id: session.user_id,
            },
            contenu.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(DocumentMetaDto {
            id: doc.id,
            eleve_id: doc.eleve_id,
            titre: doc.titre,
            tiroir: doc.tiroir.as_str().to_string(),
            confidentialite: doc.confidentialite.as_str().to_string(),
            source: doc.source,
            code: doc.code,
            accessible: true,
            grant_etat: None,
        }),
    ))
}

async fn telecharger_document_fichier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, String)> {
    let (doc, bytes) = state
        .ent
        .telecharger_document_bytes_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "telechargement", "document", &format!("Téléchargement du document {id}")).await;
    let (filename, mime) = ent_db::nom_fichier_document(&doc);
    attachment_response_dyn(bytes, &filename, &mime)
}

/// Réponse des routes `…/lien` : URL courte d'accès au fichier.
#[derive(Debug, Serialize)]
struct LienFichierDto {
    /// URL à ouvrir telle quelle (relative à l'API en mode local, absolue en mode S3).
    url: String,
    /// Expiration, secondes Unix (UTC). Durée courte (60 s par défaut, 15 min maximum).
    expires_at: i64,
}

impl From<ent_db::LienFichier> for LienFichierDto {
    fn from(l: ent_db::LienFichier) -> Self {
        Self {
            url: l.url,
            expires_at: l.expires_at,
        }
    }
}

/// Émet un lien court vers le fichier d'un document. Mêmes droits que
/// `GET /documents/{id}/fichier` ; le client ne fournit jamais de clé de stockage.
async fn lien_document_fichier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<LienFichierDto>, (StatusCode, String)> {
    let lien = state
        .ent
        .lien_document_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(lien.into()))
}

/// Lien court vers la photo d'un élève (mêmes droits que `GET /eleves/{id}/photo`).
async fn lien_photo_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<LienFichierDto>, (StatusCode, String)> {
    let lien = state
        .ent
        .lien_photo_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?
        .ok_or((StatusCode::NOT_FOUND, "pas de photo".into()))?;
    Ok(Json(lien.into()))
}

/// Lien court vers une pièce du PAI (mêmes droits que `GET /pai/pieces/{id}`).
async fn lien_pai_piece(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<LienFichierDto>, (StatusCode, String)> {
    let lien = state
        .ent
        .lien_pai_piece_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(lien.into()))
}

/// Lien court vers une pièce jointe de messagerie (mêmes droits que `GET /messages/pieces/{id}`).
async fn lien_piece_message(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<LienFichierDto>, (StatusCode, String)> {
    let lien = state
        .ent
        .lien_piece_message_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(lien.into()))
}

/// Mode local des liens courts : sert le fichier désigné par un jeton signé.
///
/// Route publique : le jeton (HMAC-SHA256, expiration courte) est la seule preuve.
/// L'objet doit appartenir à l'établissement signé ; type, nom et disposition viennent
/// du jeton (jamais du client) ; les types actifs (html, svg, xml…) sont forcés en
/// téléchargement. Le jeton n'est jamais journalisé (le span ne porte que le gabarit).
async fn servir_fichier_signe(
    State(state): State<AppState>,
    Path(jeton): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    use ent_db::presign::{
        content_disposition, key_belongs_to_etab, now_unix, Disposition, PresignError,
    };
    let claims = state
        .ent
        .presigner()
        .verify_local(&jeton, now_unix())
        .map_err(|e| match e {
            PresignError::Expired => (StatusCode::GONE, "Lien expiré.".to_string()),
            PresignError::Disabled => (StatusCode::NOT_FOUND, "Introuvable.".to_string()),
            _ => (StatusCode::FORBIDDEN, "Lien invalide.".to_string()),
        })?;
    // Défense en profondeur : l'objet doit appartenir à l'établissement signé.
    if !key_belongs_to_etab(&claims.key, claims.etablissement_id) {
        return Err((StatusCode::FORBIDDEN, "Lien invalide.".into()));
    }
    // Lecture contrôlée : empreinte et taille comparées à celles de la base.
    let bytes = state
        .ent
        .lire_objet(claims.etablissement_id, &claims.key)
        .await
        .map_err(map_db)?;
    let mut res = Response::new(Body::from(bytes));
    *res.status_mut() = StatusCode::OK;
    let h = res.headers_mut();
    let ct = HeaderValue::from_str(&claims.mime)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    h.insert(header::CONTENT_TYPE, ct);
    h.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&content_disposition(claims.disposition, &claims.filename))
            .map_err(|_| erreur_interne(&"en-tête de disposition invalide"))?,
    );
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    // Aucun contenu actif : rien ne se charge ni ne s'exécute depuis ce fichier. Le
    // bac à sable est omis uniquement pour un PDF affiché en ligne (le lecteur PDF du
    // navigateur est bloqué par `sandbox`) ; un PDF n'est jamais interprété comme HTML
    // (type signé + nosniff).
    let csp = if claims.disposition == Disposition::Inline && claims.mime == "application/pdf" {
        "default-src 'none'; frame-ancestors 'self'"
    } else {
        "default-src 'none'; sandbox"
    };
    h.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(csp),
    );
    h.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-site"),
    );
    Ok(res)
}

async fn demander_grant_document(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DemanderGrantBody>,
) -> Result<(StatusCode, Json<DocumentGrantDto>), (StatusCode, String)> {
    let g = state
        .ent
        .demander_grant_for_actor(
            &session.role,
            session.etablissement_id,
            id,
            session.user_id,
            body.duration_days,
            body.note,
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(
            enrich_grant(
                &state,
                session.etablissement_id,
                DocumentGrantDto::from(&g),
            )
            .await?,
        ),
    ))
}

async fn list_grants_pending(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<DocumentGrantDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_grants_pending_pour_acteur(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_grants(
            &state,
            session.etablissement_id,
            list.iter().map(DocumentGrantDto::from).collect(),
        )
        .await?,
    ))
}

async fn decider_grant_document(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeciderGrantBody>,
) -> Result<Json<DocumentGrantDto>, (StatusCode, String)> {
    let g = state
        .ent
        .decider_grant_for_actor(
            &session.role,
            DeciderGrant {
                etablissement_id: session.etablissement_id,
                grant_id: id,
                accepter: body.accepter,
                actor_user_id: session.user_id,
                note: body.note,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_grant(
            &state,
            session.etablissement_id,
            DocumentGrantDto::from(&g),
        )
        .await?,
    ))
}

fn appel_dto(appel: &Appel, lignes: &[AppelLigne]) -> AppelDto {
    AppelDto {
        id: appel.id,
        classe: appel.classe.clone(),
        jour: appel.jour,
        statut: appel.statut.as_str().to_string(),
        creneau_id: appel.creneau_id,
        matiere_libelle: appel.matiere_libelle.clone(),
        lignes: lignes
            .iter()
            .map(|l| AppelLigneDto {
                eleve_id: l.eleve_id,
                eleve_label: None,
                etat: l.etat.as_str().to_string(),
            })
            .collect(),
    }
}

async fn enrich_appel(
    state: &AppState,
    etablissement_id: Uuid,
    mut dto: AppelDto,
) -> Result<AppelDto, (StatusCode, String)> {
    let ids: Vec<Uuid> = dto.lignes.iter().map(|l| l.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for l in &mut dto.lignes {
        l.eleve_label = labels.get(&l.eleve_id).cloned();
    }
    Ok(dto)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "service": "ent-api",
        "version": crate::VERSION,
        "git_sha": crate::GIT_SHA,
        "migration_tag": ent_db::MIGRATION_TAG,
    }))
}

/// Readiness : vérifie que la base répond (sinon 503 → le conteneur n'est pas prêt).
async fn ready(State(state): State<AppState>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.ent.ping().await.map_err(|e| {
        tracing::error!(error = %e, "readiness : base indisponible");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "base indisponible".to_string(),
        )
    })?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "service": "ent-api",
        "db": "up",
        "version": crate::VERSION,
        "git_sha": crate::GIT_SHA,
        "migration_tag": ent_db::MIGRATION_TAG,
    })))
}

/// Inventaire d'intégrité de **l'établissement de la session** (Direction /
/// Admin) — référence avant sauvegarde. Aucun compteur ne sort du tenant.
async fn pra_inventaire(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<ent_db::Inventaire>, (StatusCode, String)> {
    session.require_sauvegarde()?;
    let inventaire = state
        .ent
        .inventaire_etablissement_pour_acteur(session.etablissement_id, &session.role)
        .await
        .map_err(map_pra)?;
    Ok(Json(inventaire))
}

/// Vérifie qu'une base (typiquement **restaurée**) recalcule exactement
/// l'inventaire fourni. Refuse un inventaire d'un autre établissement.
async fn pra_verifier_inventaire(
    State(state): State<AppState>,
    session: EntSession,
    Json(attendu): Json<ent_db::Inventaire>,
) -> Result<Json<ent_db::RapportIntegrite>, (StatusCode, String)> {
    session.require_sauvegarde()?;
    let rapport = state
        .ent
        .verifier_inventaire_etablissement_pour_acteur(
            session.etablissement_id,
            &session.role,
            &attendu,
        )
        .await
        .map_err(map_pra)?;
    Ok(Json(rapport))
}

/// Erreurs PRA / inventaire → statut HTTP explicite.
pub(crate) fn map_pra(err: ent_db::EntDbError) -> (StatusCode, String) {
    if let Some(r) = erreur_infra(&err) {
        return r;
    }
    match &err {
        ent_db::EntDbError::Forbidden | ent_db::EntDbError::InventairePorteeEtrangere => {
            (StatusCode::FORBIDDEN, err.to_string())
        }
        _ => (StatusCode::BAD_REQUEST, err.to_string()),
    }
}

#[cfg(feature = "labo")]
#[derive(Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    /// Ignoré si l'adresse désigne un établissement (C2, D-012).
    #[serde(default)]
    pub etablissement_id: Option<Uuid>,
    #[serde(default = "default_role")]
    pub role: String,
}

#[cfg(feature = "labo")]
fn default_role() -> String {
    "staff".into()
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
    /// Ignoré si l'adresse désigne un établissement (C2, D-012).
    #[serde(default)]
    pub etablissement_id: Option<Uuid>,
    /// `web` (défaut) ou `mobile` : durée de session (D-011).
    #[serde(default)]
    pub client: Option<String>,
}

/// Libellé d'appareil lisible (écran « appareils connectés »), tiré du User-Agent.
fn libelle_appareil(headers: &HeaderMap) -> Option<String> {
    let ua = headers.get(axum::http::header::USER_AGENT)?.to_str().ok()?;
    let ua = ua.trim();
    (!ua.is_empty()).then(|| ua.chars().take(120).collect())
}

#[derive(Serialize)]
pub struct AuthSessionDto {
    pub token: String,
    pub user_id: Uuid,
    pub email: String,
    pub etablissement_id: Uuid,
    pub role: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct MeDto {
    pub user_id: Uuid,
    pub email: String,
    pub etablissement_id: Uuid,
    pub role: String,
    /// Présent si le compte est une face élève liée.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eleve_id: Option<Uuid>,
    /// Nombre de passkeys enrôlées : > 0 ⇒ le 2ᵉ facteur est actif.
    pub passkey_count: i64,
    /// `complete` ou `enrolement` (passkey obligatoire à enrôler avant tout).
    pub portee: &'static str,
    /// Casquette active : `principale` ou `parent` (socle 6, P2).
    pub casquette: String,
    /// `mot_de_passe`, `passkey` ou `sso`.
    pub assurance: &'static str,
    /// `web` ou `mobile`.
    pub client: &'static str,
    /// La politique de l'établissement impose la passkey à ce rôle.
    pub passkey_obligatoire: bool,
}

/// Réponse de connexion : soit une session (champs à plat), soit l'exigence
/// d'un 2ᵉ facteur passkey (`mfa_required: true` + challenge).
#[derive(Debug, Serialize)]
pub struct LoginResponseDto {
    pub mfa_required: bool,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub session: Option<AuthSessionDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passkey: Option<PasskeyChallengeDto>,
}

/// Challenge passkey renvoyé au navigateur (options WebAuthn + token opaque).
#[derive(Serialize)]
pub struct PasskeyChallengeDto {
    pub token: String,
    pub options: serde_json::Value,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Challenge d'enrôlement (pas d'expiration exposée : parcours immédiat).
#[derive(Serialize)]
pub struct PasskeyRegisterStartDto {
    pub token: String,
    pub options: serde_json::Value,
}

#[derive(Deserialize)]
pub struct PasskeyRegisterFinishBody {
    pub token: String,
    pub credential: serde_json::Value,
    #[serde(default)]
    pub name: Option<String>,
    /// Mot de passe actuel, re-vérifié avant tout ajout de passkey.
    #[serde(default)]
    pub current_password: String,
}

/// Corps de `DELETE /auth/passkeys/{id}` : mot de passe actuel obligatoire.
#[derive(Deserialize)]
pub struct PasskeyDeleteBody {
    #[serde(default)]
    pub current_password: String,
}

#[derive(Debug, Deserialize)]
pub struct PasskeyLoginStartBody {
    pub email: String,
    /// Ignoré si l'adresse désigne un établissement (C2, D-012).
    #[serde(default)]
    pub etablissement_id: Option<Uuid>,
    #[serde(default)]
    pub client: Option<String>,
}

#[derive(Deserialize)]
pub struct PasskeyLoginFinishBody {
    pub token: String,
    pub credential: serde_json::Value,
}

/// Passkey d'un compte, vue « gestion » (jamais la clé publique).
#[derive(Debug, Serialize)]
pub struct PasskeyDto {
    pub id: Uuid,
    pub name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<ent_db::PasskeyRow> for PasskeyDto {
    fn from(row: ent_db::PasskeyRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
        }
    }
}

/// Fournisseur SSO tel qu'exposé à la page de connexion (aucune donnée
/// sensible : ni client_id, ni issuer, ni secret).
#[derive(Debug, Serialize)]
pub struct OidcPublicProviderDto {
    pub slug: String,
    pub libelle: String,
}

/// Fournisseur SSO en vue gestion (staff) : jamais le `client_secret`.
#[derive(Debug, Serialize)]
pub struct OidcProviderDto {
    pub slug: String,
    pub libelle: String,
    pub issuer: String,
    pub client_id: String,
    pub scopes: String,
    pub redirect_uri: String,
    pub actif: bool,
}

impl From<ent_db::OidcProviderRow> for OidcProviderDto {
    fn from(row: ent_db::OidcProviderRow) -> Self {
        Self {
            slug: row.slug,
            libelle: row.libelle,
            issuer: row.issuer,
            client_id: row.client_id,
            scopes: row.scopes,
            redirect_uri: row.redirect_uri,
            actif: row.actif,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OidcEtabQuery {
    /// Ignoré si l'adresse désigne un établissement (C2, D-012).
    #[serde(default)]
    pub etablissement_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct OidcStartBody {
    /// Ignoré si l'adresse désigne un établissement (C2, D-012).
    #[serde(default)]
    pub etablissement_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct OidcStartDto {
    /// URL d'autorisation de l'IdP : le navigateur y est redirigé.
    pub url: String,
}

#[derive(Deserialize)]
pub struct OidcCompleteBody {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
pub struct OidcUpsertBody {
    pub slug: String,
    pub libelle: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub scopes: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

#[cfg(feature = "labo")]
async fn register(
    State(state): State<AppState>,
    tenant: TenantHote,
    Json(body): Json<RegisterBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    if !registration_open() {
        return Err((
            StatusCode::FORBIDDEN,
            "inscription fermée sur cet environnement".into(),
        ));
    }
    let user = state
        .ent
        .register_user(
            &body.email,
            &body.password,
            body.display_name.as_deref(),
            tenant.etablissement_effectif(body.etablissement_id)?,
            &body.role,
        )
        .await
        .map_err(map_auth)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": user.id,
            "email": user.email,
            "display_name": user.display_name,
        })),
    ))
}

// L'IP cliente fiable (socket, puis `X-Forwarded-For` seulement derrière un
// proxy de confiance déclaré par `ENT_TRUST_PROXY`) est calculée par
// l'extracteur `crate::client_ip::ClientIp`.

async fn login(
    State(state): State<AppState>,
    ip: ClientIp,
    tenant: TenantHote,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> Result<Json<LoginResponseDto>, (StatusCode, String)> {
    let etablissement_id = tenant.etablissement_effectif(body.etablissement_id)?;
    let ip = ip.0;
    let outcome = state
        .ent
        .login_start_client(
            &body.email,
            &body.password,
            etablissement_id,
            ip.as_deref(),
            ent_db::ClientSession::parse_client(body.client.as_deref()),
            libelle_appareil(&headers),
        )
        .await
        .map_err(map_auth)?;
    Ok(Json(login_response(outcome)))
}

/// Traduit l'issue de connexion en réponse HTTP (session à plat ou 2FA).
fn login_response(outcome: ent_db::LoginOutcome) -> LoginResponseDto {
    match outcome {
        ent_db::LoginOutcome::Session(s) => LoginResponseDto {
            mfa_required: false,
            session: Some(AuthSessionDto {
                token: s.token,
                user_id: s.user_id,
                email: s.email,
                etablissement_id: s.etablissement_id,
                role: s.role,
                expires_at: s.expires_at,
            }),
            passkey: None,
        },
        ent_db::LoginOutcome::PasskeyRequired {
            token,
            options,
            expires_at,
        } => LoginResponseDto {
            mfa_required: true,
            session: None,
            passkey: Some(PasskeyChallengeDto {
                token,
                options,
                expires_at,
            }),
        },
    }
}

/// Enrôlement passkey — étape 1 (options). Requiert une session : on n'enrôle
/// une passkey que pour un compte authentifié.
async fn passkey_register_start(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<PasskeyRegisterStartDto>, (StatusCode, String)> {
    let (token, options) = state
        .ent
        .passkey_register_start_for_actor(session.etablissement_id, session.user_id)
        .await
        .map_err(|e| map_compte_sensible(&e).unwrap_or_else(|| map_passkey(e)))?;
    Ok(Json(PasskeyRegisterStartDto { token, options }))
}

/// Enrôlement passkey — étape 2 (vérification + persistance).
async fn passkey_register_finish(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<PasskeyRegisterFinishBody>,
) -> Result<(StatusCode, Json<PasskeyDto>), (StatusCode, String)> {
    let credential: ent_auth::RegisterPublicKeyCredential =
        serde_json::from_value(body.credential)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("credential invalide: {e}")))?;
    let row = state
        .ent
        .passkey_register_finish_session(
            session.etablissement_id,
            session.user_id,
            session.session_id,
            session.portee,
            session.assurance,
            &body.current_password,
            &body.token,
            &credential,
            body.name.clone(),
        )
        .await
        .map_err(|e| map_compte_sensible(&e).unwrap_or_else(|| map_passkey(e)))?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Passkeys du compte connecté.
async fn list_passkeys(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<PasskeyDto>>, (StatusCode, String)> {
    let rows = state
        .ent
        .list_passkeys_for_actor(session.etablissement_id, session.user_id)
        .await
        .map_err(map_passkey)?;
    Ok(Json(rows.into_iter().map(PasskeyDto::from).collect()))
}

/// Supprime une passkey du compte connecté.
async fn delete_passkey(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<PasskeyDeleteBody>>,
) -> Result<StatusCode, (StatusCode, String)> {
    let password = body.map(|Json(b)| b.current_password).unwrap_or_default();
    let removed = state
        .ent
        .delete_passkey_for_actor(
            session.etablissement_id,
            session.user_id,
            session.session_id,
            &password,
            id,
        )
        .await
        .map_err(|e| map_compte_sensible(&e).unwrap_or_else(|| map_passkey(e)))?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "passkey introuvable".into()))
    }
}

/// Connexion sans mot de passe — étape 1 (options pour l'e-mail donné).
/// Réponse **de même forme** (200 + challenge) que le compte existe, soit
/// membre ou non, avec ou sans passkey : quand il n'y en a pas, un challenge
/// factice plausible est renvoyé (anti-énumération). L'étape `finish` le refuse
/// comme tout jeton inconnu.
async fn passkey_login_start(
    State(state): State<AppState>,
    tenant: TenantHote,
    Json(body): Json<PasskeyLoginStartBody>,
) -> Result<Json<PasskeyChallengeDto>, (StatusCode, String)> {
    let (token, options, expires_at) = match state
        .ent
        .passkey_login_start_client(
            tenant.etablissement_effectif(body.etablissement_id)?,
            &body.email,
            ent_db::ClientSession::parse_client(body.client.as_deref()),
        )
        .await
        .map_err(map_passkey)?
    {
        Some(reel) => reel,
        None => state.ent.passkey_challenge_factice(&body.email),
    };
    Ok(Json(PasskeyChallengeDto {
        token,
        options,
        expires_at,
    }))
}

/// Connexion par passkey — étape 2 (vérification de l'assertion → session).
/// Couvre le 2ᵉ facteur (après mot de passe) et le parcours sans mot de passe.
async fn passkey_login_finish(
    State(state): State<AppState>,
    tenant: TenantHote,
    Json(body): Json<PasskeyLoginFinishBody>,
) -> Result<Json<AuthSessionDto>, (StatusCode, String)> {
    let credential: ent_auth::PublicKeyCredential = serde_json::from_value(body.credential)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("credential invalide: {e}")))?;
    let session = state
        .ent
        .passkey_login_finish(&body.token, &credential)
        .await
        .map_err(map_passkey)?;
    // C2 : parcours commencé sur l'adresse d'un autre établissement → aucun jeton.
    if !tenant.accepte(session.etablissement_id) {
        return Err((StatusCode::UNAUTHORIZED, "session d'un autre établissement".into()));
    }
    Ok(Json(AuthSessionDto {
        token: session.token,
        user_id: session.user_id,
        email: session.email,
        etablissement_id: session.etablissement_id,
        role: session.role,
        expires_at: session.expires_at,
    }))
}

/// Récupération : Direction / Admin retire les passkeys d'un membre (perte
/// d'appareil). L'utilisateur retombe sur mot de passe puis réenrôle.
async fn reset_passkeys_admin(
    State(state): State<AppState>,
    session: EntSession,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let removed = state
        .ent
        .reset_passkeys_admin_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            user_id,
        )
        .await
        .map_err(map_passkey)?;
    Ok(Json(serde_json::json!({ "removed": removed })))
}

/// Fournisseurs SSO visibles sur la page de connexion (public : slug + libellé).
async fn oidc_public_providers(
    State(state): State<AppState>,
    tenant: TenantHote,
    Query(q): Query<OidcEtabQuery>,
) -> Result<Json<Vec<OidcPublicProviderDto>>, (StatusCode, String)> {
    let rows = state
        .ent
        .list_oidc_providers_public(tenant.etablissement_effectif(q.etablissement_id)?)
        .await
        .map_err(map_oidc)?;
    Ok(Json(
        rows.into_iter()
            .map(|p| OidcPublicProviderDto {
                slug: p.slug,
                libelle: p.libelle,
            })
            .collect(),
    ))
}

/// Fournisseurs SSO de l'établissement de la session (staff — sans secret).
async fn list_oidc_providers(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<OidcProviderDto>>, (StatusCode, String)> {
    let rows = state
        .ent
        .list_oidc_providers_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_oidc)?;
    Ok(Json(rows.into_iter().map(OidcProviderDto::from).collect()))
}

/// Déclare / met à jour un fournisseur SSO (Direction / Admin). La
/// configuration est validée par découverte réelle de l'IdP.
async fn upsert_oidc_provider(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<OidcUpsertBody>,
) -> Result<Json<OidcProviderDto>, (StatusCode, String)> {
    let row = state
        .ent
        .upsert_oidc_provider_for_actor(
            session.etablissement_id,
            &session.role,
            ent_db::OidcProviderParams {
                slug: body.slug,
                libelle: body.libelle,
                issuer: body.issuer,
                client_id: body.client_id,
                client_secret: body.client_secret,
                scopes: body.scopes,
                redirect_uri: body.redirect_uri,
            },
        )
        .await
        .map_err(map_oidc)?;
    Ok(Json(row.into()))
}

async fn delete_oidc_provider(
    State(state): State<AppState>,
    session: EntSession,
    Path(slug): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let removed = state
        .ent
        .delete_oidc_provider_for_actor(session.etablissement_id, &session.role, &slug)
        .await
        .map_err(map_oidc)?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "fournisseur SSO introuvable".into()))
    }
}

/// Démarre le parcours SSO : renvoie l'URL d'autorisation à ouvrir.
async fn oidc_start(
    State(state): State<AppState>,
    tenant: TenantHote,
    Path(slug): Path<String>,
    Json(body): Json<OidcStartBody>,
) -> Result<(HeaderMap, Json<OidcStartDto>), (StatusCode, String)> {
    let (url, binding) = state
        .ent
        .oidc_start(tenant.etablissement_effectif(body.etablissement_id)?, &slug)
        .await
        .map_err(map_oidc)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        oidc_binding_cookie(&binding, OIDC_BINDING_MAX_AGE_SECS),
    );
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    Ok((headers, Json(OidcStartDto { url })))
}

/// Cookie de liaison du parcours SSO au navigateur (A7) : préfixe `__Host-`
/// (Secure, Path=/, sans Domain), `HttpOnly`, `SameSite=Lax`, durée courte.
pub(crate) const OIDC_BINDING_COOKIE: &str = "__Host-ent_oidc";
const OIDC_BINDING_MAX_AGE_SECS: u32 = 600;

pub(crate) fn oidc_binding_cookie(value: &str, max_age: u32) -> axum::http::HeaderValue {
    // La valeur est un jeton hex généré par le serveur (ou vide à l'effacement).
    axum::http::HeaderValue::from_str(&format!(
        "{OIDC_BINDING_COOKIE}={value}; Max-Age={max_age}; Path=/; Secure; HttpOnly; SameSite=Lax"
    ))
    .unwrap_or_else(|_| axum::http::HeaderValue::from_static(""))
}

/// Valeur du cookie de liaison dans l'en-tête `Cookie` (vide si absent).
pub(crate) fn oidc_binding_from_headers(headers: &HeaderMap) -> String {
    headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == OIDC_BINDING_COOKIE)
        .map(|(_, v)| v.trim().to_string())
        .unwrap_or_default()
}

/// Retour de l'IdP : vérifie l'`id_token`, rattache au compte local et ouvre
/// la session (même contrat de réponse que la connexion mot de passe — un
/// 2ᵉ facteur passkey enrôlé reste exigé).
async fn oidc_complete(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Json(body): Json<OidcCompleteBody>,
) -> Result<(HeaderMap, Json<LoginResponseDto>), (StatusCode, String)> {
    let binding = oidc_binding_from_headers(&headers);
    let outcome = state
        .ent
        .oidc_complete(&body.code, &body.state, &binding)
        .await
        .map_err(map_oidc)?;
    // C2 : parcours commencé sur l'adresse d'un autre établissement → aucun jeton.
    if let ent_db::LoginOutcome::Session(s) = &outcome {
        if !tenant.accepte(s.etablissement_id) {
            return Err((StatusCode::UNAUTHORIZED, "session d'un autre établissement".into()));
        }
    }
    // Le cookie de liaison est à usage unique : effacé dès le retour traité.
    let mut out = HeaderMap::new();
    out.insert(axum::http::header::SET_COOKIE, oidc_binding_cookie("", 0));
    out.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    Ok((out, Json(login_response(outcome))))
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, String)> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "authorization manquante".into()))?;
    state.ent.logout(header).await.map_err(map_auth)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct ChangePasswordBody {
    current_password: String,
    new_password: String,
}

/// Change le mot de passe de l'utilisateur connecté (révoque ses autres sessions).
async fn change_password(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<ChangePasswordBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .ent
        .change_password(
            session.etablissement_id,
            session.user_id,
            session.session_id,
            &body.current_password,
            &body.new_password,
        )
        .await
        .map_err(map_auth)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct ForgotPasswordBody {
    email: String,
}

#[derive(Deserialize)]
struct ResetPasswordBody {
    token: String,
    new_password: String,
}

/// Demande de réinitialisation de mot de passe (public).
/// Réponse **toujours 202** : on ne révèle jamais si l'e-mail existe.
async fn forgot_password(
    State(state): State<AppState>,
    Json(body): Json<ForgotPasswordBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    if body.email.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "e-mail vide".into()));
    }
    // Borne avant tout traitement (sans lien avec l'existence du compte).
    if body.email.trim().len() > ent_auth::MAX_EMAIL_LEN {
        return Err((StatusCode::BAD_REQUEST, "e-mail trop long".into()));
    }
    let base = base_url_publique();
    // Traitement différé : la recherche du compte, l'INSERT du jeton et l'envoi
    // SMTP (seulement si le compte existe) se font hors requête, pour que le
    // temps de réponse ne distingue pas un compte existant d'un compte inconnu.
    // Best-effort : une erreur interne est seulement journalisée.
    let ent = state.ent.clone();
    let email = body.email;
    state.lancer_tache(async move {
        if let Err(e) = ent
            .demander_reinitialisation_mot_de_passe(&email, &base)
            .await
        {
            tracing::warn!("forgot-password: {e}");
        }
    });
    Ok(StatusCode::ACCEPTED)
}

/// Applique une réinitialisation par jeton (public).
/// Change le mot de passe et révoque toutes les sessions de l'utilisateur.
async fn reset_password(
    State(state): State<AppState>,
    Json(body): Json<ResetPasswordBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    if body.token.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "jeton manquant".into()));
    }
    if body.token.len() > 256 {
        return Err((StatusCode::BAD_REQUEST, "lien de réinitialisation invalide".into()));
    }
    state
        .ent
        .reinitialiser_mot_de_passe_par_token(&body.token, &body.new_password)
        .await
        .map_err(map_auth)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct RecuperationPasskeyBody {
    email: String,
    etablissement_id: Uuid,
}

#[derive(Deserialize)]
struct RecuperationPasskeyJeton {
    token: String,
}

/// Borne un jeton de récupération reçu (jamais journalisé).
fn jeton_recuperation_valide(token: &str) -> Result<(), (StatusCode, String)> {
    if token.trim().is_empty() || token.len() > 256 {
        return Err((
            StatusCode::BAD_REQUEST,
            "lien de récupération invalide ou expiré".into(),
        ));
    }
    Ok(())
}

fn map_recuperation_passkey(err: ent_db::EntDbError) -> (StatusCode, String) {
    match &err {
        ent_db::EntDbError::RecuperationLienInvalide => {
            (StatusCode::BAD_REQUEST, err.to_string())
        }
        ent_db::EntDbError::RecuperationDelaiEnCours(_) => {
            (StatusCode::CONFLICT, err.to_string())
        }
        _ => map_compte_sensible(&err).unwrap_or_else(|| map_db(err)),
    }
}

/// Récupération de passkey (D-018), étape 1 — public. Réponse **toujours 202**
/// (compte inconnu, sans passkey, hors établissement ou valide : indiscernable) ;
/// traitement et envoi différés comme `forgot-password`.
async fn recuperation_passkey_demander(
    State(state): State<AppState>,
    Json(body): Json<RecuperationPasskeyBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    if body.email.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "e-mail vide".into()));
    }
    if body.email.trim().len() > ent_auth::MAX_EMAIL_LEN {
        return Err((StatusCode::BAD_REQUEST, "e-mail trop long".into()));
    }
    let base = base_url_publique();
    let ent = state.ent.clone();
    state.lancer_tache(async move {
        if let Err(e) = ent
            .demander_recuperation_passkey(&body.email, body.etablissement_id, &base)
            .await
        {
            tracing::warn!("récupération de passkey : {e}");
        }
    });
    Ok(StatusCode::ACCEPTED)
}

/// Étape 2 — confirmation par le lien reçu : démarre le délai de sécurité.
async fn recuperation_passkey_confirmer(
    State(state): State<AppState>,
    Json(body): Json<RecuperationPasskeyJeton>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    jeton_recuperation_valide(&body.token)?;
    let fin = state
        .ent
        .confirmer_recuperation_passkey(&body.token, &base_url_publique())
        .await
        .map_err(map_recuperation_passkey)?;
    Ok(Json(serde_json::json!({ "effective_at": fin })))
}

/// Annulation par le lien d'annulation reçu par e-mail.
async fn recuperation_passkey_annuler(
    State(state): State<AppState>,
    Json(body): Json<RecuperationPasskeyJeton>,
) -> Result<StatusCode, (StatusCode, String)> {
    jeton_recuperation_valide(&body.token)?;
    state
        .ent
        .annuler_recuperation_passkey(&body.token)
        .await
        .map_err(map_recuperation_passkey)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Finalisation (après le délai) : supprime les passkeys, révoque les sessions.
async fn recuperation_passkey_finaliser(
    State(state): State<AppState>,
    Json(body): Json<RecuperationPasskeyJeton>,
) -> Result<StatusCode, (StatusCode, String)> {
    jeton_recuperation_valide(&body.token)?;
    state
        .ent
        .finaliser_recuperation_passkey(&body.token)
        .await
        .map_err(map_recuperation_passkey)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct RegisterPushDeviceBody {
    platform: String,
    token: String,
    app_id: Option<String>,
}

async fn register_push_device(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<RegisterPushDeviceBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let device = state
        .ent
        .register_push_device_for_actor(
            session.etablissement_id,
            session.user_id,
            &body.platform,
            &body.token,
            body.app_id,
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": device.id,
            "platform": device.platform,
            "token": device.token,
        })),
    ))
}

async fn me(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<MeDto>, (StatusCode, String)> {
    let eleve_id = if session.role == "eleve" {
        state
            .ent
            .eleve_id_for_compte(session.etablissement_id, session.user_id)
            .await
            .map_err(map_db)?
    } else {
        None
    };
    let passkey_count = state
        .ent
        .count_passkeys_for_user(session.user_id)
        .await
        .map_err(map_db)?;
    let passkey_obligatoire = state
        .ent
        .passkey_obligatoire_compte(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(MeDto {
        user_id: session.user_id,
        email: session.email,
        etablissement_id: session.etablissement_id,
        role: session.role,
        eleve_id,
        passkey_count,
        portee: session.portee.as_str(),
        casquette: session.casquette.clone(),
        assurance: session.assurance.as_str(),
        client: session.client.as_str(),
        passkey_obligatoire,
    }))
}

async fn create_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreateEleveBody>,
) -> Result<(StatusCode, Json<EleveDto>), (StatusCode, String)> {
    session.require_manage_eleves()?;
    let eleve = state
        .ent
        .create_eleve(CreateEleve {
            etablissement_id: session.etablissement_id,
            nom: body.nom,
            prenom: body.prenom,
            ine: body.ine,
            sexe: body.sexe,
            date_naissance: body.date_naissance,
            numero_badge: body.numero_badge,
            redoublant: body.redoublant.unwrap_or(false),
            classe: body.classe,
            regime: body.regime,
            mef: body.mef,
            site: body.site,
            date_entree: body.date_entree,
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: Some(session.actor_user_id),
        })
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(EleveDto::from(&eleve))))
}

async fn get_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<EleveDto>, (StatusCode, String)> {
    let vue = state
        .ent
        .get_eleve_vue_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "élève introuvable".into()))?;
    Ok(Json(EleveDto::depuis_vue(&vue)))
}

async fn change_classe(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ChangeClasseBody>,
) -> Result<Json<EleveDto>, (StatusCode, String)> {
    session.require_manage_eleves()?;
    let eleve = state
        .ent
        .change_classe(ChangeClasse {
            etablissement_id: session.etablissement_id,
            eleve_id: id,
            nouvelle_classe: body.nouvelle_classe,
            date_effet: body.date_effet,
            actor_user_id: Some(session.actor_user_id),
        })
        .await
        .map_err(map_db)?;
    Ok(Json(EleveDto::from(&eleve)))
}

async fn change_regime(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ChangeRegimeBody>,
) -> Result<Json<EleveDto>, (StatusCode, String)> {
    session.require_manage_eleves()?;
    let eleve = state
        .ent
        .change_regime(ChangeRegime {
            etablissement_id: session.etablissement_id,
            eleve_id: id,
            nouveau_regime: body.nouveau_regime,
            date_effet: body.date_effet,
            actor_user_id: Some(session.actor_user_id),
        })
        .await
        .map_err(map_db)?;
    Ok(Json(EleveDto::from(&eleve)))
}

async fn sortir(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<SortirBody>,
) -> Result<Json<EleveDto>, (StatusCode, String)> {
    session.require_manage_eleves()?;
    let eleve = state
        .ent
        .sortir(SortirEleve {
            etablissement_id: session.etablissement_id,
            eleve_id: id,
            date_sortie: body.date_sortie,
            motif_sortie: body.motif_sortie,
            actor_user_id: Some(session.actor_user_id),
        })
        .await
        .map_err(map_db)?;
    Ok(Json(EleveDto::from(&eleve)))
}

async fn update_identite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateIdentiteBody>,
) -> Result<Json<EleveDto>, (StatusCode, String)> {
    session.require_manage_eleves()?;
    // Les changements de nom, prénom, sexe, naissance sont des écritures d'identité (E2).
    session
        .require_action(&state.ent, ent_auth::Action::ElevesIdentiteModifier, "rôle insuffisant pour modifier l'identité d'un élève")
        .await?;
    let ine = body
        .ine
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let sexe = body
        .sexe
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let badge = body
        .numero_badge
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let eleve = state
        .ent
        .update_identite(UpdateIdentite {
            etablissement_id: session.etablissement_id,
            eleve_id: id,
            nom: Some(body.nom),
            prenom: Some(body.prenom),
            ine: Some(ine),
            sexe: Some(sexe),
            date_naissance: Some(body.date_naissance),
            numero_badge: Some(badge),
            redoublant: Some(body.redoublant),
            photo_key: None,
            date_entree: Some(body.date_entree),
            actor_user_id: Some(session.actor_user_id),
        })
        .await
        .map_err(map_db)?;
    Ok(Json(EleveDto::from(&eleve)))
}

// ───────────── Socle 2 (l'élève), lot E2 : identité versionnée ─────────────

fn modif_identite(c: ent_types::identite::ChampsIdentiteBody) -> domain_eleve::ModifIdentite {
    domain_eleve::ModifIdentite {
        nom_naissance: c.nom_naissance,
        nom_usage: c.nom_usage,
        prenoms: c.prenoms,
        rang_prenom_usuel: c.rang_prenom_usuel,
        prenom_usage: c.prenom_usage,
        sexe: c.sexe,
        date_naissance: c.date_naissance,
        naissance_commune: c.naissance_commune,
        naissance_commune_cog: c.naissance_commune_cog,
        naissance_pays_cog: c.naissance_pays_cog,
        naissance_lieu_etranger: c.naissance_lieu_etranger,
        date_naissance_precision: c.date_naissance_precision,
        date_naissance_statut: c.date_naissance_statut,
        nom_ecriture_origine: c.nom_ecriture_origine,
        prenoms_ecriture_origine: c.prenoms_ecriture_origine,
    }
}

fn version_identite_dto(v: &domain_eleve::IdentiteVersion) -> ent_types::identite::IdentiteVersionDto {
    ent_types::identite::IdentiteVersionDto {
        id: v.id,
        nom_naissance: v.nom_naissance.clone(),
        nom_usage: v.nom_usage.clone(),
        prenoms: v.prenoms.clone(),
        rang_prenom_usuel: v.rang_prenom_usuel,
        prenom_usage: v.prenom_usage.clone(),
        sexe: v.sexe.clone(),
        date_naissance: v.date_naissance,
        naissance_commune: v.naissance_commune.clone(),
        naissance_commune_cog: v.naissance_commune_cog.clone(),
        naissance_pays_cog: v.naissance_pays_cog.clone(),
        naissance_lieu_etranger: v.naissance_lieu_etranger.clone(),
        date_naissance_precision: v.date_naissance_precision.clone(),
        date_naissance_statut: v.date_naissance_statut.clone(),
        nom_ecriture_origine: v.nom_ecriture_origine.clone(),
        prenoms_ecriture_origine: v.prenoms_ecriture_origine.clone(),
        motif_code: v.motif_code.clone(),
        nature: v.nature.as_str().to_string(),
        effet_du: v.effet_du,
        remplace_version_id: v.remplace_version_id,
        remplacee_par: v.remplacee_par,
        motif: v.motif.clone(),
        piece_document_id: v.piece_document_id,
        source: v.source.clone(),
        sensible: v.sensible,
        enregistre_le: v.enregistre_le,
        enregistre_par: v.enregistre_par,
    }
}

fn resultat_identite_dto(
    r: &ent_db::identite::ResultatIdentite,
) -> ent_types::identite::ResultatIdentiteDto {
    ent_types::identite::ResultatIdentiteDto {
        version: version_identite_dto(&r.version),
        champs: r.champs.iter().map(|c| (*c).to_string()).collect(),
    }
}

/// Les sources réservées au système (import, fusion, reprise…) ne sont pas saisissables
/// depuis une route de personnel.
fn source_saisie(s: Option<String>) -> Result<String, (StatusCode, String)> {
    match s.as_deref() {
        None | Some("saisie") => Ok("saisie".into()),
        Some("ancien_etablissement") => Ok("ancien_etablissement".into()),
        Some(_) => Err((StatusCode::BAD_REQUEST, "source non autorisée pour cette saisie".into())),
    }
}

/// ELV-03 — `POST /eleves/{id}/identite/corrections`.
async fn corriger_identite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::identite::CorrigerIdentiteBody>,
) -> Result<(StatusCode, Json<ent_types::identite::ResultatIdentiteDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::ElevesIdentiteModifier, "rôle insuffisant pour modifier l'identité d'un élève")
        .await?;
    let source = source_saisie(body.source)?;
    let r = state
        .ent
        .corriger_identite(
            &session.role,
            ent_db::identite::CorrectionIdentite {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                version_id: body.version_id,
                modif: modif_identite(body.champs),
                motif_code: body.motif_code,
                motif: body.motif,
                piece_document_id: body.piece_document_id,
                source,
                acteur: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(resultat_identite_dto(&r))))
}

/// ELV-04 — `POST /eleves/{id}/identite/changements`.
async fn changer_etat_civil_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::identite::ChangerEtatCivilBody>,
) -> Result<(StatusCode, Json<ent_types::identite::ResultatIdentiteDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::ElevesIdentiteModifier, "rôle insuffisant pour modifier l'identité d'un élève")
        .await?;
    let source = source_saisie(body.source)?;
    let r = state
        .ent
        .enregistrer_changement_etat_civil(
            &session.role,
            ent_db::identite::ChangementEtatCivil {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                modif: modif_identite(body.champs),
                effet_du: body.effet_du,
                motif_code: body.motif_code,
                motif: body.motif,
                piece_document_id: body.piece_document_id,
                source,
                acteur: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(resultat_identite_dto(&r))))
}

/// ELV-05 — `POST /eleves/{id}/identite/usages`.
async fn definir_usage_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::identite::DefinirUsageBody>,
) -> Result<(StatusCode, Json<ent_types::identite::ResultatIdentiteDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::ElevesIdentiteModifier, "rôle insuffisant pour modifier l'identité d'un élève")
        .await?;
    let source = source_saisie(body.source)?;
    let r = state
        .ent
        .definir_usage(
            &session.role,
            ent_db::identite::UsageIdentite {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                nom_usage: body.nom_usage,
                prenom_usage: body.prenom_usage,
                effet_du: body.effet_du,
                motif_code: body.motif_code,
                motif: body.motif,
                piece_document_id: body.piece_document_id,
                source,
                acteur: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(resultat_identite_dto(&r))))
}

/// ELV-06 — `GET /eleves/{id}/identite/historique` (G5 ; versions sensibles : G6).
async fn historique_identite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ent_types::identite::IdentiteVersionDto>>, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesHistoriqueLire, "rôle insuffisant pour l'historique d'identité")
        .await?;
    let versions = state
        .ent
        .historique_identite_pour_acteur(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(versions.iter().map(version_identite_dto).collect()))
}

// ───────────── Socle 2 (l'élève), lot E3 : identifiants externes ─────────────

fn identifiant_dto(i: &ent_db::identifiants::IdentifiantEleve) -> ent_types::identifiants::IdentifiantDto {
    ent_types::identifiants::IdentifiantDto {
        id: i.id,
        eleve_id: i.eleve_id,
        systeme: i.systeme.clone(),
        valeur: i.valeur.clone(),
        statut: i.statut.clone(),
        provenance: i.provenance.clone(),
        preuve_document_id: i.preuve_document_id,
        valide_du: i.valide_du,
        valide_au: i.valide_au,
        confirme_le: i.confirme_le,
        confirme_par_source: i.confirme_par_source.clone(),
        motif: i.motif.clone(),
        enregistre_le: i.enregistre_le,
        utilisable_pour_export: domain_eleve::StatutIdentifiant::parse(&i.statut)
            .is_some_and(domain_eleve::identifiant_utilisable_pour_export),
    }
}

/// ELV-07 — `GET /eleves/{id}/identifiants` (G3, lecture journalisée).
async fn lister_identifiants_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ent_types::identifiants::IdentifiantDto>>, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesIdentifiantsLire, "rôle insuffisant pour lire les identifiants")
        .await?;
    let l = state
        .ent
        .identifiants_pour_acteur(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(identifiant_dto).collect()))
}

/// ELV-07 — `POST /eleves/{id}/identifiants`. Une collision n'échoue pas : l'identifiant
/// est enregistré `conteste` (réponse 201, `collision = true`).
async fn ajouter_identifiant_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::identifiants::AjouterIdentifiantBody>,
) -> Result<(StatusCode, Json<ent_types::identifiants::ResultatIdentifiantDto>), (StatusCode, String)> {
    session.require_staff()?;
    let systeme = domain_eleve::SystemeIdentifiant::parse(&body.systeme)
        .ok_or((StatusCode::BAD_REQUEST, "système d'identifiant inconnu".into()))?;
    let provenance = match body.provenance.as_deref() {
        None => domain_eleve::ProvenanceIdentifiant::Saisie,
        Some(p) => domain_eleve::ProvenanceIdentifiant::parse(p)
            .ok_or((StatusCode::BAD_REQUEST, "provenance inconnue".into()))?,
    };
    let r = state
        .ent
        .ajouter_identifiant(
            &session.role,
            ent_db::identifiants::AjoutIdentifiant {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                systeme,
                valeur: body.valeur,
                provenance,
                valide_du: body.valide_du,
                preuve_document_id: body.preuve_document_id,
                acteur: Some(session.user_id),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(ent_types::identifiants::ResultatIdentifiantDto {
            identifiant: identifiant_dto(&r.identifiant),
            collision: r.collision,
            autre_eleve_id: r.collision_eleve_id,
            remplace: r.remplace,
            existant: r.existant,
        }),
    ))
}

/// ELV-07 — `POST /eleves/{id}/identifiants/{identifiant_id}/confirmer`.
async fn confirmer_identifiant_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, identifiant_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ent_types::identifiants::ConfirmerIdentifiantBody>,
) -> Result<Json<ent_types::identifiants::IdentifiantDto>, (StatusCode, String)> {
    session.require_staff()?;
    let i = state
        .ent
        .confirmer_identifiant(&session.role, session.etablissement_id, id, identifiant_id, session.user_id, &body.source)
        .await
        .map_err(map_db)?;
    Ok(Json(identifiant_dto(&i)))
}

/// ELV-07 — `POST /eleves/{id}/identifiants/{identifiant_id}/contester`.
async fn contester_identifiant_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, identifiant_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ent_types::identifiants::MotifIdentifiantBody>,
) -> Result<Json<ent_types::identifiants::IdentifiantDto>, (StatusCode, String)> {
    session.require_staff()?;
    let i = state
        .ent
        .contester_identifiant(&session.role, session.etablissement_id, id, identifiant_id, session.user_id, Some(&body.motif))
        .await
        .map_err(map_db)?;
    Ok(Json(identifiant_dto(&i)))
}

/// ELV-07 — `POST /eleves/{id}/identifiants/{identifiant_id}/revoquer`.
async fn revoquer_identifiant_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, identifiant_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ent_types::identifiants::MotifIdentifiantBody>,
) -> Result<Json<ent_types::identifiants::IdentifiantDto>, (StatusCode, String)> {
    session.require_staff()?;
    let i = state
        .ent
        .revoquer_identifiant(&session.role, session.etablissement_id, id, identifiant_id, session.user_id, Some(&body.motif))
        .await
        .map_err(map_db)?;
    Ok(Json(identifiant_dto(&i)))
}

/// ELV-41 — `POST /eleves/{id}/badge/perdu` : l'ancien badge est révoqué, le nouveau actif.
async fn badge_perdu_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::identifiants::BadgePerduBody>,
) -> Result<Json<Option<ent_types::identifiants::ResultatIdentifiantDto>>, (StatusCode, String)> {
    session.require_staff()?;
    let r = state
        .ent
        .declarer_badge_perdu(&session.role, session.etablissement_id, id, session.user_id, body.nouveau_badge.as_deref())
        .await
        .map_err(map_db)?;
    Ok(Json(r.map(|r| ent_types::identifiants::ResultatIdentifiantDto {
        identifiant: identifiant_dto(&r.identifiant),
        collision: r.collision,
        autre_eleve_id: r.collision_eleve_id,
        remplace: r.remplace,
        existant: r.existant,
    })))
}

/// Qualité des données — `GET /eleves/identifiants/contestes`.
async fn lister_identifiants_contestes(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<ent_types::identifiants::IdentifiantConsteDto>>, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesIdentifiantsLire, "rôle insuffisant pour lire les identifiants")
        .await?;
    let l = state
        .ent
        .identifiants_contestes(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(
        l.into_iter()
            .map(|c| ent_types::identifiants::IdentifiantConsteDto {
                identifiant_id: c.identifiant_id,
                eleve_id: c.eleve_id,
                systeme: c.systeme,
                motif: c.motif,
                autre_eleve_id: c.autre_eleve_id,
            })
            .collect(),
    ))
}

// ───────────── Socle 2 (l'élève), lot E4 : scolarité ─────────────

fn etab_scolarite(b: ent_types::scolarite::EtablissementBody) -> Result<domain_eleve::EtablissementScolarite, (StatusCode, String)> {
    let type_ = match b.type_.as_deref() {
        None => None,
        Some(t) => Some(
            domain_eleve::TypeEtablissement::parse(t)
                .ok_or((StatusCode::BAD_REQUEST, "type d'établissement inconnu".into()))?,
        ),
    };
    Ok(domain_eleve::EtablissementScolarite { uai: b.uai, libelle: b.libelle, type_ })
}

fn etab_body(uai: &Option<String>, libelle: &Option<String>, type_: &Option<String>) -> ent_types::scolarite::EtablissementBody {
    ent_types::scolarite::EtablissementBody { uai: uai.clone(), libelle: libelle.clone(), type_: type_.clone() }
}

fn scolarisation_dto(s: &ent_db::scolarite::ScolarisationEleve) -> ent_types::scolarite::ScolarisationDto {
    ent_types::scolarite::ScolarisationDto {
        id: s.id,
        eleve_id: s.eleve_id,
        date_entree: s.date_entree,
        motif_entree: s.motif_entree.clone(),
        origine: etab_body(&s.origine_uai, &s.origine_libelle, &s.origine_type),
        date_sortie: s.date_sortie,
        motif_sortie: s.motif_sortie.clone(),
        motif_sortie_libre: s.motif_sortie_libre.clone(),
        destination: etab_body(&s.destination_uai, &s.destination_libelle, &s.destination_type),
        exeat_document_id: s.exeat_document_id,
        source: s.source.clone(),
        cree_le: s.cree_le,
    }
}

fn inscription_dto(i: &ent_db::scolarite::InscriptionEleve) -> ent_types::scolarite::InscriptionDto {
    ent_types::scolarite::InscriptionDto {
        id: i.id,
        annee_id: i.annee_id,
        scolarisation_id: i.scolarisation_id,
        niveau_id: i.niveau_id,
        mef_id: i.mef_id,
        statut_apprenant: i.statut_apprenant.clone(),
        etat: i.etat.clone(),
        redoublement: i.redoublement,
        decision_passage: i.decision_passage.clone(),
        source: i.source.clone(),
        annee_libelle: None,
    }
}

fn resultat_scolarisation_dto(r: &ent_db::scolarite::ResultatScolarisation) -> ent_types::scolarite::ResultatScolarisationDto {
    ent_types::scolarite::ResultatScolarisationDto {
        scolarisation: scolarisation_dto(&r.scolarisation),
        statut: r.statut.clone(),
        statut_change: r.statut_change,
    }
}

fn code_inconnu<T>(v: Option<T>, quoi: &str) -> Result<T, (StatusCode, String)> {
    v.ok_or((StatusCode::BAD_REQUEST, format!("{quoi} inconnu")))
}

/// ELV-21 (partie scolarité) — `GET /eleves/{id}/scolarite` : statut dérivé et historique (G1).
async fn scolarite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<ent_types::scolarite::ScolariteDto>, (StatusCode, String)> {
    // Responsable légal (démo P1 n° 3) : lecture minimisée de la scolarité de SON enfant ;
    // les droits sont revérifiés dans le dépôt (lien actif, pas de « ne pas solliciter »).
    let mut annees: Vec<(Uuid, String)> = Vec::new();
    let (statut, scol, insc) = if session.role_parsed() == Some(ent_auth::Role::Parent) {
        let (statut, scol, insc, a) = state
            .ent
            .scolarite_pour_famille(session.etablissement_id, session.user_id, id)
            .await
            .map_err(|e| match e {
                ent_db::EntDbError::Forbidden => {
                    (StatusCode::FORBIDDEN, "la scolarité de cet élève ne vous est pas ouverte".to_string())
                }
                autre => map_db(autre),
            })?;
        annees = a;
        (statut, scol, insc)
    } else {
        session.require_staff()?;
        session
            .require_action(&state.ent, ent_auth::Action::ElevesLire, "rôle insuffisant pour lire la scolarité")
            .await?;
        state
            .ent
            .scolarite_pour_acteur(session.etablissement_id, session.user_id, &session.role, id)
            .await
            .map_err(map_db)?
    };
    Ok(Json(ent_types::scolarite::ScolariteDto {
        statut,
        scolarisations: scol.iter().map(scolarisation_dto).collect(),
        inscriptions: insc
            .iter()
            .map(|i| {
                let mut d = inscription_dto(i);
                d.annee_libelle = annees.iter().find(|(id, _)| *id == i.annee_id).map(|(_, l)| l.clone());
                d
            })
            .collect(),
    }))
}

/// ELV-08 — `POST /eleves/{id}/scolarisations`.
async fn ouvrir_scolarisation_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::OuvrirScolarisationBody>,
) -> Result<(StatusCode, Json<ent_types::scolarite::ResultatScolarisationDto>), (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    let motif = code_inconnu(domain_eleve::MotifEntree::parse(&body.motif), "motif d'entrée")?;
    let r = state
        .ent
        .ouvrir_scolarisation(
            &session.role,
            ent_db::scolarite::OuvertureScolarisation {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                date_entree: body.date_entree,
                motif,
                origine: etab_scolarite(body.origine)?,
                source: "saisie".into(),
                source_ref: None,
                acteur: Some(session.user_id),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(resultat_scolarisation_dto(&r))))
}

/// ELV-12 — `POST /eleves/{id}/scolarisations/reintegrer`.
async fn reintegrer_eleve_route(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::ReintegrerBody>,
) -> Result<(StatusCode, Json<ent_types::scolarite::ResultatScolarisationDto>), (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    let r = state
        .ent
        .reintegrer_eleve(
            &session.role,
            ent_db::scolarite::OuvertureScolarisation {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                date_entree: body.date_entree,
                motif: domain_eleve::MotifEntree::Retour,
                origine: etab_scolarite(body.origine)?,
                source: "saisie".into(),
                source_ref: None,
                acteur: Some(session.user_id),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(resultat_scolarisation_dto(&r))))
}

/// ELV-11 ★ — `POST /eleves/{id}/scolarisations/cloturer`.
async fn cloturer_scolarisation_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::CloturerScolarisationBody>,
) -> Result<Json<ent_types::scolarite::ResultatScolarisationDto>, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesSortie, "rôle insuffisant pour clôturer une scolarisation")
        .await?;
    let motif = code_inconnu(domain_eleve::MotifSortie::parse(&body.motif), "motif de sortie")?;
    let r = state
        .ent
        .cloturer_scolarisation(
            &session.role,
            ent_db::scolarite::ClotureScolarisation {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                date_sortie: body.date_sortie,
                motif,
                motif_libre: body.motif_libre,
                destination: etab_scolarite(body.destination)?,
                exeat_document_id: body.exeat_document_id,
                acteur: Some(session.user_id),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(resultat_scolarisation_dto(&r)))
}

/// ELV-34 ★ — `POST /eleves/{id}/deces` : sortie pour décès, `eleve.decede` sans détail.
async fn deces_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::DecesBody>,
) -> Result<Json<ent_types::scolarite::ResultatScolarisationDto>, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesSortie, "rôle insuffisant pour enregistrer un décès")
        .await?;
    let r = state
        .ent
        .enregistrer_deces(&session.role, session.etablissement_id, id, body.date, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(resultat_scolarisation_dto(&r)))
}

/// ELV-09 — `PUT /eleves/{id}/inscriptions/{annee_id}`.
async fn inscription_annuelle_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, annee_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ent_types::scolarite::InscriptionAnnuelleBody>,
) -> Result<Json<ent_types::scolarite::InscriptionDto>, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    let etat = code_inconnu(domain_eleve::EtatInscription::parse(&body.etat), "état d'inscription")?;
    let statut_apprenant = match body.statut_apprenant.as_deref() {
        None => domain_eleve::StatutApprenant::Scolaire,
        Some(s) => code_inconnu(domain_eleve::StatutApprenant::parse(s), "statut d'apprenant")?,
    };
    let i = state
        .ent
        .enregistrer_inscription_annuelle(
            &session.role,
            ent_db::scolarite::InscriptionAnnuelle {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                annee_id,
                niveau_id: body.niveau_id,
                mef_id: body.mef_id,
                statut_apprenant,
                etat,
                redoublement: body.redoublement,
                source: "saisie".into(),
                acteur: Some(session.user_id),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(inscription_dto(&i)))
}

/// ELV-09 — `POST /eleves/{id}/scolarite/classe`.
async fn changer_classe_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::ChangerClasseBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    state
        .ent
        .changer_classe(
            &session.role,
            ent_db::scolarite::ChangementClasse {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                classe_id: body.classe_id,
                date_effet: body.date_effet,
                acteur: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// ELV-09 — `POST /eleves/{id}/scolarite/options`.
async fn changer_options_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::ChangerOptionsBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    state
        .ent
        .changer_options(
            &session.role,
            ent_db::scolarite::ChangementOptions {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                annee_id: body.annee_id,
                ajouter: body.ajouter,
                retirer: body.retirer,
                date_effet: body.date_effet,
                acteur: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn changer_regime_ou_site_route(
    state: AppState,
    session: EntSession,
    id: Uuid,
    kind: domain_eleve::AttributKind,
    body: ent_types::scolarite::ChangerRegimeSiteBody,
) -> Result<StatusCode, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    let derogation = match body.derogation.as_deref() {
        None => None,
        Some(d) => Some(code_inconnu(domain_eleve::MotifDerogationRegime::parse(d), "motif de dérogation")?),
    };
    state
        .ent
        .changer_regime_ou_site(
            &session.role,
            ent_db::scolarite::ChangementRegimeSite {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                kind,
                valeur: body.valeur,
                date_effet: body.date_effet,
                derogation,
                acteur: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// ELV-10 — `POST /eleves/{id}/scolarite/regime` (dates autorisées ou dérogation motivée).
async fn changer_regime_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::ChangerRegimeSiteBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    changer_regime_ou_site_route(state, session, id, domain_eleve::AttributKind::Regime, body).await
}

/// ELV-10 — `POST /eleves/{id}/scolarite/site`.
async fn changer_site_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::ChangerRegimeSiteBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    changer_regime_ou_site_route(state, session, id, domain_eleve::AttributKind::Site, body).await
}

fn capacite_dto(c: &ent_db::scolarite::CapaciteEleve) -> ent_types::scolarite::CapaciteDto {
    ent_types::scolarite::CapaciteDto {
        id: c.id,
        eleve_id: c.eleve_id,
        nature: c.nature.clone(),
        du: c.du,
        au: c.au,
        piece_document_id: c.piece_document_id,
        motif: c.motif.clone(),
        saisi_le: c.saisi_le,
        levee_le: c.levee_le,
    }
}

/// ELV-13 ★ — `POST /eleves/{id}/capacites`.
async fn declarer_capacite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::DeclarerCapaciteBody>,
) -> Result<(StatusCode, Json<ent_types::scolarite::CapaciteDto>), (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesIdentiteModifier, "rôle insuffisant pour déclarer une capacité")
        .await?;
    let nature = code_inconnu(domain_eleve::NatureCapacite::parse(&body.nature), "nature de capacité")?;
    let c = state
        .ent
        .declarer_capacite(
            &session.role,
            ent_db::scolarite::DeclarationCapacite {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                nature,
                du: body.du,
                piece_document_id: body.piece_document_id,
                motif: body.motif,
                acteur: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(capacite_dto(&c))))
}

/// ELV-13 ★ — `POST /eleves/{id}/capacites/{capacite_id}/lever`.
async fn lever_capacite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, capacite_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ent_types::scolarite::LeverCapaciteBody>,
) -> Result<Json<ent_types::scolarite::CapaciteDto>, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesIdentiteModifier, "rôle insuffisant pour lever une capacité")
        .await?;
    let c = state
        .ent
        .lever_capacite(&session.role, session.etablissement_id, id, capacite_id, body.au, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(capacite_dto(&c)))
}

/// ELV-35 — `POST /eleves/{id}/mobilites`.
async fn ouvrir_mobilite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::OuvrirMobiliteBody>,
) -> Result<(StatusCode, Json<ent_types::scolarite::MobiliteDto>), (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    let type_ = code_inconnu(domain_eleve::TypeMobilite::parse(&body.type_), "type de mobilité")?;
    let m = state
        .ent
        .ouvrir_mobilite(&session.role, session.etablissement_id, id, type_, body.du, body.retour_prevu, session.user_id)
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(ent_types::scolarite::MobiliteDto {
            id: m.id,
            eleve_id: m.eleve_id,
            type_: m.type_,
            du: m.du,
            au: m.au,
            retour_prevu: m.retour_prevu,
        }),
    ))
}

/// ELV-35 — `POST /eleves/{id}/mobilites/{mobilite_id}/fermer`.
async fn fermer_mobilite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, mobilite_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ent_types::scolarite::FinPeriodeBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    state
        .ent
        .fermer_mobilite(&session.role, session.etablissement_id, id, mobilite_id, body.au, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// ELV-36 — `POST /eleves/{id}/modalites`.
async fn enregistrer_modalite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::scolarite::EnregistrerModaliteBody>,
) -> Result<(StatusCode, Json<ent_types::scolarite::ModaliteDto>), (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    let code = code_inconnu(domain_eleve::CodeModalite::parse(&body.code), "code de modalité")?;
    let m = state
        .ent
        .enregistrer_modalite(&session.role, session.etablissement_id, id, code, body.du, body.au, body.piece_document_id, session.user_id)
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(ent_types::scolarite::ModaliteDto { id: m.id, eleve_id: m.eleve_id, code: m.code, du: m.du, au: m.au }),
    ))
}

/// ELV-36 — `POST /eleves/{id}/modalites/{modalite_id}/terminer`.
async fn terminer_modalite_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, modalite_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ent_types::scolarite::FinPeriodeBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    session.require_staff()?;
    session
        .require_action(&state.ent, ent_auth::Action::ElevesScolariteGerer, "rôle insuffisant pour gérer la scolarité")
        .await?;
    state
        .ent
        .terminer_modalite(&session.role, session.etablissement_id, id, modalite_id, body.au, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn change_mef(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ChangeMefBody>,
) -> Result<Json<EleveDto>, (StatusCode, String)> {
    session.require_manage_eleves()?;
    let eleve = state
        .ent
        .change_mef(ChangeMef {
            etablissement_id: session.etablissement_id,
            eleve_id: id,
            nouveau_mef: body.nouveau_mef,
            date_effet: body.date_effet,
            actor_user_id: Some(session.actor_user_id),
        })
        .await
        .map_err(map_db)?;
    Ok(Json(EleveDto::from(&eleve)))
}

async fn change_site(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ChangeSiteBody>,
) -> Result<Json<EleveDto>, (StatusCode, String)> {
    session.require_manage_eleves()?;
    let eleve = state
        .ent
        .change_site(ChangeSite {
            etablissement_id: session.etablissement_id,
            eleve_id: id,
            nouveau_site: body.nouveau_site,
            date_effet: body.date_effet,
            actor_user_id: Some(session.actor_user_id),
        })
        .await
        .map_err(map_db)?;
    Ok(Json(EleveDto::from(&eleve)))
}

async fn list_periodes(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<PeriodeDto>>, (StatusCode, String)> {
    // Même filtre que GET élève (parent lié / staff).
    state
        .ent
        .get_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "élève introuvable".into()))?;
    let periodes = state
        .ent
        .list_periodes(session.etablissement_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(periodes.iter().map(PeriodeDto::from).collect()))
}

async fn list_absences(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<AbsenceDto>>, (StatusCode, String)> {
    let allowed = state
        .ent
        .actor_can_read_absences(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    if !allowed {
        return Err((
            StatusCode::FORBIDDEN,
            "pas le droit de lire les absences de cet élève".into(),
        ));
    }
    let list = state
        .ent
        .list_absences_eleve(session.etablissement_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(AbsenceDto::from).collect()))
}

async fn declare_absence(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeclareAbsenceBody>,
) -> Result<(StatusCode, Json<AbsenceDto>), (StatusCode, String)> {
    let allowed = state
        .ent
        .actor_can_declare_absence(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    if !allowed {
        return Err((
            StatusCode::FORBIDDEN,
            "pas le droit de déclarer une absence pour cet élève".into(),
        ));
    }
    let absence = state
        .ent
        .declare_absence(DeclareAbsence {
            etablissement_id: session.etablissement_id,
            eleve_id: id,
            debut: body.debut,
            fin: body.fin,
            motif: body.motif,
            actor_user_id: Some(session.actor_user_id),
        })
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(AbsenceDto::from(&absence))))
}

async fn justifier_absence(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<JustifierAbsenceBody>,
) -> Result<Json<AbsenceDto>, (StatusCode, String)> {
    let absence = state
        .ent
        .justifier_absence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            body.motif_justif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(AbsenceDto::from(&absence)))
}

async fn annuler_soumission_justificatif(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<AbsenceDto>, (StatusCode, String)> {
    let absence = state
        .ent
        .annuler_soumission_justificatif_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(AbsenceDto::from(&absence)))
}

async fn accepter_justificatif(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DecisionAbsenceBody>,
) -> Result<Json<AbsenceDto>, (StatusCode, String)> {
    let absence = state
        .ent
        .accepter_justificatif_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            body.note,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(AbsenceDto::from(&absence)))
}

async fn refuser_justificatif(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DecisionAbsenceBody>,
) -> Result<Json<AbsenceDto>, (StatusCode, String)> {
    let absence = state
        .ent
        .refuser_justificatif_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            body.note,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(AbsenceDto::from(&absence)))
}

async fn annuler_absence_present(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DecisionAbsenceBody>,
) -> Result<Json<AbsenceDto>, (StatusCode, String)> {
    let absence = state
        .ent
        .annuler_absence_vers_present_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            body.note,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(AbsenceDto::from(&absence)))
}

async fn list_absences_a_traiter(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<VsAbsencesQuery>,
) -> Result<Json<Vec<AbsenceDto>>, (StatusCode, String)> {
    if !session
        .role_parsed()
        .map(|r| r.can_lire_vs_absences())
        .unwrap_or(false)
    {
        return Err((StatusCode::FORBIDDEN, "suivi VS réservé au staff".into()));
    }
    let jour = q.jour.unwrap_or_else(today_utc);
    let mut list = state
        .ent
        .list_absences_a_traiter(session.etablissement_id, jour)
        .await
        .map_err(map_db)?;
    if let Some(suivis) = eleves_suivis_si_prof(&state, &session).await? {
        list.retain(|a| suivis.contains(&a.eleve_id));
    }
    Ok(Json(
        enrich_absences_eleve(
            &state,
            session.etablissement_id,
            list.iter().map(AbsenceDto::from).collect(),
        )
        .await?,
    ))
}

/// D-017 : élèves dont l'acteur peut lire la vie scolaire (filtre des listes
/// d'établissement) ; `None` = tout l'établissement.
async fn eleves_suivis_si_prof(
    state: &AppState,
    session: &EntSession,
) -> Result<Option<std::collections::HashSet<Uuid>>, (StatusCode, String)> {
    state
        .ent
        .eleves_autorises(
            session.etablissement_id,
            session.user_id,
            &session.role,
            ent_auth::Action::AbsencesLire,
        )
        .await
        .map_err(map_db)
}

async fn list_signaux_vs(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<SignalAbsenceDto>>, (StatusCode, String)> {
    if !session
        .role_parsed()
        .map(|r| r.can_lire_vs_absences())
        .unwrap_or(false)
    {
        return Err((StatusCode::FORBIDDEN, "signaux VS réservés au staff".into()));
    }
    let mut list = state
        .ent
        .list_signaux_absences_ouverts(session.etablissement_id)
        .await
        .map_err(map_db)?;
    if let Some(suivis) = eleves_suivis_si_prof(&state, &session).await? {
        list.retain(|a| suivis.contains(&a.eleve_id));
    }
    Ok(Json(
        enrich_signaux_eleve(
            &state,
            session.etablissement_id,
            list.iter().map(SignalAbsenceDto::from).collect(),
        )
        .await?,
    ))
}

async fn prendre_signal_vs(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<SignalAbsenceDto>, (StatusCode, String)> {
    let s = state
        .ent
        .prendre_en_charge_signal_for_actor(
            session.etablissement_id,
            id,
            session.user_id,
            &session.role,
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_signaux_eleve(
        &state,
        session.etablissement_id,
        vec![SignalAbsenceDto::from(&s)],
    )
    .await?;
    Ok(Json(
        enriched
            .pop()
            .unwrap_or_else(|| SignalAbsenceDto::from(&s)),
    ))
}

async fn clore_signal_vs(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<CloreSignalBody>,
) -> Result<Json<SignalAbsenceDto>, (StatusCode, String)> {
    let s = state
        .ent
        .clore_signal_absence_for_actor(
            session.etablissement_id,
            id,
            session.user_id,
            &session.role,
            body.note,
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_signaux_eleve(
        &state,
        session.etablissement_id,
        vec![SignalAbsenceDto::from(&s)],
    )
    .await?;
    Ok(Json(
        enriched
            .pop()
            .unwrap_or_else(|| SignalAbsenceDto::from(&s)),
    ))
}

#[derive(Debug, Deserialize)]
struct CommissionDepuisSignalBody {
    jour: Option<NaiveDate>,
    heure: Option<String>,
}

#[derive(Debug, Serialize)]
struct CommissionDepuisSignalDto {
    commission: CommissionDto,
    dossier: CommissionDossierDto,
    signal: SignalAbsenceDto,
}

async fn convoquer_commission_depuis_signal(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<CommissionDepuisSignalBody>,
) -> Result<Json<CommissionDepuisSignalDto>, (StatusCode, String)> {
    let jour = body.jour.unwrap_or_else(today_utc);
    let heure = body
        .heure
        .as_deref()
        .and_then(|s| {
            chrono::NaiveTime::parse_from_str(s, "%H:%M")
                .ok()
                .or_else(|| chrono::NaiveTime::parse_from_str(s, "%H:%M:%S").ok())
        })
        .unwrap_or_else(|| chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap());
    let (commission, dossier, signal) = state
        .ent
        .convoquer_commission_depuis_signal_for_actor(
            session.etablissement_id,
            id,
            &session.role,
            session.user_id,
            jour,
            heure,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(CommissionDepuisSignalDto {
        commission: CommissionDto::from(&commission),
        dossier: CommissionDossierDto::from(&dossier),
        signal: SignalAbsenceDto::from(&signal),
    }))
}

#[derive(Debug, Deserialize)]
struct CommissionsQuery {
    jour: Option<NaiveDate>,
}

async fn list_commissions(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<CommissionsQuery>,
) -> Result<Json<Vec<CommissionDto>>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanctionsPrononcer, "rôle insuffisant pour la commission / sanctions VS")
        .await?;
    let jour = q.jour.unwrap_or_else(today_utc);
    let list = state
        .ent
        .list_commissions_pour_acteur(session.etablissement_id, session.user_id, &session.role, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(CommissionDto::from).collect()))
}

async fn creer_commission(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerCommissionBody>,
) -> Result<(StatusCode, Json<CommissionDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanctionsPrononcer, "rôle insuffisant pour la commission / sanctions VS")
        .await?;
    let heure = parse_heure_hm(&body.heure).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "heure invalide (HH:MM)".into(),
        )
    })?;
    let c = state
        .ent
        .creer_commission_for_actor(
            &session.role,
            CreerCommissionVs {
                etablissement_id: session.etablissement_id,
                jour: body.jour,
                heure,
                objet: body.objet,
                lieu: body.lieu.unwrap_or_default(),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(CommissionDto::from(&c))))
}

async fn list_dossiers_commission(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<CommissionDossierDto>>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanctionsPrononcer, "rôle insuffisant pour la commission / sanctions VS")
        .await?;
    let list = state
        .ent
        .list_dossiers_commission_pour_acteur(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_commission_dossiers(
            &state,
            session.etablissement_id,
            list.iter().map(CommissionDossierDto::from).collect(),
        )
        .await?,
    ))
}

async fn convoquer_commission(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ConvoquerCommissionBody>,
) -> Result<(StatusCode, Json<CommissionDossierDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanctionsPrononcer, "rôle insuffisant pour la commission / sanctions VS")
        .await?;
    let d = state
        .ent
        .convoquer_commission_for_actor(
            &session.role,
            ConvoquerCommission {
                etablissement_id: session.etablissement_id,
                commission_id: id,
                eleve_id: body.eleve_id,
                motifs: body.motifs,
                note_interne: body.note_interne.unwrap_or_default(),
                sanction_id: body.sanction_id,
                incident_id: body.incident_id,
                signale_famille: body.signale_famille.unwrap_or(true),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_commission_dossiers(
        &state,
        session.etablissement_id,
        vec![CommissionDossierDto::from(&d)],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(enriched.pop().unwrap_or_else(|| CommissionDossierDto::from(&d))),
    ))
}

async fn decider_commission_dossier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeciderCommissionBody>,
) -> Result<Json<CommissionDossierDto>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanctionsPrononcer, "rôle insuffisant pour la commission / sanctions VS")
        .await?;
    let decision = DecisionCommission::parse(&body.decision).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "decision invalide".into(),
        )
    })?;
    let d = state
        .ent
        .decider_commission_dossier_for_actor(
            &session.role,
            DeciderCommissionDossier {
                etablissement_id: session.etablissement_id,
                dossier_id: id,
                decision,
                decision_texte: body.decision_texte.unwrap_or_default(),
                points_retires: body.points_retires.unwrap_or(0),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_commission_dossiers(
        &state,
        session.etablissement_id,
        vec![CommissionDossierDto::from(&d)],
    )
    .await?;
    Ok(Json(
        enriched
            .pop()
            .unwrap_or_else(|| CommissionDossierDto::from(&d)),
    ))
}

async fn list_commissions_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<CommissionDossierDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_commissions_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_commission_dossiers(
            &state,
            session.etablissement_id,
            list.iter().map(CommissionDossierDto::from).collect(),
        )
        .await?,
    ))
}

async fn list_incident_types(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<ReferentielVsDto>>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::IncidentsDeclarer, "rôle insuffisant pour déclarer un incident")
        .await?;
    let list = state
        .ent
        .list_incident_types(session.etablissement_id)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(ReferentielVsDto::from).collect()))
}

async fn creer_incident_type(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerReferentielVsBody>,
) -> Result<(StatusCode, Json<ReferentielVsDto>), (StatusCode, String)> {
    session.require_manage_vs_referentiels()?;
    let t = state
        .ent
        .creer_incident_type_for_actor(
            &session.role,
            CreerReferentielVs {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(ReferentielVsDto::from(&t))))
}

async fn list_sanction_types(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<ReferentielVsDto>>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanctionsPrononcer, "rôle insuffisant pour la commission / sanctions VS")
        .await?;
    let list = state
        .ent
        .list_sanction_types(session.etablissement_id)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(ReferentielVsDto::from).collect()))
}

async fn creer_sanction_type(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerReferentielVsBody>,
) -> Result<(StatusCode, Json<ReferentielVsDto>), (StatusCode, String)> {
    session.require_manage_vs_referentiels()?;
    let t = state
        .ent
        .creer_sanction_type_for_actor(
            &session.role,
            CreerReferentielVs {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(ReferentielVsDto::from(&t))))
}

async fn list_sanction_motifs(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<ReferentielVsDto>>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanctionsPrononcer, "rôle insuffisant pour la commission / sanctions VS")
        .await?;
    let list = state
        .ent
        .list_sanction_motifs(session.etablissement_id)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(ReferentielVsDto::from).collect()))
}

async fn creer_sanction_motif(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerReferentielVsBody>,
) -> Result<(StatusCode, Json<ReferentielVsDto>), (StatusCode, String)> {
    session.require_manage_vs_referentiels()?;
    let m = state
        .ent
        .creer_sanction_motif_for_actor(
            &session.role,
            CreerReferentielVs {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(ReferentielVsDto::from(&m))))
}

async fn list_incidents_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<IncidentDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_incidents_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_incidents(
            &state,
            session.etablissement_id,
            list.iter().map(IncidentDto::from).collect(),
        )
        .await?,
    ))
}

async fn declarer_incident(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeclarerIncidentBody>,
) -> Result<(StatusCode, Json<IncidentDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::IncidentsDeclarer, "rôle insuffisant pour déclarer un incident")
        .await?;
    let incident = state
        .ent
        .declarer_incident_for_actor(
            &session.role,
            DeclarerIncident {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                type_id: body.type_id,
                jour: body.jour,
                description: body.description,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_incidents(
        &state,
        session.etablissement_id,
        vec![IncidentDto::from(&incident)],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(enriched.pop().unwrap_or_else(|| IncidentDto::from(&incident))),
    ))
}

async fn list_sanctions_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SanctionDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_sanctions_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_sanctions(
            &state,
            session.etablissement_id,
            list.iter().map(SanctionDto::from).collect(),
        )
        .await?,
    ))
}

async fn prononcer_sanction(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PrononcerSanctionBody>,
) -> Result<(StatusCode, Json<SanctionDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanctionsPrononcer, "rôle insuffisant pour la commission / sanctions VS")
        .await?;
    let sanction = state
        .ent
        .prononcer_sanction_for_actor(
            &session.role,
            PrononcerSanction {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                type_id: body.type_id,
                motif_id: body.motif_id,
                incident_id: body.incident_id,
                jour: body.jour,
                duree_minutes: body.duree_minutes,
                note: body.note,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_sanctions(
        &state,
        session.etablissement_id,
        vec![SanctionDto::from(&sanction)],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(enriched.pop().unwrap_or_else(|| SanctionDto::from(&sanction))),
    ))
}

async fn annuler_sanction(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<SanctionDto>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanctionsPrononcer, "rôle insuffisant pour la commission / sanctions VS")
        .await?;
    let sanction = state
        .ent
        .annuler_sanction_for_actor(
            &session.role,
            AnnulerSanction {
                etablissement_id: session.etablissement_id,
                sanction_id: id,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_sanctions(
        &state,
        session.etablissement_id,
        vec![SanctionDto::from(&sanction)],
    )
    .await?;
    Ok(Json(
        enriched
            .pop()
            .unwrap_or_else(|| SanctionDto::from(&sanction)),
    ))
}

async fn solde_points_vs(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<SoldePointsDto>, (StatusCode, String)> {
    let solde = state
        .ent
        .solde_points_vs_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(SoldePointsDto {
        eleve_id: id,
        solde,
        depart: POINTS_VS_DEPART,
    }))
}

async fn list_chambres_internat(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<ChambreInternatDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_chambres_internat_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(ChambreInternatDto::from).collect()))
}

async fn creer_chambre_internat(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerChambreInternatBody>,
) -> Result<(StatusCode, Json<ChambreInternatDto>), (StatusCode, String)> {
    let c = state
        .ent
        .creer_chambre_internat_for_actor(
            &session.role,
            CreerChambreInternat {
                etablissement_id: session.etablissement_id,
                batiment: body.batiment,
                etage: body.etage,
                code: body.code,
                capacite: body.capacite,
                genre: body.genre,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(ChambreInternatDto::from(&c))))
}

async fn list_affectations_internat(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Result<Json<Vec<AffectationInternatDto>>, (StatusCode, String)> {
    let jour = match q.jour {
        Some(j) => j,
        None => state
            .ent
            .nuit_courante_internat(session.etablissement_id)
            .await
            .map_err(map_db)?,
    };
    let list = state
        .ent
        .list_affectations_internat_jour_for_actor(session.etablissement_id, &session.role, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_affectations_internat(
            &state,
            session.etablissement_id,
            &session.role,
            list.iter().map(AffectationInternatDto::from).collect(),
        )
        .await?,
    ))
}

async fn affecter_eleve_internat(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<AffecterInternatBody>,
) -> Result<(StatusCode, Json<AffectationInternatDto>), (StatusCode, String)> {
    let a = state
        .ent
        .affecter_eleve_internat_for_actor(
            &session.role,
            AffecterEleveInternat {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                chambre_id: body.chambre_id,
                lit: body.lit,
                debut: body.debut,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_affectations_internat(
        &state,
        session.etablissement_id,
        &session.role,
        vec![AffectationInternatDto::from(&a)],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(
            enriched
                .pop()
                .unwrap_or_else(|| AffectationInternatDto::from(&a)),
        ),
    ))
}

async fn clore_affectation_internat(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<CloreAffectationInternatBody>,
) -> Result<Json<AffectationInternatDto>, (StatusCode, String)> {
    let a = state
        .ent
        .clore_affectation_internat_for_actor(
            session.etablissement_id,
            &session.role,
            id,
            body.fin,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_affectations_internat(
        &state,
        session.etablissement_id,
        &session.role,
        vec![AffectationInternatDto::from(&a)],
    )
    .await?;
    Ok(Json(
        enriched
            .pop()
            .unwrap_or_else(|| AffectationInternatDto::from(&a)),
    ))
}

async fn pointer_presence_internat(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<PointerPresenceInternatBody>,
) -> Result<Json<PresenceInternatDto>, (StatusCode, String)> {
    // La nuit par défaut est celle de l'établissement (date du soir, fuseau local : R-IT5).
    let jour = match body.jour {
        Some(j) => j,
        None => state
            .ent
            .nuit_courante_internat(session.etablissement_id)
            .await
            .map_err(map_db)?,
    };
    let p = state
        .ent
        .pointer_presence_internat_for_actor(
            &session.role,
            PointerPresenceInternat {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                jour,
                statut: body.statut,
                note: body.note,
                actor_user_id: session.user_id,
                motif_correction_code: body.motif_correction_code,
                motif_correction_texte: body.motif_correction_texte,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(PresenceInternatDto::from(&p)))
}

#[derive(Debug, Deserialize)]
struct HistoriquePresenceQuery {
    eleve_id: Uuid,
    jour: NaiveDate,
}

async fn historique_presence_internat(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<HistoriquePresenceQuery>,
) -> Result<Json<Vec<PresenceInternatDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .historique_presence_internat_for_actor(session.etablissement_id, &session.role, q.eleve_id, q.jour)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(PresenceInternatDto::from).collect()))
}

async fn modifier_chambre_internat(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ent_types::internat::ChambreModifDto>,
) -> Result<Json<ent_types::internat::ChambreEtendueDto>, (StatusCode, String)> {
    let c = state
        .ent
        .modifier_chambre_internat_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &body,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(c))
}

async fn list_presences_internat(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Result<Json<Vec<PresenceInternatDto>>, (StatusCode, String)> {
    let jour = match q.jour {
        Some(j) => j,
        None => state
            .ent
            .nuit_courante_internat(session.etablissement_id)
            .await
            .map_err(map_db)?,
    };
    let list = state
        .ent
        .list_presences_internat_jour_for_actor(session.etablissement_id, &session.role, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_presences_internat(
            &state,
            session.etablissement_id,
            list.iter().map(PresenceInternatDto::from).collect(),
        )
        .await?,
    ))
}

async fn get_recap_internat(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<JourQuery>,
) -> Result<Json<RecapInternatDto>, (StatusCode, String)> {
    let jour = match q.jour {
        Some(j) => j,
        None => state
            .ent
            .nuit_courante_internat(session.etablissement_id)
            .await
            .map_err(map_db)?,
    };
    let recap = state
        .ent
        .get_recap_internat_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            jour,
        )
        .await
        .map_err(map_db)?;
    let mut affectation = recap.affectation.as_ref().map(AffectationInternatDto::from);
    if let Some(a) = affectation.take() {
        let mut enriched = enrich_affectations_internat(
            &state,
            session.etablissement_id,
            &session.role,
            vec![a],
        )
        .await?;
        affectation = enriched.pop();
    }
    let mut presence_du_jour = recap
        .presence_du_jour
        .as_ref()
        .map(PresenceInternatDto::from);
    if let Some(p) = presence_du_jour.take() {
        let mut enriched = enrich_presences_internat(
            &state,
            session.etablissement_id,
            vec![p],
        )
        .await?;
        presence_du_jour = enriched.pop();
    }
    if let (Some(ref mut a), Some(ref ch)) = (affectation.as_mut(), recap.chambre.as_ref()) {
        if a.chambre_label.is_none() {
            a.chambre_label = Some(format!("{} · {} (ét. {})", ch.code, ch.batiment, ch.etage));
        }
    }
    Ok(Json(RecapInternatDto {
        eleve_id: id,
        affectation,
        chambre: recap.chambre.as_ref().map(ChambreInternatDto::from),
        presence_du_jour,
        suivis: recap
            .suivis_famille
            .iter()
            .map(SuiviInternatDto::from)
            .collect(),
    }))
}

async fn creer_suivi_internat(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<CreerSuiviInternatBody>,
) -> Result<(StatusCode, Json<SuiviInternatDto>), (StatusCode, String)> {
    let s = state
        .ent
        .creer_suivi_internat_for_actor(
            &session.role,
            CreerSuiviInternat {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                jour: body.jour.unwrap_or_else(today_utc),
                texte: body.texte,
                visible_famille: body.visible_famille,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut dto = SuiviInternatDto::from(&s);
    if domain_eleve::mots_de_sante_detectes(&s.texte) {
        dto.avertissement = Some("sante".to_string());
    }
    Ok((StatusCode::CREATED, Json(dto)))
}

async fn list_dispenses_vs(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<ActivesQuery>,
) -> Result<Json<Vec<DispenseVsDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_dispenses_vs_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            q.actives.unwrap_or(false),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(DispenseVsDto::from).collect()))
}

async fn poser_dispense_vs(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PoserDispenseVsBody>,
) -> Result<(StatusCode, Json<DispenseVsDto>), (StatusCode, String)> {
    let d = state
        .ent
        .poser_dispense_vs_for_actor(
            &session.role,
            PoserDispenseVs {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                motif: body.motif,
                debut: body.debut,
                fin: body.fin,
                commentaire: body.commentaire,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(DispenseVsDto::from(&d))))
}

async fn clore_dispense_vs(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<DispenseVsDto>, (StatusCode, String)> {
    let d = state
        .ent
        .clore_dispense_vs_for_actor(
            session.etablissement_id,
            &session.role,
            id,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(DispenseVsDto::from(&d)))
}

async fn list_dispenses_vs_jour(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Result<Json<Vec<DispenseVsDto>>, (StatusCode, String)> {
    let jour = q.jour.unwrap_or_else(today_utc);
    let list = state
        .ent
        .list_dispenses_vs_jour_for_actor(session.etablissement_id, session.user_id, &session.role, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_dispenses_vs(
            &state,
            session.etablissement_id,
            list.iter().map(DispenseVsDto::from).collect(),
        )
        .await?,
    ))
}

async fn list_journal_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<JournalEntreeDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_journal_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            q.limit.unwrap_or(50),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(JournalEntreeDto::from).collect()))
}

#[derive(Debug, Deserialize)]
struct AuditJournalQuery {
    domaine: Option<String>,
    limit: Option<i64>,
}

/// Journal d'audit par domaine (défaut `auth`) — staff habilité.
async fn list_audit_journal(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<AuditJournalQuery>,
) -> Result<Json<Vec<JournalEntreeDto>>, (StatusCode, String)> {
    let domaine = q.domaine.unwrap_or_else(|| "auth".into());
    let list = state
        .ent
        .list_journal_domaine_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &domaine,
            q.limit.unwrap_or(100),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(JournalEntreeDto::from).collect()))
}

#[derive(Debug, Deserialize)]
struct CopiloteAskBody {
    question: String,
}

/// Copilote ENT — question en langage naturel, réponse lecture seule.
/// Réservé au personnel (chaque intention revérifie la capacité du rôle).
async fn copilote_ask(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CopiloteAskBody>,
) -> Result<Json<ent_db::ReponseCopilote>, (StatusCode, String)> {
    if !session
        .role_parsed()
        .map(|r| r.is_staff())
        .unwrap_or(false)
    {
        return Err((StatusCode::FORBIDDEN, "réservé au personnel".into()));
    }
    let question = body.question.trim();
    if question.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "question vide".into()));
    }
    if question.chars().count() > 300 {
        return Err((
            StatusCode::BAD_REQUEST,
            "question trop longue (300 caractères max)".into(),
        ));
    }
    let reponse = state
        .ent
        .copilote_repondre(
            session.etablissement_id,
            session.user_id,
            &session.role,
            question,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(reponse))
}

/// Questions supportées par le copilote (aide / autocomplétion UI).
async fn copilote_capacites(
    session: EntSession,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !session
        .role_parsed()
        .map(|r| r.is_staff())
        .unwrap_or(false)
    {
        return Err((StatusCode::FORBIDDEN, "réservé au personnel".into()));
    }
    Ok(Json(serde_json::json!({ "capacites": ent_db::CAPACITES })))
}

async fn list_alertes_cpe_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<AlerteCpeDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_alertes_cpe_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(AlerteCpeDto::from).collect()))
}

async fn creer_alerte_cpe(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<CreerAlerteCpeBody>,
) -> Result<(StatusCode, Json<AlerteCpeDto>), (StatusCode, String)> {
    let a = state
        .ent
        .creer_alerte_cpe_for_actor(
            &session.role,
            CreerAlerteCpe {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                titre: body.titre,
                detail: body.detail,
                severite: body.severite,
                visible_famille: body.visible_famille,
                jour: body.jour.unwrap_or_else(today_utc),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(AlerteCpeDto::from(&a))))
}

async fn list_alertes_cpe_ouvertes(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<AlerteCpeDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_alertes_cpe_ouvertes_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_alertes_cpe(
            &state,
            session.etablissement_id,
            list.iter().map(AlerteCpeDto::from).collect(),
        )
        .await?,
    ))
}

async fn clore_alerte_cpe(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<CloreAlerteCpeBody>,
) -> Result<Json<AlerteCpeDto>, (StatusCode, String)> {
    let a = state
        .ent
        .clore_alerte_cpe_for_actor(
            session.etablissement_id,
            &session.role,
            id,
            session.user_id,
            body.note,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(AlerteCpeDto::from(&a)))
}

async fn list_docs_a_signer_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<ActivesQuery>,
) -> Result<Json<Vec<DocASignerDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_docs_a_signer_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            q.actives.unwrap_or(false),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(DocASignerDto::from).collect()))
}

async fn publier_doc_a_signer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PublierDocASignerBody>,
) -> Result<(StatusCode, Json<DocASignerDto>), (StatusCode, String)> {
    let d = state
        .ent
        .publier_doc_a_signer_for_actor(
            &session.role,
            PublierDocASigner {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                code: body.code,
                titre: body.titre,
                corps: body.corps,
                annee_scolaire: body.annee_scolaire,
                echeance: body.echeance,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(DocASignerDto::from(&d))))
}

async fn signer_doc_a_signer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<DocASignerDto>, (StatusCode, String)> {
    let d = state
        .ent
        .signer_doc_a_signer_for_actor(
            &session.role,
            SignerDocASigner {
                etablissement_id: session.etablissement_id,
                doc_id: id,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(DocASignerDto::from(&d)))
}

async fn annuler_doc_a_signer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<DocASignerDto>, (StatusCode, String)> {
    let d = state
        .ent
        .annuler_doc_a_signer_for_actor(
            session.etablissement_id,
            &session.role,
            id,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(DocASignerDto::from(&d)))
}

async fn list_docs_a_signer_etab(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<DocASignerDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_docs_a_signer_etab_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_docs_a_signer(
            &state,
            session.etablissement_id,
            list.iter().map(DocASignerDto::from).collect(),
        )
        .await?,
    ))
}

/// Annonce en deux temps (M0b, D8) : sans `confirmer`, un aperçu chiffré est créé et rien n'est
/// envoyé ; `POST /annonces/{id}/confirmer` publie et met les envois en file.
async fn publier_annonce(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerAnnonceBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let cmd = CreerAnnonce {
        etablissement_id: session.etablissement_id,
        titre: body.titre,
        corps: body.corps,
        audience: body.audience,
        cible_classe: body.cible_classe,
        actor_user_id: session.user_id,
    };
    if body.confirmer {
        let a = state
            .ent
            .publier_annonce_for_actor(&session.role, cmd)
            .await
            .map_err(map_db)?;
        return Ok((
            StatusCode::CREATED,
            Json(serde_json::to_value(AnnonceDto::from(&a)).unwrap_or_default()),
        ));
    }
    let apercu = state
        .ent
        .apercu_annonce_for_actor(&session.role, cmd)
        .await
        .map_err(map_db)?;
    let mut v = serde_json::to_value(AnnonceDto::from(&apercu.annonce)).unwrap_or_default();
    v["nb_eleves"] = apercu.nb_eleves.into();
    v["nb_responsables"] = apercu.nb_responsables.into();
    v["nb_eleves_sans_responsable"] = apercu.nb_eleves_sans_responsable.into();
    Ok((StatusCode::CREATED, Json(v)))
}

async fn confirmer_annonce(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<AnnonceDto>, (StatusCode, String)> {
    let a = state
        .ent
        .confirmer_annonce_for_actor(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(AnnonceDto::from(&a)))
}

async fn retirer_annonce(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<RetirerAnnonceBody>,
) -> Result<Json<AnnonceDto>, (StatusCode, String)> {
    let a = state
        .ent
        .retirer_annonce_for_actor(
            &session.role,
            session.etablissement_id,
            session.user_id,
            id,
            &body.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(AnnonceDto::from(&a)))
}

async fn list_annonces(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<AnnonceDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_annonces_for_actor(session.etablissement_id, &session.role, 100)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(AnnonceDto::from).collect()))
}

/// Notification interne produite par la toile des événements (socle 2, S2-0).
#[derive(Debug, serde::Serialize)]
struct NotificationDto {
    id: Uuid,
    eleve_id: Option<Uuid>,
    kind: String,
    titre: String,
    corps: String,
    lu: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Notifications de l'utilisateur connecté, dans l'établissement de sa session.
async fn mes_notifications(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<NotificationDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .mes_notifications(session.etablissement_id, session.user_id, 100)
        .await
        .map_err(map_db)?;
    Ok(Json(
        list.into_iter()
            .map(|n| NotificationDto {
                id: n.id,
                eleve_id: n.eleve_id,
                kind: n.kind,
                titre: n.titre,
                corps: n.corps,
                lu: n.lu_le.is_some(),
                created_at: n.created_at,
            })
            .collect(),
    ))
}

async fn list_annonces_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<AnnonceDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_annonces_famille_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(AnnonceDto::from).collect()))
}

async fn declarer_retard(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeclarerRetardBody>,
) -> Result<(StatusCode, Json<RetardDto>), (StatusCode, String)> {
    let r = state
        .ent
        .declarer_retard_for_actor(
            &session.role,
            DeclarerRetard {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                jour: body.jour,
                minutes: body.minutes,
                motif: body.motif,
                source_appel_id: None,
                actor_user_id: Some(session.user_id),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(RetardDto::from(&r))))
}

async fn list_retards_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<RetardDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_retards_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            200,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(RetardDto::from).collect()))
}

#[derive(Debug, Deserialize)]
struct CompetencePeriodeQuery {
    periode_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct InclureInactifsQuery {
    inclure_inactifs: Option<bool>,
}

async fn list_competences_items(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<InclureInactifsQuery>,
) -> Result<Json<Vec<CompetenceItemDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_competences_items_for_actor(
            &session.role,
            session.etablissement_id,
            q.inclure_inactifs.unwrap_or(false),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(CompetenceItemDto::from).collect()))
}

async fn creer_competence_item(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerCompetenceItemBody>,
) -> Result<(StatusCode, Json<CompetenceItemDto>), (StatusCode, String)> {
    let item = state
        .ent
        .creer_competence_item_for_actor(
            &session.role,
            CreerCompetenceItem {
                etablissement_id: session.etablissement_id,
                domaine: body.domaine,
                code: body.code,
                libelle: body.libelle,
                ordre: body.ordre,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(CompetenceItemDto::from(&item))))
}

async fn list_competences_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<CompetencePeriodeQuery>,
) -> Result<Json<Vec<EvaluationCompetenceDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_evaluations_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            q.periode_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        list.iter().map(EvaluationCompetenceDto::from).collect(),
    ))
}

async fn evaluer_competence(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<EvaluerCompetenceBody>,
) -> Result<(StatusCode, Json<EvaluationCompetenceDto>), (StatusCode, String)> {
    session.require_saisir_notes()?;
    let planned = state
        .ent
        .evaluer_competence_for_actor(
            &session.role,
            EvaluerCompetence {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                item_id: body.item_id,
                periode_id: body.periode_id,
                niveau: body.niveau,
                commentaire: body.commentaire,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    // Relit la ligne enrichie (libellé / domaine) pour la réponse famille/staff.
    let lue = state
        .ent
        .list_evaluations_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            Some(planned.periode_id),
        )
        .await
        .map_err(map_db)?
        .into_iter()
        .find(|e| e.item_id == planned.item_id)
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "évaluation non relue".to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(EvaluationCompetenceDto::from(&lue)),
    ))
}

async fn publier_livret_competences(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PublierBulletinBody>,
) -> Result<(StatusCode, Json<DocumentMetaDto>), (StatusCode, String)> {
    let doc = state
        .ent
        .publier_livret_competences_for_actor(
            &session.role,
            session.etablissement_id,
            id,
            body.periode_id,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(DocumentMetaDto {
            id: doc.id,
            eleve_id: doc.eleve_id,
            titre: doc.titre,
            tiroir: doc.tiroir.as_str().to_string(),
            confidentialite: doc.confidentialite.as_str().to_string(),
            source: doc.source,
            code: doc.code,
            accessible: true,
            grant_etat: None,
        }),
    ))
}

async fn list_conseils_classe(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ClasseQuery>,
) -> Result<Json<Vec<ConseilClasseDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_conseils_classe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.classe.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(ConseilClasseDto::from).collect()))
}

async fn creer_conseil_classe(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerConseilClasseBody>,
) -> Result<(StatusCode, Json<ConseilClasseDto>), (StatusCode, String)> {
    let heure = NaiveTime::parse_from_str(&body.heure, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(&body.heure, "%H:%M:%S"))
        .map_err(|_| (StatusCode::BAD_REQUEST, "heure invalide (HH:MM)".into()))?;
    let c = state
        .ent
        .creer_conseil_classe_for_actor(
            &session.role,
            CreerConseilClasse {
                etablissement_id: session.etablissement_id,
                classe: body.classe,
                periode_libelle: body.periode_libelle,
                jour: body.jour,
                heure,
                lieu: body.lieu,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(ConseilClasseDto::from(&c))))
}

async fn tenir_conseil_classe(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<ConseilClasseDto>, (StatusCode, String)> {
    let c = state
        .ent
        .tenir_conseil_classe_for_actor(
            session.etablissement_id,
            &session.role,
            id,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(ConseilClasseDto::from(&c)))
}

async fn list_dossiers_conseil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ConseilClasseDossierDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_dossiers_conseil_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_conseil_dossiers(
            &state,
            session.etablissement_id,
            list.iter().map(ConseilClasseDossierDto::from).collect(),
        )
        .await?,
    ))
}

async fn ajouter_dossier_conseil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<AjouterDossierConseilBody>,
) -> Result<(StatusCode, Json<ConseilClasseDossierDto>), (StatusCode, String)> {
    let d = state
        .ent
        .ajouter_dossier_conseil_for_actor(
            &session.role,
            AjouterDossierConseil {
                etablissement_id: session.etablissement_id,
                conseil_id: id,
                eleve_id: body.eleve_id,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_conseil_dossiers(
        &state,
        session.etablissement_id,
        vec![ConseilClasseDossierDto::from(&d)],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(
            enriched
                .pop()
                .unwrap_or_else(|| ConseilClasseDossierDto::from(&d)),
        ),
    ))
}

/// Route historique **fermée** (lot V0, socle 21) : 410 avec le chemin de remplacement.
async fn decider_dossier_conseil(_session: EntSession) -> (StatusCode, String) {
    (
        StatusCode::GONE,
        "route historique fermée (V0, D4) : les décisions du conseil se prennent par PUT /evaluations/conseils/{id}/decisions".into(),
    )
}

async fn list_conseils_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ConseilClasseDossierDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_conseils_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_conseil_dossiers(
            &state,
            session.etablissement_id,
            list.iter().map(ConseilClasseDossierDto::from).collect(),
        )
        .await?,
    ))
}

async fn list_info_comp_champs(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<InfoCompChampDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_info_comp_champs_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(InfoCompChampDto::from).collect()))
}

async fn creer_info_comp_champ(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerInfoCompChampBody>,
) -> Result<(StatusCode, Json<InfoCompChampDto>), (StatusCode, String)> {
    let c = state
        .ent
        .creer_info_comp_champ_for_actor(
            &session.role,
            CreerInfoCompChamp {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                type_valeur: body.type_valeur,
                obligatoire: body.obligatoire,
                visible_famille: body.visible_famille,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(InfoCompChampDto::from(&c))))
}

async fn list_info_comp_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<InfoCompEleveDto>, (StatusCode, String)> {
    let (champs, valeurs) = state
        .ent
        .list_info_comp_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    let champs_dto: Vec<InfoCompChampDto> = champs.iter().map(InfoCompChampDto::from).collect();
    let by_champ: std::collections::HashMap<Uuid, String> = champs_dto
        .iter()
        .map(|c| (c.id, format!("{} — {}", c.code, c.libelle)))
        .collect();
    let valeurs_dto: Vec<InfoCompValeurDto> = valeurs
        .iter()
        .map(|v| {
            let mut dto = InfoCompValeurDto::from(v);
            dto.champ_label = by_champ.get(&v.champ_id).cloned();
            dto
        })
        .collect();
    Ok(Json(InfoCompEleveDto {
        champs: champs_dto,
        valeurs: valeurs_dto,
    }))
}

async fn upsert_info_comp_valeur(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<UpsertInfoCompValeurBody>,
) -> Result<Json<InfoCompValeurDto>, (StatusCode, String)> {
    let v = state
        .ent
        .upsert_info_comp_valeur_for_actor(
            &session.role,
            UpsertInfoCompValeur {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                champ_id: body.champ_id,
                valeur: body.valeur,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut dto = InfoCompValeurDto::from(&v);
    if let Ok(champs) = state
        .ent
        .list_info_comp_champs_for_actor(session.etablissement_id, &session.role)
        .await
    {
        if let Some(c) = champs.iter().find(|c| c.id == v.champ_id) {
            dto.champ_label = Some(format!("{} — {}", c.code, c.libelle));
        }
    }
    Ok(Json(dto))
}

async fn list_periodes_vacances(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<PeriodeVacancesDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_periodes_vacances_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(PeriodeVacancesDto::from).collect()))
}

async fn creer_periode_vacances(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerPeriodeVacancesBody>,
) -> Result<(StatusCode, Json<PeriodeVacancesDto>), (StatusCode, String)> {
    let p = state
        .ent
        .creer_periode_vacances_for_actor(
            &session.role,
            CreerPeriodeVacances {
                etablissement_id: session.etablissement_id,
                libelle: body.libelle,
                kind: body.kind,
                debut: body.debut,
                fin: body.fin,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(PeriodeVacancesDto::from(&p))))
}

async fn jour_ferme(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Result<Json<JourFermeDto>, (StatusCode, String)> {
    let jour = q.jour.unwrap_or_else(today_utc);
    let ferme = state
        .ent
        .jour_ferme_for_actor(session.etablissement_id, &session.role, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(JourFermeDto { jour, ferme }))
}

async fn supprimer_periode_vacances(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .ent
        .supprimer_periode_vacances_for_actor(session.etablissement_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn rechercher_homonymes(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<RechercherHomonymesBody>,
) -> Result<Json<Vec<HomonymeCandidatDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .rechercher_homonymes_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            RechercherHomonymes {
                etablissement_id: session.etablissement_id,
                nom: body.nom,
                prenom: body.prenom,
                date_naissance: body.date_naissance,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(HomonymeCandidatDto::from).collect()))
}

#[derive(Debug, Deserialize)]
struct MenusCantineQuery {
    debut: Option<NaiveDate>,
    fin: Option<NaiveDate>,
}

async fn list_menus_cantine(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<MenusCantineQuery>,
) -> Result<Json<Vec<MenuCantineDto>>, (StatusCode, String)> {
    let debut = q.debut.unwrap_or_else(today_utc);
    let fin = q.fin.unwrap_or(debut + chrono::Duration::days(6));
    let list = state
        .ent
        .list_menus_cantine_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            debut,
            fin,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(MenuCantineDto::from).collect()))
}

async fn upsert_menu_cantine(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<UpsertMenuCantineBody>,
) -> Result<Json<MenuCantineDto>, (StatusCode, String)> {
    let m = state
        .ent
        .upsert_menu_cantine_for_actor(
            &session.role,
            UpsertMenuCantine {
                etablissement_id: session.etablissement_id,
                jour: body.jour,
                entree: body.entree,
                plat: body.plat,
                dessert: body.dessert,
                notes: body.notes,
                publie: body.publie,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(MenuCantineDto::from(&m)))
}

async fn list_tarifs_conso(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Result<Json<Vec<TarifConsoDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_tarifs_conso_for_actor(session.etablissement_id, &session.role, q.jour)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(TarifConsoDto::from).collect()))
}

async fn creer_tarif_conso(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerTarifConsoBody>,
) -> Result<(StatusCode, Json<TarifConsoDto>), (StatusCode, String)> {
    let t = state
        .ent
        .creer_tarif_conso_for_actor(
            &session.role,
            CreerTarifConso {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                montant_centimes: body.montant_centimes,
                debut: body.debut,
                fin: body.fin,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(TarifConsoDto::from(&t))))
}

#[derive(Debug, Deserialize)]
struct PeriodeDatesQuery {
    debut: Option<NaiveDate>,
    fin: Option<NaiveDate>,
}

async fn list_absences_rh(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<PeriodeDatesQuery>,
) -> Result<Json<Vec<AbsenceRhDto>>, (StatusCode, String)> {
    let debut = q.debut.unwrap_or_else(today_utc);
    let fin = q.fin.unwrap_or(debut);
    let list = state
        .ent
        .list_absences_rh_for_actor(session.etablissement_id, &session.role, debut, fin)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_absences_rh(&state, session.etablissement_id, list.iter().map(AbsenceRhDto::from).collect()).await?,
    ))
}

async fn declarer_absence_rh(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<DeclarerAbsenceRhBody>,
) -> Result<(StatusCode, Json<AbsenceRhDto>), (StatusCode, String)> {
    let a = state
        .ent
        .declarer_absence_rh_for_actor(
            &session.role,
            DeclarerAbsenceRh {
                etablissement_id: session.etablissement_id,
                user_id: body.user_id.unwrap_or(session.user_id),
                debut: body.debut,
                fin: body.fin,
                motif: body.motif,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_absences_rh(&state, session.etablissement_id, vec![AbsenceRhDto::from(&a)]).await?;
    Ok((StatusCode::CREATED, Json(enriched.remove(0))))
}

async fn valider_absence_rh(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<AbsenceRhDto>, (StatusCode, String)> {
    let a = state
        .ent
        .valider_absence_rh_for_actor(
            session.etablissement_id,
            &session.role,
            id,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_absences_rh(&state, session.etablissement_id, vec![AbsenceRhDto::from(&a)]).await?;
    Ok(Json(enriched.remove(0)))
}

#[derive(Debug, Deserialize)]
struct QuotientQuery {
    annee_scolaire: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FoyersQuery {
    q: Option<String>,
    limit: Option<i64>,
}

async fn create_foyer(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreateFoyerBody>,
) -> Result<(StatusCode, Json<FoyerListItemDto>), (StatusCode, String)> {
    let id = state
        .ent
        .create_foyer_for_actor(
            &session.role,
            session.etablissement_id,
            body.libelle.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(FoyerListItemDto {
            id,
            libelle: body.libelle,
            nb_eleves: 0,
        }),
    ))
}

async fn rattacher_responsable_foyer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<RattacherResponsableFoyerBody>,
) -> Result<(StatusCode, Json<EleveLienDto>), (StatusCode, String)> {
    let (saved, activation) = state
        .ent
        .rattacher_responsable_foyer_for_actor(
            &session.role,
            session.etablissement_id,
            session.user_id,
            id,
            body.eleve_id,
            body.email.trim(),
            body.display_name.as_deref(),
            body.telephone.as_deref(),
            &base_url_publique(),
            body.lien.as_deref().unwrap_or("autre"),
            body.peut_voir_notes.unwrap_or(true),
            body.peut_voir_vs.unwrap_or(true),
            body.peut_justifier_absence.unwrap_or(true),
            body.peut_autoriser_sorties.unwrap_or(true),
            body.est_payeur.unwrap_or(false),
            body.part_paiement,
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(EleveLienDto {
            id: saved.id,
            eleve_id: saved.eleve_id,
            user_id: saved.user_id,
            foyer_id: saved.foyer_id,
            lien: saved.lien,
            peut_voir_notes: saved.peut_voir_notes,
            peut_voir_vs: saved.peut_voir_vs,
            peut_justifier_absence: saved.peut_justifier_absence,
            peut_autoriser_sorties: saved.peut_autoriser_sorties,
            est_payeur: saved.est_payeur,
            part_paiement: saved.part_paiement,
            activation_envoyee: activation,
        }),
    ))
}

async fn set_user_telephone(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<SetTelephoneBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .ent
        .set_user_telephone_for_actor(
            &session.role,
            session.etablissement_id,
            session.user_id,
            id,
            &body.telephone,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_user_email(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<SetEmailBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .ent
        .set_user_email_for_actor(
            &session.role,
            session.etablissement_id,
            session.user_id,
            id,
            body.email.trim(),
            body.current_password.as_deref(),
            &base_url_publique(),
        )
        .await
        .map_err(|e| map_compte_sensible(&e).unwrap_or_else(|| map_db(e)))?;
    // 202 : le changement n'est effectif qu'après confirmation par la nouvelle adresse.
    Ok(StatusCode::ACCEPTED)
}

/// Changement d'e-mail, confirmation par le lien reçu à la nouvelle adresse (public).
async fn email_confirmer(
    State(state): State<AppState>,
    Json(body): Json<RecuperationPasskeyJeton>,
) -> Result<StatusCode, (StatusCode, String)> {
    jeton_recuperation_valide(&body.token)?;
    state
        .ent
        .confirmer_changement_email(&body.token)
        .await
        .map_err(map_email_lien)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Changement d'e-mail, annulation / rétablissement par le lien de l'ancienne adresse (public).
async fn email_annuler(
    State(state): State<AppState>,
    Json(body): Json<RecuperationPasskeyJeton>,
) -> Result<StatusCode, (StatusCode, String)> {
    jeton_recuperation_valide(&body.token)?;
    state
        .ent
        .annuler_changement_email(&body.token)
        .await
        .map_err(map_email_lien)?;
    Ok(StatusCode::NO_CONTENT)
}

fn map_email_lien(err: ent_db::EntDbError) -> (StatusCode, String) {
    match &err {
        ent_db::EntDbError::EmailLienInvalide => (StatusCode::BAD_REQUEST, err.to_string()),
        _ => map_compte_sensible(&err).unwrap_or_else(|| map_db(err)),
    }
}

async fn delete_lien_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path((eleve_id, lien_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .ent
        .delete_lien_eleve_for_actor(
            &session.role,
            session.etablissement_id,
            Some(eleve_id),
            lien_id,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_foyers(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<FoyersQuery>,
) -> Result<Json<Vec<FoyerListItemDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_foyers_for_actor(
            session.etablissement_id,
            &session.role,
            q.q.as_deref(),
            q.limit.unwrap_or(80),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        list.into_iter()
            .map(|(id, libelle, nb_eleves)| FoyerListItemDto {
                id,
                libelle,
                nb_eleves,
            })
            .collect(),
    ))
}

async fn get_foyer_detail(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<FoyerDetailDto>, (StatusCode, String)> {
    let (fid, libelle, liens) = state
        .ent
        .get_foyer_detail_for_actor(session.etablissement_id, &session.role, id)
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "foyer introuvable".into()))?;
    Ok(Json(FoyerDetailDto {
        id: fid,
        libelle,
        liens: liens
            .into_iter()
            .map(|l| FoyerLienDto {
                lien_id: l.lien_id,
                eleve_id: l.eleve_id,
                eleve_label: format!("{} {}", l.eleve_prenom, l.eleve_nom)
                    .trim()
                    .to_string(),
                eleve_classe: l.eleve_classe,
                user_id: l.user_id,
                display_name: l.display_name,
                user_email: l.user_email,
                telephone: l.telephone,
                lien: l.lien,
                peut_voir_notes: l.peut_voir_notes,
                peut_voir_vs: l.peut_voir_vs,
                peut_justifier_absence: l.peut_justifier_absence,
                peut_autoriser_sorties: l.peut_autoriser_sorties,
                est_payeur: l.est_payeur,
                part_paiement: l.part_paiement,
            })
            .collect(),
    }))
}

async fn update_lien_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path((eleve_id, lien_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateLienBody>,
) -> Result<Json<EleveLienDto>, (StatusCode, String)> {
    let updated = state
        .ent
        .update_lien_for_actor(
            &session.role,
            session.etablissement_id,
            lien_id,
            ent_db::UpdateLien {
                lien: body.lien.unwrap_or_default(),
                peut_voir_notes: body.peut_voir_notes,
                peut_voir_vs: body.peut_voir_vs,
                peut_justifier_absence: body.peut_justifier_absence,
                peut_autoriser_sorties: body.peut_autoriser_sorties,
                est_payeur: body.est_payeur,
                part_paiement: body.part_paiement,
            },
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    if updated.eleve_id != eleve_id {
        return Err((StatusCode::BAD_REQUEST, "lien hors élève".into()));
    }
    Ok(Json(EleveLienDto {
        id: updated.id,
        eleve_id: updated.eleve_id,
        user_id: updated.user_id,
        foyer_id: updated.foyer_id,
        lien: updated.lien,
        peut_voir_notes: updated.peut_voir_notes,
        peut_voir_vs: updated.peut_voir_vs,
        peut_justifier_absence: updated.peut_justifier_absence,
        peut_autoriser_sorties: updated.peut_autoriser_sorties,
        est_payeur: updated.est_payeur,
        part_paiement: updated.part_paiement,
        activation_envoyee: None,
    }))
}

async fn update_liens_eleve_batch(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
    Json(body): Json<UpdateLiensEleveBody>,
) -> Result<Json<Vec<EleveLienDto>>, (StatusCode, String)> {
    let cmds = body
        .liens
        .into_iter()
        .map(|i| {
            (
                i.lien_id,
                ent_db::UpdateLien {
                    lien: i.lien.unwrap_or_default(),
                    peut_voir_notes: i.peut_voir_notes,
                    peut_voir_vs: i.peut_voir_vs,
                    peut_justifier_absence: i.peut_justifier_absence,
                    peut_autoriser_sorties: i.peut_autoriser_sorties,
                    est_payeur: i.est_payeur,
                    part_paiement: i.part_paiement,
                },
            )
        })
        .collect();
    let saved = state
        .ent
        .update_liens_eleve_for_actor(
            &session.role,
            session.etablissement_id,
            eleve_id,
            cmds,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        saved
            .into_iter()
            .map(|u| EleveLienDto {
                id: u.id,
                eleve_id: u.eleve_id,
                user_id: u.user_id,
                foyer_id: u.foyer_id,
                lien: u.lien,
                peut_voir_notes: u.peut_voir_notes,
                peut_voir_vs: u.peut_voir_vs,
                peut_justifier_absence: u.peut_justifier_absence,
                peut_autoriser_sorties: u.peut_autoriser_sorties,
                est_payeur: u.est_payeur,
                part_paiement: u.part_paiement,
                activation_envoyee: None,
            })
            .collect(),
    ))
}

async fn get_quotient_foyer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<QuotientQuery>,
) -> Result<Json<Option<QuotientFoyerDto>>, (StatusCode, String)> {
    let annee = q
        .annee_scolaire
        .unwrap_or_else(|| annee_scolaire_de(today_utc()));
    let q = state
        .ent
        .get_quotient_foyer_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &annee,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(q.as_ref().map(QuotientFoyerDto::from)))
}

async fn upsert_quotient_foyer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<UpsertQuotientBody>,
) -> Result<Json<QuotientFoyerDto>, (StatusCode, String)> {
    if body.foyer_id != id {
        return Err((StatusCode::BAD_REQUEST, "foyer_id incohérent".into()));
    }
    let q = state
        .ent
        .upsert_quotient_foyer_for_actor(
            &session.role,
            UpsertQuotientFoyer {
                etablissement_id: session.etablissement_id,
                foyer_id: id,
                annee_scolaire: body.annee_scolaire,
                quotient_centimes: body.quotient_centimes,
                parts_x100: body.parts_x100,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(QuotientFoyerDto::from(&q)))
}

async fn list_taux_tva(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<TauxTvaDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_taux_tva_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(TauxTvaDto::from).collect()))
}

async fn creer_taux_tva(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerTauxTvaBody>,
) -> Result<(StatusCode, Json<TauxTvaDto>), (StatusCode, String)> {
    let t = state
        .ent
        .creer_taux_tva_for_actor(
            &session.role,
            CreerTauxTva {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                taux_bps: body.taux_bps,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(TauxTvaDto::from(&t))))
}

#[derive(Debug, Deserialize)]
struct CalculTtcQuery {
    ht_centimes: i64,
    taux_bps: i32,
}

async fn calcul_ttc(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<CalculTtcQuery>,
) -> Result<Json<CalculTtcDto>, (StatusCode, String)> {
    let ttc = state
        .ent
        .calculer_ttc_for_actor(
            session.etablissement_id,
            &session.role,
            q.ht_centimes,
            q.taux_bps,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(CalculTtcDto {
        ht_centimes: q.ht_centimes,
        taux_bps: q.taux_bps,
        ttc_centimes: ttc,
    }))
}

async fn list_trombi_classe(
    State(state): State<AppState>,
    session: EntSession,
    Path(classe): Path<String>,
    Query(q): Query<JourQuery>,
) -> Result<Json<Vec<TrombiEntreeDto>>, (StatusCode, String)> {
    let jour = q.jour.unwrap_or_else(today_utc);
    let list = state
        .ent
        .list_trombi_classe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &classe,
            jour,
        )
        .await
        .map_err(map_db)?;
    // RP1-03 : un professeur de groupe ne voit que ses élèves, pas toute la classe.
    let suivis = state
        .ent
        .eleves_suivis_restreints(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(
        list.iter()
            .filter(|e| suivis.as_ref().is_none_or(|s| s.contains(&e.eleve_id)))
            .map(TrombiEntreeDto::from)
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
struct PoserPhotoBody {
    mime_type: String,
    contenu_base64: String,
}

async fn poser_photo_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PoserPhotoBody>,
) -> Result<Json<EleveDto>, (StatusCode, String)> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body.contenu_base64.trim())
        .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".into()))?;
    let eleve = state
        .ent
        .poser_photo_eleve_for_actor(
            &session.role,
            session.etablissement_id,
            id,
            session.user_id,
            &body.mime_type,
            &bytes,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(EleveDto::from(&eleve)))
}

async fn get_photo_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, String)> {
    let bytes = state
        .ent
        .get_photo_eleve_bytes_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?
        .ok_or((StatusCode::NOT_FOUND, "pas de photo".into()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .body(Body::from(bytes))
        .map_err(|e| erreur_interne(&e))?)
}

async fn list_params_etab(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<ParamEtabDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_params_etab_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(ParamEtabDto::from).collect()))
}

async fn upsert_param_etab(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<UpsertParamEtabBody>,
) -> Result<Json<ParamEtabDto>, (StatusCode, String)> {
    let p = state
        .ent
        .upsert_param_etab_for_actor(
            &session.role,
            UpsertParamEtab {
                etablissement_id: session.etablissement_id,
                cle: body.cle,
                valeur: body.valeur,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(ParamEtabDto::from(&p)))
}

async fn get_param_etab(
    State(state): State<AppState>,
    session: EntSession,
    Path(cle): Path<String>,
) -> Result<Json<Option<ParamEtabDto>>, (StatusCode, String)> {
    let p = state
        .ent
        .get_param_etab_for_actor(session.etablissement_id, &session.role, &cle)
        .await
        .map_err(map_db)?;
    Ok(Json(p.as_ref().map(ParamEtabDto::from)))
}

async fn list_salles(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<SalleDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_salles_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(SalleDto::from).collect()))
}

async fn list_motifs_absence(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<AbsenceMotifDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_motifs_absence_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(AbsenceMotifDto::from).collect()))
}

async fn creer_motif_absence(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerAbsenceMotifBody>,
) -> Result<(StatusCode, Json<AbsenceMotifDto>), (StatusCode, String)> {
    let m = state
        .ent
        .create_motif_absence_etab_for_actor(
            &session.role,
            session.etablissement_id,
            &body.code,
            &body.libelle,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(AbsenceMotifDto::from(&m))))
}

async fn creer_salle(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerSalleBody>,
) -> Result<(StatusCode, Json<SalleDto>), (StatusCode, String)> {
    let s = state
        .ent
        .creer_salle_for_actor(
            &session.role,
            CreerSalle {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                capacite: body.capacite,
                site: body.site,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(SalleDto::from(&s))))
}

async fn list_filieres(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<FiliereDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_filieres_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(FiliereDto::from).collect()))
}

async fn creer_filiere(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerFiliereBody>,
) -> Result<(StatusCode, Json<FiliereDto>), (StatusCode, String)> {
    let f = state
        .ent
        .creer_filiere_for_actor(
            &session.role,
            CreerFiliere {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                mef: body.mef,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(FiliereDto::from(&f))))
}

#[derive(Debug, Deserialize)]
struct VoeuxQuery {
    annee_scolaire: Option<String>,
}

async fn list_voeux_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<VoeuxQuery>,
) -> Result<Json<Vec<VoeuDto>>, (StatusCode, String)> {
    let annee = q
        .annee_scolaire
        .unwrap_or_else(|| annee_scolaire_de(today_utc()));
    let list = state
        .ent
        .list_voeux_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &annee,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_voeux(
            &state,
            session.etablissement_id,
            &session.role,
            list.iter().map(VoeuDto::from).collect(),
        )
        .await?,
    ))
}

async fn retirer_voeu(
    State(state): State<AppState>,
    session: EntSession,
    Path((eleve_id, voeu_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .ent
        .retirer_voeu_for_actor(
            &session.role,
            session.etablissement_id,
            session.user_id,
            eleve_id,
            voeu_id,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn poser_voeu(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PoserVoeuBody>,
) -> Result<(StatusCode, Json<VoeuDto>), (StatusCode, String)> {
    let v = state
        .ent
        .poser_voeu_for_actor(
            &session.role,
            PoserVoeu {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                filiere_id: body.filiere_id,
                rang: body.rang,
                annee_scolaire: body.annee_scolaire,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_voeux(
        &state,
        session.etablissement_id,
        &session.role,
        vec![VoeuDto::from(&v)],
    )
    .await?;
    Ok((StatusCode::CREATED, Json(enriched.remove(0))))
}

async fn list_mails(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<MailOutboxDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_mails_recents_for_actor(
            session.etablissement_id,
            &session.role,
            q.limit.unwrap_or(50),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(MailOutboxDto::from).collect()))
}

async fn list_sms(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<SmsOutboxDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_sms_recents_for_actor(
            session.etablissement_id,
            &session.role,
            q.limit.unwrap_or(50),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(SmsOutboxDto::from).collect()))
}

async fn get_inscription_suivi(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<QuotientQuery>,
) -> Result<Json<Option<InscriptionSuiviDto>>, (StatusCode, String)> {
    let annee = q
        .annee_scolaire
        .unwrap_or_else(|| annee_scolaire_de(today_utc()));
    let s = state
        .ent
        .get_inscription_suivi_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &annee,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(match s.as_ref() {
        Some(suivi) => {
            let mut enriched = enrich_inscriptions_suivi(
                &state,
                session.etablissement_id,
                vec![InscriptionSuiviDto::from(suivi)],
            )
            .await?;
            enriched.pop()
        }
        None => None,
    }))
}

async fn upsert_inscription_suivi(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<UpsertInscriptionSuiviBody>,
) -> Result<Json<InscriptionSuiviDto>, (StatusCode, String)> {
    let s = state
        .ent
        .upsert_inscription_suivi_for_actor(
            &session.role,
            UpsertInscriptionSuivi {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                annee_scolaire: body.annee_scolaire,
                etape: body.etape,
                motif_code: body.motif_code,
                commentaire: body.commentaire,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_inscriptions_suivi(
        &state,
        session.etablissement_id,
        vec![InscriptionSuiviDto::from(&s)],
    )
    .await?;
    Ok(Json(
        enriched
            .pop()
            .unwrap_or_else(|| InscriptionSuiviDto::from(&s)),
    ))
}

#[derive(Debug, Deserialize)]
struct InscriptionsEtapeQuery {
    annee_scolaire: Option<String>,
    etape: String,
}

async fn list_inscriptions_par_etape(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<InscriptionsEtapeQuery>,
) -> Result<Json<Vec<InscriptionSuiviDto>>, (StatusCode, String)> {
    let annee = q
        .annee_scolaire
        .unwrap_or_else(|| annee_scolaire_de(today_utc()));
    let list = state
        .ent
        .list_inscriptions_par_etape_for_actor(
            session.etablissement_id,
            &session.role,
            &annee,
            &q.etape,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_inscriptions_suivi(
            &state,
            session.etablissement_id,
            list.iter().map(InscriptionSuiviDto::from).collect(),
        )
        .await?,
    ))
}

async fn list_iban_histo(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<IbanHistoDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_iban_histo_foyer_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            q.limit.unwrap_or(20),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(IbanHistoDto::from).collect()))
}

#[derive(Debug, Deserialize)]
struct ExportLslQuery {
    periode_id: Uuid,
    classe: String,
    jour: Option<NaiveDate>,
}

async fn export_lsl(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ExportLslQuery>,
) -> Result<Response, (StatusCode, String)> {
    let csv = state
        .ent
        .exporter_lsl_csv_for_actor(
            &session.role,
            session.user_id,
            ExporterLsl {
                etablissement_id: session.etablissement_id,
                periode_id: q.periode_id,
                classe: Some(q.classe),
                jour_reference: q.jour.unwrap_or_else(today_utc),
            },
        )
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "export", "lsl", "Export LSL").await;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-lsl.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| erreur_interne(&e))?)
}

async fn export_lsun(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ExportLslQuery>,
) -> Result<Response, (StatusCode, String)> {
    let csv = state
        .ent
        .exporter_lsun_csv_for_actor(
            &session.role,
            session.user_id,
            ExporterLsun {
                etablissement_id: session.etablissement_id,
                periode_id: q.periode_id,
                classe: Some(q.classe),
                jour_reference: q.jour.unwrap_or_else(today_utc),
            },
        )
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "export", "lsun", "Export LSUN").await;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-lsun.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| erreur_interne(&e))?)
}

async fn list_editions_vs(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<EditionVsDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_editions_vs_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(EditionVsDto::from).collect()))
}

async fn creer_edition_vs(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerEditionVsBody>,
) -> Result<(StatusCode, Json<EditionVsDto>), (StatusCode, String)> {
    let tpl = state
        .ent
        .creer_edition_vs_for_actor(
            &session.role,
            CreerEditionVs {
                etablissement_id: session.etablissement_id,
                code: body.code,
                titre: body.titre,
                corps: body.corps,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(EditionVsDto::from(&tpl))))
}

async fn rendre_edition_vs(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<RendreEditionVsBody>,
) -> Result<Json<EditionVsRenduDto>, (StatusCode, String)> {
    let texte = state
        .ent
        .rendre_edition_vs_for_actor(
            session.etablissement_id,
            &session.role,
            id,
            body.eleve_id,
            body.date.unwrap_or_else(today_utc),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(EditionVsRenduDto { texte }))
}

async fn rendre_edition_vs_par_code(
    State(state): State<AppState>,
    session: EntSession,
    Path(code): Path<String>,
    Json(body): Json<RendreEditionVsBody>,
) -> Result<Json<EditionVsRenduDto>, (StatusCode, String)> {
    let texte = state
        .ent
        .rendre_edition_vs_par_code_for_actor(
            session.etablissement_id,
            &session.role,
            &code,
            body.eleve_id,
            body.date.unwrap_or_else(today_utc),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(EditionVsRenduDto { texte }))
}

async fn list_entreprises(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<EntrepriseDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_entreprises_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(EntrepriseDto::from).collect()))
}

async fn creer_entreprise(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerEntrepriseBody>,
) -> Result<(StatusCode, Json<EntrepriseDto>), (StatusCode, String)> {
    let e = state
        .ent
        .creer_entreprise_for_actor(
            &session.role,
            CreerEntreprise {
                etablissement_id: session.etablissement_id,
                raison_sociale: body.raison_sociale,
                siret: body.siret,
                adresse: body.adresse.unwrap_or_default(),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(EntrepriseDto::from(&e))))
}

async fn creer_convention_stage(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerConventionStageBody>,
) -> Result<(StatusCode, Json<ConventionStageDto>), (StatusCode, String)> {
    let c = state
        .ent
        .creer_convention_stage_for_actor(
            &session.role,
            CreerConventionStage {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                entreprise_id: body.entreprise_id,
                debut: body.debut,
                fin: body.fin,
                tuteur_nom: body.tuteur_nom,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_conventions_stage(
        &state,
        session.etablissement_id,
        vec![ConventionStageDto::from(&c)],
    )
    .await?;
    Ok((StatusCode::CREATED, Json(enriched.remove(0))))
}

async fn list_conventions_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ConventionStageDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_conventions_eleve_for_actor(
            session.etablissement_id,
            &session.role,
            session.user_id,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_conventions_stage(
            &state,
            session.etablissement_id,
            list.iter().map(ConventionStageDto::from).collect(),
        )
        .await?,
    ))
}

#[derive(Debug, Deserialize)]
struct ExportOndeQuery {
    classe: Option<String>,
    jour: Option<NaiveDate>,
}

async fn export_onde(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ExportOndeQuery>,
) -> Result<Response, (StatusCode, String)> {
    let csv = state
        .ent
        .exporter_onde_csv_for_actor(
            &session.role,
            session.user_id,
            ExporterOnde {
                etablissement_id: session.etablissement_id,
                classe: q.classe,
                jour_reference: q.jour.unwrap_or_else(today_utc),
            },
        )
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "export", "onde", "Export ONDE").await;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-onde.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| erreur_interne(&e))?)
}

async fn export_notanet(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ExportLslQuery>,
) -> Result<Response, (StatusCode, String)> {
    let csv = state
        .ent
        .exporter_notanet_csv_for_actor(
            &session.role,
            session.user_id,
            ExporterNotanet {
                etablissement_id: session.etablissement_id,
                periode_id: q.periode_id,
                classe: q.classe,
            },
            q.jour.unwrap_or_else(today_utc),
        )
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "export", "notanet", "Export Notanet").await;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-notanet.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| erreur_interne(&e))?)
}

#[derive(Debug, Deserialize)]
struct ExportParcoursupQuery {
    annee_scolaire: String,
    classe: String,
    jour: Option<NaiveDate>,
}

async fn export_parcoursup(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ExportParcoursupQuery>,
) -> Result<Response, (StatusCode, String)> {
    let csv = state
        .ent
        .exporter_parcoursup_csv_for_actor(
            &session.role,
            session.user_id,
            ExporterParcoursup {
                etablissement_id: session.etablissement_id,
                annee_scolaire: q.annee_scolaire,
                classe: q.classe,
            },
            q.jour.unwrap_or_else(today_utc),
        )
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "export", "parcoursup", "Export Parcoursup").await;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-parcoursup.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| erreur_interne(&e))?)
}

async fn get_recap_sortie(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<RecapSortieDto>, (StatusCode, String)> {
    let recap = state
        .ent
        .get_recap_sortie_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_recap_sortie(&state, session.etablissement_id, RecapSortieDto::from(&recap))
            .await?,
    ))
}

async fn upsert_autorisation_sortie(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<UpsertAutorisationSortieBody>,
) -> Result<Json<AutorisationSortieDto>, (StatusCode, String)> {
    let a = state
        .ent
        .upsert_autorisation_sortie_for_actor(
            &session.role,
            UpsertAutorisationSortie {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                matin: body.matin,
                midi: body.midi,
                soir: body.soir,
                interdit_etablissement: body.interdit_etablissement,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_autorisations_sortie(
        &state,
        session.etablissement_id,
        vec![AutorisationSortieDto::from(&a)],
    )
    .await?;
    Ok(Json(
        enriched
            .pop()
            .unwrap_or_else(|| AutorisationSortieDto::from(&a)),
    ))
}

async fn demander_sortie_exceptionnelle(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DemanderSortieBody>,
) -> Result<(StatusCode, Json<DemandeSortieDto>), (StatusCode, String)> {
    let creneau = SortieCreneau::parse(&body.creneau)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "créneau invalide (matin|midi|soir)".into()))?;
    let d = state
        .ent
        .demander_sortie_exceptionnelle_for_actor(
            &session.role,
            DemanderSortieExceptionnelle {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                jour: body.jour,
                creneau,
                motif: body.motif,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_demandes_sortie(
        &state,
        session.etablissement_id,
        vec![DemandeSortieDto::from(&d)],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(enriched.pop().unwrap_or_else(|| DemandeSortieDto::from(&d))),
    ))
}

async fn list_demandes_sortie_en_attente(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<DemandeSortieDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_demandes_sortie_en_attente_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_demandes_sortie(
            &state,
            session.etablissement_id,
            list.iter().map(DemandeSortieDto::from).collect(),
        )
        .await?,
    ))
}

async fn decider_demande_sortie(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeciderSortieBody>,
) -> Result<Json<DemandeSortieDto>, (StatusCode, String)> {
    let d = state
        .ent
        .decider_demande_sortie_for_actor(
            &session.role,
            DeciderDemandeSortie {
                etablissement_id: session.etablissement_id,
                demande_id: id,
                accepter: body.accepter,
                decision_note: body.decision_note.unwrap_or_default(),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_demandes_sortie(
        &state,
        session.etablissement_id,
        vec![DemandeSortieDto::from(&d)],
    )
    .await?;
    Ok(Json(
        enriched
            .pop()
            .unwrap_or_else(|| DemandeSortieDto::from(&d)),
    ))
}

async fn controle_sortie_badge(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<ControleSortieBody>,
) -> Result<Json<ControleSortieDto>, (StatusCode, String)> {
    let creneau = SortieCreneau::parse(&body.creneau)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "créneau invalide (matin|midi|soir)".into()))?;
    let jour = body.jour.unwrap_or_else(today_utc);
    let maintenant = body
        .heure
        .as_deref()
        .and_then(|h| NaiveTime::parse_from_str(h, "%H:%M").ok())
        .or_else(|| Some(Utc::now().time()));
    let c = state
        .ent
        .controle_sortie_for_actor(
            session.etablissement_id,
            &session.role,
            session.user_id,
            body.eleve_id,
            jour,
            creneau,
            body.site_portail,
            maintenant,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(ControleSortieDto::from(&c)))
}

async fn get_sortie_param(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<SortieParamDto>, (StatusCode, String)> {
    let p = state
        .ent
        .get_sortie_param_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(SortieParamDto {
        autoriser_changement_site: p.map(|x| x.autoriser_changement_site).unwrap_or(false),
    }))
}

async fn upsert_sortie_param(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<UpsertSortieParamBody>,
) -> Result<Json<SortieParamDto>, (StatusCode, String)> {
    let p = state
        .ent
        .upsert_sortie_param_for_actor(
            &session.role,
            UpsertSortieParamEtab {
                etablissement_id: session.etablissement_id,
                autoriser_changement_site: body.autoriser_changement_site,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(SortieParamDto {
        autoriser_changement_site: p.autoriser_changement_site,
    }))
}

#[derive(Debug, Deserialize)]
struct ControlesSortieQuery {
    jour: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
struct JourQuery {
    jour: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
struct ActivesQuery {
    actives: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LimitQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ClasseQuery {
    classe: Option<String>,
}

async fn list_controles_sortie(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ControlesSortieQuery>,
) -> Result<Json<Vec<ControleSortieDto>>, (StatusCode, String)> {
    let jour = q.jour.unwrap_or_else(today_utc);
    let list = state
        .ent
        .list_controles_sortie_jour_for_actor(session.etablissement_id, &session.role, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_controles_sortie(
            &state,
            session.etablissement_id,
            list.iter().map(ControleSortieDto::from).collect(),
        )
        .await?,
    ))
}

async fn list_sortie_regles(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<SortieRegleNiveauDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_sortie_regles_niveau_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(SortieRegleNiveauDto::from).collect()))
}

async fn upsert_sortie_regle(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<UpsertSortieRegleBody>,
) -> Result<Json<SortieRegleNiveauDto>, (StatusCode, String)> {
    let r = state
        .ent
        .upsert_sortie_regle_niveau_for_actor(
            &session.role,
            UpsertSortieRegleNiveau {
                etablissement_id: session.etablissement_id,
                niveau: body.niveau,
                matin_autorise: body.matin_autorise,
                midi_autorise: body.midi_autorise,
                soir_autorise: body.soir_autorise,
                choix_parent_interdit: body.choix_parent_interdit,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(SortieRegleNiveauDto::from(&r)))
}

async fn list_sortie_regles_regime(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<SortieRegleRegimeDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_sortie_regles_regime_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(SortieRegleRegimeDto::from).collect()))
}

async fn upsert_sortie_regle_regime(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<UpsertSortieRegleRegimeBody>,
) -> Result<Json<SortieRegleRegimeDto>, (StatusCode, String)> {
    let r = state
        .ent
        .upsert_sortie_regle_regime_for_actor(
            &session.role,
            UpsertSortieRegleRegime {
                etablissement_id: session.etablissement_id,
                regime: body.regime,
                matin_autorise: body.matin_autorise,
                midi_autorise: body.midi_autorise,
                soir_autorise: body.soir_autorise,
                choix_parent_interdit: body.choix_parent_interdit,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(SortieRegleRegimeDto::from(&r)))
}

#[derive(Debug, Deserialize)]
pub struct VsAbsencesQuery {
    pub jour: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct AbsentQuery {
    pub jour: NaiveDate,
}

async fn eleve_absent(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<AbsentQuery>,
) -> Result<Json<AbsentDto>, (StatusCode, String)> {
    let autorise = state
        .ent
        .actor_can_know_absent(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    if !autorise {
        return Err((StatusCode::FORBIDDEN, "accès refusé".into()));
    }
    let absent = state
        .ent
        .eleve_est_absent(session.etablissement_id, id, q.jour)
        .await
        .map_err(map_db)?;
    Ok(Json(AbsentDto {
        eleve_id: id,
        jour: q.jour,
        absent,
    }))
}

#[derive(Debug, Deserialize)]
pub struct EffectifQuery {
    pub jour: NaiveDate,
}

async fn effectif_classe(
    State(state): State<AppState>,
    session: EntSession,
    Path(classe): Path<String>,
    Query(q): Query<EffectifQuery>,
) -> Result<Json<EffectifDto>, (StatusCode, String)> {
    session.require_read_effectif()?;
    let eleves = state
        .ent
        .eleves_en_classe_au(session.etablissement_id, &classe, q.jour)
        .await
        .map_err(map_db)?;
    // RP1-03 / RP1-07 : bornée aux élèves que l'acteur suit (professeur) ; vide pour la compta.
    // La comptabilité garde l'effectif (identité minimisée par `niveau_identite`) : c'est son
    // besoin légitime ; seul le professeur est borné à ses élèves.
    let suivis = if session.role == "compta" {
        None
    } else {
        state
            .ent
            .eleves_suivis_restreints(session.etablissement_id, session.user_id, &session.role)
            .await
            .map_err(map_db)?
    };
    let eleves: Vec<_> = eleves
        .into_iter()
        .filter(|e| suivis.as_ref().is_none_or(|s| s.contains(&e.id)))
        .collect();
    let vues = state
        .ent
        .vues_eleves_pour_acteur(session.etablissement_id, session.user_id, &session.role, eleves)
        .await
        .map_err(map_db)?;
    Ok(Json(EffectifDto {
        classe,
        jour: q.jour,
        eleves: vues.iter().map(EleveDto::depuis_vue).collect(),
    }))
}

async fn create_matiere(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreateMatiereBody>,
) -> Result<(StatusCode, Json<MatiereDto>), (StatusCode, String)> {
    session.require_manage_matieres()?;
    let matiere = state
        .ent
        .create_matiere(
            CreateMatiere {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                source_nomenclature: body.source_nomenclature,
            },
            Some(session.actor_user_id),
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(MatiereDto::from(&matiere))))
}

async fn update_matiere(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateMatiereBody>,
) -> Result<Json<MatiereDto>, (StatusCode, String)> {
    session.require_manage_matieres()?;
    let matiere = state
        .ent
        .update_matiere_libelle(
            session.etablissement_id,
            id,
            body.libelle,
            Some(session.actor_user_id),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(MatiereDto::from(&matiere)))
}

async fn list_matieres(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<MatiereDto>>, (StatusCode, String)> {
    // Catalogue visible au staff ; parents n'administrent pas les matières.
    if session.role_parsed().map(|r| r.is_staff()).unwrap_or(false) {
        let list = state
            .ent
            .list_matieres(session.etablissement_id)
            .await
            .map_err(map_db)?;
        Ok(Json(list.iter().map(MatiereDto::from).collect()))
    } else {
        Err((StatusCode::FORBIDDEN, "catalogue matières réservé au staff".into()))
    }
}

/// Route historique **fermée** (lot V0, socle 21) : 410 avec le chemin de remplacement.
async fn saisir_note(_session: EntSession) -> (StatusCode, String) {
    (
        StatusCode::GONE,
        "route historique fermée (V0, D1) : saisir les notes par la grille d'évaluation (PUT /evaluations/devoirs/{id}/resultats), qui contrôle le service, l'effectif, la période et l'historique".into(),
    )
}

async fn list_notes(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<NoteDto>>, (StatusCode, String)> {
    let allowed = state
        .ent
        .actor_can_read_notes(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    if !allowed {
        return Err((StatusCode::FORBIDDEN, "pas le droit de lire ces notes".into()));
    }
    let notes = state
        .ent
        .notes_for_eleve(session.etablissement_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_notes(
            &state,
            session.etablissement_id,
            notes.iter().map(NoteDto::from).collect(),
        )
        .await?,
    ))
}

#[derive(Debug, Deserialize)]
struct AppreciationsQuery {
    periode_id: Option<Uuid>,
}

async fn list_appreciations(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<AppreciationsQuery>,
) -> Result<Json<Vec<AppreciationDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_appreciations_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            q.periode_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_appreciations(
            &state,
            session.etablissement_id,
            list.iter().map(AppreciationDto::from).collect(),
        )
        .await?,
    ))
}

async fn saisir_appreciation(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<SaisirAppreciationBody>,
) -> Result<(StatusCode, Json<AppreciationDto>), (StatusCode, String)> {
    session.require_saisir_notes()?;
    let a = state
        .ent
        .saisir_appreciation_for_actor(
            &session.role,
            SaisirAppreciation {
                etablissement_id: session.etablissement_id,
                periode_id: body.periode_id,
                eleve_id: id,
                matiere_id: body.matiere_id,
                texte: body.texte,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_appreciations(
        &state,
        session.etablissement_id,
        vec![AppreciationDto::from(&a)],
    )
    .await?;
    Ok((StatusCode::CREATED, Json(enriched.remove(0))))
}

/// Route historique **fermée** (lot V0, socle 21) : 410 avec le chemin de remplacement.
async fn publier_bulletin(_session: EntSession) -> (StatusCode, String) {
    (
        StatusCode::GONE,
        "route historique fermée (V0, D5) : les bulletins se génèrent puis se publient par version (POST /evaluations/bulletins/generer puis /publier)".into(),
    )
}

#[derive(Debug, Deserialize)]
struct MoyennesQuery {
    periode_id: Uuid,
}

async fn list_moyennes(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<MoyennesQuery>,
) -> Result<Json<Vec<MoyenneMatiereDto>>, (StatusCode, String)> {
    let allowed = state
        .ent
        .actor_can_read_notes(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    if !allowed {
        return Err((StatusCode::FORBIDDEN, "pas le droit de lire ces notes".into()));
    }
    let list = state
        .ent
        .moyennes_eleve_periode(session.etablissement_id, id, q.periode_id)
        .await
        .map_err(map_db)?;
    let mut dtos: Vec<MoyenneMatiereDto> = list.iter().map(MoyenneMatiereDto::from).collect();
    if !dtos.is_empty() {
        let matieres = state
            .ent
            .list_matieres(session.etablissement_id)
            .await
            .map_err(map_db)?;
        let by_id: std::collections::HashMap<Uuid, String> =
            matieres.into_iter().map(|m| (m.id, m.libelle)).collect();
        for m in &mut dtos {
            m.matiere_libelle = by_id.get(&m.matiere_id).cloned();
        }
    }
    Ok(Json(dtos))
}

async fn list_note_periodes(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<NotePeriodeDto>>, (StatusCode, String)> {
    // Visible staff + parents/élèves (lecture calendrier scolaire notes).
    let list = state
        .ent
        .list_note_periodes(session.etablissement_id)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(NotePeriodeDto::from).collect()))
}

async fn creer_note_periode(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerNotePeriodeBody>,
) -> Result<(StatusCode, Json<NotePeriodeDto>), (StatusCode, String)> {
    session.require_manage_matieres()?;
    let periode = state
        .ent
        .creer_note_periode(CreerNotePeriode {
            etablissement_id: session.etablissement_id,
            code: body.code,
            libelle: body.libelle,
            date_debut: body.date_debut,
            date_fin: body.date_fin,
            actor_user_id: Some(session.user_id.to_string()),
        })
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(NotePeriodeDto::from(&periode))))
}

/// Route historique **fermée** (lot V0, socle 21) : 410 avec le chemin de remplacement.
async fn set_note_periode_statut(_session: EntSession) -> (StatusCode, String) {
    (
        StatusCode::GONE,
        "route historique fermée (V0, D2) : le statut d'une période se change par PUT /evaluations/periodes/{id}/statut (droit « periodes.verrouiller », motif obligatoire pour rouvrir, journal)".into(),
    )
}

#[derive(Debug, Deserialize)]
struct DevoirsQuery {
    periode_id: Uuid,
    classe: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DevoirsEleveQuery {
    periode_id: Uuid,
}

async fn list_devoirs(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<DevoirsQuery>,
) -> Result<Json<Vec<DevoirDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_devoirs_pour_acteur(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.periode_id,
            q.classe.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_devoirs(
            &state,
            session.etablissement_id,
            list.iter().map(DevoirDto::from).collect(),
        )
        .await?,
    ))
}

async fn list_devoirs_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<DevoirsEleveQuery>,
) -> Result<Json<Vec<DevoirDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_devoirs_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            q.periode_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_devoirs(
            &state,
            session.etablissement_id,
            list.iter().map(DevoirDto::from).collect(),
        )
        .await?,
    ))
}

async fn creer_devoir(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerDevoirBody>,
) -> Result<(StatusCode, Json<DevoirDto>), (StatusCode, String)> {
    session.require_saisir_notes()?;
    let devoir = state
        .ent
        .creer_devoir_for_actor(&session.role, CreerDevoir {
            etablissement_id: session.etablissement_id,
            periode_id: body.periode_id,
            matiere_id: body.matiere_id,
            classe: body.classe,
            titre: body.titre,
            date_devoir: body.date_devoir,
            coef: body.coef.unwrap_or(1.0),
            sur: body.sur.unwrap_or(20.0),
            actor_user_id: session.user_id,
        })
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(
            enrich_devoir(
                &state,
                session.etablissement_id,
                DevoirDto::from(&devoir),
            )
            .await?,
        ),
    ))
}

async fn supprimer_devoir(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    session.require_saisir_notes()?;
    state
        .ent
        .supprimer_devoir_for_actor(session.etablissement_id, &session.role, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_edt_creneaux(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<AbsentQuery>,
) -> Result<Json<Vec<EdtCreneauDto>>, (StatusCode, String)> {
    session.require_faire_appel()?;
    let list = state
        .ent
        .list_edt_creneaux_jour(session.etablissement_id, q.jour)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_edt_creneaux(&state, session.etablissement_id, list.iter().map(VersDto::vers_dto).collect()).await?,
    ))
}

#[derive(Debug, Deserialize)]
struct EdtEleveQuery {
    jour: Option<NaiveDate>,
}

async fn list_edt_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<EdtEleveQuery>,
) -> Result<Json<Vec<EdtCreneauDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_edt_eleve_jour_for_actor(
            session.etablissement_id,
            &session.role,
            session.user_id,
            id,
            q.jour.unwrap_or_else(today_utc),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_edt_creneaux(&state, session.etablissement_id, list.iter().map(VersDto::vers_dto).collect()).await?,
    ))
}

#[derive(Debug, Deserialize)]
struct EdtIcalQuery {
    debut: Option<NaiveDate>,
    fin: Option<NaiveDate>,
}

const EDT_ICAL_MAX_DAYS: i64 = 62;

fn resolve_edt_ical_range(
    debut: Option<NaiveDate>,
    fin: Option<NaiveDate>,
) -> Result<(NaiveDate, NaiveDate), (StatusCode, String)> {
    let today = today_utc();
    let (debut, fin) = match (debut, fin) {
        (None, None) => {
            let lundi = domain_eleve::lundi_iso(today);
            (lundi, lundi + chrono::Duration::days(6))
        }
        (Some(d), None) => (d, d + chrono::Duration::days(13)),
        (None, Some(f)) => (f - chrono::Duration::days(13), f),
        (Some(d), Some(f)) => (d, f),
    };
    if fin < debut {
        return Err((
            StatusCode::BAD_REQUEST,
            "fin doit être ≥ debut".into(),
        ));
    }
    let span = (fin - debut).num_days();
    if span > EDT_ICAL_MAX_DAYS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("période max {EDT_ICAL_MAX_DAYS} jours"),
        ));
    }
    Ok((debut, fin))
}

async fn edt_eleve_occurrences_for_range(
    state: &AppState,
    session: &EntSession,
    eleve_id: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
) -> Result<Vec<EdtOccurrence>, (StatusCode, String)> {
    let mut out = Vec::new();
    let mut jour = debut;
    while jour <= fin {
        let list = state
            .ent
            .list_edt_eleve_jour_for_actor(
                session.etablissement_id,
                &session.role,
                session.user_id,
                eleve_id,
                jour,
            )
            .await
            .map_err(map_db)?;
        for creneau in list {
            out.push(EdtOccurrence { date: jour, creneau });
        }
        jour += chrono::Duration::days(1);
    }
    Ok(out)
}

async fn edt_mes_occurrences_for_range(
    state: &AppState,
    session: &EntSession,
    debut: NaiveDate,
    fin: NaiveDate,
) -> Result<Vec<EdtOccurrence>, (StatusCode, String)> {
    if session.role != "eleve" {
        session.require_faire_appel()?;
    }
    let mut out = Vec::new();
    let mut jour = debut;
    while jour <= fin {
        let list = state
            .ent
            .list_mes_creneaux(
                session.etablissement_id,
                session.user_id,
                &session.role,
                jour,
            )
            .await
            .map_err(map_db)?;
        for creneau in list {
            out.push(EdtOccurrence { date: jour, creneau });
        }
        jour += chrono::Duration::days(1);
    }
    Ok(out)
}

async fn export_edt_eleve_ics(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<EdtIcalQuery>,
) -> Result<Response, (StatusCode, String)> {
    let (debut, fin) = resolve_edt_ical_range(q.debut, q.fin)?;
    let occurrences =
        edt_eleve_occurrences_for_range(&state, &session, id, debut, fin).await?;
    let cal_name = format!("EDT élève {debut} — {fin}");
    let body = render_edt_calendar(&cal_name, &occurrences);
    let filename = format!("edt-{id}-{debut}.ics");
    attachment_response(body.into_bytes(), &filename, "text/calendar; charset=utf-8")
}

async fn export_mes_creneaux_ics(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<EdtIcalQuery>,
) -> Result<Response, (StatusCode, String)> {
    let (debut, fin) = resolve_edt_ical_range(q.debut, q.fin)?;
    let occurrences = edt_mes_occurrences_for_range(&state, &session, debut, fin).await?;
    let cal_name = format!("Mon EDT {debut} — {fin}");
    let body = render_edt_calendar(&cal_name, &occurrences);
    let filename = format!("mon-edt-{debut}.ics");
    attachment_response(body.into_bytes(), &filename, "text/calendar; charset=utf-8")
}

async fn affecter_edt_creneau(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<AffecterEdtCreneauBody>,
) -> Result<Json<EdtCreneauDto>, (StatusCode, String)> {
    session.require_manage_edt()?;
    let c = state
        .ent
        .affecter_edt_creneau_for_actor(
            &session.role,
            AffecterEdtCreneau {
                etablissement_id: session.etablissement_id,
                creneau_id: id,
                prof_user_id: body.prof_user_id,
                salle: body.salle,
                actor_user_id: session.user_id,
            },
            body.salle_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_edt_creneau(&state, session.etablissement_id, c.vers_dto()).await?,
    ))
}

fn parse_hm(s: &str) -> Option<NaiveTime> {
    let t = s.trim();
    NaiveTime::parse_from_str(t, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(t, "%H:%M:%S"))
        .ok()
}

async fn list_edt_contraintes(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<EdtContrainteDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_edt_contraintes_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_edt_contraintes(&state, session.etablissement_id, list.iter().map(EdtContrainteDto::from).collect()).await?,
    ))
}

async fn creer_edt_contrainte(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerEdtContrainteBody>,
) -> Result<(StatusCode, Json<EdtContrainteDto>), (StatusCode, String)> {
    session.require_manage_edt()?;
    let kind = EdtContrainteKind::parse(&body.kind).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "kind contrainte EDT invalide".into(),
        )
    })?;
    let c = state
        .ent
        .creer_edt_contrainte_for_actor(
            &session.role,
            CreerEdtContrainte {
                etablissement_id: session.etablissement_id,
                kind,
                prof_user_id: body.prof_user_id,
                salle_code: body.salle_code,
                jour_semaine: body.jour_semaine,
                heure_debut: body.heure_debut.as_deref().and_then(parse_hm),
                heure_fin: body.heure_fin.as_deref().and_then(parse_hm),
                detail: body.detail.unwrap_or_default(),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let enriched =
        enrich_edt_contraintes(&state, session.etablissement_id, vec![EdtContrainteDto::from(&c)]).await?;
    Ok((
        StatusCode::CREATED,
        Json(
            enriched
                .into_iter()
                .next()
                .unwrap_or_else(|| EdtContrainteDto::from(&c)),
        ),
    ))
}

async fn list_mes_creneaux(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<VsAbsencesQuery>,
) -> Result<Json<Vec<EdtCreneauDto>>, (StatusCode, String)> {
    // Staff appel + élève (son EDT) — le service tranche le RBAC.
    if session.role != "eleve" {
        session.require_faire_appel()?;
    }
    let jour = q.jour.unwrap_or_else(today_utc);
    let list = state
        .ent
        .list_mes_creneaux(
            session.etablissement_id,
            session.user_id,
            &session.role,
            jour,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_edt_creneaux(&state, session.etablissement_id, list.iter().map(VersDto::vers_dto).collect()).await?,
    ))
}

async fn list_edt_exceptions(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<AbsentQuery>,
) -> Result<Json<Vec<EdtExceptionDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_edt_exceptions_jour(session.etablissement_id, &session.role, q.jour)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_edt_exceptions(
            &state,
            session.etablissement_id,
            list.iter().map(EdtExceptionDto::from).collect(),
        )
        .await?,
    ))
}

async fn poser_edt_exception(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<PoserEdtExceptionBody>,
) -> Result<(StatusCode, Json<EdtExceptionDto>), (StatusCode, String)> {
    session.require_manage_edt()?;
    let kind = EdtExceptionKind::parse(&body.kind).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "type d'exception EDT invalide".into(),
        )
    })?;
    let ex = state
        .ent
        .poser_edt_exception_for_actor(
            &session.role,
            PoserEdtException {
                etablissement_id: session.etablissement_id,
                creneau_id: body.creneau_id,
                jour: body.jour,
                kind,
                remplacant_user_id: body.remplacant_user_id,
                nouvelle_salle: body.nouvelle_salle,
                motif: body.motif,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_edt_exceptions(
        &state,
        session.etablissement_id,
        vec![EdtExceptionDto::from(&ex)],
    )
    .await?;
    Ok((StatusCode::CREATED, Json(enriched.remove(0))))
}

#[derive(Debug, Deserialize)]
struct EdtImportBody {
    csv: String,
    #[serde(default)]
    dry_run: bool,
    /// `native` (défaut) ou `pronote` (e-mail enseignant + jours FR).
    #[serde(default)]
    format: Option<String>,
}

#[derive(Debug, Serialize)]
struct EdtImportDto {
    lines: usize,
    ok_count: usize,
    conflicts: usize,
    dry_run: bool,
    format: String,
}

async fn importer_edt_csv(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<EdtImportBody>,
) -> Result<Json<EdtImportDto>, (StatusCode, String)> {
    let format = body
        .format
        .as_deref()
        .unwrap_or("native")
        .trim()
        .to_ascii_lowercase();
    let report = state
        .ent
        .importer_edt_csv_format_for_actor(
            session.etablissement_id,
            &session.role,
            session.user_id,
            &body.csv,
            body.dry_run,
            &format,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(EdtImportDto {
        lines: report.lines,
        ok_count: report.ok_count,
        conflicts: report.conflicts.len(),
        dry_run: body.dry_run,
        format,
    }))
}

#[derive(Debug, Deserialize)]
struct EdtDemandeBody {
    classe: String,
    matiere: String,
    prof_user_id: Uuid,
    #[serde(default = "default_duree_edt")]
    duree_minutes: i32,
    salle: Option<String>,
    #[serde(default)]
    semaine_ab: Option<String>,
}

fn default_duree_edt() -> i32 {
    60
}

#[derive(Debug, Deserialize)]
struct EdtResoudreBody {
    demandes: Vec<EdtDemandeBody>,
    #[serde(default)]
    dry_run: bool,
}

#[derive(Debug, Serialize)]
struct EdtResoudreDto {
    placed: usize,
    unplaced: usize,
    dry_run: bool,
}

async fn resoudre_edt(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<EdtResoudreBody>,
) -> Result<Json<EdtResoudreDto>, (StatusCode, String)> {
    use domain_eleve::{EdtDemande, EdtSemaineAb};
    let demandes: Vec<EdtDemande> = body
        .demandes
        .into_iter()
        .map(|d| EdtDemande {
            classe: d.classe,
            matiere_libelle: d.matiere,
            prof_user_id: d.prof_user_id,
            duree_minutes: d.duree_minutes,
            salle: d.salle,
            semaine_ab: d
                .semaine_ab
                .as_deref()
                .and_then(EdtSemaineAb::parse)
                .unwrap_or(EdtSemaineAb::Toutes),
        })
        .collect();
    let report = state
        .ent
        .resoudre_edt_for_actor(
            session.etablissement_id,
            &session.role,
            session.user_id,
            demandes,
            body.dry_run,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(EdtResoudreDto {
        placed: report.placed.len(),
        unplaced: report.unplaced.len(),
        dry_run: body.dry_run,
    }))
}

#[derive(Debug, Deserialize)]
struct CahierTexteQuery {
    classe: Option<String>,
    eleve_id: Option<Uuid>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

async fn list_cahier_texte(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<CahierTexteQuery>,
) -> Result<Json<Vec<CahierTexteDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_cahier_texte_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.classe,
            q.eleve_id,
            q.from,
            q.to,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(CahierTexteDto::from).collect()))
}

async fn creer_cahier_texte(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerCahierTexteBody>,
) -> Result<(StatusCode, Json<CahierTexteDto>), (StatusCode, String)> {
    let entree = state
        .ent
        .creer_cahier_texte_for_actor(
            &session.role,
            CreerCahierTexte {
                etablissement_id: session.etablissement_id,
                classe: body.classe,
                matiere_libelle: body.matiere_libelle,
                titre: body.titre,
                consigne: body.consigne,
                pour_le: body.pour_le,
                cree_le: body.cree_le.unwrap_or_else(today_utc),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(CahierTexteDto::from(&entree))))
}

async fn get_edt_calendrier_ab(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Option<EdtCalendrierAbDto>>, (StatusCode, String)> {
    let cal = state
        .ent
        .get_edt_calendrier_ab(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(cal.as_ref().map(EdtCalendrierAbDto::from)))
}

async fn configurer_edt_calendrier_ab(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<ConfigurerEdtCalendrierAbBody>,
) -> Result<Json<EdtCalendrierAbDto>, (StatusCode, String)> {
    session.require_manage_edt()?;
    let cal = state
        .ent
        .configurer_edt_calendrier_ab_for_actor(
            &session.role,
            ConfigurerEdtCalendrierAb {
                etablissement_id: session.etablissement_id,
                reference_lundi: body.reference_lundi,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(EdtCalendrierAbDto::from(&cal)))
}

async fn ouvrir_appel(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<OuvrirAppelBody>,
) -> Result<(StatusCode, Json<AppelDto>), (StatusCode, String)> {
    session.require_faire_appel()?;
    let (appel, lignes) = state
        .ent
        .ouvrir_appel(
            session.etablissement_id,
            body.creneau_id,
            body.jour,
            session.user_id,
            &session.role,
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(enrich_appel(&state, session.etablissement_id, appel_dto(&appel, &lignes)).await?),
    ))
}

async fn get_appel(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<AppelDto>, (StatusCode, String)> {
    session.require_faire_appel()?;
    let (appel, lignes) = state
        .ent
        .get_appel(session.etablissement_id, id, session.user_id, &session.role)
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "appel introuvable".into()))?;
    Ok(Json(
        enrich_appel(&state, session.etablissement_id, appel_dto(&appel, &lignes)).await?,
    ))
}

async fn marquer_appel_ligne(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<MarquerAppelLigneBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    session.require_faire_appel()?;
    let etat = LigneEtat::parse(&body.etat).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "état invalide (present|absent|retard)".into(),
        )
    })?;
    state
        .ent
        .marquer_appel_ligne(
            session.etablissement_id,
            id,
            session.user_id,
            &session.role,
            body.eleve_id,
            etat,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn clore_appel(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<AppelDto>, (StatusCode, String)> {
    session.require_faire_appel()?;
    let appel = state
        .ent
        .clore_appel(
            session.etablissement_id,
            id,
            session.user_id,
            &session.role,
            Some(session.actor_user_id),
        )
        .await
        .map_err(map_db)?;
    let lignes = state
        .ent
        .get_appel(session.etablissement_id, id, session.user_id, &session.role)
        .await
        .map_err(map_db)?
        .map(|(_, l)| l)
        .unwrap_or_default();
    Ok(Json(
        enrich_appel(&state, session.etablissement_id, appel_dto(&appel, &lignes)).await?,
    ))
}

async fn list_carnet(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<CarnetEntreeDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_carnet_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_carnet(
            &state,
            session.etablissement_id,
            list.iter().map(CarnetEntreeDto::from).collect(),
        )
        .await?,
    ))
}

async fn creer_carnet(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<CreerCarnetBody>,
) -> Result<(StatusCode, Json<CarnetEntreeDto>), (StatusCode, String)> {
    // M0a (D9) : droit effectif « suivi.eleve » ; le contrôle par élève est dans le service.
    session
        .require_action(&state.ent, ent_auth::Action::SuiviEleve, "droit insuffisant pour écrire au carnet")
        .await?;
    let categorie = CarnetCategorie::parse(&body.categorie).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "catégorie invalide (correspondance|accompagnement|information)".into(),
        )
    })?;
    let entree = state
        .ent
        .creer_carnet_entree_for_actor(
            &session.role,
            CreerCarnetEntree {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                date_entree: body.date_entree,
                categorie,
                titre: body.titre,
                corps: body.corps,
                visible_famille: body.visible_famille,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_carnet(
        &state,
        session.etablissement_id,
        vec![CarnetEntreeDto::from(&entree)],
    )
    .await?;
    Ok((StatusCode::CREATED, Json(enriched.remove(0))))
}

#[derive(Debug, Deserialize)]
struct FilsQuery {
    eleve_id: Option<Uuid>,
    /// Staff boîte globale : inclure les fils archivés (défaut false).
    #[serde(default)]
    archives: bool,
    /// Staff : filtre dossier (ex. `vs`).
    dossier: Option<String>,
    /// Staff (M0b, D12) : recherche sur le sujet, côté serveur.
    q: Option<String>,
    /// Staff : curseur de la page suivante (en-tête `x-curseur-suivant` de la page précédente).
    apres: Option<String>,
    /// Staff : taille de page, 1 à 200 (50 par défaut).
    limite: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ModelesQuery {
    /// Inclure les modèles inactifs (défaut : actifs seulement).
    #[serde(default)]
    tous: bool,
}

#[derive(Debug, Deserialize)]
struct InfirmeriePassagesQuery {
    eleve_id: Option<Uuid>,
    jour: Option<NaiveDate>,
}

async fn list_passages_infirmerie(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<InfirmeriePassagesQuery>,
) -> Result<Json<Vec<PassageInfirmerieDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_passages_infirmerie_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.eleve_id,
            q.jour,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_passages_infirmerie(
            &state,
            session.etablissement_id,
            list.iter().map(PassageInfirmerieDto::from).collect(),
        )
        .await?,
    ))
}

async fn creer_passage_infirmerie(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerPassageInfirmerieBody>,
) -> Result<(StatusCode, Json<PassageInfirmerieDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanteSaisir, "rôle insuffisant pour l'infirmerie")
        .await?;
    let motif = PassageInfirmerieMotif::parse(&body.motif).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "motif infirmière invalide".into(),
        )
    })?;
    let passage = state
        .ent
        .creer_passage_infirmerie_for_actor(
            &session.role,
            CreerPassageInfirmerie {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                jour: body.jour,
                motif,
                note_soins: body.note_soins,
                visible_famille: body.visible_famille.unwrap_or(false),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(PassageInfirmerieDto::from(&passage)),
    ))
}

async fn get_fiche_sante(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Option<FicheSanteDto>>, (StatusCode, String)> {
    let fiche = state
        .ent
        .get_fiche_sante_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(fiche.as_ref().map(FicheSanteDto::from)))
}

async fn pdf_resume_fiche_sante(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, String)> {
    let bytes = state
        .ent
        .resume_fiche_sante_pdf_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    pdf_response(bytes, &format!("fiche-sante-{id}.pdf"))
}

async fn get_pai(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Option<PaiDto>>, (StatusCode, String)> {
    let pai = state
        .ent
        .get_pai_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(pai.as_ref().map(PaiDto::from)))
}

async fn upsert_pai(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<UpsertPaiBody>,
) -> Result<Json<PaiDto>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::SanteSaisir, "rôle insuffisant pour l'infirmerie")
        .await?;
    let pai = state
        .ent
        .upsert_pai_for_actor(
            &session.role,
            UpsertPai {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                motif: body.motif,
                mesures_accueil: body.mesures_accueil,
                protocole_urgence: body.protocole_urgence,
                note_interne: body.note_interne.unwrap_or_default(),
                date_debut: body.date_debut,
                date_fin: body.date_fin,
                actif: body.actif.unwrap_or(true),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(PaiDto::from(&pai)))
}

async fn signer_pai_famille(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<PaiDto>, (StatusCode, String)> {
    let pai = state
        .ent
        .signer_pai_famille_for_actor(
            &session.role,
            SignerPaiFamille {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(PaiDto::from(&pai)))
}

async fn list_pai_pieces(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<PaiPieceDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_pai_pieces_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(PaiPieceDto::from).collect()))
}

async fn attacher_ordonnance_pai(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<AttacherPieceBody>,
) -> Result<(StatusCode, Json<PaiPieceDto>), (StatusCode, String)> {
    // Infirmerie ou parent lié — contrôle métier dans attacher_ordonnance_pai_for_actor.
    let contenu = base64::engine::general_purpose::STANDARD
        .decode(body.contenu_base64.trim())
        .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".into()))?;
    let piece = state
        .ent
        .attacher_ordonnance_pai_for_actor(
            &session.role,
            AttacherOrdonnancePai {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                nom: body.nom,
                mime_type: body.mime_type,
                contenu,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(PaiPieceDto::from(&piece))))
}

async fn telecharger_pai_piece(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, String)> {
    let piece = state
        .ent
        .telecharger_pai_piece_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    attachment_response_dyn(piece.contenu, &piece.nom, &piece.mime_type)
}

async fn upsert_fiche_sante(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<UpsertFicheSanteBody>,
) -> Result<Json<FicheSanteDto>, (StatusCode, String)> {
    // Infirmerie staff OU parent lié (champs famille) — RBAC dans le service.
    let fiche = state
        .ent
        .upsert_fiche_sante_for_actor(
            &session.role,
            UpsertFicheSante {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                allergies: body.allergies,
                traitements: body.traitements,
                contre_indications: body.contre_indications,
                pai_actif: body.pai_actif,
                medecin_nom: body.medecin_nom,
                urgence_tel: body.urgence_tel,
                note_confidentielle: body.note_confidentielle,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(FicheSanteDto::from(&fiche)))
}

#[derive(Debug, Deserialize)]
struct PsyEntretiensQuery {
    eleve_id: Option<Uuid>,
    jour: Option<NaiveDate>,
}

async fn list_entretiens_psy(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<PsyEntretiensQuery>,
) -> Result<Json<Vec<EntretienPsyDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_entretiens_psy_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.eleve_id,
            q.jour,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_entretiens_psy(
            &state,
            session.etablissement_id,
            list.iter().map(EntretienPsyDto::from).collect(),
        )
        .await?,
    ))
}

async fn creer_entretien_psy(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerEntretienPsyBody>,
) -> Result<(StatusCode, Json<EntretienPsyDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::PsySaisir, "rôle insuffisant pour la psychologie scolaire")
        .await?;
    let kind = EntretienPsyKind::parse(&body.kind).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "type d'entretien psy invalide".into(),
        )
    })?;
    let entretien = state
        .ent
        .creer_entretien_psy_for_actor(
            &session.role,
            CreerEntretienPsy {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                jour: body.jour,
                kind,
                duree_minutes: body.duree_minutes,
                note_confidentielle: body.note_confidentielle,
                signale_famille: body.signale_famille.unwrap_or(false),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_entretiens_psy(
        &state,
        session.etablissement_id,
        vec![EntretienPsyDto::from(&entretien)],
    )
    .await?;
    Ok((StatusCode::CREATED, Json(enriched.remove(0))))
}

fn parse_heure_hm(s: &str) -> Option<NaiveTime> {
    let t = s.trim();
    NaiveTime::parse_from_str(t, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(t, "%H:%M"))
        .ok()
}

async fn list_rdvs_psy(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<PsyEntretiensQuery>,
) -> Result<Json<Vec<RdvPsyDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_rdvs_psy_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.eleve_id,
            q.jour,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_rdvs_psy(
            &state,
            session.etablissement_id,
            list.iter().map(RdvPsyDto::from).collect(),
        )
        .await?,
    ))
}

async fn creer_rdv_psy(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerRdvPsyBody>,
) -> Result<(StatusCode, Json<RdvPsyDto>), (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::PsySaisir, "rôle insuffisant pour la psychologie scolaire")
        .await?;
    let kind = EntretienPsyKind::parse(&body.kind).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "type de RDV psy invalide".into(),
        )
    })?;
    let heure_debut = parse_heure_hm(&body.heure_debut).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "heure_debut invalide (HH:MM)".into(),
        )
    })?;
    let heure_fin = parse_heure_hm(&body.heure_fin).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "heure_fin invalide (HH:MM)".into(),
        )
    })?;
    let rdv = state
        .ent
        .creer_rdv_psy_for_actor(
            &session.role,
            CreerRdvPsy {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                jour: body.jour,
                heure_debut,
                heure_fin,
                kind,
                motif: body.motif,
                note_interne: body.note_interne.unwrap_or_default(),
                signale_famille: body.signale_famille.unwrap_or(false),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_rdvs_psy(
        &state,
        session.etablissement_id,
        vec![RdvPsyDto::from(&rdv)],
    )
    .await?;
    Ok((StatusCode::CREATED, Json(enriched.remove(0))))
}

async fn changer_statut_rdv_psy(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ChangerStatutRdvPsyBody>,
) -> Result<Json<RdvPsyDto>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::PsySaisir, "rôle insuffisant pour la psychologie scolaire")
        .await?;
    let statut = RdvPsyStatut::parse(&body.statut).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "statut RDV psy invalide".into(),
        )
    })?;
    let rdv = state
        .ent
        .changer_statut_rdv_psy_for_actor(
            &session.role,
            ChangerStatutRdvPsy {
                etablissement_id: session.etablissement_id,
                rdv_id: id,
                statut,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_rdvs_psy(
        &state,
        session.etablissement_id,
        vec![RdvPsyDto::from(&rdv)],
    )
    .await?;
    Ok(Json(enriched.remove(0)))
}

async fn get_dossier_psy(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
) -> Result<Json<Option<DossierPsyDto>>, (StatusCode, String)> {
    let dossier = state
        .ent
        .get_dossier_psy_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            eleve_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(dossier.as_ref().map(DossierPsyDto::from)))
}

async fn upsert_dossier_psy(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
    Json(body): Json<UpsertDossierPsyBody>,
) -> Result<Json<DossierPsyDto>, (StatusCode, String)> {
    session
        .require_action(&state.ent, ent_auth::Action::PsySaisir, "rôle insuffisant pour la psychologie scolaire")
        .await?;
    let dossier = state
        .ent
        .upsert_dossier_psy_for_actor(
            &session.role,
            UpsertDossierPsy {
                etablissement_id: session.etablissement_id,
                eleve_id,
                axes_suivi: body.axes_suivi,
                contacts_externes: body.contacts_externes,
                note_confidentielle: body.note_confidentielle,
                suivi_actif: body.suivi_actif,
                signale_famille: body.signale_famille.unwrap_or(false),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(DossierPsyDto::from(&dossier)))
}

async fn pdf_courrier_orientation_psy(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
) -> Result<Response, (StatusCode, String)> {
    let bytes = state
        .ent
        .courrier_orientation_psy_pdf_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            eleve_id,
        )
        .await
        .map_err(map_db)?;
    pdf_response(bytes, &format!("courrier-orientation-{eleve_id}.pdf"))
}

async fn envoyer_courrier_orientation_psy(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let n = state
        .ent
        .envoyer_courrier_orientation_psy_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            eleve_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(serde_json::json!({ "mails": n })))
}

async fn list_fils(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<FilsQuery>,
) -> Result<(HeaderMap, Json<Vec<MsgFilDto>>), (StatusCode, String)> {
    let page = state
        .ent
        .list_fils_page_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            ent_db::ParamsListeFils {
                eleve_id: q.eleve_id,
                include_archives: q.archives,
                dossier: q.dossier,
                recherche: q.q,
                apres: q.apres,
                limite: q.limite,
            },
        )
        .await
        .map_err(map_db)?;
    let mut headers = HeaderMap::new();
    if let Some(suivant) = page.suivant.as_deref() {
        if let Ok(v) = HeaderValue::from_str(suivant) {
            headers.insert("x-curseur-suivant", v);
        }
    }
    Ok((
        headers,
        Json(
            enrich_msg_fils(
                &state,
                session.etablissement_id,
                session.user_id,
                page.fils.iter().map(MsgFilDto::from).collect(),
            )
            .await?,
        ),
    ))
}

async fn ouvrir_fil(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<OuvrirFilBody>,
) -> Result<(StatusCode, Json<FilDetailDto>), (StatusCode, String)> {
    let mut ouverts = state
        .ent
        .ouvrir_fils_for_actor(
            &session.role,
            OuvrirFil {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                sujet: body.sujet,
                premier_message: body.message,
                actor_user_id: session.user_id,
            },
            ent_db::OptionsEnvoiMessage { urgence_motif: body.urgence_motif },
        )
        .await
        .map_err(map_db)?;
    let premier = ouverts.remove(0);
    Ok((
        StatusCode::CREATED,
        Json(FilDetailDto {
            fil: MsgFilDto::from(&premier.fil),
            messages: enrich_messages(
                &state,
                session.etablissement_id,
                vec![MessageDto::from(&premier.message)],
            )
            .await?,
            remise_prevue_le: premier.remise_prevue_le.map(|d| d.to_rfc3339()),
            autres_fils: ouverts.iter().map(|o| o.fil.id).collect(),
        }),
    ))
}

async fn ouvrir_fil_depuis_modele(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<OuvrirFilDepuisModeleBody>,
) -> Result<(StatusCode, Json<FilDetailDto>), (StatusCode, String)> {
    let (fil, msg) = state
        .ent
        .ouvrir_fil_depuis_modele_for_actor(
            &session.role,
            session.etablissement_id,
            session.user_id,
            body.modele_id,
            body.eleve_id,
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(FilDetailDto {
            fil: MsgFilDto::from(&fil),
            messages: enrich_messages(&state, session.etablissement_id, vec![MessageDto::from(&msg)]).await?,
            remise_prevue_le: None,
            autres_fils: Vec::new(),
        }),
    ))
}

async fn list_msg_modeles(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ModelesQuery>,
) -> Result<Json<Vec<MsgModeleDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_msg_modeles_for_actor(session.etablissement_id, session.user_id, &session.role, !q.tous)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(MsgModeleDto::from).collect()))
}

async fn creer_msg_modele(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerMsgModeleBody>,
) -> Result<(StatusCode, Json<MsgModeleDto>), (StatusCode, String)> {
    let modele = state
        .ent
        .creer_msg_modele_for_actor(
            &session.role,
            CreerMsgModele {
                etablissement_id: session.etablissement_id,
                titre: body.titre,
                sujet: body.sujet,
                corps: body.corps,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(MsgModeleDto::from(&modele))))
}

async fn archiver_fil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<MsgFilDto>, (StatusCode, String)> {
    let fil = state
        .ent
        .archiver_fil_for_actor(
            &session.role,
            ArchiverFil {
                etablissement_id: session.etablissement_id,
                fil_id: id,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(MsgFilDto::from(&fil)))
}

async fn classer_fil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ClasserFilBody>,
) -> Result<Json<MsgFilDto>, (StatusCode, String)> {
    let priorite = MsgPriorite::parse(&body.priorite).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "priorité invalide (normale|haute)".into(),
        )
    })?;
    let fil = state
        .ent
        .classer_fil_for_actor(
            &session.role,
            ClasserFil {
                etablissement_id: session.etablissement_id,
                fil_id: id,
                priorite,
                dossier: body.dossier,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(MsgFilDto::from(&fil)))
}

async fn get_fil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<FilDetailDto>, (StatusCode, String)> {
    let (fil, msgs) = state
        .ent
        .list_messages_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(FilDetailDto {
        fil: MsgFilDto::from(&fil),
        messages: enrich_messages(&state, session.etablissement_id, msgs.iter().map(MessageDto::from).collect()).await?,
        remise_prevue_le: None,
        autres_fils: Vec::new(),
    }))
}

async fn repondre_fil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<RepondreFilBody>,
) -> Result<(StatusCode, Json<MessageEnvoyeDto>), (StatusCode, String)> {
    let envoye = state
        .ent
        .repondre_fil_avec_options(
            &session.role,
            RepondreFil {
                etablissement_id: session.etablissement_id,
                fil_id: id,
                corps: body.corps,
                actor_user_id: session.user_id,
            },
            ent_db::OptionsEnvoiMessage { urgence_motif: body.urgence_motif },
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(MessageEnvoyeDto {
            message: enrich_message(
                &state,
                session.etablissement_id,
                MessageDto::from(&envoye.message),
            )
            .await?,
            remise_prevue_le: envoye.remise_prevue_le.map(|d| d.to_rfc3339()),
        }),
    ))
}

/// Rouvre un fil archivé (M0b, D11).
async fn desarchiver_fil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<MsgFilDto>, (StatusCode, String)> {
    let fil = state
        .ent
        .desarchiver_fil_for_actor(
            &session.role,
            ArchiverFil {
                etablissement_id: session.etablissement_id,
                fil_id: id,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(MsgFilDto::from(&fil)))
}

/// Annule un message différé par la plage calme, avant sa remise (R-M48).
async fn annuler_message_differe(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .ent
        .annuler_message_differe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct NonTraitesQuery {
    limite: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
struct FilNonTraiteDto {
    fil: MsgFilDto,
    en_attente_depuis: String,
}

/// File « messages des familles non traités » (MSG-329).
async fn list_fils_non_traites(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<NonTraitesQuery>,
) -> Result<Json<Vec<FilNonTraiteDto>>, (StatusCode, String)> {
    let liste = state
        .ent
        .list_fils_non_traites_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.limite.unwrap_or(100),
        )
        .await
        .map_err(map_db)?;
    let fils = enrich_msg_fils(
        &state,
        session.etablissement_id,
        session.user_id,
        liste.iter().map(|n| MsgFilDto::from(&n.fil)).collect(),
    )
    .await?;
    Ok(Json(
        fils.into_iter()
            .zip(liste.iter())
            .map(|(fil, n)| FilNonTraiteDto {
                fil,
                en_attente_depuis: n.en_attente_depuis.to_rfc3339(),
            })
            .collect(),
    ))
}

/// Réglage de la plage calme (D24).
async fn get_reglage_plage_calme(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<ent_db::ReglagePlageCalme>, (StatusCode, String)> {
    Ok(Json(
        state
            .ent
            .reglage_plage_calme(session.etablissement_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn put_reglage_plage_calme(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<ReglagePlageCalmeBody>,
) -> Result<Json<ent_db::ReglagePlageCalme>, (StatusCode, String)> {
    Ok(Json(
        state
            .ent
            .definir_reglage_plage_calme_for_actor(
                session.etablissement_id,
                session.user_id,
                &session.role,
                ent_db::ReglagePlageCalmeSaisie {
                    plage_calme: body.plage_calme,
                    debut_heure: body.debut_heure,
                    fin_heure: body.fin_heure,
                    weekend: body.weekend,
                    motif: body.motif,
                },
            )
            .await
            .map_err(map_db)?,
    ))
}

/// Liens créés après une interdiction de communication par libellé (D25).
async fn list_liens_a_verifier(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<ent_db::LienAVerifier>>, (StatusCode, String)> {
    Ok(Json(
        state
            .ent
            .liens_a_verifier_for_actor(session.etablissement_id, session.user_id, &session.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn verifier_lien_messagerie(
    State(state): State<AppState>,
    session: EntSession,
    Path(lien_id): Path<Uuid>,
    Json(body): Json<VerifierLienBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .ent
        .verifier_lien_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            lien_id,
            body.interdiction_id,
            body.confirmer,
            Some(&body.motif),
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn attacher_piece(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<AttacherPieceBody>,
) -> Result<(StatusCode, Json<MsgPieceDto>), (StatusCode, String)> {
    let contenu = base64::engine::general_purpose::STANDARD
        .decode(body.contenu_base64.trim())
        .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".into()))?;
    let meta = state
        .ent
        .attacher_piece_for_actor(
            &session.role,
            AttacherPiece {
                etablissement_id: session.etablissement_id,
                message_id: id,
                nom: body.nom,
                mime_type: body.mime_type,
                contenu,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(MsgPieceDto::from(&meta))))
}

async fn telecharger_piece(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, String)> {
    let piece = state
        .ent
        .telecharger_piece_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "telechargement", "piece_message", &format!("Téléchargement de la pièce jointe {id}")).await;
    attachment_response_dyn(piece.contenu, &piece.nom, &piece.mime_type)
}

async fn get_campagne(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Option<CampagneDto>>, (StatusCode, String)> {
    let jour = today_utc();
    let (_, _, np1) = annees_continuum(jour);
    let camp = state
        .ent
        .get_campagne_reinscription(session.etablissement_id, &np1)
        .await
        .map_err(map_db)?;
    Ok(Json(camp.as_ref().map(CampagneDto::from)))
}

async fn ouvrir_campagne(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<OuvrirCampagneBody>,
) -> Result<(StatusCode, Json<CampagneDto>), (StatusCode, String)> {
    if !ent_db::PgEnt::role_can_manage_continuum(&session.role) {
        return Err((StatusCode::FORBIDDEN, "campagne réservée admin/direction".into()));
    }
    let camp = state
        .ent
        .ouvrir_campagne_reinscription_for_actor(
            &session.role,
            OuvrirCampagneReinscription {
                etablissement_id: session.etablissement_id,
                annee_cible: body.annee_cible,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(CampagneDto::from(&camp))))
}

async fn configurer_acompte_campagne(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<ConfigurerAcompteBody>,
) -> Result<Json<CampagneDto>, (StatusCode, String)> {
    if !ent_db::PgEnt::role_can_manage_continuum(&session.role) {
        return Err((StatusCode::FORBIDDEN, "acompte réservé admin".into()));
    }
    let camp = state
        .ent
        .configurer_acompte_campagne_for_actor(
            &session.role,
            ConfigurerAcompteCampagne {
                etablissement_id: session.etablissement_id,
                annee_cible: body.annee_cible,
                acompte_centimes: body.acompte_centimes,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(CampagneDto::from(&camp)))
}

async fn montee_rapide(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<MonteeRapideBody>,
) -> Result<(StatusCode, Json<ScolariteAnneeDto>), (StatusCode, String)> {
    if !ent_db::PgEnt::role_can_manage_continuum(&session.role) {
        return Err((StatusCode::FORBIDDEN, "montée rapide réservée admin".into()));
    }
    let jour = today_utc();
    let annee_n = body
        .annee_courante
        .unwrap_or_else(|| annee_scolaire_de(jour));
    let s = state
        .ent
        .montee_rapide_for_actor(
            &session.role,
            MonteeRapide {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                annee_courante: annee_n,
                actor_user_id: Some(session.user_id.to_string()),
            },
            jour,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(ScolariteAnneeDto::from(&s))))
}

#[derive(Debug, Deserialize)]
struct CandidatsMonteeQuery {
    niveau: String,
    annee_courante: Option<String>,
}

async fn candidats_montee_lot(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<CandidatsMonteeQuery>,
) -> Result<Json<CandidatsMonteeLotDto>, (StatusCode, String)> {
    if !ent_db::PgEnt::role_can_manage_continuum(&session.role) {
        return Err((StatusCode::FORBIDDEN, "aperçu lot réservé admin".into()));
    }
    let jour = today_utc();
    let annee_n = q
        .annee_courante
        .unwrap_or_else(|| annee_scolaire_de(jour));
    let (niveau_source, niveau_cible, candidats) = state
        .ent
        .candidats_montee_lot(session.etablissement_id, &annee_n, &q.niveau, jour)
        .await
        .map_err(map_db)?;
    let dtos: Vec<ScolariteAnneeDto> = candidats.iter().map(ScolariteAnneeDto::from).collect();
    let count = dtos.len();
    Ok(Json(CandidatsMonteeLotDto {
        niveau_source,
        niveau_cible,
        annee_courante: annee_n,
        candidats: dtos,
        count,
    }))
}

async fn montee_lot(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<MonteeLotBody>,
) -> Result<(StatusCode, Json<LotMonteeResultatDto>), (StatusCode, String)> {
    if !ent_db::PgEnt::role_can_manage_continuum(&session.role) {
        return Err((StatusCode::FORBIDDEN, "lot montée réservé admin".into()));
    }
    let jour = today_utc();
    let annee_n = body
        .annee_courante
        .unwrap_or_else(|| annee_scolaire_de(jour));
    let r = state
        .ent
        .montee_lot_for_actor(
            &session.role,
            MonteeLot {
                etablissement_id: session.etablissement_id,
                annee_courante: annee_n,
                niveau_source: body.niveau_source,
                actor_user_id: Some(session.user_id.to_string()),
            },
            jour,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(LotMonteeResultatDto::from(&r))))
}

#[derive(Debug, Deserialize)]
struct ExigencesQuery {
    annee_cible: Option<String>,
}

async fn list_exigences_doc(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ExigencesQuery>,
) -> Result<Json<Vec<ExigenceDocDto>>, (StatusCode, String)> {
    if !ent_db::PgEnt::role_can_manage_continuum(&session.role) {
        return Err((StatusCode::FORBIDDEN, "exigences réservées admin".into()));
    }
    let jour = today_utc();
    let (_, _, annee_n1) = annees_continuum(jour);
    let annee = q.annee_cible.unwrap_or(annee_n1);
    let list = state
        .ent
        .list_exigences_doc(session.etablissement_id, &annee)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(ExigenceDocDto::from).collect()))
}

async fn poser_exigence_doc(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<PoserExigenceDocBody>,
) -> Result<(StatusCode, Json<ExigenceDocDto>), (StatusCode, String)> {
    if !ent_db::PgEnt::role_can_manage_continuum(&session.role) {
        return Err((StatusCode::FORBIDDEN, "exigences réservées admin".into()));
    }
    let tiroir = DocTiroir::parse(body.tiroir.as_deref().unwrap_or("famille")).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "tiroir invalide".into(),
        )
    })?;
    let ex = state
        .ent
        .poser_exigence_doc_for_actor(
            &session.role,
            PoserExigenceDoc {
                etablissement_id: session.etablissement_id,
                annee_cible: body.annee_cible,
                code: body.code,
                libelle: body.libelle,
                tiroir,
                obligatoire: body.obligatoire,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(ExigenceDocDto::from(&ex))))
}

async fn famille_reinscription(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<ReinscriptionFamilleDto>, (StatusCode, String)> {
    let enfants = state
        .ent
        .mes_enfants_for_parent(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    let jour = today_utc();
    let (_, annee_n, annee_n1) = annees_continuum(jour);
    let campagne = state
        .ent
        .get_campagne_reinscription(session.etablissement_id, &annee_n1)
        .await
        .map_err(map_db)?;
    let exigences = state
        .ent
        .list_exigences_doc(session.etablissement_id, &annee_n1)
        .await
        .map_err(map_db)?;
    let mut out = Vec::new();
    for e in &enfants {
        let projection = state
            .ent
            .scolarite_eleve_annee(session.etablissement_id, e.eleve.id, &annee_n1)
            .await
            .map_err(map_db)?;
        let options = state
            .ent
            .options_continuum_eleve(session.etablissement_id, e.eleve.id, &annee_n1)
            .await
            .map_err(map_db)?;
        let confirmee = projection
            .as_ref()
            .map(|p| p.statut == ScolariteAnneeStatut::Reinscription)
            .unwrap_or(false);
        let (_ex, manquants) = state
            .ent
            .vannes_docs_eleve(session.etablissement_id, e.eleve.id, &annee_n1)
            .await
            .map_err(map_db)?;
        let acompte_requis = campagne
            .as_ref()
            .map(|c| c.acompte_centimes)
            .unwrap_or(0);
        let foyer_id = state
            .ent
            .foyer_principal_eleve(session.etablissement_id, e.eleve.id)
            .await
            .map_err(map_db)?;
        let acompte_encaisse = if let Some(fid) = foyer_id {
            state
                .ent
                .encaisse_acompte_reinscription(session.etablissement_id, fid, &annee_n1)
                .await
                .map_err(map_db)?
        } else {
            0
        };
        let acompte_ok = acompte_requis <= 0 || acompte_encaisse >= acompte_requis;
        out.push(ReinscriptionEnfantDto {
            enfant: EnfantFamilleDto::from(e),
            projection_n1: projection.as_ref().map(ScolariteAnneeDto::from),
            options: options.iter().map(OptionAnneeDto::from).collect(),
            confirmee,
            documents_manquants: manquants,
            exigences: exigences.iter().map(ExigenceDocDto::from).collect(),
            acompte_requis_centimes: acompte_requis,
            acompte_encaisse_centimes: acompte_encaisse,
            acompte_ok,
        });
    }
    Ok(Json(ReinscriptionFamilleDto {
        campagne: campagne.as_ref().map(CampagneDto::from),
        annee_n,
        annee_n1,
        exigences: exigences.iter().map(ExigenceDocDto::from).collect(),
        enfants: out,
    }))
}

async fn famille_deposer_doc_reinscription(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeposerDocReinscriptionBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let jour = today_utc();
    let (_, _, annee_n1) = annees_continuum(jour);
    let contenu = if let Some(b64) = body.contenu_base64.as_ref() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".into()))?;
        Some(bytes)
    } else {
        None
    };
    let doc = state
        .ent
        .deposer_doc_reinscription_for_parent(
            session.etablissement_id,
            session.user_id,
            id,
            &annee_n1,
            &body.code,
            body.titre.as_deref().unwrap_or(""),
            body.mime_type.as_deref(),
            contenu.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": doc.id,
            "code": doc.code,
            "titre": doc.titre,
            "tiroir": doc.tiroir.as_str(),
        })),
    ))
}

async fn famille_retenir_option(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ChoisirOptionBody>,
) -> Result<Json<OptionAnneeDto>, (StatusCode, String)> {
    let jour = today_utc();
    let (_, _, annee_n1) = annees_continuum(jour);
    let nature = OptionNature::parse(body.nature.as_deref().unwrap_or("academique")).ok_or_else(
        || {
            (
                StatusCode::BAD_REQUEST,
                "nature option invalide (academique|interne)".into(),
            )
        },
    )?;
    let opt = state
        .ent
        .choisir_option_n1_for_parent(
            session.etablissement_id,
            session.user_id,
            ChoisirOptionN1 {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                annee_scolaire: annee_n1,
                nature,
                code: body.code,
                libelle: body.libelle,
                retenir: body.retenir,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(OptionAnneeDto::from(&opt)))
}

async fn famille_confirmer_reinscription(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<ScolariteAnneeDto>, (StatusCode, String)> {
    let jour = today_utc();
    let (_, _, annee_n1) = annees_continuum(jour);
    let s = state
        .ent
        .confirmer_reinscription_for_parent(
            session.etablissement_id,
            session.user_id,
            ConfirmerReinscription {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                annee_scolaire: annee_n1,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(ScolariteAnneeDto::from(&s)))
}

async fn list_mes_enfants(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<EnfantFamilleDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .mes_enfants_for_parent(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(EnfantFamilleDto::from).collect()))
}

async fn famille_resume_enfant(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<FamilleResumeDto>, (StatusCode, String)> {
    // Parent lié uniquement — get_eleve_for_actor filtre.
    state
        .ent
        .get_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "enfant introuvable".into()))?;

    let lien = state
        .ent
        .lien_parent_eleve(session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::FORBIDDEN, "pas de lien famille".into()))?;

    let absences = if lien.peut_justifier_absence || lien.peut_voir_vs {
        state
            .ent
            .absences_a_justifier_eleve(session.etablissement_id, id)
            .await
            .map_err(map_db)?
    } else {
        Vec::new()
    };

    let carnet = if lien.peut_voir_vs {
        state
            .ent
            .carnet_a_signer_eleve(session.etablissement_id, id)
            .await
            .map_err(map_db)?
    } else {
        Vec::new()
    };

    let notes = if lien.peut_voir_notes {
        state
            .ent
            .notes_for_eleve(session.etablissement_id, id)
            .await
            .map_err(map_db)?
    } else {
        Vec::new()
    };

    Ok(Json(FamilleResumeDto {
        eleve_id: id,
        absences_a_justifier: enrich_absences_eleve(
            &state,
            session.etablissement_id,
            absences.iter().map(AbsenceDto::from).collect(),
        )
        .await?,
        carnet_a_signer: enrich_carnet(
            &state,
            session.etablissement_id,
            carnet.iter().map(CarnetEntreeDto::from).collect(),
        )
        .await?,
        notes: enrich_notes(
            &state,
            session.etablissement_id,
            notes.iter().map(NoteDto::from).collect(),
        )
        .await?,
    }))
}

async fn signer_carnet(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<CarnetEntreeDto>, (StatusCode, String)> {
    let entree = state
        .ent
        .signer_carnet_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_carnet(
        &state,
        session.etablissement_id,
        vec![CarnetEntreeDto::from(&entree)],
    )
    .await?;
    Ok(Json(enriched.remove(0)))
}

/// Valeurs de `ENT_ENV` (après trim, insensible à la casse) désignant un
/// environnement explicitement NON productif. Toute autre valeur (absente,
/// vide, `prod`, `production`, inconnue) est traitée comme productive.
pub const ENVS_NON_PROD: [&str; 5] = ["lab", "labo", "dev", "local", "test"];

/// Porte logique pure, fail-closed : ouverte seulement si `ENT_ENV` vaut
/// explicitement un environnement non productif (`ENVS_NON_PROD`) ou si la
/// dérogation explicite vaut `"1"` (y compris en prod).
///
/// Défense en profondeur (L4) : ces portes n'existent QUE dans un binaire
/// compilé avec la feature `labo` ; sans elle, les routes ne sont pas
/// enregistrées (404) et aucun code de labo n'est présent.
#[cfg(feature = "labo")]
fn gate_open(env: Option<&str>, allow: Option<&str>) -> bool {
    let non_prod = env
        .map(|e| e.trim().to_ascii_lowercase())
        .is_some_and(|v| ENVS_NON_PROD.contains(&v.as_str()));
    non_prod || allow == Some("1")
}

/// `ENT_ENV` désigne explicitement un environnement non productif (sans
/// dérogation `ENT_ALLOW_*`) : seul critère pour afficher l'aide labo de /login.
#[cfg(feature = "labo")]
pub(crate) fn env_non_prod() -> bool {
    gate_open(std::env::var("ENT_ENV").ok().as_deref(), None)
}

/// Inscription libre : fermée sauf `ENT_ENV` non productif explicite ou `ENT_ALLOW_REGISTRATION=1`.
#[cfg(feature = "labo")]
fn registration_open() -> bool {
    gate_open(
        std::env::var("ENT_ENV").ok().as_deref(),
        std::env::var("ENT_ALLOW_REGISTRATION").ok().as_deref(),
    )
}

/// Seed de démo / labo : fermé sauf `ENT_ENV` non productif explicite ou `ENT_ALLOW_LABO=1`.
#[cfg(feature = "labo")]
fn labo_open() -> bool {
    gate_open(
        std::env::var("ENT_ENV").ok().as_deref(),
        std::env::var("ENT_ALLOW_LABO").ok().as_deref(),
    )
}

/// Mapping dédié au SSO : une identité inconnue ou un `state` rejoué sont des
/// refus d'authentification (401), une configuration invalide est un 400, un
/// IdP injoignable un 502.
pub(crate) fn map_oidc(err: ent_db::EntDbError) -> (StatusCode, String) {
    if let Some(r) = erreur_infra(&err) {
        return r;
    }
    match &err {
        ent_db::EntDbError::OidcCompteInconnu | ent_db::EntDbError::OidcStateInvalide => {
            (StatusCode::UNAUTHORIZED, err.to_string())
        }
        ent_db::EntDbError::Oidc(ent_db::OidcError::EmailNotVerified) => {
            (StatusCode::UNAUTHORIZED, err.to_string())
        }
        ent_db::EntDbError::OidcProviderInconnu => (StatusCode::NOT_FOUND, err.to_string()),
        ent_db::EntDbError::Forbidden => (StatusCode::FORBIDDEN, err.to_string()),
        // Détails réseau / IdP (URL, hôte, texte distant) : au journal seulement.
        ent_db::EntDbError::Oidc(ent_db::OidcError::Network(_)) => {
            tracing::warn!(error = %err, "SSO : fournisseur injoignable");
            (StatusCode::BAD_GATEWAY, "Fournisseur d'identité injoignable.".to_string())
        }
        ent_db::EntDbError::Oidc(ent_db::OidcError::Http { status }) => {
            // Le statut de l'IdP est au journal seulement : jamais relayé tel quel au client.
            tracing::warn!(error = %err, "SSO : réponse HTTP du fournisseur");
            let _ = status;
            (
                StatusCode::BAD_GATEWAY,
                "Réponse invalide du fournisseur d'identité.".to_string(),
            )
        }
        ent_db::EntDbError::Oidc(ent_db::OidcError::Discovery(_)) | ent_db::EntDbError::Url(_) => {
            tracing::warn!(error = %err, "SSO : configuration du fournisseur invalide");
            (StatusCode::BAD_REQUEST, "Configuration du fournisseur invalide.".to_string())
        }
        _ => (StatusCode::BAD_REQUEST, err.to_string()),
    }
}

/// Mapping dédié aux parcours passkey : une assertion refusée est un 401 (et
/// non un 403) ; une config WebAuthn absente est un 503 (fail closed explicite).
fn map_passkey(err: ent_db::EntDbError) -> (StatusCode, String) {
    if let Some(r) = erreur_infra(&err) {
        return r;
    }
    let msg = err.to_string();
    if msg.contains("accès refusé") || msg.contains("Forbidden") {
        (StatusCode::FORBIDDEN, msg)
    } else if msg.contains("configuration WebAuthn") {
        (StatusCode::SERVICE_UNAVAILABLE, msg)
    } else if msg.contains("authentification par passkey refusée")
        || msg.contains("vérification utilisateur absente")
    {
        (StatusCode::UNAUTHORIZED, msg)
    } else {
        (StatusCode::BAD_REQUEST, msg)
    }
}

fn map_auth(err: ent_db::EntDbError) -> (StatusCode, String) {
    if let Some(r) = erreur_infra(&err) {
        return r;
    }
    let msg = err.to_string();
    if msg.contains("trop de tentatives") {
        (StatusCode::TOO_MANY_REQUESTS, msg)
    } else if msg.contains("déjà") {
        (StatusCode::CONFLICT, msg)
    } else if msg.contains("trop court") || msg.contains("trop long") || msg.contains("e-mail vide") {
        (StatusCode::BAD_REQUEST, msg)
    } else if msg.contains("réinitialisation") {
        (StatusCode::BAD_REQUEST, msg)
    } else if msg.contains("membre") || msg.contains("refusé") {
        (StatusCode::FORBIDDEN, msg)
    } else {
        (StatusCode::UNAUTHORIZED, msg)
    }
}

async fn list_factures(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<FactureDto>>, (StatusCode, String)> {
    if !ent_db::PgEnt::role_can_lire_facturation(&session.role) {
        return Err((
            StatusCode::FORBIDDEN,
            "utiliser /facturation/famille".into(),
        ));
    }
    let list = state
        .ent
        .list_factures_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    // F0 (R-F36) : toute lecture nominative du personnel est journalisée.
    tracer_acces(&state, &session, "facturation", "lecture", "Liste des factures").await;
    Ok(Json(list.iter().map(FactureDto::from).collect()))
}

async fn list_factures_famille(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<FactureFamilleListItemDto>>, (StatusCode, String)> {
    // F0 (FD-26) : décision par **droit**, pas par un nom de rôle en dur. Un compte qui
    // consulte la facturation de l'établissement utilise `/facturation/factures` ; ici,
    // le service ne rend que les factures des foyers dont l'appelant est payeur.
    if ent_db::PgEnt::role_can_lire_facturation(&session.role) {
        return Err((
            StatusCode::FORBIDDEN,
            "utiliser /facturation/factures".into(),
        ));
    }
    let list = state
        .ent
        .list_factures_famille_enriched(
            session.etablissement_id,
            session.user_id,
            &session.role,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        list.into_iter()
            .map(|(f, encaisse, reste)| FactureFamilleListItemDto {
                facture: FactureDto::from(&f),
                encaisse_centimes: encaisse,
                reste_centimes: reste,
            })
            .collect(),
    ))
}

async fn creer_facture(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerFactureBody>,
) -> Result<(StatusCode, Json<FactureDetailDto>), (StatusCode, String)> {
    let (facture, lignes) = state
        .ent
        .creer_facture_for_actor(
            &session.role,
            CreerFacture {
                etablissement_id: session.etablissement_id,
                foyer_id: body.foyer_id,
                numero: body.numero,
                libelle: body.libelle,
                nature: body.nature,
                annee_scolaire: body.annee_scolaire,
                lignes: body
                    .lignes
                    .into_iter()
                    .map(|l| LigneFactureCmd {
                        eleve_id: l.eleve_id,
                        libelle: l.libelle,
                        quantite: l.quantite,
                        montant_unitaire_centimes: l.montant_unitaire_centimes,
                    })
                    .collect(),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(FactureDetailDto {
            facture: FactureDto::from(&facture),
            lignes: lignes.iter().map(FactureLigneDto::from).collect(),
            encaissements: vec![],
            echeances: vec![],
            quittances: vec![],
            prelevements: vec![],
            encaisse_centimes: 0,
            reste_centimes: facture.total_centimes,
        }),
    ))
}

async fn get_facture(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<FactureDetailDto>, (StatusCode, String)> {
    let (facture, lignes, encaissements, encaisse, echeances, quittances, prelevements) = state
        .ent
        .get_facture_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    if ent_db::PgEnt::role_can_lire_facturation(&session.role) {
        // F0 (R-F36) : consultation nominative d'un compte par le personnel.
        tracer_acces(&state, &session, "facturation", "lecture", "Consultation d'une facture").await;
    }
    Ok(Json(FactureDetailDto {
        facture: FactureDto::from(&facture),
        lignes: lignes.iter().map(FactureLigneDto::from).collect(),
        encaissements: encaissements.iter().map(EncaissementDto::from).collect(),
        echeances: echeances.iter().map(EcheanceDto::from).collect(),
        quittances: quittances.iter().map(QuittanceDto::from).collect(),
        prelevements: prelevements.iter().map(PrelevementSepaDto::from).collect(),
        encaisse_centimes: encaisse,
        reste_centimes: reste_a_payer(facture.total_centimes, encaisse),
    }))
}

pub(crate) fn attachment_response(
    bytes: Vec<u8>,
    filename: &str,
    content_type: &'static str,
) -> Result<Response, (StatusCode, String)> {
    attachment_response_dyn(bytes, filename, content_type)
}

fn attachment_response_dyn(
    bytes: Vec<u8>,
    filename: &str,
    content_type: &str,
) -> Result<Response, (StatusCode, String)> {
    let safe: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let disp = format!("attachment; filename=\"{safe}\"");
    let mut res = Response::new(Body::from(bytes));
    *res.status_mut() = StatusCode::OK;
    let headers = res.headers_mut();
    let ct = HeaderValue::from_str(content_type).unwrap_or_else(|_| {
        HeaderValue::from_static("application/octet-stream")
    });
    headers.insert(header::CONTENT_TYPE, ct);
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disp).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "nom de fichier invalide".into(),
            )
        })?,
    );
    Ok(res)
}

fn pdf_response(bytes: Vec<u8>, filename: &str) -> Result<Response, (StatusCode, String)> {
    attachment_response(bytes, filename, "application/pdf")
}

#[derive(Debug, Deserialize)]
struct OgecExportQuery {
    debut: NaiveDate,
    fin: NaiveDate,
}

async fn export_ogec(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<OgecExportQuery>,
) -> Result<Response, (StatusCode, String)> {
    let bytes = state
        .ent
        .export_ogec_for_actor(
            &session.role,
            ExporterOgec {
                etablissement_id: session.etablissement_id,
                debut: q.debut,
                fin: q.fin,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "export", "ogec", "Export OGEC").await;
    let name = format!("ogec-{}-{}.csv", q.debut, q.fin);
    attachment_response(bytes, &name, "text/csv; charset=utf-8")
}

#[derive(Debug, Deserialize)]
struct VolumesRepasQuery {
    debut: NaiveDate,
    fin: NaiveDate,
    kind: Option<String>,
}

fn volumes_cmd(
    etablissement_id: Uuid,
    q: &VolumesRepasQuery,
) -> Result<ExporterVolumesRepas, (StatusCode, String)> {
    let kind = match q.kind.as_deref() {
        None | Some("") => None,
        Some(s) => Some(
            RepasActiviteKind::parse(s)
                .ok_or_else(|| (StatusCode::BAD_REQUEST, "kind repas invalide".into()))?,
        ),
    };
    Ok(ExporterVolumesRepas {
        etablissement_id,
        debut: q.debut,
        fin: q.fin,
        kind,
    })
}

async fn list_volumes_repas(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<VolumesRepasQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let cmd = volumes_cmd(session.etablissement_id, &q)?;
    let (lignes, _) = state
        .ent
        .export_volumes_cuisine_for_actor(&session.role, cmd)
        .await
        .map_err(map_db)?;
    Ok(Json(serde_json::json!({
        "debut": q.debut,
        "fin": q.fin,
        "lignes": lignes.iter().map(|l| serde_json::json!({
            "jour": l.jour,
            "activite_id": l.activite_id,
            "activite_code": l.activite_code,
            "kind": l.kind.as_str(),
            "nb_prevus": l.nb_prevus,
        })).collect::<Vec<_>>(),
    })))
}

async fn export_volumes_cuisine(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<VolumesRepasQuery>,
) -> Result<Response, (StatusCode, String)> {
    let cmd = volumes_cmd(session.etablissement_id, &q)?;
    let (_lignes, bytes) = state
        .ent
        .export_volumes_cuisine_for_actor(&session.role, cmd)
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "export", "volumes_cuisine", "Export volumes cuisine").await;
    let name = format!("volumes-cuisine-{}-{}.csv", q.debut, q.fin);
    attachment_response(bytes, &name, "text/csv; charset=utf-8")
}

#[derive(Debug, Deserialize)]
struct PassagesRepasQuery {
    jour: Option<NaiveDate>,
}

async fn list_passages_repas(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<PassagesRepasQuery>,
) -> Result<Json<Vec<PassageRepasDto>>, (StatusCode, String)> {
    let jour = q.jour.unwrap_or_else(today_utc);
    let list = state
        .ent
        .list_passages_repas_jour_for_actor(session.etablissement_id, &session.role, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_passages_repas(
            &state,
            session.etablissement_id,
            list.iter().map(PassageRepasDto::from).collect(),
        )
        .await?,
    ))
}

#[derive(Debug, Deserialize)]
struct PassagesEleveQuery {
    debut: Option<NaiveDate>,
    fin: Option<NaiveDate>,
}

async fn list_passages_repas_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PassagesEleveQuery>,
) -> Result<Json<Vec<PassageRepasDto>>, (StatusCode, String)> {
    let debut = q.debut.unwrap_or_else(|| today_utc() - chrono::Duration::days(14));
    let fin = q.fin.unwrap_or_else(today_utc);
    let list = state
        .ent
        .list_passages_repas_eleve_for_actor(
            session.etablissement_id,
            &session.role,
            session.user_id,
            id,
            debut,
            fin,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(PassageRepasDto::from).collect()))
}

async fn scan_badge_repas(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<ScanBadgeRepasBody>,
) -> Result<(StatusCode, Json<PassageRepasDto>), (StatusCode, String)> {
    let kind = RepasActiviteKind::parse(&body.kind)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "kind repas invalide".into()))?;
    let jour = body.jour.unwrap_or_else(today_utc);
    let (passage, eleve) = state
        .ent
        .scan_badge_repas_for_actor(
            session.etablissement_id,
            &session.role,
            &body.badge,
            kind,
            jour,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    let mut dto = PassageRepasDto::from(&passage);
    dto.eleve_nom = Some(eleve.nom);
    dto.eleve_prenom = Some(eleve.prenom);
    Ok((StatusCode::CREATED, Json(dto)))
}

#[derive(Debug, Deserialize)]
struct CantineBadgeQuery {
    eleve_id: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
    #[serde(default = "default_midi")]
    kind: String,
    #[serde(default = "default_prix_cantine")]
    prix_unitaire_centimes: i64,
}

fn default_prix_cantine() -> i64 {
    450
}

async fn apercu_cantine_badge(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<CantineBadgeQuery>,
) -> Result<Json<CantineBadgeApercuDto>, (StatusCode, String)> {
    let kind = RepasActiviteKind::parse(&q.kind)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "kind repas invalide".into()))?;
    let n = state
        .ent
        .apercu_cantine_badge_for_actor(
            &session.role,
            session.etablissement_id,
            q.eleve_id,
            q.debut,
            q.fin,
            kind,
        )
        .await
        .map_err(map_db)?;
    let total = (n as i64).saturating_mul(q.prix_unitaire_centimes);
    Ok(Json(CantineBadgeApercuDto {
        eleve_id: q.eleve_id,
        debut: q.debut,
        fin: q.fin,
        nb_passages: n,
        prix_unitaire_centimes: q.prix_unitaire_centimes,
        total_centimes: total,
    }))
}

async fn creer_facture_cantine_badge(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CantineBadgeFactureBody>,
) -> Result<(StatusCode, Json<FactureDetailDto>), (StatusCode, String)> {
    let kind = RepasActiviteKind::parse(&body.kind)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "kind repas invalide".into()))?;
    let (facture, _lignes, _nb) = state
        .ent
        .creer_facture_cantine_badge_for_actor(
            &session.role,
            session.etablissement_id,
            body.foyer_id,
            body.eleve_id,
            body.debut,
            body.fin,
            body.prix_unitaire_centimes,
            body.numero,
            session.user_id,
            kind,
        )
        .await
        .map_err(map_db)?;
    let detail = get_facture(State(state), session, Path(facture.id)).await?;
    Ok((StatusCode::CREATED, detail))
}

async fn list_sepa_mandats(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<MandatSepaDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_mandats_sepa_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    if ent_db::PgEnt::role_can_lire_facturation(&session.role) {
        // F0 (R-F36, R-P30) : consultation des mandats par le personnel.
        tracer_acces(&state, &session, "facturation", "lecture", "Liste des mandats SEPA").await;
    }
    Ok(Json(list.iter().map(MandatSepaDto::from).collect()))
}

async fn creer_sepa_mandat(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerMandatSepaBody>,
) -> Result<(StatusCode, Json<MandatSepaDto>), (StatusCode, String)> {
    let mandat = state
        .ent
        .creer_mandat_sepa_for_actor(
            &session.role,
            session.user_id,
            CreerMandatSepa {
                etablissement_id: session.etablissement_id,
                foyer_id: body.foyer_id,
                rum: body.rum,
                iban: body.iban,
                bic: body.bic,
                titulaire: body.titulaire,
                signe_at: Utc::now(),
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(MandatSepaDto::from(&mandat))))
}

async fn revoquer_sepa_mandat(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<MandatSepaDto>, (StatusCode, String)> {
    let mandat = state
        .ent
        .revoquer_mandat_sepa_for_actor(
            session.etablissement_id,
            id,
            &session.role,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(MandatSepaDto::from(&mandat)))
}

async fn planifier_sepa_prelevement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PlanifierPrelevementBody>,
) -> Result<(StatusCode, Json<PrelevementSepaDto>), (StatusCode, String)> {
    let prev = state
        .ent
        .planifier_prelevement_for_actor(
            &session.role,
            PlanifierPrelevement {
                etablissement_id: session.etablissement_id,
                facture_id: id,
                mandat_id: body.mandat_id,
                montant_centimes: body.montant_centimes,
                date_execution: body.date_execution,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(PrelevementSepaDto::from(&prev)),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SepaExportBody {
    debut: NaiveDate,
    fin: NaiveDate,
    /// `csv` (défaut) ou `pain008`
    format: Option<String>,
    // F0 (FD-03, R-P2) : aucun champ `cdtr_*`. Le nom, l'IBAN, le BIC et l'ICS du
    // créancier sont lus côté serveur dans les paramètres de l'établissement. Un
    // champ inconnu est refusé (`deny_unknown_fields` ci-dessous), pas ignoré.
}

async fn export_sepa(
    State(state): State<AppState>,
    session: EntSession,
    Json(q): Json<SepaExportBody>,
) -> Result<Response, (StatusCode, String)> {
    let format = q.format.as_deref().unwrap_or("csv");
    let (bytes, kind) = state
        .ent
        .export_sepa_for_actor(
            session.etablissement_id,
            &session.role,
            q.debut,
            q.fin,
            session.user_id,
            format,
        )
        .await
        .map_err(map_db)?;
    tracer_acces(&state, &session, "export", "sepa", "Export SEPA").await;
    if kind == "pain008" {
        let name = format!("sepa-pain008-{}-{}.xml", q.debut, q.fin);
        attachment_response(bytes, &name, "application/xml; charset=utf-8")
    } else {
        let name = format!("sepa-{}-{}.csv", q.debut, q.fin);
        attachment_response(bytes, &name, "text/csv; charset=utf-8")
    }
}

async fn executer_sepa_prelevement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<PrelevementSepaDto>, (StatusCode, String)> {
    let (prev, _) = state
        .ent
        .executer_prelevement_for_actor(
            session.etablissement_id,
            id,
            &session.role,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(PrelevementSepaDto::from(&prev)))
}

async fn rejeter_sepa_prelevement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<RejeterPrelevementBody>,
) -> Result<Json<PrelevementSepaDto>, (StatusCode, String)> {
    let prev = state
        .ent
        .rejeter_prelevement_for_actor(
            session.etablissement_id,
            id,
            &session.role,
            &body.motif,
            session.user_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(PrelevementSepaDto::from(&prev)))
}

#[derive(Debug, Deserialize)]
struct RapprocherBanqueBody {
    csv: String,
}

#[derive(Debug, Serialize)]
struct BankRapprochementDto {
    lines: usize,
    executed: usize,
    rejected: usize,
    unmatched: usize,
    errors: usize,
}

async fn rapprocher_releve_banque(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<RapprocherBanqueBody>,
) -> Result<Json<BankRapprochementDto>, (StatusCode, String)> {
    let summary = state
        .ent
        .rapprocher_releve_banque_for_actor(
            session.etablissement_id,
            &session.role,
            session.user_id,
            &body.csv,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(BankRapprochementDto {
        lines: summary.lines,
        executed: summary.executed,
        rejected: summary.rejected,
        unmatched: summary.unmatched,
        errors: summary.errors,
    }))
}

async fn list_rappels(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<RappelFactureDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_rappels_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(RappelFactureDto::from).collect()))
}

async fn lancer_rappels(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<(StatusCode, Json<Vec<RappelFactureDto>>), (StatusCode, String)> {
    let envoyes = state
        .ent
        .lancer_rappels_echeances_for_actor(
            session.etablissement_id,
            &session.role,
            session.user_id,
            today_utc(),
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(envoyes.iter().map(RappelFactureDto::from).collect()),
    ))
}

async fn pdf_facture(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, String)> {
    let (facture, lignes, _, encaisse, _, _, _) = state
        .ent
        .get_facture_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    let bytes = render_facture_pdf(&facture, &lignes, encaisse);
    pdf_response(bytes, &format!("facture-{}.pdf", facture.numero))
}

async fn emettre_facture(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<FactureDto>, (StatusCode, String)> {
    let facture = state
        .ent
        .emettre_facture_for_actor(
            session.etablissement_id,
            id,
            session.user_id,
            &session.role,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(FactureDto::from(&facture)))
}

async fn encaisser_facture(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<EncaisserBody>,
) -> Result<(StatusCode, Json<FactureDetailDto>), (StatusCode, String)> {
    let moyen = MoyenPaiement::parse(&body.moyen).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "moyen invalide (especes|cheque|virement|cb|autre)".into(),
        )
    })?;
    let (_enc, _, _quittance) = state
        .ent
        .encaisser_facture_for_actor(
            &session.role,
            EncaisserFacture {
                etablissement_id: session.etablissement_id,
                facture_id: id,
                montant_centimes: body.montant_centimes,
                moyen,
                reference: body.reference,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    let (facture, lignes, encaissements, encaisse, echeances, quittances, prelevements) = state
        .ent
        .get_facture_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(FactureDetailDto {
            facture: FactureDto::from(&facture),
            lignes: lignes.iter().map(FactureLigneDto::from).collect(),
            encaissements: encaissements.iter().map(EncaissementDto::from).collect(),
            echeances: echeances.iter().map(EcheanceDto::from).collect(),
            quittances: quittances.iter().map(QuittanceDto::from).collect(),
            prelevements: prelevements.iter().map(PrelevementSepaDto::from).collect(),
            encaisse_centimes: encaisse,
            reste_centimes: reste_a_payer(facture.total_centimes, encaisse),
        }),
    ))
}

async fn list_quittances_famille(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<QuittanceDto>>, (StatusCode, String)> {
    if session.role != "parent" {
        return Err((StatusCode::FORBIDDEN, "réservé aux parents".into()));
    }
    let list = state
        .ent
        .list_quittances_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(QuittanceDto::from).collect()))
}

async fn get_quittance(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<QuittanceDto>, (StatusCode, String)> {
    let q = state
        .ent
        .get_quittance_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(QuittanceDto::from(&q)))
}

async fn pdf_quittance(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, String)> {
    let q = state
        .ent
        .get_quittance_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    let (facture, _, _, _, _, _, _) = state
        .ent
        .get_facture_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.facture_id,
        )
        .await
        .map_err(map_db)?;
    let bytes = render_quittance_pdf(&q, &facture.numero);
    pdf_response(bytes, &format!("quittance-{}.pdf", q.numero))
}

async fn poser_echeancier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PoserEcheancierBody>,
) -> Result<(StatusCode, Json<FactureDetailDto>), (StatusCode, String)> {
    state
        .ent
        .poser_echeancier_for_actor(
            &session.role,
            PoserEcheancier {
                etablissement_id: session.etablissement_id,
                facture_id: id,
                echeances: body
                    .echeances
                    .into_iter()
                    .map(|e| EcheanceCmd {
                        libelle: e.libelle,
                        montant_centimes: e.montant_centimes,
                        date_echeance: e.date_echeance,
                    })
                    .collect(),
            },
        )
        .await
        .map_err(map_db)?;
    let detail = get_facture(State(state), session, Path(id)).await?;
    Ok((StatusCode::CREATED, detail))
}

async fn poser_echeancier_parts(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<EcheancierPartsBody>,
) -> Result<(StatusCode, Json<FactureDetailDto>), (StatusCode, String)> {
    state
        .ent
        .poser_echeancier_parts_for_actor(
            session.etablissement_id,
            id,
            &session.role,
            body.dates,
        )
        .await
        .map_err(map_db)?;
    let detail = get_facture(State(state), session, Path(id)).await?;
    Ok((StatusCode::CREATED, detail))
}

async fn enrich_edt_creneaux(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<EdtCreneauDto>,
) -> Result<Vec<EdtCreneauDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|c| c.prof_user_id).collect();
    let labels = state.ent.user_labels(etablissement_id, &ids).await.map_err(map_db)?;
    for c in &mut list {
        c.prof_label = labels.get(&c.prof_user_id).cloned();
    }
    Ok(list)
}

async fn enrich_edt_creneau(
    state: &AppState,
    etablissement_id: Uuid,
    mut c: EdtCreneauDto,
) -> Result<EdtCreneauDto, (StatusCode, String)> {
    let labels = state
        .ent
        .user_labels(etablissement_id, &[c.prof_user_id])
        .await
        .map_err(map_db)?;
    c.prof_label = labels.get(&c.prof_user_id).cloned();
    Ok(c)
}

async fn enrich_notes(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<NoteDto>,
) -> Result<Vec<NoteDto>, (StatusCode, String)> {
    if list.is_empty() {
        return Ok(list);
    }
    let matieres = state
        .ent
        .list_matieres(etablissement_id)
        .await
        .map_err(map_db)?;
    let by_id: std::collections::HashMap<Uuid, String> = matieres
        .into_iter()
        .map(|m| (m.id, m.libelle))
        .collect();
    for n in &mut list {
        n.matiere_libelle = by_id.get(&n.matiere_id).cloned();
    }
    Ok(list)
}

async fn enrich_messages(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<MessageDto>,
) -> Result<Vec<MessageDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|m| m.auteur_user_id).collect();
    let labels = state.ent.user_labels(etablissement_id, &ids).await.map_err(map_db)?;
    for m in &mut list {
        m.auteur_label = labels.get(&m.auteur_user_id).cloned();
    }
    Ok(list)
}

async fn enrich_message(
    state: &AppState,
    etablissement_id: Uuid,
    mut m: MessageDto,
) -> Result<MessageDto, (StatusCode, String)> {
    let labels = state
        .ent
        .user_labels(etablissement_id, &[m.auteur_user_id])
        .await
        .map_err(map_db)?;
    m.auteur_label = labels.get(&m.auteur_user_id).cloned();
    Ok(m)
}

async fn enrich_grants(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<DocumentGrantDto>,
) -> Result<Vec<DocumentGrantDto>, (StatusCode, String)> {
    let user_ids: Vec<Uuid> = list.iter().map(|g| g.grantee_user_id).collect();
    let doc_ids: Vec<Uuid> = list.iter().map(|g| g.document_id).collect();
    let labels = state.ent.user_labels(etablissement_id, &user_ids).await.map_err(map_db)?;
    let titres = state
        .ent
        .document_labels(etablissement_id, &doc_ids)
        .await
        .map_err(map_db)?;
    for g in &mut list {
        g.grantee_label = labels.get(&g.grantee_user_id).cloned();
        g.document_titre = titres.get(&g.document_id).cloned();
    }
    Ok(list)
}

async fn enrich_grant(
    state: &AppState,
    etablissement_id: Uuid,
    mut g: DocumentGrantDto,
) -> Result<DocumentGrantDto, (StatusCode, String)> {
    let labels = state
        .ent
        .user_labels(etablissement_id, &[g.grantee_user_id])
        .await
        .map_err(map_db)?;
    let titres = state
        .ent
        .document_labels(etablissement_id, &[g.document_id])
        .await
        .map_err(map_db)?;
    g.grantee_label = labels.get(&g.grantee_user_id).cloned();
    g.document_titre = titres.get(&g.document_id).cloned();
    Ok(g)
}

async fn enrich_devoirs(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<DevoirDto>,
) -> Result<Vec<DevoirDto>, (StatusCode, String)> {
    if list.is_empty() {
        return Ok(list);
    }
    let matieres = state
        .ent
        .list_matieres(etablissement_id)
        .await
        .map_err(map_db)?;
    let by_id: std::collections::HashMap<Uuid, String> =
        matieres.into_iter().map(|m| (m.id, m.libelle)).collect();
    for d in &mut list {
        d.matiere_libelle = by_id.get(&d.matiere_id).cloned();
    }
    Ok(list)
}

async fn enrich_devoir(
    state: &AppState,
    etablissement_id: Uuid,
    d: DevoirDto,
) -> Result<DevoirDto, (StatusCode, String)> {
    let mut list = enrich_devoirs(state, etablissement_id, vec![d]).await?;
    Ok(list.remove(0))
}

async fn enrich_appreciations(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<AppreciationDto>,
) -> Result<Vec<AppreciationDto>, (StatusCode, String)> {
    if list.is_empty() {
        return Ok(list);
    }
    let matieres = state
        .ent
        .list_matieres(etablissement_id)
        .await
        .map_err(map_db)?;
    let by_mat: std::collections::HashMap<Uuid, String> =
        matieres.into_iter().map(|m| (m.id, m.libelle)).collect();
    let auteurs: Vec<Uuid> = list.iter().map(|a| a.auteur_user_id).collect();
    let labels = state.ent.user_labels(etablissement_id, &auteurs).await.map_err(map_db)?;
    for a in &mut list {
        a.matiere_libelle = a.matiere_id.and_then(|id| by_mat.get(&id).cloned());
        a.auteur_label = labels.get(&a.auteur_user_id).cloned();
    }
    Ok(list)
}

async fn enrich_edt_exceptions(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<EdtExceptionDto>,
) -> Result<Vec<EdtExceptionDto>, (StatusCode, String)> {
    let remplacant_ids: Vec<Uuid> = list
        .iter()
        .filter_map(|e| e.remplacant_user_id)
        .collect();
    let creneau_ids: Vec<Uuid> = list.iter().map(|e| e.creneau_id).collect();
    let labels = state
        .ent
        .user_labels(etablissement_id, &remplacant_ids)
        .await
        .map_err(map_db)?;
    let creneau_labels = state
        .ent
        .edt_creneau_labels(etablissement_id, &creneau_ids)
        .await
        .map_err(map_db)?;
    for e in &mut list {
        e.remplacant_label = e
            .remplacant_user_id
            .and_then(|id| labels.get(&id).cloned());
        e.creneau_label = creneau_labels.get(&e.creneau_id).cloned();
    }
    Ok(list)
}

async fn enrich_edt_contraintes(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<EdtContrainteDto>,
) -> Result<Vec<EdtContrainteDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().filter_map(|c| c.prof_user_id).collect();
    let labels = state.ent.user_labels(etablissement_id, &ids).await.map_err(map_db)?;
    for c in &mut list {
        c.prof_label = c.prof_user_id.and_then(|id| labels.get(&id).cloned());
    }
    Ok(list)
}

async fn enrich_absences_rh(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<AbsenceRhDto>,
) -> Result<Vec<AbsenceRhDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|a| a.user_id).collect();
    let labels = state.ent.user_labels(etablissement_id, &ids).await.map_err(map_db)?;
    for a in &mut list {
        a.user_label = labels.get(&a.user_id).cloned();
    }
    Ok(list)
}

async fn enrich_voeux(
    state: &AppState,
    etablissement_id: Uuid,
    role: &str,
    mut list: Vec<VoeuDto>,
) -> Result<Vec<VoeuDto>, (StatusCode, String)> {
    if list.is_empty() {
        return Ok(list);
    }
    let filieres = state
        .ent
        .list_filieres_for_actor(etablissement_id, role)
        .await
        .map_err(map_db)?;
    let by_id: std::collections::HashMap<Uuid, String> = filieres
        .into_iter()
        .map(|f| (f.id, format!("{} — {}", f.code, f.libelle)))
        .collect();
    for v in &mut list {
        v.filiere_label = by_id.get(&v.filiere_id).cloned();
    }
    Ok(list)
}

async fn enrich_invitations(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<InvitationCompteDto>,
) -> Result<Vec<InvitationCompteDto>, (StatusCode, String)> {
    let user_ids: Vec<Uuid> = list.iter().filter_map(|i| i.activated_user_id).collect();
    let eleve_ids: Vec<Uuid> = list.iter().map(|i| i.eleve_id).collect();
    let labels = state.ent.user_labels(etablissement_id, &user_ids).await.map_err(map_db)?;
    let eleves = state
        .ent
        .eleve_labels(etablissement_id, &eleve_ids)
        .await
        .map_err(map_db)?;
    for i in &mut list {
        i.activated_user_label = i
            .activated_user_id
            .and_then(|id| labels.get(&id).cloned());
        i.eleve_label = eleves.get(&i.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_absences_eleve(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<AbsenceDto>,
) -> Result<Vec<AbsenceDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|a| a.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for a in &mut list {
        a.eleve_label = labels.get(&a.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_signaux_eleve(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<SignalAbsenceDto>,
) -> Result<Vec<SignalAbsenceDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|s| s.eleve_id).collect();
    let user_ids: Vec<Uuid> = list.iter().filter_map(|s| s.traite_par_user_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    let users = state.ent.user_labels(etablissement_id, &user_ids).await.map_err(map_db)?;
    for s in &mut list {
        s.eleve_label = labels.get(&s.eleve_id).cloned();
        s.traite_par_label = s
            .traite_par_user_id
            .and_then(|id| users.get(&id).cloned());
    }
    Ok(list)
}

async fn enrich_dispenses_vs(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<DispenseVsDto>,
) -> Result<Vec<DispenseVsDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|d| d.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for d in &mut list {
        d.eleve_label = labels.get(&d.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_alertes_cpe(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<AlerteCpeDto>,
) -> Result<Vec<AlerteCpeDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|a| a.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for a in &mut list {
        a.eleve_label = labels.get(&a.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_docs_a_signer(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<DocASignerDto>,
) -> Result<Vec<DocASignerDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|d| d.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for d in &mut list {
        d.eleve_label = labels.get(&d.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_passages_infirmerie(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<PassageInfirmerieDto>,
) -> Result<Vec<PassageInfirmerieDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|p| p.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for p in &mut list {
        p.eleve_label = labels.get(&p.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_commission_dossiers(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<CommissionDossierDto>,
) -> Result<Vec<CommissionDossierDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|d| d.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for d in &mut list {
        d.eleve_label = labels.get(&d.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_conventions_stage(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<ConventionStageDto>,
) -> Result<Vec<ConventionStageDto>, (StatusCode, String)> {
    let eleve_ids: Vec<Uuid> = list.iter().map(|c| c.eleve_id).collect();
    let entreprise_ids: Vec<Uuid> = list.iter().map(|c| c.entreprise_id).collect();
    let eleves = state
        .ent
        .eleve_labels(etablissement_id, &eleve_ids)
        .await
        .map_err(map_db)?;
    let entreprises = state
        .ent
        .entreprise_labels(etablissement_id, &entreprise_ids)
        .await
        .map_err(map_db)?;
    for c in &mut list {
        c.eleve_label = eleves.get(&c.eleve_id).cloned();
        c.entreprise_label = entreprises.get(&c.entreprise_id).cloned();
    }
    Ok(list)
}

async fn enrich_demandes_sortie(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<DemandeSortieDto>,
) -> Result<Vec<DemandeSortieDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|d| d.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for d in &mut list {
        d.eleve_label = labels.get(&d.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_msg_fils(
    state: &AppState,
    etablissement_id: Uuid,
    lecteur_user_id: Uuid,
    mut list: Vec<MsgFilDto>,
) -> Result<Vec<MsgFilDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|f| f.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    let fil_ids: Vec<Uuid> = list.iter().map(|f| f.id).collect();
    let non_lus = state
        .ent
        .count_msg_non_lus_par_fils(etablissement_id, lecteur_user_id, &fil_ids)
        .await
        .map_err(map_db)?;
    for f in &mut list {
        f.eleve_label = labels.get(&f.eleve_id).cloned();
        f.nb_non_lus = non_lus.get(&f.id).copied().unwrap_or(0);
    }
    Ok(list)
}

async fn enrich_presences_internat(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<PresenceInternatDto>,
) -> Result<Vec<PresenceInternatDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|p| p.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for p in &mut list {
        p.eleve_label = labels.get(&p.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_controles_sortie(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<ControleSortieDto>,
) -> Result<Vec<ControleSortieDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|c| c.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for c in &mut list {
        c.eleve_label = labels.get(&c.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_entretiens_psy(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<EntretienPsyDto>,
) -> Result<Vec<EntretienPsyDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|e| e.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for e in &mut list {
        e.eleve_label = labels.get(&e.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_rdvs_psy(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<RdvPsyDto>,
) -> Result<Vec<RdvPsyDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|r| r.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for r in &mut list {
        r.eleve_label = labels.get(&r.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_carnet(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<CarnetEntreeDto>,
) -> Result<Vec<CarnetEntreeDto>, (StatusCode, String)> {
    let eleve_ids: Vec<Uuid> = list.iter().map(|c| c.eleve_id).collect();
    let user_ids: Vec<Uuid> = list.iter().filter_map(|c| c.signe_par_user_id).collect();
    let eleves = state
        .ent
        .eleve_labels(etablissement_id, &eleve_ids)
        .await
        .map_err(map_db)?;
    let users = state.ent.user_labels(etablissement_id, &user_ids).await.map_err(map_db)?;
    for c in &mut list {
        c.eleve_label = eleves.get(&c.eleve_id).cloned();
        c.signe_par_label = c
            .signe_par_user_id
            .and_then(|id| users.get(&id).cloned());
    }
    Ok(list)
}

async fn enrich_passages_repas(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<PassageRepasDto>,
) -> Result<Vec<PassageRepasDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|p| p.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for p in &mut list {
        if let Some(label) = labels.get(&p.eleve_id) {
            let mut parts = label.splitn(2, ' ');
            p.eleve_prenom = parts.next().map(|s| s.to_string());
            p.eleve_nom = parts.next().map(|s| s.to_string());
        }
    }
    Ok(list)
}

async fn enrich_sanctions(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<SanctionDto>,
) -> Result<Vec<SanctionDto>, (StatusCode, String)> {
    let eleve_ids: Vec<Uuid> = list.iter().map(|s| s.eleve_id).collect();
    let eleves = state
        .ent
        .eleve_labels(etablissement_id, &eleve_ids)
        .await
        .map_err(map_db)?;
    let types = state
        .ent
        .list_sanction_types(etablissement_id)
        .await
        .map_err(map_db)?;
    let motifs = state
        .ent
        .list_sanction_motifs(etablissement_id)
        .await
        .map_err(map_db)?;
    let by_type: std::collections::HashMap<Uuid, String> =
        types.into_iter().map(|t| (t.id, t.libelle)).collect();
    let by_motif: std::collections::HashMap<Uuid, String> =
        motifs.into_iter().map(|m| (m.id, m.libelle)).collect();
    for s in &mut list {
        s.eleve_label = eleves.get(&s.eleve_id).cloned();
        s.type_label = by_type.get(&s.type_id).cloned();
        s.motif_label = s.motif_id.and_then(|id| by_motif.get(&id).cloned());
    }
    Ok(list)
}

async fn enrich_incidents(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<IncidentDto>,
) -> Result<Vec<IncidentDto>, (StatusCode, String)> {
    let eleve_ids: Vec<Uuid> = list.iter().map(|i| i.eleve_id).collect();
    let eleves = state
        .ent
        .eleve_labels(etablissement_id, &eleve_ids)
        .await
        .map_err(map_db)?;
    let types = state
        .ent
        .list_incident_types(etablissement_id)
        .await
        .map_err(map_db)?;
    let by_type: std::collections::HashMap<Uuid, String> =
        types.into_iter().map(|t| (t.id, t.libelle)).collect();
    for i in &mut list {
        i.eleve_label = eleves.get(&i.eleve_id).cloned();
        i.type_label = by_type.get(&i.type_id).cloned();
    }
    Ok(list)
}

async fn enrich_autorisations_sortie(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<AutorisationSortieDto>,
) -> Result<Vec<AutorisationSortieDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|a| a.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for a in &mut list {
        a.eleve_label = labels.get(&a.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_recap_sortie(
    state: &AppState,
    etablissement_id: Uuid,
    mut recap: RecapSortieDto,
) -> Result<RecapSortieDto, (StatusCode, String)> {
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &[recap.eleve_id])
        .await
        .map_err(map_db)?;
    let label = labels.get(&recap.eleve_id).cloned();
    recap.eleve_label = label.clone();
    if let Some(ref mut a) = recap.autorisation {
        a.eleve_label = label.clone();
    }
    recap.demandes_ouvertes = enrich_demandes_sortie(
        state,
        etablissement_id,
        recap.demandes_ouvertes,
    )
    .await?;
    Ok(recap)
}

async fn enrich_conseil_dossiers(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<ConseilClasseDossierDto>,
) -> Result<Vec<ConseilClasseDossierDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|d| d.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for d in &mut list {
        d.eleve_label = labels.get(&d.eleve_id).cloned();
    }
    Ok(list)
}

async fn enrich_grilles_repas(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<GrilleRepasDto>,
) -> Result<Vec<GrilleRepasDto>, (StatusCode, String)> {
    let eleve_ids: Vec<Uuid> = list.iter().map(|g| g.eleve_id).collect();
    let eleves = state
        .ent
        .eleve_labels(etablissement_id, &eleve_ids)
        .await
        .map_err(map_db)?;
    let acts = state
        .ent
        .list_repas_activites(etablissement_id)
        .await
        .map_err(map_db)?;
    let by_act: std::collections::HashMap<Uuid, String> = acts
        .into_iter()
        .map(|a| (a.id, format!("{} — {}", a.code, a.libelle)))
        .collect();
    for g in &mut list {
        g.eleve_label = eleves.get(&g.eleve_id).cloned();
        g.activite_label = by_act.get(&g.activite_id).cloned();
    }
    Ok(list)
}

async fn enrich_dispenses_repas(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<DispenseRepasDto>,
) -> Result<Vec<DispenseRepasDto>, (StatusCode, String)> {
    let eleve_ids: Vec<Uuid> = list.iter().map(|d| d.eleve_id).collect();
    let eleves = state
        .ent
        .eleve_labels(etablissement_id, &eleve_ids)
        .await
        .map_err(map_db)?;
    let acts = state
        .ent
        .list_repas_activites(etablissement_id)
        .await
        .map_err(map_db)?;
    let by_act: std::collections::HashMap<Uuid, String> = acts
        .into_iter()
        .map(|a| (a.id, format!("{} — {}", a.code, a.libelle)))
        .collect();
    for d in &mut list {
        d.eleve_label = eleves.get(&d.eleve_id).cloned();
        d.activite_label = by_act.get(&d.activite_id).cloned();
    }
    Ok(list)
}

async fn enrich_bourses(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<EleveBourseDto>,
) -> Result<Vec<EleveBourseDto>, (StatusCode, String)> {
    let eleve_ids: Vec<Uuid> = list.iter().map(|b| b.eleve_id).collect();
    let eleves = state
        .ent
        .eleve_labels(etablissement_id, &eleve_ids)
        .await
        .map_err(map_db)?;
    let types = state
        .ent
        .list_bourse_types(etablissement_id)
        .await
        .map_err(map_db)?;
    let by_type: std::collections::HashMap<Uuid, String> = types
        .into_iter()
        .map(|t| (t.id, format!("{} — {}", t.code, t.libelle)))
        .collect();
    for b in &mut list {
        b.eleve_label = eleves.get(&b.eleve_id).cloned();
        b.bourse_type_label = by_type.get(&b.bourse_type_id).cloned();
    }
    Ok(list)
}

async fn enrich_affectations_internat(
    state: &AppState,
    etablissement_id: Uuid,
    role: &str,
    mut list: Vec<AffectationInternatDto>,
) -> Result<Vec<AffectationInternatDto>, (StatusCode, String)> {
    let eleve_ids: Vec<Uuid> = list.iter().map(|a| a.eleve_id).collect();
    let eleves = state
        .ent
        .eleve_labels(etablissement_id, &eleve_ids)
        .await
        .map_err(map_db)?;
    let by_chambre: std::collections::HashMap<Uuid, String> =
        if ent_db::PgEnt::role_can_gerer_internat(role) {
            let chambres = state
                .ent
                .list_chambres_internat_for_actor(etablissement_id, role)
                .await
                .map_err(map_db)?;
            chambres
                .into_iter()
                .map(|c| {
                    (
                        c.id,
                        format!("{} · {} (ét. {})", c.code, c.batiment, c.etage),
                    )
                })
                .collect()
        } else {
            std::collections::HashMap::new()
        };
    for a in &mut list {
        a.eleve_label = eleves.get(&a.eleve_id).cloned();
        a.chambre_label = by_chambre.get(&a.chambre_id).cloned();
    }
    Ok(list)
}

async fn enrich_inscriptions_suivi(
    state: &AppState,
    etablissement_id: Uuid,
    mut list: Vec<InscriptionSuiviDto>,
) -> Result<Vec<InscriptionSuiviDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|s| s.eleve_id).collect();
    let labels = state
        .ent
        .eleve_labels(etablissement_id, &ids)
        .await
        .map_err(map_db)?;
    for s in &mut list {
        s.eleve_label = labels.get(&s.eleve_id).cloned();
    }
    Ok(list)
}

/// Statut HTTP des refus de règle de l'internat (`None` : autre erreur, classée plus loin).
fn statut_internat(e: &domain_eleve::DomainError) -> Option<StatusCode> {
    use domain_eleve::DomainError as D;
    match e {
        D::InternatChambrePleine
        | D::InternatDejaAffecte
        | D::InternatRegimeRequis
        | D::InternatPublicIncompatible
        | D::InternatInterdictionContact
        | D::InternatChambreFermee
        | D::InternatAccessibiliteRequise
        | D::InternatCapaciteSousOccupation
        | D::InternatChambreOccupee
        | D::InternatLitIndisponible
        | D::InternatAffectationDejaClose => Some(StatusCode::CONFLICT),
        D::InternatNuitFuture
        | D::InternatMotifRequis
        | D::InternatChambreInvalide
        | D::InternatAffectationInvalide
        | D::InternatPresenceInvalide
        | D::InternatSuiviInvalide
        | D::InternatSejourInvalide
        | D::InternatRythmeInvalide
        | D::InternatZoneInvalide => Some(StatusCode::UNPROCESSABLE_ENTITY),
        _ => None,
    }
}

pub(crate) fn map_db(err: ent_db::EntDbError) -> (StatusCode, String) {
    if let Some(r) = erreur_infra(&err) {
        return r;
    }
    // Classement par VARIANTE avant toute heuristique de texte : une erreur de
    // validation métier reste un 4xx, seules les vraies pannes sont des 500.
    match &err {
        ent_db::EntDbError::Refus(nature, _) => {
            let code = match nature {
                ent_db::RefusMetier::Invalide => StatusCode::UNPROCESSABLE_ENTITY,
                ent_db::RefusMetier::Conflit => StatusCode::CONFLICT,
                ent_db::RefusMetier::Interdit => StatusCode::FORBIDDEN,
            };
            return (code, err.to_string());
        }
        ent_db::EntDbError::FichierAbsent => return (StatusCode::NOT_FOUND, err.to_string()),
        ent_db::EntDbError::LiensIndisponibles => {
            tracing::error!("liens de fichiers désactivés : ENT_OBJECT_SIGNING_KEY absente ou invalide (fail-closed)");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Liens de fichiers indisponibles.".to_string(),
            );
        }
        ent_db::EntDbError::Object(ent_db::ObjectStoreError::NotFound(_)) => {
            return (StatusCode::NOT_FOUND, "Fichier introuvable.".to_string());
        }
        ent_db::EntDbError::Object(ent_db::ObjectStoreError::InvalidKey) => {
            return (StatusCode::BAD_REQUEST, "Fichier invalide.".to_string());
        }
        // Stockage : l'échec est visible (jamais un fichier vide ni altéré servi en silence).
        ent_db::EntDbError::Object(ent_db::ObjectStoreError::Integrity) => {
            tracing::error!("intégrité d'un objet non vérifiée : lecture refusée");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Fichier indisponible : intégrité non vérifiée.".to_string(),
            );
        }
        ent_db::EntDbError::Object(ent_db::ObjectStoreError::Immutable) => {
            return (StatusCode::CONFLICT, "Fichier déjà enregistré.".to_string());
        }
        ent_db::EntDbError::Object(
            ent_db::ObjectStoreError::Backend(_) | ent_db::ObjectStoreError::Unavailable,
        ) => {
            tracing::error!("stockage d'objets indisponible");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Stockage de fichiers indisponible.".to_string(),
            );
        }
        ent_db::EntDbError::Domain(domain_eleve::DomainError::TenantMismatch) => {
            return (StatusCode::FORBIDDEN, err.to_string());
        }
        // Internat (socle 41) : refus de règle → 409 (état incompatible) ou 422 (saisie invalide).
        ent_db::EntDbError::Domain(e) if statut_internat(e).is_some() => {
            return (statut_internat(e).unwrap_or(StatusCode::BAD_REQUEST), err.to_string());
        }
        // Dossier élève (S2-1) : classement par variante, messages sans donnée personnelle.
        ent_db::EntDbError::SaisieInvalide(_) => return (StatusCode::BAD_REQUEST, err.to_string()),
        ent_db::EntDbError::ConflitDossier(_) | ent_db::EntDbError::ConflitDossierDetail(_) => {
            return (StatusCode::CONFLICT, err.to_string())
        }
        // D2 : fichier déposé refusé (message sans donnée sensible).
        ent_db::EntDbError::Fichier(e) if e.est_taille() => {
            return (StatusCode::PAYLOAD_TOO_LARGE, err.to_string());
        }
        ent_db::EntDbError::Fichier(e) if e.est_indisponibilite() => {
            tracing::error!("analyse antivirus indisponible : dépôt refusé (fail-closed)");
            return (StatusCode::SERVICE_UNAVAILABLE, err.to_string());
        }
        ent_db::EntDbError::Fichier(_) => {
            return (StatusCode::UNPROCESSABLE_ENTITY, err.to_string());
        }
        _ => {}
    }
    let msg = err.to_string();
    if msg.contains("accès refusé") || msg.contains("Forbidden") {
        (StatusCode::FORBIDDEN, msg)
    } else if msg.contains("introuvable") || msg.contains("NotFound") {
        (StatusCode::NOT_FOUND, msg)
    } else if msg.contains("déjà utilisé")
        || msg.contains("déjà existant")
        || msg.contains("déjà existante")
        || msg.contains("déjà prise")
        || msg.contains("déjà pris")
        || msg.contains("en conflit")
    {
        (StatusCode::CONFLICT, msg)
    } else if msg.contains("parts payeurs")
        || msg.contains("Identity")
        || msg.contains("motif")
        || msg.contains("période")
        || msg.contains("bulletin")
        || msg.contains("date d'effet")
        || msg.contains("valeur")
        || msg.contains("référence inconnue")
        || msg.contains("attribut")
        || msg.contains("matière")
        || msg.contains("note")
        || msg.contains("barème")
        || msg.contains("appel")
        || msg.contains("carnet")
        || msg.contains("message")
        || msg.contains("fil")
        || msg.contains("campagne")
        || msg.contains("réinscription")
        || msg.contains("reinscription")
        || msg.contains("facture")
        || msg.contains("encaissement")
        || msg.contains("échéance")
        || msg.contains("echeancier")
        || msg.contains("échéancier")
        || msg.contains("quittance")
        || msg.contains("OGEC")
        || msg.contains("ogec")
        || msg.contains("SEPA")
        || msg.contains("sepa")
        || msg.contains("IBAN")
        || msg.contains("mandat")
        || msg.contains("prélèvement")
        || msg.contains("prelevement")
        || msg.contains("rappel")
        || msg.contains("dispense")
        || msg.contains("grille repas")
        || msg.contains("signal")
        || msg.contains("badge")
        || msg.contains("passage")
        || msg.contains("invitation")
        || msg.contains("e-mail")
        || msg.contains("Email")
        || msg.contains("infirmière")
        || msg.contains("infirmerie")
        || msg.contains("fiche santé")
        || msg.contains("entretien psy")
        || msg.contains("RDV psy")
        || msg.contains("psy")
        || msg.contains("exception EDT")
        || msg.contains("calendrier EDT")
        || msg.contains("cahier de texte")
        || msg.contains("priorité ou dossier")
        || msg.contains("EDT")
        || msg.contains("internat")
        || msg.contains("chambre")
        || msg.contains("sortie")
        || msg.contains("journal")
        || msg.contains("alerte")
        || msg.contains("document")
        || msg.contains("conseil")
        || msg.contains("mention")
        || msg.contains("annonce")
        || msg.contains("retard")
        || msg.contains("compétence")
        || msg.contains("competence")
    {
        (StatusCode::BAD_REQUEST, msg)
    } else {
        match &err {
            // Toute autre erreur de domaine / d'import / de parsing est une validation
            // métier (message fixe, non technique) : 4xx, jamais 500.
            ent_db::EntDbError::Domain(_)
            | ent_db::EntDbError::Bank(_)
            | ent_db::EntDbError::EdtImport(_)
            | ent_db::EntDbError::EdtImportConflicts => (StatusCode::BAD_REQUEST, msg),
            _ => erreur_interne(&err),
        }
    }
}

async fn list_repas_activites(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<RepasActiviteDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_repas_activites_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(RepasActiviteDto::from).collect()))
}

async fn list_bourse_types(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<BourseTypeDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_bourse_types_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(BourseTypeDto::from).collect()))
}

async fn creer_bourse_type(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerBourseTypeBody>,
) -> Result<(StatusCode, Json<BourseTypeDto>), (StatusCode, String)> {
    let t = state
        .ent
        .creer_bourse_type_for_actor(
            &session.role,
            CreerBourseType {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(BourseTypeDto::from(&t))))
}

async fn affecter_bourse(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<AffecterBourseBody>,
) -> Result<(StatusCode, Json<EleveBourseDto>), (StatusCode, String)> {
    let b = state
        .ent
        .affecter_bourse_for_actor(
            &session.role,
            AffecterBourse {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                bourse_type_id: body.bourse_type_id,
                echelon: body.echelon,
                avec_procuration: body.avec_procuration,
                date_debut: body.date_debut,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_bourses(
        &state,
        session.etablissement_id,
        vec![EleveBourseDto::from(&b)],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(enriched.pop().unwrap_or_else(|| EleveBourseDto::from(&b))),
    ))
}

async fn decider_bourse(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeciderBourseBody>,
) -> Result<Json<EleveBourseDto>, (StatusCode, String)> {
    let b = state
        .ent
        .decider_bourse_for_actor(
            &session.role,
            DeciderBourse {
                etablissement_id: session.etablissement_id,
                bourse_id: id,
                accepter: body.accepter,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_bourses(
        &state,
        session.etablissement_id,
        vec![EleveBourseDto::from(&b)],
    )
    .await?;
    Ok(Json(
        enriched.pop().unwrap_or_else(|| EleveBourseDto::from(&b)),
    ))
}

async fn creer_repas_activite(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerRepasActiviteBody>,
) -> Result<(StatusCode, Json<RepasActiviteDto>), (StatusCode, String)> {
    let kind = RepasActiviteKind::parse(body.kind.trim())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "kind repas invalide".into()))?;
    let act = state
        .ent
        .creer_repas_activite_for_actor(
            &session.role,
            CreerRepasActivite {
                etablissement_id: session.etablissement_id,
                code: body.code,
                libelle: body.libelle,
                kind,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(RepasActiviteDto::from(&act))))
}

async fn list_grilles_repas(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<GrilleRepasDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_grilles_repas_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_grilles_repas(
            &state,
            session.etablissement_id,
            list.iter().map(GrilleRepasDto::from).collect(),
        )
        .await?,
    ))
}

async fn poser_grille_repas(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PoserGrilleRepasBody>,
) -> Result<(StatusCode, Json<GrilleRepasDto>), (StatusCode, String)> {
    let g = state
        .ent
        .poser_grille_repas_for_actor(
            &session.role,
            PoserGrilleRepas {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                activite_id: body.activite_id,
                jours: JoursMasque::from(&body.jours),
                date_debut: body.date_debut,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_grilles_repas(
        &state,
        session.etablissement_id,
        vec![GrilleRepasDto::from(&g)],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(enriched.pop().unwrap_or_else(|| GrilleRepasDto::from(&g))),
    ))
}

async fn list_dispenses_repas(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DispenseRepasDto>>, (StatusCode, String)> {
    // Accès via dossier (même authz lecture repas).
    let dossier = state
        .ent
        .get_dossier_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            today_utc(),
        )
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "dossier inaccessible".into()))?;
    if !dossier.sections.repas {
        return Err((StatusCode::FORBIDDEN, "section repas inaccessible".into()));
    }
    Ok(Json(
        enrich_dispenses_repas(
            &state,
            session.etablissement_id,
            dossier
                .dispenses_repas
                .iter()
                .map(DispenseRepasDto::from)
                .collect(),
        )
        .await?,
    ))
}

async fn poser_dispense_repas(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PoserDispenseRepasBody>,
) -> Result<(StatusCode, Json<DispenseRepasDto>), (StatusCode, String)> {
    let source = if session.role == "parent" || session.role == "eleve" {
        DispenseSource::Parent
    } else {
        DispenseSource::Staff
    };
    let d = state
        .ent
        .poser_dispense_repas_for_actor(
            &session.role,
            PoserDispenseRepas {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                activite_id: body.activite_id,
                jour: body.jour,
                motif: body.motif,
                source,
                actor_user_id: Some(session.user_id.to_string()),
            },
        )
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_dispenses_repas(
        &state,
        session.etablissement_id,
        vec![DispenseRepasDto::from(&d)],
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(enriched.pop().unwrap_or_else(|| DispenseRepasDto::from(&d))),
    ))
}

async fn annuler_dispense_repas(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, dispense_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DispenseRepasDto>, (StatusCode, String)> {
    let _ = id;
    let d = state
        .ent
        .annuler_dispense_repas_for_actor(
            session.etablissement_id,
            dispense_id,
            session.user_id,
            &session.role,
        )
        .await
        .map_err(map_db)?;
    if d.eleve_id != id {
        return Err((StatusCode::NOT_FOUND, "dispense introuvable".into()));
    }
    let mut enriched = enrich_dispenses_repas(
        &state,
        session.etablissement_id,
        vec![DispenseRepasDto::from(&d)],
    )
    .await?;
    Ok(Json(
        enriched.pop().unwrap_or_else(|| DispenseRepasDto::from(&d)),
    ))
}

#[derive(Debug, Deserialize)]
struct VolumeRepasQuery {
    debut: NaiveDate,
    fin: NaiveDate,
    #[serde(default = "default_midi")]
    kind: String,
}

fn default_midi() -> String {
    "midi".into()
}

async fn volume_repas_facturable(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<VolumeRepasQuery>,
) -> Result<Json<VolumeRepasDto>, (StatusCode, String)> {
    let dossier = state
        .ent
        .get_dossier_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            today_utc(),
        )
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "dossier inaccessible".into()))?;
    if !dossier.sections.repas && session.role != "compta" {
        return Err((StatusCode::FORBIDDEN, "section repas inaccessible".into()));
    }
    let kind = RepasActiviteKind::parse(&q.kind)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "kind repas invalide".into()))?;
    let n = state
        .ent
        .compter_repas_facturables_eleve(session.etablissement_id, id, kind, q.debut, q.fin)
        .await
        .map_err(map_db)?;
    Ok(Json(VolumeRepasDto {
        eleve_id: id,
        kind: kind.as_str().to_string(),
        debut: q.debut,
        fin: q.fin,
        jours_facturables: n,
    }))
}

#[cfg(all(test, feature = "labo"))]
mod tests {
    use super::gate_open;

    #[test]
    fn gate_ouverte_seulement_si_env_non_prod_explicite() {
        for v in ["lab", "labo", "dev", "local", "test", "Labo", "  LAB  "] {
            assert!(gate_open(Some(v), None), "{v:?} doit ouvrir");
        }
    }

    #[test]
    fn gate_fermee_si_absent_vide_ou_inconnu() {
        assert!(!gate_open(None, None));
        assert!(!gate_open(Some(""), None));
        assert!(!gate_open(Some("   "), None));
        assert!(!gate_open(Some("staging"), None));
        assert!(!gate_open(Some("laboratoire"), Some("0")));
    }

    #[test]
    fn gate_ferme_en_prod_sauf_derogation() {
        assert!(!gate_open(Some("prod"), None));
        assert!(!gate_open(Some("PRODUCTION"), Some("0")));
        assert!(gate_open(Some("prod"), Some("1")));
        assert!(gate_open(None, Some("1")));
    }
}

/// Panne d'infrastructure (base, migration) → 500 générique ; le détail va
/// uniquement dans le journal. À appeler **avant** toute classification par
/// sous-chaîne du message : une panne dont le texte contient « note » ou
/// « psy » ne doit jamais devenir un 400, ni une 404/401.
pub(crate) fn erreur_infra(err: &ent_db::EntDbError) -> Option<(StatusCode, String)> {
    let infra = matches!(
        err,
        ent_db::EntDbError::Sqlx(_)
            | ent_db::EntDbError::Migrate(_)
            | ent_db::EntDbError::Domain(domain_eleve::DomainError::Infrastructure)
            | ent_db::EntDbError::Event(ent_db::EventBusError::Infrastructure)
    );
    if matches!(
        err,
        ent_db::EntDbError::Domain(domain_eleve::DomainError::Reessayer)
    ) {
        return Some((
            StatusCode::SERVICE_UNAVAILABLE,
            "Opération concurrente, veuillez réessayer.".to_string(),
        ));
    }
    if infra {
        tracing::error!(error = %err, "erreur d'infrastructure");
        Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Erreur interne.".to_string(),
        ))
    } else {
        None
    }
}

/// Erreur interne inattendue : détail au journal, message générique au client.
fn erreur_interne(err: &dyn std::fmt::Display) -> (StatusCode, String) {
    tracing::error!(error = %err, "erreur interne");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Erreur interne.".to_string(),
    )
}

// --- `Debug` masqué des corps / DTO portant secrets (voir `masque.rs`) ------
debug_masque!(PersonnelProvisionneDto { user_id, role, invitation_envoyee } masque { email });
debug_masque!(MotDePasseReinitialiseDto { user_id, lien_envoye } masque { });
#[cfg(feature = "labo")]
debug_masque!(RegisterBody { etablissement_id, role } masque { email, password });
debug_masque!(LoginBody { etablissement_id } masque { email, password });
debug_masque!(AuthSessionDto { user_id, etablissement_id, role, expires_at } masque { token, email });
debug_masque!(PasskeyChallengeDto { expires_at } masque { token, options });
debug_masque!(PasskeyRegisterStartDto { } masque { token, options });
debug_masque!(PasskeyRegisterFinishBody { name } masque { token, credential, current_password });
debug_masque!(PasskeyLoginFinishBody { } masque { token, credential });
debug_masque!(ChangePasswordBody { } masque { current_password, new_password });
debug_masque!(ResetPasswordBody { } masque { token, new_password });
debug_masque!(RegisterPushDeviceBody { platform, app_id } masque { token });
debug_masque!(OidcCompleteBody { } masque { code, state });
debug_masque!(OidcUpsertBody { slug, issuer } masque { client_id, client_secret });

/// Trace un téléchargement ou un export sensible (journal en ajout seul).
pub(crate) async fn tracer_acces(
    state: &AppState,
    session: &EntSession,
    domaine: &str,
    action: &str,
    libelle: &str,
) {
    state
        .ent
        .tracer_acces_sensible(
            session.etablissement_id,
            session.user_id,
            &session.role,
            None,
            domaine,
            action,
            libelle,
        )
        .await;
}

// ───────────── Socle 1 : appareils connectés et politique de sécurité ─────────────

#[derive(Serialize)]
pub struct SessionAppareilDto {
    pub id: Uuid,
    pub client: String,
    pub assurance: String,
    pub appareil: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// La session qui fait la requête.
    pub courante: bool,
}

/// Appareils connectés de la personne, **dans cet établissement** seulement
/// (D-013 : aucune visibilité entre établissements).
async fn list_sessions_appareils(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<SessionAppareilDto>>, (StatusCode, String)> {
    let rows = state
        .ent
        .sessions_actives_for_actor(session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        rows.into_iter()
            .filter(|r| r.etablissement_id == session.etablissement_id)
            .map(|r| SessionAppareilDto {
                id: r.session_id,
                courante: r.session_id == session.session_id,
                client: r.client,
                assurance: r.assurance,
                appareil: r.appareil,
                created_at: r.created_at,
                last_seen_at: r.last_seen_at,
                expires_at: r.expires_at,
            })
            .collect(),
    ))
}

/// « Déconnecter cet appareil » (une de ses propres sessions).
async fn revoquer_session_appareil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let ok = state
        .ent
        .revoquer_session_for_actor(session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    if ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "session introuvable".into()))
    }
}

#[derive(Serialize, Deserialize)]
pub struct PolitiqueSecuriteDto {
    /// Rôles pour lesquels la passkey est obligatoire (ex. `["prof", "cpe"]`).
    pub passkey_obligatoire_roles: Vec<String>,
}

async fn get_politique_securite(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<PolitiqueSecuriteDto>, (StatusCode, String)> {
    if !matches!(session.role_parsed(), Some(ent_auth::Role::Admin | ent_auth::Role::Direction)) {
        return Err((StatusCode::FORBIDDEN, "réservé à la direction".into()));
    }
    let p = state
        .ent
        .politique_securite(session.etablissement_id)
        .await
        .map_err(map_db)?;
    Ok(Json(PolitiqueSecuriteDto {
        passkey_obligatoire_roles: p.passkey_obligatoire_roles,
    }))
}

async fn put_politique_securite(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<PolitiqueSecuriteDto>,
) -> Result<Json<PolitiqueSecuriteDto>, (StatusCode, String)> {
    let p = state
        .ent
        .definir_politique_securite_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &body.passkey_obligatoire_roles,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(PolitiqueSecuriteDto {
        passkey_obligatoire_roles: p.passkey_obligatoire_roles,
    }))
}

// ───────────── Socle 1 (C2, D-012) : établissement déduit de l'adresse ─────────────

#[derive(Debug, Serialize)]
pub struct EtablissementPublicDto {
    pub id: Uuid,
    pub slug: String,
    pub nom: String,
}

#[derive(Debug, Deserialize)]
pub struct EtablissementFicheBody {
    pub slug: String,
    pub nom: String,
}

#[derive(Debug, Deserialize)]
pub struct EtablissementDomaineBody {
    pub domaine: String,
}

#[derive(Debug, Serialize)]
pub struct EtablissementDomainesDto {
    pub etablissement: Option<EtablissementPublicDto>,
    pub domaines: Vec<String>,
}

impl From<ent_db::EtablissementInfo> for EtablissementPublicDto {
    fn from(e: ent_db::EtablissementInfo) -> Self {
        Self {
            id: e.id,
            slug: e.slug,
            nom: e.nom,
        }
    }
}

/// Public : établissement servi par l'adresse (page de connexion, sans
/// `etablissement_id` à saisir). 404 si l'adresse ne désigne aucun établissement.
async fn etablissement_de_l_hote(
    State(state): State<AppState>,
    tenant: TenantHote,
) -> Result<Json<EtablissementPublicDto>, (StatusCode, String)> {
    let introuvable = || {
        (
            StatusCode::NOT_FOUND,
            "établissement inconnu pour cette adresse".to_string(),
        )
    };
    let id = tenant.etablissement_id.ok_or_else(introuvable)?;
    let info = state
        .ent
        .etablissement_info(id)
        .await
        .map_err(map_db)?
        .ok_or_else(introuvable)?;
    Ok(Json(info.into()))
}

/// Domaines de l'établissement de la session (Admin).
async fn list_etablissement_domaines(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<EtablissementDomainesDto>, (StatusCode, String)> {
    let domaines = state
        .ent
        .domaines_for_actor(session.etablissement_id, &session.role)
        .await
        .map_err(map_db)?;
    let etablissement = state
        .ent
        .etablissement_info(session.etablissement_id)
        .await
        .map_err(map_db)?
        .map(EtablissementPublicDto::from);
    Ok(Json(EtablissementDomainesDto {
        etablissement,
        domaines,
    }))
}

/// Déclare / renomme l'établissement de la session (Admin, journalisé).
async fn put_etablissement_fiche(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<EtablissementFicheBody>,
) -> Result<Json<EtablissementPublicDto>, (StatusCode, String)> {
    let info = state
        .ent
        .declarer_etablissement_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &body.slug,
            &body.nom,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(info.into()))
}

/// Rattache un domaine à l'établissement de la session (Admin, journalisé).
async fn post_etablissement_domaine(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<EtablissementDomaineBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let domaine = state
        .ent
        .ajouter_domaine_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &body.domaine,
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "domaine": domaine })),
    ))
}

/// Retire un domaine de l'établissement de la session (Admin, journalisé).
async fn delete_etablissement_domaine(
    State(state): State<AppState>,
    session: EntSession,
    Path(domaine): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let retire = state
        .ent
        .retirer_domaine_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &domaine,
        )
        .await
        .map_err(map_db)?;
    if retire {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "domaine introuvable".into()))
    }
}
