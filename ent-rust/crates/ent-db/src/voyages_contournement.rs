//! Socle 42, lot V3b : **contournement tracé** de la règle « sortie entière » (contre-revue des suites, Moyen 3, D-069).
//!
//! Les acomptes fournisseurs (création, paiement, annulation) et l'annulation collective d'avoirs engagent la sortie
//! entière : la personne qui les accomplit n'est liée à **aucun** participant ni payeuse d'aucun de ses comptes (M-3 de
//! V3). La règle reste **stricte** (l'intégrité prime sur la disponibilité) ; mais quand la seule personne habilitée est
//! liée à un participant, la sortie ne doit pas rester bloquée **ni** être débloquée en silence :
//!
//! 1. la personne liée **demande** un contournement (action visée, motif obligatoire) ;
//! 2. une **seconde personne**, distincte, habilitée (`voyages.argent`) et **libre** de tout lien avec la sortie, l'**autorise**
//!    (ou le refuse, avec motif) ;
//! 3. la demandeuse l'**utilise**, **une seule fois**, pour **une seule** écriture (la cible est tracée).
//!
//! Trois faits en ajout seul (`ent_sortie_argent_contournement`, `…_evt`), sans nom d'élève ni montant, jamais dans le
//! journal de l'organisateur. La base garde le contournement : un acompte d'une personne liée n'est accepté que si un
//! contournement autorisé par une autre personne libre a été utilisé pour cette écriture (`ent_sortie_contournement_valide`).
//! Une demande expire après [`DELAI_CONTOURNEMENT_JOURS`] jours (demande non décidée, ou autorisation non utilisée).
//! L'annulation collective n'a pas de garde de base (elle ne fait que *proposer* des avoirs) : le contrôle est applicatif.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::voyages_argent_repo::{peut_argent, verrou_sortie};
use crate::voyages_argent_v3::{est_refus_regle3, exiger_libre_de_la_sortie};
use crate::voyages_repo::{charger_avec_acces, interdit, refus, texte, tx};

/// Actions qu'un contournement peut couvrir.
pub const ACTIONS_CONTOURNEMENT: [&str; 4] = [
    "acompte_creer",
    "acompte_payer",
    "acompte_annuler",
    "annulation_collective",
];

/// Une demande non décidée, ou une autorisation non utilisée, expire après ce délai.
pub const DELAI_CONTOURNEMENT_JOURS: i64 = 30;

/// Refus : la demandeuse ne décide pas de sa propre demande (tracé comme les refus de la règle 3).
pub(crate) const MSG_DEMANDEUSE_DECIDE: &str =
    "Une seconde personne autorise : la demandeuse ne décide pas de sa propre demande.";

/// Vue d'un contournement pour le personnel habilité (`voyages.argent`).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ContournementVue {
    pub id: Uuid,
    pub action: String,
    pub motif: String,
    pub demande_le: DateTime<Utc>,
    /// `en_attente`, `autorise`, `refuse`, `utilise` ou `expire`.
    pub statut: String,
    #[serde(skip)]
    pub demande_par_user_id: Uuid,
    #[serde(skip)]
    pub decide_par_user_id: Option<Uuid>,
}

const LECTURE: &str = "SELECT c.id, c.action, c.motif, c.demande_le, c.demande_par_user_id,
        (SELECT e.par_user_id FROM ent_sortie_argent_contournement_evt e
          WHERE e.etablissement_id = c.etablissement_id AND e.contournement_id = c.id
            AND e.evenement IN ('autorise', 'refuse')) AS decide_par_user_id,
        CASE
          WHEN EXISTS (SELECT 1 FROM ent_sortie_argent_contournement_evt e
                        WHERE e.etablissement_id = c.etablissement_id AND e.contournement_id = c.id
                          AND e.evenement = 'utilise') THEN 'utilise'
          WHEN EXISTS (SELECT 1 FROM ent_sortie_argent_contournement_evt e
                        WHERE e.etablissement_id = c.etablissement_id AND e.contournement_id = c.id
                          AND e.evenement = 'refuse') THEN 'refuse'
          WHEN EXISTS (SELECT 1 FROM ent_sortie_argent_contournement_evt e
                        WHERE e.etablissement_id = c.etablissement_id AND e.contournement_id = c.id
                          AND e.evenement = 'autorise' AND e.le + make_interval(days => $3) >= $4) THEN 'autorise'
          WHEN EXISTS (SELECT 1 FROM ent_sortie_argent_contournement_evt e
                        WHERE e.etablissement_id = c.etablissement_id AND e.contournement_id = c.id
                          AND e.evenement = 'autorise') THEN 'expire'
          WHEN c.demande_le + make_interval(days => $3) >= $4 THEN 'en_attente'
          ELSE 'expire'
        END AS statut
   FROM ent_sortie_argent_contournement c";

async fn lire(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    id: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<Option<ContournementVue>, EntDbError> {
    sqlx::query_as(&format!(
        "{LECTURE} WHERE c.etablissement_id = $1 AND c.sortie_id = $2 AND c.id = $5"
    ))
    .bind(etab)
    .bind(sortie)
    .bind(i32::try_from(DELAI_CONTOURNEMENT_JOURS).unwrap_or(30))
    .bind(maintenant)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

/// Règle « sortie entière » **ou** contournement autorisé : la personne est libre de la sortie, sinon elle présente un
/// contournement (le sien, pour cette action et cette sortie) autorisé par une autre personne encore libre, non expiré et
/// non utilisé ; il est alors **consommé** (fait « utilisé » avec la cible), dans la transaction de l'écriture.
/// Sans contournement valable, le refus est celui de la règle 3 (et se trace comme tel).
pub(crate) async fn exiger_libre_ou_contourne(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    sortie: Uuid,
    action: &str,
    contournement: Option<Uuid>,
    cible: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let etab = a.etablissement_id;
    let e = match exiger_libre_de_la_sortie(conn, etab, a.user_id, sortie).await {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    if !est_refus_regle3(&e) {
        return Err(e);
    }
    let Some(cid) = contournement else {
        return Err(e);
    };
    let valable: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1
              FROM ent_sortie_argent_contournement c
              JOIN ent_sortie_argent_contournement_evt ok
                ON ok.etablissement_id = c.etablissement_id AND ok.contournement_id = c.id AND ok.evenement = 'autorise'
             WHERE c.etablissement_id = $1 AND c.id = $2 AND c.sortie_id = $3 AND c.action = $4
               AND c.demande_par_user_id = $5
               AND ok.le + make_interval(days => $6) >= $7
               AND ok.par_user_id <> $5
               AND ent_sortie_libre_de_la_sortie($1, $3, ok.par_user_id)
               AND NOT EXISTS (SELECT 1 FROM ent_sortie_argent_contournement_evt u
                                WHERE u.etablissement_id = c.etablissement_id AND u.contournement_id = c.id
                                  AND u.evenement = 'utilise'))",
    )
    .bind(etab)
    .bind(cid)
    .bind(sortie)
    .bind(action)
    .bind(a.user_id)
    .bind(i32::try_from(DELAI_CONTOURNEMENT_JOURS).unwrap_or(30))
    .bind(maintenant)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !valable {
        return Err(e);
    }
    sqlx::query(
        "INSERT INTO ent_sortie_argent_contournement_evt
             (id, etablissement_id, contournement_id, evenement, par_user_id, cible_id, le)
         VALUES ($1, $2, $3, 'utilise', $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cid)
    .bind(a.user_id)
    .bind(cible)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

impl PgEnt {
    /// La personne liée à un participant **demande** à contourner la règle « sortie entière » pour une action précise.
    /// Personnel habilité (`voyages.argent`) ; motif obligatoire ; refusée si elle n'est pas liée (aucun contournement
    /// nécessaire) ou si une demande de même nature est déjà ouverte.
    pub async fn voyage_contournement_demander(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        action: &str,
        motif: &str,
    ) -> Result<ContournementVue, EntDbError> {
        self.voyage_contournement_demander_a(a, id, action, motif, Utc::now())
            .await
    }

    /// Variante à horloge passée en paramètre.
    pub async fn voyage_contournement_demander_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        action: &str,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<ContournementVue, EntDbError> {
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        if !ACTIONS_CONTOURNEMENT.contains(&action) {
            return Err(invalide());
        }
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        // Libre de la sortie : aucun contournement n'est nécessaire (elle accomplit l'action elle-même).
        match exiger_libre_de_la_sortie(&mut t, a.etablissement_id, a.user_id, s.id).await {
            Ok(()) => return Err(refus(
                RefusMetier::Conflit,
                "Aucun contournement nécessaire : tu n'es lié à aucun participant de cette sortie.",
            )),
            Err(e) if est_refus_regle3(&e) => {}
            Err(e) => return Err(e),
        }
        let ouverte: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_argent_contournement c
                             WHERE c.etablissement_id = $1 AND c.sortie_id = $2 AND c.action = $3
                               AND c.demande_par_user_id = $4
                               AND c.demande_le + make_interval(days => $5) >= $6
                               AND NOT EXISTS (SELECT 1 FROM ent_sortie_argent_contournement_evt e
                                                WHERE e.etablissement_id = c.etablissement_id
                                                  AND e.contournement_id = c.id
                                                  AND e.evenement IN ('refuse', 'utilise')))",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(action)
        .bind(a.user_id)
        .bind(i32::try_from(DELAI_CONTOURNEMENT_JOURS).unwrap_or(30))
        .bind(maintenant)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if ouverte {
            return Err(conflit());
        }
        let nid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_argent_contournement
                 (id, etablissement_id, sortie_id, action, demande_par_user_id, motif, demande_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(nid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(action)
        .bind(a.user_id)
        .bind(&motif)
        .bind(maintenant)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let vue = lire(&mut t, a.etablissement_id, s.id, nid, maintenant)
            .await?
            .ok_or_else(introuvable)?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Une **autre** personne habilitée et **libre** de tout lien avec la sortie autorise ou refuse la demande (motif
    /// obligatoire au refus). Une demande expirée ou déjà décidée : 409. La décision est tracée avec son auteur.
    pub async fn voyage_contournement_decider(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        contournement: Uuid,
        autoriser: bool,
        motif: Option<&str>,
    ) -> Result<ContournementVue, EntDbError> {
        self.voyage_contournement_decider_a(a, id, contournement, autoriser, motif, Utc::now())
            .await
    }

    /// Variante à horloge passée en paramètre.
    pub async fn voyage_contournement_decider_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        contournement: Uuid,
        autoriser: bool,
        motif: Option<&str>,
        maintenant: DateTime<Utc>,
    ) -> Result<ContournementVue, EntDbError> {
        let r = self
            .voyage_contournement_decider_corps(a, id, contournement, autoriser, motif, maintenant)
            .await;
        self.tracer_refus_regle3(a, Some(id), "voyage_contournement_decider_a", r)
            .await
    }

    async fn voyage_contournement_decider_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        contournement: Uuid,
        autoriser: bool,
        motif: Option<&str>,
        maintenant: DateTime<Utc>,
    ) -> Result<ContournementVue, EntDbError> {
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        let motif = match (autoriser, motif.map(str::trim).filter(|m| !m.is_empty())) {
            (false, None) => return Err(invalide()),
            (_, Some(m)) => Some(texte(m, 500)?),
            (true, None) => None,
        };
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let c = lire(&mut t, a.etablissement_id, s.id, contournement, maintenant)
            .await?
            .ok_or_else(introuvable)?;
        if c.statut != "en_attente" {
            return Err(conflit());
        }
        if c.demande_par_user_id == a.user_id {
            return Err(refus(RefusMetier::Interdit, MSG_DEMANDEUSE_DECIDE));
        }
        // La seconde personne est libre de tout lien avec la sortie (même règle que pour l'action elle-même).
        exiger_libre_de_la_sortie(&mut t, a.etablissement_id, a.user_id, s.id).await?;
        sqlx::query(
            "INSERT INTO ent_sortie_argent_contournement_evt
                 (id, etablissement_id, contournement_id, evenement, par_user_id, motif, le)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(c.id)
        .bind(if autoriser { "autorise" } else { "refuse" })
        .bind(a.user_id)
        .bind(&motif)
        .bind(maintenant)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let vue = lire(&mut t, a.etablissement_id, s.id, c.id, maintenant)
            .await?
            .ok_or_else(introuvable)?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Les contournements de la sortie (personnel habilité, `voyages.argent`), les plus récents d'abord.
    pub async fn voyage_contournements_lister(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<ContournementVue>, EntDbError> {
        self.voyage_contournements_lister_a(a, id, Utc::now()).await
    }

    /// Variante à horloge passée en paramètre (le statut « expiré » en dépend).
    pub async fn voyage_contournements_lister_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<ContournementVue>, EntDbError> {
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let v: Vec<ContournementVue> = sqlx::query_as(&format!(
            "{LECTURE} WHERE c.etablissement_id = $1 AND c.sortie_id = $2 ORDER BY c.demande_le DESC, c.id"
        ))
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(i32::try_from(DELAI_CONTOURNEMENT_JOURS).unwrap_or(30))
        .bind(maintenant)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }
}

/// Durée de validité d'une demande (pour les tests et l'écran).
#[must_use]
pub fn delai_contournement() -> Duration {
    Duration::days(DELAI_CONTOURNEMENT_JOURS)
}
