//! Messagerie M12 : flux « quelque chose a changé » (temps réel, §27.1). Interrogation longue (aucune dépendance
//! nouvelle : un flux SSE demanderait une brique de flux ; voir le rapport de livraison). Le web et l'appli unique
//! (D-003) appellent la **même** route : parité des droits.
//!
//! Règles : le flux ne porte **jamais de contenu** (identifiant du fil et instant, rien d'autre) ; chaque appel relit la
//! vue `ent_msg_fil_lisible` (R-M2) puis le jugement de lecture complet : aucun événement pour un non-participant ni
//! pour qui n'a pas le droit de lire le fil, plus rien dès qu'un participant est retiré. Le client rafraîchit ensuite le fil par les routes ordinaires, qui rejugent les droits.

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
    /// `nouveau` (un message est devenu visible) ou `retire` (un message a été retiré : aucun contenu).
    pub genre: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FluxMessagerie {
    /// À rendre tel quel comme `apres` au prochain appel.
    pub curseur: DateTime<Utc>,
    /// À rendre comme `apres_fil` quand `tronque` (sinon absent).
    pub curseur_fil: Option<Uuid>,
    /// Plus d'événements que la limite : rappeler aussitôt avec ce curseur.
    pub tronque: bool,
    /// Curseur trop ancien (au-delà de la fenêtre maximale) : le client recharge tout, puis repart du curseur.
    pub resynchro: bool,
    pub fils: Vec<FilActif>,
}

impl PgEnt {
    /// Fils où un message est devenu visible (ou a été retiré) après `apres`. Sans `apres` : aucun événement,
    /// seulement le curseur de départ. `attente_s` (borné à [`FLUX_ATTENTE_MAX_SECONDES`]) : tant que rien ne s'est
    /// passé, l'appel patiente et relit (droits compris) toutes les deux secondes.
    ///
    /// Règle 1 : un fil n'est rendu que si le **même jugement que la lecture** (`actor_can_access_fil`, avec le rôle de
    /// la session) l'autorise, réévalué à chaque appel.
    pub async fn flux_messagerie_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        apres: Option<(DateTime<Utc>, Option<Uuid>)>,
        attente_s: u64,
    ) -> Result<FluxMessagerie, EntDbError> {
        self.flux_messagerie_limite(etablissement_id, user_id, role, apres, attente_s, FLUX_FILS_MAX).await
    }

    pub(crate) async fn flux_messagerie_limite(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        apres: Option<(DateTime<Utc>, Option<Uuid>)>,
        attente_s: u64,
        limite: i64,
    ) -> Result<FluxMessagerie, EntDbError> {
        let Some((instant, fil)) = apres else {
            let r = self
                .messages
                .evenements_depuis(etablissement_id, user_id, (Utc::now(), Uuid::nil()), 1)
                .await?;
            return Ok(FluxMessagerie {
                curseur: r.curseur,
                curseur_fil: None,
                tronque: false,
                resynchro: false,
                fils: Vec::new(),
            });
        };
        let mut depuis = (instant, fil.unwrap_or_else(Uuid::nil));
        let fin = tokio::time::Instant::now() + Duration::from_secs(attente_s.min(FLUX_ATTENTE_MAX_SECONDES));
        loop {
            let r = self.messages.evenements_depuis(etablissement_id, user_id, depuis, limite).await?;
            let mut fils = Vec::new();
            for (fil_id, instant, genre) in &r.evenements {
                let Some(fil) = self.messages.get_fil(etablissement_id, *fil_id).await? else {
                    continue;
                };
                if self.actor_can_access_fil(etablissement_id, user_id, role, &fil).await? {
                    fils.push(FilActif { fil_id: *fil_id, instant: *instant, genre: genre.clone() });
                }
            }
            if !fils.is_empty() || r.resynchro || r.tronque || tokio::time::Instant::now() >= fin {
                return Ok(FluxMessagerie {
                    curseur: r.curseur,
                    curseur_fil: r.curseur_fil,
                    tronque: r.tronque,
                    resynchro: r.resynchro,
                    fils,
                });
            }
            // Candidats tous écartés par le jugement de lecture : ne pas les rejuger à chaque tour.
            if let Some((f, i, _)) = r.evenements.last() {
                depuis = (*i, *f);
            }
            tokio::time::sleep(FLUX_PAS).await;
        }
    }
}
