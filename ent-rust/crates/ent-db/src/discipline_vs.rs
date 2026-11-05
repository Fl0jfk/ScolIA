//! Discipline de la vie scolaire (socle 2, lot S2-4) : punitions scolaires
//! **distinctes** des sanctions, procédure disciplinaire, effacement planifié des
//! sanctions, signalement d'assiduité.
//!
//! Toutes les règles réglementaires (catégories d'effacement, délai de
//! convocation du conseil, seuil de signalement) sont **paramétrables** et à
//! **vérifier sur les textes officiels avant pilote** (D-019) : les valeurs par
//! défaut ci-dessous sont des hypothèses de travail, pas des vérités juridiques.

use crate::outbox::{Causalite, Publier};
use crate::service::{EntDbError, PgEnt};
use crate::vie_scolaire::{db, introuvable, invalide, types_vs};
use chrono::{Datelike, Months, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

const MAX_TEXTE: usize = 2000;

// ---------------------------------------------------------------------------
// Catégories de sanctions et règles d'effacement (pur)
// ---------------------------------------------------------------------------

/// Échelle des sanctions disciplinaires [H] (à vérifier : R511-13 du Code de
/// l'éducation). `autre` : sanction propre à l'établissement, sans effacement
/// automatique tant qu'une règle n'est pas paramétrée.
pub const CATEGORIES_SANCTION: [&str; 7] = [
    "avertissement",
    "blame",
    "mesure_responsabilisation",
    "exclusion_temporaire_classe",
    "exclusion_temporaire_etablissement",
    "exclusion_definitive",
    "autre",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", content = "delai_mois", rename_all = "snake_case")]
pub enum ModeEffacement {
    /// Effacée à l'issue de l'année scolaire (1ᵉʳ septembre suivant le prononcé).
    FinAnneeScolaire,
    /// Effacée N mois après le prononcé.
    DelaiMois(u32),
    /// Jamais effacée automatiquement.
    Jamais,
}

/// Règle par défaut d'une catégorie [H : hypothèse de travail à vérifier].
pub fn regle_effacement_defaut(categorie: &str) -> ModeEffacement {
    match categorie {
        "avertissement" | "blame" | "mesure_responsabilisation" => ModeEffacement::FinAnneeScolaire,
        "exclusion_temporaire_classe" | "exclusion_temporaire_etablissement" => {
            ModeEffacement::DelaiMois(12)
        }
        _ => ModeEffacement::Jamais,
    }
}

/// Premier jour à partir duquel une sanction prononcée le `jour` est à effacer ;
/// `None` si elle ne l'est jamais.
pub fn date_effacement(jour: NaiveDate, mode: ModeEffacement) -> Option<NaiveDate> {
    match mode {
        ModeEffacement::Jamais => None,
        ModeEffacement::DelaiMois(n) => jour.checked_add_months(Months::new(n)),
        ModeEffacement::FinAnneeScolaire => {
            // Année scolaire : 1ᵉʳ septembre → 31 août ; effacement le 1ᵉʳ septembre suivant.
            let annee = if jour.month() >= 9 { jour.year() + 1 } else { jour.year() };
            NaiveDate::from_ymd_opt(annee, 9, 1)
        }
    }
}

// ---------------------------------------------------------------------------
// DTO
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PunitionType {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actif: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Punition {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub type_id: Uuid,
    pub jour: NaiveDate,
    pub motif: String,
    pub echeance: Option<NaiveDate>,
    pub duree_minutes: Option<i32>,
    pub incident_id: Option<Uuid>,
    pub statut: String,
    pub donnee_par_user_id: Uuid,
}

const COLONNES_PUNITION: &str = "id, etablissement_id, eleve_id, type_id, jour, motif, echeance, \
    duree_minutes, incident_id, statut, donnee_par_user_id";

#[derive(Debug, Clone)]
pub struct DonnerPunition {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub type_id: Uuid,
    pub jour: NaiveDate,
    pub motif: String,
    pub echeance: Option<NaiveDate>,
    pub duree_minutes: Option<i32>,
    pub incident_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProcedureDisciplinaire {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub incident_id: Option<Uuid>,
    pub instance: String,
    pub statut: String,
    pub faits: String,
    pub engagee_le: NaiveDate,
    pub engagee_par_user_id: Uuid,
    pub observations_recueillies_le: Option<NaiveDate>,
    pub convocation_envoyee_le: Option<NaiveDate>,
    pub conseil_le: Option<NaiveDate>,
    pub delai_convocation_jours: i32,
    pub decision: Option<String>,
    pub decision_le: Option<NaiveDate>,
    pub sanction_id: Option<Uuid>,
}

const COLONNES_PROCEDURE: &str = "id, etablissement_id, eleve_id, incident_id, instance, statut, \
    faits, engagee_le, engagee_par_user_id, observations_recueillies_le, convocation_envoyee_le, \
    conseil_le, delai_convocation_jours, decision, decision_le, sanction_id";

#[derive(Debug, Clone)]
pub struct EngagerProcedure {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub incident_id: Option<Uuid>,
    /// `chef_etablissement` ou `conseil_discipline`.
    pub instance: String,
    pub faits: String,
    /// Délai minimal de convocation du conseil, en jours (défaut 8 [H]).
    pub delai_convocation_jours: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffacementRapport {
    pub examinees: usize,
    pub effacees: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SignalementAssiduite {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub mois: NaiveDate,
    pub demi_journees: i32,
    pub seuil: i32,
    pub statut: String,
    pub dossier: serde_json::Value,
}

fn aujourdhui() -> NaiveDate {
    PgEnt::maintenant_local().date()
}

fn texte(v: &str, max: usize) -> Result<String, EntDbError> {
    let v = v.trim();
    if v.is_empty() || v.chars().count() > max || v.chars().any(|c| c.is_control() && c != '\n') {
        return Err(invalide());
    }
    Ok(v.to_string())
}

impl PgEnt {
    // -----------------------------------------------------------------------
    // Punitions scolaires (≠ sanctions)
    // -----------------------------------------------------------------------

    /// Amorce les types de punitions usuels [H] (circulaire de 2011 sur les
    /// règles de vie scolaire : excuse, devoir supplémentaire, retenue, exclusion
    /// ponctuelle de cours). Idempotent. Action `discipline.gerer`.
    pub async fn amorcer_types_punition(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<PunitionType>, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::DisciplineGerer)
            .await?;
        for (code, libelle) in [
            ("excuse", "Excuse orale ou écrite"),
            ("devoir_supplementaire", "Devoir supplémentaire"),
            ("retenue", "Retenue"),
            ("exclusion_ponctuelle_cours", "Exclusion ponctuelle de cours"),
        ] {
            sqlx::query(
                "INSERT INTO ent_punition_type (id, etablissement_id, code, libelle)
                 VALUES ($1,$2,$3,$4) ON CONFLICT (etablissement_id, code) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etablissement_id)
            .bind(code)
            .bind(libelle)
            .execute(self.pool())
            .await
            .map_err(db)?;
        }
        self.types_punition(etablissement_id).await
    }

    pub async fn types_punition(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<PunitionType>, EntDbError> {
        sqlx::query_as::<_, PunitionType>(
            "SELECT id, code, libelle, actif FROM ent_punition_type
              WHERE etablissement_id = $1 ORDER BY code",
        )
        .bind(etablissement_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// VS-15 : donne une punition scolaire. Action `punitions.donner` (professeur :
    /// ses élèves ; vie scolaire : tout l'établissement). Aucune procédure : c'est
    /// ce qui la distingue d'une sanction.
    pub async fn donner_punition(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: DonnerPunition,
    ) -> Result<Punition, EntDbError> {
        let etab = cmd.etablissement_id;
        self.exiger_action_eleve(etab, acteur, role, Action::PunitionsDonner, cmd.eleve_id)
            .await?;
        let motif = texte(&cmd.motif, MAX_TEXTE)?;
        if cmd.duree_minutes.is_some_and(|d| d <= 0 || d > 600) {
            return Err(invalide());
        }
        if cmd.echeance.is_some_and(|e| e < cmd.jour) {
            return Err(invalide());
        }
        let type_actif: Option<(bool,)> = sqlx::query_as(
            "SELECT actif FROM ent_punition_type WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(cmd.type_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        if type_actif != Some((true,)) {
            return Err(EntDbError::Domain(DomainError::ReferentielVsInconnu));
        }
        if let Some(inc) = cmd.incident_id {
            let ok: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_incident WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(inc)
            .bind(cmd.eleve_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
            if ok.is_none() {
                return Err(invalide());
            }
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_punition
                (id, etablissement_id, eleve_id, type_id, jour, motif, echeance, duree_minutes,
                 incident_id, donnee_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(cmd.eleve_id)
        .bind(cmd.type_id)
        .bind(cmd.jour)
        .bind(&motif)
        .bind(cmd.echeance)
        .bind(cmd.duree_minutes)
        .bind(cmd.incident_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_vs::PUNITION_DONNEE.to_string(),
            // Agrégat réutilisé (pas de variante dédiée tant que l'enum est partagé).
            aggregate: AggregateKind::Sanction,
            aggregate_id: id,
            eleve_id: Some(cmd.eleve_id),
            payload: json!({
                "type_id": cmd.type_id,
                "jour": cmd.jour.to_string(),
                "echeance": cmd.echeance.map(|d| d.to_string()),
            }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        let p = sqlx::query_as::<_, Punition>(&format!(
            "SELECT {COLONNES_PUNITION} FROM ent_punition WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }

    /// Marque une punition « effectuée » ou « annulée ». Son auteur, ou le personnel
    /// ayant `punitions.donner` sur tout l'établissement.
    pub async fn clore_punition(
        &self,
        etablissement_id: Uuid,
        punition_id: Uuid,
        acteur: Uuid,
        role: &str,
        statut: &str,
    ) -> Result<Punition, EntDbError> {
        if !matches!(statut, "effectuee" | "annulee") {
            return Err(invalide());
        }
        let p = sqlx::query_as::<_, Punition>(&format!(
            "SELECT {COLONNES_PUNITION} FROM ent_punition WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etablissement_id)
        .bind(punition_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if p.donnee_par_user_id != acteur {
            self.exiger_action_etab(etablissement_id, acteur, role, Action::PunitionsDonner)
                .await?;
        } else {
            self.exiger_action_eleve(
                etablissement_id,
                acteur,
                role,
                Action::PunitionsDonner,
                p.eleve_id,
            )
            .await?;
        }
        if p.statut != "donnee" {
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        sqlx::query(
            "UPDATE ent_punition SET statut = $3,
                    effectuee_at = CASE WHEN $3 = 'effectuee' THEN now() ELSE NULL END,
                    annulee_par_user_id = CASE WHEN $3 = 'annulee' THEN $4 ELSE NULL END,
                    annulee_at = CASE WHEN $3 = 'annulee' THEN now() ELSE NULL END
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'donnee'",
        )
        .bind(etablissement_id)
        .bind(punition_id)
        .bind(statut)
        .bind(acteur)
        .execute(self.pool())
        .await
        .map_err(db)?;
        Ok(Punition {
            statut: statut.to_string(),
            ..p
        })
    }

    /// Punitions d'un élève. Personnel : `punitions.donner` ou `absences.lire` sur
    /// l'élève ; responsable lié avec `peut_voir_vs` ; l'élève lui-même.
    pub async fn punitions_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<Punition>, EntDbError> {
        self.exiger_eleve_existe(etablissement_id, eleve_id).await?;
        let autorise = match Role::parse(role) {
            Some(r) if r.is_staff() => {
                self.autoriser_eleve(etablissement_id, acteur, role, Action::PunitionsDonner, eleve_id)
                    .await?
                    || self
                        .autoriser_eleve(etablissement_id, acteur, role, Action::AbsencesLire, eleve_id)
                        .await?
            }
            _ => {
                self.actor_can_read_absences(etablissement_id, acteur, role, eleve_id)
                    .await?
            }
        };
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        sqlx::query_as::<_, Punition>(&format!(
            "SELECT {COLONNES_PUNITION} FROM ent_punition
              WHERE etablissement_id = $1 AND eleve_id = $2 AND statut <> 'annulee'
              ORDER BY jour DESC, created_at DESC LIMIT 200"
        ))
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    // -----------------------------------------------------------------------
    // Sanctions : catégories et effacement planifié (VS-18)
    // -----------------------------------------------------------------------

    /// Range un type de sanction dans une catégorie de l'échelle [H]. Action
    /// `discipline.gerer`.
    pub async fn categoriser_type_sanction(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        type_id: Uuid,
        categorie: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::DisciplineGerer)
            .await?;
        if !CATEGORIES_SANCTION.contains(&categorie) {
            return Err(invalide());
        }
        let r = sqlx::query(
            "UPDATE ent_sanction_type SET categorie = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(type_id)
        .bind(categorie)
        .execute(self.pool())
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        Ok(())
    }

    /// Règle d'effacement d'une catégorie (paramétrée, sinon défaut [H]).
    pub async fn regle_effacement(
        &self,
        etablissement_id: Uuid,
        categorie: &str,
    ) -> Result<ModeEffacement, EntDbError> {
        let row: Option<(String, Option<i32>)> = sqlx::query_as(
            "SELECT mode, delai_mois FROM ent_regle_effacement
              WHERE etablissement_id = $1 AND categorie = $2",
        )
        .bind(etablissement_id)
        .bind(categorie)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(match row {
            Some((m, _)) if m == "fin_annee_scolaire" => ModeEffacement::FinAnneeScolaire,
            Some((m, Some(n))) if m == "delai_mois" && n > 0 => ModeEffacement::DelaiMois(n as u32),
            Some((m, _)) if m == "jamais" => ModeEffacement::Jamais,
            _ => regle_effacement_defaut(categorie),
        })
    }

    /// Paramètre la règle d'effacement d'une catégorie. Action `discipline.gerer`.
    pub async fn definir_regle_effacement(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        categorie: &str,
        mode: ModeEffacement,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::DisciplineGerer)
            .await?;
        if !CATEGORIES_SANCTION.contains(&categorie) {
            return Err(invalide());
        }
        let (m, delai) = match mode {
            ModeEffacement::FinAnneeScolaire => ("fin_annee_scolaire", None),
            ModeEffacement::DelaiMois(n) if (1..=120).contains(&n) => ("delai_mois", Some(n as i32)),
            ModeEffacement::DelaiMois(_) => return Err(invalide()),
            ModeEffacement::Jamais => ("jamais", None),
        };
        sqlx::query(
            "INSERT INTO ent_regle_effacement (etablissement_id, categorie, mode, delai_mois, updated_at)
             VALUES ($1,$2,$3,$4, now())
             ON CONFLICT (etablissement_id, categorie) DO UPDATE
                SET mode = EXCLUDED.mode, delai_mois = EXCLUDED.delai_mois, updated_at = now()",
        )
        .bind(etablissement_id)
        .bind(categorie)
        .bind(m)
        .bind(delai)
        .execute(self.pool())
        .await
        .map_err(db)?;
        Ok(())
    }

    /// Efface du dossier administratif les sanctions dont le délai est échu à la
    /// date `au_jour`. Idempotent. L'effacement vide le texte libre (`note`) et
    /// date l'opération ; la trace d'existence (type, jour) reste pour l'audit.
    /// Appelable par une tâche planifiée (système) via [`Self::effacer_sanctions_echues`].
    pub async fn effacer_sanctions_echues_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        au_jour: NaiveDate,
    ) -> Result<EffacementRapport, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::DisciplineGerer)
            .await?;
        // L'effacement est irréversible (texte libre vidé) : jamais à une date future.
        if au_jour > aujourdhui() {
            return Err(invalide());
        }
        self.effacer_sanctions_echues(etablissement_id, au_jour, Some(acteur))
            .await
    }

    pub async fn effacer_sanctions_echues(
        &self,
        etablissement_id: Uuid,
        au_jour: NaiveDate,
        acteur: Option<Uuid>,
    ) -> Result<EffacementRapport, EntDbError> {
        let candidates: Vec<(Uuid, Uuid, NaiveDate, String)> = sqlx::query_as(
            "SELECT s.id, s.eleve_id, s.jour, t.categorie
               FROM ent_sanction s
               JOIN ent_sanction_type t
                 ON t.id = s.type_id AND t.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1 AND s.effacee_at IS NULL
                AND t.categorie IS NOT NULL
              ORDER BY s.jour, s.id",
        )
        .bind(etablissement_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut rapport = EffacementRapport {
            examinees: candidates.len(),
            effacees: Vec::new(),
        };
        for (id, eleve_id, jour, categorie) in candidates {
            let mode = self.regle_effacement(etablissement_id, &categorie).await?;
            let Some(due) = date_effacement(jour, mode) else {
                continue;
            };
            if au_jour < due {
                continue;
            }
            let mut tx = self.pool().begin().await.map_err(db)?;
            let r = sqlx::query(
                "UPDATE ent_sanction
                    SET effacee_at = now(), effacee_motif = 'delai_reglementaire', note = NULL
                  WHERE etablissement_id = $1 AND id = $2 AND effacee_at IS NULL",
            )
            .bind(etablissement_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if r.rows_affected() == 1 {
                tx.publier(MetierEventDraft {
                    etablissement_id,
                    type_name: types_vs::SANCTION_EFFACEE.to_string(),
                    aggregate: AggregateKind::Sanction,
                    aggregate_id: id,
                    eleve_id: Some(eleve_id),
                    payload: json!({ "categorie": categorie, "echeance": due.to_string() }),
                    actor_user_id: acteur.map(|a| a.to_string()),
                })
                .await?;
                rapport.effacees.push(id);
            }
            tx.commit().await.map_err(db)?;
        }
        Ok(rapport)
    }

    // -----------------------------------------------------------------------
    // Procédure disciplinaire (VS-17)
    // -----------------------------------------------------------------------

    /// Engage une procédure disciplinaire. Action `discipline.gerer` (direction).
    pub async fn engager_procedure(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: EngagerProcedure,
    ) -> Result<ProcedureDisciplinaire, EntDbError> {
        let etab = cmd.etablissement_id;
        self.exiger_action_eleve(etab, acteur, role, Action::DisciplineGerer, cmd.eleve_id)
            .await?;
        if !matches!(cmd.instance.as_str(), "chef_etablissement" | "conseil_discipline") {
            return Err(invalide());
        }
        let faits = texte(&cmd.faits, MAX_TEXTE)?;
        let delai = cmd.delai_convocation_jours.unwrap_or(8);
        if !(0..=60).contains(&delai) {
            return Err(invalide());
        }
        if let Some(inc) = cmd.incident_id {
            let ok: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_incident WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(inc)
            .bind(cmd.eleve_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
            if ok.is_none() {
                return Err(invalide());
            }
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_procedure_disciplinaire
                (id, etablissement_id, eleve_id, incident_id, instance, faits, engagee_le,
                 engagee_par_user_id, delai_convocation_jours)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(cmd.eleve_id)
        .bind(cmd.incident_id)
        .bind(&cmd.instance)
        .bind(&faits)
        .bind(Utc::now().date_naive())
        .bind(acteur)
        .bind(delai)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_vs::PROCEDURE_ENGAGEE.to_string(),
            aggregate: AggregateKind::Sanction,
            aggregate_id: id,
            eleve_id: Some(cmd.eleve_id),
            payload: json!({ "instance": cmd.instance }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        let p = self.lire_procedure_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }

    async fn lire_procedure_tx(
        &self,
        tx: &mut sqlx::PgConnection,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<ProcedureDisciplinaire, EntDbError> {
        sqlx::query_as::<_, ProcedureDisciplinaire>(&format!(
            "SELECT {COLONNES_PROCEDURE} FROM ent_procedure_disciplinaire
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    /// Droits de la défense : consigne que l'élève (et ses responsables s'il est
    /// mineur) ont été entendus ou mis en mesure de présenter leurs observations.
    pub async fn recueillir_observations(
        &self,
        etablissement_id: Uuid,
        procedure_id: Uuid,
        acteur: Uuid,
        role: &str,
        le: NaiveDate,
    ) -> Result<ProcedureDisciplinaire, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::DisciplineGerer)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = self
            .lire_procedure_tx(&mut tx, etablissement_id, procedure_id)
            .await?;
        if matches!(p.statut.as_str(), "decidee" | "close") || le < p.engagee_le {
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        if le > aujourdhui() {
            return Err(invalide());
        }
        sqlx::query(
            "UPDATE ent_procedure_disciplinaire SET observations_recueillies_le = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(procedure_id)
        .bind(le)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let p = self.lire_procedure_tx(&mut tx, etablissement_id, procedure_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }

    /// Convoque le conseil de discipline : la date du conseil doit respecter le
    /// délai minimal après l'envoi de la convocation [H : 8 jours par défaut].
    pub async fn convoquer_conseil(
        &self,
        etablissement_id: Uuid,
        procedure_id: Uuid,
        acteur: Uuid,
        role: &str,
        convocation_envoyee_le: NaiveDate,
        conseil_le: NaiveDate,
    ) -> Result<ProcedureDisciplinaire, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::DisciplineGerer)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = self
            .lire_procedure_tx(&mut tx, etablissement_id, procedure_id)
            .await?;
        if p.instance != "conseil_discipline" || p.statut != "engagee" {
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        let minimum = convocation_envoyee_le + chrono::Duration::days(i64::from(p.delai_convocation_jours));
        if convocation_envoyee_le < p.engagee_le
            || convocation_envoyee_le > aujourdhui()
            || conseil_le < minimum
        {
            return Err(invalide());
        }
        sqlx::query(
            "UPDATE ent_procedure_disciplinaire
                SET statut = 'convoquee', convocation_envoyee_le = $3, conseil_le = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(procedure_id)
        .bind(convocation_envoyee_le)
        .bind(conseil_le)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let p = self.lire_procedure_tx(&mut tx, etablissement_id, procedure_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }

    /// Décide la procédure. Chef d'établissement : observations obligatoires
    /// (contradictoire) ; conseil : convoqué, et décision à partir du jour du
    /// conseil. La sanction prononcée (optionnelle) doit concerner le même élève.
    pub async fn decider_procedure(
        &self,
        etablissement_id: Uuid,
        procedure_id: Uuid,
        acteur: Uuid,
        role: &str,
        decision: &str,
        decision_le: NaiveDate,
        sanction_id: Option<Uuid>,
    ) -> Result<ProcedureDisciplinaire, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::DisciplineGerer)
            .await?;
        let decision = texte(decision, MAX_TEXTE)?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = self
            .lire_procedure_tx(&mut tx, etablissement_id, procedure_id)
            .await?;
        let pret = match p.instance.as_str() {
            "chef_etablissement" => p.statut == "engagee" && p.observations_recueillies_le.is_some(),
            _ => p.statut == "convoquee" && p.conseil_le.is_some_and(|c| decision_le >= c),
        };
        if !pret {
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        // Une décision n'est jamais datée dans le futur ni avant le contradictoire.
        if decision_le > aujourdhui()
            || p.observations_recueillies_le.is_some_and(|o| decision_le < o)
        {
            return Err(invalide());
        }
        if let Some(sid) = sanction_id {
            let ok: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_sanction WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3",
            )
            .bind(etablissement_id)
            .bind(sid)
            .bind(p.eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if ok.is_none() {
                return Err(invalide());
            }
        }
        sqlx::query(
            "UPDATE ent_procedure_disciplinaire
                SET statut = 'decidee', decision = $3, decision_le = $4,
                    decision_par_user_id = $5, sanction_id = $6
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(procedure_id)
        .bind(&decision)
        .bind(decision_le)
        .bind(acteur)
        .bind(sanction_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let type_evt = if p.instance == "conseil_discipline" {
            types_vs::CONSEIL_DISCIPLINE_DECIDE
        } else {
            types_vs::PROCEDURE_DECIDEE
        };
        tx.publier(MetierEventDraft {
            etablissement_id,
            type_name: type_evt.to_string(),
            aggregate: AggregateKind::Sanction,
            aggregate_id: procedure_id,
            eleve_id: Some(p.eleve_id),
            payload: json!({ "instance": p.instance, "sanction_id": sanction_id }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        let p = self.lire_procedure_tx(&mut tx, etablissement_id, procedure_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }

    /// Classe la procédure sans suite.
    pub async fn classer_procedure(
        &self,
        etablissement_id: Uuid,
        procedure_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<ProcedureDisciplinaire, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::DisciplineGerer)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = self
            .lire_procedure_tx(&mut tx, etablissement_id, procedure_id)
            .await?;
        if matches!(p.statut.as_str(), "decidee" | "close") {
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        sqlx::query(
            "UPDATE ent_procedure_disciplinaire SET statut = 'close'
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(procedure_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let p = self.lire_procedure_tx(&mut tx, etablissement_id, procedure_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }

    pub async fn procedures_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<ProcedureDisciplinaire>, EntDbError> {
        self.exiger_action_eleve(etablissement_id, acteur, role, Action::DisciplineGerer, eleve_id)
            .await?;
        sqlx::query_as::<_, ProcedureDisciplinaire>(&format!(
            "SELECT {COLONNES_PROCEDURE} FROM ent_procedure_disciplinaire
              WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY engagee_le DESC"
        ))
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    // -----------------------------------------------------------------------
    // Signalement d'assiduité (VS-21, A5)
    // -----------------------------------------------------------------------

    /// Prépare les signalements d'un mois : élèves dont le décompte de demi-journées
    /// non justifiées atteint le seuil. Le CPE propose (`assiduite.signaler`).
    /// Idempotent ; une proposition déjà décidée n'est pas modifiée.
    pub async fn proposer_signalements(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        mois: NaiveDate,
    ) -> Result<Vec<SignalementAssiduite>, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AssiduiteSignaler)
            .await?;
        let decompte = self.decompte_mensuel(etablissement_id, mois).await?;
        let (debut, suivant) = crate::assiduite::bornes_mois(mois);
        let dernier = suivant - chrono::Duration::days(1);
        let dj = self
            .demi_journees(etablissement_id, None, debut, dernier)
            .await?;
        for e in decompte.eleves.iter().filter(|e| e.seuil_atteint) {
            let detail: Vec<serde_json::Value> = dj
                .iter()
                .filter(|d| d.eleve_id == e.eleve_id && d.non_justifiee)
                .map(|d| {
                    json!({"jour": d.jour.to_string(), "periode": d.periode.as_str(), "minutes": d.minutes})
                })
                .collect();
            let dossier = json!({
                "eleve": {"id": e.eleve_id, "nom": e.nom, "prenom": e.prenom},
                "mois": debut.to_string(),
                "demi_journees_non_justifiees": e.demi_journees_non_justifiees,
                "seuil": decompte.seuil_signalement,
                "demi_journees": detail,
            });
            sqlx::query(
                "INSERT INTO ent_signalement_assiduite
                    (id, etablissement_id, eleve_id, mois, demi_journees, seuil, propose_par_user_id, dossier)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
                 ON CONFLICT (etablissement_id, eleve_id, mois) DO UPDATE
                    SET demi_journees = EXCLUDED.demi_journees, seuil = EXCLUDED.seuil,
                        dossier = EXCLUDED.dossier
                  WHERE ent_signalement_assiduite.statut = 'propose'",
            )
            .bind(Uuid::new_v4())
            .bind(etablissement_id)
            .bind(e.eleve_id)
            .bind(debut)
            .bind(e.demi_journees_non_justifiees as i32)
            .bind(decompte.seuil_signalement)
            .bind(acteur)
            .bind(dossier)
            .execute(self.pool())
            .await
            .map_err(db)?;
        }
        self.signalements_du_mois(etablissement_id, debut).await
    }

    pub(crate) async fn signalements_du_mois(
        &self,
        etablissement_id: Uuid,
        mois: NaiveDate,
    ) -> Result<Vec<SignalementAssiduite>, EntDbError> {
        sqlx::query_as::<_, SignalementAssiduite>(
            "SELECT id, eleve_id, mois, demi_journees, seuil, statut, dossier
               FROM ent_signalement_assiduite
              WHERE etablissement_id = $1 AND mois = $2
              ORDER BY demi_journees DESC, eleve_id",
        )
        .bind(etablissement_id)
        .bind(mois)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Liste (CPE, direction) des signalements d'un mois.
    pub async fn signalements_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        mois: NaiveDate,
    ) -> Result<Vec<SignalementAssiduite>, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AssiduiteSignaler)
            .await?;
        let (debut, _) = crate::assiduite::bornes_mois(mois);
        self.signalements_du_mois(etablissement_id, debut).await
    }

    /// Décision de la direction : `valider`, `refuser` puis `emettre`
    /// (signalement envoyé à l'autorité académique). Action `discipline.gerer` :
    /// le CPE propose, la direction décide.
    pub async fn decider_signalement(
        &self,
        etablissement_id: Uuid,
        signalement_id: Uuid,
        acteur: Uuid,
        role: &str,
        action: &str,
    ) -> Result<SignalementAssiduite, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::DisciplineGerer)
            .await?;
        let (attendu, nouveau) = match action {
            "valider" => ("propose", "valide"),
            "refuser" => ("propose", "refuse"),
            "emettre" => ("valide", "emis"),
            _ => return Err(invalide()),
        };
        let mut tx = self.pool().begin().await.map_err(db)?;
        let s = sqlx::query_as::<_, SignalementAssiduite>(
            "SELECT id, eleve_id, mois, demi_journees, seuil, statut, dossier
               FROM ent_signalement_assiduite
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(signalement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if s.statut != attendu {
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        sqlx::query(
            "UPDATE ent_signalement_assiduite
                SET statut = $3, decide_par_user_id = $4, decide_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(signalement_id)
        .bind(nouveau)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if nouveau == "emis" {
            tx.publier_avec(
                MetierEventDraft {
                    etablissement_id,
                    type_name: types_vs::SIGNALEMENT_ASSIDUITE_EMIS.to_string(),
                    aggregate: AggregateKind::Absence,
                    aggregate_id: signalement_id,
                    eleve_id: Some(s.eleve_id),
                    payload: json!({
                        "mois": s.mois.to_string(),
                        "demi_journees": s.demi_journees,
                    }),
                    actor_user_id: Some(acteur.to_string()),
                },
                Causalite::default(),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(SignalementAssiduite {
            statut: nouveau.to_string(),
            ..s
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, j).unwrap()
    }

    #[test]
    fn effacement_fin_d_annee_scolaire() {
        let m = ModeEffacement::FinAnneeScolaire;
        // Prononcée en octobre 2026 → effacée le 1er septembre 2027.
        assert_eq!(date_effacement(d(2026, 10, 12), m), Some(d(2027, 9, 1)));
        // Prononcée en juin 2027 → le 1er septembre 2027.
        assert_eq!(date_effacement(d(2027, 6, 30), m), Some(d(2027, 9, 1)));
        // Bornes : 31 août / 1er septembre.
        assert_eq!(date_effacement(d(2027, 8, 31), m), Some(d(2027, 9, 1)));
        assert_eq!(date_effacement(d(2027, 9, 1), m), Some(d(2028, 9, 1)));
    }

    #[test]
    fn effacement_delai_en_mois_et_jamais() {
        assert_eq!(
            date_effacement(d(2026, 1, 31), ModeEffacement::DelaiMois(1)),
            Some(d(2026, 2, 28))
        );
        assert_eq!(
            date_effacement(d(2026, 3, 15), ModeEffacement::DelaiMois(12)),
            Some(d(2027, 3, 15))
        );
        assert_eq!(date_effacement(d(2026, 3, 15), ModeEffacement::Jamais), None);
    }

    #[test]
    fn regles_par_defaut_couvrent_l_echelle() {
        for c in CATEGORIES_SANCTION {
            // Aucune catégorie ne panique ; l'exclusion définitive n'est jamais effacée.
            let _ = regle_effacement_defaut(c);
        }
        assert_eq!(regle_effacement_defaut("exclusion_definitive"), ModeEffacement::Jamais);
        assert_eq!(regle_effacement_defaut("blame"), ModeEffacement::FinAnneeScolaire);
    }
}
