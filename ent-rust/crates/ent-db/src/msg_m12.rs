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

use crate::msg_repo::PositionFlux;
use crate::service::{EntDbError, PgEnt};

/// Attente maximale d'un appel (secondes) : au-delà, l'appel rend un flux vide et un curseur.
pub const FLUX_ATTENTE_MAX_SECONDES: u64 = 25;
/// Intervalle de relecture d'un appel en attente, en « secondes » du flux (voir [`ReglagesFlux::seconde`]).
const FLUX_PAS_SECONDES: u32 = 2;
const FLUX_FILS_MAX: i64 = 200;

/// Réglages du temps et de la taille d'une réponse. La production prend les valeurs par défaut ; un test les réduit
/// (`seconde` de quelques millisecondes : une attente de 25 s ne coûte alors presque rien) et fixe une petite limite de
/// fils pour provoquer la troncature sans écrire des centaines de messages.
#[derive(Debug, Clone, Copy)]
pub struct ReglagesFlux {
    /// Durée d'une « seconde » du flux : l'attente demandée (`attente`, en secondes) et le pas de relecture la multiplient.
    pub seconde: Duration,
    /// Nombre maximal d'événements par flux (immédiats, différés) dans une réponse.
    pub fils_max: i64,
}

impl Default for ReglagesFlux {
    fn default() -> Self {
        Self { seconde: Duration::from_secs(1), fils_max: FLUX_FILS_MAX }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FilActif {
    pub fil_id: Uuid,
    pub instant: DateTime<Utc>,
    /// `nouveau` (un message est devenu visible) ou `retire` (un message a été retiré : aucun contenu).
    pub genre: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FluxMessagerie {
    /// Position de reprise, **opaque** : à rendre telle quelle comme `apres` au prochain appel (format `v2`, M1 :
    /// il change avec l'abandon de l'horloge ; un ancien curseur provoque `resynchro`).
    pub curseur: String,
    /// Plus d'événements que la limite : rappeler aussitôt avec ce curseur.
    pub tronque: bool,
    /// Curseur trop ancien (au-delà de la fenêtre maximale) ou illisible : le client recharge tout, puis repart du curseur.
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
        apres: Option<&str>,
        attente_s: u64,
    ) -> Result<FluxMessagerie, EntDbError> {
        self.flux_messagerie_reglages(etablissement_id, user_id, role, apres, attente_s, ReglagesFlux::default())
            .await
    }

    pub async fn flux_messagerie_reglages(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        apres: Option<&str>,
        attente_s: u64,
        reglages: ReglagesFlux,
    ) -> Result<FluxMessagerie, EntDbError> {
        let Some(texte) = apres else {
            let depart = self.messages.position_depart_flux(etablissement_id).await?;
            return Ok(FluxMessagerie { curseur: depart.encoder(), tronque: false, resynchro: false, fils: Vec::new() });
        };
        let Some(mut depuis) = PositionFlux::decoder(texte) else {
            // Ancien format ou valeur fabriquée : rien n'est rendu, le client recharge tout et repart d'ici.
            let depart = self.messages.position_depart_flux(etablissement_id).await?;
            return Ok(FluxMessagerie { curseur: depart.encoder(), tronque: false, resynchro: true, fils: Vec::new() });
        };
        let attente = reglages.seconde * (attente_s.min(FLUX_ATTENTE_MAX_SECONDES) as u32);
        let pas = reglages.seconde * FLUX_PAS_SECONDES;
        let fin = tokio::time::Instant::now() + attente;
        loop {
            let r = self.messages.evenements_depuis(etablissement_id, user_id, &depuis, reglages.fils_max).await?;
            // Jugement de lecture groupé (une série de requêtes pour tous les candidats, pas une par fil).
            let ids: Vec<Uuid> = r.evenements.iter().map(|(f, _, _)| *f).collect();
            let candidats = self.messages.get_fils_lot(etablissement_id, &ids).await?;
            let lisibles = self.actor_can_access_fils_lot(etablissement_id, user_id, role, &candidats).await?;
            let fils: Vec<FilActif> = r
                .evenements
                .iter()
                .filter(|(f, _, _)| lisibles.contains(f))
                .map(|(fil_id, instant, genre)| FilActif { fil_id: *fil_id, instant: *instant, genre: genre.clone() })
                .collect();
            if !fils.is_empty() || r.resynchro || r.tronque || tokio::time::Instant::now() >= fin {
                return Ok(FluxMessagerie { curseur: r.position.encoder(), tronque: r.tronque, resynchro: r.resynchro, fils });
            }
            // M3 : candidats tous écartés par le jugement de lecture, ou rien de nouveau. On repart de la position
            // rendue : un événement n'est jamais servi deux fois, et rien ne rend l'appel suivant immédiat. Le temps
            // restant est borné par `fin` : jamais de boucle serrée.
            depuis = r.position;
            tokio::time::sleep(pas.min(fin.saturating_duration_since(tokio::time::Instant::now()).max(Duration::from_millis(1)))).await;
        }
    }
}
