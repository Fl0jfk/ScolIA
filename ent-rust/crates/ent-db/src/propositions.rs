//! Socle 2 (l'élève), lot E5 — propositions : dépôt, décision, règles automatiques, lectures.
//!
//! PRINCIPE (§7.8, §8.3 de `02_ELEVE.md`) : **aucune donnée venue de l'extérieur n'écrit
//! directement dans la fiche** (import SIECLE, famille, élève, ancien établissement, personnel
//! qui signale une erreur). Elle devient une proposition (valeur actuelle → valeur proposée,
//! source, pièce, statut), acceptée par une règle automatique explicite ou par une personne.
//!
//! * ELV-18 [`PgEnt::deposer_proposition_famille`] (responsable légal, élève majeur ou
//!   émancipé), ELV-40 [`PgEnt::deposer_signalement_personnel`], [`PgEnt::deposer_proposition_systeme`]
//!   (imports IOP, socle 3 : interface générique, aucune route) ;
//! * ELV-19 [`PgEnt::decider_proposition`] (accepter tout ou partie, nature choisie, refus
//!   motivé), [`PgEnt::retirer_proposition`] (le déposant) ;
//! * lectures : [`PgEnt::propositions_pour_acteur`] (file du secrétariat), [`PgEnt::mes_propositions`]
//!   (le déposant seulement), [`PgEnt::champs_signalables`] (ce que l'établissement ouvre).
//!
//! Garanties : les valeurs (`actuel`, `propose`) sont **scellées** (`champs_chiffres`) ; un champ
//! verrouillé ou dont une source externe fait foi est respecté ; la famille ne propose que ce
//! que le réglage autorise (jamais l'INE) ; une règle automatique est toujours explicite, ne
//! s'applique jamais à un champ sensible ni en cas de conflit avec une modification locale plus
//! récente ; une version d'identité de source externe ne s'écrit que par une proposition décidée
//! (garde SQL, migration 289). Événements et journal : identifiants et NOMS de champs, jamais
//! une valeur. Chaque commande = UNE `TenantTx`.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    champ_catalogue, champ_catalogue_de, evaluer_auto, modif_depuis_valeurs, normaliser_identifiant,
    nom_regle_auto, planifier_version_identite, statut_final, valeur_identite, verifier_acceptation,
    verifier_depot, CibleProposition, ContexteAuto, DefChamp, DemandeVersion, Deposant,
    ErreurIdentifiant, ErreurIdentite, ErreurProposition, NatureVersion, OrigineProposition,
    ProvenanceIdentifiant, ReglageChamp, SourceProposition, SystemeIdentifiant, TypeValeur,
    MOTIFS_REFUS,
};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::{json, Value};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::identifiants::AjoutIdentifiant;
use crate::identite::PgIdentiteRepo;
use crate::outbox::Publier;
use crate::param_eleve::{regle_auto_active_tx, reglages_tx};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

const TABLE_CHAMP: &str = "ent_eleve_proposition_champ";
/// Plafond de propositions en attente par déposant (anti-inondation d'un compte famille).
const PLAFOND_EN_ATTENTE: i64 = 20;
/// Plafond de champs par proposition.
const PLAFOND_CHAMPS: usize = 20;

fn aujourdhui() -> NaiveDate {
    Utc::now().date_naive()
}

fn refus_prop(e: ErreurProposition) -> EntDbError {
    match e {
        ErreurProposition::Invalide(m) => EntDbError::SaisieInvalide(m),
        ErreurProposition::Refus(m) => EntDbError::Refus(RefusMetier::Interdit, m),
        ErreurProposition::Conflit(m) => EntDbError::ConflitDossier(m),
    }
}

fn erreur_identite(e: ErreurIdentite) -> EntDbError {
    match e {
        ErreurIdentite::Invalide(m) => EntDbError::SaisieInvalide(m),
        ErreurIdentite::Conflit(m) => EntDbError::ConflitDossier(m),
    }
}

fn erreur_identifiant(e: ErreurIdentifiant) -> EntDbError {
    match e {
        ErreurIdentifiant::Invalide(m) => EntDbError::SaisieInvalide(m),
        ErreurIdentifiant::Conflit(m) => EntDbError::ConflitDossier(m),
    }
}

// ───────────────────────── Types ─────────────────────────

/// Un champ d'une proposition (valeurs ouvertes : `actuel` est absent pour la famille).
#[derive(Debug, Clone)]
pub struct ChampProposition {
    pub champ: String,
    pub actuel: Option<Value>,
    pub propose: Value,
    /// `en_attente`, `acceptee`, `refusee` ou `obsolete`.
    pub decision: String,
}

/// En-tête d'une proposition.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EnteteProposition {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub cible: String,
    pub source: String,
    pub origine: Option<String>,
    pub source_ref: Option<Uuid>,
    pub depose_par: Option<Uuid>,
    pub piece_document_id: Option<Uuid>,
    pub commentaire: Option<String>,
    pub statut: String,
    pub regle_auto: Option<String>,
    pub base_version_id: Option<Uuid>,
    pub decision_code: Option<String>,
    pub decision_motif: Option<String>,
    pub decide_par: Option<Uuid>,
    pub decide_le: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
    pub expire_le: Option<DateTime<Utc>>,
}

const COLONNES: &str = "id, etablissement_id, eleve_id, cible, source, origine, source_ref,
    depose_par, piece_document_id, commentaire, statut, regle_auto, base_version_id,
    decision_code, decision_motif, decide_par, decide_le, cree_le, expire_le";

/// Une proposition et ses champs.
#[derive(Debug, Clone)]
pub struct PropositionEleve {
    pub entete: EnteteProposition,
    pub champs: Vec<ChampProposition>,
}

/// Dépôt d'une proposition (interface générique : imports IOP, socle 3, famille, personnel).
#[derive(Debug, Clone)]
pub struct DepotProposition {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub cible: CibleProposition,
    pub source: SourceProposition,
    pub origine: Option<OrigineProposition>,
    /// Lot d'import, demande, dossier d'admission…
    pub source_ref: Option<Uuid>,
    /// Champ du catalogue → valeur proposée (JSON ; `null` retire un champ facultatif).
    pub champs: Vec<(String, Value)>,
    pub piece_document_id: Option<Uuid>,
    pub commentaire: Option<String>,
    pub expire_le: Option<DateTime<Utc>>,
    /// Compte déposant (famille, élève, personnel) ; `None` pour un traitement du serveur.
    pub depose_par: Option<Uuid>,
}

/// Résultat d'un dépôt.
#[derive(Debug, Clone, Default)]
pub struct ResultatDepot {
    /// Proposition en attente (absente si tout a été appliqué automatiquement, ou s'il n'y a
    /// aucune différence).
    pub en_attente: Option<PropositionEleve>,
    /// Proposition appliquée automatiquement par une règle explicite.
    pub appliquee_auto: Option<PropositionEleve>,
    /// Champs de la proposition identiques à la fiche, écartés sans bruit (import idempotent).
    pub champs_identiques: Vec<String>,
    /// Pour chaque champ resté en attente : pourquoi la règle automatique ne s'est pas appliquée.
    pub raisons_non_auto: Vec<(String, &'static str)>,
}

impl ResultatDepot {
    pub fn aucune_difference(&self) -> bool {
        self.en_attente.is_none() && self.appliquee_auto.is_none()
    }
}

/// Décision d'une personne (ELV-19) : les champs acceptés, tout le reste est refusé.
#[derive(Debug, Clone, Default)]
pub struct DecisionProposition {
    pub proposition_id: Uuid,
    pub etablissement_id: Uuid,
    pub acteur: Uuid,
    pub champs_acceptes: Vec<String>,
    /// Nature de la version d'identité : `correction` ou `changement_etat_civil` (propositions
    /// d'une personne) ; `import` est implicite pour un flux machine. Sans objet pour un nom ou
    /// un prénom d'usage et pour un identifiant.
    pub nature: Option<NatureVersion>,
    pub motif_code: Option<String>,
    pub motif: Option<String>,
    /// Pièce exigée par un changement d'état civil (à défaut, celle de la proposition).
    pub piece_document_id: Option<Uuid>,
    pub effet_du: Option<NaiveDate>,
    /// Motif (code) du refus, obligatoire si un champ est refusé ; visible du déposant.
    pub refus_code: Option<String>,
    pub refus_motif: Option<String>,
}

/// Filtre de la file des propositions.
#[derive(Debug, Clone, Default)]
pub struct FiltrePropositions {
    pub statut: Option<String>,
    pub eleve_id: Option<Uuid>,
    pub source: Option<String>,
    pub champ: Option<String>,
    pub limite: Option<i64>,
    /// Pagination : propositions créées avant cette date.
    pub avant: Option<DateTime<Utc>>,
}

/// Un champ que la famille peut signaler.
#[derive(Debug, Clone)]
pub struct ChampSignalable {
    pub code: &'static str,
    pub libelle: &'static str,
    pub piece_requise: bool,
    pub sensible: bool,
}

// ───────────────────────── Accès aux données ─────────────────────────

fn coffre() -> ChampsSensibles {
    ChampsSensibles::depuis_env()
}

fn sceller_valeur(champ_id: Uuid, colonne: &str, v: &Value) -> Result<String, EntDbError> {
    let brut = serde_json::to_string(v).map_err(|_| EntDbError::SaisieInvalide("valeur illisible"))?;
    coffre()
        .sceller(TABLE_CHAMP, colonne, champ_id, &brut)
        .map_err(EntDbError::Domain)
}

fn ouvrir_valeur(champ_id: Uuid, colonne: &str, stocke: &str) -> Result<Value, EntDbError> {
    let clair = coffre()
        .ouvrir(TABLE_CHAMP, colonne, champ_id, stocke)
        .map_err(EntDbError::Domain)?;
    serde_json::from_str(&clair).map_err(|_| EntDbError::SaisieInvalide("valeur illisible"))
}

type LigneChamp = (Uuid, Uuid, String, Option<String>, Option<String>, String);

/// Champs de plusieurs propositions (valeurs ouvertes ; `avec_actuel` = false pour la famille).
async fn champs_de(
    conn: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
    avec_actuel: bool,
) -> Result<HashMap<Uuid, Vec<ChampProposition>>, EntDbError> {
    let lignes: Vec<LigneChamp> = sqlx::query_as(
        "SELECT id, proposition_id, champ, actuel, propose, decision
         FROM ent_eleve_proposition_champ
         WHERE etablissement_id = $1 AND proposition_id = ANY($2)
         ORDER BY proposition_id, champ",
    )
    .bind(etab)
    .bind(ids)
    .fetch_all(&mut *conn)
    .await?;
    let mut m: HashMap<Uuid, Vec<ChampProposition>> = HashMap::new();
    for (id, prop, champ, actuel, propose, decision) in lignes {
        let actuel = match (avec_actuel, actuel) {
            (true, Some(a)) => Some(ouvrir_valeur(id, "actuel", &a)?),
            _ => None,
        };
        let propose = match propose {
            Some(p) => ouvrir_valeur(id, "propose", &p)?,
            None => Value::Null,
        };
        m.entry(prop).or_default().push(ChampProposition {
            champ,
            actuel,
            propose,
            decision,
        });
    }
    Ok(m)
}

async fn assembler(
    conn: &mut PgConnection,
    etab: Uuid,
    entetes: Vec<EnteteProposition>,
    avec_actuel: bool,
) -> Result<Vec<PropositionEleve>, EntDbError> {
    let ids: Vec<Uuid> = entetes.iter().map(|e| e.id).collect();
    let mut champs = champs_de(conn, etab, &ids, avec_actuel).await?;
    Ok(entetes
        .into_iter()
        .map(|e| PropositionEleve {
            champs: champs.remove(&e.id).unwrap_or_default(),
            entete: e,
        })
        .collect())
}

async fn entete_verrouillee(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<Option<EnteteProposition>, EntDbError> {
    Ok(sqlx::query_as::<_, EnteteProposition>(&format!(
        "SELECT {COLONNES} FROM ent_eleve_proposition
         WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?)
}

/// Verrouille la proposition DANS L'ORDRE commun à tous les chemins : l'élève d'abord, puis la
/// proposition (un dépôt, une décision, un retrait ou une expiration simultanés ne s'interbloquent pas).
async fn verrouiller_proposition(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<Option<EnteteProposition>, EntDbError> {
    let l: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT eleve_id FROM ent_eleve_proposition WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((eleve,)) = l else { return Ok(None) };
    if let Some(eleve) = eleve {
        verrouiller_eleve(&mut *conn, etab, eleve).await?;
    }
    entete_verrouillee(conn, etab, id).await
}

async fn verrouiller_eleve(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<(), EntDbError> {
    if PgIdentiteRepo::verrouiller_eleve(conn, etab, eleve).await? {
        Ok(())
    } else {
        Err(domain_eleve::DomainError::NotFound.into())
    }
}

/// Valeur actuelle d'un identifiant de l'élève (actif, non révoqué, non contesté).
async fn identifiant_actuel(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    systeme: SystemeIdentifiant,
) -> Result<Option<String>, EntDbError> {
    let r: Option<(String,)> = sqlx::query_as(
        "SELECT valeur FROM ent_eleve_identifiant
         WHERE etablissement_id = $1 AND eleve_id = $2 AND systeme = $3
           AND valide_au IS NULL AND statut IN ('declare', 'a_verifier', 'confirme')
         ORDER BY enregistre_le DESC, id LIMIT 1",
    )
    .bind(etab)
    .bind(eleve)
    .bind(systeme.code())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(r.map(|(v,)| v))
}

fn systeme_du_champ(def: &DefChamp) -> Option<SystemeIdentifiant> {
    match def.type_valeur {
        TypeValeur::Identifiant(s) => Some(s),
        _ => None,
    }
}

/// Valeurs actuelles de la fiche pour des champs du catalogue (lecture dans la transaction).
async fn valeurs_actuelles(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    cible: CibleProposition,
    champs: &[String],
) -> Result<HashMap<String, Value>, EntDbError> {
    let mut m = HashMap::new();
    match cible {
        CibleProposition::Identite => {
            let base = PgIdentiteRepo::en_vigueur_a(conn, etab, eleve, aujourdhui())
                .await?
                .ok_or(domain_eleve::DomainError::NotFound)?;
            for c in champs {
                if let Some(v) = valeur_identite(&base, c) {
                    m.insert(c.clone(), v);
                }
            }
        }
        CibleProposition::Identifiant => {
            for c in champs {
                let s = champ_catalogue(c)
                    .and_then(systeme_du_champ)
                    .ok_or(EntDbError::SaisieInvalide("champ inconnu du catalogue"))?;
                let v = identifiant_actuel(conn, etab, eleve, s).await?;
                m.insert(c.clone(), v.map_or(Value::Null, |v| json!(v)));
            }
        }
        _ => return Err(EntDbError::SaisieInvalide("cible non prise en charge")),
    }
    Ok(m)
}

/// Différences entre la fiche et les valeurs proposées : `(champ, actuel, propose normalisé)`.
/// Les champs identiques à la fiche sont écartés. La validation des formats est celle de
/// `planifier_version` (identité) et de `normaliser_identifiant` : une seule autorité.
async fn calculer_differences(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    cible: CibleProposition,
    entrees: &[(String, Value)],
) -> Result<(Option<Uuid>, Vec<(String, Value, Value)>, Vec<String>), EntDbError> {
    let mut diffs = Vec::new();
    let mut identiques = Vec::new();
    match cible {
        CibleProposition::Identite => {
            let jour = aujourdhui();
            let base = PgIdentiteRepo::en_vigueur_a(conn, etab, eleve, jour)
                .await?
                .ok_or(domain_eleve::DomainError::NotFound)?;
            let dates = PgIdentiteRepo::dates_en_vigueur(conn, etab, eleve).await?;
            let modif = modif_depuis_valeurs(entrees).map_err(refus_prop)?;
            let plan = planifier_version_identite(
                &base,
                &dates,
                DemandeVersion {
                    nature: NatureVersion::Import,
                    modif,
                    effet_du: None,
                    motif_code: None,
                    motif: None,
                    piece_document_id: None,
                    source: "import".into(),
                    // Simple essai à blanc : rien n'est écrit.
                    source_ref: Some(Uuid::nil()),
                    acteur: None,
                },
                jour,
            );
            match plan {
                Err(ErreurIdentite::Invalide("aucune modification")) => {
                    identiques.extend(entrees.iter().map(|(c, _)| c.clone()));
                }
                Err(e) => return Err(erreur_identite(e)),
                Ok(plan) => {
                    let changes: Vec<&'static str> = plan
                        .champs
                        .iter()
                        .filter_map(|c| champ_catalogue_de(*c))
                        .collect();
                    for (c, _) in entrees {
                        if changes.contains(&c.as_str()) {
                            let actuel = valeur_identite(&base, c).unwrap_or(Value::Null);
                            let propose = valeur_identite(&plan.version, c).unwrap_or(Value::Null);
                            diffs.push((c.clone(), actuel, propose));
                        } else {
                            identiques.push(c.clone());
                        }
                    }
                }
            }
            Ok((Some(base.id), diffs, identiques))
        }
        CibleProposition::Identifiant => {
            for (c, v) in entrees {
                let def = champ_catalogue(c).ok_or(EntDbError::SaisieInvalide("champ inconnu du catalogue"))?;
                let systeme = systeme_du_champ(def)
                    .ok_or(EntDbError::SaisieInvalide("champ de cible différente"))?;
                let brut = v
                    .as_str()
                    .ok_or(EntDbError::SaisieInvalide("un identifiant se propose en texte"))?;
                let norm = normaliser_identifiant(systeme, brut).map_err(erreur_identifiant)?;
                let actuel = identifiant_actuel(conn, etab, eleve, systeme).await?;
                if actuel.as_deref() == Some(norm.as_str()) {
                    identiques.push(c.clone());
                } else {
                    diffs.push((
                        c.clone(),
                        actuel.map_or(Value::Null, |a| json!(a)),
                        json!(norm),
                    ));
                }
            }
            Ok((None, diffs, identiques))
        }
        _ => Err(EntDbError::SaisieInvalide(
            "cette cible n'est pas encore prise en charge",
        )),
    }
}

fn commentaire_valide(c: &Option<String>) -> Result<Option<String>, EntDbError> {
    match c {
        None => Ok(None),
        Some(s) => {
            let n = s.trim();
            if n.is_empty() {
                Ok(None)
            } else if n.chars().count() > 500 || n.chars().any(|c| c.is_control() && c != '\n') {
                Err(EntDbError::SaisieInvalide("commentaire invalide (500 caractères au plus)"))
            } else {
                Ok(Some(n.to_string()))
            }
        }
    }
}

/// Événement et journal : identifiants, cible, source, NOMS de champs. Jamais une valeur.
#[allow(clippy::too_many_arguments)]
async fn tracer(
    tx: &mut TenantTx<'static>,
    type_name: &'static str,
    action: &'static str,
    e: &EnteteProposition,
    champs: &[String],
    statut: &str,
    acteur: Option<Uuid>,
) -> Result<(), EntDbError> {
    let eleve = e.eleve_id.ok_or(EntDbError::SaisieInvalide("élève inconnu"))?;
    tx.publier(MetierEventDraft {
        etablissement_id: e.etablissement_id,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: eleve,
        eleve_id: Some(eleve),
        payload: json!({
            "proposition_id": e.id,
            "cible": e.cible,
            "source": e.source,
            "statut": statut,
            "champs": champs,
        }),
        actor_user_id: acteur.map(|a| a.to_string()),
    })
    .await?;
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,$3,'administratif',$4,$5,$6,false,$7, now())",
    )
    .bind(Uuid::new_v4())
    .bind(e.etablissement_id)
    .bind(eleve)
    .bind(format!("proposition_{action}"))
    .bind("Proposition de modification de la fiche de l'élève")
    .bind(json!({ "proposition_id": e.id, "cible": e.cible, "source": e.source, "statut": statut, "champs": champs }))
    .bind(acteur)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

// ───────────────────────── Dépôt ─────────────────────────

/// Qui dépose, côté contrôle des réglages (les traitements du serveur ne sont pas bornés par
/// `saisie_famille`, mais le sont par le catalogue et le verrou à la décision).
#[derive(Debug, Clone, Copy)]
enum Qui {
    Systeme,
    Famille,
    Personnel,
}

impl PgEnt {
    /// Insère la proposition (en attente) et ses champs scellés.
    async fn inserer_proposition(
        tx: &mut TenantTx<'static>,
        cmd: &DepotProposition,
        base_version: Option<Uuid>,
        commentaire: &Option<String>,
        diffs: &[(String, Value, Value)],
    ) -> Result<EnteteProposition, EntDbError> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve_proposition
                (id, etablissement_id, eleve_id, cible, source, origine, source_ref, depose_par,
                 piece_document_id, commentaire, base_version_id, expire_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(id)
        .bind(cmd.etablissement_id)
        .bind(cmd.eleve_id)
        .bind(cmd.cible.code())
        .bind(cmd.source.code())
        .bind(cmd.origine.map(|o| o.code()))
        .bind(cmd.source_ref)
        .bind(cmd.depose_par)
        .bind(cmd.piece_document_id)
        .bind(commentaire)
        .bind(base_version)
        .bind(cmd.expire_le)
        .execute(&mut **tx)
        .await?;
        for (champ, actuel, propose) in diffs {
            let cid = Uuid::new_v4();
            let actuel = sceller_valeur(cid, "actuel", actuel)?;
            let propose = sceller_valeur(cid, "propose", propose)?;
            sqlx::query(
                "INSERT INTO ent_eleve_proposition_champ
                    (id, etablissement_id, proposition_id, champ, actuel, propose)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(cid)
            .bind(cmd.etablissement_id)
            .bind(id)
            .bind(champ)
            .bind(actuel)
            .bind(propose)
            .execute(&mut **tx)
            .await?;
        }
        Ok(sqlx::query_as::<_, EnteteProposition>(&format!(
            "SELECT {COLONNES} FROM ent_eleve_proposition WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(cmd.etablissement_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await?)
    }

    /// Une proposition plus récente du même déposant sur le même champ remplace l'ancienne
    /// (un import hebdomadaire, une famille qui se corrige) : l'ancienne devient `obsolete`.
    async fn remplacer_anciennes(
        tx: &mut TenantTx<'static>,
        cmd: &DepotProposition,
        champs: &[String],
    ) -> Result<(), EntDbError> {
        let anciennes: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT p.id, c.id, c.champ FROM ent_eleve_proposition p
             JOIN ent_eleve_proposition_champ c
               ON c.etablissement_id = p.etablissement_id AND c.proposition_id = p.id
             WHERE p.etablissement_id = $1 AND p.eleve_id = $2 AND p.cible = $3
               AND p.source = $4 AND p.depose_par IS NOT DISTINCT FROM $5
               AND p.statut = 'en_attente' AND c.decision = 'en_attente' AND c.champ = ANY($6)
             FOR UPDATE OF p, c",
        )
        .bind(cmd.etablissement_id)
        .bind(cmd.eleve_id)
        .bind(cmd.cible.code())
        .bind(cmd.source.code())
        .bind(cmd.depose_par)
        .bind(champs)
        .fetch_all(&mut **tx)
        .await?;
        let mut touchees: Vec<Uuid> = Vec::new();
        for (prop, champ_id, _) in &anciennes {
            sqlx::query("UPDATE ent_eleve_proposition_champ SET decision = 'obsolete' WHERE etablissement_id = $1 AND id = $2")
                .bind(cmd.etablissement_id)
                .bind(champ_id)
                .execute(&mut **tx)
                .await?;
            if !touchees.contains(prop) {
                touchees.push(*prop);
            }
        }
        for prop in touchees {
            let (restants,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_eleve_proposition_champ
                 WHERE etablissement_id = $1 AND proposition_id = $2 AND decision = 'en_attente'",
            )
            .bind(cmd.etablissement_id)
            .bind(prop)
            .fetch_one(&mut **tx)
            .await?;
            if restants == 0 {
                sqlx::query(
                    "UPDATE ent_eleve_proposition
                     SET statut = 'obsolete', decision_code = 'remplacee', decide_le = now()
                     WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(cmd.etablissement_id)
                .bind(prop)
                .execute(&mut **tx)
                .await?;
            }
        }
        Ok(())
    }

    /// Cœur du dépôt, dans la transaction de l'appelant (aucun commit).
    async fn deposer_tx(
        tx: &mut TenantTx<'static>,
        cmd: &DepotProposition,
        qui: Qui,
    ) -> Result<ResultatDepot, EntDbError> {
        let (etab, eleve) = (cmd.etablissement_id, cmd.eleve_id);
        if !cmd.cible.applicable() {
            return Err(EntDbError::SaisieInvalide(
                "cette cible n'est pas encore prise en charge",
            ));
        }
        if cmd.champs.is_empty() || cmd.champs.len() > PLAFOND_CHAMPS {
            return Err(EntDbError::SaisieInvalide("1 à 20 champs par proposition"));
        }
        let mut vus: Vec<&str> = Vec::new();
        for (c, _) in &cmd.champs {
            let def = champ_catalogue(c).ok_or(EntDbError::SaisieInvalide("champ inconnu du catalogue"))?;
            if def.cible != cmd.cible {
                return Err(EntDbError::SaisieInvalide(
                    "tous les champs d'une proposition visent la même cible",
                ));
            }
            if vus.contains(&c.as_str()) {
                return Err(EntDbError::SaisieInvalide("champ en double"));
            }
            vus.push(c);
        }
        if matches!(qui, Qui::Systeme) && !cmd.source.deposable_par_systeme() {
            return Err(EntDbError::SaisieInvalide(
                "cette source est déposée par une personne connectée",
            ));
        }
        if matches!(qui, Qui::Famille) && !matches!(cmd.source, SourceProposition::Famille | SourceProposition::Eleve)
            || matches!(qui, Qui::Personnel) && cmd.source != SourceProposition::Personnel
        {
            return Err(EntDbError::SaisieInvalide("source incohérente avec le déposant"));
        }
        let commentaire = commentaire_valide(&cmd.commentaire)?;
        verrouiller_eleve(&mut *tx, etab, eleve).await?;

        // Droits de dépôt par champ : le réglage fait foi.
        let reglages = reglages_tx(&mut *tx, etab).await?;
        let reglage = |c: &str| reglages.get(c).cloned().unwrap_or_else(ReglageChamp::defaut);
        let deposant = match qui {
            Qui::Famille => Some(Deposant::Famille),
            Qui::Personnel => Some(Deposant::Personnel),
            Qui::Systeme => None,
        };
        let mut piece_requise = false;
        if let Some(dep) = deposant {
            for (c, _) in &cmd.champs {
                let def = champ_catalogue(c).expect("catalogue vérifié ci-dessus");
                verifier_depot(def, &reglage(c), dep).map_err(refus_prop)?;
                piece_requise |= matches!(dep, Deposant::Famille) && def.piece_famille;
            }
        }
        match cmd.piece_document_id {
            Some(p) => {
                if !PgIdentiteRepo::piece_de_l_eleve(&mut *tx, etab, eleve, p).await? {
                    return Err(EntDbError::SaisieInvalide(
                        "pièce justificative inconnue ou non admise pour cet élève",
                    ));
                }
            }
            None if piece_requise => {
                return Err(EntDbError::SaisieInvalide(
                    "une pièce justificative est requise pour ce champ",
                ));
            }
            None => {}
        }
        // Anti-inondation : un compte ne garde pas plus de PLAFOND_EN_ATTENTE propositions.
        if let Some(dep) = cmd.depose_par {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_eleve_proposition
                 WHERE etablissement_id = $1 AND depose_par = $2 AND statut = 'en_attente'",
            )
            .bind(etab)
            .bind(dep)
            .fetch_one(&mut **tx)
            .await?;
            if n >= PLAFOND_EN_ATTENTE {
                return Err(EntDbError::Refus(
                    RefusMetier::Conflit,
                    "trop de propositions en attente : attendre leur traitement",
                ));
            }
        }

        let (base_version, diffs, identiques) =
            calculer_differences(&mut *tx, etab, eleve, cmd.cible, &cmd.champs).await?;
        let mut resultat = ResultatDepot {
            champs_identiques: identiques,
            ..Default::default()
        };
        if diffs.is_empty() {
            return Ok(resultat);
        }
        let noms: Vec<String> = diffs.iter().map(|(c, _, _)| c.clone()).collect();
        Self::remplacer_anciennes(tx, cmd, &noms).await?;

        // Conflit : la version d'identité en vigueur est une modification LOCALE (correction,
        // changement ou usage saisis par une personne, ou acceptés d'une source locale), et non
        // une reprise, une création ou un import. Un import ne l'écrase alors jamais seul : il
        // attend une décision (la décision, qui compare les valeurs, tranche). Grain : la
        // version entière, pas le champ.
        let conflit_local = match (cmd.cible, base_version) {
            (CibleProposition::Identite, Some(v)) => {
                let (nature, source): (String, String) = sqlx::query_as(
                    "SELECT nature, source FROM ent_eleve_identite WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(v)
                .fetch_one(&mut **tx)
                .await?;
                matches!(nature.as_str(), "correction" | "changement_etat_civil" | "usage")
                    && source != "import"
            }
            _ => false,
        };
        // Partage : champs que les règles automatiques peuvent appliquer / champs à décider.
        let mut auto: Vec<(String, Value, Value)> = Vec::new();
        let mut manuel: Vec<(String, Value, Value)> = Vec::new();
        for d in diffs {
            let def = champ_catalogue(&d.0).expect("catalogue vérifié ci-dessus");
            let r = reglage(&d.0);
            let regle_active = cmd.source.automatisable()
                && regle_auto_active_tx(&mut *tx, etab, &d.0, cmd.source).await?;
            match evaluer_auto(ContexteAuto {
                def,
                reglage: &r,
                source: cmd.source,
                origine: cmd.origine,
                regle_active,
                conflit: conflit_local,
            }) {
                Ok(()) => auto.push(d),
                Err(raison) => {
                    resultat.raisons_non_auto.push((d.0.clone(), raison.code()));
                    manuel.push(d);
                }
            }
        }
        if !auto.is_empty() {
            // Point de reprise AVANT d'insérer : si l'application échoue (ex. un INE confirmé
            // existe), la part automatique est défaite sans trace et rejoint les champs à décider
            // (aucune modification attendue ne disparaît).
            sqlx::query("SAVEPOINT application_auto").execute(&mut **tx).await?;
            let entete = Self::inserer_proposition(tx, cmd, base_version, &commentaire, &auto).await?;
            let noms_auto: Vec<String> = auto.iter().map(|(c, _, _)| c.clone()).collect();
            tracer(tx, types::ELEVE_PROPOSITION_DEPOSEE, "deposee", &entete, &noms_auto, "en_attente", cmd.depose_par)
                .await?;
            let acceptes: Vec<(String, Value)> = auto.iter().map(|(c, _, p)| (c.clone(), p.clone())).collect();
            let params = ParamsApplication {
                nature: Some(NatureVersion::Import),
                ..Default::default()
            };
            let regle = nom_regle_auto(cmd.source);
            match Self::finaliser(tx, &entete, &acceptes, &[], &params, None, Some(&regle)).await {
                Ok(()) => {
                    sqlx::query("RELEASE SAVEPOINT application_auto").execute(&mut **tx).await?;
                    // Relecture : l'en-tête a changé (statut, règle, date de décision).
                    let apres = entete_verrouillee(&mut *tx, etab, entete.id)
                        .await?
                        .ok_or(domain_eleve::DomainError::NotFound)?;
                    let mut v = assembler(&mut *tx, etab, vec![apres], true).await?;
                    resultat.appliquee_auto = v.pop();
                }
                Err(e @ (EntDbError::Sqlx(_) | EntDbError::Domain(_) | EntDbError::Event(_))) => {
                    // Panne technique : on ne masque pas, toute la transaction est défaite.
                    return Err(e);
                }
                Err(_) => {
                    sqlx::query("ROLLBACK TO SAVEPOINT application_auto").execute(&mut **tx).await?;
                    sqlx::query("RELEASE SAVEPOINT application_auto").execute(&mut **tx).await?;
                    for (c, _, _) in &auto {
                        resultat.raisons_non_auto.push((c.clone(), "application_impossible"));
                    }
                    manuel.extend(auto);
                }
            }
        }
        if !manuel.is_empty() {
            let entete = Self::inserer_proposition(tx, cmd, base_version, &commentaire, &manuel).await?;
            let noms_manuel: Vec<String> = manuel.iter().map(|(c, _, _)| c.clone()).collect();
            tracer(tx, types::ELEVE_PROPOSITION_DEPOSEE, "deposee", &entete, &noms_manuel, "en_attente", cmd.depose_par)
                .await?;
            let mut v = assembler(&mut *tx, etab, vec![entete], true).await?;
            resultat.en_attente = v.pop();
        }
        Ok(resultat)
    }

    /// ELV-18 : un responsable légal (lien actif) ou l'élève majeur/émancipé signale une
    /// erreur et propose une valeur. Seuls les champs que l'établissement ouvre à la famille
    /// sont proposables (jamais l'INE) ; une pièce est exigée pour l'état civil.
    pub async fn deposer_proposition_famille(
        &self,
        role: &str,
        mut cmd: DepotProposition,
        acteur: Uuid,
    ) -> Result<ResultatDepot, EntDbError> {
        let source = self
            .exiger_deposant_famille(cmd.etablissement_id, acteur, role, cmd.eleve_id)
            .await?;
        cmd.source = source;
        cmd.origine = None;
        cmd.depose_par = Some(acteur);
        // Ces champs viennent de la route : jamais de la famille.
        cmd.source_ref = None;
        cmd.expire_le = None;
        let mut tx = TenantTx::begin(self.pool(), cmd.etablissement_id).await?;
        let r = Self::deposer_tx(&mut tx, &cmd, Qui::Famille).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// ELV-40 : le personnel signale une erreur d'identité vue sur le terrain. Aucune
    /// modification directe : une proposition vers le secrétariat. Il faut pouvoir voir
    /// l'élève (`eleves.annuaire` sur lui).
    pub async fn deposer_signalement_personnel(
        &self,
        role: &str,
        mut cmd: DepotProposition,
        acteur: Uuid,
    ) -> Result<ResultatDepot, EntDbError> {
        if role == "parent" || role == "eleve" {
            return Err(EntDbError::Forbidden);
        }
        if !self
            .autoriser_eleve(cmd.etablissement_id, acteur, role, Action::ElevesAnnuaire, cmd.eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        cmd.source = SourceProposition::Personnel;
        cmd.origine = None;
        cmd.depose_par = Some(acteur);
        cmd.source_ref = None;
        cmd.expire_le = None;
        let mut tx = TenantTx::begin(self.pool(), cmd.etablissement_id).await?;
        let r = Self::deposer_tx(&mut tx, &cmd, Qui::Personnel).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Chemin **système** (imports IOP, socle 3) : interface générique, sans contrôle de rôle,
    /// jamais une route. Les valeurs identiques à la fiche sont écartées (un import rejoué ne
    /// crée rien) ; les règles automatiques explicites s'appliquent, le reste attend une décision.
    pub async fn deposer_proposition_systeme(
        &self,
        cmd: DepotProposition,
    ) -> Result<ResultatDepot, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), cmd.etablissement_id).await?;
        let r = Self::deposer_tx(&mut tx, &cmd, Qui::Systeme).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Le compte peut-il déposer pour cet élève au nom de la famille ? Responsable légal : lien
    /// actif (point unique `ent_eleve_lien_actif`) ; élève : son propre compte ET majeur ou
    /// émancipé (un mineur passe par ses responsables). Renvoie la source de la proposition.
    async fn exiger_deposant_famille(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<SourceProposition, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        match role {
            "parent" => {
                let (ok,): (bool,) = sqlx::query_as(
                    // Responsable légal : lien actif ET autorité parentale (un tiers lié à l'élève
                    // ne propose pas de modifier son état civil).
                    "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_actif
                                     WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3
                                       AND autorite_parentale)",
                )
                .bind(etab)
                .bind(user)
                .bind(eleve)
                .fetch_one(&mut *tx)
                .await?;
                if ok {
                    Ok(SourceProposition::Famille)
                } else {
                    Err(EntDbError::Forbidden)
                }
            }
            "eleve" => {
                let (ok,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (
                        SELECT 1 FROM ent_eleve_compte c
                        JOIN ent_eleve e ON e.etablissement_id = c.etablissement_id AND e.id = c.eleve_id
                        WHERE c.etablissement_id = $1 AND c.user_id = $2 AND c.eleve_id = $3
                          AND (
                            ent_eleve_majorite_le(e.date_naissance, e.date_naissance_precision, e.date_naissance_statut)
                                <= ent_jour_etablissement(c.etablissement_id)
                            OR EXISTS (SELECT 1 FROM ent_eleve_capacite k
                                        WHERE k.etablissement_id = c.etablissement_id AND k.eleve_id = c.eleve_id
                                          AND k.nature = 'emancipation' AND k.levee_le IS NULL
                                          AND k.du <= ent_jour_etablissement(c.etablissement_id)
                                          AND (k.au IS NULL OR k.au >= ent_jour_etablissement(c.etablissement_id)))
                          )
                     )",
                )
                .bind(etab)
                .bind(user)
                .bind(eleve)
                .fetch_one(&mut *tx)
                .await?;
                if ok {
                    Ok(SourceProposition::Eleve)
                } else {
                    Err(EntDbError::Forbidden)
                }
            }
            _ => Err(EntDbError::Forbidden),
        }
    }

    /// Champs que cette famille peut signaler pour cet élève (réglage de l'établissement).
    pub async fn champs_signalables(
        &self,
        role: &str,
        etab: Uuid,
        user: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<ChampSignalable>, EntDbError> {
        self.exiger_deposant_famille(etab, user, role, eleve).await?;
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        let reglages = reglages_tx(&mut conn, etab).await?;
        Ok(domain_eleve::CATALOGUE_CHAMPS_ELEVE
            .iter()
            .filter(|d| {
                let r = reglages.get(d.code).cloned().unwrap_or_else(ReglageChamp::defaut);
                d.cible.applicable() && verifier_depot(d, &r, Deposant::Famille).is_ok()
            })
            .map(|d| ChampSignalable {
                code: d.code,
                libelle: d.libelle,
                piece_requise: d.piece_famille,
                sensible: d.sensible,
            })
            .collect())
    }
}

// ───────────────────────── Décision et application ─────────────────────────

/// Paramètres d'application des champs acceptés.
#[derive(Debug, Clone, Default)]
struct ParamsApplication {
    nature: Option<NatureVersion>,
    motif_code: Option<String>,
    motif: Option<String>,
    piece: Option<Uuid>,
    effet_du: Option<NaiveDate>,
    refus_code: Option<String>,
    refus_motif: Option<String>,
}

fn provenance_identifiant(source: SourceProposition, origine: Option<OrigineProposition>) -> ProvenanceIdentifiant {
    use SourceProposition as S;
    match source {
        S::Import | S::Systeme | S::Educonnect => origine
            .and_then(|o| ProvenanceIdentifiant::parse(o.provenance_identifiant()))
            .unwrap_or(ProvenanceIdentifiant::AncienEtablissement),
        S::Famille | S::Eleve | S::VerificationCampagne | S::Admission => ProvenanceIdentifiant::DocumentFamille,
        S::Personnel => ProvenanceIdentifiant::Saisie,
        S::AncienEtablissement => ProvenanceIdentifiant::AncienEtablissement,
    }
}

impl PgEnt {
    /// Écrit la décision (champs puis proposition) PUIS applique les champs acceptés : la
    /// garde SQL de la migration 289 exige que la proposition soit déjà décidée quand la
    /// version d'identité qui la référence est insérée. Dans la transaction de l'appelant.
    async fn finaliser(
        tx: &mut TenantTx<'static>,
        e: &EnteteProposition,
        acceptes: &[(String, Value)],
        refuses: &[String],
        p: &ParamsApplication,
        acteur: Option<Uuid>,
        regle_auto: Option<&str>,
    ) -> Result<(), EntDbError> {
        let total = acceptes.len() + refuses.len();
        for (c, _) in acceptes {
            sqlx::query(
                "UPDATE ent_eleve_proposition_champ SET decision = 'acceptee'
                 WHERE etablissement_id = $1 AND proposition_id = $2 AND champ = $3",
            )
            .bind(e.etablissement_id)
            .bind(e.id)
            .bind(c)
            .execute(&mut **tx)
            .await?;
        }
        for c in refuses {
            sqlx::query(
                "UPDATE ent_eleve_proposition_champ SET decision = 'refusee'
                 WHERE etablissement_id = $1 AND proposition_id = $2 AND champ = $3",
            )
            .bind(e.etablissement_id)
            .bind(e.id)
            .bind(c)
            .execute(&mut **tx)
            .await?;
        }
        let statut = if regle_auto.is_some() {
            "appliquee_auto"
        } else {
            statut_final(acceptes.len(), total)
        };
        sqlx::query(
            "UPDATE ent_eleve_proposition
             SET statut = $3, regle_auto = $4, decision_code = $5, decision_motif = $6,
                 decide_par = $7, decide_le = now()
             WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e.etablissement_id)
        .bind(e.id)
        .bind(statut)
        .bind(regle_auto)
        .bind(&p.refus_code)
        .bind(&p.refus_motif)
        .bind(acteur)
        .execute(&mut **tx)
        .await?;
        if !acceptes.is_empty() {
            Self::appliquer(tx, e, acceptes, p, acteur).await?;
        }
        let noms: Vec<String> = acceptes.iter().map(|(c, _)| c.clone()).collect();
        tracer(tx, types::ELEVE_PROPOSITION_DECIDEE, "decidee", e, &noms, statut, acteur).await
    }

    /// Applique les champs acceptés selon la cible. Une seule voie d'écriture : les versions
    /// d'identité (`ecrire_version`) et les identifiants (`ecrire_identifiant`).
    async fn appliquer(
        tx: &mut TenantTx<'static>,
        e: &EnteteProposition,
        acceptes: &[(String, Value)],
        p: &ParamsApplication,
        acteur: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        let source = SourceProposition::parse(&e.source).ok_or(EntDbError::SaisieInvalide("source inconnue"))?;
        let origine = e.origine.as_deref().and_then(OrigineProposition::parse);
        let eleve = e.eleve_id.ok_or(EntDbError::SaisieInvalide("élève inconnu"))?;
        let etab = e.etablissement_id;
        match CibleProposition::parse(&e.cible) {
            Some(CibleProposition::Identite) => {
                let (usage, civil): (Vec<_>, Vec<_>) = acceptes
                    .iter()
                    .cloned()
                    .partition(|(c, _)| champ_catalogue(c).is_some_and(|d| d.usage));
                let jour = aujourdhui();
                // Une version est un instantané complet : une version « usage » et une version
                // d'état civil écrites séparément (et datées différemment) se contrediraient.
                // Si l'état civil change, les noms d'usage acceptés voyagent dans la MÊME
                // version ; sinon (usage seul) la version est de nature `usage`.
                if civil.is_empty() {
                    let base = PgIdentiteRepo::en_vigueur_a(&mut *tx, etab, eleve, jour)
                        .await?
                        .ok_or(domain_eleve::DomainError::NotFound)?;
                    let dates = PgIdentiteRepo::dates_en_vigueur(&mut *tx, etab, eleve).await?;
                    let motif_code = match source {
                        SourceProposition::Famille => Some("demande_famille".to_string()),
                        SourceProposition::Eleve => Some("demande_eleve".to_string()),
                        _ => None,
                    };
                    let plan = planifier_version_identite(
                        &base,
                        &dates,
                        DemandeVersion {
                            nature: NatureVersion::Usage,
                            modif: modif_depuis_valeurs(&usage).map_err(refus_prop)?,
                            effet_du: p.effet_du,
                            motif_code,
                            motif: None,
                            piece_document_id: None,
                            source: source.source_identite().into(),
                            source_ref: Some(e.id),
                            acteur,
                        },
                        jour,
                    )
                    .map_err(erreur_identite)?;
                    Self::ecrire_version(tx, plan, acteur).await?;
                } else {
                    let nature = match p.nature {
                        Some(n @ (NatureVersion::Correction | NatureVersion::ChangementEtatCivil)) => n,
                        Some(NatureVersion::Import) | None
                            if matches!(
                                source,
                                SourceProposition::Import | SourceProposition::Educonnect | SourceProposition::Systeme
                            ) =>
                        {
                            NatureVersion::Import
                        }
                        _ => {
                            return Err(EntDbError::SaisieInvalide(
                                "choisir la nature : correction ou changement d'état civil",
                            ))
                        }
                    };
                    let piece = p.piece.or(e.piece_document_id);
                    let effet = p.effet_du.unwrap_or(jour);
                    let base = match nature {
                        NatureVersion::ChangementEtatCivil => {
                            match PgIdentiteRepo::en_vigueur_a(&mut *tx, etab, eleve, effet).await? {
                                Some(b) => Some(b),
                                None => PgIdentiteRepo::plus_ancienne(&mut *tx, etab, eleve).await?,
                            }
                        }
                        _ => PgIdentiteRepo::en_vigueur_a(&mut *tx, etab, eleve, jour).await?,
                    }
                    .ok_or(domain_eleve::DomainError::NotFound)?;
                    let dates = PgIdentiteRepo::dates_en_vigueur(&mut *tx, etab, eleve).await?;
                    let tous: Vec<(String, Value)> = civil.into_iter().chain(usage).collect();
                    let plan = planifier_version_identite(
                        &base,
                        &dates,
                        DemandeVersion {
                            nature,
                            modif: modif_depuis_valeurs(&tous).map_err(refus_prop)?,
                            effet_du: (nature == NatureVersion::ChangementEtatCivil).then_some(effet),
                            motif_code: (nature != NatureVersion::Import).then(|| p.motif_code.clone()).flatten(),
                            motif: p.motif.clone(),
                            piece_document_id: piece,
                            source: source.source_identite().into(),
                            source_ref: Some(e.id),
                            acteur,
                        },
                        jour,
                    )
                    .map_err(erreur_identite)?;
                    Self::ecrire_version(tx, plan, acteur).await?;
                }
                Ok(())
            }
            Some(CibleProposition::Identifiant) => {
                for (champ, valeur) in acceptes {
                    let systeme = champ_catalogue(champ)
                        .and_then(systeme_du_champ)
                        .ok_or(EntDbError::SaisieInvalide("champ inconnu du catalogue"))?;
                    let valeur = valeur
                        .as_str()
                        .ok_or(EntDbError::SaisieInvalide("un identifiant se propose en texte"))?;
                    Self::ecrire_identifiant(
                        tx,
                        &AjoutIdentifiant {
                            etablissement_id: etab,
                            eleve_id: eleve,
                            systeme,
                            valeur: valeur.to_string(),
                            provenance: provenance_identifiant(source, origine),
                            valide_du: None,
                            preuve_document_id: e.piece_document_id,
                            acteur,
                        },
                    )
                    .await?;
                }
                Ok(())
            }
            _ => Err(EntDbError::SaisieInvalide(
                "cette cible n'est pas encore prise en charge",
            )),
        }
    }

    /// ELV-19 : décide une proposition (accepter tout ou partie, nature choisie, ou refuser avec
    /// un motif). `eleves.propositions.traiter`, portée établissement (contrôlé ici). Un champ
    /// verrouillé ou dont une source externe fait foi ne s'accepte pas (sauf proposition de
    /// cette source) ; une fiche modifiée depuis le dépôt (conflit) interdit l'acceptation.
    pub async fn decider_proposition(
        &self,
        role: &str,
        d: DecisionProposition,
    ) -> Result<PropositionEleve, EntDbError> {
        let etab = d.etablissement_id;
        if !self
            .a_action_etablissement(etab, d.acteur, role, Action::ElevesPropositionsTraiter)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Ordre des verrous identique à celui du dépôt (élève, puis proposition).
        let e = verrouiller_proposition(&mut tx, etab, d.proposition_id)
            .await?
            .ok_or(domain_eleve::DomainError::NotFound)?;
        let eleve = e.eleve_id.ok_or(EntDbError::SaisieInvalide("élève inconnu"))?;
        if e.statut != "en_attente" {
            return Err(EntDbError::ConflitDossier("cette proposition est déjà décidée"));
        }
        let source = SourceProposition::parse(&e.source).ok_or(EntDbError::SaisieInvalide("source inconnue"))?;
        let origine = e.origine.as_deref().and_then(OrigineProposition::parse);
        let cible = CibleProposition::parse(&e.cible).ok_or(EntDbError::SaisieInvalide("cible inconnue"))?;
        let champs = champs_de(&mut tx, etab, &[e.id], true).await?.remove(&e.id).unwrap_or_default();
        let en_attente: Vec<&ChampProposition> =
            champs.iter().filter(|c| c.decision == "en_attente").collect();
        if en_attente.is_empty() {
            return Err(EntDbError::ConflitDossier("cette proposition n'a plus de champ à décider"));
        }
        for c in &d.champs_acceptes {
            if !en_attente.iter().any(|x| &x.champ == c) {
                return Err(EntDbError::SaisieInvalide("champ absent de la proposition"));
            }
        }
        let acceptes: Vec<(String, Value)> = en_attente
            .iter()
            .filter(|c| d.champs_acceptes.contains(&c.champ))
            .map(|c| (c.champ.clone(), c.propose.clone()))
            .collect();
        let refuses: Vec<String> = en_attente
            .iter()
            .filter(|c| !d.champs_acceptes.contains(&c.champ))
            .map(|c| c.champ.clone())
            .collect();
        // Refus : motif codé obligatoire (visible du déposant), texte libre pour « autre ».
        let mut p = ParamsApplication {
            nature: d.nature,
            motif_code: d.motif_code.clone(),
            motif: d.motif.clone(),
            piece: d.piece_document_id,
            effet_du: d.effet_du,
            ..Default::default()
        };
        if !refuses.is_empty() {
            let code = d
                .refus_code
                .as_deref()
                .ok_or(EntDbError::SaisieInvalide("un refus exige un motif (code)"))?;
            if !MOTIFS_REFUS.contains(&code) {
                return Err(EntDbError::SaisieInvalide("motif de refus inconnu"));
            }
            let texte = d.refus_motif.as_deref().map(str::trim).filter(|t| !t.is_empty());
            if code == "autre" && texte.is_none() {
                return Err(EntDbError::SaisieInvalide("le motif « autre » exige une précision en texte"));
            }
            if texte.is_some_and(|t| t.chars().count() > 500 || t.chars().any(|c| c.is_control() && c != '\n')) {
                return Err(EntDbError::SaisieInvalide("motif invalide (500 caractères au plus)"));
            }
            p.refus_code = Some(code.to_string());
            p.refus_motif = texte.map(str::to_string);
        }
        // Verrous et conflits : seulement pour ce qu'on accepte.
        if !acceptes.is_empty() {
            let reglages = reglages_tx(&mut tx, etab).await?;
            for (c, _) in &acceptes {
                let r = reglages.get(c).cloned().unwrap_or_else(ReglageChamp::defaut);
                verifier_acceptation(&r, source, origine).map_err(refus_prop)?;
            }
            let noms: Vec<String> = acceptes.iter().map(|(c, _)| c.clone()).collect();
            let courant = valeurs_actuelles(&mut tx, etab, eleve, cible, &noms).await?;
            for c in champs.iter().filter(|c| noms.contains(&c.champ)) {
                if courant.get(&c.champ) != c.actuel.as_ref() {
                    return Err(EntDbError::ConflitDossier(
                        "la fiche a changé depuis le dépôt : refuser la proposition ou la redéposer",
                    ));
                }
            }
        }
        Self::finaliser(&mut tx, &e, &acceptes, &refuses, &p, Some(d.acteur), None).await?;
        let apres = entete_verrouillee(&mut tx, etab, e.id)
            .await?
            .ok_or(domain_eleve::DomainError::NotFound)?;
        let mut v = assembler(&mut tx, etab, vec![apres], true).await?;
        tx.commit().await?;
        Ok(v.remove(0))
    }

    /// Le déposant retire sa proposition tant qu'elle est en attente.
    pub async fn retirer_proposition(
        &self,
        etab: Uuid,
        user: Uuid,
        proposition_id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let e = verrouiller_proposition(&mut tx, etab, proposition_id)
            .await?
            .ok_or(domain_eleve::DomainError::NotFound)?;
        // Un autre compte ne retire pas la demande de quelqu'un d'autre ; même réponse qu'une
        // proposition inexistante (aucune fuite d'existence).
        if e.depose_par != Some(user) {
            return Err(domain_eleve::DomainError::NotFound.into());
        }
        if e.statut != "en_attente" {
            return Err(EntDbError::ConflitDossier("cette proposition est déjà décidée"));
        }
        sqlx::query(
            "UPDATE ent_eleve_proposition_champ SET decision = 'obsolete'
             WHERE etablissement_id = $1 AND proposition_id = $2 AND decision = 'en_attente'",
        )
        .bind(etab)
        .bind(e.id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_eleve_proposition
             SET statut = 'obsolete', decision_code = 'retiree', decide_par = $3, decide_le = now()
             WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(e.id)
        .bind(user)
        .execute(&mut *tx)
        .await?;
        tracer(&mut tx, types::ELEVE_PROPOSITION_DECIDEE, "retiree", &e, &[], "obsolete", Some(user)).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Tâche de fond : les propositions en attente dont la date d'échéance est passée
    /// deviennent `expiree`. Aucune durée par défaut (Q-E15 : durées à valider par le DPO) :
    /// seules celles qui ont une échéance (posée par un import) expirent. Renvoie le nombre.
    pub async fn expirer_propositions_echues(&self) -> Result<i64, EntDbError> {
        let dues: Vec<(Uuid, Uuid)> = crate::tenant_tx::lire_par_etablissement(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::ExpirationPropositions,
            "SELECT etablissement_id, id FROM ent_eleve_proposition
             WHERE statut = 'en_attente' AND expire_le IS NOT NULL AND expire_le < now()
             ORDER BY expire_le LIMIT 500",
        )
        .await?;
        let mut n = 0;
        for (etab, id) in dues {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            let Some(e) = verrouiller_proposition(&mut tx, etab, id).await? else { continue };
            if e.statut != "en_attente" {
                continue;
            }
            sqlx::query(
                "UPDATE ent_eleve_proposition_champ SET decision = 'obsolete'
                 WHERE etablissement_id = $1 AND proposition_id = $2 AND decision = 'en_attente'",
            )
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE ent_eleve_proposition
                 SET statut = 'expiree', decision_code = 'expiree', decide_le = now()
                 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            tracer(&mut tx, types::ELEVE_PROPOSITION_DECIDEE, "expiree", &e, &[], "expiree", None).await?;
            tx.commit().await?;
            n += 1;
        }
        Ok(n)
    }
}

// ───────────────────────── Lectures ─────────────────────────

impl PgEnt {
    /// File des propositions de l'établissement (secrétariat). `eleves.propositions.traiter`.
    /// Les valeurs sont celles de l'état civil (G2) et des identifiants (G3) : la lecture est
    /// journalisée (acteur et objet, jamais les valeurs).
    pub async fn propositions_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        f: FiltrePropositions,
    ) -> Result<Vec<PropositionEleve>, EntDbError> {
        if !self
            .a_action_etablissement(etab, acteur, role, Action::ElevesPropositionsTraiter)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let limite = f.limite.unwrap_or(50).clamp(1, 200);
        let entetes = sqlx::query_as::<_, EnteteProposition>(&format!(
            "SELECT {COLONNES} FROM ent_eleve_proposition p
             WHERE etablissement_id = $1
               AND ($2::text IS NULL OR statut = $2)
               AND ($3::uuid IS NULL OR eleve_id = $3)
               AND ($4::text IS NULL OR source = $4)
               AND ($5::text IS NULL OR EXISTS (SELECT 1 FROM ent_eleve_proposition_champ c
                                                 WHERE c.etablissement_id = p.etablissement_id
                                                   AND c.proposition_id = p.id AND c.champ = $5))
               AND ($6::timestamptz IS NULL OR cree_le < $6)
             ORDER BY cree_le DESC, id
             LIMIT $7"
        ))
        .bind(etab)
        .bind(&f.statut)
        .bind(f.eleve_id)
        .bind(&f.source)
        .bind(&f.champ)
        .bind(f.avant)
        .bind(limite)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        let l = assembler(&mut conn, etab, entetes, true).await?;
        drop(conn);
        self.tracer_acces_sensible(
            etab,
            acteur,
            role,
            f.eleve_id,
            "propositions",
            "consultation",
            "Consultation des propositions de modification",
        )
        .await;
        Ok(l)
    }

    /// Une proposition (secrétariat). `eleves.propositions.traiter`.
    pub async fn proposition_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        id: Uuid,
    ) -> Result<PropositionEleve, EntDbError> {
        if !self
            .a_action_etablissement(etab, acteur, role, Action::ElevesPropositionsTraiter)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        let e = sqlx::query_as::<_, EnteteProposition>(&format!(
            "SELECT {COLONNES} FROM ent_eleve_proposition WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(domain_eleve::DomainError::NotFound)?;
        let eleve = e.eleve_id;
        let mut v = assembler(&mut conn, etab, vec![e], true).await?;
        drop(conn);
        self.tracer_acces_sensible(
            etab,
            acteur,
            role,
            eleve,
            "propositions",
            "consultation",
            "Consultation d'une proposition de modification",
        )
        .await;
        Ok(v.remove(0))
    }

    /// Les propositions que CE compte a déposées pour cet élève (famille, élève majeur) : jamais
    /// celles d'un autre responsable (un prénom d'usage demandé par l'un ne se voit pas de
    /// l'autre), jamais la valeur actuelle de la fiche.
    pub async fn mes_propositions(
        &self,
        role: &str,
        etab: Uuid,
        user: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<PropositionEleve>, EntDbError> {
        self.exiger_deposant_famille(etab, user, role, eleve).await?;
        let entetes = sqlx::query_as::<_, EnteteProposition>(&format!(
            "SELECT {COLONNES} FROM ent_eleve_proposition
             WHERE etablissement_id = $1 AND eleve_id = $2 AND depose_par = $3
             ORDER BY cree_le DESC, id LIMIT 100"
        ))
        .bind(etab)
        .bind(eleve)
        .bind(user)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        assembler(&mut conn, etab, entetes, false).await
    }
}
