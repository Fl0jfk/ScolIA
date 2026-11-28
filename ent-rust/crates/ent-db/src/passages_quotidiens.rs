//! Passages quotidiens branchés sur le planificateur d'`ent-api` (suites du 18 novembre) :
//!
//! 1. **alertes d'échéance des acomptes fournisseurs** des voyages (V2a, VOY-270) ;
//! 2. **purge des exports CSV de l'argent des voyages** à 30 jours (catégorie de conservation
//!    `sor.export_argent`, F7) ;
//! 3. **balayage des objets orphelins** : un objet écrit au magasin et inscrit au registre (`ent_objet`) mais
//!    jamais référencé par sa ligne métier parce que le processus s'est arrêté entre l'écriture et le commit
//!    (photos d'élèves, F2 d'E9 ; exports de l'argent des voyages, F7) ;
//! 4. **arrêt automatique des notes** à la date d'arrêt de la période (socle 21, `arret_auto`, M8) : chaque classe
//!    encore ouverte passe en `notes_arretees` avec l'origine `auto`, au jour de l'établissement, une seule fois
//!    (une classe rouverte par la direction n'est jamais ré-arrêtée) ; puis la **fin de période** du lot V8
//!    (`eva_v8_passage_au`) : réouvertures échues, publication à l'arrêt et instantané, clôture des appréciations la
//!    veille du conseil, rappels J-7 / J-2.
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

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::objets::marquer_objet_supprime;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme, TenantTx};

/// Retard maximal d'une publication programmée (F7) : période de la boucle dédiée d'`ent-api` (5 minutes) + une minute
/// de temps de passage. Une évaluation programmée à l'heure H est publiée au plus tard à H + ce délai.
pub const RETARD_MAX_PUBLICATION_PROGRAMMEE_SECONDES: i64 = 6 * 60;

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
    /// Classes dont les notes ont été arrêtées automatiquement à la date d'arrêt de la période (socle 21, M8).
    pub periodes_arretees: usize,
    /// Évaluations publiées automatiquement (programmées ou au délai maximal) et rappels envoyés (socle 21, V3).
    pub publications_automatiques: usize,
    /// Fin de période (socle 21, V8) : réouvertures échues refermées, arrêts finalisés (publication et instantané),
    /// appréciations closes la veille du conseil, rappels J-7 / J-2 aux professeurs.
    pub fin_de_periode: usize,
    /// Étapes en échec (journalisées ; l'étape suivante et l'établissement suivant sont traités).
    pub erreurs: usize,
}

impl BilanPassagesQuotidiens {
    fn ajouter(&mut self, autre: &Self) {
        self.acomptes_alertes += autre.acomptes_alertes;
        self.exports_purges += autre.exports_purges;
        self.photos_orphelines += autre.photos_orphelines;
        self.exports_orphelins += autre.exports_orphelins;
        self.periodes_arretees += autre.periodes_arretees;
        self.publications_automatiques += autre.publications_automatiques;
        self.fin_de_periode += autre.fin_de_periode;
        self.erreurs += autre.erreurs;
    }
}

impl PgEnt {
    /// Passages quotidiens de tous les établissements, au jour `jour` (`None` : le jour de chaque établissement).
    /// Idempotent. Une étape en échec est comptée et journalisée sans bloquer les suivantes.
    pub async fn passages_quotidiens(&self, jour: Option<NaiveDate>) -> Result<BilanPassagesQuotidiens, EntDbError> {
        self.passages_quotidiens_a(jour, None).await
    }

    /// Comme [`Self::passages_quotidiens`], avec l'instant `maintenant` **fourni** pour les publications automatiques
    /// des évaluations (`None` : l'horloge réelle si `jour` est `None`, sinon la FIN du jour demandé, heure de
    /// l'établissement : un jour explicite veut dire « tout ce qui est dû d'ici la fin de ce jour »).
    pub async fn passages_quotidiens_a(
        &self,
        jour: Option<NaiveDate>,
        maintenant: Option<DateTime<Utc>>,
    ) -> Result<BilanPassagesQuotidiens, EntDbError> {
        let etabs = etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("passages_quotidiens"))
            .await
            .map_err(EntDbError::from)?;
        let mut b = BilanPassagesQuotidiens { etablissements: etabs.len(), ..Default::default() };
        for etab in etabs {
            b.ajouter(&self.passages_quotidiens_etablissement_a(etab, jour, maintenant).await);
        }
        Ok(b)
    }

    /// Publications automatiques des évaluations pour TOUS les établissements à l'instant fourni (F7). Planificateur
    /// dédié, **toutes les 5 minutes** (`ent-api`) : une publication programmée sort au plus
    /// [`RETARD_MAX_PUBLICATION_PROGRAMMEE_SECONDES`] après l'heure choisie (période de la boucle + temps de passage),
    /// et non plus jusqu'à une heure plus tard comme avec le passage horaire.
    pub async fn eva_v3_publications_automatiques_tous(
        &self,
        maintenant: DateTime<Utc>,
    ) -> Result<crate::evaluations_v3_eval::BilanPublications, EntDbError> {
        let etabs = etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("publications_evaluations"))
            .await
            .map_err(EntDbError::from)?;
        let mut total = crate::evaluations_v3_eval::BilanPublications::default();
        for etab in etabs {
            let jour = match self.aujourdhui_etablissement(etab).await {
                Ok(j) => j,
                Err(e) => {
                    tracing::warn!(%etab, erreur = classe_erreur(&e), "publications automatiques : jour de l'établissement illisible");
                    continue;
                }
            };
            match self.eva_v3_publications_automatiques_au(etab, maintenant, jour).await {
                Ok(b) => {
                    total.programmees += b.programmees;
                    total.delai_max += b.delai_max;
                    total.rappels += b.rappels;
                }
                Err(e) => tracing::warn!(%etab, erreur = classe_erreur(&e), "publications automatiques des évaluations en échec"),
            }
        }
        Ok(total)
    }

    /// Passages quotidiens d'**un** établissement (les étapes sont indépendantes : une étape en échec est comptée et
    /// journalisée, les suivantes s'exécutent).
    pub async fn passages_quotidiens_etablissement(&self, etab: Uuid, jour: Option<NaiveDate>) -> BilanPassagesQuotidiens {
        self.passages_quotidiens_etablissement_a(etab, jour, None).await
    }

    /// Voir [`Self::passages_quotidiens_a`] pour `maintenant`.
    pub async fn passages_quotidiens_etablissement_a(
        &self,
        etab: Uuid,
        jour: Option<NaiveDate>,
        maintenant: Option<DateTime<Utc>>,
    ) -> BilanPassagesQuotidiens {
        let mut b = BilanPassagesQuotidiens { etablissements: 1, ..Default::default() };
        match self.voyages_acomptes_alertes_traiter_au(etab, jour).await {
            Ok(n) => b.acomptes_alertes += n,
            Err(e) => {
                b.erreurs += 1;
                tracing::warn!(%etab, erreur = classe_erreur(&e), "passage quotidien : alertes d'acomptes en échec");
            }
        }
        match self.voyages_argent_exports_purger_au(etab, jour).await {
            Ok(n) => b.exports_purges += n,
            Err(e) => {
                b.erreurs += 1;
                tracing::warn!(%etab, erreur = classe_erreur(&e), "passage quotidien : purge des exports en échec");
            }
        }
        // Arrêt automatique des notes à la date d'arrêt (réglage `arret_auto`), au jour de l'établissement.
        match self.eva_v1_arret_auto_au(etab, jour).await {
            Ok(n) => b.periodes_arretees += n,
            Err(e) => {
                b.erreurs += 1;
                tracing::warn!(%etab, erreur = classe_erreur(&e), "passage quotidien : arrêt automatique des notes en échec");
            }
        }
        // Publications automatiques des évaluations (programmées, délai maximal, rappels) : horloge fournie.
        let jour_eval = match jour {
            Some(j) => Ok(j),
            None => self.aujourdhui_etablissement(etab).await,
        };
        // F7 : l'instant est celui qu'on fournit ; à défaut, l'horloge réelle (jour implicite) ou la FIN du jour demandé
        // à l'heure de l'établissement (jour explicite). Avant, un jour explicite figeait « maintenant » à 12 h UTC :
        // une publication programmée à 14 h ne sortait pas ce jour-là.
        let maintenant = match (maintenant, jour) {
            (Some(m), _) => m,
            (None, None) => Utc::now(),
            (None, Some(j)) => match self.fin_du_jour_etablissement(etab, j).await {
                Ok(m) => m,
                Err(e) => {
                    b.erreurs += 1;
                    tracing::warn!(%etab, erreur = classe_erreur(&e), "passage quotidien : fin du jour de l'établissement illisible");
                    Utc::now()
                }
            },
        };
        // V8 : réouvertures échues, publication et instantané des classes arrêtées, clôture des appréciations la veille
        // du conseil, rappels J-7 / J-2 (après l'arrêt automatique, pour finaliser les classes qu'il vient d'arrêter).
        match self.eva_v8_passage_au(etab, jour, maintenant).await {
            Ok(bilan) => b.fin_de_periode += bilan.total(),
            Err(e) => {
                b.erreurs += 1;
                tracing::warn!(%etab, erreur = classe_erreur(&e), "passage quotidien : fin de période des évaluations en échec");
            }
        }
        match jour_eval {
            Ok(j) => match self.eva_v3_publications_automatiques_au(etab, maintenant, j).await {
                Ok(bilan) => b.publications_automatiques += bilan.programmees + bilan.delai_max + bilan.rappels,
                Err(e) => {
                    b.erreurs += 1;
                    tracing::warn!(%etab, erreur = classe_erreur(&e), "passage quotidien : publications automatiques des évaluations en échec");
                }
            },
            Err(e) => {
                b.erreurs += 1;
                tracing::warn!(%etab, erreur = classe_erreur(&e), "passage quotidien : jour de l'établissement illisible");
            }
        }
        for (famille, compteur) in [(&PHOTOS, &mut b.photos_orphelines), (&EXPORTS_ARGENT, &mut b.exports_orphelins)] {
            match self.balayer_objets_orphelins(etab, famille, jour).await {
                Ok(n) => *compteur += n,
                Err(e) => {
                    b.erreurs += 1;
                    tracing::warn!(%etab, famille = famille.nom, erreur = classe_erreur(&e), "passage quotidien : balayage des objets en échec");
                }
            }
        }
        b
    }

    /// Dernière seconde du jour `jour`, heure de l'établissement, en UTC.
    async fn fin_du_jour_etablissement(&self, etab: Uuid, jour: NaiveDate) -> Result<DateTime<Utc>, EntDbError> {
        let mut t = TenantTx::begin(self.pool(), etab).await?;
        let m: DateTime<Utc> = sqlx::query_scalar(
            "SELECT (($1::date + 1)::timestamp AT TIME ZONE COALESCE(
                       (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $2), 'Europe/Paris'))
                    - interval '1 second'",
        )
        .bind(jour)
        .bind(etab)
        .fetch_one(&mut *t)
        .await?;
        Ok(m)
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
                tracing::warn!(%etab, famille = famille.nom, erreur = classe_erreur(&e), "objet orphelin non supprimé : repris au prochain passage");
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

/// V3c : ce qu'un passage quotidien journalise d'une erreur. **Jamais le message** : un message d'erreur de base
/// (clé dupliquée, ligne en cause, « DETAIL ») ou un refus métier peut citer un nom, un identifiant d'élève ou une
/// valeur. On ne garde qu'une famille et, pour la base, le code SQLSTATE (5 caractères, sans donnée).
fn classe_erreur(e: &EntDbError) -> String {
    match e {
        EntDbError::Sqlx(sqlx::Error::Database(d)) => {
            format!("base:{}", d.code().as_deref().unwrap_or("?"))
        }
        EntDbError::Sqlx(_) => "base".to_string(),
        EntDbError::Forbidden => "interdit".to_string(),
        EntDbError::SaisieInvalide(_) => "saisie_invalide".to_string(),
        EntDbError::ConflitDossier(_) => "conflit".to_string(),
        _ => "autre".to_string(),
    }
}

#[cfg(test)]
mod tests_journaux {
    use super::*;

    /// V3c : un passage quotidien ne journalise jamais le message d'une erreur (donnée d'élève possible), seulement une
    /// famille et, pour la base, le code SQLSTATE.
    #[test]
    fn la_classe_d_erreur_ne_contient_aucune_donnee_personnelle() {
        let e = EntDbError::SaisieInvalide("Dupont Marie, élève de 4A, absente le 12");
        let c = classe_erreur(&e);
        assert_eq!(c, "saisie_invalide");
        assert!(!c.contains("Dupont"));
        assert_eq!(classe_erreur(&EntDbError::Forbidden), "interdit");
        assert_eq!(classe_erreur(&EntDbError::ConflitDossier("Dupont")), "conflit");
        assert_eq!(classe_erreur(&EntDbError::Sqlx(sqlx::Error::RowNotFound)), "base");
    }
}
