//! Socle 42, lot V0b : rappels de l'organisateur (NX-16, VOY-132, VOY-133).
//!
//! Un **rappel** est un fait daté, créé **une seule fois** par sortie et par code (contrainte
//! d'unicité : un rattrapage ou un second passage ne double rien). Il publie
//! `voyages.rappel_organisateur` dans la transaction ; le consommateur de notifications
//! (`voyages_reactions.rs`, socle 8) prévient le **porteur**. Rien n'est envoyé aux familles ni aux
//! prestataires (R-SV3) : ce sont des rappels internes. Le calcul des échéances est pur
//! (`voyages_budget::rappels_dus`) ; les dates sont celles de Paris (R-SV29).

use chrono::{DateTime, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::ref_commun::{db, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::voyages_budget::{message_rappel, rappels_dus};
use crate::voyages_repo::{charger_avec_acces, interdit, tx};

#[derive(Debug, Clone, Serialize)]
pub struct RappelVue {
    pub code: String,
    /// Jour (JJ/MM/AAAA, Paris) où le rappel a été constaté.
    pub jour: String,
    pub jours_avant: i32,
    pub message: String,
}

impl PgEnt {
    /// Constate les rappels dus pour un établissement à l'instant `maintenant` et publie ceux qui
    /// sont nouveaux. Idempotent. Retourne le nombre de rappels créés.
    pub async fn voyages_rappels_traiter(
        &self,
        etablissement_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etablissement_id)
            .await
            .map_err(db)?;
        let sorties: Vec<(Uuid, String, String, bool, bool, i32)> = sqlx::query_as(
            "SELECT id, etat, forme, besoin_car, (liste_confirmee_le IS NOT NULL),
                    ((debut AT TIME ZONE 'Europe/Paris')::date
                       - ($2::timestamptz AT TIME ZONE 'Europe/Paris')::date)
               FROM ent_sortie
              WHERE etablissement_id = $1
                AND etat IN ('soumise', 'a_completer', 'preparation', 'a_signer', 'budget_a_valider',
                             'vote', 'a_autoriser', 'ouverte', 'prete')
                AND (debut AT TIME ZONE 'Europe/Paris')::date
                    >= ($2::timestamptz AT TIME ZONE 'Europe/Paris')::date
              ORDER BY debut, id",
        )
        .bind(etablissement_id)
        .bind(maintenant)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut events = Vec::new();
        for (id, etat, forme, car, liste, jours) in sorties {
            for r in rappels_dus(jours as i64, &etat, &forme, car, liste) {
                let cree: Option<(Uuid,)> = sqlx::query_as(
                    "INSERT INTO ent_sortie_rappel (id, etablissement_id, sortie_id, code, jour, jours_avant)
                     VALUES ($1, $2, $3, $4, ($5::timestamptz AT TIME ZONE 'Europe/Paris')::date, $6)
                     ON CONFLICT (sortie_id, code) DO NOTHING
                     RETURNING id",
                )
                .bind(Uuid::new_v4())
                .bind(etablissement_id)
                .bind(id)
                .bind(r.code)
                .bind(maintenant)
                .bind(jours)
                .fetch_optional(&mut *t)
                .await
                .map_err(db)?;
                if cree.is_some() {
                    let ev = t
                        .publier(MetierEventDraft {
                            etablissement_id,
                            type_name: "voyages.rappel_organisateur".to_string(),
                            aggregate: AggregateKind::Voyage,
                            aggregate_id: id,
                            eleve_id: None,
                            payload: json!({ "code": r.code, "jours_avant": jours }),
                            actor_user_id: None,
                        })
                        .await?;
                    events.push(ev);
                }
            }
        }
        t.commit().await.map_err(db)?;
        let n = events.len();
        self.diffuser_apres_commit(&events);
        Ok(n)
    }

    /// Passage planifié sur **tous** les établissements (à lancer une fois par jour).
    pub async fn voyages_rappels_traiter_tous(
        &self,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        let etabs: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_etablissement ORDER BY id")
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        let mut n = 0;
        for (etab,) in etabs {
            n += self.voyages_rappels_traiter(etab, maintenant).await?;
        }
        Ok(n)
    }

    /// Passage déclenché à la main par la direction pour son établissement.
    pub async fn voyages_rappels_declencher(
        &self,
        a: &Acteur<'_>,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        self.voyages_exiger(a, Action::VoyagesAutoriser).await?;
        self.voyages_rappels_traiter(a.etablissement_id, maintenant).await
    }

    /// Rappels déjà constatés pour une sortie : le porteur, ceux qui gèrent la sortie, la direction.
    pub async fn voyage_rappels(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<RappelVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (_, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !(acces.porteur
            || acces.a(Action::VoyagesModifier)
            || acces.a(Action::VoyagesValider)
            || acces.a(Action::VoyagesAutoriser))
        {
            return Err(interdit());
        }
        let rows: Vec<(String, String, i32)> = sqlx::query_as(
            "SELECT code, to_char(jour, 'DD/MM/YYYY'), jours_avant FROM ent_sortie_rappel
              WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY cree_le, code",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(code, jour, jours_avant)| RappelVue {
                message: message_rappel(&code).to_string(),
                code,
                jour,
                jours_avant,
            })
            .collect())
    }
}
