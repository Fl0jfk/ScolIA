//! Socle 32, lots ET0 et ET1 — tâches de fond des espaces : analyse antivirus, libération des réservations expirées, purge
//! de la corbeille (30 jours), élagage des versions (S-3210), réconciliation des compteurs de quota
//! (`32_BUREAUTIQUE.md` §10, §15 ; CET-14, CET-15, CET-18, CET-20, CET-21 ; R-ET6).
//!
//! * Chaque fonction travaille pour **un** établissement (transaction locataire) et prend l'**instant** en paramètre : aucune
//!   dépendance à l'heure réelle ; les variantes `_tous_etablissements` parcourent les établissements pour l'ordonnanceur.
//! * **Analyse fermée par défaut** : sans verdict « sain » (antivirus absent ou indisponible), un fichier reste `analyse`, donc
//!   invisible des autres membres. Un fichier infecté passe en quarantaine, son quota est rendu et son objet effacé ; l'auteur
//!   et les animateurs sont prévenus (avis neutre).
//! * **Jamais de version figée par un autre socle élaguée ou effacée** (R-ET6).
//! * L'effacement physique d'un objet n'a lieu qu'**après** la validation de la transaction qui l'a marqué supprimé.

use crate::espaces::*;
use crate::espaces_contenu::purger_element;
use crate::espaces_quota::{occupation_reconcilier, quota_rendre, reservations_expirees_liberer};
use crate::espaces_service::*;
use crate::notifications::db;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::{etablissements_systeme, ContexteSysteme, TenantTx};
use chrono::{DateTime, Duration, Utc};
use ent_object::upload::{AnalyseurAntivirus, ContexteAnalyse, TypeFichier, UsageFichier, VerdictAnalyse};
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Bilan d'une passe d'analyse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BilanAnalyse {
    pub saines: usize,
    pub infectees: usize,
    /// Antivirus indisponible ou objet illisible : le fichier reste invisible (fermé par défaut).
    pub indisponibles: usize,
}

/// Bilan d'une passe de purge de la corbeille.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BilanPurge {
    pub effaces: usize,
    /// Éléments épargnés parce qu'une version est figée par un autre socle (R-ET6).
    pub gardes: usize,
}

impl PgEnt {
    /// Analyse les versions déposées en attente (`analyse`), au plus `limite` par passe. L'appel à l'antivirus se fait dans
    /// un fil bloquant (client réseau synchrone) ; un verdict « indisponible » laisse la version invisible et compte une tentative.
    pub async fn espaces_analyser_versions(
        &self,
        etab: Uuid,
        antivirus: Arc<dyn AnalyseurAntivirus>,
        limite: i64,
        maintenant: DateTime<Utc>,
    ) -> Result<BilanAnalyse, EntDbError> {
        let attente = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let l = sqlx::query(
                "SELECT v.id, v.element_id, v.numero, v.analyse_tentatives, o.cle_objet, v.type_reel, e.nom, e.espace_id,
                        v.auteur_personne_id, v.taille, o.id
                   FROM ent_espace_version v
                   JOIN ent_espace_element e ON e.etablissement_id = v.etablissement_id AND e.id = v.element_id
                   JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
                  WHERE v.etablissement_id = $1 AND v.etat = 'analyse'
                  ORDER BY v.cree_le, v.numero
                  LIMIT $2",
            )
            .bind(etab)
            .bind(limite.clamp(1, 200))
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            l
        };
        let mut bilan = BilanAnalyse::default();
        for r in &attente {
            let version_id: Uuid = r.get(0);
            let element_id: Uuid = r.get(1);
            let numero: i32 = r.get(2);
            let tentatives: i32 = r.get(3);
            let cle: String = r.get(4);
            let type_reel: String = r.get(5);
            let nom: String = r.get(6);
            let espace_id: Uuid = r.get(7);
            let auteur: Uuid = r.get(8);
            let taille: i64 = r.get(9);
            let objet_id: Uuid = r.get(10);
            // Lecture contrôlée contre le registre (taille et empreinte) : un objet altéré n'est jamais déclaré sain.
            let octets = match self.lire_objet(etab, &cle).await {
                Ok(o) => o,
                Err(e) => {
                    tracing::warn!(erreur = %e, "espaces : objet illisible, analyse reportée (fermé par défaut)");
                    self.noter_tentative(etab, version_id, "objet illisible", tentatives, maintenant).await?;
                    bilan.indisponibles += 1;
                    continue;
                }
            };
            let av = antivirus.clone();
            let type_fichier = TypeFichier::depuis_mime(&type_reel).unwrap_or(TypeFichier::Texte);
            let nom_a = nom.clone();
            let verdict = tokio::task::spawn_blocking(move || {
                av.analyser(&ContexteAnalyse { usage: UsageFichier::Espace, type_fichier, nom: &nom_a, octets: &octets })
            })
            .await
            .unwrap_or(VerdictAnalyse::Indisponible);
            match verdict {
                VerdictAnalyse::Indisponible => {
                    self.noter_tentative(etab, version_id, "antivirus indisponible", tentatives, maintenant).await?;
                    bilan.indisponibles += 1;
                }
                VerdictAnalyse::Sain => {
                    let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                    let r = sqlx::query(
                        "UPDATE ent_espace_version SET etat = 'disponible', derniere_analyse_le = $3, analyse_motif = NULL
                          WHERE etablissement_id = $1 AND id = $2 AND etat = 'analyse'",
                    )
                    .bind(etab)
                    .bind(version_id)
                    .bind(maintenant)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    if r.rows_affected() == 1 {
                        // La version courante est la dernière version disponible ; le fichier devient visible.
                        sqlx::query(
                            "UPDATE ent_espace_element e
                                SET version_courante_id = (SELECT v.id FROM ent_espace_version v
                                                            WHERE v.etablissement_id = e.etablissement_id AND v.element_id = e.id
                                                              AND v.etat = 'disponible' ORDER BY v.numero DESC LIMIT 1),
                                    etat = CASE WHEN e.etat IN ('analyse', 'quarantaine') THEN 'disponible' ELSE e.etat END,
                                    etat_avant_corbeille = CASE WHEN e.etat_avant_corbeille IN ('analyse', 'quarantaine')
                                                                THEN 'disponible' ELSE e.etat_avant_corbeille END,
                                    modifie_le = $3
                              WHERE e.etablissement_id = $1 AND e.id = $2",
                        )
                        .bind(etab)
                        .bind(element_id)
                        .bind(maintenant)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                        journaliser(&mut tx, etab, Some(espace_id), Some(element_id), None, "analyse.saine", None, maintenant).await?;
                        let (evt, agg) = if numero == 1 { ("espaces.element_depose", element_id) } else { ("espaces.version_creee", version_id) };
                        publier_evenement(&mut tx, etab, evt, agg, None, json!({ "espace_id": espace_id, "element_id": element_id, "numero": numero })).await?;
                        bilan.saines += 1;
                    }
                    tx.commit().await.map_err(db)?;
                }
                VerdictAnalyse::Infecte(signature) => {
                    let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                    let motif: String = format!("menace : {}", signature.chars().filter(|c| c.is_ascii_graphic() || *c == ' ').take(80).collect::<String>());
                    let r = sqlx::query(
                        "UPDATE ent_espace_version SET etat = 'quarantaine', derniere_analyse_le = $3, analyse_motif = $4
                          WHERE etablissement_id = $1 AND id = $2 AND etat = 'analyse'",
                    )
                    .bind(etab)
                    .bind(version_id)
                    .bind(maintenant)
                    .bind(&motif)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    if r.rows_affected() == 1 {
                        // Sans version disponible, l'élément lui-même passe en quarantaine.
                        sqlx::query(
                            "UPDATE ent_espace_element e SET etat = 'quarantaine', modifie_le = $3
                              WHERE e.etablissement_id = $1 AND e.id = $2 AND e.etat = 'analyse'
                                AND NOT EXISTS (SELECT 1 FROM ent_espace_version v
                                                 WHERE v.etablissement_id = e.etablissement_id AND v.element_id = e.id AND v.etat = 'disponible')",
                        )
                        .bind(etab)
                        .bind(element_id)
                        .bind(maintenant)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                        // Le quota est rendu, l'objet marqué supprimé (effacement physique après validation).
                        sqlx::query("UPDATE ent_objet SET supprime_le = $3 WHERE etablissement_id = $1 AND id = $2 AND supprime_le IS NULL")
                            .bind(etab)
                            .bind(objet_id)
                            .bind(maintenant)
                            .execute(&mut *tx)
                            .await
                            .map_err(db)?;
                        quota_rendre(&mut tx, etab, espace_id, taille, maintenant).await?;
                        journaliser(&mut tx, etab, Some(espace_id), Some(element_id), None, "analyse.infecte", Some("menace"), maintenant).await?;
                        let evt = publier_evenement(&mut tx, etab, "espaces.contenu_refuse", version_id, None, json!({ "espace_id": espace_id, "element_id": element_id })).await?;
                        // Auteur et animateurs prévenus (CET-14), sans nom de fichier.
                        let mut a_prevenir = vec![auteur];
                        let animateurs: Vec<Uuid> = sqlx::query_scalar(
                            "SELECT personne_id FROM ent_espace_membre_effectif
                              WHERE etablissement_id = $1 AND espace_id = $2 AND role = 'animateur'",
                        )
                        .bind(etab)
                        .bind(espace_id)
                        .fetch_all(&mut *tx)
                        .await
                        .map_err(db)?;
                        for a in animateurs {
                            if !a_prevenir.contains(&a) {
                                a_prevenir.push(a);
                            }
                        }
                        for p in a_prevenir {
                            avertir(
                                &mut tx,
                                etab,
                                p,
                                "espaces.analyse",
                                "Un fichier a été refusé par l'analyse de sécurité",
                                "Un fichier déposé dans un espace a été refusé : il n'est pas disponible. Contactez l'administrateur si besoin.",
                                version_id,
                                evt,
                                maintenant,
                            )
                            .await?;
                        }
                        bilan.infectees += 1;
                    }
                    tx.commit().await.map_err(db)?;
                    self.effacer_objet_sans_registre(&cle).await;
                }
            }
        }
        Ok(bilan)
    }

    async fn noter_tentative(
        &self,
        etab: Uuid,
        version: Uuid,
        motif: &str,
        tentatives: i32,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_espace_version SET analyse_tentatives = analyse_tentatives + 1, derniere_analyse_le = $3, analyse_motif = $4
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'analyse'",
        )
        .bind(etab)
        .bind(version)
        .bind(maintenant)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        if tentatives + 1 == ANALYSE_TENTATIVES_ALERTE {
            // Alerte d'exploitation : le fichier reste invisible tant que l'analyse n'aboutit pas.
            tracing::error!(version = %version, "espaces : analyse impossible après plusieurs tentatives (fichier toujours invisible)");
        }
        Ok(())
    }

    /// Libère les réservations de quota expirées de l'établissement (envoi coupé, CET-18) et efface les objets qui auraient
    /// été écrits après coup sous leur clé. Renvoie le nombre de réservations libérées.
    pub async fn espaces_liberer_reservations_expirees(&self, etab: Uuid, maintenant: DateTime<Utc>) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let liberees = reservations_expirees_liberer(&mut tx, etab, maintenant).await?;
        tx.commit().await.map_err(db)?;
        for r in &liberees {
            self.effacer_objet_sans_registre(&r.cle_objet).await;
        }
        Ok(liberees.len())
    }

    /// Efface définitivement les éléments en corbeille depuis plus de 30 jours (S-3205), sauf ceux dont une version est
    /// figée par un autre socle (R-ET6). Au plus `limite` éléments par passe.
    pub async fn espaces_purger_corbeille(&self, etab: Uuid, limite: i64, maintenant: DateTime<Utc>) -> Result<BilanPurge, EntDbError> {
        let echus: Vec<Uuid> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let l = sqlx::query_scalar(
                "SELECT id FROM ent_espace_element
                  WHERE etablissement_id = $1 AND etat = 'corbeille' AND corbeille_le <= $2
                  ORDER BY corbeille_le LIMIT $3",
            )
            .bind(etab)
            .bind(maintenant - Duration::days(CORBEILLE_JOURS))
            .bind(limite.clamp(1, 1000))
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            l
        };
        let mut bilan = BilanPurge::default();
        for id in echus {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let jour = jour_a(&mut tx, etab, maintenant).await?;
            let espace: Option<Uuid> = sqlx::query_scalar("SELECT espace_id FROM ent_espace_element WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
            match purger_element(&mut tx, etab, id, jour, maintenant).await? {
                Some(cles) => {
                    journaliser(&mut tx, etab, espace, Some(id), None, "element.efface", Some("echeance"), maintenant).await?;
                    publier_evenement(&mut tx, etab, "espaces.element_efface", id, None, json!({ "element_id": id })).await?;
                    tx.commit().await.map_err(db)?;
                    for c in cles {
                        self.effacer_objet_sans_registre(&c).await;
                    }
                    bilan.effaces += 1;
                }
                None => {
                    tx.commit().await.map_err(db)?;
                    bilan.gardes += 1;
                }
            }
        }
        Ok(bilan)
    }

    /// Élague les anciennes versions (S-3210) : toutes pendant 30 jours, puis une par jour jusqu'à 90 jours, puis une par
    /// semaine ; la courante, les versions figées par un autre socle, les versions nommées et celles non encore analysées
    /// ne sont jamais touchées. Le quota est rendu. Renvoie le nombre de versions élaguées.
    pub async fn espaces_elaguer_versions(&self, etab: Uuid, maintenant: DateTime<Utc>) -> Result<usize, EntDbError> {
        let candidats: Vec<Uuid> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let l = sqlx::query_scalar(
                "SELECT e.id FROM ent_espace_element e
                  WHERE e.etablissement_id = $1 AND e.etat IN ('disponible', 'masque')
                    AND (SELECT count(*) FROM ent_espace_version v
                          WHERE v.etablissement_id = e.etablissement_id AND v.element_id = e.id AND v.elaguee_le IS NULL) > 1
                    AND EXISTS (SELECT 1 FROM ent_espace_version v
                                 WHERE v.etablissement_id = e.etablissement_id AND v.element_id = e.id AND v.elaguee_le IS NULL
                                   AND v.cree_le < $2)
                  ORDER BY e.id LIMIT 500",
            )
            .bind(etab)
            .bind(maintenant - Duration::days(ELAGAGE_TOUT_JOURS))
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            l
        };
        let mut total = 0usize;
        for element in candidats {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let jour = jour_a(&mut tx, etab, maintenant).await?;
            let l = sqlx::query(
                "SELECT v.id, v.numero, v.cree_le, (v.id = e.version_courante_id),
                        (v.nom_version IS NOT NULL OR v.etat <> 'disponible'
                         OR EXISTS (SELECT 1 FROM ent_espace_version_figee f
                                     WHERE f.etablissement_id = v.etablissement_id AND f.version_id = v.id
                                       AND (f.jusqu_au IS NULL OR f.jusqu_au >= $3))),
                        v.taille, o.id, o.cle_objet, e.espace_id
                   FROM ent_espace_version v
                   JOIN ent_espace_element e ON e.etablissement_id = v.etablissement_id AND e.id = v.element_id
                   JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
                  WHERE v.etablissement_id = $1 AND v.element_id = $2 AND v.elaguee_le IS NULL
                  FOR UPDATE OF e",
            )
            .bind(etab)
            .bind(element)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let versions: Vec<VersionElagable> = l
                .iter()
                .map(|r| VersionElagable { id: r.get(0), numero: r.get(1), cree_le: r.get(2), courante: r.get(3), protegee: r.get(4) })
                .collect();
            let plan = versions_a_elaguer(&versions, maintenant);
            let mut cles = Vec::new();
            let mut espace = None;
            for id in &plan {
                if let Some(r) = l.iter().find(|r| r.get::<Uuid, _>(0) == *id) {
                    sqlx::query("UPDATE ent_espace_version SET elaguee_le = $3 WHERE etablissement_id = $1 AND id = $2 AND elaguee_le IS NULL")
                        .bind(etab)
                        .bind(*id)
                        .bind(maintenant)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    sqlx::query("UPDATE ent_objet SET supprime_le = $3 WHERE etablissement_id = $1 AND id = $2 AND supprime_le IS NULL")
                        .bind(etab)
                        .bind(r.get::<Uuid, _>(6))
                        .bind(maintenant)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    quota_rendre(&mut tx, etab, r.get::<Uuid, _>(8), r.get::<i64, _>(5), maintenant).await?;
                    cles.push(r.get::<String, _>(7));
                    espace = Some(r.get::<Uuid, _>(8));
                }
            }
            if !plan.is_empty() {
                journaliser(&mut tx, etab, espace, Some(element), None, "version.elaguee", Some("s3210"), maintenant).await?;
            }
            tx.commit().await.map_err(db)?;
            total += plan.len();
            for c in cles {
                self.effacer_objet_sans_registre(&c).await;
            }
        }
        Ok(total)
    }

    /// Recalcule les compteurs de quota de tous les espaces de l'établissement depuis les lignes de vérité (versions non
    /// effacées, réservations ouvertes) : maintenance et après restauration. Renvoie le nombre d'espaces traités.
    pub async fn espaces_reconcilier_occupation(&self, etab: Uuid, maintenant: DateTime<Utc>) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let espaces: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM ent_espace WHERE etablissement_id = $1 AND etat <> 'efface' ORDER BY id")
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        for e in &espaces {
            occupation_reconcilier(&mut tx, etab, *e, maintenant).await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(espaces.len())
    }

    /// Passe d'ordonnanceur pour tous les établissements : réservations expirées, purge de la corbeille, élagage, analyse
    /// (si un antivirus est configuré). Les échecs d'un établissement n'arrêtent pas les autres.
    pub async fn espaces_tache_tous_etablissements(
        &self,
        antivirus: Option<Arc<dyn AnalyseurAntivirus>>,
        maintenant: DateTime<Utc>,
    ) -> Result<(usize, BilanPurge, usize, BilanAnalyse), EntDbError> {
        let etabs = etablissements_systeme(self.pool(), ContexteSysteme::JobPlanifie("espaces_taches")).await.map_err(db)?;
        let (mut liberees, mut purge, mut elaguees, mut analyse) = (0usize, BilanPurge::default(), 0usize, BilanAnalyse::default());
        for etab in etabs {
            match self.espaces_liberer_reservations_expirees(etab, maintenant).await {
                Ok(n) => liberees += n,
                Err(e) => tracing::warn!(%etab, erreur = %e, "espaces : libération des réservations"),
            }
            match self.espaces_purger_corbeille(etab, 200, maintenant).await {
                Ok(b) => {
                    purge.effaces += b.effaces;
                    purge.gardes += b.gardes;
                }
                Err(e) => tracing::warn!(%etab, erreur = %e, "espaces : purge de la corbeille"),
            }
            match self.espaces_elaguer_versions(etab, maintenant).await {
                Ok(n) => elaguees += n,
                Err(e) => tracing::warn!(%etab, erreur = %e, "espaces : élagage des versions"),
            }
            if let Some(av) = antivirus.clone() {
                match self.espaces_analyser_versions(etab, av, 20, maintenant).await {
                    Ok(b) => {
                        analyse.saines += b.saines;
                        analyse.infectees += b.infectees;
                        analyse.indisponibles += b.indisponibles;
                    }
                    Err(e) => tracing::warn!(%etab, erreur = %e, "espaces : analyse"),
                }
            }
        }
        Ok((liberees, purge, elaguees, analyse))
    }
}
