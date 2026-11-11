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

use crate::notifications::{notifier_personne, EnvoiAPlanifier};
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
    // S2-11 : information de la famille après un passage à l'infirmerie.
    "infirmerie.famille_informee",
];

/// Consommateurs installés par défaut dans le distributeur de l'ENT.
pub fn consommateurs_standard() -> Vec<Arc<dyn Consommateur>> {
    vec![
        Arc::new(ConsommateurNotifications),
        Arc::new(ConsommateurRelanceAppel),
        Arc::new(crate::cahier_reactions::ConsommateurCahier),
        Arc::new(crate::edt_reactions::ConsommateurNotificationsEdt),
        Arc::new(ConsommateurEvaluations),
        Arc::new(crate::voyages_reactions::ConsommateurVoyages),
    ]
}

struct Message {
    kind: &'static str,
    titre: String,
    corps: String,
    /// Les décisions et retards se regroupent en résumé quotidien ; une absence
    /// constatée part tout de suite.
    regroupable: bool,
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
                regroupable: false,
                titre: format!("Absence de {prenom}"),
                corps: format!(
                    "{prenom} a été noté(e) absent(e) à l'appel le {jour}. \
                     Vous pouvez justifier cette absence depuis le portail famille."
                ),
            })
        }
        types::ELEVE_ABSENCE_JUSTIFIED => Some(Message {
            kind: "absence_justifiee",
            regroupable: true,
            titre: format!("Absence de {prenom} justifiée"),
            corps: format!("Le justificatif d'absence de {prenom} a été accepté."),
        }),
        types::ELEVE_ABSENCE_REFUSED => Some(Message {
            kind: "absence_refusee",
            regroupable: true,
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
                regroupable: true,
                titre: format!("Retard de {prenom}"),
                corps: format!("{prenom} a été noté(e) en retard le {jour}."),
            })
        }
        // Aucun détail médical : ni motif, ni soins, ni issue (SAN-03).
        "infirmerie.famille_informee" => {
            let jour = jour_fr(p.get("jour").and_then(|v| v.as_str()));
            Some(Message {
                kind: "infirmerie_passage",
                regroupable: false,
                titre: format!("Passage de {prenom} à l'infirmerie"),
                corps: format!(
                    "{prenom} s'est présenté(e) à l'infirmerie le {jour}. \
                     Pour en savoir plus, contactez l'infirmerie ou la vie scolaire."
                ),
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
        FROM ent_eleve_lien_sollicitable l
        JOIN ent_membership m
          ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
        WHERE l.etablissement_id = $1 AND l.eleve_id = $2
          AND (l.peut_voir_vs OR $3) AND m.role = 'parent'
        "#,
    )
    .bind(etab)
    .bind(eleve_id)
    // Information de santé : tous les responsables au lien actif, indépendamment
    // du droit « vie scolaire » (la famille doit savoir, sans détail).
    .bind(evt.event().type_name == "infirmerie.famille_informee")
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    for (user_id,) in destinataires {
        // Dans l'ENT (idempotent) puis envois e-mail / SMS / push planifiés selon les
        // préférences de la personne (S2-8). Les envois sont expédiés plus tard, hors
        // de cette transaction : ce consommateur reste rejouable.
        notifier_personne(
            &mut *tx,
            EnvoiAPlanifier {
                etablissement_id: etab,
                user_id,
                kind: m.kind,
                titre: &m.titre,
                corps: &m.corps,
                eleve_id: Some(eleve_id),
                notification_id: None,
                source_type: "evenement",
                source_id: evt.event().id,
                cle_dedup: format!("evt:{}:{}", evt.event().id, m.kind),
                droit_requis: Some("peut_voir_vs"),
                regroupable: m.regroupable,
                canaux_voulus: &["email", "sms", "push"],
            },
            evt.event().id,
            Utc::now(),
        )
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
    .fetch_all(&mut *crate::tenant_tx::TenantTx::begin(pool, etablissement_id).await?)
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

// ---------------------------------------------------------------------------
// Évaluations (S2-7) : évaluation publiée, bulletin publié → familles et élève.
// ---------------------------------------------------------------------------

pub const CONSOMMATEUR_EVALUATIONS: &str = "d6.evaluations";

pub struct ConsommateurEvaluations;

const TYPES_EVALUATIONS: &[&str] = &["evaluation.publiee", types::NOTE_BULLETIN_PUBLIE];

/// Destinataires d'un élève relus au traitement : responsables au lien ACTIF avec
/// `peut_voir_notes` (compte parent membre) et le compte de l'élève lui-même.
async fn destinataires_notes(
    tx: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
) -> Result<Vec<Uuid>, String> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT l.user_id
          FROM ent_eleve_lien_sollicitable l
          JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
         WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.peut_voir_notes AND m.role = 'parent'
        UNION
        SELECT c.user_id
          FROM ent_eleve_compte c
          JOIN ent_membership m ON m.user_id = c.user_id AND m.etablissement_id = c.etablissement_id
         WHERE c.etablissement_id = $1 AND c.eleve_id = $2 AND m.role = 'eleve'
        "#,
    )
    .bind(etab)
    .bind(eleve_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(u,)| u).collect())
}

#[allow(clippy::too_many_arguments)]
async fn inserer_notification(
    tx: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
    eleve: Uuid,
    kind: &str,
    titre: &str,
    corps: &str,
    source: Uuid,
) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO ent_notification (
            id, etablissement_id, destinataire_user_id, eleve_id, kind, titre, corps, source_event_id
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
        ON CONFLICT (source_event_id, destinataire_user_id, kind) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(user)
    .bind(eleve)
    .bind(kind)
    .bind(titre)
    .bind(corps)
    .bind(source)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Aucune note ni appréciation recopiée : la famille consulte le portail (droits relus).
async fn notifier_evaluations(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let e = evt.event();
    match e.type_name.as_str() {
        types::NOTE_BULLETIN_PUBLIE => {
            let Some(eleve_id) = e.eleve_id else {
                return Ok(());
            };
            let Some(bulletin_id) = e
                .payload
                .get("bulletin_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
            else {
                return Ok(());
            };
            // Relu dans l'établissement de l'événement : bulletin publié de CET élève.
            let row: Option<(String, String)> = sqlx::query_as(
                r#"
                SELECT el.prenom, p.libelle
                  FROM ent_bulletin b
                  JOIN ent_eleve el ON el.id = b.eleve_id AND el.etablissement_id = b.etablissement_id
                  JOIN ent_note_periode p ON p.id = b.periode_id AND p.etablissement_id = b.etablissement_id
                 WHERE b.etablissement_id = $1 AND b.id = $2 AND b.eleve_id = $3 AND b.statut = 'publie'
                "#,
            )
            .bind(etab)
            .bind(bulletin_id)
            .bind(eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            let Some((prenom, periode)) = row else {
                return Ok(());
            };
            for user in destinataires_notes(tx, etab, eleve_id).await? {
                inserer_notification(
                    tx,
                    etab,
                    user,
                    eleve_id,
                    "bulletin_publie",
                    &format!("Bulletin de {prenom} disponible"),
                    &format!("Le bulletin « {periode} » de {prenom} est disponible dans l'espace évaluations."),
                    e.id,
                )
                .await?;
            }
        }
        "evaluation.publiee" => {
            let row: Option<(String, String)> = sqlx::query_as(
                r#"
                SELECT d.titre, m.libelle
                  FROM ent_devoir d
                  JOIN ent_matiere m ON m.id = d.matiere_id AND m.etablissement_id = d.etablissement_id
                 WHERE d.etablissement_id = $1 AND d.id = $2 AND d.publie_le IS NOT NULL
                "#,
            )
            .bind(etab)
            .bind(e.aggregate_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            let Some((titre, matiere)) = row else {
                return Ok(());
            };
            let eleves: Vec<(Uuid, String)> = sqlx::query_as(
                r#"
                SELECT DISTINCT n.eleve_id, el.prenom
                  FROM ent_note n
                  JOIN ent_eleve el ON el.id = n.eleve_id AND el.etablissement_id = n.etablissement_id
                 WHERE n.etablissement_id = $1 AND n.devoir_id = $2
                 ORDER BY el.prenom
                "#,
            )
            .bind(etab)
            .bind(e.aggregate_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            // Une notification par destinataire (frères et sœurs regroupés).
            let mut par_user: Vec<(Uuid, Uuid, Vec<String>)> = Vec::new();
            for (eleve_id, prenom) in eleves {
                for user in destinataires_notes(tx, etab, eleve_id).await? {
                    match par_user.iter_mut().find(|(u, _, _)| *u == user) {
                        Some((_, _, p)) => p.push(prenom.clone()),
                        None => par_user.push((user, eleve_id, vec![prenom.clone()])),
                    }
                }
            }
            for (user, eleve_id, prenoms) in par_user {
                inserer_notification(
                    tx,
                    etab,
                    user,
                    eleve_id,
                    "evaluation_publiee",
                    &format!("Nouvelle note en {matiere}"),
                    &format!(
                        "Les résultats de « {titre} » ({matiere}) sont disponibles pour {}.",
                        prenoms.join(", ")
                    ),
                    e.id,
                )
                .await?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl Consommateur for ConsommateurEvaluations {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_EVALUATIONS
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_EVALUATIONS
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(notifier_evaluations(tx, evt))
    }
}
