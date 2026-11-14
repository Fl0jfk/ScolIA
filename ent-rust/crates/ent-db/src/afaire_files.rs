//! Socle 29, lot P4 : **files des personnels** (`docs/socles/29_PORTAILS.md` §8.4, §18.3, §29.4).
//!
//! * **Aucun second modèle de file.** Une file est ou bien une file de l'index des tâches (déclarée
//!   dans [`FILES`], membres **calculés** depuis les droits, jamais saisis), ou bien une file du socle N1
//!   (demandes : `ent_dem_file`, membres calculés depuis les missions du socle 6). Les tuiles de
//!   « Mes files » **lisent** ces files par leurs ports ; elles n'en créent aucune.
//! * **Deux personnes ne traitent jamais la même tâche** : la prise en charge est une ligne
//!   `ent_afaire_prise` dont l'unicité partielle `(tâche, niveau)` est tenue **en base** ; la
//!   seconde prise reçoit « déjà prise » (CPO-36). Une prise expire (S-2901 : 2 jours ouvrés) et
//!   se relâche à la clôture de la tâche.
//! * **Aucune tâche orpheline** : une tâche ouverte dont aucun destinataire n'est servi (compte parti,
//!   file sans membre) est rendue à la file de repli de sa nature, puis à la direction, puis à
//!   l'administration (R-PO9, A13).
//! * **Deux niveaux** (POR-254, POR-255) : « préparer » puis « décider » ; la tâche préparée passe
//!   `en_attente` (« préparée par X »).
//! * **Supervision par nombres** (POR-161, POR-162) : jamais un nom, jamais un classement.
//!
//! Le jour et l'heure sont ceux de l'établissement ; l'instant est toujours **fourni** par l'appelant.

use crate::afaire::{
    ajouter_destinataire, fermer_tache, fin_de_journee, heure_locale, journaliser, nature,
    priorite_effective, version_de, Destinataire, FermetureTache, MotifClotureAfaire, FILE_ADMIN,
    FILE_DECISIONS, FILE_JUSTIFICATIFS, FILE_PROTECTIONS, NATURES,
};
use crate::afaire_service::DEJA_TRAITEE;
use crate::service::{EntDbError, PgEnt};
use crate::structure_calendrier::calendrier;
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Duration, Utc};
use ent_auth::{Action, Role};
use ent_types::afaire::{
    ActionTache, Collegue, EtatAfaire, FileAfaire, LigneSupervision, MesFiles, NiveauPrise,
    OrigineFile, Orpheline, PrioriteAfaire, PriseVue, RapportFiles, Supervision, TacheFile,
    TuileFile, MESSAGE_DEJA_PRISE,
};
use sqlx::{PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

fn introuvable() -> EntDbError {
    EntDbError::Domain(domain_eleve::DomainError::NotFound)
}

/// Relâchée « par le système » : l'identifiant nul (la contrainte lie `relachee_le` et `relachee_par`).
const SYSTEME: Uuid = Uuid::nil();

// ---------------------------------------------------------------------------------------------
// Déclaration des files de l'index (code → libellé, écran du propriétaire, droits des niveaux)
// ---------------------------------------------------------------------------------------------

/// Droit qui fait d'une personne un membre d'une file à un niveau. Les membres ne sont **jamais
/// saisis** : ils se déduisent des droits effectifs (profil, rôles paramétrés, délégations du socle 6).
#[derive(Clone, Copy, Debug)]
pub enum DroitFile {
    /// Instruire les justificatifs d'absence : profil de la vie scolaire **ou** droit
    /// `absences.justifier` (rôle paramétré, délégation en cours).
    Justificatifs,
    Action(Action),
}

/// Une file de l'index des tâches.
#[derive(Clone, Copy, Debug)]
pub struct FileDeclaree {
    pub code: &'static str,
    pub libelle: &'static str,
    /// Écran du propriétaire où la file se traite (POR-151).
    pub lien: Option<&'static str>,
    /// Calendrier d'ouverture qui compte les jours ouvrés d'une prise (socle 5, R-S2).
    pub service: &'static str,
    pub decider: DroitFile,
    /// File à deux niveaux (POR-254) : le droit de **préparer**, distinct de celui de décider.
    pub preparer: Option<DroitFile>,
}

impl FileDeclaree {
    pub fn deux_niveaux(&self) -> bool {
        self.preparer.is_some()
    }
}

/// Files déclarées. Chaque code est celui d'une `file_repli` ou d'une escalade des natures
/// ([`crate::afaire::NATURES`]) : un test refuse une file de nature qui ne serait pas déclarée ici.
pub const FILES: &[FileDeclaree] = &[
    FileDeclaree {
        code: FILE_JUSTIFICATIFS,
        libelle: "Vie scolaire : justificatifs, mots et appels",
        lien: Some("/web/absences"),
        service: "vie_scolaire",
        decider: DroitFile::Justificatifs,
        preparer: None,
    },
    FileDeclaree {
        code: FILE_PROTECTIONS,
        libelle: "Direction : protections à confirmer",
        lien: Some("/web/direction/protections"),
        service: "secretariat",
        decider: DroitFile::Action(Action::LiensSuspendre),
        // Le secrétariat instruit (il gère déjà les données protégées) ; seule la direction décide.
        preparer: Some(DroitFile::Action(Action::ResponsablesProtegeGerer)),
    },
    FileDeclaree {
        code: FILE_DECISIONS,
        libelle: "Direction : décisions en attente",
        lien: Some("/web/a-faire"),
        service: "secretariat",
        decider: DroitFile::Action(Action::LiensSuspendre),
        preparer: None,
    },
    FileDeclaree {
        code: FILE_ADMIN,
        libelle: "Administration : décisions sans réponse",
        lien: Some("/web/a-faire"),
        service: "secretariat",
        decider: DroitFile::Action(Action::AfaireRapprocher),
        preparer: None,
    },
];

/// Une file de l'index par son code.
pub fn file_declaree(code: &str) -> Option<&'static FileDeclaree> {
    FILES.iter().find(|f| f.code == code)
}

/// Contrôle des déclarations (POR-243 étendu aux files) : toute file citée par une nature (repli,
/// escalade) est déclarée, et inversement aucune file déclarée n'est inconnue des natures.
pub fn verifier_files() -> Result<(), Vec<String>> {
    let mut fautes = Vec::new();
    for n in NATURES {
        if file_declaree(n.file_repli).is_none() {
            fautes.push(format!(
                "{} : file de repli « {} » non déclarée",
                n.code, n.file_repli
            ));
        }
        if let Some(e) = n.escalade {
            if file_declaree(e.file).is_none() {
                fautes.push(format!(
                    "{} : file d'escalade « {} » non déclarée",
                    n.code, e.file
                ));
            }
        }
    }
    for f in FILES {
        if f.service.is_empty() || f.libelle.is_empty() {
            fautes.push(format!("{} : libellé ou service manquant", f.code));
        }
        if FILES.iter().filter(|g| g.code == f.code).count() > 1 {
            fautes.push(format!("{} : déclarée deux fois", f.code));
        }
    }
    if fautes.is_empty() {
        Ok(())
    } else {
        Err(fautes)
    }
}

/// Accès d'une personne à une file, par niveau.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AccesFile {
    pub file: &'static FileDeclaree,
    pub decider: bool,
    pub preparer: bool,
}

/// Nom court d'affichage : « prénom + initiale » (défaut du socle N1 : `prenom_initiale`). Jamais
/// l'adresse électronique.
pub(crate) fn nom_court(nom: Option<&str>) -> String {
    let mots: Vec<&str> = nom.unwrap_or("").split_whitespace().collect();
    match mots.as_slice() {
        [] => "Un collègue".to_string(),
        [seul] => (*seul).to_string(),
        [premier, .., dernier] => {
            let initiale: String = dernier.chars().take(1).collect();
            format!("{premier} {}.", initiale.to_uppercase())
        }
    }
}

const OUVERTS: &str = "('ouverte', 'prise', 'en_attente')";

/// Une tâche d'une file, telle que l'index la porte (avant revérification).
struct LigneFile {
    id: Uuid,
    nature: String,
    objet_id: Uuid,
    eleve_prenom: Option<String>,
    titre: String,
    priorite: PrioriteAfaire,
    echeance_le: Option<DateTime<Utc>>,
    file: String,
}

/// Une prise active.
struct PriseLigne {
    niveau: NiveauPrise,
    compte: Uuid,
    nom: String,
    depuis: DateTime<Utc>,
    jusqu_a: DateTime<Utc>,
}

/// La tâche verrouillée pour un geste de prise.
struct TacheVerrouillee {
    nature: String,
    objet_id: Uuid,
    etat: EtatAfaire,
    files: Vec<String>,
}

impl PgEnt {
    // ───────────────────────── Accès : membres calculés ─────────────────────────

    async fn a_droit_file(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        d: DroitFile,
    ) -> Result<bool, EntDbError> {
        match d {
            DroitFile::Justificatifs => {
                if Self::role_can_trancher_justificatif(role) {
                    return Ok(true);
                }
                self.a_action_etablissement(etab, user, role, Action::AbsencesJustifier)
                    .await
            }
            DroitFile::Action(a) => self.a_action_etablissement(etab, user, role, a).await,
        }
    }

    /// Files de l'index auxquelles la personne a accès, avec son niveau (membres **calculés**).
    pub(crate) async fn acces_files(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<Vec<AccesFile>, EntDbError> {
        let Some(base) = Role::parse(role) else {
            return Ok(Vec::new());
        };
        if !base.is_staff() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for f in FILES {
            let decider = self.a_droit_file(etab, user, role, f.decider).await?;
            let preparer = match f.preparer {
                Some(d) => self.a_droit_file(etab, user, role, d).await?,
                None => false,
            };
            if decider || preparer {
                out.push(AccesFile {
                    file: f,
                    decider,
                    preparer,
                });
            }
        }
        Ok(out)
    }

    /// Membres d'une file (comptes du personnel qui y ont accès au niveau demandé) : sert à la
    /// réattribution et à la détection des files sans membre. Calcul à la demande.
    async fn membres_de_file(
        &self,
        etab: Uuid,
        file: &str,
        niveau: NiveauPrise,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let comptes: Vec<(Uuid, String)> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query_as("SELECT user_id, role FROM ent_membership WHERE etablissement_id = $1 ORDER BY user_id")
                .bind(etab)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?
        };
        let mut out = Vec::new();
        for (user, role) in comptes {
            for a in self.acces_files(etab, user, &role).await? {
                let permis = match niveau {
                    NiveauPrise::Decider => a.decider,
                    NiveauPrise::Preparer => a.preparer && a.file.deux_niveaux(),
                };
                if a.file.code == file && permis {
                    out.push(user);
                }
            }
        }
        Ok(out)
    }

    // ───────────────────────── Lecture : tuiles et file ─────────────────────────

    /// Lignes de l'index pour des files (tâches ouvertes, visibles maintenant).
    async fn lignes_files(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        files: &[String],
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<LigneFile>, EntDbError> {
        let rows = sqlx::query(&format!(
            "SELECT a.id, a.nature_code, a.objet_id, e.prenom, a.titre_neutre, a.priorite, a.etat,
                    a.echeance_le, d.file_code
               FROM ent_afaire a
               JOIN ent_afaire_destinataire d
                 ON d.etablissement_id = a.etablissement_id AND d.afaire_id = a.id
                AND d.retire_le IS NULL AND d.type = 'file' AND d.file_code = ANY($2)
               LEFT JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
              WHERE a.etablissement_id = $1 AND a.etat IN {OUVERTS} AND a.visible_a_partir_de <= $3
              ORDER BY a.id, d.file_code"
        ))
        .bind(etab)
        .bind(files)
        .bind(maintenant)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let Some(priorite) = PrioriteAfaire::parse(&r.get::<String, _>(5)) else {
                continue;
            };
            out.push(LigneFile {
                id: r.get(0),
                nature: r.get(1),
                objet_id: r.get(2),
                eleve_prenom: r.get(3),
                titre: r.get(4),
                priorite,
                echeance_le: r.get(7),
                file: r.get(8),
            });
        }
        Ok(out)
    }

    /// Ne garde que les lignes que le propriétaire dit encore ouvertes pour cette file (R-PO3, R-PO12 :
    /// une source en panne retire ses tâches, jamais la page). Une lecture par nature.
    async fn lignes_vivantes(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        lignes: Vec<LigneFile>,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<LigneFile>, EntDbError> {
        let mut par_nature: HashMap<&str, Vec<Uuid>> = HashMap::new();
        for l in &lignes {
            par_nature
                .entry(l.nature.as_str())
                .or_default()
                .push(l.objet_id);
        }
        let vivantes = self
            .sources_vivantes(conn, etab, par_nature, maintenant)
            .await?;
        Ok(lignes
            .into_iter()
            .filter(|l| {
                let Some(n) = nature(&l.nature) else {
                    return false;
                };
                if !n.derivee {
                    return true;
                }
                match vivantes.get(&(l.nature.clone(), l.objet_id)) {
                    None => false, // la source ne la dit plus ouverte
                    Some(dests) if dests.is_empty() => l.file == n.file_repli,
                    Some(dests) => dests
                        .iter()
                        .any(|d| matches!(d, Destinataire::File(f) if *f == l.file)),
                }
            })
            .collect())
    }

    /// Prises **actives** (non relâchées, non échues) de tâches.
    async fn prises_actives(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        taches: &[Uuid],
        maintenant: DateTime<Utc>,
    ) -> Result<HashMap<Uuid, Vec<PriseLigne>>, EntDbError> {
        let rows = sqlx::query(
            "SELECT p.afaire_id, p.niveau, p.compte_id, u.display_name, p.depuis, p.jusqu_a
               FROM ent_afaire_prise p
               JOIN ent_user u ON u.id = p.compte_id
              WHERE p.etablissement_id = $1 AND p.afaire_id = ANY($2)
                AND p.relachee_le IS NULL AND p.jusqu_a > $3
              ORDER BY p.afaire_id, p.niveau",
        )
        .bind(etab)
        .bind(taches)
        .bind(maintenant)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut out: HashMap<Uuid, Vec<PriseLigne>> = HashMap::new();
        for r in rows {
            let Some(niveau) = NiveauPrise::parse(&r.get::<String, _>(1)) else {
                continue;
            };
            let nom: Option<String> = r.get(3);
            out.entry(r.get(0)).or_default().push(PriseLigne {
                niveau,
                compte: r.get(2),
                nom: nom_court(nom.as_deref()),
                depuis: r.get(4),
                jusqu_a: r.get(5),
            });
        }
        Ok(out)
    }

    /// « Préparée par X » : l'auteur de la dernière préparation, pour les tâches `en_attente`.
    async fn preparateurs(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        taches: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, EntDbError> {
        let rows = sqlx::query(
            "SELECT DISTINCT ON (j.afaire_id) j.afaire_id, u.display_name
               FROM ent_afaire_journal j
               LEFT JOIN ent_user u ON u.id = j.par_compte_id
              WHERE j.etablissement_id = $1 AND j.afaire_id = ANY($2) AND j.motif_code = 'prepare'
              ORDER BY j.afaire_id, j.le DESC",
        )
        .bind(etab)
        .bind(taches)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let nom: Option<String> = r.get(1);
                (r.get::<Uuid, _>(0), nom_court(nom.as_deref()))
            })
            .collect())
    }

    /// `GET /afaire/files` : « Mes files » (POR-150). Des nombres et une échéance, jamais de contenu.
    pub async fn mes_files(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<MesFiles, EntDbError> {
        let base = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !base.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let acces = self.acces_files(etab, user, role).await?;
        let mut tuiles = Vec::new();
        if !acces.is_empty() {
            let codes: Vec<String> = acces.iter().map(|a| a.file.code.to_string()).collect();
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let lignes = self.lignes_files(&mut tx, etab, &codes, maintenant).await?;
            let lignes = self
                .lignes_vivantes(&mut tx, etab, lignes, maintenant)
                .await?;
            let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
            let prises = self.prises_actives(&mut tx, etab, &ids, maintenant).await?;
            tx.commit().await.map_err(db)?;
            for a in &acces {
                let siennes: Vec<&LigneFile> =
                    lignes.iter().filter(|l| l.file == a.file.code).collect();
                let qui = |l: &LigneFile| -> (bool, bool) {
                    let ps = prises.get(&l.id);
                    let moi = ps.is_some_and(|v| v.iter().any(|p| p.compte == user));
                    let autre = ps.is_some_and(|v| v.iter().any(|p| p.compte != user));
                    (moi, autre)
                };
                tuiles.push(TuileFile {
                    code: a.file.code.to_string(),
                    libelle: a.file.libelle.to_string(),
                    origine: OrigineFile::Afaire,
                    ouvertes: siennes.len() as i64,
                    en_retard: Some(
                        siennes
                            .iter()
                            .filter(|l| l.echeance_le.is_some_and(|e| e < maintenant))
                            .count() as i64,
                    ),
                    prises_par_moi: siennes.iter().filter(|l| qui(l).0).count() as i64,
                    prises_par_autres: siennes.iter().filter(|l| qui(l).1 && !qui(l).0).count()
                        as i64,
                    plus_ancienne: siennes.iter().filter_map(|l| l.echeance_le).min(),
                    lien: a.file.lien.map(str::to_string),
                });
            }
        }
        tuiles.extend(self.tuiles_demandes(etab, user, role).await);
        // Ordre stable (alphabétique par libellé) : jamais par volume ni par retard.
        tuiles.sort_by(|a, b| {
            (a.libelle.as_str(), a.code.as_str()).cmp(&(b.libelle.as_str(), b.code.as_str()))
        });
        Ok(MesFiles { tuiles })
    }

    /// Tuiles des files de demandes du socle N1 dont la personne est membre : lues par les ports de N1
    /// (`ma_file_for_actor`, graphe des files), jamais recalculées ici. Une file de N1 en panne ou une
    /// personne sans appartenance donne **zéro tuile**, jamais une erreur (R-PO12).
    async fn tuiles_demandes(&self, etab: Uuid, user: Uuid, role: &str) -> Vec<TuileFile> {
        let Ok(sources) = self
            .sources_demandes_for_actor(etab, user, role, None)
            .await
        else {
            return Vec::new();
        };
        let Ok(demandes) = self.ma_file_for_actor(etab, user, role, &sources).await else {
            return Vec::new();
        };
        if demandes.is_empty() {
            return Vec::new();
        }
        let Ok(mut tx) = TenantTx::begin(self.pool(), etab).await else {
            return Vec::new();
        };
        let Ok(jour) = crate::demandes_files::jour_metier(&mut tx, etab).await else {
            return Vec::new();
        };
        let Ok(graphe) = crate::demandes_files::charger_graphe(&mut tx, etab, jour).await else {
            return Vec::new();
        };
        let mut par_file: BTreeMap<Uuid, (i64, Option<DateTime<Utc>>)> = BTreeMap::new();
        for d in &demandes {
            let e = par_file.entry(d.file_id).or_insert((0, None));
            e.0 += 1;
            e.1 = match (e.1, d.depose_le) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
        }
        par_file
            .into_iter()
            .filter_map(|(id, (n, plus_ancienne))| {
                let f = graphe.files.get(&id)?;
                Some(TuileFile {
                    code: f.code.clone(),
                    libelle: f.libelle.clone(),
                    origine: OrigineFile::Demandes,
                    ouvertes: n,
                    en_retard: None, // N1 ne publie pas d'échéance dans sa file : jamais un faux zéro
                    prises_par_moi: 0,
                    prises_par_autres: 0,
                    plus_ancienne,
                    lien: None,
                })
            })
            .collect()
    }

    /// `GET /afaire/files/{code}` : le contenu d'une file de l'index pour l'un de ses membres
    /// (POR-155 : qui a pris quoi). Un non-membre ne distingue pas « inconnue » de « interdite » (404).
    pub async fn file_afaire(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        code: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<FileAfaire, EntDbError> {
        let acces = self.acces_files(etab, user, role).await?;
        let Some(a) = acces.iter().find(|a| a.file.code == code) else {
            return Err(introuvable());
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = self
            .lignes_files(&mut tx, etab, &[code.to_string()], maintenant)
            .await?;
        let lignes = self
            .lignes_vivantes(&mut tx, etab, lignes, maintenant)
            .await?;
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
        let prises = self.prises_actives(&mut tx, etab, &ids, maintenant).await?;
        let prepares = self.preparateurs(&mut tx, etab, &ids).await?;
        tx.commit().await.map_err(db)?;

        let mut taches: Vec<TacheFile> = Vec::new();
        for l in lignes {
            let Some(n) = nature(&l.nature) else { continue };
            let vues: Vec<PriseVue> = prises
                .get(&l.id)
                .map(|v| {
                    v.iter()
                        .map(|p| PriseVue {
                            par_moi: p.compte == user,
                            par: p.nom.clone(),
                            niveau: p.niveau,
                            depuis: p.depuis,
                            jusqu_a: p.jusqu_a,
                        })
                        .collect()
                })
                .unwrap_or_default();
            // L'état affiché suit les prises ACTIVES : une prise échue n'est plus une prise.
            let etat = if !vues.is_empty() {
                EtatAfaire::Prise
            } else if prepares.contains_key(&l.id) && a.file.deux_niveaux() {
                EtatAfaire::EnAttente
            } else {
                EtatAfaire::Ouverte
            };
            taches.push(TacheFile {
                id: l.id,
                nature: l.nature.clone(),
                titre: l.titre,
                eleve_prenom: l.eleve_prenom,
                priorite: priorite_effective(
                    l.priorite,
                    l.echeance_le,
                    n.montee_due_heures,
                    maintenant,
                ),
                etat,
                echeance_le: l.echeance_le,
                prises: vues,
                prepare_par: prepares
                    .get(&l.id)
                    .filter(|_| etat == EtatAfaire::EnAttente)
                    .cloned(),
                action: n.action.map(|x| match x {
                    crate::afaire::ActionDeclaree::Carte { libelle } => ActionTache::Carte {
                        libelle: libelle.to_string(),
                    },
                    crate::afaire::ActionDeclaree::Lien { libelle, lien } => ActionTache::Lien {
                        libelle: libelle.to_string(),
                        lien: lien.to_string(),
                    },
                }),
            });
        }
        // Tri (§8.5) : bloquantes, échéance la plus proche, puis identifiant (stable).
        taches.sort_by(|x, y| {
            let ech = |t: &TacheFile| {
                t.echeance_le
                    .map_or((1, DateTime::<Utc>::MAX_UTC), |e| (0, e))
            };
            (x.priorite.rang(), ech(x), x.id).cmp(&(y.priorite.rang(), ech(y), y.id))
        });
        let peut_reattribuer = self
            .a_action_etablissement(etab, user, role, Action::AfaireReattribuer)
            .await?;
        let collegues = if peut_reattribuer {
            self.collegues_de_file(etab, code).await?
        } else {
            Vec::new()
        };
        Ok(FileAfaire {
            code: code.to_string(),
            libelle: a.file.libelle.to_string(),
            deux_niveaux: a.file.deux_niveaux(),
            taches,
            peut_reattribuer,
            collegues,
        })
    }

    async fn collegues_de_file(&self, etab: Uuid, code: &str) -> Result<Vec<Collegue>, EntDbError> {
        let mut ids: BTreeSet<Uuid> = self
            .membres_de_file(etab, code, NiveauPrise::Decider)
            .await?
            .into_iter()
            .collect();
        if file_declaree(code).is_some_and(FileDeclaree::deux_niveaux) {
            ids.extend(
                self.membres_de_file(etab, code, NiveauPrise::Preparer)
                    .await?,
            );
        }
        let ids: Vec<Uuid> = ids.into_iter().collect();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query("SELECT id, display_name FROM ent_user WHERE id = ANY($1)")
            .bind(&ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut out: Vec<Collegue> = rows
            .into_iter()
            .map(|r| {
                let nom: Option<String> = r.get(1);
                Collegue {
                    compte_id: r.get(0),
                    nom: nom_court(nom.as_deref()),
                }
            })
            .collect();
        out.sort_by(|a, b| (a.nom.as_str(), a.compte_id).cmp(&(b.nom.as_str(), b.compte_id)));
        Ok(out)
    }

    // ───────────────────────── Prises en charge ─────────────────────────

    async fn tache_verrouillee(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        tache: Uuid,
    ) -> Result<Option<TacheVerrouillee>, EntDbError> {
        let r = sqlx::query(
            "SELECT nature_code, objet_id, etat FROM ent_afaire
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(tache)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some(r) = r else { return Ok(None) };
        let etat = EtatAfaire::parse(&r.get::<String, _>(2)).unwrap_or(EtatAfaire::Ouverte);
        let files: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT file_code FROM ent_afaire_destinataire
              WHERE etablissement_id = $1 AND afaire_id = $2 AND type = 'file' AND retire_le IS NULL
                AND file_code IS NOT NULL",
        )
        .bind(etab)
        .bind(tache)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(Some(TacheVerrouillee {
            nature: r.get(0),
            objet_id: r.get(1),
            etat,
            files: files.into_iter().map(|(f,)| f).collect(),
        }))
    }

    /// Fin d'une prise : 2 jours ouvrés du calendrier de la file (S-2901), à 23 h 59 heure locale. Un
    /// calendrier indisponible donne 48 h : une prise ne dure jamais indéfiniment.
    async fn fin_de_prise(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        service: &str,
        maintenant: DateTime<Utc>,
    ) -> DateTime<Utc> {
        let secours = maintenant + Duration::hours(48);
        let Ok(local) = heure_locale(conn, etab, maintenant).await else {
            return secours;
        };
        let Ok(jour) =
            calendrier::echeance_jours_ouvres(conn, etab, local.date(), 2, service).await
        else {
            return secours;
        };
        match fin_de_journee(conn, etab, jour).await {
            Ok(i) if i > maintenant => i,
            _ => secours,
        }
    }

    /// L'état de la tâche suit ses prises : `prise` s'il y en a une active, sinon `en_attente`
    /// (préparée, à décider) ou `ouverte`. Journalise le changement.
    #[allow(clippy::too_many_arguments)]
    async fn recalculer_etat(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        tache: Uuid,
        socle: i16,
        par: Option<Uuid>,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let r = sqlx::query(
            "SELECT etat,
                    EXISTS (SELECT 1 FROM ent_afaire_prise p WHERE p.etablissement_id = a.etablissement_id
                             AND p.afaire_id = a.id AND p.relachee_le IS NULL AND p.jusqu_a > $3),
                    EXISTS (SELECT 1 FROM ent_afaire_journal j WHERE j.etablissement_id = a.etablissement_id
                             AND j.afaire_id = a.id AND j.motif_code = 'prepare')
               FROM ent_afaire a WHERE a.etablissement_id = $1 AND a.id = $2",
        )
        .bind(etab)
        .bind(tache)
        .bind(maintenant)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some(r) = r else { return Ok(()) };
        let actuel = EtatAfaire::parse(&r.get::<String, _>(0)).unwrap_or(EtatAfaire::Ouverte);
        if !actuel.est_ouvert() {
            return Ok(());
        }
        let (prise, preparee): (bool, bool) = (r.get(1), r.get(2));
        let cible = if prise {
            EtatAfaire::Prise
        } else if preparee {
            EtatAfaire::EnAttente
        } else {
            EtatAfaire::Ouverte
        };
        if cible == actuel {
            return Ok(());
        }
        sqlx::query("UPDATE ent_afaire SET etat = $3, modifie_le = $4 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(tache)
            .bind(cible.code())
            .bind(maintenant)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        journaliser(
            conn,
            etab,
            tache,
            Some(actuel),
            cible,
            par,
            socle,
            Some(motif),
            maintenant,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    /// Insère une prise ; `Ok(true)` si posée, `Ok(false)` si une prise active existe déjà à ce niveau.
    async fn poser_prise(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        tache: Uuid,
        compte: Uuid,
        niveau: NiveauPrise,
        jusqu_a: DateTime<Utc>,
        maintenant: DateTime<Utc>,
    ) -> Result<bool, EntDbError> {
        let n = sqlx::query(
            "INSERT INTO ent_afaire_prise (id, etablissement_id, afaire_id, compte_id, niveau, depuis, jusqu_a)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (afaire_id, niveau) WHERE relachee_le IS NULL DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(tache)
        .bind(compte)
        .bind(niveau.code())
        .bind(maintenant)
        .bind(jusqu_a)
        .execute(&mut *conn)
        .await
        .map_err(db)?
        .rows_affected();
        Ok(n > 0)
    }

    /// Relâche les prises **échues** d'une tâche à un niveau (le temps a passé : S-2901).
    async fn relacher_echues(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        tache: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "UPDATE ent_afaire_prise SET relachee_le = $3, relachee_par = $4
              WHERE etablissement_id = $1 AND afaire_id = $2 AND relachee_le IS NULL AND jusqu_a <= $3",
        )
        .bind(etab)
        .bind(tache)
        .bind(maintenant)
        .bind(SYSTEME)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        Ok(())
    }

    /// Une tâche que le propriétaire ne dit plus ouverte n'est plus prenable : « déjà traitée ».
    async fn exiger_vivante(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        t: &TacheVerrouillee,
        file: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        if !t.etat.est_ouvert() {
            return Err(DEJA_TRAITEE);
        }
        let Some(n) = nature(&t.nature) else {
            return Err(DEJA_TRAITEE);
        };
        if !n.derivee {
            return Ok(());
        }
        let mut par: HashMap<&str, Vec<Uuid>> = HashMap::new();
        par.insert(t.nature.as_str(), vec![t.objet_id]);
        let v = self.sources_vivantes(conn, etab, par, maintenant).await?;
        match v.get(&(t.nature.clone(), t.objet_id)) {
            None => Err(DEJA_TRAITEE),
            Some(d) if d.is_empty() && n.file_repli != file => Err(DEJA_TRAITEE),
            Some(d)
                if !d.is_empty()
                    && !d
                        .iter()
                        .any(|x| matches!(x, Destinataire::File(f) if f == file)) =>
            {
                Err(DEJA_TRAITEE)
            }
            Some(_) => Ok(()),
        }
    }

    /// `POST /afaire/{id}/prendre` (POR-152) : prendre en charge une tâche d'une file dont on est
    /// membre, au niveau demandé. **La première prise gagne** (unicité partielle en base) ; la seconde
    /// reçoit « déjà prise ». Reprendre sa propre prise est sans effet (idempotent).
    pub async fn afaire_prendre(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        tache: Uuid,
        niveau: NiveauPrise,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AfairePrendre)
            .await?;
        let acces = self.acces_files(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(t) = self.tache_verrouillee(&mut tx, etab, tache).await? else {
            return Err(introuvable());
        };
        let permis = |a: &AccesFile| match niveau {
            NiveauPrise::Decider => a.decider,
            NiveauPrise::Preparer => a.preparer && a.file.deux_niveaux(),
        };
        let Some(ma_file) = acces
            .iter()
            .find(|a| t.files.iter().any(|f| f == a.file.code) && permis(a))
        else {
            // Ni membre de la file à ce niveau, ni tâche d'une file : même réponse (pas d'énumération).
            return Err(introuvable());
        };
        self.exiger_vivante(&mut tx, etab, &t, ma_file.file.code, maintenant)
            .await?;
        self.relacher_echues(&mut tx, etab, tache, maintenant)
            .await?;
        let jusqu_a = self
            .fin_de_prise(&mut tx, etab, ma_file.file.service, maintenant)
            .await;
        let socle = nature(&t.nature).map_or(29, |n| n.socle);
        if self
            .poser_prise(&mut tx, etab, tache, user, niveau, jusqu_a, maintenant)
            .await?
        {
            let motif = if niveau == NiveauPrise::Preparer {
                "prise_preparer"
            } else {
                "prise"
            };
            self.recalculer_etat(&mut tx, etab, tache, socle, Some(user), motif, maintenant)
                .await?;
            tx.commit().await.map_err(db)?;
            return Ok(());
        }
        let (porteur,): (Uuid,) = sqlx::query_as(
            "SELECT compte_id FROM ent_afaire_prise
              WHERE etablissement_id = $1 AND afaire_id = $2 AND niveau = $3 AND relachee_le IS NULL",
        )
        .bind(etab)
        .bind(tache)
        .bind(niveau.code())
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        if porteur == user {
            Ok(())
        } else {
            Err(EntDbError::ConflitDossier(MESSAGE_DEJA_PRISE))
        }
    }

    /// `POST /afaire/{id}/relacher` (POR-153) : relâcher **sa** prise. Sans prise : sans effet.
    pub async fn afaire_relacher(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        tache: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AfairePrendre)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(t) = self.tache_verrouillee(&mut tx, etab, tache).await? else {
            return Err(introuvable());
        };
        let autres: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT compte_id FROM ent_afaire_prise
              WHERE etablissement_id = $1 AND afaire_id = $2 AND relachee_le IS NULL AND jusqu_a > $3",
        )
        .bind(etab)
        .bind(tache)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if autres.is_empty() {
            return Ok(());
        }
        if !autres.iter().any(|(c,)| *c == user) {
            // Une prise de collègue ne se relâche pas : elle se réattribue (responsable) ou expire.
            return Err(EntDbError::Forbidden);
        }
        sqlx::query(
            "UPDATE ent_afaire_prise SET relachee_le = $4, relachee_par = $3
              WHERE etablissement_id = $1 AND afaire_id = $2 AND compte_id = $3 AND relachee_le IS NULL",
        )
        .bind(etab)
        .bind(tache)
        .bind(user)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let socle = nature(&t.nature).map_or(29, |n| n.socle);
        self.recalculer_etat(
            &mut tx,
            etab,
            tache,
            socle,
            Some(user),
            "relache",
            maintenant,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// `POST /afaire/{id}/preparee` (POR-254) : celui qui **prépare** remet la tâche à celui qui
    /// **décide** : sa prise « préparer » est relâchée, la tâche passe `en_attente` (« préparée par X »).
    pub async fn afaire_preparee(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        tache: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AfairePrendre)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(t) = self.tache_verrouillee(&mut tx, etab, tache).await? else {
            return Err(introuvable());
        };
        if !t.etat.est_ouvert() {
            return Err(DEJA_TRAITEE);
        }
        let n = sqlx::query(
            "UPDATE ent_afaire_prise SET relachee_le = $4, relachee_par = $3
              WHERE etablissement_id = $1 AND afaire_id = $2 AND compte_id = $3
                AND niveau = 'preparer' AND relachee_le IS NULL AND jusqu_a > $4",
        )
        .bind(etab)
        .bind(tache)
        .bind(user)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            // Seul celui qui prépare peut remettre : les autres n'apprennent rien de plus.
            return Err(introuvable());
        }
        let socle = nature(&t.nature).map_or(29, |n| n.socle);
        // La journalisation « prepare » précède le recalcul : c'est elle qui rend la tâche « préparée ».
        sqlx::query(
            "INSERT INTO ent_afaire_journal
                (id, etablissement_id, afaire_id, de_etat, vers_etat, par_compte_id, par_socle, motif_code, le)
             VALUES ($1, $2, $3, $4, $4, $5, $6, 'prepare', $7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(tache)
        .bind(t.etat.code())
        .bind(user)
        .bind(socle)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        self.recalculer_etat(
            &mut tx,
            etab,
            tache,
            socle,
            Some(user),
            "prepare_remise",
            maintenant,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// `POST /afaire/{id}/reattribuer` (POR-154) : un responsable de service remet une tâche prise (ou
    /// ouverte) à **un collègue de la même file**. Journalisé (qui, quand) ; jamais un nom dans le journal.
    #[allow(clippy::too_many_arguments)]
    pub async fn afaire_reattribuer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        tache: Uuid,
        vers: Uuid,
        niveau: NiveauPrise,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AfaireReattribuer)
            .await?;
        // Le destinataire est un personnel de CET établissement, membre de la file au bon niveau.
        let role_vers: Option<(String,)> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query_as(
                "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
            )
            .bind(etab)
            .bind(vers)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
        };
        let Some((role_vers,)) = role_vers else {
            return Err(introuvable());
        };
        let acces_acteur = self.acces_files(etab, acteur, role).await?;
        let acces_vers = self.acces_files(etab, vers, &role_vers).await?;
        let permis = |a: &AccesFile| match niveau {
            NiveauPrise::Decider => a.decider,
            NiveauPrise::Preparer => a.preparer && a.file.deux_niveaux(),
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(t) = self.tache_verrouillee(&mut tx, etab, tache).await? else {
            return Err(introuvable());
        };
        // Le responsable doit lui-même voir la file de la tâche (jamais une tâche d'un autre service).
        let file_commune = t.files.iter().find(|f| {
            acces_acteur.iter().any(|a| a.file.code == f.as_str())
                && acces_vers
                    .iter()
                    .any(|a| a.file.code == f.as_str() && permis(a))
        });
        let Some(file) = file_commune else {
            return Err(introuvable());
        };
        self.exiger_vivante(&mut tx, etab, &t, file, maintenant)
            .await?;
        let service = file_declaree(file).map_or("secretariat", |f| f.service);
        sqlx::query(
            "UPDATE ent_afaire_prise SET relachee_le = $4, relachee_par = $3
              WHERE etablissement_id = $1 AND afaire_id = $2 AND niveau = $5 AND relachee_le IS NULL",
        )
        .bind(etab)
        .bind(tache)
        .bind(acteur)
        .bind(maintenant)
        .bind(niveau.code())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let jusqu_a = self.fin_de_prise(&mut tx, etab, service, maintenant).await;
        let posee = self
            .poser_prise(&mut tx, etab, tache, vers, niveau, jusqu_a, maintenant)
            .await?;
        if !posee {
            return Err(EntDbError::ConflitDossier(MESSAGE_DEJA_PRISE));
        }
        let socle = nature(&t.nature).map_or(29, |n| n.socle);
        // Une ligne de journal dédiée (même état) : qui a réattribué, sans le nom du collègue.
        sqlx::query(
            "INSERT INTO ent_afaire_journal
                (id, etablissement_id, afaire_id, de_etat, vers_etat, par_compte_id, par_socle, motif_code, le)
             VALUES ($1, $2, $3, $4, 'prise', $5, $6, 'reattribuee', $7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(tache)
        .bind(t.etat.code())
        .bind(acteur)
        .bind(socle)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        self.recalculer_etat(
            &mut tx,
            etab,
            tache,
            socle,
            Some(acteur),
            "prise",
            maintenant,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// La tâche est-elle **prise par un collègue** de l'une de mes files ? Sert aux cartes d'action et à
    /// l'aperçu : « déjà prise » plutôt qu'un 404 — **seulement** pour un membre de la file (un
    /// non-membre n'apprend rien : il a un 404 en amont).
    pub(crate) async fn prise_par_collegue(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        tache: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<bool, EntDbError> {
        let acces = self.acces_files(etab, user, role).await?;
        if acces.is_empty() {
            return Ok(false);
        }
        let codes: Vec<String> = acces.iter().map(|a| a.file.code.to_string()).collect();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_afaire_prise p
              WHERE p.etablissement_id = $1 AND p.afaire_id = $2 AND p.relachee_le IS NULL
                AND p.jusqu_a > $5 AND p.compte_id <> $3
                AND EXISTS (SELECT 1 FROM ent_afaire_destinataire d
                             WHERE d.etablissement_id = p.etablissement_id AND d.afaire_id = p.afaire_id
                               AND d.retire_le IS NULL AND d.type = 'file' AND d.file_code = ANY($4))
                AND NOT EXISTS (SELECT 1 FROM ent_afaire_prise q
                                 WHERE q.etablissement_id = $1 AND q.afaire_id = $2
                                   AND q.relachee_le IS NULL AND q.jusqu_a > $5 AND q.compte_id = $3)",
        )
        .bind(etab)
        .bind(tache)
        .bind(user)
        .bind(&codes)
        .bind(maintenant)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(n > 0)
    }

    // ───────────────────────── Supervision, orphelines ─────────────────────────

    /// `GET /afaire/supervision` (POR-161, POR-162) : nombres par file et par nature, délai médian de
    /// traitement des tâches closes depuis 7 jours. **Jamais un nom, jamais un classement** : l'ordre est
    /// alphabétique, indépendant des volumes et des retards.
    pub async fn afaire_supervision(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<Supervision, EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AfaireSuperviser)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ouvertes = sqlx::query(&format!(
            "SELECT d.file_code, a.nature_code,
                    count(DISTINCT a.id),
                    count(DISTINCT a.id) FILTER (WHERE a.echeance_le IS NOT NULL AND a.echeance_le < $2)
               FROM ent_afaire a
               JOIN ent_afaire_destinataire d
                 ON d.etablissement_id = a.etablissement_id AND d.afaire_id = a.id
                AND d.retire_le IS NULL AND d.type = 'file'
              WHERE a.etablissement_id = $1 AND a.etat IN {OUVERTS}
              GROUP BY d.file_code, a.nature_code"
        ))
        .bind(etab)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let medians = sqlx::query(
            "SELECT d.file_code, a.nature_code,
                    (percentile_cont(0.5) WITHIN GROUP (
                        ORDER BY extract(epoch FROM (a.close_le - a.ouverte_le)) / 60.0))::bigint
               FROM ent_afaire a
               JOIN ent_afaire_destinataire d
                 ON d.etablissement_id = a.etablissement_id AND d.afaire_id = a.id AND d.type = 'file'
              WHERE a.etablissement_id = $1 AND a.etat = 'faite' AND a.close_le >= $2 - interval '7 days'
              GROUP BY d.file_code, a.nature_code",
        )
        .bind(etab)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;

        let mut cases: BTreeMap<(String, String), LigneSupervision> = BTreeMap::new();
        for r in ouvertes {
            let (file, nat): (String, String) = (r.get(0), r.get(1));
            cases.insert(
                (file.clone(), nat.clone()),
                LigneSupervision {
                    libelle: file_declaree(&file).map_or(file.clone(), |f| f.libelle.to_string()),
                    file,
                    nature: nat,
                    ouvertes: r.get(2),
                    en_retard: r.get(3),
                    delai_median_minutes: None,
                },
            );
        }
        for r in medians {
            let (file, nat): (String, String) = (r.get(0), r.get(1));
            let m: Option<i64> = r.get(2);
            cases
                .entry((file.clone(), nat.clone()))
                .or_insert_with(|| LigneSupervision {
                    libelle: file_declaree(&file).map_or(file.clone(), |f| f.libelle.to_string()),
                    file: file.clone(),
                    nature: nat.clone(),
                    ouvertes: 0,
                    en_retard: 0,
                    delai_median_minutes: None,
                })
                .delai_median_minutes = m;
        }
        let orphelines = self.orphelines_de(etab).await?.len() as i64;
        Ok(Supervision {
            lignes: cases.into_values().collect(),
            orphelines,
        })
    }

    /// Une tâche ouverte est **orpheline** quand aucun de ses destinataires actifs n'est servi : un
    /// compte parti de l'établissement, une file sans aucun membre. Les personnes et les foyers
    /// (familles) ne se jugent pas ici : leur résolution appartient au propriétaire (rapprochement).
    async fn orphelines_de(
        &self,
        etab: Uuid,
    ) -> Result<Vec<(Uuid, String, Uuid, String, DateTime<Utc>)>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(&format!(
            "SELECT a.id, a.nature_code, a.objet_id, a.titre_neutre, a.ouverte_le,
                    d.type, d.file_code,
                    (d.type = 'compte' AND EXISTS (SELECT 1 FROM ent_membership m
                        WHERE m.etablissement_id = a.etablissement_id AND m.user_id = d.compte_id))
               FROM ent_afaire a
               LEFT JOIN ent_afaire_destinataire d
                 ON d.etablissement_id = a.etablissement_id AND d.afaire_id = a.id AND d.retire_le IS NULL
              WHERE a.etablissement_id = $1 AND a.etat IN {OUVERTS}
              ORDER BY a.id"
        ))
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        type Tache = (String, Uuid, String, DateTime<Utc>);
        type Dest = (Option<String>, Option<String>, bool);
        let mut taches: BTreeMap<Uuid, (Tache, Vec<Dest>)> = BTreeMap::new();
        for r in rows {
            let id: Uuid = r.get(0);
            let e = taches
                .entry(id)
                .or_insert_with(|| ((r.get(1), r.get(2), r.get(3), r.get(4)), Vec::new()));
            e.1.push((
                r.get(5),
                r.get(6),
                r.get::<Option<bool>, _>(7).unwrap_or(false),
            ));
        }
        // Une file n'est « servie » que si elle a au moins un membre qui décide.
        let mut files_vues: HashMap<String, bool> = HashMap::new();
        for (_, dests) in taches.values() {
            for (t, f, _) in dests {
                if let (Some("file"), Some(code)) = (t.as_deref(), f.as_deref()) {
                    if !files_vues.contains_key(code) {
                        let servie = if file_declaree(code).is_some() {
                            !self
                                .membres_de_file(etab, code, NiveauPrise::Decider)
                                .await?
                                .is_empty()
                        } else {
                            true // file d'un autre propriétaire : jamais jugée ici
                        };
                        files_vues.insert(code.to_string(), servie);
                    }
                }
            }
        }
        let mut out = Vec::new();
        for (id, ((nat, objet, titre, ouverte), dests)) in taches {
            let servie = dests.iter().any(|(t, f, compte_ok)| match t.as_deref() {
                None => false, // aucun destinataire actif
                Some("compte") => *compte_ok,
                Some("file") => f
                    .as_deref()
                    .is_some_and(|c| files_vues.get(c).copied().unwrap_or(true)),
                Some(_) => true, // personne, foyer
            });
            if !servie {
                out.push((id, nat, objet, titre, ouverte));
            }
        }
        Ok(out)
    }

    /// `GET /afaire/orphelines` (POR-168) : les tâches sans destinataire servi, titre neutre seulement.
    pub async fn afaire_orphelines(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<Vec<Orpheline>, EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AfaireSuperviser)
            .await?;
        Ok(self
            .orphelines_de(etab)
            .await?
            .into_iter()
            .map(|(id, nature, _, titre, ouverte_le)| Orpheline {
                id,
                nature,
                titre,
                ouverte_le,
            })
            .collect())
    }

    /// `POST /afaire/orphelines/{id}/sans-objet` (POR-169) : clore « sans objet » une orpheline
    /// **après vérification** auprès du propriétaire : s'il la dit encore ouverte, elle reste.
    pub async fn afaire_clore_orpheline(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        tache: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AfaireSuperviser)
            .await?;
        let orph = self.orphelines_de(etab).await?;
        let Some((_, nat, objet, _, _)) = orph.into_iter().find(|(id, ..)| *id == tache) else {
            return Err(introuvable());
        };
        let Some(n) = nature(&nat) else {
            return Err(introuvable());
        };
        if n.derivee {
            let source = self
                .taches_source(etab, n.code, Some(&[objet]), maintenant)
                .await?;
            if !source.is_empty() {
                return Err(EntDbError::ConflitDossier(
                    "Le propriétaire de cette tâche la dit encore ouverte : elle ne peut pas être close.",
                ));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        fermer_tache(
            &mut tx,
            &FermetureTache {
                etablissement_id: etab,
                nature: n.code,
                objet_id: objet,
                destinataire: None,
                motif: MotifClotureAfaire::SansObjet,
                par_compte_id: Some(user),
                version_source: version_de(maintenant),
                maintenant,
            },
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Réparation (rapprochement) ─────────────────────────

    /// Passe de réparation des files, lancée avec le rapprochement : libère les prises échues (POR-156)
    /// et rend à leur file de repli les tâches orphelines (R-PO9, CPO-39). Idempotent.
    pub async fn reparer_files(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<RapportFiles, EntDbError> {
        let mut rapport = RapportFiles::default();
        // 1. Prises échues.
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let echues: Vec<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_afaire_prise SET relachee_le = $2, relachee_par = $3
              WHERE etablissement_id = $1 AND relachee_le IS NULL AND jusqu_a <= $2
          RETURNING afaire_id",
        )
        .bind(etab)
        .bind(maintenant)
        .bind(SYSTEME)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        rapport.prises_liberees = echues.len() as i64;
        let taches: BTreeSet<Uuid> = echues.into_iter().map(|(t,)| t).collect();
        for t in taches {
            let nat: Option<(String,)> = sqlx::query_as(
                "SELECT nature_code FROM ent_afaire WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(t)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let socle = nat
                .and_then(|(c,)| nature(&c).map(|n| n.socle))
                .unwrap_or(29);
            self.recalculer_etat(&mut tx, etab, t, socle, None, "prise_expiree", maintenant)
                .await?;
        }
        tx.commit().await.map_err(db)?;

        // 2. Orphelines : retour à la file de repli de la nature, puis direction, puis administration.
        for (id, nat, objet, _, _) in self.orphelines_de(etab).await? {
            let Some(n) = nature(&nat) else { continue };
            let mut chaine: Vec<&str> = vec![n.file_repli, FILE_DECISIONS, FILE_ADMIN];
            chaine.dedup();
            let mut cible = None;
            for f in chaine {
                if !self
                    .membres_de_file(etab, f, NiveauPrise::Decider)
                    .await?
                    .is_empty()
                {
                    cible = Some(f);
                    break;
                }
            }
            let Some(cible) = cible else { continue };
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            if ajouter_destinataire(
                &mut tx,
                etab,
                id,
                &nat,
                objet,
                &Destinataire::File(cible.to_string()),
                "repli",
                maintenant,
            )
            .await?
            {
                rapport.reaffectees_repli += 1;
                journaliser(
                    &mut tx,
                    etab,
                    id,
                    None,
                    EtatAfaire::Ouverte,
                    None,
                    n.socle,
                    Some("repli"),
                    maintenant,
                )
                .await
                .ok();
            }
            tx.commit().await.map_err(db)?;
        }
        Ok(rapport)
    }

    /// Ensemble des identifiants de tâches **prises par un autre** que `user` (prise active) : l'à-faire
    /// personnel d'un membre n'affiche pas ce qu'un collègue traite (deux personnes, jamais la même tâche).
    pub(crate) async fn prises_par_autres(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<HashSet<Uuid>, EntDbError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT p.afaire_id FROM ent_afaire_prise p
              WHERE p.etablissement_id = $1 AND p.relachee_le IS NULL AND p.jusqu_a > $3 AND p.compte_id <> $2
                AND NOT EXISTS (SELECT 1 FROM ent_afaire_prise q
                                 WHERE q.etablissement_id = $1 AND q.afaire_id = p.afaire_id
                                   AND q.relachee_le IS NULL AND q.jusqu_a > $3 AND q.compte_id = $2)",
        )
        .bind(etab)
        .bind(user)
        .bind(maintenant)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(|(t,)| t).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_files_des_natures_sont_toutes_declarees() {
        verifier_files().expect("files déclarées");
    }

    #[test]
    fn nom_court_est_prenom_et_initiale_jamais_une_adresse() {
        assert_eq!(nom_court(Some("Camille Durand")), "Camille D.");
        assert_eq!(nom_court(Some("  Anne  de la Tour ")), "Anne T.");
        assert_eq!(nom_court(Some("Mélissa")), "Mélissa");
        assert_eq!(nom_court(None), "Un collègue");
        assert_eq!(nom_court(Some("   ")), "Un collègue");
    }

    #[test]
    fn une_seule_file_a_deux_niveaux_et_son_droit_de_preparer_est_distinct() {
        for f in FILES {
            if let Some(DroitFile::Action(p)) = f.preparer {
                let DroitFile::Action(d) = f.decider else {
                    panic!("{} : décider doit être un droit", f.code)
                };
                assert_ne!(p, d, "{} : préparer et décider doivent différer", f.code);
            }
        }
        assert_eq!(FILES.iter().filter(|f| f.deux_niveaux()).count(), 1);
    }
}
