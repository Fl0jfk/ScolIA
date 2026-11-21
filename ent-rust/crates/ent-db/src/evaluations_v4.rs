//! Socle 21, lot V4 « absences, dispenses, aménagements, rattrapages » : ports 12, 34, 35, propositions d'absents,
//! rattrapages, évaluation de remplacement et « seul 0 autorisé ».
//!
//! * **Ports** (`21_EVALUATIONS.md` §4.3) : 12 (absence à la séance : *lecture* de l'assiduité), 34 (dispenses : table de
//!   **simulation** `eva_dispense_port`, sans motif), 35 (aménagements : *lecture* de `ent_amenagement`, codes du catalogue
//!   fermé) et indisponibilité d'un AESH (simulation). Un seul point de lecture par port, dans la base
//!   (`eva_absence_a_la_seance`, `eva_dispense_a_la_date`, `eva_amenage_a_la_date`).
//! * **L'appel propose, le professeur confirme** : les propositions sont **recalculées** par le serveur à chaque lecture
//!   et à chaque confirmation (un statut ne vient jamais du client) ; une case déjà remplie n'est jamais écrasée.
//! * **L'absence ne vaut jamais 0** (R-V5, règle 2) : le 0 n'existe que pour une convocation de **remplacement**, restée
//!   « absent non justifié », par un acte explicite portant la confirmation de la référence réglementaire : le
//!   **professeur du cours** tant que la période de l'élève est ouverte (avec un motif, l'absence non justifiée étant
//!   constatée par l'assiduité), la **direction** (`notes.corriger`) après l'arrêt ; l'assiduité est relue au moment du
//!   zéro (V3d, M-4, M-5). La base le garantit aussi (`eva_rattrapage_zero_chk`).
//! * **Ordre des contrôles** (règle 5, V3d F-2) : personnel, puis droit sur le cours, puis visibilité de l'élève, puis
//!   existence et état : rien n'est révélé avant le droit.
//! * **Aucune écriture sur un état périmé** (règle 4, V3d M-1) : la case est relue sous verrou, remplaçable seulement si
//!   elle est vide ou « absent », « à rattraper », « non rendu », « non noté » ; sinon conflit.
//! * **Tout ce qui change une note est attribué et historisé** : toute écriture passe par `ecrire_resultat_tx` (versions,
//!   période par élève, historique, événement neutre).
//! * **Rien pour la famille ni l'élève** : ces lectures et écritures sont réservées au personnel ; les événements sont
//!   **neutres** (jamais d'`eleve_id`, de statut, de note ni de motif).

use std::collections::{BTreeSet, HashSet};

use chrono::{NaiveDate, NaiveTime};
use domain_eleve::evaluation_moteur::StatutCalcul;
use domain_eleve::DomainError;
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use ent_types::evaluations_v3::{analyser_cellule, Analyse, ContrainteCellule};
use ent_types::evaluations_v4::{
    AbsenceRepeteeDto, AmenagementCodeDto, AmenagementsEvaluationDto, AnnulerRattrapageBody,
    AppliquerPropositionsBody, AttribuerZeroBody, DeclarerDispenseBody, DeplacerRattrapageBody,
    DispenseDto, EleveAmenageDto, IncoherenceAppelDto, NoterRattrapageBody,
    PlanifierRattrapageBody, PropositionAppliqueeDto, PropositionDto, PropositionsDto,
    RattrapageDto, ReconvoquerBody, ZeroAttribueDto, MODALITES_RATTRAPAGE, NATURES_DISPENSE,
    REFERENCE_ZERO, SEUIL_ABSENCES_REPETEES,
};
use serde_json::json;
use sqlx::postgres::PgRow;
use sqlx::Row;
use uuid::Uuid;

use crate::evaluations::{conflit, db, introuvable, refus};
use crate::evaluations_v1::{periode_eleve_tx, regles_tx, Attendu, EcrireResultat, SourceResultat};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Types d'événements de V4. Charges : identifiants seulement, **jamais** d'`eleve_id`, de statut ni de motif.
pub mod types_v4 {
    pub const RATTRAPAGE_CONVOQUE: &str = "evaluations.rattrapage_convoque";
    pub const RATTRAPAGE_DEPLACE: &str = "evaluations.rattrapage_deplace";
    pub const RATTRAPAGE_PASSE: &str = "evaluations.rattrapage_passe";
    pub const RATTRAPAGE_MANQUE: &str = "evaluations.rattrapage_manque";
    pub const RATTRAPAGE_ANNULE: &str = "evaluations.rattrapage_annule";
    pub const REMPLACEMENT_CONVOQUE: &str = "evaluations.remplacement_convoque";
    pub const ZERO_REMPLACEMENT: &str = "evaluations.zero_remplacement";
}

/// Plafond d'élèves convoqués en un geste (un cours n'en compte pas plus, F8).
const MAX_CONVOQUES: usize = 300;

/// Plafond d'élèves examinés et rendus par la lecture des absences répétées (F-5, coût borné).
const MAX_ABSENCES_REPETEES: i64 = 500;

/// Aménagements qui demandent une aide humaine (AESH) le jour de l'évaluation (CE-37).
const AMENAGEMENTS_AIDE_HUMAINE: [&str; 4] = [
    "lecteur",
    "secretaire",
    "pauses_surveillees",
    "consignes_reformulees",
];

/// Contexte d'une évaluation, lu une fois.
struct EvalCtx {
    cours_id: Uuid,
    date_eval: NaiveDate,
    matiere_id: Option<Uuid>,
    annee_id: Uuid,
    bareme_m: i64,
    hors_bareme: bool,
    /// Annoncée aux élèves ou déjà publiée (seules ces évaluations existent pour un AESH, F-8).
    annoncee: bool,
}

async fn charger_eval(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
) -> Result<EvalCtx, EntDbError> {
    let r = sqlx::query(
        "SELECT e.cours_id, e.date_eval, c.matiere_id, e.annee_id, (e.bareme * 1000)::bigint AS bareme_m,
                e.hors_bareme_autorise, e.etat, e.mode,
                (e.annoncee OR e.publiee_le IS NOT NULL) AS annoncee
           FROM eva_evaluation e
           JOIN ent_cours c ON c.id = e.cours_id AND c.etablissement_id = e.etablissement_id
          WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(etab)
    .bind(evaluation_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let etat: String = r.try_get("etat").map_err(db)?;
    let mode: String = r.try_get("mode").map_err(db)?;
    if etat != "ouverte" || mode != "note" {
        return Err(conflit(
            "évaluation annulée ou sans note : rien à proposer ni à convoquer",
        ));
    }
    Ok(EvalCtx {
        cours_id: r.try_get("cours_id").map_err(db)?,
        date_eval: r.try_get("date_eval").map_err(db)?,
        matiere_id: r.try_get("matiere_id").map_err(db)?,
        annee_id: r.try_get("annee_id").map_err(db)?,
        bareme_m: r.try_get("bareme_m").map_err(db)?,
        hors_bareme: r.try_get("hors_bareme_autorise").map_err(db)?,
        annoncee: r.try_get("annoncee").map_err(db)?,
    })
}

/// Élèves du public de l'évaluation à sa date (restriction comprise), triés (ordre stable).
async fn public_evaluation(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
    ev: &EvalCtx,
) -> Result<Vec<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT pu.eleve_id FROM ent_public_cours_jour($1, $2, $3) pu
          WHERE NOT EXISTS (SELECT 1 FROM eva_evaluation_restriction rr
                             WHERE rr.etablissement_id = $1 AND rr.evaluation_id = $4)
             OR EXISTS (SELECT 1 FROM eva_evaluation_restriction rr
                         WHERE rr.etablissement_id = $1 AND rr.evaluation_id = $4 AND rr.eleve_id = pu.eleve_id)
          ORDER BY pu.eleve_id",
    )
    .bind(etab)
    .bind(ev.date_eval)
    .bind(ev.cours_id)
    .bind(evaluation_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)
}

fn modalite_ou_defaut(m: Option<&str>) -> Result<&'static str, EntDbError> {
    match m {
        None => Ok("ecrit"),
        Some(x) => MODALITES_RATTRAPAGE
            .iter()
            .copied()
            .find(|c| *c == x.trim())
            .ok_or(refus("modalité de rattrapage inconnue")),
    }
}

fn lieu_valide(l: Option<&str>) -> Result<Option<String>, EntDbError> {
    match l.map(str::trim).filter(|x| !x.is_empty()) {
        None => Ok(None),
        Some(x) if x.chars().count() <= 80 => Ok(Some(x.to_string())),
        Some(_) => Err(refus("lieu trop long (80 caractères au plus)")),
    }
}

fn dto_rattrapage(r: &PgRow, jour: NaiveDate) -> Result<RattrapageDto, EntDbError> {
    let etat: String = r.try_get("etat").map_err(db)?;
    let nature: String = r.try_get("nature").map_err(db)?;
    let echeance: NaiveDate = r.try_get("echeance").map_err(db)?;
    let zero_attribue: bool = r.try_get("zero_attribue").map_err(db)?;
    Ok(RattrapageDto {
        id: r.try_get("id").map_err(db)?,
        evaluation_id: r.try_get("evaluation_id").map_err(db)?,
        eleve_id: r.try_get("eleve_id").map_err(db)?,
        seance_id: r.try_get("seance_id").map_err(db)?,
        precedent_id: r.try_get("precedent_id").map_err(db)?,
        prevu_le: r.try_get("prevu_le").map_err(db)?,
        heure: r.try_get::<Option<NaiveTime>, _>("heure").map_err(db)?,
        lieu: r.try_get("lieu").map_err(db)?,
        modalite: r.try_get("modalite").map_err(db)?,
        echeance,
        echu: etat == "convoque" && echeance < jour,
        version: r.try_get("version").map_err(db)?,
        zero_possible: nature == "remplacement" && etat == "absent_non_justifie" && !zero_attribue,
        zero_attribue,
        nature,
        etat,
    })
}

const COLONNES_RATTRAPAGE: &str =
    "id, evaluation_id, eleve_id, nature, seance_id, precedent_id, prevu_le, heure, lieu,
    modalite, echeance, etat, version, (zero_attribue_le IS NOT NULL) AS zero_attribue";

async fn evt_neutre(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    type_name: &str,
    evaluation_id: Uuid,
    rattrapage_id: Uuid,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    tx.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Devoir,
        aggregate_id: evaluation_id,
        // Neutre (règle 1) : jamais d'élève, de statut, de note ni de motif ; ni titre, ni nom.
        eleve_id: None,
        payload: json!({ "evaluation_id": evaluation_id, "rattrapage_id": rattrapage_id }),
        actor_user_id: Some(acteur.to_string()),
    })
    .await?;
    Ok(())
}

/// Résultat actuel d'un élève à une évaluation : (statut, version).
async fn resultat_actuel(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
    eleve_id: Uuid,
) -> Result<Option<(String, i32)>, EntDbError> {
    sqlx::query_as(
        "SELECT statut, version FROM eva_resultat
          WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(etab)
    .bind(evaluation_id)
    .bind(eleve_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)
}

/// Statuts d'une case que V4 peut **remplacer** (une case vide aussi) : jamais une note, une dispense ni un statut de
/// neutralisation saisis entre-temps.
const STATUTS_REMPLACABLES: [&str; 4] = ["absent", "a_rattraper", "non_rendu", "non_note"];

/// V3d (M-1) : relit la case **sous verrou** (`FOR UPDATE`) et rend la version à attendre à l'écriture : `Vide` si la case
/// l'est encore, `Version(v)` si elle porte un statut remplaçable ; sinon **conflit** (une note, une dispense ou une
/// annulation arrivée depuis la lecture de l'écran ne s'écrase jamais). Aucune écriture ne passe en `Attendu::Aucun`.
async fn attendu_case_remplacable(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
    eleve_id: Uuid,
) -> Result<Attendu, EntDbError> {
    let existant: Option<(String, i32)> = sqlx::query_as(
        "SELECT statut, version FROM eva_resultat
          WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3 FOR UPDATE",
    )
    .bind(etab)
    .bind(evaluation_id)
    .bind(eleve_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    match existant {
        None => Ok(Attendu::Vide),
        Some((st, v)) if STATUTS_REMPLACABLES.contains(&st.as_str()) => Ok(Attendu::Version(v)),
        Some(_) => Err(conflit(
            "le résultat de cet élève a changé depuis votre lecture (note, dispense ou annulation) : relire avant d'écrire",
        )),
    }
}

/// Résultat de l'analyse des propositions (partagé par la lecture et la confirmation).
struct CalculPropositions {
    seance_trouvee: bool,
    propositions: Vec<PropositionDto>,
    inaptes_partiels: Vec<Uuid>,
    incoherences: Vec<IncoherenceAppelDto>,
}

async fn calculer_propositions(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    evaluation_id: Uuid,
    ev: &EvalCtx,
    caches: &HashSet<Uuid>,
) -> Result<CalculPropositions, EntDbError> {
    let seance_trouvee: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_edt_seance s
                         WHERE s.etablissement_id = $1 AND s.cours_id = $2 AND s.jour = $3
                           AND s.etat NOT IN ('annulee', 'deplacee', 'neutralisee', 'non_assuree', 'retiree'))",
    )
    .bind(etab)
    .bind(ev.cours_id)
    .bind(ev.date_eval)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?;
    let public = public_evaluation(tx, etab, evaluation_id, ev).await?;
    let lignes = sqlx::query(
        "SELECT e.eleve_id,
                eva_absence_a_la_seance($1, e.eleve_id, $3, $2) AS absence,
                eva_dispense_a_la_date($1, e.eleve_id, $5, $2) AS dispense,
                r.statut AS statut_existant
           FROM unnest($6::uuid[]) AS e(eleve_id)
           LEFT JOIN eva_resultat r ON r.etablissement_id = $1 AND r.evaluation_id = $4 AND r.eleve_id = e.eleve_id
          ORDER BY e.eleve_id",
    )
    .bind(etab)
    .bind(ev.date_eval)
    .bind(ev.cours_id)
    .bind(evaluation_id)
    .bind(ev.matiere_id)
    .bind(&public)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut sortie = CalculPropositions {
        seance_trouvee,
        propositions: Vec::new(),
        inaptes_partiels: Vec::new(),
        incoherences: Vec::new(),
    };
    for l in &lignes {
        let eleve_id: Uuid = l.try_get("eleve_id").map_err(db)?;
        if caches.contains(&eleve_id) {
            continue;
        }
        let absence: Option<String> = l.try_get("absence").map_err(db)?;
        let dispense: Option<String> = l.try_get("dispense").map_err(db)?;
        let existant: Option<String> = l.try_get("statut_existant").map_err(db)?;
        if dispense.as_deref() == Some("inaptitude_partielle") {
            sortie.inaptes_partiels.push(eleve_id);
        }
        match existant.as_deref() {
            // CE-31 : une note existe alors que l'appel dit « absent » : la saisie prévaut, la vie scolaire est prévenue.
            Some("note") if absence.is_some() => {
                sortie.incoherences.push(IncoherenceAppelDto { eleve_id });
                continue;
            }
            // Une case déjà remplie (statut ou note) n'est jamais écrasée par une proposition.
            Some(_) => continue,
            None => {}
        }
        match (dispense.as_deref(), absence) {
            (Some(n @ ("dispense" | "inaptitude_totale")), _) => {
                sortie.propositions.push(PropositionDto {
                    eleve_id,
                    statut: "dispense".into(),
                    justification: None,
                    source: "dispense".into(),
                    nature_dispense: Some(n.to_string()),
                    rattrapage_propose: false,
                })
            }
            (_, Some(j)) => sortie.propositions.push(PropositionDto {
                eleve_id,
                statut: "absent".into(),
                // CE-25 : justifiée → « programmer un rattrapage » ; en attente / non justifiée : le professeur décide.
                rattrapage_propose: j == "justifiee",
                justification: Some(j),
                source: "appel".into(),
                nature_dispense: None,
            }),
            _ => {}
        }
    }
    Ok(sortie)
}

impl PgEnt {
    async fn eva_v4_cours_de(&self, etab: Uuid, evaluation_id: Uuid) -> Result<Uuid, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let c: Option<Uuid> = sqlx::query_scalar(
            "SELECT cours_id FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        c.ok_or_else(introuvable)
    }

    /// V3d (F-2), règle 5 : personnel, puis **droit sur le cours** de l'évaluation. Rien n'est révélé avant le droit :
    /// l'existence de l'évaluation, son état, l'élève et les cases ne sont lus qu'ensuite. (Une évaluation inconnue ou
    /// d'un autre établissement est « introuvable » : l'isolation par ligne la rend invisible.)
    async fn eva_v4_droit_cours(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        evaluation_id: Uuid,
        jour: NaiveDate,
    ) -> Result<(Uuid, crate::evaluations_v3::AccesCours), EntDbError> {
        Self::exiger_staff(role)?;
        let cours = self.eva_v4_cours_de(etab, evaluation_id).await?;
        let acces = self.eva_v3_acces(etab, acteur, role, cours, jour).await?;
        Ok((cours, acces))
    }

    /// Élèves cachés à l'acteur (garde du socle 2), calculés une fois pour un ensemble d'élèves.
    async fn eva_v4_caches(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        ids: &[Uuid],
    ) -> Result<HashSet<Uuid>, EntDbError> {
        let profil = self.profil_champs(etab, acteur, role).await?;
        self.proteges_caches(etab, role, &profil, ids).await
    }

    // ───────────────────────── Propositions d'absents et de dispensés ─────────────────────────

    /// Statuts **proposés** pour les cases vides de l'évaluation : absents (appel de la séance du cours, port 12),
    /// dispensés (port 34). Réservé à celui qui saisit (titulaire, co-enseignant, suppléant en saisie).
    pub async fn eva_v4_propositions_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        evaluation_id: Uuid,
        jour: NaiveDate,
    ) -> Result<PropositionsDto, EntDbError> {
        let (_, acces) = self
            .eva_v4_droit_cours(etab, acteur, role, evaluation_id, jour)
            .await?;
        if !acces.saisie {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = charger_eval(&mut tx, etab, evaluation_id).await?;
        let public = public_evaluation(&mut tx, etab, evaluation_id, &ev).await?;
        tx.commit().await.map_err(db)?;
        let caches = self.eva_v4_caches(etab, acteur, role, &public).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let c = calculer_propositions(&mut tx, etab, evaluation_id, &ev, &caches).await?;
        tx.commit().await.map_err(db)?;
        Ok(PropositionsDto {
            evaluation_id,
            seance_trouvee: c.seance_trouvee,
            propositions: c.propositions,
            inaptes_partiels: c.inaptes_partiels,
            incoherences: c.incoherences,
        })
    }

    /// Le professeur **confirme** tout ou partie des propositions : le serveur les recalcule, écrit chaque statut par
    /// l'écriture commune d'un résultat (source `appel` ou `dispense`, case vide attendue) et ne touche jamais une case déjà
    /// remplie.
    pub async fn eva_v4_appliquer_propositions_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &AppliquerPropositionsBody,
    ) -> Result<Vec<PropositionAppliqueeDto>, EntDbError> {
        let (cours, acces) = self
            .eva_v4_droit_cours(etab, acteur, role, body.evaluation_id, jour)
            .await?;
        if !acces.saisie {
            return Err(EntDbError::Forbidden);
        }
        if cours != body.cours_id {
            return Err(refus("évaluation d'un autre cours"));
        }
        // F-3 : la liste demandée est bornée (un cours n'a pas plus d'élèves que `MAX_CONVOQUES`).
        if body
            .eleves
            .as_ref()
            .is_some_and(|v| v.len() > MAX_CONVOQUES)
        {
            return Err(refus("trop d'élèves dans une même confirmation"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = charger_eval(&mut tx, etab, body.evaluation_id).await?;
        let public = public_evaluation(&mut tx, etab, body.evaluation_id, &ev).await?;
        tx.commit().await.map_err(db)?;
        let caches = self.eva_v4_caches(etab, acteur, role, &public).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let calcul = calculer_propositions(&mut tx, etab, body.evaluation_id, &ev, &caches).await?;
        let voulus: Option<BTreeSet<Uuid>> =
            body.eleves.as_ref().map(|v| v.iter().copied().collect());
        let mut retours = Vec::new();
        // Élèves demandés mais non proposés (déjà remplis, cachés, hors public…) : réponse explicite, aucune écriture.
        if let Some(v) = &voulus {
            let proposes: HashSet<Uuid> = calcul.propositions.iter().map(|p| p.eleve_id).collect();
            for e in v {
                if !proposes.contains(e) {
                    retours.push(PropositionAppliqueeDto {
                        eleve_id: *e,
                        issue: "non_propose".into(),
                        statut: None,
                    });
                }
            }
        }
        for p in &calcul.propositions {
            if voulus.as_ref().is_some_and(|v| !v.contains(&p.eleve_id)) {
                continue;
            }
            let (statut, source) = if p.statut == "dispense" {
                (StatutCalcul::Dispense, SourceResultat::Dispense)
            } else {
                (StatutCalcul::Absent, SourceResultat::Appel)
            };
            let r = EcrireResultat {
                evaluation_id: body.evaluation_id,
                eleve_id: p.eleve_id,
                statut,
                valeur_m: None,
                motif: None,
                commentaire_eleve: None,
                source,
            };
            sqlx::query("SAVEPOINT eva_v4_prop")
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            match Self::ecrire_resultat_tx(
                &mut tx,
                etab,
                acteur,
                acces.corriger,
                jour,
                &r,
                Attendu::Vide,
            )
            .await
            {
                Ok(_) => {
                    sqlx::query("RELEASE SAVEPOINT eva_v4_prop")
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    retours.push(PropositionAppliqueeDto {
                        eleve_id: p.eleve_id,
                        issue: "ecrit".into(),
                        statut: Some(p.statut.clone()),
                    });
                }
                Err(e) => {
                    sqlx::query("ROLLBACK TO SAVEPOINT eva_v4_prop")
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    sqlx::query("RELEASE SAVEPOINT eva_v4_prop")
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                    match e {
                        EntDbError::ConflitDossier(_) => retours.push(PropositionAppliqueeDto {
                            eleve_id: p.eleve_id,
                            issue: "deja_saisi".into(),
                            statut: None,
                        }),
                        // F-3 : une panne d'infrastructure n'est jamais un « refusé » : elle remonte (la transaction
                        // entière est abandonnée, rien n'est écrit à moitié).
                        EntDbError::Domain(DomainError::Infrastructure) => {
                            return Err(EntDbError::Domain(DomainError::Infrastructure))
                        }
                        EntDbError::Forbidden
                        | EntDbError::SaisieInvalide(_)
                        | EntDbError::Domain(_) => retours.push(PropositionAppliqueeDto {
                            eleve_id: p.eleve_id,
                            issue: "refuse".into(),
                            statut: None,
                        }),
                        autre => return Err(autre),
                    }
                }
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(retours)
    }

    // ───────────────────────── Port 34 : dispenses (simulation) ─────────────────────────

    /// Déclare une dispense ou une inaptitude **datée, sans motif** (`notes.corriger`). Ce sera le socle 34 qui
    /// l'alimentera ; en attendant, la direction la saisit ici.
    pub async fn eva_v4_declarer_dispense_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        body: &DeclarerDispenseBody,
    ) -> Result<DispenseDto, EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, body.eleve_id)
            .await?;
        if !NATURES_DISPENSE.contains(&body.nature.as_str()) {
            return Err(refus("nature de dispense inconnue"));
        }
        if body.au.is_some_and(|a| a < body.du) {
            return Err(refus("la fin de la dispense précède son début"));
        }
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO eva_dispense_port (id, etablissement_id, eleve_id, matiere_id, nature, du, au, saisi_par)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(etab)
        .bind(body.eleve_id)
        .bind(body.matiere_id)
        .bind(&body.nature)
        .bind(body.du)
        .bind(body.au)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(|e| match e.as_database_error().and_then(|d| d.code()).as_deref() {
            Some("23503") | Some("23514") => refus("élève ou enseignement introuvable"),
            _ => db(e),
        })?;
        tx.commit().await.map_err(db)?;
        Ok(DispenseDto {
            id,
            eleve_id: body.eleve_id,
            matiere_id: body.matiere_id,
            nature: body.nature.clone(),
            du: body.du,
            au: body.au,
            retiree: false,
        })
    }

    /// Retire une dispense (`notes.corriger`) ; elle reste au registre avec son retrait.
    pub async fn eva_v4_retirer_dispense_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        dispense_id: Uuid,
    ) -> Result<(), EntDbError> {
        Self::exiger_staff(role)?;
        if !self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleve: Uuid = sqlx::query_scalar(
            "SELECT eleve_id FROM eva_dispense_port WHERE etablissement_id = $1 AND id = $2 AND retiree_le IS NULL",
        )
        .bind(etab)
        .bind(dispense_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE eva_dispense_port SET retiree_le = now(), retiree_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND retiree_le IS NULL",
        )
        .bind(etab)
        .bind(dispense_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Dispenses d'un élève, sans motif (`notes.lire` à portée établissement ou `notes.corriger`).
    pub async fn eva_v4_dispenses_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<DispenseDto>, EntDbError> {
        Self::exiger_staff(role)?;
        let lecture = self
            .a_action_etablissement(etab, acteur, role, Action::NotesLire)
            .await?;
        let corriger = self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?;
        if !lecture && !corriger {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(
            "SELECT id, eleve_id, matiere_id, nature, du, au, (retiree_le IS NOT NULL) AS retiree
               FROM eva_dispense_port WHERE etablissement_id = $1 AND eleve_id = $2
              ORDER BY du, id",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        rows.iter()
            .map(|r| {
                Ok(DispenseDto {
                    id: r.try_get("id").map_err(db)?,
                    eleve_id: r.try_get("eleve_id").map_err(db)?,
                    matiere_id: r.try_get("matiere_id").map_err(db)?,
                    nature: r.try_get("nature").map_err(db)?,
                    du: r.try_get("du").map_err(db)?,
                    au: r.try_get("au").map_err(db)?,
                    retiree: r.try_get("retiree").map_err(db)?,
                })
            })
            .collect()
    }

    // ───────────────────────── Port 35 : aménagements d'évaluation ─────────────────────────

    /// Aménagements fonctionnels des élèves de l'évaluation, à sa date : le professeur du cours (titulaire,
    /// co-enseignant, suppléant), la direction (`notes.corriger`) et l'**AESH** de chaque élève (pour ses élèves seulement).
    /// Libellés du catalogue fermé : jamais le plan (PAP, PPS), jamais un diagnostic, jamais une note.
    pub async fn eva_v4_amenagements_evaluation_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        evaluation_id: Uuid,
        jour: NaiveDate,
    ) -> Result<AmenagementsEvaluationDto, EntDbError> {
        Self::exiger_staff(role)?;
        let corriger = self
            .a_action_etablissement(etab, acteur, role, Action::NotesCorriger)
            .await?;
        let cours = self.eva_v4_cours_de(etab, evaluation_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let acces =
            Self::eva_v3_acces_tx(&mut tx, etab, acteur, cours, jour, false, corriger).await?;
        let ev = charger_eval(&mut tx, etab, evaluation_id).await?;
        let public = public_evaluation(&mut tx, etab, evaluation_id, &ev).await?;
        // F-8 : une donnée d'accompagnement ne se lit que si le module santé de l'établissement est actif (fermé par
        // défaut), après le droit sur le cours.
        self.exiger_module_sante(etab).await?;
        let voit_tous = acces.prof_du_cours || acces.saisie || acces.corriger;
        // L'AESH n'a de droit que sur ses élèves, à la date de l'évaluation.
        let siens: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT eleve_id FROM ent_aesh_affectation
              WHERE etablissement_id = $1 AND aesh_user_id = $2
                AND date_debut <= $3 AND (date_fin IS NULL OR date_fin >= $3)",
        )
        .bind(etab)
        .bind(acteur)
        .bind(ev.date_eval)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // F-8 : un AESH ne connaît que les évaluations **annoncées** (ou publiées) de ses élèves : une évaluation que le
        // professeur n'a pas annoncée n'existe pas pour lui.
        if !voit_tous && (siens.is_empty() || !ev.annoncee) {
            return Err(EntDbError::Forbidden);
        }
        let visibles: Vec<Uuid> = if voit_tous {
            public.clone()
        } else {
            public
                .iter()
                .copied()
                .filter(|e| siens.contains(e))
                .collect()
        };
        tx.commit().await.map_err(db)?;
        let caches = self.eva_v4_caches(etab, acteur, role, &visibles).await?;
        let visibles: Vec<Uuid> = visibles
            .into_iter()
            .filter(|e| !caches.contains(e))
            .collect();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes = sqlx::query(
            "SELECT a.eleve_id, a.code,
                    EXISTS (SELECT 1 FROM ent_aesh_affectation af
                              JOIN eva_aesh_indisponibilite_port ind
                                ON ind.etablissement_id = af.etablissement_id AND ind.aesh_user_id = af.aesh_user_id
                             WHERE af.etablissement_id = a.etablissement_id AND af.eleve_id = a.eleve_id
                               AND af.date_debut <= $3 AND (af.date_fin IS NULL OR af.date_fin >= $3)
                               AND ind.du <= $3 AND ind.au >= $3) AS aesh_indisponible
               FROM ent_amenagement a
              WHERE a.etablissement_id = $1 AND a.eleve_id = ANY($2) AND a.actif AND a.retire_at IS NULL
                AND a.portee IN ('examen', 'les_deux')
                AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
              ORDER BY a.eleve_id, a.code",
        )
        .bind(etab)
        .bind(&visibles)
        .bind(ev.date_eval)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut eleves: Vec<EleveAmenageDto> = Vec::new();
        for l in &lignes {
            let eleve_id: Uuid = l.try_get("eleve_id").map_err(db)?;
            let code: String = l.try_get("code").map_err(db)?;
            let indispo: bool = l.try_get("aesh_indisponible").map_err(db)?;
            // F-8 : un code hors du catalogue fermé n'est jamais montré (ni son code, ni un libellé générique).
            let Some(entree) = crate::accompagnement::amenagement_du_catalogue(&code) else {
                continue;
            };
            let libelle = entree.libelle.to_string();
            if eleves.last().is_none_or(|e| e.eleve_id != eleve_id) {
                eleves.push(EleveAmenageDto {
                    eleve_id,
                    amenagements: Vec::new(),
                    aesh_indisponible: indispo,
                    alerte: None,
                });
            }
            let Some(cur) = eleves.last_mut() else {
                continue;
            };
            if indispo && cur.alerte.is_none() && AMENAGEMENTS_AIDE_HUMAINE.contains(&code.as_str())
            {
                cur.alerte = Some(format!(
                    "L'aménagement « {libelle} » ne pourra pas être assuré : l'AESH est indisponible ce jour. \
                     Reporter l'évaluation pour cet élève (rattrapage) ?"
                ));
            }
            cur.amenagements.push(AmenagementCodeDto { code, libelle });
        }
        // F-8 : la lecture d'aménagements (donnée d'accompagnement) est journalisée, élève par élève.
        for e in &eleves {
            self.tracer_acces_sensible(
                etab,
                acteur,
                role,
                Some(e.eleve_id),
                "evaluations",
                "consultation_amenagements",
                "Consultation des aménagements d'une évaluation",
            )
            .await?;
        }
        Ok(AmenagementsEvaluationDto {
            evaluation_id,
            date_eval: ev.date_eval,
            eleves,
        })
    }

    // ───────────────────────── Rattrapages et évaluation de remplacement ─────────────────────────

    /// Planifie et convoque un rattrapage (un ou plusieurs élèves : séance collective, EVA-259).
    pub async fn eva_v4_planifier_rattrapage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &PlanifierRattrapageBody,
    ) -> Result<Vec<RattrapageDto>, EntDbError> {
        self.eva_v4_convoquer(etab, acteur, role, jour, body, "rattrapage", None)
            .await
    }

    /// Convoque à l'**évaluation de remplacement** (EVA-172) : réservé à `notes.corriger` (direction).
    pub async fn eva_v4_convoquer_remplacement_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &PlanifierRattrapageBody,
    ) -> Result<Vec<RattrapageDto>, EntDbError> {
        self.eva_v4_convoquer(etab, acteur, role, jour, body, "remplacement", None)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn eva_v4_convoquer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        body: &PlanifierRattrapageBody,
        nature: &'static str,
        precedent: Option<Uuid>,
    ) -> Result<Vec<RattrapageDto>, EntDbError> {
        // F-2 : personnel, puis droit sur le cours, AVANT toute lecture ou validation du contenu.
        let (_, acces) = self
            .eva_v4_droit_cours(etab, acteur, role, body.evaluation_id, jour)
            .await?;
        let autorise = if nature == "remplacement" {
            acces.corriger
        } else {
            acces.saisie || acces.corriger
        };
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        let eleves: Vec<Uuid> = body
            .eleves
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if eleves.is_empty() {
            return Err(refus("aucun élève à convoquer"));
        }
        if eleves.len() > MAX_CONVOQUES {
            return Err(refus("trop d'élèves dans une même convocation"));
        }
        if body.prevu_le < jour {
            return Err(refus("la date du rattrapage est déjà passée"));
        }
        let echeance = body.echeance.unwrap_or(body.prevu_le);
        if echeance < body.prevu_le {
            return Err(refus("l'échéance précède le jour prévu"));
        }
        let modalite = modalite_ou_defaut(body.modalite.as_deref())?;
        let lieu = lieu_valide(body.lieu.as_deref())?;
        let caches = self.eva_v4_caches(etab, acteur, role, &eleves).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ev = charger_eval(&mut tx, etab, body.evaluation_id).await?;
        let public: HashSet<Uuid> = public_evaluation(&mut tx, etab, body.evaluation_id, &ev)
            .await?
            .into_iter()
            .collect();
        let seance_id = Uuid::new_v4();
        let mut crees: Vec<Uuid> = Vec::new();
        for eleve in &eleves {
            if caches.contains(eleve) || !public.contains(eleve) {
                return Err(refus(
                    "élève introuvable ou non concerné par cette évaluation",
                ));
            }
            let actuel = resultat_actuel(&mut tx, etab, body.evaluation_id, *eleve).await?;
            // Un rattrapage se prévoit pour une case vide ou « sans note » ; jamais pour une note déjà saisie, ni pour une
            // dispense. (Un remplacement vise l'évaluation de remplacement elle-même : mêmes cases.)
            if let Some((st, _)) = &actuel {
                if !["absent", "a_rattraper", "non_rendu", "non_note", "annule"]
                    .contains(&st.as_str())
                {
                    return Err(refus(
                        "cet élève a déjà un résultat sur cette évaluation : rien à rattraper",
                    ));
                }
            }
            // V3d (F-4, doc V4 §2.3) : période de l'élève arrêtée sans rattrapage autorisé = convocation refusée, QUI QUE
            // SOIT l'auteur (la direction aussi : elle autorise d'abord, avec un motif, puis convoque). Rien n'est écrit.
            if nature == "rattrapage" {
                let ouverte = periode_eleve_tx(&mut tx, etab, body.evaluation_id, *eleve)
                    .await?
                    .is_some_and(|p| p.etat == "ouverte");
                if !ouverte {
                    let autorise: bool =
                        sqlx::query_scalar("SELECT eva_rattrapage_en_vigueur($1, $2, $3)")
                            .bind(etab)
                            .bind(body.evaluation_id)
                            .bind(eleve)
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(db)?;
                    if !autorise {
                        return Err(refus(
                            "période arrêtée pour cet élève : la direction doit d'abord autoriser le rattrapage",
                        ));
                    }
                }
            }
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO eva_rattrapage (id, etablissement_id, evaluation_id, eleve_id, nature, seance_id, precedent_id,
                                             prevu_le, heure, lieu, modalite, echeance, cree_par, modifie_par)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13)",
            )
            .bind(id)
            .bind(etab)
            .bind(body.evaluation_id)
            .bind(eleve)
            .bind(nature)
            .bind(seance_id)
            .bind(precedent)
            .bind(body.prevu_le)
            .bind(body.heure)
            .bind(&lieu)
            .bind(modalite)
            .bind(echeance)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(|e| match e.as_database_error().and_then(|d| d.code()).as_deref() {
                Some("23505") => conflit("une convocation est déjà en cours pour cet élève et cette évaluation"),
                Some("23514") | Some("23503") => refus("convocation refusée par les règles du rattrapage"),
                _ => db(e),
            })?;
            // Le résultat se lit « à rattraper » (jamais un zéro) : seulement par celui qui saisit, et seulement si la case
            // n'a pas déjà un statut équivalent. Une période arrêtée sans rattrapage autorisé refuse la convocation entière.
            if acces.saisie
                && nature == "rattrapage"
                && !matches!(
                    actuel.as_ref().map(|(s, _)| s.as_str()),
                    Some("a_rattraper" | "non_note" | "annule")
                )
            {
                let r = EcrireResultat {
                    evaluation_id: body.evaluation_id,
                    eleve_id: *eleve,
                    statut: StatutCalcul::ARattraper,
                    valeur_m: None,
                    motif: None,
                    commentaire_eleve: None,
                    source: SourceResultat::Saisie,
                };
                let attendu = match &actuel {
                    None => Attendu::Vide,
                    Some((_, v)) => Attendu::Version(*v),
                };
                Self::ecrire_resultat_tx(&mut tx, etab, acteur, acces.corriger, jour, &r, attendu)
                    .await
                    .map_err(|e| match e {
                        EntDbError::Forbidden => refus(
                            "période arrêtée pour cet élève : la direction doit d'abord autoriser le rattrapage",
                        ),
                        EntDbError::ConflitDossier(_) => {
                            conflit("le résultat de cet élève a changé : relire avant de convoquer")
                        }
                        autre => autre,
                    })?;
            }
            crees.push(id);
        }
        let type_name = if nature == "remplacement" {
            types_v4::REMPLACEMENT_CONVOQUE
        } else {
            types_v4::RATTRAPAGE_CONVOQUE
        };
        evt_neutre(
            &mut tx,
            etab,
            type_name,
            body.evaluation_id,
            seance_id,
            acteur,
        )
        .await?;
        let rows = sqlx::query(&format!(
            "SELECT {COLONNES_RATTRAPAGE} FROM eva_rattrapage WHERE etablissement_id = $1 AND id = ANY($2)
              ORDER BY eleve_id, id"
        ))
        .bind(etab)
        .bind(&crees)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        rows.iter().map(|r| dto_rattrapage(r, jour)).collect()
    }

    /// Verrouille une convocation et vérifie droits, élève visible et version. Rend la ligne et le contexte de l'évaluation.
    async fn eva_v4_charger_convocation(
        &self,
        tx_etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        rattrapage_id: Uuid,
        version_attendue: i32,
    ) -> Result<
        (
            crate::evaluations_v3::AccesCours,
            Uuid,
            Uuid,
            String,
            String,
            Uuid,
        ),
        EntDbError,
    > {
        Self::exiger_staff(role)?;
        let etab = tx_etab;
        let (evaluation_id, eleve_id): (Uuid, Uuid) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query_as(
                "SELECT evaluation_id, eleve_id FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(rattrapage_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?
        };
        // F-2, règle 5 : droit sur le cours d'abord, puis visibilité de l'élève, puis état et version.
        let (_, acces) = self
            .eva_v4_droit_cours(etab, acteur, role, evaluation_id, jour)
            .await?;
        self.exiger_eleve_visible(etab, acteur, role, eleve_id)
            .await?;
        let (nature, etat, version, cree_par): (String, String, i32, Uuid) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query_as(
                "SELECT nature, etat, version, cree_par FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(rattrapage_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?
        };
        if version != version_attendue {
            return Err(conflit("la convocation a changé depuis votre lecture"));
        }
        Ok((acces, evaluation_id, eleve_id, nature, etat, cree_par))
    }

    /// Déplace une convocation encore en cours (date, heure, lieu, modalité, échéance).
    pub async fn eva_v4_deplacer_rattrapage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        rattrapage_id: Uuid,
        body: &DeplacerRattrapageBody,
    ) -> Result<RattrapageDto, EntDbError> {
        let (acces, evaluation_id, _, nature, etat, cree_par) = self
            .eva_v4_charger_convocation(
                etab,
                acteur,
                role,
                jour,
                rattrapage_id,
                body.version_attendue,
            )
            .await?;
        // V3d (M-2, CE-44) : un suppléant ne déplace ni n'annule la convocation du titulaire sans droit de correction
        // (comme V3 pour les évaluations : `auteur || !suppléant || corriger`).
        if !(acces.corriger
            || (acces.saisie && nature == "rattrapage" && (cree_par == acteur || !acces.suppleant)))
        {
            return Err(EntDbError::Forbidden);
        }
        if etat != "convoque" {
            return Err(conflit("seule une convocation en cours se déplace"));
        }
        if body.prevu_le < jour {
            return Err(refus("la date du rattrapage est déjà passée"));
        }
        let echeance = body.echeance.unwrap_or(body.prevu_le);
        if echeance < body.prevu_le {
            return Err(refus("l'échéance précède le jour prévu"));
        }
        let modalite = modalite_ou_defaut(body.modalite.as_deref())?;
        let lieu = lieu_valide(body.lieu.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE eva_rattrapage SET prevu_le = $3, heure = $4, lieu = $5, modalite = $6, echeance = $7, modifie_par = $8
              WHERE etablissement_id = $1 AND id = $2 AND version = $9 AND etat = 'convoque'",
        )
        .bind(etab)
        .bind(rattrapage_id)
        .bind(body.prevu_le)
        .bind(body.heure)
        .bind(&lieu)
        .bind(modalite)
        .bind(echeance)
        .bind(acteur)
        .bind(body.version_attendue)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("la convocation a changé depuis votre lecture"));
        }
        evt_neutre(
            &mut tx,
            etab,
            types_v4::RATTRAPAGE_DEPLACE,
            evaluation_id,
            rattrapage_id,
            acteur,
        )
        .await?;
        let r = sqlx::query(&format!(
            "SELECT {COLONNES_RATTRAPAGE} FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(rattrapage_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        dto_rattrapage(&r, jour)
    }

    /// Annule une convocation en cours, avec motif (3 à 500 caractères).
    pub async fn eva_v4_annuler_rattrapage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        rattrapage_id: Uuid,
        body: &AnnulerRattrapageBody,
    ) -> Result<RattrapageDto, EntDbError> {
        // F-2 : le personnel d'abord ; la forme du corps ensuite.
        Self::exiger_staff(role)?;
        let motif = body.motif.trim();
        if motif.chars().count() < 3 || motif.chars().count() > 500 {
            return Err(refus("un motif de 3 à 500 caractères est requis"));
        }
        let (acces, evaluation_id, _, nature, etat, cree_par) = self
            .eva_v4_charger_convocation(
                etab,
                acteur,
                role,
                jour,
                rattrapage_id,
                body.version_attendue,
            )
            .await?;
        // V3d (M-2, CE-44) : un suppléant ne déplace ni n'annule la convocation du titulaire sans droit de correction
        // (comme V3 pour les évaluations : `auteur || !suppléant || corriger`).
        if !(acces.corriger
            || (acces.saisie && nature == "rattrapage" && (cree_par == acteur || !acces.suppleant)))
        {
            return Err(EntDbError::Forbidden);
        }
        if etat != "convoque" {
            return Err(conflit("seule une convocation en cours s'annule"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE eva_rattrapage SET etat = 'annule', motif_annulation = $3, modifie_par = $4
              WHERE etablissement_id = $1 AND id = $2 AND version = $5 AND etat = 'convoque'",
        )
        .bind(etab)
        .bind(rattrapage_id)
        .bind(motif)
        .bind(acteur)
        .bind(body.version_attendue)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("la convocation a changé depuis votre lecture"));
        }
        evt_neutre(
            &mut tx,
            etab,
            types_v4::RATTRAPAGE_ANNULE,
            evaluation_id,
            rattrapage_id,
            acteur,
        )
        .await?;
        let r = sqlx::query(&format!(
            "SELECT {COLONNES_RATTRAPAGE} FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(rattrapage_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        dto_rattrapage(&r, jour)
    }

    /// Note le rattrapage (une note, `nn`, `disp`) **ou** constate l'absence au rattrapage lui-même (justifiée ou non).
    /// Une absence ne devient jamais un zéro : la convocation passe « absent justifié » (reconvoquer, CE-28) ou « absent non
    /// justifié » (seule une convocation de remplacement peut alors mener au zéro).
    pub async fn eva_v4_noter_rattrapage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        rattrapage_id: Uuid,
        body: &NoterRattrapageBody,
    ) -> Result<RattrapageDto, EntDbError> {
        let (acces, evaluation_id, eleve_id, _nature, etat, _cree_par) = self
            .eva_v4_charger_convocation(
                etab,
                acteur,
                role,
                jour,
                rattrapage_id,
                body.version_attendue,
            )
            .await?;
        if !(acces.saisie || acces.corriger) {
            return Err(EntDbError::Forbidden);
        }
        if etat != "convoque" {
            return Err(conflit("cette convocation est terminée"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // F-4 : l'échéance est appliquée. Après elle, seule la direction (`notes.corriger`) note, avec un motif ; le
        // constat d'une absence au rattrapage reste possible (il ne change aucune note).
        let echeance: NaiveDate = sqlx::query_scalar(
            "SELECT echeance FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(rattrapage_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let motif_apres_echeance = if body.absence.is_none() && echeance < jour {
            if !acces.corriger {
                return Err(refus(
                    "l'échéance du rattrapage est dépassée : seule la direction peut encore noter, avec un motif",
                ));
            }
            let m = body.motif.as_deref().map(str::trim).unwrap_or("");
            if m.chars().count() < 3 || m.chars().count() > 500 {
                return Err(refus(
                    "échéance dépassée : un motif de 3 à 500 caractères est requis pour noter",
                ));
            }
            Some(m.to_string())
        } else {
            None
        };
        let nouvel_etat = match body.absence.as_deref() {
            Some("justifiee") => "absent_justifie",
            Some("non_justifiee") => "absent_non_justifie",
            Some(_) => return Err(refus("absence inconnue : justifiee ou non_justifiee")),
            None => {
                let texte = body
                    .texte
                    .as_deref()
                    .ok_or(refus("saisir un résultat ou constater l'absence"))?;
                if texte.chars().count() > 40 {
                    return Err(refus("texte de case trop long (40 caractères au plus)"));
                }
                let ev = charger_eval(&mut tx, etab, evaluation_id).await?;
                let pas_m = regles_tx(&mut tx, etab, ev.annee_id).await?.pas_saisie_m;
                let contrainte = ContrainteCellule {
                    bareme_m: ev.bareme_m,
                    hors_bareme_autorise: ev.hors_bareme,
                    pas_m,
                };
                let (statut, valeur_m) = match analyser_cellule(texte, &contrainte) {
                    Analyse::Note(m) => (StatutCalcul::Note, Some(m)),
                    // La copie est perdue (CE-50), l'élève est dispensé : le résultat du rattrapage est un statut.
                    Analyse::Statut(s @ ("non_note" | "dispense")) => {
                        (StatutCalcul::parse(s).ok_or(refus("statut inconnu"))?, None)
                    }
                    Analyse::Erreur(_) => return Err(refus("résultat refusé : vérifiez le barème et le pas de saisie")),
                    _ => {
                        return Err(refus(
                            "un rattrapage se note, ou se constate absent : « abs » et « ar » ne sont pas des résultats",
                        ))
                    }
                };
                let r = EcrireResultat {
                    evaluation_id,
                    eleve_id,
                    statut,
                    valeur_m,
                    motif: motif_apres_echeance,
                    commentaire_eleve: None,
                    source: SourceResultat::Saisie,
                };
                // M-1 : case relue sous verrou ; une note, une dispense ou une annulation arrivée entre-temps = conflit.
                let attendu =
                    attendu_case_remplacable(&mut tx, etab, evaluation_id, eleve_id).await?;
                Self::ecrire_resultat_tx(&mut tx, etab, acteur, acces.corriger, jour, &r, attendu)
                    .await
                    .map_err(|e| match e {
                        // Message exact selon le cas : la direction (sans être du cours) en période ouverte ; ou le
                        // professeur après l'arrêt, sans rattrapage autorisé.
                        EntDbError::Forbidden if acces.corriger => refus(
                            "période de l'élève ouverte : le rattrapage est noté par le professeur du cours",
                        ),
                        EntDbError::Forbidden => refus(
                            "période de l'élève arrêtée : la direction doit d'abord autoriser le rattrapage",
                        ),
                        autre => autre,
                    })?;
                "passe"
            }
        };
        let n = sqlx::query(
            "UPDATE eva_rattrapage SET etat = $3, modifie_par = $4
              WHERE etablissement_id = $1 AND id = $2 AND version = $5 AND etat = 'convoque'",
        )
        .bind(etab)
        .bind(rattrapage_id)
        .bind(nouvel_etat)
        .bind(acteur)
        .bind(body.version_attendue)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("la convocation a changé depuis votre lecture"));
        }
        let type_evt = if nouvel_etat == "passe" {
            types_v4::RATTRAPAGE_PASSE
        } else {
            types_v4::RATTRAPAGE_MANQUE
        };
        evt_neutre(
            &mut tx,
            etab,
            type_evt,
            evaluation_id,
            rattrapage_id,
            acteur,
        )
        .await?;
        let r = sqlx::query(&format!(
            "SELECT {COLONNES_RATTRAPAGE} FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(rattrapage_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        dto_rattrapage(&r, jour)
    }

    /// CE-28 : après une absence **justifiée** au rattrapage (ou au remplacement), nouvelle convocation, liée à la précédente.
    pub async fn eva_v4_reconvoquer_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        rattrapage_id: Uuid,
        body: &ReconvoquerBody,
    ) -> Result<RattrapageDto, EntDbError> {
        Self::exiger_staff(role)?;
        let (evaluation_id, eleve_id, nature, etat): (Uuid, Uuid, String, String) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query_as(
                "SELECT evaluation_id, eleve_id, nature, etat FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(rattrapage_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?
        };
        // F-2, règle 5 : droit sur le cours, puis visibilité de l'élève, puis seulement l'état de la convocation.
        let (_, acces) = self
            .eva_v4_droit_cours(etab, acteur, role, evaluation_id, jour)
            .await?;
        let autorise = if nature == "remplacement" {
            acces.corriger
        } else {
            acces.saisie || acces.corriger
        };
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve_id)
            .await?;
        if etat != "absent_justifie" {
            return Err(conflit("on ne reconvoque qu'après une absence justifiée"));
        }
        let nature_statique: &'static str = if nature == "remplacement" {
            "remplacement"
        } else {
            "rattrapage"
        };
        let corps = PlanifierRattrapageBody {
            evaluation_id,
            eleves: vec![eleve_id],
            prevu_le: body.prevu_le,
            heure: body.heure,
            lieu: body.lieu.clone(),
            modalite: body.modalite.clone(),
            echeance: body.echeance,
        };
        let mut v = self
            .eva_v4_convoquer(
                etab,
                acteur,
                role,
                jour,
                &corps,
                nature_statique,
                Some(rattrapage_id),
            )
            .await?;
        v.pop().ok_or_else(introuvable)
    }

    /// EVA-174, CE-29 : **seul** cas où la note 0 est attribuée pour une absence : convocation de remplacement restée
    /// « absent non justifié », confirmation de la référence réglementaire, acte tracé.
    ///
    /// Qui : **après l'arrêt de la période de l'élève**, la direction (`notes.corriger`, avec le motif de correction) ;
    /// **période ouverte**, le **professeur du cours** (titulaire ou co-enseignant, pas un suppléant) avec un **motif**
    /// (V3d, M-4) : il n'a pas `notes.corriger`, et l'écriture commune réserve la période ouverte au professeur du cours.
    /// Dans les deux cas, l'assiduité (socle 12) est **relue au moment du zéro** (V3d, M-5) : une justification arrivée
    /// après coup, ou une absence pas encore qualifiée, bloque le zéro ; le professeur exige en plus une absence
    /// **non justifiée constatée** par l'assiduité.
    pub async fn eva_v4_attribuer_zero_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        rattrapage_id: Uuid,
        body: &AttribuerZeroBody,
    ) -> Result<ZeroAttribueDto, EntDbError> {
        // Règle 5 : personnel, droit sur le cours, élève visible, puis seulement la confirmation, la nature et l'état.
        let (acces, evaluation_id, eleve_id, nature, etat, _cree_par) = self
            .eva_v4_charger_convocation(
                etab,
                acteur,
                role,
                jour,
                rattrapage_id,
                body.version_attendue,
            )
            .await?;
        let prof_titulaire = acces.saisie && !acces.suppleant;
        if !(acces.corriger || prof_titulaire) {
            return Err(EntDbError::Forbidden);
        }
        if !body.confirme {
            return Err(refus(
                "confirmer la lecture de la référence réglementaire avant d'attribuer un zéro",
            ));
        }
        let motif_acteur = body
            .motif
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty());
        if motif_acteur.is_some_and(|m| m.chars().count() > 500) {
            return Err(refus("motif trop long (500 caractères au plus)"));
        }
        if !acces.corriger && motif_acteur.is_none_or(|m| m.chars().count() < 3) {
            return Err(refus(
                "un motif de 3 à 500 caractères est requis pour attribuer le zéro",
            ));
        }
        if nature != "remplacement" {
            return Err(refus(
                "le zéro n'existe que pour une évaluation de remplacement (R-01)",
            ));
        }
        if etat != "absent_non_justifie" {
            return Err(conflit("le zéro suppose une absence non justifiée constatée à l'évaluation de remplacement"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let deja: bool = sqlx::query_scalar(
            "SELECT zero_attribue_le IS NOT NULL FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(rattrapage_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit("le zéro a déjà été attribué"));
        }
        // M-5 : l'assiduité est relue MAINTENANT (sous le verrou de la convocation), pas au moment du constat.
        let (cours_eval, date_eval): (Uuid, NaiveDate) = sqlx::query_as(
            "SELECT cours_id, date_eval FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(evaluation_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let assiduite: Option<String> =
            sqlx::query_scalar("SELECT eva_absence_a_la_seance($1, $2, $3, $4)")
                .bind(etab)
                .bind(eleve_id)
                .bind(cours_eval)
                .bind(date_eval)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        match assiduite.as_deref() {
            Some("justifiee") => {
                return Err(conflit(
                    "l'absence a été justifiée depuis le constat : le zéro n'est plus possible (reconvoquer l'élève)",
                ))
            }
            Some("en_attente") => {
                return Err(conflit(
                    "l'absence n'est pas encore qualifiée au suivi de l'assiduité : attendre sa qualification avant le zéro",
                ))
            }
            None if !acces.corriger => {
                return Err(refus(
                    "le professeur n'attribue le zéro que sur une absence non justifiée constatée par l'assiduité",
                ))
            }
            _ => {}
        }
        let motif = match motif_acteur {
            Some(m) => format!("Évaluation de remplacement : absence non justifiée (R-01). {m}"),
            None => "Évaluation de remplacement : absence non justifiée (R-01)".to_string(),
        };
        let r = EcrireResultat {
            evaluation_id,
            eleve_id,
            statut: StatutCalcul::Note,
            valeur_m: Some(0),
            motif: Some(motif),
            commentaire_eleve: None,
            source: SourceResultat::Saisie,
        };
        // M-1 : case relue sous verrou ; une note ou une dispense saisie entre-temps n'est jamais écrasée.
        let attendu = attendu_case_remplacable(&mut tx, etab, evaluation_id, eleve_id).await?;
        let a_la_direction = acces.corriger;
        Self::ecrire_resultat_tx(&mut tx, etab, acteur, acces.corriger, jour, &r, attendu)
            .await
            .map_err(|e| match e {
                EntDbError::Forbidden if a_la_direction => refus(
                    "période de l'élève ouverte : le zéro est attribué par le professeur du cours (la direction le fait après l'arrêt)",
                ),
                EntDbError::Forbidden => refus(
                    "période de l'élève arrêtée : le zéro est attribué par la direction, avec un motif",
                ),
                autre => autre,
            })?;
        sqlx::query(
            "UPDATE eva_rattrapage SET zero_attribue_le = now(), zero_attribue_par = $3, modifie_par = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(rattrapage_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        evt_neutre(
            &mut tx,
            etab,
            types_v4::ZERO_REMPLACEMENT,
            evaluation_id,
            rattrapage_id,
            acteur,
        )
        .await?;
        let row = sqlx::query(&format!(
            "SELECT {COLONNES_RATTRAPAGE} FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(rattrapage_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(ZeroAttribueDto {
            rattrapage: dto_rattrapage(&row, jour)?,
            reference: REFERENCE_ZERO.to_string(),
        })
    }

    /// Convocations d'une évaluation (lecture du cours : professeur, direction, secrétariat en lecture).
    pub async fn eva_v4_rattrapages_evaluation_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        evaluation_id: Uuid,
    ) -> Result<Vec<RattrapageDto>, EntDbError> {
        let (_, acces) = self
            .eva_v4_droit_cours(etab, acteur, role, evaluation_id, jour)
            .await?;
        let _ = acces;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows = sqlx::query(&format!(
            "SELECT {COLONNES_RATTRAPAGE} FROM eva_rattrapage WHERE etablissement_id = $1 AND evaluation_id = $2
              ORDER BY prevu_le, eleve_id, version, id"
        ))
        .bind(etab)
        .bind(evaluation_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let ids: Vec<Uuid> = rows
            .iter()
            .filter_map(|r| r.try_get("eleve_id").ok())
            .collect();
        let caches = self.eva_v4_caches(etab, acteur, role, &ids).await?;
        rows.iter()
            .filter(|r| {
                r.try_get::<Uuid, _>("eleve_id")
                    .is_ok_and(|e| !caches.contains(&e))
            })
            .map(|r| dto_rattrapage(r, jour))
            .collect()
    }

    // ───────────────────────── Absences répétées aux évaluations (CE-27) ─────────────────────────

    /// Élèves absents, **sans absence justifiée** au socle 12, à au moins 3 évaluations de la période (EVA-169, CE-27).
    /// `cours_id` : le professeur du cours (ses évaluations) ; sans `cours_id` : `notes.lire` à portée établissement.
    pub async fn eva_v4_absences_repetees_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
        periode_id: Uuid,
        cours_id: Option<Uuid>,
    ) -> Result<Vec<AbsenceRepeteeDto>, EntDbError> {
        Self::exiger_staff(role)?;
        match cours_id {
            Some(c) => {
                let a = self.eva_v3_acces(etab, acteur, role, c, jour).await?;
                if !(a.prof_du_cours || a.saisie || a.corriger) {
                    return Err(EntDbError::Forbidden);
                }
            }
            None => {
                if !self
                    .a_action_etablissement(etab, acteur, role, Action::NotesLire)
                    .await?
                {
                    return Err(EntDbError::Forbidden);
                }
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let bornes: Option<(NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT date_debut, date_fin FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(periode_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (du, au) = bornes.ok_or_else(introuvable)?;
        // F-5 : coût borné. L'étape 1 ne regarde que les cases (aucun appel au socle 12) et retient les élèves qui ont au
        // moins `$5` cases absentes dans la période ; l'assiduité n'est interrogée que pour ces élèves-là, et la réponse est
        // plafonnée. Faux positifs : une absence justifiée l'est à la séance du cours, ou, pour une évaluation hors séance
        // (aucune séance du cours ce jour-là), par une absence justifiée du jour (séance ou demi-journée).
        let rows = sqlx::query(
            "WITH candidats AS (
                 SELECT r.eleve_id
                   FROM eva_resultat r
                   JOIN eva_evaluation e ON e.id = r.evaluation_id AND e.etablissement_id = r.etablissement_id
                  WHERE r.etablissement_id = $1 AND e.mode = 'note' AND e.etat = 'ouverte'
                    AND e.date_eval BETWEEN $2 AND $3
                    AND ($4::uuid IS NULL OR e.cours_id = $4)
                    AND r.statut IN ('absent', 'a_rattraper')
                  GROUP BY r.eleve_id
                 HAVING count(DISTINCT r.evaluation_id) >= $5
                  ORDER BY r.eleve_id
                  LIMIT $6)
             SELECT r.eleve_id, count(DISTINCT r.evaluation_id)::int AS nb,
                    EXISTS (SELECT 1 FROM eva_rattrapage z WHERE z.etablissement_id = r.etablissement_id
                               AND z.eleve_id = r.eleve_id AND z.nature = 'remplacement' AND z.etat <> 'annule') AS rempl
               FROM eva_resultat r
               JOIN candidats c ON c.eleve_id = r.eleve_id
               JOIN eva_evaluation e ON e.id = r.evaluation_id AND e.etablissement_id = r.etablissement_id
               LEFT JOIN LATERAL eva_absence_a_la_seance(r.etablissement_id, r.eleve_id, e.cours_id, e.date_eval) AS ab(v) ON true
              WHERE r.etablissement_id = $1 AND e.mode = 'note' AND e.etat = 'ouverte'
                AND e.date_eval BETWEEN $2 AND $3
                AND ($4::uuid IS NULL OR e.cours_id = $4)
                AND r.statut IN ('absent', 'a_rattraper')
                AND COALESCE(ab.v, 'sans') <> 'justifiee'
                AND NOT (ab.v IS NULL AND eva_absence_justifiee_ce_jour(r.etablissement_id, r.eleve_id, e.date_eval))
              GROUP BY r.etablissement_id, r.eleve_id
             HAVING count(DISTINCT r.evaluation_id) >= $5
              ORDER BY r.eleve_id",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .bind(cours_id)
        .bind(SEUIL_ABSENCES_REPETEES as i32)
        .bind(MAX_ABSENCES_REPETEES)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let ids: Vec<Uuid> = rows
            .iter()
            .filter_map(|r| r.try_get("eleve_id").ok())
            .collect();
        let caches = self.eva_v4_caches(etab, acteur, role, &ids).await?;
        let mut sortie = Vec::new();
        for r in &rows {
            let eleve_id: Uuid = r.try_get("eleve_id").map_err(db)?;
            if caches.contains(&eleve_id) {
                continue;
            }
            sortie.push(AbsenceRepeteeDto {
                eleve_id,
                nb_evaluations: u32::try_from(r.try_get::<i32, _>("nb").map_err(db)?).unwrap_or(0),
                remplacement_convoque: r.try_get("rempl").map_err(db)?,
            });
        }
        Ok(sortie)
    }
}
