//! Module « santé » activable par établissement (S2-11).
//!
//! Règle produit : **pas de données de santé tant que l'hébergement n'est pas certifié HDS**.
//! Le module est donc **désactivé par défaut** pour chaque établissement : l'absence de ligne
//! dans `ent_module_etablissement` vaut « désactivé ». Seul un administrateur (droit
//! `securite.politique`, établissement entier) peut l'activer ou le désactiver ; chaque
//! changement est journalisé. Le refus des routes de santé est appliqué côté serveur par
//! `ent-api` (`sante::exiger_module`), et une erreur de lecture du paramètre refuse aussi.

use ent_auth::Action;
use serde_json::json;
use uuid::Uuid;

use crate::ref_commun::db;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

impl PgEnt {
    /// Le module santé est-il actif pour cet établissement ? Fermé par défaut.
    pub async fn module_sante_actif(&self, etablissement_id: Uuid) -> Result<bool, EntDbError> {
        let actif: Option<bool> = sqlx::query_scalar(
            "SELECT actif FROM ent_module_etablissement
              WHERE etablissement_id = $1 AND module = 'sante'",
        )
        .bind(etablissement_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await
        .map_err(db)?;
        Ok(actif.unwrap_or(false))
    }

    /// Active ou désactive le module santé de l'établissement de l'acteur. Réservé aux
    /// administrateurs (`securite.politique`, établissement entier) ; journalisé.
    pub async fn definir_module_sante(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        actif: bool,
    ) -> Result<bool, EntDbError> {
        if !self
            .a_action_etablissement(etablissement_id, acteur, role, Action::SecuritePolitique)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = crate::tenant_tx::TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_module_etablissement (etablissement_id, module, actif, modifie_par)
             VALUES ($1, 'sante', $2, $3)
             ON CONFLICT (etablissement_id, module)
             DO UPDATE SET actif = EXCLUDED.actif, modifie_par = EXCLUDED.modifie_par, modifie_le = now()",
        )
        .bind(etablissement_id)
        .bind(actif)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.audit_compte(
            etablissement_id,
            acteur,
            role,
            if actif { "module_sante_active" } else { "module_sante_desactive" },
            "Module santé (activation par l'établissement)",
            json!({ "actif": actif }),
        )
        .await;
        Ok(actif)
    }
}

impl PgEnt {
    /// Verrou « pas de santé sans HDS » **dans la couche métier** (audit D-02) : toute fonction
    /// qui lit ou écrit une donnée de santé, d'accompagnement ou de psychologie l'appelle en
    /// premier, de sorte qu'une nouvelle route HTTP ou un autre module ne puisse pas y échapper.
    /// Module inactif (ou absent) : `Forbidden`. Erreur de lecture : refus (fermé par défaut).
    pub async fn exiger_module_sante(&self, etablissement_id: Uuid) -> Result<(), EntDbError> {
        if self.module_sante_actif(etablissement_id).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }
}

#[cfg(test)]
impl PgEnt {
    /// Tests : active le module santé d'un établissement sans acteur (le verrou D-02 le demande).
    pub(crate) async fn activer_module_sante_test(&self, etablissement_id: Uuid) {
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await.expect("tx");
        sqlx::query(
            "INSERT INTO ent_module_etablissement (etablissement_id, module, actif)
             VALUES ($1, 'sante', true)
             ON CONFLICT (etablissement_id, module) DO UPDATE SET actif = true",
        )
        .bind(etablissement_id)
        .execute(&mut *tx)
        .await
        .expect("activation module santé");
        tx.commit().await.expect("commit");
    }
}
