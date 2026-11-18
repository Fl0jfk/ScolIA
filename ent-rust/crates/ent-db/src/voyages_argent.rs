//! Socle 42, lot V2a : règles **pures** de l'argent en mode suivi (R-SV44, `42_SORTIES_VOYAGES.md` §12.5, §32.5).
//!
//! Aucune base, aucune horloge : les dates arrivent en paramètre (le jour est celui de l'établissement, calculé
//! par l'appelant). Tout est en **centimes entiers** ; les opérations sont contrôlées (`checked_*`), un dépassement
//! est une erreur et jamais un arrondi silencieux.
//!
//! * répartition d'une part entre les payeurs (plus fort reste, somme exacte) ;
//! * échéancier proposé (acompte, intermédiaires, solde au plus tard à J-30) et plan long ;
//! * situation d'un payeur : soldé, à jour, en retard, plan en cours ;
//! * avoir **proposé** (désistement, baisse de prix, annulation) : un montant, jamais une écriture comptable ;
//! * statut d'un acompte fournisseur ;
//! * ligne de CSV pour la comptabilité.

use chrono::{Duration, NaiveDate};
use uuid::Uuid;

use crate::finances_calcul::repartir;

/// Moyens de versement pointables (VOY-267).
pub const MOYENS: [&str; 4] = ["cheque", "especes", "virement", "cheques_vacances"];

/// Un versement ne dépasse pas 100 000 € (même borne que la base).
pub const VERSEMENT_MAX: i64 = 10_000_000;

/// Un plan standard compte au plus 3 échéances ; au-delà, il est **accordé** par la comptabilité (§12.2).
pub const ECHEANCES_STANDARD_MAX: i32 = 3;
pub const ECHEANCES_MAX: i32 = 10;

/// Part d'un payeur : `(compte, centimes)`. La somme vaut exactement `montant`.
pub fn repartir_entre_payeurs(montant: i64, parts_bps: &[(Uuid, i64)]) -> Vec<(Uuid, i64)> {
    repartir(montant, parts_bps)
}

/// Une échéance proposée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Echeance {
    pub date: NaiveDate,
    pub centimes: i64,
}

/// Dernier jour du plan : **J-30** pour un plan standard (§12.2) ; le jour du départ au plus tard pour un plan long
/// accordé. Jamais avant la première échéance (une inscription tardive paie tout de suite).
pub fn derniere_echeance(premiere: NaiveDate, depart: NaiveDate, plan_long: bool) -> NaiveDate {
    let cible = if plan_long { depart } else { depart - Duration::days(30) };
    cible.max(premiere)
}

/// Échéancier de `nb` versements de `montant`, du `premiere` au `derniere` (inclus), dates réparties régulièrement.
/// Le reste de la division va à la **première** échéance (l'acompte). Deux échéances le même jour sont fusionnées.
/// `None` si `nb` est hors de 1..=10, le montant négatif ou les dates incohérentes.
pub fn echeancier(montant: i64, nb: i32, premiere: NaiveDate, derniere: NaiveDate) -> Option<Vec<Echeance>> {
    if !(1..=ECHEANCES_MAX).contains(&nb) || montant < 0 || derniere < premiere {
        return None;
    }
    let n = i64::from(nb);
    let base = montant / n;
    let reste = montant - base.checked_mul(n)?;
    let jours = (derniere - premiere).num_days();
    let mut out: Vec<Echeance> = Vec::new();
    for i in 0..n {
        let date = if n == 1 { premiere } else { premiere + Duration::days(jours * i / (n - 1)) };
        let centimes = if i == 0 { base.checked_add(reste)? } else { base };
        match out.last_mut() {
            Some(l) if l.date == date => l.centimes = l.centimes.checked_add(centimes)?,
            _ => out.push(Echeance { date, centimes }),
        }
    }
    Some(out)
}

/// État d'un payeur sur une part, à un jour donné.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatPaiement {
    /// Tout est versé.
    Solde,
    /// Rien n'est dû à ce jour au-delà de ce qui est versé.
    AJour,
    /// Un plan LONG accordé est en cours et à jour.
    PlanEnCours,
    /// Des échéances échues ne sont pas couvertes par les versements.
    EnRetard,
}

impl EtatPaiement {
    pub fn code(self) -> &'static str {
        match self {
            Self::Solde => "solde",
            Self::AJour => "a_jour",
            Self::PlanEnCours => "plan_en_cours",
            Self::EnRetard => "en_retard",
        }
    }
    /// Gravité pour agréger plusieurs payeurs d'un même élève : le pire l'emporte.
    pub fn gravite(self) -> u8 {
        match self {
            Self::Solde => 0,
            Self::AJour => 1,
            Self::PlanEnCours => 2,
            Self::EnRetard => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Situation {
    pub du_centimes: i64,
    pub verse_centimes: i64,
    pub reste_centimes: i64,
    /// Montant des échéances échues non couvertes.
    pub retard_centimes: i64,
    pub etat: EtatPaiement,
}

/// Situation à `jour` : `verse` est le **net** (versements moins annulations). Un trop-versé n'est jamais accepté
/// à la saisie, mais le calcul le tolère (reste 0).
pub fn situation(echeances: &[Echeance], verse: i64, jour: NaiveDate, plan_long: bool) -> Option<Situation> {
    let mut total = 0i64;
    let mut echu = 0i64;
    for e in echeances {
        total = total.checked_add(e.centimes)?;
        if e.date <= jour {
            echu = echu.checked_add(e.centimes)?;
        }
    }
    let reste = (total - verse).max(0);
    let retard = (echu - verse).max(0);
    let etat = if verse >= total {
        EtatPaiement::Solde
    } else if retard > 0 {
        EtatPaiement::EnRetard
    } else if plan_long {
        EtatPaiement::PlanEnCours
    } else {
        EtatPaiement::AJour
    };
    Some(Situation {
        du_centimes: total,
        verse_centimes: verse,
        reste_centimes: reste,
        retard_centimes: retard,
        etat,
    })
}

/// Cause d'un avoir proposé (§12.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasAvoir {
    /// Désistement avant l'engagement des dépenses : avoir total.
    DesistementAvantEngagement,
    /// Désistement après engagement : avoir des dépenses non engagées ; les frais réels restent dus (VOY-273).
    DesistementApresEngagement,
    /// Motif légitime (maladie, deuil, déménagement) : avoir total proposé, frais compris.
    MotifLegitime,
    /// Annulation collective par l'établissement : avoir total pour chaque payeur.
    AnnulationCollective,
    /// Départ en cours de préparation décidé par l'établissement (VOY-286) : avoir total.
    DepartEnPreparation,
    /// Baisse du prix annoncé : l'écart, dans la limite de ce qui a été payé au-delà du nouveau prix.
    BaissePrix { ancienne: i64, nouvelle: i64 },
}

impl CasAvoir {
    pub fn code(self) -> &'static str {
        match self {
            Self::DesistementAvantEngagement => "desistement_avant_engagement",
            Self::DesistementApresEngagement => "desistement_apres_engagement",
            Self::MotifLegitime => "motif_legitime",
            Self::AnnulationCollective => "annulation_collective",
            Self::DepartEnPreparation => "depart_en_preparation",
            Self::BaissePrix { .. } => "baisse_prix",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvoirCalcule {
    pub paye_net_centimes: i64,
    pub frais_retenus_centimes: i64,
    pub montant_propose_centimes: i64,
}

/// Montant d'avoir **proposé** pour un payeur. Jamais plus que ce que ce payeur a réellement versé (le
/// remboursement va au payeur qui a payé, CF-69) ; jamais négatif. `frais_reels` est la part des frais réels
/// de ce payeur (déjà répartie par l'appelant).
pub fn calculer_avoir(cas: CasAvoir, paye_net: i64, frais_reels: i64) -> AvoirCalcule {
    let paye = paye_net.max(0);
    let frais = frais_reels.max(0);
    let (montant, retenus) = match cas {
        CasAvoir::DesistementAvantEngagement
        | CasAvoir::MotifLegitime
        | CasAvoir::AnnulationCollective
        | CasAvoir::DepartEnPreparation => (paye, 0),
        CasAvoir::DesistementApresEngagement => {
            let retenus = frais.min(paye);
            (paye - retenus, retenus)
        }
        CasAvoir::BaissePrix { ancienne, nouvelle } => {
            let ecart = (ancienne - nouvelle).max(0);
            ((paye - nouvelle.max(0)).max(0).min(ecart), 0)
        }
    };
    AvoirCalcule {
        paye_net_centimes: paye,
        frais_retenus_centimes: retenus,
        montant_propose_centimes: montant,
    }
}

/// Statut d'un acompte fournisseur à `jour`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatutAcompte {
    Paye,
    Annule,
    EnRetard,
    /// Échéance dans la fenêtre d'alerte (à payer bientôt).
    Bientot,
    APayer,
}

impl StatutAcompte {
    pub fn code(self) -> &'static str {
        match self {
            Self::Paye => "paye",
            Self::Annule => "annule",
            Self::EnRetard => "en_retard",
            Self::Bientot => "bientot",
            Self::APayer => "a_payer",
        }
    }
}

pub fn statut_acompte(
    echeance: NaiveDate,
    paye: bool,
    annule: bool,
    jour: NaiveDate,
    alerte_jours: i64,
) -> StatutAcompte {
    if annule {
        StatutAcompte::Annule
    } else if paye {
        StatutAcompte::Paye
    } else if echeance < jour {
        StatutAcompte::EnRetard
    } else if (echeance - jour).num_days() <= alerte_jours {
        StatutAcompte::Bientot
    } else {
        StatutAcompte::APayer
    }
}

/// Montant en euros à la française pour l'export (`12,50`), sans flottant.
pub fn euros(centimes: i64) -> String {
    let signe = if centimes < 0 { "-" } else { "" };
    let a = centimes.unsigned_abs();
    format!("{signe}{},{:02}", a / 100, a % 100)
}

/// Analyse un montant saisi en texte (`12,50`, `12.5`, `12`) en centimes : au plus deux décimales, jamais de
/// flottant, jamais de signe, jamais de séparateur de milliers (ambigu).
pub fn centimes_de_texte(t: &str) -> Option<i64> {
    let t = t.trim().replace(',', ".");
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_digit() || c == '.') || t.matches('.').count() > 1 {
        return None;
    }
    let (e, d) = t.split_once('.').unwrap_or((&t, ""));
    if e.is_empty() || e.len() > 10 || d.len() > 2 {
        return None;
    }
    let entiers: i64 = e.parse().ok()?;
    let dec: i64 = if d.is_empty() { 0 } else { format!("{d:0<2}").parse().ok()? };
    entiers.checked_mul(100)?.checked_add(dec)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(j: u32, m: u32, a: i32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn la_repartition_entre_payeurs_somme_exactement() {
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        for montant in [0, 1, 49_999, 50_000, 123_457] {
            let v = repartir_entre_payeurs(montant, &[(a, 5_000), (b, 5_000)]);
            assert_eq!(v.iter().map(|x| x.1).sum::<i64>(), montant);
        }
        let v = repartir_entre_payeurs(50_001, &[(a, 6_000), (b, 4_000)]);
        assert_eq!(v, vec![(a, 30_001), (b, 20_000)]);
    }

    #[test]
    fn echeancier_standard_acompte_puis_solde_a_j_moins_30() {
        let premiere = d(10, 1, 2027);
        let depart = d(15, 4, 2027);
        let fin = derniere_echeance(premiere, depart, false);
        assert_eq!(fin, d(16, 3, 2027));
        let e = echeancier(50_001, 3, premiere, fin).unwrap();
        assert_eq!(e.len(), 3);
        assert_eq!(e.iter().map(|x| x.centimes).sum::<i64>(), 50_001);
        assert_eq!(e[0].date, premiere);
        assert_eq!(e[2].date, fin);
        assert_eq!(e[0].centimes, 16_667, "le reste va à l'acompte");
        assert!(e[1].date > e[0].date && e[1].date < e[2].date);
    }

    #[test]
    fn inscription_tardive_tout_est_du_tout_de_suite() {
        // Inscription à J-20 : le solde « à J-30 » est déjà passé, les échéances sont fusionnées en une.
        let premiere = d(26, 3, 2027);
        let fin = derniere_echeance(premiere, d(15, 4, 2027), false);
        assert_eq!(fin, premiere);
        let e = echeancier(30_000, 3, premiere, fin).unwrap();
        assert_eq!(e, vec![Echeance { date: premiere, centimes: 30_000 }]);
    }

    #[test]
    fn plan_long_va_jusqu_au_depart_et_bornes() {
        let premiere = d(10, 1, 2027);
        let fin = derniere_echeance(premiere, d(15, 4, 2027), true);
        assert_eq!(fin, d(15, 4, 2027));
        let e = echeancier(100_000, 10, premiere, fin).unwrap();
        assert_eq!(e.iter().map(|x| x.centimes).sum::<i64>(), 100_000);
        assert!(echeancier(100, 0, premiere, fin).is_none());
        assert!(echeancier(100, 11, premiere, fin).is_none());
        assert!(echeancier(-1, 2, premiere, fin).is_none());
        assert!(echeancier(100, 2, fin, premiere).is_none());
        assert!(echeancier(i64::MAX, 3, premiere, fin).is_some(), "pas de dépassement sur la division");
    }

    #[test]
    fn situation_solde_a_jour_retard_et_plan_en_cours() {
        let e = echeancier(30_000, 3, d(1, 1, 2027), d(1, 3, 2027)).unwrap();
        let s = situation(&e, 10_000, d(2, 1, 2027), false).unwrap();
        assert_eq!((s.etat, s.retard_centimes, s.reste_centimes), (EtatPaiement::AJour, 0, 20_000));
        let s = situation(&e, 10_000, d(2, 3, 2027), false).unwrap();
        assert_eq!((s.etat, s.retard_centimes), (EtatPaiement::EnRetard, 20_000));
        let s = situation(&e, 30_000, d(2, 3, 2027), false).unwrap();
        assert_eq!((s.etat, s.reste_centimes), (EtatPaiement::Solde, 0));
        let s = situation(&e, 10_000, d(2, 1, 2027), true).unwrap();
        assert_eq!(s.etat, EtatPaiement::PlanEnCours);
        // Un plan long en retard est en retard, pas « en cours ».
        let s = situation(&e, 0, d(2, 3, 2027), true).unwrap();
        assert_eq!(s.etat, EtatPaiement::EnRetard);
        assert!(situation(&[Echeance { date: d(1, 1, 2027), centimes: i64::MAX }, Echeance { date: d(1, 1, 2027), centimes: 1 }], 0, d(1, 1, 2027), false).is_none());
    }

    #[test]
    fn avoirs_proposes_jamais_au_dela_du_verse() {
        use CasAvoir::*;
        let c = |cas, paye, frais| calculer_avoir(cas, paye, frais).montant_propose_centimes;
        assert_eq!(c(DesistementAvantEngagement, 30_000, 9_999), 30_000);
        assert_eq!(c(DesistementApresEngagement, 30_000, 12_000), 18_000);
        assert_eq!(c(DesistementApresEngagement, 5_000, 12_000), 0, "frais plus gros que le versé : rien à rendre, jamais négatif");
        assert_eq!(c(MotifLegitime, 30_000, 12_000), 30_000, "motif légitime : avoir total (CV-71)");
        assert_eq!(c(AnnulationCollective, 30_000, 0), 30_000);
        assert_eq!(c(DepartEnPreparation, 30_000, 0), 30_000);
        assert_eq!(c(DesistementAvantEngagement, -5, 0), 0);
        let a = calculer_avoir(DesistementApresEngagement, 30_000, 12_000);
        assert_eq!((a.frais_retenus_centimes, a.paye_net_centimes), (12_000, 30_000));
    }

    #[test]
    fn baisse_de_prix_rend_seulement_le_trop_verse() {
        use CasAvoir::BaissePrix;
        let c = |paye| calculer_avoir(BaissePrix { ancienne: 50_000, nouvelle: 45_000 }, paye, 0).montant_propose_centimes;
        assert_eq!(c(50_000), 5_000, "soldé : l'écart est rendu");
        assert_eq!(c(47_000), 2_000, "versé 470 € pour un prix de 450 € : 20 € à rendre");
        assert_eq!(c(20_000), 0, "versé moins que le nouveau prix : rien à rendre");
    }

    #[test]
    fn statuts_d_acompte_fournisseur() {
        let j = d(10, 3, 2027);
        assert_eq!(statut_acompte(d(9, 3, 2027), false, false, j, 14), StatutAcompte::EnRetard);
        assert_eq!(statut_acompte(d(10, 3, 2027), false, false, j, 14), StatutAcompte::Bientot);
        assert_eq!(statut_acompte(d(24, 3, 2027), false, false, j, 14), StatutAcompte::Bientot);
        assert_eq!(statut_acompte(d(25, 3, 2027), false, false, j, 14), StatutAcompte::APayer);
        assert_eq!(statut_acompte(d(1, 1, 2027), true, false, j, 14), StatutAcompte::Paye);
        assert_eq!(statut_acompte(d(1, 1, 2027), false, true, j, 14), StatutAcompte::Annule);
    }

    #[test]
    fn montants_en_texte_et_en_euros() {
        assert_eq!(centimes_de_texte("12,50"), Some(1_250));
        assert_eq!(centimes_de_texte("12.5"), Some(1_250));
        assert_eq!(centimes_de_texte("12"), Some(1_200));
        assert_eq!(centimes_de_texte(" 0,05 "), Some(5));
        for mauvais in ["", "-3", "1 200", "1,234", "1,2,3", "abc", "1e3", ",5", "99999999999"] {
            assert_eq!(centimes_de_texte(mauvais), None, "{mauvais}");
        }
        assert_eq!(euros(1_250), "12,50");
        assert_eq!(euros(5), "0,05");
        assert_eq!(euros(0), "0,00");
    }
}
