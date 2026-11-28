//! Socle 21, lot V3f (complément de V3) : barème détaillé, réussite par exercice, détail montré à l'élève, conversion
//! des notes à un changement de barème ou à l'annulation d'un exercice, duplication de plusieurs évaluations.
//!
//! * **La note est la somme des points** (§8.9) : le détail d'un élève s'écrit en même temps que sa note, par la même voie
//!   que la grille (`eva_v3_une_case` : droit, période de l'élève, version, historique, événement neutre) ; le moteur de
//!   calcul ne change pas.
//! * **Un détail n'est vrai que pour la version de la note qu'il compose** (`resultat_version`) : une note changée à la
//!   main dans la grille rend le détail « périmé » ; il n'est alors ni compté dans la réussite, ni montré à la famille.
//! * **Conversion** (CE-39, CE-83) : aperçu (notes avant / après, empreinte), puis confirmation qui exige la même
//!   empreinte et un motif ; **tout ou rien** dans une transaction ; l'historique de chaque note garde l'ancienne valeur ;
//!   le changement de barème est journalisé par la base (M4).
//! * **Droits** : définir le barème, le montrer, convertir = le propriétaire de l'évaluation (titulaire, co-enseignant ;
//!   un suppléant pour ses propres évaluations), et après l'arrêt la direction (`notes.corriger`) pour convertir ; lire =
//!   comme la grille ; famille et élève : la seule lecture du détail **publié** et **montré**, sous « voir les notes ».
//!
//! Isolation : toutes les requêtes portent `etablissement_id` et passent par `TenantTx` (RLS).

use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::NaiveDate;
use domain_eleve::evaluation_moteur::StatutCalcul;
use ent_types::evaluations_v3::{formater_milliemes, CelluleRetourDto, CelluleSaisieDto};
use ent_types::evaluations_v3f::{
    convertir_proportion, tranche, ApercuConversionDto, AppliquerConversionBody, BaremeDetailleDto,
    ConversionAppliqueeDto, ConversionBody, DefinirBaremeBody, DetailEleveDto, DetailFamilleDto,
    DetailsEvaluationDto, DuplicationDto, DupliquerBody, EvaluationDupliqueeDto, ExerciceDto,
    ExerciceFamilleDto, LigneConversionDto, ReussiteExerciceDto, SaisirDetailsBody,
    SaisirDetailsRetourDto, VisibiliteDetailBody, MAX_DUPLICATIONS, MAX_ELEVES_DETAILS,
    MAX_EXERCICES,
};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus};
use crate::evaluations_v1::{regles_tx, Attendu, EcrireResultat, SourceResultat};
use crate::evaluations_v3::{types_v3, AccesCours};
use crate::evaluations_v3_eval::{map_garde, publier_evt};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Types d'événements propres à V3f. Charges : identifiant de l'évaluation seulement.
pub mod types_v3f {
    pub const BAREME_DETAILLE_DEFINI: &str = "evaluations.bareme_detaille_defini";
    pub const NOTES_CONVERTIES: &str = "evaluations.notes_converties";
}

const MAX_MOTIF: usize = 500;

/// Une évaluation lue pour ce lot.
struct EvalV3f {
    cours_id: Uuid,
    annee_id: Uuid,
    version: i32,
    etat: String,
    mode: String,
    bareme_m: i64,
    hors_bareme: bool,
    cree_par: Option<Uuid>,
    date_eval: NaiveDate,
    publiee: bool,
    detail_visible: bool,
    titre: String,
}

/// Un exercice lu.
#[derive(Clone)]
struct Exercice {
    id: Uuid,
    ordre: i16,
    libelle: String,
    maximum_m: i64,
    annule: bool,
}

impl Exercice {
    fn dto(&self) -> ExerciceDto {
        ExerciceDto {
            id: self.id,
            ordre: self.ordre,
            libelle: self.libelle.clone(),
            maximum_m: self.maximum_m,
            annule: self.annule,
        }
    }
}

/// Une note lue (statut `note` ou autre) et son détail.
struct NoteLue {
    resultat_id: Uuid,
    eleve_id: Uuid,
    statut: String,
    valeur_m: Option<i64>,
    version: i32,
    /// Points par exercice NON ANNULÉ et version de la note que ce détail compose (les points d'un exercice annulé sont
    /// gardés en base mais ne composent plus la note).
    points: HashMap<Uuid, (i64, Option<i32>)>,
}

impl NoteLue {
    /// Le détail compose la note actuelle : statut `note`, au moins un point, tous à la version actuelle.
    fn detail_a_jour(&self) -> bool {
        self.statut == "note"
            && !self.points.is_empty()
            && self.points.values().all(|(_, v)| *v == Some(self.version))
    }
}

async fn charger_eval(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
    verrou: bool,
) -> Result<EvalV3f, EntDbError> {
    if verrou {
        // Verrou dans une requête à part, puis relecture (même motif que la modification d'une évaluation, F8 de V3).
        sqlx::query(
            "SELECT 1 FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
    }
    let r = sqlx::query(
        "SELECT e.cours_id, e.annee_id, e.version, e.etat, e.mode, COALESCE((e.bareme * 1000)::bigint, 0) AS bareme_m,
                e.hors_bareme_autorise, e.cree_par, e.date_eval, (e.publiee_le IS NOT NULL) AS publiee,
                e.detail_visible_eleve, e.titre
           FROM eva_evaluation e WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(etab)
    .bind(evaluation_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    Ok(EvalV3f {
        cours_id: r.try_get("cours_id").map_err(db)?,
        annee_id: r.try_get("annee_id").map_err(db)?,
        version: r.try_get("version").map_err(db)?,
        etat: r.try_get("etat").map_err(db)?,
        mode: r.try_get("mode").map_err(db)?,
        bareme_m: r.try_get("bareme_m").map_err(db)?,
        hors_bareme: r.try_get("hors_bareme_autorise").map_err(db)?,
        cree_par: r.try_get("cree_par").map_err(db)?,
        date_eval: r.try_get("date_eval").map_err(db)?,
        publiee: r.try_get("publiee").map_err(db)?,
        detail_visible: r.try_get("detail_visible_eleve").map_err(db)?,
        titre: r.try_get("titre").map_err(db)?,
    })
}

async fn charger_exercices(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
) -> Result<Vec<Exercice>, EntDbError> {
    let rows = sqlx::query(
        "SELECT id, ordre, libelle, (maximum * 1000)::bigint AS maximum_m, annule
           FROM eva_bareme_item WHERE etablissement_id = $1 AND evaluation_id = $2 ORDER BY ordre, id",
    )
    .bind(etab)
    .bind(evaluation_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    rows.iter()
        .map(|r| {
            Ok(Exercice {
                id: r.try_get("id")?,
                ordre: r.try_get("ordre")?,
                libelle: r.try_get("libelle")?,
                maximum_m: r.try_get("maximum_m")?,
                annule: r.try_get("annule")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(db)
}

async fn charger_notes(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
) -> Result<Vec<NoteLue>, EntDbError> {
    let rows = sqlx::query(
        "SELECT r.id, r.eleve_id, r.statut, (r.valeur * 1000)::bigint AS valeur_m, r.version
           FROM eva_resultat r WHERE r.etablissement_id = $1 AND r.evaluation_id = $2 ORDER BY r.eleve_id",
    )
    .bind(etab)
    .bind(evaluation_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut notes = Vec::with_capacity(rows.len());
    let mut index: HashMap<Uuid, usize> = HashMap::new();
    for r in &rows {
        let id: Uuid = r.try_get("id").map_err(db)?;
        index.insert(id, notes.len());
        notes.push(NoteLue {
            resultat_id: id,
            eleve_id: r.try_get("eleve_id").map_err(db)?,
            statut: r.try_get("statut").map_err(db)?,
            valeur_m: r.try_get("valeur_m").map_err(db)?,
            version: r.try_get("version").map_err(db)?,
            points: HashMap::new(),
        });
    }
    if !notes.is_empty() {
        let items = sqlx::query(
            "SELECT ri.resultat_id, ri.item_id, (ri.valeur * 1000)::bigint AS valeur_m, ri.resultat_version
               FROM eva_resultat_item ri
               JOIN eva_resultat r ON r.etablissement_id = ri.etablissement_id AND r.id = ri.resultat_id
               JOIN eva_bareme_item i ON i.etablissement_id = ri.etablissement_id AND i.id = ri.item_id
              WHERE ri.etablissement_id = $1 AND r.evaluation_id = $2 AND NOT i.annule",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
        for it in &items {
            let rid: Uuid = it.try_get("resultat_id").map_err(db)?;
            if let Some(&i) = index.get(&rid) {
                notes[i].points.insert(
                    it.try_get("item_id").map_err(db)?,
                    (
                        it.try_get("valeur_m").map_err(db)?,
                        it.try_get("resultat_version").map_err(db)?,
                    ),
                );
            }
        }
    }
    Ok(notes)
}

/// Nombre d'élèves du public dont la période est arrêtée à la date de l'évaluation.
async fn nb_arretes(tx: &mut TenantTx<'_>, etab: Uuid, ev: &EvalV3f) -> Result<i64, EntDbError> {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_public_cours_jour($1, $2, $3) pu
           CROSS JOIN LATERAL eva_periode_eleve_jour($1, pu.eleve_id, $2) pe
          WHERE pe.etat <> 'ouverte'",
    )
    .bind(etab)
    .bind(ev.date_eval)
    .bind(ev.cours_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)
}

/// Propriétaire de l'évaluation (CE-44 : un suppléant ne règle que les siennes).
fn proprietaire(acces: &AccesCours, ev: &EvalV3f, acteur: Uuid) -> bool {
    acces.saisie && (ev.cree_par == Some(acteur) || !acces.suppleant)
}

fn motif_valide(m: Option<&str>) -> Result<Option<String>, EntDbError> {
    let m = m.map(str::trim).filter(|m| !m.is_empty());
    if m.is_some_and(|m| m.chars().count() > MAX_MOTIF) {
        return Err(refus("motif trop long (500 caractères au plus)"));
    }
    Ok(m.map(str::to_string))
}

/// Ce qu'une conversion ferait : nouveau barème, exercice annulé, lignes proposées, notes non convertibles.
struct PlanConversion {
    bareme_apres_m: i64,
    exercice: Option<Uuid>,
    lignes: Vec<(NoteLue, i64, &'static str)>,
    non_convertibles: i32,
}

fn empreinte(ev_id: Uuid, ev: &EvalV3f, plan: &PlanConversion) -> String {
    let mut h = Sha256::new();
    h.update(
        format!(
            "v3f|{ev_id}|{}|{}|{}|{}|{}|",
            ev.version,
            ev.bareme_m,
            plan.bareme_apres_m,
            plan.exercice
                .map_or_else(|| "-".to_string(), |e| e.to_string()),
            plan.non_convertibles
        )
        .as_bytes(),
    );
    for (n, apres, source) in &plan.lignes {
        h.update(
            format!(
                "{}|{}|{}|{}|{source};",
                n.eleve_id,
                n.version,
                n.valeur_m.unwrap_or(-1),
                apres
            )
            .as_bytes(),
        );
    }
    format!("{:x}", h.finalize())
}

impl PgEnt {
    async fn eva_v3f_acces_evaluation(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        evaluation_id: Uuid,
    ) -> Result<AccesCours, EntDbError> {
        Self::exiger_staff(role)?;
        let cours_id = self.eva_v3_cours_de(etab, evaluation_id).await?;
        self.eva_v3_acces(etab, acteur, role, cours_id, jour).await
    }

    // ───────────────────────── Barème détaillé (EVA-262, 263) ─────────────────────────

    /// Le barème détaillé d'une évaluation et la réussite par exercice (moyenne, répartition), sur les seuls détails à
    /// jour. Lisible comme la grille (professeur du cours ; direction et secrétariat en lecture).
    pub async fn eva_v3f_bareme_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        evaluation_id: Uuid,
    ) -> Result<BaremeDetailleDto, EntDbError> {
        self.eva_v3f_acces_evaluation(etab, acteur, role, jour, evaluation_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = charger_eval(&mut tx, etab, evaluation_id, false).await?;
        let exercices = charger_exercices(&mut tx, etab, evaluation_id).await?;
        let notes = charger_notes(&mut tx, etab, evaluation_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(Self::eva_v3f_bareme_dto(
            evaluation_id,
            &ev,
            &exercices,
            &notes,
        ))
    }

    fn eva_v3f_bareme_dto(
        evaluation_id: Uuid,
        ev: &EvalV3f,
        exercices: &[Exercice],
        notes: &[NoteLue],
    ) -> BaremeDetailleDto {
        let a_jour: Vec<&NoteLue> = notes.iter().filter(|n| n.detail_a_jour()).collect();
        let perimes = notes
            .iter()
            .filter(|n| !n.points.is_empty() && !n.detail_a_jour())
            .count();
        let reussite = exercices
            .iter()
            .filter(|x| !x.annule)
            .map(|x| {
                let pts: Vec<i64> = a_jour
                    .iter()
                    .filter_map(|n| n.points.get(&x.id).map(|(p, _)| *p))
                    .collect();
                let mut repartition = [0i32; 4];
                for p in &pts {
                    repartition[tranche(*p, x.maximum_m)] += 1;
                }
                let somme: i128 = pts.iter().map(|p| i128::from(*p)).sum();
                ReussiteExerciceDto {
                    exercice_id: x.id,
                    effectif: i32::try_from(pts.len()).unwrap_or(i32::MAX),
                    moyenne_m: (!pts.is_empty())
                        .then(|| i64::try_from(somme / pts.len() as i128).unwrap_or(i64::MAX)),
                    repartition,
                }
            })
            .collect();
        BaremeDetailleDto {
            evaluation_id,
            version: ev.version,
            bareme_m: ev.bareme_m,
            somme_maximums_m: exercices
                .iter()
                .filter(|x| !x.annule)
                .map(|x| x.maximum_m)
                .sum(),
            detail_visible_eleve: ev.detail_visible,
            publiee: ev.publiee,
            exercices: exercices.iter().map(Exercice::dto).collect(),
            reussite,
            nb_details_a_jour: i32::try_from(a_jour.len()).unwrap_or(i32::MAX),
            nb_details_perimes: i32::try_from(perimes).unwrap_or(i32::MAX),
            redefinissable: notes.iter().all(|n| n.points.is_empty())
                && exercices.iter().all(|x| !x.annule),
        }
    }

    /// Définit (ou redéfinit) le barème détaillé (EVA-262) : la somme des maximums doit égaler le barème ; tant qu'aucun
    /// point n'est saisi et qu'aucun exercice n'est annulé. Une liste vide retire le barème détaillé.
    pub async fn eva_v3f_definir_bareme_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        evaluation_id: Uuid,
        body: &DefinirBaremeBody,
    ) -> Result<BaremeDetailleDto, EntDbError> {
        let acces = self
            .eva_v3f_acces_evaluation(etab, acteur, role, jour, evaluation_id)
            .await?;
        if body.exercices.len() > MAX_EXERCICES {
            return Err(refus("un barème détaillé compte au plus 40 exercices"));
        }
        let mut somme: i64 = 0;
        let mut libelles = Vec::with_capacity(body.exercices.len());
        for x in &body.exercices {
            let l = x.libelle.trim();
            if l.is_empty() || l.chars().count() > 160 || l.chars().any(char::is_control) {
                return Err(refus(
                    "le libellé d'un exercice est obligatoire (160 caractères au plus)",
                ));
            }
            if x.maximum_m <= 0 || x.maximum_m > 1_000_000 {
                return Err(refus(
                    "le maximum d'un exercice est entre 0,001 et 1000 points",
                ));
            }
            somme += x.maximum_m;
            libelles.push(l.to_string());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = charger_eval(&mut tx, etab, evaluation_id, true).await?;
        if !proprietaire(&acces, &ev, acteur) {
            return Err(EntDbError::Forbidden);
        }
        if ev.version != body.version_attendue {
            return Err(conflit("l'évaluation a changé depuis votre lecture"));
        }
        if ev.etat != "ouverte" || ev.mode != "note" {
            return Err(refus(
                "une évaluation annulée ou sans note n'a pas de barème détaillé",
            ));
        }
        if !body.exercices.is_empty() && somme != ev.bareme_m {
            return Err(refus(
                "la somme des maximums des exercices doit égaler le barème de l'évaluation : changez d'abord le barème",
            ));
        }
        let exercices = charger_exercices(&mut tx, etab, evaluation_id).await?;
        let notes = charger_notes(&mut tx, etab, evaluation_id).await?;
        if exercices.iter().any(|x| x.annule) || notes.iter().any(|n| !n.points.is_empty()) {
            return Err(refus(
                "des points sont saisis ou un exercice est annulé : le barème détaillé ne se redéfinit plus",
            ));
        }
        sqlx::query(
            "DELETE FROM eva_bareme_item WHERE etablissement_id = $1 AND evaluation_id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .execute(&mut *tx)
        .await
        .map_err(map_garde)?;
        for (i, (x, l)) in body.exercices.iter().zip(&libelles).enumerate() {
            sqlx::query(
                "INSERT INTO eva_bareme_item (id, etablissement_id, evaluation_id, ordre, libelle, maximum)
                 VALUES ($1, $2, $3, $4, $5, $6::bigint / 1000.0)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(evaluation_id)
            .bind(i16::try_from(i + 1).unwrap_or(i16::MAX))
            .bind(l)
            .bind(x.maximum_m)
            .execute(&mut *tx)
            .await
            .map_err(map_garde)?;
        }
        // La version de l'évaluation avance : tout aperçu ou écran ouvert avant ce changement devient périmé.
        sqlx::query(
            "UPDATE eva_evaluation SET modifie_par = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(map_garde)?;
        publier_evt(
            &mut tx,
            etab,
            types_v3f::BAREME_DETAILLE_DEFINI,
            evaluation_id,
            acteur,
        )
        .await?;
        let ev = charger_eval(&mut tx, etab, evaluation_id, false).await?;
        let exercices = charger_exercices(&mut tx, etab, evaluation_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(Self::eva_v3f_bareme_dto(
            evaluation_id,
            &ev,
            &exercices,
            &notes,
        ))
    }

    /// Montre ou cache le détail par exercice à l'élève et à sa famille (EVA-264). Le détail ne se voit qu'avec la note
    /// publiée, et seulement s'il est à jour.
    pub async fn eva_v3f_visibilite_detail_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        evaluation_id: Uuid,
        body: &VisibiliteDetailBody,
    ) -> Result<BaremeDetailleDto, EntDbError> {
        let acces = self
            .eva_v3f_acces_evaluation(etab, acteur, role, jour, evaluation_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = charger_eval(&mut tx, etab, evaluation_id, true).await?;
        if !proprietaire(&acces, &ev, acteur) {
            return Err(EntDbError::Forbidden);
        }
        if ev.version != body.version_attendue {
            return Err(conflit("l'évaluation a changé depuis votre lecture"));
        }
        if ev.etat != "ouverte" {
            return Err(refus("une évaluation annulée ne se modifie pas"));
        }
        let exercices = charger_exercices(&mut tx, etab, evaluation_id).await?;
        if body.visible && exercices.iter().all(|x| x.annule) {
            return Err(refus("cette évaluation n'a pas de barème détaillé"));
        }
        if ev.detail_visible != body.visible {
            sqlx::query(
                "UPDATE eva_evaluation SET detail_visible_eleve = $3, modifie_par = $4
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(evaluation_id)
            .bind(body.visible)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(map_garde)?;
            publier_evt(
                &mut tx,
                etab,
                types_v3::EVALUATION_MODIFIEE,
                evaluation_id,
                acteur,
            )
            .await?;
        }
        let ev = charger_eval(&mut tx, etab, evaluation_id, false).await?;
        let notes = charger_notes(&mut tx, etab, evaluation_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(Self::eva_v3f_bareme_dto(
            evaluation_id,
            &ev,
            &exercices,
            &notes,
        ))
    }

    // ───────────────────────── Détail par élève ─────────────────────────

    /// La grille de détail : élèves notés et du public, points par exercice, note et version, détail à jour ou non.
    pub async fn eva_v3f_details_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        evaluation_id: Uuid,
    ) -> Result<DetailsEvaluationDto, EntDbError> {
        self.eva_v3f_acces_evaluation(etab, acteur, role, jour, evaluation_id)
            .await?;
        let profil = self.profil_champs(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let exercices = charger_exercices(&mut tx, etab, evaluation_id).await?;
        let notes = charger_notes(&mut tx, etab, evaluation_id).await?;
        let public: Vec<Uuid> =
            sqlx::query_scalar("SELECT g.eleve_id FROM eva_grille_eleves($1, $2) g")
                .bind(etab)
                .bind(evaluation_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
        let mut ids: BTreeSet<Uuid> = public.into_iter().collect();
        ids.extend(notes.iter().map(|n| n.eleve_id));
        let ids: Vec<Uuid> = ids.into_iter().collect();
        let noms: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let caches = self.proteges_caches(etab, role, &profil, &ids).await?;
        let par_eleve: HashMap<Uuid, &NoteLue> = notes.iter().map(|n| (n.eleve_id, n)).collect();
        let mut eleves: Vec<DetailEleveDto> = noms
            .into_iter()
            .filter(|(id, _, _)| !caches.contains(id))
            .map(|(id, nom, prenom)| {
                let n = par_eleve.get(&id);
                DetailEleveDto {
                    eleve_id: id,
                    nom,
                    prenom,
                    statut: n.map(|n| n.statut.clone()),
                    note_m: n.and_then(|n| n.valeur_m),
                    version: n.map(|n| n.version),
                    points_m: exercices
                        .iter()
                        .map(|x| n.and_then(|n| n.points.get(&x.id).map(|(p, _)| *p)))
                        .collect(),
                    a_jour: n.is_some_and(|n| n.detail_a_jour()),
                }
            })
            .collect();
        eleves.sort_by_key(|e| (e.nom.to_lowercase(), e.prenom.to_lowercase(), e.eleve_id));
        Ok(DetailsEvaluationDto {
            evaluation_id,
            exercices: exercices.iter().map(Exercice::dto).collect(),
            eleves,
        })
    }

    /// Saisit le détail d'un ou plusieurs élèves : un point par exercice non annulé ; la note écrite est leur somme, par
    /// la voie de la grille (droits, période de l'élève, version attendue, historique). Chaque élève est indépendant.
    pub async fn eva_v3f_saisir_details_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        evaluation_id: Uuid,
        body: &SaisirDetailsBody,
    ) -> Result<SaisirDetailsRetourDto, EntDbError> {
        if body.eleves.len() > MAX_ELEVES_DETAILS {
            return Err(refus("trop d'élèves dans un même envoi (300 au plus)"));
        }
        let acces = self
            .eva_v3f_acces_evaluation(etab, acteur, role, jour, evaluation_id)
            .await?;
        if !acces.saisie && !acces.corriger {
            return Err(EntDbError::Forbidden);
        }
        let profil = self.profil_champs(etab, acteur, role).await?;
        let ids: Vec<Uuid> = body.eleves.iter().map(|e| e.eleve_id).collect();
        let caches = self.proteges_caches(etab, role, &profil, &ids).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = charger_eval(&mut tx, etab, evaluation_id, false).await?;
        let exercices: Vec<Exercice> = charger_exercices(&mut tx, etab, evaluation_id)
            .await?
            .into_iter()
            .filter(|x| !x.annule)
            .collect();
        if exercices.is_empty() {
            return Err(refus("cette évaluation n'a pas de barème détaillé"));
        }
        let pas_m = regles_tx(&mut tx, etab, ev.annee_id).await?.pas_saisie_m;
        let mut evs = HashMap::new();
        evs.insert(
            evaluation_id,
            (
                ev.cours_id,
                ev.bareme_m,
                ev.hors_bareme,
                ev.etat.clone(),
                ev.annee_id,
            ),
        );
        let mut pas_par_annee = HashMap::new();
        pas_par_annee.insert(ev.annee_id, pas_m);
        let max_par_ex: HashMap<Uuid, i64> =
            exercices.iter().map(|x| (x.id, x.maximum_m)).collect();
        // Ordre de verrouillage stable (élève), réponse dans l'ordre de la requête (comme la grille, V3c).
        let mut ordre: Vec<usize> = (0..body.eleves.len()).collect();
        ordre.sort_by_key(|&i| (body.eleves[i].eleve_id, i));
        let mut retours: Vec<Option<CelluleRetourDto>> = vec![None; body.eleves.len()];
        let mut vus: HashSet<Uuid> = HashSet::new();
        for &i in &ordre {
            let d = &body.eleves[i];
            let base = |issue: &str, message: &str| CelluleRetourDto {
                evaluation_id,
                eleve_id: d.eleve_id,
                issue: issue.to_string(),
                version: None,
                statut: None,
                valeur_m: None,
                message: Some(message.to_string()),
                conflit: None,
            };
            if !vus.insert(d.eleve_id) {
                retours[i] = Some(base("refuse", "élève présent deux fois dans l'envoi"));
                continue;
            }
            // Un point par exercice non annulé, ni plus ni moins, dans [0, maximum], au pas de saisie.
            let mut points: HashMap<Uuid, i64> = HashMap::new();
            let mut erreur: Option<&str> = None;
            for p in &d.points {
                match max_par_ex.get(&p.exercice_id) {
                    None => erreur = Some("exercice inconnu ou annulé"),
                    Some(max) if p.points_m < 0 || p.points_m > *max => {
                        erreur = Some("des points dépassent le maximum de l'exercice")
                    }
                    Some(_) if pas_m > 1 && p.points_m % pas_m != 0 => {
                        erreur =
                            Some("des points ne respectent pas le pas de saisie de l'établissement")
                    }
                    Some(_) => {
                        if points.insert(p.exercice_id, p.points_m).is_some() {
                            erreur = Some("un exercice est donné deux fois");
                        }
                    }
                }
            }
            if erreur.is_none() && points.len() != exercices.len() {
                erreur = Some("chaque exercice doit recevoir ses points (0 compris)");
            }
            if let Some(m) = erreur {
                retours[i] = Some(base("refuse", m));
                continue;
            }
            let somme: i64 = points.values().sum();
            let cell = CelluleSaisieDto {
                evaluation_id,
                eleve_id: d.eleve_id,
                texte: formater_milliemes(somme),
                version_attendue: d.version_attendue,
                motif: d.motif.clone(),
            };
            sqlx::query("SAVEPOINT eva_v3f_eleve")
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            let mut retour = self
                .eva_v3_une_case(
                    &mut tx,
                    etab,
                    acteur,
                    jour,
                    &acces,
                    ev.cours_id,
                    SourceResultat::Saisie,
                    &evs,
                    &mut pas_par_annee,
                    &caches,
                    &cell,
                )
                .await?;
            if matches!(retour.issue.as_str(), "ecrit" | "inchange") {
                let version = retour.version.unwrap_or(1);
                let ecrit = self
                    .eva_v3f_ecrire_points(
                        &mut tx,
                        etab,
                        acteur,
                        evaluation_id,
                        d.eleve_id,
                        version,
                        &points,
                    )
                    .await;
                match ecrit {
                    Ok(change) => {
                        if change && retour.issue == "inchange" {
                            retour.issue = "ecrit".to_string();
                        }
                    }
                    Err(_) => {
                        sqlx::query("ROLLBACK TO SAVEPOINT eva_v3f_eleve")
                            .execute(&mut *tx)
                            .await
                            .map_err(db)?;
                        retour = base("refuse", "détail refusé par les règles du barème");
                    }
                }
            }
            sqlx::query("RELEASE SAVEPOINT eva_v3f_eleve")
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            retours[i] = Some(retour);
        }
        tx.commit().await.map_err(db)?;
        Ok(SaisirDetailsRetourDto {
            eleves: retours.into_iter().flatten().collect(),
        })
    }

    /// Écrit les points d'un élève à la version `version` de sa note. Rend vrai si un point a changé.
    #[allow(clippy::too_many_arguments)]
    async fn eva_v3f_ecrire_points(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        acteur: Uuid,
        evaluation_id: Uuid,
        eleve_id: Uuid,
        version: i32,
        points: &HashMap<Uuid, i64>,
    ) -> Result<bool, EntDbError> {
        let resultat_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
        )
        .bind(etab)
        .bind(evaluation_id)
        .bind(eleve_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        let mut change = false;
        let mut cles: Vec<(&Uuid, &i64)> = points.iter().collect();
        cles.sort();
        for (item, p) in cles {
            let n = sqlx::query(
                "INSERT INTO eva_resultat_item (resultat_id, item_id, etablissement_id, valeur, resultat_version, saisi_par)
                 VALUES ($1, $2, $3, $4::bigint / 1000.0, $5, $6)
                 ON CONFLICT (resultat_id, item_id) DO UPDATE
                    SET valeur = EXCLUDED.valeur, resultat_version = EXCLUDED.resultat_version, saisi_par = EXCLUDED.saisi_par
                  WHERE eva_resultat_item.valeur IS DISTINCT FROM EXCLUDED.valeur
                     OR eva_resultat_item.resultat_version IS DISTINCT FROM EXCLUDED.resultat_version",
            )
            .bind(resultat_id)
            .bind(item)
            .bind(etab)
            .bind(p)
            .bind(version)
            .bind(acteur)
            .execute(&mut **tx)
            .await
            .map_err(map_garde)?;
            change |= n.rows_affected() > 0;
        }
        Ok(change)
    }

    // ───────────────────────── Conversion (CE-39, CE-83) ─────────────────────────

    async fn eva_v3f_plan(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        acces: &AccesCours,
        ev: &EvalV3f,
        evaluation_id: Uuid,
        nouveau_bareme_m: Option<i64>,
        annuler_exercice_id: Option<Uuid>,
    ) -> Result<PlanConversion, EntDbError> {
        if ev.etat != "ouverte" || ev.mode != "note" {
            return Err(refus(
                "une évaluation annulée ou sans note ne se convertit pas",
            ));
        }
        let arretes = nb_arretes(tx, etab, ev).await?;
        if !proprietaire(acces, ev, acteur) && !(acces.corriger && arretes > 0) {
            return Err(EntDbError::Forbidden);
        }
        if arretes > 0 && !acces.corriger {
            return Err(EntDbError::Forbidden);
        }
        let exercices = charger_exercices(tx, etab, evaluation_id).await?;
        let notes = charger_notes(tx, etab, evaluation_id).await?;
        let pas_m = regles_tx(tx, etab, ev.annee_id).await?.pas_saisie_m;
        let (bareme_apres_m, exercice) = match (nouveau_bareme_m, annuler_exercice_id) {
            (Some(b), None) => {
                if b <= 0 || b > 1_000_000 {
                    return Err(refus("barème invalide (entre 0,001 et 1000 points)"));
                }
                if b == ev.bareme_m {
                    return Err(refus("le nouveau barème est le barème actuel"));
                }
                if exercices.iter().any(|x| !x.annule) {
                    return Err(refus(
                        "barème détaillé : annulez un exercice ou redéfinissez les exercices au lieu de changer le barème",
                    ));
                }
                (b, None)
            }
            (None, Some(x)) => {
                let Some(ex) = exercices.iter().find(|e| e.id == x) else {
                    return Err(introuvable());
                };
                if ex.annule {
                    return Err(refus("cet exercice est déjà annulé"));
                }
                let restants: i64 = exercices
                    .iter()
                    .filter(|e| !e.annule && e.id != x)
                    .map(|e| e.maximum_m)
                    .sum();
                if restants <= 0 {
                    return Err(refus(
                        "le dernier exercice ne s'annule pas : annulez l'évaluation",
                    ));
                }
                if ev.bareme_m - ex.maximum_m != restants {
                    return Err(refus(
                        "le barème de l'évaluation ne correspond plus à ses exercices : corrigez le barème détaillé",
                    ));
                }
                (restants, Some(x))
            }
            _ => {
                return Err(refus(
                    "indiquez soit un nouveau barème, soit l'exercice à annuler",
                ))
            }
        };
        let profil = self.profil_champs(etab, acteur, role).await?;
        let ids: Vec<Uuid> = notes.iter().map(|n| n.eleve_id).collect();
        let caches = self.proteges_caches(etab, role, &profil, &ids).await?;
        let mut lignes = Vec::new();
        let mut non_convertibles = 0;
        for n in notes {
            if n.statut != "note" {
                continue;
            }
            let Some(avant) = n.valeur_m else { continue };
            if caches.contains(&n.eleve_id) {
                non_convertibles += 1;
                continue;
            }
            let (apres, source) = match exercice {
                Some(x) if n.detail_a_jour() => (
                    n.points
                        .iter()
                        .filter(|(id, _)| **id != x)
                        .map(|(_, (p, _))| *p)
                        .sum::<i64>(),
                    "detail",
                ),
                _ => (
                    convertir_proportion(avant, ev.bareme_m, bareme_apres_m, pas_m)
                        .ok_or(refus("conversion impossible pour une note"))?,
                    "proportion",
                ),
            };
            lignes.push((n, apres, source));
        }
        Ok(PlanConversion {
            bareme_apres_m,
            exercice,
            lignes,
            non_convertibles,
        })
    }

    /// Aperçu d'une conversion : chaque note avant et après, et l'empreinte à rendre à la confirmation. N'écrit rien.
    pub async fn eva_v3f_apercu_conversion_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        evaluation_id: Uuid,
        body: &ConversionBody,
    ) -> Result<ApercuConversionDto, EntDbError> {
        let acces = self
            .eva_v3f_acces_evaluation(etab, acteur, role, jour, evaluation_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = charger_eval(&mut tx, etab, evaluation_id, false).await?;
        if ev.version != body.version_attendue {
            return Err(conflit("l'évaluation a changé depuis votre lecture"));
        }
        let plan = self
            .eva_v3f_plan(
                &mut tx,
                etab,
                acteur,
                role,
                &acces,
                &ev,
                evaluation_id,
                body.nouveau_bareme_m,
                body.annuler_exercice_id,
            )
            .await?;
        let ids: Vec<Uuid> = plan.lignes.iter().map(|(n, _, _)| n.eleve_id).collect();
        let noms: HashMap<Uuid, (String, String)> = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT id, nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(id, n, p)| (id, (n, p)))
        .collect();
        tx.commit().await.map_err(db)?;
        let mut lignes: Vec<LigneConversionDto> = plan
            .lignes
            .iter()
            .map(|(n, apres, source)| {
                let (nom, prenom) = noms.get(&n.eleve_id).cloned().unwrap_or_default();
                LigneConversionDto {
                    eleve_id: n.eleve_id,
                    nom,
                    prenom,
                    version: n.version,
                    avant_m: n.valeur_m.unwrap_or(0),
                    apres_m: *apres,
                    source: (*source).to_string(),
                }
            })
            .collect();
        lignes.sort_by_key(|l| (l.nom.to_lowercase(), l.prenom.to_lowercase(), l.eleve_id));
        Ok(ApercuConversionDto {
            evaluation_id,
            bareme_avant_m: ev.bareme_m,
            bareme_apres_m: plan.bareme_apres_m,
            lignes,
            nb_non_convertibles: plan.non_convertibles,
            publiee: ev.publiee,
            empreinte: empreinte(evaluation_id, &ev, &plan),
        })
    }

    /// Applique une conversion : même demande que l'aperçu, même empreinte, motif ; **tout ou rien**. Chaque note est
    /// réécrite par la voie commune (historique : l'ancienne valeur reste), le barème change par une correction motivée
    /// (journal de l'évaluation, M4), l'exercice annulé garde sa date et son auteur ; un détail à jour suit la note.
    pub async fn eva_v3f_appliquer_conversion_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        evaluation_id: Uuid,
        body: &AppliquerConversionBody,
    ) -> Result<ConversionAppliqueeDto, EntDbError> {
        let motif = motif_valide(Some(&body.motif))?
            .filter(|m| m.chars().count() >= 3)
            .ok_or(refus("motif écrit obligatoire (3 caractères au moins)"))?;
        let acces = self
            .eva_v3f_acces_evaluation(etab, acteur, role, jour, evaluation_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = charger_eval(&mut tx, etab, evaluation_id, true).await?;
        if ev.version != body.version_attendue {
            return Err(conflit("l'évaluation a changé depuis votre lecture"));
        }
        let plan = self
            .eva_v3f_plan(
                &mut tx,
                etab,
                acteur,
                role,
                &acces,
                &ev,
                evaluation_id,
                body.nouveau_bareme_m,
                body.annuler_exercice_id,
            )
            .await?;
        if empreinte(evaluation_id, &ev, &plan) != body.empreinte {
            return Err(conflit(
                "les notes ont changé depuis l'aperçu : refaites l'aperçu avant de confirmer",
            ));
        }
        if plan.non_convertibles > 0 {
            return Err(refus(
                "des notes de cette évaluation ne vous sont pas accessibles : la conversion revient à la direction",
            ));
        }
        let hausse = plan.bareme_apres_m > ev.bareme_m;
        if hausse {
            Self::eva_v3f_changer_bareme(
                &mut tx,
                etab,
                acteur,
                evaluation_id,
                plan.bareme_apres_m,
                &motif,
            )
            .await?;
        }
        let mut n_converties = 0;
        for (n, apres, source) in &plan.lignes {
            if Some(*apres) == n.valeur_m {
                continue;
            }
            let ecrit = Self::ecrire_resultat_cause_tx(
                &mut tx,
                etab,
                acteur,
                acces.corriger,
                jour,
                &EcrireResultat {
                    evaluation_id,
                    eleve_id: n.eleve_id,
                    statut: StatutCalcul::Note,
                    valeur_m: Some(*apres),
                    motif: Some(motif.clone()),
                    commentaire_eleve: None,
                    source: SourceResultat::Saisie,
                },
                Attendu::Version(n.version),
            )
            .await
            .map_err(|e| match e {
                EntDbError::RefusDroitEcriture(c) => EntDbError::SaisieInvalide(c.message()),
                autre => autre,
            })?;
            n_converties += 1;
            // Le détail à jour suit la note : il compose désormais la nouvelle version.
            if *source == "detail" {
                sqlx::query(
                    "UPDATE eva_resultat_item SET resultat_version = $3
                      WHERE etablissement_id = $1 AND resultat_id = $2",
                )
                .bind(etab)
                .bind(n.resultat_id)
                .bind(ecrit.version)
                .execute(&mut *tx)
                .await
                .map_err(map_garde)?;
            }
        }
        if let Some(x) = plan.exercice {
            sqlx::query(
                "UPDATE eva_bareme_item SET annule = true, annule_le = now(), annule_par = $3
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(x)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(map_garde)?;
            // Les détails restés à jour (note inchangée : l'exercice annulé portait 0) composent toujours la note.
        }
        if !hausse {
            Self::eva_v3f_changer_bareme(
                &mut tx,
                etab,
                acteur,
                evaluation_id,
                plan.bareme_apres_m,
                &motif,
            )
            .await?;
        }
        publier_evt(
            &mut tx,
            etab,
            types_v3f::NOTES_CONVERTIES,
            evaluation_id,
            acteur,
        )
        .await?;
        let apres = charger_eval(&mut tx, etab, evaluation_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(ConversionAppliqueeDto {
            evaluation_id,
            version: apres.version,
            bareme_m: apres.bareme_m,
            nb_notes_converties: n_converties,
        })
    }

    async fn eva_v3f_changer_bareme(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        acteur: Uuid,
        evaluation_id: Uuid,
        bareme_m: i64,
        motif: &str,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "UPDATE eva_evaluation SET bareme = $3::bigint / 1000.0, modifie_par = $4, motif_correction = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .bind(bareme_m)
        .bind(acteur)
        .bind(motif)
        .execute(&mut **tx)
        .await
        .map_err(map_garde)?;
        Ok(())
    }

    // ───────────────────────── Duplication (EVA-125, M-667) ─────────────────────────

    /// Duplique plusieurs évaluations (de l'an passé, ou d'un autre de ses cours) dans l'un de ses cours, **sans
    /// résultats** : titre, barème, coefficient, comptage, nature, exercices non annulés, choix « détail montré » ;
    /// publication explicite, non annoncée, public entier. Tout ou rien.
    pub async fn eva_v3f_dupliquer_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &DupliquerBody,
    ) -> Result<DuplicationDto, EntDbError> {
        if body.evaluations.is_empty() || body.evaluations.len() > MAX_DUPLICATIONS {
            return Err(refus("choisissez de 1 à 60 évaluations à dupliquer"));
        }
        let cible = self
            .eva_v3_acces(etab, acteur, role, body.cours_cible_id, jour)
            .await?;
        if !cible.saisie {
            return Err(EntDbError::Forbidden);
        }
        // Chaque source : lisible par l'acteur à SA date (le professeur de l'an passé l'était ce jour-là).
        let mut sources = Vec::with_capacity(body.evaluations.len());
        for it in &body.evaluations {
            let (cours, date): (Uuid, NaiveDate) = {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                let r: Option<(Uuid, NaiveDate)> = sqlx::query_as(
                    "SELECT cours_id, date_eval FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(it.evaluation_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                tx.commit().await.map_err(db)?;
                r.ok_or_else(introuvable)?
            };
            let a = self.eva_v3_acces(etab, acteur, role, cours, date).await?;
            if !a.prof_du_cours {
                return Err(EntDbError::Forbidden);
            }
            sources.push(it);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (annee_cible, debut, fin, matiere_cible): (Uuid, NaiveDate, NaiveDate, Option<Uuid>) = sqlx::query_as(
            "SELECT c.annee_id, y.date_debut, y.date_fin, c.matiere_id
               FROM ent_cours c JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = $2",
        )
        .bind(etab)
        .bind(body.cours_cible_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let mut creees = Vec::with_capacity(sources.len());
        for it in sources {
            if it.date_eval < debut || it.date_eval > fin {
                return Err(refus(
                    "la date d'une copie est hors de l'année scolaire du cours",
                ));
            }
            let public: Vec<(Uuid, Option<String>)> = sqlx::query_as(
                "SELECT pu.eleve_id, pe.etat
                   FROM ent_public_cours_jour($1, $2, $3) pu
                   LEFT JOIN LATERAL eva_periode_eleve_jour($1, pu.eleve_id, $2) pe ON true",
            )
            .bind(etab)
            .bind(it.date_eval)
            .bind(body.cours_cible_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            if !public.is_empty() && public.iter().all(|(_, e)| e.as_deref() != Some("ouverte")) {
                return Err(refus(
                    "à la date d'une copie, la période est arrêtée pour tous les élèves du cours",
                ));
            }
            let src = sqlx::query(
                "SELECT e.titre, e.mode, e.etat, (e.bareme * 1000)::bigint AS bareme_m, (e.coef * 1000)::bigint AS coef_m, e.comptage,
                        e.hors_bareme_autorise, e.nature_id, e.matiere_id, e.detail_visible_eleve, c.matiere_id AS cours_matiere
                   FROM eva_evaluation e JOIN ent_cours c ON c.id = e.cours_id AND c.etablissement_id = e.etablissement_id
                  WHERE e.etablissement_id = $1 AND e.id = $2",
            )
            .bind(etab)
            .bind(it.evaluation_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let etat: String = src.try_get("etat").map_err(db)?;
            let mode: String = src.try_get("mode").map_err(db)?;
            if etat == "annulee" || mode != "note" {
                return Err(refus(
                    "une évaluation annulée ou sans note ne se duplique pas",
                ));
            }
            let cours_matiere: Option<Uuid> = src.try_get("cours_matiere").map_err(db)?;
            // La sous-matière ne suit que si le cours cible enseigne la même matière (garde F11 de la base).
            let sous_matiere: Option<Uuid> = src
                .try_get::<Option<Uuid>, _>("matiere_id")
                .map_err(db)?
                .filter(|_| cours_matiere.is_some() && cours_matiere == matiere_cible);
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO eva_evaluation (id, etablissement_id, annee_id, cours_id, date_eval, titre, mode, bareme, coef,
                                             comptage, hors_bareme_autorise, nature_id, matiere_id, etat, publication,
                                             annoncee, cree_par, detail_visible_eleve)
                 VALUES ($1, $2, $3, $4, $5, $6, 'note', $7::bigint / 1000.0, $8::bigint / 1000.0, $9, $10, $11, $12, 'ouverte', 'explicite',
                         false, $13, $14)",
            )
            .bind(id)
            .bind(etab)
            .bind(annee_cible)
            .bind(body.cours_cible_id)
            .bind(it.date_eval)
            .bind(src.try_get::<String, _>("titre").map_err(db)?)
            .bind(src.try_get::<i64, _>("bareme_m").map_err(db)?)
            .bind(src.try_get::<i64, _>("coef_m").map_err(db)?)
            .bind(src.try_get::<String, _>("comptage").map_err(db)?)
            .bind(src.try_get::<bool, _>("hors_bareme_autorise").map_err(db)?)
            .bind(src.try_get::<Option<Uuid>, _>("nature_id").map_err(db)?)
            .bind(sous_matiere)
            .bind(acteur)
            .bind(src.try_get::<bool, _>("detail_visible_eleve").map_err(db)?)
            .execute(&mut *tx)
            .await
            .map_err(map_garde)?;
            let n = sqlx::query(
                "INSERT INTO eva_bareme_item (id, etablissement_id, evaluation_id, ordre, libelle, maximum)
                 SELECT gen_random_uuid(), i.etablissement_id, $3, row_number() OVER (ORDER BY i.ordre, i.id), i.libelle, i.maximum
                   FROM eva_bareme_item i
                  WHERE i.etablissement_id = $1 AND i.evaluation_id = $2 AND NOT i.annule",
            )
            .bind(etab)
            .bind(it.evaluation_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_garde)?;
            publier_evt(&mut tx, etab, types_v3::EVALUATION_CREEE, id, acteur).await?;
            creees.push(EvaluationDupliqueeDto {
                source_id: it.evaluation_id,
                evaluation_id: id,
                date_eval: it.date_eval,
                nb_exercices: i32::try_from(n.rows_affected()).unwrap_or(i32::MAX),
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(DuplicationDto { creees })
    }

    // ───────────────────────── Famille et élève (EVA-264) ─────────────────────────

    /// Le détail par exercice d'une note **publiée**, si le professeur l'a montré et que le détail est à jour, pour
    /// l'élève ou un lien qui autorise la lecture des notes. Sinon : introuvable (rien n'est révélé).
    pub async fn eva_v3f_detail_famille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        evaluation_id: Uuid,
    ) -> Result<DetailFamilleDto, EntDbError> {
        if !self
            .actor_can_read_notes(etab, acteur, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let publie: Option<(String, Option<i64>, i64)> = sqlx::query_as(
            "SELECT p.statut, (p.valeur * 1000)::bigint, (p.bareme * 1000)::bigint
               FROM eva_resultat_publie p
              WHERE p.etablissement_id = $1 AND p.eleve_id = $2 AND p.evaluation_id = $3 AND p.etat_evaluation = 'ouverte'",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(evaluation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((statut, Some(note_m), bareme_m)) = publie else {
            return Err(introuvable());
        };
        let ev = charger_eval(&mut tx, etab, evaluation_id, false).await?;
        if statut != "note" || !ev.detail_visible {
            return Err(introuvable());
        }
        let exercices = charger_exercices(&mut tx, etab, evaluation_id).await?;
        let notes = charger_notes(&mut tx, etab, evaluation_id).await?;
        tx.commit().await.map_err(db)?;
        let Some(n) = notes.iter().find(|n| n.eleve_id == eleve_id) else {
            return Err(introuvable());
        };
        if !n.detail_a_jour() {
            return Err(introuvable());
        }
        Ok(DetailFamilleDto {
            evaluation_id,
            titre: ev.titre,
            bareme_m,
            note_m,
            exercices: exercices
                .iter()
                .map(|x| ExerciceFamilleDto {
                    libelle: x.libelle.clone(),
                    maximum_m: x.maximum_m,
                    annule: x.annule,
                    points_m: if x.annule {
                        None
                    } else {
                        n.points.get(&x.id).map(|(p, _)| *p)
                    },
                })
                .collect(),
        })
    }
}
