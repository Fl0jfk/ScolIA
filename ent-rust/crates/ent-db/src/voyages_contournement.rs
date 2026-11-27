//! Socle 42, lot V3b : **contournement tracé** de la règle « sortie entière » (contre-revue des suites, Moyen 3, D-069).
//!
//! Les acomptes fournisseurs (création, paiement, annulation) et l'annulation collective d'avoirs engagent la sortie
//! entière : la personne qui les accomplit n'est liée à **aucun** participant ni payeuse d'aucun de ses comptes (M-3 de
//! V3). La règle reste **stricte** (l'intégrité prime sur la disponibilité) ; mais quand la seule personne habilitée est
//! liée à un participant, la sortie ne doit pas rester bloquée **ni** être débloquée en silence :
//!
//! 1. la personne liée **demande** un contournement : action visée, motif obligatoire, **et ce que la demande couvre** (pas
//!    d'autorisation « en blanc ») : l'acompte visé (payer, annuler), le plafond et le prestataire (créer, payer) ;
//! 2. une **seconde personne**, distincte, membre habilitée (`voyages.argent`) et **libre** de tout lien avec la sortie, voit
//!    cette portée (API) et l'**autorise** (ou le refuse, avec motif) ;
//! 3. la demandeuse l'**utilise**, **une seule fois**, pour **une seule** écriture dans cette portée (la cible est tracée).
//!
//! Trois faits en ajout seul (`ent_sortie_argent_contournement`, `…_evt`), sans nom d'élève ni montant d'élève, jamais dans
//! le journal de l'organisateur ; les horodatages sont posés par la base. **La base garde tout** : un acompte ou une annulation
//! collective d'une personne liée n'est accepté que si un contournement autorisé par une autre personne libre, encore
//! habilitée, non expiré ([`DELAI_CONTOURNEMENT_JOURS`] jours depuis l'autorisation) a été utilisé pour cette écriture, dans la
//! cible, sous le plafond et pour le prestataire de la demande (`ent_sortie_contournement_valide`, déclencheurs de 656).
//!
//! **Consigne d'écran (RGPD, D-069)** : le motif est libre (500 caractères) : l'écran de demande affiche « n'indique aucun nom
//! d'élève ni de famille ». Les faits sont conservés tant que la sortie existe et au plus [`CONSERVATION_CONTOURNEMENT_ANS`]
//! ans (durée des pièces comptables) ; la purge est un traitement du socle d'archivage, à brancher.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::voyages_argent::centimes_de_texte;
use crate::voyages_argent_repo::{peut_argent, verrou_sortie};
use crate::voyages_argent_v3::{db_pouvoir, est_refus_regle3, exiger_libre_de_la_sortie};
use crate::voyages_repo::{charger_avec_acces, refus, texte, tx};

/// Actions qu'un contournement peut couvrir.
pub const ACTIONS_CONTOURNEMENT: [&str; 4] = [
    "acompte_creer",
    "acompte_payer",
    "acompte_annuler",
    "annulation_collective",
];

/// Une demande non décidée expire 30 jours après la demande ; une autorisation non utilisée, 30 jours après l'autorisation
/// (la base applique la même valeur : `interval '30 days'` dans les gardes de 656).
pub const DELAI_CONTOURNEMENT_JOURS: i64 = 30;

/// Durée maximale de conservation des faits de contournement (D-069, valeur prudente : pièces comptables).
pub const CONSERVATION_CONTOURNEMENT_ANS: i64 = 10;

/// Ce qu'une personne demande de contourner, avec sa portée.
#[derive(Debug, Clone, Deserialize)]
pub struct DemandeContournement {
    pub action: String,
    pub motif: String,
    /// Acompte visé (payer, annuler) ; vide pour créer et pour l'annulation collective.
    #[serde(default)]
    pub cible_id: Option<Uuid>,
    /// Plafond en euros (« 120,50 ») de l'écriture : créer, payer.
    #[serde(default)]
    pub montant_max: Option<String>,
    /// Prestataire de l'acompte : créer, payer.
    #[serde(default)]
    pub prestataire_id: Option<Uuid>,
}

/// Refus : la demandeuse ne décide pas de sa propre demande (tracé comme les refus de la règle 3).
pub(crate) const MSG_DEMANDEUSE_DECIDE: &str =
    "Une seconde personne autorise : la demandeuse ne décide pas de sa propre demande.";

/// Refus : une autre personne que l'autrice d'une autorisation tente de la révoquer (tracé comme les refus de la règle 3,
/// faible F-6 de la contre-revue V3b suites 2).
pub(crate) const MSG_REVOCATION_AUTRICE: &str =
    "Seule la personne qui a autorisé le contournement peut le révoquer.";

/// Vue d'un contournement pour le personnel habilité (`voyages.argent`).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ContournementVue {
    pub id: Uuid,
    pub action: String,
    pub motif: String,
    pub demande_le: DateTime<Utc>,
    /// Ce que la demande couvre (l'autorisatrice le voit) : l'acompte visé, le plafond (centimes), le prestataire.
    pub cible_id: Option<Uuid>,
    pub montant_max_centimes: Option<i64>,
    pub prestataire_id: Option<Uuid>,
    /// `en_attente`, `autorise`, `refuse`, `utilise`, `revoque` ou `expire`.
    pub statut: String,
    #[serde(skip)]
    pub demande_par_user_id: Uuid,
    #[serde(skip)]
    pub decide_par_user_id: Option<Uuid>,
}

const LECTURE: &str = "SELECT c.id, c.action, c.motif, c.demande_le, c.demande_par_user_id,
        c.cible_id, c.montant_max_centimes, c.prestataire_id,
        (SELECT e.par_user_id FROM ent_sortie_argent_contournement_evt e
          WHERE e.etablissement_id = c.etablissement_id AND e.contournement_id = c.id
            AND e.evenement IN ('autorise', 'refuse')) AS decide_par_user_id,
        CASE
          WHEN EXISTS (SELECT 1 FROM ent_sortie_argent_contournement_evt e
                        WHERE e.etablissement_id = c.etablissement_id AND e.contournement_id = c.id
                          AND e.evenement = 'utilise') THEN 'utilise'
          WHEN EXISTS (SELECT 1 FROM ent_sortie_argent_contournement_evt e
                        WHERE e.etablissement_id = c.etablissement_id AND e.contournement_id = c.id
                          AND e.evenement = 'revoque') THEN 'revoque'
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
/// contournement (le sien, pour cette action et cette sortie) autorisé par une autre personne encore habilitée et libre, non
/// expiré, non utilisé, et dont la cible est celle de l'écriture ; il est alors **consommé** (fait « utilisé » avec la cible),
/// dans la transaction de l'écriture. Le plafond et le prestataire sont contrôlés par la base à l'écriture (déclencheur de
/// pouvoir) ; `montant` et `presta`, quand l'appelant les connaît, les contrôlent aussi ici (refus plus tôt).
/// Sans contournement valable, le refus est celui de la règle 3 (et se trace comme tel).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn exiger_libre_ou_contourne(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    sortie: Uuid,
    action: &str,
    contournement: Option<Uuid>,
    cible: Uuid,
    montant: Option<i64>,
    presta: Option<Uuid>,
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
             WHERE c.etablissement_id = $1 AND c.id = $2
               AND ent_sortie_contournement_ouvert($1, c.id, $3, $5, $4, $6,
                       COALESCE($7, c.montant_max_centimes), COALESCE($8, c.prestataire_id), $9)
               AND NOT EXISTS (SELECT 1 FROM ent_sortie_argent_contournement_evt u
                                WHERE u.etablissement_id = c.etablissement_id AND u.contournement_id = c.id
                                  AND u.evenement = 'utilise'))",
    )
    .bind(etab)
    .bind(cid)
    .bind(sortie)
    .bind(action)
    .bind(a.user_id)
    .bind(cible)
    .bind(montant)
    .bind(presta)
    .bind(maintenant)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !valable {
        return Err(e);
    }
    // L'horodatage (`le`) est posé par la base.
    sqlx::query(
        "INSERT INTO ent_sortie_argent_contournement_evt
             (id, etablissement_id, contournement_id, evenement, par_user_id, cible_id)
         VALUES ($1, $2, $3, 'utilise', $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(cid)
    .bind(a.user_id)
    .bind(cible)
    .execute(&mut *conn)
    .await
    .map_err(db_pouvoir)?;
    Ok(())
}

impl PgEnt {
    /// La personne liée à un participant **demande** à contourner la règle « sortie entière » pour une action précise, avec
    /// sa portée (cible, plafond, prestataire : voir [`DemandeContournement`]). Personnel habilité (`voyages.argent`) ; motif
    /// obligatoire ; refusée si elle n'est pas liée (aucun contournement nécessaire) ou si une demande identique est déjà
    /// ouverte (en attente, ou autorisée et non expirée).
    pub async fn voyage_contournement_demander(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        d: DemandeContournement,
    ) -> Result<ContournementVue, EntDbError> {
        self.voyage_contournement_demander_a(a, id, d, Utc::now())
            .await
    }

    /// Variante à horloge passée en paramètre (le statut en dépend ; l'horodatage de la demande est posé par la base).
    pub async fn voyage_contournement_demander_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        d: DemandeContournement,
        maintenant: DateTime<Utc>,
    ) -> Result<ContournementVue, EntDbError> {
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        if !ACTIONS_CONTOURNEMENT.contains(&d.action.as_str()) {
            return Err(invalide());
        }
        let action = d.action.as_str();
        // Portée exigée par action (la base la répète en contrainte).
        let montant_max = match d.montant_max.as_deref() {
            Some(m) => Some(
                centimes_de_texte(m)
                    .filter(|c| *c > 0 && *c <= 100_000_000_000)
                    .ok_or_else(invalide)?,
            ),
            None => None,
        };
        let portee_ok = match action {
            "acompte_creer" => {
                d.cible_id.is_none() && montant_max.is_some() && d.prestataire_id.is_some()
            }
            "acompte_payer" => {
                d.cible_id.is_some() && montant_max.is_some() && d.prestataire_id.is_some()
            }
            "acompte_annuler" => {
                d.cible_id.is_some() && montant_max.is_none() && d.prestataire_id.is_none()
            }
            _ => d.cible_id.is_none() && montant_max.is_none() && d.prestataire_id.is_none(),
        };
        if !portee_ok {
            return Err(invalide());
        }
        let motif = texte(&d.motif, 500)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
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
        // L'acompte visé existe sur cette sortie et n'est ni payé ni annulé ; payer : même prestataire que la demande.
        if let Some(cible) = d.cible_id {
            let ligne: Option<(Uuid, bool, bool)> = sqlx::query_as(
                "SELECT prestataire_id, paye_le IS NOT NULL, annule_le IS NOT NULL
                   FROM ent_sortie_acompte_fournisseur
                  WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
            )
            .bind(a.etablissement_id)
            .bind(s.id)
            .bind(cible)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            let (presta, paye, annule) = ligne.ok_or_else(introuvable)?;
            if paye || annule {
                return Err(conflit());
            }
            if action == "acompte_payer" && d.prestataire_id != Some(presta) {
                return Err(invalide());
            }
        }
        // Faible 1 : une seule demande ouverte par personne, action et portée ; « ouverte » = en attente (30 jours depuis
        // la demande) ou autorisée non utilisée (30 jours depuis l'AUTORISATION, pas depuis la demande).
        let memes: Vec<ContournementVue> = sqlx::query_as(&format!(
            "{LECTURE} WHERE c.etablissement_id = $1 AND c.sortie_id = $2 AND c.demande_par_user_id = $5
                AND c.action = $6 AND c.cible_id IS NOT DISTINCT FROM $7 AND c.prestataire_id IS NOT DISTINCT FROM $8"
        ))
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(i32::try_from(DELAI_CONTOURNEMENT_JOURS).unwrap_or(30))
        .bind(maintenant)
        .bind(a.user_id)
        .bind(action)
        .bind(d.cible_id)
        .bind(d.prestataire_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        if memes
            .iter()
            .any(|v| matches!(v.statut.as_str(), "en_attente" | "autorise"))
        {
            return Err(conflit());
        }
        let nid = Uuid::new_v4();
        // `demande_le` est posé par la base (déclencheur) : jamais fourni.
        sqlx::query(
            "INSERT INTO ent_sortie_argent_contournement
                 (id, etablissement_id, sortie_id, action, demande_par_user_id, motif, cible_id, montant_max_centimes, prestataire_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(nid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(action)
        .bind(a.user_id)
        .bind(&motif)
        .bind(d.cible_id)
        .bind(montant_max)
        .bind(d.prestataire_id)
        .execute(&mut *t)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(x) if x.code().as_deref() == Some("23503") => introuvable(),
            e => db(e),
        })?;
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
            return Err(introuvable());
        }
        let c = lire(&mut t, a.etablissement_id, s.id, contournement, maintenant)
            .await?
            .ok_or_else(introuvable)?;
        if c.statut != "en_attente" {
            return Err(conflit());
        }
        // Transverse « individu » (659) : un second compte de la demandeuse ne décide pas non plus.
        if crate::individu::meme_individu(&mut t, a.etablissement_id, c.demande_par_user_id, a.user_id).await? {
            return Err(refus(RefusMetier::Interdit, MSG_DEMANDEUSE_DECIDE));
        }
        // La seconde personne est libre de tout lien avec la sortie (même règle que pour l'action elle-même).
        exiger_libre_de_la_sortie(&mut t, a.etablissement_id, a.user_id, s.id).await?;
        sqlx::query(
            "INSERT INTO ent_sortie_argent_contournement_evt
                 (id, etablissement_id, contournement_id, evenement, par_user_id, motif)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(c.id)
        .bind(if autoriser { "autorise" } else { "refuse" })
        .bind(a.user_id)
        .bind(&motif)
        .execute(&mut *t)
        .await
        .map_err(db_pouvoir)?;
        let vue = lire(&mut t, a.etablissement_id, s.id, c.id, maintenant)
            .await?
            .ok_or_else(introuvable)?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// L'autrice d'une autorisation la **révoque** tant qu'elle n'a pas servi (fait « revoque », ajout seul). Une autre
    /// personne, une demande non autorisée, déjà utilisée ou déjà révoquée : refus (la base garde aussi).
    pub async fn voyage_contournement_revoquer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        contournement: Uuid,
    ) -> Result<ContournementVue, EntDbError> {
        self.voyage_contournement_revoquer_a(a, id, contournement, Utc::now())
            .await
    }

    /// Variante à horloge passée en paramètre.
    pub async fn voyage_contournement_revoquer_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        contournement: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<ContournementVue, EntDbError> {
        // F-6 : un refus de révocation par une autre personne que l'autrice se trace (comme la règle 3).
        let r = self
            .voyage_contournement_revoquer_corps(a, id, contournement, maintenant)
            .await;
        self.tracer_refus_regle3(a, Some(id), "voyage_contournement_revoquer_a", r)
            .await
    }

    async fn voyage_contournement_revoquer_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        contournement: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<ContournementVue, EntDbError> {
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let c = lire(&mut t, a.etablissement_id, s.id, contournement, maintenant)
            .await?
            .ok_or_else(introuvable)?;
        if c.statut != "autorise" && c.statut != "expire" {
            return Err(conflit());
        }
        // F-4 : une demande expirée **sans** autorisation n'a rien à révoquer : conflit d'état (409), pas un refus de
        // personne (403). `decide_par_user_id` ne porte qu'« autorise » ou « refuse », et « refuse » a son propre statut.
        if c.statut == "expire" && c.decide_par_user_id.is_none() {
            return Err(conflit());
        }
        if c.decide_par_user_id != Some(a.user_id) {
            return Err(refus(RefusMetier::Interdit, MSG_REVOCATION_AUTRICE));
        }
        sqlx::query(
            "INSERT INTO ent_sortie_argent_contournement_evt
                 (id, etablissement_id, contournement_id, evenement, par_user_id)
             VALUES ($1, $2, $3, 'revoque', $4)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(c.id)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db_pouvoir)?;
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
            return Err(introuvable());
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
