//! Socle 12, lot AS5 : **familles et notifications** (`docs/socles/12_ASSIDUITE.md` §10.5, §14, R-AS13, R-AS14,
//! R-AS18, R-AS25, §24.1).
//!
//! Tout passe par le **moteur de notifications du socle 8** (`ntf::service::aviser`) : ce lot ne remet,
//! n'expédie et ne résout aucun destinataire lui-même. Il décide **quand** et **pour quelle demi-journée**
//! on avise, et **quand on corrige** :
//!
//! - **avis par demi-journée** : l'appel validé émet `assiduite.absence_ouverte` ; le consommateur
//!   `assiduite.familles` crée **un avis par élève et par demi-journée** (registre
//!   `ent_asd_avis_demi_journee`, un seul avis vivant par demi-journée : une journée d'absence ne donne jamais
//!   plus de deux avis), avec le délai de consolidation du moteur (R131-5 : l'avis part à la fin du délai si
//!   rien ne l'a annulé) ;
//! - **rien n'est envoyé à tort** : une arrivée à l'infirmerie (localisation) ou un retard dans le délai annule
//!   l'avis avant toute remise ; un avis déjà parti puis corrigé (correction d'appel, séance annulée, élève
//!   remis présent) est suivi d'un **démenti** notifié (R-AS14) ; une requalification qui change le sens
//!   (justifiée ↔ non justifiée) est notifiée (`absence_decision`) ;
//! - **parent sous interdiction, lien sans droit** : le moteur relit les droits du lien (socle 4) et les
//!   gels à la création et à chaque remise ; rien n'est décidé ici ;
//! - **relance graduée** (§10.5) d'un épisode sans motif : J+1 rappel, J+3 canal externe, J+5 « sans
//!   réponse », J+8 proposition de qualification « sans motif » (jamais qualifiée d'elle-même, R-AS18) ;
//! - **contacts tracés** avec la famille (ajout seul) et **courriers postaux** préparés, **décidés** par une
//!   personne puis notés « postés » : ScolIA n'en envoie aucun d'elle-même (R-AS18).
//!
//! Les événements ne portent que des codes et des identifiants (R-AS25) ; aucun texte libre ne traverse le
//! moteur. Le jour est le jour métier de l'établissement (`ent_aujourdhui_etab`), jamais la date UTC.
//!
//! Bascule progressive (socle 8) : tant que `ent_ntf_nature_etab.via_brique` n'est pas posé pour
//! `vie_scolaire.absence_constatee` (avis par demi-journée et démenti), `…absence_decision` (décisions
//! modifiées) et `…relance_justificatif` (relance graduée), l'ancien circuit prévient comme avant.

use crate::ntf::catalogue::{
    ABSENCE_CONSTATEE, ABSENCE_DECISION, ABSENCE_DEMENTI, RELANCE_JUSTIFICATIF,
};
use crate::ntf::contrat::{Param, Urgence, ValeurParam};
use crate::ntf::service::{
    annuler_avis, aviser, reglage_nature, Annulation, DemandeAvis, RefusAvis,
};
use crate::outbox::{Causalite, Consommateur, EvenementDistribue, FutureTraitement, Publier};
use crate::service::{EntDbError, PgEnt};
use crate::structure_calendrier::{calendrier, Cible};
use crate::tenant_tx::TenantTx;
use crate::vie_scolaire::{db, introuvable};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc, Weekday};
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

/// Événements du lot (codes et identifiants seulement, R-AS25).
pub mod types_as5 {
    pub const ABSENCE_OUVERTE: &str = "assiduite.absence_ouverte";
    pub const RELANCE_ENVOYEE: &str = "assiduite.relance_envoyee";
    pub const CONTACT_TRACE: &str = "assiduite.contact_trace";
    pub const COURRIER_PREPARE: &str = "assiduite.courrier_prepare";
}

pub const CONSOMMATEUR_FAMILLES: &str = "assiduite.familles";

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

// ───────────────────────── Demi-journée et jours scolaires ─────────────────────────

/// Demi-journée d'une heure : avant la coupure, le matin.
pub fn periode_de(heure: NaiveTime, coupure: NaiveTime) -> &'static str {
    if heure < coupure {
        "matin"
    } else {
        "apres_midi"
    }
}

/// Coupure matin / après-midi d'un élève un jour donné : grille du socle 5 (STR-35) de sa classe, à défaut la
/// règle d'assiduité datée de ce jour (jamais celle d'aujourd'hui, R-AS15), à défaut midi.
pub(crate) async fn coupure_du_jour(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
) -> Result<NaiveTime, EntDbError> {
    let grille: Option<NaiveTime> = sqlx::query_scalar(
        "SELECT g.coupure_demi_journee
           FROM ent_eleve e
           JOIN ent_classe c ON c.etablissement_id = e.etablissement_id AND c.id = e.classe_id
           LEFT JOIN ent_mef m ON m.id = c.mef_id AND m.etablissement_id = c.etablissement_id
           JOIN ent_grille g ON g.etablissement_id = c.etablissement_id
            AND g.id = ent_grille_pour(c.etablissement_id, $3, c.site_id, COALESCE(c.niveau_id, m.niveau_id))
          WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .flatten();
    if let Some(c) = grille {
        return Ok(c);
    }
    let regle: Option<NaiveTime> = sqlx::query_scalar(
        "SELECT heure_coupure FROM ent_asd_regle WHERE etablissement_id = $1 AND valide_du <= $2
          ORDER BY valide_du DESC LIMIT 1",
    )
    .bind(etab)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(regle.unwrap_or_else(|| NaiveTime::from_hms_opt(12, 0, 0).expect("midi")))
}

/// La demi-journée compte-t-elle encore une absence **à avertir** ? Il faut une séance absente non retirée
/// (épisode non retiré) qui n'est pas couverte par une situation « ailleurs » (localisation, exclusion de cours,
/// sortie… : jamais une absence, R-AS4). Une situation sans heure couvre la journée entière.
async fn periode_vivante(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    periode: &str,
    coupure: NaiveTime,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1
              FROM ent_asd_absence_seance s
              JOIN ent_asd_absence a ON a.etablissement_id = s.etablissement_id AND a.id = s.absence_id
             WHERE s.etablissement_id = $1 AND s.eleve_id = $2 AND s.jour = $3
               AND s.retiree_le IS NULL AND a.etat <> 'retire'
               AND CASE WHEN s.kind = 'demi_journee' THEN s.periode
                        WHEN s.heure_debut < $5 THEN 'matin' ELSE 'apres_midi' END = $4
               AND NOT EXISTS (
                    SELECT 1 FROM ent_asd_situation x
                     WHERE x.etablissement_id = s.etablissement_id AND x.eleve_id = s.eleve_id
                       AND x.etat <> 'annulee'
                       AND x.nature NOT IN ('declaration', 'retard_annonce', 'autorisation', 'sortie_anticipee')
                       AND x.debut_jour <= s.jour AND x.fin_jour >= s.jour
                       AND CASE WHEN s.kind = 'seance'
                                THEN (x.debut_jour < s.jour OR x.debut_heure IS NULL OR x.debut_heure < s.heure_fin)
                                 AND (x.fin_jour > s.jour OR x.fin_heure IS NULL OR x.fin_heure > s.heure_debut)
                                ELSE (x.debut_jour < s.jour OR x.debut_heure IS NULL)
                                 AND (x.fin_jour > s.jour OR x.fin_heure IS NULL) END))",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(jour)
    .bind(periode)
    .bind(coupure)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)
}

/// N-ième jour scolaire **strictement après** `depuis` (calendrier du socle 5 pour la classe ; repli : lundi au
/// vendredi quand l'établissement n'a défini aucune année scolaire).
pub(crate) async fn jour_scolaire_apres(
    conn: &mut PgConnection,
    etab: Uuid,
    depuis: NaiveDate,
    n: i64,
    classe_id: Option<Uuid>,
) -> Result<NaiveDate, EntDbError> {
    debug_assert!(n >= 1);
    let du = depuis + Duration::days(1);
    let au = depuis + Duration::days(60);
    let annees: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1)",
    )
    .bind(etab)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let ouvres: Vec<NaiveDate> = if annees {
        let cible = Cible {
            site_id: None,
            niveau_id: None,
            classe_id,
            groupe_id: None,
        };
        calendrier::jours_ouvres(conn, etab, du, au, &cible).await?
    } else {
        du.iter_days()
            .take(61)
            .filter(|d| !matches!(d.weekday(), Weekday::Sat | Weekday::Sun))
            .collect()
    };
    Ok(ouvres
        .get((n - 1) as usize)
        .copied()
        .unwrap_or(depuis + Duration::days(n)))
}

// ───────────────────────── Rythme de relance (S-473) ─────────────────────────

/// Délai de consolidation (minutes, 0 à 30, R131-5) et rythme de relance de la **règle d'assiduité en vigueur** à
/// un jour (`ent_asd_regle`, versionnée et jamais rétroactive, R-AS15) ; à défaut de version, les valeurs du code.
pub(crate) async fn reglage_famille(
    conn: &mut PgConnection,
    etab: Uuid,
    jour: NaiveDate,
) -> Result<(i64, String), EntDbError> {
    let r: Option<(i32, String)> = sqlx::query_as(
        "SELECT delai_consolidation_min, rythme_relance FROM ent_asd_regle
          WHERE etablissement_id = $1 AND valide_du <= $2 ORDER BY valide_du DESC LIMIT 1",
    )
    .bind(etab)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let (delai, rythme) = r.unwrap_or((10, "standard".to_string()));
    Ok((i64::from(delai.clamp(0, 30)), rythme))
}

pub const RYTHMES: [&str; 3] = ["court", "standard", "long"];
pub const ETAPES: [&str; 4] = [
    "rappel",
    "canal_externe",
    "sans_reponse",
    "proposition_sans_motif",
];

/// Jours scolaires, après la dernière séance manquée, de chaque étape (un seul paramètre, D-023).
pub fn delais_rythme(rythme: &str) -> Option<[i64; 4]> {
    match rythme {
        "court" => Some([1, 2, 3, 5]),
        "standard" => Some([1, 3, 5, 8]),
        "long" => Some([2, 5, 8, 12]),
        _ => None,
    }
}

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AvisDemiJourneeAsd {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub periode: String,
    pub generation: i16,
    pub statut: String,
    pub cause_code: Option<String>,
    pub avis_id: Uuid,
    pub dementi_avis_id: Option<Uuid>,
    /// État de l'avis dans le moteur (`planifie`, `en_cours`, `termine`, `annule`).
    pub avis_statut: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamFamille {
    /// Délai de consolidation avant l'avis d'absence, en minutes (0 à 30, R131-5).
    pub delai_consolidation_min: i16,
    pub rythme_relance: String,
    /// Les trois bascules du moteur : avis par demi-journée et démenti, décision modifiée, relance graduée.
    pub avis_demi_journee_actif: bool,
    pub decision_active: bool,
    pub relance_active: bool,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct BilanRelance {
    pub rappels: usize,
    pub canaux_externes: usize,
    pub sans_reponse: usize,
    pub propositions: usize,
    pub sautees: usize,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RelanceAsd {
    pub id: Uuid,
    pub absence_id: Uuid,
    pub eleve_id: Uuid,
    pub etape: String,
    pub jour_prevu: NaiveDate,
    pub statut: String,
    pub saut_code: Option<String>,
    pub cree_le: DateTime<Utc>,
}

/// Épisode « sans réponse » ou à proposer en « sans motif » : la file de la vie scolaire.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct EpisodeSansReponse {
    pub absence_id: Uuid,
    pub eleve_id: Uuid,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    pub derniere_etape: String,
    pub courrier_en_cours: bool,
}

pub const CANAUX_CONTACT: [&str; 5] = ["telephone", "sms", "courriel", "rencontre", "courrier"];
pub const ISSUES_CONTACT: [&str; 6] = [
    "joint",
    "message_laisse",
    "ne_repond_pas",
    "numero_faux",
    "envoye",
    "rencontre_faite",
];

#[derive(Debug, Clone, Deserialize)]
pub struct TracerContact {
    pub eleve_id: Uuid,
    pub absence_id: Option<Uuid>,
    pub personne_id: Option<Uuid>,
    pub canal: String,
    /// `sortant` (défaut) ou `entrant` (la famille a appelé).
    pub sens: Option<String>,
    pub issue: String,
    /// Note interne courte : jamais visible de la famille ni du professeur (R-AS22).
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ContactAsd {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub absence_id: Option<Uuid>,
    pub personne_id: Option<Uuid>,
    pub canal: String,
    pub sens: String,
    pub issue: String,
    pub note_interne: Option<String>,
    pub auteur_user_id: Uuid,
    pub tache_numero_le: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
}

/// Ce que la famille voit de ses contacts avec la vie scolaire (jamais la note, ni l'auteur).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CommunicationFamille {
    pub type_communication: String,
    pub canal: String,
    pub cree_le: DateTime<Utc>,
}

pub const NATURES_COURRIER: [&str; 3] = ["relance_absence", "avis_absence", "autre"];
pub const ABANDONS_COURRIER: [&str; 4] = [
    "motif_recu",
    "erreur",
    "famille_jointe",
    "decision_direction",
];

#[derive(Debug, Clone, Deserialize)]
pub struct PreparerCourrier {
    pub eleve_id: Uuid,
    pub absence_id: Option<Uuid>,
    pub personne_id: Option<Uuid>,
    pub nature: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CourrierAsd {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub absence_id: Option<Uuid>,
    pub personne_id: Option<Uuid>,
    pub nature: String,
    pub statut: String,
    pub prepare_par: Uuid,
    pub prepare_le: DateTime<Utc>,
    pub valide_par: Option<Uuid>,
    pub valide_le: Option<DateTime<Utc>>,
    pub poste_par: Option<Uuid>,
    pub poste_le: Option<DateTime<Utc>>,
    pub abandon_code: Option<String>,
}

// ───────────────────────── Émission d'événements ─────────────────────────

async fn emettre(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate_id: Uuid,
    eleve_id: Uuid,
    payload: serde_json::Value,
    acteur: Option<Uuid>,
) -> Result<(), EntDbError> {
    conn.publier_avec(
        MetierEventDraft {
            etablissement_id: etab,
            type_name: type_name.to_string(),
            aggregate: AggregateKind::Absence,
            aggregate_id,
            eleve_id: Some(eleve_id),
            payload,
            actor_user_id: acteur.map(|u| u.to_string()),
        },
        Causalite::default(),
    )
    .await?;
    Ok(())
}

// ───────────────────────── Consommateur « assiduite.familles » ─────────────────────────

pub struct ConsommateurFamilles;

const TYPES_ECOUTES: &[&str] = &[
    types_as5::ABSENCE_OUVERTE,
    crate::assiduite_as2::types_appel::ABSENCE_DEMENTIE,
    crate::assiduite_as2::types_appel::LOCALISE,
    crate::assiduite_as3::types_as3::ABSENCE_REQUALIFIEE,
    ent_events::types::ELEVE_ABSENCE_ANNULEE_PRESENT,
    ent_events::types::RETARD_DECLARE,
    ent_events::types::ELEVE_DECEDE,
];

impl Consommateur for ConsommateurFamilles {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_FAMILLES
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_ECOUTES
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(async move { traiter(tx, evt).await.map_err(|e| e.to_string()) })
    }
}

async fn traiter(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), EntDbError> {
    let e = evt.event();
    let etab = evt.etablissement_id();
    let Some(eleve_id) = e.eleve_id else {
        return Ok(());
    };
    // Un seul traitement à la fois par élève (les événements d'un même élève ont des agrégats distincts que l'outbox
    // peut distribuer en parallèle) : sinon un avis peut naître pendant que sa correction ne voit encore rien.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("asd-familles:{etab}:{eleve_id}"))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    // Relecture dans l'établissement de l'événement : un élève d'un autre établissement est introuvable.
    let existe: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
    if existe.is_none() {
        return Ok(());
    }
    let jour = |cle: &str| {
        e.payload
            .get(cle)
            .and_then(|v| v.as_str())
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    };
    match e.type_name.as_str() {
        types_as5::ABSENCE_OUVERTE => {
            let prevenue = e
                .payload
                .get("prevenue")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let appel_id = e
                .payload
                .get("appel_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            if let (Some(appel_id), Some(j)) = (appel_id, jour("jour")) {
                avis_absence_ouverte(
                    tx,
                    etab,
                    eleve_id,
                    appel_id,
                    j,
                    prevenue,
                    e.created_at,
                    e.id,
                )
                .await
            } else {
                Ok(())
            }
        }
        t if t == crate::assiduite_as2::types_appel::ABSENCE_DEMENTIE => match jour("jour") {
            Some(j) => {
                reconcilier_jour(tx, etab, eleve_id, j, "correction", e.created_at, e.id).await
            }
            None => Ok(()),
        },
        t if t == crate::assiduite_as2::types_appel::LOCALISE => match jour("jour") {
            Some(j) => {
                reconcilier_jour(tx, etab, eleve_id, j, "localisation", e.created_at, e.id).await
            }
            None => Ok(()),
        },
        t if t == ent_events::types::ELEVE_ABSENCE_ANNULEE_PRESENT => {
            // L'ancien circuit « remettre présent » (jour de l'absence ancienne).
            let j = match jour("debut") {
                Some(j) => Some(j),
                None => sqlx::query_scalar::<_, NaiveDate>(
                    "SELECT debut FROM ent_absence WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(e.aggregate_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?,
            };
            match j {
                Some(j) => {
                    reconcilier_jour(tx, etab, eleve_id, j, "present", e.created_at, e.id).await
                }
                None => Ok(()),
            }
        }
        t if t == ent_events::types::RETARD_DECLARE => match jour("jour") {
            Some(j) => reconcilier_jour(tx, etab, eleve_id, j, "retard", e.created_at, e.id).await,
            None => Ok(()),
        },
        t if t == ent_events::types::ELEVE_DECEDE => {
            // Le décès gèle le sujet (consommateur d'arrêt) : les avis sont annulés ; le registre suit.
            sqlx::query(
                "UPDATE ent_asd_avis_demi_journee SET statut = 'annule', cause_code = 'sujet_gele', modifie_le = now()
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'programme'",
            )
            .bind(etab)
            .bind(eleve_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            Ok(())
        }
        t if t == crate::assiduite_as3::types_as3::ABSENCE_REQUALIFIEE => {
            decision_modifiee(
                tx,
                etab,
                eleve_id,
                e.aggregate_id,
                &e.payload,
                e.created_at,
                e.id,
            )
            .await
        }
        _ => Ok(()),
    }
}

/// Avis d'absence d'une demi-journée (R-AS13) : un seul avis vivant par élève et demi-journée.
#[allow(clippy::too_many_arguments)]
async fn avis_absence_ouverte(
    tx: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    appel_id: Uuid,
    jour: NaiveDate,
    prevenue: bool,
    effet_le: DateTime<Utc>,
    cause_event_id: Uuid,
) -> Result<(), EntDbError> {
    let r = reglage_nature(tx, etab, ABSENCE_CONSTATEE.code)
        .await
        .map_err(db)?;
    if !r.via_brique || r.desactivee {
        return Ok(());
    }
    // Absence déjà annoncée par la famille : elle est déjà prévenue (comme l'ancien circuit).
    if prevenue {
        return Ok(());
    }
    let heure: Option<NaiveTime> = sqlx::query_scalar(
        "SELECT COALESCE(a.heure_debut, c.heure_debut)
           FROM ent_appel a LEFT JOIN ent_edt_creneau c ON c.etablissement_id = a.etablissement_id AND c.id = a.creneau_id
          WHERE a.etablissement_id = $1 AND a.id = $2",
    )
    .bind(etab)
    .bind(appel_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?
    .flatten();
    let Some(heure) = heure else {
        return Ok(());
    };
    let coupure = coupure_du_jour(tx, etab, eleve_id, jour).await?;
    let periode = periode_de(heure, coupure);
    // La demi-journée compte-t-elle encore une absence à avertir (déjà corrigée, déjà localisée…) ?
    if !periode_vivante(tx, etab, eleve_id, jour, periode, coupure).await? {
        return Ok(());
    }
    let vivant: Option<(Uuid, String, Option<String>)> = sqlx::query_as(
        "SELECT l.id, a.statut, a.annule_motif
           FROM ent_asd_avis_demi_journee l
           JOIN ent_ntf_avis a ON a.etablissement_id = l.etablissement_id AND a.id = l.avis_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.jour = $3 AND l.periode = $4 AND l.statut = 'programme'
          FOR UPDATE OF l",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(jour)
    .bind(periode)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?;
    if let Some((id, avis_statut, motif)) = vivant {
        if avis_statut != "annule" {
            // Un avis vivant couvre déjà cette demi-journée : pas de second avis (R-AS13).
            return Ok(());
        }
        // Le moteur a retiré cet avis (retard dans le délai, décès) sans que le registre l'ait encore suivi : on le
        // met à jour avant de décider d'un nouvel avis, pour que la demi-journée ne reste pas bloquée.
        let cause = match motif.as_deref() {
            Some("annulee_par_evenement") => "retard",
            Some("deces") => "sujet_gele",
            _ => "correction",
        };
        marquer(tx, etab, id, "annule", cause, None).await?;
    }
    let generation: i16 = sqlx::query_scalar(
        "SELECT (COALESCE(max(generation), 0) + 1)::smallint FROM ent_asd_avis_demi_journee
          WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3 AND periode = $4",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(jour)
    .bind(periode)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    // Prévenir maintenant (AS6) a déjà alerté la famille aujourd'hui en urgence : pas de second message ordinaire.
    let urgence_faite: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_asd_prevenir WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3)",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(jour)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    if urgence_faite {
        return Ok(());
    }
    // Délai de consolidation de la règle en vigueur ce jour-là (R-AS15), jamais celui d'un autre établissement.
    let (mut delai_min, _) = reglage_famille(tx, etab, jour).await?;
    // Absence après présence (R-N67) : la famille n'est prévenue qu'après 15 minutes au moins, le temps que la vie
    // scolaire localise l'élève (infirmerie, permanence…).
    if crate::assiduite_as6::etait_present_plus_tot(tx, etab, eleve_id, jour, heure, Some(appel_id)).await? {
        delai_min = crate::assiduite_as6::delai_apres_presence(delai_min);
    }
    let parametres: Vec<Param> = vec![
        ("jour", ValeurParam::Jour(jour)),
        ("periode", ValeurParam::Texte(periode.to_string())),
    ];
    let cree = match aviser(
        tx,
        DemandeAvis {
            etablissement_id: etab,
            nature: &ABSENCE_CONSTATEE,
            eleve_id: Some(eleve_id),
            cle: format!("asd:{eleve_id}:{jour}:{periode}:{generation}"),
            parametres,
            emetteur_brique: Some("vie_scolaire"),
            source_type: "assiduite_demi_journee".into(),
            source_id: Some(appel_id),
            cause_event_id: Some(cause_event_id),
            effet_le,
            urgence_plus: None,
            delai_min: Some(delai_min),
        },
    )
    .await
    {
        Ok(c) => c,
        // Élève décédé (gel) ou nature désactivée depuis : aucun avis, rien à tracer.
        Err(RefusAvis::SujetGele | RefusAvis::NatureDesactivee) => return Ok(()),
        Err(RefusAvis::Base(e)) => return Err(db(e)),
        Err(_) => {
            return Err(invalide(
                "avis d'absence refusé par le moteur de notifications",
            ))
        }
    };
    sqlx::query(
        "INSERT INTO ent_asd_avis_demi_journee (id, etablissement_id, eleve_id, jour, periode, generation, avis_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve_id)
    .bind(jour)
    .bind(periode)
    .bind(generation)
    .bind(cree.avis_id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Remet le registre d'accord avec les faits d'un jour : avis à annuler (rien n'est parti) ou à démentir
/// (l'avis est parti). Idempotent : une demi-journée déjà traitée ne l'est plus.
async fn reconcilier_jour(
    tx: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    cause: &str,
    maintenant: DateTime<Utc>,
    cause_event_id: Uuid,
) -> Result<(), EntDbError> {
    let lignes: Vec<(Uuid, String, Uuid)> = sqlx::query_as(
        "SELECT id, periode, avis_id FROM ent_asd_avis_demi_journee
          WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3 AND statut = 'programme'
          ORDER BY periode, generation FOR UPDATE",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(jour)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    if lignes.is_empty() {
        return Ok(());
    }
    let coupure = coupure_du_jour(tx, etab, eleve_id, jour).await?;
    for (id, periode, avis_id) in lignes {
        let (avis_statut, motif): (String, Option<String>) = sqlx::query_as(
            "SELECT statut, annule_motif FROM ent_ntf_avis WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(avis_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if avis_statut == "annule" {
            // Déjà retiré par le moteur (retard dans le délai, décès) : le registre suit.
            let c = match motif.as_deref() {
                Some("annulee_par_evenement") => "retard",
                Some("deces") => "sujet_gele",
                _ => "correction",
            };
            marquer(tx, etab, id, "annule", c, None).await?;
            continue;
        }
        if periode_vivante(tx, etab, eleve_id, jour, &periode, coupure).await? {
            continue;
        }
        match annuler_avis(tx, etab, avis_id, "absence_corrigee", maintenant)
            .await
            .map_err(db)?
        {
            Annulation::Annule | Annulation::Introuvable => {
                marquer(tx, etab, id, "annule", cause, None).await?
            }
            Annulation::DejaRemis { .. } => {
                // L'avis est parti : un démenti suit (R-AS14), avec la même demi-journée.
                let dementi = match aviser(
                    tx,
                    DemandeAvis {
                        etablissement_id: etab,
                        nature: &ABSENCE_DEMENTI,
                        eleve_id: Some(eleve_id),
                        cle: format!("asd:dementi:{id}"),
                        parametres: vec![
                            ("jour", ValeurParam::Jour(jour)),
                            ("periode", ValeurParam::Texte(periode.clone())),
                        ],
                        emetteur_brique: Some("vie_scolaire"),
                        source_type: "assiduite_dementi".into(),
                        source_id: Some(id),
                        cause_event_id: Some(cause_event_id),
                        effet_le: maintenant,
                        urgence_plus: None,
                        // Le démenti n'attend pas : il corrige un avis déjà parti.
                        delai_min: Some(0),
                    },
                )
                .await
                {
                    Ok(c) => Some(c.avis_id),
                    Err(RefusAvis::SujetGele | RefusAvis::NatureDesactivee) => None,
                    Err(RefusAvis::Base(e)) => return Err(db(e)),
                    Err(_) => {
                        return Err(invalide("démenti refusé par le moteur de notifications"))
                    }
                };
                marquer(tx, etab, id, "dementi", cause, dementi).await?;
            }
        }
    }
    Ok(())
}

pub(crate) async fn marquer(
    tx: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    statut: &str,
    cause: &str,
    dementi: Option<Uuid>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_asd_avis_demi_journee SET statut = $3, cause_code = $4, dementi_avis_id = $5, modifie_le = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .bind(statut)
    .bind(cause)
    .bind(dementi)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Requalification qui change le sens (justifiée ↔ non justifiée) : la famille en est informée (U0).
async fn decision_modifiee(
    tx: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    absence_id: Uuid,
    payload: &serde_json::Value,
    effet_le: DateTime<Utc>,
    cause_event_id: Uuid,
) -> Result<(), EntDbError> {
    if payload.get("sens_change").and_then(|v| v.as_bool()) != Some(true) {
        return Ok(());
    }
    let r = reglage_nature(tx, etab, ABSENCE_DECISION.code)
        .await
        .map_err(db)?;
    if !r.via_brique || r.desactivee {
        return Ok(());
    }
    let jour_debut: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT jour_debut FROM ent_asd_absence WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(absence_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?;
    let Some(jour_debut) = jour_debut else {
        return Ok(());
    };
    let justifiee = matches!(
        payload.get("qualification").and_then(|v| v.as_str()),
        Some("reputee_legitime" | "excuse_valable")
    );
    match aviser(
        tx,
        DemandeAvis {
            etablissement_id: etab,
            nature: &ABSENCE_DECISION,
            eleve_id: Some(eleve_id),
            cle: format!("asd:decision:{cause_event_id}"),
            parametres: vec![
                ("jour", ValeurParam::Jour(jour_debut)),
                (
                    "decision",
                    ValeurParam::Texte(
                        if justifiee {
                            "justifiee"
                        } else {
                            "non_justifiee"
                        }
                        .into(),
                    ),
                ),
            ],
            emetteur_brique: Some("vie_scolaire"),
            source_type: "assiduite_decision".into(),
            source_id: Some(absence_id),
            cause_event_id: Some(cause_event_id),
            effet_le,
            urgence_plus: None,
            delai_min: None,
        },
    )
    .await
    {
        Ok(_) | Err(RefusAvis::SujetGele | RefusAvis::NatureDesactivee) => Ok(()),
        Err(RefusAvis::Base(e)) => Err(db(e)),
        Err(_) => Err(invalide(
            "avis de décision refusé par le moteur de notifications",
        )),
    }
}

// ───────────────────────── Relance graduée (§10.5) ─────────────────────────

#[derive(sqlx::FromRow)]
struct Candidat {
    id: Uuid,
    eleve_id: Uuid,
    jour_debut: NaiveDate,
    jour_fin: NaiveDate,
    classe_id: Option<Uuid>,
}

/// Fenêtre de rattrapage : un épisode plus ancien n'est pas relancé (on ne réveille pas l'année passée à
/// l'activation de la relance).
const FENETRE_RELANCE_JOURS: i64 = 45;

impl PgEnt {
    /// Passe de relance d'un établissement au **jour métier** `jour` : chaque épisode sans motif reçoit au plus
    /// **une** étape par passage (la plus avancée qui est due ; les précédentes sont notées « rattrapées »).
    /// Inactive tant que `vie_scolaire.relance_justificatif` n'est pas basculée sur la brique pour
    /// l'établissement. Idempotent : une étape ne part qu'une fois par épisode.
    pub async fn relancer_jour(
        &self,
        etab: Uuid,
        jour: NaiveDate,
        maintenant: DateTime<Utc>,
    ) -> Result<BilanRelance, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut bilan = BilanRelance::default();
        let r = reglage_nature(&mut tx, etab, RELANCE_JUSTIFICATIF.code)
            .await
            .map_err(db)?;
        if !r.via_brique || r.desactivee {
            return Ok(bilan);
        }
        let (_, rythme) = reglage_famille(&mut tx, etab, jour).await?;
        let delais = delais_rythme(&rythme).unwrap_or([1, 3, 5, 8]);
        let candidats: Vec<Candidat> = sqlx::query_as(
            "SELECT a.id, a.eleve_id, a.jour_debut, a.jour_fin, e.classe_id
               FROM ent_asd_absence a
               JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
              WHERE a.etablissement_id = $1 AND a.etat IN ('ouvert', 'a_qualifier') AND a.qualification = 'en_attente'
                AND a.prevenue = false AND a.motif_id IS NULL AND a.nature_code IS NULL
                AND a.jour_fin < $2 AND a.jour_fin >= $2 - $3::int
                AND NOT EXISTS (SELECT 1 FROM ent_asd_absence_declaration d
                                 WHERE d.etablissement_id = a.etablissement_id AND d.absence_id = a.id)
                AND NOT EXISTS (SELECT 1 FROM ent_asd_justificatif j
                                 WHERE j.etablissement_id = a.etablissement_id AND j.absence_id = a.id AND j.presente <> 'retire')
              ORDER BY a.jour_fin, a.id",
        )
        .bind(etab)
        .bind(jour)
        .bind(FENETRE_RELANCE_JOURS as i32)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for c in candidats {
            let faites: Vec<(String,)> = sqlx::query_as(
                "SELECT etape FROM ent_asd_relance WHERE etablissement_id = $1 AND absence_id = $2",
            )
            .bind(etab)
            .bind(c.id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let mut dues: Vec<(usize, NaiveDate)> = Vec::new();
            for (i, etape) in ETAPES.iter().enumerate() {
                if faites.iter().any(|(f,)| f == etape) {
                    continue;
                }
                let prevu =
                    jour_scolaire_apres(&mut tx, etab, c.jour_fin, delais[i], c.classe_id).await?;
                if prevu <= jour {
                    dues.push((i, prevu));
                }
            }
            let Some(&(derniere, prevu_derniere)) = dues.last() else {
                continue;
            };
            for &(i, prevu) in &dues[..dues.len() - 1] {
                enregistrer_relance(
                    &mut tx,
                    etab,
                    &c,
                    ETAPES[i],
                    prevu,
                    "sautee",
                    Some("rattrape"),
                    None,
                )
                .await?;
                bilan.sautees += 1;
            }
            let etape = ETAPES[derniere];
            match etape {
                "rappel" | "canal_externe" => {
                    let issue = aviser(
                        &mut tx,
                        DemandeAvis {
                            etablissement_id: etab,
                            nature: &RELANCE_JUSTIFICATIF,
                            eleve_id: Some(c.eleve_id),
                            cle: format!("asd:relance:{}:{etape}", c.id),
                            parametres: vec![("jour", ValeurParam::Jour(c.jour_debut))],
                            emetteur_brique: Some("vie_scolaire"),
                            source_type: "assiduite_relance".into(),
                            source_id: Some(c.id),
                            cause_event_id: None,
                            effet_le: maintenant,
                            // J+3 : montée à U2, qui ouvre le SMS (si le canal est ouvert) et le courriel.
                            urgence_plus: (etape == "canal_externe").then_some(Urgence::U2),
                            delai_min: None,
                        },
                    )
                    .await;
                    match issue {
                        Ok(a) => {
                            let (statut, saut, avis) = if a.cree && a.retenus.is_empty() {
                                ("sautee", Some("sans_destinataire"), Some(a.avis_id))
                            } else {
                                ("envoyee", None, Some(a.avis_id))
                            };
                            enregistrer_relance(
                                &mut tx,
                                etab,
                                &c,
                                etape,
                                prevu_derniere,
                                statut,
                                saut,
                                avis,
                            )
                            .await?;
                            if statut == "envoyee" {
                                if etape == "rappel" {
                                    bilan.rappels += 1;
                                } else {
                                    bilan.canaux_externes += 1;
                                }
                                emettre(
                                    &mut tx,
                                    etab,
                                    types_as5::RELANCE_ENVOYEE,
                                    c.id,
                                    c.eleve_id,
                                    json!({ "absence_id": c.id, "etape": etape }),
                                    None,
                                )
                                .await?;
                            } else {
                                bilan.sautees += 1;
                            }
                        }
                        Err(RefusAvis::NatureDesactivee) => {
                            enregistrer_relance(
                                &mut tx,
                                etab,
                                &c,
                                etape,
                                prevu_derniere,
                                "sautee",
                                Some("nature_desactivee"),
                                None,
                            )
                            .await?;
                            bilan.sautees += 1;
                        }
                        Err(RefusAvis::SujetGele) => {
                            enregistrer_relance(
                                &mut tx,
                                etab,
                                &c,
                                etape,
                                prevu_derniere,
                                "sautee",
                                Some("sujet_gele"),
                                None,
                            )
                            .await?;
                            bilan.sautees += 1;
                        }
                        Err(RefusAvis::Base(e)) => return Err(db(e)),
                        Err(_) => {
                            return Err(invalide("relance refusée par le moteur de notifications"))
                        }
                    }
                }
                // « Sans réponse » (courrier postal à décider) et proposition « sans motif » : pour la
                // vie scolaire, jamais une écriture à la famille ni une qualification d'office (R-AS18).
                _ => {
                    enregistrer_relance(
                        &mut tx,
                        etab,
                        &c,
                        etape,
                        prevu_derniere,
                        "proposee",
                        None,
                        None,
                    )
                    .await?;
                    if etape == "sans_reponse" {
                        bilan.sans_reponse += 1;
                    } else {
                        bilan.propositions += 1;
                    }
                    emettre(
                        &mut tx,
                        etab,
                        types_as5::RELANCE_ENVOYEE,
                        c.id,
                        c.eleve_id,
                        json!({ "absence_id": c.id, "etape": etape }),
                        None,
                    )
                    .await?;
                }
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(bilan)
    }

    /// Tâche système horaire : relance de chaque établissement au **jour métier** qui lui est propre.
    pub async fn assiduite_relance_tache(&self) -> Result<usize, EntDbError> {
        // Lecture système (sans contexte d'établissement) des établissements qui ont activé la relance.
        let etabs: Vec<(Uuid,)> = crate::tenant_tx::lire_par_etablissement(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("assiduite_relance"),
            "SELECT etablissement_id FROM ent_ntf_nature_etab
              WHERE nature = 'vie_scolaire.relance_justificatif' AND via_brique AND NOT desactivee",
        )
        .await?;
        let mut total = 0;
        for (etab,) in etabs {
            let jour = self.aujourdhui_etablissement(etab).await?;
            // Un établissement en erreur ne bloque pas les suivants (R-09).
            let b = match self.relancer_jour(etab, jour, Utc::now()).await {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(error = %e, "relance d'assiduité non exécutée pour un établissement");
                    continue;
                }
            };
            total += b.rappels + b.canaux_externes + b.sans_reponse + b.propositions;
        }
        Ok(total)
    }
}

#[allow(clippy::too_many_arguments)]
async fn enregistrer_relance(
    tx: &mut PgConnection,
    etab: Uuid,
    c: &Candidat,
    etape: &str,
    jour_prevu: NaiveDate,
    statut: &str,
    saut: Option<&str>,
    avis_id: Option<Uuid>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_asd_relance (id, etablissement_id, absence_id, eleve_id, etape, jour_prevu, statut, saut_code, avis_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT (etablissement_id, absence_id, etape) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(c.id)
    .bind(c.eleve_id)
    .bind(etape)
    .bind(jour_prevu)
    .bind(statut)
    .bind(saut)
    .bind(avis_id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok(())
}

// ───────────────────────── API : paramètres, relances, contacts, courriers ─────────────────────────

impl PgEnt {
    /// Réglages de l'information des familles (`assiduite.suivre`), lus de la **règle d'assiduité en vigueur** (la
    /// direction les change en posant une nouvelle version de la règle : `assiduite.regle_gerer`, AS1) et des
    /// trois bascules du moteur de notifications.
    pub async fn param_famille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<ParamFamille, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (delai, rythme) = reglage_famille(&mut tx, etab, jour).await?;
        let actif = |r: crate::ntf::service::ReglageNature| r.via_brique && !r.desactivee;
        Ok(ParamFamille {
            delai_consolidation_min: delai as i16,
            rythme_relance: rythme,
            avis_demi_journee_actif: actif(
                reglage_nature(&mut tx, etab, ABSENCE_CONSTATEE.code)
                    .await
                    .map_err(db)?,
            ),
            decision_active: actif(
                reglage_nature(&mut tx, etab, ABSENCE_DECISION.code)
                    .await
                    .map_err(db)?,
            ),
            relance_active: actif(
                reglage_nature(&mut tx, etab, RELANCE_JUSTIFICATIF.code)
                    .await
                    .map_err(db)?,
            ),
        })
    }

    /// Avis d'absence d'un élève et leur sort (`assiduite.suivre`) : par demi-journée, avec l'état dans le moteur.
    pub async fn avis_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        du: Option<NaiveDate>,
    ) -> Result<Vec<AvisDemiJourneeAsd>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        self.exiger_eleve_existe(etab, eleve_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as(
            "SELECT l.id, l.eleve_id, l.jour, l.periode, l.generation, l.statut, l.cause_code, l.avis_id,
                    l.dementi_avis_id, a.statut AS avis_statut
               FROM ent_asd_avis_demi_journee l
               JOIN ent_ntf_avis a ON a.etablissement_id = l.etablissement_id AND a.id = l.avis_id
              WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND ($3::date IS NULL OR l.jour >= $3)
              ORDER BY l.jour DESC, l.periode, l.generation LIMIT 200",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(du)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    /// File de la vie scolaire : épisodes « sans réponse » (J+5) ou à proposer « sans motif » (J+8), avec
    /// l'indication d'un courrier déjà en cours (`assiduite.suivre`).
    pub async fn sans_reponse_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<EpisodeSansReponse>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as(
            "SELECT a.id AS absence_id, a.eleve_id, a.jour_debut, a.jour_fin,
                    (SELECT r.etape FROM ent_asd_relance r
                      WHERE r.etablissement_id = a.etablissement_id AND r.absence_id = a.id AND r.statut <> 'sautee'
                        AND r.etape IN ('sans_reponse', 'proposition_sans_motif')
                      ORDER BY CASE r.etape WHEN 'proposition_sans_motif' THEN 2 ELSE 1 END DESC LIMIT 1) AS derniere_etape,
                    EXISTS (SELECT 1 FROM ent_asd_courrier c
                             WHERE c.etablissement_id = a.etablissement_id AND c.absence_id = a.id
                               AND c.statut IN ('prepare', 'valide')) AS courrier_en_cours
               FROM ent_asd_absence a
              WHERE a.etablissement_id = $1 AND a.etat IN ('ouvert', 'a_qualifier')
                AND EXISTS (SELECT 1 FROM ent_asd_relance r
                             WHERE r.etablissement_id = a.etablissement_id AND r.absence_id = a.id
                               AND r.statut <> 'sautee' AND r.etape IN ('sans_reponse', 'proposition_sans_motif'))
              ORDER BY a.jour_fin, a.id LIMIT 500",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    /// Étapes de relance d'un épisode (`assiduite.suivre`).
    pub async fn relances_episode_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<RelanceAsd>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as(
            "SELECT id, absence_id, eleve_id, etape, jour_prevu, statut, saut_code, cree_le FROM ent_asd_relance
              WHERE etablissement_id = $1 AND absence_id = $2 ORDER BY jour_prevu, cree_le",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    // ── Contacts tracés (§14.4) ───────────────────────────────────────────────────────────

    /// Trace un contact avec la famille en un geste (`assiduite.suivre`) : « joint », « message laissé », « ne
    /// répond pas », « numéro faux »… Visible de toute l'équipe en temps réel (fin des doubles appels).
    pub async fn tracer_contact_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        c: TracerContact,
    ) -> Result<ContactAsd, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        if !CANAUX_CONTACT.contains(&c.canal.as_str()) {
            return Err(invalide("canal de contact inconnu"));
        }
        if !ISSUES_CONTACT.contains(&c.issue.as_str()) {
            return Err(invalide("issue de contact inconnue"));
        }
        let sens = c.sens.clone().unwrap_or_else(|| "sortant".into());
        if !matches!(sens.as_str(), "sortant" | "entrant") {
            return Err(invalide("sens de contact inconnu"));
        }
        let note = c.note.as_deref().map(str::trim).filter(|n| !n.is_empty());
        if note.is_some_and(|n| n.chars().count() > 300) {
            return Err(invalide("note de contact trop longue (300 caractères)"));
        }
        let coherent = match c.issue.as_str() {
            "numero_faux" => matches!(c.canal.as_str(), "telephone" | "sms"),
            "envoye" => matches!(c.canal.as_str(), "sms" | "courriel" | "courrier"),
            "rencontre_faite" => c.canal == "rencontre",
            _ => true,
        };
        if !coherent {
            return Err(invalide("issue incompatible avec le canal"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleve: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(c.eleve_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        if eleve.is_none() {
            return Err(introuvable());
        }
        if let Some(a) = c.absence_id {
            let ok: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_asd_absence WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(a)
            .bind(c.eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if ok.is_none() {
                return Err(introuvable());
            }
        }
        if let Some(p) = c.personne_id {
            // La personne jointe est un responsable lié à cet élève.
            let ok: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3
                   AND suspendu_le IS NULL AND (date_fin IS NULL OR date_fin >= ent_aujourdhui_etab(etablissement_id, now())) LIMIT 1",
            )
            .bind(etab)
            .bind(c.eleve_id)
            .bind(p)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if ok.is_none() {
                return Err(introuvable());
            }
        }
        let id = Uuid::new_v4();
        let ligne: ContactAsd = sqlx::query_as(
            "INSERT INTO ent_asd_contact (id, etablissement_id, eleve_id, absence_id, personne_id, canal, sens, issue,
                                          note_interne, auteur_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             RETURNING id, eleve_id, absence_id, personne_id, canal, sens, issue, note_interne, auteur_user_id,
                       tache_numero_le, cree_le",
        )
        .bind(id)
        .bind(etab)
        .bind(c.eleve_id)
        .bind(c.absence_id)
        .bind(c.personne_id)
        .bind(&c.canal)
        .bind(&sens)
        .bind(&c.issue)
        .bind(note)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        emettre(
            &mut tx,
            etab,
            types_as5::CONTACT_TRACE,
            id,
            c.eleve_id,
            json!({ "contact_id": id, "canal": c.canal, "issue": c.issue }),
            Some(acteur),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ligne)
    }

    /// Contacts d'un élève (ou d'un épisode), du plus récent au plus ancien (`assiduite.suivre`).
    pub async fn contacts_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        absence_id: Option<Uuid>,
    ) -> Result<Vec<ContactAsd>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        self.exiger_eleve_existe(etab, eleve_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as(
            "SELECT id, eleve_id, absence_id, personne_id, canal, sens, issue, note_interne, auteur_user_id,
                    tache_numero_le, cree_le
               FROM ent_asd_contact
              WHERE etablissement_id = $1 AND eleve_id = $2 AND ($3::uuid IS NULL OR absence_id = $3)
              ORDER BY cree_le DESC, id LIMIT 200",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    /// Tâches « numéro faux » ouvertes, pour le secrétariat (`assiduite.suivre`).
    pub async fn numeros_faux_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<ContactAsd>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as(
            "SELECT id, eleve_id, absence_id, personne_id, canal, sens, issue, note_interne, auteur_user_id,
                    tache_numero_le, cree_le
               FROM ent_asd_contact
              WHERE etablissement_id = $1 AND issue = 'numero_faux' AND tache_numero_le IS NULL
              ORDER BY cree_le, id LIMIT 200",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    /// Clôt la tâche « numéro faux » une fois la fiche corrigée (une seule fois).
    pub async fn cloturer_numero_faux_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        contact_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE ent_asd_contact SET tache_numero_le = now(), tache_numero_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND issue = 'numero_faux' AND tache_numero_le IS NULL",
        )
        .bind(etab)
        .bind(contact_id)
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

    // ── Courriers postaux (RA-04, R-AS18) ─────────────────────────────────────────────────

    /// Prépare un courrier postal (`assiduite.suivre`). Rien ne part : une personne habilitée le **décide**.
    pub async fn preparer_courrier_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        p: PreparerCourrier,
    ) -> Result<CourrierAsd, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        if !NATURES_COURRIER.contains(&p.nature.as_str()) {
            return Err(invalide("nature de courrier inconnue"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleve: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(p.eleve_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        if eleve.is_none() {
            return Err(introuvable());
        }
        if let Some(a) = p.absence_id {
            let ok: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_asd_absence WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(a)
            .bind(p.eleve_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if ok.is_none() {
                return Err(introuvable());
            }
        }
        if let Some(personne) = p.personne_id {
            let ok: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_eleve_lien WHERE etablissement_id = $1 AND eleve_id = $2 AND personne_id = $3
                   AND suspendu_le IS NULL AND (date_fin IS NULL OR date_fin >= ent_aujourdhui_etab(etablissement_id, now())) LIMIT 1",
            )
            .bind(etab)
            .bind(p.eleve_id)
            .bind(personne)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if ok.is_none() {
                return Err(introuvable());
            }
        }
        let id = Uuid::new_v4();
        let inseree = sqlx::query_as::<_, CourrierAsd>(&format!(
            "INSERT INTO ent_asd_courrier (id, etablissement_id, eleve_id, absence_id, personne_id, nature, prepare_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT DO NOTHING
             RETURNING {COURRIER_COLONNES}"
        ))
        .bind(id)
        .bind(etab)
        .bind(p.eleve_id)
        .bind(p.absence_id)
        .bind(p.personne_id)
        .bind(&p.nature)
        .bind(acteur)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some(c) = inseree else {
            return Err(EntDbError::ConflitDossier(
                "un courrier de cette nature est déjà en cours pour cet épisode",
            ));
        };
        emettre(
            &mut tx,
            etab,
            types_as5::COURRIER_PREPARE,
            id,
            p.eleve_id,
            json!({ "courrier_id": id, "nature": p.nature }),
            Some(acteur),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(c)
    }

    /// **Décide** d'un courrier préparé (`assiduite.signaler` : CPE, direction) : R-AS18, un courrier ne part
    /// qu'après une décision humaine, et pas par celui qui l'a seulement préparé sans ce droit.
    pub async fn valider_courrier_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<CourrierAsd, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSignaler)
            .await?;
        self.avancer_courrier(etab, id, "valide", acteur, None)
            .await
    }

    /// Note que le courrier validé a été **posté** (`assiduite.suivre`).
    pub async fn poster_courrier_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<CourrierAsd, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        self.avancer_courrier(etab, id, "poste", acteur, None).await
    }

    /// Abandonne un courrier non posté, avec un code (`assiduite.suivre`).
    pub async fn abandonner_courrier_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        code: &str,
    ) -> Result<CourrierAsd, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        if !ABANDONS_COURRIER.contains(&code) {
            return Err(invalide("code d'abandon inconnu"));
        }
        self.avancer_courrier(etab, id, "abandonne", acteur, Some(code))
            .await
    }

    async fn avancer_courrier(
        &self,
        etab: Uuid,
        id: Uuid,
        vers: &str,
        acteur: Uuid,
        abandon: Option<&str>,
    ) -> Result<CourrierAsd, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let actuel: Option<(String,)> = sqlx::query_as(
            "SELECT statut FROM ent_asd_courrier WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((statut,)) = actuel else {
            return Err(introuvable());
        };
        let permis = matches!(
            (statut.as_str(), vers),
            ("prepare", "valide") | ("valide", "poste") | ("prepare" | "valide", "abandonne")
        );
        if !permis {
            return Err(EntDbError::ConflitDossier(
                "ce courrier n'est pas dans l'état attendu",
            ));
        }
        // Trois requêtes complètes (le filtre d'établissement est lisible par la garde statique d'isolation).
        let sql = match vers {
            "valide" => {
                "UPDATE ent_asd_courrier SET statut = 'valide', valide_par = $3, valide_le = now()
                  WHERE etablissement_id = $1 AND id = $2 AND $4::text IS NULL
                  RETURNING id, eleve_id, absence_id, personne_id, nature, statut, prepare_par, prepare_le, valide_par,
                            valide_le, poste_par, poste_le, abandon_code"
            }
            "poste" => {
                "UPDATE ent_asd_courrier SET statut = 'poste', poste_par = $3, poste_le = now()
                  WHERE etablissement_id = $1 AND id = $2 AND $4::text IS NULL
                  RETURNING id, eleve_id, absence_id, personne_id, nature, statut, prepare_par, prepare_le, valide_par,
                            valide_le, poste_par, poste_le, abandon_code"
            }
            _ => {
                "UPDATE ent_asd_courrier SET statut = 'abandonne', abandonne_par = $3, abandonne_le = now(), abandon_code = $4
                  WHERE etablissement_id = $1 AND id = $2
                  RETURNING id, eleve_id, absence_id, personne_id, nature, statut, prepare_par, prepare_le, valide_par,
                            valide_le, poste_par, poste_le, abandon_code"
            }
        };
        let c: CourrierAsd = sqlx::query_as(sql)
            .bind(etab)
            .bind(id)
            .bind(acteur)
            .bind(abandon)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(c)
    }

    /// Courriers d'un élève (`assiduite.suivre`).
    pub async fn courriers_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<CourrierAsd>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSuivre)
            .await?;
        self.exiger_eleve_existe(etab, eleve_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as(&format!(
            "SELECT {COURRIER_COLONNES} FROM ent_asd_courrier WHERE etablissement_id = $1 AND eleve_id = $2
              ORDER BY prepare_le DESC, id LIMIT 100"
        ))
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    // ── Vue de la famille (§14.1, §15.2) ──────────────────────────────────────────────────

    /// Ce que la famille a reçu de la vie scolaire : les contacts qui l'ont **atteinte** et les courriers
    /// **postés**. Jamais la note interne, l'auteur, un contact « ne répond pas » ou « numéro faux » (la
    /// famille n'a pas à lire le suivi interne), ni un courrier non décidé.
    pub async fn communications_famille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<CommunicationFamille>, EntDbError> {
        // Réservé au responsable (lien avec droit de lecture) : l'élève, même majeur, ne lit pas le suivi des
        // contacts entre adultes.
        if self
            .qualite_famille(etab, acteur, role, eleve_id, true)
            .await?
            != Some("famille")
        {
            return Err(introuvable());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Cloisonnement par responsable : seuls les contacts et courriers qui le visent, ou sans destinataire
        // précis, sont visibles (un parent ne lit pas ce qui a été échangé avec l'autre).
        sqlx::query_as(
            "WITH moi AS (SELECT personne_id FROM ent_eleve_lien
                           WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3)
             SELECT 'contact' AS type_communication, canal, cree_le FROM ent_asd_contact
              WHERE etablissement_id = $1 AND eleve_id = $2 AND issue IN ('joint', 'message_laisse', 'envoye', 'rencontre_faite')
                AND (personne_id IS NULL OR personne_id IN (SELECT personne_id FROM moi))
             UNION ALL
             SELECT 'courrier', 'courrier', poste_le FROM ent_asd_courrier
              WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'poste'
                AND (personne_id IS NULL OR personne_id IN (SELECT personne_id FROM moi))
             ORDER BY cree_le DESC LIMIT 100",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(acteur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }
}

const COURRIER_COLONNES: &str =
    "id, eleve_id, absence_id, personne_id, nature, statut, prepare_par, prepare_le, valide_par, \
     valide_le, poste_par, poste_le, abandon_code";

/// Rôle de l'acteur lisible dans les messages d'erreur internes (jamais exposé).
#[allow(dead_code)]
fn _role(r: &str) -> Option<Role> {
    Role::parse(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn la_coupure_partage_matin_et_apres_midi() {
        assert_eq!(periode_de(t(8, 0), t(12, 0)), "matin");
        assert_eq!(periode_de(t(11, 59), t(12, 0)), "matin");
        assert_eq!(periode_de(t(12, 0), t(12, 0)), "apres_midi");
        assert_eq!(periode_de(t(13, 30), t(12, 30)), "apres_midi");
    }

    #[test]
    fn les_rythmes_sont_croissants_et_connus() {
        for r in RYTHMES {
            let d = delais_rythme(r).expect("rythme connu");
            assert!(
                d.windows(2).all(|w| w[0] < w[1]),
                "{r} : délais strictement croissants"
            );
        }
        assert_eq!(delais_rythme("standard"), Some([1, 3, 5, 8]));
        assert_eq!(delais_rythme("inconnu"), None);
    }

    #[test]
    fn issues_et_canaux_couvrent_le_cahier_des_charges() {
        for issue in ["joint", "message_laisse", "ne_repond_pas", "numero_faux"] {
            assert!(ISSUES_CONTACT.contains(&issue));
        }
        assert_eq!(ETAPES.len(), 4);
    }
}
