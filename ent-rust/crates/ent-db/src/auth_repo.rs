use chrono::{DateTime, Duration, Utc};
use ent_auth::{
    assert_password_max_len, assert_password_policy, generate_session_token, hash_password, hash_token, normalize_email,
    verify_password, AuthError, Role,
};
use sqlx::PgPool;
use uuid::Uuid;

fn env_i64(nom: &str, defaut: i64) -> i64 {
    std::env::var(nom)
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|d| *d > 0)
        .unwrap_or(defaut)
}

/// Web (D-011) : inactivité maximale, en minutes (défaut 120).
fn session_web_inactivite_minutes() -> i64 {
    env_i64("ENT_SESSION_WEB_INACTIVITE_MINUTES", 120)
}

/// Web (D-011) : durée maximale depuis la connexion, en heures (défaut 12).
fn session_web_max_heures() -> i64 {
    env_i64("ENT_SESSION_WEB_MAX_HEURES", 12)
}

/// Mobile (D-011) : durée glissante, en jours (défaut 90), prolongée à l'usage.
fn session_mobile_jours() -> i64 {
    env_i64("ENT_SESSION_MOBILE_JOURS", 90)
}

/// Mobile : plafond absolu depuis la connexion, en jours (défaut 365).
fn session_mobile_max_jours() -> i64 {
    env_i64("ENT_SESSION_MOBILE_MAX_JOURS", 365)
}

/// Session « enrôlement seulement » : courte (défaut 15 minutes).
fn session_enrolement_minutes() -> i64 {
    env_i64("ENT_SESSION_ENROLEMENT_MINUTES", 15)
}

/// Type de client (D-011) : durées différentes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClientSession {
    #[default]
    Web,
    Mobile,
}

impl ClientSession {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Mobile => "mobile",
        }
    }
    /// Valeur fournie par le client à la connexion ; tout inconnu → web (le plus court).
    pub fn parse_client(v: Option<&str>) -> Self {
        match v.map(|s| s.trim().to_ascii_lowercase()) {
            Some(s) if s == "mobile" => Self::Mobile,
            _ => Self::Web,
        }
    }
}

/// Niveau d'assurance de l'authentification qui a ouvert la session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AssuranceSession {
    #[default]
    MotDePasse,
    Passkey,
    Sso,
}

impl AssuranceSession {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MotDePasse => "mot_de_passe",
            Self::Passkey => "passkey",
            Self::Sso => "sso",
        }
    }
    fn from_db(v: &str) -> Self {
        match v {
            "passkey" => Self::Passkey,
            "sso" => Self::Sso,
            _ => Self::MotDePasse,
        }
    }
}

/// Portée de la session : complète, ou limitée à l'enrôlement d'une passkey
/// (compte soumis à l'obligation sans passkey, D-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PorteeSession {
    #[default]
    Complete,
    Enrolement,
}

impl PorteeSession {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Enrolement => "enrolement",
        }
    }
    fn from_db(v: &str) -> Self {
        if v == "enrolement" {
            Self::Enrolement
        } else {
            Self::Complete
        }
    }
}

/// Paramètres d'émission d'une session.
#[derive(Debug, Clone, Default)]
pub struct OptionsSession {
    pub client: ClientSession,
    pub assurance: AssuranceSession,
    pub portee: PorteeSession,
    pub appareil: Option<String>,
    pub ip: Option<String>,
}

/// Session listée pour l'écran « appareils connectés » (jamais le jeton).
#[derive(Debug, Clone)]
pub struct SessionAppareil {
    pub session_id: Uuid,
    pub etablissement_id: Uuid,
    pub client: String,
    pub assurance: String,
    pub appareil: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Nombre d'échecs de connexion avant verrouillage temporaire.
fn login_max_failed() -> i32 {
    std::env::var("ENT_LOGIN_MAX_FAILED")
        .ok()
        .and_then(|v| v.trim().parse::<i32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(5)
}

/// Échecs de ré-authentification (mot de passe actuel d'une session ouverte)
/// avant verrouillage de ces seules routes. Compteur distinct du login.
fn reauth_max_failed() -> i32 {
    std::env::var("ENT_REAUTH_MAX_FAILED")
        .ok()
        .and_then(|v| v.trim().parse::<i32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(5)
}

/// Seuil d'échecs par **adresse IP** (défense en profondeur : freine le
/// bourrage d'identifiants réparti sur de nombreux comptes). Volontairement
/// plus haut que le seuil par compte : une école derrière un NAT partage une IP.
fn login_max_failed_ip() -> i32 {
    std::env::var("ENT_LOGIN_MAX_FAILED_IP")
        .ok()
        .and_then(|v| v.trim().parse::<i32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(50)
}

/// Fenêtre glissante de comptage des échecs (minutes).
fn login_window_minutes() -> i64 {
    std::env::var("ENT_LOGIN_WINDOW_MINUTES")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(15)
}

/// Durée du verrouillage après trop d'échecs (minutes).
fn login_lock_minutes() -> i64 {
    std::env::var("ENT_LOGIN_LOCK_MINUTES")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(15)
}

fn env_u64(nom: &str, defaut: u64) -> u64 {
    std::env::var(nom)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(defaut)
}

/// Ralentissement progressif par compte (toutes IP confondues) : au-delà de
/// `ENT_LOGIN_SLOWDOWN_AFTER` échecs nets dans la fenêtre (défaut 5), chaque
/// tentative est retardée de `ENT_LOGIN_SLOWDOWN_BASE_MS` (défaut 200 ms)
/// doublé à chaque échec supplémentaire, plafonné à `ENT_LOGIN_SLOWDOWN_MAX_MS`
/// (défaut 3000 ms). Retarde l'attaque distribuée sans jamais refuser un
/// utilisateur légitime.
fn slowdown_ms(echecs_comptes: i32) -> u64 {
    slowdown_calcul(
        echecs_comptes,
        env_u64("ENT_LOGIN_SLOWDOWN_AFTER", 5) as i32,
        env_u64("ENT_LOGIN_SLOWDOWN_BASE_MS", 200),
        env_u64("ENT_LOGIN_SLOWDOWN_MAX_MS", 3000),
    )
}

fn slowdown_calcul(echecs: i32, apres: i32, base_ms: u64, max_ms: u64) -> u64 {
    if echecs <= apres || base_ms == 0 {
        return 0;
    }
    let exposant = (echecs - apres - 1).clamp(0, 16) as u32;
    base_ms.saturating_mul(1u64 << exposant).min(max_ms)
}

/// Résultat d'une tentative comptabilisée par `login_attempt`.
struct Tentative {
    /// Tentatives nettes dans la fenêtre, celle-ci comprise.
    compte: i32,
    verrouille: bool,
}

/// Hash factice (généré une fois) : on vérifie un mot de passe même quand
/// l'e-mail est inconnu, pour lisser le temps de réponse (anti-énumération).
static DUMMY_PASSWORD_HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    hash_password("mot-de-passe-factice-anti-enumeration")
        .unwrap_or_else(|_| "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0$0000000000000000000000000000000000000000000".into())
});

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
}

/// Membre staff d’un établissement (annuaire / pickers RH·EDT).
#[derive(Debug, Clone)]
pub struct StaffMember {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    // Pas de téléphone : l'annuaire du personnel n'expose que des coordonnées professionnelles
    // (e-mail de l'ENT). Le téléphone personnel reste hors annuaire (socle 6 P0, §2.2 n° 12).
    pub role: String,
}

#[derive(Clone)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub etablissement_id: Uuid,
    pub email: String,
    pub role: String,
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub client: ClientSession,
    pub assurance: AssuranceSession,
    pub portee: PorteeSession,
}

/// Identité vérifiée après contrôle du mot de passe et de l'appartenance à
/// l'établissement, **avant** toute émission de session. Permet d'insérer une
/// étape de 2ᵉ facteur (passkey) sans dupliquer la logique de garde.
#[derive(Debug, Clone)]
pub struct LoginIdentity {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub role: String,
}

pub struct PgAuthRepo {
    pool: PgPool,
}

impl PgAuthRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn register(
        &self,
        email: &str,
        password: &str,
        display_name: Option<&str>,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<AuthUser, AuthError> {
        let email = normalize_email(email)?;
        assert_password_policy(password)?;
        if etablissement_id.is_nil() {
            return Err(AuthError::NotMember);
        }
        let role = Role::parse(role).ok_or(AuthError::NotMember)?.as_str();

        let password_hash = hash_password(password)?;
        let user_id = Uuid::new_v4();

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        let insert = sqlx::query(
            r#"
            INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, now(), now())
            "#,
        )
        .bind(user_id)
        .bind(&email)
        .bind(&password_hash)
        .bind(display_name)
        .execute(&mut *tx)
        .await;

        if let Err(e) = insert {
            if let sqlx::Error::Database(db) = &e {
                if db.constraint() == Some("ent_user_email_unique") {
                    return Err(AuthError::EmailTaken);
                }
            }
            return Err(AuthError::InvalidCredentials);
        }

        sqlx::query(
            r#"
            INSERT INTO ent_membership (user_id, etablissement_id, role, created_at)
            VALUES ($1, $2, $3, now())
            "#,
        )
        .bind(user_id)
        .bind(etablissement_id)
        .bind(role)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        tx.commit()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        Ok(AuthUser {
            id: user_id,
            email,
            display_name: display_name.map(|s| s.to_string()),
        })
    }

    /// Contrôle mot de passe + appartenance, **sans** créer de session.
    /// C'est le point d'entrée pour un parcours à 2ᵉ facteur (passkey).
    ///
    /// Anti-force brute (trois compteurs, tous incrémentés **avant** la
    /// vérification du mot de passe, de façon atomique) :
    /// - verrou dur par couple **(compte, IP)** : un tiers qui martèle un compte
    ///   depuis son IP ne bloque que lui-même, jamais la victime (M1) ;
    /// - verrou dur par **IP** (bourrage d'identifiants réparti sur des comptes) ;
    /// - **ralentissement progressif** par compte, toutes IP confondues
    ///   (`slowdown_ms`) : freine l'attaque distribuée sans jamais refuser la
    ///   victime.
    ///
    /// `NotMember` n'est jamais exposé : mot de passe correct mais hors
    /// établissement = mêmes `InvalidCredentials` qu'un mauvais mot de passe
    /// (la cause réelle est seulement journalisée).
    pub async fn verify_credentials(
        &self,
        email: &str,
        password: &str,
        etablissement_id: Uuid,
        client_ip: Option<&str>,
    ) -> Result<LoginIdentity, AuthError> {
        let email = normalize_email(email)?;
        // Borne de taille avant tout hachage Argon2.
        assert_password_max_len(password)?;
        if etablissement_id.is_nil() {
            tracing::info!("connexion refusée : établissement nul");
            return Err(AuthError::InvalidCredentials);
        }

        // Clés de garde, toutes préfixées (`email:`, `ip:`, `compte-ip:`). Revue finale
        // socle 1 : la clé globale était l'e-mail brut ; un « e-mail » forgé
        // `compte-ip:victime@x|<ip>` (accepté par `normalize_email`) incrémentait alors
        // la clé dure (compte, IP) d'une victime depuis n'importe quelle IP.
        let ip_key = client_ip.map(|ip| format!("ip:{ip}"));
        let hard_key = format!("compte-ip:{email}|{}", client_ip.unwrap_or("-"));
        let global_key = format!("email:{email}");

        if self
            .login_attempt(&hard_key, login_max_failed())
            .await?
            .verrouille
        {
            return Err(AuthError::TooManyAttempts);
        }
        if let Some(k) = &ip_key {
            if self.login_attempt(k, login_max_failed_ip()).await?.verrouille {
                return Err(AuthError::TooManyAttempts);
            }
        }
        // Compteur global du compte (aucun seuil de refus) → ralentissement.
        let global = self.login_attempt(&global_key, i32::MAX).await?;
        let delai = slowdown_ms(global.compte);
        if delai > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delai)).await;
        }

        let row: Option<(Uuid, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, password_hash, display_name FROM ent_user WHERE email = $1
            "#,
        )
        .bind(&email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        let Some((user_id, password_hash, display_name)) = row else {
            // E-mail inconnu : on hache quand même un mot de passe factice pour
            // éviter de révéler l'existence du compte par le temps de réponse.
            let _ = verify_password(password, DUMMY_PASSWORD_HASH.as_str());
            return Err(AuthError::InvalidCredentials);
        };

        if !verify_password(password, &password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }

        let membership: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT role FROM ent_membership
            WHERE user_id = $1 AND etablissement_id = $2
              AND etat = 'ouvert'
              AND (date_fin IS NULL OR date_fin >= ent_aujourdhui_etab(etablissement_id))
            "#,
        )
        .bind(user_id)
        .bind(etablissement_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        let Some((role,)) = membership else {
            // Mot de passe correct mais hors établissement : réponse identique à
            // un échec (ne confirme pas le mot de passe), tentative non rendue.
            tracing::info!(
                user_id = %user_id,
                etablissement_id = %etablissement_id,
                "connexion refusée : mot de passe correct mais compte non membre de l'établissement"
            );
            return Err(AuthError::InvalidCredentials);
        };

        // Connexion réussie : le verrou (compte, IP) repart à zéro, et les
        // tentatives réussies sont rendues aux compteurs IP et compte (qui ne
        // reflètent que les échecs : un succès isolé ne rend pas son budget à
        // un attaquant qui partage l'IP, il ne fait que compenser sa propre
        // tentative).
        self.clear_login_failures(&hard_key).await?;
        if let Some(k) = &ip_key {
            self.login_attempt_refund(k).await?;
        }
        self.login_attempt_refund(&global_key).await?;

        Ok(LoginIdentity {
            user_id,
            email,
            display_name,
            role,
        })
    }

    /// Émet une session pour une identité **déjà vérifiée** (mot de passe ou
    /// passkey). Centralise la création du token et sa persistance.
    ///
    /// Durées (D-011) : web = `ENT_SESSION_WEB_INACTIVITE_MINUTES` d'inactivité et
    /// `ENT_SESSION_WEB_MAX_HEURES` au plus ; mobile = `ENT_SESSION_MOBILE_JOURS`
    /// glissants (plafond `ENT_SESSION_MOBILE_MAX_JOURS`) ; enrôlement = courte.
    pub async fn issue_session(
        &self,
        user_id: Uuid,
        etablissement_id: Uuid,
        email: &str,
        role: &str,
        opts: &OptionsSession,
    ) -> Result<SessionInfo, AuthError> {
        // Socle 6 P0 : quel que soit le chemin (mot de passe, passkey, SSO), aucune session ne
        // s'ouvre sur un accès suspendu, fermé ou échu.
        if !self.acces_ouvert(etablissement_id, user_id).await? {
            tracing::info!(
                user_id = %user_id,
                etablissement_id = %etablissement_id,
                "ouverture de session refusée : accès non ouvert"
            );
            return Err(AuthError::NotMember);
        }
        let (raw_token, token_hash) = generate_session_token();
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let (expires_at, inactivite_max_secs): (DateTime<Utc>, Option<i32>) =
            match (opts.portee, opts.client) {
                (PorteeSession::Enrolement, _) => (
                    now + Duration::minutes(session_enrolement_minutes()),
                    None,
                ),
                (_, ClientSession::Web) => (
                    now + Duration::hours(session_web_max_heures()),
                    Some((session_web_inactivite_minutes() * 60).min(i32::MAX as i64) as i32),
                ),
                (_, ClientSession::Mobile) => (now + Duration::days(session_mobile_jours()), None),
            };
        let appareil = opts
            .appareil
            .as_deref()
            .map(|a| a.chars().filter(|c| !c.is_control()).take(120).collect::<String>());
        let ip = opts.ip.as_deref().map(|a| a.chars().take(64).collect::<String>());

        sqlx::query(
            r#"
            INSERT INTO ent_session (
                id, user_id, etablissement_id, token_hash, expires_at, created_at,
                client, last_seen_at, inactivite_max_secs, assurance, portee, appareil, ip_creation
            ) VALUES ($1,$2,$3,$4,$5, now(), $6, now(), $7, $8, $9, $10, $11)
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .bind(etablissement_id)
        .bind(&token_hash)
        .bind(expires_at)
        .bind(opts.client.as_str())
        .bind(inactivite_max_secs)
        .bind(opts.assurance.as_str())
        .bind(opts.portee.as_str())
        .bind(appareil)
        .bind(ip)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        Ok(SessionInfo {
            session_id,
            user_id,
            etablissement_id,
            email: email.to_string(),
            role: role.to_string(),
            token: raw_token,
            expires_at,
            client: opts.client,
            assurance: opts.assurance,
            portee: opts.portee,
        })
    }

    /// Élève une session « enrôlement » en session complète (après l'enrôlement
    /// d'une passkey). Sans effet sur une session déjà complète.
    pub async fn completer_session_apres_enrolement(
        &self,
        session_id: Uuid,
    ) -> Result<(), AuthError> {
        let inactivite = (session_web_inactivite_minutes() * 60).min(i32::MAX as i64) as i32;
        sqlx::query(
            r#"
            UPDATE ent_session
               SET portee = 'complete',
                   expires_at = CASE client
                       WHEN 'mobile' THEN now() + make_interval(days => $2)
                       ELSE now() + make_interval(hours => $3) END,
                   inactivite_max_secs = CASE client WHEN 'mobile' THEN NULL ELSE $4 END,
                   last_seen_at = now()
             WHERE id = $1 AND portee = 'enrolement' AND revoked_at IS NULL
            "#,
        )
        .bind(session_id)
        .bind(session_mobile_jours() as i32)
        .bind(session_web_max_heures() as i32)
        .bind(inactivite)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;
        Ok(())
    }

    /// Sessions actives d'une personne (écran « appareils connectés »).
    pub async fn list_sessions_actives(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<SessionAppareil>, AuthError> {
        let rows: Vec<(
            Uuid,
            Uuid,
            String,
            String,
            Option<String>,
            DateTime<Utc>,
            DateTime<Utc>,
            DateTime<Utc>,
        )> = sqlx::query_as(
            r#"
            SELECT id, etablissement_id, client, assurance, appareil,
                   created_at, last_seen_at, expires_at
              FROM ent_session
             WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now()
               AND (inactivite_max_secs IS NULL
                    OR last_seen_at > now() - make_interval(secs => inactivite_max_secs))
             ORDER BY last_seen_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;
        Ok(rows
            .into_iter()
            .map(
                |(session_id, etablissement_id, client, assurance, appareil, created_at, last_seen_at, expires_at)| {
                    SessionAppareil {
                        session_id,
                        etablissement_id,
                        client,
                        assurance,
                        appareil,
                        created_at,
                        last_seen_at,
                        expires_at,
                    }
                },
            )
            .collect())
    }

    /// Révoque une session précise de la personne (« déconnecter cet appareil »).
    pub async fn revoke_session_of_user(
        &self,
        user_id: Uuid,
        session_id: Uuid,
    ) -> Result<bool, AuthError> {
        let res = sqlx::query(
            r#"UPDATE ent_session SET revoked_at = now()
                WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL"#,
        )
        .bind(session_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;
        Ok(res.rows_affected() > 0)
    }

    /// Comptabilise une tentative de façon **atomique et avant** la vérification
    /// du mot de passe (une seule requête `INSERT … ON CONFLICT … DO UPDATE …
    /// RETURNING`) : plusieurs requêtes concurrentes ne peuvent pas toutes
    /// passer sous le seuil (course contrôle/enregistrement corrigée). Au-delà de
    /// `max_failed` tentatives nettes dans la fenêtre, la clé est verrouillée
    /// (`login_lock_minutes`) et la tentative refusée. Un succès rend son
    /// décompte (`login_attempt_refund`) ou efface la clé : le compteur reflète
    /// donc les **échecs**. Sans seuil (`i32::MAX`) : simple compteur.
    /// Le compteur vit en base : la politique est partagée entre instances.
    async fn login_attempt(&self, key: &str, max_failed: i32) -> Result<Tentative, AuthError> {
        let (compte, verrouille): (i32, bool) = sqlx::query_as(
            r#"
            INSERT INTO ent_login_guard AS g (
                email, failed_count, window_start, locked_until, last_attempt_at
            ) VALUES ($1, 1, now(), NULL, now())
            ON CONFLICT (email) DO UPDATE SET
                failed_count = CASE
                    WHEN (g.locked_until IS NOT NULL AND g.locked_until <= now())
                      OR (g.locked_until IS NULL
                          AND g.window_start < now() - make_interval(secs => $3::float8))
                    THEN 1 ELSE g.failed_count + 1 END,
                window_start = CASE
                    WHEN (g.locked_until IS NOT NULL AND g.locked_until <= now())
                      OR (g.locked_until IS NULL
                          AND g.window_start < now() - make_interval(secs => $3::float8))
                    THEN now() ELSE g.window_start END,
                locked_until = CASE
                    WHEN (g.locked_until IS NOT NULL AND g.locked_until <= now())
                      OR (g.locked_until IS NULL
                          AND g.window_start < now() - make_interval(secs => $3::float8))
                    THEN NULL
                    WHEN g.failed_count + 1 > $2
                    THEN COALESCE(g.locked_until, now() + make_interval(secs => $4::float8))
                    ELSE g.locked_until END,
                last_attempt_at = now()
            RETURNING failed_count, (failed_count > $2)
            "#,
        )
        .bind(key)
        .bind(max_failed)
        .bind((login_window_minutes() * 60) as f64)
        .bind((login_lock_minutes() * 60) as f64)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        // Ménage opportuniste : au-delà d'un jour, un compteur n'a plus d'usage.
        if compte == 1 {
            let _ = sqlx::query(
                "DELETE FROM ent_login_guard WHERE last_attempt_at < now() - interval '1 day'",
            )
            .execute(&self.pool)
            .await;
        }
        Ok(Tentative { compte, verrouille })
    }

    /// Rend le décompte d'une tentative **réussie** (le compteur ne reflète que
    /// les échecs). Sans effet si la clé est verrouillée.
    async fn login_attempt_refund(&self, key: &str) -> Result<(), AuthError> {
        // Une ligne revenue à zéro est supprimée (pas de résidu par connexion
        // réussie) ; sinon le compteur est décrémenté. Une seule requête.
        sqlx::query(
            "WITH purge AS ( \
                 DELETE FROM ent_login_guard \
                 WHERE email = $1 AND locked_until IS NULL AND failed_count <= 1 \
                 RETURNING 1 \
             ) \
             UPDATE ent_login_guard SET failed_count = GREATEST(failed_count - 1, 0) \
             WHERE email = $1 AND locked_until IS NULL \
               AND NOT EXISTS (SELECT 1 FROM purge)",
        )
        .bind(key)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(())
    }

    async fn clear_login_failures(&self, email: &str) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM ent_login_guard WHERE email = $1")
            .bind(email)
            .execute(&self.pool)
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(())
    }

    /// L'accès de cette personne à cet établissement est-il **ouvert aujourd'hui** (état `ouvert`,
    /// date de fin non dépassée dans le fuseau de l'établissement) ? Source unique : la fonction SQL
    /// `ent_acces_ouvert` (migration 350).
    pub async fn acces_ouvert(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, AuthError> {
        let (ok,): (bool,) = sqlx::query_as("SELECT ent_acces_ouvert($1, $2)")
            .bind(user_id)
            .bind(etablissement_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(ok)
    }

    /// Rôle d'un utilisateur dans un établissement (`None` s'il n'est pas membre).
    ///
    /// Renvoie le profil même si l'accès est suspendu ou fermé : c'est une lecture d'appartenance
    /// (cible d'une commande d'administration). Ne jamais s'en servir pour autoriser une connexion
    /// ou une action au nom de la personne : voir [`Self::acces_ouvert`].
    pub async fn role_in(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, AuthError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE user_id = $1 AND etablissement_id = $2",
        )
        .bind(user_id)
        .bind(etablissement_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(r,)| r))
    }

    /// Écrit directement un nouveau mot de passe (réinitialisation par un tiers).
    pub async fn set_password(&self, user_id: Uuid, new_password: &str) -> Result<(), AuthError> {
        assert_password_policy(new_password)?;
        let hash = hash_password(new_password)?;
        let res =
            sqlx::query("UPDATE ent_user SET password_hash = $2, updated_at = now() WHERE id = $1")
                .bind(user_id)
                .bind(&hash)
                .execute(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;
        if res.rows_affected() == 0 {
            return Err(AuthError::InvalidCredentials);
        }
        Ok(())
    }

    /// E-mail d'un utilisateur (audit, libellés).
    pub async fn resolve_user_email(&self, user_id: Uuid) -> Result<String, AuthError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT email FROM ent_user WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;
        row.map(|(email,)| email).ok_or(AuthError::InvalidCredentials)
    }

    /// E-mail + nom affiché d'un utilisateur (enrôlement passkey : le nom
    /// affiché apparaît dans le sélecteur natif de l'appareil).
    pub async fn user_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<(String, Option<String>)>, AuthError> {
        let row: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT email, display_name FROM ent_user WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row)
    }

    /// Purge des sessions expirées (maintenance, au démarrage).
    pub async fn purge_expired_sessions(&self) -> Result<u64, AuthError> {
        let res = sqlx::query("DELETE FROM ent_session WHERE expires_at < now()")
            .execute(&self.pool)
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(res.rows_affected())
    }

    pub async fn resolve_bearer(&self, authorization: &str) -> Result<SessionInfo, AuthError> {
        let raw = authorization
            .strip_prefix("Bearer ")
            .or_else(|| authorization.strip_prefix("bearer "))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or(AuthError::InvalidSession)?;

        let token_hash = hash_token(raw);
        let now = Utc::now();

        type Ligne = (
            Uuid,
            Uuid,
            Uuid,
            String,
            String,
            chrono::DateTime<Utc>,
            String,
            String,
            String,
            chrono::DateTime<Utc>,
            chrono::DateTime<Utc>,
        );
        // Inactivité (web) contrôlée en base : un jeton inutilisé depuis plus
        // longtemps que `inactivite_max_secs` est refusé, quel que soit le porteur.
        let row: Option<Ligne> = sqlx::query_as(
            r#"
                SELECT s.id, s.user_id, s.etablissement_id, u.email, m.role, s.expires_at,
                       s.client, s.assurance, s.portee, s.last_seen_at, s.created_at
                FROM ent_session s
                JOIN ent_user u ON u.id = s.user_id
                JOIN ent_membership m
                  ON m.user_id = s.user_id AND m.etablissement_id = s.etablissement_id
                 -- Socle 6 P0 : un accès suspendu, fermé ou échu invalide aussi les sessions et
                 -- jetons déjà ouverts, sans attendre leur révocation explicite.
                 AND m.etat = 'ouvert'
                 AND (m.date_fin IS NULL OR m.date_fin >= ent_aujourdhui_etab(m.etablissement_id))
                WHERE s.token_hash = $1
                  AND s.revoked_at IS NULL
                  AND s.expires_at > $2
                  AND (s.inactivite_max_secs IS NULL
                       OR s.last_seen_at > $2 - make_interval(secs => s.inactivite_max_secs))
                "#,
        )
        .bind(&token_hash)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;

        let Some((
            session_id,
            user_id,
            etablissement_id,
            email,
            role,
            mut expires_at,
            client,
            assurance,
            portee,
            last_seen_at,
            created_at,
        )) = row
        else {
            return Err(AuthError::InvalidSession);
        };
        let client = ClientSession::parse_client(Some(&client));
        let portee = PorteeSession::from_db(&portee);

        // Activité : écriture amortie (au plus une fois par minute). Mobile :
        // prolongation glissante, plafonnée depuis la connexion.
        if now - last_seen_at > Duration::seconds(60) {
            if client == ClientSession::Mobile && portee == PorteeSession::Complete {
                let plafond = created_at + Duration::days(session_mobile_max_jours());
                let nouvelle = (now + Duration::days(session_mobile_jours())).min(plafond);
                if nouvelle > expires_at {
                    expires_at = nouvelle;
                }
            }
            let _ = sqlx::query(
                r#"UPDATE ent_session SET last_seen_at = $2, expires_at = $3
                    WHERE id = $1 AND revoked_at IS NULL"#,
            )
            .bind(session_id)
            .bind(now)
            .bind(expires_at)
            .execute(&self.pool)
            .await;
        }

        Ok(SessionInfo {
            session_id,
            user_id,
            etablissement_id,
            email,
            role,
            token: String::new(),
            expires_at,
            client,
            assurance: AssuranceSession::from_db(&assurance),
            portee,
        })
    }

    /// Premier membre staff dont le rôle est dans la liste (ordre = priorité).
    pub async fn first_member_with_roles(
        &self,
        etablissement_id: Uuid,
        roles: &[&str],
    ) -> Result<Option<Uuid>, AuthError> {
        if roles.is_empty() {
            return Ok(None);
        }
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT user_id FROM ent_membership
            WHERE etablissement_id = $1 AND role = ANY($2)
            ORDER BY array_position($2::text[], role), created_at
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(roles)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(id,)| id))
    }

    /// Membres staff dont le rôle est dans la liste (sans doublon user_id).
    pub async fn list_user_ids_with_roles(
        &self,
        etablissement_id: Uuid,
        roles: &[&str],
    ) -> Result<Vec<Uuid>, AuthError> {
        if roles.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT user_id FROM ent_membership
            WHERE etablissement_id = $1 AND role = ANY($2)
            "#,
        )
        .bind(etablissement_id)
        .bind(roles)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Membre **avec un accès ouvert** : un accès suspendu, fermé ou échu n'est pas un membre actif.
    pub async fn is_member(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, AuthError> {
        let row: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT 1
            FROM ent_membership
            WHERE etablissement_id = $1 AND user_id = $2
              AND etat = 'ouvert'
              AND (date_fin IS NULL OR date_fin >= ent_aujourdhui_etab(etablissement_id))
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.is_some())
    }

    /// Lookup membre établissement par e-mail (import Pronote / annuaire).
    pub async fn find_member_id_by_email(
        &self,
        etablissement_id: Uuid,
        email: &str,
    ) -> Result<Option<Uuid>, AuthError> {
        let email = normalize_email(email)?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT u.id
            FROM ent_user u
            INNER JOIN ent_membership m
              ON m.user_id = u.id AND m.etablissement_id = $1
            WHERE u.email = $2
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(&email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn set_telephone(
        &self,
        user_id: Uuid,
        telephone: &str,
    ) -> Result<(), AuthError> {
        let tel = telephone.trim();
        let tel = if tel.is_empty() { None } else { Some(tel) };
        sqlx::query(
            r#"
            UPDATE ent_user SET telephone = $2, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(tel)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(())
    }

    /// Vrai si une personne portant cette adresse est membre de l'établissement.
    pub async fn email_membre_de(
        &self,
        etablissement_id: Uuid,
        email: &str,
    ) -> Result<bool, AuthError> {
        let email = normalize_email(email)?;
        let r: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT u.id FROM ent_user u
            JOIN ent_membership m ON m.user_id = u.id AND m.etablissement_id = $2
            WHERE u.email = $1
            "#,
        )
        .bind(&email)
        .bind(etablissement_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(r.is_some())
    }

    pub async fn set_email(&self, user_id: Uuid, email: &str) -> Result<(), AuthError> {
        let email = normalize_email(email)?;
        let updated = sqlx::query(
            r#"
            UPDATE ent_user SET email = $2, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(&email)
        .execute(&self.pool)
        .await;
        match updated {
            Ok(r) if r.rows_affected() > 0 => Ok(()),
            Ok(_) => Err(AuthError::InvalidCredentials),
            Err(e) => {
                if let sqlx::Error::Database(db) = &e {
                    if db.constraint() == Some("ent_user_email_unique") {
                        return Err(AuthError::EmailTaken);
                    }
                }
                Err(AuthError::InvalidCredentials)
            }
        }
    }

    pub async fn revoke_bearer(&self, authorization: &str) -> Result<(), AuthError> {
        let raw = authorization
            .strip_prefix("Bearer ")
            .or_else(|| authorization.strip_prefix("bearer "))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or(AuthError::InvalidSession)?;
        let token_hash = hash_token(raw);
        sqlx::query(
            r#"
            UPDATE ent_session SET revoked_at = now()
            WHERE token_hash = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(&token_hash)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;
        Ok(())
    }

    /// Change le mot de passe d'un utilisateur après vérification de l'actuel.
    /// Révoque ensuite toutes les sessions sauf celle indiquée (si fournie).
    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), AuthError> {
        assert_password_policy(new_password)?;

        // Revue finale socle 1 : même compteur de ré-authentification que les autres
        // routes sensibles (`reauth:<user_id>`, 5 échecs / 15 min) ; avant, une session
        // volée pouvait deviner le mot de passe actuel sans limite.
        if !self.verify_current_password(user_id, current_password).await? {
            return Err(AuthError::InvalidCredentials);
        }

        let new_hash = hash_password(new_password)?;
        sqlx::query("UPDATE ent_user SET password_hash = $2, updated_at = now() WHERE id = $1")
            .bind(user_id)
            .bind(&new_hash)
            .execute(&self.pool)
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        Ok(())
    }

    /// Révoque toutes les sessions actives d'un utilisateur sauf une (optionnelle).
    /// Renvoie le nombre de sessions révoquées.
    pub async fn revoke_other_sessions_for_user(
        &self,
        user_id: Uuid,
        keep_session_id: Option<Uuid>,
    ) -> Result<u64, AuthError> {
        let res = sqlx::query(
            r#"
            UPDATE ent_session SET revoked_at = now()
            WHERE user_id = $1 AND revoked_at IS NULL
              AND ($2::uuid IS NULL OR id <> $2)
            "#,
        )
        .bind(user_id)
        .bind(keep_session_id)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;
        Ok(res.rows_affected())
    }

    /// Révoque toutes les sessions actives d'un utilisateur (sortie / radiation élève).
    /// Renvoie le nombre de sessions révoquées.
    pub async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<u64, AuthError> {
        let res = sqlx::query(
            r#"
            UPDATE ent_session SET revoked_at = now()
            WHERE user_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;
        Ok(res.rows_affected())
    }

    // ---- Réinitialisation de mot de passe (jeton à usage unique) ----

    /// Retrouve l'utilisateur par e-mail normalisé (insensible à la casse).
    pub async fn find_user_id_by_email(&self, email: &str) -> Result<Option<Uuid>, AuthError> {
        let email = normalize_email(email)?;
        let row: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_user WHERE email = $1")
                .bind(&email)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(id,)| id))
    }

    /// Crée un jeton de réinitialisation. Invalide d'abord les jetons non utilisés
    /// du même utilisateur (un seul lien actif).
    pub async fn create_password_reset(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), AuthError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        sqlx::query(
            "UPDATE ent_password_reset SET used_at = now()
             WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        sqlx::query(
            "INSERT INTO ent_password_reset (id, user_id, token_hash, expires_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        tx.commit()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(())
    }

    /// Consomme un jeton (usage unique, non expiré) et applique le nouveau mot de
    /// passe, en révoquant **toutes** les sessions de l'utilisateur. Atomique :
    /// un jeton rejoué ne peut pas passer deux fois.
    pub async fn consume_password_reset(
        &self,
        token_hash: &str,
        new_password: &str,
    ) -> Result<Uuid, AuthError> {
        assert_password_policy(new_password)?;
        let new_hash = hash_password(new_password)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_password_reset SET used_at = now()
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
               AND NOT EXISTS (SELECT 1 FROM ent_user u
                               WHERE u.id = ent_password_reset.user_id
                                 AND u.recuperation_gel_jusqu > now())
             RETURNING user_id",
        )
        .bind(token_hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        let Some((user_id,)) = row else {
            return Err(AuthError::ResetTokenInvalide);
        };
        // Les autres liens encore actifs du même compte meurent avec celui-ci.
        sqlx::query(
            "UPDATE ent_password_reset SET used_at = now()
             WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        sqlx::query("UPDATE ent_user SET password_hash = $2, updated_at = now() WHERE id = $1")
            .bind(user_id)
            .bind(&new_hash)
            .execute(&mut *tx)
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        sqlx::query(
            "UPDATE ent_session SET revoked_at = now()
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        tx.commit()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(user_id)
    }

    /// Invalide tous les jetons de réinitialisation encore actifs d'un compte
    /// (changement d'e-mail, réinitialisation par le personnel, reset passkeys).
    pub async fn invalidate_pending_resets(&self, user_id: Uuid) -> Result<u64, AuthError> {
        let res = sqlx::query(
            "UPDATE ent_password_reset SET used_at = now()
             WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(res.rows_affected())
    }

    /// Vérifie le mot de passe actuel d'un utilisateur **déjà authentifié**
    /// (ré-authentification d'une action sensible) sans toucher au compteur de
    /// CONNEXION : un mauvais mot de passe n'incrémente que le compteur dédié
    /// `reauth:<user_id>` (même table de garde, clé distincte d'un e-mail), qui
    /// verrouille ces seules routes (`TooManyAttempts`), jamais le login.
    pub async fn verify_current_password(
        &self,
        user_id: Uuid,
        password: &str,
    ) -> Result<bool, AuthError> {
        let key = format!("reauth:{user_id}");
        assert_password_max_len(password)?;
        // Tentative comptabilisée atomiquement AVANT la vérification.
        if self
            .login_attempt(&key, reauth_max_failed())
            .await?
            .verrouille
        {
            return Err(AuthError::TooManyAttempts);
        }
        let row: Option<(String,)> =
            sqlx::query_as("SELECT password_hash FROM ent_user WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;
        let Some((hash,)) = row else {
            let _ = verify_password(password, DUMMY_PASSWORD_HASH.as_str());
            return Ok(false);
        };
        if verify_password(password, &hash)? {
            self.clear_login_failures(&key).await?;
            Ok(true)
        } else {
            // Échec déjà comptabilisé (incrément avant vérification).
            Ok(false)
        }
    }

    /// Purge les jetons de réinitialisation expirés ou consommés (> 1 jour).
    pub async fn prune_password_resets(&self) -> Result<u64, AuthError> {
        let res = sqlx::query(
            "DELETE FROM ent_password_reset
             WHERE (used_at IS NOT NULL OR expires_at < now())
               AND created_at < now() - interval '1 day'",
        )
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(res.rows_affected())
    }

    /// Établissements + rôles d'un utilisateur (audit d'une réinit. par jeton).
    pub async fn list_memberships_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>, AuthError> {
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT etablissement_id, role FROM ent_membership WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(rows)
    }

    /// Résout des libellés d’affichage pour une liste d’utilisateurs (display_name ou e-mail).
    pub async fn user_labels(
        &self,
        etablissement_id: Uuid,
        user_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, AuthError> {
        if user_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT u.id, u.email, u.display_name
            FROM ent_user u
            JOIN ent_membership m
              ON m.user_id = u.id AND m.etablissement_id = $2
            WHERE u.id = ANY($1)
            "#,
        )
        .bind(user_ids)
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(rows
            .into_iter()
            .map(|(id, email, display_name)| {
                let label = display_name
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(email);
                (id, label)
            })
            .collect())
    }

    pub async fn get_staff_member(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<StaffMember>, AuthError> {
        let row: Option<(Uuid, String, Option<String>, String)> = sqlx::query_as(
            r#"
            SELECT u.id, u.email, u.display_name, m.role
            FROM ent_user u
            INNER JOIN ent_membership m
              ON m.user_id = u.id AND m.etablissement_id = $1
            WHERE u.id = $2
              AND m.role NOT IN ('parent', 'eleve')
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(user_id, email, display_name, role)| StaffMember {
            user_id,
            email,
            display_name,
            role,
        }))
    }

    /// Annuaire staff (hors parent / élève) — recherche e-mail, nom affiché ou rôle.
    pub async fn search_staff(
        &self,
        etablissement_id: Uuid,
        q: &str,
        limit: i64,
    ) -> Result<Vec<StaffMember>, AuthError> {
        let needle = q.trim();
        let pattern = if needle.is_empty() {
            "%".to_string()
        } else {
            format!("%{}%", needle)
        };
        let limit = limit.clamp(1, 100);
        let rows: Vec<(Uuid, String, Option<String>, String)> = sqlx::query_as(
            r#"
            SELECT u.id, u.email, u.display_name, m.role
            FROM ent_user u
            INNER JOIN ent_membership m
              ON m.user_id = u.id AND m.etablissement_id = $1
            WHERE m.role NOT IN ('parent', 'eleve')
              AND m.etat = 'ouvert'
              AND (m.date_fin IS NULL OR m.date_fin >= ent_aujourdhui_etab(m.etablissement_id))
              AND (
                u.email ILIKE $2
                OR COALESCE(u.display_name, '') ILIKE $2
                OR m.role ILIKE $2
              )
            ORDER BY m.role, COALESCE(u.display_name, u.email)
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(rows
            .into_iter()
            .map(|(user_id, email, display_name, role)| StaffMember {
                user_id,
                email,
                display_name,
                role,
            })
            .collect())
    }
}

// `Debug` masqué : jamais de jeton / e-mail dans un `{:?}`.
impl std::fmt::Debug for SessionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionInfo")
            .field("session_id", &self.session_id)
            .field("user_id", &self.user_id)
            .field("etablissement_id", &self.etablissement_id)
            .field("role", &self.role)
            .field("expires_at", &self.expires_at)
            .field("email", &"[masqué]")
            .field("token", &"[masqué]")
            .finish()
    }
}

#[cfg(test)]
mod tests_ralentissement {
    use super::slowdown_calcul;

    #[test]
    fn ralentissement_progressif_plafonne() {
        // Aucun délai jusqu'au seuil.
        assert_eq!(slowdown_calcul(1, 5, 200, 3000), 0);
        assert_eq!(slowdown_calcul(5, 5, 200, 3000), 0);
        // Puis doublement à chaque échec, plafonné.
        assert_eq!(slowdown_calcul(6, 5, 200, 3000), 200);
        assert_eq!(slowdown_calcul(7, 5, 200, 3000), 400);
        assert_eq!(slowdown_calcul(8, 5, 200, 3000), 800);
        assert_eq!(slowdown_calcul(10, 5, 200, 3000), 3000);
        assert_eq!(slowdown_calcul(i32::MAX, 5, 200, 3000), 3000);
        // Désactivable.
        assert_eq!(slowdown_calcul(50, 5, 0, 3000), 0);
    }
}
