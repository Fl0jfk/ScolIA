//! Socle 7, lot D5b — tâches de fond des pièces justificatives.
//!
//! * **Analyse antivirus** des pièces en quarantaine, pour tous les établissements, au rythme de l'ordonnanceur
//!   de `ent-api`. Fermée par défaut : sans démon (`CLAMAV_ADDR` absente) la tâche n'est pas lancée et rien ne sort
//!   de quarantaine ; démon injoignable → les pièces y restent (réessais comptés), et la passe s'arrête après trois
//!   indisponibilités de suite (un démon éteint ne coûte pas trente secondes par pièce).
//! * **Purge des objets des pièces infectées** (D9) : l'objet reçu d'une pièce `infectee` est détruit au magasin
//!   après un délai de rétention, puis marqué supprimé au registre. La ligne de la pièce et ses décisions restent
//!   (preuve) ; le **journal** (`ent_doc_journal`) ne porte ni nom de fichier, ni nom de personne, ni clé d'objet :
//!   l'identifiant de la pièce seulement.

use std::sync::Arc;

use ent_object::upload::AnalyseurAntivirus;
use uuid::Uuid;

use crate::documents_pieces::{systeme, RapportAnalyse, ResultatAnalyse};
use crate::documents_repo::journaliser;
use crate::ref_commun::db;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::{lire_par_etablissement, ContexteSysteme, TenantTx};

/// Indisponibilités consécutives de l'antivirus après lesquelles la passe s'arrête.
pub const INDISPONIBLES_AVANT_ARRET: usize = 3;
/// Rétention par défaut d'un objet infecté avant sa destruction (jours).
pub const RETENTION_INFECTEES_JOURS: i64 = 7;

/// L'antivirus configuré par `CLAMAV_ADDR` (démon `clamd`, `hôte:port`). `None` si la variable est absente ou vide :
/// aucune analyse n'est alors lancée et toute pièce reste en quarantaine (fermé par défaut). Jamais d'antivirus
/// « factice » ni de repli : sans démon, pas de pièce servie.
pub fn antivirus_depuis_env() -> Option<Arc<dyn AnalyseurAntivirus>> {
    ent_object::clamav::ClamAv::depuis_env().map(|c| Arc::new(c) as Arc<dyn AnalyseurAntivirus>)
}

/// Rétention avant destruction d'un objet infecté : `ENT_PIECES_INFECTEES_RETENTION_JOURS` (0 à 365 jours),
/// sinon [`RETENTION_INFECTEES_JOURS`].
pub fn retention_infectees_depuis_env() -> i64 {
    std::env::var("ENT_PIECES_INFECTEES_RETENTION_JOURS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|j| (0..=365).contains(j))
        .unwrap_or(RETENTION_INFECTEES_JOURS)
}

/// Bilan d'une passe d'analyse.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BilanAnalyseTache {
    pub analyses: RapportAnalyse,
    /// La passe s'est arrêtée parce que l'antivirus ne répondait pas.
    pub antivirus_indisponible: bool,
}

/// Bilan d'une purge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BilanPurgePieces {
    /// Objets détruits au magasin et marqués au registre.
    pub purges: usize,
    /// Pièces dont la destruction a échoué (réessayées à la prochaine passe).
    pub echecs: usize,
}

impl PgEnt {
    /// Une passe de la tâche d'analyse, pour tous les établissements ayant des pièces en quarantaine.
    pub async fn analyser_pieces_tache(
        &self,
        antivirus: Arc<dyn AnalyseurAntivirus>,
        limite: i64,
    ) -> Result<BilanAnalyseTache, EntDbError> {
        let etabs: Vec<(Uuid,)> = lire_par_etablissement(
            self.pool(),
            ContexteSysteme::JobPlanifie("pieces_analyse"),
            "SELECT DISTINCT etablissement_id FROM ent_doc_piece WHERE etat = 'en_quarantaine'",
        )
        .await?;
        let mut total = BilanAnalyseTache::default();
        for (etab,) in etabs {
            let b = self
                .analyser_pieces_etablissement(etab, antivirus.clone(), limite)
                .await?;
            total.analyses.recues += b.analyses.recues;
            total.analyses.en_quarantaine += b.analyses.en_quarantaine;
            total.analyses.infectees += b.analyses.infectees;
            total.analyses.rejetees += b.analyses.rejetees;
            if b.antivirus_indisponible {
                // Un démon éteint l'est pour tous les établissements : inutile d'insister ailleurs.
                total.antivirus_indisponible = true;
                break;
            }
        }
        Ok(total)
    }

    /// Analyse jusqu'à `limite` pièces en quarantaine d'un établissement : les moins essayées d'abord (une pièce
    /// que le démon refuse de lire ne bloque pas les suivantes), puis les plus anciennes. S'arrête après
    /// [`INDISPONIBLES_AVANT_ARRET`] indisponibilités de suite.
    pub async fn analyser_pieces_etablissement(
        &self,
        etab: Uuid,
        antivirus: Arc<dyn AnalyseurAntivirus>,
        limite: i64,
    ) -> Result<BilanAnalyseTache, EntDbError> {
        let mut bilan = BilanAnalyseTache::default();
        let mut de_suite = 0usize;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_doc_piece WHERE etablissement_id = $1 AND etat = 'en_quarantaine'
              ORDER BY analyse_tentatives, deposee_le, id LIMIT $2",
        )
        .bind(etab)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        for (id,) in ids {
            match self.analyser_piece(etab, id, antivirus.clone()).await {
                Ok(ResultatAnalyse::Recue) => {
                    bilan.analyses.recues += 1;
                    de_suite = 0;
                }
                Ok(ResultatAnalyse::Infectee) => {
                    bilan.analyses.infectees += 1;
                    de_suite = 0;
                }
                Ok(ResultatAnalyse::Rejetee(_)) => {
                    bilan.analyses.rejetees += 1;
                    de_suite = 0;
                }
                Ok(ResultatAnalyse::DejaTraitee) => {}
                Ok(ResultatAnalyse::EnQuarantaine) => {
                    bilan.analyses.en_quarantaine += 1;
                    de_suite += 1;
                    if de_suite >= INDISPONIBLES_AVANT_ARRET {
                        bilan.antivirus_indisponible = true;
                        break;
                    }
                }
                // Un échec technique sur une pièce (objet illisible…) ne bloque pas les autres.
                Err(e) => tracing::warn!(piece = %id, erreur = %e, "analyse d'une pièce impossible"),
            }
        }
        Ok(bilan)
    }

    /// Une passe de la purge des objets de pièces infectées, pour tous les établissements.
    pub async fn purger_objets_pieces_infectees_tache(
        &self,
        retention_jours: i64,
    ) -> Result<BilanPurgePieces, EntDbError> {
        let etabs: Vec<(Uuid,)> = lire_par_etablissement(
            self.pool(),
            ContexteSysteme::JobPlanifie("pieces_infectees_purge"),
            "SELECT DISTINCT p.etablissement_id FROM ent_doc_piece p
               JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.cle_objet = p.cle_objet_original
              WHERE p.etat = 'infectee' AND o.supprime_le IS NULL",
        )
        .await?;
        let mut bilan = BilanPurgePieces::default();
        for (etab,) in etabs {
            let b = self.purger_objets_pieces_infectees(etab, retention_jours).await?;
            bilan.purges += b.purges;
            bilan.echecs += b.echecs;
        }
        Ok(bilan)
    }

    /// Détruit les objets reçus des pièces **infectées** d'un établissement dont l'analyse date d'au moins
    /// `retention_jours` jours. L'objet est détruit au magasin **avant** d'être marqué supprimé au registre :
    /// une panne entre les deux se rattrape à la passe suivante (suppression idempotente). Un objet que
    /// référence une autre pièce n'est jamais détruit.
    pub async fn purger_objets_pieces_infectees(
        &self,
        etab: Uuid,
        retention_jours: i64,
    ) -> Result<BilanPurgePieces, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a_purger: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT p.id, p.cle_objet_original
               FROM ent_doc_piece p
               JOIN ent_objet o ON o.etablissement_id = p.etablissement_id AND o.cle_objet = p.cle_objet_original
              WHERE p.etablissement_id = $1 AND p.etat = 'infectee' AND o.supprime_le IS NULL
                AND p.derniere_analyse_le <= now() - make_interval(days => $2::int)
                AND NOT EXISTS (SELECT 1 FROM ent_doc_piece q
                                 WHERE q.etablissement_id = p.etablissement_id AND q.id <> p.id
                                   AND (q.cle_objet_original = p.cle_objet_original OR q.cle_objet_servi = p.cle_objet_original))
              ORDER BY p.derniere_analyse_le, p.id
              LIMIT 200",
        )
        .bind(etab)
        .bind(retention_jours.clamp(0, 3650) as i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;

        let sys = systeme(etab);
        let mut bilan = BilanPurgePieces::default();
        for (piece_id, cle) in a_purger {
            if let Err(e) = self.objet_supprimer(&cle).await {
                // Ni la clé ni aucun nom dans le journal d'exploitation : l'identifiant de la pièce suffit.
                tracing::warn!(piece = %piece_id, erreur = %e, "purge d'une pièce infectée : destruction impossible");
                bilan.echecs += 1;
                continue;
            }
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let r = sqlx::query(
                "UPDATE ent_objet SET supprime_le = now()
                  WHERE etablissement_id = $1 AND cle_objet = $2 AND supprime_le IS NULL",
            )
            .bind(etab)
            .bind(&cle)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if r.rows_affected() == 1 {
                journaliser(
                    &mut tx,
                    &sys,
                    None,
                    None,
                    "piece_objets_purges",
                    (Some("infectee"), Some("infectee")),
                    Some(&piece_id.to_string()),
                    Some("fichier reçu détruit après analyse positive"),
                )
                .await?;
                bilan.purges += 1;
            }
            tx.commit().await.map_err(db)?;
        }
        Ok(bilan)
    }
}
