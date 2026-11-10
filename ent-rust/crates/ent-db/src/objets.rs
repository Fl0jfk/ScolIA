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

impl PgEnt {
    /// Écrit un objet immuable (empreinte vérifiée) puis l'enregistre en base.
    /// Toute erreur — stockage ou base — remonte ; il n'y a pas de mode « au mieux ».
    pub(crate) async fn stocker_objet(
        &self,
        etablissement_id: Uuid,
        key: &str,
        mime: Option<&str>,
        bytes: &[u8],
    ) -> Result<ObjectInfo, EntDbError> {
        let info = self.objects().put(key, bytes)?;
        enregistrer_objet(self.pool(), etablissement_id, &info, mime).await?;
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
        .fetch_optional(self.pool())
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
            Some(a) => self.objects().get_attendu(key, &a)?,
            None => self.objects().get(key)?,
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
        if !self.objects().existe(key)? {
            self.stocker_objet(etablissement_id, key, mime, bytes)
                .await?;
        }
        Ok(())
    }
}
