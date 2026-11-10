//! Appréciations, conseil de classe outillé, orientation et bilan de fin de cycle
//! (socle 2, lot S2-7 : EVA-05, EVA-06, EVA-08, EVA-09, EVA-11, ORI-02, ORI-03).
//!
//! Droits :
//! - appréciation de matière : professeur d'un service de cette matière qui suit l'élève
//!   (ou `notes.saisir` établissement), et `notes.saisir` sur l'élève ;
//! - appréciation générale : professeur principal de la classe ou `bulletins.publier` ;
//! - conseil : `conseils.gerer` **et** (lecture des notes de l'établissement ou
//!   enseignement dans la classe) ; décisions, mentions, avis : direction
//!   (`bulletins.publier`) ou professeur principal ;
//! - réponse de la famille à un avis : responsable au lien actif, seulement après la
//!   publication du bulletin de la période (§4.3 : pas de notification avant) ;
//! - bilan de cycle : `competences.evaluer` sur l'élève ; validation : PP ou direction.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use domain_eleve::evaluation::composante_connue;
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::evaluations::{
    AppreciationBody, AppreciationDto, AvisVoeuBody, BilanCycleBody, BilanCycleDto,
    ComposanteNiveauDto, ConseilDto, CreerConseilBody, DecisionConseilBody, DecisionConseilDto,
    PreparationConseilDto, PreparationEleveDto, VoeuAvisDto,
};
use serde_json::json;
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus, statut_periode_tx, texte_borne, types_eva};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};

const MENTIONS: [&str; 6] = [
    "felicitations",
    "complement",
    "encouragements",
    "mise_en_garde_travail",
    "mise_en_garde_conduite",
    "aucune",
];
const DECISIONS: [&str; 5] = ["aucune", "passage", "redoublement", "orientation", "reserve"];
const AVIS: [&str; 3] = ["favorable", "reserve", "defavorable"];

type ConseilTuple = (
    Uuid,
    Option<Uuid>,
    String,
    Option<Uuid>,
    String,
    NaiveDate,
    NaiveTime,
    String,
    String,
);

const COLONNES_CONSEIL: &str =
    "id, classe_id, classe, periode_id, periode_libelle, jour, heure, lieu, statut";

fn conseil_de(t: ConseilTuple) -> ConseilDto {
    ConseilDto {
        id: t.0,
        classe_id: t.1,
        classe: t.2,
        periode_id: t.3,
        periode_libelle: t.4,
        jour: t.5,
        heure: t.6,
        lieu: t.7,
        statut: t.8,
    }
}

impl PgEnt {
    async fn eva_conseil(&self, etab: Uuid, id: Uuid) -> Result<ConseilDto, EntDbError> {
        let row: Option<ConseilTuple> = sqlx::query_as(&format!(
            "SELECT {COLONNES_CONSEIL} FROM ent_conseil_classe WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        row.map(conseil_de).ok_or_else(introuvable)
    }

    /// Direction (`bulletins.publier`) ou professeur principal de la classe.
    async fn eva_peut_decider(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        classe_id: Uuid,
    ) -> Result<bool, EntDbError> {
        if Role::parse(role).is_none_or(|r| !r.is_staff()) {
            return Ok(false);
        }
        Ok(self
            .a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
            .await?
            || self.eva_est_pp(etab, acteur, classe_id).await?)
    }

    /// `conseils.gerer` et (lecture des notes de l'établissement, ou enseigne dans la classe).
    async fn eva_peut_preparer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        classe_id: Uuid,
    ) -> Result<bool, EntDbError> {
        if Role::parse(role).is_none_or(|r| !r.is_staff())
            || !self
                .a_action_etablissement(etab, acteur, role, Action::ConseilsGerer)
                .await?
        {
            return Ok(false);
        }
        Ok(self
            .a_action_etablissement(etab, acteur, role, Action::NotesLire)
            .await?
            || self.eva_enseigne_classe(etab, acteur, classe_id).await?)
    }

    /// D-022 : élèves de la classe que l'acteur voit dans le dossier du conseil.
    /// `None` = la classe entière (direction, professeur principal, rôles à portée
    /// établissement) ; sinon seulement ces élèves (professeur d'un groupe de la classe).
    pub(crate) async fn eva_eleves_conseil(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        classe_id: Uuid,
    ) -> Result<Option<std::collections::HashSet<Uuid>>, EntDbError> {
        if self.eva_peut_decider(etab, acteur, role, classe_id).await? {
            return Ok(None);
        }
        self.eleves_autorises(etab, acteur, role, Action::NotesLire).await
    }

    // ───────────────────────────── Appréciations ─────────────────────────────

    /// EVA-05 / EVA-06 : appréciation de matière ou générale (mise à jour si elle existe).
    pub async fn saisir_appreciation_eva_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &AppreciationBody,
    ) -> Result<AppreciationDto, EntDbError> {
        Self::exiger_staff(role)?;
        let periode = self.eva_periode(etab, body.periode_id).await?;
        if !periode.statut.autorise_appreciations() {
            return Err(conflit("période fermée : appréciations verrouillées"));
        }
        self.exiger_eleve_existe(etab, body.eleve_id).await?;
        let texte = texte_borne(&body.texte, 1500)?;
        if texte.is_empty() {
            return Err(refus("appréciation vide"));
        }
        let classe = self
            .eva_classe_eleve(etab, body.eleve_id, periode.date_debut, periode.date_fin)
            .await?;
        match body.matiere_id {
            Some(matiere) => {
                let connue: Option<(i32,)> = sqlx::query_as(
                    "SELECT 1 FROM ent_matiere WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(matiere)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?;
                if connue.is_none() {
                    return Err(introuvable());
                }
                let etab_entier = self
                    .a_action_etablissement(etab, acteur, role, Action::NotesSaisir)
                    .await?;
                if !etab_entier {
                    // Professeur d'un service de cette matière dont l'effectif contient l'élève.
                    let (ok,): (bool,) = sqlx::query_as(
                        "SELECT EXISTS (
                           SELECT 1 FROM ent_service_enseignement s
                             JOIN ent_affectation a
                               ON a.etablissement_id = s.etablissement_id
                              AND ((s.classe_id IS NOT NULL AND a.classe_id = s.classe_id)
                                OR (s.groupe_id IS NOT NULL AND a.groupe_id = s.groupe_id))
                            WHERE s.etablissement_id = $1 AND s.prof_user_id = $2 AND s.matiere_id = $3
                              AND a.eleve_id = $4
                              AND a.date_debut <= $6 AND (a.date_fin IS NULL OR a.date_fin >= $5))",
                    )
                    .bind(etab)
                    .bind(acteur)
                    .bind(matiere)
                    .bind(body.eleve_id)
                    .bind(periode.date_debut)
                    .bind(periode.date_fin)
                    .fetch_one(self.pool())
                    .await
                    .map_err(db)?;
                    if !ok
                        || !self
                            .autoriser_eleve(etab, acteur, role, Action::NotesSaisir, body.eleve_id)
                            .await?
                    {
                        return Err(EntDbError::Forbidden);
                    }
                }
            }
            None => {
                let pp = match classe {
                    Some(c) => self.eva_est_pp(etab, acteur, c).await?,
                    None => false,
                };
                if !pp
                    && !self
                        .a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
                        .await?
                {
                    return Err(EntDbError::Forbidden);
                }
            }
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        if !statut_periode_tx(&mut tx, etab, body.periode_id).await?.autorise_appreciations() {
            return Err(conflit("période fermée : appréciations verrouillées"));
        }
        let (id,): (Uuid,) = if let Some(matiere) = body.matiere_id {
            sqlx::query_as(
                "INSERT INTO ent_note_appreciation (id, etablissement_id, periode_id, eleve_id, matiere_id, texte, auteur_user_id)
                 VALUES ($1,$2,$3,$4,$5,$6,$7)
                 ON CONFLICT (etablissement_id, periode_id, eleve_id, matiere_id) WHERE matiere_id IS NOT NULL
                 DO UPDATE SET texte = EXCLUDED.texte, auteur_user_id = EXCLUDED.auteur_user_id, updated_at = now()
                 RETURNING id",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(body.periode_id)
            .bind(body.eleve_id)
            .bind(matiere)
            .bind(&texte)
            .bind(acteur)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?
        } else {
            sqlx::query_as(
                "INSERT INTO ent_note_appreciation (id, etablissement_id, periode_id, eleve_id, matiere_id, texte, auteur_user_id)
                 VALUES ($1,$2,$3,$4,NULL,$5,$6)
                 ON CONFLICT (etablissement_id, periode_id, eleve_id) WHERE matiere_id IS NULL
                 DO UPDATE SET texte = EXCLUDED.texte, auteur_user_id = EXCLUDED.auteur_user_id, updated_at = now()
                 RETURNING id",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(body.periode_id)
            .bind(body.eleve_id)
            .bind(&texte)
            .bind(acteur)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?
        };
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: if body.matiere_id.is_some() {
                types::NOTE_APPRECIATION.to_string()
            } else {
                types_eva::APPRECIATION_GENERALE.to_string()
            },
            aggregate: AggregateKind::Note,
            aggregate_id: id,
            eleve_id: Some(body.eleve_id),
            // Texte jamais recopié dans l'événement.
            payload: json!({ "periode_id": body.periode_id, "matiere_id": body.matiere_id }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        // Alerte EVA-06 : matières avec moyenne mais sans appréciation.
        let mut manquantes = Vec::new();
        if body.matiere_id.is_none() {
            if let Some(c) = classe {
                let calc = self.eva_calculer_classe(etab, c, body.periode_id, false).await?;
                let avec: Vec<(Uuid,)> = sqlx::query_as(
                    "SELECT matiere_id FROM ent_note_appreciation
                      WHERE etablissement_id = $1 AND periode_id = $2 AND eleve_id = $3 AND matiere_id IS NOT NULL",
                )
                .bind(etab)
                .bind(body.periode_id)
                .bind(body.eleve_id)
                .fetch_all(self.pool())
                .await
                .map_err(db)?;
                for (m, lib, _, _) in &calc.matieres {
                    if calc.moyennes.contains_key(&(body.eleve_id, *m)) && !avec.iter().any(|(x,)| x == m) {
                        manquantes.push(lib.clone());
                    }
                }
            }
        }
        Ok(AppreciationDto {
            id,
            eleve_id: body.eleve_id,
            periode_id: body.periode_id,
            matiere_id: body.matiere_id,
            texte,
            matieres_sans_appreciation: manquantes,
        })
    }

    // ───────────────────────────── Conseil de classe ─────────────────────────────

    /// Planifie un conseil rattaché à une classe et une période (EVA-08).
    pub async fn creer_conseil_eva_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &CreerConseilBody,
    ) -> Result<ConseilDto, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::ConseilsGerer)
            .await?
            || !self.eva_peut_decider(etab, acteur, role, body.classe_id).await?
        {
            return Err(EntDbError::Forbidden);
        }
        let periode = self.eva_periode(etab, body.periode_id).await?;
        let lieu = texte_borne(&body.lieu, 200)?;
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        let ok: Option<(String,)> =
            sqlx::query_as("SELECT code FROM ent_classe WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(body.classe_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        let (code,) = ok.ok_or_else(introuvable)?;
        sqlx::query(
            "INSERT INTO ent_conseil_classe (id, etablissement_id, classe, classe_id, periode_libelle, periode_id,
                                             jour, heure, lieu, statut, created_by_user_id, president_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'planifie',$10,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(&code)
        .bind(body.classe_id)
        .bind(&periode.libelle)
        .bind(periode.id)
        .bind(body.jour)
        .bind(body.heure)
        .bind(&lieu)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::CONSEIL_CLASSE.to_string(),
            aggregate: AggregateKind::ConseilClasse,
            aggregate_id: id,
            eleve_id: None,
            payload: json!({ "classe_id": body.classe_id, "periode_id": periode.id, "jour": body.jour.to_string() }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        self.eva_conseil(etab, id).await
    }

    /// Conseils rattachés à une période (classes où l'acteur peut préparer).
    pub async fn conseils_eva_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        classe_id: Option<Uuid>,
    ) -> Result<Vec<ConseilDto>, EntDbError> {
        Self::exiger_staff(role)?;
        let rows: Vec<ConseilTuple> = sqlx::query_as(&format!(
            "SELECT {COLONNES_CONSEIL} FROM ent_conseil_classe
              WHERE etablissement_id = $1 AND periode_id IS NOT NULL AND classe_id IS NOT NULL
                AND ($2::uuid IS NULL OR classe_id = $2)
              ORDER BY jour DESC, heure DESC"
        ))
        .bind(etab)
        .bind(classe_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for c in rows.into_iter().map(conseil_de) {
            if let Some(cl) = c.classe_id {
                if self.eva_peut_preparer(etab, acteur, role, cl).await? {
                    out.push(c);
                }
            }
        }
        Ok(out)
    }

    /// EVA-08 : tableau de préparation calculé à la demande.
    pub async fn preparation_conseil_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        conseil_id: Uuid,
    ) -> Result<PreparationConseilDto, EntDbError> {
        Self::exiger_staff(role)?;
        let conseil = self.eva_conseil(etab, conseil_id).await?;
        let (Some(classe_id), Some(periode_id)) = (conseil.classe_id, conseil.periode_id) else {
            return Err(refus("conseil sans classe ou sans période de notes"));
        };
        if !self.eva_peut_preparer(etab, acteur, role, classe_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let peut_decider = self.eva_peut_decider(etab, acteur, role, classe_id).await?;
        let calc = self.eva_calculer_classe(etab, classe_id, periode_id, false).await?;
        let a = calc.regles.arrondi;
        // D-022 : un professeur de groupe ne voit que les élèves de son groupe.
        let visibles = self.eva_eleves_conseil(etab, acteur, role, classe_id).await?;
        let calc_eleves: Vec<(Uuid, String, String)> = calc
            .eleves
            .iter()
            .filter(|e| visibles.as_ref().is_none_or(|v| v.contains(&e.0)))
            .cloned()
            .collect();
        let ids: Vec<Uuid> = calc_eleves.iter().map(|e| e.0).collect();
        let (debut, fin) = (calc.periode.date_debut, calc.periode.date_fin);
        let apprs: Vec<(Uuid, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT eleve_id, matiere_id, texte FROM ent_note_appreciation
              WHERE etablissement_id = $1 AND periode_id = $2 AND eleve_id = ANY($3)",
        )
        .bind(etab)
        .bind(periode_id)
        .bind(&ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let absences: Vec<(Uuid, i64, i64)> = sqlx::query_as(
            "SELECT eleve_id, count(*)::bigint,
                    count(*) FILTER (WHERE statut NOT IN ('justifiee', 'annulee_present'))::bigint
               FROM ent_absence
              WHERE etablissement_id = $1 AND eleve_id = ANY($2) AND debut <= $4 AND fin >= $3
                AND statut <> 'annulee_present'
              GROUP BY eleve_id",
        )
        .bind(etab)
        .bind(&ids)
        .bind(debut)
        .bind(fin)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let retards: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT eleve_id, count(*)::bigint FROM ent_retard
              WHERE etablissement_id = $1 AND eleve_id = ANY($2) AND jour BETWEEN $3 AND $4
              GROUP BY eleve_id",
        )
        .bind(etab)
        .bind(&ids)
        .bind(debut)
        .bind(fin)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let sanctions: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT eleve_id, count(*)::bigint FROM ent_sanction
              WHERE etablissement_id = $1 AND eleve_id = ANY($2) AND jour BETWEEN $3 AND $4
                AND statut = 'active' AND effacee_at IS NULL
              GROUP BY eleve_id",
        )
        .bind(etab)
        .bind(&ids)
        .bind(debut)
        .bind(fin)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        type DossierTuple = (Uuid, String, Option<String>, Option<String>, Option<String>, Option<DateTime<Utc>>);
        let dossiers: Vec<DossierTuple> = sqlx::query_as(
            "SELECT eleve_id, appreciation, mention, decision_code, avis_orientation, decided_at
               FROM ent_conseil_classe_dossier WHERE etablissement_id = $1 AND conseil_id = $2",
        )
        .bind(etab)
        .bind(conseil_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let voeux = self.eva_voeux_avis(etab, &ids, Some(conseil_id)).await?;
        // Absences et retards : seulement pour les élèves que l'acteur suit en vie scolaire
        // (`absences.lire`) ; sanctions : `suivi.eleve`. Sinon « non communiqué ».
        let voit_absences = self.eleves_autorises(etab, acteur, role, Action::AbsencesLire).await?;
        let voit_suivi = self.eleves_autorises(etab, acteur, role, Action::SuiviEleve).await?;
        // D-022 (précision du 2026-11-07) : la portée au groupe vaut aussi au conseil. Un
        // professeur qui n'a qu'un groupe de la classe ne voit que les élèves de ce groupe.
        let voit_notes = self.eleves_autorises(etab, acteur, role, Action::NotesLire).await?;
        let dans = |s: &Option<std::collections::HashSet<Uuid>>, id: &Uuid| s.as_ref().is_none_or(|s| s.contains(id));
        let absences: HashMap<Uuid, (i64, i64)> =
            absences.into_iter().map(|(e, n, nj)| (e, (n, nj))).collect();
        let retards: HashMap<Uuid, i64> = retards.into_iter().collect();
        let sanctions: HashMap<Uuid, i64> = sanctions.into_iter().collect();
        let dossiers: HashMap<Uuid, DecisionConseilDto> = dossiers
            .into_iter()
            .map(|(e, appreciation, mention, decision_code, avis_orientation, decided_at)| {
                (
                    e,
                    DecisionConseilDto {
                        appreciation,
                        mention,
                        decision_code,
                        avis_orientation,
                        decided_at,
                    },
                )
            })
            .collect();
        let (mut total_manquantes, mut generales_manquantes) = (0u32, 0u32);
        let mut eleves = Vec::with_capacity(calc_eleves.len());
        for (id, nom, prenom) in &calc_eleves {
            if !dans(&voit_notes, id) {
                continue;
            }
            let manquantes: Vec<String> = calc
                .matieres
                .iter()
                .filter(|(m, _, _, _)| {
                    calc.moyennes.contains_key(&(*id, *m))
                        && !apprs.iter().any(|(e, mm, _)| e == id && *mm == Some(*m))
                })
                .map(|(_, lib, _, _)| lib.clone())
                .collect();
            total_manquantes += manquantes.len() as u32;
            let generale = apprs
                .iter()
                .find(|(e, m, _)| e == id && m.is_none())
                .map(|(_, _, t)| t.clone());
            if generale.is_none() {
                generales_manquantes += 1;
            }
            let (abs, abs_nj) = absences.get(id).copied().unwrap_or((0, 0));
            eleves.push(PreparationEleveDto {
                eleve_id: *id,
                nom: nom.clone(),
                prenom: prenom.clone(),
                moyenne_generale: calc.generales.get(id).map(|g| g.arrondie(a)),
                appreciations_manquantes: manquantes,
                appreciation_generale: generale,
                absences: dans(&voit_absences, id).then_some(abs),
                absences_non_justifiees: dans(&voit_absences, id).then_some(abs_nj),
                retards: dans(&voit_absences, id).then(|| retards.get(id).copied().unwrap_or(0)),
                sanctions: dans(&voit_suivi, id).then(|| sanctions.get(id).copied().unwrap_or(0)),
                decision: dossiers.get(id).cloned(),
                voeux: voeux.get(id).cloned().unwrap_or_default(),
            });
        }
        Ok(PreparationConseilDto {
            conseil,
            periode: Some(calc.periode.dto()),
            arrondi: a.as_str().to_string(),
            // Agrégat de la classe entière : non communiqué à un professeur de groupe.
            moyenne_classe: if visibles.is_none() {
                calc.stats_generale.map(|s| s.moyenne.arrondie(a))
            } else {
                None
            },
            appreciations_manquantes: total_manquantes,
            appreciations_generales_manquantes: generales_manquantes,
            peut_decider,
            eleves,
        })
    }

    /// Vœux des élèves et avis (du conseil donné, ou du plus récent conseil).
    async fn eva_voeux_avis(
        &self,
        etab: Uuid,
        eleves: &[Uuid],
        conseil: Option<Uuid>,
    ) -> Result<HashMap<Uuid, Vec<VoeuAvisDto>>, EntDbError> {
        type VoeuTuple = (Uuid, Uuid, i16, String, Option<Uuid>, Option<String>, Option<String>, Option<String>);
        let rows: Vec<VoeuTuple> = sqlx::query_as(
            "SELECT v.eleve_id, v.id, v.rang, f.libelle, a.id, a.avis, a.motivation, a.reponse_famille
               FROM ent_voeu_inscription v
               JOIN ent_filiere f ON f.id = v.filiere_id AND f.etablissement_id = v.etablissement_id
               LEFT JOIN LATERAL (
                    SELECT x.id, x.avis, x.motivation, x.reponse_famille FROM ent_orientation_avis x
                     WHERE x.etablissement_id = v.etablissement_id AND x.voeu_id = v.id
                       AND ($3::uuid IS NULL OR x.conseil_id = $3)
                     ORDER BY x.donne_le DESC LIMIT 1) a ON true
              WHERE v.etablissement_id = $1 AND v.eleve_id = ANY($2)
              ORDER BY v.eleve_id, v.annee_scolaire DESC, v.rang",
        )
        .bind(etab)
        .bind(eleves)
        .bind(conseil)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out: HashMap<Uuid, Vec<VoeuAvisDto>> = HashMap::new();
        for (eleve, voeu, rang, lib, avis_id, avis, motivation, reponse) in rows {
            out.entry(eleve).or_default().push(VoeuAvisDto {
                voeu_id: voeu,
                avis_id,
                rang,
                filiere_libelle: lib,
                avis,
                motivation: motivation.unwrap_or_default(),
                reponse_famille: reponse,
            });
        }
        Ok(out)
    }

    /// EVA-09 : appréciation du conseil, mention, décision et avis d'orientation d'un élève.
    pub async fn decider_conseil_eva_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        conseil_id: Uuid,
        body: &DecisionConseilBody,
    ) -> Result<DecisionConseilDto, EntDbError> {
        Self::exiger_staff(role)?;
        let conseil = self.eva_conseil(etab, conseil_id).await?;
        let (Some(classe_id), Some(periode_id)) = (conseil.classe_id, conseil.periode_id) else {
            return Err(refus("conseil sans classe ou sans période de notes"));
        };
        if conseil.statut == "annule" {
            return Err(conflit("conseil annulé"));
        }
        if !self.eva_peut_decider(etab, acteur, role, classe_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let periode = self.eva_periode(etab, periode_id).await?;
        let effectif = self
            .eva_effectif(etab, Some(classe_id), None, periode.date_debut, periode.date_fin)
            .await?;
        if !effectif.iter().any(|(id, _, _)| *id == body.eleve_id) {
            return Err(refus("élève hors de la classe du conseil"));
        }
        let choix = |v: &Option<String>, liste: &[&str], msg: &'static str| -> Result<Option<String>, EntDbError> {
            match v.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                None => Ok(None),
                Some(s) if liste.contains(&s) => Ok(Some(s.to_string())),
                Some(_) => Err(refus(msg)),
            }
        };
        // V0 (D11) : les anciens codes « avertissement_* » sont reclassés en mises en garde.
        let mention_norm = body.mention.as_deref().map(|m| match m.trim() {
            "avertissement_travail" => "mise_en_garde_travail".to_string(),
            "avertissement_conduite" => "mise_en_garde_conduite".to_string(),
            autre => autre.to_string(),
        });
        let mention = choix(&mention_norm, &MENTIONS, "mention inconnue")?;
        let decision = choix(&body.decision_code, &DECISIONS, "décision inconnue")?;
        let appreciation = texte_borne(&body.appreciation, 2000)?;
        let avis = match body.avis_orientation.as_deref() {
            Some(t) => Some(texte_borne(t, 1000)?).filter(|s| !s.is_empty()),
            None => None,
        };
        let mut tx = self.pool().begin().await.map_err(db)?;
        let (dossier_id, decided_at): (Uuid, DateTime<Utc>) = sqlx::query_as(
            "INSERT INTO ent_conseil_classe_dossier (id, etablissement_id, conseil_id, eleve_id, appreciation,
                                                     mention, decision_code, avis_orientation, decision_texte,
                                                     decided_at, decided_by_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'',now(),$9)
             ON CONFLICT (conseil_id, eleve_id) DO UPDATE
                SET appreciation = EXCLUDED.appreciation, mention = EXCLUDED.mention,
                    decision_code = EXCLUDED.decision_code, avis_orientation = EXCLUDED.avis_orientation,
                    decided_at = now(), decided_by_user_id = EXCLUDED.decided_by_user_id
              WHERE ent_conseil_classe_dossier.etablissement_id = EXCLUDED.etablissement_id
             RETURNING id, decided_at",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(conseil_id)
        .bind(body.eleve_id)
        .bind(&appreciation)
        .bind(&mention)
        .bind(&decision)
        .bind(&avis)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_eva::CONSEIL_DECISION.to_string(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: body.eleve_id,
            eleve_id: Some(body.eleve_id),
            payload: json!({
                "conseil_id": conseil_id,
                "dossier_id": dossier_id,
                "periode_id": periode_id,
                "decision_code": decision,
                "mention": mention,
            }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(DecisionConseilDto {
            appreciation,
            mention,
            decision_code: decision,
            avis_orientation: avis,
            decided_at: Some(decided_at),
        })
    }

    /// ORI-02 : avis du conseil sur un vœu.
    pub async fn avis_voeu_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        conseil_id: Uuid,
        body: &AvisVoeuBody,
    ) -> Result<VoeuAvisDto, EntDbError> {
        Self::exiger_staff(role)?;
        let conseil = self.eva_conseil(etab, conseil_id).await?;
        let (Some(classe_id), Some(periode_id)) = (conseil.classe_id, conseil.periode_id) else {
            return Err(refus("conseil sans classe ou sans période de notes"));
        };
        if !self.eva_peut_decider(etab, acteur, role, classe_id).await? {
            return Err(EntDbError::Forbidden);
        }
        if !AVIS.contains(&body.avis.as_str()) {
            return Err(refus("avis inconnu"));
        }
        let motivation = texte_borne(&body.motivation, 1000)?;
        let voeu: Option<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_voeu_inscription WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(body.voeu_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let (eleve_id,) = voeu.ok_or_else(introuvable)?;
        let periode = self.eva_periode(etab, periode_id).await?;
        let effectif = self
            .eva_effectif(etab, Some(classe_id), None, periode.date_debut, periode.date_fin)
            .await?;
        if !effectif.iter().any(|(id, _, _)| *id == eleve_id) {
            return Err(refus("élève hors de la classe du conseil"));
        }
        if conseil.statut == "annule" {
            return Err(conflit("conseil annulé"));
        }
        let (publie,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_bulletin
                             WHERE etablissement_id = $1 AND eleve_id = $2 AND periode_id = $3 AND statut = 'publie')",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(periode_id)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        if publie {
            return Err(conflit("bulletin publié : l'avis communiqué à la famille ne se modifie plus"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let (avis_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO ent_orientation_avis (id, etablissement_id, voeu_id, eleve_id, conseil_id, avis,
                                               motivation, donne_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT (etablissement_id, voeu_id, conseil_id) DO UPDATE
                SET avis = EXCLUDED.avis, motivation = EXCLUDED.motivation,
                    donne_par_user_id = EXCLUDED.donne_par_user_id, donne_le = now(),
                    reponse_famille = NULL, reponse_par_user_id = NULL, reponse_le = NULL
             RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(body.voeu_id)
        .bind(eleve_id)
        .bind(conseil_id)
        .bind(&body.avis)
        .bind(&motivation)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_eva::ORIENTATION_AVIS.to_string(),
            aggregate: AggregateKind::Voeu,
            aggregate_id: body.voeu_id,
            eleve_id: Some(eleve_id),
            payload: json!({ "conseil_id": conseil_id, "avis_id": avis_id, "avis": body.avis }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        let tous = self.eva_voeux_avis(etab, &[eleve_id], Some(conseil_id)).await?;
        tous.get(&eleve_id)
            .and_then(|v| v.iter().find(|x| x.voeu_id == body.voeu_id).cloned())
            .ok_or_else(introuvable)
    }

    /// Orientation vue par la famille ou l'élève : avis visibles **après publication du
    /// bulletin** de la période du conseil (staff : tous les avis).
    pub async fn orientation_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<VoeuAvisDto>, EntDbError> {
        let staff = Role::parse(role).is_some_and(|r| r.is_staff());
        let autorise = if staff {
            self.autoriser_eleve(etab, acteur, role, Action::SuiviEleve, eleve_id).await?
                || self.autoriser_eleve(etab, acteur, role, Action::NotesLire, eleve_id).await?
        } else {
            self.actor_can_read_notes(etab, acteur, role, eleve_id).await?
        };
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        let mut voeux = self
            .eva_voeux_avis(etab, &[eleve_id], None)
            .await?
            .remove(&eleve_id)
            .unwrap_or_default();
        if !staff {
            let visibles: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT a.id FROM ent_orientation_avis a
                   JOIN ent_conseil_classe c ON c.id = a.conseil_id AND c.etablissement_id = a.etablissement_id
                  WHERE a.etablissement_id = $1 AND a.eleve_id = $2
                    AND EXISTS (SELECT 1 FROM ent_bulletin b
                                 WHERE b.etablissement_id = a.etablissement_id AND b.eleve_id = a.eleve_id
                                   AND b.periode_id = c.periode_id AND b.statut = 'publie')",
            )
            .bind(etab)
            .bind(eleve_id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
            for v in &mut voeux {
                if !v.avis_id.is_some_and(|id| visibles.iter().any(|(x,)| *x == id)) {
                    v.avis_id = None;
                    v.avis = None;
                    v.motivation = String::new();
                    v.reponse_famille = None;
                }
            }
        }
        Ok(voeux)
    }

    /// ORI-03 : réponse d'un responsable (accord ou désaccord) à un avis publié.
    pub async fn repondre_avis_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        avis_id: Uuid,
        reponse: &str,
    ) -> Result<(), EntDbError> {
        if Role::parse(role) != Some(Role::Parent) {
            return Err(EntDbError::Forbidden);
        }
        if !matches!(reponse, "accord" | "desaccord") {
            return Err(refus("réponse attendue : accord ou desaccord"));
        }
        let row: Option<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT a.eleve_id, a.voeu_id FROM ent_orientation_avis a
               JOIN ent_conseil_classe c ON c.id = a.conseil_id AND c.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.id = $2
                AND EXISTS (SELECT 1 FROM ent_bulletin b
                             WHERE b.etablissement_id = a.etablissement_id AND b.eleve_id = a.eleve_id
                               AND b.periode_id = c.periode_id AND b.statut = 'publie')",
        )
        .bind(etab)
        .bind(avis_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let (eleve_id, voeu_id) = row.ok_or_else(introuvable)?;
        if self.lien_parent_eleve(etab, acteur, eleve_id).await?.is_none()
            || !self.actor_can_read_notes(etab, acteur, role, eleve_id).await?
        {
            return Err(introuvable());
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let maj = sqlx::query(
            "UPDATE ent_orientation_avis SET reponse_famille = $3, reponse_par_user_id = $4, reponse_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND reponse_famille IS NULL",
        )
        .bind(etab)
        .bind(avis_id)
        .bind(reponse)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if maj.rows_affected() == 0 {
            return Err(conflit("réponse déjà transmise"));
        }
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_eva::ORIENTATION_REPONSE.to_string(),
            aggregate: AggregateKind::Voeu,
            aggregate_id: voeu_id,
            eleve_id: Some(eleve_id),
            payload: json!({ "avis_id": avis_id, "reponse": reponse }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────────── Bilan de fin de cycle ─────────────────────────────

    async fn eva_bilan(&self, etab: Uuid, id: Uuid) -> Result<BilanCycleDto, EntDbError> {
        type BilanTuple = (Uuid, Uuid, i16, String, String, Option<DateTime<Utc>>);
        let row: Option<BilanTuple> = sqlx::query_as(
            "SELECT id, eleve_id, cycle, statut, synthese, valide_le FROM ent_bilan_cycle
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let (id, eleve_id, cycle, statut, synthese, valide_le) = row.ok_or_else(introuvable)?;
        let comps: Vec<(String, i16)> = sqlx::query_as(
            "SELECT composante, niveau FROM ent_bilan_cycle_composante
              WHERE etablissement_id = $1 AND bilan_id = $2 ORDER BY composante",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(BilanCycleDto {
            id,
            eleve_id,
            cycle,
            statut,
            synthese,
            composantes: comps
                .into_iter()
                .map(|(composante, niveau)| ComposanteNiveauDto { composante, niveau })
                .collect(),
            valide_le,
        })
    }

    async fn eva_annee_courante(&self, etab: Uuid) -> Result<Uuid, EntDbError> {
        Ok(self.referentiel_annee_courante_assuree(etab).await?.id)
    }

    /// EVA-11 : saisie (brouillon) du bilan de fin de cycle de l'année courante.
    pub async fn saisir_bilan_cycle_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &BilanCycleBody,
    ) -> Result<BilanCycleDto, EntDbError> {
        Self::exiger_staff(role)?;
        self.exiger_eleve_existe(etab, body.eleve_id).await?;
        if !self
            .autoriser_eleve(etab, acteur, role, Action::CompetencesEvaluer, body.eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if !matches!(body.cycle, 2..=4) {
            return Err(refus("cycle attendu : 2, 3 ou 4"));
        }
        let mut vus = std::collections::HashSet::new();
        for c in &body.composantes {
            if !composante_connue(&c.composante) || !(1..=4).contains(&c.niveau) || !vus.insert(&c.composante) {
                return Err(refus("composante du socle ou niveau invalide"));
            }
        }
        let synthese = texte_borne(&body.synthese, 2000)?;
        let annee = self.eva_annee_courante(etab).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let existant: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id, statut FROM ent_bilan_cycle
              WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3 AND cycle = $4 FOR UPDATE",
        )
        .bind(etab)
        .bind(body.eleve_id)
        .bind(annee)
        .bind(body.cycle)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let id = match existant {
            Some((_, s)) if s == "valide" => return Err(conflit("bilan de cycle déjà validé")),
            Some((id, _)) => {
                sqlx::query(
                    "UPDATE ent_bilan_cycle SET synthese = $3, saisi_par_user_id = $4, updated_at = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(&synthese)
                .bind(acteur)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                id
            }
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_bilan_cycle (id, etablissement_id, eleve_id, annee_id, cycle, synthese, saisi_par_user_id)
                     VALUES ($1,$2,$3,$4,$5,$6,$7)",
                )
                .bind(id)
                .bind(etab)
                .bind(body.eleve_id)
                .bind(annee)
                .bind(body.cycle)
                .bind(&synthese)
                .bind(acteur)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                id
            }
        };
        for c in &body.composantes {
            sqlx::query(
                "INSERT INTO ent_bilan_cycle_composante (etablissement_id, bilan_id, composante, niveau)
                 VALUES ($1,$2,$3,$4)
                 ON CONFLICT (bilan_id, composante) DO UPDATE SET niveau = EXCLUDED.niveau
                  WHERE ent_bilan_cycle_composante.etablissement_id = EXCLUDED.etablissement_id",
            )
            .bind(etab)
            .bind(id)
            .bind(&c.composante)
            .bind(c.niveau)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_eva::BILAN_CYCLE_SAISI.to_string(),
            aggregate: AggregateKind::Competence,
            aggregate_id: id,
            eleve_id: Some(body.eleve_id),
            payload: json!({ "cycle": body.cycle }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        self.eva_bilan(etab, id).await
    }

    /// Validation du bilan (8 composantes renseignées) : professeur principal ou direction.
    pub async fn valider_bilan_cycle_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        bilan_id: Uuid,
    ) -> Result<BilanCycleDto, EntDbError> {
        Self::exiger_staff(role)?;
        let bilan = self.eva_bilan(etab, bilan_id).await?;
        let classe = self
            .eva_classe_eleve(etab, bilan.eleve_id, domain_eleve::today_utc(), domain_eleve::today_utc())
            .await?;
        let pp = match classe {
            Some(c) => self.eva_est_pp(etab, acteur, c).await?,
            None => false,
        };
        if !pp
            && !self
                .a_action_etablissement(etab, acteur, role, Action::BulletinsPublier)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if bilan.statut == "valide" {
            return Ok(bilan);
        }
        if bilan.composantes.len() != 8 {
            return Err(refus("les 8 composantes du socle doivent être positionnées"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_bilan_cycle SET statut = 'valide', valide_par_user_id = $3, valide_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(bilan_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_eva::BILAN_CYCLE_VALIDE.to_string(),
            aggregate: AggregateKind::Competence,
            aggregate_id: bilan_id,
            eleve_id: Some(bilan.eleve_id),
            payload: json!({ "cycle": bilan.cycle }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        self.eva_bilan(etab, bilan_id).await
    }

    /// Bilans de cycle d'un élève (lecture des notes ; familles : validés seulement).
    pub async fn bilans_cycle_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<BilanCycleDto>, EntDbError> {
        if !self.actor_can_read_notes(etab, acteur, role, eleve_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let staff = Role::parse(role).is_some_and(|r| r.is_staff());
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_bilan_cycle
              WHERE etablissement_id = $1 AND eleve_id = $2 AND ($3 OR statut = 'valide')
              ORDER BY cycle, updated_at",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(staff)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (id,) in ids {
            out.push(self.eva_bilan(etab, id).await?);
        }
        Ok(out)
    }
}
