//! Transaction « locataire » : ouvre une transaction et y pose
//! `app.etablissement_id` (`set_config(..., true)` = valable pour la seule
//! transaction, jamais conservé sur la connexion rendue au pool).
//!
//! C'est le pendant applicatif de la RLS (migration 100, `docs/RLS_ISOLATION.md`).
//! Mode `ENT_RLS_MODE` (lu une fois) :
//! - `off` (défaut) : aucune variable posée -> les politiques restent permissives ;
//! - `permissif` : `app.etablissement_id` posé ; une requête sans variable reste permise ;
//! - `strict` : en plus `app.rls_strict = on` pour la transaction (à défaut d'un
//!   réglage au niveau du rôle, recommandé : `ALTER ROLE ... SET app.rls_strict = 'on'`).

use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlsMode {
    Off,
    Permissif,
    Strict,
}

impl RlsMode {
    pub fn parse(v: Option<&str>) -> Self {
        match v.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            Some("strict") => Self::Strict,
            Some("permissif") | Some("on") | Some("1") => Self::Permissif,
            _ => Self::Off,
        }
    }

    pub fn from_env() -> Self {
        #[cfg(test)]
        if let Some(m) = MODE_TEST.with(|c| c.get()) {
            return m;
        }
        static MODE: OnceLock<RlsMode> = OnceLock::new();
        *MODE.get_or_init(|| Self::parse(std::env::var("ENT_RLS_MODE").ok().as_deref()))
    }
}

// Tests : force le mode pour le fil courant (`#[tokio::test]` = un seul fil), sans
// toucher à la variable d'environnement lue une seule fois par processus.
#[cfg(test)]
thread_local! {
    static MODE_TEST: std::cell::Cell<Option<RlsMode>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn forcer_mode_test(mode: Option<RlsMode>) {
    MODE_TEST.with(|c| c.set(mode));
}

pub struct TenantTx<'c> {
    tx: Transaction<'c, Postgres>,
}

impl TenantTx<'static> {
    /// Ouvre la transaction et pose l'établissement (selon `ENT_RLS_MODE`).
    pub async fn begin(pool: &PgPool, etablissement_id: Uuid) -> Result<Self, sqlx::Error> {
        Self::begin_mode(pool, etablissement_id, RlsMode::from_env()).await
    }

    pub async fn begin_mode(
        pool: &PgPool,
        etablissement_id: Uuid,
        mode: RlsMode,
    ) -> Result<Self, sqlx::Error> {
        let mut tx = pool.begin().await?;
        if mode != RlsMode::Off {
            sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
                .bind(etablissement_id.to_string())
                .execute(&mut *tx)
                .await?;
            if mode == RlsMode::Strict {
                sqlx::query("SELECT set_config('app.rls_strict', 'on', true)")
                    .execute(&mut *tx)
                    .await?;
            }
        }
        Ok(Self { tx })
    }
}

impl TenantTx<'_> {
    pub async fn commit(self) -> Result<(), sqlx::Error> {
        self.tx.commit().await
    }

    /// Annule explicitement la transaction (sinon l'annulation est implicite à la destruction).
    pub async fn rollback(self) -> Result<(), sqlx::Error> {
        self.tx.rollback().await
    }
}

// ---------------------------------------------------------------------------------------------
// Accès légitimement transverses (docs/securite/RLS_STRICTE.md §4)
// ---------------------------------------------------------------------------------------------
//
// Règle : **jamais de contournement de la RLS** (ni `BYPASSRLS`, ni politique « système »). Un
// traitement qui doit parcourir plusieurs établissements (tâche planifiée, worker d'envoi,
// rechiffrement) liste d'abord les établissements dans `ent_etablissement` (table sans RLS, lue
// avant tout contexte), puis ouvre **un `TenantTx` par établissement**. Chaque usage est nommé
// par un [`ContexteSysteme`] (liste fermée, revue à chaque ajout) et journalisé
// (`target = "ent_rls_systeme"`), de sorte qu'un accès transverse se retrouve par recherche.

/// Usage transverse nommé. Ajouter une variante = ajouter un accès transverse : à justifier dans
/// `docs/securite/RLS_STRICTE.md` §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContexteSysteme {
    /// Détection nocturne des doublons d'élèves.
    DoublonsEleves,
    /// Tâche nocturne élèves (statuts dérivés, sorties, majorité, identités).
    TacheNocturneEleves,
    /// Expiration des propositions de correction échues.
    ExpirationPropositions,
    /// Relances et échéances de facturation (jobs planifiés).
    JobsFinances,
    /// Workers d'envoi (e-mail, SMS, push) : un lot par établissement.
    WorkerEnvoi,
    /// Rechiffrement des champs sensibles au démarrage.
    Rechiffrement,
    /// Autre job planifié (nom libre, journalisé).
    JobPlanifie(&'static str),
}

impl ContexteSysteme {
    pub fn nom(self) -> &'static str {
        match self {
            Self::DoublonsEleves => "doublons_eleves",
            Self::TacheNocturneEleves => "tache_nocturne_eleves",
            Self::ExpirationPropositions => "expiration_propositions",
            Self::JobsFinances => "jobs_finances",
            Self::WorkerEnvoi => "worker_envoi",
            Self::Rechiffrement => "rechiffrement",
            Self::JobPlanifie(n) => n,
        }
    }
}

/// Établissements à parcourir pour un usage transverse nommé (table `ent_etablissement`, sans RLS).
pub async fn etablissements_systeme(
    pool: &PgPool,
    contexte: ContexteSysteme,
) -> Result<Vec<Uuid>, sqlx::Error> {
    // Racine des établissements + établissements portant des adhésions : deux tables sans RLS
    // (lues avant tout contexte). Jamais une table sous RLS.
    let ids: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_etablissement UNION SELECT etablissement_id FROM ent_membership ORDER BY 1",
    )
    .fetch_all(pool)
    .await?;
    tracing::info!(target: "ent_rls_systeme", contexte = contexte.nom(), etablissements = ids.len(), "acces_systeme");
    Ok(ids.into_iter().map(|(i,)| i).collect())
}

/// Ligne dont la première colonne est l'identifiant d'établissement.
pub trait PremiereColonneEtab {
    fn etablissement(&self) -> Uuid;
}
macro_rules! premiere_colonne_etab {
    ($($t:ident),*) => {
        impl<$($t),*> PremiereColonneEtab for (Uuid, $($t,)*) {
            fn etablissement(&self) -> Uuid { self.0 }
        }
    };
}
premiere_colonne_etab!();
premiere_colonne_etab!(A);
premiere_colonne_etab!(A, B);
premiere_colonne_etab!(A, B, C);
premiere_colonne_etab!(A, B, C, D);
premiere_colonne_etab!(A, B, C, D, E);

/// Exécute une requête de sélection **sans paramètre**, dont la première colonne est
/// `etablissement_id`, une fois par établissement, dans le contexte de celui-ci. Les lignes
/// d'un autre établissement sont écartées (en mode `off` la base ne filtre pas : sans cela chaque
/// ligne serait renvoyée autant de fois qu'il y a d'établissements).
pub async fn lire_par_etablissement<T>(
    pool: &PgPool,
    contexte: ContexteSysteme,
    sql: &'static str,
) -> Result<Vec<T>, sqlx::Error>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + PremiereColonneEtab + Send + Unpin,
{
    let mut tout = Vec::new();
    for etab in etablissements_systeme(pool, contexte).await? {
        let mut tx = TenantTx::begin(pool, etab).await?;
        let mut lignes: Vec<T> = sqlx::query_as(sql).fetch_all(&mut *tx).await?;
        lignes.retain(|l| l.etablissement() == etab);
        tout.extend(lignes);
    }
    Ok(tout)
}

/// Validation d'une transaction, qu'elle soit « locataire » ou nue (aides de service génériques
/// pendant la conversion des domaines vers [`TenantTx`]).
pub trait Validable: Send {
    fn valider(self) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl Validable for TenantTx<'_> {
    fn valider(self) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send {
        async move { self.tx.commit().await }
    }
}

impl Validable for Transaction<'_, Postgres> {
    fn valider(self) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send {
        async move { self.commit().await }
    }
}

impl Deref for TenantTx<'_> {
    type Target = PgConnection;
    fn deref(&self) -> &PgConnection {
        &self.tx
    }
}

impl DerefMut for TenantTx<'_> {
    fn deref_mut(&mut self) -> &mut PgConnection {
        &mut self.tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_par_defaut_desactive() {
        assert_eq!(RlsMode::parse(None), RlsMode::Off);
        assert_eq!(RlsMode::parse(Some("")), RlsMode::Off);
        assert_eq!(RlsMode::parse(Some("bizarre")), RlsMode::Off);
        assert_eq!(RlsMode::parse(Some("Permissif")), RlsMode::Permissif);
        assert_eq!(RlsMode::parse(Some(" STRICT ")), RlsMode::Strict);
    }
}
