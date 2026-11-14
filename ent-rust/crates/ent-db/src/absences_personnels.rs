//! Socle 15, lot A1 — cœur de l'absence d'un personnel
//! (`docs/socles/15_ABSENCES_PERSONNELS.md` §8 à §10, §22, §23, §28.1, §32.1 ; ABP-100 à ABP-139).
//!
//! Une absence est **trois choses à droits différents** (§1) : un fait RH confidentiel (ce module), une
//! conséquence publique sans motif (le port `absences_personnels_ports.rs` et la vue « organisation »),
//! une réponse d'organisation (lot A4). Ce module possède le fait RH.
//!
//! | Commande                                   | Droit (D-017)                                                  | ABP            |
//! |--------------------------------------------|----------------------------------------------------------------|----------------|
//! | `declarer_absence_for_actor`               | soi-même, ou `personnels.absences.declarer` pour un collègue   | 100, 101, 106 à 117 |
//! | `signaler_absence_for_actor`               | `personnels.absences.signaler` (sans nature)                    | 102, 103       |
//! | `confirmer_absence_for_actor`              | l'agent ; par rappel : `lire_organisation` ou `valider`         | 112            |
//! | `valider_absence_for_actor`                | `personnels.absences.valider` ; **jamais l'absent**              | 121            |
//! | `refuser_absence_for_actor`                | `valider` ; seulement si la nature le permet (R-AP12)           | 122            |
//! | `demander_precision_for_actor`             | `valider` ou `instruire`                                        | 123            |
//! | `traiter_` / `clore_absence_for_actor`     | `instruire` (ou `valider` pour clore)                           | 125, 126       |
//! | `rouvrir_absence_for_actor`                | `valider` ; jamais l'absent                                     | 127            |
//! | `requalifier_absence_for_actor`            | `valider` ou `instruire` ; l'agent avant décision               | 128            |
//! | `prolonger_` / `ecourter_absence_for_actor`| l'agent, `instruire` (prolonger), `lire_organisation` (écourter)| 110, 111       |
//! | `annuler_absence_for_actor`                | l'agent avant le début, ou `valider`                            | 113            |
//! | `noter_arret_recu_for_actor`               | `instruire`                                                     | 131            |
//! | `enregistrer_decision_externe_for_actor`   | `instruire` ou `valider`                                        | 288, 289       |
//! | lectures `mes_absences`, `absence`, `historique`, `organisation`, `file_a_valider` | voir chaque fonction | 120, 134, 150 |
//!
//! **Règles tenues ici** : R-AP1 (pas de motif libre : une nature + 200 caractères relus pour les natures
//! non sensibles), R-AP2 (la nature ne sort jamais : événements, journal des décisions et vue organisation
//! ne la portent pas), R-AP4 (une absence par engagement touché), R-AP5 (absent, déclarant, auteur de la
//! saisie : trois colonnes), R-AP7 (faits ajoutés, jamais de réécriture), R-AP8 (rien ne s'efface),
//! R-AP9 (circuit figé), R-AP10 (personne ne valide sa propre absence : service **et** base), R-AP12
//! (natures non discrétionnaires : refus impossible, traitement imposé), R-AP13 (clôture automatique),
//! R-AP14 (retard ≤ 4 h), R-AP16, R-AP20, R-AP41 (mots exclus), R-AP46 (signalement non confirmé),
//! R-AP47 (circuit externe : prise en compte, l'employeur décide).
//!
//! Chaque commande est une transaction `TenantTx` (RLS) : fait + décision + journal + événement. Les
//! événements portent des identifiants et des dates, **jamais la nature ni un texte libre**.

#![allow(clippy::type_complexity)] // lignes de requête SQL lues sous forme de tuples

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use ent_types::absences_personnels::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::personnels::{agent_du_user_tx, conflit, db, ecriture, introuvable, invalide};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Lot A2 : pièces, fil, conservation (migration 2303).
pub mod pieces;

/// Lot A3 : impact, tableaux, files (migration 2304).
pub mod impact;

/// Lot A4 : réponse et sollicitation (migration 2305).
pub mod reponse;

/// Lot A5 : absence longue et suppléance (migration 2306).
pub mod suppleance;

/// Lot A6 : remplacement de courte durée, plan, vivier, remontée hebdomadaire (migration 2307).
pub mod rcd;

// ---------------------------------------------------------------------------------------------
// Événements (identifiants et dates seulement : jamais la nature, jamais un texte libre — R-AP2)
// ---------------------------------------------------------------------------------------------

pub const EVT_SIGNALEE: &str = "absence_personnel.signalee";
pub const EVT_DECLAREE: &str = "absence_personnel.declaree";
pub const EVT_VALIDEE: &str = "absence_personnel.validee";
pub const EVT_REFUSEE: &str = "absence_personnel.refusee";
pub const EVT_PRISE_EN_COMPTE: &str = "absence_personnel.prise_en_compte";
pub const EVT_TRAITEE: &str = "absence_personnel.traitee";
pub const EVT_CLOSE: &str = "absence_personnel.close";
pub const EVT_MODIFIEE: &str = "absence_personnel.modifiee";
pub const EVT_REQUALIFIEE: &str = "absence_personnel.requalifiee";
pub const EVT_ANNULEE: &str = "absence_personnel.annulee";
pub const EVT_REOUVERTE: &str = "absence_personnel.reouverte";

// ---------------------------------------------------------------------------------------------
// Logique pure (testée sans base)
// ---------------------------------------------------------------------------------------------

/// Traitements permis pour un agent de l'État (§9.3).
pub const TRAITEMENTS_ETAT: &[&str] = &[
    "sans_incidence",
    "rattrapage_interne",
    "transmission_rectorat",
    "service_non_fait",
];
/// Traitements permis pour un salarié de l'OGEC (§9.3).
pub const TRAITEMENTS_OGEC: &[&str] = &[
    "sans_incidence",
    "rattrapage",
    "retenue_salaire",
    "maintien_salaire",
    "conge_paye",
];
/// Retard (≤ 4 h) : sans incidence ou rattrapage (§9.3).
const TRAITEMENTS_RETARD: &[&str] = &["sans_incidence", "rattrapage", "rattrapage_interne"];
const VOIES_TRANSMISSION: &[&str] = &["rectorat", "dsden", "paie", "comptabilite", "autre"];
const EMPLOYEURS_EXTERNES: &[&str] = &["rectorat", "dsden", "employeur_mutualisateur", "autre"];
const DECISIONS_EXTERNES: &[&str] = &["accordee", "refusee", "requalifiee"];

/// Mots dont la présence bloque un texte libre (R-AP41) : santé, vie privée, opinions. La liste est
/// volontairement courte et lisible ; elle s'ajoute aux libellés des natures sensibles. Comparaison
/// sans accent ni casse, sur des fragments (« hospital » attrape « hospitalisé »).
const MOTS_EXCLUS: &[&str] = &[
    "maladie",
    "malade",
    "arret de travail",
    "hospital",
    "cancer",
    "depress",
    "burn out",
    "burnout",
    "enceinte",
    "grossesse",
    "maternite",
    "paternite",
    "accident",
    "medecin",
    "medical",
    "chirurg",
    "psychiatr",
    "psycholog",
    "therapie",
    "chimio",
    "syndica",
    "greve",
    "greviste",
    "divorce",
    "deces",
    "decede",
    "obseques",
    "sante",
    "covid",
    "diagnostic",
    "handicap",
    "fausse couche",
];

fn replier(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'œ' => 'o',
            c => c.to_ascii_lowercase(),
        })
        .collect::<String>()
        .replace(['-', '\'', '’'], " ")
}

/// Premier mot exclu trouvé dans un texte destiné à sortir (R-AP41), sinon `None`.
pub fn mot_exclu(texte: &str) -> Option<&'static str> {
    let t = replier(texte);
    MOTS_EXCLUS.iter().find(|m| t.contains(*m)).copied()
}

/// Classe de l'employeur d'un engagement : l'État (agent de l'État, maître) ou l'OGEC (salarié de droit
/// privé de l'organisme de gestion) — §3 point 3. Tout ce qui n'est pas l'organisme de gestion, une
/// association ou une collectivité suit le circuit de l'État (secrétariat).
pub fn classe_employeur(employeur_type: &str) -> &'static str {
    match employeur_type {
        "organisme_gestion" | "association" | "collectivite" => "ogec",
        _ => "etat",
    }
}

/// Traitement des heures d'une validation : imposé par la nature non discrétionnaire (R-AP12), sinon
/// celui choisi par le valideur parmi les traitements permis, sinon le défaut de la nature.
pub fn choisir_traitement(
    nature: &NatureLigne,
    employeur: &str,
    demande: Option<&str>,
) -> Result<String, &'static str> {
    let defaut = if employeur == "ogec" {
        nature.traitement_ogec.as_deref()
    } else {
        nature.traitement_etat.as_deref()
    };
    if nature.traitement_impose {
        return Ok(defaut.unwrap_or("sans_incidence").to_string());
    }
    let permis: &[&str] = if nature.code == "retard" {
        TRAITEMENTS_RETARD
    } else if employeur == "ogec" {
        TRAITEMENTS_OGEC
    } else {
        TRAITEMENTS_ETAT
    };
    match demande {
        Some(t) if permis.contains(&t) => Ok(t.to_string()),
        Some(_) => Err("Ce traitement n'est pas permis pour cet employeur."),
        None => Ok(defaut.unwrap_or("sans_incidence").to_string()),
    }
}

/// `interne` : la direction décide ; `externe` : l'établissement prend en compte, l'employeur décide
/// (R-AP47). Sans nature, la décision est interne.
pub fn mode_decision(nature: Option<&NatureLigne>, employeur: &str) -> &'static str {
    let Some(n) = nature else { return "interne" };
    let d = if employeur == "ogec" {
        &n.decision_ogec
    } else {
        &n.decision_etat
    };
    if d == "externe" {
        "externe"
    } else {
        "interne"
    }
}

/// États depuis lesquels une commande est permise (une seule table, testée).
pub fn etats_permis(commande: &str) -> &'static [&'static str] {
    match commande {
        "confirmer" => &["signalee"],
        "valider" | "refuser" | "instruction" => &["declaree", "en_instruction"],
        "traiter" => &["validee", "prise_en_compte", "decision_employeur"],
        "clore" => &["traitee", "validee"],
        "rouvrir" => &[
            "validee",
            "refusee",
            "prise_en_compte",
            "decision_employeur",
            "traitee",
            "close",
        ],
        "annuler" => &[
            "signalee",
            "declaree",
            "en_instruction",
            "validee",
            "refusee",
            "prise_en_compte",
            "decision_employeur",
            "contestee",
            "traitee",
        ],
        "modifier" => &[
            "signalee",
            "declaree",
            "en_instruction",
            "validee",
            "prise_en_compte",
            "decision_employeur",
            "traitee",
        ],
        "decision_externe" => &["prise_en_compte", "decision_employeur"],
        _ => &[],
    }
}

/// États qui **comptent** pour l'organisation de la journée : l'agent n'est pas là (R-AP15 : même
/// refusée, l'agent n'était pas là). Une absence annulée ne compte plus.
pub const ETATS_COMPTANT: &[&str] = &[
    "signalee",
    "declaree",
    "en_instruction",
    "validee",
    "refusee",
    "prise_en_compte",
    "decision_employeur",
    "contestee",
    "traitee",
    "close",
];

/// États **décidés** : l'absence est acceptée (validée, prise en compte, ou décision de l'employeur).
/// Ce sont eux qui libèrent les séances du port « absences validées non remplacées » (D-058).
pub const ETATS_DECIDES: &[&str] = &[
    "validee",
    "prise_en_compte",
    "decision_employeur",
    "traitee",
    "close",
];

// ---------------------------------------------------------------------------------------------
// Lignes lues
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NatureLigne {
    pub id: Uuid,
    pub code: String,
    pub libelle_agent: String,
    pub famille: String,
    pub employeur: String,
    pub sensible: bool,
    pub refus_possible: bool,
    pub piece_attendue: String,
    pub delai_piece_heures: Option<i16>,
    pub traitement_etat: Option<String>,
    pub traitement_ogec: Option<String>,
    pub traitement_impose: bool,
    pub decision_etat: String,
    pub decision_ogec: String,
    pub texte_libre_permis: bool,
    pub direction_seule: bool,
    pub duree_max_heures: Option<i16>,
}

const NATURE_COLONNES: &str = "n.id, n.code, n.libelle_agent, n.famille, n.employeur, n.sensible,
    n.refus_possible, n.piece_attendue, n.delai_piece_heures, n.traitement_etat, n.traitement_ogec,
    n.traitement_impose, n.decision_etat, n.decision_ogec, n.texte_libre_permis, n.direction_seule,
    n.duree_max_heures";

/// Nature active de l'établissement par son code (celle de l'établissement d'abord, puis la fournie).
async fn nature_par_code(
    conn: &mut PgConnection,
    etab: Uuid,
    code: &str,
) -> Result<Option<NatureLigne>, EntDbError> {
    let sql = format!(
        "SELECT {NATURE_COLONNES} FROM ent_absp_nature n
           LEFT JOIN ent_absp_nature_etab ne ON ne.etablissement_id = $1 AND ne.nature_id = n.id
          WHERE n.code = $2 AND (n.etablissement_id = $1 OR n.etablissement_id IS NULL)
            AND COALESCE(ne.actif, n.actif)
          ORDER BY (n.etablissement_id IS NULL) LIMIT 1"
    );
    sqlx::query_as::<_, NatureLigne>(&sql)
        .bind(etab)
        .bind(code)
        .fetch_optional(conn)
        .await
        .map_err(db)
}

async fn nature_par_id(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<Option<NatureLigne>, EntDbError> {
    // Nature fournie (établissement NULL) ou propre à l'établissement courant : jamais celle d'un autre.
    let sql = format!(
        "SELECT {NATURE_COLONNES} FROM ent_absp_nature n
          WHERE n.id = $2 AND (n.etablissement_id = $1 OR n.etablissement_id IS NULL)"
    );
    sqlx::query_as::<_, NatureLigne>(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct AbsenceLigne {
    id: Uuid,
    agent_id: Uuid,
    employeur: String,
    nature_id: Option<Uuid>,
    texte_libre: Option<String>,
    etat: String,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    traitement: Option<String>,
    absence_liee_id: Option<Uuid>,
    agent_user_id: Option<Uuid>,
}

const ABSENCE_COLONNES: &str = "a.id, a.agent_id, a.employeur, a.nature_id, a.texte_libre, a.etat,
    a.debut, a.fin, a.traitement, a.absence_liee_id, p.user_id AS agent_user_id";

async fn charger(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<AbsenceLigne, EntDbError> {
    let sql = format!(
        "SELECT {ABSENCE_COLONNES} FROM ent_absp_absence a
           JOIN ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
           JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
          WHERE a.etablissement_id = $1 AND a.id = $2 {}",
        if verrou { "FOR UPDATE OF a" } else { "" }
    );
    sqlx::query_as::<_, AbsenceLigne>(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

/// Instant (UTC) d'une date et d'une heure locales de l'établissement (R-AP6). Sans heure : minuit.
async fn instant(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    heure: Option<NaiveTime>,
) -> Result<DateTime<Utc>, EntDbError> {
    let (i,): (DateTime<Utc>,) = sqlx::query_as(
        "SELECT (($2::date + COALESCE($3::time, time '00:00'))::timestamp AT TIME ZONE COALESCE(
            (SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1), 'Europe/Paris'))",
    )
    .bind(etab)
    .bind(jour)
    .bind(heure)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(i)
}

/// Fin (exclusive) d'une plage : l'heure de fin ce jour-là, sinon minuit du lendemain (journée entière).
async fn instant_fin(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    heure: Option<NaiveTime>,
) -> Result<DateTime<Utc>, EntDbError> {
    match heure {
        Some(h) => instant(conn, etab, jour, Some(h)).await,
        None => instant(conn, etab, jour + Duration::days(1), None).await,
    }
}

// ---------------------------------------------------------------------------------------------
// Faits communs : décision, journal, événement
// ---------------------------------------------------------------------------------------------

#[derive(Default)]
struct NouvelleDecision<'a> {
    type_: &'a str,
    etat_avant: Option<&'a str>,
    etat_apres: Option<&'a str>,
    nature_avant: Option<Uuid>,
    nature_apres: Option<Uuid>,
    traitement_avant: Option<&'a str>,
    traitement_apres: Option<&'a str>,
    fin_avant: Option<DateTime<Utc>>,
    fin_apres: Option<DateTime<Utc>>,
    raison_code: Option<&'a str>,
    note: Option<&'a str>,
    voie: Option<&'a str>,
}

async fn decision(
    conn: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
    actor: Uuid,
    d: NouvelleDecision<'_>,
) -> Result<(), EntDbError> {
    let auteur_agent = agent_du_user_tx(conn, etab, actor).await?;
    sqlx::query(
        "INSERT INTO ent_absp_decision
           (id, etablissement_id, absence_id, type, auteur_agent_id, auteur_user_id, etat_avant, etat_apres,
            nature_avant_id, nature_apres_id, traitement_avant, traitement_apres, fin_avant, fin_apres,
            raison_code, note, voie_transmission)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(absence_id)
    .bind(d.type_)
    .bind(auteur_agent)
    .bind(actor)
    .bind(d.etat_avant)
    .bind(d.etat_apres)
    .bind(d.nature_avant)
    .bind(d.nature_apres)
    .bind(d.traitement_avant)
    .bind(d.traitement_apres)
    .bind(d.fin_avant)
    .bind(d.fin_apres)
    .bind(d.raison_code)
    .bind(d.note)
    .bind(d.voie)
    .execute(conn)
    .await
    .map_err(|e| match e {
        // Le déclencheur d'auto-validation (R-AP10) : refus explicite, pas une panne.
        sqlx::Error::Database(ref de) if de.code().as_deref() == Some("23514") => {
            EntDbError::Refus(
                crate::service::RefusMetier::Interdit,
                "Personne ne valide ni ne refuse sa propre absence.",
            )
        }
        e => db(e),
    })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn journal(
    conn: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
    action: &str,
    colonne: Option<&str>,
    avant: Option<&str>,
    apres: Option<&str>,
    actor: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_absp_journal
           (id, etablissement_id, objet, objet_id, action, auteur_user_id, colonne, valeur_avant, valeur_apres)
         VALUES ($1,$2,'absence',$3,$4,$5,$6,$7,$8)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(absence_id)
    .bind(action)
    .bind(actor)
    .bind(colonne)
    .bind(avant)
    .bind(apres)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Événement du domaine dans l'outbox (même transaction). **Jamais** la nature ni un texte libre.
async fn evenement(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    absence_id: Uuid,
    agent_id: Uuid,
    extra: serde_json::Value,
    actor: Uuid,
) -> Result<ent_events::MetierEvent, EntDbError> {
    let mut payload = json!({ "absence_id": absence_id, "agent_id": agent_id, "cause": "rh" });
    if let (Some(p), Some(e)) = (payload.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            p.insert(k.clone(), v.clone());
        }
    }
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::AbsenceRh,
        aggregate_id: absence_id,
        eleve_id: None,
        payload,
        actor_user_id: Some(actor.to_string()),
    })
    .await
    .map_err(EntDbError::from)
}

/// Événement de transition historique, émis **en plus** pendant A1 puis retiré au lot A14 (§24.1).
#[allow(clippy::too_many_arguments)]
async fn evenement_historique(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    absence_id: Uuid,
    user_id: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
    actor: Uuid,
) -> Result<ent_events::MetierEvent, EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::AbsenceRh,
        aggregate_id: absence_id,
        eleve_id: None,
        payload: json!({
            "user_id": user_id, "debut": debut.to_string(), "fin": fin.to_string(),
        }),
        actor_user_id: Some(actor.to_string()),
    })
    .await
    .map_err(EntDbError::from)
}

fn texte_borne(
    t: &Option<String>,
    max: usize,
    message: &'static str,
) -> Result<Option<String>, EntDbError> {
    match t.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) if s.chars().count() > max => Err(invalide(message)),
        Some(s) => {
            if mot_exclu(s).is_some() {
                return Err(invalide(
                    "Ce texte contient un mot qui ne doit pas figurer ici (santé, vie privée, \
                     opinion) : reformulez, ou choisissez la nature qui convient.",
                ));
            }
            Ok(Some(s.to_string()))
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Circuit (§9.4) : figé à la création
// ---------------------------------------------------------------------------------------------

/// Calcule et fige le circuit : valideur (règle de l'établissement, sinon le chef d'établissement),
/// instructeurs et informés nommés par règle. **Jamais l'absent** (R-AP10).
async fn figer_circuit(
    conn: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
    agent_id: Uuid,
    employeur: &str,
    famille: Option<&str>,
) -> Result<(), EntDbError> {
    let regles: Vec<(String, Uuid, i16)> = sqlx::query_as(
        "SELECT role, agent_id, rang FROM ent_absp_circuit_regle
          WHERE etablissement_id = $1 AND actif AND employeur IN ($2, 'tous')
            AND (famille IS NULL OR famille = $3)
          ORDER BY rang, cree_le",
    )
    .bind(etab)
    .bind(employeur)
    .bind(famille)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut a_valideur = false;
    for (role, agent, rang) in &regles {
        if *agent == agent_id {
            continue; // jamais l'absent : on « remonte d'un cran » (R-AP9)
        }
        if role == "valideur" {
            a_valideur = true;
        }
        inserer_circuit(conn, etab, absence_id, role, *agent, *rang, "regle").await?;
    }
    if !a_valideur {
        // À défaut de règle : le chef d'établissement (rôle de base « direction »), hors l'absent.
        let chefs: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT a.id FROM ent_membership m
               JOIN ent_personne p ON p.etablissement_id = m.etablissement_id AND p.user_id = m.user_id
               JOIN ent_agent a ON a.etablissement_id = p.etablissement_id AND a.personne_id = p.id
                                AND a.fusionne_dans_agent_id IS NULL
              WHERE m.etablissement_id = $1 AND m.role = 'direction' AND m.etat = 'ouvert' AND a.id <> $2",
        )
        .bind(etab)
        .bind(agent_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (chef,) in chefs {
            inserer_circuit(conn, etab, absence_id, "valideur", chef, 1, "defaut").await?;
        }
    }
    Ok(())
}

async fn inserer_circuit(
    conn: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
    role: &str,
    agent: Uuid,
    rang: i16,
    source: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_absp_circuit (id, etablissement_id, absence_id, role, agent_id, rang, source)
         VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (absence_id, role, agent_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(absence_id)
    .bind(role)
    .bind(agent)
    .bind(rang)
    .bind(source)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Création : déclarer, signaler (une absence par engagement touché — R-AP4)
// ---------------------------------------------------------------------------------------------

struct PlageCalculee {
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
}

struct Creation<'a> {
    agent_id: Uuid,
    nature: Option<&'a NatureLigne>,
    texte_libre: Option<String>,
    plages: Vec<PlageCalculee>,
    fin_provisoire: bool,
    preference_traitement: Option<String>,
    canal: &'a str,
    /// Confirmée par l'agent lui-même à la création.
    confirmee: bool,
    engagement_filtre: Option<Uuid>,
    /// Ancien motif libre d'une route historique : zone de qualification réservée à la direction, vidée
    /// à la qualification (Q-AP15). Jamais lu hors de `lire_nature`.
    zone_qualification: Option<String>,
}

impl PgEnt {
    async fn exiger_personnel_abs(&self, role: &str) -> Result<Role, EntDbError> {
        let r = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !r.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        Ok(r)
    }

    async fn a_droit_abs(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        action: Action,
    ) -> Result<bool, EntDbError> {
        self.a_action_etablissement(etab, actor, role, action).await
    }

    /// Plages de dates (inclusives) → instants. Contrôle l'ordre, la durée (≤ 366 jours) et l'absence de
    /// chevauchement.
    #[allow(clippy::too_many_arguments)]
    async fn plages_depuis(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
        heure_debut: Option<NaiveTime>,
        heure_fin: Option<NaiveTime>,
        plages: &[PlageBody],
    ) -> Result<Vec<PlageCalculee>, EntDbError> {
        let sources: Vec<PlageBody> = if plages.is_empty() {
            vec![PlageBody {
                debut,
                fin,
                heure_debut,
                heure_fin,
            }]
        } else {
            plages.to_vec()
        };
        if sources.len() > 60 {
            return Err(invalide("Trop de plages pour une même absence."));
        }
        let mut out: Vec<PlageCalculee> = Vec::new();
        for p in &sources {
            if p.fin < p.debut || (p.fin - p.debut).num_days() > 366 {
                return Err(invalide("Les dates de l'absence sont incohérentes."));
            }
            let d = instant(conn, etab, p.debut, p.heure_debut).await?;
            let f = instant_fin(conn, etab, p.fin, p.heure_fin).await?;
            if f <= d {
                return Err(invalide("L'heure de fin doit suivre l'heure de début."));
            }
            out.push(PlageCalculee { debut: d, fin: f });
        }
        out.sort_by_key(|p| p.debut);
        if out.windows(2).any(|w| w[0].fin > w[1].debut) {
            return Err(invalide("Deux plages de la même absence se chevauchent."));
        }
        Ok(out)
    }

    /// Crée une absence par engagement touché (une seule si l'agent n'en a qu'un, ou aucun). Renvoie
    /// les identifiants, les événements à diffuser et les chevauchements à signaler.
    async fn creer_absences_tx(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        c: Creation<'_>,
    ) -> Result<(Vec<Uuid>, Vec<ent_events::MetierEvent>, Vec<Uuid>), EntDbError> {
        let debut = c
            .plages
            .first()
            .map(|p| p.debut)
            .ok_or_else(|| invalide("Aucune plage."))?;
        let fin = c
            .plages
            .iter()
            .map(|p| p.fin)
            .max()
            .ok_or_else(|| invalide("Aucune plage."))?;
        // Engagements courants au premier jour de l'absence (fuseau de l'établissement).
        let (jour_debut,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, $2)")
            .bind(etab)
            .bind(debut)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let engagements: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, employeur_type FROM ent_agent_engagement
              WHERE etablissement_id = $1 AND agent_id = $2 AND annule_le IS NULL
                AND du <= $3 AND (au IS NULL OR au >= $3)
              ORDER BY du, id",
        )
        .bind(etab)
        .bind(c.agent_id)
        .bind(jour_debut)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // Un engagement par classe d'employeur (deux engagements de même classe : un seul circuit).
        let mut visees: Vec<(Option<Uuid>, &'static str)> = Vec::new();
        for (id, ty) in &engagements {
            let classe = classe_employeur(ty);
            if let Some(f) = c.engagement_filtre {
                if f != *id {
                    continue;
                }
            }
            if !visees.iter().any(|(_, k)| *k == classe) {
                visees.push((Some(*id), classe));
            }
        }
        if let Some(f) = c.engagement_filtre {
            if !engagements.iter().any(|(id, _)| *id == f) {
                return Err(invalide("Cet engagement n'est pas courant pour cet agent."));
            }
        }
        if visees.is_empty() {
            visees.push((None, "etat")); // agent sans engagement courant : circuit de l'État
        }
        // La nature doit être permise pour la classe d'employeur (une nature d'OGEC seulement ne se
        // déclare pas pour un agent de l'État).
        let mut ids: Vec<Uuid> = Vec::new();
        let mut evenements = Vec::new();
        let (d1, d2) = (debut, fin);
        let (jour_1, jour_2): (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT (ent_aujourdhui_etab($1, $2)), (ent_aujourdhui_etab($1, $3::timestamptz - interval '1 microsecond'))",
        )
        .bind(etab)
        .bind(d1)
        .bind(d2)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let (user_agent,): (Option<Uuid>,) = sqlx::query_as(
            "SELECT p.user_id FROM ent_agent a
               JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
              WHERE a.etablissement_id = $1 AND a.id = $2",
        )
        .bind(etab)
        .bind(c.agent_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let declarant_agent = agent_du_user_tx(tx, etab, actor).await?;
        let mut premiere: Option<Uuid> = None;
        for (engagement, classe) in visees {
            if let Some(n) = c.nature {
                if n.employeur != "tous" && n.employeur != classe {
                    continue;
                }
            }
            let id = Uuid::new_v4();
            let etat = if c.confirmee { "declaree" } else { "signalee" };
            sqlx::query(
                "INSERT INTO ent_absp_absence
                   (id, etablissement_id, agent_id, engagement_id, employeur, cause, nature_id, texte_libre,
                    etat, debut, fin, fin_provisoire, declare_par_agent_id, saisie_par_user_id, saisie_canal,
                    preference_traitement, confirmee_par, confirmee_le, absence_liee_id, zone_qualification)
                 VALUES ($1,$2,$3,$4,$5,'rh',$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)",
            )
            .bind(id)
            .bind(etab)
            .bind(c.agent_id)
            .bind(engagement)
            .bind(classe)
            .bind(c.nature.map(|n| n.id))
            .bind(&c.texte_libre)
            .bind(etat)
            .bind(debut)
            .bind(fin)
            .bind(c.fin_provisoire)
            .bind(declarant_agent)
            .bind(actor)
            .bind(c.canal)
            .bind(&c.preference_traitement)
            .bind(c.confirmee.then_some("agent"))
            .bind(c.confirmee.then(Utc::now))
            .bind(premiere)
            .bind(&c.zone_qualification)
            .execute(&mut *tx)
            .await
            .map_err(ecriture)?;
            for p in &c.plages {
                sqlx::query(
                    "INSERT INTO ent_absp_plage (id, etablissement_id, absence_id, debut, fin)
                     VALUES ($1,$2,$3,$4,$5)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(id)
                .bind(p.debut)
                .bind(p.fin)
                .execute(&mut *tx)
                .await
                .map_err(ecriture)?;
            }
            figer_circuit(
                tx,
                etab,
                id,
                c.agent_id,
                classe,
                c.nature.map(|n| n.famille.as_str()),
            )
            .await?;
            // Arrêt attendu : la ligne d'arrêt initial porte l'échéance de transmission (R-AP16).
            if let Some(n) = c.nature {
                if n.piece_attendue == "arret" {
                    sqlx::query(
                        "INSERT INTO ent_absp_arret (id, etablissement_id, absence_id, type, du, au, echeance_le, cree_par_user_id)
                         VALUES ($1,$2,$3,'initial',$4,$5,now() + make_interval(hours => $6), $7)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(etab)
                    .bind(id)
                    .bind(jour_1)
                    .bind(jour_2)
                    .bind(i32::from(n.delai_piece_heures.unwrap_or(48)))
                    .bind(actor)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                }
            }
            decision(
                tx,
                etab,
                id,
                actor,
                NouvelleDecision {
                    type_: if etat == "signalee" {
                        "signalement"
                    } else {
                        "declaration"
                    },
                    etat_apres: Some(etat),
                    nature_apres: c.nature.map(|n| n.id),
                    fin_apres: Some(fin),
                    ..Default::default()
                },
            )
            .await?;
            journal(tx, etab, id, "creee", Some("etat"), None, Some(etat), actor).await?;
            let type_evt = if etat == "signalee" {
                EVT_SIGNALEE
            } else {
                EVT_DECLAREE
            };
            evenements.push(
                evenement(
                    tx,
                    etab,
                    type_evt,
                    id,
                    c.agent_id,
                    json!({ "debut": debut, "fin": fin, "plages": c.plages.len(), "engagement_id": engagement }),
                    actor,
                )
                .await?,
            );
            if let Some(u) = user_agent {
                evenements.push(
                    evenement_historique(
                        tx,
                        etab,
                        ent_events::types::ABSENCE_RH,
                        id,
                        u,
                        jour_1,
                        jour_2,
                        actor,
                    )
                    .await?,
                );
            }
            if premiere.is_none() {
                premiere = Some(id);
            }
            ids.push(id);
        }
        if ids.is_empty() {
            return Err(invalide(
                "Cette nature ne s'applique pas à l'employeur de cet agent.",
            ));
        }
        // Les absences d'un double engagement se référencent entre elles (la première pointe la seconde).
        if ids.len() > 1 {
            sqlx::query("UPDATE ent_absp_absence SET absence_liee_id = $3 WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(ids[0])
                .bind(ids[1])
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let chevauchements: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_absp_absence
              WHERE etablissement_id = $1 AND agent_id = $2 AND cause = 'rh'
                AND etat NOT IN ('annulee', 'refusee') AND debut < $4 AND fin > $3
                AND id <> ALL($5)
              ORDER BY debut",
        )
        .bind(etab)
        .bind(c.agent_id)
        .bind(debut)
        .bind(fin)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok((ids, evenements, chevauchements))
    }

    /// ABP-100, 101, 106 à 109, 114 à 117 : déclare une absence (la sienne, ou celle d'un collègue avec
    /// `personnels.absences.declarer`). Une absence par engagement touché (R-AP4).
    pub async fn declarer_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &DeclarerAbsenceBody,
    ) -> Result<DeclarationDto, EntDbError> {
        self.declarer_absence_interne(etab, actor, role, body, None)
            .await
    }

    /// Variante interne : `zone` est l'ancien motif libre d'une route historique (jamais exposée).
    pub(crate) async fn declarer_absence_interne(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &DeclarerAbsenceBody,
        zone: Option<String>,
    ) -> Result<DeclarationDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let agent_id = match body.agent_id.or(mon_agent) {
            Some(a) => a,
            None => return Err(invalide("Aucun agent n'est rattaché à ce compte.")),
        };
        let pour_moi = Some(agent_id) == mon_agent;
        if !pour_moi
            && !self
                .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesDeclarer)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2
                             AND fusionne_dans_agent_id IS NULL)",
        )
        .bind(etab)
        .bind(agent_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let code = body
            .nature_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("inconnue");
        let nature = nature_par_code(&mut tx, etab, code)
            .await?
            .ok_or_else(|| invalide("Nature inconnue ou désactivée."))?;
        if nature.code == "autre_service" {
            return Err(invalide(
                "Cette nature est posée par un autre module, pas à la main.",
            ));
        }
        if nature.direction_seule
            && !self
                .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let texte = texte_borne(
            &body.texte_libre,
            200,
            "Le texte est limité à 200 caractères.",
        )?;
        if texte.is_some() && !nature.texte_libre_permis {
            return Err(invalide("Pas de texte libre pour cette nature (R-AP1)."));
        }
        let plages = self
            .plages_depuis(
                &mut tx,
                etab,
                body.debut,
                body.fin,
                body.heure_debut,
                body.heure_fin,
                &body.plages,
            )
            .await?;
        // R-AP14 : un retard dure au plus 4 h, le même jour.
        if let Some(max) = nature.duree_max_heures {
            let (d, f) = (
                plages[0].debut,
                plages.last().map(|p| p.fin).unwrap_or(plages[0].fin),
            );
            if plages.len() > 1 || f - d > Duration::hours(i64::from(max)) {
                return Err(invalide(
                    "Un retard dure moins de 4 h : déclarez plutôt une absence.",
                ));
            }
        }
        let pref = body
            .preference_traitement
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(p) = &pref {
            if !TRAITEMENTS_ETAT.contains(&p.as_str()) && !TRAITEMENTS_OGEC.contains(&p.as_str()) {
                return Err(invalide("Préférence de traitement inconnue."));
            }
        }
        let (ids, evenements, chevauchements) = self
            .creer_absences_tx(
                &mut tx,
                etab,
                actor,
                Creation {
                    agent_id,
                    nature: Some(&nature),
                    texte_libre: texte,
                    plages,
                    fin_provisoire: body.fin_provisoire,
                    preference_traitement: pref,
                    canal: if pour_moi { "agent" } else { "collegue" },
                    confirmee: pour_moi,
                    engagement_filtre: body.engagement_id,
                    zone_qualification: zone,
                },
            )
            .await?;
        let montrer = pour_moi
            || self
                .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
                .await?;
        let mut absences = Vec::new();
        for id in &ids {
            absences.push(vue_absence(&mut tx, etab, *id, montrer).await?);
        }
        for id in &ids {
            self.recalculer_impact_tolerant(&mut tx, etab, *id, actor).await;
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evenements);
        Ok(DeclarationDto {
            absences,
            chevauchements,
        })
    }

    /// ABP-102, 103 : signale l'absence d'un agent **sans nature** (accueil, vie scolaire, direction).
    /// Aucun effet extérieur tant qu'elle n'est pas confirmée (R-AP46).
    pub async fn signaler_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &SignalerAbsenceBody,
    ) -> Result<DeclarationDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesSignaler)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2
                             AND fusionne_dans_agent_id IS NULL)",
        )
        .bind(etab)
        .bind(body.agent_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let plages = self
            .plages_depuis(
                &mut tx,
                etab,
                body.debut,
                body.fin,
                body.heure_debut,
                body.heure_fin,
                &[],
            )
            .await?;
        let canal = if matches!(Role::parse(role), Some(Role::Accueil)) {
            "accueil"
        } else {
            "collegue"
        };
        let (ids, evenements, chevauchements) = self
            .creer_absences_tx(
                &mut tx,
                etab,
                actor,
                Creation {
                    agent_id: body.agent_id,
                    nature: None,
                    texte_libre: None,
                    plages,
                    fin_provisoire: true,
                    preference_traitement: None,
                    canal,
                    confirmee: false,
                    engagement_filtre: None,
                    zone_qualification: None,
                },
            )
            .await?;
        // Sans nature : la vue ne porte jamais la nature (il n'y en a pas) ; on rend la vue cercle RH
        // seulement si l'appelant peut la lire, sinon un DTO minimal.
        let mut absences = Vec::new();
        for id in &ids {
            absences.push(vue_absence(&mut tx, etab, *id, false).await?);
        }
        for id in &ids {
            self.recalculer_impact_tolerant(&mut tx, etab, *id, actor).await;
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evenements);
        Ok(DeclarationDto {
            absences,
            chevauchements,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Vues
// ---------------------------------------------------------------------------------------------

async fn circuit_de(
    conn: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
) -> Result<Vec<CircuitLigneDto>, EntDbError> {
    let rows: Vec<(String, Uuid, i16, String)> = sqlx::query_as(
        "SELECT role, agent_id, rang, source FROM ent_absp_circuit
          WHERE etablissement_id = $1 AND absence_id = $2 ORDER BY role, rang, cree_le",
    )
    .bind(etab)
    .bind(absence_id)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|(role, agent_id, rang, source)| CircuitLigneDto {
            role,
            agent_id,
            rang,
            source,
        })
        .collect())
}

/// Vue « cercle RH » d'une absence. `montrer_nature = false` : ni la nature, ni le texte libre, ni le
/// traitement ne sont rendus.
async fn vue_absence(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    montrer_nature: bool,
) -> Result<AbsenceDto, EntDbError> {
    let l = charger(conn, etab, id, false).await?;
    let nature = match l.nature_id {
        Some(n) => nature_par_id(conn, etab, n).await?,
        None => None,
    };
    let (label, fin_provisoire, confirmee_par): (Option<String>, bool, Option<String>) = sqlx::query_as(
        "SELECT btrim(coalesce(p.prenom, '') || ' ' || p.nom), a.fin_provisoire, a.confirmee_par
           FROM ent_absp_absence a
           JOIN ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
           JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
          WHERE a.etablissement_id = $1 AND a.id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let circuit = circuit_de(conn, etab, id).await?;
    Ok(AbsenceDto {
        id: l.id,
        agent_id: l.agent_id,
        agent_label: label,
        etat: l.etat.clone(),
        debut: l.debut,
        fin: l.fin,
        fin_provisoire,
        nature_code: montrer_nature
            .then(|| nature.as_ref().map(|n| n.code.clone()))
            .flatten(),
        nature_libelle: montrer_nature
            .then(|| nature.as_ref().map(|n| n.libelle_agent.clone()))
            .flatten(),
        texte_libre: if montrer_nature {
            l.texte_libre.clone()
        } else {
            None
        },
        traitement: if montrer_nature {
            l.traitement.clone()
        } else {
            None
        },
        decision: mode_decision(nature.as_ref(), &l.employeur).to_string(),
        confirmee_par,
        absence_liee_id: l.absence_liee_id,
        circuit,
    })
}

// ---------------------------------------------------------------------------------------------
// Décisions et modifications
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cmd {
    Valider,
    Refuser,
    Instruction,
    Traiter,
    Clore,
    Rouvrir,
    Annuler,
}

impl Cmd {
    fn nom(self) -> &'static str {
        match self {
            Self::Valider => "valider",
            Self::Refuser => "refuser",
            Self::Instruction => "instruction",
            Self::Traiter => "traiter",
            Self::Clore => "clore",
            Self::Rouvrir => "rouvrir",
            Self::Annuler => "annuler",
        }
    }
}

fn instruit(r: Role, employeur: &str) -> bool {
    match r {
        Role::Direction => true,
        Role::Administratif => employeur == "etat",
        Role::Compta => employeur == "ogec",
        _ => true,
    }
}

impl PgEnt {
    /// L'acteur peut-il instruire cette absence (droit `instruire`, cloisonné par employeur : le
    /// secrétariat ne voit que l'État, la comptabilité que l'OGEC — §23.3), ou est-il désigné au
    /// circuit comme instructeur ?
    async fn peut_instruire_abs(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        a: &AbsenceLigne,
    ) -> Result<bool, EntDbError> {
        let r = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesInstruire)
            .await?
            && instruit(r, &a.employeur)
        {
            return Ok(true);
        }
        let Some(mon_agent) = agent_du_user_tx(conn, etab, actor).await? else {
            return Ok(false);
        };
        let (nommé,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_absp_circuit WHERE etablissement_id = $1 AND absence_id = $2
                             AND role = 'instructeur' AND agent_id = $3)",
        )
        .bind(etab)
        .bind(a.id)
        .bind(mon_agent)
        .fetch_one(conn)
        .await
        .map_err(db)?;
        Ok(nommé)
    }

    /// Cœur commun des décisions du valideur et de l'instructeur.
    async fn decider_absence(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        cmd: Cmd,
        body: &DecisionBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let valider_droit = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
            .await?;
        let lire_nature = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, id, true).await?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let suis_absent = mon_agent == Some(a.agent_id) || a.agent_user_id == Some(actor);
        let instructeur = self
            .peut_instruire_abs(&mut tx, etab, actor, role, &a)
            .await?;
        let nature = match a.nature_id {
            Some(n) => nature_par_id(&mut tx, etab, n).await?,
            None => None,
        };
        // Droits par commande. Personne ne valide ni ne refuse sa propre absence (R-AP10), même s'il
        // détient `valider` ; l'agent peut annuler la sienne avant son début (ABP-113).
        let autorise = match cmd {
            Cmd::Valider | Cmd::Refuser | Cmd::Rouvrir => valider_droit && !suis_absent,
            Cmd::Instruction => (valider_droit || instructeur) && !suis_absent,
            Cmd::Traiter => instructeur && !suis_absent,
            Cmd::Clore => (valider_droit || instructeur) && !suis_absent,
            Cmd::Annuler => {
                (valider_droit && !suis_absent) || (suis_absent && a.debut > Utc::now())
            }
        };
        if !autorise {
            // Une tentative sur sa propre absence est un refus explicite ; le reste est un 403 net.
            if suis_absent
                && matches!(cmd, Cmd::Valider | Cmd::Refuser | Cmd::Rouvrir)
                && valider_droit
            {
                return Err(EntDbError::Refus(
                    crate::service::RefusMetier::Interdit,
                    "Personne ne valide ni ne refuse sa propre absence.",
                ));
            }
            return Err(EntDbError::Forbidden);
        }
        let cible: &str = match cmd {
            Cmd::Valider => "validee",
            Cmd::Refuser => "refusee",
            Cmd::Instruction => "en_instruction",
            Cmd::Traiter => "traitee",
            Cmd::Clore => "close",
            Cmd::Rouvrir => "declaree",
            Cmd::Annuler => "annulee",
        };
        // Idempotence : l'état demandé est déjà posé.
        if a.etat == cible || (cmd == Cmd::Valider && ETATS_DECIDES.contains(&a.etat.as_str())) {
            let v = vue_absence(&mut tx, etab, id, lire_nature || suis_absent).await?;
            return Ok(v);
        }
        if a.etat == "signalee" && matches!(cmd, Cmd::Valider | Cmd::Refuser | Cmd::Instruction) {
            return Err(conflit(
                "Cette absence n'est pas encore confirmée par l'agent : confirmez-la d'abord.",
            ));
        }
        if !etats_permis(cmd.nom()).contains(&a.etat.as_str()) {
            return Err(conflit(
                "Cette absence n'est pas dans un état qui permet cette action.",
            ));
        }
        let note = texte_borne(&body.note, 1000, "La note est limitée à 1000 caractères.")?;
        let raison = body
            .raison_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(rc) = &raison {
            if rc.len() > 60
                || !rc
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                return Err(invalide("Raison invalide : un code court en minuscules."));
            }
        }
        let mut etat_final = cible.to_string();
        let mut traitement: Option<String> = a.traitement.clone();
        let mut type_decision = match cmd {
            Cmd::Valider => "validation",
            Cmd::Refuser => "refus",
            Cmd::Instruction => "instruction",
            Cmd::Traiter => "traitement",
            Cmd::Clore => "cloture",
            Cmd::Rouvrir => "reouverture",
            Cmd::Annuler => "annulation",
        };
        let mut evt = match cmd {
            Cmd::Valider => EVT_VALIDEE,
            Cmd::Refuser => EVT_REFUSEE,
            Cmd::Traiter => EVT_TRAITEE,
            Cmd::Clore => EVT_CLOSE,
            Cmd::Rouvrir => EVT_REOUVERTE,
            Cmd::Annuler => EVT_ANNULEE,
            Cmd::Instruction => EVT_MODIFIEE,
        };
        match cmd {
            Cmd::Valider => {
                let nat = nature.as_ref();
                let externe = mode_decision(nat, &a.employeur) == "externe";
                // Une absence sans nature précisée se valide avec la nature « inconnue » : le traitement
                // par défaut est sans incidence (la qualification viendra).
                let t = match nat {
                    Some(n) => choisir_traitement(n, &a.employeur, body.traitement.as_deref())
                        .map_err(invalide)?,
                    None => body
                        .traitement
                        .clone()
                        .unwrap_or_else(|| "sans_incidence".into()),
                };
                traitement = Some(t.clone());
                if externe {
                    // R-AP47 : l'établissement prend en compte et transmet ; l'employeur décide.
                    etat_final = "prise_en_compte".into();
                    type_decision = "prise_en_compte";
                    evt = EVT_PRISE_EN_COMPTE;
                }
                // R-AP13 (clôture automatique quand rien n'est dû) dépend des heures et rattrapages
                // (lots A7, A8) : en A1 la clôture est une décision distincte (`clore`).
            }
            Cmd::Refuser => {
                // R-AP12 : refus impossible pour une nature non discrétionnaire ; R-AP47 : jamais
                // proposé à l'établissement quand l'employeur décide.
                let refus_ok = nature.as_ref().is_none_or(|n| n.refus_possible)
                    && mode_decision(nature.as_ref(), &a.employeur) == "interne";
                if !refus_ok {
                    return Err(conflit(
                        "Cette absence ne peut pas être refusée : la nature l'interdit, ou l'employeur décide.",
                    ));
                }
                traitement = Some("service_non_fait".into()); // R-AP15 : « non autorisée » ; retenue = décision humaine
            }
            Cmd::Traiter => {
                if let Some(v) = body.voie_transmission.as_deref() {
                    if !VOIES_TRANSMISSION.contains(&v) {
                        return Err(invalide("Voie de transmission inconnue."));
                    }
                }
            }
            Cmd::Clore => {
                // R-AP13 : clôture interdite si un traitement est dû (transmission, retenue, maintien…).
                let du = !matches!(
                    a.traitement.as_deref(),
                    None | Some("sans_incidence") | Some("rattrapage_interne") | Some("rattrapage")
                ) && a.etat != "traitee";
                if du {
                    return Err(conflit(
                        "Un traitement administratif est dû : traitez l'absence avant de la clore.",
                    ));
                }
            }
            Cmd::Rouvrir => {
                traitement = None;
            }
            Cmd::Annuler | Cmd::Instruction => {}
        }
        decision(
            &mut tx,
            etab,
            id,
            actor,
            NouvelleDecision {
                type_: type_decision,
                etat_avant: Some(&a.etat),
                etat_apres: Some(&etat_final),
                traitement_avant: a.traitement.as_deref(),
                traitement_apres: traitement.as_deref(),
                raison_code: raison.as_deref(),
                note: note.as_deref(),
                voie: body
                    .voie_transmission
                    .as_deref()
                    .filter(|_| cmd == Cmd::Traiter),
                ..Default::default()
            },
        )
        .await?;
        sqlx::query(
            "UPDATE ent_absp_absence SET etat = $3, traitement = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&etat_final)
        .bind(&traitement)
        .execute(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref de) if de.code().as_deref() == Some("23514") => {
                EntDbError::Refus(
                    crate::service::RefusMetier::Interdit,
                    "Personne ne valide ni ne refuse sa propre absence.",
                )
            }
            e => db(e),
        })?;
        journal(
            &mut tx,
            etab,
            id,
            type_decision,
            Some("etat"),
            Some(&a.etat),
            Some(&etat_final),
            actor,
        )
        .await?;
        if a.traitement != traitement {
            journal(
                &mut tx,
                etab,
                id,
                type_decision,
                Some("traitement"),
                a.traitement.as_deref(),
                traitement.as_deref(),
                actor,
            )
            .await?;
        }
        let mut evenements = vec![
            evenement(
                &mut tx,
                etab,
                evt,
                id,
                a.agent_id,
                json!({ "etat": etat_final }),
                actor,
            )
            .await?,
        ];
        // Événement historique `rh.absence_validee` (transition, §24.1).
        if cmd == Cmd::Valider {
            if let Some(u) = a.agent_user_id {
                let (d1, d2): (NaiveDate, NaiveDate) = sqlx::query_as(
                    "SELECT ent_aujourdhui_etab($1, $2), ent_aujourdhui_etab($1, $3::timestamptz - interval '1 microsecond')",
                )
                .bind(etab)
                .bind(a.debut)
                .bind(a.fin)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                evenements.push(
                    evenement_historique(
                        &mut tx,
                        etab,
                        ent_events::types::ABSENCE_RH_VALIDEE,
                        id,
                        u,
                        d1,
                        d2,
                        actor,
                    )
                    .await?,
                );
            }
        }
        let vue = vue_absence(&mut tx, etab, id, lire_nature || suis_absent).await?;
        // Impact : une validation, un refus, une réouverture ou une annulation changent ce que l'absence touche.
        self.recalculer_impact_tolerant(&mut tx, etab, id, actor).await;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evenements);
        Ok(vue)
    }

    /// ABP-121 : valide (ou, si l'employeur décide, **prend en compte**, R-AP47). Jamais l'absent.
    pub async fn valider_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.decider_absence(etab, actor, role, id, Cmd::Valider, body)
            .await
    }

    /// ABP-122 : refuse, seulement si la nature le permet (R-AP12) et si l'établissement décide (R-AP47).
    pub async fn refuser_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.decider_absence(etab, actor, role, id, Cmd::Refuser, body)
            .await
    }

    /// ABP-123 : demande une pièce ou une précision (`en_instruction`).
    pub async fn demander_precision_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.decider_absence(etab, actor, role, id, Cmd::Instruction, body)
            .await
    }

    /// ABP-125 : traite administrativement (transmis au rectorat, à la paie).
    pub async fn traiter_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.decider_absence(etab, actor, role, id, Cmd::Traiter, body)
            .await
    }

    /// ABP-126 : clôture (interdite si un traitement est dû, R-AP13).
    pub async fn clore_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.decider_absence(etab, actor, role, id, Cmd::Clore, body)
            .await
    }

    /// ABP-127 : rouvre (retour à `declaree`) ; jamais l'absent.
    pub async fn rouvrir_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.decider_absence(etab, actor, role, id, Cmd::Rouvrir, body)
            .await
    }

    /// ABP-113 : annule (saisie par erreur, n'a pas eu lieu) ; rien n'est supprimé.
    pub async fn annuler_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.decider_absence(etab, actor, role, id, Cmd::Annuler, body)
            .await
    }

    /// ABP-112 : l'agent confirme une absence signalée ; il peut en préciser la nature (ABP-117).
    /// Par rappel tracé (R-AP46) : l'adjoint ou la direction confirme après avoir appelé l'agent.
    pub async fn confirmer_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &ConfirmerBody,
        par_rappel: bool,
    ) -> Result<AbsenceDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, id, true).await?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let suis_absent = mon_agent == Some(a.agent_id) || a.agent_user_id == Some(actor);
        if par_rappel {
            let ok = !suis_absent
                && (self
                    .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
                    .await?
                    || self
                        .a_droit_abs(
                            etab,
                            actor,
                            role,
                            Action::PersonnelsAbsencesLireOrganisation,
                        )
                        .await?);
            if !ok {
                return Err(EntDbError::Forbidden);
            }
        } else if !suis_absent {
            return Err(EntDbError::Forbidden);
        }
        if a.etat != "signalee" {
            let v = vue_absence(&mut tx, etab, id, suis_absent).await?;
            return Ok(v); // déjà confirmée : idempotent
        }
        let nature = match body
            .nature_code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(code) => {
                let n = nature_par_code(&mut tx, etab, code)
                    .await?
                    .ok_or_else(|| invalide("Nature inconnue ou désactivée."))?;
                if n.direction_seule || n.code == "autre_service" {
                    return Err(invalide("Cette nature n'est pas à choisir ici."));
                }
                if n.employeur != "tous" && n.employeur != a.employeur {
                    return Err(invalide("Cette nature ne s'applique pas à cet employeur."));
                }
                Some(n)
            }
            None => None,
        };
        // Sans nature précisée à la confirmation : « je préciserai plus tard » (relance à J+2, §21.1).
        let nature = match nature {
            Some(n) => n,
            None => nature_par_code(&mut tx, etab, "inconnue")
                .await?
                .ok_or_else(|| invalide("Référentiel des natures incomplet."))?,
        };
        sqlx::query(
            "UPDATE ent_absp_absence SET etat = 'declaree', nature_id = $3, confirmee_par = $4,
                    confirmee_le = now(), rappel_par_user_id = $5, zone_qualification = NULL
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(nature.id)
        .bind(if par_rappel { "rappel" } else { "agent" })
        .bind(par_rappel.then_some(actor))
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        decision(
            &mut tx,
            etab,
            id,
            actor,
            NouvelleDecision {
                type_: "confirmation",
                etat_avant: Some("signalee"),
                etat_apres: Some("declaree"),
                nature_apres: Some(nature.id),
                raison_code: par_rappel.then_some("rappel_trace"),
                ..Default::default()
            },
        )
        .await?;
        journal(
            &mut tx,
            etab,
            id,
            "confirmee",
            Some("etat"),
            Some("signalee"),
            Some("declaree"),
            actor,
        )
        .await?;
        let ev = evenement(
            &mut tx,
            etab,
            EVT_DECLAREE,
            id,
            a.agent_id,
            json!({}),
            actor,
        )
        .await?;
        let v = vue_absence(&mut tx, etab, id, suis_absent || par_rappel).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// ABP-128 : requalifie la nature (valideur, instructeur ; l'agent avant décision, §32.2). Le
    /// traitement est recalculé ; les corrections sont tracées (R-AP17). La zone de qualification est
    /// vidée.
    pub async fn requalifier_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &RequalifierBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let valider = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
            .await?;
        let lire_nature = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, id, true).await?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let suis_absent = mon_agent == Some(a.agent_id) || a.agent_user_id == Some(actor);
        let instructeur = self
            .peut_instruire_abs(&mut tx, etab, actor, role, &a)
            .await?;
        let avant_decision = matches!(a.etat.as_str(), "signalee" | "declaree" | "en_instruction");
        let autorise =
            ((valider || instructeur) && !suis_absent) || (suis_absent && avant_decision);
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        if !etats_permis("modifier").contains(&a.etat.as_str()) {
            return Err(conflit(
                "Cette absence n'est pas dans un état qui permet cette action.",
            ));
        }
        let nouvelle = nature_par_code(&mut tx, etab, body.nature_code.trim())
            .await?
            .ok_or_else(|| invalide("Nature inconnue ou désactivée."))?;
        if (nouvelle.direction_seule && !valider) || nouvelle.code == "autre_service" {
            return Err(EntDbError::Forbidden);
        }
        if nouvelle.employeur != "tous" && nouvelle.employeur != a.employeur {
            return Err(invalide("Cette nature ne s'applique pas à cet employeur."));
        }
        let ancienne = match a.nature_id {
            Some(n) => nature_par_id(&mut tx, etab, n).await?,
            None => None,
        };
        if ancienne.as_ref().map(|n| n.id) == Some(nouvelle.id) {
            let v = vue_absence(&mut tx, etab, id, lire_nature || suis_absent).await?;
            return Ok(v);
        }
        let note = texte_borne(&body.note, 1000, "La note est limitée à 1000 caractères.")?;
        // Traitement recalculé pour une absence déjà décidée ; sinon il se choisira à la validation.
        let traitement = if ETATS_DECIDES.contains(&a.etat.as_str()) {
            Some(choisir_traitement(&nouvelle, &a.employeur, None).map_err(invalide)?)
        } else {
            a.traitement.clone()
        };
        // Le texte libre ne survit pas à une nature qui ne le permet pas (R-AP1).
        let texte = if nouvelle.texte_libre_permis {
            a.texte_libre.clone()
        } else {
            None
        };
        sqlx::query(
            "UPDATE ent_absp_absence SET nature_id = $3, traitement = $4, texte_libre = $5,
                    zone_qualification = NULL
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(nouvelle.id)
        .bind(&traitement)
        .bind(&texte)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        decision(
            &mut tx,
            etab,
            id,
            actor,
            NouvelleDecision {
                type_: "requalification",
                etat_avant: Some(&a.etat),
                etat_apres: Some(&a.etat),
                nature_avant: a.nature_id,
                nature_apres: Some(nouvelle.id),
                traitement_avant: a.traitement.as_deref(),
                traitement_apres: traitement.as_deref(),
                raison_code: body.raison_code.as_deref().filter(|s| !s.is_empty()),
                note: note.as_deref(),
                ..Default::default()
            },
        )
        .await?;
        journal(
            &mut tx,
            etab,
            id,
            "requalifiee",
            Some("nature"),
            ancienne.as_ref().map(|n| n.code.as_str()),
            Some(&nouvelle.code),
            actor,
        )
        .await?;
        // La requalification ne porte QUE l'identifiant : jamais l'ancienne ni la nouvelle nature.
        let ev = evenement(
            &mut tx,
            etab,
            EVT_REQUALIFIEE,
            id,
            a.agent_id,
            json!({}),
            actor,
        )
        .await?;
        let v = vue_absence(&mut tx, etab, id, lire_nature || suis_absent).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// ABP-110 : prolonge (nouvelle dernière journée). L'agent, ou l'instructeur.
    pub async fn prolonger_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &ChangerFinBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.changer_fin(etab, actor, role, id, body, true).await
    }

    /// ABP-111 : écourte (reprise anticipée). L'agent, l'adjoint (`lire_organisation`), `valider`,
    /// `instruire`.
    pub async fn ecourter_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &ChangerFinBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.changer_fin(etab, actor, role, id, body, false).await
    }

    async fn changer_fin(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &ChangerFinBody,
        prolonger: bool,
    ) -> Result<AbsenceDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let valider = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
            .await?;
        let organisation = self
            .a_droit_abs(
                etab,
                actor,
                role,
                Action::PersonnelsAbsencesLireOrganisation,
            )
            .await?;
        let lire_nature = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, id, true).await?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let suis_absent = mon_agent == Some(a.agent_id) || a.agent_user_id == Some(actor);
        let instructeur = self
            .peut_instruire_abs(&mut tx, etab, actor, role, &a)
            .await?;
        let autorise = suis_absent || instructeur || valider || (!prolonger && organisation);
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        if !etats_permis("modifier").contains(&a.etat.as_str()) {
            return Err(conflit(
                "Cette absence n'est pas dans un état qui permet cette action.",
            ));
        }
        let nouvelle_fin = instant_fin(&mut tx, etab, body.fin, body.heure_fin).await?;
        if nouvelle_fin == a.fin {
            let v = vue_absence(&mut tx, etab, id, lire_nature || suis_absent).await?;
            return Ok(v);
        }
        if prolonger && nouvelle_fin <= a.fin {
            return Err(invalide(
                "La nouvelle fin doit être postérieure à la fin actuelle.",
            ));
        }
        if !prolonger && nouvelle_fin >= a.fin {
            return Err(invalide(
                "La reprise anticipée doit précéder la fin prévue.",
            ));
        }
        if nouvelle_fin <= a.debut {
            return Err(invalide(
                "La nouvelle fin doit suivre le début de l'absence.",
            ));
        }
        if prolonger && (nouvelle_fin - a.fin) > Duration::days(366) {
            return Err(invalide("Prolongation trop longue."));
        }
        if prolonger {
            // La dernière plage active est prolongée si elle se termine à la fin de l'absence ; sinon
            // une plage est ajoutée à la suite.
            let dernier: Option<(Uuid, DateTime<Utc>)> = sqlx::query_as(
                "SELECT id, fin FROM ent_absp_plage WHERE etablissement_id = $1 AND absence_id = $2
                    AND retiree_le IS NULL ORDER BY fin DESC LIMIT 1",
            )
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match dernier {
                Some((pid, pfin)) if pfin == a.fin => {
                    sqlx::query("UPDATE ent_absp_plage SET fin = $3 WHERE etablissement_id = $1 AND id = $2")
                        .bind(etab).bind(pid).bind(nouvelle_fin)
                        .execute(&mut *tx).await.map_err(ecriture)?;
                }
                _ => {
                    sqlx::query(
                        "INSERT INTO ent_absp_plage (id, etablissement_id, absence_id, debut, fin) VALUES ($1,$2,$3,$4,$5)",
                    )
                    .bind(Uuid::new_v4()).bind(etab).bind(id).bind(a.fin).bind(nouvelle_fin)
                    .execute(&mut *tx).await.map_err(ecriture)?;
                }
            }
            // Nature à arrêt : la prolongation est un nouvel arrêt (R-AP16).
            let attend_arret = match a.nature_id {
                Some(n) => nature_par_id(&mut tx, etab, n)
                    .await?
                    .is_some_and(|n| n.piece_attendue == "arret"),
                None => false,
            };
            if attend_arret {
                let (du, au): (NaiveDate, NaiveDate) = sqlx::query_as(
                    "SELECT ent_aujourdhui_etab($1, $2), ent_aujourdhui_etab($1, $3::timestamptz - interval '1 microsecond')",
                )
                .bind(etab).bind(a.fin).bind(nouvelle_fin)
                .fetch_one(&mut *tx).await.map_err(db)?;
                sqlx::query(
                    "INSERT INTO ent_absp_arret (id, etablissement_id, absence_id, type, du, au, echeance_le, cree_par_user_id)
                     VALUES ($1,$2,$3,'prolongation',$4,$5,now() + interval '48 hours',$6)",
                )
                .bind(Uuid::new_v4()).bind(etab).bind(id).bind(du).bind(au).bind(actor)
                .execute(&mut *tx).await.map_err(db)?;
            }
        } else {
            // Reprise anticipée : les plages au-delà sont retirées (jamais supprimées), la dernière est tronquée.
            sqlx::query(
                "UPDATE ent_absp_plage SET retiree_le = now()
                  WHERE etablissement_id = $1 AND absence_id = $2 AND retiree_le IS NULL AND debut >= $3",
            )
            .bind(etab).bind(id).bind(nouvelle_fin)
            .execute(&mut *tx).await.map_err(db)?;
            sqlx::query(
                "UPDATE ent_absp_plage SET fin = $3
                  WHERE etablissement_id = $1 AND absence_id = $2 AND retiree_le IS NULL AND fin > $3",
            )
            .bind(etab).bind(id).bind(nouvelle_fin)
            .execute(&mut *tx).await.map_err(db)?;
        }
        sqlx::query(
            "UPDATE ent_absp_absence SET fin = $3, fin_provisoire = CASE WHEN $4 THEN false ELSE fin_provisoire END
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab).bind(id).bind(nouvelle_fin).bind(prolonger)
        .execute(&mut *tx).await.map_err(ecriture)?;
        let type_decision = if prolonger {
            "prolongation"
        } else {
            "ecourtement"
        };
        decision(
            &mut tx,
            etab,
            id,
            actor,
            NouvelleDecision {
                type_: type_decision,
                etat_avant: Some(&a.etat),
                etat_apres: Some(&a.etat),
                fin_avant: Some(a.fin),
                fin_apres: Some(nouvelle_fin),
                ..Default::default()
            },
        )
        .await?;
        journal(
            &mut tx,
            etab,
            id,
            type_decision,
            Some("fin"),
            Some(&a.fin.to_rfc3339()),
            Some(&nouvelle_fin.to_rfc3339()),
            actor,
        )
        .await?;
        let ev = evenement(
            &mut tx,
            etab,
            EVT_MODIFIEE,
            id,
            a.agent_id,
            json!({ "fin_avant": a.fin, "fin_apres": nouvelle_fin }),
            actor,
        )
        .await?;
        let v = vue_absence(&mut tx, etab, id, lire_nature || suis_absent).await?;
        // Impact : la fin change, les séances touchées aussi.
        self.recalculer_impact_tolerant(&mut tx, etab, id, actor).await;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(v)
    }

    /// ABP-131 : l'instructeur note la réception de l'arrêt ; l'échéance de 48 h est affichée, ScolIA ne
    /// sanctionne pas (R-AP16).
    pub async fn noter_arret_recu_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &ArretRecuBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let lire_nature = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, id, true).await?;
        if !self
            .peut_instruire_abs(&mut tx, etab, actor, role, &a)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let arret: Option<Uuid> = match body.arret_id {
            Some(x) => sqlx::query_scalar(
                "SELECT id FROM ent_absp_arret WHERE etablissement_id = $1 AND absence_id = $2 AND id = $3",
            )
            .bind(etab).bind(id).bind(x)
            .fetch_optional(&mut *tx).await.map_err(db)?,
            None => sqlx::query_scalar(
                "SELECT id FROM ent_absp_arret WHERE etablissement_id = $1 AND absence_id = $2
                    AND recu_le IS NULL ORDER BY du DESC LIMIT 1",
            )
            .bind(etab).bind(id)
            .fetch_optional(&mut *tx).await.map_err(db)?,
        };
        let arret = arret
            .ok_or_else(|| conflit("Aucun arrêt en attente de réception pour cette absence."))?;
        sqlx::query(
            "UPDATE ent_absp_arret SET recu_le = now(), recu_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2 AND recu_le IS NULL",
        )
        .bind(etab)
        .bind(arret)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            id,
            "arret_recu",
            Some("arret"),
            None,
            Some("recu"),
            actor,
        )
        .await?;
        let v = vue_absence(&mut tx, etab, id, lire_nature).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// ABP-288, 289 : enregistre la décision de l'employeur externe (rectorat, DSDEN…). L'organisation et
    /// les heures ne dépendent pas de cette décision (R-AP47).
    pub async fn enregistrer_decision_externe_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionExterneBody,
    ) -> Result<AbsenceDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let valider = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
            .await?;
        let lire_nature = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?;
        if !EMPLOYEURS_EXTERNES.contains(&body.employeur.as_str())
            || !DECISIONS_EXTERNES.contains(&body.decision.as_str())
        {
            return Err(invalide("Employeur ou décision inconnus."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, id, true).await?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let suis_absent = mon_agent == Some(a.agent_id) || a.agent_user_id == Some(actor);
        let instructeur = self
            .peut_instruire_abs(&mut tx, etab, actor, role, &a)
            .await?;
        if suis_absent || !(valider || instructeur) {
            return Err(EntDbError::Forbidden);
        }
        let nature = match a.nature_id {
            Some(n) => nature_par_id(&mut tx, etab, n).await?,
            None => None,
        };
        if mode_decision(nature.as_ref(), &a.employeur) != "externe"
            || !etats_permis("decision_externe").contains(&a.etat.as_str())
        {
            return Err(conflit(
                "Cette absence ne relève pas d'une décision de l'employeur.",
            ));
        }
        let nature_apres = if body.decision == "requalifiee" {
            let code = body
                .nature_code
                .as_deref()
                .ok_or_else(|| invalide("Indiquez la nature retenue par l'employeur."))?;
            Some(
                nature_par_code(&mut tx, etab, code)
                    .await?
                    .ok_or_else(|| invalide("Nature inconnue ou désactivée."))?,
            )
        } else {
            None
        };
        sqlx::query(
            "INSERT INTO ent_absp_decision_externe
               (id, etablissement_id, absence_id, employeur, decision, nature_apres_id, le, saisie_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(Uuid::new_v4()).bind(etab).bind(id).bind(&body.employeur).bind(&body.decision)
        .bind(nature_apres.as_ref().map(|n| n.id)).bind(body.le).bind(actor)
        .execute(&mut *tx).await.map_err(db)?;
        if let Some(n) = &nature_apres {
            sqlx::query(
                "UPDATE ent_absp_absence SET nature_id = $3, zone_qualification = NULL, texte_libre = NULL
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab).bind(id).bind(n.id)
            .execute(&mut *tx).await.map_err(ecriture)?;
        }
        sqlx::query("UPDATE ent_absp_absence SET etat = 'decision_employeur' WHERE etablissement_id = $1 AND id = $2")
            .bind(etab).bind(id)
            .execute(&mut *tx).await.map_err(db)?;
        decision(
            &mut tx,
            etab,
            id,
            actor,
            NouvelleDecision {
                type_: "transmission",
                etat_avant: Some(&a.etat),
                etat_apres: Some("decision_employeur"),
                nature_avant: a.nature_id,
                nature_apres: nature_apres.as_ref().map(|n| n.id),
                raison_code: Some(match body.decision.as_str() {
                    "accordee" => "decision_accordee",
                    "refusee" => "decision_refusee",
                    _ => "decision_requalifiee",
                }),
                ..Default::default()
            },
        )
        .await?;
        journal(
            &mut tx,
            etab,
            id,
            "decision_externe",
            Some("decision"),
            None,
            Some(&body.decision),
            actor,
        )
        .await?;
        let v = vue_absence(&mut tx, etab, id, lire_nature).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }
}

// ---------------------------------------------------------------------------------------------
// Lectures
// ---------------------------------------------------------------------------------------------

impl PgEnt {
    /// Natures proposées à l'agent connecté pour son engagement (liste courte, dans ses mots) : actives,
    /// permises pour la classe de son employeur, jamais celles réservées à la direction ni les natures
    /// posées par un autre module.
    pub async fn natures_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<NatureDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let classes: HashSet<&'static str> = match mon_agent {
            Some(a) => sqlx::query_scalar::<_, String>(
                "SELECT employeur_type FROM ent_agent_engagement
                  WHERE etablissement_id = $1 AND agent_id = $2 AND annule_le IS NULL
                    AND du <= ent_aujourdhui_etab($1) AND (au IS NULL OR au >= ent_aujourdhui_etab($1))",
            )
            .bind(etab)
            .bind(a)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
            .iter()
            .map(|t| classe_employeur(t))
            .collect(),
            None => HashSet::new(),
        };
        let classes: HashSet<&'static str> = if classes.is_empty() {
            ["etat"].into_iter().collect()
        } else {
            classes
        };
        let rows: Vec<(String, String, String, bool, String, String)> = sqlx::query_as(
            "SELECT n.code, n.libelle_agent, n.famille, n.texte_libre_permis, n.piece_attendue, n.employeur
               FROM ent_absp_nature n
               LEFT JOIN ent_absp_nature_etab ne ON ne.etablissement_id = $1 AND ne.nature_id = n.id
              WHERE (n.etablissement_id = $1 OR n.etablissement_id IS NULL)
                AND COALESCE(ne.actif, n.actif) AND NOT n.direction_seule AND n.code <> 'autre_service'
              ORDER BY (n.code = 'inconnue'), n.famille, n.libelle_agent",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .filter(|(.., e)| e == "tous" || classes.iter().any(|c| c == e))
            .map(|(code, libelle, famille, libre, piece, _)| NatureDto {
                code,
                libelle,
                famille,
                texte_libre_permis: libre,
                piece_attendue: piece,
            })
            .collect())
    }

    /// Mes absences (l'agent lit la sienne, nature comprise).
    pub async fn mes_absences_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<AbsenceDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let Some(mon_agent) = agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Ok(Vec::new());
        };
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_absp_absence WHERE etablissement_id = $1 AND agent_id = $2 AND cause = 'rh'
              ORDER BY debut DESC LIMIT 200",
        )
        .bind(etab)
        .bind(mon_agent)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for id in ids {
            out.push(vue_absence(&mut tx, etab, id, true).await?);
        }
        Ok(out)
    }

    /// Une absence, vue du cercle RH : l'agent lui-même, `lire_nature`, ou le circuit (valideur,
    /// instructeur du même employeur). Tout autre profil reçoit « introuvable » (pas d'oracle).
    pub async fn absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<AbsenceDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        let lire_nature = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = charger(&mut tx, etab, id, false).await?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let suis_absent = mon_agent == Some(a.agent_id) || a.agent_user_id == Some(actor);
        let au_circuit = match mon_agent {
            Some(m) => {
                let (b,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_absp_circuit WHERE etablissement_id = $1 AND absence_id = $2
                                     AND agent_id = $3)",
                )
                .bind(etab).bind(id).bind(m)
                .fetch_one(&mut *tx).await.map_err(db)?;
                b
            }
            None => false,
        };
        let instructeur = self
            .peut_instruire_abs(&mut tx, etab, actor, role, &a)
            .await?;
        // `lire_nature` s'applique à l'établissement ; le secrétariat voit l'État, la comptabilité l'OGEC.
        let r = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let lecteur_nature =
            lire_nature && (matches!(r, Role::Direction) || instruit(r, &a.employeur));
        if !(suis_absent || lecteur_nature || au_circuit || instructeur) {
            return Err(introuvable());
        }
        vue_absence(
            &mut tx,
            etab,
            id,
            suis_absent || lecteur_nature || au_circuit || instructeur,
        )
        .await
    }

    /// ABP-134 : historique complet d'une absence (journal en ajout seul), cercle RH seulement.
    pub async fn historique_absence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Vec<JournalAbsenceDto>, EntDbError> {
        // Même contrôle d'accès que la fiche : « introuvable » pour tout autre.
        self.absence_for_actor(etab, actor, role, id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(DateTime<Utc>, String, Uuid, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT le, action, auteur_user_id, colonne, valeur_avant, valeur_apres FROM ent_absp_journal
              WHERE etablissement_id = $1 AND objet = 'absence' AND objet_id = $2 ORDER BY le, id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(
                |(le, action, auteur_user_id, colonne, valeur_avant, valeur_apres)| {
                    JournalAbsenceDto {
                        le,
                        action,
                        auteur_user_id,
                        colonne,
                        valeur_avant,
                        valeur_apres,
                    }
                },
            )
            .collect())
    }

    /// Vue **organisation** (ABP-150 à 157) : qui est absent, quand, **sans nature** (R-AP3). Droit
    /// `lire_organisation` ou `lire_nature`. Les absences annulées n'y figurent pas ; une absence refusée
    /// y figure (l'agent n'était pas là, R-AP15) ; une absence non confirmée porte `confirmee = false`.
    pub async fn absences_organisation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<AbsenceOrganisationDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !(self
            .a_droit_abs(
                etab,
                actor,
                role,
                Action::PersonnelsAbsencesLireOrganisation,
            )
            .await?
            || self
                .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
                .await?)
        {
            return Err(EntDbError::Forbidden);
        }
        if au < du || (au - du).num_days() > 92 {
            return Err(invalide("Période invalide (92 jours au plus)."));
        }
        // Confidentialité renforcée (ABP-164) : une absence ainsi marquée n'est visible que de la direction et
        // des valideurs ; la vie scolaire ne voit plus l'agent (elle voit « séance à couvrir » à l'impact, A3).
        let voit_renforcee = matches!(Role::parse(role), Some(Role::Direction))
            || self
                .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
                .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Colonnes explicites : aucune colonne de nature n'est lue (R-AP3).
        let rows: Vec<(Uuid, Uuid, Option<String>, DateTime<Utc>, DateTime<Utc>, bool)> = sqlx::query_as(
            "SELECT a.id, a.agent_id, btrim(coalesce(p.prenom, '') || ' ' || p.nom), a.debut, a.fin,
                    a.etat <> 'signalee' OR a.confirmee_le IS NOT NULL
               FROM ent_absp_absence a
               JOIN ent_agent ag ON ag.etablissement_id = a.etablissement_id AND ag.id = a.agent_id
               JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
              WHERE a.etablissement_id = $1 AND a.cause = 'rh' AND a.etat = ANY($4)
                AND (NOT a.confidentialite_renforcee OR $5)
                AND (a.debut AT TIME ZONE COALESCE((SELECT fuseau FROM ent_etablissement_parametre
                        WHERE etablissement_id = $1), 'Europe/Paris'))::date <= $3
                AND ((a.fin - interval '1 microsecond') AT TIME ZONE COALESCE((SELECT fuseau FROM ent_etablissement_parametre
                        WHERE etablissement_id = $1), 'Europe/Paris'))::date >= $2
              ORDER BY a.debut, a.id",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .bind(ETATS_COMPTANT.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .bind(voit_renforcee)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, agent_id, agent_label, debut, fin, confirmee)| AbsenceOrganisationDto {
                    id,
                    agent_id,
                    agent_label,
                    debut,
                    fin,
                    confirmee,
                },
            )
            .collect())
    }

    /// File « à valider » du valideur : absences déclarées (ou en instruction) des autres agents. Droit
    /// `valider`. Une absence dont le valideur est l'absent n'y figure jamais (R-AP10).
    pub async fn file_a_valider_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<AbsenceDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesValider)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mon_agent = agent_du_user_tx(&mut tx, etab, actor).await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_absp_absence
              WHERE etablissement_id = $1 AND cause = 'rh' AND etat IN ('declaree', 'en_instruction')
                AND ($2::uuid IS NULL OR agent_id <> $2)
              ORDER BY debut, id LIMIT 500",
        )
        .bind(etab)
        .bind(mon_agent)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let lire_nature = self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesLireNature)
            .await?;
        let mut out = Vec::new();
        for id in ids {
            out.push(vue_absence(&mut tx, etab, id, lire_nature).await?);
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------------------------
    // Paramétrage : règles de circuit (ABP-259, droit `parametrer`)
    // -----------------------------------------------------------------------------------------

    pub async fn regles_circuit_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<RegleCircuitDto>, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesParametrer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(Uuid, String, Option<String>, String, Uuid, i16)> = sqlx::query_as(
            "SELECT id, employeur, famille, role, agent_id, rang FROM ent_absp_circuit_regle
              WHERE etablissement_id = $1 AND actif ORDER BY employeur, role, rang, cree_le",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, employeur, famille, role, agent_id, rang)| RegleCircuitDto {
                    id,
                    employeur,
                    famille,
                    role,
                    agent_id,
                    rang,
                },
            )
            .collect())
    }

    pub async fn creer_regle_circuit_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &CreerRegleCircuitBody,
    ) -> Result<RegleCircuitDto, EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesParametrer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if !["etat", "ogec", "tous"].contains(&body.employeur.as_str())
            || !["valideur", "instructeur", "informe"].contains(&body.role.as_str())
        {
            return Err(invalide("Employeur ou rôle de circuit inconnu."));
        }
        let rang = body.rang.unwrap_or(1);
        if !(1..=5).contains(&rang) {
            return Err(invalide("Rang de 1 à 5."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2
                             AND fusionne_dans_agent_id IS NULL)",
        )
        .bind(etab)
        .bind(body.agent_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_absp_circuit_regle
               (id, etablissement_id, employeur, famille, role, agent_id, rang, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(&body.employeur)
        .bind(&body.famille)
        .bind(&body.role)
        .bind(body.agent_id)
        .bind(rang)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        sqlx::query(
            "INSERT INTO ent_absp_journal
               (id, etablissement_id, objet, objet_id, action, auteur_user_id, colonne, valeur_apres)
             VALUES ($1,$2,'circuit_regle',$3,'regle_creee',$4,'role',$5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(actor)
        .bind(&body.role)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(RegleCircuitDto {
            id,
            employeur: body.employeur.clone(),
            famille: body.famille.clone(),
            role: body.role.clone(),
            agent_id: body.agent_id,
            rang,
        })
    }

    /// Retire une règle de circuit (désactivée, jamais supprimée). Les absences déjà déclarées gardent
    /// leur circuit figé.
    pub async fn retirer_regle_circuit_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_personnel_abs(role).await?;
        if !self
            .a_droit_abs(etab, actor, role, Action::PersonnelsAbsencesParametrer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE ent_absp_circuit_regle SET actif = false
              WHERE etablissement_id = $1 AND id = $2 AND actif",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        sqlx::query(
            "INSERT INTO ent_absp_journal
               (id, etablissement_id, objet, objet_id, action, auteur_user_id)
             VALUES ($1,$2,'circuit_regle',$3,'regle_retiree',$4)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests_purs {
    use super::*;

    fn nature(code: &str) -> NatureLigne {
        NatureLigne {
            id: Uuid::new_v4(),
            code: code.into(),
            libelle_agent: code.into(),
            famille: "autre".into(),
            employeur: "tous".into(),
            sensible: false,
            refus_possible: true,
            piece_attendue: "aucune".into(),
            delai_piece_heures: None,
            traitement_etat: Some("sans_incidence".into()),
            traitement_ogec: Some("sans_incidence".into()),
            traitement_impose: false,
            decision_etat: "interne".into(),
            decision_ogec: "interne".into(),
            texte_libre_permis: true,
            direction_seule: false,
            duree_max_heures: None,
        }
    }

    #[test]
    fn mots_exclus_sans_accent_ni_casse() {
        assert_eq!(mot_exclu("Réunion au rectorat"), None);
        assert_eq!(mot_exclu("Formation REP+ à Lyon"), None);
        assert!(mot_exclu("Hospitalisé depuis lundi").is_some());
        assert!(mot_exclu("rendez-vous MÉDECIN").is_some());
        assert!(mot_exclu("grève du 12").is_some());
        assert!(mot_exclu("burn-out").is_some());
        assert!(mot_exclu("Arrêt de travail").is_some());
    }

    #[test]
    fn classe_employeur_etat_ou_ogec() {
        assert_eq!(classe_employeur("etat"), "etat");
        assert_eq!(classe_employeur("organisme_gestion"), "ogec");
        assert_eq!(classe_employeur("association"), "ogec");
        assert_eq!(classe_employeur("inconnu"), "etat");
    }

    #[test]
    fn traitement_impose_par_la_nature_non_discretionnaire() {
        let mut n = nature("maladie");
        n.traitement_impose = true;
        n.traitement_etat = Some("transmission_rectorat".into());
        n.traitement_ogec = Some("maintien_salaire".into());
        // R-AP12 : le valideur ne choisit pas, même s'il demande autre chose.
        assert_eq!(
            choisir_traitement(&n, "etat", Some("sans_incidence")).unwrap(),
            "transmission_rectorat"
        );
        assert_eq!(
            choisir_traitement(&n, "ogec", None).unwrap(),
            "maintien_salaire"
        );
    }

    #[test]
    fn traitement_choisi_parmi_les_permis_de_l_employeur() {
        let n = nature("convenance");
        assert_eq!(
            choisir_traitement(&n, "etat", Some("rattrapage_interne")).unwrap(),
            "rattrapage_interne"
        );
        assert!(
            choisir_traitement(&n, "etat", Some("retenue_salaire")).is_err(),
            "retenue : OGEC seulement"
        );
        assert_eq!(
            choisir_traitement(&n, "ogec", Some("retenue_salaire")).unwrap(),
            "retenue_salaire"
        );
        assert!(choisir_traitement(&n, "ogec", Some("transmission_rectorat")).is_err());
        assert_eq!(
            choisir_traitement(&n, "etat", None).unwrap(),
            "sans_incidence"
        );
    }

    #[test]
    fn decision_externe_selon_l_employeur() {
        let mut n = nature("maladie");
        n.decision_etat = "externe".into();
        assert_eq!(mode_decision(Some(&n), "etat"), "externe");
        assert_eq!(mode_decision(Some(&n), "ogec"), "interne");
        assert_eq!(mode_decision(None, "etat"), "interne");
    }

    #[test]
    fn table_des_etats_permis() {
        assert!(etats_permis("valider").contains(&"declaree"));
        assert!(
            !etats_permis("valider").contains(&"signalee"),
            "il faut d'abord confirmer"
        );
        assert!(!etats_permis("valider").contains(&"annulee"));
        assert!(etats_permis("confirmer") == ["signalee"]);
        assert!(!etats_permis("rouvrir").contains(&"annulee"));
        assert!(!etats_permis("modifier").contains(&"refusee"));
        // Ce qui compte pour l'organisation : jamais une absence annulée ; une refusée compte (R-AP15).
        assert!(!ETATS_COMPTANT.contains(&"annulee"));
        assert!(ETATS_COMPTANT.contains(&"refusee"));
        // Ce qui libère une séance : décidé seulement, jamais une simple signalée ou refusée.
        assert!(ETATS_DECIDES.iter().all(|e| ETATS_COMPTANT.contains(e)));
        assert!(!ETATS_DECIDES.contains(&"refusee") && !ETATS_DECIDES.contains(&"signalee"));
    }

    #[test]
    fn texte_libre_borne_et_controle() {
        assert!(texte_borne(&Some("réunion de bassin".into()), 200, "x").is_ok());
        assert!(texte_borne(&Some("a".repeat(201)), 200, "trop long").is_err());
        assert!(texte_borne(&Some("rendez-vous chez le médecin".into()), 200, "x").is_err());
        assert_eq!(texte_borne(&Some("   ".into()), 200, "x").unwrap(), None);
    }
}
