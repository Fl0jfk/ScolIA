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

    /// Lecture stricte : une valeur présente mais inconnue (faute de frappe : `stricte`,
    /// `strcit`…) est une erreur, jamais un `off` silencieux (audit B-04). Absent ou vide
    /// = `off` ici ; c'est `verifier_demarrage` qui l'interdit hors labo.
    pub fn try_parse(v: Option<&str>) -> Result<Self, String> {
        match v.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            None | Some("") | Some("off") | Some("0") => Ok(Self::Off),
            Some("strict") => Ok(Self::Strict),
            Some("permissif") | Some("on") | Some("1") => Ok(Self::Permissif),
            Some(autre) => Err(format!(
                "ENT_RLS_MODE={autre:?} inconnu (valeurs : strict, permissif, off)"
            )),
        }
    }

    /// Garde de démarrage (audit B-04) : valeur inconnue → refus partout ; hors
    /// environnement non productif (`ENT_ENV` absent ou inconnu = production), seul
    /// `strict` est accepté. Renvoie le mode effectif, que l'appelant journalise.
    pub fn verifier_demarrage(v: Option<&str>, non_prod: bool) -> Result<Self, String> {
        let mode = Self::try_parse(v)?;
        if !non_prod && mode != Self::Strict {
            return Err(format!(
                "ENT_RLS_MODE doit valoir `strict` hors labo/test (mode lu : {mode:?}) : \
                 sans cela les politiques RLS restent permissives"
            ));
        }
        Ok(mode)
    }

    pub fn from_env() -> Self {
        #[cfg(test)]
        if let Some(m) = MODE_TEST.with(|c| c.get()) {
            return m;
        }
        static MODE: OnceLock<RlsMode> = OnceLock::new();
        *MODE.get_or_init(|| {
            let brut = std::env::var("ENT_RLS_MODE").ok();
            Self::try_parse(brut.as_deref()).unwrap_or_else(|e| {
                // `PgEnt::connect` refuse déjà de démarrer ; ce repli ne sert qu'aux
                // usages hors serveur (outils, tests) et ne doit pas être silencieux.
                tracing::error!("{e} : traité comme `off`");
                Self::Off
            })
        })
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

static ECHECS_ETABLISSEMENT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Nombre cumulé (depuis le démarrage) de passages de tâche de fond qui ont échoué pour un
/// établissement sans empêcher les suivants (audit R-09). À exposer en métrique / alerte.
pub fn echecs_par_etablissement() -> u64 {
    ECHECS_ETABLISSEMENT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Enregistre l'échec d'un établissement : compteur + journal technique **sans identifiant
/// d'établissement ni donnée** (E14), seulement le contexte et la nature de l'erreur.
pub fn signaler_echec_etablissement(contexte: &str, erreur: &dyn std::fmt::Display) {
    ECHECS_ETABLISSEMENT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    tracing::error!(
        target: "ent_tache_fond",
        contexte,
        erreur = %erreur,
        "tâche de fond en échec pour un établissement ; les autres établissements continuent"
    );
}

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
        // R-09 : une erreur chez un établissement ne bloque pas ceux qui le suivent.
        let r: Result<Vec<T>, sqlx::Error> = async {
            let mut tx = TenantTx::begin(pool, etab).await?;
            let mut lignes: Vec<T> = sqlx::query_as(sql).fetch_all(&mut *tx).await?;
            lignes.retain(|l| l.etablissement() == etab);
            // Validation : la sélection peut appeler une fonction SQL qui écrit (ex. `ent_eleve_statut_recalculer()`).
            tx.commit().await?;
            Ok(lignes)
        }
        .await;
        match r {
            Ok(lignes) => tout.extend(lignes),
            Err(e) => signaler_echec_etablissement(contexte.nom(), &e),
        }
    }
    Ok(tout)
}

/// Variante pour une **fonction SQL qui écrit** et renvoie ce qu'elle a changé (ex. `ent_eleve_statut_recalculer()`).
/// Sans `etablissement_id` filtré côté Rust : en mode `off` la base ne restreint pas la fonction, qui traite alors tous
/// les établissements dès le premier appel (les suivants ne trouvent plus rien) ; les lignes renvoyées ne doivent pas
/// être perdues. Sous RLS (permissif ou strict), chaque appel ne voit que son établissement : aucune ligne en double.
pub async fn appeler_par_etablissement<T>(
    pool: &PgPool,
    contexte: ContexteSysteme,
    sql: &'static str,
) -> Result<Vec<T>, sqlx::Error>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let mut tout = Vec::new();
    for etab in etablissements_systeme(pool, contexte).await? {
        let r: Result<Vec<T>, sqlx::Error> = async {
            let mut tx = TenantTx::begin(pool, etab).await?;
            let lignes: Vec<T> = sqlx::query_as(sql).fetch_all(&mut *tx).await?;
            tx.commit().await?;
            Ok(lignes)
        }
        .await;
        match r {
            Ok(lignes) => tout.extend(lignes),
            Err(e) => signaler_echec_etablissement(contexte.nom(), &e),
        }
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
        // Audit B-04 : la lecture stricte ne laisse plus passer une faute de frappe.
        assert!(RlsMode::try_parse(Some("stricte")).is_err());
        assert!(RlsMode::try_parse(Some("strcit")).is_err());
        assert_eq!(RlsMode::try_parse(Some(" Strict ")), Ok(RlsMode::Strict));
        assert_eq!(RlsMode::try_parse(None), Ok(RlsMode::Off));
        // Hors labo : seul `strict` démarre ; en labo tout mode valide passe.
        assert!(RlsMode::verifier_demarrage(None, false).is_err());
        assert!(RlsMode::verifier_demarrage(Some("permissif"), false).is_err());
        assert!(RlsMode::verifier_demarrage(Some("stricte"), true).is_err());
        assert_eq!(RlsMode::verifier_demarrage(Some("strict"), false), Ok(RlsMode::Strict));
        assert_eq!(RlsMode::verifier_demarrage(None, true), Ok(RlsMode::Off));
        assert_eq!(RlsMode::parse(Some("Permissif")), RlsMode::Permissif);
        assert_eq!(RlsMode::parse(Some(" STRICT ")), RlsMode::Strict);
    }

    /// Audit R-09 : un établissement en erreur ne bloque pas les suivants.
    #[tokio::test]
    async fn un_etablissement_en_erreur_ne_bloque_pas_les_suivants() {
        let Some(url) = crate::url_guard::url_labo_ou_echec() else {
            return;
        };
        let pool = sqlx::PgPool::connect(&url).await.expect("pool");
        let mut ent = crate::PgEnt::from_pool(pool.clone());
        ent.migrate().await.expect("migrations");
        // Deux établissements (UUID triés : le premier échoue, le second doit être traité).
        let (mut a, mut b) = (Uuid::new_v4(), Uuid::new_v4());
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        for e in [a, b] {
            sqlx::query("INSERT INTO ent_etablissement (id, slug, nom) VALUES ($1, $2, 'R09')")
                .bind(e)
                .bind(format!("r09-{}", e.simple()))
                .execute(&pool)
                .await
                .unwrap();
        }
        forcer_mode_test(Some(RlsMode::Permissif));
        // Erreur (division par zéro) uniquement dans le contexte de l'établissement `a`.
        let sql: &'static str = Box::leak(
            format!(
                "SELECT id AS etablissement_id, \
                        1 / (CASE WHEN current_setting('app.etablissement_id', true) = '{a}' THEN 0 ELSE 1 END) \
                   FROM ent_etablissement WHERE id IN ('{a}', '{b}')"
            )
            .into_boxed_str(),
        );
        let avant = echecs_par_etablissement();
        let lignes: Vec<(Uuid, i32)> =
            lire_par_etablissement(&pool, ContexteSysteme::JobPlanifie("test_r09"), sql)
                .await
                .expect("le passage global réussit malgré l'échec d'un établissement");
        let appels: Vec<(Uuid, i32)> =
            appeler_par_etablissement(&pool, ContexteSysteme::JobPlanifie("test_r09"), sql)
                .await
                .expect("idem pour les fonctions qui écrivent");
        forcer_mode_test(None);
        assert!(lignes.iter().any(|l| l.0 == b), "l'établissement suivant est traité");
        assert!(lignes.iter().all(|l| l.0 != a), "l'établissement en erreur est écarté");
        assert!(appels.iter().any(|l| l.0 == b));
        assert!(echecs_par_etablissement() >= avant + 2, "échecs comptés");
        sqlx::query("DELETE FROM ent_etablissement WHERE id = ANY($1)")
            .bind(vec![a, b])
            .execute(&pool)
            .await
            .unwrap();
    }
}
