#![allow(clippy::too_many_arguments, clippy::type_complexity)]
//! Socle 10 (emploi du temps), lot T4 : le tableau « demain » et « maintenant » (§14.3, R-E26, R-E27, R-E37).
//!
//! **Tableau « demain »** (direction, secrétariat, vie scolaire) : absences de professeurs connues et ce qu'elles
//! touchent, séances non assurées, remplacements faits et à faire, salles fermées, changements posés (annulées,
//! déplacées, modifiées), permanences attendues (effectifs par plage), notifications parties et à partir. Calculé à
//! la demande : il est donc toujours « mis à jour en continu » ; la veille à 18 h il suffit de l'ouvrir.
//!
//! **« Maintenant »** : pour un élève, sa séance en cours ou « pas de cours » ; pour une salle, son occupation ; pour
//! un professeur, **présent dans l'établissement selon l'emploi du temps : oui ou non** — jamais où il est quand il n'a
//! pas cours. La lecture d'un élève est **journalisée** (`ent_edt_localisation_journal`, ajout seul : qui, quel élève,
//! quel usage, quelle nature de réponse, jamais le lieu). Une personne sous interdiction (remise, visite, contact avec
//! l'élève) et un élève sous confidentialité renforcée donnent la même réponse neutre « indisponible » : aucun oracle.
//!
//! **Élèves sans cours** (EDT-177, lecture) : les élèves dont la séance est annulée, déplacée ou non assurée et qui n'ont
//! rien d'autre au même moment (une séance neutralisée est un « ailleurs » : sortie, examen), comptés par plage pour
//! prévoir la permanence ; la répartition (permanence, étude, CDI, sortie selon le régime) est une écriture du socle 14.

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use ent_auth::{Action, Role};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::edt_modif_ops::SeanceNonAssuree;
use crate::ref_commun::{db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

pub const USAGES_LOCALISATION: [&str; 4] = ["remise", "urgence", "recherche", "infirmerie"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
pub struct AbsentDemain {
    pub user_id: Uuid,
    /// Séances encore actives de cet intervenant ce jour-là.
    pub nb_seances: i64,
    /// Parmi elles, celles qui n'ont ni remplaçant ni décision.
    pub nb_sans_decision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
pub struct SalleFermeeDemain {
    pub salle_id: Uuid,
    /// Séances actives du jour encore placées dans cette salle fermée.
    pub nb_seances_a_deplacer: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
pub struct PermanenceAttendue {
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    /// Élèves sans cours sur cette plage (séance annulée, déplacée ou non assurée, rien d'autre au même moment).
    pub effectif: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ChangementsDuJour {
    pub annulees: i64,
    pub deplacees: i64,
    pub neutralisees: i64,
    pub modifiees: i64,
    pub ajoutees: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct NotificationsDemain {
    /// Envois du socle 8 liés à l'emploi du temps déjà partis (ou simulés) pour ce jour.
    pub envoyees: i64,
    /// Envois en attente (message de la veille, résumés).
    pub a_envoyer: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableauDemain {
    pub jour: NaiveDate,
    pub intervenants_absents: Vec<AbsentDemain>,
    pub seances_non_assurees: Vec<SeanceNonAssuree>,
    /// Remplacements faits (modification `intervenant` vivante) sur les séances du jour.
    pub remplacements_faits: i64,
    /// Séances d'intervenants absents sans remplaçant ni décision : à traiter.
    pub remplacements_a_faire: i64,
    pub salles_fermees: Vec<SalleFermeeDemain>,
    pub changements: ChangementsDuJour,
    pub permanences: Vec<PermanenceAttendue>,
    pub notifications: NotificationsDemain,
}

/// Ce que « maintenant » répond pour un élève : jamais un lieu hors séance, jamais la raison d'un refus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum Maintenant {
    EnCours {
        seance_id: Uuid,
        cours_id: Option<Uuid>,
        matiere: String,
        classe: String,
        heure_debut: NaiveTime,
        heure_fin: NaiveTime,
        lieu_id: Option<Uuid>,
        /// Compte en saisie (titulaire ou remplaçant).
        intervenant_user_id: Uuid,
    },
    /// Pas de cours à cet instant (permanence, ou sortie selon le régime : décision du socle 14).
    PasDeCours,
    /// Réponse unique pour une personne sous interdiction, un élève sous confidentialité renforcée ou un lecteur
    /// qui n'a pas le droit de voir : aucun oracle.
    Indisponible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OccupationSalle {
    pub occupee: bool,
    pub seance_id: Option<Uuid>,
    pub classe: Option<String>,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PresenceProfesseur {
    /// Présent dans l'établissement selon l'emploi du temps (entre sa première et sa dernière séance active du
    /// jour). Jamais où il se trouve quand il n'a pas cours.
    pub present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
pub struct EleveSansCours {
    pub eleve_id: Uuid,
    pub classe_id: Option<Uuid>,
    pub seance_id: Uuid,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
}

impl PgEnt {
    /// Tableau « demain » (`jour` : par défaut le lendemain du jour de l'établissement).
    pub async fn edt_tableau_demain(
        &self,
        a: &Acteur<'_>,
        jour: Option<NaiveDate>,
    ) -> Result<TableauDemain, EntDbError> {
        self.exiger_edt(a, Action::EdtDemainLire).await?;
        let etab = a.etablissement_id;
        let jour = match jour {
            Some(j) => j,
            None => self.aujourdhui_etablissement(etab).await? + Duration::days(1),
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Intervenants absents (absence validée, ou signalée et confirmée : même règle que le remplacement).
        let absents: Vec<AbsentDemain> = sqlx::query_as(
            "SELECT s.u AS user_id, count(*) AS nb_seances,
                    count(*) FILTER (WHERE s.remplacant_user_id IS NULL AND s.statut = 'prevue') AS nb_sans_decision
               FROM (SELECT coalesce(x.remplacant_user_id, x.prof_user_id) AS u, x.remplacant_user_id, x.statut
                       FROM ent_edt_seance x
                      WHERE x.etablissement_id = $1 AND x.jour = $2 AND x.etat IN ('prevue', 'modifiee', 'ajoutee')) s
              WHERE EXISTS (SELECT 1 FROM ent_absence_rh r
                             WHERE r.etablissement_id = $1 AND r.user_id = s.u
                               AND (r.statut = 'validee' OR (r.statut = 'signalee' AND r.confirmee))
                               AND r.debut <= $2 AND r.fin >= $2)
              GROUP BY s.u ORDER BY 3 DESC, 2 DESC, 1",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let a_faire: i64 = absents.iter().map(|x| x.nb_sans_decision).sum();
        let non_assurees: Vec<SeanceNonAssuree> = sqlx::query_as(
            "SELECT s.id AS seance_id, s.jour, s.heure_debut, s.heure_fin, s.cours_id, s.classe,
                    CASE WHEN s.etat = 'non_assuree' THEN 'non_assuree' ELSE 'cours_vacant' END AS raison
               FROM ent_edt_seance s
              WHERE s.etablissement_id = $1 AND s.jour = $2
                AND (s.etat = 'non_assuree'
                     OR (s.etat IN ('prevue', 'modifiee', 'ajoutee') AND s.remplacant_user_id IS NULL
                         AND s.cours_id IN (SELECT v.cours_id FROM ent_cours_vacants(s.etablissement_id, s.jour) v)))
              ORDER BY s.heure_debut, s.id",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let (faits, annulees, deplacees, neutralisees, modifiees, ajoutees): (i64, i64, i64, i64, i64, i64) =
            sqlx::query_as(
                "SELECT (SELECT count(*) FROM ent_edt_modification m
                          WHERE m.etablissement_id = $1 AND m.jour = $2 AND m.type = 'intervenant' AND m.levee_le IS NULL),
                        count(*) FILTER (WHERE s.etat = 'annulee'), count(*) FILTER (WHERE s.etat = 'deplacee'),
                        count(*) FILTER (WHERE s.etat = 'neutralisee'), count(*) FILTER (WHERE s.etat = 'modifiee'),
                        count(*) FILTER (WHERE s.etat = 'ajoutee')
                   FROM ent_edt_seance s WHERE s.etablissement_id = $1 AND s.jour = $2",
            )
            .bind(etab)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let salles: Vec<SalleFermeeDemain> = sqlx::query_as(
            "WITH fz AS (SELECT coalesce((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1),
                                         'Europe/Paris') AS z)
             SELECT i.salle_id, count(DISTINCT s.id) AS nb_seances_a_deplacer
               FROM ent_salle_indisponibilite i
               CROSS JOIN fz
               LEFT JOIN ent_edt_seance_lieu l ON l.etablissement_id = i.etablissement_id AND l.lieu_id = i.salle_id
               LEFT JOIN ent_edt_seance s ON s.id = l.seance_id AND s.etablissement_id = l.etablissement_id
                AND s.jour = $2 AND s.etat IN ('prevue', 'modifiee', 'ajoutee')
                AND (i.du AT TIME ZONE fz.z) < ($2::date + s.heure_fin) AND ($2::date + s.heure_debut) < (i.au AT TIME ZONE fz.z)
              WHERE i.etablissement_id = $1 AND i.salle_id IS NOT NULL AND i.annulee_le IS NULL
                AND (i.du AT TIME ZONE fz.z) < ($2::date + 1) AND (i.au AT TIME ZONE fz.z) > $2::date
              GROUP BY i.salle_id ORDER BY 2 DESC, 1",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let permanences = permanences(&mut tx, etab, jour, None).await?;
        // Envois du socle 8 liés aux changements DE CE JOUR : l'événement qui les a produits porte le jour de la séance
        // (ou la plage d'un lot qui le contient).
        let (envoyees, a_envoyer): (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE n.statut IN ('envoye', 'simule')),
                    count(*) FILTER (WHERE n.statut = 'en_attente')
               FROM ent_notification_envoi n
               JOIN ent_metier_event ev ON ev.id = n.source_id AND ev.etablissement_id = n.etablissement_id
              WHERE n.etablissement_id = $1 AND n.source_type = 'evenement' AND n.cle_dedup LIKE 'edt:%'
                AND (ev.payload->>'jour' = $2::text
                     OR (ev.payload ? 'du' AND (ev.payload->>'du')::date <= $2 AND (ev.payload->>'au')::date >= $2))",
        )
        .bind(etab)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(TableauDemain {
            jour,
            intervenants_absents: absents,
            seances_non_assurees: non_assurees,
            remplacements_faits: faits,
            remplacements_a_faire: a_faire,
            salles_fermees: salles,
            changements: ChangementsDuJour {
                annulees,
                deplacees,
                neutralisees,
                modifiees,
                ajoutees,
            },
            permanences,
            notifications: NotificationsDemain {
                envoyees,
                a_envoyer,
            },
        })
    }

    /// Élèves sans cours sur une plage du jour (EDT-177, lecture) : pour que la vie scolaire prévoie la permanence.
    pub async fn edt_eleves_sans_cours(
        &self,
        a: &Acteur<'_>,
        jour: NaiveDate,
        debut: Option<NaiveTime>,
        fin: Option<NaiveTime>,
    ) -> Result<Vec<EleveSansCours>, EntDbError> {
        self.exiger_edt(a, Action::EdtDemainLire).await?;
        if matches!((debut, fin), (Some(d), Some(f)) if f <= d) || debut.is_some() != fin.is_some()
        {
            return Err(invalide());
        }
        let etab = a.etablissement_id;
        sqlx::query_as::<_, EleveSansCours>(
            "SELECT DISTINCT p.eleve_id, s.classe_id, s.id AS seance_id, s.heure_debut, s.heure_fin
               FROM ent_edt_seance s
               CROSS JOIN LATERAL ent_public_cours_jour(s.etablissement_id, s.jour, s.cours_id) p
              WHERE s.etablissement_id = $1 AND s.jour = $2 AND s.cours_id IS NOT NULL
                AND s.etat IN ('annulee', 'non_assuree', 'deplacee')
                AND ($3::time IS NULL OR (s.heure_debut < $4 AND $3 < s.heure_fin))
                AND NOT EXISTS (
                    SELECT 1 FROM ent_edt_seance o
                      JOIN LATERAL ent_public_cours_jour(o.etablissement_id, o.jour, o.cours_id) q ON q.eleve_id = p.eleve_id
                     WHERE o.etablissement_id = s.etablissement_id AND o.jour = s.jour AND o.id <> s.id
                       AND o.cours_id IS NOT NULL AND o.etat IN ('prevue', 'modifiee', 'ajoutee', 'neutralisee')
                       AND o.heure_debut < s.heure_fin AND s.heure_debut < o.heure_fin)
              ORDER BY s.heure_debut, s.classe_id, p.eleve_id, s.id",
        )
        .bind(etab)
        .bind(jour)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }

    /// « Maintenant » pour un élève : sa séance en cours ou « pas de cours ». **Journalisé.**
    /// `demandeur_personne_id` : la personne pour qui l'on regarde (un adulte qui vient chercher l'enfant) ; sous
    /// interdiction, la réponse est la même que pour un élève protégé : « indisponible ».
    pub async fn edt_maintenant_eleve(
        &self,
        a: &Acteur<'_>,
        eleve_id: Uuid,
        usage: &str,
        demandeur_personne_id: Option<Uuid>,
        instant: Option<DateTime<Utc>>,
    ) -> Result<Maintenant, EntDbError> {
        self.exiger_edt(a, Action::EdtMaintenantLire).await?;
        if !USAGES_LOCALISATION.contains(&usage) {
            return Err(invalide());
        }
        let etab = a.etablissement_id;
        let instant = instant.unwrap_or_else(Utc::now);
        // L'élève existe dans l'établissement (sinon 404 : même réponse qu'une inconnue).
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let role = Role::parse(a.role).ok_or(EntDbError::Forbidden)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour: NaiveDate = sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(etab)
            .bind(instant)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        // Élève sous confidentialité renforcée : localisation réservée à la direction. Personne sous interdiction
        // (remise, visite, contact avec l'élève) : rien. Même réponse dans les deux cas.
        let protege: bool = sqlx::query_scalar("SELECT ent_eleve_confidentiel($1, $2, $3)")
            .bind(etab)
            .bind(eleve_id)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let interdit: bool = match demandeur_personne_id {
            Some(p) => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_eleve_interdiction i
                                 WHERE i.etablissement_id = $1 AND i.eleve_id = $2 AND i.personne_id = $3
                                   AND i.levee_le IS NULL AND i.du <= $4 AND (i.au IS NULL OR i.au >= $4)
                                   AND i.objet && ARRAY['remise', 'visite', 'contact_eleve']::text[])",
            )
            .bind(etab)
            .bind(eleve_id)
            .bind(p)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?,
            None => false,
        };
        let reponse = if interdit || (protege && role != Role::Direction) {
            Maintenant::Indisponible
        } else {
            #[derive(FromRow)]
            struct L {
                id: Uuid,
                cours_id: Option<Uuid>,
                matiere: String,
                classe: String,
                heure_debut: NaiveTime,
                heure_fin: NaiveTime,
                lieu_id: Option<Uuid>,
                intervenant: Uuid,
            }
            let s: Option<L> = sqlx::query_as(
                "SELECT s.id, s.cours_id, s.matiere_libelle AS matiere, s.classe, s.heure_debut, s.heure_fin,
                        (SELECT l.lieu_id FROM ent_edt_seance_lieu l WHERE l.seance_id = s.id
                          AND l.etablissement_id = s.etablissement_id ORDER BY l.principal DESC, l.lieu_id LIMIT 1) AS lieu_id,
                        coalesce(s.remplacant_user_id, s.prof_user_id) AS intervenant
                   FROM ent_edt_seance s
                  WHERE s.etablissement_id = $1 AND s.debut_utc <= $3 AND $3 < s.fin_utc
                    AND s.etat IN ('prevue', 'modifiee', 'ajoutee')
                    AND EXISTS (SELECT 1 FROM ent_public_cours_jour(s.etablissement_id, s.jour, s.cours_id, NULL, $2) p)
                  ORDER BY s.debut_utc LIMIT 1",
            )
            .bind(etab)
            .bind(eleve_id)
            .bind(instant)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match s {
                Some(l) => Maintenant::EnCours {
                    seance_id: l.id,
                    cours_id: l.cours_id,
                    matiere: l.matiere,
                    classe: l.classe,
                    heure_debut: l.heure_debut,
                    heure_fin: l.heure_fin,
                    lieu_id: l.lieu_id,
                    intervenant_user_id: l.intervenant,
                },
                None => Maintenant::PasDeCours,
            }
        };
        // Journal en ajout seul : la nature de la réponse, jamais le lieu.
        let nature = match &reponse {
            Maintenant::EnCours { .. } => "en_cours",
            Maintenant::PasDeCours => "pas_de_cours",
            Maintenant::Indisponible => "indisponible",
        };
        sqlx::query(
            "INSERT INTO ent_edt_localisation_journal (id, etablissement_id, lecteur_user_id, eleve_id, usage, reponse)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(a.user_id)
        .bind(eleve_id)
        .bind(usage)
        .bind(nature)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(reponse)
    }

    /// « Maintenant » pour une salle : son occupation (ce qui l'occupe, pas qui : D-058).
    pub async fn edt_maintenant_salle(
        &self,
        a: &Acteur<'_>,
        salle_id: Uuid,
        instant: Option<DateTime<Utc>>,
    ) -> Result<OccupationSalle, EntDbError> {
        self.exiger_edt(a, Action::EdtMaintenantLire).await?;
        let etab = a.etablissement_id;
        let instant = instant.unwrap_or_else(Utc::now);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_salle WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(salle_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let s: Option<(Uuid, String, NaiveTime, NaiveTime)> = sqlx::query_as(
            "SELECT s.id, s.classe, s.heure_debut, s.heure_fin
               FROM ent_edt_seance s
               JOIN ent_edt_seance_lieu l ON l.seance_id = s.id AND l.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1 AND l.lieu_id = $2 AND s.debut_utc <= $3 AND $3 < s.fin_utc
                AND s.etat IN ('prevue', 'modifiee', 'ajoutee')
              ORDER BY s.debut_utc LIMIT 1",
        )
        .bind(etab)
        .bind(salle_id)
        .bind(instant)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        Ok(match s {
            Some((id, classe, d, f)) => OccupationSalle {
                occupee: true,
                seance_id: Some(id),
                classe: Some(classe),
                heure_debut: Some(d),
                heure_fin: Some(f),
            },
            None => OccupationSalle {
                occupee: false,
                seance_id: None,
                classe: None,
                heure_debut: None,
                heure_fin: None,
            },
        })
    }

    /// « Maintenant » pour un professeur : présent dans l'établissement selon l'emploi du temps, oui ou non.
    pub async fn edt_maintenant_professeur(
        &self,
        a: &Acteur<'_>,
        user_id: Uuid,
        instant: Option<DateTime<Utc>>,
    ) -> Result<PresenceProfesseur, EntDbError> {
        self.exiger_edt(a, Action::EdtMaintenantLire).await?;
        let etab = a.etablissement_id;
        let instant = instant.unwrap_or_else(Utc::now);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let membre: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2)",
        )
        .bind(etab)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !membre {
            return Err(introuvable());
        }
        // Présent entre sa première et sa dernière séance active du jour de l'établissement.
        let present: bool = sqlx::query_scalar(
            "WITH j AS (SELECT ent_aujourdhui_etab($1, $3) AS jour)
             SELECT coalesce((SELECT min(s.debut_utc) <= $3 AND $3 < max(s.fin_utc)
                                FROM ent_edt_seance s, j
                               WHERE s.etablissement_id = $1 AND s.jour = j.jour
                                 AND coalesce(s.remplacant_user_id, s.prof_user_id) = $2
                                 AND s.etat IN ('prevue', 'modifiee', 'ajoutee')), false)",
        )
        .bind(etab)
        .bind(user_id)
        .bind(instant)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(PresenceProfesseur { present })
    }
}

/// Permanences attendues d'un jour : effectifs d'élèves sans cours par plage de séance inactive.
async fn permanences(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    jour: NaiveDate,
    _plage: Option<(NaiveTime, NaiveTime)>,
) -> Result<Vec<PermanenceAttendue>, EntDbError> {
    sqlx::query_as::<_, PermanenceAttendue>(
        "SELECT s.heure_debut, s.heure_fin, count(DISTINCT p.eleve_id) AS effectif
           FROM ent_edt_seance s
           CROSS JOIN LATERAL ent_public_cours_jour(s.etablissement_id, s.jour, s.cours_id) p
          WHERE s.etablissement_id = $1 AND s.jour = $2 AND s.cours_id IS NOT NULL
            AND s.etat IN ('annulee', 'non_assuree', 'deplacee')
            AND NOT EXISTS (
                SELECT 1 FROM ent_edt_seance o
                  JOIN LATERAL ent_public_cours_jour(o.etablissement_id, o.jour, o.cours_id) q ON q.eleve_id = p.eleve_id
                 WHERE o.etablissement_id = s.etablissement_id AND o.jour = s.jour AND o.id <> s.id
                   AND o.cours_id IS NOT NULL AND o.etat IN ('prevue', 'modifiee', 'ajoutee', 'neutralisee')
                   AND o.heure_debut < s.heure_fin AND s.heure_debut < o.heure_fin)
          GROUP BY s.heure_debut, s.heure_fin ORDER BY s.heure_debut, s.heure_fin",
    )
    .bind(etab)
    .bind(jour)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)
}

#[derive(Debug, Clone, Deserialize)]
pub struct DemainQuery {
    #[serde(default)]
    pub jour: Option<NaiveDate>,
}
