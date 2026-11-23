//! Évaluations (socle 2, lot S2-7) : périodes et règles, relevé, vue de classe, services évaluables.
//!
//! **V3b (M2-M3 du plan de migration)** : tout ce qui se LIT passe par `eva_*`. Le calcul d'une
//! moyenne n'existe qu'à un endroit, le moteur unique de V2 (`domain_eleve::evaluation_moteur`,
//! chargé par `evaluations_v1::moyennes_tx`) : la vue de classe, le relevé, le bulletin, le
//! conseil et la grille V3 lisent donc la **même** valeur, au centième. Les anciennes routes qui
//! ÉCRIVAIENT dans `ent_devoir` / `ent_note` (grille historique) sont fermées : la saisie, la
//! publication et la correction se font par la grille V3 (`/eva/*`).
//!
//! Droits (moteur D-017) :
//! - statut des périodes et règles de calcul : `periodes.verrouiller` ; coefficients de matière :
//!   `matieres.gerer` ;
//! - familles et élèves : `actor_can_read_notes` (lien actif + `peut_voir_notes`, ou soi),
//!   **évaluations publiées seulement**.
//!
//! Tous les faits sont publiés dans la transaction de l'écriture (outbox S2-0).

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use domain_eleve::evaluation::Arrondi;
use domain_eleve::evaluation_moteur::{statistiques, Precision, Rat, ReglesMoteur, Statistiques};
use domain_eleve::{today_utc, DomainError, NotePeriodeStatut};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::evaluations::{
    CoefMatiereBody, LigneClasseDto, MatiereColonneDto, PeriodeNotesDto, ReglesCalculDto,
    ReleveEleveDto, ReleveEvaluationDto, ReleveMatiereDto, ServiceEvaluationDto, StatsDto,
    VueClasseDto,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations_v1::{coefficient_matiere_tx, moyennes_tx, regles_tx, ReglesEtablissement};
use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Types d'événements du lot S2-7 (les noms existants d'`ent_events::types` sont réutilisés).
pub mod types_eva {
    pub const EVALUATION_PUBLIEE: &str = "evaluation.publiee";
    pub const RESULTATS_SAISIS: &str = "evaluation.resultats_saisis";
    pub const NOTE_CORRIGEE: &str = "note.corrigee";
    pub const APPRECIATION_GENERALE: &str = "evaluation.appreciation_generale";
    pub const REGLES_MODIFIEES: &str = "evaluation.regles_modifiees";
    pub const CONSEIL_DECISION: &str = "conseil.decision_prise";
    pub const ORIENTATION_AVIS: &str = "orientation.avis_donne";
    pub const ORIENTATION_REPONSE: &str = "orientation.reponse_famille";
    pub const BILAN_CYCLE_SAISI: &str = "evaluation.bilan_cycle_saisi";
    pub const BILAN_CYCLE_VALIDE: &str = "evaluation.bilan_cycle_valide";
    pub const BULLETIN_GENERE: &str = "evaluation.bulletin_genere";
    pub const EXPORT_REALISE: &str = "export.realise";
}

pub(crate) fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("evaluations", err))
}

pub(crate) fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

pub(crate) fn refus(msg: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(msg)
}

pub(crate) fn conflit(msg: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(msg)
}

/// Texte libre borné, sans caractère de contrôle (hors saut de ligne).
pub(crate) fn texte_borne(v: &str, max: usize) -> Result<String, EntDbError> {
    let v = v.trim();
    if v.chars().count() > max || v.chars().any(|c| c.is_control() && c != '\n') {
        return Err(refus("texte trop long ou caractères interdits"));
    }
    Ok(v.to_string())
}


/// Relit le statut de la période **dans la transaction** : il est DÉRIVÉ de la machine d'états
/// (`eva_periode_etat`, vue `ent_note_periode_v`), jamais de la colonne gelée de `ent_note_periode`.
/// Un verrou partagé sur les états de classe de la période fait attendre un changement concurrent.
pub(crate) async fn statut_periode_tx(
    tx: &mut sqlx::PgConnection,
    etab: Uuid,
    periode_id: Uuid,
) -> Result<NotePeriodeStatut, EntDbError> {
    sqlx::query(
        "SELECT 1 FROM eva_periode_etat e
           JOIN ent_note_periode np ON np.etablissement_id = e.etablissement_id
          WHERE np.etablissement_id = $1 AND np.id = $2
            AND e.periode_id = ent_periode_de_note_pour_classe(e.etablissement_id, np.id, e.classe_id)
          FOR SHARE OF e",
    )
    .bind(etab)
    .bind(periode_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT statut FROM ent_note_periode_v WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(periode_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?;
    let (s,) = row.ok_or_else(introuvable)?;
    Ok(NotePeriodeStatut::parse(&s).unwrap_or(NotePeriodeStatut::Fermee))
}

// ───────────────────────────── Lectures internes ─────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct PeriodeRow {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub statut: NotePeriodeStatut,
}

impl PeriodeRow {
    pub(crate) fn dto(&self) -> PeriodeNotesDto {
        PeriodeNotesDto {
            id: self.id,
            code: self.code.clone(),
            libelle: self.libelle.clone(),
            date_debut: self.date_debut,
            date_fin: self.date_fin,
            statut: self.statut.as_str().to_string(),
        }
    }
}


#[derive(Debug, Clone)]
pub(crate) struct ServiceRow {
    pub id: Uuid,
    /// Année du service (lue pour les contrôles futurs de bascule d'année).
    pub _annee_id: Uuid,
    pub prof_user_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub cible_code: String,
    pub professeur_principal: bool,
}

impl ServiceRow {
    fn dto(&self) -> Option<ServiceEvaluationDto> {
        Some(ServiceEvaluationDto {
            service_id: self.id,
            classe_id: self.classe_id,
            groupe_id: self.groupe_id,
            cible_code: self.cible_code.clone(),
            matiere_id: self.matiere_id?,
            matiere_libelle: self.matiere_libelle.clone(),
            prof_user_id: self.prof_user_id,
            professeur_principal: self.professeur_principal,
        })
    }
}

type ServiceTuple = (
    Uuid,
    Uuid,
    Uuid,
    Option<Uuid>,
    String,
    Option<Uuid>,
    Option<Uuid>,
    Option<Uuid>,
    String,
    bool,
);

const SELECT_SERVICE: &str = "SELECT s.id, s.annee_id, s.prof_user_id, s.matiere_id,
        COALESCE(m.libelle, NULLIF(s.matiere_libelle, ''), '—'),
        s.classe_id, s.groupe_id, COALESCE(s.classe_id, g.classe_id),
        COALESCE(c.code, g.code, ''), s.professeur_principal
   FROM ent_service_enseignement s
   LEFT JOIN ent_matiere m ON m.id = s.matiere_id AND m.etablissement_id = s.etablissement_id
   LEFT JOIN ent_classe c ON c.id = s.classe_id AND c.etablissement_id = s.etablissement_id
   LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
  WHERE s.etablissement_id = $1";

fn service_de(t: ServiceTuple) -> ServiceRow {
    ServiceRow {
        id: t.0,
        _annee_id: t.1,
        prof_user_id: t.2,
        matiere_id: t.3,
        matiere_libelle: t.4,
        classe_id: t.5,
        groupe_id: t.6,
        cible_code: t.8,
        professeur_principal: t.9,
    }
}

/// Valeur d'AFFICHAGE d'une moyenne : millièmes, déjà arrondis par la règle d'affichage du moteur unique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Aff(pub i64);

impl Aff {
    pub(crate) fn valeur(self) -> f64 {
        self.0 as f64 / 1000.0
    }
}

/// Moyenne, minimum, maximum d'une série de moyennes (valeurs d'affichage).
#[derive(Debug, Clone, Copy)]
pub(crate) struct StatsAff {
    pub moyenne: Aff,
    pub min: Aff,
    pub max: Aff,
    pub effectif: usize,
    /// Statistiques brutes du moteur : seule `Statistiques::pour_famille` décide de ce qu'on montre hors du personnel.
    pub brut: Statistiques,
}

fn stats_dto(s: Option<StatsAff>) -> StatsDto {
    match s {
        Some(s) => StatsDto {
            moyenne: Some(s.moyenne.valeur()),
            min: Some(s.min.valeur()),
            max: Some(s.max.valeur()),
            effectif: s.effectif as u32,
        },
        None => StatsDto::default(),
    }
}

/// Moyennes de tous les élèves d'une classe sur une période, calculées par le moteur UNIQUE (V2).
#[derive(Debug, Clone)]
pub(crate) struct CalculClasse {
    pub classe_id: Uuid,
    pub classe_code: String,
    pub periode: PeriodeRow,
    /// Précision d'affichage de l'établissement (« centieme », « dixieme », « demi_point »…).
    pub arrondi: String,
    pub regles: ReglesMoteur,
    pub eleves: Vec<(Uuid, String, String)>,
    pub matieres: Vec<(Uuid, String, String, f64)>,
    pub moyennes: HashMap<(Uuid, Uuid), Aff>,
    pub generales: HashMap<Uuid, Aff>,
    pub stats_matiere: HashMap<Uuid, StatsAff>,
    pub stats_generale: Option<StatsAff>,
}

impl CalculClasse {
    /// V3c (M4) : statistiques de matière **montrables hors du personnel** (bulletin, livret : documents remis aux
    /// familles). V3d (M-3) : décidé par `Statistiques::pour_famille`, comme le relevé : réglage `stats_familles` coupé →
    /// rien ; sous `seuil_effectif_stats` (plancher 3, 5 par défaut, R-V12) → rien : une moyenne de 2 élèves identifie
    /// l'autre.
    pub(crate) fn stats_matiere_montrables(&self, matiere: &Uuid) -> Option<&StatsAff> {
        self.stats_matiere
            .get(matiere)
            .filter(|s| s.brut.pour_famille(&self.regles).is_some())
    }

    /// Idem pour la moyenne générale de la classe.
    pub(crate) fn stats_generale_montrables(&self) -> Option<&StatsAff> {
        self.stats_generale
            .as_ref()
            .filter(|s| s.brut.pour_famille(&self.regles).is_some())
    }
}


impl PgEnt {
    /// Période de notes, avec son statut DÉRIVÉ (machine d'états de ses classes).
    pub(crate) async fn eva_periode(
        &self,
        etab: Uuid,
        periode_id: Uuid,
    ) -> Result<PeriodeRow, EntDbError> {
        let row: Option<(Uuid, String, String, NaiveDate, NaiveDate, String)> = sqlx::query_as(
            "SELECT id, code, libelle, date_debut, date_fin, statut
               FROM ent_note_periode_v WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(periode_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let (id, code, libelle, date_debut, date_fin, statut) = row.ok_or_else(introuvable)?;
        Ok(PeriodeRow {
            id,
            code,
            libelle,
            date_debut,
            date_fin,
            statut: NotePeriodeStatut::parse(&statut).unwrap_or(NotePeriodeStatut::Fermee),
        })
    }

    /// Période du référentiel (`ent_periode`) qui correspond à une période de notes pour la périodisation d'UNE classe
    /// (SQL `ent_periode_de_note_pour_classe`) : celle que lisent le moteur et la machine d'états de cette classe.
    pub(crate) async fn eva_periode_pour_classe(
        &self,
        etab: Uuid,
        periode_note: Uuid,
        classe_id: Uuid,
    ) -> Result<Option<Uuid>, EntDbError> {
        let p: Option<Uuid> = sqlx::query_scalar("SELECT ent_periode_de_note_pour_classe($1, $2, $3)")
            .bind(etab)
            .bind(periode_note)
            .bind(classe_id)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?;
        Ok(p)
    }

    /// Effectif d'une classe ou d'un groupe sur la période (affectations du
    /// référentiel qui la chevauchent), trié par nom.
    pub(crate) async fn eva_effectif(
        &self,
        etab: Uuid,
        classe_id: Option<Uuid>,
        groupe_id: Option<Uuid>,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<(Uuid, String, String)>, EntDbError> {
        sqlx::query_as(
            "SELECT DISTINCT e.id, e.nom, e.prenom
               FROM ent_affectation a
               JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1
                AND (($2::uuid IS NOT NULL AND a.classe_id = $2
                      -- V5 (D15) : un élève n'est dans l'effectif QUE de sa classe de rattachement pour la période.
                      AND eva_classe_rattachement(a.etablissement_id, a.eleve_id, $4, $5) = $2)
                     OR ($3::uuid IS NOT NULL AND a.groupe_id = $3))
                AND a.date_debut <= $5 AND (a.date_fin IS NULL OR a.date_fin >= $4)
                AND (e.date_sortie IS NULL OR e.date_sortie >= $4)
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(classe_id)
        .bind(groupe_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }

    /// Professeur principal de la classe (année du service ou courante) ?
    pub(crate) async fn eva_est_pp(
        &self,
        etab: Uuid,
        acteur: Uuid,
        classe_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement
                             WHERE etablissement_id = $1 AND prof_user_id = $2
                               AND classe_id = $3 AND professeur_principal
                               AND EXISTS (SELECT 1 FROM ent_annee_scolaire y
                                            WHERE y.id = annee_id AND y.etablissement_id = $1
                                              AND y.date_debut <= $4 AND y.date_fin >= $4))",
        )
        .bind(etab)
        .bind(acteur)
        .bind(classe_id)
        .bind(today_utc())
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// L'acteur enseigne-t-il dans la classe (service sur la classe ou un de ses groupes) ?
    pub(crate) async fn eva_enseigne_classe(
        &self,
        etab: Uuid,
        acteur: Uuid,
        classe_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement s
                              LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
                             WHERE s.etablissement_id = $1 AND s.prof_user_id = $2
                               AND (s.classe_id = $3 OR g.classe_id = $3)
                               AND EXISTS (SELECT 1 FROM ent_annee_scolaire y
                                            WHERE y.id = s.annee_id AND y.etablissement_id = $1
                                              AND y.date_debut <= $4 AND y.date_fin >= $4))",
        )
        .bind(etab)
        .bind(acteur)
        .bind(classe_id)
        .bind(today_utc())
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Classe de l'élève sur la période (affectation du référentiel), si une.
    pub(crate) async fn eva_classe_eleve(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Option<Uuid>, EntDbError> {
        // V5 : une seule implémentation (SQL `eva_classe_rattachement`), la même que celle de l'effectif.
        let c: Option<Uuid> = sqlx::query_scalar("SELECT eva_classe_rattachement($1, $2, $3, $4)")
            .bind(etab)
            .bind(eleve_id)
            .bind(debut)
            .bind(fin)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?;
        Ok(c)
    }

    // ───────────────────────────── Référentiels ─────────────────────────────

    /// Services évaluables : ceux du professeur, ou tous ceux de l'année courante pour
    /// qui a `notes.saisir` / `notes.lire` sur l'établissement.
    pub async fn services_evaluables_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<ServiceEvaluationDto>, EntDbError> {
        if Role::parse(role).is_none_or(|r| !r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        let tous = self
            .a_action_etablissement(etab, acteur, role, Action::NotesSaisir)
            .await?
            || self
                .a_action_etablissement(etab, acteur, role, Action::NotesLire)
                .await?;
        let rows: Vec<ServiceTuple> = sqlx::query_as(&format!(
            "{SELECT_SERVICE} AND s.matiere_id IS NOT NULL AND ($2 OR s.prof_user_id = $3)
               AND EXISTS (SELECT 1 FROM ent_annee_scolaire y
                            WHERE y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
                              AND y.date_debut <= $4 AND y.date_fin >= $4)
             ORDER BY 9, 5"
        ))
        .bind(etab)
        .bind(tous)
        .bind(acteur)
        .bind(today_utc())
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(rows.into_iter().filter_map(|t| service_de(t).dto()).collect())
    }

    /// Périodes de notes de l'établissement (tout membre connecté) ; le statut est celui de la machine d'états.
    pub async fn periodes_notes(&self, etab: Uuid) -> Result<Vec<PeriodeNotesDto>, EntDbError> {
        let rows: Vec<(Uuid, String, String, NaiveDate, NaiveDate, String)> = sqlx::query_as(
            "SELECT id, code, libelle, date_debut, date_fin, statut
               FROM ent_note_periode_v WHERE etablissement_id = $1 ORDER BY date_debut, code",
        )
        .bind(etab)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, code, libelle, date_debut, date_fin, statut)| PeriodeNotesDto {
                id,
                code,
                libelle,
                date_debut,
                date_fin,
                statut,
            })
            .collect())
    }

    /// EVA-07 : ouvrir, passer en « appréciations seulement » ou fermer une période (`periodes.verrouiller`).
    ///
    /// **V3b** : le statut n'est plus écrit dans `ent_note_periode` (colonne gelée) ; il est porté par la machine
    /// d'états de CHAQUE classe de la période (`eva_periode_etat`, un cran à la fois) : « partielle_appreciations »
    /// = notes arrêtées, « fermee » = appréciations closes. Toute transition est aussi inscrite dans le journal
    /// en ajout seul `ent_note_periode_journal` (acteur, avant, après, motif), dans la même transaction.
    /// **Rouvrir** exige un **motif écrit** d'au moins 3 caractères.
    pub async fn changer_statut_periode_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode_id: Uuid,
        statut: &str,
        motif: Option<&str>,
    ) -> Result<PeriodeNotesDto, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let statut = NotePeriodeStatut::parse(statut).ok_or(refus("statut de période inconnu"))?;
        let motif = motif.map(str::trim).filter(|m| !m.is_empty());
        let rang = |s: NotePeriodeStatut| match s {
            NotePeriodeStatut::Ouverte => 0usize,
            NotePeriodeStatut::PartielleAppreciations => 1,
            NotePeriodeStatut::Fermee => 2,
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Verrou de ligne : deux changements concurrents sont sérialisés et l'« avant » journalisé est celui
        // réellement remplacé.
        let existe: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_note_periode WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(periode_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        existe.ok_or_else(introuvable)?;
        let avant_s: String = sqlx::query_scalar(
            "SELECT statut FROM ent_note_periode_v WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(periode_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let avant_statut = NotePeriodeStatut::parse(&avant_s).unwrap_or(NotePeriodeStatut::Fermee);
        let reouverture = rang(statut) < rang(avant_statut);
        if reouverture && motif.is_none_or(|m| m.chars().count() < 3) {
            return Err(refus(
                "rouvrir une période exige un motif écrit (trace dans le journal de la période)",
            ));
        }
        if let Some(m) = motif {
            if m.chars().count() > 500 {
                return Err(refus("motif trop long (500 caractères au plus)"));
            }
        }
        // Les classes des années que la période recouvre, chacune avec SA période du référentiel (périodisation propre).
        let classes: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT c.id, ent_periode_de_note_pour_classe(np.etablissement_id, np.id, c.id)
               FROM ent_note_periode np
               JOIN ent_annee_scolaire y ON y.etablissement_id = np.etablissement_id
                                        AND y.date_debut <= np.date_fin AND y.date_fin >= np.date_debut
               JOIN ent_classe c ON c.etablissement_id = np.etablissement_id AND c.annee_id = y.id
              WHERE np.etablissement_id = $1 AND np.id = $2
              ORDER BY c.id",
        )
        .bind(etab)
        .bind(periode_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut alignees = 0usize;
        for (classe, pe) in classes {
            let Some(pe) = pe else { continue };
            Self::aligner_classe_sur_statut_tx(&mut tx, etab, acteur, pe, classe, rang(statut), motif).await?;
            alignees += 1;
        }
        if alignees == 0 {
            return Err(refus(
                "aucune classe n'a de période du référentiel qui recouvre cette période de notes : rien à faire avancer",
            ));
        }
        sqlx::query(
            "INSERT INTO ent_note_periode_journal
                 (id, etablissement_id, periode_id, statut_avant, statut_apres, motif, acteur_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(periode_id)
        .bind(avant_statut.as_str())
        .bind(statut.as_str())
        .bind(motif)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::NOTE_PERIODE_STATUT.to_string(),
            aggregate: AggregateKind::NotePeriode,
            aggregate_id: periode_id,
            eleve_id: None,
            payload: json!({
                "statut": statut.as_str(),
                "avant": avant_statut.as_str(),
                "reouverture": reouverture,
            }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(self.eva_periode(etab, periode_id).await?.dto())
    }

    /// Fait avancer (un cran à la fois) ou reculer (réouverture motivée) l'état d'une classe jusqu'au rang visé :
    /// 0 ouverte, 1 notes arrêtées (« partielle_appreciations »), 2 appréciations closes (« fermee »). Une classe
    /// déjà au rang visé ou au-delà d'un rang « fermee » ne bouge pas.
    pub(crate) async fn aligner_classe_sur_statut_tx(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        acteur: Uuid,
        periode_eva: Uuid,
        classe_id: Uuid,
        rang_vise: usize,
        motif: Option<&str>,
    ) -> Result<(), EntDbError> {
        const ETATS: [&str; 5] = ["ouverte", "notes_arretees", "appreciations_closes", "conseil_tenu", "close"];
        sqlx::query("SELECT eva_etat_assurer($1, $2, $3)")
            .bind(etab)
            .bind(periode_eva)
            .bind(classe_id)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        let etat: String = sqlx::query_scalar(
            "SELECT etat FROM eva_periode_etat WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(periode_eva)
        .bind(classe_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        let actuel = ETATS.iter().position(|e| *e == etat).unwrap_or(0);
        let etapes: Vec<usize> = if rang_vise > actuel {
            (actuel + 1..=rang_vise).collect()
        } else if rang_vise < actuel {
            vec![rang_vise]
        } else {
            Vec::new()
        };
        for e in etapes {
            sqlx::query(
                "UPDATE eva_periode_etat SET etat = $4, origine = 'geste', modifie_par = $5, motif = $6
                  WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3",
            )
            .bind(etab)
            .bind(periode_eva)
            .bind(classe_id)
            .bind(ETATS[e])
            .bind(acteur)
            .bind(motif)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        }
        Ok(())
    }


    /// Journal des changements de statut d'une période (direction : `periodes.verrouiller`),
    /// du plus récent au plus ancien.
    pub async fn journal_periode_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        periode_id: Uuid,
    ) -> Result<Vec<ent_types::evaluations::JournalPeriodeDto>, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let rows: Vec<(Uuid, Uuid, String, String, Option<String>, Uuid, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT id, periode_id, statut_avant, statut_apres, motif, acteur_user_id, cree_le
                   FROM ent_note_periode_journal
                  WHERE etablissement_id = $1 AND periode_id = $2
                  ORDER BY cree_le DESC, id DESC",
            )
            .bind(etab)
            .bind(periode_id)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, periode_id, a, b, motif, acteur_user_id, cree_le)| {
                ent_types::evaluations::JournalPeriodeDto {
                    id,
                    periode_id,
                    statut_avant: a,
                    statut_apres: b,
                    motif,
                    acteur_user_id,
                    cree_le,
                }
            })
            .collect())
    }

    pub(crate) fn exiger_staff(role: &str) -> Result<(), EntDbError> {
        if Role::parse(role).is_some_and(|r| r.is_staff()) {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Règles de calcul : l'unique source est le réglage de l'année courante (`eva_reglage`, moteur V2). Le
    /// « arrondi » historique est la précision d'affichage ; un absent ne compte jamais zéro (R-V5).
    pub async fn regles_calcul_for_actor(
        &self,
        etab: Uuid,
        role: &str,
    ) -> Result<ReglesCalculDto, EntDbError> {
        Self::exiger_staff(role)?;
        let annee = self.eva_annee_courante(etab).await?;
        let r = self.eva_v1_regles(etab, annee).await?;
        Ok(ReglesCalculDto {
            arrondi: r.regles.affichage.precision.as_str().to_string(),
            absent_compte_zero: false,
        })
    }

    pub async fn definir_regles_calcul_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &ReglesCalculDto,
    ) -> Result<ReglesCalculDto, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let arrondi = Arrondi::parse(&body.arrondi).ok_or(refus("arrondi inconnu"))?;
        if body.absent_compte_zero {
            return Err(refus(
                "une absence à une évaluation n'est jamais convertie en zéro : statut « absent » \
                 ou « à rattraper », puis évaluation de remplacement",
            ));
        }
        let precision = Precision::parse(arrondi.as_str()).ok_or(refus("arrondi inconnu"))?;
        let annee = self.eva_annee_courante(etab).await?;
        let actuelles: ReglesEtablissement = self.eva_v1_regles(etab, annee).await?;
        let mut regles = actuelles.regles;
        regles.affichage.precision = precision;
        self.eva_v1_definir_regles_for_actor(etab, acteur, role, annee, &regles, actuelles.pas_saisie_m)
            .await?;
        self.regles_calcul_for_actor(etab, role).await
    }

    /// Coefficient d'une matière dans la moyenne générale d'une classe (`matieres.gerer`). **V3b** : il s'écrit dans
    /// `eva_coefficient_matiere` (la seule source des moyennes), DATÉ du jour de l'établissement : l'ancienne ligne
    /// est close la veille, jamais réécrite (R-V23) ; un coefficient qui toucherait des résultats déjà saisis est
    /// refusé par la base (jamais rétroactif).
    pub async fn definir_coef_matiere_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &CoefMatiereBody,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::MatieresGerer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let Some(coef_c) = domain_eleve::evaluation::centiemes(body.coef).filter(|c| *c > 0 && *c <= 10_000) else {
            return Err(refus("coefficient invalide (0 < coefficient ≤ 100)"));
        };
        let coef_m = coef_c * 10;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let annee: Option<Uuid> = sqlx::query_scalar(
            "SELECT c.annee_id FROM ent_classe c JOIN ent_matiere m ON m.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = $2 AND m.id = $3",
        )
        .bind(etab)
        .bind(body.classe_id)
        .bind(body.matiere_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let annee = annee.ok_or_else(introuvable)?;
        let courant: Option<(Uuid, i64, NaiveDate)> = sqlx::query_as(
            "SELECT id, (coef * 1000)::bigint, valable_du FROM eva_coefficient_matiere
              WHERE etablissement_id = $1 AND annee_id = $2 AND matiere_id = $3 AND classe_id = $4
                AND valable_du <= $5 AND (valable_au IS NULL OR valable_au >= $5)
              FOR UPDATE",
        )
        .bind(etab)
        .bind(annee)
        .bind(body.matiere_id)
        .bind(body.classe_id)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((id, actuel_m, depuis)) = courant {
            if actuel_m == coef_m {
                return Ok(());
            }
            if depuis >= jour {
                return Err(conflit(
                    "un coefficient posé aujourd'hui ne se remplace que demain (une ligne datée ne se réécrit pas)",
                ));
            }
            sqlx::query("UPDATE eva_coefficient_matiere SET valable_au = $3 WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .bind(jour - chrono::Duration::days(1))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        sqlx::query(
            "INSERT INTO eva_coefficient_matiere (id, etablissement_id, annee_id, matiere_id, classe_id, coef, valable_du, cree_par)
             VALUES ($1, $2, $3, $4, $5, $6::bigint / 1000.0, $7, $8)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(annee)
        .bind(body.matiere_id)
        .bind(body.classe_id)
        .bind(coef_m)
        .bind(jour)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(())
    }

    // ───────────────────────────── Calcul de classe ─────────────────────────────

    /// Moteur commun : moyennes de tous les élèves d'une classe sur une période, par le moteur UNIQUE (V2,
    /// `moyennes_tx`) : la valeur lue ici est celle de la grille, du bulletin et du conseil, au millième près.
    /// `publiees_seulement` : ne compter que les évaluations visibles des familles.
    pub(crate) async fn eva_calculer_classe(
        &self,
        etab: Uuid,
        classe_id: Uuid,
        periode_id: Uuid,
        publiees_seulement: bool,
    ) -> Result<CalculClasse, EntDbError> {
        let periode = self.eva_periode(etab, periode_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let row: Option<(String,)> =
            sqlx::query_as("SELECT code FROM ent_classe WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(classe_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let (classe_code,) = row.ok_or_else(introuvable)?;
        let eleves = self
            .eva_effectif(etab, Some(classe_id), None, periode.date_debut, periode.date_fin)
            .await?;
        let periode_eva = self.eva_periode_pour_classe(etab, periode_id, classe_id).await?;
        let mut regles = ReglesEtablissement::default().regles;
        if let Some(p) = periode_eva {
            let annee: Option<Uuid> = sqlx::query_scalar(
                "SELECT annee_id FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(p)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some(a) = annee {
                regles = regles_tx(&mut tx, etab, a).await?.regles;
            }
        }
        let mut moyennes: HashMap<(Uuid, Uuid), Aff> = HashMap::new();
        let mut generales: HashMap<Uuid, Aff> = HashMap::new();
        let mut par_matiere: HashMap<Uuid, Vec<Rat>> = HashMap::new();
        let mut exactes_generale: Vec<Rat> = Vec::new();
        let mut coefs: HashMap<Uuid, f64> = HashMap::new();
        // CE-03 : un élève parti avant la fin de la période garde sa moyenne (bulletin de départ sur demande) mais SORT
        // des statistiques de la classe.
        let partis: HashSet<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_eleve
              WHERE etablissement_id = $1 AND id = ANY($2) AND date_sortie IS NOT NULL AND date_sortie < $3",
        )
        .bind(etab)
        .bind(eleves.iter().map(|(e, _, _)| *e).collect::<Vec<_>>())
        .bind(periode.date_fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        if let Some(p) = periode_eva {
            for (eleve, _, _) in &eleves {
                let r = moyennes_tx(&mut tx, etab, *eleve, p, publiees_seulement).await?;
                for m in &r.matieres {
                    if m.matiere_id == Uuid::nil() {
                        continue;
                    }
                    coefs.entry(m.matiere_id).or_insert(m.coef_m as f64 / 1000.0);
                    if let (Some(x), Some(a)) = (m.moyenne.exacte, m.moyenne.affichee_m) {
                        moyennes.insert((*eleve, m.matiere_id), Aff(a));
                        if !partis.contains(eleve) {
                            par_matiere.entry(m.matiere_id).or_default().push(x);
                        }
                    }
                }
                if let (Some(x), Some(a)) = (r.generale.exacte, r.generale.affichee_m) {
                    generales.insert(*eleve, Aff(a));
                    if !partis.contains(eleve) {
                        exactes_generale.push(x);
                    }
                }
            }
        }
        // Matières : celles des services de la classe (même sans note) et celles qui ont une moyenne.
        let mut ids_matieres: HashSet<Uuid> = coefs.keys().copied().collect();
        let services: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT s.matiere_id FROM ent_service_enseignement s
               LEFT JOIN ent_groupe g ON g.id = s.groupe_id AND g.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1 AND s.matiere_id IS NOT NULL
                AND (s.classe_id = $2 OR g.classe_id = $2)",
        )
        .bind(etab)
        .bind(classe_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        ids_matieres.extend(services.into_iter().map(|(m,)| m));
        let ids_matieres: Vec<Uuid> = ids_matieres.into_iter().collect();
        let matieres_rows: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT m.id, m.libelle, m.code FROM ent_matiere m
              WHERE m.etablissement_id = $1 AND m.id = ANY($2) ORDER BY m.libelle, m.code",
        )
        .bind(etab)
        .bind(&ids_matieres)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut matieres: Vec<(Uuid, String, String, f64)> = Vec::new();
        for (id, lib, code) in matieres_rows {
            let coef = match coefs.get(&id) {
                Some(c) => *c,
                None => match periode_eva {
                    Some(p) => {
                        let annee: Option<Uuid> = sqlx::query_scalar(
                            "SELECT annee_id FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
                        )
                        .bind(etab)
                        .bind(p)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(db)?;
                        match annee {
                            Some(a) => coefficient_matiere_tx(&mut tx, etab, a, id, Some(classe_id), periode.date_fin)
                                .await?
                                .map_or(1.0, |(m, _)| m as f64 / 1000.0),
                            None => 1.0,
                        }
                    }
                    None => 1.0,
                },
            };
            matieres.push((id, lib, code, coef));
        }
        let affichage = regles.affichage;
        let stats = |valeurs: &[Rat]| -> Result<Option<StatsAff>, EntDbError> {
            let Some(s) = statistiques(valeurs, &regles).map_err(|_| refus("statistiques de classe indisponibles"))? else {
                return Ok(None);
            };
            let conv = |x: Rat| -> Result<Aff, EntDbError> {
                affichage
                    .en_milliemes(x)
                    .map(Aff)
                    .map_err(|_| refus("statistiques de classe indisponibles"))
            };
            Ok(Some(StatsAff {
                moyenne: conv(s.moyenne)?,
                min: conv(s.min)?,
                max: conv(s.max)?,
                effectif: s.effectif,
                brut: s,
            }))
        };
        let mut stats_matiere = HashMap::new();
        for (m, v) in &par_matiere {
            if let Some(s) = stats(v)? {
                stats_matiere.insert(*m, s);
            }
        }
        let stats_generale = stats(&exactes_generale)?;
        Ok(CalculClasse {
            classe_id,
            classe_code,
            arrondi: affichage.precision.as_str().to_string(),
            periode,
            regles,
            eleves,
            matieres,
            moyennes,
            generales,
            stats_matiere,
            stats_generale,
        })
    }

    /// Vue de classe : moyennes par matière et générales, statistiques (`notes.lire`).
    pub async fn vue_classe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        classe_id: Uuid,
        periode_id: Uuid,
    ) -> Result<VueClasseDto, EntDbError> {
        Self::exiger_staff(role)?;
        let calc = self.eva_calculer_classe(etab, classe_id, periode_id, false).await?;
        let autorises = self
            .eleves_autorises(etab, acteur, role, Action::NotesLire)
            .await?;
        let lignes: Vec<LigneClasseDto> = calc
            .eleves
            .iter()
            .filter(|(id, _, _)| autorises.as_ref().is_none_or(|s| s.contains(id)))
            .map(|(id, nom, prenom)| LigneClasseDto {
                eleve_id: *id,
                nom: nom.clone(),
                prenom: prenom.clone(),
                moyennes: calc
                    .matieres
                    .iter()
                    .map(|(m, _, _, _)| calc.moyennes.get(&(*id, *m)).map(|x| x.valeur()))
                    .collect(),
                moyenne_generale: calc.generales.get(id).map(|x| x.valeur()),
            })
            .collect();
        if lignes.is_empty() && !calc.eleves.is_empty() {
            return Err(EntDbError::Forbidden);
        }
        if autorises.is_some() && calc.eleves.is_empty() {
            // Classe vide : seul qui a la lecture sur l'établissement la consulte.
            return Err(EntDbError::Forbidden);
        }
        Ok(VueClasseDto {
            classe_id,
            classe_code: calc.classe_code.clone(),
            periode: calc.periode.dto(),
            arrondi: calc.arrondi.clone(),
            matieres: calc
                .matieres
                .iter()
                .map(|(m, lib, _, coef)| MatiereColonneDto {
                    matiere_id: *m,
                    libelle: lib.clone(),
                    coef: *coef,
                    // D-022 : agrégats de la classe entière non communiqués à qui ne la voit pas.
                    stats: stats_dto(calc.stats_matiere.get(m).copied().filter(|_| autorises.is_none())),
                })
                .collect(),
            lignes,
            stats_generale: stats_dto(calc.stats_generale.filter(|_| autorises.is_none())),
        })
    }

    /// EVA-13 : relevé d'un élève. Famille et élève : évaluations **publiées** seulement ; les moyennes viennent
    /// du moteur unique. Les statistiques de classe ne sont montrées à une famille que si l'établissement les
    /// autorise et que l'effectif atteint son seuil (R-V12).
    pub async fn releve_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        periode_id: Uuid,
    ) -> Result<ReleveEleveDto, EntDbError> {
        if !self.actor_can_read_notes(etab, acteur, role, eleve_id).await? {
            return Err(EntDbError::Forbidden);
        }
        // V3c : garde du socle 2 sur TOUTE lecture par élève (élève protégé « introuvable » pour un tiers du personnel).
        self.exiger_eleve_visible(etab, acteur, role, eleve_id).await?;
        let staff = Role::parse(role).is_some_and(|r| r.is_staff());
        let periode = self.eva_periode(etab, periode_id).await?;
        let Some(classe_id) = self
            .eva_classe_eleve(etab, eleve_id, periode.date_debut, periode.date_fin)
            .await?
        else {
            let annee = self.eva_annee_courante(etab).await?;
            let regles = self.eva_v1_regles(etab, annee).await?;
            return Ok(ReleveEleveDto {
                eleve_id,
                periode: periode.dto(),
                arrondi: regles.regles.affichage.precision.as_str().to_string(),
                matieres: Vec::new(),
                moyenne_generale: None,
            });
        };
        let calc = self.eva_calculer_classe(etab, classe_id, periode_id, !staff).await?;
        let lignes = match self.eva_periode_pour_classe(etab, periode_id, classe_id).await? {
            Some(p) => {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                let r = sqlx::query(
                    "SELECT q.titre, q.date_eval, q.coef_m, q.bareme_m, q.matiere_id, q.statut, q.valeur_m FROM (
                       SELECT DISTINCT ON (r.evaluation_id)
                            ev.titre, ev.date_eval, ev.id AS evaluation_id, (ev.coef * 1000)::bigint AS coef_m,
                            (ev.bareme * 1000)::bigint AS bareme_m, co.matiere_id, r.statut, (r.valeur * 1000)::bigint AS valeur_m
                       FROM eva_resultat r
                       JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
                       JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
                      WHERE r.etablissement_id = $1 AND r.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
                        AND ev.mode = 'note' AND ev.etat = 'ouverte'
                        AND ($4 = false OR ev.publiee_le IS NOT NULL)
                        AND eva_periode_dans(ev.etablissement_id, $3, ev.date_eval)
                      -- Comme le moteur (`moyennes_tx`) : la note suit l'élève conservé ; une évaluation ne ressort qu'une fois
                      -- (priorité à la fiche conservée, puis saisie la plus récente).
                      ORDER BY r.evaluation_id, (r.eleve_id = $2) DESC, r.saisi_le DESC, r.id
                      ) q
                      ORDER BY q.date_eval, q.evaluation_id",
                )
                .bind(etab)
                .bind(eleve_id)
                .bind(p)
                .bind(!staff)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                tx.commit().await.map_err(db)?;
                r
            }
            None => Vec::new(),
        };
        let matieres = calc
            .matieres
            .iter()
            .map(|(m, lib, _, _)| {
                let evaluations = lignes
                    .iter()
                    .filter(|l| l.try_get::<Option<Uuid>, _>("matiere_id").ok().flatten() == Some(*m))
                    .filter_map(|l| {
                        let statut: String = l.try_get("statut").ok()?;
                        let valeur_m: Option<i64> = l.try_get("valeur_m").ok()?;
                        Some(ReleveEvaluationDto {
                            titre: l.try_get("titre").ok()?,
                            date_devoir: l.try_get("date_eval").ok()?,
                            coef: l.try_get::<i64, _>("coef_m").ok()? as f64 / 1000.0,
                            sur: l.try_get::<i64, _>("bareme_m").ok()? as f64 / 1000.0,
                            valeur: (statut == "note").then(|| valeur_m.map(|v| v as f64 / 1000.0)).flatten(),
                            statut,
                        })
                    })
                    .collect();
                // V3e : hors personnel, la décision appartient au moteur (`pour_famille` : réglage, seuil et plancher d'effectif).
                let stats = calc
                    .stats_matiere
                    .get(m)
                    .filter(|s| staff || s.brut.pour_famille(&calc.regles).is_some());
                ReleveMatiereDto {
                    matiere_id: *m,
                    matiere_libelle: lib.clone(),
                    moyenne: calc.moyennes.get(&(eleve_id, *m)).map(|x| x.valeur()),
                    moyenne_classe: stats.map(|s| s.moyenne.valeur()),
                    evaluations,
                }
            })
            .collect();
        Ok(ReleveEleveDto {
            eleve_id,
            periode: calc.periode.dto(),
            arrondi: calc.arrondi.clone(),
            matieres,
            moyenne_generale: calc.generales.get(&eleve_id).map(|x| x.valeur()),
        })
    }
}

