//! Finances reliées (socle 2, lot S2-9 ; FIN-01 → FIN-17 ; D-014, D-017).
//!
//! * **Paramétrage** (FIN-01, FIN-05, FIN-07) : articles, grilles tarifaires
//!   annuelles (niveau, régime, tranche de quotient familial), réductions
//!   (fratrie, personnel), barème des bourses, calendrier d'échéances.
//! * **Moteur de facturation** : une **source** (inscription validée, changement
//!   de régime, sortie, absence prolongée, consommation) produit **au plus une
//!   ligne par foyer payeur** (`ent_fac_ligne.source_cle`, unique en base). Les
//!   montants sont répartis entre les foyers payeurs selon leurs parts
//!   (S2-1 : `ent_eleve_lien.est_payeur` / `part_paiement`), au centime près.
//!   Les pièces produites sont des **brouillons** : l'émission (FIN-03 ★) est un
//!   geste humain (règle §4.1-6 : file de validation humaine).
//! * **Génération annuelle** (FIN-02 ★) à partir des inscriptions validées et
//!   **facturation par événements** (`finances_reactions.rs`) : les deux passent
//!   par les mêmes clés de source, donc rejouer l'un après l'autre ne double rien.
//! * **Avoirs** (FIN-04) : pièce `avoir` liée à la facture d'origine, imputée
//!   à l'émission (lettrage).
//!
//! SEPA, règlements, lettrage, relances et export comptable : `finances_sepa.rs`.
//! Calculs (prorata, arrondis, répartition) : `finances_calcul.rs`.
//!
//! Toute requête est bornée à l'établissement et passe par `TenantTx` (RLS,
//! migrations 100 et 206) ; une référence d'un autre établissement répond
//! « introuvable ».

#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use domain_eleve::{masquer_iban, DomainError};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::finances::*;
use serde_json::{json, Value};
use sqlx::PgConnection;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

use crate::finances_calcul::{self as calcul, Calendrier, TarifCandidat};
use crate::outbox::{Causalite, Publier};
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

pub const CATEGORIES: [&str; 7] = [
    "scolarite",
    "restauration",
    "internat",
    "activite",
    "frais",
    "bourse",
    "reduction",
];
pub const REGIMES: [&str; 3] = ["externe", "demi_pension", "interne"];

pub(crate) fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("finances", err))
}

pub(crate) fn invalide(msg: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(msg)
}

pub(crate) fn conflit(msg: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(msg)
}

pub(crate) fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

/// Jour courant **en heure de Paris** (F0, FD-24, R-F24) : un acte posé à 0 h 30
/// heure locale appartient au jour même, pas à la veille en UTC.
pub(crate) fn aujourd_hui() -> NaiveDate {
    date_paris(Utc::now())
}

pub(crate) use domain_eleve::horloge::date_paris;

fn est_doublon(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .and_then(|d| d.code())
        .is_some_and(|c| c == "23505")
}

fn annee_valide(a: &str) -> Result<String, EntDbError> {
    let a = a.trim();
    Calendrier::new(a, 9, 10)
        .map(|_| a.to_string())
        .ok_or(invalide("année scolaire attendue au format AAAA-AAAA"))
}

pub(crate) fn texte(s: &str, max: usize, msg: &'static str) -> Result<String, EntDbError> {
    let t = s.trim();
    if t.is_empty() || t.chars().count() > max {
        return Err(invalide(msg));
    }
    Ok(t.to_string())
}

pub(crate) fn opt_texte(s: &Option<String>, max: usize) -> Option<String> {
    s.as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.chars().take(max).collect())
}

/// Publication dans la transaction du fait (outbox S2-0).
pub(crate) async fn publier(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate: AggregateKind,
    aggregate_id: Uuid,
    eleve_id: Option<Uuid>,
    payload: Value,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<ent_events::MetierEvent, EntDbError> {
    Ok(conn
        .publier_avec(
            MetierEventDraft {
                etablissement_id: etab,
                type_name: type_name.to_string(),
                aggregate,
                aggregate_id,
                eleve_id,
                payload,
                actor_user_id: actor.map(|a| a.to_string()),
            },
            causalite,
        )
        .await?)
}

/// Numéro suivant d'une série continue (factures, avoirs, exports).
pub(crate) async fn prochain_numero(
    conn: &mut PgConnection,
    etab: Uuid,
    cle: &str,
) -> Result<i64, EntDbError> {
    let (n,): (i64,) = sqlx::query_as(
        "INSERT INTO ent_fin_compteur (etablissement_id, cle, dernier) VALUES ($1, $2, 1)
         ON CONFLICT (etablissement_id, cle) DO UPDATE SET dernier = ent_fin_compteur.dernier + 1
         RETURNING dernier",
    )
    .bind(etab)
    .bind(cle)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(n)
}

/// Verrou de transaction par foyer : sérialise les écritures d'un même compte.
pub(crate) async fn verrou_foyer(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("fin:{etab}:{foyer}"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Paramètres
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct Params {
    /// Identifiant de la ligne de paramètres (authentifie le chiffrement de l'IBAN créancier).
    pub id: Option<Uuid>,
    pub mois_debut: i16,
    pub nb_mois: i16,
    pub jour_echeance: i16,
    pub jours_prenotification: i16,
    pub frais_rejet_centimes: i64,
    pub rejets_avant_manuel: i16,
    pub seuil_absence_jours: i16,
    pub crediteur_nom: Option<String>,
    pub crediteur_iban: Option<String>,
    pub crediteur_bic: Option<String>,
    pub crediteur_ics: Option<String>,
    pub compte_clients: String,
    pub compte_banque: String,
    pub journal_ventes: String,
    pub journal_banque: String,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            id: None,
            mois_debut: 9,
            nb_mois: 10,
            jour_echeance: 5,
            jours_prenotification: 14,
            frais_rejet_centimes: 0,
            rejets_avant_manuel: 2,
            seuil_absence_jours: 15,
            crediteur_nom: None,
            crediteur_iban: None,
            crediteur_bic: None,
            crediteur_ics: None,
            compte_clients: "411000".into(),
            compte_banque: "512000".into(),
            journal_ventes: "VT".into(),
            journal_banque: "BQ".into(),
        }
    }
}

impl Params {
    pub fn calendrier(&self, annee: &str) -> Option<Calendrier> {
        Calendrier::new(annee, self.mois_debut as u32, self.nb_mois as u32)
    }
}

#[derive(sqlx::FromRow)]
struct ParamsRow {
    id: Uuid,
    mois_debut: i16,
    nb_mois: i16,
    jour_echeance: i16,
    jours_prenotification: i16,
    frais_rejet_centimes: i64,
    rejets_avant_manuel: i16,
    seuil_absence_jours: i16,
    crediteur_nom: Option<String>,
    crediteur_iban: Option<String>,
    crediteur_bic: Option<String>,
    crediteur_ics: Option<String>,
    compte_clients: String,
    compte_banque: String,
    journal_ventes: String,
    journal_banque: String,
}

const T_PARAMETRES: &str = "ent_fin_parametres";

pub(crate) async fn params_tx(conn: &mut PgConnection, etab: Uuid) -> Result<Params, EntDbError> {
    let r: Option<ParamsRow> = sqlx::query_as(
        "SELECT id, mois_debut, nb_mois, jour_echeance, jours_prenotification, frais_rejet_centimes,
                rejets_avant_manuel, seuil_absence_jours, crediteur_nom, crediteur_iban, crediteur_bic,
                crediteur_ics, compte_clients, compte_banque, journal_ventes, journal_banque
           FROM ent_fin_parametres WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(match r {
        None => Params::default(),
        Some(r) => Params {
            // F0 (FD-11, R-P21) : l'IBAN du créancier est chiffré au repos ; en mémoire il
            // est en clair le temps d'écrire le fichier bancaire, jamais renvoyé tel quel.
            crediteur_iban: crate::champs_chiffres::ChampsSensibles::depuis_env()
                .ouvrir_opt(T_PARAMETRES, "crediteur_iban", r.id, r.crediteur_iban)
                .map_err(EntDbError::from)?,
            id: Some(r.id),
            mois_debut: r.mois_debut,
            nb_mois: r.nb_mois,
            jour_echeance: r.jour_echeance,
            jours_prenotification: r.jours_prenotification,
            frais_rejet_centimes: r.frais_rejet_centimes,
            rejets_avant_manuel: r.rejets_avant_manuel,
            seuil_absence_jours: r.seuil_absence_jours,
            crediteur_nom: r.crediteur_nom,
            crediteur_bic: r.crediteur_bic,
            crediteur_ics: r.crediteur_ics,
            compte_clients: r.compte_clients,
            compte_banque: r.compte_banque,
            journal_ventes: r.journal_ventes,
            journal_banque: r.journal_banque,
        },
    })
}

/// Paliers de relance ; défaut documenté si l'établissement n'en a posé aucun.
pub(crate) async fn paliers_tx(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<Vec<PalierRelanceDto>, EntDbError> {
    let rows: Vec<(i16, i16, String, i64)> = sqlx::query_as(
        "SELECT rang, jours_retard, libelle, frais_centimes FROM ent_fin_palier_relance
          WHERE etablissement_id = $1 ORDER BY rang",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    if rows.is_empty() {
        return Ok([
            (1, 7, "Rappel amiable"),
            (2, 30, "Relance"),
            (3, 60, "Dernière relance avant mise en demeure"),
        ]
        .into_iter()
        .map(|(rang, jours_retard, libelle)| PalierRelanceDto {
            rang,
            jours_retard,
            libelle: libelle.into(),
            frais_centimes: 0,
        })
        .collect());
    }
    Ok(rows
        .into_iter()
        .map(
            |(rang, jours_retard, libelle, frais_centimes)| PalierRelanceDto {
                rang,
                jours_retard,
                libelle,
                frais_centimes,
            },
        )
        .collect())
}

// ---------------------------------------------------------------------------
// Moteur : lignes, factures brouillon, avoirs
// ---------------------------------------------------------------------------

/// Ligne à facturer, déjà répartie sur un foyer payeur.
#[derive(Debug, Clone)]
pub(crate) struct LigneAFacturer {
    pub foyer_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub article_id: Option<Uuid>,
    pub categorie: String,
    pub libelle: String,
    pub quantite: i32,
    pub unitaire: i64,
    pub source_cle: String,
    pub periode: Option<(NaiveDate, NaiveDate)>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct ResultatInsertion {
    pub creees: i64,
    pub deja: i64,
    pub factures: Vec<Uuid>,
    pub montant: i64,
}

impl ResultatInsertion {
    pub(crate) fn ajouter(&mut self, o: ResultatInsertion) {
        self.creees += o.creees;
        self.deja += o.deja;
        self.montant += o.montant;
        for f in o.factures {
            if !self.factures.contains(&f) {
                self.factures.push(f);
            }
        }
    }
}

async fn sources_existantes(
    conn: &mut PgConnection,
    etab: Uuid,
    cles: &[String],
) -> Result<Vec<String>, EntDbError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT source_cle FROM ent_fac_ligne WHERE etablissement_id = $1 AND source_cle = ANY($2)",
    )
    .bind(etab)
    .bind(cles)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

async fn recalculer_total(
    conn: &mut PgConnection,
    etab: Uuid,
    facture: Uuid,
) -> Result<i64, EntDbError> {
    let (t,): (i64,) = sqlx::query_as(
        "UPDATE ent_fac_facture SET total_centimes = (
             SELECT coalesce(sum(quantite::BIGINT * montant_unitaire_centimes), 0)::BIGINT
               FROM ent_fac_ligne WHERE etablissement_id = $1 AND facture_id = $2)
          WHERE etablissement_id = $1 AND id = $2 RETURNING total_centimes",
    )
    .bind(etab)
    .bind(facture)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(t)
}

/// Facture (ou avoir) brouillon d'un foyer pour une origine : retrouvée ou créée.
async fn piece_brouillon(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
    type_piece: &str,
    origine: &str,
    annee: Option<&str>,
    facture_origine: Option<Uuid>,
    libelle: &str,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<Uuid, EntDbError> {
    let existante: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_fac_facture
          WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'brouillon' AND type_piece = $3
            AND origine = $4 AND annee_scolaire IS NOT DISTINCT FROM $5
            AND facture_origine_id IS NOT DISTINCT FROM $6
          ORDER BY created_at LIMIT 1 FOR UPDATE",
    )
    .bind(etab)
    .bind(foyer)
    .bind(type_piece)
    .bind(origine)
    .bind(annee)
    .bind(facture_origine)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some((id,)) = existante {
        return Ok(id);
    }
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_fac_facture (id, etablissement_id, foyer_id, numero, libelle, statut, total_centimes,
                nature, annee_scolaire, type_piece, origine, facture_origine_id, created_by_user_id, created_at)
         VALUES ($1, $2, $3, $4, $5, 'brouillon', 0, $6, $7, $8, $6, $9, $10, now())",
    )
    .bind(id)
    .bind(etab)
    .bind(foyer)
    .bind(format!("BR-{}", &id.simple().to_string()[..12]))
    .bind(libelle)
    .bind(origine)
    .bind(annee)
    .bind(type_piece)
    .bind(facture_origine)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    publier(
        conn,
        etab,
        types::FAC_FACTURE_CREEE,
        AggregateKind::Facture,
        id,
        None,
        json!({ "foyer_id": foyer, "type_piece": type_piece, "origine": origine, "annee_scolaire": annee }),
        actor,
        causalite,
    )
    .await?;
    Ok(id)
}

/// Insère des lignes dans les factures brouillon de leurs foyers. Idempotent :
/// une clé de source déjà présente n'est jamais réinsérée (index unique).
pub(crate) async fn inserer_lignes(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Option<&str>,
    origine: &str,
    libelle_piece: &str,
    lignes: Vec<LigneAFacturer>,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<ResultatInsertion, EntDbError> {
    let mut res = ResultatInsertion::default();
    let mut par_foyer: BTreeMap<Uuid, Vec<LigneAFacturer>> = BTreeMap::new();
    for l in lignes {
        par_foyer.entry(l.foyer_id).or_default().push(l);
    }
    for (foyer, lignes) in par_foyer {
        verrou_foyer(conn, etab, foyer).await?;
        let cles: Vec<String> = lignes.iter().map(|l| l.source_cle.clone()).collect();
        let deja = sources_existantes(conn, etab, &cles).await?;
        let nouvelles: Vec<&LigneAFacturer> = lignes
            .iter()
            .filter(|l| !deja.contains(&l.source_cle))
            .collect();
        res.deja += (lignes.len() - nouvelles.len()) as i64;
        if nouvelles.is_empty() {
            continue;
        }
        let facture = piece_brouillon(
            conn,
            etab,
            foyer,
            "facture",
            origine,
            annee,
            None,
            libelle_piece,
            actor,
            causalite,
        )
        .await?;
        let mut creees = 0_i64;
        let mut montant = 0_i64;
        for l in nouvelles {
            let r = sqlx::query(
                "INSERT INTO ent_fac_ligne (id, etablissement_id, facture_id, eleve_id, libelle, quantite,
                        montant_unitaire_centimes, source_cle, article_id, categorie, periode_debut, periode_fin)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 ON CONFLICT (etablissement_id, source_cle) WHERE source_cle IS NOT NULL DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(facture)
            .bind(l.eleve_id)
            .bind(&l.libelle)
            .bind(l.quantite)
            .bind(l.unitaire)
            .bind(&l.source_cle)
            .bind(l.article_id)
            .bind(&l.categorie)
            .bind(l.periode.map(|p| p.0))
            .bind(l.periode.map(|p| p.1))
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            if r.rows_affected() == 1 {
                creees += 1;
                montant += l.unitaire * l.quantite as i64;
            } else {
                res.deja += 1;
            }
        }
        recalculer_total(conn, etab, facture).await?;
        if creees > 0 {
            publier(
                conn,
                etab,
                types::FAC_LIGNE_AJOUTEE,
                AggregateKind::Facture,
                facture,
                None,
                json!({ "foyer_id": foyer, "lignes": creees, "montant_centimes": montant, "origine": origine }),
                actor,
                causalite,
            )
            .await?;
        }
        res.creees += creees;
        res.montant += montant;
        if !res.factures.contains(&facture) {
            res.factures.push(facture);
        }
    }
    Ok(res)
}

/// Ligne existante facturée à un élève (base des avoirs).
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct LigneExistante {
    pub id: Uuid,
    pub facture_id: Uuid,
    pub foyer_id: Uuid,
    pub categorie: Option<String>,
    /// Catégorie de l'article (une réduction porte l'article qu'elle réduit).
    pub categorie_article: Option<String>,
    pub libelle: String,
    pub total: i64,
    pub periode_debut: Option<NaiveDate>,
    pub periode_fin: Option<NaiveDate>,
    pub deja_credite: i64,
}

const SELECT_LIGNE_EXISTANTE: &str = "SELECT l.id, l.facture_id, f.foyer_id, l.categorie,
        ar.categorie AS categorie_article, l.libelle,
        (l.quantite::BIGINT * l.montant_unitaire_centimes)::BIGINT AS total,
        l.periode_debut, l.periode_fin,
        coalesce((SELECT -sum(a.quantite::BIGINT * a.montant_unitaire_centimes)
                    FROM ent_fac_ligne a
                   WHERE a.etablissement_id = l.etablissement_id AND a.ligne_origine_id = l.id), 0)::BIGINT
            AS deja_credite
   FROM ent_fac_ligne l
   JOIN ent_fac_facture f ON f.id = l.facture_id AND f.etablissement_id = l.etablissement_id
   LEFT JOIN ent_fin_article ar ON ar.id = l.article_id AND ar.etablissement_id = l.etablissement_id";

pub(crate) async fn lignes_eleve_annee(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    annee: &str,
) -> Result<Vec<LigneExistante>, EntDbError> {
    sqlx::query_as::<_, LigneExistante>(&format!(
        "{SELECT_LIGNE_EXISTANTE}
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND f.annee_scolaire = $3
            AND f.type_piece = 'facture' AND l.ligne_origine_id IS NULL AND l.periode_debut IS NOT NULL
          ORDER BY l.periode_debut, l.id"
    ))
    .bind(etab)
    .bind(eleve)
    .bind(annee)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

fn nb_mois_entre(a: NaiveDate, b: NaiveDate) -> i64 {
    (b.year() - a.year()) as i64 * 12 + b.month() as i64 - a.month() as i64 + 1
}

/// Montant à créditer sur une ligne pour ne garder que les mois jusqu'à celui
/// de `date` inclus (mois commencé dû). Négatif pour une ligne de réduction.
pub(crate) fn credit_au_mois(l: &LigneExistante, date: NaiveDate) -> i64 {
    let (Some(deb), Some(fin)) = (l.periode_debut, l.periode_fin) else {
        return 0;
    };
    let total_mois = nb_mois_entre(deb, fin).max(1);
    let garde = if date < deb {
        0
    } else {
        nb_mois_entre(deb, date).clamp(0, total_mois)
    };
    let du = calcul::proportion(l.total, garde, total_mois);
    let credit = l.total - du - l.deja_credite;
    if credit.signum() == l.total.signum() {
        credit
    } else {
        0
    }
}

/// Montant à créditer pour les jours d'absence sur la période de la ligne.
pub(crate) fn credit_aux_jours(l: &LigneExistante, debut: NaiveDate, fin: NaiveDate) -> i64 {
    let (Some(deb), Some(lfin)) = (l.periode_debut, l.periode_fin) else {
        return 0;
    };
    let couverts = (lfin - deb).num_days() + 1;
    let d = debut.max(deb);
    let f = fin.min(lfin);
    if f < d || couverts <= 0 {
        return 0;
    }
    let jours = (f - d).num_days() + 1;
    let credit = calcul::proportion(l.total, jours, couverts);
    // Jamais au-delà de ce qui reste sur la ligne.
    let reste = l.total - l.deja_credite;
    if l.total >= 0 {
        credit.clamp(0, reste.max(0))
    } else {
        credit.clamp(reste.min(0), 0)
    }
}

/// Ligne d'avoir à créer.
#[derive(Debug, Clone)]
pub(crate) struct AvoirAFaire {
    pub origine: LigneExistante,
    pub eleve_id: Option<Uuid>,
    /// Montant crédité (positif pour une ligne facturée, négatif pour une réduction).
    pub credit: i64,
    pub libelle: String,
    pub source_cle: String,
}

/// Crée les lignes d'avoir dans un avoir brouillon par facture d'origine.
pub(crate) async fn inserer_avoirs(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Option<&str>,
    motif: &str,
    avoirs: Vec<AvoirAFaire>,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<ResultatInsertion, EntDbError> {
    let mut res = ResultatInsertion::default();
    let mut par_facture: BTreeMap<(Uuid, Uuid), Vec<AvoirAFaire>> = BTreeMap::new();
    for a in avoirs.into_iter().filter(|a| a.credit != 0) {
        par_facture
            .entry((a.origine.foyer_id, a.origine.facture_id))
            .or_default()
            .push(a);
    }
    for ((foyer, facture), avoirs) in par_facture {
        verrou_foyer(conn, etab, foyer).await?;
        let cles: Vec<String> = avoirs.iter().map(|a| a.source_cle.clone()).collect();
        let deja = sources_existantes(conn, etab, &cles).await?;
        let nouvelles: Vec<&AvoirAFaire> = avoirs
            .iter()
            .filter(|a| !deja.contains(&a.source_cle))
            .collect();
        res.deja += (avoirs.len() - nouvelles.len()) as i64;
        if nouvelles.is_empty() {
            continue;
        }
        let avoir = piece_brouillon(
            conn,
            etab,
            foyer,
            "avoir",
            "evenement",
            annee,
            Some(facture),
            &format!("Avoir — {motif}"),
            actor,
            causalite,
        )
        .await?;
        let mut creees = 0;
        for a in nouvelles {
            let r = sqlx::query(
                "INSERT INTO ent_fac_ligne (id, etablissement_id, facture_id, eleve_id, libelle, quantite,
                        montant_unitaire_centimes, source_cle, categorie, ligne_origine_id, periode_debut, periode_fin)
                 VALUES ($1, $2, $3, $4, $5, 1, $6, $7, $8, $9, $10, $11)
                 ON CONFLICT (etablissement_id, source_cle) WHERE source_cle IS NOT NULL DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(avoir)
            .bind(a.eleve_id)
            .bind(&a.libelle)
            .bind(-a.credit)
            .bind(&a.source_cle)
            .bind(&a.origine.categorie)
            .bind(a.origine.id)
            .bind(a.origine.periode_debut)
            .bind(a.origine.periode_fin)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            if r.rows_affected() == 1 {
                creees += 1;
                res.montant -= a.credit;
            } else {
                res.deja += 1;
            }
        }
        recalculer_total(conn, etab, avoir).await?;
        if creees > 0 {
            publier(
                conn,
                etab,
                types::FAC_LIGNE_AJOUTEE,
                AggregateKind::Facture,
                avoir,
                None,
                json!({ "foyer_id": foyer, "lignes": creees, "type_piece": "avoir", "facture_origine_id": facture }),
                actor,
                causalite,
            )
            .await?;
        }
        res.creees += creees;
        if !res.factures.contains(&avoir) {
            res.factures.push(avoir);
        }
    }
    Ok(res)
}

// ---------------------------------------------------------------------------
// Moteur : facturation annuelle d'un élève
// ---------------------------------------------------------------------------

pub(crate) struct EleveFacturable {
    pub nom: String,
    pub prenom: String,
    pub regime: String,
    pub niveau: Option<String>,
}

pub(crate) async fn eleve_facturable(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
) -> Result<Option<EleveFacturable>, EntDbError> {
    let r: Option<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT e.nom, e.prenom, e.regime, c.niveau FROM ent_eleve e
           LEFT JOIN ent_classe c ON c.id = e.classe_id AND c.etablissement_id = e.etablissement_id
          WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.map(|r| EleveFacturable {
        nom: r.0,
        prenom: r.1,
        regime: calcul::regime_normalise(r.2.as_deref()),
        niveau: r.3,
    }))
}

/// Foyers payeurs d'un élève et leurs parts (S2-1). Liens suspendus exclus.
pub(crate) async fn payeurs(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
) -> Result<Vec<(Uuid, i64)>, EntDbError> {
    let rows: Vec<(Uuid, Option<i16>)> = sqlx::query_as(
        "SELECT foyer_id, part_paiement FROM ent_eleve_lien
          WHERE etablissement_id = $1 AND eleve_id = $2 AND est_payeur AND foyer_id IS NOT NULL
            AND suspendu_le IS NULL
          ORDER BY part_paiement DESC NULLS LAST, foyer_id",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    // Deux liens payeurs dans le même foyer : parts additionnées.
    let mut out: Vec<(Uuid, i64)> = Vec::new();
    for (f, p) in rows {
        let p = p.unwrap_or(0) as i64;
        if let Some(x) = out.iter_mut().find(|x| x.0 == f) {
            x.1 += p;
        } else {
            out.push((f, p));
        }
    }
    // Parts absentes : répartition égale.
    if !out.is_empty() && out.iter().all(|x| x.1 == 0) {
        for x in out.iter_mut() {
            x.1 = 1;
        }
    }
    Ok(out)
}

/// Rang de l'élève dans la fratrie payée par un foyer (aîné = 1), parmi les
/// élèves actifs ou préinscrits dont ce foyer est payeur.
async fn rang_fratrie(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
    eleve: Uuid,
) -> Result<i64, EntDbError> {
    let rows: Vec<(Uuid, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT DISTINCT e.id, e.date_naissance FROM ent_eleve e
           JOIN ent_eleve_lien l ON l.eleve_id = e.id AND l.etablissement_id = e.etablissement_id
          WHERE e.etablissement_id = $1 AND l.foyer_id = $2 AND l.est_payeur AND l.suspendu_le IS NULL
            AND (e.status IN ('actif', 'preinscrit') OR e.id = $3)
          ORDER BY e.date_naissance NULLS LAST, e.id",
    )
    .bind(etab)
    .bind(foyer)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .iter()
        .position(|r| r.0 == eleve)
        .map(|p| p as i64 + 1)
        .unwrap_or(1))
}

#[derive(sqlx::FromRow)]
struct TarifRow {
    id: Uuid,
    article_id: Uuid,
    niveau: Option<String>,
    regime: Option<String>,
    qf_min_centimes: Option<i64>,
    qf_max_centimes: Option<i64>,
    montant_annuel_centimes: i64,
}

/// Calcule les lignes annuelles d'un élève (sans écrire).
///
/// * `depuis` : date d'entrée (prorata au mois, mois commencé dû) ;
/// * `categories` : restreint aux catégories citées (changement de régime) ;
/// * `prefixe` : préfixe des clés de source (`annuel:<année>` ou `regime:<événement>`).
///
/// `Ok(Err(motif))` : élève non facturable (anomalie à afficher, rien d'écrit).
pub(crate) async fn lignes_annuelles(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve_id: Uuid,
    depuis: Option<NaiveDate>,
    regime_force: Option<&str>,
    categories: Option<&[&str]>,
    prefixe: &str,
) -> Result<Result<Vec<LigneAFacturer>, String>, EntDbError> {
    let p = params_tx(conn, etab).await?;
    let Some(cal) = p.calendrier(annee) else {
        return Ok(Err("année scolaire invalide".into()));
    };
    let Some(eleve) = eleve_facturable(conn, etab, eleve_id).await? else {
        return Ok(Err("élève introuvable".into()));
    };
    let regime = regime_force
        .map(|r| calcul::regime_normalise(Some(r)))
        .unwrap_or(eleve.regime.clone());
    let parts = payeurs(conn, etab, eleve_id).await?;
    if parts.is_empty() {
        return Ok(Err(
            "aucun foyer payeur défini (dossier élève, payeurs)".into()
        ));
    }
    let mois_dus = match depuis {
        None => cal.nb_mois,
        Some(d) => cal.mois_dus_depuis(d),
    };
    if mois_dus == 0 {
        return Ok(Ok(Vec::new()));
    }
    let premier_mois = cal.mois(cal.nb_mois - mois_dus);
    let periode = (
        NaiveDate::from_ymd_opt(premier_mois.0, premier_mois.1, 1).expect("date valide"),
        cal.dernier_jour(),
    );
    let nom = format!("{} {}", eleve.prenom, eleve.nom);

    let candidats: Vec<TarifCandidat> = sqlx::query_as::<_, TarifRow>(
        "SELECT t.id, t.article_id, t.niveau, t.regime, t.qf_min_centimes, t.qf_max_centimes,
                t.montant_annuel_centimes
           FROM ent_fin_tarif t
           JOIN ent_fin_article a ON a.id = t.article_id AND a.etablissement_id = t.etablissement_id
          WHERE t.etablissement_id = $1 AND t.annee_scolaire = $2 AND t.actif AND a.actif",
    )
    .bind(etab)
    .bind(annee)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .into_iter()
    .map(|t| TarifCandidat {
        id: t.id,
        article_id: t.article_id,
        niveau: t.niveau,
        regime: t.regime,
        qf_min: t.qf_min_centimes,
        qf_max: t.qf_max_centimes,
        montant_annuel: t.montant_annuel_centimes,
    })
    .collect();
    let articles: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, libelle, categorie FROM ent_fin_article
          WHERE etablissement_id = $1 AND actif ORDER BY code",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let qf: HashMap<Uuid, i64> = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT foyer_id, quotient_centimes FROM ent_foyer_quotient
          WHERE etablissement_id = $1 AND annee_scolaire = $2 AND foyer_id = ANY($3)",
    )
    .bind(etab)
    .bind(annee)
    .bind(parts.iter().map(|p| p.0).collect::<Vec<_>>())
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .into_iter()
    .collect();
    let reductions: Vec<(Uuid, String, String, Option<i16>, i32, Vec<String>, bool)> =
        sqlx::query_as(
            "SELECT r.id, r.libelle, r.nature, r.rang_min, r.taux_bps, r.categories,
                    EXISTS (SELECT 1 FROM ent_fin_reduction_eleve x
                             WHERE x.etablissement_id = r.etablissement_id AND x.reduction_id = r.id
                               AND x.eleve_id = $3) AS beneficiaire
               FROM ent_fin_reduction r
              WHERE r.etablissement_id = $1 AND r.annee_scolaire = $2 AND r.actif",
        )
        .bind(etab)
        .bind(annee)
        .bind(eleve_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    let mut rangs: HashMap<Uuid, i64> = HashMap::new();
    for (foyer, _) in &parts {
        rangs.insert(*foyer, rang_fratrie(conn, etab, *foyer, eleve_id).await?);
    }

    let mut lignes = Vec::new();
    let mut aucun_tarif = true;
    for (art_id, art_libelle, art_categorie) in &articles {
        if !calcul::categorie_due(art_categorie, &regime) {
            continue;
        }
        if categories.is_some_and(|c| !c.contains(&art_categorie.as_str())) {
            continue;
        }
        // Montant par foyer (le quotient peut différer d'un foyer à l'autre) :
        // chaque foyer paie sa part, au centime près (plus fort reste).
        for (foyer, _) in &parts {
            let Some(t) = calcul::choisir_tarif(
                &candidats,
                *art_id,
                eleve.niveau.as_deref(),
                &regime,
                qf.get(foyer).copied(),
            ) else {
                continue;
            };
            aucun_tarif = false;
            let du = calcul::prorata(t.montant_annuel, mois_dus, cal.nb_mois);
            let part = calcul::repartir(du, &parts)
                .into_iter()
                .find(|x| x.0 == *foyer)
                .map(|x| x.1)
                .unwrap_or(0);
            if part == 0 {
                continue;
            }
            let base_cle = format!("{prefixe}:{eleve_id}:{art_id}");
            lignes.push(LigneAFacturer {
                foyer_id: *foyer,
                eleve_id: Some(eleve_id),
                article_id: Some(*art_id),
                categorie: art_categorie.clone(),
                libelle: format!("{art_libelle} — {nom} ({annee})"),
                quantite: 1,
                unitaire: part,
                source_cle: format!("{base_cle}:{foyer}"),
                periode: Some(periode),
            });
            // Réductions (FIN-07) : la meilleure fratrie atteinte + celles accordées à l'élève.
            let rang = rangs.get(foyer).copied().unwrap_or(1);
            let fratrie = reductions
                .iter()
                .filter(|r| r.2 == "fratrie" && r.3.is_some_and(|m| rang >= m as i64))
                .filter(|r| r.5.contains(art_categorie))
                .max_by_key(|r| r.4);
            let individuelles = reductions
                .iter()
                .filter(|r| r.2 != "fratrie" && r.6 && r.5.contains(art_categorie));
            for r in fratrie.into_iter().chain(individuelles) {
                let remise = calcul::taux(part, r.4);
                if remise == 0 {
                    continue;
                }
                lignes.push(LigneAFacturer {
                    foyer_id: *foyer,
                    eleve_id: Some(eleve_id),
                    article_id: Some(*art_id),
                    categorie: "reduction".into(),
                    libelle: format!("{} — {art_libelle} — {nom}", r.1),
                    quantite: 1,
                    unitaire: -remise,
                    source_cle: format!("{base_cle}:red:{}:{foyer}", r.0),
                    periode: Some(periode),
                });
            }
        }
    }
    // Bourses acceptées avec procuration (versées à l'établissement), au barème.
    if categories.is_none_or(|c| c.contains(&"bourse")) {
        let bourses: Vec<(Uuid, String, i64)> = sqlx::query_as(
            "SELECT b.id, t.libelle, br.montant_annuel_centimes
               FROM ent_eleve_bourse b
               JOIN ent_bourse_type t ON t.id = b.bourse_type_id AND t.etablissement_id = b.etablissement_id
               JOIN ent_fin_bourse_bareme br ON br.etablissement_id = b.etablissement_id
                    AND br.bourse_type_id = b.bourse_type_id AND br.echelon = b.echelon
                    AND br.annee_scolaire = $3
              WHERE b.etablissement_id = $1 AND b.eleve_id = $2 AND b.status = 'acceptee'
                AND b.avec_procuration
                AND b.date_debut <= $5 AND (b.date_fin IS NULL OR b.date_fin >= $4)",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(annee)
        .bind(cal.premier_jour())
        .bind(cal.dernier_jour())
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        // F0 (FD-12, règle provisoire R-F14) : une bourse ne se déduit que des lignes de
        // restauration et d'internat du foyer qui la reçoit, à hauteur de ces lignes ;
        // jamais de la scolarité ni des activités. L'excédent n'est pas imputé (il sera
        // reversé à la famille quand le socle 26 existera).
        let mut plafond: HashMap<Uuid, i64> = HashMap::new();
        for l in lignes
            .iter()
            .filter(|l| l.categorie == "restauration" || l.categorie == "internat")
        {
            *plafond.entry(l.foyer_id).or_insert(0) += l.unitaire * i64::from(l.quantite);
        }
        for (bourse_id, libelle, annuel) in bourses {
            let du = calcul::prorata(annuel, mois_dus, cal.nb_mois);
            for (foyer, part) in calcul::repartir(du, &parts) {
                let reste = plafond.entry(foyer).or_insert(0);
                let part = part.min((*reste).max(0));
                if part == 0 {
                    continue;
                }
                *reste -= part;
                lignes.push(LigneAFacturer {
                    foyer_id: foyer,
                    eleve_id: Some(eleve_id),
                    article_id: None,
                    categorie: "bourse".into(),
                    libelle: format!("Bourse ({libelle}) — {nom}"),
                    quantite: 1,
                    unitaire: -part,
                    source_cle: format!("{prefixe}:{eleve_id}:bourse:{bourse_id}:{foyer}"),
                    periode: Some(periode),
                });
            }
        }
    }
    if aucun_tarif && categories.is_none() {
        return Ok(Err(format!(
            "aucun tarif applicable (niveau {}, régime {regime})",
            eleve.niveau.as_deref().unwrap_or("inconnu")
        )));
    }
    Ok(Ok(lignes))
}

/// Facturation annuelle d'un élève (inscription validée, génération FIN-02).
pub(crate) async fn facturer_annee_eleve(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve_id: Uuid,
    depuis: Option<NaiveDate>,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<Result<ResultatInsertion, String>, EntDbError> {
    let prefixe = format!("annuel:{annee}");
    let lignes =
        match lignes_annuelles(conn, etab, annee, eleve_id, depuis, None, None, &prefixe).await? {
            Ok(l) => l,
            Err(motif) => return Ok(Err(motif)),
        };
    let r = inserer_lignes(
        conn,
        etab,
        Some(annee),
        "annuelle",
        &format!("Scolarité et services {annee}"),
        lignes,
        actor,
        causalite,
    )
    .await?;
    Ok(Ok(r))
}

/// Commande de facturation appelée par l'annulation d'une validation erronée (INS-68, R-I11) :
/// retire les lignes **annuelles brouillon** d'un élève pour une année. Les factures déjà émises
/// ne sont jamais touchées : si une ligne annuelle figure sur une facture émise ou soldée, la
/// commande refuse (`Err(raison)`) et ne retire rien. Les factures brouillon vidées sont
/// supprimées ; les totaux des autres sont recalculés. Renvoie le nombre de lignes retirées.
pub(crate) async fn annuler_lignes_annuelles_brouillon_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve_id: Uuid,
) -> Result<Result<i64, String>, EntDbError> {
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    let prefixe = format!("annuel:{annee}%");
    let (emises,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_fac_ligne l JOIN ent_fac_facture f ON f.id = l.facture_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.source_cle LIKE $3
            AND f.statut <> 'brouillon'",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(&prefixe)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if emises > 0 {
        return Ok(Err(
            "une facture a été émise pour cet élève : l'annulation n'est plus possible, utilisez le désistement"
                .to_string(),
        ));
    }
    let factures: Vec<Uuid> = sqlx::query_scalar(
        "DELETE FROM ent_fac_ligne l USING ent_fac_facture f
          WHERE f.id = l.facture_id AND f.statut = 'brouillon' AND l.etablissement_id = $1
            AND l.eleve_id = $2 AND l.source_cle LIKE $3
         RETURNING l.facture_id",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(&prefixe)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let retirees = i64::try_from(factures.len()).unwrap_or(i64::MAX);
    if !factures.is_empty() {
        sqlx::query(
            "UPDATE ent_fac_facture f SET total_centimes = COALESCE(
                 (SELECT sum(l.quantite::bigint * l.montant_unitaire_centimes)
                    FROM ent_fac_ligne l WHERE l.facture_id = f.id), 0)
              WHERE f.etablissement_id = $1 AND f.id = ANY($2) AND f.statut = 'brouillon'",
        )
        .bind(etab)
        .bind(&factures)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        sqlx::query(
            "DELETE FROM ent_fac_facture f
              WHERE f.etablissement_id = $1 AND f.id = ANY($2) AND f.statut = 'brouillon'
                AND NOT EXISTS (SELECT 1 FROM ent_fac_ligne l WHERE l.facture_id = f.id)",
        )
        .bind(etab)
        .bind(&factures)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(Ok(retirees))
}

/// Changement de régime à `date` : avoir sur les catégories qui ne sont plus
/// dues (réductions associées comprises), lignes nouvelles pour celles qui le
/// deviennent (à partir du mois suivant).
pub(crate) async fn facturer_changement_regime(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve_id: Uuid,
    ancien: &str,
    nouveau: &str,
    date: NaiveDate,
    evenement: Uuid,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<ResultatInsertion, EntDbError> {
    let ancien = calcul::regime_normalise(Some(ancien));
    let nouveau = calcul::regime_normalise(Some(nouveau));
    let mut res = ResultatInsertion::default();
    let perdues: Vec<&str> = ["restauration", "internat"]
        .into_iter()
        .filter(|c| calcul::categorie_due(c, &ancien) && !calcul::categorie_due(c, &nouveau))
        .collect();
    let gagnees: Vec<&str> = ["restauration", "internat"]
        .into_iter()
        .filter(|c| !calcul::categorie_due(c, &ancien) && calcul::categorie_due(c, &nouveau))
        .collect();
    if !perdues.is_empty() {
        let lignes = lignes_eleve_annee(conn, etab, eleve_id, annee).await?;
        let avoirs = lignes
            .into_iter()
            .filter(|l| {
                let cat = l.categorie.as_deref().unwrap_or("");
                let cat_art = l.categorie_article.as_deref().unwrap_or("");
                perdues.contains(&cat) || (cat == "reduction" && perdues.contains(&cat_art))
            })
            .map(|l| AvoirAFaire {
                credit: credit_au_mois(&l, date),
                eleve_id: Some(eleve_id),
                libelle: format!(
                    "Changement de régime au {} — {}",
                    date.format("%d/%m/%Y"),
                    l.libelle
                ),
                source_cle: format!("regime:{evenement}:avoir:{}", l.id),
                origine: l,
            })
            .collect();
        res.ajouter(
            inserer_avoirs(
                conn,
                etab,
                Some(annee),
                "changement de régime",
                avoirs,
                actor,
                causalite,
            )
            .await?,
        );
    }
    if !gagnees.is_empty() {
        let suivant = calcul::fin_de_mois(date.year(), date.month()) + Duration::days(1);
        if let Ok(lignes) = lignes_annuelles(
            conn,
            etab,
            annee,
            eleve_id,
            Some(suivant),
            Some(&nouveau),
            Some(&gagnees),
            &format!("regime:{evenement}"),
        )
        .await?
        {
            res.ajouter(
                inserer_lignes(
                    conn,
                    etab,
                    Some(annee),
                    "evenement",
                    &format!("Changement de régime {annee}"),
                    lignes,
                    actor,
                    causalite,
                )
                .await?,
            );
        }
    }
    Ok(res)
}

async fn annees_facturees(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
) -> Result<Vec<String>, EntDbError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT f.annee_scolaire FROM ent_fac_ligne l
           JOIN ent_fac_facture f ON f.id = l.facture_id AND f.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND f.annee_scolaire IS NOT NULL",
    )
    .bind(etab)
    .bind(eleve_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Sortie de l'élève à `date` : avoir de tous les mois postérieurs.
pub(crate) async fn facturer_sortie(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    date: NaiveDate,
    evenement: Uuid,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<ResultatInsertion, EntDbError> {
    let mut res = ResultatInsertion::default();
    for annee in annees_facturees(conn, etab, eleve_id).await? {
        let lignes = lignes_eleve_annee(conn, etab, eleve_id, &annee).await?;
        let avoirs = lignes
            .into_iter()
            .map(|l| AvoirAFaire {
                credit: credit_au_mois(&l, date),
                eleve_id: Some(eleve_id),
                libelle: format!("Sortie au {} — {}", date.format("%d/%m/%Y"), l.libelle),
                source_cle: format!("sortie:{evenement}:{}", l.id),
                origine: l,
            })
            .collect();
        res.ajouter(
            inserer_avoirs(
                conn,
                etab,
                Some(&annee),
                "sortie de l'élève",
                avoirs,
                actor,
                causalite,
            )
            .await?,
        );
    }
    Ok(res)
}

/// Absence prolongée justifiée (remise d'ordre) : avoir sur la restauration et
/// l'internat au prorata des jours, si la durée atteint le seuil paramétré.
pub(crate) async fn facturer_absence_prolongee(
    conn: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<ResultatInsertion, EntDbError> {
    let abs: Option<(Uuid, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT eleve_id, debut, fin FROM ent_absence WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(absence_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((eleve_id, debut, fin)) = abs else {
        return Ok(ResultatInsertion::default());
    };
    let p = params_tx(conn, etab).await?;
    if (fin - debut).num_days() + 1 < p.seuil_absence_jours as i64 {
        return Ok(ResultatInsertion::default());
    }
    let mut res = ResultatInsertion::default();
    for annee in annees_facturees(conn, etab, eleve_id).await? {
        let lignes = lignes_eleve_annee(conn, etab, eleve_id, &annee).await?;
        let avoirs = lignes
            .into_iter()
            .filter(|l| {
                let c = if l.categorie.as_deref() == Some("reduction") {
                    l.categorie_article.as_deref()
                } else {
                    l.categorie.as_deref()
                };
                matches!(c, Some("restauration") | Some("internat"))
            })
            .map(|l| AvoirAFaire {
                credit: credit_aux_jours(&l, debut, fin),
                eleve_id: Some(eleve_id),
                libelle: format!(
                    "Absence du {} au {} (remise d'ordre) — {}",
                    debut.format("%d/%m/%Y"),
                    fin.format("%d/%m/%Y"),
                    l.libelle
                ),
                source_cle: format!("absence:{absence_id}:{}", l.id),
                origine: l,
            })
            .collect();
        res.ajouter(
            inserer_avoirs(
                conn,
                etab,
                Some(&annee),
                "absence prolongée",
                avoirs,
                actor,
                causalite,
            )
            .await?,
        );
    }
    Ok(res)
}

/// Année scolaire contenant `date` selon les paramètres de l'établissement.
pub(crate) fn annee_de(p: &Params, date: NaiveDate) -> String {
    let debut = if date.month() as i16 >= p.mois_debut {
        date.year()
    } else {
        date.year() - 1
    };
    format!("{}-{}", debut, debut + 1)
}

// ---------------------------------------------------------------------------
// Lectures communes
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
pub(crate) struct FactureRow {
    pub id: Uuid,
    pub numero: String,
    pub type_piece: String,
    pub origine: Option<String>,
    pub statut: String,
    pub foyer_id: Uuid,
    pub foyer: Option<String>,
    pub libelle: String,
    pub annee_scolaire: Option<String>,
    pub total_centimes: i64,
    pub encaisse: i64,
    pub avoirs: i64,
    pub facture_origine_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub emise_at: Option<DateTime<Utc>>,
}

pub(crate) const SELECT_FACTURE: &str = "SELECT f.id, f.numero, f.type_piece, f.origine, f.statut, f.foyer_id,
        fo.libelle AS foyer, f.libelle, f.annee_scolaire, f.total_centimes,
        coalesce((SELECT sum(e.montant_centimes) FROM ent_fac_encaissement e
                   WHERE e.etablissement_id = f.etablissement_id AND e.facture_id = f.id), 0)::BIGINT AS encaisse,
        coalesce((SELECT sum(x.montant_centimes) FROM ent_fin_lettrage x
                   WHERE x.etablissement_id = f.etablissement_id AND x.facture_id = f.id AND x.piece = 'avoir'), 0)::BIGINT AS avoirs,
        f.facture_origine_id, f.created_at, f.emise_at
   FROM ent_fac_facture f
   LEFT JOIN ent_foyer fo ON fo.id = f.foyer_id AND fo.etablissement_id = f.etablissement_id";

impl FactureRow {
    pub fn regle(&self) -> i64 {
        self.encaisse + self.avoirs
    }

    pub fn reste(&self) -> i64 {
        if self.type_piece == "avoir" || self.statut == "brouillon" {
            0
        } else {
            (self.total_centimes - self.regle()).max(0)
        }
    }

    pub fn libelle_foyer(&self) -> String {
        self.foyer
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("Foyer {}", &self.foyer_id.simple().to_string()[..8]))
    }

    pub fn dto(&self) -> FactureResumeDto {
        FactureResumeDto {
            id: self.id,
            numero: self.numero.clone(),
            type_piece: self.type_piece.clone(),
            origine: self.origine.clone(),
            statut: self.statut.clone(),
            foyer_id: self.foyer_id,
            foyer: self.libelle_foyer(),
            libelle: self.libelle.clone(),
            annee_scolaire: self.annee_scolaire.clone(),
            total_centimes: self.total_centimes,
            regle_centimes: self.regle(),
            reste_centimes: self.reste(),
            facture_origine_id: self.facture_origine_id,
            created_at: self.created_at,
            emise_at: self.emise_at,
        }
    }
}

pub(crate) async fn facture_row(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<FactureRow, EntDbError> {
    sqlx::query_as::<_, FactureRow>(&format!(
        "{SELECT_FACTURE} WHERE f.etablissement_id = $1 AND f.id = $2"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct EcheanceRow {
    pub id: Uuid,
    pub rang: i32,
    pub libelle: String,
    pub montant_centimes: i64,
    pub date_echeance: NaiveDate,
    pub prelevement: Option<String>,
}

/// Échéances d'une facture avec leur couverture (encaissements + avoirs imputés,
/// dans l'ordre des rangs).
pub(crate) async fn echeances_couvertes(
    conn: &mut PgConnection,
    etab: Uuid,
    f: &FactureRow,
) -> Result<Vec<(EcheanceRow, i64)>, EntDbError> {
    let rows: Vec<EcheanceRow> = sqlx::query_as(
        "SELECT e.id, e.rang, e.libelle, e.montant_centimes, e.date_echeance,
                (SELECT p.statut FROM ent_fac_sepa_prelevement p
                  WHERE p.etablissement_id = e.etablissement_id AND p.echeance_id = e.id
                    AND p.statut IN ('prevu', 'exporte', 'execute') LIMIT 1) AS prelevement
           FROM ent_fac_echeance e
          WHERE e.etablissement_id = $1 AND e.facture_id = $2 ORDER BY e.rang",
    )
    .bind(etab)
    .bind(f.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut pool = f.regle().max(0);
    Ok(rows
        .into_iter()
        .map(|e| {
            let c = pool.min(e.montant_centimes);
            pool -= c;
            (e, c)
        })
        .collect())
}

pub(crate) fn echeance_dto(e: &EcheanceRow, couvert: i64, today: NaiveDate) -> EcheanceDto {
    let etat = if couvert >= e.montant_centimes {
        "payee"
    } else if e.date_echeance < today {
        "en_retard"
    } else {
        "a_venir"
    };
    EcheanceDto {
        id: e.id,
        rang: e.rang,
        libelle: e.libelle.clone(),
        date_echeance: e.date_echeance,
        montant_centimes: e.montant_centimes,
        couvert_centimes: couvert,
        etat: etat.into(),
        prelevement: e.prelevement.clone(),
    }
}

/// Comptes utilisateurs des payeurs d'un foyer (liens actifs, S2-1).
pub(crate) async fn comptes_payeurs(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
) -> Result<Vec<Uuid>, EntDbError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT l.user_id FROM ent_eleve_lien_actif l
          WHERE l.etablissement_id = $1 AND l.foyer_id = $2 AND l.est_payeur AND l.user_id IS NOT NULL",
    )
    .bind(etab)
    .bind(foyer)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Notification interne (D0) aux payeurs d'un foyer, idempotente par événement.
pub(crate) async fn notifier_payeurs(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
    evenement: Uuid,
    kind: &str,
    titre: &str,
    corps: &str,
) -> Result<i64, EntDbError> {
    let mut n = 0;
    for user in comptes_payeurs(conn, etab, foyer).await? {
        let r = sqlx::query(
            "INSERT INTO ent_notification (id, etablissement_id, destinataire_user_id, kind, titre, corps, source_event_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (source_event_id, destinataire_user_id, kind) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(user)
        .bind(kind)
        .bind(titre)
        .bind(corps)
        .bind(evenement)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        n += r.rows_affected() as i64;
    }
    Ok(n)
}

pub(crate) fn euros(c: i64) -> String {
    let signe = if c < 0 { "-" } else { "" };
    let a = c.unsigned_abs();
    format!("{signe}{},{:02} €", a / 100, a % 100)
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

impl PgEnt {
    pub(crate) async fn fin_exiger(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        action: Action,
    ) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etab, actor, role, action)
            .await?
        {
            return Ok(());
        }
        // `facturation.gerer` inclut la lecture (docs/ROLES_ET_DROITS.md).
        if action == Action::FacturationLire
            && self
                .a_action_etablissement(etab, actor, role, Action::FacturationGerer)
                .await?
        {
            return Ok(());
        }
        Err(EntDbError::Forbidden)
    }

    pub(crate) async fn fin_tx(&self, etab: Uuid) -> Result<TenantTx<'static>, EntDbError> {
        TenantTx::begin(self.pool(), etab).await.map_err(db)
    }

    // -- Paramètres ---------------------------------------------------------

    pub async fn parametres_finances_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<ParametresFinancesDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let p = params_tx(&mut tx, etab).await?;
        let paliers = paliers_tx(&mut tx, etab).await?;
        tx.commit().await.map_err(db)?;
        Ok(params_dto(&p, paliers))
    }

    pub async fn modifier_parametres_finances_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ParametresFinancesBody,
    ) -> Result<ParametresFinancesDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let mut p = params_tx(&mut tx, etab).await?;
        let avant_crediteur = (
            p.crediteur_nom.clone(),
            p.crediteur_iban.clone(),
            p.crediteur_bic.clone(),
            p.crediteur_ics.clone(),
        );
        macro_rules! maj {
            ($champ:ident) => {
                if let Some(v) = b.$champ {
                    p.$champ = v;
                }
            };
        }
        maj!(mois_debut);
        maj!(nb_mois);
        maj!(jour_echeance);
        maj!(jours_prenotification);
        maj!(frais_rejet_centimes);
        maj!(rejets_avant_manuel);
        maj!(seuil_absence_jours);
        if !(1..=12).contains(&p.mois_debut) || !(1..=12).contains(&p.nb_mois) {
            return Err(invalide("mois de début et nombre de mois entre 1 et 12"));
        }
        if !(1..=28).contains(&p.jour_echeance) {
            return Err(invalide("jour d'échéance entre 1 et 28"));
        }
        if !(1..=60).contains(&p.jours_prenotification) {
            return Err(invalide("pré-notification entre 1 et 60 jours"));
        }
        // F0 (FD-13, R-P16) : aucun frais de recouvrement à la charge du débiteur sans
        // titre exécutoire (CPCE L. 111-8) ; frais de rejet : à qualifier par un juriste.
        if p.frais_rejet_centimes != 0 {
            return Err(invalide(
                "frais de rejet : aucun frais de recouvrement ne peut être facturé à la famille (décision juridique requise)",
            ));
        }
        if !(1..=10).contains(&p.rejets_avant_manuel) || !(1..=365).contains(&p.seuil_absence_jours)
        {
            return Err(invalide("seuils hors bornes"));
        }
        if b.crediteur_nom.is_some() {
            p.crediteur_nom = opt_texte(&b.crediteur_nom, 70);
        }
        if let Some(iban) = b
            .crediteur_iban
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            p.crediteur_iban = Some(
                domain_eleve::normalize_iban(iban)
                    .map_err(|_| invalide("IBAN du créancier invalide"))?,
            );
        }
        if b.crediteur_bic.is_some() {
            p.crediteur_bic = opt_texte(&b.crediteur_bic, 11).map(|s| s.to_ascii_uppercase());
        }
        if b.crediteur_ics.is_some() {
            p.crediteur_ics = opt_texte(&b.crediteur_ics, 35).map(|s| s.to_ascii_uppercase());
        }
        // F0 (FD-11) : changer le compte crédité est un geste de niveau direction. Un
        // autre profil financier ne peut pas rediriger les prélèvements des familles.
        let apres_crediteur = (
            p.crediteur_nom.clone(),
            p.crediteur_iban.clone(),
            p.crediteur_bic.clone(),
            p.crediteur_ics.clone(),
        );
        let crediteur_change = avant_crediteur != apres_crediteur;
        if crediteur_change
            && !matches!(
                ent_auth::Role::parse(role),
                Some(ent_auth::Role::Admin | ent_auth::Role::Direction)
            )
        {
            return Err(EntDbError::Forbidden);
        }
        for (src, dst) in [
            (&b.compte_clients, &mut p.compte_clients),
            (&b.compte_banque, &mut p.compte_banque),
            (&b.journal_ventes, &mut p.journal_ventes),
            (&b.journal_banque, &mut p.journal_banque),
        ] {
            if let Some(v) = opt_texte(src, 20) {
                if !v.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return Err(invalide(
                        "compte ou journal : lettres et chiffres seulement",
                    ));
                }
                *dst = v;
            }
        }
        let param_id = p.id.unwrap_or_else(Uuid::new_v4);
        let iban_scelle = crate::champs_chiffres::ChampsSensibles::depuis_env()
            .sceller_opt(T_PARAMETRES, "crediteur_iban", param_id, p.crediteur_iban.as_deref())
            .map_err(EntDbError::from)?;
        sqlx::query(
            "INSERT INTO ent_fin_parametres (etablissement_id, mois_debut, nb_mois, jour_echeance,
                    jours_prenotification, frais_rejet_centimes, rejets_avant_manuel, seuil_absence_jours,
                    crediteur_nom, crediteur_iban, crediteur_bic, crediteur_ics, compte_clients, compte_banque,
                    journal_ventes, journal_banque, updated_by, updated_at, id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17, now(), $18)
             ON CONFLICT (etablissement_id) DO UPDATE SET mois_debut = $2, nb_mois = $3, jour_echeance = $4,
                    jours_prenotification = $5, frais_rejet_centimes = $6, rejets_avant_manuel = $7,
                    seuil_absence_jours = $8, crediteur_nom = $9, crediteur_iban = $10, crediteur_bic = $11,
                    crediteur_ics = $12, compte_clients = $13, compte_banque = $14, journal_ventes = $15,
                    journal_banque = $16, updated_by = $17, updated_at = now()",
        )
        .bind(etab)
        .bind(p.mois_debut)
        .bind(p.nb_mois)
        .bind(p.jour_echeance)
        .bind(p.jours_prenotification)
        .bind(p.frais_rejet_centimes)
        .bind(p.rejets_avant_manuel)
        .bind(p.seuil_absence_jours)
        .bind(&p.crediteur_nom)
        .bind(&iban_scelle)
        .bind(&p.crediteur_bic)
        .bind(&p.crediteur_ics)
        .bind(&p.compte_clients)
        .bind(&p.compte_banque)
        .bind(&p.journal_ventes)
        .bind(&p.journal_banque)
        .bind(actor)
        .bind(param_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if crediteur_change {
            // Événement + alerte à la direction (IBAN toujours masqué).
            let evt = publier(
                &mut tx,
                etab,
                types::FIN_CREDITEUR_MODIFIE,
                AggregateKind::Facture,
                etab,
                None,
                json!({
                    "iban_avant": avant_crediteur.1.as_deref().map(masquer_iban),
                    "iban_apres": p.crediteur_iban.as_deref().map(masquer_iban),
                    "ics_change": avant_crediteur.3 != apres_crediteur.3,
                    "nom_change": avant_crediteur.0 != apres_crediteur.0,
                }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
            let direction: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT DISTINCT user_id FROM ent_membership
                  WHERE etablissement_id = $1 AND role IN ('direction', 'admin')",
            )
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for (user,) in direction {
                sqlx::query(
                    "INSERT INTO ent_notification (id, etablissement_id, destinataire_user_id, kind, titre, corps, source_event_id)
                     VALUES ($1, $2, $3, 'fin.crediteur_modifie', $4, $5, $6)
                     ON CONFLICT (source_event_id, destinataire_user_id, kind) DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(user)
                .bind("Compte créancier modifié")
                .bind("Le compte bancaire qui reçoit les prélèvements des familles vient d'être modifié. Si ce n'est pas voulu, contactez immédiatement l'administrateur.")
                .bind(evt.id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        if let Some(paliers) = &b.paliers {
            if paliers.len() > 5 {
                return Err(invalide("5 paliers de relance au plus"));
            }
            let mut prec = 0;
            for (i, pa) in paliers.iter().enumerate() {
                if pa.rang != i as i16 + 1
                    || pa.jours_retard <= prec
                    || pa.jours_retard > 365
                    || pa.frais_centimes != 0
                {
                    return Err(invalide(
                        "paliers : rangs 1, 2, 3… et jours de retard croissants ; aucun frais de relance facturable",
                    ));
                }
                prec = pa.jours_retard;
                texte(
                    &pa.libelle,
                    80,
                    "libellé de palier obligatoire (80 caractères)",
                )?;
            }
            sqlx::query("DELETE FROM ent_fin_palier_relance WHERE etablissement_id = $1")
                .bind(etab)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            for pa in paliers {
                sqlx::query(
                    "INSERT INTO ent_fin_palier_relance (id, etablissement_id, rang, jours_retard, libelle, frais_centimes)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(pa.rang)
                .bind(pa.jours_retard)
                .bind(pa.libelle.trim())
                .bind(pa.frais_centimes)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        let paliers = paliers_tx(&mut tx, etab).await?;
        tx.commit().await.map_err(db)?;
        Ok(params_dto(&p, paliers))
    }

    // -- Articles, tarifs, réductions, bourses, calendrier --------------------

    pub async fn articles_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<ArticleDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, String, String, String, String, bool)> = sqlx::query_as(
            "SELECT id, code, libelle, categorie, compte_produit, actif FROM ent_fin_article
              WHERE etablissement_id = $1 ORDER BY categorie, code",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| ArticleDto {
                id: r.0,
                code: r.1,
                libelle: r.2,
                categorie: r.3,
                compte_produit: r.4,
                actif: r.5,
            })
            .collect())
    }

    pub async fn enregistrer_article_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Option<Uuid>,
        b: &ArticleBody,
    ) -> Result<ArticleDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let code = texte(&b.code, 30, "code d'article obligatoire (30 caractères)")?;
        if !code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(invalide("code d'article : lettres, chiffres, _ et -"));
        }
        let libelle = texte(
            &b.libelle,
            120,
            "libellé d'article obligatoire (120 caractères)",
        )?;
        if !CATEGORIES.contains(&b.categorie.as_str()) {
            return Err(invalide("catégorie inconnue"));
        }
        let compte = opt_texte(&b.compte_produit, 20).unwrap_or_else(|| "706000".into());
        if !compte.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(invalide("compte de produit : lettres et chiffres"));
        }
        let actif = b.actif.unwrap_or(true);
        let mut tx = self.fin_tx(etab).await?;
        let id = match id {
            Some(id) => {
                let r = sqlx::query(
                    "UPDATE ent_fin_article SET code = $3, libelle = $4, categorie = $5, compte_produit = $6, actif = $7
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(id)
                .bind(&code)
                .bind(&libelle)
                .bind(&b.categorie)
                .bind(&compte)
                .bind(actif)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    if est_doublon(&e) {
                        conflit("un article porte déjà ce code")
                    } else {
                        db(e)
                    }
                })?;
                if r.rows_affected() == 0 {
                    return Err(introuvable());
                }
                id
            }
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_fin_article (id, etablissement_id, code, libelle, categorie, compte_produit, actif, created_by)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                )
                .bind(id)
                .bind(etab)
                .bind(&code)
                .bind(&libelle)
                .bind(&b.categorie)
                .bind(&compte)
                .bind(actif)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    if est_doublon(&e) {
                        conflit("un article porte déjà ce code")
                    } else {
                        db(e)
                    }
                })?;
                id
            }
        };
        tx.commit().await.map_err(db)?;
        Ok(ArticleDto {
            id,
            code,
            libelle,
            categorie: b.categorie.clone(),
            compte_produit: compte,
            actif,
        })
    }

    pub async fn tarifs_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        annee: Option<String>,
    ) -> Result<Vec<TarifDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows = tarifs_tx(&mut tx, etab, annee.as_deref(), None).await?;
        tx.commit().await.map_err(db)?;
        Ok(rows)
    }

    pub async fn creer_tarif_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &TarifBody,
    ) -> Result<TarifDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let annee = annee_valide(&b.annee_scolaire)?;
        let regime = opt_texte(&b.regime, 20);
        if regime.as_deref().is_some_and(|r| !REGIMES.contains(&r)) {
            return Err(invalide("régime inconnu (externe, demi_pension, interne)"));
        }
        if !(0..=100_000_000).contains(&b.montant_annuel_centimes) {
            return Err(invalide("montant annuel entre 0 et 1 000 000 €"));
        }
        if b.qf_min_centimes.is_some_and(|m| m < 0)
            || matches!((b.qf_min_centimes, b.qf_max_centimes), (Some(a), Some(z)) if z <= a)
        {
            return Err(invalide("tranche de quotient : minimum < maximum"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let article: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_fin_article WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(b.article_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        article.ok_or_else(introuvable)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_tarif (id, etablissement_id, annee_scolaire, article_id, niveau, regime,
                    qf_min_centimes, qf_max_centimes, montant_annuel_centimes, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(id)
        .bind(etab)
        .bind(&annee)
        .bind(b.article_id)
        .bind(opt_texte(&b.niveau, 20))
        .bind(&regime)
        .bind(b.qf_min_centimes)
        .bind(b.qf_max_centimes)
        .bind(b.montant_annuel_centimes)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::FAC_TARIF_DEFINI,
            AggregateKind::Facture,
            id,
            None,
            json!({ "tarif_id": id, "annee_scolaire": annee, "article_id": b.article_id,
                    "montant_annuel_centimes": b.montant_annuel_centimes }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let dto = tarifs_tx(&mut tx, etab, None, Some(id))
            .await?
            .pop()
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    pub async fn supprimer_tarif_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        // Désactivation : les lignes déjà facturées restent liées à leur article.
        let r = sqlx::query(
            "UPDATE ent_fin_tarif SET actif = false WHERE etablissement_id = $1 AND id = $2 AND actif",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    pub async fn reductions_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        annee: Option<String>,
    ) -> Result<Vec<ReductionDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, String, String, String, String, Option<i16>, i32, Vec<String>)> =
            sqlx::query_as(
                "SELECT id, annee_scolaire, code, libelle, nature, rang_min, taux_bps, categories
                   FROM ent_fin_reduction
                  WHERE etablissement_id = $1 AND actif AND ($2::TEXT IS NULL OR annee_scolaire = $2)
                  ORDER BY annee_scolaire DESC, code",
            )
            .bind(etab)
            .bind(annee)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut out = Vec::new();
        for r in rows {
            let eleves: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT eleve_id FROM ent_fin_reduction_eleve WHERE etablissement_id = $1 AND reduction_id = $2",
            )
            .bind(etab)
            .bind(r.0)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            out.push(ReductionDto {
                id: r.0,
                annee_scolaire: r.1,
                code: r.2,
                libelle: r.3,
                nature: r.4,
                rang_min: r.5,
                taux_bps: r.6,
                categories: r.7,
                eleves: eleves.into_iter().map(|e| e.0).collect(),
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    pub async fn creer_reduction_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ReductionBody,
    ) -> Result<ReductionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let annee = annee_valide(&b.annee_scolaire)?;
        let code = texte(&b.code, 30, "code obligatoire (30 caractères)")?;
        let libelle = texte(&b.libelle, 120, "libellé obligatoire (120 caractères)")?;
        if !["fratrie", "personnel", "autre"].contains(&b.nature.as_str()) {
            return Err(invalide("nature : fratrie, personnel ou autre"));
        }
        if b.nature == "fratrie" && !b.rang_min.is_some_and(|r| (2..=20).contains(&r)) {
            return Err(invalide("fratrie : rang minimal entre 2 et 20"));
        }
        let rang_min = if b.nature == "fratrie" {
            b.rang_min
        } else {
            None
        };
        if !(1..=10_000).contains(&b.taux_bps) {
            return Err(invalide("taux entre 0,01 % et 100 %"));
        }
        let categories = if b.categories.is_empty() {
            vec!["scolarite".to_string()]
        } else {
            b.categories.clone()
        };
        if categories.iter().any(|c| {
            !["scolarite", "restauration", "internat", "activite", "frais"].contains(&c.as_str())
        }) {
            return Err(invalide(
                "catégories : scolarite, restauration, internat, activite, frais",
            ));
        }
        let mut tx = self.fin_tx(etab).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_reduction (id, etablissement_id, annee_scolaire, code, libelle, nature, rang_min,
                    taux_bps, categories, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(id)
        .bind(etab)
        .bind(&annee)
        .bind(&code)
        .bind(&libelle)
        .bind(&b.nature)
        .bind(rang_min)
        .bind(b.taux_bps)
        .bind(&categories)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if est_doublon(&e) {
                conflit("une réduction porte déjà ce code pour cette année")
            } else {
                db(e)
            }
        })?;
        tx.commit().await.map_err(db)?;
        Ok(ReductionDto {
            id,
            annee_scolaire: annee,
            code,
            libelle,
            nature: b.nature.clone(),
            rang_min,
            taux_bps: b.taux_bps,
            categories,
            eleves: vec![],
        })
    }

    pub async fn accorder_reduction_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        reduction: Uuid,
        eleve: Uuid,
    ) -> Result<(), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let r: Option<(String,)> = sqlx::query_as(
            "SELECT nature FROM ent_fin_reduction WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(reduction)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (nature,) = r.ok_or_else(introuvable)?;
        if nature == "fratrie" {
            return Err(invalide("la réduction fratrie est automatique"));
        }
        eleve_facturable(&mut tx, etab, eleve)
            .await?
            .ok_or_else(introuvable)?;
        sqlx::query(
            "INSERT INTO ent_fin_reduction_eleve (id, etablissement_id, reduction_id, eleve_id, created_by)
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (etablissement_id, reduction_id, eleve_id) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(reduction)
        .bind(eleve)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::FAC_REMISE_APPLIQUEE,
            AggregateKind::Eleve,
            eleve,
            Some(eleve),
            json!({ "reduction_id": reduction }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    pub async fn baremes_bourses_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        annee: Option<String>,
    ) -> Result<Vec<BaremeBourseDto>, EntDbError> {
        if self
            .fin_exiger(etab, actor, role, Action::BoursesGerer)
            .await
            .is_err()
        {
            self.fin_exiger(etab, actor, role, Action::FacturationLire)
                .await?;
        }
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, String, Uuid, String, i64)> = sqlx::query_as(
            "SELECT id, annee_scolaire, bourse_type_id, echelon, montant_annuel_centimes FROM ent_fin_bourse_bareme
              WHERE etablissement_id = $1 AND ($2::TEXT IS NULL OR annee_scolaire = $2)
              ORDER BY annee_scolaire DESC, echelon",
        )
        .bind(etab)
        .bind(annee)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| BaremeBourseDto {
                id: r.0,
                annee_scolaire: r.1,
                bourse_type_id: r.2,
                echelon: r.3,
                montant_annuel_centimes: r.4,
            })
            .collect())
    }

    /// Barème de bourse : action `bourses.gerer` (D-017).
    pub async fn poser_bareme_bourse_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &BaremeBourseBody,
    ) -> Result<BaremeBourseDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::BoursesGerer)
            .await?;
        let annee = annee_valide(&b.annee_scolaire)?;
        let echelon = texte(&b.echelon, 20, "échelon obligatoire")?;
        if !(1..=100_000_000).contains(&b.montant_annuel_centimes) {
            return Err(invalide("montant annuel de bourse positif"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let t: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_bourse_type WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(b.bourse_type_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        t.ok_or_else(introuvable)?;
        let (id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO ent_fin_bourse_bareme (id, etablissement_id, annee_scolaire, bourse_type_id, echelon,
                    montant_annuel_centimes, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (etablissement_id, annee_scolaire, bourse_type_id, echelon)
             DO UPDATE SET montant_annuel_centimes = $6
             RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(&annee)
        .bind(b.bourse_type_id)
        .bind(&echelon)
        .bind(b.montant_annuel_centimes)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(BaremeBourseDto {
            id,
            annee_scolaire: annee,
            bourse_type_id: b.bourse_type_id,
            echelon,
            montant_annuel_centimes: b.montant_annuel_centimes,
        })
    }

    pub async fn echeancier_type_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        annee: &str,
    ) -> Result<EcheancierTypeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let annee = annee_valide(annee)?;
        let mut tx = self.fin_tx(etab).await?;
        let dates = dates_echeancier(&mut tx, etab, &annee).await?;
        tx.commit().await.map_err(db)?;
        Ok(EcheancierTypeDto {
            annee_scolaire: annee,
            dates,
        })
    }

    pub async fn poser_echeancier_type_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &EcheancierTypeDto,
    ) -> Result<EcheancierTypeDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let annee = annee_valide(&b.annee_scolaire)?;
        if b.dates.len() > 24 {
            return Err(invalide("24 échéances au plus"));
        }
        if b.dates.windows(2).any(|w| w[1] <= w[0]) {
            return Err(invalide("dates d'échéance strictement croissantes"));
        }
        let mut tx = self.fin_tx(etab).await?;
        sqlx::query(
            "DELETE FROM ent_fin_echeancier_type WHERE etablissement_id = $1 AND annee_scolaire = $2",
        )
        .bind(etab)
        .bind(&annee)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (i, d) in b.dates.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_fin_echeancier_type (id, etablissement_id, annee_scolaire, rang, date_echeance)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(&annee)
            .bind(i as i16 + 1)
            .bind(d)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(EcheancierTypeDto {
            annee_scolaire: annee,
            dates: b.dates.clone(),
        })
    }

    // -- Génération annuelle (FIN-02 ★) ----------------------------------------

    /// Génère (ou prévisualise) les lignes annuelles de tous les élèves dont
    /// l'inscription est validée pour l'année. Idempotent : mêmes clés de source
    /// que la réaction à `inscription.validee`.
    pub async fn generer_annee_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &GenerationBody,
    ) -> Result<RapportGenerationDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let annee = annee_valide(&b.annee_scolaire)?;
        let mut tx = self.fin_tx(etab).await?;
        let eleves: Vec<(Uuid, String, String, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT DISTINCT ON (d.eleve_id) d.eleve_id, e.prenom, e.nom, d.valide_le
               FROM ent_inscription_dossier d
               JOIN ent_inscription_campagne c ON c.id = d.campagne_id AND c.etablissement_id = d.etablissement_id
               JOIN ent_eleve e ON e.id = d.eleve_id AND e.etablissement_id = d.etablissement_id
              WHERE d.etablissement_id = $1 AND c.annee_cible = $2 AND d.statut = 'validee'
                AND d.eleve_id IS NOT NULL
              ORDER BY d.eleve_id, d.valide_le",
        )
        .bind(etab)
        .bind(&annee)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut rapport = RapportGenerationDto {
            annee_scolaire: annee.clone(),
            apercu: b.apercu,
            eleves: eleves.len() as i64,
            ..Default::default()
        };
        let mut total = ResultatInsertion::default();
        for (eleve_id, prenom, nom, valide_le) in eleves {
            match facturer_annee_eleve(
                &mut tx,
                etab,
                &annee,
                eleve_id,
                valide_le.map(|d| d.date_naive()),
                Some(actor),
                Causalite::default(),
            )
            .await?
            {
                Ok(r) => total.ajouter(r),
                Err(motif) => rapport.anomalies.push(AnomalieGenerationDto {
                    eleve_id,
                    eleve: format!("{prenom} {nom}"),
                    motif,
                }),
            }
        }
        rapport.lignes_creees = total.creees;
        rapport.lignes_deja_presentes = total.deja;
        rapport.factures_touchees = total.factures.len() as i64;
        rapport.montant_centimes = total.montant;
        if b.apercu {
            // Aperçu : tout est calculé dans la transaction, puis annulé.
            drop(tx);
            return Ok(rapport);
        }
        publier(
            &mut tx,
            etab,
            types::FAC_GENERATION_ANNUELLE,
            AggregateKind::Facture,
            etab,
            None,
            json!({ "annee_scolaire": annee, "lignes": total.creees, "factures": total.factures.len() }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(rapport)
    }

    /// FIN-06 : facturer une consommation (repas, activité, séjour…) — au plus une
    /// ligne par (source, foyer payeur).
    pub async fn facturer_consommation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ConsommationBody,
    ) -> Result<ResultatSourceDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let source = texte(&b.source, 120, "clé de source obligatoire (120 caractères)")?;
        let libelle = texte(&b.libelle, 160, "libellé obligatoire")?;
        if !(1..=1000).contains(&b.quantite)
            // F0 (FD-16, R-F13) : une consommation est une quantité >= 0 à montant >= 0 ;
            // un montant négatif serait un avoir déguisé, sans pièce ni motif.
            || !(0..=100_000_000).contains(&b.montant_unitaire_centimes)
        {
            return Err(invalide(
                "quantité (1 à 1000) ou montant (0 à 1 000 000 €) hors bornes ; un avoir se fait par une pièce d'avoir",
            ));
        }
        if matches!((b.periode_debut, b.periode_fin), (Some(a), Some(z)) if z < a) {
            return Err(invalide("période : fin avant début"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let art: Option<(String,)> = sqlx::query_as(
            "SELECT categorie FROM ent_fin_article WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(b.article_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (categorie,) = art.ok_or_else(introuvable)?;
        eleve_facturable(&mut tx, etab, b.eleve_id)
            .await?
            .ok_or_else(introuvable)?;
        let parts = payeurs(&mut tx, etab, b.eleve_id).await?;
        if parts.is_empty() {
            return Err(conflit("aucun foyer payeur défini pour cet élève"));
        }
        let total = b.montant_unitaire_centimes * b.quantite as i64;
        let p = params_tx(&mut tx, etab).await?;
        let annee = annee_de(&p, b.periode_debut.unwrap_or_else(aujourd_hui));
        let lignes = calcul::repartir(total, &parts)
            .into_iter()
            .filter(|(_, m)| *m != 0)
            .map(|(foyer, m)| LigneAFacturer {
                foyer_id: foyer,
                eleve_id: Some(b.eleve_id),
                article_id: Some(b.article_id),
                categorie: categorie.clone(),
                libelle: libelle.clone(),
                quantite: 1,
                unitaire: m,
                source_cle: format!("conso:{source}:{foyer}"),
                periode: b.periode_debut.zip(b.periode_fin),
            })
            .collect();
        let r = inserer_lignes(
            &mut tx,
            etab,
            Some(&annee),
            "consommation",
            &format!("Consommations {annee}"),
            lignes,
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ResultatSourceDto {
            lignes_creees: r.creees,
            lignes_deja_presentes: r.deja,
            factures: r.factures,
        })
    }

    // -- Factures : listes, détail, émission, avoirs ---------------------------

    pub async fn factures_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        statut: Option<String>,
        annee: Option<String>,
        foyer: Option<Uuid>,
    ) -> Result<Vec<FactureResumeDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<FactureRow> = sqlx::query_as(&format!(
            "{SELECT_FACTURE} WHERE f.etablissement_id = $1 AND ($2::TEXT IS NULL OR f.statut = $2)
               AND ($3::TEXT IS NULL OR f.annee_scolaire = $3) AND ($4::UUID IS NULL OR f.foyer_id = $4)
             ORDER BY f.created_at DESC LIMIT 1000"
        ))
        .bind(etab)
        .bind(statut)
        .bind(annee)
        .bind(foyer)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows.iter().map(FactureRow::dto).collect())
    }

    pub async fn facture_detail_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<FactureDetailDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let d = detail_tx(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    /// FIN-03 ★ : émettre des brouillons (numéro définitif, échéancier, imputation
    /// des avoirs). Les factures sont émises avant les avoirs du même lot.
    pub async fn emettre_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &EmissionBody,
    ) -> Result<RapportEmissionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        if b.factures.is_empty() || b.factures.len() > 500 {
            return Err(invalide("entre 1 et 500 pièces à émettre"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let p = params_tx(&mut tx, etab).await?;
        let mut pieces = Vec::new();
        for id in &b.factures {
            pieces.push(facture_row(&mut tx, etab, *id).await?);
        }
        pieces.sort_by_key(|f| (f.type_piece == "avoir", f.created_at));
        let mut rapport = RapportEmissionDto::default();
        let today = aujourd_hui();
        for f in pieces {
            verrou_foyer(&mut tx, etab, f.foyer_id).await?;
            let f = facture_row(&mut tx, etab, f.id).await?;
            let refus = |m: &str| RefusDto {
                id: f.id,
                motif: m.into(),
            };
            if f.statut != "brouillon" {
                rapport.refus.push(refus("déjà émise"));
                continue;
            }
            if f.type_piece == "facture" && f.total_centimes <= 0 {
                rapport
                    .refus
                    .push(refus("montant nul ou négatif : à corriger avant émission"));
                continue;
            }
            if f.type_piece == "avoir" {
                if f.total_centimes >= 0 {
                    rapport.refus.push(refus("avoir sans montant"));
                    continue;
                }
                let Some(o) = f.facture_origine_id else {
                    rapport.refus.push(refus("avoir sans facture d'origine"));
                    continue;
                };
                if facture_row(&mut tx, etab, o).await?.statut == "brouillon" {
                    rapport
                        .refus
                        .push(refus("émettre d'abord la facture d'origine"));
                    continue;
                }
            }
            emettre_piece_tx(
                &mut tx,
                etab,
                &p,
                &f,
                today,
                Some(actor),
                Causalite::default(),
            )
            .await?;
            rapport
                .emises
                .push(facture_row(&mut tx, etab, f.id).await?.dto());
        }
        tx.commit().await.map_err(db)?;
        Ok(rapport)
    }

    /// FIN-04 ★ : avoir manuel sur une facture émise (total, ou partiel réparti
    /// sur les lignes au prorata de leur restant). Créé en brouillon.
    pub async fn creer_avoir_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        facture_id: Uuid,
        b: &AvoirBody,
    ) -> Result<FactureDetailDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let motif = texte(&b.motif, 160, "motif d'avoir obligatoire (160 caractères)")?;
        let mut tx = self.fin_tx(etab).await?;
        let f = facture_row(&mut tx, etab, facture_id).await?;
        if f.type_piece != "facture" || f.statut == "brouillon" {
            return Err(conflit("un avoir porte sur une facture émise"));
        }
        verrou_foyer(&mut tx, etab, f.foyer_id).await?;
        let lignes: Vec<LigneExistante> = sqlx::query_as(&format!(
            "{SELECT_LIGNE_EXISTANTE} WHERE l.etablissement_id = $1 AND l.facture_id = $2 ORDER BY l.id"
        ))
        .bind(etab)
        .bind(facture_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let restant: i64 = lignes.iter().map(|l| l.total - l.deja_credite).sum();
        if restant <= 0 {
            return Err(conflit("facture déjà entièrement créditée"));
        }
        let montant = b.montant_centimes.unwrap_or(restant);
        if montant <= 0 || montant > restant {
            return Err(invalide(
                "montant d'avoir entre 0,01 € et le restant de la facture",
            ));
        }
        let eleves: HashMap<Uuid, Option<Uuid>> = sqlx::query_as::<_, (Uuid, Option<Uuid>)>(
            "SELECT id, eleve_id FROM ent_fac_ligne WHERE etablissement_id = $1 AND facture_id = $2",
        )
        .bind(etab)
        .bind(facture_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        // Avoir total : chaque ligne pour son restant. Partiel : au prorata des
        // lignes positives (plus fort reste, somme exacte).
        let cle = Uuid::new_v4();
        let credits: Vec<(usize, i64)> = if montant == restant {
            lignes
                .iter()
                .enumerate()
                .map(|(i, l)| (i, l.total - l.deja_credite))
                .collect()
        } else {
            let poids: Vec<(usize, i64)> = lignes
                .iter()
                .enumerate()
                .filter(|(_, l)| l.total - l.deja_credite > 0)
                .map(|(i, l)| (i, l.total - l.deja_credite))
                .collect();
            calcul::repartir(montant, &poids)
        };
        let avoirs: Vec<AvoirAFaire> = credits
            .into_iter()
            .filter(|(_, m)| *m != 0)
            .map(|(i, m)| {
                let l = lignes[i].clone();
                AvoirAFaire {
                    eleve_id: eleves.get(&l.id).copied().flatten(),
                    credit: m,
                    libelle: format!("{motif} — {}", l.libelle),
                    source_cle: format!("avoir:{cle}:{}", l.id),
                    origine: l,
                }
            })
            .collect();
        let r = inserer_avoirs(
            &mut tx,
            etab,
            f.annee_scolaire.as_deref(),
            &motif,
            avoirs,
            Some(actor),
            Causalite::default(),
        )
        .await?;
        let avoir_id = *r.factures.first().ok_or_else(introuvable)?;
        let d = detail_tx(&mut tx, etab, avoir_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }

    // -- Compte foyer, espace famille ----------------------------------------

    pub async fn compte_foyer_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        foyer: Uuid,
    ) -> Result<CompteFoyerDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        foyer_existe(&mut tx, etab, foyer).await?;
        let c = compte_tx(&mut tx, etab, foyer, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(c)
    }

    pub async fn modifier_mode_paiement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        foyer: Uuid,
        b: &ModePaiementBody,
    ) -> Result<CompteFoyerDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        if !["prelevement", "manuel"].contains(&b.mode.as_str()) {
            return Err(invalide("mode : prelevement ou manuel"));
        }
        let mut tx = self.fin_tx(etab).await?;
        foyer_existe(&mut tx, etab, foyer).await?;
        poser_mode(
            &mut tx,
            etab,
            foyer,
            &b.mode,
            opt_texte(&b.motif, 160).as_deref(),
            Some(actor),
        )
        .await?;
        let c = compte_tx(&mut tx, etab, foyer, true).await?;
        tx.commit().await.map_err(db)?;
        Ok(c)
    }

    /// FIN-17 : comptes des foyers dont la personne connectée est **payeuse**
    /// (lien actif, S2-1). Pièces émises seulement ; IBAN masqués.
    pub async fn espace_finances_famille(
        &self,
        etab: Uuid,
        user: Uuid,
    ) -> Result<EspaceFamilleFinancesDto, EntDbError> {
        let mut tx = self.fin_tx(etab).await?;
        let foyers: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT l.foyer_id FROM ent_eleve_lien_actif l
              WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.est_payeur AND l.foyer_id IS NOT NULL",
        )
        .bind(etab)
        .bind(user)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut comptes = Vec::new();
        for (f,) in foyers {
            comptes.push(compte_tx(&mut tx, etab, f, false).await?);
        }
        tx.commit().await.map_err(db)?;
        Ok(EspaceFamilleFinancesDto {
            comptes,
            paiement_en_ligne: Some(crate::paiement_en_ligne::etat_dto(
                &crate::paiement_en_ligne::PaiementDesactive,
            )),
        })
    }

    // -- Scolarité (D3) : régime et sortie, déclencheurs de facturation -------

    /// Changement de régime à une date d'effet : met à jour l'élève et son
    /// historique daté, publie `eleve.regime_changed` (consommé par D10).
    pub async fn changer_regime_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        b: &RegimeBody,
    ) -> Result<EvenementScolariteDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::ElevesGerer)
            .await?;
        if !REGIMES.contains(&b.regime.as_str()) {
            return Err(invalide("régime inconnu (externe, demi_pension, interne)"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let actuel = eleve_facturable(&mut tx, etab, eleve)
            .await?
            .ok_or_else(introuvable)?;
        if actuel.regime == b.regime {
            return Err(conflit("l'élève a déjà ce régime"));
        }
        sqlx::query(
            "UPDATE ent_eleve SET regime = $3, updated_at = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .bind(&b.regime)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_eleve_periode SET date_fin = GREATEST(date_debut, $3::DATE - 1)
              WHERE etablissement_id = $1 AND eleve_id = $2 AND kind = 'regime' AND date_fin IS NULL",
        )
        .bind(etab)
        .bind(eleve)
        .bind(b.date_effet)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_eleve_periode (id, etablissement_id, eleve_id, kind, valeur, date_debut)
             VALUES ($1, $2, $3, 'regime', $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(eleve)
        .bind(&b.regime)
        .bind(b.date_effet)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let evt = publier(
            &mut tx,
            etab,
            types::ELEVE_REGIME_CHANGED,
            AggregateKind::Eleve,
            eleve,
            Some(eleve),
            json!({ "eleve_id": eleve, "ancien": actuel.regime, "nouveau": b.regime, "date_effet": b.date_effet }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(EvenementScolariteDto {
            eleve_id: eleve,
            evenement_id: evt.id,
        })
    }

    /// Sortie de l'élève : date et motif (le motif n'est pas dans l'événement).
    pub async fn enregistrer_sortie_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve: Uuid,
        b: &SortieBody,
    ) -> Result<EvenementScolariteDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::ElevesGerer)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let r: Option<(Option<NaiveDate>,)> = sqlx::query_as(
            "SELECT date_sortie FROM ent_eleve WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (deja,) = r.ok_or_else(introuvable)?;
        if deja.is_some() {
            return Err(conflit("sortie déjà enregistrée"));
        }
        sqlx::query(
            "UPDATE ent_eleve SET date_sortie = $3, motif_sortie = $4, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .bind(b.date_sortie)
        .bind(opt_texte(&b.motif, 200))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let evt = publier(
            &mut tx,
            etab,
            types::ELEVE_SORTIE,
            AggregateKind::Eleve,
            eleve,
            Some(eleve),
            json!({ "eleve_id": eleve, "date": b.date_sortie }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(EvenementScolariteDto {
            eleve_id: eleve,
            evenement_id: evt.id,
        })
    }
}

pub(crate) async fn foyer_existe(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
) -> Result<(), EntDbError> {
    let existe: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM ent_foyer WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(foyer)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
    existe.map(|_| ()).ok_or_else(introuvable)
}

fn params_dto(p: &Params, paliers: Vec<PalierRelanceDto>) -> ParametresFinancesDto {
    ParametresFinancesDto {
        mois_debut: p.mois_debut,
        nb_mois: p.nb_mois,
        jour_echeance: p.jour_echeance,
        jours_prenotification: p.jours_prenotification,
        frais_rejet_centimes: p.frais_rejet_centimes,
        rejets_avant_manuel: p.rejets_avant_manuel,
        seuil_absence_jours: p.seuil_absence_jours,
        crediteur_nom: p.crediteur_nom.clone(),
        crediteur_iban: p.crediteur_iban.as_deref().map(masquer_iban),
        crediteur_bic: p.crediteur_bic.clone(),
        crediteur_ics: p.crediteur_ics.clone(),
        compte_clients: p.compte_clients.clone(),
        compte_banque: p.compte_banque.clone(),
        journal_ventes: p.journal_ventes.clone(),
        journal_banque: p.journal_banque.clone(),
        paliers,
    }
}

#[derive(sqlx::FromRow)]
struct TarifDtoRow {
    id: Uuid,
    annee_scolaire: String,
    article_id: Uuid,
    code: String,
    libelle: String,
    categorie: String,
    niveau: Option<String>,
    regime: Option<String>,
    qf_min_centimes: Option<i64>,
    qf_max_centimes: Option<i64>,
    montant_annuel_centimes: i64,
}

async fn tarifs_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Option<&str>,
    id: Option<Uuid>,
) -> Result<Vec<TarifDto>, EntDbError> {
    let rows: Vec<TarifDtoRow> = sqlx::query_as(
        "SELECT t.id, t.annee_scolaire, t.article_id, a.code, a.libelle, a.categorie, t.niveau, t.regime,
                t.qf_min_centimes, t.qf_max_centimes, t.montant_annuel_centimes
           FROM ent_fin_tarif t
           JOIN ent_fin_article a ON a.id = t.article_id AND a.etablissement_id = t.etablissement_id
          WHERE t.etablissement_id = $1 AND t.actif AND ($2::TEXT IS NULL OR t.annee_scolaire = $2)
            AND ($3::UUID IS NULL OR t.id = $3)
          ORDER BY t.annee_scolaire DESC, a.categorie, a.code, t.niveau NULLS FIRST, t.regime NULLS FIRST,
                   t.qf_min_centimes NULLS FIRST",
    )
    .bind(etab)
    .bind(annee)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| TarifDto {
            id: r.id,
            annee_scolaire: r.annee_scolaire,
            article_id: r.article_id,
            article_code: r.code,
            article_libelle: r.libelle,
            categorie: r.categorie,
            niveau: r.niveau,
            regime: r.regime,
            qf_min_centimes: r.qf_min_centimes,
            qf_max_centimes: r.qf_max_centimes,
            montant_annuel_centimes: r.montant_annuel_centimes,
        })
        .collect())
}

pub(crate) async fn dates_echeancier(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
) -> Result<Vec<NaiveDate>, EntDbError> {
    let rows: Vec<(NaiveDate,)> = sqlx::query_as(
        "SELECT date_echeance FROM ent_fin_echeancier_type
          WHERE etablissement_id = $1 AND annee_scolaire = $2 ORDER BY rang",
    )
    .bind(etab)
    .bind(annee)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Émet une pièce brouillon (contrôles faits par l'appelant) : numéro continu,
/// échéancier (facture) ou imputation sur la facture d'origine (avoir).
///
/// Échéancier d'une facture annuelle : dates à venir du calendrier de l'année
/// (`ent_fin_echeancier_type`), sinon le jour paramétré de chaque mois restant ;
/// autres factures : une échéance à 30 jours. Parts égales, reste sur la première.
pub(crate) async fn emettre_piece_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    p: &Params,
    f: &FactureRow,
    today: NaiveDate,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<String, EntDbError> {
    let (prefixe, cle) = if f.type_piece == "avoir" {
        ("A", format!("avoir-{}", today.year()))
    } else {
        ("F", format!("facture-{}", today.year()))
    };
    let n = prochain_numero(conn, etab, &cle).await?;
    let numero = format!("{prefixe}{}-{n:06}", today.year());
    sqlx::query(
        "UPDATE ent_fac_facture SET statut = 'emise', numero = $3, emise_at = now()
          WHERE etablissement_id = $1 AND id = $2 AND statut = 'brouillon'",
    )
    .bind(etab)
    .bind(f.id)
    .bind(&numero)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    if f.type_piece == "facture" {
        let mut dates = match (&f.annee_scolaire, f.origine.as_deref()) {
            (Some(a), Some("annuelle")) => {
                let posees: Vec<NaiveDate> = dates_echeancier(conn, etab, a)
                    .await?
                    .into_iter()
                    .filter(|d| *d >= today)
                    .collect();
                if !posees.is_empty() {
                    posees
                } else {
                    p.calendrier(a)
                        .map(|c| {
                            c.echeances_par_defaut(today, p.jour_echeance as u32)
                                .into_iter()
                                .filter(|d| *d >= today)
                                .collect()
                        })
                        .unwrap_or_default()
                }
            }
            _ => Vec::new(),
        };
        if dates.is_empty() {
            dates.push(today + Duration::days(30));
        }
        let n = (dates.len() as i64).min(f.total_centimes).max(1) as usize;
        for (i, (d, m)) in dates[..n]
            .iter()
            .zip(calcul::echeances(f.total_centimes, n))
            .enumerate()
        {
            sqlx::query(
                "INSERT INTO ent_fac_echeance (id, etablissement_id, facture_id, rang, libelle, montant_centimes, date_echeance)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(f.id)
            .bind(i as i32 + 1)
            .bind(format!("Échéance {}/{}", i + 1, n))
            .bind(m)
            .bind(d)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
        let evt = publier(
            conn,
            etab,
            types::FAC_FACTURE_EMISE,
            AggregateKind::Facture,
            f.id,
            None,
            json!({ "facture_id": f.id, "foyer_id": f.foyer_id, "numero": numero,
                    "total_centimes": f.total_centimes, "echeances": n }),
            actor,
            causalite,
        )
        .await?;
        notifier_payeurs(
            conn,
            etab,
            f.foyer_id,
            evt.id,
            "fin.facture",
            "Nouvelle facture",
            &format!(
                "La facture {numero} ({}) est disponible dans votre espace.",
                euros(f.total_centimes)
            ),
        )
        .await?;
    } else if let Some(origine) = f.facture_origine_id {
        // Avoir : imputé sur le reste de la facture d'origine ; l'excédent reste au crédit du foyer.
        let o = facture_row(conn, etab, origine).await?;
        let impute = (-f.total_centimes).min(o.reste());
        if impute > 0 {
            sqlx::query(
                "INSERT INTO ent_fin_lettrage (id, etablissement_id, facture_id, piece, piece_id, montant_centimes)
                 VALUES ($1, $2, $3, 'avoir', $4, $5)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(origine)
            .bind(f.id)
            .bind(impute)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            crate::finances_sepa::solder_si_paye(conn, etab, origine).await?;
        }
        if impute == -f.total_centimes {
            sqlx::query(
                "UPDATE ent_fac_facture SET statut = 'soldee', soldee_at = now() WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(f.id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
        let evt = publier(
            conn,
            etab,
            types::FAC_AVOIR_EMIS,
            AggregateKind::Facture,
            origine,
            None,
            json!({ "avoir_id": f.id, "facture_id": origine, "foyer_id": f.foyer_id, "numero": numero,
                    "total_centimes": f.total_centimes, "impute_centimes": impute }),
            actor,
            causalite,
        )
        .await?;
        notifier_payeurs(
            conn,
            etab,
            f.foyer_id,
            evt.id,
            "fin.avoir",
            "Avoir émis",
            &format!(
                "Un avoir {numero} de {} a été émis en votre faveur.",
                euros(-f.total_centimes)
            ),
        )
        .await?;
    }
    Ok(numero)
}

#[derive(sqlx::FromRow)]
struct LigneDetailRow {
    id: Uuid,
    eleve_id: Option<Uuid>,
    eleve: Option<String>,
    libelle: String,
    categorie: Option<String>,
    quantite: i32,
    montant_unitaire_centimes: i64,
    periode_debut: Option<NaiveDate>,
    periode_fin: Option<NaiveDate>,
}

pub(crate) async fn detail_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<FactureDetailDto, EntDbError> {
    let f = facture_row(conn, etab, id).await?;
    let lignes: Vec<LigneDetailRow> = sqlx::query_as(
        "SELECT l.id, l.eleve_id, e.prenom || ' ' || e.nom AS eleve, l.libelle, l.categorie, l.quantite,
                l.montant_unitaire_centimes, l.periode_debut, l.periode_fin
           FROM ent_fac_ligne l
           LEFT JOIN ent_eleve e ON e.id = l.eleve_id AND e.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.facture_id = $2
          ORDER BY e.nom NULLS LAST, l.libelle, l.id",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let today = aujourd_hui();
    let echeances = echeances_couvertes(conn, etab, &f)
        .await?
        .iter()
        .map(|(e, c)| echeance_dto(e, *c, today))
        .collect();
    let lettrages: Vec<(Option<String>, String, Uuid, i64, DateTime<Utc>)> = sqlx::query_as(
        "SELECT code, piece, piece_id, montant_centimes, created_at FROM ent_fin_lettrage
          WHERE etablissement_id = $1 AND facture_id = $2 ORDER BY created_at",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let avoirs: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_fac_facture WHERE etablissement_id = $1 AND facture_origine_id = $2 ORDER BY created_at",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(FactureDetailDto {
        facture: f.dto(),
        lignes: lignes
            .into_iter()
            .map(|l| LigneFactureDto {
                id: l.id,
                eleve_id: l.eleve_id,
                eleve: l.eleve,
                libelle: l.libelle,
                categorie: l.categorie,
                quantite: l.quantite,
                montant_unitaire_centimes: l.montant_unitaire_centimes,
                total_centimes: l.montant_unitaire_centimes * l.quantite as i64,
                periode_debut: l.periode_debut,
                periode_fin: l.periode_fin,
            })
            .collect(),
        echeances,
        lettrages: lettrages
            .into_iter()
            .map(|l| LettrageDto {
                code: l.0.unwrap_or_default(),
                piece: l.1,
                piece_id: l.2,
                montant_centimes: l.3,
                created_at: l.4,
            })
            .collect(),
        avoirs: avoirs.into_iter().map(|a| a.0).collect(),
    })
}

pub(crate) async fn mode_paiement(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
) -> Result<String, EntDbError> {
    let r: Option<(String,)> = sqlx::query_as(
        "SELECT mode_paiement FROM ent_fin_compte_payeur WHERE etablissement_id = $1 AND foyer_id = $2",
    )
    .bind(etab)
    .bind(foyer)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.map(|r| r.0).unwrap_or_else(|| "prelevement".into()))
}

pub(crate) async fn poser_mode(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
    mode: &str,
    motif: Option<&str>,
    actor: Option<Uuid>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_fin_compte_payeur (etablissement_id, foyer_id, mode_paiement, motif, depuis, updated_by)
         VALUES ($1, $2, $3, $4, now(), $5)
         ON CONFLICT (etablissement_id, foyer_id)
         DO UPDATE SET mode_paiement = $3, motif = $4, depuis = now(), updated_by = $5",
    )
    .bind(etab)
    .bind(foyer)
    .bind(mode)
    .bind(motif)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Compte d'un foyer : pièces, échéances, mandats masqués, solde, crédit.
/// `personnel = false` (famille) : brouillons exclus.
pub(crate) async fn compte_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
    personnel: bool,
) -> Result<CompteFoyerDto, EntDbError> {
    let libelle: Option<(Option<String>,)> =
        sqlx::query_as("SELECT libelle FROM ent_foyer WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(foyer)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
    let factures: Vec<FactureRow> = sqlx::query_as(&format!(
        "{SELECT_FACTURE} WHERE f.etablissement_id = $1 AND f.foyer_id = $2 AND ($3 OR f.statut <> 'brouillon')
         ORDER BY f.created_at DESC"
    ))
    .bind(etab)
    .bind(foyer)
    .bind(personnel)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let today = aujourd_hui();
    let mut echeances = Vec::new();
    for f in factures
        .iter()
        .filter(|f| f.type_piece == "facture" && f.statut != "brouillon")
    {
        for (e, c) in echeances_couvertes(conn, etab, f).await? {
            echeances.push(EcheanceFoyerDto {
                facture_id: f.id,
                facture_numero: f.numero.clone(),
                echeance: echeance_dto(&e, c, today),
            });
        }
    }
    echeances.sort_by_key(|e| (e.echeance.date_echeance, e.echeance.rang));
    let solde: i64 = factures.iter().map(FactureRow::reste).sum();
    // Crédit : trop-perçus non imputés + avoirs émis non imputés.
    let (trop_percu,): (i64,) = sqlx::query_as(
        "SELECT coalesce(sum(non_impute_centimes), 0)::BIGINT FROM ent_fin_reglement
          WHERE etablissement_id = $1 AND foyer_id = $2",
    )
    .bind(etab)
    .bind(foyer)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let (avoirs_libres,): (i64,) = sqlx::query_as(
        "SELECT coalesce(sum(-f.total_centimes - coalesce((SELECT sum(x.montant_centimes) FROM ent_fin_lettrage x
                   WHERE x.etablissement_id = f.etablissement_id AND x.piece = 'avoir' AND x.piece_id = f.id), 0)), 0)::BIGINT
           FROM ent_fac_facture f
          WHERE f.etablissement_id = $1 AND f.foyer_id = $2 AND f.type_piece = 'avoir' AND f.statut = 'emise'",
    )
    .bind(etab)
    .bind(foyer)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let mandats: Vec<(Uuid, String, String, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, rum, iban, titulaire, statut, signe_at FROM ent_fac_sepa_mandat
          WHERE etablissement_id = $1 AND foyer_id = $2 ORDER BY signe_at DESC",
    )
    .bind(etab)
    .bind(foyer)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let champs = crate::champs_chiffres::ChampsSensibles::depuis_env();
    let mut ms = Vec::new();
    for m in mandats {
        let iban = champs.ouvrir("ent_fac_sepa_mandat", "iban", m.0, &m.2)?;
        let titulaire = champs.ouvrir("ent_fac_sepa_mandat", "titulaire", m.0, &m.3)?;
        ms.push(MandatResumeDto {
            id: m.0,
            rum: m.1,
            iban_masque: masquer_iban(&iban),
            titulaire,
            statut: m.4,
            signe_at: m.5,
        });
    }
    Ok(CompteFoyerDto {
        foyer_id: foyer,
        foyer: libelle
            .and_then(|l| l.0)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("Foyer {}", &foyer.simple().to_string()[..8])),
        mode_paiement: mode_paiement(conn, etab, foyer).await?,
        solde_centimes: solde,
        credit_centimes: trop_percu + avoirs_libres.max(0),
        factures: factures.iter().map(FactureRow::dto).collect(),
        echeances,
        mandats: ms,
    })
}

#[cfg(test)]
mod tests_fuseau_f0 {
    use super::date_paris;
    use chrono::{NaiveDate, TimeZone, Utc};

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    /// F0 (FD-24, R-F24) : un instant à 23 h 30 UTC l'été (1 h 30 le lendemain à Paris) tombe
    /// le jour suivant ; l'hiver (UTC+1) aussi à 23 h 30 ; à 22 h 30 UTC l'été c'est déjà le lendemain.
    #[test]
    fn f0_date_comptable_en_heure_de_paris() {
        // Été : UTC+2.
        assert_eq!(date_paris(Utc.with_ymd_and_hms(2026, 7, 14, 21, 59, 0).unwrap()), d(2026, 7, 14));
        assert_eq!(date_paris(Utc.with_ymd_and_hms(2026, 7, 14, 22, 0, 0).unwrap()), d(2026, 7, 15));
        // Hiver : UTC+1.
        assert_eq!(date_paris(Utc.with_ymd_and_hms(2026, 12, 1, 22, 59, 0).unwrap()), d(2026, 12, 1));
        assert_eq!(date_paris(Utc.with_ymd_and_hms(2026, 12, 1, 23, 0, 0).unwrap()), d(2026, 12, 2));
        // Changements d'heure 2026 : 29 mars 01:00 UTC (passage à l'été) et 25 octobre 01:00 UTC.
        assert_eq!(date_paris(Utc.with_ymd_and_hms(2026, 3, 28, 22, 30, 0).unwrap()), d(2026, 3, 28));
        assert_eq!(date_paris(Utc.with_ymd_and_hms(2026, 3, 29, 22, 30, 0).unwrap()), d(2026, 3, 30));
        assert_eq!(date_paris(Utc.with_ymd_and_hms(2026, 10, 24, 22, 30, 0).unwrap()), d(2026, 10, 25));
        assert_eq!(date_paris(Utc.with_ymd_and_hms(2026, 10, 25, 22, 30, 0).unwrap()), d(2026, 10, 25));
        assert_eq!(date_paris(Utc.with_ymd_and_hms(2026, 10, 25, 23, 30, 0).unwrap()), d(2026, 10, 26));
    }
}
