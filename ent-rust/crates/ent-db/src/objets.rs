//! Registre des objets stockés hors base (`ent_objet`, migration 500 ; PRA E5, F-013).
//!
//! Tout fichier écrit dans le magasin d'objets laisse une ligne : clé opaque, taille et
//! empreinte SHA-256 **calculées avant l'envoi**, type, date. À la lecture, le contenu
//! relu est comparé à ces valeurs (`lire_objet`) : une corruption ou un remplacement de
//! l'objet côté fournisseur est détecté et remonte en erreur, jamais en silence.
//!
//! Les objets historiques (écrits avant la migration 500) n'ont pas de ligne : ils sont
//! lus avec la seule empreinte conservée par le fournisseur, et comptés dans
//! `lectures_sans_empreinte` jusqu'à leur reprise.

use ent_object::{ObjectAttendu, ObjectInfo};
use sqlx::{PgExecutor, Row};
use uuid::Uuid;

use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Clé d'objet d'une pièce historique : `{etab}/{espace}/{id}.{ext}`. Seuls l'établissement,
/// un mot fixe, l'UUID de la pièce et l'extension (assainie) y figurent — jamais le nom du
/// fichier (E14). Même dérivation pour le lien court d'une pièce historique et pour la reprise :
/// un objet déjà recopié est donc retrouvé, pas dupliqué.
pub(crate) fn cle_reprise(etablissement_id: Uuid, espace: &str, id: Uuid, nom: &str) -> String {
    let ext = nom.rsplit_once('.').map(|(_, e)| e).unwrap_or("bin");
    ent_object::ObjectStore::key_for(etablissement_id, espace, id, ext)
}

/// Enregistre (idempotent) l'empreinte et la taille d'un objet déjà écrit et vérifié.
/// À appeler dans la **même transaction** que la ligne métier qui référence la clé.
pub(crate) async fn enregistrer_objet<'e, E: PgExecutor<'e>>(
    ex: E,
    etablissement_id: Uuid,
    info: &ObjectInfo,
    mime: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_objet (id, etablissement_id, cle_objet, taille, sha256, mime)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (etablissement_id, cle_objet) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etablissement_id)
    .bind(&info.key)
    .bind(info.taille as i64)
    .bind(&info.sha256)
    .bind(mime)
    .execute(ex)
    .await?;
    Ok(())
}

/// Exécute un appel de stockage (client S3 **synchrone**, 60 s × 3 tentatives) sur le pool
/// des tâches bloquantes de Tokio : un S3 lent ne fige plus les fils qui servent l'API (R-06).
async fn hors_runtime<T, F>(store: &ent_object::ObjectStore, f: F) -> Result<T, EntDbError>
where
    T: Send + 'static,
    F: FnOnce(ent_object::ObjectStore) -> Result<T, ent_object::ObjectStoreError> + Send + 'static,
{
    let store = store.clone();
    tokio::task::spawn_blocking(move || f(store))
        .await
        .map_err(|_| {
            EntDbError::Object(ent_object::ObjectStoreError::Backend(
                "tâche de stockage interrompue".into(),
            ))
        })?
        .map_err(EntDbError::from)
}

impl PgEnt {
    /// Écriture synchrone du magasin, hors des fils de l'API (R-06).
    pub(crate) async fn objet_put(&self, key: &str, bytes: &[u8]) -> Result<ObjectInfo, EntDbError> {
        let (key, bytes) = (key.to_string(), bytes.to_vec());
        hors_runtime(self.objects(), move |s| s.put(&key, &bytes)).await
    }

    pub(crate) async fn objet_get(&self, key: &str) -> Result<Vec<u8>, EntDbError> {
        let key = key.to_string();
        hors_runtime(self.objects(), move |s| s.get(&key)).await
    }

    pub(crate) async fn objet_get_attendu(
        &self,
        key: &str,
        attendu: ObjectAttendu,
    ) -> Result<Vec<u8>, EntDbError> {
        let key = key.to_string();
        hors_runtime(self.objects(), move |s| s.get_attendu(&key, &attendu)).await
    }

    pub(crate) async fn objet_existe(&self, key: &str) -> Result<bool, EntDbError> {
        let key = key.to_string();
        hors_runtime(self.objects(), move |s| s.existe(&key)).await
    }

    /// Écrit un objet immuable (empreinte vérifiée) puis l'enregistre en base.
    /// Toute erreur — stockage ou base — remonte ; il n'y a pas de mode « au mieux ».
    pub(crate) async fn stocker_objet(
        &self,
        etablissement_id: Uuid,
        key: &str,
        mime: Option<&str>,
        bytes: &[u8],
    ) -> Result<ObjectInfo, EntDbError> {
        let info = self.objet_put(key, bytes).await?;
        let mut tx = crate::tenant_tx::TenantTx::begin(self.pool(), etablissement_id).await?;
        enregistrer_objet(&mut *tx, etablissement_id, &info, mime).await?;
        tx.commit().await?;
        Ok(info)
    }

    /// Empreinte et taille enregistrées pour une clé (`None` : objet historique).
    pub(crate) async fn objet_attendu(
        &self,
        etablissement_id: Uuid,
        key: &str,
    ) -> Result<Option<ObjectAttendu>, EntDbError> {
        let row = sqlx::query(
            "SELECT taille, sha256 FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2",
        )
        .bind(etablissement_id)
        .bind(key)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await?;
        Ok(row.map(|r| ObjectAttendu {
            taille: r.get::<i64, _>("taille").max(0) as u64,
            sha256: r.get("sha256"),
        }))
    }

    /// Lit un objet en contrôlant son empreinte et sa taille contre la base.
    pub async fn lire_objet(
        &self,
        etablissement_id: Uuid,
        key: &str,
    ) -> Result<Vec<u8>, EntDbError> {
        Ok(match self.objet_attendu(etablissement_id, key).await? {
            Some(a) => self.objet_get_attendu(key, a).await?,
            None => self.objet_get(key).await?,
        })
    }

    /// Lit un objet et, si la ligne métier porte sa propre empreinte, la compare aussi : deux
    /// références indépendantes (`ent_objet` et la pièce) doivent s'accorder avec le fournisseur.
    pub(crate) async fn lire_objet_empreinte(
        &self,
        etablissement_id: Uuid,
        key: &str,
        sha256_ligne: Option<&str>,
    ) -> Result<Vec<u8>, EntDbError> {
        let octets = self.lire_objet(etablissement_id, key).await?;
        if let Some(attendu) = sha256_ligne {
            if ent_object::sha256_hex(&octets) != attendu {
                tracing::error!("pièce : empreinte du magasin différente de celle de la ligne");
                return Err(EntDbError::Object(ent_object::ObjectStoreError::Integrity));
            }
        }
        Ok(octets)
    }

    /// Recopie dans le magasin (et le registre) un contenu historique stocké en base.
    /// Idempotent : un objet déjà présent n'est pas réécrit.
    pub(crate) async fn stocker_objet_si_absent(
        &self,
        etablissement_id: Uuid,
        key: &str,
        mime: Option<&str>,
        bytes: &[u8],
    ) -> Result<(), EntDbError> {
        if !self.objet_existe(key).await? {
            self.stocker_objet(etablissement_id, key, mime, bytes)
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests_non_bloquant {
    use super::*;
    use ent_object::provider::{ObjectHead, StorageProvider, StoredObject};
    use ent_object::{ObjectStore, ObjectStoreError};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// Fournisseur synchrone très lent (comme un S3 qui répond mal).
    #[derive(Default)]
    struct Lent(ent_object::provider::MemoryProvider);
    impl StorageProvider for Lent {
        fn nom(&self) -> &'static str {
            "lent"
        }
        fn uri(&self, key: &str) -> String {
            format!("lent://{key}")
        }
        fn put(&self, key: &str, bytes: &[u8], sha: &str) -> Result<(), ObjectStoreError> {
            std::thread::sleep(Duration::from_millis(600));
            self.0.put(key, bytes, sha)
        }
        fn get(&self, key: &str) -> Result<StoredObject, ObjectStoreError> {
            self.0.get(key)
        }
        fn head(&self, key: &str) -> Result<Option<ObjectHead>, ObjectStoreError> {
            self.0.head(key)
        }
    }

    /// Audit R-06 : un stockage lent ne fige pas l'exécuteur. Runtime à UN seul fil : si l'appel
    /// synchrone s'exécutait sur lui, aucune autre tâche ne progresserait pendant 600 ms.
    #[tokio::test(flavor = "current_thread")]
    async fn un_stockage_lent_ne_bloque_pas_les_autres_taches() {
        let store = ObjectStore::with_provider(Arc::new(Lent::default()));
        let ticker = tokio::spawn(async {
            let debut = Instant::now();
            let mut ticks = 0;
            while debut.elapsed() < Duration::from_millis(400) {
                tokio::time::sleep(Duration::from_millis(20)).await;
                ticks += 1;
            }
            ticks
        });
        let info = hors_runtime(&store, |s| s.put("etab/test/cle.bin", b"contenu"))
            .await
            .expect("écriture");
        assert_eq!(info.taille, 7);
        let ticks = ticker.await.unwrap();
        assert!(ticks >= 10, "l'exécuteur doit rester disponible pendant l'écriture : {ticks} tours");
    }
}
