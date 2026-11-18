//! Passages quotidiens branchés sur le planificateur d'`ent-api` (suites du 18 novembre) :
//!
//! 1. **alertes d'échéance des acomptes fournisseurs** des voyages (V2a, VOY-270) ;
//! 2. **purge des exports CSV de l'argent des voyages** à 30 jours (catégorie de conservation
//!    `sor.export_argent`, F7) ;
//! 3. **balayage des objets orphelins** : un objet écrit au magasin et inscrit au registre (`ent_objet`) mais
//!    jamais référencé par sa ligne métier parce que le processus s'est arrêté entre l'écriture et le commit
//!    (photos d'élèves, F2 d'E9 ; exports de l'argent des voyages, F7).
//!
//! Même modèle que `tache_nocturne_eleves` : **idempotent**, lancé toutes les heures (le jour dépend du fuseau de
//! chaque établissement), désactivable par `ENT_TACHES_VOYAGES_OBJETS=0`, un échec sur un établissement ne bloque
//! pas les autres. Aucune horloge réelle n'est exigée : [`PgEnt::passages_quotidiens`] prend le jour à utiliser
//! (`None` : le jour de chaque établissement), ce qui permet aux tests de choisir le jour.
//!
//! **Balayage** (sûreté avant tout) :
//! * **délai de grâce** : un objet n'est jamais supprimé avant [`GRACE_OBJETS_JOURS`] jours de l'établissement après
//!   son inscription au registre (une écriture encore en cours ou un commit lent ne sont jamais touchés) ;
//! * **borné** : au plus [`LOT_BALAYAGE`] objets par famille, par établissement et par passage (le reste attend le
//!   passage suivant) ;
//! * **périmètre fermé** : seules deux familles de clés sont balayées, chacune avec sa table de référence — photos
//!   d'élèves de forme immuable `{etab}/trombi/{eleve}/{uuid}.{ext}` (la forme historique n'est jamais touchée) et
//!   exports `{etab}/voyages-argent/{uuid}.csv` ;
//! * **jamais un objet référencé** : la requête exclut toute clé présente dans la table de référence ;
//! * **journalisé** : une ligne de journal d'application par passage, par établissement et par famille (nombres
//!   seulement, jamais de clé : celle d'une photo contient l'identifiant de l'élève) ; la trace durable de chaque
//!   objet balayé est la date `supprime_le` du registre des objets (immuable) ;
//! * **ordre** : l'objet physique est supprimé d'abord, puis le registre est marqué ; une suppression qui échoue
//!   laisse la ligne du registre intacte, l'objet est repris au passage suivant.

use chrono::NaiveDate;
use uuid::Uuid;

use crate::objets::marquer_objet_supprime;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme, TenantTx};

/// Jours (de l'établissement) pendant lesquels un objet inscrit au registre mais non référencé n'est jamais supprimé.
pub const GRACE_OBJETS_JOURS: i32 = 2;
/// Objets balayés au plus, par famille, par établissement et par passage.
pub const LOT_BALAYAGE: i64 = 200;

/// Une famille d'objets balayée : requête **statique** qui rend `(id du registre, clé)` des objets non référencés
/// passé le délai de grâce. `$1` établissement, `$2` jour de référence (`NULL` : jour de l'établissement),
/// `$3` délai de grâce en jours, `$4` borne.
struct Famille {
    nom: &'static str,
    sql: &'static str,
}

const PHOTOS: Famille = Famille {
    nom: "photos_eleves",
    sql: "SELECT o.id, o.cle_objet FROM ent_objet o
           WHERE o.etablissement_id = $1 AND o.supprime_le IS NULL
             AND o.cle_objet LIKE $1::text || '/trombi/%/%'
             AND ent_aujourdhui_etab($1, o.cree_le) + $3::int <= COALESCE($2::date, ent_aujourdhui_etab($1, now()))
             AND NOT EXISTS (SELECT 1 FROM ent_eleve_photo p
                              WHERE p.etablissement_id = $1 AND p.object_key = o.cle_objet)
             AND NOT EXISTS (SELECT 1 FROM ent_eleve e
                              WHERE e.etablissement_id = $1 AND e.photo_key = o.cle_objet)
           ORDER BY o.cree_le, o.id LIMIT $4",
};

const EXPORTS_ARGENT: Famille = Famille {
    nom: "exports_argent_voyages",
    sql: "SELECT o.id, o.cle_objet FROM ent_objet o
           WHERE o.etablissement_id = $1 AND o.supprime_le IS NULL
             AND o.cle_objet LIKE $1::text || '/voyages-argent/%'
             AND ent_aujourdhui_etab($1, o.cree_le) + $3::int <= COALESCE($2::date, ent_aujourdhui_etab($1, now()))
             AND NOT EXISTS (SELECT 1 FROM ent_sortie_argent_export x
                              WHERE x.etablissement_id = $1 AND x.cle_objet = o.cle_objet)
           ORDER BY o.cree_le, o.id LIMIT $4",
};

/// Bilan d'un passage (tous établissements).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BilanPassagesQuotidiens {
    pub etablissements: usize,
    /// Alertes d'échéance d'acomptes fournisseurs émises.
    pub acomptes_alertes: usize,
    /// Fichiers d'export de l'argent des voyages détruits (30 jours).
    pub exports_purges: usize,
    /// Objets de photo orphelins balayés.
    pub photos_orphelines: usize,
    /// Objets d'export orphelins balayés.
    pub exports_orphelins: usize,
    /// Étapes en échec (journalisées ; l'étape suivante et l'établissement suivant sont traités).
    pub erreurs: usize,
}

impl BilanPassagesQuotidiens {
    fn ajouter(&mut self, autre: &Self) {
        self.acomptes_alertes += autre.acomptes_alertes;
        self.exports_purges += autre.exports_purges;
        self.photos_orphelines += autre.photos_orphelines;
        self.exports_orphelins += autre.exports_orphelins;
        self.erreurs += autre.erreurs;
    }
}

impl PgEnt {
    /// Passages quotidiens de tous les établissements, au jour `jour` (`None` : le jour de chaque établissement).
    /// Idempotent. Une étape en échec est comptée et journalisée sans bloquer les suivantes.
    pub async fn passages_quotidiens(&self, jour: Option<NaiveDate>) -> Result<BilanPassagesQuotidiens, EntDbError> {
        let etabs = etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("passages_quotidiens"))
            .await
            .map_err(EntDbError::from)?;
        let mut b = BilanPassagesQuotidiens { etablissements: etabs.len(), ..Default::default() };
        for etab in etabs {
            b.ajouter(&self.passages_quotidiens_etablissement(etab, jour).await);
        }
        Ok(b)
    }

    /// Passages quotidiens d'**un** établissement (les étapes sont indépendantes : une étape en échec est comptée et
    /// journalisée, les suivantes s'exécutent).
    pub async fn passages_quotidiens_etablissement(&self, etab: Uuid, jour: Option<NaiveDate>) -> BilanPassagesQuotidiens {
        let mut b = BilanPassagesQuotidiens { etablissements: 1, ..Default::default() };
        match self.voyages_acomptes_alertes_traiter_au(etab, jour).await {
            Ok(n) => b.acomptes_alertes += n,
            Err(e) => {
                b.erreurs += 1;
                tracing::warn!(%etab, erreur = %e, "passage quotidien : alertes d'acomptes en échec");
            }
        }
        match self.voyages_argent_exports_purger_au(etab, jour).await {
            Ok(n) => b.exports_purges += n,
            Err(e) => {
                b.erreurs += 1;
                tracing::warn!(%etab, erreur = %e, "passage quotidien : purge des exports en échec");
            }
        }
        for (famille, compteur) in [(&PHOTOS, &mut b.photos_orphelines), (&EXPORTS_ARGENT, &mut b.exports_orphelins)] {
            match self.balayer_objets_orphelins(etab, famille, jour).await {
                Ok(n) => *compteur += n,
                Err(e) => {
                    b.erreurs += 1;
                    tracing::warn!(%etab, famille = famille.nom, erreur = %e, "passage quotidien : balayage des objets en échec");
                }
            }
        }
        b
    }

    /// Balaie les objets orphelins d'une famille pour un établissement (voir la documentation du module).
    /// Retourne le nombre d'objets supprimés.
    async fn balayer_objets_orphelins(
        &self,
        etab: Uuid,
        famille: &Famille,
        jour: Option<NaiveDate>,
    ) -> Result<usize, EntDbError> {
        let candidats: Vec<(Uuid, String)> = {
            let mut t = TenantTx::begin(self.pool(), etab).await?;
            sqlx::query_as(famille.sql)
                .bind(etab)
                .bind(jour)
                .bind(GRACE_OBJETS_JOURS)
                .bind(LOT_BALAYAGE)
                .fetch_all(&mut *t)
                .await?
        };
        let mut balayes = 0usize;
        let mut echecs = 0usize;
        for (_, cle) in &candidats {
            // L'objet physique d'abord ; le registre n'est marqué qu'une fois la suppression faite.
            if let Err(e) = self.objet_supprimer(cle).await {
                echecs += 1;
                tracing::warn!(%etab, famille = famille.nom, erreur = %e, "objet orphelin non supprimé : repris au prochain passage");
                continue;
            }
            let mut t = TenantTx::begin(self.pool(), etab).await?;
            marquer_objet_supprime(&mut *t, etab, cle).await?;
            t.commit().await?;
            balayes += 1;
        }
        if balayes > 0 || echecs > 0 {
            // Nombres seulement : la clé d'une photo porte l'identifiant de l'élève.
            tracing::info!(
                %etab,
                famille = famille.nom,
                balayes,
                echecs,
                borne = LOT_BALAYAGE,
                reste_possible = candidats.len() as i64 >= LOT_BALAYAGE,
                "balayage des objets orphelins"
            );
        }
        Ok(balayes)
    }
}
