//! Socle 21, lot V3 « grille et saisie » : la grille du professeur sur les tables `eva_*`.
//!
//! * **Un cours, une grille** : la grille se fait par **cours** (socle 5), jamais par « service avec
//!   matière du référentiel » : un cours sans matière (libellé libre) a sa grille (c'était la cause du blocage
//!   « service sans matière » de l'ancienne grille).
//! * **Le moteur unique** (`domain_eleve::evaluation_moteur`) calcule toutes les moyennes (élève, classe,
//!   évaluation) ; aucun calcul n'est fait côté client.
//! * **Droits** : saisie = titulaire, co-enseignant ou suppléant en saisie à la date (`ent_prof_sur_cours`) ;
//!   lecture = idem + `notes.lire` à portée établissement (direction, secrétariat) ; famille et élève : jamais.
//!   Après l'arrêt d'une période, seul `notes.corriger` (direction) écrit, avec un motif (R-V10).
//! * **Conflits** : chaque case porte sa version ; une écriture avec une version périmée est refusée et
//!   rend l'état actuel, jamais d'écrasement silencieux (CE-54, R-V27).
//! * **Événements** neutres : identifiants seulement (R-V17).
//!
//! Isolation : toutes les requêtes portent `etablissement_id` et passent par `TenantTx` (RLS).

use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::evaluation_moteur::{
    moyenne_matiere, statistiques, statistiques_evaluation, CoefSousMatiere, Comptage,
    EntreeCalcul, EvalCalcul, Rat, ReglesMoteur, StatutCalcul,
};
use ent_auth::Action;
use ent_types::evaluations_v3::{
    analyser_cellule, Analyse, CelluleDto, CelluleRetourDto, CelluleSaisieDto, ConflitDto,
    ContrainteCellule, EleveLigneDto, EnseignementV3Dto, EvaluationColonneDto, ExplicationDto,
    GrilleV3Dto, LigneHistoriqueV3Dto, PeriodeV3Dto, SaisieLotBody, SaisieLotRetourDto,
    SousMatiereV3Dto, StatsV3Dto,
};
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{db, introuvable, refus};
use crate::evaluations_v1::{regles_tx, Attendu, EcrireResultat, SourceResultat};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Plafond de cases par envoi (collage de 40 élèves × 20 évaluations = 800).
pub const MAX_CELLULES_PAR_LOT: usize = 2000;
/// Longueur maximale du texte tapé dans une case (F8) : « 14,5 », « abs », « disp »… jamais un paragraphe.
const MAX_TEXTE_CASE: usize = 40;
/// Longueur maximale d'un motif écrit (F8).
const MAX_MOTIF: usize = 500;

/// Types d'événements de V3. Charges : identifiants seulement.
pub mod types_v3 {
    pub const EVALUATION_CREEE: &str = "evaluations.creee";
    pub const EVALUATION_MODIFIEE: &str = "evaluations.modifiee";
    pub const EVALUATION_ANNULEE: &str = "evaluations.annulee";
    pub const EVALUATION_PUBLIEE: &str = "evaluations.publiee";
    pub const PUBLICATION_PROGRAMMEE: &str = "evaluations.publication_programmee";
    pub const PUBLICATION_RETIREE: &str = "evaluations.publication_retiree";
    pub const RAPPEL_PUBLICATION: &str = "evaluations.rappel_publication";
    pub const IMPORT_EFFECTUE: &str = "evaluations.import_effectue";
}

/// Ce que l'utilisateur peut faire sur un cours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccesCours {
    /// Titulaire, co-enseignant ou suppléant **en saisie** à la date.
    pub saisie: bool,
    /// Lecture (service, ou `notes.lire` établissement).
    pub lecture: bool,
    /// `notes.corriger` : corrections motivées après l'arrêt.
    pub corriger: bool,
    /// Suppléant ce jour-là (ne modifie pas les évaluations d'un autre auteur sans droit de correction, CE-44).
    pub suppleant: bool,
    /// Professeur DU COURS (service, y compris en lecture après la fin du service) : distinct de `notes.lire`
    /// d'établissement (secrétariat) qui lit la grille mais pas l'historique d'une case (EVA-139, F6).
    pub prof_du_cours: bool,
}

/// Un élève de la grille.
#[derive(Debug, Clone)]
pub(crate) struct EleveGrille {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
}

/// État d'une case (élève × évaluation) côté période et public.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EtatCase {
    pub concerne: bool,
    pub saisie_ouverte: bool,
    pub rattrapage_autorise: bool,
    pub periode_id: Option<Uuid>,
}

/// Un résultat lu.
#[derive(Debug, Clone)]
pub(crate) struct ResultatLu {
    pub id: Uuid,
    pub statut: String,
    pub valeur_m: Option<i64>,
    pub version: i32,
    pub commentaire: bool,
}

/// Une évaluation lue pour la grille.
#[derive(Debug, Clone)]
pub(crate) struct EvalLue {
    pub id: Uuid,
    pub titre: String,
    pub date_eval: NaiveDate,
    pub bareme_m: i64,
    pub coef_m: i64,
    pub comptage: String,
    pub hors_bareme: bool,
    pub etat: String,
    pub motif_annulation: Option<String>,
    pub version: i32,
    pub publication: String,
    pub prevue_le: Option<DateTime<Utc>>,
    pub publiee_le: Option<DateTime<Utc>>,
    pub retiree_le: Option<DateTime<Utc>>,
    pub en_cours: bool,
    pub sous_matiere_id: Option<Uuid>,
    pub restreinte: bool,
    pub commune: bool,
    pub annoncee: bool,
    pub a_des_resultats: bool,
    pub cree_par: Option<Uuid>,
}

impl EvalLue {
    pub(crate) fn contrainte(&self, pas_m: i64) -> ContrainteCellule {
        ContrainteCellule {
            bareme_m: self.bareme_m,
            hors_bareme_autorise: self.hors_bareme,
            pas_m,
        }
    }

    fn calcul(&self) -> EvalCalcul {
        EvalCalcul {
            id: self.id,
            titre: self.titre.clone(),
            bareme_m: self.bareme_m,
            coef_m: self.coef_m,
            comptage: Comptage::parse(&self.comptage).unwrap_or_default(),
            hors_bareme_autorise: self.hors_bareme,
            sous_matiere_id: self.sous_matiere_id,
            cours_id: None,
            annulee: self.etat == "annulee",
        }
    }
}

/// Tout ce qu'il faut pour afficher ou rejouer une grille, lu dans une seule transaction.
pub(crate) struct GrilleInterne {
    pub cours: EnseignementV3Dto,
    pub annee_id: Uuid,
    pub periode: PeriodeV3Dto,
    pub evals: Vec<EvalLue>,
    pub eleves: Vec<EleveGrille>,
    pub cases: HashMap<(Uuid, Uuid), EtatCase>,
    pub resultats: HashMap<(Uuid, Uuid), ResultatLu>,
    /// Élèves qui sont encore dans le public du cours à la fin de la période (ou aujourd'hui).
    pub dans_public_fin: HashSet<Uuid>,
    pub reglage: crate::evaluations_v1::ReglesEtablissement,
}

pub(crate) fn libelle_stats(
    s: Option<domain_eleve::evaluation_moteur::Statistiques>,
    regles: &ReglesMoteur,
) -> Option<StatsV3Dto> {
    let a = s?.affichees(regles).ok()?;
    Some(StatsV3Dto {
        effectif: a.effectif,
        moyenne_m: a.moyenne_m,
        min_m: a.min_m,
        max_m: a.max_m,
        mediane_m: a.mediane_m,
    })
}

fn tri_nom(e: &EleveGrille) -> (String, String, Uuid) {
    (e.nom.to_lowercase(), e.prenom.to_lowercase(), e.id)
}

impl PgEnt {
    // ───────────────────────── Droits ─────────────────────────

    /// Droits de l'acteur sur un cours à un jour. Un cours d'un autre établissement ou inconnu est
    /// « introuvable » ; sans aucun droit de lecture, « interdit ».
    pub(crate) async fn eva_v3_acces(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        cours_id: Uuid,
        jour: NaiveDate,
    ) -> Result<AccesCours, EntDbError> {
        Self::exiger_staff(role)?;
        let etab_lecture = self
            .a_action_etablissement(etab, acteur, role, Action::NotesLire)
            .await?;
        let corriger = self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = Self::eva_v3_acces_tx(
            &mut tx,
            etab,
            acteur,
            cours_id,
            jour,
            etab_lecture,
            corriger,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        if !a.lecture {
            return Err(EntDbError::Forbidden);
        }
        Ok(a)
    }

    pub(crate) async fn eva_v3_acces_tx(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        acteur: Uuid,
        cours_id: Uuid,
        jour: NaiveDate,
        etab_lecture: bool,
        corriger: bool,
    ) -> Result<AccesCours, EntDbError> {
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_cours WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(cours_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let r = sqlx::query(
            "SELECT ent_prof_sur_cours($1, $2, $3, $4, true)
                      OR EXISTS (SELECT 1 FROM ent_service_enseignement s
                                  WHERE s.etablissement_id = $1 AND s.cours_id = $2 AND s.prof_user_id = $3
                                    AND NOT EXISTS (SELECT 1 FROM ent_service_intervenant z
                                                     WHERE z.etablissement_id = $1 AND z.service_id = s.id
                                                       AND z.nature = 'suppleant' AND z.annule_le IS NULL AND z.saisie
                                                       AND z.du <= $4 AND (z.au IS NULL OR z.au >= $4))) AS saisie,
                    ent_prof_sur_cours($1, $2, $3, $4, false) AS lecture,
                    EXISTS (SELECT 1 FROM ent_service_enseignement s
                              JOIN ent_service_intervenant i ON i.etablissement_id = s.etablissement_id AND i.service_id = s.id
                              JOIN ent_agent ag ON ag.etablissement_id = i.etablissement_id AND ag.id = i.agent_id
                              JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                             WHERE s.etablissement_id = $1 AND s.cours_id = $2 AND pe.user_id = $3
                               AND i.nature = 'suppleant' AND i.annule_le IS NULL
                               AND i.du <= $4 AND (i.au IS NULL OR i.au >= $4)) AS suppleant",
        )
        .bind(etab)
        .bind(cours_id)
        .bind(acteur)
        .bind(jour)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        let saisie: bool = r.try_get("saisie").map_err(db)?;
        let lecture_cours: bool = r.try_get("lecture").map_err(db)?;
        let suppleant: bool = r.try_get("suppleant").map_err(db)?;
        Ok(AccesCours {
            saisie,
            lecture: saisie || lecture_cours || etab_lecture,
            corriger,
            suppleant,
            prof_du_cours: saisie || lecture_cours,
        })
    }

    // ───────────────────────── Enseignements et périodes ─────────────────────────

    /// Les cours auxquels l'acteur a accès : les siens (saisie, ou lecture si le service est clos
    /// depuis peu), et **tous** ceux de l'établissement pour un détenteur de `notes.lire` (direction,
    /// secrétariat), en lecture. Cours sans matière du référentiel inclus.
    pub async fn eva_v3_enseignements_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<Vec<EnseignementV3Dto>, EntDbError> {
        Self::exiger_staff(role)?;
        let etab_lecture = self
            .a_action_etablissement(etab, acteur, role, Action::NotesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = sqlx::query(
            "SELECT c.id, c.matiere_id,
                    COALESCE(NULLIF(btrim(m.libelle), ''), NULLIF(btrim(c.libelle), ''), 'Enseignement') AS libelle,
                    COALESCE(cl.code, g.code, '') AS cible_code,
                    ent_prof_sur_cours($1, c.id, $2, $3, true) AS saisie,
                    ent_prof_sur_cours($1, c.id, $2, $3, false) AS lecture
               FROM ent_cours c
               JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
               LEFT JOIN ent_matiere m ON m.id = c.matiere_id AND m.etablissement_id = c.etablissement_id
               LEFT JOIN ent_classe cl ON cl.id = c.classe_id AND cl.etablissement_id = c.etablissement_id
               LEFT JOIN ent_groupe g ON g.id = c.groupe_id AND g.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND y.date_debut <= $3 AND y.date_fin >= $3
                AND NOT (c.source = 'reprise' AND c.a_verifier)
              ORDER BY cible_code, libelle, c.id
              LIMIT 1000",
        )
        .bind(etab)
        .bind(acteur)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut out = Vec::new();
        for l in &lignes {
            let saisie: bool = l.try_get("saisie").map_err(db)?;
            let lecture: bool = l.try_get("lecture").map_err(db)?;
            if !(saisie || lecture || etab_lecture) {
                continue;
            }
            let matiere_id: Option<Uuid> = l.try_get("matiere_id").map_err(db)?;
            out.push(EnseignementV3Dto {
                cours_id: l.try_get("id").map_err(db)?,
                libelle: l.try_get("libelle").map_err(db)?,
                cible_code: l.try_get("cible_code").map_err(db)?,
                matiere_id,
                sans_matiere: matiere_id.is_none(),
                droit: if saisie { "saisie" } else { "lecture" }.to_string(),
            });
        }
        Ok(out)
    }

    /// Les périodes que peut afficher la grille d'un cours : celles de la périodisation d'évaluation de
    /// la classe du cours (ou des classes de ses élèves, pour un groupe).
    pub async fn eva_v3_periodes_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        cours_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<PeriodeV3Dto>, EntDbError> {
        self.eva_v3_acces(etab, acteur, role, cours_id, jour)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = sqlx::query(
            "SELECT DISTINCT p.id, p.code, p.libelle, p.date_debut, p.date_fin
               FROM ent_periode p
              WHERE p.etablissement_id = $1
                AND p.periodisation_id IN (
                    SELECT ent_periodisation_pour_classe($1, k.classe_id)
                      FROM (SELECT c.classe_id FROM ent_cours c
                             WHERE c.etablissement_id = $1 AND c.id = $2 AND c.classe_id IS NOT NULL
                            UNION
                            SELECT ac.classe_id
                              FROM ent_cours c
                              JOIN ent_affectation ag ON ag.etablissement_id = c.etablissement_id
                                                     AND ag.groupe_id = c.groupe_id AND ag.annee_id = c.annee_id
                              JOIN ent_affectation ac ON ac.etablissement_id = ag.etablissement_id
                                                     AND ac.eleve_id = ag.eleve_id AND ac.annee_id = ag.annee_id
                                                     AND ac.classe_id IS NOT NULL
                             WHERE c.etablissement_id = $1 AND c.id = $2 AND c.groupe_id IS NOT NULL) k)
              ORDER BY p.date_debut, p.code",
        )
        .bind(etab)
        .bind(cours_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        lignes
            .iter()
            .map(|l| {
                Ok(PeriodeV3Dto {
                    id: l.try_get("id")?,
                    code: l.try_get("code")?,
                    libelle: l.try_get("libelle")?,
                    date_debut: l.try_get("date_debut")?,
                    date_fin: l.try_get("date_fin")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)
    }

    /// Les sous-matières de la matière du cours (vide pour un cours sans matière du référentiel).
    pub async fn eva_v3_sous_matieres_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        cours_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<SousMatiereV3Dto>, EntDbError> {
        self.eva_v3_acces(etab, acteur, role, cours_id, jour)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT m.id, m.libelle FROM ent_matiere m
               JOIN ent_cours c ON c.etablissement_id = m.etablissement_id AND c.matiere_id = m.parent_id
              WHERE c.etablissement_id = $1 AND c.id = $2 ORDER BY m.libelle, m.id",
        )
        .bind(etab)
        .bind(cours_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, libelle)| SousMatiereV3Dto { id, libelle })
            .collect())
    }

    // ───────────────────────── Chargement de la grille ─────────────────────────

    /// Charge la grille d'un cours pour une période (lecture seule, une transaction). Les élèves protégés
    /// que l'acteur ne doit pas voir en sont retirés (garde du socle 2).
    pub(crate) async fn eva_v3_charger(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        cours_id: Uuid,
        periode_id: Uuid,
        jour: NaiveDate,
        avec_brouillons: bool,
    ) -> Result<GrilleInterne, EntDbError> {
        let profil = self.profil_champs(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut g =
            Self::eva_v3_charger_tx(&mut tx, etab, cours_id, periode_id, jour, avec_brouillons)
                .await?;
        tx.commit().await.map_err(db)?;
        let ids: Vec<Uuid> = g.eleves.iter().map(|e| e.id).collect();
        let caches = self.proteges_caches(etab, role, &profil, &ids).await?;
        g.eleves.retain(|e| !caches.contains(&e.id));
        Ok(g)
    }

    pub(crate) async fn eva_v3_charger_tx(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        cours_id: Uuid,
        periode_id: Uuid,
        jour: NaiveDate,
        avec_brouillons: bool,
    ) -> Result<GrilleInterne, EntDbError> {
        let c = sqlx::query(
            "SELECT c.annee_id, c.matiere_id,
                    COALESCE(NULLIF(btrim(m.libelle), ''), NULLIF(btrim(c.libelle), ''), 'Enseignement') AS libelle,
                    COALESCE(cl.code, g.code, '') AS cible_code
               FROM ent_cours c
               LEFT JOIN ent_matiere m ON m.id = c.matiere_id AND m.etablissement_id = c.etablissement_id
               LEFT JOIN ent_classe cl ON cl.id = c.classe_id AND cl.etablissement_id = c.etablissement_id
               LEFT JOIN ent_groupe g ON g.id = c.groupe_id AND g.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = $2",
        )
        .bind(etab)
        .bind(cours_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let annee_id: Uuid = c.try_get("annee_id").map_err(db)?;
        let matiere_id: Option<Uuid> = c.try_get("matiere_id").map_err(db)?;
        let cours = EnseignementV3Dto {
            cours_id,
            libelle: c.try_get("libelle").map_err(db)?,
            cible_code: c.try_get("cible_code").map_err(db)?,
            matiere_id,
            sans_matiere: matiere_id.is_none(),
            droit: "lecture".to_string(),
        };
        let p = sqlx::query(
            "SELECT id, code, libelle, date_debut, date_fin FROM ent_periode
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(periode_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let periode = PeriodeV3Dto {
            id: p.try_get("id").map_err(db)?,
            code: p.try_get("code").map_err(db)?,
            libelle: p.try_get("libelle").map_err(db)?,
            date_debut: p.try_get("date_debut").map_err(db)?,
            date_fin: p.try_get("date_fin").map_err(db)?,
        };
        let reglage = regles_tx(tx, etab, annee_id).await?;

        let lignes = sqlx::query(
            "SELECT e.id, e.titre, e.date_eval, (e.bareme * 1000)::bigint AS bareme_m, (e.coef * 1000)::bigint AS coef_m,
                    e.comptage, e.hors_bareme_autorise, e.etat, e.motif_annulation, e.version, e.publication,
                    e.publication_prevue_le, e.publiee_le, e.retiree_le, e.en_cours_de_correction, e.matiere_id,
                    EXISTS (SELECT 1 FROM eva_evaluation_restriction r
                             WHERE r.etablissement_id = e.etablissement_id AND r.evaluation_id = e.id) AS restreinte,
                    (e.commune_id IS NOT NULL) AS commune, e.annoncee, e.cree_par,
                    EXISTS (SELECT 1 FROM eva_resultat r
                             WHERE r.etablissement_id = e.etablissement_id AND r.evaluation_id = e.id) AS a_des_resultats
               FROM eva_evaluation e
              WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.mode = 'note'
                AND e.date_eval BETWEEN $3 AND $4
                AND (e.etat IN ('ouverte', 'annulee') OR ($5 AND e.etat = 'brouillon'))
              ORDER BY e.date_eval, e.cree_le, e.id",
        )
        .bind(etab)
        .bind(cours_id)
        .bind(periode.date_debut)
        .bind(periode.date_fin)
        .bind(avec_brouillons)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
        let mut evals = Vec::with_capacity(lignes.len());
        for l in &lignes {
            evals.push(EvalLue {
                id: l.try_get("id").map_err(db)?,
                titre: l.try_get("titre").map_err(db)?,
                date_eval: l.try_get("date_eval").map_err(db)?,
                bareme_m: l.try_get("bareme_m").map_err(db)?,
                coef_m: l.try_get("coef_m").map_err(db)?,
                comptage: l.try_get("comptage").map_err(db)?,
                hors_bareme: l.try_get("hors_bareme_autorise").map_err(db)?,
                etat: l.try_get("etat").map_err(db)?,
                motif_annulation: l.try_get("motif_annulation").map_err(db)?,
                version: l.try_get("version").map_err(db)?,
                publication: l.try_get("publication").map_err(db)?,
                prevue_le: l.try_get("publication_prevue_le").map_err(db)?,
                publiee_le: l.try_get("publiee_le").map_err(db)?,
                retiree_le: l.try_get("retiree_le").map_err(db)?,
                en_cours: l.try_get("en_cours_de_correction").map_err(db)?,
                sous_matiere_id: l.try_get("matiere_id").map_err(db)?,
                restreinte: l.try_get("restreinte").map_err(db)?,
                commune: l.try_get("commune").map_err(db)?,
                annoncee: l.try_get("annoncee").map_err(db)?,
                a_des_resultats: l.try_get("a_des_resultats").map_err(db)?,
                cree_par: l.try_get("cree_par").map_err(db)?,
            });
        }
        let ids_eval: Vec<Uuid> = evals.iter().map(|e| e.id).collect();

        // Public de chaque évaluation à SA date (R-V2), période et état de chaque élève (R-V29).
        let mut cases: HashMap<(Uuid, Uuid), EtatCase> = HashMap::new();
        let mut eleves_ids: BTreeSet<Uuid> = BTreeSet::new();
        if !ids_eval.is_empty() {
            let rows = sqlx::query(
                "SELECT e.id AS evaluation_id, g.eleve_id, g.periode_id, g.saisie_ouverte, g.rattrapage_autorise
                   FROM eva_evaluation e
                  CROSS JOIN LATERAL eva_grille_eleves(e.etablissement_id, e.id) g
                  WHERE e.etablissement_id = $1 AND e.id = ANY($2)",
            )
            .bind(etab)
            .bind(&ids_eval)
            .fetch_all(&mut **tx)
            .await
            .map_err(db)?;
            for r in &rows {
                let ev: Uuid = r.try_get("evaluation_id").map_err(db)?;
                let el: Uuid = r.try_get("eleve_id").map_err(db)?;
                eleves_ids.insert(el);
                cases.insert(
                    (ev, el),
                    EtatCase {
                        concerne: true,
                        saisie_ouverte: r
                            .try_get::<Option<bool>, _>("saisie_ouverte")
                            .map_err(db)?
                            .unwrap_or(false),
                        rattrapage_autorise: r.try_get("rattrapage_autorise").map_err(db)?,
                        periode_id: r.try_get("periode_id").map_err(db)?,
                    },
                );
            }
        }
        // Public à la fin de la période (ou aujourd'hui, si elle est en cours) : élèves sans évaluation incluse.
        let jour_ref = jour.clamp(periode.date_debut, periode.date_fin);
        let public: Vec<Uuid> =
            sqlx::query_scalar("SELECT eleve_id FROM ent_public_cours_jour($1, $2, $3)")
                .bind(etab)
                .bind(jour_ref)
                .bind(cours_id)
                .fetch_all(&mut **tx)
                .await
                .map_err(db)?;
        let dans_public_fin: HashSet<Uuid> = public.iter().copied().collect();
        eleves_ids.extend(public.iter().copied());

        let ids_vec: Vec<Uuid> = eleves_ids.iter().copied().collect();
        let mut eleves: Vec<EleveGrille> = Vec::new();
        if !ids_vec.is_empty() {
            let rows = sqlx::query(
                "SELECT id, nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
            )
            .bind(etab)
            .bind(&ids_vec)
            .fetch_all(&mut **tx)
            .await
            .map_err(db)?;
            for r in &rows {
                eleves.push(EleveGrille {
                    id: r.try_get("id").map_err(db)?,
                    nom: r.try_get("nom").map_err(db)?,
                    prenom: r.try_get("prenom").map_err(db)?,
                });
            }
            eleves.sort_by_key(tri_nom);
        }

        let mut resultats: HashMap<(Uuid, Uuid), ResultatLu> = HashMap::new();
        if !ids_eval.is_empty() {
            let rows = sqlx::query(
                "SELECT r.id, r.evaluation_id, r.eleve_id, r.statut, (r.valeur * 1000)::bigint AS valeur_m, r.version,
                        (r.commentaire_eleve IS NOT NULL) AS commentaire
                   FROM eva_resultat r
                  WHERE r.etablissement_id = $1 AND r.evaluation_id = ANY($2)",
            )
            .bind(etab)
            .bind(&ids_eval)
            .fetch_all(&mut **tx)
            .await
            .map_err(db)?;
            for r in &rows {
                resultats.insert(
                    (
                        r.try_get("evaluation_id").map_err(db)?,
                        r.try_get("eleve_id").map_err(db)?,
                    ),
                    ResultatLu {
                        id: r.try_get("id").map_err(db)?,
                        statut: r.try_get("statut").map_err(db)?,
                        valeur_m: r.try_get("valeur_m").map_err(db)?,
                        version: r.try_get("version").map_err(db)?,
                        commentaire: r.try_get("commentaire").map_err(db)?,
                    },
                );
            }
        }
        Ok(GrilleInterne {
            cours,
            annee_id,
            periode,
            evals,
            eleves,
            cases,
            resultats,
            dans_public_fin,
            reglage,
        })
    }

    /// Coefficients de sous-matière de l'année, à la fin de la période (comme `moyennes_tx`).
    async fn eva_v3_coefs_sous_matieres(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        g: &GrilleInterne,
    ) -> Result<Vec<CoefSousMatiere>, EntDbError> {
        let rows = sqlx::query(
            "SELECT matiere_id, (coef * 1000)::bigint AS coef_m FROM eva_coefficient_sous_matiere
              WHERE etablissement_id = $1 AND annee_id = $2
                AND valable_du <= $3 AND (valable_au IS NULL OR valable_au >= $3)",
        )
        .bind(etab)
        .bind(g.annee_id)
        .bind(g.periode.date_fin)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
        rows.iter()
            .map(|r| {
                Ok(CoefSousMatiere {
                    sous_matiere_id: Some(r.try_get("matiere_id")?),
                    coef_m: r.try_get("coef_m")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)
    }

    /// Les entrées du moteur pour un élève : seulement les cases où il est concerné, dont **sa** période est
    /// celle affichée (R-V29), qui portent un résultat, d'une évaluation ouverte ou annulée.
    fn eva_v3_entrees(g: &GrilleInterne, eleve: Uuid) -> Vec<EntreeCalcul> {
        let mut out = Vec::new();
        for ev in &g.evals {
            if ev.etat == "brouillon" {
                continue;
            }
            let Some(case) = g.cases.get(&(ev.id, eleve)) else {
                continue;
            };
            if !case.concerne || case.periode_id != Some(g.periode.id) {
                continue;
            }
            let Some(r) = g.resultats.get(&(ev.id, eleve)) else {
                continue;
            };
            let Some(statut) = StatutCalcul::parse(&r.statut) else {
                continue;
            };
            out.push(EntreeCalcul {
                evaluation: ev.calcul(),
                statut,
                valeur_m: r.valeur_m,
            });
        }
        out
    }

    // ───────────────────────── La grille ─────────────────────────

    /// La grille d'un cours pour une période : colonnes (évaluations), lignes (élèves), cases, moyennes de
    /// l'élève et de la classe **calculées par le moteur**.
    pub async fn eva_v3_grille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        cours_id: Uuid,
        periode_id: Uuid,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<GrilleV3Dto, EntDbError> {
        let acces = self
            .eva_v3_acces(etab, acteur, role, cours_id, jour)
            .await?;
        let g = self
            .eva_v3_charger(etab, acteur, role, cours_id, periode_id, jour, acces.saisie)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let coefs = Self::eva_v3_coefs_sous_matieres(&mut tx, etab, &g).await?;
        let delai: Option<i16> = sqlx::query_scalar(
            "SELECT delai_publication_max_jours FROM eva_reglage WHERE etablissement_id = $1 AND annee_id = $2",
        )
        .bind(etab)
        .bind(g.annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .flatten();
        tx.commit().await.map_err(db)?;
        let regles = &g.reglage.regles;
        let mut avertissements: Vec<String> = Vec::new();

        // Colonnes
        let mut colonnes: Vec<EvaluationColonneDto> = Vec::with_capacity(g.evals.len());
        for ev in &g.evals {
            let notes: Vec<(StatutCalcul, Option<i64>)> = g
                .eleves
                .iter()
                .filter(|e| g.cases.get(&(ev.id, e.id)).is_some_and(|c| c.concerne))
                .filter_map(|e| g.resultats.get(&(ev.id, e.id)))
                .filter_map(|r| StatutCalcul::parse(&r.statut).map(|s| (s, r.valeur_m)))
                .collect();
            let stats = match statistiques_evaluation(&ev.calcul(), &notes, regles) {
                Ok(s) => libelle_stats(s, regles),
                Err(_) => {
                    avertissements.push(format!(
                        "Statistiques de « {} » indisponibles : calcul impossible.",
                        ev.titre
                    ));
                    None
                }
            };
            let publiee_il_y_a_moins_de_24h = ev.publiee_le.is_some_and(|p| {
                maintenant.signed_duration_since(p) <= chrono::Duration::hours(24)
            });
            let auteur = ev.cree_par == Some(acteur);
            let peut_modifier = acces.saisie
                && ev.etat == "ouverte"
                && (auteur || !acces.suppleant || acces.corriger);
            colonnes.push(EvaluationColonneDto {
                id: ev.id,
                titre: ev.titre.clone(),
                date_eval: ev.date_eval,
                bareme_m: ev.bareme_m,
                coef_m: ev.coef_m,
                comptage: ev.comptage.clone(),
                hors_bareme_autorise: ev.hors_bareme,
                etat: ev.etat.clone(),
                motif_annulation: ev.motif_annulation.clone(),
                version: ev.version,
                publication: ev.publication.clone(),
                publication_prevue_le: ev.prevue_le,
                publiee_le: ev.publiee_le,
                retiree_le: ev.retiree_le,
                en_cours_de_correction: ev.en_cours,
                sous_matiere_id: ev.sous_matiere_id,
                restreinte: ev.restreinte,
                commune: ev.commune,
                annoncee: ev.annoncee,
                a_des_resultats: ev.a_des_resultats,
                peut_modifier,
                retrait_possible: ev.publiee_le.is_some()
                    && ((acces.saisie && publiee_il_y_a_moins_de_24h && peut_modifier)
                        || acces.corriger),
                stats,
            });
        }

        // Lignes
        let mut lignes: Vec<EleveLigneDto> = Vec::with_capacity(g.eleves.len());
        let mut moyennes: Vec<Rat> = Vec::new();
        let mut moyenne_indisponible = false;
        for e in &g.eleves {
            let cellules: Vec<CelluleDto> = g
                .evals
                .iter()
                .map(|ev| {
                    let case = g.cases.get(&(ev.id, e.id)).copied().unwrap_or_default();
                    let r = g.resultats.get(&(ev.id, e.id));
                    CelluleDto {
                        concerne: case.concerne,
                        statut: r.map(|r| r.statut.clone()),
                        valeur_m: r.and_then(|r| r.valeur_m),
                        version: r.map(|r| r.version),
                        resultat_id: r.map(|r| r.id),
                        saisie_ouverte: case.saisie_ouverte,
                        rattrapage_autorise: case.rattrapage_autorise,
                        hors_periode: case.concerne && case.periode_id != Some(g.periode.id),
                        corrige: r.is_some_and(|r| r.version > 1),
                        commentaire: r.is_some_and(|r| r.commentaire),
                    }
                })
                .collect();
            // Mouvement : arrivé ou parti en cours de période.
            let concernes: Vec<bool> = cellules.iter().map(|c| c.concerne).collect();
            let premier = concernes.iter().position(|c| *c);
            let dernier = concernes.iter().rposition(|c| *c);
            let mouvement = match (premier, dernier) {
                (Some(p), _) if p > 0 => Some("Arrivé(e) en cours de période".to_string()),
                (_, Some(d))
                    if d + 1 < concernes.len()
                        || (!g.dans_public_fin.contains(&e.id) && !concernes.is_empty()) =>
                {
                    Some("Parti(e) en cours de période".to_string())
                }
                (None, None) if !g.dans_public_fin.contains(&e.id) && !concernes.is_empty() => {
                    Some("Hors du public du cours".to_string())
                }
                _ => None,
            };
            let entrees = Self::eva_v3_entrees(&g, e.id);
            let (moyenne_m, nb) = match moyenne_matiere(&entrees, &coefs, regles) {
                Ok(m) => {
                    if let Some(x) = m.exacte {
                        moyennes.push(x);
                    }
                    (m.affichee_m, m.nb_comptees())
                }
                Err(_) => {
                    moyenne_indisponible = true;
                    (None, 0)
                }
            };
            lignes.push(EleveLigneDto {
                eleve_id: e.id,
                nom: e.nom.clone(),
                prenom: e.prenom.clone(),
                mouvement,
                cellules,
                moyenne_m,
                nb_comptees: nb,
            });
        }
        if moyenne_indisponible {
            avertissements.push(
                "La moyenne de certains élèves est indisponible : calcul impossible (données ou réglage incohérents)."
                    .to_string(),
            );
        }
        let moyenne_classe = match statistiques(&moyennes, regles) {
            Ok(s) => libelle_stats(s, regles),
            Err(_) => None,
        };
        let mut enseignement = g.cours.clone();
        enseignement.droit = if acces.saisie { "saisie" } else { "lecture" }.to_string();
        Ok(GrilleV3Dto {
            enseignement,
            periode: g.periode.clone(),
            jour,
            peut_saisir: acces.saisie,
            peut_corriger: acces.corriger,
            echelle_m: regles.echelle_m,
            pas_m: g.reglage.pas_saisie_m,
            delai_publication_max_jours: delai,
            evaluations: colonnes,
            eleves: lignes,
            moyenne_classe,
            avertissements,
        })
    }

    /// Explication ligne à ligne de la moyenne d'un élève dans ce cours (survol / détail).
    pub async fn eva_v3_explication_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        cours_id: Uuid,
        periode_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<ExplicationDto, EntDbError> {
        let acces = self
            .eva_v3_acces(etab, acteur, role, cours_id, jour)
            .await?;
        let g = self
            .eva_v3_charger(etab, acteur, role, cours_id, periode_id, jour, acces.saisie)
            .await?;
        if !g.eleves.iter().any(|e| e.id == eleve_id) {
            return Err(introuvable());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let coefs = Self::eva_v3_coefs_sous_matieres(&mut tx, etab, &g).await?;
        tx.commit().await.map_err(db)?;
        let entrees = Self::eva_v3_entrees(&g, eleve_id);
        let regles = &g.reglage.regles;
        match moyenne_matiere(&entrees, &coefs, regles) {
            Ok(m) => Ok(ExplicationDto {
                moyenne_m: m.affichee_m,
                echelle_m: regles.echelle_m,
                lignes: m.explication_fr(regles),
                avertissements: m.avertissements.clone(),
            }),
            Err(_) => Ok(ExplicationDto {
                moyenne_m: None,
                echelle_m: regles.echelle_m,
                lignes: Vec::new(),
                avertissements: vec![
                    "Calcul impossible : données ou réglage incohérents.".to_string()
                ],
            }),
        }
    }

    // ───────────────────────── Saisie d'un lot de cases ─────────────────────────

    /// Écrit un lot de cases (frappe continue, collage, import confirmé). **Chaque case est indépendante**
    /// (point de sauvegarde) : une case refusée ou en conflit n'empêche pas les autres, et le retour dit
    /// laquelle a échoué et pourquoi. Jamais d'écrasement silencieux : la version lue par l'écran est
    /// exigée (`version_attendue`), un écart rend l'état actuel de la case.
    pub async fn eva_v3_saisir_lot_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &SaisieLotBody,
    ) -> Result<SaisieLotRetourDto, EntDbError> {
        self.eva_v3_saisir_lot_interne(etab, acteur, role, jour, body, None)
            .await
    }

    pub(crate) async fn eva_v3_saisir_lot_interne(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &SaisieLotBody,
        journal_mode: Option<&str>,
    ) -> Result<SaisieLotRetourDto, EntDbError> {
        if body.cellules.len() > MAX_CELLULES_PAR_LOT {
            return Err(refus("trop de cases dans un même envoi"));
        }
        let acces = self
            .eva_v3_acces(etab, acteur, role, body.cours_id, jour)
            .await?;
        if !acces.saisie && !acces.corriger {
            return Err(EntDbError::Forbidden);
        }
        // F3 : `import` est RÉSERVÉ à la confirmation d'un import de fichier (seul appelant qui passe `journal_mode`) ;
        // `/eva/saisie` écrit TOUJOURS `saisie`, quoi que dise le corps de la requête.
        if !matches!(body.origine.as_deref(), None | Some("saisie") | Some("import")) {
            return Err(refus("origine de saisie inconnue"));
        }
        let origine = if journal_mode.is_some() && body.origine.as_deref() == Some("import") {
            SourceResultat::Import
        } else {
            SourceResultat::Saisie
        };
        // Élèves protégés : « introuvables » pour un tiers (garde du socle 2), calculé une fois.
        let profil = self.profil_champs(etab, acteur, role).await?;
        let ids: Vec<Uuid> = body.cellules.iter().map(|c| c.eleve_id).collect();
        let caches = self.proteges_caches(etab, role, &profil, &ids).await?;

        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Évaluations visées (une seule lecture) : doivent être du cours et ouvertes.
        let ev_ids: Vec<Uuid> = body
            .cellules
            .iter()
            .map(|c| c.evaluation_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let rows = sqlx::query(
            "SELECT e.id, e.cours_id, (e.bareme * 1000)::bigint AS bareme_m, e.hors_bareme_autorise, e.etat, c.annee_id
               FROM eva_evaluation e JOIN ent_cours c ON c.id = e.cours_id AND c.etablissement_id = e.etablissement_id
              WHERE e.etablissement_id = $1 AND e.id = ANY($2)",
        )
        .bind(etab)
        .bind(&ev_ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut evs: HashMap<Uuid, (Uuid, i64, bool, String, Uuid)> = HashMap::new();
        for r in &rows {
            evs.insert(
                r.try_get("id").map_err(db)?,
                (
                    r.try_get("cours_id").map_err(db)?,
                    r.try_get("bareme_m").map_err(db)?,
                    r.try_get("hors_bareme_autorise").map_err(db)?,
                    r.try_get("etat").map_err(db)?,
                    r.try_get("annee_id").map_err(db)?,
                ),
            );
        }
        let mut pas_par_annee: HashMap<Uuid, i64> = HashMap::new();
        let mut retours: Vec<CelluleRetourDto> = Vec::with_capacity(body.cellules.len());
        let (mut n_ecrites, mut n_inchangees, mut n_conflits, mut n_refusees) =
            (0i32, 0i32, 0i32, 0i32);
        for cell in &body.cellules {
            let retour = self
                .eva_v3_une_case(
                    &mut tx,
                    etab,
                    acteur,
                    jour,
                    &acces,
                    body.cours_id,
                    origine,
                    &evs,
                    &mut pas_par_annee,
                    &caches,
                    cell,
                )
                .await?;
            match retour.issue.as_str() {
                "ecrit" => n_ecrites += 1,
                "inchange" | "vide" => n_inchangees += 1,
                "conflit" => n_conflits += 1,
                _ => n_refusees += 1,
            }
            retours.push(retour);
        }
        if let Some(mode) = journal_mode {
            sqlx::query(
                "INSERT INTO eva_import_journal (id, etablissement_id, cours_id, mode, nb_lignes, nb_cellules,
                                                 nb_ecrites, nb_inchangees, nb_conflits, nb_refusees, par)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(body.cours_id)
            .bind(mode)
            .bind(body.cellules.iter().map(|c| c.eleve_id).collect::<BTreeSet<_>>().len() as i32)
            .bind(body.cellules.len() as i32)
            .bind(n_ecrites)
            .bind(n_inchangees)
            .bind(n_conflits)
            .bind(n_refusees)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(SaisieLotRetourDto { cellules: retours })
    }

    #[allow(clippy::too_many_arguments)]
    async fn eva_v3_une_case(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        acteur: Uuid,
        jour: NaiveDate,
        acces: &AccesCours,
        cours_id: Uuid,
        origine: SourceResultat,
        evs: &HashMap<Uuid, (Uuid, i64, bool, String, Uuid)>,
        pas_par_annee: &mut HashMap<Uuid, i64>,
        caches: &HashSet<Uuid>,
        cell: &CelluleSaisieDto,
    ) -> Result<CelluleRetourDto, EntDbError> {
        let base = |issue: &str, message: Option<String>| CelluleRetourDto {
            evaluation_id: cell.evaluation_id,
            eleve_id: cell.eleve_id,
            issue: issue.to_string(),
            version: None,
            statut: None,
            valeur_m: None,
            message,
            conflit: None,
        };
        let Some((ev_cours, bareme_m, hors_bareme, etat, annee_id)) = evs.get(&cell.evaluation_id)
        else {
            return Ok(base("refuse", Some("évaluation introuvable".to_string())));
        };
        if *ev_cours != cours_id {
            return Ok(base(
                "refuse",
                Some("évaluation d'un autre cours".to_string()),
            ));
        }
        if caches.contains(&cell.eleve_id) {
            return Ok(base("refuse", Some("élève introuvable".to_string())));
        }
        if etat != "ouverte" {
            return Ok(base(
                "refuse",
                Some("évaluation annulée ou non ouverte à la saisie".to_string()),
            ));
        }
        let pas_m = match pas_par_annee.get(annee_id) {
            Some(p) => *p,
            None => {
                let p = regles_tx(tx, etab, *annee_id).await?.pas_saisie_m;
                pas_par_annee.insert(*annee_id, p);
                p
            }
        };
        let contrainte = ContrainteCellule {
            bareme_m: *bareme_m,
            hors_bareme_autorise: *hors_bareme,
            pas_m,
        };
        if cell.texte.chars().count() > MAX_TEXTE_CASE {
            return Ok(base("refuse", Some("texte de case trop long (40 caractères au plus)".to_string())));
        }
        if cell.motif.as_deref().is_some_and(|m| m.trim().chars().count() > MAX_MOTIF) {
            return Ok(base("refuse", Some("motif trop long (500 caractères au plus)".to_string())));
        }
        let (statut, valeur_m) = match analyser_cellule(&cell.texte, &contrainte) {
            Analyse::Vide => return Ok(base("vide", None)),
            Analyse::Erreur(m) => return Ok(base("refuse", Some(m))),
            Analyse::Statut(s) => (StatutCalcul::parse(s).ok_or(refus("statut inconnu"))?, None),
            Analyse::Note(m) => (StatutCalcul::Note, Some(m)),
        };
        // Direction (sans droit de saisie) : seulement une correction après l'arrêt, motivée.
        if !acces.saisie {
            let periode = crate::evaluations_v1::periode_eleve_tx(
                tx,
                etab,
                cell.evaluation_id,
                cell.eleve_id,
            )
            .await?;
            if periode.as_ref().is_some_and(|p| p.etat == "ouverte") {
                return Ok(base(
                    "refuse",
                    Some("période ouverte : la saisie revient au professeur du cours".to_string()),
                ));
            }
        }
        let attendu = match cell.version_attendue {
            None => Attendu::Vide,
            Some(v) => Attendu::Version(v),
        };
        let body = EcrireResultat {
            evaluation_id: cell.evaluation_id,
            eleve_id: cell.eleve_id,
            statut,
            valeur_m,
            motif: cell.motif.clone(),
            commentaire_eleve: None,
            source: origine,
        };
        sqlx::query("SAVEPOINT eva_v3_case")
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        match Self::ecrire_resultat_tx(tx, etab, acteur, acces.corriger, jour, &body, attendu).await
        {
            Ok(ecrit) => {
                sqlx::query("RELEASE SAVEPOINT eva_v3_case")
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                let inchange = matches!(attendu, Attendu::Version(v) if v == ecrit.version);
                Ok(CelluleRetourDto {
                    version: Some(ecrit.version),
                    statut: Some(statut.as_str().to_string()),
                    valeur_m,
                    ..base(if inchange { "inchange" } else { "ecrit" }, None)
                })
            }
            Err(e) => {
                sqlx::query("ROLLBACK TO SAVEPOINT eva_v3_case")
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                sqlx::query("RELEASE SAVEPOINT eva_v3_case")
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                match e {
                    EntDbError::ConflitDossier(_) => {
                        let actuel = Self::eva_v3_conflit(tx, etab, cell).await?;
                        Ok(CelluleRetourDto {
                            conflit: actuel,
                            ..base(
                                "conflit",
                                Some("la case a changé depuis votre lecture".to_string()),
                            )
                        })
                    }
                    EntDbError::Forbidden => Ok(base(
                        "refuse",
                        Some(
                            "période arrêtée : seule la direction corrige, avec un motif écrit"
                                .to_string(),
                        ),
                    )),
                    EntDbError::SaisieInvalide(m) => Ok(base("refuse", Some(m.to_string()))),
                    EntDbError::Domain(domain_eleve::DomainError::NotFound) => Ok(base(
                        "refuse",
                        Some("élève ou évaluation introuvable".to_string()),
                    )),
                    EntDbError::Domain(_) | EntDbError::Sqlx(_) => Ok(base(
                        "refuse",
                        Some(
                            "refusé par les règles de l'évaluation (public, période ou barème)"
                                .to_string(),
                        ),
                    )),
                    autre => Err(autre),
                }
            }
        }
    }

    async fn eva_v3_conflit(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        cell: &CelluleSaisieDto,
    ) -> Result<Option<ConflitDto>, EntDbError> {
        let r = sqlx::query(
            "SELECT r.statut, (r.valeur * 1000)::bigint AS valeur_m, r.version, r.saisi_le,
                    (SELECT btrim(COALESCE(p.prenom, '') || ' ' || p.nom) FROM ent_personne p
                      WHERE p.etablissement_id = r.etablissement_id AND p.user_id = r.saisi_par LIMIT 1) AS par_nom
               FROM eva_resultat r
              WHERE r.etablissement_id = $1 AND r.evaluation_id = $2 AND r.eleve_id = $3",
        )
        .bind(etab)
        .bind(cell.evaluation_id)
        .bind(cell.eleve_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        r.map(|r| {
            Ok(ConflitDto {
                statut: r.try_get("statut")?,
                valeur_m: r.try_get("valeur_m")?,
                version: r.try_get("version")?,
                par_nom: r.try_get("par_nom")?,
                le: Some(r.try_get("saisi_le")?),
            })
        })
        .transpose()
        .map_err(db)
    }

    // ───────────────────────── Historique d'une note ─────────────────────────

    /// Historique d'une case : qui, quand, avant, après, motif (EVA-139). Lisible du professeur du cours et
    /// de la direction ; élève protégé « introuvable » pour un tiers.
    pub async fn eva_v3_historique_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        evaluation_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<LigneHistoriqueV3Dto>, EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cours_id: Uuid = sqlx::query_scalar(
            "SELECT cours_id FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        let acces = self
            .eva_v3_acces(etab, acteur, role, cours_id, jour)
            .await?;
        // F6 (EVA-139) : l'historique d'une case est réservé au professeur du cours et à la direction
        // (`notes.corriger`), pas à tout détenteur de `notes.lire` (le secrétariat lit la grille, pas l'historique).
        if !(acces.prof_du_cours || acces.corriger) {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT h.version, h.statut_avant, (h.valeur_avant * 1000)::bigint AS valeur_avant_m, h.statut_apres,
                    (h.valeur_apres * 1000)::bigint AS valeur_apres_m, h.correction, h.motif, h.source, h.le,
                    (SELECT btrim(COALESCE(p.prenom, '') || ' ' || p.nom) FROM ent_personne p
                      WHERE p.etablissement_id = h.etablissement_id AND p.user_id = h.auteur LIMIT 1) AS auteur_nom
               FROM eva_resultat_historique h
              WHERE h.etablissement_id = $1 AND h.evaluation_id = $2 AND h.eleve_id = $3
              ORDER BY h.version",
        )
        .bind(etab)
        .bind(evaluation_id)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        rows.iter()
            .map(|r| {
                Ok(LigneHistoriqueV3Dto {
                    version: r.try_get("version")?,
                    statut_avant: r.try_get("statut_avant")?,
                    valeur_avant_m: r.try_get("valeur_avant_m")?,
                    statut_apres: r.try_get("statut_apres")?,
                    valeur_apres_m: r.try_get("valeur_apres_m")?,
                    correction: r.try_get("correction")?,
                    motif: r.try_get("motif")?,
                    source: r.try_get("source")?,
                    auteur_nom: r.try_get("auteur_nom")?,
                    le: r.try_get("le")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)
    }
}
