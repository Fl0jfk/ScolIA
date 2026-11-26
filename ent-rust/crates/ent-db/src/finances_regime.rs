//! Facturation au régime **mois par mois** (socle 24, F3 ; conception `docs/pilotage/conceptions/24_F3_REGIME_MOIS.md`).
//!
//! Le régime d'un mois se lit dans l'historique daté de l'élève (`ent_eleve_periode`, `kind = 'regime'`), comme le tarif
//! se choisit mois par mois : il n'y a plus qu'un espace de clés (`annuel:`) et une seule fonction de régularisation
//! (`regulariser_regime_annee`), appelée par la réaction au changement de régime et par la génération annuelle.
//!
//! Règles prudentes (D-069), toutes notées dans `docs/socles/24_FACTURATION_F3_LIVRAISON.md` :
//! * le mois est au régime **en vigueur le 1er** ; à défaut, au premier régime commencé dans le mois ;
//! * un mois **inconnu** (avant la première période connue) n'est ni facturé ni crédité (restauration, internat) ;
//! * un régime de **repli** (`ent_eleve.regime`, élève sans historique) ne donne jamais lieu à un avoir ;
//! * un mois en partie crédité (ancien format d'avoir) reste couvert : il n'est jamais refacturé ;
//! * une sortie ou une remise d'ordre ne découvrent aucun mois.

use chrono::{Datelike, NaiveDate};
use sqlx::PgConnection;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

use crate::facturation_payeurs::{
    ouvrir_tache, ouvrir_tache_evenement, resoudre_taches_evenement, resoudre_taches_sans_evenement,
};
use crate::finances::{
    db, indice_mois, inserer_avoirs, lignes_eleve_annee, params_tx, premier_du_mois, AvoirAFaire,
    LigneExistante, ResultatInsertion, NATURE_TACHE_REGIME_NON_FACTURE,
};
use crate::finances_calcul::{self as calcul};
use crate::outbox::Causalite;
use crate::service::EntDbError;

/// Régime d'un mois, tel que la facturation le lit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RegimeMois {
    /// Lu dans une période de l'historique : seul un régime connu autorise un avoir.
    Connu(String),
    /// Élève sans aucune période de régime : `ent_eleve.regime` (comportement d'avant), jamais d'avoir.
    Repli(String),
    /// L'élève a un historique, mais aucune période ne couvre ce mois (avant la première).
    Inconnu,
}

impl RegimeMois {
    /// Valeur du régime, `None` pour un mois inconnu.
    pub(crate) fn valeur(&self) -> Option<&str> {
        match self {
            Self::Connu(v) | Self::Repli(v) => Some(v),
            Self::Inconnu => None,
        }
    }

    /// La catégorie est-elle due ce mois ? Un mois inconnu ne facture ni restauration ni internat ; scolarité et frais
    /// restent dus (leur tarif se choisit alors avec le régime de la fiche).
    pub(crate) fn du(&self, categorie: &str) -> bool {
        match self {
            Self::Connu(v) | Self::Repli(v) => calcul::categorie_due(categorie, v),
            Self::Inconnu => calcul::categorie_due(categorie, "externe"),
        }
    }
}

/// Une période de régime de l'historique de l'élève.
#[derive(Debug, Clone)]
pub(crate) struct PeriodeRegime {
    pub debut: NaiveDate,
    pub fin: Option<NaiveDate>,
    pub valeur: String,
}

/// Régime du mois commençant le `premier` (§ 2.1) : fonction pure.
pub(crate) fn regime_du_mois(
    periodes: &[PeriodeRegime],
    repli: &str,
    premier: NaiveDate,
) -> RegimeMois {
    let dernier = calcul::fin_de_mois(premier.year(), premier.month());
    // 1. la période en vigueur le 1er jour (la plus récente si l'historique se recouvre)
    if let Some(p) = periodes
        .iter()
        .filter(|p| p.debut <= premier && p.fin.is_none_or(|f| f >= premier))
        .max_by_key(|p| p.debut)
    {
        return RegimeMois::Connu(calcul::regime_normalise(Some(&p.valeur)));
    }
    // 2. à défaut, la première période commencée dans le mois (mois d'arrivée)
    if let Some(p) = periodes
        .iter()
        .filter(|p| p.debut > premier && p.debut <= dernier)
        .min_by_key(|p| p.debut)
    {
        return RegimeMois::Connu(calcul::regime_normalise(Some(&p.valeur)));
    }
    // 3. sans aucune période : la fiche ; avec un historique qui ne couvre pas ce mois : inconnu
    if periodes.is_empty() {
        RegimeMois::Repli(calcul::regime_normalise(Some(repli)))
    } else {
        RegimeMois::Inconnu
    }
}

/// Régime de chacun des mois commençant aux dates `mois` (premiers jours), lu dans l'historique de l'élève.
pub(crate) async fn regimes_par_mois(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    mois: &[NaiveDate],
) -> Result<Vec<RegimeMois>, EntDbError> {
    let rows: Vec<(NaiveDate, Option<NaiveDate>, String)> = sqlx::query_as(
        "SELECT date_debut, date_fin, valeur FROM ent_eleve_periode
          WHERE etablissement_id = $1 AND eleve_id = $2 AND kind = 'regime' ORDER BY date_debut",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let periodes: Vec<PeriodeRegime> = rows
        .into_iter()
        .map(|(debut, fin, valeur)| PeriodeRegime { debut, fin, valeur })
        .collect();
    let repli: Option<(Option<String>,)> =
        sqlx::query_as("SELECT regime FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleve)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
    let repli = repli.and_then(|r| r.0).unwrap_or_default();
    Ok(mois
        .iter()
        .map(|m| regime_du_mois(&periodes, &repli, *m))
        .collect())
}

// ---------------------------------------------------------------------------
// Mois couverts nets (§ 2.4)
// ---------------------------------------------------------------------------

/// Un avoir de régime déjà posé sur une ligne.
#[derive(Debug, Clone)]
pub(crate) struct AvoirRegime {
    pub cle: String,
    pub periode: Option<(NaiveDate, NaiveDate)>,
    /// Montant signé de la ligne d'avoir (négatif pour le crédit d'une ligne positive).
    pub montant: i64,
}

/// Mois de la période `[debut, fin]` d'une ligne de total `total` crédités par des avoirs de régime (fonction pure) :
/// * nouveau format (`regime:mois:avoir:…`, période propre) : ses mois exactement ;
/// * ancien format (`regime:{événement}:avoir:…`, période recopiée de la ligne) : les derniers mois, au nombre de
///   `floor(mois × crédit / total)` (un mois en partie crédité reste couvert, D-069) ;
/// * les avoirs de sortie et d'absence ne passent pas par ici : ils ne découvrent aucun mois.
pub(crate) fn mois_decouverts(
    debut: NaiveDate,
    fin: NaiveDate,
    total: i64,
    avoirs: &[AvoirRegime],
) -> BTreeSet<i32> {
    let (premier, dernier) = (indice_mois(debut), indice_mois(fin));
    let mut decouverts = BTreeSet::new();
    let mut ancien_credit: i128 = 0;
    for a in avoirs {
        if a.cle.starts_with("regime:mois:avoir:") {
            if let Some((d, f)) = a.periode {
                decouverts.extend(
                    (indice_mois(d)..=indice_mois(f)).filter(|m| (premier..=dernier).contains(m)),
                );
            }
        } else {
            ancien_credit += i128::from(a.montant).abs();
        }
    }
    if ancien_credit > 0 && total != 0 {
        let nm = i128::from(dernier - premier + 1);
        let a_retirer = (nm * ancien_credit / i128::from(total).abs()).clamp(0, nm) as i32;
        decouverts.extend(((dernier - a_retirer + 1)..=dernier).filter(|m| *m >= premier));
    }
    decouverts
}

/// Renseigne `decouverts` de chaque ligne à partir des avoirs de régime posés sur elle.
pub(crate) async fn remplir_decouverts(
    conn: &mut PgConnection,
    etab: Uuid,
    lignes: &mut [LigneExistante],
) -> Result<(), EntDbError> {
    if lignes.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
    let rows: Vec<(Uuid, String, Option<NaiveDate>, Option<NaiveDate>, i64)> = sqlx::query_as(
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
    let mut par_ligne: HashMap<Uuid, Vec<AvoirRegime>> = HashMap::new();
    for (id, cle, d, f, montant) in rows {
        par_ligne.entry(id).or_default().push(AvoirRegime {
            cle,
            periode: d.zip(f),
            montant,
        });
    }
    for l in lignes.iter_mut() {
        if let (Some(d), Some(f), Some(avoirs)) =
            (l.periode_debut, l.periode_fin, par_ligne.get(&l.id))
        {
            l.decouverts = mois_decouverts(d, f, l.total, avoirs);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Régularisation (§ 2.3)
// ---------------------------------------------------------------------------

fn categorie_de_ligne(l: &LigneExistante) -> &str {
    if l.categorie.as_deref() == Some("reduction") {
        l.categorie_article.as_deref().unwrap_or("")
    } else {
        l.categorie.as_deref().unwrap_or("")
    }
}

fn est_regime(categorie: &str) -> bool {
    matches!(categorie, "restauration" | "internat")
}

/// Suites ininterrompues d'indices de mois triés.
fn suites(mois: &[i32]) -> Vec<(i32, i32)> {
    let mut out: Vec<(i32, i32)> = Vec::new();
    for &m in mois {
        match out.last_mut() {
            Some((_, fin)) if *fin + 1 == m => *fin = m,
            _ => out.push((m, m)),
        }
    }
    out
}

/// Montant du crédit pour `k` mois sur `nm` : en faveur de la famille (ligne positive : centime supérieur ; réduction :
/// reprise au centime inférieur, en valeur absolue).
fn credit_prorata(total: i64, k: i64, nm: i64) -> i64 {
    if nm <= 0 {
        return 0;
    }
    let abs = i128::from(total).abs() * i128::from(k);
    let nm = i128::from(nm);
    let valeur = if total >= 0 {
        (abs + nm - 1) / nm
    } else {
        abs / nm
    };
    let valeur = i64::try_from(valeur).unwrap_or(i64::MAX);
    if total >= 0 {
        valeur
    } else {
        -valeur
    }
}

/// Crédite les mois facturés hors régime (restauration, internat et leurs réductions) de l'élève pour l'année :
/// seulement ceux dont le régime est **lu dans l'historique** et pour lesquels la catégorie n'est pas due. Idempotente
/// (clé `regime:mois:avoir:{ligne}:m{AAAA-MM}`, un mois crédité n'est plus couvert). Verrou d'élève pris d'abord.
pub(crate) async fn regulariser_regime_annee(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve: Uuid,
    evenement: Option<Uuid>,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<ResultatInsertion, EntDbError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(format!("fin-regime:{etab}:{eleve}"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    let lignes: Vec<LigneExistante> = lignes_eleve_annee(conn, etab, eleve, annee)
        .await?
        .into_iter()
        .filter(|l| {
            est_regime(categorie_de_ligne(l)) && l.periode_debut.zip(l.periode_fin).is_some()
        })
        .collect();
    if lignes.is_empty() {
        return Ok(ResultatInsertion::default());
    }
    let mut indices: BTreeSet<i32> = BTreeSet::new();
    for l in &lignes {
        if let (Some(d), Some(f)) = (l.periode_debut, l.periode_fin) {
            indices.extend((indice_mois(d)..=indice_mois(f)).filter(|m| !l.decouverts.contains(m)));
        }
    }
    let premiers: Vec<NaiveDate> = indices.iter().map(|i| premier_du_mois(*i)).collect();
    let regimes = regimes_par_mois(conn, etab, eleve, &premiers).await?;
    let par_mois: HashMap<i32, RegimeMois> = indices.iter().copied().zip(regimes).collect();
    let mut avoirs: Vec<AvoirAFaire> = Vec::new();
    for l in lignes {
        let (Some(d), Some(f)) = (l.periode_debut, l.periode_fin) else {
            continue;
        };
        let categorie = categorie_de_ligne(&l).to_string();
        let nm = i64::from(indice_mois(f) - indice_mois(d) + 1);
        let a_crediter: Vec<i32> = (indice_mois(d)..=indice_mois(f))
            .filter(|m| !l.decouverts.contains(m))
            .filter(|m| matches!(par_mois.get(m), Some(RegimeMois::Connu(r)) if !calcul::categorie_due(&categorie, r)))
            .collect();
        // Reste créditable : ce qui reste sur la ligne moins la valeur des mois **conservés** (couverts nets non crédités),
        // pour qu'un avoir de sortie ou d'absence déjà posé ne mène jamais à rembourser des mois dus (double crédit).
        let nets = (indice_mois(d)..=indice_mois(f))
            .filter(|m| !l.decouverts.contains(m))
            .count() as i64;
        let conserves = (nets - a_crediter.len() as i64).max(0);
        let reserve = {
            let v = i128::from(l.total).abs() * i128::from(conserves) / i128::from(nm.max(1));
            let v = i64::try_from(v).unwrap_or(i64::MAX);
            if l.total >= 0 {
                v
            } else {
                -v
            }
        };
        let mut reste = l.total - l.deja_credite - reserve;
        for (debut, fin) in suites(&a_crediter) {
            let credit = credit_prorata(l.total, i64::from(fin - debut + 1), nm);
            // borné par ce qui reste sur la ligne (même signe que le total)
            let credit = if reste.signum() == l.total.signum() && l.total != 0 {
                if l.total > 0 {
                    credit.min(reste)
                } else {
                    credit.max(reste)
                }
            } else {
                0
            };
            if credit == 0 {
                continue;
            }
            reste -= credit;
            let (p1, p2) = (premier_du_mois(debut), premier_du_mois(fin));
            avoirs.push(AvoirAFaire {
                credit,
                eleve_id: Some(eleve),
                libelle: format!(
                    "Régime : {categorie} non due du {} au {} — {}",
                    p1.format("%m/%Y"),
                    p2.format("%m/%Y"),
                    l.libelle
                ),
                source_cle: format!("regime:mois:avoir:{}:m{}", l.id, p1.format("%Y-%m")),
                periode: Some((p1, calcul::fin_de_mois(p2.year(), p2.month()))),
                origine: l.clone(),
            });
        }
    }
    let resultat = inserer_avoirs(
        conn,
        etab,
        Some(annee),
        "changement de régime",
        avoirs,
        actor,
        causalite,
    )
    .await?;
    if resultat.creees > 0 {
        tracing::info!(etablissement = %etab, ?evenement, lignes = resultat.creees, "avoirs de régime posés");
    }
    Ok(resultat)
}

// ---------------------------------------------------------------------------
// Mois inconnus (§ 2.1) : tâche sans événement
// ---------------------------------------------------------------------------

/// Signale les mois de régime **inconnu** que la facturation ne peut pas poser : tâche `regularisation_regime_non_
/// facturee` sans événement, une seule ouverte par élève. Les mois déjà couverts par une ligne de restauration ou
/// d'internat posée ne comptent pas. Aucun mois à signaler : la tâche sans événement est close.
pub(crate) async fn signaler_mois_inconnus(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve: Uuid,
    inconnus: &[NaiveDate],
) -> Result<(), EntDbError> {
    let mut non_poses: Vec<NaiveDate> = Vec::new();
    if !inconnus.is_empty() {
        let mut couverts: BTreeSet<i32> = BTreeSet::new();
        for l in lignes_eleve_annee(conn, etab, eleve, annee).await? {
            if let (true, Some(d), Some(f)) = (
                est_regime(categorie_de_ligne(&l)),
                l.periode_debut,
                l.periode_fin,
            ) {
                couverts.extend(
                    (indice_mois(d)..=indice_mois(f)).filter(|m| !l.decouverts.contains(m)),
                );
            }
        }
        non_poses = inconnus
            .iter()
            .copied()
            .filter(|m| !couverts.contains(&indice_mois(*m)))
            .collect();
    }
    if let (Some(premier), Some(dernier)) = (non_poses.first(), non_poses.last()) {
        let detail = format!(
            "régime inconnu du {} au {} : restauration et internat non facturés pour ces mois ; complétez l'historique de régime de l'élève",
            premier.format("%d/%m/%Y"),
            calcul::fin_de_mois(dernier.year(), dernier.month()).format("%d/%m/%Y")
        );
        ouvrir_tache(
            conn,
            etab,
            NATURE_TACHE_REGIME_NON_FACTURE,
            Some(eleve),
            None,
            None,
            *premier,
            &detail,
            None,
        )
        .await?;
    } else {
        resoudre_taches_sans_evenement(conn, etab, NATURE_TACHE_REGIME_NON_FACTURE, eleve).await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Réaction au changement de régime (§ 2.5)
// ---------------------------------------------------------------------------

/// Date à partir de laquelle l'élève est facturé pour `annee` : première inscription validée de l'année, à défaut
/// `ent_eleve.date_entree`, à défaut aucune (année entière).
pub(crate) async fn depuis_facturation(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &str,
    eleve: Uuid,
) -> Result<Option<NaiveDate>, EntDbError> {
    let valide: Option<(Option<chrono::DateTime<chrono::Utc>>,)> = sqlx::query_as(
        "SELECT min(d.valide_le)
           FROM ent_inscription_dossier d
           JOIN ent_inscription_campagne c ON c.id = d.campagne_id AND c.etablissement_id = d.etablissement_id
          WHERE d.etablissement_id = $1 AND c.annee_cible = $2 AND d.statut = 'validee' AND d.eleve_id = $3",
    )
    .bind(etab)
    .bind(annee)
    .bind(eleve)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some(date) = valide.and_then(|v| v.0) {
        return Ok(Some(date.date_naive()));
    }
    let entree: Option<(Option<NaiveDate>,)> =
        sqlx::query_as("SELECT date_entree FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleve)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
    Ok(entree.and_then(|e| e.0))
}

/// Catégories (restauration, internat) **gagnées** à la date d'effet : dues au régime du mois de `date_effet` (règle du
/// 1er) ou du mois suivant, et non dues au régime du mois précédent. Repli sur la charge de l'événement quand
/// l'historique ne permet pas de conclure.
pub(crate) fn categories_gagnees(
    precedent: &RegimeMois,
    courant: &RegimeMois,
    suivant: &RegimeMois,
    charge: Option<(&str, &str)>,
) -> Vec<&'static str> {
    let (avant, apres): (Option<String>, Vec<String>) = match (courant.valeur(), suivant.valeur()) {
        (None, None) => match charge {
            Some((ancien, nouveau)) => (
                Some(calcul::regime_normalise(Some(ancien))),
                vec![calcul::regime_normalise(Some(nouveau))],
            ),
            None => return Vec::new(),
        },
        (c, s) => (
            precedent.valeur().map(str::to_string),
            [c, s].into_iter().flatten().map(str::to_string).collect(),
        ),
    };
    ["restauration", "internat"]
        .into_iter()
        .filter(|cat| {
            apres.iter().any(|r| calcul::categorie_due(cat, r))
                && !avant
                    .as_deref()
                    .is_some_and(|r| calcul::categorie_due(cat, r))
        })
        .collect()
}

/// Réaction à `eleve.regime_changed` (les deux chemins : finances et ELV-10) : régularisation des mois facturés hors
/// régime, puis annuel limité aux catégories gagnées. Plus aucune ligne `regime:` n'est créée : les clés sont celles de
/// la génération annuelle, un rejeu (avant ou après) ne double rien. Seules `eleve` et `date_effet` sont exigées ;
/// `charge` = (ancien, nouveau) de l'événement ne sert qu'en repli.
// 9 arguments : connexion, contexte (établissement, élève, date, événement), charge, acteur et causalité.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn reagir_changement_regime(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    date_effet: NaiveDate,
    evenement: Uuid,
    charge: Option<(&str, &str)>,
    actor: Option<Uuid>,
    causalite: Causalite,
) -> Result<ResultatInsertion, EntDbError> {
    let p = params_tx(conn, etab).await?;
    let annee = crate::finances::annee_de(&p, date_effet);
    let mut res =
        regulariser_regime_annee(conn, etab, &annee, eleve, Some(evenement), actor, causalite)
            .await?;
    let courant_1er =
        NaiveDate::from_ymd_opt(date_effet.year(), date_effet.month(), 1).expect("date valide");
    let suivant_1er =
        calcul::fin_de_mois(date_effet.year(), date_effet.month()) + chrono::Duration::days(1);
    let precedent_1er = courant_1er - chrono::Duration::days(1);
    let precedent_1er = NaiveDate::from_ymd_opt(precedent_1er.year(), precedent_1er.month(), 1)
        .expect("date valide");
    let r = regimes_par_mois(
        conn,
        etab,
        eleve,
        &[precedent_1er, courant_1er, suivant_1er],
    )
    .await?;
    let gagnees = categories_gagnees(&r[0], &r[1], &r[2], charge);
    if gagnees.is_empty() {
        return Ok(res);
    }
    let depuis = depuis_facturation(conn, etab, &annee, eleve).await?;
    match crate::finances::facturer_annee_lignes(
        conn,
        etab,
        &annee,
        eleve,
        depuis,
        Some(&gagnees),
        actor,
        causalite,
    )
    .await?
    {
        Err(motif) => {
            // Une régularisation de régime non facturable n'est jamais silencieuse : journal et tâche d'anomalie liée à
            // l'événement, visible des gestionnaires ; close dès que l'événement est rejoué avec succès.
            tracing::warn!(etablissement = %etab, evenement = %evenement, %motif,
                "changement de régime non facturé : à régulariser à la main");
            let detail = format!(
                "changement de régime au {} non facturé ({}) : {motif}",
                date_effet.format("%d/%m/%Y"),
                gagnees.join(", ")
            );
            ouvrir_tache_evenement(
                conn,
                etab,
                NATURE_TACHE_REGIME_NON_FACTURE,
                Some(eleve),
                None,
                None,
                date_effet,
                &detail,
                actor,
                Some(evenement),
            )
            .await?;
        }
        Ok(r) => {
            res.ajouter(r);
            resoudre_taches_evenement(
                conn,
                etab,
                NATURE_TACHE_REGIME_NON_FACTURE,
                eleve,
                evenement,
                actor,
            )
            .await?;
        }
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }
    fn p(debut: NaiveDate, fin: Option<NaiveDate>, v: &str) -> PeriodeRegime {
        PeriodeRegime {
            debut,
            fin,
            valeur: v.into(),
        }
    }
    fn connu(v: &str) -> RegimeMois {
        RegimeMois::Connu(v.into())
    }

    #[test]
    fn regime_du_mois_changement_le_1er_le_mois_est_au_nouveau_regime() {
        let h = [
            p(d(2030, 1, 1), Some(d(2030, 10, 31)), "externe"),
            p(d(2030, 11, 1), None, "demi_pension"),
        ];
        assert_eq!(
            regime_du_mois(&h, "externe", d(2030, 10, 1)),
            connu("externe")
        );
        assert_eq!(
            regime_du_mois(&h, "externe", d(2030, 11, 1)),
            connu("demi_pension")
        );
    }

    #[test]
    fn regime_du_mois_changement_le_15_le_mois_reste_a_l_ancien_regime() {
        let h = [
            p(d(2030, 1, 1), Some(d(2030, 10, 14)), "externe"),
            p(d(2030, 10, 15), None, "demi_pension"),
        ];
        assert_eq!(
            regime_du_mois(&h, "demi_pension", d(2030, 9, 1)),
            connu("externe")
        );
        assert_eq!(
            regime_du_mois(&h, "demi_pension", d(2030, 10, 1)),
            connu("externe")
        );
        assert_eq!(
            regime_du_mois(&h, "demi_pension", d(2030, 11, 1)),
            connu("demi_pension")
        );
    }

    #[test]
    fn regime_du_mois_arrivee_le_3_le_mois_d_arrivee_prend_le_premier_regime_commence() {
        let h = [p(d(2030, 9, 3), None, "demi_pension")];
        assert_eq!(
            regime_du_mois(&h, "externe", d(2030, 9, 1)),
            connu("demi_pension")
        );
        assert_eq!(
            regime_du_mois(&h, "externe", d(2030, 10, 1)),
            connu("demi_pension")
        );
    }

    #[test]
    fn regime_du_mois_sans_aucun_historique_repli_sur_la_fiche() {
        assert_eq!(
            regime_du_mois(&[], "demi_pension", d(2030, 9, 1)),
            RegimeMois::Repli("demi_pension".into())
        );
        assert_eq!(
            regime_du_mois(&[], "DP", d(2031, 6, 1)),
            RegimeMois::Repli("demi_pension".into())
        );
    }

    #[test]
    fn regime_du_mois_avant_la_premiere_periode_est_inconnu() {
        let h = [p(d(2031, 2, 1), None, "externe")];
        assert_eq!(
            regime_du_mois(&h, "demi_pension", d(2030, 12, 1)),
            RegimeMois::Inconnu
        );
        assert_eq!(
            regime_du_mois(&h, "demi_pension", d(2031, 2, 1)),
            connu("externe")
        );
        // un mois inconnu ne facture ni restauration ni internat, mais la scolarité reste due
        assert!(!RegimeMois::Inconnu.du("restauration") && !RegimeMois::Inconnu.du("internat"));
        assert!(RegimeMois::Inconnu.du("scolarite") && RegimeMois::Inconnu.du("frais"));
    }

    #[test]
    fn mois_decouverts_nouveau_format_ancien_format_et_autres_avoirs() {
        let (deb, fin) = (d(2030, 9, 1), d(2031, 6, 30));
        let nouveau = AvoirRegime {
            cle: "regime:mois:avoir:x:m2030-12".into(),
            periode: Some((d(2030, 12, 1), d(2031, 2, 28))),
            montant: -13_500,
        };
        let m = mois_decouverts(deb, fin, 45_000, &[nouveau]);
        let attendus: BTreeSet<i32> =
            (indice_mois(d(2030, 12, 1))..=indice_mois(d(2031, 2, 1))).collect();
        assert_eq!(m, attendus);
        // ancien format : floor(10 × 20 000 / 45 000) = 4 derniers mois ; un mois en partie crédité reste couvert
        let ancien = AvoirRegime {
            cle: "regime:evt:avoir:x".into(),
            periode: Some((deb, fin)),
            montant: -20_000,
        };
        let m = mois_decouverts(deb, fin, 45_000, &[ancien]);
        assert_eq!(m.len(), 4);
        assert_eq!(
            m.iter().next_back().copied(),
            Some(indice_mois(d(2031, 6, 1)))
        );
        let petit = AvoirRegime {
            cle: "regime:evt:avoir:x".into(),
            periode: None,
            montant: -4_499,
        };
        assert!(mois_decouverts(deb, fin, 45_000, &[petit]).is_empty());
    }

    #[test]
    fn credit_prorata_en_faveur_de_la_famille() {
        assert_eq!(credit_prorata(45_000, 7, 10), 31_500);
        assert_eq!(credit_prorata(10_000, 1, 3), 3_334); // centime supérieur
        assert_eq!(credit_prorata(-10_000, 1, 3), -3_333); // reprise au centime inférieur
        assert_eq!(credit_prorata(i64::MAX, 10, 10), i64::MAX);
    }

    #[test]
    fn categories_gagnees_selon_l_historique_et_repli_sur_la_charge() {
        let ext = connu("externe");
        let dp = connu("demi_pension");
        assert_eq!(
            categories_gagnees(&ext, &ext, &dp, None),
            vec!["restauration"]
        );
        assert_eq!(
            categories_gagnees(&ext, &dp, &dp, None),
            vec!["restauration"]
        );
        assert!(categories_gagnees(&dp, &dp, &dp, None).is_empty());
        assert!(categories_gagnees(&dp, &ext, &ext, None).is_empty());
        assert_eq!(
            categories_gagnees(&dp, &connu("interne"), &connu("interne"), None),
            vec!["internat"]
        );
        assert_eq!(
            categories_gagnees(
                &RegimeMois::Inconnu,
                &RegimeMois::Inconnu,
                &RegimeMois::Inconnu,
                Some(("externe", "demi_pension"))
            ),
            vec!["restauration"]
        );
        assert!(categories_gagnees(
            &RegimeMois::Inconnu,
            &RegimeMois::Inconnu,
            &RegimeMois::Inconnu,
            None
        )
        .is_empty());
    }

    #[test]
    fn suites_de_mois() {
        assert_eq!(suites(&[1, 2, 3, 6, 7, 9]), vec![(1, 3), (6, 7), (9, 9)]);
        assert!(suites(&[]).is_empty());
    }
}
