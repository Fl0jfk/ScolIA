//! Seed labo local — école dense multi-rôles pour exploration autonome.
//! **Uniquement** via Postgres loopback (`PgEnt::connect` déjà gardé).
//!
//! Le cast n'est pas un script figé : lots de personas, flags parent divergents,
//! classes mixtes. Le runner `ent-lab` fait jouer ces vies et signale les trous.

use crate::foyer_repo::CreateLien;
use crate::service::{EntDbError, PgEnt};
use chrono::{NaiveDate, NaiveTime};
use domain_eleve::{today_utc, CreateEleve};
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
}
