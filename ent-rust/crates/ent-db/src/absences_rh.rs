//! Socle 15, lot A0 — correctifs de la route historique des absences de personnel
//! (`15_ABSENCES_PERSONNELS.md` §28.4 ; AR-01, AR-06, AR-07, AR-13, AR-16, AR-17).
//!
//! | Commande                         | Droit (D-017)                                                       |
//! |----------------------------------|---------------------------------------------------------------------|
//! | `declarer_absence_rh_for_actor`  | soi-même (tout le personnel) ou `personnels.absences.declarer`      |
//! | `valider_` / `refuser_`          | `personnels.absences.valider`, **jamais l'absent lui-même**          |
//! | `annuler_absence_rh_for_actor`   | `valider`, ou l'agent lui-même avant le début de l'absence           |
//! | `list_absences_rh_for_actor`     | `lire_organisation` ou `lire_nature` ; sinon **ses propres** lignes |
//!
//! Le **motif** (texte libre, parfois médical) n'est rendu qu'à qui détient
//! `personnels.absences.lire_nature` ou à l'agent lui-même ; tous les autres voient « Absence »
//! (R-AP2). Chaque fait est journalisé (`ent_journal_modif`, domaine `personnels_absences`) dans la
//! même transaction que l'événement de l'outbox ; ni le journal ni l'événement ne portent le motif.
//! Aucune valeur de motif n'est jamais écrite dans un événement (S2-5, RH-05).

use chrono::NaiveDate;
use domain_eleve::{
    plan_annuler_absence_rh, plan_declarer_absence_rh, plan_refuser_absence_rh,
    plan_valider_absence_rh, AbsenceRh, DeclarerAbsenceRh, DomainError,
};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::absence_rh_repo::{Row, COLONNES};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

/// Libellé montré à qui n'a pas le droit de lire la nature (R-AP2, AR-01).
pub const MOTIF_MASQUE: &str = "Absence";

const DOMAINE_JOURNAL: &str = "personnels_absences";

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Domain(crate::pg_err::map_sqlx("absence_rh", e))
}

fn auto_validation() -> EntDbError {
    EntDbError::Refus(
        RefusMetier::Interdit,
        "Personne ne valide ni ne refuse sa propre absence.",
    )
}

fn depuis_domaine(e: DomainError) -> EntDbError {
    match e {
        DomainError::AbsenceRhAutoValidation => auto_validation(),
        DomainError::AbsenceRhInvalide => EntDbError::Refus(
            RefusMetier::Conflit,
            "Cette absence n'est plus dans un état qui permet cette action.",
        ),
        autre => autre.into(),
    }
}

/// Écrit un fait dans le journal d'audit (AR-17). Identifiants et codes seulement : jamais le motif.
pub(crate) async fn journal_absence(
    conn: &mut PgConnection,
    etab: Uuid,
    action: &str,
    libelle: &str,
    absence_id: Uuid,
    agent_user_id: Uuid,
    actor: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,NULL,$3,$4,$5,$6,false,$7, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(DOMAINE_JOURNAL)
    .bind(action)
    .bind(libelle)
    .bind(json!({ "absence_id": absence_id, "user_id": agent_user_id }))
    .bind(actor)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn charger_verrou(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<AbsenceRh, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES} FROM ent_absence_rh WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
    );
    sqlx::query_as::<_, Row>(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .map(Row::into_absence)
        .ok_or(EntDbError::Domain(DomainError::NotFound))
}

impl PgEnt {
    async fn droit_absences(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        action: Action,
    ) -> Result<bool, EntDbError> {
        self.a_action_etablissement(etab, actor, role, action).await
    }

    /// Déclare une absence. Soi-même : aucun droit. Pour un collègue : `personnels.absences.declarer`.
    /// Une absence déclarée par un tiers reste « signalée » (non confirmée) tant que l'agent ne l'a
    /// pas confirmée.
    pub async fn declarer_absence_rh_for_actor(
        &self,
        role: &str,
        cmd: DeclarerAbsenceRh,
    ) -> Result<AbsenceRh, EntDbError> {
        // Le domaine de l'absence est `absences_personnels.rs` (lot A1) : la route historique n'est
        // plus qu'un adaptateur. Un seul chemin d'écriture, donc une seule source de vérité ; la
        // table `ent_absence_rh` en est la projection (déclencheur de la migration 2302).
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let (etab, actor) = (cmd.etablissement_id, cmd.actor_user_id);
        let agent = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            crate::personnels::agent_du_user_tx(&mut tx, etab, cmd.user_id).await?
        };
        let Some(agent_id) = agent else {
            return Err(EntDbError::Refus(
                RefusMetier::Invalide,
                "Aucun agent n'est rattaché à ce compte : l'absence ne peut pas être déclarée.",
            ));
        };
        // Validation de forme historique (dates, motif non vide ≤ 200).
        let planned = plan_declarer_absence_rh(cmd.clone())?;
        if cmd.user_id != actor
            && !self
                .droit_absences(etab, actor, role, Action::PersonnelsAbsencesDeclarer)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if !self.auth_repo().is_member(etab, cmd.user_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let body = ent_types::absences_personnels::DeclarerAbsenceBody {
            agent_id: Some(agent_id),
            nature_code: None, // « je préciserai plus tard » : la nature se qualifie ensuite
            debut: cmd.debut,
            fin: cmd.fin,
            ..Default::default()
        };
        // L'ancien motif libre n'est pas une nature : il va en zone de qualification (Q-AP15).
        let dto = self
            .declarer_absence_interne(etab, actor, role, &body, Some(planned.motif))
            .await?;
        let id = dto
            .absences
            .first()
            .map(|a| a.id)
            .ok_or(DomainError::NotFound)?;
        Ok(self
            .absences_rh
            .get(etab, id)
            .await?
            .ok_or(DomainError::NotFound)?)
    }

    /// Valide une absence (AR-06 : jamais la sienne ; AR-07 : action dédiée ; AR-13 : le déclarant
    /// est conservé). Idempotent : une absence déjà validée est rendue telle quelle.
    pub async fn valider_absence_rh_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        absence_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<AbsenceRh, EntDbError> {
        if self
            .absence_du_domaine(etablissement_id, absence_id)
            .await?
        {
            self.valider_absence_for_actor(
                etablissement_id,
                actor_user_id,
                role,
                absence_id,
                &Default::default(),
            )
            .await?;
            return self.relire_absence_rh(etablissement_id, absence_id).await;
        }
        self.decider_absence_rh(
            etablissement_id,
            role,
            absence_id,
            actor_user_id,
            Decision::Valider,
        )
        .await
    }

    /// Refuse une absence (AR-16). Même règle que la validation : jamais l'absent lui-même.
    pub async fn refuser_absence_rh_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        absence_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<AbsenceRh, EntDbError> {
        if self
            .absence_du_domaine(etablissement_id, absence_id)
            .await?
        {
            self.refuser_absence_for_actor(
                etablissement_id,
                actor_user_id,
                role,
                absence_id,
                &Default::default(),
            )
            .await?;
            return self.relire_absence_rh(etablissement_id, absence_id).await;
        }
        self.decider_absence_rh(
            etablissement_id,
            role,
            absence_id,
            actor_user_id,
            Decision::Refuser,
        )
        .await
    }

    /// Annule une absence saisie par erreur ou qui n'a pas eu lieu (AR-16) : rien n'est supprimé.
    /// Le détenteur de `valider`, ou l'agent lui-même tant que l'absence n'a pas commencé.
    pub async fn annuler_absence_rh_for_actor(
        &self,
        etablissement_id: Uuid,
        role: &str,
        absence_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<AbsenceRh, EntDbError> {
        if self
            .absence_du_domaine(etablissement_id, absence_id)
            .await?
        {
            self.annuler_absence_for_actor(
                etablissement_id,
                actor_user_id,
                role,
                absence_id,
                &Default::default(),
            )
            .await?;
            return self.relire_absence_rh(etablissement_id, absence_id).await;
        }
        self.decider_absence_rh(
            etablissement_id,
            role,
            absence_id,
            actor_user_id,
            Decision::Annuler,
        )
        .await
    }

    /// L'absence est-elle portée par le domaine du lot A1 (`ent_absp_absence`) ?
    async fn absence_du_domaine(&self, etab: Uuid, id: Uuid) -> Result<bool, EntDbError> {
        let (b,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_absp_absence WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        Ok(b)
    }

    async fn relire_absence_rh(&self, etab: Uuid, id: Uuid) -> Result<AbsenceRh, EntDbError> {
        Ok(self
            .absences_rh
            .get(etab, id)
            .await?
            .ok_or(DomainError::NotFound)?)
    }

    async fn decider_absence_rh(
        &self,
        etab: Uuid,
        role: &str,
        absence_id: Uuid,
        actor: Uuid,
        decision: Decision,
    ) -> Result<AbsenceRh, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let a_valider = self
            .droit_absences(etab, actor, role, Action::PersonnelsAbsencesValider)
            .await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let current = charger_verrou(&mut tx, etab, absence_id).await?;
        let autorise = a_valider
            || (decision == Decision::Annuler && current.user_id == actor && current.debut > jour);
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        // Idempotence : l'état demandé est déjà posé (et la règle de l'auto-validation passe avant).
        if decision != Decision::Annuler && actor == current.user_id {
            return Err(auto_validation());
        }
        if current.statut == decision.statut_cible() {
            return Ok(current);
        }
        let planned = match decision {
            Decision::Valider => plan_valider_absence_rh(current, actor),
            Decision::Refuser => plan_refuser_absence_rh(current, actor),
            Decision::Annuler => plan_annuler_absence_rh(current, actor),
        }
        .map_err(depuis_domaine)?;
        let sql = "UPDATE ent_absence_rh
                      SET statut = $3, valide_par_user_id = $4, valide_le = $5,
                          refuse_par_user_id = $6, refuse_le = $7,
                          annule_par_user_id = $8, annule_le = $9
                    WHERE etablissement_id = $1 AND id = $2";
        sqlx::query(sql)
            .bind(etab)
            .bind(absence_id)
            .bind(&planned.statut)
            .bind(planned.valide_par_user_id)
            .bind(planned.valide_le)
            .bind(planned.refuse_par_user_id)
            .bind(planned.refuse_le)
            .bind(planned.annule_par_user_id)
            .bind(planned.annule_le)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let (action, libelle, evenement) = decision.faits();
        journal_absence(
            &mut tx,
            etab,
            action,
            libelle,
            absence_id,
            planned.user_id,
            actor,
        )
        .await?;
        let ev = {
            let conn: &mut PgConnection = &mut tx;
            conn.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: evenement.to_string(),
                aggregate: AggregateKind::AbsenceRh,
                aggregate_id: planned.id,
                eleve_id: None,
                payload: json!({
                    "statut": planned.statut,
                    "user_id": planned.user_id,
                    "debut": planned.debut.to_string(),
                    "fin": planned.fin.to_string(),
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?
        };
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(planned)
    }

    /// Absences de personnel d'une période. **Le motif n'est rendu qu'à qui détient
    /// `personnels.absences.lire_nature`** (et à l'agent pour sa propre absence) ; qui n'a ni
    /// `lire_organisation` ni `lire_nature` ne voit que les siennes (AR-01, R-AP2).
    pub async fn list_absences_rh_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<AbsenceRh>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        let nature = self
            .droit_absences(
                etablissement_id,
                actor_user_id,
                role,
                Action::PersonnelsAbsencesLireNature,
            )
            .await?;
        let organisation = nature
            || self
                .droit_absences(
                    etablissement_id,
                    actor_user_id,
                    role,
                    Action::PersonnelsAbsencesLireOrganisation,
                )
                .await?;
        let mut liste = self
            .absences_rh
            .list_periode(etablissement_id, debut, fin)
            .await?;
        if !organisation {
            liste.retain(|a| a.user_id == actor_user_id);
        }
        if !nature {
            for a in &mut liste {
                if a.user_id != actor_user_id {
                    a.motif = MOTIF_MASQUE.to_string();
                }
            }
        }
        Ok(liste)
    }

    /// Nombre de personnels absents un jour donné (tuile de la direction : compte seulement, jamais
    /// la nature ni un nom). Port interne : l'appelant a déjà contrôlé le profil.
    pub(crate) async fn nombre_absents_rh_le(
        &self,
        etab: Uuid,
        jour: NaiveDate,
    ) -> Result<i64, EntDbError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(DISTINCT user_id) FROM ent_absence_rh
              WHERE etablissement_id = $1 AND debut <= $2 AND fin >= $2
                AND statut IN ('signalee', 'validee')",
        )
        .bind(etab)
        .bind(jour)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        Ok(n)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Decision {
    Valider,
    Refuser,
    Annuler,
}

impl Decision {
    fn statut_cible(self) -> &'static str {
        match self {
            Self::Valider => "validee",
            Self::Refuser => "refusee",
            Self::Annuler => "annulee",
        }
    }

    /// (action de journal, libellé, type d'événement).
    fn faits(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Valider => (
                "absence_validee",
                "Absence de personnel validée",
                types::ABSENCE_RH_VALIDEE,
            ),
            Self::Refuser => (
                "absence_refusee",
                "Absence de personnel refusée",
                "rh.absence_refusee",
            ),
            Self::Annuler => (
                "absence_annulee",
                "Absence de personnel annulée",
                "rh.absence_annulee",
            ),
        }
    }
}
