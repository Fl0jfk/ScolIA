//! Réactions à `inscription.validee` (socle 2, lot S2-3 ; toile §4.3).
//!
//! | Consommateur | Domaine | Effet | Rejouable |
//! |---|---|---|---|
//! | `d1.affectation_inscription` | référentiel | affectation de l'élève à la classe cible (année de la classe) ; élève rendu actif dans la classe de l'année courante | oui |
//! | `d2.invitations_responsables` | personnes | une invitation par responsable avec e-mail et sans compte rattaché (second parent compris, D-014), et une invitation de **confirmation** envoyée au titulaire d'un compte existant dont le lien est en attente (I0) ; messages mis en file | **non** |
//! | `d8.accueil_inscription` | communication | message d'accueil : notification interne aux responsables déjà titulaires d'un compte, e-mail au déposant (mis en file) | **non** |
//!
//! **Aucun e-mail n'est envoyé dans la transaction du distributeur** (I0, §2.2 pt 17) : les
//! messages sont écrits dans `ent_inscription_envoi` avec le fait métier, puis expédiés après
//! validation par `expedier_envois` (reprise avec délai croissant, échecs tracés).
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
use ent_auth::hash_token;
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
pub fn consommateurs_inscriptions(
    mailer: MailService,
    dry_run_accepte: bool,
) -> Vec<Arc<dyn Consommateur>> {
    // L'envoi est fait par `expedier_envois` (après commit) : plus de messagerie ici.
    let _ = (mailer, dry_run_accepte);
    vec![
        Arc::new(Affectation),
        Arc::new(Invitations),
        Arc::new(Accueil),
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
    /// `Ok` si le message est parti (dry-run accepté seulement en labo / test).
    async fn envoyer(&self, to: &str, subject: String, body_text: String) -> Result<(), String> {
        if !self.mailer.is_ready() {
            return Err("messagerie non prête".into());
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
                MailSendStatus::Sent => Ok(()),
                MailSendStatus::DryRun if self.dry_run_accepte => Ok(()),
                other => Err(format!("envoi non abouti ({other:?})")),
            },
            Err(e) => Err(format!("erreur d'envoi : {e}")),
        }
    }
}

/// Jeton d'invitation : dérivé (HMAC) de l'identifiant sous le secret serveur, de sorte que
/// l'e-mail puisse être composé APRÈS validation de la transaction sans stocker le jeton
/// en clair (seule son empreinte SHA-256 est en base).
pub(crate) fn jeton_invitation(invitation_id: Uuid) -> Result<String, String> {
    let sb = ent_auth::SecretBox::from_env().map_err(|e| e.to_string())?;
    sb.hmac_hex("invitation-v1", invitation_id.as_bytes())
        .map_err(|e| e.to_string())
}

const ENVOI_ESSAIS_MAX: i32 = 5;

/// Expédie les messages en file dont l'échéance est atteinte. À appeler HORS de toute
/// transaction métier (tâche de fond, ou explicitement dans les tests). Un échec est
/// consigné (`derniere_erreur`) et repris avec un délai croissant ; au-delà de
/// [`ENVOI_ESSAIS_MAX`] essais le message passe en `echec` et l'invitation en `non_envoyee`.
pub async fn expedier_envois(
    pool: &sqlx::PgPool,
    mailer: &MailService,
    dry_run_accepte: bool,
    limite: i64,
) -> Result<usize, sqlx::Error> {
    let envoi = Envoi {
        mailer: mailer.clone(),
        dry_run_accepte,
    };
    let mut tx = pool.begin().await?;
    let lot: Vec<(Uuid, Uuid, Option<Uuid>, String, String, String, String, i32)> =
        sqlx::query_as(
            "SELECT id, etablissement_id, invitation_id, kind, destinataire, sujet, corps, tentatives
               FROM ent_inscription_envoi
              WHERE statut = 'a_envoyer' AND prochain_essai <= now()
                AND (invitation_id IS NULL OR EXISTS (
                      SELECT 1 FROM ent_inscription_invitation i
                       WHERE i.id = ent_inscription_envoi.invitation_id
                         AND i.etablissement_id = ent_inscription_envoi.etablissement_id
                         AND i.statut IN ('a_envoyer', 'non_envoyee', 'envoyee')
                         AND i.expire_le > now()))
              ORDER BY prochain_essai, created_at LIMIT $1
              FOR UPDATE SKIP LOCKED",
        )
        .bind(limite)
        .fetch_all(&mut *tx)
        .await?;
    let mut traites = 0;
    for (id, etab, invitation_id, kind, to, sujet, corps, tentatives) in lot {
        let corps = match (kind.as_str(), invitation_id) {
            ("invitation", Some(inv)) => jeton_invitation(inv).map(|j| {
                corps.replace(
                    "{LIEN}",
                    &format!(
                        "{}/web/invitation?jeton={j}",
                        base_publique().trim_end_matches('/')
                    ),
                )
            }),
            _ => Ok(corps),
        };
        // Un jeton non dérivable (secret absent) compte comme un échec : tentative et délai
        // avancent, la ligne ne bloque pas la tête de file.
        let res = match corps {
            Ok(corps) => envoi.envoyer(&to, sujet, corps).await,
            Err(e) => Err(format!("jeton d'invitation non dérivable : {e}")),
        };
        match res {
            Ok(()) => {
                sqlx::query(
                    "UPDATE ent_inscription_envoi SET statut = 'envoye', envoye_le = now(),
                            tentatives = tentatives + 1, derniere_erreur = NULL
                      WHERE id = $1 AND etablissement_id = $2",
                )
                .bind(id)
                .bind(etab)
                .execute(&mut *tx)
                .await?;
                if let Some(inv) = invitation_id {
                    sqlx::query(
                        "UPDATE ent_inscription_invitation SET statut = 'envoyee'
                          WHERE id = $1 AND etablissement_id = $2 AND statut IN ('a_envoyer', 'non_envoyee')",
                    )
                    .bind(inv)
                    .bind(etab)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            Err(e) => {
                let n = tentatives + 1;
                let definitif = n >= ENVOI_ESSAIS_MAX;
                tracing::warn!(envoi = %id, tentative = n, "envoi inscription en échec : {e}");
                sqlx::query(
                    "UPDATE ent_inscription_envoi
                        SET tentatives = $2, derniere_erreur = left($3, 500),
                            statut = CASE WHEN $4 THEN 'echec' ELSE statut END,
                            prochain_essai = now() + make_interval(mins => $5::int)
                      WHERE id = $1 AND etablissement_id = $6",
                )
                .bind(id)
                .bind(n)
                .bind(&e)
                .bind(definitif)
                .bind(1_i32 << n.min(8))
                .bind(etab)
                .execute(&mut *tx)
                .await?;
                if definitif {
                    if let Some(inv) = invitation_id {
                        sqlx::query(
                            "UPDATE ent_inscription_invitation SET statut = 'non_envoyee'
                              WHERE id = $1 AND etablissement_id = $2 AND statut = 'a_envoyer'",
                        )
                        .bind(inv)
                        .bind(etab)
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
        }
        traites += 1;
    }
    tx.commit().await?;
    Ok(traites)
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
        p.get("classe_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok()),
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
    let eleve: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
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
    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(affecter(tx, evt))
    }
}

// ---------------------------------------------------------------------------
// D2 — invitations des responsables (second parent compris)
// ---------------------------------------------------------------------------

struct Invitations;

async fn inviter(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let Some(dossier_id) = dossier_de(evt) else {
        return Ok(());
    };
    // Responsables à inviter : personne sans compte (création / rattachement), ou lien en
    // attente vers une personne qui a déjà un compte (confirmation : le message va à
    // l'adresse du COMPTE, jamais à celle saisie dans le dossier).
    let cibles: Vec<(Uuid, Uuid, String, String, String, i16, bool)> = sqlx::query_as(
        "SELECT r.id, r.personne_id, COALESCE(u.email, p.email, r.email), d.eleve_prenom, d.eleve_nom, r.rang,
                (la.id IS NOT NULL) AS confirmation
           FROM ent_inscription_responsable r
           JOIN ent_inscription_dossier d ON d.id = r.dossier_id AND d.etablissement_id = r.etablissement_id
           JOIN ent_personne p ON p.id = r.personne_id AND p.etablissement_id = r.etablissement_id
           LEFT JOIN ent_user u ON u.id = p.user_id
           LEFT JOIN ent_lien_attente la
                  ON la.etablissement_id = r.etablissement_id AND la.source = 'inscription'
                 AND la.source_ref = r.id AND la.statut = 'en_attente'
          WHERE r.etablissement_id = $1 AND r.dossier_id = $2 AND d.statut = 'validee'
            AND ((r.email IS NOT NULL AND p.user_id IS NULL) OR la.id IS NOT NULL)
            -- A0-c : AUCUNE invitation ni demande d'accord vers une personne « ne pas solliciter »
            -- (déclarée sur le dossier ou posée sur la fiche) : l'invitation révélerait l'élève
            -- et l'établissement à la personne dont la famille est protégée.
            AND NOT r.ne_pas_solliciter
            AND NOT EXISTS (SELECT 1 FROM ent_personne_reserve nps
                             WHERE nps.etablissement_id = r.etablissement_id
                               AND nps.personne_id = r.personne_id AND nps.ne_pas_solliciter)
          ORDER BY r.rang",
    )
    .bind(etab)
    .bind(dossier_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(sql)?;
    for (resp_id, personne_id, email, prenom, nom, rang, confirmation) in cibles {
        let id = Uuid::new_v4();
        let hashed = hash_token(&jeton_invitation(id)?);
        let ins = sqlx::query(
            "INSERT INTO ent_inscription_invitation
                (id, etablissement_id, dossier_id, responsable_id, personne_id, email, jeton_hash,
                 statut, expire_le, source_event_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,'a_envoyer',$8,$9)
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
            continue; // déjà invité (rejeu) : aucun nouveau message
        }
        let qui = if rang == 1 { "votre" } else { "le" };
        let corps = if confirmation {
            format!(
                "Bonjour,\n\nL'établissement a enregistré l'inscription de {prenom} {nom} et vous indique comme \
responsable. Aucun accès n'est ouvert tant que vous n'avez pas confirmé. Si vous êtes bien responsable de cet \
enfant, confirmez ici :\n{{LIEN}}\n\nSi ce n'est pas le cas, ignorez ce message ou refusez depuis la même page.\n\
Ce lien est personnel et valable {} jours.\n",
                crate::inscriptions::INVITATION_TTL_JOURS
            )
        } else {
            format!(
                "Bonjour,\n\nL'inscription de {prenom} {nom} est validée. Vous êtes invité(e) à confirmer \
{qui} adresse et à créer votre compte (ou à rattacher votre compte existant) :\n{{LIEN}}\n\n\
Ce lien est personnel et valable {} jours.\n",
                crate::inscriptions::INVITATION_TTL_JOURS
            )
        };
        sqlx::query(
            "INSERT INTO ent_inscription_envoi
                (id, etablissement_id, dossier_id, invitation_id, cle, kind, destinataire, sujet, corps)
             VALUES ($1,$2,$3,$4,$5,'invitation',$6,$7,$8)
             ON CONFLICT (etablissement_id, cle) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(dossier_id)
        .bind(id)
        .bind(format!("invitation:{id}"))
        .bind(&email)
        .bind(if confirmation {
            "Confirmez votre lien avec un élève"
        } else {
            "Invitation à l'espace famille de l'établissement"
        })
        .bind(corps)
        .execute(&mut *tx)
        .await
        .map_err(sql)?;
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
    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(inviter(tx, evt))
    }
}

// ---------------------------------------------------------------------------
// D8 — message d'accueil
// ---------------------------------------------------------------------------

struct Accueil;

async fn accueillir(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
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
        sqlx::query(
            "INSERT INTO ent_inscription_envoi
                (id, etablissement_id, dossier_id, cle, kind, destinataire, sujet, corps)
             VALUES ($1,$2,$3,$4,'accueil',$5,$6,$7)
             ON CONFLICT (etablissement_id, cle) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(dossier_id)
        .bind(format!("accueil:{}", evt.event().id))
        .bind(&to)
        .bind(&titre)
        .bind(format!("Bonjour,\n\n{corps}\n"))
        .execute(&mut *tx)
        .await
        .map_err(sql)?;
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
    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(accueillir(tx, evt))
    }
}
