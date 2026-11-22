//! Messagerie M12 : flux « quelque chose a changé » (temps réel, §27.1). Interrogation longue (aucune dépendance
//! nouvelle : un flux SSE demanderait une brique de flux ; voir le rapport de livraison). Le web et l'appli unique
//! (D-003) appellent la **même** route : parité des droits.
//!
//! Règles : le flux ne porte **jamais de contenu** (identifiant du fil et instant, rien d'autre) ; chaque appel relit la
//! vue `ent_msg_fil_lisible` (R-M2) : aucun événement pour un non-participant, plus rien dès qu'un participant est
//! retiré. Le client rafraîchit ensuite le fil par les routes ordinaires, qui rejugent les droits.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::service::{EntDbError, PgEnt};

/// Attente maximale d'un appel (secondes) : au-delà, l'appel rend un flux vide et un curseur.
pub const FLUX_ATTENTE_MAX_SECONDES: u64 = 25;
const FLUX_PAS: Duration = Duration::from_secs(2);
const FLUX_FILS_MAX: i64 = 200;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FilActif {
    pub fil_id: Uuid,
    pub instant: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FluxMessagerie {
    /// À rendre tel quel comme `apres` au prochain appel.
    pub curseur: DateTime<Utc>,
    pub fils: Vec<FilActif>,
}

impl PgEnt {
    /// Fils lisibles de ce compte dont un message est devenu visible après `apres`. Sans `apres` : aucun événement,
    /// seulement le curseur de départ. `attente_s` (borné à [`FLUX_ATTENTE_MAX_SECONDES`]) : tant que rien ne s'est
    /// passé, l'appel patiente et relit (droits compris) toutes les deux secondes.
    pub async fn flux_messagerie_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        apres: Option<DateTime<Utc>>,
        attente_s: u64,
    ) -> Result<FluxMessagerie, EntDbError> {
        let Some(depuis) = apres else {
            let (curseur, _) = self
                .messages
                .evenements_depuis(etablissement_id, user_id, Utc::now(), 1)
                .await?;
            return Ok(FluxMessagerie { curseur, fils: Vec::new() });
        };
        let fin = tokio::time::Instant::now() + Duration::from_secs(attente_s.min(FLUX_ATTENTE_MAX_SECONDES));
        loop {
            let (curseur, fils) = self
                .messages
                .evenements_depuis(etablissement_id, user_id, depuis, FLUX_FILS_MAX)
                .await?;
            if !fils.is_empty() || tokio::time::Instant::now() >= fin {
                return Ok(FluxMessagerie {
                    curseur,
                    fils: fils.into_iter().map(|(fil_id, instant)| FilActif { fil_id, instant }).collect(),
                });
            }
            tokio::time::sleep(FLUX_PAS).await;
        }
    }
}
