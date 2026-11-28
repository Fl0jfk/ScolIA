//! Socle 21, lot V8 « fin de période » (`docs/socles/21_EVALUATIONS.md` §9, §17.8, §17.9 ; EVA-178 à EVA-193).
//!
//! * **Arrêt** (EVA-180) : aperçu (publications automatiques, cases vides), puis geste de la direction classe par
//!   classe ; à l'arrêt, toutes les évaluations de la période de la classe sont **publiées** (R-V9) et l'**instantané**
//!   est pris (R-V11). Le passage quotidien fait de même pour un arrêt automatique (EVA-181).
//! * **Instantané versionné** : moyennes de matière et générale de chaque élève (valeur exacte, valeur affichée,
//!   coefficient) et versions des appréciations validées ; empreinte SHA-256 d'une forme canonique ; une nouvelle version
//!   n'est prise que si l'empreinte change (sauf un nouvel arrêt, qui en prend toujours une).
//! * **Après l'arrêt** (R-V10, §9.3) : le professeur **demande** une correction, la direction **décide** en voyant
//!   l'effet sur les moyennes (seuils franchis signalés, CE-58), ou corrige directement ; chaque correction reversionne
//!   l'instantané. Rattrapage autorisé jusqu'à la veille du conseil.
//! * **Réouverture** (EVA-185 ★) : motif, durée, classes ; journalisée ; refermée au lendemain du terme par le passage.
//! * **Clôture des appréciations** la veille du conseil (EVA-184, passage) ou par geste ; **clôture de la période** après
//!   le conseil, bloquée en 3ᵉ par le contrôle des moyennes annuelles (EVA-188, R-03).
//! * **Pilotage** (EVA-178, EVA-179) : état de saisie par cours et par professeur, relances confirmées, rappels J-7 et
//!   J-2 automatiques (une fois par échéance) ; des nombres, jamais un nom d'élève.
//!
//! Droits (D-069, faute d'actions `evaluations.*` dédiées au catalogue) : `periodes.verrouiller` pour les gestes sur la
//! période (arrêt, clôtures, réouverture, instantané) ; `notes.corriger` pour les corrections et les rattrapages ; la
//! lecture (aperçu, état de saisie, instantanés, journaux) est ouverte à ces deux droits et au professeur principal de la
//! classe. Personnel seulement.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::evaluation_moteur::{valider_saisie, StatutCalcul};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use ent_types::evaluations_v8::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus, texte_borne};
use crate::evaluations_v1::{
    moyennes_tx, periode_eleve_tx, regles_tx, types_v1, Attendu, EcrireResultat, MoyennesEleve,
    PeriodeEleve, SourceResultat,
};
use crate::evaluations_v3::types_v3;
use crate::evaluations_v3_eval::{publier_auto, publier_evt_sys};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Événements du lot (outbox, dans la transaction du fait). Aucun ne porte d'`eleve_id` (R-V17).
pub mod types_v8 {
    pub const APPRECIATIONS_CLOSES: &str = "evaluations.appreciations_closes";
    pub const DATE_ARRET_MODIFIEE: &str = "evaluations.date_arret_modifiee";
    pub const INSTANTANE_PRIS: &str = "evaluations.instantane_pris";
    pub const RATTRAPAGE_AUTORISE: &str = "evaluations.rattrapage_autorise";
    pub const CORRECTION_DEMANDEE: &str = "evaluations.correction_demandee";
    pub const CORRECTION_DECIDEE: &str = "evaluations.correction_decidee";
    pub const RELANCE: &str = "evaluations.relance";
    pub const RAPPEL_SAISIE: &str = "evaluations.rappel_saisie";
    pub const PERIODE_CLOSE: &str = "evaluations.periode_close";
}

const STATUTS_PROPOSABLES: [&str; 6] = ["note", "absent", "a_rattraper", "dispense", "non_rendu", "non_note"];

/// Rang d'un état de période (même échelle que `eva_etat_rang`, 463).
fn rang(etat: &str) -> i32 {
    match etat {
        "ouverte" => 0,
        "notes_arretees" => 1,
        "appreciations_closes" => 2,
        "conseil_tenu" => 3,
        _ => 4,
    }
}

/// Une classe pour une période, lue avec son état.
#[derive(Debug, Clone)]
struct ClassePeriode {
    periode_id: Uuid,
    classe_id: Uuid,
    annee_id: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
    date_arret: Option<NaiveDate>,
    date_conseil: Option<NaiveDate>,
    etat: String,
}

/// Bilan du passage quotidien du lot.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BilanFinDePeriode {
    pub reouvertures_refermees: usize,
    pub instantanes: usize,
    pub publiees: usize,
    pub appreciations_closes: usize,
    pub rappels: usize,
}

impl BilanFinDePeriode {
    pub fn total(&self) -> usize {
        self.reouvertures_refermees + self.instantanes + self.publiees + self.appreciations_closes + self.rappels
    }
}

/// Une ligne calculée de l'instantané.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LigneCalc {
    eleve: Uuid,
    nature: &'static str,
    matiere: Option<Uuid>,
    num: Option<i128>,
    den: Option<i128>,
    affichee_m: Option<i64>,
    coef_m: Option<i64>,
}

struct Calcul {
    lignes: Vec<LigneCalc>,
    appreciations: Vec<(Uuid, i32)>,
    empreinte: String,
}

fn motif_borne(m: &str, min: usize, max: usize, message: &'static str) -> Result<String, EntDbError> {
    let m = texte_borne(m, max)?;
    if m.chars().count() < min {
        return Err(refus(message));
    }
    Ok(m)
}

fn classes_du_geste(classes: &[Uuid]) -> Result<Vec<Uuid>, EntDbError> {
    let mut vues = HashSet::new();
    let classes: Vec<Uuid> = classes.iter().copied().filter(|c| vues.insert(*c)).collect();
    if classes.is_empty() {
        return Err(refus("au moins une classe est requise"));
    }
    if classes.len() > MAX_CLASSES_PAR_GESTE {
        return Err(refus("trop de classes pour un seul geste (60 au plus)"));
    }
    Ok(classes)
}

async fn classe_periode_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    periode: Uuid,
    classe: Uuid,
    pour_maj: bool,
) -> Result<ClassePeriode, EntDbError> {
    let l = sqlx::query(
        "SELECT p.annee_id, p.date_debut, p.date_fin, p.date_conseil,
                (ent_periodisation_pour_classe(c.etablissement_id, c.id) = p.periodisation_id) AS applicable
           FROM ent_periode p
           JOIN ent_classe c ON c.id = $3 AND c.etablissement_id = p.etablissement_id AND c.annee_id = p.annee_id
          WHERE p.etablissement_id = $1 AND p.id = $2",
    )
    .bind(etab)
    .bind(periode)
    .bind(classe)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    if !l.try_get::<Option<bool>, _>("applicable").map_err(db)?.unwrap_or(false) {
        return Err(refus("cette période n'appartient pas à la périodisation de la classe"));
    }
    sqlx::query("SELECT eva_etat_assurer($1, $2, $3)")
        .bind(etab)
        .bind(periode)
        .bind(classe)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    let sql = if pour_maj {
        "SELECT e.etat, COALESCE(e.date_arret_effective, p.date_arret_notes) AS date_arret
           FROM eva_periode_etat e JOIN ent_periode p ON p.id = e.periode_id AND p.etablissement_id = e.etablissement_id
          WHERE e.etablissement_id = $1 AND e.periode_id = $2 AND e.classe_id = $3 FOR UPDATE OF e"
    } else {
        "SELECT e.etat, COALESCE(e.date_arret_effective, p.date_arret_notes) AS date_arret
           FROM eva_periode_etat e JOIN ent_periode p ON p.id = e.periode_id AND p.etablissement_id = e.etablissement_id
          WHERE e.etablissement_id = $1 AND e.periode_id = $2 AND e.classe_id = $3"
    };
    let e = sqlx::query(sql)
        .bind(etab)
        .bind(periode)
        .bind(classe)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
    Ok(ClassePeriode {
        periode_id: periode,
        classe_id: classe,
        annee_id: l.try_get("annee_id").map_err(db)?,
        debut: l.try_get("date_debut").map_err(db)?,
        fin: l.try_get("date_fin").map_err(db)?,
        date_arret: e.try_get("date_arret").map_err(db)?,
        date_conseil: l.try_get("date_conseil").map_err(db)?,
        etat: e.try_get("etat").map_err(db)?,
    })
}

async fn est_pp_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    acteur: Uuid,
    classe: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement s
                          JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
                         WHERE s.etablissement_id = $1 AND s.prof_user_id = $2 AND s.classe_id = $3
                           AND s.professeur_principal AND y.date_debut <= $4 AND y.date_fin >= $4)",
    )
    .bind(etab)
    .bind(acteur)
    .bind(classe)
    .bind(jour)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)
}

/// Élèves de la classe pour la période (dernière affectation de classe dans la période, 704).
async fn eleves_classe_tx(tx: &mut TenantTx<'_>, etab: Uuid, cp: &ClassePeriode) -> Result<Vec<Uuid>, EntDbError> {
    sqlx::query_scalar("SELECT eleve_id FROM eva_eleves_classe_periode($1, $2, $3) ORDER BY eleve_id")
        .bind(etab)
        .bind(cp.periode_id)
        .bind(cp.classe_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)
}

/// Évaluations ouvertes de la période qui concernent au moins un élève de la classe, avec leurs cases vides.
async fn evaluations_classe_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
) -> Result<Vec<EvaluationArretDto>, EntDbError> {
    let lignes = sqlx::query(
        "SELECT e.id, e.cours_id, e.titre, e.date_eval, (e.publiee_le IS NULL) AS a_publier,
                (e.retiree_le IS NOT NULL) AS retiree, e.en_cours_de_correction,
                count(*) AS concernes, count(*) FILTER (WHERE r.id IS NULL) AS cases_vides
           FROM eva_evaluation e
          CROSS JOIN LATERAL eva_grille_eleves(e.etablissement_id, e.id) g
           LEFT JOIN eva_resultat r ON r.etablissement_id = e.etablissement_id AND r.evaluation_id = e.id
                                   AND r.eleve_id = g.eleve_id
          WHERE e.etablissement_id = $1 AND e.etat = 'ouverte' AND e.date_eval BETWEEN $4 AND $5
            AND g.classe_id = $3 AND g.periode_id = $2
          GROUP BY e.id
          ORDER BY e.date_eval, e.id",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .bind(cp.debut)
    .bind(cp.fin)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    lignes
        .iter()
        .map(|l| {
            Ok(EvaluationArretDto {
                evaluation_id: l.try_get("id")?,
                cours_id: l.try_get("cours_id")?,
                titre: l.try_get("titre")?,
                date_eval: l.try_get("date_eval")?,
                a_publier: l.try_get("a_publier")?,
                retiree: l.try_get("retiree")?,
                en_cours_de_correction: l.try_get("en_cours_de_correction")?,
                concernes: l.try_get("concernes")?,
                cases_vides: l.try_get("cases_vides")?,
            })
        })
        .collect::<Result<_, sqlx::Error>>()
        .map_err(db)
}

fn ligne_moyenne(eleve: Uuid, nature: &'static str, matiere: Option<Uuid>, m: &domain_eleve::evaluation_moteur::MoyenneCalculee, coef_m: Option<i64>) -> LigneCalc {
    LigneCalc {
        eleve,
        nature,
        matiere,
        num: m.exacte.map(|r| r.numerateur()),
        den: m.exacte.map(|r| r.denominateur()),
        affichee_m: m.affichee_m,
        coef_m,
    }
}

fn lignes_eleve(eleve: Uuid, m: &MoyennesEleve) -> Vec<LigneCalc> {
    let mut out: Vec<LigneCalc> = m
        .matieres
        .iter()
        .map(|x| {
            let matiere = (x.matiere_id != Uuid::nil()).then_some(x.matiere_id);
            ligne_moyenne(eleve, "moyenne_matiere", matiere, &x.moyenne, Some(x.coef_m))
        })
        .collect();
    out.push(ligne_moyenne(eleve, "moyenne_generale", None, &m.generale, None));
    out
}

fn empreinte(lignes: &[LigneCalc], appreciations: &[(Uuid, i32)]) -> String {
    let mut h = Sha256::new();
    h.update(b"eva-instantane-v8\n");
    for l in lignes {
        h.update(
            format!(
                "L|{}|{}|{}|{}|{}|{}|{}\n",
                l.eleve,
                l.nature,
                l.matiere.map(|m| m.to_string()).unwrap_or_default(),
                l.num.map(|n| n.to_string()).unwrap_or_default(),
                l.den.map(|n| n.to_string()).unwrap_or_default(),
                l.affichee_m.map(|n| n.to_string()).unwrap_or_default(),
                l.coef_m.map(|n| n.to_string()).unwrap_or_default(),
            )
            .as_bytes(),
        );
    }
    for (id, v) in appreciations {
        h.update(format!("A|{id}|{v}\n").as_bytes());
    }
    format!("{:x}", h.finalize())
}

/// Calcule ce que serait l'instantané de la classe maintenant (moteur du personnel : toutes les évaluations ouvertes).
async fn calculer_tx(tx: &mut TenantTx<'_>, etab: Uuid, cp: &ClassePeriode) -> Result<Calcul, EntDbError> {
    let eleves = eleves_classe_tx(tx, etab, cp).await?;
    let mut lignes = Vec::new();
    for eleve in &eleves {
        let m = moyennes_tx(tx, etab, *eleve, cp.periode_id, false).await?;
        lignes.extend(lignes_eleve(*eleve, &m));
    }
    let appreciations: Vec<(Uuid, i32)> = sqlx::query_as(
        "SELECT id, version FROM eva_appreciation
          WHERE etablissement_id = $1 AND periode_id = $2 AND etat = 'validee' AND eleve_id = ANY($3)
          ORDER BY id",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(&eleves)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let empreinte = empreinte(&lignes, &appreciations);
    Ok(Calcul { lignes, appreciations, empreinte })
}

async fn derniere_version_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
) -> Result<Option<(i32, String)>, EntDbError> {
    sqlx::query_as(
        "SELECT version, empreinte FROM eva_instantane
          WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3 ORDER BY version DESC LIMIT 1",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)
}

/// Prend une version de l'instantané. Sans `forcer`, une empreinte identique à la dernière version ne crée rien.
/// Rend (version, empreinte, nouvelle).
async fn prendre_instantane_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
    par: Option<Uuid>,
    motif: Option<&str>,
    forcer: bool,
) -> Result<(i32, String, bool), EntDbError> {
    // Numérotation sérialisée par le verrou de l'état de la classe.
    sqlx::query("SELECT 1 FROM eva_periode_etat WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3 FOR UPDATE")
        .bind(etab)
        .bind(cp.periode_id)
        .bind(cp.classe_id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    let calcul = calculer_tx(tx, etab, cp).await?;
    let derniere = derniere_version_tx(tx, etab, cp).await?;
    if let Some((v, e)) = &derniere {
        if !forcer && *e == calcul.empreinte {
            return Ok((*v, e.clone(), false));
        }
    }
    let version = derniere.as_ref().map_or(1, |(v, _)| v + 1);
    let motif = if version == 1 { None } else { Some(motif.unwrap_or("nouvelle version").to_string()) };
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO eva_instantane (id, etablissement_id, classe_id, periode_id, version, empreinte, motif, pris_par)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(etab)
    .bind(cp.classe_id)
    .bind(cp.periode_id)
    .bind(version)
    .bind(&calcul.empreinte)
    .bind(motif.as_deref())
    .bind(par)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    for l in &calcul.lignes {
        sqlx::query(
            "INSERT INTO eva_instantane_ligne (id, etablissement_id, instantane_id, eleve_id, nature, matiere_id,
                                               valeur_num, valeur_den, valeur_affichee, coef)
             VALUES ($1, $2, $3, $4, $5, $6, $7::numeric, $8::numeric, $9::bigint / 1000.0, $10::bigint / 1000.0)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(l.eleve)
        .bind(l.nature)
        .bind(l.matiere)
        .bind(l.num.map(|n| n.to_string()))
        .bind(l.den.map(|n| n.to_string()))
        .bind(l.affichee_m)
        .bind(l.coef_m)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    }
    for (app, v) in &calcul.appreciations {
        sqlx::query(
            "INSERT INTO eva_instantane_appreciation (id, etablissement_id, instantane_id, appreciation_id, version)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(app)
        .bind(v)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    }
    tx.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: types_v8::INSTANTANE_PRIS.to_string(),
        aggregate: AggregateKind::NotePeriode,
        aggregate_id: cp.periode_id,
        eleve_id: None,
        payload: json!({ "periode_id": cp.periode_id, "classe_id": cp.classe_id, "version": version }),
        actor_user_id: par.map(|p| p.to_string()),
    })
    .await?;
    Ok((version, calcul.empreinte, true))
}

/// Après une correction : si la classe a un instantané et n'est pas close, nouvelle version si l'empreinte change.
async fn reversionner_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
    par: Uuid,
    motif: &str,
) -> Result<Option<i32>, EntDbError> {
    if !(1..=3).contains(&rang(&cp.etat)) || derniere_version_tx(tx, etab, cp).await?.is_none() {
        return Ok(None);
    }
    let (v, _, _) = prendre_instantane_tx(tx, etab, cp, Some(par), Some(motif), false).await?;
    Ok(Some(v))
}

/// Publication à l'arrêt (R-V9) puis instantané (forcé : chaque arrêt a sa version). Rend (publiées, version, empreinte).
async fn finaliser_arret_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
    par: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<(i64, i32, String), EntDbError> {
    let evaluations = evaluations_classe_tx(tx, etab, cp).await?;
    let mut publiees = 0i64;
    for e in evaluations.iter().filter(|e| e.a_publier) {
        let auteur: Option<Uuid> = sqlx::query_scalar(
            "SELECT cree_par FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2 AND publiee_le IS NULL FOR UPDATE",
        )
        .bind(etab)
        .bind(e.evaluation_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .flatten();
        publier_auto(tx, etab, e.evaluation_id, maintenant, "publiee_auto_arret").await?;
        publier_evt_sys(tx, etab, types_v3::EVALUATION_PUBLIEE, e.evaluation_id, auteur).await?;
        publiees += 1;
    }
    let premiere = derniere_version_tx(tx, etab, cp).await?.is_none();
    let motif = (!premiere).then_some("nouvel arrêt des notes");
    let (v, emp, _) = prendre_instantane_tx(tx, etab, cp, par, motif, true).await?;
    Ok((publiees, v, emp))
}

async fn refermer_reouverture_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
    origine: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE eva_reouverture SET refermee_le = now(), refermee_origine = $4
          WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3 AND refermee_le IS NULL",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .bind(origine)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

async fn changer_etat_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
    cible: &str,
    origine: &str,
    par: Option<Uuid>,
    motif: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        // Un arrêt (depuis « ouverte ») date la classe du jour de l'établissement, même si une date d'arrêt future
        // avait été fixée (EVA-182) : la date effective est celle de l'arrêt réel.
        "UPDATE eva_periode_etat SET etat = $4, origine = $5, modifie_par = $6, motif = $7,
                date_arret_effective = CASE WHEN $4 = 'notes_arretees' AND etat = 'ouverte'
                                            THEN ent_jour_etablissement($1) ELSE date_arret_effective END
          WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .bind(cible)
    .bind(origine)
    .bind(par)
    .bind(motif)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    let type_name = match cible {
        "ouverte" => types_v1::PERIODE_ROUVERTE,
        "notes_arretees" if rang(&cp.etat) > 1 => types_v1::PERIODE_ROUVERTE,
        "notes_arretees" => types_v1::NOTES_ARRETEES,
        "appreciations_closes" => types_v8::APPRECIATIONS_CLOSES,
        _ => types_v8::PERIODE_CLOSE,
    };
    tx.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::NotePeriode,
        aggregate_id: cp.periode_id,
        eleve_id: None,
        payload: json!({ "periode_id": cp.periode_id, "classe_id": cp.classe_id, "origine": origine }),
        actor_user_id: par.map(|p| p.to_string()),
    })
    .await?;
    Ok(())
}

/// Moyenne affichée de la matière (ou `None`).
fn moyenne_de(m: &MoyennesEleve, matiere: Option<Uuid>) -> Option<i64> {
    let cle = matiere.unwrap_or(Uuid::nil());
    m.matieres.iter().find(|x| x.matiere_id == cle).and_then(|x| x.moyenne.affichee_m)
}

/// Contexte d'un résultat à corriger : période de l'élève, matière du cours, barème, cours.
struct CibleCorrection {
    periode: PeriodeEleve,
    matiere: Option<Uuid>,
    cours_id: Uuid,
    bareme_m: Option<i64>,
    hors_bareme: bool,
    annee_id: Uuid,
}

async fn cible_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation: Uuid,
    eleve: Uuid,
) -> Result<Option<CibleCorrection>, EntDbError> {
    let ev = sqlx::query(
        "SELECT e.cours_id, co.matiere_id, (e.bareme * 1000)::bigint AS bareme_m, e.hors_bareme_autorise, e.annee_id
           FROM eva_evaluation e JOIN ent_cours co ON co.id = e.cours_id AND co.etablissement_id = e.etablissement_id
          WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(etab)
    .bind(evaluation)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    let Some(ev) = ev else { return Ok(None) };
    let Some(periode) = periode_eleve_tx(tx, etab, evaluation, eleve).await? else {
        return Ok(None);
    };
    Ok(Some(CibleCorrection {
        periode,
        matiere: ev.try_get("matiere_id").map_err(db)?,
        cours_id: ev.try_get("cours_id").map_err(db)?,
        bareme_m: ev.try_get("bareme_m").map_err(db)?,
        hors_bareme: ev.try_get("hors_bareme_autorise").map_err(db)?,
        annee_id: ev.try_get("annee_id").map_err(db)?,
    }))
}

fn exiger_fenetre_correction(etat: &str) -> Result<(), EntDbError> {
    match rang(etat) {
        0 => Err(conflit("la période de l'élève est ouverte : la note se saisit dans la grille")),
        1..=3 => Ok(()),
        _ => Err(conflit("période close : la correction passe par un bulletin rectificatif (socle 23)")),
    }
}

fn statut_propose(statut: &str, valeur_m: Option<i64>) -> Result<StatutCalcul, EntDbError> {
    if !STATUTS_PROPOSABLES.contains(&statut) {
        return Err(refus("statut inconnu ou non saisissable"));
    }
    let s = StatutCalcul::parse(statut).ok_or(refus("statut inconnu ou non saisissable"))?;
    if (s == StatutCalcul::Note) != valeur_m.is_some() {
        return Err(refus("une note porte une valeur, un autre statut n'en porte pas"));
    }
    Ok(s)
}

/// Écrit la correction et en mesure l'effet (avant / après, dans la même transaction).
#[allow(clippy::too_many_arguments)]
async fn ecrire_avec_effet_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    acteur: Uuid,
    jour: NaiveDate,
    cible: &CibleCorrection,
    corps: &EcrireResultat,
    attendu: Attendu,
) -> Result<(crate::evaluations_v1::ResultatEcrit, EffetCorrectionDto), EntDbError> {
    let avant = moyennes_tx(tx, etab, corps.eleve_id, cible.periode.periode_id, false).await?;
    let ecrit = PgEnt::ecrire_resultat_tx(tx, etab, acteur, true, jour, corps, attendu).await?;
    let apres = moyennes_tx(tx, etab, corps.eleve_id, cible.periode.periode_id, false).await?;
    let generale_avant = avant.generale.affichee_m;
    let generale_apres = apres.generale.affichee_m;
    let seuils = seuils_franchis(generale_avant, generale_apres);
    let mut messages = Vec::new();
    if !seuils.is_empty() {
        messages.push(
            "La moyenne générale franchit un seuil : une mention ou une décision déjà envisagée peut en dépendre."
                .to_string(),
        );
    }
    if rang(&cible.periode.etat) >= 3 {
        messages.push(
            "Le conseil de classe a déjà eu lieu : ses décisions ne changent pas seules ; si l'effet est significatif, \
             la direction rouvre la décision (socle 23)."
                .to_string(),
        );
    }
    Ok((
        ecrit,
        EffetCorrectionDto {
            eleve_id: corps.eleve_id,
            periode_id: cible.periode.periode_id,
            matiere_id: cible.matiere,
            matiere_avant_m: moyenne_de(&avant, cible.matiere),
            matiere_apres_m: moyenne_de(&apres, cible.matiere),
            generale_avant_m: generale_avant,
            generale_apres_m: generale_apres,
            seuils_franchis_m: seuils,
            etat_periode: cible.periode.etat.clone(),
            messages,
        },
    ))
}

fn dto_demande(r: &sqlx::postgres::PgRow, acteur: Uuid) -> Result<DemandeCorrectionDto, EntDbError> {
    let demandeur: Uuid = r.try_get("demandeur").map_err(db)?;
    Ok(DemandeCorrectionDto {
        id: r.try_get("id").map_err(db)?,
        evaluation_id: r.try_get("evaluation_id").map_err(db)?,
        eleve_id: r.try_get("eleve_id").map_err(db)?,
        resultat_id: r.try_get("resultat_id").map_err(db)?,
        demandeur,
        de_moi: demandeur == acteur,
        motif: r.try_get("motif").map_err(db)?,
        version_vue: r.try_get("version_vue").map_err(db)?,
        statut_propose: r.try_get("statut_propose").map_err(db)?,
        valeur_proposee_m: r.try_get("valeur_proposee_m").map_err(db)?,
        etat: r.try_get("etat").map_err(db)?,
        decision_motif: r.try_get("decision_motif").map_err(db)?,
        decide_le: r.try_get("decide_le").map_err(db)?,
        cree_le: r.try_get("cree_le").map_err(db)?,
        matiere_avant_m: r.try_get("matiere_avant_m").map_err(db)?,
        matiere_apres_m: r.try_get("matiere_apres_m").map_err(db)?,
        generale_avant_m: r.try_get("generale_avant_m").map_err(db)?,
        generale_apres_m: r.try_get("generale_apres_m").map_err(db)?,
    })
}

const SELECT_DEMANDE: &str = "SELECT d.id, r.evaluation_id, r.eleve_id, d.resultat_id, d.demandeur, d.motif, d.version_vue,
        d.statut_propose, (d.valeur_proposee * 1000)::bigint AS valeur_proposee_m, d.etat, d.decision_motif, d.decide_le,
        d.cree_le, d.matiere_avant_m, d.matiere_apres_m, d.generale_avant_m, d.generale_apres_m
   FROM eva_correction_demande d
   JOIN eva_resultat r ON r.id = d.resultat_id AND r.etablissement_id = d.etablissement_id";

/// Ce qui manque, cours par cours et professeur par professeur, pour une classe (EVA-178).
async fn etat_saisie_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
    jour: NaiveDate,
) -> Result<EtatSaisieDto, EntDbError> {
    let eleves = eleves_classe_tx(tx, etab, cp).await?;
    let evaluations = evaluations_classe_tx(tx, etab, cp).await?;
    let cours_classe: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_cours WHERE etablissement_id = $1 AND annee_id = $2 AND classe_id = $3
            AND ouvert_le <= $5 AND (ferme_le IS NULL OR ferme_le >= $4) ORDER BY id",
    )
    .bind(etab)
    .bind(cp.annee_id)
    .bind(cp.classe_id)
    .bind(cp.debut)
    .bind(cp.fin)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut cours: BTreeSet<Uuid> = cours_classe.iter().copied().collect();
    cours.extend(evaluations.iter().map(|e| e.cours_id));
    let cours: Vec<Uuid> = cours.into_iter().collect();
    let libelles: BTreeMap<Uuid, String> = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, libelle FROM ent_cours WHERE etablissement_id = $1 AND id = ANY($2)",
    )
    .bind(etab)
    .bind(&cours)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    .into_iter()
    .collect();
    let mut profs: BTreeMap<Uuid, BTreeSet<Uuid>> = BTreeMap::new();
    for (c, p) in sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT cours_id, prof_user_id FROM ent_service_enseignement
          WHERE etablissement_id = $1 AND cours_id = ANY($2) AND prof_user_id IS NOT NULL",
    )
    .bind(etab)
    .bind(&cours)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    {
        profs.entry(c).or_default().insert(p);
    }
    // Élèves de la classe concernés par chaque cours (publics des évaluations de la période).
    let mut concernes: BTreeMap<Uuid, BTreeSet<Uuid>> = BTreeMap::new();
    for (c, e) in sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT DISTINCT e.cours_id, g.eleve_id
           FROM eva_evaluation e
          CROSS JOIN LATERAL eva_grille_eleves(e.etablissement_id, e.id) g
          WHERE e.etablissement_id = $1 AND e.etat = 'ouverte' AND e.date_eval BETWEEN $4 AND $5
            AND g.classe_id = $3 AND g.periode_id = $2",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .bind(cp.debut)
    .bind(cp.fin)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    {
        concernes.entry(c).or_default().insert(e);
    }
    let avec_appreciation: HashSet<(Uuid, Uuid)> = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT cours_id, eleve_id FROM eva_appreciation
          WHERE etablissement_id = $1 AND periode_id = $2 AND type = 'cours' AND etat = 'validee' AND eleve_id = ANY($3)",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(&eleves)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    .into_iter()
    .collect();
    let classe_eleves: BTreeSet<Uuid> = eleves.iter().copied().collect();
    let arretee = rang(&cp.etat) >= 1;
    let mut lignes = Vec::new();
    let mut sans_evaluation = 0i64;
    for c in &cours {
        let evs: Vec<&EvaluationArretDto> = evaluations.iter().filter(|e| e.cours_id == *c).collect();
        let public = match concernes.get(c) {
            Some(s) => s.clone(),
            None if cours_classe.contains(c) => classe_eleves.clone(),
            None => BTreeSet::new(),
        };
        let manquantes = if arretee {
            public.iter().filter(|e| !avec_appreciation.contains(&(*c, **e))).count() as i64
        } else {
            0
        };
        if evs.is_empty() && cours_classe.contains(c) {
            sans_evaluation += 1;
        }
        lignes.push(CoursSaisieDto {
            cours_id: *c,
            libelle: libelles.get(c).cloned().unwrap_or_default(),
            professeurs: profs.get(c).map(|s| s.iter().copied().collect()).unwrap_or_default(),
            evaluations: evs.len() as i64,
            non_publiees: evs.iter().filter(|e| e.a_publier).count() as i64,
            cases_vides: evs.iter().map(|e| e.cases_vides).sum(),
            appreciations_manquantes: manquantes,
        });
    }
    let relances_du_jour: HashSet<Uuid> = sqlx::query_scalar::<_, Uuid>(
        "SELECT prof_user_id FROM eva_relance WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3
            AND nature = 'manuelle' AND jour = $4",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .bind(jour)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    .into_iter()
    .collect();
    let tous_profs: BTreeSet<Uuid> = profs.values().flatten().copied().collect();
    let professeurs = tous_profs
        .iter()
        .map(|p| {
            let siens: Vec<&CoursSaisieDto> = lignes.iter().filter(|l| l.professeurs.contains(p)).collect();
            let non_publiees = siens.iter().map(|l| l.non_publiees).sum();
            let cases_vides = siens.iter().map(|l| l.cases_vides).sum();
            let sans = siens.iter().filter(|l| l.evaluations == 0 && cours_classe.contains(&l.cours_id)).count() as i64;
            let appreciations = siens.iter().map(|l| l.appreciations_manquantes).sum();
            ManquesProfesseurDto {
                prof_user_id: *p,
                non_publiees,
                cases_vides,
                cours_sans_evaluation: sans,
                appreciations_manquantes: appreciations,
                message: message_relance(non_publiees, cases_vides, sans, appreciations, cp.date_arret.filter(|_| !arretee)),
                relance_aujourdhui: relances_du_jour.contains(p),
            }
        })
        .collect();
    Ok(EtatSaisieDto {
        periode_id: cp.periode_id,
        classe_id: cp.classe_id,
        etat: cp.etat.clone(),
        date_arret: cp.date_arret,
        jours_avant_arret: cp.date_arret.map(|d| (d - jour).num_days()),
        date_conseil: cp.date_conseil,
        eleves: eleves.len() as i64,
        cours: lignes,
        cours_sans_evaluation: sans_evaluation,
        professeurs,
    })
}

#[allow(clippy::too_many_arguments)]
async fn inserer_relance_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
    m: &ManquesProfesseurDto,
    nature: &str,
    jour: NaiveDate,
    echeance: Option<NaiveDate>,
    message: Option<&str>,
    par: Option<Uuid>,
) -> Result<Option<Uuid>, EntDbError> {
    let id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO eva_relance (id, etablissement_id, periode_id, classe_id, prof_user_id, nature, jour, echeance,
                                  non_publiees, cases_vides, cours_sans_evaluation, message, par)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT DO NOTHING RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .bind(m.prof_user_id)
    .bind(nature)
    .bind(jour)
    .bind(echeance)
    .bind(m.non_publiees as i32)
    .bind(m.cases_vides as i32)
    .bind(m.cours_sans_evaluation as i32)
    .bind(message)
    .bind(par)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    if let Some(id) = id {
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: if nature == "manuelle" { types_v8::RELANCE } else { types_v8::RAPPEL_SAISIE }.to_string(),
            aggregate: AggregateKind::NotePeriode,
            aggregate_id: cp.periode_id,
            eleve_id: None,
            payload: json!({ "relance_id": id, "classe_id": cp.classe_id }),
            actor_user_id: par.map(|p| p.to_string()),
        })
        .await?;
    }
    Ok(id)
}

/// Contrôle de fin d'année de 3ᵉ (R-03, CE-63, EVA-188).
async fn controle_3e_tx(tx: &mut TenantTx<'_>, etab: Uuid, cp: &ClassePeriode) -> Result<Controle3eDto, EntDbError> {
    let applicable: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM ent_classe c
              LEFT JOIN ent_mef m ON m.id = c.mef_id AND m.etablissement_id = c.etablissement_id
              JOIN ent_niveau n ON n.etablissement_id = c.etablissement_id AND n.id = COALESCE(c.niveau_id, m.niveau_id)
             WHERE c.etablissement_id = $1 AND c.id = $2
               AND (btrim(n.code) ~* '^3(e|eme|ème)?$' OR btrim(n.libelle) ~* '^(3 ?(e|eme|ème)|troisi[eè]me)([^[:alnum:]]|$)'))
         AND NOT EXISTS (
            SELECT 1 FROM ent_periode p JOIN ent_periode p2 ON p2.etablissement_id = p.etablissement_id
               AND p2.periodisation_id = p.periodisation_id AND p2.annee_id = p.annee_id AND p2.date_debut > p.date_debut
             WHERE p.etablissement_id = $1 AND p.id = $3)",
    )
    .bind(etab)
    .bind(cp.classe_id)
    .bind(cp.periode_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?;
    let mut manques = Vec::new();
    if applicable {
        let periodes: Vec<Uuid> = sqlx::query_scalar(
            "SELECT p2.id FROM ent_periode p JOIN ent_periode p2 ON p2.etablissement_id = p.etablissement_id
                AND p2.periodisation_id = p.periodisation_id AND p2.annee_id = p.annee_id
              WHERE p.etablissement_id = $1 AND p.id = $2 ORDER BY p2.date_debut",
        )
        .bind(etab)
        .bind(cp.periode_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
        let (annee_debut, annee_fin): (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(cp.annee_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        for eleve in eleves_classe_tx(tx, etab, cp).await? {
            let mut avec_moyenne: HashSet<Uuid> = HashSet::new();
            for p in &periodes {
                let m = moyennes_tx(tx, etab, eleve, *p, false).await?;
                avec_moyenne.extend(m.matieres.iter().filter(|x| x.moyenne.exacte.is_some()).map(|x| x.matiere_id));
            }
            let dispenses: Vec<Option<Uuid>> = sqlx::query_scalar(
                "SELECT matiere_id FROM eva_dispense_port
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND nature = 'dispense' AND retiree_le IS NULL
                    AND du <= $4 AND (au IS NULL OR au >= $3)",
            )
            .bind(etab)
            .bind(eleve)
            .bind(annee_debut)
            .bind(annee_fin)
            .fetch_all(&mut **tx)
            .await
            .map_err(db)?;
            if dispenses.iter().any(Option::is_none) {
                continue;
            }
            let obligatoires: Vec<(Uuid, String)> = sqlx::query_as(
                "SELECT DISTINCT co.matiere_id, mt.libelle
                   FROM ent_cours co
                   JOIN ent_matiere mt ON mt.id = co.matiere_id AND mt.etablissement_id = co.etablissement_id
                   LEFT JOIN ent_offre_enseignement o ON o.id = co.offre_id AND o.etablissement_id = co.etablissement_id
                  WHERE co.etablissement_id = $1 AND co.annee_id = $2 AND co.matiere_id IS NOT NULL
                    AND COALESCE(o.modalite, 'commun') NOT IN ('option_facultative', 'dispositif', 'accompagnement', 'autre')
                    AND (co.classe_id = $3 OR EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $4, co.id, NULL, $5)))
                  ORDER BY mt.libelle, co.matiere_id",
            )
            .bind(etab)
            .bind(cp.annee_id)
            .bind(cp.classe_id)
            .bind(cp.fin)
            .bind(eleve)
            .fetch_all(&mut **tx)
            .await
            .map_err(db)?;
            for (matiere, libelle) in obligatoires {
                if !avec_moyenne.contains(&matiere) && !dispenses.contains(&Some(matiere)) {
                    manques.push(ManqueAnnuelDto { eleve_id: eleve, matiere_id: matiere, matiere: libelle });
                }
            }
        }
    }
    Ok(Controle3eDto {
        periode_id: cp.periode_id,
        classe_id: cp.classe_id,
        applicable,
        cloture_permise: !applicable || manques.is_empty(),
        manques,
    })
}

impl PgEnt {
    async fn eva_v8_exiger(&self, etab: Uuid, acteur: Uuid, role: &str, action: Action) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if !self.a_action_etablissement(etab, acteur, role, action).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    /// Lecture du pilotage : `periodes.verrouiller`, `notes.corriger` ou professeur principal de la classe.
    async fn eva_v8_exiger_lecture(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        classe: Uuid,
        jour: NaiveDate,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if self.a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller).await?
            || self.a_action_etablissement(etab, acteur, role, Action::NotesCorriger).await?
            || est_pp_tx(tx, etab, acteur, classe, jour).await?
        {
            return Ok(());
        }
        Err(EntDbError::Forbidden)
    }

    async fn eva_v8_caches(&self, etab: Uuid, acteur: Uuid, role: &str, ids: &[Uuid]) -> Result<HashSet<Uuid>, EntDbError> {
        let profil = self.profil_champs(etab, acteur, role).await?;
        self.proteges_caches(etab, role, &profil, ids).await
    }

    // ───────────────────────── Arrêt ─────────────────────────

    /// Aperçu de l'arrêt d'une classe (EVA-180) : ce qui sera publié, les cases vides.
    pub async fn eva_v8_apercu_arret_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        classe: Uuid,
    ) -> Result<ApercuArretDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.eva_v8_exiger_lecture(&mut tx, etab, acteur, role, classe, jour).await?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, false).await?;
        let eleves = eleves_classe_tx(&mut tx, etab, &cp).await?;
        let evaluations = evaluations_classe_tx(&mut tx, etab, &cp).await?;
        tx.rollback().await.map_err(db)?;
        let mut avertissements = Vec::new();
        let retirees = evaluations.iter().filter(|e| e.a_publier && e.retiree).count();
        if retirees > 0 {
            avertissements.push(format!(
                "{retirees} évaluation(s) retirée(s) pour vérification seront publiées à l'arrêt : vérifiez-les avant."
            ));
        }
        let en_correction = evaluations.iter().filter(|e| e.a_publier && e.en_cours_de_correction).count();
        if en_correction > 0 {
            avertissements.push(format!(
                "{en_correction} évaluation(s) « en cours de correction » seront publiées à l'arrêt, telles quelles."
            ));
        }
        if rang(&cp.etat) > 0 {
            avertissements.push("Cette classe est déjà arrêtée.".to_string());
        }
        Ok(ApercuArretDto {
            periode_id: periode,
            classe_id: classe,
            etat: cp.etat.clone(),
            date_arret: cp.date_arret,
            eleves: eleves.len() as i64,
            a_publier: evaluations.iter().filter(|e| e.a_publier).count() as i64,
            cases_vides: evaluations.iter().map(|e| e.cases_vides).sum(),
            evaluations,
            avertissements,
        })
    }

    /// Arrête les notes des classes (EVA-180 ★) : publication de tout (R-V9), instantané (R-V11). Tout ou rien.
    pub async fn eva_v8_arreter_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        maintenant: DateTime<Utc>,
        periode: Uuid,
        body: &ArreterBody,
    ) -> Result<Vec<GesteClasseDto>, EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::PeriodesVerrouiller).await?;
        let classes = classes_du_geste(&body.classes)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut out = Vec::new();
        for c in classes {
            let cp = classe_periode_tx(&mut tx, etab, periode, c, true).await?;
            if cp.etat != "ouverte" {
                return Err(conflit("une des classes n'est pas ouverte : rien n'a été arrêté"));
            }
            changer_etat_tx(&mut tx, etab, &cp, "notes_arretees", "geste", Some(acteur), "arrêt des notes").await?;
            refermer_reouverture_tx(&mut tx, etab, &cp, "geste").await?;
            let cp = ClassePeriode { etat: "notes_arretees".into(), ..cp };
            let (publiees, v, emp) = finaliser_arret_tx(&mut tx, etab, &cp, Some(acteur), maintenant).await?;
            out.push(GesteClasseDto {
                classe_id: c,
                etat: cp.etat.clone(),
                publiees,
                instantane_version: Some(v),
                empreinte: Some(emp),
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Fixe une date d'arrêt propre à des classes encore ouvertes (EVA-182, CE-41).
    pub async fn eva_v8_date_arret_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        body: &DateArretBody,
    ) -> Result<(), EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::PeriodesVerrouiller).await?;
        let classes = classes_du_geste(&body.classes)?;
        if body.date_arret < jour {
            return Err(refus("la date d'arrêt est déjà passée : arrêtez les notes maintenant"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        for c in classes {
            let cp = classe_periode_tx(&mut tx, etab, periode, c, true).await?;
            if cp.etat != "ouverte" {
                return Err(conflit("une des classes est déjà arrêtée : rien n'a été modifié"));
            }
            if body.date_arret < cp.debut || body.date_arret > cp.fin {
                return Err(refus("la date d'arrêt doit tomber dans la période"));
            }
            sqlx::query(
                "UPDATE eva_periode_etat SET date_arret_effective = $4, modifie_par = $5
                  WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3",
            )
            .bind(etab)
            .bind(periode)
            .bind(c)
            .bind(body.date_arret)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_v8::DATE_ARRET_MODIFIEE.to_string(),
                aggregate: AggregateKind::NotePeriode,
                aggregate_id: periode,
                eleve_id: None,
                payload: json!({ "periode_id": periode, "classe_id": c, "date_arret": body.date_arret }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Clôt les appréciations des classes (EVA-184), puis reversionne leur instantané (appréciations comprises).
    pub async fn eva_v8_clore_appreciations_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode: Uuid,
        body: &ClassesBody,
    ) -> Result<Vec<GesteClasseDto>, EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::PeriodesVerrouiller).await?;
        let classes = classes_du_geste(&body.classes)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut out = Vec::new();
        for c in classes {
            let cp = classe_periode_tx(&mut tx, etab, periode, c, true).await?;
            if cp.etat != "notes_arretees" {
                return Err(conflit("une des classes n'est pas au temps des appréciations : rien n'a été clos"));
            }
            changer_etat_tx(&mut tx, etab, &cp, "appreciations_closes", "geste", Some(acteur), "clôture des appréciations").await?;
            refermer_reouverture_tx(&mut tx, etab, &cp, "geste").await?;
            let cp = ClassePeriode { etat: "appreciations_closes".into(), ..cp };
            let v = reversionner_tx(&mut tx, etab, &cp, acteur, "clôture des appréciations").await?;
            out.push(GesteClasseDto { classe_id: c, etat: cp.etat.clone(), publiees: 0, instantane_version: v, empreinte: None });
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Rouvre la période pour des classes (EVA-185 ★) : motif d'au moins 10 caractères, durée bornée, journal.
    pub async fn eva_v8_rouvrir_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        body: &RouvrirBody,
    ) -> Result<Vec<ReouvertureDto>, EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::PeriodesVerrouiller).await?;
        let classes = classes_du_geste(&body.classes)?;
        let motif = motif_borne(&body.motif, MOTIF_REOUVERTURE_MIN, 500, "un motif de dix caractères au moins est requis")?;
        let vers = body.vers.as_deref().unwrap_or("ouverte");
        if !["ouverte", "notes_arretees"].contains(&vers) {
            return Err(refus("une réouverture revient à « ouverte » ou à « notes arrêtées »"));
        }
        if body.jusqu_au < jour || body.jusqu_au > jour + Duration::days(REOUVERTURE_MAX_JOURS) {
            return Err(refus("la réouverture dure de aujourd'hui à trente jours au plus"));
        }
        let lot = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut out = Vec::new();
        for c in classes {
            let cp = classe_periode_tx(&mut tx, etab, periode, c, true).await?;
            if rang(&cp.etat) <= rang(vers) {
                return Err(conflit("une des classes n'est pas plus avancée que l'état demandé : rien n'a été rouvert"));
            }
            refermer_reouverture_tx(&mut tx, etab, &cp, "geste").await?;
            changer_etat_tx(&mut tx, etab, &cp, vers, "geste", Some(acteur), &motif).await?;
            let r = sqlx::query(
                "INSERT INTO eva_reouverture (id, etablissement_id, lot_id, periode_id, classe_id, etat_avant, etat_apres,
                                              motif, jusqu_au, par)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 RETURNING id, le",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(lot)
            .bind(periode)
            .bind(c)
            .bind(&cp.etat)
            .bind(vers)
            .bind(&motif)
            .bind(body.jusqu_au)
            .bind(acteur)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            out.push(ReouvertureDto {
                id: r.try_get("id").map_err(db)?,
                lot_id: lot,
                classe_id: c,
                etat_avant: cp.etat.clone(),
                etat_apres: vers.to_string(),
                motif: motif.clone(),
                jusqu_au: body.jusqu_au,
                le: r.try_get("le").map_err(db)?,
                refermee_le: None,
                refermee_origine: None,
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Journal des réouvertures d'une classe pour la période.
    pub async fn eva_v8_reouvertures_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        classe: Uuid,
    ) -> Result<Vec<ReouvertureDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.eva_v8_exiger_lecture(&mut tx, etab, acteur, role, classe, jour).await?;
        let lignes = sqlx::query(
            "SELECT id, lot_id, classe_id, etat_avant, etat_apres, motif, jusqu_au, le, refermee_le, refermee_origine
               FROM eva_reouverture WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3 ORDER BY le, id",
        )
        .bind(etab)
        .bind(periode)
        .bind(classe)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.rollback().await.map_err(db)?;
        lignes
            .iter()
            .map(|r| {
                Ok(ReouvertureDto {
                    id: r.try_get("id")?,
                    lot_id: r.try_get("lot_id")?,
                    classe_id: r.try_get("classe_id")?,
                    etat_avant: r.try_get("etat_avant")?,
                    etat_apres: r.try_get("etat_apres")?,
                    motif: r.try_get("motif")?,
                    jusqu_au: r.try_get("jusqu_au")?,
                    le: r.try_get("le")?,
                    refermee_le: r.try_get("refermee_le")?,
                    refermee_origine: r.try_get("refermee_origine")?,
                })
            })
            .collect::<Result<_, sqlx::Error>>()
            .map_err(db)
    }

    /// Contrôle de 3ᵉ (EVA-188) : lecture.
    pub async fn eva_v8_controle_3e_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        classe: Uuid,
    ) -> Result<Controle3eDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.eva_v8_exiger_lecture(&mut tx, etab, acteur, role, classe, jour).await?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, false).await?;
        let r = controle_3e_tx(&mut tx, etab, &cp).await?;
        tx.rollback().await.map_err(db)?;
        let caches = self.eva_v8_caches(etab, acteur, role, &r.manques.iter().map(|m| m.eleve_id).collect::<Vec<_>>()).await?;
        Ok(Controle3eDto { manques: r.manques.into_iter().filter(|m| !caches.contains(&m.eleve_id)).collect(), ..r })
    }

    /// Clôt la période d'une classe après le conseil (EVA-187), sauf contrôle de 3ᵉ en échec (R-03).
    pub async fn eva_v8_clore_periode_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode: Uuid,
        classe: Uuid,
    ) -> Result<GesteClasseDto, EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::PeriodesVerrouiller).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, true).await?;
        if cp.etat != "conseil_tenu" {
            return Err(conflit("la période se clôt après le conseil de classe"));
        }
        let controle = controle_3e_tx(&mut tx, etab, &cp).await?;
        if !controle.cloture_permise {
            return Err(conflit(
                "contrôle de 3ᵉ : des enseignements obligatoires n'ont pas de moyenne annuelle (voir le contrôle)",
            ));
        }
        changer_etat_tx(&mut tx, etab, &cp, "close", "geste", Some(acteur), "clôture de la période").await?;
        refermer_reouverture_tx(&mut tx, etab, &cp, "geste").await?;
        tx.commit().await.map_err(db)?;
        Ok(GesteClasseDto { classe_id: classe, etat: "close".into(), publiees: 0, instantane_version: None, empreinte: None })
    }

    // ───────────────────────── Instantané ─────────────────────────

    /// Versions de l'instantané d'une classe et fraîcheur de la dernière (EVA-186).
    pub async fn eva_v8_instantanes_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        classe: Uuid,
    ) -> Result<InstantanesDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.eva_v8_exiger_lecture(&mut tx, etab, acteur, role, classe, jour).await?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, false).await?;
        let versions = versions_tx(&mut tx, etab, &cp).await?;
        let courante = calculer_tx(&mut tx, etab, &cp).await?.empreinte;
        tx.rollback().await.map_err(db)?;
        let a_jour = versions.last().is_some_and(|v| v.empreinte == courante);
        Ok(InstantanesDto { periode_id: periode, classe_id: classe, etat: cp.etat, versions, empreinte_courante: courante, a_jour })
    }

    /// Une version de l'instantané, et ses écarts avec `comparer_a` (EVA-186).
    #[allow(clippy::too_many_arguments)]
    pub async fn eva_v8_instantane_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        classe: Uuid,
        version: i32,
        comparer_a: Option<i32>,
    ) -> Result<InstantaneDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.eva_v8_exiger_lecture(&mut tx, etab, acteur, role, classe, jour).await?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, false).await?;
        let versions = versions_tx(&mut tx, etab, &cp).await?;
        let v = versions.iter().find(|x| x.version == version).cloned().ok_or_else(introuvable)?;
        let lignes = lignes_version_tx(&mut tx, etab, &cp, version).await?;
        let (ecarts, apps_changees) = match comparer_a {
            Some(autre) => {
                if !versions.iter().any(|x| x.version == autre) {
                    return Err(introuvable());
                }
                let anciennes = lignes_version_tx(&mut tx, etab, &cp, autre).await?;
                let apps: i64 = sqlx::query_scalar(
                    "WITH a AS (SELECT ia.appreciation_id, ia.version FROM eva_instantane_appreciation ia
                                  JOIN eva_instantane i ON i.id = ia.instantane_id AND i.etablissement_id = ia.etablissement_id
                                 WHERE i.etablissement_id = $1 AND i.periode_id = $2 AND i.classe_id = $3 AND i.version = $4),
                          b AS (SELECT ia.appreciation_id, ia.version FROM eva_instantane_appreciation ia
                                  JOIN eva_instantane i ON i.id = ia.instantane_id AND i.etablissement_id = ia.etablissement_id
                                 WHERE i.etablissement_id = $1 AND i.periode_id = $2 AND i.classe_id = $3 AND i.version = $5)
                     SELECT count(*) FROM a FULL JOIN b ON a.appreciation_id = b.appreciation_id
                      WHERE a.version IS DISTINCT FROM b.version",
                )
                .bind(etab)
                .bind(periode)
                .bind(classe)
                .bind(autre)
                .bind(version)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                (ecarts(&anciennes, &lignes), apps)
            }
            None => (Vec::new(), 0),
        };
        tx.rollback().await.map_err(db)?;
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.eleve_id).collect::<BTreeSet<_>>().into_iter().collect();
        let caches = self.eva_v8_caches(etab, acteur, role, &ids).await?;
        Ok(InstantaneDto {
            version: v,
            lignes: lignes.into_iter().filter(|l| !caches.contains(&l.eleve_id)).collect(),
            comparee_a: comparer_a,
            ecarts: ecarts.into_iter().filter(|l| !caches.contains(&l.eleve_id)).collect(),
            appreciations_changees: apps_changees,
        })
    }

    /// Reprend l'instantané (après un rattrapage, par exemple) : nouvelle version motivée si l'empreinte a changé.
    pub async fn eva_v8_reprendre_instantane_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode: Uuid,
        classe: Uuid,
        body: &ReprendreInstantaneBody,
    ) -> Result<RepriseInstantaneDto, EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::PeriodesVerrouiller).await?;
        let motif = motif_borne(&body.motif, 3, 500, "un motif de trois caractères au moins est requis")?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, true).await?;
        if !(1..=3).contains(&rang(&cp.etat)) {
            return Err(conflit("l'instantané se prend entre l'arrêt des notes et la clôture de la période"));
        }
        let (version, empreinte, nouvelle) =
            prendre_instantane_tx(&mut tx, etab, &cp, Some(acteur), Some(&motif), false).await?;
        tx.commit().await.map_err(db)?;
        Ok(RepriseInstantaneDto { version, empreinte, nouvelle })
    }

    // ───────────────────────── Rattrapages et corrections ─────────────────────────

    /// Autorise un rattrapage après l'arrêt (EVA-183, R-V10) : classe arrêtée, au plus jusqu'à la veille du conseil.
    pub async fn eva_v8_autoriser_rattrapage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &AutoriserRattrapageBody,
    ) -> Result<(), EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::NotesCorriger).await?;
        self.exiger_eleve_visible(etab, acteur, role, body.eleve_id).await?;
        let motif = motif_borne(&body.motif, 3, 500, "un motif de trois caractères au moins est requis")?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cible = cible_tx(&mut tx, etab, body.evaluation_id, body.eleve_id).await?.ok_or_else(introuvable)?;
        match rang(&cible.periode.etat) {
            0 => return Err(conflit("la période de l'élève est ouverte : le rattrapage se saisit sans autorisation")),
            1 | 2 => {}
            _ => return Err(conflit("le conseil a eu lieu : un rattrapage n'est plus possible (correction motivée)")),
        }
        let conseil: Option<NaiveDate> = sqlx::query_scalar("SELECT date_conseil FROM ent_periode WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(cible.periode.periode_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if body.jusqu_au < jour {
            return Err(refus("la date de fin du rattrapage est déjà passée"));
        }
        if conseil.is_some_and(|c| body.jusqu_au >= c) {
            return Err(refus("un rattrapage s'arrête au plus tard la veille du conseil de classe"));
        }
        sqlx::query(
            "INSERT INTO eva_rattrapage_autorise (id, evaluation_id, eleve_id, etablissement_id, jusqu_au, motif, par)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(body.evaluation_id)
        .bind(body.eleve_id)
        .bind(etab)
        .bind(body.jusqu_au)
        .bind(&motif)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_v8::RATTRAPAGE_AUTORISE.to_string(),
            aggregate: AggregateKind::Devoir,
            aggregate_id: body.evaluation_id,
            eleve_id: None,
            payload: json!({ "evaluation_id": body.evaluation_id }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Demande de correction après l'arrêt (EVA-189), par un professeur du cours.
    pub async fn eva_v8_demander_correction_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &DemanderCorrectionBody,
    ) -> Result<DemandeCorrectionDto, EntDbError> {
        Self::exiger_staff(role)?;
        let motif = motif_borne(&body.motif, 3, 1000, "un motif de trois caractères au moins est requis")?;
        let statut = statut_propose(&body.statut, body.valeur_m)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cible = cible_tx(&mut tx, etab, body.evaluation_id, body.eleve_id).await?;
        let Some(cible) = cible else { return Err(EntDbError::Forbidden) };
        let du_cours: bool = sqlx::query_scalar(
            "SELECT ent_prof_sur_cours($1, $2, $3, $4, true)
                 OR EXISTS (SELECT 1 FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2
                                                                     AND prof_user_id = $3)",
        )
        .bind(etab)
        .bind(cible.cours_id)
        .bind(acteur)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !du_cours {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, body.eleve_id).await?;
        exiger_fenetre_correction(&cible.periode.etat)?;
        let pas = regles_tx(&mut tx, etab, cible.annee_id).await?.pas_saisie_m;
        let bareme = cible.bareme_m.ok_or(refus("évaluation sans barème"))?;
        valider_saisie(statut, body.valeur_m, bareme, cible.hors_bareme, pas)
            .map_err(|_| refus("proposition invalide : note hors barème, hors pas de saisie ou statut incohérent"))?;
        let r: Option<(Uuid, i32)> = sqlx::query_as(
            "SELECT id, version FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(body.evaluation_id)
        .bind(body.eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((resultat_id, version)) = r else {
            return Err(refus("aucun résultat à corriger : demandez un rattrapage à la direction"));
        };
        if version != body.version_vue {
            return Err(conflit("version périmée : la case a changé depuis votre lecture"));
        }
        let id = Uuid::new_v4();
        let ins = sqlx::query(
            "INSERT INTO eva_correction_demande (id, etablissement_id, resultat_id, demandeur, motif, version_vue,
                                                 statut_propose, valeur_proposee)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8::bigint / 1000.0)",
        )
        .bind(id)
        .bind(etab)
        .bind(resultat_id)
        .bind(acteur)
        .bind(&motif)
        .bind(version)
        .bind(statut.as_str())
        .bind(body.valeur_m)
        .execute(&mut *tx)
        .await;
        match ins {
            Ok(_) => {}
            Err(e) if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") => {
                return Err(conflit("une demande est déjà en attente pour ce résultat"));
            }
            Err(e) => return Err(db(e)),
        }
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_v8::CORRECTION_DEMANDEE.to_string(),
            aggregate: AggregateKind::Devoir,
            aggregate_id: body.evaluation_id,
            eleve_id: None,
            payload: json!({ "demande_id": id, "classe_id": cible.periode.classe_id }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        let r = sqlx::query(&format!("{SELECT_DEMANDE} WHERE d.etablissement_id = $1 AND d.id = $2"))
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let dto = dto_demande(&r, acteur)?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Demandes de correction : toutes pour `notes.corriger`, les siennes pour un professeur.
    pub async fn eva_v8_demandes_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        toutes: bool,
    ) -> Result<Vec<DemandeCorrectionDto>, EntDbError> {
        Self::exiger_staff(role)?;
        let direction = self.a_action_etablissement(etab, acteur, role, Action::NotesCorriger).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = sqlx::query(&format!(
            "{SELECT_DEMANDE} WHERE d.etablissement_id = $1 AND d.resultat_id IS NOT NULL
               AND ($2 OR d.demandeur = $3) AND ($4 OR d.etat = 'en_attente')
             ORDER BY d.cree_le DESC, d.id LIMIT 500"
        ))
        .bind(etab)
        .bind(direction)
        .bind(acteur)
        .bind(toutes)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.rollback().await.map_err(db)?;
        let mut out = lignes.iter().map(|r| dto_demande(r, acteur)).collect::<Result<Vec<_>, _>>()?;
        let ids: Vec<Uuid> = out.iter().map(|d| d.eleve_id).collect::<BTreeSet<_>>().into_iter().collect();
        let caches = self.eva_v8_caches(etab, acteur, role, &ids).await?;
        out.retain(|d| !caches.contains(&d.eleve_id));
        Ok(out)
    }

    async fn eva_v8_charger_demande(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        acteur: Uuid,
        id: Uuid,
    ) -> Result<DemandeCorrectionDto, EntDbError> {
        let r = sqlx::query(&format!("{SELECT_DEMANDE} WHERE d.etablissement_id = $1 AND d.id = $2 FOR UPDATE OF d"))
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        dto_demande(&r, acteur)
    }

    fn corps_demande(d: &DemandeCorrectionDto, motif: &str) -> Result<EcrireResultat, EntDbError> {
        let statut = d.statut_propose.as_deref().ok_or(refus("demande sans proposition (antérieure au lot V8)"))?;
        Ok(EcrireResultat {
            evaluation_id: d.evaluation_id,
            eleve_id: d.eleve_id,
            statut: StatutCalcul::parse(statut).ok_or(refus("statut inconnu"))?,
            valeur_m: d.valeur_proposee_m,
            motif: Some(motif.chars().take(500).collect()),
            commentaire_eleve: None,
            source: SourceResultat::Saisie,
        })
    }

    /// Effet d'une demande, sans rien écrire (transaction annulée).
    pub async fn eva_v8_effet_demande_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        id: Uuid,
    ) -> Result<EffetCorrectionDto, EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::NotesCorriger).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = self.eva_v8_charger_demande(&mut tx, etab, acteur, id).await?;
        self.exiger_eleve_visible(etab, acteur, role, d.eleve_id).await?;
        if d.etat != "en_attente" {
            return Err(conflit("demande déjà traitée"));
        }
        let cible = cible_tx(&mut tx, etab, d.evaluation_id, d.eleve_id).await?.ok_or_else(introuvable)?;
        exiger_fenetre_correction(&cible.periode.etat)?;
        let corps = Self::corps_demande(&d, &format!("Simulation : {}", d.motif))?;
        let attendu = d.version_vue.map_or(Attendu::Aucun, Attendu::Version);
        let (_, effet) = ecrire_avec_effet_tx(&mut tx, etab, acteur, jour, &cible, &corps, attendu).await?;
        tx.rollback().await.map_err(db)?;
        Ok(effet)
    }

    /// Décide d'une demande (EVA-190 ★) : refus motivé, ou acceptation qui écrit la correction, en garde l'effet et
    /// reversionne l'instantané.
    pub async fn eva_v8_decider_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        id: Uuid,
        body: &DeciderCorrectionBody,
    ) -> Result<DemandeCorrectionDto, EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::NotesCorriger).await?;
        let motif = body
            .motif
            .as_deref()
            .map(|m| motif_borne(m, 3, 1000, "un motif de trois caractères au moins est requis"))
            .transpose()?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = self.eva_v8_charger_demande(&mut tx, etab, acteur, id).await?;
        self.exiger_eleve_visible(etab, acteur, role, d.eleve_id).await?;
        if d.etat != "en_attente" {
            return Err(conflit("demande déjà traitée"));
        }
        if !body.accepter {
            let motif = motif.ok_or(refus("un refus est motivé"))?;
            sqlx::query(
                "UPDATE eva_correction_demande SET etat = 'refusee', decide_par = $3, decide_le = now(), decision_motif = $4
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(acteur)
            .bind(&motif)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        } else {
            let cible = cible_tx(&mut tx, etab, d.evaluation_id, d.eleve_id).await?.ok_or_else(introuvable)?;
            exiger_fenetre_correction(&cible.periode.etat)?;
            let texte = format!("Correction demandée par le professeur et acceptée : {}", d.motif);
            let corps = Self::corps_demande(&d, &texte)?;
            let attendu = d.version_vue.map_or(Attendu::Aucun, Attendu::Version);
            let (ecrit, effet) = ecrire_avec_effet_tx(&mut tx, etab, acteur, jour, &cible, &corps, attendu).await?;
            sqlx::query(
                "UPDATE eva_correction_demande SET etat = 'acceptee', decide_par = $3, decide_le = now(), decision_motif = $4,
                        version_ecrite = $5, matiere_avant_m = $6, matiere_apres_m = $7, generale_avant_m = $8,
                        generale_apres_m = $9
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(acteur)
            .bind(motif.as_deref())
            .bind(ecrit.version)
            .bind(effet.matiere_avant_m)
            .bind(effet.matiere_apres_m)
            .bind(effet.generale_avant_m)
            .bind(effet.generale_apres_m)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let cp = classe_periode_tx(&mut tx, etab, cible.periode.periode_id, cible.periode.classe_id, false).await?;
            reversionner_tx(&mut tx, etab, &cp, acteur, "correction acceptée").await?;
        }
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_v8::CORRECTION_DECIDEE.to_string(),
            aggregate: AggregateKind::Devoir,
            aggregate_id: d.evaluation_id,
            eleve_id: None,
            payload: json!({ "demande_id": id, "acceptee": body.accepter }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        let dto = self.eva_v8_charger_demande(&mut tx, etab, acteur, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Retire sa demande tant qu'elle est en attente.
    pub async fn eva_v8_retirer_demande_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = self.eva_v8_charger_demande(&mut tx, etab, acteur, id).await?;
        if !d.de_moi {
            return Err(introuvable());
        }
        if d.etat != "en_attente" {
            return Err(conflit("demande déjà traitée"));
        }
        sqlx::query("UPDATE eva_correction_demande SET etat = 'retiree', retiree_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Correction directe par la direction (EVA-191 ★), avec son effet et la nouvelle version de l'instantané.
    pub async fn eva_v8_corriger_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &CorrigerBody,
    ) -> Result<CorrectionFaiteDto, EntDbError> {
        self.eva_v8_exiger(etab, acteur, role, Action::NotesCorriger).await?;
        self.exiger_eleve_visible(etab, acteur, role, body.eleve_id).await?;
        let motif = motif_borne(&body.motif, 3, 500, "un motif de trois caractères au moins est requis")?;
        let statut = statut_propose(&body.statut, body.valeur_m)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cible = cible_tx(&mut tx, etab, body.evaluation_id, body.eleve_id).await?.ok_or_else(introuvable)?;
        exiger_fenetre_correction(&cible.periode.etat)?;
        let corps = EcrireResultat {
            evaluation_id: body.evaluation_id,
            eleve_id: body.eleve_id,
            statut,
            valeur_m: body.valeur_m,
            motif: Some(motif.clone()),
            commentaire_eleve: None,
            source: SourceResultat::Saisie,
        };
        let attendu = body.version_attendue.map_or(Attendu::Vide, Attendu::Version);
        let (ecrit, effet) = ecrire_avec_effet_tx(&mut tx, etab, acteur, jour, &cible, &corps, attendu).await?;
        let cp = classe_periode_tx(&mut tx, etab, cible.periode.periode_id, cible.periode.classe_id, false).await?;
        let v = reversionner_tx(&mut tx, etab, &cp, acteur, &format!("correction : {motif}")).await?;
        tx.commit().await.map_err(db)?;
        Ok(CorrectionFaiteDto { resultat_id: ecrit.resultat_id, version: ecrit.version, effet, instantane_version: v })
    }

    /// Journal des corrections d'une classe pour la période (EVA-193 : direction ; export pour un recours).
    pub async fn eva_v8_journal_corrections_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode: Uuid,
        classe: Uuid,
    ) -> Result<Vec<LigneJournalCorrectionDto>, EntDbError> {
        Self::exiger_staff(role)?;
        if !(self.a_action_etablissement(etab, acteur, role, Action::NotesCorriger).await?
            || self.a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller).await?)
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, false).await?;
        let lignes = sqlx::query(
            "SELECT h.le, h.evaluation_id, ev.titre, h.eleve_id, h.version, h.statut_avant,
                    (h.valeur_avant * 1000)::bigint AS valeur_avant_m, h.statut_apres,
                    (h.valeur_apres * 1000)::bigint AS valeur_apres_m, h.motif, h.auteur
               FROM eva_resultat_historique h
               JOIN eva_evaluation ev ON ev.id = h.evaluation_id AND ev.etablissement_id = h.etablissement_id
              WHERE h.etablissement_id = $1 AND h.correction
                AND h.eleve_id IN (SELECT eleve_id FROM eva_eleves_classe_periode($1, $2, $3))
                AND eva_periode_dans($1, $2, ev.date_eval)
              ORDER BY h.le, h.id LIMIT 2000",
        )
        .bind(etab)
        .bind(cp.periode_id)
        .bind(cp.classe_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.rollback().await.map_err(db)?;
        let mut out = lignes
            .iter()
            .map(|r| {
                Ok(LigneJournalCorrectionDto {
                    le: r.try_get("le")?,
                    evaluation_id: r.try_get("evaluation_id")?,
                    titre: r.try_get("titre")?,
                    eleve_id: r.try_get("eleve_id")?,
                    version: r.try_get("version")?,
                    statut_avant: r.try_get("statut_avant")?,
                    valeur_avant_m: r.try_get("valeur_avant_m")?,
                    statut_apres: r.try_get("statut_apres")?,
                    valeur_apres_m: r.try_get("valeur_apres_m")?,
                    motif: r.try_get("motif")?,
                    auteur: r.try_get("auteur")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)?;
        let ids: Vec<Uuid> = out.iter().map(|l| l.eleve_id).collect::<BTreeSet<_>>().into_iter().collect();
        let caches = self.eva_v8_caches(etab, acteur, role, &ids).await?;
        out.retain(|l| !caches.contains(&l.eleve_id));
        Ok(out)
    }

    // ───────────────────────── Pilotage ─────────────────────────

    /// État de saisie d'une classe (EVA-178), avec le message prérempli de chaque relance (aperçu d'EVA-179).
    pub async fn eva_v8_etat_saisie_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        classe: Uuid,
    ) -> Result<EtatSaisieDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.eva_v8_exiger_lecture(&mut tx, etab, acteur, role, classe, jour).await?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, false).await?;
        let r = etat_saisie_tx(&mut tx, etab, &cp, jour).await?;
        tx.rollback().await.map_err(db)?;
        Ok(r)
    }

    /// Relance des professeurs (EVA-179) : une fois par jour, par professeur, par classe.
    #[allow(clippy::too_many_arguments)]
    pub async fn eva_v8_relancer_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        classe: Uuid,
        body: &RelancerBody,
    ) -> Result<Vec<RelanceDto>, EntDbError> {
        let message = body
            .message
            .as_deref()
            .map(|m| motif_borne(m, 3, 1000, "un message de trois caractères au moins est requis"))
            .transpose()?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.eva_v8_exiger_lecture(&mut tx, etab, acteur, role, classe, jour).await?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, false).await?;
        let etat = etat_saisie_tx(&mut tx, etab, &cp, jour).await?;
        let cibles: Vec<&ManquesProfesseurDto> = if body.professeurs.is_empty() {
            etat.professeurs.iter().filter(|p| p.a_des_manques()).collect()
        } else {
            let mut v = Vec::new();
            for p in &body.professeurs {
                v.push(
                    etat.professeurs
                        .iter()
                        .find(|x| x.prof_user_id == *p)
                        .ok_or(refus("ce professeur n'enseigne pas à cette classe"))?,
                );
            }
            v
        };
        let mut ids = Vec::new();
        for m in cibles {
            let texte = message.clone().unwrap_or_else(|| m.message.clone());
            if let Some(id) =
                inserer_relance_tx(&mut tx, etab, &cp, m, "manuelle", jour, None, Some(&texte), Some(acteur)).await?
            {
                ids.push(id);
            }
        }
        let relances = relances_tx(&mut tx, etab, &cp, Some(&ids)).await?;
        tx.commit().await.map_err(db)?;
        Ok(relances)
    }

    /// Journal des relances et rappels d'une classe.
    pub async fn eva_v8_relances_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode: Uuid,
        classe: Uuid,
    ) -> Result<Vec<RelanceDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.eva_v8_exiger_lecture(&mut tx, etab, acteur, role, classe, jour).await?;
        let cp = classe_periode_tx(&mut tx, etab, periode, classe, false).await?;
        let r = relances_tx(&mut tx, etab, &cp, None).await?;
        tx.rollback().await.map_err(db)?;
        Ok(r)
    }

    // ───────────────────────── Passage quotidien ─────────────────────────

    /// Passage du lot pour un établissement (idempotent) : réouvertures échues refermées, arrêts finalisés
    /// (publication et instantané), appréciations closes la veille du conseil, rappels J-7 et J-2.
    pub async fn eva_v8_passage_au(
        &self,
        etab: Uuid,
        jour: Option<NaiveDate>,
        maintenant: DateTime<Utc>,
    ) -> Result<BilanFinDePeriode, EntDbError> {
        let jour = match jour {
            Some(j) => j,
            None => self.aujourdhui_etablissement(etab).await?,
        };
        let mut b = BilanFinDePeriode::default();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // 1. Réouvertures échues : la classe rouverte en saisie est de nouveau arrêtée.
        let echues: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT periode_id, classe_id, etat_apres FROM eva_reouverture
              WHERE etablissement_id = $1 AND refermee_le IS NULL AND jusqu_au < $2 ORDER BY le, id",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (p, c, _) in echues {
            let cp = classe_periode_tx(&mut tx, etab, p, c, true).await?;
            if cp.etat == "ouverte" {
                changer_etat_tx(&mut tx, etab, &cp, "notes_arretees", "auto", None, "fin de la réouverture").await?;
            }
            refermer_reouverture_tx(&mut tx, etab, &cp, "auto").await?;
            b.reouvertures_refermees += 1;
        }
        // 2. Arrêts sans instantané depuis leur dernier arrêt (arrêt automatique, fin de réouverture, reprise).
        let a_finaliser: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT e.periode_id, e.classe_id FROM eva_periode_etat e
              WHERE e.etablissement_id = $1 AND e.etat = 'notes_arretees'
                AND NOT EXISTS (SELECT 1 FROM eva_instantane i
                                 WHERE i.etablissement_id = e.etablissement_id AND i.periode_id = e.periode_id
                                   AND i.classe_id = e.classe_id AND i.pris_le >= COALESCE(e.arretee_le, '-infinity'))
              ORDER BY e.periode_id, e.classe_id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (p, c) in a_finaliser {
            let cp = classe_periode_tx(&mut tx, etab, p, c, true).await?;
            let (publiees, _, _) = finaliser_arret_tx(&mut tx, etab, &cp, None, maintenant).await?;
            b.publiees += publiees as usize;
            b.instantanes += 1;
        }
        // 3. Clôture des appréciations la veille du conseil (réglage d'arrêt automatique), hors réouverture en cours.
        let a_clore: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT e.periode_id, e.classe_id FROM eva_periode_etat e
               JOIN ent_periode p ON p.id = e.periode_id AND p.etablissement_id = e.etablissement_id
               LEFT JOIN eva_reglage g ON g.etablissement_id = p.etablissement_id AND g.annee_id = p.annee_id
              WHERE e.etablissement_id = $1 AND e.etat = 'notes_arretees' AND COALESCE(g.arret_auto, true)
                AND p.date_conseil IS NOT NULL AND p.date_conseil - 1 <= $2
                AND NOT EXISTS (SELECT 1 FROM eva_reouverture r WHERE r.etablissement_id = e.etablissement_id
                                   AND r.periode_id = e.periode_id AND r.classe_id = e.classe_id AND r.refermee_le IS NULL)
              ORDER BY e.periode_id, e.classe_id",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (p, c) in a_clore {
            let cp = classe_periode_tx(&mut tx, etab, p, c, true).await?;
            changer_etat_tx(&mut tx, etab, &cp, "appreciations_closes", "auto", None, "clôture automatique la veille du conseil").await?;
            let cp = ClassePeriode { etat: "appreciations_closes".into(), ..cp };
            if derniere_version_tx(&mut tx, etab, &cp).await?.is_some() {
                prendre_instantane_tx(&mut tx, etab, &cp, None, Some("clôture des appréciations"), false).await?;
            }
            b.appreciations_closes += 1;
        }
        // 4. Rappels J-7 et J-2 aux professeurs des classes encore ouvertes.
        let ouvertes: Vec<(Uuid, Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT p.id, c.id, COALESCE(e.date_arret_effective, p.date_arret_notes)
               FROM ent_periode p
               JOIN ent_classe c ON c.etablissement_id = p.etablissement_id AND c.annee_id = p.annee_id
                AND ent_periodisation_pour_classe(c.etablissement_id, c.id) = p.periodisation_id
               LEFT JOIN eva_periode_etat e ON e.etablissement_id = p.etablissement_id AND e.periode_id = p.id
                AND e.classe_id = c.id
              WHERE p.etablissement_id = $1 AND COALESCE(e.etat, 'ouverte') = 'ouverte'
                AND COALESCE(e.date_arret_effective, p.date_arret_notes) BETWEEN $2 AND $2 + 7
              ORDER BY p.id, c.id",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (p, c, arret) in ouvertes {
            let reste = (arret - jour).num_days();
            let nature = if reste <= RAPPEL_J2 { "j2" } else if reste <= RAPPEL_J7 { "j7" } else { continue };
            let cp = classe_periode_tx(&mut tx, etab, p, c, false).await?;
            let etat = etat_saisie_tx(&mut tx, etab, &cp, jour).await?;
            for m in etat.professeurs.iter().filter(|m| m.a_des_manques()) {
                if inserer_relance_tx(&mut tx, etab, &cp, m, nature, jour, Some(arret), Some(&m.message), None).await?.is_some() {
                    b.rappels += 1;
                }
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(b)
    }
}

async fn versions_tx(tx: &mut TenantTx<'_>, etab: Uuid, cp: &ClassePeriode) -> Result<Vec<VersionInstantaneDto>, EntDbError> {
    let lignes = sqlx::query(
        "SELECT i.version, i.empreinte, i.motif, i.pris_le, i.pris_par,
                (SELECT count(*) FROM eva_instantane_ligne l WHERE l.etablissement_id = i.etablissement_id AND l.instantane_id = i.id) AS lignes,
                (SELECT count(*) FROM eva_instantane_appreciation a WHERE a.etablissement_id = i.etablissement_id AND a.instantane_id = i.id) AS appreciations
           FROM eva_instantane i
          WHERE i.etablissement_id = $1 AND i.periode_id = $2 AND i.classe_id = $3 ORDER BY i.version",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    lignes
        .iter()
        .map(|r| {
            Ok(VersionInstantaneDto {
                version: r.try_get("version")?,
                empreinte: r.try_get("empreinte")?,
                motif: r.try_get("motif")?,
                pris_le: r.try_get("pris_le")?,
                pris_par: r.try_get("pris_par")?,
                lignes: r.try_get("lignes")?,
                appreciations: r.try_get("appreciations")?,
            })
        })
        .collect::<Result<_, sqlx::Error>>()
        .map_err(db)
}

async fn lignes_version_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
    version: i32,
) -> Result<Vec<LigneInstantaneDto>, EntDbError> {
    let lignes = sqlx::query(
        "SELECT l.eleve_id, l.nature, l.matiere_id, (l.valeur_affichee * 1000)::bigint AS valeur_m,
                (l.coef * 1000)::bigint AS coef_m
           FROM eva_instantane_ligne l
           JOIN eva_instantane i ON i.id = l.instantane_id AND i.etablissement_id = l.etablissement_id
          WHERE i.etablissement_id = $1 AND i.periode_id = $2 AND i.classe_id = $3 AND i.version = $4
          ORDER BY l.eleve_id, l.nature DESC, l.matiere_id NULLS LAST",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .bind(version)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    lignes
        .iter()
        .map(|r| {
            Ok(LigneInstantaneDto {
                eleve_id: r.try_get("eleve_id")?,
                nature: r.try_get("nature")?,
                matiere_id: r.try_get("matiere_id")?,
                valeur_m: r.try_get("valeur_m")?,
                coef_m: r.try_get("coef_m")?,
            })
        })
        .collect::<Result<_, sqlx::Error>>()
        .map_err(db)
}

fn ecarts(avant: &[LigneInstantaneDto], apres: &[LigneInstantaneDto]) -> Vec<EcartInstantaneDto> {
    type Cle = (Uuid, String, Option<Uuid>);
    let a: BTreeMap<Cle, Option<i64>> =
        avant.iter().map(|l| ((l.eleve_id, l.nature.clone(), l.matiere_id), l.valeur_m)).collect();
    let b: BTreeMap<Cle, Option<i64>> =
        apres.iter().map(|l| ((l.eleve_id, l.nature.clone(), l.matiere_id), l.valeur_m)).collect();
    let cles: BTreeSet<&Cle> = a.keys().chain(b.keys()).collect();
    cles.into_iter()
        .filter_map(|k| {
            let x = a.get(k).copied().flatten();
            let y = b.get(k).copied().flatten();
            (x != y || a.contains_key(k) != b.contains_key(k)).then(|| EcartInstantaneDto {
                eleve_id: k.0,
                nature: k.1.clone(),
                matiere_id: k.2,
                avant_m: x,
                apres_m: y,
            })
        })
        .collect()
}

async fn relances_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    cp: &ClassePeriode,
    seulement: Option<&[Uuid]>,
) -> Result<Vec<RelanceDto>, EntDbError> {
    let lignes = sqlx::query(
        "SELECT id, prof_user_id, nature, jour, echeance, non_publiees, cases_vides, cours_sans_evaluation, message, par, le
           FROM eva_relance
          WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3 AND ($4::uuid[] IS NULL OR id = ANY($4))
          ORDER BY le, id",
    )
    .bind(etab)
    .bind(cp.periode_id)
    .bind(cp.classe_id)
    .bind(seulement.map(<[Uuid]>::to_vec))
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    lignes
        .iter()
        .map(|r| {
            Ok(RelanceDto {
                id: r.try_get("id")?,
                prof_user_id: r.try_get("prof_user_id")?,
                nature: r.try_get("nature")?,
                jour: r.try_get("jour")?,
                echeance: r.try_get("echeance")?,
                non_publiees: i64::from(r.try_get::<i32, _>("non_publiees")?),
                cases_vides: i64::from(r.try_get::<i32, _>("cases_vides")?),
                cours_sans_evaluation: i64::from(r.try_get::<i32, _>("cours_sans_evaluation")?),
                message: r.try_get("message")?,
                par: r.try_get("par")?,
                le: r.try_get("le")?,
            })
        })
        .collect::<Result<_, sqlx::Error>>()
        .map_err(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(e: u128, nature: &str, m: Option<u128>, v: Option<i64>) -> LigneInstantaneDto {
        LigneInstantaneDto {
            eleve_id: Uuid::from_u128(e),
            nature: nature.into(),
            matiere_id: m.map(Uuid::from_u128),
            valeur_m: v,
            coef_m: None,
        }
    }

    #[test]
    fn ecarts_entre_versions() {
        let a = vec![l(1, "moyenne_matiere", Some(9), Some(9_960)), l(1, "moyenne_generale", None, Some(9_960))];
        let b = vec![
            l(1, "moyenne_matiere", Some(9), Some(10_040)),
            l(1, "moyenne_generale", None, Some(9_960)),
            l(2, "moyenne_generale", None, None),
        ];
        let e = ecarts(&a, &b);
        assert_eq!(e.len(), 2, "{e:?}");
        assert_eq!((e[0].avant_m, e[0].apres_m), (Some(9_960), Some(10_040)));
        assert_eq!(e[1].eleve_id, Uuid::from_u128(2));
        assert!(ecarts(&a, &a).is_empty());
    }

    #[test]
    fn empreinte_stable_et_sensible() {
        let ligne = LigneCalc {
            eleve: Uuid::from_u128(1),
            nature: "moyenne_generale",
            matiere: None,
            num: Some(249),
            den: Some(25),
            affichee_m: Some(9_960),
            coef_m: None,
        };
        let e1 = empreinte(std::slice::from_ref(&ligne), &[]);
        assert_eq!(e1, empreinte(std::slice::from_ref(&ligne), &[]));
        assert_eq!(e1.len(), 64);
        assert_ne!(e1, empreinte(std::slice::from_ref(&ligne), &[(Uuid::from_u128(5), 1)]));
        let autre = LigneCalc { num: Some(251), affichee_m: Some(10_040), ..ligne };
        assert_ne!(e1, empreinte(&[autre], &[]));
    }

    #[test]
    fn fenetre_de_correction() {
        assert!(exiger_fenetre_correction("ouverte").is_err());
        assert!(exiger_fenetre_correction("notes_arretees").is_ok());
        assert!(exiger_fenetre_correction("conseil_tenu").is_ok());
        assert!(exiger_fenetre_correction("close").is_err());
    }
}
