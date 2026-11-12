//! Socle 10, lot T1 — versions d'emploi du temps, besoins, pièces et placements
//! (`docs/socles/10_EMPLOI_DU_TEMPS.md` §8.4, §21.2, §22 M1–M2).
//!
//! * Une **version** est un plan (brouillon → publiée) qui vise des **cours** (socle 5). Les écritures
//!   du plan se font en **brouillon** ; la publication (contrôles, deux temps, retour arrière) est le
//!   lot T5 : seule la **reprise** des créneaux existants produit ici une version publiée.
//! * La **reprise** (M1–M2) lit `ent_edt_creneau` et crée, de façon **rejouable**, une version de base
//!   « reprise » couvrant l'année : un besoin par cours, une pièce et un placement par créneau. Un
//!   créneau sans cours est **listé**, jamais deviné ; un créneau dont l'heure ne tombe sur aucun code
//!   de grille est repris « hors grille » et compté ; une salle inconnue du référentiel est comptée.
//!   Rien n'est supprimé ni modifié dans `ent_edt_creneau`, qui reste la source des lecteurs actuels
//!   (appel, cahier, notes) jusqu'à la bascule du lot T11.
//! * Les **conflits** d'un brouillon (cours, intervenant, lieu) sont des **avertissements** consultables
//!   (`edt_version_conflits`) : un plan se construit par approximations successives, il ne se bloque
//!   qu'à la publication (T5).
//! * Droits : `edt.gerer` (moteur de droits, D-017) pour écrire ; le personnel lit. Une famille ou un
//!   élève ne lit jamais un brouillon (aucun accès à ce module).
//! * E1/E2/E3/E6/E10 : voir les migrations 441 et 442 ; tout passe par `TenantTx` (RLS).

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ent_auth::{Action, Role};
use ent_events::AggregateKind;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, publier_agg, Acteur};
use crate::referentiel_structure::CreneauGrille;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

// ───────────────────────────── Types ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct VersionEdt {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub libelle: String,
    pub nature: String,
    pub base_version_id: Option<Uuid>,
    pub periode_libelle: Option<String>,
    pub etat: String,
    pub valable_du: Option<NaiveDate>,
    pub valable_au: Option<NaiveDate>,
    pub provisoire: bool,
    pub origine: String,
    pub publiee_le: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
}

const SELECT_VERSION: &str =
    "SELECT id, etablissement_id, annee_id, libelle, nature, base_version_id, periode_libelle, etat,
        valable_du, valable_au, provisoire, origine, publiee_le, cree_le
   FROM ent_edt_version";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementEdt {
    pub id: Uuid,
    pub version_id: Uuid,
    pub piece_id: Uuid,
    pub cours_id: Uuid,
    pub jour_semaine: i16,
    pub creneau_debut_code: Option<String>,
    pub duree_creneaux: Option<i16>,
    /// Placement « hors grille » (reprise) : heures libres.
    pub heure_debut_libre: Option<NaiveTime>,
    pub heure_fin_libre: Option<NaiveTime>,
    pub semaine_type: String,
    pub verrouille: bool,
    pub origine: String,
    pub creneau_id: Option<Uuid>,
    /// Lieux, le principal en premier.
    pub lieux: Vec<Uuid>,
}

#[derive(Debug, Clone, FromRow)]
struct PlacementRow {
    id: Uuid,
    version_id: Uuid,
    piece_id: Uuid,
    cours_id: Uuid,
    jour_semaine: i16,
    creneau_debut_code: Option<String>,
    duree_creneaux: Option<i16>,
    heure_debut_libre: Option<NaiveTime>,
    heure_fin_libre: Option<NaiveTime>,
    semaine_type: String,
    verrouille: bool,
    origine: String,
    creneau_id: Option<Uuid>,
}

const SELECT_PLACEMENT: &str = "SELECT id, etablissement_id, version_id, piece_id, cours_id, jour_semaine, creneau_debut_code,
        duree_creneaux, heure_debut_libre, heure_fin_libre, semaine_type, verrouille, origine, creneau_id
   FROM ent_edt_placement";

/// Création ou remplacement de la position d'un placement (brouillon).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionPlacement {
    pub jour_semaine: i16,
    /// Code de grille du premier créneau (« M2 »).
    pub creneau_debut_code: String,
    pub duree_creneaux: i16,
    /// `toutes` ou étiquette de semaine du socle 5 (`A`, `B`…).
    pub semaine_type: String,
    /// Lieux (le premier est le principal).
    pub lieux: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NouveauPlacement {
    pub version_id: Uuid,
    pub cours_id: Uuid,
    pub position: PositionPlacement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreneauSansCours {
    pub creneau_id: Uuid,
    pub classe: String,
    pub matiere_libelle: String,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportReprise {
    pub annee_id: Uuid,
    pub version_id: Uuid,
    /// La version de reprise vient d'être créée par cet appel.
    pub version_creee: bool,
    pub placements_crees: i64,
    pub placements_mis_a_jour: i64,
    pub placements_inchanges: i64,
    /// Placements retirés parce que leur créneau n'existe plus.
    pub placements_retires: i64,
    /// Créneaux repris aux heures libres faute de code de grille correspondant.
    pub hors_grille: i64,
    /// Créneaux dont la salle n'est pas dans le référentiel des lieux.
    pub sans_lieu: i64,
    /// Créneaux sans cours (jamais devinés) : liste à traiter par la gestion.
    pub sans_cours: Vec<CreneauSansCours>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatureConflitPlan {
    /// Deux placements du même cours se chevauchent.
    Cours,
    /// Un même intervenant (service du socle 6) est demandé sur deux placements simultanés.
    Intervenant,
    /// Un même lieu est occupé par deux placements simultanés.
    Lieu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflitPlan {
    pub nature: NatureConflitPlan,
    pub placement_a: Uuid,
    pub placement_b: Uuid,
    pub jour_semaine: i16,
}

// ───────────────────────────── Logique pure ─────────────────────────────

/// Fenêtre horaire d'un placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fenetre {
    pub debut: NaiveTime,
    pub fin: NaiveTime,
}

impl Fenetre {
    pub fn chevauche(&self, autre: &Fenetre) -> bool {
        self.debut < autre.fin && autre.debut < self.fin
    }
}

/// Les semaines types se recoupent-elles ? (`toutes` recoupe tout ; sinon même étiquette.)
pub fn semaines_se_recoupent(a: &str, b: &str) -> bool {
    a == "toutes" || b == "toutes" || a.eq_ignore_ascii_case(b)
}

/// Créneaux de grille de `site_id` (à défaut ceux de l'établissement), dans l'ordre.
fn grille_du_site(grille: &[CreneauGrille], site_id: Option<Uuid>) -> Vec<&CreneauGrille> {
    let mut v: Vec<&CreneauGrille> = site_id
        .map(|s| {
            grille
                .iter()
                .filter(|g| g.site_id == Some(s))
                .collect::<Vec<_>>()
        })
        .filter(|v: &Vec<&CreneauGrille>| !v.is_empty())
        .unwrap_or_else(|| grille.iter().filter(|g| g.site_id.is_none()).collect());
    v.sort_by_key(|g| g.ordre);
    v
}

/// Position en grille d'une plage horaire : `(code du premier créneau, nombre de créneaux)` si la plage
/// commence au début d'un créneau et finit à la fin d'un créneau consécutif ; sinon `None` (hors grille).
pub fn position_en_grille(
    grille: &[CreneauGrille],
    site_id: Option<Uuid>,
    debut: NaiveTime,
    fin: NaiveTime,
) -> Option<(String, i16)> {
    let g = grille_du_site(grille, site_id);
    let i0 = g.iter().position(|c| c.heure_debut == debut)?;
    let mut n = 0i16;
    for c in &g[i0..] {
        n += 1;
        if c.heure_fin == fin {
            return Some((g[i0].code.clone(), n));
        }
        if c.heure_fin > fin {
            return None;
        }
    }
    None
}

/// Fenêtre horaire d'un placement exprimé en codes de grille.
pub fn fenetre_en_grille(
    grille: &[CreneauGrille],
    site_id: Option<Uuid>,
    code: &str,
    duree: i16,
) -> Option<Fenetre> {
    let g = grille_du_site(grille, site_id);
    let i0 = g.iter().position(|c| c.code == code)?;
    let i1 = i0 + usize::try_from(duree).ok()?.checked_sub(1)?;
    let (premier, dernier) = (g.get(i0)?, g.get(i1)?);
    Some(Fenetre {
        debut: premier.heure_debut,
        fin: dernier.heure_fin,
    })
}

/// Un placement tel que le contrôle des conflits le voit.
#[derive(Debug, Clone)]
pub struct PlacementPourConflits {
    pub id: Uuid,
    pub cours_id: Uuid,
    pub jour_semaine: i16,
    pub semaine_type: String,
    pub fenetre: Fenetre,
    pub lieux: Vec<Uuid>,
    /// Intervenants (agents) des services du cours.
    pub intervenants: Vec<Uuid>,
}

/// Conflits d'un ensemble de placements (fonction pure, testée sans base).
pub fn conflits_du_plan(placements: &[PlacementPourConflits]) -> Vec<ConflitPlan> {
    let mut out = Vec::new();
    for (i, a) in placements.iter().enumerate() {
        for b in &placements[i + 1..] {
            if a.jour_semaine != b.jour_semaine
                || !semaines_se_recoupent(&a.semaine_type, &b.semaine_type)
                || !a.fenetre.chevauche(&b.fenetre)
            {
                continue;
            }
            let mut pousse = |nature| {
                out.push(ConflitPlan {
                    nature,
                    placement_a: a.id,
                    placement_b: b.id,
                    jour_semaine: a.jour_semaine,
                });
            };
            if a.cours_id == b.cours_id {
                pousse(NatureConflitPlan::Cours);
            }
            if a.intervenants.iter().any(|x| b.intervenants.contains(x)) {
                pousse(NatureConflitPlan::Intervenant);
            }
            if a.lieux.iter().any(|x| b.lieux.contains(x)) {
                pousse(NatureConflitPlan::Lieu);
            }
        }
    }
    out
}

/// Heures hebdomadaires d'un ensemble de plages (minutes → heures, arrondi au centième).
fn heures_hebdo(plages: &[(NaiveTime, NaiveTime)]) -> f64 {
    let minutes: i64 = plages.iter().map(|(d, f)| (*f - *d).num_minutes()).sum();
    ((minutes as f64 / 60.0) * 100.0).round() / 100.0
}

fn semaine_pour_plan(semaine_ab: &str) -> String {
    match semaine_ab.to_ascii_lowercase().as_str() {
        "a" => "A".to_string(),
        "b" => "B".to_string(),
        _ => "toutes".to_string(),
    }
}

fn est_personnel(role: &str) -> bool {
    Role::parse(role).is_some_and(|r| r.is_staff())
}

// ───────────────────────────── Service ─────────────────────────────

impl PgEnt {
    fn exiger_lecture_plan(&self, a: &Acteur<'_>) -> Result<(), EntDbError> {
        if est_personnel(a.role) {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Versions d'une année (la plus récente d'abord).
    pub async fn edt_versions(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<Vec<VersionEdt>, EntDbError> {
        self.exiger_lecture_plan(a)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        sqlx::query_as::<_, VersionEdt>(&format!(
            "{SELECT_VERSION} WHERE etablissement_id = $1 AND annee_id = $2 ORDER BY cree_le DESC, id"
        ))
        .bind(a.etablissement_id)
        .bind(annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    /// Crée une version en **brouillon**. Une version temporaire vise une version de base de la même
    /// année (§8.4) ; sa plage est posée à la publication (T5).
    pub async fn edt_version_creer(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        libelle: &str,
        base_version_id: Option<Uuid>,
        periode_libelle: Option<&str>,
    ) -> Result<VersionEdt, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let libelle = libelle.trim();
        if libelle.is_empty() || libelle.chars().count() > 160 {
            return Err(invalide());
        }
        let periode = periode_libelle.map(str::trim).filter(|p| !p.is_empty());
        if periode.is_some_and(|p| p.chars().count() > 120) {
            return Err(invalide());
        }
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let (annee_ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire
                             WHERE etablissement_id = $1 AND id = $2 AND statut <> 'cloturee')",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !annee_ok {
            return Err(introuvable());
        }
        let nature = if let Some(base) = base_version_id {
            let b: Option<(String,)> = sqlx::query_as(
                "SELECT nature FROM ent_edt_version WHERE etablissement_id = $1 AND id = $2 AND annee_id = $3",
            )
            .bind(e)
            .bind(base)
            .bind(annee_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match b {
                Some((n,)) if n == "base" => "temporaire",
                Some(_) => return Err(invalide()), // une temporaire se rattache à une version de base
                None => return Err(introuvable()),
            }
        } else {
            "base"
        };
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_version
               (id, etablissement_id, annee_id, libelle, nature, base_version_id, periode_libelle, origine, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,'saisie',$8)",
        )
        .bind(id)
        .bind(e)
        .bind(annee_id)
        .bind(libelle)
        .bind(nature)
        .bind(base_version_id)
        .bind(periode)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            "edt.version_creee",
            id,
            json!({ "version_id": id, "annee_id": annee_id, "nature": nature }),
        )
        .await?;
        let v = sqlx::query_as::<_, VersionEdt>(&format!(
            "{SELECT_VERSION} WHERE etablissement_id = $2 AND id = $1"
        ))
        .bind(id)
        .bind(e)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// Abandonne un brouillon (il reste en base pour l'historique, jamais supprimé).
    pub async fn edt_version_abandonner(
        &self,
        a: &Acteur<'_>,
        version_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let n = sqlx::query(
            "UPDATE ent_edt_version SET etat = 'abandonnee', modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'brouillon'",
        )
        .bind(e)
        .bind(version_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit()); // inconnue, ou déjà publiée : on ne l'abandonne pas
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Placements d'une version, lieux compris.
    pub async fn edt_placements(
        &self,
        a: &Acteur<'_>,
        version_id: Uuid,
    ) -> Result<Vec<PlacementEdt>, EntDbError> {
        self.exiger_lecture_plan(a)?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let rows = sqlx::query_as::<_, PlacementRow>(&format!(
            "{SELECT_PLACEMENT} WHERE etablissement_id = $1 AND version_id = $2
              ORDER BY jour_semaine, creneau_debut_code NULLS LAST, heure_debut_libre NULLS LAST, id"
        ))
        .bind(e)
        .bind(version_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let lieux: Vec<(Uuid, Uuid, bool)> = sqlx::query_as(
            "SELECT l.placement_id, l.lieu_id, l.principal
               FROM ent_edt_placement_lieu l
               JOIN ent_edt_placement p ON p.id = l.placement_id AND p.etablissement_id = l.etablissement_id
              WHERE l.etablissement_id = $1 AND p.version_id = $2
              ORDER BY l.principal DESC, l.lieu_id",
        )
        .bind(e)
        .bind(version_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut par_placement: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (p, l, _) in lieux {
            par_placement.entry(p).or_default().push(l);
        }
        Ok(rows
            .into_iter()
            .map(|r| PlacementEdt {
                lieux: par_placement.remove(&r.id).unwrap_or_default(),
                id: r.id,
                version_id: r.version_id,
                piece_id: r.piece_id,
                cours_id: r.cours_id,
                jour_semaine: r.jour_semaine,
                creneau_debut_code: r.creneau_debut_code,
                duree_creneaux: r.duree_creneaux,
                heure_debut_libre: r.heure_debut_libre,
                heure_fin_libre: r.heure_fin_libre,
                semaine_type: r.semaine_type,
                verrouille: r.verrouille,
                origine: r.origine,
                creneau_id: r.creneau_id,
            })
            .collect())
    }

    /// Contrôles communs d'une position (version en brouillon, code de grille et durée valides, lieux
    /// de l'établissement). Retourne la version.
    async fn valider_position(
        tx: &mut sqlx::PgConnection,
        e: Uuid,
        version_id: Uuid,
        pos: &PositionPlacement,
    ) -> Result<VersionEdt, EntDbError> {
        let v = sqlx::query_as::<_, VersionEdt>(&format!(
            "{SELECT_VERSION} WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(e)
        .bind(version_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if v.etat != "brouillon" {
            return Err(conflit()); // une version publiée ne se modifie pas : nouveau brouillon
        }
        if !(1..=7).contains(&pos.jour_semaine)
            || !(1..=12).contains(&pos.duree_creneaux)
            || pos.semaine_type.trim().is_empty()
            || pos.semaine_type.chars().count() > 20
            || pos.lieux.len() > 6
            || pos.lieux.iter().collect::<HashSet<_>>().len() != pos.lieux.len()
        {
            return Err(invalide());
        }
        // Le code existe dans la grille (établissement ou un site) et la durée tient dans la journée.
        let grille: Vec<CreneauGrille> = sqlx::query_as(
            "SELECT id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin
               FROM ent_grille_horaire WHERE etablissement_id = $1 AND code = $2",
        )
        .bind(e)
        .bind(&pos.creneau_debut_code)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if grille.is_empty() {
            return Err(invalide());
        }
        let toute: Vec<CreneauGrille> = sqlx::query_as(
            "SELECT id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin
               FROM ent_grille_horaire WHERE etablissement_id = $1",
        )
        .bind(e)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let site_ids: HashSet<Option<Uuid>> = grille.iter().map(|g| g.site_id).collect();
        if !site_ids.iter().any(|s| {
            fenetre_en_grille(&toute, *s, &pos.creneau_debut_code, pos.duree_creneaux).is_some()
        }) {
            return Err(invalide());
        }
        if !pos.lieux.is_empty() {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_salle WHERE etablissement_id = $1 AND id = ANY($2)",
            )
            .bind(e)
            .bind(&pos.lieux)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if n as usize != pos.lieux.len() {
                return Err(invalide());
            }
        }
        Ok(v)
    }

    async fn poser_lieux(
        tx: &mut sqlx::PgConnection,
        e: Uuid,
        placement_id: Uuid,
        lieux: &[Uuid],
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "DELETE FROM ent_edt_placement_lieu WHERE etablissement_id = $1 AND placement_id = $2",
        )
        .bind(e)
        .bind(placement_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (i, l) in lieux.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_edt_placement_lieu (placement_id, etablissement_id, lieu_id, principal)
                 VALUES ($1,$2,$3,$4)",
            )
            .bind(placement_id)
            .bind(e)
            .bind(l)
            .bind(i == 0)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        Ok(())
    }

    /// Crée un placement dans un **brouillon** : besoin et pièce sont créés au besoin. Les conflits ne
    /// bloquent pas (voir [`Self::edt_version_conflits`]).
    pub async fn edt_placement_creer(
        &self,
        a: &Acteur<'_>,
        cmd: NouveauPlacement,
    ) -> Result<PlacementEdt, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let v = Self::valider_position(&mut *tx, e, cmd.version_id, &cmd.position).await?;
        // Le cours existe, est de la même année et n'est pas fermé.
        let cours: Option<(Option<f64>,)> = sqlx::query_as(
            "SELECT heures_hebdo::float8 FROM ent_cours
              WHERE etablissement_id = $1 AND id = $2 AND annee_id = $3 AND ferme_le IS NULL",
        )
        .bind(e)
        .bind(cmd.cours_id)
        .bind(v.annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((heures_cours,)) = cours else {
            return Err(introuvable());
        };
        let besoin: Uuid = match sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM ent_edt_besoin WHERE etablissement_id = $1 AND version_id = $2 AND cours_id = $3",
        )
        .bind(e)
        .bind(cmd.version_id)
        .bind(cmd.cours_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        {
            Some(b) => b,
            None => {
                let b = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_edt_besoin (id, etablissement_id, version_id, annee_id, cours_id, heures_hebdo, source)
                     VALUES ($1,$2,$3,$4,$5,$6::float8::numeric,'cours')",
                )
                .bind(b)
                .bind(e)
                .bind(cmd.version_id)
                .bind(v.annee_id)
                .bind(cmd.cours_id)
                .bind(heures_cours.unwrap_or(0.0))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                b
            }
        };
        let ordre: i16 = sqlx::query_scalar(
            "SELECT (coalesce(max(ordre), 0) + 1)::smallint FROM ent_edt_piece WHERE etablissement_id = $1 AND besoin_id = $2",
        )
        .bind(e)
        .bind(besoin)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let piece = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_piece (id, etablissement_id, besoin_id, duree_creneaux, semaine_type, ordre)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(piece)
        .bind(e)
        .bind(besoin)
        .bind(cmd.position.duree_creneaux)
        .bind(cmd.position.semaine_type.trim())
        .bind(ordre)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_placement
               (id, etablissement_id, version_id, piece_id, cours_id, jour_semaine, creneau_debut_code,
                duree_creneaux, semaine_type, origine, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'main',$10)",
        )
        .bind(id)
        .bind(e)
        .bind(cmd.version_id)
        .bind(piece)
        .bind(cmd.cours_id)
        .bind(cmd.position.jour_semaine)
        .bind(&cmd.position.creneau_debut_code)
        .bind(cmd.position.duree_creneaux)
        .bind(cmd.position.semaine_type.trim())
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Self::poser_lieux(&mut *tx, e, id, &cmd.position.lieux).await?;
        tx.commit().await.map_err(db)?;
        self.edt_placement_lire(a, id).await
    }

    async fn edt_placement_lire(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<PlacementEdt, EntDbError> {
        let e = a.etablissement_id;
        let version: Uuid = sqlx::query_scalar(
            "SELECT version_id FROM ent_edt_placement WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), e).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        self.edt_placements(a, version)
            .await?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(introuvable)
    }

    /// Déplace un placement d'un **brouillon** (jour, créneaux, semaine, lieux). Refusé s'il est verrouillé.
    pub async fn edt_placement_deplacer(
        &self,
        a: &Acteur<'_>,
        placement_id: Uuid,
        pos: PositionPlacement,
    ) -> Result<PlacementEdt, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let (version_id, piece_id, verrouille): (Uuid, Uuid, bool) = sqlx::query_as(
            "SELECT version_id, piece_id, verrouille FROM ent_edt_placement
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(e)
        .bind(placement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if verrouille {
            return Err(conflit());
        }
        Self::valider_position(&mut *tx, e, version_id, &pos).await?;
        sqlx::query(
            "UPDATE ent_edt_placement
                SET jour_semaine = $3, creneau_debut_code = $4, duree_creneaux = $5, semaine_type = $6,
                    heure_debut_libre = NULL, heure_fin_libre = NULL, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(placement_id)
        .bind(pos.jour_semaine)
        .bind(&pos.creneau_debut_code)
        .bind(pos.duree_creneaux)
        .bind(pos.semaine_type.trim())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_edt_piece SET duree_creneaux = $3, semaine_type = $4, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(piece_id)
        .bind(pos.duree_creneaux)
        .bind(pos.semaine_type.trim())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Self::poser_lieux(&mut *tx, e, placement_id, &pos.lieux).await?;
        tx.commit().await.map_err(db)?;
        self.edt_placement_lire(a, placement_id).await
    }

    /// Retire un placement d'un **brouillon** (sa pièce, et son besoin s'il ne reste plus de pièce).
    /// Jamais sur une version publiée ni sur un placement verrouillé.
    pub async fn edt_placement_retirer(
        &self,
        a: &Acteur<'_>,
        placement_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let row: Option<(Uuid, bool, String)> = sqlx::query_as(
            "SELECT p.piece_id, p.verrouille, v.etat
               FROM ent_edt_placement p JOIN ent_edt_version v ON v.id = p.version_id AND v.etablissement_id = p.etablissement_id
              WHERE p.etablissement_id = $1 AND p.id = $2 FOR UPDATE OF p",
        )
        .bind(e)
        .bind(placement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((piece_id, verrouille, etat)) = row else {
            return Err(introuvable());
        };
        if verrouille || etat != "brouillon" {
            return Err(conflit());
        }
        Self::supprimer_placement_et_orphelins(&mut *tx, e, placement_id, piece_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    async fn supprimer_placement_et_orphelins(
        tx: &mut sqlx::PgConnection,
        e: Uuid,
        placement_id: Uuid,
        piece_id: Uuid,
    ) -> Result<(), EntDbError> {
        let besoin: Option<Uuid> = sqlx::query_scalar(
            "SELECT besoin_id FROM ent_edt_piece WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(piece_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("DELETE FROM ent_edt_placement WHERE etablissement_id = $1 AND id = $2")
            .bind(e)
            .bind(placement_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "DELETE FROM ent_edt_piece p WHERE p.etablissement_id = $1 AND p.id = $2
                AND NOT EXISTS (SELECT 1 FROM ent_edt_placement x
                                 WHERE x.piece_id = p.id AND x.etablissement_id = p.etablissement_id)",
        )
        .bind(e)
        .bind(piece_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(b) = besoin {
            sqlx::query(
                "DELETE FROM ent_edt_besoin b WHERE b.etablissement_id = $1 AND b.id = $2
                    AND NOT EXISTS (SELECT 1 FROM ent_edt_piece p
                                     WHERE p.besoin_id = b.id AND p.etablissement_id = b.etablissement_id)",
            )
            .bind(e)
            .bind(b)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        Ok(())
    }

    /// Conflits d'un plan (cours, intervenants des services du socle 6, lieux) : avertissements.
    pub async fn edt_version_conflits(
        &self,
        a: &Acteur<'_>,
        version_id: Uuid,
    ) -> Result<Vec<ConflitPlan>, EntDbError> {
        self.exiger_lecture_plan(a)?;
        let e = a.etablissement_id;
        let placements = self.edt_placements(a, version_id).await?;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let (annee_id,): (Uuid,) = sqlx::query_as(
            "SELECT annee_id FROM ent_edt_version WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(version_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let grille: Vec<CreneauGrille> = sqlx::query_as(
            "SELECT id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin
               FROM ent_grille_horaire WHERE etablissement_id = $1",
        )
        .bind(e)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // Site d'un cours : celui de sa classe, ou (groupe) celui de la classe du groupe n'est pas connu ici :
        // la grille de l'établissement est le repli.
        let sites: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT c.id, cl.site_id FROM ent_cours c
               LEFT JOIN ent_classe cl ON cl.id = c.classe_id AND cl.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.annee_id = $2",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let site_du_cours: HashMap<Uuid, Option<Uuid>> = sites.into_iter().collect();
        let interv: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT s.cours_id, s.prof_user_id FROM ent_service_enseignement s
              WHERE s.etablissement_id = $1 AND s.cours_id IS NOT NULL AND s.annee_id = $2
                AND s.etat = 'ouvert'",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut agents_du_cours: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (c, ag) in interv {
            agents_du_cours.entry(c).or_default().push(ag);
        }
        let pour: Vec<PlacementPourConflits> = placements
            .iter()
            .filter_map(|p| {
                let fenetre = match (&p.creneau_debut_code, p.duree_creneaux) {
                    (Some(code), Some(d)) => {
                        let site = site_du_cours.get(&p.cours_id).copied().flatten();
                        fenetre_en_grille(&grille, site, code, d)
                            .or_else(|| fenetre_en_grille(&grille, None, code, d))?
                    }
                    _ => Fenetre {
                        debut: p.heure_debut_libre?,
                        fin: p.heure_fin_libre?,
                    },
                };
                Some(PlacementPourConflits {
                    id: p.id,
                    cours_id: p.cours_id,
                    jour_semaine: p.jour_semaine,
                    semaine_type: p.semaine_type.clone(),
                    fenetre,
                    lieux: p.lieux.clone(),
                    intervenants: agents_du_cours
                        .get(&p.cours_id)
                        .cloned()
                        .unwrap_or_default(),
                })
            })
            .collect();
        Ok(conflits_du_plan(&pour))
    }

    // ───────────────────────────── Reprise des créneaux (M1–M2) ─────────────────────────────

    /// Reprend les créneaux de l'année dans la version de base « reprise » (publiée sur l'année).
    /// **Rejouable** : un second appel ne change rien ; un créneau modifié met son placement à jour ;
    /// un créneau supprimé retire son placement. Les créneaux sans cours sont listés, jamais devinés.
    pub async fn edt_reprendre_creneaux(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<RapportReprise, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let Some(rapport) = Self::reprendre_creneaux_tx(&mut *tx, a, annee_id).await? else {
            return Err(conflit()); // une autre version de base est déjà publiée sur l'année
        };
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            "edt.creneaux_repris",
            rapport.version_id,
            json!({ "version_id": rapport.version_id, "annee_id": annee_id, "crees": rapport.placements_crees,
                    "mis_a_jour": rapport.placements_mis_a_jour, "retires": rapport.placements_retires,
                    "sans_cours": rapport.sans_cours.len(), "hors_grille": rapport.hors_grille }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(rapport)
    }

    /// Cœur de la reprise, dans la transaction de l'appelant (la génération des séances l'appelle avant
    /// d'enrichir les séances). `None` : une version de base AUTRE que la reprise est publiée sur
    /// l'année — la reprise n'a plus lieu d'être (les séances ne sont plus rattachées par ce chemin).
    pub(crate) async fn reprendre_creneaux_tx(
        tx: &mut sqlx::PgConnection,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<Option<RapportReprise>, EntDbError> {
        let e = a.etablissement_id;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 2))")
            .bind(format!("edt-reprise-{e}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let annee: Option<(NaiveDate, NaiveDate, String)> = sqlx::query_as(
            "SELECT date_debut, date_fin, statut FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((debut, fin, statut)) = annee else {
            return Err(introuvable());
        };
        if statut == "cloturee" {
            return Err(conflit());
        }
        let mut rapport = RapportReprise {
            annee_id,
            ..Default::default()
        };

        let existante: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_edt_version
              WHERE etablissement_id = $1 AND annee_id = $2 AND origine = 'reprise' AND nature = 'base'
                AND etat = 'publiee'",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let version_id = match existante {
            Some(v) => v,
            None => {
                // Une autre version de base est déjà publiée sur l'année : pas de reprise.
                let autre: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_edt_version
                                     WHERE etablissement_id = $1 AND annee_id = $2
                                       AND nature = 'base' AND etat = 'publiee')",
                )
                .bind(e)
                .bind(annee_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if autre {
                    return Ok(None);
                }
                let v = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_edt_version
                       (id, etablissement_id, annee_id, libelle, nature, etat, valable_du, valable_au, origine,
                        publiee_par, publiee_le, cree_par)
                     VALUES ($1,$2,$3,'Reprise de l''emploi du temps existant','base','publiee',$4,$5,'reprise',$6,now(),$6)",
                )
                .bind(v)
                .bind(e)
                .bind(annee_id)
                .bind(debut)
                .bind(fin)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                rapport.version_creee = true;
                v
            }
        };
        rapport.version_id = version_id;

        #[derive(FromRow)]
        struct Src {
            id: Uuid,
            cours_id: Option<Uuid>,
            classe: String,
            matiere_libelle: String,
            site_id: Option<Uuid>,
            jour_semaine: i16,
            heure_debut: NaiveTime,
            heure_fin: NaiveTime,
            salle: Option<String>,
            semaine_ab: String,
        }
        let sources: Vec<Src> = sqlx::query_as(
            "SELECT c.id, c.cours_id, btrim(c.classe) AS classe, c.matiere_libelle, cl.site_id,
                    c.jour_semaine, c.heure_debut, c.heure_fin, c.salle, c.semaine_ab
               FROM ent_edt_creneau c
               JOIN ent_classe cl
                 ON cl.etablissement_id = c.etablissement_id AND cl.annee_id = $2
                AND cl.code = btrim(c.classe) AND cl.archivee_le IS NULL
              WHERE c.etablissement_id = $1
              ORDER BY c.id",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let grille: Vec<CreneauGrille> = sqlx::query_as(
            "SELECT id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin
               FROM ent_grille_horaire WHERE etablissement_id = $1",
        )
        .bind(e)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let salles: Vec<(String, Uuid)> = sqlx::query_as(
            "SELECT lower(btrim(code)), id FROM ent_salle WHERE etablissement_id = $1",
        )
        .bind(e)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let salle_id: HashMap<String, Uuid> = salles.into_iter().collect();

        // Les cours doivent exister dans l'année (un créneau relié à un cours d'une autre année est listé).
        let cours_valides: HashSet<Uuid> = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM ent_cours WHERE etablissement_id = $1 AND annee_id = $2",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();

        let mut par_cours: BTreeMap<Uuid, Vec<&Src>> = BTreeMap::new();
        for s in &sources {
            match s.cours_id.filter(|c| cours_valides.contains(c)) {
                Some(c) => par_cours.entry(c).or_default().push(s),
                None => rapport.sans_cours.push(CreneauSansCours {
                    creneau_id: s.id,
                    classe: s.classe.clone(),
                    matiere_libelle: s.matiere_libelle.clone(),
                    jour_semaine: s.jour_semaine,
                    heure_debut: s.heure_debut,
                    heure_fin: s.heure_fin,
                }),
            }
        }

        let mut creneaux_vivants: HashSet<Uuid> = HashSet::new();
        for (cours_id, creneaux) in &par_cours {
            let heures = heures_hebdo(
                &creneaux
                    .iter()
                    .map(|c| (c.heure_debut, c.heure_fin))
                    .collect::<Vec<_>>(),
            )
            .clamp(0.0, 40.0);
            let besoin: Uuid = match sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM ent_edt_besoin WHERE etablissement_id = $1 AND version_id = $2 AND cours_id = $3",
            )
            .bind(e)
            .bind(version_id)
            .bind(cours_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            {
                Some(b) => {
                    sqlx::query(
                        "UPDATE ent_edt_besoin SET heures_hebdo = $2::float8::numeric, modifie_le = now()
                          WHERE etablissement_id = $3 AND id = $1 AND heures_hebdo IS DISTINCT FROM $2::float8::numeric",
                    )
                    .bind(b)
                    .bind(heures)
                    .bind(e)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    b
                }
                None => {
                    let b = Uuid::new_v4();
                    sqlx::query(
                        "INSERT INTO ent_edt_besoin (id, etablissement_id, version_id, annee_id, cours_id, heures_hebdo, source)
                         VALUES ($1,$2,$3,$4,$5,$6::float8::numeric,'ajuste')",
                    )
                    .bind(b)
                    .bind(e)
                    .bind(version_id)
                    .bind(annee_id)
                    .bind(cours_id)
                    .bind(heures)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    b
                }
            };
            for (ordre, c) in creneaux.iter().enumerate() {
                creneaux_vivants.insert(c.id);
                let semaine = semaine_pour_plan(&c.semaine_ab);
                let position = position_en_grille(&grille, c.site_id, c.heure_debut, c.heure_fin);
                if position.is_none() {
                    rapport.hors_grille += 1;
                }
                let (code, duree, libre_d, libre_f) = match &position {
                    Some((code, n)) => (Some(code.clone()), Some(*n), None, None),
                    None => (None, None, Some(c.heure_debut), Some(c.heure_fin)),
                };
                let lieu: Option<Uuid> = c
                    .salle
                    .as_deref()
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .and_then(|s| salle_id.get(&s).copied());
                if c.salle.as_deref().is_some_and(|s| !s.trim().is_empty()) && lieu.is_none() {
                    rapport.sans_lieu += 1;
                }
                let courant: Option<(Uuid, Uuid, i16, Option<String>, Option<i16>, Option<NaiveTime>, Option<NaiveTime>, String)> =
                    sqlx::query_as(
                        "SELECT id, piece_id, jour_semaine, creneau_debut_code, duree_creneaux, heure_debut_libre,
                                heure_fin_libre, semaine_type
                           FROM ent_edt_placement
                          WHERE etablissement_id = $1 AND version_id = $2 AND creneau_id = $3",
                    )
                    .bind(e)
                    .bind(version_id)
                    .bind(c.id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
                let lieux_voulus: Vec<Uuid> = lieu.into_iter().collect();
                match courant {
                    None => {
                        let piece = Uuid::new_v4();
                        sqlx::query(
                            "INSERT INTO ent_edt_piece (id, etablissement_id, besoin_id, duree_creneaux, semaine_type, ordre)
                             VALUES ($1,$2,$3,$4,$5,$6)",
                        )
                        .bind(piece)
                        .bind(e)
                        .bind(besoin)
                        .bind(duree.unwrap_or(1))
                        .bind(&semaine)
                        .bind(i16::try_from(ordre + 1).unwrap_or(1))
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                        let pid = Uuid::new_v4();
                        sqlx::query(
                            "INSERT INTO ent_edt_placement
                               (id, etablissement_id, version_id, piece_id, cours_id, jour_semaine, creneau_debut_code,
                                duree_creneaux, heure_debut_libre, heure_fin_libre, semaine_type, origine, creneau_id, cree_par)
                             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'reprise',$12,$13)",
                        )
                        .bind(pid)
                        .bind(e)
                        .bind(version_id)
                        .bind(piece)
                        .bind(cours_id)
                        .bind(c.jour_semaine)
                        .bind(&code)
                        .bind(duree)
                        .bind(libre_d)
                        .bind(libre_f)
                        .bind(&semaine)
                        .bind(c.id)
                        .bind(a.user_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                        Self::poser_lieux(&mut *tx, e, pid, &lieux_voulus).await?;
                        rapport.placements_crees += 1;
                    }
                    Some((pid, piece, jour, code_c, duree_c, ld_c, lf_c, sem_c)) => {
                        let lieux_actuels: Vec<Uuid> = sqlx::query_scalar(
                            "SELECT lieu_id FROM ent_edt_placement_lieu WHERE etablissement_id = $1 AND placement_id = $2",
                        )
                        .bind(e)
                        .bind(pid)
                        .fetch_all(&mut *tx)
                        .await
                        .map_err(db)?;
                        let identique = jour == c.jour_semaine
                            && code_c == code
                            && duree_c == duree
                            && ld_c == libre_d
                            && lf_c == libre_f
                            && sem_c == semaine
                            && lieux_actuels == lieux_voulus;
                        if identique {
                            rapport.placements_inchanges += 1;
                        } else {
                            sqlx::query(
                                "UPDATE ent_edt_placement
                                    SET jour_semaine = $3, creneau_debut_code = $4, duree_creneaux = $5,
                                        heure_debut_libre = $6, heure_fin_libre = $7, semaine_type = $8,
                                        cours_id = $9, modifie_le = now()
                                  WHERE etablissement_id = $1 AND id = $2",
                            )
                            .bind(e)
                            .bind(pid)
                            .bind(c.jour_semaine)
                            .bind(&code)
                            .bind(duree)
                            .bind(libre_d)
                            .bind(libre_f)
                            .bind(&semaine)
                            .bind(cours_id)
                            .execute(&mut *tx)
                            .await
                            .map_err(db)?;
                            sqlx::query(
                                "UPDATE ent_edt_piece SET duree_creneaux = $3, semaine_type = $4, besoin_id = $5, modifie_le = now()
                                  WHERE etablissement_id = $1 AND id = $2",
                            )
                            .bind(e)
                            .bind(piece)
                            .bind(duree.unwrap_or(1))
                            .bind(&semaine)
                            .bind(besoin)
                            .execute(&mut *tx)
                            .await
                            .map_err(db)?;
                            Self::poser_lieux(&mut *tx, e, pid, &lieux_voulus).await?;
                            rapport.placements_mis_a_jour += 1;
                        }
                    }
                }
            }
        }

        // Créneaux disparus (ou devenus « sans cours ») : leur placement de reprise est retiré.
        let vivants: Vec<Uuid> = creneaux_vivants.into_iter().collect();
        let a_retirer: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, piece_id FROM ent_edt_placement
              WHERE etablissement_id = $1 AND version_id = $2 AND origine = 'reprise'
                AND (creneau_id IS NULL OR NOT (creneau_id = ANY($3)))",
        )
        .bind(e)
        .bind(version_id)
        .bind(&vivants)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (pid, piece) in &a_retirer {
            Self::supprimer_placement_et_orphelins(&mut *tx, e, *pid, *piece).await?;
            rapport.placements_retires += 1;
        }
        // Besoins devenus vides (plus aucune pièce) : retirés.
        sqlx::query(
            "DELETE FROM ent_edt_besoin b WHERE b.etablissement_id = $1 AND b.version_id = $2
                AND NOT EXISTS (SELECT 1 FROM ent_edt_piece p WHERE p.besoin_id = b.id AND p.etablissement_id = b.etablissement_id)",
        )
        .bind(e)
        .bind(version_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;

        Ok(Some(rapport))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(a: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(a, m, 0).unwrap()
    }

    fn grille() -> Vec<CreneauGrille> {
        let mk = |code: &str, ordre: i16, d: (u32, u32), f: (u32, u32)| CreneauGrille {
            id: Uuid::new_v4(),
            site_id: None,
            code: code.into(),
            libelle: code.into(),
            demi_journee: "matin".into(),
            ordre,
            heure_debut: h(d.0, d.1),
            heure_fin: h(f.0, f.1),
        };
        vec![
            mk("M1", 1, (8, 0), (9, 0)),
            mk("M2", 2, (9, 0), (10, 0)),
            mk("M3", 3, (10, 15), (11, 15)),
        ]
    }

    #[test]
    fn position_en_grille_simple_double_et_hors_grille() {
        let g = grille();
        assert_eq!(
            position_en_grille(&g, None, h(8, 0), h(9, 0)),
            Some(("M1".into(), 1))
        );
        assert_eq!(
            position_en_grille(&g, None, h(8, 0), h(10, 0)),
            Some(("M1".into(), 2))
        );
        assert_eq!(
            position_en_grille(&g, None, h(9, 0), h(11, 15)),
            Some(("M2".into(), 2))
        );
        // Début décalé ou fin qui tombe au milieu d'un créneau : hors grille.
        assert_eq!(position_en_grille(&g, None, h(8, 15), h(9, 0)), None);
        assert_eq!(position_en_grille(&g, None, h(8, 0), h(9, 30)), None);
        // Fenêtre d'un placement en codes.
        let f = fenetre_en_grille(&g, None, "M1", 2).unwrap();
        assert_eq!((f.debut, f.fin), (h(8, 0), h(10, 0)));
        assert!(
            fenetre_en_grille(&g, None, "M3", 2).is_none(),
            "dépasse la journée"
        );
        assert!(fenetre_en_grille(&g, None, "Z9", 1).is_none());
    }

    #[test]
    fn semaines_et_heures() {
        assert!(semaines_se_recoupent("toutes", "A"));
        assert!(semaines_se_recoupent("A", "a"));
        assert!(!semaines_se_recoupent("A", "B"));
        assert_eq!(
            heures_hebdo(&[(h(8, 0), h(9, 0)), (h(9, 0), h(10, 30))]),
            2.5
        );
        assert_eq!(semaine_pour_plan("a"), "A");
        assert_eq!(semaine_pour_plan("toutes"), "toutes");
    }

    fn p(
        cours: Uuid,
        jour: i16,
        sem: &str,
        d: (u32, u32),
        f: (u32, u32),
        lieu: Option<Uuid>,
        agent: Option<Uuid>,
    ) -> PlacementPourConflits {
        PlacementPourConflits {
            id: Uuid::new_v4(),
            cours_id: cours,
            jour_semaine: jour,
            semaine_type: sem.into(),
            fenetre: Fenetre {
                debut: h(d.0, d.1),
                fin: h(f.0, f.1),
            },
            lieux: lieu.into_iter().collect(),
            intervenants: agent.into_iter().collect(),
        }
    }

    #[test]
    fn conflits_cours_intervenant_lieu() {
        let (c1, c2, l, ag) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let a = p(c1, 1, "toutes", (8, 0), (9, 0), Some(l), Some(ag));
        // Même agent et même lieu, cours différent, même créneau.
        let b = p(c2, 1, "toutes", (8, 30), (9, 30), Some(l), Some(ag));
        // Semaine B contre semaine A : compatibles ; autre jour : compatibles.
        let c = p(c2, 1, "A", (8, 0), (9, 0), Some(Uuid::new_v4()), None);
        let d = p(c1, 1, "B", (8, 0), (9, 0), None, None);
        let e = p(c1, 2, "toutes", (8, 0), (9, 0), Some(l), Some(ag));
        let r = conflits_du_plan(&[a.clone(), b.clone(), c.clone(), d.clone(), e.clone()]);
        let natures: Vec<_> = r
            .iter()
            .filter(|x| x.placement_a == a.id && x.placement_b == b.id)
            .map(|x| x.nature)
            .collect();
        assert!(natures.contains(&NatureConflitPlan::Intervenant));
        assert!(natures.contains(&NatureConflitPlan::Lieu));
        assert!(!natures.contains(&NatureConflitPlan::Cours));
        // a (toutes) et d (B, même cours) : conflit de cours ; d et c (A / B) : aucun.
        assert!(r.iter().any(|x| x.nature == NatureConflitPlan::Cours
            && x.placement_a == a.id
            && x.placement_b == d.id));
        assert!(!r
            .iter()
            .any(|x| (x.placement_a == c.id && x.placement_b == d.id) || x.placement_b == e.id));
        // Plages contiguës (8-9 / 9-10) : pas de chevauchement.
        let f = p(c1, 1, "toutes", (9, 0), (10, 0), Some(l), Some(ag));
        assert!(conflits_du_plan(&[a, f]).is_empty());
    }
}
