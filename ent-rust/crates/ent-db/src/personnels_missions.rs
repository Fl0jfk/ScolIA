//! Personnels, lot P4 — missions, périmètre des droits, explication et simulation
//! (`06_PERSONNELS.md` §8.6, §9, §10.5, PRS-60 à 65, 67, 101, 102 ; migration 355).
//!
//! Une **mission** confie à un agent une responsabilité sur un **périmètre**. Quand son type porte un
//! modèle de rôle (professeur principal, CPE référent, ou un rôle de l'établissement), la mission
//! crée (ou réutilise) une attribution de ce rôle dont les CIBLES sont le périmètre (`ent_attribution_cible`,
//! datées) : la portée `mon_perimetre` les lit (`droits_repo.rs`), et la fin ou l'annulation de la mission
//! termine les cibles par déclencheur. Un seul modèle « Professeur principal » sert donc à toutes les classes.
//!
//! | Commande / port                                  | Droit                  |
//! |--------------------------------------------------|------------------------|
//! | `types_mission_for_actor`, `missions_for_actor`  | `personnels.services.lire` |
//! | `mes_missions_for_actor`                         | portée `moi` : être agent ici |
//! | `creer_type_mission_for_actor` (PRS-64)          | `missions.types.gerer` |
//! | `nommer_mission_for_actor` (PRS-60, 62)          | `missions.gerer`       |
//! | `tableau_pp_for_actor`, `nommer_pp_lot_for_actor` (PRS-61) | `missions.gerer` |
//! | `terminer_`, `transferer_`, `annuler_mission_for_actor` (PRS-63) | `missions.gerer` |
//! | `reconduire_missions_for_actor` (PRS-65)         | `missions.gerer`       |
//! | `attribuer_mission_socle_tiers` (PRS-67)         | port : le socle propriétaire du type |
//! | `simuler_mission_for_actor`, `expliquer_acces_for_actor` (PRS-101, 102) | `acces.revue` (soi-même : sans droit) |
//!
//! Chaque commande est une transaction `TenantTx` (RLS) = fait + journal + événement. Journal et
//! événements : identifiants, codes, dates — jamais de nom. Les contrôles d'éligibilité sont des
//! AVERTISSEMENTS (`effet_regle = avertir`) ou des refus (`bloquer`) selon le type.
//!
//! **Périmètre d'enseignement** (écarts 1 à 3 de §24, recommandations de P3 appliquées par défaut) : la
//! fonction SQL `ent_public_cours_jour` lit désormais les INTERVENTIONS (migration 355). Les fonctions de
//! ce module ajoutent la distinction lecture / saisie (`eleve_en_lecture_seule`).

#![allow(clippy::type_complexity)] // lignes de requête SQL lues sous forme de tuples

use std::collections::{HashMap, HashSet};

use crate::Droit;
use chrono::NaiveDate;
use ent_auth::{Action, Portee};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::personnels_missions::*;
use serde_json::{json, Value};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::dossier_personnes::aujourdhui_tx;
use crate::outbox::Publier;
use crate::personnels::{agent_tx, conflit, db, introuvable, invalide, journal_tx};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

pub const NATURES_PERIMETRE: &[&str] = &[
    "etablissement",
    "site",
    "niveau",
    "classe",
    "groupe",
    "eleves",
    "agents",
    "discipline",
];
const MAX_PERIMETRES: usize = 50;
/// Natures qui portent une cible lue par la portée `mon_perimetre`.
const NATURES_CIBLES: &[&str] = &["site", "niveau", "classe", "groupe", "eleves", "agents"];

/// Droits d'un modèle fourni. TOUS à portée `mon_perimetre` : la mission fournit les cibles.
pub fn droits_du_modele(code: &str) -> Vec<Action> {
    match code {
        "professeur_principal" => vec![
            Action::DossierLire,
            Action::AbsencesLire,
            Action::NotesLire,
            Action::SuiviEleve,
            Action::MessagerieEleve,
            Action::ConseilsGerer,
            Action::JournalLire,
        ],
        "cpe_referent" => vec![
            Action::DossierLire,
            Action::AbsencesLire,
            Action::AbsencesJustifier,
            Action::SuiviEleve,
            Action::MessagerieEleve,
            Action::JournalLire,
        ],
        _ => Vec::new(),
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
struct TypeLigne {
    id: Uuid,
    etablissement_id: Option<Uuid>,
    code: String,
    libelle: String,
    description: Option<String>,
    perimetres_admis: Vec<String>,
    cardinalite_max: Option<i16>,
    annuelle: bool,
    modele_role_id: Option<Uuid>,
    modele_code: Option<String>,
    visible_familles: bool,
    visible_eleves: bool,
    regle_eligibilite: String,
    effet_regle: String,
    fonctions_eligibles: Option<Vec<String>>,
    proprietaire_socle: Option<i16>,
    actif: bool,
}

const TYPE_COLONNES: &str = "t.id, t.etablissement_id, t.code, t.libelle, t.description, t.perimetres_admis,
    t.cardinalite_max, t.annuelle, t.modele_role_id, t.modele_code, t.visible_familles, t.visible_eleves,
    t.regle_eligibilite, t.effet_regle, t.fonctions_eligibles, t.proprietaire_socle, t.actif";

impl TypeLigne {
    fn dto(&self) -> TypeMissionDto {
        TypeMissionDto {
            id: self.id,
            code: self.code.clone(),
            libelle: self.libelle.clone(),
            description: self.description.clone(),
            perimetres_admis: self.perimetres_admis.clone(),
            cardinalite_max: self.cardinalite_max,
            annuelle: self.annuelle,
            porte_un_role: self.modele_role_id.is_some() || self.modele_code.is_some(),
            visible_familles: self.visible_familles,
            visible_eleves: self.visible_eleves,
            regle_eligibilite: self.regle_eligibilite.clone(),
            effet_regle: self.effet_regle.clone(),
            proprietaire_socle: self.proprietaire_socle,
            fourni: self.etablissement_id.is_none(),
            actif: self.actif,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Erreurs de la base → refus lisibles
// ---------------------------------------------------------------------------------------------

fn erreur_mission(err: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &err {
        let msg = d.message();
        if msg.contains("mission_cardinalite") {
            return conflit("Cette cible a déjà le nombre maximal de missions de ce type.");
        }
        if msg.contains("mission_perimetre_non_admis") {
            return invalide("Ce type de mission n'admet pas cette nature de périmètre.");
        }
        if msg.contains("mission_perimetre_cible_inconnue")
            || msg.contains("mission_type_autre_etablissement")
            || msg.contains("mission_type_inconnu")
        {
            return introuvable();
        }
        if msg.contains("mission_") {
            return invalide("Mission non modifiable.");
        }
        let code = d.code().map(|c| c.to_string()).unwrap_or_default();
        if code == "23505" || code == "23P01" {
            return conflit("Cette attribution existe déjà.");
        }
    }
    db(err)
}

// ---------------------------------------------------------------------------------------------
// Lectures partagées
// ---------------------------------------------------------------------------------------------

async fn type_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Option<Uuid>,
    code: Option<&str>,
) -> Result<TypeLigne, EntDbError> {
    let sql = format!(
        "SELECT {TYPE_COLONNES} FROM ent_mission_type t
          WHERE (t.etablissement_id IS NULL OR t.etablissement_id = $1)
            AND (($2::uuid IS NOT NULL AND t.id = $2) OR ($2::uuid IS NULL AND t.code = $3))
          ORDER BY (t.etablissement_id IS NULL) LIMIT 1"
    );
    sqlx::query_as::<_, TypeLigne>(&sql)
        .bind(etab)
        .bind(id)
        .bind(code.map(str::trim))
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

async fn user_de_l_agent_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    let r: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT p.user_id FROM ent_agent a
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND a.id = $2",
    )
    .bind(etab)
    .bind(agent_id)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.and_then(|(u,)| u))
}

type LigneMission = (
    Uuid,
    Uuid,
    String,
    String,
    Uuid,
    String,
    Option<Uuid>,
    NaiveDate,
    Option<NaiveDate>,
    String,
    bool,
    bool,
);

const MISSION_SELECT: &str = "SELECT m.id, m.type_id, t.code, t.libelle, m.agent_id,
        btrim(coalesce(p.prenom || ' ', '') || p.nom) AS agent, m.annee_id, m.du, m.au, m.origine,
        (m.annule_le IS NOT NULL) AS annulee,
        EXISTS (SELECT 1 FROM ent_attribution_cible c
                 WHERE c.etablissement_id = m.etablissement_id AND c.mission_id = m.id) AS droits
   FROM ent_mission m
   JOIN ent_mission_type t ON t.id = m.type_id
   JOIN ent_agent a ON a.etablissement_id = m.etablissement_id AND a.id = m.agent_id
   JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id";

async fn dtos_missions_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    lignes: Vec<LigneMission>,
    jour: NaiveDate,
    avertissements: &HashMap<Uuid, Vec<String>>,
) -> Result<Vec<MissionDto>, EntDbError> {
    let ids: Vec<Uuid> = lignes.iter().map(|l| l.0).collect();
    let rows: Vec<(Uuid, String, Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT mission_id, nature, cible_id, discipline_code FROM ent_mission_perimetre
          WHERE etablissement_id = $1 AND mission_id = ANY($2) ORDER BY cree_le, id",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut per: HashMap<Uuid, Vec<PerimetreMissionDto>> = HashMap::new();
    for (m, nature, cible_id, discipline_code) in rows {
        per.entry(m).or_default().push(PerimetreMissionDto {
            nature,
            cible_id,
            discipline_code,
        });
    }
    Ok(lignes
        .into_iter()
        .map(
            |(
                id,
                type_id,
                type_code,
                type_libelle,
                agent_id,
                agent,
                annee_id,
                du,
                au,
                origine,
                annulee,
                droits,
            )| {
                MissionDto {
                    id,
                    type_id,
                    type_code,
                    type_libelle,
                    agent_id,
                    agent,
                    annee_id,
                    du,
                    au,
                    origine,
                    perimetres: per.remove(&id).unwrap_or_default(),
                    annulee,
                    active: !annulee && du <= jour && au.is_none_or(|a| a >= jour),
                    droits_appliques: droits,
                    avertissements: avertissements.get(&id).cloned().unwrap_or_default(),
                }
            },
        )
        .collect())
}

async fn mission_dto_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    jour: NaiveDate,
    avertissements: &HashMap<Uuid, Vec<String>>,
) -> Result<MissionDto, EntDbError> {
    let l: LigneMission = sqlx::query_as(&format!(
        "{MISSION_SELECT} WHERE m.etablissement_id = $1 AND m.id = $2"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    Ok(dtos_missions_tx(conn, etab, vec![l], jour, avertissements)
        .await?
        .remove(0))
}

// ---------------------------------------------------------------------------------------------
// Validation, éligibilité
// ---------------------------------------------------------------------------------------------

fn valider_perimetres(t: &TypeLigne, p: &[PerimetreMissionDto]) -> Result<(), EntDbError> {
    if p.is_empty() || p.len() > MAX_PERIMETRES {
        return Err(invalide("Précisez le périmètre (1 à 50 éléments)."));
    }
    let mut vus: HashSet<(String, Option<Uuid>, Option<String>)> = HashSet::new();
    for e in p {
        if !NATURES_PERIMETRE.contains(&e.nature.as_str()) {
            return Err(invalide("Nature de périmètre inconnue."));
        }
        if !t.perimetres_admis.contains(&e.nature) {
            return Err(invalide(
                "Ce type de mission n'admet pas cette nature de périmètre.",
            ));
        }
        let forme_ok = match e.nature.as_str() {
            "etablissement" => e.cible_id.is_none() && e.discipline_code.is_none(),
            "discipline" => {
                e.cible_id.is_none()
                    && e.discipline_code.as_deref().is_some_and(|c| {
                        (2..=60).contains(&c.len())
                            && c.starts_with(|x: char| x.is_ascii_lowercase())
                            && c.chars()
                                .all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '_')
                    })
            }
            _ => e.cible_id.is_some() && e.discipline_code.is_none(),
        };
        if !forme_ok {
            return Err(invalide("Périmètre mal formé."));
        }
        if !vus.insert((e.nature.clone(), e.cible_id, e.discipline_code.clone())) {
            return Err(invalide("Périmètre en double."));
        }
    }
    Ok(())
}

/// Éligibilité (§8.6) : codes d'avertissement ; `bloquer` refuse.
async fn eligibilite_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    t: &TypeLigne,
    agent_id: Uuid,
    p: &[PerimetreMissionDto],
    du: NaiveDate,
) -> Result<Vec<String>, EntDbError> {
    let mut codes: Vec<String> = Vec::new();
    let (engage,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_agent_engagement e
                         WHERE e.etablissement_id = $1 AND e.agent_id = $2
                           AND e.du <= $3 AND (e.au IS NULL OR e.au >= $3))",
    )
    .bind(etab)
    .bind(agent_id)
    .bind(du)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !engage {
        codes.push("agent_sans_engagement_actif".to_string());
    }
    match t.regle_eligibilite.as_str() {
        "enseigne_dans_perimetre" => {
            for e in p
                .iter()
                .filter(|e| matches!(e.nature.as_str(), "classe" | "groupe" | "niveau"))
            {
                let (ok,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (
                       SELECT 1
                         FROM ent_service_intervenant i
                         JOIN ent_service_enseignement s ON s.etablissement_id = i.etablissement_id AND s.id = i.service_id
                         JOIN ent_cours c ON c.etablissement_id = s.etablissement_id AND c.id = s.cours_id
                        WHERE i.etablissement_id = $1 AND i.agent_id = $2 AND i.annule_le IS NULL
                          AND i.nature IN ('titulaire', 'co_enseignant', 'suppleant', 'intervenant')
                          AND s.etat IN ('ouvert', 'prevu')
                          AND coalesce(i.au, 'infinity'::date) >= $3
                          AND (CASE $4
                                 WHEN 'classe' THEN c.classe_id = $5 OR c.groupe_id IN
                                      (SELECT g.id FROM ent_groupe g WHERE g.etablissement_id = $1 AND g.classe_id = $5)
                                 WHEN 'groupe' THEN c.groupe_id = $5
                                 ELSE c.classe_id IN
                                      (SELECT k.id FROM ent_classe k WHERE k.etablissement_id = $1 AND k.niveau_id = $5)
                               END))",
                )
                .bind(etab)
                .bind(agent_id)
                .bind(du)
                .bind(&e.nature)
                .bind(e.cible_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                if !ok {
                    codes.push("agent_n_enseigne_pas_dans_le_perimetre".to_string());
                    break;
                }
            }
        }
        "fonction" => {
            let attendues = t.fonctions_eligibles.clone().unwrap_or_default();
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (
                   SELECT 1 FROM ent_agent_fonction af
                     JOIN ent_fonction f ON f.id = af.fonction_id
                    WHERE af.etablissement_id = $1 AND af.agent_id = $2 AND af.annule_le IS NULL
                      AND af.du <= $3 AND (af.au IS NULL OR af.au >= $3)
                      AND f.code = ANY($4))",
            )
            .bind(etab)
            .bind(agent_id)
            .bind(du)
            .bind(&attendues)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if !ok {
                codes.push("agent_sans_fonction_eligible".to_string());
            }
        }
        _ => {}
    }
    let regle_violee = codes.iter().any(|c| c != "agent_sans_engagement_actif");
    if regle_violee && t.effet_regle == "bloquer" {
        return Err(conflit("L'agent n'est pas éligible à cette mission."));
    }
    Ok(codes)
}

// ---------------------------------------------------------------------------------------------
// Rôle appliqué par le type
// ---------------------------------------------------------------------------------------------

/// Le rôle du type : celui de l'établissement, ou le modèle fourni, créé à la première utilisation
/// (tous ses droits sont à portée `mon_perimetre`).
async fn role_du_type_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    t: &TypeLigne,
) -> Result<Option<Uuid>, EntDbError> {
    if let Some(r) = t.modele_role_id {
        return Ok(Some(r));
    }
    let Some(code) = t.modele_code.as_deref() else {
        return Ok(None);
    };
    let existant: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_role WHERE etablissement_id = $1 AND modele = $2 ORDER BY created_at LIMIT 1",
    )
    .bind(etab)
    .bind(code)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some((id,)) = existant {
        return Ok(Some(id));
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("modele-role:{etab}:{code}"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    let existant: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_role WHERE etablissement_id = $1 AND modele = $2 ORDER BY created_at LIMIT 1",
    )
    .bind(etab)
    .bind(code)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some((id,)) = existant {
        return Ok(Some(id));
    }
    let id = Uuid::new_v4();
    // Le nom du rôle est unique dans l'établissement : un doublon de nom prend un suffixe.
    let nom_pris: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_role WHERE etablissement_id = $1 AND lower(btrim(nom)) = lower(btrim($2))",
    )
    .bind(etab)
    .bind(&t.libelle)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let nom = if nom_pris.is_some() {
        format!("{} (mission)", t.libelle)
    } else {
        t.libelle.clone()
    };
    sqlx::query(
        "INSERT INTO ent_role (id, etablissement_id, nom, description, modele, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(etab)
    .bind(nom.chars().take(80).collect::<String>())
    .bind("Rôle appliqué au périmètre d'une mission (créé par ScolIA).")
    .bind(code)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    for a in droits_du_modele(code) {
        sqlx::query(
            "INSERT INTO ent_role_droit (role_id, etablissement_id, action, portee, cibles)
             VALUES ($1, $2, $3, 'mon_perimetre', '{}')",
        )
        .bind(id)
        .bind(etab)
        .bind(a.code())
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(Some(id))
}

/// Attribution du rôle à l'agent (créée ou réutilisée) et cibles datées de la mission.
async fn appliquer_role_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    role_id: Uuid,
    user_id: Uuid,
    mission_id: Uuid,
    perimetres: &[PerimetreMissionDto],
    du: NaiveDate,
) -> Result<(), EntDbError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("attribution-mission:{role_id}:{user_id}"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    let existante: Option<(Uuid, NaiveDate)> = sqlx::query_as(
        "SELECT id, debut FROM ent_role_attribution
          WHERE etablissement_id = $1 AND role_id = $2 AND user_id = $3 AND clos_le IS NULL
            AND (fin IS NULL OR fin >= $4)
          ORDER BY debut LIMIT 1",
    )
    .bind(etab)
    .bind(role_id)
    .bind(user_id)
    .bind(du)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let attribution = match existante {
        Some((id, debut)) => {
            if du < debut {
                sqlx::query("UPDATE ent_role_attribution SET debut = $2 WHERE id = $1 AND etablissement_id = $3")
                    .bind(id)
                    .bind(du)
                    .bind(etab)
                    .execute(&mut *conn)
                    .await
                    .map_err(erreur_mission)?;
            }
            id
        }
        None => {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_role_attribution
                     (id, role_id, etablissement_id, user_id, debut, fin, attribue_par_user_id, source, source_ref)
                 VALUES ($1, $2, $3, $4, $5, NULL, $6, 'mission', $7)",
            )
            .bind(id)
            .bind(role_id)
            .bind(etab)
            .bind(user_id)
            .bind(du)
            .bind(actor)
            .bind(mission_id)
            .execute(&mut *conn)
            .await
            .map_err(erreur_mission)?;
            id
        }
    };
    for e in perimetres
        .iter()
        .filter(|e| NATURES_CIBLES.contains(&e.nature.as_str()))
    {
        sqlx::query(
            "INSERT INTO ent_attribution_cible
                 (id, etablissement_id, attribution_id, mission_id, nature, cible_id, du, au)
             SELECT $1, $2, $3, m.id, $4, $5, m.du, m.au FROM ent_mission m
              WHERE m.etablissement_id = $2 AND m.id = $6",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(attribution)
        .bind(&e.nature)
        .bind(e.cible_id)
        .bind(mission_id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

/// Quand plus aucune cible ne reste active ou à venir sur une attribution de mission, elle prend fin
/// (annulation : clôture, aucune trace de droit).
async fn fermer_attributions_vides_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    mission_id: Uuid,
    au: NaiveDate,
    annulee: bool,
) -> Result<(), EntDbError> {
    let attrs: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT attribution_id FROM ent_attribution_cible
          WHERE etablissement_id = $1 AND mission_id = $2",
    )
    .bind(etab)
    .bind(mission_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (a,) in attrs {
        let (reste,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_attribution_cible c
                             WHERE c.attribution_id = $1 AND c.etablissement_id = $3 AND c.annule_le IS NULL
                               AND (c.au IS NULL OR c.au >= $2))",
        )
        .bind(a)
        .bind(au)
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if reste {
            continue;
        }
        if annulee {
            sqlx::query(
                "UPDATE ent_role_attribution SET clos_le = now(), clos_par_user_id = $2
                  WHERE id = $1 AND etablissement_id = $3 AND source = 'mission' AND clos_le IS NULL",
            )
            .bind(a)
            .bind(actor)
            .bind(etab)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        } else {
            sqlx::query(
                "UPDATE ent_role_attribution SET fin = greatest(debut, $2)
                  WHERE id = $1 AND etablissement_id = $3 AND source = 'mission' AND clos_le IS NULL AND (fin IS NULL OR fin > $2)",
            )
            .bind(a)
            .bind(au)
            .bind(etab)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Événements
// ---------------------------------------------------------------------------------------------

async fn publier_mission_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    mission_id: Uuid,
    payload: Value,
    actor: Uuid,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Referentiel,
        aggregate_id: mission_id,
        eleve_id: None,
        payload,
        actor_user_id: Some(actor.to_string()),
    })
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Noyau : poser une mission
// ---------------------------------------------------------------------------------------------

struct Nomination<'a> {
    type_id: Option<Uuid>,
    type_code: Option<&'a str>,
    agent_id: Uuid,
    perimetres: &'a [PerimetreMissionDto],
    du: Option<NaiveDate>,
    au: Option<NaiveDate>,
    annee_id: Option<Uuid>,
    origine: &'a str,
    origine_ref: Option<Uuid>,
    /// Le socle appelant, pour un type réservé (PRS-67) ; `None` : un type réservé est refusé.
    socle: Option<i16>,
}

async fn annee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
) -> Result<(NaiveDate, NaiveDate), EntDbError> {
    sqlx::query_as("SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(annee_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

async fn nommer_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    actor: Uuid,
    n: &Nomination<'_>,
    jour: NaiveDate,
) -> Result<(Uuid, Vec<String>), EntDbError> {
    let t = type_tx(tx, etab, n.type_id, n.type_code).await?;
    if !t.actif {
        return Err(conflit("Ce type de mission est désactivé."));
    }
    match (t.proprietaire_socle, n.socle) {
        (Some(p), Some(s)) if p == s => {}
        (Some(_), _) => return Err(EntDbError::Forbidden),
        (None, _) => {}
    }
    valider_perimetres(&t, n.perimetres)?;
    // L'agent doit être de CET établissement (404 uniforme).
    agent_tx(tx, etab, n.agent_id, false).await?;
    let mut du = n.du.unwrap_or(jour);
    let mut au = n.au;
    let mut annee_id = n.annee_id;
    if let Some(a) = annee_id {
        let (debut, fin) = annee_tx(tx, etab, a).await?;
        if n.du.is_none() && jour < debut {
            du = debut;
        }
        if au.is_none() && t.annuelle {
            au = Some(fin);
        }
    } else if t.annuelle {
        let courante: Option<(Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT id, date_fin FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2",
        )
        .bind(etab)
        .bind(du)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        if let Some((id, fin)) = courante {
            annee_id = Some(id);
            if au.is_none() {
                au = Some(fin);
            }
        }
    }
    if let Some(a) = au {
        if a < du {
            return Err(invalide("La fin précède le début."));
        }
    }
    let avertissements = eligibilite_tx(tx, etab, &t, n.agent_id, n.perimetres, du).await?;
    let mission_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_mission (id, etablissement_id, type_id, agent_id, annee_id, du, au, origine, origine_ref,
                                  cree_par_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(mission_id)
    .bind(etab)
    .bind(t.id)
    .bind(n.agent_id)
    .bind(annee_id)
    .bind(du)
    .bind(au)
    .bind(n.origine)
    .bind(n.origine_ref)
    .bind(actor)
    .execute(&mut **tx)
    .await
    .map_err(erreur_mission)?;
    for e in n.perimetres {
        sqlx::query(
            "INSERT INTO ent_mission_perimetre (id, etablissement_id, mission_id, nature, cible_id, discipline_code)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(mission_id)
        .bind(&e.nature)
        .bind(e.cible_id)
        .bind(&e.discipline_code)
        .execute(&mut **tx)
        .await
        .map_err(erreur_mission)?;
    }
    let mut droits = false;
    if let Some(role_id) = role_du_type_tx(tx, etab, actor, &t).await? {
        if let Some(user) = user_de_l_agent_tx(tx, etab, n.agent_id).await? {
            appliquer_role_tx(tx, etab, actor, role_id, user, mission_id, n.perimetres, du).await?;
            droits = true;
        }
    }
    let payload = json!({ "mission_id": mission_id, "type": t.code, "agent_id": n.agent_id,
        "du": du, "au": au, "origine": n.origine, "droits_appliques": droits,
        "perimetres": n.perimetres.iter().map(|p| json!({ "nature": p.nature, "cible_id": p.cible_id,
            "discipline": p.discipline_code })).collect::<Vec<_>>() });
    journal_tx(
        tx,
        etab,
        "mission_attribuee",
        "Mission attribuée",
        payload.clone(),
        actor,
    )
    .await?;
    publier_mission_tx(
        tx,
        etab,
        types::MISSION_ATTRIBUEE,
        mission_id,
        payload,
        actor,
    )
    .await?;
    Ok((mission_id, avertissements))
}

fn charge_nomination<'a>(
    b: &'a NommerMissionBody,
    origine: &'a str,
    socle: Option<i16>,
) -> Nomination<'a> {
    Nomination {
        type_id: b.type_id,
        type_code: b.type_code.as_deref(),
        agent_id: b.agent_id,
        perimetres: &b.perimetres,
        du: b.du,
        au: b.au,
        annee_id: b.annee_id,
        origine,
        origine_ref: None,
        socle,
    }
}

impl PgEnt {
    async fn exiger_missions_gerer(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::MissionsGerer)
            .await
    }

    // -----------------------------------------------------------------------------------------
    // Lectures
    // -----------------------------------------------------------------------------------------
    pub async fn types_mission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<TypeMissionDto>, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let sql = format!(
            "SELECT {TYPE_COLONNES} FROM ent_mission_type t
              WHERE t.etablissement_id IS NULL OR t.etablissement_id = $1
              ORDER BY (t.etablissement_id IS NOT NULL), lower(t.libelle)"
        );
        let lignes = sqlx::query_as::<_, TypeLigne>(&sql)
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        Ok(lignes.iter().map(TypeLigne::dto).collect())
    }

    pub async fn missions_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        f: &FiltreMissions,
    ) -> Result<Vec<MissionDto>, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsServicesLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let le = f.le.unwrap_or(jour);
        let lignes: Vec<LigneMission> = sqlx::query_as(&format!(
            "{MISSION_SELECT}
              WHERE m.etablissement_id = $1
                AND ($2::text IS NULL OR t.code = $2)
                AND ($3::uuid IS NULL OR m.agent_id = $3)
                AND ($4 OR (m.annule_le IS NULL AND m.du <= $5 AND (m.au IS NULL OR m.au >= $5)))
              ORDER BY t.libelle, m.du DESC, m.id LIMIT 500"
        ))
        .bind(etab)
        .bind(f.type_code.as_deref().map(str::trim))
        .bind(f.agent_id)
        .bind(f.avec_historique)
        .bind(le)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        dtos_missions_tx(&mut tx, etab, lignes, le, &HashMap::new()).await
    }

    /// Mes missions (portée `moi`) : tout agent de l'établissement voit les siennes.
    pub async fn mes_missions_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
    ) -> Result<Vec<MissionDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let Some(agent) = crate::personnels::agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Err(introuvable());
        };
        let lignes: Vec<LigneMission> = sqlx::query_as(&format!(
            "{MISSION_SELECT}
              WHERE m.etablissement_id = $1 AND m.agent_id = $2 AND m.annule_le IS NULL
                AND (m.au IS NULL OR m.au >= $3 - 365)
              ORDER BY m.du DESC, m.id LIMIT 200"
        ))
        .bind(etab)
        .bind(agent)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        dtos_missions_tx(&mut tx, etab, lignes, jour, &HashMap::new()).await
    }

    // -----------------------------------------------------------------------------------------
    // PRS-64 : type de mission propre à l'établissement
    // -----------------------------------------------------------------------------------------
    pub async fn creer_type_mission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &CreerTypeMissionBody,
    ) -> Result<TypeMissionDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::MissionsTypesGerer)
            .await?;
        let code = b.code.trim();
        if !(2..=60).contains(&code.len())
            || !code.starts_with(|c: char| c.is_ascii_lowercase())
            || !code
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(invalide(
                "Code : minuscules, chiffres et _ (2 à 60 caractères).",
            ));
        }
        let libelle = b.libelle.trim();
        if libelle.is_empty() || libelle.chars().count() > 120 {
            return Err(invalide("Libellé : 1 à 120 caractères."));
        }
        if b.description
            .as_deref()
            .is_some_and(|d| d.chars().count() > 500)
        {
            return Err(invalide("Description : 500 caractères au plus."));
        }
        if b.perimetres_admis.is_empty()
            || b.perimetres_admis.len() > 8
            || b.perimetres_admis
                .iter()
                .any(|p| !NATURES_PERIMETRE.contains(&p.as_str()))
        {
            return Err(invalide("Périmètres admis : 1 à 8 natures connues."));
        }
        if b.cardinalite_max.is_some_and(|c| !(1..=20).contains(&c)) {
            return Err(invalide("Cardinalité : 1 à 20."));
        }
        let regle = b.regle_eligibilite.as_deref().unwrap_or("aucune");
        if !matches!(regle, "enseigne_dans_perimetre" | "fonction" | "aucune") {
            return Err(invalide("Règle d'éligibilité inconnue."));
        }
        let effet = b.effet_regle.as_deref().unwrap_or("avertir");
        if !matches!(effet, "avertir" | "bloquer") {
            return Err(invalide("Effet de la règle inconnu."));
        }
        if regle == "fonction"
            && b.fonctions_eligibles
                .as_ref()
                .is_none_or(|f| f.is_empty() || f.len() > 20)
        {
            return Err(invalide(
                "Règle « fonction » : indiquez les fonctions éligibles.",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if let Some(rid) = b.modele_role_id {
            // Le rôle est de CET établissement, ne contient que des droits `mon_perimetre` (la mission
            // fournit les cibles) et l'acteur en détient chacun (anti-escalade).
            let droits: Vec<(String, String)> = sqlx::query_as(
                "SELECT d.action, d.portee FROM ent_role r
                   JOIN ent_role_droit d ON d.role_id = r.id AND d.etablissement_id = r.etablissement_id
                  WHERE r.etablissement_id = $1 AND r.id = $2",
            )
            .bind(etab)
            .bind(rid)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let (existe,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_role WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(rid)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !existe {
                return Err(introuvable());
            }
            if droits.is_empty() || droits.iter().any(|(_, p)| p != "mon_perimetre") {
                return Err(invalide(
                    "Le rôle d'une mission ne contient que des droits à portée « mon périmètre ».",
                ));
            }
            let a_couvrir: Vec<Droit> = droits
                .iter()
                .filter_map(|(a, p)| {
                    Some(Droit {
                        action: Action::parse(a)?,
                        portee: Portee::parse(p)?,
                        cibles: Vec::new(),
                    })
                })
                .collect();
            self.exiger_couverture(etab, actor, role, &a_couvrir)
                .await?;
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_mission_type (id, etablissement_id, code, libelle, description, perimetres_admis,
                 cardinalite_max, annuelle, modele_role_id, visible_familles, visible_eleves,
                 regle_eligibilite, effet_regle, fonctions_eligibles, cree_par_user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(id)
        .bind(etab)
        .bind(code)
        .bind(libelle)
        .bind(b.description.as_deref().map(str::trim).filter(|d| !d.is_empty()))
        .bind(&b.perimetres_admis)
        .bind(b.cardinalite_max)
        .bind(b.annuelle)
        .bind(b.modele_role_id)
        .bind(b.visible_familles)
        .bind(b.visible_eleves)
        .bind(regle)
        .bind(effet)
        .bind(&b.fonctions_eligibles)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(d) = &e {
                if d.code().as_deref() == Some("23505") {
                    return conflit("Ce code de type de mission existe déjà.");
                }
            }
            db(e)
        })?;
        let payload =
            json!({ "type_id": id, "code": code, "role_applique": b.modele_role_id.is_some() });
        journal_tx(
            &mut tx,
            etab,
            "mission_type_cree",
            "Type de mission créé",
            payload.clone(),
            actor,
        )
        .await?;
        publier_mission_tx(&mut tx, etab, types::MISSION_TYPE_CREE, id, payload, actor).await?;
        let t = type_tx(&mut tx, etab, Some(id), None).await?;
        tx.commit().await.map_err(db)?;
        Ok(t.dto())
    }

    // -----------------------------------------------------------------------------------------
    // PRS-60, 62 : nommer
    // -----------------------------------------------------------------------------------------
    pub async fn nommer_mission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &NommerMissionBody,
    ) -> Result<MissionDto, EntDbError> {
        self.exiger_missions_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let (id, av) = nommer_tx(
            &mut tx,
            etab,
            actor,
            &charge_nomination(b, "saisie", None),
            jour,
        )
        .await?;
        let dto = mission_dto_tx(&mut tx, etab, id, jour, &HashMap::from([(id, av)])).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-67 — port pour un autre socle (AESH : 35, tuteur de stage : 39). Le socle appelant a déjà
    /// contrôlé le droit de son propre domaine ; ce port refuse tout type dont il n'est pas propriétaire,
    /// et ne contourne jamais un type d'un autre socle.
    pub async fn attribuer_mission_socle_tiers(
        &self,
        etab: Uuid,
        actor: Uuid,
        socle: i16,
        b: &NommerMissionBody,
    ) -> Result<MissionDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let (id, av) = nommer_tx(
            &mut tx,
            etab,
            actor,
            &charge_nomination(b, "socle_tiers", Some(socle)),
            jour,
        )
        .await?;
        let dto = mission_dto_tx(&mut tx, etab, id, jour, &HashMap::from([(id, av)])).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // PRS-61 : professeurs principaux, toutes les classes
    // -----------------------------------------------------------------------------------------
    /// Annuler une mission posée par erreur (aucun droit ne subsiste : l'attribution est close si elle
    /// ne porte plus d'autre cible). La mission reste dans l'historique.
    pub async fn annuler_mission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        mission_id: Uuid,
    ) -> Result<MissionDto, EntDbError> {
        self.exiger_missions_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let (type_code,): (String,) = sqlx::query_as(
            "SELECT t.code FROM ent_mission m JOIN ent_mission_type t ON t.id = m.type_id
              WHERE m.etablissement_id = $1 AND m.id = $2 AND m.annule_le IS NULL FOR UPDATE OF m",
        )
        .bind(etab)
        .bind(mission_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        sqlx::query(
            "UPDATE ent_mission SET annule_le = now(), annule_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(mission_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(erreur_mission)?;
        fermer_attributions_vides_tx(&mut tx, etab, actor, mission_id, jour, true).await?;
        let payload = json!({ "mission_id": mission_id, "type": type_code });
        journal_tx(
            &mut tx,
            etab,
            "mission_annulee",
            "Mission annulée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_mission_tx(
            &mut tx,
            etab,
            types::MISSION_ANNULEE,
            mission_id,
            payload,
            actor,
        )
        .await?;
        let dto = mission_dto_tx(&mut tx, etab, mission_id, jour, &HashMap::new()).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    pub async fn tableau_pp_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        annee_id: Option<Uuid>,
    ) -> Result<TableauPpDto, EntDbError> {
        self.exiger_missions_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let annee: Option<Uuid> = match annee_id {
            Some(a) => {
                annee_tx(&mut tx, etab, a).await?;
                Some(a)
            }
            None => sqlx::query_scalar(
                "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2",
            )
            .bind(etab)
            .bind(jour)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?,
        };
        let classes: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, libelle FROM ent_classe WHERE etablissement_id = $1 AND annee_id IS NOT DISTINCT FROM $2
              ORDER BY lower(libelle)",
        )
        .bind(etab)
        .bind(annee)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let pps: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT p.cible_id, m.agent_id, btrim(coalesce(pe.prenom || ' ', '') || pe.nom)
               FROM ent_mission m
               JOIN ent_mission_type t ON t.id = m.type_id AND t.code = 'professeur_principal'
               JOIN ent_mission_perimetre p ON p.etablissement_id = m.etablissement_id AND p.mission_id = m.id
                    AND p.nature = 'classe'
               JOIN ent_agent a ON a.etablissement_id = m.etablissement_id AND a.id = m.agent_id
               JOIN ent_personne pe ON pe.etablissement_id = a.etablissement_id AND pe.id = a.personne_id
              WHERE m.etablissement_id = $1 AND m.annule_le IS NULL
                AND m.du <= $2 AND (m.au IS NULL OR m.au >= $2)",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let candidats: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT DISTINCT c.classe_id, i.agent_id, btrim(coalesce(pe.prenom || ' ', '') || pe.nom)
               FROM ent_cours c
               JOIN ent_service_enseignement s ON s.etablissement_id = c.etablissement_id AND s.cours_id = c.id
                    AND s.etat IN ('ouvert', 'clos')
               JOIN ent_service_intervenant i ON i.etablissement_id = s.etablissement_id AND i.service_id = s.id
                    AND i.annule_le IS NULL AND i.nature IN ('titulaire', 'co_enseignant', 'suppleant')
                    AND i.du <= $3 AND (i.au IS NULL OR i.au >= $3)
               JOIN ent_agent a ON a.etablissement_id = i.etablissement_id AND a.id = i.agent_id
               JOIN ent_personne pe ON pe.etablissement_id = a.etablissement_id AND pe.id = a.personne_id
              WHERE c.etablissement_id = $1 AND c.annee_id IS NOT DISTINCT FROM $2 AND c.classe_id IS NOT NULL",
        )
        .bind(etab)
        .bind(annee)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out: Vec<ClassePpDto> = Vec::new();
        let mut sans: Vec<Uuid> = Vec::new();
        for (cid, lib) in classes {
            let pp: Vec<CandidatPpDto> = pps
                .iter()
                .filter(|(c, _, _)| *c == cid)
                .map(|(_, a, n)| CandidatPpDto {
                    agent_id: *a,
                    agent: n.clone(),
                })
                .collect();
            let mut cand: Vec<CandidatPpDto> = candidats
                .iter()
                .filter(|(c, _, _)| *c == cid)
                .map(|(_, a, n)| CandidatPpDto {
                    agent_id: *a,
                    agent: n.clone(),
                })
                .collect();
            cand.sort_by(|x, y| x.agent.to_lowercase().cmp(&y.agent.to_lowercase()));
            if pp.is_empty() {
                sans.push(cid);
            }
            out.push(ClassePpDto {
                classe_id: cid,
                classe: lib,
                professeurs_principaux: pp,
                candidats: cand,
            });
        }
        Ok(TableauPpDto {
            annee_id: annee,
            classes: out,
            classes_sans_pp: sans,
        })
    }

    pub async fn nommer_pp_lot_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &NommerPpLotBody,
    ) -> Result<LotMissionsDto, EntDbError> {
        self.exiger_missions_gerer(etab, actor, role).await?;
        if b.nominations.is_empty() || b.nominations.len() > 200 {
            return Err(invalide("Une à 200 nominations par lot."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let mut out: Vec<(Uuid, Vec<String>)> = Vec::new();
        for n in &b.nominations {
            let per = [PerimetreMissionDto {
                nature: "classe".into(),
                cible_id: Some(n.classe_id),
                discipline_code: None,
            }];
            let nom = Nomination {
                type_id: None,
                type_code: Some("professeur_principal"),
                agent_id: n.agent_id,
                perimetres: &per,
                du: b.du,
                au: b.au,
                annee_id: b.annee_id,
                origine: "saisie",
                origine_ref: None,
                socle: None,
            };
            out.push(nommer_tx(&mut tx, etab, actor, &nom, jour).await?);
        }
        let mut dtos = Vec::new();
        for (id, av) in out {
            dtos.push(mission_dto_tx(&mut tx, etab, id, jour, &HashMap::from([(id, av)])).await?);
        }
        tx.commit().await.map_err(db)?;
        Ok(LotMissionsDto { missions: dtos })
    }

    // -----------------------------------------------------------------------------------------
    // PRS-63 : terminer, transférer
    // -----------------------------------------------------------------------------------------
    pub async fn terminer_mission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        mission_id: Uuid,
        b: &TerminerMissionBody,
    ) -> Result<MissionDto, EntDbError> {
        self.exiger_missions_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        terminer_tx(&mut tx, etab, actor, mission_id, b.au).await?;
        let dto = mission_dto_tx(&mut tx, etab, mission_id, jour, &HashMap::new()).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    pub async fn transferer_mission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        mission_id: Uuid,
        b: &TransfererMissionBody,
    ) -> Result<MissionDto, EntDbError> {
        self.exiger_missions_gerer(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let (type_id, agent, du, au, annee_id): (
            Uuid,
            Uuid,
            NaiveDate,
            Option<NaiveDate>,
            Option<Uuid>,
        ) = sqlx::query_as(
            "SELECT type_id, agent_id, du, au, annee_id FROM ent_mission
                  WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NULL FOR UPDATE",
        )
        .bind(etab)
        .bind(mission_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if b.agent_id == agent {
            return Err(invalide("La mission est déjà confiée à cet agent."));
        }
        if b.le <= du || au.is_some_and(|a| a < b.le) {
            return Err(invalide("Date de transfert hors de la mission."));
        }
        let perimetres: Vec<(String, Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT nature, cible_id, discipline_code FROM ent_mission_perimetre
              WHERE etablissement_id = $1 AND mission_id = $2 ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(mission_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let per: Vec<PerimetreMissionDto> = perimetres
            .into_iter()
            .map(|(nature, cible_id, discipline_code)| PerimetreMissionDto {
                nature,
                cible_id,
                discipline_code,
            })
            .collect();
        // Fin de l'ancienne la veille ; la nouvelle reprend le jour même, jusqu'à la fin d'origine.
        terminer_tx(
            &mut tx,
            etab,
            actor,
            mission_id,
            b.le - chrono::Duration::days(1),
        )
        .await?;
        let nom = Nomination {
            type_id: Some(type_id),
            type_code: None,
            agent_id: b.agent_id,
            perimetres: &per,
            du: Some(b.le),
            au,
            annee_id,
            origine: "transfert",
            origine_ref: Some(mission_id),
            socle: None,
        };
        // Un type réservé à un socle ne se transfère pas ici (nommer_tx le refuse).
        let (id, av) = nommer_tx(&mut tx, etab, actor, &nom, jour).await?;
        let dto = mission_dto_tx(&mut tx, etab, id, jour, &HashMap::from([(id, av)])).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // PRS-65 : reconduire les missions annuelles
    // -----------------------------------------------------------------------------------------
    pub async fn reconduire_missions_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ReconduireMissionsBody,
    ) -> Result<ReconductionDto, EntDbError> {
        self.exiger_missions_gerer(etab, actor, role).await?;
        if b.annee_source_id == b.annee_cible_id {
            return Err(invalide("L'année cible doit différer de l'année source."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        annee_tx(&mut tx, etab, b.annee_source_id).await?;
        let (debut, fin) = annee_tx(&mut tx, etab, b.annee_cible_id).await?;
        let sources: Vec<(Uuid, Uuid, String, Uuid, String, bool, bool)> = sqlx::query_as(
            "SELECT m.id, m.type_id, t.code, m.agent_id, btrim(coalesce(pe.prenom || ' ', '') || pe.nom),
                    EXISTS (SELECT 1 FROM ent_mission_perimetre p
                             WHERE p.etablissement_id = m.etablissement_id AND p.mission_id = m.id
                               AND p.nature IN ('classe', 'groupe')),
                    EXISTS (SELECT 1 FROM ent_mission r WHERE r.etablissement_id = m.etablissement_id
                               AND r.origine = 'reconduction' AND r.origine_ref = m.id AND r.annule_le IS NULL)
               FROM ent_mission m
               JOIN ent_mission_type t ON t.id = m.type_id AND t.annuelle AND t.actif
                    AND t.proprietaire_socle IS NULL
               JOIN ent_agent a ON a.etablissement_id = m.etablissement_id AND a.id = m.agent_id
               JOIN ent_personne pe ON pe.etablissement_id = a.etablissement_id AND pe.id = a.personne_id
              WHERE m.etablissement_id = $1 AND m.annee_id = $2 AND m.annule_le IS NULL
              ORDER BY t.code, m.du, m.id",
        )
        .bind(etab)
        .bind(b.annee_source_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let cibles: HashMap<Uuid, Uuid> = b
            .cibles
            .iter()
            .map(|c| (c.mission_id, c.cible_id))
            .collect();
        let mut props = Vec::new();
        for (mid, type_id, code, agent, nom, vise_classe, deja) in sources {
            let (engage,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_agent_engagement e
                                 WHERE e.etablissement_id = $1 AND e.agent_id = $2
                                   AND e.du <= $3 AND (e.au IS NULL OR e.au >= $3))",
            )
            .bind(etab)
            .bind(agent)
            .bind(debut)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let etat = if deja {
                "deja_reconduite"
            } else if !engage {
                "agent_sans_engagement"
            } else if vise_classe && !cibles.contains_key(&mid) {
                "cible_a_choisir"
            } else {
                "reconductible"
            };
            let mut appliquee = false;
            if b.appliquer && etat == "reconductible" {
                let src: Vec<(String, Option<Uuid>, Option<String>)> = sqlx::query_as(
                    "SELECT nature, cible_id, discipline_code FROM ent_mission_perimetre
                      WHERE etablissement_id = $1 AND mission_id = $2 ORDER BY cree_le, id",
                )
                .bind(etab)
                .bind(mid)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                let mut per: Vec<PerimetreMissionDto> = Vec::new();
                for (nature, cible_id, discipline_code) in src {
                    let cible_id = if matches!(nature.as_str(), "classe" | "groupe") {
                        let nouvelle = *cibles
                            .get(&mid)
                            .ok_or_else(|| invalide("Cible manquante."))?;
                        let (ok,): (bool,) = sqlx::query_as(
                            "SELECT EXISTS (SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2 AND annee_id = $3
                                            AND $4 = 'classe')
                                 OR EXISTS (SELECT 1 FROM ent_groupe WHERE etablissement_id = $1 AND id = $2 AND annee_id = $3
                                            AND $4 = 'groupe')",
                        )
                        .bind(etab)
                        .bind(nouvelle)
                        .bind(b.annee_cible_id)
                        .bind(&nature)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(db)?;
                        if !ok {
                            return Err(invalide(
                                "La cible doit être une classe ou un groupe de l'année suivante.",
                            ));
                        }
                        Some(nouvelle)
                    } else {
                        cible_id
                    };
                    per.push(PerimetreMissionDto {
                        nature,
                        cible_id,
                        discipline_code,
                    });
                }
                let nom_n = Nomination {
                    type_id: Some(type_id),
                    type_code: None,
                    agent_id: agent,
                    perimetres: &per,
                    du: Some(debut),
                    au: Some(fin),
                    annee_id: Some(b.annee_cible_id),
                    origine: "reconduction",
                    origine_ref: Some(mid),
                    socle: None,
                };
                nommer_tx(&mut tx, etab, actor, &nom_n, jour).await?;
                appliquee = true;
            }
            props.push(PropositionReconductionDto {
                mission_id: mid,
                type_code: code,
                agent_id: agent,
                agent: nom,
                etat: etat.to_string(),
                appliquee,
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(ReconductionDto {
            propositions: props,
        })
    }

    // -----------------------------------------------------------------------------------------
    // PRS-102 : simuler ; PRS-101 : expliquer
    // -----------------------------------------------------------------------------------------
    pub async fn simuler_mission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &SimulerMissionBody,
    ) -> Result<SimulationDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::AccesRevue)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let t = type_tx(&mut tx, etab, b.type_id, b.type_code.as_deref()).await?;
        valider_perimetres(&t, &b.perimetres)?;
        agent_tx(&mut tx, etab, b.agent_id, false).await?;
        let du = b.du.unwrap_or(jour);
        let mut au = b.au;
        if au.is_none() && t.annuelle {
            if let Some(a) = b.annee_id {
                au = Some(annee_tx(&mut tx, etab, a).await?.1);
            }
        }
        let avertissements = eligibilite_tx(&mut tx, etab, &t, b.agent_id, &b.perimetres, du)
            .await
            .or_else(|e| match e {
                // La simulation montre l'effet ; le refus `bloquer` est signalé, pas levé.
                EntDbError::Refus(_, _) => Ok(vec!["agent_non_eligible_bloquant".to_string()]),
                autre => Err(autre),
            })?;
        let user = user_de_l_agent_tx(&mut tx, etab, b.agent_id).await?;
        let reference = du.max(jour);
        let natures: Vec<String> = b.perimetres.iter().map(|p| p.nature.clone()).collect();
        let cibles: Vec<Option<Uuid>> = b.perimetres.iter().map(|p| p.cible_id).collect();
        let concernes = eleves_du_perimetre_tx(&mut tx, etab, &natures, &cibles, reference).await?;
        let deja: HashSet<Uuid> = match user {
            Some(u) => sqlx::query_as::<_, (Uuid,)>(
                "SELECT eleve_id FROM ent_public_cours_jour($1, $2, NULL, $3, NULL)",
            )
            .bind(etab)
            .bind(reference)
            .bind(u)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
            .into_iter()
            .map(|(e,)| e)
            .collect(),
            None => HashSet::new(),
        };
        let supplementaires = concernes.iter().filter(|e| !deja.contains(e)).count() as i64;
        let actions: Vec<String> = if let Some(code) = &t.modele_code {
            droits_du_modele(code)
                .into_iter()
                .map(|a| a.code().to_string())
                .collect()
        } else if let Some(r) = t.modele_role_id {
            sqlx::query_as::<_, (String,)>(
                "SELECT action FROM ent_role_droit WHERE etablissement_id = $1 AND role_id = $2 ORDER BY action",
            )
            .bind(etab)
            .bind(r)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
            .into_iter()
            .map(|(a,)| a)
            .collect()
        } else {
            Vec::new()
        };
        Ok(SimulationDto {
            eleves_supplementaires: supplementaires,
            eleves_du_perimetre: concernes.len() as i64,
            sans_effet_sur_les_droits: actions.is_empty(),
            actions,
            du,
            au,
            agent_sans_compte: user.is_none(),
            avertissements,
        })
    }

    /// PRS-101 : « Pourquoi peut-il voir cet élève ? » — la chaîne de raisons, à la date du jour de
    /// l'établissement. Réservé à `acces.revue` ; un agent explique SES propres droits sans droit particulier.
    pub async fn expliquer_acces_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<ExplicationAccesDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let moi =
            crate::personnels::agent_du_user_tx(&mut tx, etab, actor).await? == Some(agent_id);
        drop(tx);
        if !moi {
            self.exiger_personnels(etab, actor, role, Action::AccesRevue)
                .await?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        agent_tx(&mut tx, etab, agent_id, false).await?;
        let (eleve_existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !eleve_existe {
            return Err(introuvable());
        }
        let mut raisons: Vec<RaisonAccesDto> = Vec::new();
        let Some(user) = user_de_l_agent_tx(&mut tx, etab, agent_id).await? else {
            return Ok(ExplicationAccesDto {
                agent_id,
                eleve_id,
                date: jour,
                voit: false,
                raisons,
            });
        };
        let role_base: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(etab)
        .bind(user)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        drop(tx);
        // 1. Droits d'établissement (profil de base, rôles attribués) — lecture des élèves.
        if let Some((rb,)) = role_base {
            for d in self.droits_effectifs(etab, user, &rb).await? {
                if d.portee == Portee::Etablissement
                    && matches!(
                        d.action,
                        Action::DossierLire | Action::NotesLire | Action::AbsencesLire
                    )
                {
                    raisons.push(RaisonAccesDto {
                        origine: "role_etablissement".into(),
                        action: Some(d.action.code().to_string()),
                        service_id: None,
                        intervention_nature: None,
                        mission_id: None,
                        mission_type_code: None,
                        groupe_id: None,
                        du: None,
                        au: None,
                        lecture_seule: false,
                    });
                }
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // 2. Interventions : services dont le cours contient l'élève.
        let interventions: Vec<(Uuid, String, NaiveDate, Option<NaiveDate>, bool)> = sqlx::query_as(
            "SELECT s.id, i.nature, i.du, i.au,
                    NOT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $4, c.id, $3, $5, true))
               FROM ent_public_cours_jour($1, $4, NULL, $3, $5, false) p
               JOIN ent_cours c ON c.etablissement_id = $1 AND c.id = p.cours_id
               JOIN ent_service_enseignement s ON s.etablissement_id = c.etablissement_id AND s.cours_id = c.id
                    AND s.etat IN ('ouvert', 'clos')
               JOIN ent_service_intervenant i ON i.etablissement_id = s.etablissement_id AND i.service_id = s.id
                    AND i.annule_le IS NULL AND i.du <= $4 AND (i.au IS NULL OR i.au + 7 >= $4)
                    AND i.nature IN ('titulaire', 'co_enseignant', 'suppleant', 'intervenant', 'assistant')
               JOIN ent_agent ag ON ag.etablissement_id = i.etablissement_id AND ag.id = i.agent_id
               JOIN ent_personne pe ON pe.etablissement_id = ag.etablissement_id AND pe.id = ag.personne_id
                    AND pe.user_id = $3
              ORDER BY i.du",
        )
        .bind(etab)
        .bind(agent_id)
        .bind(user)
        .bind(jour)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (service_id, nature, du, au, lecture) in interventions {
            raisons.push(RaisonAccesDto {
                origine: "intervention".into(),
                action: None,
                service_id: Some(service_id),
                intervention_nature: Some(nature),
                mission_id: None,
                mission_type_code: None,
                groupe_id: None,
                du: Some(du),
                au,
                lecture_seule: lecture,
            });
        }
        // 3. Missions dont le périmètre contient l'élève (cibles datées d'une attribution).
        let missions: Vec<(Uuid, String, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT DISTINCT m.id, t.code, m.du, m.au
               FROM ent_mission m
               JOIN ent_mission_type t ON t.id = m.type_id
               JOIN ent_mission_perimetre p ON p.etablissement_id = m.etablissement_id AND p.mission_id = m.id
              WHERE m.etablissement_id = $1 AND m.agent_id = $2 AND m.annule_le IS NULL
                AND m.du <= $3 AND (m.au IS NULL OR m.au >= $3)
                AND (p.nature = 'etablissement'
                     OR (p.nature = 'eleves' AND p.cible_id = $4)
                     OR EXISTS (SELECT 1 FROM ent_affectation a
                                  WHERE a.etablissement_id = $1 AND a.eleve_id = $4
                                    AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
                                    AND ((p.nature = 'classe' AND a.classe_id = p.cible_id)
                                      OR (p.nature = 'groupe' AND a.groupe_id = p.cible_id)
                                      OR (p.nature IN ('site', 'niveau') AND EXISTS (
                                            SELECT 1 FROM ent_classe k
                                             WHERE k.etablissement_id = $1 AND k.id = a.classe_id
                                               AND ((p.nature = 'site' AND k.site_id = p.cible_id)
                                                 OR (p.nature = 'niveau' AND k.niveau_id = p.cible_id)))))))",
        )
        .bind(etab)
        .bind(agent_id)
        .bind(jour)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (mid, code, du, au) in missions {
            raisons.push(RaisonAccesDto {
                origine: "mission".into(),
                action: None,
                service_id: None,
                intervention_nature: None,
                mission_id: Some(mid),
                mission_type_code: Some(code),
                groupe_id: None,
                du: Some(du),
                au,
                lecture_seule: false,
            });
        }
        // 4. Responsabilités de groupes.
        let groupes: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT r.groupe_id FROM ent_groupe_responsable r
               JOIN ent_affectation a ON a.etablissement_id = r.etablissement_id AND a.groupe_id = r.groupe_id
              WHERE r.etablissement_id = $1 AND r.user_id = $2 AND a.eleve_id = $3
                AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $4)",
        )
        .bind(etab)
        .bind(user)
        .bind(eleve_id)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (g,) in groupes {
            raisons.push(RaisonAccesDto {
                origine: "groupe_responsable".into(),
                action: None,
                service_id: None,
                intervention_nature: None,
                mission_id: None,
                mission_type_code: None,
                groupe_id: Some(g),
                du: None,
                au: None,
                lecture_seule: false,
            });
        }
        // Tracé : une consultation de l'accès d'autrui est une action sensible (identifiants seulement).
        if !moi {
            journal_tx(
                &mut tx,
                etab,
                "acces_explique",
                "Accès expliqué",
                json!({ "agent_id": agent_id, "eleve_id": eleve_id }),
                actor,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(ExplicationAccesDto {
            agent_id,
            eleve_id,
            date: jour,
            voit: !raisons.is_empty(),
            raisons,
        })
    }

    // -----------------------------------------------------------------------------------------
    // Périmètre d'enseignement : lecture seule (bascule du lot P4)
    // -----------------------------------------------------------------------------------------

    /// Le professeur voit-il l'élève en LECTURE seulement (intervention sans saisie effective) ?
    /// Les actions de saisie (`notes.saisir`, `cahier.ecrire`…) ne s'ouvrent pas dans ce cas.
    pub(crate) async fn eleve_en_lecture_seule(
        &self,
        etab: Uuid,
        user_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (r,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $3, NULL, $2, $4, false))
                AND NOT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $3, NULL, $2, $4, true))",
        )
        .bind(etab)
        .bind(user_id)
        .bind(jour)
        .bind(eleve_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(r)
    }

    /// Même règle, en ensemble : les élèves vus en lecture seule (filtre des listes de saisie).
    pub(crate) async fn eleves_en_lecture_seule_du_prof(
        &self,
        etab: Uuid,
        user_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(Uuid,)> =
            sqlx::query_as("SELECT eleve_id FROM ent_eleves_lecture_seule($1, $2, $3)")
                .bind(etab)
                .bind(user_id)
                .bind(jour)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
        Ok(rows.into_iter().map(|(e,)| e).collect())
    }
}

async fn terminer_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    actor: Uuid,
    mission_id: Uuid,
    au: NaiveDate,
) -> Result<(), EntDbError> {
    let (du, type_code, deja): (NaiveDate, String, Option<NaiveDate>) = sqlx::query_as(
        "SELECT m.du, t.code, m.au FROM ent_mission m JOIN ent_mission_type t ON t.id = m.type_id
          WHERE m.etablissement_id = $1 AND m.id = $2 AND m.annule_le IS NULL FOR UPDATE OF m",
    )
    .bind(etab)
    .bind(mission_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    if au < du {
        return Err(invalide("La fin précède le début de la mission."));
    }
    if deja.is_some_and(|d| d <= au) {
        return Err(conflit("La mission se termine déjà à cette date ou avant."));
    }
    sqlx::query("UPDATE ent_mission SET au = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(mission_id)
        .bind(au)
        .execute(&mut **tx)
        .await
        .map_err(erreur_mission)?;
    fermer_attributions_vides_tx(tx, etab, actor, mission_id, au, false).await?;
    let payload = json!({ "mission_id": mission_id, "type": type_code, "au": au });
    journal_tx(
        tx,
        etab,
        "mission_terminee",
        "Mission terminée",
        payload.clone(),
        actor,
    )
    .await?;
    publier_mission_tx(
        tx,
        etab,
        types::MISSION_TERMINEE,
        mission_id,
        payload,
        actor,
    )
    .await?;
    Ok(())
}

/// Élèves du périmètre (natures parallèles aux cibles), affectés et présents à `jour`.
async fn eleves_du_perimetre_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    natures: &[String],
    cibles: &[Option<Uuid>],
    jour: NaiveDate,
) -> Result<Vec<Uuid>, EntDbError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "WITH c AS (SELECT n AS nature, i AS cible FROM unnest($2::text[], $3::uuid[]) AS t(n, i))
         SELECT DISTINCT a.eleve_id
           FROM ent_affectation a
           JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
          WHERE a.etablissement_id = $1
            AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $4)
            AND (e.date_sortie IS NULL OR e.date_sortie > $4)
            AND EXISTS (
                SELECT 1 FROM c
                 WHERE c.nature = 'etablissement'
                    OR (c.nature = 'eleves' AND a.eleve_id = c.cible)
                    OR (c.nature = 'classe' AND a.classe_id = c.cible)
                    OR (c.nature = 'groupe' AND a.groupe_id = c.cible)
                    OR (c.nature IN ('site', 'niveau') AND EXISTS (
                          SELECT 1 FROM ent_classe k
                           WHERE k.etablissement_id = a.etablissement_id AND k.id = a.classe_id
                             AND ((c.nature = 'site' AND k.site_id = c.cible)
                               OR (c.nature = 'niveau' AND k.niveau_id = c.cible)))))",
    )
    .bind(etab)
    .bind(natures)
    .bind(cibles)
    .bind(jour)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|(e,)| e).collect())
}
