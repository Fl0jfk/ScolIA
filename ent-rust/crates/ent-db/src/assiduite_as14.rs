//! Socle 12, lot AS14b : lecture unitaire de la situation d'un élève pour l'accueil (AS-216).
//!
//! L'accueil (qui a `absences.declarer`) voit, pour **un** élève à la fois, s'il est « en sortie »,
//! « prévenu » ou « absent ce matin » **avant** de déclarer ou de répondre à un appel : jamais un motif,
//! jamais un texte libre (R-AS22), jamais la liste de la journée. Chaque lecture est **journalisée**
//! (événement `assiduite.situation_accueil_lue`, identifiants seulement). Même réponse pour « inexistant »
//! et « d'un autre établissement » (R-AS35).

use crate::outbox::{Causalite, Publier};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::vie_scolaire::db;
use chrono::{NaiveDate, NaiveTime};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Type d'événement du journal de lecture (constante locale).
pub const SITUATION_ACCUEIL_LUE: &str = "assiduite.situation_accueil_lue";

/// Natures « attendu ailleurs » qui font dire « en sortie » à l'accueil.
const NATURES_EN_SORTIE: [&str; 9] = [
    "sortie",
    "sortie_anticipee",
    "examen",
    "mission",
    "pfmp",
    "exclusion_inclusion",
    "mobilite",
    "dispense_presence",
    "prise_en_charge_exterieure",
];

/// Natures d'une absence ou d'un retard annoncé par la famille.
const NATURES_PREVENU: [&str; 3] = ["declaration", "autorisation", "retard_annonce"];

/// Ce que l'accueil sait d'un élève à l'instant : jamais un motif.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SituationAccueil {
    pub eleve_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub en_sortie: bool,
    pub prevenu: bool,
    pub heure_arrivee_prevue: Option<NaiveTime>,
    pub lieu_code: Option<String>,
    pub absent_ce_matin: bool,
}

/// Calcul pur : synthèse d'une liste de situations `(nature, heure d'arrivée prévue, lieu)`.
pub fn synthese_accueil(
    eleve_id: Uuid,
    situations: &[(String, Option<NaiveTime>, Option<String>)],
    absent_ce_matin: bool,
) -> SituationAccueil {
    let mut s = SituationAccueil {
        eleve_id,
        absent_ce_matin,
        ..Default::default()
    };
    for (nature, arrivee, lieu) in situations {
        let n = nature.as_str();
        if NATURES_EN_SORTIE.contains(&n) {
            s.en_sortie = true;
        }
        if NATURES_PREVENU.contains(&n) {
            s.prevenu = true;
            if n == "retard_annonce" && arrivee.is_some() {
                s.heure_arrivee_prevue = *arrivee;
            }
        }
        if n == "localisation" && s.lieu_code.is_none() {
            s.lieu_code = lieu.clone();
        }
    }
    s
}

impl PgEnt {
    /// Situation d'un élève pour l'accueil : `absences.declarer` sur l'établissement, élève existant dans
    /// l'établissement ; la lecture est journalisée dans la même transaction.
    pub async fn situation_accueil_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<SituationAccueil, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AbsencesDeclarer)
            .await?;
        self.exiger_eleve_existe(etab, eleve_id).await?;
        let jour: NaiveDate = self.jour_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let situations: Vec<(String, Option<NaiveTime>, Option<String>)> = sqlx::query_as(
            "SELECT nature, heure_arrivee_prevue, lieu_code FROM ent_asd_situation
              WHERE etablissement_id = $1 AND eleve_id = $2 AND etat = 'active'
                AND debut_jour <= $3 AND fin_jour >= $3
              ORDER BY debut_jour, cree_le",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // « Ce matin » : une séance du jour commençant avant midi, appel non neutralisé, élève absent.
        let (absent,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM ent_appel_ligne l
                  JOIN ent_appel a ON a.etablissement_id = l.etablissement_id AND a.id = l.appel_id
                 WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.etat = 'absent'
                   AND a.jour = $3 AND a.etat <> 'neutralise'
                   AND (a.heure_debut IS NULL OR a.heure_debut < TIME '12:00'))",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        // Journal : qui a lu quoi, quand — identifiants seulement.
        (&mut *tx)
            .publier_avec(
                MetierEventDraft {
                    etablissement_id: etab,
                    type_name: SITUATION_ACCUEIL_LUE.to_string(),
                    aggregate: AggregateKind::Eleve,
                    aggregate_id: eleve_id,
                    eleve_id: Some(eleve_id),
                    payload: json!({ "jour": jour.to_string() }),
                    actor_user_id: Some(acteur.to_string()),
                },
                Causalite::default(),
            )
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(synthese_accueil(eleve_id, &situations, absent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, 0, 0).unwrap()
    }

    #[test]
    fn rien_de_connu_rien_a_dire() {
        let s = synthese_accueil(Uuid::nil(), &[], false);
        assert!(!s.en_sortie && !s.prevenu && !s.absent_ce_matin && s.lieu_code.is_none());
    }

    #[test]
    fn sortie_prevenu_et_localisation_se_cumulent_sans_motif() {
        let sit = vec![
            ("sortie".to_string(), None, None),
            ("declaration".to_string(), None, None),
            (
                "localisation".to_string(),
                None,
                Some("infirmerie".to_string()),
            ),
            ("retard_annonce".to_string(), Some(t(10)), None),
        ];
        let s = synthese_accueil(Uuid::nil(), &sit, true);
        assert!(s.en_sortie && s.prevenu && s.absent_ce_matin);
        assert_eq!(s.lieu_code.as_deref(), Some("infirmerie"));
        assert_eq!(s.heure_arrivee_prevue, Some(t(10)));
    }

    #[test]
    fn une_localisation_n_est_pas_une_sortie() {
        let sit = vec![("localisation".to_string(), None, Some("cdi".to_string()))];
        let s = synthese_accueil(Uuid::nil(), &sit, false);
        assert!(!s.en_sortie && !s.prevenu);
    }
}
