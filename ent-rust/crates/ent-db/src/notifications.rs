//! Domaine Notifications unifié (socle 2, lot S2-8, D0 / PLT-05).
//!
//! **Chaîne.** Un fait métier (événement de l'outbox, S2-0) ou une action
//! explicite (campagne, rappel de rendez-vous) produit :
//!
//! 1. une notification **dans l'ENT** (`ent_notification`, toujours, non
//!    désactivable) ;
//! 2. des **envois** par canal e-mail / SMS / push (`ent_notification_envoi`),
//!    planifiés selon les **préférences** de la personne : canal choisi, résumé
//!    quotidien (regroupement), heures calmes (heure de Paris).
//!
//! Les envois sont ensuite expédiés par l'[`Expediteur`] (boucle tokio ou appel
//! direct en test). L'effet externe est **hors** de la transaction du
//! consommateur d'événements : un consommateur reste rejouable, et un rejeu ne
//! renvoie rien (clé de dédoublonnage unique par destinataire et canal).
//!
//! **Garanties.**
//! - *Idempotence* : `UNIQUE (établissement, destinataire, canal, cle_dedup)` ;
//!   un envoi est « réclamé » (`en_attente` → `envoi`) par `FOR UPDATE SKIP LOCKED`,
//!   donc deux expéditeurs ne traitent jamais la même ligne. Un plantage entre
//!   la réclamation et le résultat laisse la ligne en `envoi` : elle passe en
//!   `echec` (motif `interrompu`) après un délai, jamais renvoyée automatiquement
//!   (au plus une fois : un SMS a un coût et ne se rattrape pas).
//! - *Droits relus à l'envoi* : un lien suspendu entre-temps annule l'envoi.
//! - *Plafonds SMS* (par personne et par jour, par établissement et par jour) :
//!   emplacement réservé sous verrou avant l'appel au fournisseur ; au plus
//!   les plafonds, jamais au-delà, même avec plusieurs expéditeurs.
//! - *SMS* : désactivé par défaut (consentement explicite), jamais regroupé.

use crate::assiduite::heure_paris;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};
use chrono::{DateTime, Duration, NaiveDateTime, NaiveTime, Utc};
use ent_auth::Action;
use ent_mail::{MailSendStatus, MailService, OutgoingMail};
use ent_push::{OutgoingPush, PushPlatform, PushSendStatus, PushService};
use ent_sms::{OutgoingSms, SmsSendStatus, SmsService};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

pub(crate) fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("notifications", err))
}

pub const CANAUX: [&str; 3] = ["email", "sms", "push"];

/// Heure d'envoi des résumés quotidiens (heure de Paris), en minutes.
pub const HEURE_RESUME_MIN: u16 = 7 * 60 + 30;

/// Longueur maximale d'un SMS composé (3 segments de 7 bits).
const SMS_MAX: usize = 306;

// ---------------------------------------------------------------------------
// Préférences et plafonds (valeurs et règles pures)
// ---------------------------------------------------------------------------

/// Préférences de notification d'une personne dans un établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub email: bool,
    pub sms: bool,
    pub push: bool,
    /// `immediat` ou `quotidien` (résumé unique, e-mail et push seulement).
    pub resume: String,
    /// Heures calmes, en minutes depuis minuit (heure de Paris).
    pub calme_debut: Option<u16>,
    pub calme_fin: Option<u16>,
}

impl Default for Preferences {
    /// Défauts : e-mail et push actifs, **SMS désactivé** (consentement explicite).
    fn default() -> Self {
        Self {
            email: true,
            sms: false,
            push: true,
            resume: "immediat".into(),
            calme_debut: None,
            calme_fin: None,
        }
    }
}

impl Preferences {
    pub fn valider(&self) -> Result<(), EntDbError> {
        if !matches!(self.resume.as_str(), "immediat" | "quotidien") {
            return Err(EntDbError::SaisieInvalide("mode de résumé inconnu"));
        }
        match (self.calme_debut, self.calme_fin) {
            (None, None) => {}
            (Some(d), Some(f)) if d < 1440 && f < 1440 && d != f => {}
            _ => return Err(EntDbError::SaisieInvalide("heures calmes invalides")),
        }
        Ok(())
    }

    pub fn canal_actif(&self, canal: &str) -> bool {
        match canal {
            "email" => self.email,
            "sms" => self.sms,
            "push" => self.push,
            _ => false,
        }
    }
}

/// Plafonds anti-abus d'un établissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plafonds {
    pub sms_par_jour: i32,
    pub sms_par_personne_jour: i32,
    pub sms_par_campagne: i32,
    pub campagnes_par_jour: i32,
}

impl Default for Plafonds {
    fn default() -> Self {
        Self {
            sms_par_jour: 200,
            sms_par_personne_jour: 3,
            sms_par_campagne: 300,
            campagnes_par_jour: 5,
        }
    }
}

/// `minute` (0..1440) est-elle dans la plage `[debut, fin)` (qui peut passer minuit) ?
pub fn dans_plage(minute: u16, debut: u16, fin: u16) -> bool {
    if debut < fin {
        debut <= minute && minute < fin
    } else {
        minute >= debut || minute < fin
    }
}

fn minute_du_jour(t: NaiveDateTime) -> u16 {
    use chrono::Timelike;
    (t.hour() * 60 + t.minute()) as u16
}

fn a_minute(date: chrono::NaiveDate, minute: u16) -> NaiveDateTime {
    date.and_time(
        NaiveTime::from_hms_opt(u32::from(minute / 60), u32::from(minute % 60), 0)
            .expect("minute valide"),
    )
}

/// Instant local (heure de Paris) auquel un envoi doit partir : résumé quotidien
/// pour un message regroupable, puis report hors des heures calmes.
pub fn echeance_locale(
    pref: &Preferences,
    canal: &str,
    regroupable: bool,
    local: NaiveDateTime,
) -> NaiveDateTime {
    let mut t = local;
    if regroupable && pref.resume == "quotidien" && canal != "sms" {
        let cible = a_minute(local.date(), HEURE_RESUME_MIN);
        t = if local < cible {
            cible
        } else {
            a_minute(local.date() + Duration::days(1), HEURE_RESUME_MIN)
        };
    }
    if let (Some(d), Some(f)) = (pref.calme_debut, pref.calme_fin) {
        if dans_plage(minute_du_jour(t), d, f) {
            let mut fin = a_minute(t.date(), f);
            if fin <= t {
                fin = a_minute(t.date() + Duration::days(1), f);
            }
            t = fin;
        }
    }
    t
}

/// Canaux réellement utilisables : voulus, choisis par la personne et joignables.
pub fn canaux_disponibles(
    pref: &Preferences,
    voulus: &[&str],
    a_email: bool,
    a_telephone: bool,
    a_push: bool,
) -> Vec<&'static str> {
    CANAUX
        .into_iter()
        .filter(|c| voulus.contains(c) && pref.canal_actif(c))
        .filter(|c| match *c {
            "email" => a_email,
            "sms" => a_telephone,
            "push" => a_push,
            _ => false,
        })
        .collect()
}

fn sms_texte(titre: &str, corps: &str) -> String {
    let t = format!("{titre} : {corps}");
    if t.chars().count() > SMS_MAX {
        t.chars().take(SMS_MAX - 1).collect::<String>() + "…"
    } else {
        t
    }
}

// ---------------------------------------------------------------------------
// Lecture / écriture des préférences et des plafonds
// ---------------------------------------------------------------------------

type LignePref = (bool, bool, bool, String, Option<i16>, Option<i16>);

fn pref_depuis(l: LignePref) -> Preferences {
    Preferences {
        email: l.0,
        sms: l.1,
        push: l.2,
        resume: l.3,
        calme_debut: l.4.map(|v| v as u16),
        calme_fin: l.5.map(|v| v as u16),
    }
}

pub(crate) async fn preferences_de(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
) -> Result<Preferences, EntDbError> {
    let l: Option<LignePref> = sqlx::query_as(
        "SELECT canal_email, canal_sms, canal_push, resume, calme_debut, calme_fin
           FROM ent_preference_notification
          WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(l.map(pref_depuis).unwrap_or_default())
}

pub(crate) async fn plafonds_de(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<Plafonds, EntDbError> {
    let l: Option<(i32, i32, i32, i32)> = sqlx::query_as(
        "SELECT sms_par_jour, sms_par_personne_jour, sms_par_campagne, campagnes_par_jour
           FROM ent_notification_plafond WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(l.map(|l| Plafonds {
        sms_par_jour: l.0,
        sms_par_personne_jour: l.1,
        sms_par_campagne: l.2,
        campagnes_par_jour: l.3,
    })
    .unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Planification des envois
// ---------------------------------------------------------------------------

/// Un message à distribuer à une personne (tous canaux confondus).
pub(crate) struct EnvoiAPlanifier<'a> {
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub kind: &'a str,
    pub titre: &'a str,
    pub corps: &'a str,
    pub eleve_id: Option<Uuid>,
    pub notification_id: Option<Uuid>,
    pub source_type: &'static str,
    pub source_id: Uuid,
    pub cle_dedup: String,
    pub droit_requis: Option<&'static str>,
    pub regroupable: bool,
    /// Canaux voulus par l'émetteur (campagne : sa sélection ; événement : tous).
    pub canaux_voulus: &'a [&'a str],
}

/// Contacts d'une personne dans un établissement (membre uniquement).
pub(crate) struct Contacts {
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub push: bool,
}

pub(crate) async fn contacts_de(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
) -> Result<Option<Contacts>, EntDbError> {
    let l: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT u.email, u.telephone
           FROM ent_user u
           JOIN ent_membership m ON m.user_id = u.id AND m.etablissement_id = $1
          WHERE u.id = $2",
    )
    .bind(etab)
    .bind(user)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((email, tel)) = l else {
        return Ok(None);
    };
    let push: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM ent_push_device
          WHERE etablissement_id = $1 AND user_id = $2 AND revoked_at IS NULL LIMIT 1",
    )
    .bind(etab)
    .bind(user)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(Some(Contacts {
        email: Some(email).filter(|e| !e.trim().is_empty()),
        telephone: tel.filter(|t| !ent_sms::normalize_phone(t).is_empty()),
        push: push.is_some(),
    }))
}

/// Planifie les envois d'un message selon les préférences de la personne.
/// Retourne le nombre d'envois nouvellement créés (0 si déjà planifiés).
pub(crate) async fn planifier_envois(
    conn: &mut PgConnection,
    e: &EnvoiAPlanifier<'_>,
    maintenant: DateTime<Utc>,
) -> Result<usize, EntDbError> {
    let Some(contacts) = contacts_de(conn, e.etablissement_id, e.user_id).await? else {
        return Ok(0);
    };
    let pref = preferences_de(conn, e.etablissement_id, e.user_id).await?;
    let canaux = canaux_disponibles(
        &pref,
        e.canaux_voulus,
        contacts.email.is_some(),
        contacts.telephone.is_some(),
        contacts.push,
    );
    let local = heure_paris(maintenant);
    let mut crees = 0usize;
    for canal in canaux {
        let regroupable = e.regroupable && canal != "sms";
        let cible = echeance_locale(&pref, canal, regroupable, local);
        let r = sqlx::query(
            "INSERT INTO ent_notification_envoi
                (id, etablissement_id, destinataire_user_id, canal, kind, titre, corps, eleve_id,
                 notification_id, source_type, source_id, cle_dedup, droit_requis, regroupable,
                 planifie_pour)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,
                     CASE WHEN $15::timestamp = $16::timestamp THEN $17::timestamptz
                          ELSE $15::timestamp AT TIME ZONE 'Europe/Paris' END)
             ON CONFLICT (etablissement_id, destinataire_user_id, canal, cle_dedup) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(e.etablissement_id)
        .bind(e.user_id)
        .bind(canal)
        .bind(e.kind)
        .bind(e.titre)
        .bind(e.corps)
        .bind(e.eleve_id)
        .bind(e.notification_id)
        .bind(e.source_type)
        .bind(e.source_id)
        .bind(&e.cle_dedup)
        .bind(e.droit_requis)
        .bind(regroupable)
        .bind(cible)
        .bind(local)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        crees += r.rows_affected() as usize;
    }
    Ok(crees)
}

/// Notification complète d'une personne : dans l'ENT (toujours, idempotente sur
/// `source_event_id`) puis envois externes planifiés selon ses préférences.
pub(crate) async fn notifier_personne(
    conn: &mut PgConnection,
    mut e: EnvoiAPlanifier<'_>,
    source_event_id: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<usize, EntDbError> {
    let notif: Option<(Uuid,)> = sqlx::query_as(
        "INSERT INTO ent_notification
            (id, etablissement_id, destinataire_user_id, eleve_id, kind, titre, corps,
             source_event_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT (source_event_id, destinataire_user_id, kind) DO NOTHING
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(e.etablissement_id)
    .bind(e.user_id)
    .bind(e.eleve_id)
    .bind(e.kind)
    .bind(e.titre)
    .bind(e.corps)
    .bind(source_event_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    e.notification_id = notif.map(|n| n.0);
    planifier_envois(conn, &e, maintenant).await
}

// ---------------------------------------------------------------------------
// API de service (préférences, plafonds, traçabilité)
// ---------------------------------------------------------------------------

/// Envoi tel que vu par la personne ou l'administrateur (sans adresse ni numéro).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct EnvoiVu {
    pub id: Uuid,
    pub canal: String,
    pub kind: String,
    pub titre: String,
    pub statut: String,
    pub motif: Option<String>,
    pub source_type: String,
    pub source_id: Uuid,
    pub planifie_pour: DateTime<Utc>,
    pub traite_le: Option<DateTime<Utc>>,
}

const COLONNES_ENVOI_VU: &str =
    "id, canal, kind, titre, statut, motif, source_type, source_id, planifie_pour, traite_le";

/// Communications de masse (annonce, campagne) : SMS et push sans contenu (M0a, D4) ; le texte
/// complet reste dans l'ENT et dans l'e-mail.
fn canal_externe_neutre(kind: &str) -> bool {
    matches!(kind, "campagne" | "annonce")
}
const TITRE_NEUTRE_PUSH: &str = "Message de l'établissement";
const TEXTE_NEUTRE_PUSH: &str = "Nouveau message — ouvrez ScolIA pour le lire.";
const TEXTE_NEUTRE_SMS: &str = "ScolIA : nouveau message de l'établissement. Connectez-vous pour le lire.";

impl PgEnt {
    /// Préférences de la personne connectée (défauts si jamais enregistrées).
    pub async fn mes_preferences_notification(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Preferences, EntDbError> {
        let mut c = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        preferences_de(&mut c, etablissement_id, user_id).await
    }

    /// Enregistre les préférences de la personne connectée (jamais celles d'une autre).
    pub async fn definir_preferences_notification(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        pref: Preferences,
    ) -> Result<Preferences, EntDbError> {
        pref.valider()?;
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        // La personne doit être membre de l'établissement de sa session.
        let membre: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etablissement_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if membre.is_none() {
            return Err(EntDbError::Forbidden);
        }
        sqlx::query(
            "INSERT INTO ent_preference_notification
                (etablissement_id, user_id, canal_email, canal_sms, canal_push, resume,
                 calme_debut, calme_fin, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())
             ON CONFLICT (etablissement_id, user_id) DO UPDATE SET
                canal_email = EXCLUDED.canal_email, canal_sms = EXCLUDED.canal_sms,
                canal_push = EXCLUDED.canal_push, resume = EXCLUDED.resume,
                calme_debut = EXCLUDED.calme_debut, calme_fin = EXCLUDED.calme_fin,
                updated_at = now()",
        )
        .bind(etablissement_id)
        .bind(user_id)
        .bind(pref.email)
        .bind(pref.sms)
        .bind(pref.push)
        .bind(&pref.resume)
        .bind(pref.calme_debut.map(|v| v as i16))
        .bind(pref.calme_fin.map(|v| v as i16))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(pref)
    }

    /// Mes envois récents (traçabilité côté personne : sans adresse ni numéro).
    pub async fn mes_envois_notification(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        limite: i64,
    ) -> Result<Vec<EnvoiVu>, EntDbError> {
        sqlx::query_as(&format!(
            "SELECT {COLONNES_ENVOI_VU} FROM ent_notification_envoi
              WHERE etablissement_id = $1 AND destinataire_user_id = $2
              ORDER BY created_at DESC LIMIT $3"
        ))
        .bind(etablissement_id)
        .bind(user_id)
        .bind(limite.clamp(1, 200))
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await
        .map_err(db)
    }

    /// Plafonds de l'établissement (lecture : `notifications.gerer`).
    pub async fn plafonds_notification_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Plafonds, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::NotificationsGerer)
            .await?;
        let mut c = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        plafonds_de(&mut c, etablissement_id).await
    }

    pub async fn definir_plafonds_notification(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        p: Plafonds,
    ) -> Result<Plafonds, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::NotificationsGerer)
            .await?;
        if [p.sms_par_jour, p.sms_par_personne_jour, p.sms_par_campagne, p.campagnes_par_jour]
            .iter()
            .any(|v| !(0..=100_000).contains(v))
        {
            return Err(EntDbError::SaisieInvalide("plafond hors limites"));
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query(
            "INSERT INTO ent_notification_plafond
                (etablissement_id, sms_par_jour, sms_par_personne_jour, sms_par_campagne,
                 campagnes_par_jour, updated_by, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6, now())
             ON CONFLICT (etablissement_id) DO UPDATE SET
                sms_par_jour = EXCLUDED.sms_par_jour,
                sms_par_personne_jour = EXCLUDED.sms_par_personne_jour,
                sms_par_campagne = EXCLUDED.sms_par_campagne,
                campagnes_par_jour = EXCLUDED.campagnes_par_jour,
                updated_by = EXCLUDED.updated_by, updated_at = now()",
        )
        .bind(etablissement_id)
        .bind(p.sms_par_jour)
        .bind(p.sms_par_personne_jour)
        .bind(p.sms_par_campagne)
        .bind(p.campagnes_par_jour)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(p)
    }

    /// Envois d'une source (campagne, rendez-vous…) — traçabilité pour l'émetteur
    /// ou pour `notifications.gerer`. Sans adresse ni numéro.
    pub async fn envois_de_source_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<Vec<EnvoiVu>, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::NotificationsGerer)
            .await?;
        sqlx::query_as(&format!(
            "SELECT {COLONNES_ENVOI_VU} FROM ent_notification_envoi
              WHERE etablissement_id = $1 AND source_type = $2 AND source_id = $3
              ORDER BY created_at LIMIT 1000"
        ))
        .bind(etablissement_id)
        .bind(source_type)
        .bind(source_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await
        .map_err(db)
    }

    /// Synthèse par statut des envois d'une source (campagne).
    pub async fn bilan_envois_source(
        &self,
        etablissement_id: Uuid,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<Vec<(String, String, i64)>, EntDbError> {
        sqlx::query_as(
            "SELECT canal, statut, count(*) FROM ent_notification_envoi
              WHERE etablissement_id = $1 AND source_type = $2 AND source_id = $3
              GROUP BY canal, statut ORDER BY canal, statut",
        )
        .bind(etablissement_id)
        .bind(source_type)
        .bind(source_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await?)
        .await
        .map_err(db)
    }

    /// Expéditeur des envois planifiés, branché sur les services de l'ENT
    /// (e-mail, SMS, push : simulés par défaut, réels seulement si configurés).
    pub fn expediteur(&self, config: ConfigExpedition) -> Expediteur {
        Expediteur::nouveau(
            self.pool().clone(),
            self.mailer().clone(),
            self.sms().clone(),
            self.push().clone(),
            config,
        )
    }
}

// ---------------------------------------------------------------------------
// Expéditeur
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ConfigExpedition {
    pub taille_lot: i64,
    /// Une ligne `envoi` plus ancienne que ce délai est déclarée interrompue.
    pub perime_apres: std::time::Duration,
    pub intervalle: std::time::Duration,
    /// Restreint l'expédition à un établissement (tests, rattrapage ciblé).
    pub etablissement: Option<Uuid>,
}

impl Default for ConfigExpedition {
    fn default() -> Self {
        Self {
            taille_lot: 100,
            perime_apres: std::time::Duration::from_secs(15 * 60),
            intervalle: std::time::Duration::from_secs(5),
            etablissement: None,
        }
    }
}

/// Résultat d'un passage de l'expéditeur.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BilanExpedition {
    pub reclames: usize,
    pub envoyes: usize,
    pub simules: usize,
    pub echecs: usize,
    pub ignores: usize,
    pub plafonnes: usize,
    pub regroupes: usize,
    pub interrompus: usize,
}

pub struct Expediteur {
    pool: PgPool,
    mailer: MailService,
    sms: SmsService,
    push: PushService,
    config: ConfigExpedition,
    arret: AtomicBool,
}

#[derive(Debug, sqlx::FromRow, Clone)]
struct EnvoiReclame {
    id: Uuid,
    etablissement_id: Uuid,
    destinataire_user_id: Uuid,
    canal: String,
    kind: String,
    titre: String,
    corps: String,
    eleve_id: Option<Uuid>,
    droit_requis: Option<String>,
    regroupable: bool,
}

#[derive(Clone)]
enum Issue {
    Envoye(Option<String>),
    Simule(Option<String>),
    Echec(String),
    Ignore(&'static str),
    Plafonne(&'static str),
}

impl Expediteur {
    pub fn nouveau(
        pool: PgPool,
        mailer: MailService,
        sms: SmsService,
        push: PushService,
        config: ConfigExpedition,
    ) -> Self {
        Self {
            pool,
            mailer,
            sms,
            push,
            config,
            arret: AtomicBool::new(false),
        }
    }

    pub fn arreter(&self) {
        self.arret.store(true, Ordering::SeqCst);
    }

    /// Boucle d'expédition en tâche tokio.
    pub fn lancer(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tracing::info!("expéditeur de notifications démarré");
            while !self.arret.load(Ordering::SeqCst) {
                match self.expedier(Utc::now()).await {
                    Ok(b) if b.reclames as i64 >= self.config.taille_lot => continue,
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "expéditeur : passage en échec"),
                }
                tokio::time::sleep(self.config.intervalle).await;
            }
        })
    }

    /// Un passage : récupère les envois échus et les expédie.
    ///
    /// Worker transverse nommé (`ContexteSysteme::WorkerEnvoi`, docs/securite/RLS_STRICTE.md §4) :
    /// un passage **par établissement**, chacun dans son contexte (RLS stricte) ; un seul
    /// établissement si la configuration en désigne un.
    pub async fn expedier(&self, maintenant: DateTime<Utc>) -> Result<BilanExpedition, EntDbError> {
        let etabs = match self.config.etablissement {
            Some(e) => vec![e],
            None => crate::tenant_tx::etablissements_systeme(&self.pool, crate::tenant_tx::ContexteSysteme::WorkerEnvoi)
                .await
                .map_err(db)?,
        };
        let mut total = BilanExpedition::default();
        for etab in etabs {
            let b = self.expedier_etab(etab, maintenant).await?;
            total.reclames += b.reclames;
            total.envoyes += b.envoyes;
            total.simules += b.simules;
            total.echecs += b.echecs;
            total.ignores += b.ignores;
            total.plafonnes += b.plafonnes;
            total.regroupes += b.regroupes;
            total.interrompus += b.interrompus;
        }
        Ok(total)
    }

    async fn expedier_etab(&self, etab: Uuid, maintenant: DateTime<Utc>) -> Result<BilanExpedition, EntDbError> {
        let mut bilan = BilanExpedition::default();
        // 1. Lignes abandonnées en cours d'envoi : échec (jamais renvoyées).
        let mut tx = TenantTx::begin(&self.pool, etab).await.map_err(db)?;
        let perimes = sqlx::query(
            "UPDATE ent_notification_envoi
                SET statut = 'echec', motif = 'interrompu', traite_le = $1
              WHERE statut = 'envoi' AND tentative_le < $2
                AND ($3::uuid IS NULL OR etablissement_id = $3)",
        )
        .bind(maintenant)
        .bind(
            maintenant
                - Duration::from_std(self.config.perime_apres).unwrap_or(Duration::minutes(15)),
        )
        .bind(Some(etab))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        bilan.interrompus = perimes.rows_affected() as usize;

        // 2. Réclamation atomique des envois échus.
        let mut claim = TenantTx::begin(&self.pool, etab).await.map_err(db)?;
        let lot: Vec<EnvoiReclame> = sqlx::query_as(
            "UPDATE ent_notification_envoi e
                SET statut = 'envoi', tentative_le = $1
              WHERE e.id IN (
                    SELECT id FROM ent_notification_envoi
                     WHERE statut = 'en_attente' AND planifie_pour <= $1
                       AND ($2::uuid IS NULL OR etablissement_id = $2)
                     ORDER BY planifie_pour, created_at
                     LIMIT $3
                     FOR UPDATE SKIP LOCKED)
              RETURNING e.id, e.etablissement_id, e.destinataire_user_id, e.canal, e.kind,
                        e.titre, e.corps, e.eleve_id,
                        e.droit_requis, e.regroupable",
        )
        .bind(maintenant)
        .bind(Some(etab))
        .bind(self.config.taille_lot)
        .fetch_all(&mut *claim)
        .await
        .map_err(db)?;
        claim.commit().await.map_err(db)?;
        bilan.reclames = lot.len();

        // 3. Droits relus, puis regroupement en résumé ou envoi individuel.
        let mut a_envoyer: Vec<EnvoiReclame> = Vec::new();
        for e in lot {
            if !self.droit_toujours_valide(&e).await? {
                self.finaliser(&e, Issue::Ignore("droit_retire"), None, &mut bilan, maintenant)
                    .await?;
                continue;
            }
            a_envoyer.push(e);
        }
        let mut individuels: Vec<EnvoiReclame> = Vec::new();
        let mut groupes: std::collections::BTreeMap<(Uuid, Uuid, String), Vec<EnvoiReclame>> =
            Default::default();
        for e in a_envoyer {
            let resume = e.regroupable && e.canal != "sms" && {
                let mut c = TenantTx::begin(&self.pool, e.etablissement_id).await.map_err(db)?;
                preferences_de(&mut c, e.etablissement_id, e.destinataire_user_id)
                    .await?
                    .resume
                    == "quotidien"
            };
            if resume {
                groupes
                    .entry((e.etablissement_id, e.destinataire_user_id, e.canal.clone()))
                    .or_default()
                    .push(e);
            } else {
                individuels.push(e);
            }
        }
        for e in individuels {
            let issue = self.expedier_un(&e, &e.titre, &e.corps, maintenant).await?;
            self.finaliser(&e, issue, None, &mut bilan, maintenant).await?;
        }
        for ((_, _, _), membres) in groupes {
            let premier = membres[0].clone();
            let n = membres.len();
            let (titre, corps) = if n == 1 {
                (premier.titre.clone(), premier.corps.clone())
            } else {
                (
                    format!("Résumé de vos notifications ({n})"),
                    membres
                        .iter()
                        .map(|m| format!("- {}", m.titre))
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            };
            let issue = self.expedier_un(&premier, &titre, &corps, maintenant).await?;
            let reussi = matches!(issue, Issue::Envoye(_) | Issue::Simule(_));
            // Un seul envoi réel pour tout le groupe : les membres sont « regroupés ».
            let resume_id = (reussi && n > 1).then(Uuid::new_v4);
            for e in &membres {
                self.finaliser(e, issue.clone(), resume_id, &mut bilan, maintenant)
                    .await?;
            }
        }
        Ok(bilan)
    }

    async fn droit_toujours_valide(&self, e: &EnvoiReclame) -> Result<bool, EntDbError> {
        let mut tx = TenantTx::begin(&self.pool, e.etablissement_id).await?;
        // Le destinataire doit encore être membre de l'établissement.
        let membre: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(e.etablissement_id)
        .bind(e.destinataire_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if membre.is_none() {
            return Ok(false);
        }
        let Some(droit) = e.droit_requis.as_deref() else {
            return Ok(true);
        };
        let Some(eleve) = e.eleve_id else {
            return Ok(false);
        };
        // Colonne choisie dans une liste fermée (jamais d'entrée libre dans le SQL).
        let colonne = match droit {
            "peut_voir_vs" => "l.peut_voir_vs",
            "peut_voir_notes" => "l.peut_voir_notes",
            "lien" => "true",
            _ => return Ok(false),
        };
        let ok: Option<(i32,)> = sqlx::query_as(&format!(
            "SELECT 1 FROM ent_eleve_lien_sollicitable l
              WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.user_id = $3
                AND {colonne} LIMIT 1"
        ))
        .bind(e.etablissement_id)
        .bind(eleve)
        .bind(e.destinataire_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        Ok(ok.is_some())
    }

    /// Envoie un message sur le canal de `e`. Réserve d'abord l'emplacement SMS.
    async fn expedier_un(
        &self,
        e: &EnvoiReclame,
        titre: &str,
        corps: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<Issue, EntDbError> {
        let mut c = TenantTx::begin(&self.pool, e.etablissement_id).await.map_err(db)?;
        let Some(contacts) = contacts_de(&mut c, e.etablissement_id, e.destinataire_user_id).await?
        else {
            return Ok(Issue::Ignore("destinataire_inconnu"));
        };
        drop(c);
        match e.canal.as_str() {
            "email" => {
                let Some(to) = contacts.email else {
                    return Ok(Issue::Ignore("sans_adresse"));
                };
                if !self.mailer.is_ready() {
                    return Ok(Issue::Ignore("canal_non_configure"));
                }
                let r = self
                    .mailer
                    .send(OutgoingMail {
                        to: to.clone(),
                        subject: titre.to_string(),
                        body_text: corps.to_string(),
                        attachments: Vec::new(),
                    })
                    .await;
                let (statut, erreur, mid) = match r {
                    Ok(r) => (r.status, r.error, r.message_id),
                    Err(err) => (MailSendStatus::Failed, Some(err.to_string()), None),
                };
                let _ = crate::mail_repo::PgMailRepo::new(self.pool.clone())
                    .insert(crate::mail_repo::InsertMailOutbox {
                        etablissement_id: e.etablissement_id,
                        eleve_id: e.eleve_id,
                        kind: e.kind.clone(),
                        to_email: to,
                        subject: titre.to_string(),
                        body_text: corps.to_string(),
                        status: statut,
                        error: erreur.clone(),
                        source_aggregate: Some("notification_envoi".into()),
                        source_id: Some(e.id),
                        message_id: mid.clone(),
                    })
                    .await;
                Ok(match statut {
                    MailSendStatus::Sent => Issue::Envoye(mid),
                    MailSendStatus::DryRun => Issue::Simule(mid),
                    MailSendStatus::SkippedNoConfig => Issue::Ignore("canal_non_configure"),
                    MailSendStatus::Failed => {
                        Issue::Echec(erreur.unwrap_or_else(|| "échec d'envoi".into()))
                    }
                })
            }
            "sms" => {
                let Some(tel) = contacts.telephone else {
                    return Ok(Issue::Ignore("sans_telephone"));
                };
                if !self.sms.is_ready() {
                    return Ok(Issue::Ignore("canal_non_configure"));
                }
                if let Some(motif) = self.reserver_sms(e, maintenant).await? {
                    return Ok(Issue::Plafonne(motif));
                }
                let texte = if canal_externe_neutre(&e.kind) {
                    TEXTE_NEUTRE_SMS.to_string()
                } else {
                    sms_texte(titre, corps)
                };
                let r = self
                    .sms
                    .send(OutgoingSms {
                        to: tel,
                        body: texte.clone(),
                    })
                    .await;
                let (statut, erreur, mid, to) = match r {
                    Ok(r) => (r.status, r.error, r.message_id, r.to),
                    Err(err) => (SmsSendStatus::Failed, Some(err.to_string()), None, String::new()),
                };
                if !to.is_empty() {
                    let _ = crate::sms_repo::PgSmsRepo::new(self.pool.clone())
                        .insert(crate::sms_repo::InsertSmsOutbox {
                            etablissement_id: e.etablissement_id,
                            eleve_id: e.eleve_id,
                            kind: e.kind.clone(),
                            to_phone: to,
                            body: texte,
                            status: statut,
                            error: erreur.clone(),
                            source_aggregate: Some("notification_envoi".into()),
                            source_id: Some(e.id),
                            message_id: mid.clone(),
                        })
                        .await;
                }
                Ok(match statut {
                    SmsSendStatus::Sent => Issue::Envoye(mid),
                    SmsSendStatus::DryRun => Issue::Simule(mid),
                    SmsSendStatus::SkippedNoConfig => Issue::Ignore("canal_non_configure"),
                    SmsSendStatus::Failed => {
                        Issue::Echec(erreur.unwrap_or_else(|| "échec d'envoi".into()))
                    }
                })
            }
            "push" => {
                let appareils: Vec<(String, String)> = sqlx::query_as(
                    "SELECT platform, token FROM ent_push_device
                      WHERE etablissement_id = $1 AND user_id = $2 AND revoked_at IS NULL
                      ORDER BY last_seen_at DESC LIMIT 5",
                )
                .bind(e.etablissement_id)
                .bind(e.destinataire_user_id)
                .fetch_all(&mut *TenantTx::begin(&self.pool, e.etablissement_id).await?)
                .await
                .map_err(db)?;
                if appareils.is_empty() {
                    return Ok(Issue::Ignore("sans_appareil"));
                }
                if !self.push.is_ready() {
                    return Ok(Issue::Ignore("canal_non_configure"));
                }
                let (mut envoye, mut simule, mut derniere) = (None, None, None);
                // M0a (D4) : un push (Apple, Google, écran verrouillé) ne porte jamais le contenu
                // d'une communication de masse.
                let (titre, corps): (&str, &str) = if canal_externe_neutre(&e.kind) {
                    (TITRE_NEUTRE_PUSH, TEXTE_NEUTRE_PUSH)
                } else {
                    (titre, corps)
                };
                for (plateforme, token) in appareils {
                    let Some(pf) = PushPlatform::parse(&plateforme) else {
                        continue;
                    };
                    let r = self
                        .push
                        .send(OutgoingPush {
                            token: token.clone(),
                            platform: pf,
                            title: titre.to_string(),
                            body: corps.chars().take(240).collect(),
                            data_json: None,
                        })
                        .await;
                    let (statut, erreur, mid) = match r {
                        Ok(r) => (r.status, r.error, r.message_id),
                        Err(err) => (PushSendStatus::Failed, Some(err.to_string()), None),
                    };
                    let _ = crate::push_repo::PgPushRepo::new(self.pool.clone())
                        .insert_outbox(crate::push_repo::InsertPushOutbox {
                            etablissement_id: e.etablissement_id,
                            eleve_id: e.eleve_id,
                            user_id: Some(e.destinataire_user_id),
                            kind: e.kind.clone(),
                            title: titre.to_string(),
                            body: corps.chars().take(240).collect(),
                            platform: Some(plateforme),
                            token: Some(token),
                            status: statut,
                            error: erreur.clone(),
                            source_aggregate: Some("notification_envoi".into()),
                            source_id: Some(e.id),
                            message_id: mid.clone(),
                        })
                        .await;
                    match statut {
                        PushSendStatus::Sent => envoye = Some(mid),
                        PushSendStatus::DryRun => simule = Some(mid),
                        PushSendStatus::Failed => derniere = erreur,
                        PushSendStatus::SkippedNoConfig => {}
                    }
                }
                Ok(if let Some(m) = envoye {
                    Issue::Envoye(m)
                } else if let Some(m) = simule {
                    Issue::Simule(m)
                } else if let Some(err) = derniere {
                    Issue::Echec(err)
                } else {
                    Issue::Ignore("canal_non_configure")
                })
            }
            _ => Ok(Issue::Ignore("canal_inconnu")),
        }
    }

    /// Réserve un emplacement dans les plafonds SMS du jour, sous verrou
    /// d'établissement. `Some(motif)` : plafond atteint (rien n'est envoyé).
    async fn reserver_sms(
        &self,
        e: &EnvoiReclame,
        maintenant: DateTime<Utc>,
    ) -> Result<Option<&'static str>, EntDbError> {
        let mut tx = TenantTx::begin(&self.pool, e.etablissement_id).await.map_err(db)?;
        // Verrou transactionnel par établissement : les réservations sont sérialisées.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 8))")
            .bind(e.etablissement_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let plafonds = plafonds_de(&mut tx, e.etablissement_id).await?;
        // Début du jour (Paris) → instant UTC.
        let (debut_jour,): (DateTime<Utc>,) = sqlx::query_as(
            "SELECT (date_trunc('day', $1::timestamptz AT TIME ZONE 'Europe/Paris')
                     AT TIME ZONE 'Europe/Paris')",
        )
        .bind(maintenant)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let (etab_jour, perso_jour): (i64, i64) = sqlx::query_as(
            "SELECT count(*), count(*) FILTER (WHERE destinataire_user_id = $3)
               FROM ent_notification_envoi
              WHERE etablissement_id = $1 AND canal = 'sms' AND plafond_reserve
                AND tentative_le >= $2",
        )
        .bind(e.etablissement_id)
        .bind(debut_jour)
        .bind(e.destinataire_user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if perso_jour >= i64::from(plafonds.sms_par_personne_jour) {
            tx.rollback().await.map_err(db)?;
            return Ok(Some("plafond_personne"));
        }
        if etab_jour >= i64::from(plafonds.sms_par_jour) {
            tx.rollback().await.map_err(db)?;
            return Ok(Some("plafond_etablissement"));
        }
        sqlx::query(
            "UPDATE ent_notification_envoi SET plafond_reserve = true
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e.etablissement_id)
        .bind(e.id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(None)
    }

    async fn finaliser(
        &self,
        e: &EnvoiReclame,
        issue: Issue,
        resume_id: Option<Uuid>,
        bilan: &mut BilanExpedition,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (statut, motif, mid) = match issue {
            Issue::Envoye(m) | Issue::Simule(m) if resume_id.is_some() => {
                bilan.regroupes += 1;
                ("regroupe", None, m)
            }
            Issue::Envoye(m) => {
                bilan.envoyes += 1;
                ("envoye", None, m)
            }
            Issue::Simule(m) => {
                bilan.simules += 1;
                ("simule", None, m)
            }
            Issue::Echec(err) => {
                bilan.echecs += 1;
                // Message borné : aucun détail de fournisseur au-delà de 200 caractères.
                ("echec", Some(err.chars().take(200).collect::<String>()), None)
            }
            Issue::Ignore(m) => {
                bilan.ignores += 1;
                ("ignore", Some(m.to_string()), None)
            }
            Issue::Plafonne(m) => {
                bilan.plafonnes += 1;
                ("plafonne", Some(m.to_string()), None)
            }
        };
        let mut tx = TenantTx::begin(&self.pool, e.etablissement_id).await?;
        sqlx::query(
            "UPDATE ent_notification_envoi
                SET statut = $3, motif = $4, message_id = $5, resume_id = $6, traite_le = $7
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'envoi'",
        )
        .bind(e.etablissement_id)
        .bind(e.id)
        .bind(statut)
        .bind(motif)
        .bind(mid)
        .bind(resume_id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn t(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 6)
            .unwrap()
            .and_hms_opt(h, m, 0)
            .unwrap()
    }

    #[test]
    fn plages_passant_minuit() {
        // 21:00 → 07:00
        assert!(dans_plage(22 * 60, 21 * 60, 7 * 60));
        assert!(dans_plage(3 * 60, 21 * 60, 7 * 60));
        assert!(!dans_plage(12 * 60, 21 * 60, 7 * 60));
        assert!(!dans_plage(7 * 60, 21 * 60, 7 * 60), "borne de fin exclue");
        // plage dans la journée
        assert!(dans_plage(13 * 60, 12 * 60, 14 * 60));
        assert!(!dans_plage(14 * 60, 12 * 60, 14 * 60));
    }

    #[test]
    fn echeance_immediate_sans_regle() {
        let p = Preferences::default();
        assert_eq!(echeance_locale(&p, "email", true, t(10, 0)), t(10, 0));
    }

    #[test]
    fn echeance_heures_calmes_reporte_au_matin() {
        let p = Preferences {
            calme_debut: Some(21 * 60),
            calme_fin: Some(7 * 60),
            ..Preferences::default()
        };
        // 22:00 → lendemain 07:00
        let lendemain = t(7, 0) + Duration::days(1);
        assert_eq!(echeance_locale(&p, "push", false, t(22, 0)), lendemain);
        // 03:00 → le jour même 07:00
        assert_eq!(echeance_locale(&p, "push", false, t(3, 0)), t(7, 0));
        // 10:00 : hors plage
        assert_eq!(echeance_locale(&p, "sms", false, t(10, 0)), t(10, 0));
    }

    #[test]
    fn echeance_resume_quotidien() {
        let p = Preferences {
            resume: "quotidien".into(),
            ..Preferences::default()
        };
        // Avant 07:30 : le matin même ; après : le lendemain matin.
        assert_eq!(echeance_locale(&p, "email", true, t(6, 0)), t(7, 30));
        assert_eq!(
            echeance_locale(&p, "email", true, t(9, 0)),
            t(7, 30) + Duration::days(1)
        );
        // Non regroupable ou SMS : immédiat.
        assert_eq!(echeance_locale(&p, "email", false, t(9, 0)), t(9, 0));
        assert_eq!(echeance_locale(&p, "sms", true, t(9, 0)), t(9, 0));
    }

    #[test]
    fn resume_dans_les_heures_calmes_attend_leur_fin() {
        // Calme 6:00 → 9:00 recouvre l'heure du résumé (7:30) : envoi à 9:00.
        let p = Preferences {
            resume: "quotidien".into(),
            calme_debut: Some(6 * 60),
            calme_fin: Some(9 * 60),
            ..Preferences::default()
        };
        assert_eq!(echeance_locale(&p, "email", true, t(5, 0)), t(9, 0));
    }

    #[test]
    fn canaux_selon_preferences_et_joignabilite() {
        let mut p = Preferences::default();
        let tous = ["email", "sms", "push"];
        // SMS désactivé par défaut même si joignable.
        assert_eq!(canaux_disponibles(&p, &tous, true, true, true), ["email", "push"]);
        p.sms = true;
        p.email = false;
        assert_eq!(canaux_disponibles(&p, &tous, true, true, true), ["sms", "push"]);
        // Sans téléphone ni appareil : rien de plus.
        assert!(canaux_disponibles(&p, &tous, true, false, false).is_empty());
        // L'émetteur restreint les canaux.
        assert_eq!(canaux_disponibles(&p, &["push"], true, true, true), ["push"]);
    }

    #[test]
    fn preferences_validees() {
        assert!(Preferences::default().valider().is_ok());
        let mut p = Preferences::default();
        p.resume = "hebdo".into();
        assert!(p.valider().is_err());
        let p = Preferences {
            calme_debut: Some(60),
            calme_fin: None,
            ..Preferences::default()
        };
        assert!(p.valider().is_err());
        let p = Preferences {
            calme_debut: Some(1440),
            calme_fin: Some(60),
            ..Preferences::default()
        };
        assert!(p.valider().is_err());
        let p = Preferences {
            calme_debut: Some(60),
            calme_fin: Some(60),
            ..Preferences::default()
        };
        assert!(p.valider().is_err());
    }

    #[test]
    fn sms_borne() {
        let long = "x".repeat(1000);
        let s = sms_texte("Titre", &long);
        assert!(s.chars().count() <= SMS_MAX);
        assert!(s.starts_with("Titre : "));
    }
}
