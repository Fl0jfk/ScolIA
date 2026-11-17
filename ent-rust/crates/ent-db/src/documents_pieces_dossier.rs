//! Socle 7 — commandes sur les pièces dont le **sujet est un dossier d'inscription** (lot I3 du socle 3).
//!
//! Le module d'inscriptions n'écrit jamais dans les tables d'un autre socle (garde
//! `tests/garde_inscriptions_ecritures.rs`) : il appelle ces commandes, qui sont au socle 7 et appliquent ses
//! règles (quarantaine, contrôle humain d'une pièce reçue seulement, décisions en ajout seul, objets partagés
//! jamais détruits). Rien ici ne décide d'un fait d'inscription : le socle 3 qualifie la pièce, le socle 7
//! la garde.

use chrono::NaiveDate;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::documents_pieces::decision;
use crate::ref_commun::db;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Retire une pièce déposée par erreur ou remplacée, tant qu'elle n'est pas contrôlée et qu'aucune pièce
/// d'inscription ne la référence plus.
pub(crate) async fn retirer_si_orphelin_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    doc: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_doc_piece SET etat = 'retiree', maj_le = now()
          WHERE etablissement_id = $1 AND id = $2 AND etat IN ('en_quarantaine', 'recue')
            AND NOT EXISTS (SELECT 1 FROM ent_inscription_piece q
                             WHERE q.etablissement_id = $1 AND q.piece_doc_id = $2)",
    )
    .bind(etab)
    .bind(doc)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Accepte une pièce **reçue** (sain, converti). `false` : elle n'était plus à contrôler (autre agent passé).
pub(crate) async fn accepter_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    doc: Uuid,
    valable_jusqu_au: Option<NaiveDate>,
    acteur: Uuid,
    commentaire: Option<&str>,
) -> Result<bool, EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_doc_piece SET etat = 'acceptee', valable_jusqu_au = $3, controle_par = $4,
                controle_le = now(), maj_le = now()
          WHERE etablissement_id = $1 AND id = $2 AND etat = 'recue'",
    )
    .bind(etab)
    .bind(doc)
    .bind(valable_jusqu_au)
    .bind(acteur)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() != 1 {
        return Ok(false);
    }
    decision(conn, etab, doc, None, "acceptee", None, commentaire, acteur).await?;
    Ok(true)
}

/// Refuse une pièce reçue avec un motif type (`illisible`, `incomplet`, `perime`, `ne_correspond_pas`,
/// `mauvais_document`, `autre`).
pub(crate) async fn refuser_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    doc: Uuid,
    motif: &str,
    commentaire: Option<&str>,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_doc_piece SET etat = 'refusee', refus_motif = $3, commentaire = $4, controle_par = $5,
                controle_le = now(), maj_le = now()
          WHERE etablissement_id = $1 AND id = $2 AND etat = 'recue'",
    )
    .bind(etab)
    .bind(doc)
    .bind(motif)
    .bind(commentaire)
    .bind(acteur)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() == 1 {
        decision(conn, etab, doc, None, "refusee", Some(motif), commentaire, acteur).await?;
    }
    Ok(())
}

/// **Rattache** une pièce acceptée à l'élève (validation de l'inscription) : une nouvelle pièce du socle 7 au
/// sujet de l'élève, mêmes objets (aucune copie d'octets), décision reprise. `None` : la pièce source n'est
/// pas acceptée.
pub(crate) async fn rattacher_a_eleve_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    doc: Uuid,
    eleve: Uuid,
    acteur: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    let nouveau = Uuid::new_v4();
    let r = sqlx::query(
        "INSERT INTO ent_doc_piece
            (id, etablissement_id, type_code, eleve_id, motif_autre, deposant_user_id, deposant_role, deposee_le, etat,
             cle_objet_original, sha256_original, taille_originale, mime_original, nb_fichiers_origine,
             cle_objet_servi, sha256_servi, nb_pages, valable_jusqu_au, controle_par, controle_le, analyse_tentatives)
         SELECT $3, etablissement_id, type_code, $4, motif_autre, deposant_user_id, deposant_role, deposee_le, 'acceptee',
                cle_objet_original, sha256_original, taille_originale, mime_original, nb_fichiers_origine,
                cle_objet_servi, sha256_servi, nb_pages, valable_jusqu_au, controle_par, controle_le, analyse_tentatives
           FROM ent_doc_piece WHERE etablissement_id = $1 AND id = $2 AND etat = 'acceptee'",
    )
    .bind(etab)
    .bind(doc)
    .bind(nouveau)
    .bind(eleve)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() != 1 {
        return Ok(None);
    }
    decision(
        conn,
        etab,
        nouveau,
        None,
        "acceptee",
        Some("rattachee"),
        Some("rattachée à l'élève à la validation de l'inscription"),
        acteur,
    )
    .await?;
    Ok(Some(nouveau))
}

impl PgEnt {
    /// Détruit au magasin les objets d'une pièce de dossier que **aucune autre pièce** ne référence (une pièce
    /// rattachée à un élève garde les siens), puis les marque supprimés au registre. Rend le nombre d'objets
    /// détruits et `false` si une destruction a échoué (réessayée à la passe suivante : la suppression est
    /// idempotente et la destruction précède le marquage).
    pub(crate) async fn detruire_objets_orphelins(
        &self,
        etab: Uuid,
        doc: Uuid,
        cles: &[String],
    ) -> Result<(usize, bool), EntDbError> {
        let mut detruits = 0;
        let mut ok = true;
        for cle in cles {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let (autres,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_doc_piece q
                  WHERE q.etablissement_id = $1 AND q.id <> $2
                    AND (q.cle_objet_original = $3 OR q.cle_objet_servi = $3)",
            )
            .bind(etab)
            .bind(doc)
            .bind(cle)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            if autres > 0 {
                continue;
            }
            if let Err(e) = self.objet_supprimer(cle).await {
                tracing::warn!(piece = %doc, erreur = %e, "purge d'une pièce de dossier : destruction impossible");
                ok = false;
                continue;
            }
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let r = sqlx::query(
                "UPDATE ent_objet SET supprime_le = now()
                  WHERE etablissement_id = $1 AND cle_objet = $2 AND supprime_le IS NULL",
            )
            .bind(etab)
            .bind(cle)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            detruits += usize::try_from(r.rows_affected()).unwrap_or(0);
            tx.commit().await.map_err(db)?;
        }
        Ok((detruits, ok))
    }
}
