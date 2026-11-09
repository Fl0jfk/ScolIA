//! Socle 5, lot S7 : périodisations (`05_STRUCTURE.md` §9.3, STR-31 à STR-33).
//!
//! Une **périodisation** découpe l'année (trimestres, semestres, périodes libres) ; elle est
//! assignée à des niveaux, des MEF, des classes ou posée par défaut. Pour une classe, la
//! périodisation d'évaluation est celle de la cible la plus précise (classe > MEF > niveau >
//! défaut) : un lycée en semestres et un collège en trimestres cohabitent dans le même
//! établissement. **Une seule source de vérité** : la fonction SQL
//! `ent_periodisation_pour_classe` (migration 335), lue par le port [`structure`].
//!
//! Règles du lot :
//! * **droits côté serveur** : lecture `referentiel.lire`, écriture `referentiel.gerer`
//!   (administrateur, direction, administratif ; l'action `periodes.gerer` du §13.2 n'existe pas
//!   encore au catalogue, `referentiel.gerer` en est le raccourci prévu, comme au lot S2) ;
//!   **modifier une période commencée (STR-33 ★) est réservé à la direction et à l'administrateur** ;
//! * **cloisonnement** : l'établissement vient de l'acteur ; écritures par `TenantTx` (RLS), clés
//!   composites (établissement, année) en base ;
//! * **journal** de structure en ajout seul (objets `periodisation`, `periode`) avec avant/après ;
//! * événements `referentiel.*` **sans identité d'élève** ;
//! * une période retirée ou une périodisation archivée est refusée tant que la saisie des notes
//!   s'appuie dessus (`ent_note_periode`, socle 21).

use std::collections::{HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use ent_auth::{Action, Role};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::{journaliser, publier, requis};
use crate::tenant_tx::TenantTx;

pub const KINDS: [&str; 3] = ["trimestre", "semestre", "periode"];
pub const USAGES: [&str; 2] = ["evaluation", "autre"];

async fn tx(e: &PgEnt, a: &Acteur<'_>) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(e.pool(), a.etablissement_id)
        .await
        .map_err(db)
}

fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(d)?))
}

fn usage_evaluation() -> String {
    "evaluation".to_string()
}

fn est_direction(a: &Acteur<'_>) -> bool {
    matches!(Role::parse(a.role), Some(Role::Direction | Role::Admin))
}

// ───────────────────────── Types ─────────────────────────

/// Période saisie (création ou remplacement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodeS7Saisie {
    pub code: String,
    pub libelle: String,
    /// `trimestre` | `semestre` | `periode`.
    pub kind: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    /// Dernier jour de saisie des notes ; absent = la fin de la période.
    #[serde(default)]
    pub date_arret_notes: Option<NaiveDate>,
    /// Date du conseil de classe de la période (information : le conseil est planifié au socle 23).
    #[serde(default)]
    pub date_conseil: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize)]
pub struct PeriodeVue {
    pub id: Uuid,
    pub periodisation_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub kind: String,
    pub ordre: i16,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub date_arret_notes: Option<NaiveDate>,
    pub date_conseil: Option<NaiveDate>,
}

impl PeriodeVue {
    /// Dernier jour de saisie des notes : la date d'arrêt, à défaut la fin de la période.
    pub fn arret_effectif(&self) -> NaiveDate {
        self.date_arret_notes.unwrap_or(self.date_fin)
    }

    /// La saisie des notes de cette période est-elle encore ouverte ce jour-là ?
    pub fn saisie_ouverte(&self, jour: NaiveDate) -> bool {
        jour >= self.date_debut && jour <= self.arret_effectif()
    }
}

/// Cible d'une périodisation : exactement un des quatre champs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CibleSaisie {
    #[serde(default)]
    pub niveau_id: Option<Uuid>,
    #[serde(default)]
    pub mef_id: Option<Uuid>,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub defaut: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize)]
pub struct CibleVue {
    pub id: Uuid,
    pub niveau_id: Option<Uuid>,
    pub mef_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub defaut: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeriodisationVue {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub usage: String,
    pub origine: String,
    pub archivee: bool,
    pub cibles: Vec<CibleVue>,
    pub periodes: Vec<PeriodeVue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PeriodisationSaisie {
    pub code: String,
    pub libelle: String,
    #[serde(default = "usage_evaluation")]
    pub usage: String,
    /// `trimestres` | `semestres` : découpe l'année en parts égales (à ajuster ensuite).
    #[serde(default)]
    pub modele: Option<String>,
    /// Périodes saisies à la main (si pas de modèle).
    #[serde(default)]
    pub periodes: Option<Vec<PeriodeS7Saisie>>,
    #[serde(default)]
    pub cibles: Vec<CibleSaisie>,
}

/// Modification d'une seule période (STR-33). Une date absente est inchangée ; `null` efface la
/// date d'arrêt ou de conseil.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PeriodeModif {
    #[serde(default)]
    pub libelle: Option<String>,
    #[serde(default)]
    pub date_debut: Option<NaiveDate>,
    #[serde(default)]
    pub date_fin: Option<NaiveDate>,
    #[serde(default, deserialize_with = "double_option")]
    pub date_arret_notes: Option<Option<NaiveDate>>,
    #[serde(default, deserialize_with = "double_option")]
    pub date_conseil: Option<Option<NaiveDate>>,
    /// Décale aussi la frontière de la période voisine contiguë (défaut : oui), pour garder une
    /// couverture sans trou ni chevauchement.
    #[serde(default = "vrai")]
    pub ajuster_voisines: bool,
}

fn vrai() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelePeriodes {
    pub modele: String,
    pub libelle: String,
    pub periodes: Vec<PeriodeS7Saisie>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClasseSansPeriodisation {
    pub classe_id: Uuid,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeriodisationIncomplete {
    pub periodisation_id: Uuid,
    pub code: String,
    pub raison: String,
}

/// Contrôle STR-32 : chaque classe doit résoudre exactement une périodisation d'évaluation.
#[derive(Debug, Clone, Serialize)]
pub struct ControlePeriodisation {
    pub annee_id: Uuid,
    pub classes_sans_periodisation: Vec<ClasseSansPeriodisation>,
    pub periodisations_incompletes: Vec<PeriodisationIncomplete>,
    /// Périodisations d'évaluation sans aucune cible : elles ne s'appliquent à personne.
    pub periodisations_sans_cible: Vec<Uuid>,
    pub ok: bool,
}

// ───────────────────────── Calculs purs (testables sans base) ─────────────────────────

/// Découpe `[debut, fin]` en `n` parts contiguës de durée voisine.
pub fn decouper(debut: NaiveDate, fin: NaiveDate, n: i64) -> Vec<(NaiveDate, NaiveDate)> {
    let total = (fin - debut).num_days() + 1;
    if n < 1 || total < n {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut d = debut;
    for i in 0..n {
        let jours = total / n + if i < total % n { 1 } else { 0 };
        let f = d + Duration::days(jours - 1);
        out.push((d, f));
        d = f + Duration::days(1);
    }
    out
}

/// Modèle prêt à l'emploi : `trimestres` (T1 à T3) ou `semestres` (S1, S2).
pub fn modele_periodes(modele: &str, debut: NaiveDate, fin: NaiveDate) -> Option<Vec<PeriodeS7Saisie>> {
    let (n, prefixe, kind, nom) = match modele {
        "trimestres" => (3, "T", "trimestre", "Trimestre"),
        "semestres" => (2, "S", "semestre", "Semestre"),
        _ => return None,
    };
    let parts = decouper(debut, fin, n);
    if parts.is_empty() {
        return None;
    }
    Some(
        parts
            .into_iter()
            .enumerate()
            .map(|(i, (d, f))| PeriodeS7Saisie {
                code: format!("{prefixe}{}", i + 1),
                libelle: format!("{nom} {}", i + 1),
                kind: kind.to_string(),
                date_debut: d,
                date_fin: f,
                date_arret_notes: None,
                date_conseil: None,
            })
            .collect(),
    )
}

/// Valide et normalise un jeu de périodes dans les bornes `[debut, fin]` de l'année. Pour une
/// périodisation d'évaluation, la couverture doit être **totale, sans trou ni chevauchement**.
pub fn valider_periodes(
    periodes: &[PeriodeS7Saisie],
    debut: NaiveDate,
    fin: NaiveDate,
    usage: &str,
) -> Result<Vec<PeriodeS7Saisie>, EntDbError> {
    if periodes.is_empty() || periodes.len() > 12 {
        return Err(invalide());
    }
    let mut v: Vec<PeriodeS7Saisie> = Vec::with_capacity(periodes.len());
    let mut codes = HashSet::new();
    for p in periodes {
        let code = requis(&p.code, 20)?.to_uppercase();
        if !code
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
            || !codes.insert(code.clone())
        {
            return Err(invalide());
        }
        if !KINDS.contains(&p.kind.as_str())
            || p.date_fin < p.date_debut
            || p.date_debut < debut
            || p.date_fin > fin
        {
            return Err(invalide());
        }
        for d in [p.date_arret_notes, p.date_conseil].into_iter().flatten() {
            if d < p.date_debut || d > fin {
                return Err(invalide());
            }
        }
        if let (Some(a), Some(c)) = (p.date_arret_notes, p.date_conseil) {
            if c < a {
                return Err(invalide());
            }
        }
        v.push(PeriodeS7Saisie {
            code,
            libelle: requis(&p.libelle, 120)?,
            ..p.clone()
        });
    }
    v.sort_by_key(|p| p.date_debut);
    for w in v.windows(2) {
        if w[1].date_debut <= w[0].date_fin {
            return Err(invalide());
        }
        if usage == "evaluation" && w[1].date_debut != w[0].date_fin + Duration::days(1) {
            return Err(invalide());
        }
    }
    if usage == "evaluation" && (v[0].date_debut != debut || v[v.len() - 1].date_fin != fin) {
        return Err(invalide());
    }
    Ok(v)
}

// ───────────────────────── Lectures partagées ─────────────────────────

const SQL_PERIODE: &str = "SELECT id, periodisation_id, code, libelle, kind, ordre, date_debut, date_fin,
        date_arret_notes, date_conseil FROM ent_periode";

async fn periodes_de(
    conn: &mut PgConnection,
    etab: Uuid,
    pz: Uuid,
) -> Result<Vec<PeriodeVue>, EntDbError> {
    sqlx::query_as::<_, PeriodeVue>(&format!(
        "{SQL_PERIODE} WHERE etablissement_id = $1 AND periodisation_id = $2 ORDER BY date_debut"
    ))
    .bind(etab)
    .bind(pz)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn cibles_de(
    conn: &mut PgConnection,
    etab: Uuid,
    pz: Uuid,
) -> Result<Vec<CibleVue>, EntDbError> {
    sqlx::query_as::<_, CibleVue>(
        "SELECT id, niveau_id, mef_id, classe_id, defaut FROM ent_periodisation_cible
          WHERE etablissement_id = $1 AND periodisation_id = $2
          ORDER BY defaut DESC, niveau_id, mef_id, classe_id",
    )
    .bind(etab)
    .bind(pz)
    .fetch_all(conn)
    .await
    .map_err(db)
}

#[derive(FromRow)]
struct LignePz {
    id: Uuid,
    annee_id: Uuid,
    code: String,
    libelle: String,
    usage: String,
    origine: String,
    archivee: bool,
}

const SQL_PZ: &str = "SELECT id, annee_id, code, libelle, usage, origine, archivee_le IS NOT NULL AS archivee
                        FROM ent_periodisation";

async fn vue_de(
    conn: &mut PgConnection,
    etab: Uuid,
    l: LignePz,
) -> Result<PeriodisationVue, EntDbError> {
    let periodes = periodes_de(conn, etab, l.id).await?;
    let cibles = cibles_de(conn, etab, l.id).await?;
    Ok(PeriodisationVue {
        id: l.id,
        annee_id: l.annee_id,
        code: l.code,
        libelle: l.libelle,
        usage: l.usage,
        origine: l.origine,
        archivee: l.archivee,
        cibles,
        periodes,
    })
}

async fn pz_lue(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<LignePz, EntDbError> {
    let sql = format!(
        "{SQL_PZ} WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as::<_, LignePz>(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

struct AnneeBornes {
    debut: NaiveDate,
    fin: NaiveDate,
}

/// Verrouille l'année et renvoie ses bornes ; une année clôturée n'accepte plus d'écriture.
async fn annee_verrouillee(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
) -> Result<AnneeBornes, EntDbError> {
    let r: Option<(NaiveDate, NaiveDate, bool)> = sqlx::query_as(
        "SELECT date_debut, date_fin, cloturee_le IS NOT NULL FROM ent_annee_scolaire
          WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(etab)
    .bind(annee_id)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    match r {
        None => Err(introuvable()),
        Some((_, _, true)) => Err(conflit()),
        Some((debut, fin, false)) => Ok(AnneeBornes { debut, fin }),
    }
}

fn json_periode(p: &PeriodeVue) -> Value {
    json!({
        "code": p.code, "du": p.date_debut, "au": p.date_fin,
        "arret_notes": p.date_arret_notes, "conseil": p.date_conseil,
    })
}

async fn cibles_valides(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
    cibles: &[CibleSaisie],
) -> Result<(), EntDbError> {
    let mut vues = HashSet::new();
    for c in cibles {
        let n = c.niveau_id.is_some() as u8
            + c.mef_id.is_some() as u8
            + c.classe_id.is_some() as u8
            + c.defaut as u8;
        if n != 1 || !vues.insert((c.niveau_id, c.mef_id, c.classe_id, c.defaut)) {
            return Err(invalide());
        }
        let ok: i64 = if let Some(n) = c.niveau_id {
            sqlx::query_scalar("SELECT count(*) FROM ent_niveau WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(n)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?
        } else if let Some(m) = c.mef_id {
            sqlx::query_scalar("SELECT count(*) FROM ent_mef WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(m)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?
        } else if let Some(cl) = c.classe_id {
            sqlx::query_scalar(
                "SELECT count(*) FROM ent_classe WHERE etablissement_id = $1 AND id = $2 AND annee_id = $3",
            )
            .bind(etab)
            .bind(cl)
            .bind(annee_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?
        } else {
            1
        };
        if ok == 0 {
            // Cible d'un autre établissement ou d'une autre année : inexistante pour l'appelant.
            return Err(introuvable());
        }
    }
    Ok(())
}

async fn poser_cibles(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
    pz: Uuid,
    usage: &str,
    cibles: &[CibleSaisie],
) -> Result<(), EntDbError> {
    for c in cibles {
        sqlx::query(
            "INSERT INTO ent_periodisation_cible
               (id, etablissement_id, annee_id, usage, periodisation_id, niveau_id, mef_id, classe_id, defaut)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(annee_id)
        .bind(usage)
        .bind(pz)
        .bind(c.niveau_id)
        .bind(c.mef_id)
        .bind(c.classe_id)
        .bind(c.defaut)
        .execute(&mut *conn)
        .await
        .map_err(db)?; // 23505 : une autre périodisation d'évaluation vise déjà cette cible → 409
    }
    Ok(())
}

async fn inserer_periode(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
    pz: Uuid,
    ordre: usize,
    p: &PeriodeS7Saisie,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_periode
           (id, etablissement_id, annee_id, periodisation_id, code, libelle, kind, ordre,
            date_debut, date_fin, date_arret_notes, date_conseil, origine)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,'service')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(annee_id)
    .bind(pz)
    .bind(&p.code)
    .bind(&p.libelle)
    .bind(&p.kind)
    .bind((ordre + 1) as i16)
    .bind(p.date_debut)
    .bind(p.date_fin)
    .bind(p.date_arret_notes)
    .bind(p.date_conseil)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Résultat d'un remplacement : périodes modifiées (avant → après) pour l'événement et le journal.
struct Diff {
    ajoutees: Vec<String>,
    retirees: Vec<String>,
    modifiees: Vec<Value>,
    /// Une période déjà commencée est touchée (retirée ou modifiée) : réservé à la direction.
    touche_commencee: bool,
}

/// Remplace atomiquement les périodes d'une périodisation (déjà verrouillée et validée).
async fn remplacer_periodes(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
    pz: Uuid,
    nouvelles: &[PeriodeS7Saisie],
    aujourdhui: NaiveDate,
    droit_direction: bool,
) -> Result<Diff, EntDbError> {
    let existantes = periodes_de(conn, etab, pz).await?;
    let par_code: HashMap<&str, &PeriodeVue> =
        existantes.iter().map(|p| (p.code.as_str(), p)).collect();
    let cibles_codes: HashSet<&str> = nouvelles.iter().map(|p| p.code.as_str()).collect();
    let mut diff = Diff {
        ajoutees: Vec::new(),
        retirees: Vec::new(),
        modifiees: Vec::new(),
        touche_commencee: false,
    };
    for e in &existantes {
        if !cibles_codes.contains(e.code.as_str()) {
            diff.retirees.push(e.code.clone());
            diff.touche_commencee |= e.date_debut <= aujourdhui;
        }
    }
    for n in nouvelles {
        match par_code.get(n.code.as_str()) {
            None => diff.ajoutees.push(n.code.clone()),
            Some(e) => {
                let change = e.date_debut != n.date_debut
                    || e.date_fin != n.date_fin
                    || e.date_arret_notes != n.date_arret_notes
                    || e.date_conseil != n.date_conseil
                    || e.libelle != n.libelle
                    || e.kind != n.kind;
                if change {
                    diff.touche_commencee |= e.date_debut <= aujourdhui;
                    diff.modifiees.push(json!({
                        "code": n.code,
                        "avant": json_periode(e),
                        "apres": { "code": n.code, "du": n.date_debut, "au": n.date_fin,
                                   "arret_notes": n.date_arret_notes, "conseil": n.date_conseil },
                    }));
                }
            }
        }
    }
    if diff.touche_commencee && !droit_direction {
        return Err(EntDbError::Forbidden);
    }
    // Retraits : refusés si la saisie de notes s'appuie dessus.
    if !diff.retirees.is_empty() {
        let utilisees: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_periode p
              WHERE p.etablissement_id = $1 AND p.periodisation_id = $2 AND p.code = ANY($3)
                AND EXISTS (SELECT 1 FROM ent_note_periode np
                             WHERE np.etablissement_id = p.etablissement_id AND np.periode_id = p.id)",
        )
        .bind(etab)
        .bind(pz)
        .bind(&diff.retirees)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if utilisees > 0 {
            return Err(conflit());
        }
        sqlx::query(
            "DELETE FROM ent_periode WHERE etablissement_id = $1 AND periodisation_id = $2 AND code = ANY($3)",
        )
        .bind(etab)
        .bind(pz)
        .bind(&diff.retirees)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    // Les périodes voisines se décalent dans la même transaction : contrainte vérifiée au commit.
    sqlx::query("SET CONSTRAINTS ent_periode_chevauchement DEFERRED")
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    for (i, n) in nouvelles.iter().enumerate() {
        match par_code.get(n.code.as_str()) {
            Some(e) => {
                sqlx::query(
                    "UPDATE ent_periode SET libelle = $3, kind = $4, ordre = $5, date_debut = $6,
                            date_fin = $7, date_arret_notes = $8, date_conseil = $9, origine = 'service'
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(e.id)
                .bind(&n.libelle)
                .bind(&n.kind)
                .bind((i + 1) as i16)
                .bind(n.date_debut)
                .bind(n.date_fin)
                .bind(n.date_arret_notes)
                .bind(n.date_conseil)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            }
            None => inserer_periode(conn, etab, annee_id, pz, i, n).await?,
        }
    }
    Ok(diff)
}

// ───────────────────────── Service ─────────────────────────

impl PgEnt {
    /// STR-31 : modèles prêts à l'emploi (3 trimestres, 2 semestres) calculés sur l'année.
    pub async fn structure_periodisation_modeles(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<Vec<ModelePeriodes>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let r: Option<(NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let (d, f) = r.ok_or_else(introuvable)?;
        Ok([("trimestres", "Trois trimestres"), ("semestres", "Deux semestres")]
            .into_iter()
            .filter_map(|(m, l)| {
                modele_periodes(m, d, f).map(|periodes| ModelePeriodes {
                    modele: m.to_string(),
                    libelle: l.to_string(),
                    periodes,
                })
            })
            .collect())
    }

    pub async fn structure_periodisations(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        archivees: bool,
    ) -> Result<Vec<PeriodisationVue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let lignes = sqlx::query_as::<_, LignePz>(&format!(
            "{SQL_PZ} WHERE etablissement_id = $1 AND annee_id = $2 AND ($3 OR archivee_le IS NULL)
              ORDER BY usage, code"
        ))
        .bind(a.etablissement_id)
        .bind(annee_id)
        .bind(archivees)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for l in lignes {
            out.push(vue_de(&mut t, a.etablissement_id, l).await?);
        }
        t.commit().await.map_err(db)?;
        Ok(out)
    }

    pub async fn structure_periodisation(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<PeriodisationVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let l = pz_lue(&mut t, a.etablissement_id, id, false).await?;
        let v = vue_de(&mut t, a.etablissement_id, l).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// STR-31 + STR-32 : crée une périodisation (modèle ou périodes saisies) et, si fournies, ses
    /// cibles. Une périodisation d'évaluation couvre l'année sans trou ni chevauchement.
    pub async fn structure_periodisation_creer(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        s: PeriodisationSaisie,
    ) -> Result<PeriodisationVue, EntDbError> {
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
        if !USAGES.contains(&s.usage.as_str()) {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let annee = annee_verrouillee(&mut t, e, annee_id).await?;
        let brutes = match (&s.modele, &s.periodes) {
            (Some(m), None) => modele_periodes(m, annee.debut, annee.fin).ok_or_else(invalide)?,
            (None, Some(p)) => p.clone(),
            _ => return Err(invalide()),
        };
        let periodes = valider_periodes(&brutes, annee.debut, annee.fin, &s.usage)?;
        cibles_valides(&mut t, e, annee_id, &s.cibles).await?;
        let pz = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_periodisation (id, etablissement_id, annee_id, code, libelle, usage, origine, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,'service',$7)",
        )
        .bind(pz)
        .bind(e)
        .bind(annee_id)
        .bind(&code)
        .bind(&libelle)
        .bind(&s.usage)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?; // 23505 : code déjà pris dans l'année → 409
        for (i, p) in periodes.iter().enumerate() {
            inserer_periode(&mut t, e, annee_id, pz, i, p).await?;
        }
        poser_cibles(&mut t, e, annee_id, pz, &s.usage, &s.cibles).await?;
        journaliser(
            &mut t,
            a,
            "periodisation",
            &pz.to_string(),
            "creee",
            None,
            Some(json!({
                "code": code, "usage": s.usage, "annee_id": annee_id,
                "periodes": periodes.iter().map(|p| json!({"code": p.code, "du": p.date_debut, "au": p.date_fin})).collect::<Vec<_>>(),
                "cibles": s.cibles.len(),
            })),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.periodes_definies",
            pz,
            json!({ "annee_id": annee_id, "usage": s.usage,
                    "periodes": periodes.iter().map(|p| p.code.clone()).collect::<Vec<_>>() }),
        )
        .await?;
        let l = pz_lue(&mut t, e, pz, false).await?;
        let v = vue_de(&mut t, e, l).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// STR-31 / STR-33 : remplace l'ensemble des périodes d'une périodisation. Toucher une période
    /// **déjà commencée** (dates, retrait) est réservé à la direction (★) ; les périodes futures
    /// restent à la portée de l'administratif.
    pub async fn structure_periodisation_remplacer_periodes(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        periodes: Vec<PeriodeS7Saisie>,
    ) -> Result<PeriodisationVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut t = tx(self, a).await?;
        let pz = pz_lue(&mut t, e, id, true).await?;
        let annee = annee_verrouillee(&mut t, e, pz.annee_id).await?;
        let valides = valider_periodes(&periodes, annee.debut, annee.fin, &pz.usage)?;
        let jour = crate::structure_etab::structure::aujourdhui(&mut t, e).await?;
        let diff = remplacer_periodes(&mut t, e, pz.annee_id, id, &valides, jour, est_direction(a)).await?;
        self.cloturer_modification(&mut t, a, &pz, &diff).await?;
        let ev = self.evenement_modification(&mut t, a, &pz, &diff).await?;
        let l = pz_lue(&mut t, e, id, false).await?;
        let v = vue_de(&mut t, e, l).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&ev);
        Ok(v)
    }

    /// STR-33 : modifie une seule période (dates, libellé, arrêt des notes, conseil). Par défaut,
    /// la frontière de la période voisine contiguë suit (couverture conservée).
    pub async fn structure_periode_modifier(
        &self,
        a: &Acteur<'_>,
        periode_id: Uuid,
        m: PeriodeModif,
    ) -> Result<PeriodisationVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut t = tx(self, a).await?;
        let pid: Uuid = sqlx::query_scalar(
            "SELECT periodisation_id FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(periode_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let pz = pz_lue(&mut t, e, pid, true).await?;
        let annee = annee_verrouillee(&mut t, e, pz.annee_id).await?;
        let actuelles = periodes_de(&mut t, e, pid).await?;
        let mut sai: Vec<PeriodeS7Saisie> = actuelles
            .iter()
            .map(|p| PeriodeS7Saisie {
                code: p.code.clone(),
                libelle: p.libelle.clone(),
                kind: p.kind.clone(),
                date_debut: p.date_debut,
                date_fin: p.date_fin,
                date_arret_notes: p.date_arret_notes,
                date_conseil: p.date_conseil,
            })
            .collect();
        let i = actuelles
            .iter()
            .position(|p| p.id == periode_id)
            .ok_or_else(introuvable)?;
        if let Some(l) = &m.libelle {
            sai[i].libelle = l.clone();
        }
        if let Some(d) = m.date_debut {
            sai[i].date_debut = d;
            if m.ajuster_voisines && i > 0 && actuelles[i - 1].date_fin + Duration::days(1) == actuelles[i].date_debut {
                sai[i - 1].date_fin = d - Duration::days(1);
            }
        }
        if let Some(d) = m.date_fin {
            sai[i].date_fin = d;
            if m.ajuster_voisines
                && i + 1 < sai.len()
                && actuelles[i].date_fin + Duration::days(1) == actuelles[i + 1].date_debut
            {
                sai[i + 1].date_debut = d + Duration::days(1);
            }
        }
        if let Some(v) = m.date_arret_notes {
            sai[i].date_arret_notes = v;
        }
        if let Some(v) = m.date_conseil {
            sai[i].date_conseil = v;
        }
        // Un arrêt ou un conseil devenus antérieurs au début suivent le début modifié : refus net.
        let valides = valider_periodes(&sai, annee.debut, annee.fin, &pz.usage)?;
        let jour = crate::structure_etab::structure::aujourdhui(&mut t, e).await?;
        let diff = remplacer_periodes(&mut t, e, pz.annee_id, pid, &valides, jour, est_direction(a)).await?;
        self.cloturer_modification(&mut t, a, &pz, &diff).await?;
        let ev = self.evenement_modification(&mut t, a, &pz, &diff).await?;
        let l = pz_lue(&mut t, e, pid, false).await?;
        let v = vue_de(&mut t, e, l).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&ev);
        Ok(v)
    }

    async fn cloturer_modification(
        &self,
        t: &mut TenantTx<'static>,
        a: &Acteur<'_>,
        pz: &LignePzPublic,
        diff: &Diff,
    ) -> Result<(), EntDbError> {
        if diff.ajoutees.is_empty() && diff.retirees.is_empty() && diff.modifiees.is_empty() {
            return Ok(());
        }
        journaliser(
            t,
            a,
            "periode",
            &pz.id.to_string(),
            "modifiees",
            Some(json!({ "retirees": diff.retirees })),
            Some(json!({ "ajoutees": diff.ajoutees, "modifiees": diff.modifiees })),
        )
        .await
    }

    async fn evenement_modification(
        &self,
        t: &mut TenantTx<'static>,
        a: &Acteur<'_>,
        pz: &LignePzPublic,
        diff: &Diff,
    ) -> Result<Vec<ent_events::MetierEvent>, EntDbError> {
        let mut evs = Vec::new();
        if !diff.ajoutees.is_empty() || !diff.retirees.is_empty() {
            evs.push(
                publier(
                    t,
                    a,
                    "referentiel.periodes_definies",
                    pz.id,
                    json!({ "annee_id": pz.annee_id, "usage": pz.usage,
                            "ajoutees": diff.ajoutees, "retirees": diff.retirees }),
                )
                .await?,
            );
        }
        if !diff.modifiees.is_empty() {
            // Les socles 21 (notes hors période) et 23 (conseils planifiés) relisent par le port.
            evs.push(
                publier(
                    t,
                    a,
                    "referentiel.periode_modifiee",
                    pz.id,
                    json!({ "annee_id": pz.annee_id, "usage": pz.usage, "periodes": diff.modifiees }),
                )
                .await?,
            );
        }
        Ok(evs)
    }

    /// STR-32 : remplace les cibles d'une périodisation. Pour l'évaluation, une cible n'a qu'une
    /// périodisation par année (409 sinon).
    pub async fn structure_periodisation_assigner(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        cibles: Vec<CibleSaisie>,
    ) -> Result<PeriodisationVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut t = tx(self, a).await?;
        let pz = pz_lue(&mut t, e, id, true).await?;
        annee_verrouillee(&mut t, e, pz.annee_id).await?;
        if pz.archivee {
            return Err(conflit());
        }
        cibles_valides(&mut t, e, pz.annee_id, &cibles).await?;
        let avant = cibles_de(&mut t, e, id).await?;
        sqlx::query("DELETE FROM ent_periodisation_cible WHERE etablissement_id = $1 AND periodisation_id = $2")
            .bind(e)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        poser_cibles(&mut t, e, pz.annee_id, id, &pz.usage, &cibles).await?;
        journaliser(
            &mut t,
            a,
            "periodisation",
            &id.to_string(),
            "assignee",
            Some(json!({ "cibles": avant.len() })),
            Some(json!({ "cibles": cibles.len() })),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.periodisation_assignee",
            id,
            json!({ "annee_id": pz.annee_id, "usage": pz.usage, "cibles": cibles.len() }),
        )
        .await?;
        let l = pz_lue(&mut t, e, id, false).await?;
        let v = vue_de(&mut t, e, l).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// Archive une périodisation (elle ne se supprime pas) : refusé tant que la saisie des notes
    /// s'appuie sur l'une de ses périodes. Ses classes retombent sur la cible suivante.
    pub async fn structure_periodisation_archiver(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<PeriodisationVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut t = tx(self, a).await?;
        let pz = pz_lue(&mut t, e, id, true).await?;
        if pz.archivee {
            return Err(conflit());
        }
        let utilisees: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_periode p
              WHERE p.etablissement_id = $1 AND p.periodisation_id = $2
                AND EXISTS (SELECT 1 FROM ent_note_periode np
                             WHERE np.etablissement_id = p.etablissement_id AND np.periode_id = p.id)",
        )
        .bind(e)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if utilisees > 0 {
            return Err(conflit());
        }
        sqlx::query("UPDATE ent_periodisation SET archivee_le = now(), modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(e)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        journaliser(&mut t, a, "periodisation", &id.to_string(), "archivee", None, Some(json!({ "code": pz.code })))
            .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.periodisation_assignee",
            id,
            json!({ "annee_id": pz.annee_id, "usage": pz.usage, "archivee": true }),
        )
        .await?;
        let l = pz_lue(&mut t, e, id, false).await?;
        let v = vue_de(&mut t, e, l).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// STR-32 : contrôle « chaque classe résout exactement une périodisation d'évaluation ».
    pub async fn structure_periodisation_controle(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<ControlePeriodisation, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let e = a.etablissement_id;
        let mut t = tx(self, a).await?;
        let bornes: (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let sans: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT c.id, c.code FROM ent_classe c
              WHERE c.etablissement_id = $1 AND c.annee_id = $2 AND c.archivee_le IS NULL
                AND ent_periodisation_pour_classe(c.etablissement_id, c.id) IS NULL
              ORDER BY c.code",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let lignes = sqlx::query_as::<_, LignePz>(&format!(
            "{SQL_PZ} WHERE etablissement_id = $1 AND annee_id = $2 AND usage = 'evaluation'
                AND archivee_le IS NULL ORDER BY code"
        ))
        .bind(e)
        .bind(annee_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut incompletes = Vec::new();
        let mut sans_cible = Vec::new();
        for l in lignes {
            if cibles_de(&mut t, e, l.id).await?.is_empty() {
                sans_cible.push(l.id);
            }
            let ps = periodes_de(&mut t, e, l.id).await?;
            let saisies: Vec<PeriodeS7Saisie> = ps
                .iter()
                .map(|p| PeriodeS7Saisie {
                    code: p.code.clone(),
                    libelle: p.libelle.clone(),
                    kind: p.kind.clone(),
                    date_debut: p.date_debut,
                    date_fin: p.date_fin,
                    date_arret_notes: p.date_arret_notes,
                    date_conseil: p.date_conseil,
                })
                .collect();
            if ps.is_empty() {
                incompletes.push(PeriodisationIncomplete {
                    periodisation_id: l.id,
                    code: l.code,
                    raison: "aucune période".to_string(),
                });
            } else if valider_periodes(&saisies, bornes.0, bornes.1, "evaluation").is_err() {
                incompletes.push(PeriodisationIncomplete {
                    periodisation_id: l.id,
                    code: l.code,
                    raison: "ne couvre pas l'année sans trou ni chevauchement".to_string(),
                });
            }
        }
        t.commit().await.map_err(db)?;
        let ok = sans.is_empty() && incompletes.is_empty();
        Ok(ControlePeriodisation {
            annee_id,
            classes_sans_periodisation: sans
                .into_iter()
                .map(|(classe_id, code)| ClasseSansPeriodisation { classe_id, code })
                .collect(),
            periodisations_incompletes: incompletes,
            periodisations_sans_cible: sans_cible,
            ok,
        })
    }

    /// Périodes de la périodisation d'évaluation d'une classe (lecture autorisée par
    /// `referentiel.lire`, tout le personnel). Passe par le port [`structure`].
    pub async fn structure_periodes_de_classe(
        &self,
        a: &Acteur<'_>,
        classe_id: Uuid,
    ) -> Result<Vec<PeriodeVue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let ps = structure::periodes_de_classe(&mut t, a.etablissement_id, classe_id).await?;
        t.commit().await.map_err(db)?;
        Ok(ps)
    }
}

/// Alias interne : la ligne de périodisation lue pour le journal et les événements.
type LignePzPublic = LignePz;

// ───────────────────────── Port lu par les autres socles ─────────────────────────

/// Port `structure::periode(date, classe)` / `periodes(annee, classe)` du §6.3 : fonctions de
/// lecture prenant la connexion/transaction de l'appelant (qui a déjà autorisé). **Lu par les
/// évaluations (socle 21), les bulletins et conseils (23), le suivi (24).** Aucune écriture, aucune
/// création implicite : une classe sans cible applicable répond « aucune périodisation ».
pub mod structure {
    use super::*;

    /// Périodisation d'évaluation d'une classe (classe > MEF > niveau > défaut), si elle existe.
    pub async fn periodisation_de_classe(
        conn: &mut PgConnection,
        etab: Uuid,
        classe_id: Uuid,
    ) -> Result<Option<Uuid>, EntDbError> {
        sqlx::query_scalar("SELECT ent_periodisation_pour_classe($1, $2)")
            .bind(etab)
            .bind(classe_id)
            .fetch_one(conn)
            .await
            .map_err(db)
    }

    /// Toutes les périodes de la classe, dans l'ordre chronologique (vide si aucune périodisation).
    pub async fn periodes_de_classe(
        conn: &mut PgConnection,
        etab: Uuid,
        classe_id: Uuid,
    ) -> Result<Vec<PeriodeVue>, EntDbError> {
        match periodisation_de_classe(&mut *conn, etab, classe_id).await? {
            None => Ok(Vec::new()),
            Some(pz) => periodes_de(conn, etab, pz).await,
        }
    }

    /// Période de la classe qui contient `jour` (`None` : hors période, par exemple entre deux
    /// découpages incomplets ou hors année).
    pub async fn periode_a_la_date(
        conn: &mut PgConnection,
        etab: Uuid,
        classe_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<PeriodeVue>, EntDbError> {
        Ok(periodes_de_classe(conn, etab, classe_id)
            .await?
            .into_iter()
            .find(|p| p.date_debut <= jour && jour <= p.date_fin))
    }

    /// Dates de la période identifiée **dans la périodisation de la classe** (jamais une période
    /// d'une autre périodisation : `None` si elle n'en fait pas partie).
    pub async fn periode_de_classe(
        conn: &mut PgConnection,
        etab: Uuid,
        classe_id: Uuid,
        periode_id: Uuid,
    ) -> Result<Option<PeriodeVue>, EntDbError> {
        Ok(periodes_de_classe(conn, etab, classe_id)
            .await?
            .into_iter()
            .find(|p| p.id == periode_id))
    }

    /// La saisie des notes de cette période est-elle ouverte ce jour-là pour cette classe ? (La
    /// date d'arrêt est lue ici ; le verrou d'état de saisie reste au socle 21.)
    pub async fn saisie_notes_ouverte(
        conn: &mut PgConnection,
        etab: Uuid,
        classe_id: Uuid,
        periode_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        Ok(periode_de_classe(conn, etab, classe_id, periode_id)
            .await?
            .is_some_and(|p| p.saisie_ouverte(jour)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, j).unwrap()
    }

    #[test]
    fn decoupe_en_parts_contigues_sans_perte() {
        let (a, b) = (d(2026, 9, 1), d(2027, 7, 4));
        for n in [2, 3] {
            let p = decouper(a, b, n);
            assert_eq!(p.len() as i64, n);
            assert_eq!(p[0].0, a);
            assert_eq!(p[p.len() - 1].1, b);
            for w in p.windows(2) {
                assert_eq!(w[1].0, w[0].1 + Duration::days(1));
            }
        }
        assert!(decouper(a, a, 3).is_empty());
    }

    #[test]
    fn modeles_trimestres_et_semestres() {
        let t = modele_periodes("trimestres", d(2026, 9, 1), d(2027, 6, 30)).unwrap();
        assert_eq!(t.iter().map(|p| p.code.as_str()).collect::<Vec<_>>(), ["T1", "T2", "T3"]);
        let s = modele_periodes("semestres", d(2026, 9, 1), d(2027, 6, 30)).unwrap();
        assert_eq!(s.len(), 2);
        assert!(s.iter().all(|p| p.kind == "semestre"));
        assert!(modele_periodes("quadrimestres", d(2026, 9, 1), d(2027, 6, 30)).is_none());
        // Un modèle est toujours une couverture valide.
        assert!(valider_periodes(&t, d(2026, 9, 1), d(2027, 6, 30), "evaluation").is_ok());
    }

    #[test]
    fn couverture_evaluation_stricte_et_libre_pour_autre() {
        let (a, b) = (d(2026, 9, 1), d(2027, 6, 30));
        let mut t = modele_periodes("trimestres", a, b).unwrap();
        // Trou entre T1 et T2.
        t[1].date_debut = t[1].date_debut + Duration::days(3);
        assert!(valider_periodes(&t, a, b, "evaluation").is_err());
        assert!(valider_periodes(&t, a, b, "autre").is_ok());
        // Chevauchement refusé dans tous les cas.
        let mut t = modele_periodes("trimestres", a, b).unwrap();
        t[1].date_debut = t[0].date_fin;
        assert!(valider_periodes(&t, a, b, "autre").is_err());
        // Début qui ne commence pas à celui de l'année.
        let mut t = modele_periodes("trimestres", a, b).unwrap();
        t[0].date_debut = a + Duration::days(1);
        assert!(valider_periodes(&t, a, b, "evaluation").is_err());
        // Hors année.
        let mut t = modele_periodes("trimestres", a, b).unwrap();
        t[2].date_fin = b + Duration::days(1);
        assert!(valider_periodes(&t, a, b, "autre").is_err());
    }

    #[test]
    fn arret_des_notes_et_conseil_coherents() {
        let (a, b) = (d(2026, 9, 1), d(2027, 6, 30));
        let mut t = modele_periodes("trimestres", a, b).unwrap();
        t[0].date_arret_notes = Some(t[0].date_fin);
        t[0].date_conseil = Some(t[0].date_fin + Duration::days(7));
        assert!(valider_periodes(&t, a, b, "evaluation").is_ok());
        // Conseil avant l'arrêt des notes : refusé.
        t[0].date_conseil = Some(t[0].date_fin - Duration::days(1));
        assert!(valider_periodes(&t, a, b, "evaluation").is_err());
        // Arrêt avant le début : refusé.
        let mut t = modele_periodes("trimestres", a, b).unwrap();
        t[1].date_arret_notes = Some(t[1].date_debut - Duration::days(1));
        assert!(valider_periodes(&t, a, b, "evaluation").is_err());
    }

    #[test]
    fn codes_doubles_ou_invalides_refuses() {
        let (a, b) = (d(2026, 9, 1), d(2027, 6, 30));
        let mut t = modele_periodes("trimestres", a, b).unwrap();
        t[1].code = "t1".into(); // même code après mise en majuscules
        assert!(valider_periodes(&t, a, b, "evaluation").is_err());
        let mut t = modele_periodes("trimestres", a, b).unwrap();
        t[0].code = "T 1; DROP".into();
        assert!(valider_periodes(&t, a, b, "evaluation").is_err());
        let mut t = modele_periodes("trimestres", a, b).unwrap();
        t[0].kind = "quadrimestre".into();
        assert!(valider_periodes(&t, a, b, "evaluation").is_err());
    }

    #[test]
    fn saisie_ouverte_jusqu_a_l_arret() {
        let p = PeriodeVue {
            id: Uuid::nil(),
            periodisation_id: Uuid::nil(),
            code: "T1".into(),
            libelle: "T1".into(),
            kind: "trimestre".into(),
            ordre: 1,
            date_debut: d(2026, 9, 1),
            date_fin: d(2026, 12, 18),
            date_arret_notes: Some(d(2026, 12, 11)),
            date_conseil: None,
        };
        assert!(p.saisie_ouverte(d(2026, 12, 11)));
        assert!(!p.saisie_ouverte(d(2026, 12, 12)));
        assert!(!p.saisie_ouverte(d(2026, 8, 31)));
        let sans_arret = PeriodeVue { date_arret_notes: None, ..p };
        assert!(sans_arret.saisie_ouverte(d(2026, 12, 18)));
    }
}
