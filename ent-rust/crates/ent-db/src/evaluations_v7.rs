//! Socle 21, lot V7 « famille et élève » (`docs/socles/21_EVALUATIONS.md` §10, §17.6, §17.15 ; EVA-159 à EVA-168,
//! EVA-269 à EVA-272 ; livraison : `docs/socles/21_EVALUATIONS_V7_LIVRAISON.md`).
//!
//! * **Qui lit** : la famille (lien « voir les notes », socle 4) et l'élève (son compte), sous la garde de l'élève protégé
//!   du socle 2. Le personnel lit par les grilles (V3) ; ces lectures-ci lui répondent 403, pour qu'aucune vue « famille »
//!   ne serve de raccourci à un droit qu'il n'a pas.
//! * **Rien sans publication** (R-V7) : relevé, explication, statistiques et révisions ne portent que sur des
//!   évaluations publiées ; la moyenne vient du **même moteur** (`moyennes_tx`, publiées seulement) ; une évaluation
//!   retirée après sa première publication est dite « retirée pour vérification » (§10.1).
//! * **Statistiques** par évaluation : `Statistiques::pour_famille` seulement (seuil d'effectif, réglage S-113, médiane).
//! * **Provisoire** : tant que la classe de l'élève n'a pas arrêté ses notes de la période (R-V29, par élève).
//! * **Demande de révision** (EVA-166, CE-59) : un message tracé au professeur du cours ; rien ne change sans son geste ;
//!   une demande ouverte par résultat et par demandeur ; jusqu'au conseil de la période de l'élève (D-069) ; un parent ne
//!   voit que les siennes (R-V46). Événements neutres (`evaluations.revision_demandee`, `evaluations.revision_repondue`),
//!   avis du socle 8 sans note ni texte (`reactions.rs`).
//! * **Simulation** (EVA-270, S-130) : l'élève seulement ; même moteur, aucune écriture.
//! * **Journal des consultations** (R-V26) : chaque relevé lu par la famille ou l'élève laisse une ligne
//!   `eva_consultation` ; elle n'est lue que par la direction (aucun parent ne voit l'activité de l'autre).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use domain_eleve::evaluation_moteur::{
    statistiques_evaluation, Comptage, EntreeCalcul, EvalCalcul, MethodeAnnuelle, MethodeSousMatieres,
    PonderationGenerale, PonderationNotes, ReglesMoteur, StatutCalcul, TraitementBonus, TraitementNonRendu,
};
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use ent_types::evaluations_v7::{
    controler_message_revision, controler_reponse_revision, controler_simulation, libelle_dispense,
    mention_provisoire, AVenirDto, CalendrierPeriodeDto, CoefficientFamilleDto, DemanderRevisionBody,
    DispenseFamilleDto, EvaluationAnnonceeDto, EvaluationRetireeDto, MatiereReleveDto, ProjetEvaluationDto,
    RattrapageFamilleDto, ReleveV7Dto, RepondreRevisionBody, ResultatReleveDto, RevisionDto, RevisionPersonnelDto,
    SimulationBody, SimulationDto, StatsFamilleDto, HORIZON_A_VENIR_JOURS, MAX_REVISIONS_OUVERTES,
    MENTION_RETIREE, MENTION_SIMULATION,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus};
use crate::evaluations_v1::{coefficient_matiere_tx, moyennes_tx, moyennes_tx_avec, regles_tx};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Types d'événements de V7. Charges : identifiants seulement, jamais de note, de message ni de nom.
pub mod types_v7 {
    pub const REVISION_DEMANDEE: &str = "evaluations.revision_demandee";
    pub const REVISION_REPONDUE: &str = "evaluations.revision_repondue";
}

/// Rang d'un état de période à partir duquel une demande de révision n'est plus reçue (conseil tenu, D-069).
const RANG_FIN_REVISION: i32 = 3;
/// Plafonds de lecture (coût borné, jamais de troncature silencieuse : les listes sont triées et bornées).
const MAX_A_VENIR: i64 = 100;
const MAX_REVISIONS_LISTE: i64 = 200;

/// Situation de l'élève dans une période : bornes, classe et état au jour de référence.
struct Situation {
    libelle: String,
    debut: NaiveDate,
    fin: NaiveDate,
    annee_id: Uuid,
    date_conseil: Option<NaiveDate>,
    date_arret_ref: Option<NaiveDate>,
    classe_id: Option<Uuid>,
    etat: Option<String>,
    date_arret_effective: Option<NaiveDate>,
}

async fn situation_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    eleve: Uuid,
    periode: Uuid,
    jour: NaiveDate,
) -> Result<Situation, EntDbError> {
    let p = sqlx::query(
        "SELECT libelle, date_debut, date_fin, annee_id, date_arret_notes, date_conseil
           FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(periode)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let debut: NaiveDate = p.try_get("date_debut").map_err(db)?;
    let fin: NaiveDate = p.try_get("date_fin").map_err(db)?;
    let jour_ref = jour.clamp(debut, fin);
    let mut s = Situation {
        libelle: p.try_get("libelle").map_err(db)?,
        debut,
        fin,
        annee_id: p.try_get("annee_id").map_err(db)?,
        date_conseil: p.try_get("date_conseil").map_err(db)?,
        date_arret_ref: p.try_get("date_arret_notes").map_err(db)?,
        classe_id: None,
        etat: None,
        date_arret_effective: None,
    };
    // La classe de l'élève au jour de référence, sinon à la fin de la période (élève sorti) ; l'état n'est retenu que si
    // la période trouvée est bien celle demandée (périodisation de sa classe).
    for j in [jour_ref, fin] {
        let r: Option<(Uuid, Uuid, String, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT classe_id, periode_id, etat, date_arret_effective FROM eva_periode_eleve_jour($1, $2, $3)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(j)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        if let Some((classe, p_eleve, etat, arret)) = r {
            s.classe_id = Some(classe);
            if p_eleve == periode {
                s.etat = Some(etat);
                s.date_arret_effective = arret;
            }
            break;
        }
    }
    Ok(s)
}

fn rang(etat: &str) -> i32 {
    match etat {
        "ouverte" => 0,
        "notes_arretees" => 1,
        "appreciations_closes" => 2,
        "conseil_tenu" => 3,
        _ => 4,
    }
}

/// Les règles de l'établissement, en phrases pour la famille (EVA-164).
fn phrases_regles(r: &ReglesMoteur, pas_saisie_m: i64) -> Vec<String> {
    let f = domain_eleve::evaluation_moteur::formater_fr;
    let mut v = vec![format!("Les moyennes sont calculées sur {}.", f(r.echelle_m))];
    v.push(match r.ponderation {
        PonderationNotes::NotesRamenees => {
            "Chaque note est d'abord ramenée sur cette échelle, puis pondérée par le coefficient de l'évaluation.".into()
        }
        PonderationNotes::PointsCumules => {
            "Les points obtenus sont additionnés, puis rapportés au total des barèmes, pondérés par les coefficients.".into()
        }
    });
    v.push("Une absence ne vaut jamais zéro : elle ne compte pas dans la moyenne.".into());
    v.push(match r.non_rendu {
        TraitementNonRendu::Exclu => "Un travail non rendu ne compte pas dans la moyenne.".into(),
        TraitementNonRendu::Zero => "Un travail non rendu compte zéro.".into(),
    });
    v.push(match r.bonus {
        TraitementBonus::Ignore => "Les notes bonus ne sont pas prises en compte.".to_string(),
        TraitementBonus::CommeNormale => "Les notes bonus comptent comme des notes ordinaires.".to_string(),
        TraitementBonus::PointsAuDessusDe { seuil_m } => {
            format!("D'une note bonus, seuls les points au-dessus de {} s'ajoutent.", f(seuil_m))
        }
    });
    v.push(match r.sous_matieres {
        MethodeSousMatieres::MoyenneDesMoyennes => {
            "Dans une matière à plusieurs parties, la moyenne de la matière est la moyenne pondérée des parties.".into()
        }
        MethodeSousMatieres::ToutesLesNotes => {
            "Dans une matière à plusieurs parties, toutes les notes comptent ensemble.".into()
        }
    });
    v.push(match r.generale {
        PonderationGenerale::Ponderee => "La moyenne générale pondère chaque matière par son coefficient.".into(),
        PonderationGenerale::NonPonderee => "La moyenne générale donne le même poids à chaque matière.".into(),
    });
    v.push(match r.annuelle {
        MethodeAnnuelle::MoyenneDesPeriodes => "La moyenne de l'année est la moyenne des périodes.".into(),
        MethodeAnnuelle::ToutesLesEvaluations => "La moyenne de l'année reprend toutes les évaluations de l'année.".into(),
    });
    v.push(format!(
        "Les moyennes sont affichées au {} près ; les notes se saisissent au {} près.",
        f(r.affichage.precision.pas_milliemes()),
        f(pas_saisie_m)
    ));
    if r.stats_familles {
        v.push(format!(
            "La moyenne de la classe, la note la plus basse et la plus haute sont montrées à partir de {} élèves notés{}.",
            r.seuil_effectif_stats.max(domain_eleve::evaluation_moteur::SEUIL_STATS_PLANCHER),
            if r.mediane_familles { ", avec la médiane" } else { "" }
        ));
    } else {
        v.push("L'établissement ne montre pas de statistiques de classe aux familles.".into());
    }
    v
}

fn heure_fr(h: Option<NaiveTime>) -> Option<String> {
    h.map(|h| h.format("%H:%M").to_string())
}

impl PgEnt {
    /// Famille ou élève seulement, avec le droit de lire les notes de cet élève (lien « voir les notes » ou compte de
    /// l'élève) et la garde de l'élève protégé. Le personnel reçoit 403 (il lit par les grilles).
    async fn eva_v7_acces_famille(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<Role, EntDbError> {
        let r = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !matches!(r, Role::Parent | Role::Eleve) {
            return Err(EntDbError::Forbidden);
        }
        if !self.actor_can_read_notes(etab, acteur, role, eleve).await? {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        Ok(r)
    }

    // ───────────────────────── Relevé (EVA-159 à EVA-161, EVA-163, EVA-269) ─────────────────────────

    /// Relevé d'une période : résultats publiés, moyennes provisoires et leur explication, statistiques autorisées,
    /// évaluations retirées pour vérification, dates prévues. Laisse une trace de consultation (R-V26).
    pub async fn eva_v7_releve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        periode: Uuid,
        jour: NaiveDate,
    ) -> Result<ReleveV7Dto, EntDbError> {
        self.eva_v7_acces_famille(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = situation_tx(&mut tx, etab, eleve, periode, jour).await?;
        let reglage = regles_tx(&mut tx, etab, s.annee_id).await?;
        let regles = reglage.regles;
        let calc = moyennes_tx(&mut tx, etab, eleve, periode, true).await?;

        // Résultats publiés de la période (même sélection que le moteur : une évaluation une fois, la fiche conservée
        // d'abord). Ni la note privée, ni le motif, ni l'auteur.
        let lignes = sqlx::query(
            "SELECT q.* FROM (
               SELECT DISTINCT ON (r.evaluation_id)
                    r.evaluation_id, ev.titre, ev.date_eval, r.statut, (r.valeur * 1000)::bigint AS valeur_m,
                    (ev.bareme * 1000)::bigint AS bareme_m, (ev.coef * 1000)::bigint AS coef_m, r.commentaire_eleve,
                    ev.etat AS etat_evaluation, ev.hors_bareme_autorise, co.matiere_id,
                    COALESCE((SELECT eva_etat_rang(pe.etat) FROM eva_periode_eleve_jour($1, $2, ev.date_eval) pe), 4)
                        AS rang_periode
               FROM eva_resultat r
               JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
               JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
              WHERE r.etablissement_id = $1 AND r.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
                AND ev.mode = 'note' AND ev.etat IN ('ouverte', 'annulee') AND ev.publiee_le IS NOT NULL
                AND eva_periode_dans(ev.etablissement_id, $3, ev.date_eval)
              ORDER BY r.evaluation_id, (r.eleve_id = $2) DESC, r.saisi_le DESC, r.id
             ) q
             ORDER BY q.date_eval, q.titre, q.evaluation_id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(periode)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = lignes
            .iter()
            .map(|l| l.try_get("evaluation_id"))
            .collect::<Result<_, _>>()
            .map_err(db)?;

        // Statistiques par évaluation : toutes les cases de l'évaluation publiée, passées au moteur, puis au seul
        // filtre famille (`pour_famille`). Rien n'est calculé pour une évaluation annulée.
        let mut series: HashMap<Uuid, Vec<(StatutCalcul, Option<i64>)>> = HashMap::new();
        if !ids.is_empty() && regles.stats_familles {
            let toutes = sqlx::query(
                "SELECT evaluation_id, statut, (valeur * 1000)::bigint AS valeur_m FROM eva_resultat
                  WHERE etablissement_id = $1 AND evaluation_id = ANY($2)",
            )
            .bind(etab)
            .bind(&ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for r in &toutes {
                let st: String = r.try_get("statut").map_err(db)?;
                if let Some(st) = StatutCalcul::parse(&st) {
                    series
                        .entry(r.try_get("evaluation_id").map_err(db)?)
                        .or_default()
                        .push((st, r.try_get("valeur_m").map_err(db)?));
                }
            }
        }

        let mut par_matiere: BTreeMap<Uuid, Vec<ResultatReleveDto>> = BTreeMap::new();
        for l in &lignes {
            let evaluation_id: Uuid = l.try_get("evaluation_id").map_err(db)?;
            let statut: String = l.try_get("statut").map_err(db)?;
            let st = StatutCalcul::parse(&statut);
            let bareme_m: i64 = l.try_get("bareme_m").map_err(db)?;
            let annulee = l.try_get::<String, _>("etat_evaluation").map_err(db)? == "annulee";
            let stats = if annulee {
                None
            } else {
                let ev = EvalCalcul {
                    id: evaluation_id,
                    titre: String::new(),
                    bareme_m,
                    coef_m: 1000,
                    comptage: Comptage::Normal,
                    hors_bareme_autorise: l.try_get("hors_bareme_autorise").map_err(db)?,
                    sous_matiere_id: None,
                    cours_id: None,
                    annulee: false,
                };
                series
                    .get(&evaluation_id)
                    .and_then(|serie| statistiques_evaluation(&ev, serie, &regles).ok().flatten())
                    .and_then(|x| x.pour_famille(&regles))
                    .and_then(|x| {
                        Some(StatsFamilleDto {
                            effectif: x.effectif(),
                            moyenne_m: regles.affichage.en_milliemes(x.moyenne()).ok()?,
                            min_m: regles.affichage.en_milliemes(x.min()).ok()?,
                            max_m: regles.affichage.en_milliemes(x.max()).ok()?,
                            mediane_m: match x.mediane() {
                                Some(m) => Some(regles.affichage.en_milliemes(m).ok()?),
                                None => None,
                            },
                        })
                    })
            };
            let matiere: Option<Uuid> = l.try_get("matiere_id").map_err(db)?;
            let rang_periode: i32 = l.try_get("rang_periode").map_err(db)?;
            par_matiere
                .entry(matiere.unwrap_or(Uuid::nil()))
                .or_default()
                .push(ResultatReleveDto {
                    evaluation_id,
                    titre: l.try_get("titre").map_err(db)?,
                    date_eval: l.try_get("date_eval").map_err(db)?,
                    abreviation: st.map(|x| x.abreviation()).unwrap_or("").to_string(),
                    valeur_m: if statut == "note" { l.try_get("valeur_m").map_err(db)? } else { None },
                    statut,
                    bareme_m,
                    coef_m: l.try_get("coef_m").map_err(db)?,
                    commentaire: l.try_get("commentaire_eleve").map_err(db)?,
                    stats,
                    revision_possible: !annulee && rang_periode < RANG_FIN_REVISION,
                });
        }

        // Libellés des matières (référentiel de l'établissement).
        let mut ids_matieres: BTreeSet<Uuid> = calc.matieres.iter().map(|m| m.matiere_id).collect();
        ids_matieres.extend(par_matiere.keys().copied());
        let ids_matieres: Vec<Uuid> = ids_matieres.into_iter().filter(|m| !m.is_nil()).collect();
        let libelles: HashMap<Uuid, String> = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, libelle FROM ent_matiere WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etab)
        .bind(&ids_matieres)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let libelle = |m: Uuid| {
            libelles
                .get(&m)
                .cloned()
                .unwrap_or_else(|| "Autre enseignement".to_string())
        };

        let mut matieres: Vec<MatiereReleveDto> = Vec::new();
        for m in &calc.matieres {
            matieres.push(MatiereReleveDto {
                matiere_id: (!m.matiere_id.is_nil()).then_some(m.matiere_id),
                matiere_libelle: libelle(m.matiere_id),
                moyenne_m: m.moyenne.affichee_m,
                coef_m: m.coef_m,
                hors_moyenne_generale: m.hors_moyenne_generale,
                explication: m.moyenne.explication_fr(&calc.regles),
                resultats: par_matiere.remove(&m.matiere_id).unwrap_or_default(),
            });
        }
        // Une matière qui n'a que des résultats sans effet (le moteur l'ignore) reste montrée, sans moyenne.
        for (m, resultats) in par_matiere {
            matieres.push(MatiereReleveDto {
                matiere_id: (!m.is_nil()).then_some(m),
                matiere_libelle: libelle(m),
                moyenne_m: None,
                coef_m: 0,
                hors_moyenne_generale: true,
                explication: Vec::new(),
                resultats,
            });
        }
        matieres.sort_by_key(|m| m.matiere_libelle.to_lowercase());

        // Retirées après publication (§10.1) : l'élève y avait un résultat, l'évaluation n'est plus publiée.
        let retirees = sqlx::query(
            "SELECT ev.id, ev.titre, ev.date_eval, COALESCE(m.libelle, 'Autre enseignement') AS matiere_libelle
               FROM eva_evaluation ev
               JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
               LEFT JOIN ent_matiere m ON m.id = co.matiere_id AND m.etablissement_id = co.etablissement_id
              WHERE ev.etablissement_id = $1 AND ev.etat = 'ouverte' AND ev.mode = 'note'
                AND ev.publiee_le IS NULL AND ev.retiree_le IS NOT NULL AND ev.premiere_publiee_le IS NOT NULL
                AND eva_periode_dans(ev.etablissement_id, $3, ev.date_eval)
                AND EXISTS (SELECT 1 FROM eva_resultat r
                             WHERE r.etablissement_id = ev.etablissement_id AND r.evaluation_id = ev.id
                               AND r.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2)))
              ORDER BY ev.date_eval, ev.titre, ev.id",
        )
        .bind(etab)
        .bind(eleve)
        .bind(periode)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .iter()
        .map(|r| {
            Ok(EvaluationRetireeDto {
                evaluation_id: r.try_get("id")?,
                titre: r.try_get("titre")?,
                date_eval: r.try_get("date_eval")?,
                matiere_libelle: r.try_get("matiere_libelle")?,
                mention: MENTION_RETIREE.to_string(),
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(db)?;

        // Journal des consultations (R-V26) : lu par la direction seulement.
        sqlx::query(
            "INSERT INTO eva_consultation (id, etablissement_id, eleve_id, objet, consulte_par, role)
             VALUES ($1, $2, $3, 'releve', $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(eleve)
        .bind(acteur)
        .bind(role)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;

        let date_arret = s.date_arret_effective.or(s.date_arret_ref);
        let provisoire = s.etat.as_deref().is_none_or(|e| rang(e) == 0);
        Ok(ReleveV7Dto {
            eleve_id: eleve,
            periode_id: periode,
            periode_libelle: s.libelle,
            echelle_m: regles.echelle_m,
            etat_periode: s.etat,
            provisoire,
            mention_provisoire: provisoire.then(|| mention_provisoire(date_arret)),
            calendrier: CalendrierPeriodeDto {
                date_debut: s.debut,
                date_fin: s.fin,
                date_arret,
                date_conseil: s.date_conseil,
            },
            matieres,
            moyenne_generale_m: calc.generale.affichee_m,
            explication_generale: calc.generale.explication_fr(&calc.regles),
            retirees,
        })
    }

    // ───────────────────────── À venir (EVA-162, EVA-163) ─────────────────────────

    /// Évaluations annoncées et non publiées des cours de l'élève (sans note), rattrapages convoqués, dispenses en
    /// vigueur (nature seulement).
    pub async fn eva_v7_a_venir_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        jour: NaiveDate,
    ) -> Result<AVenirDto, EntDbError> {
        self.eva_v7_acces_famille(etab, acteur, role, eleve).await?;
        let horizon = jour + Duration::days(HORIZON_A_VENIR_JOURS);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let evaluations = sqlx::query(
            "SELECT ev.id, ev.titre, ev.date_eval, COALESCE(m.libelle, 'Autre enseignement') AS matiere_libelle,
                    (ev.bareme * 1000)::bigint AS bareme_m, (ev.coef * 1000)::bigint AS coef_m
               FROM eva_evaluation ev
               JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
               LEFT JOIN ent_matiere m ON m.id = co.matiere_id AND m.etablissement_id = co.etablissement_id
              WHERE ev.etablissement_id = $1 AND ev.etat = 'ouverte' AND ev.annoncee AND ev.publiee_le IS NULL
                AND ev.date_eval BETWEEN $3 AND $4
                AND EXISTS (SELECT 1 FROM ent_public_cours_jour($1, ev.date_eval, ev.cours_id, NULL, $2) pu)
                AND (NOT EXISTS (SELECT 1 FROM eva_evaluation_restriction rr
                                  WHERE rr.etablissement_id = ev.etablissement_id AND rr.evaluation_id = ev.id)
                     OR EXISTS (SELECT 1 FROM eva_evaluation_restriction rr
                                 WHERE rr.etablissement_id = ev.etablissement_id AND rr.evaluation_id = ev.id
                                   AND rr.eleve_id = $2))
              ORDER BY ev.date_eval, ev.titre, ev.id
              LIMIT $5",
        )
        .bind(etab)
        .bind(eleve)
        .bind(jour)
        .bind(horizon)
        .bind(MAX_A_VENIR)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .iter()
        .map(|r| {
            Ok(EvaluationAnnonceeDto {
                evaluation_id: r.try_get("id")?,
                titre: r.try_get("titre")?,
                date_eval: r.try_get("date_eval")?,
                matiere_libelle: r.try_get("matiere_libelle")?,
                bareme_m: r.try_get("bareme_m")?,
                coef_m: r.try_get("coef_m")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(db)?;
        let rattrapages = sqlx::query(
            "SELECT ra.id, ev.titre, COALESCE(m.libelle, 'Autre enseignement') AS matiere_libelle, ra.nature, ra.prevu_le,
                    ra.heure, ra.lieu, ra.modalite, ra.echeance
               FROM eva_rattrapage ra
               JOIN eva_evaluation ev ON ev.id = ra.evaluation_id AND ev.etablissement_id = ra.etablissement_id
               JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
               LEFT JOIN ent_matiere m ON m.id = co.matiere_id AND m.etablissement_id = co.etablissement_id
              WHERE ra.etablissement_id = $1 AND ra.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
                AND ra.etat = 'convoque' AND ra.echeance >= $3 AND ev.etat = 'ouverte'
              ORDER BY ra.prevu_le, ra.heure NULLS LAST, ra.id
              LIMIT $4",
        )
        .bind(etab)
        .bind(eleve)
        .bind(jour)
        .bind(MAX_A_VENIR)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .iter()
        .map(|r| {
            Ok(RattrapageFamilleDto {
                rattrapage_id: r.try_get("id")?,
                evaluation_titre: r.try_get("titre")?,
                matiere_libelle: r.try_get("matiere_libelle")?,
                nature: r.try_get("nature")?,
                prevu_le: r.try_get("prevu_le")?,
                heure: heure_fr(r.try_get("heure")?),
                lieu: r.try_get("lieu")?,
                modalite: r.try_get("modalite")?,
                echeance: r.try_get("echeance")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(db)?;
        let dispenses = sqlx::query(
            "SELECT COALESCE(m.libelle, 'tous les enseignements') AS matiere_libelle, d.nature, d.du, d.au
               FROM eva_dispense_port d
               LEFT JOIN ent_matiere m ON m.id = d.matiere_id AND m.etablissement_id = d.etablissement_id
              WHERE d.etablissement_id = $1 AND d.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
                AND d.retiree_le IS NULL AND (d.au IS NULL OR d.au >= $3)
              ORDER BY d.du, d.id
              LIMIT $4",
        )
        .bind(etab)
        .bind(eleve)
        .bind(jour)
        .bind(MAX_A_VENIR)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .iter()
        .map(|r| {
            let nature: String = r.try_get("nature")?;
            Ok(DispenseFamilleDto {
                matiere_libelle: r.try_get("matiere_libelle")?,
                libelle: libelle_dispense(&nature).to_string(),
                nature,
                du: r.try_get("du")?,
                au: r.try_get("au")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(AVenirDto {
            eleve_id: eleve,
            jour,
            evaluations,
            rattrapages,
            dispenses,
        })
    }

    // ───────────────────────── Projet d'évaluation (EVA-164) ─────────────────────────

    /// Matières suivies par l'élève dans la période (services de sa classe et de ses groupes), au jour de référence.
    async fn eva_v7_matieres_suivies_tx(
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        eleve: Uuid,
        s: &Situation,
    ) -> Result<Vec<(Uuid, String)>, EntDbError> {
        sqlx::query_as(
            "SELECT DISTINCT m.id, m.libelle
               FROM ent_service_enseignement se
               JOIN ent_matiere m ON m.id = se.matiere_id AND m.etablissement_id = se.etablissement_id
              WHERE se.etablissement_id = $1 AND se.annee_id = $2
                AND (se.classe_id = $3
                     OR se.groupe_id IN (SELECT a.groupe_id FROM ent_affectation a
                                          WHERE a.etablissement_id = $1 AND a.eleve_id = $4 AND a.groupe_id IS NOT NULL
                                            AND a.date_debut <= $6 AND (a.date_fin IS NULL OR a.date_fin >= $5)))
              ORDER BY m.libelle, m.id",
        )
        .bind(etab)
        .bind(s.annee_id)
        .bind(s.classe_id)
        .bind(eleve)
        .bind(s.debut)
        .bind(s.fin)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)
    }

    /// Le projet d'évaluation : règles de calcul en phrases et coefficients des matières de l'élève.
    pub async fn eva_v7_projet_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        periode: Uuid,
        jour: NaiveDate,
    ) -> Result<ProjetEvaluationDto, EntDbError> {
        self.eva_v7_acces_famille(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = situation_tx(&mut tx, etab, eleve, periode, jour).await?;
        let reglage = regles_tx(&mut tx, etab, s.annee_id).await?;
        let mut coefficients = Vec::new();
        for (matiere_id, matiere_libelle) in Self::eva_v7_matieres_suivies_tx(&mut tx, etab, eleve, &s).await? {
            let c = coefficient_matiere_tx(&mut tx, etab, s.annee_id, matiere_id, s.classe_id, s.fin).await?;
            coefficients.push(CoefficientFamilleDto {
                matiere_id,
                matiere_libelle,
                coef_m: c.map(|x| x.0),
                hors_moyenne_generale: c.is_some_and(|x| x.1),
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(ProjetEvaluationDto {
            eleve_id: eleve,
            periode_id: periode,
            echelle_m: reglage.regles.echelle_m,
            regles: phrases_regles(&reglage.regles, reglage.pas_saisie_m),
            coefficients,
        })
    }

    // ───────────────────────── Simulation (EVA-270) ─────────────────────────

    /// « Si j'ai 12 au prochain devoir » : l'élève seulement, sur ses moyennes publiées ; rien n'est enregistré.
    pub async fn eva_v7_simuler_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
        jour: NaiveDate,
        b: &SimulationBody,
    ) -> Result<SimulationDto, EntDbError> {
        if self.eva_v7_acces_famille(etab, acteur, role, eleve).await? != Role::Eleve {
            return Err(EntDbError::Forbidden);
        }
        let (valeur_m, bareme_m, coef_m) = controler_simulation(b).map_err(refus)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = situation_tx(&mut tx, etab, eleve, b.periode_id, jour).await?;
        let avant = moyennes_tx(&mut tx, etab, eleve, b.periode_id, true).await?;
        let suivie = avant.matieres.iter().any(|m| m.matiere_id == b.matiere_id)
            || Self::eva_v7_matieres_suivies_tx(&mut tx, etab, eleve, &s)
                .await?
                .iter()
                .any(|(m, _)| *m == b.matiere_id);
        if !suivie {
            return Err(refus("matière non suivie par l'élève dans cette période"));
        }
        let imaginee = EntreeCalcul {
            evaluation: EvalCalcul {
                id: Uuid::new_v4(),
                titre: "Note imaginée".to_string(),
                bareme_m,
                coef_m,
                comptage: Comptage::Normal,
                hors_bareme_autorise: false,
                sous_matiere_id: None,
                cours_id: None,
                annulee: false,
            },
            statut: StatutCalcul::Note,
            valeur_m: Some(valeur_m),
        };
        let apres =
            moyennes_tx_avec(&mut tx, etab, eleve, b.periode_id, true, Some((b.matiere_id, imaginee))).await?;
        // Lecture seule : rien n'a été écrit, la transaction se termine sans effet.
        tx.rollback().await.map_err(db)?;
        let de = |m: &crate::evaluations_v1::MoyennesEleve| {
            m.matieres
                .iter()
                .find(|x| x.matiere_id == b.matiere_id)
                .and_then(|x| x.moyenne.affichee_m)
        };
        Ok(SimulationDto {
            matiere_id: b.matiere_id,
            matiere_avant_m: de(&avant),
            matiere_apres_m: de(&apres),
            generale_avant_m: avant.generale.affichee_m,
            generale_apres_m: apres.generale.affichee_m,
            mention: MENTION_SIMULATION.to_string(),
        })
    }

    // ───────────────────────── Demandes de révision (EVA-166, CE-59) ─────────────────────────

    /// Demander la révision d'un résultat publié. Une demande ouverte par résultat et par demandeur ; jusqu'au conseil de
    /// la période de l'élève à la date de l'évaluation ; au plus 20 demandes ouvertes par demandeur.
    pub async fn eva_v7_demander_revision_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        b: &DemanderRevisionBody,
    ) -> Result<RevisionDto, EntDbError> {
        let r = self.eva_v7_acces_famille(etab, acteur, role, b.eleve_id).await?;
        let message = controler_message_revision(&b.message).map_err(refus)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Le résultat de l'élève (fiche conservée d'abord) à une évaluation PUBLIÉE et ouverte ; sinon « introuvable »
        // (une évaluation non publiée n'existe pas pour la famille : aucun oracle).
        let cible = sqlx::query(
            "SELECT r.id AS resultat_id, ev.titre, ev.cours_id, COALESCE(m.libelle, 'Autre enseignement') AS matiere_libelle,
                    COALESCE((SELECT eva_etat_rang(pe.etat) FROM eva_periode_eleve_jour($1, $2, ev.date_eval) pe), 4)
                        AS rang_periode
               FROM eva_resultat r
               JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
               JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
               LEFT JOIN ent_matiere m ON m.id = co.matiere_id AND m.etablissement_id = co.etablissement_id
              WHERE r.etablissement_id = $1 AND r.evaluation_id = $3
                AND r.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
                AND ev.mode = 'note' AND ev.etat = 'ouverte' AND ev.publiee_le IS NOT NULL
              ORDER BY (r.eleve_id = $2) DESC, r.saisi_le DESC, r.id
              LIMIT 1",
        )
        .bind(etab)
        .bind(b.eleve_id)
        .bind(b.evaluation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let resultat_id: Uuid = cible.try_get("resultat_id").map_err(db)?;
        let rang_periode: i32 = cible.try_get("rang_periode").map_err(db)?;
        if rang_periode >= RANG_FIN_REVISION {
            return Err(conflit(
                "le conseil de classe de la période s'est tenu : adressez-vous à la direction de l'établissement",
            ));
        }
        // Verrou par demandeur : le plafond et l'unicité se vérifient sans course entre deux onglets.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('eva_revision:' || $1::text, 0))")
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let (deja, ouvertes): (bool, i64) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM eva_revision_demande
                             WHERE etablissement_id = $1 AND resultat_id = $2 AND demandeur = $3 AND etat = 'ouverte'),
                    (SELECT count(*) FROM eva_revision_demande
                      WHERE etablissement_id = $1 AND demandeur = $3 AND etat = 'ouverte')",
        )
        .bind(etab)
        .bind(resultat_id)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit("une demande est déjà en cours pour ce résultat : attendez la réponse ou retirez-la"));
        }
        if ouvertes >= MAX_REVISIONS_OUVERTES {
            return Err(conflit("trop de demandes en cours : attendez les réponses avant d'en écrire d'autres"));
        }
        let id = Uuid::new_v4();
        let cree_le: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO eva_revision_demande (id, etablissement_id, resultat_id, demandeur, demandeur_role, message)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING cree_le",
        )
        .bind(id)
        .bind(etab)
        .bind(resultat_id)
        .bind(acteur)
        .bind(if r == Role::Eleve { "eleve" } else { "parent" })
        .bind(&message)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_v7::REVISION_DEMANDEE.to_string(),
            aggregate: AggregateKind::Devoir,
            aggregate_id: b.evaluation_id,
            // Neutre : ni élève, ni note, ni message ; l'avis relit la demande.
            eleve_id: None,
            payload: json!({ "evaluation_id": b.evaluation_id, "revision_id": id }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(RevisionDto {
            id,
            eleve_id: b.eleve_id,
            evaluation_id: b.evaluation_id,
            evaluation_titre: cible.try_get("titre").map_err(db)?,
            matiere_libelle: cible.try_get("matiere_libelle").map_err(db)?,
            message,
            etat: "ouverte".into(),
            cree_le,
            reponse: None,
            repondue_le: None,
        })
    }

    /// Les demandes de l'acteur pour cet élève (jamais celles de l'autre parent, R-V46).
    pub async fn eva_v7_mes_revisions_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<Vec<RevisionDto>, EntDbError> {
        self.eva_v7_acces_famille(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT d.id, r.evaluation_id, ev.titre, COALESCE(m.libelle, 'Autre enseignement') AS matiere_libelle,
                    d.message, d.etat, d.cree_le, d.reponse, d.repondue_le
               FROM eva_revision_demande d
               JOIN eva_resultat r ON r.id = d.resultat_id AND r.etablissement_id = d.etablissement_id
               JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
               JOIN ent_cours co ON co.id = ev.cours_id AND co.etablissement_id = ev.etablissement_id
               LEFT JOIN ent_matiere m ON m.id = co.matiere_id AND m.etablissement_id = co.etablissement_id
              WHERE d.etablissement_id = $1 AND d.demandeur = $3
                AND r.eleve_id IN (SELECT eva_fiches_de_l_eleve($1, $2))
              ORDER BY d.cree_le DESC, d.id
              LIMIT $4",
        )
        .bind(etab)
        .bind(eleve)
        .bind(acteur)
        .bind(MAX_REVISIONS_LISTE)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        rows.iter()
            .map(|r| {
                Ok(RevisionDto {
                    id: r.try_get("id")?,
                    eleve_id: eleve,
                    evaluation_id: r.try_get("evaluation_id")?,
                    evaluation_titre: r.try_get("titre")?,
                    matiere_libelle: r.try_get("matiere_libelle")?,
                    message: r.try_get("message")?,
                    etat: r.try_get("etat")?,
                    cree_le: r.try_get("cree_le")?,
                    reponse: r.try_get("reponse")?,
                    repondue_le: r.try_get("repondue_le")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)
    }

    /// Retirer sa demande tant qu'elle est ouverte. Une demande d'un autre est « introuvable ».
    pub async fn eva_v7_retirer_revision_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        revision_id: Uuid,
    ) -> Result<(), EntDbError> {
        if !matches!(Role::parse(role), Some(Role::Parent | Role::Eleve)) {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let etat: Option<String> = sqlx::query_scalar(
            "SELECT etat FROM eva_revision_demande WHERE etablissement_id = $1 AND id = $2 AND demandeur = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(revision_id)
        .bind(acteur)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        match etat.as_deref() {
            None => return Err(introuvable()),
            Some("ouverte") => {}
            Some(_) => return Err(conflit("demande déjà close")),
        }
        sqlx::query(
            "UPDATE eva_revision_demande SET etat = 'retiree', retiree_le = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(revision_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Les demandes à traiter par le personnel : celles des cours de l'acteur (professeur) ou toutes (direction,
    /// `notes.corriger`). `ouvertes_seulement` par défaut.
    pub async fn eva_v7_revisions_personnel_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        ouvertes_seulement: bool,
    ) -> Result<Vec<RevisionPersonnelDto>, EntDbError> {
        Self::exiger_staff(role)?;
        let corriger = self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?;
        self.eva_v7_revisions_personnel(etab, acteur, corriger, jour, ouvertes_seulement, None)
            .await
    }

    async fn eva_v7_revisions_personnel(
        &self,
        etab: Uuid,
        acteur: Uuid,
        corriger: bool,
        jour: NaiveDate,
        ouvertes_seulement: bool,
        une: Option<Uuid>,
    ) -> Result<Vec<RevisionPersonnelDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT d.id, r.eleve_id, el.nom, el.prenom, r.evaluation_id, ev.titre, ev.cours_id, d.demandeur_role,
                    d.message, d.etat, d.cree_le, d.reponse, d.repondue_le
               FROM eva_revision_demande d
               JOIN eva_resultat r ON r.id = d.resultat_id AND r.etablissement_id = d.etablissement_id
               JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
               JOIN ent_eleve el ON el.id = r.eleve_id AND el.etablissement_id = r.etablissement_id
              WHERE d.etablissement_id = $1 AND ($4 = false OR d.etat = 'ouverte')
                AND ($7::uuid IS NULL OR d.id = $7)
                AND ($3 OR ent_prof_sur_cours($1, ev.cours_id, $2, $5, false))
              ORDER BY d.cree_le DESC, d.id
              LIMIT $6",
        )
        .bind(etab)
        .bind(acteur)
        .bind(corriger)
        .bind(ouvertes_seulement)
        .bind(jour)
        .bind(MAX_REVISIONS_LISTE)
        .bind(une)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        rows.iter()
            .map(|r| {
                Ok(RevisionPersonnelDto {
                    id: r.try_get("id")?,
                    eleve_id: r.try_get("eleve_id")?,
                    eleve_nom: r.try_get("nom")?,
                    eleve_prenom: r.try_get("prenom")?,
                    evaluation_id: r.try_get("evaluation_id")?,
                    evaluation_titre: r.try_get("titre")?,
                    cours_id: r.try_get("cours_id")?,
                    demandeur_role: r.try_get("demandeur_role")?,
                    message: r.try_get("message")?,
                    etat: r.try_get("etat")?,
                    cree_le: r.try_get("cree_le")?,
                    reponse: r.try_get("reponse")?,
                    repondue_le: r.try_get("repondue_le")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db)
    }

    /// Répondre à une demande : un professeur du cours de l'évaluation, ou la direction (`notes.corriger`). La réponse
    /// ne change pas la note (le professeur corrige par la grille, la direction après l'arrêt, CE-59).
    pub async fn eva_v7_repondre_revision_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        revision_id: Uuid,
        b: &RepondreRevisionBody,
    ) -> Result<RevisionPersonnelDto, EntDbError> {
        Self::exiger_staff(role)?;
        let etab_lecture = self
            .a_action_etablissement(etab, acteur, role, Action::NotesLire)
            .await?;
        let corriger = self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?;
        let reponse = controler_reponse_revision(&b.reponse).map_err(refus)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = sqlx::query(
            "SELECT d.etat, r.eleve_id, ev.id AS evaluation_id, ev.cours_id
               FROM eva_revision_demande d
               JOIN eva_resultat r ON r.id = d.resultat_id AND r.etablissement_id = d.etablissement_id
               JOIN eva_evaluation ev ON ev.id = r.evaluation_id AND ev.etablissement_id = r.etablissement_id
              WHERE d.etablissement_id = $1 AND d.id = $2",
        )
        .bind(etab)
        .bind(revision_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        // Ordre des contrôles (règle 5) : une demande inconnue répond comme un droit refusé à qui n'a pas `notes.corriger`.
        let Some(d) = d else {
            return Err(if corriger { introuvable() } else { EntDbError::Forbidden });
        };
        let cours_id: Uuid = d.try_get("cours_id").map_err(db)?;
        let evaluation_id: Uuid = d.try_get("evaluation_id").map_err(db)?;
        let eleve_id: Uuid = d.try_get("eleve_id").map_err(db)?;
        let acces = Self::eva_v3_acces_tx(&mut tx, etab, acteur, cours_id, jour, etab_lecture, corriger).await?;
        if !(acces.prof_du_cours || corriger) {
            return Err(EntDbError::Forbidden);
        }
        tx.commit().await.map_err(db)?;
        self.exiger_eleve_visible(etab, acteur, role, eleve_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let etat: String = sqlx::query_scalar(
            "SELECT etat FROM eva_revision_demande WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(revision_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if etat != "ouverte" {
            return Err(conflit("demande déjà close : répondue ou retirée"));
        }
        sqlx::query(
            "UPDATE eva_revision_demande SET etat = 'repondue', reponse = $3, repondue_par = $4, repondue_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(revision_id)
        .bind(&reponse)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types_v7::REVISION_REPONDUE.to_string(),
            aggregate: AggregateKind::Devoir,
            aggregate_id: evaluation_id,
            eleve_id: None,
            payload: json!({ "evaluation_id": evaluation_id, "revision_id": revision_id }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        tx.commit().await.map_err(db)?;
        self.eva_v7_revisions_personnel(etab, acteur, true, jour, false, Some(revision_id))
            .await?
            .into_iter()
            .next()
            .ok_or_else(introuvable)
    }
}
