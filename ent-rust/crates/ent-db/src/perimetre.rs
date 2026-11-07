//! Périmètre relationnel des acteurs (socle 1, lot B1).
//!
//! Règle unique « un professeur ne voit que ses élèves » : un professeur **suit**
//! un élève s'il lui enseigne aujourd'hui. Sources, dans l'ordre :
//! 1. référentiel scolaire (`enseigne_a` : service d'enseignement ∩ affectation) ;
//! 2. emploi du temps historique (créneau du professeur sur la classe texte de l'élève),
//!    tant que le référentiel n'est pas la source unique des classes ;
//! 3. remplacement récent ou prochain (±14 jours) sur un créneau de la classe.
//!
//! Défaut sûr : élève inconnu, sorti, ou d'un autre établissement → `false`.
//! Les autres personnels (direction, administratif, CPE…) gardent leur périmètre
//! d'établissement ; seuls les professeurs sont restreints ici.

use chrono::NaiveDate;
use domain_eleve::today_utc;
use uuid::Uuid;

use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("perimetre", err))
}

/// Fenêtre (jours) pendant laquelle un remplaçant suit les élèves de la classe.
const FENETRE_REMPLACEMENT_JOURS: i32 = 14;

impl PgEnt {
    /// Le professeur suit-il cet élève aujourd'hui ? (voir le module)
    pub async fn prof_suit_eleve(
        &self,
        etablissement_id: Uuid,
        prof_user_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        self.prof_suit_eleve_le(etablissement_id, prof_user_id, eleve_id, today_utc())
            .await
    }

    /// Variante datée (tests, consultations historiques).
    pub async fn prof_suit_eleve_le(
        &self,
        etablissement_id: Uuid,
        prof_user_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        if self
            .enseigne_a(etablissement_id, prof_user_id, eleve_id, jour)
            .await?
        {
            return Ok(true);
        }
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
               SELECT 1
                 FROM ent_eleve e
                 JOIN ent_edt_creneau c
                   ON c.etablissement_id = e.etablissement_id AND c.classe = e.classe
                WHERE e.etablissement_id = $1 AND e.id = $3
                  AND (e.date_sortie IS NULL OR e.date_sortie > $4)
                  AND (c.prof_user_id = $2
                       OR EXISTS (
                          SELECT 1 FROM ent_edt_exception x
                           WHERE x.etablissement_id = c.etablissement_id
                             AND x.creneau_id = c.id
                             AND x.remplacant_user_id = $2
                             AND x.jour BETWEEN $4 - $5 AND $4 + $5)))",
        )
        .bind(etablissement_id)
        .bind(prof_user_id)
        .bind(eleve_id)
        .bind(jour)
        .bind(FENETRE_REMPLACEMENT_JOURS)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// L'élève est-il majeur à la date donnée ? (`false` si date de naissance inconnue.)
    pub async fn eleve_est_majeur(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT coalesce((
               SELECT e.date_naissance + INTERVAL '18 years' <= $3
                 FROM ent_eleve e
                WHERE e.etablissement_id = $1 AND e.id = $2), false)",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Élèves suivis par un professeur à une date (même règle que
    /// [`Self::prof_suit_eleve_le`], en ensemble) : filtre des listes.
    pub async fn eleves_suivis_par_prof(
        &self,
        etablissement_id: Uuid,
        prof_user_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT a.eleve_id
               FROM ent_service_enseignement s
               JOIN ent_annee_scolaire y
                 ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
               JOIN ent_affectation a
                 ON a.etablissement_id = s.etablissement_id AND a.annee_id = s.annee_id
                AND ((s.classe_id IS NOT NULL AND a.classe_id = s.classe_id)
                  OR (s.groupe_id IS NOT NULL AND a.groupe_id = s.groupe_id))
               JOIN ent_eleve e
                 ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
              WHERE s.etablissement_id = $1 AND s.prof_user_id = $2
                AND y.date_debut <= $3 AND y.date_fin >= $3
                AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
                AND (e.date_sortie IS NULL OR e.date_sortie > $3)
             UNION
             SELECT e.id
               FROM ent_eleve e
               JOIN ent_edt_creneau c
                 ON c.etablissement_id = e.etablissement_id AND c.classe = e.classe
              WHERE e.etablissement_id = $1
                AND (e.date_sortie IS NULL OR e.date_sortie > $3)
                AND (c.prof_user_id = $2
                     OR EXISTS (
                        SELECT 1 FROM ent_edt_exception x
                         WHERE x.etablissement_id = c.etablissement_id
                           AND x.creneau_id = c.id
                           AND x.remplacant_user_id = $2
                           AND x.jour BETWEEN $3 - $4 AND $3 + $4))",
        )
        .bind(etablissement_id)
        .bind(prof_user_id)
        .bind(jour)
        .bind(FENETRE_REMPLACEMENT_JOURS)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}
