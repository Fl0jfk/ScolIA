//! D-017 étape R2 — droits d'établissement de la requête en cours.
//!
//! Les contrôles de gestion (facturation, EDT, annonces, repas, internat,
//! sorties, élèves, matières, conseils, documents à signer) sont testés à
//! l'intérieur d'environ 150 fonctions de service qui reçoivent le profil de
//! base (`role: &str`) mais pas toujours l'identité de l'acteur. Plutôt que de
//! dupliquer la règle (route d'un côté, service de l'autre), **le moteur est
//! l'unique source** : les droits effectifs (`droits_effectifs` : modèle du
//! profil ∪ rôles attribués) sont chargés **une fois par requête** puis
//! consultés par `peut_etablissement`.
//!
//! Sécurité : l'absence de contexte (appel direct d'un test, tâche détachée)
//! retombe sur le **modèle par défaut du profil**, c'est-à-dire les droits
//! historiques exacts : on ne peut jamais obtenir plus que le profil par oubli,
//! seulement moins qu'un rôle attribué. Le contexte n'est honoré que si son
//! profil de base est celui passé à la fonction de service.

use std::future::Future;
use std::sync::Arc;

use ent_auth::{modele_par_defaut, Action, Portee, Role};
use uuid::Uuid;

use crate::droits_repo::Droit;
use crate::service::{EntDbError, PgEnt};

/// Droits effectifs d'une personne dans un établissement, figés pour la requête.
#[derive(Debug, Clone)]
pub struct DroitsActeur {
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub role_base: Role,
    pub droits: Vec<Droit>,
}

tokio::task_local! {
    static ACTEUR: Arc<DroitsActeur>;
}

impl DroitsActeur {
    fn a_etablissement(&self, action: Action) -> bool {
        self.droits
            .iter()
            .any(|d| d.portee == Portee::Etablissement && accorde(d.action, action))
    }
}

/// `accorde(detenue, demandee)` : la détention de `detenue` suffit-elle ?
fn accorde(detenue: Action, demandee: Action) -> bool {
    detenue == demandee || demandee.accordee_par().contains(&detenue)
}

impl PgEnt {
    /// Charge les droits effectifs (lus en base à chaque appel : effet immédiat).
    pub async fn charger_droits_acteur(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role_base: &str,
    ) -> Result<Option<Arc<DroitsActeur>>, EntDbError> {
        let Some(base) = Role::parse(role_base) else {
            return Ok(None);
        };
        let droits = self
            .droits_effectifs(etablissement_id, user_id, role_base)
            .await?;
        Ok(Some(Arc::new(DroitsActeur {
            etablissement_id,
            user_id,
            role_base: base,
            droits,
        })))
    }

    /// Exécute `fut` avec les droits de l'acteur : les contrôles de gestion des
    /// services en tiennent compte. Utilisé par l'API (une fois par requête) et
    /// par les tests.
    pub async fn avec_droits_acteur<F: Future>(ctx: Arc<DroitsActeur>, fut: F) -> F::Output {
        ACTEUR.scope(ctx, fut).await
    }

    /// Variante pratique : charge puis exécute.
    pub async fn avec_acteur<F: Future>(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role_base: &str,
        fut: F,
    ) -> Result<F::Output, EntDbError> {
        match self
            .charger_droits_acteur(etablissement_id, user_id, role_base)
            .await?
        {
            Some(ctx) => Ok(Self::avec_droits_acteur(ctx, fut).await),
            None => Ok(fut.await),
        }
    }

    /// L'acteur de la requête détient-il `action` sur l'établissement entier ?
    /// `role` est le profil de base porté par la session.
    pub fn peut_etablissement(role: &str, action: Action) -> bool {
        let Some(base) = Role::parse(role) else {
            return false;
        };
        if !base.is_staff() {
            return false; // garde-fou 1 : familles et élèves hors rôles libres
        }
        let ctx = ACTEUR
            .try_with(|a| a.clone())
            .ok()
            .filter(|a| a.role_base == base);
        match ctx {
            Some(a) => a.a_etablissement(action),
            None => modele_par_defaut(base)
                .iter()
                .any(|(a, p)| *p == Portee::Etablissement && accorde(*a, action)),
        }
    }

    /// Variante qui échoue en `Forbidden`.
    pub fn exiger_etablissement(role: &str, action: Action) -> Result<(), EntDbError> {
        if Self::peut_etablissement(role, action) {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }
}
