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
