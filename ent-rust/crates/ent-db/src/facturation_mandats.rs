//! Socle 24, lot F1 — commande du service unique : **mandat SEPA signé à l'inscription**.
//!
//! Avant F1, `inscriptions.rs` écrivait lui-même dans `ent_fac_sepa_mandat` et
//! `ent_foyer_iban_histo` (exception nominative de la garde du socle 3). La commande vit
//! désormais dans le service de facturation (R-F1) : le module d'inscriptions la **demande**, il
//! n'écrit plus dans les tables financières. Le mandat suit le foyer ; l'IBAN, le BIC et le
//! titulaire sont chiffrés au repos (D3) avec les mêmes primitives que le chemin historique.

use domain_eleve::MandatSepa;
use ent_events::{types, AggregateKind};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::facturation_service::{journaliser_tx, Acte, Chemin};
use crate::finances::{db, publier};
use crate::outbox::Causalite;
use crate::service::EntDbError;

const T_MANDAT: &str = "ent_fac_sepa_mandat";
const T_HISTO: &str = "ent_foyer_iban_histo";

/// Crée le mandat actif et l'historique d'IBAN du foyer, journalise le geste et publie
/// `facturation.mandat_modifie`, **dans la transaction de l'appelant** (la signature, le statut de
/// la demande et le mandat sont validés ensemble ou annulés ensemble).
///
/// `source` : provenance du geste (`inscription`), sans donnée personnelle.
pub(crate) async fn creer_mandat_signe_tx(
    conn: &mut PgConnection,
    champs: &ChampsSensibles,
    m: &MandatSepa,
    source: &str,
) -> Result<(), EntDbError> {
    let etab = m.etablissement_id;
    sqlx::query(
        "INSERT INTO ent_fac_sepa_mandat (id, etablissement_id, foyer_id, rum, iban, bic, titulaire,
                                          statut, signe_at, created_by_user_id, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,'actif',$8,$9, now())",
    )
    .bind(m.id)
    .bind(etab)
    .bind(m.foyer_id)
    .bind(&m.rum)
    .bind(champs.sceller(T_MANDAT, "iban", m.id, &m.iban)?)
    .bind(champs.sceller_opt(T_MANDAT, "bic", m.id, m.bic.as_deref())?)
    .bind(champs.sceller(T_MANDAT, "titulaire", m.id, &m.titulaire)?)
    .bind(m.signe_at)
    .bind(m.created_by_user_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let h = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_foyer_iban_histo (id, etablissement_id, foyer_id, iban, bic, titulaire, mandat_id,
                                           created_by_user_id, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())",
    )
    .bind(h)
    .bind(etab)
    .bind(m.foyer_id)
    .bind(champs.sceller(T_HISTO, "iban", h, &m.iban)?)
    .bind(champs.sceller_opt(T_HISTO, "bic", h, m.bic.as_deref())?)
    .bind(champs.sceller(T_HISTO, "titulaire", h, &m.titulaire)?)
    .bind(m.id)
    .bind(m.created_by_user_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journaliser_tx(
        conn,
        &Acte::new(
            etab,
            m.created_by_user_id,
            "famille",
            Chemin::Famille,
            "mandat.signer",
            "mandat",
        )
        .objet(m.id)
        .foyer(m.foyer_id)
        .detail(&format!("mandat signé par la famille ({source})")),
    )
    .await?;
    publier(
        conn,
        etab,
        types::FAC_SEPA_MANDAT,
        AggregateKind::Facture,
        m.id,
        None,
        json!({ "foyer_id": m.foyer_id, "mandat_id": m.id, "source": source }),
        m.created_by_user_id,
        Causalite::default(),
    )
    .await?;
    Ok(())
}
