//! Socle 3, lot I3 — **pièces réelles** d'un dossier d'inscription.
//! Conception : `docs/socles/03_INSCRIPTIONS.md` §8.6, §11.3 (INS-33 à INS-38), §15.2, §15.4.
//!
//! Le socle 3 garde l'**exigence** de sa démarche (quelle pièce, à quel moment, bloquante ou non, sous quelle
//! condition) et l'état de conformité du dossier. Le fichier, lui, est une pièce du **socle 7**
//! (`documents_pieces`) : le catalogue des types, le contrôle du type réel à l'entrée, la quarantaine,
//! l'antivirus (fermé par défaut), la conversion des photos en un PDF sans métadonnées, le PDF désarmé,
//! l'empreinte, la machine à états gardée par la base et la purge des objets. Rien de cela n'est recodé ici.
//!
//! * **Règles** (`ent_inscription_piece_regle`) : une règle par pièce et par campagne, avec des conditions
//!   typées en lignes (D-036) : « telle pièce si tel cas » (responsables de foyers distincts, tuteur, niveau,
//!   régime, autre établissement d'origine…). Les règles se **réappliquent** aux dossiers ouverts quand la
//!   campagne ou la situation change ; une pièce dont la condition tombe est conservée mais ne compte plus.
//! * **Dépôt** : la famille (déposant vérifié) ou le secrétariat (guichet, scan). Un PDF ou une à cinquante
//!   photos, **type réel contrôlé sur le contenu**, taille bornée, quarantaine puis analyse. Le sujet de la
//!   pièce du socle 7 est le **dossier** (l'élève n'existe pas avant la validation).
//! * **Pièce de famille** (`portee = demarche`) : déposée **une fois** pour tous les enfants de la démarche.
//!   Pour une pièce de portée « foyer » du catalogue demandée enfant par enfant, la réutilisation est
//!   **proposée**, jamais appliquée en silence.
//! * **Contrôle** : accepter, refuser (motif type) ou dispenser (motif). Seul un fichier reçu (sain) peut être
//!   accepté ; une pièce infectée ou illisible repasse « refusée » toute seule.
//! * **Rattachement** (validation) : chaque pièce acceptée devient une pièce du socle 7 au sujet de l'élève
//!   (mêmes objets, aucune copie d'octets). **Purge** : les fichiers des dossiers non aboutis sont détruits au
//!   magasin après un délai ; la ligne et son historique restent.

use std::collections::{HashMap, HashSet};

use ent_auth::Action;
use ent_events::{types, AggregateKind};
use ent_types::inscriptions_pieces::{
    BilanPurgeInscriptionDto, ConditionPieceDto, ControlePieceBody, DepotPieceDto, PieceAttendueDto,
    ReglePieceDto,
};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::documents_coffre::jour_etab;
use crate::documents_pieces_dossier::{
    accepter_tx, rattacher_a_eleve_tx, refuser_tx, retirer_si_orphelin_tx,
};
use crate::documents_pieces::{
    decision, fin_de_validite, motif_interdit, preparer_depot, systeme, type_piece, DepotInsere, PieceLue, PortePiece,
};
use crate::documents_repo::journaliser;
use crate::inscriptions::{commit, conflit, dossier_tx, introuvable, invalide, publier, suivi_tx, DossierRow};
use crate::inscriptions_etats::etat_effectif;
use crate::ref_commun::{db, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Dépôts de fichiers par dossier et par 24 h (anti-abus, comme le plafond du socle 7).
pub const DEPOTS_PAR_DOSSIER_ET_JOUR: i64 = 40;
/// Délai avant la destruction des fichiers d'un dossier non abouti [R : fin de campagne + délai de recours,
/// durée à valider avec le DPO, §15.4].
pub const PURGE_NON_ABOUTIS_JOURS: i64 = 180;

const MOMENTS: [&str; 4] = ["depot", "admission", "confirmation", "rentree"];
const CLES_BOOL: [&str; 4] = [
    "foyers_distincts",
    "autorite_exclusive",
    "responsable_tuteur",
    "autre_etablissement",
];
const CLES_TEXTE: [&str; 3] = ["niveau", "regime", "nature"];
const NATURES: [&str; 4] = ["admission", "en_cours_annee", "reinscription", "affectes"];
const MOTIFS_REFUS: [&str; 6] = [
    "illisible",
    "incomplet",
    "perime",
    "ne_correspond_pas",
    "mauvais_document",
    "autre",
];

/// États terminaux : plus aucune pièce ne se dépose ni ne se contrôle.
const ETATS_CLOS: [&str; 11] = [
    "validee",
    "refuse",
    "retire",
    "desiste",
    "sans_suite",
    "annule",
    "non_reinscription_famille",
    "non_reinscription_etablissement",
    "sans_reponse",
    "non_presente",
    "reaffecte",
];

/// États dans lesquels un dossier est purgé (jamais abouti).
const ETATS_NON_ABOUTIS: [&str; 10] = [
    "refuse",
    "annule",
    "sans_suite",
    "retire",
    "desiste",
    "non_reinscription_famille",
    "non_reinscription_etablissement",
    "sans_reponse",
    "non_presente",
    "reaffecte",
];

/// Condition SQL commune : une pièce **bloquante** non satisfaite, à produire avant la validation.
pub(crate) const SQL_PIECE_BLOQUANTE_MANQUANTE: &str =
    "p.applicable AND p.bloquante AND p.moment <> 'rentree' AND p.statut NOT IN ('acceptee', 'dispensee')";

fn motif_refus_libelle(code: &str) -> &'static str {
    match code {
        "illisible" => "illisible",
        "incomplet" => "incomplet",
        "perime" => "périmé",
        "ne_correspond_pas" => "ne correspond pas à la pièce demandée",
        "mauvais_document" => "ce n'est pas le bon document",
        _ => "autre motif",
    }
}

/// Le dépôt est-il ouvert pour ce dossier ? (`etat` effectif : un dossier bloqué pour désaccord garde l'état
/// d'avant, la famille peut y déposer l'accord écrit qui le débloquera.)
fn depot_ouvert(nature: &str, etat: &str) -> bool {
    if ETATS_CLOS.contains(&etat) {
        return false;
    }
    match nature {
        "reinscription" => matches!(etat, "a_confirmer" | "confirmee" | "en_attente_passage"),
        "affectes" => matches!(etat, "importe" | "a_completer" | "complete"),
        _ => matches!(
            etat,
            "deposee" | "en_instruction" | "attente_famille" | "liste_attente" | "admis" | "confirme"
        ),
    }
}

fn est_clos(etat: &str) -> bool {
    ETATS_CLOS.contains(&etat)
}

// ───────────────────────────── Règles ─────────────────────────────

/// Valide l'ensemble des règles d'une campagne (avant toute écriture).
fn valider_regles(regles: &[ReglePieceDto]) -> Result<(), EntDbError> {
    if regles.len() > 40 {
        return Err(invalide("quarante pièces au plus par campagne"));
    }
    let mut codes = HashSet::new();
    for r in regles {
        let ok_code = !r.code_type.is_empty()
            && r.code_type.len() <= 40
            && r.code_type.starts_with(|c: char| c.is_ascii_lowercase())
            && r.code_type
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !ok_code {
            return Err(invalide("pièce : code (a-z, 0-9, _) obligatoire"));
        }
        if !codes.insert(r.code_type.as_str()) {
            return Err(invalide("code de pièce en double"));
        }
        let libelle = r.libelle.trim();
        if libelle.is_empty() || libelle.chars().count() > 160 {
            return Err(invalide("pièce : libellé de 1 à 160 caractères"));
        }
        if r.consigne.as_deref().is_some_and(|c| c.chars().count() > 500) {
            return Err(invalide("consigne trop longue (500 caractères)"));
        }
        if !MOMENTS.contains(&r.moment.as_str()) {
            return Err(invalide("moment : depot, admission, confirmation ou rentree"));
        }
        if !matches!(r.portee.as_str(), "dossier" | "demarche") {
            return Err(invalide("portée : dossier ou demarche"));
        }
        match type_piece(&r.code_type) {
            Some(t) => {
                if !t.demandable {
                    return Err(invalide("ce type de pièce ne se demande pas"));
                }
                if !t.copie_gardee {
                    return Err(invalide(
                        "une pièce d'identité n'est pas demandée par une campagne d'inscription (elle n'est pas conservée)",
                    ));
                }
                if r.portee == "demarche" && t.portee != PortePiece::Foyer {
                    return Err(invalide(
                        "seule une pièce de foyer (livret de famille, domicile…) se dépose une fois pour toute la démarche",
                    ));
                }
            }
            None => {
                // Code propre à la campagne : traité comme une « autre » pièce, motivée par son libellé ;
                // jamais ce qu'aucun établissement ne peut demander (§11.2 du socle 7).
                if motif_interdit(libelle) || r.consigne.as_deref().is_some_and(motif_interdit) {
                    return Err(invalide("cette pièce ne peut être demandée par aucun établissement"));
                }
                if r.portee == "demarche" {
                    return Err(invalide("une pièce de famille est un type du catalogue"));
                }
            }
        }
        let mut cles_bool = HashSet::new();
        for c in &r.conditions {
            if CLES_BOOL.contains(&c.cle.as_str()) {
                if !matches!(c.valeur.as_str(), "true" | "false") {
                    return Err(invalide("condition : valeur true ou false"));
                }
                if !cles_bool.insert(c.cle.as_str()) {
                    return Err(invalide("condition en double"));
                }
            } else if CLES_TEXTE.contains(&c.cle.as_str()) {
                if c.valeur.trim().is_empty() || c.valeur.chars().count() > 40 {
                    return Err(invalide("condition : valeur de 1 à 40 caractères"));
                }
                if c.cle == "nature" && !NATURES.contains(&c.valeur.as_str()) {
                    return Err(invalide("condition : nature inconnue"));
                }
            } else {
                return Err(invalide("condition inconnue"));
            }
        }
    }
    Ok(())
}

/// Faits d'un dossier sur lesquels les conditions se jugent.
struct Faits {
    nature: String,
    niveau: Option<String>,
    regime: Option<String>,
    autre_etablissement: bool,
    foyers_distincts: bool,
    autorite_exclusive: bool,
    responsable_tuteur: bool,
}

impl Faits {
    fn satisfait(&self, conditions: &[(String, Option<bool>, Option<String>)]) -> bool {
        let mut textes: HashMap<&str, Vec<&str>> = HashMap::new();
        for (cle, b, t) in conditions {
            if let Some(b) = b {
                let fait = match cle.as_str() {
                    "foyers_distincts" => self.foyers_distincts,
                    "autorite_exclusive" => self.autorite_exclusive,
                    "responsable_tuteur" => self.responsable_tuteur,
                    "autre_etablissement" => self.autre_etablissement,
                    _ => return false,
                };
                if fait != *b {
                    return false;
                }
            } else if let Some(t) = t {
                textes.entry(cle.as_str()).or_default().push(t.as_str());
            }
        }
        for (cle, valeurs) in textes {
            let fait = match cle {
                "niveau" => self.niveau.as_deref(),
                "regime" => self.regime.as_deref(),
                "nature" => Some(self.nature.as_str()),
                _ => return false,
            };
            match fait {
                Some(f) if valeurs.iter().any(|v| v.eq_ignore_ascii_case(f)) => {}
                _ => return false,
            }
        }
        true
    }
}

async fn faits_tx(conn: &mut PgConnection, etab: Uuid, d: &DossierRow) -> Result<Faits, EntDbError> {
    let decl: Vec<(String, String, bool)> = sqlx::query_as(
        "SELECT foyer_cle, lien_code, autorite_exclusive_declaree
           FROM ent_inscription_declaration_responsable
          WHERE etablissement_id = $1 AND demarche_id = $2
            AND (cardinality(pour_enfants) = 0 OR $3 = ANY (pour_enfants))",
    )
    .bind(etab)
    .bind(d.demarche_id)
    .bind(d.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let foyers: HashSet<&str> = decl.iter().map(|x| x.0.as_str()).collect();
    Ok(Faits {
        nature: d.nature.clone(),
        niveau: d.niveau_demande.clone(),
        regime: d.regime_demande.clone(),
        autre_etablissement: d.etablissement_origine.as_deref().is_some_and(|e| !e.trim().is_empty()),
        foyers_distincts: foyers.len() > 1,
        autorite_exclusive: decl.iter().any(|x| x.2),
        responsable_tuteur: decl.iter().any(|x| x.1 == "tuteur"),
    })
}

type LigneRegle = (Uuid, String, String, Option<String>, String, bool, String, i16);

async fn regles_brutes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne: Uuid,
) -> Result<Vec<LigneRegle>, EntDbError> {
    sqlx::query_as(
        "SELECT id, code_type, libelle, consigne, moment, bloquante, portee, ordre
           FROM ent_inscription_piece_regle
          WHERE etablissement_id = $1 AND campagne_id = $2 ORDER BY ordre, code_type",
    )
    .bind(etab)
    .bind(campagne)
    .fetch_all(conn)
    .await
    .map_err(db)
}

/// Les règles d'une campagne, conditions comprises.
pub(crate) async fn regles_dto_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne: Uuid,
) -> Result<Vec<ReglePieceDto>, EntDbError> {
    let regles = regles_brutes_tx(conn, etab, campagne).await?;
    let conds: Vec<(Uuid, String, Option<bool>, Option<String>)> = sqlx::query_as(
        "SELECT c.regle_id, c.cle, c.valeur_bool, c.valeur_texte
           FROM ent_inscription_piece_regle_condition c
           JOIN ent_inscription_piece_regle r ON r.etablissement_id = c.etablissement_id AND r.id = c.regle_id
          WHERE c.etablissement_id = $1 AND r.campagne_id = $2 ORDER BY c.cle, c.valeur_texte",
    )
    .bind(etab)
    .bind(campagne)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(regles
        .into_iter()
        .map(|(id, code, libelle, consigne, moment, bloquante, portee, _)| ReglePieceDto {
            code_type: code,
            libelle,
            consigne,
            moment,
            bloquante,
            portee,
            conditions: conds
                .iter()
                .filter(|c| c.0 == id)
                .map(|(_, cle, b, t)| ConditionPieceDto {
                    cle: cle.clone(),
                    valeur: b.map_or_else(|| t.clone().unwrap_or_default(), |b| b.to_string()),
                })
                .collect(),
        })
        .collect())
}

/// Écrit l'ensemble des règles d'une campagne (remplacement). `simple_seulement` : écriture venue de
/// l'ancien formulaire de campagne (code, libellé, obligatoire) — les règles à condition, de moment ou de
/// portée particuliers qu'il ne sait pas exprimer ne sont jamais supprimées par lui.
pub(crate) async fn ecrire_regles_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne: Uuid,
    regles: &[ReglePieceDto],
    simple_seulement: bool,
) -> Result<(), EntDbError> {
    let gardees: HashSet<&str> = regles.iter().map(|r| r.code_type.as_str()).collect();
    let existantes: Vec<(Uuid, String, String, String, bool)> = sqlx::query_as(
        "SELECT r.id, r.code_type, r.moment, r.portee,
                EXISTS (SELECT 1 FROM ent_inscription_piece_regle_condition c
                         WHERE c.etablissement_id = r.etablissement_id AND c.regle_id = r.id)
           FROM ent_inscription_piece_regle r WHERE r.etablissement_id = $1 AND r.campagne_id = $2",
    )
    .bind(etab)
    .bind(campagne)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (id, code, moment, portee, a_conditions) in &existantes {
        let simple = !a_conditions && moment == "depot" && portee == "dossier";
        if !gardees.contains(code.as_str()) && (!simple_seulement || simple) {
            sqlx::query("DELETE FROM ent_inscription_piece_regle WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
        }
    }
    for (i, r) in regles.iter().enumerate() {
        let ordre = i16::try_from(i).unwrap_or(i16::MAX);
        let id: Uuid = if simple_seulement {
            // Libellé et caractère obligatoire seulement : moment, portée et conditions restent.
            sqlx::query_scalar(
                "INSERT INTO ent_inscription_piece_regle
                    (id, etablissement_id, campagne_id, code_type, libelle, moment, bloquante, portee, ordre)
                 VALUES ($1,$2,$3,$4,$5,'depot',$6,'dossier',$7)
                 ON CONFLICT (campagne_id, code_type) DO UPDATE
                    SET libelle = EXCLUDED.libelle, bloquante = EXCLUDED.bloquante, ordre = EXCLUDED.ordre
                 RETURNING id",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(campagne)
            .bind(&r.code_type)
            .bind(r.libelle.trim())
            .bind(r.bloquante)
            .bind(ordre)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?
        } else {
            sqlx::query_scalar(
                "INSERT INTO ent_inscription_piece_regle
                    (id, etablissement_id, campagne_id, code_type, libelle, consigne, moment, bloquante, portee, ordre)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                 ON CONFLICT (campagne_id, code_type) DO UPDATE
                    SET libelle = EXCLUDED.libelle, consigne = EXCLUDED.consigne, moment = EXCLUDED.moment,
                        bloquante = EXCLUDED.bloquante, portee = EXCLUDED.portee, ordre = EXCLUDED.ordre
                 RETURNING id",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(campagne)
            .bind(&r.code_type)
            .bind(r.libelle.trim())
            .bind(r.consigne.as_deref().map(str::trim).filter(|c| !c.is_empty()))
            .bind(&r.moment)
            .bind(r.bloquante)
            .bind(&r.portee)
            .bind(ordre)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?
        };
        if simple_seulement {
            continue;
        }
        sqlx::query("DELETE FROM ent_inscription_piece_regle_condition WHERE etablissement_id = $1 AND regle_id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        for c in &r.conditions {
            let (b, t) = if CLES_BOOL.contains(&c.cle.as_str()) {
                (Some(c.valeur == "true"), None)
            } else {
                (None, Some(c.valeur.trim().to_string()))
            };
            sqlx::query(
                "INSERT INTO ent_inscription_piece_regle_condition
                    (id, etablissement_id, regle_id, cle, valeur_bool, valeur_texte)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(&c.cle)
            .bind(b)
            .bind(t)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
    }
    Ok(())
}

/// Calcule les pièces attendues d'un dossier depuis les règles de sa campagne et la situation de la famille.
/// Idempotent. Rend les codes des pièces **nouvellement demandées** (ligne créée ou redevenue applicable).
pub(crate) async fn appliquer_regles_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
) -> Result<Vec<String>, EntDbError> {
    let d = dossier_tx(&mut *conn, etab, dossier_id).await?;
    let faits = faits_tx(&mut *conn, etab, &d).await?;
    let regles = regles_brutes_tx(&mut *conn, etab, d.campagne_id).await?;
    let conds: Vec<(Uuid, String, Option<bool>, Option<String>)> = sqlx::query_as(
        "SELECT c.regle_id, c.cle, c.valeur_bool, c.valeur_texte
           FROM ent_inscription_piece_regle_condition c
           JOIN ent_inscription_piece_regle r ON r.etablissement_id = c.etablissement_id AND r.id = c.regle_id
          WHERE c.etablissement_id = $1 AND r.campagne_id = $2",
    )
    .bind(etab)
    .bind(d.campagne_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let existantes: Vec<(String, bool)> = sqlx::query_as(
        "SELECT code, applicable FROM ent_inscription_piece WHERE etablissement_id = $1 AND dossier_id = $2",
    )
    .bind(etab)
    .bind(dossier_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut nouvelles = Vec::new();
    let mut presentes: HashSet<String> = HashSet::new();
    for (rid, code, libelle, _consigne, moment, bloquante, portee, _ordre) in &regles {
        let cs: Vec<(String, Option<bool>, Option<String>)> = conds
            .iter()
            .filter(|c| c.0 == *rid)
            .map(|c| (c.1.clone(), c.2, c.3.clone()))
            .collect();
        let applicable = faits.satisfait(&cs);
        presentes.insert(code.clone());
        let avant = existantes.iter().find(|e| &e.0 == code);
        if applicable {
            // Pièce de famille déjà déposée pour un autre enfant de la démarche : reprise d'office (un seul
            // fichier pour toute la démarche), sans nouvelle demande.
            let frere: Option<(Uuid, Uuid, String)> = if portee == "demarche" && avant.is_none() {
                sqlx::query_as(
                    "SELECT id, piece_doc_id, statut FROM ent_inscription_piece
                      WHERE etablissement_id = $1 AND demarche_id = $2 AND code = $3 AND dossier_id <> $4
                        AND piece_doc_id IS NOT NULL AND statut IN ('deposee', 'acceptee', 'reprise')
                      ORDER BY maj_le DESC NULLS LAST LIMIT 1",
                )
                .bind(etab)
                .bind(d.demarche_id)
                .bind(code)
                .bind(dossier_id)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?
            } else {
                None
            };
            sqlx::query(
                "INSERT INTO ent_inscription_piece
                    (id, etablissement_id, dossier_id, code, libelle, obligatoire, demarche_id, regle_id, moment,
                     bloquante, portee, applicable, statut, piece_doc_id, reprise_de_piece_id, maj_le)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$6,$10,true,$11,$12,$13, CASE WHEN $12::uuid IS NULL THEN NULL ELSE now() END)
                 ON CONFLICT (dossier_id, code) DO UPDATE
                    SET libelle = EXCLUDED.libelle, obligatoire = EXCLUDED.bloquante, bloquante = EXCLUDED.bloquante,
                        moment = EXCLUDED.moment, portee = EXCLUDED.portee, regle_id = EXCLUDED.regle_id,
                        demarche_id = EXCLUDED.demarche_id, applicable = true",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(dossier_id)
            .bind(code)
            .bind(libelle)
            .bind(*bloquante)
            .bind(d.demarche_id)
            .bind(rid)
            .bind(moment)
            .bind(portee)
            .bind(frere.as_ref().map_or("attendue", |f| if f.2 == "acceptee" { "acceptee" } else { "reprise" }))
            .bind(frere.as_ref().map(|f| f.1))
            .bind(frere.as_ref().map(|f| f.0))
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            if avant.is_none_or(|(_, a)| !a) && frere.is_none() {
                nouvelles.push(code.clone());
            }
        } else if avant.is_some_and(|(_, a)| *a) {
            sqlx::query(
                "UPDATE ent_inscription_piece SET applicable = false, maj_le = now()
                  WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3",
            )
            .bind(etab)
            .bind(dossier_id)
            .bind(code)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
    }
    // Pièce dont la règle a disparu : conservée, ne compte plus.
    for (code, applicable) in &existantes {
        if *applicable && !presentes.contains(code) {
            sqlx::query(
                "UPDATE ent_inscription_piece SET applicable = false, maj_le = now()
                  WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3",
            )
            .bind(etab)
            .bind(dossier_id)
            .bind(code)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
    }
    Ok(nouvelles)
}

/// Les fichiers refusés à l'analyse (infecté, illisible) ou retirés repassent seuls au statut qui permet un
/// nouveau dépôt : la famille n'attend pas un humain pour savoir que son fichier n'a pas été retenu.
pub(crate) async fn synchroniser_dossier_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_inscription_piece p
            SET statut = 'refusee',
                motif_refus = CASE d.etat
                    WHEN 'infectee' THEN 'Le fichier a été refusé par l''analyse de sécurité : déposez-en un autre.'
                    ELSE 'Le fichier n''a pas pu être lu : déposez un PDF ou des photos nettes.' END,
                maj_le = now()
           FROM ent_doc_piece d
          WHERE p.etablissement_id = $1 AND p.dossier_id = $2
            AND d.etablissement_id = p.etablissement_id AND d.id = p.piece_doc_id
            AND p.statut IN ('deposee', 'reprise') AND d.etat IN ('infectee', 'rejetee')",
    )
    .bind(etab)
    .bind(dossier_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_inscription_piece p SET statut = 'attendue', piece_doc_id = NULL, maj_le = now()
           FROM ent_doc_piece d
          WHERE p.etablissement_id = $1 AND p.dossier_id = $2
            AND d.etablissement_id = p.etablissement_id AND d.id = p.piece_doc_id
            AND p.statut IN ('deposee', 'reprise') AND d.etat = 'retiree'",
    )
    .bind(etab)
    .bind(dossier_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

type LignePiece = (
    Uuid,           // id
    String,         // code
    String,         // libelle
    Option<String>, // consigne
    String,         // moment
    bool,           // bloquante
    String,         // portee
    String,         // statut
    Option<String>, // fichier_etat
    Option<i32>,    // nb_pages
    Option<String>, // motif_refus
    Option<String>, // dispense_motif
    bool,           // applicable
);

/// Les pièces d'un dossier avec l'état de leur fichier (dossier verrouillé ou non par l'appelant).
pub(crate) async fn pieces_dossier_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
) -> Result<Vec<PieceAttendueDto>, EntDbError> {
    let d = dossier_tx(&mut *conn, etab, dossier_id).await?;
    synchroniser_dossier_tx(&mut *conn, etab, dossier_id).await?;
    let rows: Vec<LignePiece> = sqlx::query_as(
        "SELECT p.id, p.code, p.libelle, r.consigne, p.moment, p.bloquante, p.portee, p.statut,
                f.etat, f.nb_pages, p.motif_refus, p.dispense_motif, p.applicable
           FROM ent_inscription_piece p
           LEFT JOIN ent_inscription_piece_regle r ON r.etablissement_id = p.etablissement_id AND r.id = p.regle_id
           LEFT JOIN ent_doc_piece f ON f.etablissement_id = p.etablissement_id AND f.id = p.piece_doc_id
          WHERE p.etablissement_id = $1 AND p.dossier_id = $2 AND p.applicable
          ORDER BY r.ordre NULLS LAST, p.code",
    )
    .bind(etab)
    .bind(dossier_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut sortie = Vec::with_capacity(rows.len());
    for (_id, code, libelle, consigne, moment, bloquante, portee, statut, fichier_etat, nb_pages, motif_refus, dispense_motif, _) in
        rows
    {
        let decl = type_piece(&code);
        // Réutilisation proposée : une pièce valable de foyer déjà fournie pour un autre enfant de la démarche.
        let proposition = if matches!(statut.as_str(), "attendue" | "refusee")
            && decl.is_some_and(|t| t.portee == PortePiece::Foyer)
        {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT p.dossier_id FROM ent_inscription_piece p
                   JOIN ent_doc_piece f ON f.etablissement_id = p.etablissement_id AND f.id = p.piece_doc_id
                  WHERE p.etablissement_id = $1 AND p.demarche_id = $2 AND p.code = $3 AND p.dossier_id <> $4
                    AND f.etat IN ('recue', 'acceptee') AND p.statut IN ('deposee', 'acceptee', 'reprise')
                  ORDER BY (f.etat = 'acceptee') DESC, p.maj_le DESC NULLS LAST LIMIT 1",
            )
            .bind(etab)
            .bind(d.demarche_id)
            .bind(&code)
            .bind(dossier_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?
        } else {
            None
        };
        sortie.push(PieceAttendueDto {
            conseil: decl.map(|t| t.conseil.to_string()).unwrap_or_default(),
            code,
            libelle,
            consigne,
            moment,
            bloquante,
            portee,
            statut,
            fichier_etat,
            nb_pages,
            motif_refus,
            proposition_dossier_id: proposition,
            dispense_motif,
        });
    }
    Ok(sortie)
}

/// Libellés des pièces **bloquantes** qui manquent encore avant la validation (à produire, en analyse, reçues
/// mais pas contrôlées, refusées).
pub(crate) async fn bloquantes_manquantes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
) -> Result<Vec<String>, EntDbError> {
    synchroniser_dossier_tx(&mut *conn, etab, dossier_id).await?;
    sqlx::query_scalar(&format!(
        "SELECT p.libelle FROM ent_inscription_piece p
          WHERE p.etablissement_id = $1 AND p.dossier_id = $2 AND {SQL_PIECE_BLOQUANTE_MANQUANTE}
          ORDER BY p.code"
    ))
    .bind(etab)
    .bind(dossier_id)
    .fetch_all(conn)
    .await
    .map_err(db)
}

/// Rattache les pièces acceptées à l'élève (validation, §8.6) : une pièce du socle 7 au sujet de l'élève,
/// mêmes objets (aucune copie d'octets), décision reprise. Idempotent ; une validation annulée puis refaite
/// pour un autre élève se rattache à nouveau. Rend le nombre de pièces rattachées.
pub(crate) async fn rattacher_pieces_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
    eleve_id: Uuid,
    actor: Uuid,
) -> Result<usize, EntDbError> {
    let rows: Vec<(Uuid, String, Uuid, Option<Uuid>, Option<Option<Uuid>>)> = sqlx::query_as(
        "SELECT p.id, p.code, p.piece_doc_id, p.rattachee_doc_id,
                (SELECT r.eleve_id FROM ent_doc_piece r WHERE r.etablissement_id = p.etablissement_id AND r.id = p.rattachee_doc_id)
           FROM ent_inscription_piece p
          WHERE p.etablissement_id = $1 AND p.dossier_id = $2 AND p.applicable AND p.statut = 'acceptee'
            AND p.piece_doc_id IS NOT NULL AND p.purgee_le IS NULL",
    )
    .bind(etab)
    .bind(dossier_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut n = 0;
    for (ligne, code, doc, rattachee, eleve_rattache) in rows {
        // Les copies d'identité ne se gardent pas ; un code libre est une « autre » pièce.
        if type_piece(&code).is_some_and(|t| !t.copie_gardee) {
            continue;
        }
        if rattachee.is_some() && eleve_rattache == Some(Some(eleve_id)) {
            continue;
        }
        let Some(nouveau) = rattacher_a_eleve_tx(&mut *conn, etab, doc, eleve_id, actor).await? else {
            continue;
        };
        sqlx::query(
            "UPDATE ent_inscription_piece SET rattachee_doc_id = $3, rattachee_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(ligne)
        .bind(nouveau)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        n += 1;
    }
    Ok(n)
}

/// Texte lisible du statut d'une pièce, pour le suivi.
fn texte_suivi(libelle: &str, action: &str) -> String {
    format!("Pièce « {libelle} » {action}")
}

impl PgEnt {
    // ───────────── Règles ─────────────

    /// Règles de pièces d'une campagne (INS-33).
    pub async fn regles_pieces_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne_id: Uuid,
    ) -> Result<Vec<ReglePieceDto>, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let existe: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(campagne_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        if existe.is_none() {
            return Err(introuvable());
        }
        let r = regles_dto_tx(&mut tx, etab, campagne_id).await?;
        tx.rollback().await.map_err(db)?;
        Ok(r)
    }

    /// Remplace les règles d'une campagne et les **réapplique** aux dossiers ouverts : une pièce devenue
    /// nécessaire est demandée (la famille en est prévenue), une pièce qui ne l'est plus cesse de compter.
    pub async fn definir_regles_pieces_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne_id: Uuid,
        regles: &[ReglePieceDto],
    ) -> Result<Vec<ReglePieceDto>, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        valider_regles(regles)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let statut: Option<(String,)> = sqlx::query_as(
            "SELECT statut FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2 FOR NO KEY UPDATE",
        )
        .bind(etab)
        .bind(campagne_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((statut,)) = statut else {
            return Err(introuvable());
        };
        if statut == "archivee" {
            return Err(conflit("campagne archivée : ses pièces ne se modifient plus"));
        }
        ecrire_regles_tx(&mut tx, etab, campagne_id, regles, false).await?;
        self.reappliquer_regles_tx(&mut tx, etab, campagne_id, Some(actor)).await?;
        let sortie = regles_dto_tx(&mut tx, etab, campagne_id).await?;
        commit(tx).await?;
        Ok(sortie)
    }

    /// Réapplique les règles à tous les dossiers non clos d'une campagne ; prévient les familles des pièces
    /// nouvellement demandées (suivi visible + événement pour le moteur de notifications).
    pub(crate) async fn reappliquer_regles_tx(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        campagne_id: Uuid,
        actor: Option<Uuid>,
    ) -> Result<usize, EntDbError> {
        let dossiers: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT id, etat, etat_avant_blocage FROM ent_inscription_dossier
              WHERE etablissement_id = $1 AND campagne_id = $2",
        )
        .bind(etab)
        .bind(campagne_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
        let mut total = 0;
        for (id, etat, avant) in dossiers {
            let eff = etat_effectif(&etat, avant.as_deref()).to_string();
            if est_clos(&eff) {
                continue;
            }
            let nouvelles = appliquer_regles_tx(tx, etab, id).await?;
            if nouvelles.is_empty() || eff == "brouillon" {
                continue;
            }
            total += nouvelles.len();
            suivi_tx(
                tx,
                etab,
                id,
                "piece",
                None,
                Some(&format!("{} pièce(s) supplémentaire(s) demandée(s)", nouvelles.len())),
                true,
                actor,
            )
            .await?;
            publier(
                tx,
                etab,
                types::INSCRIPTION_PIECE_DEMANDEE,
                AggregateKind::Admission,
                id,
                None,
                json!({ "dossier_id": id, "campagne_id": campagne_id, "codes": nouvelles }),
                actor,
            )
            .await?;
        }
        Ok(total)
    }

    // ───────────── Lecture ─────────────

    /// Pièces d'un dossier, côté personnel (droit d'instruire).
    pub async fn pieces_dossier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
    ) -> Result<Vec<PieceAttendueDto>, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        self.refuser_si_ecarte(etab, actor, dossier_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = pieces_dossier_tx(&mut tx, etab, dossier_id).await?;
        commit(tx).await?;
        Ok(r)
    }

    /// Pièces d'un dossier, côté famille : le déposant vérifié seulement.
    pub async fn pieces_dossier_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        dossier_id: Uuid,
    ) -> Result<Vec<PieceAttendueDto>, EntDbError> {
        let (_, d) = self.dossier_du_deposant(etab, jeton, dossier_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = pieces_dossier_tx(&mut tx, etab, d.id).await?;
        commit(tx).await?;
        Ok(r)
    }

    // ───────────── Dépôt ─────────────

    /// La famille dépose un PDF ou des photos pour une pièce attendue. Le déposant doit avoir l'adresse vérifiée
    /// du dossier (sinon « introuvable »).
    pub async fn deposer_fichiers_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        dossier_id: Uuid,
        code: &str,
        fichiers: Vec<Vec<u8>>,
    ) -> Result<DepotPieceDto, EntDbError> {
        let (v, d) = self.dossier_du_deposant(etab, jeton, dossier_id).await?;
        // Règle 1 : l'état visible seul autorise l'action de la famille ; une décision non publiée ne se révèle pas.
        // Le contrôle se fait **sous le verrou du dossier**, dans la transaction du dépôt (D-069 F12), jamais avant.
        self.deposer_core(etab, d.id, code, fichiers, (v.user_id.unwrap_or_default(), "famille"), None, true)
            .await
    }

    /// Le secrétariat dépose ce qu'une famille a apporté au guichet (scan, photo) : droit d'instruire.
    pub async fn deposer_fichiers_guichet_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
        code: &str,
        fichiers: Vec<Vec<u8>>,
    ) -> Result<DepotPieceDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        self.refuser_si_ecarte(etab, actor, dossier_id).await?;
        self.deposer_core(etab, dossier_id, code, fichiers, (actor, "guichet"), Some((actor, role)), false)
            .await
    }

    async fn deposer_core(
        &self,
        etab: Uuid,
        dossier_id: Uuid,
        code: &str,
        fichiers: Vec<Vec<u8>>,
        deposant: (Uuid, &str),
        agent: Option<(Uuid, &str)>,
        famille: bool,
    ) -> Result<DepotPieceDto, EntDbError> {
        // Contrôle de forme à l'entrée (type réel sur le contenu, taille, nombre) : celui du socle 7.
        let (original, mime) = preparer_depot(&fichiers)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        if famille {
            crate::inscriptions_decisions::exiger_dossier_stable_famille_tx(&mut tx, etab, dossier_id).await?;
        }
        let eff = etat_effectif(&d.etat, d.etat_avant_blocage.as_deref()).to_string();
        if !depot_ouvert(&d.nature, &eff) {
            return Err(conflit("le dépôt de pièces n'est plus ouvert pour ce dossier"));
        }
        synchroniser_dossier_tx(&mut tx, etab, dossier_id).await?;
        let piece: Option<(Uuid, String, String, String, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, libelle, statut, portee, code, piece_doc_id FROM ent_inscription_piece
              WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3 AND applicable FOR UPDATE",
        )
        .bind(etab)
        .bind(dossier_id)
        .bind(code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((piece_id, libelle, statut, portee, _, ancien_doc)) = piece else {
            return Err(introuvable());
        };
        match statut.as_str() {
            "acceptee" => return Err(conflit("cette pièce est déjà acceptée")),
            "dispensee" => return Err(conflit("cette pièce est dispensée : rien à déposer")),
            _ => {}
        }
        let (recents,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_doc_piece
              WHERE etablissement_id = $1 AND dossier_id = $2 AND deposee_le > now() - interval '24 hours'",
        )
        .bind(etab)
        .bind(dossier_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if recents >= DEPOTS_PAR_DOSSIER_ET_JOUR {
            return Err(conflit(
                "trop de dépôts aujourd'hui pour ce dossier : réessayez demain ou rendez-vous au secrétariat",
            ));
        }
        // Type du socle 7 : le type du catalogue, ou « autre » (motif = libellé de la campagne).
        let (type_code, motif_autre) = match type_piece(code) {
            Some(t) if t.demandable => (t.code.to_string(), None),
            _ => ("autre".to_string(), Some(libelle.trim().chars().take(300).collect::<String>())),
        };
        if let Some(m) = &motif_autre {
            if m.chars().count() < 3 || motif_interdit(m) {
                return Err(conflit("cette pièce ne peut être demandée ni conservée par un établissement"));
            }
        }
        if let (Some((u, r)), Some(t)) = (agent, type_piece(&type_code)) {
            let a = Acteur { etablissement_id: etab, user_id: u, role: r };
            if !self.droit_du_type(&a, t, None).await? {
                return Err(EntDbError::Forbidden);
            }
        }
        let id = Uuid::new_v4();
        self.inserer_depot(
            &mut tx,
            etab,
            &DepotInsere {
                id,
                type_code: &type_code,
                sujet: (None, None, None, Some(dossier_id)),
                demande_id: None,
                motif_autre: motif_autre.as_deref(),
                deposant,
                original: &original,
                mime,
                nb_fichiers: fichiers.len(),
            },
        )
        .await?;
        // Pièce de famille : le même fichier vaut pour tous les enfants de la démarche encore en attente.
        let cibles: Vec<(Uuid, Uuid)> = if portee == "demarche" {
            sqlx::query_as(
                "SELECT id, dossier_id FROM ent_inscription_piece
                  WHERE etablissement_id = $1 AND demarche_id = $2 AND code = $3 AND applicable
                    AND statut IN ('attendue', 'refusee', 'deposee', 'reprise') FOR UPDATE",
            )
            .bind(etab)
            .bind(d.demarche_id)
            .bind(code)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
        } else {
            vec![(piece_id, dossier_id)]
        };
        for (ligne, doss) in &cibles {
            sqlx::query(
                "UPDATE ent_inscription_piece
                    SET statut = 'deposee', piece_doc_id = $3, motif_refus = NULL, nom_fichier = NULL,
                        reprise_de_piece_id = CASE WHEN id = $4 THEN NULL ELSE $4 END,
                        maj_par = $5, maj_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(ligne)
            .bind(id)
            .bind(piece_id)
            .bind(agent.map(|a| a.0))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            suivi_tx(
                &mut tx,
                etab,
                *doss,
                "piece",
                None,
                Some(&texte_suivi(&libelle, "déposée")),
                true,
                agent.map(|a| a.0),
            )
            .await?;
        }
        // L'ancien fichier, s'il n'est plus référencé et pas encore contrôlé, est retiré (déposé par erreur).
        if let Some(ancien) = ancien_doc {
            retirer_si_orphelin_tx(&mut tx, etab, ancien).await?;
        }
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_PIECE_DEPOSEE,
            AggregateKind::Admission,
            dossier_id,
            d.eleve_id,
            json!({ "dossier_id": dossier_id, "code": code, "pages": fichiers.len() }),
            agent.map(|a| a.0),
        )
        .await?;
        commit(tx).await?;
        Ok(DepotPieceDto {
            code: code.to_string(),
            statut: "deposee".into(),
            fichier_etat: "en_quarantaine".into(),
        })
    }

    // ───────────── Réutilisation ─────────────

    /// Reprend, **sur confirmation**, la pièce de foyer déjà fournie pour un autre enfant de la démarche
    /// (« déjà fournie pour le frère »). La famille la confirme depuis son espace ; rien n'est repris en silence.
    pub async fn reutiliser_piece_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        dossier_id: Uuid,
        code: &str,
        source_dossier_id: Uuid,
    ) -> Result<Vec<PieceAttendueDto>, EntDbError> {
        let (v, d) = self.dossier_du_deposant(etab, jeton, dossier_id).await?;
        // Contrôle de l'état visible sous le verrou du dossier, dans la transaction de la reprise (D-069 F12).
        self.reutiliser_core(etab, d.id, code, source_dossier_id, v.user_id.unwrap_or_default(), true)
            .await
    }

    /// Même geste au guichet.
    pub async fn reutiliser_piece_dossier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
        code: &str,
        source_dossier_id: Uuid,
    ) -> Result<Vec<PieceAttendueDto>, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        self.refuser_si_ecarte(etab, actor, source_dossier_id).await?;
        self.refuser_si_ecarte(etab, actor, dossier_id).await?;
        self.reutiliser_core(etab, dossier_id, code, source_dossier_id, actor, false).await
    }

    async fn reutiliser_core(
        &self,
        etab: Uuid,
        dossier_id: Uuid,
        code: &str,
        source_dossier_id: Uuid,
        acteur: Uuid,
        famille: bool,
    ) -> Result<Vec<PieceAttendueDto>, EntDbError> {
        let decl = type_piece(code).filter(|t| t.portee == PortePiece::Foyer).ok_or_else(|| {
            conflit("cette pièce est propre à chaque enfant : elle ne se réutilise pas")
        })?;
        if dossier_id == source_dossier_id {
            return Err(invalide("la pièce à reprendre est celle d'un autre enfant"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Deux dossiers : leurs offres, puis les dossiers dans l'ordre des UUID (deux réutilisations croisées ne
        // s'interbloquent pas), en un seul lot.
        crate::inscriptions_attente::verrouiller_dossiers_tx(&mut tx, etab, None, &[dossier_id, source_dossier_id], &[])
            .await?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        let src = dossier_tx(&mut tx, etab, source_dossier_id).await?;
        if famille {
            crate::inscriptions_decisions::exiger_dossier_stable_famille_tx(&mut tx, etab, dossier_id).await?;
        }
        if d.demarche_id != src.demarche_id {
            // Aucun indice sur l'existence du dossier d'une autre démarche.
            return Err(introuvable());
        }
        let eff = etat_effectif(&d.etat, d.etat_avant_blocage.as_deref()).to_string();
        if !depot_ouvert(&d.nature, &eff) {
            return Err(conflit("le dépôt de pièces n'est plus ouvert pour ce dossier"));
        }
        synchroniser_dossier_tx(&mut tx, etab, source_dossier_id).await?;
        let source: Option<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT p.id, p.piece_doc_id, f.etat FROM ent_inscription_piece p
               JOIN ent_doc_piece f ON f.etablissement_id = p.etablissement_id AND f.id = p.piece_doc_id
              WHERE p.etablissement_id = $1 AND p.dossier_id = $2 AND p.code = $3
                AND p.statut IN ('deposee', 'acceptee', 'reprise') AND f.etat IN ('recue', 'acceptee')",
        )
        .bind(etab)
        .bind(source_dossier_id)
        .bind(decl.code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((source_ligne, doc, etat_doc)) = source else {
            return Err(conflit("aucune pièce valable à reprendre pour cet enfant"));
        };
        let cible: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, libelle, statut FROM ent_inscription_piece
              WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3 AND applicable FOR UPDATE",
        )
        .bind(etab)
        .bind(dossier_id)
        .bind(decl.code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((ligne, libelle, statut)) = cible else {
            return Err(introuvable());
        };
        if !matches!(statut.as_str(), "attendue" | "refusee") {
            return Err(conflit("cette pièce est déjà fournie ou traitée"));
        }
        sqlx::query(
            "UPDATE ent_inscription_piece
                SET statut = $3, piece_doc_id = $4, reprise_de_piece_id = $5, motif_refus = NULL, maj_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(ligne)
        .bind(if etat_doc == "acceptee" { "acceptee" } else { "reprise" })
        .bind(doc)
        .bind(source_ligne)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        decision(&mut tx, etab, doc, None, "reutilisee", None, Some("reprise pour un autre enfant de la démarche"), acteur)
            .await?;
        suivi_tx(
            &mut tx,
            etab,
            dossier_id,
            "piece",
            None,
            Some(&texte_suivi(&libelle, "reprise d'un autre dossier de la démarche")),
            true,
            None,
        )
        .await?;
        let r = pieces_dossier_tx(&mut tx, etab, dossier_id).await?;
        commit(tx).await?;
        Ok(r)
    }

    // ───────────── Contrôle ─────────────

    /// Accepter, refuser ou dispenser une pièce. Seul un fichier **reçu** (sain, converti) peut être accepté ;
    /// un type sensible (décision de justice) exige en plus le droit de son domaine.
    pub async fn controler_piece_inscription_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
        code: &str,
        b: &ControlePieceBody,
    ) -> Result<Vec<PieceAttendueDto>, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        self.refuser_si_ecarte(etab, actor, dossier_id).await?;
        let motif = b.motif.as_deref().map(str::trim).filter(|m| !m.is_empty());
        let commentaire = b.commentaire.as_deref().map(str::trim).filter(|m| !m.is_empty());
        if commentaire.is_some_and(|c| c.chars().count() > 500) {
            return Err(invalide("commentaire trop long (500 caractères)"));
        }
        match b.decision.as_str() {
            "accepter" => {}
            "refuser" => {
                if !motif.is_some_and(|m| MOTIFS_REFUS.contains(&m)) {
                    return Err(invalide(
                        "motif du refus : illisible, incomplet, perime, ne_correspond_pas, mauvais_document ou autre",
                    ));
                }
                if motif == Some("autre") && commentaire.is_none() {
                    return Err(invalide("un refus « autre motif » s'explique en commentaire"));
                }
            }
            "dispenser" => {
                if !motif.is_some_and(|m| (3..=500).contains(&m.chars().count())) {
                    return Err(invalide("motif de la dispense obligatoire (3 à 500 caractères)"));
                }
            }
            _ => return Err(invalide("décision : accepter, refuser ou dispenser")),
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        let eff = etat_effectif(&d.etat, d.etat_avant_blocage.as_deref()).to_string();
        if est_clos(&eff) {
            return Err(conflit("dossier clos"));
        }
        synchroniser_dossier_tx(&mut tx, etab, dossier_id).await?;
        let piece: Option<(Uuid, String, String, bool, Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT p.id, p.libelle, p.statut, p.bloquante, p.piece_doc_id, f.etat
               FROM ent_inscription_piece p
               LEFT JOIN ent_doc_piece f ON f.etablissement_id = p.etablissement_id AND f.id = p.piece_doc_id
              WHERE p.etablissement_id = $1 AND p.dossier_id = $2 AND p.code = $3 AND p.applicable FOR UPDATE OF p",
        )
        .bind(etab)
        .bind(dossier_id)
        .bind(code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((ligne, libelle, statut, bloquante, doc, etat_doc)) = piece else {
            return Err(introuvable());
        };
        let a = Acteur { etablissement_id: etab, user_id: actor, role };
        let type_decl = type_piece(code);
        if let Some(t) = type_decl {
            if !self.droit_du_type(&a, t, None).await? {
                return Err(EntDbError::Forbidden);
            }
        }
        let jour = jour_etab(&mut tx, etab).await?;
        let (nouveau_statut, texte): (&str, String) = match b.decision.as_str() {
            "accepter" => {
                if statut == "acceptee" {
                    return Err(conflit("cette pièce est déjà acceptée"));
                }
                let Some(doc) = doc else {
                    return Err(conflit(
                        "aucun fichier à accepter : la pièce n'a pas été déposée (une pièce se dépose avant d'être contrôlée)",
                    ));
                };
                match etat_doc.as_deref() {
                    Some("recue") => {}
                    Some("acceptee") => {}
                    Some("en_quarantaine") => {
                        return Err(conflit("le fichier est en cours d'analyse : réessayez dans un instant"))
                    }
                    _ => return Err(conflit("le fichier a été refusé à l'analyse : demandez un nouveau dépôt")),
                }
                let fin = match type_decl {
                    Some(t) => fin_de_validite(t, jour, b.valable_jusqu_au)?,
                    None => b.valable_jusqu_au,
                };
                if etat_doc.as_deref() == Some("recue")
                    && !accepter_tx(&mut tx, etab, doc, fin, actor, commentaire).await?
                {
                    return Err(conflit("la pièce vient d'être traitée par un autre agent"));
                }
                // Le même fichier sert plusieurs lignes (pièce de famille, réutilisation) : toutes suivent.
                sqlx::query(
                    "UPDATE ent_inscription_piece SET statut = 'acceptee', motif_refus = NULL, maj_par = $3, maj_le = now()
                      WHERE etablissement_id = $1 AND piece_doc_id = $2 AND applicable AND statut IN ('deposee', 'reprise')",
                )
                .bind(etab)
                .bind(doc)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                ("acceptee", texte_suivi(&libelle, "acceptée"))
            }
            "refuser" => {
                let Some(doc) = doc.filter(|_| matches!(statut.as_str(), "deposee" | "reprise")) else {
                    return Err(conflit("aucun fichier déposé à refuser"));
                };
                let motif = motif.unwrap_or("autre");
                if etat_doc.as_deref() == Some("recue") {
                    refuser_tx(&mut tx, etab, doc, motif, commentaire, actor).await?;
                }
                let affiche = match commentaire {
                    Some(c) if motif == "autre" => c.to_string(),
                    Some(c) => format!("{} — {c}", motif_refus_libelle(motif)),
                    None => motif_refus_libelle(motif).to_string(),
                };
                sqlx::query(
                    "UPDATE ent_inscription_piece SET statut = 'refusee', motif_refus = left($3, 500), maj_par = $4, maj_le = now()
                      WHERE etablissement_id = $1 AND piece_doc_id = $2 AND applicable AND statut IN ('deposee', 'reprise')",
                )
                .bind(etab)
                .bind(doc)
                .bind(&affiche)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                ("refusee", format!("Pièce « {libelle} » refusée : {affiche}"))
            }
            _ => {
                // Dispenser : une pièce bloquante ne se dispense que par la direction (exception motivée).
                if bloquante {
                    self.ins_exiger(etab, actor, role, Action::InscriptionsDecider).await?;
                }
                if statut == "dispensee" {
                    return Err(conflit("cette pièce est déjà dispensée"));
                }
                sqlx::query(
                    "UPDATE ent_inscription_piece SET statut = 'dispensee', dispense_motif = $3, dispense_par = $4,
                            dispense_le = now(), maj_par = $4, maj_le = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(ligne)
                .bind(motif)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                ("dispensee", texte_suivi(&libelle, "dispensée"))
            }
        };
        suivi_tx(&mut tx, etab, dossier_id, "piece", None, Some(&texte), b.decision != "dispenser", Some(actor)).await?;
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_PIECE_CONTROLEE,
            AggregateKind::Admission,
            dossier_id,
            d.eleve_id,
            json!({ "dossier_id": dossier_id, "code": code, "statut": nouveau_statut }),
            Some(actor),
        )
        .await?;
        let r = pieces_dossier_tx(&mut tx, etab, dossier_id).await?;
        commit(tx).await?;
        Ok(r)
    }

    /// Lit le fichier **servi** d'une pièce (le PDF désarmé, jamais l'original), pour le personnel : droit
    /// d'instruire, et le droit du domaine pour un type sensible. Chaque lecture laisse une ligne au journal.
    pub async fn lire_piece_inscription_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
        code: &str,
    ) -> Result<PieceLue, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        self.refuser_si_ecarte(etab, actor, dossier_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        dossier_tx(&mut tx, etab, dossier_id).await?;
        synchroniser_dossier_tx(&mut tx, etab, dossier_id).await?;
        let ligne: Option<(Uuid, String, String, i32, String, String, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
            "SELECT f.id, f.type_code, f.etat, f.nb_pages, f.sha256_servi, f.cle_objet_servi, o.supprime_le
               FROM ent_inscription_piece p
               JOIN ent_doc_piece f ON f.etablissement_id = p.etablissement_id AND f.id = p.piece_doc_id
               JOIN ent_objet o ON o.etablissement_id = f.etablissement_id AND o.cle_objet = f.cle_objet_servi
              WHERE p.etablissement_id = $1 AND p.dossier_id = $2 AND p.code = $3
                AND f.etat IN ('recue', 'acceptee', 'refusee') AND f.cle_objet_servi IS NOT NULL",
        )
        .bind(etab)
        .bind(dossier_id)
        .bind(code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((id, type_code, etat, pages, sha, cle, supprime)) = ligne else {
            return Err(introuvable());
        };
        if supprime.is_some() {
            return Err(conflit("le fichier de cette pièce a été détruit (dossier non abouti)"));
        }
        let a = Acteur { etablissement_id: etab, user_id: actor, role };
        if let Some(t) = type_piece(&type_code) {
            if !self.droit_du_type(&a, t, None).await? {
                return Err(EntDbError::Forbidden);
            }
        }
        journaliser(&mut tx, &a, None, None, "piece_inscription_lue", (None, None), Some(&id.to_string()), None)
            .await?;
        commit(tx).await?;
        let octets = self.lire_objet_empreinte(etab, &cle, Some(&sha)).await?;
        Ok(PieceLue {
            piece_id: id,
            type_code,
            etat,
            nb_pages: pages,
            empreinte_sha256: sha,
            octets,
        })
    }

    // ───────────── Purge ─────────────

    /// Une passe de la purge des pièces de dossiers non aboutis, pour tous les établissements (tâche de fond
    /// horaire). Un établissement en erreur ne bloque pas les suivants.
    pub async fn purger_pieces_non_aboutis_tache(
        &self,
        delai_jours: i64,
    ) -> Result<BilanPurgeInscriptionDto, EntDbError> {
        let etabs: Vec<(Uuid,)> = crate::tenant_tx::lire_par_etablissement(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("pieces_inscription_purge"),
            "SELECT DISTINCT p.etablissement_id FROM ent_inscription_piece p
               JOIN ent_inscription_dossier d ON d.etablissement_id = p.etablissement_id AND d.id = p.dossier_id
              WHERE p.purgee_le IS NULL AND p.piece_doc_id IS NOT NULL
                AND d.etat IN ('refuse', 'annule', 'sans_suite', 'retire', 'desiste', 'non_reinscription_famille',
                               'non_reinscription_etablissement', 'sans_reponse', 'non_presente', 'reaffecte')",
        )
        .await
        .map_err(db)?;
        let mut total = BilanPurgeInscriptionDto::default();
        for (etab,) in etabs {
            let b = self.purger_pieces_non_aboutis(etab, delai_jours).await?;
            total.dossiers += b.dossiers;
            total.fichiers_detruits += b.fichiers_detruits;
            total.echecs += b.echecs;
        }
        Ok(total)
    }

    /// **Purge** des fichiers des dossiers non aboutis (refusés, annulés, sans suite, retirés, désistés, non
    /// réinscrits) clos depuis au moins `delai_jours` : l'objet est détruit au magasin **avant** d'être marqué
    /// supprimé au registre (une panne se rattrape à la passe suivante), la ligne de la pièce et son historique
    /// restent (preuve). Un objet encore référencé par une autre pièce (rattachée à un élève) n'est jamais détruit.
    pub async fn purger_pieces_non_aboutis(
        &self,
        etab: Uuid,
        delai_jours: i64,
    ) -> Result<BilanPurgeInscriptionDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a_purger: Vec<(Uuid, Uuid, Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT p.dossier_id, p.id, f.id, f.cle_objet_original, f.cle_objet_servi
               FROM ent_inscription_piece p
               JOIN ent_inscription_dossier d ON d.etablissement_id = p.etablissement_id AND d.id = p.dossier_id
               JOIN ent_doc_piece f ON f.etablissement_id = p.etablissement_id AND f.id = p.piece_doc_id
              WHERE p.etablissement_id = $1 AND p.purgee_le IS NULL
                AND d.etat = ANY($2) AND COALESCE(d.clos_le, d.updated_at) <= now() - make_interval(days => $3::int)
              ORDER BY p.dossier_id, p.id LIMIT 500",
        )
        .bind(etab)
        .bind(ETATS_NON_ABOUTIS.iter().map(|s| (*s).to_string()).collect::<Vec<_>>())
        .bind(delai_jours.clamp(0, 3650) as i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let sys = systeme(etab);
        let mut bilan = BilanPurgeInscriptionDto::default();
        let mut dossiers: HashSet<Uuid> = HashSet::new();
        for (dossier, ligne, doc, original, servi) in a_purger {
            let mut cles = vec![original];
            cles.extend(servi);
            let (detruits, ok) = self.detruire_objets_orphelins(etab, doc, &cles).await?;
            bilan.fichiers_detruits += detruits;
            if !ok {
                bilan.echecs += 1;
            }
            if !ok {
                continue;
            }
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query(
                "UPDATE ent_inscription_piece SET purgee_le = now() WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(ligne)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if dossiers.insert(dossier) {
                journaliser(
                    &mut tx,
                    &sys,
                    None,
                    None,
                    "pieces_inscription_purgees",
                    (None, None),
                    Some(&dossier.to_string()),
                    Some("fichiers détruits : dossier non abouti"),
                )
                .await?;
                publier(
                    &mut tx,
                    etab,
                    types::INSCRIPTION_PIECES_PURGEES,
                    AggregateKind::Admission,
                    dossier,
                    None,
                    json!({ "dossier_id": dossier }),
                    None,
                )
                .await?;
            }
            tx.commit().await.map_err(db)?;
        }
        bilan.dossiers = dossiers.len();
        Ok(bilan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regle(code: &str) -> ReglePieceDto {
        ReglePieceDto {
            code_type: code.into(),
            libelle: "Libellé".into(),
            consigne: None,
            moment: "depot".into(),
            bloquante: true,
            portee: "dossier".into(),
            conditions: vec![],
        }
    }

    #[test]
    fn regles_valides_et_refus() {
        assert!(valider_regles(&[regle("livret_famille"), regle("certificat_special")]).is_ok());
        // Code mal formé, en double, moment ou portée inconnus.
        assert!(valider_regles(&[regle("Livret")]).is_err());
        assert!(valider_regles(&[regle("exeat"), regle("exeat")]).is_err());
        let mut r = regle("exeat");
        r.moment = "jamais".into();
        assert!(valider_regles(&[r]).is_err());
        // Une pièce d'identité n'est pas demandée par une campagne (jamais conservée).
        assert!(valider_regles(&[regle("piece_identite")]).is_err());
        // Ce qu'aucun établissement ne peut demander, même sous un code libre.
        let mut r = regle("papier_special");
        r.libelle = "Copie du titre de séjour".into();
        assert!(valider_regles(&[r]).is_err());
        // Pièce de famille : un type de foyer du catalogue seulement.
        let mut r = regle("acte_naissance");
        r.portee = "demarche".into();
        assert!(valider_regles(&[r]).is_err());
        let mut r = regle("livret_famille");
        r.portee = "demarche".into();
        assert!(valider_regles(&[r]).is_ok());
    }

    #[test]
    fn conditions_validees() {
        let mut r = regle("decision_justice");
        r.conditions = vec![ConditionPieceDto { cle: "foyers_distincts".into(), valeur: "true".into() }];
        assert!(valider_regles(std::slice::from_ref(&r)).is_ok());
        r.conditions = vec![ConditionPieceDto { cle: "foyers_distincts".into(), valeur: "oui".into() }];
        assert!(valider_regles(std::slice::from_ref(&r)).is_err());
        r.conditions = vec![ConditionPieceDto { cle: "au_hasard".into(), valeur: "true".into() }];
        assert!(valider_regles(std::slice::from_ref(&r)).is_err());
        r.conditions = vec![
            ConditionPieceDto { cle: "niveau".into(), valeur: "6EME".into() },
            ConditionPieceDto { cle: "niveau".into(), valeur: "5EME".into() },
        ];
        assert!(valider_regles(std::slice::from_ref(&r)).is_ok());
    }

    fn faits() -> Faits {
        Faits {
            nature: "admission".into(),
            niveau: Some("6EME".into()),
            regime: Some("externe".into()),
            autre_etablissement: true,
            foyers_distincts: false,
            autorite_exclusive: false,
            responsable_tuteur: false,
        }
    }

    #[test]
    fn conditions_evaluees_et_ou() {
        let f = faits();
        let c = |cle: &str, b: Option<bool>, t: Option<&str>| (cle.to_string(), b, t.map(str::to_string));
        assert!(f.satisfait(&[]));
        assert!(f.satisfait(&[c("autre_etablissement", Some(true), None)]));
        assert!(!f.satisfait(&[c("foyers_distincts", Some(true), None)]));
        assert!(f.satisfait(&[c("foyers_distincts", Some(false), None)]));
        // OU entre les valeurs d'une même clé, ET entre les clés.
        assert!(f.satisfait(&[c("niveau", None, Some("5eme")), c("niveau", None, Some("6eme"))]));
        assert!(!f.satisfait(&[c("niveau", None, Some("5eme"))]));
        assert!(!f.satisfait(&[c("niveau", None, Some("6eme")), c("regime", None, Some("interne"))]));
        assert!(f.satisfait(&[c("niveau", None, Some("6eme")), c("regime", None, Some("EXTERNE"))]));
    }

    #[test]
    fn depot_ouvert_selon_la_nature() {
        assert!(depot_ouvert("admission", "deposee"));
        assert!(depot_ouvert("admission", "attente_famille"));
        assert!(!depot_ouvert("admission", "brouillon"));
        assert!(!depot_ouvert("admission", "validee"));
        assert!(!depot_ouvert("admission", "refuse"));
        assert!(depot_ouvert("reinscription", "a_confirmer"));
        assert!(!depot_ouvert("reinscription", "non_reinscription_famille"));
        assert!(depot_ouvert("affectes", "a_completer"));
    }
}
