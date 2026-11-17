//! **File « à décider » de la reprise** (socle 28, lot R3) : ce que voient et font le secrétariat et la direction.
//!
//! * **droits côté serveur** : toute méthode exige `reprise.decider` (direction, secrétariat) ; rien n'est lisible
//!   ni décidable sans lui ;
//! * **minimisé** : la file ne montre que ce qui sert à distinguer deux personnes (nom, prénom, naissance, classe, fin
//!   d'INE, adresse masquée) ; aucune donnée de santé, aucun contact complet, aucune adresse postale ;
//! * **tracée** : une décision = qui, quand, quoi, quelles raisons étaient affichées (`rep_decision`, ajout seul) ;
//! * **réversible tant que la reprise n'est pas appliquée** : l'annulation remet la ligne « à décider » ;
//! * **jamais de fusion sur une ressemblance** : une décision en lot ne vaut que pour les propositions fortes sans
//!   contradiction, chacune tracée à l'unité ; créer une fiche ou ignorer exige une décision (et un motif quand une
//!   fiche est probable ou pour ignorer).
//!
//! Aucune écriture dans les tables métier : une décision pose une **correspondance** (`rep_correspondance`) que
//! l'application de la reprise (lot suivant, deux personnes) exécutera par les commandes des socles propriétaires.

use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_reprise::correspondance::{id_derive, SOURCE_NEXT};
use ent_reprise::rapprochement::{
    cle_synonyme, eligible_lot, existante_de_decision, valider_decision, ActionDecision, Candidat,
    Confiance, DecisionDemandee, PropositionFile, Raison, RefusDecision, Resume,
};
use serde::Serialize;
use sqlx::{PgConnection, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Lignes d'un lot au plus.
pub const LOT_MAX: usize = 500;
/// Lignes d'une page au plus.
pub const PAGE_MAX: i64 = 200;
const MOTIF_LOT: &str = "décision en lot : confiance forte, sans contradiction";

#[derive(Debug, Clone, Serialize)]
pub struct RaisonVue {
    pub code: String,
    pub polarite: String,
    pub explication: String,
}

/// Ce qui est montré d'une fiche : le strict nécessaire pour décider.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct FicheVue {
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub naissance: Option<NaiveDate>,
    pub classe: Option<String>,
    pub ine_fin: Option<String>,
    pub contact_masque: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidatVue {
    pub rang: i16,
    pub cible_id: Uuid,
    pub confiance: String,
    pub raisons: Vec<RaisonVue>,
    pub fiche: FicheVue,
    /// Peut figurer dans une décision en lot (confiance forte, aucune contradiction).
    pub lot_possible: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionVue {
    pub id: Uuid,
    pub decision: String,
    pub cible_id: Option<Uuid>,
    pub par_user_id: Uuid,
    pub le: DateTime<Utc>,
    pub motif: Option<String>,
    pub en_lot: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LigneFileVue {
    /// Identifiant opaque de la proposition : c'est lui qu'on passe pour décider.
    pub id: Uuid,
    pub objet: String,
    pub source_ref: String,
    pub motif: String,
    pub meilleure_confiance: Option<String>,
    pub source: FicheVue,
    pub candidats: Vec<CandidatVue>,
    pub decision: Option<DecisionVue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageFile {
    pub execution_numero: Option<i32>,
    pub total: i64,
    pub lignes: Vec<LigneFileVue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatFile {
    ADecider,
    Decidee,
}

#[derive(Debug, Clone, Default)]
pub struct FiltreFile {
    pub objet: Option<String>,
    pub etat: Option<EtatFile>,
    /// 0 = valeur par défaut (50).
    pub limite: i64,
    pub decalage: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LigneResume {
    pub objet: String,
    pub decidee: bool,
    pub meilleure_confiance: String,
    pub nombre: i64,
}

#[derive(Debug, Clone)]
pub struct DemandeDecision {
    pub proposition_id: Uuid,
    pub action: ActionDecision,
    pub cible_id: Option<Uuid>,
    pub motif: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeLot {
    /// Rattacher chaque ligne à son meilleur candidat, si confiance forte et aucune contradiction.
    RattacherMeilleurCandidat,
    /// Créer la fiche des lignes qui n'ont aucun candidat.
    CreerSansCandidat,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RapportLot {
    pub decidees: u32,
    /// Lignes écartées du lot, avec la raison (code) : elles restent « à décider ».
    pub ecartees: Vec<(Uuid, String)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SynonymeVue {
    pub id: Uuid,
    pub domaine: String,
    pub valeur_cle: String,
    pub cible_id: Uuid,
    pub propose_par: String,
    pub valide_par: Option<Uuid>,
    pub valide_le: Option<DateTime<Utc>>,
}

// ───────────────────────── Interne ─────────────────────────

enum Echec {
    Refus(RefusDecision),
    DejaTraitee,
    Perimee,
    Appliquee,
    Introuvable,
    Sql(sqlx::Error),
}

impl From<sqlx::Error> for Echec {
    fn from(e: sqlx::Error) -> Self {
        Echec::Sql(e)
    }
}

impl From<Echec> for EntDbError {
    fn from(e: Echec) -> Self {
        match e {
            Echec::Refus(r) => EntDbError::SaisieInvalide(r.message()),
            Echec::DejaTraitee => EntDbError::ConflitDossier("cette ligne n'est plus à décider"),
            Echec::Perimee => EntDbError::ConflitDossier(
                "cette ligne date d'une exécution plus ancienne : relancer la simulation avant de décider",
            ),
            Echec::Appliquee => EntDbError::ConflitDossier(
                "la reprise est appliquée : les décisions ne se modifient plus",
            ),
            Echec::Introuvable => EntDbError::SaisieInvalide("ligne inconnue"),
            Echec::Sql(e) => EntDbError::Sqlx(e),
        }
    }
}

struct PropositionChargee {
    execution_id: Uuid,
    objet: String,
    source_id: String,
    cible_objet: String,
    src_nom: Option<String>,
    candidats: Vec<Candidat>,
}

async fn reprise_appliquee(tx: &mut PgConnection, etab: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM rep_execution WHERE etablissement_id = $1 AND mode = 'application' AND etat = 'terminee')",
    )
    .bind(etab)
    .fetch_one(&mut *tx)
    .await
}

/// Dernière exécution terminée : c'est sa file qui est décidée.
async fn derniere_execution(
    tx: &mut PgConnection,
    etab: Uuid,
) -> Result<Option<(Uuid, i32)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, numero FROM rep_execution WHERE etablissement_id = $1 AND etat = 'terminee' ORDER BY numero DESC LIMIT 1",
    )
    .bind(etab)
    .fetch_optional(&mut *tx)
    .await
}

fn fiche_de(r: &sqlx::postgres::PgRow, prefixe: &str) -> FicheVue {
    let c = |n: &str| format!("{prefixe}{n}");
    FicheVue {
        nom: r.get(c("nom").as_str()),
        prenom: r.get(c("prenom").as_str()),
        naissance: r.get(c("naissance").as_str()),
        classe: r.get(c("classe").as_str()),
        ine_fin: r.get(c("ine_fin").as_str()),
        contact_masque: r.get(c("contact").as_str()),
        detail: r.get(c("detail").as_str()),
    }
}

/// Candidats (avec raisons) de plusieurs propositions, groupés par proposition.
async fn charger_candidats(
    tx: &mut PgConnection,
    etab: Uuid,
    propositions: &[Uuid],
) -> Result<BTreeMap<Uuid, Vec<(i16, Uuid, Candidat)>>, sqlx::Error> {
    let mut raisons: BTreeMap<(Uuid, i16), Vec<Raison>> = BTreeMap::new();
    for l in sqlx::query(
        "SELECT proposition_id, rang, code FROM rep_proposition_raison
          WHERE etablissement_id = $1 AND proposition_id = ANY($2)",
    )
    .bind(etab)
    .bind(propositions)
    .fetch_all(&mut *tx)
    .await?
    {
        let code: String = l.get("code");
        if let Some(r) = Raison::depuis(&code) {
            raisons
                .entry((l.get("proposition_id"), l.get("rang")))
                .or_default()
                .push(r);
        }
    }
    let mut out: BTreeMap<Uuid, Vec<(i16, Uuid, Candidat)>> = BTreeMap::new();
    for l in sqlx::query(
        "SELECT proposition_id, rang, cible_id, confiance, cand_nom, cand_prenom, cand_naissance, cand_classe,
                cand_ine_fin, cand_contact, cand_detail
           FROM rep_proposition_candidat
          WHERE etablissement_id = $1 AND proposition_id = ANY($2)
          ORDER BY proposition_id, rang",
    )
    .bind(etab)
    .bind(propositions)
    .fetch_all(&mut *tx)
    .await?
    {
        let (pid, rang): (Uuid, i16) = (l.get("proposition_id"), l.get("rang"));
        let cible: Uuid = l.get("cible_id");
        let conf: String = l.get("confiance");
        let naissance: Option<NaiveDate> = l.get("cand_naissance");
        let mut rs = raisons.remove(&(pid, rang)).unwrap_or_default();
        rs.sort();
        out.entry(pid).or_default().push((
            rang,
            cible,
            Candidat {
                cible_id: cible.to_string(),
                confiance: Confiance::depuis(&conf).unwrap_or(Confiance::Faible),
                raisons: rs,
                resume: Resume {
                    nom: l.get("cand_nom"),
                    prenom: l.get("cand_prenom"),
                    naissance: naissance.map(|d| d.to_string()),
                    classe: l.get("cand_classe"),
                    ine_fin: l.get("cand_ine_fin"),
                    contact_masque: l.get("cand_contact"),
                    detail: l.get("cand_detail"),
                },
            },
        ));
    }
    Ok(out)
}

async fn charger_proposition(
    tx: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<Option<PropositionChargee>, sqlx::Error> {
    let Some(l) = sqlx::query(
        "SELECT id, execution_id, objet, source_id, cible_objet, src_nom FROM rep_proposition
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(None);
    };
    let mut cands = charger_candidats(tx, etab, &[id]).await?;
    Ok(Some(PropositionChargee {
        execution_id: l.get("execution_id"),
        objet: l.get("objet"),
        source_id: l.get("source_id"),
        cible_objet: l.get("cible_objet"),
        src_nom: l.get("src_nom"),
        candidats: cands
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, _, c)| c)
            .collect(),
    }))
}

fn id_correspondance(etab: Uuid, objet: &str, source_id: &str) -> Uuid {
    Uuid::parse_str(&id_derive(
        &etab.to_string(),
        "rep_correspondance",
        objet,
        source_id,
    ))
    .expect("uuid dérivé")
}

/// Pose une décision pour une proposition déjà chargée. Une seule transaction (celle de l'appelant).
async fn decider_interne(
    tx: &mut PgConnection,
    etab: Uuid,
    acteur: Uuid,
    p: &PropositionChargee,
    dem: &DecisionDemandee,
    en_lot: bool,
) -> Result<Uuid, Echec> {
    let derniere = derniere_execution(tx, etab).await?;
    if derniere.map(|d| d.0) != Some(p.execution_id) {
        return Err(Echec::Perimee);
    }
    valider_decision(
        dem,
        &PropositionFile {
            objet: p.objet.clone(),
            source_id: p.source_id.clone(),
            candidats: p.candidats.clone(),
        },
    )
    .map_err(Echec::Refus)?;
    let existante = existante_de_decision(dem, &etab.to_string());
    let cible = Uuid::parse_str(&existante.cible_id).map_err(|_| Echec::Introuvable)?;
    let corr = id_correspondance(etab, &p.objet, &p.source_id);
    let r = sqlx::query(
        "INSERT INTO rep_correspondance
             (id, etablissement_id, source, source_objet, source_id, cible_objet, cible_id, regle_code, statut, execution_id, empreinte_source)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL)
         ON CONFLICT (etablissement_id, source, source_objet, source_id) DO UPDATE
            SET cible_id = EXCLUDED.cible_id, regle_code = EXCLUDED.regle_code, statut = EXCLUDED.statut,
                execution_id = EXCLUDED.execution_id, empreinte_source = NULL, updated_at = now()
          WHERE rep_correspondance.statut = 'a_decider'",
    )
    .bind(corr)
    .bind(etab)
    .bind(SOURCE_NEXT)
    .bind(&p.objet)
    .bind(&p.source_id)
    .bind(&p.cible_objet)
    .bind(cible)
    .bind(dem.action.regle())
    .bind(existante.statut.code())
    .bind(p.execution_id)
    .execute(&mut *tx)
    .await?;
    if r.rows_affected() == 0 {
        return Err(Echec::DejaTraitee);
    }
    let choisi = dem
        .cible_id
        .as_ref()
        .and_then(|id| p.candidats.iter().find(|c| &c.cible_id == id));
    let decision_id = Uuid::new_v4();
    let motif = dem
        .motif
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty());
    sqlx::query(
        "INSERT INTO rep_decision
             (id, etablissement_id, correspondance_id, execution_id, decision, cible_id, par_user_id, motif, confiance_vue, en_lot)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(decision_id)
    .bind(etab)
    .bind(corr)
    .bind(p.execution_id)
    .bind(dem.action.code())
    .bind(dem.cible_id.as_deref().and_then(|c| Uuid::parse_str(c).ok()))
    .bind(acteur)
    .bind(motif)
    .bind(
        choisi
            .or_else(|| p.candidats.first())
            .map(|c| c.confiance.code()),
    )
    .bind(en_lot)
    .execute(&mut *tx)
    .await?;
    // Ce que la personne avait sous les yeux : les raisons du candidat choisi (ou du meilleur).
    if let Some(c) = choisi.or_else(|| p.candidats.first()) {
        for raison in &c.raisons {
            sqlx::query(
                "INSERT INTO rep_decision_raison (etablissement_id, decision_id, code, polarite) VALUES ($1, $2, $3, $4)",
            )
            .bind(etab)
            .bind(decision_id)
            .bind(raison.code())
            .bind(raison.polarite().code())
            .execute(&mut *tx)
            .await?;
        }
    }
    // Rattacher une classe ou un site mémorise l'équivalence de libellé, relue une fois (T0 §6.3).
    if dem.action == ActionDecision::Rattacher {
        if let Some((domaine, cle)) = p.src_nom.as_deref().and_then(|n| cle_synonyme(&p.objet, n)) {
            sqlx::query(
                "INSERT INTO rep_synonyme
                     (id, etablissement_id, source, domaine, valeur_cle, valeur_cible_id, propose_par, valide_par, valide_le, decision_id)
                 VALUES ($1, $2, $3, $4, $5, $6, 'personne', $7, now(), $8)
                 ON CONFLICT (etablissement_id, source, domaine, valeur_cle) DO UPDATE
                    SET valeur_cible_id = EXCLUDED.valeur_cible_id, propose_par = 'personne', valide_par = EXCLUDED.valide_par,
                        valide_le = now(), decision_id = EXCLUDED.decision_id, annule_le = NULL",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(SOURCE_NEXT)
            .bind(domaine)
            .bind(cle)
            .bind(cible)
            .bind(acteur)
            .bind(decision_id)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(decision_id)
}

fn demande_de(p: &PropositionChargee, d: &DemandeDecision) -> DecisionDemandee {
    DecisionDemandee {
        objet: p.objet.clone(),
        source_id: p.source_id.clone(),
        action: d.action,
        cible_id: d.cible_id.map(|c| c.to_string()),
        motif: d.motif.clone(),
    }
}

// ───────────────────────── Méthodes (droit `reprise.decider`) ─────────────────────────

impl PgEnt {
    /// Page de la file « à décider » de la dernière exécution terminée : fiche de l'export, candidats ScolIA avec
    /// leurs raisons et leur confiance, décision éventuelle. Droit `reprise.decider`.
    pub async fn reprise_file_lister(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        filtre: FiltreFile,
    ) -> Result<PageFile, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::RepriseDecider)
            .await?;
        let limite = if filtre.limite <= 0 {
            50
        } else {
            filtre.limite.min(PAGE_MAX)
        };
        let decalage = filtre.decalage.max(0);
        let etat = filtre.etat.map(|e| match e {
            EtatFile::ADecider => "a_decider",
            EtatFile::Decidee => "decidee",
        });
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let Some((execution, numero)) = derniere_execution(&mut tx, etab).await? else {
            tx.rollback().await?;
            return Ok(PageFile {
                execution_numero: None,
                total: 0,
                lignes: vec![],
            });
        };
        let total: i64 = sqlx::query_scalar(
            "SELECT count(*)
               FROM rep_proposition p
               LEFT JOIN rep_correspondance c ON c.etablissement_id = p.etablissement_id AND c.source = 'next'
                    AND c.source_objet = p.objet AND c.source_id = p.source_id
               LEFT JOIN rep_decision d ON d.etablissement_id = c.etablissement_id AND d.correspondance_id = c.id
                    AND d.annulee_le IS NULL
              WHERE p.etablissement_id = $1 AND p.execution_id = $2
                AND ($3::text IS NULL OR p.objet = $3)
                AND ($4::text IS NULL OR (($4 = 'a_decider') = (d.id IS NULL)))",
        )
        .bind(etab)
        .bind(execution)
        .bind(&filtre.objet)
        .bind(etat)
        .fetch_one(&mut *tx)
        .await?;
        let rows = sqlx::query(
            "SELECT p.id, p.objet, p.source_ref, p.motif, p.meilleure_confiance,
                    p.src_nom, p.src_prenom, p.src_naissance, p.src_classe, p.src_ine_fin, p.src_contact, p.src_detail,
                    d.id AS dec_id, d.decision AS dec_decision, d.cible_id AS dec_cible, d.par_user_id AS dec_par,
                    d.le AS dec_le, d.motif AS dec_motif, d.en_lot AS dec_lot
               FROM rep_proposition p
               LEFT JOIN rep_correspondance c ON c.etablissement_id = p.etablissement_id AND c.source = 'next'
                    AND c.source_objet = p.objet AND c.source_id = p.source_id
               LEFT JOIN rep_decision d ON d.etablissement_id = c.etablissement_id AND d.correspondance_id = c.id
                    AND d.annulee_le IS NULL
              WHERE p.etablissement_id = $1 AND p.execution_id = $2
                AND ($3::text IS NULL OR p.objet = $3)
                AND ($4::text IS NULL OR (($4 = 'a_decider') = (d.id IS NULL)))
              ORDER BY p.objet, p.source_ref, p.id
              LIMIT $5 OFFSET $6",
        )
        .bind(etab)
        .bind(execution)
        .bind(&filtre.objet)
        .bind(etat)
        .bind(limite)
        .bind(decalage)
        .fetch_all(&mut *tx)
        .await?;
        let ids: Vec<Uuid> = rows.iter().map(|r| r.get("id")).collect();
        let mut candidats = charger_candidats(&mut tx, etab, &ids).await?;
        tx.rollback().await?;
        let lignes =
            rows.iter()
                .map(|r| {
                    let id: Uuid = r.get("id");
                    let dec: Option<Uuid> = r.get("dec_id");
                    LigneFileVue {
                        id,
                        objet: r.get("objet"),
                        source_ref: r.get("source_ref"),
                        motif: r.get("motif"),
                        meilleure_confiance: r.get("meilleure_confiance"),
                        source: fiche_de(r, "src_"),
                        candidats: candidats
                            .remove(&id)
                            .unwrap_or_default()
                            .into_iter()
                            .map(|(rang, cible_id, c)| CandidatVue {
                                rang,
                                cible_id,
                                confiance: c.confiance.code().to_string(),
                                lot_possible: eligible_lot(&c),
                                raisons: c
                                    .raisons
                                    .iter()
                                    .map(|x| RaisonVue {
                                        code: x.code().to_string(),
                                        polarite: x.polarite().code().to_string(),
                                        explication: x.explication().to_string(),
                                    })
                                    .collect(),
                                fiche: FicheVue {
                                    nom: c.resume.nom,
                                    prenom: c.resume.prenom,
                                    naissance: c.resume.naissance.and_then(|d| {
                                        NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()
                                    }),
                                    classe: c.resume.classe,
                                    ine_fin: c.resume.ine_fin,
                                    contact_masque: c.resume.contact_masque,
                                    detail: c.resume.detail,
                                },
                            })
                            .collect(),
                        decision: dec.map(|did| DecisionVue {
                            id: did,
                            decision: r.get("dec_decision"),
                            cible_id: r.get("dec_cible"),
                            par_user_id: r.get("dec_par"),
                            le: r.get("dec_le"),
                            motif: r.get("dec_motif"),
                            en_lot: r.get("dec_lot"),
                        }),
                    }
                })
                .collect();
        Ok(PageFile {
            execution_numero: Some(numero),
            total,
            lignes,
        })
    }

    /// Comptes de la file (par objet, état, meilleure confiance) : aucune donnée personnelle. Droit `reprise.decider`.
    pub async fn reprise_file_resume(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<LigneResume>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::RepriseDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let Some((execution, _)) = derniere_execution(&mut tx, etab).await? else {
            tx.rollback().await?;
            return Ok(vec![]);
        };
        let rows = sqlx::query(
            "SELECT p.objet, (d.id IS NOT NULL) AS decidee, COALESCE(p.meilleure_confiance, 'aucune') AS confiance, count(*) AS n
               FROM rep_proposition p
               LEFT JOIN rep_correspondance c ON c.etablissement_id = p.etablissement_id AND c.source = 'next'
                    AND c.source_objet = p.objet AND c.source_id = p.source_id
               LEFT JOIN rep_decision d ON d.etablissement_id = c.etablissement_id AND d.correspondance_id = c.id
                    AND d.annulee_le IS NULL
              WHERE p.etablissement_id = $1 AND p.execution_id = $2
              GROUP BY 1, 2, 3 ORDER BY 1, 2, 3",
        )
        .bind(etab)
        .bind(execution)
        .fetch_all(&mut *tx)
        .await?;
        tx.rollback().await?;
        Ok(rows
            .iter()
            .map(|r| LigneResume {
                objet: r.get("objet"),
                decidee: r.get("decidee"),
                meilleure_confiance: r.get("confiance"),
                nombre: r.get("n"),
            })
            .collect())
    }

    /// Décide une ligne : rattacher à un candidat proposé, créer, ou ignorer. Tracée, rejouée aux exécutions
    /// suivantes, réversible tant que la reprise n'est pas appliquée. Retourne l'identifiant de la décision.
    pub async fn reprise_decider(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        d: DemandeDecision,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::RepriseDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let id = async {
            if reprise_appliquee(&mut tx, etab).await? {
                return Err(Echec::Appliquee);
            }
            let p = charger_proposition(&mut tx, etab, d.proposition_id)
                .await?
                .ok_or(Echec::Introuvable)?;
            decider_interne(&mut tx, etab, acteur, &p, &demande_de(&p, &d), false).await
        }
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Décision **en lot** : seules les lignes éligibles sont décidées (confiance forte sans contradiction pour
    /// rattacher ; aucun candidat pour créer), chacune tracée à l'unité ; les autres sont écartées et restent « à
    /// décider », avec la raison. 500 lignes au plus.
    pub async fn reprise_decider_lot(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        propositions: Vec<Uuid>,
        mode: ModeLot,
    ) -> Result<RapportLot, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::RepriseDecider)
            .await?;
        if propositions.is_empty() || propositions.len() > LOT_MAX {
            return Err(EntDbError::SaisieInvalide(
                "un lot compte de 1 à 500 lignes",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rapport = async {
            if reprise_appliquee(&mut tx, etab).await? {
                return Err(Echec::Appliquee);
            }
            let mut rapport = RapportLot::default();
            for id in propositions {
                let Some(p) = charger_proposition(&mut tx, etab, id).await? else {
                    rapport.ecartees.push((id, "inconnue".into()));
                    continue;
                };
                let demande = match mode {
                    ModeLot::RattacherMeilleurCandidat => match p.candidats.first() {
                        Some(c) if eligible_lot(c) => DecisionDemandee {
                            objet: p.objet.clone(),
                            source_id: p.source_id.clone(),
                            action: ActionDecision::Rattacher,
                            cible_id: Some(c.cible_id.clone()),
                            motif: Some(MOTIF_LOT.to_string()),
                        },
                        _ => {
                            rapport.ecartees.push((id, "non_eligible_lot".into()));
                            continue;
                        }
                    },
                    ModeLot::CreerSansCandidat if p.candidats.is_empty() => DecisionDemandee {
                        objet: p.objet.clone(),
                        source_id: p.source_id.clone(),
                        action: ActionDecision::Creer,
                        cible_id: None,
                        motif: None,
                    },
                    ModeLot::CreerSansCandidat => {
                        rapport.ecartees.push((id, "candidats_presents".into()));
                        continue;
                    }
                };
                match decider_interne(&mut tx, etab, acteur, &p, &demande, true).await {
                    Ok(_) => rapport.decidees += 1,
                    Err(Echec::DejaTraitee) => rapport.ecartees.push((id, "deja_decidee".into())),
                    Err(Echec::Refus(_)) => rapport.ecartees.push((id, "refusee".into())),
                    Err(autre) => return Err(autre),
                }
            }
            Ok(rapport)
        }
        .await?;
        tx.commit().await?;
        Ok(rapport)
    }

    /// Annule la décision en vigueur d'une ligne : elle redevient « à décider ». Motif obligatoire. Refusé quand la
    /// reprise est appliquée.
    pub async fn reprise_decision_annuler(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        proposition_id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::RepriseDecider)
            .await?;
        let motif = motif.trim();
        if !(3..=500).contains(&motif.chars().count()) {
            return Err(EntDbError::SaisieInvalide(
                "un motif de 3 à 500 caractères est obligatoire pour annuler une décision",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        async {
            if reprise_appliquee(&mut tx, etab).await? {
                return Err(Echec::Appliquee);
            }
            let p = charger_proposition(&mut tx, etab, proposition_id)
                .await?
                .ok_or(Echec::Introuvable)?;
            let corr = id_correspondance(etab, &p.objet, &p.source_id);
            let active: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM rep_decision WHERE etablissement_id = $1 AND correspondance_id = $2 AND annulee_le IS NULL",
            )
            .bind(etab)
            .bind(corr)
            .fetch_optional(&mut *tx)
            .await?;
            let decision = active.ok_or(Echec::DejaTraitee)?;
            sqlx::query(
                "UPDATE rep_decision SET annulee_le = now(), annulee_par = $3, annulee_motif = $4
                  WHERE etablissement_id = $1 AND id = $2 AND annulee_le IS NULL",
            )
            .bind(etab)
            .bind(decision)
            .bind(acteur)
            .bind(motif)
            .execute(&mut *tx)
            .await?;
            let derive = Uuid::parse_str(&id_derive(&etab.to_string(), SOURCE_NEXT, &p.objet, &p.source_id))
                .map_err(|_| Echec::Introuvable)?;
            sqlx::query(
                "UPDATE rep_correspondance SET statut = 'a_decider', regle_code = 'a_decider', cible_id = $3,
                        empreinte_source = NULL, updated_at = now()
                  WHERE etablissement_id = $1 AND id = $2 AND statut IN ('rapprochee', 'creee', 'ignoree')",
            )
            .bind(etab)
            .bind(corr)
            .bind(derive)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE rep_synonyme SET annule_le = now()
                  WHERE etablissement_id = $1 AND decision_id = $2 AND annule_le IS NULL",
            )
            .bind(etab)
            .bind(decision)
            .execute(&mut *tx)
            .await?;
            Ok::<(), Echec>(())
        }
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Équivalences de libellés validées (classes, sites), non annulées. Droit `reprise.decider`.
    pub async fn reprise_synonymes_lister(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<SynonymeVue>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::RepriseDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows = sqlx::query(
            "SELECT id, domaine, valeur_cle, valeur_cible_id, propose_par, valide_par, valide_le FROM rep_synonyme
              WHERE etablissement_id = $1 AND source = 'next' AND annule_le IS NULL
              ORDER BY domaine, valeur_cle",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await?;
        tx.rollback().await?;
        Ok(rows
            .iter()
            .map(|r| SynonymeVue {
                id: r.get("id"),
                domaine: r.get("domaine"),
                valeur_cle: r.get("valeur_cle"),
                cible_id: r.get("valeur_cible_id"),
                propose_par: r.get("propose_par"),
                valide_par: r.get("valide_par"),
                valide_le: r.get("valide_le"),
            })
            .collect())
    }

    /// Retire une équivalence validée : les exécutions suivantes ne la rejouent plus. Refusé quand la reprise est
    /// appliquée.
    pub async fn reprise_synonyme_retirer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        synonyme_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::RepriseDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if reprise_appliquee(&mut tx, etab).await? {
            return Err(Echec::Appliquee.into());
        }
        let r = sqlx::query(
            "UPDATE rep_synonyme SET annule_le = now() WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NULL",
        )
        .bind(etab)
        .bind(synonyme_id)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            return Err(Echec::Introuvable.into());
        }
        tx.commit().await?;
        Ok(())
    }
}
