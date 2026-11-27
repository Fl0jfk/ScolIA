//! Registre d'idempotence générique (socle 11, lot MB1, `docs/socles/11_APPLICATIONS_MOBILES.md` §5.2 et §5.5).
//!
//! Une écriture authentifiée qui porte `Idempotency-Key` est inscrite ici **avant** son exécution (revendication de
//! la clé, transaction propre et validée) ; la réponse de succès est conservée à la fin. Même clé + même empreinte
//! (méthode, chemin, corps) = même réponse rejouée pendant 24 h ; même clé + autre empreinte = conflit ; deux
//! requêtes concurrentes de même clé : une seule revendique, l'autre attend ou reçoit « en cours ».
//!
//! Cloisonnement : la clé primaire est (établissement, session, clé) et la RLS d'établissement s'applique ; la clé
//! d'une autre session ou d'une autre personne ne rejoue rien. L'horloge est un **paramètre** (`maintenant`) : aucun
//! appel à l'heure du système ici, pour que les tests fixent leur instant.

use crate::tenant_tx::{etablissements_systeme, ContexteSysteme, TenantTx};
use crate::PgEnt;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

/// Durée de conservation (et de rejeu) d'une clé.
pub const IDEMPOTENCE_DUREE_H: i64 = 24;
/// Au-delà de ce délai, une revendication « en cours » est tenue pour abandonnée (processus tombé) : un rejeu la
/// reprend. Supérieur au délai maximal d'une requête (120 s).
pub const IDEMPOTENCE_ABANDON_S: i64 = 150;
/// Borne de l'exécution d'une écriture idempotente (palier des faibles n° 2) : strictement inférieure au délai
/// d'abandon, pour qu'une exécution encore en cours ne soit jamais reprise par un nouvel essai.
pub const IDEMPOTENCE_EXECUTION_MAX_S: i64 = 120;
/// Taille maximale d'un corps de réponse conservé (alignée sur la contrainte de la table).
pub const IDEMPOTENCE_CORPS_MAX: usize = 262_144;

/// Réponse de succès conservée pour être rejouée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReponseConservee {
    pub statut_http: u16,
    pub type_contenu: Option<String>,
    pub emplacement: Option<String>,
    /// `None` quand la réponse n'a pas pu être conservée (`rejouable` faux) ou qu'elle est vide.
    pub corps: Option<String>,
    pub rejouable: bool,
}

/// Résultat de la revendication d'une clé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevendicationCle {
    /// Première requête (ou reprise d'une revendication abandonnée) : l'appelant exécute puis conserve la réponse.
    Nouvelle,
    /// Même clé, même empreinte, déjà terminée : à rejouer.
    Rejeu(ReponseConservee),
    /// Même clé, autre empreinte.
    Conflit,
    /// Même clé, première requête encore en cours.
    EnCours,
    /// Même clé, terminée, mais sa réponse n'a pas pu être conservée.
    NonRejouable,
}

impl PgEnt {
    /// Revendique `(établissement, session, clé)` pour l'empreinte donnée. Purge au passage les clés de
    /// l'établissement de plus de 24 h (index `(etablissement_id, cree_le)`).
    pub async fn idem_revendiquer(
        &self,
        etablissement_id: Uuid,
        session_id: Uuid,
        cle: Uuid,
        empreinte: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<RevendicationCle, sqlx::Error> {
        let limite = maintenant - Duration::hours(IDEMPOTENCE_DUREE_H);
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query("DELETE FROM ent_idempotence_cle WHERE etablissement_id = $1 AND cree_le < $2")
            .bind(etablissement_id)
            .bind(limite)
            .execute(&mut *tx)
            .await?;
        let insere = sqlx::query(
            "INSERT INTO ent_idempotence_cle (etablissement_id, session_id, cle, empreinte, statut, cree_le)
             VALUES ($1, $2, $3, $4, 'en_cours', $5)
             ON CONFLICT (etablissement_id, session_id, cle) DO NOTHING",
        )
        .bind(etablissement_id)
        .bind(session_id)
        .bind(cle)
        .bind(empreinte)
        .bind(maintenant)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if insere == 1 {
            tx.commit().await?;
            return Ok(RevendicationCle::Nouvelle);
        }
        type Ligne = (
            String,
            String,
            Option<i16>,
            Option<String>,
            Option<String>,
            Option<String>,
            bool,
            DateTime<Utc>,
        );
        let ligne: Option<Ligne> = sqlx::query_as(
            "SELECT empreinte, statut, statut_http, type_contenu, emplacement, corps, rejouable, cree_le
               FROM ent_idempotence_cle
              WHERE etablissement_id = $1 AND session_id = $2 AND cle = $3",
        )
        .bind(etablissement_id)
        .bind(session_id)
        .bind(cle)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((emp, statut, http, type_contenu, emplacement, corps, rejouable, cree_le)) = ligne
        else {
            // Supprimée entre-temps par une purge : l'appelant réessaie.
            tx.rollback().await?;
            return Ok(RevendicationCle::EnCours);
        };
        let resultat = if emp != empreinte {
            RevendicationCle::Conflit
        } else if statut == "terminee" {
            if rejouable {
                RevendicationCle::Rejeu(ReponseConservee {
                    statut_http: http.and_then(|h| u16::try_from(h).ok()).unwrap_or(200),
                    type_contenu,
                    emplacement,
                    corps,
                    rejouable: true,
                })
            } else {
                RevendicationCle::NonRejouable
            }
        } else if cree_le < maintenant - Duration::seconds(IDEMPOTENCE_ABANDON_S) {
            // Reprise atomique d'une revendication abandonnée : un seul des rejeux concurrents la gagne.
            let repris = sqlx::query(
                "UPDATE ent_idempotence_cle SET cree_le = $4
                  WHERE etablissement_id = $1 AND session_id = $2 AND cle = $3
                    AND statut = 'en_cours' AND cree_le = $5",
            )
            .bind(etablissement_id)
            .bind(session_id)
            .bind(cle)
            .bind(maintenant)
            .bind(cree_le)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if repris == 1 {
                RevendicationCle::Nouvelle
            } else {
                RevendicationCle::EnCours
            }
        } else {
            RevendicationCle::EnCours
        };
        tx.commit().await?;
        Ok(resultat)
    }

    /// Conserve la réponse de succès de la requête qui a revendiqué la clé.
    pub async fn idem_terminer(
        &self,
        etablissement_id: Uuid,
        session_id: Uuid,
        cle: Uuid,
        reponse: &ReponseConservee,
        maintenant: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query(
            "UPDATE ent_idempotence_cle
                SET statut = 'terminee', statut_http = $4, type_contenu = $5, emplacement = $6, corps = $7,
                    rejouable = $8, termine_le = $9
              WHERE etablissement_id = $1 AND session_id = $2 AND cle = $3 AND statut = 'en_cours'",
        )
        .bind(etablissement_id)
        .bind(session_id)
        .bind(cle)
        .bind(i16::try_from(reponse.statut_http).unwrap_or(200))
        .bind(&reponse.type_contenu)
        .bind(&reponse.emplacement)
        .bind(&reponse.corps)
        .bind(reponse.rejouable)
        .bind(maintenant)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// Libère une clé revendiquée dont l'écriture a échoué (erreur, refus) : un nouvel essai s'exécutera pour de bon.
    pub async fn idem_abandonner(
        &self,
        etablissement_id: Uuid,
        session_id: Uuid,
        cle: Uuid,
    ) -> Result<(), sqlx::Error> {
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await?;
        sqlx::query(
            "DELETE FROM ent_idempotence_cle
              WHERE etablissement_id = $1 AND session_id = $2 AND cle = $3 AND statut = 'en_cours'",
        )
        .bind(etablissement_id)
        .bind(session_id)
        .bind(cle)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// Purge de tous les établissements : supprime les clés de plus de 24 h à l'instant donné. Retourne le nombre de
    /// lignes supprimées. (Sous RLS stricte, établissement par établissement.)
    pub async fn idem_purger(&self, maintenant: DateTime<Utc>) -> Result<u64, sqlx::Error> {
        let limite = maintenant - Duration::hours(IDEMPOTENCE_DUREE_H);
        let etabs = etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("purge_idempotence")).await?;
        let mut total = 0;
        for e in etabs {
            let mut tx = TenantTx::begin(self.pool(), e).await?;
            total += sqlx::query(
                "DELETE FROM ent_idempotence_cle WHERE etablissement_id = $1 AND cree_le < $2",
            )
            .bind(e)
            .bind(limite)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            tx.commit().await?;
        }
        Ok(total)
    }
}

/// Empreinte d'une requête d'écriture : SHA-256 (hexadécimal) de la méthode, du chemin avec sa requête et du corps,
/// chaque partie précédée de sa longueur (aucune ambiguïté de frontière). La même clé sur une autre route ou avec un
/// autre corps donne une autre empreinte.
pub fn empreinte_requete(methode: &str, chemin_et_requete: &str, corps: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for part in [methode.as_bytes(), chemin_et_requete.as_bytes(), corps] {
        h.update((part.len() as u64).to_be_bytes());
        h.update(part);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
