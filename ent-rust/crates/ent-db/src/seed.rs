//! Seed labo local — école dense multi-rôles pour exploration autonome.
//! **Uniquement** via Postgres loopback (`PgEnt::connect` déjà gardé).
//!
//! Le cast n'est pas un script figé : lots de personas, flags parent divergents,
//! classes mixtes. Le runner `ent-lab` fait jouer ces vies et signale les trous.

use crate::foyer_repo::CreateLien;
use crate::matiere_repo::CreateMatiere;
use crate::note_repo::SaisirNote;
use crate::service::{EntDbError, PgEnt};
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use domain_eleve::{
    annee_scolaire_de, annees_continuum, debut_annee_scolaire, today_utc, CreateEleve,
    CreerAlerteCpe, CreerCarnetEntree, CreerDevoir, CreerNotePeriode, CreerPassageInfirmerie,
    AbsenceStatut, CarnetCategorie, ChoisirOptionN1, ConvoquerCommission, CreerCommissionVs,
    CreerFacture, CreerInfoCompChamp, DeclarerIncident, DeclareAbsence, DemanderGrant,
    DocConfidentialite, DocTiroir, EncaisserFacture, FactureStatut, LigneFactureCmd, MonteeRapide,
    MoyenPaiement,
    OptionAnneeStatut, OptionNature, OuvrirCampagneReinscription, PassageInfirmerieMotif,
    PrononcerSanction, PublierDocASigner, RepasActiviteKind, SaisirAppreciation, UpsertFicheSante,
    UpsertInfoCompValeur, UpsertMenuCantine, UpsertPai,
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
            let parent = self
                .register_login(
                    &format!("parent{idx}-{short}@ent.lab"),
                    Role::Parent,
                    parent_display,
                    etab,
                )
                .await?;
            // Mobile lab (SMS dry-run) — unique par parent.
            let _ = self
                .set_user_telephone(
                    parent.user_id,
                    &format!("06{:08}", 12_000_000 + idx as i32),
                )
                .await;

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

            let foyer = self
                .create_foyer(etab, Some(&format!("Foyer {nom}")))
                .await?;

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

        let lea = &eleves[0];
        let tom = &eleves[1];

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
            lea,
            tom,
            &profs,
            &cpes,
            &infirmieres,
            &admin,
            &comptas[0],
            &administratifs[0],
            jour,
        )
        .await?;

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
    async fn seed_labo_journee_audit(
        &self,
        etab: Uuid,
        harry: &LaboEleve,
        draco: &LaboEleve,
        profs: &[LaboPersona],
        cpes: &[LaboPersona],
        infirmieres: &[LaboPersona],
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

        self.creer_alerte_cpe_for_actor(
            Role::Cpe.as_str(),
            CreerAlerteCpe {
                etablissement_id: etab,
                eleve_id: harry.id,
                titre: "Retard répété — suivi CPE".into(),
                detail: "Seed labo : alerte visible famille pour audit personas.".into(),
                severite: "urgent".into(),
                visible_famille: true,
                jour,
                actor_user_id: cpe.user_id,
            },
        )
        .await?;

        let etab_tag = etab.simple().to_string();
        let etab_short = &etab_tag[..8.min(etab_tag.len())];
        let role_compta = Role::Compta.as_str();
        let role_admini = Role::Administratif.as_str();

        // --- Facture foyer Harry (+ encaissement partiel idempotent)
        let fac_numero = format!("LAB-HARRY-{etab_short}");
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
            let montant = facture_harry.total_centimes.min(2250);
            let _ = self
                .encaisser_facture_for_actor(
                    role_compta,
                    EncaisserFacture {
                        etablissement_id: etab,
                        facture_id: facture_harry.id,
                        montant_centimes: montant,
                        moyen: MoyenPaiement::Virement,
                        reference: Some("SEED-HARRY-AUDIT".into()),
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
            if !docs_harry
                .iter()
                .any(|(d, _)| d.code.as_deref() == Some(code_justif.as_str()))
            {
                self.deposer_document_with_contenu(
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
                .await?;
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
        if let Some(p) = self
            .list_note_periodes(etab)
            .await?
            .into_iter()
            .find(|p| p.code.eq_ignore_ascii_case("T1"))
        {
            return Ok(p);
        }
        self.creer_note_periode(CreerNotePeriode {
            etablissement_id: etab,
            code: "T1".into(),
            libelle: "Trimestre 1".into(),
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
