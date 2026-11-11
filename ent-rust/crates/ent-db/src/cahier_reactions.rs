//! Réactions du cahier de textes (lot S2-6, domaine D7 → D0).
//!
//! Consommateur `d7.notifications` : transforme des faits du cahier en notifications internes
//! (`ent_notification`), destinataires et textes relus **au traitement** dans l'établissement
//! de l'événement.
//!
//! | Événement | Destinataires |
//! |---|---|
//! | `cahier.travail_donne` / `cahier.travail_a_rendre` | élèves de la cible et responsables liés |
//! | `cahier.travail_modifie` (si un élève l'avait déjà vu) | idem |
//! | `cahier.travail_rendu` | professeur auteur du travail |
//! | `cahier.copie_rendue` (copie corrigée) | l'élève et ses responsables liés (le responsable seulement si son lien exerce l'autorité parentale, et, pour un retour portant une note, a le droit « voir les notes ») |
//! | `cahier.travail_supprime` (retrait par la direction) | professeur auteur du travail |
//!
//! C-D5 : les élèves avertis sont ceux du public **à la date du traitement** (affectation active, élève non
//! parti). C-D4 : un lien suspendu, hors dates ou sans autorité parentale ne reçoit rien.
//!
//! Idempotence : marque du distributeur **et** clé unique (`source_event_id`, destinataire,
//! `kind`) ; un rejeu ne duplique rien. Aucun envoi externe : consommateur rejouable.

use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};
use chrono::NaiveDate;
use ent_events::types;
use sqlx::PgConnection;
use uuid::Uuid;

pub const CONSOMMATEUR_CAHIER: &str = "d7.notifications";

pub struct ConsommateurCahier;

const TYPES_CAHIER: &[&str] = &[
    types::CAHIER_TRAVAIL_DONNE,
    types::CAHIER_TRAVAIL_A_RENDRE,
    types::CAHIER_TRAVAIL_MODIFIE,
    types::CAHIER_TRAVAIL_RENDU,
    types::CAHIER_COPIE_RENDUE,
    types::CAHIER_TRAVAIL_SUPPRIME,
];

fn jour_fr(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

struct Message {
    kind: &'static str,
    titre: String,
    corps: String,
}

fn payload_uuid(evt: &EvenementDistribue, cle: &str) -> Option<Uuid> {
    evt.event()
        .payload
        .get(cle)
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

async fn inserer(
    tx: &mut PgConnection,
    evt: &EvenementDistribue,
    destinataires: &[Uuid],
    eleve_id: Option<Uuid>,
    m: &Message,
) -> Result<(), String> {
    for user_id in destinataires {
        sqlx::query(
            r#"
            INSERT INTO ent_notification (
                id, etablissement_id, destinataire_user_id, eleve_id,
                kind, titre, corps, source_event_id
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (source_event_id, destinataire_user_id, kind) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(evt.etablissement_id())
        .bind(user_id)
        .bind(eleve_id)
        .bind(m.kind)
        .bind(&m.titre)
        .bind(&m.corps)
        .bind(evt.event().id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Comptes de l'élève (profil `eleve`) et responsables liés (profil `parent` de l'établissement).
async fn comptes_eleve(
    tx: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    pour_note: bool,
) -> Result<Vec<Uuid>, String> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT c.user_id FROM ent_eleve_compte c
         WHERE c.etablissement_id = $1 AND c.eleve_id = $2
        UNION
        SELECT l.user_id FROM ent_eleve_lien_actif l
          JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
         WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND m.role = 'parent'
           AND l.autorite_parentale AND (NOT $3 OR l.peut_voir_notes)
        "#,
    )
    .bind(etab)
    .bind(eleve)
    .bind(pour_note)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(u,)| u).collect())
}

async fn traiter_cahier(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let e = evt.event();
    match e.type_name.as_str() {
        types::CAHIER_TRAVAIL_DONNE | types::CAHIER_TRAVAIL_A_RENDRE | types::CAHIER_TRAVAIL_MODIFIE => {
            if e.type_name == types::CAHIER_TRAVAIL_MODIFIE
                && e.payload.get("deja_vu").and_then(|v| v.as_bool()) != Some(true)
            {
                return Ok(());
            }
            let Some(travail_id) = payload_uuid(evt, "travail_id") else {
                return Ok(());
            };
            // Relu dans l'établissement de l'événement ; supprimé entre-temps → rien.
            let t: Option<(String, String, NaiveDate)> = sqlx::query_as(
                "SELECT matiere_libelle, titre, pour_le FROM ent_travail
                  WHERE etablissement_id = $1 AND id = $2 AND supprime_le IS NULL",
            )
            .bind(etab)
            .bind(travail_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            let Some((matiere, titre, pour_le)) = t else {
                return Ok(());
            };
            let (kind, entete) = match e.type_name.as_str() {
                types::CAHIER_TRAVAIL_DONNE => ("travail_donne", "Nouveau travail"),
                types::CAHIER_TRAVAIL_A_RENDRE => ("travail_a_rendre", "Travail à rendre"),
                _ => ("travail_modifie", "Travail modifié"),
            };
            let eleves: Vec<(Uuid,)> = sqlx::query_as(
                r#"
                SELECT DISTINCT a.eleve_id
                  FROM ent_travail t
                  JOIN ent_affectation a ON a.etablissement_id = t.etablissement_id
                   AND (a.classe_id = t.classe_id OR a.groupe_id = t.groupe_id)
                   AND a.date_debut <= ent_aujourdhui_etab(t.etablissement_id)
                   AND (a.date_fin IS NULL OR a.date_fin >= ent_aujourdhui_etab(t.etablissement_id))
                  JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
                   AND (e.date_sortie IS NULL OR e.date_sortie > ent_aujourdhui_etab(t.etablissement_id))
                 WHERE t.etablissement_id = $1 AND t.id = $2
                "#,
            )
            .bind(etab)
            .bind(travail_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            let m = Message {
                kind,
                titre: format!("{entete} en {matiere}"),
                corps: format!("« {titre} », pour le {}.", jour_fr(pour_le)),
            };
            for (eleve,) in eleves {
                let dest = comptes_eleve(tx, etab, eleve, false).await?;
                inserer(tx, evt, &dest, Some(eleve), &m).await?;
            }
            Ok(())
        }
        types::CAHIER_TRAVAIL_RENDU => {
            let (Some(travail_id), Some(remise_id)) =
                (payload_uuid(evt, "travail_id"), payload_uuid(evt, "remise_id"))
            else {
                return Ok(());
            };
            let r: Option<(Uuid, String, String, String, bool, Uuid)> = sqlx::query_as(
                r#"
                SELECT t.auteur_user_id, t.titre, e.prenom, e.nom, r.en_retard, e.id
                  FROM ent_travail_remise r
                  JOIN ent_travail t ON t.id = r.travail_id AND t.etablissement_id = r.etablissement_id
                  JOIN ent_eleve e ON e.id = r.eleve_id AND e.etablissement_id = r.etablissement_id
                 WHERE r.etablissement_id = $1 AND r.id = $2 AND t.id = $3
                "#,
            )
            .bind(etab)
            .bind(remise_id)
            .bind(travail_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            let Some((auteur, titre, prenom, nom, retard, eleve)) = r else {
                return Ok(());
            };
            let m = Message {
                kind: "copie_a_corriger",
                titre: format!("Copie reçue : {prenom} {nom}"),
                corps: if retard {
                    format!("{prenom} {nom} a rendu « {titre} » en retard.")
                } else {
                    format!("{prenom} {nom} a rendu « {titre} ».")
                },
            };
            inserer(tx, evt, &[auteur], Some(eleve), &m).await
        }
        types::CAHIER_COPIE_RENDUE => {
            let (Some(travail_id), Some(eleve)) = (payload_uuid(evt, "travail_id"), e.eleve_id) else {
                return Ok(());
            };
            let t: Option<(String, String, bool)> = sqlx::query_as(
                r#"
                SELECT t.matiere_libelle, t.titre,
                       EXISTS (SELECT 1 FROM ent_travail_remise r
                                WHERE r.etablissement_id = t.etablissement_id AND r.travail_id = t.id
                                  AND r.eleve_id = e.id AND r.note_id IS NOT NULL)
                  FROM ent_travail t
                  JOIN ent_eleve e ON e.etablissement_id = t.etablissement_id AND e.id = $3
                 WHERE t.etablissement_id = $1 AND t.id = $2
                "#,
            )
            .bind(etab)
            .bind(travail_id)
            .bind(eleve)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            let Some((matiere, titre, avec_note)) = t else {
                return Ok(());
            };
            let m = Message {
                kind: "copie_corrigee",
                titre: format!("Copie corrigée en {matiere}"),
                corps: format!("La correction de « {titre} » est disponible."),
            };
            let dest = comptes_eleve(tx, etab, eleve, avec_note).await?;
            inserer(tx, evt, &dest, Some(eleve), &m).await
        }
        types::CAHIER_TRAVAIL_SUPPRIME => {
            // Seul le retrait par la direction avertit l'auteur ; sa propre suppression, non.
            if e.payload.get("retire_par_direction").and_then(|v| v.as_bool()) != Some(true) {
                return Ok(());
            }
            let Some(travail_id) = payload_uuid(evt, "travail_id") else {
                return Ok(());
            };
            let t: Option<(Uuid, String, Option<String>)> = sqlx::query_as(
                "SELECT auteur_user_id, titre, retire_motif FROM ent_travail
                  WHERE etablissement_id = $1 AND id = $2 AND retire_par_user_id IS NOT NULL",
            )
            .bind(etab)
            .bind(travail_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            let Some((auteur, titre, motif)) = t else {
                return Ok(());
            };
            let m = Message {
                kind: "travail_retire",
                titre: "Travail retiré de la diffusion".to_string(),
                corps: format!(
                    "La direction a retiré « {titre} » de la diffusion. Motif : {}.",
                    motif.unwrap_or_default()
                ),
            };
            inserer(tx, evt, &[auteur], None, &m).await
        }
        _ => Ok(()),
    }
}

impl Consommateur for ConsommateurCahier {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_CAHIER
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_CAHIER
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(traiter_cahier(tx, evt))
    }
}
