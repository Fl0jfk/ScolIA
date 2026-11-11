//! Briques communes des modules du référentiel S2-2 : acteur, droits par le moteur
//! (`ReferentielLire` / `ReferentielGerer`, D-017) et publication transactionnelle
//! des faits (outbox, S2-0).

use ent_auth::Action;
use ent_events::{AggregateKind, MetierEvent, MetierEventDraft};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};
use domain_eleve::DomainError;

/// Acteur d'une opération : établissement de la session, utilisateur, profil de base.
#[derive(Debug, Clone, Copy)]
pub struct Acteur<'a> {
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub role: &'a str,
}

/// Pose l'établissement courant pour la transaction (variable locale `app.etablissement_id`) dans
/// une transaction ouverte à l'ancienne (`pool().begin()`), pour que les tables à RLS du socle 5
/// que ces anciens chemins touchent (périodisations, grilles) restent lisibles et inscriptibles
/// sous un rôle en mode strict. Identique à ce que fait `TenantTx::begin`.
pub(crate) async fn contexte_etab(conn: &mut PgConnection, etab: Uuid) -> Result<(), EntDbError> {
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(conn)
        .await
        .map_err(db)?;
    Ok(())
}

pub(crate) fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("referentiel_s2", err))
}

pub(crate) fn invalide() -> EntDbError {
    EntDbError::Domain(DomainError::ValeurInvalide)
}

pub(crate) fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

pub(crate) fn conflit() -> EntDbError {
    EntDbError::Domain(DomainError::Conflit)
}

impl PgEnt {
    /// Variante booléenne de [`PgEnt::exiger_referentiel`] pour la couche API.
    pub async fn ref_peut(&self, a: &Acteur<'_>, action: Action) -> Result<bool, EntDbError> {
        match self.exiger_referentiel(a, action).await {
            Ok(()) => Ok(true),
            Err(EntDbError::Forbidden) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Vérifie une action du catalogue sur l'établissement entier (moteur de droits).
    /// La gestion implique la lecture.
    pub(crate) async fn exiger_referentiel(
        &self,
        a: &Acteur<'_>,
        action: Action,
    ) -> Result<(), EntDbError> {
        let droits = self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?;
        let ok = droits.iter().any(|d| {
            d.portee == ent_auth::Portee::Etablissement
                && (d.action == action
                    || (action == Action::ReferentielLire && d.action == Action::ReferentielGerer))
        });
        if ok {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }
}

/// Publie un fait du référentiel dans la transaction de l'écriture (même commit).
pub(crate) async fn publier_ref(
    tx: &mut sqlx::PgConnection,
    a: &Acteur<'_>,
    type_name: &str,
    aggregate_id: Uuid,
    payload: serde_json::Value,
) -> Result<MetierEvent, EntDbError> {
    publier_agg(tx, a, AggregateKind::Referentiel, type_name, aggregate_id, payload).await
}

/// Variante pour un fait d'un autre agrégat (ex. `edt.seances_generees`).
pub(crate) async fn publier_agg(
    tx: &mut sqlx::PgConnection,
    a: &Acteur<'_>,
    aggregate: AggregateKind,
    type_name: &str,
    aggregate_id: Uuid,
    payload: serde_json::Value,
) -> Result<MetierEvent, EntDbError> {
    let conn: &mut PgConnection = tx;
    Ok(conn
        .publier(MetierEventDraft {
            etablissement_id: a.etablissement_id,
            type_name: type_name.to_string(),
            aggregate,
            aggregate_id,
            eleve_id: None,
            payload,
            actor_user_id: Some(a.user_id.to_string()),
        })
        .await?)
}
