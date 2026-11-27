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

use crate::facturation_payeurs::{
    categorie_de_designation, payeurs_pour_facturer, PartPayeur,
};
use crate::facturation_service::{journaliser_tx, Acte, Chemin};
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

pub(crate) fn est_doublon(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .and_then(|d| d.code())
        .is_some_and(|c| c == "23505")
}

pub(crate) fn annee_valide(a: &str) -> Result<String, EntDbError> {
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

/// Texte facultatif : vide → `None` ; trop long → 422 (jamais de troncature silencieuse, D-069).
pub(crate) fn opt_texte(s: &Option<String>, max: usize) -> Result<Option<String>, EntDbError> {
    match s.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        None => Ok(None),
        Some(t) if t.chars().count() > max => Err(invalide("texte trop long")),
        Some(t) => Ok(Some(t.to_string())),
    }
}

/// Publication dans la transaction du fait (outbox S2-0), **sans transformation** : helper partagé avec
/// d'autres domaines (demandes, internat) dont les charges et les noms d'événements restent inchangés.
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

/// Publication d'un événement **financier** (F1) : nom canonique `facturation.*` (alias `fac.*` / `fin.*`
/// lisible) et charge neutre — liste noire et liste blanche **propre au type** (`CLES_PAR_TYPE`), sans montant, nom ni IBAN (R-F25). Réservé
/// aux événements de finances ; les autres domaines passent par [`publier`], inchangé.
pub(crate) async fn publier_finance(
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
    publier(
        conn,
        etab,
        ent_events::canonique_facturation(type_name),
        aggregate,
        aggregate_id,
        eleve_id,
        crate::facturation_service::payload_neutre_pour(type_name, payload),
        actor,
        causalite,
    )
    .await
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

/// Nature de la tâche d'anomalie ouverte quand un changement de régime ne peut pas être facturé (migration 532).
pub(crate) const NATURE_TACHE_REGIME_NON_FACTURE: &str = "regularisation_regime_non_facturee";

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
    publier_finance(
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
            publier_finance(
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
            if actor.is_none() {
                // Automatisme (consommateur d'événement) : la commande humaine journalise son propre geste.
                journaliser_tx(
                    conn,
                    &Acte::systeme(etab, Chemin::Systeme, "lignes.facturer_auto", "facture")
                        .objet(facture)
                        .foyer(foyer)
                        .montant(montant)
                        .detail(&format!("{creees} lignes créées (origine {origine})")),
                )
                .await?;
            }
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
    /// Mois (indice, voir `indice_mois`) de la période déjà crédités par un avoir de **régime** (§ 2.4 de la conception
    /// « régime mois par mois ») : retirés des mois couverts. Rempli par `finances_regime::remplir_decouverts`.
    #[sqlx(skip)]
    pub decouverts: std::collections::BTreeSet<i32>,
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

/// Lignes existantes par identifiant (base des avoirs d'un changement de payeur, F2).
pub(crate) async fn lignes_existantes(
    conn: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
) -> Result<Vec<LigneExistante>, EntDbError> {
    let mut lignes = sqlx::query_as::<_, LigneExistante>(&format!(
        "{SELECT_LIGNE_EXISTANTE} WHERE l.etablissement_id = $1 AND l.id = ANY($2) ORDER BY l.id"
    ))
    .bind(etab)
    .bind(ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    crate::finances_regime::remplir_decouverts(conn, etab, &mut lignes).await?;
    Ok(lignes)
}

pub(crate) async fn lignes_eleve_annee(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    annee: &str,
) -> Result<Vec<LigneExistante>, EntDbError> {
    let mut lignes = sqlx::query_as::<_, LigneExistante>(&format!(
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
    .map_err(db)?;
    crate::finances_regime::remplir_decouverts(conn, etab, &mut lignes).await?;
    Ok(lignes)
}

/// Lignes de l'élève pour l'année **sans période complète** (consommation saisie sans dates, par exemple) : elles ne
/// passent pas par la facturation au mois, mais comptent dans le plafond de la bourse (F-9, R-F14).
pub(crate) async fn lignes_eleve_annee_sans_periode(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    annee: &str,
) -> Result<Vec<LigneExistante>, EntDbError> {
    sqlx::query_as::<_, LigneExistante>(&format!(
        "{SELECT_LIGNE_EXISTANTE}
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND f.annee_scolaire = $3
            AND f.type_piece = 'facture' AND l.ligne_origine_id IS NULL
            AND (l.periode_debut IS NULL OR l.periode_fin IS NULL)
          ORDER BY l.id"
    ))
    .bind(etab)
    .bind(eleve)
    .bind(annee)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

pub(crate) fn nb_mois_entre(a: NaiveDate, b: NaiveDate) -> i64 {
    (b.year() - a.year()) as i64 * 12 + b.month() as i64 - a.month() as i64 + 1
}

/// Montant à créditer sur une ligne pour ne garder que les mois jusqu'à celui
/// de `date` inclus (mois commencé dû). Négatif pour une ligne de réduction.
pub(crate) fn credit_au_mois(l: &LigneExistante, date: NaiveDate) -> i64 {
    let (Some(deb), Some(fin)) = (l.periode_debut, l.periode_fin) else {
        return 0;
    };
    let total_mois = nb_mois_entre(deb, fin).max(1);
    // Mois gardés = mois couverts **nets** (hors avoirs de régime) jusqu'à celui de `date` inclus (§ 2.4) : un mois déjà
    // crédité par un changement de régime n'est jamais gardé, il ne sera pas crédité une seconde fois ; sans avoir de
    // régime, c'est le décompte d'avant.
    let garde = if date < deb {
        0
    } else {
        (indice_mois(deb)..=indice_mois(fin))
            .filter(|m| *m <= indice_mois(date) && !l.decouverts.contains(m))
            .count() as i64
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
    // Les jours tombant dans un mois déjà crédité par un avoir de régime ne sont pas crédités une seconde fois (§ 2.4).
    let jours: i64 = (indice_mois(d)..=indice_mois(f))
        .filter(|m| !l.decouverts.contains(m))
        .map(|m| {
            let premier = premier_du_mois(m);
            let dernier = calcul::fin_de_mois(premier.year(), premier.month());
            let (a, b) = (d.max(premier), f.min(dernier));
            (b - a).num_days() + 1
        })
        .sum();
    if jours <= 0 {
        return 0;
    }
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
    /// Période propre de l'avoir (avoir de régime mois par mois) ; `None` : celle de la ligne d'origine.
    pub periode: Option<(NaiveDate, NaiveDate)>,
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
            .bind(a.periode.map(|p| p.0).or(a.origine.periode_debut))
            .bind(a.periode.map(|p| p.1).or(a.origine.periode_fin))
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
            publier_finance(
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
            if actor.is_none() {
                journaliser_tx(
                    conn,
                    &Acte::systeme(etab, Chemin::Systeme, "avoirs.facturer_auto", "avoir")
                        .objet(avoir)
                        .foyer(foyer)
                        .detail(&format!("{creees} lignes d'avoir créées sur la pièce {facture}")),
                )
                .await?;
            }
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

/// Foyers payeurs d'un élève et leurs parts (S2-1), lus sur les **liens** actuels. Liens suspendus exclus.
/// F2 : ce n'est plus que l'adaptateur du port `payeurs` (MIG-5) ; la facturation lit par
/// `facturation_payeurs::payeurs_a`.
pub(crate) async fn payeurs_liens(
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
    // Un lien payeur sans part, à côté de payeurs qui en ont : il ne paie rien (pas de ligne à zéro).
    out.retain(|x| x.1 > 0);
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

/// Prise en charge active d'un organisme qui couvre une date et une catégorie (E2, CF-83).
#[derive(Clone, Debug)]
pub(crate) struct PriseCouvrante {
    pub id: Uuid,
    pub plafond: Option<i64>,
    pub categories: Vec<String>,
    pub du: NaiveDate,
    pub au: NaiveDate,
}

/// Verrou du foyer (M2, M3) : pris **avant** toute lecture du restant ou des lignes existantes, pour que deux
/// générations concurrentes ne dépassent pas un plafond ni ne doublent une ligne. Levé à la fin de la transaction.
pub(crate) async fn verrouiller_foyer(conn: &mut PgConnection, etab: Uuid, foyer: Uuid) -> Result<(), EntDbError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(format!("fin-foyer:{etab}:{foyer}"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    Ok(())
}

/// Prises en charge d'un compte d'organisme pour cet élève, couvrant `jour` et la catégorie (ou `toutes`) : **active
/// ou échue** (M1, suites 3 : une prise échue reste valide pour les mois de sa période, c'est la date qui borne, le
/// statut `echue` n'est posé qu'après son terme) ; seule une prise **annulée** est exclue. Vide : l'organisme n'est
/// pas facturé (E2).
pub(crate) async fn prises_couvrantes(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    compte: Uuid,
    categorie: &str,
    jour: NaiveDate,
) -> Result<Vec<PriseCouvrante>, EntDbError> {
    let rows: Vec<(Uuid, Option<i64>, Vec<String>, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT id, plafond_centimes, categories, du, au FROM ent_fin_prise_en_charge
          WHERE etablissement_id = $1 AND eleve_id = $2 AND compte_id = $3 AND statut IN ('active', 'echue')
            AND du <= $4 AND au >= $4 AND (categories && ARRAY[$5, 'toutes']::text[])
          ORDER BY du, id",
    )
    .bind(etab)
    .bind(eleve)
    .bind(compte)
    .bind(jour)
    .bind(categorie)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| PriseCouvrante { id: r.0, plafond: r.1, categories: r.2, du: r.3, au: r.4 })
        .collect())
}

/// Restant d'**une** prise en charge (M2) : son plafond moins ce qui est déjà facturé au compte pour **ses**
/// catégories et **sa** période, **net des avoirs** posés sur ces lignes (F-5 : une ligne entièrement créditée ne
/// consomme plus le plafond) ; jamais additionné aux autres prises. Arithmétique contrôlée.
///
/// `propres` = (préfixe `{génération}:{élève}:`, articles de la génération) : les lignes que **cette** génération
/// reforme (même préfixe, mêmes articles) ne comptent pas, la génération les recalcule à neuf et les consomme dans
/// l'ordre (suites 4, M1) ; sinon un rejeu verrait le principal déjà posé comme un plafond épuisé et le payeur suivant
/// désigné après coup ne recevrait jamais la part non couverte (ou, prolongation sous plafond serré, un faux refus).
async fn restant_prise(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    foyer: Uuid,
    p: &PriseCouvrante,
    propres: (&str, &[Uuid], (NaiveDate, NaiveDate)),
) -> Result<i64, EntDbError> {
    let Some(plafond) = p.plafond else { return Ok(i64::MAX) };
    // Palier des faibles de facturation (contre-revue, M-2) : une ligne **propre** est recalculée brute par la génération,
    // mais les avoirs **hors régime** posés dessus (avoir manuel, sortie, remise d'ordre) ont réellement rendu du plafond :
    // ils sont rendus au restant (les avoirs de régime, eux, découvrent des mois que la garde refacture).
    let (facture,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(CASE
                    WHEN starts_with(l.source_cle, $8) AND l.article_id = ANY($9)
                         AND l.periode_debut >= $10 AND l.periode_debut <= $11
                    THEN COALESCE((SELECT SUM(a.quantite::BIGINT * a.montant_unitaire_centimes)
                                     FROM ent_fac_ligne a
                                    WHERE a.etablissement_id = l.etablissement_id AND a.ligne_origine_id = l.id
                                      AND COALESCE(a.source_cle, '') NOT LIKE 'regime:%:avoir:%'), 0)
                    ELSE l.quantite::BIGINT * l.montant_unitaire_centimes
                         + COALESCE((SELECT SUM(a.quantite::BIGINT * a.montant_unitaire_centimes)
                                       FROM ent_fac_ligne a
                                      WHERE a.etablissement_id = l.etablissement_id
                                        AND a.ligne_origine_id = l.id), 0)
                END), 0)::BIGINT
           FROM ent_fac_ligne l
           JOIN ent_fac_facture f ON f.etablissement_id = l.etablissement_id AND f.id = l.facture_id
           LEFT JOIN ent_fin_article ar ON ar.etablissement_id = l.etablissement_id AND ar.id = l.article_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND f.foyer_id = $3 AND f.type_piece = 'facture'
            AND l.ligne_origine_id IS NULL AND l.periode_debut >= $4 AND l.periode_debut <= $5
            AND ($6 OR COALESCE(ar.categorie, l.categorie) = ANY($7))",
    )
    .bind(etab)
    .bind(eleve)
    .bind(foyer)
    .bind(p.du)
    .bind(p.au)
    .bind(p.categories.iter().any(|c| c == "toutes"))
    .bind(&p.categories)
    .bind(propres.0)
    .bind(propres.1)
    .bind(propres.2 .0)
    .bind(propres.2 .1)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    // `facture` peut être négatif (avoirs hors régime de lignes propres) : le restant dépasse alors le plafond, car la
    // génération reconsomme les lignes propres **brutes** ; le net de l'organisme reste sous son plafond.
    Ok(plafond.checked_sub(facture).unwrap_or(0).max(0))
}

/// Condition SQL « ce tarif s'applique au jour `{jour}` » (M1, règle 2) : une grille ne tarifie qu'à partir de son
/// `en_vigueur_du` ; la version précédente reste applicable jusqu'à la veille ; avant toute version, seuls les
/// tarifs historiques de l'année comptent (jamais un brouillon, une grille proposée, votée ou supplantée).
/// L'alias de la table des tarifs est `t`.
pub(crate) fn sql_tarif_applicable(jour: &str) -> String {
    format!(
        "(CASE WHEN t.grille_version_id IS NULL THEN NOT EXISTS (
                  SELECT 1 FROM ent_fin_grille_version v WHERE v.etablissement_id = t.etablissement_id
                     AND v.annee_scolaire = t.annee_scolaire AND v.statut IN ('en_vigueur', 'close')
                     AND v.en_vigueur_du <= {jour})
              ELSE t.grille_version_id = (
                  SELECT v.id FROM ent_fin_grille_version v WHERE v.etablissement_id = t.etablissement_id
                     AND v.annee_scolaire = t.annee_scolaire AND v.statut IN ('en_vigueur', 'close')
                     AND v.en_vigueur_du <= {jour}
                   ORDER BY v.en_vigueur_du DESC, v.numero DESC LIMIT 1)
         END)"
    )
}

/// Calcule les lignes annuelles d'un élève (sans écrire).
///
/// * `depuis` : date d'entrée (prorata au mois, mois commencé dû) ;
/// * `categories` : restreint aux catégories citées (changement de régime : catégories gagnées) ;
/// * `prefixe` : préfixe des clés de source (`annuel:<année>`) ;
///
/// Le **régime se lit mois par mois** dans l'historique daté de l'élève (`finances_regime`, conception « régime mois par
/// mois ») : un mois où la catégorie n'est pas due n'est pas un trou de grille ; un mois de régime inconnu ne facture ni
/// restauration ni internat (tâche, sans événement) ; les payeurs d'une catégorie sont lus à son premier mois dû.
///
/// Le tarif se choisit **mois par mois** (D-069, M-A) : pour chaque mois dû, la grille applicable est celle du
/// **premier jour de ce mois** (jamais le jour de génération). Les mois consécutifs tarifés par les mêmes lignes
/// forment un segment, facturé au prorata de ses mois ; le premier segment garde la clé de source historique
/// (`<préfixe>:<élève>:<article>`), les suivants portent le suffixe `:m<AAAA-MM>` de leur premier mois.
///
/// `Ok(Err(motif))` : élève non facturable (anomalie à afficher, rien d'écrit).
pub(crate) async fn lignes_annuelles(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve_id: Uuid,
    depuis: Option<NaiveDate>,
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
    let mois_dus = match depuis {
        None => cal.nb_mois,
        Some(d) => cal.mois_dus_depuis(d),
    };
    if mois_dus == 0 {
        return Ok(Ok(Vec::new()));
    }
    let premier_mois = cal.mois(cal.nb_mois - mois_dus);
    // Période complète de la génération (premier mois dû → dernier jour de l'année) : la fenêtre des lignes propres du
    // restant d'une prise (défaut 2, § 2.8 a) ; `periode` reste celle du tronçon en cours dans la boucle.
    let periode_gen = (
        NaiveDate::from_ymd_opt(premier_mois.0, premier_mois.1, 1).expect("date valide"),
        cal.dernier_jour(),
    );
    let nom = format!("{} {}", eleve.prenom, eleve.nom);
    let sql_candidats = format!(
        "SELECT t.id, t.article_id, t.niveau, t.regime, t.qf_min_centimes, t.qf_max_centimes,
                t.montant_annuel_centimes
           FROM ent_fin_tarif t
           JOIN ent_fin_article a ON a.id = t.article_id AND a.etablissement_id = t.etablissement_id
          WHERE t.etablissement_id = $1 AND t.annee_scolaire = $2 AND t.actif AND a.actif
            AND {}
          ORDER BY t.id",
        sql_tarif_applicable("$3")
    );
    // Cache par **jour de vigueur distinct** (D-069, faible d) : la grille applicable un jour donné ne dépend que de
    // la plus récente date de vigueur (version en vigueur ou close) antérieure ou égale à ce jour ; au plus une
    // requête par version, et non une par mois.
    let dates_vigueur: Vec<NaiveDate> = sqlx::query_scalar(
        "SELECT en_vigueur_du FROM ent_fin_grille_version
          WHERE etablissement_id = $1 AND annee_scolaire = $2 AND statut IN ('en_vigueur', 'close')
            AND en_vigueur_du IS NOT NULL ORDER BY en_vigueur_du",
    )
    .bind(etab)
    .bind(annee)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut cache: HashMap<Option<NaiveDate>, Vec<TarifCandidat>> = HashMap::new();
    // Un élément par mois dû : (premier jour, dernier jour, clé du cache).
    let mut mois_tarifes: Vec<(NaiveDate, NaiveDate, Option<NaiveDate>)> = Vec::new();
    for i in (cal.nb_mois - mois_dus)..cal.nb_mois {
        let (an, mo) = cal.mois(i);
        let premier = NaiveDate::from_ymd_opt(an, mo, 1).expect("date valide");
        let cle = dates_vigueur.iter().copied().filter(|d| *d <= premier).max();
        if let std::collections::hash_map::Entry::Vacant(libre) = cache.entry(cle) {
            let rows = sqlx::query_as::<_, TarifRow>(&sql_candidats)
                .bind(etab)
                .bind(annee)
                .bind(cle.unwrap_or(premier))
                .fetch_all(&mut *conn)
                .await
                .map_err(db)?;
            libre.insert(
                rows.into_iter()
                    .map(|t| TarifCandidat {
                        id: t.id,
                        article_id: t.article_id,
                        niveau: t.niveau,
                        regime: t.regime,
                        qf_min: t.qf_min_centimes,
                        qf_max: t.qf_max_centimes,
                        montant_annuel: t.montant_annuel_centimes,
                    })
                    .collect(),
            );
        }
        mois_tarifes.push((premier, calcul::fin_de_mois(an, mo), cle));
    }
    let articles: Vec<(Uuid, String, String, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT id, libelle, categorie, retire_a_partir_du FROM ent_fin_article
          WHERE etablissement_id = $1 AND actif ORDER BY code",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    // F2 (R-F4) : payeurs lus par le port **à la date de la prestation** et **par catégorie** ; une
    // catégorie qui n'a pas 100 % de payeurs n'est pas facturée et ouvre une tâche « payeur manquant »
    // (FAC-210) : jamais « au responsable 1 ».
    //
    // Régime mois par mois (§ 2.1, 2.2) : le régime de chaque mois dû est lu dans l'historique de l'élève ; la catégorie
    // est lue au **premier mois où elle est due** (une désignation de payeur posée au changement de novembre n'est pas
    // « manquante » pour une restauration due à partir de novembre).
    let premiers_mois: Vec<NaiveDate> = mois_tarifes.iter().map(|m| m.0).collect();
    let regimes = crate::finances_regime::regimes_par_mois(conn, etab, eleve_id, &premiers_mois).await?;
    // régime servant à choisir le tarif d'un mois : celui du mois, ou la fiche quand il est inconnu
    let regime_tarif: Vec<String> = regimes
        .iter()
        .map(|r| r.valeur().map(str::to_string).unwrap_or_else(|| eleve.regime.clone()))
        .collect();
    let libelle_regime = |m: usize| regime_tarif[m].clone();
    let mut premier_du: BTreeMap<String, usize> = BTreeMap::new();
    for (_, _, art_categorie, _) in &articles {
        if categories.is_some_and(|c| !c.contains(&art_categorie.as_str())) {
            continue;
        }
        if let Some(m) = (0..regimes.len()).find(|m| regimes[*m].du(art_categorie)) {
            let e = premier_du.entry(categorie_de_designation(art_categorie).to_string()).or_insert(m);
            *e = (*e).min(m);
        }
    }
    let mut parts_par_cat: BTreeMap<String, Vec<PartPayeur>> = BTreeMap::new();
    for (cat_des, m) in premier_du {
        let lus = payeurs_pour_facturer(conn, etab, eleve_id, &cat_des, premiers_mois[m]).await?;
        parts_par_cat.insert(cat_des, lus.map(|l| l.parts).unwrap_or_default());
    }
    if !parts_par_cat.is_empty() && parts_par_cat.values().all(|v| v.is_empty()) {
        return Ok(Err(
            "aucun foyer payeur défini (dossier élève, payeurs)".into()
        ));
    }
    let mut tous_foyers: Vec<Uuid> = parts_par_cat
        .values()
        .flat_map(|v| v.iter().map(|p| p.foyer_id))
        .collect();
    tous_foyers.sort();
    tous_foyers.dedup();
    let qf: HashMap<Uuid, i64> = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT foyer_id, quotient_centimes FROM ent_foyer_quotient
          WHERE etablissement_id = $1 AND annee_scolaire = $2 AND foyer_id = ANY($3)",
    )
    .bind(etab)
    .bind(annee)
    .bind(&tous_foyers)
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
    for foyer in &tous_foyers {
        rangs.insert(*foyer, rang_fratrie(conn, etab, *foyer, eleve_id).await?);
    }

    // F2 (CF-82, CF-83, E2) : restant de chaque prise en charge, consommé au fil des lignes de cette génération.
    let mut restants: HashMap<Uuid, i64> = HashMap::new();
    // Lignes principales dont la prise en charge est plafonnée : clé -> dû non plafonné du tronçon (suites 4, M4).
    let mut dus: HashMap<String, i64> = HashMap::new();
    let propres_prefixe = format!("{prefixe}:{eleve_id}:");
    // Seuls les articles **réellement recalculés** par cette génération (catégorie demandée et due au moins un mois) :
    // une ligne annuelle d'un autre article (scolarité pendant une réaction de régime) compte dans le plafond.
    let propres_articles: Vec<Uuid> = articles
        .iter()
        .filter(|a| categories.is_none_or(|c| c.contains(&a.2.as_str())) && regimes.iter().any(|r| r.du(&a.2)))
        .map(|a| a.0)
        .collect();
    let mut verrouilles: Vec<Uuid> = Vec::new();
    // Tâches `part_non_couverte` à régler : (catégorie, début, fin, lignes `:suivant:` censées porter la part non couverte
    // sous la forme (préfixe `{génération}:{élève}:{article}`, queue `:suivant:{foyer}:{foyer2}`)). Invariant de clôture
    // (défaut 2, § 2.8 c) : une tâche n'est close que si ces lignes sortent de la garde ou sont déjà posées.
    let mut a_resoudre: Vec<ARegler> = Vec::new();
    let mut lignes = Vec::new();
    let mut aucun_tarif = true;
    // Découpage par **(article, foyer payeur)**, au contenu (D-069, faibles c et 2) : les mois consécutifs où un article
    // est tarifé du même montant pour ce foyer, **et** couverts par les mêmes prises en charge si ce foyer est un
    // organisme (faible 3 : la couverture est contrôlée sur toute la période, pas au seul début du tronçon), forment
    // une seule ligne, quelles que soient les versions de grille, les tarifs des autres articles et ceux des autres
    // foyers (un article ou un foyer inchangé n'est ni coupé ni arrondi deux fois). Le premier tronçon garde la clé de
    // source historique, les suivants portent le suffixe `:m<AAAA-MM>` de leur premier mois.
    // Un article tarifé certains mois seulement n'est jamais gratuit en silence : anomalie bloquante.
    // Arrondi (D-069, faible 1 et M3) : chaque tronçon est arrondi **au centime inférieur** (prorata) et chaque
    // remise **au centime supérieur** : la famille ne paie jamais un centime en trop (somme des tronçons inférieure ou
    // égale au prorata de l'année entière, remises supérieures ou égales à leur valeur exacte), l'écart reste de moins
    // d'un centime par tronçon.
    let mut couvertures: HashMap<(Uuid, String, NaiveDate), Vec<Uuid>> = HashMap::new();
    let mut travaux: Vec<(usize, usize, usize, usize, Uuid)> = Vec::new();
    for (ai, (art_id, art_libelle, art_categorie, retire)) in articles.iter().enumerate() {
        // Mois où la catégorie est due selon le régime du mois : un mois non dû n'est pas un trou de grille. Un article
        // n'est écarté que s'il n'est dû aucun mois.
        let du_regime: Vec<bool> = regimes.iter().map(|r| r.du(art_categorie)).collect();
        if !du_regime.iter().any(|b| *b) {
            continue;
        }
        if categories.is_some_and(|c| !c.contains(&art_categorie.as_str())) {
            continue;
        }
        let cat_des = categorie_de_designation(art_categorie);
        let foyers: Vec<Uuid> = parts_par_cat
            .get(cat_des)
            .map(|v| v.iter().filter(|p| p.rang == 1).map(|p| p.foyer_id).collect())
            .unwrap_or_default();
        if foyers.is_empty() {
            continue;
        }
        // Fin de tarif datée (suites 3, D-069) : un mois qui commence à partir de `retire_a_partir_du` n'est plus dû
        // pour cet article ; les mois commencés avant le restent. Ce n'est pas un trou de grille : aucune anomalie.
        let nb_dus = mois_tarifes
            .iter()
            .take_while(|(premier, _, _)| retire.is_none_or(|r| *premier < r))
            .count();
        // montants[j][m] : montant annuel de l'article pour le foyer j le mois m (None : pas de tarif, ou plus dû)
        let montants: Vec<Vec<Option<i64>>> = foyers
            .iter()
            .map(|f| {
                mois_tarifes
                    .iter()
                    .enumerate()
                    .map(|(m, (_, _, cle))| {
                        if m >= nb_dus || !du_regime[m] {
                            return None;
                        }
                        calcul::choisir_tarif(
                            &cache[cle],
                            *art_id,
                            eleve.niveau.as_deref(),
                            &regime_tarif[m],
                            qf.get(f).copied(),
                        )
                            .map(|t| t.montant_annuel)
                    })
                    .collect()
            })
            .collect();
        for mj in &montants {
            // Contre-revue du pilote (Moyen 2, D-069) : seul un trou qui SUIT un mois tarifé bloque (article retiré d'une
            // grille en cours d'année : les mois suivants seraient gratuits en silence). Un trou en tête est l'arrivée
            // normale d'un service en cours d'année : l'article est facturé à partir de son premier mois tarifé. (Un mois
            // sans AUCUN tarif, grille oubliée, reste bloquant plus bas, quelle que soit sa place.) Les mois d'après une fin
            // de tarif datée ne sont pas des trous (le tarif s'arrête, il n'est pas oublié) : on ne regarde que les mois dus.
            // Seuls les mois **dus** (par le régime et par la fin de tarif datée) comptent.
            let dus_idx: Vec<usize> = (0..nb_dus).filter(|m| du_regime[*m]).collect();
            let premier_tarife = dus_idx.iter().position(|m| mj[*m].is_some());
            if let Some(m) = premier_tarife
                .and_then(|p| dus_idx.iter().skip(p).find(|m| mj[**m].is_none()).copied())
            {
                // un mois sans aucune ligne de grille applicable est annoncé en premier (message d'ensemble)
                if categories.is_none() {
                    if let Some(m_debut) = mois_tarifes.iter().position(|(_, _, cle)| cache[cle].is_empty()) {
                        let debut = mois_tarifes[m_debut].0;
                        return Ok(Err(format!(
                            "aucun tarif applicable à partir du {} (niveau {}, régime {}) : mettez une grille en vigueur pour ces mois",
                            debut.format("%d/%m/%Y"),
                            eleve.niveau.as_deref().unwrap_or("inconnu"),
                            libelle_regime(m_debut)
                        )));
                    }
                }
                return Ok(Err(format!(
                    "aucun tarif applicable pour l'article « {art_libelle} » à partir du {} (niveau {}, régime {}) alors qu'il est tarifé d'autres mois : complétez la grille en vigueur pour ces mois",
                    mois_tarifes[m].0.format("%d/%m/%Y"),
                    eleve.niveau.as_deref().unwrap_or("inconnu"),
                    libelle_regime(m)
                )));
            }
        }
        for (j, foyer) in foyers.iter().enumerate() {
            // Couverture mois par mois d'un organisme payeur : prises actives couvrant le **premier jour** du mois.
            let compte_organisme = parts_par_cat
                .get(cat_des)
                .and_then(|v| v.iter().find(|p| p.foyer_id == *foyer && p.rang == 1))
                .filter(|p| p.type_debiteur == "organisme")
                .map(|p| p.compte_id);
            let mut prises_du_mois: Vec<Vec<Uuid>> = vec![Vec::new(); mois_tarifes.len()];
            if let Some(compte) = compte_organisme {
                if !verrouilles.contains(foyer) {
                    verrouiller_foyer(conn, etab, *foyer).await?;
                    verrouilles.push(*foyer);
                }
                for (m, (premier, _, _)) in mois_tarifes.iter().enumerate() {
                    if montants[j][m].is_none() {
                        continue;
                    }
                    let cle = (compte, cat_des.to_string(), *premier);
                    if !couvertures.contains_key(&cle) {
                        let ids = prises_couvrantes(conn, etab, eleve_id, compte, cat_des, *premier)
                            .await?
                            .into_iter()
                            .map(|p| p.id)
                            .collect();
                        couvertures.insert(cle.clone(), ids);
                    }
                    prises_du_mois[m] = couvertures[&cle].clone();
                }
            }
            let signature = |m: usize| (montants[j][m], &prises_du_mois[m]);
            let mut debut = 0;
            while debut < mois_tarifes.len() {
                let mut fin = debut + 1;
                while fin < mois_tarifes.len() && signature(fin) == signature(debut) {
                    fin += 1;
                }
                if montants[j][debut].is_some() {
                    travaux.push((debut, ai, j, fin - debut, *foyer));
                }
                debut = fin;
            }
        }
    }
    travaux.sort();
    for (debut_idx, ai, _, nb_mois_run, foyer_run) in travaux {
    let (art_id, art_libelle, art_categorie, _) = &articles[ai];
    let candidats = &cache[&mois_tarifes[debut_idx].2];
    let mois_dus = nb_mois_run as u32;
    let periode = (mois_tarifes[debut_idx].0, mois_tarifes[debut_idx + nb_mois_run - 1].1);
    let suffixe_segment = if debut_idx == 0 {
        String::new()
    } else {
        format!(":m{}", periode.0.format("%Y-%m"))
    };
    {
        // Montant par foyer (le quotient peut différer d'un foyer à l'autre) :
        // chaque foyer paie sa part, au centime près (plus fort reste). Parts de la catégorie (F2).
        let parts: Vec<(Uuid, i64)> = parts_par_cat
            .get(categorie_de_designation(art_categorie))
            .map(|v| v.iter().filter(|p| p.rang == 1).map(|p| (p.foyer_id, p.part_bps)).collect())
            .unwrap_or_default();
        if parts.is_empty() {
            continue;
        }
        for (foyer, _) in parts.iter().filter(|(f, _)| *f == foyer_run) {
            let Some(t) = calcul::choisir_tarif(
                candidats,
                *art_id,
                eleve.niveau.as_deref(),
                &regime_tarif[debut_idx],
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
            // Organisme payeur (E2, CF-83) : une part portée par un compte `organisme` exige une prise en charge
            // **active** qui couvre la date et la catégorie ; sinon l'organisme n'est pas facturé et une tâche
            // `part_non_couverte` est ouverte (échue, annulée ou hors période = blocage). Le plafond de chaque
            // prise est celui de **cette** prise (M2) ; ce qui dépasse va au payeur suivant (rang 2), sinon tâche.
            let cat_des = categorie_de_designation(art_categorie);
            let mut part = part;
            let mut non_couvert = 0;
            let mut sans_prise = false;
            let mut plafonnee_org = false;
            let est_organisme = parts_par_cat
                .get(cat_des)
                .and_then(|v| v.iter().find(|p| p.foyer_id == *foyer && p.rang == 1))
                .filter(|p| p.type_debiteur == "organisme");
            if let Some(po) = est_organisme {
                if !verrouilles.contains(foyer) {
                    verrouiller_foyer(conn, etab, *foyer).await?;
                    verrouilles.push(*foyer);
                }
                let prises = prises_couvrantes(conn, etab, eleve_id, po.compte_id, cat_des, periode.0).await?;
                if prises.is_empty() {
                    sans_prise = true;
                    non_couvert = part;
                    part = 0;
                } else {
                    // M-2 : une prise **plafonnée** garde sa ligne sous l'examen de `completer_plafonnee` même quand le plafond
                    // ne bride pas (encore) : un plafond relevé ou abaissé n'est alors jamais lu comme un « tarif changé ».
                    plafonnee_org = prises.iter().any(|pr| pr.plafond.is_some());
                    let mut a_placer = part;
                    let mut place = 0_i64;
                    for pr in &prises {
                        if a_placer == 0 {
                            break;
                        }
                        let restant = match restants.get(&pr.id) {
                            Some(r) => *r,
                            None => restant_prise(conn, etab, eleve_id, *foyer, pr, (&propres_prefixe, &propres_articles, periode_gen)).await?,
                        };
                        let pris = a_placer.min(restant.max(0));
                        restants.insert(pr.id, if restant == i64::MAX { restant } else { restant - pris });
                        place = place.checked_add(pris).ok_or_else(|| invalide("montant hors limites"))?;
                        a_placer -= pris;
                    }
                    non_couvert = part - place;
                    part = place;
                }
            }
            let base_cle = format!("{prefixe}:{eleve_id}:{art_id}{suffixe_segment}");
            if sans_prise {
                crate::facturation_payeurs::ouvrir_tache(
                    conn,
                    etab,
                    "part_non_couverte",
                    Some(eleve_id),
                    est_organisme.map(|p| p.compte_id),
                    Some(cat_des),
                    periode.0,
                    "organisme désigné sans prise en charge active couvrant cette catégorie et cette date : l'organisme n'est pas facturé",
                    None,
                )
                .await?;
            } else if non_couvert > 0 {
                let suivants: Vec<(Uuid, i64)> = parts_par_cat
                    .get(cat_des)
                    .map(|v| v.iter().filter(|p| p.rang == 2).map(|p| (p.foyer_id, p.part_bps)).collect())
                    .unwrap_or_default();
                if suivants.is_empty() {
                    crate::facturation_payeurs::ouvrir_tache(
                        conn,
                        etab,
                        "part_non_couverte",
                        Some(eleve_id),
                        None,
                        Some(cat_des),
                        periode.0,
                        "la prise en charge de l'organisme est épuisée et aucun payeur suivant n'est désigné : la part non couverte n'est facturée à personne sans décision",
                        None,
                    )
                    .await?;
                } else {
                    // la part non couverte est portée par le payeur suivant : la tâche éventuelle est réglée (M3), une
                    // fois la garde passée et la ligne `:suivant:` effectivement portée
                    let mut porteurs: Vec<(String, String)> = Vec::new();
                    for (foyer2, montant) in calcul::repartir(non_couvert, &suivants) {
                        if montant == 0 {
                            continue;
                        }
                        porteurs.push((
                            format!("{prefixe}:{eleve_id}:{art_id}"),
                            format!(":suivant:{foyer}:{foyer2}"),
                        ));
                        lignes.push(LigneAFacturer {
                            foyer_id: foyer2,
                            eleve_id: Some(eleve_id),
                            article_id: Some(*art_id),
                            categorie: art_categorie.clone(),
                            libelle: format!("{art_libelle} — {nom} ({annee}) — part non couverte par la prise en charge"),
                            quantite: 1,
                            unitaire: montant,
                            source_cle: format!("{base_cle}:suivant:{foyer}:{foyer2}"),
                            periode: Some(periode),
                        });
                    }
                    a_resoudre.push((cat_des, periode.0, periode.1, porteurs, format!("{base_cle}:{foyer}")));
                }
            }
            if est_organisme.is_some() && !sans_prise && non_couvert == 0 {
                // tronçon entièrement couvert par la prise en charge : une tâche `part_non_couverte` ouverte
                // pour ces mois (prise renouvelée ou prolongée) est réglée (M3)
                a_resoudre.push((cat_des, periode.0, periode.1, Vec::new(), format!("{base_cle}:{foyer}")));
            }
            // Défaut 2 (§ 2.8 b) : sur une prise **plafonnée**, la ligne principale est toujours transmise à la garde,
            // même à part 0 (plafond épuisé : `completer_plafonnee` y rattache la ligne `:suivant:`) ; une ligne à 0 €
            // n'est jamais insérée (filtrée en sortie de garde).
            if part == 0 && !(plafonnee_org && non_couvert > 0) {
                continue;
            }
            if non_couvert > 0 || plafonnee_org {
                dus.insert(format!("{base_cle}:{foyer}"), part + non_couvert);
            }
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
                let remise = calcul::remise_haute(part, r.4);
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
        // Parts de la bourse : celles de la restauration, à défaut de l'internat, à défaut de `toutes`.
        let parts_bourse: Vec<(Uuid, i64)> = ["restauration", "internat", "toutes"]
            .iter()
            .find_map(|c| parts_par_cat.get(*c).filter(|v| !v.is_empty()))
            .map(|v| v.iter().filter(|p| p.rang == 1).map(|p| (p.foyer_id, p.part_bps)).collect())
            .unwrap_or_default();
        for (bourse_id, libelle, annuel) in bourses {
            let du = calcul::prorata(annuel, mois_dus, cal.nb_mois);
            for (foyer, part) in calcul::repartir(du, &parts_bourse) {
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
                    periode: Some(periode_gen),
                });
            }
        }
    }
    if aucun_tarif && categories.is_none() {
        return Ok(Err(format!(
            "aucun tarif applicable (niveau {}, régime {})",
            eleve.niveau.as_deref().unwrap_or("inconnu"),
            eleve.regime
        )));
    }
    // Contre-revue du pilote (Moyen 1, D-069) : un mois dû sans aucun tarif applicable n'est jamais gratuit en silence.
    if !aucun_tarif && categories.is_none() {
        if let Some(m_debut) = mois_tarifes.iter().position(|(_, _, cle)| cache[cle].is_empty()) {
            return Ok(Err(format!(
                "aucun tarif applicable à partir du {} (niveau {}, régime {}) : mettez une grille en vigueur pour ces mois",
                mois_tarifes[m_debut].0.format("%d/%m/%Y"),
                eleve.niveau.as_deref().unwrap_or("inconnu"),
                libelle_regime(m_debut)
            )));
        }
    }
    // Mois de régime **inconnu** (§ 2.1) : restauration et internat ne sont ni facturés ni crédités ; tâche sans événement
    // (une seule ouverte par élève) tant que des mois inconnus ne sont couverts par aucune ligne posée.
    let regime_concerne = articles
        .iter()
        .any(|a| matches!(a.2.as_str(), "restauration" | "internat") && categories.is_none_or(|c| c.contains(&a.2.as_str())));
    let inconnus: Vec<NaiveDate> = if regime_concerne {
        regimes
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r, crate::finances_regime::RegimeMois::Inconnu))
            .map(|(m, _)| premiers_mois[m])
            .collect()
    } else {
        Vec::new()
    };
    if regime_concerne {
        crate::finances_regime::signaler_mois_inconnus(conn, etab, annee, eleve_id, &inconnus).await?;
    }
    // Garde anti double facturation, **par périodes** (suites 3, E1 + E2 ; D-069) : voir `garde_periodes`.
    let libelles: HashMap<Uuid, String> = articles.iter().map(|a| (a.0, a.1.clone())).collect();
    let mut signaux: Vec<(String, NaiveDate, String)> = Vec::new();
    let mut non_portes: HashMap<String, i64> = HashMap::new();
    let resultat =
        garde_periodes(conn, etab, eleve_id, prefixe, lignes, &libelles, &dus, (&mut signaux, &mut non_portes)).await?;
    // M-1 (suites 4, correctifs) : les tâches `part_non_couverte` ne sont closes qu'une fois la garde passée, dans la
    // transaction de l'appelant qui insère ensuite les lignes (une erreur d'insertion annule tout) ; un refus de la
    // garde laisse les tâches ouvertes, la part n'étant pas facturée.
    if let Ok(sortie) = &resultat {
        for (cat, du, au, porteurs, principal) in a_resoudre {
            // F-7 : invariant par **montant** : la garde a rendu, pour chaque ligne principale plafonnée examinée, ce qui
            // du dû du tronçon n'est ni déjà posé ni réellement insérable ; la tâche reste ouverte tant qu'il en reste.
            let mut portee = non_portes.get(&principal).is_none_or(|r| *r == 0);
            for (pre, queue) in &porteurs {
                if !suivant_porte(conn, etab, sortie, pre, queue, du, au).await? {
                    portee = false;
                    break;
                }
            }
            if portee {
                crate::facturation_payeurs::resoudre_taches_periode(conn, etab, "part_non_couverte", eleve_id, cat, du, au)
                    .await?;
            }
        }
        // M-2 : signalements de plafond modifié, après les clôtures (une tâche ouverte par situation)
        for (cat, depuis, detail) in &signaux {
            crate::facturation_payeurs::ouvrir_tache(
                conn, etab, "part_non_couverte", Some(eleve_id), None, Some(cat.as_str()), *depuis, detail, None,
            )
            .await?;
        }
    }
    Ok(resultat)
}

/// Tâche `part_non_couverte` à régler après la garde : (catégorie, début, fin, lignes `:suivant:` qui doivent la porter,
/// clé de la ligne principale du tronçon, pour l'invariant par montant de F-7).
type ARegler = (&'static str, NaiveDate, NaiveDate, Vec<(String, String)>, String);

/// La part non couverte d'un tronçon est-elle réellement portée par une ligne `:suivant:` (invariant de clôture, § 2.8 c) ?
/// Oui si la sortie de la garde en contient une (clé d'origine ou complément `:c…`) qui chevauche la période, ou si une
/// telle ligne est déjà posée en base.
async fn suivant_porte(
    conn: &mut PgConnection,
    etab: Uuid,
    sortie: &[LigneAFacturer],
    pre: &str,
    queue: &str,
    du: NaiveDate,
    au: NaiveDate,
) -> Result<bool, EntDbError> {
    let chevauche = |p: Option<(NaiveDate, NaiveDate)>| p.is_some_and(|(a, b)| a <= au && b >= du);
    if sortie.iter().any(|l| {
        l.unitaire != 0 && l.source_cle.starts_with(pre) && l.source_cle.ends_with(queue) && chevauche(l.periode)
    }) {
        return Ok(true);
    }
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_fac_ligne l
          WHERE l.etablissement_id = $1 AND l.source_cle LIKE $2 AND left(l.source_cle, length($3)) = $3
            AND right(l.source_cle, length($4)) = $4 AND l.periode_debut <= $6 AND l.periode_fin >= $5
            AND l.quantite::BIGINT * l.montant_unitaire_centimes <> 0",
    )
    .bind(etab)
    .bind(format!("{}%", echapper_like(pre)))
    .bind(pre)
    .bind(queue)
    .bind(du)
    .bind(au)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(n > 0)
}

/// Une ligne déjà posée pour (élève, article), lue pour la garde par périodes.
struct LignePosee {
    cle: String,
    groupe: String,
    queue: String,
    periode: Option<(NaiveDate, NaiveDate)>,
    montant: i64,
    /// Mois **couverts nets** : ceux de la période moins ceux crédités par un avoir de régime (§ 2.4).
    nets: std::collections::BTreeSet<i32>,
}

/// `(groupe, queue)` d'une clé de ligne annuelle : la queue est ce qui suit `{prefixe}:{eleve}:{article}` sans le
/// suffixe de tronçon (`:m<AAAA-MM>`) ni de complément (`:c<AAAA-MM>`) ; le groupe rassemble la ligne principale d'un
/// foyer et ses lignes `:suivant:` (un même mois est porté par l'une ou par l'autre, jamais par les deux).
/// Retire le suffixe de mois `:mAAAA-MM` ou `:cAAAA-MM` d'un reste de clé, **seulement si les chiffres y sont** :
/// un foyer dont l'UUID commence par `c` ou `m` ne doit pas voir sa clé tronquée.
pub(crate) fn sans_suffixe_mois(reste: &str) -> &str {
    let b = reste.as_bytes();
    if reste.len() >= 9
        && b[0] == b':'
        && (b[1] == b'm' || b[1] == b'c')
        && b[2..6].iter().all(u8::is_ascii_digit)
        && b[6] == b'-'
        && b[7..9].iter().all(u8::is_ascii_digit)
    {
        &reste[9..]
    } else {
        reste
    }
}

fn groupe_de_cle(cle: &str, pre: &str) -> Option<(String, String)> {
    let reste = sans_suffixe_mois(cle.strip_prefix(pre)?);
    let groupe = match reste.strip_prefix(":suivant:") {
        Some(origine) => format!(":{}", origine.split(':').next().unwrap_or("")),
        None => reste.to_string(),
    };
    Some((groupe, reste.to_string()))
}

/// Échappe `\`, `%` et `_` d'un préfixe de clé pour un `LIKE`.
fn echapper_like(p: &str) -> String {
    p.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Clé de la ligne principale dont une ligne `:suivant:` est issue (`{base}:suivant:{foyer}:{foyer2}` →
/// `{base}:{foyer}`).
fn principal_de_suivant(cle: &str) -> Option<String> {
    let i = cle.find(":suivant:")?;
    let foyer = cle[i + ":suivant:".len()..].split(':').next()?;
    Some(format!("{}:{foyer}", &cle[..i]))
}

pub(crate) fn indice_mois(d: NaiveDate) -> i32 {
    d.year() * 12 + d.month0() as i32
}

pub(crate) fn premier_du_mois(i: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(i.div_euclid(12), i.rem_euclid(12) as u32 + 1, 1).expect("date valide")
}

/// Garde anti double facturation fondée sur le **chevauchement des périodes** et le **montant** des lignes déjà
/// posées pour (élève, article, foyer), réductions et `:suivant:` compris (E1, E2 des suites 3 ; D-069).
///
/// Pour chaque ligne à poser :
/// - période **disjointe** de tout ce qui est posé (renouvellement d'une prise, nouveau tronçon) : facturable telle quelle ;
/// - **même clé, même période** : rejeu, laissée telle quelle (la base l'ignore ; un montant plus faible vient d'un
///   plafond de prise consumé, un montant plus fort est refusé ; une ligne `:suivant:` n'est jamais comparée) ;
/// - période **déjà en partie couverte** (prolongation d'une prise, mois déjà posés sous une autre clé) : seuls les
///   **mois manquants** sont facturés, sous une clé nouvelle `:c<mois de début>` ; aucune ligne posée n'est refacturée ;
/// - mois couverts **à un autre tarif** que celui d'aujourd'hui : refus explicite (régularisation par complément ou
///   avoir, F4), jamais d'ignorance silencieuse.
///
/// `sorties.1` reçoit, pour chaque ligne principale plafonnée examinée par `completer_plafonnee`, le montant du dû de son
/// tronçon qui n'est **porté par personne** au-delà des arrondis (F-7) : 0 quand tout est posé ou insérable.
// 8 arguments : connexion, contexte (établissement, élève, préfixe), lignes, deux tables de lecture et les collecteurs.
#[allow(clippy::too_many_arguments)]
async fn garde_periodes(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    prefixe: &str,
    lignes: Vec<LigneAFacturer>,
    libelles: &HashMap<Uuid, String>,
    dus: &HashMap<String, i64>,
    sorties: (&mut Vec<(String, NaiveDate, String)>, &mut HashMap<String, i64>),
) -> Result<Result<Vec<LigneAFacturer>, String>, EntDbError> {
    let (signaux, non_portes) = sorties;
    // Lignes `:suivant:` d'une ligne principale plafonnée : traitées avec elle (voir plus bas), jamais seules.
    let mut suivants_de: HashMap<String, Vec<LigneAFacturer>> = HashMap::new();
    let mut lignes = lignes;
    if !dus.is_empty() {
        let (suivants, autres): (Vec<_>, Vec<_>) = lignes
            .into_iter()
            .partition(|l| principal_de_suivant(&l.source_cle).is_some_and(|k| dus.contains_key(&k)));
        for l in suivants {
            if let Some(k) = principal_de_suivant(&l.source_cle) {
                suivants_de.entry(k).or_default().push(l);
            }
        }
        lignes = autres;
    }
    let mut prefixes: Vec<String> = lignes
        .iter()
        .filter_map(|l| l.article_id.map(|a| format!("{prefixe}:{eleve_id}:{a}")))
        .collect();
    prefixes.sort();
    prefixes.dedup();
    if prefixes.is_empty() {
        return Ok(Ok(lignes.into_iter().filter(|l| l.unitaire != 0).collect()));
    }
    // Faible 5 puis F-1 (suites 4, correctifs) : une requête **par préfixe**, le motif `LIKE` étant alors une
    // constante pour le planificateur (avec la jointure sur `unnest`, il n'est qu'un paramètre de boucle et ne borne
    // jamais l'index). Le `left()` confirme l'égalité exacte du préfixe non échappé. Voir `docs/socles/24_…` : la borne
    // de plage ne sert l'index unique `(établissement, source_cle)` que sous la collation `C` ; sous une autre
    // collation, la lecture reste bornée par l'établissement et le préfixe (jamais par table entière hors établissement).
    let mut rows: Vec<(String, Option<NaiveDate>, Option<NaiveDate>, i64, Uuid)> = Vec::new();
    for pre in &prefixes {
        let mut r: Vec<(String, Option<NaiveDate>, Option<NaiveDate>, i64, Uuid)> = sqlx::query_as(
            "SELECT l.source_cle, l.periode_debut, l.periode_fin, (l.quantite::BIGINT * l.montant_unitaire_centimes), l.id
               FROM ent_fac_ligne l
              WHERE l.etablissement_id = $1 AND l.source_cle LIKE $2 AND left(l.source_cle, length($3)) = $3",
        )
        .bind(etab)
        .bind(format!("{}%", echapper_like(pre)))
        .bind(pre)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        rows.append(&mut r);
    }
    // Couverture croisée (faible 9, suites 4 ; E-1, correctifs) : un changement de régime a facturé la catégorie sous
    // `regime:<événement>` ; un rejeu **annuel** (régime désormais celui d'arrivée) ne la refacture jamais sous
    // `annuel:` (la clé serait nouvelle, la période chevaucherait : double facturation). Règle précise, **par mois** :
    // * seules les lignes de régime dont la **période chevauche** celle des lignes annuelles à poser comptent (l'article
    //   et la clé ne portent pas l'année : le changement de l'année N ne dit rien de l'année N+1) ;
    // * seuls les **mois effectivement couverts** par une ligne de régime sont écartés de la ligne annuelle (jamais
    //   le groupe entier) : les autres mois restent facturés, sous clé `:c<AAAA-MM>` ;
    // * une ligne de régime **créditée par avoir** ne couvre que les mois que l'avoir lui laisse (les premiers, l'avoir
    //   créditant la fin de période) : entièrement créditée, elle ne couvre rien ; **règle prudente** pour un avoir
    //   partiel, la part non créditée est arrondie au mois supérieur (jamais de double facturation) ;
    // * une ligne de régime sans période (inexistante en pratique : toute génération en pose une) ne couvre rien ;
    // * contre-revue du pilote (D-069) : les mois de l'année **avant** le changement de régime sont écartés aussi (l'élève
    //   n'était pas à ce régime : l'annuel, calculé au régime courant, les surfacturerait).
    // Seul le sens annuel → régime est lu : un changement de régime reste facturable à nouveau après un avoir.
    let mut croisees: HashMap<(Uuid, String), std::collections::BTreeSet<i32>> = HashMap::new();
    let bornes = lignes
        .iter()
        .filter_map(|l| l.periode)
        .fold(None, |acc: Option<(NaiveDate, NaiveDate)>, (d, f)| {
            Some(acc.map_or((d, f), |(a, b)| (a.min(d), b.max(f))))
        });
    if let (true, Some((min_d, max_f))) = (prefixe.starts_with("annuel:"), bornes) {
        let arts: Vec<Uuid> = lignes.iter().filter_map(|l| l.article_id).collect();
        let rows: Vec<(Uuid, String, NaiveDate, NaiveDate, i64, i64)> = sqlx::query_as(
            "SELECT l.article_id, l.source_cle, l.periode_debut, l.periode_fin,
                    (l.quantite::BIGINT * l.montant_unitaire_centimes),
                    COALESCE((SELECT SUM(a.quantite::BIGINT * a.montant_unitaire_centimes) FROM ent_fac_ligne a
                               WHERE a.etablissement_id = l.etablissement_id AND a.ligne_origine_id = l.id), 0)::BIGINT
               FROM ent_fac_ligne l
              WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.article_id = ANY($3)
                AND l.source_cle LIKE 'regime:%' AND l.source_cle NOT LIKE 'regime:%:avoir:%'
                AND l.periode_debut IS NOT NULL AND l.periode_fin IS NOT NULL
                AND l.periode_debut <= $5 AND l.periode_fin >= $4",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(&arts)
        .bind(min_d)
        .bind(max_f)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (art, cle, pd, pf, total, credits) in rows {
            let marque = format!(":{eleve_id}:{art}");
            let Some(i) = cle.find(&marque) else { continue };
            let pre = &cle[..i + marque.len()];
            let Some((groupe, _)) = groupe_de_cle(&cle, pre) else { continue };
            let net = total + credits;
            if total == 0 || net == 0 || net.signum() != total.signum() {
                continue; // entièrement créditée (ou sans montant) : ne couvre rien
            }
            let nm = i128::from(indice_mois(pf) - indice_mois(pd) + 1);
            let gardes = (i128::from(net.abs()) * nm + i128::from(total.abs()) - 1) / i128::from(total.abs());
            let gardes = gardes.clamp(0, nm) as i32;
            // Contre-revue du pilote (D-069, jamais de surfacturation) : l'annuel porte l'année au régime COURANT ; les mois
            // de l'année qui PRÉCÈDENT le changement de régime n'étaient pas à ce régime (ex. externe en septembre-octobre,
            // demi-pension depuis novembre) : ils ne sont jamais facturés par l'annuel à ce régime. Écartés aussi.
            let debut_annee = indice_mois(min_d).min(indice_mois(pd));
            croisees
                .entry((art, groupe))
                .or_default()
                .extend(debut_annee..indice_mois(pd) + gardes);
        }
    }
    // Mois couverts nets (§ 2.4) : les avoirs de régime posés sur ces lignes en retirent des mois.
    let ids: Vec<Uuid> = rows.iter().map(|r| r.4).collect();
    let credits: Vec<(Uuid, String, Option<NaiveDate>, Option<NaiveDate>, i64)> = sqlx::query_as(
        "SELECT a.ligne_origine_id, a.source_cle, a.periode_debut, a.periode_fin,
                (a.quantite::BIGINT * a.montant_unitaire_centimes)::BIGINT
           FROM ent_fac_ligne a
          WHERE a.etablissement_id = $1 AND a.ligne_origine_id = ANY($2) AND a.source_cle LIKE 'regime:%:avoir:%'",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut avoirs_de: HashMap<Uuid, Vec<crate::finances_regime::AvoirRegime>> = HashMap::new();
    for (id, cle, d, f, montant) in credits {
        avoirs_de
            .entry(id)
            .or_default()
            .push(crate::finances_regime::AvoirRegime { cle, periode: d.zip(f), montant });
    }
    let mut posees: HashMap<String, Vec<LignePosee>> = HashMap::new();
    for (cle, debut, fin, montant, id) in rows {
        let Some(pre) = prefixes.iter().find(|p| cle.starts_with(p.as_str())) else { continue };
        let Some((groupe, queue)) = groupe_de_cle(&cle, pre) else { continue };
        let nets = match debut.zip(fin) {
            Some((d, f)) => {
                let decouverts = crate::finances_regime::mois_decouverts(
                    d,
                    f,
                    montant,
                    avoirs_de.get(&id).map(Vec::as_slice).unwrap_or(&[]),
                );
                (indice_mois(d)..=indice_mois(f)).filter(|m| !decouverts.contains(m)).collect()
            }
            None => Default::default(),
        };
        posees.entry(pre.clone()).or_default().push(LignePosee {
            cle,
            groupe,
            queue,
            periode: debut.zip(fin),
            montant,
            nets,
        });
    }
    let mut sortie = Vec::with_capacity(lignes.len());
    for l in lignes {
        let (Some(art), Some((d, f))) = (l.article_id, l.periode) else {
            sortie.push(l);
            continue;
        };
        let pre = format!("{prefixe}:{eleve_id}:{art}");
        let Some((groupe, queue)) = groupe_de_cle(&l.source_cle, &pre) else {
            sortie.push(l);
            continue;
        };
        let posees_vide = Vec::new();
        let deja = posees.get(&pre).unwrap_or(&posees_vide);
        let cle_posee = deja.iter().find(|p| p.cle == l.source_cle);
        // mois de cette ligne déjà facturés par un changement de régime (E-1) : écartés, pas le groupe entier
        let nl_mois = indice_mois(d)..=indice_mois(f);
        let croises: std::collections::BTreeSet<i32> = croisees
            .get(&(art, groupe.clone()))
            .map(|c| c.iter().copied().filter(|m| nl_mois.contains(m)).collect())
            .unwrap_or_default();
        if cle_posee.is_none() && croises.len() == nl_mois.clone().count() {
            suivants_de.remove(&l.source_cle);
            continue; // tous les mois facturés par un changement de régime : jamais refacturés sous `annuel:`
        }
        if deja.is_empty() && croises.is_empty() {
            if let Some(suivants) = suivants_de.remove(&l.source_cle) {
                sortie.extend(suivants);
            }
            sortie.push(l);
            continue;
        }
        if let Some(p) = cle_posee {
            match p.periode {
                // ligne sans période (ancienne) : traitée comme un rejeu
                None => {
                    sortie.push(l);
                    continue;
                }
                // Rejeu seulement si la ligne posée couvre encore **tous** ses mois : un mois crédité par un avoir de régime
                // n'est plus couvert, un retour au régime se refacture sous une clé nouvelle (`:c<mois>`, § 2.3).
                Some((pd, pf)) if (pd, pf) == (d, f) && p.nets.len() == nl_mois.clone().count() => {
                    // même clé, même période : rejeu. Plus faible = plafond de prise consumé (la part passe au payeur
                    // suivant) ; plus fort = le montant a monté depuis la génération : refus, jamais ignoré.
                    // Une ligne `:suivant:` n'est pas comparée : son montant suit le restant des plafonds, qui varie.
                    // M-2 (correctifs) : une ligne plafonnée a pour montant le plafond **restant**, qui varie (plafond relevé,
                    // payeur suivant désigné après coup) : elle est examinée par `completer_plafonnee`, jamais refusée
                    // comme un « tarif changé ».
                    if !queue.starts_with(":suivant:")
                        && !dus.contains_key(&l.source_cle)
                        && l.unitaire.abs() > p.montant.abs() + 1
                    {
                        return Ok(Err(refus_tarif(libelles, art, p, &l)));
                    }
                    // Prise plafonnée (M1, suites 4) : le rejeu reste à examiner (payeur suivant désigné après coup).
                    if !dus.contains_key(&l.source_cle) {
                        sortie.push(l);
                        continue;
                    }
                }
                Some(_) => {}
            }
        }
        let nl = i64::from(indice_mois(f) - indice_mois(d) + 1);
        let a = l.unitaire;
        let memes: Vec<&LignePosee> = deja.iter().filter(|p| p.groupe == groupe && p.periode.is_some()).collect();
        let mois_l = indice_mois(d)..=indice_mois(f);
        let couverts: std::collections::BTreeSet<i32> = memes
            .iter()
            .flat_map(|p| p.nets.iter().copied())
            .filter(|m| mois_l.contains(m))
            .collect();
        // Contre-revue du pilote (M2) : une ligne ancienne SANS période, du même groupe (article, foyer), peut couvrir toute
        // l'année : une clé nouvelle n'est jamais jugée « disjointe » face à elle. Refus explicite, rien n'est facturé deux fois.
        if cle_posee.is_none() && deja.iter().any(|p| p.groupe == groupe && p.periode.is_none()) {
            let nom = libelles.get(&art).map(String::as_str).unwrap_or("?");
            return Ok(Err(format!(
                "l'article « {nom} » a déjà été facturé sans période (génération ancienne) : la nouvelle ligne du {} au {} ({} c) n'est pas posée pour ne rien facturer deux fois ; régularisez par un complément ou un avoir",
                d.format("%d/%m/%Y"),
                f.format("%d/%m/%Y"),
                l.unitaire
            )));
        }
        let couverts_tot: std::collections::BTreeSet<i32> = couverts.union(&croises).copied().collect();
        if couverts_tot.is_empty() && cle_posee.is_none() {
            // période disjointe : facturable (avec les lignes `:suivant:` de sa prise plafonnée, le cas échéant)
            if let Some(suivants) = suivants_de.remove(&l.source_cle) {
                sortie.extend(suivants);
            }
            sortie.push(l);
            continue;
        }
        if let Some(&du) = dus.get(&l.source_cle) {
            // Prise en charge plafonnée (M1 et M4, suites 4) : voir `completer_plafonnee`.
            let suivants = suivants_de.remove(&l.source_cle).unwrap_or_default();
            match completer_plafonnee(&pre, art, &l, suivants, du, &memes, &couverts, &croises, cle_posee.is_some(), libelles) {
                Ok((v, signal, non_porte)) => {
                    non_portes.insert(l.source_cle.clone(), non_porte);
                    sortie.extend(v);
                    if let Some(detail) = signal {
                        // plafond qui baisse (ou consommé ailleurs) sous ce qui est déjà facturé : jamais refacturé ni
                        // remboursé en silence ; la tâche est ouverte par l'appelant, après la clôture des tâches réglées
                        signaux.push((categorie_de_designation(&l.categorie).to_string(), d, detail));
                    }
                }
                Err(motif) => return Ok(Err(motif)),
            }
            continue;
        }
        // mois déjà couverts par une ligne de **même nature** : même tarif au mois, aux arrondis près
        for p in memes.iter().filter(|p| p.queue == queue && !queue.starts_with(":suivant:")) {
            let Some((pd, pf)) = p.periode else { continue };
            let (lo, hi) = (indice_mois(pd).max(indice_mois(d)), indice_mois(pf).min(indice_mois(f)));
            if lo > hi {
                continue;
            }
            let np = i64::from(indice_mois(pf) - indice_mois(pd) + 1);
            let ecart = (i128::from(p.montant) * i128::from(nl) - i128::from(a) * i128::from(np)).abs();
            if ecart > 3 * i128::from(nl + np) {
                return Ok(Err(refus_tarif(libelles, art, p, &l)));
            }
        }
        if cle_posee.is_some() {
            sortie.push(l.clone()); // la clé existe : la base ignore cette ligne, les mois manquants suivent
        }
        // mois manquants, par suites ininterrompues : une ligne chacune, clé nouvelle `:c<AAAA-MM>`
        let est_remise = queue.starts_with(":red:");
        let mut m = *mois_l.start();
        while m <= *mois_l.end() {
            if couverts_tot.contains(&m) {
                m += 1;
                continue;
            }
            let debut = m;
            while m <= *mois_l.end() && !couverts_tot.contains(&m) {
                m += 1;
            }
            let nb = i64::from(m - debut);
            let part = calcul::proportion(a.abs(), nb, nl);
            let part = if est_remise && part * nl < a.abs() * nb { part + 1 } else { part };
            let part = if a < 0 { -part } else { part };
            if part == 0 {
                continue;
            }
            let (d2, f2) = (premier_du_mois(debut), calcul::fin_de_mois(premier_du_mois(m - 1).year(), premier_du_mois(m - 1).month()));
            let reste = l.source_cle.strip_prefix(pre.as_str()).unwrap_or("");
            let reste = sans_suffixe_mois(reste);
            let mut c = l.clone();
            c.unitaire = part;
            c.periode = Some((d2, f2));
            c.source_cle = format!("{pre}:c{}{reste}", d2.format("%Y-%m"));
            c.libelle = format!("{} — complément {}", l.libelle, d2.format("%m/%Y"));
            sortie.push(c);
        }
    }
    // Aucune ligne à 0 € n'est jamais insérée (défaut 2, § 2.8 b : la ligne principale d'une prise plafonnée à part 0
    // traverse la garde pour que `completer_plafonnee` y rattache la ligne `:suivant:`).
    sortie.retain(|l| l.unitaire != 0);
    Ok(Ok(sortie))
}

/// Ligne principale **plafonnée** (le dû du tronçon dépasse ce que la prise en charge couvre) déjà en partie posée.
/// Le plafond est cumulatif, pas mensuel : la part d'un mois manquant n'est pas le prorata de la ligne plafonnée.
///
/// * `du` : dû non plafonné du tronçon (principal + part non couverte) ; `l.unitaire` : plafond restant utilisé ;
/// * **mois manquants** : par suite ininterrompue, le principal prend `min(dû de la suite, plafond non consommé)`
///   (plafond non consommé = ligne plafonnée − principal déjà posé sur le tronçon), le reste va aux lignes `:suivant:`
///   (clé `:c<AAAA-MM>`) ; sans payeur suivant, ce reste n'est pas facturé (la tâche `part_non_couverte` est ouverte) ;
/// * **mois couverts mais insuffisants** (payeur suivant désigné après coup) : le manque au dû
///   (`dû × mois couverts / mois − déjà posé`) est facturé au payeur suivant, sous la clé `:suivant:` d'origine, une
///   seule fois (la base ignore une clé déjà posée) ;
/// * déjà posé **au-dessus** du dû (tarif baissé) : refus explicite ; jamais de refacturation ;
/// * manque d'un payeur suivant dont la ligne `:suivant:` est **déjà posée** (dû en hausse sur le même tronçon : quotient
///   familial ou niveau corrigés) : clé de complément `:c<premier mois libre>`, la clé d'origine étant ignorée par la base
///   (palier des faibles de facturation, C3 : la hausse n'était facturée à personne).
///
/// Rend aussi le **non porté** (F-7) : dû du tronçon (hors mois facturés par un changement de régime) − déjà posé (mois
/// couverts nets) − lignes rendues réellement insérables (une clé déjà posée ne compte pas), 0 aux arrondis près.
#[allow(clippy::too_many_arguments)]
fn completer_plafonnee(
    pre: &str,
    art: Uuid,
    l: &LigneAFacturer,
    suivants: Vec<LigneAFacturer>,
    du: i64,
    memes: &[&LignePosee],
    couverts: &std::collections::BTreeSet<i32>,
    croises: &std::collections::BTreeSet<i32>,
    cle_posee: bool,
    libelles: &HashMap<Uuid, String>,
) -> Result<(Vec<LigneAFacturer>, Option<String>, i64), String> {
    let Some((d, f)) = l.periode else { return Ok((vec![l.clone()], None, 0)) };
    let mois_l = indice_mois(d)..=indice_mois(f);
    let nl = i64::from(indice_mois(f) - indice_mois(d) + 1);
    let (mut pose_total, mut pose_principal) = (0_i128, 0_i128);
    for p in memes {
        let Some((pd, pf)) = p.periode else { continue };
        let np = i128::from(indice_mois(pf) - indice_mois(pd) + 1);
        // mois **couverts nets** de cette ligne dans la fenêtre de la ligne à poser (§ 2.4)
        let nets_communs = p.nets.iter().filter(|m| mois_l.contains(m)).count() as i128;
        if nets_communs == 0 {
            continue;
        }
        let part = i128::from(p.montant) * nets_communs / np;
        pose_total += part;
        if !p.queue.starts_with(":suivant:") {
            pose_principal += part;
        }
    }
    let c = couverts.len() as i128;
    if pose_total * i128::from(nl) - i128::from(du) * c > 3 * (i128::from(nl) + c) {
        let nom = libelles.get(&art).map(String::as_str).unwrap_or("?");
        return Err(format!(
            "le tarif de l'article « {nom} » a baissé depuis la génération précédente pour des mois déjà facturés : {pose_total} c déjà posés pour {du} c dus ; rien n'est refacturé, régularisez par un complément ou un avoir"
        ));
    }
    let mut sortie = Vec::new();
    if cle_posee {
        sortie.push(l.clone()); // la clé existe : la base ignore cette ligne
    }
    let poids: Vec<(usize, i64)> = suivants.iter().enumerate().map(|(i, s)| (i, s.unitaire)).collect();
    let reste_cle = |cle: &str| -> String {
        sans_suffixe_mois(cle.strip_prefix(pre).unwrap_or("")).to_string()
    };
    // mois manquants : principal sous le plafond non consommé, le reste au payeur suivant
    let mut plafond_libre = (i128::from(l.unitaire) - pose_principal).max(0);
    let mut m = *mois_l.start();
    while m <= *mois_l.end() {
        if couverts.contains(&m) || croises.contains(&m) {
            m += 1;
            continue;
        }
        let debut = m;
        while m <= *mois_l.end() && !couverts.contains(&m) && !croises.contains(&m) {
            m += 1;
        }
        let du_suite = calcul::proportion(du, i64::from(m - debut), nl);
        let principal = i128::from(du_suite).min(plafond_libre) as i64;
        plafond_libre -= i128::from(principal);
        let d2 = premier_du_mois(debut);
        let f2 = calcul::fin_de_mois(premier_du_mois(m - 1).year(), premier_du_mois(m - 1).month());
        let suffixe = format!("{pre}:c{}", d2.format("%Y-%m"));
        if principal > 0 {
            let mut p = l.clone();
            p.unitaire = principal;
            p.periode = Some((d2, f2));
            p.source_cle = format!("{suffixe}{}", reste_cle(&l.source_cle));
            p.libelle = format!("{} — complément {}", l.libelle, d2.format("%m/%Y"));
            sortie.push(p);
        }
        let reste = du_suite - principal;
        if reste > 0 && !suivants.is_empty() {
            for (i, montant) in calcul::repartir(reste, &poids) {
                if montant == 0 {
                    continue;
                }
                let mut sv = suivants[i].clone();
                sv.unitaire = montant;
                sv.periode = Some((d2, f2));
                sv.source_cle = format!("{suffixe}{}", reste_cle(&suivants[i].source_cle));
                sv.libelle = format!("{} — complément {}", suivants[i].libelle, d2.format("%m/%Y"));
                sortie.push(sv);
            }
        }
    }
    // mois couverts mais insuffisants : manque au dû, au payeur suivant désigné (après coup)
    let mut manque = i128::from(calcul::proportion(du, c as i64, nl)) - pose_total;
    // M-2 : plafond **relevé** (même clé, même période) : le principal prend d'abord ce que le plafond libre permet,
    // en complément sous une clé nouvelle `:c<premier mois libre>` ; le reste va au payeur suivant désigné.
    let existantes: Vec<&str> = memes.iter().map(|p| p.cle.as_str()).collect();
    if manque > 3 * c && plafond_libre > 0 {
        let extra = manque.min(plafond_libre);
        let reste = reste_cle(&l.source_cle);
        let libre = mois_l.clone().find(|mm| {
            let cle = format!("{pre}:c{}{reste}", premier_du_mois(*mm).format("%Y-%m"));
            !existantes.contains(&cle.as_str()) && !sortie.iter().any(|s| s.source_cle == cle)
        });
        if let Some(mm) = libre {
            let mut p = l.clone();
            p.unitaire = extra as i64;
            p.source_cle = format!("{pre}:c{}{reste}", premier_du_mois(mm).format("%Y-%m"));
            p.libelle = format!("{} — complément (plafond relevé)", l.libelle);
            sortie.push(p);
            manque -= extra;
            plafond_libre -= extra;
        }
    }
    if manque > 3 * c && !suivants.is_empty() {
        for (i, montant) in calcul::repartir(manque as i64, &poids) {
            if montant == 0 {
                continue;
            }
            let mut sv = suivants[i].clone();
            sv.unitaire = montant;
            if existantes.contains(&sv.source_cle.as_str()) {
                // C3 : ligne `:suivant:` déjà posée (à un dû plus bas) : la même clé serait ignorée par la base ; le
                // manque va sous une clé de complément, au premier mois libre (rejeu : plus de manque, rien de posé)
                let reste = reste_cle(&sv.source_cle);
                // contre-revue M-1 : ni une clé posée, ni une clé déjà rendue par cette passe (complément des mois
                // manquants du même payeur suivant)
                let Some(mm) = mois_l.clone().find(|mm| {
                    let cle = format!("{pre}:c{}{reste}", premier_du_mois(*mm).format("%Y-%m"));
                    !existantes.contains(&cle.as_str()) && !sortie.iter().any(|s| s.source_cle == cle)
                }) else {
                    continue; // tous les mois portent déjà un complément : non porté, la tâche reste ouverte
                };
                sv.source_cle = format!("{pre}:c{}{reste}", premier_du_mois(mm).format("%Y-%m"));
                sv.libelle = format!("{} — complément", suivants[i].libelle);
            }
            sortie.push(sv);
        }
    }
    // M-2 : plafond qui **baisse** (ou consommé ailleurs) sous le principal déjà facturé : règle prudente, rien n'est
    // refacturé ni crédité d'office ; le dépassement est signalé par une tâche.
    let signal = (cle_posee && pose_principal > i128::from(l.unitaire) + 1).then(|| {
        format!(
            "plafond de la prise en charge modifié : {pose_principal} c déjà facturés à l'organisme pour {} c de plafond restant ; rien n'est refacturé ni crédité d'office, à régulariser",
            l.unitaire
        )
    });
    // Suites 4, moyen 5 : plafond **non consommé** (relevé, ou libéré par un avoir de régime) alors qu'une part est déjà
    // facturée au payeur suivant : l'organisme pourrait désormais la prendre, mais rien n'est crédité ni refacturé
    // d'office (règle prudente, D-069) ; c'est signalé, sans montant (le calcul est par tronçon, une tâche par
    // catégorie : le montant d'un tronçon serait partiel).
    let pose_suivant = pose_total - pose_principal;
    let signal = signal.or_else(|| {
        let reprenable = plafond_libre.min(pose_suivant);
        (cle_posee && reprenable > 3 * (c + 1)).then(|| {
            "plafond de la prise en charge non consommé alors qu'une part est facturée au payeur suivant : elle pourrait revenir à l'organisme ; rien n'est crédité ni refacturé d'office, à régulariser".to_string()
        })
    });
    // F-7 : non porté = dû du tronçon (hors mois facturés par un changement de régime) − déjà posé − insérable.
    let hors = croises.iter().filter(|m| !couverts.contains(m)).count() as i64;
    let attendu = i128::from(du) - i128::from(calcul::proportion(du, hors, nl));
    // chaque clé une seule fois (la base garde la première ligne d'une clé et ignore les suivantes)
    let mut vues: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let insere: i128 = sortie
        .iter()
        .filter(|s| !existantes.contains(&s.source_cle.as_str()) && vues.insert(s.source_cle.as_str()))
        .map(|s| i128::from(s.unitaire) * i128::from(s.quantite))
        .sum();
    let non_porte = attendu - pose_total - insere;
    let non_porte = if non_porte > 3 * (i128::from(nl) + 1) { i64::try_from(non_porte).unwrap_or(i64::MAX) } else { 0 };
    Ok((sortie, signal, non_porte))
}

fn refus_tarif(libelles: &HashMap<Uuid, String>, art: Uuid, p: &LignePosee, l: &LigneAFacturer) -> String {
    let nom = libelles.get(&art).map(String::as_str).unwrap_or("?");
    let periode = p
        .periode
        .map(|(a, b)| format!(" (période du {} au {})", a.format("%d/%m/%Y"), b.format("%d/%m/%Y")))
        .unwrap_or_default();
    if l.source_cle.contains(":red:") {
        // M4 (suites 4) : un taux de remise qui change n'est pas un changement de tarif : motif distinct.
        return format!(
            "la remise sur l'article « {nom} » a changé depuis la génération précédente pour des mois déjà facturés{periode} : {} c déjà posés, {} c aujourd'hui (taux de remise modifié) ; rien n'est refacturé, régularisez par un complément ou un avoir",
            p.montant.abs(),
            l.unitaire.abs()
        );
    }
    format!(
        "le tarif de l'article « {nom} » a changé depuis la génération précédente pour des mois déjà facturés{periode} : {} c déjà posés, {} c aujourd'hui ; rien n'est refacturé, régularisez par un complément ou un avoir",
        p.montant, l.unitaire
    )
}

/// Facturation annuelle d'un élève (inscription validée, génération FIN-02). Le tarif se choisit mois par mois,
/// au premier jour de chaque mois dû (D-069, M-A) : le jour de génération n'intervient pas. Le régime aussi se lit mois
/// par mois ; les mois facturés hors régime sont **régularisés d'abord** (avoirs comptés au rapport), si bien qu'un
/// changement dont la réaction n'a jamais tourné est rattrapé par la génération suivante.
pub(crate) async fn facturer_annee_eleve(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve_id: Uuid,
    depuis: Option<NaiveDate>,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<Result<ResultatInsertion, String>, EntDbError> {
    let mut avoirs =
        crate::finances_regime::regulariser_regime_annee(conn, etab, annee, eleve_id, None, actor, causalite).await?;
    Ok(facturer_annee_lignes(conn, etab, annee, eleve_id, depuis, None, actor, causalite)
        .await?
        .map(|r| {
            avoirs.ajouter(r);
            avoirs
        }))
}

/// Calcule puis insère les lignes annuelles (sans régularisation) ; `categories` restreint aux catégories citées
/// (réaction au changement de régime : catégories gagnées). Clés `annuel:{année}`, les mêmes dans tous les cas.
// 8 arguments : connexion, contexte (établissement, année, élève, date), filtre de catégories, acteur et causalité.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn facturer_annee_lignes(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve_id: Uuid,
    depuis: Option<NaiveDate>,
    categories: Option<&[&str]>,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<Result<ResultatInsertion, String>, EntDbError> {
    let prefixe = format!("annuel:{annee}");
    let lignes = match lignes_annuelles(conn, etab, annee, eleve_id, depuis, categories, &prefixe).await? {
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
                periode: None,
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
                // période propre : les jours de l'absence sur la ligne (M-5, imputation mois par mois)
                periode: l
                    .periode_debut
                    .zip(l.periode_fin)
                    .map(|(ld, lf)| (debut.max(ld), fin.min(lf)))
                    .filter(|(a, b)| a <= b),
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
        self.fin_exiger(etab, actor, role, Action::FacturationParametrer)
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
            p.crediteur_nom = opt_texte(&b.crediteur_nom, 70)?;
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
            p.crediteur_bic = opt_texte(&b.crediteur_bic, 11)?.map(|s| s.to_ascii_uppercase());
        }
        if b.crediteur_ics.is_some() {
            p.crediteur_ics = opt_texte(&b.crediteur_ics, 35)?.map(|s| s.to_ascii_uppercase());
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
            if let Some(v) = opt_texte(src, 20)? {
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
            let evt = publier_finance(
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "parametres.modifier", "parametres").detail(
                if crediteur_change {
                    "paramètres de facturation modifiés, dont le compte créancier (valeurs non journalisées)"
                } else {
                    "paramètres de facturation modifiés"
                },
            ),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_PARAMETRES_MODIFIES,
            AggregateKind::Facture,
            etab,
            None,
            json!({ "crediteur_modifie": crediteur_change }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
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
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
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
        let compte = opt_texte(&b.compte_produit, 20)?.unwrap_or_else(|| "706000".into());
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
                if crate::facturation_grilles::exiger_qualification_tx(&mut tx, etab).await? {
                    return Err(EntDbError::Refus(
                        crate::service::RefusMetier::Conflit,
                        "la qualification des articles est exigée : créer l'article depuis un modèle (nature légale, TVA)",
                    ));
                }
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "article.enregistrer", "article")
                .objet(id)
                .detail(&format!("article {code}")),
        )
        .await?;
        publier_finance(
            &mut tx,
            etab,
            types::FACTURATION_ARTICLE_ENREGISTRE,
            AggregateKind::Facture,
            id,
            None,
            json!({ "article_id": id }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
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
        a_la_date: Option<NaiveDate>,
    ) -> Result<Vec<TarifDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows = tarifs_tx(&mut tx, etab, annee.as_deref(), None, a_la_date).await?;
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
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let annee = annee_valide(&b.annee_scolaire)?;
        let regime = opt_texte(&b.regime, 20)?;
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
        if crate::facturation_grilles::annee_a_grille_votee_tx(&mut tx, etab, &annee).await? {
            return Err(EntDbError::Refus(
                crate::service::RefusMetier::Conflit,
                "l'année a une grille votée : passer par une version de grille (R-F10)",
            ));
        }
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
        .bind(opt_texte(&b.niveau, 20)?)
        .bind(&regime)
        .bind(b.qf_min_centimes)
        .bind(b.qf_max_centimes)
        .bind(b.montant_annuel_centimes)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier_finance(
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "tarif.definir", "tarif")
                .objet(id)
                .montant(b.montant_annuel_centimes)
                .detail(&format!("tarif annuel de l'année {annee}")),
        )
        .await?;
        let dto = tarifs_tx(&mut tx, etab, None, Some(id), None)
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
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        // Désactivation : les lignes déjà facturées restent liées à leur article.
        let r = sqlx::query(
            "UPDATE ent_fin_tarif SET actif = false
              WHERE etablissement_id = $1 AND id = $2 AND actif AND grille_version_id IS NULL",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "tarif.desactiver", "tarif").objet(id),
        )
        .await?;
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
        self.fin_exiger(etab, actor, role, Action::FacturationTarifs)
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "reduction.creer", "reduction")
                .objet(id)
                .detail(&format!("réduction {code} ({})", b.nature)),
        )
        .await?;
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
        self.fin_exiger(etab, actor, role, Action::FacturationReductions)
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
        publier_finance(
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "reduction.accorder", "reduction")
                .objet(reduction)
                .eleve(eleve),
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "bareme_bourse.poser", "bareme")
                .objet(id)
                .montant(b.montant_annuel_centimes)
                .detail(&format!("échelon {echelon}")),
        )
        .await?;
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
        self.fin_exiger(etab, actor, role, Action::FacturationParametrer)
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "echeancier_type.poser", "echeancier_type")
                .detail(&format!("calendrier type de l'année {annee} : {} échéances", b.dates.len())),
        )
        .await?;
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
        for (eleve_id, prenom, nom, _) in eleves {
            // même date d'entrée que la réaction au changement de régime (première inscription validée de l'année)
            let depuis = crate::finances_regime::depuis_facturation(&mut tx, etab, &annee, eleve_id).await?;
            match facturer_annee_eleve(
                &mut tx,
                etab,
                &annee,
                eleve_id,
                depuis,
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
        publier_finance(
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "generation.annuelle", "generation")
                .montant(total.montant)
                .detail(&format!(
                    "année {annee} : {} lignes créées, {} pièces touchées",
                    total.creees,
                    total.factures.len()
                )),
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
        // F2 (R-F4) : le payeur de la catégorie à la date de la prestation, jamais « le responsable 1 ».
        let date_prestation = b.periode_debut.unwrap_or_else(aujourd_hui);
        let Some(lus) = payeurs_pour_facturer(
            &mut tx,
            etab,
            b.eleve_id,
            categorie_de_designation(&categorie),
            date_prestation,
        )
        .await?
        else {
            // La tâche « payeur manquant » est conservée (on valide la transaction avant de refuser).
            tx.commit().await.map_err(db)?;
            return Err(conflit(
                "aucun payeur défini pour cette catégorie à cette date (tâche ouverte pour le secrétariat)",
            ));
        };
        let parts: Vec<(Uuid, i64)> = lus.rang1().map(|p| (p.foyer_id, p.part_bps)).collect();
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "consommation.facturer", "consommation")
                .eleve(b.eleve_id)
                .montant(total)
                .detail(&format!("source {source}")),
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
        self.fin_exiger(etab, actor, role, Action::FacturationEmettre)
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
            if f.type_piece == "facture"
                && crate::facturation_grilles::exiger_qualification_tx(&mut tx, etab).await?
                && crate::facturation_grilles::articles_non_qualifies_de_piece_tx(&mut tx, etab, f.id).await? > 0
            {
                rapport.refus.push(refus(
                    "article non qualifié (nature légale et TVA) : qualifier le catalogue avant d'émettre (R-F9)",
                ));
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
                role,
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
        self.fin_exiger(etab, actor, role, Action::FacturationAvoirs)
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
                    periode: None,
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
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "avoir.creer", "avoir")
                .objet(avoir_id)
                .foyer(f.foyer_id)
                .motif(&motif)
                .detail("avoir en brouillon sur une pièce émise (à émettre)"),
        )
        .await?;
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
            opt_texte(&b.motif, 160)?.as_deref(),
            Some(actor),
        )
        .await?;
        let c = compte_tx(&mut tx, etab, foyer, true).await?;
        journaliser_tx(
            &mut tx,
            &Acte::new(etab, Some(actor), role, Chemin::Finances, "mode_paiement.modifier", "compte")
                .foyer(foyer)
                .detail(&format!("mode de paiement : {}", b.mode))
                .motif(b.motif.as_deref().unwrap_or("")),
        )
        .await?;
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
        // F2 (FD-15) : un élève majeur payeur voit le compte de sa fiche « personne ».
        let mut foyers: Vec<Uuid> = foyers.into_iter().map(|f| f.0).collect();
        foyers.extend(
            crate::facturation_comptes::foyers_eleve_majeur_payeur(&mut tx, etab, user).await?,
        );
        foyers.sort();
        foyers.dedup();
        let mut comptes = Vec::new();
        for f in foyers {
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
        let evt = publier_finance(
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
        .bind(opt_texte(&b.motif, 200)?)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let evt = publier_finance(
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
    a_la_date: Option<NaiveDate>,
) -> Result<Vec<TarifDto>, EntDbError> {
    let sql = format!(
        "SELECT t.id, t.annee_scolaire, t.article_id, a.code, a.libelle, a.categorie, t.niveau, t.regime,
                t.qf_min_centimes, t.qf_max_centimes, t.montant_annuel_centimes
           FROM ent_fin_tarif t
           JOIN ent_fin_article a ON a.id = t.article_id AND a.etablissement_id = t.etablissement_id
          WHERE t.etablissement_id = $1 AND t.actif AND ($2::TEXT IS NULL OR t.annee_scolaire = $2)
            AND ($3::UUID IS NULL OR t.id = $3)
            AND {}
          ORDER BY t.annee_scolaire DESC, a.categorie, a.code, t.niveau NULLS FIRST, t.regime NULLS FIRST,
                   t.qf_min_centimes NULLS FIRST",
        sql_tarif_applicable("COALESCE($4::DATE, ent_aujourdhui_etab($1))")
    );
    let rows: Vec<TarifDtoRow> = sqlx::query_as(&sql)
    .bind(etab)
    .bind(annee)
    .bind(id)
    .bind(a_la_date)
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
    casquette: &str,
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
        let evt = publier_finance(
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
        let evt = publier_finance(
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
    let (action, objet) = if f.type_piece == "avoir" {
        ("avoir.emettre", "avoir")
    } else {
        ("facture.emettre", "facture")
    };
    let mut acte = Acte::new(
        etab,
        actor,
        casquette,
        Chemin::Finances,
        action,
        objet,
    )
    .objet(f.id)
    .foyer(f.foyer_id)
    .montant(f.total_centimes)
    .detail(&format!("pièce {numero}"));
    if casquette == "systeme" {
        acte = Acte::systeme(etab, Chemin::Systeme, action, objet)
            .objet(f.id)
            .foyer(f.foyer_id)
            .montant(f.total_centimes)
            .detail(&format!("pièce {numero}"));
    }
    journaliser_tx(conn, &acte).await?;
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

/// Palier des faibles de facturation (C3 et F-7) : `completer_plafonnee` en fonction pure.
#[cfg(test)]
mod tests_faibles_fac_plafonnee {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    /// Demi-pension 450 € dus, plafond 400 € épuisé : organisme 400 € et famille 20 € déjà posés (dû d'avant : 420 €).
    fn cas(avec_suivant: bool, complements_poses: bool) -> (Vec<LigneAFacturer>, i64) {
        cas_du(avec_suivant, complements_poses, false, 45_000)
    }

    /// `septembre_credite` : septembre découvert par un avoir de régime (mois manquant de la passe).
    fn cas_du(avec_suivant: bool, complements_poses: bool, septembre_credite: bool, du: i64) -> (Vec<LigneAFacturer>, i64) {
        let (art, fo, ff) = (Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));
        let pre = format!("annuel:2030-2031:{}:{art}", Uuid::from_u128(4));
        let periode = (d(2030, 9, 1), d(2031, 6, 30));
        let mois: std::collections::BTreeSet<i32> = (indice_mois(periode.0)..=indice_mois(periode.1)).collect();
        let nets: std::collections::BTreeSet<i32> =
            mois.iter().copied().filter(|m| !(septembre_credite && *m == indice_mois(periode.0))).collect();
        let ligne = |foyer: Uuid, cle: String, unitaire: i64| LigneAFacturer {
            foyer_id: foyer,
            eleve_id: None,
            article_id: Some(art),
            categorie: "restauration".into(),
            libelle: "Demi-pension".into(),
            quantite: 1,
            unitaire,
            source_cle: cle,
            periode: Some(periode),
        };
        let principal = ligne(fo, format!("{pre}:{fo}"), 40_000);
        let cle_suivant = format!("{pre}:suivant:{fo}:{ff}");
        let posee = |cle: String, montant: i64| {
            let (groupe, queue) = groupe_de_cle(&cle, &pre).unwrap();
            LignePosee { cle, groupe, queue, periode: Some(periode), montant, nets: nets.clone() }
        };
        let suivant_pose = if septembre_credite { 5_000 } else { 2_000 };
        let mut posees = vec![posee(principal.source_cle.clone(), 40_000), posee(cle_suivant.clone(), suivant_pose)];
        if complements_poses {
            for m in mois.iter() {
                let cle = format!("{pre}:c{}:suivant:{fo}:{ff}", premier_du_mois(*m).format("%Y-%m"));
                posees.push(posee(cle, 0));
            }
        }
        let memes: Vec<&LignePosee> = posees.iter().collect();
        let suivants = if avec_suivant { vec![ligne(ff, cle_suivant, 5_000)] } else { Vec::new() };
        let (sortie, _signal, non_porte) = completer_plafonnee(
            &pre,
            art,
            &principal,
            suivants,
            du,
            &memes,
            &nets,
            &Default::default(),
            true,
            &HashMap::new(),
        )
        .unwrap();
        (sortie, non_porte)
    }

    #[test]
    fn c3_manque_du_payeur_suivant_deja_pose_sous_une_cle_de_complement() {
        let (sortie, non_porte) = cas(true, false);
        let complement: Vec<&LigneAFacturer> = sortie.iter().filter(|l| l.source_cle.contains(":c2030-09:suivant:")).collect();
        assert_eq!(complement.len(), 1, "{sortie:?}");
        assert_eq!(complement[0].unitaire, 3_000, "30 € de hausse à la famille");
        assert_eq!(non_porte, 0, "tout est posé ou insérable");
    }

    /// Contre-revue M-1 : septembre crédité (régime) puis redevenu dû, et dû passé à 500 € : le complément des mois
    /// manquants du payeur suivant prend `:c2030-09` ; le manque (45 €) ne doit pas reprendre la même clé.
    #[test]
    fn m1_complement_du_manque_distinct_du_complement_des_mois_manquants() {
        let (sortie, non_porte) = cas_du(true, false, true, 50_000);
        let cles: Vec<&str> = sortie.iter().map(|l| l.source_cle.as_str()).collect();
        let mut uniques = cles.clone();
        uniques.sort();
        uniques.dedup();
        assert_eq!(uniques.len(), cles.len(), "aucune clé deux fois dans la passe : {cles:?}");
        let famille: i64 = sortie.iter().filter(|l| l.source_cle.contains(":suivant:")).map(|l| l.unitaire).sum();
        assert_eq!(famille, 1_000 + 4_500, "10 € de septembre et 45 € de manque");
        assert_eq!(non_porte, 0);
    }

    #[test]
    fn f7_non_porte_rendu_par_la_garde() {
        // sans payeur suivant : les 30 € ne sont portés par personne
        assert_eq!(cas(false, false).1, 3_000);
        // toutes les clés de complément déjà prises : rien d'insérable, non porté
        let (sortie, non_porte) = cas(true, true);
        assert!(sortie.iter().all(|l| !l.source_cle.contains(":suivant:")), "{sortie:?}");
        assert_eq!(non_porte, 3_000);
    }
}
