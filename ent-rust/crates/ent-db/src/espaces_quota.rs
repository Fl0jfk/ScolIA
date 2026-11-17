//! Socle 32, lot ET0 — **quotas atomiques** des espaces (`32_BUREAUTIQUE.md` §15 point 4, R-ET20, R-ET21, ND32-01, ND32-15).
//!
//! * Le volume d'un espace est un **compteur verrouillé** (`ent_espace_occupation`, `SELECT … FOR UPDATE`) : octets
//!   confirmés (versions conservées, corbeille comprise : « le quota d'espace compte toutes les versions », CET-21) et
//!   octets réservés (envois annoncés, pas encore confirmés). Plus d'inventaire du stockage à chaque envoi (ND32-15).
//! * Un envoi **réserve** sa taille (celle qui sera signée dans l'URL d'envoi) dans la transaction de la demande ; la
//!   réservation est **confirmée** après la réconciliation de la taille et de l'empreinte réelles, ou **libérée**
//!   (expiration à 15 minutes, taille différente, refus, annulation : CET-18, CET-19).
//! * Tout passe par la transaction de l'appelant : réservation et ligne métier se valident ensemble ou pas du tout.
//!   Deux envois parallèles se sérialisent sur le verrou de la ligne d'occupation : jamais de dépassement (le test des
//!   vingt envois parallèles le prouve).
//! * La taille et le quota sont **décidés par le serveur** : ni le navigateur (ND32-01) ni une valeur déclarée ne comptent.

use crate::service::{EntDbError, RefusMetier};
use chrono::{DateTime, Duration, Utc};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::espaces::RESERVATION_MINUTES;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

/// Occupation lue sous verrou.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Occupation {
    pub confirmes: i64,
    pub reserves: i64,
    pub quota: i64,
}


/// Une réservation expirée libérée (l'objet éventuellement écrit sous sa clé est à effacer par l'appelant).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationExpiree {
    pub etablissement_id: Uuid,
    pub reservation_id: Uuid,
    pub espace_id: Uuid,
    pub cle_objet: String,
}

/// Crée (idempotent) la ligne d'occupation d'un espace.
pub(crate) async fn occupation_assurer(conn: &mut PgConnection, etab: Uuid, espace: Uuid) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_espace_occupation (etablissement_id, espace_id) VALUES ($1, $2)
         ON CONFLICT (etablissement_id, espace_id) DO NOTHING",
    )
    .bind(etab)
    .bind(espace)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Verrouille la ligne d'occupation (`FOR UPDATE`) et libère d'abord, dans la même transaction, les réservations
/// expirées de l'espace (un envoi coupé n'immobilise pas le quota, CET-18).
pub(crate) async fn occupation_verrouiller(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<Occupation, EntDbError> {
    let ligne = sqlx::query(
        "SELECT o.octets_confirmes, o.octets_reserves, e.quota_octets
           FROM ent_espace_occupation o
           JOIN ent_espace e ON e.etablissement_id = o.etablissement_id AND e.id = o.espace_id
          WHERE o.etablissement_id = $1 AND o.espace_id = $2
          FOR UPDATE OF o",
    )
    .bind(etab)
    .bind(espace)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some(ligne) = ligne else {
        return Err(EntDbError::Domain(domain_eleve::DomainError::NotFound));
    };
    let (confirmes, mut reserves, quota): (i64, i64, i64) = (ligne.get(0), ligne.get(1), ligne.get(2));
    let rendus: Option<i64> = sqlx::query_scalar(
        "WITH liberees AS (
            UPDATE ent_espace_reservation
               SET liberee_le = $3, liberee_motif = 'expiree'
             WHERE etablissement_id = $1 AND espace_id = $2
               AND confirmee_le IS NULL AND liberee_le IS NULL AND expire_le <= $3
         RETURNING octets)
         SELECT COALESCE(SUM(octets), 0)::bigint FROM liberees",
    )
    .bind(etab)
    .bind(espace)
    .bind(maintenant)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if let Some(r) = rendus.filter(|r| *r > 0) {
        reserves = (reserves - r).max(0);
        sqlx::query(
            "UPDATE ent_espace_occupation SET octets_reserves = $3, modifie_le = $4
              WHERE etablissement_id = $1 AND espace_id = $2",
        )
        .bind(etab)
        .bind(espace)
        .bind(reserves)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(Occupation { confirmes, reserves, quota })
}

/// Réserve `octets` : refuse si le quota de l'espace serait dépassé. À appeler dans la transaction de la demande
/// d'envoi, **avant** d'émettre l'URL signée.
pub(crate) async fn quota_reserver(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Uuid,
    octets: i64,
    cle_objet: &str,
    demandeur_user: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(Uuid, DateTime<Utc>), EntDbError> {
    if octets <= 0 {
        return Err(EntDbError::Refus(RefusMetier::Invalide, "La taille annoncée doit être positive."));
    }
    let occ = occupation_verrouiller(conn, etab, espace, maintenant).await?;
    if occ.confirmes + occ.reserves + octets > occ.quota {
        return Err(EntDbError::Refus(
            RefusMetier::Conflit,
            "Le quota de cet espace serait dépassé : libérez de la place (corbeille, anciennes versions) ou demandez une extension.",
        ));
    }
    let id = Uuid::new_v4();
    let expire = maintenant + Duration::minutes(RESERVATION_MINUTES);
    sqlx::query(
        "INSERT INTO ent_espace_reservation
            (id, etablissement_id, espace_id, octets, cle_objet, demandeur_user_id, expire_le, cree_le)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(etab)
    .bind(espace)
    .bind(octets)
    .bind(cle_objet)
    .bind(demandeur_user)
    .bind(expire)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_espace_occupation SET octets_reserves = octets_reserves + $3, modifie_le = $4
          WHERE etablissement_id = $1 AND espace_id = $2",
    )
    .bind(etab)
    .bind(espace)
    .bind(octets)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok((id, expire))
}

/// Une réservation ouverte (ni confirmée ni libérée), lue **sans verrou** : l'ordre des verrous est toujours
/// « occupation d'abord, réservation ensuite » (pas d'inversion, donc pas d'interblocage) ; la garde d'unicité est le
/// `UPDATE … WHERE confirmee_le IS NULL AND liberee_le IS NULL` de `quota_confirmer` et de `quota_liberer`.
#[derive(Debug, Clone)]
pub(crate) struct ReservationOuverte {
    pub espace_id: Uuid,
    pub octets: i64,
    pub cle_objet: String,
    pub demandeur_user_id: Uuid,
    pub expire_le: DateTime<Utc>,
}

pub(crate) async fn reservation_ouverte(
    conn: &mut PgConnection,
    etab: Uuid,
    reservation: Uuid,
) -> Result<Option<ReservationOuverte>, EntDbError> {
    let l = sqlx::query(
        "SELECT espace_id, octets, cle_objet, demandeur_user_id, expire_le
           FROM ent_espace_reservation
          WHERE etablissement_id = $1 AND id = $2 AND confirmee_le IS NULL AND liberee_le IS NULL",
    )
    .bind(etab)
    .bind(reservation)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(l.map(|l| ReservationOuverte {
        espace_id: l.get(0),
        octets: l.get(1),
        cle_objet: l.get(2),
        demandeur_user_id: l.get(3),
        expire_le: l.get(4),
    }))
}

/// Confirme une réservation : la taille **réelle** (égale à la taille signée, vérifiée par l'appelant) passe des
/// octets réservés aux octets confirmés. Verrouille l'occupation.
pub(crate) async fn quota_confirmer(
    conn: &mut PgConnection,
    etab: Uuid,
    reservation: Uuid,
    espace: Uuid,
    octets: i64,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    occupation_verrouiller(conn, etab, espace, maintenant).await?;
    let r = sqlx::query(
        "UPDATE ent_espace_reservation SET confirmee_le = $3
          WHERE etablissement_id = $1 AND id = $2 AND confirmee_le IS NULL AND liberee_le IS NULL",
    )
    .bind(etab)
    .bind(reservation)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() != 1 {
        return Err(EntDbError::Refus(RefusMetier::Conflit, "La réservation de cet envoi a expiré."));
    }
    sqlx::query(
        "UPDATE ent_espace_occupation
            SET octets_reserves = GREATEST(octets_reserves - $3, 0), octets_confirmes = octets_confirmes + $3, modifie_le = $4
          WHERE etablissement_id = $1 AND espace_id = $2",
    )
    .bind(etab)
    .bind(espace)
    .bind(octets)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Libère une réservation ouverte (idempotent : une réservation déjà confirmée ou libérée ne change rien).
pub(crate) async fn quota_liberer(
    conn: &mut PgConnection,
    etab: Uuid,
    reservation: Uuid,
    motif: &str,
    maintenant: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let Some(r) = reservation_ouverte(conn, etab, reservation).await? else {
        return Ok(false);
    };
    occupation_verrouiller(conn, etab, r.espace_id, maintenant).await?;
    // L'occupation vient peut-être de libérer cette réservation expirée : on vérifie qu'elle est encore ouverte.
    let l = sqlx::query(
        "UPDATE ent_espace_reservation SET liberee_le = $3, liberee_motif = $4
          WHERE etablissement_id = $1 AND id = $2 AND confirmee_le IS NULL AND liberee_le IS NULL",
    )
    .bind(etab)
    .bind(reservation)
    .bind(maintenant)
    .bind(motif)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if l.rows_affected() == 1 {
        sqlx::query(
            "UPDATE ent_espace_occupation SET octets_reserves = GREATEST(octets_reserves - $3, 0), modifie_le = $4
              WHERE etablissement_id = $1 AND espace_id = $2",
        )
        .bind(etab)
        .bind(r.espace_id)
        .bind(r.octets)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(l.rows_affected() == 1)
}

/// Rend des octets confirmés (purge d'une version, élagage). Verrouille l'occupation.
pub(crate) async fn quota_rendre(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Uuid,
    octets: i64,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    occupation_verrouiller(conn, etab, espace, maintenant).await?;
    sqlx::query(
        "UPDATE ent_espace_occupation SET octets_confirmes = GREATEST(octets_confirmes - $3, 0), modifie_le = $4
          WHERE etablissement_id = $1 AND espace_id = $2",
    )
    .bind(etab)
    .bind(espace)
    .bind(octets)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Réservations ouvertes dont l'échéance est passée, pour **tous** les espaces d'un établissement : elles sont
/// libérées (le compteur d'occupation suit) et rendues à l'appelant pour effacer l'éventuel objet écrit après coup.
pub(crate) async fn reservations_expirees_liberer(
    conn: &mut PgConnection,
    etab: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<Vec<ReservationExpiree>, EntDbError> {
    let espaces: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT espace_id FROM ent_espace_reservation
          WHERE etablissement_id = $1 AND confirmee_le IS NULL AND liberee_le IS NULL AND expire_le <= $2",
    )
    .bind(etab)
    .bind(maintenant)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut out = Vec::new();
    for espace in espaces {
        let avant: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, cle_objet FROM ent_espace_reservation
              WHERE etablissement_id = $1 AND espace_id = $2 AND confirmee_le IS NULL AND liberee_le IS NULL
                AND expire_le <= $3",
        )
        .bind(etab)
        .bind(espace)
        .bind(maintenant)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        // `occupation_verrouiller` libère les réservations expirées de l'espace et ajuste le compteur.
        occupation_verrouiller(conn, etab, espace, maintenant).await?;
        out.extend(avant.into_iter().map(|(reservation_id, cle_objet)| ReservationExpiree {
            etablissement_id: etab,
            reservation_id,
            espace_id: espace,
            cle_objet,
        }));
    }
    Ok(out)
}

/// **Réconciliation** (maintenance, restauration) : recalcule les compteurs d'un espace depuis les lignes de vérité
/// (versions non effacées, réservations ouvertes). Renvoie `(confirmés, réservés)`.
pub(crate) async fn occupation_reconcilier(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(i64, i64), EntDbError> {
    occupation_assurer(conn, etab, espace).await?;
    let verrou = sqlx::query(
        "SELECT 1 FROM ent_espace_occupation WHERE etablissement_id = $1 AND espace_id = $2 FOR UPDATE",
    )
    .bind(etab)
    .bind(espace)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let _ = verrou;
    let confirmes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(o.taille), 0)::bigint
           FROM ent_espace_version v
           JOIN ent_espace_element e ON e.etablissement_id = v.etablissement_id AND e.id = v.element_id
           JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
          WHERE v.etablissement_id = $1 AND e.espace_id = $2 AND o.supprime_le IS NULL AND v.elaguee_le IS NULL",
    )
    .bind(etab)
    .bind(espace)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let reserves: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(octets), 0)::bigint FROM ent_espace_reservation
          WHERE etablissement_id = $1 AND espace_id = $2 AND confirmee_le IS NULL AND liberee_le IS NULL",
    )
    .bind(etab)
    .bind(espace)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_espace_occupation SET octets_confirmes = $3, octets_reserves = $4, modifie_le = $5
          WHERE etablissement_id = $1 AND espace_id = $2",
    )
    .bind(etab)
    .bind(espace)
    .bind(confirmes)
    .bind(reserves)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok((confirmes, reserves))
}
