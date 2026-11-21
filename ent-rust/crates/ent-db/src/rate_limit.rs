//! Limitation de débit partagée entre instances (seau à jetons en PostgreSQL).
//!
//! Chaque seau est une ligne de `ent_rate_limit` identifiée par une empreinte
//! `route | type de clé | SHA-256(valeur)` : ni adresse e-mail ni IP en clair
//! en base. La décision est prise par **une seule requête atomique**
//! (`INSERT … ON CONFLICT … DO UPDATE … RETURNING`) : recharge du seau selon le
//! temps écoulé, retrait d'un jeton s'il en reste, sinon refus (le seau n'est
//! alors pas débité). Deux instances ne peuvent donc pas dépasser le quota en
//! course.
//!
//! Politique de panne : si la base répond en erreur, la limitation est
//! **ouverte** (la requête passe, l'erreur est journalisée) — les routes
//! protégées dépendent elles-mêmes de la base, une panne ne doit pas s'ajouter
//! un second mode de défaillance.

use ent_auth::hash_token;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};

/// Quota d'un seau : `max` requêtes par `fenetre_secs` (rafale initiale = `max`,
/// recharge continue à `max / fenetre_secs` jetons par seconde).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quota {
    pub max: u32,
    pub fenetre_secs: u32,
}

impl Quota {
    pub const fn new(max: u32, fenetre_secs: u32) -> Self {
        Self { max, fenetre_secs }
    }

    /// Analyse `"max/secondes"` (ex. `"10/900"`). `off`, `0`, `none` ou
    /// `illimite` → `Ok(None)` (limitation désactivée pour ce seau).
    pub fn analyser(valeur: &str) -> Result<Option<Quota>, String> {
        let v = valeur.trim().to_ascii_lowercase();
        if matches!(
            v.as_str(),
            "off" | "0" | "none" | "non" | "illimite" | "illimité"
        ) {
            return Ok(None);
        }
        let (a, b) = v
            .split_once('/')
            .ok_or_else(|| format!("quota « {valeur} » : format attendu max/secondes"))?;
        let max: u32 = a
            .trim()
            .parse()
            .map_err(|_| format!("quota « {valeur} » : max invalide"))?;
        let fenetre: u32 = b
            .trim()
            .parse()
            .map_err(|_| format!("quota « {valeur} » : fenêtre invalide"))?;
        if max == 0 || fenetre == 0 {
            return Ok(None);
        }
        Ok(Some(Quota::new(max, fenetre)))
    }

    /// Quota lu dans la variable d'environnement `nom` (format `max/secondes`),
    /// à défaut ou si invalide : `defaut` (`None` = désactivé par défaut).
    pub fn depuis_env(nom: &str, defaut: Option<Quota>) -> Option<Quota> {
        match std::env::var(nom) {
            Ok(v) if !v.trim().is_empty() => match Self::analyser(&v) {
                Ok(q) => q,
                Err(e) => {
                    tracing::warn!("{nom} ignoré ({e}) : quota par défaut conservé");
                    defaut
                }
            },
            _ => defaut,
        }
    }
}

/// Décision d'un contrôle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Autorise,
    /// Quota épuisé ; `retry_after_secs` ≥ 1.
    Limite {
        retry_after_secs: u64,
    },
}

impl Decision {
    pub fn est_limite(&self) -> bool {
        matches!(self, Decision::Limite { .. })
    }
}

/// Longueur maximale d'un identifiant avant empreinte (borne le coût).
const IDENT_MAX: usize = 320;
/// Un appel sur N purge les seaux inactifs.
const PURGE_TOUS_LES: u64 = 512;
static APPELS: AtomicU64 = AtomicU64::new(0);

/// Normalise un identifiant (e-mail…) pour la clé : espaces retirés,
/// minuscules ASCII, tronqué.
pub fn normaliser_identifiant(valeur: &str) -> String {
    valeur
        .trim()
        .chars()
        .take(IDENT_MAX)
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Clé de seau : `route|type|empreinte(valeur)`.
pub fn cle_seau(route: &str, type_cle: &str, valeur: &str) -> String {
    let h = hash_token(valeur);
    format!("{route}|{type_cle}|{}", &h[..32])
}

#[derive(Clone)]
pub struct RateLimiter {
    pool: PgPool,
}

impl RateLimiter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Débite un jeton du seau `(route, type_cle, valeur)`.
    /// `valeur` est normalisée (minuscules, trim) avant empreinte.
    pub async fn verifier(
        &self,
        route: &str,
        type_cle: &str,
        valeur: &str,
        quota: Quota,
    ) -> Decision {
        let cle = cle_seau(route, type_cle, &normaliser_identifiant(valeur));
        let capacite = f64::from(quota.max);
        let debit = capacite / f64::from(quota.fenetre_secs);
        // `refill` est recopié dans chaque expression : PostgreSQL n'autorise pas
        // de réutiliser un alias dans ON CONFLICT DO UPDATE.
        let res: Result<(bool, f64), sqlx::Error> = sqlx::query_as(
            r#"
            INSERT INTO ent_rate_limit AS r (bucket_key, tokens, updated_at, last_ok)
            VALUES ($1, $2::float8 - 1.0, now(), TRUE)
            ON CONFLICT (bucket_key) DO UPDATE SET
                tokens = CASE
                    WHEN LEAST($2::float8, r.tokens
                         + GREATEST(EXTRACT(EPOCH FROM (now() - r.updated_at)), 0) * $3::float8) >= 1.0
                    THEN LEAST($2::float8, r.tokens
                         + GREATEST(EXTRACT(EPOCH FROM (now() - r.updated_at)), 0) * $3::float8) - 1.0
                    ELSE LEAST($2::float8, r.tokens
                         + GREATEST(EXTRACT(EPOCH FROM (now() - r.updated_at)), 0) * $3::float8)
                END,
                last_ok = LEAST($2::float8, r.tokens
                         + GREATEST(EXTRACT(EPOCH FROM (now() - r.updated_at)), 0) * $3::float8) >= 1.0,
                updated_at = now()
            RETURNING last_ok, tokens
            "#,
        )
        .bind(&cle)
        .bind(capacite)
        .bind(debit)
        .fetch_one(&self.pool)
        .await;

        let decision = match res {
            Ok((true, _)) => Decision::Autorise,
            Ok((false, jetons)) => {
                let manque = (1.0 - jetons).max(0.0);
                let secs = (manque / debit).ceil().max(1.0);
                Decision::Limite {
                    retry_after_secs: secs as u64,
                }
            }
            Err(e) => {
                tracing::error!("limitation de débit indisponible (ouverte) : {e}");
                Decision::Autorise
            }
        };

        if APPELS.fetch_add(1, Ordering::Relaxed) % PURGE_TOUS_LES == PURGE_TOUS_LES - 1 {
            let _ = self.purger().await;
        }
        decision
    }

    /// Supprime les seaux inactifs depuis plus d'un jour (plus long que toute
    /// fenêtre configurée raisonnable : un seau absent équivaut à un seau plein).
    pub async fn purger(&self) -> Result<u64, sqlx::Error> {
        let r =
            sqlx::query("DELETE FROM ent_rate_limit WHERE updated_at < now() - interval '1 day'")
                .execute(&self.pool)
                .await?;
        Ok(r.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyse_quota() {
        assert_eq!(Quota::analyser("10/900"), Ok(Some(Quota::new(10, 900))));
        assert_eq!(Quota::analyser(" 5 / 60 "), Ok(Some(Quota::new(5, 60))));
        assert_eq!(Quota::analyser("off"), Ok(None));
        assert_eq!(Quota::analyser("0/60"), Ok(None));
        assert!(Quota::analyser("abc").is_err());
        assert!(Quota::analyser("1/x").is_err());
    }

    #[test]
    fn cle_ne_contient_pas_la_valeur() {
        let k = cle_seau("forgot", "id", "victime@ecole.fr");
        assert!(k.starts_with("forgot|id|"));
        assert!(!k.contains("victime"));
        assert_eq!(k, cle_seau("forgot", "id", "victime@ecole.fr"));
        assert_ne!(k, cle_seau("forgot", "ip", "victime@ecole.fr"));
    }

    #[test]
    fn identifiant_normalise() {
        assert_eq!(normaliser_identifiant("  Foo@BAR.fr "), "foo@bar.fr");
        assert_eq!(normaliser_identifiant(&"a".repeat(1000)).len(), IDENT_MAX);
    }
}
