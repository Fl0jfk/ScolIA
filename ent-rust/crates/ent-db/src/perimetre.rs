//! Périmètre relationnel des acteurs (socle 1, lot B1).
//!
//! Règle unique « un professeur ne voit que ses élèves » : un professeur **suit**
//! un élève s'il lui enseigne aujourd'hui. Sources, dans l'ordre :
//! 1. référentiel scolaire (`enseigne_a` : **public du cours**, `ent_public_cours_jour`,
//!    socle 5 S1 : classe ou groupe + membres individuels − exclusions). Un cours porte sur
//!    une **classe** (professeur principal, enseignement de la classe entière) ou sur un
//!    **groupe** : dans une classe mixte (LV2 allemand / espagnol), le professeur d'allemand
//!    ne suit que le groupe allemand (D-022). Matières, groupes et services sont du
//!    paramétrage libre de l'établissement : aucune liste dans le code. La règle n'existe
//!    qu'à un endroit (la fonction SQL) ; ce module et `enseignements.rs` l'appellent ;
//! 2. emploi du temps historique (créneau du professeur sur la classe texte de l'élève),
//!    **uniquement pour les élèves que le référentiel ne couvre pas encore** : la classe
//!    texte ignore les groupes et ne doit jamais élargir la portée (D-022) ;
//! 3. remplacement récent ou prochain (±14 jours) sur un créneau de la classe : le
//!    remplaçant suit les élèves du **titulaire** remplacé (groupes inclus).
//!
//! Défaut sûr : élève inconnu, sorti, ou d'un autre établissement → `false`.
//! Les autres personnels (direction, administratif, CPE…) gardent leur périmètre
//! d'établissement ; seuls les professeurs sont restreints ici.

use chrono::NaiveDate;
use uuid::Uuid;

use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("perimetre", err))
}

/// Fenêtre (jours) pendant laquelle un remplaçant suit les élèves de la classe.
const FENETRE_REMPLACEMENT_JOURS: i32 = 14;

impl PgEnt {
    /// Le professeur suit-il cet élève aujourd'hui ? (voir le module)
    ///
    /// « Aujourd'hui » est le jour **de l'établissement** (fuseau du paramètre d'établissement,
    /// Europe/Paris par défaut ; `acces::aujourdhui_etablissement`), pas le jour UTC : un service ou
    /// une attribution qui finit le 31 août reste actif jusqu'à minuit heure locale, et s'éteint
    /// ensuite (socle 6 P0, `06_PERSONNELS.md` §2.2 n° 6 ; R-P2).
    pub async fn prof_suit_eleve(
        &self,
        etablissement_id: Uuid,
        prof_user_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let jour = self.aujourdhui_etablissement(etablissement_id).await?;
        self.prof_suit_eleve_le(etablissement_id, prof_user_id, eleve_id, jour)
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
                  AND (c.groupe_id IS NULL OR EXISTS (
                          SELECT 1 FROM ent_affectation ga
                           WHERE ga.etablissement_id = e.etablissement_id
                             AND ga.groupe_id = c.groupe_id AND ga.eleve_id = e.id
                             AND ga.date_debut <= $4
                             AND (ga.date_fin IS NULL OR ga.date_fin >= $4)))
                  AND ((ent_repli_texte_actif(c.etablissement_id) AND NOT EXISTS (
                          SELECT 1 FROM ent_affectation fa
                            JOIN ent_annee_scolaire fy
                              ON fy.id = fa.annee_id AND fy.etablissement_id = fa.etablissement_id
                           WHERE fa.etablissement_id = e.etablissement_id AND fa.eleve_id = e.id
                             AND fa.classe_id IS NOT NULL
                             AND fy.date_debut <= $4 AND fy.date_fin >= $4
                             AND fa.date_debut <= $4
                             AND (fa.date_fin IS NULL OR fa.date_fin >= $4))
                        AND c.prof_user_id = $2)
                       OR (EXISTS (
                          SELECT 1 FROM ent_edt_exception x
                           WHERE x.etablissement_id = c.etablissement_id
                             AND x.creneau_id = c.id
                             AND x.remplacant_user_id = $2
                             AND x.jour BETWEEN $4 - $5 AND $4 + $5)
                           AND (NOT EXISTS (
                          SELECT 1 FROM ent_affectation fa
                            JOIN ent_annee_scolaire fy
                              ON fy.id = fa.annee_id AND fy.etablissement_id = fa.etablissement_id
                           WHERE fa.etablissement_id = e.etablissement_id AND fa.eleve_id = e.id
                             AND fa.classe_id IS NOT NULL
                             AND fy.date_debut <= $4 AND fy.date_fin >= $4
                             AND fa.date_debut <= $4
                             AND (fa.date_fin IS NULL OR fa.date_fin >= $4))
                                OR EXISTS (SELECT 1 FROM ent_public_cours_jour(c.etablissement_id, $4, NULL, c.prof_user_id, e.id))))))",
        )
        .bind(etablissement_id)
        .bind(prof_user_id)
        .bind(eleve_id)
        .bind(jour)
        .bind(FENETRE_REMPLACEMENT_JOURS)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
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
        .fetch_one(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
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
            "SELECT p.eleve_id FROM ent_public_cours_jour($1, $3, NULL, $2, NULL) p
             UNION
             SELECT e.id
               FROM ent_eleve e
               JOIN ent_edt_creneau c
                 ON c.etablissement_id = e.etablissement_id AND c.classe = e.classe
              WHERE e.etablissement_id = $1
                AND (e.date_sortie IS NULL OR e.date_sortie > $3)
                AND (c.groupe_id IS NULL OR EXISTS (
                          SELECT 1 FROM ent_affectation ga
                           WHERE ga.etablissement_id = e.etablissement_id
                             AND ga.groupe_id = c.groupe_id AND ga.eleve_id = e.id
                             AND ga.date_debut <= $3
                             AND (ga.date_fin IS NULL OR ga.date_fin >= $3)))
                AND ((ent_repli_texte_actif(c.etablissement_id) AND NOT EXISTS (
                          SELECT 1 FROM ent_affectation fa
                            JOIN ent_annee_scolaire fy
                              ON fy.id = fa.annee_id AND fy.etablissement_id = fa.etablissement_id
                           WHERE fa.etablissement_id = e.etablissement_id AND fa.eleve_id = e.id
                             AND fa.classe_id IS NOT NULL
                             AND fy.date_debut <= $3 AND fy.date_fin >= $3
                             AND fa.date_debut <= $3
                             AND (fa.date_fin IS NULL OR fa.date_fin >= $3))
                      AND c.prof_user_id = $2)
                     OR (EXISTS (
                        SELECT 1 FROM ent_edt_exception x
                         WHERE x.etablissement_id = c.etablissement_id
                           AND x.creneau_id = c.id
                           AND x.remplacant_user_id = $2
                           AND x.jour BETWEEN $3 - $4 AND $3 + $4)
                         AND (NOT EXISTS (
                          SELECT 1 FROM ent_affectation fa
                            JOIN ent_annee_scolaire fy
                              ON fy.id = fa.annee_id AND fy.etablissement_id = fa.etablissement_id
                           WHERE fa.etablissement_id = e.etablissement_id AND fa.eleve_id = e.id
                             AND fa.classe_id IS NOT NULL
                             AND fy.date_debut <= $3 AND fy.date_fin >= $3
                             AND fa.date_debut <= $3
                             AND (fa.date_fin IS NULL OR fa.date_fin >= $3))
                              OR EXISTS (SELECT 1 FROM ent_public_cours_jour(c.etablissement_id, $3, NULL, c.prof_user_id, e.id)))))",
        )
        .bind(etablissement_id)
        .bind(prof_user_id)
        .bind(jour)
        .bind(FENETRE_REMPLACEMENT_JOURS)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}
