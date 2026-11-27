//! Socle 24, lot F2 — **payeurs** : port `payeurs(élève, catégorie, date)` du socle 4 (adaptateur),
//! désignations datées, découpage par date de désignation, conséquences d'un changement de payeur.
//!
//! # Le port
//!
//! Le socle 4 (§8.9) possédera `ent_eleve_payeur` (lot « payeurs », migration 268). Tant qu'il n'existe
//! pas, la facturation lit à travers [`payeurs_a`] **par adaptateur** :
//!
//! 1. les désignations **opposables** (`ent_fin_designation.statut = 'effective'`) en vigueur à la date,
//!    la catégorie précise l'emportant sur `toutes` ;
//! 2. à défaut, les liens payeurs actuels (`ent_eleve_lien.est_payeur` / `part_paiement`, MIG-5) :
//!    mêmes résultats que le chemin S2-9, parts exprimées en points de base.
//!
//! Le jour où le socle 4 livre sa table, seule cette lecture change : le reste du lot ne connaît que
//! [`PartPayeur`]. Le 24 n'écrit jamais dans le 4 : les écritures de ce fichier visent les tables du 24.
//!
//! # Règles (24_FINANCES.md)
//!
//! * R-F4 : le payeur est lu **à la date de la prestation**, jamais déduit du foyer ou du rang du responsable ;
//!   une catégorie qui n'a pas 100 % de payeurs n'est **pas** facturée : tâche « payeur manquant » (FAC-210).
//! * R-F30 : une désignation exige l'**acceptation du payeur désigné** ou une **pièce** (jugement, convention
//!   homologuée, accord des deux). La demande d'un seul parent crée une tâche (CF-75).
//! * §9.2 : un changement de payeur ne touche **aucune pièce émise** ; il produit des conséquences
//!   **proposées** (avoir au sortant, ligne à l'entrant), validées puis émises par la comptabilité.
//! * Parts en points de base (10 000 = 100 %), montants en centimes entiers, arrondis en faveur de la
//!   famille (avoir au centime supérieur, facture à l'inférieur), somme exacte au plus fort reste.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::facturation_service::{journaliser_tx, Acte, Chemin};
use crate::finances::{conflit, db, introuvable, invalide, nb_mois_entre, publier_finance};
use crate::finances_calcul as calcul;
use crate::outbox::Causalite;
use crate::service::EntDbError;
use ent_events::{types, AggregateKind};
use serde_json::json;

/// Catégories de désignation (colonne `ent_fin_designation.categorie`).
pub const CATEGORIES_DESIGNATION: [&str; 8] = [
    "toutes",
    "scolarite",
    "restauration",
    "internat",
    "activite",
    "frais",
    "voyages",
    "fournitures",
];

/// Catégorie de désignation d'un article ou d'une ligne (`bourse` et `reduction` suivent leur article ;
/// une catégorie inconnue retombe sur `toutes`).
pub fn categorie_de_designation(categorie_article: &str) -> &'static str {
    CATEGORIES_DESIGNATION
        .iter()
        .copied()
        .find(|c| *c != "toutes" && *c == categorie_article)
        .unwrap_or("toutes")
}

pub const PART_ENTIERE: i64 = 10_000;

// ---------------------------------------------------------------------------
// Types du port
// ---------------------------------------------------------------------------

/// Une part de payeur lue à une date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartPayeur {
    pub compte_id: Uuid,
    pub foyer_id: Uuid,
    pub libelle: String,
    pub type_debiteur: String,
    /// Points de base (10 000 = 100 %).
    pub part_bps: i64,
    /// 1 = payeur principal ; 2 = payeur suivant (reçoit ce qu'un plafond d'organisme ne couvre pas).
    pub rang: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origine {
    /// Désignations datées de la facturation (adaptateur du port).
    Designation,
    /// Liens payeurs actuels (`ent_eleve_lien`), faute de désignation (MIG-5).
    Liens,
    /// Aucun payeur.
    Aucune,
}

impl Origine {
    pub fn code(self) -> &'static str {
        match self {
            Self::Designation => "designation",
            Self::Liens => "liens",
            Self::Aucune => "aucune",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayeursLus {
    pub parts: Vec<PartPayeur>,
    pub origine: Origine,
    /// Catégorie réellement lue : celle demandée si une désignation précise existe, sinon `toutes`.
    pub categorie_lue: String,
}

impl PayeursLus {
    pub fn vide(categorie: &str) -> Self {
        Self {
            parts: Vec::new(),
            origine: Origine::Aucune,
            categorie_lue: categorie.to_string(),
        }
    }

    /// Somme des parts d'un rang, en points de base.
    pub fn somme(&self, rang: i16) -> i64 {
        somme_rang(&self.parts, rang)
    }

    /// Une répartition est complète quand le rang 1 vaut exactement 100 % (et le rang 2, s'il existe).
    /// Sinon la catégorie n'est **pas facturée** et une tâche « payeur manquant » est ouverte.
    pub fn complete(&self) -> bool {
        self.somme(1) == PART_ENTIERE && (self.somme(2) == 0 || self.somme(2) == PART_ENTIERE)
    }

    pub fn rang1(&self) -> impl Iterator<Item = &PartPayeur> {
        self.parts.iter().filter(|p| p.rang == 1)
    }

    pub fn rang2(&self) -> impl Iterator<Item = &PartPayeur> {
        self.parts.iter().filter(|p| p.rang == 2)
    }
}

pub fn somme_rang(parts: &[PartPayeur], rang: i16) -> i64 {
    parts
        .iter()
        .filter(|p| p.rang == rang)
        .map(|p| p.part_bps)
        .sum()
}

// ---------------------------------------------------------------------------
// Calcul pur : validation, découpage par date, plafond, conséquences
// ---------------------------------------------------------------------------

/// Valide une proposition de répartition : (compte, part en points de base, rang).
/// Rang 1 obligatoire et somme 10 000 ; rang 2 facultatif (somme 10 000 s'il existe) ; chaque compte
/// une seule fois ; chaque part entre 1 et 10 000.
pub fn valider_repartition(parts: &[(Uuid, i64, i16)]) -> Result<(), &'static str> {
    if parts.is_empty() {
        return Err("au moins un payeur");
    }
    // F7 : borne (une famille recomposée n'a pas plus de huit payeurs ; protège les boucles et les tâches).
    if parts.len() > 8 {
        return Err("huit payeurs au plus par répartition");
    }
    let mut vus = std::collections::BTreeSet::new();
    for (compte, bps, rang) in parts {
        if !vus.insert(*compte) {
            return Err("un payeur ne figure qu'une fois dans une répartition");
        }
        if !(1..=PART_ENTIERE).contains(bps) {
            return Err("chaque part est comprise entre 0,01 % et 100 %");
        }
        if !matches!(rang, 1 | 2) {
            return Err("rang 1 (principal) ou 2 (suivant)");
        }
    }
    let somme = |r: i16| -> i64 {
        parts
            .iter()
            .filter(|p| p.2 == r)
            .map(|p| p.1)
            .sum::<i64>()
    };
    if somme(1) != PART_ENTIERE {
        return Err("la somme des parts du payeur principal doit valoir exactement 100 %");
    }
    if somme(2) != 0 && somme(2) != PART_ENTIERE {
        return Err("la somme des parts des payeurs suivants doit valoir exactement 100 %");
    }
    Ok(())
}

/// Un intervalle de dates (inclusif) où la même répartition est en vigueur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tranche {
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// (compte, points de base) du rang 1 ; vide = aucun payeur sur cette tranche.
    pub parts: Vec<(Uuid, i64)>,
}

/// Résultat d'un découpage : ce que doit chaque compte, et ce qui n'a pas de payeur.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Decoupage {
    pub parts: Vec<(Uuid, i64)>,
    pub non_attribue: i64,
}

/// **Découpage par date de désignation** (§9.2) : une pièce couvrant [`du`, `au`] est répartie entre les
/// tranches au prorata des **mois** (comme tout le moteur : mois commencé dû à celui qui l'a commencé),
/// puis, dans chaque tranche, entre les payeurs au prorata de leurs parts. La somme des morceaux est
/// **exactement** `montant` (plus fort reste, deux fois). Une tranche sans payeur laisse sa part dans
/// `non_attribue` (la catégorie n'y est pas facturée).
///
/// Les tranches sont rognées à la période ; une tranche hors période est ignorée.
pub fn decouper(du: NaiveDate, au: NaiveDate, montant: i64, tranches: &[Tranche]) -> Decoupage {
    let mut utiles: Vec<(i64, &Tranche)> = Vec::new();
    for t in tranches {
        let debut = t.du.max(du);
        let fin = t.au.min(au);
        if fin < debut {
            continue;
        }
        // Mois commencés dans la période de la tranche, sans compter deux fois le mois d'une frontière.
        utiles.push((nb_mois_entre(debut, fin).max(1), t));
    }
    if utiles.is_empty() {
        return Decoupage {
            parts: Vec::new(),
            non_attribue: montant,
        };
    }
    let poids: Vec<(usize, i64)> = utiles.iter().enumerate().map(|(i, (m, _))| (i, *m)).collect();
    let par_tranche = calcul::repartir(montant, &poids);
    let mut cumul: BTreeMap<Uuid, i64> = BTreeMap::new();
    let mut non_attribue = 0;
    for ((_, t), (_, m)) in utiles.iter().zip(par_tranche) {
        if t.parts.is_empty() {
            non_attribue += m;
            continue;
        }
        for (compte, c) in calcul::repartir(m, &t.parts) {
            *cumul.entry(compte).or_insert(0) += c;
        }
    }
    Decoupage {
        parts: cumul.into_iter().collect(),
        non_attribue,
    }
}

/// Plafonnement d'une prise en charge (CF-82) : `montants` = ce que chaque compte devrait payer ;
/// `plafonds` = ce qu'un organisme peut encore couvrir (centimes). La part non couverte est reversée aux
/// payeurs **suivants** (rang 2, au prorata de leurs parts) ; sans payeur suivant, elle est rendue en
/// `non_couvert` (tâche : jamais reportée sur la famille sans décision, CF-83).
pub fn plafonner(
    montants: &[(Uuid, i64)],
    plafonds: &BTreeMap<Uuid, i64>,
    suivants: &[(Uuid, i64)],
) -> (Vec<(Uuid, i64)>, i64) {
    let mut sortie: BTreeMap<Uuid, i64> = BTreeMap::new();
    let mut reste = 0;
    for (compte, m) in montants {
        let m = (*m).max(0);
        match plafonds.get(compte) {
            Some(p) if m > (*p).max(0) => {
                sortie.insert(*compte, (*p).max(0));
                reste += m - (*p).max(0);
            }
            _ => {
                sortie.insert(*compte, m);
            }
        }
    }
    if reste == 0 {
        return (sortie.into_iter().collect(), 0);
    }
    if suivants.is_empty() {
        return (sortie.into_iter().collect(), reste);
    }
    for (compte, m) in calcul::repartir(reste, suivants) {
        *sortie.entry(compte).or_insert(0) += m;
    }
    (sortie.into_iter().collect(), 0)
}

/// Mois d'une ligne qui changent de payeur : l'ancien payeur **garde** les mois jusqu'à celui de la
/// veille de la date d'effet inclus (mois commencé dû à celui qui l'a commencé), le nouveau reçoit les
/// suivants. Renvoie (mois de la ligne, mois transférés).
pub fn mois_transferes(debut: NaiveDate, fin: NaiveDate, date_effet: NaiveDate) -> (i64, i64) {
    let total = nb_mois_entre(debut, fin).max(1);
    let veille = date_effet - chrono::Duration::days(1);
    let gardes = if veille < debut {
        0
    } else {
        nb_mois_entre(debut, veille).clamp(0, total)
    };
    (total, total - gardes)
}

/// Quotient `montant × num / (den × base)` arrondi **au supérieur** (avoir : au centime supérieur, en
/// faveur de la famille). Calcul en 128 bits.
pub fn proportion_sup(montant: i64, num: i64, den: i64) -> i64 {
    if den <= 0 || montant <= 0 || num <= 0 {
        return 0;
    }
    let n = montant as i128 * num as i128;
    ((n + den as i128 - 1) / den as i128) as i64
}

/// Une conséquence proposée pour une ligne de pièce émise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsequenceCalculee {
    pub ligne_id: Uuid,
    pub facture_id: Uuid,
    pub compte_sortant: Uuid,
    pub mois_total: i64,
    pub mois_apres: i64,
    /// Avoir proposé au payeur sortant (centimes, ≥ 0).
    pub credit: i64,
    /// Ce qui revient à chaque payeur entrant (centimes, ≥ 0).
    pub debits: Vec<(Uuid, i64)>,
}

/// Calcule la conséquence d'un changement de payeur sur **une** ligne émise (§9.2).
///
/// * `avant` / `apres` : (compte, points de base) du rang 1 à la date d'effet, avant et après le changement ;
/// * le compte de la ligne est le payeur sortant : s'il garde toute sa part, il n'y a rien ;
/// * le crédit est la part libérée des mois transférés, **au centime supérieur** ; les débits répartissent
///   ce même montant (à l'inférieur, plus fort reste) entre ceux dont la part augmente. L'écart d'arrondi
///   est d'un centime au plus et **jamais en défaveur de la famille**.
///
/// Renvoie `None` quand la ligne n'est pas concernée (compte absent de l'ancienne répartition, part
/// inchangée ou en hausse, aucun mois transféré).
pub fn consequence_ligne(
    ligne_id: Uuid,
    facture_id: Uuid,
    compte_ligne: Uuid,
    total_ligne: i64,
    periode: (NaiveDate, NaiveDate),
    date_effet: NaiveDate,
    avant: &[(Uuid, i64)],
    apres: &[(Uuid, i64)],
) -> Option<ConsequenceCalculee> {
    let ancienne = avant.iter().find(|p| p.0 == compte_ligne)?.1;
    let nouvelle = apres
        .iter()
        .find(|p| p.0 == compte_ligne)
        .map(|p| p.1)
        .unwrap_or(0);
    if nouvelle >= ancienne || ancienne <= 0 {
        return None;
    }
    let (mois_total, mois_apres) = mois_transferes(periode.0, periode.1, date_effet);
    if mois_apres == 0 || total_ligne == 0 {
        return None;
    }
    let liberee = ancienne - nouvelle;
    // |montant| × mois_apres × liberee / (mois_total × ancienne) : le signe d'une ligne de réduction suit.
    let signe = if total_ligne < 0 { -1 } else { 1 };
    let abs = total_ligne.abs();
    let den = mois_total as i128 * ancienne as i128;
    let num = abs as i128 * mois_apres as i128 * liberee as i128;
    let credit = ((num + den - 1) / den) as i64;
    let base_plancher = (num / den) as i64;
    let hausses: Vec<(Uuid, i64)> = apres
        .iter()
        .filter(|(c, p)| {
            *c != compte_ligne && *p > avant.iter().find(|a| a.0 == *c).map(|a| a.1).unwrap_or(0)
        })
        .map(|(c, p)| (*c, *p - avant.iter().find(|a| a.0 == *c).map(|a| a.1).unwrap_or(0)))
        .collect();
    let debits: Vec<(Uuid, i64)> = calcul::repartir(base_plancher, &hausses)
        .into_iter()
        .map(|(c, m)| (c, signe * m))
        .collect();
    Some(ConsequenceCalculee {
        ligne_id,
        facture_id,
        compte_sortant: compte_ligne,
        mois_total,
        mois_apres,
        credit: signe * credit,
        debits,
    })
}

// ---------------------------------------------------------------------------
// Lecture : le port
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow, Clone, Debug)]
struct DesignationLue {
    categorie: String,
    compte_id: Uuid,
    foyer_id: Uuid,
    libelle: String,
    type_debiteur: String,
    part_bps: i32,
    rang: i16,
    du: NaiveDate,
    au: Option<NaiveDate>,
}

async fn designations_opposables(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    categorie: &str,
    du: NaiveDate,
    au: Option<NaiveDate>,
) -> Result<Vec<DesignationLue>, EntDbError> {
    sqlx::query_as::<_, DesignationLue>(
        "SELECT d.categorie, d.compte_id, c.foyer_id, c.libelle_facturation AS libelle, c.type_debiteur,
                d.part_bps, d.rang, d.du, d.au
           FROM ent_fin_designation d
           JOIN ent_fin_compte c ON c.etablissement_id = d.etablissement_id AND c.id = d.compte_id
          WHERE d.etablissement_id = $1 AND d.eleve_id = $2 AND d.statut = 'effective'
            AND d.categorie IN ($3, 'toutes')
            AND d.du <= COALESCE($5, 'infinity'::date) AND (d.au IS NULL OR d.au >= $4)
          ORDER BY d.du, d.rang, d.part_bps DESC, d.compte_id",
    )
    .bind(etab)
    .bind(eleve)
    .bind(categorie)
    .bind(du)
    .bind(au)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

fn en_vigueur(d: &DesignationLue, date: NaiveDate) -> bool {
    d.du <= date && d.au.is_none_or(|a| a >= date)
}

/// Choix en mémoire à une date : la catégorie précise l'emporte sur `toutes`.
fn choisir(rows: &[DesignationLue], categorie: &str, date: NaiveDate) -> Vec<PartPayeur> {
    let a_date: Vec<&DesignationLue> = rows.iter().filter(|d| en_vigueur(d, date)).collect();
    let precises: Vec<&&DesignationLue> = a_date.iter().filter(|d| d.categorie == categorie).collect();
    let retenues: Vec<&DesignationLue> = if categorie != "toutes" && !precises.is_empty() {
        precises.into_iter().copied().collect()
    } else {
        a_date.into_iter().filter(|d| d.categorie == "toutes").collect()
    };
    retenues
        .into_iter()
        .map(|d| PartPayeur {
            compte_id: d.compte_id,
            foyer_id: d.foyer_id,
            libelle: d.libelle.clone(),
            type_debiteur: d.type_debiteur.clone(),
            part_bps: i64::from(d.part_bps),
            rang: d.rang,
        })
        .collect()
}

/// Adaptateur MIG-5 : liens payeurs actuels (foyer, part en %) convertis en points de base **sans
/// renormaliser** (les poids relatifs du chemin S2-9 sont conservés au centime) ; comptes lus ou créés.
async fn payeurs_des_liens(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    creer_comptes: bool,
) -> Result<Vec<PartPayeur>, EntDbError> {
    let foyers = crate::finances::payeurs_liens(conn, etab, eleve).await?;
    let mut out = Vec::new();
    for (foyer, part_pct) in foyers {
        let compte = if creer_comptes {
            Some(compte_du_foyer(conn, etab, foyer).await?)
        } else {
            compte_existant_du_foyer(conn, etab, foyer).await?
        };
        let Some(compte) = compte else { continue };
        let (libelle, type_debiteur): (String, String) = sqlx::query_as(
            "SELECT libelle_facturation, type_debiteur FROM ent_fin_compte
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(compte)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        out.push(PartPayeur {
            compte_id: compte,
            foyer_id: foyer,
            libelle,
            type_debiteur,
            part_bps: part_pct * 100,
            rang: 1,
        });
    }
    // M9 : parts renormalisées à 100 % (un seul payeur à NULL vaut 100 % ; des parts qui ne font pas 100 sont
    // ramenées à 10 000 points de base, plus fort reste, comme l'ancien chemin) : jamais de répartition fausse.
    let total: i64 = out.iter().map(|p| p.part_bps).sum();
    if !out.is_empty() && total != PART_ENTIERE {
        let poids: Vec<(Uuid, i64)> = out.iter().map(|p| (p.compte_id, p.part_bps.max(1))).collect();
        for (p, (_, bps)) in out.iter_mut().zip(calcul::repartir(PART_ENTIERE, &poids)) {
            p.part_bps = bps;
        }
    }
    Ok(out)
}

/// Compte du foyer (le crée au besoin : une seule implémentation, la fonction SQL `ent_fin_compte_du_foyer`).
pub(crate) async fn compte_du_foyer(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
) -> Result<Uuid, EntDbError> {
    let (id,): (Uuid,) = sqlx::query_as("SELECT ent_fin_compte_du_foyer($1, $2, NULL, 'reprise')")
        .bind(etab)
        .bind(foyer)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    Ok(id)
}

pub(crate) async fn compte_existant_du_foyer(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    let r: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_fin_compte WHERE etablissement_id = $1 AND foyer_id = $2",
    )
    .bind(etab)
    .bind(foyer)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.map(|x| x.0))
}

/// **Le port** : qui paie `categorie` pour `eleve` à `date` ? `creer_comptes` : les chemins de
/// facturation créent le compte du foyer au besoin ; les lectures ne créent rien.
pub async fn payeurs_a(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    categorie: &str,
    date: NaiveDate,
    creer_comptes: bool,
) -> Result<PayeursLus, EntDbError> {
    let rows = designations_opposables(conn, etab, eleve, categorie, date, Some(date)).await?;
    let parts = choisir(&rows, categorie, date);
    if !parts.is_empty() {
        let lue = if categorie != "toutes" && rows.iter().any(|d| d.categorie == categorie && en_vigueur(d, date)) {
            categorie
        } else {
            "toutes"
        };
        return Ok(PayeursLus {
            parts,
            origine: Origine::Designation,
            categorie_lue: lue.to_string(),
        });
    }
    // Aucune désignation en vigueur à cette date : liens actuels, **seulement si l'élève n'a jamais eu de
    // désignation** (sinon une période sans payeur reste sans payeur, jamais « au responsable 1 »). Une désignation qui ne commence qu'après `date` ne change rien à
    // cette date : l'ancienne répartition (liens) reste en vigueur jusque-là.
    let (deja,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_fin_designation WHERE etablissement_id = $1 AND eleve_id = $2
                         AND statut = 'effective' AND du <= $3)",
    )
    .bind(etab)
    .bind(eleve)
    .bind(date)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if deja {
        return Ok(PayeursLus::vide(categorie));
    }
    let parts = payeurs_des_liens(conn, etab, eleve, creer_comptes).await?;
    if parts.is_empty() {
        return Ok(PayeursLus::vide(categorie));
    }
    Ok(PayeursLus {
        parts,
        origine: Origine::Liens,
        categorie_lue: "toutes".into(),
    })
}

/// Tranches de répartition d'une période (pour [`decouper`]) : un intervalle par changement de désignation.
pub async fn tranches(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    categorie: &str,
    du: NaiveDate,
    au: NaiveDate,
) -> Result<Vec<Tranche>, EntDbError> {
    let rows = designations_opposables(conn, etab, eleve, categorie, du, Some(au)).await?;
    // Points de rupture : début de période, début de chaque désignation, lendemain de chaque fin.
    let mut coupures = std::collections::BTreeSet::new();
    coupures.insert(du);
    for d in &rows {
        if d.du > du && d.du <= au {
            coupures.insert(d.du);
        }
        if let Some(a) = d.au {
            let lendemain = a + chrono::Duration::days(1);
            if lendemain > du && lendemain <= au {
                coupures.insert(lendemain);
            }
        }
    }
    let debuts: Vec<NaiveDate> = coupures.into_iter().collect();
    // Avant la première désignation de l'élève, la répartition est celle des liens (comme `payeurs_a`).
    let (premiere,): (Option<NaiveDate>,) = sqlx::query_as(
        "SELECT min(du) FROM ent_fin_designation
          WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'effective'",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let liens: Vec<(Uuid, i64)> = payeurs_des_liens(conn, etab, eleve, false)
        .await?
        .into_iter()
        .map(|p| (p.compte_id, p.part_bps))
        .collect();
    let mut out = Vec::new();
    for (i, debut) in debuts.iter().enumerate() {
        let fin = debuts
            .get(i + 1)
            .map(|d| *d - chrono::Duration::days(1))
            .unwrap_or(au);
        let parts: Vec<(Uuid, i64)> = choisir(&rows, categorie, *debut)
            .into_iter()
            .filter(|p| p.rang == 1)
            .map(|p| (p.compte_id, p.part_bps))
            .collect();
        let parts = if parts.is_empty() && premiere.is_none_or(|p| *debut < p) {
            liens.clone()
        } else {
            parts
        };
        let total: i64 = parts.iter().map(|p| p.1).sum();
        out.push(Tranche {
            du: *debut,
            au: fin,
            // Une répartition incomplète n'est pas facturée.
            parts: if total == PART_ENTIERE { parts } else { Vec::new() },
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tâches (FAC-210, FAC-218, FAC-257)
// ---------------------------------------------------------------------------

/// Ouvre (idempotent) une tâche pour le secrétariat ou la comptabilité. Renvoie `true` si créée.
pub(crate) async fn ouvrir_tache(
    conn: &mut PgConnection,
    etab: Uuid,
    nature: &str,
    eleve: Option<Uuid>,
    compte: Option<Uuid>,
    categorie: Option<&str>,
    depuis: NaiveDate,
    detail: &str,
    acteur: Option<Uuid>,
) -> Result<bool, EntDbError> {
    ouvrir_tache_evenement(conn, etab, nature, eleve, compte, categorie, depuis, detail, acteur, None).await
}

/// Comme `ouvrir_tache`, liée à un **événement** (M2, suites 3) : une tâche ouverte par (situation, événement).
pub(crate) async fn ouvrir_tache_evenement(
    conn: &mut PgConnection,
    etab: Uuid,
    nature: &str,
    eleve: Option<Uuid>,
    compte: Option<Uuid>,
    categorie: Option<&str>,
    depuis: NaiveDate,
    detail: &str,
    acteur: Option<Uuid>,
    evenement: Option<Uuid>,
) -> Result<bool, EntDbError> {
    let r = sqlx::query(
        "INSERT INTO ent_fin_tache (id, etablissement_id, nature, eleve_id, compte_id, categorie, depuis, detail, cree_par, evenement_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
         ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(nature)
    .bind(eleve)
    .bind(compte)
    .bind(categorie)
    .bind(depuis)
    .bind(detail.chars().take(300).collect::<String>())
    .bind(acteur)
    .bind(evenement)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let creee = r.rows_affected() == 1;
    if creee {
        publier_finance(
            conn,
            etab,
            types::FACTURATION_TACHE_PAYEUR,
            AggregateKind::Facture,
            eleve.unwrap_or(etab),
            eleve,
            json!({ "nature": nature, "categorie": categorie, "compte_id": compte }),
            acteur,
            Causalite::default(),
        )
        .await?;
    }
    Ok(creee)
}

/// Résout la tâche ouverte d'une nature liée à **cet** événement (M2) : les tâches des autres événements du même
/// élève restent ouvertes.
pub(crate) async fn resoudre_taches_evenement(
    conn: &mut PgConnection,
    etab: Uuid,
    nature: &str,
    eleve: Uuid,
    evenement: Uuid,
    par: Option<Uuid>,
) -> Result<u64, EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_fin_tache SET statut = 'resolue', resolue_le = now(), resolue_par = $5
          WHERE etablissement_id = $1 AND nature = $2 AND eleve_id = $3 AND evenement_id = $4 AND statut = 'ouverte'",
    )
    .bind(etab)
    .bind(nature)
    .bind(eleve)
    .bind(evenement)
    .bind(par)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.rows_affected())
}

/// Résout les tâches ouvertes d'une nature **sans événement** pour un élève (régime mois par mois : la tâche « régime
/// inconnu » est close quand plus aucun mois inconnu n'est à poser) ; les tâches liées à un événement restent ouvertes.
pub(crate) async fn resoudre_taches_sans_evenement(
    conn: &mut PgConnection,
    etab: Uuid,
    nature: &str,
    eleve: Uuid,
) -> Result<u64, EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_fin_tache SET statut = 'resolue', resolue_le = now()
          WHERE etablissement_id = $1 AND nature = $2 AND eleve_id = $3 AND statut = 'ouverte'
            AND evenement_id IS NULL",
    )
    .bind(etab)
    .bind(nature)
    .bind(eleve)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.rows_affected())
}

/// Résout les tâches ouvertes d'une nature pour (élève, catégorie) dont la date `depuis` tombe dans `du..=au` (suites 4,
/// M3) : la part non couverte d'un tronçon est désormais facturée (prise renouvelée ou prolongée, payeur suivant désigné).
/// Les tâches des autres périodes de l'élève restent ouvertes, ainsi que les signalements de plafond modifié ou non
/// consommé (« plafond de la prise en charge … ») : ils attendent une régularisation humaine, la garde les rouvrirait à
/// chaque rejeu (contre-revue du palier, M-2).
pub(crate) async fn resoudre_taches_periode(
    conn: &mut PgConnection,
    etab: Uuid,
    nature: &str,
    eleve: Uuid,
    categorie: &str,
    du: NaiveDate,
    au: NaiveDate,
) -> Result<u64, EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_fin_tache SET statut = 'resolue', resolue_le = now()
          WHERE etablissement_id = $1 AND nature = $2 AND eleve_id = $3 AND statut = 'ouverte'
            AND categorie = $4 AND depuis >= $5 AND depuis <= $6
            AND NOT starts_with(coalesce(detail, ''), 'plafond de la prise en charge')",
    )
    .bind(etab)
    .bind(nature)
    .bind(eleve)
    .bind(categorie)
    .bind(du)
    .bind(au)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.rows_affected())
}

/// Résout les tâches ouvertes d'une nature pour (élève, catégorie) : la situation est réglée.
pub(crate) async fn resoudre_taches(
    conn: &mut PgConnection,
    etab: Uuid,
    nature: &str,
    eleve: Uuid,
    categorie: Option<&str>,
    par: Option<Uuid>,
) -> Result<u64, EntDbError> {
    let r = sqlx::query(
        "UPDATE ent_fin_tache SET statut = 'resolue', resolue_le = now(), resolue_par = $5
          WHERE etablissement_id = $1 AND nature = $2 AND eleve_id = $3 AND statut = 'ouverte'
            AND ($4::text IS NULL OR categorie = $4)",
    )
    .bind(etab)
    .bind(nature)
    .bind(eleve)
    .bind(categorie)
    .bind(par)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(r.rows_affected())
}

/// Garde de facturation d'une catégorie (R-F4) : renvoie les parts à facturer, ou `None` quand la
/// catégorie n'a pas 100 % de payeurs **à la date** — alors une tâche « payeur manquant » est ouverte
/// (« Léo : restauration sans payeur depuis le 01/02 ») et rien n'est facturé « au responsable 1 ».
pub(crate) async fn payeurs_pour_facturer(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    categorie: &str,
    date: NaiveDate,
) -> Result<Option<PayeursLus>, EntDbError> {
    let lus = payeurs_a(conn, etab, eleve, categorie, date, true).await?;
    if lus.complete() {
        resoudre_taches(conn, etab, "payeur_manquant", eleve, Some(categorie), None).await?;
        return Ok(Some(lus));
    }
    ouvrir_tache(
        conn,
        etab,
        "payeur_manquant",
        Some(eleve),
        None,
        Some(categorie),
        date,
        "catégorie sans payeur (ou répartition ≠ 100 %) : rien n'est facturé tant que ce n'est pas réglé",
        None,
    )
    .await?;
    Ok(None)
}

// ---------------------------------------------------------------------------
// Désignation : proposer, accepter, activer (R-F30, R-F31)
// ---------------------------------------------------------------------------

/// Pièce de répartition (FAC-217) : jugement, convention homologuée, accord des deux.
#[derive(Clone, Debug)]
pub struct PieceRepartition {
    pub nature: String,
    pub reference: String,
    pub document_id: Option<Uuid>,
}

pub const NATURES_PIECE: [&str; 3] = ["jugement", "convention_homologuee", "accord_des_deux"];

#[derive(Clone, Debug)]
pub struct PropositionDesignation {
    pub eleve_id: Uuid,
    pub categorie: String,
    /// (compte, points de base, rang).
    pub parts: Vec<(Uuid, i64, i16)>,
    pub date_effet: NaiveDate,
    pub piece: Option<PieceRepartition>,
    pub motif: Option<String>,
    pub source: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultatDesignation {
    pub groupe_id: Uuid,
    /// `effective` (pièce fournie ou origine qui porte son consentement) ou `en_attente` (acceptation attendue).
    pub statut: &'static str,
    pub consequences: usize,
}

async fn verrou_eleve(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<(), EntDbError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("fin-designation:{etab}:{eleve}"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    Ok(())
}

/// M4 : une pièce de répartition est contrôlée. « jugement » et « convention homologuée » exigent un document
/// **existant dans l'établissement** ; « accord des deux » n'est qu'une déclaration : elle ne rend jamais la
/// répartition opposable à elle seule (les acceptations de **tous** les payeurs désignés sont exigées).
async fn valider_piece(conn: &mut PgConnection, etab: Uuid, piece: &PieceRepartition) -> Result<(), EntDbError> {
    if !NATURES_PIECE.contains(&piece.nature.as_str())
        || piece.reference.trim().is_empty()
        || piece.reference.chars().count() > 120
    {
        return Err(invalide(
            "pièce : jugement, convention homologuée ou accord des deux, avec sa référence",
        ));
    }
    if piece.nature != "accord_des_deux" {
        let Some(doc) = piece.document_id else {
            return Err(invalide("pièce : le document du jugement ou de la convention est obligatoire"));
        };
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_document WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(doc)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if !ok {
            return Err(introuvable());
        }
    }
    Ok(())
}

/// M4 : un compte désigné est **lié à l'élève** (un lien de l'élève dans son foyer, ou la fiche « personne »
/// de l'élève majeur lui-même) ou est un **organisme** avec une prise en charge **active ou échue** pour cet élève (une prise échue reste valide pour les mois de sa période : M1 des suites 3, faible 7 des suites 4).
async fn compte_lie_a_eleve(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    compte: Uuid,
) -> Result<bool, EntDbError> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM ent_fin_compte c
             WHERE c.etablissement_id = $1 AND c.id = $3
               AND ( EXISTS (SELECT 1 FROM ent_eleve_lien l
                              WHERE l.etablissement_id = c.etablissement_id AND l.eleve_id = $2
                                AND l.foyer_id = c.foyer_id)
                  OR EXISTS (SELECT 1 FROM ent_personne p
                              WHERE p.etablissement_id = c.etablissement_id AND p.id = c.personne_id
                                AND p.eleve_id = $2)
                  OR (c.type_debiteur = 'organisme' AND EXISTS (
                        SELECT 1 FROM ent_fin_prise_en_charge pc
                         WHERE pc.etablissement_id = c.etablissement_id AND pc.compte_id = c.id
                           AND pc.eleve_id = $2 AND pc.statut IN ('active', 'echue')))))",
    )
    .bind(etab)
    .bind(eleve)
    .bind(compte)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(ok)
}

/// Propose une répartition (FAC-217, FAC-218, FAC-253). Effective **tout de suite** avec une pièce ;
/// sinon **en attente** de l'acceptation de chaque payeur désigné (R-F30) : rien ne change tant qu'un
/// payeur n'a pas accepté (CF-74) et une tâche prévient le secrétariat.
pub(crate) async fn proposer_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    acteur: Uuid,
    casquette: &str,
    p: &PropositionDesignation,
) -> Result<ResultatDesignation, EntDbError> {
    if !CATEGORIES_DESIGNATION.contains(&p.categorie.as_str()) {
        return Err(invalide("catégorie inconnue"));
    }
    valider_repartition(&p.parts).map_err(invalide)?;
    if let Some(piece) = &p.piece {
        valider_piece(conn, etab, piece).await?;
    }
    verrou_eleve(conn, etab, p.eleve_id).await?;
    // L'élève et les comptes appartiennent à l'établissement : sinon « introuvable » (jamais 2xx).
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
    )
    .bind(etab)
    .bind(p.eleve_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !ok {
        return Err(introuvable());
    }
    for (compte, _, _) in &p.parts {
        let c: Option<(String,)> = sqlx::query_as(
            "SELECT etat FROM ent_fin_compte WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(compte)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        match c {
            None => return Err(introuvable()),
            Some((etat,)) if etat != "actif" => return Err(conflit("un compte archivé ne peut plus être désigné")),
            _ => {}
        }
        // M4 : seul un compte lié à l'élève (ou un organisme avec prise en charge) peut être désigné ;
        // même réponse qu'un compte inconnu (pas de sondage des comptes d'autres familles).
        if !compte_lie_a_eleve(conn, etab, p.eleve_id, *compte).await? {
            return Err(introuvable());
        }
    }
    let groupe = Uuid::new_v4();
    // Une proposition de la facturation exige l'acceptation, sauf pièce (R-F30, R-F31).
    // « accord des deux » seul ne suffit pas (M4) : il est enregistré mais l'acceptation reste exigée.
    let avec_piece = p.piece.as_ref().is_some_and(|x| x.nature != "accord_des_deux");
    let acceptation = if avec_piece || p.source == "dossier" { "non_requise" } else { "en_attente" };
    let statut = if avec_piece || p.source == "dossier" { "effective" } else { "en_attente" };
    for (compte, bps, rang) in &p.parts {
        sqlx::query(
            "INSERT INTO ent_fin_designation (id, etablissement_id, groupe_id, eleve_id, categorie, compte_id,
                    part_bps, rang, du, source, statut, acceptation, piece_nature, piece_reference,
                    piece_document_id, motif, decide_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'en_attente',$11,$12,$13,$14,$15,$16)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(groupe)
        .bind(p.eleve_id)
        .bind(&p.categorie)
        .bind(compte)
        .bind(*bps as i32)
        .bind(*rang)
        .bind(p.date_effet)
        .bind(p.source)
        .bind(acceptation)
        .bind(p.piece.as_ref().map(|x| x.nature.clone()))
        .bind(p.piece.as_ref().map(|x| x.reference.trim().to_string()))
        .bind(p.piece.as_ref().and_then(|x| x.document_id))
        .bind(p.motif.as_deref().map(|m| m.chars().take(300).collect::<String>()))
        .bind(acteur)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    journaliser_tx(
        conn,
        &Acte::new(etab, Some(acteur), casquette, Chemin::Comptes, "designation.proposer", "designation")
            .objet(groupe)
            .eleve(p.eleve_id)
            .detail(&format!(
                "catégorie {} à compter du {}, {} payeur(s), {}",
                p.categorie,
                p.date_effet,
                p.parts.len(),
                if avec_piece { "avec pièce de répartition" } else { "acceptation attendue" }
            )),
    )
    .await?;
    if statut == "effective" {
        let consequences = activer_groupe_tx(conn, etab, acteur, casquette, groupe).await?;
        return Ok(ResultatDesignation {
            groupe_id: groupe,
            statut: "effective",
            consequences,
        });
    }
    // En attente : une tâche par payeur dont l'acceptation est attendue.
    for (compte, _, _) in &p.parts {
        ouvrir_tache(
            conn,
            etab,
            "designation_a_accepter",
            Some(p.eleve_id),
            Some(*compte),
            Some(&p.categorie),
            p.date_effet,
            "répartition proposée : acceptation du payeur désigné attendue (ou pièce de répartition)",
            Some(acteur),
        )
        .await?;
    }
    Ok(ResultatDesignation {
        groupe_id: groupe,
        statut: "en_attente",
        consequences: 0,
    })
}

#[derive(sqlx::FromRow)]
struct LigneGroupe {
    id: Uuid,
    eleve_id: Uuid,
    categorie: String,
    compte_id: Uuid,
    du: NaiveDate,
    source: String,
    statut: String,
    acceptation: String,
    piece_nature: Option<String>,
}

async fn lignes_du_groupe(
    conn: &mut PgConnection,
    etab: Uuid,
    groupe: Uuid,
) -> Result<Vec<LigneGroupe>, EntDbError> {
    sqlx::query_as::<_, LigneGroupe>(
        "SELECT id, eleve_id, categorie, compte_id, du, source, statut, acceptation, piece_nature
           FROM ent_fin_designation WHERE etablissement_id = $1 AND groupe_id = $2
          ORDER BY id FOR UPDATE",
    )
    .bind(etab)
    .bind(groupe)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

/// Rend une proposition **opposable** : ferme l'ancienne répartition de la catégorie la veille de la date
/// d'effet, passe les lignes en `effective`, calcule les conséquences proposées sur les pièces émises
/// (aucune n'est touchée), résout les tâches « payeur manquant » et publie `facturation.consequences_proposees`.
pub(crate) async fn activer_groupe_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    acteur: Uuid,
    casquette: &str,
    groupe: Uuid,
) -> Result<usize, EntDbError> {
    let lignes = lignes_du_groupe(conn, etab, groupe).await?;
    let Some(premiere) = lignes.first() else {
        return Err(introuvable());
    };
    if lignes.iter().any(|l| l.statut != "en_attente") {
        return Err(conflit("cette répartition n'est plus en attente"));
    }
    let (eleve, categorie, date_effet) = (premiere.eleve_id, premiere.categorie.clone(), premiere.du);
    let sans_consentement = lignes.iter().any(|l| {
        !(l.source == "dossier"
            || l.source == "reprise"
            || l.acceptation == "acceptee"
            || (l.acceptation == "non_requise" && l.piece_nature.is_some()))
    });
    if sans_consentement {
        return Err(conflit(
            "chaque payeur désigné doit avoir accepté, ou une pièce de répartition doit être jointe",
        ));
    }
    verrou_eleve(conn, etab, eleve).await?;
    // Avant : la répartition lue la veille, pour les conséquences.
    let veille = date_effet - chrono::Duration::days(1);
    let avant = payeurs_a(conn, etab, eleve, &categorie, veille, false).await?;
    // 1. Ferme ce qui commence avant la date d'effet ; annule ce qui commence à partir d'elle.
    sqlx::query(
        "UPDATE ent_fin_designation SET au = $4, updated_at = now()
          WHERE etablissement_id = $1 AND eleve_id = $2 AND categorie = $3 AND statut = 'effective'
            AND du < $5 AND (au IS NULL OR au >= $5)",
    )
    .bind(etab)
    .bind(eleve)
    .bind(&categorie)
    .bind(veille)
    .bind(date_effet)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let futures = sqlx::query(
        "UPDATE ent_fin_designation SET statut = 'annulee', updated_at = now()
          WHERE etablissement_id = $1 AND eleve_id = $2 AND categorie = $3 AND statut = 'effective'
            AND du >= $4",
    )
    .bind(etab)
    .bind(eleve)
    .bind(&categorie)
    .bind(date_effet)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let _ = futures;
    // 2. La nouvelle répartition devient opposable.
    sqlx::query(
        "UPDATE ent_fin_designation SET statut = 'effective', decide_par = $3, updated_at = now()
          WHERE etablissement_id = $1 AND groupe_id = $2 AND statut = 'en_attente'",
    )
    .bind(etab)
    .bind(groupe)
    .bind(acteur)
    .execute(&mut *conn)
    .await
    .map_err(|e| {
        if e.as_database_error().and_then(|d| d.code()).is_some_and(|c| c == "23P01") {
            conflit("une répartition en vigueur chevauche cette période")
        } else {
            db(e)
        }
    })?;
    // 3. Les tâches de cette situation sont réglées.
    resoudre_taches(conn, etab, "designation_a_accepter", eleve, Some(&categorie), Some(acteur)).await?;
    resoudre_taches(conn, etab, "demande_repartition", eleve, Some(&categorie), Some(acteur)).await?;
    resoudre_taches(conn, etab, "payeur_manquant", eleve, Some(&categorie), Some(acteur)).await?;
    // 4. Conséquences sur les pièces émises (proposées seulement).
    let apres = payeurs_a(conn, etab, eleve, &categorie, date_effet, false).await?;
    let n = proposer_consequences_tx(conn, etab, groupe, eleve, &categorie, date_effet, &avant, &apres).await?;
    journaliser_tx(
        conn,
        &Acte::new(etab, Some(acteur), casquette, Chemin::Comptes, "designation.activer", "designation")
            .objet(groupe)
            .eleve(eleve)
            .detail(&format!(
                "répartition {categorie} opposable au {date_effet} ; {n} conséquence(s) proposée(s), aucune pièce émise touchée"
            )),
    )
    .await?;
    publier_finance(
        conn,
        etab,
        types::FACTURATION_CONSEQUENCES_PROPOSEES,
        AggregateKind::Eleve,
        eleve,
        Some(eleve),
        json!({ "groupe_id": groupe, "categorie": categorie, "date_effet": date_effet, "propositions": n }),
        Some(acteur),
        Causalite::default(),
    )
    .await?;
    Ok(n)
}

/// Compte dont l'acceptation est exprimée : accepte (ou refuse) **chaque ligne** de la proposition qui
/// vise ce compte ; la proposition devient opposable quand toutes ses lignes sont acceptées.
pub(crate) async fn repondre_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    acteur: Uuid,
    casquette: &str,
    groupe: Uuid,
    comptes_de_l_acteur: &[Uuid],
    accepte: bool,
) -> Result<Option<usize>, EntDbError> {
    let lignes = lignes_du_groupe(conn, etab, groupe).await?;
    if lignes.is_empty() {
        return Err(introuvable());
    }
    let mine: Vec<&LigneGroupe> = lignes
        .iter()
        .filter(|l| comptes_de_l_acteur.contains(&l.compte_id))
        .collect();
    // Une personne qui n'est pas désignée ne répond pas : « introuvable », rien de plus (R-F5).
    if mine.is_empty() {
        return Err(introuvable());
    }
    if lignes.iter().any(|l| l.statut != "en_attente") {
        return Err(conflit("cette répartition n'est plus en attente"));
    }
    let (eleve, categorie, date_effet) =
        (lignes[0].eleve_id, lignes[0].categorie.clone(), lignes[0].du);
    for l in &mine {
        sqlx::query(
            "UPDATE ent_fin_designation SET acceptation = $3, acceptee_le = now(), acceptee_par = $4,
                    statut = CASE WHEN $3 = 'refusee' THEN 'refusee' ELSE statut END, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(l.id)
        .bind(if accepte { "acceptee" } else { "refusee" })
        .bind(acteur)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    journaliser_tx(
        conn,
        &Acte::new(
            etab,
            Some(acteur),
            casquette,
            Chemin::Famille,
            if accepte { "designation.accepter" } else { "designation.refuser" },
            "designation",
        )
        .objet(groupe)
        .eleve(eleve),
    )
    .await?;
    if !accepte {
        // CF-74 : la répartition est refusée ; rien ne change, le secrétariat est prévenu.
        sqlx::query(
            "UPDATE ent_fin_designation SET statut = 'refusee', updated_at = now()
              WHERE etablissement_id = $1 AND groupe_id = $2 AND statut = 'en_attente'",
        )
        .bind(etab)
        .bind(groupe)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        resoudre_taches(conn, etab, "designation_a_accepter", eleve, Some(&categorie), None).await?;
        ouvrir_tache(
            conn,
            etab,
            "demande_repartition",
            Some(eleve),
            None,
            Some(&categorie),
            date_effet,
            "répartition refusée par le payeur désigné : une pièce (jugement, convention) ou un accord est nécessaire",
            Some(acteur),
        )
        .await?;
        return Ok(None);
    }
    let restant: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_fin_designation
          WHERE etablissement_id = $1 AND groupe_id = $2 AND statut = 'en_attente' AND acceptation <> 'acceptee'
            AND NOT (acceptation = 'non_requise' AND piece_nature IS NOT NULL)",
    )
    .bind(etab)
    .bind(groupe)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if restant > 0 {
        return Ok(None);
    }
    Ok(Some(activer_groupe_tx(conn, etab, acteur, casquette, groupe).await?))
}

/// Joint une pièce de répartition (FAC-217) à une proposition en attente : elle devient opposable
/// (R-F31 : un jugement est adopté comme répartition avec sa pièce ; il ne libère pas un co-signataire
/// solidaire, ce qui relève du recouvrement et du juriste).
pub(crate) async fn joindre_piece_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    acteur: Uuid,
    casquette: &str,
    groupe: Uuid,
    piece: &PieceRepartition,
) -> Result<Option<usize>, EntDbError> {
    valider_piece(conn, etab, piece).await?;
    let lignes = lignes_du_groupe(conn, etab, groupe).await?;
    if lignes.is_empty() {
        return Err(introuvable());
    }
    if lignes.iter().any(|l| l.statut != "en_attente") {
        return Err(conflit("cette répartition n'est plus en attente"));
    }
    let opposable = piece.nature != "accord_des_deux";
    sqlx::query(
        "UPDATE ent_fin_designation SET piece_nature = $3, piece_reference = $4, piece_document_id = $5,
                acceptation = CASE WHEN $6 THEN 'non_requise' ELSE acceptation END, updated_at = now()
          WHERE etablissement_id = $1 AND groupe_id = $2",
    )
    .bind(etab)
    .bind(groupe)
    .bind(&piece.nature)
    .bind(piece.reference.trim())
    .bind(piece.document_id)
    .bind(opposable)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journaliser_tx(
        conn,
        &Acte::new(etab, Some(acteur), casquette, Chemin::Comptes, "designation.joindre_piece", "designation")
            .objet(groupe)
            .eleve(lignes[0].eleve_id)
            .detail(&format!("pièce de répartition : {}", piece.nature)),
    )
    .await?;
    if !opposable {
        // M4 : l'accord des deux n'est qu'une déclaration : l'acceptation de chaque payeur reste exigée.
        return Ok(None);
    }
    Ok(Some(activer_groupe_tx(conn, etab, acteur, casquette, groupe).await?))
}

// ---------------------------------------------------------------------------
// Miroir du dossier (socle 4) : `foyer.payeur_defini`
// ---------------------------------------------------------------------------

/// Quand le dossier (socle 4, DOS-07) redéfinit les payeurs d'un élève, la facturation **date** la
/// nouvelle répartition « toutes catégories » à la date de l'événement (§9.2) : l'ancienne est fermée la
/// veille, la nouvelle devient opposable (source `dossier`, qui porte déjà le consentement du dossier), les
/// conséquences sur les pièces émises sont proposées. Idempotent : rejouer l'événement ne change rien.
///
/// Si une répartition de la **facturation** (acceptation, pièce de jugement) est en vigueur pour `toutes`,
/// le dossier ne l'écrase pas : une tâche « répartition incomplète » prévient le secrétariat.
pub(crate) async fn miroir_dossier_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    date: NaiveDate,
    acteur: Uuid,
) -> Result<usize, EntDbError> {
    verrou_eleve(conn, etab, eleve).await?;
    let liens = payeurs_des_liens(conn, etab, eleve, true).await?;
    if liens.is_empty() {
        return Ok(0);
    }
    let souhaitees: Vec<(Uuid, i64)> = calcul::repartir(
        PART_ENTIERE,
        &liens.iter().map(|p| (p.compte_id, p.part_bps)).collect::<Vec<_>>(),
    );
    // Déjà à jour ? (rejeu, ou répartition identique)
    let courantes: Vec<(Uuid, i32, String)> = sqlx::query_as(
        "SELECT compte_id, part_bps, source FROM ent_fin_designation
          WHERE etablissement_id = $1 AND eleve_id = $2 AND categorie = 'toutes' AND statut = 'effective'
            AND rang = 1 AND du <= $3 AND (au IS NULL OR au >= $3)",
    )
    .bind(etab)
    .bind(eleve)
    .bind(date)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut a: Vec<(Uuid, i64)> = courantes.iter().map(|c| (c.0, i64::from(c.1))).collect();
    let mut b = souhaitees.clone();
    a.sort();
    b.sort();
    if a == b {
        return Ok(0);
    }
    // M8 : le dossier porte le consentement (socle 4) pour une première définition ou un payeur unique ;
    // mais dès qu'une répartition existante (ou la nouvelle) compte plusieurs payeurs et **change**, rien
    // n'est rendu effectif automatiquement : une tâche prévient le secrétariat (changement entre parents).
    let plusieurs = courantes.len() > 1 || souhaitees.len() > 1;
    if courantes.iter().any(|c| c.2 == "finances" || c.2 == "majeur") || (plusieurs && !courantes.is_empty()) {
        ouvrir_tache(
            conn,
            etab,
            "repartition_incomplete",
            Some(eleve),
            None,
            Some("toutes"),
            date,
            "le dossier a changé les payeurs alors qu'une répartition est en vigueur (acceptation, pièce ou plusieurs payeurs) : rien n'a été écrasé, à traiter par le secrétariat",
            Some(acteur),
        )
        .await?;
        return Ok(0);
    }
    let prop = PropositionDesignation {
        eleve_id: eleve,
        categorie: "toutes".into(),
        parts: souhaitees.iter().map(|(c, p)| (*c, *p, 1)).filter(|(_, p, _)| *p > 0).collect(),
        date_effet: date,
        piece: None,
        motif: Some("payeurs définis au dossier de l'élève".into()),
        source: "dossier",
    };
    let r = proposer_tx(conn, etab, acteur, "reaction", &prop).await?;
    Ok(r.consequences)
}

// ---------------------------------------------------------------------------
// Conséquences : proposer
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct LigneEmise {
    id: Uuid,
    facture_id: Uuid,
    compte_id: Uuid,
    total: i64,
    periode_debut: NaiveDate,
    periode_fin: NaiveDate,
    categorie_effective: String,
}

pub(crate) async fn proposer_consequences_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    groupe: Uuid,
    eleve: Uuid,
    categorie: &str,
    date_effet: NaiveDate,
    avant: &PayeursLus,
    apres: &PayeursLus,
) -> Result<usize, EntDbError> {
    let avant_v: Vec<(Uuid, i64)> = avant.rang1().map(|p| (p.compte_id, p.part_bps)).collect();
    let apres_v: Vec<(Uuid, i64)> = apres.rang1().map(|p| (p.compte_id, p.part_bps)).collect();
    if avant_v.is_empty() || avant_v == apres_v {
        return Ok(0);
    }
    // Lignes de pièces **émises** (jamais un brouillon, jamais un avoir) qui couvrent des mois à venir.
    let lignes: Vec<LigneEmise> = sqlx::query_as(
        "SELECT l.id, l.facture_id, f.compte_id,
                (l.quantite::BIGINT * l.montant_unitaire_centimes)::BIGINT AS total,
                l.periode_debut, l.periode_fin,
                COALESCE(ar.categorie, l.categorie, 'toutes') AS categorie_effective
           FROM ent_fac_ligne l
           JOIN ent_fac_facture f ON f.etablissement_id = l.etablissement_id AND f.id = l.facture_id
           LEFT JOIN ent_fin_article ar ON ar.etablissement_id = l.etablissement_id AND ar.id = l.article_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = $2
            AND f.type_piece = 'facture' AND f.statut <> 'brouillon' AND f.compte_id IS NOT NULL
            AND l.ligne_origine_id IS NULL
            AND l.periode_debut IS NOT NULL AND l.periode_fin IS NOT NULL AND l.periode_fin >= $3
          ORDER BY l.periode_debut, l.id",
    )
    .bind(etab)
    .bind(eleve)
    .bind(date_effet)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut n = 0;
    for l in lignes {
        let cat_ligne = categorie_de_designation(&l.categorie_effective);
        // `toutes` concerne les catégories sans désignation précise ; une catégorie précise, elle-même.
        let concerne = if categorie == "toutes" {
            !payeur_precis_existe(conn, etab, eleve, cat_ligne, date_effet).await?
        } else {
            cat_ligne == categorie
        };
        if !concerne {
            continue;
        }
        let Some(c) = consequence_ligne(
            l.id,
            l.facture_id,
            l.compte_id,
            l.total,
            (l.periode_debut, l.periode_fin),
            date_effet,
            &avant_v,
            &apres_v,
        ) else {
            continue;
        };
        if c.credit > 0 {
            inserer_consequence(conn, etab, groupe, eleve, categorie, date_effet, "credit", c.compte_sortant, &c, c.credit).await?;
            n += 1;
        }
        for (compte, montant) in &c.debits {
            if *montant != 0 {
                inserer_consequence(conn, etab, groupe, eleve, categorie, date_effet, "debit", *compte, &c, *montant).await?;
                n += 1;
            }
        }
    }
    Ok(n)
}

async fn payeur_precis_existe(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    categorie: &str,
    date: NaiveDate,
) -> Result<bool, EntDbError> {
    if categorie == "toutes" {
        return Ok(false);
    }
    let (b,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_fin_designation
                         WHERE etablissement_id = $1 AND eleve_id = $2 AND categorie = $3
                           AND statut = 'effective' AND du <= $4 AND (au IS NULL OR au >= $4))",
    )
    .bind(etab)
    .bind(eleve)
    .bind(categorie)
    .bind(date)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(b)
}

async fn inserer_consequence(
    conn: &mut PgConnection,
    etab: Uuid,
    groupe: Uuid,
    eleve: Uuid,
    categorie: &str,
    date_effet: NaiveDate,
    sens: &str,
    compte: Uuid,
    c: &ConsequenceCalculee,
    montant: i64,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_fin_consequence (id, etablissement_id, designation_groupe_id, eleve_id, categorie,
                date_effet, sens, compte_id, ligne_id, facture_id, mois_total, mois_apres, montant_centimes)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
         ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(groupe)
    .bind(eleve)
    .bind(categorie)
    .bind(date_effet)
    .bind(sens)
    .bind(compte)
    .bind(c.ligne_id)
    .bind(c.facture_id)
    .bind(c.mois_total as i32)
    .bind(c.mois_apres as i32)
    .bind(montant.abs())
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }
    fn u(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn repartition_valide_somme_et_unicite() {
        assert!(valider_repartition(&[(u(1), 5000, 1), (u(2), 5000, 1)]).is_ok());
        assert!(valider_repartition(&[(u(1), 10_000, 1)]).is_ok());
        assert!(valider_repartition(&[(u(1), 6000, 1), (u(2), 3999, 1)]).is_err(), "99,99 %");
        assert!(valider_repartition(&[(u(1), 6000, 1), (u(2), 4001, 1)]).is_err(), "100,01 %");
        assert!(valider_repartition(&[(u(1), 5000, 1), (u(1), 5000, 1)]).is_err(), "même compte deux fois");
        assert!(valider_repartition(&[]).is_err());
        assert!(valider_repartition(&[(u(1), 0, 1), (u(2), 10_000, 1)]).is_err(), "part nulle");
        assert!(valider_repartition(&[(u(1), 10_001, 1)]).is_err());
        // Rang 2 : somme 100 % ou rien.
        assert!(valider_repartition(&[(u(1), 10_000, 1), (u(2), 4000, 2)]).is_err());
        assert!(valider_repartition(&[(u(1), 10_000, 1), (u(2), 4000, 2), (u(3), 6000, 2)]).is_ok());
        assert!(valider_repartition(&[(u(2), 10_000, 2)]).is_err(), "pas de principal");
        assert!(valider_repartition(&[(u(1), 10_000, 3)]).is_err());
    }

    #[test]
    fn categorie_de_designation_suit_l_article() {
        assert_eq!(categorie_de_designation("restauration"), "restauration");
        assert_eq!(categorie_de_designation("scolarite"), "scolarite");
        assert_eq!(categorie_de_designation("reduction"), "toutes");
        assert_eq!(categorie_de_designation("bourse"), "toutes");
        assert_eq!(categorie_de_designation("n'importe quoi"), "toutes");
    }

    /// CF-03 : séparation au 1er décembre, facture annuelle de 10 mois (sept.–juin) : 3 mois (sept.–nov.)
    /// à la mère, 7 mois répartis 50/50.
    #[test]
    fn decoupage_a_la_date_de_designation_cf_03() {
        let mere = u(1);
        let pere = u(2);
        let tr = [
            Tranche { du: d(2026, 9, 1), au: d(2026, 11, 30), parts: vec![(mere, 10_000)] },
            Tranche { du: d(2026, 12, 1), au: d(2027, 6, 30), parts: vec![(mere, 5_000), (pere, 5_000)] },
        ];
        let r = decouper(d(2026, 9, 1), d(2027, 6, 30), 200_000, &tr);
        assert_eq!(r.non_attribue, 0);
        let m = r.parts.iter().find(|p| p.0 == mere).unwrap().1;
        let p = r.parts.iter().find(|p| p.0 == pere).unwrap().1;
        // 3/10 = 60 000 + 7/10 = 140 000 : mère 60 000 + 70 000, père 70 000.
        assert_eq!((m, p), (130_000, 70_000));
        assert_eq!(m + p, 200_000, "la somme est exacte au centime");
    }

    #[test]
    fn decoupage_somme_exacte_et_tranche_sans_payeur() {
        let (a, b, c) = (u(1), u(2), u(3));
        // 1 003 centimes en trois tranches inégales, parts 1/3 chacune : jamais un centime perdu.
        let tr = [
            Tranche { du: d(2026, 9, 1), au: d(2026, 10, 31), parts: vec![(a, 3334), (b, 3333), (c, 3333)] },
            Tranche { du: d(2026, 11, 1), au: d(2026, 12, 31), parts: vec![(a, 10_000)] },
            Tranche { du: d(2027, 1, 1), au: d(2027, 6, 30), parts: vec![] },
        ];
        let r = decouper(d(2026, 9, 1), d(2027, 6, 30), 1_003, &tr);
        let somme: i64 = r.parts.iter().map(|p| p.1).sum::<i64>() + r.non_attribue;
        assert_eq!(somme, 1_003);
        // Six mois de la tranche sans payeur : 6/10 non attribués (catégorie non facturée sur ces mois).
        assert!(r.non_attribue >= 601 && r.non_attribue <= 602, "{}", r.non_attribue);
    }

    #[test]
    fn decoupage_tranche_hors_periode_ou_aucune_tranche() {
        let r = decouper(d(2026, 9, 1), d(2027, 6, 30), 500, &[]);
        assert_eq!(r, Decoupage { parts: vec![], non_attribue: 500 });
        let hors = [Tranche { du: d(2025, 1, 1), au: d(2025, 12, 31), parts: vec![(u(1), 10_000)] }];
        assert_eq!(decouper(d(2026, 9, 1), d(2027, 6, 30), 500, &hors).non_attribue, 500);
    }

    /// CF-82 : l'employeur couvre 100 % de la cantine dans la limite de 900 € ; au-delà, la part non
    /// couverte passe au payeur suivant ; sans payeur suivant, elle est rendue (tâche).
    #[test]
    fn plafond_d_organisme_cf_82() {
        let (employeur, mere) = (u(1), u(2));
        let mut plafonds = BTreeMap::new();
        plafonds.insert(employeur, 90_000);
        let (parts, non_couvert) = plafonner(&[(employeur, 120_000)], &plafonds, &[(mere, 10_000)]);
        assert_eq!(non_couvert, 0);
        assert_eq!(parts, vec![(employeur, 90_000), (mere, 30_000)]);
        let (parts, non_couvert) = plafonner(&[(employeur, 120_000)], &plafonds, &[]);
        assert_eq!(parts, vec![(employeur, 90_000)]);
        assert_eq!(non_couvert, 30_000, "jamais reporté sur la famille sans décision");
        // Sous le plafond : rien ne change.
        let (parts, non_couvert) = plafonner(&[(employeur, 50_000)], &plafonds, &[(mere, 10_000)]);
        assert_eq!((parts, non_couvert), (vec![(employeur, 50_000)], 0));
    }

    #[test]
    fn mois_transferes_a_la_veille_de_la_date_d_effet() {
        let (deb, fin) = (d(2026, 9, 1), d(2027, 6, 30));
        assert_eq!(mois_transferes(deb, fin, d(2026, 12, 1)), (10, 7), "01/12 : décembre à juin");
        // 15/12 : décembre est commencé par l'ancien payeur ; le nouveau reçoit janvier à juin.
        assert_eq!(mois_transferes(deb, fin, d(2026, 12, 15)), (10, 6));
        assert_eq!(mois_transferes(deb, fin, d(2026, 9, 1)), (10, 10), "dès la rentrée : tout");
        assert_eq!(mois_transferes(deb, fin, d(2025, 1, 1)), (10, 10), "antérieur à la période : tout");
        assert_eq!(mois_transferes(deb, fin, d(2027, 7, 1)), (10, 0), "après la période : rien");
    }

    /// CF-03 chiffré : la mère a payé 2 000,00 € ; au 01/12 le père reprend 50 % : avoir de 700,00 €,
    /// débit de 700,00 € au père.
    #[test]
    fn consequence_separation_cf_03() {
        let (mere, pere) = (u(1), u(2));
        let c = consequence_ligne(
            u(10),
            u(11),
            mere,
            200_000,
            (d(2026, 9, 1), d(2027, 6, 30)),
            d(2026, 12, 1),
            &[(mere, 10_000)],
            &[(mere, 5_000), (pere, 5_000)],
        )
        .unwrap();
        assert_eq!((c.mois_total, c.mois_apres), (10, 7));
        assert_eq!(c.credit, 70_000);
        assert_eq!(c.debits, vec![(pere, 70_000)]);
    }

    #[test]
    fn consequence_arrondi_en_faveur_de_la_famille() {
        let (mere, pere) = (u(1), u(2));
        // 1 000 centimes, 7 mois sur 10, moitié : 350 exacts ; avec 1 001 : 350,35 → avoir 351, débit 350.
        let c = consequence_ligne(
            u(1),
            u(2),
            mere,
            1_001,
            (d(2026, 9, 1), d(2027, 6, 30)),
            d(2026, 12, 1),
            &[(mere, 10_000)],
            &[(mere, 5_000), (pere, 5_000)],
        )
        .unwrap();
        assert_eq!(c.credit, 351, "avoir : au centime supérieur");
        assert_eq!(c.debits, vec![(pere, 350)], "facture : à l'inférieur");
        assert!(c.credit >= c.debits.iter().map(|d| d.1).sum::<i64>(), "jamais en défaveur de la famille");
    }

    #[test]
    fn consequence_ligne_de_reduction_signe_conserve() {
        let (mere, pere) = (u(1), u(2));
        let c = consequence_ligne(
            u(1), u(2), mere, -20_000,
            (d(2026, 9, 1), d(2027, 6, 30)), d(2026, 12, 1),
            &[(mere, 10_000)], &[(mere, 5_000), (pere, 5_000)],
        )
        .unwrap();
        assert_eq!(c.credit, -7_000);
        assert_eq!(c.debits, vec![(pere, -7_000)]);
    }

    #[test]
    fn consequence_nulle_quand_rien_ne_change_pour_le_compte_de_la_ligne() {
        let (mere, pere) = (u(1), u(2));
        // La ligne est celle du père dont la part augmente : rien pour lui sur sa propre ligne.
        assert!(consequence_ligne(u(1), u(2), pere, 5_000, (d(2026, 9, 1), d(2027, 6, 30)), d(2026, 12, 1),
            &[(mere, 5_000), (pere, 5_000)], &[(mere, 2_000), (pere, 8_000)]).is_none());
        // Part inchangée.
        assert!(consequence_ligne(u(1), u(2), mere, 5_000, (d(2026, 9, 1), d(2027, 6, 30)), d(2026, 12, 1),
            &[(mere, 10_000)], &[(mere, 10_000)]).is_none());
        // Période entièrement écoulée.
        assert!(consequence_ligne(u(1), u(2), mere, 5_000, (d(2026, 9, 1), d(2026, 11, 30)), d(2026, 12, 1),
            &[(mere, 10_000)], &[(pere, 10_000)]).is_none());
        // Compte absent de l'ancienne répartition.
        assert!(consequence_ligne(u(1), u(2), u(9), 5_000, (d(2026, 9, 1), d(2027, 6, 30)), d(2026, 12, 1),
            &[(mere, 10_000)], &[(pere, 10_000)]).is_none());
    }

    /// Le payeur sortant quitte tout : avoir de 100 % des mois à venir, tout à l'entrant (CF-84 : l'élève
    /// majeur devient payeur ; les mois passés restent aux parents).
    #[test]
    fn consequence_transfert_total_au_majeur_cf_84() {
        let (parents, majeur) = (u(1), u(2));
        let c = consequence_ligne(u(1), u(2), parents, 100_000, (d(2026, 9, 1), d(2027, 6, 30)), d(2027, 1, 1),
            &[(parents, 10_000)], &[(majeur, 10_000)]).unwrap();
        assert_eq!((c.mois_total, c.mois_apres), (10, 6));
        assert_eq!(c.credit, 60_000);
        assert_eq!(c.debits, vec![(majeur, 60_000)]);
    }

    #[test]
    fn debits_repartis_entre_plusieurs_entrants_somme_exacte() {
        let (a, b, c3) = (u(1), u(2), u(3));
        let c = consequence_ligne(u(1), u(2), a, 100_000, (d(2026, 9, 1), d(2027, 6, 30)), d(2026, 9, 1),
            &[(a, 10_000)], &[(a, 4_000), (b, 3_000), (c3, 3_000)]).unwrap();
        assert_eq!(c.credit, 60_000);
        assert_eq!(c.debits.iter().map(|d| d.1).sum::<i64>(), 60_000);
    }

    #[test]
    fn proportion_sup_arrondit_au_superieur() {
        assert_eq!(proportion_sup(1001, 7, 20), 351);
        assert_eq!(proportion_sup(1000, 7, 20), 350);
        assert_eq!(proportion_sup(0, 7, 20), 0);
        assert_eq!(proportion_sup(100, 0, 20), 0);
        assert_eq!(proportion_sup(100, 1, 0), 0);
    }

    fn lue(categorie: &str, compte: u128, bps: i32, rang: i16, du: NaiveDate, au: Option<NaiveDate>) -> DesignationLue {
        DesignationLue {
            categorie: categorie.into(),
            compte_id: u(compte),
            foyer_id: u(compte + 100),
            libelle: format!("compte {compte}"),
            type_debiteur: "personne".into(),
            part_bps: bps,
            rang,
            du,
            au,
        }
    }

    /// CF-02 : la mère paie la scolarité, le père la cantine ; une catégorie précise l'emporte sur `toutes`.
    #[test]
    fn categorie_precise_l_emporte_sur_toutes_cf_02() {
        let rows = vec![
            lue("toutes", 1, 10_000, 1, d(2026, 9, 1), None),
            lue("restauration", 2, 10_000, 1, d(2026, 9, 1), None),
        ];
        let resto = choisir(&rows, "restauration", d(2026, 10, 1));
        assert_eq!(resto.len(), 1);
        assert_eq!(resto[0].compte_id, u(2));
        let scol = choisir(&rows, "scolarite", d(2026, 10, 1));
        assert_eq!(scol.len(), 1);
        assert_eq!(scol[0].compte_id, u(1));
        // Avant la date de la désignation précise : retour à `toutes`.
        let rows2 = vec![
            lue("toutes", 1, 10_000, 1, d(2026, 9, 1), None),
            lue("restauration", 2, 10_000, 1, d(2027, 1, 1), None),
        ];
        assert_eq!(choisir(&rows2, "restauration", d(2026, 12, 31))[0].compte_id, u(1));
        assert_eq!(choisir(&rows2, "restauration", d(2027, 1, 1))[0].compte_id, u(2));
        // Après la fin : plus de payeur.
        let rows3 = vec![lue("toutes", 1, 10_000, 1, d(2026, 9, 1), Some(d(2026, 12, 31)))];
        assert!(choisir(&rows3, "scolarite", d(2027, 1, 1)).is_empty());
    }

    #[test]
    fn repartition_complete_exige_cent_pour_cent() {
        let part = |bps, rang| PartPayeur {
            compte_id: Uuid::new_v4(), foyer_id: Uuid::new_v4(), libelle: "x".into(),
            type_debiteur: "personne".into(), part_bps: bps, rang,
        };
        let ok = PayeursLus { parts: vec![part(6000, 1), part(4000, 1)], origine: Origine::Designation, categorie_lue: "toutes".into() };
        assert!(ok.complete());
        let manque = PayeursLus { parts: vec![part(6000, 1)], origine: Origine::Designation, categorie_lue: "toutes".into() };
        assert!(!manque.complete());
        assert!(!PayeursLus::vide("restauration").complete());
        let rang2 = PayeursLus { parts: vec![part(10_000, 1), part(5000, 2)], origine: Origine::Designation, categorie_lue: "toutes".into() };
        assert!(!rang2.complete(), "le rang 2 doit valoir 100 % s'il existe");
    }
}
