//! Socle 42, lot V3 « argent relié » (`docs/socles/42_SORTIES_VOYAGES.md` §12.4, §12.5, §6.2 ; VOY-224, VOY-286 ;
//! D-068 : la comptabilité des voyages est épaissie et reliée au reste de l'ENT, pas refondue).
//!
//! Trois liens, tous dans la continuité du mode suivi de V2a (R-SV44, D-034 : aucune facture, aucun avoir comptable) :
//!
//! * **aides** (APEL, fonds social, collectivité, entreprise, don affecté) appliquées à la part d'**un** payeur : elles
//!   réduisent ce qui reste dû, elles ne sont **jamais** de l'argent versé par la famille — un avoir ne les rend donc
//!   pas au payeur (CF-69) ; une aide d'un participant parti est signalée « à restituer ou réaffecter » ;
//! * **reports** de versement d'un participant à un autre **du même payeur** (frère ou sœur, enfant remplacé depuis la
//!   liste d'attente) : l'argent ne change jamais de payeur ; l'avoir du partant se recalcule sur ce qu'il lui reste ;
//! * **internat** (socle 41) : un interne dont le **départ est constaté** (participant « parti ») est retiré de l'attendu
//!   des nuits concernées (motif « séjour scolaire ») ; avant, il reste attendu et appelé, même « prêt » (D-069, règle 1).
//!   L'attendu suit toujours l'état réel du voyage : **une seule fonction**, [`internat_resynchroniser`], appelée dans la
//!   transaction de chaque changement (annulation, refus, réouverture, dates, retrait d'un participant, retour anticipé).
//!
//! Tout mouvement d'argent est inscrit en **ajout seul** (annulation par écriture inverse motivée, vérifiée par la
//! base) ; les montants sont des centimes entiers venus d'un texte (`centimes_de_texte`), jamais d'un nombre du
//! navigateur ; aucun montant ni nom ne part dans l'outbox ; seule la comptabilité (`voyages.argent`) voit le détail.
//!
//! **Pouvoir sur l'argent (D-069, règle 3).** Toute aide et tout report passent par une *demande* motivée. Le décideur
//! n'est jamais lié à l'élève (`ent_eleve_lien`) ni payeur du compte ; au-delà d'un seuil paramétrable par
//! établissement (défaut : toute aide du fonds social, tout montant > 150 €) une **seconde personne** valide. Chaque aide
//! et chaque report est tracé (demandeur, valideur, motif obligatoire, instant) dans des tables en ajout seul, jamais
//! dans le journal de l'organisateur (le motif d'une aide ne doit pas lui parvenir). Clé d'idempotence (`cle`) : un
//! double clic ne produit qu'une écriture. Horloge passée en paramètre (`maintenant`, comme au socle 41).

use chrono::{DateTime, NaiveDate, Utc};
use ent_events::{types, AggregateKind};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::outbox::Causalite;
use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::voyages::est_terminal;
use crate::voyages_argent::{centimes_de_texte, VERSEMENT_MAX};
use crate::voyages_argent_repo::{
    aides_nettes, paye_net, peut_argent, recalculer_depart, synchroniser, verrou_sortie,
    ETATS_PAYANTS,
};
use crate::voyages_repo::{
    charger_avec_acces, interdit, journaliser, refus, texte, texte_opt, tx, Fait, SortieLigne,
};

/// Sources d'une aide (codes ; le libellé est l'affaire de l'écran).
pub const SOURCES_AIDE: [&str; 6] = [
    "apel",
    "fonds_social",
    "collectivite",
    "entreprise",
    "don_affecte",
    "autre",
];

/// États d'un participant dont le **départ est constaté** (D-069, règle 1) : seuls ceux-là produisent un retrait de
/// l'attendu de l'internat. « prêt » n'en fait pas partie : tant que l'élève n'est pas parti, il reste attendu et appelé.
/// « rentré » y figure : ses nuits passées hors de l'établissement restent retirées, jusqu'à la veille de son retour.
pub(crate) const ETATS_DEPART_CONSTATE: [&str; 4] =
    ["parti", "remis", "parti_seul_autorise", "rentre"];

/// États de la sortie qui produisent un retrait (D-069, règle 2). Une sortie rentrée ou close garde l'historique des nuits
/// passées hors de l'établissement : elle ne resynchronise plus rien.
const ETATS_SORTIE_ACTIFS: [&str; 3] = ["ouverte", "prete", "en_cours"];

/// Clés du registre des paramètres d'établissement (socle 5, `structure_etab::registre`).
pub const CLE_SEUIL_SECONDE_VALIDATION: &str = "voyages.aide_seuil_seconde_validation_centimes";
pub const CLE_FONDS_SOCIAL_SECONDE_VALIDATION: &str =
    "voyages.aide_fonds_social_seconde_validation";
/// Défaut prudent : au-delà de 150 € (le montant du seuil lui-même ne déclenche pas la seconde validation).
pub const SEUIL_SECONDE_VALIDATION_DEFAUT: i64 = 15_000;

/// Avertissement affiché à la saisie d'un motif (F2) : le motif d'une aide ou d'un report est lu par la comptabilité
/// et la direction ; il ne décrit jamais la situation sociale, de revenus ou de santé d'une famille.
pub const AVERTISSEMENT_MOTIF: &str = "N'écris aucun détail de situation sociale, de revenus ou de santé : un motif général suffit (« décision de la commission », « report entre frères »). Le détail reste dans le dossier de la commission, pas ici.";

/// Causes d'une fin de retrait posée par la synchronisation (trace `ent_sortie_internat_retrait_fin`).
mod cause {
    pub const DATES: &str = "dates";
    pub const PARTICIPANT: &str = "participant";
    pub const SORTIE: &str = "sortie";
    pub const RETOUR_ANTICIPE: &str = "retour_anticipe";
}

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct AideSaisie {
    pub participant_id: Uuid,
    pub compte_id: Uuid,
    pub source: String,
    /// Texte au format français (`80,00`) : jamais un nombre venu du navigateur.
    pub montant: String,
    #[serde(default)]
    pub reference: Option<String>,
    /// Motif **obligatoire**, jamais un détail de situation sociale (voir [`AVERTISSEMENT_MOTIF`]).
    pub motif: String,
    /// Clé d'idempotence (8 à 64 caractères) choisie par le client : un double clic ne produit qu'une écriture.
    pub cle: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AideVue {
    pub id: Uuid,
    pub participant_id: Uuid,
    pub compte_id: Uuid,
    pub sens: String,
    pub source: String,
    pub montant_centimes: i64,
    pub reference: Option<String>,
    pub annule_id: Option<Uuid>,
    pub motif: String,
    pub decide_le: DateTime<Utc>,
    /// Aide encore active d'un participant qui n'est plus inscrit : à restituer à sa source ou à réaffecter.
    pub a_restituer: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReportSaisie {
    pub de_participant_id: Uuid,
    pub vers_participant_id: Uuid,
    pub compte_id: Uuid,
    pub montant: String,
    pub motif: String,
    /// Clé d'idempotence (8 à 64 caractères).
    pub cle: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ReportVue {
    pub id: Uuid,
    pub de_participant_id: Uuid,
    pub vers_participant_id: Uuid,
    pub compte_id: Uuid,
    pub sens: String,
    pub montant_centimes: i64,
    pub annule_id: Option<Uuid>,
    pub motif: String,
    pub decide_le: DateTime<Utc>,
}

/// Où en est une demande d'argent (aide ou report).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatutDemande {
    /// Appliquée : l'aide ou le report est écrit.
    Appliquee,
    /// Au-delà du seuil : une seconde personne doit valider.
    EnAttente,
    Refusee,
    Retiree,
}

impl StatutDemande {
    fn de_issue(issue: &str) -> Self {
        match issue {
            "appliquee" => Self::Appliquee,
            "refusee" => Self::Refusee,
            _ => Self::Retiree,
        }
    }
}

/// Résultat d'une demande d'aide : appliquée tout de suite, ou en attente de la seconde validation.
#[derive(Debug, Clone, Serialize)]
pub struct DecisionAide {
    pub demande_id: Uuid,
    pub statut: StatutDemande,
    pub aide: Option<AideVue>,
    pub avertissement_motif: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionReport {
    pub demande_id: Uuid,
    pub statut: StatutDemande,
    pub report: Option<ReportVue>,
    pub avertissement_motif: &'static str,
}

/// Issue d'une validation par la seconde personne.
#[derive(Debug, Clone, Serialize)]
pub struct DecisionValidee {
    pub demande_id: Uuid,
    pub statut: StatutDemande,
    pub aide: Option<AideVue>,
    pub report: Option<ReportVue>,
}

/// Demande vue par la comptabilité (nominative, consultation journalisée).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DemandeVue {
    pub id: Uuid,
    pub nature: String,
    pub participant_id: Uuid,
    pub vers_participant_id: Option<Uuid>,
    pub compte_id: Uuid,
    pub source: Option<String>,
    pub montant_centimes: i64,
    pub reference: Option<String>,
    pub motif: String,
    pub seconde_validation: bool,
    pub demande_par_user_id: Uuid,
    pub demande_le: DateTime<Utc>,
    /// `en_attente`, `expiree` (en attente depuis plus de 30 jours : on ne la valide plus), `appliquee`, `refusee` ou `retiree`.
    pub statut: String,
    pub issue_motif: Option<String>,
}

/// Résultat de la synchronisation avec l'internat : des nombres, aucun nom.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InternatSynchro {
    pub poses: usize,
    pub annules: usize,
    pub inchanges: usize,
    /// Nuits passées hors de l'établissement : 0 pour un séjour d'une seule journée (rien à retirer).
    pub nuits: i64,
    /// Participants que le CPE a rétablis dans l'attendu de l'internat : son annulation est respectée (M2).
    pub respectes: usize,
}

/// Demande d'argent lue en base.
#[derive(Debug, Clone, FromRow)]
struct DemandeLigne {
    id: Uuid,
    nature: String,
    participant_id: Uuid,
    vers_participant_id: Option<Uuid>,
    compte_id: Uuid,
    source: Option<String>,
    montant_centimes: i64,
    reference: Option<String>,
    motif: String,
    seconde_validation: bool,
    demande_par_user_id: Uuid,
    demande_le: DateTime<Utc>,
}

// ───────────────────────── Lectures internes ─────────────────────────

/// Part courante (centimes) de ce payeur pour ce participant ; `None` si ce payeur n'en porte aucune.
async fn part_courante(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    participant: Uuid,
    compte: Uuid,
) -> Result<Option<i64>, EntDbError> {
    sqlx::query_scalar(
        "SELECT montant_centimes FROM ent_sortie_part
          WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4
          ORDER BY prix_rang DESC LIMIT 1",
    )
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .bind(compte)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

/// État du participant de cette sortie, ligne verrouillée.
async fn etat_participant(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    participant: Uuid,
) -> Result<String, EntDbError> {
    sqlx::query_scalar(
        "SELECT etat FROM ent_sortie_participant WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 FOR UPDATE",
    )
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

const SELECT_AIDE: &str = "SELECT x.id, x.participant_id, x.compte_id, x.sens, x.source, x.montant_centimes, x.reference,
                x.annule_id, x.motif, x.decide_le,
                (x.sens = 'aide'
                 AND NOT EXISTS (SELECT 1 FROM ent_sortie_aide y WHERE y.etablissement_id = x.etablissement_id AND y.annule_id = x.id)
                 AND NOT EXISTS (SELECT 1 FROM ent_sortie_participant p
                                  WHERE p.etablissement_id = x.etablissement_id AND p.id = x.participant_id
                                    AND p.etat = ANY($3))) AS a_restituer
           FROM ent_sortie_aide x";

async fn lire_aide(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<AideVue, EntDbError> {
    sqlx::query_as(&format!(
        "{SELECT_AIDE} WHERE x.etablissement_id = $1 AND x.id = $2"
    ))
    .bind(etab)
    .bind(id)
    .bind(
        ETATS_PAYANTS
            .iter()
            .map(|e| (*e).to_string())
            .collect::<Vec<_>>(),
    )
    .fetch_one(conn)
    .await
    .map_err(db)
}

async fn lire_report(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<ReportVue, EntDbError> {
    sqlx::query_as(
        "SELECT id, de_participant_id, vers_participant_id, compte_id, sens, montant_centimes, annule_id, motif, decide_le
           FROM ent_sortie_report WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(conn)
    .await
    .map_err(db)
}

/// Le participant est-il parti de la sortie (désisté ou retiré) ?
fn est_parti(etat: &str) -> bool {
    matches!(etat, "desiste" | "retire")
}

/// M4 : aucune aide, aucun report, aucun pointage ne s'inscrit sur une sortie close, annulée ou refusée.
pub(crate) fn exiger_sortie_ouverte_a_l_argent(s: &SortieLigne) -> Result<(), EntDbError> {
    if est_terminal(&s.etat) {
        return Err(refus(
            RefusMetier::Conflit,
            "Cette sortie est close, annulée ou refusée : aucun mouvement d'argent ne s'y inscrit.",
        ));
    }
    Ok(())
}

/// Message du refus de la règle 3 (le même partout : il sert aussi à reconnaître ce refus pour le tracer).
pub(crate) const MSG_REGLE3: &str = "Un pouvoir sur l'argent ne s'exerce pas pour son propre enfant ni pour son propre compte : une autre personne doit décider.";

/// Ce refus est-il celui de la règle 3 ?
pub(crate) fn est_refus_regle3(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(RefusMetier::Interdit, m) if *m == MSG_REGLE3)
}

/// Refus de seconde personne : la validation ou l'annulation vient de la demandeuse ou de la décideuse d'origine.
pub(crate) const MSG_SECONDE_VALIDATION: &str =
    "Une seconde personne valide : le demandeur ne valide pas sa propre demande.";

/// Faible 4 : ce refus laisse-t-il une trace (`ent_sortie_argent_refus`) ? Tous les refus d'un pouvoir sur l'argent exercé
/// seul ou pour soi (règle 3, seconde personne exigée, contournement), pas les refus de forme ou de droit.
pub(crate) fn est_refus_a_tracer(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Refus(RefusMetier::Interdit, m)
        if *m == MSG_REGLE3 || *m == MSG_SECONDE_ANNULATION || *m == MSG_SECONDE_VALIDATION
            || *m == crate::voyages_contournement::MSG_DEMANDEUSE_DECIDE
            || *m == crate::voyages_contournement::MSG_REVOCATION_AUTRICE)
}

/// Faible 4 : un `23514` des déclencheurs de pouvoir sur l'argent (656/655/653), venu **après** le contrôle applicatif
/// (course : un lien créé entre les deux), est la même interdiction que celle de la règle 3 : il ne se transforme ni en
/// erreur 500 ni en refus muet, il se trace comme elle.
pub(crate) fn db_pouvoir(err: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &err {
        // Palier des faibles, lot 2 (F-3 et F-5 de la contre-revue V3b suites 2) : un refus de **cycle de vie** d'un
        // contournement (déjà utilisé, révoqué, expiré, non autorisé), venu de la base après le contrôle applicatif
        // (course), est un conflit d'état (409), pas une interdiction de la règle 3 tracée comme telle ; un second avoir
        // collectif pour un payeur déjà couvert aussi. La révocation par une autre personne que l'autrice reste une
        // interdiction (tracée).
        if d.code().as_deref() == Some("23514") {
            let m = d.message();
            if m.starts_with("seule l'autrice d'une autorisation") {
                return refus(
                    RefusMetier::Interdit,
                    crate::voyages_contournement::MSG_REVOCATION_AUTRICE,
                );
            }
            if [
                "un contournement déjà utilisé",
                "un contournement révoqué",
                "un contournement expiré",
                "un contournement non autorisé",
                "la décision d'un contournement ne se prend plus",
            ]
            .iter()
            .any(|p| m.starts_with(p))
            {
                return refus(RefusMetier::Conflit, MSG_CONTOURNEMENT_ETAT);
            }
            if m.starts_with("un avoir d'annulation collective") {
                return refus(RefusMetier::Conflit, MSG_AVOIR_COLLECTIF_BORNE);
            }
        }
        if d.code().as_deref() == Some("23514")
            && (d.message().starts_with("un pouvoir sur l'argent")
                || d.message().starts_with("la décision d'un contournement")
                || d.message()
                    .starts_with("la personne qui autorise un contournement")
                || d.message().starts_with("seule la demandeuse")
                || d.message().starts_with("un contournement"))
        {
            return refus(RefusMetier::Interdit, MSG_REGLE3);
        }
        if d.code().as_deref() == Some("23514")
            && d.message().starts_with("une seconde personne valide")
        {
            return refus(RefusMetier::Interdit, MSG_SECONDE_VALIDATION);
        }
    }
    db(err)
}

/// Refus de cycle de vie d'un contournement venu de la base (course) : son état a changé entre-temps.
pub(crate) const MSG_CONTOURNEMENT_ETAT: &str =
    "Ce contournement a changé d'état entre-temps (utilisé, révoqué ou expiré) : rechargez la page.";

/// Refus d'un avoir collectif venu de la base (course) : ce payeur est déjà couvert, ou rien n'a été versé.
pub(crate) const MSG_AVOIR_COLLECTIF_BORNE: &str =
    "Un avoir d'annulation collective ne dépasse pas l'argent versé : ce payeur est déjà couvert pour ce participant.";

/// Faible 4 : nombre maximal de traces de refus par personne et par heure.
pub const PLAFOND_TRACES_REFUS_PAR_HEURE: i64 = 30;

/// Délai au-delà duquel une demande en attente expire (F-c) : on ne la valide plus, on la refuse ou on la retire.
pub const DELAI_EXPIRATION_DEMANDE_JOURS: i64 = 30;

/// Règle 3 : le décideur (ou le valideur) n'est lié à aucun des élèves concernés et n'est pas payeur du compte.
/// Étendue (M-3, D-069) à tout pouvoir sur l'argent : plans, versements, frais réels, avoirs, acomptes.
pub(crate) async fn exiger_decideur_libre(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
    compte: Uuid,
    participants: &[Uuid],
) -> Result<(), EntDbError> {
    let eleves: Vec<Uuid> = sqlx::query_scalar(
        "SELECT eleve_id FROM ent_sortie_participant WHERE etablissement_id = $1 AND id = ANY($2)",
    )
    .bind(etab)
    .bind(participants)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let interdit_: bool =
        sqlx::query_scalar("SELECT ent_sortie_argent_decideur_interdit($1, $2, $3, $4)")
            .bind(etab)
            .bind(user)
            .bind(compte)
            .bind(&eleves)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
    if interdit_ {
        return Err(refus(RefusMetier::Interdit, MSG_REGLE3));
    }
    Ok(())
}

/// M-3 : variante pour un pouvoir qui vise des participants sans compte précis (frais réel, annulation collective,
/// acompte fournisseur) : la personne n'est liée à aucun de ces élèves **et** n'est payeuse d'aucun compte qui porte une
/// part de l'un d'eux.
pub(crate) async fn exiger_libre_des_participants(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
    participants: &[Uuid],
) -> Result<(), EntDbError> {
    exiger_decideur_libre(conn, etab, user, Uuid::nil(), participants).await?;
    let payeur: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_sortie_part q
                         WHERE q.etablissement_id = $1 AND q.participant_id = ANY($3)
                           AND ent_sortie_argent_decideur_interdit($1, $2, q.compte_id, ARRAY[]::uuid[]))",
    )
    .bind(etab)
    .bind(user)
    .bind(participants)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if payeur {
        return Err(refus(RefusMetier::Interdit, MSG_REGLE3));
    }
    Ok(())
}

/// M-3 : un pouvoir sur la sortie entière (acompte fournisseur) : la personne n'est liée à AUCUN participant de la sortie.
pub(crate) async fn exiger_libre_de_la_sortie(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
    sortie: Uuid,
) -> Result<(), EntDbError> {
    let participants: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_sortie_participant WHERE etablissement_id = $1 AND sortie_id = $2",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    exiger_libre_des_participants(conn, etab, user, &participants).await
}

/// Cumul (centimes) déjà engagé pour ce participant et ce payeur de cette sortie, en cette nature : écritures appliquées
/// (nettes des annulations) **plus** demandes encore en attente (M-2 : le seuil de seconde validation porte sur le cumul,
/// jamais sur chaque demande isolément). V3b (Faible 2) : une demande **expirée** (plus de
/// [`DELAI_EXPIRATION_DEMANDE_JOURS`] jours, donc plus validable) ne compte plus : elle n'engage rien. `avec_attente` :
/// faux pour une **annulation** (seul compte ce qui est appliqué : une demande en attente n'est pas encore un pouvoir exercé).
#[allow(clippy::too_many_arguments)] // sortie, participant, payeur, nature, horloge : tous distincts
async fn cumul_engage(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    participant: Uuid,
    compte: Uuid,
    nature: &str,
    maintenant: DateTime<Utc>,
    avec_attente: bool,
) -> Result<i64, EntDbError> {
    let ecrit = if nature == "aide" {
        "SELECT COALESCE(SUM(CASE WHEN sens = 'aide' THEN montant_centimes ELSE -montant_centimes END), 0)::bigint
           FROM ent_sortie_aide
          WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4"
    } else {
        "SELECT COALESCE(SUM(CASE WHEN sens = 'report' THEN montant_centimes ELSE -montant_centimes END), 0)::bigint
           FROM ent_sortie_report
          WHERE etablissement_id = $1 AND sortie_id = $2 AND de_participant_id = $3 AND compte_id = $4"
    };
    let applique: i64 = sqlx::query_scalar(ecrit)
        .bind(etab)
        .bind(sortie)
        .bind(participant)
        .bind(compte)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    if !avec_attente {
        return Ok(applique);
    }
    let attente: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(d.montant_centimes), 0)::bigint FROM ent_sortie_argent_demande d
          WHERE d.etablissement_id = $1 AND d.sortie_id = $2 AND d.nature = $5
            AND d.participant_id = $3 AND d.compte_id = $4
            AND d.demande_le + make_interval(days => $6) >= $7
            AND NOT EXISTS (SELECT 1 FROM ent_sortie_argent_issue i
                             WHERE i.etablissement_id = d.etablissement_id AND i.demande_id = d.id)",
    )
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .bind(compte)
    .bind(nature)
    .bind(i32::try_from(DELAI_EXPIRATION_DEMANDE_JOURS).unwrap_or(30))
    .bind(maintenant)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    applique.checked_add(attente).ok_or_else(invalide)
}

/// Message du refus d'une annulation qui dépasse le seuil (tracé comme les refus de la règle 3).
pub(crate) const MSG_SECONDE_ANNULATION: &str = "Cette annulation dépasse le seuil de seconde validation : une autre personne que celle qui a décidé (ou demandé) doit l'annuler.";

/// F-c : l'annulation d'une aide ou d'un report soumis au seuil ne se fait pas seul : la personne qui annule n'est ni celle
/// qui a exercé le pouvoir ni le demandeur d'origine. V3b (Faible 2) : le seuil porte aussi sur le **cumul** engagé de ce
/// participant et de ce payeur (écritures appliquées nettes), pas sur la seule écriture annulée : sans cela, une aide
/// fractionnée sous le seuil s'annulerait seule morceau par morceau.
#[allow(clippy::too_many_arguments)] // table, écriture, auteur, montant, source et horloge : tous distincts, aucun regroupement naturel
async fn exiger_seconde_personne_annulation(
    conn: &mut PgConnection,
    etab: Uuid,
    table: &str,
    ecriture: Uuid,
    user: Uuid,
    montant: i64,
    source: Option<&str>,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let (seuil, fonds) = seuil_seconde_validation(conn, etab).await?;
    let (nature, col_participant) = if table == "ent_sortie_aide" {
        ("aide", "x.participant_id")
    } else {
        ("report", "x.de_participant_id")
    };
    let (decide, demande, sortie, participant, compte): (Uuid, Option<Uuid>, Uuid, Uuid, Uuid) =
        sqlx::query_as(&format!(
            "SELECT x.decide_par_user_id, d.demande_par_user_id, x.sortie_id, {col_participant}, x.compte_id
               FROM {table} x
               LEFT JOIN ent_sortie_argent_demande d ON d.etablissement_id = x.etablissement_id AND d.id = x.demande_id
              WHERE x.etablissement_id = $1 AND x.id = $2"
        ))
        .bind(etab)
        .bind(ecriture)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
    let cumul = cumul_engage(
        conn,
        etab,
        sortie,
        participant,
        compte,
        nature,
        maintenant,
        false,
    )
    .await?;
    if !(montant > seuil || cumul > seuil || (fonds && source == Some("fonds_social"))) {
        return Ok(());
    }
    if user == decide || demande == Some(user) {
        return Err(refus(RefusMetier::Interdit, MSG_SECONDE_ANNULATION));
    }
    Ok(())
}

/// Seuil de seconde validation de l'établissement (registre des paramètres, défaut prudent 150 €).
async fn seuil_seconde_validation(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<(i64, bool), EntDbError> {
    let seuil: Option<i64> = sqlx::query_scalar(
        "SELECT valeur_entier FROM ent_parametre
          WHERE etablissement_id = $1 AND cle = $2 AND type_param = 'entier'",
    )
    .bind(etab)
    .bind(CLE_SEUIL_SECONDE_VALIDATION)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .flatten();
    let fonds: Option<bool> = sqlx::query_scalar(
        "SELECT valeur_booleen FROM ent_parametre
          WHERE etablissement_id = $1 AND cle = $2 AND type_param = 'booleen'",
    )
    .bind(etab)
    .bind(CLE_FONDS_SOCIAL_SECONDE_VALIDATION)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .flatten();
    Ok((
        seuil.unwrap_or(SEUIL_SECONDE_VALIDATION_DEFAUT),
        fonds.unwrap_or(true),
    ))
}

async fn demande_de_la_cle(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    cle: &str,
) -> Result<Option<DemandeLigne>, EntDbError> {
    sqlx::query_as(
        "SELECT id, nature, participant_id, vers_participant_id, compte_id, source, montant_centimes, reference, motif,
                seconde_validation, demande_par_user_id, demande_le
           FROM ent_sortie_argent_demande WHERE etablissement_id = $1 AND sortie_id = $2 AND cle = $3",
    )
    .bind(etab)
    .bind(sortie)
    .bind(cle)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

/// Issue de la demande : `(issue, aide_id, report_id, motif)`.
type IssueLigne = (String, Option<Uuid>, Option<Uuid>, Option<String>);

async fn issue_de(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
) -> Result<Option<IssueLigne>, EntDbError> {
    sqlx::query_as(
        "SELECT issue, aide_id, report_id, motif FROM ent_sortie_argent_issue
          WHERE etablissement_id = $1 AND demande_id = $2",
    )
    .bind(etab)
    .bind(demande)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

fn cle_valide(cle: &str) -> Result<String, EntDbError> {
    let c = cle.trim();
    if (8..=64).contains(&c.chars().count())
        && c.chars()
            .all(|x| x.is_ascii_alphanumeric() || x == '-' || x == '_')
    {
        Ok(c.to_string())
    } else {
        Err(invalide())
    }
}

/// Entrée commune d'une demande d'argent.
struct EntreeDemande<'a> {
    nature: &'static str,
    participant: Uuid,
    vers: Option<Uuid>,
    compte: Uuid,
    source: Option<&'a str>,
    montant: i64,
    reference: Option<&'a str>,
    motif: &'a str,
    cle: &'a str,
}

/// Même contenu qu'une demande déjà enregistrée sous cette clé ? (sinon la clé est réutilisée à tort : conflit.)
fn meme_demande(d: &DemandeLigne, e: &EntreeDemande<'_>, user: Uuid) -> bool {
    d.nature == e.nature
        && d.participant_id == e.participant
        && d.vers_participant_id == e.vers
        && d.compte_id == e.compte
        && d.source.as_deref() == e.source
        && d.montant_centimes == e.montant
        && d.reference.as_deref() == e.reference
        && d.demande_par_user_id == user
}

/// Contrôles métier d'une aide **au moment où elle s'applique** (reste dû, état du participant).
async fn controler_aide(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &SortieLigne,
    d: &DemandeLigne,
) -> Result<(), EntDbError> {
    let etat = etat_participant(conn, etab, s.id, d.participant_id).await?;
    if !ETATS_PAYANTS.contains(&etat.as_str()) {
        return Err(refus(
            RefusMetier::Conflit,
            "Ce participant n'est plus inscrit : aucune aide ne s'inscrit.",
        ));
    }
    let part = part_courante(conn, etab, s.id, d.participant_id, d.compte_id)
        .await?
        .ok_or_else(introuvable)?;
    let verse = paye_net(conn, etab, s.id, d.participant_id, d.compte_id).await?;
    let aides = aides_nettes(conn, etab, s.id)
        .await?
        .get(&(d.participant_id, d.compte_id))
        .copied()
        .unwrap_or(0);
    let couvert = verse
        .checked_add(aides)
        .and_then(|x| x.checked_add(d.montant_centimes))
        .ok_or_else(invalide)?;
    if couvert > part {
        return Err(refus(
            RefusMetier::Conflit,
            "Cette aide dépasse ce qui reste dû sur la part de ce payeur.",
        ));
    }
    Ok(())
}

/// Contrôles métier d'un report au moment où il s'applique. Retourne l'état du participant qui cède.
async fn controler_report(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &SortieLigne,
    d: &DemandeLigne,
) -> Result<String, EntDbError> {
    let vers = d.vers_participant_id.ok_or_else(invalide)?;
    // Ordre déterministe des verrous de lignes.
    let (premier, second) = if d.participant_id < vers {
        (d.participant_id, vers)
    } else {
        (vers, d.participant_id)
    };
    let e1 = etat_participant(conn, etab, s.id, premier).await?;
    let e2 = etat_participant(conn, etab, s.id, second).await?;
    let (etat_de, etat_vers) = if premier == d.participant_id {
        (e1, e2)
    } else {
        (e2, e1)
    };
    if !ETATS_PAYANTS.contains(&etat_vers.as_str()) {
        return Err(refus(
            RefusMetier::Conflit,
            "Le participant qui reçoit le report n'est plus inscrit.",
        ));
    }
    let part_vers = part_courante(conn, etab, s.id, vers, d.compte_id).await?;
    let part_de = part_courante(conn, etab, s.id, d.participant_id, d.compte_id).await?;
    let (Some(part_vers), Some(_)) = (part_vers, part_de) else {
        return Err(refus(
            RefusMetier::Conflit,
            "Un report reste chez le même payeur : ce payeur doit porter une part des deux participants.",
        ));
    };
    let verse_de = paye_net(conn, etab, s.id, d.participant_id, d.compte_id).await?;
    if d.montant_centimes > verse_de {
        return Err(refus(
            RefusMetier::Conflit,
            "Ce report dépasse ce que ce payeur a versé pour ce participant.",
        ));
    }
    let verse_vers = paye_net(conn, etab, s.id, vers, d.compte_id).await?;
    let aides_vers = aides_nettes(conn, etab, s.id)
        .await?
        .get(&(vers, d.compte_id))
        .copied()
        .unwrap_or(0);
    let couvert = verse_vers
        .checked_add(aides_vers)
        .and_then(|x| x.checked_add(d.montant_centimes))
        .ok_or_else(invalide)?;
    if couvert > part_vers {
        return Err(refus(
            RefusMetier::Conflit,
            "Ce report dépasse ce qui reste dû pour le participant qui le reçoit.",
        ));
    }
    Ok(etat_de)
}

/// Applique une demande : écrit l'aide ou le report **et** son issue, dans la transaction de l'appelant. `par` est la
/// personne qui exerce le pouvoir (le demandeur sous le seuil, sinon la seconde personne).
async fn appliquer_demande(
    conn: &mut PgConnection,
    par: &Acteur<'_>,
    s: &SortieLigne,
    d: &DemandeLigne,
    maintenant: DateTime<Utc>,
) -> Result<(Option<Uuid>, Option<Uuid>), EntDbError> {
    let etab = par.etablissement_id;
    exiger_sortie_ouverte_a_l_argent(s)?;
    let mut participants = vec![d.participant_id];
    participants.extend(d.vers_participant_id);
    exiger_decideur_libre(conn, etab, par.user_id, d.compte_id, &participants).await?;
    let (mut aide, mut report) = (None, None);
    if d.nature == "aide" {
        controler_aide(conn, etab, s, d).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_aide
                 (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, reference,
                  motif, demande_id, decide_par_user_id, decide_le)
             VALUES ($1,$2,$3,$4,$5,'aide',$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(id)
        .bind(etab)
        .bind(s.id)
        .bind(d.participant_id)
        .bind(d.compte_id)
        .bind(d.source.as_deref())
        .bind(d.montant_centimes)
        .bind(&d.reference)
        .bind(&d.motif)
        .bind(d.id)
        .bind(par.user_id)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db_pouvoir)?;
        aide = Some(id);
    } else {
        let vers = d.vers_participant_id.ok_or_else(invalide)?;
        let etat_de = controler_report(conn, etab, s, d).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_report
                 (id, etablissement_id, sortie_id, de_participant_id, vers_participant_id, compte_id, sens,
                  montant_centimes, motif, demande_id, decide_par_user_id, decide_le)
             VALUES ($1,$2,$3,$4,$5,$6,'report',$7,$8,$9,$10,$11)",
        )
        .bind(id)
        .bind(etab)
        .bind(s.id)
        .bind(d.participant_id)
        .bind(vers)
        .bind(d.compte_id)
        .bind(d.montant_centimes)
        .bind(&d.motif)
        .bind(d.id)
        .bind(par.user_id)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db_pouvoir)?;
        if est_parti(&etat_de) {
            recalculer_depart(conn, par, s, d.participant_id).await?;
        }
        report = Some(id);
    }
    ecrire_issue(
        conn,
        etab,
        d.id,
        "appliquee",
        None,
        aide,
        report,
        par.user_id,
        maintenant,
    )
    .await?;
    Ok((aide, report))
}

#[allow(clippy::too_many_arguments)]
async fn ecrire_issue(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    issue: &str,
    motif: Option<&str>,
    aide: Option<Uuid>,
    report: Option<Uuid>,
    par: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_sortie_argent_issue
             (id, etablissement_id, demande_id, issue, motif, aide_id, report_id, par_user_id, le)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(demande)
    .bind(issue)
    .bind(motif)
    .bind(aide)
    .bind(report)
    .bind(par)
    .bind(maintenant)
    .execute(conn)
    .await
    .map_err(db_pouvoir)?;
    Ok(())
}

/// Cœur d'une demande (aide ou report) : idempotence, contrôles, seuil, enregistrement, application si le seuil n'est
/// pas atteint. Retourne `(demande, statut, aide_id, report_id)`.
async fn traiter_demande(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    e: &EntreeDemande<'_>,
    maintenant: DateTime<Utc>,
) -> Result<(Uuid, StatutDemande, Option<Uuid>, Option<Uuid>), EntDbError> {
    let etab = a.etablissement_id;
    // F1 : rejeu d'une même demande (double clic, réessai réseau) = le même résultat, aucune seconde écriture.
    if let Some(d) = demande_de_la_cle(conn, etab, s.id, e.cle).await? {
        if !meme_demande(&d, e, a.user_id) {
            return Err(conflit());
        }
        return Ok(match issue_de(conn, etab, d.id).await? {
            None => (d.id, StatutDemande::EnAttente, None, None),
            Some((i, aide, report, _)) => (d.id, StatutDemande::de_issue(&i), aide, report),
        });
    }
    exiger_sortie_ouverte_a_l_argent(s)?;
    let mut participants = vec![e.participant];
    participants.extend(e.vers);
    exiger_decideur_libre(conn, etab, a.user_id, e.compte, &participants).await?;
    let (seuil, fonds) = seuil_seconde_validation(conn, etab).await?;
    // M-2 : le seuil porte sur le CUMUL (aides ou reports appliqués + demandes en attente) de ce participant et de ce payeur.
    // Limite documentée : la règle « fonds social » reste déclarative (`source` saisie) ; le cumul, lui, vaut pour toutes
    // les sources, ce qui empêche de fractionner ou d'étiqueter « autre » pour passer sous le seuil.
    let cumul = cumul_engage(
        conn,
        etab,
        s.id,
        e.participant,
        e.compte,
        e.nature,
        maintenant,
        true,
    )
    .await?;
    let seconde = cumul.checked_add(e.montant).ok_or_else(invalide)? > seuil
        || (fonds && e.source == Some("fonds_social"));
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_sortie_argent_demande
             (id, etablissement_id, sortie_id, nature, participant_id, vers_participant_id, compte_id, source,
              montant_centimes, reference, motif, seconde_validation, cle, demande_par_user_id, demande_le)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
    )
    .bind(id)
    .bind(etab)
    .bind(s.id)
    .bind(e.nature)
    .bind(e.participant)
    .bind(e.vers)
    .bind(e.compte)
    .bind(e.source)
    .bind(e.montant)
    .bind(e.reference)
    .bind(e.motif)
    .bind(seconde)
    .bind(e.cle)
    .bind(a.user_id)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if seconde {
        // On vérifie dès la demande que les montants tiennent (l'application les revérifie à la validation).
        let d = DemandeLigne {
            id,
            nature: e.nature.into(),
            participant_id: e.participant,
            vers_participant_id: e.vers,
            compte_id: e.compte,
            source: e.source.map(str::to_string),
            montant_centimes: e.montant,
            reference: e.reference.map(str::to_string),
            motif: e.motif.into(),
            seconde_validation: true,
            demande_par_user_id: a.user_id,
            demande_le: maintenant,
        };
        if d.nature == "aide" {
            controler_aide(conn, etab, s, &d).await?;
        } else {
            controler_report(conn, etab, s, &d).await?;
        }
        return Ok((id, StatutDemande::EnAttente, None, None));
    }
    let d = demande_de_la_cle(conn, etab, s.id, e.cle)
        .await?
        .ok_or_else(introuvable)?;
    let (aide, report) = appliquer_demande(conn, a, s, &d, maintenant).await?;
    Ok((id, StatutDemande::Appliquee, aide, report))
}

// ───────────────────────── Internat : une seule fonction de synchronisation ─────────────────────────

/// Fin d'un retrait de voyage par la synchronisation : annulé (daté, signé, jamais supprimé) **et** tracé, ce qui le
/// distingue d'une annulation faite par le CPE (M2).
async fn annuler_retrait(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    sortie: Uuid,
    retrait: Uuid,
    eleve: Uuid,
    du: NaiveDate,
    au: NaiveDate,
    cause: &str,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_internat_retrait SET annule_le = $4, annule_par_user_id = $3
          WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NULL",
    )
    .bind(a.etablissement_id)
    .bind(retrait)
    .bind(a.user_id)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if r.rows_affected() == 1 {
        sqlx::query(
            "INSERT INTO ent_sortie_internat_retrait_fin (id, etablissement_id, sortie_id, retrait_id, cause, par_user_id, le)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(sortie)
        .bind(retrait)
        .bind(cause)
        .bind(a.user_id)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        crate::finances::publier(
            conn,
            a.etablissement_id,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            retrait,
            Some(eleve),
            json!({ "du": du.to_string(), "au": au.to_string(), "cause": "retrait_annule", "motif_code": "sejour_scolaire" }),
            Some(a.user_id),
            Causalite::default(),
        )
        .await?;
    }
    Ok(())
}

/// **La** fonction qui aligne l'attendu de l'internat sur l'état réel du voyage (D-069, règle 2), dans la transaction de
/// l'appelant (verrou de la sortie déjà pris). Appelée à chaque changement : état de la sortie (annulation, refus,
/// réouverture, confirmation), dates, désistement ou retrait d'un participant, retour anticipé, et par la route de
/// synchronisation.
///
/// * Seules les sorties **ouvertes, prêtes ou en cours** produisent un retrait ; une sortie rentrée ou close garde
///   l'historique des nuits passées hors de l'établissement et ne change plus rien.
/// * Un retrait n'est posé que pour un interne dont le **départ est constaté** (règle 1) ; « prêt » n'en pose aucun.
/// * Un participant « rentré » (retour anticipé) n'est retiré que jusqu'à la veille de son retour.
/// * Une sortie **annulée** après le départ ne retire que les nuits déjà passées hors de l'établissement.
/// * M2 : un retrait de voyage annulé par le CPE (sans trace de fin de la synchronisation) n'est **pas reposé** : le
///   participant, que le CPE a vu à l'internat, reste attendu ; le décompte est rendu (`respectes`).
pub(crate) async fn internat_resynchroniser(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    sortie: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<InternatSynchro, EntDbError> {
    let etab = a.etablissement_id;
    let s: SortieLigne =
        sqlx::query_as("SELECT * FROM ent_sortie WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(sortie)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
    // Une sortie close ne change plus rien. Une sortie RENTRÉE (F-f) n'est plus ignorée : seuls les participants constatés
    // « rentrés » gardent leurs nuits passées dehors (jusqu'à la veille de leur retour) ; un participant resté « parti »
    // sur une sortie rentrée n'est plus retiré (sens sûr : attendu, appelé, cherché).
    if s.etat == "close" {
        return Ok(InternatSynchro {
            poses: 0,
            annules: 0,
            inchanges: 0,
            nuits: 0,
            respectes: 0,
        });
    }
    let active = ETATS_SORTIE_ACTIFS.contains(&s.etat.as_str());
    let annulee = s.etat == "annulee";
    let rentree = s.etat == "rentree";
    // Jours de l'établissement (jamais UTC) : nuit du départ à la veille du retour.
    // M-4 : le jour de l'annulation vient de `annulee_le` (posé à la transition, immuable), jamais de `modifie_le`.
    let (du, retour, jour_annulation): (NaiveDate, NaiveDate, NaiveDate) = sqlx::query_as(
        "SELECT ent_aujourdhui_etab($1, $2), ent_aujourdhui_etab($1, $3), ent_aujourdhui_etab($1, $4)",
    )
    .bind(etab)
    .bind(s.debut)
    .bind(s.fin)
    .bind(s.annulee_le.unwrap_or(s.fin))
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let au = retour - chrono::Duration::days(1);
    // V3b (Moyen 2) : sortie rentrée : un participant resté « parti » n'est retiré que pour les nuits qui précèdent le jour
    // de ce passage (`rentree_le`, posé à la transition, immuable) ; les nuits à venir qu'il ne passera pas dehors le
    // laissent attendu. Jamais lu dans `modifie_le` sauf à défaut d'instant posé (sortie rentrée avant 656, rattrapée).
    let jour_rentree: Option<NaiveDate> = if rentree {
        Some(
            sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
                .bind(etab)
                .bind(s.rentree_le.unwrap_or(s.modifie_le))
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?,
        )
    } else {
        None
    };
    let nuits = if au >= du {
        (au - du).num_days() + 1
    } else {
        0
    };
    if nuits > 400 {
        return Err(refus(
            RefusMetier::Invalide,
            "Un séjour de plus de 400 nuits ne se retire pas de l'internat en une fois.",
        ));
    }
    // Sortie annulée en cours de route : seules les nuits déjà passées hors de l'établissement restent retirées.
    let au_sortie = if annulee {
        au.min(jour_annulation - chrono::Duration::days(1))
    } else {
        au
    };
    // Internes dont le départ est constaté, avec la dernière nuit de leur absence (retour anticipé : veille du retour).
    let voulus: Vec<(Uuid, Uuid, NaiveDate)> = if (!active && !annulee && !rentree)
        || au_sortie < du
    {
        Vec::new()
    } else {
        let lignes: Vec<(Uuid, Uuid, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT sp.id, sp.eleve_id,
                    CASE WHEN sp.etat = 'rentre' THEN
                        ent_aujourdhui_etab($1, COALESCE((SELECT max(f.a) FROM ent_sortie_participant_fait f
                                                           WHERE f.etablissement_id = sp.etablissement_id
                                                             AND f.participant_id = sp.id AND f.etat_apres = 'rentre'),
                                                         sp.modifie_le)) - 1
                         WHEN $6::date IS NOT NULL THEN $6::date - 1
                    END
               FROM ent_sortie_participant sp
              WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2 AND sp.etat = ANY($3)
                AND EXISTS (SELECT 1 FROM ent_internat_sejour j
                             WHERE j.etablissement_id = sp.etablissement_id AND j.eleve_id = sp.eleve_id
                               AND daterange(j.du, j.au, '[]') && daterange($4, $5, '[]'))
              ORDER BY sp.id",
        )
        .bind(etab)
        .bind(s.id)
        .bind(
            ETATS_DEPART_CONSTATE
                .iter()
                .map(|e| (*e).to_string())
                .collect::<Vec<_>>(),
        )
        .bind(du)
        .bind(au)
        .bind(jour_rentree)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        lignes
            .into_iter()
            .filter_map(|(p, e, cap)| {
                let fin = cap.map_or(au_sortie, |c| c.min(au_sortie));
                (fin >= du).then_some((p, e, fin))
            })
            .collect()
    };
    // Retraits de séjour encore actifs posés pour cette sortie.
    let existants: Vec<(Uuid, Uuid, Uuid, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT r.id, l.participant_id, r.eleve_id, r.du_nuit, r.au_nuit
           FROM ent_sortie_internat_retrait l
           JOIN ent_internat_retrait r ON r.etablissement_id = l.etablissement_id AND r.id = l.retrait_id
          WHERE l.etablissement_id = $1 AND l.sortie_id = $2 AND r.annule_le IS NULL",
    )
    .bind(etab)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    // M2 : participants dont un retrait a été annulé par le CPE (annulé sans trace de la synchronisation).
    let ecartes: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT l.participant_id
           FROM ent_sortie_internat_retrait l
           JOIN ent_internat_retrait r ON r.etablissement_id = l.etablissement_id AND r.id = l.retrait_id
          WHERE l.etablissement_id = $1 AND l.sortie_id = $2 AND r.annule_le IS NOT NULL
            AND NOT EXISTS (SELECT 1 FROM ent_sortie_internat_retrait_fin f
                             WHERE f.etablissement_id = l.etablissement_id AND f.retrait_id = l.retrait_id)",
    )
    .bind(etab)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let (mut poses, mut annules, mut inchanges, mut respectes) = (0usize, 0usize, 0usize, 0usize);
    for (retrait, participant, eleve, rdu, rau) in &existants {
        let voulu = voulus.iter().find(|(p, _, _)| p == participant);
        let garde = matches!(voulu, Some((_, _, fin)) if *rdu == du && rau == fin);
        if garde {
            inchanges += 1;
            continue;
        }
        let cause = match voulu {
            None if active => cause::PARTICIPANT,
            None => cause::SORTIE,
            Some((_, _, fin)) if *fin < au_sortie => cause::RETOUR_ANTICIPE,
            Some(_) => cause::DATES,
        };
        annuler_retrait(
            conn, a, s.id, *retrait, *eleve, *rdu, *rau, cause, maintenant,
        )
        .await?;
        annules += 1;
    }
    for (participant, eleve, fin) in &voulus {
        let deja = existants
            .iter()
            .any(|(_, p, _, rdu, rau)| p == participant && *rdu == du && rau == fin);
        if deja {
            continue;
        }
        if ecartes.contains(participant) {
            respectes += 1;
            continue;
        }
        let retrait = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_internat_retrait
                 (id, etablissement_id, eleve_id, du_nuit, au_nuit, motif_code, source, par_user_id, cree_le)
             VALUES ($1,$2,$3,$4,$5,'sejour_scolaire','voyage',$6,$7)",
        )
        .bind(retrait)
        .bind(etab)
        .bind(eleve)
        .bind(du)
        .bind(fin)
        .bind(a.user_id)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_sortie_internat_retrait
                 (id, etablissement_id, sortie_id, participant_id, retrait_id, du_nuit, au_nuit, cree_par_user_id, cree_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(s.id)
        .bind(participant)
        .bind(retrait)
        .bind(du)
        .bind(fin)
        .bind(a.user_id)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        crate::finances::publier(
            conn,
            etab,
            types::INTERNAT_ATTENDU_MODIFIE,
            AggregateKind::Internat,
            retrait,
            Some(*eleve),
            json!({ "du": du.to_string(), "au": fin.to_string(), "cause": "retrait", "motif_code": "sejour_scolaire" }),
            Some(a.user_id),
            Causalite::default(),
        )
        .await?;
        poses += 1;
    }
    Ok(InternatSynchro {
        poses,
        annules,
        inchanges,
        nuits,
        respectes,
    })
}

impl PgEnt {
    /// F-a (contre-revue V3) : l'ordre des contrôles est **personnel → droit → verrou → existence**. Rien ne prend le
    /// verrou consultatif d'une sortie, ni ne révèle son existence, avant que l'acteur soit du personnel (un parent ou un
    /// élève n'a rien à faire ici : « introuvable », comme avant) puis, quand la route l'exige, qu'il ait le droit
    /// d'établissement (sinon « interdit »). `droit` : `None` quand le porteur de la sortie suffit aussi (le droit précis est
    /// alors vérifié après chargement de la sortie).
    pub(crate) async fn avant_verrou(
        &self,
        a: &Acteur<'_>,
        droit: Option<ent_auth::Action>,
    ) -> Result<(), EntDbError> {
        if matches!(a.role, "parent" | "eleve") {
            return Err(introuvable());
        }
        if let Some(x) = droit {
            self.voyages_exiger(a, x).await?;
        }
        Ok(())
    }

    /// F-c : la trace d'un refus de la règle 3 s'écrit dans **sa propre transaction** (celle de l'écriture refusée est
    /// annulée) : qui, quelle action, quelle sortie, quand. Ni nom, ni montant, ni motif. Une trace qui ne s'écrit pas ne
    /// masque jamais le refus lui-même.
    pub(crate) async fn tracer_refus_regle3<T>(
        &self,
        a: &Acteur<'_>,
        sortie: Option<Uuid>,
        action: &str,
        r: Result<T, EntDbError>,
    ) -> Result<T, EntDbError> {
        if let Err(e) = &r {
            if est_refus_a_tracer(e) {
                if let Ok(mut t) = tx(self, a).await {
                    // Plafond de débit, par (personne, action) et par heure : au plus `PLAFOND_TRACES_REFUS_PAR_HEURE`
                    // traces (un refus répété à la main ou par script ne gonfle pas une table en ajout seul, jamais
                    // purgée, et le flot d'une action ne masque pas une autre). Au-delà : UNE ligne « plafond atteint »
                    // par (personne, action) et par heure signale le dépassement. Le refus lui-même n'est jamais masqué.
                    let ecrit = sqlx::query(
                        "INSERT INTO ent_sortie_argent_refus (id, etablissement_id, sortie_id, user_id, action, plafond_atteint)
                         SELECT $1, $2, $3, $4, $5, n.n >= $6
                           FROM (SELECT count(*) AS n FROM ent_sortie_argent_refus r
                                  WHERE r.etablissement_id = $2 AND r.user_id = $4 AND r.action = $5
                                    AND r.le > now() - interval '1 hour') n
                          WHERE n.n < $6
                             OR (n.n = $6 AND NOT EXISTS (SELECT 1 FROM ent_sortie_argent_refus m
                                                           WHERE m.etablissement_id = $2 AND m.user_id = $4 AND m.action = $5
                                                             AND m.plafond_atteint AND m.le > now() - interval '1 hour'))",
                    )
                    .bind(Uuid::new_v4())
                    .bind(a.etablissement_id)
                    .bind(sortie)
                    .bind(a.user_id)
                    .bind(action)
                    .bind(PLAFOND_TRACES_REFUS_PAR_HEURE)
                    .execute(&mut *t)
                    .await;
                    if ecrit.is_ok() {
                        let _ = t.commit().await;
                    }
                }
            }
        }
        r
    }

    // ───────────────────────── Aides (VOY-224) ─────────────────────────

    /// Demande une aide sur la part d'**un** payeur d'un participant. Comptabilité et direction (`voyages.argent`).
    /// L'aide ne dépasse jamais ce qui reste dû sur cette part (versé et aides déjà inscrites déduits). Sous le seuil de
    /// l'établissement elle s'applique tout de suite ; au-delà (et pour toute aide du fonds social par défaut), elle
    /// attend la validation d'une **seconde personne** (règle 3).
    pub async fn voyage_aide_accorder(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: AideSaisie,
    ) -> Result<DecisionAide, EntDbError> {
        self.voyage_aide_accorder_a(a, id, v, Utc::now()).await
    }

    /// Variante à horloge passée en paramètre (F4).
    pub async fn voyage_aide_accorder_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: AideSaisie,
        maintenant: DateTime<Utc>,
    ) -> Result<DecisionAide, EntDbError> {
        let r = self
            .voyage_aide_accorder_a_corps(a, id, v, maintenant)
            .await;
        self.tracer_refus_regle3(a, Some(id), "voyage_aide_accorder_a", r)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_aide_accorder_a_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: AideSaisie,
        maintenant: DateTime<Utc>,
    ) -> Result<DecisionAide, EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        if !SOURCES_AIDE.contains(&v.source.as_str()) {
            return Err(invalide());
        }
        let montant = centimes_de_texte(&v.montant)
            .filter(|m| (1..=VERSEMENT_MAX).contains(m))
            .ok_or_else(invalide)?;
        let reference = texte_opt(v.reference.as_deref(), 40)?;
        let motif = texte(&v.motif, 500)?;
        let cle = cle_valide(&v.cle)?;
        let mut t = tx(self, a).await?;
        // M1 : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        if s.nature == "obligatoire" {
            return Err(refus(
                RefusMetier::Conflit,
                "Sortie obligatoire : rien n'est dû par les familles, aucune aide à inscrire.",
            ));
        }
        synchroniser(&mut t, a, &s).await?;
        let (demande, statut, aide, _) = traiter_demande(
            &mut t,
            a,
            &s,
            &EntreeDemande {
                nature: "aide",
                participant: v.participant_id,
                vers: None,
                compte: v.compte_id,
                source: Some(&v.source),
                montant,
                reference: reference.as_deref(),
                motif: &motif,
                cle: &cle,
            },
            maintenant,
        )
        .await?;
        let vue = match aide {
            Some(aid) => Some(lire_aide(&mut t, a.etablissement_id, aid).await?),
            None => None,
        };
        t.commit().await.map_err(db)?;
        Ok(DecisionAide {
            demande_id: demande,
            statut,
            aide: vue,
            avertissement_motif: AVERTISSEMENT_MOTIF,
        })
    }

    /// Annule une aide par une écriture **inverse motivée** (l'aide d'origine n'est jamais modifiée). Le décideur de
    /// l'annulation obéit à la même règle : jamais pour son propre enfant ni son propre compte.
    pub async fn voyage_aide_annuler(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        aide_id: Uuid,
        motif: &str,
    ) -> Result<AideVue, EntDbError> {
        self.voyage_aide_annuler_a(a, id, aide_id, motif, Utc::now())
            .await
    }

    pub async fn voyage_aide_annuler_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        aide_id: Uuid,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<AideVue, EntDbError> {
        let r = self
            .voyage_aide_annuler_a_corps(a, id, aide_id, motif, maintenant)
            .await;
        self.tracer_refus_regle3(a, Some(id), "voyage_aide_annuler_a", r)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_aide_annuler_a_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        aide_id: Uuid,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<AideVue, EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let o = lire_aide(&mut t, a.etablissement_id, aide_id)
            .await
            .map_err(|_| introuvable())?;
        let de_cette_sortie: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_aide WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3)",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(aide_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if !de_cette_sortie {
            return Err(introuvable());
        }
        if o.sens != "aide" {
            return Err(invalide());
        }
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_aide WHERE etablissement_id = $1 AND annule_id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(aide_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit());
        }
        exiger_decideur_libre(
            &mut t,
            a.etablissement_id,
            a.user_id,
            o.compte_id,
            &[o.participant_id],
        )
        .await?;
        // F-c : une annulation au-dessus du seuil ne se fait pas seul.
        exiger_seconde_personne_annulation(
            &mut t,
            a.etablissement_id,
            "ent_sortie_aide",
            aide_id,
            a.user_id,
            o.montant_centimes,
            Some(&o.source),
            maintenant,
        )
        .await?;
        let nid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_aide
                 (id, etablissement_id, sortie_id, participant_id, compte_id, sens, source, montant_centimes, reference,
                  annule_id, motif, decide_par_user_id, decide_le)
             VALUES ($1,$2,$3,$4,$5,'annulation',$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(nid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(o.participant_id)
        .bind(o.compte_id)
        .bind(&o.source)
        .bind(o.montant_centimes)
        .bind(&o.reference)
        .bind(aide_id)
        .bind(&motif)
        .bind(a.user_id)
        .bind(maintenant)
        .execute(&mut *t)
        .await
        .map_err(db_pouvoir)?;
        let vue = lire_aide(&mut t, a.etablissement_id, nid).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Liste **nominative** des aides d'une sortie : comptabilité et direction seulement, **consultation journalisée**.
    pub async fn voyage_aides(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<AideVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let v: Vec<AideVue> = sqlx::query_as(&format!(
            "{SELECT_AIDE} WHERE x.etablissement_id = $1 AND x.sortie_id = $2 ORDER BY x.decide_le, x.id"
        ))
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(
            ETATS_PAYANTS
                .iter()
                .map(|e| (*e).to_string())
                .collect::<Vec<_>>(),
        )
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        self.acces_nominatif(&mut t, a, Some(s.id), "liste_paiements", v.len())
            .await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    // ───────────────────────── Reports de versement (VOY-286, écart 9 de V2a) ─────────────────────────

    /// Demande le report d'un montant déjà versé d'un participant à un autre **du même payeur**. Jamais plus que ce que
    /// ce payeur a versé pour le premier, jamais plus que ce qui reste dû pour le second ; si le premier est parti, son
    /// avoir proposé se recalcule sur ce qu'il lui reste. Même seuil et même seconde validation que pour une aide.
    pub async fn voyage_report_effectuer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: ReportSaisie,
    ) -> Result<DecisionReport, EntDbError> {
        self.voyage_report_effectuer_a(a, id, v, Utc::now()).await
    }

    pub async fn voyage_report_effectuer_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: ReportSaisie,
        maintenant: DateTime<Utc>,
    ) -> Result<DecisionReport, EntDbError> {
        let r = self
            .voyage_report_effectuer_a_corps(a, id, v, maintenant)
            .await;
        self.tracer_refus_regle3(a, Some(id), "voyage_report_effectuer_a", r)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_report_effectuer_a_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        v: ReportSaisie,
        maintenant: DateTime<Utc>,
    ) -> Result<DecisionReport, EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        let montant = centimes_de_texte(&v.montant)
            .filter(|m| (1..=VERSEMENT_MAX).contains(m))
            .ok_or_else(invalide)?;
        let motif = texte(&v.motif, 500)?;
        let cle = cle_valide(&v.cle)?;
        if v.de_participant_id == v.vers_participant_id {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        if s.nature == "obligatoire" {
            return Err(refus(
                RefusMetier::Conflit,
                "Sortie obligatoire : rien n'est dû par les familles.",
            ));
        }
        synchroniser(&mut t, a, &s).await?;
        let (demande, statut, _, report) = traiter_demande(
            &mut t,
            a,
            &s,
            &EntreeDemande {
                nature: "report",
                participant: v.de_participant_id,
                vers: Some(v.vers_participant_id),
                compte: v.compte_id,
                source: None,
                montant,
                reference: None,
                motif: &motif,
                cle: &cle,
            },
            maintenant,
        )
        .await?;
        let vue = match report {
            Some(rid) => Some(lire_report(&mut t, a.etablissement_id, rid).await?),
            None => None,
        };
        t.commit().await.map_err(db)?;
        Ok(DecisionReport {
            demande_id: demande,
            statut,
            report: vue,
            avertissement_motif: AVERTISSEMENT_MOTIF,
        })
    }

    /// Défait un report par une écriture inverse motivée. Refusé si l'argent reporté n'est plus disponible chez le
    /// receveur (déjà remboursé ou reporté ailleurs) : la comptabilité règle d'abord ce mouvement.
    pub async fn voyage_report_annuler(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        report_id: Uuid,
        motif: &str,
    ) -> Result<ReportVue, EntDbError> {
        self.voyage_report_annuler_a(a, id, report_id, motif, Utc::now())
            .await
    }

    pub async fn voyage_report_annuler_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        report_id: Uuid,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<ReportVue, EntDbError> {
        let r = self
            .voyage_report_annuler_a_corps(a, id, report_id, motif, maintenant)
            .await;
        self.tracer_refus_regle3(a, Some(id), "voyage_report_annuler_a", r)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_report_annuler_a_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        report_id: Uuid,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<ReportVue, EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let o: ReportVue = sqlx::query_as(
            "SELECT id, de_participant_id, vers_participant_id, compte_id, sens, montant_centimes, annule_id, motif, decide_le
               FROM ent_sortie_report WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(report_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if o.sens != "report" {
            return Err(invalide());
        }
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_report WHERE etablissement_id = $1 AND annule_id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(report_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit());
        }
        exiger_decideur_libre(
            &mut t,
            a.etablissement_id,
            a.user_id,
            o.compte_id,
            &[o.de_participant_id, o.vers_participant_id],
        )
        .await?;
        // F-c : une annulation au-dessus du seuil ne se fait pas seul.
        exiger_seconde_personne_annulation(
            &mut t,
            a.etablissement_id,
            "ent_sortie_report",
            report_id,
            a.user_id,
            o.montant_centimes,
            None,
            maintenant,
        )
        .await?;
        let verse_vers = paye_net(
            &mut t,
            a.etablissement_id,
            s.id,
            o.vers_participant_id,
            o.compte_id,
        )
        .await?;
        if verse_vers < o.montant_centimes {
            return Err(refus(
                RefusMetier::Conflit,
                "L'argent reporté n'est plus disponible chez le participant qui l'a reçu : régler d'abord ce mouvement.",
            ));
        }
        let nid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_report
                 (id, etablissement_id, sortie_id, de_participant_id, vers_participant_id, compte_id, sens,
                  montant_centimes, annule_id, motif, decide_par_user_id, decide_le)
             VALUES ($1,$2,$3,$4,$5,$6,'annulation',$7,$8,$9,$10,$11)",
        )
        .bind(nid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(o.de_participant_id)
        .bind(o.vers_participant_id)
        .bind(o.compte_id)
        .bind(o.montant_centimes)
        .bind(report_id)
        .bind(&motif)
        .bind(a.user_id)
        .bind(maintenant)
        .execute(&mut *t)
        .await
        .map_err(db_pouvoir)?;
        // Les avoirs déjà proposés ou retenus du receveur reposent sur un versé qui baisse : « à revoir » (M5).
        let revus = sqlx::query(
            "UPDATE ent_sortie_avoir_propose SET statut = 'a_revoir'
              WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4
                AND statut IN ('propose', 'retenu')",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(o.vers_participant_id)
        .bind(o.compte_id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if revus > 0 {
            // F5 : la remise « à revoir » d'avoirs a un acteur et une cause (aucun nom, aucun montant).
            journaliser(
                &mut t,
                a,
                s.id,
                Fait {
                    champ: Some("avoirs_a_revoir"),
                    apres: Some(format!("{revus} avoir(s) à revoir : report annulé")),
                    ..Fait::simple("avoirs_a_revoir")
                },
            )
            .await?;
        }
        let etat_de: String =
            etat_participant(&mut t, a.etablissement_id, s.id, o.de_participant_id).await?;
        if est_parti(&etat_de) {
            recalculer_depart(&mut t, a, &s, o.de_participant_id).await?;
        }
        let vue = lire_report(&mut t, a.etablissement_id, nid).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Liste des reports d'une sortie : comptabilité et direction, consultation journalisée.
    pub async fn voyage_reports(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<ReportVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let v: Vec<ReportVue> = sqlx::query_as(
            "SELECT id, de_participant_id, vers_participant_id, compte_id, sens, montant_centimes, annule_id, motif, decide_le
               FROM ent_sortie_report WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY decide_le, id",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        self.acces_nominatif(&mut t, a, Some(s.id), "liste_paiements", v.len())
            .await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    // ───────────────────────── Demandes : seconde validation (règle 3) ─────────────────────────

    /// Demandes d'argent d'une sortie, avec leur statut : comptabilité et direction, consultation journalisée.
    pub async fn voyage_demandes(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<DemandeVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let v: Vec<DemandeVue> = sqlx::query_as(
            "SELECT d.id, d.nature, d.participant_id, d.vers_participant_id, d.compte_id, d.source, d.montant_centimes,
                    d.reference, d.motif, d.seconde_validation, d.demande_par_user_id, d.demande_le,
                    CASE WHEN i.id IS NULL AND d.demande_le < $3 - make_interval(days => $4::int) THEN 'expiree'
                         WHEN i.id IS NULL THEN 'en_attente'
                         WHEN i.issue = 'appliquee' THEN 'appliquee' ELSE i.issue END AS statut,
                    i.motif AS issue_motif
               FROM ent_sortie_argent_demande d
               LEFT JOIN ent_sortie_argent_issue i ON i.etablissement_id = d.etablissement_id AND i.demande_id = d.id
              WHERE d.etablissement_id = $1 AND d.sortie_id = $2 ORDER BY d.demande_le, d.id",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(Utc::now())
        .bind(DELAI_EXPIRATION_DEMANDE_JOURS as i32)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        self.acces_nominatif(&mut t, a, Some(s.id), "liste_paiements", v.len())
            .await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Valide une demande en attente : **une autre personne** que le demandeur, libre de tout lien avec l'élève et le
    /// compte. Les montants sont revérifiés à cet instant. Compare-et-écrit : une demande déjà tranchée → 409.
    pub async fn voyage_demande_valider(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        demande_id: Uuid,
    ) -> Result<DecisionValidee, EntDbError> {
        self.voyage_demande_valider_a(a, id, demande_id, Utc::now())
            .await
    }

    pub async fn voyage_demande_valider_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        demande_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<DecisionValidee, EntDbError> {
        let r = self
            .voyage_demande_valider_a_corps(a, id, demande_id, maintenant)
            .await;
        self.tracer_refus_regle3(a, Some(id), "voyage_demande_valider_a", r)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_demande_valider_a_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        demande_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<DecisionValidee, EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let d = self
            .demande_de_la_sortie(&mut t, a, s.id, demande_id)
            .await?;
        if issue_de(&mut t, a.etablissement_id, d.id).await?.is_some() {
            return Err(conflit());
        }
        if d.seconde_validation && d.demande_par_user_id == a.user_id {
            return Err(refus(RefusMetier::Interdit, MSG_SECONDE_VALIDATION));
        }
        // F-c : une demande en attente expire ; on ne la valide plus (on la refuse ou on la retire, puis on la redépose).
        if maintenant - d.demande_le > chrono::Duration::days(DELAI_EXPIRATION_DEMANDE_JOURS) {
            return Err(refus(
                RefusMetier::Conflit,
                "Cette demande est en attente depuis plus de 30 jours : elle a expiré. Refusez-la ou retirez-la, puis redéposez-la.",
            ));
        }
        let (aide, report) = appliquer_demande(&mut t, a, &s, &d, maintenant).await?;
        let aide_vue = match aide {
            Some(x) => Some(lire_aide(&mut t, a.etablissement_id, x).await?),
            None => None,
        };
        let report_vue = match report {
            Some(x) => Some(lire_report(&mut t, a.etablissement_id, x).await?),
            None => None,
        };
        t.commit().await.map_err(db)?;
        Ok(DecisionValidee {
            demande_id: d.id,
            statut: StatutDemande::Appliquee,
            aide: aide_vue,
            report: report_vue,
        })
    }

    /// Refuse (seconde personne) ou retire (demandeur) une demande en attente, motif obligatoire.
    pub async fn voyage_demande_refuser(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        demande_id: Uuid,
        motif: &str,
    ) -> Result<StatutDemande, EntDbError> {
        self.voyage_demande_refuser_a(a, id, demande_id, motif, Utc::now())
            .await
    }

    pub async fn voyage_demande_refuser_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        demande_id: Uuid,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<StatutDemande, EntDbError> {
        let r = self
            .voyage_demande_refuser_a_corps(a, id, demande_id, motif, maintenant)
            .await;
        self.tracer_refus_regle3(a, Some(id), "voyage_demande_refuser_a", r)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn voyage_demande_refuser_a_corps(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        demande_id: Uuid,
        motif: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<StatutDemande, EntDbError> {
        // F-a : personnel, puis droit, AVANT le verrou de la sortie et l'existence.
        self.avant_verrou(a, Some(ent_auth::Action::VoyagesArgent))
            .await?;
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let d = self
            .demande_de_la_sortie(&mut t, a, s.id, demande_id)
            .await?;
        if issue_de(&mut t, a.etablissement_id, d.id).await?.is_some() {
            return Err(conflit());
        }
        let (issue, statut) = if d.demande_par_user_id == a.user_id {
            ("retiree", StatutDemande::Retiree)
        } else {
            let mut participants = vec![d.participant_id];
            participants.extend(d.vers_participant_id);
            exiger_decideur_libre(
                &mut t,
                a.etablissement_id,
                a.user_id,
                d.compte_id,
                &participants,
            )
            .await?;
            ("refusee", StatutDemande::Refusee)
        };
        ecrire_issue(
            &mut t,
            a.etablissement_id,
            d.id,
            issue,
            Some(&motif),
            None,
            None,
            a.user_id,
            maintenant,
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(statut)
    }

    async fn demande_de_la_sortie(
        &self,
        conn: &mut PgConnection,
        a: &Acteur<'_>,
        sortie: Uuid,
        demande: Uuid,
    ) -> Result<DemandeLigne, EntDbError> {
        sqlx::query_as(
            "SELECT id, nature, participant_id, vers_participant_id, compte_id, source, montant_centimes, reference,
                    motif, seconde_validation, demande_par_user_id, demande_le
               FROM ent_sortie_argent_demande WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(sortie)
        .bind(demande)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    // ───────────────────────── Internat (socle 41) ─────────────────────────

    /// Aligne l'attendu de l'internat sur l'état réel du voyage (voir [`internat_resynchroniser`]). Idempotent : rejouer
    /// ne change rien. Le porteur, la direction ou un droit de modification de sortie.
    pub async fn voyage_internat_synchroniser(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<InternatSynchro, EntDbError> {
        self.voyage_internat_synchroniser_a(a, id, Utc::now()).await
    }

    pub async fn voyage_internat_synchroniser_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<InternatSynchro, EntDbError> {
        // F-a : personnel avant le verrou et l'existence ; le droit précis (porteur ou droit d'établissement) suit le chargement.
        self.avant_verrou(a, None).await?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(ent_auth::Action::VoyagesModifier) {
            return Err(interdit());
        }
        let r = internat_resynchroniser(&mut t, a, s.id, maintenant).await?;
        if r.poses + r.annules > 0 {
            journaliser(
                &mut t,
                a,
                s.id,
                Fait {
                    champ: Some("internat"),
                    ..Fait::simple("internat_synchronise")
                },
            )
            .await?;
        }
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    /// **Retour anticipé** d'un participant parti (maladie, rapatriement, décision de la famille) : il repasse à
    /// « rentré », ses nuits d'après son retour ne sont plus retirées et il est de nouveau attendu à l'internat, dans la
    /// même transaction. C'est aussi la voie du CPE quand il voit à l'internat un élève que le voyage disait parti
    /// (M2) : le porteur ou la direction constate le retour ; le CPE peut en outre annuler le retrait côté internat,
    /// annulation mémorisée et respectée.
    pub async fn voyage_participant_retour_anticipe(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        participant_id: Uuid,
        motif_code: &str,
    ) -> Result<InternatSynchro, EntDbError> {
        self.voyage_participant_retour_anticipe_a(
            a,
            id,
            participant_id,
            motif_code,
            None,
            Utc::now(),
        )
        .await
    }

    pub async fn voyage_participant_retour_anticipe_a(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        participant_id: Uuid,
        motif_code: &str,
        retour_le: Option<DateTime<Utc>>,
        maintenant: DateTime<Utc>,
    ) -> Result<InternatSynchro, EntDbError> {
        if !crate::voyages::MOTIFS_PARTICIPANT.contains(&motif_code) {
            return Err(invalide());
        }
        // F-a : personnel avant le verrou et l'existence ; le droit précis (porteur ou droit d'établissement) suit le chargement.
        self.avant_verrou(a, None).await?;
        let mut t = tx(self, a).await?;
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.gere(ent_auth::Action::VoyagesParticipants) {
            return Err(interdit());
        }
        // F-b : un retour anticipé ne s'enregistre que sur une sortie en cours de route (jamais rentrée, close, annulée).
        if !ETATS_SORTIE_ACTIFS.contains(&s.etat.as_str()) {
            return Err(refus(
                RefusMetier::Conflit,
                "Cette sortie n'est plus en cours : un retour anticipé ne s'y enregistre plus.",
            ));
        }
        let actuel = etat_participant(&mut t, a.etablissement_id, s.id, participant_id).await?;
        if actuel != "parti" {
            return Err(refus(
                RefusMetier::Conflit,
                "Seul un participant parti peut rentrer avant la fin du séjour.",
            ));
        }
        // F-b : le retour est l'instant où l'élève est réellement rentré (saisie tardive possible), borné : jamais dans le
        // futur, jamais avant son départ constaté ; l'instant de la saisie reste tracé dans le fait.
        let depart: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT max(a) FROM ent_sortie_participant_fait
              WHERE etablissement_id = $1 AND participant_id = $2 AND etat_apres = 'parti'",
        )
        .bind(a.etablissement_id)
        .bind(participant_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let retour = retour_le.unwrap_or(maintenant);
        if retour > maintenant || retour < depart.unwrap_or(s.debut) {
            return Err(refus(
                RefusMetier::Invalide,
                "La date de retour ne peut être ni dans le futur ni avant le départ du participant.",
            ));
        }
        let trace = (retour != maintenant).then(|| {
            format!(
                "retour constaté le {}, saisi le {}",
                retour.to_rfc3339(),
                maintenant.to_rfc3339()
            )
        });
        sqlx::query(
            "UPDATE ent_sortie_participant SET etat = 'rentre', modifie_le = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(participant_id)
        .bind(maintenant)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        crate::voyages_repo::fait_participant_a(
            &mut t,
            a,
            participant_id,
            "etat",
            Some("parti"),
            "rentre",
            Some(motif_code),
            trace.as_deref(),
            retour,
        )
        .await?;
        let r = internat_resynchroniser(&mut t, a, s.id, maintenant).await?;
        journaliser(
            &mut t,
            a,
            s.id,
            Fait {
                champ: Some("internat"),
                motif: Some(motif_code),
                ..Fait::simple("retour_anticipe")
            },
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }
}
