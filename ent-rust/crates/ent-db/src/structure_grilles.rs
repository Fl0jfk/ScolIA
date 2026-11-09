//! Socle 5, lot S7 : grille horaire **versionnée** (`05_STRUCTURE.md` §9.3, STR-34 et STR-35).
//!
//! Une grille est une **version** (`ent_grille`, migration 336) : créneaux de la journée (cours,
//! récréations, pause méridienne, étude), coupure des demi-journées, valable à partir d'une date.
//! Les versions d'une même grille partagent leur `code` et ne se chevauchent pas. Elle s'applique à
//! des **cibles** (un site, un niveau, les deux, des jours, ou tout l'établissement) ; pour
//! (date, site, niveau, jour) la cible la plus précise gagne (`ent_grille_pour`, une seule
//! implémentation, lue par le port [`structure`]).
//!
//! « Sans casser l'existant » :
//! * on ne modifie jamais le passé : une version commencée est **gelée par la base** ; on crée une
//!   nouvelle version à une date (la version en cours est close la veille) ;
//! * l'ancienne table plate `ent_grille_horaire` (migration 110) n'est pas modifiée : elle est
//!   reprise à la migration, **miroitée** quand l'ancienne route l'écrit, et **projetée** depuis la
//!   version en vigueur quand le service l'écrit, tant que la génération des séances (socle 10) la
//!   lit encore ;
//! * la modification d'une version future ne touche que l'avenir ; l'événement
//!   `referentiel.grille_horaire_definie` porte `a_partir_du` pour que le socle 10 régénère les
//!   séances futures.
//!
//! Droits : lecture `referentiel.lire`, écriture `referentiel.gerer` (direction, administratif,
//! administrateur ; l'action `etablissement.parametrer` du §13.2 n'existe pas encore au catalogue).
//! **La coupure des demi-journées (STR-35) est réservée à la direction et à l'administrateur.**

use chrono::{Duration, NaiveDate, NaiveTime, Timelike};
use ent_auth::{Action, Role};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::{journaliser, net, publier, requis};
use crate::tenant_tx::TenantTx;

pub const NATURES: [&str; 4] = ["cours", "recreation", "pause_meridienne", "etude"];
const MAX_CRENEAUX: usize = 20;

async fn tx(e: &PgEnt, a: &Acteur<'_>) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(e.pool(), a.etablissement_id)
        .await
        .map_err(db)
}

fn nature_cours() -> String {
    "cours".to_string()
}

fn est_direction(a: &Acteur<'_>) -> bool {
    matches!(Role::parse(a.role), Some(Role::Direction | Role::Admin))
}

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreneauS7Saisie {
    pub code: String,
    pub libelle: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default = "nature_cours")]
    pub nature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize)]
pub struct CreneauVue {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub nature: String,
    pub ordre: i16,
}

/// Cible d'une grille. Tout à vide = tout l'établissement, tous les jours.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CibleGrilleSaisie {
    #[serde(default)]
    pub site_id: Option<Uuid>,
    #[serde(default)]
    pub niveau_id: Option<Uuid>,
    /// Jours ISO (1 = lundi … 7 = dimanche) ; absent = tous les jours.
    #[serde(default)]
    pub jours: Option<Vec<i16>>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize)]
pub struct CibleGrilleVue {
    pub id: Uuid,
    pub site_id: Option<Uuid>,
    pub niveau_id: Option<Uuid>,
    pub jours: Option<Vec<i16>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrilleSaisie {
    /// Code de la grille (commun à toutes ses versions).
    pub code: String,
    pub libelle: String,
    /// Date d'effet : aujourd'hui ou plus tard (le passé ne se réécrit pas).
    pub valable_du: NaiveDate,
    /// Absente : milieu de la pause méridienne, à défaut 12 h 30.
    #[serde(default)]
    pub coupure_demi_journee: Option<NaiveTime>,
    #[serde(default)]
    pub motif: Option<String>,
    pub creneaux: Vec<CreneauS7Saisie>,
    /// Absentes : toute l'établissement.
    #[serde(default)]
    pub cibles: Vec<CibleGrilleSaisie>,
}

/// Modification d'une version qui n'a pas encore commencé.
#[derive(Debug, Clone, Deserialize)]
pub struct GrilleModif {
    pub libelle: String,
    #[serde(default)]
    pub coupure_demi_journee: Option<NaiveTime>,
    #[serde(default)]
    pub motif: Option<String>,
    pub creneaux: Vec<CreneauS7Saisie>,
    #[serde(default)]
    pub cibles: Vec<CibleGrilleSaisie>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrilleVue {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub valable_du: NaiveDate,
    pub valable_au: Option<NaiveDate>,
    pub coupure_demi_journee: NaiveTime,
    pub motif: Option<String>,
    pub origine: String,
    /// La version a commencé : elle est gelée.
    pub commencee: bool,
    pub cibles: Vec<CibleGrilleVue>,
    pub creneaux: Vec<CreneauVue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DemiJournee {
    Matin,
    ApresMidi,
}

// ───────────────────────── Calculs purs ─────────────────────────

fn minute(t: NaiveTime) -> u32 {
    t.hour() * 60 + t.minute()
}

/// Coupure par défaut : milieu de la pause méridienne s'il y en a une, sinon 12 h 30.
pub fn coupure_par_defaut(creneaux: &[CreneauS7Saisie]) -> NaiveTime {
    creneaux
        .iter()
        .find(|c| c.nature == "pause_meridienne")
        .map(|c| {
            let m = (minute(c.heure_debut) + minute(c.heure_fin)) / 2;
            NaiveTime::from_hms_opt(m / 60, m % 60, 0).unwrap_or_default()
        })
        .unwrap_or_else(|| NaiveTime::from_hms_opt(12, 30, 0).unwrap_or_default())
}

/// Valide et normalise les créneaux : au moins un cours, codes uniques (majuscules), heures à la
/// minute, pas de chevauchement, coupure jamais au milieu d'un cours. Renvoie les créneaux triés
/// et la coupure effective.
pub fn valider_creneaux(
    creneaux: &[CreneauS7Saisie],
    coupure: Option<NaiveTime>,
) -> Result<(Vec<CreneauS7Saisie>, NaiveTime), EntDbError> {
    if creneaux.is_empty() || creneaux.len() > MAX_CRENEAUX {
        return Err(invalide());
    }
    let mut v: Vec<CreneauS7Saisie> = Vec::with_capacity(creneaux.len());
    let mut codes = std::collections::HashSet::new();
    for c in creneaux {
        let code = requis(&c.code, 20)?.to_uppercase();
        if !code
            .chars()
            .all(|x| x.is_alphanumeric() || matches!(x, '_' | '-' | '.'))
            || !codes.insert(code.clone())
            || !NATURES.contains(&c.nature.as_str())
            || c.heure_fin <= c.heure_debut
            || c.heure_debut.second() != 0
            || c.heure_fin.second() != 0
            || c.heure_debut.nanosecond() != 0
            || c.heure_fin.nanosecond() != 0
        {
            return Err(invalide());
        }
        v.push(CreneauS7Saisie {
            code,
            libelle: requis(&c.libelle, 60)?,
            ..c.clone()
        });
    }
    v.sort_by_key(|c| c.heure_debut);
    if v.windows(2).any(|w| w[1].heure_debut < w[0].heure_fin) || !v.iter().any(|c| c.nature == "cours")
    {
        return Err(invalide());
    }
    let coupure = coupure.unwrap_or_else(|| coupure_par_defaut(&v));
    if coupure.second() != 0
        || coupure.nanosecond() != 0
        || v.iter()
            .any(|c| c.nature == "cours" && c.heure_debut < coupure && coupure < c.heure_fin)
    {
        return Err(invalide());
    }
    Ok((v, coupure))
}

/// Matin ou après-midi selon la coupure (l'instant de la coupure est déjà l'après-midi).
pub fn demi_journee_de(coupure: NaiveTime, heure: NaiveTime) -> DemiJournee {
    if heure < coupure {
        DemiJournee::Matin
    } else {
        DemiJournee::ApresMidi
    }
}

/// Cibles normalisées : par défaut tout l'établissement ; jours triés, uniques, de 1 à 7.
pub fn valider_cibles(cibles: &[CibleGrilleSaisie]) -> Result<Vec<CibleGrilleSaisie>, EntDbError> {
    if cibles.len() > 50 {
        return Err(invalide());
    }
    if cibles.is_empty() {
        return Ok(vec![CibleGrilleSaisie::default()]);
    }
    let mut out: Vec<CibleGrilleSaisie> = Vec::with_capacity(cibles.len());
    for c in cibles {
        let jours = match &c.jours {
            None => None,
            Some(j) => {
                let mut j = j.clone();
                j.sort_unstable();
                j.dedup();
                if j.is_empty() || j.iter().any(|d| !(1..=7).contains(d)) {
                    return Err(invalide());
                }
                if j.len() == 7 {
                    None
                } else {
                    Some(j)
                }
            }
        };
        let n = CibleGrilleSaisie {
            site_id: c.site_id,
            niveau_id: c.niveau_id,
            jours,
        };
        if out.iter().any(|o| o.site_id == n.site_id && o.niveau_id == n.niveau_id) {
            return Err(invalide());
        }
        out.push(n);
    }
    Ok(out)
}

// ───────────────────────── Lectures partagées ─────────────────────────

#[derive(FromRow)]
struct LigneGrille {
    id: Uuid,
    code: String,
    libelle: String,
    valable_du: NaiveDate,
    valable_au: Option<NaiveDate>,
    coupure_demi_journee: NaiveTime,
    motif: Option<String>,
    origine: String,
}

const SQL_GRILLE: &str = "SELECT id, code, libelle, valable_du, valable_au, coupure_demi_journee, motif, origine
                            FROM ent_grille";

async fn creneaux_de(
    conn: &mut PgConnection,
    etab: Uuid,
    grille: Uuid,
) -> Result<Vec<CreneauVue>, EntDbError> {
    sqlx::query_as::<_, CreneauVue>(
        "SELECT id, code, libelle, heure_debut, heure_fin, nature, ordre FROM ent_grille_creneau
          WHERE etablissement_id = $1 AND grille_id = $2 ORDER BY heure_debut",
    )
    .bind(etab)
    .bind(grille)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn cibles_de(
    conn: &mut PgConnection,
    etab: Uuid,
    grille: Uuid,
) -> Result<Vec<CibleGrilleVue>, EntDbError> {
    sqlx::query_as::<_, CibleGrilleVue>(
        "SELECT id, site_id, niveau_id, jours FROM ent_grille_cible
          WHERE etablissement_id = $1 AND grille_id = $2
          ORDER BY site_id NULLS FIRST, niveau_id NULLS FIRST",
    )
    .bind(etab)
    .bind(grille)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn vue_de(
    conn: &mut PgConnection,
    etab: Uuid,
    l: LigneGrille,
    aujourdhui: NaiveDate,
) -> Result<GrilleVue, EntDbError> {
    let creneaux = creneaux_de(conn, etab, l.id).await?;
    let cibles = cibles_de(conn, etab, l.id).await?;
    Ok(GrilleVue {
        id: l.id,
        code: l.code,
        libelle: l.libelle,
        valable_du: l.valable_du,
        valable_au: l.valable_au,
        coupure_demi_journee: l.coupure_demi_journee,
        motif: l.motif,
        origine: l.origine,
        commencee: l.valable_du < aujourdhui,
        cibles,
        creneaux,
    })
}

async fn grille_lue(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<LigneGrille, EntDbError> {
    sqlx::query_as::<_, LigneGrille>(&format!(
        "{SQL_GRILLE} WHERE etablissement_id = $1 AND id = $2 AND archivee_le IS NULL{}",
        if verrou { " FOR UPDATE" } else { "" }
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

/// Sérialise les écritures de grilles d'un établissement (versions voisines, cibles).
async fn verrou_etab(conn: &mut PgConnection, etab: Uuid) -> Result<(), EntDbError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('ent_grille:' || $1::text, 0))")
        .bind(etab)
        .execute(conn)
        .await
        .map_err(db)?;
    Ok(())
}

async fn cibles_existent(
    conn: &mut PgConnection,
    etab: Uuid,
    cibles: &[CibleGrilleSaisie],
) -> Result<(), EntDbError> {
    for c in cibles {
        if let Some(s) = c.site_id {
            let n: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_site WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(s)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if n == 0 {
                return Err(introuvable());
            }
        }
        if let Some(nv) = c.niveau_id {
            let n: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_niveau WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(nv)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if n == 0 {
                return Err(introuvable());
            }
        }
    }
    Ok(())
}

/// Deux grilles de codes différents ne visent jamais le même (site, niveau) sur les mêmes jours
/// pendant la même période : ce serait deux réponses à « quelle grille ce jour-là ? ». Une cible
/// qui précise des jours (« le mercredi ») peut coexister avec une cible sans jours : elle l'emporte.
async fn sans_conflit_de_cible(
    conn: &mut PgConnection,
    etab: Uuid,
    grille: Uuid,
) -> Result<(), EntDbError> {
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_grille g1
           JOIN ent_grille_cible c1 ON c1.grille_id = g1.id
           JOIN ent_grille g2 ON g2.etablissement_id = g1.etablissement_id AND g2.id <> g1.id
                             AND g2.code <> g1.code AND g2.archivee_le IS NULL
                             AND daterange(g2.valable_du, g2.valable_au, '[]')
                                 && daterange(g1.valable_du, g1.valable_au, '[]')
           JOIN ent_grille_cible c2 ON c2.grille_id = g2.id
                                   AND c2.site_id IS NOT DISTINCT FROM c1.site_id
                                   AND c2.niveau_id IS NOT DISTINCT FROM c1.niveau_id
                                   AND ((c1.jours IS NULL AND c2.jours IS NULL) OR c1.jours && c2.jours)
          WHERE g1.etablissement_id = $1 AND g1.id = $2",
    )
    .bind(etab)
    .bind(grille)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    if n > 0 {
        Err(conflit())
    } else {
        Ok(())
    }
}

async fn poser_enfants(
    conn: &mut PgConnection,
    etab: Uuid,
    grille: Uuid,
    creneaux: &[CreneauS7Saisie],
    cibles: &[CibleGrilleSaisie],
) -> Result<(), EntDbError> {
    for c in cibles {
        sqlx::query(
            "INSERT INTO ent_grille_cible (id, etablissement_id, grille_id, site_id, niveau_id, jours)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(grille)
        .bind(c.site_id)
        .bind(c.niveau_id)
        .bind(&c.jours)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    for (i, c) in creneaux.iter().enumerate() {
        sqlx::query(
            "INSERT INTO ent_grille_creneau
               (id, etablissement_id, grille_id, code, libelle, heure_debut, heure_fin, nature, ordre)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(grille)
        .bind(&c.code)
        .bind(&c.libelle)
        .bind(c.heure_debut)
        .bind(c.heure_fin)
        .bind(&c.nature)
        .bind((i + 1) as i16)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

struct Ecriture<'a> {
    code: &'a str,
    libelle: &'a str,
    valable_du: NaiveDate,
    coupure: NaiveTime,
    motif: Option<&'a str>,
    origine: &'a str,
    creneaux: &'a [CreneauS7Saisie],
    cibles: &'a [CibleGrilleSaisie],
}

/// Crée la version `valable_du` d'un code : close la version en cours la veille, s'arrête avant la
/// version suivante s'il y en a une. Appelée avec le verrou d'établissement posé.
async fn ecrire_version(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    w: Ecriture<'_>,
    aujourdhui: NaiveDate,
) -> Result<Uuid, EntDbError> {
    let e = a.etablissement_id;
    if w.valable_du < aujourdhui {
        return Err(invalide()); // le passé ne se réécrit pas
    }
    let versions: Vec<(Uuid, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT id, valable_du, valable_au FROM ent_grille
          WHERE etablissement_id = $1 AND code = $2 AND archivee_le IS NULL ORDER BY valable_du",
    )
    .bind(e)
    .bind(w.code)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    if versions.iter().any(|v| v.1 == w.valable_du) {
        return Err(conflit()); // une version commence déjà ce jour-là : la remplacer
    }
    let precedente = versions
        .iter()
        .rev()
        .find(|v| v.1 < w.valable_du)
        .filter(|v| v.2.map_or(true, |f| f >= w.valable_du));
    let suivante = versions.iter().find(|v| v.1 > w.valable_du);
    let valable_au = match (suivante, precedente) {
        (Some(s), _) => Some(s.1 - Duration::days(1)),
        (None, Some(p)) => p.2, // la nouvelle version reprend la borne de celle qu'elle relaie
        (None, None) => None,
    };
    if let Some(p) = precedente {
        sqlx::query("UPDATE ent_grille SET valable_au = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(e)
            .bind(p.0)
            .bind(w.valable_du - Duration::days(1))
            .execute(&mut *conn)
            .await
            .map_err(db)?;
    }
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_grille (id, etablissement_id, code, libelle, valable_du, valable_au,
                                 coupure_demi_journee, motif, origine, cree_par)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(id)
    .bind(e)
    .bind(w.code)
    .bind(w.libelle)
    .bind(w.valable_du)
    .bind(valable_au)
    .bind(w.coupure)
    .bind(w.motif)
    .bind(w.origine)
    .bind(a.user_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    poser_enfants(conn, e, id, w.creneaux, w.cibles).await?;
    sans_conflit_de_cible(conn, e, id).await?;
    Ok(id)
}

/// Projette une version en vigueur aujourd'hui vers l'ancienne table plate, pour chaque cible
/// « site seul, tous les jours » : la génération des séances (socle 10) la lit encore.
async fn projeter_ancienne(
    conn: &mut PgConnection,
    etab: Uuid,
    grille: Uuid,
    aujourdhui: NaiveDate,
) -> Result<(), EntDbError> {
    let en_vigueur: Option<NaiveTime> = sqlx::query_scalar(
        "SELECT coupure_demi_journee FROM ent_grille
          WHERE etablissement_id = $1 AND id = $2 AND archivee_le IS NULL
            AND valable_du <= $3 AND (valable_au IS NULL OR valable_au >= $3)",
    )
    .bind(etab)
    .bind(grille)
    .bind(aujourdhui)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some(coupure) = en_vigueur else {
        return Ok(()); // future ou close : rien à projeter aujourd'hui
    };
    let cibles: Vec<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT site_id FROM ent_grille_cible
          WHERE etablissement_id = $1 AND grille_id = $2 AND niveau_id IS NULL AND jours IS NULL",
    )
    .bind(etab)
    .bind(grille)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let creneaux = creneaux_de(conn, etab, grille).await?;
    for (site,) in cibles {
        sqlx::query(
            "DELETE FROM ent_grille_horaire WHERE etablissement_id = $1 AND site_id IS NOT DISTINCT FROM $2",
        )
        .bind(etab)
        .bind(site)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        for (i, c) in creneaux.iter().filter(|c| c.nature == "cours").enumerate() {
            let demi = match demi_journee_de(coupure, c.heure_debut) {
                DemiJournee::Matin => "matin",
                DemiJournee::ApresMidi => "apres_midi",
            };
            sqlx::query(
                "INSERT INTO ent_grille_horaire
                   (id, etablissement_id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(site)
            .bind(&c.code)
            .bind(&c.libelle)
            .bind(demi)
            .bind((i + 1) as i16)
            .bind(c.heure_debut)
            .bind(c.heure_fin)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
    }
    Ok(())
}

fn code_reprise(site: Option<Uuid>) -> String {
    format!(
        "REPRISE-{}",
        &site.map_or_else(|| "ETAB".to_string(), |s| s.simple().to_string())[..site.map_or(4, |_| 12)]
    )
}

/// Miroir de l'ancienne route `ref_definir_grille` dans le nouveau modèle : une version
/// « REPRISE-<site> » valable aujourd'hui. Appelée par l'ancien écrivain, qui ignore l'échec du
/// miroir (l'ancienne route ne doit jamais casser).
pub(crate) async fn miroir_ancienne_grille(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    site_id: Option<Uuid>,
    creneaux: &[crate::referentiel_structure::CreneauGrilleSaisie],
) -> Result<(), EntDbError> {
    let e = a.etablissement_id;
    verrou_etab(conn, e).await?;
    let aujourdhui = crate::structure_etab::structure::aujourdhui(conn, e).await?;
    let code = code_reprise(site_id);
    let neufs: Vec<CreneauS7Saisie> = creneaux
        .iter()
        .map(|c| CreneauS7Saisie {
            code: c.code.clone(),
            libelle: c.libelle.clone(),
            heure_debut: c.heure_debut,
            heure_fin: c.heure_fin,
            nature: "cours".to_string(),
        })
        .collect();
    // Version qui commence aujourd'hui : on la remplace ; sinon on versionne.
    let du_jour: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_grille WHERE etablissement_id = $1 AND code = $2 AND valable_du = $3 AND archivee_le IS NULL",
    )
    .bind(e)
    .bind(&code)
    .bind(aujourdhui)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some(id) = du_jour {
        sqlx::query("DELETE FROM ent_grille_creneau WHERE etablissement_id = $1 AND grille_id = $2")
            .bind(e)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        if neufs.is_empty() {
            sqlx::query("DELETE FROM ent_grille WHERE etablissement_id = $1 AND id = $2")
                .bind(e)
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            return Ok(());
        }
        let (v, coupure) = valider_creneaux(&neufs, None)?;
        sqlx::query("UPDATE ent_grille SET coupure_demi_journee = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(e)
            .bind(id)
            .bind(coupure)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        poser_enfants(conn, e, id, &v, &[]).await?;
        return Ok(());
    }
    if neufs.is_empty() {
        // Grille vidée : la version en cours se termine hier.
        sqlx::query(
            "UPDATE ent_grille SET valable_au = $3 - 1
              WHERE etablissement_id = $1 AND code = $2 AND archivee_le IS NULL
                AND valable_du < $3 AND (valable_au IS NULL OR valable_au >= $3)",
        )
        .bind(e)
        .bind(&code)
        .bind(aujourdhui)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        return Ok(());
    }
    let (v, coupure) = valider_creneaux(&neufs, None)?;
    let cibles = [CibleGrilleSaisie {
        site_id,
        niveau_id: None,
        jours: None,
    }];
    ecrire_version(
        conn,
        a,
        Ecriture {
            code: &code,
            libelle: "Grille horaire (reprise)",
            valable_du: aujourdhui,
            coupure,
            motif: Some("Mise à jour par l'ancienne route de la grille"),
            origine: "reprise",
            creneaux: &v,
            cibles: &cibles,
        },
        aujourdhui,
    )
    .await?;
    Ok(())
}

fn json_grille(g: &GrilleVue) -> Value {
    json!({
        "code": g.code, "valable_du": g.valable_du, "valable_au": g.valable_au,
        "coupure": g.coupure_demi_journee, "creneaux": g.creneaux.len(), "cibles": g.cibles.len(),
    })
}

// ───────────────────────── Service ─────────────────────────

impl PgEnt {
    /// Toutes les versions de grille (la plus récente d'abord pour un même code).
    pub async fn structure_grilles(&self, a: &Acteur<'_>) -> Result<Vec<GrilleVue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let e = a.etablissement_id;
        let mut t = tx(self, a).await?;
        let jour = crate::structure_etab::structure::aujourdhui(&mut t, e).await?;
        let lignes = sqlx::query_as::<_, LigneGrille>(&format!(
            "{SQL_GRILLE} WHERE etablissement_id = $1 AND archivee_le IS NULL ORDER BY code, valable_du DESC"
        ))
        .bind(e)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for l in lignes {
            out.push(vue_de(&mut t, e, l, jour).await?);
        }
        t.commit().await.map_err(db)?;
        Ok(out)
    }

    pub async fn structure_grille(&self, a: &Acteur<'_>, id: Uuid) -> Result<GrilleVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let e = a.etablissement_id;
        let mut t = tx(self, a).await?;
        let jour = crate::structure_etab::structure::aujourdhui(&mut t, e).await?;
        let l = grille_lue(&mut t, e, id, false).await?;
        let v = vue_de(&mut t, e, l, jour).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// STR-34 : crée une nouvelle **version** de la grille `code` à la date `valable_du`
    /// (aujourd'hui ou plus tard). La version en cours est close la veille ; ses séances passées
    /// ne bougent pas.
    pub async fn structure_grille_versionner(
        &self,
        a: &Acteur<'_>,
        s: GrilleSaisie,
    ) -> Result<GrilleVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let code = requis(&s.code, 40)?.to_uppercase();
        if !code
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
        {
            return Err(invalide());
        }
        let libelle = requis(&s.libelle, 120)?;
        let motif = net(s.motif.as_deref(), 300)?;
        let (creneaux, coupure) = valider_creneaux(&s.creneaux, s.coupure_demi_journee)?;
        let cibles = valider_cibles(&s.cibles)?;
        let mut t = tx(self, a).await?;
        verrou_etab(&mut t, e).await?;
        let jour = crate::structure_etab::structure::aujourdhui(&mut t, e).await?;
        cibles_existent(&mut t, e, &cibles).await?;
        let id = ecrire_version(
            &mut t,
            a,
            Ecriture {
                code: &code,
                libelle: &libelle,
                valable_du: s.valable_du,
                coupure,
                motif: motif.as_deref(),
                origine: "service",
                creneaux: &creneaux,
                cibles: &cibles,
            },
            jour,
        )
        .await?;
        projeter_ancienne(&mut t, e, id, jour).await?;
        let l = grille_lue(&mut t, e, id, false).await?;
        let v = vue_de(&mut t, e, l, jour).await?;
        journaliser(&mut t, a, "grille", &id.to_string(), "versionnee", None, Some(json_grille(&v))).await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.grille_horaire_definie",
            id,
            json!({ "code": v.code, "a_partir_du": v.valable_du, "creneaux": v.creneaux.len() }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// STR-34 : modifie une version qui n'a **pas encore commencé** (créneaux, cibles, coupure).
    /// Une version commencée est gelée : 409, il faut en créer une nouvelle.
    pub async fn structure_grille_remplacer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        m: GrilleModif,
    ) -> Result<GrilleVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let libelle = requis(&m.libelle, 120)?;
        let motif = net(m.motif.as_deref(), 300)?;
        let (creneaux, coupure) = valider_creneaux(&m.creneaux, m.coupure_demi_journee)?;
        let cibles = valider_cibles(&m.cibles)?;
        let mut t = tx(self, a).await?;
        verrou_etab(&mut t, e).await?;
        let jour = crate::structure_etab::structure::aujourdhui(&mut t, e).await?;
        let avant = grille_lue(&mut t, e, id, true).await?;
        if avant.valable_du < jour {
            return Err(conflit());
        }
        cibles_existent(&mut t, e, &cibles).await?;
        let ligne = grille_lue(&mut t, e, id, false).await?;
        let avant_vue = vue_de(&mut t, e, ligne, jour).await?;
        sqlx::query("DELETE FROM ent_grille_creneau WHERE etablissement_id = $1 AND grille_id = $2")
            .bind(e)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        sqlx::query("DELETE FROM ent_grille_cible WHERE etablissement_id = $1 AND grille_id = $2")
            .bind(e)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        sqlx::query(
            "UPDATE ent_grille SET libelle = $3, coupure_demi_journee = $4, motif = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(id)
        .bind(&libelle)
        .bind(coupure)
        .bind(motif.as_deref())
        .execute(&mut *t)
        .await
        .map_err(db)?;
        poser_enfants(&mut t, e, id, &creneaux, &cibles).await?;
        sans_conflit_de_cible(&mut t, e, id).await?;
        projeter_ancienne(&mut t, e, id, jour).await?;
        let l = grille_lue(&mut t, e, id, false).await?;
        let v = vue_de(&mut t, e, l, jour).await?;
        journaliser(
            &mut t,
            a,
            "grille",
            &id.to_string(),
            "remplacee",
            Some(json_grille(&avant_vue)),
            Some(json_grille(&v)),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.grille_horaire_definie",
            id,
            json!({ "code": v.code, "a_partir_du": v.valable_du, "creneaux": v.creneaux.len() }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// STR-34 : supprime une version qui n'a pas commencé ; la version précédente reprend sa date
    /// de fin si elle avait été close pour lui céder la place.
    pub async fn structure_grille_supprimer(&self, a: &Acteur<'_>, id: Uuid) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut t = tx(self, a).await?;
        verrou_etab(&mut t, e).await?;
        let jour = crate::structure_etab::structure::aujourdhui(&mut t, e).await?;
        let g = grille_lue(&mut t, e, id, true).await?;
        if g.valable_du < jour {
            return Err(conflit());
        }
        let ligne = grille_lue(&mut t, e, id, false).await?;
        let avant = vue_de(&mut t, e, ligne, jour).await?;
        sqlx::query("DELETE FROM ent_grille WHERE etablissement_id = $1 AND id = $2")
            .bind(e)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        // La version qui s'arrêtait la veille reprend la fin de celle qui disparaît.
        sqlx::query(
            "UPDATE ent_grille SET valable_au = $4
              WHERE etablissement_id = $1 AND code = $2 AND archivee_le IS NULL AND valable_au = $3 - 1",
        )
        .bind(e)
        .bind(&g.code)
        .bind(g.valable_du)
        .bind(g.valable_au)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(&mut t, a, "grille", &id.to_string(), "supprimee", Some(json_grille(&avant)), None).await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.grille_horaire_definie",
            id,
            json!({ "code": g.code, "a_partir_du": g.valable_du, "supprimee": true }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    /// STR-35 : change la coupure des demi-journées de la grille `code` à partir de `a_partir_du`
    /// (aujourd'hui ou plus tard). **Direction et administrateur seulement.** Si la version en
    /// vigueur commence ce jour-là, elle est corrigée sur place ; sinon une nouvelle version est
    /// créée (mêmes créneaux et cibles), l'historique du décompte reste inchangé.
    pub async fn structure_grille_coupure(
        &self,
        a: &Acteur<'_>,
        code: &str,
        a_partir_du: NaiveDate,
        coupure: NaiveTime,
        motif: Option<&str>,
    ) -> Result<GrilleVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !est_direction(a) {
            return Err(EntDbError::Forbidden);
        }
        let e = a.etablissement_id;
        let code = requis(code, 40)?.to_uppercase();
        let motif = net(motif, 300)?;
        let mut t = tx(self, a).await?;
        verrou_etab(&mut t, e).await?;
        let jour = crate::structure_etab::structure::aujourdhui(&mut t, e).await?;
        if a_partir_du < jour {
            return Err(invalide());
        }
        let en_vigueur = sqlx::query_as::<_, LigneGrille>(&format!(
            "{SQL_GRILLE} WHERE etablissement_id = $1 AND code = $2 AND archivee_le IS NULL
                AND valable_du <= $3 AND (valable_au IS NULL OR valable_au >= $3) FOR UPDATE"
        ))
        .bind(e)
        .bind(&code)
        .bind(a_partir_du)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let ligne = grille_lue(&mut t, e, en_vigueur.id, false).await?;
        let avant = vue_de(&mut t, e, ligne, jour).await?;
        let saisis: Vec<CreneauS7Saisie> = avant
            .creneaux
            .iter()
            .map(|c| CreneauS7Saisie {
                code: c.code.clone(),
                libelle: c.libelle.clone(),
                heure_debut: c.heure_debut,
                heure_fin: c.heure_fin,
                nature: c.nature.clone(),
            })
            .collect();
        let (_, coupure) = valider_creneaux(&saisis, Some(coupure))?;
        let id = if en_vigueur.valable_du == a_partir_du {
            sqlx::query("UPDATE ent_grille SET coupure_demi_journee = $3 WHERE etablissement_id = $1 AND id = $2")
                .bind(e)
                .bind(en_vigueur.id)
                .bind(coupure)
                .execute(&mut *t)
                .await
                .map_err(db)?;
            en_vigueur.id
        } else {
            let cibles: Vec<CibleGrilleSaisie> = avant
                .cibles
                .iter()
                .map(|c| CibleGrilleSaisie {
                    site_id: c.site_id,
                    niveau_id: c.niveau_id,
                    jours: c.jours.clone(),
                })
                .collect();
            ecrire_version(
                &mut t,
                a,
                Ecriture {
                    code: &code,
                    libelle: &avant.libelle,
                    valable_du: a_partir_du,
                    coupure,
                    motif: motif.as_deref().or(Some("Changement de la coupure des demi-journées")),
                    origine: "service",
                    creneaux: &saisis,
                    cibles: &cibles,
                },
                jour,
            )
            .await?
        };
        projeter_ancienne(&mut t, e, id, jour).await?;
        let l = grille_lue(&mut t, e, id, false).await?;
        let v = vue_de(&mut t, e, l, jour).await?;
        journaliser(
            &mut t,
            a,
            "grille",
            &id.to_string(),
            "coupure_definie",
            Some(json!({ "coupure": avant.coupure_demi_journee })),
            Some(json!({ "coupure": v.coupure_demi_journee, "a_partir_du": a_partir_du })),
        )
        .await?;
        // Le socle 12 (décompte en demi-journées) relit la coupure par le port.
        let ev = publier(
            &mut t,
            a,
            "referentiel.grille_horaire_definie",
            id,
            json!({ "code": v.code, "a_partir_du": a_partir_du, "coupure": v.coupure_demi_journee }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// Grille en vigueur pour (jour, site, niveau), par le port [`structure`].
    pub async fn structure_grille_a_la_date(
        &self,
        a: &Acteur<'_>,
        jour: NaiveDate,
        site_id: Option<Uuid>,
        niveau_id: Option<Uuid>,
    ) -> Result<Option<structure::GrilleResolue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let g = structure::grille(&mut t, a.etablissement_id, jour, site_id, niveau_id).await?;
        t.commit().await.map_err(db)?;
        Ok(g)
    }
}

// ───────────────────────── Port lu par les autres socles ─────────────────────────

/// Port `structure::grille(date, site, niveau)` du §6.3 : lecture seule, connexion de l'appelant
/// (qui a déjà autorisé). **Lu par le socle 10 (séances) et le socle 12 (décompte en
/// demi-journées).** Aucune création implicite.
pub mod structure {
    use super::*;

    #[derive(Debug, Clone, Serialize)]
    pub struct GrilleResolue {
        pub id: Uuid,
        pub code: String,
        pub libelle: String,
        pub valable_du: NaiveDate,
        pub valable_au: Option<NaiveDate>,
        pub coupure_demi_journee: NaiveTime,
        pub creneaux: Vec<CreneauVue>,
    }

    impl GrilleResolue {
        /// Matin ou après-midi pour une heure murale de cette grille.
        pub fn demi_journee(&self, heure: NaiveTime) -> DemiJournee {
            demi_journee_de(self.coupure_demi_journee, heure)
        }

        /// Créneau (cours, récréation…) qui contient `heure` (début inclus, fin exclue).
        pub fn creneau_a(&self, heure: NaiveTime) -> Option<&CreneauVue> {
            self.creneaux
                .iter()
                .find(|c| c.heure_debut <= heure && heure < c.heure_fin)
        }
    }

    /// Grille en vigueur ce jour-là pour ce site et ce niveau (cible la plus précise).
    pub async fn grille(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        site_id: Option<Uuid>,
        niveau_id: Option<Uuid>,
    ) -> Result<Option<GrilleResolue>, EntDbError> {
        let id: Option<Uuid> = sqlx::query_scalar("SELECT ent_grille_pour($1, $2, $3, $4)")
            .bind(etab)
            .bind(jour)
            .bind(site_id)
            .bind(niveau_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
        let Some(id) = id else { return Ok(None) };
        let l = grille_lue(conn, etab, id, false).await?;
        let creneaux = creneaux_de(conn, etab, id).await?;
        Ok(Some(GrilleResolue {
            id: l.id,
            code: l.code,
            libelle: l.libelle,
            valable_du: l.valable_du,
            valable_au: l.valable_au,
            coupure_demi_journee: l.coupure_demi_journee,
            creneaux,
        }))
    }

    /// Grille en vigueur pour une classe : son site, son niveau (ou celui de son MEF).
    pub async fn grille_de_classe(
        conn: &mut PgConnection,
        etab: Uuid,
        classe_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<GrilleResolue>, EntDbError> {
        let c: Option<(Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT c.site_id, COALESCE(c.niveau_id, m.niveau_id)
               FROM ent_classe c LEFT JOIN ent_mef m ON m.id = c.mef_id AND m.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = $2",
        )
        .bind(etab)
        .bind(classe_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let (site, niveau) = c.ok_or_else(introuvable)?;
        grille(conn, etab, jour, site, niveau).await
    }

    /// Coupure des demi-journées en vigueur ce jour-là (`None` : aucune grille ne s'applique).
    pub async fn coupure(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        site_id: Option<Uuid>,
        niveau_id: Option<Uuid>,
    ) -> Result<Option<NaiveTime>, EntDbError> {
        Ok(grille(conn, etab, jour, site_id, niveau_id)
            .await?
            .map(|g| g.coupure_demi_journee))
    }

    /// Demi-journée d'une heure ce jour-là (`None` : aucune grille ne s'applique).
    pub async fn demi_journee(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        site_id: Option<Uuid>,
        niveau_id: Option<Uuid>,
        heure: NaiveTime,
    ) -> Result<Option<DemiJournee>, EntDbError> {
        Ok(coupure(conn, etab, jour, site_id, niveau_id)
            .await?
            .map(|c| demi_journee_de(c, heure)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn cr(code: &str, d: NaiveTime, f: NaiveTime, nature: &str) -> CreneauS7Saisie {
        CreneauS7Saisie {
            code: code.into(),
            libelle: code.into(),
            heure_debut: d,
            heure_fin: f,
            nature: nature.into(),
        }
    }

    fn journee() -> Vec<CreneauS7Saisie> {
        vec![
            cr("M1", h(8, 0), h(8, 55), "cours"),
            cr("M2", h(8, 55), h(9, 50), "cours"),
            cr("R1", h(9, 50), h(10, 5), "recreation"),
            cr("M3", h(10, 5), h(11, 0), "cours"),
            cr("M4", h(11, 0), h(11, 55), "cours"),
            cr("PM", h(11, 55), h(13, 5), "pause_meridienne"),
            cr("S1", h(13, 5), h(14, 0), "cours"),
            cr("S2", h(14, 0), h(14, 55), "cours"),
        ]
    }

    #[test]
    fn journee_type_valide_et_coupure_au_milieu_de_la_pause() {
        let (v, c) = valider_creneaux(&journee(), None).unwrap();
        assert_eq!(v.len(), 8);
        assert_eq!(c, h(12, 30));
        assert_eq!(demi_journee_de(c, h(11, 0)), DemiJournee::Matin);
        assert_eq!(demi_journee_de(c, h(12, 30)), DemiJournee::ApresMidi);
        assert_eq!(demi_journee_de(c, h(13, 5)), DemiJournee::ApresMidi);
    }

    #[test]
    fn creneaux_invalides_refuses() {
        // Chevauchement.
        let mut j = journee();
        j[1].heure_debut = h(8, 50);
        assert!(valider_creneaux(&j, None).is_err());
        // Code en double (casse ignorée).
        let mut j = journee();
        j[1].code = "m1".into();
        assert!(valider_creneaux(&j, None).is_err());
        // Nature inconnue.
        let mut j = journee();
        j[0].nature = "sieste".into();
        assert!(valider_creneaux(&j, None).is_err());
        // Aucun cours.
        let j = vec![cr("R", h(10, 0), h(10, 15), "recreation")];
        assert!(valider_creneaux(&j, None).is_err());
        // Vide.
        assert!(valider_creneaux(&[], None).is_err());
        // Secondes non gérées (granularité : la minute).
        let mut j = journee();
        j[0].heure_debut = NaiveTime::from_hms_opt(8, 0, 30).unwrap();
        assert!(valider_creneaux(&j, None).is_err());
    }

    #[test]
    fn coupure_jamais_au_milieu_d_un_cours() {
        assert!(valider_creneaux(&journee(), Some(h(11, 30))).is_err()); // dans M4
        assert!(valider_creneaux(&journee(), Some(h(12, 0))).is_ok()); // dans la pause
        assert!(valider_creneaux(&journee(), Some(h(11, 55))).is_ok()); // à la frontière
        assert!(valider_creneaux(&journee(), Some(h(12, 0))).unwrap().1 == h(12, 0));
    }

    #[test]
    fn sans_pause_la_coupure_par_defaut_est_midi_et_demi() {
        let j = vec![cr("A", h(8, 0), h(9, 0), "cours"), cr("B", h(14, 0), h(15, 0), "cours")];
        assert_eq!(valider_creneaux(&j, None).unwrap().1, h(12, 30));
    }

    #[test]
    fn cibles_par_defaut_et_jours_normalises() {
        let c = valider_cibles(&[]).unwrap();
        assert_eq!(c, vec![CibleGrilleSaisie::default()]);
        let c = valider_cibles(&[CibleGrilleSaisie {
            site_id: None,
            niveau_id: None,
            jours: Some(vec![3, 1, 3]),
        }])
        .unwrap();
        assert_eq!(c[0].jours, Some(vec![1, 3]));
        // Sept jours = tous les jours.
        let c = valider_cibles(&[CibleGrilleSaisie {
            jours: Some(vec![1, 2, 3, 4, 5, 6, 7]),
            ..Default::default()
        }])
        .unwrap();
        assert_eq!(c[0].jours, None);
        // Jour hors plage, jours vides, cible en double.
        assert!(valider_cibles(&[CibleGrilleSaisie { jours: Some(vec![0]), ..Default::default() }]).is_err());
        assert!(valider_cibles(&[CibleGrilleSaisie { jours: Some(vec![8]), ..Default::default() }]).is_err());
        assert!(valider_cibles(&[CibleGrilleSaisie { jours: Some(vec![]), ..Default::default() }]).is_err());
        assert!(valider_cibles(&[CibleGrilleSaisie::default(), CibleGrilleSaisie::default()]).is_err());
    }

    #[test]
    fn code_de_reprise_stable() {
        assert_eq!(code_reprise(None), "REPRISE-ETAB");
        let s = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        assert_eq!(code_reprise(Some(s)), "REPRISE-111111112222");
    }
}
