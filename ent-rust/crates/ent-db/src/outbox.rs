//! Outbox transactionnelle et distribution des événements (socle 2, lot S2-0).
//!
//! **Publication.** Un fait métier et son événement s'écrivent dans la même
//! transaction : `tx.publier(draft)` insère dans le journal (`ent_metier_event`)
//! et dans l'outbox (`ent_outbox`), avec un numéro d'ordre par agrégat. Si la
//! transaction est annulée, l'événement disparaît avec le fait.
//!
//! **Distribution.** Le [`Distributeur`] lit l'outbox par lots
//! (`FOR NO KEY UPDATE SKIP LOCKED` : compatible avec la clé étrangère
//! des marques de traitement), dans l'ordre de chaque agrégat, et appelle les
//! [`Consommateur`]s enregistrés pour le type de l'événement. Livraison « au
//! moins une fois » ; chaque consommateur est idempotent grâce à
//! `ent_event_traitement` (son effet et la marque « traité » sont dans une seule
//! transaction). Échec → reprise avec délai croissant, puis file des échecs.
//!
//! **Cloisonnement.** Un consommateur ne reçoit que l'événement et sa transaction ;
//! l'établissement est celui de l'événement (variable de transaction
//! `ent.etablissement_id` positionnée, prête pour une future RLS). Il doit filtrer
//! toutes ses lectures et écritures sur `evt.etablissement_id()`.

use crate::event_store::{aggregate_str, parse_aggregate};
use chrono::{DateTime, Utc};
use ent_events::{EventBusError, MetierEvent, MetierEventDraft};
use sqlx::{PgConnection, PgPool};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Profondeur maximale d'une chaîne de réactions (événement → réaction → événement…).
pub const PROFONDEUR_MAX: i16 = 5;

/// Taille maximale conservée d'un message d'erreur de consommateur.
const ERREUR_MAX: usize = 500;

/// Chaîne de causalité d'un événement (« pourquoi cet événement existe-t-il ? »).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Causalite {
    pub correlation_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub profondeur: i16,
}

impl Causalite {
    /// Événement émis en réaction à `cause` (même corrélation, profondeur + 1).
    pub fn suite_de(cause: &MetierEvent, causalite_cause: Causalite) -> Self {
        Self {
            correlation_id: causalite_cause.correlation_id.or(Some(cause.id)),
            causation_id: Some(cause.id),
            profondeur: causalite_cause.profondeur.saturating_add(1),
        }
    }
}

/// Publication dans la transaction du fait métier : `tx.publier(draft)`.
///
/// Implémenté pour `PgConnection` : s'utilise directement sur une
/// `sqlx::Transaction<'_, Postgres>` (déréférencement) et sur la transaction
/// qu'un consommateur reçoit.
pub trait Publier {
    fn publier(
        &mut self,
        draft: MetierEventDraft,
    ) -> impl Future<Output = Result<MetierEvent, EventBusError>> + Send;

    fn publier_avec(
        &mut self,
        draft: MetierEventDraft,
        causalite: Causalite,
    ) -> impl Future<Output = Result<MetierEvent, EventBusError>> + Send;
}

impl Publier for PgConnection {
    fn publier(
        &mut self,
        draft: MetierEventDraft,
    ) -> impl Future<Output = Result<MetierEvent, EventBusError>> + Send {
        publier_sur(self, draft, Causalite::default())
    }

    fn publier_avec(
        &mut self,
        draft: MetierEventDraft,
        causalite: Causalite,
    ) -> impl Future<Output = Result<MetierEvent, EventBusError>> + Send {
        publier_sur(self, draft, causalite)
    }
}

fn infra(contexte: &'static str) -> impl Fn(sqlx::Error) -> EventBusError {
    move |e| {
        tracing::error!(contexte, error = %e, "outbox : écriture échouée");
        EventBusError::Infrastructure
    }
}

async fn publier_sur(
    conn: &mut PgConnection,
    draft: MetierEventDraft,
    causalite: Causalite,
) -> Result<MetierEvent, EventBusError> {
    if draft.etablissement_id.is_nil() {
        return Err(EventBusError::MissingEtablissement);
    }
    if draft.type_name.trim().is_empty() {
        return Err(EventBusError::EmptyType);
    }
    if causalite.profondeur < 0 || causalite.profondeur > PROFONDEUR_MAX {
        return Err(EventBusError::ProfondeurDepassee);
    }
    let event = draft.into_event();
    let aggregate = aggregate_str(event.aggregate);

    sqlx::query(
        r#"
        INSERT INTO ent_metier_event (
            id, etablissement_id, type, aggregate, aggregate_id,
            eleve_id, payload, actor_user_id, created_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
        "#,
    )
    .bind(event.id)
    .bind(event.etablissement_id)
    .bind(&event.type_name)
    .bind(aggregate)
    .bind(event.aggregate_id)
    .bind(event.eleve_id)
    .bind(&event.payload)
    .bind(&event.actor_user_id)
    .bind(event.created_at)
    .execute(&mut *conn)
    .await
    .map_err(infra("journal"))?;

    // Le compteur est verrouillé jusqu'au commit : deux transactions sur le même
    // agrégat sont sérialisées, l'ordre `seq_agregat` est donc l'ordre de commit.
    let (seq_agregat,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO ent_outbox_agregat (etablissement_id, aggregate, aggregate_id, dernier_seq)
        VALUES ($1, $2, $3, 1)
        ON CONFLICT (etablissement_id, aggregate, aggregate_id)
        DO UPDATE SET dernier_seq = ent_outbox_agregat.dernier_seq + 1
        RETURNING dernier_seq
        "#,
    )
    .bind(event.etablissement_id)
    .bind(aggregate)
    .bind(event.aggregate_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(infra("ordre agrégat"))?;

    sqlx::query(
        r#"
        INSERT INTO ent_outbox (
            event_id, etablissement_id, type, aggregate, aggregate_id, seq_agregat,
            correlation_id, causation_id, profondeur, created_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
        "#,
    )
    .bind(event.id)
    .bind(event.etablissement_id)
    .bind(&event.type_name)
    .bind(aggregate)
    .bind(event.aggregate_id)
    .bind(seq_agregat)
    .bind(causalite.correlation_id)
    .bind(causalite.causation_id)
    .bind(causalite.profondeur)
    .bind(event.created_at)
    .execute(&mut *conn)
    .await
    .map_err(infra("outbox"))?;

    Ok(event)
}

/// Événement tel qu'un consommateur le reçoit.
#[derive(Debug, Clone)]
pub struct EvenementDistribue {
    event: MetierEvent,
    pub seq: i64,
    pub seq_agregat: i64,
    pub causalite: Causalite,
    /// Nombre de tentatives déjà faites avant celle-ci.
    pub tentatives: i32,
}

impl EvenementDistribue {
    pub fn event(&self) -> &MetierEvent {
        &self.event
    }

    /// Seul établissement dans lequel le consommateur a le droit d'agir.
    pub fn etablissement_id(&self) -> Uuid {
        self.event.etablissement_id
    }

    /// Causalité à utiliser pour un événement publié en réaction.
    pub fn causalite_suivante(&self) -> Causalite {
        Causalite::suite_de(&self.event, self.causalite)
    }
}

pub type FutureTraitement<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Réaction d'un domaine aux événements d'un autre.
///
/// `traiter` reçoit la transaction dans laquelle la marque « traité » a déjà été
/// posée : l'effet et la marque sont validés ensemble ou annulés ensemble. Un
/// consommateur n'écrit que dans son propre domaine et seulement pour
/// `evt.etablissement_id()`.
pub trait Consommateur: Send + Sync {
    /// Nom stable (clé d'idempotence) : ne jamais le renommer sans migration.
    fn nom(&self) -> &'static str;
    /// Types d'événements écoutés.
    fn types(&self) -> &'static [&'static str];
    /// Un consommateur à effet externe (mail, SMS, push) n'est **pas** rejouable.
    fn rejouable(&self) -> bool {
        true
    }
    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a>;
}

/// Réglages de la boucle de distribution.
#[derive(Debug, Clone)]
pub struct ConfigDistribution {
    pub taille_lot: i64,
    pub max_tentatives: i32,
    /// Délai après le premier échec ; doublé à chaque échec suivant.
    pub delai_base: Duration,
    pub delai_max: Duration,
    /// Pause entre deux lots quand l'outbox est vide.
    pub intervalle: Duration,
    /// Restreint la distribution à un établissement (rattrapage ciblé, tests).
    /// `None` en production : un seul distributeur pour tous les établissements.
    pub etablissement: Option<Uuid>,
}

impl Default for ConfigDistribution {
    fn default() -> Self {
        Self {
            taille_lot: 100,
            max_tentatives: 8,
            delai_base: Duration::from_secs(2),
            delai_max: Duration::from_secs(15 * 60),
            intervalle: Duration::from_millis(500),
            etablissement: None,
        }
    }
}

impl ConfigDistribution {
    /// Délai avant la tentative suivante, après `tentatives` échecs (≥ 1).
    pub fn delai_reprise(&self, tentatives: i32) -> Duration {
        let n = tentatives.clamp(1, 30) as u32 - 1;
        let facteur = 1u32.checked_shl(n).unwrap_or(u32::MAX);
        self.delai_base
            .checked_mul(facteur)
            .unwrap_or(self.delai_max)
            .min(self.delai_max)
    }
}

/// Compteurs simples du processus (exposables dans `/metrics`).
#[derive(Debug, Default)]
pub struct MetriquesDistribution {
    lots: AtomicU64,
    distribues: AtomicU64,
    traitements: AtomicU64,
    doublons: AtomicU64,
    reprises: AtomicU64,
    echecs: AtomicU64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstantaneMetriques {
    pub lots: u64,
    /// Événements entièrement distribués.
    pub distribues: u64,
    /// Effets de consommateur validés.
    pub traitements: u64,
    /// Livraisons répétées ignorées (déjà traitées).
    pub doublons: u64,
    /// Échecs suivis d'une reprise planifiée.
    pub reprises: u64,
    /// Événements passés en file des échecs.
    pub echecs: u64,
}

impl MetriquesDistribution {
    pub fn instantane(&self) -> InstantaneMetriques {
        InstantaneMetriques {
            lots: self.lots.load(Ordering::Relaxed),
            distribues: self.distribues.load(Ordering::Relaxed),
            traitements: self.traitements.load(Ordering::Relaxed),
            doublons: self.doublons.load(Ordering::Relaxed),
            reprises: self.reprises.load(Ordering::Relaxed),
            echecs: self.echecs.load(Ordering::Relaxed),
        }
    }
}

/// Résultat d'un lot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RapportLot {
    pub lus: usize,
    pub distribues: usize,
    pub reprises: usize,
    pub echecs: usize,
}

/// État de l'outbox pour un établissement (supervision).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EtatOutbox {
    pub a_distribuer: i64,
    pub distribues: i64,
    pub echecs: i64,
}

/// Une entrée de la file des échecs.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EchecOutbox {
    pub event_id: Uuid,
    pub seq: i64,
    #[sqlx(rename = "type")]
    pub type_name: String,
    pub aggregate: String,
    pub aggregate_id: Uuid,
    pub seq_agregat: i64,
    pub tentatives: i32,
    pub derniere_erreur: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum ErreurRejeu {
    #[error("consommateur inconnu")]
    ConsommateurInconnu,
    #[error("consommateur non rejouable (effet externe)")]
    NonRejouable,
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(sqlx::FromRow)]
struct LigneOutbox {
    event_id: Uuid,
    seq: i64,
    seq_agregat: i64,
    correlation_id: Option<Uuid>,
    causation_id: Option<Uuid>,
    profondeur: i16,
    tentatives: i32,
    etablissement_id: Uuid,
    #[sqlx(rename = "type")]
    type_name: String,
    aggregate: String,
    aggregate_id: Uuid,
    eleve_id: Option<Uuid>,
    payload: serde_json::Value,
    actor_user_id: Option<String>,
    created_at: DateTime<Utc>,
}

impl LigneOutbox {
    fn into_distribue(self) -> EvenementDistribue {
        EvenementDistribue {
            event: MetierEvent {
                id: self.event_id,
                etablissement_id: self.etablissement_id,
                type_name: self.type_name,
                aggregate: parse_aggregate(&self.aggregate),
                aggregate_id: self.aggregate_id,
                eleve_id: self.eleve_id,
                payload: self.payload,
                actor_user_id: self.actor_user_id,
                created_at: self.created_at,
            },
            seq: self.seq,
            seq_agregat: self.seq_agregat,
            causalite: Causalite {
                correlation_id: self.correlation_id,
                causation_id: self.causation_id,
                profondeur: self.profondeur,
            },
            tentatives: self.tentatives,
        }
    }
}

enum Issue {
    Traite,
    Doublon,
}

/// Distributeur de l'outbox : registre des consommateurs + boucle de fond.
pub struct Distributeur {
    pool: PgPool,
    config: ConfigDistribution,
    consommateurs: Vec<Arc<dyn Consommateur>>,
    par_type: HashMap<&'static str, Vec<usize>>,
    metriques: Arc<MetriquesDistribution>,
    arret: Arc<AtomicBool>,
}

impl Distributeur {
    pub fn new(pool: PgPool, config: ConfigDistribution) -> Self {
        Self {
            pool,
            config,
            consommateurs: Vec::new(),
            par_type: HashMap::new(),
            metriques: Arc::new(MetriquesDistribution::default()),
            arret: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Enregistre un consommateur pour ses types. Un nom en double est refusé
    /// (il casserait l'idempotence).
    pub fn enregistrer(&mut self, c: Arc<dyn Consommateur>) -> Result<(), String> {
        if self.consommateurs.iter().any(|x| x.nom() == c.nom()) {
            return Err(format!("consommateur déjà enregistré : {}", c.nom()));
        }
        let idx = self.consommateurs.len();
        for t in c.types() {
            self.par_type.entry(*t).or_default().push(idx);
        }
        self.consommateurs.push(c);
        Ok(())
    }

    pub fn noms_consommateurs(&self) -> Vec<&'static str> {
        self.consommateurs.iter().map(|c| c.nom()).collect()
    }

    pub fn metriques(&self) -> InstantaneMetriques {
        self.metriques.instantane()
    }

    pub fn config(&self) -> &ConfigDistribution {
        &self.config
    }

    /// Demande l'arrêt de la boucle lancée par [`Distributeur::lancer`].
    pub fn arreter(&self) {
        self.arret.store(true, Ordering::SeqCst);
    }

    /// Boucle de distribution en tâche tokio.
    pub fn lancer(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tracing::info!(
                consommateurs = ?self.noms_consommateurs(),
                "distributeur d'événements démarré"
            );
            while !self.arret.load(Ordering::SeqCst) {
                match self.distribuer_un_lot().await {
                    Ok(r) if r.lus as i64 >= self.config.taille_lot => continue,
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "distributeur : lot en échec"),
                }
                tokio::time::sleep(self.config.intervalle).await;
            }
            tracing::info!("distributeur d'événements arrêté");
        })
    }

    /// Distribue les lots disponibles jusqu'à épuisement (tests, rattrapage).
    /// Les événements en attente de reprise ne sont pas attendus.
    pub async fn distribuer_tout(&self) -> Result<RapportLot, sqlx::Error> {
        let mut total = RapportLot::default();
        for _ in 0..1000 {
            let r = self.distribuer_un_lot().await?;
            total.lus += r.lus;
            total.distribues += r.distribues;
            total.reprises += r.reprises;
            total.echecs += r.echecs;
            if r.lus == 0 {
                break;
            }
        }
        Ok(total)
    }

    /// Un lot : événements prêts, dont tous les prédécesseurs du même agrégat
    /// sont distribués (ordre par agrégat), verrouillés sans attente.
    pub async fn distribuer_un_lot(&self) -> Result<RapportLot, sqlx::Error> {
        let mut verrou = self.pool.begin().await?;
        let lignes = sqlx::query_as::<_, LigneOutbox>(
            r#"
            SELECT o.event_id, o.seq, o.seq_agregat, o.correlation_id, o.causation_id,
                   o.profondeur, o.tentatives,
                   e.etablissement_id, e.type, e.aggregate, e.aggregate_id,
                   e.eleve_id, e.payload, e.actor_user_id, e.created_at
            FROM ent_outbox o
            JOIN ent_metier_event e ON e.id = o.event_id
            WHERE o.statut = 'a_distribuer'
              AND o.prochaine_tentative <= now()
              AND e.etablissement_id = o.etablissement_id
              AND ($2::uuid IS NULL OR o.etablissement_id = $2)
              AND NOT EXISTS (
                  SELECT 1 FROM ent_outbox p
                  WHERE p.etablissement_id = o.etablissement_id
                    AND p.aggregate = o.aggregate
                    AND p.aggregate_id = o.aggregate_id
                    AND p.seq_agregat < o.seq_agregat
                    AND p.statut <> 'distribue'
              )
            ORDER BY o.seq
            LIMIT $1
            FOR NO KEY UPDATE OF o SKIP LOCKED
            "#,
        )
        .bind(self.config.taille_lot)
        .bind(self.config.etablissement)
        .fetch_all(&mut *verrou)
        .await?;

        let mut rapport = RapportLot {
            lus: lignes.len(),
            ..RapportLot::default()
        };
        for ligne in lignes {
            let evt = ligne.into_distribue();
            match self.distribuer_evenement(&evt).await {
                Ok(()) => {
                    sqlx::query(
                        r#"
                        UPDATE ent_outbox
                        SET statut = 'distribue', distribue_le = now(),
                            tentatives = tentatives + 1, derniere_erreur = NULL
                        WHERE event_id = $1
                        "#,
                    )
                    .bind(evt.event.id)
                    .execute(&mut *verrou)
                    .await?;
                    rapport.distribues += 1;
                    self.metriques.distribues.fetch_add(1, Ordering::Relaxed);
                }
                Err(erreur) => {
                    let tentatives = evt.tentatives + 1;
                    let erreur: String = erreur.chars().take(ERREUR_MAX).collect();
                    if tentatives >= self.config.max_tentatives {
                        tracing::error!(
                            event_id = %evt.event.id,
                            type_evt = %evt.event.type_name,
                            tentatives,
                            "événement placé en file des échecs"
                        );
                        sqlx::query(
                            r#"
                            UPDATE ent_outbox
                            SET statut = 'echec', tentatives = $2, derniere_erreur = $3
                            WHERE event_id = $1
                            "#,
                        )
                        .bind(evt.event.id)
                        .bind(tentatives)
                        .bind(&erreur)
                        .execute(&mut *verrou)
                        .await?;
                        rapport.echecs += 1;
                        self.metriques.echecs.fetch_add(1, Ordering::Relaxed);
                    } else {
                        let delai = self.config.delai_reprise(tentatives);
                        tracing::warn!(
                            event_id = %evt.event.id,
                            type_evt = %evt.event.type_name,
                            tentatives,
                            delai_ms = delai.as_millis() as u64,
                            "distribution en échec, reprise planifiée"
                        );
                        sqlx::query(
                            r#"
                            UPDATE ent_outbox
                            SET tentatives = $2, derniere_erreur = $3,
                                prochaine_tentative = now() + make_interval(secs => $4)
                            WHERE event_id = $1
                            "#,
                        )
                        .bind(evt.event.id)
                        .bind(tentatives)
                        .bind(&erreur)
                        .bind(delai.as_secs_f64())
                        .execute(&mut *verrou)
                        .await?;
                        rapport.reprises += 1;
                        self.metriques.reprises.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
        verrou.commit().await?;
        self.metriques.lots.fetch_add(1, Ordering::Relaxed);
        Ok(rapport)
    }

    /// Appelle tous les consommateurs du type ; le premier échec interrompt
    /// (les consommateurs déjà passés seront des doublons ignorés à la reprise).
    async fn distribuer_evenement(&self, evt: &EvenementDistribue) -> Result<(), String> {
        let Some(indices) = self.par_type.get(evt.event.type_name.as_str()) else {
            return Ok(());
        };
        for &i in indices {
            let c = &self.consommateurs[i];
            match self.traiter_par(c.as_ref(), evt).await {
                Ok(Issue::Traite) => {
                    self.metriques.traitements.fetch_add(1, Ordering::Relaxed);
                }
                Ok(Issue::Doublon) => {
                    self.metriques.doublons.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => return Err(format!("{} : {e}", c.nom())),
            }
        }
        Ok(())
    }

    async fn traiter_par(
        &self,
        c: &dyn Consommateur,
        evt: &EvenementDistribue,
    ) -> Result<Issue, String> {
        let etab = evt.etablissement_id();
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        sqlx::query("SELECT set_config('ent.etablissement_id', $1, true)")
            .bind(etab.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        let marque = sqlx::query(
            r#"
            INSERT INTO ent_event_traitement (consommateur, event_id, etablissement_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (consommateur, event_id) DO NOTHING
            "#,
        )
        .bind(c.nom())
        .bind(evt.event.id)
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        if marque.rows_affected() == 0 {
            tx.rollback().await.map_err(|e| e.to_string())?;
            return Ok(Issue::Doublon);
        }
        c.traiter(&mut tx, evt).await?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(Issue::Traite)
    }

    /// Supervision : volumes par statut pour un établissement.
    pub async fn etat(&self, etablissement_id: Uuid) -> Result<EtatOutbox, sqlx::Error> {
        let (a, d, e): (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT count(*) FILTER (WHERE statut = 'a_distribuer'),
                   count(*) FILTER (WHERE statut = 'distribue'),
                   count(*) FILTER (WHERE statut = 'echec')
            FROM ent_outbox WHERE etablissement_id = $1
            "#,
        )
        .bind(etablissement_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(EtatOutbox {
            a_distribuer: a,
            distribues: d,
            echecs: e,
        })
    }

    /// File des échecs d'un établissement (plus anciens d'abord).
    pub async fn file_des_echecs(
        &self,
        etablissement_id: Uuid,
        limite: i64,
    ) -> Result<Vec<EchecOutbox>, sqlx::Error> {
        sqlx::query_as::<_, EchecOutbox>(
            r#"
            SELECT event_id, seq, type, aggregate, aggregate_id, seq_agregat,
                   tentatives, derniere_erreur, created_at
            FROM ent_outbox
            WHERE etablissement_id = $1 AND statut = 'echec'
            ORDER BY seq
            LIMIT $2
            "#,
        )
        .bind(etablissement_id)
        .bind(limite.clamp(1, 500))
        .fetch_all(&self.pool)
        .await
    }

    /// Remet un événement de la file des échecs en distribution (après correction).
    pub async fn rejouer_echec(
        &self,
        etablissement_id: Uuid,
        event_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let r = sqlx::query(
            r#"
            UPDATE ent_outbox
            SET statut = 'a_distribuer', tentatives = 0, prochaine_tentative = now()
            WHERE etablissement_id = $1 AND event_id = $2 AND statut = 'echec'
            "#,
        )
        .bind(etablissement_id)
        .bind(event_id)
        .execute(&self.pool)
        .await?;
        Ok(r.rows_affected() == 1)
    }

    /// Rejoue un consommateur rejouable sur les événements d'un établissement à
    /// partir de `depuis_seq` (reconstruction d'une projection). Les autres
    /// consommateurs gardent leurs marques : ils ignorent la nouvelle livraison
    /// (aucun envoi externe rejoué). Renvoie le nombre d'événements remis.
    pub async fn rejouer_consommateur(
        &self,
        nom: &str,
        etablissement_id: Uuid,
        depuis_seq: i64,
    ) -> Result<u64, ErreurRejeu> {
        let c = self
            .consommateurs
            .iter()
            .find(|c| c.nom() == nom)
            .ok_or(ErreurRejeu::ConsommateurInconnu)?;
        if !c.rejouable() {
            return Err(ErreurRejeu::NonRejouable);
        }
        let types: Vec<String> = c.types().iter().map(|t| t.to_string()).collect();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            DELETE FROM ent_event_traitement t
            USING ent_outbox o
            WHERE t.event_id = o.event_id
              AND t.consommateur = $1
              AND o.etablissement_id = $2
              AND o.seq >= $3
              AND o.type = ANY($4)
            "#,
        )
        .bind(nom)
        .bind(etablissement_id)
        .bind(depuis_seq)
        .bind(&types)
        .execute(&mut *tx)
        .await?;
        let r = sqlx::query(
            r#"
            UPDATE ent_outbox
            SET statut = 'a_distribuer', tentatives = 0, prochaine_tentative = now()
            WHERE etablissement_id = $1 AND seq >= $2 AND type = ANY($3)
              AND statut = 'distribue'
            "#,
        )
        .bind(etablissement_id)
        .bind(depuis_seq)
        .bind(&types)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(r.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delai_croissant_plafonne() {
        let c = ConfigDistribution {
            delai_base: Duration::from_secs(2),
            delai_max: Duration::from_secs(60),
            ..ConfigDistribution::default()
        };
        assert_eq!(c.delai_reprise(1), Duration::from_secs(2));
        assert_eq!(c.delai_reprise(2), Duration::from_secs(4));
        assert_eq!(c.delai_reprise(3), Duration::from_secs(8));
        assert_eq!(c.delai_reprise(10), Duration::from_secs(60));
        assert_eq!(c.delai_reprise(1000), Duration::from_secs(60));
    }

    #[test]
    fn causalite_suit_la_cause() {
        let cause = MetierEventDraft {
            etablissement_id: Uuid::new_v4(),
            type_name: "a.b".into(),
            aggregate: ent_events::AggregateKind::Appel,
            aggregate_id: Uuid::new_v4(),
            eleve_id: None,
            payload: serde_json::json!({}),
            actor_user_id: None,
        }
        .into_event();
        let c1 = Causalite::suite_de(&cause, Causalite::default());
        assert_eq!(c1.correlation_id, Some(cause.id));
        assert_eq!(c1.causation_id, Some(cause.id));
        assert_eq!(c1.profondeur, 1);
        let corr = Uuid::new_v4();
        let c2 = Causalite::suite_de(
            &cause,
            Causalite {
                correlation_id: Some(corr),
                causation_id: None,
                profondeur: 3,
            },
        );
        assert_eq!(c2.correlation_id, Some(corr));
        assert_eq!(c2.profondeur, 4);
    }
}
