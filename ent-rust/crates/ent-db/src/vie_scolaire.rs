//! Vie scolaire complète (socle 2, lot S2-4) : absences à la séance, déclarations
//! (accueil, famille) et rapprochement avec l'appel, décompte en demi-journées,
//! vérification de l'appel, tableau du jour.
//!
//! **Séance provisoire.** Tant que les séances datées du référentiel (S2-2)
//! ne sont pas fusionnées, une « séance » est un créneau EDT + un jour, filtré par
//! semaine A/B et annulations. Ce choix est isolé dans [`PgEnt::seances_du_jour`]
//! et dans la colonne `ent_absence_seance.seance_source` : S2-2 remplacera
//! l'implémentation sans toucher au reste.
//!
//! **Droits.** Tous les contrôles passent par le moteur (`autoriser_eleve`,
//! `a_action_etablissement`, D-017) avec les actions `absences.declarer`,
//! `appel.verifier`, `assiduite.signaler`, `absences.lire`.
//!
//! **Événements.** Chaque fait est écrit avec ses événements dans **une seule
//! transaction** (outbox, S2-0) : `vs.absence_declaree`, `vs.absence_rapprochee`,
//! `vs.appel_relance`.

use crate::assiduite::{
    bornes_mois, deriver_demi_journees, heure_paris, seance_couverte, Contribution, Periode,
    RegleAssiduite,
};
use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use domain_eleve::DomainError;
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use sqlx::PgConnection;
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

/// Types d'événements de la vie scolaire (lot S2-4). Constantes locales : le
/// catalogue central (`ent_events::types`) est partagé entre lots en parallèle.
pub mod types_vs {
    pub const ABSENCE_DECLAREE: &str = "vs.absence_declaree";
    pub const DECLARATION_ANNULEE: &str = "vs.declaration_annulee";
    pub const ABSENCE_RAPPROCHEE: &str = "vs.absence_rapprochee";
    pub const APPEL_RELANCE: &str = "vs.appel_relance";
    pub const PUNITION_DONNEE: &str = "vs.punition_donnee";
    pub const PROCEDURE_ENGAGEE: &str = "vs.procedure_engagee";
    pub const PROCEDURE_DECIDEE: &str = "vs.procedure_decidee";
    pub const CONSEIL_DISCIPLINE_DECIDE: &str = "vs.conseil_discipline_decide";
    pub const SANCTION_EFFACEE: &str = "vs.sanction_effacee";
    pub const SIGNALEMENT_ASSIDUITE_EMIS: &str = "vs.signalement_assiduite_emis";
}

pub(crate) fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("vie_scolaire", err))
}

pub(crate) fn invalide() -> EntDbError {
    EntDbError::Domain(DomainError::ValeurInvalide)
}

pub(crate) fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

/// Longueurs maximales des textes libres.
const MAX_COMMENTAIRE: usize = 1000;
const MAX_NOM: usize = 120;
const MAX_MOTIF: usize = 40;
/// Une déclaration porte sur au plus ce nombre de jours, et reste dans cette
/// fenêtre autour d'aujourd'hui (avant/après, VS-07).
const MAX_DUREE_DECLARATION_JOURS: i64 = 120;
const FENETRE_DECLARATION_JOURS: i64 = 200;

// ---------------------------------------------------------------------------
// Séance (interface provisoire)
// ---------------------------------------------------------------------------

/// Séance datée : l'unité de l'appel et de l'absence (D-020).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Seance {
    /// Identifiant de la séance (créneau EDT tant que S2-2 n'est pas fusionné).
    pub seance_ref: Uuid,
    pub source: String,
    pub jour: NaiveDate,
    pub classe: String,
    pub matiere: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    /// Enseignant effectif du jour (le remplaçant s'il y en a un).
    pub prof_user_id: Uuid,
}

// ---------------------------------------------------------------------------
// DTO
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DeclarationAbsence {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub source: String,
    pub debut_jour: NaiveDate,
    pub debut_heure: Option<NaiveTime>,
    pub fin_jour: NaiveDate,
    pub fin_heure: Option<NaiveTime>,
    pub motif: String,
    pub commentaire: Option<String>,
    pub declarant_nom: Option<String>,
    pub declarant_qualite: Option<String>,
    pub declarant_user_id: Option<Uuid>,
    pub piece_document_id: Option<Uuid>,
    pub statut: String,
    pub created_at: chrono::DateTime<Utc>,
}

const COLONNES_DECLARATION: &str = "id, etablissement_id, eleve_id, source, debut_jour, \
    debut_heure, fin_jour, fin_heure, motif, commentaire, declarant_nom, declarant_qualite, \
    declarant_user_id, piece_document_id, statut, created_at";

#[derive(Debug, Clone)]
pub struct DeclarerAbsence {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub debut_jour: NaiveDate,
    pub debut_heure: Option<NaiveTime>,
    pub fin_jour: NaiveDate,
    pub fin_heure: Option<NaiveTime>,
    pub motif: String,
    pub commentaire: Option<String>,
    /// Accueil : identité de la personne qui prévient (obligatoire).
    pub declarant_nom: Option<String>,
    pub declarant_qualite: Option<String>,
    pub piece_document_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DemiJourneeEleve {
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub periode: Periode,
    pub minutes: i64,
    pub minutes_non_justifiees: i64,
    pub comptee: bool,
    pub non_justifiee: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecompteEleve {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub demi_journees_comptees: i64,
    pub demi_journees_non_justifiees: i64,
    /// Seuil de signalement atteint (règle de l'établissement).
    pub seuil_atteint: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecompteMois {
    pub mois: NaiveDate,
    pub seuil_signalement: i32,
    pub eleves: Vec<DecompteEleve>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeanceSansAppel {
    pub creneau_id: Uuid,
    pub classe: String,
    pub matiere: String,
    pub prof_user_id: Uuid,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    /// `non_fait` : aucun appel ; `non_clos` : appel ouvert alors que la séance est finie.
    pub etat: String,
    pub minutes_depuis_debut: i64,
    pub nb_relances: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IncoherenceAppel {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub jour: NaiveDate,
    /// `absent_puis_present` | `present_malgre_declaration`.
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationAppels {
    pub jour: NaiveDate,
    pub heure_locale: NaiveTime,
    pub delai_minutes: i32,
    pub seances_sans_appel: Vec<SeanceSansAppel>,
    pub incoherences: Vec<IncoherenceAppel>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelanceAppel {
    pub id: Uuid,
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub destinataire_user_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct AbsentDuJour {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub absence_id: Uuid,
    pub statut: String,
    pub prevenue: bool,
    /// Séances manquées (horaires) ; vide pour une absence historique à la journée.
    pub seances: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeclarationDuJour {
    #[serde(flatten)]
    pub declaration: DeclarationAbsence,
    pub nom: String,
    pub prenom: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetardDuJour {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableauJour {
    pub jour: NaiveDate,
    pub absents: Vec<AbsentDuJour>,
    pub retards: Vec<RetardDuJour>,
    pub declarations: Vec<DeclarationDuJour>,
    /// Absences du jour dont le justificatif reste à traiter.
    pub a_traiter: i64,
    pub verification: VerificationAppels,
}

// ---------------------------------------------------------------------------
// Garde-fous communs
// ---------------------------------------------------------------------------

impl PgEnt {
    /// L'acteur a-t-il l'action sur tout l'établissement ?
    pub(crate) async fn exiger_action_etab(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        action: Action,
    ) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etablissement_id, acteur, role, action)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// L'acteur a-t-il l'action sur cet élève (selon la portée de son rôle) ?
    pub(crate) async fn exiger_action_eleve(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        action: Action,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_eleve_existe(etablissement_id, eleve_id).await?;
        if self
            .autoriser_eleve(etablissement_id, acteur, role, action, eleve_id)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    pub(crate) async fn exiger_eleve_existe(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        let ok: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                .bind(etablissement_id)
                .bind(eleve_id)
                .fetch_optional(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
                .await
                .map_err(db)?;
        ok.map(|_| ()).ok_or_else(introuvable)
    }

    // -----------------------------------------------------------------------
    // Règles d'assiduité
    // -----------------------------------------------------------------------

    /// Règles de l'établissement (valeurs par défaut tant qu'il n'a rien paramétré).
    pub async fn regle_assiduite(
        &self,
        etablissement_id: Uuid,
    ) -> Result<RegleAssiduite, EntDbError> {
        let row: Option<(NaiveTime, i32, i32, i32)> = sqlx::query_as(
            "SELECT heure_coupure, seuil_minutes, seuil_signalement, delai_verification_minutes
               FROM ent_vs_regle_assiduite WHERE etablissement_id = $1",
        )
        .bind(etablissement_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await
        .map_err(db)?;
        Ok(row
            .map(|(c, s, g, d)| RegleAssiduite {
                heure_coupure: c,
                seuil_minutes: s,
                seuil_signalement: g,
                delai_verification_minutes: d,
            })
            .unwrap_or_default())
    }

    /// Paramètre les règles d'assiduité ([H] à valider sur les textes officiels).
    pub async fn definir_regle_assiduite(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        regle: RegleAssiduite,
    ) -> Result<RegleAssiduite, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AssiduiteSignaler)
            .await?;
        if !regle.valider() {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query(
            "INSERT INTO ent_vs_regle_assiduite
                (etablissement_id, heure_coupure, seuil_minutes, seuil_signalement,
                 delai_verification_minutes, updated_at)
             VALUES ($1,$2,$3,$4,$5, now())
             ON CONFLICT (etablissement_id) DO UPDATE SET
                heure_coupure = EXCLUDED.heure_coupure,
                seuil_minutes = EXCLUDED.seuil_minutes,
                seuil_signalement = EXCLUDED.seuil_signalement,
                delai_verification_minutes = EXCLUDED.delai_verification_minutes,
                updated_at = now()",
        )
        .bind(etablissement_id)
        .bind(regle.heure_coupure)
        .bind(regle.seuil_minutes)
        .bind(regle.seuil_signalement)
        .bind(regle.delai_verification_minutes)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(regle)
    }

    // -----------------------------------------------------------------------
    // Séances (interface provisoire sur l'EDT)
    // -----------------------------------------------------------------------

    /// Séances de l'établissement un jour donné : créneaux EDT de la semaine
    /// A/B, annulations retirées, remplaçant substitué. **Point de remplacement
    /// par les séances datées de S2-2.**
    pub async fn seances_du_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Seance>, EntDbError> {
        let creneaux = self.list_edt_creneaux_jour(etablissement_id, jour).await?;
        let exceptions: Vec<(Uuid, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT creneau_id, kind, remplacant_user_id FROM ent_edt_exception
              WHERE etablissement_id = $1 AND jour = $2",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await
        .map_err(db)?;
        let annules: HashSet<Uuid> = exceptions
            .iter()
            .filter(|(_, k, _)| k == "annulation")
            .map(|(id, _, _)| *id)
            .collect();
        let remplacants: HashMap<Uuid, Uuid> = exceptions
            .iter()
            .filter(|(_, k, _)| k == "remplacement")
            .filter_map(|(id, _, r)| r.map(|r| (*id, r)))
            .collect();
        let mut out: Vec<Seance> = creneaux
            .into_iter()
            .filter(|c| !annules.contains(&c.id))
            .map(|c| Seance {
                seance_ref: c.id,
                source: "edt_creneau".into(),
                jour,
                classe: c.classe,
                matiere: c.matiere_libelle,
                heure_debut: c.heure_debut,
                heure_fin: c.heure_fin,
                prof_user_id: remplacants.get(&c.id).copied().unwrap_or(c.prof_user_id),
            })
            .collect();
        out.sort_by_key(|s| (s.heure_debut, s.classe.clone()));
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Absence à la séance + rapprochement (dans la transaction de la clôture)
    // -----------------------------------------------------------------------

    /// Enregistre l'absence à la séance d'un appel clos, dans la transaction de la
    /// clôture, et la rapproche des déclarations actives. Renvoie `true` si
    /// l'absence était **prévenue** (une déclaration couvre la séance).
    pub(crate) async fn enregistrer_absence_seance_tx(
        &self,
        tx: &mut PgConnection,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        absence_id: Uuid,
        appel_id: Uuid,
        creneau_id: Option<Uuid>,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let Some(creneau_id) = creneau_id else {
            // Appel historique sans créneau : pas d'horaire, l'absence à la journée
            // reste lue par le repli « demi-journées entières » du décompte.
            return Ok(false);
        };
        let horaires: Option<(NaiveTime, NaiveTime)> = sqlx::query_as(
            "SELECT heure_debut, heure_fin FROM ent_edt_creneau
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(creneau_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((debut, fin)) = horaires else {
            return Ok(false);
        };
        let seance_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_absence_seance
                (id, etablissement_id, eleve_id, absence_id, jour, kind, seance_ref,
                 seance_source, heure_debut, heure_fin, appel_id, source)
             VALUES ($1,$2,$3,$4,$5,'seance',$6,'edt_creneau',$7,$8,$9,'appel')",
        )
        .bind(seance_id)
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(absence_id)
        .bind(jour)
        .bind(creneau_id)
        .bind(debut)
        .bind(fin)
        .bind(appel_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let decls = rapprocher_seance_tx(
            tx,
            etablissement_id,
            eleve_id,
            seance_id,
            jour,
            debut,
            fin,
        )
        .await?;
        Ok(!decls.is_empty())
    }

    // -----------------------------------------------------------------------
    // Déclarations
    // -----------------------------------------------------------------------

    /// VS-06 : absence annoncée à l'accueil (appel ou visite d'un responsable).
    /// Action `absences.declarer` ; l'identité de l'appelant est obligatoire.
    pub async fn declarer_absence_accueil(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: DeclarerAbsence,
    ) -> Result<DeclarationAbsence, EntDbError> {
        self.exiger_action_eleve(
            cmd.etablissement_id,
            acteur,
            role,
            Action::AbsencesDeclarer,
            cmd.eleve_id,
        )
        .await?;
        let nom = cmd.declarant_nom.as_deref().map(str::trim).unwrap_or("");
        if nom.is_empty() {
            return Err(invalide());
        }
        self.enregistrer_declaration(acteur, "accueil", cmd).await
    }

    /// VS-07 : absence déclarée par la famille, avant ou après. Un responsable lié
    /// avec le droit `peut_justifier_absence`, ou l'élève **majeur** lui-même
    /// (un élève mineur ne déclare pas, B3).
    pub async fn declarer_absence_famille(
        &self,
        acteur: Uuid,
        role: &str,
        mut cmd: DeclarerAbsence,
    ) -> Result<DeclarationAbsence, EntDbError> {
        let etab = cmd.etablissement_id;
        match Role::parse(role) {
            Some(Role::Parent) => {
                let lien = self
                    .lien_parent_eleve(etab, acteur, cmd.eleve_id)
                    .await?
                    .ok_or(EntDbError::Forbidden)?;
                if !lien.peut_justifier_absence {
                    return Err(EntDbError::Forbidden);
                }
            }
            Some(Role::Eleve) => {
                let lie = self.eleve_id_for_compte(etab, acteur).await?;
                if lie != Some(cmd.eleve_id)
                    || !self
                        .eleve_est_majeur(etab, cmd.eleve_id, Utc::now().date_naive())
                        .await?
                {
                    return Err(EntDbError::Forbidden);
                }
            }
            _ => return Err(EntDbError::Forbidden),
        }
        // L'identité du déclarant n'est pas saisie par la famille : elle vient du compte.
        cmd.declarant_nom = None;
        cmd.declarant_qualite = None;
        self.enregistrer_declaration(acteur, "famille", cmd).await
    }

    async fn enregistrer_declaration(
        &self,
        acteur: Uuid,
        source: &str,
        cmd: DeclarerAbsence,
    ) -> Result<DeclarationAbsence, EntDbError> {
        let etab = cmd.etablissement_id;
        let aujourdhui = Utc::now().date_naive();
        if cmd.fin_jour < cmd.debut_jour
            || (cmd.fin_jour - cmd.debut_jour).num_days() > MAX_DUREE_DECLARATION_JOURS
            || (cmd.debut_jour - aujourdhui).num_days().abs() > FENETRE_DECLARATION_JOURS
            || (cmd.fin_jour - aujourdhui).num_days().abs() > FENETRE_DECLARATION_JOURS
        {
            return Err(invalide());
        }
        if cmd.fin_jour == cmd.debut_jour {
            if let (Some(d), Some(f)) = (cmd.debut_heure, cmd.fin_heure) {
                if f <= d {
                    return Err(invalide());
                }
            }
        }
        let motif = cmd.motif.trim().to_string();
        if motif.is_empty() || motif.chars().count() > MAX_MOTIF {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), cmd.etablissement_id).await?;
        // Le motif doit exister pour l'établissement (référentiel de motifs).
        let connu: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_absence_motif m WHERE m.code = $1
             UNION ALL
             SELECT 1 FROM ent_absence_motif_etab e
              WHERE e.etablissement_id = $2 AND e.code = $1
             LIMIT 1",
        )
        .bind(&motif)
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if connu.is_none() {
            return Err(EntDbError::MotifInconnu);
        }
        let commentaire = nettoyer(cmd.commentaire.as_deref(), MAX_COMMENTAIRE)?;
        let declarant_nom = nettoyer(cmd.declarant_nom.as_deref(), MAX_NOM)?;
        let declarant_qualite = nettoyer(cmd.declarant_qualite.as_deref(), MAX_NOM)?;
        if let Some(doc) = cmd.piece_document_id {
            // La pièce doit être un document de CET élève, dans CET établissement.
            let ok: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_eleve_document
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3",
            )
            .bind(etab)
            .bind(cmd.eleve_id)
            .bind(doc)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if ok.is_none() {
                return Err(invalide());
            }
        }
        let id = Uuid::new_v4();
        let declarant_user = (source == "famille").then_some(acteur);
        let mut tx = TenantTx::begin(self.pool(), cmd.etablissement_id).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_absence_declaration
                (id, etablissement_id, eleve_id, source, debut_jour, debut_heure, fin_jour,
                 fin_heure, motif, commentaire, declarant_nom, declarant_qualite,
                 declarant_user_id, piece_document_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(id)
        .bind(etab)
        .bind(cmd.eleve_id)
        .bind(source)
        .bind(cmd.debut_jour)
        .bind(cmd.debut_heure)
        .bind(cmd.fin_jour)
        .bind(cmd.fin_heure)
        .bind(&motif)
        .bind(&commentaire)
        .bind(&declarant_nom)
        .bind(&declarant_qualite)
        .bind(declarant_user)
        .bind(cmd.piece_document_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_vs::ABSENCE_DECLAREE.to_string(),
                aggregate: AggregateKind::Absence,
                aggregate_id: id,
                eleve_id: Some(cmd.eleve_id),
                payload: json!({
                    "source": source,
                    "debut_jour": cmd.debut_jour.to_string(),
                    "fin_jour": cmd.fin_jour.to_string(),
                    "motif": motif,
                }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        // Déclaration après coup : les absences déjà constatées sur la période
        // deviennent « prévenues » (rapprochement dans l'autre sens).
        let rapprochees =
            rapprocher_declaration_tx(&mut tx, etab, id, cmd.eleve_id).await?;
        if rapprochees > 0 {
            tx.publier_avec(
                MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types_vs::ABSENCE_RAPPROCHEE.to_string(),
                    aggregate: AggregateKind::Absence,
                    aggregate_id: id,
                    eleve_id: Some(cmd.eleve_id),
                    payload: json!({ "declaration_id": id, "absences_seances": rapprochees }),
                    actor_user_id: Some(acteur.to_string()),
                },
                crate::outbox::Causalite::suite_de(&evt, crate::outbox::Causalite::default()),
            )
            .await?;
        }
        let decl = lire_declaration_tx(&mut tx, etab, id)
            .await?
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(decl)
    }

    /// Annule une déclaration (erreur de saisie, enfant finalement présent). Les
    /// absences qu'elle avait rendues « prévenues » le redeviennent selon les
    /// autres déclarations. Accueil/vie scolaire (`absences.declarer`) ou
    /// déclarant d'origine.
    pub async fn annuler_declaration(
        &self,
        etablissement_id: Uuid,
        declaration_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        let decl = sqlx::query_as::<_, DeclarationAbsence>(&format!(
            "SELECT {COLONNES_DECLARATION} FROM ent_absence_declaration
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(etablissement_id)
        .bind(declaration_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let propre = decl.declarant_user_id == Some(acteur);
        if !propre {
            self.exiger_action_eleve(
                etablissement_id,
                acteur,
                role,
                Action::AbsencesDeclarer,
                decl.eleve_id,
            )
            .await?;
        }
        if decl.statut != "active" {
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        let touchees: Vec<(Uuid,)> = sqlx::query_as(
            "DELETE FROM ent_absence_rapprochement
              WHERE etablissement_id = $1 AND declaration_id = $2
              RETURNING absence_seance_id",
        )
        .bind(etablissement_id)
        .bind(declaration_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = touchees.into_iter().map(|(i,)| i).collect();
        sqlx::query(
            "UPDATE ent_absence_seance s
                SET prevenue = EXISTS (SELECT 1 FROM ent_absence_rapprochement r
                                        WHERE r.absence_seance_id = s.id)
              WHERE s.etablissement_id = $1 AND s.id = ANY($2)",
        )
        .bind(etablissement_id)
        .bind(&ids)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_absence_declaration
                SET statut = 'annulee', annulee_par_user_id = $3, annulee_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(declaration_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id,
            type_name: types_vs::DECLARATION_ANNULEE.to_string(),
            aggregate: AggregateKind::Absence,
            aggregate_id: declaration_id,
            eleve_id: Some(decl.eleve_id),
            payload: json!({ "declaration_id": declaration_id }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Déclarations d'un élève (lecture : `absences.lire` du personnel, ou
    /// responsable lié avec `peut_voir_vs`, ou l'élève).
    pub async fn declarations_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<DeclarationAbsence>, EntDbError> {
        self.exiger_eleve_existe(etablissement_id, eleve_id).await?;
        if !self
            .actor_can_read_absences(etablissement_id, acteur, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        sqlx::query_as::<_, DeclarationAbsence>(&format!(
            "SELECT {COLONNES_DECLARATION} FROM ent_absence_declaration
              WHERE etablissement_id = $1 AND eleve_id = $2
              ORDER BY debut_jour DESC, created_at DESC LIMIT 200"
        ))
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await
        .map_err(db)
    }

    /// A1 : élèves « attendus absents » à l'appel — une déclaration active couvre
    /// entièrement la séance. Même contrôle d'accès que l'appel lui-même.
    pub async fn attendus_absents_appel(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<Uuid>, EntDbError> {
        // `get_appel` applique le contrôle d'accès (titulaire, remplaçant, supervision).
        let (appel, lignes) = self
            .get_appel(etablissement_id, appel_id, acteur, role)
            .await?
            .ok_or(EntDbError::AppelInvalide)?;
        let Some(creneau_id) = appel.creneau_id else {
            return Ok(Vec::new());
        };
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let horaires: Option<(NaiveTime, NaiveTime)> = sqlx::query_as(
            "SELECT heure_debut, heure_fin FROM ent_edt_creneau
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(creneau_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((debut, fin)) = horaires else {
            return Ok(Vec::new());
        };
        let eleves: Vec<Uuid> = lignes.iter().map(|l| l.eleve_id).collect();
        let decls: Vec<(Uuid, NaiveDate, Option<NaiveTime>, NaiveDate, Option<NaiveTime>)> =
            sqlx::query_as(
                "SELECT eleve_id, debut_jour, debut_heure, fin_jour, fin_heure
                   FROM ent_absence_declaration
                  WHERE etablissement_id = $1 AND statut = 'active' AND eleve_id = ANY($2)
                    AND debut_jour <= $3 AND fin_jour >= $3",
            )
            .bind(etablissement_id)
            .bind(&eleves)
            .bind(appel.jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut out: Vec<Uuid> = decls
            .into_iter()
            .filter(|(_, dj, dh, fj, fh)| {
                seance_couverte(appel.jour, debut, fin, *dj, *dh, *fj, *fh)
            })
            .map(|(e, ..)| e)
            .collect();
        out.sort();
        out.dedup();
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // Décompte en demi-journées
    // -----------------------------------------------------------------------

    /// Contributions d'absence par (élève, jour) sur `[du, au]` inclus : lignes à
    /// la séance + repli sur les absences à la journée qui n'ont aucune ligne.
    async fn contributions(
        &self,
        etablissement_id: Uuid,
        eleve_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<BTreeMap<(Uuid, NaiveDate), Vec<Contribution>>, EntDbError> {
        let mut out: BTreeMap<(Uuid, NaiveDate), Vec<Contribution>> = BTreeMap::new();
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let lignes: Vec<(
            Uuid,
            NaiveDate,
            String,
            Option<String>,
            Option<NaiveTime>,
            Option<NaiveTime>,
            bool,
        )> = sqlx::query_as(
            "SELECT s.eleve_id, s.jour, s.kind, s.periode, s.heure_debut, s.heure_fin,
                    (a.statut = 'justifiee')
               FROM ent_absence_seance s
               JOIN ent_absence a
                 ON a.id = s.absence_id AND a.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3
                AND ($4::uuid IS NULL OR s.eleve_id = $4)
                AND a.statut <> 'annulee_present'",
        )
        .bind(etablissement_id)
        .bind(du)
        .bind(au)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (e, jour, kind, periode, debut, fin, justifiee) in lignes {
            let c = match (kind.as_str(), periode.as_deref().and_then(Periode::parse), debut, fin) {
                ("demi_journee", Some(p), _, _) => Contribution::DemiJournee {
                    periode: p,
                    justifiee,
                },
                ("seance", _, Some(d), Some(f)) => Contribution::Seance {
                    debut: d,
                    fin: f,
                    justifiee,
                },
                _ => continue,
            };
            out.entry((e, jour)).or_default().push(c);
        }
        // Repli : absences à la journée sans aucune ligne (créées par l'ancien
        // chemin ou appels sans créneau) = journées ouvrées entières.
        let anciennes: Vec<(Uuid, NaiveDate, bool)> = sqlx::query_as(
            "SELECT a.eleve_id, d::date, (a.statut = 'justifiee')
               FROM ent_absence a
               CROSS JOIN LATERAL generate_series(
                    GREATEST(a.debut, $2)::timestamp, LEAST(a.fin, $3)::timestamp, interval '1 day') d
              WHERE a.etablissement_id = $1
                AND a.fin >= $2 AND a.debut <= $3
                AND ($4::uuid IS NULL OR a.eleve_id = $4)
                AND a.statut <> 'annulee_present'
                AND EXTRACT(ISODOW FROM d) < 6
                AND NOT EXISTS (SELECT 1 FROM ent_absence_seance s WHERE s.absence_id = a.id)",
        )
        .bind(etablissement_id)
        .bind(du)
        .bind(au)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (e, jour, justifiee) in anciennes {
            let v = out.entry((e, jour)).or_default();
            for periode in [Periode::Matin, Periode::ApresMidi] {
                v.push(Contribution::DemiJournee { periode, justifiee });
            }
        }
        Ok(out)
    }

    /// Demi-journées dérivées sur une période (sans contrôle d'accès : usage interne).
    pub(crate) async fn demi_journees(
        &self,
        etablissement_id: Uuid,
        eleve_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<DemiJourneeEleve>, EntDbError> {
        let regle = self.regle_assiduite(etablissement_id).await?;
        let contribs = self
            .contributions(etablissement_id, eleve_id, du, au)
            .await?;
        let mut out = Vec::new();
        for ((eleve, jour), cs) in contribs {
            for dj in deriver_demi_journees(&cs, &regle) {
                out.push(DemiJourneeEleve {
                    eleve_id: eleve,
                    jour,
                    periode: dj.periode,
                    minutes: dj.minutes,
                    minutes_non_justifiees: dj.minutes_non_justifiees,
                    comptee: dj.comptee,
                    non_justifiee: dj.non_justifiee,
                });
            }
        }
        Ok(out)
    }

    /// Demi-journées d'absence d'un élève sur `[du, au]` (inclus). Mêmes droits que
    /// la lecture des absences.
    pub async fn demi_journees_eleve_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<DemiJourneeEleve>, EntDbError> {
        self.exiger_eleve_existe(etablissement_id, eleve_id).await?;
        if au < du || (au - du).num_days() > 400 {
            return Err(invalide());
        }
        if !self
            .actor_can_read_absences(etablissement_id, acteur, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.demi_journees(etablissement_id, Some(eleve_id), du, au)
            .await
    }

    /// Décompte réglementaire du mois (A5) : demi-journées non justifiées par
    /// élève, seuil de signalement de l'établissement. Action `assiduite.signaler`.
    pub async fn decompte_mensuel_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        mois: NaiveDate,
    ) -> Result<DecompteMois, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AssiduiteSignaler)
            .await?;
        self.decompte_mensuel(etablissement_id, mois).await
    }

    pub(crate) async fn decompte_mensuel(
        &self,
        etablissement_id: Uuid,
        mois: NaiveDate,
    ) -> Result<DecompteMois, EntDbError> {
        let regle = self.regle_assiduite(etablissement_id).await?;
        let (debut, suivant) = bornes_mois(mois);
        let dernier = suivant - Duration::days(1);
        let dj = self
            .demi_journees(etablissement_id, None, debut, dernier)
            .await?;
        let mut par_eleve: BTreeMap<Uuid, (i64, i64)> = BTreeMap::new();
        for d in dj.iter().filter(|d| d.comptee) {
            let e = par_eleve.entry(d.eleve_id).or_default();
            e.0 += 1;
            if d.non_justifiee {
                e.1 += 1;
            }
        }
        let ids: Vec<Uuid> = par_eleve.keys().copied().collect();
        let noms: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etablissement_id)
        .bind(&ids)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await
        .map_err(db)?;
        let noms: HashMap<Uuid, (String, String)> =
            noms.into_iter().map(|(i, n, p)| (i, (n, p))).collect();
        let mut eleves: Vec<DecompteEleve> = par_eleve
            .into_iter()
            .filter_map(|(id, (comptees, non_just))| {
                let (nom, prenom) = noms.get(&id)?.clone();
                Some(DecompteEleve {
                    eleve_id: id,
                    nom,
                    prenom,
                    demi_journees_comptees: comptees,
                    demi_journees_non_justifiees: non_just,
                    seuil_atteint: non_just >= i64::from(regle.seuil_signalement),
                })
            })
            .collect();
        eleves.sort_by(|a, b| {
            b.demi_journees_non_justifiees
                .cmp(&a.demi_journees_non_justifiees)
                .then(a.nom.cmp(&b.nom))
        });
        Ok(DecompteMois {
            mois: debut,
            seuil_signalement: regle.seuil_signalement,
            eleves,
        })
    }

    // -----------------------------------------------------------------------
    // Vérification de l'appel (VS-04, A2)
    // -----------------------------------------------------------------------

    /// Appels non faits (ou non clos après la fin de séance) et incohérences d'un
    /// jour. `maintenant` est l'heure **locale** de l'établissement ([`heure_paris`]).
    /// Action `appel.verifier`.
    pub async fn verifier_appels_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        maintenant: NaiveDateTime,
    ) -> Result<VerificationAppels, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AppelVerifier)
            .await?;
        self.verifier_appels(etablissement_id, jour, maintenant).await
    }

    pub(crate) async fn verifier_appels(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
        maintenant: NaiveDateTime,
    ) -> Result<VerificationAppels, EntDbError> {
        let regle = self.regle_assiduite(etablissement_id).await?;
        let seances = self.seances_du_jour(etablissement_id, jour).await?;
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let appels: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT id, creneau_id, statut FROM ent_appel
              WHERE etablissement_id = $1 AND jour = $2 AND creneau_id IS NOT NULL",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let appel_par_creneau: HashMap<Uuid, (Uuid, String)> = appels
            .iter()
            .map(|(id, c, s)| (*c, (*id, s.clone())))
            .collect();
        let relances: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT creneau_id, COUNT(*)::bigint FROM ent_appel_relance
              WHERE etablissement_id = $1 AND jour = $2 GROUP BY creneau_id",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let relances: HashMap<Uuid, i64> = relances.into_iter().collect();

        let mut sans = Vec::new();
        for s in &seances {
            let debut_dt = jour.and_time(s.heure_debut);
            let fin_dt = jour.and_time(s.heure_fin);
            let echeance = debut_dt + Duration::minutes(i64::from(regle.delai_verification_minutes));
            let etat = match appel_par_creneau.get(&s.seance_ref) {
                None if maintenant >= echeance => "non_fait",
                Some((_, statut)) if statut == "ouvert" && maintenant >= fin_dt => "non_clos",
                _ => continue,
            };
            sans.push(SeanceSansAppel {
                creneau_id: s.seance_ref,
                classe: s.classe.clone(),
                matiere: s.matiere.clone(),
                prof_user_id: s.prof_user_id,
                heure_debut: s.heure_debut,
                heure_fin: s.heure_fin,
                etat: etat.into(),
                minutes_depuis_debut: (maintenant - debut_dt).num_minutes(),
                nb_relances: relances.get(&s.seance_ref).copied().unwrap_or(0),
            });
        }
        let incoherences = self.incoherences_appels(etablissement_id, jour).await?;
        Ok(VerificationAppels {
            jour,
            heure_locale: maintenant.time(),
            delai_minutes: regle.delai_verification_minutes,
            seances_sans_appel: sans,
            incoherences,
        })
    }

    /// Incohérences entre appels d'une même journée : élève absent puis présent
    /// à une séance ultérieure ; élève présent alors qu'une déclaration active
    /// couvre la séance.
    async fn incoherences_appels(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<IncoherenceAppel>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let lignes: Vec<(Uuid, String, String, NaiveTime, NaiveTime, String)> = sqlx::query_as(
            "SELECT l.eleve_id, e.nom, e.prenom, c.heure_debut, c.heure_fin, l.etat
               FROM ent_appel a
               JOIN ent_appel_ligne l ON l.appel_id = a.id
               JOIN ent_edt_creneau c ON c.id = a.creneau_id AND c.etablissement_id = a.etablissement_id
               JOIN ent_eleve e ON e.id = l.eleve_id AND e.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.jour = $2 AND a.statut = 'clos'",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let decls: Vec<(Uuid, NaiveDate, Option<NaiveTime>, NaiveDate, Option<NaiveTime>)> =
            sqlx::query_as(
                "SELECT eleve_id, debut_jour, debut_heure, fin_jour, fin_heure
                   FROM ent_absence_declaration
                  WHERE etablissement_id = $1 AND statut = 'active'
                    AND debut_jour <= $2 AND fin_jour >= $2",
            )
            .bind(etablissement_id)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut par_eleve: BTreeMap<Uuid, Vec<&(Uuid, String, String, NaiveTime, NaiveTime, String)>> =
            BTreeMap::new();
        for l in &lignes {
            par_eleve.entry(l.0).or_default().push(l);
        }
        let mut out = Vec::new();
        for (eleve, mut ls) in par_eleve {
            ls.sort_by_key(|l| l.3);
            let (nom, prenom) = (ls[0].1.clone(), ls[0].2.clone());
            // Absent puis présent (ou en retard) à une séance qui commence ensuite.
            if let Some(absent) = ls.iter().find(|l| l.5 == "absent") {
                if let Some(present) = ls
                    .iter()
                    .find(|l| l.3 >= absent.4 && (l.5 == "present"))
                {
                    out.push(IncoherenceAppel {
                        eleve_id: eleve,
                        nom: nom.clone(),
                        prenom: prenom.clone(),
                        jour,
                        kind: "absent_puis_present".into(),
                        detail: format!(
                            "absent à la séance de {}, présent à celle de {}",
                            absent.3.format("%H:%M"),
                            present.3.format("%H:%M")
                        ),
                    });
                }
            }
            for l in ls.iter().filter(|l| l.5 == "present") {
                let couverte = decls.iter().any(|(e, dj, dh, fj, fh)| {
                    *e == eleve && seance_couverte(jour, l.3, l.4, *dj, *dh, *fj, *fh)
                });
                if couverte {
                    out.push(IncoherenceAppel {
                        eleve_id: eleve,
                        nom: nom.clone(),
                        prenom: prenom.clone(),
                        jour,
                        kind: "present_malgre_declaration".into(),
                        detail: format!(
                            "marqué présent à la séance de {} alors qu'une absence était annoncée",
                            l.3.format("%H:%M")
                        ),
                    });
                }
            }
        }
        Ok(out)
    }

    /// Relance l'enseignant d'une séance dont l'appel n'est pas fait / pas clos :
    /// trace + événement (notification de l'enseignant par le distributeur).
    /// Action `appel.verifier`.
    pub async fn relancer_appel(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        creneau_id: Uuid,
        jour: NaiveDate,
    ) -> Result<RelanceAppel, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AppelVerifier)
            .await?;
        let seance = self
            .seances_du_jour(etablissement_id, jour)
            .await?
            .into_iter()
            .find(|s| s.seance_ref == creneau_id)
            .ok_or_else(introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let clos: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_appel
              WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3 AND statut = 'clos'",
        )
        .bind(etablissement_id)
        .bind(creneau_id)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if clos.is_some() {
            // Rien à relancer : l'appel est fait.
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_appel_relance
                (id, etablissement_id, creneau_id, jour, destinataire_user_id, envoyee_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(creneau_id)
        .bind(jour)
        .bind(seance.prof_user_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id,
            type_name: types_vs::APPEL_RELANCE.to_string(),
            aggregate: AggregateKind::Appel,
            aggregate_id: id,
            eleve_id: None,
            payload: json!({
                "relance_id": id,
                "creneau_id": creneau_id,
                "jour": jour.to_string(),
                "destinataire_user_id": seance.prof_user_id,
            }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(RelanceAppel {
            id,
            creneau_id,
            jour,
            destinataire_user_id: seance.prof_user_id,
        })
    }

    // -----------------------------------------------------------------------
    // Tableau du jour (VS-25)
    // -----------------------------------------------------------------------

    /// Tableau du jour de la vie scolaire / de l'accueil : absents (avec
    /// « prévenue »), retards, déclarations du jour, à traiter, vérification des
    /// appels. Lecture sur tout l'établissement : `absences.lire` portée
    /// établissement **et** `appel.verifier`.
    pub async fn tableau_du_jour_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        maintenant: NaiveDateTime,
    ) -> Result<TableauJour, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AbsencesLire)
            .await?;
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AppelVerifier)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        let lignes: Vec<(Uuid, String, String, Uuid, String, bool, Option<NaiveTime>, Option<NaiveTime>)> =
            sqlx::query_as(
                "SELECT a.eleve_id, e.nom, e.prenom, a.id, a.statut, COALESCE(bool_or(s.prevenue), false),
                        s.heure_debut, s.heure_fin
                   FROM ent_absence a
                   JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
                   LEFT JOIN ent_absence_seance s ON s.absence_id = a.id AND s.jour = $2
                  WHERE a.etablissement_id = $1 AND a.debut <= $2 AND a.fin >= $2
                    AND a.statut <> 'annulee_present'
                  GROUP BY a.eleve_id, e.nom, e.prenom, a.id, a.statut, s.heure_debut, s.heure_fin
                  ORDER BY e.nom, e.prenom, s.heure_debut",
            )
            .bind(etablissement_id)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut absents: Vec<AbsentDuJour> = Vec::new();
        for (eleve_id, nom, prenom, absence_id, statut, prevenue, d, f) in lignes {
            let horaire = d
                .zip(f)
                .map(|(d, f)| format!("{}–{}", d.format("%H:%M"), f.format("%H:%M")));
            match absents.iter_mut().find(|x| x.eleve_id == eleve_id) {
                Some(x) => {
                    // Un élève = une ligne : séances agrégées ; « prévenue » seulement si
                    // toutes ses absences le sont ; à traiter dès qu'une absence l'est.
                    x.prevenue &= prevenue;
                    if statut == "a_traiter" {
                        x.statut = statut;
                    }
                    if let Some(h) = horaire {
                        if !x.seances.contains(&h) {
                            x.seances.push(h);
                        }
                    }
                }
                None => absents.push(AbsentDuJour {
                    eleve_id,
                    nom,
                    prenom,
                    absence_id,
                    statut,
                    prevenue,
                    seances: horaire.into_iter().collect(),
                }),
            }
        }
        // Justificatifs à traiter : par absence (et non par élève).
        let a_traiter: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM ent_absence
              WHERE etablissement_id = $1 AND debut <= $2 AND fin >= $2 AND statut = 'a_traiter'",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let retards: Vec<(Uuid, String, String, Option<i32>)> = sqlx::query_as(
            "SELECT r.eleve_id, e.nom, e.prenom, r.minutes
               FROM ent_retard r
               JOIN ent_eleve e ON e.id = r.eleve_id AND e.etablissement_id = r.etablissement_id
              WHERE r.etablissement_id = $1 AND r.jour = $2
              ORDER BY e.nom, e.prenom",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let decls = sqlx::query_as::<_, DeclarationAbsence>(&format!(
            "SELECT {COLONNES_DECLARATION} FROM ent_absence_declaration
              WHERE etablissement_id = $1 AND statut = 'active'
                AND debut_jour <= $2 AND fin_jour >= $2
              ORDER BY created_at DESC LIMIT 500"
        ))
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = decls.iter().map(|d| d.eleve_id).collect();
        let noms: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etablissement_id)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let noms: HashMap<Uuid, (String, String)> =
            noms.into_iter().map(|(i, n, p)| (i, (n, p))).collect();
        let declarations = decls
            .into_iter()
            .map(|d| {
                let (nom, prenom) = noms.get(&d.eleve_id).cloned().unwrap_or_default();
                DeclarationDuJour {
                    declaration: d,
                    nom,
                    prenom,
                }
            })
            .collect();
        let verification = self.verifier_appels(etablissement_id, jour, maintenant).await?;
        Ok(TableauJour {
            jour,
            absents,
            retards: retards
                .into_iter()
                .map(|(eleve_id, nom, prenom, minutes)| RetardDuJour {
                    eleve_id,
                    nom,
                    prenom,
                    minutes,
                })
                .collect(),
            declarations,
            a_traiter,
            verification,
        })
    }

    /// Heure locale de l'établissement (Paris) à l'instant présent.
    pub fn maintenant_local() -> NaiveDateTime {
        heure_paris(Utc::now())
    }
}

// ---------------------------------------------------------------------------
// Rapprochement (fonctions de transaction)
// ---------------------------------------------------------------------------

fn nettoyer(valeur: Option<&str>, max: usize) -> Result<Option<String>, EntDbError> {
    let Some(v) = valeur.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    if v.chars().count() > max || v.chars().any(|c| c.is_control() && c != '\n') {
        return Err(invalide());
    }
    Ok(Some(v.to_string()))
}

async fn lire_declaration_tx(
    tx: &mut PgConnection,
    etablissement_id: Uuid,
    id: Uuid,
) -> Result<Option<DeclarationAbsence>, EntDbError> {
    sqlx::query_as::<_, DeclarationAbsence>(&format!(
        "SELECT {COLONNES_DECLARATION} FROM ent_absence_declaration
          WHERE etablissement_id = $1 AND id = $2"
    ))
    .bind(etablissement_id)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)
}

/// Rapproche une absence à la séance des déclarations actives de l'élève qui
/// couvrent entièrement la séance. Renvoie les déclarations rapprochées.
async fn rapprocher_seance_tx(
    tx: &mut PgConnection,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    seance_id: Uuid,
    jour: NaiveDate,
    debut: NaiveTime,
    fin: NaiveTime,
) -> Result<Vec<Uuid>, EntDbError> {
    let decls: Vec<(Uuid, NaiveDate, Option<NaiveTime>, NaiveDate, Option<NaiveTime>)> =
        sqlx::query_as(
            "SELECT id, debut_jour, debut_heure, fin_jour, fin_heure
               FROM ent_absence_declaration
              WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'active'
                AND debut_jour <= $3 AND fin_jour >= $3",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
    let mut couvrantes = Vec::new();
    for (id, dj, dh, fj, fh) in decls {
        if seance_couverte(jour, debut, fin, dj, dh, fj, fh) {
            sqlx::query(
                "INSERT INTO ent_absence_rapprochement (etablissement_id, declaration_id, absence_seance_id)
                 VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
            )
            .bind(etablissement_id)
            .bind(id)
            .bind(seance_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            couvrantes.push(id);
        }
    }
    if !couvrantes.is_empty() {
        sqlx::query("UPDATE ent_absence_seance SET prevenue = true WHERE etablissement_id = $1 AND id = $2")
            .bind(etablissement_id)
            .bind(seance_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
    }
    Ok(couvrantes)
}

/// Rapprochement dans l'autre sens : une déclaration (souvent faite après coup)
/// couvre des absences à la séance déjà constatées. Renvoie leur nombre.
async fn rapprocher_declaration_tx(
    tx: &mut PgConnection,
    etablissement_id: Uuid,
    declaration_id: Uuid,
    eleve_id: Uuid,
) -> Result<usize, EntDbError> {
    let (dj, dh, fj, fh): (NaiveDate, Option<NaiveTime>, NaiveDate, Option<NaiveTime>) =
        sqlx::query_as(
            "SELECT debut_jour, debut_heure, fin_jour, fin_heure FROM ent_absence_declaration
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(declaration_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
    let seances: Vec<(Uuid, NaiveDate, NaiveTime, NaiveTime)> = sqlx::query_as(
        "SELECT s.id, s.jour, s.heure_debut, s.heure_fin
           FROM ent_absence_seance s
           JOIN ent_absence a ON a.id = s.absence_id AND a.etablissement_id = s.etablissement_id
          WHERE s.etablissement_id = $1 AND s.eleve_id = $2 AND s.kind = 'seance'
            AND s.jour BETWEEN $3 AND $4 AND a.statut <> 'annulee_present'",
    )
    .bind(etablissement_id)
    .bind(eleve_id)
    .bind(dj)
    .bind(fj)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let mut n = 0;
    for (id, jour, debut, fin) in seances {
        if seance_couverte(jour, debut, fin, dj, dh, fj, fh) {
            sqlx::query(
                "INSERT INTO ent_absence_rapprochement (etablissement_id, declaration_id, absence_seance_id)
                 VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
            )
            .bind(etablissement_id)
            .bind(declaration_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query("UPDATE ent_absence_seance SET prevenue = true WHERE etablissement_id = $1 AND id = $2")
                .bind(etablissement_id)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            n += 1;
        }
    }
    Ok(n)
}
