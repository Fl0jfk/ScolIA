//! Socle 20, lot C6 : **continuité** (`20_CAHIER_DE_TEXTES.md` §8.4, §12 ; CC-11 à CC-30, CC-69).
//!
//! * **Reports** (§8.4) : quand une séance est annulée, l'entrée qu'elle portait et les travaux qui l'attendaient donnent lieu à
//!   une **proposition**, jamais à un effet silencieux : rien ne disparaît, une échéance n'est jamais avancée sans accord
//!   explicite, un seul avis par changement effectif. Une proposition de travail non traitée à l'heure de la séance
//!   s'applique seule (S20-05), jamais en avançant l'échéance. Le socle 10 n'émet pas encore ses événements : les propositions
//!   se calculent sur l'état des séances, de façon idempotente ; le consommateur d'événements appellera la même fonction.
//! * **Continuité d'un élève** (absence longue, exclusion temporaire) : un bandeau « élève en continuité à distance » dit où,
//!   jamais pourquoi ; le travail pour lui seul est un travail à public restreint du cahier (C2), pas une punition.
//! * **Professeur absent, fermeture, poste vacant** : un travail déposé pour une séance précise, publié à l'heure de la séance ;
//!   la direction publie pour un cours sans intervenant en saisie (le cahier n'est pas « non tenu »).
//! * **Remplaçant d'une séance, suppléant** : lecture utile (cahier du cours, aménagements en pastilles, jamais de copies) ;
//!   décision sur les copies en attente à l'ouverture d'une suppléance : aucune ne reste sans décision (CC-69).

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use ent_auth::{Action, Portee, Role};
use ent_events::types;
use ent_types::cahier_c2::TravailC2Body;
use ent_types::cahier_c6::*;
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::cahier::publier_cdt;
use crate::cahier_c2::{conflit, invalide, miroir_entree};
use crate::cahier_c2_travaux::miroir_travail;
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Jours passés pendant lesquels une séance annulée donne encore lieu à une proposition.
const FENETRE_PASSE_JOURS: i64 = 60;
/// Propositions traitées montrées encore pendant ce nombre de jours.
const TRAITEES_VISIBLES_JOURS: i64 = 30;
/// Cours au plus d'une continuité.
const MAX_COURS_CONTINUITE: usize = 15;

fn systeme(etab: Uuid) -> Acteur<'static> {
    Acteur { etablissement_id: etab, user_id: Uuid::nil(), role: "systeme" }
}

#[derive(Debug, Clone, FromRow)]
struct ReportRow {
    id: Uuid,
    cours_id: Option<Uuid>,
    objet: String,
    objet_id: Uuid,
    origine_seance_id: Uuid,
    vers_seance_id: Option<Uuid>,
    etat: String,
    traite_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
struct SeanceMini {
    id: Uuid,
    jour: NaiveDate,
    heure_debut: NaiveTime,
    statut: String,
}

impl SeanceMini {
    fn dto(&self) -> SeanceReportDto {
        SeanceReportDto { seance_id: self.id, jour: self.jour, heure_debut: self.heure_debut.format("%H:%M").to_string() }
    }
}

async fn seance_mini(tx: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<SeanceMini, EntDbError> {
    sqlx::query_as("SELECT id, jour, heure_debut, statut FROM ent_edt_seance WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

impl PgEnt {
    /// Le compte écrit-il au niveau de l'établissement (direction, administration) ?
    async fn cdt_direction_ecrit(&self, a: &Acteur<'_>) -> Result<bool, EntDbError> {
        Ok(matches!(Role::parse(a.role), Some(Role::Admin | Role::Direction))
            && self
                .droits_effectifs(a.etablissement_id, a.user_id, a.role)
                .await?
                .iter()
                .any(|d| d.action == Action::CahierEcrire && d.portee == Portee::Etablissement))
    }

    /// Droit sur un cours : intervenant en saisie à la date, ou direction.
    async fn cdt_exiger_saisie_ou_direction(&self, a: &Acteur<'_>, cours: Uuid) -> Result<(), EntDbError> {
        if self.cdt_exiger_saisie(a, cours, None).await.is_ok() || self.cdt_direction_ecrit(a).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    // ───────────────────────────── propositions de report ─────────────────────────────

    /// CDT-147 : calcule les propositions de report des séances annulées (idempotent : une seule proposition par séance et par
    /// objet, même refusée) puis applique seules celles dont l'heure de séance est passée (CDT-150, S20-05).
    pub async fn cdt_reports_synchroniser(&self, etab: Uuid) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let crees: Vec<(Uuid,)> = sqlx::query_as(
            "WITH nouveaux AS (
                SELECT e.cours_id, s.id AS seance, 'entree'::text AS objet, e.id AS objet_id
                  FROM ent_cdt_entree e
                  JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
                 WHERE e.etablissement_id = $1 AND s.statut = 'annulee' AND e.etat IN ('brouillon', 'programmee', 'publiee')
                   AND s.jour >= ent_aujourdhui_etab($1) - $2::int
                UNION ALL
                SELECT t.cours_id, s.id, 'travail', t.id
                  FROM ent_cdt_travail t
                  JOIN ent_edt_seance s ON s.etablissement_id = t.etablissement_id AND s.id = t.echeance_seance_id
                 WHERE t.etablissement_id = $1 AND t.echeance_mode = 'seance' AND s.statut = 'annulee'
                   AND t.etat IN ('brouillon', 'programme', 'publie')
                   AND s.jour >= ent_aujourdhui_etab($1) - $2::int)
             INSERT INTO ent_cdt_report (id, etablissement_id, origine_seance_id, objet, objet_id, vers_seance_id, cours_id)
             SELECT gen_random_uuid(), $1, n.seance, n.objet, n.objet_id, ent_cdt_prochaine_seance($1, n.cours_id, n.seance), n.cours_id
               FROM nouveaux n
             ON CONFLICT (etablissement_id, origine_seance_id, objet, objet_id) DO NOTHING
             RETURNING id",
        )
        .bind(etab)
        .bind(FENETRE_PASSE_JOURS as i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if !crees.is_empty() {
            // Un événement par détection (une grève de 40 séances = une proposition groupée, un avis) ; identifiants seulement.
            publier_cdt(&mut tx, &systeme(etab), types::CAHIER_REPORT_PROPOSE, crees[0].0, None, json!({ "nb": crees.len() })).await?;
        }
        // Propositions de travail non traitées à l'heure de la séance : appliquées seules, jamais en avançant l'échéance.
        let echues: Vec<ReportRow> = sqlx::query_as(
            "SELECT r.id, r.cours_id, r.objet, r.objet_id, r.origine_seance_id, r.vers_seance_id, r.etat, r.traite_le
               FROM ent_cdt_report r JOIN ent_edt_seance o ON o.etablissement_id = r.etablissement_id AND o.id = r.origine_seance_id
              WHERE r.etablissement_id = $1 AND r.etat = 'proposee' AND r.objet = 'travail' AND r.vers_seance_id IS NOT NULL
                AND ent_cdt_instant_local(r.etablissement_id, o.jour, o.heure_debut) <= now()
              ORDER BY r.cree_le, r.id LIMIT 200 FOR UPDATE OF r SKIP LOCKED",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for r in echues {
            let Some(vers) = r.vers_seance_id else { continue };
            // Un échec isolé (échéance qui serait avancée) laisse la proposition ouverte pour le professeur.
            sqlx::query("SAVEPOINT report_auto").execute(&mut *tx).await.map_err(db)?;
            match self.appliquer_report(&mut tx, etab, &r, vers, None, true, false).await {
                Ok(()) => {
                    sqlx::query("RELEASE SAVEPOINT report_auto").execute(&mut *tx).await.map_err(db)?;
                }
                Err(_) => {
                    sqlx::query("ROLLBACK TO SAVEPOINT report_auto").execute(&mut *tx).await.map_err(db)?;
                }
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Applique un report : la séance d'arrivée est celle du cours, non annulée ; l'entrée ne s'écrase pas (une par séance) ;
    /// une échéance avancée demande une confirmation explicite et n'est jamais appliquée seule. Version gardée.
    #[allow(clippy::too_many_arguments)]
    async fn appliquer_report(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        r: &ReportRow,
        vers: Uuid,
        par: Option<Uuid>,
        auto: bool,
        confirmer_avance: bool,
    ) -> Result<(), EntDbError> {
        let cours = r.cours_id.ok_or_else(introuvable)?;
        // E1 : un report écrit sur le cahier ; une année figée n'en reçoit plus (la base refuse de toute façon).
        self.cdt_exiger_cours_ouvert_tx(tx, etab, cours).await?;
        let cible = seance_mini(tx, etab, vers).await?;
        if cible.statut == "annulee" {
            return Err(conflit("la séance d'arrivée est annulée"));
        }
        let (appartient,): (bool,) = sqlx::query_as("SELECT ent_cdt_cours_de_seance($1, $2) = $3")
            .bind(etab)
            .bind(vers)
            .bind(cours)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if !appartient {
            return Err(invalide("la séance d'arrivée n'est pas une séance de ce cours"));
        }
        match r.objet.as_str() {
            REPORT_TRAVAIL => {
                let (nouvelle,): (DateTime<Utc>,) = sqlx::query_as("SELECT ent_cdt_instant_local($1, $2, $3)")
                    .bind(etab)
                    .bind(cible.jour)
                    .bind(cible.heure_debut)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                let (ancienne, etat): (DateTime<Utc>, String) =
                    sqlx::query_as("SELECT echeance_le, etat FROM ent_cdt_travail WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
                        .bind(etab)
                        .bind(r.objet_id)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(db)?
                        .ok_or_else(introuvable)?;
                if etat == "retire" {
                    return Err(conflit("ce travail a été retiré de la diffusion"));
                }
                // Jamais une échéance avancée sans accord explicite ; jamais seule.
                if nouvelle < ancienne && (auto || !confirmer_avance) {
                    return Err(conflit("ce report avancerait l'échéance : confirmez-le explicitement"));
                }
                sqlx::query(
                    "INSERT INTO ent_cdt_travail_version (id, etablissement_id, travail_id, version, titre, consigne, duree_estimee_min,
                            echeance_le, etat, auteur_user_id, auteur_agent_id, motif, sans_avis)
                     SELECT gen_random_uuid(), t.etablissement_id, t.id, t.version, t.titre, t.consigne, t.duree_estimee_min, t.echeance_le,
                            t.etat, t.auteur_user_id, t.auteur_agent_id, 'report : séance annulée', false
                       FROM ent_cdt_travail t WHERE t.etablissement_id = $1 AND t.id = $2",
                )
                .bind(etab)
                .bind(r.objet_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                sqlx::query(
                    "UPDATE ent_cdt_travail SET echeance_seance_id = $3, echeance_le = $4, version = version + 1, modifie_le = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(r.objet_id)
                .bind(vers)
                .bind(nouvelle)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                miroir_travail(tx, etab, r.objet_id).await?;
            }
            REPORT_ENTREE => {
                let deja: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_cdt_entree WHERE etablissement_id = $1 AND cours_id = $2 AND seance_id = $3)",
                )
                .bind(etab)
                .bind(cours)
                .bind(vers)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if deja {
                    return Err(conflit("la séance d'arrivée a déjà une entrée : rien n'est écrasé"));
                }
                sqlx::query(
                    "INSERT INTO ent_cdt_entree_version (id, etablissement_id, entree_id, version, titre, contenu, etat, auteur_user_id,
                            auteur_agent_id, motif, sans_avis)
                     SELECT gen_random_uuid(), e.etablissement_id, e.id, e.version, e.titre, e.contenu, e.etat, e.auteur_user_id,
                            e.auteur_agent_id, 'report : séance annulée', false
                       FROM ent_cdt_entree e WHERE e.etablissement_id = $1 AND e.id = $2",
                )
                .bind(etab)
                .bind(r.objet_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                sqlx::query("UPDATE ent_cdt_entree SET seance_id = $3, version = version + 1, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(r.objet_id)
                    .bind(vers)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                // Ancienne ligne de compatibilité : suit la séance d'arrivée quand celle-ci n'en porte pas déjà une.
                sqlx::query(
                    "UPDATE ent_seance_contenu o SET seance_id = $3, updated_at = now()
                      FROM ent_cdt_entree e
                     WHERE e.etablissement_id = $1 AND e.id = $2 AND o.etablissement_id = $1 AND o.id = COALESCE(e.ancien_id, e.id)
                       AND NOT EXISTS (SELECT 1 FROM ent_seance_contenu x WHERE x.etablissement_id = $1 AND x.seance_id = $3)",
                )
                .bind(etab)
                .bind(r.objet_id)
                .bind(vers)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                let _ = miroir_entree(tx, etab, r.objet_id).await?;
            }
            _ => return Err(invalide("objet de report inconnu")),
        }
        sqlx::query(
            "UPDATE ent_cdt_report SET etat = $3, vers_seance_id = $4, traite_par_user_id = $5, traite_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'proposee'",
        )
        .bind(etab)
        .bind(r.id)
        .bind(if auto { "appliquee_auto" } else { "appliquee" })
        .bind(vers)
        .bind(par)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Un seul avis par changement effectif, identifiants seulement.
        publier_cdt(
            tx,
            &Acteur { etablissement_id: etab, user_id: par.unwrap_or_default(), role: if auto { "systeme" } else { "prof" } },
            types::CAHIER_REPORT_APPLIQUE,
            r.objet_id,
            None,
            json!({ "report_id": r.id, "objet": r.objet, "auto": auto }),
        )
        .await?;
        Ok(())
    }

    /// CDT-147, 149 : les propositions du professeur (ou de la direction), groupées sur un seul écran.
    pub async fn cdt_reports_lister(&self, a: &Acteur<'_>) -> Result<ReportsDto, EntDbError> {
        let etab = a.etablissement_id;
        let direction = self.cdt_direction_ecrit(a).await?;
        if !direction && !self.cahier_droit_ecrire_global(a).await? {
            return Err(EntDbError::Forbidden);
        }
        self.cdt_reports_synchroniser(etab).await?;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<ReportRow> = sqlx::query_as(
            "SELECT r.id, r.cours_id, r.objet, r.objet_id, r.origine_seance_id, r.vers_seance_id, r.etat, r.traite_le
               FROM ent_cdt_report r JOIN ent_edt_seance o ON o.etablissement_id = r.etablissement_id AND o.id = r.origine_seance_id
              WHERE r.etablissement_id = $1
                AND (r.etat = 'proposee' OR r.traite_le >= now() - make_interval(days => $4))
                AND ($5 OR ent_cdt_saisie_cours(r.etablissement_id, r.cours_id, $2, $3))
              ORDER BY o.jour, o.heure_debut, r.cree_le, r.id",
        )
        .bind(etab)
        .bind(a.user_id)
        .bind(jour)
        .bind(TRAITEES_VISIBLES_JOURS as i32)
        .bind(direction)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut propositions = Vec::new();
        for r in &lignes {
            propositions.push(self.report_dto(&mut tx, etab, r).await?);
        }
        let nb_ouvertes = propositions.iter().filter(|p| p.etat == "proposee").count() as i32;
        Ok(ReportsDto { propositions, nb_ouvertes })
    }

    async fn report_dto(&self, tx: &mut PgConnection, etab: Uuid, r: &ReportRow) -> Result<ReportDto, EntDbError> {
        let origine = seance_mini(tx, etab, r.origine_seance_id).await?;
        let vers = match r.vers_seance_id {
            Some(v) => Some(seance_mini(tx, etab, v).await?),
            None => None,
        };
        let titre: String = match r.objet.as_str() {
            REPORT_TRAVAIL => sqlx::query_scalar("SELECT titre FROM ent_cdt_travail WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(r.objet_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?,
            _ => sqlx::query_scalar("SELECT COALESCE(NULLIF(btrim(titre), ''), left(contenu, 80)) FROM ent_cdt_entree WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(r.objet_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?,
        };
        let avance = match (&vers, r.objet.as_str()) {
            (Some(v), REPORT_TRAVAIL) => {
                let (nouvelle, ancienne): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
                    "SELECT ent_cdt_instant_local($1, $3, $4), (SELECT echeance_le FROM ent_cdt_travail WHERE etablissement_id = $1 AND id = $2)",
                )
                .bind(etab)
                .bind(r.objet_id)
                .bind(v.jour)
                .bind(v.heure_debut)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                nouvelle < ancienne
            }
            _ => false,
        };
        Ok(ReportDto {
            id: r.id,
            cours_id: r.cours_id.unwrap_or_default(),
            objet: r.objet.clone(),
            objet_id: r.objet_id,
            titre,
            origine: origine.dto(),
            vers: vers.map(|v| v.dto()),
            etat: r.etat.clone(),
            avance_l_echeance: avance,
            traite_le: r.traite_le,
        })
    }

    /// CDT-148 : accepter (avec la séance proposée ou une autre), modifier ou refuser une proposition.
    pub async fn cdt_report_traiter(&self, a: &Acteur<'_>, id: Uuid, body: ReportDecisionBody) -> Result<ReportDto, EntDbError> {
        let etab = a.etablissement_id;
        if !["accepter", "refuser"].contains(&body.decision.as_str()) {
            return Err(invalide("décision : accepter ou refuser"));
        }
        let motif = body.motif.as_deref().map(str::trim).filter(|m| !m.is_empty()).map(str::to_string);
        if motif.as_ref().is_some_and(|m| m.chars().count() > 500) {
            return Err(invalide("motif trop long"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: ReportRow = sqlx::query_as(
            "SELECT id, cours_id, objet, objet_id, origine_seance_id, vers_seance_id, etat, traite_le FROM ent_cdt_report
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let cours = r.cours_id.ok_or_else(introuvable)?;
        drop(tx);
        self.cdt_exiger_saisie_ou_direction(a, cours).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: ReportRow = sqlx::query_as(
            "SELECT id, cours_id, objet, objet_id, origine_seance_id, vers_seance_id, etat, traite_le FROM ent_cdt_report
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if r.etat != "proposee" {
            return Err(conflit("cette proposition a déjà été traitée"));
        }
        if body.decision == "refuser" {
            // Refus : l'échéance reste, le contenu reste ; motif facultatif.
            sqlx::query("UPDATE ent_cdt_report SET etat = 'refusee', motif = $3, traite_par_user_id = $4, traite_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .bind(&motif)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        } else {
            let vers = body.vers_seance_id.or(r.vers_seance_id).ok_or_else(|| invalide("aucune séance d'arrivée : indiquez-en une"))?;
            self.appliquer_report(&mut tx, etab, &r, vers, Some(a.user_id), false, body.confirmer_avance).await?;
        }
        let r2: ReportRow = sqlx::query_as(
            "SELECT id, cours_id, objet, objet_id, origine_seance_id, vers_seance_id, etat, traite_le FROM ent_cdt_report WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let dto = self.report_dto(&mut tx, etab, &r2).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// CDT-149 : traiter en lot (grève, fermeture) : même décision sur plusieurs propositions ; ce qui ne peut pas être appliqué
    /// (échéance avancée, séance d'arrivée absente, accès) reste ouvert, avec sa raison.
    pub async fn cdt_reports_lot(&self, a: &Acteur<'_>, body: ReportsLotBody) -> Result<ReportsLotDto, EntDbError> {
        // Garde d'entrée comme pour une proposition seule : un rôle sans droit d'écriture sur le cahier est refusé (403) avant
        // tout traitement, pas par identifiant « ignoré ».
        if !(self.cahier_droit_ecrire_global(a).await? || self.cdt_direction_ecrit(a).await?) {
            return Err(EntDbError::Forbidden);
        }
        if body.ids.is_empty() || body.ids.len() > 400 {
            return Err(invalide("de 1 à 400 propositions à la fois"));
        }
        let mut sortie = ReportsLotDto { appliques: 0, refuses: 0, ignores: vec![] };
        let vus: HashSet<Uuid> = body.ids.iter().copied().collect();
        for id in vus {
            let r = self
                .cdt_report_traiter(a, id, ReportDecisionBody { decision: body.decision.clone(), vers_seance_id: None, confirmer_avance: false, motif: None })
                .await;
            match r {
                Ok(_) if body.decision == "refuser" => sortie.refuses += 1,
                Ok(_) => sortie.appliques += 1,
                Err(EntDbError::Forbidden) => sortie.ignores.push(ReportIgnoreDto { id, raison: "accès refusé".into() }),
                Err(EntDbError::Refus(_, m)) => sortie.ignores.push(ReportIgnoreDto { id, raison: m.to_string() }),
                Err(EntDbError::Domain(_)) => sortie.ignores.push(ReportIgnoreDto { id, raison: "proposition introuvable".into() }),
                Err(e) => return Err(e),
            }
        }
        Ok(sortie)
    }

    // ───────────────────────────── continuité d'un élève ─────────────────────────────

    /// L'acteur est-il habilité à ouvrir ou clore une continuité pour cet élève ? Absence longue : PP de sa classe, CPE,
    /// direction ; exclusion temporaire : vie scolaire (CPE) et direction seulement.
    async fn cdt_peut_continuite(&self, a: &Acteur<'_>, eleve: Uuid, nature: &str, jour: NaiveDate) -> Result<bool, EntDbError> {
        let role = Role::parse(a.role);
        if matches!(role, Some(Role::Admin | Role::Direction)) {
            return Ok(true);
        }
        if role == Some(Role::Cpe) {
            return Ok(self.droits_effectifs(a.etablissement_id, a.user_id, a.role).await?.iter().any(|d| d.action == Action::CahierLire));
        }
        if nature == CONTINUITE_EXCLUSION || role != Some(Role::Prof) {
            return Ok(false);
        }
        let pp: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM ent_service_enseignement s
                  JOIN ent_affectation af ON af.etablissement_id = s.etablissement_id AND af.classe_id = s.classe_id
                 WHERE s.etablissement_id = $1 AND s.professeur_principal AND af.eleve_id = $2
                   AND af.date_debut <= $4 AND (af.date_fin IS NULL OR af.date_fin >= $4)
                   AND (s.prof_user_id = $3
                        OR EXISTS (SELECT 1 FROM ent_service_intervenants_a($1, s.id, $4) x
                                     JOIN ent_agent ag ON ag.etablissement_id = $1 AND ag.id = x.agent_id
                                     JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                                    WHERE pe.user_id = $3 AND x.nature = 'titulaire')))",
        )
        .bind(a.etablissement_id)
        .bind(eleve)
        .bind(a.user_id)
        .bind(jour)
        .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        Ok(pp)
    }

    /// CDT-205, 208 : ouvre une continuité pour un élève (période, cours). Aucun motif n'est stocké ni servi.
    pub async fn cdt_continuite_ouvrir(&self, a: &Acteur<'_>, body: ContinuiteBody) -> Result<ContinuiteDto, EntDbError> {
        let etab = a.etablissement_id;
        if ![CONTINUITE_ABSENCE_LONGUE, CONTINUITE_EXCLUSION].contains(&body.nature.as_str()) {
            return Err(invalide("nature : absence_longue ou exclusion"));
        }
        if body.au.is_some_and(|au| au < body.du) {
            return Err(invalide("la fin précède le début"));
        }
        if body.cours_ids.is_empty() || body.cours_ids.len() > MAX_COURS_CONTINUITE {
            return Err(invalide("indiquez de 1 à 15 cours"));
        }
        let mut cours = body.cours_ids.clone();
        cours.sort();
        cours.dedup();
        let jour = self.jour_etab(etab).await?;
        if body.du < jour - Duration::days(60) || body.du > jour + Duration::days(365) {
            return Err(invalide("début de continuité hors des bornes (60 jours passés, un an à venir)"));
        }
        if !self.cdt_peut_continuite(a, body.eleve_id, &body.nature, jour).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (nom, prenom): (String, String) = sqlx::query_as("SELECT nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(body.eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        // Les cours indiqués sont ceux de l'élève au début de la période.
        let n: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT c) FROM unnest($2::uuid[]) AS u(c)
              WHERE EXISTS (SELECT 1 FROM ent_public_cours_jour($1, GREATEST($4, $5), c, NULL, $3, false))",
        )
        .bind(etab)
        .bind(&cours)
        .bind(body.eleve_id)
        .bind(body.du)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n as usize != cours.len() {
            return Err(invalide("un cours indiqué n'est pas un cours de cet élève"));
        }
        let chevauche: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_cdt_continuite c JOIN ent_cdt_continuite_cours cc ON cc.etablissement_id = c.etablissement_id AND cc.continuite_id = c.id
              WHERE c.etablissement_id = $1 AND c.eleve_id = $2 AND c.close_par_user_id IS NULL AND cc.cours_id = ANY($3)
                AND c.du <= COALESCE($5, 'infinity'::date) AND COALESCE(c.au, 'infinity'::date) >= $4)",
        )
        .bind(etab)
        .bind(body.eleve_id)
        .bind(&cours)
        .bind(body.du)
        .bind(body.au)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if chevauche {
            return Err(conflit("une continuité est déjà ouverte pour cet élève sur ces cours à ces dates"));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_cdt_continuite (id, etablissement_id, eleve_id, du, au, ouverte_par_user_id, nature) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(id)
            .bind(etab)
            .bind(body.eleve_id)
            .bind(body.du)
            .bind(body.au)
            .bind(a.user_id)
            .bind(&body.nature)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        for c in &cours {
            sqlx::query("INSERT INTO ent_cdt_continuite_cours (etablissement_id, continuite_id, cours_id) VALUES ($1, $2, $3)")
                .bind(etab)
                .bind(id)
                .bind(c)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        publier_cdt(&mut tx, a, types::CAHIER_CONTINUITE_OUVERTE, id, Some(body.eleve_id), json!({ "continuite_id": id, "nb_cours": cours.len() })).await?;
        tx.commit().await.map_err(db)?;
        Ok(ContinuiteDto {
            id,
            eleve_id: body.eleve_id,
            nom,
            prenom,
            du: body.du,
            au: body.au,
            cours_ids: cours,
            nature: Some(body.nature),
            close: false,
        })
    }

    /// CDT-207 : clore une continuité à une date.
    pub async fn cdt_continuite_clore(&self, a: &Acteur<'_>, id: Uuid, body: ContinuiteCloreBody) -> Result<(), EntDbError> {
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (eleve, du, nature, close): (Uuid, NaiveDate, String, bool) = sqlx::query_as(
            "SELECT eleve_id, du, nature, close_par_user_id IS NOT NULL FROM ent_cdt_continuite WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        drop(tx);
        if !self.cdt_peut_continuite(a, eleve, &nature, jour).await? {
            return Err(EntDbError::Forbidden);
        }
        if close {
            return Err(conflit("cette continuité est déjà close"));
        }
        if body.au < du {
            return Err(invalide("la fin précède le début"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("UPDATE ent_cdt_continuite SET au = $3, close_par_user_id = $4 WHERE etablissement_id = $1 AND id = $2 AND close_par_user_id IS NULL")
            .bind(etab)
            .bind(id)
            .bind(body.au)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        publier_cdt(&mut tx, a, types::CAHIER_CONTINUITE_CLOSE, id, Some(eleve), json!({ "continuite_id": id })).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Continuités ouvertes ou récentes que l'acteur peut gérer. La nature (exclusion) n'est servie qu'à la vie scolaire.
    pub async fn cdt_continuites_lister(&self, a: &Acteur<'_>) -> Result<Vec<ContinuiteDto>, EntDbError> {
        let etab = a.etablissement_id;
        let role = Role::parse(a.role);
        let vie_scolaire = matches!(role, Some(Role::Admin | Role::Direction | Role::Cpe));
        if !vie_scolaire && role != Some(Role::Prof) {
            return Err(EntDbError::Forbidden);
        }
        let jour = self.jour_etab(etab).await?;
        let lignes: Vec<(Uuid, Uuid, String, String, NaiveDate, Option<NaiveDate>, String, bool)> = sqlx::query_as(
            "SELECT c.id, c.eleve_id, e.nom, e.prenom, c.du, c.au, c.nature, c.close_par_user_id IS NOT NULL
               FROM ent_cdt_continuite c JOIN ent_eleve e ON e.etablissement_id = c.etablissement_id AND e.id = c.eleve_id
              WHERE c.etablissement_id = $1 AND (c.au IS NULL OR c.au >= $2 - 30)
                AND ($3 OR EXISTS (SELECT 1 FROM ent_service_enseignement s JOIN ent_affectation af ON af.etablissement_id = s.etablissement_id AND af.classe_id = s.classe_id
                                    WHERE s.etablissement_id = c.etablissement_id AND s.professeur_principal AND af.eleve_id = c.eleve_id
                                      AND af.date_debut <= $2 AND (af.date_fin IS NULL OR af.date_fin >= $2) AND s.prof_user_id = $4))
              ORDER BY c.du DESC, c.id",
        )
        .bind(etab)
        .bind(jour)
        .bind(vie_scolaire)
        .bind(a.user_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        for (id, eleve_id, nom, prenom, du, au, nature, close) in lignes {
            let cours: Vec<Uuid> = sqlx::query_scalar("SELECT cours_id FROM ent_cdt_continuite_cours WHERE etablissement_id = $1 AND continuite_id = $2 ORDER BY cours_id")
                .bind(etab)
                .bind(id)
                .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
                .await
                .map_err(db)?;
            sortie.push(ContinuiteDto { id, eleve_id, nom, prenom, du, au, cours_ids: cours, nature: vie_scolaire.then_some(nature), close });
        }
        Ok(sortie)
    }

    /// Bandeau d'un cours (professeur en saisie, ou direction) : élèves en continuité à distance, **sans motif** ; poste vacant ;
    /// suppléance en cours ; copies en attente de décision.
    pub async fn cdt_cours_bandeau(&self, a: &Acteur<'_>, cours: Uuid) -> Result<BandeauCoursDto, EntDbError> {
        self.cdt_exiger_saisie_ou_direction(a, cours).await?;
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // La direction passe le contrôle de saisie sans cours précis : un cours d'un autre établissement (ou inexistant) est
        // introuvable, jamais un bandeau vide en 200.
        let existe: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_cours WHERE etablissement_id = $1 AND id = $2)")
            .bind(etab)
            .bind(cours)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if !existe {
            return Err(EntDbError::Domain(domain_eleve::DomainError::NotFound));
        }
        let eleves: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT DISTINCT e.id, e.nom, e.prenom FROM ent_cdt_continuite c
               JOIN ent_cdt_continuite_cours cc ON cc.etablissement_id = c.etablissement_id AND cc.continuite_id = c.id
               JOIN ent_eleve e ON e.etablissement_id = c.etablissement_id AND e.id = c.eleve_id
              WHERE c.etablissement_id = $1 AND cc.cours_id = $2 AND c.close_par_user_id IS NULL
                AND c.du <= $3 AND (c.au IS NULL OR c.au >= $3)
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(cours)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let vacant: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_services_vacants($1, $3) v JOIN ent_service_enseignement s ON s.etablissement_id = $1 AND s.id = v.service_id WHERE s.cours_id = $2)",
        )
        .bind(etab)
        .bind(cours)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let suppleance: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT i.du FROM ent_service_intervenant i JOIN ent_service_enseignement s ON s.etablissement_id = i.etablissement_id AND s.id = i.service_id
              WHERE i.etablissement_id = $1 AND s.cours_id = $2 AND i.nature = 'suppleant' AND i.annule_le IS NULL AND i.saisie
                AND i.du <= $3 AND (i.au IS NULL OR i.au >= $3) ORDER BY i.du DESC LIMIT 1",
        )
        .bind(etab)
        .bind(cours)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let a_decider = match suppleance {
            Some(du) => {
                let deja: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_cdt_copies_decision WHERE etablissement_id = $1 AND cours_id = $2
                                      AND cree_le >= ent_cdt_instant_local($1, $3, '00:00'))",
                )
                .bind(etab)
                .bind(cours)
                .bind(du)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if deja { 0 } else { copies_en_attente(&mut tx, etab, cours).await? }
            }
            None => 0,
        };
        Ok(BandeauCoursDto {
            cours_id: cours,
            eleves_en_continuite: eleves.into_iter().map(|(eleve_id, nom, prenom)| EleveContinuiteDto { eleve_id, nom, prenom }).collect(),
            poste_vacant: vacant,
            suppleance_en_cours: suppleance.is_some(),
            copies_a_decider: a_decider,
        })
    }

    // ───────────────────────────── poste vacant, absence, fermeture ─────────────────────────────

    /// CDT-209 : les cours sans intervenant en saisie aujourd'hui (poste vacant) : la direction y publie un travail.
    pub async fn cdt_vacants_lister(&self, a: &Acteur<'_>) -> Result<VacantsDto, EntDbError> {
        if !self.cdt_direction_ecrit(a).await? {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let lignes: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT c.id, c.libelle, COALESCE(cl.code, g.code, '')
               FROM ent_services_vacants($1, $2) v
               JOIN ent_service_enseignement s ON s.etablissement_id = $1 AND s.id = v.service_id
               JOIN ent_cours c ON c.etablissement_id = $1 AND c.id = s.cours_id
               LEFT JOIN ent_classe cl ON cl.etablissement_id = $1 AND cl.id = c.classe_id
               LEFT JOIN ent_groupe g ON g.etablissement_id = $1 AND g.id = c.groupe_id
              ORDER BY 3, 2, 1",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(VacantsDto { cours: lignes.into_iter().map(|(cours_id, libelle, cible)| CoursVacantDto { cours_id, libelle, cible }).collect() })
    }

    /// CDT-209 : la direction publie un travail pour un ou plusieurs cours **vacants** (jamais pour un cours dont un
    /// intervenant est en saisie). Le travail porte le contexte « poste vacant ».
    pub async fn cdt_vacant_publier(&self, a: &Acteur<'_>, body: TravailC2Body) -> Result<Vec<Uuid>, EntDbError> {
        if !self.cdt_direction_ecrit(a).await? {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        for c in &body.cours_ids {
            let vacant: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_services_vacants($1, $3) v JOIN ent_service_enseignement s ON s.etablissement_id = $1 AND s.id = v.service_id WHERE s.cours_id = $2)",
            )
            .bind(etab)
            .bind(c)
            .bind(jour)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?;
            if !vacant {
                return Err(conflit("ce cours a un intervenant en saisie : le poste n'est pas vacant"));
            }
        }
        let crees = self.cdt_travaux_creer_avec(a, body, true).await?;
        let ids: Vec<Uuid> = crees.travaux.iter().map(|t| t.id).collect();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("UPDATE ent_cdt_travail SET contexte = 'poste_vacant' WHERE etablissement_id = $1 AND id = ANY($2)")
            .bind(etab)
            .bind(&ids)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(ids)
    }

    /// CDT-?? §12.2 et §12.3 : travail déposé pour **une séance** précise — « en son absence » (titulaire ou direction) ou
    /// « à distance » (séance annulée par une fermeture) — publié à l'heure de la séance ; échéance : la séance suivante du cours.
    pub async fn cdt_travail_seance(&self, a: &Acteur<'_>, seance_id: Uuid, body: TravailSeanceBody) -> Result<TravailSeanceDto, EntDbError> {
        let etab = a.etablissement_id;
        if ![CONTEXTE_ABSENCE, CONTEXTE_DISTANCE].contains(&body.contexte.as_str()) {
            return Err(invalide("contexte : absence_professeur ou distance"));
        }
        let seance = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            seance_mini(&mut tx, etab, seance_id).await?
        };
        let cours: Uuid = sqlx::query_scalar::<_, Option<Uuid>>("SELECT ent_cdt_cours_de_seance($1, $2)")
            .bind(etab)
            .bind(seance_id)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?
            .ok_or_else(|| conflit("cette séance n'est rattachée à aucun cours"))?;
        self.cdt_exiger_saisie_ou_direction(a, cours).await?;
        let jour = self.jour_etab(etab).await?;
        if seance.jour < jour {
            return Err(conflit("cette séance est passée : utilisez le cahier du cours"));
        }
        match (body.contexte.as_str(), seance.statut.as_str()) {
            (CONTEXTE_DISTANCE, "annulee") | (CONTEXTE_ABSENCE, "prevue") | (CONTEXTE_ABSENCE, "remplacee") => {}
            (CONTEXTE_DISTANCE, _) => return Err(conflit("un travail à distance se dépose pour une séance annulée")),
            _ => return Err(conflit("cette séance est annulée : déposez un travail à distance")),
        }
        let suivante: Option<Uuid> = sqlx::query_scalar("SELECT ent_cdt_prochaine_seance($1, $2, $3)")
            .bind(etab)
            .bind(cours)
            .bind(seance_id)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?;
        let Some(suivante) = suivante else {
            return Err(conflit("aucune séance ultérieure du cours : l'échéance ne peut pas être posée"));
        };
        let corps = TravailC2Body {
            cours_ids: vec![cours],
            seance_donnee_id: None,
            echeance_seance_id: Some(suivante),
            echeance_prochaine_seance: false,
            echeance_jour: None,
            echeance_heure: None,
            nature: "travail".into(),
            titre: body.titre,
            consigne: body.consigne,
            duree_estimee_min: body.duree_estimee_min,
            rendu: "aucun".into(),
            retard_accepte: true,
            formats: vec![],
            eleve_ids: vec![],
            ia_autorisee: "non".into(),
            publication: "programmer".into(),
            publie_a: None,
            publie_jour: Some(seance.jour),
            publie_heure: Some(seance.heure_debut),
        };
        let crees = self.cdt_travaux_creer_avec(a, corps, true).await?;
        let t = crees.travaux.first().ok_or_else(introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("UPDATE ent_cdt_travail SET contexte = $3, seance_speciale_id = $4 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(t.id)
            .bind(&body.contexte)
            .bind(seance_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let visible_a: Option<DateTime<Utc>> = sqlx::query_scalar("SELECT visible_a FROM ent_cdt_travail WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(t.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        publier_cdt(&mut tx, a, types::CAHIER_TRAVAIL_ABSENCE_DEPOSE, t.id, None, json!({ "travail_id": t.id, "seance_id": seance_id, "contexte": body.contexte })).await?;
        tx.commit().await.map_err(db)?;
        Ok(TravailSeanceDto { travail_id: t.id, seance_id, visible_a, contexte: body.contexte })
    }

    // ───────────────────────────── remplaçant, suppléant ─────────────────────────────

    /// CDT-203 : vue du remplaçant désigné d'une séance : le cahier du cours (entrées publiées passées), les élèves avec les
    /// seuls aménagements utiles (pastilles). Ni copies, ni retours. Plus aucun droit le lendemain (CC-23).
    pub async fn cdt_remplacement_vue(&self, a: &Acteur<'_>, seance_id: Uuid) -> Result<RemplacementVueDto, EntDbError> {
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let (seance, remplacant): (SeanceMini, Option<Uuid>) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let s = seance_mini(&mut tx, etab, seance_id).await?;
            let r: Option<Uuid> = sqlx::query_scalar("SELECT remplacant_user_id FROM ent_edt_seance WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(seance_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            (s, r)
        };
        if remplacant != Some(a.user_id) || seance.statut == "annulee" || seance.jour < jour || !self.cahier_droit_ecrire_global(a).await? {
            return Err(EntDbError::Forbidden);
        }
        let cours: Uuid = sqlx::query_scalar::<_, Option<Uuid>>("SELECT ent_cdt_cours_de_seance($1, $2)")
            .bind(etab)
            .bind(seance_id)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(|_| EntDbError::Forbidden)?
            .ok_or(EntDbError::Forbidden)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let entrees: Vec<(NaiveDate, NaiveTime, Uuid, Option<String>, String)> = sqlx::query_as(
            "SELECT s.jour, s.heure_debut, s.id, e.titre, e.contenu FROM ent_cdt_entree e
               JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
              WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.etat = 'publiee' AND e.visible_a <= now() AND s.jour <= $3
              ORDER BY s.jour DESC, s.heure_debut DESC LIMIT 60",
        )
        .bind(etab)
        .bind(cours)
        .bind(seance.jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let eleves: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT e.id, e.nom, e.prenom FROM ent_public_cours_jour($1, $3, $2, NULL, NULL, false) p
               JOIN ent_eleve e ON e.etablissement_id = $1 AND e.id = p.eleve_id ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(cours)
        .bind(seance.jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        drop(tx);
        let ids: Vec<Uuid> = eleves.iter().map(|e| e.0).collect();
        let mut pastilles = self.cdt_amenagements_utiles(etab, &ids, seance.jour).await?;
        Ok(RemplacementVueDto {
            seance: seance.dto(),
            cours_id: cours,
            entrees: entrees
                .into_iter()
                .map(|(jour, h, sid, titre, contenu)| EntreeRemplacementDto {
                    seance: SeanceReportDto { seance_id: sid, jour, heure_debut: h.format("%H:%M").to_string() },
                    titre,
                    contenu,
                })
                .collect(),
            eleves: eleves
                .into_iter()
                .map(|(eleve_id, nom, prenom)| ElevePastilleDto {
                    eleve_id,
                    nom,
                    prenom,
                    pastilles: pastilles.remove(&eleve_id).unwrap_or_default().iter().map(|p| p.pastille()).collect(),
                })
                .collect(),
        })
    }

    /// Le compte est-il suppléant nommé de ce cours (même pas encore en poste) ? Renvoie (début, saisie ouverte aujourd'hui).
    async fn suppleance_de(&self, a: &Acteur<'_>, cours: Uuid, jour: NaiveDate) -> Result<Option<(NaiveDate, bool)>, EntDbError> {
        let etab = a.etablissement_id;
        sqlx::query_as(
            "SELECT i.du, (i.saisie AND i.du <= $4 AND (i.au IS NULL OR i.au >= $4) AND (i.droits IS NULL OR 'cahier' = ANY(i.droits)))
               FROM ent_service_intervenant i
               JOIN ent_service_enseignement s ON s.etablissement_id = i.etablissement_id AND s.id = i.service_id
               JOIN ent_agent ag ON ag.etablissement_id = i.etablissement_id AND ag.id = i.agent_id
               JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
              WHERE i.etablissement_id = $1 AND s.cours_id = $2 AND pe.user_id = $3 AND i.nature = 'suppleant'
                AND i.annule_le IS NULL AND (i.au IS NULL OR i.au >= $4)
              ORDER BY i.du LIMIT 1",
        )
        .bind(etab)
        .bind(cours)
        .bind(a.user_id)
        .bind(jour)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }

    /// CDT-202, 255 : « où en était le cours » — le suppléant nommé lit le cahier dès sa nomination (lecture seule, pas de
    /// copies) ; à sa prise de poste la saisie s'ouvre et le nombre de copies en attente apparaît.
    pub async fn cdt_suppleance_accueil(&self, a: &Acteur<'_>, cours: Uuid) -> Result<SuppleanceAccueilDto, EntDbError> {
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let Some((_du, saisie)) = self.suppleance_de(a, cours, jour).await? else {
            return Err(EntDbError::Forbidden);
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let entrees: Vec<(NaiveDate, NaiveTime, Uuid, Option<String>, String)> = sqlx::query_as(
            "SELECT s.jour, s.heure_debut, s.id, e.titre, e.contenu FROM ent_cdt_entree e
               JOIN ent_edt_seance s ON s.etablissement_id = e.etablissement_id AND s.id = e.seance_id
              WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.etat = 'publiee' AND e.visible_a <= now()
              ORDER BY s.jour DESC, s.heure_debut DESC LIMIT 10",
        )
        .bind(etab)
        .bind(cours)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let travaux: Vec<(Uuid, String, NaiveDate, i64)> = sqlx::query_as(
            "SELECT t.id, t.titre, ent_aujourdhui_etab(t.etablissement_id, t.echeance_le),
                    (SELECT count(*) FROM ent_cdt_remise r WHERE r.etablissement_id = t.etablissement_id AND r.travail_id = t.id AND r.dernier_depot_id IS NOT NULL)
               FROM ent_cdt_travail t
              WHERE t.etablissement_id = $1 AND t.cours_id = $2 AND t.etat = 'publie' AND t.echeance_le >= now() - interval '14 days'
              ORDER BY t.echeance_le LIMIT 30",
        )
        .bind(etab)
        .bind(cours)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let copies = if saisie { copies_en_attente(&mut tx, etab, cours).await? } else { 0 };
        Ok(SuppleanceAccueilDto {
            cours_id: cours,
            entrees: entrees
                .into_iter()
                .map(|(j, h, sid, titre, contenu)| EntreeRemplacementDto {
                    seance: SeanceReportDto { seance_id: sid, jour: j, heure_debut: h.format("%H:%M").to_string() },
                    titre,
                    contenu,
                })
                .collect(),
            travaux_en_cours: travaux
                .into_iter()
                .map(|(id, titre, echeance_jour, n)| TravailEnCoursDto { id, titre, echeance_jour, nb_remises: n as i32 })
                .collect(),
            copies_en_attente: copies,
            saisie_ouverte: saisie,
        })
    }

    /// CDT-256, CC-69 : décision sur les copies en attente à l'ouverture d'une suppléance : « je corrige », « le titulaire
    /// corrigera », « rendues sans correction ». Aucune ne reste sans décision ; la décision est tracée.
    pub async fn cdt_copies_decider(&self, a: &Acteur<'_>, cours: Uuid, body: CopiesDecisionBody) -> Result<CopiesDecisionDto, EntDbError> {
        let etab = a.etablissement_id;
        if !["je_corrige", "titulaire_corrigera", "rendues_sans_correction"].contains(&body.decision.as_str()) {
            return Err(invalide("décision : je_corrige, titulaire_corrigera ou rendues_sans_correction"));
        }
        let jour = self.jour_etab(etab).await?;
        let suppleant_en_poste = matches!(self.suppleance_de(a, cours, jour).await?, Some((_, true)));
        if !suppleant_en_poste && !self.cdt_direction_ecrit(a).await? {
            return Err(EntDbError::Forbidden);
        }
        // Le suppléant seul peut « corriger lui-même » ; la direction décide du sort des copies sans les corriger.
        if body.decision == "je_corrige" && !suppleant_en_poste {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let en_attente: Vec<(Uuid, Option<Uuid>, Uuid)> = sqlx::query_as(
            "SELECT r.id, r.eleve_id, t.id FROM ent_cdt_remise r JOIN ent_cdt_travail t ON t.etablissement_id = r.etablissement_id AND t.id = r.travail_id
              WHERE r.etablissement_id = $1 AND t.cours_id = $2 AND r.dernier_depot_id IS NOT NULL
                AND NOT EXISTS (SELECT 1 FROM ent_cdt_retour rt WHERE rt.etablissement_id = r.etablissement_id AND rt.remise_id = r.id AND rt.etat = 'rendu')
              ORDER BY r.id FOR UPDATE OF r",
        )
        .bind(etab)
        .bind(cours)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if body.decision == "rendues_sans_correction" {
            for (remise, eleve, travail) in &en_attente {
                sqlx::query(
                    "INSERT INTO ent_cdt_retour (id, etablissement_id, remise_id, etat, commentaire, rendu_le, auteur_user_id)
                     VALUES ($1, $2, $3, 'rendu', '', now(), $4)
                     ON CONFLICT (etablissement_id, remise_id) DO UPDATE SET etat = 'rendu', rendu_le = now(), version = ent_cdt_retour.version + 1,
                        modifie_le = now() WHERE ent_cdt_retour.etat = 'brouillon'",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(remise)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                publier_cdt(&mut tx, a, types::CAHIER_COPIE_RENDUE, *travail, *eleve, json!({ "travail_id": travail, "remise_id": remise })).await?;
            }
        }
        sqlx::query("INSERT INTO ent_cdt_copies_decision (id, etablissement_id, cours_id, decision, nb_copies, decide_par_user_id) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(cours)
            .bind(&body.decision)
            .bind(en_attente.len() as i32)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        publier_cdt(&mut tx, a, types::CAHIER_COPIES_REATTRIBUEES, cours, None, json!({ "cours_id": cours, "decision": body.decision, "nb": en_attente.len() })).await?;
        tx.commit().await.map_err(db)?;
        Ok(CopiesDecisionDto { decision: body.decision, nb_copies: en_attente.len() as i32 })
    }

    /// CDT-258 : les travaux donnés par un **remplaçant** pour une séance du titulaire (à garder, modifier ou retirer par les
    /// fonctions ordinaires du cahier).
    pub async fn cdt_travaux_de_remplacants(&self, a: &Acteur<'_>) -> Result<Vec<TravailDeRemplacantDto>, EntDbError> {
        if !self.cahier_droit_ecrire_global(a).await? {
            return Err(EntDbError::Forbidden);
        }
        let etab = a.etablissement_id;
        let jour = self.jour_etab(etab).await?;
        let lignes: Vec<(Uuid, String, Uuid, NaiveDate, NaiveTime, String)> = sqlx::query_as(
            "SELECT t.id, t.titre, s.id, s.jour, s.heure_debut, ent_cdt_nom_affichage(t.etablissement_id, t.auteur_user_id)
               FROM ent_cdt_travail t JOIN ent_edt_seance s ON s.etablissement_id = t.etablissement_id AND s.id = t.seance_donnee_id
              WHERE t.etablissement_id = $1 AND t.etat IN ('publie', 'programme') AND s.remplacant_user_id IS NOT NULL
                AND t.auteur_user_id = s.remplacant_user_id AND t.auteur_user_id <> $2
                AND s.jour >= $3 - 30 AND ent_cdt_saisie_cours(t.etablissement_id, t.cours_id, $2, $3)
              ORDER BY s.jour DESC, t.id",
        )
        .bind(etab)
        .bind(a.user_id)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(|(travail_id, titre, sid, j, h, donne_par)| TravailDeRemplacantDto {
                travail_id,
                titre,
                seance: SeanceReportDto { seance_id: sid, jour: j, heure_debut: h.format("%H:%M").to_string() },
                donne_par,
            })
            .collect())
    }
}

/// Copies déposées sans retour rendu d'un cours.
async fn copies_en_attente(tx: &mut PgConnection, etab: Uuid, cours: Uuid) -> Result<i32, EntDbError> {
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_cdt_remise r JOIN ent_cdt_travail t ON t.etablissement_id = r.etablissement_id AND t.id = r.travail_id
          WHERE r.etablissement_id = $1 AND t.cours_id = $2 AND r.dernier_depot_id IS NOT NULL
            AND NOT EXISTS (SELECT 1 FROM ent_cdt_retour rt WHERE rt.etablissement_id = r.etablissement_id AND rt.remise_id = r.id AND rt.etat = 'rendu')",
    )
    .bind(etab)
    .bind(cours)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    Ok(n as i32)
}
