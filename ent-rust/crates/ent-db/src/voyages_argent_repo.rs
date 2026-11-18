//! Socle 42, lot V2a : l'argent des sorties en **mode suivi** (R-SV44, D-034 ; `42_SORTIES_VOYAGES.md` §12, §32.5).
//!
//! ScolIA **décide** la part des familles (budget, prix annoncé figé), la **répartit** entre les payeurs par le port
//! `payeurs_a` du socle 24/4 (aucune logique de répartition dupliquée), **pointe** les versements reçus et **propose**
//! des avoirs ; il n'émet ni facture, ni avis de prélèvement, ni écriture comptable. Charlemagne garde la comptabilité.
//!
//! * **Montants** en centimes entiers (`BIGINT`, `CHECK` de bornes, arithmétique contrôlée) ;
//! * **ajout seul** : les versements, parts, plans, frais réels sont inscrits, jamais modifiés ; une erreur de
//!   pointage s'annule par une écriture inverse **motivée** (déclencheurs de la base) ;
//! * **droits** : l'état agrégé est visible de l'organisateur (porteur) ; le nominatif est réservé à `voyages.argent`
//!   (comptabilité, direction) et au responsable désigné par la direction, **chaque accès journalisé** ; un parent ne
//!   voit que la part de son propre compte et ses versements (jamais ceux de l'autre payeur du même élève) ;
//! * **isolation** : toutes les requêtes portent `etablissement_id` ;
//! * **outbox** : aucun montant, aucun nom ; seulement des identifiants et des codes.
//!
//! Le calcul pur (échéancier, situation, avoir proposé, statut d'acompte) est dans `voyages_argent.rs`.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::{Action, Role};
use ent_object::presign::{Disposition, LinkSpec, PresignError};
use ent_object::ObjectStore;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, LienFichier, PgEnt, RefusMetier};
use crate::voyages::champ_csv;
use crate::voyages_argent::{
    calculer_avoir, centimes_de_texte, derniere_echeance, echeancier, euros, repartir_entre_payeurs, situation,
    CasAvoir, Echeance, EtatPaiement, ECHEANCES_MAX, ECHEANCES_STANDARD_MAX, MOYENS, VERSEMENT_MAX,
};
use crate::voyages_repo::{charger_avec_acces, interdit, refus, texte, texte_opt, tx, Acces, SortieLigne};

/// États d'un participant qui doit sa part (liste de participants confirmée ou en cours de séjour).
pub(crate) const ETATS_PAYANTS: [&str; 8] =
    ["inscrit", "pret", "bloque", "parti", "absent_depart", "rentre", "remis", "parti_seul_autorise"];

// ───────────────────────── Types ─────────────────────────

/// État agrégé pour l'organisateur (R-SV18) : **des nombres, jamais un nom**.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EtatAgrege {
    pub participants_payants: i64,
    pub soldes: i64,
    pub a_jour: i64,
    pub plans_en_cours: i64,
    pub en_retard: i64,
    /// Participants dont la part n'est pas encore calculée (prix non figé, payeur à désigner).
    pub a_definir: i64,
    pub du_total_centimes: i64,
    pub verse_total_centimes: i64,
    pub reste_total_centimes: i64,
    /// Sortie obligatoire : part 0 imposée, rien n'est dû (CV-81).
    pub gratuite: bool,
    /// Effectif trop petit (moins de 3 payants) : les compteurs sont masqués pour qui n'a pas `voyages.argent` (F8).
    pub masque: bool,
}

/// Une part (un payeur d'un participant) avec sa situation : vue **nominative**.
#[derive(Debug, Clone, Serialize)]
pub struct LignePaiement {
    pub participant_id: Uuid,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub compte_id: Uuid,
    pub payeur: String,
    pub part_centimes: i64,
    pub verse_centimes: i64,
    pub reste_centimes: i64,
    pub retard_centimes: i64,
    pub etat: String,
    pub nb_echeances: i32,
    pub plan_long: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub echeances: Vec<EcheanceVue>,
}

/// Ce que voit le **responsable désigné** (E1) : jamais l'échéancier, le nombre d'échéances ni le plan long ; un plan en
/// cours est replié sur « à jour ».
#[derive(Debug, Clone, Serialize)]
pub struct LignePaiementResponsable {
    pub participant_id: Uuid,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub compte_id: Uuid,
    pub payeur: String,
    pub part_centimes: i64,
    pub verse_centimes: i64,
    pub reste_centimes: i64,
    pub retard_centimes: i64,
    pub etat: String,
}

/// Réponse de la liste nominative : le DTO est choisi selon `voyages.argent`, jamais selon la demande.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PaiementsVue {
    Complet(Vec<LignePaiement>),
    Responsable(Vec<LignePaiementResponsable>),
}

#[derive(Debug, Clone, Serialize)]
pub struct EcheanceVue {
    pub date: NaiveDate,
    pub centimes: i64,
}

/// Ce qu'un parent voit : **sa** part et **ses** versements.
#[derive(Debug, Clone, Serialize)]
pub struct MaPart {
    pub participant_id: Uuid,
    pub eleve_prenom: String,
    pub compte_id: Uuid,
    pub payeur: String,
    pub part_centimes: i64,
    pub verse_centimes: i64,
    pub reste_centimes: i64,
    pub etat: String,
    pub echeances: Vec<EcheanceVue>,
    pub versements: Vec<VersementFamille>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct VersementFamille {
    pub id: Uuid,
    pub sens: String,
    pub moyen: String,
    pub montant_centimes: i64,
    pub date_valeur: NaiveDate,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersementSaisie {
    pub participant_id: Uuid,
    pub compte_id: Uuid,
    pub moyen: String,
    /// Texte au format français (`125,50`) : jamais un nombre venu du navigateur.
    pub montant: String,
    /// Jour de la valeur ; par défaut le jour de l'établissement. Jamais dans le futur.
    #[serde(default)]
    pub date_valeur: Option<NaiveDate>,
    #[serde(default)]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct VersementVue {
    pub id: Uuid,
    pub participant_id: Uuid,
    pub compte_id: Uuid,
    pub sens: String,
    pub moyen: String,
    pub montant_centimes: i64,
    pub date_valeur: NaiveDate,
    pub reference: Option<String>,
    pub annule_id: Option<Uuid>,
    pub motif: Option<String>,
    pub saisi_le: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FraisVue {
    pub id: Uuid,
    pub participant_id: Uuid,
    pub nature: String,
    pub libelle: String,
    pub montant_centimes: i64,
    pub annule_id: Option<Uuid>,
    pub saisi_le: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FraisSaisie {
    pub participant_id: Uuid,
    pub nature: String,
    pub libelle: String,
    pub montant: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AvoirVue {
    pub id: Uuid,
    pub participant_id: Uuid,
    pub compte_id: Uuid,
    pub motif_code: String,
    pub paye_net_centimes: i64,
    pub frais_retenus_centimes: i64,
    pub montant_propose_centimes: i64,
    pub statut: String,
    pub montant_retenu_centimes: Option<i64>,
    pub decision_motif: Option<String>,
    pub cree_le: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct NominatifVue {
    pub id: Uuid,
    pub personne_id: Uuid,
    pub motif: String,
    pub ouvert_le: DateTime<Utc>,
    pub ferme_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportFait {
    pub lien: String,
    pub expire_a: i64,
    pub nb_lignes: usize,
    pub sha256: String,
}

// ───────────────────────── Aides internes ─────────────────────────

/// Jour de l'établissement (fuseau de l'établissement, jamais UTC).
pub(crate) async fn jour_etab(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)")
        .bind(etab)
        .fetch_one(conn)
        .await
        .map_err(db)
}

/// Jour de l'établissement à un instant donné (jour du départ d'une sortie).
async fn jour_etab_a(conn: &mut PgConnection, etab: Uuid, instant: DateTime<Utc>) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(conn)
        .await
        .map_err(db)
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Reglages {
    pub collecte_organisateur: bool,
    pub cheques_vacances: bool,
    pub alerte_acompte_jours: i32,
}

impl Default for Reglages {
    fn default() -> Self {
        Self { collecte_organisateur: true, cheques_vacances: false, alerte_acompte_jours: 14 }
    }
}

pub(crate) async fn reglages(conn: &mut PgConnection, etab: Uuid) -> Result<Reglages, EntDbError> {
    let r: Option<Reglages> = sqlx::query_as(
        "SELECT collecte_organisateur, cheques_vacances, alerte_acompte_jours
           FROM ent_sortie_argent_reglage WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.unwrap_or_default())
}

/// Régime de l'établissement (socle 24) : `Some(true)` public, `Some(false)` privé sous contrat, `None` si aucune entité
/// de facturation n'est configurée (M6 : on ne devine pas).
pub(crate) async fn regime_public(conn: &mut PgConnection, etab: Uuid) -> Result<Option<bool>, EntDbError> {
    let r: Option<String> =
        sqlx::query_scalar("SELECT regime FROM ent_fin_entite_facturation WHERE etablissement_id = $1 LIMIT 1")
            .bind(etab)
            .fetch_optional(conn)
            .await
            .map_err(db)?;
    Ok(r.map(|x| x == "public_eple"))
}

pub(crate) fn peut_argent(acces: &Acces) -> bool {
    acces.a(Action::VoyagesArgent)
}

pub(crate) fn peut_remise(acces: &Acces) -> bool {
    acces.a(Action::VoyagesArgent) || acces.a(Action::VoyagesRemise)
}

fn somme(a: i64, b: i64) -> Result<i64, EntDbError> {
    a.checked_add(b).ok_or_else(invalide)
}

#[derive(Debug, Clone, FromRow)]
struct PartRow {
    participant_id: Uuid,
    compte_id: Uuid,
    libelle: String,
    montant_centimes: i64,
    premier_jour: NaiveDate,
}

/// Dernière part (prix annoncé le plus récent) de chaque couple participant/payeur d'une sortie.
async fn parts_courantes(conn: &mut PgConnection, etab: Uuid, sortie: Uuid) -> Result<Vec<PartRow>, EntDbError> {
    sqlx::query_as(
        "SELECT DISTINCT ON (p.participant_id, p.compte_id)
                p.participant_id, p.compte_id, c.libelle_facturation AS libelle,
                p.montant_centimes,
                (SELECT min(q.jour_calcul) FROM ent_sortie_part q
                  WHERE q.etablissement_id = p.etablissement_id AND q.participant_id = p.participant_id
                    AND q.compte_id = p.compte_id) AS premier_jour
           FROM ent_sortie_part p
           JOIN ent_fin_compte c ON c.etablissement_id = p.etablissement_id AND c.id = p.compte_id
          WHERE p.etablissement_id = $1 AND p.sortie_id = $2
          ORDER BY p.participant_id, p.compte_id, p.prix_rang DESC",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_all(conn)
    .await
    .map_err(db)
}

/// Versé net (versements moins annulations) par couple participant/payeur.
async fn verses(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
) -> Result<HashMap<(Uuid, Uuid), i64>, EntDbError> {
    let rows: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
        "SELECT participant_id, compte_id,
                COALESCE(SUM(CASE WHEN sens = 'versement' THEN montant_centimes ELSE -montant_centimes END), 0)::bigint
           FROM ent_sortie_versement WHERE etablissement_id = $1 AND sortie_id = $2
          GROUP BY participant_id, compte_id",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|(p, c, m)| ((p, c), m)).collect())
}

/// Dernier plan accordé par couple : nombre d'échéances.
async fn plans(conn: &mut PgConnection, etab: Uuid, sortie: Uuid) -> Result<HashMap<(Uuid, Uuid), i32>, EntDbError> {
    let rows: Vec<(Uuid, Uuid, i32)> = sqlx::query_as(
        "SELECT DISTINCT ON (participant_id, compte_id) participant_id, compte_id, nb_echeances
           FROM ent_sortie_plan WHERE etablissement_id = $1 AND sortie_id = $2
          ORDER BY participant_id, compte_id, rang DESC",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|(p, c, n)| ((p, c), n)).collect())
}

async fn etats_participants(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
) -> Result<HashMap<Uuid, String>, EntDbError> {
    let rows: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, etat FROM ent_sortie_participant WHERE etablissement_id = $1 AND sortie_id = $2")
            .bind(etab)
            .bind(sortie)
            .fetch_all(conn)
            .await
            .map_err(db)?;
    Ok(rows.into_iter().collect())
}

/// Une part avec sa situation calculée (interne, sans identité).
struct Calculee {
    part: PartRow,
    nb: i32,
    plan_long: bool,
    echeances: Vec<Echeance>,
    verse: i64,
    sit: crate::voyages_argent::Situation,
}

/// Calcule la situation de chaque part **d'un participant actif**.
async fn calculer(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &SortieLigne,
    jour: NaiveDate,
) -> Result<(Vec<Calculee>, HashMap<Uuid, String>), EntDbError> {
    let etats = etats_participants(conn, etab, s.id).await?;
    let parts = parts_courantes(conn, etab, s.id).await?;
    let verse = verses(conn, etab, s.id).await?;
    let pl = plans(conn, etab, s.id).await?;
    let depart = jour_etab_a(conn, etab, s.debut).await?;
    let mut out = Vec::new();
    for part in parts {
        let actif = etats.get(&part.participant_id).is_some_and(|e| ETATS_PAYANTS.contains(&e.as_str()));
        if !actif {
            continue;
        }
        let nb = pl.get(&(part.participant_id, part.compte_id)).copied().unwrap_or(ECHEANCES_STANDARD_MAX);
        let plan_long = nb > ECHEANCES_STANDARD_MAX;
        let derniere = derniere_echeance(part.premier_jour, depart, plan_long);
        let echeances = echeancier(part.montant_centimes, nb, part.premier_jour, derniere).ok_or_else(invalide)?;
        let v = verse.get(&(part.participant_id, part.compte_id)).copied().unwrap_or(0);
        let sit = situation(&echeances, v, jour, plan_long).ok_or_else(invalide)?;
        out.push(Calculee { part, nb, plan_long, echeances, verse: v, sit });
    }
    Ok((out, etats))
}

// ───────────────────────── Parts : synchronisation avec le prix annoncé ─────────────────────────

/// Dernier prix annoncé figé : `(rang, centimes)`.
async fn prix_annonce(conn: &mut PgConnection, etab: Uuid, sortie: Uuid) -> Result<Option<(i32, i64)>, EntDbError> {
    sqlx::query_as(
        "SELECT rang, montant_centimes FROM ent_sortie_prix_fige
          WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY rang DESC LIMIT 1",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

/// Les dépenses sont-elles engagées ? (une commande envoyée au prestataire) : distingue le désistement « avant »
/// et « après » engagement (§12.5).
async fn depenses_engagees(conn: &mut PgConnection, etab: Uuid, sortie: Uuid) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_sortie_commande
                         WHERE etablissement_id = $1 AND sortie_id = $2
                           AND statut IN ('envoyee', 'a_confirmer', 'confirmee'))",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_one(conn)
    .await
    .map_err(db)
}

async fn paye_net(conn: &mut PgConnection, etab: Uuid, sortie: Uuid, participant: Uuid, compte: Uuid) -> Result<i64, EntDbError> {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(CASE WHEN sens = 'versement' THEN montant_centimes ELSE -montant_centimes END), 0)::bigint
           FROM ent_sortie_versement
          WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4",
    )
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .bind(compte)
    .fetch_one(conn)
    .await
    .map_err(db)
}

async fn frais_total(conn: &mut PgConnection, etab: Uuid, sortie: Uuid, participant: Uuid) -> Result<i64, EntDbError> {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(f.montant_centimes), 0)::bigint FROM ent_sortie_frais_reel f
          WHERE f.etablissement_id = $1 AND f.sortie_id = $2 AND f.participant_id = $3
            AND f.annule_id IS NULL
            AND NOT EXISTS (SELECT 1 FROM ent_sortie_frais_reel x
                             WHERE x.etablissement_id = f.etablissement_id AND x.annule_id = f.id)",
    )
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .fetch_one(conn)
    .await
    .map_err(db)
}

async fn inserer_avoir(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: Uuid,
    participant: Uuid,
    compte: Uuid,
    cas: CasAvoir,
    cle: &str,
    paye: i64,
    frais: i64,
) -> Result<bool, EntDbError> {
    let c = calculer_avoir(cas, paye, frais);
    let r = sqlx::query(
        "INSERT INTO ent_sortie_avoir_propose
             (id, etablissement_id, sortie_id, participant_id, compte_id, motif_code, cle,
              paye_net_centimes, frais_retenus_centimes, montant_propose_centimes)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
         ON CONFLICT (etablissement_id, cle) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(sortie)
    .bind(participant)
    .bind(compte)
    .bind(cas.code())
    .bind(cle)
    .bind(c.paye_net_centimes)
    .bind(c.frais_retenus_centimes)
    .bind(c.montant_propose_centimes)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(r.rows_affected() == 1)
}

/// Verrou de transaction par sortie : sérialise les écritures d'argent d'une même sortie.
pub(crate) async fn verrou_sortie(conn: &mut PgConnection, sortie: Uuid) -> Result<(), EntDbError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('voyages_argent:' || $1::text, 0))")
        .bind(sortie)
        .execute(conn)
        .await
        .map_err(db)?;
    Ok(())
}

/// Calcule (une seule fois par prix annoncé) la part de chaque payeur du participant. Idempotent.
///
/// * sortie **obligatoire** : rien n'est dû (part 0 imposée, CV-81) ;
/// * pas de prix annoncé figé : rien ;
/// * premier calcul : les payeurs viennent du **port du 24** (catégorie « voyages », jour de l'établissement) ;
///   s'ils ne totalisent pas 100 %, le participant reste « à définir » (jamais « au responsable 1 », R-F4) ;
/// * recalcul après un nouveau prix : **les mêmes payeurs** (un prix qui change ne change pas qui paie) ; une baisse
///   propose un avoir pour le trop-versé ; un changement de payeur est une décision de la comptabilité (CF-03).
async fn synchroniser_participant(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    participant: Uuid,
    eleve: Uuid,
    jour: NaiveDate,
) -> Result<(), EntDbError> {
    if s.nature == "obligatoire" {
        return Ok(());
    }
    let Some((rang, prix)) = prix_annonce(conn, a.etablissement_id, s.id).await? else {
        return Ok(());
    };
    let deja: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_sortie_part
                         WHERE etablissement_id = $1 AND participant_id = $2 AND prix_rang = $3)",
    )
    .bind(a.etablissement_id)
    .bind(participant)
    .bind(rang)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if deja {
        return Ok(());
    }
    // Payeurs du dernier calcul, sinon le port du socle 24/4.
    let precedents: Vec<(Uuid, i32, i64)> = sqlx::query_as(
        "SELECT p.compte_id, p.part_bps, p.montant_centimes FROM ent_sortie_part p
          WHERE p.etablissement_id = $1 AND p.participant_id = $2
            AND p.prix_rang = (SELECT max(prix_rang) FROM ent_sortie_part
                                WHERE etablissement_id = $1 AND participant_id = $2)
          ORDER BY p.compte_id",
    )
    .bind(a.etablissement_id)
    .bind(participant)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let poids: Vec<(Uuid, i64)> = if precedents.is_empty() {
        let lus = crate::facturation_payeurs::payeurs_a(conn, a.etablissement_id, eleve, "voyages", jour, false).await?;
        if !lus.complete() {
            return Ok(());
        }
        lus.rang1().map(|p| (p.compte_id, p.part_bps)).collect()
    } else {
        precedents.iter().map(|(c, bps, _)| (*c, i64::from(*bps))).collect()
    };
    let parts = repartir_entre_payeurs(prix, &poids);
    // M7 : le premier jour de l'échéancier est celui de l'inscription du participant ou du prix figé (le plus tardif),
    // jamais celui de la première consultation.
    let jour: NaiveDate = sqlx::query_scalar(
        "SELECT GREATEST(ent_aujourdhui_etab($1, sp.cree_le), ent_aujourdhui_etab($1, pf.fige_le))
           FROM ent_sortie_participant sp, ent_sortie_prix_fige pf
          WHERE sp.etablissement_id = $1 AND sp.id = $2 AND pf.etablissement_id = $1 AND pf.sortie_id = $3 AND pf.rang = $4",
    )
    .bind(a.etablissement_id)
    .bind(participant)
    .bind(s.id)
    .bind(rang)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    for ((compte, centimes), (_, bps)) in parts.iter().zip(poids.iter()) {
        sqlx::query(
            "INSERT INTO ent_sortie_part
                 (id, etablissement_id, sortie_id, participant_id, compte_id, prix_rang, montant_centimes, part_bps, jour_calcul)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             ON CONFLICT (participant_id, compte_id, prix_rang) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(participant)
        .bind(compte)
        .bind(rang)
        .bind(centimes)
        .bind((*bps).clamp(1, 10_000) as i32)
        .bind(jour)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    // Baisse du prix annoncé : avoir proposé du trop-versé, par payeur (CF-69 : au payeur qui a payé).
    for (compte, nouvelle) in &parts {
        let Some((_, _, ancienne)) = precedents.iter().find(|(c, _, _)| c == compte) else { continue };
        if nouvelle < ancienne {
            let paye = paye_net(conn, a.etablissement_id, s.id, participant, *compte).await?;
            let cle = format!("prix:{rang}:{participant}:{compte}");
            let c = calculer_avoir(CasAvoir::BaissePrix { ancienne: *ancienne, nouvelle: *nouvelle }, paye, 0);
            if c.montant_propose_centimes > 0 {
                inserer_avoir(
                    conn,
                    a.etablissement_id,
                    s.id,
                    participant,
                    *compte,
                    CasAvoir::BaissePrix { ancienne: *ancienne, nouvelle: *nouvelle },
                    &cle,
                    paye,
                    0,
                )
                .await?;
            }
        }
    }
    Ok(())
}

/// Calcule les parts de tous les participants payants de la sortie (idempotent).
pub(crate) async fn synchroniser(conn: &mut PgConnection, a: &Acteur<'_>, s: &SortieLigne) -> Result<(), EntDbError> {
    if s.nature == "obligatoire" {
        return Ok(());
    }
    verrou_sortie(conn, s.id).await?;
    let jour = jour_etab(conn, a.etablissement_id).await?;
    let ps: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, eleve_id FROM ent_sortie_participant
          WHERE etablissement_id = $1 AND sortie_id = $2 AND etat = ANY($3) ORDER BY id",
    )
    .bind(a.etablissement_id)
    .bind(s.id)
    .bind(ETATS_PAYANTS.iter().map(|x| x.to_string()).collect::<Vec<_>>())
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (participant, eleve) in ps {
        synchroniser_participant(conn, a, s, participant, eleve, jour).await?;
    }
    Ok(())
}

/// Branchement de la désinscription (VOY-145, VOY-146, VOY-286) : calcule les parts si besoin puis **propose** un
/// avoir à chaque payeur ayant versé. `vers` : `desiste` ou `retire` ; `motif_code` : motif codé du participant.
/// Appelé dans la transaction du changement d'état, AVANT la mise à jour (le participant est encore payant).
pub(crate) async fn avant_depart_participant(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    participant: Uuid,
    vers: &str,
    motif_code: &str,
) -> Result<(), EntDbError> {
    if s.nature == "obligatoire" {
        return Ok(());
    }
    verrou_sortie(conn, s.id).await?;
    let eleve: Uuid = sqlx::query_scalar(
        "SELECT eleve_id FROM ent_sortie_participant WHERE etablissement_id = $1 AND id = $2 AND sortie_id = $3",
    )
    .bind(a.etablissement_id)
    .bind(participant)
    .bind(s.id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let jour = jour_etab(conn, a.etablissement_id).await?;
    synchroniser_participant(conn, a, s, participant, eleve, jour).await?;
    let engage = depenses_engagees(conn, a.etablissement_id, s.id).await?;
    let cas = cas_depart(vers, motif_code, engage);
    proposer_avoirs_participant(conn, a, s, participant, cas, "depart", false).await
}

/// Choix de la règle de l'avoir (§12.5, VOY-286) : santé et famille (maladie, deuil, déménagement) sont des motifs
/// légitimes proposés ; une décision de l'établissement rend l'avoir total ; le reste suit l'engagement des dépenses.
pub(crate) fn cas_depart(vers: &str, motif_code: &str, engage: bool) -> CasAvoir {
    if vers == "retire" && matches!(motif_code, "decision_etablissement" | "financier") {
        CasAvoir::DepartEnPreparation
    } else if matches!(motif_code, "sante" | "famille") {
        CasAvoir::MotifLegitime
    } else if engage {
        CasAvoir::DesistementApresEngagement
    } else {
        CasAvoir::DesistementAvantEngagement
    }
}

/// Propose (ou recalcule, tant que rien n'est retenu) l'avoir de chaque payeur d'un participant parti.
async fn proposer_avoirs_participant(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    participant: Uuid,
    cas: CasAvoir,
    prefixe: &str,
    strict: bool,
) -> Result<(), EntDbError> {
    let etab = a.etablissement_id;
    let parts: Vec<(Uuid, i32)> = sqlx::query_as(
        "SELECT DISTINCT ON (compte_id) compte_id, part_bps FROM ent_sortie_part
          WHERE etablissement_id = $1 AND participant_id = $2 ORDER BY compte_id, prix_rang DESC",
    )
    .bind(etab)
    .bind(participant)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let frais = frais_total(conn, etab, s.id, participant).await?;
    let poids: Vec<(Uuid, i64)> = parts.iter().map(|(c, b)| (*c, i64::from(*b))).collect();
    let frais_par: HashMap<Uuid, i64> = repartir_entre_payeurs(frais, &poids).into_iter().collect();
    for (compte, _) in &parts {
        let paye = paye_net(conn, etab, s.id, participant, *compte).await?;
        if paye <= 0 {
            continue;
        }
        // Révision : l'éventuelle proposition ouverte est écartée (remplacée), une retenue ne se recalcule pas.
        let ouverte: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT id, statut FROM ent_sortie_avoir_propose
              WHERE etablissement_id = $1 AND participant_id = $2 AND compte_id = $3
                AND cle LIKE $4 AND statut IN ('propose', 'retenu', 'a_revoir') ORDER BY cree_le DESC LIMIT 1",
        )
        .bind(etab)
        .bind(participant)
        .bind(compte)
        .bind(format!("{prefixe}:{participant}:{compte}:r%"))
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if let Some((id, statut)) = &ouverte {
            if statut == "retenu" {
                // F1 : un désistement n'échoue JAMAIS parce qu'un avoir a été retenu (message neutre côté frais réels).
                if strict {
                    return Err(refus(RefusMetier::Conflit, "Cet avoir a déjà été décidé : la suite se traite dans la comptabilité."));
                }
                continue;
            }
            // F2 : un écartement fait par le calcul est « signé système » (aucun acteur humain).
            sqlx::query(
                "UPDATE ent_sortie_avoir_propose
                    SET statut = 'ecarte', decide_par_user_id = NULL, decide_le = now(), decision_auto = true,
                        decision_motif = 'remplacé par un nouveau calcul', montant_retenu_centimes = NULL
                  WHERE etablissement_id = $1 AND id = $2 AND statut IN ('propose', 'a_revoir')",
            )
            .bind(etab)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        } else {
            // F2 : l'écart HUMAIN est mémorisé : une proposition écartée par la comptabilité n'est pas relancée.
            let ecart_humain: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_sortie_avoir_propose
                                 WHERE etablissement_id = $1 AND participant_id = $2 AND compte_id = $3
                                   AND cle LIKE $4 AND statut = 'ecarte' AND NOT decision_auto)",
            )
            .bind(etab)
            .bind(participant)
            .bind(compte)
            .bind(format!("{prefixe}:{participant}:{compte}:r%"))
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            if ecart_humain {
                continue;
            }
        }
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_sortie_avoir_propose
              WHERE etablissement_id = $1 AND participant_id = $2 AND compte_id = $3 AND cle LIKE $4",
        )
        .bind(etab)
        .bind(participant)
        .bind(compte)
        .bind(format!("{prefixe}:{participant}:{compte}:r%"))
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        let cle = format!("{prefixe}:{participant}:{compte}:r{}", n + 1);
        let f = frais_par.get(compte).copied().unwrap_or(0);
        inserer_avoir(conn, etab, s.id, participant, *compte, cas, &cle, paye, f).await?;
    }
    Ok(())
}

// ───────────────────────── Lectures ─────────────────────────

fn vue_echeances(e: &[Echeance]) -> Vec<EcheanceVue> {
    e.iter().map(|x| EcheanceVue { date: x.date, centimes: x.centimes }).collect()
}

impl PgEnt {
    /// VOY-222 : état agrégé des paiements (R-SV18), visible du porteur, de la comptabilité, de la direction.
    /// Des nombres seulement : **aucun nom** associé à un retard.
    pub async fn voyage_argent_etat(&self, a: &Acteur<'_>, id: Uuid) -> Result<EtatAgrege, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !(acces.porteur || acces.a(Action::VoyagesArgent) || acces.a(Action::VoyagesLire)) {
            return Err(introuvable());
        }
        // M2 : une lecture n'écrit rien ; les parts sont calculées par les événements (inscription, prix figé, baisse,
        // désistement) ou par la comptabilité (`POST …/argent/synchroniser`).
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let (lignes, etats) = calculer(&mut t, a.etablissement_id, &s, jour).await?;
        let payants = etats.values().filter(|e| ETATS_PAYANTS.contains(&e.as_str())).count() as i64;
        // Pire état par participant.
        let mut par: HashMap<Uuid, EtatPaiement> = HashMap::new();
        let (mut du, mut verse, mut reste) = (0i64, 0i64, 0i64);
        for l in &lignes {
            du = somme(du, l.sit.du_centimes)?;
            verse = somme(verse, l.sit.verse_centimes)?;
            reste = somme(reste, l.sit.reste_centimes)?;
            let e = par.entry(l.part.participant_id).or_insert(l.sit.etat);
            if l.sit.etat.gravite() > e.gravite() {
                *e = l.sit.etat;
            }
        }
        let n = |x: EtatPaiement| par.values().filter(|e| **e == x).count() as i64;
        let vue = EtatAgrege {
            participants_payants: payants,
            soldes: n(EtatPaiement::Solde),
            a_jour: n(EtatPaiement::AJour),
            plans_en_cours: n(EtatPaiement::PlanEnCours),
            en_retard: n(EtatPaiement::EnRetard),
            a_definir: if s.nature == "obligatoire" { 0 } else { (payants - par.len() as i64).max(0) },
            du_total_centimes: du,
            verse_total_centimes: verse,
            reste_total_centimes: reste,
            gratuite: s.nature == "obligatoire",
            masque: false,
        };
        let vue = if !peut_argent(&acces) && payants < 3 && !vue.gratuite {
            EtatAgrege {
                participants_payants: payants,
                soldes: 0,
                a_jour: 0,
                plans_en_cours: 0,
                en_retard: 0,
                a_definir: 0,
                du_total_centimes: 0,
                verse_total_centimes: 0,
                reste_total_centimes: 0,
                gratuite: false,
                masque: true,
            }
        } else {
            vue
        };
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Le responsable désigné : le porteur de la sortie, **si** la direction a ouvert la liste nominative.
    async fn nominatif_ouvert_a(&self, conn: &mut PgConnection, a: &Acteur<'_>, s: &SortieLigne, acces: &Acces) -> Result<bool, EntDbError> {
        if !acces.porteur {
            return Ok(false);
        }
        let ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_nominatif_ouverture o
                             JOIN ent_personne p ON p.etablissement_id = o.etablissement_id AND p.id = o.personne_id
                            WHERE o.etablissement_id = $1 AND o.sortie_id = $2 AND o.ferme_le IS NULL
                              AND p.user_id = $3)",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(a.user_id)
        .fetch_one(conn)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    pub(crate) async fn acces_nominatif(&self, conn: &mut PgConnection, a: &Acteur<'_>, sortie: Option<Uuid>, portee: &str, n: usize) -> Result<(), EntDbError> {
        sqlx::query(
            "INSERT INTO ent_sortie_argent_acces (id, etablissement_id, sortie_id, user_id, role, portee, nb_lignes)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(sortie)
        .bind(a.user_id)
        .bind(a.role)
        .bind(portee)
        .bind(n as i32)
        .execute(conn)
        .await
        .map_err(db)?;
        Ok(())
    }

    /// Liste **nominative** des paiements : comptabilité, direction, ou responsable désigné par la direction.
    /// Chaque consultation est journalisée (qui, quand, combien de lignes).
    pub async fn voyage_argent_paiements(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<LignePaiement>, EntDbError> {
        Ok(self.lire_paiements(a, id, true).await?.0)
    }

    /// Liste nominative selon le droit : DTO complet pour `voyages.argent`, DTO réduit pour le responsable désigné (E1).
    pub async fn voyage_argent_paiements_vue(&self, a: &Acteur<'_>, id: Uuid) -> Result<PaiementsVue, EntDbError> {
        let (v, argent) = self.lire_paiements(a, id, false).await?;
        if argent {
            return Ok(PaiementsVue::Complet(v));
        }
        Ok(PaiementsVue::Responsable(
            v.into_iter()
                .map(|l| LignePaiementResponsable {
                    participant_id: l.participant_id,
                    eleve_nom: l.eleve_nom,
                    eleve_prenom: l.eleve_prenom,
                    compte_id: l.compte_id,
                    payeur: l.payeur,
                    part_centimes: l.part_centimes,
                    verse_centimes: l.verse_centimes,
                    reste_centimes: l.reste_centimes,
                    retard_centimes: l.retard_centimes,
                    etat: if l.etat == "plan_en_cours" { "a_jour".to_string() } else { l.etat },
                })
                .collect(),
        ))
    }

    async fn lire_paiements(&self, a: &Acteur<'_>, id: Uuid, exiger_argent: bool) -> Result<(Vec<LignePaiement>, bool), EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        let argent = peut_argent(&acces);
        if exiger_argent && !argent {
            return Err(introuvable());
        }
        if !(argent || self.nominatif_ouvert_a(&mut t, a, &s, &acces).await?) {
            return Err(introuvable());
        }
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let (lignes, _) = calculer(&mut t, a.etablissement_id, &s, jour).await?;
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.part.participant_id).collect();
        let noms: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT sp.id, e.nom, e.prenom FROM ent_sortie_participant sp
               JOIN ent_eleve e ON e.etablissement_id = sp.etablissement_id AND e.id = sp.eleve_id
              WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2 AND sp.id = ANY($3)",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(&ids)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let noms: HashMap<Uuid, (String, String)> = noms.into_iter().map(|(i, n, p)| (i, (n, p))).collect();
        let mut out: Vec<LignePaiement> = lignes
            .into_iter()
            .map(|l| {
                let (nom, prenom) = noms.get(&l.part.participant_id).cloned().unwrap_or_default();
                LignePaiement {
                    participant_id: l.part.participant_id,
                    eleve_nom: nom,
                    eleve_prenom: prenom,
                    compte_id: l.part.compte_id,
                    payeur: l.part.libelle,
                    part_centimes: l.part.montant_centimes,
                    verse_centimes: l.verse,
                    reste_centimes: l.sit.reste_centimes,
                    retard_centimes: l.sit.retard_centimes,
                    etat: l.sit.etat.code().to_string(),
                    nb_echeances: l.nb,
                    plan_long: l.plan_long,
                    echeances: vue_echeances(&l.echeances),
                }
            })
            .collect();
        out.sort_by(|x, y| (&x.eleve_nom, &x.eleve_prenom, &x.payeur).cmp(&(&y.eleve_nom, &y.eleve_prenom, &y.payeur)));
        self.acces_nominatif(&mut t, a, Some(s.id), "liste_paiements", out.len()).await?;
        t.commit().await.map_err(db)?;
        Ok((out, argent))
    }

    /// Espace famille : **ma** part et **mes** versements pour une sortie. Un autre payeur du même élève
    /// (parents séparés) n'apparaît jamais : le filtre porte sur les comptes dont la personne connectée est payeur.
    pub async fn voyage_argent_famille(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<MaPart>, EntDbError> {
        if Role::parse(a.role) != Some(Role::Parent) && Role::parse(a.role) != Some(Role::Eleve) {
            return Err(introuvable());
        }
        let mut t = tx(self, a).await?;
        let mes: Vec<Uuid> = crate::facturation_comptes::comptes_du_user(&mut t, a.etablissement_id, a.user_id)
            .await?
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        if mes.is_empty() {
            return Err(introuvable());
        }
        let s: SortieLigne = sqlx::query_as("SELECT * FROM ent_sortie WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let (lignes, _) = calculer(&mut t, a.etablissement_id, &s, jour).await?;
        let mut out = Vec::new();
        for l in lignes.into_iter().filter(|l| mes.contains(&l.part.compte_id)) {
            let prenom: String = sqlx::query_scalar(
                "SELECT e.prenom FROM ent_sortie_participant sp
                   JOIN ent_eleve e ON e.etablissement_id = sp.etablissement_id AND e.id = sp.eleve_id
                  WHERE sp.etablissement_id = $1 AND sp.id = $2",
            )
            .bind(a.etablissement_id)
            .bind(l.part.participant_id)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            let versements: Vec<VersementFamille> = sqlx::query_as(
                "SELECT id, sens, moyen, montant_centimes, date_valeur FROM ent_sortie_versement
                  WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4
                  ORDER BY saisi_le, id",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(l.part.participant_id)
            .bind(l.part.compte_id)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            out.push(MaPart {
                participant_id: l.part.participant_id,
                eleve_prenom: prenom,
                compte_id: l.part.compte_id,
                payeur: l.part.libelle,
                part_centimes: l.part.montant_centimes,
                verse_centimes: l.verse,
                reste_centimes: l.sit.reste_centimes,
                etat: l.sit.etat.code().to_string(),
                echeances: vue_echeances(&l.echeances),
                versements,
            });
        }
        t.commit().await.map_err(db)?;
        if out.is_empty() {
            return Err(introuvable());
        }
        Ok(out)
    }

    /// Calcule les parts maintenant (comptabilité, direction). Idempotent.
    pub async fn voyage_argent_synchroniser(&self, a: &Acteur<'_>, id: Uuid) -> Result<EtatAgrege, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        synchroniser(&mut t, a, &s).await?;
        t.commit().await.map_err(db)?;
        self.voyage_argent_etat(a, id).await
    }

    // ───────────────────────── Versements (VOY-267) ─────────────────────────

    /// Pointe un versement sur la part d'un payeur, **sans facture**. `voyages.argent` (comptabilité, direction).
    pub async fn voyage_versement_pointer(&self, a: &Acteur<'_>, id: Uuid, v: VersementSaisie) -> Result<VersementVue, EntDbError> {
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let r = pointer_tx(&mut t, a, &s, &v, None).await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Annule un versement par une écriture **inverse motivée** : l'original n'est jamais modifié.
    pub async fn voyage_versement_annuler(&self, a: &Acteur<'_>, id: Uuid, versement_id: Uuid, motif: &str) -> Result<VersementVue, EntDbError> {
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        verrou_sortie(&mut t, s.id).await?;
        let o: VersementVue = sqlx::query_as(
            "SELECT id, participant_id, compte_id, sens, moyen, montant_centimes, date_valeur, reference, annule_id, motif, saisi_le
               FROM ent_sortie_versement WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(versement_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if o.sens != "versement" {
            return Err(invalide());
        }
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_versement WHERE etablissement_id = $1 AND annule_id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(versement_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit());
        }
        let jour = jour_etab(&mut t, a.etablissement_id).await?;
        let nid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_versement
                 (id, etablissement_id, sortie_id, participant_id, compte_id, sens, moyen, montant_centimes,
                  date_valeur, reference, annule_id, motif, saisi_par_user_id)
             VALUES ($1,$2,$3,$4,$5,'annulation',$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(nid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(o.participant_id)
        .bind(o.compte_id)
        .bind(&o.moyen)
        .bind(o.montant_centimes)
        .bind(jour)
        .bind(&o.reference)
        .bind(versement_id)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        // M5 : tout avoir proposé ou retenu pour ce couple participant/payeur repose sur un versé qui change : « à revoir ».
        sqlx::query(
            "UPDATE ent_sortie_avoir_propose SET statut = 'a_revoir'
              WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4
                AND statut IN ('propose', 'retenu')",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(o.participant_id)
        .bind(o.compte_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let vue = lire_versement(&mut t, a.etablissement_id, nid).await?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    pub async fn voyage_versements(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<VersementVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let v: Vec<VersementVue> = sqlx::query_as(
            "SELECT id, participant_id, compte_id, sens, moyen, montant_centimes, date_valeur, reference, annule_id, motif, saisi_le
               FROM ent_sortie_versement WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY saisi_le, id",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        self.acces_nominatif(&mut t, a, Some(s.id), "liste_paiements", v.len()).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    // ───────────────────────── Plan de paiement ─────────────────────────

    /// Accorde un plan de paiement à un payeur : standard (1 à 3 fois) ou **long** (4 à 10 fois, motif obligatoire).
    /// L'organisateur ne le voit jamais ; il voit « plans en cours » dans l'état agrégé.
    pub async fn voyage_plan_accorder(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        participant_id: Uuid,
        compte_id: Uuid,
        nb_echeances: i32,
        motif: Option<&str>,
    ) -> Result<(), EntDbError> {
        if !(1..=ECHEANCES_MAX).contains(&nb_echeances) {
            return Err(invalide());
        }
        let motif = texte_opt(motif, 500)?;
        if nb_echeances > ECHEANCES_STANDARD_MAX && motif.is_none() {
            return Err(refus(RefusMetier::Invalide, "Un plan long (plus de trois fois) s'accorde avec un motif."));
        }
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        verrou_sortie(&mut t, s.id).await?;
        synchroniser(&mut t, a, &s).await?;
        let existe: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_part
                             WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4)",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(participant_id)
        .bind(compte_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if !existe {
            return Err(introuvable());
        }
        sqlx::query(
            "INSERT INTO ent_sortie_plan
                 (id, etablissement_id, sortie_id, participant_id, compte_id, rang, nb_echeances, motif, accorde_par_user_id)
             VALUES ($1,$2,$3,$4,$5,
                     (SELECT COALESCE(max(rang), 0) + 1 FROM ent_sortie_plan
                       WHERE etablissement_id = $2 AND participant_id = $4 AND compte_id = $5),
                     $6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(participant_id)
        .bind(compte_id)
        .bind(nb_echeances)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Frais réels et avoirs proposés (VOY-273, §12.5) ─────────────────────────

    /// VOY-273 : enregistre un frais réel d'un désistement (billet nominatif, pénalité) ; recalcule l'avoir proposé.
    pub async fn voyage_frais_saisir(&self, a: &Acteur<'_>, id: Uuid, f: FraisSaisie) -> Result<FraisVue, EntDbError> {
        let montant = centimes_de_texte(&f.montant).filter(|m| *m > 0 && *m <= 100_000_000_000).ok_or_else(invalide)?;
        if !["billet_nominatif", "penalite_prestataire", "hebergement_non_remboursable", "autre"].contains(&f.nature.as_str()) {
            return Err(invalide());
        }
        let libelle = texte(&f.libelle, 200)?;
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        verrou_sortie(&mut t, s.id).await?;
        controle_participant_parti(&mut t, a, s.id, f.participant_id).await?;
        let fid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_sortie_frais_reel
                 (id, etablissement_id, sortie_id, participant_id, nature, libelle, montant_centimes, saisi_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(fid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(f.participant_id)
        .bind(&f.nature)
        .bind(&libelle)
        .bind(montant)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        recalculer_depart(&mut t, a, &s, f.participant_id).await?;
        let vue = sqlx::query_as::<_, FraisVue>(
            "SELECT id, participant_id, nature, libelle, montant_centimes, annule_id, saisi_le
               FROM ent_sortie_frais_reel WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(fid)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// Annule un frais réel (écriture inverse motivée) et recalcule l'avoir proposé.
    pub async fn voyage_frais_annuler(&self, a: &Acteur<'_>, id: Uuid, frais_id: Uuid, motif: &str) -> Result<(), EntDbError> {
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        verrou_sortie(&mut t, s.id).await?;
        let o: Option<(Uuid, String, String, i64, Option<Uuid>)> = sqlx::query_as(
            "SELECT participant_id, nature, libelle, montant_centimes, annule_id FROM ent_sortie_frais_reel
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(frais_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (participant, nature, libelle, montant, annule) = o.ok_or_else(introuvable)?;
        if annule.is_some() {
            return Err(invalide());
        }
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_frais_reel WHERE etablissement_id = $1 AND annule_id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(frais_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit());
        }
        sqlx::query(
            "INSERT INTO ent_sortie_frais_reel
                 (id, etablissement_id, sortie_id, participant_id, nature, libelle, montant_centimes, annule_id, motif, saisi_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(participant)
        .bind(&nature)
        .bind(&libelle)
        .bind(montant)
        .bind(frais_id)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        recalculer_depart(&mut t, a, &s, participant).await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    pub async fn voyage_frais(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<FraisVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let v = sqlx::query_as::<_, FraisVue>(
            "SELECT id, participant_id, nature, libelle, montant_centimes, annule_id, saisi_le
               FROM ent_sortie_frais_reel WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY saisi_le, id",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        self.acces_nominatif(&mut t, a, Some(s.id), "frais", v.len()).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    pub async fn voyage_avoirs(&self, a: &Acteur<'_>, id: Uuid) -> Result<Vec<AvoirVue>, EntDbError> {
        let mut t = tx(self, a).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, false).await?;
        if !peut_argent(&acces) {
            return Err(introuvable());
        }
        let v = lire_avoirs(&mut t, a.etablissement_id, s.id).await?;
        self.acces_nominatif(&mut t, a, Some(s.id), "avoirs", v.len()).await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Décision de la comptabilité sur un avoir proposé : retenu (montant ≤ ce qui a été versé) ou écarté, motivée.
    /// ScolIA n'émet aucun avoir : la décision est exportée, la comptabilité l'écrit dans Charlemagne.
    pub async fn voyage_avoir_decider(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        avoir_id: Uuid,
        retenir: bool,
        montant_retenu: Option<&str>,
        motif_code: Option<&str>,
        motif: &str,
    ) -> Result<AvoirVue, EntDbError> {
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        let (statut, propose, participant, compte): (String, i64, Uuid, Uuid) = sqlx::query_as(
            "SELECT statut, montant_propose_centimes, participant_id, compte_id FROM ent_sortie_avoir_propose
              WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(avoir_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if !matches!(statut.as_str(), "propose" | "a_revoir") {
            return Err(conflit());
        }
        let code = match motif_code.map(str::trim).filter(|c| !c.is_empty()) {
            None => None,
            Some(c @ ("frais_non_dus" | "erreur_de_calcul" | "geste_commercial" | "autre_justifie")) => Some(c.to_string()),
            Some(_) => return Err(invalide()),
        };
        let retenu = if retenir {
            let m = match montant_retenu {
                Some(x) => centimes_de_texte(x).ok_or_else(invalide)?,
                None => propose,
            };
            // M5 : la borne est le versé NET COURANT du couple, jamais le versé figé au moment de la proposition.
            let paye = paye_net(&mut t, a.etablissement_id, s.id, participant, compte).await?;
            if m > paye {
                return Err(refus(RefusMetier::Invalide, "L'avoir ne dépasse jamais ce que le payeur a versé (net actuel)."));
            }
            if m > propose && code.is_none() {
                return Err(refus(RefusMetier::Invalide, "Retenir plus que le montant proposé demande un motif codé."));
            }
            Some(m)
        } else {
            None
        };
        sqlx::query(
            "UPDATE ent_sortie_avoir_propose
                SET statut = $3, montant_retenu_centimes = $4, decide_par_user_id = $5, decide_le = now(), decision_motif = $6,
                    decision_code = $7, decision_auto = false
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(avoir_id)
        .bind(if retenir { "retenu" } else { "ecarte" })
        .bind(retenu)
        .bind(a.user_id)
        .bind(&motif)
        .bind(&code)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let v = lire_avoirs(&mut t, a.etablissement_id, s.id).await?;
        t.commit().await.map_err(db)?;
        v.into_iter().find(|x| x.id == avoir_id).ok_or_else(introuvable)
    }

    /// Annulation collective par l'établissement : avoir total proposé à chaque payeur ayant versé (§12.5).
    /// Réservé aux sorties **annulées**.
    pub async fn voyage_avoirs_annulation_collective(&self, a: &Acteur<'_>, id: Uuid) -> Result<usize, EntDbError> {
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        if s.etat != "annulee" {
            return Err(refus(RefusMetier::Conflit, "L'annulation collective suppose une sortie annulée."));
        }
        verrou_sortie(&mut t, s.id).await?;
        let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_avoir_propose WHERE etablissement_id = $1 AND sortie_id = $2")
            .bind(a.etablissement_id)
            .bind(s.id)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        let ps: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT participant_id FROM ent_sortie_versement WHERE etablissement_id = $1 AND sortie_id = $2",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        for p in ps {
            // Les cles « collective » sont idempotentes : un second appel n'ajoute rien.
            collective_participant(&mut t, a, &s, p).await?;
        }
        let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_sortie_avoir_propose WHERE etablissement_id = $1 AND sortie_id = $2")
            .bind(a.etablissement_id)
            .bind(s.id)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok((apres - avant) as usize)
    }

    // ───────────────────────── Accès nominatif (VOY-223) ─────────────────────────

    /// La direction ouvre la liste nominative des paiements au **seul** responsable du séjour (le porteur), avec motif.
    pub async fn voyage_nominatif_ouvrir(&self, a: &Acteur<'_>, id: Uuid, motif: &str) -> Result<NominatifVue, EntDbError> {
        let motif = texte(motif, 500)?;
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.a(Action::VoyagesAutoriser) {
            return Err(interdit());
        }
        let oid = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_sortie_nominatif_ouverture (id, etablissement_id, sortie_id, personne_id, motif, ouvert_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(oid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(s.porteur_personne_id)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await;
        match r {
            Ok(_) => {}
            Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23505") => return Err(conflit()),
            Err(e) => return Err(db(e)),
        }
        let v = sqlx::query_as::<_, NominatifVue>(
            "SELECT id, personne_id, motif, ouvert_le, ferme_le FROM ent_sortie_nominatif_ouverture
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(oid)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    pub async fn voyage_nominatif_fermer(&self, a: &Acteur<'_>, id: Uuid) -> Result<(), EntDbError> {
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !acces.a(Action::VoyagesAutoriser) {
            return Err(interdit());
        }
        let r = sqlx::query(
            "UPDATE ent_sortie_nominatif_ouverture SET ferme_le = now(), ferme_par_user_id = $3
              WHERE etablissement_id = $1 AND sortie_id = $2 AND ferme_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        t.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Conservation de l'export (F7) ─────────────────────────

    /// Détruit les fichiers d'export de plus de 30 jours d'un établissement (objet supprimé, date de purge inscrite).
    pub(crate) async fn exports_purger_tx(&self, conn: &mut PgConnection, etab: Uuid) -> Result<usize, EntDbError> {
        let vieux: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, cle_objet FROM ent_sortie_argent_export
              WHERE etablissement_id = $1 AND purge_le IS NULL AND cree_le < now() - interval '30 days' ORDER BY cree_le",
        )
        .bind(etab)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (id, cle) in &vieux {
            self.objet_supprimer(cle).await?;
            sqlx::query("UPDATE ent_sortie_argent_export SET purge_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
        }
        Ok(vieux.len())
    }

    /// Passage de purge d'un établissement (catégorie de conservation `sor.export_argent`, 30 jours).
    pub async fn voyages_argent_exports_purger(&self, etab: Uuid) -> Result<usize, EntDbError> {
        let mut t = crate::tenant_tx::TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = self.exports_purger_tx(&mut t, etab).await?;
        t.commit().await.map_err(db)?;
        Ok(n)
    }

    /// Passage de purge sur tous les établissements (une fois par jour).
    pub async fn voyages_argent_exports_purger_tous(&self) -> Result<usize, EntDbError> {
        let etabs = crate::tenant_tx::etablissements_systeme(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("voyages_argent_exports"),
        )
        .await
        .map_err(db)?;
        let mut n = 0;
        for e in etabs {
            n += self.voyages_argent_exports_purger(e).await?;
        }
        Ok(n)
    }

    // ───────────────────────── Export vers la comptabilité (VOY-268) ─────────────────────────

    /// Exporte parts, versements, frais réels et avoirs proposés en CSV lisible (UTF-8 avec BOM, séparateur `;`,
    /// montants à la française). Droits `voyages.argent`, **lien court** émis après contrôle, export journalisé.
    pub async fn voyage_argent_export(&self, a: &Acteur<'_>, id: Uuid) -> Result<ExportFait, EntDbError> {
        if !self.presigner().is_enabled() {
            return Err(EntDbError::LiensIndisponibles);
        }
        let mut t = tx(self, a).await?;
        // M1 : un seul ordre de verrous partout : le verrou consultatif de la sortie AVANT tout FOR UPDATE.
        verrou_sortie(&mut t, id).await?;
        let (s, acces) = charger_avec_acces(self, &mut t, a, id, true).await?;
        if !peut_argent(&acces) {
            return Err(interdit());
        }
        // F7 : les exports de plus de 30 jours sont détruits avant d'en écrire un nouveau.
        self.exports_purger_tx(&mut t, a.etablissement_id).await?;
        let csv = construire_csv(&mut t, a, &s).await?;
        let nb = csv.lignes;
        let octets = csv.texte.into_bytes();
        let eid = Uuid::new_v4();
        let cle = ObjectStore::key_for(a.etablissement_id, "voyages-argent", eid, "csv");
        let cle_objet = cle.clone();
        let info = self.objet_put(&cle, &octets).await?;
        crate::objets::enregistrer_objet(&mut *t, a.etablissement_id, &info, Some("text/csv")).await?;
        sqlx::query(
            "INSERT INTO ent_sortie_argent_export (id, etablissement_id, sortie_id, nb_lignes, cle_objet, sha256, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(eid)
        .bind(a.etablissement_id)
        .bind(s.id)
        .bind(nb as i32)
        .bind(&cle)
        .bind(&info.sha256)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        self.acces_nominatif(&mut t, a, Some(s.id), "export", nb).await?;
        let lien = self
            .presigner()
            .issue(
                &LinkSpec {
                    key: cle,
                    user_id: a.user_id,
                    etablissement_id: a.etablissement_id,
                    disposition: Disposition::Attachment,
                    filename: format!("{}-suivi-argent.csv", s.reference),
                    mime: "text/csv".to_string(),
                },
                ent_object::presign::now_unix(),
            )
            .map_err(|e| match e {
                PresignError::Disabled => EntDbError::LiensIndisponibles,
                _ => EntDbError::FichierAbsent,
            })?;
        // F7 : l'objet est déjà au magasin ; si la validation échoue, il est balayé tout de suite (jamais d'orphelin).
        if let Err(e) = t.commit().await {
            let _ = self.objet_supprimer(&cle_objet).await;
            return Err(db(e));
        }
        let LienFichier { url, expires_at } = LienFichier { url: lien.url, expires_at: lien.expires_at };
        Ok(ExportFait { lien: url, expire_a: expires_at, nb_lignes: nb, sha256: info.sha256 })
    }
}

// ───────────────────────── Écritures partagées ─────────────────────────

async fn lire_versement(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<VersementVue, EntDbError> {
    sqlx::query_as(
        "SELECT id, participant_id, compte_id, sens, moyen, montant_centimes, date_valeur, reference, annule_id, motif, saisi_le
           FROM ent_sortie_versement WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(conn)
    .await
    .map_err(db)
}

async fn lire_avoirs(conn: &mut PgConnection, etab: Uuid, sortie: Uuid) -> Result<Vec<AvoirVue>, EntDbError> {
    sqlx::query_as(
        "SELECT id, participant_id, compte_id, motif_code, paye_net_centimes, frais_retenus_centimes,
                montant_propose_centimes, statut, montant_retenu_centimes, decision_motif, cree_le
           FROM ent_sortie_avoir_propose WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY cree_le, id",
    )
    .bind(etab)
    .bind(sortie)
    .fetch_all(conn)
    .await
    .map_err(db)
}

/// Un participant parti (désisté ou retiré) dans cette sortie : condition de saisie des frais réels.
async fn controle_participant_parti(conn: &mut PgConnection, a: &Acteur<'_>, sortie: Uuid, participant: Uuid) -> Result<(), EntDbError> {
    let etat: Option<String> = sqlx::query_scalar(
        "SELECT etat FROM ent_sortie_participant WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3",
    )
    .bind(a.etablissement_id)
    .bind(sortie)
    .bind(participant)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    match etat.as_deref() {
        None => Err(introuvable()),
        Some("desiste" | "retire") => Ok(()),
        Some(_) => Err(refus(RefusMetier::Conflit, "Les frais réels se saisissent pour un participant qui s'est désisté ou a été retiré.")),
    }
}

/// Recalcule l'avoir d'un participant parti après un frais réel (VOY-273).
async fn recalculer_depart(conn: &mut PgConnection, a: &Acteur<'_>, s: &SortieLigne, participant: Uuid) -> Result<(), EntDbError> {
    let (etat, motif): (String, Option<String>) = sqlx::query_as(
        "SELECT sp.etat, (SELECT f.motif_code FROM ent_sortie_participant_fait f
                           WHERE f.etablissement_id = sp.etablissement_id AND f.participant_id = sp.id
                             AND f.type IN ('desistement', 'retrait') ORDER BY f.a DESC LIMIT 1)
           FROM ent_sortie_participant sp WHERE sp.etablissement_id = $1 AND sp.id = $2",
    )
    .bind(a.etablissement_id)
    .bind(participant)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let engage = depenses_engagees(conn, a.etablissement_id, s.id).await?;
    let cas = cas_depart(&etat, motif.as_deref().unwrap_or("autre"), engage);
    // L'engagement est forcé quand des frais réels existent : ils ne sont dus qu'après engagement des dépenses.
    let cas = match cas {
        CasAvoir::DesistementAvantEngagement if frais_total(conn, a.etablissement_id, s.id, participant).await? > 0 => {
            CasAvoir::DesistementApresEngagement
        }
        autre => autre,
    };
    proposer_avoirs_participant(conn, a, s, participant, cas, "depart", true).await
}

async fn collective_participant(conn: &mut PgConnection, a: &Acteur<'_>, s: &SortieLigne, participant: Uuid) -> Result<(), EntDbError> {
    let etab = a.etablissement_id;
    let comptes: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT compte_id FROM ent_sortie_versement WHERE etablissement_id = $1 AND participant_id = $2",
    )
    .bind(etab)
    .bind(participant)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for compte in comptes {
        let paye = paye_net(conn, etab, s.id, participant, compte).await?;
        // Un payeur déjà couvert par un avoir proposé ou retenu (désistement) n'en reçoit pas un second.
        let couvert: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_sortie_avoir_propose
                             WHERE etablissement_id = $1 AND participant_id = $2 AND compte_id = $3
                               AND statut IN ('propose', 'retenu', 'a_revoir'))",
        )
        .bind(etab)
        .bind(participant)
        .bind(compte)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if paye > 0 && !couvert {
            let cle = format!("collective:{participant}:{compte}");
            inserer_avoir(conn, etab, s.id, participant, compte, CasAvoir::AnnulationCollective, &cle, paye, 0).await?;
        }
    }
    Ok(())
}

/// Écriture d'un versement dans la transaction de l'appelant (aussi utilisée pour une ligne de bordereau).
pub(crate) async fn pointer_tx(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    s: &SortieLigne,
    v: &VersementSaisie,
    bordereau_ligne: Option<Uuid>,
) -> Result<VersementVue, EntDbError> {
    let etab = a.etablissement_id;
    if s.nature == "obligatoire" {
        return Err(refus(RefusMetier::Conflit, "Sortie obligatoire : rien n'est dû par les familles."));
    }
    if !MOYENS.contains(&v.moyen.as_str()) {
        return Err(invalide());
    }
    let montant = centimes_de_texte(&v.montant).filter(|m| (1..=VERSEMENT_MAX).contains(m)).ok_or_else(invalide)?;
    let reference = texte_opt(v.reference.as_deref(), 40)?;
    let regl = reglages(conn, etab).await?;
    if v.moyen == "cheques_vacances" && !regl.cheques_vacances {
        return Err(refus(RefusMetier::Interdit, "Les chèques-vacances ne sont acceptés que si l'établissement est agréé ANCV."));
    }
    verrou_sortie(conn, s.id).await?;
    synchroniser(conn, a, s).await?;
    let jour = jour_etab(conn, etab).await?;
    let date = v.date_valeur.unwrap_or(jour);
    if date > jour {
        return Err(refus(RefusMetier::Invalide, "La date d'un versement ne peut pas être dans le futur."));
    }
    if date < jour - chrono::Duration::days(366) {
        return Err(refus(RefusMetier::Invalide, "Date de versement trop ancienne."));
    }
    let etat: Option<String> = sqlx::query_scalar(
        "SELECT etat FROM ent_sortie_participant WHERE etablissement_id = $1 AND sortie_id = $2 AND id = $3 FOR UPDATE",
    )
    .bind(etab)
    .bind(s.id)
    .bind(v.participant_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let etat = etat.ok_or_else(introuvable)?;
    if !ETATS_PAYANTS.contains(&etat.as_str()) {
        return Err(refus(RefusMetier::Conflit, "Ce participant n'est plus inscrit : aucun versement ne se pointe (voir les avoirs proposés)."));
    }
    // La part courante de CE payeur.
    let part: Option<(i64,)> = sqlx::query_as(
        "SELECT montant_centimes FROM ent_sortie_part
          WHERE etablissement_id = $1 AND sortie_id = $2 AND participant_id = $3 AND compte_id = $4
          ORDER BY prix_rang DESC LIMIT 1",
    )
    .bind(etab)
    .bind(s.id)
    .bind(v.participant_id)
    .bind(v.compte_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let part = part.ok_or_else(introuvable)?.0;
    let deja = paye_net(conn, etab, s.id, v.participant_id, v.compte_id).await?;
    if somme(deja, montant)? > part {
        return Err(refus(RefusMetier::Conflit, "Ce versement dépasse la part de ce payeur : vérifiez le montant ou le payeur."));
    }
    let vid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_sortie_versement
             (id, etablissement_id, sortie_id, participant_id, compte_id, sens, moyen, montant_centimes,
              date_valeur, reference, bordereau_ligne_id, saisi_par_user_id)
         VALUES ($1,$2,$3,$4,$5,'versement',$6,$7,$8,$9,$10,$11)",
    )
    .bind(vid)
    .bind(etab)
    .bind(s.id)
    .bind(v.participant_id)
    .bind(v.compte_id)
    .bind(&v.moyen)
    .bind(montant)
    .bind(date)
    .bind(&reference)
    .bind(bordereau_ligne)
    .bind(a.user_id)
    .execute(&mut *conn)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(d) if d.code().as_deref() == Some("23505") => conflit(),
        e => db(e),
    })?;
    lire_versement(conn, etab, vid).await
}

// ───────────────────────── Export CSV ─────────────────────────

struct Csv {
    texte: String,
    lignes: usize,
}

async fn construire_csv(conn: &mut PgConnection, a: &Acteur<'_>, s: &SortieLigne) -> Result<Csv, EntDbError> {
    let etab = a.etablissement_id;
    let mut out = String::from("\u{feff}type;sortie;eleve_nom;eleve_prenom;payeur;date;moyen;reference;montant;etat;motif\r\n");
    let mut n = 0usize;
    let mut ligne = |champs: [&str; 11]| {
        out.push_str(&champs.iter().map(|c| champ_csv(c)).collect::<Vec<_>>().join(";"));
        out.push_str("\r\n");
        n += 1;
    };
    // Identité des participants et libellé des payeurs.
    let ident: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT sp.id, e.nom, e.prenom FROM ent_sortie_participant sp
           JOIN ent_eleve e ON e.etablissement_id = sp.etablissement_id AND e.id = sp.eleve_id
          WHERE sp.etablissement_id = $1 AND sp.sortie_id = $2",
    )
    .bind(etab)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let ident: HashMap<Uuid, (String, String)> = ident.into_iter().map(|(i, n, p)| (i, (n, p))).collect();
    let nom = |p: &Uuid| ident.get(p).cloned().unwrap_or_default();
    // F7 : seulement les comptes concernés par CETTE sortie (parts, versements, avoirs).
    let comptes: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT c.id, c.libelle_facturation FROM ent_fin_compte c
          WHERE c.etablissement_id = $1
            AND (EXISTS (SELECT 1 FROM ent_sortie_part p WHERE p.etablissement_id = c.etablissement_id AND p.compte_id = c.id AND p.sortie_id = $2)
              OR EXISTS (SELECT 1 FROM ent_sortie_versement v WHERE v.etablissement_id = c.etablissement_id AND v.compte_id = c.id AND v.sortie_id = $2)
              OR EXISTS (SELECT 1 FROM ent_sortie_avoir_propose x WHERE x.etablissement_id = c.etablissement_id AND x.compte_id = c.id AND x.sortie_id = $2))",
    )
    .bind(etab)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let comptes: HashMap<Uuid, String> = comptes.into_iter().collect();
    let payeur = |c: &Uuid| comptes.get(c).cloned().unwrap_or_default();

    let jour = jour_etab(conn, etab).await?;
    let (lignes, _) = calculer(conn, etab, s, jour).await?;
    for l in &lignes {
        let (nm, pr) = nom(&l.part.participant_id);
        ligne([
            "part", &s.reference, &nm, &pr, &payeur(&l.part.compte_id), "", "", "",
            &euros(l.part.montant_centimes), l.sit.etat.code(), "",
        ]);
    }
    let vs: Vec<VersementVue> = sqlx::query_as(
        "SELECT id, participant_id, compte_id, sens, moyen, montant_centimes, date_valeur, reference, annule_id, motif, saisi_le
           FROM ent_sortie_versement WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY saisi_le, id",
    )
    .bind(etab)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for v in &vs {
        let (nm, pr) = nom(&v.participant_id);
        ligne([
            &v.sens, &s.reference, &nm, &pr, &payeur(&v.compte_id), &v.date_valeur.to_string(), &v.moyen,
            v.reference.as_deref().unwrap_or(""), &euros(v.montant_centimes), "", v.motif.as_deref().unwrap_or(""),
        ]);
    }
    let fs: Vec<FraisVue> = sqlx::query_as(
        "SELECT id, participant_id, nature, libelle, montant_centimes, annule_id, saisi_le
           FROM ent_sortie_frais_reel WHERE etablissement_id = $1 AND sortie_id = $2 ORDER BY saisi_le, id",
    )
    .bind(etab)
    .bind(s.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for f in &fs {
        let (nm, pr) = nom(&f.participant_id);
        let t = if f.annule_id.is_some() { "frais_reel_annule" } else { "frais_reel" };
        ligne([t, &s.reference, &nm, &pr, "", "", &f.nature, "", &euros(f.montant_centimes), "", &f.libelle]);
    }
    for av in lire_avoirs(conn, etab, s.id).await? {
        let (nm, pr) = nom(&av.participant_id);
        let montant = av.montant_retenu_centimes.unwrap_or(av.montant_propose_centimes);
        // Un avoir n'est qu'un MONTANT PROPOSÉ : jamais une écriture comptable (R-SV44).
        ligne([
            "avoir_propose", &s.reference, &nm, &pr, &payeur(&av.compte_id), "", "", "", &euros(montant),
            &av.statut, &av.motif_code,
        ]);
    }
    Ok(Csv { texte: out, lignes: n })
}
