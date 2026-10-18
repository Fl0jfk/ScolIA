//! Sauvegardes / PRA — **inventaire d'intégrité vérifiable**.
//!
//! Un dump Postgres (ou une restauration de snapshot) ne prouve rien par
//! lui-même : il faut pouvoir affirmer « la base restaurée contient exactement
//! les mêmes lignes qu'au moment du dump ». C'est le rôle de cet inventaire :
//!
//! - `inventaire_global()`       → toutes les tables métier, tous établissements ;
//! - `inventaire_etablissement()` → tables portant `etablissement_id`, filtrées ;
//! - `verifier_inventaire()`     → recalcule et compare (lignes **et** empreinte).
//!
//! Chaque table produit `count(*)` + `md5(string_agg(md5(to_jsonb(ligne))))`,
//! agrégé dans un ordre stable : le digest change si une seule valeur change.
//! L'`empreinte_schema` couvre le catalogue (`table.colonne:type:not null`) :
//! une migration non rejouée est détectée même si les données sont identiques.
//!
//! **Tables volatiles exclues** (voir [`TABLES_VOLATILES`]) : sessions, défis
//! passkey, états OIDC, verrous de connexion, jetons de réinitialisation. Ce
//! sont des états de sécurité éphémères : les restaurer depuis un dump ancien
//! rouvrirait des sessions révoquées ou relâcherait un verrou anti-bourrage.
//!
//! Coût : un `count(*)` + une agrégation par table. À lancer hors pointe pour
//! le périmètre global ; le périmètre établissement reste borné par tenant.

use super::service::{EntDbError, PgEnt};
use crate::migrate::MIGRATION_TAG;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

/// Version du format d'inventaire (à incrémenter si la sémantique change).
pub const INVENTAIRE_VERSION: u32 = 1;

/// Tables **volatiles** exclues de tout inventaire (état de sécurité éphémère).
pub const TABLES_VOLATILES: &[&str] = &[
    "ent_session",
    "ent_passkey_challenge",
    "ent_oidc_state",
    "ent_login_guard",
    "ent_password_reset",
];

/// Empreinte d'une table : nombre de lignes + digest du contenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventaireTable {
    pub nom: String,
    pub lignes: i64,
    pub digest: String,
}

/// Inventaire complet ou restreint à un établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventaire {
    /// `INVENTAIRE_VERSION` au moment de la prise.
    pub version: u32,
    /// Dernière migration connue du binaire (preuve de schéma).
    pub migration_tag: String,
    /// Digest du catalogue `public`.
    pub empreinte_schema: String,
    /// `Some(etab)` = inventaire d'établissement ; `None` = global.
    pub portee: Option<Uuid>,
    pub cree_le: DateTime<Utc>,
    pub tables: Vec<InventaireTable>,
}

impl Inventaire {
    /// Fiche d'une table, si l'inventaire la couvre.
    pub fn table(&self, nom: &str) -> Option<&InventaireTable> {
        self.tables.iter().find(|t| t.nom == nom)
    }

    /// Nombre de tables couvertes.
    pub fn nombre_tables(&self) -> usize {
        self.tables.len()
    }

    /// Total des lignes couvertes (indicateur de volumétrie du périmètre).
    pub fn lignes_total(&self) -> i64 {
        self.tables.iter().map(|t| t.lignes).sum()
    }
}

/// Écart constaté sur une table présente des deux côtés.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcartInventaire {
    pub nom: String,
    pub lignes_attendues: i64,
    pub lignes_obtenues: i64,
    pub digest_attendu: String,
    pub digest_obtenu: String,
}

/// Résultat d'une vérification d'intégrité (fidélité d'une restauration).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportIntegrite {
    pub conforme: bool,
    pub version_attendue: u32,
    pub portee: Option<Uuid>,
    pub tables_conformes: usize,
    pub ecarts: Vec<EcartInventaire>,
    /// Tables de l'inventaire de référence absentes de la base vérifiée.
    pub tables_absentes: Vec<String>,
    /// Tables présentes en base mais absentes de l'inventaire de référence.
    pub tables_en_plus: Vec<String>,
    pub empreinte_schema_attendue: String,
    pub empreinte_schema_obtenue: String,
    pub verifie_le: DateTime<Utc>,
}

impl RapportIntegrite {
    /// Une phrase lisible pour un journal d'exploitation.
    pub fn resume(&self) -> String {
        if self.conforme {
            format!(
                "intégrité conforme ({} tables, empreinte de schéma identique)",
                self.tables_conformes
            )
        } else {
            format!(
                "intégrité NON conforme : {} écart(s), {} table(s) absente(s), {} en trop",
                self.ecarts.len(),
                self.tables_absentes.len(),
                self.tables_en_plus.len()
            )
        }
    }
}

/// Un identifiant SQL (nom de table issue du catalogue) est toujours cité :
/// doublement des guillemets internes, jamais d'interpolation nue.
fn citer(identifiant: &str) -> String {
    format!("\"{}\"", identifiant.replace('"', "\"\""))
}

/// Tables couvertes par un périmètre.
///
/// - `portee = None` : toutes les tables de base de `public` hors volatiles ;
/// - `portee = Some(_)` : uniquement celles portant une colonne
///   `etablissement_id` (c'est le critère de cloisonnement multi-tenant).
async fn tables_du_perimetre(ent: &PgEnt, portee: Option<Uuid>) -> Result<Vec<String>, EntDbError> {
    let volatiles: Vec<String> = TABLES_VOLATILES.iter().map(|s| s.to_string()).collect();
    let lignes = if portee.is_some() {
        sqlx::query(
            r#"
            SELECT c.relname AS nom
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = 'public'
              AND c.relkind = 'r'
              AND NOT (c.relname = ANY($1))
              AND EXISTS (
                    SELECT 1 FROM pg_attribute a
                    WHERE a.attrelid = c.oid
                      AND a.attname = 'etablissement_id'
                      AND a.attnum > 0
                      AND NOT a.attisdropped
              )
            ORDER BY c.relname
            "#,
        )
        .bind(&volatiles)
        .fetch_all(ent.pool())
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT c.relname AS nom
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = 'public'
              AND c.relkind = 'r'
              AND NOT (c.relname = ANY($1))
            ORDER BY c.relname
            "#,
        )
        .bind(&volatiles)
        .fetch_all(ent.pool())
        .await?
    };
    Ok(lignes.iter().map(|r| r.get::<String, _>("nom")).collect())
}

/// Empreinte du catalogue : `table.colonne:type:not null` sur `public`.
async fn empreinte_schema(ent: &PgEnt) -> Result<String, EntDbError> {
    let volatiles: Vec<String> = TABLES_VOLATILES.iter().map(|s| s.to_string()).collect();
    let ligne = sqlx::query(
        r#"
        SELECT COALESCE(md5(string_agg(cle, '|' ORDER BY cle)), '') AS empreinte
        FROM (
            SELECT c.relname || '.' || a.attname || ':'
                   || format_type(a.atttypid, a.atttypmod) || ':'
                   || a.attnotnull::text AS cle
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            JOIN pg_attribute a ON a.attrelid = c.oid
            WHERE n.nspname = 'public'
              AND c.relkind = 'r'
              AND a.attnum > 0
              AND NOT a.attisdropped
              AND NOT (c.relname = ANY($1))
        ) s
        "#,
    )
    .bind(&volatiles)
    .fetch_one(ent.pool())
    .await?;
    Ok(ligne.get::<String, _>("empreinte"))
}

/// Digest d'une table : nombre de lignes + `md5` de l'agrégat ordonné des
/// digests de chaque ligne sérialisée en JSON canonique (`jsonb`).
async fn empreinte_table(
    ent: &PgEnt,
    table: &str,
    portee: Option<Uuid>,
) -> Result<InventaireTable, EntDbError> {
    let nom_sql = citer(table);
    let requete = match portee {
        Some(_) => format!(
            r#"
            SELECT count(*)::bigint AS lignes,
                   COALESCE(md5(string_agg(h, '|' ORDER BY h)), '') AS digest
            FROM (
                SELECT md5(to_jsonb(__pra_ligne)::text) AS h
                FROM public.{nom_sql} AS __pra_ligne
                WHERE __pra_ligne."etablissement_id" = $1
            ) s
            "#
        ),
        None => format!(
            r#"
            SELECT count(*)::bigint AS lignes,
                   COALESCE(md5(string_agg(h, '|' ORDER BY h)), '') AS digest
            FROM (
                SELECT md5(to_jsonb(__pra_ligne)::text) AS h
                FROM public.{nom_sql} AS __pra_ligne
            ) s
            "#
        ),
    };

    let ligne = match portee {
        Some(etab) => {
            sqlx::query(&requete)
                .bind(etab)
                .fetch_one(ent.pool())
                .await?
        }
        None => sqlx::query(&requete).fetch_one(ent.pool()).await?,
    };

    Ok(InventaireTable {
        nom: table.to_string(),
        lignes: ligne.get::<i64, _>("lignes"),
        digest: ligne.get::<String, _>("digest"),
    })
}

async fn inventaire(ent: &PgEnt, portee: Option<Uuid>) -> Result<Inventaire, EntDbError> {
    let empreinte = empreinte_schema(ent).await?;
    let mut tables = Vec::new();
    for table in tables_du_perimetre(ent, portee).await? {
        tables.push(empreinte_table(ent, &table, portee).await?);
    }
    Ok(Inventaire {
        version: INVENTAIRE_VERSION,
        migration_tag: MIGRATION_TAG.to_string(),
        empreinte_schema: empreinte,
        portee,
        cree_le: Utc::now(),
        tables,
    })
}

impl PgEnt {
    /// Inventaire **global** (toutes les tables métier, tous établissements).
    /// Surface opérateur (labo / script de sauvegarde), pas une route HTTP.
    pub async fn inventaire_global(&self) -> Result<Inventaire, EntDbError> {
        inventaire(self, None).await
    }

    /// Inventaire **d'établissement** : tables portant `etablissement_id`,
    /// filtrées sur ce tenant. L'empreinte de schéma reste globale (le schéma
    /// est commun) mais aucun compteur ne traverse les établissements.
    pub async fn inventaire_etablissement(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Inventaire, EntDbError> {
        inventaire(self, Some(etablissement_id)).await
    }

    /// Vérifie qu'une base **recalcule** exactement l'inventaire attendu.
    ///
    /// Utilisation PRA : inventaire avant dump → restauration → vérification.
    /// Un `conforme: true` prouve la fidélité ligne à ligne, pas seulement le
    /// nombre de tables.
    pub async fn verifier_inventaire(
        &self,
        attendu: &Inventaire,
    ) -> Result<RapportIntegrite, EntDbError> {
        let obtenu = inventaire(self, attendu.portee).await?;

        let mut ecarts = Vec::new();
        let mut tables_absentes = Vec::new();
        let mut tables_en_plus = Vec::new();
        let mut conformes = 0usize;

        for attendue in &attendu.tables {
            match obtenu.table(&attendue.nom) {
                None => tables_absentes.push(attendue.nom.clone()),
                Some(reelle) => {
                    if reelle.lignes == attendue.lignes && reelle.digest == attendue.digest {
                        conformes += 1;
                    } else {
                        ecarts.push(EcartInventaire {
                            nom: attendue.nom.clone(),
                            lignes_attendues: attendue.lignes,
                            lignes_obtenues: reelle.lignes,
                            digest_attendu: attendue.digest.clone(),
                            digest_obtenu: reelle.digest.clone(),
                        });
                    }
                }
            }
        }
        for reelle in &obtenu.tables {
            if attendu.table(&reelle.nom).is_none() {
                tables_en_plus.push(reelle.nom.clone());
            }
        }

        let conforme = ecarts.is_empty()
            && tables_absentes.is_empty()
            && tables_en_plus.is_empty()
            && obtenu.empreinte_schema == attendu.empreinte_schema;

        Ok(RapportIntegrite {
            conforme,
            version_attendue: attendu.version,
            portee: attendu.portee,
            tables_conformes: conformes,
            ecarts,
            tables_absentes,
            tables_en_plus,
            empreinte_schema_attendue: attendu.empreinte_schema.clone(),
            empreinte_schema_obtenue: obtenu.empreinte_schema,
            verifie_le: Utc::now(),
        })
    }

    /// Inventaire d'établissement **gardé par rôle** (Direction / Admin) : la
    /// portée est celle de la session, jamais un paramètre d'appel.
    pub async fn inventaire_etablissement_pour_acteur(
        &self,
        etablissement_id: Uuid,
        actor_role: &str,
    ) -> Result<Inventaire, EntDbError> {
        if !Self::role_can_sauvegarde(actor_role) {
            return Err(EntDbError::Forbidden);
        }
        self.inventaire_etablissement(etablissement_id).await
    }

    /// Vérification **gardée par rôle** d'un inventaire d'établissement.
    ///
    /// Refuse un inventaire dont la portée n'est pas l'établissement de la
    /// session : un manifeste d'un autre tenant ne peut pas servir de
    /// référence (fuite d'information par comparaison).
    pub async fn verifier_inventaire_etablissement_pour_acteur(
        &self,
        etablissement_id: Uuid,
        actor_role: &str,
        attendu: &Inventaire,
    ) -> Result<RapportIntegrite, EntDbError> {
        if !Self::role_can_sauvegarde(actor_role) {
            return Err(EntDbError::Forbidden);
        }
        if attendu.portee != Some(etablissement_id) {
            return Err(EntDbError::InventairePorteeEtrangere);
        }
        self.verifier_inventaire(attendu).await
    }
}
