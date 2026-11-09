//! Référentiel S2-2 : cycle de vie de l'année et bascule (REF-01, 02, 03, 11).
//!
//! `preparation` → `courante` → `cloturee`. Une seule année courante par établissement
//! (index unique). Ouvrir une année clôt la précédente (elle passe en lecture seule).
//! La **reconduction des structures** (classes, groupes, périodes, services) vers l'année
//! suivante est idempotente : relancée, elle ne crée rien de plus. Les affectations d'élèves
//! se posent en lot, avec prévisualisation, par correspondance de classes (montée).

use chrono::{Datelike, NaiveDate};
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, publier_ref, Acteur};
use crate::referentiel_structure::{verifier_couverture, PeriodeSaisie};
use crate::service::{EntDbError, PgEnt};

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct AnneeDetail {
    pub id: Uuid,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub courante: bool,
    pub statut: String,
}

const COLONNES_ANNEE: &str = "id, libelle, date_debut, date_fin, courante, statut";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportReconduction {
    pub annee_cible_id: Uuid,
    pub classes_creees: i64,
    pub groupes_crees: i64,
    pub periodes_creees: i64,
    pub services_crees: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportOuverture {
    pub annee_id: Uuid,
    /// Année précédemment courante, désormais clôturée.
    pub annee_cloturee_id: Option<Uuid>,
    /// Élèves dont la classe « courante » (texte + `classe_id`) a basculé sur la nouvelle année.
    pub eleves_synchronises: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportLot {
    pub apercu: bool,
    pub a_creer: i64,
    pub deja_conformes: i64,
    /// Élèves déjà affectés à une autre classe dans l'année cible (non modifiés).
    pub conflits: Vec<Uuid>,
    /// Élèves inconnus de l'établissement ou sortis avant l'année (ignorés).
    pub ignores: Vec<Uuid>,
}

/// Décalage en années entre deux dates de début d'année (2026-09-01 → 2027-09-01 = 1).
fn decaler(d: NaiveDate, annees: i32) -> NaiveDate {
    d.with_year(d.year() + annees)
        .or_else(|| NaiveDate::from_ymd_opt(d.year() + annees, d.month(), 28))
        .unwrap_or(d)
}

impl PgEnt {
    pub async fn ref_annees(&self, a: &Acteur<'_>) -> Result<Vec<AnneeDetail>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        sqlx::query_as::<_, AnneeDetail>(&format!(
            "SELECT {COLONNES_ANNEE} FROM ent_annee_scolaire
              WHERE etablissement_id = $1 ORDER BY date_debut DESC"
        ))
        .bind(a.etablissement_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    async fn annee_detail(&self, etab: Uuid, annee_id: Uuid) -> Result<AnneeDetail, EntDbError> {
        sqlx::query_as::<_, AnneeDetail>(&format!(
            "SELECT {COLONNES_ANNEE} FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(annee_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    /// Refuse l'écriture dans une année clôturée (lecture seule).
    pub(crate) async fn exiger_annee_modifiable(
        &self,
        etablissement_id: Uuid,
        annee_id: Uuid,
    ) -> Result<(), EntDbError> {
        let cloturee: Option<bool> = sqlx::query_scalar(
            "SELECT cloturee_le IS NOT NULL FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(annee_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        match cloturee {
            None => Err(introuvable()),
            Some(true) => Err(conflit()),
            Some(false) => Ok(()),
        }
    }

    /// REF-01 : crée une année « en préparation ». Les dates ne chevauchent aucune autre
    /// année, le libellé est unique.
    pub async fn ref_creer_annee(
        &self,
        a: &Acteur<'_>,
        libelle: &str,
        date_debut: NaiveDate,
        date_fin: NaiveDate,
    ) -> Result<AnneeDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let libelle = libelle.trim();
        if libelle.is_empty() || libelle.chars().count() > 40 || date_fin <= date_debut {
            return Err(invalide());
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        // Sérialise les créations d'année d'un même établissement.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(a.etablissement_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let chevauche: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND date_debut <= $3 AND date_fin >= $2",
        )
        .bind(a.etablissement_id)
        .bind(date_debut)
        .bind(date_fin)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if chevauche > 0 {
            return Err(conflit());
        }
        let annee = sqlx::query_as::<_, AnneeDetail>(&format!(
            "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
             VALUES ($1,$2,$3,$4,$5,false) RETURNING {COLONNES_ANNEE}"
        ))
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(libelle)
        .bind(date_debut)
        .bind(date_fin)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.annee_creee",
            annee.id,
            json!({ "libelle": annee.libelle }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(annee)
    }

    /// REF-02 : ouvre l'année (elle devient la courante) ; l'ancienne courante est clôturée.
    /// Préconditions : année en préparation, au moins une classe active, périodes globales
    /// couvrant l'année sans trou. Les élèves ayant une affectation de classe ouverte dans
    /// l'année voient leur classe « courante » (`ent_eleve.classe` + `classe_id`) basculer.
    pub async fn ref_ouvrir_annee(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<RapportOuverture, EntDbError> {
        self.ref_ouvrir_annee_forcee(a, annee_id, false).await
    }

    /// Comme [`Self::ref_ouvrir_annee`] ; `forcer` lève le refus quand l'ancienne courante a
    /// encore des périodes de notes ouvertes (mêmes contrôles que la clôture directe, socle 5
    /// S0 défaut n° 4 ; `forcer` est tracé dans l'événement de clôture).
    pub async fn ref_ouvrir_annee_forcee(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        forcer: bool,
    ) -> Result<RapportOuverture, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut tx = self.pool().begin().await.map_err(db)?;
        crate::ref_commun::contexte_etab(&mut tx, e).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(e)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let annee: Option<(NaiveDate, NaiveDate, String)> = sqlx::query_as(
            "SELECT date_debut, date_fin, statut FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((debut, fin, statut)) = annee else {
            return Err(introuvable());
        };
        if statut != "preparation" {
            return Err(conflit());
        }
        let classes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_classe
              WHERE etablissement_id = $1 AND annee_id = $2 AND archivee_le IS NULL",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let periodes: Vec<(String, String, String, NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT p.code, p.libelle, p.kind, p.date_debut, p.date_fin FROM ent_periode p
              WHERE p.etablissement_id = $1 AND p.annee_id = $2
                AND p.periodisation_id IN (
                    SELECT c.periodisation_id FROM ent_periodisation_cible c
                     WHERE c.etablissement_id = $1 AND c.annee_id = $2
                       AND c.usage = 'evaluation' AND c.defaut)",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let saisies: Vec<PeriodeSaisie> = periodes
            .into_iter()
            .map(|(code, libelle, kind, date_debut, date_fin)| PeriodeSaisie {
                code,
                libelle,
                kind,
                date_debut,
                date_fin,
            })
            .collect();
        if classes == 0 || verifier_couverture(&saisies, debut, fin).is_err() {
            return Err(invalide());
        }
        // Ancienne courante : mêmes contrôles que la clôture directe (périodes de notes).
        let ouvertes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_note_periode np
               JOIN ent_periode p ON p.id = np.periode_id AND p.etablissement_id = np.etablissement_id
               JOIN ent_annee_scolaire y ON y.id = p.annee_id AND y.etablissement_id = p.etablissement_id
              WHERE np.etablissement_id = $1 AND y.courante AND y.id <> $2 AND np.statut <> 'fermee'",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if ouvertes > 0 && !forcer {
            return Err(conflit());
        }
        // Clôturée d'abord (index unique « une seule courante »).
        let ancienne: Option<Uuid> = sqlx::query_scalar(
            "UPDATE ent_annee_scolaire SET cloturee_le = now()
              WHERE etablissement_id = $1 AND courante AND id <> $2 RETURNING id",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_annee_scolaire SET courante = true WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(annee_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let synchro = sqlx::query(
            "UPDATE ent_eleve el SET classe = c.code, classe_id = c.id
               FROM ent_affectation af
               JOIN ent_classe c ON c.id = af.classe_id AND c.etablissement_id = af.etablissement_id
              WHERE el.etablissement_id = $1 AND af.etablissement_id = $1
                AND af.annee_id = $2 AND af.classe_id IS NOT NULL AND af.date_fin IS NULL
                AND el.id = af.eleve_id
                AND (el.classe_id IS DISTINCT FROM c.id OR el.classe IS DISTINCT FROM c.code)",
        )
        .bind(e)
        .bind(annee_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut events = Vec::new();
        if let Some(old) = ancienne {
            events.push(
                publier_ref(&mut tx, a, "referentiel.annee_cloturee", old, json!({ "par": "ouverture", "forcer": forcer }))
                    .await?,
            );
        }
        events.push(
            publier_ref(
                &mut tx,
                a,
                "referentiel.annee_ouverte",
                annee_id,
                json!({ "annee_precedente_id": ancienne }),
            )
            .await?,
        );
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        Ok(RapportOuverture {
            annee_id,
            annee_cloturee_id: ancienne,
            eleves_synchronises: synchro.rows_affected() as i64,
        })
    }

    /// REF-03 : clôture l'année (archivage, lecture seule). Refusée tant qu'une période de
    /// notes de l'année reste ouverte, sauf `forcer` (tracé dans l'événement).
    pub async fn ref_cloturer_annee(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        forcer: bool,
    ) -> Result<AnneeDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let statut: Option<String> = sqlx::query_scalar(
            "SELECT statut FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        match statut.as_deref() {
            None => return Err(introuvable()),
            Some("cloturee") => return Err(conflit()),
            _ => {}
        }
        let ouvertes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_note_periode np
               JOIN ent_periode p ON p.id = np.periode_id AND p.etablissement_id = np.etablissement_id
              WHERE np.etablissement_id = $1 AND p.annee_id = $2 AND np.statut <> 'fermee'",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if ouvertes > 0 && !forcer {
            return Err(conflit());
        }
        // Fermer l'année courante sans successeur laisserait l'établissement sans année
        // courante (socle 5 S0, défaut n° 5) : refusé sauf `forcer`.
        let sans_successeur: bool = sqlx::query_scalar(
            "SELECT y.courante AND NOT EXISTS (
                      SELECT 1 FROM ent_annee_scolaire n
                       WHERE n.etablissement_id = y.etablissement_id AND n.id <> y.id
                         AND n.cloturee_le IS NULL AND n.date_debut > y.date_debut)
               FROM ent_annee_scolaire y WHERE y.etablissement_id = $1 AND y.id = $2",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if sans_successeur && !forcer {
            return Err(conflit());
        }
        sqlx::query(
            "UPDATE ent_annee_scolaire SET cloturee_le = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(annee_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.annee_cloturee",
            annee_id,
            json!({ "par": "cloture", "periodes_ouvertes": ouvertes, "forcee": forcer && ouvertes > 0 }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        self.annee_detail(e, annee_id).await
    }

    /// Reconduit les structures d'une année vers une autre (idempotent) : classes (site,
    /// niveau, MEF conservés), groupes, périodes (dates décalées d'autant d'années que les
    /// débuts d'année) et, si demandé, services d'enseignement (sans professeur principal :
    /// à redésigner chaque année). Les affectations d'élèves ne sont jamais reconduites ici.
    pub async fn ref_reconduire_structures(
        &self,
        a: &Acteur<'_>,
        source_id: Uuid,
        cible_id: Uuid,
        avec_services: bool,
    ) -> Result<RapportReconduction, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if source_id == cible_id {
            return Err(invalide());
        }
        let e = a.etablissement_id;
        let source = self.annee_detail(e, source_id).await?;
        let cible = self.annee_detail(e, cible_id).await?;
        if cible.statut == "cloturee" {
            return Err(conflit());
        }
        let decalage = cible.date_debut.year() - source.date_debut.year();
        let mut tx = self.pool().begin().await.map_err(db)?;
        crate::ref_commun::contexte_etab(&mut tx, e).await?;
        let mut r = RapportReconduction {
            annee_cible_id: cible_id,
            ..Default::default()
        };
        r.classes_creees = sqlx::query(
            "INSERT INTO ent_classe
               (id, etablissement_id, annee_id, code, libelle, niveau, site_id, niveau_id, mef_id)
             SELECT gen_random_uuid(), etablissement_id, $3, code, libelle, niveau, site_id, niveau_id, mef_id
               FROM ent_classe
              WHERE etablissement_id = $1 AND annee_id = $2 AND archivee_le IS NULL
             ON CONFLICT DO NOTHING",
        )
        .bind(e)
        .bind(source_id)
        .bind(cible_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected() as i64;
        r.groupes_crees = sqlx::query(
            "INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle, classe_id)
             SELECT gen_random_uuid(), g.etablissement_id, $3, g.code, g.libelle, cc.id
               FROM ent_groupe g
               LEFT JOIN ent_classe cs ON cs.id = g.classe_id
               LEFT JOIN ent_classe cc ON cc.etablissement_id = g.etablissement_id
                    AND cc.annee_id = $3 AND cc.code = cs.code
              WHERE g.etablissement_id = $1 AND g.annee_id = $2
                AND (g.classe_id IS NULL OR cc.id IS NOT NULL)
             ON CONFLICT DO NOTHING",
        )
        .bind(e)
        .bind(source_id)
        .bind(cible_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected() as i64;
        let periodes: Vec<(Option<Uuid>, String, String, String, i16, NaiveDate, NaiveDate)> =
            sqlx::query_as(
                "SELECT niveau_id, code, libelle, kind, ordre, date_debut, date_fin FROM ent_periode
                  WHERE etablissement_id = $1 AND annee_id = $2
                    AND (niveau_id IS NOT NULL OR periodisation_id IN (
                        SELECT c.periodisation_id FROM ent_periodisation_cible c
                         WHERE c.etablissement_id = $1 AND c.annee_id = $2
                           AND c.usage = 'evaluation' AND c.defaut))",
            )
            .bind(e)
            .bind(source_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        // Dates décalées d'années entières, puis continuité rétablie : une année bissextile
        // ne doit créer ni trou ni chevauchement (la fin = veille du début de la suivante).
        let mut periodes = periodes;
        periodes.sort_by_key(|p| (p.0, p.5));
        let mut decalees: Vec<(NaiveDate, NaiveDate)> = periodes
            .iter()
            .map(|p| (decaler(p.5, decalage), decaler(p.6, decalage)))
            .collect();
        for i in 0..periodes.len().saturating_sub(1) {
            let contigues = periodes[i].0 == periodes[i + 1].0
                && periodes[i + 1].5 == periodes[i].6 + chrono::Duration::days(1);
            if contigues {
                decalees[i].1 = decalees[i + 1].0 - chrono::Duration::days(1);
            }
        }
        for (i, (niveau_id, code, libelle, kind, ordre, _, _)) in periodes.into_iter().enumerate() {
            let (d, f) = decalees[i];
            let n = sqlx::query(
                "INSERT INTO ent_periode
                   (id, etablissement_id, annee_id, niveau_id, code, libelle, kind, ordre, date_debut, date_fin)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(e)
            .bind(cible_id)
            .bind(niveau_id)
            .bind(code)
            .bind(libelle)
            .bind(kind)
            .bind(ordre)
            .bind(d)
            .bind(f)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            r.periodes_creees += n.rows_affected() as i64;
        }
        if avec_services {
            r.services_crees = sqlx::query(
                "INSERT INTO ent_service_enseignement
                   (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle,
                    classe_id, groupe_id, professeur_principal)
                 SELECT gen_random_uuid(), s.etablissement_id, $3, s.prof_user_id, s.matiere_id,
                        s.matiere_libelle, cc.id, gg.id, false
                   FROM ent_service_enseignement s
                   LEFT JOIN ent_classe cs ON cs.id = s.classe_id
                   LEFT JOIN ent_classe cc ON cc.etablissement_id = s.etablissement_id
                        AND cc.annee_id = $3 AND cc.code = cs.code
                   LEFT JOIN ent_groupe gs ON gs.id = s.groupe_id
                   LEFT JOIN ent_groupe gg ON gg.etablissement_id = s.etablissement_id
                        AND gg.annee_id = $3 AND gg.code = gs.code
                  WHERE s.etablissement_id = $1 AND s.annee_id = $2
                    AND (cc.id IS NOT NULL OR gg.id IS NOT NULL)
                 ON CONFLICT DO NOTHING",
            )
            .bind(e)
            .bind(source_id)
            .bind(cible_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected() as i64;
        }
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.structures_reconduites",
            cible_id,
            json!({ "source": source_id, "classes": r.classes_creees, "groupes": r.groupes_crees,
                    "periodes": r.periodes_creees, "services": r.services_crees }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(r)
    }

    /// Prépare l'année suivant `source_id` (dates +1 an, libellé « AAAA-AAAA » suivant),
    /// puis reconduit les structures. Idempotent : si l'année suivante existe déjà, elle est
    /// réutilisée.
    pub async fn ref_preparer_annee_suivante(
        &self,
        a: &Acteur<'_>,
        source_id: Uuid,
        avec_services: bool,
    ) -> Result<(AnneeDetail, RapportReconduction), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let source = self.annee_detail(a.etablissement_id, source_id).await?;
        let (debut, fin) = (decaler(source.date_debut, 1), decaler(source.date_fin, 1));
        let existante: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND date_debut = $2 AND date_fin = $3",
        )
        .bind(a.etablissement_id)
        .bind(debut)
        .bind(fin)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let cible = match existante {
            Some(id) => self.annee_detail(a.etablissement_id, id).await?,
            None => {
                let libelle = format!("{}-{}", debut.year(), fin.year());
                self.ref_creer_annee(a, &libelle, debut, fin).await?
            }
        };
        let rapport = self
            .ref_reconduire_structures(a, source_id, cible.id, avec_services)
            .await?;
        Ok((cible, rapport))
    }

    /// REF-11 : affecte des élèves à des classes de l'année cible, en lot. `apercu` calcule
    /// sans écrire. Un élève déjà affecté à la même classe est conforme ; à une autre classe
    /// de l'année, c'est un conflit (jamais écrasé). Idempotent.
    pub async fn ref_affecter_lot(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        items: &[(Uuid, Uuid)],
        apercu: bool,
        source: &str,
    ) -> Result<RapportLot, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !matches!(source, "lot" | "montee") || items.len() > 10_000 {
            return Err(invalide());
        }
        let e = a.etablissement_id;
        let annee = self.annee_detail(e, annee_id).await?;
        if annee.statut == "cloturee" {
            return Err(conflit());
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let mut rapport = RapportLot {
            apercu,
            ..Default::default()
        };
        for &(eleve_id, classe_id) in items {
            let classe_ok: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_classe
                  WHERE etablissement_id = $1 AND id = $2 AND annee_id = $3 AND archivee_le IS NULL",
            )
            .bind(e)
            .bind(classe_id)
            .bind(annee_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if classe_ok == 0 {
                // Classe d'une autre année ou d'un autre établissement : tout le lot est refusé.
                return Err(introuvable());
            }
            let eleve_ok: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_eleve
                  WHERE etablissement_id = $1 AND id = $2
                    AND (date_sortie IS NULL OR date_sortie >= $3)",
            )
            .bind(e)
            .bind(eleve_id)
            .bind(annee.date_debut)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if eleve_ok == 0 {
                rapport.ignores.push(eleve_id);
                continue;
            }
            let ouverte: Option<Uuid> = sqlx::query_scalar(
                "SELECT classe_id FROM ent_affectation
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3
                    AND classe_id IS NOT NULL AND date_fin IS NULL",
            )
            .bind(e)
            .bind(eleve_id)
            .bind(annee_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match ouverte {
                Some(c) if c == classe_id => rapport.deja_conformes += 1,
                Some(_) => rapport.conflits.push(eleve_id),
                None => {
                    rapport.a_creer += 1;
                    if !apercu {
                        sqlx::query(
                            "INSERT INTO ent_affectation
                               (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut, source)
                             VALUES ($1,$2,$3,$4,$5,$6,$7)",
                        )
                        .bind(Uuid::new_v4())
                        .bind(e)
                        .bind(annee_id)
                        .bind(eleve_id)
                        .bind(classe_id)
                        .bind(annee.date_debut)
                        .bind(source)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    }
                }
            }
        }
        let mut events = Vec::new();
        if !apercu {
            events.push(
                publier_ref(
                    &mut tx,
                    a,
                    "referentiel.affectations_lot",
                    annee_id,
                    json!({ "source": source, "creees": rapport.a_creer,
                            "conformes": rapport.deja_conformes, "conflits": rapport.conflits.len() }),
                )
                .await?,
            );
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        Ok(rapport)
    }

    /// Montée : pour chaque correspondance (classe de l'année source → classe de l'année
    /// cible), affecte les élèves actuellement dans la classe source à la classe cible.
    pub async fn ref_monter(
        &self,
        a: &Acteur<'_>,
        source_id: Uuid,
        cible_id: Uuid,
        correspondances: &[(Uuid, Uuid)],
        apercu: bool,
    ) -> Result<RapportLot, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        self.annee_detail(e, source_id).await?;
        self.annee_detail(e, cible_id).await?;
        let mut items: Vec<(Uuid, Uuid)> = Vec::new();
        for &(src, dst) in correspondances {
            let ok: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_classe WHERE etablissement_id = $1 AND id = $2 AND annee_id = $3",
            )
            .bind(e)
            .bind(src)
            .bind(source_id)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if ok == 0 {
                return Err(introuvable());
            }
            let eleves: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT DISTINCT eleve_id FROM ent_affectation
                  WHERE etablissement_id = $1 AND annee_id = $2 AND classe_id = $3
                    AND date_fin IS NULL",
            )
            .bind(e)
            .bind(source_id)
            .bind(src)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
            items.extend(eleves.into_iter().map(|(id,)| (id, dst)));
        }
        self.ref_affecter_lot(a, cible_id, &items, apercu, "montee").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decalage_d_annees() {
        let d = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        assert_eq!(decaler(d, 1), NaiveDate::from_ymd_opt(2027, 9, 1).unwrap());
        let bissextile = NaiveDate::from_ymd_opt(2028, 2, 29).unwrap();
        assert_eq!(decaler(bissextile, 1), NaiveDate::from_ymd_opt(2029, 2, 28).unwrap());
    }
}
