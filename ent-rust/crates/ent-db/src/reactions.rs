//! Premières réactions entre domaines (socle 2, lot S2-0).
//!
//! Consommateur « notifications » (D0) : transforme des faits de vie scolaire
//! (D5) en notifications internes (`ent_notification`) pour les responsables
//! **autorisés** de l'élève, relus au moment du traitement :
//!
//! | Événement | Condition | Destinataires |
//! |---|---|---|
//! | `eleve.absence_declared` | source = appel (absence constatée) | responsables `peut_voir_vs` |
//! | `eleve.absence_justified` / `eleve.absence_refused` | — | responsables `peut_voir_vs` |
//! | `vs.retard` | — | responsables `peut_voir_vs` |
//!
//! Règles de la toile respectées : charge utile minimale relue (prénom de
//! l'élève relu dans l'établissement de l'événement) ; aucun texte libre de
//! l'événement recopié (ni motif, ni note de décision) ; écriture dans le seul
//! domaine D0 ; idempotence par la marque du distributeur **et** par la clé
//! unique (`source_event_id`, destinataire, kind) : un rejeu ne duplique rien.
//! Ce consommateur est rejouable (aucun envoi externe) ; le canal mail/SMS/push
//! viendra comme consommateur séparé, non rejouable.

use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};
use ent_events::types;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use std::sync::Arc;
use uuid::Uuid;

pub const CONSOMMATEUR_NOTIFICATIONS: &str = "d0.notifications";

pub struct ConsommateurNotifications;

const TYPES_NOTIFICATIONS: &[&str] = &[
    types::ELEVE_ABSENCE_DECLARED,
    types::ELEVE_ABSENCE_JUSTIFIED,
    types::ELEVE_ABSENCE_REFUSED,
    types::RETARD_DECLARE,
];

/// Consommateurs installés par défaut dans le distributeur de l'ENT.
pub fn consommateurs_standard() -> Vec<Arc<dyn Consommateur>> {
    vec![
        Arc::new(ConsommateurNotifications),
        Arc::new(ConsommateurRelanceAppel),
        Arc::new(crate::cahier_reactions::ConsommateurCahier),
    ]
}

struct Message {
    kind: &'static str,
    titre: String,
    corps: String,
}

fn jour_fr(iso: Option<&str>) -> String {
    iso.and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| d.format("%d/%m/%Y").to_string())
        .unwrap_or_else(|| "ce jour".into())
}

/// Message à produire, ou `None` si l'événement n'appelle pas de notification.
fn message_pour(evt: &EvenementDistribue, prenom: &str) -> Option<Message> {
    let e = evt.event();
    let p = &e.payload;
    match e.type_name.as_str() {
        types::ELEVE_ABSENCE_DECLARED => {
            // Seule l'absence **constatée** à l'appel est notifiée : une absence
            // déclarée par la famille ou l'accueil est déjà connue de la famille.
            if p.get("source").and_then(|v| v.as_str()) != Some("appel") {
                return None;
            }
            // Absence déjà annoncée (déclaration de l'accueil ou de la famille
            // rapprochée à l'appel, S2-4) : la famille est déjà prévenue.
            if p.get("prevenue").and_then(|v| v.as_bool()) == Some(true) {
                return None;
            }
            let jour = jour_fr(p.get("debut").and_then(|v| v.as_str()));
            Some(Message {
                kind: "absence_constatee",
                titre: format!("Absence de {prenom}"),
                corps: format!(
                    "{prenom} a été noté(e) absent(e) à l'appel le {jour}. \
                     Vous pouvez justifier cette absence depuis le portail famille."
                ),
            })
        }
        types::ELEVE_ABSENCE_JUSTIFIED => Some(Message {
            kind: "absence_justifiee",
            titre: format!("Absence de {prenom} justifiée"),
            corps: format!("Le justificatif d'absence de {prenom} a été accepté."),
        }),
        types::ELEVE_ABSENCE_REFUSED => Some(Message {
            kind: "absence_refusee",
            titre: format!("Justificatif de {prenom} refusé"),
            corps: format!(
                "Le justificatif d'absence de {prenom} n'a pas été accepté. \
                 Consultez la vie scolaire depuis le portail famille."
            ),
        }),
        types::RETARD_DECLARE => {
            let jour = jour_fr(p.get("jour").and_then(|v| v.as_str()));
            Some(Message {
                kind: "retard",
                titre: format!("Retard de {prenom}"),
                corps: format!("{prenom} a été noté(e) en retard le {jour}."),
            })
        }
        _ => None,
    }
}

async fn notifier(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let Some(eleve_id) = evt.event().eleve_id else {
        return Ok(());
    };
    // Relecture dans l'établissement de l'événement : un élève d'un autre
    // établissement est introuvable ici, donc aucune notification.
    let prenom: Option<(String,)> =
        sqlx::query_as("SELECT prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    let Some((prenom,)) = prenom else {
        return Ok(());
    };
    let Some(m) = message_pour(evt, &prenom) else {
        return Ok(());
    };
    // Droits relus maintenant : lien ACTIF (vue unique, S2-1 : ni suspendu ni
    // hors de ses dates) dans cet établissement, droit de voir
    // la vie scolaire, compte « parent » membre de l'établissement.
    let destinataires: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT l.user_id
        FROM ent_eleve_lien_actif l
        JOIN ent_membership m
          ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
        WHERE l.etablissement_id = $1 AND l.eleve_id = $2
          AND l.peut_voir_vs AND m.role = 'parent'
        "#,
    )
    .bind(etab)
    .bind(eleve_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    for (user_id,) in destinataires {
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
        .bind(etab)
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

impl Consommateur for ConsommateurNotifications {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_NOTIFICATIONS
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_NOTIFICATIONS
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(notifier(tx, evt))
    }
}

/// Notification interne telle que lue par son destinataire.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct NotificationInterne {
    pub id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub kind: String,
    pub titre: String,
    pub corps: String,
    pub source_event_id: Uuid,
    pub lu_le: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub(crate) async fn notifications_de(
    pool: &PgPool,
    etablissement_id: Uuid,
    user_id: Uuid,
    limite: i64,
) -> Result<Vec<NotificationInterne>, sqlx::Error> {
    sqlx::query_as::<_, NotificationInterne>(
        r#"
        SELECT id, eleve_id, kind, titre, corps, source_event_id, lu_le, created_at
        FROM ent_notification
        WHERE etablissement_id = $1 AND destinataire_user_id = $2
        ORDER BY created_at DESC, id
        LIMIT $3
        "#,
    )
    .bind(etablissement_id)
    .bind(user_id)
    .bind(limite.clamp(1, 200))
    .fetch_all(pool)
    .await
}

// ---------------------------------------------------------------------------
// Relance d'un appel non fait (S2-4, VS-04 / A2) : notifie l'enseignant.
// ---------------------------------------------------------------------------

pub const CONSOMMATEUR_RELANCE_APPEL: &str = "d5.relance_appel";

pub struct ConsommateurRelanceAppel;

const TYPES_RELANCE: &[&str] = &["vs.appel_relance"];

/// Le destinataire est **relu** dans la relance de l'établissement de l'événement
/// (jamais pris dans la charge utile) : un événement forgé ne notifie personne.
async fn notifier_relance(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let Some(relance_id) = evt
        .event()
        .payload
        .get("relance_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    else {
        return Ok(());
    };
    let row: Option<(Uuid, chrono::NaiveDate, String, String, chrono::NaiveTime)> = sqlx::query_as(
        r#"
        SELECT r.destinataire_user_id, r.jour, c.classe, c.matiere_libelle, c.heure_debut
        FROM ent_appel_relance r
        JOIN ent_edt_creneau c
          ON c.id = r.creneau_id AND c.etablissement_id = r.etablissement_id
        JOIN ent_membership m
          ON m.user_id = r.destinataire_user_id AND m.etablissement_id = r.etablissement_id
        WHERE r.etablissement_id = $1 AND r.id = $2
        "#,
    )
    .bind(etab)
    .bind(relance_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let Some((user_id, jour, classe, matiere, heure)) = row else {
        return Ok(());
    };
    sqlx::query(
        r#"
        INSERT INTO ent_notification (
            id, etablissement_id, destinataire_user_id, eleve_id,
            kind, titre, corps, source_event_id
        ) VALUES ($1,$2,$3,NULL,'appel_a_faire',$4,$5,$6)
        ON CONFLICT (source_event_id, destinataire_user_id, kind) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(user_id)
    .bind(format!("Appel à faire : {classe}"))
    .bind(format!(
        "L'appel de {matiere} ({classe}) du {} à {} n'est pas fait. Merci de le faire dès que possible.",
        jour_fr(Some(&jour.to_string())),
        heure.format("%H:%M")
    ))
    .bind(evt.event().id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

impl Consommateur for ConsommateurRelanceAppel {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_RELANCE_APPEL
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_RELANCE
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(notifier_relance(tx, evt))
    }
}
