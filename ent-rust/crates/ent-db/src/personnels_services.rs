//! Personnels, lot P3 — services d'enseignement et interventions (`06_PERSONNELS.md` §8.5, §10.4,
//! PRS-40 à PRS-49 et PRS-54 ; Q-P28, R-P3 ; migrations 354 et 362).
//!
//! Le **cours** (`ent_cours`, socle 5) dit « quoi, à qui » ; le **service** dit « qui l'assure »
//! (heures, modalité, période, état) ; l'**intervention** est la présence datée d'un agent sur un
//! service (titulaire, co-enseignant, suppléant, assistant, intervenant, observateur). Une saisie
//! par jour sauf co-enseignement : le suppléant retire la saisie au titulaire PENDANT la période, par
//! calcul (`ent_service_intervenants_a`), sans rien écrire ; le titulaire de retour retrouve tout.
//!
//! | Commande / port                               | Droit                                  |
//! |-----------------------------------------------|----------------------------------------|
//! | `creer_service_for_actor` (PRS-40)            | `services.gerer`                       |
//! | `modifier_service_for_actor` (PRS-41)         | `services.gerer`                       |
//! | `remplacer_service_for_actor` (PRS-42)        | `services.gerer`                       |
//! | `clore_`, `annuler_`, `ouvrir_service_for_actor` (PRS-43) | `services.gerer`           |
//! | `ajouter_intervenant_for_actor` (PRS-44, 45)  | `services.gerer`                       |
//! | `nommer_suppleant_for_actor` (PRS-46)         | `services.gerer`                       |
//! | `terminer_intervention_for_actor`             | `services.gerer`                       |
//! | `corriger_intervention_for_actor` (PRS-47)    | `services.gerer`                       |
//! | `remplacer_agent_for_actor` (PRS-48)          | `services.gerer`                       |
//! | `repartition_for_actor` (PRS-49)              | `personnels.services.lire`             |
//! | `mes_services_for_actor` (PRS-54)             | portée `moi` : être agent ici          |
//! | `intervenants_du_service`, `en_saisie`, `intervenants_de_l_eleve` | ports internes     |
//!
//! Chaque commande est une transaction `TenantTx` (RLS) = fait + journal + événement. Journal et
//! événements : identifiants, codes, dates ; jamais de nom ni de motif détaillé. La base garde les
//! invariants (un titulaire et un suppléant à la fois, interventions jamais supprimées, service
//! clos immuable) ; les contrôles de cohérence sont des ALERTES, jamais des blocages (§8.5).
//!
//! **Limite du lot** : `prof_user_id` reste la projection du titulaire pour les lecteurs
//! historiques ; le suppléant n'ouvre un droit sur les élèves qu'à la bascule du périmètre (lot P4).

#![allow(clippy::type_complexity)] // lignes de requête SQL lues sous forme de tuples

use std::collections::{HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::personnels_services::*;
use serde_json::{json, Value};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::dossier_personnes::aujourdhui_tx;
use crate::outbox::Publier;
use crate::personnels::{
    agent_du_user_tx, agent_tx, conflit, db, ecriture, introuvable, invalide, journal_tx,
};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

pub const MODALITES_SERVICE: &[&str] = &[
    "cours",
    "tp",
    "co_intervention",
    "accompagnement",
    "option",
    "dispositif",
    "autre",
];
pub const NATURES_INTERVENTION: &[&str] = &[
    "titulaire",
    "co_enseignant",
    "suppleant",
    "assistant",
    "intervenant",
    "stagiaire_observateur",
];
pub const DROITS_INTERVENTION: &[&str] = &["appel", "notes", "cahier", "appreciations"];
pub const MOTIFS_CORRECTION: &[&str] = &["erreur_saisie", "doublon", "autre"];
const ORIGINES_SUPPLEANCE: &[&str] = &["absence", "vacance"];

fn peut_saisir(nature: &str) -> bool {
    matches!(nature, "titulaire" | "co_enseignant" | "suppleant")
}

// ---------------------------------------------------------------------------------------------
// Événements et journal
// ---------------------------------------------------------------------------------------------

async fn publier_service_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    service_id: Uuid,
    payload: Value,
    actor: Uuid,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Referentiel,
        aggregate_id: service_id,
        eleve_id: None,
        payload,
        actor_user_id: Some(actor.to_string()),
    })
    .await?;
    Ok(())
}

fn heures_valides(h: Option<f64>, max: f64) -> Result<(), EntDbError> {
    match h {
        Some(v) if !v.is_finite() || v < 0.0 || v > max => {
            Err(invalide("Nombre d'heures hors limites."))
        }
        _ => Ok(()),
    }
}

fn code_valide(code: &str) -> Result<String, EntDbError> {
    let c = code.trim();
    if c.is_empty() || c.chars().count() > 40 {
        return Err(invalide("Code du service : 1 à 40 caractères."));
    }
    Ok(c.to_string())
}

// ---------------------------------------------------------------------------------------------
// Lignes lues
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
struct ServiceLigne {
    id: Uuid,
    annee_id: Uuid,
    cours_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
    matiere_id: Option<Uuid>,
    matiere_libelle: String,
    modalite: String,
    code: Option<String>,
    heures_hebdo: Option<f64>,
    heures_annuelles: Option<f64>,
    du: Option<NaiveDate>,
    au: Option<NaiveDate>,
    etat: String,
    source: String,
    remplace_service_id: Option<Uuid>,
}

const SERVICE_COLONNES: &str =
    "s.id, s.annee_id, s.cours_id, s.classe_id, s.groupe_id, s.matiere_id,
    coalesce(m.libelle, s.matiere_libelle) AS matiere_libelle, s.modalite, s.code,
    s.heures_hebdo::float8 AS heures_hebdo, s.heures_annuelles::float8 AS heures_annuelles,
    s.du, s.au, s.etat, s.source, s.remplace_service_id";

#[derive(Debug, Clone, sqlx::FromRow)]
struct InterventionLigne {
    id: Uuid,
    service_id: Uuid,
    agent_id: Uuid,
    agent: String,
    nature: String,
    du: NaiveDate,
    au: Option<NaiveDate>,
    heures_hebdo: Option<f64>,
    saisie: bool,
    droits: Option<Vec<String>>,
    origine: String,
    origine_ref: Option<Uuid>,
    annulee: bool,
}

/// Service de l'établissement (verrouillé si `verrou`) ; 404 uniforme sinon.
async fn service_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    service_id: Uuid,
    verrou: bool,
) -> Result<ServiceLigne, EntDbError> {
    let sql = format!(
        "SELECT {SERVICE_COLONNES} FROM ent_service_enseignement s
           LEFT JOIN ent_matiere m ON m.id = s.matiere_id AND m.etablissement_id = s.etablissement_id
          WHERE s.etablissement_id = $1 AND s.id = $2 {}",
        if verrou { "FOR UPDATE OF s" } else { "" }
    );
    sqlx::query_as::<_, ServiceLigne>(&sql)
        .bind(etab)
        .bind(service_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

async fn interventions_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    service_ids: &[Uuid],
    avec_annulees: bool,
) -> Result<Vec<InterventionLigne>, EntDbError> {
    sqlx::query_as::<_, InterventionLigne>(
        "SELECT i.id, i.service_id, i.agent_id,
                btrim(coalesce(p.prenom || ' ', '') || p.nom) AS agent,
                i.nature, i.du, i.au, i.heures_hebdo::float8 AS heures_hebdo, i.saisie, i.droits,
                i.origine, i.origine_ref, (i.annule_le IS NOT NULL) AS annulee
           FROM ent_service_intervenant i
           JOIN ent_agent a ON a.etablissement_id = i.etablissement_id AND a.id = i.agent_id
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE i.etablissement_id = $1 AND i.service_id = ANY($2) AND ($3 OR i.annule_le IS NULL)
          ORDER BY i.du, i.cree_le",
    )
    .bind(etab)
    .bind(service_ids)
    .bind(avec_annulees)
    .fetch_all(conn)
    .await
    .map_err(db)
}

/// Active à la date : non annulée, commencée, non terminée.
fn active_le(i: &InterventionLigne, jour: NaiveDate) -> bool {
    !i.annulee && i.du <= jour && i.au.is_none_or(|a| a >= jour)
}

/// Même règle que `ent_service_intervenants_a` : un suppléant actif retire la saisie au titulaire.
fn saisie_effective(i: &InterventionLigne, toutes: &[&InterventionLigne], jour: NaiveDate) -> bool {
    if !active_le(i, jour) || !i.saisie {
        return false;
    }
    if i.nature == "titulaire" {
        let suppleant = toutes
            .iter()
            .any(|o| o.nature == "suppleant" && o.saisie && active_le(o, jour));
        return !suppleant;
    }
    true
}

fn dto_intervention(
    i: &InterventionLigne,
    toutes: &[&InterventionLigne],
    jour: NaiveDate,
) -> InterventionDto {
    InterventionDto {
        id: i.id,
        agent_id: i.agent_id,
        agent: i.agent.clone(),
        nature: i.nature.clone(),
        du: i.du,
        au: i.au,
        heures_hebdo: i.heures_hebdo,
        saisie: i.saisie,
        saisie_effective: saisie_effective(i, toutes, jour),
        droits: i.droits.clone(),
        origine: i.origine.clone(),
        origine_ref: i.origine_ref,
        annulee: i.annulee,
        active: active_le(i, jour),
    }
}

/// Alertes de cohérence (jamais bloquantes) pour un ensemble de services, à une date.
async fn alertes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    service_ids: &[Uuid],
    jour: NaiveDate,
) -> Result<HashMap<Uuid, Vec<AlerteServiceDto>>, EntDbError> {
    let mut out: HashMap<Uuid, Vec<AlerteServiceDto>> = HashMap::new();
    // Agent intervenant hors de sa période d'engagement.
    let hors: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT DISTINCT i.service_id, i.agent_id
           FROM ent_service_intervenant i
          WHERE i.etablissement_id = $1 AND i.service_id = ANY($2)
            AND i.annule_le IS NULL AND (i.au IS NULL OR i.au >= $3)
            AND NOT EXISTS (
                SELECT 1 FROM ent_agent_engagement e
                 WHERE e.etablissement_id = i.etablissement_id AND e.agent_id = i.agent_id
                   AND e.annule_le IS NULL
                   AND e.du <= greatest(i.du, $3) AND (e.au IS NULL OR e.au >= greatest(i.du, $3)))",
    )
    .bind(etab)
    .bind(service_ids)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (s, a) in hors {
        out.entry(s).or_default().push(AlerteServiceDto {
            code: "hors_engagement".into(),
            service_id: Some(s),
            agent_id: Some(a),
        });
    }
    // Doublon probable : même cours, même modalité, même code, un autre service vivant.
    let doublons: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT s.id FROM ent_service_enseignement s
          WHERE s.etablissement_id = $1 AND s.id = ANY($2) AND s.etat IN ('prevu', 'ouvert')
            AND s.cours_id IS NOT NULL
            AND EXISTS (SELECT 1 FROM ent_service_enseignement o
                         WHERE o.etablissement_id = s.etablissement_id AND o.id <> s.id
                           AND o.cours_id = s.cours_id AND o.modalite = s.modalite
                           AND coalesce(o.code, '') = coalesce(s.code, '') AND o.etat IN ('prevu', 'ouvert'))",
    )
    .bind(etab)
    .bind(service_ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (s,) in doublons {
        out.entry(s).or_default().push(AlerteServiceDto {
            code: "doublon_probable".into(),
            service_id: Some(s),
            agent_id: None,
        });
    }
    // Vacants (services ouverts sans saisie ce jour).
    let vacants: Vec<(Uuid,)> =
        sqlx::query_as("SELECT service_id FROM ent_services_vacants($1, $2)")
            .bind(etab)
            .bind(jour)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
    let demandes: HashSet<Uuid> = service_ids.iter().copied().collect();
    for (s,) in vacants {
        if demandes.contains(&s) {
            out.entry(s).or_default().push(AlerteServiceDto {
                code: "vacant".into(),
                service_id: Some(s),
                agent_id: None,
            });
        }
    }
    Ok(out)
}

async fn construire_dtos_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    lignes: Vec<ServiceLigne>,
    jour: NaiveDate,
    avec_annulees: bool,
) -> Result<Vec<ServiceDto>, EntDbError> {
    let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let interventions = interventions_tx(conn, etab, &ids, avec_annulees).await?;
    let mut par_service: HashMap<Uuid, Vec<&InterventionLigne>> = HashMap::new();
    for i in &interventions {
        par_service.entry(i.service_id).or_default().push(i);
    }
    let mut alertes = alertes_tx(conn, etab, &ids, jour).await?;
    Ok(lignes
        .into_iter()
        .map(|l| {
            let toutes = par_service.remove(&l.id).unwrap_or_default();
            let al = alertes.remove(&l.id).unwrap_or_default();
            let vacant = al.iter().any(|a| a.code == "vacant");
            ServiceDto {
                id: l.id,
                annee_id: l.annee_id,
                cours_id: l.cours_id,
                classe_id: l.classe_id,
                groupe_id: l.groupe_id,
                matiere_id: l.matiere_id,
                matiere_libelle: l.matiere_libelle,
                modalite: l.modalite,
                code: l.code,
                heures_hebdo: l.heures_hebdo,
                heures_annuelles: l.heures_annuelles,
                du: l.du,
                au: l.au,
                etat: l.etat,
                source: l.source,
                remplace_service_id: l.remplace_service_id,
                vacant,
                intervenants: toutes
                    .iter()
                    .map(|i| dto_intervention(i, &toutes, jour))
                    .collect(),
                alertes: al,
            }
        })
        .collect())
}

async fn service_dto_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    service_id: Uuid,
    jour: NaiveDate,
) -> Result<ServiceDto, EntDbError> {
    let l = service_tx(conn, etab, service_id, false).await?;
    Ok(construire_dtos_tx(conn, etab, vec![l], jour, true)
        .await?
        .remove(0))
}

/// Le service est-il vacant à cette date ? (ouvert, dans sa période, sans intervention en saisie)
async fn vacant_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    service_id: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_services_vacants($1, $2) v WHERE v.service_id = $3)",
    )
    .bind(etab)
    .bind(jour)
    .bind(service_id)
    .fetch_one(conn)
    .await
    .map_err(db)
}

/// Émet `service.vacant` / `service.pourvu` quand la vacance a changé pendant la commande.
async fn signaler_vacance_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    service_id: Uuid,
    jour: NaiveDate,
    avant: bool,
    actor: Uuid,
) -> Result<(), EntDbError> {
    let apres = vacant_tx(conn, etab, service_id, jour).await?;
    if apres != avant {
        publier_service_tx(
            conn,
            etab,
            if apres {
                types::SERVICE_VACANT
            } else {
                types::SERVICE_POURVU
            },
            service_id,
            json!({ "service_id": service_id, "depuis": jour }),
            actor,
        )
        .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Écritures partagées
// ---------------------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
async fn inserer_intervention_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    service_id: Uuid,
    agent_id: Uuid,
    nature: &str,
    du: NaiveDate,
    au: Option<NaiveDate>,
    heures: Option<f64>,
    droits: Option<&[String]>,
    origine: &str,
    origine_ref: Option<Uuid>,
    remplace: Option<Uuid>,
) -> Result<Uuid, EntDbError> {
    if !NATURES_INTERVENTION.contains(&nature) {
        return Err(invalide("Nature d'intervention inconnue."));
    }
    if au.is_some_and(|a| a < du) {
        return Err(invalide("La fin précède le début."));
    }
    heures_valides(heures, 40.0)?;
    if let Some(d) = droits {
        if d.iter().any(|x| !DROITS_INTERVENTION.contains(&x.as_str())) {
            return Err(invalide("Droit de saisie inconnu."));
        }
    }
    // L'agent doit exister ici et ne pas avoir été fusionné dans un autre.
    let agent = agent_tx(conn, etab, agent_id, false).await?;
    if agent.fusionne_dans_agent_id.is_some() {
        return Err(conflit("Cet agent a été fusionné dans un autre."));
    }
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_service_intervenant
           (id, etablissement_id, service_id, agent_id, nature, du, au, heures_hebdo, saisie, droits,
            origine, origine_ref, remplace_intervention_id, cree_par_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8::float8::numeric(4,2),$9,$10,$11,$12,$13,$14)",
    )
    .bind(id)
    .bind(etab)
    .bind(service_id)
    .bind(agent_id)
    .bind(nature)
    .bind(du)
    .bind(au)
    .bind(heures)
    .bind(peut_saisir(nature))
    .bind(droits)
    .bind(origine)
    .bind(origine_ref)
    .bind(remplace)
    .bind(actor)
    .execute(conn)
    .await
    .map_err(ecriture)?;
    Ok(id)
}

/// Termine à `au` les interventions encore actives après cette date ; celles qui n'ont pas commencé
/// sont annulées (motif `autre`) : une intervention ne se supprime jamais.
async fn arreter_interventions_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    service_id: Uuid,
    au: NaiveDate,
    actor: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_service_intervenant
            SET annule_le = now(), annule_motif = 'autre', annule_par_user_id = $4
          WHERE etablissement_id = $1 AND service_id = $2 AND annule_le IS NULL AND du > $3",
    )
    .bind(etab)
    .bind(service_id)
    .bind(au)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(ecriture)?;
    sqlx::query(
        "UPDATE ent_service_intervenant SET au = $3
          WHERE etablissement_id = $1 AND service_id = $2 AND annule_le IS NULL
            AND du <= $3 AND (au IS NULL OR au > $3)",
    )
    .bind(etab)
    .bind(service_id)
    .bind(au)
    .execute(conn)
    .await
    .map_err(ecriture)?;
    Ok(())
}

async fn intervention_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<InterventionLigne, EntDbError> {
    sqlx::query_as::<_, InterventionLigne>(
        "SELECT i.id, i.service_id, i.agent_id, '' AS agent, i.nature, i.du, i.au,
                i.heures_hebdo::float8 AS heures_hebdo, i.saisie, i.droits, i.origine, i.origine_ref,
                (i.annule_le IS NOT NULL) AS annulee
           FROM ent_service_intervenant i
          WHERE i.etablissement_id = $1 AND i.id = $2 FOR UPDATE OF i",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

async fn compte_saisies_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    service_id: Uuid,
) -> Result<i64, EntDbError> {
    sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM ent_devoir d WHERE d.etablissement_id = $1 AND d.service_id = $2)
              + (SELECT count(*) FROM ent_edt_creneau c WHERE c.etablissement_id = $1 AND c.service_id = $2)
              + (SELECT count(*) FROM ent_edt_seance s WHERE s.etablissement_id = $1 AND s.service_id = $2)",
    )
    .bind(etab)
    .bind(service_id)
    .fetch_one(conn)
    .await
    .map_err(db)
}

fn exiger_modifiable(s: &ServiceLigne) -> Result<(), EntDbError> {
    if matches!(s.etat.as_str(), "clos" | "annule") {
        return Err(conflit(
            "Ce service est clos ou annulé : il ne se modifie plus.",
        ));
    }
    Ok(())
}

impl PgEnt {
    async fn exiger_services_gerer(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::ServicesGerer)
            .await
    }

    // -----------------------------------------------------------------------------------------
    // PRS-40 : créer un service
    // -----------------------------------------------------------------------------------------
    pub async fn creer_service_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &CreerServiceBody,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        let modalite = body.modalite.as_deref().unwrap_or("cours");
        if !MODALITES_SERVICE.contains(&modalite) {
            return Err(invalide("Modalité inconnue."));
        }
        let etat = body.etat.as_deref().unwrap_or("ouvert");
        if !matches!(etat, "ouvert" | "prevu") {
            return Err(invalide("Un service se crée ouvert ou prévu."));
        }
        heures_valides(body.heures_hebdo, 40.0)?;
        heures_valides(body.heures_annuelles, 2000.0)?;
        let code = body.code.as_deref().map(code_valide).transpose()?;
        if let (Some(d), Some(a)) = (body.du, body.au) {
            if a < d {
                return Err(invalide("La fin précède le début."));
            }
        }
        let libelle = body.matiere_libelle.as_deref().map(str::trim).unwrap_or("");
        if libelle.chars().count() > 160 {
            return Err(invalide("Libellé de matière : 160 caractères au plus."));
        }
        if body.cours_id.is_none() {
            if body.annee_id.is_none() || (body.classe_id.is_some() == body.groupe_id.is_some()) {
                return Err(invalide(
                    "Précisez un cours, ou une année et une classe OU un groupe.",
                ));
            }
            if body.matiere_id.is_none() && libelle.is_empty() {
                return Err(invalide("Précisez la matière."));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let (annee_id, cours_id) = if let Some(cid) = body.cours_id {
            let c: Option<(Uuid, Option<NaiveDate>)> = sqlx::query_as(
                "SELECT annee_id, ferme_le FROM ent_cours WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(cid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let (annee, ferme) = c.ok_or_else(introuvable)?;
            if ferme.is_some() {
                return Err(conflit("Ce cours est fermé."));
            }
            (annee, Some(cid))
        } else {
            (body.annee_id.unwrap_or_default(), None)
        };
        let cloturee: Option<(Option<chrono::DateTime<chrono::Utc>>, NaiveDate)> = sqlx::query_as(
            "SELECT cloturee_le, date_debut FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (cloturee_le, debut_annee) = cloturee.ok_or_else(introuvable)?;
        if cloturee_le.is_some() {
            return Err(conflit("L'année est clôturée."));
        }
        if let Some(m) = body.matiere_id {
            let ok: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_matiere WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(m)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        for (table, id) in [
            ("ent_classe", body.classe_id),
            ("ent_groupe", body.groupe_id),
        ] {
            if let Some(id) = id {
                let ok: bool = sqlx::query_scalar(&format!(
                    "SELECT EXISTS (SELECT 1 FROM {table} WHERE etablissement_id = $1 AND id = $2)"
                ))
                .bind(etab)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if !ok {
                    return Err(introuvable());
                }
            }
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_service_enseignement
               (id, etablissement_id, annee_id, cours_id, matiere_id, matiere_libelle, classe_id, groupe_id,
                modalite, code, heures_hebdo, heures_annuelles, du, au, etat, source)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11::float8::numeric(4,2),$12::float8::numeric(6,2),$13,$14,$15,'saisie')",
        )
        .bind(id)
        .bind(etab)
        .bind(annee_id)
        .bind(cours_id)
        .bind(body.matiere_id)
        .bind(libelle)
        .bind(body.classe_id)
        .bind(body.groupe_id)
        .bind(modalite)
        .bind(code.as_deref())
        .bind(body.heures_hebdo)
        .bind(body.heures_annuelles)
        .bind(body.du)
        .bind(body.au)
        .bind(etat)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        if let Some(t) = &body.titulaire {
            let du = t.du.or(body.du).unwrap_or(debut_annee);
            let iid = inserer_intervention_tx(
                &mut tx,
                etab,
                actor,
                id,
                t.agent_id,
                "titulaire",
                du,
                None,
                t.heures_hebdo,
                None,
                "saisie",
                None,
                None,
            )
            .await?;
            let payload = json!({ "service_id": id, "intervention_id": iid, "agent_id": t.agent_id,
                                  "nature": "titulaire", "du": du });
            journal_tx(
                &mut tx,
                etab,
                "service_intervenant_ajoute",
                "Intervenant ajouté à un service",
                payload.clone(),
                actor,
            )
            .await?;
            publier_service_tx(
                &mut tx,
                etab,
                types::SERVICE_INTERVENANT_AJOUTE,
                id,
                payload,
                actor,
            )
            .await?;
        }
        let s = service_tx(&mut tx, etab, id, false).await?;
        journal_tx(
            &mut tx,
            etab,
            "service_cree",
            "Service d'enseignement créé",
            json!({ "service_id": id, "cours_id": s.cours_id, "modalite": modalite, "etat": etat }),
            actor,
        )
        .await?;
        publier_service_tx(
            &mut tx,
            etab,
            types::SERVICE_CREE,
            id,
            json!({ "service_id": id, "annee_id": s.annee_id, "cours_id": s.cours_id,
                    "classe_id": s.classe_id, "groupe_id": s.groupe_id, "matiere_id": s.matiere_id,
                    "du": s.du, "au": s.au, "etat": etat }),
            actor,
        )
        .await?;
        if etat == "ouvert" && vacant_tx(&mut tx, etab, id, jour).await? {
            publier_service_tx(
                &mut tx,
                etab,
                types::SERVICE_VACANT,
                id,
                json!({ "service_id": id, "depuis": jour }),
                actor,
            )
            .await?;
        }
        let dto = service_dto_tx(&mut tx, etab, id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // PRS-41 : modifier les heures, la période, la modalité
    // -----------------------------------------------------------------------------------------
    pub async fn modifier_service_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        service_id: Uuid,
        body: &ModifierServiceBody,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        if let Some(m) = body.modalite.as_deref() {
            if !MODALITES_SERVICE.contains(&m) {
                return Err(invalide("Modalité inconnue."));
            }
        }
        heures_valides(body.heures_hebdo, 40.0)?;
        heures_valides(body.heures_annuelles, 2000.0)?;
        let code = body.code.as_deref().map(code_valide).transpose()?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let s = service_tx(&mut tx, etab, service_id, true).await?;
        exiger_modifiable(&s)?;
        let du = if body.effacer_du {
            None
        } else {
            body.du.or(s.du)
        };
        let au = if body.effacer_au {
            None
        } else {
            body.au.or(s.au)
        };
        if let (Some(d), Some(a)) = (du, au) {
            if a < d {
                return Err(invalide("La fin précède le début."));
            }
        }
        let code_final = if body.effacer_code {
            None
        } else {
            code.or(s.code.clone())
        };
        let avant = vacant_tx(&mut tx, etab, service_id, jour).await?;
        sqlx::query(
            "UPDATE ent_service_enseignement
                SET modalite = $3, code = $4, heures_hebdo = $5::float8::numeric(4,2),
                    heures_annuelles = $6::float8::numeric(6,2), du = $7, au = $8
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(service_id)
        .bind(body.modalite.as_deref().unwrap_or(&s.modalite))
        .bind(code_final.as_deref())
        .bind(body.heures_hebdo.or(s.heures_hebdo))
        .bind(body.heures_annuelles.or(s.heures_annuelles))
        .bind(du)
        .bind(au)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let mut champs: Vec<&str> = Vec::new();
        if body.modalite.is_some() {
            champs.push("modalite");
        }
        if body.code.is_some() || body.effacer_code {
            champs.push("code");
        }
        if body.heures_hebdo.is_some() || body.heures_annuelles.is_some() {
            champs.push("heures");
        }
        if body.du.is_some() || body.au.is_some() || body.effacer_du || body.effacer_au {
            champs.push("periode");
        }
        journal_tx(
            &mut tx,
            etab,
            "service_modifie",
            "Service d'enseignement modifié",
            json!({ "service_id": service_id, "champs": champs }),
            actor,
        )
        .await?;
        publier_service_tx(
            &mut tx,
            etab,
            types::SERVICE_MODIFIE,
            service_id,
            json!({ "service_id": service_id, "champs": champs }),
            actor,
        )
        .await?;
        signaler_vacance_tx(&mut tx, etab, service_id, jour, avant, actor).await?;
        let dto = service_dto_tx(&mut tx, etab, service_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // PRS-42 : changer la cible ou la matière en cours d'année
    // -----------------------------------------------------------------------------------------
    pub async fn remplacer_service_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        service_id: Uuid,
        body: &RemplacerServiceBody,
    ) -> Result<ServiceRemplaceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        if body.classe_id.is_some() && body.groupe_id.is_some() {
            return Err(invalide("Une classe OU un groupe."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let ancien = service_tx(&mut tx, etab, service_id, true).await?;
        if ancien.etat != "ouvert" {
            return Err(conflit("Seul un service ouvert peut être remplacé."));
        }
        if ancien.du.is_some_and(|d| body.a_partir_du <= d) {
            return Err(invalide(
                "La date de remplacement doit suivre le début du service.",
            ));
        }
        let veille = body.a_partir_du - Duration::days(1);
        let (classe, groupe) = if body.classe_id.is_some() || body.groupe_id.is_some() {
            (body.classe_id, body.groupe_id)
        } else {
            (ancien.classe_id, ancien.groupe_id)
        };
        let (matiere, libelle) = if body.matiere_id.is_some() || body.matiere_libelle.is_some() {
            (
                body.matiere_id,
                body.matiere_libelle
                    .clone()
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            )
        } else {
            (ancien.matiere_id, ancien.matiere_libelle.clone())
        };
        if classe == ancien.classe_id
            && groupe == ancien.groupe_id
            && matiere == ancien.matiere_id
            && (matiere.is_some() || libelle.eq_ignore_ascii_case(&ancien.matiere_libelle))
        {
            return Err(invalide(
                "Rien ne change : indiquez une autre cible ou une autre matière.",
            ));
        }
        let nouveau_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_service_enseignement
               (id, etablissement_id, annee_id, matiere_id, matiere_libelle, classe_id, groupe_id,
                modalite, code, heures_hebdo, heures_annuelles, du, au, etat, source, remplace_service_id)
             SELECT $3, s.etablissement_id, s.annee_id, $4, $5, $6, $7, s.modalite, s.code,
                    s.heures_hebdo, s.heures_annuelles, $8, s.au, 'ouvert', 'saisie', s.id
               FROM ent_service_enseignement s WHERE s.etablissement_id = $1 AND s.id = $2",
        )
        .bind(etab)
        .bind(service_id)
        .bind(nouveau_id)
        .bind(matiere)
        .bind(&libelle)
        .bind(classe)
        .bind(groupe)
        .bind(body.a_partir_du)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        if body.reprendre_intervenants {
            let actives: Vec<InterventionLigne> =
                interventions_tx(&mut tx, etab, &[service_id], false)
                    .await?
                    .into_iter()
                    .filter(|i| {
                        i.au.is_none_or(|a| a >= body.a_partir_du) && i.nature != "suppleant"
                    })
                    .collect();
            for i in actives {
                inserer_intervention_tx(
                    &mut tx,
                    etab,
                    actor,
                    nouveau_id,
                    i.agent_id,
                    &i.nature,
                    i.du.max(body.a_partir_du),
                    i.au,
                    i.heures_hebdo,
                    i.droits.as_deref(),
                    "reconduction",
                    None,
                    None,
                )
                .await?;
            }
        }
        arreter_interventions_tx(&mut tx, etab, service_id, veille, actor).await?;
        sqlx::query(
            "UPDATE ent_service_enseignement SET au = $3, etat = 'clos', etat_par_user_id = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(service_id)
        .bind(veille)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let devoirs: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_devoir WHERE etablissement_id = $1 AND service_id = $2",
        )
        .bind(etab)
        .bind(service_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            "service_remplace",
            "Service d'enseignement remplacé (nouvelle cible ou matière)",
            json!({ "ancien_id": service_id, "nouveau_id": nouveau_id, "a_partir_du": body.a_partir_du }),
            actor,
        )
        .await?;
        publier_service_tx(
            &mut tx,
            etab,
            types::SERVICE_REMPLACE,
            nouveau_id,
            json!({ "ancien_id": service_id, "nouveau_id": nouveau_id, "a_partir_du": body.a_partir_du }),
            actor,
        )
        .await?;
        let nouveau = service_dto_tx(&mut tx, etab, nouveau_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(ServiceRemplaceDto {
            ancien_id: service_id,
            nouveau,
            devoirs_a_rattacher: devoirs,
        })
    }

    // -----------------------------------------------------------------------------------------
    // PRS-43 : clore, annuler (et ouvrir un service prévu)
    // -----------------------------------------------------------------------------------------
    pub async fn clore_service_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        service_id: Uuid,
        body: &CloreServiceBody,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let s = service_tx(&mut tx, etab, service_id, true).await?;
        if s.etat != "ouvert" {
            return Err(conflit(
                "Seul un service ouvert se clôt (un service prévu s'annule).",
            ));
        }
        let au = body.au.unwrap_or(jour);
        if s.du.is_some_and(|d| au < d) {
            return Err(invalide("La fin précède le début du service."));
        }
        arreter_interventions_tx(&mut tx, etab, service_id, au, actor).await?;
        sqlx::query(
            "UPDATE ent_service_enseignement SET au = $3, etat = 'clos', etat_par_user_id = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(service_id)
        .bind(au)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        journal_tx(
            &mut tx,
            etab,
            "service_clos",
            "Service d'enseignement clos",
            json!({ "service_id": service_id, "au": au }),
            actor,
        )
        .await?;
        publier_service_tx(
            &mut tx,
            etab,
            types::SERVICE_CLOS,
            service_id,
            json!({ "service_id": service_id, "au": au }),
            actor,
        )
        .await?;
        let dto = service_dto_tx(&mut tx, etab, service_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Annuler un service : seulement s'il n'a AUCUNE saisie (évaluation, créneau, séance).
    pub async fn annuler_service_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        service_id: Uuid,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let s = service_tx(&mut tx, etab, service_id, true).await?;
        exiger_modifiable(&s)?;
        if compte_saisies_tx(&mut tx, etab, service_id).await? > 0 {
            return Err(conflit(
                "Ce service a déjà des évaluations, des créneaux ou des séances : clôturez-le plutôt.",
            ));
        }
        let avant = vacant_tx(&mut tx, etab, service_id, jour).await?;
        sqlx::query(
            "UPDATE ent_service_intervenant
                SET annule_le = now(), annule_motif = 'autre', annule_par_user_id = $3
              WHERE etablissement_id = $1 AND service_id = $2 AND annule_le IS NULL",
        )
        .bind(etab)
        .bind(service_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        sqlx::query(
            "UPDATE ent_service_enseignement SET etat = 'annule', etat_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(service_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        journal_tx(
            &mut tx,
            etab,
            "service_annule",
            "Service d'enseignement annulé",
            json!({ "service_id": service_id }),
            actor,
        )
        .await?;
        publier_service_tx(
            &mut tx,
            etab,
            types::SERVICE_ANNULE,
            service_id,
            json!({ "service_id": service_id }),
            actor,
        )
        .await?;
        let _ = avant;
        let dto = service_dto_tx(&mut tx, etab, service_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Un service prévu devient ouvert (ses titulaires deviennent visibles des lecteurs historiques).
    pub async fn ouvrir_service_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        service_id: Uuid,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let s = service_tx(&mut tx, etab, service_id, true).await?;
        if s.etat != "prevu" {
            return Err(conflit("Seul un service prévu s'ouvre."));
        }
        sqlx::query(
            "UPDATE ent_service_enseignement SET etat = 'ouvert', etat_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(service_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        journal_tx(
            &mut tx,
            etab,
            "service_ouvert",
            "Service d'enseignement ouvert",
            json!({ "service_id": service_id }),
            actor,
        )
        .await?;
        publier_service_tx(
            &mut tx,
            etab,
            types::SERVICE_OUVERT,
            service_id,
            json!({ "service_id": service_id, "annee_id": s.annee_id }),
            actor,
        )
        .await?;
        if vacant_tx(&mut tx, etab, service_id, jour).await? {
            publier_service_tx(
                &mut tx,
                etab,
                types::SERVICE_VACANT,
                service_id,
                json!({ "service_id": service_id, "depuis": jour }),
                actor,
            )
            .await?;
        }
        let dto = service_dto_tx(&mut tx, etab, service_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // PRS-44, PRS-45 : titulaire, co-enseignant, assistant, intervenant, observateur
    // -----------------------------------------------------------------------------------------
    pub async fn ajouter_intervenant_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        service_id: Uuid,
        body: &AjouterIntervenantBody,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        if body.nature == "suppleant" {
            return Err(invalide(
                "Un suppléant se nomme par la suppléance (une saisie par jour).",
            ));
        }
        if !NATURES_INTERVENTION.contains(&body.nature.as_str()) {
            return Err(invalide("Nature d'intervention inconnue."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let s = service_tx(&mut tx, etab, service_id, true).await?;
        exiger_modifiable(&s)?;
        let debut_annee: NaiveDate = sqlx::query_scalar(
            "SELECT date_debut FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(s.annee_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let du = match body.du {
            Some(d) => d,
            None if body.nature == "titulaire" => {
                let deja: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_service_intervenant
                      WHERE etablissement_id = $1 AND service_id = $2 AND nature = 'titulaire' AND annule_le IS NULL)",
                )
                .bind(etab)
                .bind(service_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if deja {
                    jour
                } else {
                    s.du.unwrap_or(debut_annee)
                }
            }
            None => jour,
        };
        let avant = vacant_tx(&mut tx, etab, service_id, jour).await?;
        let iid = inserer_intervention_tx(
            &mut tx,
            etab,
            actor,
            service_id,
            body.agent_id,
            &body.nature,
            du,
            body.au,
            body.heures_hebdo,
            body.droits.as_deref(),
            "saisie",
            None,
            None,
        )
        .await?;
        journal_tx(
            &mut tx,
            etab,
            "service_intervenant_ajoute",
            "Intervenant ajouté à un service",
            json!({ "service_id": service_id, "intervention_id": iid, "agent_id": body.agent_id,
                    "nature": body.nature, "du": du, "au": body.au }),
            actor,
        )
        .await?;
        publier_service_tx(
            &mut tx,
            etab,
            types::SERVICE_INTERVENANT_AJOUTE,
            service_id,
            json!({ "service_id": service_id, "intervention_id": iid, "agent_id": body.agent_id,
                    "nature": body.nature, "du": du, "au": body.au }),
            actor,
        )
        .await?;
        signaler_vacance_tx(&mut tx, etab, service_id, jour, avant, actor).await?;
        let dto = service_dto_tx(&mut tx, etab, service_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // PRS-46 : nommer un suppléant
    // -----------------------------------------------------------------------------------------
    pub async fn nommer_suppleant_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        service_id: Uuid,
        body: &NommerSuppleantBody,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        if let Some(o) = body.origine.as_deref() {
            if !ORIGINES_SUPPLEANCE.contains(&o) {
                return Err(invalide("Origine de suppléance inconnue."));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let s = service_tx(&mut tx, etab, service_id, true).await?;
        exiger_modifiable(&s)?;
        let titulaire: Option<Uuid> = sqlx::query_scalar(
            "SELECT agent_id FROM ent_service_intervenant
              WHERE etablissement_id = $1 AND service_id = $2 AND nature = 'titulaire' AND annule_le IS NULL
                AND du <= $3 AND (au IS NULL OR au >= $3)",
        )
        .bind(etab)
        .bind(service_id)
        .bind(body.du)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let origine = body.origine.as_deref().unwrap_or(if titulaire.is_some() {
            "absence"
        } else {
            "vacance"
        });
        let avant = vacant_tx(&mut tx, etab, service_id, jour).await?;
        let iid = inserer_intervention_tx(
            &mut tx,
            etab,
            actor,
            service_id,
            body.agent_id,
            "suppleant",
            body.du,
            body.au,
            None,
            None,
            origine,
            body.origine_ref,
            None,
        )
        .await?;
        let payload = json!({ "service_id": service_id, "intervention_id": iid,
            "suppleant_agent_id": body.agent_id, "titulaire_agent_id": titulaire,
            "du": body.du, "au": body.au, "origine": origine, "origine_ref": body.origine_ref });
        journal_tx(
            &mut tx,
            etab,
            "service_suppleance_ouverte",
            "Suppléance ouverte",
            payload.clone(),
            actor,
        )
        .await?;
        publier_service_tx(
            &mut tx,
            etab,
            types::SERVICE_SUPPLEANCE_OUVERTE,
            service_id,
            payload,
            actor,
        )
        .await?;
        signaler_vacance_tx(&mut tx, etab, service_id, jour, avant, actor).await?;
        let dto = service_dto_tx(&mut tx, etab, service_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // Terminer une intervention (fin de suppléance, départ d'un co-enseignant…)
    // -----------------------------------------------------------------------------------------
    pub async fn terminer_intervention_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        intervention_id: Uuid,
        body: &TerminerInterventionBody,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let i = intervention_tx(&mut tx, etab, intervention_id).await?;
        if i.annulee {
            return Err(conflit("Cette intervention a été annulée."));
        }
        let au = body.au.unwrap_or(jour);
        if au < i.du {
            return Err(invalide("La fin précède le début de l'intervention."));
        }
        let avant = vacant_tx(&mut tx, etab, i.service_id, jour).await?;
        sqlx::query(
            "UPDATE ent_service_intervenant SET au = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(intervention_id)
        .bind(au)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let (type_name, action, libelle) = if i.nature == "suppleant" {
            (
                types::SERVICE_SUPPLEANCE_CLOSE,
                "service_suppleance_close",
                "Suppléance close",
            )
        } else {
            (
                types::SERVICE_INTERVENANT_TERMINE,
                "service_intervenant_termine",
                "Intervention terminée",
            )
        };
        let payload = json!({ "service_id": i.service_id, "intervention_id": intervention_id,
            "agent_id": i.agent_id, "nature": i.nature, "au": au });
        journal_tx(&mut tx, etab, action, libelle, payload.clone(), actor).await?;
        publier_service_tx(&mut tx, etab, type_name, i.service_id, payload, actor).await?;
        signaler_vacance_tx(&mut tx, etab, i.service_id, jour, avant, actor).await?;
        let dto = service_dto_tx(&mut tx, etab, i.service_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // PRS-47 : corriger une intervention erronée (annulation + remplacement, rien n'est effacé)
    // -----------------------------------------------------------------------------------------
    pub async fn corriger_intervention_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        intervention_id: Uuid,
        body: &CorrigerInterventionBody,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        if !MOTIFS_CORRECTION.contains(&body.motif.as_str()) {
            return Err(invalide("Motif de correction inconnu."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let i = intervention_tx(&mut tx, etab, intervention_id).await?;
        if i.annulee {
            return Err(conflit("Cette intervention est déjà annulée."));
        }
        let avant = vacant_tx(&mut tx, etab, i.service_id, jour).await?;
        sqlx::query(
            "UPDATE ent_service_intervenant
                SET annule_le = now(), annule_motif = $3, annule_par_user_id = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(intervention_id)
        .bind(&body.motif)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let nouvelle = match &body.remplacement {
            Some(r) => Some(
                inserer_intervention_tx(
                    &mut tx,
                    etab,
                    actor,
                    i.service_id,
                    r.agent_id,
                    &r.nature,
                    r.du,
                    r.au,
                    r.heures_hebdo,
                    None,
                    "saisie",
                    None,
                    Some(intervention_id),
                )
                .await?,
            ),
            None => None,
        };
        // Pas de motif détaillé ni de nom : identifiants et code seulement.
        let payload = json!({ "service_id": i.service_id, "annulee": intervention_id,
            "nouvelle": nouvelle, "motif": body.motif });
        journal_tx(
            &mut tx,
            etab,
            "service_intervention_corrigee",
            "Intervention corrigée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_service_tx(
            &mut tx,
            etab,
            types::SERVICE_INTERVENTION_CORRIGEE,
            i.service_id,
            payload,
            actor,
        )
        .await?;
        signaler_vacance_tx(&mut tx, etab, i.service_id, jour, avant, actor).await?;
        let dto = service_dto_tx(&mut tx, etab, i.service_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // PRS-48 : remplacer un agent sur tous ses services d'un coup (aperçu ou application)
    // -----------------------------------------------------------------------------------------
    pub async fn remplacer_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &RemplacerAgentBody,
    ) -> Result<RemplacementAgentDto, EntDbError> {
        self.exiger_services_gerer(etab, actor, role).await?;
        if body.agent_sortant_id == body.agent_entrant_id {
            return Err(invalide("Le successeur doit être un autre agent."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        agent_tx(&mut tx, etab, body.agent_sortant_id, false).await?;
        let entrant = agent_tx(&mut tx, etab, body.agent_entrant_id, false).await?;
        if entrant.fusionne_dans_agent_id.is_some() {
            return Err(conflit("Le successeur a été fusionné dans un autre agent."));
        }
        let cibles: Vec<(Uuid, Uuid, String, NaiveDate, Option<NaiveDate>, Option<f64>, bool, Option<Vec<String>>)> =
            sqlx::query_as(
                "SELECT i.id, i.service_id, i.nature, i.du, i.au, i.heures_hebdo::float8, i.saisie, i.droits
                   FROM ent_service_intervenant i
                   JOIN ent_service_enseignement s ON s.etablissement_id = i.etablissement_id AND s.id = i.service_id
                  WHERE i.etablissement_id = $1 AND i.agent_id = $2 AND i.annule_le IS NULL
                    AND (i.au IS NULL OR i.au >= $3) AND s.etat IN ('prevu', 'ouvert')
                    AND ($4::uuid[] IS NULL OR i.service_id = ANY($4))
                  ORDER BY i.service_id, i.du",
            )
            .bind(etab)
            .bind(body.agent_sortant_id)
            .bind(body.a_partir_du)
            .bind(body.service_ids.as_deref())
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut lignes = Vec::new();
        let mut vacances: Vec<(Uuid, bool)> = Vec::new();
        for (iid, sid, nature, du, au, heures, _saisie, droits) in cibles {
            let debut = du.max(body.a_partir_du);
            // Le successeur intervient-il déjà sur ce service à ces dates ?
            let deja: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_service_intervenant x
                  WHERE x.etablissement_id = $1 AND x.service_id = $2 AND x.agent_id = $3
                    AND x.annule_le IS NULL AND daterange(x.du, x.au, '[]') && daterange($4, $5, '[]'))",
            )
            .bind(etab)
            .bind(sid)
            .bind(body.agent_entrant_id)
            .bind(debut)
            .bind(au)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if deja {
                lignes.push(LigneRemplacementDto {
                    service_id: sid,
                    intervention_id: iid,
                    nature,
                    resultat: "deja_intervenant".into(),
                });
                continue;
            }
            if body.apercu {
                lignes.push(LigneRemplacementDto {
                    service_id: sid,
                    intervention_id: iid,
                    nature,
                    resultat: "a_faire".into(),
                });
                continue;
            }
            let avant = vacant_tx(&mut tx, etab, sid, jour).await?;
            if du < body.a_partir_du {
                sqlx::query("UPDATE ent_service_intervenant SET au = $3 WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(iid)
                    .bind(body.a_partir_du - Duration::days(1))
                    .execute(&mut *tx)
                    .await
                    .map_err(ecriture)?;
            } else {
                sqlx::query(
                    "UPDATE ent_service_intervenant
                        SET annule_le = now(), annule_motif = 'autre', annule_par_user_id = $3
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(iid)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(ecriture)?;
            }
            let nid = inserer_intervention_tx(
                &mut tx,
                etab,
                actor,
                sid,
                body.agent_entrant_id,
                &nature,
                debut,
                au,
                heures,
                droits.as_deref(),
                "saisie",
                None,
                Some(iid),
            )
            .await?;
            publier_service_tx(
                &mut tx,
                etab,
                types::SERVICE_INTERVENANT_TERMINE,
                sid,
                json!({ "service_id": sid, "intervention_id": iid, "agent_id": body.agent_sortant_id,
                        "nature": nature, "au": body.a_partir_du - Duration::days(1), "lot": true }),
                actor,
            )
            .await?;
            publier_service_tx(
                &mut tx,
                etab,
                types::SERVICE_INTERVENANT_AJOUTE,
                sid,
                json!({ "service_id": sid, "intervention_id": nid, "agent_id": body.agent_entrant_id,
                        "nature": nature, "du": debut, "lot": true }),
                actor,
            )
            .await?;
            vacances.push((sid, avant));
            lignes.push(LigneRemplacementDto {
                service_id: sid,
                intervention_id: iid,
                nature,
                resultat: "fait".into(),
            });
        }
        if !body.apercu {
            for (sid, avant) in vacances {
                signaler_vacance_tx(&mut tx, etab, sid, jour, avant, actor).await?;
            }
            journal_tx(
                &mut tx,
                etab,
                "service_agent_remplace",
                "Agent remplacé sur ses services",
                json!({ "sortant": body.agent_sortant_id, "entrant": body.agent_entrant_id,
                        "a_partir_du": body.a_partir_du,
                        "services": lignes.iter().filter(|l| l.resultat == "fait").count() }),
                actor,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(RemplacementAgentDto {
            apercu: body.apercu,
            lignes,
        })
    }

    // -----------------------------------------------------------------------------------------
    // PRS-49 : répartition (par classe, par professeur, par matière) avec couverture
    // -----------------------------------------------------------------------------------------
    pub async fn repartition_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        filtre: &FiltreServices,
    ) -> Result<RepartitionDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = match filtre.le {
            Some(d) => d,
            None => aujourdhui_tx(&mut tx, etab).await?,
        };
        sqlx::query("SELECT ent_service_projection_rafraichir($1)")
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let services = charger_services_tx(&mut tx, etab, filtre, None, jour, true).await?;
        let agents = couverture_tx(&mut tx, etab, jour, filtre.annee_id).await?;
        let vacants = services.iter().filter(|s| s.vacant).count() as i64;
        tx.commit().await.map_err(db)?;
        Ok(RepartitionDto {
            le: jour,
            services,
            agents,
            vacants,
        })
    }

    pub async fn service_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        service_id: Uuid,
    ) -> Result<ServiceDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        service_dto_tx(&mut tx, etab, service_id, jour).await
    }

    // -----------------------------------------------------------------------------------------
    // PRS-54 : mes services et mes heures (portée `moi`)
    // -----------------------------------------------------------------------------------------
    pub async fn mes_services_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<MesServicesDto, EntDbError> {
        if !Role::parse(role).is_some_and(Role::is_staff) {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let Some(agent) = agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Ok(MesServicesDto {
                le: jour,
                agent_id: None,
                services: Vec::new(),
                heures_attribuees: 0.0,
                heures_dues: None,
                alertes: Vec::new(),
            });
        };
        let filtre = FiltreServices {
            agent_id: Some(agent),
            ..Default::default()
        };
        let services =
            charger_services_tx(&mut tx, etab, &filtre, Some(agent), jour, false).await?;
        let couv = couverture_tx(&mut tx, etab, jour, None).await?;
        let moi = couv.iter().find(|c| c.agent_id == agent);
        let mut alertes: Vec<AlerteServiceDto> = services
            .iter()
            .flat_map(|s| s.alertes.iter().filter(|a| a.code != "vacant").cloned())
            .collect();
        if let Some(m) = moi {
            if m.heures_dues.is_some_and(|d| m.heures_attribuees > d) {
                alertes.push(AlerteServiceDto {
                    code: "heures_depassees".into(),
                    service_id: None,
                    agent_id: Some(agent),
                });
            }
        }
        Ok(MesServicesDto {
            le: jour,
            agent_id: Some(agent),
            services,
            heures_attribuees: moi.map_or(0.0, |m| m.heures_attribuees),
            heures_dues: moi.and_then(|m| m.heures_dues),
            alertes,
        })
    }

    // -----------------------------------------------------------------------------------------
    // Indicateur « créneaux sans service »
    // -----------------------------------------------------------------------------------------
    pub async fn creneaux_sans_service_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<CreneauSansServiceDto>, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(Uuid, String, String, i16, String, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT c.id, c.classe, c.matiere_libelle, c.jour_semaine,
                    to_char(c.heure_debut, 'HH24:MI'), to_char(c.heure_fin, 'HH24:MI'), c.agent_id
               FROM ent_edt_creneau c
              WHERE c.etablissement_id = $1 AND c.service_id IS NULL
              ORDER BY c.jour_semaine, c.heure_debut, c.classe LIMIT 500",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    creneau_id,
                    classe,
                    matiere_libelle,
                    jour_semaine,
                    heure_debut,
                    heure_fin,
                    agent_id,
                )| {
                    CreneauSansServiceDto {
                        creneau_id,
                        classe,
                        matiere_libelle,
                        jour_semaine,
                        heure_debut,
                        heure_fin,
                        agent_id,
                    }
                },
            )
            .collect())
    }

    // -----------------------------------------------------------------------------------------
    // Ports pour les autres socles (appel, notes, cahier, familles) : aucun droit à exercer ici,
    // le socle appelant fait son propre contrôle.
    // -----------------------------------------------------------------------------------------

    /// Intervenants actifs d'un service à une date, avec la saisie effective.
    pub async fn intervenants_du_service(
        &self,
        etab: Uuid,
        service_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<IntervenantPortDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        port_intervenants_tx(&mut tx, etab, service_id, jour).await
    }

    /// Les seuls intervenants qui saisissent (appel, notes, cahier) ce jour-là.
    pub async fn en_saisie(
        &self,
        etab: Uuid,
        service_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<IntervenantPortDto>, EntDbError> {
        Ok(self
            .intervenants_du_service(etab, service_id, jour)
            .await?
            .into_iter()
            .filter(|i| i.saisie_effective)
            .collect())
    }

    /// Qui enseigne à cet élève ce jour-là : intervenants des services ouverts des cours dont il est
    /// membre (classe ou groupe, inclusions et exclusions individuelles comprises).
    pub async fn intervenants_de_l_eleve(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<IntervenantPortDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let services: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT s.id
               FROM ent_public_cours_jour($1, $2, NULL, NULL, $3) pc
               JOIN ent_service_enseignement s
                 ON s.etablissement_id = $1 AND s.cours_id = pc.cours_id AND s.etat = 'ouvert'",
        )
        .bind(etab)
        .bind(jour)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (sid,) in services {
            out.extend(port_intervenants_tx(&mut tx, etab, sid, jour).await?);
        }
        Ok(out)
    }
}

async fn port_intervenants_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    service_id: Uuid,
    jour: NaiveDate,
) -> Result<Vec<IntervenantPortDto>, EntDbError> {
    let rows: Vec<(Uuid, Uuid, String, bool, Option<Vec<String>>, Option<Uuid>)> = sqlx::query_as(
        "SELECT x.intervention_id, x.agent_id, x.nature, x.saisie_effective, x.droits, p.user_id
           FROM ent_service_intervenants_a($1, $2, $3) x
           JOIN ent_agent a ON a.etablissement_id = $1 AND a.id = x.agent_id
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
           JOIN ent_service_enseignement s ON s.etablissement_id = $1 AND s.id = $2
          WHERE s.etat IN ('ouvert', 'clos')
          ORDER BY x.nature, x.intervention_id",
    )
    .bind(etab)
    .bind(service_id)
    .bind(jour)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(
            |(intervention_id, agent_id, nature, saisie_effective, droits, user_id)| {
                IntervenantPortDto {
                    service_id,
                    intervention_id,
                    agent_id,
                    user_id,
                    nature,
                    saisie_effective,
                    droits,
                }
            },
        )
        .collect())
}

/// Services filtrés (cible, matière, agent, état, vacants), avec leurs intervenants et alertes.
async fn charger_services_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    f: &FiltreServices,
    agent_seul: Option<Uuid>,
    jour: NaiveDate,
    avec_annulees: bool,
) -> Result<Vec<ServiceDto>, EntDbError> {
    if let Some(e) = f.etat.as_deref() {
        if !matches!(e, "prevu" | "ouvert" | "clos" | "annule") {
            return Err(invalide("État de service inconnu."));
        }
    }
    // Année par défaut : celle qui contient la date de calcul (sinon toutes).
    let annee = match f.annee_id {
        Some(a) => Some(a),
        None => sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2
              ORDER BY date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(jour)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?,
    };
    let sql = format!(
        "SELECT {SERVICE_COLONNES}
           FROM ent_service_enseignement s
           LEFT JOIN ent_matiere m ON m.id = s.matiere_id AND m.etablissement_id = s.etablissement_id
          WHERE s.etablissement_id = $1
            AND ($2::uuid IS NULL OR s.annee_id = $2)
            AND ($3::uuid IS NULL OR s.classe_id = $3
                 OR s.groupe_id IN (SELECT g.id FROM ent_groupe g WHERE g.etablissement_id = $1 AND g.classe_id = $3))
            AND ($4::uuid IS NULL OR s.groupe_id = $4)
            AND ($5::uuid IS NULL OR s.matiere_id = $5)
            AND ($6::uuid IS NULL OR EXISTS (
                  SELECT 1 FROM ent_service_intervenant i
                   WHERE i.etablissement_id = s.etablissement_id AND i.service_id = s.id
                     AND i.agent_id = $6 AND i.annule_le IS NULL))
            AND (($7::text IS NULL AND s.etat <> 'annule') OR s.etat = $7)
          ORDER BY coalesce(m.libelle, s.matiere_libelle), s.created_at, s.id"
    );
    let lignes = sqlx::query_as::<_, ServiceLigne>(&sql)
        .bind(etab)
        .bind(annee)
        .bind(f.classe_id)
        .bind(f.groupe_id)
        .bind(f.matiere_id)
        .bind(agent_seul.or(f.agent_id))
        .bind(f.etat.as_deref())
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    let mut dtos = construire_dtos_tx(conn, etab, lignes, jour, avec_annulees).await?;
    if let Some(v) = f.vacants {
        dtos.retain(|s| s.vacant == v);
    }
    Ok(dtos)
}

/// Heures attribuées contre heures dues, par agent (titulaires, co-enseignants, intervenants,
/// assistants des services ouverts à la date ; les suppléances ne comptent pas : elles remplacent).
async fn couverture_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    annee: Option<Uuid>,
) -> Result<Vec<CouvertureAgentDto>, EntDbError> {
    let rows: Vec<(Uuid, String, f64, Option<f64>, i64)> = sqlx::query_as(
        "SELECT a.id, btrim(coalesce(p.prenom || ' ', '') || p.nom),
                coalesce(sum(coalesce(i.heures_hebdo, s.heures_hebdo, 0))::float8, 0),
                (SELECT sum(e.heures_dues_hebdo)::float8 FROM ent_agent_engagement e
                  WHERE e.etablissement_id = a.etablissement_id AND e.agent_id = a.id AND e.annule_le IS NULL
                    AND e.du <= $2 AND (e.au IS NULL OR e.au >= $2) AND e.heures_dues_hebdo IS NOT NULL),
                count(DISTINCT s.id)
           FROM ent_service_intervenant i
           JOIN ent_service_enseignement s ON s.etablissement_id = i.etablissement_id AND s.id = i.service_id
           JOIN ent_agent a ON a.etablissement_id = i.etablissement_id AND a.id = i.agent_id
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE i.etablissement_id = $1 AND i.annule_le IS NULL AND s.etat = 'ouvert'
            AND i.nature IN ('titulaire', 'co_enseignant', 'intervenant', 'assistant')
            AND i.du <= $2 AND (i.au IS NULL OR i.au >= $2)
            AND ($3::uuid IS NULL OR s.annee_id = $3)
          GROUP BY a.id, a.etablissement_id, p.prenom, p.nom
          ORDER BY lower(p.nom), lower(coalesce(p.prenom, ''))",
    )
    .bind(etab)
    .bind(jour)
    .bind(annee)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(
            |(agent_id, agent, heures_attribuees, heures_dues, services)| CouvertureAgentDto {
                agent_id,
                agent,
                heures_attribuees,
                heures_dues,
                services,
            },
        )
        .collect())
}
