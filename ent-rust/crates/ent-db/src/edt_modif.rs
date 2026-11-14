//! Socle 10 (emploi du temps), lot T3 : modifier le réel d'une séance (EDT-160 à 168, 173) —
//! `docs/socles/10_EMPLOI_DU_TEMPS.md` §9, §21.3.
//!
//! **Une seule commande, un seul journal.** [`PgEnt::edt_modifier_seance`] applique un changement
//! (annulation, déplacement, changement de lieu ou d'horaire, ajout, fusion, neutralisation, changement
//! d'intervenant) à une séance datée, après les contrôles du §9.3 (public, intervenants et lieu libres, jour
//! ouvré, capacité) ; un conflit **dur** bloque sauf **forçage motivé** de la direction (`edt.forcer_conflit`),
//! journalisé avec les codes des conflits forcés ; un avertissement s'affiche sans bloquer. Chaque changement
//! laisse une ligne de `ent_edt_modification` (état avant et après, motif PUBLIC, origine, clé d'idempotence) ;
//! le motif INTERNE est dans `ent_edt_modification_interne`, lu seulement par la direction et le secrétariat.
//!
//! **Compatibilité (pont).** Les lecteurs historiques (semaine, appel, délégations, notifications) lisent
//! `ent_edt_exception` et `ent_edt_deplacement` ; ces tables restent la projection que le socle 10 écrit dans la
//! même transaction que le journal, et les anciens écrivains (socle 15 compris) journalisent à leur tour
//! (`pont_journaliser`). Le retrait de la projection est le lot T11.
//!
//! **Le passé ne change pas** (§9.6) : une séance dont l'heure de fin est dépassée n'est modifiable que par
//! [`PgEnt::edt_corriger_seance_passee`] (droit spécial, motif interne obligatoire), qui signale ses effets en
//! aval (appel déjà fait, cahier, travail) sans les défaire.
//!
//! Heure de Paris : « passé » se juge sur `fin_utc` (heure murale de l'établissement convertie dans son fuseau
//! par le déclencheur de 443) ; « aujourd'hui » est le jour de l'établissement, jamais le jour UTC.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use chrono::{NaiveDate, NaiveTime};
use ent_auth::Action;
use ent_events::AggregateKind;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::edt_modif_ops::{CiblePlacement, Placement};
use crate::ref_commun::{conflit, db, introuvable, invalide, publier_agg, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Liste courte des motifs PUBLICS (§9.1). Le texte libre qui l'accompagne est relu, ≤ 200 caractères.
pub const MOTIFS_PUBLICS: [&str; 8] = [
    "absence_professeur",
    "sortie",
    "examen",
    "reunion",
    "travaux",
    "intemperies",
    "mouvement_social",
    "autre",
];

/// Origines admises d'une modification saisie (les autres sont posées par le système).
pub const ORIGINES_SAISIE: [&str; 6] = [
    "saisie",
    "absence",
    "fermeture",
    "sortie",
    "examen",
    "calendrier",
];

/// Événements du §18.1 émis par le lot T3 (le lot T4 les consomme).
pub const EVT_SEANCE_MODIFIEE: &str = "edt.seance_modifiee";
pub const EVT_SEANCE_ANNULEE: &str = "edt.seance_annulee";
pub const EVT_SEANCE_RETABLIE: &str = "edt.seance_retablie";
pub const EVT_SEANCE_AJOUTEE: &str = "edt.seance_ajoutee";
pub const EVT_LOT_MODIFICATIONS: &str = "edt.lot_modifications";

/// Un changement de séance (§9.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChangementSeance {
    Annulation,
    /// Absorbée par un événement qui remplace les cours (examen, sortie de tout le public, journée banalisée).
    Neutralisation,
    Deplacement {
        nouveau_jour: NaiveDate,
        heure_debut: NaiveTime,
        heure_fin: NaiveTime,
        #[serde(default)]
        lieu_id: Option<Uuid>,
    },
    ChangementLieu {
        lieu_id: Uuid,
    },
    ChangementHoraire {
        heure_debut: NaiveTime,
        heure_fin: NaiveTime,
    },
    /// Remplacement ponctuel (le socle 15 y passe) : agit par la commande de remplacement historique.
    Intervenant {
        remplacant_user_id: Uuid,
    },
    /// La séance de la commande est conservée ; `avec_seance_id` y est absorbée.
    Fusion {
        avec_seance_id: Uuid,
    },
    /// Remplacement par un **autre cours** (EDT-166, CL-12) : la séance d'origine est annulée, une séance du cours
    /// de remplacement est ajoutée au même jour, aux mêmes heures et dans le même lieu (le collègue de français
    /// fait français à la place des maths). Contrôles du §9.3 sur la séance ajoutée.
    AutreCours {
        cours_id: Uuid,
    },
    /// Séance hors plan (rattrapage, soutien, devoir surveillé). `seance_id` de la commande reste vide.
    Ajout {
        cours_id: Uuid,
        jour: NaiveDate,
        heure_debut: NaiveTime,
        heure_fin: NaiveTime,
        #[serde(default)]
        lieu_id: Option<Uuid>,
    },
}

impl ChangementSeance {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Annulation => "annulation",
            Self::Neutralisation => "neutralisation",
            Self::Deplacement { .. } => "deplacement",
            Self::ChangementLieu { .. } => "changement_lieu",
            Self::ChangementHoraire { .. } => "changement_horaire",
            Self::Intervenant { .. } => "intervenant",
            Self::Fusion { .. } => "fusion",
            Self::Ajout { .. } => "ajout",
            // La clé d'idempotence est portée par l'annulation de la séance d'origine.
            Self::AutreCours { .. } => "annulation",
        }
    }

    fn action(&self) -> Action {
        match self {
            Self::ChangementLieu { .. } => Action::EdtModifierLieu,
            _ => Action::EdtModifier,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CmdModifierSeance {
    /// Séance visée ; vide seulement pour un `Ajout`.
    #[serde(default)]
    pub seance_id: Option<Uuid>,
    pub changement: ChangementSeance,
    pub motif_public_code: String,
    #[serde(default)]
    pub motif_public: Option<String>,
    /// Motif interne : lu seulement par la direction et le secrétariat, jamais diffusé.
    #[serde(default)]
    pub motif_interne: Option<String>,
    #[serde(default)]
    pub origine: Option<String>,
    #[serde(default)]
    pub origine_ref: Option<Uuid>,
    /// Clé d'idempotence : deux envois de la même clé = une seule modification.
    #[serde(default)]
    pub cle: Option<String>,
    /// Motif de forçage des conflits durs (direction seulement).
    #[serde(default)]
    pub forcer: Option<String>,
    /// Aperçu : les contrôles sont faits, rien n'est écrit.
    #[serde(default)]
    pub apercu: bool,
}

/// Un conflit ou un avertissement (§9.3). Aucun nom : le code, un décompte, la séance en face.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflitModif {
    pub code: String,
    pub dur: bool,
    pub nombre: i64,
    pub autre_seance_id: Option<Uuid>,
}

impl ConflitModif {
    pub(crate) fn dur(code: &str, nombre: i64, autre: Option<Uuid>) -> Self {
        Self {
            code: code.to_string(),
            dur: true,
            nombre,
            autre_seance_id: autre,
        }
    }
    pub(crate) fn avertissement(code: &str, nombre: i64, autre: Option<Uuid>) -> Self {
        Self {
            code: code.to_string(),
            dur: false,
            nombre,
            autre_seance_id: autre,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultatModif {
    pub applique: bool,
    pub deja_fait: bool,
    pub apercu: bool,
    pub modification_id: Option<Uuid>,
    pub seance_id: Option<Uuid>,
    pub seance_arrivee_id: Option<Uuid>,
    /// Conflits durs bloquants (non forcés) et avertissements.
    pub conflits: Vec<ConflitModif>,
    /// Conflits durs forcés (journalisés).
    pub forces: Vec<String>,
}

/// Séance telle que la lit une modification (verrouillée).
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub(crate) struct SeanceMod {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub statut: String,
    pub etat: String,
    pub source: String,
    pub cours_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub classe: String,
    pub prof_user_id: Uuid,
    pub remplacant_user_id: Option<Uuid>,
    pub salle: Option<String>,
    pub passee: bool,
    pub lieu_id: Option<Uuid>,
}

pub(crate) const SELECT_SEANCE_MOD: &str = "SELECT s.id, s.annee_id, s.creneau_id, s.jour, s.heure_debut, s.heure_fin,
        s.statut, s.etat, s.source, s.cours_id, s.classe_id, s.classe, s.prof_user_id, s.remplacant_user_id, s.salle,
        (s.fin_utc IS NOT NULL AND s.fin_utc <= now()) AS passee,
        (SELECT l.lieu_id FROM ent_edt_seance_lieu l
          WHERE l.seance_id = s.id AND l.etablissement_id = s.etablissement_id
          ORDER BY l.principal DESC, l.lieu_id LIMIT 1) AS lieu_id
   FROM ent_edt_seance s";

pub(crate) fn etat_occupant(etat: &str) -> bool {
    matches!(etat, "prevue" | "modifiee" | "ajoutee")
}

fn motif_texte(cmd_motif: Option<&str>) -> Result<Option<String>, EntDbError> {
    match cmd_motif.map(str::trim).filter(|m| !m.is_empty()) {
        None => Ok(None),
        Some(m) if m.chars().count() <= 200 => Ok(Some(m.to_string())),
        Some(_) => Err(invalide()),
    }
}

/// Motifs et clés valides ; renvoie (motif public relu, motif interne relu).
pub(crate) fn valider_motifs(
    code: &str,
    public: Option<&str>,
    interne: Option<&str>,
    cle: Option<&str>,
    origine: Option<&str>,
) -> Result<(Option<String>, Option<String>), EntDbError> {
    if !MOTIFS_PUBLICS.contains(&code) {
        return Err(invalide());
    }
    if cle.is_some_and(|c| c.is_empty() || c.chars().count() > 80) {
        return Err(invalide());
    }
    if origine.is_some_and(|o| !ORIGINES_SAISIE.contains(&o)) {
        return Err(invalide());
    }
    let interne = match interne.map(str::trim).filter(|m| !m.is_empty()) {
        None => None,
        Some(m) if m.chars().count() <= 500 => Some(m.to_string()),
        Some(_) => return Err(invalide()),
    };
    Ok((motif_texte(public)?, interne))
}

/// Une ligne du journal à écrire.
#[derive(Debug, Clone, Default)]
pub(crate) struct LigneModif {
    pub seance_id: Option<Uuid>,
    pub arrivee: Option<Uuid>,
    pub lot_id: Option<Uuid>,
    pub creneau_id: Option<Uuid>,
    pub jour: Option<NaiveDate>,
    pub type_code: &'static str,
    pub avant_etat: Option<String>,
    pub apres_etat: Option<String>,
    pub avant_jour: Option<NaiveDate>,
    pub apres_jour: Option<NaiveDate>,
    pub avant: Option<(NaiveTime, NaiveTime)>,
    pub apres: Option<(NaiveTime, NaiveTime)>,
    pub avant_lieu: Option<Uuid>,
    pub apres_lieu: Option<Uuid>,
    pub apres_salle_texte: Option<String>,
    pub apres_remplacant: Option<Uuid>,
    pub motif_code: String,
    pub motif_public: Option<String>,
    pub motif_interne: Option<String>,
    pub origine: String,
    pub origine_ref: Option<Uuid>,
    pub force_motif: Option<String>,
    pub force_conflits: Option<String>,
    pub passe: bool,
    pub cle: Option<String>,
    pub ancien_exception_id: Option<Uuid>,
    pub ancien_deplacement_id: Option<Uuid>,
}

pub(crate) async fn inserer_modification(
    tx: &mut PgConnection,
    etab: Uuid,
    par: Uuid,
    l: &LigneModif,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_modification
           (id, etablissement_id, seance_id, seance_arrivee_id, lot_id, creneau_id, jour, type, avant_etat,
            apres_etat, avant_jour, apres_jour, avant_debut, avant_fin, apres_debut, apres_fin, avant_lieu_id,
            apres_lieu_id, apres_salle_texte, apres_remplacant_user_id, motif_public_code, motif_public,
            origine, origine_ref, force_motif, force_conflits, passe, cle, cree_par, ancien_exception_id,
            ancien_deplacement_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,
                 $26,$27,$28,$29,$30,$31)",
    )
    .bind(id)
    .bind(etab)
    .bind(l.seance_id)
    .bind(l.arrivee)
    .bind(l.lot_id)
    .bind(l.creneau_id)
    .bind(l.jour)
    .bind(l.type_code)
    .bind(&l.avant_etat)
    .bind(&l.apres_etat)
    .bind(l.avant_jour)
    .bind(l.apres_jour)
    .bind(l.avant.map(|h| h.0))
    .bind(l.avant.map(|h| h.1))
    .bind(l.apres.map(|h| h.0))
    .bind(l.apres.map(|h| h.1))
    .bind(l.avant_lieu)
    .bind(l.apres_lieu)
    .bind(&l.apres_salle_texte)
    .bind(l.apres_remplacant)
    .bind(&l.motif_code)
    .bind(&l.motif_public)
    .bind(&l.origine)
    .bind(l.origine_ref)
    .bind(&l.force_motif)
    .bind(&l.force_conflits)
    .bind(l.passe)
    .bind(&l.cle)
    .bind(par)
    .bind(l.ancien_exception_id)
    .bind(l.ancien_deplacement_id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    if let Some(interne) = &l.motif_interne {
        sqlx::query(
            "INSERT INTO ent_edt_modification_interne (modification_id, etablissement_id, motif_interne, cree_par)
             VALUES ($1,$2,$3,$4)",
        )
        .bind(id)
        .bind(etab)
        .bind(interne)
        .bind(par)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    }
    Ok(id)
}

/// Relance la dérivation d'état des séances (déclencheur de 443/445) après l'écriture d'une ligne du journal.
pub(crate) async fn recalculer_etat(
    tx: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_edt_seance SET statut = statut WHERE etablissement_id = $1 AND id = ANY($2)",
    )
    .bind(etab)
    .bind(ids)
    .execute(tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Archive les exceptions historiques des séances (rien ne s'efface, AR-15) avant qu'une autre décision ne
/// les remplace ou qu'elles soient levées.
pub(crate) async fn archiver_exceptions_de(
    tx: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
    cause: &str,
    par: Uuid,
) -> Result<u64, EntDbError> {
    Ok(sqlx::query(
        "INSERT INTO ent_edt_exception_levee
           (id, etablissement_id, exception_id, creneau_id, jour, kind, nature, remplacant_user_id,
            nouvelle_salle, absence_rh_id, lot_id, pose_par_user_id, pose_le, cause, levee_par_user_id)
         SELECT gen_random_uuid(), x.etablissement_id, x.id, x.creneau_id, x.jour, x.kind, x.nature,
                x.remplacant_user_id, x.nouvelle_salle, x.absence_rh_id, x.lot_id, x.created_by_user_id,
                x.created_at, $3, $4
           FROM ent_edt_exception x
           JOIN ent_edt_seance s ON s.etablissement_id = x.etablissement_id
            AND s.creneau_id = x.creneau_id AND s.jour = x.jour
          WHERE x.etablissement_id = $1 AND s.id = ANY($2)",
    )
    .bind(etab)
    .bind(ids)
    .bind(cause)
    .bind(par)
    .execute(tx)
    .await
    .map_err(db)?
    .rows_affected())
}

/// Pose l'annulation (ou la neutralisation) de séances : exception historique (archivée si elle en remplace
/// une), délégations de remplacement révoquées, séance au statut `annulee` avec son motif PUBLIC. À appeler
/// APRÈS l'insertion des lignes du journal (le déclencheur en dérive `deplacee`/`neutralisee`).
pub(crate) async fn poser_annulations(
    tx: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
    par: Uuid,
    absence_rh_id: Option<Uuid>,
    motif_code: &str,
    motif_public: Option<&str>,
    nature: &str,
) -> Result<(), EntDbError> {
    if ids.is_empty() {
        return Ok(());
    }
    archiver_exceptions_de(tx, etab, ids, "remplacee", par).await?;
    sqlx::query(
        "WITH cibles AS (SELECT creneau_id, jour FROM ent_edt_seance
                          WHERE etablissement_id = $1 AND id = ANY($2) AND statut <> 'retiree'),
              exc AS (
                INSERT INTO ent_edt_exception
                  (id, etablissement_id, creneau_id, jour, kind, motif, created_by_user_id, created_at, nature,
                   absence_rh_id)
                SELECT gen_random_uuid(), $1, c.creneau_id, c.jour, 'annulation', '', $3, now(), $5, $4
                  FROM cibles c
                ON CONFLICT (etablissement_id, creneau_id, jour) DO UPDATE SET
                    kind = 'annulation', remplacant_user_id = NULL, nouvelle_salle = NULL,
                    created_by_user_id = EXCLUDED.created_by_user_id, created_at = now(), nature = EXCLUDED.nature,
                    absence_rh_id = coalesce(EXCLUDED.absence_rh_id, ent_edt_exception.absence_rh_id)
                RETURNING id)
         UPDATE ent_delegation SET statut = 'revoquee', revoquee_le = now(), revoquee_par_user_id = $3
          WHERE etablissement_id = $1 AND reference_id IN (SELECT id FROM exc) AND source = 'remplacement'
            AND statut = 'accordee'",
    )
    .bind(etab)
    .bind(ids)
    .bind(par)
    .bind(absence_rh_id)
    .bind(nature)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_edt_seance SET statut = 'annulee', remplacant_user_id = NULL, motif_public_code = $3,
                motif_public = $4, updated_at = now()
          WHERE etablissement_id = $1 AND id = ANY($2) AND statut <> 'retiree'",
    )
    .bind(etab)
    .bind(ids)
    .bind(motif_code)
    .bind(motif_public)
    .execute(tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Resynchronise les lieux d'une séance depuis sa salle (texte) : la salle fait foi du lieu principal.
pub(crate) async fn resynchroniser_lieux(
    tx: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
) -> Result<(), EntDbError> {
    sqlx::query(
        "DELETE FROM ent_edt_seance_lieu WHERE etablissement_id = $1 AND seance_id = ANY($2)",
    )
    .bind(etab)
    .bind(ids)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_edt_seance_lieu (seance_id, etablissement_id, lieu_id, principal)
         SELECT s.id, s.etablissement_id, sa.id, true
           FROM ent_edt_seance s
           JOIN ent_salle sa ON sa.etablissement_id = s.etablissement_id
            AND lower(btrim(sa.code)) = lower(btrim(s.salle))
          WHERE s.etablissement_id = $1 AND s.id = ANY($2)
         ON CONFLICT DO NOTHING",
    )
    .bind(etab)
    .bind(ids)
    .execute(tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Lève les exceptions historiques (annulation, neutralisation, changement de lieu) de séances : archive
/// puis supprime l'exception vivante, remet la séance à l'état du plan (statut, salle du créneau, lieux).
/// À appeler APRÈS avoir marqué les lignes du journal comme levées.
pub(crate) async fn lever_projection(
    tx: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
    par: Uuid,
) -> Result<(), EntDbError> {
    if ids.is_empty() {
        return Ok(());
    }
    archiver_exceptions_de(tx, etab, ids, "levee", par).await?;
    sqlx::query(
        "DELETE FROM ent_edt_exception x USING ent_edt_seance s
          WHERE x.etablissement_id = $1 AND s.id = ANY($2) AND s.etablissement_id = x.etablissement_id
            AND s.creneau_id = x.creneau_id AND s.jour = x.jour AND x.kind <> 'remplacement'",
    )
    .bind(etab)
    .bind(ids)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_edt_seance s SET statut = 'prevue', remplacant_user_id = NULL, motif_public = NULL,
                motif_public_code = NULL, salle = c.salle, updated_at = now()
           FROM ent_edt_creneau c
          WHERE s.etablissement_id = $1 AND s.id = ANY($2) AND s.statut <> 'retiree'
            AND c.id = s.creneau_id AND c.etablissement_id = s.etablissement_id
            AND s.source = 'genere'",
    )
    .bind(etab)
    .bind(ids)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    // Les séances ajoutées ou arrivées par déplacement n'ont pas de plan à retrouver : statut seul.
    sqlx::query(
        "UPDATE ent_edt_seance SET statut = 'prevue', remplacant_user_id = NULL, motif_public = NULL,
                motif_public_code = NULL, updated_at = now()
          WHERE etablissement_id = $1 AND id = ANY($2) AND statut <> 'retiree' AND source <> 'genere'",
    )
    .bind(etab)
    .bind(ids)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    resynchroniser_lieux(tx, etab, ids).await?;
    Ok(())
}

/// Pont des écrivains historiques (exception, remplacement, déplacement, lot) vers le journal : à appeler dans
/// leur transaction, avant leur écriture de séance. Les lignes vivantes du même créneau et du même jour sont
/// levées (la nouvelle décision les remplace), puis la nouvelle est posée.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn pont_journaliser(
    tx: &mut PgConnection,
    etab: Uuid,
    par: Uuid,
    creneau_id: Uuid,
    jour: NaiveDate,
    nature: &str,
    exception_id: Option<Uuid>,
    deplacement_id: Option<Uuid>,
    salle_texte: Option<&str>,
    remplacant: Option<Uuid>,
    absence_rh_id: Option<Uuid>,
    lot_id: Option<Uuid>,
    nouveau_jour: Option<NaiveDate>,
) -> Result<(), EntDbError> {
    let (type_code, apres_etat) = match nature {
        "remplacement" => ("intervenant", "modifiee"),
        "changement_salle" => ("changement_lieu", "modifiee"),
        "deplacement" => ("deplacement", "deplacee"),
        _ => ("annulation", "annulee"),
    };
    // Décision remplacée : les états vivants de ce créneau et de ce jour sont levés par la nouvelle.
    sqlx::query(
        "UPDATE ent_edt_modification SET levee_le = now(), levee_par = $4
          WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3 AND levee_le IS NULL
            AND type IN ('annulation', 'deplacement', 'neutralisation', 'changement_lieu', 'intervenant')",
    )
    .bind(etab)
    .bind(creneau_id)
    .bind(jour)
    .bind(par)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    let motif = if absence_rh_id.is_some() {
        "absence_professeur"
    } else {
        "autre"
    };
    let origine = if absence_rh_id.is_some() {
        "absence"
    } else if lot_id.is_some() {
        "lot"
    } else {
        "saisie"
    };
    sqlx::query(
        "INSERT INTO ent_edt_modification
           (id, etablissement_id, seance_id, seance_arrivee_id, creneau_id, jour, type, avant_etat, apres_etat,
            avant_jour, apres_jour, apres_lieu_id, apres_salle_texte, apres_remplacant_user_id,
            motif_public_code, origine, origine_ref, cree_par, ancien_exception_id, ancien_deplacement_id)
         SELECT gen_random_uuid(), $1, s.id,
                (SELECT arr.id FROM ent_edt_seance arr
                  WHERE arr.etablissement_id = $1 AND arr.creneau_id = $2 AND arr.jour = $13 AND $13 <> $3
                    AND arr.source = 'deplacement'),
                $2, $3, $4, 'prevue', $5, CASE WHEN $4 = 'deplacement' THEN $3 END, $13,
                (SELECT sa.id FROM ent_salle sa WHERE sa.etablissement_id = $1
                  AND lower(btrim(sa.code)) = lower(btrim($6))),
                $6, $7, $8, $9, $10, $11, $12, $14
           FROM (SELECT 1) un
           LEFT JOIN ent_edt_seance s ON s.etablissement_id = $1 AND s.creneau_id = $2 AND s.jour = $3
            AND s.statut <> 'retiree'",
    )
    .bind(etab)
    .bind(creneau_id)
    .bind(jour)
    .bind(type_code)
    .bind(apres_etat)
    .bind(salle_texte)
    .bind(remplacant)
    .bind(motif)
    .bind(origine)
    .bind(absence_rh_id)
    .bind(par)
    .bind(exception_id)
    .bind(nouveau_jour)
    .bind(deplacement_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "pont du journal du réel");
        db(e)
    })?;
    recalculer_etat_creneau(tx, etab, creneau_id, jour).await
}

/// Relance la dérivation d'état de la séance d'un créneau et d'un jour (la ligne du journal vient de changer).
async fn recalculer_etat_creneau(
    tx: &mut PgConnection,
    etab: Uuid,
    creneau_id: Uuid,
    jour: NaiveDate,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_edt_seance SET statut = statut
          WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3 AND statut <> 'retiree'",
    )
    .bind(etab)
    .bind(creneau_id)
    .bind(jour)
    .execute(tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Pont de la levée historique : les lignes vivantes du créneau et du jour sont levées.
pub(crate) async fn pont_lever(
    tx: &mut PgConnection,
    etab: Uuid,
    par: Uuid,
    creneau_id: Uuid,
    jour: NaiveDate,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_edt_modification SET levee_le = now(), levee_par = $4
          WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3 AND levee_le IS NULL
            AND type IN ('annulation', 'deplacement', 'neutralisation', 'changement_lieu', 'intervenant')",
    )
    .bind(etab)
    .bind(creneau_id)
    .bind(jour)
    .bind(par)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    recalculer_etat_creneau(tx, etab, creneau_id, jour).await
}

/// Pont de l'annulation en masse historique (171) : un lot du socle 10 et une ligne du journal par exception
/// du lot, dans la transaction de l'ancien écrivain. La clé d'idempotence reste celle de l'ancien lot.
pub(crate) async fn pont_lot(
    tx: &mut PgConnection,
    etab: Uuid,
    par: Uuid,
    masse_id: Uuid,
) -> Result<Uuid, EntDbError> {
    let lot_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_lot
           (id, etablissement_id, nature, du, au, portee, heure_debut, heure_fin, motif_public_code, motif_public,
            origine, nb_seances, nb_classes, cree_par, ancien_masse_id)
         SELECT $3, m.etablissement_id, 'annulation', m.du, m.au,
                CASE m.portee WHEN 'niveau' THEN 'niveau' WHEN 'classes' THEN 'classes' ELSE 'etablissement' END,
                m.heure_debut, m.heure_fin, 'autre', left(btrim(m.motif_public), 200), 'saisie',
                m.nb_seances, m.nb_classes, $4, m.id
           FROM ent_edt_annulation_masse m WHERE m.etablissement_id = $1 AND m.id = $2
         ON CONFLICT DO NOTHING",
    )
    .bind(etab)
    .bind(masse_id)
    .bind(lot_id)
    .bind(par)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    let lot_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_edt_lot WHERE etablissement_id = $1 AND ancien_masse_id = $2",
    )
    .bind(etab)
    .bind(masse_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    // Les décisions antérieures de ces séances sont remplacées (levées), puis une ligne par exception du lot.
    sqlx::query(
        "UPDATE ent_edt_modification m SET levee_le = now(), levee_par = $3
           FROM ent_edt_exception x
          WHERE x.etablissement_id = $1 AND x.lot_id = $2 AND m.etablissement_id = x.etablissement_id
            AND m.creneau_id = x.creneau_id AND m.jour = x.jour AND m.levee_le IS NULL
            AND m.type IN ('annulation', 'deplacement', 'neutralisation', 'changement_lieu', 'intervenant')",
    )
    .bind(etab)
    .bind(masse_id)
    .bind(par)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_edt_modification
           (id, etablissement_id, seance_id, lot_id, creneau_id, jour, type, avant_etat, apres_etat,
            motif_public_code, motif_public, origine, cree_par, ancien_exception_id)
         SELECT gen_random_uuid(), x.etablissement_id, s.id, $3, x.creneau_id, x.jour, 'annulation', 'prevue',
                'annulee', 'autre', l.motif_public, 'lot', $4, x.id
           FROM ent_edt_exception x
           JOIN ent_edt_lot l ON l.id = $3 AND l.etablissement_id = x.etablissement_id
           LEFT JOIN ent_edt_seance s ON s.etablissement_id = x.etablissement_id
                AND s.creneau_id = x.creneau_id AND s.jour = x.jour AND s.statut <> 'retiree'
          WHERE x.etablissement_id = $1 AND x.lot_id = $2",
    )
    .bind(etab)
    .bind(masse_id)
    .bind(lot_id)
    .bind(par)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_edt_seance s SET statut = s.statut FROM ent_edt_exception x
          WHERE x.etablissement_id = $1 AND x.lot_id = $2 AND s.etablissement_id = x.etablissement_id
            AND s.creneau_id = x.creneau_id AND s.jour = x.jour",
    )
    .bind(etab)
    .bind(masse_id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok(lot_id)
}

/// Charge utile d'un événement de modification : motif PUBLIC seulement, jamais le motif interne.
pub(crate) fn charge_evenement(
    modification_id: Uuid,
    seance_id: Option<Uuid>,
    creneau_id: Option<Uuid>,
    jour: NaiveDate,
    type_code: &str,
    motif_code: &str,
    motif_public: Option<&str>,
    origine: &str,
) -> serde_json::Value {
    json!({
        "modification_id": modification_id, "seance_id": seance_id, "creneau_id": creneau_id, "jour": jour,
        "type": type_code, "motif_public_code": motif_code, "motif_public": motif_public, "origine": origine,
    })
}

impl PgEnt {
    /// Applique un changement à une séance (EDT-160 à 167). Voir la documentation du module.
    pub async fn edt_modifier_seance(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdModifierSeance,
    ) -> Result<ResultatModif, EntDbError> {
        self.exiger_edt(a, cmd.changement.action()).await?;
        let etab = a.etablissement_id;
        let (motif_public, motif_interne) = valider_motifs(
            &cmd.motif_public_code,
            cmd.motif_public.as_deref(),
            cmd.motif_interne.as_deref(),
            cmd.cle.as_deref(),
            cmd.origine.as_deref(),
        )?;
        let forcer = cmd
            .forcer
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty());
        if cmd.forcer.is_some() && forcer.is_none() {
            return Err(invalide()); // forçage sans motif : refusé
        }
        if forcer.is_some_and(|m| m.chars().count() > 300) {
            return Err(invalide());
        }
        // Le forçage est un droit à part : la direction seule.
        let peut_forcer = if forcer.is_some() {
            self.ref_peut(a, Action::EdtForcerConflit).await?
        } else {
            false
        };
        if forcer.is_some() && !peut_forcer {
            return Err(EntDbError::Forbidden);
        }
        if let ChangementSeance::Intervenant { remplacant_user_id } = &cmd.changement {
            return self.modifier_intervenant(a, cmd, *remplacant_user_id).await;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Idempotence : la clé a déjà été appliquée.
        if let Some(cle) = cmd.cle.as_deref() {
            let deja: Option<(Uuid, Option<Uuid>, String)> = sqlx::query_as(
                "SELECT id, seance_id, type FROM ent_edt_modification WHERE etablissement_id = $1 AND cle = $2",
            )
            .bind(etab)
            .bind(cle)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((id, seance, type_code)) = deja {
                if type_code != cmd.changement.code()
                    || (cmd.seance_id.is_some() && cmd.seance_id != seance)
                {
                    return Err(conflit()); // même clé, autre demande
                }
                return Ok(ResultatModif {
                    applique: true,
                    deja_fait: true,
                    modification_id: Some(id),
                    seance_id: seance,
                    ..Default::default()
                });
            }
        }
        let origine = cmd.origine.clone().unwrap_or_else(|| "saisie".to_string());
        let ctx = CtxModif {
            a: *a,
            motif_code: cmd.motif_public_code.clone(),
            motif_public,
            motif_interne,
            origine,
            origine_ref: cmd.origine_ref,
            cle: cmd.cle.clone(),
            forcer: forcer.map(str::to_string),
            apercu: cmd.apercu,
            lot_id: None,
            evenements: std::sync::Mutex::new(Vec::new()),
        };
        let res = match &cmd.changement {
            ChangementSeance::Ajout { .. } => {
                self.appliquer_ajout(&mut tx, &ctx, &cmd.changement, None, true)
                    .await?
            }
            autre => {
                let id = cmd.seance_id.ok_or_else(invalide)?;
                self.appliquer_sur_seance(&mut tx, &ctx, id, autre).await?
            }
        };
        if !res.applique || ctx.apercu {
            return Ok(res); // rien d'écrit : la transaction est abandonnée
        }
        tx.commit().await.map_err(db)?;
        if let Ok(evs) = ctx.evenements.lock() {
            self.diffuser_apres_commit(&evs);
        }
        Ok(res)
    }
}

/// Contexte commun d'une modification.
pub(crate) struct CtxModif<'a> {
    pub a: Acteur<'a>,
    pub motif_code: String,
    pub motif_public: Option<String>,
    pub motif_interne: Option<String>,
    pub origine: String,
    pub origine_ref: Option<Uuid>,
    pub cle: Option<String>,
    pub forcer: Option<String>,
    pub apercu: bool,
    /// Lot d'appartenance (les lignes du journal le portent) ; avec un lot, les événements par séance sont
    /// remplacés par l'événement unique du lot.
    pub lot_id: Option<Uuid>,
    /// Événements publiés dans la transaction, à diffuser aux observateurs locaux après le commit.
    pub evenements: std::sync::Mutex<Vec<ent_events::MetierEvent>>,
}

impl CtxModif<'_> {
    /// Remplit le motif, l'origine et le forçage d'une ligne.
    pub(crate) fn habiller(&self, mut l: LigneModif, forces: &[String]) -> LigneModif {
        l.motif_code = self.motif_code.clone();
        l.motif_public = self.motif_public.clone();
        l.motif_interne = self.motif_interne.clone();
        l.origine = self.origine.clone();
        l.origine_ref = self.origine_ref;
        l.cle = self.cle.clone();
        l.lot_id = self.lot_id;
        if !forces.is_empty() {
            l.force_motif = self.forcer.clone();
            l.force_conflits = Some(forces.join(","));
        }
        l
    }
}

impl PgEnt {
    /// Sépare conflits durs (bloquants sauf forçage) et avertissements ; renvoie (à afficher, forcés).
    pub(crate) fn trancher_conflits(
        ctx: &CtxModif<'_>,
        conflits: Vec<ConflitModif>,
    ) -> (Vec<ConflitModif>, Vec<String>) {
        let mut affiches = Vec::new();
        let mut forces = Vec::new();
        for c in conflits {
            if c.dur
                && ctx.forcer.is_some()
                && !c.code.starts_with("seance_")
                && c.code != "creneau_deja_ce_jour"
            {
                forces.push(c.code.clone());
            } else {
                affiches.push(c);
            }
        }
        (affiches, forces)
    }

    async fn charger_seance_modif(
        tx: &mut PgConnection,
        etab: Uuid,
        id: Uuid,
    ) -> Result<SeanceMod, EntDbError> {
        sqlx::query_as::<_, SeanceMod>(&format!(
            "{SELECT_SEANCE_MOD} WHERE s.etablissement_id = $1 AND s.id = $2 FOR UPDATE OF s"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    /// Cœur d'une modification sur une séance existante (sauf l'ajout et le remplacement).
    pub(crate) async fn appliquer_sur_seance(
        &self,
        tx: &mut TenantTx<'_>,
        ctx: &CtxModif<'_>,
        seance_id: Uuid,
        changement: &ChangementSeance,
    ) -> Result<ResultatModif, EntDbError> {
        let a = &ctx.a;
        let etab = a.etablissement_id;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 6))")
            .bind(format!("{etab}:seance:{seance_id}"))
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        let s = Self::charger_seance_modif(tx, etab, seance_id).await?;
        let mut res = ResultatModif {
            seance_id: Some(s.id),
            apercu: ctx.apercu,
            ..Default::default()
        };
        if s.etat == "retiree" {
            return Err(conflit());
        }
        // §9.6 : le passé ne change plus que par la correction.
        if s.passee {
            res.conflits
                .push(ConflitModif::dur("seance_passee", 1, None));
            return Ok(res);
        }
        let inactif = !etat_occupant(&s.etat);
        match changement {
            ChangementSeance::Annulation | ChangementSeance::Neutralisation => {
                let attendu = if matches!(changement, ChangementSeance::Neutralisation) {
                    "neutralisee"
                } else {
                    "annulee"
                };
                if s.etat == attendu {
                    // Déjà annulée : idempotent quand une modification vivante la porte.
                    let vivante: Option<Uuid> = sqlx::query_scalar(
                        "SELECT id FROM ent_edt_modification WHERE etablissement_id = $1 AND seance_id = $2
                          AND levee_le IS NULL AND type = $3",
                    )
                    .bind(etab)
                    .bind(s.id)
                    .bind(changement.code())
                    .fetch_optional(&mut **tx)
                    .await
                    .map_err(db)?;
                    res.applique = true;
                    res.deja_fait = true;
                    res.modification_id = vivante;
                    return Ok(res);
                }
                if inactif {
                    return Err(conflit());
                }
                res.applique = true;
                if ctx.apercu {
                    return Ok(res);
                }
                let mid = self
                    .annuler_une(tx, ctx, &s, changement.code(), attendu, true)
                    .await?;
                res.modification_id = Some(mid);
                Ok(res)
            }
            ChangementSeance::AutreCours { cours_id } => {
                if inactif {
                    return Err(conflit());
                }
                // D'abord la séance de remplacement (les contrôles excluent la séance d'origine, qui va être
                // annulée) : sans conflit résolu, rien n'est écrit.
                let ajout = ChangementSeance::Ajout {
                    cours_id: *cours_id,
                    jour: s.jour,
                    heure_debut: s.heure_debut,
                    heure_fin: s.heure_fin,
                    lieu_id: s.lieu_id,
                };
                let mut r = self
                    .appliquer_ajout(tx, ctx, &ajout, Some(s.id), false)
                    .await?;
                r.apercu = ctx.apercu;
                if !r.applique || ctx.apercu {
                    return Ok(r);
                }
                let mid = self
                    .annuler_une(tx, ctx, &s, "annulation", "annulee", true)
                    .await?;
                if let Some(nouvelle) = r.seance_id {
                    sqlx::query("UPDATE ent_edt_seance SET remplacee_par_id = $3 WHERE etablissement_id = $1 AND id = $2")
                        .bind(etab)
                        .bind(s.id)
                        .bind(nouvelle)
                        .execute(&mut **tx)
                        .await
                        .map_err(db)?;
                }
                r.seance_arrivee_id = r.seance_id;
                r.seance_id = Some(s.id);
                r.modification_id = Some(mid);
                Ok(r)
            }
            ChangementSeance::ChangementLieu { lieu_id } => {
                if inactif {
                    return Err(conflit());
                }
                if s.lieu_id == Some(*lieu_id) {
                    res.applique = true;
                    res.deja_fait = true;
                    return Ok(res);
                }
                let salle: Option<(String, bool)> = sqlx::query_as(
                    "SELECT code, actif FROM ent_salle WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(lieu_id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(db)?;
                let Some((code, actif)) = salle else {
                    return Err(introuvable());
                };
                if !actif {
                    return Err(conflit());
                }
                let conflits = self
                    .controler_placement(
                        tx,
                        etab,
                        &CiblePlacement::de(&s),
                        &Placement {
                            jour: s.jour,
                            debut: s.heure_debut,
                            fin: s.heure_fin,
                            lieu: Some(*lieu_id),
                            public: false,
                            intervenants: false,
                        },
                    )
                    .await?;
                let (affiches, forces) = Self::trancher_conflits(ctx, conflits);
                res.forces = forces.clone();
                res.conflits = affiches;
                if res.conflits.iter().any(|c| c.dur) {
                    return Ok(res);
                }
                res.applique = true;
                if ctx.apercu {
                    return Ok(res);
                }
                let l = ctx.habiller(
                    LigneModif {
                        seance_id: Some(s.id),
                        creneau_id: Some(s.creneau_id),
                        jour: Some(s.jour),
                        type_code: "changement_lieu",
                        avant_etat: Some(s.etat.clone()),
                        apres_etat: Some("modifiee".to_string()),
                        avant_lieu: s.lieu_id,
                        apres_lieu: Some(*lieu_id),
                        apres_salle_texte: Some(code.clone()),
                        ..Default::default()
                    },
                    &forces,
                );
                // Une décision antérieure de lieu est remplacée (levée), jamais écrasée en silence.
                self.lever_vivantes(tx, etab, s.id, &["changement_lieu"], a.user_id)
                    .await?;
                let mid = inserer_modification(tx, etab, a.user_id, &l).await?;
                self.ecrire_exception_lieu(tx, etab, &s, &code, a.user_id)
                    .await?;
                res.modification_id = Some(mid);
                self.evenement_modif_edt(
                    tx,
                    a,
                    &s,
                    mid,
                    "changement_lieu",
                    ctx,
                    EVT_SEANCE_MODIFIEE,
                )
                .await?;
                Ok(res)
            }
            ChangementSeance::ChangementHoraire {
                heure_debut,
                heure_fin,
            } => {
                if inactif || heure_fin <= heure_debut {
                    return Err(if inactif { conflit() } else { invalide() });
                }
                if (s.heure_debut, s.heure_fin) == (*heure_debut, *heure_fin) {
                    res.applique = true;
                    res.deja_fait = true;
                    return Ok(res);
                }
                let conflits = self
                    .controler_placement(
                        tx,
                        etab,
                        &CiblePlacement::de(&s),
                        &Placement {
                            jour: s.jour,
                            debut: *heure_debut,
                            fin: *heure_fin,
                            lieu: s.lieu_id,
                            public: true,
                            intervenants: true,
                        },
                    )
                    .await?;
                let (affiches, forces) = Self::trancher_conflits(ctx, conflits);
                res.forces = forces.clone();
                res.conflits = affiches;
                if res.conflits.iter().any(|c| c.dur) {
                    return Ok(res);
                }
                res.applique = true;
                if ctx.apercu {
                    return Ok(res);
                }
                let l = ctx.habiller(
                    LigneModif {
                        seance_id: Some(s.id),
                        creneau_id: Some(s.creneau_id),
                        jour: Some(s.jour),
                        type_code: "changement_horaire",
                        avant_etat: Some(s.etat.clone()),
                        apres_etat: Some("modifiee".to_string()),
                        avant: Some((s.heure_debut, s.heure_fin)),
                        apres: Some((*heure_debut, *heure_fin)),
                        ..Default::default()
                    },
                    &forces,
                );
                self.lever_vivantes(tx, etab, s.id, &["changement_horaire"], a.user_id)
                    .await?;
                let mid = inserer_modification(tx, etab, a.user_id, &l).await?;
                sqlx::query(
                    "UPDATE ent_edt_seance SET heure_debut = $3, heure_fin = $4, updated_at = now(),
                            demi_journee = CASE WHEN $3 < time '12:30' THEN 'matin' ELSE 'apres_midi' END
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(s.id)
                .bind(heure_debut)
                .bind(heure_fin)
                .execute(&mut **tx)
                .await
                .map_err(db)?;
                res.modification_id = Some(mid);
                self.evenement_modif_edt(
                    tx,
                    a,
                    &s,
                    mid,
                    "changement_horaire",
                    ctx,
                    EVT_SEANCE_MODIFIEE,
                )
                .await?;
                Ok(res)
            }
            ChangementSeance::Deplacement {
                nouveau_jour,
                heure_debut,
                heure_fin,
                lieu_id,
            } => {
                if inactif || heure_fin <= heure_debut || *nouveau_jour == s.jour {
                    return Err(if inactif { conflit() } else { invalide() });
                }
                if (*nouveau_jour - s.jour).num_days().abs() > 60 {
                    return Err(invalide());
                }
                self.appliquer_deplacement(
                    tx,
                    ctx,
                    &s,
                    *nouveau_jour,
                    *heure_debut,
                    *heure_fin,
                    *lieu_id,
                    res,
                )
                .await
            }
            ChangementSeance::Fusion { avec_seance_id } => {
                if inactif || *avec_seance_id == s.id {
                    return Err(if inactif { conflit() } else { invalide() });
                }
                self.appliquer_fusion(tx, ctx, &s, *avec_seance_id, res)
                    .await
            }
            ChangementSeance::Intervenant { .. } | ChangementSeance::Ajout { .. } => {
                Err(invalide())
            }
        }
    }

    /// Annule (ou neutralise) une séance : ligne du journal d'abord, puis projection historique, puis événement.
    pub(crate) async fn annuler_une(
        &self,
        tx: &mut TenantTx<'_>,
        ctx: &CtxModif<'_>,
        s: &SeanceMod,
        type_code: &'static str,
        apres_etat: &str,
        avec_cle: bool,
    ) -> Result<Uuid, EntDbError> {
        let a = &ctx.a;
        let etab = a.etablissement_id;
        let mut l = ctx.habiller(
            LigneModif {
                seance_id: Some(s.id),
                creneau_id: Some(s.creneau_id),
                jour: Some(s.jour),
                type_code,
                avant_etat: Some(s.etat.clone()),
                apres_etat: Some(apres_etat.to_string()),
                avant: Some((s.heure_debut, s.heure_fin)),
                avant_lieu: s.lieu_id,
                ..Default::default()
            },
            &[],
        );
        if !avec_cle {
            l.cle = None;
        }
        let mid = inserer_modification(tx, etab, a.user_id, &l).await?;
        let absence = (ctx.origine == "absence")
            .then_some(ctx.origine_ref)
            .flatten();
        poser_annulations(
            tx,
            etab,
            &[s.id],
            a.user_id,
            absence,
            &ctx.motif_code,
            ctx.motif_public.as_deref(),
            "annulation",
        )
        .await?;
        self.evenement_modif_edt(tx, a, s, mid, type_code, ctx, EVT_SEANCE_ANNULEE)
            .await?;
        Ok(mid)
    }

    /// Lève (sans toucher à la séance) les lignes vivantes d'une séance : elles sont remplacées par une nouvelle.
    pub(crate) async fn lever_vivantes(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        seance_id: Uuid,
        types: &[&str],
        par: Uuid,
    ) -> Result<u64, EntDbError> {
        let types: Vec<String> = types.iter().map(|t| (*t).to_string()).collect();
        Ok(sqlx::query(
            "UPDATE ent_edt_modification SET levee_le = now(), levee_par = $3
              WHERE etablissement_id = $1 AND seance_id = $2 AND levee_le IS NULL AND type = ANY($4)",
        )
        .bind(etab)
        .bind(seance_id)
        .bind(par)
        .bind(&types)
        .execute(&mut **tx)
        .await
        .map_err(db)?
        .rows_affected())
    }

    /// Projection historique d'un changement de lieu : exception `changement_salle` (le remplaçant éventuel est
    /// gardé), salle de la séance, lieu principal.
    async fn ecrire_exception_lieu(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        s: &SeanceMod,
        code: &str,
        par: Uuid,
    ) -> Result<(), EntDbError> {
        archiver_exceptions_de(tx, etab, &[s.id], "remplacee", par).await?;
        sqlx::query(
            "INSERT INTO ent_edt_exception
               (id, etablissement_id, creneau_id, jour, kind, nouvelle_salle, motif, created_by_user_id,
                created_at, nature)
             VALUES ($1,$2,$3,$4,'changement_salle',$5,'',$6,now(),'changement_salle')
             ON CONFLICT (etablissement_id, creneau_id, jour) DO UPDATE SET
                nouvelle_salle = EXCLUDED.nouvelle_salle, created_by_user_id = EXCLUDED.created_by_user_id,
                created_at = now(),
                kind = CASE WHEN ent_edt_exception.kind = 'remplacement' THEN 'remplacement'
                            ELSE 'changement_salle' END,
                nature = 'changement_salle'",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(s.creneau_id)
        .bind(s.jour)
        .bind(code)
        .bind(par)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE ent_edt_seance SET salle = $3, updated_at = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(s.id)
            .bind(code)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        resynchroniser_lieux(tx, etab, &[s.id]).await?;
        Ok(())
    }

    /// Événements d'une modification (§18.1) : le nouveau nom, et l'ancien `edt.exception` en alias le temps de
    /// la bascule des consommateurs (§18.1, dernier alinéa). Le motif interne n'y figure jamais.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn evenement_modif_edt(
        &self,
        tx: &mut TenantTx<'_>,
        a: &Acteur<'_>,
        s: &SeanceMod,
        modification_id: Uuid,
        type_code: &str,
        ctx: &CtxModif<'_>,
        nom: &str,
    ) -> Result<(), EntDbError> {
        if ctx.lot_id.is_some() {
            return Ok(()); // l'événement unique du lot remplace les événements par séance
        }
        let charge = charge_evenement(
            modification_id,
            Some(s.id),
            Some(s.creneau_id),
            s.jour,
            type_code,
            &ctx.motif_code,
            ctx.motif_public.as_deref(),
            &ctx.origine,
        );
        let ev = publier_agg(tx, a, AggregateKind::Edt, nom, s.id, charge).await?;
        let mut evs = vec![ev];
        if matches!(
            type_code,
            "annulation" | "neutralisation" | "changement_lieu"
        ) {
            let nature = if type_code == "changement_lieu" {
                "changement_salle"
            } else {
                "annulation"
            };
            evs.push(
                publier_agg(
                    tx,
                    a,
                    AggregateKind::Edt,
                    ent_events::types::EDT_EXCEPTION,
                    s.creneau_id,
                    json!({ "creneau_id": s.creneau_id, "jour": s.jour, "nature": nature,
                            "modification_id": modification_id }),
                )
                .await?,
            );
        }
        if let Ok(mut v) = ctx.evenements.lock() {
            v.extend(evs);
        }
        Ok(())
    }
}
