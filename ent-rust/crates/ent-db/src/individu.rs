//! Transverse « la deuxième personne fondée sur la personne physique » (docs/socles/TRANSVERSE_INDIVIDU.md, migration 279).
//!
//! Un compte a au plus une fiche par établissement ; un même humain à deux comptes (ou un agent et une fiche parent sans
//! compte) a deux fiches. Ce module les **relie** (jamais de fusion, D-013) et sert **la seule définition** de « même
//! individu » aux autres socles :
//!
//! * [`meme_individu`], [`comptes_de`] : ports pour les règles « une autre personne » et « jamais pour soi », dans la
//!   transaction de l'appelant. Ils appellent les fonctions SQL `ent_meme_individu` / `ent_individu_comptes`, celles
//!   qu'utilisent aussi les gardes de la base.
//! * commandes : déclaration en deux temps (depuis un compte, confirmée depuis l'autre), lien posé par la direction ou
//!   le secrétariat (`personnes.individu.relier`), levée (`personnes.individu.lever`, direction, **jamais l'individu
//!   concerné**), propositions sur coordonnées partagées (jamais un rapprochement automatique), propositions écartées.
//!
//! Principe : l'individu **ne fait que restreindre**. Aucune règle ne s'en sert pour accorder un droit.

use chrono::{DateTime, Utc};
use ent_auth::Action;
use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::rate_limit::{Decision, Quota};
use crate::ref_commun::{db, introuvable};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

/// Déclarations d'individu par compte : 5 par jour (anti-énumération des adresses).
pub const QUOTA_DECLARATIONS: Quota = Quota::new(5, 86_400);

/// Motifs codés d'un lien posé par la direction ou le secrétariat.
pub const MOTIFS_LIEN: [&str; 4] = ["deux_comptes", "fiche_sans_compte", "doublon_agent", "autre"];
/// Motifs codés d'une levée.
pub const MOTIFS_LEVEE: [&str; 3] = ["erreur", "homonyme", "autre"];
/// Motifs codés d'une proposition écartée.
pub const MOTIFS_ECARTEE: [&str; 3] = ["homonyme", "coordonnee_partagee", "autre"];

/// Refus : l'individu concerné lève son propre lien.
pub const MSG_LEVEE_PAR_SOI: &str =
    "Un lien d'individu ne se lève pas par la personne concernée : un autre membre de la direction le lève.";
/// Refus : les deux fiches sont déjà le même individu.
pub const MSG_DEJA_RELIEES: &str = "Ces deux fiches sont déjà reliées : elles désignent le même individu.";

fn refus(nature: RefusMetier, m: &'static str) -> EntDbError {
    EntDbError::Refus(nature, m)
}

fn db_individu(err: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &err {
        if d.code().as_deref() == Some("23505") {
            return refus(RefusMetier::Conflit, MSG_DEJA_RELIEES);
        }
        if d.code().as_deref() == Some("23514") && d.message().contains("expirée") {
            return refus(RefusMetier::Conflit, "Cette déclaration a expiré : déclarez de nouveau.");
        }
    }
    db(err)
}

// ───────────────────────── Ports pour les autres socles ─────────────────────────

/// Deux comptes sont-ils le même individu dans cet établissement ? (`u1 == u2` compris.) À utiliser **à la place** d'une
/// égalité de comptes dans toute règle qui **refuse** (« une autre personne », « jamais pour soi »), jamais pour accorder.
pub(crate) async fn meme_individu(
    conn: &mut PgConnection,
    etab: Uuid,
    u1: Uuid,
    u2: Uuid,
) -> Result<bool, EntDbError> {
    if u1 == u2 {
        return Ok(true);
    }
    sqlx::query_scalar("SELECT public.ent_meme_individu($1, $2, $3)")
        .bind(etab)
        .bind(u1)
        .bind(u2)
        .fetch_one(conn)
        .await
        .map_err(db)
}

/// Variante d'un compte facultatif (auteur inconnu d'une ligne ancienne) : faux si l'un manque.
pub(crate) async fn meme_individu_opt(
    conn: &mut PgConnection,
    etab: Uuid,
    u1: Option<Uuid>,
    u2: Uuid,
) -> Result<bool, EntDbError> {
    match u1 {
        Some(u) => meme_individu(conn, etab, u, u2).await,
        None => Ok(false),
    }
}

/// Tous les comptes de l'individu d'un compte (le compte compris).
pub(crate) async fn comptes_de(conn: &mut PgConnection, etab: Uuid, u: Uuid) -> Result<Vec<Uuid>, EntDbError> {
    sqlx::query_scalar("SELECT public.ent_individu_comptes($1, $2)")
        .bind(etab)
        .bind(u)
        .fetch_one(conn)
        .await
        .map_err(db)
}

/// Fiches de l'individu d'un compte.
pub(crate) async fn personnes_du_compte(
    conn: &mut PgConnection,
    etab: Uuid,
    u: Uuid,
) -> Result<Vec<Uuid>, EntDbError> {
    sqlx::query_scalar("SELECT public.ent_individu_personnes_du_compte($1, $2)")
        .bind(etab)
        .bind(u)
        .fetch_one(conn)
        .await
        .map_err(db)
}

// ───────────────────────── Vues ─────────────────────────

/// Un lien d'individu, vu de la direction ou du secrétariat.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LienIndividu {
    pub id: Uuid,
    pub personne_a: Uuid,
    pub personne_b: Uuid,
    pub nom_a: String,
    pub nom_b: String,
    pub origine: String,
    pub motif_code: String,
    pub pose_le: DateTime<Utc>,
}

/// Une déclaration reçue, en attente de confirmation par le compte cible.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeclarationRecue {
    pub id: Uuid,
    /// Nom affiché du compte déclarant (la personne qui déclare se fait connaître de son autre compte).
    pub declarant: String,
    pub declare_le: DateTime<Utc>,
    pub expire_le: DateTime<Utc>,
}

/// Ce que le compte voit de son individu : le nombre de ses autres comptes reconnus et ses déclarations reçues.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MonIndividu {
    pub autres_comptes: i64,
    pub fiches_reliees: i64,
    pub declarations_recues: Vec<DeclarationRecue>,
}

/// Une ressemblance entre deux fiches (une au moins est un agent). Jamais la coordonnée elle-même.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PropositionIndividu {
    pub personne_a: Uuid,
    pub personne_b: Uuid,
    pub nom_a: String,
    pub nom_b: String,
    /// `email`, `telephone`.
    pub raisons: Vec<String>,
}

fn paire(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

// ───────────────────────── Commandes ─────────────────────────

impl PgEnt {
    async fn individu_exiger(&self, etab: Uuid, actor: Uuid, role: &str, action: Action) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, actor, role, action).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Déclare « ce compte est aussi moi » (adresse e-mail du compte cible). Réponse identique que l'adresse soit
    /// connue ou non de l'établissement (anti-énumération) ; 5 déclarations par jour.
    pub async fn individu_declarer_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        email_cible: &str,
    ) -> Result<(), EntDbError> {
        let email = email_cible.trim().to_lowercase();
        if email.is_empty() || email.len() > 254 || !email.contains('@') {
            return Err(refus(RefusMetier::Invalide, "Adresse e-mail du compte invalide."));
        }
        if let Decision::Limite { .. } = self
            .limiteur_debit()
            .verifier("individu.declarer", "user", &actor.to_string(), QUOTA_DECLARATIONS)
            .await
        {
            return Err(refus(RefusMetier::Conflit, "Trop de déclarations aujourd'hui : réessayez demain."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Le compte cible : membre de cet établissement (n'importe quelle casquette), autre que soi.
        let cible: Option<Uuid> = sqlx::query_scalar(
            "SELECT u.id FROM ent_user u
              WHERE lower(u.email) = $2
                AND (EXISTS (SELECT 1 FROM ent_membership m WHERE m.user_id = u.id AND m.etablissement_id = $1)
                     OR EXISTS (SELECT 1 FROM ent_personne p WHERE p.etablissement_id = $1 AND p.user_id = u.id)
                     OR EXISTS (SELECT 1 FROM ent_eleve_compte ec WHERE ec.etablissement_id = $1 AND ec.user_id = u.id))",
        )
        .bind(etab)
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some(cible) = cible.filter(|c| *c != actor) else {
            return Ok(());
        };
        if meme_individu(&mut tx, etab, actor, cible).await? {
            return Ok(());
        }
        let en_attente: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_individu_declaration
                             WHERE etablissement_id = $1 AND declarant_user_id = $2 AND cible_user_id = $3
                               AND issue IS NULL AND expire_le > now())",
        )
        .bind(etab)
        .bind(actor)
        .bind(cible)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !en_attente {
            sqlx::query(
                "INSERT INTO ent_individu_declaration (id, etablissement_id, declarant_user_id, cible_user_id, expire_le)
                 VALUES ($1, $2, $3, $4, now() + interval '24 hours')",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(actor)
            .bind(cible)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)
    }

    /// Ce que le compte voit de son individu.
    pub async fn individu_moi_for_actor(&self, etab: Uuid, actor: Uuid) -> Result<MonIndividu, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let comptes = comptes_de(&mut tx, etab, actor).await?;
        let fiches = personnes_du_compte(&mut tx, etab, actor).await?;
        let recues: Vec<(Uuid, Option<String>, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT d.id, COALESCE(NULLIF(btrim(u.display_name), ''), 'Un autre compte'), d.declare_le, d.expire_le
               FROM ent_individu_declaration d
               LEFT JOIN ent_user u ON u.id = d.declarant_user_id
              WHERE d.etablissement_id = $1 AND d.cible_user_id = $2 AND d.issue IS NULL AND d.expire_le > now()
              ORDER BY d.declare_le",
        )
        .bind(etab)
        .bind(actor)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(MonIndividu {
            autres_comptes: i64::try_from(comptes.len().saturating_sub(1)).unwrap_or(0),
            fiches_reliees: i64::try_from(fiches.len()).unwrap_or(0),
            declarations_recues: recues
                .into_iter()
                .map(|(id, d, l, e)| DeclarationRecue {
                    id,
                    declarant: d.unwrap_or_default(),
                    declare_le: l,
                    expire_le: e,
                })
                .collect(),
        })
    }

    /// Confirme (`confirmer = true`) ou refuse une déclaration reçue, **depuis le compte cible**. La confirmation relie
    /// la fiche du déclarant et celle de la cible.
    pub async fn individu_repondre_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        declaration_id: Uuid,
        confirmer: bool,
    ) -> Result<Option<Uuid>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d: Option<(Uuid, Uuid, Option<String>, DateTime<Utc>, bool)> = sqlx::query_as(
            "SELECT declarant_user_id, cible_user_id, issue, expire_le, expire_le <= now()
               FROM ent_individu_declaration WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(declaration_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        // Une déclaration qui ne vise pas ce compte n'existe pas pour lui (pas d'oracle).
        let Some((declarant, cible, issue, _, expiree)) = d.filter(|x| x.1 == actor) else {
            return Err(introuvable());
        };
        if issue.is_some() {
            return Err(refus(RefusMetier::Conflit, "Cette déclaration a déjà reçu une réponse."));
        }
        if !confirmer {
            sqlx::query("UPDATE ent_individu_declaration SET issue = 'refusee', issue_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(declaration_id)
                .execute(&mut *tx)
                .await
                .map_err(db_individu)?;
            tx.commit().await.map_err(db)?;
            return Ok(None);
        }
        if expiree {
            return Err(refus(RefusMetier::Conflit, "Cette déclaration a expiré : déclarez de nouveau."));
        }
        let fiche = |u: Uuid| {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2",
            )
            .bind(etab)
            .bind(u)
        };
        let fa: Option<Uuid> = fiche(declarant).fetch_optional(&mut *tx).await.map_err(db)?;
        let fb: Option<Uuid> = fiche(cible).fetch_optional(&mut *tx).await.map_err(db)?;
        let (Some(fa), Some(fb)) = (fa, fb) else {
            return Err(refus(
                RefusMetier::Conflit,
                "L'un des deux comptes n'a pas de fiche dans l'établissement : le secrétariat relie les fiches.",
            ));
        };
        let lien = if meme_individu(&mut tx, etab, declarant, cible).await? {
            None
        } else {
            let (a, b) = paire(fa, fb);
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_individu_lien
                    (id, etablissement_id, personne_a, personne_b, origine, motif_code, pose_par_user_id, declaration_id)
                 VALUES ($1, $2, $3, $4, 'declaration_confirmee', 'deux_comptes', $5, $6)",
            )
            .bind(id)
            .bind(etab)
            .bind(a)
            .bind(b)
            .bind(actor)
            .bind(declaration_id)
            .execute(&mut *tx)
            .await
            .map_err(db_individu)?;
            Some(id)
        };
        sqlx::query(
            "UPDATE ent_individu_declaration SET issue = 'confirmee', issue_le = now(), lien_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(declaration_id)
        .bind(lien)
        .execute(&mut *tx)
        .await
        .map_err(db_individu)?;
        tx.commit().await.map_err(db)?;
        Ok(lien)
    }

    /// Relie deux fiches du même humain (direction, secrétariat). `depuis_proposition` : la paire vient des propositions.
    #[allow(clippy::too_many_arguments)] // acteur, rôle, deux fiches, motif, origine : tous distincts
    pub async fn individu_relier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_a: Uuid,
        personne_b: Uuid,
        motif_code: &str,
        depuis_proposition: bool,
    ) -> Result<Uuid, EntDbError> {
        self.individu_exiger(etab, actor, role, Action::PersonnesIndividuRelier).await?;
        if !MOTIFS_LIEN.contains(&motif_code) {
            return Err(refus(RefusMetier::Invalide, "Motif de lien inconnu."));
        }
        if personne_a == personne_b {
            return Err(refus(RefusMetier::Invalide, "Choisissez deux fiches différentes."));
        }
        let (a, b) = paire(personne_a, personne_b);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND id IN ($2, $3)",
        )
        .bind(etab)
        .bind(a)
        .bind(b)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n != 2 {
            return Err(introuvable());
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_individu_lien
                (id, etablissement_id, personne_a, personne_b, origine, motif_code, pose_par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(etab)
        .bind(a)
        .bind(b)
        .bind(if depuis_proposition { "proposition_acceptee" } else { "declaration_direction" })
        .bind(motif_code)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db_individu)?;
        tx.commit().await.map_err(db)?;
        Ok(id)
    }

    /// Lève un lien (direction). Jamais par l'individu concerné : lever retire des refus.
    pub async fn individu_lever_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        lien_id: Uuid,
        motif_code: &str,
    ) -> Result<(), EntDbError> {
        self.individu_exiger(etab, actor, role, Action::PersonnesIndividuLever).await?;
        if !MOTIFS_LEVEE.contains(&motif_code) {
            return Err(refus(RefusMetier::Invalide, "Motif de levée inconnu."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('individu:' || $1::text, 0))")
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let l: Option<(Uuid, Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT personne_a, personne_b, leve_le FROM ent_individu_lien
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(lien_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (a, b, leve) = l.ok_or_else(introuvable)?;
        if leve.is_some() {
            return Err(refus(RefusMetier::Conflit, "Ce lien est déjà levé."));
        }
        // L'individu de l'acteur, lien compris : s'il contient l'une des deux fiches, c'est son propre lien.
        let miennes = personnes_du_compte(&mut tx, etab, actor).await?;
        if miennes.contains(&a) || miennes.contains(&b) {
            return Err(refus(RefusMetier::Interdit, MSG_LEVEE_PAR_SOI));
        }
        sqlx::query(
            "UPDATE ent_individu_lien SET leve_le = now(), leve_par_user_id = $3, leve_motif_code = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lien_id)
        .bind(actor)
        .bind(motif_code)
        .execute(&mut *tx)
        .await
        .map_err(db_individu)?;
        tx.commit().await.map_err(db)
    }

    /// Liens actifs de l'établissement (direction, secrétariat).
    pub async fn individu_liens_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<LienIndividu>, EntDbError> {
        self.individu_exiger(etab, actor, role, Action::PersonnesIndividuRelier).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, Uuid, Uuid, String, String, String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT l.id, l.personne_a, l.personne_b,
                    btrim(pa.nom || ' ' || COALESCE(pa.prenom, '')), btrim(pb.nom || ' ' || COALESCE(pb.prenom, '')),
                    l.origine, l.motif_code, l.pose_le
               FROM ent_individu_lien l
               JOIN ent_personne pa ON pa.etablissement_id = l.etablissement_id AND pa.id = l.personne_a
               JOIN ent_personne pb ON pb.etablissement_id = l.etablissement_id AND pb.id = l.personne_b
              WHERE l.etablissement_id = $1 AND l.leve_le IS NULL
              ORDER BY l.pose_le DESC, l.id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(|(id, personne_a, personne_b, nom_a, nom_b, origine, motif_code, pose_le)| LienIndividu {
                id,
                personne_a,
                personne_b,
                nom_a,
                nom_b,
                origine,
                motif_code,
                pose_le,
            })
            .collect())
    }

    /// Propositions : paires de fiches (l'une au moins est un agent) qui partagent une adresse e-mail (fiche ou compte)
    /// ou un téléphone, pas déjà le même individu, jamais écartées. Au plus 200. Jamais la coordonnée elle-même.
    pub async fn individu_propositions_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<PropositionIndividu>, EntDbError> {
        self.individu_exiger(etab, actor, role, Action::PersonnesIndividuRelier).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<(Uuid, Uuid, String, String, Vec<String>)> = sqlx::query_as(
            "WITH c AS (
                SELECT p.id, lower(btrim(p.email)) AS v, 'email' AS k
                  FROM ent_personne p
                 WHERE p.etablissement_id = $1 AND p.fusionnee_dans IS NULL AND p.email IS NOT NULL
                UNION
                SELECT p.id, lower(btrim(u.email)), 'email'
                  FROM ent_personne p JOIN ent_user u ON u.id = p.user_id
                 WHERE p.etablissement_id = $1 AND p.fusionnee_dans IS NULL
                UNION
                SELECT p.id, ent_tel_e164(t.tel), 'telephone'
                  FROM ent_personne p
                  CROSS JOIN LATERAL (VALUES (p.telephone), (p.telephone_2)) AS t(tel)
                 WHERE p.etablissement_id = $1 AND p.fusionnee_dans IS NULL AND t.tel IS NOT NULL
             ),
             paires AS (
                SELECT x.id AS a, y.id AS b, array_agg(DISTINCT x.k ORDER BY x.k) AS raisons
                  FROM c x JOIN c y ON x.k = y.k AND x.v = y.v AND x.id < y.id
                 WHERE x.v IS NOT NULL AND length(x.v) >= 6
                 GROUP BY x.id, y.id
             )
             SELECT q.a, q.b,
                    btrim(pa.nom || ' ' || COALESCE(pa.prenom, '')), btrim(pb.nom || ' ' || COALESCE(pb.prenom, '')),
                    q.raisons
               FROM paires q
               JOIN ent_personne pa ON pa.etablissement_id = $1 AND pa.id = q.a
               JOIN ent_personne pb ON pb.etablissement_id = $1 AND pb.id = q.b
              WHERE (EXISTS (SELECT 1 FROM ent_agent g WHERE g.etablissement_id = $1 AND g.personne_id IN (q.a, q.b)))
                AND NOT (q.b = ANY(ent_individu_personnes($1, ARRAY[q.a])))
                AND NOT EXISTS (SELECT 1 FROM ent_individu_proposition_ecartee e
                                 WHERE e.etablissement_id = $1 AND e.personne_a = q.a AND e.personne_b = q.b)
              ORDER BY 3, 4, q.a, q.b
              LIMIT 200",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(|(personne_a, personne_b, nom_a, nom_b, raisons)| PropositionIndividu {
                personne_a,
                personne_b,
                nom_a,
                nom_b,
                raisons,
            })
            .collect())
    }

    /// Écarte une proposition : la paire ne sera plus proposée.
    pub async fn individu_ecarter_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_a: Uuid,
        personne_b: Uuid,
        motif_code: &str,
    ) -> Result<(), EntDbError> {
        self.individu_exiger(etab, actor, role, Action::PersonnesIndividuRelier).await?;
        if !MOTIFS_ECARTEE.contains(&motif_code) {
            return Err(refus(RefusMetier::Invalide, "Motif inconnu."));
        }
        if personne_a == personne_b {
            return Err(refus(RefusMetier::Invalide, "Choisissez deux fiches différentes."));
        }
        let (a, b) = paire(personne_a, personne_b);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_personne WHERE etablissement_id = $1 AND id IN ($2, $3)",
        )
        .bind(etab)
        .bind(a)
        .bind(b)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n != 2 {
            return Err(introuvable());
        }
        sqlx::query(
            "INSERT INTO ent_individu_proposition_ecartee (id, etablissement_id, personne_a, personne_b, motif_code, par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (etablissement_id, personne_a, personne_b) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(a)
        .bind(b)
        .bind(motif_code)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)
    }
}
