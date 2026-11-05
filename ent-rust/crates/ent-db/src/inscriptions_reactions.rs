//! Réactions à `inscription.validee` (socle 2, lot S2-3 ; toile §4.3).
//!
//! | Consommateur | Domaine | Effet | Rejouable |
//! |---|---|---|---|
//! | `d1.affectation_inscription` | référentiel | affectation de l'élève à la classe cible (année de la classe) ; élève rendu actif dans la classe de l'année courante | oui |
//! | `d2.invitations_responsables` | personnes | une invitation par responsable avec e-mail et sans compte rattaché (second parent compris, D-014) ; lien envoyé par e-mail | **non** (envoi externe) |
//! | `d8.accueil_inscription` | communication | message d'accueil : notification interne aux responsables déjà titulaires d'un compte, e-mail au déposant | **non** (envoi externe) |
//!
//! Chaque réaction s'exécute dans la transaction du distributeur (marque
//! d'idempotence `ent_event_traitement` + effet) et ne touche que
//! l'établissement de l'événement. Les données sont **relues** (dossier,
//! responsables, classe) : la charge utile ne sert qu'à les retrouver.
//! Idempotence renforcée par des clés uniques : affectation ouverte par
//! (élève, année), invitation par (dossier, responsable), notification par
//! (événement, destinataire, type).

use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};
use chrono::{Duration, Utc};
use ent_auth::generate_reset_token;
use ent_events::types;
use ent_mail::{MailSendStatus, MailService, OutgoingMail};
use sqlx::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

pub const CONSOMMATEUR_AFFECTATION: &str = "d1.affectation_inscription";
pub const CONSOMMATEUR_INVITATIONS: &str = "d2.invitations_responsables";
pub const CONSOMMATEUR_ACCUEIL: &str = "d8.accueil_inscription";

const TYPES: &[&str] = &[types::INSCRIPTION_VALIDEE];

/// Consommateurs du lot S2-3 (installés par `PgEnt::distributeur`).
pub fn consommateurs_inscriptions(mailer: MailService, dry_run_accepte: bool) -> Vec<Arc<dyn Consommateur>> {
    let envoi = Envoi { mailer, dry_run_accepte };
    vec![
        Arc::new(Affectation),
        Arc::new(Invitations { envoi: envoi.clone() }),
        Arc::new(Accueil { envoi }),
    ]
}

fn sql(e: sqlx::Error) -> String {
    e.to_string()
}

fn dossier_de(evt: &EvenementDistribue) -> Option<Uuid> {
    evt.event()
        .payload
        .get("dossier_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

#[derive(Clone)]
struct Envoi {
    mailer: MailService,
    dry_run_accepte: bool,
}

impl Envoi {
    /// `true` si le message est parti (dry-run accepté seulement en labo / test).
    async fn envoyer(&self, to: &str, subject: String, body_text: String) -> bool {
        if !self.mailer.is_ready() {
            return false;
        }
        match self
            .mailer
            .send(OutgoingMail {
                to: to.to_string(),
                subject,
                body_text,
                attachments: vec![],
            })
            .await
        {
            Ok(r) => match r.status {
                MailSendStatus::Sent => true,
                MailSendStatus::DryRun => self.dry_run_accepte,
                _ => false,
            },
            Err(_) => false,
        }
    }
}

fn base_publique() -> String {
    std::env::var("ENT_PUBLIC_BASE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "http://localhost:8088".into())
}

// ---------------------------------------------------------------------------
// D1 — affectation dans le référentiel
// ---------------------------------------------------------------------------

struct Affectation;

async fn affecter(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let p = &evt.event().payload;
    let (Some(eleve_id), Some(classe_id)) = (
        evt.event().eleve_id,
        p.get("classe_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()),
    ) else {
        return Ok(()); // pas de classe cible : affectation faite plus tard (REF-11)
    };
    // Classe et élève relus dans l'établissement de l'événement.
    let classe: Option<(Uuid, bool, chrono::NaiveDate)> = sqlx::query_as(
        "SELECT c.annee_id, a.courante, a.date_debut FROM ent_classe c
           JOIN ent_annee_scolaire a ON a.id = c.annee_id AND a.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.id = $2",
    )
    .bind(etab)
    .bind(classe_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(sql)?;
    let Some((annee_id, courante, debut_annee)) = classe else {
        return Ok(());
    };
    let eleve: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(sql)?;
    if eleve.is_none() {
        return Ok(());
    }
    let debut = std::cmp::max(debut_annee, Utc::now().date_naive());
    sqlx::query(
        "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut, source)
         VALUES ($1,$2,$3,$4,$5,$6,'saisie')
         ON CONFLICT (etablissement_id, eleve_id, annee_id)
            WHERE classe_id IS NOT NULL AND date_fin IS NULL DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(annee_id)
    .bind(eleve_id)
    .bind(classe_id)
    .bind(debut)
    .execute(&mut *tx)
    .await
    .map_err(sql)?;
    if courante {
        // Classe courante de l'élève (le déclencheur `classe_id` aligne le texte).
        sqlx::query(
            "UPDATE ent_eleve SET classe_id = $3, status = 'actif', updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(classe_id)
        .execute(&mut *tx)
        .await
        .map_err(sql)?;
    }
    Ok(())
}

impl Consommateur for Affectation {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_AFFECTATION
    }
    fn types(&self) -> &'static [&'static str] {
        TYPES
    }
    fn traiter<'a>(&'a self, tx: &'a mut PgConnection, evt: &'a EvenementDistribue) -> FutureTraitement<'a> {
        Box::pin(affecter(tx, evt))
    }
}

// ---------------------------------------------------------------------------
// D2 — invitations des responsables (second parent compris)
// ---------------------------------------------------------------------------

struct Invitations {
    envoi: Envoi,
}

async fn inviter(envoi: &Envoi, tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let Some(dossier_id) = dossier_de(evt) else {
        return Ok(());
    };
    // Responsables du dossier avec e-mail, dont la personne n'a pas de compte.
    let cibles: Vec<(Uuid, Uuid, String, String, String, i16)> = sqlx::query_as(
        "SELECT r.id, r.personne_id, r.email, d.eleve_prenom, d.eleve_nom, r.rang
           FROM ent_inscription_responsable r
           JOIN ent_inscription_dossier d ON d.id = r.dossier_id AND d.etablissement_id = r.etablissement_id
           JOIN ent_personne p ON p.id = r.personne_id AND p.etablissement_id = r.etablissement_id
          WHERE r.etablissement_id = $1 AND r.dossier_id = $2 AND d.statut = 'validee'
            AND r.email IS NOT NULL AND p.user_id IS NULL
          ORDER BY r.rang",
    )
    .bind(etab)
    .bind(dossier_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(sql)?;
    for (resp_id, personne_id, email, prenom, nom, rang) in cibles {
        let (raw, hashed) = generate_reset_token();
        let id = Uuid::new_v4();
        let ins = sqlx::query(
            "INSERT INTO ent_inscription_invitation
                (id, etablissement_id, dossier_id, responsable_id, personne_id, email, jeton_hash,
                 statut, expire_le, source_event_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,'envoyee',$8,$9)
             ON CONFLICT (dossier_id, responsable_id) DO NOTHING",
        )
        .bind(id)
        .bind(etab)
        .bind(dossier_id)
        .bind(resp_id)
        .bind(personne_id)
        .bind(&email)
        .bind(&hashed)
        .bind(Utc::now() + Duration::days(crate::inscriptions::INVITATION_TTL_JOURS))
        .bind(evt.event().id)
        .execute(&mut *tx)
        .await
        .map_err(sql)?;
        if ins.rows_affected() == 0 {
            continue; // déjà invité (rejeu) : aucun nouvel envoi
        }
        let qui = if rang == 1 { "votre" } else { "le" };
        let lien = format!(
            "{}/web/invitation?jeton={raw}",
            base_publique().trim_end_matches('/')
        );
        let parti = envoi
            .envoyer(
                &email,
                "Invitation à l'espace famille de l'établissement".into(),
                format!(
                    "Bonjour,\n\nL'inscription de {prenom} {nom} est validée. Vous êtes invité(e) à confirmer \
{qui} adresse et à créer votre compte (ou à rattacher votre compte existant) :\n{lien}\n\n\
Ce lien est personnel et valable {} jours.\n",
                    crate::inscriptions::INVITATION_TTL_JOURS
                ),
            )
            .await;
        if !parti {
            sqlx::query("UPDATE ent_inscription_invitation SET statut = 'non_envoyee' WHERE id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(sql)?;
        }
    }
    Ok(())
}

impl Consommateur for Invitations {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_INVITATIONS
    }
    fn types(&self) -> &'static [&'static str] {
        TYPES
    }
    fn rejouable(&self) -> bool {
        false
    }
    fn traiter<'a>(&'a self, tx: &'a mut PgConnection, evt: &'a EvenementDistribue) -> FutureTraitement<'a> {
        Box::pin(inviter(&self.envoi, tx, evt))
    }
}

// ---------------------------------------------------------------------------
// D8 — message d'accueil
// ---------------------------------------------------------------------------

struct Accueil {
    envoi: Envoi,
}

async fn accueillir(envoi: &Envoi, tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let (Some(dossier_id), Some(eleve_id)) = (dossier_de(evt), evt.event().eleve_id) else {
        return Ok(());
    };
    let d: Option<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT d.nature, d.eleve_prenom, c.annee_cible, d.deposant_email
           FROM ent_inscription_dossier d
           JOIN ent_inscription_campagne c ON c.id = d.campagne_id AND c.etablissement_id = d.etablissement_id
          WHERE d.etablissement_id = $1 AND d.id = $2 AND d.statut = 'validee' AND d.eleve_id = $3",
    )
    .bind(etab)
    .bind(dossier_id)
    .bind(eleve_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(sql)?;
    let Some((nature, prenom, annee, deposant)) = d else {
        return Ok(());
    };
    let (kind, titre, corps) = if nature == "reinscription" {
        (
            "reinscription_validee",
            format!("Réinscription de {prenom} validée"),
            format!("La réinscription de {prenom} pour l'année {annee} est validée."),
        )
    } else {
        (
            "inscription_validee",
            format!("Bienvenue à {prenom}"),
            format!(
                "L'inscription de {prenom} pour l'année {annee} est validée. Bienvenue dans l'établissement ! \
Les informations de rentrée vous seront communiquées dans votre espace."
            ),
        )
    };
    // Responsables déjà titulaires d'un compte (lien actif, profil parent).
    let destinataires: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT l.user_id FROM ent_eleve_lien_actif l
           JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND m.role = 'parent'",
    )
    .bind(etab)
    .bind(eleve_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(sql)?;
    for (user_id,) in destinataires {
        sqlx::query(
            "INSERT INTO ent_notification (id, etablissement_id, destinataire_user_id, eleve_id, kind, titre,
                                           corps, source_event_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT (source_event_id, destinataire_user_id, kind) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(user_id)
        .bind(eleve_id)
        .bind(kind)
        .bind(&titre)
        .bind(&corps)
        .bind(evt.event().id)
        .execute(&mut *tx)
        .await
        .map_err(sql)?;
    }
    if let Some(to) = deposant {
        if !envoi.envoyer(&to, titre, format!("Bonjour,\n\n{corps}\n")).await {
            tracing::warn!("accueil : message non envoyé (messagerie)");
        }
    }
    Ok(())
}

impl Consommateur for Accueil {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_ACCUEIL
    }
    fn types(&self) -> &'static [&'static str] {
        TYPES
    }
    fn rejouable(&self) -> bool {
        false
    }
    fn traiter<'a>(&'a self, tx: &'a mut PgConnection, evt: &'a EvenementDistribue) -> FutureTraitement<'a> {
        Box::pin(accueillir(&self.envoi, tx, evt))
    }
}
