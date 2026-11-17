//! Socle 2 (l'élève), lot E7 — scellement des versions d'identité **sensibles** non courantes
//! (§10.4 : « versions sensibles non courantes → oui »).
//!
//! Une version sensible (prénom d'usage, mention de sexe, adoption, protection…) qui n'est plus la
//! courante garde sa trace — date d'effet, nature, motif codé, source, auteur, pièce — mais ses
//! **valeurs intimes** sont chiffrées (AES-256-GCM, `champs_chiffres`, une ligne par champ dans
//! `ent_eleve_identite_scellee`) et les colonnes en clair de la version sont remplacées par le
//! marqueur `[scellé]`. Seul ce service, qui détient la clé, scelle et descelle ; la base n'autorise
//! cette seule opération sur une version sensible non courante (déclencheur de la migration 298).
//!
//! * Le **descellement** est transparent pour le code de l'identité (corriger une version ancienne,
//!   attestation d'une période passée) : les lecteurs de versions rendent les valeurs ; l'accès
//!   *par les droits* reste celui de G5 / G6 (une version sensible n'est montrée qu'avec G6).
//! * Le scellement suit chaque écriture de version (même transaction) et la tâche quotidienne
//!   (une version annoncée devenue effective rend l'ancienne non courante).
//! * Sans clé (hors labo/test), le scellement échoue : **jamais** de valeur écrite en clair faute de
//!   clé, jamais de version scellée à moitié (transaction unique).

use std::collections::HashMap;

use domain_eleve::{decoder_prenoms, encoder_prenoms, ChampScelle, IdentiteVersion, MARQUEUR_SCELLE};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

const TABLE: &str = "ent_eleve_identite_scellee";
const COLONNE: &str = "valeur_scellee";

fn coffre() -> ChampsSensibles {
    ChampsSensibles::depuis_env()
}

/// Valeurs descellées, par version et par champ.
async fn valeurs(
    conn: &mut PgConnection,
    etab: Uuid,
    versions: &[Uuid],
) -> Result<HashMap<(Uuid, &'static str), String>, EntDbError> {
    let lignes: Vec<(Uuid, Uuid, String, String)> = sqlx::query_as(
        "SELECT id, version_id, champ, valeur_scellee FROM ent_eleve_identite_scellee
          WHERE etablissement_id = $1 AND version_id = ANY($2)",
    )
    .bind(etab)
    .bind(versions)
    .fetch_all(&mut *conn)
    .await?;
    let c = coffre();
    let mut out = HashMap::new();
    for (id, version, champ, stocke) in lignes {
        let Some(champ) = ChampScelle::parse(&champ) else { continue };
        out.insert((version, champ.code()), c.ouvrir(TABLE, COLONNE, id, &stocke)?);
    }
    Ok(out)
}

fn appliquer(v: &mut IdentiteVersion, vals: &HashMap<(Uuid, &'static str), String>) {
    let get = |c: ChampScelle| vals.get(&(v.id, c.code())).cloned();
    if let Some(x) = get(ChampScelle::NomNaissance) {
        v.nom_naissance = x;
    }
    v.nom_usage = get(ChampScelle::NomUsage);
    if let Some(x) = get(ChampScelle::Prenoms) {
        v.prenoms = decoder_prenoms(&x);
    }
    if let Some(x) = get(ChampScelle::RangPrenomUsuel).and_then(|x| x.parse::<i16>().ok()) {
        v.rang_prenom_usuel = x;
    }
    v.prenom_usage = get(ChampScelle::PrenomUsage);
    v.sexe = get(ChampScelle::Sexe);
    v.nom_ecriture_origine = get(ChampScelle::NomEcritureOrigine);
    v.prenoms_ecriture_origine = get(ChampScelle::PrenomsEcritureOrigine);
}

/// Rend leurs valeurs aux versions scellées de la liste (les autres ne sont pas touchées). Une
/// panne de descellement (clé absente ou altérée) est une erreur d'infrastructure, jamais « absent ».
pub(crate) async fn desceller_versions(
    conn: &mut PgConnection,
    etab: Uuid,
    versions: &mut [IdentiteVersion],
) -> Result<(), sqlx::Error> {
    let scellees: Vec<Uuid> =
        versions.iter().filter(|v| v.nom_naissance == MARQUEUR_SCELLE).map(|v| v.id).collect();
    if scellees.is_empty() {
        return Ok(());
    }
    let vals = valeurs(conn, etab, &scellees).await.map_err(|e| {
        tracing::error!(error = %e, "descellement d'une version d'identité impossible");
        sqlx::Error::Decode("version d'identité scellée illisible".into())
    })?;
    for v in versions.iter_mut().filter(|v| scellees.contains(&v.id)) {
        appliquer(v, &vals);
    }
    Ok(())
}

/// Valeurs descellées d'une version (nom de naissance, prénoms, rang, prénom d'usage, nom d'usage)
/// quand une requête a lu les colonnes en clair : si `nom_naissance` est le marqueur, rend les
/// vraies valeurs. Pour les lecteurs qui ne passent pas par `PgIdentiteRepo` (attestations).
pub(crate) async fn desceller_identite_courte(
    conn: &mut PgConnection,
    etab: Uuid,
    version: Uuid,
) -> Result<HashMap<&'static str, String>, EntDbError> {
    let vals = valeurs(conn, etab, &[version]).await?;
    Ok(vals.into_iter().map(|((_, champ), v)| (champ, v)).collect())
}

#[derive(Debug, sqlx::FromRow)]
struct ASceller {
    id: Uuid,
    nom_naissance: String,
    nom_usage: Option<String>,
    prenoms: Vec<String>,
    rang_prenom_usuel: i16,
    prenom_usage: Option<String>,
    sexe: Option<String>,
    nom_ecriture_origine: Option<String>,
    prenoms_ecriture_origine: Option<String>,
}

impl PgEnt {
    /// Scelle, dans la transaction de l'appelant, les versions sensibles **non courantes** de
    /// l'élève (remplacées, ou dépassées par une version plus récente déjà effective). Idempotent ;
    /// renvoie le nombre de versions scellées.
    pub(crate) async fn sceller_versions_sensibles_tx(
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        eleve: Uuid,
    ) -> Result<u64, EntDbError> {
        let a_sceller: Vec<ASceller> = sqlx::query_as(
            "SELECT id, nom_naissance, nom_usage, prenoms, rang_prenom_usuel, prenom_usage, sexe,
                    nom_ecriture_origine, prenoms_ecriture_origine
               FROM ent_eleve_identite
              WHERE etablissement_id = $1 AND eleve_id = $2 AND sensible AND nom_naissance <> $3
                AND (remplacee_par IS NOT NULL
                     OR (effet_du <= ent_jour_etablissement($1)
                         AND ent_eleve_identite_courante($1, $2, ent_jour_etablissement($1)) IS DISTINCT FROM id))
              ORDER BY enregistre_le, id
              FOR UPDATE",
        )
        .bind(etab)
        .bind(eleve)
        .bind(MARQUEUR_SCELLE)
        .fetch_all(&mut **tx)
        .await?;
        let c = coffre();
        let mut n = 0u64;
        for v in a_sceller {
            let mut champs: Vec<(ChampScelle, String)> = vec![
                (ChampScelle::NomNaissance, v.nom_naissance.clone()),
                (ChampScelle::Prenoms, encoder_prenoms(&v.prenoms)),
                (ChampScelle::RangPrenomUsuel, v.rang_prenom_usuel.to_string()),
            ];
            for (champ, valeur) in [
                (ChampScelle::NomUsage, v.nom_usage.as_ref()),
                (ChampScelle::PrenomUsage, v.prenom_usage.as_ref()),
                (ChampScelle::Sexe, v.sexe.as_ref()),
                (ChampScelle::NomEcritureOrigine, v.nom_ecriture_origine.as_ref()),
                (ChampScelle::PrenomsEcritureOrigine, v.prenoms_ecriture_origine.as_ref()),
            ] {
                if let Some(x) = valeur {
                    champs.push((champ, x.clone()));
                }
            }
            for (champ, valeur) in champs {
                // Un champ vide n'a rien à cacher et `sceller` ne chiffre pas le vide.
                if valeur.is_empty() {
                    continue;
                }
                let id = Uuid::new_v4();
                let stocke = c.sceller(TABLE, COLONNE, id, &valeur)?;
                sqlx::query(
                    "INSERT INTO ent_eleve_identite_scellee (id, etablissement_id, eleve_id, version_id, champ, valeur_scellee)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(id)
                .bind(etab)
                .bind(eleve)
                .bind(v.id)
                .bind(champ.code())
                .bind(stocke)
                .execute(&mut **tx)
                .await?;
            }
            // Le drapeau n'autorise l'opération que le temps de cet UPDATE (local à la transaction).
            sqlx::query("SELECT set_config('ent.identite_scellement', 'on', true)").execute(&mut **tx).await?;
            let r = sqlx::query(
                "UPDATE ent_eleve_identite
                    SET nom_naissance = $3, nom_usage = NULL, prenoms = ARRAY[$3]::TEXT[], rang_prenom_usuel = 1,
                        prenom_usage = NULL, sexe = NULL, nom_ecriture_origine = NULL, prenoms_ecriture_origine = NULL
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(v.id)
            .bind(MARQUEUR_SCELLE)
            .execute(&mut **tx)
            .await;
            sqlx::query("SELECT set_config('ent.identite_scellement', 'off', true)").execute(&mut **tx).await?;
            r?;
            n += 1;
        }
        Ok(n)
    }

    /// Tâche quotidienne : scelle les versions sensibles devenues non courantes de tous les
    /// élèves (reprise des versions antérieures au lot E7, versions annoncées devenues effectives).
    /// Idempotente ; renvoie le nombre de versions scellées.
    pub async fn sceller_identites_sensibles_anciennes(&self) -> Result<i64, EntDbError> {
        let mut total = 0i64;
        for etab in crate::tenant_tx::etablissements_systeme(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::TacheNocturneEleves,
        )
        .await?
        {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            let eleves: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT DISTINCT eleve_id FROM ent_eleve_identite
                  WHERE etablissement_id = $1 AND sensible AND nom_naissance <> $2
                    AND (remplacee_par IS NOT NULL OR effet_du <= ent_jour_etablissement($1))",
            )
            .bind(etab)
            .bind(MARQUEUR_SCELLE)
            .fetch_all(&mut *tx)
            .await?;
            tx.commit().await?;
            for (eleve,) in eleves {
                let mut tx = TenantTx::begin(self.pool(), etab).await?;
                // Verrou de la fiche : sérialise avec les commandes d'identité sur cet élève.
                sqlx::query("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
                    .bind(etab)
                    .bind(eleve)
                    .fetch_optional(&mut *tx)
                    .await?;
                total += i64::try_from(Self::sceller_versions_sensibles_tx(&mut tx, etab, eleve).await?).unwrap_or(0);
                tx.commit().await?;
            }
        }
        Ok(total)
    }
}
