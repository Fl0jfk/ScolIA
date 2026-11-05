//! Établissements et domaines (socle 1, C2, D-012) : l'établissement est déduit
//! de l'adresse utilisée (`lpnb.scolia.fr` → établissement), jamais saisi.
//!
//! - résolution publique domaine → établissement (normalisation stricte, valeur
//!   invalide = inconnue) ;
//! - administration réservée au rôle **Admin**, limitée à **son** établissement
//!   (celui de la session), journalisée avec l'acteur réel.

use ent_auth::Role;
use uuid::Uuid;

use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};
use domain_eleve::DomainError;

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("etablissement_repo", err))
}

/// Normalise un nom d'hôte (en-tête `Host`, `X-Forwarded-Host` ou saisie
/// d'administration) : minuscules, sans espaces autour, sans port, sans point
/// final. `None` si la valeur n'est pas un nom de domaine valide (adresse IP
/// IPv6 entre crochets, caractères interdits, étiquette vide ou trop longue,
/// port non numérique…).
pub fn normaliser_domaine(brut: &str) -> Option<String> {
    let s = brut.trim();
    if s.is_empty() || s.len() > 260 || s.starts_with('[') {
        return None;
    }
    // Port éventuel : `hote:8080` (un seul `:`, chiffres seulement).
    let hote = match s.rsplit_once(':') {
        Some((h, port)) => {
            if port.is_empty()
                || port.len() > 5
                || !port.bytes().all(|b| b.is_ascii_digit())
                || h.contains(':')
            {
                return None;
            }
            h
        }
        None => s,
    };
    let hote = hote.strip_suffix('.').unwrap_or(hote).to_ascii_lowercase();
    if hote.is_empty() || hote.len() > 253 {
        return None;
    }
    for etiquette in hote.split('.') {
        let b = etiquette.as_bytes();
        if b.is_empty()
            || b.len() > 63
            || b[0] == b'-'
            || b[b.len() - 1] == b'-'
            || !b
                .iter()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
        {
            return None;
        }
    }
    Some(hote)
}

/// Identifiant court d'établissement (`lpnb`) : `[a-z0-9-]`, 1 à 63 caractères,
/// sans tiret aux extrémités.
fn slug_valide(slug: &str) -> bool {
    let b = slug.as_bytes();
    !b.is_empty()
        && b.len() <= 63
        && b[0] != b'-'
        && b[b.len() - 1] != b'-'
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// Établissement tel que présenté publiquement (page de connexion).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtablissementInfo {
    pub id: Uuid,
    pub slug: String,
    pub nom: String,
    pub actif: bool,
}

fn exiger_admin(actor_role: &str) -> Result<(), EntDbError> {
    if Role::parse(actor_role) == Some(Role::Admin) {
        Ok(())
    } else {
        Err(EntDbError::Forbidden)
    }
}

impl PgEnt {
    /// Établissement servi par ce nom d'hôte (actif seulement). Une valeur
    /// invalide ou inconnue donne `None` — jamais d'erreur côté client.
    pub async fn resoudre_domaine(&self, hote: &str) -> Result<Option<Uuid>, EntDbError> {
        let Some(domaine) = normaliser_domaine(hote) else {
            return Ok(None);
        };
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT e.id
               FROM ent_etablissement_domaine d
               JOIN ent_etablissement e ON e.id = d.etablissement_id
              WHERE d.domaine = $1 AND e.actif",
        )
        .bind(&domaine)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(row.map(|(id,)| id))
    }

    /// Fiche publique (slug, nom) d'un établissement déclaré.
    pub async fn etablissement_info(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Option<EtablissementInfo>, EntDbError> {
        let row: Option<(Uuid, String, String, bool)> =
            sqlx::query_as("SELECT id, slug, nom, actif FROM ent_etablissement WHERE id = $1")
                .bind(etablissement_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?;
        Ok(row.map(|(id, slug, nom, actif)| EtablissementInfo {
            id,
            slug,
            nom,
            actif,
        }))
    }

    /// Déclare (ou renomme) l'établissement **de la session**. Admin seulement.
    pub async fn declarer_etablissement_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        slug: &str,
        nom: &str,
    ) -> Result<EtablissementInfo, EntDbError> {
        exiger_admin(actor_role)?;
        let slug = slug.trim().to_ascii_lowercase();
        let nom = nom.trim();
        if !slug_valide(&slug) || nom.is_empty() || nom.chars().count() > 200 {
            return Err(EntDbError::Domain(DomainError::ValeurInvalide));
        }
        let (id, slug, nom, actif): (Uuid, String, String, bool) = sqlx::query_as(
            "INSERT INTO ent_etablissement (id, slug, nom)
             VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE SET slug = EXCLUDED.slug, nom = EXCLUDED.nom
             RETURNING id, slug, nom, actif",
        )
        .bind(etablissement_id)
        .bind(&slug)
        .bind(nom)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "etablissement_declare",
            "Établissement déclaré ou renommé",
            serde_json::json!({ "slug": slug }),
        )
        .await;
        Ok(EtablissementInfo {
            id,
            slug,
            nom,
            actif,
        })
    }

    /// Domaines de l'établissement de la session. Admin seulement.
    pub async fn domaines_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_role: &str,
    ) -> Result<Vec<String>, EntDbError> {
        exiger_admin(actor_role)?;
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT domaine FROM ent_etablissement_domaine
              WHERE etablissement_id = $1 ORDER BY domaine",
        )
        .bind(etablissement_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(|(d,)| d).collect())
    }

    /// Rattache un domaine à l'établissement de la session. Admin seulement ;
    /// l'établissement doit être déclaré ; un domaine déjà servi par un autre
    /// établissement est refusé (conflit), jamais réattribué. Idempotent.
    pub async fn ajouter_domaine_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        domaine: &str,
    ) -> Result<String, EntDbError> {
        exiger_admin(actor_role)?;
        let domaine =
            normaliser_domaine(domaine).ok_or(EntDbError::Domain(DomainError::ValeurInvalide))?;
        let inseree: Option<(Uuid,)> = sqlx::query_as(
            "INSERT INTO ent_etablissement_domaine (domaine, etablissement_id)
             VALUES ($1, $2)
             ON CONFLICT (domaine) DO NOTHING
             RETURNING etablissement_id",
        )
        .bind(&domaine)
        .bind(etablissement_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        if inseree.is_none() {
            let proprietaire: Option<(Uuid,)> = sqlx::query_as(
                "SELECT etablissement_id FROM ent_etablissement_domaine WHERE domaine = $1",
            )
            .bind(&domaine)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
            if proprietaire.map(|(e,)| e) != Some(etablissement_id) {
                return Err(EntDbError::Domain(DomainError::Conflit));
            }
            return Ok(domaine);
        }
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "etablissement_domaine_ajoute",
            "Domaine rattaché à l'établissement",
            serde_json::json!({ "domaine": domaine }),
        )
        .await;
        Ok(domaine)
    }

    /// Retire un domaine de l'établissement de la session. Admin seulement.
    pub async fn retirer_domaine_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        domaine: &str,
    ) -> Result<bool, EntDbError> {
        exiger_admin(actor_role)?;
        let Some(domaine) = normaliser_domaine(domaine) else {
            return Ok(false);
        };
        let n = sqlx::query(
            "DELETE FROM ent_etablissement_domaine
              WHERE domaine = $1 AND etablissement_id = $2",
        )
        .bind(&domaine)
        .bind(etablissement_id)
        .execute(self.pool())
        .await
        .map_err(db)?
        .rows_affected();
        if n > 0 {
            self.audit_compte(
                etablissement_id,
                actor_user_id,
                actor_role,
                "etablissement_domaine_retire",
                "Domaine retiré de l'établissement",
                serde_json::json!({ "domaine": domaine }),
            )
            .await;
        }
        Ok(n > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation_des_domaines() {
        assert_eq!(
            normaliser_domaine("LPNB.Scolia.FR").as_deref(),
            Some("lpnb.scolia.fr")
        );
        assert_eq!(
            normaliser_domaine(" lpnb.scolia.fr:8443 ").as_deref(),
            Some("lpnb.scolia.fr")
        );
        assert_eq!(
            normaliser_domaine("lpnb.scolia.fr.").as_deref(),
            Some("lpnb.scolia.fr")
        );
        assert_eq!(
            normaliser_domaine("lpnb.scolia.fr.:443").as_deref(),
            Some("lpnb.scolia.fr")
        );
        assert_eq!(
            normaliser_domaine("localhost").as_deref(),
            Some("localhost")
        );
        assert_eq!(
            normaliser_domaine("xn--lyce-dpa.scolia.fr").as_deref(),
            Some("xn--lyce-dpa.scolia.fr")
        );
        for invalide in [
            "",
            " ",
            ".",
            "a..b",
            ".scolia.fr",
            "-a.scolia.fr",
            "a-.scolia.fr",
            "lpnb.scolia.fr:",
            "lpnb.scolia.fr:abc",
            "lpnb.scolia.fr:123456",
            "[::1]:8080",
            "::1",
            "lp nb.scolia.fr",
            "lpnb_x.scolia.fr",
            "lycée.scolia.fr",
            "a/b",
            "user@lpnb.scolia.fr",
        ] {
            assert_eq!(normaliser_domaine(invalide), None, "{invalide:?}");
        }
        assert_eq!(normaliser_domaine(&format!("{}.fr", "a".repeat(64))), None);
    }

    #[test]
    fn slugs() {
        assert!(slug_valide("lpnb"));
        assert!(slug_valide("lycee-1"));
        assert!(!slug_valide(""));
        assert!(!slug_valide("-a"));
        assert!(!slug_valide("A"));
        assert!(!slug_valide("a_b"));
    }
}
