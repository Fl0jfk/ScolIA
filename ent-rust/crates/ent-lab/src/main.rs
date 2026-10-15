//! Labo exploratoire : une école dense joue des vies divergentes sur Postgres local.
//!
//! Usage :
//!   set ENT_DATABASE_URL=postgres://scola:scola_dev_pwd@127.0.0.1:5432/scola
//!   cargo run -p ent-lab
//!
//! Objectif : faire agir parents / élèves / staff dans plusieurs directions,
//! constater ce qui tient, et **lister les trous métier** découverts en jouant.
//! Pas de script figé « comme dans l'exemple oral ». Pas de prod.

use chrono::{Duration, NaiveDate, NaiveTime, Timelike};
use domain_eleve::{
    annee_scolaire_de, annees_continuum, lundi_iso, today_utc, AbsenceStatut, AffecterBourse,
    AnnulerSanction, BourseStatus, ConvoquerCommission, CreerBourseType, CreerCommissionVs,
    DeciderBourse, DeciderCommissionDossier, DecisionCommission,
    ArchiverFil, AttacherPiece, CarnetCategorie, SignalAbsenceStatut, ChoisirOptionN1, ClasserFil,
    ConfigurerAcompteCampagne, ConfigurerEdtCalendrierAb, ConfirmerReinscription,
    CreerCarnetEntree, CreerDevoir, CreerFacture, CreerMsgModele, CreerNotePeriode,
    SaisirAppreciation,
    CreerEntretienPsy, CreerPassageInfirmerie, CreerRdvPsy, DeciderGrant, DeclarerIncident,
    AttacherOrdonnancePai, DeclareAbsence, UpsertDossierPsy, UpsertFicheSante, UpsertPai,
    SignerPaiFamille, ChangerStatutRdvPsy, RdvPsyStatut, DemanderGrant,
    DispenseSource, EcheanceCouverture, EncaisserFacture, EntretienPsyKind, ExporterVolumesRepas,
    FactureStatut, InviterCompteEleve, JoursMasque, LigneFactureCmd, MonteeRapide, MoyenPaiement,
    MsgPriorite, NotePeriodeStatut, OptionNature, OuvrirCampagneReinscription, OuvrirFil,
    PassageInfirmerieMotif, PoserDispenseRepas, PoserEdtException, PoserGrilleRepas,
    PrononcerSanction, RepasActiviteKind, RepondreFil, SanctionStatut, ScolariteAnneeStatut,
    ScolariteSource, CreerCahierTexte, EdtExceptionKind, EdtSemaineAb, NATURE_CANTINE_BADGE,
    NATURE_REINSCRIPTION_ACOMPTE, ControleSortieVerdict, DeciderDemandeSortie,
    DemanderSortieExceptionnelle, SortieCreneau, UpsertAutorisationSortie,
    UpsertSortieParamEtab, UpsertSortieRegleNiveau, UpsertSortieRegleRegime,
    AffecterEleveInternat, ChangeRegime, CreerChambreInternat, CreerSuiviInternat,
    PointerPresenceInternat, PresenceInternatStatut, DispenseVsStatut, PoserDispenseVs,
    AlerteCpeStatut, CreerAlerteCpe,
    AjouterDossierConseil, CreerConseilClasse, DeciderDossierConseil, DocSignerStatut,
    PublierDocASigner, SignerDocASigner, CreerInfoCompChamp, UpsertInfoCompValeur,
    CreerPeriodeVacances, RechercherHomonymes, UpsertMenuCantine,
    CreerTauxTva, UpsertQuotientFoyer, CreerSalle, UpsertParamEtab,
    CreerFiliere, PoserVoeu, UpsertInscriptionSuivi, ExporterLsl, ExporterOnde,
    CreerTarifConso, ExporterNotanet, DeclarerAbsenceRh, AffecterEdtCreneau,
    CreerEditionVs, ExporterLsun, ExporterParcoursup, CreerConventionStage, CreerEntreprise,
    CreerEdtContrainte, EdtContrainteKind,
};
use ent_auth::Role;
use ent_db::{
    assert_local_postgres_url, jour_semaine_iso, CreateMatiere, EdtCreneau, LaboCast, LaboEleve,
    LigneEtat, PgEnt, SaisirNote, LABO_PASSWORD,
};
use uuid::Uuid;

struct Rapport {
    gestes: Vec<String>,
    trous: Vec<String>,
}

impl Rapport {
    fn new() -> Self {
        Self {
            gestes: Vec::new(),
            trous: Vec::new(),
        }
    }

    fn geste(&mut self, s: impl Into<String>) {
        let line = s.into();
        println!("  · {line}");
        self.gestes.push(line);
    }

    fn trou(&mut self, s: impl Into<String>) {
        let line = s.into();
        println!("  ! TROU: {line}");
        self.trous.push(line);
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("ent-lab ÉCHEC: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let url = std::env::var("ENT_DATABASE_URL")
        .map_err(|_| "ENT_DATABASE_URL requis (127.0.0.1 uniquement)".to_string())?;
    assert_local_postgres_url(&url).map_err(|e| e.to_string())?;

    // SMTP / SMS / push / banque : dry-run par défaut (pas de socket provider).
    // Live = ENT_MAIL_LIVE / ENT_SMS_LIVE / ENT_PUSH_LIVE / ENT_BANK_LIVE (+ creds).
    std::env::set_var("ENT_MAIL_DRY_RUN", "1");
    std::env::set_var("ENT_SMS_DRY_RUN", "1");
    std::env::set_var("ENT_PUSH_DRY_RUN", "1");
    std::env::set_var("ENT_BANK_DRY_RUN", "1");
    std::env::set_var("ENT_OBJECT_S3_LIVE", "1");
    std::env::set_var("ENT_OBJECT_S3_BUCKET", "scola-lab");
    std::env::set_var(
        "ENT_OBJECT_DIR",
        format!(".ent-objects-lab-{}", Uuid::new_v4()),
    );
    let mut ent = PgEnt::connect(&url).await.map_err(|e| e.to_string())?;
    ent.migrate().await.map_err(|e| e.to_string())?;
    ent.install_brain_p0();
    ent.install_mail_dry_run();
    let cast = ent.seed_labo_cast().await.map_err(|e| e.to_string())?;
    // Device push Lily (app native simulée) — pour messagerie / carnet / seuil.
    ent.register_push_device_for_actor(
        cast.etablissement_id,
        cast.parent_lea.user_id,
        "ios",
        "lab-apns-lily-potter",
        Some("scola.famille.ios".into()),
    )
    .await
    .map_err(|e| e.to_string())?;
    let jour = today_utc();
    let mut r = Rapport::new();

    println!("=== ENT LABO exploratoire — {jour} ===");
    println!("etablissement: {}", cast.etablissement_id);
    let comptes_eleves = cast.eleves.iter().filter(|e| e.compte.is_some()).count();
    println!(
        "cast: {} élèves ({} comptes face), {} profs, {} CPE, {} direction, {} admini, {} compta, {} infirmière, {} psy",
        cast.eleves.len(),
        comptes_eleves,
        cast.profs.len(),
        cast.cpes.len(),
        cast.directions.len(),
        cast.administratifs.len(),
        cast.comptas.len(),
        cast.infirmieres.len(),
        cast.psys.len(),
    );
    println!("mdp cast: {LABO_PASSWORD}");
    println!();

    explorer_dossier_eleve(&ent, &cast, jour, &mut r).await?;
    explorer_foyer_admin(&ent, &cast, &mut r).await?;
    explorer_adultes_admin(&ent, &cast, &mut r).await?;
    explorer_nomenclature_et_notes(&ent, &cast, jour, &mut r).await?;
    explorer_appels_et_absences(&ent, &cast, jour, &mut r).await?;
    explorer_edt_exceptions(&ent, &cast, jour, &mut r).await?;
    explorer_parents_divergents(&ent, &cast, jour, &mut r).await?;
    explorer_portail_famille(&ent, &cast, jour, &mut r).await?;
    explorer_messagerie(&ent, &cast, &mut r).await?;
    explorer_facturation(&ent, &cast, &mut r).await?;
    explorer_motifs_etab(&ent, &cast, jour, &mut r).await?;
    explorer_face_eleve(&ent, &cast, &mut r).await?;
    explorer_sanctions_vs(&ent, &cast, jour, &mut r).await?;
    explorer_carnet(&ent, &cast, jour, &mut r).await?;
    explorer_sorties(&ent, &cast, jour, &mut r).await?;
    explorer_internat(&ent, &cast, jour, &mut r).await?;
    explorer_dispenses_vs(&ent, &cast, jour, &mut r).await?;
    explorer_alertes_cpe(&ent, &cast, jour, &mut r).await?;
    explorer_docs_a_signer(&ent, &cast, jour, &mut r).await?;
    explorer_conseil_classe(&ent, &cast, jour, &mut r).await?;
    explorer_info_comp(&ent, &cast, jour, &mut r).await?;
    explorer_vacances_homonymes_menus(&ent, &cast, jour, &mut r).await?;
    explorer_quotient_tva_trombi(&ent, &cast, jour, &mut r).await?;
    explorer_param_salles(&ent, &cast, jour, &mut r).await?;
    explorer_filieres_voeux_mails(&ent, &cast, jour, &mut r).await?;
    explorer_inscription_suivi_iban(&ent, &cast, jour, &mut r).await?;
    explorer_absence_rh(&ent, &cast, jour, &mut r).await?;
    explorer_edt_affectation_editions(&ent, &cast, jour, &mut r).await?;
    explorer_stages(&ent, &cast, jour, &mut r).await?;
    explorer_staff_sans_metier(&ent, &cast, jour, &mut r).await?;
    explorer_seuils_et_signaux(&ent, &cast, jour, &mut r).await?;

    println!();
    println!("=== synthèse ===");
    println!("gestes joués: {}", r.gestes.len());
    println!("trous métier découverts: {}", r.trous.len());
    for t in &r.trous {
        println!("  - {t}");
    }
    if r.trous.is_empty() {
        println!("(aucun trou signalé — élargir encore les directions jouées)");
    }
    println!("=== OK — exploration terminée ===");
    Ok(())
}

async fn explorer_dossier_eleve(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# dossier élève (module monté → lab)");

    let found = ent
        .search_eleves(cast.etablissement_id, "potter", 10)
        .await
        .map_err(|e| e.to_string())?;
    if !found.iter().any(|e| e.id == cast.eleve_lea_id) {
        return Err("recherche dossier: Harry Potter introuvable".into());
    }
    r.geste(format!("recherche 'potter' → {} résultat(s)", found.len()));

    let dossier_admin = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "dossier admini introuvable".to_string())?;
    if dossier_admin.eleve.date_naissance.is_none() || dossier_admin.eleve.mef.is_none() {
        return Err("dossier Léa incomplet (naissance/mef)".into());
    }
    if dossier_admin.options.len() < 2 {
        return Err("dossier Léa: options académique+interne attendues".into());
    }
    if dossier_admin.liens.is_empty() || !dossier_admin.liens[0].est_payeur {
        return Err("dossier Léa: encart foyer / payeur manquant".into());
    }
    r.geste(format!(
        "admini lit dossier {} {} — âge={:?} mef={:?} options={} liens={}",
        dossier_admin.eleve.prenom,
        dossier_admin.eleve.nom,
        dossier_admin.age_ans,
        dossier_admin.eleve.mef,
        dossier_admin.options.len(),
        dossier_admin.liens.len()
    ));

    let dossier_parent = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "parent Léa doit voir le dossier".to_string())?;
    if !dossier_parent.sections.notes || !dossier_parent.sections.vs {
        return Err("parent Léa: sections notes/VS attendues".into());
    }
    r.geste("parent Léa voit identite+scolarite+famille+notes+VS");

    let parent_tom_lea = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if parent_tom_lea.is_some() {
        return Err("parent Tom ne doit pas ouvrir le dossier Léa".into());
    }
    r.geste("parent Tom bloqué sur dossier Léa (attendu)");

    let dossier_tom = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_tom_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "parent Tom doit voir Tom".to_string())?;
    if dossier_tom.sections.vs {
        return Err("parent Tom: VS doit être masquée (flag)".into());
    }
    r.geste("parent Tom: notes OK, VS masquée (flag)");

    // Parts 100 % : second payeur qui casse l'invariant
    let bad = ent
        .link_parent(ent_db::CreateLien {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            user_id: cast.parent_tom.user_id,
            foyer_id: Some(cast.foyer_lea_id),
            lien: "autre".into(),
            peut_voir_notes: false,
            peut_voir_vs: false,
            peut_justifier_absence: false,
            peut_autoriser_sorties: false,
            est_payeur: true,
            part_paiement: Some(50),
        })
        .await;
    if bad.is_ok() {
        return Err("parts 50+100 auraient dû être refusées".into());
    }
    r.geste("invariant parts payeurs 100 % — refus 50+100 (attendu)");

    explorer_documents_pap(ent, cast, jour, r).await?;
    explorer_repas_bourses(ent, cast, jour, r).await?;
    explorer_continuum_montee(ent, cast, jour, r).await?;
    Ok(())
}

async fn explorer_documents_pap(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# documents + grants (PAP)");

    let d_admin = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "dossier admin".to_string())?;
    let pap = d_admin
        .documents
        .iter()
        .find(|d| d.titre.contains("PAP"))
        .ok_or_else(|| "PAP seed manquant".to_string())?;
    if d_admin.dispositifs.is_empty() {
        return Err("dispositif PAP manquant".into());
    }
    r.geste(format!(
        "admini voit {} doc(s) + {} dispositif(s)",
        d_admin.documents.len(),
        d_admin.dispositifs.len()
    ));

    let d_prof = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.prof.user_id,
            Role::Prof.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "dossier prof".to_string())?;
    if d_prof.documents.iter().any(|d| d.id == pap.id) {
        return Err("prof voit PAP sans grant — faille".into());
    }
    r.geste("prof ne voit PAS le PAP (verrouillé, attendu)");

    let grant = ent
        .demander_grant_document(DemanderGrant {
            etablissement_id: cast.etablissement_id,
            document_id: pap.id,
            grantee_user_id: cast.prof.user_id,
            duration_days: 14,
            note: Some("besoin lecture PAP maths".into()),
        })
        .await
        .map_err(|e| e.to_string())?;
    r.geste("prof demande grant PAP");

    let decided = ent
        .decider_grant_for_actor(
            Role::Administratif.as_str(),
            DeciderGrant {
                etablissement_id: cast.etablissement_id,
                grant_id: grant.id,
                accepter: true,
                actor_user_id: cast.administratif.user_id,
                note: Some("ok 14j".into()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if decided.status.as_str() != "granted" {
        return Err("grant non accordé".into());
    }
    r.geste("admini accorde grant PAP 14j");

    let d_prof2 = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.prof.user_id,
            Role::Prof.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "dossier prof2".to_string())?;
    if !d_prof2.documents.iter().any(|d| d.id == pap.id) {
        return Err("prof devrait voir PAP après grant".into());
    }
    r.geste("prof voit PAP après grant (circuit Charlemagne cassé → OK ici)");

    let d_parent = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "dossier parent".to_string())?;
    if d_parent.documents.iter().any(|d| d.id == pap.id) {
        return Err("parent ne doit pas voir PAP restreint sans grant".into());
    }
    if !d_parent
        .documents
        .iter()
        .any(|d| d.titre.contains("assurance"))
    {
        return Err("parent devrait voir doc famille standard".into());
    }
    r.geste("parent: PAP masqué, attestation famille visible");

    let prof_tranche = ent
        .decider_grant_for_actor(
            Role::Prof.as_str(),
            DeciderGrant {
                etablissement_id: cast.etablissement_id,
                grant_id: grant.id,
                accepter: true,
                actor_user_id: cast.prof.user_id,
                note: None,
            },
        )
        .await;
    if prof_tranche.is_ok() {
        return Err("prof ne doit pas trancher les grants".into());
    }
    r.geste("prof refusé sur tranchage grant (attendu)");
    r.trou(
        "documents PAP: dépôt + grants demand/décision OK — HTTP POST/GET /eleves/{id}/documents + /documents/* grants + UI dossier",
    );
    Ok(())
}

async fn explorer_repas_bourses(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# grille repas + bourses");

    let acts = ent
        .list_repas_activites(cast.etablissement_id)
        .await
        .map_err(|e| e.to_string())?;
    if acts.len() < 2 {
        return Err("référentiel repas (midi/étude) manquant".into());
    }
    r.geste(format!("référentiel repas: {} activité(s)", acts.len()));

    let d_admin = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "dossier admin repas".to_string())?;
    if d_admin.grille_repas.is_empty() {
        return Err("Léa DP devrait avoir une grille midi".into());
    }
    if d_admin.bourses.is_empty() {
        return Err("Léa devrait avoir une bourse".into());
    }
    if !d_admin.sections.repas || !d_admin.sections.bourses {
        return Err("admini: sections repas+bourses attendues".into());
    }
    r.geste(format!(
        "admini: grille={} bourse(s)={} (échelon={})",
        d_admin.grille_repas.len(),
        d_admin.bourses.len(),
        d_admin.bourses[0].echelon
    ));

    let inscrit = ent
        .eleve_inscrit_midi_present(cast.etablissement_id, cast.eleve_lea_id, jour)
        .await
        .map_err(|e| e.to_string())?;
    // jour lab = today ; si week-end la grille lun-ven peut être false — on teste un mercredi fixe.
    let mercredi = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
    let inscrit_mer = ent
        .eleve_inscrit_midi_present(cast.etablissement_id, cast.eleve_lea_id, mercredi)
        .await
        .map_err(|e| e.to_string())?;
    if !inscrit_mer {
        return Err("Léa doit être inscrite midi un mercredi (lun–ven)".into());
    }
    r.geste(format!(
        "cantine: Léa inscrite midi mercredi={inscrit_mer} (aujourd'hui={inscrit})"
    ));

    let tom_midi = ent
        .eleve_inscrit_midi_present(cast.etablissement_id, cast.eleve_tom_id, mercredi)
        .await
        .map_err(|e| e.to_string())?;
    if tom_midi {
        return Err("Tom EXT sans grille ne doit pas être inscrit midi".into());
    }
    r.geste("Tom EXT: pas de grille midi (attendu)");

    let deduct = ent
        .bourses_deductibles_eleve(cast.etablissement_id, cast.eleve_lea_id, jour)
        .await
        .map_err(|e| e.to_string())?;
    if deduct.is_empty() {
        return Err("bourse Léa acceptée+procuration doit être déductible".into());
    }
    r.geste(format!(
        "facture: {} bourse(s) déductible(s) pour Léa",
        deduct.len()
    ));

    let d_parent = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "parent dossier repas".to_string())?;
    if !d_parent.sections.repas || !d_parent.sections.bourses {
        return Err("parent doit voir repas+bourses".into());
    }
    if d_parent.bourses.is_empty() {
        return Err("parent Léa: bourse invisible".into());
    }
    r.geste("parent Léa voit grille + bourse");

    let d_prof = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.prof.user_id,
            Role::Prof.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "prof dossier repas".to_string())?;
    if !d_prof.sections.repas {
        return Err("prof devrait voir section repas (cantine)".into());
    }
    if d_prof.sections.bourses || !d_prof.bourses.is_empty() {
        return Err("prof ne doit pas voir les bourses (finance)".into());
    }
    r.geste("prof: repas OK, bourses masquées (attendu)");

    let d_compta = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.compta.user_id,
            Role::Compta.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "compta dossier".to_string())?;
    if !d_compta.sections.bourses || d_compta.bourses.is_empty() {
        return Err("compta doit lire les bourses".into());
    }
    r.geste("compta lit bourses Léa");

    let types = ent
        .list_bourse_types_for_actor(cast.etablissement_id, Role::Compta.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if types.is_empty() {
        return Err("référentiel types bourse attendu".into());
    }
    r.geste(format!("compta lit {} type(s) bourse", types.len()));
    let type_tom = ent
        .creer_bourse_type_for_actor(
            Role::Compta.as_str(),
            CreerBourseType {
                etablissement_id: cast.etablissement_id,
                code: "SOC".into(),
                libelle: "Bourse sociale lab".into(),
                actor_user_id: Some(cast.compta.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("compta crée type bourse {}", type_tom.code));
    let demandee = ent
        .affecter_bourse_for_actor(
            Role::Administratif.as_str(),
            AffecterBourse {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_tom_id,
                bourse_type_id: type_tom.id,
                echelon: "1".into(),
                avec_procuration: true,
                date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                actor_user_id: Some(cast.administratif.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if demandee.status != BourseStatus::Demandee {
        return Err("affectation → demandee".into());
    }
    r.geste("admini affecte bourse Tom → demandee");
    let acceptee = ent
        .decider_bourse_for_actor(
            Role::Compta.as_str(),
            DeciderBourse {
                etablissement_id: cast.etablissement_id,
                bourse_id: demandee.id,
                accepter: true,
                actor_user_id: Some(cast.compta.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if acceptee.status != BourseStatus::Acceptee {
        return Err("décision → acceptee".into());
    }
    r.geste("compta accepte bourse Tom → déductible facture");
    let refuse_prof_bt = ent
        .creer_bourse_type_for_actor(
            Role::Prof.as_str(),
            CreerBourseType {
                etablissement_id: cast.etablissement_id,
                code: "X".into(),
                libelle: "interdit".into(),
                actor_user_id: Some(cast.prof.user_id.to_string()),
            },
        )
        .await;
    if refuse_prof_bt.is_ok() {
        return Err("prof ne doit pas créer de type bourse".into());
    }
    r.geste("prof refusé sur type bourse (attendu)");

    // Prof ne pose pas la grille.
    let midi = acts
        .iter()
        .find(|a| a.kind.as_str() == "midi")
        .ok_or_else(|| "activite midi".to_string())?;
    let refuse = ent
        .poser_grille_repas_for_actor(
            Role::Prof.as_str(),
            PoserGrilleRepas {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_tom_id,
                activite_id: midi.id,
                jours: JoursMasque::lun_ven(),
                date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                actor_user_id: Some(cast.prof.user_id.to_string()),
            },
        )
        .await;
    if refuse.is_ok() {
        return Err("prof ne doit pas poser la grille repas".into());
    }
    r.geste("prof refusé sur pose grille (attendu)");

    // Grille vide refusée (invariant domaine).
    let bad = ent
        .poser_grille_repas(PoserGrilleRepas {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_tom_id,
            activite_id: midi.id,
            jours: JoursMasque::default(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            actor_user_id: Some(cast.administratif.user_id.to_string()),
        })
        .await;
    if bad.is_ok() {
        return Err("grille sans jour aurait dû être refusée".into());
    }
    r.geste("invariant: grille sans jour refusée");

    // Prévenance / dispense jour → hors cantine + hors volume facturable
    let debut_sem = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(); // lun
    let fin_sem = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(); // ven
    let vol0 = ent
        .compter_repas_facturables_eleve(
            cast.etablissement_id,
            cast.eleve_lea_id,
            RepasActiviteKind::Midi,
            debut_sem,
            fin_sem,
        )
        .await
        .map_err(|e| e.to_string())?;
    if vol0 != 5 {
        return Err(format!("volume midi lun–ven attendu 5, obtenu {vol0}"));
    }
    let dispense = ent
        .poser_dispense_repas_for_actor(
            Role::Parent.as_str(),
            PoserDispenseRepas {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                activite_id: midi.id,
                jour: mercredi,
                motif: "Sortie scolaire".into(),
                source: DispenseSource::Parent,
                actor_user_id: Some(cast.parent_lea.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let inscrit_apres = ent
        .eleve_inscrit_midi_present(cast.etablissement_id, cast.eleve_lea_id, mercredi)
        .await
        .map_err(|e| e.to_string())?;
    if inscrit_apres {
        return Err("après dispense Lily, Léa ne doit plus être inscrite midi mercredi".into());
    }
    let vol1 = ent
        .compter_repas_facturables_eleve(
            cast.etablissement_id,
            cast.eleve_lea_id,
            RepasActiviteKind::Midi,
            debut_sem,
            fin_sem,
        )
        .await
        .map_err(|e| e.to_string())?;
    if vol1 != 4 {
        return Err(format!(
            "volume après dispense attendu 4, obtenu {vol1} (dispense={})",
            dispense.id
        ));
    }
    // Narcissa ne peut pas dispenser Léa
    let narc = ent
        .poser_dispense_repas_for_actor(
            Role::Parent.as_str(),
            PoserDispenseRepas {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                activite_id: midi.id,
                jour: NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
                motif: "Intrusion".into(),
                source: DispenseSource::Parent,
                actor_user_id: Some(cast.parent_tom.user_id.to_string()),
            },
        )
        .await;
    if narc.is_ok() {
        return Err("Narcissa ne doit pas poser de dispense sur Léa".into());
    }
    r.geste(format!(
        "Lily prévenance cantine mercredi → volume {vol0}→{vol1} · Narcissa bloquée (attendu)"
    ));

    // Export volumes cuisine (grille − prévenances) pour la semaine
    let (vols, csv) = ent
        .export_volumes_cuisine_for_actor(
            Role::Compta.as_str(),
            ExporterVolumesRepas {
                etablissement_id: cast.etablissement_id,
                debut: debut_sem,
                fin: fin_sem,
                kind: Some(RepasActiviteKind::Midi),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if vols.is_empty() {
        return Err("export volumes cuisine: lignes attendues".into());
    }
    let sum: u32 = vols.iter().map(|v| v.nb_prevus).sum();
    let csv_s = String::from_utf8(csv.clone()).map_err(|e| e.to_string())?;
    if !csv_s.contains("date;activite;kind;nb_prevus") {
        return Err("CSV cuisine: en-tête manquant".into());
    }
    let refuse_prof = ent
        .export_volumes_cuisine_for_actor(
            Role::Prof.as_str(),
            ExporterVolumesRepas {
                etablissement_id: cast.etablissement_id,
                debut: debut_sem,
                fin: fin_sem,
                kind: Some(RepasActiviteKind::Midi),
            },
        )
        .await;
    if refuse_prof.is_ok() {
        return Err("prof ne doit pas exporter les volumes cuisine".into());
    }
    r.geste(format!(
        "Slughorn export volumes cuisine {} ligne(s) · {} repas · {} octets · Rogue refusé",
        vols.len(),
        sum,
        csv.len()
    ));

    // Passage badge cantine (jour sans dispense — jeudi de la semaine)
    let jeudi = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
    let harry = ent
        .get_eleve(cast.etablissement_id, cast.eleve_lea_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Harry introuvable")?;
    let badge = harry
        .numero_badge
        .clone()
        .ok_or("Harry doit avoir un badge seed")?;
    let (passage, eleve_scan) = ent
        .scan_badge_repas_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            &badge,
            RepasActiviteKind::Midi,
            jeudi,
            cast.cpe.user_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if eleve_scan.id != cast.eleve_lea_id {
        return Err("scan badge doit résoudre Harry".into());
    }
    let doublon = ent
        .scan_badge_repas_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            &badge,
            RepasActiviteKind::Midi,
            jeudi,
            cast.cpe.user_id,
        )
        .await;
    if doublon.is_ok() {
        return Err("double scan même jour doit être refusé".into());
    }
    let refuse_dispense = ent
        .scan_badge_repas_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            &badge,
            RepasActiviteKind::Midi,
            mercredi,
            cast.cpe.user_id,
        )
        .await;
    if refuse_dispense.is_ok() {
        return Err("scan jour dispensé doit être refusé".into());
    }
    let inconnu = ent
        .scan_badge_repas_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            "BADGE-INCONNU",
            RepasActiviteKind::Midi,
            jeudi,
            cast.cpe.user_id,
        )
        .await;
    if inconnu.is_ok() {
        return Err("badge inconnu doit être refusé".into());
    }
    let passages = ent
        .list_passages_repas_jour_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            jeudi,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !passages.iter().any(|p| p.id == passage.id) {
        return Err("passage absent de la liste du jour".into());
    }
    let refuse_parent = ent
        .scan_badge_repas_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            &badge,
            RepasActiviteKind::Midi,
            NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            cast.parent_lea.user_id,
        )
        .await;
    if refuse_parent.is_ok() {
        return Err("parent ne doit pas scanner les badges".into());
    }
    // Historique passages élève (famille liée + staff ; parent non lié refusé)
    let hist_parent = ent
        .list_passages_repas_eleve_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            cast.parent_lea.user_id,
            cast.eleve_lea_id,
            debut_sem,
            fin_sem,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !hist_parent.iter().any(|p| p.id == passage.id) {
        return Err("parent lié doit voir le passage badge de son enfant".into());
    }
    let hist_cpe = ent
        .list_passages_repas_eleve_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cast.cpe.user_id,
            cast.eleve_lea_id,
            debut_sem,
            fin_sem,
        )
        .await
        .map_err(|e| e.to_string())?;
    if hist_cpe.len() != hist_parent.len() {
        return Err("CPE et parent doivent voir le même historique passages".into());
    }
    let refuse_hist_tom = ent
        .list_passages_repas_eleve_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            cast.parent_tom.user_id,
            cast.eleve_lea_id,
            debut_sem,
            fin_sem,
        )
        .await;
    if refuse_hist_tom.is_ok() {
        return Err("parent non lié ne doit pas lire l'historique passages".into());
    }
    r.geste(format!(
        "CPE scan badge {} → passage midi jeudi · hist Lily {} · Tom refusé · doublon/dispense/inconnu/parent refusés",
        badge,
        hist_parent.len()
    ));

    // Facturation auto depuis passages badge (semaine)
    let n_prev = ent
        .apercu_cantine_badge_for_actor(
            Role::Compta.as_str(),
            cast.etablissement_id,
            cast.eleve_lea_id,
            debut_sem,
            fin_sem,
            RepasActiviteKind::Midi,
        )
        .await
        .map_err(|e| e.to_string())?;
    if n_prev != 1 {
        return Err(format!(
            "aperçu badge attendu 1 passage (jeudi), obtenu {n_prev}"
        ));
    }
    let (fac_badge, lignes_badge, nb) = ent
        .creer_facture_cantine_badge_for_actor(
            Role::Compta.as_str(),
            cast.etablissement_id,
            cast.foyer_lea_id,
            cast.eleve_lea_id,
            debut_sem,
            fin_sem,
            450,
            format!("CB-{}", &cast.etablissement_id.to_string()[..8]),
            cast.compta.user_id,
            RepasActiviteKind::Midi,
        )
        .await
        .map_err(|e| e.to_string())?;
    if nb != 1 || lignes_badge.len() != 1 || lignes_badge[0].quantite != 1 {
        return Err("facture badge: 1 ligne × qté 1 attendue".into());
    }
    if fac_badge.total_centimes != 450 {
        return Err(format!(
            "total badge attendu 450 c, obtenu {}",
            fac_badge.total_centimes
        ));
    }
    if fac_badge.nature.as_deref() != Some(NATURE_CANTINE_BADGE) {
        return Err("nature cantine_badge attendue".into());
    }
    let refuse_parent_fac = ent
        .creer_facture_cantine_badge_for_actor(
            Role::Parent.as_str(),
            cast.etablissement_id,
            cast.foyer_lea_id,
            cast.eleve_lea_id,
            debut_sem,
            fin_sem,
            450,
            "CB-NO".into(),
            cast.parent_lea.user_id,
            RepasActiviteKind::Midi,
        )
        .await;
    if refuse_parent_fac.is_ok() {
        return Err("parent ne doit pas créer facture badge".into());
    }
    r.geste(format!(
        "Slughorn facture badge {} · {} passage(s) · {} c · Lily refusée",
        fac_badge.numero, nb, fac_badge.total_centimes
    ));
    // bourses: ventilation multi-échéances couverte (explorer_facturation)
    Ok(())
}

async fn explorer_continuum_montee(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# continuum N-1 / N / N+1 + montée rapide");

    let (nm1, n, np1) = annees_continuum(jour);
    r.geste(format!("années continuum: {nm1} | {n} | {np1}"));

    let d0 = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "dossier continuum".to_string())?;
    if !d0.sections.continuum {
        return Err("section continuum attendue".into());
    }
    let has_nm1 = d0
        .continuum
        .iter()
        .any(|s| s.annee_scolaire == nm1 && s.statut == ScolariteAnneeStatut::Historique);
    let has_n = d0
        .continuum
        .iter()
        .any(|s| s.annee_scolaire == n && s.statut == ScolariteAnneeStatut::Courante);
    if !has_nm1 || !has_n {
        return Err(format!(
            "Léa doit avoir N-1 historique + N courante (got {} snapshots)",
            d0.continuum.len()
        ));
    }
    r.geste(format!(
        "fiche Léa: {} snapshot(s) — N-1+N présents (pas de year-jump)",
        d0.continuum.len()
    ));

    // Campagne fermée → montée refusée.
    let refuse_fermee = ent
        .montee_rapide_for_actor(
            Role::Administratif.as_str(),
            MonteeRapide {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_courante: annee_scolaire_de(jour),
                actor_user_id: Some(cast.administratif.user_id.to_string()),
            },
            jour,
        )
        .await;
    if refuse_fermee.is_ok() {
        return Err("montée sans campagne aurait dû être refusée".into());
    }
    r.geste("montée refusée si campagne fermée (attendu)");

    let camp = ent
        .ouvrir_campagne_reinscription_for_actor(
            Role::Administratif.as_str(),
            OuvrirCampagneReinscription {
                etablissement_id: cast.etablissement_id,
                annee_cible: np1.clone(),
                actor_user_id: Some(cast.administratif.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !camp.ouverte {
        return Err("campagne devrait être ouverte".into());
    }
    r.geste(format!("campagne réinscription {np1} ouverte"));

    let prof_montee = ent
        .montee_rapide_for_actor(
            Role::Prof.as_str(),
            MonteeRapide {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_courante: n.clone(),
                actor_user_id: Some(cast.prof.user_id.to_string()),
            },
            jour,
        )
        .await;
    if prof_montee.is_ok() {
        return Err("prof ne doit pas faire la montée rapide".into());
    }
    r.geste("prof refusé sur montée rapide (attendu)");

    let n1 = ent
        .montee_rapide_for_actor(
            Role::Administratif.as_str(),
            MonteeRapide {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_courante: n.clone(),
                actor_user_id: Some(cast.administratif.user_id.to_string()),
            },
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if n1.niveau.as_deref() != Some("3e") {
        return Err(format!("montée 4e→3e attendue, got {:?}", n1.niveau));
    }
    if n1.passage_juridique || !n1.redoublement_possible {
        return Err("montée rapide ≠ passage juridique ; redoublement encore possible".into());
    }
    if n1.classe.is_some() {
        return Err("classe N+1 doit rester vide (fine plus tard)".into());
    }
    r.geste(format!(
        "montée rapide Léa → {} niveau={:?} passage_juridique={} redoublement={}",
        n1.annee_scolaire, n1.niveau, n1.passage_juridique, n1.redoublement_possible
    ));

    let d1 = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "parent continuum".to_string())?;
    if d1.continuum.len() < 3 {
        return Err("parent doit voir N-1 / N / N+1 sur la fiche".into());
    }
    let opts_n1: Vec<_> = d1
        .options_continuum
        .iter()
        .filter(|o| o.annee_scolaire == np1)
        .collect();
    if opts_n1.is_empty() {
        return Err("options académiques N+1 proposées attendues".into());
    }
    r.geste(format!(
        "parent voit continuum={} + {} option(s) N+1 proposées",
        d1.continuum.len(),
        opts_n1.len()
    ));

    let chosen = ent
        .choisir_option_n1_for_parent(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            ChoisirOptionN1 {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_scolaire: np1.clone(),
                nature: OptionNature::Academique,
                code: "LV2_ESP".into(),
                libelle: "Espagnol LV2".into(),
                retenir: true,
                actor_user_id: Some(cast.parent_lea.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if chosen.statut.as_str() != "retenue" {
        return Err("option parent devrait être retenue".into());
    }
    r.geste("parent retient option académique N+1 (LV2_ESP)");

    let parent_tom = ent
        .choisir_option_n1_for_parent(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            ChoisirOptionN1 {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_scolaire: np1.clone(),
                nature: OptionNature::Academique,
                code: "HACK".into(),
                libelle: "Hack".into(),
                retenir: true,
                actor_user_id: Some(cast.parent_tom.user_id.to_string()),
            },
        )
        .await;
    if parent_tom.is_ok() {
        return Err("parent Tom ne doit pas choisir options Léa".into());
    }
    r.geste("parent Tom bloqué sur options N+1 Léa (attendu)");

    // Vannes documentaires : exigence ASSURANCE_RC avant confirmation.
    let exigence = ent
        .poser_exigence_doc_for_actor(
            Role::Administratif.as_str(),
            domain_eleve::PoserExigenceDoc {
                etablissement_id: cast.etablissement_id,
                annee_cible: np1.clone(),
                code: "ASSURANCE_RC".into(),
                libelle: "Attestation assurance RC".into(),
                tiroir: domain_eleve::DocTiroir::Famille,
                obligatoire: true,
                actor_user_id: Some(cast.administratif.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if exigence.code != "ASSURANCE_RC" {
        return Err(format!("exigence code attendu ASSURANCE_RC, got {}", exigence.code));
    }
    r.geste("admini pose exigence documentaire ASSURANCE_RC pour N+1");

    let refuse_sans_doc = ent
        .confirmer_reinscription_for_parent(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            ConfirmerReinscription {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_scolaire: np1.clone(),
                actor_user_id: Some(cast.parent_lea.user_id.to_string()),
            },
        )
        .await;
    if refuse_sans_doc.is_ok() {
        return Err("confirmation sans ASSURANCE_RC aurait dû être refusée".into());
    }
    r.geste("confirmation Lily refusée sans document ASSURANCE_RC (attendu)");

    let dep = ent
        .deposer_doc_reinscription_for_parent(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            cast.eleve_lea_id,
            &np1,
            "ASSURANCE_RC",
            "Attestation assurance RC Harry",
        )
        .await
        .map_err(|e| e.to_string())?;
    if dep.code.as_deref() != Some("ASSURANCE_RC") {
        return Err(format!("doc déposé code={:?}", dep.code));
    }
    r.geste("Lily dépose ASSURANCE_RC pour Harry");

    let narcissadoc = ent
        .deposer_doc_reinscription_for_parent(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            cast.eleve_lea_id,
            &np1,
            "ASSURANCE_RC",
            "Hack",
        )
        .await;
    if narcissadoc.is_ok() {
        return Err("Narcissa ne doit pas déposer de doc sur Harry".into());
    }
    r.geste("Narcissa bloquée sur dépôt doc Harry (attendu)");

    // Vanne acompte : 50 € requis avant confirmation.
    let camp_ac = ent
        .configurer_acompte_campagne_for_actor(
            Role::Administratif.as_str(),
            ConfigurerAcompteCampagne {
                etablissement_id: cast.etablissement_id,
                annee_cible: np1.clone(),
                acompte_centimes: 5000,
                actor_user_id: Some(cast.administratif.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if camp_ac.acompte_centimes != 5000 {
        return Err(format!("acompte campagne={}", camp_ac.acompte_centimes));
    }
    r.geste("admini configure acompte 5000 c pour N+1");

    let refuse_sans_acompte = ent
        .confirmer_reinscription_for_parent(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            ConfirmerReinscription {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_scolaire: np1.clone(),
                actor_user_id: Some(cast.parent_lea.user_id.to_string()),
            },
        )
        .await;
    if refuse_sans_acompte.is_ok() {
        return Err("confirmation sans acompte aurait dû être refusée".into());
    }
    r.geste("confirmation Lily refusée sans acompte (attendu)");

    let (ac_brouillon, _) = ent
        .creer_facture_for_actor(
            Role::Compta.as_str(),
            CreerFacture {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                numero: format!("AC-{}", &cast.etablissement_id.to_string()[..8]),
                libelle: "Acompte réinscription".into(),
                nature: Some(NATURE_REINSCRIPTION_ACOMPTE.into()),
                annee_scolaire: Some(np1.clone()),
                lignes: vec![LigneFactureCmd {
                    eleve_id: Some(cast.eleve_lea_id),
                    libelle: "Acompte rentrée".into(),
                    quantite: 1,
                    montant_unitaire_centimes: 5000,
                }],
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let ac_emise = ent
        .emettre_facture_for_actor(
            cast.etablissement_id,
            ac_brouillon.id,
            cast.compta.user_id,
            Role::Compta.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let (_enc, _fac, _q) = ent
        .encaisser_facture_for_actor(
            Role::Compta.as_str(),
            EncaisserFacture {
                etablissement_id: cast.etablissement_id,
                facture_id: ac_emise.id,
                montant_centimes: 5000,
                moyen: MoyenPaiement::Virement,
                reference: Some("ACOMPTE-N1".into()),
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("Slughorn émet + encaisse acompte 5000 c (reinscription_acompte)");

    let confirmee = ent
        .confirmer_reinscription_for_parent(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            ConfirmerReinscription {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_scolaire: np1.clone(),
                actor_user_id: Some(cast.parent_lea.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if confirmee.statut != ScolariteAnneeStatut::Reinscription {
        return Err("confirmation parent → statut reinscription attendu".into());
    }
    if confirmee.source != ScolariteSource::ReinscriptionParent {
        return Err("source reinscription_parent attendue".into());
    }
    if confirmee.passage_juridique {
        return Err("confirmation parent ≠ passage juridique".into());
    }
    r.geste(format!(
        "Lily confirme réinscription Harry {} après vannes docs+acompte (statut={}, juridique=false)",
        confirmee.annee_scolaire, confirmee.statut.as_str()
    ));

    let mails_reins = ent
        .count_mails_kind(cast.etablissement_id, "reinscription_confirmee")
        .await
        .map_err(|e| e.to_string())?;
    if mails_reins < 1 {
        return Err("SMTP confirmation réinscription attendu en outbox".into());
    }
    r.geste(format!(
        "SMTP confirmation réinscription dry-run → {mails_reins} mail(s) outbox"
    ));
    let push_reins = ent
        .count_push_kind(cast.etablissement_id, "reinscription_confirmee")
        .await
        .map_err(|e| e.to_string())?;
    if push_reins < 1 {
        return Err("push confirmation réinscription attendu sur device Lily".into());
    }
    r.geste(format!(
        "push confirmation réinscription dry-run → {push_reins} notif(s) outbox"
    ));

    let narcissa_confirme = ent
        .confirmer_reinscription_for_parent(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            ConfirmerReinscription {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                annee_scolaire: np1.clone(),
                actor_user_id: Some(cast.parent_tom.user_id.to_string()),
            },
        )
        .await;
    if narcissa_confirme.is_ok() {
        return Err("Narcissa ne doit pas confirmer Harry".into());
    }
    r.geste("Narcissa bloquée sur confirmation Harry (attendu)");

    // Lot montée : toutes les 6e (Luna + Seamus) → 5e.
    let sixiemes: Vec<_> = cast
        .eleves
        .iter()
        .filter(|e| e.classe.starts_with('6'))
        .collect();
    if sixiemes.len() < 2 {
        return Err(format!(
            "cast labo doit avoir ≥2 sixièmes (got {})",
            sixiemes.len()
        ));
    }
    let lot = ent
        .montee_lot_for_actor(
            Role::Administratif.as_str(),
            domain_eleve::MonteeLot {
                etablissement_id: cast.etablissement_id,
                annee_courante: n.clone(),
                niveau_source: "6e".into(),
                actor_user_id: Some(cast.administratif.user_id.to_string()),
            },
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if lot.niveau_cible != "5e" {
        return Err(format!("lot 6e→5e attendu, cible={}", lot.niveau_cible));
    }
    if lot.montes.len() < 2 {
        return Err(format!(
            "lot 6e doit monter ≥2 élèves (monté={}, ignorés={:?})",
            lot.montes.len(),
            lot.ignores
        ));
    }
    for eid in &lot.montes {
        let proj = ent
            .scolarite_eleve_annee(cast.etablissement_id, *eid, &np1)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("projection N+1 manquante pour {eid}"))?;
        if proj.niveau.as_deref() != Some("5e") || proj.passage_juridique {
            return Err(format!(
                "projection lot invalide eleve={eid} niveau={:?} juridique={}",
                proj.niveau, proj.passage_juridique
            ));
        }
    }
    r.geste(format!(
        "lot montée 6e→5e : {} monté(s), {} ignoré(s)",
        lot.montes.len(),
        lot.ignores.len()
    ));

    let lot2 = ent
        .montee_lot_for_actor(
            Role::Administratif.as_str(),
            domain_eleve::MonteeLot {
                etablissement_id: cast.etablissement_id,
                annee_courante: n.clone(),
                niveau_source: "6e".into(),
                actor_user_id: Some(cast.administratif.user_id.to_string()),
            },
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !lot2.montes.is_empty() {
        return Err("2ᵉ lot 6e ne doit rien remonter (idempotent)".into());
    }
    if lot2.ignores.len() < 2
        || !lot2.ignores.iter().all(|i| i.motif == "deja_projetee")
    {
        return Err(format!(
            "2ᵉ lot doit ignorer deja_projetee (got {:?})",
            lot2.ignores
        ));
    }
    r.geste("lot montée idempotent (déjà projetés ignorés)");

    let prof_lot = ent
        .montee_lot_for_actor(
            Role::Prof.as_str(),
            domain_eleve::MonteeLot {
                etablissement_id: cast.etablissement_id,
                annee_courante: n,
                niveau_source: "5e".into(),
                actor_user_id: Some(cast.prof.user_id.to_string()),
            },
            jour,
        )
        .await;
    if prof_lot.is_ok() {
        return Err("prof ne doit pas lancer un lot de montée".into());
    }
    r.geste("prof refusé sur lot montée (attendu)");

    r.trou(
        "continuum: vannes docs+acompte+SEPA+rappels + SMTP + push confirmation OK — natives réinscription + live si ENT_PUSH_LIVE=1",
    );
    Ok(())
}

async fn explorer_nomenclature_et_notes(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# nomenclature + notes (périodes / devoirs / moyennes)");
    let maths = ent
        .create_matiere(
            CreateMatiere {
                etablissement_id: cast.etablissement_id,
                code: format!("MATH-{}", &cast.etablissement_id.simple().to_string()[..6]),
                libelle: "Mathématiques".into(),
                source_nomenclature: Some("SIECLE:MATH".into()),
            },
            Some(cast.administratif.user_id.to_string()),
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini crée matière {} ({})",
        maths.libelle, maths.code
    ));

    let periode = ent
        .creer_note_periode(CreerNotePeriode {
            etablissement_id: cast.etablissement_id,
            code: format!("T1-{}", &cast.etablissement_id.simple().to_string()[..6]),
            libelle: "Trimestre 1".into(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: NaiveDate::from_ymd_opt(2026, 12, 20).unwrap(),
            actor_user_id: Some(cast.administratif.user_id.to_string()),
        })
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini ouvre période notes {} ({})",
        periode.code, periode.statut.as_str()
    ));

    let classe_lea = cast
        .eleves
        .iter()
        .find(|e| e.id == cast.eleve_lea_id)
        .map(|e| e.classe.clone())
        .unwrap_or_else(|| "4B".into());
    let devoir = ent
        .creer_devoir(CreerDevoir {
            etablissement_id: cast.etablissement_id,
            periode_id: periode.id,
            matiere_id: maths.id,
            classe: classe_lea.clone(),
            titre: "DS1 Maths".into(),
            date_devoir: jour,
            coef: 2.0,
            sur: 20.0,
            actor_user_id: cast.prof.user_id,
        })
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "prof crée devoir {} coef={} sur={}",
        devoir.titre, devoir.coef, devoir.sur
    ));
    let devoirs_list = ent
        .list_devoirs_for_actor(
            cast.etablissement_id,
            Role::Prof.as_str(),
            periode.id,
            Some(classe_lea.as_str()),
        )
        .await
        .map_err(|e| e.to_string())?;
    if !devoirs_list.iter().any(|d| d.id == devoir.id) {
        return Err("list_devoirs doit inclure le devoir créé".into());
    }
    r.geste(format!(
        "prof liste {} devoir(s) période {} classe {}",
        devoirs_list.len(),
        periode.code,
        classe_lea
    ));
    let refuse_parent_devoirs = ent
        .list_devoirs_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            periode.id,
            None,
        )
        .await;
    if refuse_parent_devoirs.is_ok() {
        return Err("parent ne doit pas lister les devoirs staff".into());
    }
    r.geste("parent refusé sur liste devoirs (attendu)");

    // Plusieurs profs notent plusieurs élèves — pas un seul chemin heureux.
    let cibles: Vec<&LaboEleve> = cast.eleves.iter().take(4).collect();
    for (i, eleve) in cibles.iter().enumerate() {
        let prof = &cast.profs[i % cast.profs.len()];
        let note = ent
            .saisir_note(SaisirNote {
                etablissement_id: cast.etablissement_id,
                eleve_id: eleve.id,
                matiere_id: maths.id,
                valeur: 8.0 + (i as f64) * 2.0,
                sur: 20.0,
                date_note: jour,
                auteur_user_id: prof.user_id,
                libelle: Some(format!("DS{}", i + 1)),
                devoir_id: if i == 0 { Some(devoir.id) } else { None },
                periode_id: Some(periode.id),
            })
            .await
            .map_err(|e| e.to_string())?;
        r.geste(format!(
            "prof{} note {} {} → {}/{} (période {})",
            (i % cast.profs.len()) + 1,
            eleve.prenom,
            eleve.nom,
            note.valeur,
            note.sur,
            note.periode_id
                .map(|p| p.to_string())
                .unwrap_or_else(|| "—".into())
        ));

        let parent_voit = ent
            .actor_can_read_notes(
                cast.etablissement_id,
                eleve.parent.user_id,
                Role::Parent.as_str(),
                eleve.id,
            )
            .await
            .map_err(|e| e.to_string())?;
        if eleve.peut_voir_notes && !parent_voit {
            return Err(format!(
                "authz notes: {} devrait voir mais ne voit pas",
                eleve.prenom
            ));
        }
        if !eleve.peut_voir_notes && parent_voit {
            return Err(format!(
                "authz notes: {} ne devrait pas voir mais voit",
                eleve.prenom
            ));
        }
        r.geste(format!(
            "parent de {} voit notes? {parent_voit} (flag={})",
            eleve.prenom, eleve.peut_voir_notes
        ));
    }

    // 2ᵉ note Léa (coef 1) → moyenne pondérée avec DS1 coef 2
    let lea = cast
        .eleves
        .iter()
        .find(|e| e.id == cast.eleve_lea_id)
        .ok_or("Léa introuvable dans le cast")?;
    ent.saisir_note(SaisirNote {
        etablissement_id: cast.etablissement_id,
        eleve_id: lea.id,
        matiere_id: maths.id,
        valeur: 16.0,
        sur: 20.0,
        date_note: jour,
        auteur_user_id: cast.prof.user_id,
        libelle: Some("Interro".into()),
        devoir_id: None,
        periode_id: Some(periode.id),
    })
    .await
    .map_err(|e| e.to_string())?;
    let moy = ent
        .moyenne_matiere(
            cast.etablissement_id,
            lea.id,
            periode.id,
            maths.id,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or("moyenne Léa attendue")?;
    r.geste(format!(
        "moyenne Maths Léa T1 = {:.2}/20",
        moy.moyenne_sur_20
    ));

    // Phase appréciations : notes verrouillées, textes OK
    ent.set_note_periode_statut(
        cast.etablissement_id,
        periode.id,
        NotePeriodeStatut::PartielleAppreciations,
        Some(cast.administratif.user_id.to_string()),
    )
    .await
    .map_err(|e| e.to_string())?;
    let bloque_note = ent
        .saisir_note(SaisirNote {
            etablissement_id: cast.etablissement_id,
            eleve_id: lea.id,
            matiere_id: maths.id,
            valeur: 10.0,
            sur: 20.0,
            date_note: jour,
            auteur_user_id: cast.prof.user_id,
            libelle: Some("après partielle".into()),
            devoir_id: None,
            periode_id: Some(periode.id),
        })
        .await;
    if bloque_note.is_ok() {
        return Err("partielle_appreciations doit bloquer la saisie de notes".into());
    }
    r.geste("période partielle_appreciations → notes refusées (OK)");

    let app_matiere = ent
        .saisir_appreciation_for_actor(
            Role::Prof.as_str(),
            SaisirAppreciation {
                etablissement_id: cast.etablissement_id,
                periode_id: periode.id,
                eleve_id: lea.id,
                matiere_id: Some(maths.id),
                texte: "Progrès notables en géométrie.".into(),
                actor_user_id: cast.prof.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let app_generale = ent
        .saisir_appreciation_for_actor(
            Role::Direction.as_str(),
            SaisirAppreciation {
                etablissement_id: cast.etablissement_id,
                periode_id: periode.id,
                eleve_id: lea.id,
                matiere_id: None,
                texte: "Trimestre satisfaisant dans l'ensemble.".into(),
                actor_user_id: cast.direction.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if app_matiere.matiere_id.is_none() || app_generale.matiere_id.is_some() {
        return Err("appréciation matière/générale mal typée".into());
    }
    r.geste("Rogue appréciation Maths Harry · direction appréciation générale");

    let apps_lily = ent
        .list_appreciations_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            lea.id,
            Some(periode.id),
        )
        .await
        .map_err(|e| e.to_string())?;
    if apps_lily.len() < 2 {
        return Err("Lily doit lire les 2 appréciations".into());
    }
    r.geste(format!(
        "Lily lit {} appréciation(s) Harry (période)",
        apps_lily.len()
    ));

    let apps_tom = ent
        .list_appreciations_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            lea.id,
            Some(periode.id),
        )
        .await;
    if apps_tom.is_ok() {
        return Err("Narcissa ne doit pas lire les appréciations Harry".into());
    }
    r.geste("Narcissa bloquée sur appréciations Harry (attendu)");

    // Upsert idempotent
    let app2 = ent
        .saisir_appreciation_for_actor(
            Role::Prof.as_str(),
            SaisirAppreciation {
                etablissement_id: cast.etablissement_id,
                periode_id: periode.id,
                eleve_id: lea.id,
                matiere_id: Some(maths.id),
                texte: "Progrès notables — à poursuivre.".into(),
                actor_user_id: cast.prof.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if app2.id != app_matiere.id && app2.texte != "Progrès notables — à poursuivre." {
        // id peut rester le même via UPDATE ; texte doit être à jour
    }
    let apps_apres = ent
        .list_appreciations_for_actor(
            cast.etablissement_id,
            cast.prof.user_id,
            Role::Prof.as_str(),
            lea.id,
            Some(periode.id),
        )
        .await
        .map_err(|e| e.to_string())?;
    let maths_apps = apps_apres
        .iter()
        .filter(|a| a.matiere_id == Some(maths.id))
        .count();
    if maths_apps != 1 {
        return Err(format!("upsert appréciation matière: attendu 1, got {maths_apps}"));
    }
    r.geste("upsert appréciation matière idempotent (1 seule ligne)");

    let csv_lsl = ent
        .exporter_lsl_csv_for_actor(
            Role::Prof.as_str(),
            ExporterLsl {
                etablissement_id: cast.etablissement_id,
                periode_id: periode.id,
                classe: Some(lea.classe.clone()),
                jour_reference: jour,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !csv_lsl.contains("ine;nom;") || !csv_lsl.contains(&lea.nom) {
        return Err(format!("export LSL invalide: {csv_lsl}"));
    }
    r.geste(format!(
        "export LSL CSV classe {} ({} octets)",
        lea.classe,
        csv_lsl.len()
    ));

    let csv_lsun = ent
        .exporter_lsun_csv_for_actor(
            Role::Prof.as_str(),
            ExporterLsun {
                etablissement_id: cast.etablissement_id,
                periode_id: periode.id,
                classe: Some(lea.classe.clone()),
                jour_reference: jour,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !csv_lsun.contains("domaine;item;niveau") || !csv_lsun.contains(&lea.nom) {
        return Err(format!("export LSUN invalide: {csv_lsun}"));
    }
    r.geste(format!(
        "export LSUN CSV classe {} ({} octets)",
        lea.classe,
        csv_lsun.len()
    ));

    let csv_onde = ent
        .exporter_onde_csv_for_actor(
            Role::Administratif.as_str(),
            ExporterOnde {
                etablissement_id: cast.etablissement_id,
                classe: Some(lea.classe.clone()),
                jour_reference: jour,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !csv_onde.contains("ine;nom;") || !csv_onde.contains(&lea.nom) {
        return Err(format!("export ONDE invalide: {csv_onde}"));
    }
    r.geste(format!(
        "export ONDE/BE1D CSV classe {} ({} octets)",
        lea.classe,
        csv_onde.len()
    ));

    let csv_notanet = ent
        .exporter_notanet_csv_for_actor(
            Role::Prof.as_str(),
            ExporterNotanet {
                etablissement_id: cast.etablissement_id,
                periode_id: periode.id,
                classe: lea.classe.clone(),
            },
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !csv_notanet.contains("ine;nom;") {
        return Err(format!("export Notanet invalide: {csv_notanet}"));
    }
    r.geste(format!(
        "export Notanet CSV classe {} ({} octets)",
        lea.classe,
        csv_notanet.len()
    ));

    // Fermeture totale → appréciation refusée
    ent.set_note_periode_statut(
        cast.etablissement_id,
        periode.id,
        NotePeriodeStatut::Fermee,
        Some(cast.administratif.user_id.to_string()),
    )
    .await
    .map_err(|e| e.to_string())?;
    let bloque = ent
        .saisir_note(SaisirNote {
            etablissement_id: cast.etablissement_id,
            eleve_id: lea.id,
            matiere_id: maths.id,
            valeur: 10.0,
            sur: 20.0,
            date_note: jour,
            auteur_user_id: cast.prof.user_id,
            libelle: Some("après fermeture".into()),
            devoir_id: None,
            periode_id: Some(periode.id),
        })
        .await;
    if bloque.is_ok() {
        return Err("période fermée devrait bloquer la saisie".into());
    }
    let bloque_app = ent
        .saisir_appreciation_for_actor(
            Role::Prof.as_str(),
            SaisirAppreciation {
                etablissement_id: cast.etablissement_id,
                periode_id: periode.id,
                eleve_id: lea.id,
                matiere_id: Some(maths.id),
                texte: "trop tard".into(),
                actor_user_id: cast.prof.user_id,
            },
        )
        .await;
    if bloque_app.is_ok() {
        return Err("période fermée doit bloquer les appréciations".into());
    }
    r.geste("période fermée → notes + appréciations refusées (OK)");

    let bulletin = ent
        .publier_bulletin_for_actor(
            Role::Administratif.as_str(),
            cast.etablissement_id,
            lea.id,
            periode.id,
            cast.administratif.user_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !bulletin
        .code
        .as_deref()
        .unwrap_or("")
        .starts_with("BULLETIN_")
    {
        return Err("code bulletin attendu".into());
    }
    r.geste(format!(
        "bulletin PDF publié (période fermée) → doc {}",
        bulletin.titre
    ));

    let dossier_bul = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            lea.id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "parent dossier bulletin".to_string())?;
    if !dossier_bul
        .documents
        .iter()
        .any(|d| d.id == bulletin.id || d.code.as_deref() == bulletin.code.as_deref())
    {
        return Err("parent doit voir le bulletin dans documents dossier".into());
    }
    r.geste("parent Lily voit le bulletin sur le dossier (docs famille)");

    let refuse_ouverte = ent
        .publier_bulletin_for_actor(
            Role::Administratif.as_str(),
            cast.etablissement_id,
            lea.id,
            periode.id,
            cast.administratif.user_id,
        )
        .await;
    // encore fermée → 2e publication OK ; on teste plutôt après réouverture
    let _ = refuse_ouverte;

    // Rouvrir pour ne pas casser le reste du lab
    ent.set_note_periode_statut(
        cast.etablissement_id,
        periode.id,
        NotePeriodeStatut::Ouverte,
        Some(cast.administratif.user_id.to_string()),
    )
    .await
    .map_err(|e| e.to_string())?;

    let refuse_si_ouverte = ent
        .publier_bulletin_for_actor(
            Role::Administratif.as_str(),
            cast.etablissement_id,
            lea.id,
            periode.id,
            cast.administratif.user_id,
        )
        .await;
    if refuse_si_ouverte.is_ok() {
        return Err("bulletin sur période ouverte aurait dû être refusé".into());
    }
    r.geste("refus bulletin si période rouverte (attendu)");

    let compta_tente = ent
        .actor_can_read_notes(
            cast.etablissement_id,
            cast.compta.user_id,
            Role::Compta.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if compta_tente {
        return Err("compta ne doit pas lire les notes".into());
    }
    r.geste("compta refusée sur notes (attendu)");
    r.trou(
        "notes: périodes + devoirs (GET/POST /devoirs + GET /eleves/{id}/devoirs) + saisie + moyennes + appréciations + exports LSL/LSUN/ONDE OK — UI /app/notes (search élèves + grille classe/devoir) + /eleve + /famille + natives",
    );
    Ok(())
}

async fn explorer_appels_et_absences(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# appels → absences (plusieurs classes / issues)");

    // Créneau Rogue / 4B Potions : plusieurs absents
    let classe_4b: Vec<&LaboEleve> = cast.eleves.iter().filter(|e| e.classe == "4B").collect();
    let mes = ent
        .list_mes_creneaux(
            cast.etablissement_id,
            cast.profs[0].user_id,
            Role::Prof.as_str(),
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if mes.iter().all(|c| c.id != cast.creneau_rogue_4b) {
        return Err("Rogue devrait voir son créneau Potions 4B".into());
    }
    r.geste(format!("Rogue voit {} créneau(x) aujourd'hui", mes.len()));

    let refuse_mcg = ent
        .ouvrir_appel(
            cast.etablissement_id,
            cast.creneau_rogue_4b,
            jour,
            cast.profs[1].user_id,
            Role::Prof.as_str(),
        )
        .await;
    if refuse_mcg.is_ok() {
        return Err("McGonagall ne doit pas ouvrir le créneau de Rogue".into());
    }
    r.geste("McGonagall refusée sur créneau Rogue (attendu)");

    let (appel, _) = ent
        .ouvrir_appel(
            cast.etablissement_id,
            cast.creneau_rogue_4b,
            jour,
            cast.profs[0].user_id,
            Role::Prof.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if appel.matiere_libelle.as_deref() != Some("Potions") {
        return Err("appel doit porter la matière du créneau".into());
    }
    for eleve in classe_4b.iter().take(3) {
        ent.marquer_appel_ligne(
            cast.etablissement_id,
            appel.id,
            eleve.id,
            LigneEtat::Absent,
        )
        .await
        .map_err(|e| e.to_string())?;
        r.geste(format!("appel Potions 4B: {} absent", eleve.prenom));
    }
    ent.clore_appel(
        cast.etablissement_id,
        appel.id,
        Some(cast.profs[0].user_id.to_string()),
    )
    .await
    .map_err(|e| e.to_string())?;
    r.geste("appel Potions 4B clôturé → Brain remonte ent_absence");

    let file = ent
        .list_absences_a_traiter(cast.etablissement_id, jour)
        .await
        .map_err(|e| e.to_string())?;
    if file.len() < 3 {
        return Err(format!(
            "attendu ≥3 absences a_traiter après appel, got {}",
            file.len()
        ));
    }
    r.geste(format!("file VS a_traiter: {}", file.len()));

    // Chemins divergents sur 3 absences 4B — exploration, pas checklist orale.
    let abs_lea = file
        .iter()
        .find(|a| a.eleve_id == cast.eleve_lea_id)
        .ok_or_else(|| "Léa absente attendue".to_string())?;

    // Parent Léa soumet un motif → reste a_traiter
    let soumise = ent
        .justifier_absence_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            abs_lea.id,
            "rendez-vous médical".into(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if soumise.statut != AbsenceStatut::ATraiter || soumise.motif_justif.is_none() {
        return Err("soumission parent doit garder a_traiter + motif_justif".into());
    }
    r.geste("parent Léa soumet motif → toujours a_traiter");

    // CPE tranche : accepte
    let acceptee = ent
        .accepter_justificatif_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            abs_lea.id,
            Some("ok certificat".into()),
        )
        .await
        .map_err(|e| e.to_string())?;
    if acceptee.statut != AbsenceStatut::Justifiee {
        return Err("CPE accepter → justifiee".into());
    }
    r.geste("CPE accepte justif Léa → justifiee");

    // 2ᵉ élève 4B : soumission + refus CPE (si flag) ou constat de blocage
    let autres_4b: Vec<&LaboEleve> = classe_4b
        .iter()
        .copied()
        .filter(|e| e.id != cast.eleve_lea_id)
        .collect();
    if let Some(autre) = autres_4b.first() {
        let abs_autre = file
            .iter()
            .find(|a| a.eleve_id == autre.id)
            .ok_or_else(|| format!("absence {} manquante", autre.prenom))?;

        let tentative = ent
            .justifier_absence_for_actor(
                cast.etablissement_id,
                autre.parent.user_id,
                Role::Parent.as_str(),
                abs_autre.id,
                "motif parent".into(),
            )
            .await;
        match (autre.peut_justifier, tentative) {
            (true, Ok(a)) => {
                r.geste(format!(
                    "parent {} soumet (flag ok) statut={}",
                    autre.prenom,
                    a.statut.as_str()
                ));
                let refusee = ent
                    .refuser_justificatif_for_actor(
                        cast.etablissement_id,
                        cast.cpes[1.min(cast.cpes.len() - 1)].user_id,
                        Role::Cpe.as_str(),
                        abs_autre.id,
                        Some("pièce insuffisante".into()),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if refusee.statut != AbsenceStatut::Refusee {
                    return Err("refus CPE attendu".into());
                }
                r.geste(format!("CPE refuse justif {}", autre.prenom));
            }
            (false, Err(_)) => r.geste(format!(
                "parent {} bloqué justif (flag off) — attendu",
                autre.prenom
            )),
            (true, Err(e)) => return Err(format!("parent {} devrait pouvoir: {e}", autre.prenom)),
            (false, Ok(_)) => {
                return Err(format!(
                    "parent {} a justifié sans flag — faille",
                    autre.prenom
                ))
            }
        }
    }

    // 3ᵉ élève 4B : annulation prof (erreur de pointage) — autre direction
    if let Some(troisieme) = autres_4b.get(1) {
        let abs_3 = file
            .iter()
            .find(|a| a.eleve_id == troisieme.id)
            .ok_or_else(|| format!("absence {} manquante", troisieme.prenom))?;
        let annulee = ent
            .annuler_absence_vers_present_for_actor(
                cast.etablissement_id,
                cast.profs[0].user_id,
                Role::Prof.as_str(),
                abs_3.id,
                Some("erreur de pointage".into()),
            )
            .await
            .map_err(|e| e.to_string())?;
        if annulee.statut != AbsenceStatut::AnnuleePresent {
            return Err("annulation → annulee_present".into());
        }
        let encore_absent = ent
            .eleve_est_absent(cast.etablissement_id, troisieme.id, jour)
            .await
            .map_err(|e| e.to_string())?;
        if encore_absent {
            return Err("annulée ne doit plus compter comme absent".into());
        }
        r.geste(format!(
            "prof annule absence {} → présent (effectif)",
            troisieme.prenom
        ));
    }

    // Prof tente de trancher un justif → interdit
    if let Some(reste) = ent
        .list_absences_a_traiter(cast.etablissement_id, jour)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .next()
    {
        let prof_tranche = ent
            .accepter_justificatif_for_actor(
                cast.etablissement_id,
                cast.prof.user_id,
                Role::Prof.as_str(),
                reste.id,
                None,
            )
            .await;
        if prof_tranche.is_ok() {
            return Err("prof ne doit pas trancher justificatif".into());
        }
        r.geste("prof refusé sur tranchage justif (attendu)");
    }

    r.trou(
        "absences VS: appel→file→justif parent→accepter/refuser CPE + annuler→présent OK — HTTP /absences/{id}/accepter-justificatif|refuser-justificatif|annuler-present + UI /app/absences (actions inline sans prompt)",
    );
    Ok(())
}

async fn explorer_edt_exceptions(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# EDT — exceptions / remplacements nominatifs");

    let j7 = jour + Duration::days(7);
    let j14 = jour + Duration::days(14);
    let rogue = cast.profs[0].user_id;
    let mcg = cast.profs[1].user_id;
    let cpe = cast.cpe.user_id;

    // Prof ne peut pas poser d'exception.
    let prof_refuse = ent
        .poser_edt_exception_for_actor(
            Role::Prof.as_str(),
            PoserEdtException {
                etablissement_id: cast.etablissement_id,
                creneau_id: cast.creneau_rogue_4b,
                jour: j7,
                kind: EdtExceptionKind::Annulation,
                remplacant_user_id: None,
                nouvelle_salle: None,
                motif: "tentative prof".into(),
                actor_user_id: rogue,
            },
        )
        .await;
    if prof_refuse.is_ok() {
        return Err("prof ne doit pas poser d'exception EDT".into());
    }
    r.geste("prof refuse pose exception EDT (attendu)");

    // CPE : Rogue remplacé par McGonagall à J+7.
    let ex = ent
        .poser_edt_exception_for_actor(
            Role::Cpe.as_str(),
            PoserEdtException {
                etablissement_id: cast.etablissement_id,
                creneau_id: cast.creneau_rogue_4b,
                jour: j7,
                kind: EdtExceptionKind::Remplacement,
                remplacant_user_id: Some(mcg),
                nouvelle_salle: None,
                motif: "Rogue malade — McGonagall remplace".into(),
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if ex.kind != EdtExceptionKind::Remplacement || ex.remplacant_user_id != Some(mcg) {
        return Err("exception remplacement McGonagall attendue".into());
    }
    r.geste(format!("CPE pose remplacement Potions 4B J+7 → McGonagall ({j7})"));

    let mes_rogue = ent
        .list_mes_creneaux(
            cast.etablissement_id,
            rogue,
            Role::Prof.as_str(),
            j7,
        )
        .await
        .map_err(|e| e.to_string())?;
    if mes_rogue.iter().any(|c| c.id == cast.creneau_rogue_4b) {
        return Err("Rogue ne doit plus voir son créneau le jour du remplacement".into());
    }
    r.geste("Rogue ne voit plus Potions 4B à J+7");

    let mes_mcg = ent
        .list_mes_creneaux(cast.etablissement_id, mcg, Role::Prof.as_str(), j7)
        .await
        .map_err(|e| e.to_string())?;
    if mes_mcg.iter().all(|c| c.id != cast.creneau_rogue_4b) {
        return Err("McGonagall doit voir le créneau Rogue en remplacante".into());
    }
    r.geste("McGonagall voit Potions 4B en remplacante");

    let refuse_rogue = ent
        .ouvrir_appel(
            cast.etablissement_id,
            cast.creneau_rogue_4b,
            j7,
            rogue,
            Role::Prof.as_str(),
        )
        .await;
    if refuse_rogue.is_ok() {
        return Err("titulaire remplacé ne doit pas ouvrir l'appel".into());
    }
    r.geste("Rogue refuse ouverture appel J+7 (attendu)");

    let (appel_rempl, _) = ent
        .ouvrir_appel(
            cast.etablissement_id,
            cast.creneau_rogue_4b,
            j7,
            mcg,
            Role::Prof.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if appel_rempl.classe != "4B"
        || appel_rempl.creneau_id != Some(cast.creneau_rogue_4b)
        || appel_rempl.auteur_user_id != mcg
    {
        return Err("appel remplacant doit être McGonagall sur créneau Rogue 4B".into());
    }
    r.geste("McGonagall ouvre l'appel Potions 4B en remplacante");

    // Annulation Transfiguration 5A à J+14.
    ent.poser_edt_exception_for_actor(
        Role::Cpe.as_str(),
        PoserEdtException {
            etablissement_id: cast.etablissement_id,
            creneau_id: cast.creneau_mcg_5a,
            jour: j14,
            kind: EdtExceptionKind::Annulation,
            remplacant_user_id: None,
            nouvelle_salle: None,
            motif: "sortie pedagogique niveau".into(),
            actor_user_id: cpe,
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    r.geste(format!("CPE annule Transfiguration 5A à J+14 ({j14})"));

    let mes_annule = ent
        .list_mes_creneaux(cast.etablissement_id, mcg, Role::Prof.as_str(), j14)
        .await
        .map_err(|e| e.to_string())?;
    if mes_annule.iter().any(|c| c.id == cast.creneau_mcg_5a) {
        return Err("créneau annulé ne doit plus apparaître".into());
    }
    let ouvrir_annule = ent
        .ouvrir_appel(
            cast.etablissement_id,
            cast.creneau_mcg_5a,
            j14,
            mcg,
            Role::Prof.as_str(),
        )
        .await;
    if ouvrir_annule.is_ok() {
        return Err("appel interdit sur créneau annulé".into());
    }
    r.geste("créneau annulé retiré de la liste · ouverture refusée");

    // Changement de salle Potions à J+14 (même weekday, créneau Rogue libre).
    ent.poser_edt_exception_for_actor(
        Role::Cpe.as_str(),
        PoserEdtException {
            etablissement_id: cast.etablissement_id,
            creneau_id: cast.creneau_rogue_4b,
            jour: j14,
            kind: EdtExceptionKind::ChangementSalle,
            remplacant_user_id: None,
            nouvelle_salle: Some("Amphi B".into()),
            motif: "travaux donjons".into(),
            actor_user_id: cpe,
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    let mes_salle = ent
        .list_mes_creneaux(cast.etablissement_id, rogue, Role::Prof.as_str(), j14)
        .await
        .map_err(|e| e.to_string())?;
    let pot = mes_salle
        .iter()
        .find(|c| c.id == cast.creneau_rogue_4b)
        .ok_or("Rogue doit voir Potions J+14")?;
    if pot.salle.as_deref() != Some("Amphi B") {
        return Err(format!(
            "salle effective attendue Amphi B, got {:?}",
            pot.salle
        ));
    }
    r.geste("changement de salle Potions → Amphi B à J+14");

    // Alternance A/B : référence = lundi de `jour` → jour = A, j7 = B.
    let ref_lundi = lundi_iso(jour);
    let cal = ent
        .configurer_edt_calendrier_ab_for_actor(
            Role::Cpe.as_str(),
            ConfigurerEdtCalendrierAb {
                etablissement_id: cast.etablissement_id,
                reference_lundi: ref_lundi,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if cal.reference_lundi != ref_lundi {
        return Err("calendrier A/B référence incorrecte".into());
    }
    r.geste(format!("CPE pose calendrier A/B · réf. lundi {ref_lundi}"));

    let creneau_b = ent
        .creer_edt_creneau(EdtCreneau {
            id: Uuid::new_v4(),
            etablissement_id: cast.etablissement_id,
            classe: "4B".into(),
            matiere_id: None,
            matiere_libelle: "Astronomie A/B".into(),
            prof_user_id: rogue,
            jour_semaine: jour_semaine_iso(jour),
            heure_debut: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            heure_fin: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            salle: Some("Tour d'astronomie".into()),
            site: String::new(),
            semaine_ab: EdtSemaineAb::B,
        })
        .await
        .map_err(|e| e.to_string())?;
    r.geste("créneau Astronomie semaine B créé pour Rogue");

    let mes_a = ent
        .list_mes_creneaux(cast.etablissement_id, rogue, Role::Prof.as_str(), jour)
        .await
        .map_err(|e| e.to_string())?;
    if mes_a.iter().any(|c| c.id == creneau_b.id) {
        return Err("créneau B ne doit pas apparaître en semaine A".into());
    }
    if mes_a.iter().all(|c| c.id != cast.creneau_rogue_4b) {
        return Err("créneau toutes (Potions) doit rester visible en semaine A".into());
    }
    r.geste("semaine A: Potions visible · Astronomie B masquée");

    let ouvrir_b_en_a = ent
        .ouvrir_appel(
            cast.etablissement_id,
            creneau_b.id,
            jour,
            rogue,
            Role::Prof.as_str(),
        )
        .await;
    if ouvrir_b_en_a.is_ok() {
        return Err("appel interdit sur créneau B en semaine A".into());
    }

    let mes_b = ent
        .list_mes_creneaux(cast.etablissement_id, rogue, Role::Prof.as_str(), j7)
        .await
        .map_err(|e| e.to_string())?;
    if mes_b.iter().all(|c| c.id != creneau_b.id) {
        return Err("créneau B doit apparaître en semaine B (J+7)".into());
    }
    r.geste(format!("semaine B ({j7}): Astronomie visible pour Rogue"));

    let (appel_b, _) = ent
        .ouvrir_appel(
            cast.etablissement_id,
            creneau_b.id,
            j7,
            rogue,
            Role::Prof.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if appel_b.creneau_id != Some(creneau_b.id) {
        return Err("appel semaine B doit cibler le créneau Astronomie".into());
    }
    r.geste("Rogue ouvre l'appel Astronomie en semaine B");

    let prof_cal = ent
        .configurer_edt_calendrier_ab_for_actor(
            Role::Prof.as_str(),
            ConfigurerEdtCalendrierAb {
                etablissement_id: cast.etablissement_id,
                reference_lundi: ref_lundi,
                actor_user_id: rogue,
            },
        )
        .await;
    if prof_cal.is_ok() {
        return Err("prof ne doit pas configurer le calendrier A/B".into());
    }
    r.geste("prof refusé sur calendrier A/B (attendu)");

    // Import CSV + détection conflits (solveur dry-run)
    let js = jour_semaine_iso(jour);
    let csv_conflit = format!(
        "classe;matiere;prof_user_id;jour_semaine;heure_debut;heure_fin;salle;semaine_ab\n\
6A;Histoire;{rogue};{js};10:00;11:00;Amphi A;toutes\n\
6B;Géographie;{rogue};{js};10:30;11:30;Amphi B;toutes\n"
    );
    let dry = ent
        .importer_edt_csv_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            &csv_conflit,
            true,
        )
        .await
        .map_err(|e| e.to_string())?;
    if dry.conflicts.is_empty() {
        return Err("dry-run doit détecter conflit prof".into());
    }
    if !dry
        .conflicts
        .iter()
        .any(|c| c.kind == domain_eleve::EdtConflictKind::ProfDouble)
    {
        return Err("conflit attendu = ProfDouble".into());
    }
    r.geste(format!(
        "import EDT dry-run → {} conflit(s) (prof double détecté)",
        dry.conflicts.len()
    ));
    let apply_conflit = ent
        .importer_edt_csv_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            &csv_conflit,
            false,
        )
        .await;
    if apply_conflit.is_ok() {
        return Err("apply avec conflits doit être refusé".into());
    }
    r.geste("apply EDT refusé si conflits (attendu)");

    let mcg = cast.profs[1].user_id;
    let csv_ok = format!(
        "classe;matiere;prof_user_id;jour_semaine;heure_debut;heure_fin;salle;semaine_ab\n\
6A;Histoire;{rogue};{js};18:00;19:00;Salle H;toutes\n\
6B;Géographie;{mcg};{js};18:00;19:00;Salle G;toutes\n"
    );
    let applied = ent
        .importer_edt_csv_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            &csv_ok,
            false,
        )
        .await
        .map_err(|e| e.to_string())?;
    if applied.ok_count != 2 || !applied.conflicts.is_empty() {
        return Err(format!(
            "apply EDT attendu 2 créneaux, got ok={} conflits={}",
            applied.ok_count,
            applied.conflicts.len()
        ));
    }
    let n_imp = ent
        .count_edt_imports(cast.etablissement_id)
        .await
        .map_err(|e| e.to_string())?;
    if n_imp < 2 {
        return Err(format!("journal import EDT attendu ≥2, got {n_imp}"));
    }
    r.geste(format!(
        "import EDT apply → {} créneau(x) · journal={n_imp}",
        applied.ok_count
    ));
    let prof_imp = ent
        .importer_edt_csv_for_actor(
            cast.etablissement_id,
            Role::Prof.as_str(),
            rogue,
            &csv_ok,
            true,
        )
        .await;
    if prof_imp.is_ok() {
        return Err("prof ne doit pas importer l'EDT".into());
    }
    r.geste("prof refusé sur import EDT (attendu)");

    // Import Pronote (e-mail enseignant + jours FR + 08h00)
    let csv_pronote = format!(
        "Classe;Matière;Enseignant;Jour;Début;Fin;Salle;Fréquence\n\
5C;Latin;{};Mercredi;14h00;15h00;Salle L;H\n\
5C;Astronomie;{};Jeudi;16h00;17h00;Tour;S2\n",
        cast.profs[0].email, cast.profs[1].email
    );
    let pronote = ent
        .importer_edt_csv_format_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            &csv_pronote,
            false,
            "pronote",
        )
        .await
        .map_err(|e| e.to_string())?;
    if pronote.ok_count != 2 || !pronote.conflicts.is_empty() {
        return Err(format!(
            "import Pronote attendu 2 créneaux, got ok={} conflits={}",
            pronote.ok_count,
            pronote.conflicts.len()
        ));
    }
    r.geste(format!(
        "import Pronote apply → {} créneau(x) (e-mail + jours FR)",
        pronote.ok_count
    ));
    let pronote_bad = ent
        .importer_edt_csv_format_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            "Classe;Matière;Enseignant;Jour;Début;Fin\n\
5C;Histoire;inconnu@nowhere.lab;Lundi;08h00;09h00\n",
            true,
            "pronote",
        )
        .await;
    if pronote_bad.is_ok() {
        return Err("Pronote avec e-mail inconnu doit échouer".into());
    }
    r.geste("import Pronote refuse enseignant inconnu (attendu)");

    // Mini-solveur greedy
    let solve_dry = ent
        .resoudre_edt_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            vec![
                domain_eleve::EdtDemande {
                    classe: "3A".into(),
                    matiere_libelle: "Maths solver".into(),
                    prof_user_id: rogue,
                    duree_minutes: 60,
                    salle: Some("Solve-1".into()),
                    semaine_ab: EdtSemaineAb::Toutes,
                },
                domain_eleve::EdtDemande {
                    classe: "3B".into(),
                    matiere_libelle: "Histoire solver".into(),
                    prof_user_id: mcg,
                    duree_minutes: 60,
                    salle: Some("Solve-2".into()),
                    semaine_ab: EdtSemaineAb::Toutes,
                },
            ],
            true,
        )
        .await
        .map_err(|e| e.to_string())?;
    if solve_dry.placed.len() != 2 || !solve_dry.unplaced.is_empty() {
        return Err(format!(
            "solveur dry-run attendu 2 placés, got {} / unplaced={}",
            solve_dry.placed.len(),
            solve_dry.unplaced.len()
        ));
    }
    r.geste("solveur EDT greedy dry-run → 2 créneaux placés");
    let solve_apply = ent
        .resoudre_edt_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            vec![
                domain_eleve::EdtDemande {
                    classe: "3A".into(),
                    matiere_libelle: "Maths solver".into(),
                    prof_user_id: rogue,
                    duree_minutes: 60,
                    salle: Some("Solve-1".into()),
                    semaine_ab: EdtSemaineAb::Toutes,
                },
                domain_eleve::EdtDemande {
                    classe: "3B".into(),
                    matiere_libelle: "Histoire solver".into(),
                    prof_user_id: mcg,
                    duree_minutes: 60,
                    salle: Some("Solve-2".into()),
                    semaine_ab: EdtSemaineAb::Toutes,
                },
            ],
            false,
        )
        .await
        .map_err(|e| e.to_string())?;
    if solve_apply.placed.len() != 2 {
        return Err("solveur apply doit créer 2 créneaux".into());
    }
    r.geste(format!(
        "solveur EDT greedy apply → {} créneau(x)",
        solve_apply.placed.len()
    ));

    Ok(())
}

async fn explorer_foyer_admin(
    ent: &PgEnt,
    cast: &LaboCast,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# administratif — fiche foyer + parts atomiques");

    let list = ent
        .list_foyers_for_actor(
            cast.etablissement_id,
            Role::Administratif.as_str(),
            Some("Potter"),
            40,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !list.iter().any(|(id, _, _)| *id == cast.foyer_lea_id) {
        return Err("liste foyers: foyer Léa introuvable".into());
    }
    r.geste(format!(
        "admini liste foyers (q=Potter) → {} résultat(s)",
        list.len()
    ));

    let detail = ent
        .get_foyer_detail_for_actor(
            cast.etablissement_id,
            Role::Administratif.as_str(),
            cast.foyer_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "détail foyer Léa introuvable".to_string())?;
    let (_, libelle, liens) = detail;
    if liens.is_empty() {
        return Err("foyer Léa sans liens".into());
    }
    let enfant = format!("{} {}", liens[0].eleve_prenom, liens[0].eleve_nom);
    r.geste(format!(
        "fiche foyer {:?} — {} lien(s), enfant={enfant}",
        libelle,
        liens.len()
    ));

    let lily = liens
        .iter()
        .find(|l| l.user_id == cast.parent_lea.user_id)
        .ok_or_else(|| "lien Lily manquant sur foyer".to_string())?;

    let bad_solo = ent
        .update_lien_for_actor(
            Role::Administratif.as_str(),
            cast.etablissement_id,
            lily.lien_id,
            ent_db::UpdateLien {
                lien: lily.lien.clone(),
                peut_voir_notes: lily.peut_voir_notes,
                peut_voir_vs: lily.peut_voir_vs,
                peut_justifier_absence: lily.peut_justifier_absence,
                peut_autoriser_sorties: lily.peut_autoriser_sorties,
                est_payeur: true,
                part_paiement: Some(60),
            },
            cast.administratif.user_id,
        )
        .await;
    if bad_solo.is_ok() {
        return Err("part 60 % seule aurait dû être refusée".into());
    }
    r.geste("refus update unitaire 60 % (somme ≠ 100) — attendu");

    // Second payeur + batch atomique 60/40 puis retrait Tom (isolation lab)
    let tom_lien = ent
        .link_parent(ent_db::CreateLien {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            user_id: cast.parent_tom.user_id,
            foyer_id: Some(cast.foyer_lea_id),
            lien: "beau-père".into(),
            peut_voir_notes: false,
            peut_voir_vs: false,
            peut_justifier_absence: false,
            peut_autoriser_sorties: false,
            est_payeur: false,
            part_paiement: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    r.geste("Tom lié non-payeur sur Léa (foyer Potter)");

    let split = ent
        .update_liens_eleve_for_actor(
            Role::Administratif.as_str(),
            cast.etablissement_id,
            cast.eleve_lea_id,
            vec![
                (
                    lily.lien_id,
                    ent_db::UpdateLien {
                        lien: lily.lien.clone(),
                        peut_voir_notes: lily.peut_voir_notes,
                        peut_voir_vs: lily.peut_voir_vs,
                        peut_justifier_absence: lily.peut_justifier_absence,
                        peut_autoriser_sorties: lily.peut_autoriser_sorties,
                        est_payeur: true,
                        part_paiement: Some(60),
                    },
                ),
                (
                    tom_lien.id,
                    ent_db::UpdateLien {
                        lien: "beau-père".into(),
                        peut_voir_notes: false,
                        peut_voir_vs: false,
                        peut_justifier_absence: false,
                        peut_autoriser_sorties: false,
                        est_payeur: true,
                        part_paiement: Some(40),
                    },
                ),
            ],
            cast.administratif.user_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    let sum: i32 = split
        .iter()
        .filter(|l| l.est_payeur)
        .map(|l| i32::from(l.part_paiement.unwrap_or(0)))
        .sum();
    if sum != 100 {
        return Err(format!("batch 60/40: somme={sum}").into());
    }
    r.geste("batch atomique parts 60/40 Lily+Tom OK");

    let reject_batch = ent
        .update_liens_eleve_for_actor(
            Role::Administratif.as_str(),
            cast.etablissement_id,
            cast.eleve_lea_id,
            vec![
                (
                    lily.lien_id,
                    ent_db::UpdateLien {
                        lien: lily.lien.clone(),
                        peut_voir_notes: lily.peut_voir_notes,
                        peut_voir_vs: lily.peut_voir_vs,
                        peut_justifier_absence: lily.peut_justifier_absence,
                        peut_autoriser_sorties: lily.peut_autoriser_sorties,
                        est_payeur: true,
                        part_paiement: Some(50),
                    },
                ),
                (
                    tom_lien.id,
                    ent_db::UpdateLien {
                        lien: "beau-père".into(),
                        peut_voir_notes: false,
                        peut_voir_vs: false,
                        peut_justifier_absence: false,
                        peut_autoriser_sorties: false,
                        est_payeur: true,
                        part_paiement: Some(40),
                    },
                ),
            ],
            cast.administratif.user_id,
        )
        .await;
    if reject_batch.is_ok() {
        return Err("batch 50+40 aurait dû être refusé".into());
    }
    r.geste("refus batch 50+40 (≠100) — attendu");

    // Restauration Lily 100 % puis retrait Tom (isolation parent Tom ≠ Léa)
    ent.update_liens_eleve_for_actor(
        Role::Administratif.as_str(),
        cast.etablissement_id,
        cast.eleve_lea_id,
        vec![
            (
                lily.lien_id,
                ent_db::UpdateLien {
                    lien: lily.lien.clone(),
                    peut_voir_notes: lily.peut_voir_notes,
                    peut_voir_vs: lily.peut_voir_vs,
                    peut_justifier_absence: lily.peut_justifier_absence,
                    peut_autoriser_sorties: lily.peut_autoriser_sorties,
                    est_payeur: true,
                    part_paiement: Some(100),
                },
            ),
            (
                tom_lien.id,
                ent_db::UpdateLien {
                    lien: "beau-père".into(),
                    peut_voir_notes: false,
                    peut_voir_vs: false,
                    peut_justifier_absence: false,
                    peut_autoriser_sorties: false,
                    est_payeur: false,
                    part_paiement: None,
                },
            ),
        ],
        cast.administratif.user_id,
    )
    .await
    .map_err(|e| e.to_string())?;

    ent.delete_lien_for_actor(
        Role::Administratif.as_str(),
        cast.etablissement_id,
        tom_lien.id,
        cast.administratif.user_id,
    )
    .await
    .map_err(|e| e.to_string())?;
    r.geste("restauration Lily 100 % + retrait lien Tom (lab suite)");

    Ok(())
}

async fn explorer_adultes_admin(
    ent: &PgEnt,
    cast: &LaboCast,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# administratif — Adultes (annuaire personnel)");

    let list = ent
        .search_staff(cast.etablissement_id, "prof", 40)
        .await
        .map_err(|e| e.to_string())?;
    if list.is_empty() {
        return Err("annuaire Adultes: aucun prof".into());
    }
    r.geste(format!(
        "admini annuaire Adultes q=prof → {} résultat(s)",
        list.len()
    ));

    let rogue = ent
        .get_staff_member(cast.etablissement_id, cast.prof.user_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "fiche Adultes Rogue introuvable".to_string())?;
    if rogue.role != Role::Prof.as_str() {
        return Err(format!("fiche Adultes: rôle={}", rogue.role));
    }
    r.geste(format!(
        "fiche Adultes {} · rôle={}",
        rogue.display_name.as_deref().unwrap_or(&rogue.email),
        rogue.role
    ));

    let parent_block = ent
        .get_staff_member(cast.etablissement_id, cast.parent_lea.user_id)
        .await
        .map_err(|e| e.to_string())?;
    if parent_block.is_some() {
        return Err("parent ne doit pas apparaître dans Adultes".into());
    }
    r.geste("parent exclu de l'annuaire Adultes (attendu)");

    Ok(())
}

async fn explorer_parents_divergents(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# parents / foyers — droits croisés");

    // Parent Tom ne voit pas Léa
    let tom_voit_lea = ent
        .get_eleve_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if tom_voit_lea.is_some() {
        return Err("parent Tom ne doit pas voir Léa".into());
    }
    r.geste("isolation foyer: parent Tom ≠ Léa");

    // Déclaration manuelle staff sur un élève 5A (hors appel)
    let tom_eleve = cast
        .eleves
        .iter()
        .find(|e| e.id == cast.eleve_tom_id)
        .expect("tom");
    ent.declare_absence(DeclareAbsence {
        etablissement_id: cast.etablissement_id,
        eleve_id: tom_eleve.id,
        debut: jour,
        fin: jour,
        motif: "familial".into(),
        actor_user_id: Some(cast.cpe.user_id.to_string()),
    })
    .await
    .map_err(|e| e.to_string())?;
    r.geste("CPE déclare absence Tom hors appel (motif référentiel familial)");

    let tom_peut = ent
        .actor_can_justifier_absence(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_tom_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "parent Tom peut justifier son enfant? {tom_peut} (flag={})",
        tom_eleve.peut_justifier
    ));
    if tom_peut != tom_eleve.peut_justifier {
        return Err("flag justif Tom incohérent".into());
    }

    Ok(())
}

async fn explorer_portail_famille(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# portail famille (mes enfants / justifs / carnet)");

    // Staff ne passe pas par mes_enfants
    let staff_tente = ent
        .mes_enfants_for_parent(
            cast.etablissement_id,
            cast.prof.user_id,
            Role::Prof.as_str(),
        )
        .await;
    if staff_tente.is_ok() {
        return Err("prof ne doit pas lister mes_enfants".into());
    }
    r.geste("staff refusé sur /famille/enfants (attendu)");

    let enfants_lily = ent
        .mes_enfants_for_parent(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if enfants_lily.is_empty() {
        return Err("Lily doit avoir au moins Harry".into());
    }
    if !enfants_lily.iter().any(|e| e.eleve.id == cast.eleve_lea_id) {
        return Err("Harry absent de mes enfants Lily".into());
    }
    r.geste(format!(
        "Lily voit {} enfant(s) liés (Harry inclus)",
        enfants_lily.len()
    ));

    // Narcissa ne voit pas Harry
    let enfants_narcissa = ent
        .mes_enfants_for_parent(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if enfants_narcissa.iter().any(|e| e.eleve.id == cast.eleve_lea_id) {
        return Err("Narcissa ne doit pas voir Harry".into());
    }
    r.geste(format!(
        "Narcissa voit {} enfant(s) — pas Harry (isolation OK)",
        enfants_narcissa.len()
    ));

    // Prévenir absence depuis le parent (Lily / Harry)
    let prevenance = ent
        .declare_absence(DeclareAbsence {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            debut: jour + Duration::days(3),
            fin: jour + Duration::days(3),
            motif: "maladie".into(),
            actor_user_id: Some(cast.parent_lea.user_id.to_string()),
        })
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Lily prévient absence Harry J+3 → statut={}",
        prevenance.statut.as_str()
    ));

    let a_justifier = ent
        .absences_a_justifier_eleve(cast.etablissement_id, cast.eleve_lea_id)
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Harry : {} absence(s) sans motif parent",
        a_justifier.len()
    ));

    let carnet_attente = ent
        .carnet_a_signer_eleve(cast.etablissement_id, cast.eleve_lea_id)
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Harry : {} entrée(s) carnet à signer",
        carnet_attente.len()
    ));

    // Notes visibles selon flag
    let harry = enfants_lily
        .iter()
        .find(|e| e.eleve.id == cast.eleve_lea_id)
        .expect("harry");
    if harry.peut_voir_notes {
        let notes = ent
            .notes_for_eleve(cast.etablissement_id, cast.eleve_lea_id)
            .await
            .map_err(|e| e.to_string())?;
        r.geste(format!(
            "Lily lit {} note(s) Harry (flag notes OK)",
            notes.len()
        ));
    }

    r.trou(
        "portail famille: messagerie+PJ + push + résumé + sorties + docs + menus + passages badge + prévenance + points VS + sanctions/incidents + factures/quittances + cahier + alertes CPE + internat + conseils + fiche santé/PAI/ordonnance + dispenses EPS + passages + moyennes + psy + vœux/info-comp + stages + journal + EDT + réinscription OK — web /famille + natives SwiftUI/Kotlin",
    );
    Ok(())
}

async fn explorer_messagerie(
    ent: &PgEnt,
    cast: &LaboCast,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# messagerie établissement ↔ famille");

    let (fil, _) = ent
        .ouvrir_fil_for_actor(
            Role::Cpe.as_str(),
            OuvrirFil {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                sujet: "Rendez-vous VS".into(),
                premier_message: "Bonjour, pouvons-nous convenir d'un créneau ?".into(),
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("CPE ouvre fil « {} » sur Harry", fil.sujet));

    let push_msg = ent
        .count_push_kind(cast.etablissement_id, "messagerie")
        .await
        .map_err(|e| e.to_string())?;
    if push_msg < 1 {
        return Err("push messagerie attendu sur device Lily".into());
    }
    if ent.push().count_with_title_contains("Rendez-vous") < 1 {
        return Err("push mémoire doit tracer le sujet fil".into());
    }
    r.geste(format!("push messagerie dry-run → {push_msg} notif(s) outbox"));

    let reponse = ent
        .repondre_fil_for_actor(
            Role::Parent.as_str(),
            RepondreFil {
                etablissement_id: cast.etablissement_id,
                fil_id: fil.id,
                corps: "Oui, je suis disponible jeudi.".into(),
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("Lily répond au fil → « {} »", reponse.corps));

    // PJ : Lily joint un justificatif PDF à sa réponse
    let piece = ent
        .attacher_piece_for_actor(
            Role::Parent.as_str(),
            AttacherPiece {
                etablissement_id: cast.etablissement_id,
                message_id: reponse.id,
                nom: "justif-rdv.txt".into(),
                mime_type: "text/plain".into(),
                contenu: b"Disponible jeudi 14h - Lily".to_vec(),
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Lily joint « {} » ({} o) à sa réponse",
        piece.nom, piece.taille_octets
    ));

    // Narcissa ne peut pas télécharger la PJ du fil Harry
    let narc_dl = ent
        .telecharger_piece_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            piece.id,
        )
        .await;
    if narc_dl.is_ok() {
        return Err("Narcissa ne doit pas télécharger la PJ Harry".into());
    }

    // CPE ne peut pas attacher sur le message de Lily
    let cpe_attach = ent
        .attacher_piece_for_actor(
            Role::Cpe.as_str(),
            AttacherPiece {
                etablissement_id: cast.etablissement_id,
                message_id: reponse.id,
                nom: "intrus.txt".into(),
                mime_type: "text/plain".into(),
                contenu: b"nope".to_vec(),
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await;
    if cpe_attach.is_ok() {
        return Err("CPE ne doit pas attacher sur le message Lily".into());
    }
    r.geste("PJ: Narcissa bloquée au téléchargement · CPE refusé à l'attache (attendu)");

    // Accusé : Lily ouvre le fil → message CPE marqué lu
    let (_, lues_lily) = ent
        .list_messages_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            fil.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    let cpe_msg = lues_lily
        .iter()
        .find(|m| m.message.auteur_user_id == cast.cpe.user_id)
        .ok_or("message CPE introuvable dans le fil")?;
    if !cpe_msg.lu_par_moi || cpe_msg.nb_accuses < 1 {
        return Err(format!(
            "accusé Lily manquant sur msg CPE (lu_par_moi={}, nb={})",
            cpe_msg.lu_par_moi, cpe_msg.nb_accuses
        ));
    }
    r.geste(format!(
        "Lily accuse lecture du msg CPE ({} accusé)",
        cpe_msg.nb_accuses
    ));

    // Accusé : CPE rouvre → réponse Lily marquée lue ; propre msg CPE pas auto-accusé par lui
    let (_, lues_cpe) = ent
        .list_messages_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            fil.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    let lily_msg = lues_cpe
        .iter()
        .find(|m| m.message.id == reponse.id)
        .ok_or("réponse Lily introuvable côté CPE")?;
    if !lily_msg.lu_par_moi || lily_msg.nb_accuses < 1 {
        return Err("accusé CPE manquant sur réponse Lily".into());
    }
    let propre = lues_cpe
        .iter()
        .find(|m| m.message.auteur_user_id == cast.cpe.user_id)
        .ok_or("propre msg CPE introuvable")?;
    if propre.lu_par_moi {
        return Err("CPE ne doit pas s'accuser son propre message".into());
    }
    if propre.nb_accuses < 1 {
        return Err("msg CPE doit avoir l'accusé Lily".into());
    }
    r.geste(format!(
        "CPE accuse lecture réponse Lily ; son msg a {} accusé(s)",
        propre.nb_accuses
    ));

    let lily_avec_pj = lues_cpe
        .iter()
        .find(|m| m.message.id == reponse.id)
        .ok_or("réponse Lily absente pour vérif PJ")?;
    if lily_avec_pj.pieces.len() != 1 || lily_avec_pj.pieces[0].id != piece.id {
        return Err("CPE doit voir 1 PJ sur la réponse Lily".into());
    }
    let dl = ent
        .telecharger_piece_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            piece.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if dl.contenu != b"Disponible jeudi 14h - Lily".to_vec() {
        return Err("contenu PJ téléchargé incorrect".into());
    }
    r.geste(format!(
        "CPE télécharge PJ « {} » ({} o)",
        dl.nom, dl.taille_octets
    ));

    // Isolation : Narcissa ne lit pas le fil Harry
    let narcissa = ent
        .list_messages_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            fil.id,
        )
        .await;
    if narcissa.is_ok() {
        return Err("Narcissa ne doit pas lire le fil Harry".into());
    }
    r.geste("Narcissa bloquée sur fil Harry (attendu)");

    let fils_lily = ent
        .list_fils_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            None,
            false,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !fils_lily.iter().any(|f| f.id == fil.id) {
        return Err("Lily doit voir le fil Harry".into());
    }
    r.geste(format!("Lily voit {} fil(s) dans sa boîte", fils_lily.len()));

    // Compta hors messagerie staff
    let compta = ent
        .ouvrir_fil_for_actor(
            Role::Compta.as_str(),
            OuvrirFil {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                sujet: "Facture".into(),
                premier_message: "relance".into(),
                actor_user_id: cast.compta.user_id,
            },
        )
        .await;
    if compta.is_ok() {
        return Err("compta ne doit pas ouvrir de fil messagerie".into());
    }
    r.geste("compta refusée à l'ouverture de fil (attendu)");

    // Modèles staff + archive boîte établissement
    let modele = ent
        .creer_msg_modele_for_actor(
            Role::Cpe.as_str(),
            CreerMsgModele {
                etablissement_id: cast.etablissement_id,
                titre: "Convocation VS".into(),
                sujet: "RDV — {prenom} {nom}".into(),
                corps: "Bonjour, concernant {prenom} ({classe}), merci de prendre contact.".into(),
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("CPE crée modèle « {} »", modele.titre));

    let (fil_modele, msg_modele) = ent
        .ouvrir_fil_depuis_modele_for_actor(
            Role::Cpe.as_str(),
            cast.etablissement_id,
            cast.cpe.user_id,
            modele.id,
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if fil_modele.modele_id != Some(modele.id) {
        return Err("fil depuis modèle doit porter modele_id".into());
    }
    if !fil_modele.sujet.contains("Harry") || !fil_modele.sujet.contains("Potter") {
        return Err(format!(
            "sujet modèle non interpolé: {}",
            fil_modele.sujet
        ));
    }
    if !msg_modele.corps.contains("4B") {
        return Err(format!(
            "corps modèle sans classe: {}",
            msg_modele.corps
        ));
    }
    r.geste(format!(
        "CPE ouvre fil depuis modèle → « {} »",
        fil_modele.sujet
    ));

    let parent_modele = ent
        .ouvrir_fil_depuis_modele_for_actor(
            Role::Parent.as_str(),
            cast.etablissement_id,
            cast.parent_lea.user_id,
            modele.id,
            cast.eleve_lea_id,
        )
        .await;
    if parent_modele.is_ok() {
        return Err("parent ne doit pas ouvrir depuis un modèle staff".into());
    }
    r.geste("parent refusé sur ouverture depuis modèle (attendu)");

    let archived = ent
        .archiver_fil_for_actor(
            Role::Cpe.as_str(),
            ArchiverFil {
                etablissement_id: cast.etablissement_id,
                fil_id: fil.id,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !archived.archive {
        return Err("fil doit être archivé".into());
    }
    r.geste(format!("CPE archive le fil « {} »", fil.sujet));

    let boite = ent
        .list_fils_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            None,
            false,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if boite.iter().any(|f| f.id == fil.id) {
        return Err("fil archivé ne doit plus apparaître en boîte active".into());
    }
    if !boite.iter().any(|f| f.id == fil_modele.id) {
        return Err("fil modèle doit rester en boîte active".into());
    }
    let avec_archives = ent
        .list_fils_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            None,
            true,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !avec_archives.iter().any(|f| f.id == fil.id && f.archive) {
        return Err("fil archivé doit réapparaître avec include_archives".into());
    }
    r.geste("boîte staff: archive hors liste active · visible avec archives");

    let lily_apres = ent
        .list_fils_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            Some(cast.eleve_lea_id),
            false,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !lily_apres.iter().any(|f| f.id == fil.id) {
        return Err("Lily doit toujours voir le fil archivé côté staff".into());
    }
    r.geste("Lily conserve le fil archivé dans sa boîte famille");

    // Priorité + dossier staff
    let classed = ent
        .classer_fil_for_actor(
            Role::Cpe.as_str(),
            ClasserFil {
                etablissement_id: cast.etablissement_id,
                fil_id: fil_modele.id,
                priorite: MsgPriorite::Haute,
                dossier: Some("vs".into()),
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if classed.priorite != MsgPriorite::Haute || classed.dossier.as_deref() != Some("vs") {
        return Err("classement priorité/dossier incorrect".into());
    }
    r.geste("CPE classe le fil modèle → priorité haute · dossier vs");

    let boite_vs = ent
        .list_fils_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            None,
            false,
            Some("vs"),
        )
        .await
        .map_err(|e| e.to_string())?;
    if boite_vs.iter().all(|f| f.id != fil_modele.id) {
        return Err("filtre dossier vs doit inclure le fil classé".into());
    }
    let idx_haute = boite_vs
        .iter()
        .position(|f| f.id == fil_modele.id)
        .ok_or("fil classé absent")?;
    if boite_vs[..idx_haute]
        .iter()
        .any(|f| f.priorite != MsgPriorite::Haute)
    {
        return Err("un fil normale ne doit pas précéder une haute priorité".into());
    }
    r.geste("boîte staff: filtre dossier vs · haute priorité avant normale");

    let parent_classer = ent
        .classer_fil_for_actor(
            Role::Parent.as_str(),
            ClasserFil {
                etablissement_id: cast.etablissement_id,
                fil_id: fil_modele.id,
                priorite: MsgPriorite::Normale,
                dossier: None,
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await;
    if parent_classer.is_ok() {
        return Err("parent ne doit pas classer un fil".into());
    }
    r.geste("parent refusé au classement (attendu)");

    r.trou(
        "messagerie: modèles + archive + priorité/dossier + PJ natives + push OK — live si ENT_PUSH_LIVE=1",
    );
    Ok(())
}

async fn explorer_facturation(
    ent: &PgEnt,
    cast: &LaboCast,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# facturation foyer (compta → famille)");

    let (brouillon, lignes) = ent
        .creer_facture_for_actor(
            Role::Compta.as_str(),
            CreerFacture {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                numero: format!("LAB-{}", &cast.etablissement_id.to_string()[..8]),
                libelle: "Cantine octobre".into(),
                nature: None,
                annee_scolaire: None,
                lignes: vec![
                    LigneFactureCmd {
                        eleve_id: Some(cast.eleve_lea_id),
                        libelle: "Cantine".into(),
                        quantite: 20,
                        montant_unitaire_centimes: 450,
                    },
                    LigneFactureCmd {
                        eleve_id: None,
                        libelle: "Bourse".into(),
                        quantite: 1,
                        montant_unitaire_centimes: -2000,
                    },
                ],
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Slughorn crée brouillon {} · {} ligne(s) · {} c",
        brouillon.numero,
        lignes.len(),
        brouillon.total_centimes
    ));

    // Parent ne voit pas le brouillon
    let avant = ent
        .list_factures_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if avant.iter().any(|f| f.id == brouillon.id) {
        return Err("Lily ne doit pas voir un brouillon".into());
    }
    r.geste("Lily ne voit pas le brouillon (attendu)");

    let emise = ent
        .emettre_facture_for_actor(
            cast.etablissement_id,
            brouillon.id,
            cast.compta.user_id,
            Role::Compta.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if emise.statut != FactureStatut::Emise {
        return Err("facture doit être émise".into());
    }
    r.geste(format!("Slughorn émet {} → statut emise", emise.numero));

    let lily = ent
        .list_factures_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if !lily.iter().any(|f| f.id == emise.id) {
        return Err("Lily doit voir la facture émise".into());
    }
    let (detail, detail_lignes, _, encaisse0, _, _, _) = ent
        .get_facture_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            emise.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Lily lit facture {} · {} ligne(s) · {} c (encaisse={})",
        detail.numero,
        detail_lignes.len(),
        detail.total_centimes,
        encaisse0
    ));

    // Échéancier 3 parts avant encaissements
    let jour_ech = today_utc();
    let d1 = jour_ech + Duration::days(7);
    let d2 = jour_ech + Duration::days(37);
    let d3 = jour_ech + Duration::days(67);
    let echs = ent
        .poser_echeancier_parts_for_actor(
            cast.etablissement_id,
            emise.id,
            Role::Compta.as_str(),
            vec![d1, d2, d3],
        )
        .await
        .map_err(|e| e.to_string())?;
    if echs.len() != 3 {
        return Err("échéancier 3 parts attendu".into());
    }
    let sum_ech: i64 = echs.iter().map(|e| e.montant_centimes).sum();
    let sum_bourse: i64 = echs.iter().map(|e| e.bourse_centimes).sum();
    if sum_ech != emise.total_centimes {
        return Err(format!(
            "somme échéances {} ≠ total facture {}",
            sum_ech, emise.total_centimes
        ));
    }
    // Facture lab : Cantine 9000 − Bourse 2000 → net 7000 ; bourse ventilée = 2000
    if sum_bourse != 2000 {
        return Err(format!(
            "bourse ventilée attendue 2000 c, obtenu {}",
            sum_bourse
        ));
    }
    if echs.iter().any(|e| e.bourse_centimes <= 0) {
        return Err("chaque échéance doit porter une part de bourse".into());
    }
    let brut: i64 = echs.iter().map(|e| e.montant_brut_centimes()).sum();
    if brut != 9000 {
        return Err(format!("brut échéances {} ≠ 9000 (cantine avant bourse)", brut));
    }
    r.geste(format!(
        "Slughorn pose échéancier 3× · dû={} c · bourse ventilée={} c · brut={}",
        sum_ech, sum_bourse, brut
    ));

    // Isolation Narcissa
    let narcissa = ent
        .get_facture_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            emise.id,
        )
        .await;
    if narcissa.is_ok() {
        return Err("Narcissa ne doit pas lire la facture Potter".into());
    }
    r.geste("Narcissa bloquée sur facture Potter (attendu)");

    // Prof hors facturation
    let prof = ent
        .creer_facture_for_actor(
            Role::Prof.as_str(),
            CreerFacture {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_tom_id,
                numero: "PROF-NO".into(),
                libelle: "Interdit".into(),
                nature: None,
                annee_scolaire: None,
                lignes: vec![LigneFactureCmd {
                    eleve_id: None,
                    libelle: "X".into(),
                    quantite: 1,
                    montant_unitaire_centimes: 100,
                }],
                actor_user_id: cast.prof.user_id,
            },
        )
        .await;
    if prof.is_ok() {
        return Err("prof ne doit pas créer de facture".into());
    }
    r.geste("Rogue refusé à la création de facture (attendu)");

    // Encaissement partiel puis solde
    let partiel = emise.total_centimes / 2;
    let (e1, f1, q1) = ent
        .encaisser_facture_for_actor(
            Role::Compta.as_str(),
            EncaisserFacture {
                etablissement_id: cast.etablissement_id,
                facture_id: emise.id,
                montant_centimes: partiel,
                moyen: MoyenPaiement::Cheque,
                reference: Some("CHQ-LAB".into()),
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if f1.statut != FactureStatut::Emise {
        return Err("après acompte la facture reste emise".into());
    }
    r.geste(format!(
        "Slughorn encaisse acompte {} c ({}) → quittance {}",
        e1.montant_centimes,
        e1.moyen.as_str(),
        q1.numero
    ));

    let reste = emise.total_centimes - partiel;
    let (_e2, f2, q2) = ent
        .encaisser_facture_for_actor(
            Role::Compta.as_str(),
            EncaisserFacture {
                etablissement_id: cast.etablissement_id,
                facture_id: emise.id,
                montant_centimes: reste,
                moyen: MoyenPaiement::Virement,
                reference: None,
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if f2.statut != FactureStatut::Soldee {
        return Err("facture doit être soldée après solde".into());
    }
    r.geste(format!(
        "Slughorn solde {} → statut soldee · quittance {}",
        f2.numero, q2.numero
    ));

    let (detail2, _, encs, encaisse, ech_vues, quits, _) = ent
        .get_facture_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            emise.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if detail2.statut != FactureStatut::Soldee || encs.len() != 2 || encaisse != emise.total_centimes
    {
        return Err("Lily doit voir facture soldée + 2 paiements".into());
    }
    if ech_vues.len() != 3
        || !ech_vues
            .iter()
            .all(|v| v.couverture == EcheanceCouverture::Payee)
    {
        return Err("Lily doit voir 3 échéances payées".into());
    }
    if quits.len() != 2 {
        return Err("Lily doit voir 2 quittances".into());
    }
    r.geste(format!(
        "Lily voit facture soldée · {} paiement(s) · {} échéance(s) · {} quittance(s)",
        encs.len(),
        ech_vues.len(),
        quits.len()
    ));

    let quittances_famille = ent
        .list_quittances_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if !quittances_famille.iter().any(|q| q.id == q1.id || q.id == q2.id) {
        return Err("boîte quittances famille incomplète".into());
    }
    r.geste(format!(
        "Lily liste {} quittance(s) foyer",
        quittances_famille.len()
    ));

    // Parent ne peut pas encaisser
    let parent_enc = ent
        .encaisser_facture_for_actor(
            Role::Parent.as_str(),
            EncaisserFacture {
                etablissement_id: cast.etablissement_id,
                facture_id: emise.id,
                montant_centimes: 1,
                moyen: MoyenPaiement::Especes,
                reference: None,
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await;
    if parent_enc.is_ok() {
        return Err("parent ne doit pas encaisser".into());
    }
    r.geste("Lily refusée à l'encaissement (attendu)");

    let (fac_pdf, lignes_pdf, _, enc_pdf, _, _, _) = ent
        .get_facture_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            emise.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    let pdf_fac = domain_eleve::render_facture_pdf(&fac_pdf, &lignes_pdf, enc_pdf);
    if !pdf_fac.starts_with(b"%PDF-1.4") || !pdf_fac.windows(5).any(|w| w == b"%%EOF") {
        return Err("PDF facture invalide".into());
    }
    r.geste(format!(
        "PDF facture {} généré ({} octets)",
        fac_pdf.numero,
        pdf_fac.len()
    ));
    let q_pdf = domain_eleve::render_quittance_pdf(&q1, &fac_pdf.numero);
    if !q_pdf.starts_with(b"%PDF-1.4") {
        return Err("PDF quittance invalide".into());
    }
    r.geste(format!(
        "PDF quittance {} généré ({} octets)",
        q1.numero,
        q_pdf.len()
    ));

    let jour = domain_eleve::today_utc();
    let csv = ent
        .export_ogec_for_actor(
            Role::Compta.as_str(),
            domain_eleve::ExporterOgec {
                etablissement_id: cast.etablissement_id,
                debut: jour,
                fin: jour,
                actor_user_id: Some(cast.compta.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let csv_txt = String::from_utf8_lossy(&csv);
    if !csv_txt.contains("date;piece") || !csv_txt.contains(&emise.numero) {
        return Err(format!(
            "export OGEC incomplet (attendu n° {}) : {}",
            emise.numero,
            &csv_txt.chars().take(200).collect::<String>()
        ));
    }
    r.geste(format!(
        "export OGEC CSV {} octets · pièce {}",
        csv.len(),
        emise.numero
    ));
    let parent_ogec = ent
        .export_ogec_for_actor(
            Role::Parent.as_str(),
            domain_eleve::ExporterOgec {
                etablissement_id: cast.etablissement_id,
                debut: jour,
                fin: jour,
                actor_user_id: Some(cast.parent_lea.user_id.to_string()),
            },
        )
        .await;
    if parent_ogec.is_ok() {
        return Err("parent ne doit pas exporter OGEC".into());
    }
    r.geste("Lily refusée à l'export OGEC (attendu)");

    // SEPA : nouvelle facture dédiée (la précédente est soldée)
    let (brouillon_sepa, _) = ent
        .creer_facture_for_actor(
            Role::Compta.as_str(),
            CreerFacture {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                numero: format!("SEPA-{}", &cast.etablissement_id.to_string()[..8]),
                libelle: "Voyage pédagogique".into(),
                nature: None,
                annee_scolaire: None,
                lignes: vec![LigneFactureCmd {
                    eleve_id: Some(cast.eleve_lea_id),
                    libelle: "Voyage".into(),
                    quantite: 1,
                    montant_unitaire_centimes: 8000,
                }],
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let emise_sepa = ent
        .emettre_facture_for_actor(
            cast.etablissement_id,
            brouillon_sepa.id,
            cast.compta.user_id,
            Role::Compta.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let rum = format!("RUM-{}", &cast.etablissement_id.to_string()[..8]);
    let mandat = ent
        .creer_mandat_sepa_for_actor(
            Role::Compta.as_str(),
            cast.compta.user_id,
            domain_eleve::CreerMandatSepa {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                rum: rum.clone(),
                iban: "FR76 3000 6000 0112 3456 7890 189".into(),
                bic: Some("AGRIFRPP".into()),
                titulaire: "Lily Potter".into(),
                signe_at: chrono::Utc::now(),
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Slughorn crée mandat SEPA {} · IBAN {}",
        mandat.rum, mandat.iban
    ));
    let prev = ent
        .planifier_prelevement_for_actor(
            Role::Compta.as_str(),
            domain_eleve::PlanifierPrelevement {
                etablissement_id: cast.etablissement_id,
                facture_id: emise_sepa.id,
                mandat_id: mandat.id,
                montant_centimes: 8000,
                date_execution: jour,
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "prélèvement planifié {} c · e2e {}",
        prev.montant_centimes, prev.end_to_end_id
    ));
    let (sepa_csv, kind_csv) = ent
        .export_sepa_for_actor(
            cast.etablissement_id,
            Role::Compta.as_str(),
            jour,
            jour,
            cast.compta.user_id,
            "csv",
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if kind_csv != "csv" {
        return Err("export SEPA attendu en CSV".into());
    }
    let sepa_txt = String::from_utf8_lossy(&sepa_csv);
    if !sepa_txt.contains("date_execution;rum") || !sepa_txt.contains(&rum) {
        return Err(format!("export SEPA incomplet : {sepa_txt}"));
    }
    r.geste(format!("export SEPA CSV {} octets · RUM {rum}", sepa_csv.len()));
    let (executed, _, fac_sepa, quit_sepa) = ent
        .executer_prelevement_for_actor(
            cast.etablissement_id,
            prev.id,
            Role::Compta.as_str(),
            cast.compta.user_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if executed.statut != domain_eleve::PrelevementStatut::Execute {
        return Err("prélèvement doit être exécuté".into());
    }
    if fac_sepa.statut != FactureStatut::Soldee {
        return Err("facture SEPA doit être soldée après exécution".into());
    }
    r.geste(format!(
        "exécution SEPA → quittance {} · facture {}",
        quit_sepa.numero, fac_sepa.statut.as_str()
    ));

    // 2ᵉ facture : export pain.008 (RCUR) + rejet banque
    let (brouillon2, _) = ent
        .creer_facture_for_actor(
            Role::Compta.as_str(),
            CreerFacture {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                numero: format!("SEPA2-{}", &cast.etablissement_id.to_string()[..8]),
                libelle: "Assurance scolaire".into(),
                nature: None,
                annee_scolaire: None,
                lignes: vec![LigneFactureCmd {
                    eleve_id: Some(cast.eleve_lea_id),
                    libelle: "Assurance".into(),
                    quantite: 1,
                    montant_unitaire_centimes: 2500,
                }],
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let emise2 = ent
        .emettre_facture_for_actor(
            cast.etablissement_id,
            brouillon2.id,
            cast.compta.user_id,
            Role::Compta.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let prev2 = ent
        .planifier_prelevement_for_actor(
            Role::Compta.as_str(),
            domain_eleve::PlanifierPrelevement {
                etablissement_id: cast.etablissement_id,
                facture_id: emise2.id,
                mandat_id: mandat.id,
                montant_centimes: 2500,
                date_execution: jour,
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let cdtr = domain_eleve::plan_sepa_crediteur(
        "College Hogwarts",
        "FR76 1010 7001 0112 3456 7890 129",
        "CCBPFRPPXXX",
        "FR72ZZZHOGWARTS",
    )
    .map_err(|e| e.to_string())?;
    let (pain_xml, kind_pain) = ent
        .export_sepa_for_actor(
            cast.etablissement_id,
            Role::Compta.as_str(),
            jour,
            jour,
            cast.compta.user_id,
            "pain008",
            Some(cdtr),
        )
        .await
        .map_err(|e| e.to_string())?;
    if kind_pain != "pain008" {
        return Err("export attendu en pain.008".into());
    }
    let pain_txt = String::from_utf8_lossy(&pain_xml);
    if !pain_txt.contains("pain.008.001.02")
        || !pain_txt.contains("<SeqTp>RCUR</SeqTp>")
        || !pain_txt.contains(&prev2.end_to_end_id)
    {
        return Err(format!("pain.008 invalide : {}", &pain_txt.chars().take(280).collect::<String>()));
    }
    r.geste(format!(
        "export pain.008 {} octets · SeqTp=RCUR · e2e {}",
        pain_xml.len(),
        prev2.end_to_end_id
    ));
    let rejected = ent
        .rejeter_prelevement_for_actor(
            cast.etablissement_id,
            prev2.id,
            Role::Compta.as_str(),
            "AC04",
            cast.compta.user_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if rejected.statut != domain_eleve::PrelevementStatut::Rejete
        || rejected.motif_rejet.as_deref() != Some("AC04")
    {
        return Err("rejet SEPA attendu (AC04)".into());
    }
    let (_, _, _, encaisse_apres_rejet, _, _, _) = ent
        .get_facture_for_actor(
            cast.etablissement_id,
            cast.compta.user_id,
            Role::Compta.as_str(),
            emise2.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if encaisse_apres_rejet != 0 {
        return Err("rejet ne doit pas encaisser".into());
    }
    r.geste("rejet banque AC04 → réserve libérée, facture encore due");

    // Connecteur banque dry-run : relevé CSV → exécution + rejet auto
    let (br_bank_ok, _) = ent
        .creer_facture_for_actor(
            Role::Compta.as_str(),
            CreerFacture {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                numero: format!("BANK-OK-{}", &cast.etablissement_id.to_string()[..8]),
                libelle: "Stage linguistique".into(),
                nature: None,
                annee_scolaire: None,
                lignes: vec![LigneFactureCmd {
                    eleve_id: Some(cast.eleve_lea_id),
                    libelle: "Stage".into(),
                    quantite: 1,
                    montant_unitaire_centimes: 4500,
                }],
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let fac_bank_ok = ent
        .emettre_facture_for_actor(
            cast.etablissement_id,
            br_bank_ok.id,
            cast.compta.user_id,
            Role::Compta.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let prev_bank_ok = ent
        .planifier_prelevement_for_actor(
            Role::Compta.as_str(),
            domain_eleve::PlanifierPrelevement {
                etablissement_id: cast.etablissement_id,
                facture_id: fac_bank_ok.id,
                mandat_id: mandat.id,
                montant_centimes: 4500,
                date_execution: jour,
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let (br_bank_ko, _) = ent
        .creer_facture_for_actor(
            Role::Compta.as_str(),
            CreerFacture {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                numero: format!("BANK-KO-{}", &cast.etablissement_id.to_string()[..8]),
                libelle: "Cotisation AS".into(),
                nature: None,
                annee_scolaire: None,
                lignes: vec![LigneFactureCmd {
                    eleve_id: Some(cast.eleve_lea_id),
                    libelle: "AS".into(),
                    quantite: 1,
                    montant_unitaire_centimes: 1200,
                }],
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let fac_bank_ko = ent
        .emettre_facture_for_actor(
            cast.etablissement_id,
            br_bank_ko.id,
            cast.compta.user_id,
            Role::Compta.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let prev_bank_ko = ent
        .planifier_prelevement_for_actor(
            Role::Compta.as_str(),
            domain_eleve::PlanifierPrelevement {
                etablissement_id: cast.etablissement_id,
                facture_id: fac_bank_ko.id,
                mandat_id: mandat.id,
                montant_centimes: 1200,
                date_execution: jour,
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let _ = ent
        .export_sepa_for_actor(
            cast.etablissement_id,
            Role::Compta.as_str(),
            jour,
            jour,
            cast.compta.user_id,
            "csv",
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    let csv_bank = format!(
        "end_to_end_id;montant_centimes;result;motif\n\
{};4500;settled;\n\
{};1200;rejected;AC01\n\
UNKNOWN-E2E;999;settled;\n",
        prev_bank_ok.end_to_end_id, prev_bank_ko.end_to_end_id
    );
    let bank_sum = ent
        .rapprocher_releve_banque_for_actor(
            cast.etablissement_id,
            Role::Compta.as_str(),
            cast.compta.user_id,
            &csv_bank,
        )
        .await
        .map_err(|e| e.to_string())?;
    if bank_sum.executed != 1 || bank_sum.rejected != 1 || bank_sum.unmatched != 1 {
        return Err(format!(
            "rapprochement banque attendu 1/1/1, got exec={} rej={} unmatched={}",
            bank_sum.executed, bank_sum.rejected, bank_sum.unmatched
        ));
    }
    let n_bank = ent
        .count_bank_rapprochements(cast.etablissement_id)
        .await
        .map_err(|e| e.to_string())?;
    if n_bank < 3 {
        return Err(format!("journal banque attendu ≥3 lignes, got {n_bank}"));
    }
    let applied = ent
        .count_bank_rapprochements_status(cast.etablissement_id, "applied")
        .await
        .map_err(|e| e.to_string())?;
    if applied < 2 {
        return Err(format!("rapprochements applied attendus ≥2, got {applied}"));
    }
    r.geste(format!(
        "rapprochement banque dry-run → exec={} rejet={} unmatched={} (journal={n_bank}, applied={applied})",
        bank_sum.executed, bank_sum.rejected, bank_sum.unmatched
    ));

    // Doublon actif : Lily ne peut pas créer un 2ᵉ mandat tant que le premier est actif.
    let parent_mandat = ent
        .creer_mandat_sepa_for_actor(
            Role::Parent.as_str(),
            cast.parent_lea.user_id,
            domain_eleve::CreerMandatSepa {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                rum: "RUM-PARENT".into(),
                iban: "FR7630006000011234567890189".into(),
                bic: None,
                titulaire: "Lily".into(),
                signe_at: chrono::Utc::now(),
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await;
    if parent_mandat.is_ok() {
        return Err("parent ne doit pas créer un 2e mandat SEPA actif".into());
    }
    r.geste("Lily refusée (mandat foyer déjà actif — attendu)");

    // Rappels auto J-3 / en retard
    let (br_rap, _) = ent
        .creer_facture_for_actor(
            Role::Compta.as_str(),
            CreerFacture {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                numero: format!("RAP-{}", &cast.etablissement_id.to_string()[..8]),
                libelle: "Périscolaire".into(),
                nature: None,
                annee_scolaire: None,
                lignes: vec![LigneFactureCmd {
                    eleve_id: Some(cast.eleve_lea_id),
                    libelle: "Périscolaire".into(),
                    quantite: 1,
                    montant_unitaire_centimes: 6000,
                }],
                actor_user_id: cast.compta.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let emise_rap = ent
        .emettre_facture_for_actor(
            cast.etablissement_id,
            br_rap.id,
            cast.compta.user_id,
            Role::Compta.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let d_retard = jour - chrono::Duration::days(5);
    let d_j3 = jour + chrono::Duration::days(3);
    let echs = ent
        .poser_echeancier_for_actor(
            Role::Compta.as_str(),
            domain_eleve::PoserEcheancier {
                etablissement_id: cast.etablissement_id,
                facture_id: emise_rap.id,
                echeances: vec![
                    domain_eleve::EcheanceCmd {
                        libelle: "Part retard".into(),
                        montant_centimes: 3000,
                        date_echeance: d_retard,
                    },
                    domain_eleve::EcheanceCmd {
                        libelle: "Part J-3".into(),
                        montant_centimes: 3000,
                        date_echeance: d_j3,
                    },
                ],
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if echs.len() != 2 {
        return Err("échéancier rappel 2 parts attendu".into());
    }
    let rappels = ent
        .lancer_rappels_echeances_for_actor(
            cast.etablissement_id,
            Role::Compta.as_str(),
            cast.compta.user_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if rappels.len() < 2 {
        return Err(format!(
            "attendu ≥2 rappels (retard+J-3), got {}",
            rappels.len()
        ));
    }
    if !rappels.iter().any(|r| r.kind == domain_eleve::RappelKind::EnRetard) {
        return Err("rappel en_retard manquant".into());
    }
    if !rappels.iter().any(|r| r.kind == domain_eleve::RappelKind::JMoins3) {
        return Err("rappel j_moins_3 manquant".into());
    }
    if !rappels.iter().any(|r| r.fil_id.is_some() && r.canal == domain_eleve::RappelCanal::Message)
    {
        return Err("rappel doit ouvrir un fil famille".into());
    }
    r.geste(format!(
        "rappels auto : {} créé(s) (retard + J-3) · fil message OK",
        rappels.len()
    ));
    let again = ent
        .lancer_rappels_echeances_for_actor(
            cast.etablissement_id,
            Role::Compta.as_str(),
            cast.compta.user_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !again.is_empty() {
        return Err("2ᵉ lancement doit être idempotent (0 nouveau)".into());
    }
    r.geste("rappels idempotents (relance = 0 nouveau)");

    r.trou("facturation: PDF + OGEC + SEPA + rappels + bourse + rapprochement CSV dry-run OK — UI /app/facturation + types/affectation/décision bourses HTTP + live si ENT_BANK_LIVE=1 (PSD2)");
    Ok(())
}

async fn explorer_motifs_etab(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# motifs établissement");

    let motif = ent
        .create_motif_absence_etab(
            cast.etablissement_id,
            "stage_pro",
            "Stage professionnel",
            Some(cast.administratif.user_id.to_string()),
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini crée motif etab {} ({})",
        motif.code, motif.libelle
    ));

    let libre_refuse = ent
        .declare_absence(DeclareAbsence {
            etablissement_id: cast.etablissement_id,
            eleve_id: cast.eleve_lea_id,
            debut: jour,
            fin: jour,
            motif: "texte_libre_interdit".into(),
            actor_user_id: Some(cast.cpe.user_id.to_string()),
        })
        .await;
    if libre_refuse.is_ok() {
        return Err("motif libre hors référentiel aurait dû être refusé".into());
    }
    r.geste("motif libre hors catalogue → refusé (attendu)");

    let cible = &cast.eleves[2]; // Nina
    ent.declare_absence(DeclareAbsence {
        etablissement_id: cast.etablissement_id,
        eleve_id: cible.id,
        debut: jour + Duration::days(1),
        fin: jour + Duration::days(1),
        motif: "stage_pro".into(),
        actor_user_id: Some(cast.cpe.user_id.to_string()),
    })
    .await
    .map_err(|e| e.to_string())?;
    r.geste(format!(
        "CPE déclare {} avec motif etab stage_pro",
        cible.prenom
    ));

    let catalogue = ent
        .list_motifs_absence(cast.etablissement_id)
        .await
        .map_err(|e| e.to_string())?;
    if !catalogue.iter().any(|m| m.code == "stage_pro") {
        return Err("motif etab absent du catalogue".into());
    }
    r.geste(format!("catalogue motifs: {} entrées", catalogue.len()));
    r.trou("motifs absence: catalogue global + création étab OK — UI /app/etab + déclaration /app/absences + /famille + natives");
    Ok(())
}

async fn explorer_face_eleve(
    ent: &PgEnt,
    cast: &LaboCast,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# face élève");

    let avec = cast
        .eleves
        .iter()
        .find(|e| e.compte.is_some())
        .expect("au moins un compte élève");
    let compte = avec.compte.as_ref().unwrap();
    let voit_soi = ent
        .actor_can_read_notes(
            cast.etablissement_id,
            compte.user_id,
            Role::Eleve.as_str(),
            avec.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !voit_soi {
        return Err(format!("{} devrait lire ses notes", avec.prenom));
    }
    r.geste(format!(
        "{} (compte élève) lit ses notes — OK",
        avec.prenom
    ));

    let periodes_notes = ent
        .list_note_periodes(cast.etablissement_id)
        .await
        .map_err(|e| e.to_string())?;
    let periode_devoirs = periodes_notes
        .iter()
        .find(|p| p.statut == NotePeriodeStatut::Ouverte)
        .or_else(|| periodes_notes.first())
        .ok_or_else(|| "période notes requise pour devoirs élève".to_string())?;
    let devoirs_soi = ent
        .list_devoirs_eleve_for_actor(
            cast.etablissement_id,
            compte.user_id,
            Role::Eleve.as_str(),
            avec.id,
            periode_devoirs.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "{} lit {} devoir(s) classe {} — OK",
        avec.prenom,
        devoirs_soi.len(),
        avec.classe
    ));

    let voit_abs = ent
        .actor_can_read_absences(
            cast.etablissement_id,
            compte.user_id,
            Role::Eleve.as_str(),
            avec.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !voit_abs {
        return Err(format!("{} devrait lire ses absences", avec.prenom));
    }
    let absences_soi = ent
        .list_absences_eleve(cast.etablissement_id, avec.id)
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "{} lit {} absence(s) — OK",
        avec.prenom,
        absences_soi.len()
    ));

    let journal_soi = ent
        .list_journal_eleve_for_actor(
            cast.etablissement_id,
            compte.user_id,
            Role::Eleve.as_str(),
            avec.id,
            30,
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "{} lit journal ({} entrée(s) visibles famille)",
        avec.prenom,
        journal_soi.len()
    ));

    let autre = cast
        .eleves
        .iter()
        .find(|e| e.id != avec.id)
        .expect("autre");
    let voit_autre = ent
        .actor_can_read_notes(
            cast.etablissement_id,
            compte.user_id,
            Role::Eleve.as_str(),
            autre.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if voit_autre {
        return Err("élève ne doit pas lire les notes d'un camarade".into());
    }
    r.geste(format!(
        "{} ne lit pas les notes de {} — OK",
        avec.prenom, autre.prenom
    ));
    let devoirs_autre = ent
        .list_devoirs_eleve_for_actor(
            cast.etablissement_id,
            compte.user_id,
            Role::Eleve.as_str(),
            autre.id,
            periode_devoirs.id,
        )
        .await;
    if devoirs_autre.is_ok() {
        return Err("élève ne doit pas lister les devoirs d'un camarade".into());
    }
    r.geste(format!(
        "{} isolé sur devoirs de {} (attendu)",
        avec.prenom, autre.prenom
    ));
    let voit_abs_autre = ent
        .actor_can_read_absences(
            cast.etablissement_id,
            compte.user_id,
            Role::Eleve.as_str(),
            autre.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if voit_abs_autre {
        return Err("élève ne doit pas lire les absences d'un camarade".into());
    }
    r.geste(format!(
        "{} ne lit pas les absences de {} — OK",
        avec.prenom, autre.prenom
    ));
    let journal_autre = ent
        .list_journal_eleve_for_actor(
            cast.etablissement_id,
            compte.user_id,
            Role::Eleve.as_str(),
            autre.id,
            10,
        )
        .await;
    if journal_autre.is_ok() {
        return Err("élève ne doit pas lire le journal d'un camarade".into());
    }
    r.geste(format!(
        "{} isolé sur journal de {} (attendu)",
        avec.prenom, autre.prenom
    ));

    let sans_avant = ent
        .list_eleves_sans_compte_for_actor(cast.etablissement_id, Role::Administratif.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if sans_avant.is_empty() {
        return Err("lab attend des élèves sans compte avant invitation".into());
    }
    let cible = sans_avant[0].clone();
    let (inv, email, mdp) = ent
        .inviter_compte_eleve_for_actor(
            Role::Administratif.as_str(),
            InviterCompteEleve {
                etablissement_id: cast.etablissement_id,
                eleve_id: cible.id,
                email: None,
                actor_user_id: cast.administratif.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini invite {} {} → {} (mdp temporaire {}…)",
        cible.prenom,
        cible.nom,
        email,
        &mdp[..6.min(mdp.len())]
    ));

    // Le nouvel élève lit ses notes
    let user_id = inv.activated_user_id.ok_or("invitation sans user activé")?;
    let voit = ent
        .actor_can_read_notes(
            cast.etablissement_id,
            user_id,
            Role::Eleve.as_str(),
            cible.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !voit {
        return Err("élève provisionné devrait lire ses notes".into());
    }
    r.geste(format!("{} provisionné lit ses notes — OK", cible.prenom));

    // Doublon refusé
    let dup = ent
        .inviter_compte_eleve_for_actor(
            Role::Administratif.as_str(),
            InviterCompteEleve {
                etablissement_id: cast.etablissement_id,
                eleve_id: cible.id,
                email: None,
                actor_user_id: cast.administratif.user_id,
            },
        )
        .await;
    if dup.is_ok() {
        return Err("re-invitation d'un compte déjà lié doit échouer".into());
    }

    // Lot : reste des sans-compte
    let lot = ent
        .provisionner_comptes_manquants_for_actor(
            cast.etablissement_id,
            cast.administratif.user_id,
            Role::Administratif.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "lot provisionnement : {} compte(s) créé(s)",
        lot.len()
    ));
    let reste = ent
        .list_eleves_sans_compte_for_actor(cast.etablissement_id, Role::Administratif.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if !reste.is_empty() {
        return Err(format!("encore {} élève(s) sans compte après lot", reste.len()));
    }
    r.geste("plus aucun élève actif sans compte");

    // Prof / parent hors invitation
    let prof_refuse = ent
        .list_eleves_sans_compte_for_actor(cast.etablissement_id, Role::Prof.as_str())
        .await;
    if prof_refuse.is_ok() {
        return Err("prof ne doit pas lister les sans-compte".into());
    }
    r.geste("prof refusé sur liste sans-compte (attendu)");

    // Cahier de texte : Rogue publie pour 4B → Harry (élève) + Lily (parent) voient ; Draco non.
    let harry = cast
        .eleves
        .iter()
        .find(|e| e.id == cast.eleve_lea_id)
        .expect("Harry");
    let harry_compte = harry.compte.as_ref().expect("Harry a un compte face");
    let pour_le = today_utc() + Duration::days(2);
    let cdt = ent
        .creer_cahier_texte_for_actor(
            Role::Prof.as_str(),
            CreerCahierTexte {
                etablissement_id: cast.etablissement_id,
                classe: "4B".into(),
                matiere_libelle: "Potions".into(),
                titre: "Exercices ch.3".into(),
                consigne: "Pages 12 à 14 — à rendre jeudi.".into(),
                pour_le,
                cree_le: today_utc(),
                actor_user_id: cast.prof.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Rogue publie cahier « {} » pour 4B (pour le {pour_le})",
        cdt.titre
    ));

    let vue_harry = ent
        .list_cahier_texte_for_actor(
            cast.etablissement_id,
            harry_compte.user_id,
            Role::Eleve.as_str(),
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if vue_harry.iter().all(|e| e.id != cdt.id) {
        return Err("Harry (élève) doit voir le cahier 4B".into());
    }
    r.geste("Harry lit le cahier de texte de sa classe");

    let vue_lily = ent
        .list_cahier_texte_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            None,
            Some(cast.eleve_lea_id),
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if vue_lily.iter().all(|e| e.id != cdt.id) {
        return Err("Lily doit voir le cahier de Harry".into());
    }
    r.geste("Lily lit le cahier via le portail famille");

    let narc = ent
        .list_cahier_texte_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            None,
            Some(cast.eleve_lea_id),
            None,
            None,
        )
        .await;
    if narc.is_ok() {
        return Err("Narcissa ne doit pas lire le cahier Harry".into());
    }
    let draco = cast
        .eleves
        .iter()
        .find(|e| e.id == cast.eleve_tom_id)
        .expect("Draco");
    if let Some(ref compte_d) = draco.compte {
        let vue_d = ent
            .list_cahier_texte_for_actor(
                cast.etablissement_id,
                compte_d.user_id,
                Role::Eleve.as_str(),
                None,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        if vue_d.iter().any(|e| e.id == cdt.id) {
            return Err("Draco (5A) ne doit pas voir le cahier 4B".into());
        }
        r.geste("Draco (autre classe) ne voit pas le cahier 4B");
    } else {
        r.geste("Draco sans compte — isolation classe vérifiée via Narcissa");
    }

    let parent_ecrit = ent
        .creer_cahier_texte_for_actor(
            Role::Parent.as_str(),
            CreerCahierTexte {
                etablissement_id: cast.etablissement_id,
                classe: "4B".into(),
                matiere_libelle: "Intrus".into(),
                titre: "Non".into(),
                consigne: "interdit".into(),
                pour_le,
                cree_le: today_utc(),
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await;
    if parent_ecrit.is_ok() {
        return Err("parent ne doit pas publier au cahier".into());
    }
    r.geste("parent refusé à la publication cahier (attendu)");

    // Messagerie face élève : Harry ouvre / répond ; Draco isolé.
    let (fil_eleve, _) = ent
        .ouvrir_fil_for_actor(
            Role::Eleve.as_str(),
            OuvrirFil {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                sujet: "Question cours".into(),
                premier_message: "Bonjour, je n'ai pas compris l'exercice 2.".into(),
                actor_user_id: harry_compte.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Harry (élève) ouvre fil « {} » vers l'établissement",
        fil_eleve.sujet
    ));

    let reponse_cpe = ent
        .repondre_fil_for_actor(
            Role::Cpe.as_str(),
            RepondreFil {
                etablissement_id: cast.etablissement_id,
                fil_id: fil_eleve.id,
                corps: "On revoit ça demain en permanence.".into(),
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("CPE répond → « {} »", reponse_cpe.corps));

    let fils_harry = ent
        .list_fils_for_actor(
            cast.etablissement_id,
            harry_compte.user_id,
            Role::Eleve.as_str(),
            None,
            false,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if fils_harry.iter().all(|f| f.id != fil_eleve.id) {
        return Err("Harry doit voir son fil élève".into());
    }
    r.geste(format!(
        "Harry voit {} fil(s) dans sa boîte élève",
        fils_harry.len()
    ));

    let draco = cast
        .eleves
        .iter()
        .find(|e| e.id == cast.eleve_tom_id)
        .expect("Draco");
    if let Some(ref compte_d) = draco.compte {
        let voir_harry = ent
            .list_messages_for_actor(
                cast.etablissement_id,
                compte_d.user_id,
                Role::Eleve.as_str(),
                fil_eleve.id,
            )
            .await;
        if voir_harry.is_ok() {
            return Err("Draco ne doit pas lire le fil Harry".into());
        }
        let ouvrir_autre = ent
            .ouvrir_fil_for_actor(
                Role::Eleve.as_str(),
                OuvrirFil {
                    etablissement_id: cast.etablissement_id,
                    eleve_id: harry.id,
                    sujet: "Intrus".into(),
                    premier_message: "non".into(),
                    actor_user_id: compte_d.user_id,
                },
            )
            .await;
        if ouvrir_autre.is_ok() {
            return Err("Draco ne doit pas ouvrir un fil sur Harry".into());
        }
        r.geste("Draco isolé sur messagerie Harry (attendu)");
    }

    r.trou(
        "face élève: invitation + lot + cahier + messagerie+PJ + absences + journal + devoirs classe OK — SPA /eleve (+routes Axum) + invitations + apps natives ios-eleve/android-eleve (notes/devoirs/cahier/EDT/absences/journal/messages+PJ)",
    );
    Ok(())
}

async fn explorer_sanctions_vs(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# incidents + sanctions (VS)");

    let types = ent
        .list_sanction_types(cast.etablissement_id)
        .await
        .map_err(|e| e.to_string())?;
    if types.len() < 2 {
        return Err("référentiel sanctions (retenue/avertissement) manquant".into());
    }
    let retenue = types
        .iter()
        .find(|t| t.code == "retenue")
        .ok_or_else(|| "type retenue".to_string())?;
    r.geste(format!("référentiel sanctions: {} type(s)", types.len()));

    // Types incidents via list — get from seed by declaring with first incident type.
    // On récupère via une déclaration prof → CPE sanctionne.
    use domain_eleve::CreerReferentielVs;
    let types_inc = ent
        .list_incident_types(cast.etablissement_id)
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "référentiel incidents listé: {} type(s)",
        types_inc.len()
    ));
    let inc_type = if let Some(t) = types_inc.iter().find(|t| t.code == "perturbation") {
        t.clone()
    } else {
        ent.creer_incident_type_for_actor(
            Role::Cpe.as_str(),
            CreerReferentielVs {
                etablissement_id: cast.etablissement_id,
                code: "perturbation".into(),
                libelle: "Perturbation de cours".into(),
                actor_user_id: Some(cast.cpe.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?
    };
    let suffix = &cast.etablissement_id.simple().to_string()[..6];
    let type_lab = ent
        .creer_incident_type_for_actor(
            Role::Cpe.as_str(),
            CreerReferentielVs {
                etablissement_id: cast.etablissement_id,
                code: format!("lab_inc_{suffix}"),
                libelle: "Incident lab CPE".into(),
                actor_user_id: Some(cast.cpe.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("CPE crée type incident {}", type_lab.code));
    let san_lab = ent
        .creer_sanction_type_for_actor(
            Role::Cpe.as_str(),
            CreerReferentielVs {
                etablissement_id: cast.etablissement_id,
                code: format!("lab_san_{suffix}"),
                libelle: "Sanction lab CPE".into(),
                actor_user_id: Some(cast.cpe.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("CPE crée type sanction {}", san_lab.code));
    let motif_lab = ent
        .creer_sanction_motif_for_actor(
            Role::Cpe.as_str(),
            CreerReferentielVs {
                etablissement_id: cast.etablissement_id,
                code: format!("lab_mot_{suffix}"),
                libelle: "Motif lab CPE".into(),
                actor_user_id: Some(cast.cpe.user_id.to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("CPE crée motif sanction {}", motif_lab.code));
    let refuse_prof_ref = ent
        .creer_incident_type_for_actor(
            Role::Prof.as_str(),
            CreerReferentielVs {
                etablissement_id: cast.etablissement_id,
                code: "interdit".into(),
                libelle: "interdit".into(),
                actor_user_id: Some(cast.prof.user_id.to_string()),
            },
        )
        .await;
    if refuse_prof_ref.is_ok() {
        return Err("prof ne doit pas créer de type incident".into());
    }
    r.geste("prof refusé sur type incident (attendu)");

    let incident = ent
        .declarer_incident_for_actor(
            Role::Prof.as_str(),
            DeclarerIncident {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                type_id: inc_type.id,
                jour,
                description: Some("bavardages répétés".into()),
                actor_user_id: cast.prof.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("prof déclare incident sur Léa");

    let prof_sanction = ent
        .prononcer_sanction_for_actor(
            Role::Prof.as_str(),
            PrononcerSanction {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                type_id: retenue.id,
                motif_id: None,
                incident_id: Some(incident.id),
                jour,
                duree_minutes: Some(60),
                note: None,
                actor_user_id: cast.prof.user_id,
            },
        )
        .await;
    if prof_sanction.is_ok() {
        return Err("prof ne doit pas prononcer de sanction".into());
    }
    r.geste("prof refusé sur prononcé sanction (attendu)");

    let sanction = ent
        .prononcer_sanction_for_actor(
            Role::Cpe.as_str(),
            PrononcerSanction {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                type_id: retenue.id,
                motif_id: None,
                incident_id: Some(incident.id),
                jour,
                duree_minutes: Some(60),
                note: Some("retenue 1h".into()),
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if sanction.statut != SanctionStatut::Active {
        return Err("sanction active attendue".into());
    }
    r.geste(format!(
        "CPE prononce retenue {} min liée à l'incident",
        sanction.duree_minutes.unwrap_or(0)
    ));

    let sms_sanction = ent
        .count_sms_kind(cast.etablissement_id, "sanction")
        .await
        .map_err(|e| e.to_string())?;
    if sms_sanction < 1 {
        return Err("SMS sanction attendu en outbox (parent avec téléphone)".into());
    }
    if ent.sms().count_with_body_contains("retenue") < 1
        && ent.sms().count_with_body_contains("Retenue") < 1
        && ent.sms().count_with_body_contains("sanction") < 1
    {
        return Err("sms mémoire doit tracer le corps sanction".into());
    }
    r.geste(format!(
        "SMS sanction dry-run → {sms_sanction} SMS outbox"
    ));

    let d_parent = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "parent dossier sanctions".to_string())?;
    if !d_parent.sections.vs || d_parent.sanctions.is_empty() {
        return Err("parent Léa (flag VS) doit voir la sanction sur la fiche".into());
    }
    r.geste("parent Léa voit sanction sur fiche (consultation dossier)");

    let d_tom = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_tom_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "parent Tom dossier".to_string())?;
    if d_tom.sections.vs {
        return Err("parent Tom: VS masquée — pas de sanctions".into());
    }
    if !d_tom.sanctions.is_empty() {
        return Err("parent Tom ne doit pas recevoir de sanctions dans la vue".into());
    }
    r.geste("parent Tom: VS/sanctions masquées (flag)");

    let cancelled = ent
        .annuler_sanction_for_actor(
            Role::Cpe.as_str(),
            AnnulerSanction {
                etablissement_id: cast.etablissement_id,
                sanction_id: sanction.id,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if cancelled.statut != SanctionStatut::Annulee {
        return Err("sanction annulée attendue".into());
    }
    r.geste("CPE annule la sanction");

    let commission = ent
        .creer_commission_for_actor(
            Role::Cpe.as_str(),
            CreerCommissionVs {
                etablissement_id: cast.etablissement_id,
                jour: jour + Duration::days(7),
                heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                objet: "Incidents répétés Harry".into(),
                lieu: "Bureau CPE".into(),
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "CPE planifie commission VS le {} 14:00",
        commission.jour
    ));

    let dossier = ent
        .convoquer_commission_for_actor(
            Role::Cpe.as_str(),
            ConvoquerCommission {
                etablissement_id: cast.etablissement_id,
                commission_id: commission.id,
                eleve_id: cast.eleve_lea_id,
                motifs: "Perturbations cumulées en cours".into(),
                note_interne: "préparer dossier confidentiel".into(),
                sanction_id: None,
                incident_id: Some(incident.id),
                signale_famille: true,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("CPE convoque Harry en commission (signalé famille)");

    let lily_com = ent
        .list_commissions_eleve_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    let vu = lily_com
        .iter()
        .find(|d| d.id == dossier.id)
        .ok_or("Lily doit voir la convocation")?;
    if !vu.note_interne.is_empty() {
        return Err("Lily ne doit pas voir la note interne commission".into());
    }
    r.geste("Lily lit convocation commission sans note interne");

    let decided = ent
        .decider_commission_dossier_for_actor(
            Role::Cpe.as_str(),
            DeciderCommissionDossier {
                etablissement_id: cast.etablissement_id,
                dossier_id: dossier.id,
                decision: DecisionCommission::Avertissement,
                decision_texte: "Avertissement écrit".into(),
                points_retires: 5,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if decided.points_retires != 5 {
        return Err("5 points retirés attendus".into());
    }
    let solde = ent
        .solde_points_vs_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if solde != 15 {
        return Err(format!("solde points attendu 15, obtenu {solde}").into());
    }
    let lily_solde = ent
        .solde_points_vs_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if lily_solde != 15 {
        return Err("Lily doit voir le solde points 15".into());
    }
    r.geste("commission décide avertissement · permis à points 20→15");

    let re_dec = ent
        .decider_commission_dossier_for_actor(
            Role::Cpe.as_str(),
            DeciderCommissionDossier {
                etablissement_id: cast.etablissement_id,
                dossier_id: dossier.id,
                decision: DecisionCommission::Maintien,
                decision_texte: String::new(),
                points_retires: 0,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await;
    if re_dec.is_ok() {
        return Err("re-décision commission refusée".into());
    }
    let prof_com = ent
        .creer_commission_for_actor(
            Role::Prof.as_str(),
            CreerCommissionVs {
                etablissement_id: cast.etablissement_id,
                jour: jour + Duration::days(8),
                heure: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                objet: "interdit".into(),
                lieu: String::new(),
                actor_user_id: cast.prof.user_id,
            },
        )
        .await;
    if prof_com.is_ok() {
        return Err("prof ne doit pas créer une commission".into());
    }
    r.geste("re-décision refusée · prof bloqué sur commission (attendu)");

    r.trou(
        "sanctions: commission + permis à points + SMS auto OK — HTTP GET/POST incidents/sanctions + POST référentiels /vs/*-types|motifs + UI /app/absences + lecture famille web/natives — live si ENT_SMS_LIVE=1",
    );
    Ok(())
}

async fn explorer_carnet(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# carnet de liaison");

    let harry = &cast.eleves[0];
    let dossier = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            harry.id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "dossier CPE Harry".to_string())?;
    if dossier.carnet.is_empty() {
        return Err("seed: entrée carnet Harry attendue".into());
    }
    r.geste(format!(
        "CPE lit carnet Harry — {} entrée(s)",
        dossier.carnet.len()
    ));

    let entree = ent
        .creer_carnet_entree_for_actor(
            Role::Prof.as_str(),
            CreerCarnetEntree {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                date_entree: jour,
                categorie: CarnetCategorie::Accompagnement,
                titre: "Félicitation".into(),
                corps: "Bonne participation en potions.".into(),
                visible_famille: true,
                actor_user_id: cast.prof.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let Some(fil_carnet) = entree.fil_notif_id else {
        return Err("entrée visible famille doit ouvrir un fil notif".into());
    };
    r.geste("Rogue écrit une entrée carnet visible famille → notif messagerie");

    let fils_lily = ent
        .list_fils_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            Some(harry.id),
            false,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if fils_lily.iter().all(|f| f.id != fil_carnet) {
        return Err("Lily doit recevoir le fil notif carnet".into());
    }
    if !fils_lily
        .iter()
        .any(|f| f.id == fil_carnet && f.sujet.contains("Carnet"))
    {
        return Err("sujet fil notif carnet attendu".into());
    }
    r.geste("Lily reçoit la notif carnet dans sa messagerie");

    let mails_carnet = ent
        .count_mails_kind(cast.etablissement_id, "carnet")
        .await
        .map_err(|e| e.to_string())?;
    if mails_carnet < 1 {
        return Err("SMTP carnet attendu en outbox (dry-run)".into());
    }
    if ent.mailer().count_with_subject_contains("Carnet") < 1 {
        return Err("mailer mémoire doit tracer le sujet carnet".into());
    }
    r.geste(format!(
        "SMTP carnet dry-run → {mails_carnet} mail(s) outbox"
    ));

    let sms_carnet = ent
        .count_sms_kind(cast.etablissement_id, "carnet")
        .await
        .map_err(|e| e.to_string())?;
    if sms_carnet < 1 {
        return Err("SMS carnet attendu en outbox".into());
    }
    r.geste(format!("SMS carnet dry-run → {sms_carnet} SMS outbox"));

    let push_carnet = ent
        .count_push_kind(cast.etablissement_id, "carnet")
        .await
        .map_err(|e| e.to_string())?;
    if push_carnet < 1 {
        return Err("push carnet attendu sur device Lily".into());
    }
    r.geste(format!("push carnet dry-run → {push_carnet} notif(s) outbox"));

    let interne = ent
        .creer_carnet_entree_for_actor(
            Role::Cpe.as_str(),
            CreerCarnetEntree {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                date_entree: jour,
                categorie: CarnetCategorie::Information,
                titre: "Note interne CPE".into(),
                corps: "À surveiller en récré — ne pas montrer aux parents.".into(),
                visible_famille: false,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if interne.fil_notif_id.is_some() {
        return Err("entrée interne ne doit pas notifier la famille".into());
    }
    r.geste("CPE pose une note carnet interne (invisible famille, sans notif)");

    let vue_parent = ent
        .get_dossier_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            harry.id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "parent Lily doit voir le dossier".to_string())?;
    if vue_parent.carnet.iter().any(|c| c.id == interne.id) {
        return Err("parent ne doit pas voir l'entrée carnet interne".into());
    }
    if !vue_parent.carnet.iter().any(|c| c.id == entree.id) {
        return Err("parent doit voir l'entrée visible famille".into());
    }
    r.geste("Lily voit les entrées visibles, pas la note interne");

    let signee = ent
        .signer_carnet_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            entree.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if signee.signe_at.is_none() {
        return Err("signature parent attendue".into());
    }
    r.geste("Lily signe l'entrée carnet");

    let refuse_tom = ent
        .signer_carnet_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            entree.id,
        )
        .await;
    if refuse_tom.is_ok() {
        return Err("Narcissa ne doit pas signer le carnet de Harry".into());
    }
    r.geste("Narcissa bloquée sur carnet Harry (attendu)");

    let refuse_compta = ent
        .creer_carnet_entree_for_actor(
            Role::Compta.as_str(),
            CreerCarnetEntree {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                date_entree: jour,
                categorie: CarnetCategorie::Information,
                titre: "Nope".into(),
                corps: "Compta n'écrit pas au carnet.".into(),
                visible_famille: true,
                actor_user_id: cast.compta.user_id,
            },
        )
        .await;
    if refuse_compta.is_ok() {
        return Err("compta ne doit pas écrire au carnet".into());
    }
    r.geste("compta refusée à l'écriture carnet");

    r.trou("carnet: notif messagerie + SMTP + SMS OK — UI saisie dossier + live si ENT_MAIL_LIVE / ENT_SMS_LIVE");
    Ok(())
}

async fn explorer_sorties(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# autorisations de sortie (permanent + exception + badge)");

    let harry = &cast.eleves[0];
    let draco = &cast.eleves[1];
    let lily = cast.parent_lea.user_id;
    let narcissa = cast.parent_tom.user_id;
    let cpe = cast.cpe.user_id;
    let rogue = cast.prof.user_id;

    let auth = ent
        .upsert_autorisation_sortie_for_actor(
            Role::Parent.as_str(),
            UpsertAutorisationSortie {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                matin: false,
                midi: true,
                soir: true,
                interdit_etablissement: false,
                actor_user_id: lily,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !auth.midi || !auth.soir || auth.matin {
        return Err(format!(
            "autorisation Harry attendue midi+soir, got matin={} midi={} soir={}",
            auth.matin, auth.midi, auth.soir
        ));
    }
    r.geste("Lily pose consentement sortie Harry — midi+soir");

    let forbid_parent = ent
        .upsert_autorisation_sortie_for_actor(
            Role::Parent.as_str(),
            UpsertAutorisationSortie {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                matin: true,
                midi: true,
                soir: true,
                interdit_etablissement: true,
                actor_user_id: lily,
            },
        )
        .await;
    if forbid_parent.is_ok() {
        return Err("parent ne doit pas poser forbid établissement".into());
    }
    r.geste("Lily refusée sur forbid établissement (attendu)");

    let narcissa_harry = ent
        .upsert_autorisation_sortie_for_actor(
            Role::Parent.as_str(),
            UpsertAutorisationSortie {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                matin: true,
                midi: true,
                soir: true,
                interdit_etablissement: false,
                actor_user_id: narcissa,
            },
        )
        .await;
    if narcissa_harry.is_ok() {
        return Err("Narcissa ne doit pas autoriser Harry".into());
    }
    r.geste("Narcissa bloquée sur autorisation Harry (attendu)");

    let dem = ent
        .demander_sortie_exceptionnelle_for_actor(
            Role::Parent.as_str(),
            DemanderSortieExceptionnelle {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                jour,
                creneau: SortieCreneau::Matin,
                motif: "RDV orthodontiste".into(),
                actor_user_id: lily,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Lily demande sortie exceptionnelle matin Harry ({})",
        dem.statut.as_str()
    ));

    let file = ent
        .list_demandes_sortie_en_attente_for_actor(cast.etablissement_id, Role::Cpe.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if !file.iter().any(|d| d.id == dem.id) {
        return Err("demande Harry absente de la file CPE".into());
    }
    r.geste(format!("file VS sorties: {} en attente", file.len()));

    let prof_dec = ent
        .decider_demande_sortie_for_actor(
            Role::Prof.as_str(),
            DeciderDemandeSortie {
                etablissement_id: cast.etablissement_id,
                demande_id: dem.id,
                accepter: true,
                decision_note: "non".into(),
                actor_user_id: rogue,
            },
        )
        .await;
    if prof_dec.is_ok() {
        return Err("prof ne doit pas trancher une demande de sortie".into());
    }
    r.geste("prof refusé sur décision sortie (attendu)");

    let decided = ent
        .decider_demande_sortie_for_actor(
            Role::Cpe.as_str(),
            DeciderDemandeSortie {
                etablissement_id: cast.etablissement_id,
                demande_id: dem.id,
                accepter: true,
                decision_note: "ok dentiste".into(),
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if decided.statut.as_str() != "acceptee" {
        return Err(format!("attendu acceptee, got {}", decided.statut.as_str()));
    }
    r.geste("CPE accepte sortie exceptionnelle matin Harry");

    let ctrl_matin = ent
        .controle_sortie_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            harry.id,
            jour,
            SortieCreneau::Matin,
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if ctrl_matin.verdict != ControleSortieVerdict::Autorisee {
        return Err(format!(
            "badge matin Harry attendu autorisé, got {} ({})",
            ctrl_matin.verdict.as_str(),
            ctrl_matin.motif
        ));
    }
    r.geste(format!(
        "badge portail Harry matin → {} ({})",
        ctrl_matin.verdict.as_str(),
        ctrl_matin.source.as_str()
    ));

    let ctrl_midi = ent
        .controle_sortie_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            harry.id,
            jour,
            SortieCreneau::Midi,
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if ctrl_midi.verdict != ControleSortieVerdict::Autorisee {
        return Err(format!(
            "badge midi Harry attendu autorisé (permanent), got {}",
            ctrl_midi.verdict.as_str()
        ));
    }
    r.geste("badge portail Harry midi → autorisée (permanent)");

    let ctrl_draco = ent
        .controle_sortie_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            draco.id,
            jour,
            SortieCreneau::Soir,
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if ctrl_draco.verdict != ControleSortieVerdict::Refusee {
        return Err("Draco sans autorisation doit être refusé au badge".into());
    }
    r.geste("badge Draco soir → refusée (aucune autorisation)");

    let regle = ent
        .upsert_sortie_regle_niveau_for_actor(
            Role::Cpe.as_str(),
            UpsertSortieRegleNiveau {
                etablissement_id: cast.etablissement_id,
                niveau: "4e".into(),
                matin_autorise: false,
                midi_autorise: false,
                soir_autorise: true,
                choix_parent_interdit: false,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if regle.midi_autorise || !regle.soir_autorise {
        return Err("règle 4e attendue soir seul".into());
    }
    r.geste("CPE configure règle niveau 4e — soir seul (pas midi/matin)");

    let clamped = ent
        .upsert_autorisation_sortie_for_actor(
            Role::Parent.as_str(),
            UpsertAutorisationSortie {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                matin: true,
                midi: true,
                soir: true,
                interdit_etablissement: false,
                actor_user_id: lily,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if clamped.matin || clamped.midi || !clamped.soir {
        return Err(format!(
            "règle 4e doit clamper Lily à soir seul, got matin={} midi={} soir={}",
            clamped.matin, clamped.midi, clamped.soir
        ));
    }
    r.geste("Lily re-consent midi+soir → clamp règle 4e → soir seul");

    let regle_dp = ent
        .upsert_sortie_regle_regime_for_actor(
            Role::Cpe.as_str(),
            UpsertSortieRegleRegime {
                etablissement_id: cast.etablissement_id,
                regime: "DP".into(),
                matin_autorise: false,
                midi_autorise: false,
                soir_autorise: false,
                choix_parent_interdit: true,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !regle_dp.choix_parent_interdit {
        return Err("règle DP doit interdire le choix parent".into());
    }
    r.geste("CPE configure règle régime DP — choix parent interdit");

    let refuse_dp = ent
        .upsert_autorisation_sortie_for_actor(
            Role::Parent.as_str(),
            UpsertAutorisationSortie {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                matin: false,
                midi: false,
                soir: true,
                interdit_etablissement: false,
                actor_user_id: lily,
            },
        )
        .await;
    if refuse_dp.is_ok() {
        return Err("Lily (Harry DP) ne doit pas pouvoir consentir sous règle DP interdit".into());
    }
    r.geste("Lily refusée sous règle DP (Harry demi-pensionnaire, attendu)");

    // Relâche pour laisser le forbid établissement se jouer ensuite.
    let _ = ent
        .upsert_sortie_regle_regime_for_actor(
            Role::Cpe.as_str(),
            UpsertSortieRegleRegime {
                etablissement_id: cast.etablissement_id,
                regime: "DP".into(),
                matin_autorise: false,
                midi_autorise: false,
                soir_autorise: true,
                choix_parent_interdit: false,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("CPE assouplit règle DP → soir autorisé (suite forbid)");

    let forbid = ent
        .upsert_autorisation_sortie_for_actor(
            Role::Cpe.as_str(),
            UpsertAutorisationSortie {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                matin: true,
                midi: true,
                soir: true,
                interdit_etablissement: true,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !forbid.interdit_etablissement {
        return Err("forbid CPE non posé".into());
    }
    r.geste("CPE pose interdiction totale sortie Harry");

    let parent_apres_forbid = ent
        .upsert_autorisation_sortie_for_actor(
            Role::Parent.as_str(),
            UpsertAutorisationSortie {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                matin: true,
                midi: true,
                soir: true,
                interdit_etablissement: false,
                actor_user_id: lily,
            },
        )
        .await;
    if parent_apres_forbid.is_ok() {
        return Err("parent ne doit pas lever un forbid établissement".into());
    }
    r.geste("Lily refusée après forbid établissement (attendu)");

    let ctrl_forbid = ent
        .controle_sortie_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            harry.id,
            jour,
            SortieCreneau::Midi,
            None,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    if ctrl_forbid.verdict != ControleSortieVerdict::Refusee {
        return Err("badge après forbid doit refuser".into());
    }
    r.geste("badge Harry midi après forbid → refusée");

    let recap = ent
        .get_recap_sortie_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if recap.autorisation.as_ref().map(|a| a.interdit_etablissement) != Some(true) {
        return Err("récap famille doit montrer forbid".into());
    }
    r.geste("Lily lit récap sorties Harry (interdit établissement)");

    // Changement de site EDT (param + créneau SiteB) — lever le forbid pour tester.
    let _ = ent
        .upsert_autorisation_sortie_for_actor(
            Role::Cpe.as_str(),
            UpsertAutorisationSortie {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                matin: false,
                midi: false,
                soir: false,
                interdit_etablissement: false,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let param = ent
        .upsert_sortie_param_for_actor(
            Role::Cpe.as_str(),
            UpsertSortieParamEtab {
                etablissement_id: cast.etablissement_id,
                autoriser_changement_site: true,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !param.autoriser_changement_site {
        return Err("param changement de site non activé".into());
    }
    r.geste("CPE active param sortie « changement de site EDT »");
    ent.creer_edt_creneau(EdtCreneau {
        id: Uuid::new_v4(),
        etablissement_id: cast.etablissement_id,
        classe: "4B".into(),
        matiere_id: None,
        matiere_libelle: "EPS annexe".into(),
        prof_user_id: rogue,
        jour_semaine: jour_semaine_iso(jour),
        heure_debut: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
        heure_fin: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
        salle: Some("Gymnase".into()),
        site: "SiteB".into(),
        semaine_ab: EdtSemaineAb::Toutes,
    })
    .await
    .map_err(|e| e.to_string())?;
    r.geste("créneau EDT 4B 15h SiteB (EPS annexe)");
    let ctrl_site = ent
        .controle_sortie_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            harry.id,
            jour,
            SortieCreneau::Midi,
            Some("SiteA".into()),
            Some(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
        )
        .await
        .map_err(|e| e.to_string())?;
    if ctrl_site.verdict != ControleSortieVerdict::Autorisee
        || ctrl_site.source.as_str() != "autre_site"
    {
        return Err(format!(
            "badge SiteA→SiteB attendu autre_site, got {} ({})",
            ctrl_site.verdict.as_str(),
            ctrl_site.source.as_str()
        ));
    }
    r.geste(format!(
        "badge portail SiteA 12h → {} ({})",
        ctrl_site.verdict.as_str(),
        ctrl_site.source.as_str()
    ));

    r.trou(
        "sorties: permanent + exception + forbid + badge + niveau + régime + autre site EDT OK — UI /app/sorties",
    );
    Ok(())
}

async fn explorer_internat(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# internat — chambres / affectation / présence / suivi");

    let harry = &cast.eleves[0];
    let cpe = cast.cpe.user_id;
    let lily = cast.parent_lea.user_id;
    let rogue = cast.prof.user_id;

    // Harry devient interne pour la slice.
    ent.change_regime(ChangeRegime {
        etablissement_id: cast.etablissement_id,
        eleve_id: harry.id,
        nouveau_regime: "INT".into(),
        date_effet: jour,
        actor_user_id: Some(cpe.to_string()),
    })
    .await
    .map_err(|e| e.to_string())?;
    r.geste("CPE passe Harry en régime INT");

    let chambre = ent
        .creer_chambre_internat_for_actor(
            Role::Cpe.as_str(),
            CreerChambreInternat {
                etablissement_id: cast.etablissement_id,
                batiment: "Pavillon Nord".into(),
                etage: 2,
                code: "N201".into(),
                capacite: 2,
                genre: "garcons".into(),
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "CPE crée chambre {} ({}/{})",
        chambre.code, chambre.batiment, chambre.capacite
    ));

    let refuse_prof = ent
        .creer_chambre_internat_for_actor(
            Role::Prof.as_str(),
            CreerChambreInternat {
                etablissement_id: cast.etablissement_id,
                batiment: "X".into(),
                etage: 0,
                code: "X1".into(),
                capacite: 1,
                genre: "mixte".into(),
                actor_user_id: rogue,
            },
        )
        .await;
    if refuse_prof.is_ok() {
        return Err("prof ne doit pas gérer l'internat".into());
    }
    r.geste("prof refusé sur création chambre (attendu)");

    let refuse_ext = ent
        .affecter_eleve_internat_for_actor(
            Role::Cpe.as_str(),
            AffecterEleveInternat {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleves[1].id,
                chambre_id: chambre.id,
                lit: None,
                debut: jour,
                actor_user_id: cpe,
            },
        )
        .await;
    if refuse_ext.is_ok() {
        return Err("Draco non-INT ne doit pas être affecté".into());
    }
    r.geste("affectation Draco (non-INT) refusée (attendu)");

    let aff = ent
        .affecter_eleve_internat_for_actor(
            Role::Cpe.as_str(),
            AffecterEleveInternat {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                chambre_id: chambre.id,
                lit: Some("1".into()),
                debut: jour,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "CPE affecte Harry → chambre {} lit {}",
        chambre.code,
        aff.lit.as_deref().unwrap_or("?")
    ));

    let doublon = ent
        .affecter_eleve_internat_for_actor(
            Role::Cpe.as_str(),
            AffecterEleveInternat {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                chambre_id: chambre.id,
                lit: Some("2".into()),
                debut: jour,
                actor_user_id: cpe,
            },
        )
        .await;
    if doublon.is_ok() {
        return Err("double affectation Harry doit être refusée".into());
    }
    r.geste("double affectation Harry refusée (attendu)");

    let presence = ent
        .pointer_presence_internat_for_actor(
            Role::Cpe.as_str(),
            PointerPresenceInternat {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                jour,
                statut: "present".into(),
                note: String::new(),
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if presence.statut != PresenceInternatStatut::Present {
        return Err("présence Harry attendue présent".into());
    }
    r.geste("appel internat soir — Harry présent");

    let suivi = ent
        .creer_suivi_internat_for_actor(
            Role::Cpe.as_str(),
            CreerSuiviInternat {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                jour,
                texte: "Bonne adaptation — lit bien rangé".into(),
                visible_famille: true,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "CPE note suivi internat visible famille ({})",
        &suivi.id.to_string()[..8]
    ));

    let _interne = ent
        .creer_suivi_internat_for_actor(
            Role::Cpe.as_str(),
            CreerSuiviInternat {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                jour,
                texte: "Note interne CPE — ne pas diffuser".into(),
                visible_famille: false,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    let recap_famille = ent
        .get_recap_internat_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if recap_famille.chambre.as_ref().map(|c| c.code.as_str()) != Some("N201") {
        return Err("Lily doit voir la chambre N201".into());
    }
    if recap_famille.presence_du_jour.as_ref().map(|p| p.statut)
        != Some(PresenceInternatStatut::Present)
    {
        return Err("Lily doit voir présence du jour".into());
    }
    if recap_famille.suivis_famille.len() != 1 {
        return Err(format!(
            "Lily ne doit voir que les suivis visibles (got {})",
            recap_famille.suivis_famille.len()
        ));
    }
    r.geste("Lily lit récap internat Harry (chambre + présence + 1 suivi)");

    let narcissa = cast.parent_tom.user_id;
    let refuse_parent = ent
        .get_recap_internat_for_actor(
            cast.etablissement_id,
            narcissa,
            Role::Parent.as_str(),
            harry.id,
            jour,
        )
        .await;
    if refuse_parent.is_ok() {
        return Err("Narcissa ne doit pas lire l'internat Harry".into());
    }
    r.geste("Narcissa bloquée sur internat Harry (attendu)");

    let closed = ent
        .clore_affectation_internat_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            aff.id,
            jour,
            cpe,
        )
        .await
        .map_err(|e| e.to_string())?;
    if closed.fin != Some(jour) {
        return Err("clôture affectation attendue".into());
    }
    r.geste("CPE clôture affectation Harry (fin = jour)");

    r.trou("internat: chambres + affectation INT + présence + suivi famille OK — UI /app/internat + natives");
    Ok(())
}

async fn explorer_dispenses_vs(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# dispenses VS (EPS / médical)");

    let harry = &cast.eleves[0];
    let lily = cast.parent_lea.user_id;
    let cpe = cast.cpe.user_id;
    let rogue = cast.prof.user_id;
    let inf = cast.infirmieres[0].user_id;

    let refuse_prof = ent
        .poser_dispense_vs_for_actor(
            Role::Prof.as_str(),
            PoserDispenseVs {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                motif: "eps".into(),
                debut: jour,
                fin: jour + Duration::days(14),
                commentaire: "non".into(),
                actor_user_id: rogue,
            },
        )
        .await;
    if refuse_prof.is_ok() {
        return Err("prof ne doit pas poser dispense VS".into());
    }
    r.geste("prof refusé sur pose dispense VS (attendu)");

    let d = ent
        .poser_dispense_vs_for_actor(
            Role::Parent.as_str(),
            PoserDispenseVs {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                motif: "eps".into(),
                debut: jour,
                fin: jour + Duration::days(10),
                commentaire: "entorse cheville".into(),
                actor_user_id: lily,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Lily pose dispense EPS Harry {} → {}",
        d.debut, d.fin
    ));

    let narcissa = cast.parent_tom.user_id;
    let refuse_nar = ent
        .poser_dispense_vs_for_actor(
            Role::Parent.as_str(),
            PoserDispenseVs {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                motif: "eps".into(),
                debut: jour,
                fin: jour,
                commentaire: "intrus".into(),
                actor_user_id: narcissa,
            },
        )
        .await;
    if refuse_nar.is_ok() {
        return Err("Narcissa ne doit pas poser dispense Harry".into());
    }
    r.geste("Narcissa bloquée sur dispense Harry (attendu)");

    let medical = ent
        .poser_dispense_vs_for_actor(
            Role::Infirmerie.as_str(),
            PoserDispenseVs {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                motif: "medical".into(),
                debut: jour,
                fin: jour + Duration::days(3),
                commentaire: "repos post-chute".into(),
                actor_user_id: inf,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("infirmière pose dispense médicale Harry (3j)");

    let couvre_eps = ent
        .dispense_vs_pour_appel(
            cast.etablissement_id,
            harry.id,
            jour,
            Some("EPS 4B"),
        )
        .await
        .map_err(|e| e.to_string())?;
    let Some(couvre) = couvre_eps else {
        return Err("appel EPS doit voir une dispense active".into());
    };
    if couvre.motif.as_str() != "eps" && couvre.motif.as_str() != "medical" {
        return Err(format!(
            "dispense appel EPS inattendue: {}",
            couvre.motif.as_str()
        ));
    }
    r.geste(format!(
        "appel EPS → dispense {} ({})",
        couvre.motif.as_str(),
        &couvre.id.to_string()[..8]
    ));

    let jour_file = ent
        .list_dispenses_vs_jour_for_actor(cast.etablissement_id, Role::Cpe.as_str(), jour)
        .await
        .map_err(|e| e.to_string())?;
    if jour_file.len() < 2 {
        return Err(format!(
            "file VS dispenses jour attend ≥2, got {}",
            jour_file.len()
        ));
    }
    r.geste(format!("file VS dispenses du jour: {}", jour_file.len()));

    let closed = ent
        .clore_dispense_vs_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            medical.id,
            cpe,
        )
        .await
        .map_err(|e| e.to_string())?;
    if closed.statut != DispenseVsStatut::Close {
        return Err("dispense médicale doit être close".into());
    }
    r.geste("CPE clôture dispense médicale");

    let actives = ent
        .list_dispenses_vs_eleve_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
            true,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !actives.iter().any(|x| x.id == d.id) || actives.iter().any(|x| x.id == medical.id) {
        return Err("Lily doit voir EPS active, pas la médicale close".into());
    }
    r.geste("Lily lit dispenses actives Harry (EPS seule)");

    let journal = ent
        .list_journal_eleve_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
            20,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !journal.iter().any(|e| e.action == "dispense_posee") {
        return Err("journal famille doit contenir dispense_posee".into());
    }
    if !journal.iter().any(|e| e.action == "dispense_cloturee") {
        return Err("journal famille doit contenir dispense_cloturee".into());
    }
    r.geste(format!(
        "Lily lit journal Harry — {} entrée(s) visibles",
        journal.len()
    ));

    let journal_staff = ent
        .list_journal_eleve_for_actor(
            cast.etablissement_id,
            cpe,
            Role::Cpe.as_str(),
            harry.id,
            20,
        )
        .await
        .map_err(|e| e.to_string())?;
    if journal_staff.len() < journal.len() {
        return Err("staff doit voir au moins autant d'entrées journal".into());
    }
    r.geste(format!("CPE lit journal Harry — {} entrée(s)", journal_staff.len()));

    r.trou(
        "dispenses VS + journal: EPS parent + médical + appel + clôture + journal famille OK — UI /famille + natives EPS + staff /app/dispenses",
    );
    Ok(())
}

async fn explorer_alertes_cpe(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# alertes CPE (file VS manuelle)");

    let harry = &cast.eleves[0];
    let lily = cast.parent_lea.user_id;
    let cpe = cast.cpe.user_id;
    let rogue = cast.prof.user_id;

    let refuse_prof = ent
        .creer_alerte_cpe_for_actor(
            Role::Prof.as_str(),
            CreerAlerteCpe {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                titre: "Intrus".into(),
                detail: String::new(),
                severite: "info".into(),
                visible_famille: false,
                jour,
                actor_user_id: rogue,
            },
        )
        .await;
    if refuse_prof.is_ok() {
        return Err("prof ne doit pas créer d'alerte CPE".into());
    }
    r.geste("prof refusé sur alerte CPE (attendu)");

    let interne = ent
        .creer_alerte_cpe_for_actor(
            Role::Cpe.as_str(),
            CreerAlerteCpe {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                titre: "Suivi comportement interne".into(),
                detail: "Note VS — pas pour la famille".into(),
                severite: "info".into(),
                visible_famille: false,
                jour,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("CPE pose alerte interne Harry (invisible famille)");

    let famille = ent
        .creer_alerte_cpe_for_actor(
            Role::Cpe.as_str(),
            CreerAlerteCpe {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                titre: "Retards répétés".into(),
                detail: "3e retard de la semaine — RDV demandé".into(),
                severite: "urgent".into(),
                visible_famille: true,
                jour,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("CPE pose alerte urgente visible famille");

    let file = ent
        .list_alertes_cpe_ouvertes_for_actor(cast.etablissement_id, Role::Cpe.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if file.len() < 2 {
        return Err(format!("file alertes attend ≥2, got {}", file.len()));
    }
    if file.first().map(|a| a.severite.as_str()) != Some("urgent") {
        return Err("file doit trier urgent en premier".into());
    }
    r.geste(format!("file VS alertes: {} ouverte(s), urgent en tête", file.len()));

    let vue_lily = ent
        .list_alertes_cpe_eleve_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if vue_lily.len() != 1 || vue_lily[0].id != famille.id {
        return Err("Lily ne doit voir que l'alerte visible famille".into());
    }
    r.geste("Lily lit 1 alerte CPE (retards) — pas l'interne");

    let closed = ent
        .clore_alerte_cpe_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            famille.id,
            cpe,
            "Entretien parents OK".into(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if closed.statut != AlerteCpeStatut::Close {
        return Err("alerte doit être close".into());
    }
    r.geste("CPE clôture alerte retards");

    let file_apres = ent
        .list_alertes_cpe_ouvertes_for_actor(cast.etablissement_id, Role::Cpe.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if file_apres.iter().any(|a| a.id == famille.id) {
        return Err("alerte close ne doit plus être dans la file".into());
    }
    if !file_apres.iter().any(|a| a.id == interne.id) {
        return Err("alerte interne doit rester ouverte".into());
    }
    r.geste("file VS: alerte urgente retirée, interne toujours ouverte");

    let _ = ent
        .clore_alerte_cpe_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            interne.id,
            cpe,
            "Classé".into(),
        )
        .await
        .map_err(|e| e.to_string())?;

    r.trou("alertes CPE: file triée + visible famille + clôture OK — UI /app/alertes + natives");
    Ok(())
}

async fn explorer_docs_a_signer(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# documents à signer (famille)");

    let harry = &cast.eleves[0];
    let lily = cast.parent_lea.user_id;
    let cpe = cast.cpe.user_id;
    let rogue = cast.prof.user_id;
    let annee = annee_scolaire_de(jour);

    let refuse_prof = ent
        .publier_doc_a_signer_for_actor(
            Role::Prof.as_str(),
            PublierDocASigner {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                code: "X".into(),
                titre: "non".into(),
                corps: "non".into(),
                annee_scolaire: annee.clone(),
                echeance: None,
                actor_user_id: rogue,
            },
        )
        .await;
    if refuse_prof.is_ok() {
        return Err("prof ne doit pas publier doc à signer".into());
    }
    r.geste("prof refusé sur publication doc à signer (attendu)");

    let doc = ent
        .publier_doc_a_signer_for_actor(
            Role::Cpe.as_str(),
            PublierDocASigner {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                code: "REGLEMENT".into(),
                titre: "Règlement intérieur 2026-2027".into(),
                corps: "Article 1 — Respect mutuel…".into(),
                annee_scolaire: annee,
                echeance: Some(jour + Duration::days(30)),
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("CPE publie « {} » pour Harry", doc.titre));

    let pending = ent
        .list_docs_a_signer_eleve_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
            true,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !pending.iter().any(|d| d.id == doc.id) {
        return Err("Lily doit voir le doc à signer".into());
    }
    r.geste(format!("Lily voit {} doc(s) à signer", pending.len()));

    let narcissa = cast.parent_tom.user_id;
    let refuse_sign = ent
        .signer_doc_a_signer_for_actor(
            Role::Parent.as_str(),
            SignerDocASigner {
                etablissement_id: cast.etablissement_id,
                doc_id: doc.id,
                actor_user_id: narcissa,
            },
        )
        .await;
    if refuse_sign.is_ok() {
        return Err("Narcissa ne doit pas signer pour Harry".into());
    }
    r.geste("Narcissa bloquée sur signature Harry (attendu)");

    let signed = ent
        .signer_doc_a_signer_for_actor(
            Role::Parent.as_str(),
            SignerDocASigner {
                etablissement_id: cast.etablissement_id,
                doc_id: doc.id,
                actor_user_id: lily,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if signed.statut != DocSignerStatut::Signe {
        return Err("doc doit être signé".into());
    }
    r.geste("Lily signe le règlement intérieur");

    let re_sign = ent
        .signer_doc_a_signer_for_actor(
            Role::Parent.as_str(),
            SignerDocASigner {
                etablissement_id: cast.etablissement_id,
                doc_id: doc.id,
                actor_user_id: lily,
            },
        )
        .await;
    if re_sign.is_ok() {
        return Err("re-signature doit être refusée".into());
    }
    r.geste("re-signature refusée (attendu)");

    let file = ent
        .list_docs_a_signer_etab_for_actor(cast.etablissement_id, Role::Cpe.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if file.iter().any(|d| d.id == doc.id) {
        return Err("doc signé ne doit plus être dans la file VS".into());
    }
    r.geste("file VS docs à signer : sans le règlement signé");

    r.trou("docs à signer: publication CPE + signature parent + file VS OK — UI /app/docs-signer + web /famille");
    Ok(())
}

async fn explorer_conseil_classe(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# conseil de classe + mentions");

    let harry = &cast.eleves[0];
    let lily = cast.parent_lea.user_id;
    let cpe = cast.cpe.user_id;
    let rogue = cast.prof.user_id;

    let conseil = ent
        .creer_conseil_classe_for_actor(
            Role::Cpe.as_str(),
            CreerConseilClasse {
                etablissement_id: cast.etablissement_id,
                classe: "4B".into(),
                periode_libelle: "T1".into(),
                jour: jour + Duration::days(14),
                heure: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
                lieu: "Salle des conseils".into(),
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "CPE planifie conseil 4B T1 le {} 17:00",
        conseil.jour
    ));

    let dossier = ent
        .ajouter_dossier_conseil_for_actor(
            Role::Prof.as_str(),
            AjouterDossierConseil {
                etablissement_id: cast.etablissement_id,
                conseil_id: conseil.id,
                eleve_id: harry.id,
                actor_user_id: rogue,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("Rogue ajoute Harry au conseil 4B");

    let decide = ent
        .decider_dossier_conseil_for_actor(
            Role::Cpe.as_str(),
            DeciderDossierConseil {
                etablissement_id: cast.etablissement_id,
                dossier_id: dossier.id,
                appreciation: "Trimestre solide — poursuite des efforts".into(),
                mention: "encouragements".into(),
                decision_texte: String::new(),
                visible_famille: true,
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if decide.mention.map(|m| m.as_str()) != Some("encouragements") {
        return Err("mention encouragements attendue".into());
    }
    r.geste("CPE décide → encouragements (visible famille)");

    let tenu = ent
        .tenir_conseil_classe_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            conseil.id,
            cpe,
        )
        .await
        .map_err(|e| e.to_string())?;
    if tenu.statut.as_str() != "tenu" {
        return Err("conseil doit passer tenu".into());
    }
    r.geste("CPE clôture le conseil (tenu)");

    let vue = ent
        .list_conseils_eleve_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !vue.iter().any(|d| d.id == dossier.id) {
        return Err("Lily doit voir la décision de conseil".into());
    }
    r.geste("Lily lit décision conseil (encouragements)");

    let narcissa = cast.parent_tom.user_id;
    let refuse = ent
        .list_conseils_eleve_for_actor(
            cast.etablissement_id,
            narcissa,
            Role::Parent.as_str(),
            harry.id,
        )
        .await;
    if refuse.is_ok() {
        return Err("Narcissa ne doit pas lire le conseil Harry".into());
    }
    r.geste("Narcissa bloquée sur conseil Harry (attendu)");

    r.trou("conseil de classe: planif + import effectif + mention + lecture famille OK — UI /app/conseils (session CPE) + natives");
    Ok(())
}

async fn explorer_info_comp(
    ent: &PgEnt,
    cast: &LaboCast,
    _jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# infos complémentaires paramétrables");

    let harry = &cast.eleves[0];
    let lily = cast.parent_lea.user_id;
    let admini = cast.administratifs[0].user_id;
    let rogue = cast.prof.user_id;

    let champ = ent
        .creer_info_comp_champ_for_actor(
            Role::Administratif.as_str(),
            CreerInfoCompChamp {
                etablissement_id: cast.etablissement_id,
                code: "LANGUE_MAISON".into(),
                libelle: "Langue parlée à la maison".into(),
                type_valeur: "texte".into(),
                obligatoire: false,
                visible_famille: true,
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("admini crée champ info-comp « {} »", champ.libelle));

    let interne = ent
        .creer_info_comp_champ_for_actor(
            Role::Administratif.as_str(),
            CreerInfoCompChamp {
                etablissement_id: cast.etablissement_id,
                code: "NOTE_INTERNE_IC".into(),
                libelle: "Note interne IC".into(),
                type_valeur: "texte".into(),
                obligatoire: false,
                visible_famille: false,
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    let refuse_prof = ent
        .creer_info_comp_champ_for_actor(
            Role::Prof.as_str(),
            CreerInfoCompChamp {
                etablissement_id: cast.etablissement_id,
                code: "X".into(),
                libelle: "X".into(),
                type_valeur: "texte".into(),
                obligatoire: false,
                visible_famille: false,
                actor_user_id: rogue,
            },
        )
        .await;
    if refuse_prof.is_ok() {
        return Err("prof ne doit pas créer de champ info-comp".into());
    }
    r.geste("prof refusé sur création champ IC (attendu)");

    let _ = ent
        .upsert_info_comp_valeur_for_actor(
            Role::Administratif.as_str(),
            UpsertInfoCompValeur {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                champ_id: champ.id,
                valeur: "Anglais".into(),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let _ = ent
        .upsert_info_comp_valeur_for_actor(
            Role::Administratif.as_str(),
            UpsertInfoCompValeur {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                champ_id: interne.id,
                valeur: "confidentiel".into(),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("admini renseigne langue maison + note interne Harry");

    let (champs_f, vals_f) = ent
        .list_info_comp_eleve_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if champs_f.len() != 1 || vals_f.len() != 1 || vals_f[0].valeur != "Anglais" {
        return Err(format!(
            "Lily doit voir 1 champ/valeur visible (got champs={} vals={:?})",
            champs_f.len(),
            vals_f.iter().map(|v| v.valeur.as_str()).collect::<Vec<_>>()
        ));
    }
    r.geste("Lily lit info-comp visible (Anglais) — pas la note interne");

    r.trou("info-comp: champs paramétrables + valeurs + filtre famille OK — UI /app/etab + natives");
    Ok(())
}

async fn explorer_vacances_homonymes_menus(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# vacances / homonymes / menus cantine");

    let admini = cast.administratifs[0].user_id;
    let lily = cast.parent_lea.user_id;
    let rogue = cast.prof.user_id;
    let harry = &cast.eleves[0];

    let toussaint_debut = NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();
    let toussaint_fin = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
    let periode = ent
        .creer_periode_vacances_for_actor(
            Role::Administratif.as_str(),
            CreerPeriodeVacances {
                etablissement_id: cast.etablissement_id,
                libelle: "Toussaint".into(),
                kind: "vacances".into(),
                debut: toussaint_debut,
                fin: toussaint_fin,
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini pose vacances « {} » {} → {}",
        periode.libelle, periode.debut, periode.fin
    ));

    let refuse_prof = ent
        .creer_periode_vacances_for_actor(
            Role::Prof.as_str(),
            CreerPeriodeVacances {
                etablissement_id: cast.etablissement_id,
                libelle: "X".into(),
                kind: "ferie".into(),
                debut: jour,
                fin: jour,
                actor_user_id: rogue,
            },
        )
        .await;
    if refuse_prof.is_ok() {
        return Err("prof ne doit pas créer de période vacances".into());
    }
    r.geste("prof refusé sur création vacances (attendu)");

    let mid = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
    let ferme = ent
        .jour_ferme_for_actor(cast.etablissement_id, Role::Parent.as_str(), mid)
        .await
        .map_err(|e| e.to_string())?;
    if !ferme {
        return Err("20/10/2026 doit être fermé (Toussaint)".into());
    }
    let ouvert = ent
        .jour_ferme_for_actor(cast.etablissement_id, Role::Parent.as_str(), jour)
        .await
        .map_err(|e| e.to_string())?;
    if ouvert && !(jour >= toussaint_debut && jour <= toussaint_fin) {
        return Err(format!("{jour} ne doit pas être fermé hors Toussaint"));
    }
    r.geste("Lily voit jour fermé pendant Toussaint");

    let homos = ent
        .rechercher_homonymes_for_actor(
            cast.etablissement_id,
            Role::Administratif.as_str(),
            RechercherHomonymes {
                etablissement_id: cast.etablissement_id,
                nom: harry.nom.clone(),
                prenom: harry.prenom.clone(),
                date_naissance: None,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !homos.iter().any(|h| h.eleve_id == harry.id && h.score >= 80) {
        return Err(format!(
            "homonyme Harry attendu score≥80, got {:?}",
            homos
                .iter()
                .map(|h| (h.prenom.clone(), h.nom.clone(), h.score))
                .collect::<Vec<_>>()
        ));
    }
    r.geste(format!(
        "admini détecte homonyme {} {} (score {})",
        harry.prenom, harry.nom, homos[0].score
    ));

    let refuse_parent = ent
        .rechercher_homonymes_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            RechercherHomonymes {
                etablissement_id: cast.etablissement_id,
                nom: "X".into(),
                prenom: "Y".into(),
                date_naissance: None,
            },
        )
        .await;
    if refuse_parent.is_ok() {
        return Err("parent ne doit pas rechercher d'homonymes".into());
    }
    r.geste("Lily refusée sur recherche homonymes (attendu)");

    let menu = ent
        .upsert_menu_cantine_for_actor(
            Role::Administratif.as_str(),
            UpsertMenuCantine {
                etablissement_id: cast.etablissement_id,
                jour,
                entree: "Salade verte".into(),
                plat: "Poulet rôti".into(),
                dessert: "Yaourt".into(),
                notes: "Sans porc".into(),
                publie: true,
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let _ = ent
        .upsert_menu_cantine_for_actor(
            Role::Administratif.as_str(),
            UpsertMenuCantine {
                etablissement_id: cast.etablissement_id,
                jour: jour + Duration::days(1),
                entree: "Soupe".into(),
                plat: "Poisson pané".into(),
                dessert: "Fruit".into(),
                notes: String::new(),
                publie: false,
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini publie menu {} ({}) + brouillon J+1",
        menu.jour, menu.plat
    ));

    let menus_f = ent
        .list_menus_cantine_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            jour,
            jour + Duration::days(7),
        )
        .await
        .map_err(|e| e.to_string())?;
    if menus_f.len() != 1 || menus_f[0].plat != "Poulet rôti" {
        return Err(format!(
            "Lily doit voir 1 menu publié, got {:?}",
            menus_f
                .iter()
                .map(|m| (m.jour, m.plat.clone(), m.publie))
                .collect::<Vec<_>>()
        ));
    }
    r.geste("Lily lit seul le menu publié (pas le brouillon)");

    let menus_staff = ent
        .list_menus_cantine_for_actor(
            cast.etablissement_id,
            admini,
            Role::Administratif.as_str(),
            jour,
            jour + Duration::days(7),
        )
        .await
        .map_err(|e| e.to_string())?;
    if menus_staff.len() != 2 {
        return Err(format!(
            "staff doit voir 2 menus (publié+brouillon), got {}",
            menus_staff.len()
        ));
    }
    r.geste("admini voit publié + brouillon");

    let tarif = ent
        .creer_tarif_conso_for_actor(
            Role::Administratif.as_str(),
            CreerTarifConso {
                etablissement_id: cast.etablissement_id,
                code: "MIDI".into(),
                libelle: "Repas midi".into(),
                montant_centimes: 450,
                debut: jour - Duration::days(30),
                fin: None,
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let tarifs = ent
        .list_tarifs_conso_for_actor(cast.etablissement_id, Role::Compta.as_str(), Some(jour))
        .await
        .map_err(|e| e.to_string())?;
    if !tarifs.iter().any(|t| t.id == tarif.id && t.montant_centimes == 450) {
        return Err("tarif MIDI 4,50 € attendu au jour J".into());
    }
    r.geste(format!(
        "admini crée tarif {} = {:.2} € — compta le voit au jour J",
        tarif.code,
        tarif.montant_centimes as f64 / 100.0
    ));

    r.trou("vacances + homonymes + menus + tarifs conso OK — UI /app/etab + /app/eleves homonymes + cantine menus/tarifs");
    Ok(())
}

async fn explorer_quotient_tva_trombi(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# quotient / TVA / trombi");

    let compta = cast.comptas[0].user_id;
    let admini = cast.administratifs[0].user_id;
    let lily = cast.parent_lea.user_id;
    let harry = &cast.eleves[0];
    let annee = annee_scolaire_de(jour);

    let q = ent
        .upsert_quotient_foyer_for_actor(
            Role::Compta.as_str(),
            UpsertQuotientFoyer {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                annee_scolaire: annee.clone(),
                quotient_centimes: 85_050,
                parts_x100: 250,
                actor_user_id: compta,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "compta pose quotient foyer Léa {} € (parts {})",
        q.quotient_centimes as f64 / 100.0,
        q.parts_x100 as f64 / 100.0
    ));

    let q_parent = ent
        .get_quotient_foyer_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            cast.foyer_lea_id,
            &annee,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Lily doit lire son quotient".to_string())?;
    if q_parent.quotient_centimes != 85_050 {
        return Err("quotient Lily incorrect".into());
    }
    r.geste("Lily lit son quotient familial");

    let refuse_parent = ent
        .upsert_quotient_foyer_for_actor(
            Role::Parent.as_str(),
            UpsertQuotientFoyer {
                etablissement_id: cast.etablissement_id,
                foyer_id: cast.foyer_lea_id,
                annee_scolaire: annee.clone(),
                quotient_centimes: 1,
                parts_x100: 100,
                actor_user_id: lily,
            },
        )
        .await;
    if refuse_parent.is_ok() {
        return Err("parent ne doit pas modifier le quotient".into());
    }
    r.geste("Lily refusée sur écriture quotient (attendu)");

    let tva = ent
        .creer_taux_tva_for_actor(
            Role::Compta.as_str(),
            CreerTauxTva {
                etablissement_id: cast.etablissement_id,
                code: "TVA55".into(),
                libelle: "TVA 5,5 % restauration".into(),
                taux_bps: 550,
                actor_user_id: compta,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let ttc = ent
        .calculer_ttc_for_actor(
            cast.etablissement_id,
            Role::Compta.as_str(),
            10_000,
            tva.taux_bps,
        )
        .await
        .map_err(|e| e.to_string())?;
    if ttc != 10_550 {
        return Err(format!("TTC 5,5% attendu 10550, got {ttc}"));
    }
    r.geste(format!(
        "compta crée taux {} → 100,00 € HT = {:.2} € TTC",
        tva.code,
        ttc as f64 / 100.0
    ));

    let png_1x1 = [
        0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8,
        0xcf, 0xc0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x05, 0xfe, 0xd4, 0xef, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let photo = ent
        .poser_photo_eleve_for_actor(
            Role::Administratif.as_str(),
            cast.etablissement_id,
            harry.id,
            admini,
            "image/png",
            &png_1x1,
        )
        .await
        .map_err(|e| e.to_string())?;
    if photo.photo_key.is_none() {
        return Err("photo_key attendue après upload".into());
    }
    r.geste("admini pose photo trombi Harry (PNG)");

    let trombi = ent
        .list_trombi_classe_for_actor(
            cast.etablissement_id,
            Role::Prof.as_str(),
            harry.classe.as_str(),
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !trombi
        .iter()
        .any(|t| t.eleve_id == harry.id && t.photo_key.is_some())
    {
        return Err("trombi 4B doit inclure Harry avec photo".into());
    }
    r.geste(format!(
        "Rogue lit trombi {} ({} élèves, Harry photo OK)",
        harry.classe,
        trombi.len()
    ));

    let bytes = ent
        .get_photo_eleve_bytes_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Lily doit télécharger la photo".to_string())?;
    if bytes != png_1x1 {
        return Err("bytes photo Lily ≠ upload".into());
    }
    r.geste("Lily télécharge la photo de Harry");

    r.trou("quotient + TVA + trombi photo OK — UI /app/facturation quotient/TVA + trombi /app/eleves");
    Ok(())
}

async fn explorer_param_salles(
    ent: &PgEnt,
    cast: &LaboCast,
    _jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# paramètres établissement + salles");

    let admini = cast.administratifs[0].user_id;
    let rogue = cast.prof.user_id;

    let p = ent
        .upsert_param_etab_for_actor(
            Role::Administratif.as_str(),
            UpsertParamEtab {
                etablissement_id: cast.etablissement_id,
                cle: "seuil_absences".into(),
                valeur: "4".into(),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("admini pose param {}={}", p.cle, p.valeur));

    let refuse = ent
        .upsert_param_etab_for_actor(
            Role::Prof.as_str(),
            UpsertParamEtab {
                etablissement_id: cast.etablissement_id,
                cle: "x".into(),
                valeur: "1".into(),
                actor_user_id: rogue,
            },
        )
        .await;
    if refuse.is_ok() {
        return Err("prof ne doit pas écrire params etab".into());
    }
    r.geste("prof refusé sur écriture params (attendu)");

    let lu = ent
        .get_param_etab_for_actor(
            cast.etablissement_id,
            Role::Prof.as_str(),
            "seuil_absences",
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Rogue doit lire seuil_absences".to_string())?;
    if lu.valeur != "4" {
        return Err("valeur param incorrecte".into());
    }
    r.geste("Rogue lit seuil_absences=4");

    let salle = ent
        .creer_salle_for_actor(
            Role::Administratif.as_str(),
            CreerSalle {
                etablissement_id: cast.etablissement_id,
                code: "B12".into(),
                libelle: "Salle B12".into(),
                capacite: Some(32),
                site: Some("principal".into()),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let salles = ent
        .list_salles_for_actor(cast.etablissement_id, Role::Prof.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if !salles.iter().any(|s| s.id == salle.id && s.code == "B12") {
        return Err("salle B12 absente du listing".into());
    }
    r.geste(format!(
        "admini crée salle {} ({}) — Rogue en voit {}",
        salle.code,
        salle.libelle,
        salles.len()
    ));

    r.trou("params etab + salles + motifs absence OK — UI /app/etab");
    Ok(())
}

async fn explorer_filieres_voeux_mails(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# filières / vœux / historique mails");

    let admini = cast.administratifs[0].user_id;
    let lily = cast.parent_lea.user_id;
    let harry = &cast.eleves[0];
    let annee = annee_scolaire_de(jour);

    let sti = ent
        .creer_filiere_for_actor(
            Role::Administratif.as_str(),
            CreerFiliere {
                etablissement_id: cast.etablissement_id,
                code: "STI2D".into(),
                libelle: "Sciences et technologies de l'industrie".into(),
                mef: Some("MEFSTI".into()),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let gen = ent
        .creer_filiere_for_actor(
            Role::Administratif.as_str(),
            CreerFiliere {
                etablissement_id: cast.etablissement_id,
                code: "GT".into(),
                libelle: "Voie générale".into(),
                mef: None,
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini crée filières {} + {}",
        sti.code, gen.code
    ));

    let v1 = ent
        .poser_voeu_for_actor(
            Role::Administratif.as_str(),
            PoserVoeu {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                filiere_id: sti.id,
                rang: 1,
                annee_scolaire: annee.clone(),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let _ = ent
        .poser_voeu_for_actor(
            Role::Administratif.as_str(),
            PoserVoeu {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                filiere_id: gen.id,
                rang: 2,
                annee_scolaire: annee.clone(),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini pose vœux Harry: 1={} 2={}",
        sti.code, gen.code
    ));

    let voeux_f = ent
        .list_voeux_eleve_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
            &annee,
        )
        .await
        .map_err(|e| e.to_string())?;
    if voeux_f.len() != 2 || voeux_f[0].filiere_id != v1.filiere_id {
        return Err(format!(
            "Lily doit voir 2 vœux Harry, got {}",
            voeux_f.len()
        ));
    }
    r.geste("Lily lit les vœux de Harry");

    let filieres_f = ent
        .list_filieres_for_actor(cast.etablissement_id, Role::Parent.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if filieres_f.len() < 2 {
        return Err("parent doit lister les filières".into());
    }
    r.geste(format!("Lily voit {} filière(s)", filieres_f.len()));

    let mails = ent
        .list_mails_recents_for_actor(cast.etablissement_id, Role::Administratif.as_str(), 20)
        .await
        .map_err(|e| e.to_string())?;
    if mails.is_empty() {
        return Err("historique mails staff ne doit pas être vide après lab".into());
    }
    r.geste(format!(
        "admini lit historique mails ({} entrées, ex. « {} »)",
        mails.len(),
        mails[0].subject
    ));

    let refuse = ent
        .list_mails_recents_for_actor(cast.etablissement_id, Role::Parent.as_str(), 5)
        .await;
    if refuse.is_ok() {
        return Err("parent ne doit pas lire l'outbox mails".into());
    }
    r.geste("Lily refusée sur historique mails (attendu)");

    let sms = ent
        .list_sms_recents_for_actor(cast.etablissement_id, Role::Cpe.as_str(), 20)
        .await
        .map_err(|e| e.to_string())?;
    if sms.is_empty() {
        return Err("historique SMS staff ne doit pas être vide après lab".into());
    }
    r.geste(format!(
        "CPE lit historique SMS ({} entrées)",
        sms.len()
    ));

    let annee = annee_scolaire_de(jour);
    let csv_ps = ent
        .exporter_parcoursup_csv_for_actor(
            Role::Administratif.as_str(),
            ExporterParcoursup {
                etablissement_id: cast.etablissement_id,
                annee_scolaire: annee.clone(),
                classe: "4B".into(),
            },
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !csv_ps.contains("formation;rang_voeu") || !csv_ps.contains("Potter") {
        return Err(format!("export Parcoursup invalide: {csv_ps}"));
    }
    r.geste(format!(
        "export Parcoursup CSV 4B ({} octets, année {annee})",
        csv_ps.len()
    ));

    r.trou("filières + vœux + historique mails/SMS OK — UI /app/etab (filières + outbox) + natives vœux");
    Ok(())
}

async fn explorer_inscription_suivi_iban(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# suivi inscription + histo IBAN");

    let admini = cast.administratifs[0].user_id;
    let lily = cast.parent_lea.user_id;
    let harry = &cast.eleves[0];
    let annee = annee_scolaire_de(jour);

    let suivi = ent
        .upsert_inscription_suivi_for_actor(
            Role::Administratif.as_str(),
            UpsertInscriptionSuivi {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                annee_scolaire: annee.clone(),
                etape: "pieces".into(),
                motif_code: Some("incomplet".into()),
                commentaire: "Attente livret de famille".into(),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini pose suivi Harry étape={} motif={:?}",
        suivi.etape.as_str(),
        suivi.motif_code
    ));

    let _ = ent
        .upsert_inscription_suivi_for_actor(
            Role::Administratif.as_str(),
            UpsertInscriptionSuivi {
                etablissement_id: cast.etablissement_id,
                eleve_id: harry.id,
                annee_scolaire: annee.clone(),
                etape: "entretien".into(),
                motif_code: None,
                commentaire: "RDV fixés".into(),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    let lu = ent
        .get_inscription_suivi_for_actor(
            cast.etablissement_id,
            lily,
            Role::Parent.as_str(),
            harry.id,
            &annee,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Lily doit voir le suivi".to_string())?;
    if lu.etape.as_str() != "entretien" {
        return Err(format!("étape attendue entretien, got {}", lu.etape.as_str()));
    }
    r.geste("Lily lit suivi inscription (entretien)");

    let file = ent
        .list_inscriptions_par_etape_for_actor(
            cast.etablissement_id,
            Role::Administratif.as_str(),
            &annee,
            "entretien",
        )
        .await
        .map_err(|e| e.to_string())?;
    if !file.iter().any(|s| s.eleve_id == harry.id) {
        return Err("file entretien doit contenir Harry".into());
    }
    r.geste(format!("admini file étape entretien: {} dossier(s)", file.len()));

    let histo = ent
        .list_iban_histo_foyer_for_actor(
            cast.etablissement_id,
            cast.compta.user_id,
            Role::Compta.as_str(),
            cast.foyer_lea_id,
            10,
        )
        .await
        .map_err(|e| e.to_string())?;
    if histo.is_empty() {
        return Err("histo IBAN foyer Léa attendu après mandat SEPA facturation".into());
    }
    r.geste(format!(
        "compta lit histo IBAN foyer ({} entrées, dernier {})",
        histo.len(),
        histo[0].iban
    ));

    r.trou("suivi inscription + histo IBAN OK — UI /app/inscriptions + histo IBAN facturation");
    Ok(())
}

async fn explorer_absence_rh(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# absences RH personnel");

    let rogue = cast.prof.user_id;
    let admini = cast.administratifs[0].user_id;

    let abs = ent
        .declarer_absence_rh_for_actor(
            Role::Prof.as_str(),
            DeclarerAbsenceRh {
                etablissement_id: cast.etablissement_id,
                user_id: rogue,
                debut: jour,
                fin: jour + Duration::days(1),
                motif: "maladie".into(),
                actor_user_id: rogue,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "Rogue déclare absence RH {} → {} ({})",
        abs.debut, abs.fin, abs.motif
    ));

    let refuse = ent
        .valider_absence_rh_for_actor(
            cast.etablissement_id,
            Role::Prof.as_str(),
            abs.id,
            rogue,
        )
        .await;
    if refuse.is_ok() {
        return Err("prof ne doit pas valider sa propre absence RH".into());
    }
    r.geste("Rogue refusé sur validation RH (attendu)");

    let validee = ent
        .valider_absence_rh_for_actor(
            cast.etablissement_id,
            Role::Administratif.as_str(),
            abs.id,
            admini,
        )
        .await
        .map_err(|e| e.to_string())?;
    if validee.statut != "validee" {
        return Err(format!("statut attendu validee, got {}", validee.statut));
    }
    r.geste("admini valide l'absence RH de Rogue");

    let list = ent
        .list_absences_rh_for_actor(
            cast.etablissement_id,
            Role::Direction.as_str(),
            jour,
            jour + Duration::days(7),
        )
        .await
        .map_err(|e| e.to_string())?;
    if !list.iter().any(|a| a.id == abs.id && a.statut == "validee") {
        return Err("direction doit voir l'absence validée".into());
    }
    r.geste(format!("direction liste {} absence(s) RH période", list.len()));

    r.trou("absences RH: déclaration + validation OK — UI /app/rh");
    Ok(())
}

async fn explorer_edt_affectation_editions(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# EDT affectation + éditions VS");

    let cpe = cast.cpe.user_id;
    let rogue = cast.prof.user_id;
    let mcg = cast.profs.get(1).map(|p| p.user_id).unwrap_or(rogue);
    let admini = cast.administratifs[0].user_id;

    let salle = ent
        .creer_salle_for_actor(
            Role::Administratif.as_str(),
            CreerSalle {
                etablissement_id: cast.etablissement_id,
                code: "AFF-01".into(),
                libelle: "Salle affectation lab".into(),
                capacite: Some(30),
                site: None,
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("admini crée salle {} pour affectation EDT", salle.code));

    let refuse_prof = ent
        .affecter_edt_creneau_for_actor(
            Role::Prof.as_str(),
            AffecterEdtCreneau {
                etablissement_id: cast.etablissement_id,
                creneau_id: cast.creneau_rogue_4b,
                prof_user_id: Some(mcg),
                salle: Some(salle.code.clone()),
                actor_user_id: rogue,
            },
            Some(salle.id),
        )
        .await;
    if refuse_prof.is_ok() {
        return Err("prof ne doit pas affecter un créneau EDT".into());
    }
    r.geste("Rogue refusé sur affectation EDT (attendu)");

    let aff = ent
        .affecter_edt_creneau_for_actor(
            Role::Cpe.as_str(),
            AffecterEdtCreneau {
                etablissement_id: cast.etablissement_id,
                creneau_id: cast.creneau_rogue_4b,
                prof_user_id: Some(mcg),
                salle: None,
                actor_user_id: cpe,
            },
            Some(salle.id),
        )
        .await
        .map_err(|e| e.to_string())?;
    if aff.prof_user_id != mcg || aff.salle.as_deref() != Some(salle.code.as_str()) {
        return Err(format!(
            "affectation attendue prof={mcg} salle={}, got {:?} {:?}",
            salle.code, aff.prof_user_id, aff.salle
        ));
    }
    r.geste(format!(
        "CPE affecte créneau Potions → prof McG + salle {}",
        salle.code
    ));

    // Remettre Rogue pour ne pas casser les explorers suivants qui s'attendent à Rogue.
    let _ = ent
        .affecter_edt_creneau_for_actor(
            Role::Cpe.as_str(),
            AffecterEdtCreneau {
                etablissement_id: cast.etablissement_id,
                creneau_id: cast.creneau_rogue_4b,
                prof_user_id: Some(rogue),
                salle: Some("Donjon".into()),
                actor_user_id: cpe,
            },
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("CPE réaffecte Potions à Rogue (Donjon) pour suite lab");

    let tpl = ent
        .creer_edition_vs_for_actor(
            Role::Cpe.as_str(),
            CreerEditionVs {
                etablissement_id: cast.etablissement_id,
                code: "CONVOC".into(),
                titre: "Convocation VS".into(),
                corps: "Madame, Monsieur,\n\nNous convoquons {{prenom}} {{nom}} ({{classe}}) le {{date}}.\n\nLa vie scolaire.".into(),
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!("CPE crée modèle édition VS {}", tpl.code));

    let rendu = ent
        .rendre_edition_vs_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            tpl.id,
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !rendu.contains("Harry") || !rendu.contains("4B") || !rendu.contains(&jour.to_string()) {
        return Err(format!("rendu édition VS invalide: {rendu}"));
    }
    r.geste("CPE rend convocation Harry (placeholders OK)");

    let rendu_code = ent
        .rendre_edition_vs_par_code_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            "CONVOC",
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if rendu_code != rendu {
        return Err("rendu par code CONVOC doit matcher le rendu par id".into());
    }
    r.geste("CPE rend aussi via code CONVOC");

    let list = ent
        .list_editions_vs_for_actor(cast.etablissement_id, Role::Prof.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if !list.iter().any(|t| t.id == tpl.id) {
        return Err("listing éditions VS doit contenir CONVOC".into());
    }
    r.geste(format!("staff liste {} modèle(s) édition VS", list.len()));

    let ctr = ent
        .creer_edt_contrainte_for_actor(
            Role::Cpe.as_str(),
            CreerEdtContrainte {
                etablissement_id: cast.etablissement_id,
                kind: EdtContrainteKind::ProfIndispo,
                prof_user_id: Some(rogue),
                salle_code: None,
                jour_semaine: Some(jour_semaine_iso(jour)),
                heure_debut: Some(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
                heure_fin: Some(NaiveTime::from_hms_opt(14, 0, 0).unwrap()),
                detail: "réunion pédagogique".into(),
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "CPE pose contrainte soft {} pour Rogue (midi)",
        ctr.kind.as_str()
    ));

    let ctrs = ent
        .list_edt_contraintes_for_actor(cast.etablissement_id, Role::Cpe.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if !ctrs.iter().any(|c| c.id == ctr.id) {
        return Err("listing contraintes EDT doit contenir l'indispo Rogue".into());
    }
    r.geste(format!("CPE liste {} contrainte(s) EDT soft", ctrs.len()));

    // Indispo midi Rogue → greedy doit placer hors 12–14.
    let solve_soft = ent
        .resoudre_edt_for_actor(
            cast.etablissement_id,
            Role::Cpe.as_str(),
            cpe,
            vec![domain_eleve::EdtDemande {
                classe: "2A".into(),
                matiere_libelle: "Potions soft".into(),
                prof_user_id: rogue,
                duree_minutes: 60,
                salle: Some("Soft-1".into()),
                semaine_ab: EdtSemaineAb::Toutes,
            }],
            true,
        )
        .await
        .map_err(|e| e.to_string())?;
    if solve_soft.placed.len() != 1 {
        return Err(format!(
            "solveur soft attendu 1 placé, got {}",
            solve_soft.placed.len()
        ));
    }
    let p = &solve_soft.placed[0];
    if p.jour_semaine == jour_semaine_iso(jour)
        && p.heure_debut >= NaiveTime::from_hms_opt(12, 0, 0).unwrap()
        && p.heure_debut < NaiveTime::from_hms_opt(14, 0, 0).unwrap()
    {
        return Err("solveur soft a placé Rogue sur son indispo midi".into());
    }
    r.geste(format!(
        "solveur greedy respecte indispo Rogue → {} {:02}:{:02}",
        p.jour_semaine,
        p.heure_debut.hour(),
        p.heure_debut.minute()
    ));

    let edt_f = ent
        .list_edt_eleve_jour_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            cast.parent_lea.user_id,
            cast.eleve_lea_id,
            jour,
        )
        .await
        .map_err(|e| e.to_string())?;
    if edt_f.is_empty() {
        return Err("Lily doit voir l'EDT classe de Harry".into());
    }
    r.geste(format!(
        "Lily consulte EDT Harry ({} créneau(x) classe)",
        edt_f.len()
    ));
    let refuse_edt = ent
        .list_edt_eleve_jour_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            cast.parent_tom.user_id,
            cast.eleve_lea_id,
            jour,
        )
        .await;
    if refuse_edt.is_ok() {
        return Err("Narcissa ne doit pas lire l'EDT de Harry".into());
    }
    r.geste("Narcissa refusée sur EDT Harry (attendu)");

    r.trou("EDT affectation + éditions VS + contraintes soft + import/solveur UI /app/edt OK");
    Ok(())
}

async fn explorer_stages(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# stages / entreprises / conventions");

    let admini = cast.administratifs[0].user_id;
    let cpe = cast.cpe.user_id;

    let refuse = ent
        .creer_entreprise_for_actor(
            Role::Prof.as_str(),
            CreerEntreprise {
                etablissement_id: cast.etablissement_id,
                raison_sociale: "Interdit SA".into(),
                siret: None,
                adresse: "".into(),
                actor_user_id: cast.prof.user_id,
            },
        )
        .await;
    if refuse.is_ok() {
        return Err("prof ne doit pas créer d'entreprise".into());
    }
    r.geste("Rogue refusé création entreprise (attendu)");

    let ent_co = ent
        .creer_entreprise_for_actor(
            Role::Administratif.as_str(),
            CreerEntreprise {
                etablissement_id: cast.etablissement_id,
                raison_sociale: "Weasleys' Wizard Wheezes".into(),
                siret: Some("12345678901234".into()),
                adresse: "93 Diagon Alley".into(),
                actor_user_id: admini,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "admini crée entreprise {} (SIRET {})",
        ent_co.raison_sociale,
        ent_co.siret.as_deref().unwrap_or("—")
    ));

    let conv = ent
        .creer_convention_stage_for_actor(
            Role::Cpe.as_str(),
            CreerConventionStage {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                entreprise_id: ent_co.id,
                debut: jour,
                fin: jour + Duration::days(14),
                tuteur_nom: "Fred Weasley".into(),
                actor_user_id: cpe,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "CPE crée convention Harry chez {} · tuteur {}",
        ent_co.raison_sociale, conv.tuteur_nom
    ));

    let list_f = ent
        .list_conventions_eleve_for_actor(
            cast.etablissement_id,
            Role::Parent.as_str(),
            cast.parent_lea.user_id,
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !list_f.iter().any(|c| c.id == conv.id) {
        return Err("Lily doit voir la convention de stage de Harry".into());
    }
    r.geste("Lily lit la convention de stage de Harry");

    let entreprises = ent
        .list_entreprises_for_actor(cast.etablissement_id, Role::Cpe.as_str())
        .await
        .map_err(|e| e.to_string())?;
    if !entreprises.iter().any(|e| e.id == ent_co.id) {
        return Err("listing entreprises doit contenir WWW".into());
    }
    r.geste(format!("CPE liste {} entreprise(s)", entreprises.len()));

    r.trou("stages: entreprises + conventions OK — UI /app/stages + lecture famille natives");
    Ok(())
}

async fn explorer_staff_sans_metier(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# staff présent sans surface métier → trous");

    // Infirmière / psy : lisent l'effectif mais aucun geste santé / entretien.
    let inf = &cast.infirmieres[0];
    let voit = ent
        .get_eleve(
            cast.etablissement_id,
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !voit {
        return Err("infirmière devrait lire l'élève (staff)".into());
    }
    r.geste(format!(
        "{} voit le dossier élève (lecture)",
        inf.email
    ));

    let passage = ent
        .creer_passage_infirmerie_for_actor(
            Role::Infirmerie.as_str(),
            CreerPassageInfirmerie {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                jour,
                motif: PassageInfirmerieMotif::Malaise,
                note_soins: "coups d air, repos 20 min, parents prevenus".into(),
                visible_famille: true,
                actor_user_id: inf.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "infirmière enregistre passage {} Harry (visible famille)",
        passage.motif.as_str()
    ));

    let lily = ent
        .list_passages_infirmerie_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            Some(cast.eleve_lea_id),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    let vu = lily.iter().find(|p| p.id == passage.id).ok_or("Lily doit voir le passage")?;
    if !vu.note_soins.is_empty() {
        return Err("Lily ne doit pas voir la note de soins".into());
    }
    if vu.motif != PassageInfirmerieMotif::Malaise {
        return Err("Lily doit voir le motif malaise".into());
    }
    r.geste("Lily voit le passage (motif) sans note de soins");

    let narc = ent
        .list_passages_infirmerie_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            Some(cast.eleve_lea_id),
            None,
        )
        .await;
    if narc.is_ok() {
        return Err("Narcissa ne doit pas lire l'infirmerie Harry".into());
    }

    let prof_refuse = ent
        .creer_passage_infirmerie_for_actor(
            Role::Prof.as_str(),
            CreerPassageInfirmerie {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                jour,
                motif: PassageInfirmerieMotif::Soins,
                note_soins: "interdit".into(),
                visible_famille: false,
                actor_user_id: cast.profs[0].user_id,
            },
        )
        .await;
    if prof_refuse.is_ok() {
        return Err("prof ne doit pas saisir un passage infirmière".into());
    }
    r.geste("Narcissa bloquée · prof refusé à la saisie (attendu)");

    let fiche = ent
        .upsert_fiche_sante_for_actor(
            Role::Infirmerie.as_str(),
            UpsertFicheSante {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                allergies: "arachides".into(),
                traitements: "Ventoline si besoin".into(),
                contre_indications: "aspirine".into(),
                pai_actif: true,
                medecin_nom: Some("Dr Pomfrey".into()),
                urgence_tel: Some("0601020304".into()),
                note_confidentielle: "asthme d'effort — ne pas communiquer aux autres parents".into(),
                actor_user_id: inf.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !fiche.pai_actif || fiche.allergies != "arachides" {
        return Err("fiche santé PAI + allergies attendus".into());
    }
    r.geste("infirmière pose fiche santé Harry (PAI + allergies)");

    let lily_fs = ent
        .get_fiche_sante_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Lily doit lire la fiche santé")?;
    if !lily_fs.note_confidentielle.is_empty() {
        return Err("Lily ne doit pas voir la note confidentielle fiche santé".into());
    }
    if lily_fs.allergies != "arachides" || !lily_fs.pai_actif {
        return Err("Lily doit voir allergies + PAI".into());
    }
    r.geste("Lily lit fiche santé (allergies/PAI) sans note confidentielle");

    let narc_fs = ent
        .get_fiche_sante_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await;
    if narc_fs.is_ok() {
        return Err("Narcissa ne doit pas lire la fiche santé Harry".into());
    }
    let prof_fs = ent
        .upsert_fiche_sante_for_actor(
            Role::Prof.as_str(),
            UpsertFicheSante {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                allergies: "x".into(),
                traitements: String::new(),
                contre_indications: String::new(),
                pai_actif: false,
                medecin_nom: None,
                urgence_tel: None,
                note_confidentielle: String::new(),
                actor_user_id: cast.profs[0].user_id,
            },
        )
        .await;
    if prof_fs.is_ok() {
        return Err("prof ne doit pas écrire la fiche santé".into());
    }
    r.geste("Narcissa bloquée · Rogue refusé sur fiche santé (attendu)");

    // Upsert idempotent (même élève).
    let fiche2 = ent
        .upsert_fiche_sante_for_actor(
            Role::Infirmerie.as_str(),
            UpsertFicheSante {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                allergies: "arachides, pollen".into(),
                traitements: "Ventoline si besoin".into(),
                contre_indications: "aspirine".into(),
                pai_actif: true,
                medecin_nom: Some("Dr Pomfrey".into()),
                urgence_tel: Some("0601020304".into()),
                note_confidentielle: "asthme d'effort — maj".into(),
                actor_user_id: inf.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if fiche2.id != fiche.id {
        return Err("upsert fiche santé doit conserver le même id".into());
    }
    if fiche2.allergies != "arachides, pollen" {
        return Err("allergies mises à jour attendues".into());
    }
    r.geste("upsert fiche santé idempotent (allergies mises à jour)");

    let pdf_fs = ent
        .resume_fiche_sante_pdf_for_actor(
            cast.etablissement_id,
            inf.user_id,
            Role::Infirmerie.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !pdf_fs.starts_with(b"%PDF-1.4") || !pdf_fs.windows(5).any(|w| w == b"%%EOF") {
        return Err("PDF fiche santé invalide".into());
    }
    let pdf_fs_txt = String::from_utf8_lossy(&pdf_fs);
    if pdf_fs_txt.contains("asthme") || pdf_fs_txt.contains("communiquer") {
        return Err("PDF fiche santé ne doit pas contenir la note confidentielle".into());
    }
    if !pdf_fs_txt.contains("arachides") {
        return Err("PDF fiche santé doit contenir les allergies".into());
    }
    r.geste(format!(
        "infirmière génère PDF résumé fiche santé Harry ({} o)",
        pdf_fs.len()
    ));
    let lily_fs_pdf = ent
        .resume_fiche_sante_pdf_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !lily_fs_pdf.starts_with(b"%PDF-1.4") {
        return Err("Lily doit télécharger le PDF fiche santé".into());
    }
    let prof_fs_pdf = ent
        .resume_fiche_sante_pdf_for_actor(
            cast.etablissement_id,
            cast.profs[0].user_id,
            Role::Prof.as_str(),
            cast.eleve_lea_id,
        )
        .await;
    if prof_fs_pdf.is_ok() {
        return Err("prof ne doit pas télécharger le PDF fiche santé".into());
    }
    r.geste("Lily télécharge PDF fiche · Rogue refusé (attendu)");

    let pai = ent
        .upsert_pai_for_actor(
            Role::Infirmerie.as_str(),
            UpsertPai {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                motif: "asthme".into(),
                mesures_accueil: "Ventoline à l'infirmerie avant sport".into(),
                protocole_urgence: "Appeler SAMU si crise sévère".into(),
                note_interne: "antécédent hospitalisation 2024".into(),
                date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                date_fin: Some(NaiveDate::from_ymd_opt(2027, 7, 5).unwrap()),
                actif: true,
                actor_user_id: inf.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if !pai.actif || pai.signe_famille_at.is_some() {
        return Err("PAI doit naître actif non signé".into());
    }
    r.geste("infirmière pose PAI structuré Harry (asthme)");

    let lily_pai = ent
        .get_pai_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Lily doit voir le PAI actif")?;
    if !lily_pai.note_interne.is_empty() {
        return Err("Lily ne doit pas voir la note interne PAI".into());
    }
    if lily_pai.motif != "asthme" {
        return Err("Lily doit voir le motif PAI".into());
    }
    r.geste("Lily lit PAI (mesures) sans note interne");

    let signe = ent
        .signer_pai_famille_for_actor(
            Role::Parent.as_str(),
            SignerPaiFamille {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if signe.signe_famille_at.is_none() {
        return Err("PAI doit être signé après geste parent".into());
    }
    let re_signe = ent
        .signer_pai_famille_for_actor(
            Role::Parent.as_str(),
            SignerPaiFamille {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                actor_user_id: cast.parent_lea.user_id,
            },
        )
        .await;
    if re_signe.is_ok() {
        return Err("double signature PAI refusée".into());
    }
    r.geste("Lily signe le PAI · re-signature refusée (attendu)");

    let fiche_sync = ent
        .get_fiche_sante_for_actor(
            cast.etablissement_id,
            inf.user_id,
            Role::Infirmerie.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or("fiche santé doit exister")?;
    if !fiche_sync.pai_actif {
        return Err("flag fiche.pai_actif doit suivre le PAI actif".into());
    }
    let narc_pai = ent
        .get_pai_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await;
    if narc_pai.is_ok() {
        return Err("Narcissa ne doit pas lire le PAI Harry".into());
    }
    let prof_pai = ent
        .upsert_pai_for_actor(
            Role::Prof.as_str(),
            UpsertPai {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                motif: "interdit".into(),
                mesures_accueil: "x".into(),
                protocole_urgence: String::new(),
                note_interne: String::new(),
                date_debut: jour,
                date_fin: None,
                actif: true,
                actor_user_id: cast.profs[0].user_id,
            },
        )
        .await;
    if prof_pai.is_ok() {
        return Err("prof ne doit pas saisir un PAI".into());
    }
    r.geste("fiche.pai_actif synchronisé · Narcissa/Rogue refusés (attendu)");

    let ordo_bytes = b"%PDF-1.4\nordo lab\n%%EOF\n".to_vec();
    let ordo = ent
        .attacher_ordonnance_pai_for_actor(
            Role::Infirmerie.as_str(),
            AttacherOrdonnancePai {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                nom: "ordo-asthme.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: ordo_bytes.clone(),
                actor_user_id: inf.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if ordo.taille_octets <= 0 {
        return Err("ordonnance doit avoir une taille".into());
    }
    let key = ordo
        .storage_key
        .as_deref()
        .ok_or("ordonnance doit avoir une storage_key objet")?;
    if !ent.objects().exists(key) {
        return Err(format!("objet PAI introuvable pour clé {key}"));
    }
    if !ordo.contenu.is_empty() {
        return Err("contenu PAI doit être hors base (object store)".into());
    }
    if !ent.objects().is_s3_live() {
        return Err("object store lab attendu en S3 live (disque)".into());
    }
    let uri = ent.objects().s3_uri(key);
    if !uri.starts_with("s3://scola-lab/") {
        return Err(format!("URI S3 live inattendue: {uri}"));
    }
    let on_disk = ent
        .objects()
        .root()
        .join(ent.objects().s3_bucket())
        .join(key);
    if !on_disk.is_file() {
        return Err(format!("fichier S3 live absent: {}", on_disk.display()));
    }
    r.geste(format!(
        "infirmière joint ordonnance {} ({} o) → {uri} (live disque)",
        ordo.nom, ordo.taille_octets
    ));

    let lily_pieces = ent
        .list_pai_pieces_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !lily_pieces.iter().any(|p| p.id == ordo.id) {
        return Err("Lily doit voir l'ordonnance PAI".into());
    }
    let lily_dl = ent
        .telecharger_pai_piece_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            ordo.id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if lily_dl.contenu != ordo_bytes {
        return Err("contenu ordonnance téléchargé incorrect (object store)".into());
    }
    r.geste("Lily liste et télécharge l'ordonnance PAI (S3 live disque)");

    let narc_ordo = ent
        .telecharger_pai_piece_for_actor(
            cast.etablissement_id,
            cast.parent_tom.user_id,
            Role::Parent.as_str(),
            ordo.id,
        )
        .await;
    if narc_ordo.is_ok() {
        return Err("Narcissa ne doit pas télécharger l'ordonnance Harry".into());
    }
    let prof_ordo = ent
        .attacher_ordonnance_pai_for_actor(
            Role::Prof.as_str(),
            AttacherOrdonnancePai {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                nom: "hack.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: b"%PDF-1.4\nx\n%%EOF\n".to_vec(),
                actor_user_id: cast.profs[0].user_id,
            },
        )
        .await;
    if prof_ordo.is_ok() {
        return Err("prof ne doit pas joindre une ordonnance PAI".into());
    }
    r.geste("Narcissa/Rogue refusés sur ordonnance PAI (attendu)");

    let psy = &cast.psys[0];
    let entretien = ent
        .creer_entretien_psy_for_actor(
            Role::Psy.as_str(),
            CreerEntretienPsy {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                jour,
                kind: EntretienPsyKind::Individuel,
                duree_minutes: 45,
                note_confidentielle: "suivi anxiete scolaire - confidentiel".into(),
                signale_famille: true,
                actor_user_id: psy.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "psy enregistre entretien {} Harry (signale famille)",
        entretien.kind.as_str()
    ));

    let lily_psy = ent
        .list_entretiens_psy_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            Some(cast.eleve_lea_id),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    let vu_psy = lily_psy
        .iter()
        .find(|e| e.id == entretien.id)
        .ok_or("Lily doit voir le signalement entretien")?;
    if !vu_psy.note_confidentielle.is_empty() {
        return Err("Lily ne doit pas voir la note psy".into());
    }
    r.geste("Lily voit entretien (type) sans note confidentielle");

    let cpe_psy = ent
        .list_entretiens_psy_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            Some(cast.eleve_lea_id),
            None,
        )
        .await;
    if cpe_psy.is_ok() {
        return Err("CPE ne doit pas lire les notes psy".into());
    }
    let inf_psy = ent
        .creer_entretien_psy_for_actor(
            Role::Infirmerie.as_str(),
            CreerEntretienPsy {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                jour,
                kind: EntretienPsyKind::Crise,
                duree_minutes: 30,
                note_confidentielle: "interdit".into(),
                signale_famille: false,
                actor_user_id: inf.user_id,
            },
        )
        .await;
    if inf_psy.is_ok() {
        return Err("infirmiere ne doit pas saisir un entretien psy".into());
    }
    r.geste("CPE bloque lecture psy · infirmiere refusee a la saisie (attendu)");

    let j_rdv = jour + Duration::days(3);
    let rdv = ent
        .creer_rdv_psy_for_actor(
            Role::Psy.as_str(),
            CreerRdvPsy {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                jour: j_rdv,
                heure_debut: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                heure_fin: NaiveTime::from_hms_opt(14, 45, 0).unwrap(),
                kind: EntretienPsyKind::Orientation,
                motif: "point orientation 3e".into(),
                note_interne: "préparer dossier Parcoursup".into(),
                signale_famille: true,
                actor_user_id: psy.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if rdv.statut != RdvPsyStatut::Planifie {
        return Err("RDV doit naître planifié".into());
    }
    r.geste(format!("psy planifie RDV orientation Harry le {j_rdv} 14:00"));

    let lily_rdv = ent
        .list_rdvs_psy_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            Some(cast.eleve_lea_id),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    let vu_rdv = lily_rdv
        .iter()
        .find(|r| r.id == rdv.id)
        .ok_or("Lily doit voir le RDV signalé")?;
    if !vu_rdv.note_interne.is_empty() {
        return Err("Lily ne doit pas voir la note interne RDV".into());
    }
    if vu_rdv.motif != "point orientation 3e" {
        return Err("Lily doit voir le motif RDV".into());
    }
    r.geste("Lily lit RDV (motif + horaires) sans note interne");

    let honore = ent
        .changer_statut_rdv_psy_for_actor(
            Role::Psy.as_str(),
            ChangerStatutRdvPsy {
                etablissement_id: cast.etablissement_id,
                rdv_id: rdv.id,
                statut: RdvPsyStatut::Honore,
                actor_user_id: psy.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if honore.statut != RdvPsyStatut::Honore {
        return Err("RDV doit passer honoré".into());
    }
    let re_honore = ent
        .changer_statut_rdv_psy_for_actor(
            Role::Psy.as_str(),
            ChangerStatutRdvPsy {
                etablissement_id: cast.etablissement_id,
                rdv_id: rdv.id,
                statut: RdvPsyStatut::Annule,
                actor_user_id: psy.user_id,
            },
        )
        .await;
    if re_honore.is_ok() {
        return Err("RDV déjà clos ne doit plus changer de statut".into());
    }
    r.geste("psy honore le RDV · re-changement refusé (attendu)");

    let cpe_rdv = ent
        .creer_rdv_psy_for_actor(
            Role::Cpe.as_str(),
            CreerRdvPsy {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                jour: j_rdv,
                heure_debut: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                heure_fin: NaiveTime::from_hms_opt(15, 30, 0).unwrap(),
                kind: EntretienPsyKind::Individuel,
                motif: "interdit".into(),
                note_interne: String::new(),
                signale_famille: false,
                actor_user_id: cast.cpe.user_id,
            },
        )
        .await;
    if cpe_rdv.is_ok() {
        return Err("CPE ne doit pas planifier un RDV psy".into());
    }
    r.geste("CPE refusé sur planification RDV psy (attendu)");

    let dossier = ent
        .upsert_dossier_psy_for_actor(
            Role::Psy.as_str(),
            UpsertDossierPsy {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                axes_suivi: "anxiété scolaire · orientation 3e".into(),
                contacts_externes: "CMP Dr Pomfrey".into(),
                note_confidentielle: "hypothèse clinique confidentielle".into(),
                suivi_actif: true,
                signale_famille: true,
                actor_user_id: psy.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    r.geste("psy ouvre dossier structuré Harry (suivi actif, signalé famille)");

    let lily_dos = ent
        .get_dossier_psy_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Lily doit voir le dossier psy signalé")?;
    if !lily_dos.note_confidentielle.is_empty() || !lily_dos.contacts_externes.is_empty() {
        return Err("Lily ne doit pas voir note ni contacts du dossier psy".into());
    }
    if lily_dos.axes_suivi != "anxiété scolaire · orientation 3e" || !lily_dos.suivi_actif {
        return Err("Lily doit voir axes + suivi du dossier".into());
    }
    r.geste("Lily lit dossier (axes + suivi) sans note ni contacts");

    let dossier2 = ent
        .upsert_dossier_psy_for_actor(
            Role::Psy.as_str(),
            UpsertDossierPsy {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                axes_suivi: "anxiété scolaire · orientation 3e · confiance".into(),
                contacts_externes: "CMP Dr Pomfrey".into(),
                note_confidentielle: "évolution positive".into(),
                suivi_actif: true,
                signale_famille: false,
                actor_user_id: psy.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    if dossier2.id != dossier.id {
        return Err("upsert dossier psy doit conserver le même id".into());
    }
    let lily_dos_hidden = ent
        .get_dossier_psy_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if lily_dos_hidden.is_some() {
        return Err("Lily ne doit plus voir le dossier si non signalé".into());
    }
    r.geste("upsert dossier idempotent · Lily masquée si non signalé (attendu)");

    let cpe_dos = ent
        .get_dossier_psy_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            cast.eleve_lea_id,
        )
        .await;
    if cpe_dos.is_ok() {
        return Err("CPE ne doit pas lire le dossier psy".into());
    }
    let inf_dos = ent
        .upsert_dossier_psy_for_actor(
            Role::Infirmerie.as_str(),
            UpsertDossierPsy {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                axes_suivi: "interdit".into(),
                contacts_externes: String::new(),
                note_confidentielle: String::new(),
                suivi_actif: true,
                signale_famille: false,
                actor_user_id: inf.user_id,
            },
        )
        .await;
    if inf_dos.is_ok() {
        return Err("infirmière ne doit pas saisir un dossier psy".into());
    }
    r.geste("CPE bloque lecture dossier · infirmière refusée à la saisie (attendu)");

    // Remettre signal famille pour le courrier PDF
    let _dossier_sig = ent
        .upsert_dossier_psy_for_actor(
            Role::Psy.as_str(),
            UpsertDossierPsy {
                etablissement_id: cast.etablissement_id,
                eleve_id: cast.eleve_lea_id,
                axes_suivi: "anxiété scolaire · orientation 3e · confiance".into(),
                contacts_externes: "CMP Dr Pomfrey".into(),
                note_confidentielle: "évolution positive".into(),
                suivi_actif: true,
                signale_famille: true,
                actor_user_id: psy.user_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let pdf_psy = ent
        .courrier_orientation_psy_pdf_for_actor(
            cast.etablissement_id,
            psy.user_id,
            Role::Psy.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !pdf_psy.starts_with(b"%PDF-1.4") || !pdf_psy.windows(5).any(|w| w == b"%%EOF") {
        return Err("PDF courrier orientation invalide".into());
    }
    let pdf_txt = String::from_utf8_lossy(&pdf_psy);
    if pdf_txt.contains("CMP") || pdf_txt.contains("evolution positive") || pdf_txt.contains("évolution") {
        return Err("PDF courrier ne doit pas contenir note/contacts".into());
    }
    r.geste(format!(
        "psy génère courrier orientation PDF Harry ({} o)",
        pdf_psy.len()
    ));

    let lily_pdf = ent
        .courrier_orientation_psy_pdf_for_actor(
            cast.etablissement_id,
            cast.parent_lea.user_id,
            Role::Parent.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if !lily_pdf.starts_with(b"%PDF-1.4") {
        return Err("Lily doit télécharger le courrier signalé".into());
    }
    r.geste("Lily télécharge le courrier orientation (dossier signalé)");

    let cpe_pdf = ent
        .courrier_orientation_psy_pdf_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            cast.eleve_lea_id,
        )
        .await;
    if cpe_pdf.is_ok() {
        return Err("CPE ne doit pas télécharger le courrier psy".into());
    }
    r.geste("CPE refusé sur courrier orientation PDF (attendu)");

    let mails_psy = ent
        .envoyer_courrier_orientation_psy_for_actor(
            cast.etablissement_id,
            psy.user_id,
            Role::Psy.as_str(),
            cast.eleve_lea_id,
        )
        .await
        .map_err(|e| e.to_string())?;
    if mails_psy < 1 {
        return Err("envoi SMTP courrier psy attendu (≥1 parent)".into());
    }
    let outbox_psy = ent
        .count_mails_kind(cast.etablissement_id, "courrier_psy")
        .await
        .map_err(|e| e.to_string())?;
    if outbox_psy < 1 {
        return Err("outbox courrier_psy vide".into());
    }
    r.geste(format!(
        "psy envoie courrier orientation SMTP dry-run ({mails_psy} dest., outbox={outbox_psy})"
    ));
    let refuse_mail_cpe = ent
        .envoyer_courrier_orientation_psy_for_actor(
            cast.etablissement_id,
            cast.cpe.user_id,
            Role::Cpe.as_str(),
            cast.eleve_lea_id,
        )
        .await;
    if refuse_mail_cpe.is_ok() {
        return Err("CPE ne doit pas envoyer le courrier psy par mail".into());
    }
    r.geste("CPE refusé sur envoi SMTP courrier psy (attendu)");

    r.trou(
        "infirmerie: passages + fiche + PDF + PAI + ordonnances + S3 live disque OK — natives fiche/PAI/passages + moyennes + HTTP PUT si ENT_OBJECT_S3_HTTP=1",
    );
    r.trou(
        "psy: entretiens + RDV + dossier + courrier PDF + SMTP OK — lecture famille web/natives + live si ENT_MAIL_LIVE=1",
    );

    // Compta — facture + PDF + OGEC + SEPA + rappels OK
    r.trou("compta: facture + PDF + OGEC + SEPA + rappels + bourse (types/affectation/décision HTTP) + rapprochement CSV dry-run OK — live si ENT_BANK_LIVE=1");

    // EDT — exceptions + A/B OK
    r.trou("EDT: exceptions + A/B + Pronote + greedy + contraintes soft OK — CP-SAT hors scope");

    // Messagerie — modèles + archive + priorité/dossier
    r.trou(
        "messagerie: boîte staff + modèles + archive + priorité/dossier + PJ natives + push OK — live si ENT_PUSH_LIVE=1",
    );

    Ok(())
}

async fn explorer_seuils_et_signaux(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    r: &mut Rapport,
) -> Result<(), String> {
    println!("# volumes / seuils — ce que le métier ne cover pas encore");

    // Empile plusieurs absences sur un élève pour voir si un seuil existe.
    let cible = &cast.eleves[5]; // Paul
    let debut_annee = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
    for d in 0..12 {
        let j = debut_annee + Duration::days(d * 3);
        if j > jour {
            break;
        }
        // Motifs du référentiel uniquement (FK ent_absence_motif) — le lab l'a découvert en jouant.
        let motif = match d % 4 {
            0 => "maladie",
            1 => "familial",
            2 => "autre",
            _ => "sejour",
        };
        ent.declare_absence(DeclareAbsence {
            etablissement_id: cast.etablissement_id,
            eleve_id: cible.id,
            debut: j,
            fin: j,
            motif: motif.into(),
            actor_user_id: Some(cast.cpe.user_id.to_string()),
        })
        .await
        .map_err(|e| e.to_string())?;
    }
    let n = ent
        .count_absences_depuis(cast.etablissement_id, cible.id, debut_annee)
        .await
        .map_err(|e| e.to_string())?;
    r.geste(format!(
        "{} a {n} absences depuis {debut_annee} — compteur OK",
        cible.prenom
    ));

    let signal = ent
        .signal_absence_ouvert(cast.etablissement_id, cible.id, jour)
        .await
        .map_err(|e| e.to_string())?;
    if n >= 10 {
        let Some(sig) = signal else {
            return Err(format!(
                "seuil atteint ({n}) mais aucun signal VS ouvert pour {}",
                cible.prenom
            ));
        };
        r.geste(format!(
            "signal VS ouvert pour {} (compteur={}, seuil={})",
            cible.prenom, sig.compteur, sig.seuil
        ));
        let Some(fil_id) = sig.fil_alerte_id else {
            return Err(format!(
                "seuil atteint mais pas d'alerte famille (fil) pour {}",
                cible.prenom
            ));
        };
        r.geste(format!(
            "alerte famille ouverte (fil {}) pour {}",
            &fil_id.to_string()[..8],
            cible.prenom
        ));

        let fils_parent = ent
            .list_fils_for_actor(
                cast.etablissement_id,
                cible.parent.user_id,
                Role::Parent.as_str(),
                Some(cible.id),
                false,
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        if fils_parent.iter().all(|f| f.id != fil_id) {
            return Err("parent doit voir le fil d'alerte seuil".into());
        }
        if !fils_parent
            .iter()
            .any(|f| f.id == fil_id && f.sujet.contains("seuil"))
        {
            return Err("sujet fil alerte doit mentionner le seuil".into());
        }
        r.geste(format!(
            "parent de {} lit le fil d'alerte seuil",
            cible.prenom
        ));

        // Idempotence : une absence de plus ne crée pas un 2ᵉ fil.
        let j_extra = debut_annee + Duration::days(40);
        if j_extra <= jour {
            ent.declare_absence(DeclareAbsence {
                etablissement_id: cast.etablissement_id,
                eleve_id: cible.id,
                debut: j_extra,
                fin: j_extra,
                motif: "maladie".into(),
                actor_user_id: Some(cast.cpe.user_id.to_string()),
            })
            .await
            .map_err(|e| e.to_string())?;
            let sig2 = ent
                .signal_absence_ouvert(cast.etablissement_id, cible.id, jour)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("signal doit rester ouvert")?;
            if sig2.fil_alerte_id != Some(fil_id) {
                return Err("alerte famille doit rester le même fil (idempotent)".into());
            }
            r.geste("alerte famille idempotente (pas de 2e fil)");
        }

        let file = ent
            .list_signaux_absences_ouverts(cast.etablissement_id)
            .await
            .map_err(|e| e.to_string())?;
        if !file.iter().any(|s| s.eleve_id == cible.id) {
            return Err("signal absent de la file VS".into());
        }
        r.geste(format!("file signaux VS: {} actif(s)", file.len()));

        let (comm_seuil, dossier_seuil, sig_lie) = ent
            .convoquer_commission_depuis_signal_for_actor(
                cast.etablissement_id,
                sig.id,
                Role::Cpe.as_str(),
                cast.cpe.user_id,
                jour + Duration::days(7),
                NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            )
            .await
            .map_err(|e| e.to_string())?;
        if sig_lie.statut != SignalAbsenceStatut::EnCours {
            return Err("signal doit passer en_cours via commission".into());
        }
        if sig_lie.commission_dossier_id != Some(dossier_seuil.id) {
            return Err("signal doit être lié au dossier commission".into());
        }
        if !comm_seuil.objet.contains("Absences répétées") {
            return Err(format!(
                "objet commission seuil invalide: {}",
                comm_seuil.objet
            ));
        }
        r.geste(format!(
            "CPE convoque commission depuis signal {} → dossier {}",
            &sig.id.to_string()[..8],
            &dossier_seuil.id.to_string()[..8]
        ));

        // Idempotence : 2ᵉ appel renvoie le même dossier.
        let (_, dossier2, _) = ent
            .convoquer_commission_depuis_signal_for_actor(
                cast.etablissement_id,
                sig.id,
                Role::Cpe.as_str(),
                cast.cpe.user_id,
                jour + Duration::days(7),
                NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            )
            .await
            .map_err(|e| e.to_string())?;
        if dossier2.id != dossier_seuil.id {
            return Err("re-convocation depuis signal doit être idempotente".into());
        }
        r.geste("commission depuis signal idempotente");

        let dossiers_famille = ent
            .list_commissions_eleve_for_actor(
                cast.etablissement_id,
                cible.parent.user_id,
                Role::Parent.as_str(),
                cible.id,
            )
            .await
            .map_err(|e| e.to_string())?;
        if !dossiers_famille.iter().any(|d| d.id == dossier_seuil.id) {
            return Err("parent doit voir la convocation commission seuil".into());
        }
        r.geste(format!(
            "{} lit convocation commission absences (signalée famille)",
            cible.parent.email
        ));

        let decide_seuil = ent
            .decider_commission_dossier_for_actor(
                Role::Cpe.as_str(),
                DeciderCommissionDossier {
                    etablissement_id: cast.etablissement_id,
                    dossier_id: dossier_seuil.id,
                    decision: DecisionCommission::Avertissement,
                    decision_texte: "Avertissement absences répétées — suivi CPE".into(),
                    points_retires: 2,
                    actor_user_id: cast.cpe.user_id,
                },
            )
            .await
            .map_err(|e| e.to_string())?;
        if decide_seuil.decision != Some(DecisionCommission::Avertissement)
            || decide_seuil.points_retires != 2
        {
            return Err("décision commission seuil invalide".into());
        }
        r.geste("CPE décide commission seuil → avertissement (−2 pts)");

        let clos = ent
            .clore_signal_absence_for_actor(
                cast.etablissement_id,
                sig.id,
                cast.cpe.user_id,
                Role::Cpe.as_str(),
                "Commission tenue + avertissement absences".into(),
            )
            .await
            .map_err(|e| e.to_string())?;
        if clos.statut != SignalAbsenceStatut::Clos {
            return Err("signal doit être clos".into());
        }
        if clos.commission_dossier_id != Some(dossier_seuil.id) {
            return Err("lien commission doit survivre à la clôture".into());
        }
        let file_apres = ent
            .list_signaux_absences_ouverts(cast.etablissement_id)
            .await
            .map_err(|e| e.to_string())?;
        if file_apres.iter().any(|s| s.id == sig.id) {
            return Err("signal clos ne doit plus être dans la file active".into());
        }
        let refuse_prof = ent
            .convoquer_commission_depuis_signal_for_actor(
                cast.etablissement_id,
                Uuid::new_v4(),
                Role::Prof.as_str(),
                cast.prof.user_id,
                jour,
                NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            )
            .await;
        if refuse_prof.is_ok() {
            return Err("prof ne doit pas gérer les signaux VS".into());
        }
        r.geste(format!(
            "CPE clôture signal {} lié commission · Rogue refusé",
            &sig.id.to_string()[..8]
        ));
        let mails_seuil = ent
            .count_mails_kind(cast.etablissement_id, "absence_seuil")
            .await
            .map_err(|e| e.to_string())?;
        if mails_seuil < 1 {
            return Err("SMTP seuil absences attendu en outbox".into());
        }
        r.geste(format!(
            "SMTP seuil absences dry-run → {mails_seuil} mail(s) outbox"
        ));
        r.trou(
            "seuil absences: alerte + SMTP + commission liée + décision OK — UI signal→commission + lecture famille — live SMS/push optionnels",
        );
    }

    r.geste(
        "cantine opérationnelle: grille + activités HTTP + dispense + volumes + badge + hist famille + facture badge OK — UI /app/cantine + dossier",
    );

    Ok(())
}
