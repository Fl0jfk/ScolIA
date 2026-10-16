//! Seed labo local — école dense multi-rôles pour exploration autonome.
//! **Uniquement** via Postgres loopback (`PgEnt::connect` déjà gardé).
//!
//! Le cast n'est pas un script figé : lots de personas, flags parent divergents,
//! classes mixtes. Le runner `ent-lab` fait jouer ces vies et signale les trous.

use crate::foyer_repo::CreateLien;
use crate::matiere_repo::CreateMatiere;
use crate::note_repo::SaisirNote;
use crate::service::{EntDbError, PgEnt};
use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Utc};
use domain_eleve::{
    annee_scolaire_de, annees_continuum, debut_annee_scolaire, today_utc, CreateEleve,
    CreerAlerteCpe, CreerCarnetEntree, CreerDevoir, CreerNotePeriode, CreerPassageInfirmerie,
    AbsenceStatut, AffecterEleveInternat, AjouterDossierConseil, CarnetCategorie, ChangeRegime,
    ChoisirOptionN1, ConvoquerCommission, CreerCahierTexte, CreerChambreInternat,
    CreerCommissionVs, CreerConseilClasse, CreerFacture, DeciderDossierConseil,
    CreerInfoCompChamp, CreerMandatSepa, CreerRdvPsy, CreerSuiviInternat, DeclarerIncident,
    DeclareAbsence, DemanderGrant, DemanderSortieExceptionnelle, DocConfidentialite, DocTiroir,
    EcheanceCmd, EncaisserFacture,
    EntretienPsyKind, FactureStatut, MandatSepaStatut,
    AlerteCpeSeverite, AlerteCpeStatut, DispenseVsMotif, DispenseVsStatut, PoserDispenseVs,
    UpsertAutorisationSortie,
    LigneFactureCmd, MonteeRapide, MoyenPaiement, OptionAnneeStatut, OptionNature,
    NotePeriodeStatut, OuvrirCampagneReinscription, OuvrirFil, PassageInfirmerieMotif,
    RepondreFil,
    SortieCreneau,
    PointerPresenceInternat, PoserEcheancier,
    PrononcerSanction, PublierDocASigner, RepasActiviteKind, SaisirAppreciation, UpsertDossierPsy,
    UpsertFicheSante, UpsertInfoCompValeur, UpsertMenuCantine, UpsertPai, UpsertQuotientFoyer,
    UpsertInscriptionSuivi, DeciderCommissionDossier, DecisionCommission, POINTS_VS_DEPART,
    PoserDispenseRepas, DispenseSource,
};
use domain_eleve::Devoir;
use ent_auth::{AuthError, Role};
use uuid::Uuid;

/// Mot de passe unique du cast labo (local only).
pub const LABO_PASSWORD: &str = "DevLocalPass1!";

#[derive(Debug, Clone)]
pub struct LaboPersona {
    pub user_id: Uuid,
    pub email: String,
    pub role: Role,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct LaboEleve {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: String,
    pub parent: LaboPersona,
    /// Compte face élève (Role::Eleve) s'il a été seedé.
    pub compte: Option<LaboPersona>,
    pub foyer_id: Uuid,
    pub peut_justifier: bool,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
}

/// École jetable + personas. Les champs `*_lea` / `*_tom` / `prof` restent
/// des alias vers le début du cast (compat scénarios), le vrai stock est en vecteurs.
#[derive(Debug, Clone)]
pub struct LaboCast {
    pub etablissement_id: Uuid,
    pub admin: LaboPersona,
    pub direction: LaboPersona,
    pub administratif: LaboPersona,
    pub compta: LaboPersona,
    pub prof: LaboPersona,
    pub cpe: LaboPersona,
    pub parent_lea: LaboPersona,
    pub parent_tom: LaboPersona,
    pub eleve_lea_id: Uuid,
    pub eleve_tom_id: Uuid,
    pub foyer_lea_id: Uuid,
    pub foyer_tom_id: Uuid,
    pub directions: Vec<LaboPersona>,
    pub administratifs: Vec<LaboPersona>,
    pub comptas: Vec<LaboPersona>,
    pub profs: Vec<LaboPersona>,
    pub cpes: Vec<LaboPersona>,
    pub infirmieres: Vec<LaboPersona>,
    pub psys: Vec<LaboPersona>,
    pub eleves: Vec<LaboEleve>,
    /// Créneau Rogue / 4B / Potions (jour courant) — pour appel métier.
    pub creneau_rogue_4b: Uuid,
    /// Créneau McGonagall / 5A (jour courant).
    pub creneau_mcg_5a: Uuid,
    /// Molly Weasley — parent commun Ron + Ginny (après seed audit).
    pub parent_weasley: Option<LaboPersona>,
    pub eleve_ron_id: Option<Uuid>,
    pub eleve_ginny_id: Option<Uuid>,
}

/// Fiches élèves seed — cast Harry Potter (parlant en labo / UI).
/// Flags: (peut_justifier, peut_voir_notes, peut_voir_vs) sur le lien parent.
/// Index 0 = « Harry » (alias code `eleve_lea`), 1 = « Draco » (`eleve_tom`).
const ELEVES_SEED: &[(&str, &str, &str, bool, bool, bool)] = &[
    ("Harry", "Potter", "4B", true, true, true),
    ("Draco", "Malefoy", "5A", false, true, false),
    ("Hermione", "Granger", "4B", true, true, true),
    ("Ron", "Weasley", "5A", true, false, true),
    ("Neville", "Londubat", "3C", false, true, true),
    ("Luna", "Lovegood", "6A", true, true, false),
    ("Ginny", "Weasley", "4B", true, false, false),
    ("Cho", "Chang", "5A", false, false, true),
    ("Cedric", "Diggory", "3C", true, true, true),
    ("Seamus", "Finnigan", "6A", true, true, true),
];

/// Display names staff (index dans chaque lot).
const PROFS_SEED: &[&str] = &[
    "Severus Rogue",
    "Minerva McGonagall",
    "Filius Flitwick",
    "Rubeus Hagrid",
    "Pomona Chourave",
    "Horace Slughorn",
    "Remus Lupin",
    "Gilderoy Lockhart",
];

impl PgEnt {
    /// Crée un établissement jetable + école dense qui se teste entre elle.
    pub async fn seed_labo_cast(&self) -> Result<LaboCast, EntDbError> {
        let etab = Uuid::new_v4();
        let suffix = etab.simple().to_string();
        let short = &suffix[..8];

        let admin = self
            .register_login(
                &format!("admin-{short}@ent.lab"),
                Role::Admin,
                "Albus Dumbledore",
                etab,
            )
            .await?;

        let mut directions = Vec::new();
        for (i, name) in ["Albus Dumbledore", "Minerva McGonagall"]
            .iter()
            .enumerate()
        {
            let n = i + 1;
            directions.push(
                self.register_login(
                    &format!("direction{n}-{short}@ent.lab"),
                    Role::Direction,
                    name,
                    etab,
                )
                .await?,
            );
        }

        let mut administratifs = Vec::new();
        for (i, name) in ["Argus Rusard", "Irma Pince"].iter().enumerate() {
            let n = i + 1;
            administratifs.push(
                self.register_login(
                    &format!("admini{n}-{short}@ent.lab"),
                    Role::Administratif,
                    name,
                    etab,
                )
                .await?,
            );
        }

        let mut comptas = Vec::new();
        for (i, name) in ["Horace Slughorn", "Lucius Malefoy"].iter().enumerate() {
            let n = i + 1;
            comptas.push(
                self.register_login(
                    &format!("compta{n}-{short}@ent.lab"),
                    Role::Compta,
                    name,
                    etab,
                )
                .await?,
            );
        }

        let mut profs = Vec::new();
        for (i, name) in PROFS_SEED.iter().enumerate() {
            let n = i + 1;
            profs.push(
                self.register_login(
                    &format!("prof{n}-{short}@ent.lab"),
                    Role::Prof,
                    name,
                    etab,
                )
                .await?,
            );
        }

        let mut cpes = Vec::new();
        for (i, name) in ["Remus Lupin", "Severus Rogue"].iter().enumerate() {
            let n = i + 1;
            cpes.push(
                self.register_login(
                    &format!("cpe{n}-{short}@ent.lab"),
                    Role::Cpe,
                    name,
                    etab,
                )
                .await?,
            );
        }

        let mut infirmieres = Vec::new();
        for (i, name) in ["Poppy Pomfresh", "Pomona Chourave"].iter().enumerate() {
            let n = i + 1;
            infirmieres.push(
                self.register_login(
                    &format!("infirmiere{n}-{short}@ent.lab"),
                    Role::Infirmerie,
                    name,
                    etab,
                )
                .await?,
            );
        }

        let mut psys = Vec::new();
        for (i, name) in ["Sybille Trelawney", "Firenze"].iter().enumerate() {
            let n = i + 1;
            psys.push(
                self.register_login(
                    &format!("psy{n}-{short}@ent.lab"),
                    Role::Psy,
                    name,
                    etab,
                )
                .await?,
            );
        }

        let entree = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();

        // Référentiel repas + type bourse (configurable établissement).
        use domain_eleve::{
            CreerBourseType, CreerRepasActivite, RepasActiviteKind,
        };
        let act_midi = self
            .creer_repas_activite(CreerRepasActivite {
                etablissement_id: etab,
                code: "MIDI".into(),
                libelle: "Repas midi".into(),
                kind: RepasActiviteKind::Midi,
                actor_user_id: Some(admin.user_id.to_string()),
            })
            .await?;
        let _act_etude = self
            .creer_repas_activite(CreerRepasActivite {
                etablissement_id: etab,
                code: "ETUDE".into(),
                libelle: "Étude du soir".into(),
                kind: RepasActiviteKind::Etude,
                actor_user_id: Some(admin.user_id.to_string()),
            })
            .await?;
        let bourse_nat = self
            .creer_bourse_type(CreerBourseType {
                etablissement_id: etab,
                code: "BN".into(),
                libelle: "Bourse nationale".into(),
                actor_user_id: Some(admin.user_id.to_string()),
            })
            .await?;

        // Référentiels VS incidents / sanctions (CONFIGURABLE établissement).
        use domain_eleve::CreerReferentielVs;
        let _inc_pert = self
            .creer_incident_type(CreerReferentielVs {
                etablissement_id: etab,
                code: "perturbation".into(),
                libelle: "Perturbation de cours".into(),
                actor_user_id: Some(admin.user_id.to_string()),
            })
            .await?;
        let _san_ret = self
            .creer_sanction_type(CreerReferentielVs {
                etablissement_id: etab,
                code: "retenue".into(),
                libelle: "Retenue".into(),
                actor_user_id: Some(admin.user_id.to_string()),
            })
            .await?;
        let _san_aver = self
            .creer_sanction_type(CreerReferentielVs {
                etablissement_id: etab,
                code: "avertissement".into(),
                libelle: "Avertissement".into(),
                actor_user_id: Some(admin.user_id.to_string()),
            })
            .await?;
        let _motif_devoirs = self
            .creer_sanction_motif(CreerReferentielVs {
                etablissement_id: etab,
                code: "devoirs".into(),
                libelle: "Devoirs non faits".into(),
                actor_user_id: Some(admin.user_id.to_string()),
            })
            .await?;

        let mut eleves = Vec::with_capacity(ELEVES_SEED.len());
        // Molly Weasley — un compte parent + un foyer pour Ron et Ginny (cast Weasley).
        let mut weasley_parent_foyer: Option<(LaboPersona, Uuid)> = None;

        for (idx, (prenom, nom, classe, peut_justifier, peut_voir_notes, peut_voir_vs)) in
            ELEVES_SEED.iter().enumerate()
        {
            let parent_display = match *prenom {
                "Harry" => "Lily Potter",
                "Draco" => "Narcissa Malefoy",
                "Hermione" => "Parent Granger",
                "Ron" | "Ginny" => "Molly Weasley",
                "Neville" => "Parent Londubat",
                "Luna" => "Parent Lovegood",
                "Cho" => "Parent Chang",
                "Cedric" => "Parent Diggory",
                "Seamus" => "Parent Finnigan",
                _ => "Parent",
            };
            let parent = if *prenom == "Ginny" {
                weasley_parent_foyer
                    .as_ref()
                    .map(|(p, _)| p.clone())
                    .expect("Ron seedé avant Ginny dans ELEVES_SEED")
            } else {
                let email = if *prenom == "Ron" {
                    format!("weasley-{short}@ent.lab")
                } else {
                    format!("parent{idx}-{short}@ent.lab")
                };
                let p = self
                    .register_login(&email, Role::Parent, parent_display, etab)
                    .await?;
                // Mobile lab (SMS dry-run) — unique par parent.
                let _ = self
                    .set_user_telephone(
                        p.user_id,
                        &format!("06{:08}", 12_000_000 + idx as i32),
                    )
                    .await;
                p
            };

            let naissance = NaiveDate::from_ymd_opt(2012 + (idx as i32 % 4), 3 + (idx as u32), 10)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2013, 5, 1).unwrap());
            let mef = match *classe {
                "6A" => "MEF6",
                "5A" => "MEF5",
                "4B" => "MEF4",
                "3C" => "MEF3",
                _ => "MEFX",
            };
            let eleve = self
                .create_eleve(CreateEleve {
                    etablissement_id: etab,
                    nom: (*nom).into(),
                    prenom: (*prenom).into(),
                    ine: Some(format!("INE{short}{idx:02}")),
                    sexe: Some(
                        match *prenom {
                            "Harry" | "Draco" | "Ron" | "Neville" | "Cedric" | "Seamus" => "M",
                            _ => "F",
                        }
                        .into(),
                    ),
                    date_naissance: Some(naissance),
                    numero_badge: Some(format!("B{short}{idx:02}")),
                    redoublant: idx == 6,
                    classe: Some((*classe).into()),
                    regime: Some(if idx % 2 == 0 { "DP" } else { "EXT" }.into()),
                    mef: Some(mef.into()),
                    site: Some("principal".into()),
                    date_entree: Some(entree),
                    date_sortie: None,
                    motif_sortie: None,
                    actor_user_id: Some(admin.user_id.to_string()),
                })
                .await?;

            // Continuité : snapshot N (année courante) depuis la projection.
            self.sync_scolarite_courante(
                etab,
                eleve.id,
                entree,
                Some(admin.user_id.to_string()),
            )
            .await?;

            // Options : académique + interne pour les premiers dossiers.
            if idx < 4 {
                use domain_eleve::{AjouterOption, OptionNature};
                self.ajouter_option(AjouterOption {
                    etablissement_id: etab,
                    eleve_id: eleve.id,
                    nature: OptionNature::Academique,
                    code: "LV1_ANG".into(),
                    libelle: "Anglais LV1".into(),
                    date_debut: entree,
                    actor_user_id: Some(admin.user_id.to_string()),
                })
                .await?;
                self.ajouter_option(AjouterOption {
                    etablissement_id: etab,
                    eleve_id: eleve.id,
                    nature: OptionNature::Interne,
                    code: "THEATRE".into(),
                    libelle: "Théâtre".into(),
                    date_debut: entree,
                    actor_user_id: Some(admin.user_id.to_string()),
                })
                .await?;
            }

            let foyer = if *prenom == "Ginny" {
                weasley_parent_foyer
                    .as_ref()
                    .map(|(_, f)| *f)
                    .expect("foyer Weasley")
            } else {
                let f = if *prenom == "Ron" {
                    self.create_foyer(etab, Some("Foyer Weasley")).await?
                } else {
                    self.create_foyer(etab, Some(&format!("Foyer {nom}")))
                        .await?
                };
                if *prenom == "Ron" {
                    weasley_parent_foyer = Some((parent.clone(), f));
                }
                f
            };

            self.link_parent(CreateLien {
                etablissement_id: etab,
                eleve_id: eleve.id,
                user_id: parent.user_id,
                foyer_id: Some(foyer),
                lien: if idx % 2 == 0 { "mère" } else { "père" }.into(),
                peut_voir_notes: *peut_voir_notes,
                peut_voir_vs: *peut_voir_vs,
                peut_justifier_absence: *peut_justifier,
                peut_autoriser_sorties: *peut_justifier,
                est_payeur: true,
                part_paiement: Some(100),
            })
            .await?;

            // Premiers élèves : compte face élève lié (les autres restent sans → lab découvre).
            let compte = if idx < 5 {
                let c = self
                    .register_login(
                        &format!("eleve{idx}-{short}@ent.lab"),
                        Role::Eleve,
                        &format!("Élève {prenom}"),
                        etab,
                    )
                    .await?;
                self.link_eleve_compte(etab, eleve.id, c.user_id).await?;
                Some(c)
            } else {
                None
            };

            // Harry (idx 0) : PAP + document restreint — lab prouve le grant.
            if idx == 0 {
                use domain_eleve::{
                    AjouterDispositif, DeposerDocument, DispositifKind, DocConfidentialite,
                    DocTiroir,
                };
                let pap = self
                    .deposer_document(DeposerDocument {
                        etablissement_id: etab,
                        eleve_id: eleve.id,
                        tiroir: DocTiroir::Scolaire,
                        titre: "PAP Harry Potter".into(),
                        code: None,
                        mime_type: Some("application/pdf".into()),
                        storage_key: Some(format!("lab/{short}/pap-harry.pdf")),
                        confidentialite: DocConfidentialite::Restreint,
                        actor_user_id: admin.user_id,
                    })
                    .await?;
                self.ajouter_dispositif(AjouterDispositif {
                    etablissement_id: etab,
                    eleve_id: eleve.id,
                    kind: DispositifKind::Pap,
                    libelle: Some("Plan d'accompagnement personnalisé".into()),
                    document_id: Some(pap.id),
                    date_debut: entree,
                    actor_user_id: Some(admin.user_id.to_string()),
                })
                .await?;
                // Doc famille standard (parent peut voir sans grant).
                self.deposer_document(DeposerDocument {
                    etablissement_id: etab,
                    eleve_id: eleve.id,
                    tiroir: DocTiroir::Famille,
                    titre: "Attestation assurance".into(),
                    code: None,
                    mime_type: Some("application/pdf".into()),
                    storage_key: Some(format!("lab/{short}/assurance-harry.pdf")),
                    confidentialite: DocConfidentialite::Standard,
                    actor_user_id: admin.user_id,
                })
                .await?;

                // Grille midi lun–ven (DP) + bourse acceptée avec procuration.
                use domain_eleve::{
                    AffecterBourse, DeciderBourse, JoursMasque, PoserGrilleRepas,
                };
                self.poser_grille_repas(PoserGrilleRepas {
                    etablissement_id: etab,
                    eleve_id: eleve.id,
                    activite_id: act_midi.id,
                    jours: JoursMasque::lun_ven(),
                    date_debut: entree,
                    actor_user_id: Some(admin.user_id.to_string()),
                })
                .await?;
                let bourse = self
                    .affecter_bourse(AffecterBourse {
                        etablissement_id: etab,
                        eleve_id: eleve.id,
                        bourse_type_id: bourse_nat.id,
                        echelon: "2".into(),
                        avec_procuration: true,
                        date_debut: entree,
                        actor_user_id: Some(admin.user_id.to_string()),
                    })
                    .await?;
                self.decider_bourse(DeciderBourse {
                    etablissement_id: etab,
                    bourse_id: bourse.id,
                    accepter: true,
                    actor_user_id: Some(admin.user_id.to_string()),
                })
                .await?;

                // N-1 déjà dans l'établissement (continuité fiche, pas year-jump).
                use domain_eleve::{
                    EnregistrerScolariteAnnee, ScolariteAnneeStatut, ScolariteSource,
                };
                self.enregistrer_scolarite_annee(EnregistrerScolariteAnnee {
                    etablissement_id: etab,
                    eleve_id: eleve.id,
                    annee_scolaire: "2025-2026".into(),
                    niveau: Some("5e".into()),
                    mef: Some("MEF5".into()),
                    classe: Some("5A".into()),
                    regime: Some("DP".into()),
                    statut: ScolariteAnneeStatut::Historique,
                    source: ScolariteSource::Import,
                    passage_juridique: true,
                    redoublement_possible: false,
                    actor_user_id: Some(admin.user_id.to_string()),
                })
                .await?;

                // Mot carnet visible famille (Lupin) — Lily pourra signer.
                use domain_eleve::{CarnetCategorie, CreerCarnetEntree};
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: eleve.id,
                    date_entree: entree,
                    categorie: CarnetCategorie::Correspondance,
                    titre: "Travail à revoir".into(),
                    corps: "Harry doit rattraper le chapitre Potions avant vendredi.".into(),
                    visible_famille: true,
                    actor_user_id: cpes[0].user_id,
                })
                .await?;
            }

            eleves.push(LaboEleve {
                id: eleve.id,
                nom: (*nom).into(),
                prenom: (*prenom).into(),
                classe: (*classe).into(),
                parent,
                compte,
                foyer_id: foyer,
                peut_justifier: *peut_justifier,
                peut_voir_notes: *peut_voir_notes,
                peut_voir_vs: *peut_voir_vs,
            });
        }

        let lea = eleves[0].clone();
        let tom = eleves[1].clone();

        // EDT du jour : Rogue → 4B Potions ; McGonagall → 5A Transfiguration.
        let jour = today_utc();
        let creneau_rogue_4b = self
            .seed_creneau_du_jour(
                etab,
                "4B",
                "Potions",
                profs[0].user_id,
                jour,
                NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                Some("Donjons"),
            )
            .await?
            .id;
        let creneau_mcg_5a = self
            .seed_creneau_du_jour(
                etab,
                "5A",
                "Transfiguration",
                profs[1].user_id,
                jour,
                NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                Some("1er étage"),
            )
            .await?
            .id;

        self.seed_labo_journee_audit(
            etab,
            &lea,
            &tom,
            &mut eleves,
            &profs,
            &cpes,
            &infirmieres,
            &psys,
            &admin,
            &comptas[0],
            &administratifs[0],
            jour,
        )
        .await?;

        let parent_weasley = weasley_parent_foyer
            .map(|(p, _)| p)
            .or_else(|| labo_eleve_prenom(&eleves, "Ron").map(|r| r.parent.clone()));
        let eleve_ron_id = labo_eleve_prenom(&eleves, "Ron").map(|r| r.id);
        let eleve_ginny_id = labo_eleve_prenom(&eleves, "Ginny").map(|r| r.id);

        Ok(LaboCast {
            etablissement_id: etab,
            admin,
            direction: directions[0].clone(),
            administratif: administratifs[0].clone(),
            compta: comptas[0].clone(),
            prof: profs[0].clone(),
            cpe: cpes[0].clone(),
            parent_lea: lea.parent.clone(),
            parent_tom: tom.parent.clone(),
            eleve_lea_id: lea.id,
            eleve_tom_id: tom.id,
            foyer_lea_id: lea.foyer_id,
            foyer_tom_id: tom.foyer_id,
            directions,
            administratifs,
            comptas,
            profs,
            cpes,
            infirmieres,
            psys,
            eleves,
            creneau_rogue_4b,
            creneau_mcg_5a,
            parent_weasley,
            eleve_ron_id,
            eleve_ginny_id,
        })
    }

    async fn register_login(
        &self,
        email: &str,
        role: Role,
        display: &str,
        etab: Uuid,
    ) -> Result<LaboPersona, EntDbError> {
        match self
            .register_user(email, LABO_PASSWORD, Some(display), etab, role.as_str())
            .await
        {
            Ok(u) => {
                let session = self.login(email, LABO_PASSWORD, etab).await?;
                Ok(LaboPersona {
                    user_id: u.id,
                    email: email.to_string(),
                    role,
                    token: session.token,
                })
            }
            Err(EntDbError::Auth(AuthError::EmailTaken)) => {
                let session = self.login(email, LABO_PASSWORD, etab).await?;
                Ok(LaboPersona {
                    user_id: session.user_id,
                    email: email.to_string(),
                    role,
                    token: session.token,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Matières, notes, réinscription, santé, VS — journée non vide pour bots post-bootstrap.
    ///
    /// Lots audit W–Z (idempotents) : parent multi-enfants Weasley, factures Harry (reste à payer),
    /// dispense EPS + sorties permanentes, alerte CPE ouverte, cahier Potions 4B, quotient Harry,
    /// internat Neville (INT requis), psy Harry, fil CPE→parent ; docs/PAI/carnet non signés ;
    /// file inscription « pièces », outbox mail/SMS, diversité cast Hermione (note / absence / VS) ;
    /// lot A (Bot AA-A) : Luna, Cho, Seamus, mandat SEPA Harry, Molly mes_enfants Ron+Ginny ;
    /// lot B (Bot AB-A) : note Neville, absence Cedric J-1 à traiter, CHARTE_LAB, permis points Harry, dispense repas Harry ;
    /// lot C (Bot AC-A) : note Seamus, carnet Cedric, rappel facture Harry, cahier 5A, photo trombi Neville ;
    /// lot D (Bot AD-A) : carnet Luna, passage infirmerie Hermione, sortie except. Harry jour J, facture Draco ;
    /// lot E (Bot AE-A) : note Cho, absence NJ Ron (Molly), note Ginny (staff / pas notes parent), alerte CPE urgente Harry, PDF JUSTIF non vide ;
    /// lot F (Bot AF-A) : carnet Seamus (famille), absence NJ Luna, dossier conseil Harry, photo trombi Ron, renfort menus lun–ven publiés ;
    /// lot G (Bot AG-A) : absence NJ Seamus, note Cedric, RDV psy Harry J+10, carnet Draco, ré-ensure CHARTE_LAB + REGLEMENT_LAB ;
    /// lot H (Bot AH-A) : carnet Hermione (famille), absence NJ Neville, rappel SMS dry-run Harry si API, photo trombi Ginny, Molly mes_enfants Ron+Ginny ;
    /// lot I (Bot AI-A) : absence NJ Cho, carnet Luna (si pas déjà AH-A), renfort justif Harry jour J a_traiter, note Cedric (autre matière), photo trombi Seamus ;
    /// lot J (Bot AJ-A) : absence NJ Ginny (staff / Molly sans VS Ginny), note Ron (staff / Molly sans notes Ron), ré-ensure dispense EPS Harry, photo trombi Cedric, ré-ensure facture Draco ;
    /// lot K (Bot AK-A) : ré-ensure lot J Ginny/Ron, note Hermione (autre matière), photo trombi Cho, 2ᵉ message fil CPE Harry, ré-ensure facture LAB-HARRY-2 (reste > 0).
    /// lot L (Bot AL-A) : note Luna (suppl.), absence NJ Seamus, photo trombi Luna, dossier commission Harry ouvert (decision null), Molly mes_enfants Ron+Ginny.
    /// lot M (Bot AM-A) : carnet Cedric (Diggory) si pas d’entrée récente famille, note Cho, photo trombi Cedric, PAI Harry non signé, facture LAB-DRACO.
    /// lot N (Bot AN-A) : note Seamus (Finnigan), absence NJ Luna (Lovegood), photo trombi Hermione, ré-ensure CHARTE_LAB + REGLEMENT_LAB Harry, Molly mes_enfants Ron+Ginny.
    /// lot O (Bot AO-A) : note Neville, carnet Ginny (famille / visible_famille), photo trombi Seamus (Finnigan),
    /// renfort justif Harry jour J + PJ, renfort menus lun–ven publiés.
    /// lot P (Bot AP-A) : absence NJ Hermione (Granger), carnet Cedric (Diggory) récent famille,
    /// photo trombi Luna (Lovegood), ré-ensure bulletin T0 publié Harry, ré-ensure mandat SEPA foyer Harry.
    /// lot Q (Bot AQ-A) : note Draco, carnet Ron (famille), photo trombi Cho, alerte CPE urgente Harry,
    /// ré-ensure facture LAB-HARRY* (reste à payer > 0 sur au moins une).
    /// lot R (Bot AR-A) : devoir Hermione (titre AR), note Ginny, dispense ou PAI tip Neville,
    /// photo trombi Cedric, ré-ensure commission VS jour J (sans absence jour J supplémentaire).
    /// lot S (Bot AS-A) : note Luna, carnet Seamus (visible_famille), photo trombi Ron si vide,
    /// devoir Harry (titre unique AS) ; ré-ensure absence Harry jour J + commission Harry jour J (sans wipe).
    /// lot T (Bot AT-A) : note Cho, carnet Hermione (famille), photo trombi Ginny si vide, devoir Draco ;
    /// ré-ensure absence Harry jour J + commission Harry jour J (sans wipe).
    /// lot U (Bot AU-A) : note Cedric, carnet Neville (famille), photo trombi Hermione si vide, devoir Luna ;
    /// ré-ensure absence Harry jour J + commission Harry jour J (sans wipe).
    /// lot V (Bot AV-A) : devoir Ginny, note Ron, photo trombi Seamus si vide, carnet Cho (famille) ;
    /// ré-ensure absence Harry jour J + commission Harry jour J (sans wipe).
    /// lot W (Bot AW-A) : note Hermione, carnet Draco (famille), photo trombi Luna si vide, devoir Cedric ;
    /// ré-ensure absence Harry jour J + commission Harry jour J (sans wipe).
    /// lot X (Bot AX-A) : note Neville, carnet Ginny (famille), photo trombi Harry si vide, devoir Ron ;
    /// ré-ensure absence Harry jour J + commission Harry jour J (sans wipe).
    /// lot Y (Bot AY-A) : note Seamus, devoir Hermione, photo trombi Draco si vide, note Cho (autre matière si besoin) ;
    /// ré-ensure absence Harry jour J + commission Harry jour J (sans wipe).
    async fn seed_labo_journee_audit(
        &self,
        etab: Uuid,
        harry: &LaboEleve,
        draco: &LaboEleve,
        eleves: &mut [LaboEleve],
        profs: &[LaboPersona],
        cpes: &[LaboPersona],
        infirmieres: &[LaboPersona],
        psys: &[LaboPersona],
        admin: &LaboPersona,
        compta: &LaboPersona,
        administratif: &LaboPersona,
        jour: NaiveDate,
    ) -> Result<(), EntDbError> {
        let actor_admin = admin.user_id.to_string();
        let prof_rogue = &profs[0];
        let prof_mcg = &profs[1];
        let cpe = &cpes[0];
        let inf = &infirmieres[0];

        let potions = self
            .ensure_matiere_lab(
                etab,
                "POTIONS",
                "Potions",
                Some("LAB:Potions".into()),
                &actor_admin,
            )
            .await?;
        let math = self
            .ensure_matiere_lab(
                etab,
                "MATH",
                "Mathématiques",
                Some("SIECLE:MATH".into()),
                &actor_admin,
            )
            .await?;
        let _fr = self
            .ensure_matiere_lab(
                etab,
                "FR",
                "Français",
                Some("SIECLE:FR".into()),
                &actor_admin,
            )
            .await?;
        let _hg = self
            .ensure_matiere_lab(
                etab,
                "HG",
                "Histoire-Géographie",
                Some("SIECLE:HG".into()),
                &actor_admin,
            )
            .await?;

        let debut_as = debut_annee_scolaire(jour);
        let fin_t1 = NaiveDate::from_ymd_opt(debut_as.year(), 12, 20)
            .unwrap_or_else(|| debut_as + Duration::days(110));
        let periode = self
            .ensure_note_periode_t1(etab, debut_as, fin_t1, &actor_admin)
            .await?;

        let (devoir, devoir_cree) = self
            .ensure_devoir_potions_lab(
                etab,
                periode.id,
                potions.id,
                &harry.classe,
                jour,
                prof_rogue.user_id,
            )
            .await?;

        const CAHIER_POTIONS_AUDIT_TITRE: &str = "Devoir maison Potions 4B (audit)";
        let cahier_4b = self
            .list_cahier_texte_for_actor(
                etab,
                prof_rogue.user_id,
                Role::Prof.as_str(),
                Some(harry.classe.clone()),
                None,
                None,
                None,
            )
            .await?;
        if !cahier_4b
            .iter()
            .any(|e| e.titre == CAHIER_POTIONS_AUDIT_TITRE && e.matiere_libelle == "Potions")
        {
            self.creer_cahier_texte_for_actor(
                Role::Prof.as_str(),
                CreerCahierTexte {
                    etablissement_id: etab,
                    classe: harry.classe.clone(),
                    matiere_libelle: "Potions".into(),
                    titre: CAHIER_POTIONS_AUDIT_TITRE.into(),
                    consigne: "Préparer la potion de croissance (ch.4) — à rendre pour la prochaine séance."
                        .into(),
                    pour_le: jour + Duration::days(7),
                    cree_le: jour,
                    actor_user_id: prof_rogue.user_id,
                },
            )
            .await?;
        }

        if devoir_cree {
            self.saisir_note(SaisirNote {
                etablissement_id: etab,
                eleve_id: harry.id,
                matiere_id: potions.id,
                valeur: 14.5,
                sur: 20.0,
                date_note: jour,
                auteur_user_id: prof_rogue.user_id,
                libelle: Some("DS Potions".into()),
                devoir_id: Some(devoir.id),
                periode_id: Some(periode.id),
            })
            .await?;

            self.saisir_note(SaisirNote {
                etablissement_id: etab,
                eleve_id: harry.id,
                matiere_id: potions.id,
                valeur: 16.0,
                sur: 20.0,
                date_note: jour - Duration::days(7),
                auteur_user_id: prof_rogue.user_id,
                libelle: Some("Interro chapitre 2".into()),
                devoir_id: None,
                periode_id: Some(periode.id),
            })
            .await?;
        }

        const DRACO_NOTE_LIB: &str = "Interro Transfiguration lab";
        if !self
            .notes_for_eleve(etab, draco.id)
            .await?
            .iter()
            .any(|n| n.libelle.as_deref() == Some(DRACO_NOTE_LIB))
        {
            self.saisir_note(SaisirNote {
                etablissement_id: etab,
                eleve_id: draco.id,
                matiere_id: math.id,
                valeur: 11.0,
                sur: 20.0,
                date_note: jour - Duration::days(1),
                auteur_user_id: prof_mcg.user_id,
                libelle: Some(DRACO_NOTE_LIB.into()),
                devoir_id: None,
                periode_id: Some(periode.id),
            })
            .await?;
        }

        let absence_draco = jour - Duration::days(2);
        if !self.eleve_est_absent(etab, draco.id, absence_draco).await? {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "defaut".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: draco.id,
                debut: absence_draco,
                fin: absence_draco,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }

        let passages = self
            .list_passages_infirmerie_for_actor(
                etab,
                inf.user_id,
                Role::Infirmerie.as_str(),
                Some(harry.id),
                None,
            )
            .await?;
        if !passages
            .iter()
            .any(|p| p.motif == PassageInfirmerieMotif::Malaise)
        {
            self.creer_passage_infirmerie_for_actor(
                Role::Infirmerie.as_str(),
                CreerPassageInfirmerie {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    jour,
                    motif: PassageInfirmerieMotif::Malaise,
                    note_soins: "Malaise léger en cours d'EPS — repos 20 min, hydratation, parent prévenu."
                        .into(),
                    visible_famille: true,
                    actor_user_id: inf.user_id,
                },
            )
            .await?;
        }

        let incident_types = self.list_incident_types(etab).await?;
        let sanction_types = self.list_sanction_types(etab).await?;
        let sanction_motifs = self.list_sanction_motifs(etab).await?;
        let sanctions_existantes = self
            .list_sanctions_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
            )
            .await?;
        const INCIDENT_DESC: &str = "Seed labo — perturbation en Potions.";
        let incidents_harry = self
            .list_incidents_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
            )
            .await?;
        let incident_harry = if let Some(i) = incidents_harry
            .iter()
            .find(|i| i.description.as_deref() == Some(INCIDENT_DESC))
        {
            Some(i.id)
        } else if let Some(typ) = incident_types
            .iter()
            .find(|t| t.code.eq_ignore_ascii_case("perturbation"))
            .or(incident_types.first())
        {
            let inc = self
                .declarer_incident_for_actor(
                    Role::Prof.as_str(),
                    DeclarerIncident {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        type_id: typ.id,
                        jour: jour - Duration::days(1),
                        description: Some(INCIDENT_DESC.into()),
                        actor_user_id: prof_rogue.user_id,
                    },
                )
                .await?;
            Some(inc.id)
        } else {
            None
        };

        if !sanctions_existantes
            .iter()
            .any(|s| sanction_types.iter().any(|t| t.id == s.type_id && t.code.eq_ignore_ascii_case("avertissement")))
        {
            if let Some(typ) = sanction_types
                .iter()
                .find(|t| t.code.eq_ignore_ascii_case("avertissement"))
            {
                let motif_id = sanction_motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("devoirs"))
                    .map(|m| m.id);
                self.prononcer_sanction(PrononcerSanction {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    type_id: typ.id,
                    motif_id,
                    incident_id: None,
                    jour: jour - Duration::days(1),
                    duree_minutes: None,
                    note: Some("Seed labo — avertissement après perturbation en Potions.".into()),
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let (Some(incident_id), Some(retenue)) = (
            incident_harry,
            sanction_types
                .iter()
                .find(|t| t.code.eq_ignore_ascii_case("retenue")),
        ) {
            if !sanctions_existantes
                .iter()
                .any(|s| s.type_id == retenue.id)
            {
                self.prononcer_sanction(PrononcerSanction {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    type_id: retenue.id,
                    motif_id: None,
                    incident_id: Some(incident_id),
                    jour: jour - Duration::days(1),
                    duree_minutes: Some(60),
                    note: Some("Seed labo — retenue liée à l'incident Potions.".into()),
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        let appreciations = self
            .list_appreciations_for_actor(
                etab,
                prof_rogue.user_id,
                Role::Prof.as_str(),
                harry.id,
                Some(periode.id),
            )
            .await?;
        if !appreciations
            .iter()
            .any(|a| a.matiere_id == Some(potions.id))
        {
            self.saisir_appreciation_for_actor(
                Role::Prof.as_str(),
                SaisirAppreciation {
                    etablissement_id: etab,
                    periode_id: periode.id,
                    eleve_id: harry.id,
                    matiere_id: Some(potions.id),
                    texte: "Harry progresse en Potions : rigueur et application notables au T1."
                        .into(),
                    actor_user_id: prof_rogue.user_id,
                },
            )
            .await?;
        }

        let (_, _, np1) = annees_continuum(jour);
        self.ouvrir_campagne_reinscription(OuvrirCampagneReinscription {
            etablissement_id: etab,
            annee_cible: np1,
            actor_user_id: Some(actor_admin.clone()),
        })
        .await?;

        if self
            .get_fiche_sante_for_actor(
                etab,
                inf.user_id,
                Role::Infirmerie.as_str(),
                harry.id,
            )
            .await?
            .is_none()
        {
            self.upsert_fiche_sante_for_actor(
                Role::Infirmerie.as_str(),
                UpsertFicheSante {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    allergies: "Arachides".into(),
                    traitements: "Ventoline si besoin".into(),
                    contre_indications: "Aspirine".into(),
                    pai_actif: true,
                    medecin_nom: Some("Dr Pomfresh".into()),
                    urgence_tel: Some("0601020304".into()),
                    note_confidentielle: "Asthma — usage interne infirmerie".into(),
                    actor_user_id: inf.user_id,
                },
            )
            .await?;
        }

        if self
            .get_pai_for_actor(
                etab,
                inf.user_id,
                Role::Infirmerie.as_str(),
                harry.id,
            )
            .await?
            .is_none()
        {
            self.upsert_pai_for_actor(
                Role::Infirmerie.as_str(),
                UpsertPai {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    motif: "Asthme".into(),
                    mesures_accueil: "Ventoline à l'infirmerie avant EPS".into(),
                    protocole_urgence: "Appeler le SAMU en cas de crise sévère".into(),
                    note_interne: "Antécédent hospitalisation 2024".into(),
                    date_debut: debut_as,
                    date_fin: NaiveDate::from_ymd_opt(debut_as.year() + 1, 7, 5),
                    actif: true,
                    actor_user_id: inf.user_id,
                },
            )
            .await?;
        }

        let absence_jour = jour - Duration::days(3);
        if !self.eleve_est_absent(etab, harry.id, absence_jour).await? {
            // Motif = code référentiel (global ou etab), pas un libellé libre.
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone());
            let motif_code = match motif_code {
                Some(c) => c,
                None => {
                    self.create_motif_absence_etab(
                        etab,
                        "maladie",
                        "Maladie",
                        Some(actor_admin.clone()),
                    )
                    .await?
                    .code
                }
            };
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_jour,
                fin: absence_jour,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }

        let annee = annee_scolaire_de(jour);
        let docs = self
            .list_docs_a_signer_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
                false,
            )
            .await?;
        if !docs.iter().any(|d| d.code == "REGLEMENT_LAB") {
            self.publier_doc_a_signer_for_actor(
                Role::Cpe.as_str(),
                PublierDocASigner {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    code: "REGLEMENT_LAB".into(),
                    titre: "Règlement intérieur — signature requise".into(),
                    corps: "Merci de prendre connaissance du règlement et de le signer en ligne.".into(),
                    annee_scolaire: annee.clone(),
                    echeance: Some(jour + Duration::days(30)),
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }
        if !docs.iter().any(|d| d.code == "CHARTE_LAB") {
            self.publier_doc_a_signer_for_actor(
                Role::Cpe.as_str(),
                PublierDocASigner {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    code: "CHARTE_LAB".into(),
                    titre: "Charte numérique — signature requise".into(),
                    corps: "Merci de lire et signer la charte d'usage des outils numériques.".into(),
                    annee_scolaire: annee.clone(),
                    echeance: Some(jour + Duration::days(21)),
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }

        const CARNET_AUDIT_TITRE: &str = "AUDIT: Rappel conduite";
        let carnet_harry = self
            .list_carnet_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
            )
            .await?;
        if !carnet_harry.iter().any(|c| c.titre == CARNET_AUDIT_TITRE) {
            self.creer_carnet_entree(CreerCarnetEntree {
                etablissement_id: etab,
                eleve_id: harry.id,
                date_entree: jour,
                categorie: CarnetCategorie::Information,
                titre: CARNET_AUDIT_TITRE.into(),
                corps: "Merci de rappeler à Harry les règles de courtoisie en classe — signature parent attendue."
                    .into(),
                visible_famille: true,
                actor_user_id: cpe.user_id,
            })
            .await?;
        }

        const ALERTE_CPE_AUDIT_TITRE: &str = "Retard répété — suivi CPE";
        let alertes_harry = self
            .list_alertes_cpe_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
            )
            .await?;
        if !alertes_harry.iter().any(|a| {
            a.titre == ALERTE_CPE_AUDIT_TITRE && a.statut == AlerteCpeStatut::Ouverte
        }) {
            self.creer_alerte_cpe_for_actor(
                Role::Cpe.as_str(),
                CreerAlerteCpe {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    titre: ALERTE_CPE_AUDIT_TITRE.into(),
                    detail: "Seed labo : alerte visible famille pour audit personas.".into(),
                    severite: "urgent".into(),
                    visible_famille: true,
                    jour,
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }

        let etab_tag = etab.simple().to_string();
        let etab_short = &etab_tag[..8.min(etab_tag.len())];
        let role_compta = Role::Compta.as_str();
        let role_admini = Role::Administratif.as_str();

        // --- Factures foyer Harry : 1 partiellement payée + 1 émise non soldée (reste à payer visible)
        let fac_numero = format!("LAB-HARRY-{etab_short}");
        let fac2_numero = format!("LAB-HARRY-2-{etab_short}");
        let factures_compta = self
            .list_factures_for_actor(etab, compta.user_id, role_compta)
            .await?;
        let mut facture_harry = if let Some(f) = factures_compta
            .iter()
            .find(|f| f.foyer_id == harry.foyer_id && f.numero == fac_numero)
        {
            f.clone()
        } else {
            let (f, _lignes) = self
                .creer_facture_for_actor(
                    role_compta,
                    CreerFacture {
                        etablissement_id: etab,
                        foyer_id: harry.foyer_id,
                        numero: fac_numero.clone(),
                        libelle: "Cantine — seed audit Harry".into(),
                        nature: None,
                        annee_scolaire: Some(annee_scolaire_de(jour)),
                        lignes: vec![LigneFactureCmd {
                            eleve_id: Some(harry.id),
                            libelle: "Repas midi (semaine labo)".into(),
                            quantite: 5,
                            montant_unitaire_centimes: 450,
                        }],
                        actor_user_id: compta.user_id,
                    },
                )
                .await?;
            f
        };
        if facture_harry.statut == FactureStatut::Brouillon {
            facture_harry = self
                .emettre_facture_for_actor(etab, facture_harry.id, compta.user_id, role_compta)
                .await?;
        }
        let (_fac, _lig, _encs, deja_encaisse, _, _, _) = self
            .get_facture_for_actor(etab, compta.user_id, role_compta, facture_harry.id)
            .await?;
        if deja_encaisse == 0 && facture_harry.total_centimes > 0 {
            let montant_partiel = (facture_harry.total_centimes * 2) / 5;
            let montant_partiel = montant_partiel.max(1);
            let _ = self
                .encaisser_facture_for_actor(
                    role_compta,
                    EncaisserFacture {
                        etablissement_id: etab,
                        facture_id: facture_harry.id,
                        montant_centimes: montant_partiel,
                        moyen: MoyenPaiement::Virement,
                        reference: Some("SEED-HARRY-AUDIT".into()),
                        actor_user_id: compta.user_id,
                    },
                )
                .await?;
        }
        let (facture_harry_2_id, emit_harry_2) = if let Some(f) = factures_compta
            .iter()
            .find(|f| f.foyer_id == harry.foyer_id && f.numero == fac2_numero)
        {
            (f.id, f.statut == FactureStatut::Brouillon)
        } else {
            let (f, _lignes) = self
                .creer_facture_for_actor(
                    role_compta,
                    CreerFacture {
                        etablissement_id: etab,
                        foyer_id: harry.foyer_id,
                        numero: fac2_numero.clone(),
                        libelle: "Activités extrascolaires — seed audit Lily/Harry".into(),
                        nature: None,
                        annee_scolaire: Some(annee_scolaire_de(jour)),
                        lignes: vec![LigneFactureCmd {
                            eleve_id: Some(harry.id),
                            libelle: "Quidditch club (trimestre)".into(),
                            quantite: 1,
                            montant_unitaire_centimes: 3_500,
                        }],
                        actor_user_id: compta.user_id,
                    },
                )
                .await?;
            (f.id, f.statut == FactureStatut::Brouillon)
        };
        if emit_harry_2 {
            let _ = self
                .emettre_facture_for_actor(etab, facture_harry_2_id, compta.user_id, role_compta)
                .await?;
        }
        let annee_quotient = annee_scolaire_de(jour);
        if self
            .get_quotient_foyer_for_actor(
                etab,
                compta.user_id,
                role_compta,
                harry.foyer_id,
                &annee_quotient,
            )
            .await?
            .is_none()
        {
            self.upsert_quotient_foyer_for_actor(
                role_compta,
                UpsertQuotientFoyer {
                    etablissement_id: etab,
                    foyer_id: harry.foyer_id,
                    annee_scolaire: annee_quotient,
                    quotient_centimes: 850_50,
                    parts_x100: 250,
                    actor_user_id: compta.user_id,
                },
            )
            .await?;
        }

        // --- Menus cantine publiés (semaine lun–ven) + passages badge midi Harry
        let debut_sem = debut_semaine_lundi(jour);
        let fin_sem = debut_sem + Duration::days(4);
        const MENUS_LAB: [(&str, &str, &str); 5] = [
            ("Velouté", "Gratin dauphinois", "Compote"),
            ("Salade", "Poulet rôti", "Yaourt"),
            ("Soupe", "Poisson pané", "Fruit"),
            ("Taboulé", "Steak haché", "Gâteau"),
            ("Carottes râpées", "Lasagnes", "Fromage"),
        ];
        for (i, menu) in MENUS_LAB.iter().enumerate() {
            let j = debut_sem + Duration::days(i as i64);
            let menus_jour = self
                .list_menus_cantine_for_actor(etab, administratif.user_id, role_admini, j, j)
                .await?;
            if !menus_jour.iter().any(|m| m.plat == menu.1) {
                self.upsert_menu_cantine_for_actor(
                    role_admini,
                    UpsertMenuCantine {
                        etablissement_id: etab,
                        jour: j,
                        entree: menu.0.into(),
                        plat: menu.1.into(),
                        dessert: menu.2.into(),
                        notes: if harry.classe == "4B" {
                            "Menu seed audit 4B".into()
                        } else {
                            String::new()
                        },
                        publie: true,
                        actor_user_id: administratif.user_id,
                    },
                )
                .await?;
            }
        }
        if let Some(eleve_h) = self.get_eleve(etab, harry.id).await? {
            if let Some(badge) = eleve_h.numero_badge.as_deref() {
                let passages = self
                    .list_passages_repas_eleve_for_actor(
                        etab,
                        role_compta,
                        compta.user_id,
                        harry.id,
                        debut_sem,
                        fin_sem,
                    )
                    .await
                    .unwrap_or_default();
                for offset in 0..5i64 {
                    let j = debut_sem + Duration::days(offset);
                    if j > jour {
                        continue;
                    }
                    if j.weekday().num_days_from_monday() >= 5 {
                        continue;
                    }
                    if passages.iter().any(|p| p.jour == j) {
                        continue;
                    }
                    let _ = self
                        .scan_badge_repas_for_actor(
                            etab,
                            role_compta,
                            badge,
                            RepasActiviteKind::Midi,
                            j,
                            compta.user_id,
                        )
                        .await;
                }
            }
        }

        const DISPENSE_REPAS_HARRY_MOTIF: &str = "Seed audit AB-A — dispense cantine 1 jour";
        if let Some(act_midi) = self
            .list_repas_activites(etab)
            .await?
            .into_iter()
            .find(|a| a.code.eq_ignore_ascii_case("MIDI"))
        {
            let mut jour_dispense: Option<NaiveDate> = None;
            for offset in 0..5i64 {
                let j = debut_sem + Duration::days(offset);
                if j > jour {
                    break;
                }
                if j.weekday().num_days_from_monday() >= 5 {
                    continue;
                }
                jour_dispense = Some(j);
                break;
            }
            if let Some(j_disp) = jour_dispense {
                let dispenses_harry = self
                    .list_dispenses_repas_eleve(etab, harry.id)
                    .await?;
                if !dispenses_harry.iter().any(|d| {
                    !d.annulee
                        && d.activite_id == act_midi.id
                        && d.jour == j_disp
                        && d.motif == DISPENSE_REPAS_HARRY_MOTIF
                }) {
                    let _ = self
                        .poser_dispense_repas_for_actor(
                            role_admini,
                            PoserDispenseRepas {
                                etablissement_id: etab,
                                eleve_id: harry.id,
                                activite_id: act_midi.id,
                                jour: j_disp,
                                motif: DISPENSE_REPAS_HARRY_MOTIF.into(),
                                source: DispenseSource::Staff,
                                actor_user_id: Some(administratif.user_id.to_string()),
                            },
                        )
                        .await;
                }
            }
        }

        // --- Absence Harry : a_traiter + motif_justif + pièce JUSTIF_ABSENCE_{id}
        const JUSTIF_AUDIT_MOTIF: &str = "maladie";
        // Jour courant : la file CPE `/vs/absences?jour=` ne liste que le jour sélectionné.
        let absence_justif_jour = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_justif_jour)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_justif_jour,
                fin: absence_justif_jour,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }
        let absences_harry = self.list_absences_eleve(etab, harry.id).await?;
        if let Some(abs_justif) = absences_harry
            .iter()
            .find(|a| a.debut == absence_justif_jour && a.fin == absence_justif_jour)
        {
            if abs_justif.statut == AbsenceStatut::ATraiter
                && abs_justif.motif_justif.as_deref() != Some(JUSTIF_AUDIT_MOTIF)
            {
                self.justifier_absence_for_actor(
                    etab,
                    harry.parent.user_id,
                    Role::Parent.as_str(),
                    abs_justif.id,
                    JUSTIF_AUDIT_MOTIF.into(),
                )
                .await?;
            }
            let code_justif = format!("JUSTIF_ABSENCE_{}", abs_justif.id);
            let docs_harry = self
                .list_documents_meta_eleve_for_actor(
                    etab,
                    administratif.user_id,
                    role_admini,
                    harry.id,
                )
                .await?;
            let doc_justif = if let Some(found) = docs_harry
                .iter()
                .find(|(d, _)| d.code.as_deref() == Some(code_justif.as_str()))
            {
                found.0.clone()
            } else {
                self.deposer_document_with_contenu(
                    domain_eleve::DeposerDocument {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        tiroir: DocTiroir::Famille,
                        titre: format!("Justificatif absence {code_justif}"),
                        code: Some(code_justif.clone()),
                        mime_type: Some("application/pdf".into()),
                        storage_key: None,
                        confidentialite: DocConfidentialite::Standard,
                        actor_user_id: harry.parent.user_id,
                    },
                    Some(PDF_JUSTIF_MINIMAL),
                )
                .await?
            };
            let pdf_ok = self
                .telecharger_document_bytes_for_actor(
                    etab,
                    administratif.user_id,
                    role_admini,
                    doc_justif.id,
                )
                .await
                .map(|(_, b)| !b.is_empty() && b.starts_with(b"%PDF"))
                .unwrap_or(false);
            if !pdf_ok {
                if let Some(key) = doc_justif.storage_key.as_ref() {
                    let _ = self.objects().put(key, PDF_JUSTIF_MINIMAL);
                } else {
                    let _ = self
                        .deposer_document_with_contenu(
                            domain_eleve::DeposerDocument {
                                etablissement_id: etab,
                                eleve_id: harry.id,
                                tiroir: DocTiroir::Famille,
                                titre: format!("Justificatif absence {code_justif}"),
                                code: Some(code_justif),
                                mime_type: Some("application/pdf".into()),
                                storage_key: None,
                                confidentialite: DocConfidentialite::Standard,
                                actor_user_id: harry.parent.user_id,
                            },
                            Some(PDF_JUSTIF_MINIMAL),
                        )
                        .await;
                }
            }
        }

        // --- Réinscription N+1 : projection + option retenue
        let (_, annee_n, np1) = annees_continuum(jour);
        if self
            .scolarite_eleve_annee(etab, harry.id, &np1)
            .await?
            .is_none()
        {
            self.montee_rapide(
                MonteeRapide {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    annee_courante: annee_n.clone(),
                    actor_user_id: Some(actor_admin.clone()),
                },
                jour,
                false,
            )
            .await?;
        }
        let opts_np1 = self
            .options_continuum_eleve(etab, harry.id, &np1)
            .await?;
        if !opts_np1.iter().any(|o| {
            o.code == "LV1_ANG" && o.statut == OptionAnneeStatut::Retenue
        }) {
            self.choisir_option_n1(
                ChoisirOptionN1 {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    annee_scolaire: np1.clone(),
                    nature: OptionNature::Academique,
                    code: "LV1_ANG".into(),
                    libelle: "Anglais LV1".into(),
                    retenir: true,
                    actor_user_id: Some(harry.parent.user_id.to_string()),
                },
                false,
            )
            .await?;
        }

        // --- Grant PAP en attente (prof Rogue → doc restreint Harry)
        let docs_meta = self
            .list_documents_meta_eleve_for_actor(
                etab,
                administratif.user_id,
                role_admini,
                harry.id,
            )
            .await?;
        if let Some((pap, _)) = docs_meta
            .iter()
            .find(|(d, _)| d.titre.contains("PAP Harry"))
        {
            let pending = self
                .list_grants_pending_for_actor(etab, role_admini)
                .await?;
            if !pending
                .iter()
                .any(|g| g.document_id == pap.id && g.grantee_user_id == prof_rogue.user_id)
            {
                self.demander_grant_document(DemanderGrant {
                    etablissement_id: etab,
                    document_id: pap.id,
                    grantee_user_id: prof_rogue.user_id,
                    duration_days: 14,
                    note: Some("Seed labo — accès PAP en attente".into()),
                })
                .await?;
            }
        }

        // --- Dossier commission VS Harry (convocation seed)
        const COMMISSION_MOTIFS: &str = "Seed labo — commission Harry audit";
        let dossiers_vs = self
            .list_commissions_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
            )
            .await?;
        if !dossiers_vs
            .iter()
            .any(|d| d.motifs.contains(COMMISSION_MOTIFS))
        {
            let commission = self
                .creer_commission_for_actor(
                    Role::Cpe.as_str(),
                    CreerCommissionVs {
                        etablissement_id: etab,
                        // Aujourd’hui : tuile home CPE + deep-link `?com=` visibles le jour J.
                        jour,
                        heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                        objet: "Incidents répétés — Harry Potter".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?;
            let _ = self
                .convoquer_commission_for_actor(
                    Role::Cpe.as_str(),
                    ConvoquerCommission {
                        etablissement_id: etab,
                        commission_id: commission.id,
                        eleve_id: harry.id,
                        motifs: COMMISSION_MOTIFS.into(),
                        note_interne: "Seed audit — préparer dossier VS".into(),
                        sanction_id: None,
                        incident_id: incident_harry,
                        signale_famille: true,
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?;
        }

        let solde_harry = self
            .solde_points_vs_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
            )
            .await?;
        if solde_harry == POINTS_VS_DEPART {
            let dossiers_harry_pts = self
                .list_commissions_eleve_for_actor(
                    etab,
                    cpe.user_id,
                    Role::Cpe.as_str(),
                    harry.id,
                )
                .await?;
            if let Some(dossier) = dossiers_harry_pts
                .iter()
                .find(|d| d.motifs.contains(COMMISSION_MOTIFS) && d.decision.is_none())
            {
                let _ = self
                    .decider_commission_dossier_for_actor(
                        Role::Cpe.as_str(),
                        DeciderCommissionDossier {
                            etablissement_id: etab,
                            dossier_id: dossier.id,
                            decision: DecisionCommission::Avertissement,
                            decision_texte: "Avertissement écrit — seed audit permis à points.".into(),
                            points_retires: 5,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
        }

        // --- Photo trombi + 2 champs info-comp
        if self.get_eleve(etab, harry.id).await?.is_some_and(|e| e.photo_key.is_none())
        {
            let _ = self
                .poser_photo_eleve_for_actor(
                    role_admini,
                    etab,
                    harry.id,
                    administratif.user_id,
                    "image/png",
                    LABO_PNG_1X1,
                )
                .await;
        }
        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            if self
                .get_eleve(etab, hermione.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        hermione.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }
        let champs_ic = self
            .list_info_comp_champs_for_actor(etab, role_admini)
            .await?;
        let champ_lang = if let Some(c) = champs_ic.iter().find(|c| c.code == "LANGUE_MAISON") {
            c.clone()
        } else {
            self.creer_info_comp_champ_for_actor(
                role_admini,
                CreerInfoCompChamp {
                    etablissement_id: etab,
                    code: "LANGUE_MAISON".into(),
                    libelle: "Langue parlée à la maison".into(),
                    type_valeur: "texte".into(),
                    obligatoire: false,
                    visible_famille: true,
                    actor_user_id: administratif.user_id,
                },
            )
            .await?
        };
        let champ_club = if let Some(c) = champs_ic.iter().find(|c| c.code == "CLUB_HARRY") {
            c.clone()
        } else {
            self.creer_info_comp_champ_for_actor(
                role_admini,
                CreerInfoCompChamp {
                    etablissement_id: etab,
                    code: "CLUB_HARRY".into(),
                    libelle: "Club extrascolaire".into(),
                    type_valeur: "texte".into(),
                    obligatoire: false,
                    visible_famille: true,
                    actor_user_id: administratif.user_id,
                },
            )
            .await?
        };
        let (_champs_eleve, valeurs_ic) = self
            .list_info_comp_eleve_for_actor(
                etab,
                administratif.user_id,
                role_admini,
                harry.id,
            )
            .await?;
        if !valeurs_ic
            .iter()
            .any(|v| v.champ_id == champ_lang.id && v.valeur == "Anglais")
        {
            self.upsert_info_comp_valeur_for_actor(
                role_admini,
                UpsertInfoCompValeur {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    champ_id: champ_lang.id,
                    valeur: "Anglais".into(),
                    actor_user_id: administratif.user_id,
                },
            )
            .await?;
        }
        if !valeurs_ic
            .iter()
            .any(|v| v.champ_id == champ_club.id && v.valeur == "Quidditch")
        {
            self.upsert_info_comp_valeur_for_actor(
                role_admini,
                UpsertInfoCompValeur {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    champ_id: champ_club.id,
                    valeur: "Quidditch".into(),
                    actor_user_id: administratif.user_id,
                },
            )
            .await?;
        }

        // --- Parent multi-enfants : Ron + Ginny → même compte Molly (foyer Weasley)
        let role_admin = Role::Admin.as_str();
        let weasley_seed = labo_eleve_prenom(eleves, "Ron").zip(labo_eleve_prenom(eleves, "Ginny"))
            .map(|(ron, ginny)| {
                (
                    ron.parent.clone(),
                    ron.foyer_id,
                    ginny.id,
                    ginny.parent.user_id,
                    ginny.peut_voir_notes,
                    ginny.peut_voir_vs,
                    ginny.peut_justifier,
                )
            });
        if let Some((
            molly_persona,
            molly_foyer,
            ginny_id,
            ginny_old_parent,
            ginny_notes,
            ginny_vs,
            ginny_justif,
        )) = weasley_seed
        {
            let molly_user = molly_persona.user_id;
            let deja = self
                .lien_parent_eleve(etab, molly_user, ginny_id)
                .await?;
            if deja.is_none() {
                if ginny_old_parent != molly_user {
                    if let Some(old) = self
                        .lien_parent_eleve(etab, ginny_old_parent, ginny_id)
                        .await?
                    {
                        self.delete_lien_eleve_for_actor(
                            role_admin,
                            etab,
                            Some(ginny_id),
                            old.id,
                            admin.user_id,
                        )
                        .await?;
                    }
                }
                self.link_parent(CreateLien {
                    etablissement_id: etab,
                    eleve_id: ginny_id,
                    user_id: molly_user,
                    foyer_id: Some(molly_foyer),
                    lien: "mère".into(),
                    peut_voir_notes: ginny_notes,
                    peut_voir_vs: ginny_vs,
                    peut_justifier_absence: ginny_justif,
                    peut_autoriser_sorties: ginny_justif,
                    est_payeur: true,
                    part_paiement: Some(100),
                })
                .await?;
            }
            if let Some(ginny_idx) = eleves
                .iter()
                .position(|e| e.prenom.eq_ignore_ascii_case("Ginny"))
            {
                eleves[ginny_idx].parent = molly_persona;
                eleves[ginny_idx].foyer_id = molly_foyer;
            }

            let enfants_molly = self
                .mes_enfants_for_parent(etab, molly_user, Role::Parent.as_str())
                .await?;
            let ron_id = labo_eleve_prenom(eleves, "Ron").map(|r| r.id);
            let mut ids_molly: Vec<Uuid> = enfants_molly.iter().map(|e| e.eleve.id).collect();
            if let Some(rid) = ron_id {
                if !ids_molly.iter().any(|id| *id == rid) {
                    if let Some(ron) = labo_eleve_prenom(eleves, "Ron") {
                        self.link_parent(CreateLien {
                            etablissement_id: etab,
                            eleve_id: rid,
                            user_id: molly_user,
                            foyer_id: Some(molly_foyer),
                            lien: "mère".into(),
                            peut_voir_notes: ron.peut_voir_notes,
                            peut_voir_vs: ron.peut_voir_vs,
                            peut_justifier_absence: ron.peut_justifier,
                            peut_autoriser_sorties: ron.peut_justifier,
                            est_payeur: true,
                            part_paiement: Some(100),
                        })
                        .await?;
                        ids_molly.push(rid);
                    }
                }
                if !ids_molly.iter().any(|id| *id == ginny_id) {
                    self.link_parent(CreateLien {
                        etablissement_id: etab,
                        eleve_id: ginny_id,
                        user_id: molly_user,
                        foyer_id: Some(molly_foyer),
                        lien: "mère".into(),
                        peut_voir_notes: ginny_notes,
                        peut_voir_vs: ginny_vs,
                        peut_justifier_absence: ginny_justif,
                        peut_autoriser_sorties: ginny_justif,
                        est_payeur: true,
                        part_paiement: Some(100),
                    })
                    .await?;
                }
            }
        }

        // --- Dispense EPS Harry + autorisation sortie permanente (Lily)
        let fin_as = NaiveDate::from_ymd_opt(debut_as.year() + 1, 7, 5)
            .unwrap_or_else(|| debut_as + Duration::days(300));
        let dispenses_harry = self
            .list_dispenses_vs_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
                true,
            )
            .await?;
        if !dispenses_harry.iter().any(|d| {
            d.motif == DispenseVsMotif::Eps && d.statut == DispenseVsStatut::Active
        }) {
            self.poser_dispense_vs_for_actor(
                Role::Cpe.as_str(),
                PoserDispenseVs {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    motif: "eps".into(),
                    debut: debut_as,
                    fin: fin_as,
                    commentaire: "Seed audit — dispense EPS (asthme, certificat médical).".into(),
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }
        let recap_sortie = self
            .get_recap_sortie_for_actor(
                etab,
                harry.parent.user_id,
                Role::Parent.as_str(),
                harry.id,
            )
            .await?;
        let autorisation_incomplete = recap_sortie
            .autorisation
            .as_ref()
            .map(|a| !a.matin || !a.midi || !a.soir || a.interdit_etablissement)
            .unwrap_or(true);
        if autorisation_incomplete {
            self.upsert_autorisation_sortie_for_actor(
                Role::Parent.as_str(),
                UpsertAutorisationSortie {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    matin: true,
                    midi: true,
                    soir: true,
                    interdit_etablissement: false,
                    actor_user_id: harry.parent.user_id,
                },
            )
            .await?;
        }

        // --- Internat Neville (seed DP → INT : affectation exige régime INT)
        const CHAMBRE_NEVILLE_CODE: &str = "LAB-NEV-AUDIT";
        const SUIVI_NEVILLE: &str = "Seed audit — bonne soirée étude";
        if let Some(neville) = labo_eleve_prenom(eleves, "Neville") {
            let role_cpe = Role::Cpe.as_str();
            let regime = self
                .get_eleve(etab, neville.id)
                .await?
                .and_then(|e| e.regime);
            let regime_norm = regime
                .as_deref()
                .map(|r| r.trim().to_ascii_uppercase())
                .unwrap_or_default();
            if regime_norm != "INT" {
                self.change_regime(ChangeRegime {
                    etablissement_id: etab,
                    eleve_id: neville.id,
                    nouveau_regime: "INT".into(),
                    date_effet: jour,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
            let chambre = {
                let existantes = self
                    .list_chambres_internat_for_actor(etab, role_cpe)
                    .await?;
                if let Some(c) = existantes
                    .iter()
                    .find(|c| c.code.eq_ignore_ascii_case(CHAMBRE_NEVILLE_CODE))
                {
                    c.clone()
                } else {
                    self.creer_chambre_internat_for_actor(
                        role_cpe,
                        CreerChambreInternat {
                            etablissement_id: etab,
                            batiment: "Tour Nord".into(),
                            etage: 1,
                            code: CHAMBRE_NEVILLE_CODE.into(),
                            capacite: 2,
                            genre: "garcons".into(),
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await?
                }
            };
            let affectations = self
                .list_affectations_internat_jour_for_actor(etab, role_cpe, jour)
                .await?;
            if !affectations
                .iter()
                .any(|a| a.eleve_id == neville.id && a.chambre_id == chambre.id)
            {
                let _ = self
                    .affecter_eleve_internat_for_actor(
                        role_cpe,
                        AffecterEleveInternat {
                            etablissement_id: etab,
                            eleve_id: neville.id,
                            chambre_id: chambre.id,
                            lit: Some("A".into()),
                            debut: jour,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
            let presences = self
                .list_presences_internat_jour_for_actor(etab, role_cpe, jour)
                .await?;
            if !presences.iter().any(|p| p.eleve_id == neville.id) {
                let _ = self
                    .pointer_presence_internat_for_actor(
                        role_cpe,
                        PointerPresenceInternat {
                            etablissement_id: etab,
                            eleve_id: neville.id,
                            jour,
                            statut: "present".into(),
                            note: "Seed audit internat".into(),
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
            let recap = self
                .get_recap_internat_for_actor(
                    etab,
                    cpe.user_id,
                    role_cpe,
                    neville.id,
                    jour,
                )
                .await?;
            if !recap
                .suivis_famille
                .iter()
                .any(|s| s.texte.contains(SUIVI_NEVILLE))
            {
                let _ = self
                    .creer_suivi_internat_for_actor(
                        role_cpe,
                        CreerSuiviInternat {
                            etablissement_id: etab,
                            eleve_id: neville.id,
                            jour,
                            texte: SUIVI_NEVILLE.into(),
                            visible_famille: true,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
            const NEVILLE_NOTE_LIB: &str = "Interro Potions lab (Neville)";
            if !self
                .notes_for_eleve(etab, neville.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(NEVILLE_NOTE_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: neville.id,
                    matiere_id: potions.id,
                    valeur: 12.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_rogue.user_id,
                    libelle: Some(NEVILLE_NOTE_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        // --- Psy Harry : dossier + RDV signalé famille (non signé côté docs)
        if let Some(psy) = psys.first() {
            let role_psy = Role::Psy.as_str();
            if self
                .get_dossier_psy_for_actor(etab, psy.user_id, role_psy, harry.id)
                .await?
                .is_none()
            {
                self.upsert_dossier_psy_for_actor(
                    role_psy,
                    UpsertDossierPsy {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        axes_suivi: "Anxiété scolaire légère — suivi T1".into(),
                        contacts_externes: "CAMSP — Mme Dubois".into(),
                        note_confidentielle: "Seed audit — note interne psy".into(),
                        suivi_actif: true,
                        signale_famille: true,
                        actor_user_id: psy.user_id,
                    },
                )
                .await?;
            }
            const RDV_PSY_AUDIT: &str = "Seed audit — entretien de suivi Harry";
            let rdvs = self
                .list_rdvs_psy_for_actor(etab, psy.user_id, role_psy, Some(harry.id), None)
                .await?;
            if !rdvs
                .iter()
                .any(|r| r.motif == RDV_PSY_AUDIT && r.signale_famille)
            {
                self.creer_rdv_psy_for_actor(
                    role_psy,
                    CreerRdvPsy {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        jour: jour + Duration::days(5),
                        heure_debut: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                        heure_fin: NaiveTime::from_hms_opt(10, 45, 0).unwrap(),
                        kind: EntretienPsyKind::Individuel,
                        motif: RDV_PSY_AUDIT.into(),
                        note_interne: "Préparer retour post-incident Potions".into(),
                        signale_famille: true,
                        actor_user_id: psy.user_id,
                    },
                )
                .await?;
            }
        }

        // --- Outbox mail/SMS dry-run (carnet visible famille → notify_parents_* si mailer prêt)
        const CARNET_OUTBOX_AUDIT_TITRE: &str = "AUDIT: Outbox dry-run Harry";
        let carnet_outbox = self
            .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), harry.id)
            .await?;
        if !carnet_outbox
            .iter()
            .any(|c| c.titre == CARNET_OUTBOX_AUDIT_TITRE)
        {
            self.creer_carnet_entree(CreerCarnetEntree {
                etablissement_id: etab,
                eleve_id: harry.id,
                date_entree: jour,
                categorie: CarnetCategorie::Information,
                titre: CARNET_OUTBOX_AUDIT_TITRE.into(),
                corps: "Seed labo — entrée carnet déclenchant mail + SMS outbox (dry-run) vers Lily Potter."
                    .into(),
                visible_famille: true,
                actor_user_id: cpe.user_id,
            })
            .await?;
        }
        if self.count_mails_kind(etab, "carnet").await? == 0 {
            if let Some(psy) = psys.first() {
                let _ = self
                    .envoyer_courrier_orientation_psy_for_actor(
                        etab,
                        psy.user_id,
                        Role::Psy.as_str(),
                        harry.id,
                    )
                    .await;
            }
        }

        // --- File admin inscriptions : 1 dossier à l'étape « pièces » (Cedric, sans toucher Harry)
        const INSCRIPTION_FILE_ELEVE: &str = "Cedric";
        const INSCRIPTION_FILE_COMMENT: &str = "Seed audit — dossier en attente de pièces (démo /inscriptions).";
        if let Some(cedric) = labo_eleve_prenom(eleves, INSCRIPTION_FILE_ELEVE) {
            let annee_insc = annee_scolaire_de(jour);
            let deja = self
                .get_inscription_suivi_for_actor(
                    etab,
                    administratif.user_id,
                    role_admini,
                    cedric.id,
                    &annee_insc,
                )
                .await?;
            if deja.is_none() {
                self.upsert_inscription_suivi_for_actor(
                    role_admini,
                    UpsertInscriptionSuivi {
                        etablissement_id: etab,
                        eleve_id: cedric.id,
                        annee_scolaire: annee_insc,
                        etape: "pieces".into(),
                        motif_code: Some("incomplet".into()),
                        commentaire: INSCRIPTION_FILE_COMMENT.into(),
                        actor_user_id: administratif.user_id,
                    },
                )
                .await?;
            }

            let absence_cedric_j1 = jour - Duration::days(1);
            if !self
                .eleve_est_absent(etab, cedric.id, absence_cedric_j1)
                .await?
            {
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: cedric.id,
                    debut: absence_cedric_j1,
                    fin: absence_cedric_j1,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        // --- Cast Hermione : note T1, absence J-2 non justifiée, avertissement VS
        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            const HERMIONE_NOTE_LIB: &str = "Interro Mathématiques lab (Hermione)";
            if !self
                .notes_for_eleve(etab, hermione.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(HERMIONE_NOTE_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: hermione.id,
                    matiere_id: math.id,
                    valeur: 18.5,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(HERMIONE_NOTE_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }

            let absence_hermione = jour - Duration::days(2);
            if !self
                .eleve_est_absent(etab, hermione.id, absence_hermione)
                .await?
            {
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: hermione.id,
                    debut: absence_hermione,
                    fin: absence_hermione,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }

            let sanctions_hermione = self
                .list_sanctions_eleve_for_actor(
                    etab,
                    cpe.user_id,
                    Role::Cpe.as_str(),
                    hermione.id,
                )
                .await?;
            const HERMIONE_SANCTION_NOTE: &str =
                "Seed labo — avertissement Hermione (diversité cast VS).";
            if !sanctions_hermione.iter().any(|s| {
                sanction_types
                    .iter()
                    .any(|t| t.id == s.type_id && t.code.eq_ignore_ascii_case("avertissement"))
            }) {
                if let Some(typ) = sanction_types
                    .iter()
                    .find(|t| t.code.eq_ignore_ascii_case("avertissement"))
                {
                    let motif_id = sanction_motifs
                        .iter()
                        .find(|m| m.code.eq_ignore_ascii_case("devoirs"))
                        .map(|m| m.id);
                    self.prononcer_sanction(PrononcerSanction {
                        etablissement_id: etab,
                        eleve_id: hermione.id,
                        type_id: typ.id,
                        motif_id,
                        incident_id: None,
                        jour: jour - Duration::days(2),
                        duree_minutes: None,
                        note: Some(HERMIONE_SANCTION_NOTE.into()),
                        actor_user_id: cpe.user_id,
                    })
                    .await?;
                }
            }
        }

        // --- Bulletin démo : période T0 fermée (T1 reste ouverte pour la saisie)
        const T0_NOTE_LIB: &str = "Contrôle T0 démo bulletin";
        let fin_t0 = debut_as - Duration::days(1);
        let debut_t0 = fin_t0 - Duration::days(90);
        let periode_t0 = self
            .ensure_note_periode_lab(
                etab,
                "T0",
                "Trimestre 0 (démo)",
                debut_t0,
                fin_t0,
                &actor_admin,
            )
            .await?;
        if periode_t0.statut.autorise_saisie_notes()
            && !self
                .notes_for_eleve(etab, harry.id)
                .await?
                .iter()
                .any(|n| {
                    n.periode_id == Some(periode_t0.id)
                        && n.libelle.as_deref() == Some(T0_NOTE_LIB)
                })
        {
            self.saisir_note(SaisirNote {
                etablissement_id: etab,
                eleve_id: harry.id,
                matiere_id: potions.id,
                valeur: 15.0,
                sur: 20.0,
                date_note: fin_t0 - Duration::days(14),
                auteur_user_id: prof_rogue.user_id,
                libelle: Some(T0_NOTE_LIB.into()),
                devoir_id: None,
                periode_id: Some(periode_t0.id),
            })
            .await?;
        }
        let periode_t0 = if periode_t0.statut != NotePeriodeStatut::Fermee {
            self.set_note_periode_statut(
                etab,
                periode_t0.id,
                NotePeriodeStatut::Fermee,
                Some(actor_admin.clone()),
            )
            .await?
        } else {
            periode_t0
        };
        let docs_harry_bul = self
            .list_documents_meta_eleve_for_actor(
                etab,
                administratif.user_id,
                role_admini,
                harry.id,
            )
            .await?;
        let bulletin_t0_code = format!("BULLETIN_{}", periode_t0.code);
        if !docs_harry_bul
            .iter()
            .any(|(d, _)| d.code.as_deref() == Some(bulletin_t0_code.as_str()))
        {
            let _ = self
                .publier_bulletin_for_actor(
                    Role::Direction.as_str(),
                    etab,
                    harry.id,
                    periode_t0.id,
                    admin.user_id,
                )
                .await;
        }

        // --- Bot AA-A lot A : Luna (note + absence NJ), Cho (carnet famille), Seamus (incident VS),
        // mandat SEPA actif foyer Harry, renfort Molly → Ron + Ginny dans mes_enfants.
        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            const LUNA_NOTE_LIB: &str = "Interro Français lab (Luna)";
            if !self
                .notes_for_eleve(etab, luna.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(LUNA_NOTE_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: luna.id,
                    matiere_id: math.id,
                    valeur: 13.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(LUNA_NOTE_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }

            let absence_luna = jour - Duration::days(4);
            if !self.eleve_est_absent(etab, luna.id, absence_luna).await? {
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: luna.id,
                    debut: absence_luna,
                    fin: absence_luna,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        if let Some(cho) = labo_eleve_prenom(eleves, "Cho") {
            const CHO_CARNET_AUDIT_TITRE: &str = "AUDIT: Encouragements Cho (famille)";
            let carnet_cho = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), cho.id)
                .await?;
            if !carnet_cho
                .iter()
                .any(|c| c.titre == CHO_CARNET_AUDIT_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: cho.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: CHO_CARNET_AUDIT_TITRE.into(),
                    corps: "Seed labo lot A — entrée carnet visible famille pour audit portail Cho."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            const SEAMUS_INCIDENT_DESC: &str = "Seed labo lot A — incident VS mineur (retard cours).";
            let incidents_seamus = self
                .list_incidents_eleve_for_actor(
                    etab,
                    cpe.user_id,
                    Role::Cpe.as_str(),
                    seamus.id,
                )
                .await?;
            if !incidents_seamus
                .iter()
                .any(|i| i.description.as_deref() == Some(SEAMUS_INCIDENT_DESC))
            {
                if let Some(typ) = incident_types
                    .iter()
                    .find(|t| t.code.eq_ignore_ascii_case("perturbation"))
                    .or(incident_types.first())
                {
                    let _ = self
                        .declarer_incident_for_actor(
                            Role::Prof.as_str(),
                            DeclarerIncident {
                                etablissement_id: etab,
                                eleve_id: seamus.id,
                                type_id: typ.id,
                                jour: jour - Duration::days(1),
                                description: Some(SEAMUS_INCIDENT_DESC.into()),
                                actor_user_id: prof_rogue.user_id,
                            },
                        )
                        .await;
                }
            }
        }

        let rum_harry_sepa = format!("LAB-HARRY-SEPA-{etab_short}");
        let mandats_sepa = self
            .list_mandats_sepa_for_actor(etab, compta.user_id, role_compta)
            .await?;
        let harry_mandat_seed = mandats_sepa.iter().any(|m| {
            m.foyer_id == harry.foyer_id && m.rum == rum_harry_sepa
        });
        let harry_mandat_actif = mandats_sepa.iter().any(|m| {
            m.foyer_id == harry.foyer_id && m.statut == MandatSepaStatut::Actif
        });
        if !harry_mandat_seed && !harry_mandat_actif {
            let _ = self
                .creer_mandat_sepa_for_actor(
                    role_compta,
                    compta.user_id,
                    CreerMandatSepa {
                        etablissement_id: etab,
                        foyer_id: harry.foyer_id,
                        rum: rum_harry_sepa,
                        iban: "FR7630006000011234567890189".into(),
                        bic: Some("AGRIFRPP".into()),
                        titulaire: "Lily Potter".into(),
                        signe_at: Utc::now(),
                        actor_user_id: compta.user_id,
                    },
                )
                .await;
        }

        if let (Some(ron), Some(ginny)) = (
            labo_eleve_prenom(eleves, "Ron"),
            labo_eleve_prenom(eleves, "Ginny"),
        ) {
            let molly_user = ron.parent.user_id;
            let molly_foyer = ron.foyer_id;
            let enfants_molly = self
                .mes_enfants_for_parent(etab, molly_user, Role::Parent.as_str())
                .await?;
            let ids_molly: Vec<Uuid> = enfants_molly.iter().map(|e| e.eleve.id).collect();
            if !ids_molly.iter().any(|id| *id == ron.id) {
                self.link_parent(CreateLien {
                    etablissement_id: etab,
                    eleve_id: ron.id,
                    user_id: molly_user,
                    foyer_id: Some(molly_foyer),
                    lien: "mère".into(),
                    peut_voir_notes: ron.peut_voir_notes,
                    peut_voir_vs: ron.peut_voir_vs,
                    peut_justifier_absence: ron.peut_justifier,
                    peut_autoriser_sorties: ron.peut_justifier,
                    est_payeur: true,
                    part_paiement: Some(100),
                })
                .await?;
            }
            if !ids_molly.iter().any(|id| *id == ginny.id) {
                self.link_parent(CreateLien {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    user_id: molly_user,
                    foyer_id: Some(molly_foyer),
                    lien: "mère".into(),
                    peut_voir_notes: ginny.peut_voir_notes,
                    peut_voir_vs: ginny.peut_voir_vs,
                    peut_justifier_absence: ginny.peut_justifier,
                    peut_autoriser_sorties: ginny.peut_justifier,
                    est_payeur: true,
                    part_paiement: Some(100),
                })
                .await?;
            }
            if let Some(ginny_idx) = eleves
                .iter()
                .position(|e| e.prenom.eq_ignore_ascii_case("Ginny"))
            {
                eleves[ginny_idx].parent = ron.parent.clone();
                eleves[ginny_idx].foyer_id = molly_foyer;
            }
        }

        // --- Bot AC-A lot C : note Seamus, carnet Cedric, rappel facture Harry, cahier 5A, photo Neville.
        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            const SEAMUS_NOTE_LIB: &str = "Interro Potions lab (Seamus)";
            if !self
                .notes_for_eleve(etab, seamus.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(SEAMUS_NOTE_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: seamus.id,
                    matiere_id: potions.id,
                    valeur: 10.5,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_rogue.user_id,
                    libelle: Some(SEAMUS_NOTE_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(cedric) = labo_eleve_prenom(eleves, "Cedric") {
            const CEDRIC_CARNET_AUDIT_TITRE: &str = "AUDIT: Encouragements Cedric (famille)";
            let carnet_cedric = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), cedric.id)
                .await?;
            if !carnet_cedric
                .iter()
                .any(|c| c.titre == CEDRIC_CARNET_AUDIT_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: cedric.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: CEDRIC_CARNET_AUDIT_TITRE.into(),
                    corps: "Seed labo lot C — entrée carnet visible famille pour audit portail Cedric Diggory."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        const CAHIER_5A_AUDIT_TITRE: &str = "Devoir maison Transfiguration 5A (audit lot C)";
        let cahier_5a = self
            .list_cahier_texte_for_actor(
                etab,
                prof_mcg.user_id,
                Role::Prof.as_str(),
                Some("5A".into()),
                None,
                None,
                None,
            )
            .await?;
        if !cahier_5a
            .iter()
            .any(|e| e.titre == CAHIER_5A_AUDIT_TITRE && e.matiere_libelle == "Transfiguration")
        {
            self.creer_cahier_texte_for_actor(
                Role::Prof.as_str(),
                CreerCahierTexte {
                    etablissement_id: etab,
                    classe: "5A".into(),
                    matiere_libelle: "Transfiguration".into(),
                    titre: CAHIER_5A_AUDIT_TITRE.into(),
                    consigne: "Exercices ch.3 — transformer une allumette en aiguille (schéma + 5 lignes de protocole)."
                        .into(),
                    pour_le: jour + Duration::days(5),
                    cree_le: jour,
                    actor_user_id: prof_mcg.user_id,
                },
            )
            .await?;
        }

        let rappels_foyer_harry = self
            .list_rappels_for_actor(etab, role_compta)
            .await?
            .into_iter()
            .filter(|r| r.foyer_id == harry.foyer_id)
            .collect::<Vec<_>>();
        if rappels_foyer_harry.is_empty() {
            let (facture_h2, _, _, encaisse_h2, echeances_h2, _, _) = self
                .get_facture_for_actor(etab, compta.user_id, role_compta, facture_harry_2_id)
                .await?;
            if echeances_h2.is_empty() {
                let montant_rappel = facture_h2
                    .total_centimes
                    .saturating_sub(encaisse_h2)
                    .max(1);
                let _ = self
                    .poser_echeancier_for_actor(
                        role_compta,
                        PoserEcheancier {
                            etablissement_id: etab,
                            facture_id: facture_harry_2_id,
                            echeances: vec![EcheanceCmd {
                                libelle: "Seed audit lot C — échéance en retard".into(),
                                montant_centimes: montant_rappel,
                                date_echeance: jour - Duration::days(5),
                            }],
                        },
                    )
                    .await;
            }
            let _ = self
                .lancer_rappels_echeances_for_actor(
                    etab,
                    role_compta,
                    compta.user_id,
                    jour,
                )
                .await;
        }

        if let Some(neville) = labo_eleve_prenom(eleves, "Neville") {
            if self
                .get_eleve(etab, neville.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        neville.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        // --- Bot AD-A lot D : carnet Luna (famille), passage infirmerie Hermione, sortie except. Harry (jour J),
        // petite facture émise foyer Draco ; ré-ensure bulletin T0 publié.
        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            const LUNA_CARNET_AUDIT_TITRE: &str = "AUDIT: Encouragements Luna (famille)";
            let carnet_luna = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), luna.id)
                .await?;
            if !carnet_luna
                .iter()
                .any(|c| c.titre == LUNA_CARNET_AUDIT_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: luna.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: LUNA_CARNET_AUDIT_TITRE.into(),
                    corps: "Seed labo lot AD-A — entrée carnet visible famille pour audit portail Luna Lovegood."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            const HERMIONE_INFIRMERIE_AUDIT: &str =
                "Seed labo lot AD-A — pansement doigt (coupure papier).";
            let passages_hermione = self
                .list_passages_infirmerie_for_actor(
                    etab,
                    inf.user_id,
                    Role::Infirmerie.as_str(),
                    Some(hermione.id),
                    None,
                )
                .await?;
            if !passages_hermione
                .iter()
                .any(|p| p.note_soins == HERMIONE_INFIRMERIE_AUDIT)
            {
                self.creer_passage_infirmerie_for_actor(
                    Role::Infirmerie.as_str(),
                    CreerPassageInfirmerie {
                        etablissement_id: etab,
                        eleve_id: hermione.id,
                        jour,
                        motif: PassageInfirmerieMotif::Soins,
                        note_soins: HERMIONE_INFIRMERIE_AUDIT.into(),
                        visible_famille: true,
                        actor_user_id: inf.user_id,
                    },
                )
                .await?;
            }
        }

        const HARRY_SORTIE_EXCEPT_MOTIF: &str = "Seed labo lot AD-A — RDV médical jour J";
        let recap_harry_sortie = self
            .get_recap_sortie_for_actor(
                etab,
                harry.parent.user_id,
                Role::Parent.as_str(),
                harry.id,
            )
            .await?;
        if !recap_harry_sortie.demandes_ouvertes.iter().any(|d| {
            d.jour == jour && d.motif == HARRY_SORTIE_EXCEPT_MOTIF
        }) {
            let _ = self
                .demander_sortie_exceptionnelle_for_actor(
                    Role::Parent.as_str(),
                    DemanderSortieExceptionnelle {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        jour,
                        creneau: SortieCreneau::Midi,
                        motif: HARRY_SORTIE_EXCEPT_MOTIF.into(),
                        actor_user_id: harry.parent.user_id,
                    },
                )
                .await;
        }

        let fac_draco_numero = format!("LAB-DRACO-{etab_short}");
        let factures_draco = self
            .list_factures_for_actor(etab, compta.user_id, role_compta)
            .await?;
        let facture_draco = if let Some(f) = factures_draco
            .iter()
            .find(|f| f.foyer_id == draco.foyer_id && f.numero == fac_draco_numero)
        {
            f.clone()
        } else {
            let (f, _lignes) = self
                .creer_facture_for_actor(
                    role_compta,
                    CreerFacture {
                        etablissement_id: etab,
                        foyer_id: draco.foyer_id,
                        numero: fac_draco_numero.clone(),
                        libelle: "Fournitures — seed audit Draco".into(),
                        nature: None,
                        annee_scolaire: Some(annee_scolaire_de(jour)),
                        lignes: vec![LigneFactureCmd {
                            eleve_id: Some(draco.id),
                            libelle: "Cahier de correspondance".into(),
                            quantite: 1,
                            montant_unitaire_centimes: 350,
                        }],
                        actor_user_id: compta.user_id,
                    },
                )
                .await?;
            f
        };
        if facture_draco.statut == FactureStatut::Brouillon {
            let _ = self
                .emettre_facture_for_actor(etab, facture_draco.id, compta.user_id, role_compta)
                .await?;
        }

        let docs_harry_t0_ensure = self
            .list_documents_meta_eleve_for_actor(
                etab,
                administratif.user_id,
                role_admini,
                harry.id,
            )
            .await?;
        if !docs_harry_t0_ensure
            .iter()
            .any(|(d, _)| d.code.as_deref() == Some(bulletin_t0_code.as_str()))
        {
            let _ = self
                .publier_bulletin_for_actor(
                    Role::Direction.as_str(),
                    etab,
                    harry.id,
                    periode_t0.id,
                    admin.user_id,
                )
                .await;
        }

        // --- Bot AE-A lot E : note Cho, absence NJ Ron (Molly / VS), note Ginny (staff),
        // alerte CPE urgente Harry (API severite), renfort PDF JUSTIF (ci-dessus).
        if let Some(cho) = labo_eleve_prenom(eleves, "Cho") {
            const CHO_NOTE_LIB: &str = "Interro Mathématiques lab (Cho)";
            if !self
                .notes_for_eleve(etab, cho.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(CHO_NOTE_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: cho.id,
                    matiere_id: math.id,
                    valeur: 14.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(CHO_NOTE_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(ron) = labo_eleve_prenom(eleves, "Ron") {
            let absence_ron_nj = jour - Duration::days(3);
            if !self
                .eleve_est_absent(etab, ron.id, absence_ron_nj)
                .await?
            {
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: ron.id,
                    debut: absence_ron_nj,
                    fin: absence_ron_nj,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        if let Some(ginny) = labo_eleve_prenom(eleves, "Ginny") {
            const GINNY_NOTE_LIB: &str = "Interro Potions lab (Ginny)";
            if !self
                .notes_for_eleve(etab, ginny.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(GINNY_NOTE_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    matiere_id: potions.id,
                    valeur: 15.5,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_rogue.user_id,
                    libelle: Some(GINNY_NOTE_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        let alertes_harry_ae = self
            .list_alertes_cpe_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
            )
            .await?;
        let harry_a_urgent = alertes_harry_ae.iter().any(|a| {
            a.severite == AlerteCpeSeverite::Urgent && a.statut == AlerteCpeStatut::Ouverte
        });
        if !harry_a_urgent {
            const HARRY_ALERTE_AE_TITRE: &str = "AUDIT AE-A: Alerte CPE urgente Harry";
            self.creer_alerte_cpe_for_actor(
                Role::Cpe.as_str(),
                CreerAlerteCpe {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    titre: HARRY_ALERTE_AE_TITRE.into(),
                    detail: "Seed labo lot AE-A — severite urgent pour audit API / UI CPE.".into(),
                    severite: "urgent".into(),
                    visible_famille: true,
                    jour,
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }

        // --- Bot AF-A lot F : carnet Seamus (famille), absence NJ Luna, dossier conseil Harry,
        // photo trombi Ron, renfort menus lun–ven publiés.
        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            const SEAMUS_CARNET_AF_TITRE: &str = "AUDIT AF-A: Encouragements Seamus (famille)";
            let carnet_seamus = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), seamus.id)
                .await?;
            if !carnet_seamus
                .iter()
                .any(|c| c.titre == SEAMUS_CARNET_AF_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: seamus.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: SEAMUS_CARNET_AF_TITRE.into(),
                    corps: "Seed labo lot AF-A — entrée carnet visible famille pour audit portail Seamus Finnigan."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            let absences_luna = self.list_absences_eleve(etab, luna.id).await?;
            let luna_a_absence_nj = absences_luna
                .iter()
                .any(|a| a.statut == AbsenceStatut::ATraiter);
            if !luna_a_absence_nj {
                let absence_luna_af = jour - Duration::days(2);
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: luna.id,
                    debut: absence_luna_af,
                    fin: absence_luna_af,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        const AF_A_CONSEIL_PERIODE: &str = "AUDIT AF-A T1";
        const HARRY_CONSEIL_AF_APP: &str =
            "Seed labo AF-A — trimestre solide, poursuite des efforts.";
        let role_cpe_conseil = Role::Cpe.as_str();
        let conseils_harry_classe = self
            .list_conseils_classe_for_actor(etab, role_cpe_conseil, Some(&harry.classe))
            .await?;
        let conseil_harry_af = if let Some(c) = conseils_harry_classe
            .iter()
            .find(|c| c.periode_libelle == AF_A_CONSEIL_PERIODE)
        {
            c.clone()
        } else {
            self.creer_conseil_classe_for_actor(
                role_cpe_conseil,
                CreerConseilClasse {
                    etablissement_id: etab,
                    classe: harry.classe.clone(),
                    periode_libelle: AF_A_CONSEIL_PERIODE.into(),
                    jour: jour + Duration::days(7),
                    heure: NaiveTime::from_hms_opt(17, 0, 0).expect("heure conseil valide"),
                    lieu: "Salle des conseils".into(),
                    actor_user_id: cpe.user_id,
                },
            )
            .await?
        };
        let dossiers_conseil_af = self
            .list_dossiers_conseil_for_actor(etab, role_cpe_conseil, conseil_harry_af.id)
            .await?;
        let dossier_harry_af = if let Some(d) = dossiers_conseil_af
            .iter()
            .find(|d| d.eleve_id == harry.id)
        {
            d.clone()
        } else {
            self.ajouter_dossier_conseil_for_actor(
                Role::Prof.as_str(),
                AjouterDossierConseil {
                    etablissement_id: etab,
                    conseil_id: conseil_harry_af.id,
                    eleve_id: harry.id,
                    actor_user_id: prof_rogue.user_id,
                },
            )
            .await?
        };
        if dossier_harry_af.decided_at.is_none() {
            let _ = self
                .decider_dossier_conseil_for_actor(
                    role_cpe_conseil,
                    DeciderDossierConseil {
                        etablissement_id: etab,
                        dossier_id: dossier_harry_af.id,
                        appreciation: HARRY_CONSEIL_AF_APP.into(),
                        mention: "encouragements".into(),
                        decision_texte:
                            "Seed labo AF-A — décision conseil visible portail famille.".into(),
                        visible_famille: true,
                        actor_user_id: cpe.user_id,
                    },
                )
                .await;
        }

        if let Some(ron) = labo_eleve_prenom(eleves, "Ron") {
            if self
                .get_eleve(etab, ron.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        ron.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        const MENUS_AF_RENFORCE: [(&str, &str, &str); 5] = [
            ("Velouté", "Gratin dauphinois", "Compote"),
            ("Salade", "Poulet rôti", "Yaourt"),
            ("Soupe", "Poisson pané", "Fruit"),
            ("Taboulé", "Steak haché", "Gâteau"),
            ("Carottes râpées", "Lasagnes", "Fromage"),
        ];
        for (i, menu) in MENUS_AF_RENFORCE.iter().enumerate() {
            let j_menu = debut_sem + Duration::days(i as i64);
            let menus_jour = self
                .list_menus_cantine_for_actor(
                    etab,
                    administratif.user_id,
                    role_admini,
                    j_menu,
                    j_menu,
                )
                .await?;
            let manque_plat = !menus_jour.iter().any(|m| m.plat == menu.1);
            let brouillon = menus_jour.iter().any(|m| !m.publie);
            if manque_plat || brouillon {
                self.upsert_menu_cantine_for_actor(
                    role_admini,
                    UpsertMenuCantine {
                        etablissement_id: etab,
                        jour: j_menu,
                        entree: menu.0.into(),
                        plat: menu.1.into(),
                        dessert: menu.2.into(),
                        notes: if manque_plat && harry.classe == "4B" {
                            "Menu seed audit 4B".into()
                        } else {
                            String::new()
                        },
                        publie: true,
                        actor_user_id: administratif.user_id,
                    },
                )
                .await?;
            }
        }

        // --- Bot AG-A lot G : absence NJ Seamus (Finnigan), note Cedric (Diggory), RDV psy Harry J+10,
        // carnet Draco, ré-ensure CHARTE_LAB + REGLEMENT_LAB publiés.
        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            let absence_seamus_ag = jour - Duration::days(6);
            if !self
                .eleve_est_absent(etab, seamus.id, absence_seamus_ag)
                .await?
            {
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: seamus.id,
                    debut: absence_seamus_ag,
                    fin: absence_seamus_ag,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        if let Some(cedric) = labo_eleve_prenom(eleves, "Cedric") {
            const CEDRIC_NOTE_AG_LIB: &str = "Interro Histoire-Géo lab (Cedric AG-A)";
            if !self
                .notes_for_eleve(etab, cedric.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(CEDRIC_NOTE_AG_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: cedric.id,
                    matiere_id: _hg.id,
                    valeur: 11.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(CEDRIC_NOTE_AG_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(psy) = psys.first() {
            let role_psy = Role::Psy.as_str();
            const RDV_PSY_AG_AUDIT: &str = "Seed audit AG-A — entretien complémentaire Harry J+10";
            let rdvs_ag = self
                .list_rdvs_psy_for_actor(etab, psy.user_id, role_psy, Some(harry.id), None)
                .await?;
            if !rdvs_ag
                .iter()
                .any(|r| r.motif == RDV_PSY_AG_AUDIT && r.signale_famille)
            {
                self.creer_rdv_psy_for_actor(
                    role_psy,
                    CreerRdvPsy {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        jour: jour + Duration::days(10),
                        heure_debut: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
                        heure_fin: NaiveTime::from_hms_opt(11, 45, 0).unwrap(),
                        kind: EntretienPsyKind::Individuel,
                        motif: RDV_PSY_AG_AUDIT.into(),
                        note_interne: "Seed labo lot AG-A — créneau J+10 pour audit API psy.".into(),
                        signale_famille: true,
                        actor_user_id: psy.user_id,
                    },
                )
                .await?;
            }
        }

        const DRACO_CARNET_AG_TITRE: &str = "AUDIT AG-A: Suivi comportement Draco (famille)";
        let carnet_draco_ag = self
            .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), draco.id)
            .await?;
        if !carnet_draco_ag
            .iter()
            .any(|c| c.titre == DRACO_CARNET_AG_TITRE)
        {
            self.creer_carnet_entree(CreerCarnetEntree {
                etablissement_id: etab,
                eleve_id: draco.id,
                date_entree: jour,
                categorie: CarnetCategorie::Information,
                titre: DRACO_CARNET_AG_TITRE.into(),
                corps: "Seed labo lot AG-A — entrée carnet visible famille pour audit portail Draco Malefoy."
                    .into(),
                visible_famille: true,
                actor_user_id: cpe.user_id,
            })
            .await?;
        }

        let docs_ag_ensure = self
            .list_docs_a_signer_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
                false,
            )
            .await?;
        if !docs_ag_ensure.iter().any(|d| d.code == "REGLEMENT_LAB") {
            self.publier_doc_a_signer_for_actor(
                Role::Cpe.as_str(),
                PublierDocASigner {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    code: "REGLEMENT_LAB".into(),
                    titre: "Règlement intérieur — signature requise".into(),
                    corps: "Merci de prendre connaissance du règlement et de le signer en ligne.".into(),
                    annee_scolaire: annee.clone(),
                    echeance: Some(jour + Duration::days(30)),
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }
        if !docs_ag_ensure.iter().any(|d| d.code == "CHARTE_LAB") {
            self.publier_doc_a_signer_for_actor(
                Role::Cpe.as_str(),
                PublierDocASigner {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    code: "CHARTE_LAB".into(),
                    titre: "Charte numérique — signature requise".into(),
                    corps: "Merci de lire et signer la charte d'usage des outils numériques.".into(),
                    annee_scolaire: annee.clone(),
                    echeance: Some(jour + Duration::days(21)),
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }

        // --- Bot AH-A lot H : carnet Hermione (famille), absence NJ Neville, rappel SMS dry-run Harry (si API),
        // photo trombi Ginny, renfort Molly → Ron + Ginny dans mes_enfants.
        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            const HERMIONE_CARNET_AH_TITRE: &str = "AUDIT AH-A: Encouragements Hermione (famille)";
            let carnet_hermione = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), hermione.id)
                .await?;
            if !carnet_hermione
                .iter()
                .any(|c| c.titre == HERMIONE_CARNET_AH_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: hermione.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: HERMIONE_CARNET_AH_TITRE.into(),
                    corps: "Seed labo lot AH-A — entrée carnet visible famille pour audit portail Hermione Granger."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(neville) = labo_eleve_prenom(eleves, "Neville") {
            let absences_neville = self.list_absences_eleve(etab, neville.id).await?;
            let neville_a_absence_nj = absences_neville
                .iter()
                .any(|a| a.statut == AbsenceStatut::ATraiter);
            if !neville_a_absence_nj {
                let absence_neville_ah = jour - Duration::days(3);
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: neville.id,
                    debut: absence_neville_ah,
                    fin: absence_neville_ah,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        const HARRY_RAPPEL_SMS_AH_TITRE: &str = "AUDIT AH-A: Rappel SMS Harry";
        if self.sms().is_ready() {
            let carnet_harry_ah = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), harry.id)
                .await?;
            if !carnet_harry_ah
                .iter()
                .any(|c| c.titre == HARRY_RAPPEL_SMS_AH_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: HARRY_RAPPEL_SMS_AH_TITRE.into(),
                    corps: "Seed labo lot AH-A — rappel SMS dry-run vers Lily Potter (outbox si API SMS prête)."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(ginny) = labo_eleve_prenom(eleves, "Ginny") {
            if self
                .get_eleve(etab, ginny.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        ginny.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        if let (Some(ron), Some(ginny)) = (
            labo_eleve_prenom(eleves, "Ron"),
            labo_eleve_prenom(eleves, "Ginny"),
        ) {
            let molly_user = ron.parent.user_id;
            let molly_foyer = ron.foyer_id;
            let enfants_molly = self
                .mes_enfants_for_parent(etab, molly_user, Role::Parent.as_str())
                .await?;
            let ids_molly: Vec<Uuid> = enfants_molly.iter().map(|e| e.eleve.id).collect();
            if !ids_molly.iter().any(|id| *id == ron.id) {
                self.link_parent(CreateLien {
                    etablissement_id: etab,
                    eleve_id: ron.id,
                    user_id: molly_user,
                    foyer_id: Some(molly_foyer),
                    lien: "mère".into(),
                    peut_voir_notes: ron.peut_voir_notes,
                    peut_voir_vs: ron.peut_voir_vs,
                    peut_justifier_absence: ron.peut_justifier,
                    peut_autoriser_sorties: ron.peut_justifier,
                    est_payeur: true,
                    part_paiement: Some(100),
                })
                .await?;
            }
            if !ids_molly.iter().any(|id| *id == ginny.id) {
                self.link_parent(CreateLien {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    user_id: molly_user,
                    foyer_id: Some(molly_foyer),
                    lien: "mère".into(),
                    peut_voir_notes: ginny.peut_voir_notes,
                    peut_voir_vs: ginny.peut_voir_vs,
                    peut_justifier_absence: ginny.peut_justifier,
                    peut_autoriser_sorties: ginny.peut_justifier,
                    est_payeur: true,
                    part_paiement: Some(100),
                })
                .await?;
            }
            if let Some(ginny_idx) = eleves
                .iter()
                .position(|e| e.prenom.eq_ignore_ascii_case("Ginny"))
            {
                eleves[ginny_idx].parent = ron.parent.clone();
                eleves[ginny_idx].foyer_id = molly_foyer;
            }
        }

        // --- Bot AI-A lot I : absence NJ Cho, carnet Luna (si pas déjà AH-A), renfort justif Harry
        // jour J a_traiter, note Cedric (matière distincte), photo trombi Seamus.
        if let Some(cho) = labo_eleve_prenom(eleves, "Cho") {
            let absences_cho = self.list_absences_eleve(etab, cho.id).await?;
            let cho_a_absence_nj = absences_cho
                .iter()
                .any(|a| a.statut == AbsenceStatut::ATraiter);
            if !cho_a_absence_nj {
                let absence_cho_ai = jour - Duration::days(2);
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: cho.id,
                    debut: absence_cho_ai,
                    fin: absence_cho_ai,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            const LUNA_CARNET_AH_PREFIX: &str = "AUDIT AH-A:";
            const LUNA_CARNET_AI_TITRE: &str = "AUDIT AI-A: Encouragements Luna (famille)";
            let carnet_luna = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), luna.id)
                .await?;
            let luna_deja_carnet_ah = carnet_luna
                .iter()
                .any(|c| c.titre.starts_with(LUNA_CARNET_AH_PREFIX));
            if !luna_deja_carnet_ah
                && !carnet_luna.iter().any(|c| c.titre == LUNA_CARNET_AI_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: luna.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: LUNA_CARNET_AI_TITRE.into(),
                    corps: "Seed labo lot AI-A — entrée carnet visible famille pour audit portail Luna Lovegood."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        const JUSTIF_AUDIT_MOTIF_AI: &str = "maladie";
        let absence_justif_jour_ai = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_justif_jour_ai)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_justif_jour_ai,
                fin: absence_justif_jour_ai,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }
        let absences_harry_ai = self.list_absences_eleve(etab, harry.id).await?;
        if let Some(abs_justif_ai) = absences_harry_ai
            .iter()
            .find(|a| a.debut == absence_justif_jour_ai && a.fin == absence_justif_jour_ai)
        {
            if abs_justif_ai.statut == AbsenceStatut::ATraiter
                && abs_justif_ai.motif_justif.as_deref() != Some(JUSTIF_AUDIT_MOTIF_AI)
            {
                self.justifier_absence_for_actor(
                    etab,
                    harry.parent.user_id,
                    Role::Parent.as_str(),
                    abs_justif_ai.id,
                    JUSTIF_AUDIT_MOTIF_AI.into(),
                )
                .await?;
            }
            let code_justif_ai = format!("JUSTIF_ABSENCE_{}", abs_justif_ai.id);
            let docs_harry_ai = self
                .list_documents_meta_eleve_for_actor(
                    etab,
                    administratif.user_id,
                    role_admini,
                    harry.id,
                )
                .await?;
            if !docs_harry_ai
                .iter()
                .any(|(d, _)| d.code.as_deref() == Some(code_justif_ai.as_str()))
            {
                let _ = self
                    .deposer_document_with_contenu(
                        domain_eleve::DeposerDocument {
                            etablissement_id: etab,
                            eleve_id: harry.id,
                            tiroir: DocTiroir::Famille,
                            titre: format!("Justificatif absence {code_justif_ai}"),
                            code: Some(code_justif_ai),
                            mime_type: Some("application/pdf".into()),
                            storage_key: None,
                            confidentialite: DocConfidentialite::Standard,
                            actor_user_id: harry.parent.user_id,
                        },
                        Some(PDF_JUSTIF_MINIMAL),
                    )
                    .await;
            }
        }

        if let Some(cedric) = labo_eleve_prenom(eleves, "Cedric") {
            const CEDRIC_NOTE_AI_LIB: &str = "Interro Potions lab (Cedric AI-A)";
            if !self
                .notes_for_eleve(etab, cedric.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(CEDRIC_NOTE_AI_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: cedric.id,
                    matiere_id: potions.id,
                    valeur: 12.5,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_rogue.user_id,
                    libelle: Some(CEDRIC_NOTE_AI_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            if self
                .get_eleve(etab, seamus.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        seamus.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        // --- Bot AJ-A lot J : absence NJ Ginny (peut_voir_vs=false → VS staff), note Ron
        // (peut_voir_notes=false → note staff), ré-ensure dispense EPS Harry, photo trombi Cedric,
        // ré-ensure facture émise foyer Draco.
        if let Some(ginny) = labo_eleve_prenom(eleves, "Ginny") {
            let absences_ginny = self.list_absences_eleve(etab, ginny.id).await?;
            let ginny_a_absence_nj = absences_ginny
                .iter()
                .any(|a| a.statut == AbsenceStatut::ATraiter);
            if !ginny_a_absence_nj {
                let absence_ginny_aj = jour - Duration::days(4);
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    debut: absence_ginny_aj,
                    fin: absence_ginny_aj,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        if let Some(ron) = labo_eleve_prenom(eleves, "Ron") {
            const RON_NOTE_AJ_LIB: &str = "Interro Mathématiques lab (Ron AJ-A)";
            if !self
                .notes_for_eleve(etab, ron.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(RON_NOTE_AJ_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: ron.id,
                    matiere_id: math.id,
                    valeur: 11.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(RON_NOTE_AJ_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        let fin_as_eps_aj = NaiveDate::from_ymd_opt(debut_as.year() + 1, 7, 5)
            .unwrap_or_else(|| debut_as + Duration::days(300));
        let dispenses_harry_aj = self
            .list_dispenses_vs_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
                true,
            )
            .await?;
        if !dispenses_harry_aj.iter().any(|d| {
            d.motif == DispenseVsMotif::Eps && d.statut == DispenseVsStatut::Active
        }) {
            self.poser_dispense_vs_for_actor(
                Role::Cpe.as_str(),
                PoserDispenseVs {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    motif: "eps".into(),
                    debut: debut_as,
                    fin: fin_as_eps_aj,
                    commentaire: "Seed audit — dispense EPS (asthme, certificat médical).".into(),
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }

        if let Some(cedric) = labo_eleve_prenom(eleves, "Cedric") {
            if self
                .get_eleve(etab, cedric.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        cedric.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        let fac_draco_numero_aj = format!("LAB-DRACO-{etab_short}");
        let factures_draco_aj = self
            .list_factures_for_actor(etab, compta.user_id, role_compta)
            .await?;
        let facture_draco_aj = if let Some(f) = factures_draco_aj
            .iter()
            .find(|f| f.foyer_id == draco.foyer_id && f.numero == fac_draco_numero_aj)
        {
            f.clone()
        } else {
            let (f, _lignes) = self
                .creer_facture_for_actor(
                    role_compta,
                    CreerFacture {
                        etablissement_id: etab,
                        foyer_id: draco.foyer_id,
                        numero: fac_draco_numero_aj.clone(),
                        libelle: "Fournitures — seed audit Draco".into(),
                        nature: None,
                        annee_scolaire: Some(annee_scolaire_de(jour)),
                        lignes: vec![LigneFactureCmd {
                            eleve_id: Some(draco.id),
                            libelle: "Cahier de correspondance".into(),
                            quantite: 1,
                            montant_unitaire_centimes: 350,
                        }],
                        actor_user_id: compta.user_id,
                    },
                )
                .await?;
            f
        };
        if facture_draco_aj.statut == FactureStatut::Brouillon {
            let _ = self
                .emettre_facture_for_actor(
                    etab,
                    facture_draco_aj.id,
                    compta.user_id,
                    role_compta,
                )
                .await?;
        }

        // --- Bot AK-A lot K : ré-ensure lot J Ginny/Ron, note Hermione (Potions), photo trombi Cho,
        // 2ᵉ message fil CPE Harry, ré-ensure facture LAB-HARRY-2 (reste à payer > 0).
        if let Some(ginny) = labo_eleve_prenom(eleves, "Ginny") {
            let absences_ginny_ak = self.list_absences_eleve(etab, ginny.id).await?;
            let ginny_a_absence_nj = absences_ginny_ak
                .iter()
                .any(|a| a.statut == AbsenceStatut::ATraiter);
            if !ginny_a_absence_nj {
                let absence_ginny_ak = jour - Duration::days(4);
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    debut: absence_ginny_ak,
                    fin: absence_ginny_ak,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        if let Some(ron) = labo_eleve_prenom(eleves, "Ron") {
            const RON_NOTE_AJ_LIB: &str = "Interro Mathématiques lab (Ron AJ-A)";
            if !self
                .notes_for_eleve(etab, ron.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(RON_NOTE_AJ_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: ron.id,
                    matiere_id: math.id,
                    valeur: 11.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(RON_NOTE_AJ_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            const HERMIONE_NOTE_AK_LIB: &str = "Interro Potions lab (Hermione AK-A)";
            if !self
                .notes_for_eleve(etab, hermione.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(HERMIONE_NOTE_AK_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: hermione.id,
                    matiere_id: potions.id,
                    valeur: 19.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(2),
                    auteur_user_id: prof_rogue.user_id,
                    libelle: Some(HERMIONE_NOTE_AK_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(cho) = labo_eleve_prenom(eleves, "Cho") {
            if self
                .get_eleve(etab, cho.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        cho.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        let (mut facture_h2_ak, _, _, enc_h2_ak, _, _, _) = self
            .get_facture_for_actor(etab, compta.user_id, role_compta, facture_harry_2_id)
            .await?;
        if facture_h2_ak.statut == FactureStatut::Brouillon {
            facture_h2_ak = self
                .emettre_facture_for_actor(etab, facture_harry_2_id, compta.user_id, role_compta)
                .await?;
        }
        let reste_h2_ak = facture_h2_ak
            .total_centimes
            .saturating_sub(enc_h2_ak);
        debug_assert!(
            reste_h2_ak > 0 || facture_h2_ak.total_centimes == 0,
            "LAB-HARRY-2 seed audit : reste à payer attendu > 0"
        );

        // --- Messagerie CPE → parent Harry (fil audit + 2 messages si API) — sans signature docs
        const FIL_CPE_AUDIT_SUJET: &str = "AUDIT: Point conduite Harry";
        const FIL_CPE_AK_MSG2: &str =
            "Merci de nous indiquer vos disponibilités pour un entretien téléphonique la semaine prochaine.";
        let role_cpe = Role::Cpe.as_str();
        let fils_harry = self
            .list_fils_for_actor(etab, cpe.user_id, role_cpe, Some(harry.id), false, None)
            .await?;
        let fil_cpe_audit_id = if let Some(f) = fils_harry
            .iter()
            .find(|f| f.sujet == FIL_CPE_AUDIT_SUJET)
        {
            f.id
        } else {
            let (fil, _msg) = self
                .ouvrir_fil_for_actor(
                    role_cpe,
                    OuvrirFil {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        sujet: FIL_CPE_AUDIT_SUJET.into(),
                        premier_message: "Bonjour, nous souhaitons échanger avec vous sur le comportement de Harry cette semaine.".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?;
            fil.id
        };
        let (fil_cpe_audit, msgs_cpe_audit) = self
            .list_messages_for_actor(etab, cpe.user_id, role_cpe, fil_cpe_audit_id)
            .await?;
        let deja_msg2 = msgs_cpe_audit
            .iter()
            .any(|m| m.message.corps == FIL_CPE_AK_MSG2);
        if !deja_msg2 && msgs_cpe_audit.len() < 2 {
            let _ = self
                .repondre_fil_for_actor(
                    role_cpe,
                    RepondreFil {
                        etablissement_id: etab,
                        fil_id: fil_cpe_audit.id,
                        corps: FIL_CPE_AK_MSG2.into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await;
        }

        // --- Bot AL-A lot L : note Luna (suppl.), absence NJ Seamus, photo trombi Luna,
        // dossier commission VS Harry ouvert (decision null), renfort Molly → Ron + Ginny.
        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            const LUNA_NOTE_AL_LIB: &str = "Interro Français lab (Luna AL-A)";
            if !self
                .notes_for_eleve(etab, luna.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(LUNA_NOTE_AL_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: luna.id,
                    matiere_id: _fr.id,
                    valeur: 14.5,
                    sur: 20.0,
                    date_note: jour - Duration::days(2),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(LUNA_NOTE_AL_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }

            if self
                .get_eleve(etab, luna.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        luna.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            let absences_seamus_al = self.list_absences_eleve(etab, seamus.id).await?;
            let seamus_a_absence_nj = absences_seamus_al
                .iter()
                .any(|a| a.statut == AbsenceStatut::ATraiter);
            if !seamus_a_absence_nj {
                let absence_seamus_al = jour - Duration::days(3);
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: seamus.id,
                    debut: absence_seamus_al,
                    fin: absence_seamus_al,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        const COMMISSION_MOTIFS_AL: &str = "Seed labo AL-A — commission Harry (dossier ouvert)";
        let role_cpe_commission = Role::Cpe.as_str();
        let dossiers_commission_harry = self
            .list_commissions_eleve_for_actor(
                etab,
                cpe.user_id,
                role_cpe_commission,
                harry.id,
            )
            .await?;
        let harry_a_dossier_commission_ouvert = dossiers_commission_harry
            .iter()
            .any(|d| d.decision.is_none());
        if !harry_a_dossier_commission_ouvert {
            let commission_al = self
                .creer_commission_for_actor(
                    role_cpe_commission,
                    CreerCommissionVs {
                        etablissement_id: etab,
                        jour: jour + Duration::days(14),
                        heure: NaiveTime::from_hms_opt(15, 30, 0).unwrap(),
                        objet: "Suivi conduite — Harry Potter (AL-A)".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?;
            let _ = self
                .convoquer_commission_for_actor(
                    role_cpe_commission,
                    ConvoquerCommission {
                        etablissement_id: etab,
                        commission_id: commission_al.id,
                        eleve_id: harry.id,
                        motifs: COMMISSION_MOTIFS_AL.into(),
                        note_interne: "Seed labo AL-A — dossier laissé ouvert (decision null).".into(),
                        sanction_id: None,
                        incident_id: incident_harry,
                        signale_famille: true,
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?;
        }

        if let (Some(ron), Some(ginny)) = (
            labo_eleve_prenom(eleves, "Ron"),
            labo_eleve_prenom(eleves, "Ginny"),
        ) {
            let molly_user = ron.parent.user_id;
            let molly_foyer = ron.foyer_id;
            let enfants_molly = self
                .mes_enfants_for_parent(etab, molly_user, Role::Parent.as_str())
                .await?;
            let ids_molly: Vec<Uuid> = enfants_molly.iter().map(|e| e.eleve.id).collect();
            if !ids_molly.iter().any(|id| *id == ron.id) {
                self.link_parent(CreateLien {
                    etablissement_id: etab,
                    eleve_id: ron.id,
                    user_id: molly_user,
                    foyer_id: Some(molly_foyer),
                    lien: "mère".into(),
                    peut_voir_notes: ron.peut_voir_notes,
                    peut_voir_vs: ron.peut_voir_vs,
                    peut_justifier_absence: ron.peut_justifier,
                    peut_autoriser_sorties: ron.peut_justifier,
                    est_payeur: true,
                    part_paiement: Some(100),
                })
                .await?;
            }
            if !ids_molly.iter().any(|id| *id == ginny.id) {
                self.link_parent(CreateLien {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    user_id: molly_user,
                    foyer_id: Some(molly_foyer),
                    lien: "mère".into(),
                    peut_voir_notes: ginny.peut_voir_notes,
                    peut_voir_vs: ginny.peut_voir_vs,
                    peut_justifier_absence: ginny.peut_justifier,
                    peut_autoriser_sorties: ginny.peut_justifier,
                    est_payeur: true,
                    part_paiement: Some(100),
                })
                .await?;
            }
            if let Some(ginny_idx) = eleves
                .iter()
                .position(|e| e.prenom.eq_ignore_ascii_case("Ginny"))
            {
                eleves[ginny_idx].parent = ron.parent.clone();
                eleves[ginny_idx].foyer_id = molly_foyer;
            }
        }

        // --- Bot AM-A lot M : carnet Cedric (Diggory) si pas d’entrée récente visible famille,
        // note Cho, photo trombi Cedric, PAI Harry actif non signé, facture émise foyer Draco.
        if let Some(cedric) = labo_eleve_prenom(eleves, "Cedric") {
            const CEDRIC_CARNET_RECENT_JOURS: i64 = 60;
            let seuil_carnet_recent = jour - Duration::days(CEDRIC_CARNET_RECENT_JOURS);
            let carnet_cedric_am = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), cedric.id)
                .await?;
            let cedric_a_carnet_recent = carnet_cedric_am.iter().any(|c| {
                c.visible_famille && c.date_entree >= seuil_carnet_recent
            });
            if !cedric_a_carnet_recent {
                const CEDRIC_CARNET_AM_TITRE: &str = "AUDIT AM-A: Suivi positif Cedric (famille)";
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: cedric.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: CEDRIC_CARNET_AM_TITRE.into(),
                    corps: "Seed labo lot AM-A — entrée carnet visible famille pour audit portail Cedric Diggory."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }

            if self
                .get_eleve(etab, cedric.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        cedric.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        if let Some(cho) = labo_eleve_prenom(eleves, "Cho") {
            const CHO_NOTE_LIB: &str = "Interro Mathématiques lab (Cho)";
            if !self
                .notes_for_eleve(etab, cho.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(CHO_NOTE_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: cho.id,
                    matiere_id: math.id,
                    valeur: 14.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(CHO_NOTE_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        let fin_as_pai_am = NaiveDate::from_ymd_opt(debut_as.year() + 1, 7, 5)
            .unwrap_or_else(|| debut_as + Duration::days(300));
        match self
            .get_pai_for_actor(
                etab,
                inf.user_id,
                Role::Infirmerie.as_str(),
                harry.id,
            )
            .await?
        {
            None => {
                self.upsert_pai_for_actor(
                    Role::Infirmerie.as_str(),
                    UpsertPai {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        motif: "Asthme".into(),
                        mesures_accueil: "Ventoline à l'infirmerie avant EPS".into(),
                        protocole_urgence: "Appeler le SAMU en cas de crise sévère".into(),
                        note_interne: "Antécédent hospitalisation 2024".into(),
                        date_debut: debut_as,
                        date_fin: Some(fin_as_pai_am),
                        actif: true,
                        actor_user_id: inf.user_id,
                    },
                )
                .await?;
            }
            Some(pai) if pai.signe_famille_at.is_some() || !pai.actif => {
                self.upsert_pai_for_actor(
                    Role::Infirmerie.as_str(),
                    UpsertPai {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        motif: pai.motif,
                        mesures_accueil: pai.mesures_accueil,
                        protocole_urgence: pai.protocole_urgence,
                        note_interne: pai.note_interne,
                        date_debut: pai.date_debut,
                        date_fin: pai.date_fin,
                        actif: true,
                        actor_user_id: inf.user_id,
                    },
                )
                .await?;
            }
            Some(_) => {}
        }

        let fac_draco_numero_am = format!("LAB-DRACO-{etab_short}");
        let factures_draco_am = self
            .list_factures_for_actor(etab, compta.user_id, role_compta)
            .await?;
        let facture_draco_am = if let Some(f) = factures_draco_am
            .iter()
            .find(|f| f.foyer_id == draco.foyer_id && f.numero == fac_draco_numero_am)
        {
            f.clone()
        } else {
            let (f, _lignes) = self
                .creer_facture_for_actor(
                    role_compta,
                    CreerFacture {
                        etablissement_id: etab,
                        foyer_id: draco.foyer_id,
                        numero: fac_draco_numero_am.clone(),
                        libelle: "Fournitures — seed audit Draco".into(),
                        nature: None,
                        annee_scolaire: Some(annee_scolaire_de(jour)),
                        lignes: vec![LigneFactureCmd {
                            eleve_id: Some(draco.id),
                            libelle: "Cahier de correspondance".into(),
                            quantite: 1,
                            montant_unitaire_centimes: 350,
                        }],
                        actor_user_id: compta.user_id,
                    },
                )
                .await?;
            f
        };
        if facture_draco_am.statut == FactureStatut::Brouillon {
            let _ = self
                .emettre_facture_for_actor(
                    etab,
                    facture_draco_am.id,
                    compta.user_id,
                    role_compta,
                )
                .await?;
        }

        // --- Bot AN-A lot N : note Seamus (Finnigan), absence NJ Luna (Lovegood), photo trombi Hermione,
        // ré-ensure CHARTE_LAB + REGLEMENT_LAB Harry, renfort Molly → Ron + Ginny dans mes_enfants.
        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            const SEAMUS_NOTE_AN_LIB: &str = "Interro Français lab (Seamus AN-A)";
            if !self
                .notes_for_eleve(etab, seamus.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(SEAMUS_NOTE_AN_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: seamus.id,
                    matiere_id: _fr.id,
                    valeur: 12.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(SEAMUS_NOTE_AN_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            let absences_luna_an = self.list_absences_eleve(etab, luna.id).await?;
            let luna_a_absence_nj = absences_luna_an
                .iter()
                .any(|a| a.statut == AbsenceStatut::ATraiter);
            if !luna_a_absence_nj {
                let absence_luna_an = jour - Duration::days(5);
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: luna.id,
                    debut: absence_luna_an,
                    fin: absence_luna_an,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            if self
                .get_eleve(etab, hermione.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        hermione.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        let docs_an_ensure = self
            .list_docs_a_signer_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
                false,
            )
            .await?;
        if !docs_an_ensure.iter().any(|d| d.code == "REGLEMENT_LAB") {
            self.publier_doc_a_signer_for_actor(
                Role::Cpe.as_str(),
                PublierDocASigner {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    code: "REGLEMENT_LAB".into(),
                    titre: "Règlement intérieur — signature requise".into(),
                    corps: "Merci de prendre connaissance du règlement et de le signer en ligne.".into(),
                    annee_scolaire: annee.clone(),
                    echeance: Some(jour + Duration::days(30)),
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }
        if !docs_an_ensure.iter().any(|d| d.code == "CHARTE_LAB") {
            self.publier_doc_a_signer_for_actor(
                Role::Cpe.as_str(),
                PublierDocASigner {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    code: "CHARTE_LAB".into(),
                    titre: "Charte numérique — signature requise".into(),
                    corps: "Merci de lire et signer la charte d'usage des outils numériques.".into(),
                    annee_scolaire: annee.clone(),
                    echeance: Some(jour + Duration::days(21)),
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }

        if let (Some(ron), Some(ginny)) = (
            labo_eleve_prenom(eleves, "Ron"),
            labo_eleve_prenom(eleves, "Ginny"),
        ) {
            let molly_user = ron.parent.user_id;
            let molly_foyer = ron.foyer_id;
            let enfants_molly = self
                .mes_enfants_for_parent(etab, molly_user, Role::Parent.as_str())
                .await?;
            let ids_molly: Vec<Uuid> = enfants_molly.iter().map(|e| e.eleve.id).collect();
            if !ids_molly.iter().any(|id| *id == ron.id) {
                self.link_parent(CreateLien {
                    etablissement_id: etab,
                    eleve_id: ron.id,
                    user_id: molly_user,
                    foyer_id: Some(molly_foyer),
                    lien: "mère".into(),
                    peut_voir_notes: ron.peut_voir_notes,
                    peut_voir_vs: ron.peut_voir_vs,
                    peut_justifier_absence: ron.peut_justifier,
                    peut_autoriser_sorties: ron.peut_justifier,
                    est_payeur: true,
                    part_paiement: Some(100),
                })
                .await?;
            }
            if !ids_molly.iter().any(|id| *id == ginny.id) {
                self.link_parent(CreateLien {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    user_id: molly_user,
                    foyer_id: Some(molly_foyer),
                    lien: "mère".into(),
                    peut_voir_notes: ginny.peut_voir_notes,
                    peut_voir_vs: ginny.peut_voir_vs,
                    peut_justifier_absence: ginny.peut_justifier,
                    peut_autoriser_sorties: ginny.peut_justifier,
                    est_payeur: true,
                    part_paiement: Some(100),
                })
                .await?;
            }
            if let Some(ginny_idx) = eleves
                .iter()
                .position(|e| e.prenom.eq_ignore_ascii_case("Ginny"))
            {
                eleves[ginny_idx].parent = ron.parent.clone();
                eleves[ginny_idx].foyer_id = molly_foyer;
            }
        }

        // --- Bot AO-A lot O (zone exclusive AO-A) : note Neville, carnet Ginny (famille),
        // photo trombi Seamus (Finnigan), renfort justif Harry jour J + PJ, menus lun–ven publiés.
        if let Some(neville) = labo_eleve_prenom(eleves, "Neville") {
            const NEVILLE_NOTE_AO_LIB: &str = "Interro Français lab (Neville AO-A)";
            let notes_neville = self.notes_for_eleve(etab, neville.id).await?;
            if !notes_neville
                .iter()
                .any(|n| n.libelle.as_deref() == Some(NEVILLE_NOTE_AO_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: neville.id,
                    matiere_id: _fr.id,
                    valeur: 11.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(NEVILLE_NOTE_AO_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(ginny) = labo_eleve_prenom(eleves, "Ginny") {
            const GINNY_CARNET_AO_TITRE: &str = "AUDIT AO-A: Encouragements Ginny (famille)";
            let carnet_ginny = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), ginny.id)
                .await?;
            if !carnet_ginny
                .iter()
                .any(|c| c.titre == GINNY_CARNET_AO_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: GINNY_CARNET_AO_TITRE.into(),
                    corps: "Seed labo lot AO-A — entrée carnet visible_famille (Molly sans notes VS ; carnet portail OK)."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            if self
                .get_eleve(etab, seamus.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        seamus.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        const JUSTIF_AUDIT_MOTIF_AO: &str = "maladie";
        let absence_justif_jour_ao = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_justif_jour_ao)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_justif_jour_ao,
                fin: absence_justif_jour_ao,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }
        let absences_harry_ao = self.list_absences_eleve(etab, harry.id).await?;
        if let Some(abs_justif_ao) = absences_harry_ao
            .iter()
            .find(|a| a.debut == absence_justif_jour_ao && a.fin == absence_justif_jour_ao)
        {
            if abs_justif_ao.statut == AbsenceStatut::ATraiter
                && abs_justif_ao.motif_justif.as_deref() != Some(JUSTIF_AUDIT_MOTIF_AO)
            {
                self.justifier_absence_for_actor(
                    etab,
                    harry.parent.user_id,
                    Role::Parent.as_str(),
                    abs_justif_ao.id,
                    JUSTIF_AUDIT_MOTIF_AO.into(),
                )
                .await?;
            }
            let code_justif_ao = format!("JUSTIF_ABSENCE_{}", abs_justif_ao.id);
            let docs_harry_ao = self
                .list_documents_meta_eleve_for_actor(
                    etab,
                    administratif.user_id,
                    role_admini,
                    harry.id,
                )
                .await?;
            let doc_justif_ao = if let Some(found) = docs_harry_ao
                .iter()
                .find(|(d, _)| d.code.as_deref() == Some(code_justif_ao.as_str()))
            {
                found.0.clone()
            } else {
                self.deposer_document_with_contenu(
                    domain_eleve::DeposerDocument {
                        etablissement_id: etab,
                        eleve_id: harry.id,
                        tiroir: DocTiroir::Famille,
                        titre: format!("Justificatif absence {code_justif_ao}"),
                        code: Some(code_justif_ao.clone()),
                        mime_type: Some("application/pdf".into()),
                        storage_key: None,
                        confidentialite: DocConfidentialite::Standard,
                        actor_user_id: harry.parent.user_id,
                    },
                    Some(PDF_JUSTIF_MINIMAL),
                )
                .await?
            };
            let pdf_ok_ao = self
                .telecharger_document_bytes_for_actor(
                    etab,
                    administratif.user_id,
                    role_admini,
                    doc_justif_ao.id,
                )
                .await
                .map(|(_, b)| !b.is_empty() && b.starts_with(b"%PDF"))
                .unwrap_or(false);
            if !pdf_ok_ao {
                if let Some(key) = doc_justif_ao.storage_key.as_ref() {
                    let _ = self.objects().put(key, PDF_JUSTIF_MINIMAL);
                } else {
                    let _ = self
                        .deposer_document_with_contenu(
                            domain_eleve::DeposerDocument {
                                etablissement_id: etab,
                                eleve_id: harry.id,
                                tiroir: DocTiroir::Famille,
                                titre: format!("Justificatif absence {code_justif_ao}"),
                                code: Some(code_justif_ao),
                                mime_type: Some("application/pdf".into()),
                                storage_key: None,
                                confidentialite: DocConfidentialite::Standard,
                                actor_user_id: harry.parent.user_id,
                            },
                            Some(PDF_JUSTIF_MINIMAL),
                        )
                        .await;
                }
            }
        }

        const MENUS_AO_RENFORCE: [(&str, &str, &str); 5] = [
            ("Velouté", "Gratin dauphinois", "Compote"),
            ("Salade", "Poulet rôti", "Yaourt"),
            ("Soupe", "Poisson pané", "Fruit"),
            ("Taboulé", "Steak haché", "Gâteau"),
            ("Carottes râpées", "Lasagnes", "Fromage"),
        ];
        for (i, menu) in MENUS_AO_RENFORCE.iter().enumerate() {
            let j_menu = debut_sem + Duration::days(i as i64);
            let menus_jour = self
                .list_menus_cantine_for_actor(
                    etab,
                    administratif.user_id,
                    role_admini,
                    j_menu,
                    j_menu,
                )
                .await?;
            let manque_plat = !menus_jour.iter().any(|m| m.plat == menu.1);
            let brouillon = menus_jour.iter().any(|m| !m.publie);
            if manque_plat || brouillon {
                self.upsert_menu_cantine_for_actor(
                    role_admini,
                    UpsertMenuCantine {
                        etablissement_id: etab,
                        jour: j_menu,
                        entree: menu.0.into(),
                        plat: menu.1.into(),
                        dessert: menu.2.into(),
                        notes: if manque_plat && harry.classe == "4B" {
                            "Menu seed audit 4B".into()
                        } else {
                            String::new()
                        },
                        publie: true,
                        actor_user_id: administratif.user_id,
                    },
                )
                .await?;
            }
        }

        // --- Bot AP-A lot P (zone exclusive AP-A) : absence NJ Hermione (Granger),
        // carnet Cedric (Diggory) récent visible famille, photo trombi Luna (Lovegood),
        // ré-ensure bulletin T0 publié Harry, ré-ensure mandat SEPA actif foyer Harry.
        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            let absences_hermione_ap = self.list_absences_eleve(etab, hermione.id).await?;
            let hermione_a_absence_nj = absences_hermione_ap
                .iter()
                .any(|a| a.statut == AbsenceStatut::ATraiter);
            if !hermione_a_absence_nj {
                let absence_hermione_ap = jour - Duration::days(1);
                let motifs = self.list_motifs_absence(etab).await?;
                let motif_code = motifs
                    .iter()
                    .find(|m| m.code.eq_ignore_ascii_case("defaut"))
                    .or_else(|| motifs.first())
                    .map(|m| m.code.clone())
                    .unwrap_or_else(|| "defaut".into());
                self.declare_absence(DeclareAbsence {
                    etablissement_id: etab,
                    eleve_id: hermione.id,
                    debut: absence_hermione_ap,
                    fin: absence_hermione_ap,
                    motif: motif_code,
                    actor_user_id: Some(cpe.user_id.to_string()),
                })
                .await?;
            }
        }

        if let Some(cedric) = labo_eleve_prenom(eleves, "Cedric") {
            const CEDRIC_CARNET_RECENT_JOURS_AP: i64 = 60;
            let seuil_carnet_recent_ap = jour - Duration::days(CEDRIC_CARNET_RECENT_JOURS_AP);
            let carnet_cedric_ap = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), cedric.id)
                .await?;
            let cedric_a_carnet_recent = carnet_cedric_ap.iter().any(|c| {
                c.visible_famille && c.date_entree >= seuil_carnet_recent_ap
            });
            if !cedric_a_carnet_recent {
                const CEDRIC_CARNET_AP_TITRE: &str = "AUDIT AP-A: Suivi positif Cedric (famille)";
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: cedric.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: CEDRIC_CARNET_AP_TITRE.into(),
                    corps: "Seed labo lot AP-A — entrée carnet visible famille pour audit portail Cedric Diggory."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            if self
                .get_eleve(etab, luna.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        luna.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        let docs_harry_t0_ap = self
            .list_documents_meta_eleve_for_actor(
                etab,
                administratif.user_id,
                role_admini,
                harry.id,
            )
            .await?;
        if !docs_harry_t0_ap
            .iter()
            .any(|(d, _)| d.code.as_deref() == Some(bulletin_t0_code.as_str()))
        {
            let _ = self
                .publier_bulletin_for_actor(
                    Role::Direction.as_str(),
                    etab,
                    harry.id,
                    periode_t0.id,
                    admin.user_id,
                )
                .await;
        }

        let rum_harry_sepa_ap = format!("LAB-HARRY-SEPA-{etab_short}");
        let mandats_sepa_ap = self
            .list_mandats_sepa_for_actor(etab, compta.user_id, role_compta)
            .await?;
        let harry_mandat_seed_ap = mandats_sepa_ap.iter().any(|m| {
            m.foyer_id == harry.foyer_id && m.rum == rum_harry_sepa_ap
        });
        let harry_mandat_actif_ap = mandats_sepa_ap.iter().any(|m| {
            m.foyer_id == harry.foyer_id && m.statut == MandatSepaStatut::Actif
        });
        if !harry_mandat_seed_ap && !harry_mandat_actif_ap {
            let _ = self
                .creer_mandat_sepa_for_actor(
                    role_compta,
                    compta.user_id,
                    CreerMandatSepa {
                        etablissement_id: etab,
                        foyer_id: harry.foyer_id,
                        rum: rum_harry_sepa_ap,
                        iban: "FR7630006000011234567890189".into(),
                        bic: Some("AGRIFRPP".into()),
                        titulaire: "Lily Potter".into(),
                        signe_at: Utc::now(),
                        actor_user_id: compta.user_id,
                    },
                )
                .await;
        }

        // --- Bot AQ-A lot Q (zone exclusive AQ-A) : note Draco, carnet Ron (famille),
        // photo trombi Cho, alerte CPE urgente Harry, ré-ensure LAB-HARRY* reste > 0.
        const DRACO_NOTE_AQ_LIB: &str = "Interro Mathématiques lab (Draco AQ-A)";
        if !self
            .notes_for_eleve(etab, draco.id)
            .await?
            .iter()
            .any(|n| n.libelle.as_deref() == Some(DRACO_NOTE_AQ_LIB))
        {
            self.saisir_note(SaisirNote {
                etablissement_id: etab,
                eleve_id: draco.id,
                matiere_id: math.id,
                valeur: 10.5,
                sur: 20.0,
                date_note: jour - Duration::days(2),
                auteur_user_id: prof_mcg.user_id,
                libelle: Some(DRACO_NOTE_AQ_LIB.into()),
                devoir_id: None,
                periode_id: Some(periode.id),
            })
            .await?;
        }

        if let Some(ron) = labo_eleve_prenom(eleves, "Ron") {
            const RON_CARNET_AQ_TITRE: &str = "AUDIT AQ-A: Encouragements Ron (famille)";
            let carnet_ron = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), ron.id)
                .await?;
            if !carnet_ron
                .iter()
                .any(|c| c.titre == RON_CARNET_AQ_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: ron.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: RON_CARNET_AQ_TITRE.into(),
                    corps: "Seed labo lot AQ-A — entrée carnet visible famille pour audit portail Ron Weasley."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(cho) = labo_eleve_prenom(eleves, "Cho") {
            if self
                .get_eleve(etab, cho.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        cho.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        let alertes_harry_aq = self
            .list_alertes_cpe_eleve_for_actor(
                etab,
                cpe.user_id,
                Role::Cpe.as_str(),
                harry.id,
            )
            .await?;
        let harry_a_urgent_aq = alertes_harry_aq.iter().any(|a| {
            a.severite == AlerteCpeSeverite::Urgent && a.statut == AlerteCpeStatut::Ouverte
        });
        if !harry_a_urgent_aq {
            const HARRY_ALERTE_AQ_TITRE: &str = "AUDIT AQ-A: Alerte CPE urgente Harry";
            self.creer_alerte_cpe_for_actor(
                Role::Cpe.as_str(),
                CreerAlerteCpe {
                    etablissement_id: etab,
                    eleve_id: harry.id,
                    titre: HARRY_ALERTE_AQ_TITRE.into(),
                    detail: "Seed labo lot AQ-A — severite urgent pour audit API / UI CPE.".into(),
                    severite: "urgent".into(),
                    visible_famille: true,
                    jour,
                    actor_user_id: cpe.user_id,
                },
            )
            .await?;
        }

        if facture_harry.statut == FactureStatut::Brouillon {
            facture_harry = self
                .emettre_facture_for_actor(
                    etab,
                    facture_harry.id,
                    compta.user_id,
                    role_compta,
                )
                .await?;
        }
        let (_, _, _, enc_harry_1_aq, _, _, _) = self
            .get_facture_for_actor(etab, compta.user_id, role_compta, facture_harry.id)
            .await?;
        let mut reste_lab_harry_aq = facture_harry
            .total_centimes
            .saturating_sub(enc_harry_1_aq);
        let (mut facture_h2_aq, _, _, enc_harry_2_aq, _, _, _) = self
            .get_facture_for_actor(etab, compta.user_id, role_compta, facture_harry_2_id)
            .await?;
        if facture_h2_aq.statut == FactureStatut::Brouillon {
            facture_h2_aq = self
                .emettre_facture_for_actor(
                    etab,
                    facture_harry_2_id,
                    compta.user_id,
                    role_compta,
                )
                .await?;
        }
        let reste_h2_aq = facture_h2_aq
            .total_centimes
            .saturating_sub(enc_harry_2_aq);
        if reste_lab_harry_aq == 0 && reste_h2_aq > 0 {
            reste_lab_harry_aq = reste_h2_aq;
        } else if reste_lab_harry_aq == 0 && reste_h2_aq == 0 && facture_h2_aq.total_centimes > 0 {
            reste_lab_harry_aq = facture_h2_aq.total_centimes;
        }
        debug_assert!(
            reste_lab_harry_aq > 0 || facture_harry.total_centimes == 0,
            "LAB-HARRY seed audit AQ-A : reste à payer attendu > 0 sur au moins une facture"
        );

        // --- Bot AR-A lot R (zone exclusive AR-A) : devoir Hermione, note Ginny, dispense ou PAI tip
        // Neville, photo trombi Cedric, ré-ensure au moins 1 commission le jour J (sans déclarer d’absence jour J).
        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            const HERMIONE_DEVOIR_AR_TITRE: &str = "DS Mathématiques lab (Hermione AR-A)";
            let devoirs_hermione_ar = self
                .list_devoirs_for_actor(
                    etab,
                    Role::Prof.as_str(),
                    periode.id,
                    Some(hermione.classe.as_str()),
                )
                .await?;
            if !devoirs_hermione_ar
                .iter()
                .any(|d| d.titre == HERMIONE_DEVOIR_AR_TITRE)
            {
                self.creer_devoir(CreerDevoir {
                    etablissement_id: etab,
                    periode_id: periode.id,
                    matiere_id: math.id,
                    classe: hermione.classe.clone(),
                    titre: HERMIONE_DEVOIR_AR_TITRE.into(),
                    date_devoir: jour + Duration::days(10),
                    coef: 1.0,
                    sur: 20.0,
                    actor_user_id: prof_mcg.user_id,
                })
                .await?;
            }
        }

        if let Some(ginny) = labo_eleve_prenom(eleves, "Ginny") {
            const GINNY_NOTE_AR_LIB: &str = "Interro Mathématiques lab (Ginny AR-A)";
            if !self
                .notes_for_eleve(etab, ginny.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(GINNY_NOTE_AR_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    matiere_id: math.id,
                    valeur: 14.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(2),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(GINNY_NOTE_AR_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(neville) = labo_eleve_prenom(eleves, "Neville") {
            const NEVILLE_PAI_AR_MOTIF: &str = "Allergie pollen (tip seed AR-A)";
            let fin_as_pai_ar = NaiveDate::from_ymd_opt(debut_as.year() + 1, 7, 5)
                .unwrap_or_else(|| debut_as + Duration::days(300));
            if self
                .get_pai_for_actor(
                    etab,
                    inf.user_id,
                    Role::Infirmerie.as_str(),
                    neville.id,
                )
                .await?
                .is_none()
            {
                self.upsert_pai_for_actor(
                    Role::Infirmerie.as_str(),
                    UpsertPai {
                        etablissement_id: etab,
                        eleve_id: neville.id,
                        motif: NEVILLE_PAI_AR_MOTIF.into(),
                        mesures_accueil: "Antihistaminique à l’infirmerie si symptômes".into(),
                        protocole_urgence: "Contacter les secours si détresse respiratoire".into(),
                        note_interne: "Seed labo lot AR-A — PAI tip internat Neville.".into(),
                        date_debut: debut_as,
                        date_fin: Some(fin_as_pai_ar),
                        actif: true,
                        actor_user_id: inf.user_id,
                    },
                )
                .await?;
            } else {
                const NEVILLE_DISPENSE_AR_COMMENT: &str =
                    "Seed labo AR-A — dispense EPS (certificat médical tip).";
                let dispenses_neville_ar = self
                    .list_dispenses_vs_eleve_for_actor(
                        etab,
                        cpe.user_id,
                        Role::Cpe.as_str(),
                        neville.id,
                        true,
                    )
                    .await?;
                if !dispenses_neville_ar.iter().any(|d| {
                    d.commentaire == NEVILLE_DISPENSE_AR_COMMENT
                        && d.statut == DispenseVsStatut::Active
                }) {
                    self.poser_dispense_vs_for_actor(
                        Role::Cpe.as_str(),
                        PoserDispenseVs {
                            etablissement_id: etab,
                            eleve_id: neville.id,
                            motif: "eps".into(),
                            debut: debut_as,
                            fin: fin_as_pai_ar,
                            commentaire: NEVILLE_DISPENSE_AR_COMMENT.into(),
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await?;
                }
            }
        }

        if let Some(cedric) = labo_eleve_prenom(eleves, "Cedric") {
            if self
                .get_eleve(etab, cedric.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        cedric.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        const COMMISSION_MOTIFS_AR: &str = "Seed labo AR-A — commission jour J audit";
        let role_cpe_commission_ar = Role::Cpe.as_str();
        let commissions_jour_ar = self
            .list_commissions_for_actor(etab, role_cpe_commission_ar, jour)
            .await?;
        if commissions_jour_ar.is_empty() {
            let commission_ar = self
                .creer_commission_for_actor(
                    role_cpe_commission_ar,
                    CreerCommissionVs {
                        etablissement_id: etab,
                        jour,
                        heure: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
                        objet: "Suivi conduite — seed audit AR-A (jour J)".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?;
            if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
                let dossiers_hermione_ar = self
                    .list_commissions_eleve_for_actor(
                        etab,
                        cpe.user_id,
                        role_cpe_commission_ar,
                        hermione.id,
                    )
                    .await?;
                if !dossiers_hermione_ar
                    .iter()
                    .any(|d| d.motifs.contains(COMMISSION_MOTIFS_AR))
                {
                    let _ = self
                        .convoquer_commission_for_actor(
                            role_cpe_commission_ar,
                            ConvoquerCommission {
                                etablissement_id: etab,
                                commission_id: commission_ar.id,
                                eleve_id: hermione.id,
                                motifs: COMMISSION_MOTIFS_AR.into(),
                                note_interne: "Seed labo AR-A — dossier commission jour J (sans toucher absences jour J)."
                                    .into(),
                                sanction_id: None,
                                incident_id: None,
                                signale_famille: true,
                                actor_user_id: cpe.user_id,
                            },
                        )
                        .await;
                }
            }
        }

        // --- Bot AS-A lot S (zone exclusive AS-A) : note Luna, carnet Seamus (famille),
        // photo trombi Ron si vide, devoir Harry (titre AS) ; ré-ensure abs Harry jour J + commission jour J (sans wipe).
        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            const LUNA_NOTE_AS_LIB: &str = "Interro Français lab (Luna AS-A)";
            if !self
                .notes_for_eleve(etab, luna.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(LUNA_NOTE_AS_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: luna.id,
                    matiere_id: _fr.id,
                    valeur: 13.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(LUNA_NOTE_AS_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            const SEAMUS_CARNET_AS_TITRE: &str = "AUDIT AS-A: Encouragements Seamus (famille)";
            let carnet_seamus_as = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), seamus.id)
                .await?;
            if !carnet_seamus_as
                .iter()
                .any(|c| c.titre == SEAMUS_CARNET_AS_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: seamus.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: SEAMUS_CARNET_AS_TITRE.into(),
                    corps: "Seed labo lot AS-A — entrée carnet visible famille pour audit portail Seamus Finnigan."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(ron) = labo_eleve_prenom(eleves, "Ron") {
            if self
                .get_eleve(etab, ron.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        ron.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        const HARRY_DEVOIR_AS_TITRE: &str = "DS Potions lab (Harry AS-A)";
        let devoirs_harry_as = self
            .list_devoirs_for_actor(
                etab,
                Role::Prof.as_str(),
                periode.id,
                Some(harry.classe.as_str()),
            )
            .await?;
        if !devoirs_harry_as
            .iter()
            .any(|d| d.titre == HARRY_DEVOIR_AS_TITRE)
        {
            self.creer_devoir(CreerDevoir {
                etablissement_id: etab,
                periode_id: periode.id,
                matiere_id: potions.id,
                classe: harry.classe.clone(),
                titre: HARRY_DEVOIR_AS_TITRE.into(),
                date_devoir: jour + Duration::days(8),
                coef: 1.0,
                sur: 20.0,
                actor_user_id: prof_rogue.user_id,
            })
            .await?;
        }

        let absence_harry_jour_as = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_harry_jour_as)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_harry_jour_as,
                fin: absence_harry_jour_as,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }

        const COMMISSION_MOTIFS_AS_RENFORCE: &str = "Seed labo — commission Harry audit";
        let role_cpe_commission_as = Role::Cpe.as_str();
        let commissions_jour_as = self
            .list_commissions_for_actor(etab, role_cpe_commission_as, jour)
            .await?;
        let dossiers_commission_harry_as = self
            .list_commissions_eleve_for_actor(
                etab,
                cpe.user_id,
                role_cpe_commission_as,
                harry.id,
            )
            .await?;
        let harry_commission_jour_as = dossiers_commission_harry_as.iter().any(|d| {
            commissions_jour_as
                .iter()
                .any(|c| c.id == d.commission_id)
        });
        if !harry_commission_jour_as {
            let commission_as_id = if let Some(c) = commissions_jour_as.first() {
                c.id
            } else {
                self.creer_commission_for_actor(
                    role_cpe_commission_as,
                    CreerCommissionVs {
                        etablissement_id: etab,
                        jour,
                        heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                        objet: "Incidents répétés — Harry Potter (AS-A)".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?
                .id
            };
            if !dossiers_commission_harry_as
                .iter()
                .any(|d| d.commission_id == commission_as_id)
            {
                let _ = self
                    .convoquer_commission_for_actor(
                        role_cpe_commission_as,
                        ConvoquerCommission {
                            etablissement_id: etab,
                            commission_id: commission_as_id,
                            eleve_id: harry.id,
                            motifs: COMMISSION_MOTIFS_AS_RENFORCE.into(),
                            note_interne: "Seed labo AS-A — ré-ensure commission jour J (sans wipe absences)."
                                .into(),
                            sanction_id: None,
                            incident_id: incident_harry,
                            signale_famille: true,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
        }

        // --- Bot AT-A lot T (zone exclusive AT-A) : note Cho, carnet Hermione (famille),
        // photo trombi Ginny si vide, devoir Draco (titre AT) ; ré-ensure abs Harry jour J + commission jour J (sans wipe).
        if let Some(cho) = labo_eleve_prenom(eleves, "Cho") {
            const CHO_NOTE_AT_LIB: &str = "Interro Français lab (Cho AT-A)";
            if !self
                .notes_for_eleve(etab, cho.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(CHO_NOTE_AT_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: cho.id,
                    matiere_id: _fr.id,
                    valeur: 13.5,
                    sur: 20.0,
                    date_note: jour - Duration::days(2),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(CHO_NOTE_AT_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            const HERMIONE_CARNET_AT_TITRE: &str = "AUDIT AT-A: Encouragements Hermione (famille)";
            let carnet_hermione_at = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), hermione.id)
                .await?;
            if !carnet_hermione_at
                .iter()
                .any(|c| c.titre == HERMIONE_CARNET_AT_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: hermione.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: HERMIONE_CARNET_AT_TITRE.into(),
                    corps: "Seed labo lot AT-A — entrée carnet visible famille pour audit portail Hermione Granger."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(ginny) = labo_eleve_prenom(eleves, "Ginny") {
            if self
                .get_eleve(etab, ginny.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        ginny.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        const DRACO_DEVOIR_AT_TITRE: &str = "DS Mathématiques lab (Draco AT-A)";
        let devoirs_draco_at = self
            .list_devoirs_for_actor(
                etab,
                Role::Prof.as_str(),
                periode.id,
                Some(draco.classe.as_str()),
            )
            .await?;
        if !devoirs_draco_at
            .iter()
            .any(|d| d.titre == DRACO_DEVOIR_AT_TITRE)
        {
            self.creer_devoir(CreerDevoir {
                etablissement_id: etab,
                periode_id: periode.id,
                matiere_id: math.id,
                classe: draco.classe.clone(),
                titre: DRACO_DEVOIR_AT_TITRE.into(),
                date_devoir: jour + Duration::days(12),
                coef: 1.0,
                sur: 20.0,
                actor_user_id: prof_mcg.user_id,
            })
            .await?;
        }

        let absence_harry_jour_at = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_harry_jour_at)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_harry_jour_at,
                fin: absence_harry_jour_at,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }

        const COMMISSION_MOTIFS_AT_RENFORCE: &str = "Seed labo — commission Harry audit";
        let role_cpe_commission_at = Role::Cpe.as_str();
        let commissions_jour_at = self
            .list_commissions_for_actor(etab, role_cpe_commission_at, jour)
            .await?;
        let dossiers_commission_harry_at = self
            .list_commissions_eleve_for_actor(
                etab,
                cpe.user_id,
                role_cpe_commission_at,
                harry.id,
            )
            .await?;
        let harry_commission_jour_at = dossiers_commission_harry_at.iter().any(|d| {
            commissions_jour_at
                .iter()
                .any(|c| c.id == d.commission_id)
        });
        if !harry_commission_jour_at {
            let commission_at_id = if let Some(c) = commissions_jour_at.first() {
                c.id
            } else {
                self.creer_commission_for_actor(
                    role_cpe_commission_at,
                    CreerCommissionVs {
                        etablissement_id: etab,
                        jour,
                        heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                        objet: "Incidents répétés — Harry Potter (AT-A)".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?
                .id
            };
            if !dossiers_commission_harry_at
                .iter()
                .any(|d| d.commission_id == commission_at_id)
            {
                let _ = self
                    .convoquer_commission_for_actor(
                        role_cpe_commission_at,
                        ConvoquerCommission {
                            etablissement_id: etab,
                            commission_id: commission_at_id,
                            eleve_id: harry.id,
                            motifs: COMMISSION_MOTIFS_AT_RENFORCE.into(),
                            note_interne: "Seed labo AT-A — ré-ensure commission jour J (sans wipe absences)."
                                .into(),
                            sanction_id: None,
                            incident_id: incident_harry,
                            signale_famille: true,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
        }

        // --- Bot AU-A lot U (zone exclusive AU-A) : note Cedric, carnet Neville (famille),
        // photo trombi Hermione si vide, devoir Luna (titre AU) ; ré-ensure abs Harry jour J + commission jour J (sans wipe).
        if let Some(cedric) = labo_eleve_prenom(eleves, "Cedric") {
            const CEDRIC_NOTE_AU_LIB: &str = "Interro Français lab (Cedric AU-A)";
            if !self
                .notes_for_eleve(etab, cedric.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(CEDRIC_NOTE_AU_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: cedric.id,
                    matiere_id: _fr.id,
                    valeur: 14.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(2),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(CEDRIC_NOTE_AU_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(neville) = labo_eleve_prenom(eleves, "Neville") {
            const NEVILLE_CARNET_AU_TITRE: &str = "AUDIT AU-A: Encouragements Neville (famille)";
            let carnet_neville_au = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), neville.id)
                .await?;
            if !carnet_neville_au
                .iter()
                .any(|c| c.titre == NEVILLE_CARNET_AU_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: neville.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: NEVILLE_CARNET_AU_TITRE.into(),
                    corps: "Seed labo lot AU-A — entrée carnet visible famille pour audit portail Neville Londubat."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            if self
                .get_eleve(etab, hermione.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        hermione.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            const LUNA_DEVOIR_AU_TITRE: &str = "DS Français lab (Luna AU-A)";
            let devoirs_luna_au = self
                .list_devoirs_for_actor(
                    etab,
                    Role::Prof.as_str(),
                    periode.id,
                    Some(luna.classe.as_str()),
                )
                .await?;
            if !devoirs_luna_au
                .iter()
                .any(|d| d.titre == LUNA_DEVOIR_AU_TITRE)
            {
                self.creer_devoir(CreerDevoir {
                    etablissement_id: etab,
                    periode_id: periode.id,
                    matiere_id: _fr.id,
                    classe: luna.classe.clone(),
                    titre: LUNA_DEVOIR_AU_TITRE.into(),
                    date_devoir: jour + Duration::days(10),
                    coef: 1.0,
                    sur: 20.0,
                    actor_user_id: prof_mcg.user_id,
                })
                .await?;
            }
        }

        let absence_harry_jour_au = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_harry_jour_au)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_harry_jour_au,
                fin: absence_harry_jour_au,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }

        const COMMISSION_MOTIFS_AU_RENFORCE: &str = "Seed labo — commission Harry audit";
        let role_cpe_commission_au = Role::Cpe.as_str();
        let commissions_jour_au = self
            .list_commissions_for_actor(etab, role_cpe_commission_au, jour)
            .await?;
        let dossiers_commission_harry_au = self
            .list_commissions_eleve_for_actor(
                etab,
                cpe.user_id,
                role_cpe_commission_au,
                harry.id,
            )
            .await?;
        let harry_commission_jour_au = dossiers_commission_harry_au.iter().any(|d| {
            commissions_jour_au
                .iter()
                .any(|c| c.id == d.commission_id)
        });
        if !harry_commission_jour_au {
            let commission_au_id = if let Some(c) = commissions_jour_au.first() {
                c.id
            } else {
                self.creer_commission_for_actor(
                    role_cpe_commission_au,
                    CreerCommissionVs {
                        etablissement_id: etab,
                        jour,
                        heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                        objet: "Incidents répétés — Harry Potter (AU-A)".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?
                .id
            };
            if !dossiers_commission_harry_au
                .iter()
                .any(|d| d.commission_id == commission_au_id)
            {
                let _ = self
                    .convoquer_commission_for_actor(
                        role_cpe_commission_au,
                        ConvoquerCommission {
                            etablissement_id: etab,
                            commission_id: commission_au_id,
                            eleve_id: harry.id,
                            motifs: COMMISSION_MOTIFS_AU_RENFORCE.into(),
                            note_interne: "Seed labo AU-A — ré-ensure commission jour J (sans wipe absences)."
                                .into(),
                            sanction_id: None,
                            incident_id: incident_harry,
                            signale_famille: true,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
        }

        // --- Bot AV-A lot V (zone exclusive AV-A) : devoir Ginny, note Ron, photo trombi Seamus si vide,
        // carnet Cho (famille) ; ré-ensure abs Harry jour J + commission Harry jour J (sans wipe).
        if let Some(ginny) = labo_eleve_prenom(eleves, "Ginny") {
            const GINNY_DEVOIR_AV_TITRE: &str = "DS Potions lab (Ginny AV-A)";
            let devoirs_ginny_av = self
                .list_devoirs_for_actor(
                    etab,
                    Role::Prof.as_str(),
                    periode.id,
                    Some(ginny.classe.as_str()),
                )
                .await?;
            if !devoirs_ginny_av
                .iter()
                .any(|d| d.titre == GINNY_DEVOIR_AV_TITRE)
            {
                self.creer_devoir(CreerDevoir {
                    etablissement_id: etab,
                    periode_id: periode.id,
                    matiere_id: potions.id,
                    classe: ginny.classe.clone(),
                    titre: GINNY_DEVOIR_AV_TITRE.into(),
                    date_devoir: jour + Duration::days(9),
                    coef: 1.0,
                    sur: 20.0,
                    actor_user_id: prof_rogue.user_id,
                })
                .await?;
            }
        }

        if let Some(ron) = labo_eleve_prenom(eleves, "Ron") {
            const RON_NOTE_AV_LIB: &str = "Interro Potions lab (Ron AV-A)";
            if !self
                .notes_for_eleve(etab, ron.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(RON_NOTE_AV_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: ron.id,
                    matiere_id: potions.id,
                    valeur: 11.5,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_rogue.user_id,
                    libelle: Some(RON_NOTE_AV_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            if self
                .get_eleve(etab, seamus.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        seamus.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        if let Some(cho) = labo_eleve_prenom(eleves, "Cho") {
            const CHO_CARNET_AV_TITRE: &str = "AUDIT AV-A: Encouragements Cho (famille)";
            let carnet_cho_av = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), cho.id)
                .await?;
            if !carnet_cho_av
                .iter()
                .any(|c| c.titre == CHO_CARNET_AV_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: cho.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: CHO_CARNET_AV_TITRE.into(),
                    corps: "Seed labo lot AV-A — entrée carnet visible famille pour audit portail Cho Chang."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        let absence_harry_jour_av = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_harry_jour_av)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_harry_jour_av,
                fin: absence_harry_jour_av,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }

        const COMMISSION_MOTIFS_AV_RENFORCE: &str = "Seed labo — commission Harry audit";
        let role_cpe_commission_av = Role::Cpe.as_str();
        let commissions_jour_av = self
            .list_commissions_for_actor(etab, role_cpe_commission_av, jour)
            .await?;
        let dossiers_commission_harry_av = self
            .list_commissions_eleve_for_actor(
                etab,
                cpe.user_id,
                role_cpe_commission_av,
                harry.id,
            )
            .await?;
        let harry_commission_jour_av = dossiers_commission_harry_av.iter().any(|d| {
            commissions_jour_av
                .iter()
                .any(|c| c.id == d.commission_id)
        });
        if !harry_commission_jour_av {
            let commission_av_id = if let Some(c) = commissions_jour_av.first() {
                c.id
            } else {
                self.creer_commission_for_actor(
                    role_cpe_commission_av,
                    CreerCommissionVs {
                        etablissement_id: etab,
                        jour,
                        heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                        objet: "Incidents répétés — Harry Potter (AV-A)".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?
                .id
            };
            if !dossiers_commission_harry_av
                .iter()
                .any(|d| d.commission_id == commission_av_id)
            {
                let _ = self
                    .convoquer_commission_for_actor(
                        role_cpe_commission_av,
                        ConvoquerCommission {
                            etablissement_id: etab,
                            commission_id: commission_av_id,
                            eleve_id: harry.id,
                            motifs: COMMISSION_MOTIFS_AV_RENFORCE.into(),
                            note_interne: "Seed labo AV-A — ré-ensure commission jour J (sans wipe absences)."
                                .into(),
                            sanction_id: None,
                            incident_id: incident_harry,
                            signale_famille: true,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
        }

        // --- Bot AW-A lot W (zone exclusive AW-A) : note Hermione, carnet Draco (famille),
        // photo trombi Luna si vide, devoir Cedric ; ré-ensure abs Harry jour J + commission Harry jour J (sans wipe).
        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            const HERMIONE_NOTE_AW_LIB: &str = "Interro Français lab (Hermione AW-A)";
            if !self
                .notes_for_eleve(etab, hermione.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(HERMIONE_NOTE_AW_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: hermione.id,
                    matiere_id: _fr.id,
                    valeur: 17.5,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(HERMIONE_NOTE_AW_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        const DRACO_CARNET_AW_TITRE: &str = "AUDIT AW-A: Suivi Draco (famille)";
        let carnet_draco_aw = self
            .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), draco.id)
            .await?;
        if !carnet_draco_aw
            .iter()
            .any(|c| c.titre == DRACO_CARNET_AW_TITRE)
        {
            self.creer_carnet_entree(CreerCarnetEntree {
                etablissement_id: etab,
                eleve_id: draco.id,
                date_entree: jour,
                categorie: CarnetCategorie::Information,
                titre: DRACO_CARNET_AW_TITRE.into(),
                corps: "Seed labo lot AW-A — entrée carnet visible famille pour audit portail Draco Malefoy."
                    .into(),
                visible_famille: true,
                actor_user_id: cpe.user_id,
            })
            .await?;
        }

        if let Some(luna) = labo_eleve_prenom(eleves, "Luna") {
            if self
                .get_eleve(etab, luna.id)
                .await?
                .is_some_and(|e| e.photo_key.is_none())
            {
                let _ = self
                    .poser_photo_eleve_for_actor(
                        role_admini,
                        etab,
                        luna.id,
                        administratif.user_id,
                        "image/png",
                        LABO_PNG_1X1,
                    )
                    .await;
            }
        }

        if let Some(cedric) = labo_eleve_prenom(eleves, "Cedric") {
            const CEDRIC_DEVOIR_AW_TITRE: &str = "DS Potions lab (Cedric AW-A)";
            let devoirs_cedric_aw = self
                .list_devoirs_for_actor(
                    etab,
                    Role::Prof.as_str(),
                    periode.id,
                    Some(cedric.classe.as_str()),
                )
                .await?;
            if !devoirs_cedric_aw
                .iter()
                .any(|d| d.titre == CEDRIC_DEVOIR_AW_TITRE)
            {
                self.creer_devoir(CreerDevoir {
                    etablissement_id: etab,
                    periode_id: periode.id,
                    matiere_id: potions.id,
                    classe: cedric.classe.clone(),
                    titre: CEDRIC_DEVOIR_AW_TITRE.into(),
                    date_devoir: jour + Duration::days(11),
                    coef: 1.0,
                    sur: 20.0,
                    actor_user_id: prof_rogue.user_id,
                })
                .await?;
            }
        }

        let absence_harry_jour_aw = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_harry_jour_aw)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_harry_jour_aw,
                fin: absence_harry_jour_aw,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }

        const COMMISSION_MOTIFS_AW_RENFORCE: &str = "Seed labo — commission Harry audit";
        let role_cpe_commission_aw = Role::Cpe.as_str();
        let commissions_jour_aw = self
            .list_commissions_for_actor(etab, role_cpe_commission_aw, jour)
            .await?;
        let dossiers_commission_harry_aw = self
            .list_commissions_eleve_for_actor(
                etab,
                cpe.user_id,
                role_cpe_commission_aw,
                harry.id,
            )
            .await?;
        let harry_commission_jour_aw = dossiers_commission_harry_aw.iter().any(|d| {
            commissions_jour_aw
                .iter()
                .any(|c| c.id == d.commission_id)
        });
        if !harry_commission_jour_aw {
            let commission_aw_id = if let Some(c) = commissions_jour_aw.first() {
                c.id
            } else {
                self.creer_commission_for_actor(
                    role_cpe_commission_aw,
                    CreerCommissionVs {
                        etablissement_id: etab,
                        jour,
                        heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                        objet: "Incidents répétés — Harry Potter (AW-A)".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?
                .id
            };
            if !dossiers_commission_harry_aw
                .iter()
                .any(|d| d.commission_id == commission_aw_id)
            {
                let _ = self
                    .convoquer_commission_for_actor(
                        role_cpe_commission_aw,
                        ConvoquerCommission {
                            etablissement_id: etab,
                            commission_id: commission_aw_id,
                            eleve_id: harry.id,
                            motifs: COMMISSION_MOTIFS_AW_RENFORCE.into(),
                            note_interne: "Seed labo AW-A — ré-ensure commission jour J (sans wipe absences)."
                                .into(),
                            sanction_id: None,
                            incident_id: incident_harry,
                            signale_famille: true,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
        }

        // --- Bot AX-A lot X (zone exclusive AX-A) : note Neville, carnet Ginny (famille),
        // photo trombi Harry si vide, devoir Ron ; ré-ensure abs Harry jour J + commission Harry jour J (sans wipe).
        if let Some(neville) = labo_eleve_prenom(eleves, "Neville") {
            const NEVILLE_NOTE_AX_LIB: &str = "Interro Histoire-Géo lab (Neville AX-A)";
            if !self
                .notes_for_eleve(etab, neville.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(NEVILLE_NOTE_AX_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: neville.id,
                    matiere_id: _hg.id,
                    valeur: 11.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(NEVILLE_NOTE_AX_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(ginny) = labo_eleve_prenom(eleves, "Ginny") {
            const GINNY_CARNET_AX_TITRE: &str = "AUDIT AX-A: Suivi Ginny (famille)";
            let carnet_ginny_ax = self
                .list_carnet_for_actor(etab, cpe.user_id, Role::Cpe.as_str(), ginny.id)
                .await?;
            if !carnet_ginny_ax
                .iter()
                .any(|c| c.titre == GINNY_CARNET_AX_TITRE)
            {
                self.creer_carnet_entree(CreerCarnetEntree {
                    etablissement_id: etab,
                    eleve_id: ginny.id,
                    date_entree: jour,
                    categorie: CarnetCategorie::Information,
                    titre: GINNY_CARNET_AX_TITRE.into(),
                    corps: "Seed labo lot AX-A — entrée carnet visible famille pour audit portail Ginny Weasley."
                        .into(),
                    visible_famille: true,
                    actor_user_id: cpe.user_id,
                })
                .await?;
            }
        }

        if self
            .get_eleve(etab, harry.id)
            .await?
            .is_some_and(|e| e.photo_key.is_none())
        {
            let _ = self
                .poser_photo_eleve_for_actor(
                    role_admini,
                    etab,
                    harry.id,
                    administratif.user_id,
                    "image/png",
                    LABO_PNG_1X1,
                )
                .await;
        }

        if let Some(ron) = labo_eleve_prenom(eleves, "Ron") {
            const RON_DEVOIR_AX_TITRE: &str = "DS Mathématiques lab (Ron AX-A)";
            let devoirs_ron_ax = self
                .list_devoirs_for_actor(
                    etab,
                    Role::Prof.as_str(),
                    periode.id,
                    Some(ron.classe.as_str()),
                )
                .await?;
            if !devoirs_ron_ax
                .iter()
                .any(|d| d.titre == RON_DEVOIR_AX_TITRE)
            {
                self.creer_devoir(CreerDevoir {
                    etablissement_id: etab,
                    periode_id: periode.id,
                    matiere_id: math.id,
                    classe: ron.classe.clone(),
                    titre: RON_DEVOIR_AX_TITRE.into(),
                    date_devoir: jour + Duration::days(12),
                    coef: 1.0,
                    sur: 20.0,
                    actor_user_id: prof_mcg.user_id,
                })
                .await?;
            }
        }

        let absence_harry_jour_ax = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_harry_jour_ax)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_harry_jour_ax,
                fin: absence_harry_jour_ax,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }

        const COMMISSION_MOTIFS_AX_RENFORCE: &str = "Seed labo — commission Harry audit";
        let role_cpe_commission_ax = Role::Cpe.as_str();
        let commissions_jour_ax = self
            .list_commissions_for_actor(etab, role_cpe_commission_ax, jour)
            .await?;
        let dossiers_commission_harry_ax = self
            .list_commissions_eleve_for_actor(
                etab,
                cpe.user_id,
                role_cpe_commission_ax,
                harry.id,
            )
            .await?;
        let harry_commission_jour_ax = dossiers_commission_harry_ax.iter().any(|d| {
            commissions_jour_ax
                .iter()
                .any(|c| c.id == d.commission_id)
        });
        if !harry_commission_jour_ax {
            let commission_ax_id = if let Some(c) = commissions_jour_ax.first() {
                c.id
            } else {
                self.creer_commission_for_actor(
                    role_cpe_commission_ax,
                    CreerCommissionVs {
                        etablissement_id: etab,
                        jour,
                        heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                        objet: "Incidents répétés — Harry Potter (AX-A)".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?
                .id
            };
            if !dossiers_commission_harry_ax
                .iter()
                .any(|d| d.commission_id == commission_ax_id)
            {
                let _ = self
                    .convoquer_commission_for_actor(
                        role_cpe_commission_ax,
                        ConvoquerCommission {
                            etablissement_id: etab,
                            commission_id: commission_ax_id,
                            eleve_id: harry.id,
                            motifs: COMMISSION_MOTIFS_AX_RENFORCE.into(),
                            note_interne: "Seed labo AX-A — ré-ensure commission jour J (sans wipe absences)."
                                .into(),
                            sanction_id: None,
                            incident_id: incident_harry,
                            signale_famille: true,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
        }

        // --- Bot AY-A lot Y (zone exclusive AY-A) : note Seamus, devoir Hermione, photo trombi Draco si vide,
        // note Cho (autre matière) ; ré-ensure abs Harry jour J + commission Harry jour J (sans wipe).
        if let Some(seamus) = labo_eleve_prenom(eleves, "Seamus") {
            const SEAMUS_NOTE_AY_LIB: &str = "Interro Histoire-Géo lab (Seamus AY-A)";
            if !self
                .notes_for_eleve(etab, seamus.id)
                .await?
                .iter()
                .any(|n| n.libelle.as_deref() == Some(SEAMUS_NOTE_AY_LIB))
            {
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: seamus.id,
                    matiere_id: _hg.id,
                    valeur: 12.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(1),
                    auteur_user_id: prof_mcg.user_id,
                    libelle: Some(SEAMUS_NOTE_AY_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        if let Some(hermione) = labo_eleve_prenom(eleves, "Hermione") {
            const HERMIONE_DEVOIR_AY_TITRE: &str = "DS Potions lab (Hermione AY-A)";
            let devoirs_hermione_ay = self
                .list_devoirs_for_actor(
                    etab,
                    Role::Prof.as_str(),
                    periode.id,
                    Some(hermione.classe.as_str()),
                )
                .await?;
            if !devoirs_hermione_ay
                .iter()
                .any(|d| d.titre == HERMIONE_DEVOIR_AY_TITRE)
            {
                self.creer_devoir(CreerDevoir {
                    etablissement_id: etab,
                    periode_id: periode.id,
                    matiere_id: potions.id,
                    classe: hermione.classe.clone(),
                    titre: HERMIONE_DEVOIR_AY_TITRE.into(),
                    date_devoir: jour + Duration::days(11),
                    coef: 1.0,
                    sur: 20.0,
                    actor_user_id: prof_rogue.user_id,
                })
                .await?;
            }
        }

        if self
            .get_eleve(etab, draco.id)
            .await?
            .is_some_and(|e| e.photo_key.is_none())
        {
            let _ = self
                .poser_photo_eleve_for_actor(
                    role_admini,
                    etab,
                    draco.id,
                    administratif.user_id,
                    "image/png",
                    LABO_PNG_1X1,
                )
                .await;
        }

        if let Some(cho) = labo_eleve_prenom(eleves, "Cho") {
            const CHO_NOTE_AY_LIB: &str = "Interro Potions lab (Cho AY-A)";
            let notes_cho = self.notes_for_eleve(etab, cho.id).await?;
            let a_deja_note_ay = notes_cho
                .iter()
                .any(|n| n.libelle.as_deref() == Some(CHO_NOTE_AY_LIB));
            if !a_deja_note_ay {
                let matiere_cho_ay = if notes_cho.iter().any(|n| n.matiere_id == potions.id) {
                    _hg.id
                } else {
                    potions.id
                };
                self.saisir_note(SaisirNote {
                    etablissement_id: etab,
                    eleve_id: cho.id,
                    matiere_id: matiere_cho_ay,
                    valeur: 14.0,
                    sur: 20.0,
                    date_note: jour - Duration::days(2),
                    auteur_user_id: if matiere_cho_ay == potions.id {
                        prof_rogue.user_id
                    } else {
                        prof_mcg.user_id
                    },
                    libelle: Some(CHO_NOTE_AY_LIB.into()),
                    devoir_id: None,
                    periode_id: Some(periode.id),
                })
                .await?;
            }
        }

        let absence_harry_jour_ay = jour;
        if !self
            .eleve_est_absent(etab, harry.id, absence_harry_jour_ay)
            .await?
        {
            let motifs = self.list_motifs_absence(etab).await?;
            let motif_code = motifs
                .iter()
                .find(|m| m.code.eq_ignore_ascii_case("maladie"))
                .or_else(|| motifs.first())
                .map(|m| m.code.clone())
                .unwrap_or_else(|| "maladie".into());
            self.declare_absence(DeclareAbsence {
                etablissement_id: etab,
                eleve_id: harry.id,
                debut: absence_harry_jour_ay,
                fin: absence_harry_jour_ay,
                motif: motif_code,
                actor_user_id: Some(cpe.user_id.to_string()),
            })
            .await?;
        }

        const COMMISSION_MOTIFS_AY_RENFORCE: &str = "Seed labo — commission Harry audit";
        let role_cpe_commission_ay = Role::Cpe.as_str();
        let commissions_jour_ay = self
            .list_commissions_for_actor(etab, role_cpe_commission_ay, jour)
            .await?;
        let dossiers_commission_harry_ay = self
            .list_commissions_eleve_for_actor(
                etab,
                cpe.user_id,
                role_cpe_commission_ay,
                harry.id,
            )
            .await?;
        let harry_commission_jour_ay = dossiers_commission_harry_ay.iter().any(|d| {
            commissions_jour_ay
                .iter()
                .any(|c| c.id == d.commission_id)
        });
        if !harry_commission_jour_ay {
            let commission_ay_id = if let Some(c) = commissions_jour_ay.first() {
                c.id
            } else {
                self.creer_commission_for_actor(
                    role_cpe_commission_ay,
                    CreerCommissionVs {
                        etablissement_id: etab,
                        jour,
                        heure: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                        objet: "Incidents répétés — Harry Potter (AY-A)".into(),
                        lieu: "Bureau CPE".into(),
                        actor_user_id: cpe.user_id,
                    },
                )
                .await?
                .id
            };
            if !dossiers_commission_harry_ay
                .iter()
                .any(|d| d.commission_id == commission_ay_id)
            {
                let _ = self
                    .convoquer_commission_for_actor(
                        role_cpe_commission_ay,
                        ConvoquerCommission {
                            etablissement_id: etab,
                            commission_id: commission_ay_id,
                            eleve_id: harry.id,
                            motifs: COMMISSION_MOTIFS_AY_RENFORCE.into(),
                            note_interne: "Seed labo AY-A — ré-ensure commission jour J (sans wipe absences)."
                                .into(),
                            sanction_id: None,
                            incident_id: incident_harry,
                            signale_famille: true,
                            actor_user_id: cpe.user_id,
                        },
                    )
                    .await;
            }
        }

        Ok(())
    }

    async fn ensure_matiere_lab(
        &self,
        etab: Uuid,
        code: &str,
        libelle: &str,
        source: Option<String>,
        actor_user_id: &str,
    ) -> Result<crate::matiere_repo::Matiere, EntDbError> {
        if let Some(m) = self
            .list_matieres(etab)
            .await?
            .into_iter()
            .find(|m| m.code.eq_ignore_ascii_case(code))
        {
            return Ok(m);
        }
        self.create_matiere(
            CreateMatiere {
                etablissement_id: etab,
                code: code.into(),
                libelle: libelle.into(),
                source_nomenclature: source,
            },
            Some(actor_user_id.to_string()),
        )
        .await
    }

    async fn ensure_note_periode_t1(
        &self,
        etab: Uuid,
        date_debut: NaiveDate,
        date_fin: NaiveDate,
        actor_user_id: &str,
    ) -> Result<domain_eleve::NotePeriode, EntDbError> {
        self.ensure_note_periode_lab(etab, "T1", "Trimestre 1", date_debut, date_fin, actor_user_id)
            .await
    }

    async fn ensure_note_periode_lab(
        &self,
        etab: Uuid,
        code: &str,
        libelle: &str,
        date_debut: NaiveDate,
        date_fin: NaiveDate,
        actor_user_id: &str,
    ) -> Result<domain_eleve::NotePeriode, EntDbError> {
        if let Some(p) = self
            .list_note_periodes(etab)
            .await?
            .into_iter()
            .find(|p| p.code.eq_ignore_ascii_case(code))
        {
            return Ok(p);
        }
        self.creer_note_periode(CreerNotePeriode {
            etablissement_id: etab,
            code: code.into(),
            libelle: libelle.into(),
            date_debut,
            date_fin,
            actor_user_id: Some(actor_user_id.to_string()),
        })
        .await
    }

    async fn ensure_devoir_potions_lab(
        &self,
        etab: Uuid,
        periode_id: Uuid,
        matiere_id: Uuid,
        classe: &str,
        jour: NaiveDate,
        prof_user_id: Uuid,
    ) -> Result<(Devoir, bool), EntDbError> {
        const TITRE: &str = "DS Potions T1";
        if let Some(d) = self
            .list_devoirs_for_actor(etab, Role::Prof.as_str(), periode_id, Some(classe))
            .await?
            .into_iter()
            .find(|d| d.titre == TITRE)
        {
            return Ok((d, false));
        }
        let d = self
            .creer_devoir(CreerDevoir {
                etablissement_id: etab,
                periode_id,
                matiere_id,
                classe: classe.into(),
                titre: TITRE.into(),
                date_devoir: jour,
                coef: 2.0,
                sur: 20.0,
                actor_user_id: prof_user_id,
            })
            .await?;
        Ok((d, true))
    }
}

fn labo_eleve_prenom<'a>(eleves: &'a [LaboEleve], prenom: &str) -> Option<&'a LaboEleve> {
    eleves.iter().find(|e| e.prenom.eq_ignore_ascii_case(prenom))
}

/// Lundi de la semaine calendaire du jour (ISO).
fn debut_semaine_lundi(jour: NaiveDate) -> NaiveDate {
    jour - Duration::days(jour.weekday().num_days_from_monday() as i64)
}

/// PDF minimal valide pour justificatif d'absence seed.
const PDF_JUSTIF_MINIMAL: &[u8] = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n";

/// PNG 1×1 px (trombi seed).
const LABO_PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
    0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
    0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8,
    0xcf, 0xc0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x05, 0xfe, 0xd4, 0xef, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];
