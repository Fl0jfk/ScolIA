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
use sqlx::PgConnection;
use uuid::Uuid;

use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::rate_limit::{Decision, Quota};
use crate::ref_commun::{db, introuvable};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

/// Déclarations d'individu par compte : 5 par jour (anti-énumération des adresses).
pub const QUOTA_DECLARATIONS: Quota = Quota::new(5, 86_400);

pub use ent_types::individu::{
    DeclarationRecue, LienIndividu, MonIndividu, MonLienIndividu, PropositionIndividu, MOTIFS_ECARTEE, MOTIFS_LEVEE,
    MOTIFS_LIEN,
};

/// Refus : l'individu concerné lève son propre lien.
pub const MSG_LEVEE_PAR_SOI: &str =
    "Un lien d'individu ne se lève pas par la personne concernée : un autre membre de la direction le lève.";
/// Nom affiché d'une fiche fermée au lecteur (conflit d'intérêts, adresse protégée).
pub const NOM_MASQUE: &str = "Fiche protégée";
/// Une coordonnée partagée par plus de fiches que cela n'est pas un indice d'individu (standard, remplissage d'import).
const PROPOSITION_VALEUR_MAX_FICHES: i64 = 5;
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
        if d.code().as_deref() == Some("23514") && d.message().contains("16 fiches") {
            return refus(RefusMetier::Conflit, "Un individu compte au plus 16 fiches reliées : vérifiez les liens existants.");
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

/// Le compte est-il le même individu que cet agent (sa fiche, ou une fiche reliée) ? Faux si l'agent n'existe pas.
pub(crate) async fn compte_est_agent(
    conn: &mut PgConnection,
    etab: Uuid,
    u: Uuid,
    agent_id: Uuid,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT COALESCE((SELECT ag.personne_id = ANY (public.ent_individu_personnes_du_compte($1, $2)) \
           FROM public.ent_agent ag WHERE ag.etablissement_id = $1 AND ag.id = $3), false)",
    )
    .bind(etab)
    .bind(u)
    .bind(agent_id)
    .fetch_one(conn)
    .await
    .map_err(db)
}

/// Deux agents sont-ils le même individu (leurs fiches reliées) ? `a1 == a2` compris.
pub(crate) async fn agents_meme_individu(
    conn: &mut PgConnection,
    etab: Uuid,
    a1: Uuid,
    a2: Uuid,
) -> Result<bool, EntDbError> {
    if a1 == a2 {
        return Ok(true);
    }
    sqlx::query_scalar(
        "SELECT COALESCE((SELECT b.personne_id = ANY (public.ent_individu_personnes($1, ARRAY[a.personne_id])) \
           FROM public.ent_agent a, public.ent_agent b \
          WHERE a.etablissement_id = $1 AND a.id = $2 AND b.etablissement_id = $1 AND b.id = $3), false)",
    )
    .bind(etab)
    .bind(a1)
    .bind(a2)
    .fetch_one(conn)
    .await
    .map_err(db)
}

// ───────────────────────── Vues ─────────────────────────

// ───────────────────────── Avis à l'intéressé ─────────────────────────
//
// Un lien posé ou levé change ce que la personne peut faire (des refus en plus, ou en moins) : elle en est prévenue
// sur **tous** ses comptes de l'établissement, dans l'ENT et par e-mail ou push selon ses préférences. Les textes ne
// nomment ni une fiche ni une personne (un lien posé à tort désignerait quelqu'un d'autre, et l'e-mail sort de l'ENT) ;
// le détail est dans « Compte › Mes autres comptes ». Le geste qui ne regarde que son auteur ne prévient personne : un
// refus de déclaration n'est jamais annoncé au déclarant (anti-énumération des adresses).

/// Avis : deux fiches de la personne ont été reliées.
pub const AVIS_LIEN_POSE: (&str, &str, &str) = (
    "individu_lien_pose",
    "Deux de vos fiches ont été reliées",
    "L'établissement a reconnu que deux de vos fiches ou deux de vos comptes désignent la même personne. Cela n'ouvre \
     aucun accès : certaines décisions qui vous concernent, vous ou vos enfants, seront désormais prises par une autre \
     personne. En cas d'erreur, signalez-le au secrétariat. Détail : Compte, Mes autres comptes.",
);
/// Avis : un lien entre deux fiches de la personne a été levé.
pub const AVIS_LIEN_LEVE: (&str, &str, &str) = (
    "individu_lien_leve",
    "Un lien entre vos fiches a été levé",
    "La direction a levé un lien entre deux fiches qui vous désignaient comme une seule personne. Détail : Compte, Mes \
     autres comptes.",
);
/// Avis : un autre compte déclare être aussi la personne.
pub const AVIS_DECLARATION_RECUE: (&str, &str, &str) = (
    "individu_declaration_recue",
    "Un autre compte déclare être aussi vous",
    "Un autre compte de l'établissement déclare appartenir à la même personne que vous. Confirmez ou refusez dans les \
     24 heures depuis Compte, Mes autres comptes. Si vous ne reconnaissez pas cette demande, refusez-la.",
);
/// Avis : la déclaration faite depuis ce compte a été confirmée depuis l'autre.
pub const AVIS_DECLARATION_CONFIRMEE: (&str, &str, &str) = (
    "individu_declaration_confirmee",
    "Votre autre compte est reconnu",
    "Votre déclaration a été confirmée depuis votre autre compte : l'établissement sait désormais que ces deux comptes \
     sont à la même personne. Détail : Compte, Mes autres comptes.",
);

/// Comptes de l'individu formé par ces fiches (fiches reliées, fusions, comptes d'élève), triés.
async fn comptes_des_fiches(conn: &mut PgConnection, etab: Uuid, fiches: &[Uuid]) -> Result<Vec<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "WITH f AS (SELECT unnest(public.ent_individu_personnes($1, $2)) AS id)
         SELECT DISTINCT u FROM (
             SELECT p.user_id AS u FROM public.ent_personne p JOIN f ON f.id = p.id
              WHERE p.etablissement_id = $1 AND p.user_id IS NOT NULL
             UNION
             SELECT ec.user_id FROM public.ent_personne p JOIN f ON f.id = p.id
               JOIN public.ent_eleve_compte ec ON ec.etablissement_id = p.etablissement_id AND ec.eleve_id = p.eleve_id
              WHERE p.etablissement_id = $1
         ) x ORDER BY u",
    )
    .bind(etab)
    .bind(fiches)
    .fetch_all(conn)
    .await
    .map_err(db)
}

/// Prévient chaque compte (sauf `sauf`) : notification dans l'ENT, e-mail ou push selon ses préférences. Idempotent
/// par geste (`geste_id` : le lien ou la déclaration).
async fn prevenir(
    conn: &mut PgConnection,
    etab: Uuid,
    comptes: &[Uuid],
    sauf: Option<Uuid>,
    avis: (&'static str, &'static str, &'static str),
    geste_id: Uuid,
) -> Result<(), EntDbError> {
    let (kind, titre, corps) = avis;
    let maintenant = Utc::now();
    for &u in comptes.iter().filter(|u| Some(**u) != sauf) {
        notifier_personne(
            &mut *conn,
            EnvoiAPlanifier {
                etablissement_id: etab,
                user_id: u,
                kind,
                titre,
                corps,
                eleve_id: None,
                notification_id: None,
                source_type: "systeme",
                source_id: geste_id,
                cle_dedup: format!("{kind}:{geste_id}"),
                droit_requis: None,
                regroupable: false,
                canaux_voulus: &["email", "push"],
            },
            geste_id,
            maintenant,
        )
        .await?;
    }
    Ok(())
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
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_individu_declaration (id, etablissement_id, declarant_user_id, cible_user_id, expire_le)
                 VALUES ($1, $2, $3, $4, now() + interval '24 hours')",
            )
            .bind(id)
            .bind(etab)
            .bind(actor)
            .bind(cible)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            // Seul le compte cible est prévenu : le déclarant n'apprend rien de plus que la réponse neutre.
            prevenir(&mut tx, etab, &[cible], None, AVIS_DECLARATION_RECUE, id).await?;
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
        // Les liens actifs entre ses fiches : quand et comment, jamais l'autre fiche (minimisation, §4 de la conception).
        let liens: Vec<(String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT l.origine, l.motif_code, l.pose_le FROM ent_individu_lien l
              WHERE l.etablissement_id = $1 AND l.leve_le IS NULL AND l.personne_a = ANY($2)
              ORDER BY l.pose_le DESC, l.id",
        )
        .bind(etab)
        .bind(&fiches)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(MonIndividu {
            liens: liens
                .into_iter()
                .map(|(origine, motif_code, pose_le)| MonLienIndividu { origine, motif_code, pose_le })
                .collect(),
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
        // La cible vient de confirmer elle-même ; le compte déclarant est prévenu (il a prouvé posséder les deux).
        prevenir(&mut tx, etab, &[declarant], None, AVIS_DECLARATION_CONFIRMEE, declaration_id).await?;
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
        // Tous les comptes de l'individu ainsi formé sont prévenus (sauf l'auteur, s'il relie ses propres fiches).
        let comptes = comptes_des_fiches(&mut tx, etab, &[a]).await?;
        prevenir(&mut tx, etab, &comptes, Some(actor), AVIS_LIEN_POSE, id).await?;
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
        // Les comptes à prévenir se calculent **avant** la levée : après, l'individu peut se scinder en deux.
        let comptes = comptes_des_fiches(&mut tx, etab, &[a, b]).await?;
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
        prevenir(&mut tx, etab, &comptes, Some(actor), AVIS_LIEN_LEVE, lien_id).await?;
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
        drop(tx);
        // Même minimisation que le dossier (R-A18, adresse protégée) : le nom d'une fiche fermée au lecteur est masqué.
        let ids: Vec<Uuid> = lignes.iter().flat_map(|l| [l.1, l.2]).collect();
        let fermees = self.personnes_en_conflit(etab, actor, role, &ids).await?;
        let nom = |id: Uuid, n: String| if fermees.contains(&id) { NOM_MASQUE.to_string() } else { n };
        Ok(lignes
            .into_iter()
            .map(|(id, personne_a, personne_b, nom_a, nom_b, origine, motif_code, pose_le)| LienIndividu {
                id,
                personne_a,
                personne_b,
                nom_a: nom(personne_a, nom_a),
                nom_b: nom(personne_b, nom_b),
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
             -- Une valeur partagée par plus de $2 fiches (standard, adresse de remplissage d'un import) ne propose rien :
             -- c'est une coordonnée commune, pas un indice d'individu, et cela borne le nombre de paires.
             cv AS (
                SELECT c.id, c.v, c.k FROM (
                    SELECT c.*, count(*) OVER (PARTITION BY c.k, c.v) AS n FROM c
                     WHERE c.v IS NOT NULL AND length(c.v) >= 6) c
                 WHERE c.n <= $2
             ),
             -- Un côté de la paire au moins est un agent : la jointure part des fiches d'agent.
             ag AS (
                SELECT cv.* FROM cv
                 WHERE EXISTS (SELECT 1 FROM ent_agent g WHERE g.etablissement_id = $1 AND g.personne_id = cv.id)
             ),
             paires AS (
                SELECT LEAST(x.id, y.id) AS a, GREATEST(x.id, y.id) AS b, array_agg(DISTINCT x.k ORDER BY x.k) AS raisons
                  FROM ag x JOIN cv y ON x.k = y.k AND x.v = y.v AND x.id <> y.id
                 GROUP BY 1, 2
             )
             SELECT q.a, q.b,
                    btrim(pa.nom || ' ' || COALESCE(pa.prenom, '')), btrim(pb.nom || ' ' || COALESCE(pb.prenom, '')),
                    q.raisons
               FROM paires q
               JOIN ent_personne pa ON pa.etablissement_id = $1 AND pa.id = q.a
               JOIN ent_personne pb ON pb.etablissement_id = $1 AND pb.id = q.b
              WHERE NOT (q.b = ANY(ent_individu_personnes($1, ARRAY[q.a])))
                AND NOT EXISTS (SELECT 1 FROM ent_individu_proposition_ecartee e
                                 WHERE e.etablissement_id = $1 AND e.personne_a = q.a AND e.personne_b = q.b)
              ORDER BY 3, 4, q.a, q.b
              LIMIT 200",
        )
        .bind(etab)
        .bind(PROPOSITION_VALEUR_MAX_FICHES)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        drop(tx);
        // Une fiche fermée au lecteur (R-A18, adresse protégée) n'est jamais proposée : ni son nom, ni le rapprochement.
        let ids: Vec<Uuid> = lignes.iter().flat_map(|l| [l.0, l.1]).collect();
        let fermees = self.personnes_en_conflit(etab, actor, role, &ids).await?;
        Ok(lignes
            .into_iter()
            .filter(|l| !fermees.contains(&l.0) && !fermees.contains(&l.1))
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
