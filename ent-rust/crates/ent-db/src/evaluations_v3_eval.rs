//! Socle 21, lot V3 : créer, régler, annuler et **publier** une évaluation depuis la grille.
//!
//! * Création / réglage : titre, date, barème, coefficient, comptage, public (restriction), sous-matière,
//!   mode de publication. Barème, coefficient, comptage et date ne changent plus, une fois des résultats
//!   saisis ou la période arrêtée, que par **correction motivée** (M4) ; après l'arrêt, seule la direction.
//! * Publication : maintenant, programmée (heure de l'établissement), « à l'arrêt », retrait (24 h par le
//!   professeur, au-delà par la direction), « en cours de correction » ; **délai maximal** d'établissement
//!   (§10.1) : publication automatique au terme, sauf « en cours de correction », avec un rappel avant.
//! * Chaque publication est inscrite au journal des publications (ajout seul) ; les événements sont neutres.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_events::{AggregateKind, MetierEventDraft};
use ent_types::evaluations_v3::{
    AnnulerEvaluationV3Body, CreerEvaluationV3Body, EvaluationColonneDto, ModifierEvaluationV3Body,
    PublierV3Body,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus};
use crate::evaluations_v3::types_v3;
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Nombre maximal d'élèves dans un public restreint (F8) : un cours n'a pas plus d'élèves.
const MAX_RESTREINTE: usize = 300;

const COMPTAGES: [&str; 4] = ["normal", "si_ameliore", "ne_compte_pas", "bonus"];

/// Un bilan de la publication automatique.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BilanPublications {
    pub programmees: usize,
    pub delai_max: usize,
    pub rappels: usize,
}

impl PgEnt {
    /// Recharge une colonne telle que la grille la montre (après une écriture).
    async fn eva_v3_colonne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        evaluation_id: Uuid,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<EvaluationColonneDto, EntDbError> {
        let (cours_id, date_eval): (Uuid, NaiveDate) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let r: Option<(Uuid, NaiveDate)> = sqlx::query_as(
                "SELECT cours_id, date_eval FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(evaluation_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            r.ok_or_else(introuvable)?
        };
        let periodes = self
            .eva_v3_periodes_for_actor(etab, acteur, role, cours_id, jour)
            .await?;
        let periode = periodes
            .iter()
            .find(|p| p.date_debut <= date_eval && date_eval <= p.date_fin)
            .ok_or(refus(
                "la date de l'évaluation ne tombe dans aucune période de ce cours",
            ))?;
        let g = self
            .eva_v3_grille_for_actor(etab, acteur, role, cours_id, periode.id, jour, maintenant)
            .await?;
        g.evaluations
            .into_iter()
            .find(|e| e.id == evaluation_id)
            .ok_or_else(introuvable)
    }

    // ───────────────────────── Créer ─────────────────────────

    /// Crée une évaluation dans un cours (EVA-112), à publication explicite par défaut.
    pub async fn eva_v3_creer_evaluation_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
        body: &CreerEvaluationV3Body,
    ) -> Result<EvaluationColonneDto, EntDbError> {
        let acces = self
            .eva_v3_acces(etab, acteur, role, body.cours_id, jour)
            .await?;
        if !acces.saisie {
            return Err(EntDbError::Forbidden);
        }
        let titre = body.titre.trim();
        if titre.is_empty() || titre.chars().count() > 200 {
            return Err(refus("le titre est obligatoire (200 caractères au plus)"));
        }
        if body.bareme_m <= 0 || body.bareme_m > 1_000_000 {
            return Err(refus("barème invalide (entre 0,001 et 1000 points)"));
        }
        if !(0..=100_000).contains(&body.coef_m) {
            return Err(refus("coefficient invalide (entre 0 et 100)"));
        }
        let comptage = body.comptage.as_deref().unwrap_or("normal");
        if !COMPTAGES.contains(&comptage) {
            return Err(refus("comptage inconnu"));
        }
        let publication = body.publication.as_deref().unwrap_or("explicite");
        match publication {
            "explicite" | "a_l_arret" => {}
            "programmee" => {
                if body.publication_prevue_le.is_none_or(|p| p <= maintenant) {
                    return Err(refus("la publication programmée doit être dans le futur"));
                }
            }
            _ => return Err(refus("mode de publication inconnu")),
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let annee: (Uuid, NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT c.annee_id, y.date_debut, y.date_fin
               FROM ent_cours c JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = $2",
        )
        .bind(etab)
        .bind(body.cours_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if body.date_eval < annee.1 || body.date_eval > annee.2 {
            return Err(refus(
                "la date de l'évaluation est hors de l'année scolaire du cours",
            ));
        }
        // Public à la date et état des périodes : refus si tous les élèves sont arrêtés (CE-47).
        let public: Vec<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT pu.eleve_id, pe.etat
               FROM ent_public_cours_jour($1, $2, $3) pu
               LEFT JOIN LATERAL eva_periode_eleve_jour($1, pu.eleve_id, $2) pe ON true",
        )
        .bind(etab)
        .bind(body.date_eval)
        .bind(body.cours_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if !public.is_empty() && public.iter().all(|(_, e)| e.as_deref() != Some("ouverte")) {
            return Err(refus(
                "la période est arrêtée pour tous les élèves de ce cours à cette date : demandez à la direction",
            ));
        }
        if body.restreinte_a.len() > MAX_RESTREINTE {
            return Err(refus("un public restreint compte au plus 300 élèves"));
        }
        for v in &body.restreinte_a {
            if !public.iter().any(|(e, _)| e == v) {
                return Err(refus(
                    "un élève visé n'est pas dans le public du cours à cette date",
                ));
            }
        }
        let id = Uuid::new_v4();
        let ins = sqlx::query(
            "INSERT INTO eva_evaluation (id, etablissement_id, annee_id, cours_id, date_eval, titre, mode, bareme, coef,
                                         comptage, hors_bareme_autorise, matiere_id, etat, publication,
                                         publication_prevue_le, annoncee, cree_par)
             VALUES ($1, $2, $3, $4, $5, $6, 'note', $7::bigint / 1000.0, $8::bigint / 1000.0, $9, $10, $11,
                     'ouverte', $12, $13, $14, $15)",
        )
        .bind(id)
        .bind(etab)
        .bind(annee.0)
        .bind(body.cours_id)
        .bind(body.date_eval)
        .bind(titre)
        .bind(body.bareme_m)
        .bind(body.coef_m)
        .bind(comptage)
        .bind(body.hors_bareme_autorise)
        .bind(body.sous_matiere_id)
        .bind(publication)
        .bind(if publication == "programmee" { body.publication_prevue_le } else { None })
        .bind(body.annoncee)
        .bind(acteur)
        .execute(&mut *tx)
        .await;
        if let Err(e) = ins {
            return Err(map_garde(e));
        }
        for v in &body.restreinte_a {
            sqlx::query(
                "INSERT INTO eva_evaluation_restriction (evaluation_id, eleve_id, etablissement_id, motif_code)
                 VALUES ($1, $2, $3, 'autre') ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(v)
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        if publication != "explicite" {
            journal_publication(
                &mut tx,
                etab,
                id,
                if publication == "programmee" {
                    "programmee"
                } else {
                    "definie_a_l_arret"
                },
                body.publication_prevue_le
                    .filter(|_| publication == "programmee"),
                None,
                Some(acteur),
            )
            .await?;
        }
        publier_evt(&mut tx, etab, types_v3::EVALUATION_CREEE, id, acteur).await?;
        tx.commit().await.map_err(db)?;
        self.eva_v3_colonne(etab, acteur, role, id, jour, maintenant)
            .await
    }

    // ───────────────────────── Régler ─────────────────────────

    /// Modifie une évaluation (EVA-118, 120, 121). Version attendue (conflit si périmée) ; motif exigé quand
    /// l'évaluation porte déjà des résultats ; après l'arrêt d'une période de son public, seule la direction.
    pub async fn eva_v3_modifier_evaluation_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
        evaluation_id: Uuid,
        body: &ModifierEvaluationV3Body,
    ) -> Result<EvaluationColonneDto, EntDbError> {
        let cours_id = self.eva_v3_cours_de(etab, evaluation_id).await?;
        let acces = self
            .eva_v3_acces(etab, acteur, role, cours_id, jour)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // F8 (course barème / saisie) : l'évaluation est verrouillée DANS UNE REQUÊTE À PART, puis relue. Dans une seule
        // requête, les sous-requêtes (« existe-t-il des notes ? ») gardent l'instantané pris AVANT l'attente du verrou et
        // ne voient pas la note qu'une écriture concurrente vient de valider.
        sqlx::query("SELECT 1 FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(evaluation_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        let ev = sqlx::query(
            "SELECT e.version, e.etat, e.cree_par, e.date_eval, e.publiee_le, c.annee_id,
                    (e.bareme * 1000)::bigint AS bareme_m,
                    EXISTS (SELECT 1 FROM eva_resultat r WHERE r.etablissement_id = e.etablissement_id
                                                          AND r.evaluation_id = e.id) AS a_des_resultats,
                    EXISTS (SELECT 1 FROM eva_resultat r WHERE r.etablissement_id = e.etablissement_id
                                                          AND r.evaluation_id = e.id AND r.statut = 'note') AS a_des_notes
               FROM eva_evaluation e JOIN ent_cours c ON c.id = e.cours_id AND c.etablissement_id = e.etablissement_id
              WHERE e.etablissement_id = $1 AND e.id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let version: i32 = ev.try_get("version").map_err(db)?;
        let etat: String = ev.try_get("etat").map_err(db)?;
        let cree_par: Option<Uuid> = ev.try_get("cree_par").map_err(db)?;
        let date_actuelle: NaiveDate = ev.try_get("date_eval").map_err(db)?;
        let annee_id: Uuid = ev.try_get("annee_id").map_err(db)?;
        let a_des_resultats: bool = ev.try_get("a_des_resultats").map_err(db)?;
        let a_des_notes: bool = ev.try_get("a_des_notes").map_err(db)?;
        let bareme_m: i64 = ev.try_get("bareme_m").map_err(db)?;
        // Élèves du public dont la période est arrêtée à la date de l'évaluation.
        let arretes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_public_cours_jour($1, $2, $3) pu
               CROSS JOIN LATERAL eva_periode_eleve_jour($1, pu.eleve_id, $2) pe
              WHERE pe.etat <> 'ouverte'",
        )
        .bind(etab)
        .bind(date_actuelle)
        .bind(cours_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        // CE-44 (F1) : un suppléant ne modifie que ses propres évaluations. M3 : en période OUVERTE, l'évaluation
        // appartient au professeur ; un porteur de `notes.corriger` sans droit de saisie n'y touche pas (ni barème, ni
        // coefficient, ni comptage, ni titre). Après l'arrêt : correction possible, avec motif écrit, journalisée.
        let proprietaire = acces.saisie && (cree_par == Some(acteur) || !acces.suppleant);
        if !proprietaire && !(acces.corriger && arretes > 0) {
            return Err(EntDbError::Forbidden);
        }
        if !proprietaire
            && body.motif.as_deref().map(str::trim).is_none_or(|m| m.chars().count() < 3)
        {
            return Err(refus(
                "motif écrit obligatoire (3 caractères au moins) : correction après l'arrêt de la période",
            ));
        }
        if body.motif.as_deref().is_some_and(|m| m.trim().chars().count() > 500) {
            return Err(refus("motif trop long (500 caractères au plus)"));
        }
        if version != body.version_attendue {
            return Err(conflit("l'évaluation a changé depuis votre lecture"));
        }
        if etat != "ouverte" {
            return Err(refus("une évaluation annulée ne se modifie pas"));
        }
        let titre = body.titre.as_deref().map(str::trim);
        if titre.is_some_and(|t| t.is_empty() || t.chars().count() > 200) {
            return Err(refus("le titre est obligatoire (200 caractères au plus)"));
        }
        if body.bareme_m.is_some_and(|b| b <= 0 || b > 1_000_000) {
            return Err(refus("barème invalide (entre 0,001 et 1000 points)"));
        }
        if body.coef_m.is_some_and(|c| !(0..=100_000).contains(&c)) {
            return Err(refus("coefficient invalide (entre 0 et 100)"));
        }
        if body
            .comptage
            .as_deref()
            .is_some_and(|c| !COMPTAGES.contains(&c))
        {
            return Err(refus("comptage inconnu"));
        }
        let touche_calcul = body.bareme_m.is_some()
            || body.coef_m.is_some()
            || body.comptage.is_some()
            || body.hors_bareme_autorise.is_some()
            || body.date_eval.is_some_and(|d| d != date_actuelle);
        if touche_calcul {
            // Une période arrêtée pour une partie du public : seule la direction corrige (R-V10).
            if arretes > 0 && !acces.corriger {
                return Err(EntDbError::Forbidden);
            }
            let motif = body
                .motif
                .as_deref()
                .map(str::trim)
                .filter(|m| !m.is_empty());
            if (a_des_resultats || arretes > 0) && motif.is_none_or(|m| m.chars().count() < 3) {
                return Err(refus(
                    "motif écrit obligatoire : l'évaluation porte déjà des résultats ou la période est arrêtée",
                ));
            }
            // CONVERSION DES NOTES AU CHANGEMENT DE BARÈME (limite §5 de V3, tranchée en V3b) : REFUSÉ dès qu'une note
            // existe. Changer un barème sous des notes saisies change leur sens (14 sur 10 n'est pas 14 sur 20) et une
            // conversion proportionnelle réécrirait en masse des notes sans que la famille, déjà publiée, le sache ;
            // elle demande un aperçu et une confirmation que ce lot ne livre pas. Marche à suivre : annuler
            // l'évaluation (historique conservé) et la recréer au bon barème, ou corriger les notes une à une.
            if body.bareme_m.is_some_and(|b| b != bareme_m) && a_des_notes {
                return Err(refus(
                    "le barème ne change pas quand des notes sont saisies (14 sur 10 n'est pas 14 sur 20) : annulez l'évaluation et recréez-la au bon barème, ou corrigez les notes une à une",
                ));
            }
            if let Some(d) = body.date_eval {
                if d != date_actuelle {
                    let (debut, fin): (NaiveDate, NaiveDate) = sqlx::query_as(
                        "SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(annee_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                    if d < debut || d > fin {
                        return Err(refus(
                            "la date de l'évaluation est hors de l'année scolaire du cours",
                        ));
                    }
                    // Aucun élève ayant un résultat ne doit sortir du public en changeant la date.
                    let orphelins: i64 = sqlx::query_scalar(
                        "SELECT count(*) FROM eva_resultat r
                          WHERE r.etablissement_id = $1 AND r.evaluation_id = $2
                            AND NOT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $3, $4) pu WHERE pu.eleve_id = r.eleve_id)",
                    )
                    .bind(etab)
                    .bind(evaluation_id)
                    .bind(d)
                    .bind(cours_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                    if orphelins > 0 {
                        return Err(refus(
                            "à cette date, des élèves qui ont une note ne sont plus dans le public du cours",
                        ));
                    }
                }
            }
        }
        let upd = sqlx::query(
            "UPDATE eva_evaluation SET
                titre = COALESCE($3, titre),
                date_eval = COALESCE($4, date_eval),
                bareme = COALESCE($5::bigint / 1000.0, bareme),
                coef = COALESCE($6::bigint / 1000.0, coef),
                comptage = COALESCE($7, comptage),
                hors_bareme_autorise = COALESCE($8, hors_bareme_autorise),
                annoncee = COALESCE($9, annoncee),
                modifie_par = $10,
                motif_correction = $11
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .bind(titre)
        .bind(body.date_eval)
        .bind(body.bareme_m)
        .bind(body.coef_m)
        .bind(body.comptage.as_deref())
        .bind(body.hors_bareme_autorise)
        .bind(body.annoncee)
        .bind(acteur)
        .bind(
            body.motif
                .as_deref()
                .map(str::trim)
                .filter(|m| !m.is_empty()),
        )
        .execute(&mut *tx)
        .await;
        if let Err(e) = upd {
            return Err(map_garde(e));
        }
        publier_evt(
            &mut tx,
            etab,
            types_v3::EVALUATION_MODIFIEE,
            evaluation_id,
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.eva_v3_colonne(etab, acteur, role, evaluation_id, jour, maintenant)
            .await
    }

    /// Annule une évaluation (EVA-123) : motif obligatoire ; après publication, la direction seule.
    pub async fn eva_v3_annuler_evaluation_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
        evaluation_id: Uuid,
        body: &AnnulerEvaluationV3Body,
    ) -> Result<EvaluationColonneDto, EntDbError> {
        let motif = body.motif.trim();
        if motif.chars().count() < 3 || motif.chars().count() > 500 {
            return Err(refus("motif écrit obligatoire (3 à 500 caractères)"));
        }
        let cours_id = self.eva_v3_cours_de(etab, evaluation_id).await?;
        let acces = self
            .eva_v3_acces(etab, acteur, role, cours_id, jour)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = sqlx::query(
            "SELECT version, etat, cree_par, publiee_le, date_eval FROM eva_evaluation
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let version: i32 = ev.try_get("version").map_err(db)?;
        let etat: String = ev.try_get("etat").map_err(db)?;
        let cree_par: Option<Uuid> = ev.try_get("cree_par").map_err(db)?;
        let publiee: Option<DateTime<Utc>> = ev.try_get("publiee_le").map_err(db)?;
        let date_eval: NaiveDate = ev.try_get("date_eval").map_err(db)?;
        let arretes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_public_cours_jour($1, $2, $3) pu
               CROSS JOIN LATERAL eva_periode_eleve_jour($1, pu.eleve_id, $2) pe
              WHERE pe.etat <> 'ouverte'",
        )
        .bind(etab)
        .bind(date_eval)
        .bind(cours_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        // M3 : en période OUVERTE, l'évaluation appartient au professeur (titulaire, co-enseignant, ou suppléant pour
        // SES évaluations, CE-44) : un porteur de `notes.corriger` sans droit de saisie n'annule pas ; une évaluation
        // PUBLIÉE se retire d'abord (le professeur dans les 24 h, ensuite la direction). Après l'arrêt : la direction
        // (`notes.corriger`) annule, avec le motif écrit exigé plus haut, journalisé.
        let proprietaire = acces.saisie && (cree_par == Some(acteur) || !acces.suppleant);
        if arretes > 0 {
            if !acces.corriger {
                return Err(EntDbError::Forbidden);
            }
        } else {
            if !proprietaire {
                return Err(EntDbError::Forbidden);
            }
            if publiee.is_some() {
                return Err(refus(
                    "évaluation publiée : retirez d'abord la publication (le professeur dans les 24 h, ensuite la direction), puis annulez",
                ));
            }
        }
        if version != body.version_attendue {
            return Err(conflit("l'évaluation a changé depuis votre lecture"));
        }
        if etat == "annulee" {
            return Err(refus("évaluation déjà annulée"));
        }
        sqlx::query(
            "UPDATE eva_evaluation SET etat = 'annulee', motif_annulation = $3, modifie_par = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .bind(motif)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier_evt(
            &mut tx,
            etab,
            types_v3::EVALUATION_ANNULEE,
            evaluation_id,
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.eva_v3_colonne(etab, acteur, role, evaluation_id, jour, maintenant)
            .await
    }

    /// « En cours de correction » : suspend la publication automatique au délai maximal (§10.1).
    pub async fn eva_v3_en_cours_correction_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
        evaluation_id: Uuid,
        en_cours: bool,
    ) -> Result<EvaluationColonneDto, EntDbError> {
        let cours_id = self.eva_v3_cours_de(etab, evaluation_id).await?;
        let acces = self
            .eva_v3_acces(etab, acteur, role, cours_id, jour)
            .await?;
        if !acces.saisie {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // F1 (CE-44) : le suppléant saisit les notes du titulaire, mais « en cours de correction » comme la
        // modification de l'évaluation restent à son auteur ou au titulaire.
        let cree_par: Option<Uuid> = sqlx::query_scalar(
            "SELECT cree_par FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if acces.suppleant && cree_par != Some(acteur) {
            return Err(EntDbError::Forbidden);
        }
        let n = sqlx::query(
            "UPDATE eva_evaluation SET en_cours_de_correction = $3, modifie_par = $4
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'ouverte' AND publiee_le IS NULL",
        )
        .bind(etab)
        .bind(evaluation_id)
        .bind(en_cours)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(refus(
                "cette évaluation est déjà publiée ou n'est pas ouverte",
            ));
        }
        tx.commit().await.map_err(db)?;
        self.eva_v3_colonne(etab, acteur, role, evaluation_id, jour, maintenant)
            .await
    }

    pub(crate) async fn eva_v3_cours_de(
        &self,
        etab: Uuid,
        evaluation_id: Uuid,
    ) -> Result<Uuid, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Option<Uuid> = sqlx::query_scalar(
            "SELECT cours_id FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        r.ok_or_else(introuvable)
    }

    // ───────────────────────── Publier ─────────────────────────

    /// Publication d'une évaluation : `maintenant`, `programmer` (heure de l'établissement), `a_l_arret`,
    /// `annuler_programmation`, `retirer` (motif ; 24 h par le professeur, ensuite la direction).
    pub async fn eva_v3_publier_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
        evaluation_id: Uuid,
        body: &PublierV3Body,
    ) -> Result<EvaluationColonneDto, EntDbError> {
        let cours_id = self.eva_v3_cours_de(etab, evaluation_id).await?;
        let acces = self
            .eva_v3_acces(etab, acteur, role, cours_id, jour)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = sqlx::query(
            "SELECT etat, publiee_le, cree_par, premiere_publiee_le FROM eva_evaluation
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let etat: String = ev.try_get("etat").map_err(db)?;
        let publiee: Option<DateTime<Utc>> = ev.try_get("publiee_le").map_err(db)?;
        let cree_par: Option<Uuid> = ev.try_get("cree_par").map_err(db)?;
        let premiere: Option<DateTime<Utc>> = ev.try_get("premiere_publiee_le").map_err(db)?;
        let peut_publier =
            acces.saisie && (cree_par == Some(acteur) || !acces.suppleant || acces.corriger);
        match body.action.as_str() {
            "retirer" => {
                let Some(p) = publiee else {
                    return Err(refus("cette évaluation n'est pas publiée"));
                };
                // F4 : les 24 h du professeur courent depuis la PREMIÈRE publication : retirer puis republier ne
                // relance pas le délai.
                let dans_les_24h =
                    maintenant.signed_duration_since(premiere.unwrap_or(p)) <= Duration::hours(24);
                if !(acces.corriger || (peut_publier && dans_les_24h)) {
                    return Err(EntDbError::Forbidden);
                }
                let motif = body.motif.as_deref().map(str::trim).unwrap_or("");
                if motif.chars().count() < 3 || motif.chars().count() > 500 {
                    return Err(refus(
                        "motif écrit obligatoire pour retirer une publication (3 à 500 caractères)",
                    ));
                }
                sqlx::query(
                    "UPDATE eva_evaluation SET publiee_le = NULL, publiee_par = NULL, retiree_le = $3, retiree_par = $4,
                            publication = 'explicite', publication_prevue_le = NULL, modifie_par = $4
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(evaluation_id)
                .bind(maintenant)
                .bind(acteur)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                journal_publication(
                    &mut tx,
                    etab,
                    evaluation_id,
                    "retiree",
                    None,
                    Some(motif),
                    Some(acteur),
                )
                .await?;
                publier_evt(
                    &mut tx,
                    etab,
                    types_v3::PUBLICATION_RETIREE,
                    evaluation_id,
                    acteur,
                )
                .await?;
            }
            action => {
                if !peut_publier {
                    return Err(EntDbError::Forbidden);
                }
                if etat != "ouverte" {
                    return Err(refus("seule une évaluation ouverte se publie"));
                }
                if publiee.is_some() && action != "maintenant" {
                    return Err(refus(
                        "cette évaluation est déjà publiée : retirez d'abord sa publication",
                    ));
                }
                match action {
                    "maintenant" => {
                        if publiee.is_none() {
                            sqlx::query(
                                "UPDATE eva_evaluation SET publiee_le = $3, publiee_par = $4, retiree_le = NULL,
                                        retiree_par = NULL, publication = 'explicite', publication_prevue_le = NULL,
                                        en_cours_de_correction = false, modifie_par = $4
                                  WHERE etablissement_id = $1 AND id = $2",
                            )
                            .bind(etab)
                            .bind(evaluation_id)
                            .bind(maintenant)
                            .bind(acteur)
                            .execute(&mut *tx)
                            .await
                            .map_err(db)?;
                            journal_publication(
                                &mut tx,
                                etab,
                                evaluation_id,
                                "publiee",
                                None,
                                None,
                                Some(acteur),
                            )
                            .await?;
                            publier_evt(
                                &mut tx,
                                etab,
                                types_v3::EVALUATION_PUBLIEE,
                                evaluation_id,
                                acteur,
                            )
                            .await?;
                        }
                    }
                    "programmer" => {
                        let prevue_utc = match body.prevue_locale {
                            Some(locale) => sqlx::query_scalar::<_, DateTime<Utc>>(
                                "SELECT ($1::timestamp AT TIME ZONE COALESCE(
                                     (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $2),
                                     'Europe/Paris'))",
                            )
                            .bind(locale)
                            .bind(etab)
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(db)
                            .map(Some)?,
                            None => body.prevue_le,
                        };
                        let Some(prevue) = prevue_utc.filter(|p| *p > maintenant) else {
                            return Err(refus("la publication programmée doit être dans le futur"));
                        };
                        sqlx::query(
                            "UPDATE eva_evaluation SET publication = 'programmee', publication_prevue_le = $3, modifie_par = $4
                              WHERE etablissement_id = $1 AND id = $2",
                        )
                        .bind(etab)
                        .bind(evaluation_id)
                        .bind(prevue)
                        .bind(acteur)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                        journal_publication(
                            &mut tx,
                            etab,
                            evaluation_id,
                            "programmee",
                            Some(prevue),
                            None,
                            Some(acteur),
                        )
                        .await?;
                        publier_evt(
                            &mut tx,
                            etab,
                            types_v3::PUBLICATION_PROGRAMMEE,
                            evaluation_id,
                            acteur,
                        )
                        .await?;
                    }
                    "a_l_arret" => {
                        sqlx::query(
                            "UPDATE eva_evaluation SET publication = 'a_l_arret', publication_prevue_le = NULL, modifie_par = $3
                              WHERE etablissement_id = $1 AND id = $2",
                        )
                        .bind(etab)
                        .bind(evaluation_id)
                        .bind(acteur)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                        journal_publication(
                            &mut tx,
                            etab,
                            evaluation_id,
                            "definie_a_l_arret",
                            None,
                            None,
                            Some(acteur),
                        )
                        .await?;
                    }
                    "annuler_programmation" => {
                        sqlx::query(
                            "UPDATE eva_evaluation SET publication = 'explicite', publication_prevue_le = NULL, modifie_par = $3
                              WHERE etablissement_id = $1 AND id = $2 AND publication <> 'explicite'",
                        )
                        .bind(etab)
                        .bind(evaluation_id)
                        .bind(acteur)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                        journal_publication(
                            &mut tx,
                            etab,
                            evaluation_id,
                            "programmation_annulee",
                            None,
                            None,
                            Some(acteur),
                        )
                        .await?;
                    }
                    _ => return Err(refus("action de publication inconnue")),
                }
            }
        }
        tx.commit().await.map_err(db)?;
        self.eva_v3_colonne(etab, acteur, role, evaluation_id, jour, maintenant)
            .await
    }

    /// Publications automatiques d'un établissement (tâche planifiée, horloge **fournie**) :
    /// 1. les publications programmées dont l'heure est venue ;
    /// 2. au **délai maximal** de l'établissement (réglage facultatif), les évaluations qui ont des résultats,
    ///    sauf « en cours de correction » ;
    /// 3. un **rappel** unique au professeur deux jours avant.
    ///
    /// Idempotente : une évaluation déjà publiée n'est jamais republiée, un rappel n'est envoyé qu'une fois.
    pub async fn eva_v3_publications_automatiques_au(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
        jour: NaiveDate,
    ) -> Result<BilanPublications, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut bilan = BilanPublications::default();
        // 1. programmées
        let programmees: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, cree_par FROM eva_evaluation
              WHERE etablissement_id = $1 AND etat = 'ouverte' AND publiee_le IS NULL
                AND publication = 'programmee' AND publication_prevue_le <= $2
              ORDER BY publication_prevue_le, id FOR UPDATE",
        )
        .bind(etab)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, par) in &programmees {
            publier_auto(&mut tx, etab, *id, maintenant, "publiee_auto_programmee").await?;
            publier_evt_sys(&mut tx, etab, types_v3::EVALUATION_PUBLIEE, *id, *par).await?;
            bilan.programmees += 1;
        }
        // 2. délai maximal
        let delai: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT e.id, e.cree_par
               FROM eva_evaluation e
               JOIN eva_reglage g ON g.etablissement_id = e.etablissement_id AND g.annee_id = e.annee_id
              WHERE e.etablissement_id = $1 AND e.etat = 'ouverte' AND e.publiee_le IS NULL
                AND e.retiree_le IS NULL AND NOT e.en_cours_de_correction
                AND g.delai_publication_max_jours IS NOT NULL
                AND e.date_eval + g.delai_publication_max_jours::int <= $2
                AND EXISTS (SELECT 1 FROM eva_resultat r WHERE r.etablissement_id = e.etablissement_id
                                                          AND r.evaluation_id = e.id)
              ORDER BY e.date_eval, e.id FOR UPDATE OF e",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, par) in &delai {
            publier_auto(&mut tx, etab, *id, maintenant, "publiee_auto_delai").await?;
            publier_evt_sys(&mut tx, etab, types_v3::EVALUATION_PUBLIEE, *id, *par).await?;
            bilan.delai_max += 1;
        }
        // 3. rappels (deux jours avant le terme)
        let rappels: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT e.id, e.cree_par
               FROM eva_evaluation e
               JOIN eva_reglage g ON g.etablissement_id = e.etablissement_id AND g.annee_id = e.annee_id
              WHERE e.etablissement_id = $1 AND e.etat = 'ouverte' AND e.publiee_le IS NULL
                AND e.retiree_le IS NULL AND NOT e.en_cours_de_correction AND e.rappel_publication_le IS NULL
                AND g.delai_publication_max_jours IS NOT NULL
                AND e.date_eval + g.delai_publication_max_jours::int - 2 <= $2
                AND e.date_eval + g.delai_publication_max_jours::int > $2
                AND EXISTS (SELECT 1 FROM eva_resultat r WHERE r.etablissement_id = e.etablissement_id
                                                          AND r.evaluation_id = e.id)
              ORDER BY e.date_eval, e.id FOR UPDATE OF e",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, par) in &rappels {
            sqlx::query(
                "UPDATE eva_evaluation SET rappel_publication_le = $3 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            journal_publication(&mut tx, etab, *id, "rappel_delai", None, None, None).await?;
            publier_evt_sys(&mut tx, etab, types_v3::RAPPEL_PUBLICATION, *id, *par).await?;
            bilan.rappels += 1;
        }
        tx.commit().await.map_err(db)?;
        Ok(bilan)
    }
}

async fn publier_auto(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    id: Uuid,
    maintenant: DateTime<Utc>,
    action: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE eva_evaluation SET publiee_le = $3, publiee_par = NULL, retiree_le = NULL, retiree_par = NULL,
                publication = 'explicite', publication_prevue_le = NULL
          WHERE etablissement_id = $1 AND id = $2 AND publiee_le IS NULL",
    )
    .bind(etab)
    .bind(id)
    .bind(maintenant)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    journal_publication(tx, etab, id, action, None, None, None).await
}

async fn journal_publication(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
    action: &str,
    prevue_le: Option<DateTime<Utc>>,
    motif: Option<&str>,
    par: Option<Uuid>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO eva_evaluation_publication_journal (id, etablissement_id, evaluation_id, action, prevue_le, motif, par)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(evaluation_id)
    .bind(action)
    .bind(prevue_le)
    .bind(motif)
    .bind(par)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

async fn publier_evt(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    type_name: &str,
    evaluation_id: Uuid,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    tx.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Devoir,
        aggregate_id: evaluation_id,
        eleve_id: None,
        payload: json!({ "evaluation_id": evaluation_id }),
        actor_user_id: Some(acteur.to_string()),
    })
    .await?;
    Ok(())
}

async fn publier_evt_sys(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    type_name: &str,
    evaluation_id: Uuid,
    auteur: Option<Uuid>,
) -> Result<(), EntDbError> {
    tx.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Devoir,
        aggregate_id: evaluation_id,
        eleve_id: None,
        payload: json!({ "evaluation_id": evaluation_id, "auteur_id": auteur }),
        actor_user_id: None,
    })
    .await?;
    Ok(())
}

/// Un refus de garde de la base (SQLSTATE 23514 : déclencheurs et contraintes) devient un refus lisible, sans
/// recopier le texte brut de la base.
fn map_garde(e: sqlx::Error) -> EntDbError {
    if let Some(d) = e.as_database_error() {
        if d.code().as_deref() == Some("23514") || d.code().as_deref() == Some("23503") {
            return refus(
                "refusé par les règles de l'évaluation (sous-matière, barème ou période)",
            );
        }
    }
    db(e)
}
