//! Socle 21, lot V11 « alertes précoces », partie **déterministe** (`docs/socles/21_EVALUATIONS.md` §13.3, §13.8,
//! §14.2, R-V20, R-V40, S-118 ; EVA-109, EVA-154, EVA-155, EVA-209, EVA-276, EVA-277).
//!
//! * **Réglage** (EVA-109, S-118) : trois positions (souple, normal, strict), lues dans `eva_reglage.seuils_alertes`
//!   (461) ; la direction la choisit, même après le gel des règles de calcul (le déclencheur de 461 le permet).
//! * **Alertes d'une classe** (EVA-154, EVA-209) : pour chaque élève de la classe pendant la période, trois règles
//!   écrites, chacune expliquée par ses faits : baisse de la moyenne générale depuis la période précédente, matières sous
//!   le seuil, absences et non-rendus aux évaluations. Professeur principal, CPE référent (mission `cpe_referent` sur la
//!   classe ou son niveau), direction.
//! * **Alertes d'un cours** (EVA-155) : le professeur, pour **ses** cours seulement (« 4 élèves sous 8 dans ce
//!   groupe ») : moyenne du cours sous le seuil, absences et non-rendus aux évaluations du cours.
//! * **Ce qui a changé depuis le dernier conseil** (EVA-276) : évolution des moyennes, alertes nouvelles, levées ou
//!   maintenues, appréciations générale et de vie scolaire validées de la période précédente, suites posées.
//! * **Suite humaine** (EVA-154, EVA-277) : une personne trace ce qu'elle a fait (vue, entretien, famille informée,
//!   cellule de veille, sans suite). C'est la seule écriture du lot : aucune alerte n'a de conséquence automatique.
//! * **Exclusions écrites de l'IA** (R-V40), affichées au personnel qui le demande, avec leur raison.
//! * **Journal des consultations** (§13.8 : les alertes évaluent des personnes) : chaque lecture d'une vue d'alertes y
//!   laisse une ligne ; la direction le lit.
//!
//! Garde-fous : **aucun modèle**, aucun score, aucun classement (les élèves sont rangés par nom), jamais visible de la
//! famille ni de l'élève (403 avant toute lecture), élève protégé du socle 2 retiré de la vue d'un tiers.
//! CE-70 : une absence à une évaluation n'est pas comptée quand l'élève a ce jour-là un aménagement de scolarité actif
//! (`ent_amenagement`) ou que le résultat porte la marque d'aménagement (V4), **sans en dire la nature** ; un travail non
//! rendu compte toujours.
//!
//! Droits (D-069, faute d'action `evaluations.suivre_classe` au catalogue) : la direction est reconnue à
//! `periodes.verrouiller` (comme V8 et V10) ; le professeur principal par le service (`professeur_principal`) ou par la
//! mission ; le CPE par la mission `cpe_referent` seulement (le rôle CPE seul ne suffit pas : minimisation).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{Duration, NaiveDate};
use domain_eleve::evaluation_moteur::{
    formater_fr, moyenne_cours, Comptage, EntreeCalcul, EvalCalcul, ReglesMoteur, StatutCalcul,
};
use ent_auth::Action;
use ent_types::evaluations_v11::*;
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{db, introuvable, refus, texte_borne};
use crate::evaluations_v1::{moyennes_tx, regles_tx};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Rappel affiché avec toute liste d'alertes.
pub const MENTION_ALERTES: &str = "Alertes calculées par des règles fixées par l'établissement, sans intelligence \
    artificielle. Elles disent des faits, jamais un risque ni une probabilité ; elles n'ont aucune conséquence \
    automatique et ne sont jamais montrées à la famille ni à l'élève. La suite éventuelle est décidée par une personne.";

/// Fenêtre maximale de lecture du journal des consultations.
const JOURNAL_MAX_JOURS: i64 = 366;
const JOURNAL_MAX_LIGNES: i64 = 500;

/// Les faits d'un élève sur une période, d'où les alertes se déduisent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FaitsEleve {
    pub generale_m: Option<i64>,
    pub generale_precedente_m: Option<i64>,
    pub matieres: Vec<MatiereAlerteDto>,
    /// Absences comptées aux évaluations (hors aménagement de scolarité, CE-70).
    pub absences: usize,
    pub non_rendus: usize,
}

fn pluriel(n: usize, un: &str, plusieurs: &str) -> String {
    if n > 1 {
        format!("{n} {plusieurs}")
    } else {
        format!("{n} {un}")
    }
}

fn fait_absences(absences: usize, non_rendus: usize, ou: &str) -> String {
    let mut morceaux = Vec::new();
    if absences > 0 {
        morceaux.push(pluriel(absences, "absence", "absences"));
    }
    if non_rendus > 0 {
        morceaux.push(pluriel(
            non_rendus,
            "travail non rendu",
            "travaux non rendus",
        ));
    }
    format!("{} {ou}.", morceaux.join(" et "))
}

fn alerte(regle: RegleAlerte, faits: Vec<String>) -> AlerteDto {
    AlerteDto {
        regle,
        libelle: regle.libelle().to_string(),
        faits,
    }
}

/// Règles d'une classe (§13.3), pures : mêmes faits, mêmes alertes. Aucune pondération, aucun score.
pub(crate) fn evaluer_eleve(s: &SeuilsAlertes, f: &FaitsEleve) -> Vec<AlerteDto> {
    let mut out = Vec::new();
    if let (Some(avant), Some(apres)) = (f.generale_precedente_m, f.generale_m) {
        let baisse = avant - apres;
        if baisse >= s.baisse_generale_m {
            out.push(alerte(
                RegleAlerte::BaisseGenerale,
                vec![format!(
                    "Moyenne générale : {} à la période précédente, {} sur cette période, soit {} point(s) de moins \
                     (seuil : {}).",
                    formater_fr(avant),
                    formater_fr(apres),
                    formater_fr(baisse),
                    formater_fr(s.baisse_generale_m)
                )],
            ));
        }
    }
    let basses: Vec<&MatiereAlerteDto> = f
        .matieres
        .iter()
        .filter(|m| m.moyenne_m.is_some_and(|v| v < s.note_basse_m))
        .collect();
    if basses.len() >= s.matieres_sous_seuil {
        let mut faits = vec![format!(
            "{} sous {} (seuil : {} matières).",
            pluriel(basses.len(), "matière", "matières"),
            formater_fr(s.note_basse_m),
            s.matieres_sous_seuil
        )];
        for m in basses {
            faits.push(format!(
                "{} : {}.",
                m.libelle,
                formater_fr(m.moyenne_m.unwrap_or_default())
            ));
        }
        out.push(alerte(RegleAlerte::MatieresSousSeuil, faits));
    }
    if f.absences + f.non_rendus >= s.absences_evaluations {
        out.push(alerte(
            RegleAlerte::AbsencesEvaluations,
            vec![fait_absences(
                f.absences,
                f.non_rendus,
                "aux évaluations de la période",
            )],
        ));
    }
    out
}

/// Règles d'un cours (vue du professeur), pures.
pub(crate) fn evaluer_cours(
    s: &SeuilsAlertes,
    moyenne_m: Option<i64>,
    absences: usize,
    non_rendus: usize,
) -> Vec<AlerteDto> {
    let mut out = Vec::new();
    if let Some(m) = moyenne_m.filter(|m| *m < s.note_basse_m) {
        out.push(alerte(
            RegleAlerte::CoursSousSeuil,
            vec![format!(
                "Moyenne du cours : {} (seuil : {}).",
                formater_fr(m),
                formater_fr(s.note_basse_m)
            )],
        ));
    }
    if absences + non_rendus >= s.absences_evaluations {
        out.push(alerte(
            RegleAlerte::AbsencesEvaluations,
            vec![fait_absences(
                absences,
                non_rendus,
                "aux évaluations de ce cours sur la période",
            )],
        ));
    }
    out
}

/// Les exclusions écrites de l'IA (R-V40, §13.8), avec leur raison.
pub fn exclusions_ia() -> Vec<ExclusionIaDto> {
    let e = |code: &str, quoi: &str, raison: &str| ExclusionIaDto {
        code: code.into(),
        ce_que_l_ia_ne_fait_pas: quoi.into(),
        raison: raison.into(),
    };
    vec![
        e(
            "correction_copies",
            "Corriger une copie ou proposer une note.",
            "Évaluer les acquis d'apprentissage relève du haut risque (règlement européen sur l'IA, annexe III, point 3 b) : \
             la note reste l'acte du professeur.",
        ),
        e(
            "detection_fraude",
            "Détecter la fraude ou le plagiat.",
            "Surveiller et détecter les comportements interdits pendant les tests relève du haut risque (annexe III, point 3 d).",
        ),
        e(
            "formation_groupes",
            "Former des groupes d'élèves (niveau, remédiation, besoin) ou orienter individuellement leurs apprentissages.",
            "Orienter le processus d'apprentissage d'un élève relève du haut risque (annexe III, point 3 b).",
        ),
        e(
            "mention_avis_decision",
            "Proposer une mention, un avis ou une décision (passage, orientation, sanction).",
            "Aucune décision n'est produite automatiquement à partir d'une moyenne (R-V20, RGPD article 22).",
        ),
        e(
            "alertes_par_modele",
            "Remplacer les alertes précoces par un modèle.",
            "Les alertes sont des règles écrites par des personnes ; elles évaluent des personnes (profilage) : aucun modèle \
             ne s'y substitue sans l'étude juridique (Q-EV8).",
        ),
    ]
}

// ───────────────────────── Lectures communes ─────────────────────────

struct PeriodeLue {
    id: Uuid,
    annee_id: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
}

async fn periode_tx(tx: &mut TenantTx<'_>, etab: Uuid, id: Uuid) -> Result<PeriodeLue, EntDbError> {
    let l = sqlx::query("SELECT annee_id, date_debut, date_fin FROM ent_periode WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
    Ok(PeriodeLue {
        id,
        annee_id: l.try_get("annee_id").map_err(db)?,
        debut: l.try_get("date_debut").map_err(db)?,
        fin: l.try_get("date_fin").map_err(db)?,
    })
}

/// La période qui précède, dans la même périodisation, la même année et la même nature (trimestre, semestre). La
/// première période de l'année n'en a pas (la baisse ne se calcule pas d'une année sur l'autre).
async fn precedente_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    p: &PeriodeLue,
) -> Result<Option<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT q.id FROM ent_periode p
           JOIN ent_periode q ON q.etablissement_id = p.etablissement_id AND q.annee_id = p.annee_id
            AND q.periodisation_id IS NOT DISTINCT FROM p.periodisation_id AND q.kind = p.kind
            AND q.date_fin < p.date_debut
          WHERE p.etablissement_id = $1 AND p.id = $2
          ORDER BY q.date_fin DESC, q.id LIMIT 1",
    )
    .bind(etab)
    .bind(p.id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)
}

async fn seuils_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    annee: Uuid,
) -> Result<(SeuilsAlertes, ReglesMoteur), EntDbError> {
    let regles = regles_tx(tx, etab, annee).await?.regles;
    let pos: Option<String> = sqlx::query_scalar(
        "SELECT seuils_alertes FROM eva_reglage WHERE etablissement_id = $1 AND annee_id = $2",
    )
    .bind(etab)
    .bind(annee)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    let position = pos
        .as_deref()
        .and_then(PositionAlertes::parse)
        .unwrap_or_default();
    Ok((position.seuils(regles.echelle_m), regles))
}

/// L'acteur suit-il la classe au jour donné : professeur principal (service ou mission) ou CPE référent (mission sur la
/// classe ou sur son niveau) ?
async fn suit_classe_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    acteur: Uuid,
    classe: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement s
                         JOIN ent_classe c ON c.etablissement_id = s.etablissement_id AND c.id = s.classe_id
                        WHERE s.etablissement_id = $1 AND s.classe_id = $3 AND s.prof_user_id = $2
                          AND s.professeur_principal AND s.annee_id = c.annee_id)
             OR EXISTS (SELECT 1 FROM ent_mission m
                         JOIN ent_mission_type t ON t.id = m.type_id AND t.code IN ('professeur_principal', 'cpe_referent')
                         JOIN ent_agent a ON a.etablissement_id = m.etablissement_id AND a.id = m.agent_id
                         JOIN ent_personne pe ON pe.etablissement_id = a.etablissement_id AND pe.id = a.personne_id
                         JOIN ent_mission_perimetre p ON p.etablissement_id = m.etablissement_id AND p.mission_id = m.id
                         JOIN ent_classe c ON c.etablissement_id = m.etablissement_id AND c.id = $3
                        WHERE m.etablissement_id = $1 AND pe.user_id = $2 AND m.annule_le IS NULL
                          AND m.du <= $4 AND (m.au IS NULL OR m.au >= $4)
                          AND ((p.nature = 'classe' AND p.cible_id = c.id)
                               OR (t.code = 'cpe_referent' AND p.nature = 'niveau' AND p.cible_id = c.niveau_id)))",
    )
    .bind(etab)
    .bind(acteur)
    .bind(classe)
    .bind(jour)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)
}

/// Classe de l'élève pendant la période : la dernière affectation qui la recoupe.
async fn classe_eleve_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    eleve: Uuid,
    p: &PeriodeLue,
) -> Result<Option<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT a.classe_id FROM ent_affectation a
          WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.classe_id IS NOT NULL
            AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $3)
          ORDER BY a.date_debut DESC, a.id LIMIT 1",
    )
    .bind(etab)
    .bind(eleve)
    .bind(p.debut)
    .bind(p.fin)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)
}

/// Absences comptées et non-rendus d'un élève aux évaluations ouvertes de la période (une ligne par évaluation, même
/// après une fusion de fiches). `cours` restreint à un cours. CE-70 : une absence n'est pas comptée quand le résultat
/// porte la marque d'aménagement ou que l'élève a ce jour-là un aménagement de scolarité actif.
async fn absences_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    eleve: Uuid,
    periode: Uuid,
    cours: Option<Uuid>,
) -> Result<(usize, usize), EntDbError> {
    let lignes: Vec<(String, bool)> = sqlx::query_as(
        "SELECT q.statut, q.amenage FROM (
           SELECT DISTINCT ON (r.evaluation_id) r.statut,
                  (r.amenage OR EXISTS (SELECT 1 FROM ent_amenagement am
                                         WHERE am.etablissement_id = r.etablissement_id AND am.eleve_id = r.eleve_id
                                           AND am.actif AND am.retire_at IS NULL AND am.date_debut <= e.date_eval
                                           AND (am.date_fin IS NULL OR am.date_fin >= e.date_eval))) AS amenage
             FROM eva_resultat r
             JOIN eva_evaluation e ON e.etablissement_id = r.etablissement_id AND e.id = r.evaluation_id
            WHERE r.etablissement_id = $1 AND r.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
              AND e.etat = 'ouverte' AND eva_periode_dans(e.etablissement_id, $3, e.date_eval)
              AND ($4::uuid IS NULL OR e.cours_id = $4)
            ORDER BY r.evaluation_id, (r.eleve_id = $2) DESC, r.saisi_le DESC, r.id) q
          WHERE q.statut IN ('absent', 'a_rattraper', 'non_rendu')",
    )
    .bind(etab)
    .bind(eleve)
    .bind(periode)
    .bind(cours)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut absences = 0;
    let mut non_rendus = 0;
    for (statut, amenage) in lignes {
        if statut == "non_rendu" {
            non_rendus += 1;
        } else if !amenage {
            absences += 1;
        }
    }
    Ok((absences, non_rendus))
}

async fn libelles_matieres_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, String>, EntDbError> {
    let v: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, libelle FROM ent_matiere WHERE etablissement_id = $1 AND id = ANY($2)",
    )
    .bind(etab)
    .bind(ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    Ok(v.into_iter().collect())
}

/// Moyenne générale et moyennes de matière (millièmes d'affichage) d'un élève sur une période, par le moteur unique
/// (toutes les évaluations ouvertes : vue du personnel). Un calcul impossible devient un avertissement.
async fn moyennes_eleve_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    eleve: Uuid,
    periode: Uuid,
    avertissements: &mut Vec<String>,
    qui: &str,
) -> Result<(Option<i64>, Vec<MatiereAlerteDto>), EntDbError> {
    let m = match moyennes_tx(tx, etab, eleve, periode, false).await {
        Ok(m) => m,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound)) => return Err(introuvable()),
        Err(_) => {
            avertissements.push(format!(
                "Moyennes indisponibles pour {qui} : calcul impossible."
            ));
            return Ok((None, Vec::new()));
        }
    };
    let ids: Vec<Uuid> = m
        .matieres
        .iter()
        .map(|x| x.matiere_id)
        .filter(|x| !x.is_nil())
        .collect();
    let libelles = libelles_matieres_tx(tx, etab, &ids).await?;
    let mut matieres: Vec<MatiereAlerteDto> = m
        .matieres
        .iter()
        .map(|x| MatiereAlerteDto {
            matiere_id: (!x.matiere_id.is_nil()).then_some(x.matiere_id),
            libelle: libelles
                .get(&x.matiere_id)
                .cloned()
                .unwrap_or_else(|| "Matière non renseignée".into()),
            moyenne_m: x.moyenne.affichee_m,
        })
        .collect();
    matieres.sort_by(|a, b| a.libelle.cmp(&b.libelle));
    Ok((m.generale.affichee_m, matieres))
}

async fn derniere_suite_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    eleve: Uuid,
    periode: Uuid,
) -> Result<Option<SuiteDto>, EntDbError> {
    Ok(suites_tx(tx, etab, eleve, Some(periode))
        .await?
        .into_iter()
        .next())
}

async fn suites_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    eleve: Uuid,
    periode: Option<Uuid>,
) -> Result<Vec<SuiteDto>, EntDbError> {
    let lignes = sqlx::query(
        "SELECT id, eleve_id, periode_id, nature, commentaire, auteur_id, le FROM eva_alerte_suite
          WHERE etablissement_id = $1 AND eleve_id = $2 AND ($3::uuid IS NULL OR periode_id = $3)
          ORDER BY le DESC, id DESC LIMIT 200",
    )
    .bind(etab)
    .bind(eleve)
    .bind(periode)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut out = Vec::with_capacity(lignes.len());
    for l in lignes {
        let nature: String = l.try_get("nature").map_err(db)?;
        out.push(SuiteDto {
            id: l.try_get("id").map_err(db)?,
            eleve_id: l.try_get("eleve_id").map_err(db)?,
            periode_id: l.try_get("periode_id").map_err(db)?,
            nature: NatureSuite::parse(&nature).ok_or(refus("nature de suite inconnue"))?,
            commentaire: l.try_get("commentaire").map_err(db)?,
            auteur_id: l.try_get("auteur_id").map_err(db)?,
            le: l.try_get("le").map_err(db)?,
        });
    }
    Ok(out)
}

/// Une ligne du journal des consultations.
struct Consultation {
    vue: &'static str,
    periode: Option<Uuid>,
    classe: Option<Uuid>,
    cours: Option<Uuid>,
    eleve: Option<Uuid>,
}

async fn journaliser_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    acteur: Uuid,
    c: Consultation,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO eva_alerte_consultation (id, etablissement_id, acteur_id, vue, periode_id, classe_id, cours_id, eleve_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(acteur)
    .bind(c.vue)
    .bind(c.periode)
    .bind(c.classe)
    .bind(c.cours)
    .bind(c.eleve)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

fn evolution(
    matiere_id: Option<Uuid>,
    libelle: String,
    avant_m: Option<i64>,
    apres_m: Option<i64>,
) -> EvolutionMatiereDto {
    EvolutionMatiereDto {
        matiere_id,
        libelle,
        avant_m,
        apres_m,
        ecart_m: match (avant_m, apres_m) {
            (Some(a), Some(b)) => Some(b - a),
            _ => None,
        },
    }
}

impl PgEnt {
    async fn eva_v11_direction(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<bool, EntDbError> {
        Self::exiger_staff(role)?;
        self.a_action_etablissement(etab, acteur, role, Action::PeriodesVerrouiller)
            .await
    }

    /// Direction, ou personne qui suit la classe de l'élève pendant la période (PP, CPE référent). Rend la classe.
    async fn eva_v11_exiger_suivi_eleve(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        eleve: Uuid,
        p: &PeriodeLue,
    ) -> Result<(), EntDbError> {
        let direction = self.eva_v11_direction(etab, acteur, role).await?;
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        if !direction {
            let Some(classe) = classe_eleve_tx(tx, etab, eleve, p).await? else {
                return Err(EntDbError::Forbidden);
            };
            if !suit_classe_tx(tx, etab, acteur, classe, jour).await? {
                return Err(EntDbError::Forbidden);
            }
        }
        self.exiger_eleve_visible_conn(&mut **tx, etab, acteur, role, eleve)
            .await
    }

    // ───────────────────────── Réglage (EVA-109) ─────────────────────────

    /// Position et seuils des alertes d'une année. Tout le personnel.
    pub async fn eva_v11_reglage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        annee: Uuid,
    ) -> Result<ReglageAlertesDto, EntDbError> {
        // Les seuils ne sont pas une donnée d'élève : tout le personnel les lit (ils expliquent les alertes).
        let _ = acteur;
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let existe: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2)")
                .bind(etab)
                .bind(annee)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let (seuils, regles) = seuils_tx(&mut tx, etab, annee).await?;
        tx.commit().await.map_err(db)?;
        Ok(ReglageAlertesDto {
            annee_id: annee,
            seuils,
            positions: PositionAlertes::TOUTES
                .iter()
                .map(|p| p.seuils(regles.echelle_m))
                .collect(),
        })
    }

    /// Choisir la position des alertes (direction). Reste possible après le gel des règles de calcul (461). Le jeu de
    /// règles de l'année doit exister : la position ne crée pas de règles par défaut à la place de la direction.
    pub async fn eva_v11_definir_reglage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &DefinirReglageAlertesBody,
    ) -> Result<ReglageAlertesDto, EntDbError> {
        if !self.eva_v11_direction(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE eva_reglage SET seuils_alertes = $3, modifie_par = $4
              WHERE etablissement_id = $1 AND annee_id = $2 AND seuils_alertes IS DISTINCT FROM $3",
        )
        .bind(etab)
        .bind(body.annee_id)
        .bind(body.position.as_str())
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            let existe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM eva_reglage WHERE etablissement_id = $1 AND annee_id = $2)",
            )
            .bind(etab)
            .bind(body.annee_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !existe {
                return Err(refus(
                    "les règles de calcul de l'année ne sont pas encore définies",
                ));
            }
        }
        tx.commit().await.map_err(db)?;
        self.eva_v11_reglage_for_actor(etab, acteur, role, body.annee_id)
            .await
    }

    // ───────────────────────── Alertes d'une classe (EVA-154, EVA-209) ─────────────────────────

    pub async fn eva_v11_alertes_classe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        classe: Uuid,
        periode: Uuid,
    ) -> Result<AlertesClasseDto, EntDbError> {
        let direction = self.eva_v11_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let classe_libelle: String = sqlx::query_scalar(
            "SELECT libelle FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(classe)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if !direction && !suit_classe_tx(&mut tx, etab, acteur, classe, jour).await? {
            return Err(EntDbError::Forbidden);
        }
        let p = periode_tx(&mut tx, etab, periode).await?;
        let precedente = precedente_tx(&mut tx, etab, &p).await?;
        let (seuils, _) = seuils_tx(&mut tx, etab, p.annee_id).await?;
        let mut avertissements = Vec::new();
        if precedente.is_none() {
            avertissements.push(
                "Aucune période précédente dans l'année : la règle de baisse de la moyenne générale ne s'applique pas."
                    .to_string(),
            );
        }
        let membres: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT DISTINCT e.id, e.nom, e.prenom FROM ent_affectation a
               JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
              WHERE a.etablissement_id = $1 AND a.classe_id = $2 AND e.fusionne_dans IS NULL
                AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $3)
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(classe)
        .bind(p.debut)
        .bind(p.fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut eleves = Vec::new();
        let mut effectif = 0;
        for (eleve_id, nom, prenom) in membres {
            if self
                .eleve_cache_au_tiers_conn(&mut *tx, etab, acteur, role, eleve_id)
                .await?
            {
                continue;
            }
            effectif += 1;
            let qui = format!("{prenom} {nom}");
            let (generale_m, matieres) =
                moyennes_eleve_tx(&mut tx, etab, eleve_id, p.id, &mut avertissements, &qui).await?;
            let generale_precedente_m = match precedente {
                Some(pp) => {
                    moyennes_eleve_tx(&mut tx, etab, eleve_id, pp, &mut avertissements, &qui)
                        .await?
                        .0
                }
                None => None,
            };
            let (absences, non_rendus) = absences_tx(&mut tx, etab, eleve_id, p.id, None).await?;
            let faits = FaitsEleve {
                generale_m,
                generale_precedente_m,
                matieres,
                absences,
                non_rendus,
            };
            let alertes = evaluer_eleve(&seuils, &faits);
            if alertes.is_empty() {
                continue;
            }
            let derniere_suite = derniere_suite_tx(&mut tx, etab, eleve_id, p.id).await?;
            eleves.push(EleveAlerteDto {
                eleve_id,
                nom,
                prenom,
                moyenne_generale_m: generale_m,
                moyenne_generale_precedente_m: generale_precedente_m,
                alertes,
                derniere_suite,
            });
        }
        journaliser_tx(
            &mut tx,
            etab,
            acteur,
            Consultation {
                vue: "classe",
                periode: Some(p.id),
                classe: Some(classe),
                cours: None,
                eleve: None,
            },
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(AlertesClasseDto {
            classe_id: classe,
            classe_libelle,
            periode_id: p.id,
            periode_precedente_id: precedente,
            seuils,
            effectif,
            eleves,
            avertissements,
            mention: MENTION_ALERTES.to_string(),
        })
    }

    // ───────────────────────── Alertes d'un cours (EVA-155) ─────────────────────────

    pub async fn eva_v11_alertes_cours_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        cours: Uuid,
        periode: Uuid,
    ) -> Result<AlertesCoursDto, EntDbError> {
        let direction = self.eva_v11_direction(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cours_libelle: String = sqlx::query_scalar(
            "SELECT libelle FROM ent_cours WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(cours)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if !direction {
            let mien: bool = sqlx::query_scalar("SELECT ent_prof_sur_cours($1, $2, $3, $4, false)")
                .bind(etab)
                .bind(cours)
                .bind(acteur)
                .bind(jour)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            if !mien {
                return Err(EntDbError::Forbidden);
            }
        }
        let p = periode_tx(&mut tx, etab, periode).await?;
        let (seuils, regles) = seuils_tx(&mut tx, etab, p.annee_id).await?;
        // Résultats du cours dont la période DE L'ÉLÈVE est la période demandée (R-V29), élèves non fusionnés.
        let lignes = sqlx::query(
            "SELECT e.id, e.titre, (e.bareme * 1000)::bigint AS bareme_m, (e.coef * 1000)::bigint AS coef_m,
                    e.comptage, e.hors_bareme_autorise, e.mode, e.etat, r.eleve_id, r.statut,
                    (r.valeur * 1000)::bigint AS valeur_m, el.nom, el.prenom
               FROM eva_evaluation e
               JOIN eva_resultat r ON r.etablissement_id = e.etablissement_id AND r.evaluation_id = e.id
               JOIN ent_eleve el ON el.etablissement_id = r.etablissement_id AND el.id = r.eleve_id
              CROSS JOIN LATERAL eva_periode_eleve_jour(e.etablissement_id, r.eleve_id, e.date_eval) pe
              WHERE e.etablissement_id = $1 AND e.cours_id = $2 AND e.etat IN ('ouverte', 'annulee')
                AND e.date_eval BETWEEN $4 AND $5 AND pe.periode_id = $3 AND el.fusionne_dans IS NULL
              ORDER BY el.nom, el.prenom, r.eleve_id, e.date_eval, e.id",
        )
        .bind(etab)
        .bind(cours)
        .bind(p.id)
        .bind(p.debut)
        .bind(p.fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        struct Acc {
            nom: String,
            prenom: String,
            entrees: Vec<EntreeCalcul>,
            absences: usize,
            non_rendus: usize,
        }
        let mut ordre: Vec<Uuid> = Vec::new();
        let mut par_eleve: HashMap<Uuid, Acc> = HashMap::new();
        let mut vus: BTreeSet<(Uuid, Uuid)> = BTreeSet::new();
        for l in &lignes {
            let eleve: Uuid = l.try_get("eleve_id").map_err(db)?;
            let ev: Uuid = l.try_get("id").map_err(db)?;
            if !vus.insert((eleve, ev)) {
                continue;
            }
            let acc = par_eleve.entry(eleve).or_insert_with(|| {
                ordre.push(eleve);
                Acc {
                    nom: l.try_get("nom").unwrap_or_default(),
                    prenom: l.try_get("prenom").unwrap_or_default(),
                    entrees: Vec::new(),
                    absences: 0,
                    non_rendus: 0,
                }
            });
            let statut_txt: String = l.try_get("statut").map_err(db)?;
            let Some(statut) = StatutCalcul::parse(&statut_txt) else {
                continue;
            };
            let etat: String = l.try_get("etat").map_err(db)?;
            let mode: String = l.try_get("mode").map_err(db)?;
            if mode == "note" {
                acc.entrees.push(EntreeCalcul {
                    evaluation: EvalCalcul {
                        id: ev,
                        titre: l.try_get("titre").map_err(db)?,
                        bareme_m: l.try_get("bareme_m").map_err(db)?,
                        coef_m: l.try_get("coef_m").map_err(db)?,
                        comptage: Comptage::parse(&l.try_get::<String, _>("comptage").map_err(db)?)
                            .unwrap_or_default(),
                        hors_bareme_autorise: l.try_get("hors_bareme_autorise").map_err(db)?,
                        sous_matiere_id: None,
                        cours_id: Some(cours),
                        annulee: etat == "annulee",
                    },
                    statut,
                    valeur_m: l.try_get("valeur_m").map_err(db)?,
                });
            }
        }
        let mut eleves = Vec::new();
        let mut effectif = 0;
        let mut sous_seuil = 0;
        for eleve_id in ordre {
            if self
                .eleve_cache_au_tiers_conn(&mut *tx, etab, acteur, role, eleve_id)
                .await?
            {
                continue;
            }
            let Some(mut acc) = par_eleve.remove(&eleve_id) else {
                continue;
            };
            effectif += 1;
            let (a, n) = absences_tx(&mut tx, etab, eleve_id, p.id, Some(cours)).await?;
            acc.absences = a;
            acc.non_rendus = n;
            let moyenne_m = if acc.entrees.is_empty() {
                None
            } else {
                moyenne_cours(&acc.entrees, &regles)
                    .ok()
                    .and_then(|m| m.affichee_m)
            };
            if moyenne_m.is_some_and(|m| m < seuils.note_basse_m) {
                sous_seuil += 1;
            }
            let alertes = evaluer_cours(&seuils, moyenne_m, acc.absences, acc.non_rendus);
            if alertes.is_empty() {
                continue;
            }
            eleves.push(EleveCoursAlerteDto {
                eleve_id,
                nom: acc.nom,
                prenom: acc.prenom,
                moyenne_cours_m: moyenne_m,
                alertes,
            });
        }
        journaliser_tx(
            &mut tx,
            etab,
            acteur,
            Consultation {
                vue: "cours",
                periode: Some(p.id),
                classe: None,
                cours: Some(cours),
                eleve: None,
            },
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(AlertesCoursDto {
            cours_id: cours,
            cours_libelle,
            periode_id: p.id,
            seuils,
            effectif,
            sous_seuil,
            eleves,
            mention: MENTION_ALERTES.to_string(),
        })
    }

    // ───────────────────────── Ce qui a changé (EVA-276) ─────────────────────────

    pub async fn eva_v11_changements_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        eleve: Uuid,
        periode: Uuid,
    ) -> Result<ChangementsEleveDto, EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = periode_tx(&mut tx, etab, periode).await?;
        self.eva_v11_exiger_suivi_eleve(&mut tx, etab, acteur, role, jour, eleve, &p)
            .await?;
        let precedente = precedente_tx(&mut tx, etab, &p).await?;
        let (seuils, _) = seuils_tx(&mut tx, etab, p.annee_id).await?;
        let mut avertissements = Vec::new();
        let (g_apres, m_apres) =
            moyennes_eleve_tx(&mut tx, etab, eleve, p.id, &mut avertissements, "l'élève").await?;
        let (abs, nr) = absences_tx(&mut tx, etab, eleve, p.id, None).await?;
        let mut matieres: BTreeMap<String, (Option<Uuid>, Option<i64>, Option<i64>)> =
            BTreeMap::new();
        for m in &m_apres {
            matieres.insert(m.libelle.clone(), (m.matiere_id, None, m.moyenne_m));
        }
        let (g_avant, alertes_avant, appreciations) = match precedente {
            Some(pp) => {
                let (g_avant, m_avant) =
                    moyennes_eleve_tx(&mut tx, etab, eleve, pp, &mut avertissements, "l'élève")
                        .await?;
                for m in &m_avant {
                    matieres
                        .entry(m.libelle.clone())
                        .or_insert((m.matiere_id, None, None))
                        .1 = m.moyenne_m;
                }
                let pp_lue = periode_tx(&mut tx, etab, pp).await?;
                let avant_precedente = precedente_tx(&mut tx, etab, &pp_lue).await?;
                let g_avant_avant = match avant_precedente {
                    Some(q) => {
                        moyennes_eleve_tx(&mut tx, etab, eleve, q, &mut avertissements, "l'élève")
                            .await?
                            .0
                    }
                    None => None,
                };
                let (a2, n2) = absences_tx(&mut tx, etab, eleve, pp, None).await?;
                let alertes_avant = evaluer_eleve(
                    &seuils,
                    &FaitsEleve {
                        generale_m: g_avant,
                        generale_precedente_m: g_avant_avant,
                        matieres: m_avant,
                        absences: a2,
                        non_rendus: n2,
                    },
                );
                let appreciations: Vec<(String, String)> = sqlx::query_as(
                    "SELECT type, texte FROM eva_appreciation
                      WHERE etablissement_id = $1 AND eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
                        AND periode_id = $3 AND type IN ('generale', 'vie_scolaire') AND etat = 'validee'
                      ORDER BY type, maj_le DESC, id",
                )
                .bind(etab)
                .bind(eleve)
                .bind(pp)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                (g_avant, alertes_avant, appreciations)
            }
            None => {
                avertissements.push(
                    "Aucune période précédente dans l'année : rien à comparer depuis le dernier conseil.".to_string(),
                );
                (None, Vec::new(), Vec::new())
            }
        };
        let alertes_apres = evaluer_eleve(
            &seuils,
            &FaitsEleve {
                generale_m: g_apres,
                generale_precedente_m: g_avant,
                matieres: m_apres,
                absences: abs,
                non_rendus: nr,
            },
        );
        let regles_avant: BTreeSet<RegleAlerte> = alertes_avant.iter().map(|a| a.regle).collect();
        let regles_apres: BTreeSet<RegleAlerte> = alertes_apres.iter().map(|a| a.regle).collect();
        let suites = suites_tx(&mut tx, etab, eleve, None).await?;
        journaliser_tx(
            &mut tx,
            etab,
            acteur,
            Consultation {
                vue: "changements",
                periode: Some(p.id),
                classe: None,
                cours: None,
                eleve: Some(eleve),
            },
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ChangementsEleveDto {
            eleve_id: eleve,
            periode_id: p.id,
            periode_precedente_id: precedente,
            generale: evolution(None, "Moyenne générale".into(), g_avant, g_apres),
            matieres: matieres
                .into_iter()
                .map(|(lib, (id, a, b))| evolution(id, lib, a, b))
                .collect(),
            alertes_nouvelles: alertes_apres
                .into_iter()
                .filter(|a| !regles_avant.contains(&a.regle))
                .collect(),
            alertes_levees: regles_avant.difference(&regles_apres).copied().collect(),
            alertes_maintenues: regles_avant.intersection(&regles_apres).copied().collect(),
            appreciations_precedentes: appreciations
                .into_iter()
                .map(|(type_appreciation, texte)| AppreciationRappelDto {
                    type_appreciation,
                    texte,
                })
                .collect(),
            suites,
            avertissements,
        })
    }

    // ───────────────────────── Suites humaines (EVA-154, EVA-277) ─────────────────────────

    pub async fn eva_v11_poser_suite_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        eleve: Uuid,
        body: &PoserSuiteBody,
    ) -> Result<SuiteDto, EntDbError> {
        Self::exiger_staff(role)?;
        let commentaire = match body
            .commentaire
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
        {
            Some(c) => Some(texte_borne(c, 500)?),
            None => None,
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = periode_tx(&mut tx, etab, body.periode_id).await?;
        self.eva_v11_exiger_suivi_eleve(&mut tx, etab, acteur, role, jour, eleve, &p)
            .await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO eva_alerte_suite (id, etablissement_id, eleve_id, periode_id, nature, commentaire, auteur_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve)
        .bind(p.id)
        .bind(body.nature.as_str())
        .bind(&commentaire)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let dto = suites_tx(&mut tx, etab, eleve, Some(p.id))
            .await?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Suites posées pour un élève, toutes périodes (les plus récentes d'abord). Le contrôle se fait sur la période
    /// donnée (classe de l'élève pendant cette période).
    pub async fn eva_v11_suites_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        eleve: Uuid,
        periode: Uuid,
    ) -> Result<Vec<SuiteDto>, EntDbError> {
        Self::exiger_staff(role)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = periode_tx(&mut tx, etab, periode).await?;
        self.eva_v11_exiger_suivi_eleve(&mut tx, etab, acteur, role, jour, eleve, &p)
            .await?;
        let v = suites_tx(&mut tx, etab, eleve, None).await?;
        journaliser_tx(
            &mut tx,
            etab,
            acteur,
            Consultation {
                vue: "suites",
                periode: None,
                classe: None,
                cours: None,
                eleve: Some(eleve),
            },
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    // ───────────────────────── Exclusions et journal ─────────────────────────

    /// Les exclusions écrites de l'IA (R-V40) : tout le personnel.
    pub fn eva_v11_exclusions_ia_for_actor(
        &self,
        role: &str,
    ) -> Result<Vec<ExclusionIaDto>, EntDbError> {
        Self::exiger_staff(role)?;
        Ok(exclusions_ia())
    }

    /// Journal des consultations des alertes (direction), sur une fenêtre d'au plus un an.
    pub async fn eva_v11_consultations_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        du: Option<NaiveDate>,
        au: Option<NaiveDate>,
    ) -> Result<Vec<ConsultationAlerteDto>, EntDbError> {
        if !self.eva_v11_direction(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let au = au.unwrap_or(jour);
        let du = du.unwrap_or(au - Duration::days(30));
        if au < du || (au - du).num_days() > JOURNAL_MAX_JOURS {
            return Err(refus("fenêtre invalide (un an au plus)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = sqlx::query(
            "SELECT id, acteur_id, vue, periode_id, classe_id, cours_id, eleve_id, le FROM eva_alerte_consultation
              WHERE etablissement_id = $1 AND le >= $2::date AND le < ($3::date + 1)
              ORDER BY le DESC, id DESC LIMIT $4",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .bind(JOURNAL_MAX_LIGNES)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        lignes
            .into_iter()
            .map(|l| {
                Ok(ConsultationAlerteDto {
                    id: l.try_get("id").map_err(db)?,
                    acteur_id: l.try_get("acteur_id").map_err(db)?,
                    vue: l.try_get("vue").map_err(db)?,
                    periode_id: l.try_get("periode_id").map_err(db)?,
                    classe_id: l.try_get("classe_id").map_err(db)?,
                    cours_id: l.try_get("cours_id").map_err(db)?,
                    eleve_id: l.try_get("eleve_id").map_err(db)?,
                    le: l.try_get("le").map_err(db)?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s() -> SeuilsAlertes {
        PositionAlertes::Normal.seuils(20_000)
    }

    fn mat(lib: &str, m: Option<i64>) -> MatiereAlerteDto {
        MatiereAlerteDto {
            matiere_id: None,
            libelle: lib.into(),
            moyenne_m: m,
        }
    }

    #[test]
    fn aucune_alerte_sans_fait() {
        assert!(evaluer_eleve(&s(), &FaitsEleve::default()).is_empty());
    }

    #[test]
    fn baisse_de_deux_points_exactement_declenche_et_dit_les_faits() {
        let f = FaitsEleve {
            generale_m: Some(11_000),
            generale_precedente_m: Some(13_000),
            ..Default::default()
        };
        let a = evaluer_eleve(&s(), &f);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].regle, RegleAlerte::BaisseGenerale);
        assert!(
            a[0].faits[0].contains("13") && a[0].faits[0].contains("11"),
            "{:?}",
            a[0].faits
        );
        let f = FaitsEleve {
            generale_m: Some(11_001),
            generale_precedente_m: Some(13_000),
            ..Default::default()
        };
        assert!(
            evaluer_eleve(&s(), &f).is_empty(),
            "1,999 point de baisse : pas d'alerte"
        );
        let f = FaitsEleve {
            generale_m: Some(15_000),
            generale_precedente_m: Some(10_000),
            ..Default::default()
        };
        assert!(
            evaluer_eleve(&s(), &f).is_empty(),
            "une hausse n'alerte jamais"
        );
    }

    #[test]
    fn trois_matieres_strictement_sous_huit() {
        let f = FaitsEleve {
            matieres: vec![
                mat("Maths", Some(7_999)),
                mat("Français", Some(6_000)),
                mat("SVT", Some(8_000)),
                mat("EPS", None),
            ],
            ..Default::default()
        };
        assert!(
            evaluer_eleve(&s(), &f).is_empty(),
            "8 n'est pas sous 8 ; sans moyenne, pas basse"
        );
        let mut f2 = f.clone();
        f2.matieres.push(mat("Histoire", Some(5_500)));
        let a = evaluer_eleve(&s(), &f2);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].regle, RegleAlerte::MatieresSousSeuil);
        assert_eq!(
            a[0].faits.len(),
            4,
            "le constat puis une ligne par matière : {:?}",
            a[0].faits
        );
        // Position stricte : deux matières suffisent.
        assert_eq!(
            evaluer_eleve(&PositionAlertes::Strict.seuils(20_000), &f).len(),
            1
        );
    }

    #[test]
    fn absences_et_non_rendus_cumules() {
        let f = FaitsEleve {
            absences: 2,
            non_rendus: 1,
            ..Default::default()
        };
        let a = evaluer_eleve(&s(), &f);
        assert_eq!(a.len(), 1);
        assert_eq!(
            a[0].faits[0],
            "2 absences et 1 travail non rendu aux évaluations de la période."
        );
        let f = FaitsEleve {
            absences: 3,
            non_rendus: 0,
            ..Default::default()
        };
        assert!(
            evaluer_eleve(&PositionAlertes::Souple.seuils(20_000), &f).is_empty(),
            "souple : 4"
        );
    }

    #[test]
    fn vue_du_cours() {
        assert!(evaluer_cours(&s(), Some(8_000), 0, 2).is_empty());
        let a = evaluer_cours(&s(), Some(7_500), 3, 0);
        assert_eq!(
            a.iter().map(|x| x.regle).collect::<Vec<_>>(),
            vec![
                RegleAlerte::CoursSousSeuil,
                RegleAlerte::AbsencesEvaluations
            ]
        );
        assert!(evaluer_cours(&s(), None, 0, 0).is_empty());
    }

    #[test]
    fn aucun_score_dans_les_textes_et_exclusions_completes() {
        let f = FaitsEleve {
            generale_m: Some(8_000),
            generale_precedente_m: Some(12_000),
            matieres: vec![
                mat("A", Some(1_000)),
                mat("B", Some(2_000)),
                mat("C", Some(3_000)),
            ],
            absences: 5,
            non_rendus: 5,
        };
        for a in evaluer_eleve(&s(), &f) {
            for t in a.faits.iter().chain(std::iter::once(&a.libelle)) {
                let t = t.to_lowercase();
                assert!(
                    !t.contains("risque") && !t.contains("score") && !t.contains("probabilit"),
                    "{t}"
                );
            }
        }
        let codes: Vec<String> = exclusions_ia().into_iter().map(|e| e.code).collect();
        for c in [
            "correction_copies",
            "detection_fraude",
            "formation_groupes",
            "mention_avis_decision",
        ] {
            assert!(codes.iter().any(|x| x == c), "R-V40 : {c}");
        }
    }
}
