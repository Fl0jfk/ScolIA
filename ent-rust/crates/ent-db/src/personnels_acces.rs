//! Personnels, lot P2 : invitation, accueil, casquettes (socle 6 ; `docs/socles/06_PERSONNELS.md`
//! §8.4, §10.2, §13.2 ; PRS-20 à PRS-29).
//!
//! | Commande / lecture                              | PRS | Droit                          |
//! |-------------------------------------------------|-----|--------------------------------|
//! | `inviter_agent_for_actor` (e-mail ou QR)        | 20, 21 | `personnels.inviter`        |
//! | `inviter_lot_for_actor`                         | 22  | `personnels.inviter`           |
//! | `annuler_invitation_for_actor`, `relancer_…`    | 23  | `personnels.inviter`           |
//! | `invitation_apercu`, `accepter_invitation`      | 24  | **le jeton** (public)          |
//! | `preparer_arrivee`                              | 25  | l'agent lui-même               |
//! | `signaler_arrivees_du_jour`                     | 26  | tâche quotidienne idempotente  |
//! | `casquettes_de_la_session`, `basculer_casquette`| 29  | la session                     |
//!
//! Garanties (toutes testées sur Postgres) :
//! * **le jeton n'est jamais stocké ni relu** : la base ne garde que son empreinte SHA-256 ; il est
//!   à usage unique, valable 7 jours (e-mail) ou 10 minutes (QR code) ; une invitation expirée,
//!   annulée ou déjà utilisée reçoit la même réponse qu'un jeton inconnu ;
//! * **une identité existante n'est jamais fusionnée ni rattachée en silence** : il faut une session
//!   authentifiée du compte concerné ET une confirmation explicite ; une deuxième fiche « personne »
//!   du même compte est un conflit à résoudre par la fusion du socle 4, jamais un rattachement ;
//! * **une session = une casquette** : le rôle de la session est celui de la casquette ; il n'y a
//!   aucune union de droits (un professeur-parent n'est jamais professeur ET parent dans la même
//!   requête) ;
//! * **passkey obligatoire** pour la casquette personnel ouverte par une invitation, quelle que soit
//!   la politique de l'établissement : aucune session complète sans passkey enrôlée ;
//! * **accès réduit avant l'arrivée** : tant qu'aucun engagement n'est courant et qu'un engagement
//!   commence plus tard, la session est de portée `arrivee` (aucune donnée d'élève) ;
//! * cloisonnement par établissement (`TenantTx`, RLS), journal et événements sans jeton, sans
//!   adresse, sans mot de passe.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_auth::{
    assert_password_policy, generate_reset_token, hash_password, hash_token, normalize_email,
    Action, Role,
};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_mail::OutgoingMail;
use ent_types::personnels::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::auth_repo::{AssuranceSession, PorteeSession, SessionInfo};
use crate::dossier_personnes::aujourdhui_tx;
use crate::outbox::Publier;
use crate::personnels::{agent_tx, conflit, db, ecriture, introuvable, invalide, journal_tx, publier_tx};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Validité d'un lien envoyé par e-mail.
pub const TTL_EMAIL_HEURES: i64 = 7 * 24;
/// Validité d'un QR code affiché au guichet.
pub const TTL_QR_MINUTES: i64 = 10;
/// Réponse unique pour un jeton inconnu, expiré, annulé ou déjà utilisé (aucun oracle).
pub const MSG_INVITATION_INVALIDE: &str = "Invitation invalide, expirée ou déjà utilisée.";

/// Ce que les commandes de session ont besoin de connaître de la session appelante (jamais le jeton).
#[derive(Debug, Clone)]
pub struct CtxSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub etablissement_id: Uuid,
    pub casquette: String,
    pub assurance: AssuranceSession,
    pub portee: PorteeSession,
}

impl From<&SessionInfo> for CtxSession {
    fn from(s: &SessionInfo) -> Self {
        Self {
            session_id: s.session_id,
            user_id: s.user_id,
            etablissement_id: s.etablissement_id,
            casquette: s.casquette.clone(),
            assurance: s.assurance,
            portee: s.portee,
        }
    }
}

/// Issue d'une émission : le lien du QR code n'existe que dans la réponse à l'émission.
pub enum InvitationEmise {
    Email(InvitationDto),
    Qr(InvitationQrDto),
}

/// Erreurs de l'acceptation, distinguées pour que le client sache quoi demander à la personne.
#[derive(Debug)]
pub enum ErreurAcceptation {
    /// Jeton inconnu, expiré, annulé ou déjà utilisé.
    Invalide,
    /// Une identité existe : se connecter avec le compte concerné, puis recommencer.
    ConnexionRequise,
    /// Une identité existe et la personne est connectée : le rattachement doit être confirmé.
    ConfirmationRequise,
    /// Conflit explicite (adresse prise, deuxième fiche à fusionner, accès déjà ouvert…).
    Conflit(&'static str),
    /// Mot de passe refusé par la politique.
    MotDePasse(String),
    Db(EntDbError),
}

impl From<EntDbError> for ErreurAcceptation {
    fn from(e: EntDbError) -> Self {
        Self::Db(e)
    }
}

// ---------------------------------------------------------------------------------------------
// Jeton
// ---------------------------------------------------------------------------------------------

/// Jeton = identifiant de l'établissement (32 hex, pour retrouver le cloisonnement avant toute
/// lecture) + 32 octets aléatoires (64 hex). Seule l'empreinte du tout est stockée.
fn nouveau_jeton(etab: Uuid) -> (String, String) {
    let (alea, _) = generate_reset_token();
    let brut = format!("{}{}", etab.simple(), alea);
    let empreinte = hash_token(&brut);
    (brut, empreinte)
}

/// Forme attendue : 96 caractères hexadécimaux minuscules. Tout autre texte est refusé sans
/// toucher la base. Renvoie l'établissement porté par le jeton et l'empreinte.
fn lire_jeton(brut: &str) -> Option<(Uuid, String)> {
    let brut = brut.trim();
    if brut.len() != 96 || !brut.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return None;
    }
    let etab = Uuid::parse_str(&brut[..32]).ok()?;
    if etab.is_nil() {
        return None;
    }
    Some((etab, hash_token(brut)))
}

fn masquer_email(e: &str) -> String {
    match e.split_once('@') {
        Some((local, domaine)) => {
            let c: String = local.chars().take(1).collect();
            format!("{c}***@{domaine}")
        }
        None => "***".to_string(),
    }
}

// ---------------------------------------------------------------------------------------------
// Lignes et DTO
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
struct LigneInvitation {
    id: Uuid,
    agent_id: Uuid,
    canal: String,
    email_cible: Option<String>,
    profil_base: String,
    cree_le: DateTime<Utc>,
    expire_le: DateTime<Utc>,
    utilisee_le: Option<DateTime<Utc>>,
    annulee_le: Option<DateTime<Utc>>,
    rattachement_identite_existante: bool,
}

const COLONNES_INVITATION: &str = "i.id, i.agent_id, i.canal, i.email_cible, i.profil_base, i.cree_le,
    i.expire_le, i.utilisee_le, i.annulee_le, i.rattachement_identite_existante";

fn dto(l: &LigneInvitation, maintenant: DateTime<Utc>) -> InvitationDto {
    let etat = if l.utilisee_le.is_some() {
        "utilisee"
    } else if l.annulee_le.is_some() {
        "annulee"
    } else if l.expire_le <= maintenant {
        "expiree"
    } else {
        "en_attente"
    };
    InvitationDto {
        id: l.id,
        agent_id: l.agent_id,
        canal: l.canal.clone(),
        profil: l.profil_base.clone(),
        etat: etat.to_string(),
        email_masque: l.email_cible.as_deref().map(masquer_email),
        cree_le: l.cree_le,
        expire_le: l.expire_le,
        utilisee_le: l.utilisee_le,
        annulee_le: l.annulee_le,
        rattachement_identite_existante: l.rattachement_identite_existante,
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct PersonneAgent {
    personne_id: Uuid,
    user_id: Option<Uuid>,
    email: Option<String>,
    fusionnee_dans: Option<Uuid>,
}

async fn personne_de_l_agent(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
) -> Result<PersonneAgent, EntDbError> {
    sqlx::query_as::<_, PersonneAgent>(
        "SELECT p.id AS personne_id, p.user_id, p.email, p.fusionnee_dans
           FROM ent_agent a
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND a.id = $2 FOR UPDATE OF p",
    )
    .bind(etab)
    .bind(agent_id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

/// Compte (n'importe quel établissement) qui porte cette adresse.
async fn compte_par_email(conn: &mut PgConnection, email: &str) -> Result<Option<Uuid>, EntDbError> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_user WHERE lower(email) = $1")
        .bind(email)
        .fetch_optional(conn)
        .await
        .map_err(db)?;
    Ok(row.map(|(id,)| id))
}

/// Profil d'accès du compte dans l'établissement (`None` : aucun accès ici).
async fn profil_du_compte(
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
) -> Result<Option<(String, String)>, EntDbError> {
    sqlx::query_as(
        "SELECT role, etat FROM ent_membership
          WHERE etablissement_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(etab)
    .bind(user_id)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

impl PgEnt {
    /// Hiérarchie des profils invitables : admin et direction par un administrateur seulement ;
    /// administratif, comptabilité, infirmerie et psychologue par qui gère les agents
    /// (`personnels.gerer`) ; professeur, vie scolaire et accueil par tout détenteur de
    /// `personnels.inviter` (le secrétariat). Un secrétariat ne peut donc pas s'inviter un profil
    /// supérieur au sien.
    async fn profil_invitable(
        &self,
        etab: Uuid,
        actor: Uuid,
        actor_role: &str,
        profil: &str,
    ) -> Result<Role, EntDbError> {
        let cible = Role::parse(profil)
            .filter(|r| r.is_staff())
            .ok_or_else(|| invalide("Profil d'accès inconnu."))?;
        let acteur = Role::parse(actor_role).ok_or(EntDbError::Forbidden)?;
        let autorise = match cible {
            Role::Admin | Role::Direction => acteur == Role::Admin,
            Role::Administratif | Role::Compta | Role::Infirmerie | Role::Psy => {
                self.a_action_etablissement(etab, actor, actor_role, Action::PersonnelsGerer)
                    .await?
            }
            _ => true,
        };
        if autorise {
            Ok(cible)
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Émet une invitation dans la transaction : annule l'éventuelle invitation en attente de
    /// l'agent (relancer = annuler + émettre), écrit l'empreinte, journalise, publie.
    #[allow(clippy::too_many_arguments)]
    async fn emettre_invitation_tx(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        agent_id: Uuid,
        canal: &str,
        profil: Role,
        email: Option<&str>,
        relance_de: Option<Uuid>,
    ) -> Result<(String, LigneInvitation), EntDbError> {
        sqlx::query(
            "UPDATE ent_agent_invitation SET annulee_le = now(), annulee_par_user_id = $3
              WHERE etablissement_id = $1 AND agent_id = $2 AND utilisee_le IS NULL AND annulee_le IS NULL",
        )
        .bind(etab)
        .bind(agent_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let (brut, empreinte) = nouveau_jeton(etab);
        let ttl = if canal == "qr" {
            Duration::minutes(TTL_QR_MINUTES)
        } else {
            Duration::hours(TTL_EMAIL_HEURES)
        };
        let id = Uuid::new_v4();
        let ligne = sqlx::query_as::<_, LigneInvitation>(&format!(
            "INSERT INTO ent_agent_invitation AS i
                (id, etablissement_id, agent_id, canal, email_cible, profil_base, jeton_hash,
                 cree_par_user_id, expire_le, relance_de)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now() + make_interval(secs => $9), $10)
             RETURNING {COLONNES_INVITATION}"
        ))
        .bind(id)
        .bind(etab)
        .bind(agent_id)
        .bind(canal)
        .bind(email)
        .bind(profil.as_str())
        .bind(&empreinte)
        .bind(actor)
        .bind(ttl.num_seconds() as f64)
        .bind(relance_de)
        .fetch_one(&mut *tx)
        .await
        .map_err(ecriture)?;
        journal_tx(
            tx,
            etab,
            "invitation_emise",
            "Invitation d'un agent émise",
            json!({ "invitation_id": id, "agent_id": agent_id, "canal": canal,
                    "profil": profil.as_str(), "relance": relance_de.is_some() }),
            actor,
        )
        .await?;
        publier_tx(
            tx,
            etab,
            types::PERSONNEL_INVITE,
            agent_id,
            json!({ "invitation_id": id, "agent_id": agent_id, "canal": canal }),
            actor,
        )
        .await?;
        Ok((brut, ligne))
    }

    fn lien_invitation(public_base_url: &str, brut: &str) -> String {
        format!(
            "{}/invitation-personnel?token={}",
            public_base_url.trim_end_matches('/'),
            brut
        )
    }

    /// Envoie le lien par e-mail. Aucun journal ni message d'erreur ne contient le lien.
    async fn envoyer_invitation(&self, a: &str, lien: &str) -> bool {
        let envoi = self
            .mailer()
            .send(OutgoingMail {
                to: a.to_string(),
                subject: "Invitation à rejoindre l'ENT de votre établissement".into(),
                body_text: format!(
                    "Bonjour,\n\nVotre établissement vous invite à activer votre accès à l'ENT. \
                     Ouvrez ce lien pour confirmer votre identité et enregistrer votre clé de \
                     sécurité (passkey) ; il est valable 7 jours et ne peut servir qu'une fois :\n\
                     {lien}\n\nSi vous n'êtes pas concerné(e), ignorez ce message.\n"
                ),
                attachments: vec![],
            })
            .await;
        matches!(envoi, Ok(r) if self.envoi_reussi(&r))
    }

    /// PRS-20 / PRS-21 : inviter un agent par e-mail (lien, 7 jours) ou par QR code (10 minutes).
    pub async fn inviter_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        actor_role: &str,
        agent_id: Uuid,
        body: &InviterAgentBody,
        public_base_url: &str,
    ) -> Result<InvitationEmise, EntDbError> {
        self.exiger_personnels(etab, actor, actor_role, Action::PersonnelsInviter)
            .await?;
        let profil = self.profil_invitable(etab, actor, actor_role, &body.profil).await?;
        let canal = match body.canal.as_str() {
            "email" | "qr" => body.canal.as_str(),
            _ => return Err(invalide("Canal inconnu : « email » ou « qr ».")),
        };
        // Avant toute écriture : sans URL publique, le lien serait inutilisable.
        if public_base_url.trim().is_empty() || (canal == "email" && !self.mailer().is_ready()) {
            return Err(EntDbError::EnvoiImpossible);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (brut, ligne) = self
            .preparer_et_emettre_tx(&mut tx, etab, actor, agent_id, canal, profil, body.email.as_deref(), None)
            .await?;
        tx.commit().await.map_err(db)?;
        self.livrer_invitation(etab, actor, canal, ligne, brut, public_base_url).await
    }

    /// Contrôles propres à une cible, puis émission (partagé par l'invitation simple, le lot et la relance).
    #[allow(clippy::too_many_arguments)]
    async fn preparer_et_emettre_tx(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        agent_id: Uuid,
        canal: &str,
        profil: Role,
        email_saisi: Option<&str>,
        relance_de: Option<Uuid>,
    ) -> Result<(String, LigneInvitation), EntDbError> {
        let agent = agent_tx(tx, etab, agent_id, true).await?;
        if agent.fusionne_dans_agent_id.is_some() {
            return Err(conflit("Cet agent est fusionné dans un autre."));
        }
        let personne = personne_de_l_agent(tx, etab, agent_id).await?;
        if personne.fusionnee_dans.is_some() {
            return Err(conflit("Cette fiche est fusionnée dans une autre."));
        }
        // On n'invite pas soi-même ; on n'invite pas qui a déjà un accès du personnel.
        if personne.user_id == Some(actor) {
            return Err(EntDbError::Forbidden);
        }
        if let Some(uid) = personne.user_id {
            if let Some((role, _)) = profil_du_compte(tx, etab, uid).await? {
                if role != Role::Parent.as_str() {
                    return Err(conflit("Cet agent a déjà un accès à l'établissement."));
                }
            }
        }
        let email = if canal == "email" {
            let brut = email_saisi
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .or(personne.email.as_deref())
                .ok_or_else(|| invalide("Adresse e-mail manquante pour cet agent."))?;
            let e = normalize_email(brut).map_err(|_| invalide("Adresse e-mail invalide."))?;
            // L'adresse désigne-t-elle un compte qui a déjà un accès du personnel ici ?
            if let Some(uid) = compte_par_email(tx, &e).await? {
                if let Some((role, _)) = profil_du_compte(tx, etab, uid).await? {
                    if role != Role::Parent.as_str() {
                        return Err(conflit("Cette adresse a déjà un accès à l'établissement."));
                    }
                }
            }
            Some(e)
        } else {
            None
        };
        self.emettre_invitation_tx(tx, etab, actor, agent_id, canal, profil, email.as_deref(), relance_de)
            .await
    }

    /// Après validation de la transaction : envoie le lien (e-mail) ou le rend (QR). Un envoi qui
    /// échoue annule l'invitation (le jeton n'a jamais quitté le serveur).
    async fn livrer_invitation(
        &self,
        etab: Uuid,
        actor: Uuid,
        canal: &str,
        ligne: LigneInvitation,
        brut: String,
        public_base_url: &str,
    ) -> Result<InvitationEmise, EntDbError> {
        let maintenant = Utc::now();
        let lien = Self::lien_invitation(public_base_url, &brut);
        if canal == "email" {
            let a = ligne.email_cible.clone().unwrap_or_default();
            if !self.envoyer_invitation(&a, &lien).await {
                self.annuler_apres_echec(etab, actor, ligne.id).await;
                return Err(EntDbError::EnvoiImpossible);
            }
            Ok(InvitationEmise::Email(dto(&ligne, maintenant)))
        } else {
            Ok(InvitationEmise::Qr(InvitationQrDto {
                invitation: dto(&ligne, maintenant),
                lien,
            }))
        }
    }

    async fn annuler_apres_echec(&self, etab: Uuid, actor: Uuid, id: Uuid) {
        if let Ok(mut tx) = TenantTx::begin(self.pool(), etab).await {
            let _ = sqlx::query(
                "UPDATE ent_agent_invitation SET annulee_le = now(), annulee_par_user_id = $3
                  WHERE etablissement_id = $1 AND id = $2 AND utilisee_le IS NULL AND annulee_le IS NULL",
            )
            .bind(etab)
            .bind(id)
            .bind(actor)
            .execute(&mut *tx)
            .await;
            let _ = tx.commit().await;
        }
    }

    /// PRS-23 : invitations d'un agent (jamais de jeton, adresse masquée).
    pub async fn invitations_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        actor_role: &str,
        agent_id: Uuid,
    ) -> Result<Vec<InvitationDto>, EntDbError> {
        self.exiger_personnels(etab, actor, actor_role, Action::PersonnelsInviter)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        agent_tx(&mut tx, etab, agent_id, false).await?;
        let lignes = sqlx::query_as::<_, LigneInvitation>(&format!(
            "SELECT {COLONNES_INVITATION} FROM ent_agent_invitation i
              WHERE i.etablissement_id = $1 AND i.agent_id = $2 ORDER BY i.cree_le DESC LIMIT 50"
        ))
        .bind(etab)
        .bind(agent_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let now = Utc::now();
        Ok(lignes.iter().map(|l| dto(l, now)).collect())
    }

    /// PRS-23 : annuler une invitation en attente. Idempotent ; refusé sur une invitation utilisée.
    pub async fn annuler_invitation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        actor_role: &str,
        invitation_id: Uuid,
    ) -> Result<InvitationDto, EntDbError> {
        self.exiger_personnels(etab, actor, actor_role, Action::PersonnelsInviter)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l = sqlx::query_as::<_, LigneInvitation>(&format!(
            "SELECT {COLONNES_INVITATION} FROM ent_agent_invitation i
              WHERE i.etablissement_id = $1 AND i.id = $2 FOR UPDATE"
        ))
        .bind(etab)
        .bind(invitation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if l.utilisee_le.is_some() {
            return Err(conflit("Cette invitation a déjà été utilisée."));
        }
        let mut l = l;
        if l.annulee_le.is_none() {
            l = sqlx::query_as::<_, LigneInvitation>(&format!(
                "UPDATE ent_agent_invitation i SET annulee_le = now(), annulee_par_user_id = $3
                  WHERE i.etablissement_id = $1 AND i.id = $2 RETURNING {COLONNES_INVITATION}"
            ))
            .bind(etab)
            .bind(invitation_id)
            .bind(actor)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            journal_tx(
                &mut tx,
                etab,
                "invitation_annulee",
                "Invitation d'un agent annulée",
                json!({ "invitation_id": invitation_id, "agent_id": l.agent_id }),
                actor,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(dto(&l, Utc::now()))
    }

    /// PRS-23 : relancer = annuler l'invitation et en émettre une nouvelle (nouveau jeton, mêmes
    /// canal, adresse et profil). L'ancien lien cesse de fonctionner.
    pub async fn relancer_invitation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        actor_role: &str,
        invitation_id: Uuid,
        public_base_url: &str,
    ) -> Result<InvitationEmise, EntDbError> {
        self.exiger_personnels(etab, actor, actor_role, Action::PersonnelsInviter)
            .await?;
        if public_base_url.trim().is_empty() {
            return Err(EntDbError::EnvoiImpossible);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l = sqlx::query_as::<_, LigneInvitation>(&format!(
            "SELECT {COLONNES_INVITATION} FROM ent_agent_invitation i
              WHERE i.etablissement_id = $1 AND i.id = $2 FOR UPDATE"
        ))
        .bind(etab)
        .bind(invitation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if l.utilisee_le.is_some() {
            return Err(conflit("Cette invitation a déjà été utilisée."));
        }
        let profil = self.profil_invitable(etab, actor, actor_role, &l.profil_base).await?;
        if l.canal == "email" && !self.mailer().is_ready() {
            return Err(EntDbError::EnvoiImpossible);
        }
        let (brut, nouvelle) = self
            .preparer_et_emettre_tx(
                &mut tx,
                etab,
                actor,
                l.agent_id,
                &l.canal,
                profil,
                l.email_cible.as_deref(),
                Some(invitation_id),
            )
            .await?;
        tx.commit().await.map_err(db)?;
        let canal = l.canal.clone();
        self.livrer_invitation(etab, actor, &canal, nouvelle, brut, public_base_url).await
    }

    /// PRS-22 : inviter en lot (par e-mail). L'aperçu ne fait aucune écriture et liste les adresses
    /// manquantes ; chaque agent est traité dans sa propre transaction (un refus n'arrête pas le lot).
    pub async fn inviter_lot_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        actor_role: &str,
        body: &InvitationsLotBody,
        public_base_url: &str,
    ) -> Result<Vec<InvitationLotLigneDto>, EntDbError> {
        self.exiger_personnels(etab, actor, actor_role, Action::PersonnelsInviter)
            .await?;
        let profil = self.profil_invitable(etab, actor, actor_role, &body.profil).await?;
        if body.agent_ids.is_empty() || body.agent_ids.len() > 200 {
            return Err(invalide("Un lot compte de 1 à 200 agents."));
        }
        if !body.apercu && (public_base_url.trim().is_empty() || !self.mailer().is_ready()) {
            return Err(EntDbError::EnvoiImpossible);
        }
        let mut sortie = Vec::with_capacity(body.agent_ids.len());
        let mut envoyees = 0usize;
        for agent_id in &body.agent_ids {
            let resultat = self
                .invitation_du_lot(etab, actor, *agent_id, profil, body.apercu, public_base_url)
                .await;
            if resultat == "envoyee" {
                envoyees += 1;
            }
            sortie.push(InvitationLotLigneDto {
                agent_id: *agent_id,
                resultat: resultat.to_string(),
            });
        }
        if !body.apercu {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            journal_tx(
                &mut tx,
                etab,
                "invitations_lot",
                "Invitations en lot",
                json!({ "demandees": body.agent_ids.len(), "envoyees": envoyees, "profil": profil.as_str() }),
                actor,
            )
            .await?;
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::PERSONNEL_INVITATIONS_LOT.to_string(),
                aggregate: AggregateKind::Agent,
                aggregate_id: actor,
                eleve_id: None,
                payload: json!({ "demandees": body.agent_ids.len(), "envoyees": envoyees }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
            tx.commit().await.map_err(db)?;
        }
        Ok(sortie)
    }

    async fn invitation_du_lot(
        &self,
        etab: Uuid,
        actor: Uuid,
        agent_id: Uuid,
        profil: Role,
        apercu: bool,
        public_base_url: &str,
    ) -> &'static str {
        let Ok(mut tx) = TenantTx::begin(self.pool(), etab).await else {
            return "echec_envoi";
        };
        let Ok(agent) = agent_tx(&mut tx, etab, agent_id, false).await else {
            return "introuvable";
        };
        if agent.fusionne_dans_agent_id.is_some() {
            return "non_autorisee";
        }
        let Ok(personne) = personne_de_l_agent(&mut tx, etab, agent_id).await else {
            return "introuvable";
        };
        if personne.user_id == Some(actor) {
            return "non_autorisee";
        }
        if let Some(uid) = personne.user_id {
            if let Ok(Some((role, _))) = profil_du_compte(&mut tx, etab, uid).await {
                if role != Role::Parent.as_str() {
                    return "deja_un_acces";
                }
            }
        }
        let Some(adresse) = personne.email.as_deref().filter(|e| normalize_email(e).is_ok()) else {
            return "adresse_manquante";
        };
        if apercu {
            return "a_envoyer";
        }
        match self
            .preparer_et_emettre_tx(&mut tx, etab, actor, agent_id, "email", profil, Some(adresse), None)
            .await
        {
            Ok((brut, ligne)) => {
                if tx.commit().await.is_err() {
                    return "echec_envoi";
                }
                match self
                    .livrer_invitation(etab, actor, "email", ligne, brut, public_base_url)
                    .await
                {
                    Ok(_) => "envoyee",
                    Err(_) => "echec_envoi",
                }
            }
            Err(EntDbError::Forbidden) => "non_autorisee",
            Err(_) => "deja_un_acces",
        }
    }

    // -----------------------------------------------------------------------------------------
    // PRS-24 : côté personne invitée (public, le jeton est la preuve)
    // -----------------------------------------------------------------------------------------

    /// Compte désigné par l'invitation, s'il en existe un : celui de la fiche de la personne, sinon
    /// celui de l'adresse mémorisée, sinon celui de l'adresse saisie (QR code).
    async fn compte_vise_tx(
        conn: &mut PgConnection,
        personne: &PersonneAgent,
        email_cible: Option<&str>,
        email_saisi: Option<&str>,
    ) -> Result<Option<Uuid>, EntDbError> {
        if let Some(u) = personne.user_id {
            return Ok(Some(u));
        }
        for e in [email_cible, email_saisi].into_iter().flatten() {
            if let Ok(e) = normalize_email(e) {
                if let Some(u) = compte_par_email(conn, &e).await? {
                    return Ok(Some(u));
                }
            }
        }
        Ok(None)
    }

    /// Aperçu pour la personne qui détient le lien. Toute invitation non valide : même réponse.
    pub async fn invitation_personnel_apercu(&self, jeton: &str) -> Result<InvitationApercuDto, EntDbError> {
        let invalide_ = || invalide(MSG_INVITATION_INVALIDE);
        let (etab, empreinte) = lire_jeton(jeton).ok_or_else(invalide_)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l = sqlx::query_as::<_, LigneInvitation>(&format!(
            "SELECT {COLONNES_INVITATION} FROM ent_agent_invitation i
              WHERE i.etablissement_id = $1 AND i.jeton_hash = $2
                AND i.utilisee_le IS NULL AND i.annulee_le IS NULL AND i.expire_le > now()"
        ))
        .bind(etab)
        .bind(&empreinte)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(invalide_)?;
        let personne = personne_de_l_agent(&mut tx, etab, l.agent_id).await?;
        let existant = Self::compte_vise_tx(&mut tx, &personne, l.email_cible.as_deref(), None).await?;
        let (avant,): (bool,) = sqlx::query_as(
            "SELECT NOT EXISTS (SELECT 1 FROM ent_agent_engagement e
                                 WHERE e.etablissement_id = $1 AND e.agent_id = $2 AND e.annule_le IS NULL
                                   AND e.du <= ent_aujourdhui_etab($1)
                                   AND (e.au IS NULL OR e.au >= ent_aujourdhui_etab($1)))
                    AND EXISTS (SELECT 1 FROM ent_agent_engagement e
                                 WHERE e.etablissement_id = $1 AND e.agent_id = $2 AND e.annule_le IS NULL
                                   AND e.du > ent_aujourdhui_etab($1))",
        )
        .bind(etab)
        .bind(l.agent_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(InvitationApercuDto {
            etablissement_id: etab,
            canal: l.canal,
            profil: l.profil_base,
            expire_le: l.expire_le,
            identite_existante: existant.is_some(),
            avant_arrivee: avant,
        })
    }

    /// PRS-24 : accepter l'invitation. `session` est la session éventuelle de l'appelant (identité
    /// existante : connexion préalable, jamais de rattachement sur la seule foi du jeton).
    pub async fn accepter_invitation_personnel(
        &self,
        corps: &AccepterInvitationBody,
        session: Option<&SessionInfo>,
    ) -> Result<InvitationAccepteeDto, ErreurAcceptation> {
        let (etab, empreinte) = lire_jeton(&corps.token).ok_or(ErreurAcceptation::Invalide)?;
        // Politique de mot de passe vérifiée avant toute écriture (nouvelle identité seulement,
        // mais le contrôle est sans effet de bord).
        let mut tx = TenantTx::begin(self.pool(), etab)
            .await
            .map_err(|e| ErreurAcceptation::Db(db(e)))?;
        let l = sqlx::query_as::<_, LigneInvitation>(&format!(
            "SELECT {COLONNES_INVITATION} FROM ent_agent_invitation i
              WHERE i.etablissement_id = $1 AND i.jeton_hash = $2
                AND i.utilisee_le IS NULL AND i.annulee_le IS NULL AND i.expire_le > now()
              FOR UPDATE"
        ))
        .bind(etab)
        .bind(&empreinte)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ErreurAcceptation::Db(db(e)))?
        .ok_or(ErreurAcceptation::Invalide)?;
        let profil = Role::parse(&l.profil_base)
            .filter(|r| r.is_staff())
            .ok_or(ErreurAcceptation::Invalide)?;
        let agent = agent_tx(&mut tx, etab, l.agent_id, true).await?;
        if agent.fusionne_dans_agent_id.is_some() {
            return Err(ErreurAcceptation::Invalide);
        }
        let personne = personne_de_l_agent(&mut tx, etab, l.agent_id).await?;
        if personne.fusionnee_dans.is_some() {
            return Err(ErreurAcceptation::Invalide);
        }
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let vise = Self::compte_vise_tx(
            &mut tx,
            &personne,
            l.email_cible.as_deref(),
            corps.email.as_deref(),
        )
        .await?;

        let (user_id, identite_existante, ancien_role) = match vise {
            Some(u) => {
                // Identité existante : connexion préalable du compte concerné ET confirmation.
                let Some(s) = session else {
                    return Err(ErreurAcceptation::ConnexionRequise);
                };
                if s.user_id != u || s.portee != PorteeSession::Complete {
                    return Err(ErreurAcceptation::ConnexionRequise);
                }
                if !corps.confirmer_rattachement {
                    return Err(ErreurAcceptation::ConfirmationRequise);
                }
                let ancien = profil_du_compte(&mut tx, etab, u).await?;
                if let Some((role, etat)) = &ancien {
                    if role != Role::Parent.as_str() {
                        return Err(ErreurAcceptation::Conflit(
                            "Ce compte a déjà un accès du personnel à l'établissement.",
                        ));
                    }
                    if etat != "ouvert" {
                        return Err(ErreurAcceptation::Conflit(
                            "L'accès existant de ce compte n'est pas ouvert : contacter la direction.",
                        ));
                    }
                }
                (u, true, ancien.map(|(r, _)| r))
            }
            None => {
                let mdp = corps
                    .mot_de_passe
                    .as_deref()
                    .ok_or(ErreurAcceptation::MotDePasse("Mot de passe requis.".into()))?;
                assert_password_policy(mdp)
                    .map_err(|e| ErreurAcceptation::MotDePasse(e.to_string()))?;
                let adresse = l
                    .email_cible
                    .clone()
                    .or_else(|| corps.email.clone())
                    .ok_or(ErreurAcceptation::MotDePasse("Adresse e-mail requise.".into()))?;
                let adresse = normalize_email(&adresse)
                    .map_err(|_| ErreurAcceptation::MotDePasse("Adresse e-mail invalide.".into()))?;
                let hash = hash_password(mdp).map_err(|_| ErreurAcceptation::Invalide)?;
                let nom = corps
                    .nom_affiche
                    .as_deref()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(|n| n.chars().take(120).collect::<String>());
                let u = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
                     VALUES ($1,$2,$3,$4, now(), now())",
                )
                .bind(u)
                .bind(&adresse)
                .bind(&hash)
                .bind(nom)
                .execute(&mut *tx)
                .await
                .map_err(|e| match &e {
                    sqlx::Error::Database(d) if d.constraint() == Some("ent_user_email_unique") => {
                        ErreurAcceptation::Conflit("Cette adresse est déjà utilisée : connectez-vous.")
                    }
                    _ => ErreurAcceptation::Db(db(e)),
                })?;
                (u, false, None)
            }
        };

        // Une seule fiche « personne » par compte et par établissement : si le compte en a déjà
        // une autre (parent saisi par la famille), c'est une fusion du socle 4 à faire, jamais un
        // rattachement silencieux.
        let (autre_fiche,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_personne
                             WHERE etablissement_id = $1 AND user_id = $2 AND id <> $3)",
        )
        .bind(etab)
        .bind(user_id)
        .bind(personne.personne_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ErreurAcceptation::Db(db(e)))?;
        if autre_fiche {
            return Err(ErreurAcceptation::Conflit(
                "Ce compte est déjà rattaché à une autre fiche : fusionner les deux fiches d'abord.",
            ));
        }
        if personne.user_id.is_some_and(|u| u != user_id) {
            return Err(ErreurAcceptation::Invalide);
        }

        // La fiche porte le compte AVANT l'accès : le déclencheur « un agent par compte »
        // retrouve alors l'agent invité au lieu d'en créer un second.
        let lie_maintenant = personne.user_id.is_none();
        if lie_maintenant {
            sqlx::query(
                "UPDATE ent_personne SET user_id = $3, updated_at = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(personne.personne_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ErreurAcceptation::Db(ecriture(e)))?;
        }
        match ancien_role {
            Some(_) => {
                sqlx::query(
                    "UPDATE ent_membership SET role = $3 WHERE etablissement_id = $1 AND user_id = $2",
                )
                .bind(etab)
                .bind(user_id)
                .bind(profil.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|e| ErreurAcceptation::Db(ecriture(e)))?;
            }
            None => {
                sqlx::query(
                    "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at)
                     VALUES ($1,$2,$3, now())",
                )
                .bind(user_id)
                .bind(etab)
                .bind(profil.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|e| ErreurAcceptation::Db(ecriture(e)))?;
            }
        }
        // Casquette personnel (passkey exigée) ; casquette parent si la personne est déjà
        // responsable de quelqu'un dans l'établissement (ancien accès parent ou lien actif).
        sqlx::query(
            "INSERT INTO ent_membership_casquette
                (user_id, etablissement_id, casquette, profil_base, source_ref, du, passkey_requise, cree_par_user_id)
             VALUES ($1,$2,'personnel',$3,$4,$5,true,$6)
             ON CONFLICT (user_id, etablissement_id, casquette) DO UPDATE
                SET profil_base = EXCLUDED.profil_base, source_ref = EXCLUDED.source_ref,
                    etat = 'ouvert', au = NULL, passkey_requise = true",
        )
        .bind(user_id)
        .bind(etab)
        .bind(profil.as_str())
        .bind(l.agent_id)
        .bind(jour)
        .bind(l.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ErreurAcceptation::Db(ecriture(e)))?;
        let etait_parent = ancien_role.as_deref() == Some(Role::Parent.as_str());
        let parent = ouvrir_casquette_parent_tx(
            &mut tx,
            etab,
            user_id,
            personne.personne_id,
            jour,
            etait_parent,
        )
        .await?;

        let (pris,): (i64,) = sqlx::query_as(
            "WITH m AS (UPDATE ent_agent_invitation
                           SET utilisee_le = now(), user_id_rattache = $3,
                               rattachement_identite_existante = $4
                         WHERE etablissement_id = $1 AND id = $2
                           AND utilisee_le IS NULL AND annulee_le IS NULL AND expire_le > now()
                         RETURNING 1)
             SELECT count(*) FROM m",
        )
        .bind(etab)
        .bind(l.id)
        .bind(user_id)
        .bind(identite_existante)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ErreurAcceptation::Db(db(e)))?;
        if pris != 1 {
            return Err(ErreurAcceptation::Invalide);
        }
        // Le profil de l'accès a changé : les sessions ouvertes de ce compte ici (parent) sont
        // révoquées, la personne se reconnecte et passe par l'enrôlement de la passkey.
        sqlx::query(
            "UPDATE ent_session SET revoked_at = now()
              WHERE user_id = $1 AND etablissement_id = $2 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(|e| ErreurAcceptation::Db(db(e)))?;

        journal_tx(
            &mut tx,
            etab,
            "acces_rattache",
            "Invitation acceptée : accès du personnel ouvert",
            json!({ "invitation_id": l.id, "agent_id": l.agent_id, "canal": l.canal,
                    "profil": profil.as_str(), "identite_existante": identite_existante,
                    "casquette_parent": parent, "fiche_liee_au_compte": lie_maintenant }),
            user_id,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_ACCES_RATTACHE,
            l.agent_id,
            json!({ "invitation_id": l.id, "agent_id": l.agent_id,
                    "identite_existante": identite_existante }),
            user_id,
        )
        .await?;
        if lie_maintenant {
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::PERSONNE_COMPTE_RATTACHE.to_string(),
                aggregate: AggregateKind::Personne,
                aggregate_id: personne.personne_id,
                eleve_id: None,
                payload: json!({ "personne_id": personne.personne_id }),
                actor_user_id: Some(user_id.to_string()),
            })
            .await
            .map_err(|e| ErreurAcceptation::Db(e.into()))?;
        }
        let (avant,): (bool,) = sqlx::query_as("SELECT ent_agent_avant_arrivee($1, $2)")
            .bind(user_id)
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ErreurAcceptation::Db(db(e)))?;
        tx.commit()
            .await
            .map_err(|e| ErreurAcceptation::Db(db(e)))?;
        let mut casquettes = vec!["personnel".to_string()];
        if parent {
            casquettes.push("parent".to_string());
        }
        Ok(InvitationAccepteeDto {
            resultat: if identite_existante { "identite_rattachee" } else { "compte_cree" }.into(),
            etablissement_id: etab,
            passkey_a_enroler: true,
            avant_arrivee: avant,
            casquettes,
        })
    }

    // -----------------------------------------------------------------------------------------
    // PRS-25 / PRS-26 : accueil
    // -----------------------------------------------------------------------------------------

    /// « Préparer mon arrivée » : la date d'arrivée et les collègues. Aucune donnée d'élève.
    pub async fn preparer_arrivee(
        &self,
        session: &CtxSession,
    ) -> Result<PreparerArriveeDto, EntDbError> {
        let etab = session.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (avant,): (bool,) = sqlx::query_as("SELECT ent_agent_avant_arrivee($1, $2)")
            .bind(session.user_id)
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let arrivee: Option<(NaiveDate,)> = sqlx::query_as(
            "SELECT min(e.du) FROM ent_personne p
               JOIN ent_agent a ON a.etablissement_id = p.etablissement_id AND a.personne_id = p.id
               JOIN ent_agent_engagement e ON e.etablissement_id = a.etablissement_id AND e.agent_id = a.id
              WHERE p.etablissement_id = $1 AND p.user_id = $2 AND e.annule_le IS NULL
                AND e.du > ent_aujourdhui_etab($1) HAVING min(e.du) IS NOT NULL",
        )
        .bind(etab)
        .bind(session.user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let collegues = crate::personnels::annuaire_tx(
            &mut tx,
            etab,
            crate::personnels::NiveauAnnuaire::Interne,
            &crate::personnels::FiltreAnnuaire::default(),
            jour,
        )
        .await?;
        Ok(PreparerArriveeDto {
            avant_arrivee: avant,
            arrivee_le: arrivee.map(|(d,)| d),
            collegues,
        })
    }

    /// PRS-26 : tâche quotidienne idempotente. Signale, une seule fois par invitation, l'arrivée
    /// des agents invités dont le premier engagement a commencé (`personnel.arrive`). L'ouverture
    /// elle-même n'attend pas cette tâche : la portée de la session est calculée à chaque requête.
    pub async fn signaler_arrivees_du_jour(&self, etab: Uuid) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, Uuid, Option<Uuid>)> = sqlx::query_as(
            "UPDATE ent_agent_invitation i SET arrivee_signalee_le = now()
              WHERE i.etablissement_id = $1 AND i.utilisee_le IS NOT NULL AND i.arrivee_signalee_le IS NULL
                AND EXISTS (SELECT 1 FROM ent_agent_engagement e
                             WHERE e.etablissement_id = i.etablissement_id AND e.agent_id = i.agent_id
                               AND e.annule_le IS NULL AND e.du <= ent_aujourdhui_etab($1)
                               AND (e.au IS NULL OR e.au >= ent_aujourdhui_etab($1)))
              RETURNING i.id, i.agent_id, i.user_id_rattache",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (invitation_id, agent_id, user) in &lignes {
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::PERSONNEL_ARRIVE.to_string(),
                aggregate: AggregateKind::Agent,
                aggregate_id: *agent_id,
                eleve_id: None,
                payload: json!({ "agent_id": agent_id, "invitation_id": invitation_id }),
                actor_user_id: user.map(|u| u.to_string()),
            })
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(lignes.len())
    }

    // -----------------------------------------------------------------------------------------
    // PRS-29 : casquettes
    // -----------------------------------------------------------------------------------------

    /// Casquettes que la session peut endosser. Lecture seule.
    pub async fn casquettes_de_la_session(
        &self,
        session: &CtxSession,
    ) -> Result<CasquettesDto, EntDbError> {
        let etab = session.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let profil: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etab)
        .bind(session.user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((profil,)) = profil else {
            return Err(EntDbError::Forbidden);
        };
        let mut v = vec![CasquetteDto {
            code: "principale".into(),
            libelle: libelle_casquette(&profil),
            active: session.casquette == "principale",
        }];
        if Role::parse(&profil).is_some_and(|r| r.is_staff())
            && casquette_parent_disponible_tx(&mut tx, etab, session.user_id).await?
        {
            v.push(CasquetteDto {
                code: "parent".into(),
                libelle: libelle_casquette(Role::Parent.as_str()),
                active: session.casquette == "parent",
            });
        }
        Ok(CasquettesDto { casquettes: v })
    }

    /// Bascule la casquette de la session sans nouvelle authentification. Les droits de la session
    /// deviennent exactement ceux de la casquette cible (jamais l'union). Une session limitée à
    /// l'enrôlement ou à l'arrivée ne bascule pas vers la casquette parent par ce chemin : seule
    /// une session complète le peut. Journalisé.
    pub async fn basculer_casquette(
        &self,
        session: &CtxSession,
        cible: &str,
    ) -> Result<CasquettesDto, EntDbError> {
        if !matches!(cible, "principale" | "parent") {
            return Err(invalide("Casquette inconnue."));
        }
        // La bascule ne se fait qu'au sein d'une session complète : une session d'enrôlement n'a
        // aucun droit, une session d'arrivée n'a pas encore de casquette parent utile.
        if session.portee == PorteeSession::Enrolement {
            return Err(EntDbError::Forbidden);
        }
        let etab = session.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if cible == "parent" {
            let profil: Option<(String,)> = sqlx::query_as(
                "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
            )
            .bind(etab)
            .bind(session.user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some((profil,)) = profil else {
                return Err(EntDbError::Forbidden);
            };
            // Un compte dont le profil EST « parent » n'a pas de seconde casquette.
            if !Role::parse(&profil).is_some_and(|r| r.is_staff())
                || !casquette_parent_disponible_tx(&mut tx, etab, session.user_id).await?
            {
                return Err(EntDbError::Forbidden);
            }
        }
        if session.casquette != cible {
            let (n,): (i64,) = sqlx::query_as(
                "WITH m AS (UPDATE ent_session SET casquette = $2
                             WHERE id = $1 AND user_id = $3 AND etablissement_id = $4 AND revoked_at IS NULL
                         RETURNING 1) SELECT count(*) FROM m",
            )
            .bind(session.session_id)
            .bind(cible)
            .bind(session.user_id)
            .bind(etab)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if n != 1 {
                return Err(EntDbError::Forbidden);
            }
            journal_tx(
                &mut tx,
                etab,
                "casquette_changee",
                "Casquette de la session changée",
                json!({ "de": session.casquette, "vers": cible }),
                session.user_id,
            )
            .await?;
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::SESSION_CASQUETTE_CHANGEE.to_string(),
                aggregate: AggregateKind::Agent,
                aggregate_id: session.user_id,
                eleve_id: None,
                payload: json!({ "de": session.casquette, "vers": cible }),
                actor_user_id: Some(session.user_id.to_string()),
            })
            .await?;
        }
        let profil: (String,) = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etab)
        .bind(session.user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut v = vec![CasquetteDto {
            code: "principale".into(),
            libelle: libelle_casquette(&profil.0),
            active: cible == "principale",
        }];
        if Role::parse(&profil.0).is_some_and(|r| r.is_staff())
            && casquette_parent_disponible_tx(&mut tx, etab, session.user_id).await?
        {
            v.push(CasquetteDto {
                code: "parent".into(),
                libelle: libelle_casquette(Role::Parent.as_str()),
                active: cible == "parent",
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(CasquettesDto { casquettes: v })
    }
}

fn libelle_casquette(profil: &str) -> String {
    match profil {
        "prof" => "Espace professeur",
        "parent" => "Espace parent",
        "eleve" => "Espace élève",
        "admin" | "direction" => "Espace direction",
        _ => "Espace personnel",
    }
    .to_string()
}

/// La casquette parent est disponible quand la ligne existe et est ouverte, ou quand le compte a un
/// lien de responsabilité actif dans l'établissement (la ligne est alors créée, idempotent).
/// Retourne `true` si la casquette parent est utilisable.
async fn casquette_parent_disponible_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
) -> Result<bool, EntDbError> {
    let (ouverte,): (bool,) = sqlx::query_as("SELECT ent_casquette_ouverte($1, $2, 'parent')")
        .bind(user_id)
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    if ouverte {
        return Ok(true);
    }
    // Ligne fermée : on ne la rouvre pas. Ligne absente : on l'ouvre si un lien actif existe.
    let (existe,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_membership_casquette
                         WHERE user_id = $1 AND etablissement_id = $2 AND casquette = 'parent')",
    )
    .bind(user_id)
    .bind(etab)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if existe {
        return Ok(false);
    }
    let personne: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(user_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((personne_id,)) = personne else {
        return Ok(false);
    };
    let jour = aujourdhui_tx(conn, etab).await?;
    ouvrir_casquette_parent_db(conn, etab, user_id, personne_id, jour, false).await
}

/// Ouvre la casquette parent si le compte a un lien de responsabilité actif ici ou a été parent
/// (déjà une ligne). Idempotent. `true` : la casquette parent existe et est ouverte.
async fn ouvrir_casquette_parent_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
    personne_id: Uuid,
    jour: NaiveDate,
    etait_parent: bool,
) -> Result<bool, ErreurAcceptation> {
    ouvrir_casquette_parent_db(conn, etab, user_id, personne_id, jour, etait_parent)
        .await
        .map_err(ErreurAcceptation::Db)
}

async fn ouvrir_casquette_parent_db(
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
    personne_id: Uuid,
    jour: NaiveDate,
    etait_parent: bool,
) -> Result<bool, EntDbError> {
    let (lien,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_actif WHERE etablissement_id = $1 AND user_id = $2)",
    )
    .bind(etab)
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !lien && !etait_parent {
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO ent_membership_casquette
            (user_id, etablissement_id, casquette, source_ref, du, passkey_requise)
         VALUES ($1,$2,'parent',$3,$4,false)
         ON CONFLICT (user_id, etablissement_id, casquette) DO NOTHING",
    )
    .bind(user_id)
    .bind(etab)
    .bind(personne_id)
    .bind(jour)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let (ouverte,): (bool,) = sqlx::query_as("SELECT ent_casquette_ouverte($1, $2, 'parent')")
        .bind(user_id)
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    Ok(ouverte)
}

#[cfg(test)]
mod tests_unitaires {
    use super::*;

    #[test]
    fn jeton_porte_l_etablissement_et_ne_se_relit_pas_sans_forme() {
        let etab = Uuid::new_v4();
        let (brut, empreinte) = nouveau_jeton(etab);
        assert_eq!(brut.len(), 96);
        let (e, h) = lire_jeton(&brut).expect("forme valide");
        assert_eq!(e, etab);
        assert_eq!(h, empreinte);
        assert_ne!(brut, empreinte, "l'empreinte n'est pas le jeton");
        for mauvais in ["", "abc", &brut[..95], &brut.to_uppercase(), &format!("{brut}0"), &"0".repeat(96)] {
            assert!(lire_jeton(mauvais).is_none(), "{mauvais}");
        }
    }

    #[test]
    fn adresse_masquee() {
        assert_eq!(masquer_email("jean.dupont@ecole.fr"), "j***@ecole.fr");
        assert_eq!(masquer_email("nimporte"), "***");
    }
}
