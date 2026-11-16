//! Socle 8, lot N1 — planificateur et expédition des avis échus.
//!
//! Un avis échu (délai de confirmation écoulé) est remis par établissement, en `TenantTx` : relecture du lien
//! de chaque destinataire (R-N1), rendu du gabarit français, remise dans l'ENT (`ent_notification`) puis
//! transmission des canaux externes à l'**ancien moteur** (façade : préférences, heures calmes et plafonds y
//! restent jusqu'aux lots N3 et N5–N7). Rejouable : la clé d'idempotence de `ent_notification` et la clé de
//! dédoublonnage des envois empêchent tout doublon ; un plantage avant le commit ne laisse rien.

use super::catalogue::{nature, nature_de_kind_ancien};
use super::contrat::*;
use super::destinataires::reclasser;
use crate::notifications::{db, notifier_personne_cibles, EnvoiAPlanifier};
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
    for k in ["absence_constatee", "retard", "infirmerie_passage", "absence_justifiee"] {
        if nature_de_kind_ancien(k) == Some(code) {
            return k.to_string();
        }
    }
    code.split('.').nth(1).unwrap_or(code).to_string()
}

/// Motif tracé sur la remise annulée : `droit_retire` reste le mot de l'ancien chemin pour le droit du lien.
fn motif_remise(code: &str) -> &str {
    if code == "droit_insuffisant" {
        "droit_retire"
    } else {
        code
    }
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
        let destinataires: Vec<(Uuid, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, compte_id, personne_id FROM ent_ntf_destinataire
              WHERE etablissement_id = $1 AND avis_id = $2 AND retenu AND statut = 'a_remettre' ORDER BY created_at, id",
        )
        .bind(etablissement_id)
        .bind(avis_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let kind = kind_pour(n.code);
        // Le SMS ne part que pour une nature que l'ancien moteur sait neutraliser (kind connu).
        let sms_possible = nature_de_kind_ancien(&kind).is_some();
        for (dest_id, compte, personne) in destinataires {
            // Personne sans compte : remise directe par l'expéditeur (`reserver_remises_sans_compte`).
            let (Some(compte), Some(eleve), Some(personne)) = (compte, eleve_id, personne) else {
                continue;
            };
            // Relecture à chaque remise (R-N1) : lien, droit, interdiction, « ne pas solliciter », gel.
            if let Some(code) = reclasser(&mut tx, etablissement_id, eleve, personne, n.droit_requis, maintenant)
                .await
                .map_err(db)?
            {
                sqlx::query(
                    "UPDATE ent_ntf_remise SET statut = 'annulee', motif = $3
                      WHERE etablissement_id = $1 AND destinataire_id = $2 AND statut = 'planifiee'",
                )
                .bind(etablissement_id)
                .bind(dest_id)
                .bind(motif_remise(code))
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
            // Canaux externes et instants décidés à la création de l'avis (joignabilité, préférences, heures
            // calmes, résumé, plancher, dédoublonnage) : l'ancien moteur n'en décide plus rien (façade).
            let externes: Vec<(String, DateTime<Utc>, String)> = sqlx::query_as(
                "SELECT canal, planifiee_pour, motif_planification FROM ent_ntf_remise
                  WHERE etablissement_id = $1 AND destinataire_id = $2 AND canal <> 'inapp' AND statut = 'planifiee'",
            )
            .bind(etablissement_id)
            .bind(dest_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let cibles: Vec<(String, DateTime<Utc>, bool)> = externes
                .into_iter()
                .filter(|(c, _, _)| c != "sms" || sms_possible)
                .map(|(c, quand, motif)| (c, quand, motif.starts_with("resume_") || motif == "budget_bruit"))
                .collect();
            notifier_personne_cibles(
                &mut tx,
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
                    regroupable: false,
                    canaux_voulus: &[],
                },
                avis_id,
                &cibles,
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
        // Terminé quand plus personne n'attend ; sinon `en_cours` (personnes sans compte remises ensuite).
        sqlx::query(
            "UPDATE ent_ntf_avis a SET statut = CASE WHEN EXISTS (
                    SELECT 1 FROM ent_ntf_destinataire d WHERE d.etablissement_id = a.etablissement_id
                       AND d.avis_id = a.id AND d.retenu AND d.statut = 'a_remettre')
                 THEN 'en_cours' ELSE 'termine' END
              WHERE a.etablissement_id = $1 AND a.id = $2",
        )
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

// ───────────────────────── Personnes sans compte (N2) ─────────────────────────

/// Une remise à envoyer directement à une personne **sans compte** (e-mail), prise sous verrou.
#[derive(Debug, Clone)]
pub struct RemiseDirecte {
    pub remise_id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub kind: String,
    pub destination: String,
    pub sujet: String,
    pub corps: String,
}

/// Issue de l'envoi direct d'une remise.
#[derive(Debug, Clone)]
pub enum IssueDirecte {
    Envoye,
    Simule,
    Ignore(&'static str),
    Echec(String),
}

/// Prend les remises e-mail échues des personnes sans compte : relecture (R-N1), rendu du gabarit, adresse lue
/// dans la fiche de la personne. Les remises prises passent à `reservee` : un plantage ensuite ne renvoie rien.
pub async fn reserver_remises_sans_compte(
    pool: &PgPool,
    etablissement_id: Uuid,
    maintenant: DateTime<Utc>,
    limite: i64,
) -> Result<Vec<RemiseDirecte>, EntDbError> {
    let mut tx = TenantTx::begin(pool, etablissement_id).await.map_err(db)?;
    let lignes: Vec<(Uuid, Uuid, Uuid, String, Option<Uuid>, Uuid)> = sqlx::query_as(
        "SELECT r.id, d.id, a.id, a.nature, a.eleve_id, d.personne_id
           FROM ent_ntf_remise r
           JOIN ent_ntf_destinataire d ON d.etablissement_id = r.etablissement_id AND d.id = r.destinataire_id
           JOIN ent_ntf_avis a ON a.etablissement_id = d.etablissement_id AND a.id = d.avis_id
          WHERE r.etablissement_id = $1 AND r.statut = 'planifiee' AND r.canal = 'email'
            AND d.compte_id IS NULL AND d.personne_id IS NOT NULL AND d.retenu AND d.statut = 'a_remettre'
            AND a.statut IN ('planifie', 'en_cours') AND r.planifiee_pour <= $2 AND a.confirme_apres <= $2
          ORDER BY r.planifiee_pour, r.id LIMIT $3
          FOR UPDATE OF r SKIP LOCKED",
    )
    .bind(etablissement_id)
    .bind(maintenant)
    .bind(limite)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let mut prises = Vec::new();
    for (remise_id, dest_id, avis_id, code, eleve_id, personne_id) in lignes {
        let Some(n) = nature(&code) else { continue };
        let motif_ecart: Option<String> = match eleve_id {
            Some(eleve) => reclasser(&mut tx, etablissement_id, eleve, personne_id, n.droit_requis, maintenant)
                .await
                .map_err(db)?
                .map(|c| motif_remise(c).to_string()),
            None => Some("sujet_absent".into()),
        };
        let adresse: Option<String> = sqlx::query_scalar(
            "SELECT email FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(personne_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .flatten()
        .map(|e: String| e.trim().to_string())
        .filter(|e| e.contains('@'));
        let (statut, motif) = match (&motif_ecart, &adresse) {
            (Some(m), _) => ("annulee", Some(m.clone())),
            (None, None) => ("ignoree", Some("sans_adresse".to_string())),
            (None, Some(_)) => ("reservee", None),
        };
        sqlx::query(
            "UPDATE ent_ntf_remise SET statut = $3, motif = $4, tentatives = tentatives + 1
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(remise_id)
        .bind(statut)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let (Some(adresse), None) = (adresse, motif_ecart) else {
            sqlx::query("UPDATE ent_ntf_destinataire SET statut = 'annule' WHERE etablissement_id = $1 AND id = $2")
                .bind(etablissement_id)
                .bind(dest_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            terminer_si_personne_n_attend(&mut tx, etablissement_id, avis_id).await?;
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
        prises.push(RemiseDirecte {
            remise_id,
            etablissement_id,
            eleve_id,
            kind: kind_pour(n.code),
            destination: adresse,
            sujet: rendre(n.gabarit.titre, &prenom, jour),
            corps: rendre(n.gabarit.corps, &prenom, jour),
        });
    }
    tx.commit().await.map_err(db)?;
    Ok(prises)
}

async fn terminer_si_personne_n_attend(
    tx: &mut sqlx::PgConnection,
    etablissement_id: Uuid,
    avis_id: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_ntf_avis a SET statut = 'termine'
          WHERE a.etablissement_id = $1 AND a.id = $2 AND a.statut = 'en_cours'
            AND NOT EXISTS (SELECT 1 FROM ent_ntf_destinataire d WHERE d.etablissement_id = a.etablissement_id
                               AND d.avis_id = a.id AND d.retenu AND d.statut = 'a_remettre')",
    )
    .bind(etablissement_id)
    .bind(avis_id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Enregistre l'issue d'un envoi direct : remise, destinataire (remis ou non joint) et, si plus personne
/// n'attend, l'avis.
pub async fn finaliser_remise_directe(
    pool: &PgPool,
    etablissement_id: Uuid,
    remise_id: Uuid,
    issue: IssueDirecte,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let mut tx = TenantTx::begin(pool, etablissement_id).await.map_err(db)?;
    let (statut, motif, echec, remis): (&str, Option<&str>, Option<&str>, bool) = match &issue {
        IssueDirecte::Envoye => ("remise", None, None, true),
        IssueDirecte::Simule => ("remise", Some("simule"), None, true),
        IssueDirecte::Ignore(m) => ("ignoree", Some(m), None, false),
        IssueDirecte::Echec(_) => ("echec_definitif", None, Some("envoi"), false),
    };
    let (dest_id, avis_id): (Uuid, Uuid) = sqlx::query_as(
        "UPDATE ent_ntf_remise r SET statut = $3, motif = $4, echec_code = $5, fournisseur = 'courriel',
                transmise_le = $6, remise_le = CASE WHEN $7 THEN $6 END
           FROM ent_ntf_destinataire d
          WHERE r.etablissement_id = $1 AND r.id = $2 AND d.etablissement_id = r.etablissement_id AND d.id = r.destinataire_id
          RETURNING d.id, d.avis_id",
    )
    .bind(etablissement_id)
    .bind(remise_id)
    .bind(statut)
    .bind(motif)
    .bind(echec)
    .bind(maintenant)
    .bind(remis)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_ntf_destinataire SET statut = $3, premier_remis_le = CASE WHEN $4 THEN $5 END,
                notifie_le = CASE WHEN $4 THEN $5 END
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etablissement_id)
    .bind(dest_id)
    .bind(if remis { "remis" } else { "non_joint" })
    .bind(remis)
    .bind(maintenant)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    terminer_si_personne_n_attend(&mut tx, etablissement_id, avis_id).await?;
    tx.commit().await.map_err(db)?;
    Ok(())
}
