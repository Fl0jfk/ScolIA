//! Socle 8, lot N1 — planificateur et expédition des avis échus.
//!
//! Un avis échu (délai de confirmation écoulé) est remis par établissement, en `TenantTx` : relecture du lien
//! de chaque destinataire (R-N1), rendu du gabarit français, remise dans l'ENT (`ent_notification`) puis
//! transmission des canaux externes à l'**ancien moteur** (façade : préférences, heures calmes et plafonds y
//! restent jusqu'aux lots N3 et N5–N7). Rejouable : la clé d'idempotence de `ent_notification` et la clé de
//! dédoublonnage des envois empêchent tout doublon ; un plantage avant le commit ne laisse rien.

use super::catalogue::{nature, nature_de_kind_ancien};
use super::contrat::*;
use super::destinataires::lien_toujours_valable;
use crate::notifications::{db, notifier_personne, EnvoiAPlanifier};
use crate::service::EntDbError;
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BilanAvis {
    pub avis: usize,
    pub remises_inapp: usize,
    pub transmises: usize,
    pub annulees: usize,
}

/// Lot maximal d'avis traités par passage et par établissement.
const LOT: i64 = 200;

fn jour_fr(j: NaiveDate) -> String {
    j.format("%d/%m/%Y").to_string()
}

fn rendre(texte: &str, prenom: &str, jour: Option<NaiveDate>) -> String {
    texte
        .replace("{prenom}", prenom)
        .replace("{jour}", &jour.map(jour_fr).unwrap_or_else(|| "ce jour".into()))
}

/// Ancien `kind` de l'ENT pour une nature (l'interface le reconnaît) ; à défaut, le second segment du code.
fn kind_pour(code: &str) -> String {
    for k in ["absence_constatee", "retard", "infirmerie_passage"] {
        if nature_de_kind_ancien(k) == Some(code) {
            return k.to_string();
        }
    }
    code.split('.').nth(1).unwrap_or(code).to_string()
}

fn droit_ancien(d: DroitRequis) -> &'static str {
    match d {
        DroitRequis::Lien => "lien",
        DroitRequis::VieScolaire => "peut_voir_vs",
        DroitRequis::Notes => "peut_voir_notes",
    }
}

/// Remet les avis échus d'un établissement à l'instant `maintenant`.
pub async fn expedier_avis_dus(
    pool: &PgPool,
    etablissement_id: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<BilanAvis, EntDbError> {
    let mut tx = TenantTx::begin(pool, etablissement_id).await.map_err(db)?;
    let dus: Vec<(Uuid, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT id, nature, eleve_id FROM ent_ntf_avis
          WHERE etablissement_id = $1 AND statut = 'planifie' AND confirme_apres <= $2
          ORDER BY confirme_apres, id LIMIT $3 FOR UPDATE SKIP LOCKED",
    )
    .bind(etablissement_id)
    .bind(maintenant)
    .bind(LOT)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let mut bilan = BilanAvis::default();
    for (avis_id, code, eleve_id) in dus {
        let Some(n) = nature(&code) else {
            // Nature retirée du code : on ne devine pas un texte, l'avis est classé sans suite.
            sqlx::query(
                "UPDATE ent_ntf_avis SET statut = 'annule', annule_le = $3, annule_motif = 'nature_inconnue'
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etablissement_id)
            .bind(avis_id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            continue;
        };
        let jour: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT valeur_jour FROM ent_ntf_avis_param WHERE etablissement_id = $1 AND avis_id = $2 AND cle = 'jour'",
        )
        .bind(etablissement_id)
        .bind(avis_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .flatten();
        let prenom: String = match eleve_id {
            Some(e) => sqlx::query_scalar("SELECT prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                .bind(etablissement_id)
                .bind(e)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .unwrap_or_default(),
            None => String::new(),
        };
        let titre = rendre(n.gabarit.titre, &prenom, jour);
        let corps = rendre(n.gabarit.corps, &prenom, jour);
        let destinataires: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, compte_id FROM ent_ntf_destinataire
              WHERE etablissement_id = $1 AND avis_id = $2 AND retenu AND statut = 'a_remettre' ORDER BY created_at, id",
        )
        .bind(etablissement_id)
        .bind(avis_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // Le SMS ne part que pour une nature que l'ancien moteur sait neutraliser (kind connu).
        let kind = kind_pour(n.code);
        let sms_possible = nature_de_kind_ancien(&kind).is_some();
        let canaux_externes: Vec<&str> = n
            .canaux_defaut
            .iter()
            .filter(|c| c.externe() && (**c != Canal::Sms || sms_possible))
            .map(|c| c.code())
            .collect();
        for (dest_id, compte) in destinataires {
            let (Some(compte), Some(eleve)) = (compte, eleve_id) else {
                continue;
            };
            // Relecture du lien à chaque remise (R-N1) : le droit peut avoir changé depuis l'avis.
            if !lien_toujours_valable(&mut *tx, etablissement_id, eleve, compte, n.droit_requis).await.map_err(db)? {
                sqlx::query(
                    "UPDATE ent_ntf_remise SET statut = 'annulee', motif = 'droit_retire'
                      WHERE etablissement_id = $1 AND destinataire_id = $2 AND statut = 'planifiee'",
                )
                .bind(etablissement_id)
                .bind(dest_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                sqlx::query("UPDATE ent_ntf_destinataire SET statut = 'annule' WHERE etablissement_id = $1 AND id = $2")
                    .bind(etablissement_id)
                    .bind(dest_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                bilan.annulees += 1;
                continue;
            }
            notifier_personne(
                &mut *tx,
                EnvoiAPlanifier {
                    etablissement_id,
                    user_id: compte,
                    kind: &kind,
                    titre: &titre,
                    corps: &corps,
                    eleve_id,
                    notification_id: None,
                    source_type: "evenement",
                    source_id: avis_id,
                    cle_dedup: format!("avis:{avis_id}"),
                    droit_requis: Some(droit_ancien(n.droit_requis)),
                    regroupable: !matches!(n.regroupement, Regroupement::Aucun | Regroupement::FoyerDemiJournee),
                    canaux_voulus: &canaux_externes,
                },
                avis_id,
                maintenant,
            )
            .await?;
            let notification: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM ent_notification
                  WHERE source_event_id = $1 AND destinataire_user_id = $2 AND kind = $3",
            )
            .bind(avis_id)
            .bind(compte)
            .bind(&kind)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let r = sqlx::query(
                "UPDATE ent_ntf_remise SET statut = 'remise', remise_le = $3, transmise_le = $3, fournisseur = 'ent',
                        notification_id = $4, tentatives = tentatives + 1
                  WHERE etablissement_id = $1 AND destinataire_id = $2 AND canal = 'inapp' AND statut = 'planifiee'",
            )
            .bind(etablissement_id)
            .bind(dest_id)
            .bind(maintenant)
            .bind(notification)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            bilan.remises_inapp += r.rows_affected() as usize;
            let t = sqlx::query(
                "UPDATE ent_ntf_remise SET statut = 'transmise', transmise_le = $3, fournisseur = 'ancien_moteur',
                        tentatives = tentatives + 1
                  WHERE etablissement_id = $1 AND destinataire_id = $2 AND canal <> 'inapp' AND statut = 'planifiee'",
            )
            .bind(etablissement_id)
            .bind(dest_id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            bilan.transmises += t.rows_affected() as usize;
            sqlx::query(
                "UPDATE ent_ntf_destinataire SET statut = 'remis', premier_remis_le = $3, notifie_le = $3
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etablissement_id)
            .bind(dest_id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        sqlx::query("UPDATE ent_ntf_avis SET statut = 'termine' WHERE etablissement_id = $1 AND id = $2")
            .bind(etablissement_id)
            .bind(avis_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        bilan.avis += 1;
    }
    tx.commit().await.map_err(db)?;
    Ok(bilan)
}
