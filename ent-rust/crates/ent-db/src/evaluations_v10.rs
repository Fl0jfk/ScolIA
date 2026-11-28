//! Socle 21, lot V10 « pilotage et statistiques » (`docs/socles/21_EVALUATIONS.md` §17.12, §14.6 point 10, R-V44 ;
//! EVA-153, EVA-156, EVA-212 à EVA-218, EVA-279).
//!
//! * **Évaluations communes** (CE-46) : la direction crée l'évaluation commune (libellé, date, barème, examen blanc) et
//!   y rattache les évaluations que les professeurs ont créées, **une par cours**, à la même date et au même barème.
//! * **Comparaison entre groupes** (EVA-156, EVA-214) : la direction voit chaque groupe et l'ensemble ; un professeur
//!   concerné voit ses groupes et la seule moyenne **anonyme** des autres, sous le seuil d'effectif (R-V44). Jamais de
//!   classement : les groupes sont rangés par libellé.
//! * **Écarts à l'examen blanc** (EVA-216) : pour chaque groupe, la moyenne à l'examen et la moyenne de contrôle continu
//!   (moyenne du cours sur la période de l'examen, l'examen exclu) des mêmes élèves.
//! * **Statistiques de période** (EVA-213) par matière, classe, niveau ou professeur ; **répartition par nature**
//!   (EVA-215) ; **harmonisation** entre les classes d'un niveau (EVA-279). La série est celle des **moyennes de cours
//!   des élèves**, calculées par le moteur unique (`moyenne_cours`) sur les résultats dont la période, **résolue par
//!   élève** (R-V29), est la période demandée.
//! * **Charge** d'une classe par semaine (EVA-153, EVA-217) et **évaluations non publiées** depuis longtemps
//!   (EVA-218) : des dates et des nombres, sans aucune note.
//! * **Journal des consultations** (R-V44) : chaque lecture d'une vue statistique y laisse une ligne.
//!
//! Droits (D-069, faute d'actions `evaluations.piloter` au catalogue) : la direction est reconnue à
//! `periodes.verrouiller` (comme le lot V8). Un professeur lit les statistiques de **ses** cours, la comparaison des
//! évaluations communes où il a un groupe, et la charge des classes qu'il enseigne ; le professeur principal la charge
//! de sa classe. Personnel seulement : la famille et l'élève reçoivent 403.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{Datelike, Duration, NaiveDate};
use domain_eleve::evaluation_moteur::{
    moyenne_cours, notes_ramenees, statistiques, Comptage, EntreeCalcul, EvalCalcul, Rat, ReglesMoteur,
    Statistiques, StatutCalcul, SEUIL_STATS_PLANCHER,
};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use ent_types::evaluations_v10::*;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus, texte_borne};
use crate::evaluations_v1::regles_tx;
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Événements du lot (outbox, dans la transaction du fait). Aucun ne porte d'`eleve_id` (R-V17).
pub mod types_v10 {
    pub const COMMUNE_CREEE: &str = "evaluations.commune_creee";
    pub const COMMUNE_MODIFIEE: &str = "evaluations.commune_modifiee";
}

/// Statistiques affichées (millièmes) d'une série, ou `None`.
fn stats_dto(s: Option<Statistiques>, regles: &ReglesMoteur) -> Option<StatsDto> {
    let a = s?.affichees(regles).ok()?;
    Some(StatsDto {
        effectif: a.effectif as i64,
        moyenne_m: a.moyenne_m,
        mediane_m: a.mediane_m,
        min_m: a.min_m,
        max_m: a.max_m,
    })
}

/// Statistiques d'une série ; un calcul impossible devient un avertissement, jamais une valeur inventée.
fn serie(valeurs: &[Rat], regles: &ReglesMoteur, avertissements: &mut Vec<String>, quoi: &str) -> Option<StatsDto> {
    match statistiques(valeurs, regles) {
        Ok(s) => stats_dto(s, regles),
        Err(_) => {
            avertissements.push(format!("Statistiques indisponibles pour « {quoi} » : calcul impossible."));
            None
        }
    }
}

/// Moyenne seule d'une série, en millièmes d'affichage.
fn moyenne_m(valeurs: &[Rat], regles: &ReglesMoteur) -> Option<i64> {
    let s = statistiques(valeurs, regles).ok()??;
    regles.affichage.en_milliemes(s.moyenne).ok()
}

/// Le seuil d'effectif sous lequel une moyenne anonyme n'est pas montrée (R-V12, R-V44).
fn seuil_anonymat(regles: &ReglesMoteur) -> usize {
    regles.seuil_effectif_stats.max(SEUIL_STATS_PLANCHER)
}

/// Lundi de la semaine d'un jour.
fn lundi(j: NaiveDate) -> NaiveDate {
    j - Duration::days(j.weekday().num_days_from_monday() as i64)
}

/// Une évaluation notée, lue pour le calcul.
#[derive(Debug, Clone)]
struct EvalLue {
    id: Uuid,
    cours_id: Uuid,
    titre: String,
    bareme_m: i64,
    coef_m: i64,
    comptage: String,
    hors_bareme: bool,
    nature_id: Option<Uuid>,
}

impl EvalLue {
    fn calcul(&self) -> EvalCalcul {
        EvalCalcul {
            id: self.id,
            titre: self.titre.clone(),
            bareme_m: self.bareme_m,
            coef_m: self.coef_m,
            comptage: Comptage::parse(&self.comptage).unwrap_or_default(),
            hors_bareme_autorise: self.hors_bareme,
            sous_matiere_id: None,
            cours_id: Some(self.cours_id),
            annulee: false,
        }
    }
}

/// Un résultat saisi, avec la classe de l'élève à la date de l'évaluation.
#[derive(Debug, Clone)]
struct ResLu {
    evaluation_id: Uuid,
    eleve_id: Uuid,
    classe_id: Option<Uuid>,
    date_eval: NaiveDate,
    statut: Option<StatutCalcul>,
    valeur_m: Option<i64>,
}

/// Tout ce qu'une vue de période calcule, lu dans une seule transaction.
struct DonneesPeriode {
    regles: ReglesMoteur,
    evals: HashMap<Uuid, EvalLue>,
    resultats: Vec<ResLu>,
    cours: HashMap<Uuid, CoursLu>,
    classes: HashMap<Uuid, ClasseLue>,
    profs: HashMap<Uuid, BTreeSet<Uuid>>,
}

#[derive(Debug, Clone)]
struct CoursLu {
    libelle: String,
    matiere_id: Option<Uuid>,
    matiere_libelle: Option<String>,
}

#[derive(Debug, Clone)]
struct ClasseLue {
    libelle: String,
    niveau_id: Option<Uuid>,
    niveau_libelle: Option<String>,
}

/// Une moyenne de cours d'un élève (la valeur de base des statistiques de période).
#[derive(Debug, Clone)]
struct MoyenneCours {
    cours_id: Uuid,
    /// Classe de l'élève à sa dernière évaluation du cours dans la période.
    classe_id: Option<Uuid>,
    valeur: Rat,
}

impl DonneesPeriode {
    /// Moyenne de cours de chaque (élève, cours), par le moteur ; un calcul impossible est compté.
    fn moyennes_cours(&self) -> (Vec<MoyenneCours>, usize) {
        let mut par_paire: BTreeMap<(Uuid, Uuid), (Vec<EntreeCalcul>, NaiveDate, Option<Uuid>)> = BTreeMap::new();
        for r in &self.resultats {
            let Some(ev) = self.evals.get(&r.evaluation_id) else { continue };
            let Some(statut) = r.statut else { continue };
            let e = par_paire
                .entry((r.eleve_id, ev.cours_id))
                .or_insert_with(|| (Vec::new(), r.date_eval, r.classe_id));
            if r.date_eval >= e.1 {
                e.1 = r.date_eval;
                e.2 = r.classe_id;
            }
            e.0.push(EntreeCalcul { evaluation: ev.calcul(), statut, valeur_m: r.valeur_m });
        }
        let mut out = Vec::new();
        let mut impossibles = 0;
        for ((_, cours_id), (entrees, _, classe_id)) in par_paire {
            match moyenne_cours(&entrees, &self.regles) {
                Ok(m) => {
                    if let Some(v) = m.exacte {
                        out.push(MoyenneCours { cours_id, classe_id, valeur: v });
                    }
                }
                Err(_) => impossibles += 1,
            }
        }
        (out, impossibles)
    }
}

async fn periode_tx(tx: &mut TenantTx<'_>, etab: Uuid, periode: Uuid) -> Result<(Uuid, NaiveDate, NaiveDate), EntDbError> {
    let l = sqlx::query("SELECT annee_id, date_debut, date_fin FROM ent_periode WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(periode)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
    Ok((
        l.try_get("annee_id").map_err(db)?,
        l.try_get("date_debut").map_err(db)?,
        l.try_get("date_fin").map_err(db)?,
    ))
}

/// Lit les évaluations notées ouvertes et leurs résultats dont la période **de l'élève** est `periode` (R-V29 : jamais
/// `eva_evaluation.periode_id`). `cours` restreint aux cours donnés (vue du professeur).
async fn charger_periode_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    periode: Uuid,
    cours: Option<&[Uuid]>,
) -> Result<DonneesPeriode, EntDbError> {
    let (annee_id, debut, fin) = periode_tx(tx, etab, periode).await?;
    let regles = regles_tx(tx, etab, annee_id).await?.regles;
    let filtre: Option<Vec<Uuid>> = cours.map(|c| c.to_vec());
    let lignes = sqlx::query(
        "SELECT e.id, e.cours_id, e.titre, (e.bareme * 1000)::bigint AS bareme_m, (e.coef * 1000)::bigint AS coef_m,
                e.comptage, e.hors_bareme_autorise, e.nature_id, e.date_eval,
                r.eleve_id, r.statut, (r.valeur * 1000)::bigint AS valeur_m, pe.classe_id
           FROM eva_evaluation e
           JOIN eva_resultat r ON r.etablissement_id = e.etablissement_id AND r.evaluation_id = e.id
          CROSS JOIN LATERAL eva_periode_eleve_jour(e.etablissement_id, r.eleve_id, e.date_eval) pe
          WHERE e.etablissement_id = $1 AND e.mode = 'note' AND e.etat = 'ouverte'
            AND e.date_eval BETWEEN $3 AND $4 AND pe.periode_id = $2
            AND ($5::uuid[] IS NULL OR e.cours_id = ANY($5))
          ORDER BY e.date_eval, e.id, r.eleve_id",
    )
    .bind(etab)
    .bind(periode)
    .bind(debut)
    .bind(fin)
    .bind(filtre)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut evals: HashMap<Uuid, EvalLue> = HashMap::new();
    let mut resultats = Vec::with_capacity(lignes.len());
    for l in &lignes {
        let id: Uuid = l.try_get("id").map_err(db)?;
        if !evals.contains_key(&id) {
            evals.insert(
                id,
                EvalLue {
                    id,
                    cours_id: l.try_get("cours_id").map_err(db)?,
                    titre: l.try_get("titre").map_err(db)?,
                    bareme_m: l.try_get("bareme_m").map_err(db)?,
                    coef_m: l.try_get("coef_m").map_err(db)?,
                    comptage: l.try_get("comptage").map_err(db)?,
                    hors_bareme: l.try_get("hors_bareme_autorise").map_err(db)?,
                    nature_id: l.try_get("nature_id").map_err(db)?,
                },
            );
        }
        let statut: String = l.try_get("statut").map_err(db)?;
        resultats.push(ResLu {
            evaluation_id: id,
            eleve_id: l.try_get("eleve_id").map_err(db)?,
            classe_id: l.try_get("classe_id").map_err(db)?,
            date_eval: l.try_get("date_eval").map_err(db)?,
            statut: StatutCalcul::parse(&statut),
            valeur_m: l.try_get("valeur_m").map_err(db)?,
        });
    }
    let ids_cours: Vec<Uuid> = evals.values().map(|e| e.cours_id).collect::<BTreeSet<_>>().into_iter().collect();
    let ids_classes: Vec<Uuid> =
        resultats.iter().filter_map(|r| r.classe_id).collect::<BTreeSet<_>>().into_iter().collect();
    let cours = cours_tx(tx, etab, &ids_cours).await?;
    let mut classes = HashMap::new();
    for l in sqlx::query(
        "SELECT c.id, c.libelle, c.niveau_id, n.libelle AS niveau_libelle
           FROM ent_classe c LEFT JOIN ent_niveau n ON n.id = c.niveau_id AND n.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.id = ANY($2)",
    )
    .bind(etab)
    .bind(&ids_classes)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    {
        classes.insert(
            l.try_get::<Uuid, _>("id").map_err(db)?,
            ClasseLue {
                libelle: l.try_get("libelle").map_err(db)?,
                niveau_id: l.try_get("niveau_id").map_err(db)?,
                niveau_libelle: l.try_get("niveau_libelle").map_err(db)?,
            },
        );
    }
    let mut profs: HashMap<Uuid, BTreeSet<Uuid>> = HashMap::new();
    for l in sqlx::query(
        "SELECT DISTINCT cours_id, prof_user_id FROM ent_service_enseignement
          WHERE etablissement_id = $1 AND cours_id = ANY($2) AND prof_user_id IS NOT NULL",
    )
    .bind(etab)
    .bind(&ids_cours)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    {
        profs
            .entry(l.try_get("cours_id").map_err(db)?)
            .or_default()
            .insert(l.try_get("prof_user_id").map_err(db)?);
    }
    Ok(DonneesPeriode { regles, evals, resultats, cours, classes, profs })
}

async fn cours_tx(tx: &mut TenantTx<'_>, etab: Uuid, ids: &[Uuid]) -> Result<HashMap<Uuid, CoursLu>, EntDbError> {
    let mut cours = HashMap::new();
    for l in sqlx::query(
        "SELECT c.id, c.libelle, c.matiere_id, m.libelle AS matiere_libelle
           FROM ent_cours c LEFT JOIN ent_matiere m ON m.id = c.matiere_id AND m.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.id = ANY($2)",
    )
    .bind(etab)
    .bind(ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    {
        cours.insert(
            l.try_get::<Uuid, _>("id").map_err(db)?,
            CoursLu {
                libelle: l.try_get("libelle").map_err(db)?,
                matiere_id: l.try_get("matiere_id").map_err(db)?,
                matiere_libelle: l.try_get("matiere_libelle").map_err(db)?,
            },
        );
    }
    Ok(cours)
}

/// Cours, parmi `ids`, que l'acteur enseigne (ou a enseignés et lit encore) au jour donné.
async fn mes_cours_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    acteur: Uuid,
    jour: NaiveDate,
    ids: Option<&[Uuid]>,
) -> Result<HashSet<Uuid>, EntDbError> {
    let filtre: Option<Vec<Uuid>> = ids.map(|i| i.to_vec());
    let v: Vec<Uuid> = sqlx::query_scalar(
        "SELECT c.id FROM ent_cours c
          WHERE c.etablissement_id = $1 AND ($4::uuid[] IS NULL OR c.id = ANY($4))
            AND ent_prof_sur_cours($1, c.id, $2, $3, false)",
    )
    .bind(etab)
    .bind(acteur)
    .bind(jour)
    .bind(filtre)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    Ok(v.into_iter().collect())
}

/// Une ligne du journal des consultations (R-V44).
struct Consultation {
    vue: &'static str,
    periode: Option<Uuid>,
    commune: Option<Uuid>,
    niveau: Option<Uuid>,
    axe: Option<&'static str>,
}

async fn journaliser_tx(tx: &mut TenantTx<'_>, etab: Uuid, acteur: Uuid, c: Consultation) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO eva_consultation_pilotage (id, etablissement_id, acteur_id, vue, periode_id, commune_id, niveau_id, axe)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(acteur)
    .bind(c.vue)
    .bind(c.periode)
    .bind(c.commune)
    .bind(c.niveau)
    .bind(c.axe)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Une évaluation commune lue (en-tête).
#[derive(Debug, Clone)]
struct CommuneLue {
    id: Uuid,
    annee_id: Uuid,
    libelle: String,
    date_eval: NaiveDate,
    bareme_m: i64,
    examen_blanc: bool,
    cree_le: chrono::DateTime<chrono::Utc>,
}

async fn commune_tx(tx: &mut TenantTx<'_>, etab: Uuid, id: Uuid, pour_maj: bool) -> Result<CommuneLue, EntDbError> {
    let sql = if pour_maj {
        "SELECT id, annee_id, libelle, date_eval, (bareme * 1000)::bigint AS bareme_m, examen_blanc, cree_le
           FROM eva_evaluation_commune WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
    } else {
        "SELECT id, annee_id, libelle, date_eval, (bareme * 1000)::bigint AS bareme_m, examen_blanc, cree_le
           FROM eva_evaluation_commune WHERE etablissement_id = $1 AND id = $2"
    };
    let l = sqlx::query(sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
    Ok(CommuneLue {
        id: l.try_get("id").map_err(db)?,
        annee_id: l.try_get("annee_id").map_err(db)?,
        libelle: l.try_get("libelle").map_err(db)?,
        date_eval: l.try_get("date_eval").map_err(db)?,
        bareme_m: l.try_get("bareme_m").map_err(db)?,
        examen_blanc: l.try_get("examen_blanc").map_err(db)?,
        cree_le: l.try_get("cree_le").map_err(db)?,
    })
}

/// Évaluations (non annulées) d'une commune : (id, cours, titre, publiée).
async fn evaluations_commune_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    commune: Uuid,
) -> Result<Vec<(Uuid, Uuid, String, bool)>, EntDbError> {
    let lignes = sqlx::query(
        "SELECT id, cours_id, titre, (publiee_le IS NOT NULL) AS publiee FROM eva_evaluation
          WHERE etablissement_id = $1 AND commune_id = $2 AND etat <> 'annulee'
          ORDER BY id",
    )
    .bind(etab)
    .bind(commune)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    lignes
        .iter()
        .map(|l| {
            Ok((
                l.try_get("id").map_err(db)?,
                l.try_get("cours_id").map_err(db)?,
                l.try_get("titre").map_err(db)?,
                l.try_get("publiee").map_err(db)?,
            ))
        })
        .collect()
}

fn liste_evaluations(ids: &[Uuid]) -> Result<Vec<Uuid>, EntDbError> {
    let mut vues = HashSet::new();
    let ids: Vec<Uuid> = ids.iter().copied().filter(|c| vues.insert(*c)).collect();
    if ids.len() > MAX_EVALUATIONS_PAR_COMMUNE {
        return Err(refus("trop d'évaluations pour une évaluation commune (60 au plus)"));
    }
    Ok(ids)
}

/// Rattache des évaluations à une commune (verrou sur la commune puis sur chaque évaluation). Tout ou rien.
async fn rattacher_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    acteur: Uuid,
    c: &CommuneLue,
    ids: &[Uuid],
) -> Result<usize, EntDbError> {
    let mut faits = 0;
    let mut cours_vus: HashSet<Uuid> =
        evaluations_commune_tx(tx, etab, c.id).await?.into_iter().map(|(_, cours, _, _)| cours).collect();
    for id in ids {
        let l = sqlx::query(
            "SELECT cours_id, annee_id, date_eval, mode, etat, (bareme * 1000)::bigint AS bareme_m, commune_id
               FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let commune_id: Option<Uuid> = l.try_get("commune_id").map_err(db)?;
        if commune_id == Some(c.id) {
            continue;
        }
        if commune_id.is_some() {
            return Err(conflit("une évaluation appartient déjà à une autre évaluation commune : détachez-la d'abord"));
        }
        let etat: String = l.try_get("etat").map_err(db)?;
        if etat == "annulee" {
            return Err(refus("une évaluation annulée ne se rattache pas à une évaluation commune"));
        }
        let mode: String = l.try_get("mode").map_err(db)?;
        if mode != "note" {
            return Err(refus("seule une évaluation notée se rattache à une évaluation commune"));
        }
        let annee: Uuid = l.try_get("annee_id").map_err(db)?;
        let date: NaiveDate = l.try_get("date_eval").map_err(db)?;
        let bareme: Option<i64> = l.try_get("bareme_m").map_err(db)?;
        if annee != c.annee_id || date != c.date_eval {
            return Err(refus("l'évaluation n'a pas la date de l'évaluation commune (même sujet, même date)"));
        }
        if bareme != Some(c.bareme_m) {
            return Err(refus("l'évaluation n'a pas le barème de l'évaluation commune"));
        }
        let cours: Uuid = l.try_get("cours_id").map_err(db)?;
        if !cours_vus.insert(cours) {
            return Err(conflit("ce cours a déjà une évaluation dans cette évaluation commune (une par cours)"));
        }
        sqlx::query("UPDATE eva_evaluation SET commune_id = $3, modifie_par = $4 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(c.id)
            .bind(acteur)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        faits += 1;
    }
    Ok(faits)
}

async fn marquer_modifiee_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    acteur: Uuid,
    commune: Uuid,
    rattachees: usize,
    detachees: usize,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE eva_evaluation_commune SET modifie_par = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(commune)
    .bind(acteur)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    tx.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: types_v10::COMMUNE_MODIFIEE.to_string(),
        aggregate: AggregateKind::Devoir,
        aggregate_id: commune,
        eleve_id: None,
        payload: json!({ "commune_id": commune, "rattachees": rattachees, "detachees": detachees }),
        actor_user_id: Some(acteur.to_string()),
    })
    .await?;
    Ok(())
}

impl PgEnt {
    async fn eva_v10_direction(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<bool, EntDbError> {
        Self::exiger_staff(role)?;
        self.a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller).await
    }

    async fn eva_v10_exiger_direction(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        if !self.eva_v10_direction(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    async fn eva_v10_commune_dto(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        c: &CommuneLue,
        mes_cours: &HashSet<Uuid>,
    ) -> Result<CommuneDto, EntDbError> {
        let evs = evaluations_commune_tx(tx, etab, c.id).await?;
        let ids: Vec<Uuid> = evs.iter().map(|e| e.1).collect();
        let cours = cours_tx(tx, etab, &ids).await?;
        let mut evaluations: Vec<CommuneEvaluationDto> = evs
            .into_iter()
            .map(|(id, cours_id, titre, publiee)| CommuneEvaluationDto {
                evaluation_id: id,
                cours_id,
                cours_libelle: cours.get(&cours_id).map(|c| c.libelle.clone()).unwrap_or_default(),
                titre,
                publiee,
                mon_cours: mes_cours.contains(&cours_id),
            })
            .collect();
        evaluations.sort_by(|a, b| (a.cours_libelle.to_lowercase(), a.cours_id).cmp(&(b.cours_libelle.to_lowercase(), b.cours_id)));
        Ok(CommuneDto {
            id: c.id,
            libelle: c.libelle.clone(),
            date_eval: c.date_eval,
            bareme_m: c.bareme_m,
            examen_blanc: c.examen_blanc,
            cree_le: c.cree_le,
            evaluations,
        })
    }

    // ───────────────────────── Évaluations communes ─────────────────────────

    /// Crée une évaluation commune et y rattache des évaluations (CE-46). Direction. Tout ou rien.
    pub async fn eva_v10_creer_commune_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &CreerCommuneBody,
    ) -> Result<CommuneDto, EntDbError> {
        self.eva_v10_exiger_direction(etab, acteur, role).await?;
        let libelle = texte_borne(&body.libelle, 160)?;
        if libelle.is_empty() || libelle.contains('\n') {
            return Err(refus("libellé requis, sur une ligne (160 caractères au plus)"));
        }
        if body.bareme_m <= 0 || body.bareme_m > 1_000_000 || body.bareme_m % 10 != 0 {
            return Err(refus("barème invalide (de 0,01 à 1000 points)"));
        }
        let ids = liste_evaluations(&body.evaluations)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let annee: Uuid = sqlx::query_scalar(
            "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2
              ORDER BY date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(body.date_eval)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| refus("aucune année scolaire ne contient cette date"))?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO eva_evaluation_commune (id, etablissement_id, annee_id, libelle, date_eval, bareme, examen_blanc, cree_par)
             VALUES ($1, $2, $3, $4, $5, ($6::bigint)::numeric / 1000, $7, $8)",
        )
        .bind(id)
        .bind(etab)
        .bind(annee)
        .bind(&libelle)
        .bind(body.date_eval)
        .bind(body.bareme_m)
        .bind(body.examen_blanc)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let c = commune_tx(&mut tx, etab, id, true).await?;
        let n = rattacher_tx(&mut tx, etab, acteur, &c, &ids).await?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_v10::COMMUNE_CREEE.to_string(),
            aggregate: AggregateKind::Devoir,
            aggregate_id: id,
            eleve_id: None,
            payload: json!({ "commune_id": id, "rattachees": n, "examen_blanc": body.examen_blanc }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        let dto = Self::eva_v10_commune_dto(&mut tx, etab, &c, &HashSet::new()).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Rattache des évaluations à une commune. Direction. Tout ou rien ; une évaluation déjà rattachée est ignorée.
    pub async fn eva_v10_rattacher_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        commune: Uuid,
        body: &EvaluationsBody,
    ) -> Result<CommuneDto, EntDbError> {
        self.eva_v10_exiger_direction(etab, acteur, role).await?;
        let ids = liste_evaluations(&body.evaluations)?;
        if ids.is_empty() {
            return Err(refus("au moins une évaluation est requise"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let c = commune_tx(&mut tx, etab, commune, true).await?;
        let n = rattacher_tx(&mut tx, etab, acteur, &c, &ids).await?;
        if evaluations_commune_tx(&mut tx, etab, commune).await?.len() > MAX_EVALUATIONS_PAR_COMMUNE {
            return Err(refus("trop d'évaluations pour une évaluation commune (60 au plus)"));
        }
        if n > 0 {
            marquer_modifiee_tx(&mut tx, etab, acteur, commune, n, 0).await?;
        }
        let dto = Self::eva_v10_commune_dto(&mut tx, etab, &c, &HashSet::new()).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Détache des évaluations d'une commune (elles redeviennent des évaluations ordinaires de leur cours). Direction.
    pub async fn eva_v10_detacher_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        commune: Uuid,
        body: &EvaluationsBody,
    ) -> Result<CommuneDto, EntDbError> {
        self.eva_v10_exiger_direction(etab, acteur, role).await?;
        let ids = liste_evaluations(&body.evaluations)?;
        if ids.is_empty() {
            return Err(refus("au moins une évaluation est requise"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let c = commune_tx(&mut tx, etab, commune, true).await?;
        let mut n = 0;
        for id in &ids {
            let r = sqlx::query(
                "UPDATE eva_evaluation SET commune_id = NULL, modifie_par = $4
                  WHERE etablissement_id = $1 AND id = $2 AND commune_id = $3",
            )
            .bind(etab)
            .bind(id)
            .bind(commune)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if r.rows_affected() == 0 {
                return Err(refus("une des évaluations n'appartient pas à cette évaluation commune"));
            }
            n += 1;
        }
        marquer_modifiee_tx(&mut tx, etab, acteur, commune, 0, n).await?;
        let dto = Self::eva_v10_commune_dto(&mut tx, etab, &c, &HashSet::new()).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Les évaluations communes : toutes pour la direction, celles où il a un groupe pour un professeur. Plus
    /// récentes d'abord.
    pub async fn eva_v10_communes_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<CommuneDto>, EntDbError> {
        let direction = self.eva_v10_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mes_cours = mes_cours_tx(&mut tx, etab, acteur, jour, None).await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT c.id FROM eva_evaluation_commune c
              WHERE c.etablissement_id = $1
                AND ($2 OR EXISTS (SELECT 1 FROM eva_evaluation e
                                    WHERE e.etablissement_id = c.etablissement_id AND e.commune_id = c.id
                                      AND e.etat <> 'annulee' AND e.cours_id = ANY($3)))
              ORDER BY c.date_eval DESC, c.cree_le DESC, c.id
              LIMIT 500",
        )
        .bind(etab)
        .bind(direction)
        .bind(mes_cours.iter().copied().collect::<Vec<_>>())
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let c = commune_tx(&mut tx, etab, id, false).await?;
            out.push(Self::eva_v10_commune_dto(&mut tx, etab, &c, &mes_cours).await?);
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Une évaluation commune : direction, ou professeur qui y a un groupe.
    pub async fn eva_v10_commune_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        commune: Uuid,
    ) -> Result<CommuneDto, EntDbError> {
        let direction = self.eva_v10_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let c = commune_tx(&mut tx, etab, commune, false).await?;
        let cours: Vec<Uuid> = evaluations_commune_tx(&mut tx, etab, commune).await?.into_iter().map(|e| e.1).collect();
        let mes_cours = mes_cours_tx(&mut tx, etab, acteur, jour, Some(&cours)).await?;
        if !direction && mes_cours.is_empty() {
            return Err(EntDbError::Forbidden);
        }
        let dto = Self::eva_v10_commune_dto(&mut tx, etab, &c, &mes_cours).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Compare les résultats d'une évaluation commune entre groupes (EVA-156, EVA-214, R-V44). Journalisé.
    pub async fn eva_v10_comparer_commune_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        commune: Uuid,
    ) -> Result<ComparaisonCommuneDto, EntDbError> {
        let direction = self.eva_v10_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let c = commune_tx(&mut tx, etab, commune, false).await?;
        let evs = evaluations_commune_tx(&mut tx, etab, commune).await?;
        let ids_cours: Vec<Uuid> = evs.iter().map(|e| e.1).collect();
        let mes_cours = mes_cours_tx(&mut tx, etab, acteur, jour, Some(&ids_cours)).await?;
        if !direction && mes_cours.is_empty() {
            return Err(EntDbError::Forbidden);
        }
        let regles = regles_tx(&mut tx, etab, c.annee_id).await?.regles;
        let cours = cours_tx(&mut tx, etab, &ids_cours).await?;
        let ids_evals: Vec<Uuid> = evs.iter().map(|e| e.0).collect();
        let lignes = sqlx::query(
            "SELECT e.id, e.cours_id, e.titre, (e.bareme * 1000)::bigint AS bareme_m, e.hors_bareme_autorise,
                    r.statut, (r.valeur * 1000)::bigint AS valeur_m
               FROM eva_evaluation e
               JOIN eva_resultat r ON r.etablissement_id = e.etablissement_id AND r.evaluation_id = e.id
              WHERE e.etablissement_id = $1 AND e.id = ANY($2)",
        )
        .bind(etab)
        .bind(&ids_evals)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut par_eval: HashMap<Uuid, Vec<(StatutCalcul, Option<i64>)>> = HashMap::new();
        let mut entetes: HashMap<Uuid, EvalCalcul> = HashMap::new();
        for l in &lignes {
            let id: Uuid = l.try_get("id").map_err(db)?;
            let statut: String = l.try_get("statut").map_err(db)?;
            if !entetes.contains_key(&id) {
                entetes.insert(
                    id,
                    EvalCalcul {
                        id,
                        titre: l.try_get("titre").map_err(db)?,
                        bareme_m: l.try_get::<Option<i64>, _>("bareme_m").map_err(db)?.unwrap_or(c.bareme_m),
                        coef_m: 1000,
                        comptage: Comptage::Normal,
                        hors_bareme_autorise: l.try_get("hors_bareme_autorise").map_err(db)?,
                        sous_matiere_id: None,
                        cours_id: Some(l.try_get("cours_id").map_err(db)?),
                        annulee: false,
                    },
                );
            }
            if let Some(s) = StatutCalcul::parse(&statut) {
                par_eval.entry(id).or_default().push((s, l.try_get("valeur_m").map_err(db)?));
            }
        }
        let mut avertissements = Vec::new();
        let mut groupes = Vec::new();
        let mut toutes: Vec<Rat> = Vec::new();
        let mut autres: Vec<Rat> = Vec::new();
        let mut nb_autres = 0i64;
        for (id, cours_id, titre, _) in &evs {
            let resultats = par_eval.get(id).cloned().unwrap_or_default();
            let notes = match entetes.get(id) {
                Some(ev) => match notes_ramenees(ev, &resultats, &regles) {
                    Ok(n) => n,
                    Err(_) => {
                        avertissements.push(format!("Résultats de « {titre} » indisponibles : calcul impossible."));
                        Vec::new()
                    }
                },
                None => Vec::new(),
            };
            toutes.extend(notes.iter().copied());
            if direction || mes_cours.contains(cours_id) {
                let libelle = cours.get(cours_id).map(|c| c.libelle.clone()).unwrap_or_default();
                groupes.push(GroupeCommuneDto {
                    evaluation_id: *id,
                    cours_id: *cours_id,
                    stats: serie(&notes, &regles, &mut avertissements, &libelle),
                    cours_libelle: libelle,
                    non_comptes: (resultats.len() - notes.len()) as i64,
                });
            } else {
                nb_autres += 1;
                autres.extend(notes);
            }
        }
        groupes.sort_by(|a, b| (a.cours_libelle.to_lowercase(), a.cours_id).cmp(&(b.cours_libelle.to_lowercase(), b.cours_id)));
        let (ensemble, autres_groupes) = if direction {
            (serie(&toutes, &regles, &mut avertissements, &c.libelle), None)
        } else {
            let moyenne = if autres.len() >= seuil_anonymat(&regles) { moyenne_m(&autres, &regles) } else { None };
            (None, Some(AutresGroupesDto { groupes: nb_autres, effectif: autres.len() as i64, moyenne_m: moyenne }))
        };
        if evs.is_empty() {
            avertissements.push("Aucune évaluation n'est encore rattachée à cette évaluation commune.".to_string());
        }
        journaliser_tx(
            &mut tx,
            etab,
            acteur,
            Consultation { vue: "commune", periode: None, commune: Some(commune), niveau: None, axe: None },
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ComparaisonCommuneDto {
            commune_id: c.id,
            libelle: c.libelle,
            date_eval: c.date_eval,
            bareme_m: c.bareme_m,
            examen_blanc: c.examen_blanc,
            vue: if direction { "direction" } else { "professeur" }.to_string(),
            ensemble,
            groupes,
            autres_groupes,
            avertissements,
        })
    }

    /// Écarts entre l'examen blanc et le contrôle continu, groupe par groupe (EVA-216). Direction. Journalisé.
    ///
    /// Pour chaque élève noté à l'examen : sa moyenne de cours sur **sa** période à la date de l'examen, **sans**
    /// l'examen (contrôle continu). Seuls les élèves qui ont les deux entrent dans la comparaison.
    pub async fn eva_v10_ecarts_examen_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        commune: Uuid,
    ) -> Result<EcartsExamenDto, EntDbError> {
        self.eva_v10_exiger_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let c = commune_tx(&mut tx, etab, commune, false).await?;
        if !c.examen_blanc {
            return Err(refus("cette évaluation commune n'est pas un examen blanc"));
        }
        let regles = regles_tx(&mut tx, etab, c.annee_id).await?.regles;
        let evs = evaluations_commune_tx(&mut tx, etab, commune).await?;
        let ids_cours: Vec<Uuid> = evs.iter().map(|e| e.1).collect();
        let cours = cours_tx(&mut tx, etab, &ids_cours).await?;
        // Tous les résultats des cours concernés dont la période de l'élève est celle de l'examen pour cet élève.
        let lignes = sqlx::query(
            "WITH examen AS (
                 SELECT e.id AS examen_id, e.cours_id, r.eleve_id, pe.periode_id
                   FROM eva_evaluation e
                   JOIN eva_resultat r ON r.etablissement_id = e.etablissement_id AND r.evaluation_id = e.id
                  CROSS JOIN LATERAL eva_periode_eleve_jour(e.etablissement_id, r.eleve_id, e.date_eval) pe
                  WHERE e.etablissement_id = $1 AND e.commune_id = $2 AND e.etat <> 'annulee')
             SELECT x.examen_id, x.eleve_id, e.id, e.cours_id, e.titre, (e.bareme * 1000)::bigint AS bareme_m,
                    (e.coef * 1000)::bigint AS coef_m, e.comptage, e.hors_bareme_autorise,
                    r.statut, (r.valeur * 1000)::bigint AS valeur_m
               FROM examen x
               JOIN eva_evaluation e ON e.etablissement_id = $1 AND e.cours_id = x.cours_id AND e.mode = 'note'
                                    AND e.etat = 'ouverte'
               JOIN eva_resultat r ON r.etablissement_id = e.etablissement_id AND r.evaluation_id = e.id
                                  AND r.eleve_id = x.eleve_id
              CROSS JOIN LATERAL eva_periode_eleve_jour(e.etablissement_id, r.eleve_id, e.date_eval) pe
              WHERE pe.periode_id = x.periode_id",
        )
        .bind(etab)
        .bind(commune)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // (examen, élève) → (résultat à l'examen, entrées du contrôle continu)
        type Paire = (Option<EntreeCalcul>, Vec<EntreeCalcul>);
        let mut paires: BTreeMap<(Uuid, Uuid), Paire> = BTreeMap::new();
        for l in &lignes {
            let examen: Uuid = l.try_get("examen_id").map_err(db)?;
            let eleve: Uuid = l.try_get("eleve_id").map_err(db)?;
            let id: Uuid = l.try_get("id").map_err(db)?;
            let statut: String = l.try_get("statut").map_err(db)?;
            let Some(statut) = StatutCalcul::parse(&statut) else { continue };
            let Some(bareme_m) = l.try_get::<Option<i64>, _>("bareme_m").map_err(db)? else { continue };
            let ev = EvalLue {
                id,
                cours_id: l.try_get("cours_id").map_err(db)?,
                titre: l.try_get("titre").map_err(db)?,
                bareme_m,
                coef_m: l.try_get("coef_m").map_err(db)?,
                comptage: l.try_get("comptage").map_err(db)?,
                hors_bareme: l.try_get("hors_bareme_autorise").map_err(db)?,
                nature_id: None,
            };
            let entree = EntreeCalcul { evaluation: ev.calcul(), statut, valeur_m: l.try_get("valeur_m").map_err(db)? };
            let p = paires.entry((examen, eleve)).or_default();
            if id == examen {
                p.0 = Some(entree);
            } else {
                p.1.push(entree);
            }
        }
        let mut avertissements = Vec::new();
        let mut par_examen: HashMap<Uuid, (Vec<Rat>, Vec<Rat>)> = HashMap::new();
        let (mut tout_ex, mut tout_cc) = (Vec::new(), Vec::new());
        let mut impossibles = 0;
        for ((examen, _), (ex, cc)) in paires {
            let Some(ex) = ex else { continue };
            let note_ex = match notes_ramenees(&ex.evaluation, &[(ex.statut, ex.valeur_m)], &regles) {
                Ok(n) => n.first().copied(),
                Err(_) => {
                    impossibles += 1;
                    None
                }
            };
            let moyenne_cc = match moyenne_cours(&cc, &regles) {
                Ok(m) => m.exacte,
                Err(_) => {
                    impossibles += 1;
                    None
                }
            };
            if let (Some(a), Some(b)) = (note_ex, moyenne_cc) {
                let e = par_examen.entry(examen).or_default();
                e.0.push(a);
                e.1.push(b);
                tout_ex.push(a);
                tout_cc.push(b);
            }
        }
        if impossibles > 0 {
            avertissements.push(format!("{impossibles} calcul(s) impossible(s) écarté(s) de la comparaison."));
        }
        let ecart = |ex: &[Rat], cc: &[Rat], id: Uuid, cours_id: Uuid, libelle: String| {
            let a = moyenne_m(ex, &regles);
            let b = moyenne_m(cc, &regles);
            EcartGroupeDto {
                evaluation_id: id,
                cours_id,
                cours_libelle: libelle,
                effectif: ex.len() as i64,
                moyenne_examen_m: a,
                moyenne_controle_continu_m: b,
                ecart_m: a.zip(b).map(|(a, b)| a - b),
            }
        };
        let mut groupes: Vec<EcartGroupeDto> = evs
            .iter()
            .map(|(id, cours_id, _, _)| {
                let (ex, cc) = par_examen.get(id).cloned().unwrap_or_default();
                let libelle = cours.get(cours_id).map(|c| c.libelle.clone()).unwrap_or_default();
                ecart(&ex, &cc, *id, *cours_id, libelle)
            })
            .collect();
        groupes.sort_by(|a, b| (a.cours_libelle.to_lowercase(), a.cours_id).cmp(&(b.cours_libelle.to_lowercase(), b.cours_id)));
        let ensemble = ecart(&tout_ex, &tout_cc, c.id, Uuid::nil(), c.libelle.clone());
        journaliser_tx(
            &mut tx,
            etab,
            acteur,
            Consultation { vue: "ecarts_examen", periode: None, commune: Some(commune), niveau: None, axe: None },
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(EcartsExamenDto { commune_id: c.id, libelle: c.libelle, date_eval: c.date_eval, groupes, ensemble, avertissements })
    }

    // ───────────────────────── Statistiques de période ─────────────────────────

    /// Statistiques d'une période selon un axe (EVA-213). La direction choisit l'axe ; un professeur ne voit que ses
    /// cours (axe `cours` imposé). Journalisé.
    pub async fn eva_v10_statistiques_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        axe: AxeStatistiques,
    ) -> Result<StatistiquesPeriodeDto, EntDbError> {
        let direction = self.eva_v10_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (axe, filtre) = if direction {
            (axe, None)
        } else {
            let mes: Vec<Uuid> = mes_cours_tx(&mut tx, etab, acteur, jour, None).await?.into_iter().collect();
            (AxeStatistiques::Cours, Some(mes))
        };
        let d = charger_periode_tx(&mut tx, etab, periode, filtre.as_deref()).await?;
        journaliser_tx(
            &mut tx,
            etab,
            acteur,
            Consultation { vue: "statistiques", periode: Some(periode), commune: None, niveau: None, axe: Some(axe.as_str()) },
        )
        .await?;
        tx.commit().await.map_err(db)?;

        let (moyennes, impossibles) = d.moyennes_cours();
        let mut avertissements = Vec::new();
        if impossibles > 0 {
            avertissements.push(format!("{impossibles} moyenne(s) de cours indisponible(s) : calcul impossible."));
        }
        // clé → (libellé, valeurs, cours, évaluations)
        let mut groupes: BTreeMap<Option<Uuid>, (String, Vec<Rat>, BTreeSet<Uuid>)> = BTreeMap::new();
        let cles = |m: &MoyenneCours| -> Vec<(Option<Uuid>, String)> {
            let cours = d.cours.get(&m.cours_id);
            let classe = m.classe_id.and_then(|c| d.classes.get(&c));
            match axe {
                AxeStatistiques::Matiere => vec![(
                    cours.and_then(|c| c.matiere_id),
                    cours
                        .and_then(|c| c.matiere_libelle.clone())
                        .unwrap_or_else(|| "Sans matière".to_string()),
                )],
                AxeStatistiques::Classe => vec![(
                    m.classe_id,
                    classe.map(|c| c.libelle.clone()).unwrap_or_else(|| "Sans classe".to_string()),
                )],
                AxeStatistiques::Niveau => vec![(
                    classe.and_then(|c| c.niveau_id),
                    classe.and_then(|c| c.niveau_libelle.clone()).unwrap_or_else(|| "Sans niveau".to_string()),
                )],
                AxeStatistiques::Professeur => match d.profs.get(&m.cours_id) {
                    Some(p) if !p.is_empty() => p.iter().map(|p| (Some(*p), String::new())).collect(),
                    _ => vec![(None, "Sans professeur".to_string())],
                },
                AxeStatistiques::Cours => vec![(
                    Some(m.cours_id),
                    cours.map(|c| c.libelle.clone()).unwrap_or_default(),
                )],
            }
        };
        for m in &moyennes {
            for (cle, libelle) in cles(m) {
                let g = groupes.entry(cle).or_insert_with(|| (libelle, Vec::new(), BTreeSet::new()));
                g.1.push(m.valeur);
                g.2.insert(m.cours_id);
            }
        }
        let mut lignes: Vec<LigneStatsDto> = groupes
            .into_iter()
            .map(|(cle, (libelle, valeurs, cours))| {
                let evaluations = d.evals.values().filter(|e| cours.contains(&e.cours_id)).count() as i64;
                let quoi = if libelle.is_empty() { "professeur".to_string() } else { libelle.clone() };
                LigneStatsDto {
                    cle,
                    stats: serie(&valeurs, &d.regles, &mut avertissements, &quoi),
                    libelle,
                    cours: cours.len() as i64,
                    evaluations,
                }
            })
            .collect();
        lignes.sort_by(|a, b| (a.libelle.to_lowercase(), a.cle).cmp(&(b.libelle.to_lowercase(), b.cle)));
        Ok(StatistiquesPeriodeDto { periode_id: periode, axe, lignes, avertissements })
    }

    /// Répartition des évaluations et des notes par nature d'évaluation sur une période (EVA-215). Direction.
    /// Journalisé.
    pub async fn eva_v10_natures_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode: Uuid,
    ) -> Result<RepartitionNaturesDto, EntDbError> {
        self.eva_v10_exiger_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = charger_periode_tx(&mut tx, etab, periode, None).await?;
        let natures: HashMap<Uuid, String> = sqlx::query("SELECT id, libelle FROM eva_nature WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
            .iter()
            .map(|l| Ok((l.try_get("id").map_err(db)?, l.try_get("libelle").map_err(db)?)))
            .collect::<Result<_, EntDbError>>()?;
        journaliser_tx(
            &mut tx,
            etab,
            acteur,
            Consultation { vue: "natures", periode: Some(periode), commune: None, niveau: None, axe: None },
        )
        .await?;
        tx.commit().await.map_err(db)?;

        let mut par_eval: HashMap<Uuid, Vec<(StatutCalcul, Option<i64>)>> = HashMap::new();
        for r in &d.resultats {
            if let Some(s) = r.statut {
                par_eval.entry(r.evaluation_id).or_default().push((s, r.valeur_m));
            }
        }
        let mut avertissements = Vec::new();
        // nature → (évaluations, notes, coefficients, moyennes par évaluation)
        let mut groupes: BTreeMap<Option<Uuid>, (i64, i64, i64, Vec<Rat>)> = BTreeMap::new();
        let mut total_coef = 0i64;
        for ev in d.evals.values() {
            let notes = match notes_ramenees(&ev.calcul(), par_eval.get(&ev.id).map(Vec::as_slice).unwrap_or(&[]), &d.regles) {
                Ok(n) => n,
                Err(_) => {
                    avertissements.push(format!("Résultats de « {} » indisponibles : calcul impossible.", ev.titre));
                    continue;
                }
            };
            let g = groupes.entry(ev.nature_id).or_default();
            g.0 += 1;
            g.1 += notes.len() as i64;
            g.2 += ev.coef_m;
            total_coef += ev.coef_m;
            if let Ok(Some(s)) = statistiques(&notes, &d.regles) {
                g.3.push(s.moyenne);
            }
        }
        let mut lignes: Vec<LigneNatureDto> = groupes
            .into_iter()
            .map(|(nature_id, (evaluations, notes, coefficients_m, moyennes))| {
                let libelle = nature_id
                    .and_then(|n| natures.get(&n).cloned())
                    .unwrap_or_else(|| "Sans nature".to_string());
                LigneNatureDto {
                    nature_id,
                    moyennes_evaluations: serie(&moyennes, &d.regles, &mut avertissements, &libelle),
                    libelle,
                    evaluations,
                    notes,
                    coefficients_m,
                    part_coefficients_m: if total_coef > 0 { coefficients_m * 1000 / total_coef } else { 0 },
                }
            })
            .collect();
        lignes.sort_by(|a, b| (a.libelle.to_lowercase(), a.nature_id).cmp(&(b.libelle.to_lowercase(), b.nature_id)));
        Ok(RepartitionNaturesDto { periode_id: periode, lignes, avertissements })
    }

    /// Harmonisation entre les classes d'un niveau, matière par matière (EVA-279). Direction. Aucun nom de professeur,
    /// aucun classement (R-V44). Journalisé.
    pub async fn eva_v10_harmonisation_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode: Uuid,
        niveau: Uuid,
    ) -> Result<HarmonisationDto, EntDbError> {
        self.eva_v10_exiger_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let niveau_libelle: String =
            sqlx::query_scalar("SELECT libelle FROM ent_niveau WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(niveau)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?;
        let d = charger_periode_tx(&mut tx, etab, periode, None).await?;
        journaliser_tx(
            &mut tx,
            etab,
            acteur,
            Consultation { vue: "harmonisation", periode: Some(periode), commune: None, niveau: Some(niveau), axe: None },
        )
        .await?;
        tx.commit().await.map_err(db)?;

        let (moyennes, impossibles) = d.moyennes_cours();
        let mut avertissements = Vec::new();
        if impossibles > 0 {
            avertissements.push(format!("{impossibles} moyenne(s) de cours indisponible(s) : calcul impossible."));
        }
        // matière → (libellé, valeurs du niveau, classe → valeurs)
        type ParMatiere = (String, Vec<Rat>, BTreeMap<Uuid, Vec<Rat>>);
        let mut matieres: BTreeMap<Option<Uuid>, ParMatiere> = BTreeMap::new();
        for m in &moyennes {
            let Some(classe) = m.classe_id else { continue };
            if d.classes.get(&classe).and_then(|c| c.niveau_id) != Some(niveau) {
                continue;
            }
            let cours = d.cours.get(&m.cours_id);
            let e = matieres.entry(cours.and_then(|c| c.matiere_id)).or_insert_with(|| {
                (
                    cours.and_then(|c| c.matiere_libelle.clone()).unwrap_or_else(|| "Sans matière".to_string()),
                    Vec::new(),
                    BTreeMap::new(),
                )
            });
            e.1.push(m.valeur);
            e.2.entry(classe).or_default().push(m.valeur);
        }
        let mut out: Vec<MatiereHarmonisationDto> = matieres
            .into_iter()
            .map(|(matiere_id, (libelle, valeurs, classes))| {
                let niveau_stats = serie(&valeurs, &d.regles, &mut avertissements, &libelle);
                let mut classes: Vec<ClasseHarmonisationDto> = classes
                    .into_iter()
                    .map(|(classe_id, v)| {
                        let classe_libelle = d.classes.get(&classe_id).map(|c| c.libelle.clone()).unwrap_or_default();
                        let stats = serie(&v, &d.regles, &mut avertissements, &classe_libelle);
                        ClasseHarmonisationDto {
                            classe_id,
                            ecart_m: stats.zip(niveau_stats).map(|(s, n)| s.moyenne_m - n.moyenne_m),
                            classe_libelle,
                            stats,
                        }
                    })
                    .collect();
                classes.sort_by(|a, b| {
                    (a.classe_libelle.to_lowercase(), a.classe_id).cmp(&(b.classe_libelle.to_lowercase(), b.classe_id))
                });
                MatiereHarmonisationDto { matiere_id, matiere_libelle: libelle, niveau: niveau_stats, classes }
            })
            .collect();
        out.sort_by(|a, b| (a.matiere_libelle.to_lowercase(), a.matiere_id).cmp(&(b.matiere_libelle.to_lowercase(), b.matiere_id)));
        Ok(HarmonisationDto { periode_id: periode, niveau_id: niveau, niveau_libelle, matieres: out, avertissements })
    }

    // ───────────────────────── Charge et retards ─────────────────────────

    /// Charge d'évaluations d'une classe, semaine par semaine (EVA-153, EVA-217) : évaluations ouvertes qui concernent
    /// au moins un élève de la classe à leur date. Direction, professeur principal, ou professeur qui enseigne à la
    /// classe. Sans aucune note.
    pub async fn eva_v10_charge_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        classe: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<ChargeClasseDto, EntDbError> {
        let direction = self.eva_v10_direction(etab, acteur, role).await?;
        if au < du || (au - du).num_days() > CHARGE_MAX_JOURS {
            return Err(refus("fenêtre invalide (120 jours au plus)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let existe: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2)")
                .bind(etab)
                .bind(classe)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        if !direction {
            let enseigne: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement s
                                  JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
                                 WHERE s.etablissement_id = $1 AND s.prof_user_id = $2 AND s.classe_id = $3
                                   AND y.date_debut <= $4 AND y.date_fin >= $4)
                     OR EXISTS (SELECT 1 FROM ent_cours c
                                 WHERE c.etablissement_id = $1 AND c.classe_id = $3
                                   AND ent_prof_sur_cours($1, c.id, $2, $4, false))",
            )
            .bind(etab)
            .bind(acteur)
            .bind(classe)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !enseigne {
                return Err(EntDbError::Forbidden);
            }
        }
        let lignes = sqlx::query(
            "SELECT DISTINCT e.id, e.date_eval, e.cours_id, c.libelle AS cours_libelle, e.titre, e.annoncee,
                    (e.commune_id IS NOT NULL) AS commune
               FROM eva_evaluation e
               JOIN ent_cours c ON c.id = e.cours_id AND c.etablissement_id = e.etablissement_id
              CROSS JOIN LATERAL eva_grille_eleves(e.etablissement_id, e.id) g
              WHERE e.etablissement_id = $1 AND e.etat = 'ouverte' AND e.date_eval BETWEEN $3 AND $4
                AND g.classe_id = $2
              ORDER BY e.date_eval, c.libelle, e.id",
        )
        .bind(etab)
        .bind(classe)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut evaluations = Vec::with_capacity(lignes.len());
        for l in &lignes {
            evaluations.push(EvaluationChargeDto {
                evaluation_id: l.try_get("id").map_err(db)?,
                date_eval: l.try_get("date_eval").map_err(db)?,
                cours_id: l.try_get("cours_id").map_err(db)?,
                cours_libelle: l.try_get("cours_libelle").map_err(db)?,
                titre: l.try_get("titre").map_err(db)?,
                annoncee: l.try_get("annoncee").map_err(db)?,
                commune: l.try_get("commune").map_err(db)?,
            });
        }
        Ok(ChargeClasseDto { classe_id: classe, du, au, semaines: semaines(du, au, &evaluations), evaluations })
    }

    /// Évaluations ouvertes, non publiées, dont la date dépasse le délai (EVA-218). Direction. Le délai par défaut
    /// est le délai maximal de publication de l'établissement (EVA-106), sinon 15 jours.
    pub async fn eva_v10_non_publiees_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        delai_jours: Option<i64>,
    ) -> Result<NonPublieesDto, EntDbError> {
        self.eva_v10_exiger_direction(etab, acteur, role).await?;
        if let Some(j) = delai_jours {
            if !(NON_PUBLIEE_JOURS_MIN..=NON_PUBLIEE_JOURS_MAX).contains(&j) {
                return Err(refus("délai invalide (de 1 à 120 jours)"));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let delai = match delai_jours {
            Some(j) => j,
            None => sqlx::query_scalar::<_, Option<i16>>(
                "SELECT g.delai_publication_max_jours FROM eva_reglage g
                   JOIN ent_annee_scolaire y ON y.id = g.annee_id AND y.etablissement_id = g.etablissement_id
                  WHERE g.etablissement_id = $1 AND y.date_debut <= $2 AND y.date_fin >= $2
                  ORDER BY y.date_debut DESC LIMIT 1",
            )
            .bind(etab)
            .bind(jour)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .flatten()
            .map(i64::from)
            .unwrap_or(NON_PUBLIEE_JOURS_DEFAUT),
        };
        let limite = jour - Duration::days(delai);
        let lignes = sqlx::query(
            "SELECT e.id, e.date_eval, e.cours_id, c.libelle AS cours_libelle, e.titre, e.en_cours_de_correction,
                    (e.publication = 'programmee' AND e.publication_prevue_le IS NOT NULL) AS programmee,
                    (SELECT count(*) FROM eva_resultat r
                      WHERE r.etablissement_id = e.etablissement_id AND r.evaluation_id = e.id) AS saisis,
                    ARRAY(SELECT DISTINCT s.prof_user_id FROM ent_service_enseignement s
                           WHERE s.etablissement_id = e.etablissement_id AND s.cours_id = e.cours_id
                             AND s.prof_user_id IS NOT NULL ORDER BY s.prof_user_id) AS profs
               FROM eva_evaluation e
               JOIN ent_cours c ON c.id = e.cours_id AND c.etablissement_id = e.etablissement_id
              WHERE e.etablissement_id = $1 AND e.etat = 'ouverte' AND e.publiee_le IS NULL AND e.date_eval <= $2
              ORDER BY e.date_eval, c.libelle, e.id
              LIMIT 1000",
        )
        .bind(etab)
        .bind(limite)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut evaluations = Vec::with_capacity(lignes.len());
        for l in &lignes {
            let date_eval: NaiveDate = l.try_get("date_eval").map_err(db)?;
            evaluations.push(NonPublieeDto {
                evaluation_id: l.try_get("id").map_err(db)?,
                date_eval,
                jours: (jour - date_eval).num_days(),
                cours_id: l.try_get("cours_id").map_err(db)?,
                cours_libelle: l.try_get("cours_libelle").map_err(db)?,
                titre: l.try_get("titre").map_err(db)?,
                resultats_saisis: l.try_get("saisis").map_err(db)?,
                en_cours_de_correction: l.try_get("en_cours_de_correction").map_err(db)?,
                publication_programmee: l.try_get("programmee").map_err(db)?,
                professeurs: l.try_get("profs").map_err(db)?,
            });
        }
        Ok(NonPublieesDto { jour, delai_jours: delai, evaluations })
    }

    /// Le journal des consultations de statistiques (R-V44), le plus récent d'abord (500 lignes au plus). Direction.
    pub async fn eva_v10_consultations_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        filtre_acteur: Option<Uuid>,
    ) -> Result<Vec<ConsultationDto>, EntDbError> {
        self.eva_v10_exiger_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = sqlx::query(
            "SELECT id, acteur_id, vue, periode_id, commune_id, niveau_id, axe, le FROM eva_consultation_pilotage
              WHERE etablissement_id = $1 AND ($2::uuid IS NULL OR acteur_id = $2)
              ORDER BY le DESC, id LIMIT 500",
        )
        .bind(etab)
        .bind(filtre_acteur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        lignes
            .iter()
            .map(|l| {
                Ok(ConsultationDto {
                    id: l.try_get("id").map_err(db)?,
                    acteur_id: l.try_get("acteur_id").map_err(db)?,
                    vue: l.try_get("vue").map_err(db)?,
                    periode_id: l.try_get("periode_id").map_err(db)?,
                    commune_id: l.try_get("commune_id").map_err(db)?,
                    niveau_id: l.try_get("niveau_id").map_err(db)?,
                    axe: l.try_get("axe").map_err(db)?,
                    le: l.try_get("le").map_err(db)?,
                })
            })
            .collect()
    }
}

/// Semaines (du lundi) de la fenêtre, avec le nombre d'évaluations et les jours à deux évaluations ou plus.
fn semaines(du: NaiveDate, au: NaiveDate, evaluations: &[EvaluationChargeDto]) -> Vec<SemaineChargeDto> {
    let mut par_jour: BTreeMap<NaiveDate, i64> = BTreeMap::new();
    for e in evaluations {
        *par_jour.entry(e.date_eval).or_default() += 1;
    }
    let mut out = Vec::new();
    let mut l = lundi(du);
    while l <= au {
        let fin = l + Duration::days(6);
        let evaluations = par_jour.range(l..=fin).map(|(_, n)| *n).sum();
        let jours_charges = par_jour.range(l..=fin).filter(|(_, n)| **n >= 2).map(|(j, _)| *j).collect();
        out.push(SemaineChargeDto { lundi: l, evaluations, jours_charges });
        l += Duration::days(7);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn ev(date: NaiveDate) -> EvaluationChargeDto {
        EvaluationChargeDto {
            evaluation_id: Uuid::new_v4(),
            date_eval: date,
            cours_id: Uuid::new_v4(),
            cours_libelle: "Maths".into(),
            titre: "DS".into(),
            annoncee: true,
            commune: false,
        }
    }

    #[test]
    fn semaines_partent_du_lundi_et_signalent_les_jours_charges() {
        // Mercredi 2031-10-15 → lundi 2031-10-13.
        let evs = vec![ev(d(2031, 10, 15)), ev(d(2031, 10, 15)), ev(d(2031, 10, 17)), ev(d(2031, 10, 21))];
        let s = semaines(d(2031, 10, 15), d(2031, 10, 26), &evs);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].lundi, d(2031, 10, 13));
        assert_eq!(s[0].evaluations, 3);
        assert_eq!(s[0].jours_charges, vec![d(2031, 10, 15)]);
        assert_eq!(s[1].lundi, d(2031, 10, 20));
        assert_eq!(s[1].evaluations, 1);
        assert!(s[1].jours_charges.is_empty());
    }

    #[test]
    fn liste_evaluations_dedoublonne_et_borne() {
        let a = Uuid::new_v4();
        assert_eq!(liste_evaluations(&[a, a]).unwrap(), vec![a]);
        let trop: Vec<Uuid> = (0..=MAX_EVALUATIONS_PAR_COMMUNE).map(|_| Uuid::new_v4()).collect();
        assert!(liste_evaluations(&trop).is_err());
    }

    #[test]
    fn seuil_anonymat_jamais_sous_le_plancher() {
        let mut r = ReglesMoteur::default();
        r.seuil_effectif_stats = 1;
        assert_eq!(seuil_anonymat(&r), SEUIL_STATS_PLANCHER);
        r.seuil_effectif_stats = 8;
        assert_eq!(seuil_anonymat(&r), 8);
    }

    #[test]
    fn serie_vide_ne_rend_rien() {
        let r = ReglesMoteur::default();
        let mut avert = Vec::new();
        assert!(serie(&[], &r, &mut avert, "x").is_none());
        assert!(avert.is_empty());
        let s = serie(&[Rat::entier(10), Rat::entier(14)], &r, &mut avert, "x").unwrap();
        assert_eq!(s.effectif, 2);
        assert_eq!(s.moyenne_m, 12_000);
        assert_eq!(s.min_m, 10_000);
        assert_eq!(s.max_m, 14_000);
    }
}
