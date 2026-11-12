//! Établissements et domaines (socle 1, C2, D-012) : l'établissement est déduit
//! de l'adresse utilisée (`lpnb.scolia.fr` → établissement), jamais saisi.
//!
//! - résolution publique domaine → établissement (normalisation stricte, valeur
//!   invalide = inconnue) ;
//! - un administrateur d'établissement (rôle **Admin**) déclare son établissement et
//!   en renomme le nom affiché, journalisé avec l'acteur réel ; le **slug** est
//!   immuable pour lui ;
//! - les **domaines** sont validés par la plateforme (audit Fable 1, B-02) : ajout et
//!   retrait réservés à l'opérateur ScolIA (`ent-api plateforme-domaine …`), jamais à
//!   un administrateur d'établissement, hôtes réservés de la plateforme refusés.
//!   Voir `docs/audits/CORRECTIF_S01_B02.md`.

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

/// Domaine racine de la plateforme (toujours réservé, en plus de `ENT_RP_ID`).
pub const DOMAINE_PLATEFORME: &str = "scolia.fr";

/// Libellés réservés directement sous le domaine de plateforme
/// (`docs/socles/ORGANISME_ADRESSES.md` §6.5).
pub const LIBELLES_RESERVES: &[&str] = &[
    "www", "admin", "api", "app", "auth", "support", "status", "demo", "aide", "mail",
    "static", "fichiers",
];

/// `true` si l'hôte (déjà normalisé) est un hôte de la plateforme qu'aucun
/// établissement ne peut servir : le domaine de plateforme lui-même ou un de ses
/// libellés réservés. `rp_id` : valeur de `ENT_RP_ID` si elle est définie.
pub fn hote_reserve_plateforme(domaine: &str, rp_id: Option<&str>) -> bool {
    let mut racines = vec![DOMAINE_PLATEFORME.to_string()];
    if let Some(r) = rp_id.and_then(normaliser_domaine) {
        if !racines.contains(&r) {
            racines.push(r);
        }
    }
    racines.iter().any(|racine| {
        domaine == racine
            || domaine
                .strip_suffix(racine.as_str())
                .and_then(|t| t.strip_suffix('.'))
                .is_some_and(|libelle| LIBELLES_RESERVES.contains(&libelle))
    })
}

fn rp_id_env() -> Option<String> {
    std::env::var("ENT_RP_ID").ok().filter(|v| !v.trim().is_empty())
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
    ///
    /// B-02 : une fois l'établissement déclaré, son **slug est immuable** pour
    /// l'administrateur (seul le nom affiché change) ; un autre slug → `Forbidden`.
    /// Le changement de slug passe par l'opérateur
    /// ([`PgEnt::declarer_etablissement_plateforme`]).
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
        let ligne: Option<(Uuid, String, String, bool)> = sqlx::query_as(
            "INSERT INTO ent_etablissement (id, slug, nom)
             VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE SET nom = EXCLUDED.nom
              WHERE ent_etablissement.slug = EXCLUDED.slug
             RETURNING id, slug, nom, actif",
        )
        .bind(etablissement_id)
        .bind(&slug)
        .bind(nom)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        // Aucune ligne : l'établissement existe avec un autre slug (immuable ici).
        let (id, slug, nom, actif) = ligne.ok_or(EntDbError::Forbidden)?;
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

    /// Ajout d'un domaine par un administrateur d'établissement : **refusé**
    /// (audit Fable 1, B-02). Aucune preuve de possession n'était demandée et le
    /// premier arrivé était servi (`college-b.scolia.fr`, `app.scolia.fr`…). Le
    /// domaine est validé par la plateforme : [`PgEnt::ajouter_domaine_plateforme`].
    /// La preuve DNS des domaines personnalisés est à venir
    /// (`docs/audits/CORRECTIF_S01_B02.md` §2).
    pub async fn ajouter_domaine_for_actor(
        &self,
        _etablissement_id: Uuid,
        _actor_user_id: Uuid,
        actor_role: &str,
        _domaine: &str,
    ) -> Result<String, EntDbError> {
        exiger_admin(actor_role)?;
        Err(EntDbError::Forbidden)
    }

    /// Retrait d'un domaine par un administrateur d'établissement : **refusé**
    /// (B-02) ; un compte d'administrateur volé ne coupe plus l'adresse de
    /// l'établissement. Voir [`PgEnt::retirer_domaine_plateforme`].
    pub async fn retirer_domaine_for_actor(
        &self,
        _etablissement_id: Uuid,
        _actor_user_id: Uuid,
        actor_role: &str,
        _domaine: &str,
    ) -> Result<bool, EntDbError> {
        exiger_admin(actor_role)?;
        Err(EntDbError::Forbidden)
    }

    // ---- Opérateur de la plateforme (B-02) --------------------------------
    //
    // Ces fonctions n'ont **aucune route HTTP** : elles sont appelées par l'outil
    // d'exploitation (`ent-api plateforme-domaine …`, accès direct à la base) et par
    // le labo. L'acteur est l'opérateur ScolIA, journalisé comme « plateforme ».

    async fn audit_plateforme(&self, etablissement_id: Uuid, action: &str, libelle: &str, payload: serde_json::Value) {
        if let Err(e) = self
            .enregistrer_journal_interne(domain_eleve::EnregistrerJournal {
                etablissement_id,
                eleve_id: None,
                domaine: "auth".into(),
                action: action.into(),
                libelle: libelle.into(),
                payload,
                visible_famille: false,
                actor_user_id: None,
            })
            .await
        {
            tracing::error!(action, error = %e, "échec d'écriture du journal d'audit (plateforme)");
        }
    }

    /// Déclare un établissement, ou change son slug et son nom (opérateur).
    /// Un slug déjà pris par un autre établissement → erreur de conflit.
    pub async fn declarer_etablissement_plateforme(
        &self,
        etablissement_id: Uuid,
        slug: &str,
        nom: &str,
    ) -> Result<EtablissementInfo, EntDbError> {
        let slug = slug.trim().to_ascii_lowercase();
        let nom = nom.trim();
        if etablissement_id.is_nil()
            || !slug_valide(&slug)
            || nom.is_empty()
            || nom.chars().count() > 200
        {
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
        self.audit_plateforme(
            id,
            "etablissement_declare_plateforme",
            "Établissement déclaré par la plateforme",
            serde_json::json!({ "acteur": "plateforme", "slug": slug }),
        )
        .await;
        Ok(EtablissementInfo {
            id,
            slug,
            nom,
            actif,
        })
    }

    /// Rattache un domaine à un établissement **déclaré** (opérateur). Refuse
    /// une valeur invalide ou un hôte réservé de la plateforme
    /// (`ValeurInvalide`), un établissement non déclaré (`Forbidden`) et un
    /// domaine déjà servi par un autre établissement (`Conflit`, jamais de
    /// réattribution). Idempotent pour le même établissement.
    pub async fn ajouter_domaine_plateforme(
        &self,
        etablissement_id: Uuid,
        domaine: &str,
    ) -> Result<String, EntDbError> {
        let domaine =
            normaliser_domaine(domaine).ok_or(EntDbError::Domain(DomainError::ValeurInvalide))?;
        if hote_reserve_plateforme(&domaine, rp_id_env().as_deref()) {
            return Err(EntDbError::Domain(DomainError::ValeurInvalide));
        }
        if self.etablissement_info(etablissement_id).await?.is_none() {
            return Err(EntDbError::Forbidden);
        }
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
        self.audit_plateforme(
            etablissement_id,
            "etablissement_domaine_ajoute",
            "Domaine rattaché à l'établissement par la plateforme",
            serde_json::json!({ "acteur": "plateforme", "domaine": domaine }),
        )
        .await;
        Ok(domaine)
    }

    /// Retire un domaine d'un établissement (opérateur). `false` si le domaine
    /// n'était pas servi par cet établissement.
    pub async fn retirer_domaine_plateforme(
        &self,
        etablissement_id: Uuid,
        domaine: &str,
    ) -> Result<bool, EntDbError> {
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
            self.audit_plateforme(
                etablissement_id,
                "etablissement_domaine_retire",
                "Domaine retiré de l'établissement par la plateforme",
                serde_json::json!({ "acteur": "plateforme", "domaine": domaine }),
            )
            .await;
        }
        Ok(n > 0)
    }

    /// Tous les domaines servis, par établissement (opérateur, revue de l'existant).
    pub async fn lister_domaines_plateforme(&self) -> Result<Vec<(String, Uuid)>, EntDbError> {
        let rows: Vec<(String, Uuid)> = sqlx::query_as(
            "SELECT domaine, etablissement_id FROM ent_etablissement_domaine ORDER BY domaine",
        )
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows)
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
    fn hotes_reserves_de_la_plateforme() {
        for h in ["scolia.fr", "app.scolia.fr", "www.scolia.fr", "api.scolia.fr", "fichiers.scolia.fr"] {
            assert!(hote_reserve_plateforme(h, None), "{h}");
        }
        assert!(hote_reserve_plateforme("ent.exemple.org", Some("ent.exemple.org")));
        assert!(hote_reserve_plateforme("app.ent.exemple.org", Some("ENT.exemple.org")));
        for h in ["lpnb.scolia.fr", "0750001a.scolia.fr", "app.lpnb.scolia.fr", "appscolia.fr", "xapp.scolia.fr"] {
            assert!(!hote_reserve_plateforme(h, None), "{h}");
        }
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
