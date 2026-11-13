//! Socle 29, lots P2 et P3 : réactions aux faits des propriétaires, rapprochement, lecture de
//! l'à-faire d'une personne et cartes d'action (`docs/socles/29_PORTAILS.md` §8, §20, §24).
//!
//! * **Réactions** ([`ConsommateurAfaire`]) : consommateur de l'outbox, donc exécuté **après** le fait,
//!   dans une transaction rejouable ; il relit la source de vérité de l'objet visé et ouvre ou ferme
//!   la tâche (idempotent). Une réaction perdue est rattrapée par le rapprochement.
//! * **Rapprochement** : compare l'index à ce que chaque propriétaire dit être ouvert, ajoute les
//!   manquantes, ferme les orphelines, retire les destinataires qui ont perdu leur droit ; rapport en
//!   **comptes**, jamais de nom (E14). Il rebâtit l'index après une restauration (E9, R-PO24).
//! * **Lecture** ([`PgEnt::a_faire_de`]) : une tâche n'est montrée que si son propriétaire la confirme
//!   actionnable par cette personne **à l'instant de la lecture** (R-PO3), sans cache (R-PO25).
//! * **Cartes d'action** : le 29 n'exécute rien lui-même (R-PO1) ; il appelle la commande du
//!   propriétaire, qui revérifie les droits, avec aperçu et confirmation explicite.

use crate::afaire::{
    escalader_echues, fermer_tache, heure_locale, nature, ouvrir_tache, priorite_effective,
    version_de, ActionDeclaree, Destinataire, FermetureTache, MotifClotureAfaire, OuvertureTache,
    FILE_JUSTIFICATIFS, FILE_PROTECTIONS, NATURES,
};
use crate::afaire_sources::{a_carte_d_action, apercu_carte, taches_ouvertes_sql, TacheSource};
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Utc};
use ent_auth::{Action, Role};
use ent_events::types;
use ent_types::afaire::{
    AFaire, ActionTache, ApercuAction, EtatAfaire, GroupeAfaire, LigneRapprochement,
    PrioriteAfaire, RapportRapprochement, ResultatAction, TacheAfaire, MENTION_SANS_AUTORITE,
    MESSAGE_DEJA_TRAITEE,
};
use sqlx::{PgConnection, Row};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

/// Erreur renvoyée pour toute démarche déjà traitée, par qui que ce soit (R-PO7, R-PO38) : l'API la
/// traduit **toujours** par le même code et le même texte.
pub const DEJA_TRAITEE: EntDbError = EntDbError::ConflitDossier(MESSAGE_DEJA_TRAITEE);

// ---------------------------------------------------------------------------------------------
// Appel à faire : objet dérivé de (séance, jour)
// ---------------------------------------------------------------------------------------------

/// Identifiant stable d'une séance un jour donné (la même séance revient chaque semaine).
pub fn objet_appel(creneau: Uuid, jour: chrono::NaiveDate) -> Uuid {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"afaire.appel|");
    h.update(creneau.as_bytes());
    h.update(jour.format("%Y-%m-%d").to_string().as_bytes());
    let d = h.finalize();
    let mut b = [0u8; 16];
    b.copy_from_slice(&d[..16]);
    // Version 8 (déterministe), variante RFC 4122.
    b[6] = (b[6] & 0x0f) | 0x80;
    b[8] = (b[8] & 0x3f) | 0x80;
    Uuid::from_bytes(b)
}

// ---------------------------------------------------------------------------------------------
// Consommateur de l'outbox
// ---------------------------------------------------------------------------------------------

pub const CONSOMMATEUR_AFAIRE: &str = "p29.afaire";

/// Réagit aux faits des propriétaires (§20.2) : ouvre ou ferme la tâche concernée.
pub struct ConsommateurAfaire;

const TYPES_AFAIRE: &[&str] = &[
    types::ELEVE_ABSENCE_DECLARED,
    types::ELEVE_ABSENCE_JUSTIF_SOUMIS,
    types::ELEVE_ABSENCE_JUSTIF_RETRAIT,
    types::ELEVE_ABSENCE_JUSTIFIED,
    types::ELEVE_ABSENCE_REFUSED,
    types::ELEVE_ABSENCE_ANNULEE_PRESENT,
    types::ELEVE_CARNET_ENTREE,
    types::ELEVE_CARNET_SIGNEE,
    types::DOC_A_SIGNER,
    types::DOC_SIGNE,
    types::ELEVE_DECEDE,
    types::ELEVE_MAJEUR,
    types::ELEVE_LIEN_UPDATED,
];

/// (nature, motif si l'objet n'est plus dans la source) pour un événement.
fn natures_de_levenement(type_name: &str) -> Vec<(&'static str, MotifClotureAfaire)> {
    use MotifClotureAfaire::{Faite, SansObjet};
    match type_name {
        t if t == types::ELEVE_ABSENCE_JUSTIF_SOUMIS => {
            vec![
                ("vs.justifier_absence", Faite),
                ("vs.trancher_justificatif", SansObjet),
            ]
        }
        t if t == types::ELEVE_ABSENCE_JUSTIFIED || t == types::ELEVE_ABSENCE_REFUSED => {
            vec![
                ("vs.justifier_absence", Faite),
                ("vs.trancher_justificatif", Faite),
            ]
        }
        t if t == types::ELEVE_ABSENCE_DECLARED
            || t == types::ELEVE_ABSENCE_JUSTIF_RETRAIT
            || t == types::ELEVE_ABSENCE_ANNULEE_PRESENT =>
        {
            vec![
                ("vs.justifier_absence", SansObjet),
                ("vs.trancher_justificatif", SansObjet),
            ]
        }
        t if t == types::ELEVE_CARNET_SIGNEE => vec![("vs.signer_carnet", Faite)],
        t if t == types::ELEVE_CARNET_ENTREE => vec![("vs.signer_carnet", SansObjet)],
        t if t == types::DOC_SIGNE => vec![("doc.signer", Faite)],
        t if t == types::DOC_A_SIGNER => vec![("doc.signer", SansObjet)],
        _ => vec![],
    }
}

impl Consommateur for ConsommateurAfaire {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_AFAIRE
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_AFAIRE
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(async move {
            let e = evt.event();
            let etab = evt.etablissement_id();
            let maintenant = Utc::now();
            let par = e
                .actor_user_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok());
            let r: Result<(), EntDbError> = async {
                if e.type_name == types::ELEVE_DECEDE {
                    if let Some(eleve) = e.eleve_id {
                        fermer_taches_eleve(tx, etab, eleve, MotifClotureAfaire::Deces, maintenant)
                            .await?;
                    }
                    return Ok(());
                }
                if e.type_name == types::ELEVE_MAJEUR || e.type_name == types::ELEVE_LIEN_UPDATED {
                    if let Some(eleve) = e.eleve_id {
                        resynchroniser_eleve(tx, etab, eleve, maintenant).await?;
                    }
                    return Ok(());
                }
                for (code, motif) in natures_de_levenement(&e.type_name) {
                    resynchroniser_objet(tx, etab, code, e.aggregate_id, motif, par, maintenant)
                        .await?;
                }
                Ok(())
            }
            .await;
            r.map_err(|err| err.to_string())
        })
    }
}

/// Relit la source de l'objet et aligne l'index : ouvre si la source dit « ouverte », ferme sinon.
pub(crate) async fn resynchroniser_objet(
    conn: &mut PgConnection,
    etab: Uuid,
    code: &'static str,
    objet: Uuid,
    motif_si_disparu: MotifClotureAfaire,
    par: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let version = version_de(maintenant);
    let sources = taches_ouvertes_sql(conn, etab, code, Some(&[objet])).await?;
    if sources.is_empty() {
        fermer_tache(
            conn,
            &FermetureTache {
                etablissement_id: etab,
                nature: code,
                objet_id: objet,
                destinataire: None,
                motif: motif_si_disparu,
                par_compte_id: par,
                version_source: version,
                maintenant,
            },
        )
        .await?;
        return Ok(());
    }
    for s in sources {
        appliquer_source(conn, etab, code, &s, par, version, maintenant).await?;
    }
    Ok(())
}

/// Ouvre (ou met à jour) la tâche et retire les destinataires qui ont perdu leur droit.
async fn appliquer_source(
    conn: &mut PgConnection,
    etab: Uuid,
    code: &'static str,
    s: &TacheSource,
    par: Option<Uuid>,
    version: i64,
    maintenant: DateTime<Utc>,
) -> Result<crate::afaire::ResultatOuverture, EntDbError> {
    let res = ouvrir_tache(
        conn,
        &OuvertureTache {
            etablissement_id: etab,
            nature: code,
            objet_id: s.objet_id,
            eleve_id: s.eleve_id,
            valeurs: s.valeurs.clone(),
            destinataires: s.destinataires.clone(),
            echeance_le: s.echeance_le,
            version_source: version,
            par_compte_id: par,
            maintenant,
        },
    )
    .await?;
    let n = nature(code).ok_or(EntDbError::SaisieInvalide("nature inconnue"))?;
    let attendus: Vec<String> = if s.destinataires.is_empty() {
        vec![Destinataire::File(n.file_repli.to_string()).cle()]
    } else {
        s.destinataires.iter().map(Destinataire::cle).collect()
    };
    sqlx::query(
        "UPDATE ent_afaire_destinataire SET retire_le = $4, modifie_le = $4
          WHERE etablissement_id = $1 AND retire_le IS NULL
            AND role_dans_tache IN ('titulaire', 'repli')
            AND destinataire_cle <> ALL($5)
            AND afaire_id IN (SELECT afaire_id FROM ent_afaire_cle
                               WHERE etablissement_id = $1 AND nature_code = $2 AND objet_id = $3)",
    )
    .bind(etab)
    .bind(code)
    .bind(s.objet_id)
    .bind(maintenant)
    .bind(&attendus)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(res)
}

/// Décès (R-PO10) : toutes les tâches de l'élève deviennent « sans objet », **sans aucun message**.
pub(crate) async fn fermer_taches_eleve(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    motif: MotifClotureAfaire,
    maintenant: DateTime<Utc>,
) -> Result<usize, EntDbError> {
    let rows = sqlx::query(
        "SELECT DISTINCT nature_code, objet_id FROM ent_afaire
          WHERE etablissement_id = $1 AND eleve_id = $2 AND etat IN ('ouverte', 'prise', 'en_attente')",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut n = 0;
    for r in rows {
        let (code, objet): (String, Uuid) = (r.get(0), r.get(1));
        let Some(nat) = nature(&code) else { continue };
        n += fermer_tache(
            conn,
            &FermetureTache {
                etablissement_id: etab,
                nature: nat.code,
                objet_id: objet,
                destinataire: None,
                motif,
                par_compte_id: None,
                version_source: version_de(maintenant),
                maintenant,
            },
        )
        .await?;
    }
    Ok(n)
}

/// Majorité, changement de lien : les destinataires de chaque tâche ouverte de l'élève sont recalculés.
pub(crate) async fn resynchroniser_eleve(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let rows = sqlx::query(
        "SELECT DISTINCT nature_code, objet_id FROM ent_afaire
          WHERE etablissement_id = $1 AND eleve_id = $2 AND etat IN ('ouverte', 'prise', 'en_attente')",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for r in rows {
        let (code, objet): (String, Uuid) = (r.get(0), r.get(1));
        let Some(nat) = nature(&code) else { continue };
        if !nat.derivee || nat.code == "vs.appel_a_faire" {
            continue;
        }
        resynchroniser_objet(
            conn,
            etab,
            nat.code,
            objet,
            MotifClotureAfaire::DroitPerdu,
            None,
            maintenant,
        )
        .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------------------------

/// Ligne de l'index telle que lue pour une personne.
struct LigneIndex {
    id: Uuid,
    nature: String,
    objet_id: Uuid,
    eleve_id: Option<Uuid>,
    eleve_prenom: Option<String>,
    titre: String,
    priorite: PrioriteAfaire,
    etat: EtatAfaire,
    echeance_le: Option<DateTime<Utc>>,
    nouveau: bool,
    annee: Option<String>,
    /// Porte par laquelle la personne voit la tâche (personne, file) : sert à la revérification.
    dest_personne: Option<Uuid>,
    dest_file: Option<String>,
}

impl PgEnt {
    /// Tâches ouvertes d'une nature, **telles que le propriétaire les dit maintenant** (port
    /// `taches_ouvertes`). L'appel à faire dépend du service de vie scolaire ; les autres natures
    /// sont lues directement dans leurs tables.
    pub async fn taches_source(
        &self,
        etab: Uuid,
        code: &str,
        objets: Option<&[Uuid]>,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<TacheSource>, EntDbError> {
        if code == "vs.appel_a_faire" {
            return self.taches_appel(etab, objets, maintenant).await;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        taches_ouvertes_sql(&mut tx, etab, code, objets).await
    }

    async fn taches_appel(
        &self,
        etab: Uuid,
        objets: Option<&[Uuid]>,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<TacheSource>, EntDbError> {
        let local = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            heure_locale(&mut tx, etab, maintenant).await?
        };
        let v = self.verifier_appels(etab, local.date(), local).await?;
        let mut out = Vec::new();
        for s in v.seances_sans_appel {
            let objet = objet_appel(s.creneau_id, local.date());
            if objets.is_some_and(|o| !o.contains(&objet)) {
                continue;
            }
            out.push(TacheSource {
                objet_id: objet,
                eleve_id: None,
                valeurs: vec![
                    (crate::afaire::Var::Classe, s.classe),
                    (
                        crate::afaire::Var::Heure,
                        s.heure_debut.format("%H:%M").to_string(),
                    ),
                ],
                destinataires: vec![Destinataire::Compte(s.prof_user_id)],
                echeance_le: None,
            });
        }
        Ok(out)
    }

    /// Rapprochement d'un établissement (§8.10) : l'index redevient exactement ce que disent les
    /// propriétaires. Idempotent ; ne copie aucune donnée, n'écrit aucun nom dans les journaux.
    pub async fn rapprocher_afaire(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<RapportRapprochement, EntDbError> {
        let mut rapport = RapportRapprochement::default();
        let version = version_de(maintenant);
        for n in NATURES.iter().filter(|n| n.derivee) {
            let source = self.taches_source(etab, n.code, None, maintenant).await?;
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let index: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT DISTINCT objet_id FROM ent_afaire
                  WHERE etablissement_id = $1 AND nature_code = $2
                    AND etat IN ('ouverte', 'prise', 'en_attente')",
            )
            .bind(etab)
            .bind(n.code)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let objets_source: HashSet<Uuid> = source.iter().map(|s| s.objet_id).collect();
            let mut ligne = LigneRapprochement {
                nature: n.code.to_string(),
                source: source.len() as i64,
                index: index.len() as i64,
                ajoutees: 0,
                fermees: 0,
            };
            for s in &source {
                let r =
                    appliquer_source(&mut tx, etab, n.code, s, None, version, maintenant).await?;
                ligne.ajoutees += (r.taches_creees + r.taches_rouvertes) as i64;
                rapport.rouvertes += r.taches_rouvertes as i64;
                rapport.destinataires_ajoutes += r.destinataires_ajoutes as i64;
            }
            for (objet,) in index {
                if objets_source.contains(&objet) {
                    continue;
                }
                ligne.fermees += fermer_tache(
                    &mut tx,
                    &FermetureTache {
                        etablissement_id: etab,
                        nature: n.code,
                        objet_id: objet,
                        destinataire: None,
                        motif: MotifClotureAfaire::SansObjet,
                        par_compte_id: None,
                        version_source: version,
                        maintenant,
                    },
                )
                .await? as i64;
            }
            rapport.ajoutees += ligne.ajoutees;
            rapport.fermees += ligne.fermees;
            rapport.natures.push(ligne);
            tx.commit().await.map_err(db)?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        escalader_echues(&mut tx, etab, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(rapport)
    }

    /// `afaire.rapprocher` : lancer un rapprochement à la demande (POR-205). Réservé aux porteurs du droit.
    pub async fn rapprocher_afaire_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<RapportRapprochement, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AfaireRapprocher)
            .await?;
        self.rapprocher_afaire(etab, maintenant).await
    }

    /// Reprise périodique (toutes les natures, tous les établissements) : lancée par le serveur.
    pub async fn rapprocher_afaire_tous(
        &self,
        maintenant: DateTime<Utc>,
    ) -> Result<i64, EntDbError> {
        let mut total = 0;
        for etab in crate::tenant_tx::etablissements_systeme(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("rapprochement_afaire"),
        )
        .await
        .map_err(db)?
        {
            let r = self.rapprocher_afaire(etab, maintenant).await?;
            total += r.ajoutees + r.fermees;
        }
        Ok(total)
    }

    /// Files auxquelles l'acteur a accès (membres **calculés** à partir de ses droits, jamais saisis).
    async fn files_de(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<Vec<String>, EntDbError> {
        let mut files = Vec::new();
        if Self::role_can_trancher_justificatif(role) {
            files.push(FILE_JUSTIFICATIFS.to_string());
        }
        if self
            .exiger_action_etab(etab, user, role, Action::LiensSuspendre)
            .await
            .is_ok()
        {
            files.push(FILE_PROTECTIONS.to_string());
            files.push(crate::afaire::FILE_DECISIONS.to_string());
        }
        if self
            .exiger_action_etab(etab, user, role, Action::AfaireRapprocher)
            .await
            .is_ok()
        {
            files.push(crate::afaire::FILE_ADMIN.to_string());
        }
        Ok(files)
    }

    /// `GET /afaire` : l'à-faire de la personne connectée, **revérifié** à l'instant de la lecture.
    pub async fn a_faire_de(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<AFaire, EntDbError> {
        let base = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let files = self.files_de(etab, user, role).await?;
        let lignes = self
            .lire_index(etab, user, &files, maintenant, None)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;

        // Revérification par nature (R-PO3) : une lecture par nature, pas une par tâche (R-PO12).
        let mut par_nature: HashMap<&str, Vec<Uuid>> = HashMap::new();
        for l in lignes.iter().filter(|l| l.etat.est_ouvert()) {
            par_nature
                .entry(l.nature.as_str())
                .or_default()
                .push(l.objet_id);
        }
        let mut vivantes: HashMap<(String, Uuid), Vec<Destinataire>> = HashMap::new();
        for (code, objets) in par_nature {
            let n = nature(code).ok_or(EntDbError::SaisieInvalide("nature inconnue"))?;
            if !n.derivee {
                continue;
            }
            // Une source en panne ne fait pas tomber la page : ses tâches ne sont pas montrées
            // (jamais de tâche fantôme), les autres natures restent lisibles.
            let src = if code == "vs.appel_a_faire" {
                self.taches_source(etab, code, Some(&objets), maintenant)
                    .await
            } else {
                taches_ouvertes_sql(&mut tx, etab, code, Some(&objets)).await
            };
            if let Ok(src) = src {
                for s in src {
                    vivantes.insert((code.to_string(), s.objet_id), s.destinataires);
                }
            }
        }

        let personnes = self.personnes_du_compte(&mut tx, etab, user).await?;
        let mut taches: Vec<TacheAfaire> = Vec::new();
        for l in lignes {
            let ouverte = l.etat.est_ouvert();
            let n = nature(&l.nature).ok_or(EntDbError::SaisieInvalide("nature inconnue"))?;
            if ouverte && n.derivee {
                let Some(dests) = vivantes.get(&(l.nature.clone(), l.objet_id)) else {
                    continue; // la source ne la dit plus ouverte
                };
                let attendu = if dests.is_empty() {
                    // Repli : seuls les membres de la file de repli la voient.
                    l.dest_file.as_deref() == Some(n.file_repli)
                } else {
                    dests.iter().any(|d| match d {
                        Destinataire::Personne(p) => personnes.contains(p),
                        Destinataire::Compte(c) => *c == user,
                        Destinataire::File(f) => files.contains(f),
                        Destinataire::Foyer(_) => false,
                    })
                };
                if !attendu {
                    continue;
                }
            } else if !ouverte {
                // Tâche close par un autre foyer, encore affichée jusqu'au point neutre (R-PO38) :
                // seulement si le lien est toujours actif (suspension = disparition immédiate).
                if !self.lien_actif_pour(&mut tx, etab, &l, user).await? {
                    continue;
                }
            }
            let priorite =
                priorite_effective(l.priorite, l.echeance_le, n.montee_due_heures, maintenant);
            let action = match n.action {
                Some(ActionDeclaree::Carte { libelle }) => Some(ActionTache::Carte {
                    libelle: libelle.to_string(),
                }),
                Some(ActionDeclaree::Lien { libelle, lien }) => Some(ActionTache::Lien {
                    libelle: libelle.to_string(),
                    lien: lien.to_string(),
                }),
                None => None,
            };
            taches.push(TacheAfaire {
                id: l.id,
                nature: l.nature,
                titre: l.titre,
                eleve_id: l.eleve_id,
                eleve_prenom: l.eleve_prenom,
                priorite,
                etat: l.etat,
                echeance_le: l.echeance_le,
                nouveau: l.nouveau && ouverte,
                action,
                annee: l.annee,
            });
        }
        // Tri (§8.5) : bloquantes, échéance la plus proche, date d'ouverture ; prénom puis id pour la stabilité.
        taches.sort_by(|a, b| {
            let ouv = |t: &TacheAfaire| {
                t.echeance_le
                    .map_or((1, DateTime::<Utc>::MAX_UTC), |e| (0, e))
            };
            (a.priorite.rang(), ouv(a), a.eleve_prenom.clone(), a.id).cmp(&(
                b.priorite.rang(),
                ouv(b),
                b.eleve_prenom.clone(),
                b.id,
            ))
        });
        // « Nouveau / vu » : booléen personnel, remis à faux après affichage (jamais une preuve de lecture).
        for t in taches.iter().filter(|t| t.nouveau) {
            sqlx::query(
                "INSERT INTO ent_afaire_etat_personnel (etablissement_id, afaire_id, compte_id, nouveau)
                 VALUES ($1, $2, $3, false)
                 ON CONFLICT (etablissement_id, afaire_id, compte_id)
                 DO UPDATE SET nouveau = false, modifie_le = now()",
            )
            .bind(etab)
            .bind(t.id)
            .bind(user)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let mention = if base == Role::Parent {
            self.mention_sans_autorite(&mut tx, etab, user).await?
        } else {
            None
        };
        tx.commit().await.map_err(db)?;

        let compteur = taches.iter().filter(|t| t.etat.est_ouvert()).count() as i64;
        let mut groupes: Vec<GroupeAfaire> = Vec::new();
        for t in &taches {
            match groupes.iter_mut().find(|g| g.eleve_id == t.eleve_id) {
                Some(g) => g.taches.push(t.clone()),
                None => groupes.push(GroupeAfaire {
                    eleve_id: t.eleve_id,
                    eleve_prenom: t.eleve_prenom.clone(),
                    taches: vec![t.clone()],
                }),
            }
        }
        // Ordre alphabétique des prénoms, sans hiérarchie entre enfants ; les tâches sans enfant à la fin.
        groupes.sort_by(|a, b| {
            (a.eleve_id.is_none(), a.eleve_prenom.clone())
                .cmp(&(b.eleve_id.is_none(), b.eleve_prenom.clone()))
        });
        Ok(AFaire {
            taches,
            groupes,
            compteur,
            mention,
        })
    }

    /// Port `compteur_a_faire` : entier seul (pastille). Gelé pendant la plage calme (R-PO31) puisque
    /// les tâches non encore visibles ne sont pas comptées.
    pub async fn compteur_a_faire(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<i64, EntDbError> {
        Ok(self
            .a_faire_de(etab, user, role, maintenant)
            .await?
            .compteur)
    }

    async fn personnes_du_compte(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
    ) -> Result<HashSet<Uuid>, EntDbError> {
        let rows = sqlx::query(
            "SELECT id FROM ent_personne
              WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
        )
        .bind(etab)
        .bind(user)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }

    async fn lien_actif_pour(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        l: &LigneIndex,
        user: Uuid,
    ) -> Result<bool, EntDbError> {
        let (Some(eleve), Some(_)) = (l.eleve_id, l.dest_personne) else {
            return Ok(false);
        };
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_eleve_lien_actif l
              WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.user_id = $3",
        )
        .bind(etab)
        .bind(eleve)
        .bind(user)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(n > 0)
    }

    /// Mention sobre quand aucun lien ne porte l'exercice de l'autorité parentale (R-PO8).
    async fn mention_sans_autorite(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
    ) -> Result<Option<String>, EntDbError> {
        let (sans, avec): (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE NOT (l.autorite_parentale AND l.peut_justifier_absence)),
                    count(*) FILTER (WHERE l.autorite_parentale AND l.peut_justifier_absence)
               FROM ent_eleve_lien_actif l
              WHERE l.etablissement_id = $1 AND l.user_id = $2",
        )
        .bind(etab)
        .bind(user)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok((sans > 0 && avec == 0).then(|| MENTION_SANS_AUTORITE.to_string()))
    }

    /// Lecture brute de l'index pour une personne (avant revérification).
    async fn lire_index(
        &self,
        etab: Uuid,
        user: Uuid,
        files: &[String],
        maintenant: DateTime<Utc>,
        tache: Option<Uuid>,
    ) -> Result<Vec<LigneIndex>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let personnes: Vec<Uuid> = self
            .personnes_du_compte(&mut tx, etab, user)
            .await?
            .into_iter()
            .collect();
        // Une tâche appartient à la personne par UNE de ses portes : personne, compte, file.
        // Plage calme : une tâche ouverte pendant la plage n'est pas encore visible (R-PO31) ;
        // une tâche close par un autre reste visible jusqu'au point neutre (R-PO38).
        let rows = sqlx::query(
            "SELECT DISTINCT ON (a.id)
                    a.id, a.nature_code, a.objet_id, a.eleve_id, e.prenom, a.titre_neutre, a.priorite,
                    a.etat, a.echeance_le, COALESCE(ep.nouveau, true), y.libelle,
                    d.personne_id, d.file_code
               FROM ent_afaire a
               JOIN ent_afaire_destinataire d
                 ON d.etablissement_id = a.etablissement_id AND d.afaire_id = a.id AND d.retire_le IS NULL
               LEFT JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
               LEFT JOIN ent_annee_scolaire y
                 ON y.etablissement_id = a.etablissement_id AND y.id = a.annee_scolaire_id
               LEFT JOIN ent_afaire_etat_personnel ep
                 ON ep.etablissement_id = a.etablissement_id AND ep.afaire_id = a.id AND ep.compte_id = $3
              WHERE a.etablissement_id = $1
                AND ($6::uuid IS NULL OR a.id = $6)
                AND ((d.type = 'personne' AND d.personne_id = ANY($2))
                  OR (d.type = 'compte' AND d.compte_id = $3)
                  OR (d.type = 'file' AND d.file_code = ANY($4)))
                AND ((a.etat IN ('ouverte', 'prise', 'en_attente')
                      AND a.visible_a_partir_de <= $5
                      AND (ep.reportee_jusqu_au IS NULL OR ep.reportee_jusqu_au <= $5)
                      AND ep.masquee_le IS NULL)
                  OR (a.etat = 'faite' AND d.masque_a_partir_de IS NOT NULL AND d.masque_a_partir_de > $5))
              ORDER BY a.id",
        )
        .bind(etab)
        .bind(&personnes)
        .bind(user)
        .bind(files)
        .bind(maintenant)
        .bind(tache)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let (Some(priorite), Some(etat)) = (
                PrioriteAfaire::parse(&r.get::<String, _>(6)),
                EtatAfaire::parse(&r.get::<String, _>(7)),
            ) else {
                continue;
            };
            out.push(LigneIndex {
                id: r.get(0),
                nature: r.get(1),
                objet_id: r.get(2),
                eleve_id: r.get(3),
                eleve_prenom: r.get(4),
                titre: r.get(5),
                priorite,
                etat,
                echeance_le: r.get(8),
                nouveau: r.get(9),
                annee: r.get(10),
                dest_personne: r.get(11),
                dest_file: r.get(12),
            });
        }
        Ok(out)
    }

    /// Retrouve **une** tâche de l'à-faire de la personne (revérifiée) ; `Ok(None)` si elle n'existe
    /// pas pour elle. `Err(DEJA_TRAITEE)` si elle la voyait mais qu'elle est close.
    async fn tache_visible(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        tache: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<TacheAfaire, EntDbError> {
        // Lecture directe de l'index pour distinguer « pas à moi » (404 neutre) de « déjà traitée ».
        let files = self.files_de(etab, user, role).await?;
        let lignes = self
            .lire_index(etab, user, &files, maintenant, Some(tache))
            .await?;
        let Some(l) = lignes.into_iter().next() else {
            // Soit inexistante, soit close pour quelqu'un qui l'avait : « déjà traitée » si elle fut la sienne.
            return if self.fut_destinataire(etab, user, tache).await? {
                Err(DEJA_TRAITEE)
            } else {
                Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
            };
        };
        if !l.etat.est_ouvert() {
            return Err(DEJA_TRAITEE);
        }
        let liste = self.a_faire_de(etab, user, role, maintenant).await?;
        liste
            .taches
            .into_iter()
            .find(|t| t.id == tache)
            .ok_or(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    }

    async fn fut_destinataire(
        &self,
        etab: Uuid,
        user: Uuid,
        tache: Uuid,
    ) -> Result<bool, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_afaire_destinataire d
               LEFT JOIN ent_personne p ON p.etablissement_id = d.etablissement_id AND p.id = d.personne_id
              WHERE d.etablissement_id = $1 AND d.afaire_id = $2
                AND ((d.type = 'personne' AND p.user_id = $3) OR (d.type = 'compte' AND d.compte_id = $3))",
        )
        .bind(etab)
        .bind(tache)
        .bind(user)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(n > 0)
    }

    /// `POST /afaire/{id}/apercu` : aperçu de la carte d'action (§8.7). Aucune écriture.
    pub async fn afaire_apercu(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        tache: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<ApercuAction, EntDbError> {
        let t = self
            .tache_visible(etab, user, role, tache, maintenant)
            .await?;
        if !a_carte_d_action(&t.nature) {
            return Err(EntDbError::SaisieInvalide(
                "cette tâche s'ouvre dans l'écran du propriétaire",
            ));
        }
        let objet = self.objet_de(etab, tache).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some((texte, champs)) = apercu_carte(&mut tx, etab, &t.nature, objet).await? else {
            return Err(DEJA_TRAITEE);
        };
        Ok(ApercuAction {
            tache_id: tache,
            texte,
            champs,
        })
    }

    async fn objet_de(&self, etab: Uuid, tache: Uuid) -> Result<Uuid, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (o,): (Uuid,) = sqlx::query_as(
            "SELECT objet_id FROM ent_afaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(tache)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(o)
    }

    /// `POST /afaire/{id}/executer` : appelle **la commande du propriétaire** (qui revérifie les droits),
    /// après confirmation explicite. Jamais d'action sans confirmation ; jamais deux fois.
    #[allow(clippy::too_many_arguments)]
    pub async fn afaire_executer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        tache: Uuid,
        confirme: bool,
        motif: Option<String>,
        maintenant: DateTime<Utc>,
    ) -> Result<ResultatAction, EntDbError> {
        let t = self
            .tache_visible(etab, user, role, tache, maintenant)
            .await?;
        if !confirme {
            return Err(EntDbError::SaisieInvalide("confirmation requise"));
        }
        let objet = self.objet_de(etab, tache).await?;
        match t.nature.as_str() {
            "vs.justifier_absence" => {
                let motif = motif
                    .map(|m| m.trim().to_string())
                    .filter(|m| !m.is_empty());
                let Some(motif) = motif else {
                    return Err(EntDbError::SaisieInvalide("motif requis"));
                };
                if motif.chars().count() > 300 {
                    return Err(EntDbError::SaisieInvalide("motif trop long"));
                }
                self.justifier_absence_for_actor(etab, user, role, objet, motif)
                    .await?;
            }
            "vs.signer_carnet" => {
                self.signer_carnet_for_actor(etab, user, role, objet)
                    .await?;
            }
            "doc.signer" => {
                self.signer_doc_a_signer_for_actor(
                    role,
                    domain_eleve::SignerDocASigner {
                        etablissement_id: etab,
                        doc_id: objet,
                        actor_user_id: user,
                    },
                )
                .await?;
            }
            _ => {
                return Err(EntDbError::SaisieInvalide(
                    "cette tâche s'ouvre dans l'écran du propriétaire",
                ))
            }
        }
        // Fermeture immédiate (idempotente avec la réaction de l'outbox) : la liste est juste tout de suite.
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let code = nature(&t.nature)
            .ok_or(EntDbError::SaisieInvalide("nature inconnue"))?
            .code;
        resynchroniser_objet(
            &mut tx,
            etab,
            code,
            objet,
            MotifClotureAfaire::Faite,
            Some(user),
            maintenant,
        )
        .await?;
        // Une justification fait aussi naître la tâche « à trancher » côté vie scolaire.
        if code == "vs.justifier_absence" {
            resynchroniser_objet(
                &mut tx,
                etab,
                "vs.trancher_justificatif",
                objet,
                MotifClotureAfaire::SansObjet,
                Some(user),
                maintenant,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(ResultatAction {
            fait: true,
            message: "C'est fait.".into(),
        })
    }
}
