//! Absences et sorties de l'internat (socle 41, lot I3) : annonces d'absence des familles, décisions du CPE,
//! sorties typées, correspondants, départ du vendredi, contrôles de départ et de retour, remise, autorisations
//! annuelles (socle 18), élèves majeurs, désaccords entre titulaires.
//!
//! Règles appliquées ici :
//! * **R-IT19** une absence est annoncée par un titulaire (autorité parentale, lien actif) ou par l'élève
//!   majeur, jamais par un tiers ; avant l'heure limite elle est en vigueur aussitôt, après elle devient une
//!   demande au CPE ; une nuit passée ne s'annonce pas, elle se **régularise** (CIT-37) ;
//! * **R-IT20** un désaccord connu des titulaires (socle 18) bloque l'annonce et la renvoie au CPE ; une
//!   opposition d'un titulaire à une annonce déjà faite la bloque aussi : **le plus restrictif l'emporte** tant
//!   que la direction n'a pas tranché, avec trace (qui, quand, quoi) ; une famille ne voit jamais **qui** s'oppose ;
//! * **R-IT22** l'élève majeur décide de ses absences et de ses sorties dans le cadre du règlement ; ses
//!   parents sont informés selon le réglage de l'établissement (`majeur_info_parents`) ;
//! * **R-IT23** un départ accompagné se fait à une personne **autorisée aujourd'hui** (socle 4) ; une interdiction
//!   de remise bloque toujours, et la tentative est **tracée** ;
//! * **R-IT3** rien ne se supprime : états, faits datés en ajout seul ;
//! * **R-IT6** aucune donnée de santé (mots de santé refusés dans les textes courts) ;
//! * **R-IT39** aucune action par lien sans compte : toutes les routes exigent une session.
//!
//! Chaque décision se reflète dans l'attendu de la nuit (`ent_internat_attendu_calcule`, migration 2826) :
//! une absence en vigueur ou une sortie qui couvre la nuit retire l'élève de l'attendu avec son motif ; une
//! sortie plus courte ne le retire jamais (il reste attendu et se pointe « en activité »). Un blocage,
//! un refus, une annulation ou une suspension le rendent aussitôt à l'attendu : aucune fausse alerte, aucun oubli.
//!
//! Chaque fonction `*_a` reçoit l'instant (`maintenant`) : aucun test ne dépend de l'heure réelle.

use crate::accords::{DemandeVerification, Sujet};
use crate::finances::publier;
use crate::internat_hebergement::parametres_tx;
use crate::internat_nuit::{conflit, db, evoque_la_sante, nuit_a_tx, refus};
use crate::outbox::Causalite;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc, Weekday};
use domain_eleve::DomainError;
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind};
use ent_types::internat_sorties::*;
use serde_json::json;
use sqlx::{PgConnection, Row};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

mod actes;
mod tiers;
use actes::{
    actes_qui_tombent_tx, appliquer_absence_tx, appliquer_sortie_tx, comptes_vises_tx, deciders_annuel_tx,
    fin_savepoint_revue, ouvrir_savepoint_revue, suspendre_correspondants_auteur_tx, Objet, RevueBilan,
};

/// Nombre de minutes de veille consécutives en échec avant l'alerte opérationnelle.
const ALERTE_VEILLE_APRES: u32 = 3;

/// Passe 4 (R7) : suivi des échecs répétés de la veille, par établissement (en mémoire du processus ; le compteur repart à
/// zéro au redémarrage et dès qu'une veille réussit sans échec d'objet). À partir de `ALERTE_VEILLE_APRES` minutes
/// consécutives, une alerte opérationnelle est journalisée (au niveau erreur, champ `alerte`), puis rappelée toutes les 30
/// minutes. **Aucune donnée nominative ni d'établissement** : seul le nombre de minutes et d'établissements concernés.
fn suivre_echecs_veille(etab: Uuid, echec: bool) {
    static ECHECS: std::sync::Mutex<Option<HashMap<Uuid, u32>>> = std::sync::Mutex::new(None);
    let Ok(mut garde) = ECHECS.lock() else { return };
    let table = garde.get_or_insert_with(HashMap::new);
    if !echec {
        table.remove(&etab);
        return;
    }
    let n = {
        let c = table.entry(etab).or_insert(0);
        *c += 1;
        *c
    };
    if n == ALERTE_VEILLE_APRES || (n > ALERTE_VEILLE_APRES && n % 30 == 0) {
        tracing::error!(
            alerte = "internat_veille_sorties_en_echec",
            minutes_consecutives = n,
            etablissements_en_echec = table.len(),
            "veille des sorties d'internat en échec répété pour un établissement (revue des sorties, absences ou correspondants)"
        );
    }
}

/// Une absence annoncée couvre 60 nuits au plus (CHECK de la base), jamais plus d'un an à l'avance.
const HORIZON_ANNONCE_JOURS: i64 = 366;
/// Une nuit passée se régularise dans les 45 jours (au-delà, c'est une décision de la direction, hors lot).
const REGULARISATION_JOURS_MAX: i64 = 45;
/// Une sortie s'enregistre un an à l'avance au plus.
const HORIZON_SORTIE_JOURS: i64 = 366;
/// Le contrôle de départ se fait au plus tôt trois heures avant l'heure prévue.
const DEPART_AVANCE_MAX_MINUTES: i64 = 180;
/// Un retard annoncé reporte le retour de 48 heures au plus.
const RETARD_MAX_HEURES: i64 = 48;
/// Durée minimale d'une sortie.
const SORTIE_MIN_MINUTES: i64 = 15;
/// Un geste de contrôle (départ, retour) ne se date pas de plus de 30 minutes en arrière : on rattrape un geste
/// noté à la main sur un cahier, pas une journée entière (aucune antidate qui réécrirait une nuit passée).
const GESTE_ANTIDATE_MAX_MINUTES: i64 = 30;
/// Un retard annoncé ne dépasse jamais la durée maximale du type de sortie de plus de 6 heures (marge pour un train
/// supprimé ou une grève) : au-delà, c'est une autre sortie.
const RETARD_MARGE_HEURES: i64 = 6;

const MOTIFS_RETARD: [&str; 2] = ["train", "autre"];
const OBJET_ACCORD_SORTIE: &str = "sortie_internat";

/// Types de sortie par défaut (§10.2), créés à la première lecture d'un établissement (S-4105) :
/// (code, libellé, autorisation, décideur, seul permis, l'élève peut demander, correspondant requis,
/// durée maximale en heures, usage du catalogue du socle 18).
#[allow(clippy::type_complexity)]
const TYPES_DEFAUT: [(&str, &str, &str, &str, bool, bool, bool, i16, &str); 6] = [
    ("mercredi", "Sortie du mercredi après-midi", "annuelle", "titulaires", true, true, false, 8, "sortie_mercredi"),
    ("soiree", "Sortie de soirée", "annuelle_ou_ponctuelle", "titulaires_et_cpe", true, true, false, 6, "sortie_soiree"),
    (
        "nuit_correspondant",
        "Nuit chez le correspondant",
        "ponctuelle",
        "titulaires_et_cpe",
        false,
        false,
        true,
        30,
        "nuit_correspondant",
    ),
    ("week_end", "Week-end", "aucune", "titulaires", true, false, false, 100, "week_end_seul"),
    ("retour_tardif", "Retour tardif", "ponctuelle", "titulaires_et_cpe", true, true, false, 12, "retour_tardif"),
    ("exceptionnelle", "Sortie exceptionnelle", "ponctuelle", "titulaires_et_cpe", false, true, false, 72, "exceptionnelle"),
];

const AUTORISATIONS: [&str; 4] = ["annuelle", "annuelle_ou_ponctuelle", "ponctuelle", "aucune"];
const DECIDEURS: [&str; 3] = ["titulaires", "cpe", "titulaires_et_cpe"];

// ───────────────────────── Qualité de l'acteur ─────────────────────────

/// Qui agit pour l'élève : un titulaire, l'élève majeur, l'élève mineur, ou un membre du personnel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Qualite {
    Titulaire,
    EleveMajeur,
    Eleve,
    Personnel,
}

impl Qualite {
    fn code(self) -> &'static str {
        match self {
            Qualite::Titulaire => "titulaire",
            Qualite::EleveMajeur => "eleve_majeur",
            Qualite::Eleve => "eleve",
            Qualite::Personnel => "personnel",
        }
    }
}

/// Famille de l'élève vue du compte : sa qualité et, pour un titulaire, le droit du lien d'autoriser les sorties.
#[derive(Clone, Copy, Debug)]
struct Famille {
    qualite: Qualite,
    peut_autoriser: bool,
}

/// Nature d'une erreur pour les JOURNAUX (`tracing`) : jamais le texte d'une erreur sqlx, qui peut citer des valeurs de
/// lignes (noms, identifiants, contenu d'une contrainte violée). Pour la base : la catégorie et le code SQLSTATE seulement.
pub(crate) fn nature_erreur(e: &EntDbError) -> String {
    match e {
        EntDbError::Sqlx(sqlx::Error::Database(d)) => format!("base de données (SQLSTATE {})", d.code().as_deref().unwrap_or("?")),
        EntDbError::Sqlx(sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed) => "base de données (pool indisponible)".into(),
        EntDbError::Sqlx(sqlx::Error::Io(_)) => "base de données (connexion)".into(),
        EntDbError::Sqlx(_) => "base de données".into(),
        EntDbError::Domain(DomainError::NotFound) => "introuvable".into(),
        EntDbError::Forbidden => "refusé".into(),
        _ => "erreur de service".into(),
    }
}

/// Texte court facultatif (300 caractères), sans donnée de santé (R-IT6).
fn texte_court(t: Option<&str>) -> Result<Option<String>, EntDbError> {
    let Some(t) = t.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if t.chars().count() > 300 {
        return Err(refus("texte trop long (300 caractères)"));
    }
    if evoque_la_sante(t) {
        return Err(refus("aucune donnée de santé dans l'internat : le texte évoque la santé"));
    }
    Ok(Some(t.to_string()))
}

/// Cible d'un fait du journal.
#[derive(Clone, Copy)]
enum Cible {
    Absence(Uuid),
    Sortie(Uuid),
    Correspondant(Uuid),
    Tiers(Uuid),
}

#[allow(clippy::too_many_arguments)]
async fn fait_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    cible: Cible,
    acte: &str,
    qualite: &str,
    par: Option<Uuid>,
    a: DateTime<Utc>,
    motif_code: Option<&str>,
    motif_texte: Option<&str>,
    personne: Option<Uuid>,
    jusqu_a: Option<DateTime<Utc>>,
) -> Result<(), EntDbError> {
    let (absence, sortie, correspondant, tiers) = match cible {
        Cible::Absence(i) => (Some(i), None, None, None),
        Cible::Sortie(i) => (None, Some(i), None, None),
        Cible::Correspondant(i) => (None, None, Some(i), None),
        Cible::Tiers(i) => (None, None, None, Some(i)),
    };
    sqlx::query(
        "INSERT INTO ent_internat_i3_fait \
         (id, etablissement_id, absence_id, sortie_id, correspondant_id, acte, qualite, par_user_id, a, \
          motif_code, motif_texte, personne_id, jusqu_a, tiers_id) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(absence)
    .bind(sortie)
    .bind(correspondant)
    .bind(acte)
    .bind(qualite)
    .bind(par)
    .bind(a)
    .bind(motif_code)
    .bind(motif_texte)
    .bind(personne)
    .bind(jusqu_a)
    .bind(tiers)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

// ───────────────────────── Lectures communes ─────────────────────────

/// Jour local de l'établissement d'un instant.
async fn jour_local_tx(conn: &mut PgConnection, etab: Uuid, instant: DateTime<Utc>) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ($1::timestamptz AT TIME ZONE ent_internat_fuseau($2))::date")
        .bind(instant)
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

/// La nuit en cours, **si elle a déjà commencé** à cet instant (instant postérieur à l'ancre de la nuit, 22 h
/// murales) ; `None` avant. Sert à ne faire porter un blocage que sur les nuits suivantes (Moyen 5).
async fn nuit_commencee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    instant: DateTime<Utc>,
) -> Result<Option<NaiveDate>, EntDbError> {
    let courante = nuit_a_tx(conn, etab, instant).await?;
    let commencee: bool = sqlx::query_scalar("SELECT $1::timestamptz >= ent_internat_ancre_nuit($2, $3::date)")
        .bind(instant)
        .bind(etab)
        .bind(courante)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    Ok(commencee.then_some(courante))
}

/// Instant correspondant à une date et une heure murales de l'établissement.
async fn instant_mural_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    heure: NaiveTime,
) -> Result<DateTime<Utc>, EntDbError> {
    sqlx::query_scalar("SELECT (($1::date + $2::time) AT TIME ZONE ent_internat_fuseau($3))")
        .bind(jour)
        .bind(heure)
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

/// L'élève décide-t-il lui-même au jour donné ? Majorité calculée depuis la date de naissance (socle 2 : date
/// « jour » non contestée) **et** capacité juridique (migration 285) : un mineur **émancipé** est traité en
/// majeur ; un majeur **sous tutelle** ne l'est pas (le tuteur, titulaire du lien, décide à sa place).
async fn eleve_majeur_tx(conn: &mut PgConnection, etab: Uuid, eleve: Uuid, jour: NaiveDate) -> Result<bool, EntDbError> {
    // Une seule source de vérité, partagée avec l'attendu de la nuit (migration 2826).
    sqlx::query_scalar("SELECT ent_internat_eleve_majeur($1, $2, $3)")
        .bind(etab)
        .bind(eleve)
        .bind(jour)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

/// Désaccord connu entre titulaires (socle 18) sur les sorties, non levé (R-IT20).
async fn desaccord_connu_tx(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_acc_desaccord \
          WHERE etablissement_id = $1 AND sujet_eleve_id = $2 AND leve_le IS NULL AND objet IN ('sortie', 'general'))",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Un fait d'opposition existe-t-il sur l'absence ou la sortie (sans dire de qui) ?
async fn opposition_tx(conn: &mut PgConnection, etab: Uuid, cible: Cible) -> Result<bool, EntDbError> {
    let (colonne, id) = match cible {
        Cible::Absence(i) => ("absence_id", i),
        Cible::Sortie(i) => ("sortie_id", i),
        Cible::Correspondant(i) => ("correspondant_id", i),
        Cible::Tiers(i) => ("tiers_id", i),
    };
    sqlx::query_scalar(&format!(
        "SELECT EXISTS (SELECT 1 FROM ent_internat_i3_fait WHERE etablissement_id = $1 AND {colonne} = $2 \
          AND acte = 'opposition')"
    ))
    .bind(etab)
    .bind(id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Verdict d'une autorisation annuelle (socle 18) pour un usage : jamais « autorise » par défaut. **Passe 3 (point 2)** :
/// un verdict « autorisé » qui ne repose que sur des décideurs désormais visés par une interdiction de remise, de visite ou
/// de contact avec l'élève est refusé (« decideur_interdit ») : la vue des liens communicants du socle 18 n'exclut que
/// l'interdiction de communication.
async fn verdict_annuel(
    ent: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    usage: &str,
    jour: NaiveDate,
) -> String {
    verdicts_annuels(ent, conn, etab, usage, jour, &[eleve]).await.remove(&eleve).unwrap_or_else(|| "inconnu".to_string())
}

/// Verdicts annuels de plusieurs élèves pour un usage et un jour : **une** vérification du socle 18, une lecture des
/// décideurs et une lecture des interdictions pour tous les élèves (revue à la minute : une requête par type de sortie).
async fn verdicts_annuels(
    ent: &PgEnt,
    conn: &mut PgConnection,
    etab: Uuid,
    usage: &str,
    jour: NaiveDate,
    eleves: &[Uuid],
) -> HashMap<Uuid, String> {
    let mut sortie: HashMap<Uuid, String> = HashMap::new();
    if eleves.is_empty() {
        return sortie;
    }
    let demande = DemandeVerification {
        objet_code: OBJET_ACCORD_SORTIE.to_string(),
        usage_code: usage.to_string(),
        sujets: eleves.iter().map(|e| Sujet::Eleve(*e)).collect(),
        jour: Some(jour),
    };
    let verdicts = ent.verifier_interne(&mut *conn, etab, &demande).await.unwrap_or_default();
    for v in verdicts {
        if let Sujet::Eleve(e) = v.sujet {
            sortie.insert(e, v.verdict.code().to_string());
        }
    }
    let autorises: Vec<Uuid> = sortie.iter().filter(|(_, v)| v.as_str() == "autorise").map(|(e, _)| *e).collect();
    if autorises.is_empty() {
        return sortie;
    }
    let Ok(deciders) = deciders_annuel_tx(conn, etab, usage, &autorises).await else {
        // Pessimiste : décideurs illisibles, aucune autorisation.
        for e in autorises {
            sortie.insert(e, "inconnu".to_string());
        }
        return sortie;
    };
    let couples: Vec<(Uuid, Uuid)> = deciders.iter().flat_map(|(e, ds)| ds.iter().map(|d| (*e, *d))).collect();
    let Ok(vises) = comptes_vises_tx(conn, etab, &couples, jour).await else {
        for e in autorises {
            sortie.insert(e, "inconnu".to_string());
        }
        return sortie;
    };
    for (eleve, ds) in deciders {
        if !ds.is_empty() && ds.iter().all(|d| vises.contains_key(&(eleve, *d))) {
            sortie.insert(eleve, "decideur_interdit".to_string());
        }
    }
    sortie
}

/// Personne et remise (R-IT23).
#[derive(Debug, PartialEq, Eq)]
enum Remise {
    /// Autorisée aujourd'hui ; relation avec l'élève (libellé du lien ou du contact).
    Autorisee(String),
    /// Visée par une interdiction de remise, de visite ou de contact avec l'élève.
    Interdite,
    /// Une interdiction sans personne identifiée est en vigueur : on ne peut pas vérifier qui est à la porte.
    InterditeSansCible,
    NonAutorisee,
}

/// Une personne et **toute sa chaîne de fusions**, dans les deux sens : les fiches fusionnées dans elle, et la fiche
/// dans laquelle elle est elle-même fusionnée (et ainsi de suite). Une interdiction posée sur n'importe quelle fiche
/// de la chaîne vise la même personne réelle. `$1` = établissement. (La même règle existe en SQL :
/// `ent_internat_personne_chaine`, migration 2826.)
fn chaine_complete_sql(p: &str) -> String {
    format!(
        "(WITH RECURSIVE chaine(id) AS ( \
              SELECT {p}::uuid \
              UNION \
              SELECT CASE WHEN s.id = c.id THEN s.fusionnee_dans ELSE s.id END FROM ent_personne s \
                JOIN chaine c ON s.etablissement_id = $1 \
                             AND ((s.id = c.id AND s.fusionnee_dans IS NOT NULL) OR s.fusionnee_dans = c.id)) \
          SELECT id FROM chaine)"
    )
}

/// La personne est-elle visée (sur toute sa chaîne de fusions) par une interdiction de remise, de visite ou de
/// contact avec l'élève, **non levée et non terminée** ? La date de début `du` est ignorée (décision pessimiste, comme
/// E3 d'I2) : une interdiction posée en urgence joue tout de suite, même datée du lendemain.
pub(crate) async fn personne_interdite_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    Ok(interdiction_personne_depuis_tx(conn, etab, eleve, personne, jour).await?.is_some())
}

/// Date de pose (la plus récente) des interdictions qui visent la personne (chaîne de fusions comprise), de remise, de
/// visite ou de contact, non levées et non terminées ; `None` si elle n'est pas visée (`ent_internat_personne_visee`).
async fn interdiction_personne_depuis_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<Option<DateTime<Utc>>, EntDbError> {
    sqlx::query_scalar("SELECT ent_internat_personne_visee($1, $2, $3, $4)")
        .bind(etab)
        .bind(eleve)
        .bind(personne)
        .bind(jour)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

/// M-A : date de pose de l'interdiction **sans personne identifiée** (non nominative) la plus récente, non levée et non
/// terminée ; `seulement_remise` limite aux interdictions d'objet `remise`.
async fn interdiction_sans_cible_depuis_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    jour: NaiveDate,
    seulement_remise: bool,
) -> Result<Option<DateTime<Utc>>, EntDbError> {
    sqlx::query_scalar("SELECT ent_internat_interdiction_sans_cible($1, $2, $3, $4)")
        .bind(etab)
        .bind(eleve)
        .bind(jour)
        .bind(seulement_remise)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

/// Une décision de la direction (déblocage) postérieure à `depuis` figure-t-elle dans l'historique de la cible ?
/// Elle vaut décision sur l'interdiction posée à cette date : la revue ne rebloque pas ce qu'elle a débloqué.
async fn decision_apres_tx(conn: &mut PgConnection, etab: Uuid, cible: Cible, depuis: DateTime<Utc>) -> Result<bool, EntDbError> {
    let (colonne, id, filtre) = match cible {
        Cible::Absence(i) => ("absence_id", i, "acte = 'tranchee_levee'"),
        Cible::Sortie(i) => ("sortie_id", i, "acte = 'tranchee_levee'"),
        // Un correspondant suspendu ne reprend que par le CPE.
        Cible::Correspondant(i) => ("correspondant_id", i, "acte = 'reprise' AND qualite = 'cpe'"),
        Cible::Tiers(i) => ("tiers_id", i, "acte = 'tiers_levee'"),
    };
    sqlx::query_scalar(&format!(
        "SELECT EXISTS (SELECT 1 FROM ent_internat_i3_fait WHERE etablissement_id = $1 AND {colonne} = $2 \
          AND {filtre} AND a >= $3)"
    ))
    .bind(etab)
    .bind(id)
    .bind(depuis)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// M-A : tant qu'une interdiction de remise **sans personne identifiée** est en vigueur, un mineur ne part pas seul
/// (sortie ou absence) sans décision de la direction postérieure à sa pose. Vrai si le départ seul est à arrêter.
async fn seul_arrete_par_interdiction_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    cible: Cible,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    if eleve_majeur_tx(conn, etab, eleve, jour).await? {
        return Ok(false);
    }
    let Some(depuis) = interdiction_sans_cible_depuis_tx(conn, etab, eleve, jour, true).await? else {
        return Ok(false);
    };
    Ok(!decision_apres_tx(conn, etab, cible, depuis).await?)
}

/// F-B (passe 3, point 6) : l'autorisation annuelle `week_end_seul` ne couvre le départ seul d'une absence que si la
/// première nuit est une veille de week-end (le vendredi) **et** la dernière nuit au plus celle du dimanche.
fn week_end_complet(du_nuit: NaiveDate, au_nuit: NaiveDate) -> bool {
    du_nuit.weekday() == Weekday::Fri && au_nuit <= du_nuit + Duration::days(2)
}

/// Deux fiches désignent-elles la même personne (la même, ou reliées par une fusion) ?
async fn meme_personne_tx(conn: &mut PgConnection, etab: Uuid, a: Uuid, b: Uuid) -> Result<bool, EntDbError> {
    if a == b {
        return Ok(true);
    }
    let chaine = chaine_complete_sql("$2");
    sqlx::query_scalar(&format!("SELECT $3::uuid IN {chaine}"))
        .bind(etab)
        .bind(a)
        .bind(b)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

/// Une personne peut-elle prendre l'élève aujourd'hui ? (socle 4) : titulaire (lien actif, autorité parentale,
/// garde non « aucune ») ou personne autorisée à récupérer (contact validé dont le lien avec l'élève n'est ni
/// suspendu, ni en garde « aucune », ni celui d'un parent ou tuteur privé de l'autorité parentale, ni celui d'un parent
/// ou tuteur **dont le lien est terminé ou pas encore commencé** : décision prudente, voir `ent_internat_remise`), non
/// fusionnée dans une autre. **Une interdiction bloque toujours**, sur toute la chaîne de fusions, et sans attendre sa
/// date de début (décision pessimiste, comme E3 d'I2). Sans cible : voir [`remise_pour_tx`].
async fn remise_possible_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<Remise, EntDbError> {
    remise_pour_tx(conn, etab, None, eleve, personne, jour).await
}

/// Comme [`remise_possible_tx`], pour une absence ou une sortie précise : une interdiction **sans personne identifiée**
/// ne bloque plus la remise si la **direction l'a levée explicitement pour ce cas** (décision tracée, postérieure à la
/// pose de l'interdiction : levée ponctuelle). Une interdiction nominative ne se lève jamais ainsi.
async fn remise_pour_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    cible: Option<Cible>,
    eleve: Uuid,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<Remise, EntDbError> {
    let (code, relation) = remise_sql_tx(conn, etab, eleve, personne, jour, false).await?;
    if code == "sans_cible" {
        if let Some(cible) = cible {
            let depuis = interdiction_sans_cible_depuis_tx(conn, etab, eleve, jour, false).await?;
            if let Some(depuis) = depuis {
                if decision_apres_tx(conn, etab, cible, depuis).await? {
                    let (code, relation) = remise_sql_tx(conn, etab, eleve, personne, jour, true).await?;
                    return Ok(remise_de(&code, relation));
                }
            }
        }
    }
    Ok(remise_de(&code, relation))
}

async fn remise_sql_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    personne: Uuid,
    jour: NaiveDate,
    sans_cible_levee: bool,
) -> Result<(String, Option<String>), EntDbError> {
    sqlx::query_as("SELECT code, relation FROM ent_internat_remise($1, $2, $3, $4, $5)")
        .bind(etab)
        .bind(eleve)
        .bind(personne)
        .bind(jour)
        .bind(sans_cible_levee)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

/// **Point d'entrée unique pour les autres socles** (infirmerie, …) : la personne peut-elle prendre l'élève ce jour-là ?
/// Même fonction SQL de remise que l'internat (`ent_internat_remise` : titulaire, personne autorisée, tiers autorisé ;
/// interdiction toujours prioritaire sur toute la chaîne de fusions ; interdiction sans personne identifiée = refus ;
/// acte qui tombe avec son auteur). Aucun autre module ne doit réécrire cette règle.
pub(crate) async fn personne_peut_prendre_l_eleve_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    Ok(matches!(remise_possible_tx(conn, etab, eleve, personne, jour).await?, Remise::Autorisee(_)))
}

fn remise_de(code: &str, relation: Option<String>) -> Remise {
    match code {
        "interdite" => Remise::Interdite,
        "sans_cible" => Remise::InterditeSansCible,
        "autorisee" => Remise::Autorisee(relation.unwrap_or_default()),
        _ => Remise::NonAutorisee,
    }
}

/// Statut de remise de plusieurs couples (élève, personne, jour) en **une** requête (revue à la minute). Une interdiction
/// sans personne identifiée reste « sans_cible » ici : l'appelant relit alors, au cas par cas, la levée de la direction.
async fn remises_en_lot_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    couples: &[(Uuid, Uuid, NaiveDate)],
) -> Result<HashMap<(Uuid, Uuid, NaiveDate), Remise>, EntDbError> {
    if couples.is_empty() {
        return Ok(HashMap::new());
    }
    let eleves: Vec<Uuid> = couples.iter().map(|c| c.0).collect();
    let personnes: Vec<Uuid> = couples.iter().map(|c| c.1).collect();
    let jours: Vec<NaiveDate> = couples.iter().map(|c| c.2).collect();
    let lignes: Vec<(Uuid, Uuid, NaiveDate, String, Option<String>)> = sqlx::query_as(
        "SELECT k.e, k.p, k.j, r.code, r.relation \
           FROM unnest($2::uuid[], $3::uuid[], $4::date[]) AS k(e, p, j), \
                LATERAL ent_internat_remise($1, k.e, k.p, k.j, false) r",
    )
    .bind(etab)
    .bind(&eleves)
    .bind(&personnes)
    .bind(&jours)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(lignes.into_iter().map(|(e, p, j, code, rel)| ((e, p, j), remise_de(&code, rel))).collect())
}

/// Élèves (parmi les couples élève / jour) visés par une interdiction de remise **sans personne identifiée** : une requête
/// pour tous. Les autres n'ont rien à vérifier (la lecture fine, par cible, ne se fait que pour ceux-là).
async fn sans_cible_en_lot_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    couples: &[(Uuid, NaiveDate)],
) -> Result<HashSet<Uuid>, EntDbError> {
    if couples.is_empty() {
        return Ok(HashSet::new());
    }
    let eleves: Vec<Uuid> = couples.iter().map(|c| c.0).collect();
    let jours: Vec<NaiveDate> = couples.iter().map(|c| c.1).collect();
    let lignes: Vec<(Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT k.e, ent_internat_interdiction_sans_cible($1, k.e, k.j, true) \
           FROM unnest($2::uuid[], $3::date[]) AS k(e, j)",
    )
    .bind(etab)
    .bind(&eleves)
    .bind(&jours)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(lignes.into_iter().filter_map(|(e, d)| d.map(|_| e)).collect())
}

/// Types de sortie de l'établissement (créés aux valeurs par défaut à la première lecture).
async fn types_tx(conn: &mut PgConnection, etab: Uuid) -> Result<Vec<SortieTypeDto>, EntDbError> {
    for (code, libelle, autorisation, decideur, seul, eleve, corr, duree, usage) in TYPES_DEFAUT {
        sqlx::query(
            "INSERT INTO ent_internat_sortie_type \
             (etablissement_id, code, libelle, autorisation, decideur, seul_permis, eleve_peut_demander, \
              correspondant_requis, duree_max_heures, usage_accord_code) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT DO NOTHING",
        )
        .bind(etab)
        .bind(code)
        .bind(libelle)
        .bind(autorisation)
        .bind(decideur)
        .bind(seul)
        .bind(eleve)
        .bind(corr)
        .bind(duree)
        .bind(usage)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    let rows = sqlx::query(
        "SELECT code, libelle, actif, autorisation, decideur, seul_permis, eleve_peut_demander, \
                correspondant_requis, duree_max_heures, usage_accord_code \
           FROM ent_internat_sortie_type WHERE etablissement_id = $1 ORDER BY code",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| SortieTypeDto {
            code: r.get("code"),
            libelle: r.get("libelle"),
            actif: r.get("actif"),
            autorisation: r.get("autorisation"),
            decideur: r.get("decideur"),
            seul_permis: r.get("seul_permis"),
            eleve_peut_demander: r.get("eleve_peut_demander"),
            correspondant_requis: r.get("correspondant_requis"),
            duree_max_heures: r.get("duree_max_heures"),
            usage_accord_code: r.get("usage_accord_code"),
        })
        .collect())
}

async fn type_tx(conn: &mut PgConnection, etab: Uuid, code: &str) -> Result<SortieTypeDto, EntDbError> {
    types_tx(conn, etab)
        .await?
        .into_iter()
        .find(|t| t.code == code)
        .ok_or_else(|| refus("type de sortie inconnu"))
}

// ───────────────────────── Lignes ─────────────────────────

#[derive(sqlx::FromRow)]
struct AbsenceLigne {
    id: Uuid,
    eleve_id: Uuid,
    du_nuit: NaiveDate,
    au_nuit: NaiveDate,
    mode: String,
    statut: String,
    tardive: bool,
    annonce_qualite: String,
    annonce_par_user_id: Uuid,
    recupere_par_personne_id: Option<Uuid>,
    retour_prevu: Option<DateTime<Utc>>,
    decide_le: Option<DateTime<Utc>>,
    motif_refus_texte: Option<String>,
    avant_blocage: Option<String>,
    depart_effectif: Option<DateTime<Utc>>,
    depart_remis_a_personne_id: Option<Uuid>,
    cree_le: DateTime<Utc>,
}

const COLONNES_ABSENCE: &str = "id, eleve_id, du_nuit, au_nuit, mode, statut, tardive, annonce_qualite, \
     annonce_par_user_id, recupere_par_personne_id, retour_prevu, decide_le, motif_refus_texte, avant_blocage, \
     depart_effectif, depart_remis_a_personne_id, cree_le";

#[derive(sqlx::FromRow)]
struct SortieLigne {
    id: Uuid,
    eleve_id: Uuid,
    type_code: String,
    debut: DateTime<Utc>,
    fin_prevue: DateTime<Utc>,
    fin_ajustee: DateTime<Utc>,
    seul: bool,
    avec_personne_id: Option<Uuid>,
    correspondant_id: Option<Uuid>,
    demande_par_user_id: Option<Uuid>,
    statut: String,
    avant_blocage: Option<String>,
    accord_titulaire_le: Option<DateTime<Utc>>,
    accord_titulaire_user_id: Option<Uuid>,
    decision_cpe_le: Option<DateTime<Utc>>,
    par_autorisation_annuelle: bool,
    depart_effectif: Option<DateTime<Utc>>,
    retour_effectif: Option<DateTime<Utc>>,
    retard_demande_fin: Option<DateTime<Utc>>,
    cree_le: DateTime<Utc>,
}

const COLONNES_SORTIE: &str = "id, eleve_id, type_code, debut, fin_prevue, fin_ajustee, seul, avec_personne_id, \
     correspondant_id, demande_par_user_id, statut, avant_blocage, accord_titulaire_le, \
     accord_titulaire_user_id, decision_cpe_le, par_autorisation_annuelle, depart_effectif, retour_effectif, \
     retard_demande_fin, cree_le";

async fn absence_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid, verrou: bool) -> Result<AbsenceLigne, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES_ABSENCE} FROM ent_internat_absence WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(|| DomainError::NotFound.into())
}

async fn sortie_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid, verrou: bool) -> Result<SortieLigne, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES_SORTIE} FROM ent_internat_sortie WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(|| DomainError::NotFound.into())
}

fn absence_dto(l: &AbsenceLigne, nom: Option<(String, String)>, protege: bool, opposition: bool) -> AbsenceDto {
    AbsenceDto {
        id: l.id,
        eleve_id: l.eleve_id,
        nom: nom.as_ref().map(|n| n.0.clone()),
        prenom: nom.map(|n| n.1),
        protege,
        du_nuit: l.du_nuit,
        au_nuit: l.au_nuit,
        mode: l.mode.clone(),
        statut: l.statut.clone(),
        tardive: l.tardive,
        annonce_qualite: l.annonce_qualite.clone(),
        recupere_par_personne_id: l.recupere_par_personne_id,
        retour_prevu: l.retour_prevu,
        decide_le: l.decide_le,
        motif_refus_texte: l.motif_refus_texte.clone(),
        opposition,
        depart_effectif: l.depart_effectif,
        cree_le: l.cree_le,
    }
}

fn sortie_dto(
    l: &SortieLigne,
    nom: Option<(String, String)>,
    protege: bool,
    opposition: bool,
    motif_refus: Option<String>,
) -> SortieDto {
    SortieDto {
        id: l.id,
        eleve_id: l.eleve_id,
        nom: nom.as_ref().map(|n| n.0.clone()),
        prenom: nom.map(|n| n.1),
        protege,
        type_code: l.type_code.clone(),
        debut: l.debut,
        fin_prevue: l.fin_prevue,
        fin_ajustee: l.fin_ajustee,
        seul: l.seul,
        avec_personne_id: l.avec_personne_id,
        correspondant_id: l.correspondant_id,
        statut: l.statut.clone(),
        accord_titulaire: l.accord_titulaire_le.is_some(),
        decision_cpe: l.decision_cpe_le.is_some(),
        par_autorisation_annuelle: l.par_autorisation_annuelle,
        depart_effectif: l.depart_effectif,
        retour_effectif: l.retour_effectif,
        opposition,
        motif_refus_texte: motif_refus,
        retard_demande: l.retard_demande_fin,
        cree_le: l.cree_le,
    }
}

/// Motif lu par la famille : celui du dernier refus ou du dernier maintien d'un blocage **par un CPE ou la direction**
/// (jamais le texte d'un autre titulaire, refus de sortie compris).
async fn motif_refus_sortie_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<Option<String>, EntDbError> {
    sqlx::query_scalar(
        "SELECT motif_texte FROM ent_internat_i3_fait WHERE etablissement_id = $1 AND sortie_id = $2 \
            AND acte IN ('refus', 'tranchee_maintenue') AND qualite IN ('cpe', 'direction') AND motif_texte IS NOT NULL \
          ORDER BY a DESC, cree_le DESC LIMIT 1",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)
}

async fn noms_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, (String, String)>, EntDbError> {
    let rows: Vec<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)")
            .bind(etab)
            .bind(ids)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
    Ok(rows.into_iter().map(|(i, n, p)| (i, (n, p))).collect())
}

/// Statut d'une absence qui est « en vigueur » (l'élève n'est plus attendu).
fn absence_en_vigueur(statut: &str) -> bool {
    matches!(statut, "annoncee" | "acceptee" | "regularisee")
}

// ───────────────────────── Service : acteur, types, absences ─────────────────────────

impl PgEnt {
    /// Lien de titulaire du compte pour l'élève : lien communicant, autorité parentale, garde non « aucune », et
    /// **personne non visée par une interdiction** de remise, de visite ou de contact (C-1) : un parent interdit de
    /// remise n'est acteur d'aucune action du lot. `Some(peut_autoriser_sorties)` si le compte est titulaire.
    /// M-A (règle D25 de la messagerie, migration 481) : un lien créé APRÈS une interdiction sans personne identifiée,
    /// non levée et non terminée, et tant que la direction ne l'a pas confirmé, n'est ni acteur ni lecteur du lot.
    async fn titulaire_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        eleve: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<bool>, EntDbError> {
        let liens: Vec<(bool, Uuid)> = sqlx::query_as(
            "SELECT l.peut_autoriser_sorties, l.personne_id FROM ent_eleve_lien_communicant l \
              WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.eleve_id = $3 \
                AND l.autorite_parentale AND COALESCE(l.garde, '') <> 'aucune' \
                AND NOT EXISTS (SELECT 1 FROM ent_eleve_interdiction i \
                                 WHERE i.etablissement_id = l.etablissement_id AND i.eleve_id = l.eleve_id \
                                   AND i.levee_le IS NULL AND i.personne_id IS NULL \
                                   AND i.objet && ARRAY['remise', 'visite', 'contact_eleve']::text[] \
                                   AND (i.au IS NULL OR i.au >= $4) AND l.created_at >= i.created_at \
                                   AND NOT EXISTS (SELECT 1 FROM ent_msg_lien_decision v \
                                                    WHERE v.lien_id = l.id AND v.interdiction_id = i.id \
                                                      AND v.decision = 'confirme')) \
              ORDER BY l.id",
        )
        .bind(etab)
        .bind(user)
        .bind(eleve)
        .bind(jour)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (peut, personne) in liens {
            if !personne_interdite_tx(conn, etab, eleve, personne, jour).await? {
                return Ok(Some(peut));
            }
        }
        Ok(None)
    }

    /// Qualité familiale **d'acteur** du compte pour l'élève : titulaire (lien communicant, autorité parentale,
    /// garde non « aucune », personne non interdite de remise), élève (compte de l'élève ; majeur à son 18ᵉ
    /// anniversaire, ou émancipé ; jamais majeur sous tutelle). `None` pour tout autre compte.
    ///
    /// **E-1** : dès que l'élève est majeur (R-IT22), ses parents ne sont plus acteurs (annonce, accord, opposition,
    /// révocation, correspondants) ; leur lecture est réglée par `famille_lecture_tx`.
    async fn famille_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        role: &str,
        eleve: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<Famille>, EntDbError> {
        match Role::parse(role) {
            Some(Role::Parent) => {
                if eleve_majeur_tx(conn, etab, eleve, jour).await? {
                    return Ok(None);
                }
                let r = self.titulaire_tx(conn, etab, user, eleve, jour).await?;
                Ok(r.map(|peut| Famille { qualite: Qualite::Titulaire, peut_autoriser: peut }))
            }
            Some(Role::Eleve) => {
                let propre: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_eleve_compte \
                      WHERE etablissement_id = $1 AND user_id = $2 AND eleve_id = $3)",
                )
                .bind(etab)
                .bind(user)
                .bind(eleve)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                if !propre {
                    return Ok(None);
                }
                if eleve_majeur_tx(conn, etab, eleve, jour).await? {
                    Ok(Some(Famille { qualite: Qualite::EleveMajeur, peut_autoriser: true }))
                } else {
                    Ok(Some(Famille { qualite: Qualite::Eleve, peut_autoriser: false }))
                }
            }
            _ => Ok(None),
        }
    }

    /// Qualité familiale **de lecteur** : celle d'acteur, plus les parents d'un élève majeur quand le règlement le
    /// prévoit (`majeur_info_parents = toujours`). « si_a_charge » ne vise que les payeurs, notion du socle financier
    /// que ce lot ne lit pas : sans elle, la lecture est refusée (décision prudente). Un parent interdit de remise ne
    /// lit rien des absences, sorties, correspondants et historique de l'élève (C-1).
    async fn famille_lecture_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        role: &str,
        eleve: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<Famille>, EntDbError> {
        if let Some(f) = self.famille_tx(conn, etab, user, role, eleve, jour).await? {
            return Ok(Some(f));
        }
        if !matches!(Role::parse(role), Some(Role::Parent)) || !eleve_majeur_tx(conn, etab, eleve, jour).await? {
            return Ok(None);
        }
        if parametres_tx(conn, etab).await?.majeur_info_parents != "toujours" {
            return Ok(None);
        }
        let r = self.titulaire_tx(conn, etab, user, eleve, jour).await?;
        Ok(r.map(|_| Famille { qualite: Qualite::Titulaire, peut_autoriser: false }))
    }

    /// Le personnel agit sur un élève de sa portée (R-IT30) avec l'une des actions données ; `Forbidden` sinon
    /// (y compris pour un compte sans droit). Un élève protégé caché au tiers répond « introuvable » (E7).
    async fn personnel_exiger(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        actions: &[Action],
        eleve: Uuid,
    ) -> Result<(), EntDbError> {
        let portee = self.nuit_portee(etab, actor, role, actions).await?;
        if portee.as_ref().is_some_and(|p| !p.contains(&eleve)) {
            return Err(EntDbError::Forbidden);
        }
        self.exiger_eleve_visible(etab, actor, role, eleve).await
    }

    async fn eleve_existe(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<(), EntDbError> {
        let ok: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)")
            .bind(etab)
            .bind(eleve)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
        if ok {
            Ok(())
        } else {
            Err(DomainError::NotFound.into())
        }
    }

    /// Noms lisibles pour une liste du personnel : un élève protégé caché au tiers n'est pas nommé.
    async fn noms_pour_personnel(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        conn: &mut PgConnection,
        ids: &[Uuid],
    ) -> Result<(HashMap<Uuid, (String, String)>, HashSet<Uuid>, HashSet<Uuid>), EntDbError> {
        let noms = noms_tx(conn, etab, ids).await?;
        let (proteges, caches) = self.masquer_proteges(etab, actor, role, ids, "internat.sorties").await?;
        Ok((noms, proteges, caches))
    }

    // ───────────────────────── Types de sortie (S-4105) ─────────────────────────

    /// Types de sortie de l'établissement : réglage sans donnée personnelle, lisible de tout compte de
    /// l'établissement (la famille et l'élève choisissent un type).
    pub async fn lister_types_sortie_a(&self, etab: Uuid) -> Result<Vec<SortieTypeDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let t = types_tx(&mut tx, etab).await?;
        tx.commit().await.map_err(db)?;
        Ok(t)
    }

    /// Modifie un type de sortie (jamais supprimé : on le désactive). Exige `internat.gerer`.
    pub async fn modifier_type_sortie_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        code: &str,
        b: &SortieTypeSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieTypeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        if let Some(a) = b.autorisation.as_deref() {
            if !AUTORISATIONS.contains(&a) {
                return Err(refus("règle d'autorisation inconnue"));
            }
        }
        if let Some(d) = b.decideur.as_deref() {
            if !DECIDEURS.contains(&d) {
                return Err(refus("décideur inconnu"));
            }
        }
        if b.duree_max_heures.is_some_and(|h| !(1..=200).contains(&h)) {
            return Err(refus("durée maximale invalide (1 à 200 heures)"));
        }
        let libelle = b.libelle.as_deref().map(str::trim);
        if libelle.is_some_and(|l| l.is_empty() || l.chars().count() > 80) {
            return Err(refus("libellé invalide (1 à 80 caractères)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        types_tx(&mut tx, etab).await?;
        let n = sqlx::query(
            "UPDATE ent_internat_sortie_type SET \
                libelle = COALESCE($3, libelle), actif = COALESCE($4, actif), \
                autorisation = COALESCE($5, autorisation), decideur = COALESCE($6, decideur), \
                seul_permis = COALESCE($7, seul_permis), eleve_peut_demander = COALESCE($8, eleve_peut_demander), \
                correspondant_requis = COALESCE($9, correspondant_requis), \
                duree_max_heures = COALESCE($10, duree_max_heures), modifie_le = $11 \
              WHERE etablissement_id = $1 AND code = $2",
        )
        .bind(etab)
        .bind(code)
        .bind(libelle)
        .bind(b.actif)
        .bind(b.autorisation.as_deref())
        .bind(b.decideur.as_deref())
        .bind(b.seul_permis)
        .bind(b.eleve_peut_demander)
        .bind(b.correspondant_requis)
        .bind(b.duree_max_heures)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound.into());
        }
        let t = type_tx(&mut tx, etab, code).await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_PARAMETRES,
            AggregateKind::Internat,
            Uuid::new_v4(),
            None,
            json!({ "objet": "sortie_type", "code": code, "actif": t.actif }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(t)
    }

    // ───────────────────────── Absences annoncées ─────────────────────────

    /// INT-150, INT-153, INT-165 : annonce d'absence d'internat (une ou plusieurs nuits). Titulaire ou élève
    /// majeur ; le CPE peut régulariser lui-même une nuit passée. Voir l'en-tête du module pour les règles.
    pub async fn annoncer_absence_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &AbsenceAnnonceDto,
        maintenant: DateTime<Utc>,
    ) -> Result<AbsenceDto, EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        if b.au_nuit < b.du_nuit {
            return Err(refus("la dernière nuit précède la première"));
        }
        if (b.au_nuit - b.du_nuit).num_days() >= 60 {
            return Err(refus("une absence couvre 60 nuits au plus : l'annoncer en plusieurs fois"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let courante = nuit_a_tx(&mut tx, etab, maintenant).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let (qualite, par_cpe) = match self.famille_tx(&mut tx, etab, actor, role, b.eleve_id, jour).await? {
            Some(Famille { qualite: Qualite::Titulaire, .. }) => (Qualite::Titulaire, false),
            Some(Famille { qualite: Qualite::EleveMajeur, .. }) => (Qualite::EleveMajeur, false),
            Some(Famille { qualite: Qualite::Eleve, .. }) => {
                return Err(refus(
                    "un élève mineur n'annonce pas lui-même une absence : un titulaire de l'autorité parentale l'annonce",
                ))
            }
            _ => {
                // Ni titulaire ni élève : seul le CPE régularise une nuit passée (CIT-37).
                self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], b.eleve_id).await?;
                if !b.regularisation {
                    return Err(refus(
                        "une absence est annoncée par un titulaire ou par l'élève majeur (R-IT19) : le personnel pose un retrait ou régularise",
                    ));
                }
                (Qualite::Personnel, true)
            }
        };
        Self::eleve_existe(&mut tx, etab, b.eleve_id).await?;
        if b.regularisation {
            if b.au_nuit >= courante {
                return Err(refus("une régularisation ne couvre que des nuits déjà passées"));
            }
            if (courante - b.du_nuit).num_days() > REGULARISATION_JOURS_MAX {
                return Err(refus("une nuit de plus de 45 jours ne se régularise plus ici : s'adresser à la direction"));
            }
            if motif.is_none() {
                return Err(refus("une régularisation exige un motif court"));
            }
        } else {
            if b.du_nuit < courante {
                return Err(refus(
                    "une nuit passée ne s'annonce pas : demander une régularisation au CPE avec un motif (CIT-37)",
                ));
            }
            if (b.au_nuit - courante).num_days() > HORIZON_ANNONCE_JOURS {
                return Err(refus("une absence s'annonce un an à l'avance au plus"));
            }
        }
        if let Some(p) = b.recupere_par_personne_id {
            let pour = if b.regularisation { b.du_nuit } else { b.du_nuit.max(jour) };
            if !matches!(remise_possible_tx(&mut tx, etab, b.eleve_id, p, pour).await?, Remise::Autorisee(_)) {
                return Err(refus("cette personne n'est pas autorisée à récupérer l'élève"));
            }
        }
        // C-1 : pour un mineur, une absence annoncée exige la personne qui récupère l'enfant (même quand l'annonceur
        // est lui-même cette personne : le contrôle de remise ci-dessus s'applique alors aussi), sauf départ seul
        // couvert par l'autorisation annuelle valable du socle 18 (usage « départ seul le week-end »).
        if qualite == Qualite::Titulaire && !b.regularisation && b.recupere_par_personne_id.is_none() {
            let pour = b.du_nuit.max(jour);
            if !eleve_majeur_tx(&mut tx, etab, b.eleve_id, pour).await? {
                let t = type_tx(&mut tx, etab, "week_end").await?;
                if !week_end_complet(b.du_nuit, b.au_nuit)
                    || verdict_annuel(self, &mut tx, etab, b.eleve_id, &t.usage_accord_code, pour).await != "autorise"
                {
                    return Err(refus(
                        "pour un élève mineur, indiquer la personne qui le récupère (R-IT23), sauf départ seul couvert par l'autorisation annuelle de la famille (socle 18) pour une absence qui commence la veille d'un week-end",
                    ));
                }
            }
        }
        let doublon: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_internat_absence \
              WHERE etablissement_id = $1 AND eleve_id = $2 AND statut IN ('demandee', 'annoncee', 'acceptee', 'bloquee', 'regularisee') \
                AND du_nuit <= $4 AND au_nuit >= $3)",
        )
        .bind(etab)
        .bind(b.eleve_id)
        .bind(b.du_nuit)
        .bind(b.au_nuit)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if doublon {
            return Err(conflit("une absence est déjà annoncée sur certaines de ces nuits"));
        }
        // Heure limite du jour de la première nuit (S-4104) : après elle, l'annonce part au CPE comme demande.
        let limite = parametres_tx(&mut tx, etab).await?.heure_limite_annonce;
        let instant_limite = instant_mural_tx(&mut tx, etab, b.du_nuit, limite).await?;
        let tardive = !b.regularisation && maintenant > instant_limite;
        let initial = if b.regularisation {
            if par_cpe {
                "regularisee"
            } else {
                "demandee"
            }
        } else if tardive {
            "demandee"
        } else {
            "annoncee"
        };
        // R-IT20 : un désaccord connu des titulaires bloque l'annonce et la renvoie au CPE.
        let bloquee = !par_cpe && desaccord_connu_tx(&mut tx, etab, b.eleve_id).await?;
        let statut = if bloquee { "bloquee" } else { initial };
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_absence \
             (id, etablissement_id, eleve_id, du_nuit, au_nuit, mode, annonce_par_user_id, annonce_qualite, \
              recupere_par_personne_id, retour_prevu, statut, tardive, avant_blocage, decide_par_user_id, decide_le, \
              motif_refus_texte, cree_le, modifie_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,NULL,$16,$16)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(b.du_nuit)
        .bind(b.au_nuit)
        .bind(if b.regularisation { "regularisation" } else { "annonce" })
        .bind(actor)
        .bind(qualite.code())
        .bind(b.recupere_par_personne_id)
        .bind(b.retour_prevu)
        .bind(statut)
        .bind(tardive)
        .bind(bloquee.then_some(initial).filter(|i| *i != "regularisee"))
        .bind(par_cpe.then_some(actor))
        .bind(par_cpe.then_some(maintenant))
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if matches!(&e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23P01")) {
                conflit("une absence est déjà annoncée sur certaines de ces nuits")
            } else {
                db(e)
            }
        })?;
        let acte = match (b.regularisation, par_cpe) {
            (true, true) => "regularisation",
            (true, false) => "regularisation_demandee",
            (false, _) if tardive => "demande",
            _ => "annonce",
        };
        let qualite_fait = if par_cpe { "cpe" } else { qualite.code() };
        fait_tx(
            &mut tx,
            etab,
            Cible::Absence(id),
            acte,
            qualite_fait,
            Some(actor),
            maintenant,
            tardive.then_some("tardive"),
            motif.as_deref(),
            b.recupere_par_personne_id,
            b.retour_prevu,
        )
        .await?;
        if bloquee {
            fait_tx(&mut tx, etab, Cible::Absence(id), "blocage", "systeme", None, maintenant, Some("desaccord_connu"), None, None, None)
                .await?;
        }
        let evt = if b.regularisation && par_cpe { types::INTERNAT_ABSENCE_REGULARISEE } else { types::INTERNAT_ABSENCE_ANNONCEE };
        publier(
            &mut tx,
            etab,
            evt,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({
                "du": b.du_nuit.to_string(), "au": b.au_nuit.to_string(), "statut": statut,
                "tardive": tardive, "mode": if b.regularisation { "regularisation" } else { "annonce" },
                "qualite": qualite_fait
            }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        if bloquee {
            publier(
                &mut tx,
                etab,
                types::INTERNAT_ABSENCE_BLOQUEE,
                AggregateKind::Internat,
                id,
                Some(b.eleve_id),
                json!({ "du": b.du_nuit.to_string(), "au": b.au_nuit.to_string(), "cause": "desaccord_connu" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        if absence_en_vigueur(statut) {
            publier(
                &mut tx,
                etab,
                types::INTERNAT_ATTENDU_MODIFIE,
                AggregateKind::Internat,
                id,
                Some(b.eleve_id),
                json!({ "du": b.du_nuit.to_string(), "au": b.au_nuit.to_string(), "cause": "absence_annoncee" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        if qualite == Qualite::EleveMajeur {
            self.informer_parents_du_majeur_tx(&mut tx, etab, actor, b.eleve_id, "absence", id).await?;
        }
        let ligne = absence_tx(&mut tx, etab, id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(absence_dto(&ligne, None, false, false))
    }

    /// R-IT22 : les parents d'un élève majeur sont informés **seulement si** le règlement le prévoit
    /// (`majeur_info_parents`) ; « si_a_charge » ne vise que les titulaires payeurs. L'événement ne porte que
    /// des identifiants et la politique ; la résolution des destinataires relève du socle 8.
    async fn informer_parents_du_majeur_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        eleve: Uuid,
        nature: &str,
        objet: Uuid,
    ) -> Result<(), EntDbError> {
        let politique = parametres_tx(conn, etab).await?.majeur_info_parents;
        // F-E : « si_a_charge » ne vise que les payeurs, notion que ce lot ne lit pas : la lecture est refusée aux
        // parents (`famille_lecture_tx`), donc aucun événement d'information n'est publié non plus. Le futur résolveur
        // de destinataires (socle 8) écartera, de son côté, les parents visés par une interdiction (remise, visite,
        // contact) avant tout envoi.
        if politique != "toujours" {
            return Ok(());
        }
        publier(
            conn,
            etab,
            types::INTERNAT_MAJEUR_INFORME,
            AggregateKind::Internat,
            objet,
            Some(eleve),
            json!({ "nature": nature, "politique": politique }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        Ok(())
    }

    /// Absences d'un élève, des plus récentes aux plus anciennes : sa famille (titulaire, élève) ou le
    /// personnel qui lit l'internat dans sa portée. La famille voit le statut, le motif du refus et
    /// qu'« une opposition existe », jamais de qui.
    pub async fn absences_eleve_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<AbsenceDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let famille = self.famille_lecture_tx(&mut tx, etab, actor, role, eleve, jour).await?;
        if famille.is_none() {
            self.personnel_exiger(etab, actor, role, &[Action::InternatLire, Action::InternatSortiesDecider], eleve).await?;
        }
        let lignes: Vec<AbsenceLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_ABSENCE} FROM ent_internat_absence WHERE etablissement_id = $1 AND eleve_id = $2 \
              ORDER BY du_nuit DESC, cree_le DESC LIMIT 200"
        ))
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let (noms, proteges, caches) = if famille.is_some() {
            (noms_tx(&mut tx, etab, &[eleve]).await?, HashSet::new(), HashSet::new())
        } else {
            self.noms_pour_personnel(etab, actor, role, &mut tx, &[eleve]).await?
        };
        let mut sortie = Vec::with_capacity(lignes.len());
        for l in &lignes {
            let opp = opposition_tx(&mut tx, etab, Cible::Absence(l.id)).await?;
            let nom = (!caches.contains(&eleve)).then(|| noms.get(&eleve).cloned()).flatten();
            sortie.push(absence_dto(l, nom, proteges.contains(&eleve), opp));
        }
        tx.commit().await.map_err(db)?;
        Ok(sortie)
    }

    /// INT-151, INT-152 : file du CPE : demandes tardives et régularisations en attente, absences bloquées
    /// (désaccord), annonces à relire. Personnel avec `internat.sorties.decider`, dans sa portée.
    pub async fn absences_a_traiter_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<AbsenceDto>, EntDbError> {
        let portee = self.nuit_portee(etab, actor, role, &[Action::InternatSortiesDecider]).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<AbsenceLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_ABSENCE} FROM ent_internat_absence \
              WHERE etablissement_id = $1 AND statut IN ('demandee', 'bloquee', 'annoncee') \
              ORDER BY CASE statut WHEN 'bloquee' THEN 0 WHEN 'demandee' THEN 1 ELSE 2 END, du_nuit, cree_le LIMIT 500"
        ))
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let lignes: Vec<AbsenceLigne> = lignes
            .into_iter()
            .filter(|l| portee.as_ref().is_none_or(|p| p.contains(&l.eleve_id)))
            .collect();
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.eleve_id).collect();
        let (noms, proteges, caches) = self.noms_pour_personnel(etab, actor, role, &mut tx, &ids).await?;
        let mut sortie = Vec::with_capacity(lignes.len());
        for l in &lignes {
            let opp = opposition_tx(&mut tx, etab, Cible::Absence(l.id)).await?;
            let nom = (!caches.contains(&l.eleve_id)).then(|| noms.get(&l.eleve_id).cloned()).flatten();
            sortie.push(absence_dto(l, nom, proteges.contains(&l.eleve_id), opp));
        }
        tx.commit().await.map_err(db)?;
        Ok(sortie)
    }

    /// INT-151, INT-152, INT-153 : le CPE accepte, refuse (motif lu par la famille) ou régularise.
    pub async fn decider_absence_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &DecisionDto,
        maintenant: DateTime<Utc>,
    ) -> Result<AbsenceDto, EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = absence_tx(&mut tx, etab, id, true).await?;
        self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], a.eleve_id).await?;
        let (nouveau, acte) = match (b.decision.as_str(), a.statut.as_str()) {
            ("accepter", "demandee" | "annoncee") if a.mode == "annonce" => ("acceptee", "acceptation"),
            ("regulariser", "demandee") if a.mode == "regularisation" => ("regularisee", "regularisation"),
            ("refuser", "demandee" | "annoncee") => ("refusee", "refus"),
            ("accepter" | "refuser" | "regulariser", "bloquee") => {
                return Err(conflit("cette demande est bloquée par un désaccord : la direction tranche"))
            }
            ("accepter" | "refuser" | "regulariser", _) => {
                return Err(conflit("cette demande n'attend plus de décision (déjà décidée, annulée ou d'un autre type)"))
            }
            _ => return Err(refus("décision inconnue (accepter, refuser, regulariser)")),
        };
        let motif = if nouveau == "refusee" {
            Some(motif.ok_or_else(|| refus("un refus s'accompagne d'un motif lu par la famille"))?)
        } else {
            motif
        };
        if nouveau == "acceptee" {
            let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
            // Passe 4 (R3) : aucune décision du personnel sur un objet qui tombe. Accepter une absence dont un acte repose
            // sur une personne désormais visée (interdiction ou perte de droit) la ferait partir avec un acte caduc :
            // refus explicite (le personnel voit pourquoi ; la famille, elle, ne lit jamais la cause).
            if actes_qui_tombent_tx(&mut tx, etab, &[Objet::Absence(&a)], jour).await?.contains_key(&a.id) {
                return Err(conflit(
                    "cette demande repose sur un acte (annonce, personne désignée ou autorisation annuelle) posé par une personne qui n'a plus le droit d'agir pour cet élève : l'accepter est impossible ; la refuser, ou la faire annoncer de nouveau par un titulaire",
                ));
            }
            // Accepter ne rend jamais valable une remise devenue interdite (C-1).
            if let Some(p) = a.recupere_par_personne_id {
                let pour = a.du_nuit.max(jour);
                if !matches!(remise_possible_tx(&mut tx, etab, a.eleve_id, p, pour).await?, Remise::Autorisee(_)) {
                    return Err(conflit(
                        "la personne désignée ne peut plus récupérer l'élève (interdiction ou fin d'autorisation) : refuser, ou faire désigner une autre personne",
                    ));
                }
            }
        }
        sqlx::query(
            "UPDATE ent_internat_absence SET statut = $3, decide_par_user_id = $4, decide_le = $5, \
                    motif_refus_texte = CASE WHEN $3 = 'refusee' THEN $6 ELSE motif_refus_texte END, modifie_le = $5 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(nouveau)
        .bind(actor)
        .bind(maintenant)
        .bind(motif.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_tx(&mut tx, etab, Cible::Absence(id), acte, "cpe", Some(actor), maintenant, None, motif.as_deref(), None, None).await?;
        self.evenements_decision_absence_tx(&mut tx, etab, actor, &a, nouveau, a.statut.as_str()).await?;
        let ligne = absence_tx(&mut tx, etab, id, false).await?;
        let opp = opposition_tx(&mut tx, etab, Cible::Absence(id)).await?;
        tx.commit().await.map_err(db)?;
        Ok(absence_dto(&ligne, None, false, opp))
    }

    /// Événements d'une décision sur une absence : la décision (aucun texte libre), et l'attendu recalculé
    /// quand l'effet sur la nuit change.
    async fn evenements_decision_absence_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        a: &AbsenceLigne,
        nouveau: &str,
        ancien: &str,
    ) -> Result<(), EntDbError> {
        let evt = if nouveau == "regularisee" { types::INTERNAT_ABSENCE_REGULARISEE } else { types::INTERNAT_ABSENCE_DECIDEE };
        publier(
            conn,
            etab,
            evt,
            AggregateKind::Internat,
            a.id,
            Some(a.eleve_id),
            json!({
                "du": a.du_nuit.to_string(), "au": a.au_nuit.to_string(), "decision": nouveau,
                "tardive": a.tardive, "mode": a.mode
            }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        if absence_en_vigueur(nouveau) != absence_en_vigueur(ancien) {
            publier(
                conn,
                etab,
                types::INTERNAT_ATTENDU_MODIFIE,
                AggregateKind::Internat,
                a.id,
                Some(a.eleve_id),
                json!({ "du": a.du_nuit.to_string(), "au": a.au_nuit.to_string(), "cause": "absence_decidee" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        Ok(())
    }

    /// Annulation d'une absence par celui qui l'a annoncée, par un autre titulaire (qui retire sa demande) ou par
    /// le CPE ; tant qu'au moins une nuit n'est pas passée (sinon : régularisation).
    pub async fn annuler_absence_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<AbsenceDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = absence_tx(&mut tx, etab, id, true).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let courante = nuit_a_tx(&mut tx, etab, maintenant).await?;
        let famille = self.famille_tx(&mut tx, etab, actor, role, a.eleve_id, jour).await?;
        let qualite = match famille {
            Some(Famille { qualite: q @ (Qualite::Titulaire | Qualite::EleveMajeur), .. }) => q,
            Some(_) => return Err(EntDbError::Forbidden),
            None => {
                self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], a.eleve_id).await?;
                Qualite::Personnel
            }
        };
        if !matches!(a.statut.as_str(), "demandee" | "annoncee" | "acceptee" | "bloquee") {
            return Err(conflit("cette absence ne peut plus être annulée"));
        }
        if a.au_nuit < courante {
            return Err(conflit("toutes les nuits de cette absence sont passées : on ne les annule plus"));
        }
        sqlx::query(
            "UPDATE ent_internat_absence SET statut = 'annulee', avant_blocage = NULL, blocage_effet_nuit = NULL, \
                    modifie_le = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let q = if qualite == Qualite::Personnel { "cpe" } else { qualite.code() };
        fait_tx(&mut tx, etab, Cible::Absence(id), "annulation", q, Some(actor), maintenant, None, None, None, None).await?;
        self.evenements_decision_absence_tx(&mut tx, etab, actor, &a, "annulee", a.statut.as_str()).await?;
        let ligne = absence_tx(&mut tx, etab, id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(absence_dto(&ligne, None, false, false))
    }

    /// R-IT20 : un titulaire s'oppose à une absence annoncée par quelqu'un d'autre. Le plus restrictif l'emporte :
    /// l'absence est **bloquée** (l'élève redevient attendu) jusqu'à la décision de la direction, avec trace.
    pub async fn opposer_absence_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &OppositionDto,
        maintenant: DateTime<Utc>,
    ) -> Result<AbsenceDto, EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = absence_tx(&mut tx, etab, id, true).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let courante = nuit_a_tx(&mut tx, etab, maintenant).await?;
        match self.famille_tx(&mut tx, etab, actor, role, a.eleve_id, jour).await? {
            Some(Famille { qualite: Qualite::Titulaire, .. }) => {}
            _ => return Err(EntDbError::Forbidden),
        }
        if a.annonce_par_user_id == actor {
            return Err(refus("vous avez annoncé cette absence : l'annuler plutôt que s'y opposer"));
        }
        if !matches!(a.statut.as_str(), "demandee" | "annoncee" | "acceptee") {
            return Err(conflit("cette absence n'est plus en cours : aucune opposition possible"));
        }
        if a.au_nuit < courante {
            return Err(conflit("toutes les nuits de cette absence sont passées"));
        }
        // Moyen 5 : une opposition posée après le début de la nuit ne porte que sur les nuits SUIVANTES. Pour la nuit
        // en cours, l'absence continue de couvrir l'élève (il n'est pas remis à l'attendu en pleine nuit) et le CPE
        // est alerté : l'absence bloquée figure en tête de sa file.
        // M-B : seul un élève réellement parti (départ constaté) est concerné ; sans départ, il reste attendu.
        let effet_nuit = nuit_commencee_tx(&mut tx, etab, maintenant)
            .await?
            .filter(|c| a.depart_effectif.is_some() && absence_en_vigueur(&a.statut) && a.du_nuit <= *c && a.au_nuit >= *c)
            .map(|c| c + Duration::days(1));
        sqlx::query(
            "UPDATE ent_internat_absence SET statut = 'bloquee', avant_blocage = statut, blocage_effet_nuit = $4, \
                    modifie_le = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .bind(effet_nuit)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_tx(&mut tx, etab, Cible::Absence(id), "opposition", "titulaire", Some(actor), maintenant, Some("desaccord_titulaires"), motif.as_deref(), None, None)
            .await?;
        fait_tx(&mut tx, etab, Cible::Absence(id), "blocage", "systeme", None, maintenant, Some("desaccord_titulaires"), None, None, None)
            .await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_ABSENCE_BLOQUEE,
            AggregateKind::Internat,
            id,
            Some(a.eleve_id),
            json!({
                "du": a.du_nuit.to_string(), "au": a.au_nuit.to_string(), "cause": "opposition",
                "nuit_en_cours": effet_nuit.is_some()
            }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        if absence_en_vigueur(&a.statut) {
            publier(
                &mut tx,
                etab,
                types::INTERNAT_ATTENDU_MODIFIE,
                AggregateKind::Internat,
                id,
                Some(a.eleve_id),
                json!({ "du": a.du_nuit.to_string(), "au": a.au_nuit.to_string(), "cause": "absence_bloquee" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        let ligne = absence_tx(&mut tx, etab, id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(absence_dto(&ligne, None, false, true))
    }

    /// La direction tranche un désaccord entre titulaires : `lever` rétablit l'état d'avant le blocage ;
    /// `maintenir` refuse la demande (motif lu par la famille). Trace : qui, quand, quoi.
    pub async fn trancher_absence_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &TrancheDto,
        maintenant: DateTime<Utc>,
    ) -> Result<AbsenceDto, EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a = absence_tx(&mut tx, etab, id, true).await?;
        self.direction_exiger(etab, actor, role, a.eleve_id).await?;
        if a.statut != "bloquee" {
            return Err(conflit("cette absence n'est pas bloquée par un désaccord"));
        }
        let avant = a.avant_blocage.clone().ok_or_else(|| conflit("état antérieur inconnu"))?;
        let (nouveau, acte) = match b.decision.as_str() {
            "lever" => (avant.as_str(), "tranchee_levee"),
            "maintenir" => ("refusee", "tranchee_maintenue"),
            _ => return Err(refus("décision inconnue (lever, maintenir)")),
        };
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let mut motif_code = "desaccord_titulaires";
        if nouveau != "refusee" {
            motif_code = motif_de_levee_tx(&mut tx, etab, Cible::Absence(id), a.eleve_id, jour, motif.as_deref()).await?;
            // Lever un blocage ne rend jamais valable une remise devenue interdite (C-1) ; une interdiction SANS personne
            // identifiée, elle, est levée par cette décision même (levée ponctuelle de la direction).
            let pour = a.du_nuit.max(jour);
            if let Some(p) = a.recupere_par_personne_id {
                let (code, _) = remise_sql_tx(&mut tx, etab, a.eleve_id, p, pour, true).await?;
                if code != "autorisee" {
                    return Err(conflit(
                        "la personne désignée ne peut plus récupérer l'élève (interdiction ou fin d'autorisation) : maintenir le blocage, ou refuser la demande",
                    ));
                }
            } else if !eleve_majeur_tx(&mut tx, etab, a.eleve_id, pour).await? {
                // Un mineur sans personne désignée (acte tombé avec son auteur) ne part que par l'autorisation annuelle.
                let t = type_tx(&mut tx, etab, "week_end").await?;
                if !week_end_complet(a.du_nuit, a.au_nuit)
                    || verdict_annuel(self, &mut tx, etab, a.eleve_id, &t.usage_accord_code, pour).await != "autorise"
                {
                    return Err(conflit(
                        "aucune personne n'est désignée pour récupérer l'élève et aucune autorisation annuelle ne couvre son départ : maintenir le blocage ; la famille annonce de nouveau",
                    ));
                }
            }
        }
        let motif_refus = (nouveau == "refusee").then(|| {
            motif.clone().unwrap_or_else(|| "Désaccord entre les titulaires de l'autorité parentale : demande refusée par la direction".to_string())
        });
        sqlx::query(
            "UPDATE ent_internat_absence SET statut = $3, avant_blocage = NULL, blocage_effet_nuit = NULL, \
                    decide_par_user_id = $4, decide_le = $5, \
                    motif_refus_texte = COALESCE($6, motif_refus_texte), modifie_le = $5 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(nouveau)
        .bind(actor)
        .bind(maintenant)
        .bind(motif_refus.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_tx(&mut tx, etab, Cible::Absence(id), acte, "direction", Some(actor), maintenant, Some(motif_code), motif.as_deref(), None, None)
            .await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_DESACCORD_TRANCHE,
            AggregateKind::Internat,
            id,
            Some(a.eleve_id),
            json!({ "cible": "absence", "decision": b.decision, "statut": nouveau }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        if absence_en_vigueur(nouveau) {
            publier(
                &mut tx,
                etab,
                types::INTERNAT_ATTENDU_MODIFIE,
                AggregateKind::Internat,
                id,
                Some(a.eleve_id),
                json!({ "du": a.du_nuit.to_string(), "au": a.au_nuit.to_string(), "cause": "desaccord_tranche" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        let ligne = absence_tx(&mut tx, etab, id, false).await?;
        let opp = opposition_tx(&mut tx, etab, Cible::Absence(id)).await?;
        tx.commit().await.map_err(db)?;
        Ok(absence_dto(&ligne, None, false, opp))
    }

    /// Trancher un désaccord est l'acte de la direction (ou de l'administration), jamais du CPE seul.
    async fn direction_exiger(&self, etab: Uuid, actor: Uuid, role: &str, eleve: Uuid) -> Result<(), EntDbError> {
        self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], eleve).await?;
        if !matches!(Role::parse(role), Some(Role::Direction | Role::Admin)) {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }
}

// ───────────────────────── Sorties : exigences et transitions ─────────────────────────

/// (accord de la famille requis, décision du CPE requise) pour un type de sortie. L'élève majeur tient lieu de
/// titulaire (R-IT22) et son règlement ne passe pas par le CPE pour un type « titulaires et CPE ».
///
/// **Un mineur qui part seul exige toujours un accord de la famille** (le sien sur la sortie, ou l'autorisation annuelle
/// du socle 18 qui en tient lieu), quel que soit le décideur du type : le CPE seul ne décide jamais d'un départ seul.
fn exigences(t: &SortieTypeDto, eleve_majeur: bool, seul: bool) -> (bool, bool) {
    let famille = (t.autorisation != "aucune" && t.decideur != "cpe") || (!eleve_majeur && seul);
    let cpe = t.decideur == "cpe" || (t.decideur == "titulaires_et_cpe" && !eleve_majeur);
    (famille, cpe)
}

fn sortie_prete(l: &SortieLigne, t: &SortieTypeDto, eleve_majeur: bool) -> bool {
    let (famille, cpe) = exigences(t, eleve_majeur, l.seul);
    (!famille || l.accord_titulaire_le.is_some()) && (!cpe || l.decision_cpe_le.is_some())
}

/// Événements d'une sortie : la décision (statut, aucun texte) et l'attendu recalculé si l'effet sur la nuit
/// peut changer.
#[allow(clippy::too_many_arguments)]
async fn evenements_sortie_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Option<Uuid>,
    s: &SortieLigne,
    evt: &str,
    statut: &str,
    cause: &str,
    attendu: bool,
) -> Result<(), EntDbError> {
    publier(
        conn,
        etab,
        evt,
        AggregateKind::Internat,
        s.id,
        Some(s.eleve_id),
        json!({
            "type": s.type_code, "debut": s.debut.to_rfc3339(), "fin": s.fin_ajustee.to_rfc3339(),
            "statut": statut, "cause": cause
        }),
        actor,
        Causalite::default(),
    )
    .await?;
    if attendu {
        publier(
            conn,
            etab,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            s.id,
            Some(s.eleve_id),
            json!({ "debut": s.debut.to_rfc3339(), "fin": s.fin_ajustee.to_rfc3339(), "cause": "sortie" }),
            actor,
            Causalite::default(),
        )
        .await?;
    }
    Ok(())
}

/// Passe la sortie « demandee » à « autorisee » quand toutes les exigences sont réunies. Renvoie le statut.
async fn promouvoir_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &SortieLigne,
    t: &SortieTypeDto,
    eleve_majeur: bool,
    actor: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<String, EntDbError> {
    if s.statut != "demandee" || !sortie_prete(s, t, eleve_majeur) {
        return Ok(s.statut.clone());
    }
    sqlx::query("UPDATE ent_internat_sortie SET statut = 'autorisee', modifie_le = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(s.id)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    fait_tx(conn, etab, Cible::Sortie(s.id), "acceptation", "systeme", None, maintenant, None, None, None, None).await?;
    evenements_sortie_tx(conn, etab, actor, s, types::INTERNAT_SORTIE_DECIDEE, "autorisee", "exigences_reunies", true).await?;
    Ok("autorisee".to_string())
}

/// Erreur de la base sur le chevauchement de deux sorties du même élève.
fn erreur_sortie(e: sqlx::Error) -> EntDbError {
    if matches!(&e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23P01")) {
        conflit("l'élève a déjà une sortie à ces heures")
    } else {
        db(e)
    }
}

/// Nombre de nuits qu'une sortie de `debut` à `fin` couvre (commencée au plus tard à l'ancre de la nuit, retour après 6 h
/// du lendemain). Même règle que `ent_internat_attendu_calcule` (migration 2826).
async fn nuits_couvertes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
) -> Result<i64, EntDbError> {
    sqlx::query_scalar(
        "WITH j AS (SELECT (($2::timestamptz AT TIME ZONE ent_internat_fuseau($1))::date - 1) AS d0, \
                           (($3::timestamptz AT TIME ZONE ent_internat_fuseau($1))::date) AS d1) \
         SELECT count(*) FROM j, generate_series(0, j.d1 - j.d0) g(n) \
                         WHERE $2::timestamptz <= ent_internat_ancre_nuit($1, j.d0 + g.n) \
                           AND $3::timestamptz >= (((j.d0 + g.n + 1) + TIME '06:00') AT TIME ZONE ent_internat_fuseau($1))",
    )
    .bind(etab)
    .bind(debut)
    .bind(fin)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Bloque une sortie pour désaccord (le plus restrictif l'emporte, la direction tranche) : statut « bloquée », faits
/// d'opposition (avec la personne proposée, le cas échéant) et de blocage, événement d'alerte au CPE.
#[allow(clippy::too_many_arguments)]
async fn bloquer_sortie_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    qualite: &str,
    s: &SortieLigne,
    motif: Option<&str>,
    personne_proposee: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_internat_sortie SET statut = 'bloquee', avant_blocage = statut, modifie_le = $3 \
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(s.id)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    fait_tx(conn, etab, Cible::Sortie(s.id), "opposition", qualite, Some(actor), maintenant, Some("desaccord_titulaires"), motif, personne_proposee, None)
        .await?;
    fait_tx(conn, etab, Cible::Sortie(s.id), "blocage", "systeme", None, maintenant, Some("desaccord_titulaires"), None, None, None)
        .await?;
    evenements_sortie_tx(conn, etab, Some(actor), s, types::INTERNAT_ABSENCE_BLOQUEE, "bloquee", "opposition", s.statut == "autorisee").await
}

/// Blocage posé par le système (interdiction sans personne identifiée) : la direction tranche. **Gardé** : seule une sortie
/// encore « demandée », « autorisée » ou « suspendue » est bloquée (une sortie partie entre-temps ne fait pas échouer la
/// transaction) ; renvoie vrai si le blocage a eu lieu.
async fn bloquer_sortie_systeme_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &SortieLigne,
    motif: &str,
    maintenant: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let n = sqlx::query(
        "UPDATE ent_internat_sortie SET statut = 'bloquee', avant_blocage = statut, modifie_le = $3 \
          WHERE etablissement_id = $1 AND id = $2 AND statut IN ('demandee', 'autorisee', 'suspendue')",
    )
    .bind(etab)
    .bind(s.id)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?
    .rows_affected();
    if n == 0 {
        return Ok(false);
    }
    fait_tx(conn, etab, Cible::Sortie(s.id), "blocage", "systeme", None, maintenant, Some(motif), None, s.avec_personne_id, None).await?;
    evenements_sortie_tx(conn, etab, None, s, types::INTERNAT_ABSENCE_BLOQUEE, "bloquee", motif, s.statut == "autorisee").await?;
    Ok(true)
}

/// Blocage d'une absence par le système (interdiction de la personne, de l'auteur, sans personne identifiée, autorisation
/// annuelle retirée) : « bloquée », l'élève reste attendu (pas de départ constaté), le CPE est alerté. **Gardé** : seule
/// une absence encore en vigueur (« demandée », « annoncée », « acceptée ») sans départ constaté est bloquée ; renvoie
/// vrai si le blocage a eu lieu.
async fn bloquer_absence_systeme_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    ab: &AbsenceLigne,
    motif: &str,
    maintenant: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let n = sqlx::query(
        "UPDATE ent_internat_absence SET statut = 'bloquee', avant_blocage = statut, blocage_effet_nuit = NULL, \
                modifie_le = $3 \
          WHERE etablissement_id = $1 AND id = $2 AND statut IN ('demandee', 'annoncee', 'acceptee') \
            AND depart_effectif IS NULL",
    )
    .bind(etab)
    .bind(ab.id)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?
    .rows_affected();
    if n == 0 {
        return Ok(false);
    }
    fait_tx(conn, etab, Cible::Absence(ab.id), "blocage", "systeme", None, maintenant, Some(motif), None, ab.recupere_par_personne_id, None)
        .await?;
    publier(
        conn,
        etab,
        types::INTERNAT_ABSENCE_BLOQUEE,
        AggregateKind::Internat,
        ab.id,
        Some(ab.eleve_id),
        json!({ "du": ab.du_nuit.to_string(), "au": ab.au_nuit.to_string(), "cause": motif, "nuit_en_cours": false }),
        None,
        Causalite::default(),
    )
    .await?;
    Ok(true)
}

/// Moyen 1 : un titulaire qui change la personne désignée par **un autre titulaire** s'oppose à lui : le plus
/// restrictif l'emporte (l'ancienne désignation reste, la sortie est bloquée) jusqu'à la décision de la direction.
/// Vrai si `nouvelle` remplace une personne désignée par un autre titulaire (ou l'élève majeur).
async fn changement_vaut_opposition_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &SortieLigne,
    actor: Uuid,
    nouvelle: Option<Uuid>,
    devient_seul: bool,
) -> Result<bool, EntDbError> {
    let Some(ancienne) = s.avec_personne_id else { return Ok(false) };
    if !devient_seul {
        match nouvelle {
            Some(n) if meme_personne_tx(conn, etab, ancienne, n).await? => return Ok(false),
            // F-C : effacer la personne désignée par l'autre titulaire vaut changement, donc opposition.
            _ => {}
        }
    }
    let auteur: Option<(Option<Uuid>, String)> = sqlx::query_as(
        "SELECT par_user_id, qualite FROM ent_internat_i3_fait WHERE etablissement_id = $1 AND sortie_id = $2 \
            AND acte IN ('demande', 'accord_titulaire', 'reprise') AND personne_id IS NOT NULL \
          ORDER BY a DESC, cree_le DESC LIMIT 1",
    )
    .bind(etab)
    .bind(s.id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(matches!(auteur, Some((Some(u), q)) if u != actor && matches!(q.as_str(), "titulaire" | "eleve_majeur")))
}

impl PgEnt {
    async fn sortie_vers_dto(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        l: &SortieLigne,
        nom: Option<(String, String)>,
        protege: bool,
    ) -> Result<SortieDto, EntDbError> {
        let opp = opposition_tx(conn, etab, Cible::Sortie(l.id)).await?;
        let motif = motif_refus_sortie_tx(conn, etab, l.id).await?;
        Ok(sortie_dto(l, nom, protege, opp, motif))
    }

    /// Acteur d'une sortie : famille de l'élève, ou personnel (`InternatSortiesDecider`, dans sa portée).
    async fn acteur_sortie_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        jour: NaiveDate,
    ) -> Result<(Qualite, Famille), EntDbError> {
        match self.famille_tx(conn, etab, actor, role, eleve, jour).await? {
            Some(f) => Ok((f.qualite, f)),
            None => {
                self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], eleve).await?;
                Ok((Qualite::Personnel, Famille { qualite: Qualite::Personnel, peut_autoriser: false }))
            }
        }
    }

    /// INT-154, INT-155, INT-156, INT-164, INT-169 : demande (ou annonce) d'une sortie. Voir l'en-tête du module.
    pub async fn demander_sortie_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &SortieDemandeDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        if b.fin_prevue <= b.debut || (b.fin_prevue - b.debut) < Duration::minutes(SORTIE_MIN_MINUTES) {
            return Err(refus("une sortie dure 15 minutes au moins"));
        }
        if b.fin_prevue <= maintenant {
            return Err(refus("cette sortie est déjà terminée"));
        }
        if (b.debut - maintenant).num_days() > HORIZON_SORTIE_JOURS {
            return Err(refus("une sortie s'enregistre un an à l'avance au plus"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let (qualite, famille) = self.acteur_sortie_tx(&mut tx, etab, actor, role, b.eleve_id, jour).await?;
        Self::eleve_existe(&mut tx, etab, b.eleve_id).await?;
        let t = type_tx(&mut tx, etab, &b.type_code).await?;
        if !t.actif {
            return Err(refus("ce type de sortie est désactivé dans l'établissement"));
        }
        if (b.fin_prevue - b.debut) > Duration::hours(i64::from(t.duree_max_heures)) {
            return Err(refus("cette sortie dépasse la durée maximale de son type"));
        }
        match qualite {
            Qualite::Titulaire if !famille.peut_autoriser => {
                return Err(refus("votre lien avec l'élève ne vous permet pas d'autoriser ou de demander une sortie"))
            }
            Qualite::Eleve if !t.eleve_peut_demander => {
                return Err(refus("l'élève ne demande pas lui-même ce type de sortie : un titulaire ou le CPE le fait"))
            }
            _ => {}
        }
        let jour_debut = jour_local_tx(&mut tx, etab, b.debut).await?;
        let majeur = eleve_majeur_tx(&mut tx, etab, b.eleve_id, jour_debut).await?;
        // Qui récupère : seul, correspondant ou personne autorisée (R-IT23).
        let mut avec = b.avec_personne_id;
        let mut correspondant = b.correspondant_id;
        if b.seul {
            if !t.seul_permis {
                return Err(refus("ce type de sortie ne se fait pas seul"));
            }
            if avec.is_some() || correspondant.is_some() {
                return Err(refus("une sortie seul n'a ni personne ni correspondant"));
            }
        }
        if t.correspondant_requis && correspondant.is_none() {
            return Err(refus("ce type de sortie se fait chez un correspondant désigné"));
        }
        if let Some(c) = correspondant {
            let ligne = sqlx::query(
                "SELECT personne_id, statut, du, au, lundi, mardi, mercredi, jeudi, vendredi, samedi, dimanche \
                   FROM ent_internat_correspondant WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(c)
            .bind(b.eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(|| refus("correspondant inconnu pour cet élève"))?;
            let statut: String = ligne.get("statut");
            if statut != "valide" {
                return Err(refus("ce correspondant est suspendu ou n'est plus désigné"));
            }
            let du: NaiveDate = ligne.get("du");
            let au: Option<NaiveDate> = ligne.get("au");
            if jour_debut < du || au.is_some_and(|a| jour_debut > a) {
                return Err(refus("ce correspondant n'est pas désigné à cette date"));
            }
            let accueille: bool = match jour_debut.weekday() {
                chrono::Weekday::Mon => ligne.get("lundi"),
                chrono::Weekday::Tue => ligne.get("mardi"),
                chrono::Weekday::Wed => ligne.get("mercredi"),
                chrono::Weekday::Thu => ligne.get("jeudi"),
                chrono::Weekday::Fri => ligne.get("vendredi"),
                chrono::Weekday::Sat => ligne.get("samedi"),
                chrono::Weekday::Sun => ligne.get("dimanche"),
            };
            if !accueille {
                return Err(refus("le correspondant n'accueille pas l'élève ce jour-là"));
            }
            avec = Some(ligne.get("personne_id"));
        }
        if let Some(p) = avec {
            let r = remise_possible_tx(&mut tx, etab, b.eleve_id, p, jour_debut).await?;
            if !matches!(r, Remise::Autorisee(_)) {
                return Err(refus("cette personne n'est pas autorisée à récupérer l'élève"));
            }
        }
        // Accord annuel du socle 18 : seul un verdict « autorise » vaut accord (jamais par défaut).
        let accord_annuel = if matches!(t.autorisation.as_str(), "annuelle" | "annuelle_ou_ponctuelle")
            || (b.seul && !majeur)
        {
            let v = verdict_annuel(self, &mut tx, etab, b.eleve_id, &t.usage_accord_code, jour_debut).await;
            v == "autorise"
        } else {
            false
        };
        if !majeur && !accord_annuel {
            if t.autorisation == "annuelle" {
                return Err(refus(
                    "aucune autorisation annuelle valide pour ce type de sortie (socle 18) : la famille la signe d'abord",
                ));
            }
            if t.autorisation == "aucune" && b.seul {
                return Err(refus(
                    "un départ seul d'un mineur exige l'autorisation annuelle de la famille (socle 18)",
                ));
            }
        }
        // Un désaccord connu entre titulaires bloque la demande et la renvoie au CPE (R-IT20).
        let bloquee = qualite != Qualite::Personnel && desaccord_connu_tx(&mut tx, etab, b.eleve_id).await?;
        let accord_propre = matches!(qualite, Qualite::Titulaire | Qualite::EleveMajeur);
        let accord_le = (accord_propre || accord_annuel).then_some(maintenant);
        let accord_user = accord_propre.then_some(actor);
        let (_, cpe_requis) = exigences(&t, majeur, b.seul);
        let decision_cpe_le = (qualite == Qualite::Personnel && cpe_requis).then_some(maintenant);
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_sortie \
             (id, etablissement_id, eleve_id, type_code, debut, fin_prevue, fin_ajustee, seul, avec_personne_id, \
              correspondant_id, demande_par_user_id, demande_qualite, statut, avant_blocage, accord_titulaire_le, \
              accord_titulaire_user_id, decision_cpe_le, decision_cpe_user_id, par_autorisation_annuelle, cree_le, modifie_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$19)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(&b.type_code)
        .bind(b.debut)
        .bind(b.fin_prevue)
        .bind(b.seul)
        .bind(avec)
        .bind(correspondant.take())
        .bind(actor)
        .bind(qualite.code())
        .bind(if bloquee { "bloquee" } else { "demandee" })
        .bind(bloquee.then_some("demandee"))
        .bind(accord_le)
        .bind(accord_user)
        .bind(decision_cpe_le)
        .bind(decision_cpe_le.map(|_| actor))
        .bind(accord_annuel)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(erreur_sortie)?;
        let qualite_fait = if qualite == Qualite::Personnel { "personnel" } else { qualite.code() };
        fait_tx(&mut tx, etab, Cible::Sortie(id), "demande", qualite_fait, Some(actor), maintenant, None, None, avec, Some(b.fin_prevue))
            .await?;
        if accord_propre {
            fait_tx(&mut tx, etab, Cible::Sortie(id), "accord_titulaire", qualite.code(), Some(actor), maintenant, None, None, None, None)
                .await?;
        } else if accord_annuel {
            fait_tx(&mut tx, etab, Cible::Sortie(id), "accord_titulaire", "systeme", None, maintenant, None, None, None, None)
                .await?;
        }
        if decision_cpe_le.is_some() {
            fait_tx(&mut tx, etab, Cible::Sortie(id), "acceptation", "cpe", Some(actor), maintenant, None, None, None, None).await?;
        }
        if bloquee {
            fait_tx(&mut tx, etab, Cible::Sortie(id), "blocage", "systeme", None, maintenant, Some("desaccord_connu"), None, None, None)
                .await?;
        }
        let mut ligne = sortie_tx(&mut tx, etab, id, true).await?;
        evenements_sortie_tx(
            &mut tx,
            etab,
            Some(actor),
            &ligne,
            types::INTERNAT_SORTIE_DEMANDEE,
            &ligne.statut,
            qualite_fait,
            false,
        )
        .await?;
        if bloquee {
            publier(
                &mut tx,
                etab,
                types::INTERNAT_ABSENCE_BLOQUEE,
                AggregateKind::Internat,
                id,
                Some(b.eleve_id),
                json!({ "objet": "sortie", "cause": "desaccord_connu" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        } else {
            promouvoir_tx(&mut tx, etab, &ligne, &t, majeur, Some(actor), maintenant).await?;
            ligne = sortie_tx(&mut tx, etab, id, false).await?;
        }
        if qualite == Qualite::EleveMajeur {
            self.informer_parents_du_majeur_tx(&mut tx, etab, actor, b.eleve_id, "sortie", id).await?;
        }
        let dto = self.sortie_vers_dto(&mut tx, etab, &ligne, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Réponse d'un titulaire (ou de l'élève majeur) à une demande de sortie : accord, ou refus. Le plus
    /// restrictif l'emporte : un refus après l'accord d'un autre titulaire bloque la sortie (direction).
    pub async fn repondre_sortie_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &AccordTitulaireDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = sortie_tx(&mut tx, etab, id, true).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let famille = self.famille_tx(&mut tx, etab, actor, role, s.eleve_id, jour).await?;
        let qualite = match famille {
            Some(f @ Famille { qualite: Qualite::Titulaire | Qualite::EleveMajeur, .. }) if f.peut_autoriser => f.qualite,
            Some(Famille { qualite: Qualite::Titulaire, .. }) => {
                return Err(refus("votre lien avec l'élève ne vous permet pas d'autoriser une sortie"))
            }
            _ => return Err(EntDbError::Forbidden),
        };
        let t = type_tx(&mut tx, etab, &s.type_code).await?;
        let jour_debut = jour_local_tx(&mut tx, etab, s.debut).await?;
        let majeur = eleve_majeur_tx(&mut tx, etab, s.eleve_id, jour_debut).await?;
        if s.fin_ajustee <= maintenant {
            return Err(conflit("cette sortie est terminée"));
        }
        if b.accord {
            if s.statut != "demandee" {
                return Err(conflit("cette sortie n'attend plus de réponse"));
            }
            if s.accord_titulaire_user_id.is_some_and(|u| u != actor) || s.accord_titulaire_le.is_some() {
                return Err(conflit("l'accord de la famille est déjà donné"));
            }
            if let Some(p) = b.avec_personne_id {
                if s.seul {
                    return Err(refus("cette sortie se fait seul : aucune personne à désigner"));
                }
                let jd = jour_local_tx(&mut tx, etab, s.debut).await?;
                if !matches!(remise_possible_tx(&mut tx, etab, s.eleve_id, p, jd).await?, Remise::Autorisee(_)) {
                    return Err(refus("cette personne n'est pas autorisée à récupérer l'élève"));
                }
            }
            // Moyen 1 : désigner une AUTRE personne que celle qu'un autre titulaire a désignée vaut opposition.
            let opposition = match b.avec_personne_id {
                Some(p) if qualite == Qualite::Titulaire => {
                    changement_vaut_opposition_tx(&mut tx, etab, &s, actor, Some(p), false).await?
                }
                _ => false,
            };
            if opposition {
                bloquer_sortie_tx(&mut tx, etab, actor, qualite.code(), &s, motif.as_deref(), b.avec_personne_id, maintenant).await?;
            } else {
                sqlx::query(
                    "UPDATE ent_internat_sortie SET accord_titulaire_le = $3, accord_titulaire_user_id = $4, \
                            avec_personne_id = COALESCE($5, avec_personne_id), modifie_le = $3 \
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(maintenant)
                .bind(actor)
                .bind(b.avec_personne_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                fait_tx(&mut tx, etab, Cible::Sortie(id), "accord_titulaire", qualite.code(), Some(actor), maintenant, None, motif.as_deref(), b.avec_personne_id, None)
                    .await?;
                let l = sortie_tx(&mut tx, etab, id, true).await?;
                promouvoir_tx(&mut tx, etab, &l, &t, majeur, Some(actor), maintenant).await?;
            }
        } else {
            if !matches!(s.statut.as_str(), "demandee" | "autorisee") {
                return Err(conflit("cette sortie n'attend plus de réponse"));
            }
            let autre_accord = s.accord_titulaire_user_id.is_some_and(|u| u != actor);
            let (nouveau, acte, motif_code) = if autre_accord || s.statut == "autorisee" {
                ("bloquee", "opposition", Some("desaccord_titulaires"))
            } else {
                ("refusee", "refus", None)
            };
            if nouveau == "refusee" && motif.is_none() {
                return Err(refus("un refus s'accompagne d'un motif court lu par la famille"));
            }
            sqlx::query(
                "UPDATE ent_internat_sortie SET statut = $3, avant_blocage = CASE WHEN $3 = 'bloquee' THEN statut END, \
                        modifie_le = $4 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(nouveau)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            fait_tx(&mut tx, etab, Cible::Sortie(id), acte, qualite.code(), Some(actor), maintenant, motif_code, motif.as_deref(), None, None)
                .await?;
            if nouveau == "bloquee" {
                fait_tx(&mut tx, etab, Cible::Sortie(id), "blocage", "systeme", None, maintenant, Some("desaccord_titulaires"), None, None, None)
                    .await?;
            }
            let evt = if nouveau == "bloquee" { types::INTERNAT_ABSENCE_BLOQUEE } else { types::INTERNAT_SORTIE_DECIDEE };
            evenements_sortie_tx(&mut tx, etab, Some(actor), &s, evt, nouveau, "refus_titulaire", s.statut == "autorisee").await?;
        }
        let l = sortie_tx(&mut tx, etab, id, false).await?;
        let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-155 : décision du CPE sur une sortie : `autoriser` (si la famille a répondu, la sortie est autorisée)
    /// ou `refuser` (motif lu par la famille).
    pub async fn decider_sortie_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &SortieDecisionDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = sortie_tx(&mut tx, etab, id, true).await?;
        self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], s.eleve_id).await?;
        let t = type_tx(&mut tx, etab, &s.type_code).await?;
        if matches!(b.decision.as_str(), "accepter_retard" | "refuser_retard") {
            // E-2 : le CPE décide d'un retard qui ferait couvrir la nuit à une sortie qui ne la couvrait pas.
            let Some(fin) = s.retard_demande_fin else {
                return Err(conflit("aucun retard n'attend de décision pour cette sortie"));
            };
            if !matches!(s.statut.as_str(), "autorisee" | "partie") {
                return Err(conflit("cette sortie n'est plus en cours : le retard demandé ne s'applique plus"));
            }
            if b.decision == "accepter_retard" {
                sqlx::query(
                    "UPDATE ent_internat_sortie SET fin_ajustee = GREATEST(fin_ajustee, $3), retard_demande_fin = NULL, \
                            retard_demande_par_user_id = NULL, modifie_le = $4 WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(fin)
                .bind(maintenant)
                .execute(&mut *tx)
                .await
                .map_err(erreur_sortie)?;
                fait_tx(&mut tx, etab, Cible::Sortie(id), "retard_annonce", "cpe", Some(actor), maintenant, None, motif.as_deref(), None, Some(fin))
                    .await?;
                publier(
                    &mut tx,
                    etab,
                    types::INTERNAT_RETARD_ANNONCE,
                    AggregateKind::Internat,
                    id,
                    Some(s.eleve_id),
                    json!({ "nouveau_retour": fin.to_rfc3339(), "decision": "acceptee" }),
                    Some(actor),
                    Causalite::default(),
                )
                .await?;
                publier(
                    &mut tx,
                    etab,
                    types::INTERNAT_ATTENDU_MODIFIE,
                    AggregateKind::Internat,
                    id,
                    Some(s.eleve_id),
                    json!({ "debut": s.debut.to_rfc3339(), "fin": fin.to_rfc3339(), "cause": "retard_accepte" }),
                    Some(actor),
                    Causalite::default(),
                )
                .await?;
            } else {
                sqlx::query(
                    "UPDATE ent_internat_sortie SET retard_demande_fin = NULL, retard_demande_par_user_id = NULL, modifie_le = $3 \
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(maintenant)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                fait_tx(&mut tx, etab, Cible::Sortie(id), "retard_refuse", "cpe", Some(actor), maintenant, None, motif.as_deref(), None, Some(fin))
                    .await?;
            }
            let l = sortie_tx(&mut tx, etab, id, false).await?;
            let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
            tx.commit().await.map_err(db)?;
            return Ok(dto);
        }
        if s.statut == "bloquee" {
            return Err(conflit("cette sortie est bloquée par un désaccord : la direction tranche"));
        }
        if s.statut != "demandee" {
            return Err(conflit("cette sortie n'attend plus de décision"));
        }
        let jour_debut = jour_local_tx(&mut tx, etab, s.debut).await?;
        let majeur = eleve_majeur_tx(&mut tx, etab, s.eleve_id, jour_debut).await?;
        match b.decision.as_str() {
            "autoriser" => {
                // Passe 4 (R3) : aucune décision du personnel sur un objet qui tombe (voir `decider_absence_a`).
                let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
                if actes_qui_tombent_tx(&mut tx, etab, &[Objet::Sortie(&s)], jour).await?.contains_key(&s.id) {
                    return Err(conflit(
                        "cette sortie repose sur un acte (demande, accord, personne désignée ou autorisation annuelle) posé par une personne qui n'a plus le droit d'agir pour cet élève : l'autoriser est impossible ; la refuser, ou la faire redemander par un titulaire",
                    ));
                }
                sqlx::query(
                    "UPDATE ent_internat_sortie SET decision_cpe_le = $3, decision_cpe_user_id = $4, modifie_le = $3 \
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(maintenant)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                fait_tx(&mut tx, etab, Cible::Sortie(id), "acceptation", "cpe", Some(actor), maintenant, None, motif.as_deref(), None, None)
                    .await?;
                let l = sortie_tx(&mut tx, etab, id, true).await?;
                let nouveau = promouvoir_tx(&mut tx, etab, &l, &t, majeur, Some(actor), maintenant).await?;
                if nouveau == "demandee" {
                    evenements_sortie_tx(&mut tx, etab, Some(actor), &l, types::INTERNAT_SORTIE_DECIDEE, "demandee", "cpe_accepte", false).await?;
                }
            }
            "refuser" => {
                let motif = motif.ok_or_else(|| refus("un refus s'accompagne d'un motif lu par la famille"))?;
                sqlx::query("UPDATE ent_internat_sortie SET statut = 'refusee', modifie_le = $3 WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(id)
                    .bind(maintenant)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                fait_tx(&mut tx, etab, Cible::Sortie(id), "refus", "cpe", Some(actor), maintenant, Some("reglement"), Some(&motif), None, None)
                    .await?;
                evenements_sortie_tx(&mut tx, etab, Some(actor), &s, types::INTERNAT_SORTIE_DECIDEE, "refusee", "cpe_refuse", false).await?;
            }
            _ => return Err(refus("décision inconnue (autoriser, refuser)")),
        }
        let l = sortie_tx(&mut tx, etab, id, false).await?;
        let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Annulation d'une sortie (tant qu'elle n'est pas partie) : par son demandeur, un titulaire, l'élève majeur
    /// ou le CPE. L'élève redevient attendu si la sortie couvrait la nuit.
    pub async fn annuler_sortie_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = sortie_tx(&mut tx, etab, id, true).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let (qualite, _) = match self.famille_tx(&mut tx, etab, actor, role, s.eleve_id, jour).await? {
            Some(f) if f.qualite != Qualite::Eleve || s.demande_par_user_id == Some(actor) => (f.qualite, f),
            Some(_) => return Err(EntDbError::Forbidden),
            None => {
                self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], s.eleve_id).await?;
                (Qualite::Personnel, Famille { qualite: Qualite::Personnel, peut_autoriser: false })
            }
        };
        if !matches!(s.statut.as_str(), "demandee" | "autorisee" | "suspendue" | "bloquee") {
            return Err(conflit("cette sortie ne peut plus être annulée (partie, terminée ou déjà décidée)"));
        }
        sqlx::query(
            "UPDATE ent_internat_sortie SET statut = 'annulee', avant_blocage = NULL, modifie_le = $3 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let q = if qualite == Qualite::Personnel { "cpe" } else { qualite.code() };
        fait_tx(&mut tx, etab, Cible::Sortie(id), "annulation", q, Some(actor), maintenant, None, None, None, None).await?;
        evenements_sortie_tx(&mut tx, etab, Some(actor), &s, types::INTERNAT_SORTIE_DECIDEE, "annulee", "annulation", s.statut == "autorisee").await?;
        let l = sortie_tx(&mut tx, etab, id, false).await?;
        let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// R-IT20 : un titulaire s'oppose à une sortie demandée ou accordée par un autre. Le plus restrictif l'emporte :
    /// la sortie est **bloquée** (l'élève redevient attendu) jusqu'à la décision de la direction.
    pub async fn opposer_sortie_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &OppositionDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = sortie_tx(&mut tx, etab, id, true).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        match self.famille_tx(&mut tx, etab, actor, role, s.eleve_id, jour).await? {
            Some(Famille { qualite: Qualite::Titulaire, .. }) => {}
            _ => return Err(EntDbError::Forbidden),
        }
        if s.demande_par_user_id == Some(actor) || s.accord_titulaire_user_id == Some(actor) {
            return Err(refus("vous avez demandé ou accordé cette sortie : l'annuler plutôt que s'y opposer"));
        }
        if !matches!(s.statut.as_str(), "demandee" | "autorisee" | "suspendue") {
            return Err(conflit("cette sortie n'est plus en cours : aucune opposition possible"));
        }
        if s.fin_ajustee <= maintenant {
            return Err(conflit("cette sortie est terminée"));
        }
        bloquer_sortie_tx(&mut tx, etab, actor, "titulaire", &s, motif.as_deref(), None, maintenant).await?;
        let l = sortie_tx(&mut tx, etab, id, false).await?;
        let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// La direction tranche le désaccord sur une sortie (`lever` ou `maintenir`), avec trace.
    pub async fn trancher_sortie_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &TrancheDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = sortie_tx(&mut tx, etab, id, true).await?;
        self.direction_exiger(etab, actor, role, s.eleve_id).await?;
        if s.statut != "bloquee" {
            return Err(conflit("cette sortie n'est pas bloquée par un désaccord"));
        }
        let avant = s.avant_blocage.clone().ok_or_else(|| conflit("état antérieur inconnu"))?;
        let (nouveau, acte) = match b.decision.as_str() {
            "lever" => (avant.as_str(), "tranchee_levee"),
            "maintenir" => ("refusee", "tranchee_maintenue"),
            _ => return Err(refus("décision inconnue (lever, maintenir)")),
        };
        let mut motif_code = "desaccord_titulaires";
        if nouveau != "refusee" {
            let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
            motif_code = motif_de_levee_tx(&mut tx, etab, Cible::Sortie(id), s.eleve_id, jour, motif.as_deref()).await?;
        }
        let motif_fait = if nouveau == "refusee" {
            Some(motif.clone().unwrap_or_else(|| {
                "Désaccord entre les titulaires de l'autorité parentale : sortie refusée par la direction".to_string()
            }))
        } else {
            motif.clone()
        };
        sqlx::query(
            "UPDATE ent_internat_sortie SET statut = $3, avant_blocage = NULL, modifie_le = $4 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(nouveau)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(erreur_sortie)?;
        fait_tx(&mut tx, etab, Cible::Sortie(id), acte, "direction", Some(actor), maintenant, Some(motif_code), motif_fait.as_deref(), None, None)
            .await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_DESACCORD_TRANCHE,
            AggregateKind::Internat,
            id,
            Some(s.eleve_id),
            json!({ "cible": "sortie", "decision": b.decision, "statut": nouveau }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        if nouveau == "autorisee" {
            publier(
                &mut tx,
                etab,
                types::INTERNAT_ATTENDU_MODIFIE,
                AggregateKind::Internat,
                id,
                Some(s.eleve_id),
                json!({ "debut": s.debut.to_rfc3339(), "fin": s.fin_ajustee.to_rfc3339(), "cause": "desaccord_tranche" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        let l = sortie_tx(&mut tx, etab, id, false).await?;
        let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Précise qui récupère l'élève (ou départ seul) tant que la sortie n'est pas partie : titulaire, élève majeur
    /// ou CPE.
    pub async fn designer_personne_sortie_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &PersonneSortieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = sortie_tx(&mut tx, etab, id, true).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let (qualite, famille) = self.acteur_sortie_tx(&mut tx, etab, actor, role, s.eleve_id, jour).await?;
        if qualite == Qualite::Eleve || (qualite == Qualite::Titulaire && !famille.peut_autoriser) {
            return Err(EntDbError::Forbidden);
        }
        if !matches!(s.statut.as_str(), "demandee" | "autorisee" | "suspendue") || s.correspondant_id.is_some() {
            return Err(conflit("on ne change plus la personne de cette sortie (partie, terminée ou chez un correspondant)"));
        }
        let t = type_tx(&mut tx, etab, &s.type_code).await?;
        let jour_debut = jour_local_tx(&mut tx, etab, s.debut).await?;
        let majeur = eleve_majeur_tx(&mut tx, etab, s.eleve_id, jour_debut).await?;
        let mut annuel = false;
        if b.seul {
            if !t.seul_permis || b.avec_personne_id.is_some() {
                return Err(refus("ce type de sortie ne se fait pas seul, ou une personne est aussi désignée"));
            }
            if !majeur && t.autorisation != "ponctuelle" {
                let v = verdict_annuel(self, &mut tx, etab, s.eleve_id, &t.usage_accord_code, jour_debut).await;
                if v != "autorise" {
                    return Err(refus("un départ seul d'un mineur exige l'autorisation annuelle de la famille (socle 18)"));
                }
                annuel = true;
            }
        }
        if let Some(p) = b.avec_personne_id {
            let jd = jour_local_tx(&mut tx, etab, s.debut).await?;
            if !matches!(remise_possible_tx(&mut tx, etab, s.eleve_id, p, jd).await?, Remise::Autorisee(_)) {
                return Err(refus("cette personne n'est pas autorisée à récupérer l'élève"));
            }
        }
        // Moyen 1 : changer la personne désignée par un autre titulaire vaut opposition (le plus restrictif l'emporte :
        // l'ancienne désignation reste en place, la sortie est bloquée jusqu'à la décision de la direction).
        if qualite == Qualite::Titulaire
            && changement_vaut_opposition_tx(&mut tx, etab, &s, actor, b.avec_personne_id, b.seul).await?
        {
            bloquer_sortie_tx(&mut tx, etab, actor, qualite.code(), &s, None, b.avec_personne_id, maintenant).await?;
            let l = sortie_tx(&mut tx, etab, id, false).await?;
            let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
            tx.commit().await.map_err(db)?;
            return Ok(dto);
        }
        sqlx::query(
            "UPDATE ent_internat_sortie SET seul = $3, avec_personne_id = $4, modifie_le = $5 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(b.seul)
        .bind(b.avec_personne_id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let q = if qualite == Qualite::Personnel { "cpe" } else { qualite.code() };
        fait_tx(&mut tx, etab, Cible::Sortie(id), "reprise", q, Some(actor), maintenant, None, None, b.avec_personne_id, None).await?;
        // Moyen 2 : passer un MINEUR en départ « seul » remet à zéro l'accord de la famille donné pour un départ
        // accompagné : il ne vaut pas pour un départ sans personne. Seule l'autorisation annuelle (socle 18, relue
        // à l'instant) le remplace ; à défaut, la sortie retourne à la demande et la famille répond de nouveau.
        if b.seul && !s.seul && !majeur {
            sqlx::query(
                "UPDATE ent_internat_sortie SET accord_titulaire_le = $3, accord_titulaire_user_id = NULL, \
                        par_autorisation_annuelle = $4, modifie_le = $5 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(annuel.then_some(maintenant))
            .bind(annuel)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            fait_tx(&mut tx, etab, Cible::Sortie(id), "accord_remis_a_zero", "systeme", None, maintenant, None, None, None, None).await?;
            let l = sortie_tx(&mut tx, etab, id, true).await?;
            if matches!(l.statut.as_str(), "autorisee" | "suspendue") && !sortie_prete(&l, &t, majeur) {
                sqlx::query("UPDATE ent_internat_sortie SET statut = 'demandee', modifie_le = $3 WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(id)
                    .bind(maintenant)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                evenements_sortie_tx(&mut tx, etab, Some(actor), &l, types::INTERNAT_SORTIE_DECIDEE, "demandee", "accord_remis_a_zero", false).await?;
            } else {
                promouvoir_tx(&mut tx, etab, &l, &t, majeur, Some(actor), maintenant).await?;
            }
        }
        let l = sortie_tx(&mut tx, etab, id, false).await?;
        let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// INT-158 : retard de retour annoncé (train supprimé, grève) par la famille, l'élève majeur ou le personnel :
    /// le retour attendu est décalé, aucun cas ne s'ouvre tant que le délai annoncé court (CIT-23).
    pub async fn annoncer_retard_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &RetardAnnonceDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        let motif_code = b.motif_code.as_deref().unwrap_or("autre");
        if !MOTIFS_RETARD.contains(&motif_code) {
            return Err(refus("motif de retard inconnu (train, autre)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = sortie_tx(&mut tx, etab, id, true).await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        // E-2 : seul un titulaire dont le lien permet d'autoriser les sorties, l'élève majeur ou le CPE annonce un
        // retard (ni un surveillant, ni un titulaire sans ce droit, ni l'élève mineur).
        let qualite = match self.famille_tx(&mut tx, etab, actor, role, s.eleve_id, jour).await? {
            Some(Famille { qualite: Qualite::Titulaire, peut_autoriser: true }) => Qualite::Titulaire,
            Some(Famille { qualite: Qualite::EleveMajeur, .. }) => Qualite::EleveMajeur,
            Some(Famille { qualite: Qualite::Titulaire, .. }) => {
                return Err(refus("votre lien avec l'élève ne vous permet pas d'annoncer un retard de retour"))
            }
            Some(_) => return Err(EntDbError::Forbidden),
            None => {
                self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], s.eleve_id).await?;
                Qualite::Personnel
            }
        };
        if !matches!(s.statut.as_str(), "autorisee" | "partie") {
            return Err(conflit("un retard s'annonce pour une sortie autorisée ou en cours"));
        }
        if b.nouveau_retour <= s.fin_ajustee {
            return Err(refus("le nouveau retour doit être postérieur au retour attendu"));
        }
        if b.nouveau_retour > s.fin_prevue + Duration::hours(RETARD_MAX_HEURES) {
            return Err(refus("un retard décale le retour de 48 heures au plus : au-delà, joindre le CPE"));
        }
        // Plafond : la durée maximale du type, plus une marge de 6 heures (train supprimé, grève).
        let t = type_tx(&mut tx, etab, &s.type_code).await?;
        if b.nouveau_retour > s.debut + Duration::hours(i64::from(t.duree_max_heures) + RETARD_MARGE_HEURES) {
            return Err(refus(
                "ce retard dépasse la durée maximale du type de sortie (marge de 6 heures comprise) : joindre le CPE",
            ));
        }
        // Un retard qui fait couvrir une nuit de plus à la sortie : c'est une demande au CPE, sans
        // effet (le retour attendu ne bouge pas) tant qu'il n'a pas décidé. Le CPE, lui, décide en l'annonçant.
        // F-A : on compare le NOMBRE de nuits couvertes avant et après : un retard qui en ajoute (même si la sortie en
        // couvrait déjà une) part au CPE.
        let couvrait = nuits_couvertes_tx(&mut tx, etab, s.debut, s.fin_ajustee).await?;
        let couvrirait = nuits_couvertes_tx(&mut tx, etab, s.debut, b.nouveau_retour).await?;
        if qualite != Qualite::Personnel && couvrirait > couvrait {
            if s.retard_demande_fin.is_some() {
                return Err(conflit("une demande de retard attend déjà la décision du CPE"));
            }
            sqlx::query(
                "UPDATE ent_internat_sortie SET retard_demande_fin = $3, retard_demande_par_user_id = $4, modifie_le = $5 \
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(b.nouveau_retour)
            .bind(actor)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            fait_tx(&mut tx, etab, Cible::Sortie(id), "retard_demande", qualite.code(), Some(actor), maintenant, Some(motif_code), None, None, Some(b.nouveau_retour))
                .await?;
            publier(
                &mut tx,
                etab,
                types::INTERNAT_RETARD_ANNONCE,
                AggregateKind::Internat,
                id,
                Some(s.eleve_id),
                json!({ "nouveau_retour": b.nouveau_retour.to_rfc3339(), "motif_code": motif_code, "en_attente": true }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
            let l = sortie_tx(&mut tx, etab, id, false).await?;
            let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
            tx.commit().await.map_err(db)?;
            return Ok(dto);
        }
        sqlx::query(
            "UPDATE ent_internat_sortie SET fin_ajustee = $3, retard_demande_fin = NULL, retard_demande_par_user_id = NULL, \
                    modifie_le = $4 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(b.nouveau_retour)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(erreur_sortie)?;
        let q = if qualite == Qualite::Personnel { "personnel" } else { qualite.code() };
        fait_tx(&mut tx, etab, Cible::Sortie(id), "retard_annonce", q, Some(actor), maintenant, Some(motif_code), None, None, Some(b.nouveau_retour))
            .await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_RETARD_ANNONCE,
            AggregateKind::Internat,
            id,
            Some(s.eleve_id),
            json!({ "nouveau_retour": b.nouveau_retour.to_rfc3339(), "motif_code": motif_code }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            id,
            Some(s.eleve_id),
            json!({ "debut": s.debut.to_rfc3339(), "fin": b.nouveau_retour.to_rfc3339(), "cause": "retard_annonce" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let l = sortie_tx(&mut tx, etab, id, false).await?;
        let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }
}

// ───────────────────────── Sorties : lectures, contrôle de départ et de retour ─────────────────────────

/// Un cas de cette nature existe-t-il déjà (ouvert **ou clos**) pour l'élève et la nuit ? Un cas clos n'est pas rouvert.
async fn cas_existe_tx(conn: &mut PgConnection, etab: Uuid, eleve: Uuid, nuit: NaiveDate, nature: &str) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_internat_cas \
          WHERE etablissement_id = $1 AND eleve_id = $2 AND nuit = $3 AND nature = $4)",
    )
    .bind(etab)
    .bind(eleve)
    .bind(nuit)
    .bind(nature)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// Le départ est constaté : le cas « non récupéré » ouvert pour l'élève se ferme (récupéré par sa famille, à l'heure du
/// geste). Passe 3 (point 3).
async fn clore_non_recupere_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    eleve: Uuid,
    a: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let clos: Vec<(Uuid, NaiveDate)> = sqlx::query_as(
        "UPDATE ent_internat_cas SET issue = 'rentre_famille', issue_le = $3, issue_par_user_id = $4, modifie_le = now() \
          WHERE etablissement_id = $1 AND eleve_id = $2 AND issue IS NULL AND nature = 'non_recupere' RETURNING id, nuit",
    )
    .bind(etab)
    .bind(eleve)
    .bind(a)
    .bind(actor)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (cas, nuit) in clos {
        publier(
            conn,
            etab,
            types::INTERNAT_CAS_CLOS,
            AggregateKind::Internat,
            cas,
            Some(eleve),
            json!({ "nuit": nuit.to_string(), "issue": "rentre_famille" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
    }
    Ok(())
}

/// Dernier blocage posé sur la cible (code de motif) : `interdiction_remise`, `interdiction_auteur`, `desaccord_titulaires`…
async fn cause_blocage_tx(conn: &mut PgConnection, etab: Uuid, cible: Cible) -> Result<Option<String>, EntDbError> {
    let (colonne, id) = match cible {
        Cible::Absence(i) => ("absence_id", i),
        Cible::Sortie(i) => ("sortie_id", i),
        Cible::Correspondant(i) => ("correspondant_id", i),
        Cible::Tiers(i) => ("tiers_id", i),
    };
    sqlx::query_scalar(&format!(
        "SELECT motif_code FROM ent_internat_i3_fait WHERE etablissement_id = $1 AND {colonne} = $2 AND acte = 'blocage' \
          ORDER BY a DESC, cree_le DESC LIMIT 1"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)
    .map(Option::flatten)
}

/// Passe 3 (point 4) : lever un blocage dû à une interdiction (de la personne désignée, d'un auteur, ou **sans personne
/// identifiée**) est une décision explicite de la direction, **tracée avec un motif obligatoire** : c'est la levée
/// ponctuelle. Renvoie le code de motif du fait (celui du blocage s'il vient d'une interdiction).
async fn motif_de_levee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    cible: Cible,
    eleve: Uuid,
    jour: NaiveDate,
    motif: Option<&str>,
) -> Result<&'static str, EntDbError> {
    let cause = cause_blocage_tx(conn, etab, cible).await?;
    let code = match cause.as_deref() {
        Some("interdiction_remise") => "interdiction_remise",
        Some("interdiction_auteur") => "interdiction_auteur",
        _ => "desaccord_titulaires",
    };
    let sans_cible = interdiction_sans_cible_depuis_tx(conn, etab, eleve, jour, false).await?.is_some();
    if motif.is_none() && (code != "desaccord_titulaires" || sans_cible) {
        return Err(refus(
            "motif obligatoire : lever un blocage dû à une interdiction est une décision explicite de la direction, tracée",
        ));
    }
    Ok(code)
}

/// Refus d'un contrôle de départ : le fait est **écrit et validé** avant de refuser, pour que la tentative reste
/// tracée (un départ à une personne interdite se voit dans l'historique de la sortie ou de l'absence).
#[allow(clippy::too_many_arguments)]
async fn depart_refuse<T>(
    mut tx: TenantTx<'static>,
    etab: Uuid,
    cible: Cible,
    actor: Uuid,
    a: DateTime<Utc>,
    motif_code: &str,
    personne: Option<Uuid>,
    message: &'static str,
    interdit: bool,
) -> Result<T, EntDbError> {
    fait_tx(&mut tx, etab, cible, "depart_refuse", "personnel", Some(actor), a, Some(motif_code), None, personne, None)
        .await?;
    tx.commit().await.map_err(db)?;
    Err(if interdit { conflit(message) } else { refus(message) })
}

/// Heure d'un geste de contrôle : jamais dans le futur (5 minutes de tolérance d'horloge), jamais antidatée de plus
/// de 30 minutes (une antidate réécrirait une nuit passée).
fn heure_du_geste(a: Option<DateTime<Utc>>, maintenant: DateTime<Utc>) -> Result<DateTime<Utc>, EntDbError> {
    let a = a.unwrap_or(maintenant);
    if a > maintenant + Duration::minutes(5) {
        return Err(refus("l'heure du geste ne peut pas être dans le futur"));
    }
    if a < maintenant - Duration::minutes(GESTE_ANTIDATE_MAX_MINUTES) {
        return Err(refus("l'heure du geste ne peut pas être antérieure de plus de 30 minutes : noter le geste à l'heure où il est enregistré"));
    }
    Ok(a)
}

/// Ce qui se remet à une personne un jour donné : une sortie ou le départ d'une absence annoncée.
struct CibleRemise {
    sortie_id: Option<Uuid>,
    absence_id: Option<Uuid>,
    eleve_id: Uuid,
    type_code: String,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    seul: bool,
    statut: String,
    avec: Option<Uuid>,
}

impl PgEnt {
    /// Sorties d'un élève, des plus récentes aux plus anciennes : sa famille ou le personnel dans sa portée.
    pub async fn sorties_eleve_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<SortieDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let famille = self.famille_lecture_tx(&mut tx, etab, actor, role, eleve, jour).await?;
        if famille.is_none() {
            self.personnel_exiger(etab, actor, role, &[Action::InternatLire, Action::InternatSortiesDecider, Action::InternatAppel], eleve)
                .await?;
            crate::internat_nuit::journaliser_lecture_eleves_tx(&mut tx, etab, actor, "sorties", Some(eleve), None, &[eleve], maintenant)
                .await?;
        }
        let lignes: Vec<SortieLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_SORTIE} FROM ent_internat_sortie WHERE etablissement_id = $1 AND eleve_id = $2 \
              ORDER BY debut DESC LIMIT 200"
        ))
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let (noms, proteges, caches) = if famille.is_some() {
            (noms_tx(&mut tx, etab, &[eleve]).await?, HashSet::new(), HashSet::new())
        } else {
            self.noms_pour_personnel(etab, actor, role, &mut tx, &[eleve]).await?
        };
        let mut sortie = Vec::with_capacity(lignes.len());
        for l in &lignes {
            let nom = (!caches.contains(&eleve)).then(|| noms.get(&eleve).cloned()).flatten();
            sortie.push(self.sortie_vers_dto(&mut tx, etab, l, nom, proteges.contains(&eleve)).await?);
        }
        tx.commit().await.map_err(db)?;
        Ok(sortie)
    }

    /// File du CPE : sorties demandées en attente et sorties bloquées par un désaccord.
    pub async fn sorties_a_traiter_a(&self, etab: Uuid, actor: Uuid, role: &str) -> Result<Vec<SortieDto>, EntDbError> {
        let portee = self.nuit_portee(etab, actor, role, &[Action::InternatSortiesDecider]).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<SortieLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_SORTIE} FROM ent_internat_sortie \
              WHERE etablissement_id = $1 AND (statut IN ('demandee', 'bloquee') \
                     OR (statut IN ('autorisee', 'partie') AND retard_demande_fin IS NOT NULL)) \
              ORDER BY CASE statut WHEN 'bloquee' THEN 0 WHEN 'demandee' THEN 1 ELSE 2 END, debut LIMIT 500"
        ))
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let lignes: Vec<SortieLigne> = lignes
            .into_iter()
            .filter(|l| portee.as_ref().is_none_or(|p| p.contains(&l.eleve_id)))
            .collect();
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.eleve_id).collect();
        let (noms, proteges, caches) = self.noms_pour_personnel(etab, actor, role, &mut tx, &ids).await?;
        let mut sortie = Vec::with_capacity(lignes.len());
        for l in &lignes {
            let nom = (!caches.contains(&l.eleve_id)).then(|| noms.get(&l.eleve_id).cloned()).flatten();
            sortie.push(self.sortie_vers_dto(&mut tx, etab, l, nom, proteges.contains(&l.eleve_id)).await?);
        }
        tx.commit().await.map_err(db)?;
        Ok(sortie)
    }

    /// INT-159 : contrôle d'un départ (surveillant ou accueil, `internat.appel`). Vérifie, au moment du geste :
    /// la sortie est autorisée ; l'élève part seul avec une autorisation valide (majeur, accord annuel du
    /// socle 18 relu à l'instant, ou accord de la famille sur la sortie) ou est remis à **la personne désignée**, qui
    /// doit être **autorisée aujourd'hui** (R-IT23) ; **une interdiction de remise bloque toujours** et la tentative
    /// est tracée, comme toute remise à une autre personne que celle qui est désignée (appel du CPE).
    pub async fn controler_depart_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ControleDepartDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        let a = heure_du_geste(b.a, maintenant)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Passe 4 (R7) : ORDRE DE VERROUILLAGE unique, le même que la revue à la minute et la suspension d'un
        // correspondant : correspondants de l'élève, PUIS sortie, PUIS absences. Ici, la sortie n'est lue (sans verrou)
        // que pour connaître l'élève ; ses correspondants sont verrouillés avant elle.
        let eleve_de_la_sortie: Option<Uuid> = sqlx::query_scalar(
            "SELECT eleve_id FROM ent_internat_sortie WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(e) = eleve_de_la_sortie {
            sqlx::query(
                "SELECT id FROM ent_internat_correspondant WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY id FOR UPDATE",
            )
            .bind(etab)
            .bind(e)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        }
        let s = sortie_tx(&mut tx, etab, id, true).await?;
        self.personnel_exiger(etab, actor, role, &[Action::InternatAppel], s.eleve_id).await?;
        let t = type_tx(&mut tx, etab, &s.type_code).await?;
        let jour = jour_local_tx(&mut tx, etab, a).await?;
        let cible = Cible::Sortie(id);
        // Passe 3 — « un acte tombe avec son auteur » (règle unique, `actes.rs`), avant tout autre contrôle et pour tout
        // état en attente ou en vigueur (demandée, autorisée, suspendue, bloquée) : demande, accord, personne désignée,
        // autorisation annuelle d'un auteur désormais visé. La décision revient au CPE.
        if matches!(s.statut.as_str(), "demandee" | "autorisee" | "suspendue" | "bloquee") {
            let chutes = actes_qui_tombent_tx(&mut tx, etab, &[Objet::Sortie(&s)], jour).await?;
            if let Some(chute) = chutes.get(&id) {
                appliquer_sortie_tx(&mut tx, etab, &s, chute, a).await?;
                return depart_refuse(
                    tx, etab, cible, actor, a, "interdiction_auteur", None,
                    "départ refusé : un acte de cette sortie (demande, accord, personne désignée ou autorisation annuelle) a été posé par une personne qui fait désormais l'objet d'une interdiction ou qui n'a plus de droit sur l'élève : la décision revient au CPE",
                    true,
                )
                .await;
            }
        }
        match s.statut.as_str() {
            "autorisee" => {}
            "partie" => return Err(conflit("cet élève est déjà parti pour cette sortie")),
            "suspendue" => {
                return depart_refuse(
                    tx, etab, cible, actor, a, "autorisation_revoquee", None,
                    "départ refusé : l'autorisation de cette sortie est suspendue (retrait de l'autorisation ou interdiction) : appeler le CPE",
                    true,
                )
                .await
            }
            "bloquee" => {
                return depart_refuse(
                    tx, etab, cible, actor, a, "desaccord_titulaires", None,
                    "départ refusé : cette sortie est bloquée par un désaccord entre les titulaires : appeler la direction",
                    true,
                )
                .await
            }
            "demandee" => return Err(conflit("cette sortie n'est pas encore autorisée")),
            _ => return Err(conflit("cette sortie est terminée, refusée ou annulée")),
        }
        if a < s.debut - Duration::minutes(DEPART_AVANCE_MAX_MINUTES) {
            return Err(refus("trop tôt : le départ se contrôle trois heures avant l'heure prévue au plus"));
        }
        if a >= s.fin_ajustee {
            return Err(conflit("l'heure prévue de cette sortie est dépassée"));
        }
        let majeur = eleve_majeur_tx(&mut tx, etab, s.eleve_id, jour).await?;
        // E-A : un correspondant désigné par un compte désormais interdit est suspendu avant le contrôle ci-dessous.
        if s.correspondant_id.is_some() {
            let mut echecs_revue = 0;
            suspendre_correspondants_auteur_tx(&mut tx, etab, Some(s.eleve_id), jour, a, &mut echecs_revue).await?;
        }
        // Le correspondant désigné doit l'être encore (CIT-33).
        if let Some(c) = s.correspondant_id {
            let valide: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_internat_correspondant \
                  WHERE etablissement_id = $1 AND id = $2 AND statut = 'valide')",
            )
            .bind(etab)
            .bind(c)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !valide {
                return depart_refuse(
                    tx, etab, cible, actor, a, "correspondant_suspendu", s.avec_personne_id,
                    "départ refusé : le correspondant est suspendu ou n'est plus désigné : appeler le CPE",
                    true,
                )
                .await;
            }
        }
        // Moyen 3 : dès que la sortie repose sur l'autorisation annuelle (départ seul ou accompagné), le verdict du
        // socle 18 est relu au moment du départ : retirée entre-temps, la sortie est suspendue et le départ refusé.
        if s.par_autorisation_annuelle && verdict_annuel(self, &mut tx, etab, s.eleve_id, &t.usage_accord_code, jour).await != "autorise" {
            sqlx::query("UPDATE ent_internat_sortie SET statut = 'suspendue', modifie_le = $3 WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .bind(a)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            fait_tx(&mut tx, etab, cible, "suspension", "systeme", None, a, Some("autorisation_revoquee"), None, None, None).await?;
            evenements_sortie_tx(&mut tx, etab, Some(actor), &s, types::INTERNAT_SORTIE_DECIDEE, "suspendue", "autorisation_annuelle", true).await?;
            return depart_refuse(
                tx, etab, cible, actor, a, "autorisation_revoquee", None,
                "départ refusé : l'autorisation annuelle de sortie n'est plus valable : appeler le CPE",
                true,
            )
            .await;
        }
        let remis = b.remis_a_personne_id;
        let mut controle = "majeur";
        match (s.seul, remis) {
            (true, Some(_)) => return Err(refus("cette sortie se fait seul : aucune remise à une personne")),
            (true, None) => {
                // Départ seul d'un mineur : l'autorisation annuelle (relue ci-dessus) ou l'accord de la famille sur la
                // sortie, quel que soit le décideur du type (Moyen 4).
                if !majeur {
                    // M-A : une interdiction de remise sans personne identifiée est en vigueur : pas de départ seul sans
                    // décision de la direction.
                    if seul_arrete_par_interdiction_tx(&mut tx, etab, cible, s.eleve_id, jour).await? {
                        bloquer_sortie_systeme_tx(&mut tx, etab, &s, "interdiction_remise", a).await?;
                        return depart_refuse(
                            tx, etab, cible, actor, a, "interdiction_remise", None,
                            "départ seul refusé : une interdiction de remise sans personne identifiée est en vigueur : appeler la direction",
                            true,
                        )
                        .await;
                    }
                    if !s.par_autorisation_annuelle && s.accord_titulaire_user_id.is_none() {
                        return depart_refuse(
                            tx, etab, cible, actor, a, "personne_non_autorisee", None,
                            "départ refusé : un mineur ne part seul qu'avec l'accord de sa famille ou son autorisation annuelle : appeler le CPE",
                            false,
                        )
                        .await;
                    }
                    controle = "seul_autorise";
                }
            }
            (false, None) => {
                if !majeur {
                    return Err(refus("départ accompagné : indiquer la personne à qui l'élève est remis (R-IT23)"));
                }
            }
            (false, Some(p)) => {
                match remise_pour_tx(&mut tx, etab, Some(cible), s.eleve_id, p, jour).await? {
                    Remise::Autorisee(_) => {}
                    Remise::Interdite => {
                        return depart_refuse(
                            tx, etab, cible, actor, a, "interdiction_remise", Some(p),
                            "REMISE REFUSÉE : cette personne fait l'objet d'une interdiction : appeler la direction (R-IT23)",
                            true,
                        )
                        .await
                    }
                    Remise::InterditeSansCible => {
                        // Passe 3 (point 4) : une sortie accompagnée reste impossible tant que l'interdiction sans personne
                        // identifiée est en vigueur, sauf décision explicite de la direction : la sortie est bloquée, la
                        // direction tranche (levée ponctuelle, motif obligatoire).
                        bloquer_sortie_systeme_tx(&mut tx, etab, &s, "interdiction_remise", a).await?;
                        return depart_refuse(
                            tx, etab, cible, actor, a, "interdiction_remise", Some(p),
                            "REMISE REFUSÉE : une interdiction de remise sans personne identifiée est en vigueur : appeler la direction",
                            true,
                        )
                        .await
                    }
                    Remise::NonAutorisee => {
                        return depart_refuse(
                            tx, etab, cible, actor, a, "personne_non_autorisee", Some(p),
                            "remise refusée : cette personne n'est pas autorisée à récupérer l'élève aujourd'hui (R-IT23)",
                            false,
                        )
                        .await
                    }
                }
                // Moyen 1 : la remise ne se fait qu'à la personne désignée ; toute autre personne, même autorisée,
                // est refusée et le CPE est appelé (la tentative est tracée).
                let designee = match s.avec_personne_id {
                    Some(d) => meme_personne_tx(&mut tx, etab, d, p).await?,
                    None => false,
                };
                if !designee {
                    return depart_refuse(
                        tx, etab, cible, actor, a, "personne_non_autorisee", Some(p),
                        "remise refusée : ce n'est pas la personne désignée pour cette sortie (ou aucune n'est désignée) : appeler le CPE (R-IT23)",
                        false,
                    )
                    .await;
                }
                controle = "personne_autorisee";
            }
        }
        sqlx::query(
            "UPDATE ent_internat_sortie SET statut = 'partie', depart_effectif = $3, modifie_le = $4 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(a)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_tx(&mut tx, etab, cible, "depart", "personnel", Some(actor), a, None, Some(controle), remis, None).await?;
        evenements_sortie_tx(&mut tx, etab, Some(actor), &s, types::INTERNAT_SORTIE_PARTIE, "partie", controle, false).await?;
        clore_non_recupere_tx(&mut tx, etab, actor, s.eleve_id, a).await?;
        let l = sortie_tx(&mut tx, etab, id, false).await?;
        let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// C-1 : contrôle du départ d'une **absence annoncée** au portail, comme celui d'une sortie. Un mineur n'est remis
    /// qu'à la personne désignée par la famille, **autorisée aujourd'hui** (R-IT23, interdiction de remise sur toute la
    /// chaîne de fusions), ou part seul avec l'autorisation annuelle du socle 18 relue à l'instant ; un majeur part
    /// seul. Toute autre remise est refusée et tracée (appel du CPE). Le départ est un fait : la revue à la minute
    /// ne bloque plus l'absence ensuite.
    pub async fn controler_depart_absence_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ControleDepartDto,
        maintenant: DateTime<Utc>,
    ) -> Result<AbsenceDto, EntDbError> {
        let a = heure_du_geste(b.a, maintenant)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ab = absence_tx(&mut tx, etab, id, true).await?;
        self.personnel_exiger(etab, actor, role, &[Action::InternatAppel], ab.eleve_id).await?;
        let jour = jour_local_tx(&mut tx, etab, a).await?;
        let cible = Cible::Absence(id);
        match ab.statut.as_str() {
            "annoncee" | "acceptee" => {}
            "bloquee" => {
                return depart_refuse(
                    tx, etab, cible, actor, a, "desaccord_titulaires", None,
                    "départ refusé : cette absence est bloquée (désaccord, interdiction ou autorisation retirée) : appeler le CPE ou la direction",
                    true,
                )
                .await
            }
            "demandee" => return Err(conflit("cette absence n'est pas encore acceptée par le CPE")),
            _ => return Err(conflit("cette absence est refusée, annulée ou régularisée : aucun départ à contrôler")),
        }
        if ab.depart_effectif.is_some() {
            return Err(conflit("cet élève est déjà parti pour cette absence"));
        }
        if jour < ab.du_nuit || jour > ab.au_nuit {
            return Err(refus("le départ se contrôle le jour d'une des nuits de l'absence"));
        }
        let majeur = eleve_majeur_tx(&mut tx, etab, ab.eleve_id, jour).await?;
        // Passe 3 — « un acte tombe avec son auteur » (règle unique, `actes.rs`) : annonce, personne désignée ou
        // autorisation annuelle d'un auteur désormais visé. L'absence est bloquée (la personne désignée est effacée),
        // la direction tranche ; l'élève reste attendu.
        if matches!(ab.statut.as_str(), "annoncee" | "acceptee") {
            let chutes = actes_qui_tombent_tx(&mut tx, etab, &[Objet::Absence(&ab)], jour).await?;
            if let Some(chute) = chutes.get(&id) {
                appliquer_absence_tx(&mut tx, etab, &ab, chute, a).await?;
                return depart_refuse(
                    tx, etab, cible, actor, a, "interdiction_auteur", None,
                    "départ refusé : cette absence repose sur un acte (annonce, personne désignée ou autorisation annuelle) posé par une personne qui fait désormais l'objet d'une interdiction ou qui n'a plus de droit sur l'élève : appeler le CPE ou la direction",
                    true,
                )
                .await;
            }
        }
        let remis = b.remis_a_personne_id;
        let mut controle = "majeur";
        match remis {
            Some(p) => {
                let remise = remise_pour_tx(&mut tx, etab, Some(cible), ab.eleve_id, p, jour).await?;
                match remise {
                    Remise::Autorisee(_) => {}
                    Remise::Interdite | Remise::InterditeSansCible => {
                        // Sans personne identifiée : la direction seule peut lever (décision explicite, motif obligatoire).
                        if remise == Remise::InterditeSansCible {
                            bloquer_absence_systeme_tx(&mut tx, etab, &ab, "interdiction_remise", a).await?;
                        }
                        return depart_refuse(
                            tx, etab, cible, actor, a, "interdiction_remise", Some(p),
                            "REMISE REFUSÉE : cette personne fait l'objet d'une interdiction (ou une interdiction sans personne identifiée est en vigueur) : appeler la direction (R-IT23)",
                            true,
                        )
                        .await
                    }
                    Remise::NonAutorisee => {
                        return depart_refuse(
                            tx, etab, cible, actor, a, "personne_non_autorisee", Some(p),
                            "remise refusée : cette personne n'est pas autorisée à récupérer l'élève aujourd'hui (R-IT23)",
                            false,
                        )
                        .await
                    }
                }
                let designee = match ab.recupere_par_personne_id {
                    Some(d) => meme_personne_tx(&mut tx, etab, d, p).await?,
                    None => false,
                };
                if !designee {
                    return depart_refuse(
                        tx, etab, cible, actor, a, "personne_non_autorisee", Some(p),
                        "remise refusée : ce n'est pas la personne désignée par la famille (ou aucune n'est désignée) : appeler le CPE (R-IT23)",
                        false,
                    )
                    .await;
                }
                controle = "personne_autorisee";
            }
            None if !majeur => {
                if ab.recupere_par_personne_id.is_some() {
                    return Err(refus("départ accompagné : indiquer la personne à qui l'élève est remis (R-IT23)"));
                }
                if seul_arrete_par_interdiction_tx(&mut tx, etab, cible, ab.eleve_id, jour).await? {
                    bloquer_absence_systeme_tx(&mut tx, etab, &ab, "interdiction_remise", a).await?;
                    return depart_refuse(
                        tx, etab, cible, actor, a, "interdiction_remise", None,
                        "départ seul refusé : une interdiction de remise sans personne identifiée est en vigueur : appeler la direction",
                        true,
                    )
                    .await;
                }
                let t = type_tx(&mut tx, etab, "week_end").await?;
                // F-B : l'autorisation annuelle « départ seul le week-end » ne couvre une absence que si sa première nuit
                // est une veille de week-end.
                if !week_end_complet(ab.du_nuit, ab.au_nuit)
                    || verdict_annuel(self, &mut tx, etab, ab.eleve_id, &t.usage_accord_code, jour).await != "autorise"
                {
                    return depart_refuse(
                        tx, etab, cible, actor, a, "autorisation_revoquee", None,
                        "départ refusé : un mineur ne part seul qu'avec l'autorisation annuelle de sa famille (retirée, absente, ou ne couvrant que les départs de week-end) : appeler le CPE",
                        false,
                    )
                    .await;
                }
                controle = "seul_autorise";
            }
            None => {}
        }
        sqlx::query(
            "UPDATE ent_internat_absence SET depart_effectif = $3, depart_remis_a_personne_id = $4, modifie_le = $5 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(a)
        .bind(remis)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_tx(&mut tx, etab, cible, "depart", "personnel", Some(actor), a, None, Some(controle), remis, None).await?;
        clore_non_recupere_tx(&mut tx, etab, actor, ab.eleve_id, a).await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_SORTIE_PARTIE,
            AggregateKind::Internat,
            id,
            Some(ab.eleve_id),
            json!({ "objet": "absence", "du": ab.du_nuit.to_string(), "au": ab.au_nuit.to_string(), "controle": controle }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let ligne = absence_tx(&mut tx, etab, id, false).await?;
        let opp = opposition_tx(&mut tx, etab, cible).await?;
        tx.commit().await.map_err(db)?;
        Ok(absence_dto(&ligne, None, false, opp))
    }

    /// INT-160 : contrôle d'un retour. Ferme les cas « retard de retour » et « non récupéré » ouverts pour l'élève
    /// (retrouvé, dans sa chambre), et remet l'élève à l'attendu des nuits suivantes.
    pub async fn controler_retour_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &ControleRetourDto,
        maintenant: DateTime<Utc>,
    ) -> Result<SortieDto, EntDbError> {
        let a = heure_du_geste(b.a, maintenant)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let s = sortie_tx(&mut tx, etab, id, true).await?;
        self.personnel_exiger(etab, actor, role, &[Action::InternatAppel], s.eleve_id).await?;
        if s.statut != "partie" {
            return Err(conflit("cet élève n'est pas parti pour cette sortie"));
        }
        if s.depart_effectif.is_some_and(|d| a < d) {
            return Err(refus("le retour ne précède pas le départ"));
        }
        // Un retour antidaté ne remet pas l'élève à l'attendu d'une nuit PASSÉE : si le geste est daté d'avant le dernier
        // matin (6 h) déjà écoulé, l'heure de retour portée par la sortie (qui sert à l'attendu) ne précède pas ce
        // matin ; l'heure exacte du geste reste dans le fait « retour ».
        let dernier_matin: DateTime<Utc> = sqlx::query_scalar(
            "SELECT (((($2::timestamptz AT TIME ZONE ent_internat_fuseau($1))::date \
                       - CASE WHEN ($2::timestamptz AT TIME ZONE ent_internat_fuseau($1))::time >= TIME '06:00' THEN 0 ELSE 1 END) \
                      + TIME '06:00') AT TIME ZONE ent_internat_fuseau($1))",
        )
        .bind(etab)
        .bind(maintenant)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let retour = if a < dernier_matin && s.fin_ajustee >= dernier_matin { dernier_matin } else { a };
        sqlx::query(
            "UPDATE ent_internat_sortie SET statut = 'rentree', retour_effectif = $3, modifie_le = $4 \
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(retour)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        fait_tx(&mut tx, etab, Cible::Sortie(id), "retour", "personnel", Some(actor), a, None, None, None, None).await?;
        // Un cas ouvert pour ce retard (ou ce « non récupéré ») se ferme : l'élève est là.
        let clos: Vec<(Uuid, NaiveDate)> = sqlx::query_as(
            "UPDATE ent_internat_cas SET issue = 'retrouve', issue_le = $3, issue_par_user_id = $4, \
                    issue_lieu_code = 'chambre', modifie_le = now() \
              WHERE etablissement_id = $1 AND eleve_id = $2 AND issue IS NULL \
                AND nature IN ('retard_retour', 'non_recupere', 'non_rentre') RETURNING id, nuit",
        )
        .bind(etab)
        .bind(s.eleve_id)
        .bind(a)
        .bind(actor)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (cas, nuit) in clos {
            publier(
                &mut tx,
                etab,
                types::INTERNAT_CAS_CLOS,
                AggregateKind::Internat,
                cas,
                Some(s.eleve_id),
                json!({ "nuit": nuit.to_string(), "issue": "retrouve" }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        evenements_sortie_tx(&mut tx, etab, Some(actor), &s, types::INTERNAT_SORTIE_RENTREE, "rentree", "retour", true).await?;
        let l = sortie_tx(&mut tx, etab, id, false).await?;
        let dto = self.sortie_vers_dto(&mut tx, etab, &l, None, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Élèves en sortie courte (mercredi après-midi, soirée) à 22 h de la nuit donnée : ils restent attendus ; le
    /// surveillant les pointe « en activité » avec leur retour attendu. Les sorties qui couvrent toute la nuit
    /// ne figurent pas ici : l'attendu les retire déjà.
    pub async fn sorties_du_soir_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        nuit: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<SortieDuSoirDto>, EntDbError> {
        let portee = self.nuit_portee(etab, actor, role, &[Action::InternatAppel, Action::InternatLire]).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<(Uuid, Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT s.id, s.eleve_id, s.type_code, s.statut, s.fin_ajustee FROM ent_internat_sortie s \
              WHERE s.etablissement_id = $1 AND s.statut IN ('autorisee', 'partie') \
                AND s.debut <= ent_internat_ancre_nuit($1, $2) AND s.fin_ajustee > ent_internat_ancre_nuit($1, $2) \
                AND s.fin_ajustee < ((($2::date + 1) + TIME '06:00') AT TIME ZONE ent_internat_fuseau($1)) \
              ORDER BY s.fin_ajustee, s.id",
        )
        .bind(etab)
        .bind(nuit)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let rows: Vec<_> = rows.into_iter().filter(|r| portee.as_ref().is_none_or(|p| p.contains(&r.1))).collect();
        let ids: Vec<Uuid> = rows.iter().map(|r| r.1).collect();
        let (noms, proteges, caches) = self.noms_pour_personnel(etab, actor, role, &mut tx, &ids).await?;
        crate::internat_nuit::journaliser_lecture_eleves_tx(&mut tx, etab, actor, "sorties", None, None, &ids, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(sortie_id, eleve_id, type_code, statut, fin_ajustee)| SortieDuSoirDto {
                sortie_id,
                eleve_id,
                nom: (!caches.contains(&eleve_id)).then(|| noms.get(&eleve_id).map(|n| n.0.clone())).flatten(),
                prenom: (!caches.contains(&eleve_id)).then(|| noms.get(&eleve_id).map(|n| n.1.clone())).flatten(),
                protege: proteges.contains(&eleve_id),
                type_code,
                statut,
                fin_ajustee,
            })
            .collect())
    }
}

// ───────────────────────── Départs du vendredi, remise, veille ─────────────────────────

impl PgEnt {
    /// INT-157 : établit l'attendu de départ d'un jour (vendredi, veille de vacances) : pour chaque interne qui a
    /// dormi à l'internat la veille et n'y dort pas ce soir (hors rythme), une sortie « week-end » autorisée
    /// (règle implicite : l'enfant rentre chez lui), **sans personne désignée** : la famille ou le CPE la précise
    /// avant la remise. Les élèves déjà couverts (absence, sortie, stage…) ne sont pas touchés. Idempotente.
    pub async fn preparer_departs_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<DepartsPrevusDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r = Self::preparer_departs_tx(&mut tx, etab, Some(actor), jour, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    pub(crate) async fn preparer_departs_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Option<Uuid>,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<DepartsPrevusDto, EntDbError> {
        let aujourdhui = jour_local_tx(conn, etab, maintenant).await?;
        if jour < aujourdhui || (jour - aujourdhui).num_days() > 400 {
            return Err(refus("le jour de départ est passé ou trop lointain"));
        }
        let t = type_tx(conn, etab, "week_end").await?;
        if !t.actif {
            return Ok(DepartsPrevusDto { jour, crees: 0, deja_couverts: 0, sans_personne: 0 });
        }
        let debut = instant_mural_tx(conn, etab, jour, NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default()).await?;
        // Qui part : attendu la veille, pas attendu ce soir. `couvert` : déjà pris en compte, rien à créer : retiré de
        // l'attendu de ce soir (stage, mobilité…), ou une absence en vigueur couvre la nuit, ou une sortie (en vigueur
        // ou en attente) couvre l'heure de départ. Les deux calculs de l'attendu sont matérialisés une fois.
        let partants: Vec<(Uuid, bool)> = sqlx::query_as(
            "WITH hier AS MATERIALIZED (SELECT eleve_id FROM ent_internat_attendu_calcule($1, $2::date - 1) \
                                         WHERE statut IN ('attendu', 'ajoute')), \
                  ce_soir AS MATERIALIZED (SELECT eleve_id, statut FROM ent_internat_attendu_calcule($1, $2)) \
             SELECT h.eleve_id, \
                    EXISTS (SELECT 1 FROM ce_soir c WHERE c.eleve_id = h.eleve_id AND c.statut = 'retire') \
                 OR EXISTS (SELECT 1 FROM ent_internat_absence a WHERE a.etablissement_id = $1 AND a.eleve_id = h.eleve_id \
                             AND a.statut IN ('annoncee', 'acceptee', 'regularisee') AND a.du_nuit <= $2 AND a.au_nuit >= $2) \
                 OR EXISTS (SELECT 1 FROM ent_internat_sortie s WHERE s.etablissement_id = $1 AND s.eleve_id = h.eleve_id \
                             AND s.statut IN ('demandee', 'autorisee', 'suspendue', 'partie', 'bloquee') \
                             AND s.debut <= $3 AND s.fin_ajustee > $3) \
               FROM hier h \
              WHERE NOT EXISTS (SELECT 1 FROM ce_soir c WHERE c.eleve_id = h.eleve_id AND c.statut IN ('attendu', 'ajoute')) \
              ORDER BY h.eleve_id",
        )
        .bind(etab)
        .bind(jour)
        .bind(debut)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let a_creer: Vec<Uuid> = partants.iter().filter(|(_, couvert)| !*couvert).map(|(e, _)| *e).collect();
        let mut deja = partants.iter().filter(|(_, couvert)| *couvert).count() as i32;
        if a_creer.is_empty() {
            return Ok(DepartsPrevusDto { jour, crees: 0, deja_couverts: deja, sans_personne: 0 });
        }
        // Retour prévu : la première nuit où l'élève est de nouveau attendu (au plus 14 jours plus tard).
        let mut retour: HashMap<Uuid, NaiveDate> = HashMap::new();
        for plus in 1..=14i64 {
            let d = jour + Duration::days(plus);
            let revenus: Vec<Uuid> = sqlx::query_scalar(
                "SELECT eleve_id FROM ent_internat_attendu_calcule($1, $2) \
                  WHERE statut IN ('attendu', 'ajoute') AND eleve_id = ANY($3)",
            )
            .bind(etab)
            .bind(d)
            .bind(&a_creer)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
            for e in revenus {
                retour.entry(e).or_insert(d);
            }
            if retour.len() == a_creer.len() {
                break;
            }
        }
        let p = parametres_tx(conn, etab).await?;
        let mut crees = 0;
        for e in a_creer {
            let d_retour = retour.get(&e).copied().unwrap_or(jour + Duration::days(14));
            let fin = instant_mural_tx(conn, etab, d_retour, p.heure_limite_appel).await?;
            if fin <= debut {
                continue;
            }
            let id = Uuid::new_v4();
            let r = sqlx::query(
                "INSERT INTO ent_internat_sortie \
                 (id, etablissement_id, eleve_id, type_code, debut, fin_prevue, fin_ajustee, seul, \
                  demande_par_user_id, demande_qualite, statut, cree_le, modifie_le) \
                 VALUES ($1,$2,$3,'week_end',$4,$5,$5,false,NULL,'systeme','autorisee',$6,$6)",
            )
            .bind(id)
            .bind(etab)
            .bind(e)
            .bind(debut)
            .bind(fin)
            .bind(maintenant)
            .execute(&mut *conn)
            .await;
            match r {
                Ok(_) => {}
                Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23P01") => {
                    // Une sortie couvre déjà ces heures (créée entre-temps ou par la famille) : rien à faire.
                    deja += 1;
                    continue;
                }
                Err(e) => return Err(db(e)),
            }
            fait_tx(conn, etab, Cible::Sortie(id), "demande", "systeme", None, maintenant, None, None, None, Some(fin)).await?;
            fait_tx(conn, etab, Cible::Sortie(id), "acceptation", "systeme", None, maintenant, None, None, None, None).await?;
            publier(
                conn,
                etab,
                types::INTERNAT_DEPART_PREVU,
                AggregateKind::Internat,
                id,
                Some(e),
                json!({ "jour": jour.to_string(), "type": "week_end", "retour_prevu": fin.to_rfc3339() }),
                actor,
                Causalite::default(),
            )
            .await?;
            crees += 1;
        }
        Ok(DepartsPrevusDto { jour, crees, deja_couverts: deja, sans_personne: crees })
    }

    /// INT-167 : liste de remise du jour pour le surveillant et l'accueil : qui part, à quelle heure, qui peut le
    /// prendre (personnes autorisées aujourd'hui, socle 4) et **qui ne le doit pas** (interdictions de remise, nom
    /// seulement, aucun motif). Les départs des **absences annoncées** d'élèves mineurs (C-1) y figurent comme ceux des
    /// sorties. Jamais une adresse ni une coordonnée. Un élève protégé caché au tiers n'est pas nommé et on ne montre ni
    /// ses titulaires ni ses contacts. Lecture journalisée.
    pub async fn liste_remise_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<ListeRemiseLigneDto>, EntDbError> {
        let portee = self.nuit_portee(etab, actor, role, &[Action::InternatAppel]).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<SortieLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_SORTIE} FROM ent_internat_sortie \
              WHERE etablissement_id = $1 AND statut IN ('autorisee', 'suspendue', 'bloquee', 'partie') \
                AND (debut AT TIME ZONE ent_internat_fuseau($1))::date <= $2 \
                AND (fin_ajustee AT TIME ZONE ent_internat_fuseau($1))::date >= $2 \
              ORDER BY debut, id LIMIT 500"
        ))
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let absences: Vec<AbsenceLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_ABSENCE} FROM ent_internat_absence \
              WHERE etablissement_id = $1 AND statut IN ('annoncee', 'acceptee', 'bloquee') AND depart_effectif IS NULL \
                AND du_nuit <= $2 AND au_nuit >= $2 ORDER BY du_nuit, id LIMIT 500"
        ))
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // Un majeur part seul : sa famille n'a aucune remise à contrôler.
        let mut cibles: Vec<CibleRemise> = Vec::with_capacity(lignes.len() + absences.len());
        for l in lignes.into_iter().filter(|l| portee.as_ref().is_none_or(|p| p.contains(&l.eleve_id))) {
            cibles.push(CibleRemise {
                sortie_id: Some(l.id),
                absence_id: None,
                eleve_id: l.eleve_id,
                type_code: l.type_code,
                debut: l.debut,
                fin: l.fin_ajustee,
                seul: l.seul,
                statut: l.statut,
                avec: l.avec_personne_id,
            });
        }
        for ab in absences.into_iter().filter(|x| portee.as_ref().is_none_or(|p| p.contains(&x.eleve_id))) {
            if eleve_majeur_tx(&mut tx, etab, ab.eleve_id, jour).await? {
                continue;
            }
            let debut = instant_mural_tx(&mut tx, etab, ab.du_nuit, NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default()).await?;
            let fin = match ab.retour_prevu {
                Some(r) => r,
                None => instant_mural_tx(&mut tx, etab, ab.au_nuit + Duration::days(1), NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default()).await?,
            };
            cibles.push(CibleRemise {
                sortie_id: None,
                absence_id: Some(ab.id),
                eleve_id: ab.eleve_id,
                type_code: "absence".to_string(),
                debut,
                fin: fin.max(debut + Duration::hours(1)),
                seul: ab.recupere_par_personne_id.is_none(),
                statut: ab.statut,
                avec: ab.recupere_par_personne_id,
            });
        }
        let ids: Vec<Uuid> = cibles.iter().map(|c| c.eleve_id).collect();
        let (noms, proteges, caches) = self.noms_pour_personnel(etab, actor, role, &mut tx, &ids).await?;
        let mut sortie = Vec::with_capacity(cibles.len());
        for c in &cibles {
            let masque = caches.contains(&c.eleve_id);
            let (mut autorisees, mut interdites_ids_noms, mut non_nominative) = (Vec::new(), Vec::new(), false);
            if !masque {
                // Personnes qui pourraient prendre l'élève : titulaires, personnes autorisées à récupérer, tiers autorisés.
                let candidats: Vec<(Uuid, String, String)> = sqlx::query_as(
                    "SELECT p.id, p.nom, p.prenom FROM ent_personne p WHERE p.etablissement_id = $1 AND p.id IN ( \
                        SELECT l.personne_id FROM ent_eleve_lien l WHERE l.etablissement_id = $1 AND l.eleve_id = $2 \
                           AND l.personne_id IS NOT NULL AND l.suspendu_le IS NULL \
                        UNION SELECT c.personne_id FROM ent_eleve_contact_urgence c \
                         WHERE c.etablissement_id = $1 AND c.eleve_id = $2 AND c.peut_recuperer AND c.statut = 'valide' \
                        UNION SELECT t.personne_id FROM ent_internat_tiers_autorise t \
                         WHERE t.etablissement_id = $1 AND t.eleve_id = $2 AND t.statut = 'actif') \
                      ORDER BY p.nom, p.prenom, p.id",
                )
                .bind(etab)
                .bind(c.eleve_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                for (pid, nom, prenom) in &candidats {
                    if let Remise::Autorisee(relation) = remise_possible_tx(&mut tx, etab, c.eleve_id, *pid, jour).await? {
                        autorisees.push(PersonneRemiseDto { personne_id: *pid, nom: nom.clone(), prenom: prenom.clone(), relation });
                    }
                }
                interdites_ids_noms = sqlx::query_as(
                    "SELECT DISTINCT p.id, p.nom, p.prenom FROM ent_eleve_interdiction i \
                       JOIN ent_personne p ON p.etablissement_id = i.etablissement_id AND p.id = i.personne_id \
                      WHERE i.etablissement_id = $1 AND i.eleve_id = $2 AND i.levee_le IS NULL \
                        AND i.objet && ARRAY['remise', 'visite', 'contact_eleve']::text[] \
                        AND (i.au IS NULL OR i.au >= $3) ORDER BY p.nom, p.prenom, p.id",
                )
                .bind(etab)
                .bind(c.eleve_id)
                .bind(jour)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                non_nominative = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_eleve_interdiction i WHERE i.etablissement_id = $1 AND i.eleve_id = $2 \
                      AND i.levee_le IS NULL AND i.objet && ARRAY['remise', 'visite', 'contact_eleve']::text[] \
                      AND (i.au IS NULL OR i.au >= $3) AND i.personne_id IS NULL)",
                )
                .bind(etab)
                .bind(c.eleve_id)
                .bind(jour)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            }
            let interdites_ids: HashSet<Uuid> = interdites_ids_noms.iter().map(|i: &(Uuid, String, String)| i.0).collect();
            // La personne prévue n'est présentée comme telle que si elle est autorisée aujourd'hui.
            let prevue = c.avec.and_then(|p| autorisees.iter().find(|a| a.personne_id == p).cloned());
            sortie.push(ListeRemiseLigneDto {
                sortie_id: c.sortie_id,
                absence_id: c.absence_id,
                eleve_id: c.eleve_id,
                nom: (!masque).then(|| noms.get(&c.eleve_id).map(|n| n.0.clone())).flatten(),
                prenom: (!masque).then(|| noms.get(&c.eleve_id).map(|n| n.1.clone())).flatten(),
                protege: proteges.contains(&c.eleve_id),
                type_code: c.type_code.clone(),
                debut: c.debut,
                fin_ajustee: c.fin,
                seul: c.seul,
                statut: c.statut.clone(),
                personne_prevue: prevue,
                personnes_autorisees: autorisees.into_iter().filter(|a| !interdites_ids.contains(&a.personne_id)).collect(),
                personnes_interdites: interdites_ids_noms
                    .into_iter()
                    .map(|(personne_id, nom, prenom)| PersonneRemiseDto { personne_id, nom, prenom, relation: String::new() })
                    .collect(),
                interdiction_non_nominative: non_nominative,
            });
        }
        crate::internat_nuit::journaliser_lecture_eleves_tx(&mut tx, etab, actor, "liste_remise", None, None, &ids, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(sortie)
    }

    /// Veille du système (INT-161, INT-166) : ouvre un cas « retard de retour » pour toute sortie partie dont le
    /// retour attendu (à jour du retard annoncé) dépasse le seuil (S-4107) ; ouvre un cas « non récupéré » pour un
    /// départ de week-end qui n'a pas eu lieu à l'heure limite de récupération ; fait expirer les sorties
    /// jamais utilisées ; **revoit** les sorties à venir (une personne devenue interdite, un correspondant
    /// suspendu suspendent la sortie). Idempotente : un seul cas ouvert par élève et par nuit.
    pub async fn veille_sorties_a(&self, etab: Uuid, maintenant: DateTime<Utc>) -> Result<VeilleSortiesDto, EntDbError> {
        Ok(self.veille_sorties_bilan_a(etab, maintenant).await?.0)
    }

    /// Comme [`Self::veille_sorties_a`], avec le bilan de la revue (dont le nombre d'objets dont la revue a échoué (R7 : la tâche planifiée en tire
    /// l'alerte opérationnelle quand l'échec se répète)).
    pub(crate) async fn veille_sorties_bilan_a(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(VeilleSortiesDto, RevueBilan), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let nuit = nuit_a_tx(&mut tx, etab, maintenant).await?;
        let aujourdhui = jour_local_tx(&mut tx, etab, maintenant).await?;
        let p = parametres_tx(&mut tx, etab).await?;
        let seuil = Duration::minutes(i64::from(p.seuil_retard_minutes));
        let mut sortie = VeilleSortiesDto::default();
        // 1. Retard de retour.
        let en_retard: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_internat_sortie \
              WHERE etablissement_id = $1 AND statut = 'partie' AND retour_effectif IS NULL AND fin_ajustee + $2 < $3 \
              ORDER BY fin_ajustee, id",
        )
        .bind(etab)
        .bind(seuil)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (eleve,) in en_retard {
            if self.ouvrir_cas_tx(&mut tx, etab, None, None, eleve, nuit, "retard_retour", maintenant).await?.is_some() {
                sortie.cas_retard_retour += 1;
            }
        }
        // 2. Départ de week-end non récupéré à l'heure de départ prévue ou de récupération (+ seuil S-4107). Passe 3 (point 3) : seule la
        // PREMIÈRE nuit compte (le départ prévu a lieu ce jour-là) ; un cas déjà ouvert ou clos pour cette nuit n'est
        // pas rouvert chaque jour. Le cas se ferme au départ constaté (`controler_depart_*`).
        let a_recuperer: Vec<(Uuid, DateTime<Utc>, String)> = sqlx::query_as(
            "SELECT eleve_id, debut, demande_qualite FROM ent_internat_sortie \
              WHERE etablissement_id = $1 AND type_code = 'week_end' AND statut = 'autorisee' AND debut <= $2 AND fin_ajustee > $2 \
              ORDER BY debut, id",
        )
        .bind(etab)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (eleve, debut, qualite) in a_recuperer {
            if nuit_a_tx(&mut tx, etab, debut).await? != nuit {
                continue;
            }
            // Heure de départ prévue SAISIE (sortie demandée par la famille ou le CPE) ; la sortie créée par le système
            // n'a qu'un début nominal (midi) : l'heure limite est alors celle de récupération du vendredi.
            let limite = if qualite == "systeme" {
                let jour_depart = jour_local_tx(&mut tx, etab, debut).await?;
                instant_mural_tx(&mut tx, etab, jour_depart, p.heure_recuperation_vendredi).await?
            } else {
                debut
            };
            if maintenant >= limite + seuil
                && !cas_existe_tx(&mut tx, etab, eleve, nuit, "non_recupere").await?
                && self.ouvrir_cas_tx(&mut tx, etab, None, None, eleve, nuit, "non_recupere", maintenant).await?.is_some()
            {
                sortie.cas_non_recupere += 1;
            }
        }
        // 2 bis. M-B : une absence de MINEUR qui COMMENCE LE VENDREDI, restée sans départ constaté à l'heure limite de
        // récupération (même heure et même seuil que le départ de week-end) ouvre un cas « non récupéré » pour le CPE.
        // Passe 3 (point 3) : l'heure limite ne vaut que pour la première nuit d'une absence du vendredi (une absence
        // d'un autre jour n'a pas d'heure de récupération) ; une absence dont la nuit est couverte par une sortie
        // « partie » est ignorée ; un cas déjà ouvert ou clos n'est pas rouvert. L'élève, lui, est resté attendu (pas de
        // départ constaté = attendu) : l'appel du soir le compte.
        let sans_depart: Vec<(Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT ab.eleve_id, ab.du_nuit FROM ent_internat_absence ab \
              WHERE ab.etablissement_id = $1 AND ab.statut IN ('annoncee', 'acceptee') AND ab.depart_effectif IS NULL \
                AND ab.mode = 'annonce' AND ab.du_nuit = $2 AND extract(dow FROM ab.du_nuit) = 5 \
                AND NOT EXISTS (SELECT 1 FROM ent_internat_sortie so \
                                 WHERE so.etablissement_id = ab.etablissement_id AND so.eleve_id = ab.eleve_id \
                                   AND so.statut IN ('partie', 'rentree') \
                                   AND so.debut <= ent_internat_ancre_nuit($1, $2) \
                                   AND COALESCE(so.retour_effectif, so.fin_ajustee) >= \
                                       ((($2::date + 1) + TIME '06:00') AT TIME ZONE ent_internat_fuseau($1))) \
              ORDER BY ab.du_nuit, ab.id",
        )
        .bind(etab)
        .bind(nuit)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if !sans_depart.is_empty() {
            let limite = instant_mural_tx(&mut tx, etab, nuit, p.heure_recuperation_vendredi).await?;
            if maintenant >= limite + seuil {
                for (eleve, _du) in sans_depart {
                    if eleve_majeur_tx(&mut tx, etab, eleve, nuit).await? || cas_existe_tx(&mut tx, etab, eleve, nuit, "non_recupere").await? {
                        continue;
                    }
                    if self.ouvrir_cas_tx(&mut tx, etab, None, None, eleve, nuit, "non_recupere", maintenant).await?.is_some() {
                        sortie.cas_non_recupere += 1;
                    }
                }
            }
        }
        // 3. Expiration des sorties dont la fin est passée sans départ.
        let expirees: Vec<(Uuid, Uuid, String, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
            "UPDATE ent_internat_sortie SET statut = 'expiree', avant_blocage = NULL, modifie_le = $2 \
              WHERE etablissement_id = $1 AND statut IN ('demandee', 'autorisee', 'suspendue', 'bloquee') AND fin_ajustee < $2 \
          RETURNING id, eleve_id, type_code, debut, fin_ajustee",
        )
        .bind(etab)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, _eleve, _type, _debut, _fin) in &expirees {
            fait_tx(&mut tx, etab, Cible::Sortie(*id), "expiration", "systeme", None, maintenant, None, None, None, None).await?;
        }
        sortie.sorties_expirees = expirees.len() as i32;
        // 3 bis. F4 : tiers autorisés dont l'échéance approche (alerte unique, sans motif ni demandeur).
        sortie.tiers_a_echeance = self.alerter_tiers_echeance_tx(&mut tx, etab, aujourdhui, maintenant).await?;
        // 4. Revue des sorties à venir : interdiction survenue, correspondant suspendu.
        let bilan = self.reviser_sorties_tx(&mut tx, etab, None, aujourdhui, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok((sortie, bilan))
    }

    /// Revue des sorties et des absences à venir (d'un élève, ou de tous si `eleve = None`), à la minute et à chaque
    /// événement qui peut changer une autorisation. **Règle générale (passe 3) : un acte tombe avec son auteur** (`actes.rs`,
    /// une seule évaluation pour tous les objets) ; puis, pour une sortie autorisée : la personne désignée est devenue
    /// interdite ou non autorisée (socle 4), le correspondant n'est plus valide, l'autorisation annuelle (socle 18) est
    /// retirée ou n'a plus de décideur valable → la sortie est **suspendue** (l'élève redevient attendu), avec trace ; une
    /// interdiction **sans personne identifiée** la **bloque** (la direction tranche). Les absences annoncées sont revues de
    /// la même façon (bloquées). Les départs déjà constatés ouvrent un cas pour le CPE.
    ///
    /// **Charge** : la revue ne fait aucune requête par ligne. Les lectures sont regroupées (une requête par établissement
    /// et par type : lignes verrouillées `FOR UPDATE`, journal des faits, remises, correspondants, verdicts annuels par
    /// usage, interdictions) ; seules les écritures, rares, sont faites ligne à ligne. Chaque écriture est gardée par l'état
    /// attendu (`AND statut = …`) : une sortie partie entre-temps n'est pas touchée et ne fait pas échouer la transaction.
    /// Renvoie le nombre de sorties et d'absences suspendues, bloquées ou remises à la décision du CPE.
    pub(crate) async fn reviser_sorties_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        eleve: Option<Uuid>,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<RevueBilan, EntDbError> {
        let mut suspendues = 0;
        let mut echecs = 0;
        // Les correspondants désignés par un compte désormais visé sont suspendus d'abord (même règle) ; les sorties qui
        // reposent sur eux le sont aussitôt par le contrôle de validité ci-dessous.
        suspendre_correspondants_auteur_tx(conn, etab, eleve, jour, maintenant, &mut echecs).await?;
        let usages: HashMap<String, String> =
            types_tx(conn, etab).await?.into_iter().map(|t| (t.code, t.usage_accord_code)).collect();
        // ── Sorties : demandées, autorisées, suspendues ou bloquées ; lignes verrouillées.
        let lignes: Vec<SortieLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_SORTIE} FROM ent_internat_sortie \
              WHERE etablissement_id = $1 AND statut IN ('demandee', 'autorisee', 'suspendue', 'bloquee') \
                AND fin_ajustee > $2 AND ($3::uuid IS NULL OR eleve_id = $3) \
              ORDER BY debut, id FOR UPDATE"
        ))
        .bind(etab)
        .bind(maintenant)
        .bind(eleve)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let objets: Vec<Objet<'_>> = lignes.iter().map(Objet::Sortie).collect();
        let chutes = actes_qui_tombent_tx(conn, etab, &objets, jour).await?;
        // Lectures groupées pour les sorties autorisées dont l'acte ne tombe pas.
        let candidates: Vec<&SortieLigne> = lignes.iter().filter(|s| s.statut == "autorisee" && !chutes.contains_key(&s.id)).collect();
        let ids_candidates: Vec<Uuid> = candidates.iter().map(|s| s.id).collect();
        let jours_debut: HashMap<Uuid, NaiveDate> = if ids_candidates.is_empty() {
            HashMap::new()
        } else {
            sqlx::query_as::<_, (Uuid, NaiveDate)>(
                "SELECT id, (debut AT TIME ZONE ent_internat_fuseau($1))::date FROM ent_internat_sortie \
                  WHERE etablissement_id = $1 AND id = ANY($2)",
            )
            .bind(etab)
            .bind(&ids_candidates)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?
            .into_iter()
            .collect()
        };
        let pour_sortie = |s: &SortieLigne| jours_debut.get(&s.id).copied().unwrap_or(jour).max(jour);
        let couples: Vec<(Uuid, Uuid, NaiveDate)> =
            candidates.iter().filter_map(|s| s.avec_personne_id.map(|p| (s.eleve_id, p, pour_sortie(s)))).collect();
        let remises = remises_en_lot_tx(conn, etab, &couples).await?;
        let correspondants: Vec<Uuid> = candidates.iter().filter_map(|s| s.correspondant_id).collect();
        let valides: HashSet<Uuid> = if correspondants.is_empty() {
            HashSet::new()
        } else {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM ent_internat_correspondant WHERE etablissement_id = $1 AND id = ANY($2) AND statut = 'valide'",
            )
            .bind(etab)
            .bind(&correspondants)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?
            .into_iter()
            .collect()
        };
        let mut annuels: HashMap<(String, NaiveDate, Uuid), String> = HashMap::new();
        let mut groupes: HashMap<(String, NaiveDate), Vec<Uuid>> = HashMap::new();
        for s in candidates.iter().filter(|s| s.par_autorisation_annuelle) {
            if let Some(u) = usages.get(&s.type_code) {
                groupes.entry((u.clone(), pour_sortie(s))).or_default().push(s.eleve_id);
            }
        }
        for ((usage, pour), eleves) in groupes {
            for (e, v) in verdicts_annuels(self, conn, etab, &usage, pour, &eleves).await {
                annuels.insert((usage.clone(), pour, e), v);
            }
        }
        let seuls_a_verifier = sans_cible_en_lot_tx(
            conn,
            etab,
            &candidates.iter().filter(|s| s.seul).map(|s| (s.eleve_id, pour_sortie(s))).collect::<Vec<_>>(),
        )
        .await?;
        for s in &lignes {
            // Passe 4 (R7) : une erreur sur une sortie n'annule pas la revue des autres : chaque sortie est revue dans un
            // savepoint ; en cas d'échec, seul son travail est annulé et l'échec est compté.
            let mut ouvert = false;
            let revue: Result<bool, EntDbError> = async {
            if let Some(chute) = chutes.get(&s.id) {
                ouvrir_savepoint_revue(conn, &mut ouvert).await?;
                return appliquer_sortie_tx(conn, etab, s, chute, maintenant).await;
            }
            if s.statut != "autorisee" {
                return Ok(false);
            }
            let pour = pour_sortie(s);
            let mut motif: Option<&str> = None;
            if let Some(p) = s.avec_personne_id {
                let mut r = remises.get(&(s.eleve_id, p, pour)).map_or(Remise::NonAutorisee, |r| match r {
                    Remise::Autorisee(x) => Remise::Autorisee(x.clone()),
                    Remise::Interdite => Remise::Interdite,
                    Remise::InterditeSansCible => Remise::InterditeSansCible,
                    Remise::NonAutorisee => Remise::NonAutorisee,
                });
                if r == Remise::InterditeSansCible {
                    // Levée ponctuelle de la direction pour cette sortie ? (cas rare : lecture au cas par cas)
                    r = remise_pour_tx(conn, etab, Some(Cible::Sortie(s.id)), s.eleve_id, p, pour).await?;
                }
                match r {
                    Remise::Interdite => motif = Some("interdiction_remise"),
                    Remise::InterditeSansCible => {
                        // La direction tranche : la sortie accompagnée est bloquée, jamais suspendue d'elle-même.
                        ouvrir_savepoint_revue(conn, &mut ouvert).await?;
                        return bloquer_sortie_systeme_tx(conn, etab, s, "interdiction_remise", maintenant).await;
                    }
                    Remise::NonAutorisee => motif = Some("personne_non_autorisee"),
                    Remise::Autorisee(_) => {}
                }
            }
            if motif.is_none() {
                if let Some(c) = s.correspondant_id {
                    if !valides.contains(&c) {
                        motif = Some("correspondant_suspendu");
                    }
                }
            }
            // Moyen 3 : une sortie qui repose sur l'autorisation annuelle voit son verdict du socle 18 relu à chaque
            // revue : retirée, expirée ou sans décideur valable, elle est suspendue (et ne reprend jamais d'elle-même).
            if motif.is_none() && s.par_autorisation_annuelle {
                let verdict = usages
                    .get(&s.type_code)
                    .and_then(|u| annuels.get(&(u.clone(), pour, s.eleve_id)))
                    .map_or("inconnu", String::as_str);
                if verdict != "autorise" {
                    motif = Some("autorisation_revoquee");
                }
            }
            // M-A : interdiction de remise sans personne identifiée : un mineur ne part pas seul sans décision de la
            // direction (sortie bloquée).
            if motif.is_none()
                && s.seul
                && seuls_a_verifier.contains(&s.eleve_id)
                && seul_arrete_par_interdiction_tx(conn, etab, Cible::Sortie(s.id), s.eleve_id, pour).await?
            {
                ouvrir_savepoint_revue(conn, &mut ouvert).await?;
                return bloquer_sortie_systeme_tx(conn, etab, s, "interdiction_remise", maintenant).await;
            }
            let Some(motif) = motif else { return Ok(false) };
            ouvrir_savepoint_revue(conn, &mut ouvert).await?;
            let n = sqlx::query(
                "UPDATE ent_internat_sortie SET statut = 'suspendue', modifie_le = $3 \
                  WHERE etablissement_id = $1 AND id = $2 AND statut = 'autorisee'",
            )
            .bind(etab)
            .bind(s.id)
            .bind(maintenant)
            .execute(&mut *conn)
            .await
            .map_err(db)?
            .rows_affected();
            if n == 0 {
                return Ok(false);
            }
            fait_tx(conn, etab, Cible::Sortie(s.id), "suspension", "systeme", None, maintenant, Some(motif), None, s.avec_personne_id, None).await?;
            evenements_sortie_tx(conn, etab, None, s, types::INTERNAT_SORTIE_DECIDEE, "suspendue", motif, true).await?;
            Ok(true)
            }
            .await;
            fin_savepoint_revue(conn, ouvert, revue, &mut suspendues, &mut echecs).await?;
        }
        // ── C-1 : les absences annoncées dont l'élève n'est pas encore parti sont revues de la même façon : la personne
        // désignée est devenue interdite (ou n'est plus autorisée), l'autorisation annuelle d'un départ seul a disparu, ou
        // un acte tombe avec son auteur → l'absence est **bloquée** pour la direction (l'élève redevient attendu, sauf pour
        // la nuit déjà commencée).
        let absences: Vec<AbsenceLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_ABSENCE} FROM ent_internat_absence \
              WHERE etablissement_id = $1 AND statut IN ('demandee', 'annoncee', 'acceptee', 'bloquee') AND depart_effectif IS NULL \
                AND au_nuit >= ent_nuit_etab($1, $2) AND ($3::uuid IS NULL OR eleve_id = $3) \
              ORDER BY du_nuit, id FOR UPDATE"
        ))
        .bind(etab)
        .bind(maintenant)
        .bind(eleve)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let objets: Vec<Objet<'_>> = absences.iter().map(Objet::Absence).collect();
        let chutes = actes_qui_tombent_tx(conn, etab, &objets, jour).await?;
        let en_vigueur: Vec<&AbsenceLigne> =
            absences.iter().filter(|a| !matches!(a.statut.as_str(), "bloquee" | "demandee") && !chutes.contains_key(&a.id)).collect();
        let couples: Vec<(Uuid, Uuid, NaiveDate)> =
            en_vigueur.iter().filter_map(|a| a.recupere_par_personne_id.map(|p| (a.eleve_id, p, a.du_nuit.max(jour)))).collect();
        let remises = remises_en_lot_tx(conn, etab, &couples).await?;
        let sans_personne: Vec<(Uuid, NaiveDate)> =
            en_vigueur.iter().filter(|a| a.recupere_par_personne_id.is_none()).map(|a| (a.eleve_id, a.du_nuit.max(jour))).collect();
        let mineurs: HashSet<(Uuid, NaiveDate)> = if sans_personne.is_empty() {
            HashSet::new()
        } else {
            let (es, js): (Vec<Uuid>, Vec<NaiveDate>) = sans_personne.iter().copied().unzip();
            sqlx::query_as::<_, (Uuid, NaiveDate, bool)>(
                "SELECT k.e, k.j, ent_internat_eleve_majeur($1, k.e, k.j) FROM unnest($2::uuid[], $3::date[]) AS k(e, j)",
            )
            .bind(etab)
            .bind(&es)
            .bind(&js)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?
            .into_iter()
            .filter_map(|(e, j, majeur)| (!majeur).then_some((e, j)))
            .collect()
        };
        let mut annuels_absence: HashMap<(NaiveDate, Uuid), String> = HashMap::new();
        if let Some(usage) = usages.get("week_end") {
            let mut groupes: HashMap<NaiveDate, Vec<Uuid>> = HashMap::new();
            for a in en_vigueur.iter().filter(|a| a.recupere_par_personne_id.is_none() && mineurs.contains(&(a.eleve_id, a.du_nuit.max(jour)))) {
                if week_end_complet(a.du_nuit, a.au_nuit) {
                    groupes.entry(a.du_nuit.max(jour)).or_default().push(a.eleve_id);
                }
            }
            for (pour, eleves) in groupes {
                for (e, v) in verdicts_annuels(self, conn, etab, usage, pour, &eleves).await {
                    annuels_absence.insert((pour, e), v);
                }
            }
        }
        let seuls_absence = sans_cible_en_lot_tx(conn, etab, &sans_personne).await?;
        for ab in &absences {
            let mut ouvert = false;
            let revue: Result<bool, EntDbError> = async {
            if let Some(chute) = chutes.get(&ab.id) {
                ouvrir_savepoint_revue(conn, &mut ouvert).await?;
                return appliquer_absence_tx(conn, etab, ab, chute, maintenant).await;
            }
            // Passe 4 (R4) : une absence « demandée » (pas encore acceptée) est revue elle aussi pour la règle générale
            // ci-dessus ; les autres contrôles (personne, autorisation annuelle) ne valent qu'une fois acceptée.
            if matches!(ab.statut.as_str(), "bloquee" | "demandee") {
                return Ok(false);
            }
            let pour = ab.du_nuit.max(jour);
            let mineur = mineurs.contains(&(ab.eleve_id, pour));
            let mut motif: Option<&str> = match ab.recupere_par_personne_id {
                Some(p) => {
                    let mut r = remises.get(&(ab.eleve_id, p, pour)).map_or(Remise::NonAutorisee, |r| match r {
                        Remise::Autorisee(x) => Remise::Autorisee(x.clone()),
                        Remise::Interdite => Remise::Interdite,
                        Remise::InterditeSansCible => Remise::InterditeSansCible,
                        Remise::NonAutorisee => Remise::NonAutorisee,
                    });
                    if r == Remise::InterditeSansCible {
                        r = remise_pour_tx(conn, etab, Some(Cible::Absence(ab.id)), ab.eleve_id, p, pour).await?;
                    }
                    match r {
                        Remise::Interdite | Remise::InterditeSansCible => Some("interdiction_remise"),
                        Remise::NonAutorisee => Some("personne_non_autorisee"),
                        Remise::Autorisee(_) => None,
                    }
                }
                None if mineur => {
                    // F-B : l'accord annuel ne couvre que le week-end (vendredi au plus dimanche).
                    let verdict = annuels_absence.get(&(pour, ab.eleve_id)).map_or("inconnu", String::as_str);
                    (!week_end_complet(ab.du_nuit, ab.au_nuit) || verdict != "autorise").then_some("autorisation_revoquee")
                }
                None => None,
            };
            // M-A : interdiction de remise sans personne identifiée : un mineur ne part pas seul sans décision.
            if motif.is_none()
                && ab.recupere_par_personne_id.is_none()
                && seuls_absence.contains(&ab.eleve_id)
                && seul_arrete_par_interdiction_tx(conn, etab, Cible::Absence(ab.id), ab.eleve_id, pour).await?
            {
                motif = Some("interdiction_remise");
            }
            let Some(motif) = motif else { return Ok(false) };
            ouvrir_savepoint_revue(conn, &mut ouvert).await?;
            // M-B : l'absence n'a donné lieu à aucun départ constaté (la revue ne regarde que celles-là) : l'élève reste
            // attendu, bloquer l'absence ne change donc rien à l'attendu ; le CPE est alerté.
            bloquer_absence_systeme_tx(conn, etab, ab, motif, maintenant).await
            }
            .await;
            fin_savepoint_revue(conn, ouvert, revue, &mut suspendues, &mut echecs).await?;
        }
        // ── Déjà parti : un auteur ou la personne remise devenu visé ouvre un cas pour le CPE (rien n'est défait).
        self.signaler_departs_tx(conn, etab, eleve, jour, maintenant, &mut echecs).await?;
        // ── Reprise : une sortie suspendue PAR LE SYSTÈME pour une cause qui a disparu (interdiction levée, correspondant
        // repris) redevient autorisée ; une suspension due au retrait d'une autorisation annuelle ne reprend jamais seule
        // (le titulaire la redonne, ou une nouvelle demande est faite).
        let suspendues_systeme: Vec<SortieLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_SORTIE} FROM ent_internat_sortie s \
              WHERE s.etablissement_id = $1 AND s.statut = 'suspendue' AND s.fin_ajustee > $2 \
                AND ($3::uuid IS NULL OR s.eleve_id = $3) \
                AND (SELECT f.qualite = 'systeme' AND f.motif_code IN ('interdiction_remise', 'personne_non_autorisee', 'correspondant_suspendu') \
                       FROM ent_internat_i3_fait f WHERE f.etablissement_id = s.etablissement_id AND f.sortie_id = s.id \
                        AND f.acte IN ('suspension', 'revocation', 'reprise') ORDER BY f.a DESC, f.cree_le DESC LIMIT 1) \
              ORDER BY s.debut, s.id FOR UPDATE"
        ))
        .bind(etab)
        .bind(maintenant)
        .bind(eleve)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for s in suspendues_systeme {
            sqlx::query("SAVEPOINT revue_objet").execute(&mut *conn).await.map_err(db)?;
            let revue: Result<bool, EntDbError> = async {
            let personne_ok = match s.avec_personne_id {
                Some(p) => matches!(remise_possible_tx(conn, etab, s.eleve_id, p, jour).await?, Remise::Autorisee(_)),
                None => true,
            };
            let correspondant_ok = match s.correspondant_id {
                Some(c) => sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_internat_correspondant WHERE etablissement_id = $1 AND id = $2 AND statut = 'valide')",
                )
                .bind(etab)
                .bind(c)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?,
                None => true,
            };
            if !(personne_ok && correspondant_ok) {
                return Ok(false);
            }
            let n = sqlx::query(
                "UPDATE ent_internat_sortie SET statut = 'autorisee', modifie_le = $3 \
                  WHERE etablissement_id = $1 AND id = $2 AND statut = 'suspendue'",
            )
            .bind(etab)
            .bind(s.id)
            .bind(maintenant)
            .execute(&mut *conn)
            .await
            .map_err(erreur_sortie)?
            .rows_affected();
            if n == 0 {
                return Ok(false);
            }
            fait_tx(conn, etab, Cible::Sortie(s.id), "reprise", "systeme", None, maintenant, None, None, None, None).await?;
            evenements_sortie_tx(conn, etab, None, &s, types::INTERNAT_SORTIE_DECIDEE, "autorisee", "cause_disparue", true).await?;
            Ok(false)
            }
            .await;
            // Une reprise n'est pas une suspension : elle ne compte pas dans `suspendues`.
            let mut sans_effet = 0;
            fin_savepoint_revue(conn, true, revue, &mut sans_effet, &mut echecs).await?;
        }
        Ok(RevueBilan { suspendues, echecs })
    }

    /// Tâche planifiée (chaque minute) : veille des sorties de chaque établissement, à l'heure murale de son fuseau.
    pub async fn internat_sorties_tache(&self) -> Result<usize, EntDbError> {
        let mut total = 0;
        for etab in crate::tenant_tx::etablissements_systeme(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("internat_sorties_veille"),
        )
        .await
        .map_err(db)?
        {
            match self.veille_sorties_bilan_a(etab, Utc::now()).await {
                Ok((v, bilan)) => {
                    total += (v.cas_retard_retour + v.cas_non_recupere + v.sorties_expirees + bilan.suspendues) as usize;
                    suivre_echecs_veille(etab, bilan.echecs > 0);
                }
                Err(e) => {
                    tracing::warn!(nature = %nature_erreur(&e), "veille des sorties d'internat non faite pour un établissement");
                    suivre_echecs_veille(etab, true);
                }
            }
        }
        Ok(total)
    }
}

// ───────────────────────── Correspondants (§14.2) ─────────────────────────

#[derive(sqlx::FromRow)]
struct CorrespondantLigne {
    id: Uuid,
    eleve_id: Uuid,
    personne_id: Uuid,
    nom: String,
    prenom: String,
    lundi: bool,
    mardi: bool,
    mercredi: bool,
    jeudi: bool,
    vendredi: bool,
    samedi: bool,
    dimanche: bool,
    du: NaiveDate,
    au: Option<NaiveDate>,
    statut: String,
}

const SELECT_CORRESPONDANT: &str = "SELECT c.id, c.eleve_id, c.personne_id, p.nom, p.prenom, c.lundi, c.mardi, c.mercredi, \
     c.jeudi, c.vendredi, c.samedi, c.dimanche, c.du, c.au, c.statut \
     FROM ent_internat_correspondant c JOIN ent_personne p ON p.etablissement_id = c.etablissement_id AND p.id = c.personne_id";

impl From<CorrespondantLigne> for CorrespondantDto {
    fn from(l: CorrespondantLigne) -> Self {
        CorrespondantDto {
            id: l.id,
            eleve_id: l.eleve_id,
            personne_id: l.personne_id,
            nom: l.nom,
            prenom: l.prenom,
            lundi: l.lundi,
            mardi: l.mardi,
            mercredi: l.mercredi,
            jeudi: l.jeudi,
            vendredi: l.vendredi,
            samedi: l.samedi,
            dimanche: l.dimanche,
            du: l.du,
            au: l.au,
            statut: l.statut,
        }
    }
}

impl PgEnt {
    /// Désigne un correspondant d'internat : un titulaire (ou l'élève majeur) choisit, parmi les personnes
    /// autorisées à récupérer l'élève (socle 4, contact validé), celle qui l'accueille certains jours. Une personne
    /// visée par une interdiction de remise ne peut pas l'être. L'adresse n'est jamais portée.
    pub async fn designer_correspondant_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &CorrespondantSaisieDto,
        maintenant: DateTime<Utc>,
    ) -> Result<CorrespondantDto, EntDbError> {
        if !(b.lundi || b.mardi || b.mercredi || b.jeudi || b.vendredi || b.samedi || b.dimanche) {
            return Err(refus("indiquer au moins un jour d'accueil"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let qualite = match self.famille_tx(&mut tx, etab, actor, role, b.eleve_id, jour).await? {
            Some(Famille { qualite: q @ (Qualite::Titulaire | Qualite::EleveMajeur), .. }) => q,
            Some(_) => return Err(EntDbError::Forbidden),
            None => {
                self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], b.eleve_id).await?;
                Qualite::Personnel
            }
        };
        let du = b.du.unwrap_or(jour);
        if b.au.is_some_and(|a| a < du) {
            return Err(refus("la fin de la désignation précède son début"));
        }
        // La personne est une personne autorisée à récupérer l'élève (contact validé du socle 4), pas un titulaire.
        let contact: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_contact_urgence c \
               JOIN ent_personne p ON p.etablissement_id = c.etablissement_id AND p.id = c.personne_id \
              WHERE c.etablissement_id = $1 AND c.eleve_id = $2 AND c.personne_id = $3 AND c.peut_recuperer \
                AND c.statut = 'valide' AND p.fusionnee_dans IS NULL)",
        )
        .bind(etab)
        .bind(b.eleve_id)
        .bind(b.personne_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !contact {
            return Err(refus(
                "le correspondant doit être une personne autorisée à récupérer l'élève (dossier de l'élève, socle 4)",
            ));
        }
        if !matches!(remise_possible_tx(&mut tx, etab, b.eleve_id, b.personne_id, jour).await?, Remise::Autorisee(_)) {
            return Err(refus("cette personne ne peut pas être désignée"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_correspondant \
             (id, etablissement_id, eleve_id, personne_id, lundi, mardi, mercredi, jeudi, vendredi, samedi, dimanche, \
              du, au, statut, designe_par_user_id, cree_le, modifie_le) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,'valide',$14,$15,$15)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.eleve_id)
        .bind(b.personne_id)
        .bind(b.lundi)
        .bind(b.mardi)
        .bind(b.mercredi)
        .bind(b.jeudi)
        .bind(b.vendredi)
        .bind(b.samedi)
        .bind(b.dimanche)
        .bind(du)
        .bind(b.au)
        .bind(actor)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if matches!(&e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23505")) {
                conflit("cette personne est déjà désignée correspondant de cet élève")
            } else {
                db(e)
            }
        })?;
        let q = if qualite == Qualite::Personnel { "cpe" } else { qualite.code() };
        fait_tx(&mut tx, etab, Cible::Correspondant(id), "designation", q, Some(actor), maintenant, None, None, Some(b.personne_id), None).await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_CORRESPONDANT,
            AggregateKind::Internat,
            id,
            Some(b.eleve_id),
            json!({ "action": "designe" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let l: CorrespondantLigne = sqlx::query_as(&format!("{SELECT_CORRESPONDANT} WHERE c.etablissement_id = $1 AND c.id = $2"))
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l.into())
    }

    /// INT-168 : correspondants d'un élève et leurs jours (sans adresse) : sa famille ou le personnel de l'internat.
    pub async fn correspondants_eleve_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<CorrespondantDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let famille = self.famille_lecture_tx(&mut tx, etab, actor, role, eleve, jour).await?;
        if famille.is_none() {
            self.personnel_exiger(etab, actor, role, &[Action::InternatLire, Action::InternatSortiesDecider, Action::InternatAppel], eleve)
                .await?;
            crate::internat_nuit::journaliser_lecture_eleves_tx(&mut tx, etab, actor, "correspondants", Some(eleve), None, &[eleve], maintenant)
                .await?;
        }
        let lignes: Vec<CorrespondantLigne> = sqlx::query_as(&format!(
            "{SELECT_CORRESPONDANT} WHERE c.etablissement_id = $1 AND c.eleve_id = $2 AND c.statut <> 'termine' \
              ORDER BY p.nom, p.prenom, c.id"
        ))
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(lignes.into_iter().map(Into::into).collect())
    }

    /// CIT-33 : le CPE suspend un correspondant qui ne répond plus (ou le reprend). Les sorties à venir chez lui
    /// sont suspendues ; les titulaires sont avertis (événement). `suspendre = false` : reprise.
    pub async fn suspendre_correspondant_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        suspendre: bool,
        b: &CorrespondantSuspensionDto,
        maintenant: DateTime<Utc>,
    ) -> Result<CorrespondantDto, EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleve: Uuid = sqlx::query_scalar(
            "SELECT eleve_id FROM ent_internat_correspondant WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(DomainError::NotFound)?;
        self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], eleve).await?;
        let (de, vers, acte) = if suspendre { ("valide", "suspendu", "suspension") } else { ("suspendu", "valide", "reprise") };
        let n = sqlx::query(
            "UPDATE ent_internat_correspondant SET statut = $4, modifie_le = $5 \
              WHERE etablissement_id = $1 AND id = $2 AND statut = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(de)
        .bind(vers)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("l'état de ce correspondant ne permet pas cette opération"));
        }
        // Passe 4 (R2) : reprendre un correspondant suspendu parce que son auteur est visé (interdiction ou perte de droit)
        // est la décision qui porte sur la chute : motif écrit obligatoire, tracé sous le code de la chute.
        let mut motif_code = suspendre.then_some("correspondant_suspendu");
        if !suspendre {
            let derniere: Option<Option<String>> = sqlx::query_scalar(
                "SELECT motif_code FROM ent_internat_i3_fait \
                  WHERE etablissement_id = $1 AND correspondant_id = $2 AND acte = 'suspension' \
                  ORDER BY a DESC, cree_le DESC LIMIT 1",
            )
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if derniere.flatten().as_deref() == Some("interdiction_auteur") {
                if motif.is_none() {
                    return Err(refus(
                        "motif obligatoire : ce correspondant a été suspendu parce que la personne qui l'a désigné n'a plus le droit d'agir pour l'élève ; le reprendre est une décision explicite, tracée",
                    ));
                }
                motif_code = Some("interdiction_auteur");
            }
        }
        fait_tx(&mut tx, etab, Cible::Correspondant(id), acte, "cpe", Some(actor), maintenant, motif_code, motif.as_deref(), None, None)
            .await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_CORRESPONDANT,
            AggregateKind::Internat,
            id,
            Some(eleve),
            json!({ "action": vers }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        self.reviser_sorties_tx(&mut tx, etab, Some(eleve), jour, maintenant).await?;
        let l: CorrespondantLigne = sqlx::query_as(&format!("{SELECT_CORRESPONDANT} WHERE c.etablissement_id = $1 AND c.id = $2"))
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l.into())
    }

    /// Passe 4 (R5) : le personnel **requalifie** un contact d'urgence « peut récupérer » : il l'a vu et en répond. Sans cela,
    /// un contact déclaré par une personne devenue visée (interdiction ou perte de droit, chaîne de déclarants comprise)
    /// n'est plus remis à l'élève (`ent_internat_remise`). Fait daté et signé (`requalifie_le`), motif écrit obligatoire ;
    /// la requalification ne vaut que pour les pertes qui la précèdent. Les sorties suspendues pour ce motif reprennent
    /// à la revue qui suit, dans la même transaction.
    pub async fn requalifier_contact_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        contact_id: Uuid,
        b: &CorrespondantSuspensionDto,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let motif = texte_court(b.motif_texte.as_deref())?.ok_or_else(|| {
            refus("motif obligatoire : requalifier un contact d'urgence est une décision explicite du personnel, tracée")
        })?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ligne: Option<(Uuid, bool, String)> = sqlx::query_as(
            "SELECT eleve_id, peut_recuperer, statut FROM ent_eleve_contact_urgence \
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(contact_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (eleve, peut_recuperer, statut) = ligne.ok_or(DomainError::NotFound)?;
        self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], eleve).await?;
        if !peut_recuperer || statut != "valide" {
            return Err(conflit("seul un contact validé autorisé à récupérer l'élève se requalifie"));
        }
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        // Limitée aux élèves INTERNES (séjour d'internat couvrant le jour) : ce pouvoir est celui de l'internat, pas du dossier.
        let interne: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_internat_sejour WHERE etablissement_id = $1 AND eleve_id = $2 \
               AND du <= $3 AND (au IS NULL OR au >= $3))",
        )
        .bind(etab)
        .bind(eleve)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !interne {
            return Err(refus("la requalification d'un contact est limitée aux élèves internes"));
        }
        // Conflit d'intérêt : l'agent ne requalifie pas un contact dont il est la personne ou le déclarant (chaîne de fusions
        // comprise), ni pour un élève dont il est lui-même proche (lien avec l'élève).
        let conflit_interet: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_contact_urgence c \
                WHERE c.etablissement_id = $1 AND c.id = $2 AND ( \
                      EXISTS (SELECT 1 FROM ent_personne me WHERE me.etablissement_id = $1 AND me.user_id = $3 \
                                AND (me.id IN (SELECT ent_internat_personne_chaine($1, c.personne_id)) \
                                  OR me.id IN (SELECT ent_internat_personne_chaine($1, c.declare_par_personne_id)))) \
                   OR EXISTS (SELECT 1 FROM ent_eleve_lien l WHERE l.etablissement_id = $1 AND l.eleve_id = c.eleve_id \
                                AND (l.user_id = $3 OR l.personne_id IN (SELECT id FROM ent_personne \
                                                                          WHERE etablissement_id = $1 AND user_id = $3)))))",
        )
        .bind(etab)
        .bind(contact_id)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if conflit_interet {
            return Err(refus(
                "conflit d'intérêt : vous ne pouvez pas requalifier un contact dont vous êtes la personne ou le déclarant, ni pour un élève dont vous êtes un proche ; un autre agent doit le faire",
            ));
        }
        // Fait de requalification du journal I3 (ajout seul, daté, signé, motif écrit) : aucune colonne n'est ajoutée à la
        // table du dossier (copie de fusion intacte) ; la porte « contact » le lit (`ent_internat_contact_suspect`).
        sqlx::query(
            "INSERT INTO ent_internat_i3_fait (id, etablissement_id, eleve_id, personne_id, acte, qualite, par_user_id, a, motif_texte) \
             SELECT $1, $2, c.eleve_id, c.personne_id, 'requalification', 'cpe', $4, $5, $6 \
               FROM ent_eleve_contact_urgence c WHERE c.etablissement_id = $2 AND c.id = $3",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(contact_id)
        .bind(actor)
        .bind(maintenant)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_CORRESPONDANT,
            AggregateKind::Internat,
            contact_id,
            Some(eleve),
            json!({ "action": "contact_requalifie" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        self.reviser_sorties_tx(&mut tx, etab, Some(eleve), jour, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Fin d'une désignation (titulaire, élève majeur, CPE) : les sorties à venir chez lui sont suspendues.
    pub async fn terminer_correspondant_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<CorrespondantDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleve: Uuid = sqlx::query_scalar(
            "SELECT eleve_id FROM ent_internat_correspondant WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(DomainError::NotFound)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let qualite = match self.famille_tx(&mut tx, etab, actor, role, eleve, jour).await? {
            Some(Famille { qualite: q @ (Qualite::Titulaire | Qualite::EleveMajeur), .. }) => q,
            Some(_) => return Err(EntDbError::Forbidden),
            None => {
                self.personnel_exiger(etab, actor, role, &[Action::InternatSortiesDecider], eleve).await?;
                Qualite::Personnel
            }
        };
        let n = sqlx::query(
            "UPDATE ent_internat_correspondant SET statut = 'termine', au = GREATEST(du, $3), modifie_le = $4 \
              WHERE etablissement_id = $1 AND id = $2 AND statut <> 'termine'",
        )
        .bind(etab)
        .bind(id)
        .bind(jour)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("cette désignation est déjà terminée"));
        }
        let q = if qualite == Qualite::Personnel { "cpe" } else { qualite.code() };
        fait_tx(&mut tx, etab, Cible::Correspondant(id), "fin", q, Some(actor), maintenant, None, None, None, None).await?;
        publier(
            &mut tx,
            etab,
            types::INTERNAT_CORRESPONDANT,
            AggregateKind::Internat,
            id,
            Some(eleve),
            json!({ "action": "termine" }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        self.reviser_sorties_tx(&mut tx, etab, Some(eleve), jour, maintenant).await?;
        let l: CorrespondantLigne = sqlx::query_as(&format!("{SELECT_CORRESPONDANT} WHERE c.etablissement_id = $1 AND c.id = $2"))
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l.into())
    }
}

// ───────────────────────── Autorisations annuelles (socle 18) ─────────────────────────

impl PgEnt {
    /// État des autorisations annuelles de sortie d'un élève, type par type (verdict du socle 18, jamais
    /// « autorise » par défaut) : sa famille ou le personnel qui lit l'internat. Sans motif ni décideur (R-FC39).
    pub async fn autorisations_eleve_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<AutorisationsEleveDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let famille = self.famille_lecture_tx(&mut tx, etab, actor, role, eleve, jour).await?;
        if famille.is_none() {
            self.personnel_exiger(etab, actor, role, &[Action::InternatLire, Action::InternatSortiesDecider], eleve).await?;
        }
        let types_actifs: Vec<SortieTypeDto> = types_tx(&mut tx, etab).await?.into_iter().filter(|t| t.actif).collect();
        let mut lignes = Vec::new();
        for t in types_actifs.iter().filter(|t| t.autorisation != "ponctuelle") {
            let verdict = verdict_annuel(self, &mut tx, etab, eleve, &t.usage_accord_code, jour).await;
            lignes.push(AutorisationTypeDto { type_code: t.code.clone(), usage_code: t.usage_accord_code.clone(), verdict });
        }
        tx.commit().await.map_err(db)?;
        Ok(AutorisationsEleveDto { eleve_id: eleve, types: lignes })
    }

    /// INT-162 : demande des autorisations annuelles de sortie pour des élèves (rentrée, nouvel interne). Le socle 18
    /// les recueille (campagne, signature) ; le 41 n'émet que la demande, sans nom ni texte.
    pub async fn demander_autorisations_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &AutorisationsDemandeDto,
    ) -> Result<AutorisationsDemandeesDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::InternatGerer).await?;
        if b.eleve_ids.is_empty() || b.eleve_ids.len() > 500 {
            return Err(refus("indiquer de 1 à 500 élèves"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let connus: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)")
            .bind(etab)
            .bind(&b.eleve_ids)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let distincts = b.eleve_ids.iter().collect::<HashSet<_>>().len() as i64;
        if connus != distincts {
            return Err(DomainError::NotFound.into());
        }
        publier(
            &mut tx,
            etab,
            types::INTERNAT_AUTORISATIONS_DEMANDEES,
            AggregateKind::Internat,
            Uuid::new_v4(),
            None,
            json!({ "objet": OBJET_ACCORD_SORTIE, "nb": distincts, "eleve_ids": b.eleve_ids }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(AutorisationsDemandeesDto { demandes: distincts as i32 })
    }

    /// INT-163 : un titulaire (ou l'élève majeur) révoque l'autorisation annuelle de sortie : **effet immédiat**, la
    /// décision du socle 18 est retirée et les sorties à venir qui s'appuyaient sur elle sont **suspendues** (l'élève
    /// redevient attendu). Les sorties dont le titulaire a donné un accord propre restent valables.
    pub async fn revoquer_autorisation_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &RevocationDto,
        maintenant: DateTime<Utc>,
    ) -> Result<RevocationResultatDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let qualite = match self.famille_tx(&mut tx, etab, actor, role, b.eleve_id, jour).await? {
            Some(Famille { qualite: q @ (Qualite::Titulaire | Qualite::EleveMajeur), .. }) => q,
            _ => return Err(EntDbError::Forbidden),
        };
        let usages: Vec<String> = match &b.usages {
            Some(u) if !u.is_empty() => u.clone(),
            _ => types_tx(&mut tx, etab).await?.into_iter().filter(|t| t.autorisation != "ponctuelle").map(|t| t.usage_accord_code).collect(),
        };
        // Retrait dans le socle 18 (effet immédiat, sans délai de propagation) ; sans objet activé, rien à retirer.
        let verdict = verdict_annuel(self, &mut tx, etab, b.eleve_id, usages.first().map_or("sortie_mercredi", String::as_str), jour).await;
        tx.commit().await.map_err(db)?;
        if verdict != "inconnu" {
            self.accords_retirer(etab, actor, OBJET_ACCORD_SORTIE, Some(b.eleve_id), b.usages.clone().filter(|u| !u.is_empty()))
                .await?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let a_suspendre: Vec<SortieLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_SORTIE} FROM ent_internat_sortie s \
              WHERE s.etablissement_id = $1 AND s.eleve_id = $2 AND s.par_autorisation_annuelle \
                AND s.statut IN ('demandee', 'autorisee') AND s.fin_ajustee > $3 \
                AND EXISTS (SELECT 1 FROM ent_internat_sortie_type t WHERE t.etablissement_id = s.etablissement_id \
                             AND t.code = s.type_code AND t.usage_accord_code = ANY($4)) \
              ORDER BY s.debut, s.id FOR UPDATE"
        ))
        .bind(etab)
        .bind(b.eleve_id)
        .bind(maintenant)
        .bind(&usages)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut n = 0;
        for s in &a_suspendre {
            if s.statut == "autorisee" {
                sqlx::query("UPDATE ent_internat_sortie SET statut = 'suspendue', modifie_le = $3 WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(s.id)
                    .bind(maintenant)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                fait_tx(&mut tx, etab, Cible::Sortie(s.id), "revocation", qualite.code(), Some(actor), maintenant, Some("autorisation_revoquee"), None, None, None)
                    .await?;
                evenements_sortie_tx(&mut tx, etab, Some(actor), s, types::INTERNAT_SORTIE_DECIDEE, "suspendue", "autorisation_revoquee", true).await?;
                n += 1;
            } else {
                // Demandée : l'accord annuel qui lui était compté disparaît, elle attend de nouveau la famille.
                sqlx::query(
                    "UPDATE ent_internat_sortie SET accord_titulaire_le = NULL, par_autorisation_annuelle = false, modifie_le = $3 \
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(s.id)
                .bind(maintenant)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                fait_tx(&mut tx, etab, Cible::Sortie(s.id), "revocation", qualite.code(), Some(actor), maintenant, Some("autorisation_revoquee"), None, None, None)
                    .await?;
            }
        }
        publier(
            &mut tx,
            etab,
            types::INTERNAT_AUTORISATION_REVOQUEE,
            AggregateKind::Internat,
            Uuid::new_v4(),
            Some(b.eleve_id),
            json!({ "usages": usages, "sorties_suspendues": n }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(RevocationResultatDto { sorties_suspendues: n })
    }
}

// ───────────────────────── Historique ─────────────────────────

impl PgEnt {
    /// Historique (faits en ajout seul) d'une absence (`objet = "absence"`), d'une sortie (`"sortie"`) ou d'un
    /// correspondant (`"correspondant"`) : la famille de l'élève ou le personnel de l'internat dans sa portée.
    /// Voir [`FaitDto`] pour ce que chacun voit ; la lecture du personnel est journalisée (R-IT38).
    pub async fn historique_a(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        objet: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<FaitDto>, EntDbError> {
        let (table, colonne) = match objet {
            "absence" => ("ent_internat_absence", "absence_id"),
            "sortie" => ("ent_internat_sortie", "sortie_id"),
            "correspondant" => ("ent_internat_correspondant", "correspondant_id"),
            _ => return Err(refus("objet d'historique inconnu (absence, sortie, correspondant)")),
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleve: Uuid = sqlx::query_scalar(&format!("SELECT eleve_id FROM {table} WHERE etablissement_id = $1 AND id = $2"))
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or(DomainError::NotFound)?;
        let jour = jour_local_tx(&mut tx, etab, maintenant).await?;
        let famille = self.famille_lecture_tx(&mut tx, etab, actor, role, eleve, jour).await?;
        if famille.is_none() {
            self.personnel_exiger(etab, actor, role, &[Action::InternatLire, Action::InternatSortiesDecider, Action::InternatAppel], eleve)
                .await?;
            crate::internat_nuit::journaliser_lecture_eleves_tx(&mut tx, etab, actor, "sorties", Some(eleve), None, &[eleve], maintenant)
                .await?;
        }
        let rows = sqlx::query(&format!(
            "SELECT acte, qualite, a, par_user_id, motif_code, motif_texte, personne_id, jusqu_a FROM ent_internat_i3_fait \
              WHERE etablissement_id = $1 AND {colonne} = $2 ORDER BY a, cree_le, id"
        ))
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let personnel = famille.is_none();
        Ok(rows
            .into_iter()
            // Passe 3 : le signalement au CPE d'un acte dont l'auteur est visé (déjà parti) n'est jamais lisible de la famille.
            .filter(|r| personnel || r.get::<String, _>("acte") != "signalement")
            .map(|r| {
                let acte: String = r.get("acte");
                let qualite_fait: String = r.get("qualite");
                let texte_visible = personnel
                    || (matches!(acte.as_str(), "refus" | "tranchee_maintenue")
                        && matches!(qualite_fait.as_str(), "cpe" | "direction"));
                // F-D : la famille et l'élève ne lisent jamais qu'une interdiction existe (remise ou auteur) : code générique.
                let motif_code: Option<String> = r.get("motif_code");
                let motif_code = match (personnel, motif_code.as_deref()) {
                    (false, Some("interdiction_remise")) => Some("personne_non_autorisee".to_string()),
                    (false, Some("interdiction_auteur")) => Some("autre".to_string()),
                    _ => motif_code,
                };
                FaitDto {
                    par_user_id: personnel.then(|| r.get("par_user_id")).flatten(),
                    motif_texte: texte_visible.then(|| r.get("motif_texte")).flatten(),
                    personne_id: personnel.then(|| r.get("personne_id")).flatten(),
                    qualite: qualite_fait,
                    a: r.get("a"),
                    motif_code,
                    jusqu_a: r.get("jusqu_a"),
                    acte,
                }
            })
            .collect())
    }
}
