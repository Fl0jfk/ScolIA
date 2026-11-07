//! Calculs purs de la facturation (socle 2, lot S2-9) : sans base ni réseau,
//! testés unitairement. Tous les montants sont des **centimes entiers** (`i64`) ;
//! aucun flottant. Règles d'arrondi (documentées dans `docs/SOCLE2_DOMAINES.md`) :
//!
//! * **prorata au mois** : tout mois commencé est dû ; montant dû =
//!   `annuel × mois_dus / nb_mois`, arrondi au centime **inférieur** (en faveur
//!   de la famille) ;
//! * **avoir de sortie / changement** = montant facturé − montant dû sur les mois
//!   conservés : facture + avoir = exactement le dû ;
//! * **taux** (réductions) : `montant × bps / 10 000`, arrondi inférieur ;
//! * **répartition entre payeurs** (parts en %) : méthode du plus fort reste,
//!   la somme des parts est **exactement** le montant ;
//! * **échéances** : parts égales, le reste (centimes) sur la **première**.

use chrono::{Datelike, NaiveDate};
use uuid::Uuid;

/// Mois facturables d'une année scolaire « AAAA-AAAA ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Calendrier {
    pub annee_debut: i32,
    pub mois_debut: u32,
    pub nb_mois: u32,
}

impl Calendrier {
    pub fn new(annee_scolaire: &str, mois_debut: u32, nb_mois: u32) -> Option<Self> {
        let (a, b) = annee_scolaire.split_once('-')?;
        let (a, b): (i32, i32) = (a.parse().ok()?, b.parse().ok()?);
        if b != a + 1 || !(1..=12).contains(&mois_debut) || !(1..=12).contains(&nb_mois) {
            return None;
        }
        Some(Self {
            annee_debut: a,
            mois_debut,
            nb_mois,
        })
    }

    /// (année, mois) du i-ème mois facturable (0-indexé).
    pub fn mois(&self, i: u32) -> (i32, u32) {
        let m0 = self.mois_debut - 1 + i;
        (self.annee_debut + (m0 / 12) as i32, m0 % 12 + 1)
    }

    pub fn premier_jour(&self) -> NaiveDate {
        let (a, m) = self.mois(0);
        NaiveDate::from_ymd_opt(a, m, 1).expect("date valide")
    }

    pub fn dernier_jour(&self) -> NaiveDate {
        let (a, m) = self.mois(self.nb_mois - 1);
        fin_de_mois(a, m)
    }

    /// Index du mois facturable contenant `d` : `None` avant le début, `Some(nb_mois)` après la fin.
    fn index(&self, d: NaiveDate) -> Option<u32> {
        let rang = (d.year() - self.annee_debut) * 12 + d.month() as i32 - self.mois_debut as i32;
        if rang < 0 {
            None
        } else {
            Some((rang as u32).min(self.nb_mois))
        }
    }

    /// Mois dus pour une entrée à la date `d` (mois d'entrée compris).
    pub fn mois_dus_depuis(&self, d: NaiveDate) -> u32 {
        match self.index(d) {
            None => self.nb_mois,
            Some(i) => self.nb_mois - i,
        }
    }

    /// Mois conservés (dus) quand on part ou change à la date `d` : du début
    /// jusqu'au mois de `d` **inclus** (mois commencé dû).
    pub fn mois_dus_jusqu_a(&self, d: NaiveDate) -> u32 {
        match self.index(d) {
            None => 0,
            Some(i) if i >= self.nb_mois => self.nb_mois,
            Some(i) => i + 1,
        }
    }

    /// Dates d'échéance par défaut : `jour` de chaque mois facturable à partir
    /// du mois de `depuis` (au moins une échéance).
    pub fn echeances_par_defaut(&self, depuis: NaiveDate, jour: u32) -> Vec<NaiveDate> {
        let debut = self.index(depuis).unwrap_or(0).min(self.nb_mois - 1);
        (debut..self.nb_mois)
            .map(|i| {
                let (a, m) = self.mois(i);
                NaiveDate::from_ymd_opt(a, m, jour.clamp(1, 28)).expect("date valide")
            })
            .collect()
    }
}

pub fn fin_de_mois(a: i32, m: u32) -> NaiveDate {
    let (a2, m2) = if m == 12 { (a + 1, 1) } else { (a, m + 1) };
    NaiveDate::from_ymd_opt(a2, m2, 1).expect("date valide") - chrono::Duration::days(1)
}

/// `montant × num / den`, arrondi vers zéro (inférieur pour un montant positif).
pub fn proportion(montant: i64, num: i64, den: i64) -> i64 {
    if den <= 0 {
        return 0;
    }
    ((montant as i128 * num as i128) / den as i128) as i64
}

/// Montant dû pour `mois_dus` mois sur une base annuelle.
pub fn prorata(annuel: i64, mois_dus: u32, nb_mois: u32) -> i64 {
    proportion(annuel, mois_dus.min(nb_mois) as i64, nb_mois as i64)
}

/// Réduction en points de base (arrondi inférieur).
pub fn taux(montant: i64, bps: i32) -> i64 {
    proportion(montant, bps.clamp(0, 10_000) as i64, 10_000)
}

/// Répartit `montant` selon des parts (en %, somme attendue 100) par la méthode
/// du plus fort reste : la somme des résultats est **exactement** `montant`.
/// Parts nulles ou somme nulle : tout sur le premier.
pub fn repartir<K: Copy>(montant: i64, parts: &[(K, i64)]) -> Vec<(K, i64)> {
    if parts.is_empty() {
        return Vec::new();
    }
    let total: i64 = parts.iter().map(|(_, p)| (*p).max(0)).sum();
    if total == 0 {
        let mut v: Vec<(K, i64)> = parts.iter().map(|(k, _)| (*k, 0)).collect();
        v[0].1 = montant;
        return v;
    }
    let signe = if montant < 0 { -1 } else { 1 };
    let abs = montant.abs() as i128;
    let mut res: Vec<(usize, i64, i128)> = parts
        .iter()
        .enumerate()
        .map(|(i, (_, p))| {
            let num = abs * (*p).max(0) as i128;
            (i, (num / total as i128) as i64, num % total as i128)
        })
        .collect();
    let mut reste = abs as i64 - res.iter().map(|r| r.1).sum::<i64>();
    let mut ordre: Vec<usize> = (0..res.len()).collect();
    // Plus fort reste d'abord ; à égalité, l'ordre d'origine.
    ordre.sort_by(|a, b| res[*b].2.cmp(&res[*a].2).then(a.cmp(b)));
    for i in ordre {
        if reste == 0 {
            break;
        }
        res[i].1 += 1;
        reste -= 1;
    }
    res.into_iter()
        .map(|(i, m, _)| (parts[i].0, signe * m))
        .collect()
}

/// Découpe `total` en `n` échéances égales ; le reste va sur la première.
pub fn echeances(total: i64, n: usize) -> Vec<i64> {
    if n == 0 {
        return Vec::new();
    }
    let base = total / n as i64;
    let reste = total - base * n as i64;
    let mut v = vec![base; n];
    v[0] += reste;
    v
}

/// Ligne de grille tarifaire candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TarifCandidat {
    pub id: Uuid,
    pub article_id: Uuid,
    pub niveau: Option<String>,
    pub regime: Option<String>,
    pub qf_min: Option<i64>,
    pub qf_max: Option<i64>,
    pub montant_annuel: i64,
}

/// Normalise un régime : `externe`, `demi_pension`, `interne` (accepte EXT/DP/INT).
pub fn regime_normalise(r: Option<&str>) -> String {
    match r.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("demi_pension") | Some("dp") | Some("demi-pension") | Some("demi-pensionnaire") => {
            "demi_pension".into()
        }
        Some("interne") | Some("int") | Some("internat") => "interne".into(),
        _ => "externe".into(),
    }
}

/// Le tarif applicable pour un article : filtres niveau / régime / tranche de
/// quotient. Un tarif sans tranche convient à tous ; un foyer **sans quotient**
/// connu prend le tarif sans borne haute (plein tarif). Le plus spécifique gagne
/// (niveau précis > tous, régime précis > tous, tranche > sans tranche).
pub fn choisir_tarif<'a>(
    tarifs: &'a [TarifCandidat],
    article_id: Uuid,
    niveau: Option<&str>,
    regime: &str,
    qf: Option<i64>,
) -> Option<&'a TarifCandidat> {
    tarifs
        .iter()
        .filter(|t| t.article_id == article_id)
        .filter(|t| match &t.niveau {
            None => true,
            Some(n) => niveau.is_some_and(|x| x.eq_ignore_ascii_case(n)),
        })
        .filter(|t| t.regime.as_deref().is_none_or(|r| r == regime))
        .filter(|t| match (t.qf_min, t.qf_max, qf) {
            (None, None, _) => true,
            (_, max, None) => max.is_none(),
            (min, max, Some(q)) => min.is_none_or(|m| q >= m) && max.is_none_or(|m| q < m),
        })
        .max_by_key(|t| {
            (
                t.niveau.is_some() as u8,
                t.regime.is_some() as u8,
                (t.qf_min.is_some() || t.qf_max.is_some()) as u8,
            )
        })
}

/// Le régime fait-il payer cette catégorie d'article ?
pub fn categorie_due(categorie: &str, regime: &str) -> bool {
    match categorie {
        "scolarite" | "frais" => true,
        "restauration" => regime == "demi_pension" || regime == "interne",
        "internat" => regime == "interne",
        // Activités, bourses, réductions : jamais automatiques depuis la grille.
        _ => false,
    }
}

/// Palier de relance à proposer pour un retard donné : le plus haut palier
/// atteint et pas encore proposé. Jamais avant l'échéance (`retard <= 0`).
pub fn palier_a_proposer(retard_jours: i64, paliers: &[(i16, i16)], deja: &[i16]) -> Option<i16> {
    if retard_jours <= 0 {
        return None;
    }
    paliers
        .iter()
        .filter(|(_, j)| retard_jours >= *j as i64)
        .map(|(r, _)| *r)
        .max()
        .filter(|r| !deja.iter().any(|d| d >= r))
}

/// Codes de rejet SEPA courants (ISO 20022, liste de travail [NV] : à vérifier
/// sur le guide d'implémentation de la banque) et suite proposée : `true` si
/// la re-présentation n'a pas de sens (compte clos, mandat absent…).
pub fn motif_rejet(code: &str) -> Option<(&'static str, bool)> {
    Some(match code.trim().to_ascii_uppercase().as_str() {
        "AC01" => ("Numéro de compte incorrect", true),
        "AC04" => ("Compte clôturé", true),
        "AC06" => ("Compte bloqué", true),
        "AG01" => ("Prélèvement interdit sur ce compte", true),
        "AM04" => ("Provision insuffisante", false),
        "AM05" => ("Opération en double", false),
        "MD01" => ("Absence de mandat", true),
        "MD02" => ("Données du mandat incorrectes ou manquantes", true),
        "MD06" => ("Contestation du débiteur (remboursement)", false),
        "MD07" => ("Débiteur décédé", true),
        "MS02" => ("Refus du débiteur", false),
        "MS03" => ("Raison non communiquée", false),
        "RR01" | "RR02" | "RR03" | "RR04" => ("Raison réglementaire", true),
        "SL01" => ("Service spécifique de la banque du débiteur", false),
        _ => return None,
    })
}

/// Une écriture comptable (une ligne de journal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ecriture {
    pub journal: String,
    pub numero: u32,
    pub date: NaiveDate,
    pub compte: String,
    pub auxiliaire: Option<String>,
    pub piece: String,
    pub libelle: String,
    pub debit: i64,
    pub credit: i64,
    pub lettrage: Option<String>,
}

/// Contrôle d'équilibre : total des débits = total des crédits, par écriture et au total.
pub fn equilibre(ecritures: &[Ecriture]) -> Result<(i64, i64), u32> {
    let mut par_num: std::collections::BTreeMap<u32, i64> = Default::default();
    for e in ecritures {
        *par_num.entry(e.numero).or_default() += e.debit - e.credit;
    }
    if let Some((n, _)) = par_num.iter().find(|(_, s)| **s != 0) {
        return Err(*n);
    }
    Ok((
        ecritures.iter().map(|e| e.debit).sum(),
        ecritures.iter().map(|e| e.credit).sum(),
    ))
}

fn montant_fec(c: i64) -> String {
    format!("{},{:02}", c / 100, (c % 100).abs())
}

fn champ(s: &str) -> String {
    // Séparateur `|` : retiré des libellés (ainsi que les retours à la ligne).
    s.chars()
        .map(|c| {
            if c == '|' || c == '\n' || c == '\r' {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// En-tête du format `scolia_ecritures_v1` (colonnes reprises du FEC [NV]).
pub const ENTETE_ECRITURES: &str = "JournalCode|JournalLib|EcritureNum|EcritureDate|CompteNum|CompteLib|CompAuxNum|CompAuxLib|PieceRef|PieceDate|EcritureLib|Debit|Credit|EcritureLet|DateLet|ValidDate|Montantdevise|Idevise";

/// Rendu `scolia_ecritures_v1` : UTF-8, séparateur `|`, montants en euros
/// avec virgule décimale, dates `AAAAMMJJ`, une ligne par écriture.
pub fn rendre_ecritures(ecritures: &[Ecriture], validation: NaiveDate) -> String {
    let mut out = String::from(ENTETE_ECRITURES);
    out.push('\n');
    for e in ecritures {
        let journal_lib = match e.journal.as_str() {
            j if j.starts_with('B') => "Banque",
            _ => "Ventes",
        };
        let aux = e.auxiliaire.clone().unwrap_or_default();
        out.push_str(&format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}||\n",
            champ(&e.journal),
            journal_lib,
            e.numero,
            e.date.format("%Y%m%d"),
            champ(&e.compte),
            if e.compte.starts_with("411") {
                "Familles"
            } else if e.compte.starts_with("512") {
                "Banque"
            } else {
                "Produits"
            },
            champ(&aux),
            if aux.is_empty() { "" } else { "Foyer" },
            champ(&e.piece),
            e.date.format("%Y%m%d"),
            champ(&e.libelle),
            montant_fec(e.debit),
            montant_fec(e.credit),
            e.lettrage.as_deref().map(champ).unwrap_or_default(),
            "",
            validation.format("%Y%m%d"),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn calendrier_et_prorata_au_mois() {
        let c = Calendrier::new("2026-2027", 9, 10).unwrap();
        assert_eq!(c.mois(0), (2026, 9));
        assert_eq!(c.mois(4), (2027, 1));
        assert_eq!(c.dernier_jour(), d(2027, 6, 30));
        assert_eq!(
            c.mois_dus_depuis(d(2026, 8, 20)),
            10,
            "avant la rentrée : année entière"
        );
        assert_eq!(c.mois_dus_depuis(d(2026, 9, 1)), 10);
        assert_eq!(
            c.mois_dus_depuis(d(2026, 11, 30)),
            8,
            "novembre commencé : dû"
        );
        assert_eq!(c.mois_dus_depuis(d(2027, 7, 1)), 0, "après la fin : rien");
        assert_eq!(c.mois_dus_jusqu_a(d(2026, 12, 15)), 4, "sept → déc inclus");
        assert_eq!(c.mois_dus_jusqu_a(d(2026, 8, 1)), 0);
        assert_eq!(c.mois_dus_jusqu_a(d(2027, 9, 1)), 10);
        assert!(Calendrier::new("2026-2028", 9, 10).is_none());
        assert!(Calendrier::new("abc", 9, 10).is_none());
        // 1 000,00 € sur 10 mois, entrée en novembre : 800,00 €.
        assert_eq!(prorata(100_000, 8, 10), 80_000);
        // Arrondi inférieur : 1 000,01 € × 3 / 7.
        assert_eq!(prorata(100_001, 3, 7), 42_857);
    }

    #[test]
    fn facture_plus_avoir_egale_le_du() {
        let c = Calendrier::new("2026-2027", 9, 10).unwrap();
        for annuel in [99_999_i64, 123_457, 1, 0, 3_333_333] {
            let facture = prorata(annuel, c.mois_dus_depuis(d(2026, 10, 3)), 10);
            for sortie in [d(2026, 10, 3), d(2027, 2, 28), d(2027, 6, 30)] {
                let du = prorata(
                    annuel,
                    c.mois_dus_jusqu_a(sortie) - (10 - c.mois_dus_depuis(d(2026, 10, 3))),
                    10,
                );
                let avoir = facture - du;
                assert!(avoir >= 0);
                assert_eq!(facture - avoir, du);
            }
        }
    }

    #[test]
    fn repartition_exacte_plus_fort_reste() {
        let r = repartir(100_001, &[("a", 50), ("b", 50)]);
        assert_eq!(r, vec![("a", 50_001), ("b", 50_000)]);
        let r = repartir(10_000, &[("a", 33), ("b", 33), ("c", 34)]);
        assert_eq!(r.iter().map(|x| x.1).sum::<i64>(), 10_000);
        assert_eq!(r, vec![("a", 3_300), ("b", 3_300), ("c", 3_400)]);
        let r = repartir(1, &[("a", 70), ("b", 30)]);
        assert_eq!(r, vec![("a", 1), ("b", 0)]);
        // Négatif (avoir, réduction) : symétrique.
        let r = repartir(-100_001, &[("a", 50), ("b", 50)]);
        assert_eq!(r, vec![("a", -50_001), ("b", -50_000)]);
        for m in [0_i64, 1, 7, 99, 100, 12_345, 99_999_999] {
            for parts in [
                vec![(1, 60), (2, 40)],
                vec![(1, 1), (2, 1), (3, 98)],
                vec![(1, 100)],
            ] {
                assert_eq!(repartir(m, &parts).iter().map(|x| x.1).sum::<i64>(), m);
            }
        }
        assert_eq!(repartir(500, &[("a", 0)]), vec![("a", 500)]);
        assert!(repartir::<u8>(500, &[]).is_empty());
    }

    #[test]
    fn echeances_reste_sur_la_premiere() {
        assert_eq!(echeances(100_000, 3), vec![33_334, 33_333, 33_333]);
        assert_eq!(echeances(10, 10), vec![1; 10]);
        assert_eq!(echeances(7, 10), vec![7, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(echeances(5, 0), Vec::<i64>::new());
        let c = Calendrier::new("2026-2027", 9, 10).unwrap();
        let dates = c.echeances_par_defaut(d(2027, 2, 20), 5);
        assert_eq!(dates.first(), Some(&d(2027, 2, 5)));
        assert_eq!(dates.len(), 5);
        assert_eq!(
            c.echeances_par_defaut(d(2028, 1, 1), 5),
            vec![d(2027, 6, 5)]
        );
    }

    #[test]
    fn taux_et_regimes() {
        assert_eq!(taux(100_000, 1_000), 10_000);
        assert_eq!(taux(99_999, 1_500), 14_999);
        assert_eq!(taux(100, 20_000), 100, "borné à 100 %");
        assert_eq!(regime_normalise(Some("DP")), "demi_pension");
        assert_eq!(regime_normalise(Some("INT")), "interne");
        assert_eq!(regime_normalise(None), "externe");
        assert!(categorie_due("restauration", "demi_pension"));
        assert!(!categorie_due("restauration", "externe"));
        assert!(categorie_due("internat", "interne"));
        assert!(!categorie_due("activite", "interne"));
    }

    #[test]
    fn tarif_le_plus_specifique_et_quotient() {
        let a = Uuid::from_u128(1);
        let t = |id: u128,
                 niveau: Option<&str>,
                 regime: Option<&str>,
                 min: Option<i64>,
                 max: Option<i64>,
                 m: i64| TarifCandidat {
            id: Uuid::from_u128(id),
            article_id: a,
            niveau: niveau.map(Into::into),
            regime: regime.map(Into::into),
            qf_min: min,
            qf_max: max,
            montant_annuel: m,
        };
        let grille = vec![
            t(10, None, None, None, None, 100_000),
            t(11, Some("6e"), None, None, None, 90_000),
            t(12, None, None, Some(0), Some(50_000), 60_000),
            t(13, None, None, Some(50_000), None, 110_000),
        ];
        let id = |o: Option<&TarifCandidat>| o.map(|x| x.id.as_u128());
        assert_eq!(
            id(choisir_tarif(&grille, a, Some("6E"), "externe", None)),
            Some(11)
        );
        assert_eq!(
            id(choisir_tarif(
                &grille,
                a,
                Some("5e"),
                "externe",
                Some(20_000)
            )),
            Some(12)
        );
        assert_eq!(
            id(choisir_tarif(
                &grille,
                a,
                Some("5e"),
                "externe",
                Some(50_000)
            )),
            Some(13),
            "borne haute exclue"
        );
        assert_eq!(
            id(choisir_tarif(&grille, a, Some("5e"), "externe", None)),
            Some(13),
            "sans quotient : plein tarif"
        );
        assert_eq!(
            choisir_tarif(&grille, Uuid::from_u128(2), None, "externe", None),
            None
        );
    }

    #[test]
    fn paliers_de_relance() {
        let p = [(1_i16, 7_i16), (2, 30), (3, 60)];
        assert_eq!(
            palier_a_proposer(0, &p, &[]),
            None,
            "jamais avant l'échéance"
        );
        assert_eq!(palier_a_proposer(6, &p, &[]), None);
        assert_eq!(palier_a_proposer(7, &p, &[]), Some(1));
        assert_eq!(palier_a_proposer(45, &p, &[1]), Some(2));
        assert_eq!(palier_a_proposer(45, &p, &[2]), None, "déjà proposé");
        assert_eq!(
            palier_a_proposer(90, &p, &[]),
            Some(3),
            "saute au palier atteint"
        );
    }

    #[test]
    fn motifs_de_rejet() {
        assert_eq!(motif_rejet("am04"), Some(("Provision insuffisante", false)));
        assert_eq!(motif_rejet("AC04").map(|m| m.1), Some(true));
        assert_eq!(motif_rejet("ZZ99"), None);
    }

    #[test]
    fn ecritures_equilibrees_et_rendu() {
        let e = |n, compte: &str, debit, credit| Ecriture {
            journal: "VT".into(),
            numero: n,
            date: d(2026, 9, 1),
            compte: compte.into(),
            auxiliaire: Some("F1".into()),
            piece: "F2026-000001".into(),
            libelle: "Scolarité | 2026".into(),
            debit,
            credit,
            lettrage: None,
        };
        let ok = vec![e(1, "411000", 10_050, 0), e(1, "706000", 0, 10_050)];
        assert_eq!(equilibre(&ok), Ok((10_050, 10_050)));
        assert_eq!(equilibre(&[e(2, "411000", 1, 0)]), Err(2));
        let txt = rendre_ecritures(&ok, d(2026, 10, 1));
        let lignes: Vec<&str> = txt.lines().collect();
        assert_eq!(lignes[0], ENTETE_ECRITURES);
        assert_eq!(lignes[1].split('|').count(), 18);
        assert!(lignes[1].contains("|100,50|0,00|"));
        assert!(
            lignes[1].contains("Scolarité   2026"),
            "séparateur retiré du libellé"
        );
    }
}
