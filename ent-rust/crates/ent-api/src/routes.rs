use crate::auth::EntSession;
use crate::ical::{render_edt_calendar, EdtOccurrence};
use crate::dto::{
    AbsenceDto, AbsentDto, AppelDto, AppelLigneDto, CampagneDto, CarnetEntreeDto, ChangeClasseBody,
    ChangeMefBody, ChangeRegimeBody, ChangeSiteBody, ChoisirOptionBody, CreateEleveBody,
    CreateMatiereBody, CreerCarnetBody,
    CreerDevoirBody, CreerFactureBody, CreerNotePeriodeBody, DeclareAbsenceBody, DevoirDto,
    EcheanceDto, EcheancierPartsBody, EdtCreneauDto, EdtExceptionDto, EffectifDto, EleveDto,
    EncaissementDto,
    EncaisserBody, EnfantFamilleDto, FamilleResumeDto, FactureDetailDto, FactureDto,
    DecisionAbsenceBody, FactureLigneDto, FilDetailDto, JustifierAbsenceBody, MarquerAppelLigneBody,
    MatiereDto,
    CandidatsMonteeLotDto, ConfigurerAcompteBody, CreerMandatSepaBody, DeposerDocReinscriptionBody,
    AttacherPieceBody, ChangerStatutRdvPsyBody, CompteProvisionneDto, CreerEntretienPsyBody,
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
    PublierBulletinBody, SaisirAppreciationBody, SaisirNoteBody,
    ScolariteAnneeDto, ScanBadgeRepasBody, SetNotePeriodeStatutBody, SignalAbsenceDto, SortirBody,
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
    DeciderDossierConseilBody, DocASignerDto, PublierDocASignerBody,
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
    RattacherResponsableFoyerBody, SetTelephoneBody, UpdateLienBody, UpdateLiensEleveBody,
    UpdateIdentiteBody,
};
use crate::ui;
use crate::AppState;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::{NaiveDate, NaiveTime, Utc};
use domain_eleve::{
    annee_scolaire_de, annees_continuum, render_facture_pdf, render_quittance_pdf, reste_a_payer,
    today_utc, CarnetCategorie, ChangeClasse, ChangeMef, ChangeRegime, ChangeSite, ChoisirOptionN1,
    ConfigurerAcompteCampagne, ConfirmerReinscription, CreateEleve, CreerCarnetEntree, CreerDevoir,
    CreerFacture, CreerMandatSepa, CreerNotePeriode, DeclareAbsence, DocTiroir, EcheanceCmd,
    SaisirAppreciation,
    EncaisserFacture, ExporterOgec, LigneFactureCmd, MonteeLot, MonteeRapide, MoyenPaiement,
    NotePeriodeStatut, OptionNature, OuvrirCampagneReinscription, OuvrirFil, PlanifierPrelevement,
    plan_sepa_crediteur, ArchiverFil, AttacherPiece, ClasserFil, ConfigurerEdtCalendrierAb,
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
    AjouterDossierConseil, CreerConseilClasse, DeciderDossierConseil, PublierDocASigner,
    SignerDocASigner, CreerInfoCompChamp, UpsertInfoCompValeur,
    CreerPeriodeVacances, RechercherHomonymes, UpsertMenuCantine,
    CreerTauxTva, UpsertQuotientFoyer, CreerSalle, UpsertParamEtab,
    CreerFiliere, PoserVoeu, UpsertInscriptionSuivi, ExporterLsl, ExporterOnde,
    CreerTarifConso, ExporterNotanet, DeclarerAbsenceRh, AffecterEdtCreneau,
    CreerEditionVs, ExporterLsun, ExporterParcoursup, CreerConventionStage, CreerEntreprise,
    CreerEdtContrainte, EdtContrainteKind, UpdateIdentite,
};
use base64::Engine;
use ent_db::{Appel, AppelLigne, CreateMatiere, LABO_PASSWORD, LigneEtat, SaisirNote};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(ui_page))
        .route("/login", get(ui_page))
        .route("/app", get(ui_page))
        .route("/app/{*rest}", get(ui_page))
        .route("/famille", get(ui_page))
        .route("/famille/{*rest}", get(ui_page))
        .route("/eleve", get(ui_page))
        .route("/eleve/{*rest}", get(ui_page))
        .route("/health", get(health))
        .route("/labo/bootstrap", post(labo_bootstrap))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/devices/push", post(register_push_device))
        .route("/eleves", post(create_eleve))
        .route("/eleves/search", get(search_eleves))
        .route("/eleves/labels", get(eleve_labels_batch))
        .route("/personnel/search", get(search_personnel))
        .route("/personnel/{id}", get(get_personnel))
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
        .route("/eleves/{id}/dossier", get(get_dossier))
        .route(
            "/eleves/{id}/documents",
            get(list_documents_meta_eleve).post(deposer_document_eleve),
        )
        .route("/documents/{id}/fichier", get(telecharger_document_fichier))
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
            get(get_photo_eleve).post(poser_photo_eleve),
        )
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
        .route("/classes/{classe}/effectif", get(effectif_classe))
        .route("/matieres", get(list_matieres).post(create_matiere))
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
            get(list_pai_pieces).post(attacher_ordonnance_pai),
        )
        .route("/pai/pieces/{id}", get(telecharger_pai_piece))
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
        .route("/messages/fils/{id}/classer", post(classer_fil))
        .route("/messages/modeles", get(list_msg_modeles).post(creer_msg_modele))
        .route("/messages/{id}/pieces", post(attacher_piece))
        .route("/messages/pieces/{id}", get(telecharger_piece))
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
            post(famille_deposer_doc_reinscription),
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
        .route("/facturation/sepa/export", get(export_sepa))
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
        )
}

async fn ui_page() -> axum::response::Html<&'static str> {
    ui::app_html()
}

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

#[derive(Debug, Serialize)]
struct LaboBootstrapDto {
    etablissement_id: Uuid,
    password: String,
    personas: Vec<LaboPersonaDto>,
    eleves: Vec<LaboEleveDto>,
    personnel: Vec<LaboPersonnelDto>,
    foyer_lea_id: Uuid,
    foyer_tom_id: Uuid,
}

#[derive(Debug, Serialize)]
struct PersonnelDto {
    user_id: Uuid,
    email: String,
    role: String,
    label: String,
    telephone: Option<String>,
}

/// Seed une école dense (local only) et renvoie les comptes pour l'UI labo.
async fn labo_bootstrap(
    State(state): State<AppState>,
) -> Result<Json<LaboBootstrapDto>, (StatusCode, String)> {
    let cast = state
        .ent
        .seed_labo_cast()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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

    Ok(Json(LaboBootstrapDto {
        etablissement_id: cast.etablissement_id,
        password: LABO_PASSWORD.to_string(),
        personas,
        eleves,
        personnel,
        foyer_lea_id: cast.foyer_lea_id,
        foyer_tom_id: cast.foyer_tom_id,
    }))
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

async fn search_eleves(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<EleveDto>>, (StatusCode, String)> {
    let needle = q.q.unwrap_or_default();
    let rows = state
        .ent
        .search_eleves(session.etablissement_id, &needle, 50)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows.iter().map(EleveDto::from).collect()))
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
    let ids: Vec<Uuid> = q
        .ids
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| Uuid::parse_str(s.trim()).ok())
        .collect();
    let map = state
        .ent
        .eleve_labels(session.etablissement_id, &ids)
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
        telephone: p.telephone,
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows.into_iter().map(personnel_dto).collect()))
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
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
    let mut value = serde_json::to_value(&dossier)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "sanctions".into(),
            serde_json::to_value(sanctions)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        );
        obj.insert(
            "incidents".into(),
            serde_json::to_value(incidents)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        );
        obj.insert(
            "carnet".into(),
            serde_json::to_value(carnet)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        );
        obj.insert(
            "grille_repas".into(),
            serde_json::to_value(grille_repas)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        );
        obj.insert(
            "dispenses_repas".into(),
            serde_json::to_value(dispenses_repas)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        );
        obj.insert(
            "bourses".into(),
            serde_json::to_value(bourses)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
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
                storage_key: body.storage_key,
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
    let mime = doc
        .mime_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    let filename = {
        let base = doc.titre.replace('/', "_").replace('\\', "_");
        let ext = match mime {
            "application/pdf" => ".pdf",
            "image/png" => ".png",
            "image/jpeg" | "image/jpg" => ".jpg",
            "text/plain" => ".txt",
            _ => "",
        };
        if base.contains('.') {
            base
        } else {
            format!("{base}{ext}")
        }
    };
    attachment_response_dyn(bytes, &filename, mime)
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
        .list_grants_pending_for_actor(session.etablissement_id, &session.role)
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
    Json(serde_json::json!({ "ok": true, "service": "ent-api" }))
}

#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    pub etablissement_id: Uuid,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "staff".into()
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
    pub etablissement_id: Uuid,
}

#[derive(Debug, Serialize)]
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
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let user = state
        .ent
        .register_user(
            &body.email,
            &body.password,
            body.display_name.as_deref(),
            body.etablissement_id,
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

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> Result<Json<AuthSessionDto>, (StatusCode, String)> {
    let session = state
        .ent
        .login(&body.email, &body.password, body.etablissement_id)
        .await
        .map_err(map_auth)?;
    Ok(Json(AuthSessionDto {
        token: session.token,
        user_id: session.user_id,
        email: session.email,
        etablissement_id: session.etablissement_id,
        role: session.role,
        expires_at: session.expires_at,
    }))
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

#[derive(Debug, Deserialize)]
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
    Ok(Json(MeDto {
        user_id: session.user_id,
        email: session.email,
        etablissement_id: session.etablissement_id,
        role: session.role,
        eleve_id,
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
    let eleve = state
        .ent
        .get_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "élève introuvable".into()))?;
    Ok(Json(EleveDto::from(&eleve)))
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
    let list = state
        .ent
        .list_absences_a_traiter(session.etablissement_id, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_absences_eleve(
            &state,
            session.etablissement_id,
            list.iter().map(AbsenceDto::from).collect(),
        )
        .await?,
    ))
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
    let list = state
        .ent
        .list_signaux_absences_ouverts(session.etablissement_id)
        .await
        .map_err(map_db)?;
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
    session.require_saisir_sanctions()?;
    let jour = q.jour.unwrap_or_else(today_utc);
    let list = state
        .ent
        .list_commissions_for_actor(session.etablissement_id, &session.role, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(CommissionDto::from).collect()))
}

async fn creer_commission(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerCommissionBody>,
) -> Result<(StatusCode, Json<CommissionDto>), (StatusCode, String)> {
    session.require_saisir_sanctions()?;
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
    session.require_saisir_sanctions()?;
    let list = state
        .ent
        .list_dossiers_commission_for_actor(session.etablissement_id, &session.role, id)
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
    session.require_saisir_sanctions()?;
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
    session.require_saisir_sanctions()?;
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
    session.require_declarer_incident()?;
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
    session.require_saisir_sanctions()?;
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
    session.require_saisir_sanctions()?;
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
    session.require_declarer_incident()?;
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
    session.require_saisir_sanctions()?;
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
    session.require_saisir_sanctions()?;
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
    let jour = q.jour.unwrap_or_else(today_utc);
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
    let p = state
        .ent
        .pointer_presence_internat_for_actor(
            &session.role,
            PointerPresenceInternat {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                jour: body.jour.unwrap_or_else(today_utc),
                statut: body.statut,
                note: body.note,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(PresenceInternatDto::from(&p)))
}

async fn list_presences_internat(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Result<Json<Vec<PresenceInternatDto>>, (StatusCode, String)> {
    let jour = q.jour.unwrap_or_else(today_utc);
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
    let jour = q.jour.unwrap_or_else(today_utc);
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
    Ok((StatusCode::CREATED, Json(SuiviInternatDto::from(&s))))
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
        .list_dispenses_vs_jour_for_actor(session.etablissement_id, &session.role, jour)
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

async fn list_conseils_classe(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ClasseQuery>,
) -> Result<Json<Vec<ConseilClasseDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_conseils_classe_for_actor(
            session.etablissement_id,
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
        .list_dossiers_conseil_for_actor(session.etablissement_id, &session.role, id)
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

async fn decider_dossier_conseil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<DeciderDossierConseilBody>,
) -> Result<Json<ConseilClasseDossierDto>, (StatusCode, String)> {
    let d = state
        .ent
        .decider_dossier_conseil_for_actor(
            &session.role,
            DeciderDossierConseil {
                etablissement_id: session.etablissement_id,
                dossier_id: id,
                appreciation: body.appreciation,
                mention: body.mention,
                decision_texte: body.decision_texte,
                visible_famille: body.visible_famille,
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
    Ok(Json(
        enriched
            .pop()
            .unwrap_or_else(|| ConseilClasseDossierDto::from(&d)),
    ))
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
        enrich_absences_rh(&state, list.iter().map(AbsenceRhDto::from).collect()).await?,
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
    let mut enriched = enrich_absences_rh(&state, vec![AbsenceRhDto::from(&a)]).await?;
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
    let mut enriched = enrich_absences_rh(&state, vec![AbsenceRhDto::from(&a)]).await?;
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
    let saved = state
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
            body.password.as_deref(),
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
        .list_trombi_classe_for_actor(session.etablissement_id, &session.role, &classe, jour)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(TrombiEntreeDto::from).collect()))
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
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
            ExporterLsl {
                etablissement_id: session.etablissement_id,
                periode_id: q.periode_id,
                classe: Some(q.classe),
                jour_reference: q.jour.unwrap_or_else(today_utc),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-lsl.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
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
            ExporterLsun {
                etablissement_id: session.etablissement_id,
                periode_id: q.periode_id,
                classe: Some(q.classe),
                jour_reference: q.jour.unwrap_or_else(today_utc),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-lsun.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
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
            ExporterOnde {
                etablissement_id: session.etablissement_id,
                classe: q.classe,
                jour_reference: q.jour.unwrap_or_else(today_utc),
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-onde.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
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
            ExporterNotanet {
                etablissement_id: session.etablissement_id,
                periode_id: q.periode_id,
                classe: q.classe,
            },
            q.jour.unwrap_or_else(today_utc),
        )
        .await
        .map_err(map_db)?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-notanet.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
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
            ExporterParcoursup {
                etablissement_id: session.etablissement_id,
                annee_scolaire: q.annee_scolaire,
                classe: q.classe,
            },
            q.jour.unwrap_or_else(today_utc),
        )
        .await
        .map_err(map_db)?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"export-parcoursup.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
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
    Ok(Json(EffectifDto {
        classe,
        jour: q.jour,
        eleves: eleves.iter().map(EleveDto::from).collect(),
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

async fn saisir_note(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<SaisirNoteBody>,
) -> Result<(StatusCode, Json<NoteDto>), (StatusCode, String)> {
    session.require_saisir_notes()?;
    let note = state
        .ent
        .saisir_note(SaisirNote {
            etablissement_id: session.etablissement_id,
            eleve_id: id,
            matiere_id: body.matiere_id,
            valeur: body.valeur,
            sur: body.sur.unwrap_or(20.0),
            date_note: body.date_note,
            auteur_user_id: session.user_id,
            libelle: body.libelle,
            devoir_id: body.devoir_id,
            periode_id: body.periode_id,
        })
        .await
        .map_err(map_db)?;
    let mut enriched = enrich_notes(
        &state,
        session.etablissement_id,
        vec![NoteDto::from(&note)],
    )
    .await?;
    Ok((StatusCode::CREATED, Json(enriched.remove(0))))
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

async fn publier_bulletin(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<PublierBulletinBody>,
) -> Result<(StatusCode, Json<DocumentMetaDto>), (StatusCode, String)> {
    let doc = state
        .ent
        .publier_bulletin_for_actor(
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

async fn set_note_periode_statut(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<SetNotePeriodeStatutBody>,
) -> Result<Json<NotePeriodeDto>, (StatusCode, String)> {
    session.require_manage_matieres()?;
    let statut = NotePeriodeStatut::parse(&body.statut).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "statut période invalide (ouverte|fermee|partielle_appreciations)".into(),
        )
    })?;
    let periode = state
        .ent
        .set_note_periode_statut(
            session.etablissement_id,
            id,
            statut,
            Some(session.user_id.to_string()),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(NotePeriodeDto::from(&periode)))
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
        .list_devoirs_for_actor(
            session.etablissement_id,
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
        .creer_devoir(CreerDevoir {
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
        enrich_edt_creneaux(&state, list.iter().map(EdtCreneauDto::from).collect()).await?,
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
        enrich_edt_creneaux(&state, list.iter().map(EdtCreneauDto::from).collect()).await?,
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
        enrich_edt_creneau(&state, EdtCreneauDto::from(&c)).await?,
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
        enrich_edt_contraintes(&state, list.iter().map(EdtContrainteDto::from).collect()).await?,
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
        enrich_edt_contraintes(&state, vec![EdtContrainteDto::from(&c)]).await?;
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
        enrich_edt_creneaux(&state, list.iter().map(EdtCreneauDto::from).collect()).await?,
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
        .get_appel(session.etablissement_id, id)
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
        .marquer_appel_ligne(session.etablissement_id, id, body.eleve_id, etat)
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
            Some(session.actor_user_id),
        )
        .await
        .map_err(map_db)?;
    let lignes = state
        .ent
        .get_appel(session.etablissement_id, id)
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
    session.require_ecrire_carnet()?;
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
    session.require_saisir_infirmerie()?;
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
    session.require_saisir_infirmerie()?;
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
    session.require_saisir_psy()?;
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
    session.require_saisir_psy()?;
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
    session.require_saisir_psy()?;
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
    session.require_saisir_psy()?;
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
) -> Result<Json<Vec<MsgFilDto>>, (StatusCode, String)> {
    let list = state
        .ent
        .list_fils_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.eleve_id,
            q.archives,
            q.dossier.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        enrich_msg_fils(
            &state,
            session.etablissement_id,
            session.user_id,
            list.iter().map(MsgFilDto::from).collect(),
        )
        .await?,
    ))
}

async fn ouvrir_fil(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<OuvrirFilBody>,
) -> Result<(StatusCode, Json<FilDetailDto>), (StatusCode, String)> {
    let (fil, msg) = state
        .ent
        .ouvrir_fil_for_actor(
            &session.role,
            OuvrirFil {
                etablissement_id: session.etablissement_id,
                eleve_id: body.eleve_id,
                sujet: body.sujet,
                premier_message: body.message,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(FilDetailDto {
            fil: MsgFilDto::from(&fil),
            messages: enrich_messages(&state, vec![MessageDto::from(&msg)]).await?,
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
            messages: enrich_messages(&state, vec![MessageDto::from(&msg)]).await?,
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
        .list_msg_modeles_for_actor(session.etablissement_id, &session.role, !q.tous)
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
        messages: enrich_messages(&state, msgs.iter().map(MessageDto::from).collect()).await?,
    }))
}

async fn repondre_fil(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<RepondreFilBody>,
) -> Result<(StatusCode, Json<MessageDto>), (StatusCode, String)> {
    let msg = state
        .ent
        .repondre_fil_for_actor(
            &session.role,
            RepondreFil {
                etablissement_id: session.etablissement_id,
                fil_id: id,
                corps: body.corps,
                actor_user_id: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((
        StatusCode::CREATED,
        Json(enrich_message(&state, MessageDto::from(&msg)).await?),
    ))
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
            "storage_key": doc.storage_key,
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

fn map_auth(err: ent_db::EntDbError) -> (StatusCode, String) {
    let msg = err.to_string();
    if msg.contains("déjà") {
        (StatusCode::CONFLICT, msg)
    } else if msg.contains("trop court") || msg.contains("e-mail vide") {
        (StatusCode::BAD_REQUEST, msg)
    } else if msg.contains("membre") {
        (StatusCode::FORBIDDEN, msg)
    } else {
        (StatusCode::UNAUTHORIZED, msg)
    }
}

async fn list_factures(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<FactureDto>>, (StatusCode, String)> {
    if session.role == "parent" {
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
    Ok(Json(list.iter().map(FactureDto::from).collect()))
}

async fn list_factures_famille(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<FactureDto>>, (StatusCode, String)> {
    if session.role != "parent" {
        return Err((StatusCode::FORBIDDEN, "réservé aux parents".into()));
    }
    let list = state
        .ent
        .list_factures_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(FactureDto::from).collect()))
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

fn attachment_response(
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
struct SepaExportQuery {
    debut: NaiveDate,
    fin: NaiveDate,
    /// `csv` (défaut) ou `pain008`
    format: Option<String>,
    cdtr_nom: Option<String>,
    cdtr_iban: Option<String>,
    cdtr_bic: Option<String>,
    cdtr_ics: Option<String>,
}

async fn export_sepa(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<SepaExportQuery>,
) -> Result<Response, (StatusCode, String)> {
    let format = q.format.as_deref().unwrap_or("csv");
    let crediteur = if matches!(
        format.trim().to_ascii_lowercase().as_str(),
        "pain008" | "pain.008" | "xml"
    ) {
        let nom = q.cdtr_nom.as_deref().unwrap_or("Etablissement");
        let iban = q
            .cdtr_iban
            .as_deref()
            .ok_or((StatusCode::BAD_REQUEST, "cdtr_iban requis pour pain.008".into()))?;
        let bic = q
            .cdtr_bic
            .as_deref()
            .ok_or((StatusCode::BAD_REQUEST, "cdtr_bic requis pour pain.008".into()))?;
        let ics = q
            .cdtr_ics
            .as_deref()
            .ok_or((StatusCode::BAD_REQUEST, "cdtr_ics requis pour pain.008".into()))?;
        Some(
            plan_sepa_crediteur(nom, iban, bic, ics).map_err(|e| {
                (StatusCode::BAD_REQUEST, e.to_string())
            })?,
        )
    } else {
        None
    };
    let (bytes, kind) = state
        .ent
        .export_sepa_for_actor(
            session.etablissement_id,
            &session.role,
            q.debut,
            q.fin,
            session.user_id,
            format,
            crediteur,
        )
        .await
        .map_err(map_db)?;
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
    let (prev, _, _, _) = state
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
    mut list: Vec<EdtCreneauDto>,
) -> Result<Vec<EdtCreneauDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|c| c.prof_user_id).collect();
    let labels = state.ent.user_labels(&ids).await.map_err(map_db)?;
    for c in &mut list {
        c.prof_label = labels.get(&c.prof_user_id).cloned();
    }
    Ok(list)
}

async fn enrich_edt_creneau(
    state: &AppState,
    mut c: EdtCreneauDto,
) -> Result<EdtCreneauDto, (StatusCode, String)> {
    let labels = state
        .ent
        .user_labels(&[c.prof_user_id])
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
    mut list: Vec<MessageDto>,
) -> Result<Vec<MessageDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|m| m.auteur_user_id).collect();
    let labels = state.ent.user_labels(&ids).await.map_err(map_db)?;
    for m in &mut list {
        m.auteur_label = labels.get(&m.auteur_user_id).cloned();
    }
    Ok(list)
}

async fn enrich_message(
    state: &AppState,
    mut m: MessageDto,
) -> Result<MessageDto, (StatusCode, String)> {
    let labels = state
        .ent
        .user_labels(&[m.auteur_user_id])
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
    let labels = state.ent.user_labels(&user_ids).await.map_err(map_db)?;
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
        .user_labels(&[g.grantee_user_id])
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
    let labels = state.ent.user_labels(&auteurs).await.map_err(map_db)?;
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
        .user_labels(&remplacant_ids)
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
    mut list: Vec<EdtContrainteDto>,
) -> Result<Vec<EdtContrainteDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().filter_map(|c| c.prof_user_id).collect();
    let labels = state.ent.user_labels(&ids).await.map_err(map_db)?;
    for c in &mut list {
        c.prof_label = c.prof_user_id.and_then(|id| labels.get(&id).cloned());
    }
    Ok(list)
}

async fn enrich_absences_rh(
    state: &AppState,
    mut list: Vec<AbsenceRhDto>,
) -> Result<Vec<AbsenceRhDto>, (StatusCode, String)> {
    let ids: Vec<Uuid> = list.iter().map(|a| a.user_id).collect();
    let labels = state.ent.user_labels(&ids).await.map_err(map_db)?;
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
    let labels = state.ent.user_labels(&user_ids).await.map_err(map_db)?;
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
    let users = state.ent.user_labels(&user_ids).await.map_err(map_db)?;
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
    let users = state.ent.user_labels(&user_ids).await.map_err(map_db)?;
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

fn map_db(err: ent_db::EntDbError) -> (StatusCode, String) {
    let msg = err.to_string();
    if msg.contains("accès refusé") || msg.contains("Forbidden") {
        (StatusCode::FORBIDDEN, msg)
    } else if msg.contains("introuvable") || msg.contains("NotFound") {
        (StatusCode::NOT_FOUND, msg)
    } else if msg.contains("parts payeurs")
        || msg.contains("Identity")
        || msg.contains("motif")
        || msg.contains("période")
        || msg.contains("bulletin")
        || msg.contains("date d'effet")
        || msg.contains("valeur")
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
    {
        (StatusCode::BAD_REQUEST, msg)
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, msg)
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
