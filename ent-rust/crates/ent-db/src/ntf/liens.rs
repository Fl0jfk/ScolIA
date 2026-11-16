//! Socle 8, lot N2 — liens nominatifs (`08_NOTIFICATIONS.md` §9.1, §19 `ent_ntf_lien`, Q-N12).
//!
//! Un lien nominatif permet à une personne **sans compte** d'ouvrir le contenu d'un avis (e-mail d'enveloppe
//! et lien). Le jeton (244 bits d'aléa : deux UUID v4) n'est **jamais stocké** : seule son empreinte SHA-256
//! l'est. Il expire, s'ouvre un nombre limité de fois, se révoque, et ne vaut que pour **un** destinataire.
//! La compétition entre deux ouvertures est tranchée par la base (une seule mise à jour conditionnelle).
//!
//! Hors périmètre N2 (lot N1b / routes) : la page d'attente en GET et la preuve en POST ; ici le service
//! est utilisable par la route une fois posée. L'établissement est fourni par l'appelant (il figure dans
//! l'adresse du lien), la table étant sous RLS.

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgConnection;
use uuid::Uuid;

/// Préfixe lisible du jeton (aide au tri des secrets ; ne porte aucune information).
const PREFIXE: &str = "ntf_";

fn empreinte(jeton: &str) -> String {
    let mut h = Sha256::new();
    h.update(jeton.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Un lien créé : le jeton en clair n'existe que dans cette valeur de retour.
#[derive(Debug, Clone)]
pub struct LienCree {
    pub id: Uuid,
    pub jeton: String,
    pub expire_le: DateTime<Utc>,
}

pub async fn creer_lien(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    destinataire_id: Uuid,
    maintenant: DateTime<Utc>,
    validite: Duration,
    ouvertures_max: i16,
    code_requis: bool,
) -> Result<LienCree, sqlx::Error> {
    let jeton = format!("{PREFIXE}{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let id = Uuid::new_v4();
    let expire_le = maintenant + validite;
    sqlx::query(
        "INSERT INTO ent_ntf_lien (id, etablissement_id, destinataire_id, jeton_hash, code_requis, expire_le, ouvertures_max)
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(id)
    .bind(etablissement_id)
    .bind(destinataire_id)
    .bind(empreinte(&jeton))
    .bind(code_requis)
    .bind(expire_le)
    .bind(ouvertures_max)
    .execute(&mut *conn)
    .await?;
    Ok(LienCree { id, jeton, expire_le })
}

/// Ce que l'ouverture d'un lien donne : le destinataire et l'avis, jamais l'adresse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LienOuvert {
    pub destinataire_id: Uuid,
    pub avis_id: Uuid,
    pub code_requis: bool,
}

/// Ouvre un lien : refuse un lien inconnu, expiré, révoqué ou épuisé (même réponse pour tous : `None`).
pub async fn ouvrir_lien(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    jeton: &str,
    maintenant: DateTime<Utc>,
) -> Result<Option<LienOuvert>, sqlx::Error> {
    if !jeton.starts_with(PREFIXE) || jeton.len() != PREFIXE.len() + 64 {
        return Ok(None);
    }
    let r: Option<(Uuid, Uuid, bool)> = sqlx::query_as(
        "UPDATE ent_ntf_lien l SET ouvertures = l.ouvertures + 1
           FROM ent_ntf_destinataire d
          WHERE l.etablissement_id = $1 AND l.jeton_hash = $2
            AND l.revoque_le IS NULL AND l.expire_le > $3 AND l.ouvertures < l.ouvertures_max
            AND d.etablissement_id = l.etablissement_id AND d.id = l.destinataire_id AND d.retenu
          RETURNING d.id, d.avis_id, l.code_requis",
    )
    .bind(etablissement_id)
    .bind(empreinte(jeton))
    .bind(maintenant)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(r.map(|(destinataire_id, avis_id, code_requis)| LienOuvert { destinataire_id, avis_id, code_requis }))
}

/// Révoque tous les liens d'un destinataire (lien suspendu, décès, erreur de destinataire).
pub async fn revoquer_liens(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    destinataire_id: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE ent_ntf_lien SET revoque_le = $3
          WHERE etablissement_id = $1 AND destinataire_id = $2 AND revoque_le IS NULL",
    )
    .bind(etablissement_id)
    .bind(destinataire_id)
    .bind(maintenant)
    .execute(&mut *conn)
    .await?;
    Ok(r.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_empreinte_est_stable_et_ne_contient_pas_le_jeton() {
        let j = format!("{PREFIXE}{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let e = empreinte(&j);
        assert_eq!(e.len(), 64);
        assert!(e.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(e, empreinte(&j));
        assert!(!e.contains(&j[PREFIXE.len()..PREFIXE.len() + 8]));
        assert_ne!(e, empreinte(&format!("{j}x")));
    }
}
