//! Politique de sécurité par établissement (socle 1, A2) : passkey obligatoire
//! par rôle (D-009). Tant qu'un compte soumis à l'obligation n'a pas de passkey,
//! la connexion n'ouvre qu'une session « enrôlement » (voir `auth_repo`).
//!
//! Défaut : obligation pour aucun rôle (déploiement par vagues décidé par
//! l'établissement : personnels d'abord, puis familles et élèves).

use ent_auth::Role;
use uuid::Uuid;

use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("securite_repo", err))
}

/// Politique lisible par la direction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PolitiqueSecurite {
    pub passkey_obligatoire_roles: Vec<String>,
}

impl PgEnt {
    /// Politique de l'établissement (défaut : aucune obligation).
    pub async fn politique_securite(
        &self,
        etablissement_id: Uuid,
    ) -> Result<PolitiqueSecurite, EntDbError> {
        let row: Option<(Vec<String>,)> = sqlx::query_as(
            "SELECT passkey_obligatoire_roles FROM ent_politique_securite
              WHERE etablissement_id = $1",
        )
        .bind(etablissement_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(PolitiqueSecurite {
            passkey_obligatoire_roles: row.map(|(r,)| r).unwrap_or_default(),
        })
    }

    /// La passkey est-elle obligatoire pour ce rôle dans cet établissement ?
    pub async fn passkey_obligatoire(
        &self,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<bool, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(false);
        };
        let p = self.politique_securite(etablissement_id).await?;
        Ok(p.passkey_obligatoire_roles
            .iter()
            .any(|r| Role::parse(r) == Some(parsed)))
    }

    /// Fixe la politique : Admin ou Direction seulement ; rôles inconnus refusés.
    /// Journalisé (acteur réel).
    pub async fn definir_politique_securite_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        roles: &[String],
    ) -> Result<PolitiqueSecurite, EntDbError> {
        if !matches!(Role::parse(actor_role), Some(Role::Admin | Role::Direction)) {
            return Err(EntDbError::Forbidden);
        }
        let mut canon: Vec<String> = Vec::new();
        for r in roles {
            let parsed = Role::parse(r).ok_or(EntDbError::Domain(
                domain_eleve::DomainError::ValeurInvalide,
            ))?;
            let s = parsed.as_str().to_string();
            if !canon.contains(&s) {
                canon.push(s);
            }
        }
        canon.sort();
        // Revue E5 : imposer la passkey sans moteur WebAuthn configuré
        // verrouillerait tous les comptes concernés (direction comprise).
        if !canon.is_empty() {
            self.passkey_engine()?;
        }
        sqlx::query(
            "INSERT INTO ent_politique_securite
                 (etablissement_id, passkey_obligatoire_roles, modifie_par_user_id, modifie_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (etablissement_id) DO UPDATE
                SET passkey_obligatoire_roles = EXCLUDED.passkey_obligatoire_roles,
                    modifie_par_user_id = EXCLUDED.modifie_par_user_id,
                    modifie_at = now()",
        )
        .bind(etablissement_id)
        .bind(&canon)
        .bind(actor_user_id)
        .execute(self.pool())
        .await
        .map_err(db)?;
        // Revue E5 : les sessions complètes déjà ouvertes par des comptes
        // désormais soumis et sans passkey sont fermées (la prochaine connexion
        // passe par l'enrôlement). La session de l'acteur est conservée.
        sqlx::query(
            "UPDATE ent_session s SET revoked_at = now()
               FROM ent_membership m
              WHERE s.etablissement_id = $1
                AND s.revoked_at IS NULL
                AND s.portee = 'complete'
                AND s.user_id <> $3
                AND m.user_id = s.user_id AND m.etablissement_id = s.etablissement_id
                AND m.role = ANY($2)
                AND NOT EXISTS (SELECT 1 FROM ent_passkey_credential p
                                 WHERE p.user_id = s.user_id)",
        )
        .bind(etablissement_id)
        .bind(&canon)
        .bind(actor_user_id)
        .execute(self.pool())
        .await
        .map_err(db)?;
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "politique_securite",
            "Politique de sécurité : passkey obligatoire",
            serde_json::json!({ "passkey_obligatoire_roles": canon }),
        )
        .await;
        Ok(PolitiqueSecurite {
            passkey_obligatoire_roles: canon,
        })
    }

    /// Appareils connectés de la personne (toutes ses sessions actives).
    pub async fn sessions_actives_for_actor(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::SessionAppareil>, EntDbError> {
        Ok(self.auth_repo().list_sessions_actives(user_id).await?)
    }

    /// « Déconnecter cet appareil » : seulement une session de la personne.
    pub async fn revoquer_session_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        session_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let ok = self
            .auth_repo()
            .revoke_session_of_user(user_id, session_id)
            .await?;
        if ok {
            self.audit_compte(
                etablissement_id,
                user_id,
                "",
                "session_revoquee",
                "Déconnexion d'un appareil",
                serde_json::json!({ "session_id": session_id.to_string() }),
            )
            .await;
        }
        Ok(ok)
    }
}
