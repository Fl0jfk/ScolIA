//! Socle 21, lot V1 « modèle et reprise » : accès aux tables `eva_*` (migrations 461 à 469).
//!
//! Cette couche est volontairement **mince** : elle porte ce que V1 doit prouver et que les lots
//! suivants (V3 grille, V8 fin de période…) réutiliseront sans le réécrire :
//!
//! * les **règles de calcul** de l'établissement (`ReglesMoteur`, paramétrables, historisées) ;
//! * l'**écriture d'un résultat** — jamais de suppression, historique en ajout seul, correction
//!   motivée après l'arrêt, garde du socle 2 pour l'élève protégé, événement **neutre** ;
//! * la **machine d'états** d'une période par classe (arrêt, réouverture motivée) ;
//! * la **période par élève** (R-V29) ;
//! * la **reprise M2** de l'existant et son rapport ;
//! * le **chargement** des entrées du moteur (`domain_eleve::evaluation_moteur`) pour un élève et
//!   une période — le seul chemin par lequel une moyenne se calcule depuis `eva_*` ;
//! * la **lecture publiée** (R-V7) : famille et élève ne lisent que les évaluations publiées.
//!
//! Isolation : toutes les requêtes portent `etablissement_id` et passent par `TenantTx` (RLS).
//! Événements : identifiants seulement (R-V17) — jamais une note, une moyenne, un statut, un
//! motif ni un nom.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use domain_eleve::evaluation_moteur::{
    moyenne_generale, moyenne_matiere, valider_saisie, ArrondiRegle, CoefSousMatiere, Comptage,
    EntreeCalcul, ErreurSaisieMoteur, EtapesArrondi, EvalCalcul, MatiereMoyenne, MedianePaire,
    MethodeAnnuelle, MethodeSousMatieres, ModeArrondi, MoyenneCalculee, PonderationGenerale,
    PonderationNotes, Precision, Rat, ReglesMoteur, StatutCalcul, TraitementBonus,
    TraitementNonRendu, SEUIL_STATS_PLANCHER,
};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{conflit, db, refus};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Types d'événements de V1. Charges : identifiants seulement.
pub mod types_v1 {
    pub const RESULTATS_SAISIS: &str = "evaluations.resultats_saisis";
    pub const CORRIGEE: &str = "evaluations.corrigee";
    pub const NOTES_ARRETEES: &str = "evaluations.notes_arretees";
    pub const PERIODE_ROUVERTE: &str = "evaluations.periode_rouverte";
    pub const REGLES_MODIFIEES: &str = "evaluations.regles_modifiees";
    pub const REPRISE_TERMINEE: &str = "evaluations.reprise_terminee";
}

// ---------------------------------------------------------------------------
// Règles de calcul : persistance
// ---------------------------------------------------------------------------

/// Règles d'un établissement pour une année, avec ce qui n'appartient pas au moteur.
#[derive(Debug, Clone, PartialEq)]
pub struct ReglesEtablissement {
    pub regles: ReglesMoteur,
    /// Pas de saisie des notes, en millièmes (10 : centième ; 500 : demi-point).
    pub pas_saisie_m: i64,
    pub version: i32,
    /// Posé au premier arrêt de période : échelle et méthode annuelle ne changent plus (R-V21).
    pub figees: bool,
}

impl Default for ReglesEtablissement {
    fn default() -> Self {
        Self {
            regles: ReglesMoteur::default(),
            pas_saisie_m: 10,
            version: 0,
            figees: false,
        }
    }
}

fn bonus_vers_colonnes(b: TraitementBonus) -> (&'static str, Option<i64>) {
    match b {
        TraitementBonus::Ignore => ("ignore", None),
        TraitementBonus::CommeNormale => ("comme_normale", None),
        TraitementBonus::PointsAuDessusDe { seuil_m } => ("points_au_dessus", Some(seuil_m)),
    }
}

fn bonus_depuis_colonnes(code: &str, seuil_m: Option<i64>) -> TraitementBonus {
    match (code, seuil_m) {
        ("comme_normale", _) => TraitementBonus::CommeNormale,
        ("points_au_dessus", Some(seuil_m)) => TraitementBonus::PointsAuDessusDe { seuil_m },
        _ => TraitementBonus::Ignore,
    }
}

fn etape(precision: Option<String>, mode: &str) -> Option<ArrondiRegle> {
    let p = Precision::parse(precision.as_deref()?)?;
    Some(ArrondiRegle::new(
        p,
        ModeArrondi::parse(mode).unwrap_or_default(),
    ))
}

fn serie_ponderation(p: PonderationNotes) -> &'static str {
    match p {
        PonderationNotes::NotesRamenees => "notes_ramenees",
        PonderationNotes::PointsCumules => "points_cumules",
    }
}

/// Règles par défaut d'une année sans ligne : exactes, expliquées (aucun arrondi intermédiaire).
fn depuis_ligne(r: &sqlx::postgres::PgRow) -> Result<ReglesEtablissement, sqlx::Error> {
    let t = |c: &str| -> Result<String, sqlx::Error> { r.try_get::<String, _>(c) };
    let o = |c: &str| -> Result<Option<String>, sqlx::Error> { r.try_get::<Option<String>, _>(c) };
    let arrondi_precision = Precision::parse(&t("arrondi_precision")?).unwrap_or_default();
    let arrondi_mode = ModeArrondi::parse(&t("arrondi_mode")?).unwrap_or_default();
    let regles = ReglesMoteur {
        echelle_m: r.try_get::<i64, _>("echelle_m")?,
        ponderation: match t("ponderation")?.as_str() {
            "points_cumules" => PonderationNotes::PointsCumules,
            _ => PonderationNotes::NotesRamenees,
        },
        non_rendu: match t("non_rendu")?.as_str() {
            "zero" => TraitementNonRendu::Zero,
            _ => TraitementNonRendu::Exclu,
        },
        bonus: bonus_depuis_colonnes(&t("bonus")?, r.try_get::<Option<i64>, _>("bonus_seuil_m")?),
        sous_matieres: match t("sous_matieres")?.as_str() {
            "toutes_les_notes" => MethodeSousMatieres::ToutesLesNotes,
            _ => MethodeSousMatieres::MoyenneDesMoyennes,
        },
        generale: match t("generale")?.as_str() {
            "non_ponderee" => PonderationGenerale::NonPonderee,
            _ => PonderationGenerale::Ponderee,
        },
        annuelle: match t("methode_annuelle")?.as_str() {
            "toutes_evaluations" => MethodeAnnuelle::ToutesLesEvaluations,
            _ => MethodeAnnuelle::MoyenneDesPeriodes,
        },
        arrondis: EtapesArrondi {
            note_ramenee: etape(
                o("arrondi_note_ramenee_precision")?,
                &t("arrondi_note_ramenee_mode")?,
            ),
            moyenne_sous_matiere: etape(
                o("arrondi_sous_matiere_precision")?,
                &t("arrondi_sous_matiere_mode")?,
            ),
            moyenne_matiere: etape(o("arrondi_matiere_precision")?, &t("arrondi_matiere_mode")?),
            moyenne_periode: etape(o("arrondi_periode_precision")?, &t("arrondi_periode_mode")?),
        },
        affichage: ArrondiRegle::new(arrondi_precision, arrondi_mode),
        plafonner_a_l_echelle: r.try_get::<bool, _>("plafonner_echelle")?,
        seuil_effectif_stats: usize::try_from(r.try_get::<i16, _>("seuil_effectif_stats")?)
            .unwrap_or(5)
            .max(SEUIL_STATS_PLANCHER),
        stats_familles: r.try_get::<bool, _>("stats_familles")?,
        mediane_familles: r.try_get::<bool, _>("mediane_familles")?,
        mediane: match t("mediane_paire")?.as_str() {
            "inferieure" => MedianePaire::Inferieure,
            "superieure" => MedianePaire::Superieure,
            _ => MedianePaire::MoyenneDesDeux,
        },
    };
    Ok(ReglesEtablissement {
        regles,
        pas_saisie_m: r.try_get::<i64, _>("pas_saisie_m")?,
        version: r.try_get::<i32, _>("version")?,
        figees: r.try_get::<bool, _>("figees")?,
    })
}

const SELECT_REGLES: &str =
    "SELECT (echelle * 1000)::bigint AS echelle_m, (pas_saisie * 1000)::bigint AS pas_saisie_m,
        arrondi_precision, arrondi_mode,
        arrondi_note_ramenee_precision, arrondi_note_ramenee_mode,
        arrondi_sous_matiere_precision, arrondi_sous_matiere_mode,
        arrondi_matiere_precision, arrondi_matiere_mode,
        arrondi_periode_precision, arrondi_periode_mode,
        ponderation, non_rendu, bonus, (bonus_seuil * 1000)::bigint AS bonus_seuil_m,
        sous_matieres, generale, methode_annuelle, plafonner_echelle, mediane_paire,
        seuil_effectif_stats, stats_familles, mediane_familles, version, (fige_le IS NOT NULL) AS figees
   FROM eva_reglage WHERE etablissement_id = $1 AND annee_id = $2";

// ---------------------------------------------------------------------------
// Types de retour
// ---------------------------------------------------------------------------

/// Origine d'une écriture de résultat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceResultat {
    Saisie,
    Appel,
    Dispense,
    Import,
    HorsLigne,
}

impl SourceResultat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Saisie => "saisie",
            Self::Appel => "appel",
            Self::Dispense => "dispense",
            Self::Import => "import",
            Self::HorsLigne => "hors_ligne",
        }
    }
}

/// Écriture d'un résultat : une note ou un statut.
#[derive(Debug, Clone)]
pub struct EcrireResultat {
    pub evaluation_id: Uuid,
    pub eleve_id: Uuid,
    pub statut: StatutCalcul,
    /// Valeur en millièmes, présente si et seulement si le statut est `Note`.
    pub valeur_m: Option<i64>,
    /// Obligatoire (3 caractères) pour une correction après l'arrêt de la période de l'élève.
    pub motif: Option<String>,
    pub commentaire_eleve: Option<String>,
    pub source: SourceResultat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultatEcrit {
    pub resultat_id: Uuid,
    pub version: i32,
    /// L'écriture était une correction après l'arrêt (motif au journal).
    pub correction: bool,
}

/// Une ligne du journal d'un résultat : qui, quand, avant, après, motif.
#[derive(Debug, Clone, PartialEq)]
pub struct LigneHistoriqueResultat {
    pub version: i32,
    pub statut_avant: Option<String>,
    pub valeur_avant_m: Option<i64>,
    pub statut_apres: String,
    pub valeur_apres_m: Option<i64>,
    pub correction: bool,
    pub motif: Option<String>,
    pub source: String,
    pub auteur: Option<Uuid>,
}

/// Période, classe et état d'un élève pour une date (R-V29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodeEleve {
    pub classe_id: Uuid,
    pub periode_id: Uuid,
    pub etat: String,
    pub date_arret_effective: Option<NaiveDate>,
}

/// Une cellule de la grille : un élève du public, sa période et si la saisie lui est ouverte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CelluleGrille {
    pub eleve_id: Uuid,
    pub classe_id: Option<Uuid>,
    pub periode_id: Option<Uuid>,
    pub etat: Option<String>,
    pub date_arret_effective: Option<NaiveDate>,
    pub saisie_ouverte: bool,
    pub rattrapage_autorise: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompteReprise {
    pub objet: String,
    pub lues: i32,
    pub creees: i32,
    pub inchangees: i32,
    pub refusees: i32,
    pub ignorees: i32,
    pub ecarts: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneReprise {
    pub objet: String,
    pub source_id: Uuid,
    pub issue: String,
    pub code: String,
    pub detail: Option<String>,
}

/// Rapport d'une exécution de la reprise M2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RapportReprise {
    pub execution_id: Uuid,
    pub comptes: Vec<CompteReprise>,
    pub lignes: Vec<LigneReprise>,
}

impl RapportReprise {
    pub fn compte(&self, objet: &str) -> Option<&CompteReprise> {
        self.comptes.iter().find(|c| c.objet == objet)
    }
}

/// Moyenne d'une matière, avec son coefficient et son explication.
#[derive(Debug, Clone, PartialEq)]
pub struct MatiereCalculee {
    pub matiere_id: Uuid,
    pub moyenne: MoyenneCalculee,
    pub coef_m: i64,
    pub hors_moyenne_generale: bool,
}

/// Les moyennes d'un élève sur une période : par matière, puis générale.
#[derive(Debug, Clone, PartialEq)]
pub struct MoyennesEleve {
    pub matieres: Vec<MatiereCalculee>,
    pub generale: MoyenneCalculee,
    /// Règles utilisées (pour expliquer le calcul).
    pub regles: ReglesMoteur,
}

/// Un résultat visible d'une famille ou d'un élève (évaluation publiée).
#[derive(Debug, Clone, PartialEq)]
pub struct ResultatPublie {
    pub evaluation_id: Uuid,
    pub titre: String,
    pub statut: String,
    pub valeur_m: Option<i64>,
    pub bareme_m: Option<i64>,
}

fn erreur_saisie(e: ErreurSaisieMoteur) -> &'static str {
    match e {
        ErreurSaisieMoteur::Incoherent => {
            "un statut non numérique ne porte pas de valeur, une note en porte une"
        }
        ErreurSaisieMoteur::Negative => "une note ne peut pas être négative",
        ErreurSaisieMoteur::HorsBareme => "note au-dessus du barème : vérifiez le barème",
        ErreurSaisieMoteur::HorsPas => {
            "la note n'est pas un multiple du pas de saisie de l'établissement"
        }
        ErreurSaisieMoteur::StatutNonSaisissable => "ce statut se calcule, il ne se saisit pas",
        ErreurSaisieMoteur::Bareme => "barème invalide",
    }
}

fn erreur_calcul(_: domain_eleve::evaluation_moteur::ErreurCalcul) -> EntDbError {
    refus("calcul impossible : données ou réglage incohérents")
}

impl PgEnt {
    // ───────────────────────── Règles de calcul ─────────────────────────

    /// Règles d'une année (défauts recommandés si l'établissement n'en a posé aucune).
    pub async fn eva_v1_regles(
        &self,
        etab: Uuid,
        annee_id: Uuid,
    ) -> Result<ReglesEtablissement, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = regles_tx(&mut tx, etab, annee_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Pose les règles d'une année (`periodes.verrouiller`). L'échelle et la méthode annuelle ne
    /// changent plus après le premier arrêt de période (R-V21, tenu par la base).
    pub async fn eva_v1_definir_regles_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        annee_id: Uuid,
        regles: &ReglesMoteur,
        pas_saisie_m: i64,
    ) -> Result<ReglesEtablissement, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if regles.echelle_m <= 0 || pas_saisie_m <= 0 || pas_saisie_m > 1000 {
            return Err(refus("échelle ou pas de saisie invalide"));
        }
        if regles.seuil_effectif_stats < SEUIL_STATS_PLANCHER || regles.seuil_effectif_stats > 100 {
            return Err(refus(
                "le seuil d'effectif des statistiques montrées aux familles est d'au moins 3",
            ));
        }
        let (bonus, bonus_seuil_m) = bonus_vers_colonnes(regles.bonus);
        let a = regles.arrondis;
        let col = |x: Option<ArrondiRegle>| x.map(|r| r.precision.as_str().to_string());
        let mode = |x: Option<ArrondiRegle>| x.map_or("demi_haut", |r| r.mode.as_str());
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO eva_reglage (id, etablissement_id, annee_id, echelle, pas_saisie, arrondi_precision,
                arrondi_mode, arrondi_note_ramenee_precision, arrondi_note_ramenee_mode,
                arrondi_sous_matiere_precision, arrondi_sous_matiere_mode, arrondi_matiere_precision,
                arrondi_matiere_mode, arrondi_periode_precision, arrondi_periode_mode, ponderation, non_rendu,
                bonus, bonus_seuil, sous_matieres, generale, methode_annuelle, plafonner_echelle, mediane_paire,
                seuil_effectif_stats, modifie_par, stats_familles, mediane_familles)
             VALUES ($1, $2, $3, $4::bigint / 1000.0, $5::bigint / 1000.0, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                     $15, $16, $17, $18, $19::bigint / 1000.0, $20, $21, $22, $23, $24, $25, $26, $27, $28)
             ON CONFLICT (etablissement_id, annee_id) DO UPDATE SET
                echelle = EXCLUDED.echelle, pas_saisie = EXCLUDED.pas_saisie,
                arrondi_precision = EXCLUDED.arrondi_precision, arrondi_mode = EXCLUDED.arrondi_mode,
                arrondi_note_ramenee_precision = EXCLUDED.arrondi_note_ramenee_precision,
                arrondi_note_ramenee_mode = EXCLUDED.arrondi_note_ramenee_mode,
                arrondi_sous_matiere_precision = EXCLUDED.arrondi_sous_matiere_precision,
                arrondi_sous_matiere_mode = EXCLUDED.arrondi_sous_matiere_mode,
                arrondi_matiere_precision = EXCLUDED.arrondi_matiere_precision,
                arrondi_matiere_mode = EXCLUDED.arrondi_matiere_mode,
                arrondi_periode_precision = EXCLUDED.arrondi_periode_precision,
                arrondi_periode_mode = EXCLUDED.arrondi_periode_mode,
                ponderation = EXCLUDED.ponderation, non_rendu = EXCLUDED.non_rendu, bonus = EXCLUDED.bonus,
                bonus_seuil = EXCLUDED.bonus_seuil, sous_matieres = EXCLUDED.sous_matieres,
                generale = EXCLUDED.generale, methode_annuelle = EXCLUDED.methode_annuelle,
                plafonner_echelle = EXCLUDED.plafonner_echelle, mediane_paire = EXCLUDED.mediane_paire,
                seuil_effectif_stats = EXCLUDED.seuil_effectif_stats, modifie_par = EXCLUDED.modifie_par,
                stats_familles = EXCLUDED.stats_familles, mediane_familles = EXCLUDED.mediane_familles",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(annee_id)
        .bind(regles.echelle_m)
        .bind(pas_saisie_m)
        .bind(regles.affichage.precision.as_str())
        .bind(regles.affichage.mode.as_str())
        .bind(col(a.note_ramenee))
        .bind(mode(a.note_ramenee))
        .bind(col(a.moyenne_sous_matiere))
        .bind(mode(a.moyenne_sous_matiere))
        .bind(col(a.moyenne_matiere))
        .bind(mode(a.moyenne_matiere))
        .bind(col(a.moyenne_periode))
        .bind(mode(a.moyenne_periode))
        .bind(serie_ponderation(regles.ponderation))
        .bind(match regles.non_rendu {
            TraitementNonRendu::Exclu => "exclu",
            TraitementNonRendu::Zero => "zero",
        })
        .bind(bonus)
        .bind(bonus_seuil_m)
        .bind(match regles.sous_matieres {
            MethodeSousMatieres::MoyenneDesMoyennes => "moyenne_des_moyennes",
            MethodeSousMatieres::ToutesLesNotes => "toutes_les_notes",
        })
        .bind(match regles.generale {
            PonderationGenerale::Ponderee => "ponderee",
            PonderationGenerale::NonPonderee => "non_ponderee",
        })
        .bind(match regles.annuelle {
            MethodeAnnuelle::MoyenneDesPeriodes => "moyenne_periodes",
            MethodeAnnuelle::ToutesLesEvaluations => "toutes_evaluations",
        })
        .bind(regles.plafonner_a_l_echelle)
        .bind(match regles.mediane {
            MedianePaire::MoyenneDesDeux => "moyenne_des_deux",
            MedianePaire::Inferieure => "inferieure",
            MedianePaire::Superieure => "superieure",
        })
        .bind(i16::try_from(regles.seuil_effectif_stats).unwrap_or(5))
        .bind(acteur)
        .bind(regles.stats_familles)
        .bind(regles.mediane_familles)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let nouvelles = regles_tx(&mut tx, etab, annee_id).await?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_v1::REGLES_MODIFIEES.to_string(),
            aggregate: AggregateKind::NotePeriode,
            aggregate_id: annee_id,
            eleve_id: None,
            payload: json!({ "annee_id": annee_id, "version": nouvelles.version }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(nouvelles)
    }

    // ───────────────────────── Période par élève ─────────────────────────

    /// Classe, période et état d'un élève pour la date d'une évaluation (R-V29). Ne lit jamais
    /// `eva_evaluation.periode_id`.
    pub async fn eva_v1_periode_eleve(
        &self,
        etab: Uuid,
        evaluation_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<PeriodeEleve>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = periode_eleve_tx(&mut tx, etab, evaluation_id, eleve_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Le public d'une évaluation, cellule par cellule (la grille). Les élèves protégés qu'un
    /// personnel ne doit pas voir en sont retirés (garde du socle 2).
    pub async fn eva_v1_grille_eleves_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        evaluation_id: Uuid,
    ) -> Result<Vec<CelluleGrille>, EntDbError> {
        Self::exiger_staff(role)?;
        let peut_lire = self
            .a_action_etablissement(etab, acteur, role, Action::NotesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // La grille d'une évaluation (les notes de toute la classe) n'est lisible que du titulaire du cours ou d'un
        // détenteur de `notes.lire` à portée établissement (M9) : un autre professeur ou le secrétariat n'y accèdent pas.
        let cours_id: Uuid = sqlx::query_scalar(
            "SELECT cours_id FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(domain_eleve::DomainError::NotFound)?;
        let titulaire: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement
                             WHERE etablissement_id = $1 AND cours_id = $2 AND prof_user_id = $3)",
        )
        .bind(etab)
        .bind(cours_id)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !titulaire && !peut_lire {
            return Err(EntDbError::Forbidden);
        }
        let lignes = sqlx::query(
            "SELECT eleve_id, classe_id, periode_id, etat, date_arret_effective, saisie_ouverte, rattrapage_autorise
               FROM eva_grille_eleves($1, $2)",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut cellules: Vec<CelluleGrille> = Vec::with_capacity(lignes.len());
        for l in &lignes {
            cellules.push(CelluleGrille {
                eleve_id: l.try_get("eleve_id").map_err(db)?,
                classe_id: l.try_get("classe_id").map_err(db)?,
                periode_id: l.try_get("periode_id").map_err(db)?,
                etat: l.try_get("etat").map_err(db)?,
                date_arret_effective: l.try_get("date_arret_effective").map_err(db)?,
                saisie_ouverte: l
                    .try_get::<Option<bool>, _>("saisie_ouverte")
                    .map_err(db)?
                    .unwrap_or(false),
                rattrapage_autorise: l.try_get("rattrapage_autorise").map_err(db)?,
            });
        }
        // Garde du socle 2 : un élève protégé n'apparaît pas à un tiers du personnel.
        let ids: Vec<Uuid> = cellules.iter().map(|c| c.eleve_id).collect();
        let profil = self.profil_champs(etab, acteur, role).await?;
        let caches = self.proteges_caches(etab, role, &profil, &ids).await?;
        cellules.retain(|c| !caches.contains(&c.eleve_id));
        cellules.sort_by_key(|c| c.eleve_id);
        Ok(cellules)
    }

    // ───────────────────────── Écriture d'un résultat ─────────────────────────

    /// Écrit (crée ou remplace) le résultat d'un élève à une évaluation.
    ///
    /// * jamais de suppression : une valeur se remplace, l'ancienne reste au journal ;
    /// * saisie ordinaire : le professeur du cours (service) tant que la période de l'élève est
    ///   ouverte ; **après l'arrêt**, seulement `notes.corriger` (direction), avec un motif, sauf
    ///   rattrapage autorisé (R-V10) ;
    /// * un élève protégé que l'acteur n'a pas le droit de voir est « introuvable » (garde du socle 2) ;
    /// * l'événement publié est neutre (R-V17).
    pub async fn eva_v1_ecrire_resultat_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &EcrireResultat,
    ) -> Result<ResultatEcrit, EntDbError> {
        Self::exiger_staff(role)?;
        self.exiger_eleve_visible(etab, acteur, role, body.eleve_id)
            .await?;
        let peut_corriger = self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = sqlx::query(
            "SELECT cours_id, (bareme * 1000)::bigint AS bareme_m, hors_bareme_autorise, etat, mode, annee_id
               FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(body.evaluation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(domain_eleve::DomainError::NotFound)?;
        let cours_id: Uuid = ev.try_get("cours_id").map_err(db)?;
        let bareme_m: Option<i64> = ev.try_get("bareme_m").map_err(db)?;
        let hors_bareme: bool = ev.try_get("hors_bareme_autorise").map_err(db)?;
        let annee_id: Uuid = ev.try_get("annee_id").map_err(db)?;
        let etat_eval: String = ev.try_get("etat").map_err(db)?;
        if etat_eval != "ouverte" {
            return Err(conflit("évaluation non ouverte à la saisie"));
        }
        let titulaire: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement
                             WHERE etablissement_id = $1 AND cours_id = $2 AND prof_user_id = $3)",
        )
        .bind(etab)
        .bind(cours_id)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !titulaire && !peut_corriger {
            return Err(EntDbError::Forbidden);
        }
        let periode = periode_eleve_tx(&mut tx, etab, body.evaluation_id, body.eleve_id).await?;
        // Ligne existante (verrouillée) : son statut et sa version décident du rattrapage et d'une écriture sans effet.
        let existant: Option<(String, i32)> = sqlx::query_as(
            "SELECT statut, version FROM eva_resultat
              WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(body.evaluation_id)
        .bind(body.eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        // Rattrapage en vigueur (dernière autorisation, date de fin non dépassée au jour de l'établissement) et
        // seulement sur une ligne absente ou restée sans note : une note déjà saisie ne s'écrase pas par ce biais.
        let rattrapage: bool = sqlx::query_scalar("SELECT eva_rattrapage_en_vigueur($1, $2, $3)")
            .bind(etab)
            .bind(body.evaluation_id)
            .bind(body.eleve_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let rattrapage = rattrapage
            && existant.as_ref().is_none_or(|(st, _)| {
                ["absent", "a_rattraper", "non_rendu"].contains(&st.as_str())
            });
        // Un élève sans classe ou sans période à la date est verrouillé comme un élève arrêté (M8).
        let arretee = periode.as_ref().is_none_or(|p| p.etat != "ouverte") && !rattrapage;
        if arretee && !peut_corriger {
            return Err(EntDbError::Forbidden);
        }
        let pas = regles_tx(&mut tx, etab, annee_id).await?.pas_saisie_m;
        let bareme = bareme_m.ok_or(refus("évaluation sans barème"))?;
        valider_saisie(body.statut, body.valeur_m, bareme, hors_bareme, pas)
            .map_err(|e| refus(erreur_saisie(e)))?;
        // (`motif_correction = $10` et non `EXCLUDED.motif_correction` : le déclencheur d'INSERT remet l'entrée à NULL.)
        // Le motif d'une correction voyage dans la ligne (`motif_correction`), jamais dans une variable de session :
        // la base l'exige après l'arrêt, le journal le garde, une écriture ordinaire le remet à NULL. La source
        // d'un résultat ne change plus après sa création (la base le refuse).
        let ligne = sqlx::query(
            "INSERT INTO eva_resultat (id, etablissement_id, evaluation_id, eleve_id, statut, valeur,
                                       commentaire_eleve, source, saisi_par, motif_correction)
             VALUES ($1, $2, $3, $4, $5, $6::bigint / 1000.0, $7, $8, $9, $10)
             ON CONFLICT (evaluation_id, eleve_id) DO UPDATE SET
                statut = EXCLUDED.statut, valeur = EXCLUDED.valeur,
                commentaire_eleve = COALESCE(EXCLUDED.commentaire_eleve, eva_resultat.commentaire_eleve),
                saisi_par = EXCLUDED.saisi_par, motif_correction = $10
             RETURNING id, version, derniere_ecriture_correction",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(body.evaluation_id)
        .bind(body.eleve_id)
        .bind(body.statut.as_str())
        .bind(body.valeur_m)
        .bind(body.commentaire_eleve.as_deref())
        .bind(body.source.as_str())
        .bind(acteur)
        .bind(body.motif.as_deref().map(str::trim).filter(|m| !m.is_empty()))
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let version: i32 = ligne.try_get("version").map_err(db)?;
        // F4 : une écriture sans effet (même statut, même valeur, rien de changé) laisse la version telle quelle :
        // aucun journal, aucun événement.
        let sans_effet = existant.as_ref().is_some_and(|(_, v)| *v == version);
        let ecrit = ResultatEcrit {
            resultat_id: ligne.try_get("id").map_err(db)?,
            version,
            correction: !sans_effet && ligne.try_get("derniere_ecriture_correction").map_err(db)?,
        };
        if !sans_effet {
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: if ecrit.correction {
                    types_v1::CORRIGEE
                } else {
                    types_v1::RESULTATS_SAISIS
                }
                .to_string(),
                aggregate: AggregateKind::Devoir,
                aggregate_id: body.evaluation_id,
                eleve_id: Some(body.eleve_id),
                payload: json!({ "evaluation_id": body.evaluation_id, "resultat_id": ecrit.resultat_id,
                                 "version": ecrit.version }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(ecrit)
    }

    /// Journal d'un résultat : qui, quand, avant, après, motif (direction, `notes.corriger`).
    pub async fn eva_v1_historique_resultat_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        resultat_id: Uuid,
    ) -> Result<Vec<LigneHistoriqueResultat>, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleve: Uuid = sqlx::query_scalar(
            "SELECT eleve_id FROM eva_resultat WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(resultat_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(domain_eleve::DomainError::NotFound)?;
        let lignes = sqlx::query(
            "SELECT version, statut_avant, (valeur_avant * 1000)::bigint AS valeur_avant_m, statut_apres,
                    (valeur_apres * 1000)::bigint AS valeur_apres_m, correction, motif, source, auteur
               FROM eva_resultat_historique
              WHERE etablissement_id = $1 AND resultat_id = $2 ORDER BY version",
        )
        .bind(etab)
        .bind(resultat_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        lignes
            .iter()
            .map(|l| {
                Ok(LigneHistoriqueResultat {
                    version: l.try_get("version")?,
                    statut_avant: l.try_get("statut_avant")?,
                    valeur_avant_m: l.try_get("valeur_avant_m")?,
                    statut_apres: l.try_get("statut_apres")?,
                    valeur_apres_m: l.try_get("valeur_apres_m")?,
                    correction: l.try_get("correction")?,
                    motif: l.try_get("motif")?,
                    source: l.try_get("source")?,
                    auteur: l.try_get("auteur")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)
    }

    // ───────────────────────── États de période ─────────────────────────

    /// Fait avancer ou reculer l'état d'évaluation d'une classe pour une période
    /// (`periodes.verrouiller` : administratif et direction ; jamais un professeur). Une
    /// réouverture exige un motif ; la machine d'états est tenue par la base.
    #[allow(clippy::too_many_arguments)]
    pub async fn eva_v1_changer_etat_periode_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode_id: Uuid,
        classe_id: Uuid,
        cible: &str,
        motif: Option<&str>,
        date_arret: Option<NaiveDate>,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if ![
            "ouverte",
            "notes_arretees",
            "appreciations_closes",
            "conseil_tenu",
            "close",
        ]
        .contains(&cible)
        {
            return Err(refus("état de période inconnu"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("SELECT eva_etat_assurer($1, $2, $3)")
            .bind(etab)
            .bind(periode_id)
            .bind(classe_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let avant: String = sqlx::query_scalar(
            "SELECT etat FROM eva_periode_etat WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3",
        )
        .bind(etab)
        .bind(periode_id)
        .bind(classe_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE eva_periode_etat
                SET etat = $4, origine = 'geste', modifie_par = $5, motif = $6,
                    date_arret_effective = COALESCE($7, date_arret_effective)
              WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3",
        )
        .bind(etab)
        .bind(periode_id)
        .bind(classe_id)
        .bind(cible)
        .bind(acteur)
        .bind(motif)
        .bind(date_arret)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if avant != cible {
            let recul = [
                "ouverte",
                "notes_arretees",
                "appreciations_closes",
                "conseil_tenu",
                "close",
            ]
            .iter()
            .position(|e| *e == cible)
                < [
                    "ouverte",
                    "notes_arretees",
                    "appreciations_closes",
                    "conseil_tenu",
                    "close",
                ]
                .iter()
                .position(|e| *e == avant);
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: if recul {
                    types_v1::PERIODE_ROUVERTE
                } else {
                    types_v1::NOTES_ARRETEES
                }
                .to_string(),
                aggregate: AggregateKind::NotePeriode,
                aggregate_id: periode_id,
                eleve_id: None,
                payload: json!({ "periode_id": periode_id, "classe_id": classe_id }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Autorise, pour un élève et une évaluation, un rattrapage noté après l'arrêt (R-V10).
    pub async fn eva_v1_autoriser_rattrapage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        evaluation_id: Uuid,
        eleve_id: Uuid,
        jusqu_au: NaiveDate,
        motif: &str,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve_id)
            .await?;
        if motif.trim().chars().count() < 3 {
            return Err(refus("un motif de trois caractères au moins est requis"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Journalisé : chaque autorisation est une nouvelle version (ajout seul). La date de fin ne peut pas être
        // déjà passée au jour de l'établissement : une autorisation périmée ne servirait à rien.
        let posees = sqlx::query(
            "INSERT INTO eva_rattrapage_autorise (id, evaluation_id, eleve_id, etablissement_id, jusqu_au, motif, par)
             SELECT $1, $2, $3, $4, $5, $6, $7 WHERE $5 >= ent_jour_etablissement($4)",
        )
        .bind(Uuid::new_v4())
        .bind(evaluation_id)
        .bind(eleve_id)
        .bind(etab)
        .bind(jusqu_au)
        .bind(motif.trim())
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if posees == 0 {
            return Err(refus("la date de fin du rattrapage est déjà passée"));
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Reprise M2 ─────────────────────────

    /// Lance la reprise des évaluations existantes pour l'établissement (`periodes.verrouiller`) ;
    /// idempotente (insertion seule), le rapport dit ce qui a été fait, refusé, ignoré ou signalé.
    pub async fn eva_v1_reprendre_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<RapportReprise, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let exec: Uuid = sqlx::query_scalar("SELECT eva_reprendre_existant($1, 'manuel')")
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_v1::REPRISE_TERMINEE.to_string(),
            aggregate: AggregateKind::NotePeriode,
            aggregate_id: exec,
            eleve_id: None,
            payload: json!({ "execution_id": exec }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        let rapport = rapport_tx(&mut tx, etab, exec).await?;
        tx.commit().await.map_err(db)?;
        Ok(rapport)
    }

    /// Rapport d'une exécution (direction). `None` pour l'exécution la plus récente.
    pub async fn eva_v1_rapport_reprise_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        execution_id: Option<Uuid>,
    ) -> Result<Option<RapportReprise>, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let exec: Option<Uuid> = match execution_id {
            Some(e) => Some(e),
            None => sqlx::query_scalar(
                "SELECT id FROM eva_reprise_execution WHERE etablissement_id = $1 ORDER BY lancee_le DESC, id DESC LIMIT 1",
            )
            .bind(etab)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?,
        };
        let r = match exec {
            Some(e) => Some(rapport_tx(&mut tx, etab, e).await?),
            None => None,
        };
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    // ───────────────────────── Calcul des moyennes ─────────────────────────

    /// Moyennes d'un élève pour **sa famille ou lui-même** : les seules évaluations **publiées** entrent dans le
    /// calcul (`publiees_seulement` est forcé, l'appelant ne le choisit pas), pour un lien famille ou un compte
    /// élève qui autorise la lecture des notes, et sous la garde de l'élève protégé du socle 2.
    pub async fn eva_v1_moyennes_famille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        periode_id: Uuid,
    ) -> Result<MoyennesEleve, EntDbError> {
        if !self
            .actor_can_read_notes(etab, acteur, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = moyennes_tx(&mut tx, etab, eleve_id, periode_id, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Moyennes d'un élève pour **le personnel** : toutes les évaluations ouvertes, publiées ou non. Exige le
    /// droit `notes.lire` à portée établissement et la garde de l'élève protégé du socle 2 (un tiers du
    /// personnel ne calcule pas les moyennes d'un élève qu'il ne peut pas voir).
    pub async fn eva_v1_moyennes_personnel_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        periode_id: Uuid,
    ) -> Result<MoyennesEleve, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::NotesLire)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = moyennes_tx(&mut tx, etab, eleve_id, periode_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Calcul **sans aucun contrôle d'accès** : réservé aux tests du crate. Aucune route ni aucun appelant de
    /// production ne l'atteint (M1) : ils passent par `eva_v1_moyennes_famille_for_actor` ou
    /// `eva_v1_moyennes_personnel_for_actor`.
    #[cfg(all(test, feature = "labo"))]
    pub(crate) async fn eva_v1_moyennes_eleve_periode(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
        periode_id: Uuid,
        publiees_seulement: bool,
    ) -> Result<MoyennesEleve, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = moyennes_tx(&mut tx, etab, eleve_id, periode_id, publiees_seulement).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    // ───────────────────────── Arrêt automatique (M8) ─────────────────────────

    /// Passe en `notes_arretees` (origine `auto`) les classes de l'établissement dont la date d'arrêt est atteinte
    /// au jour de l'établissement (`jour` : jour explicite, sinon celui de l'établissement) et dont le réglage
    /// `arret_auto` est actif. IDEMPOTENT ; ne ré-arrête jamais une classe rouverte par la direction. Rend le
    /// nombre de classes arrêtées. Appelé par le planificateur quotidien (`passages_quotidiens`).
    pub async fn eva_v1_arret_auto_au(
        &self,
        etab: Uuid,
        jour: Option<NaiveDate>,
    ) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n: i32 = sqlx::query_scalar("SELECT eva_arret_auto($1, $2)")
            .bind(etab)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(usize::try_from(n).unwrap_or(0))
    }

    // ───────────────────────── Lecture publiée (R-V7) ─────────────────────────

    /// Ce qu'une famille ou un élève peut lire : les résultats des évaluations **publiées**, pour
    /// un élève dont le lien (ou le compte) l'autorise (`actor_can_read_notes`). Une note non
    /// publiée n'est jamais visible ; la note privée du professeur n'est jamais dans la vue.
    pub async fn eva_v1_resultats_publies_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<ResultatPublie>, EntDbError> {
        if !self
            .actor_can_read_notes(etab, acteur, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = sqlx::query(
            "SELECT evaluation_id, titre, statut, (valeur * 1000)::bigint AS valeur_m, (bareme * 1000)::bigint AS bareme_m
               FROM eva_resultat_publie WHERE etablissement_id = $1 AND eleve_id = $2
              ORDER BY date_eval, evaluation_id",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        lignes
            .iter()
            .map(|l| {
                Ok(ResultatPublie {
                    evaluation_id: l.try_get("evaluation_id")?,
                    titre: l.try_get("titre")?,
                    statut: l.try_get("statut")?,
                    valeur_m: l.try_get("valeur_m")?,
                    bareme_m: l.try_get("bareme_m")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)
    }
}

// ---------------------------------------------------------------------------
// Fonctions de transaction
// ---------------------------------------------------------------------------

async fn regles_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    annee_id: Uuid,
) -> Result<ReglesEtablissement, EntDbError> {
    let ligne = sqlx::query(SELECT_REGLES)
        .bind(etab)
        .bind(annee_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
    match ligne {
        Some(l) => depuis_ligne(&l).map_err(db),
        None => Ok(ReglesEtablissement::default()),
    }
}

async fn periode_eleve_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
    eleve_id: Uuid,
) -> Result<Option<PeriodeEleve>, EntDbError> {
    let l = sqlx::query(
        "SELECT classe_id, periode_id, etat, date_arret_effective FROM eva_periode_eleve($1, $2, $3)",
    )
    .bind(etab)
    .bind(evaluation_id)
    .bind(eleve_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    l.map(|l| {
        Ok(PeriodeEleve {
            classe_id: l.try_get("classe_id")?,
            periode_id: l.try_get("periode_id")?,
            etat: l.try_get("etat")?,
            date_arret_effective: l.try_get("date_arret_effective")?,
        })
    })
    .transpose()
    .map_err(db)
}

async fn rapport_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    exec: Uuid,
) -> Result<RapportReprise, EntDbError> {
    let comptes = sqlx::query(
        "SELECT objet, lues, creees, inchangees, refusees, ignorees, ecarts FROM eva_reprise_compte
          WHERE etablissement_id = $1 AND execution_id = $2 ORDER BY objet",
    )
    .bind(etab)
    .bind(exec)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let lignes = sqlx::query(
        "SELECT objet, source_id, issue, code, detail FROM eva_reprise_ligne
          WHERE etablissement_id = $1 AND execution_id = $2 ORDER BY objet, code, source_id",
    )
    .bind(etab)
    .bind(exec)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    Ok(RapportReprise {
        execution_id: exec,
        comptes: comptes
            .iter()
            .map(|c| {
                Ok(CompteReprise {
                    objet: c.try_get("objet")?,
                    lues: c.try_get("lues")?,
                    creees: c.try_get("creees")?,
                    inchangees: c.try_get("inchangees")?,
                    refusees: c.try_get("refusees")?,
                    ignorees: c.try_get("ignorees")?,
                    ecarts: c.try_get("ecarts")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)?,
        lignes: lignes
            .iter()
            .map(|l| {
                Ok(LigneReprise {
                    objet: l.try_get("objet")?,
                    source_id: l.try_get("source_id")?,
                    issue: l.try_get("issue")?,
                    code: l.try_get("code")?,
                    detail: l.try_get("detail")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)?,
    })
}

/// Charge les résultats d'un élève pour une période, calcule moyennes de matière et générale.
async fn moyennes_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    eleve_id: Uuid,
    periode_id: Uuid,
    publiees_seulement: bool,
) -> Result<MoyennesEleve, EntDbError> {
    let periode = sqlx::query(
        "SELECT annee_id, date_debut, date_fin FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(periode_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or(domain_eleve::DomainError::NotFound)?;
    let annee_id: Uuid = periode.try_get("annee_id").map_err(db)?;
    let date_fin: NaiveDate = periode.try_get("date_fin").map_err(db)?;
    let date_debut: NaiveDate = periode.try_get("date_debut").map_err(db)?;
    let reglage = regles_tx(tx, etab, annee_id).await?;
    let regles = reglage.regles;

    // Résultats dont la PÉRIODE DE L'ÉLÈVE (R-V29) est celle demandée. Jamais `eva_evaluation.periode_id`.
    let lignes = sqlx::query(
        "SELECT r.evaluation_id, ev.titre, (ev.bareme * 1000)::bigint AS bareme_m, (ev.coef * 1000)::bigint AS coef_m,
                ev.comptage, ev.hors_bareme_autorise, ev.etat, ev.cours_id, ev.matiere_id AS sous_matiere_id,
                co.matiere_id AS matiere_id, co.classe_id AS cours_classe_id, r.statut, (r.valeur * 1000)::bigint AS valeur_m
           FROM eva_resultat r
           JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
           JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
          WHERE r.etablissement_id = $1 AND r.eleve_id = $2 AND ev.mode = 'note'
            AND ev.etat IN ('ouverte', 'annulee')
            AND ($4 = false OR ev.publiee_le IS NOT NULL)
            AND (SELECT pe.periode_id FROM eva_periode_eleve_jour(ev.etablissement_id, r.eleve_id, ev.date_eval) pe) = $3
          ORDER BY ev.date_eval, ev.id",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(periode_id)
    .bind(publiees_seulement)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut par_matiere: BTreeMap<Uuid, Vec<EntreeCalcul>> = BTreeMap::new();
    // Classe du cours de l'évaluation, par matière : dernier recours pour un coefficient (M2).
    let mut classe_du_cours: BTreeMap<Uuid, Uuid> = BTreeMap::new();
    for l in &lignes {
        let matiere: Option<Uuid> = l.try_get("matiere_id").map_err(db)?;
        if let Some(c) = l
            .try_get::<Option<Uuid>, _>("cours_classe_id")
            .map_err(db)?
        {
            classe_du_cours
                .entry(matiere.unwrap_or(Uuid::nil()))
                .or_insert(c);
        }
        let statut = StatutCalcul::parse(&l.try_get::<String, _>("statut").map_err(db)?)
            .ok_or(refus("statut de résultat inconnu"))?;
        let evaluation = EvalCalcul {
            id: l.try_get("evaluation_id").map_err(db)?,
            titre: l.try_get("titre").map_err(db)?,
            bareme_m: l.try_get("bareme_m").map_err(db)?,
            coef_m: l.try_get("coef_m").map_err(db)?,
            comptage: Comptage::parse(&l.try_get::<String, _>("comptage").map_err(db)?)
                .unwrap_or_default(),
            hors_bareme_autorise: l.try_get("hors_bareme_autorise").map_err(db)?,
            sous_matiere_id: l.try_get("sous_matiere_id").map_err(db)?,
            cours_id: l.try_get("cours_id").map_err(db)?,
            annulee: l.try_get::<String, _>("etat").map_err(db)? == "annulee",
        };
        par_matiere
            .entry(matiere.unwrap_or(Uuid::nil()))
            .or_default()
            .push(EntreeCalcul {
                evaluation,
                statut,
                valeur_m: l.try_get("valeur_m").map_err(db)?,
            });
    }

    // Coefficients de sous-matière de l'année.
    let coefs_sm: Vec<CoefSousMatiere> = sqlx::query(
        "SELECT matiere_id, (coef * 1000)::bigint AS coef_m FROM eva_coefficient_sous_matiere
          WHERE etablissement_id = $1 AND annee_id = $2
            AND valable_du <= $3 AND (valable_au IS NULL OR valable_au >= $3)",
    )
    .bind(etab)
    .bind(annee_id)
    .bind(date_fin)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    .iter()
    .map(|r| {
        Ok(CoefSousMatiere {
            sous_matiere_id: Some(r.try_get("matiere_id")?),
            coef_m: r.try_get("coef_m")?,
        })
    })
    .collect::<Result<_, sqlx::Error>>()
    .map_err(db)?;

    // Classe de l'élève pour les coefficients de matière : la DERNIÈRE classe connue dans la période (un élève
    // sorti ou changé de classe avant la fin de la période garde la sienne), sinon la classe du cours de
    // l'évaluation. Jamais de retour silencieux au coefficient 1 : voir l'avertissement plus bas (M2).
    let classe_periode: Option<Uuid> = sqlx::query_scalar(
        "SELECT a.classe_id FROM ent_affectation a
          WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.classe_id IS NOT NULL
            AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $3)
          ORDER BY a.date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(date_debut)
    .bind(date_fin)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;

    let mut matieres = Vec::new();
    let mut pour_generale = Vec::new();
    let mut avertissements: Vec<String> = Vec::new();
    for (matiere_id, entrees) in &par_matiere {
        let moyenne = moyenne_matiere(entrees, &coefs_sm, &regles).map_err(erreur_calcul)?;
        let classe = classe_periode.or_else(|| classe_du_cours.get(matiere_id).copied());
        let trouve =
            coefficient_matiere_tx(tx, etab, annee_id, *matiere_id, classe, date_fin).await?;
        let (coef_m, hors) = match trouve {
            Some(x) => x,
            None => {
                avertissements.push(format!(
                    "Coefficient de la matière introuvable pour la classe de l'élève : coefficient 1 \
                     appliqué faute de mieux, à vérifier par la direction ({}).",
                    if *matiere_id == Uuid::nil() { "matière non renseignée" } else { "voir le référentiel des coefficients" }
                ));
                (1000, false)
            }
        };
        pour_generale.push(MatiereMoyenne {
            matiere_id: *matiere_id,
            moyenne: moyenne.exacte,
            coef_m,
            hors_moyenne_generale: hors,
        });
        matieres.push(MatiereCalculee {
            matiere_id: *matiere_id,
            moyenne,
            coef_m,
            hors_moyenne_generale: hors,
        });
    }
    let mut generale = moyenne_generale(&pour_generale, &regles).map_err(erreur_calcul)?;
    generale.avertissements.extend(avertissements);
    Ok(MoyennesEleve {
        matieres,
        generale,
        regles,
    })
}

/// Coefficient de la matière pour la classe à la date : la cible la plus précise gagne
/// (classe > MEF > niveau > défaut) ; `None` sans ligne (l'appelant le dit, il ne suppose rien).
async fn coefficient_matiere_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    annee_id: Uuid,
    matiere_id: Uuid,
    classe: Option<Uuid>,
    jour: NaiveDate,
) -> Result<Option<(i64, bool)>, EntDbError> {
    let l = sqlx::query(
        "SELECT (cm.coef * 1000)::bigint AS coef_m, cm.hors_moyenne_generale
           FROM eva_coefficient_matiere cm
           LEFT JOIN ent_classe c ON c.id = $5 AND c.etablissement_id = cm.etablissement_id
          WHERE cm.etablissement_id = $1 AND cm.annee_id = $2 AND cm.matiere_id = $3
            AND cm.valable_du <= $4 AND (cm.valable_au IS NULL OR cm.valable_au >= $4)
            AND (cm.classe_id = c.id
                 OR (cm.mef_id IS NOT NULL AND cm.mef_id = c.mef_id)
                 OR (cm.niveau_id IS NOT NULL AND cm.niveau_id = COALESCE(c.niveau_id,
                        (SELECT m.niveau_id FROM ent_mef m WHERE m.id = c.mef_id AND m.etablissement_id = c.etablissement_id)))
                 OR cm.defaut)
          ORDER BY CASE WHEN cm.classe_id IS NOT NULL THEN 0 WHEN cm.mef_id IS NOT NULL THEN 1
                        WHEN cm.niveau_id IS NOT NULL THEN 2 ELSE 3 END
          LIMIT 1",
    )
    .bind(etab)
    .bind(annee_id)
    .bind(matiere_id)
    .bind(jour)
    .bind(classe)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    l.map(|l| Ok((l.try_get("coef_m")?, l.try_get("hors_moyenne_generale")?)))
        .transpose()
        .map_err(db)
}

/// Valeur exacte → millièmes d'affichage selon les règles (outil de test et d'export).
pub fn en_milliemes(r: &ReglesMoteur, x: Rat) -> Option<i64> {
    r.affichage.en_milliemes(x).ok()
}
