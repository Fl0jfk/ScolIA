//! Assiduité (socle 2, lot S2-4, D-020) : dérivation **pure** des demi-journées
//! d'absence à partir des absences à la séance. Aucune base ici : la fonction est
//! déterministe et testée sur les cas limites d'horaires.
//!
//! Règles (toutes paramétrables, `ent_vs_regle_assiduite`, **à vérifier sur les
//! textes officiels avant pilote** — D-019) :
//! - la journée est coupée à `heure_coupure` (défaut 12:00) ; une séance à cheval
//!   est ventilée minute par minute entre le matin et l'après-midi ;
//! - une demi-journée « compte » quand les minutes d'absence cumulées (union des
//!   séances : deux séances qui se chevauchent ne sont pas comptées deux fois)
//!   atteignent `seuil_minutes` ;
//! - elle est « non justifiée » quand les minutes **non justifiées** atteignent le
//!   seuil (une absence à traiter, refusée, ou sans décision compte comme non
//!   justifiée jusqu'à décision contraire) ;
//! - une absence annulée (« remise présent ») ne contribue jamais.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

/// Minutes dans une demi-journée « entière » (absence historique à la journée) :
/// toujours au-dessus de tout seuil admis (≤ 600).
pub(crate) const MINUTES_DEMI_JOURNEE_ENTIERE: i64 = 24 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Periode {
    Matin,
    ApresMidi,
}

impl Periode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Matin => "matin",
            Self::ApresMidi => "apres_midi",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "matin" => Some(Self::Matin),
            "apres_midi" => Some(Self::ApresMidi),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegleAssiduite {
    pub heure_coupure: NaiveTime,
    pub seuil_minutes: i32,
    pub seuil_signalement: i32,
    pub delai_verification_minutes: i32,
}

impl Default for RegleAssiduite {
    fn default() -> Self {
        Self {
            heure_coupure: NaiveTime::from_hms_opt(12, 0, 0).expect("12:00"),
            seuil_minutes: 30,
            seuil_signalement: 4,
            delai_verification_minutes: 15,
        }
    }
}

impl RegleAssiduite {
    /// Bornes admises (miroir des contraintes SQL) ; refuse sinon.
    pub fn valider(&self) -> bool {
        (1..=600).contains(&self.seuil_minutes)
            && (1..=62).contains(&self.seuil_signalement)
            && (0..=240).contains(&self.delai_verification_minutes)
    }
}

/// Ce qu'une ligne d'absence apporte à un jour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contribution {
    /// Absence à une séance horodatée.
    Seance {
        debut: NaiveTime,
        fin: NaiveTime,
        justifiee: bool,
    },
    /// Absence historique à la journée : la demi-journée entière.
    DemiJournee { periode: Periode, justifiee: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemiJournee {
    pub periode: Periode,
    /// Minutes d'absence (union) dans la demi-journée.
    pub minutes: i64,
    pub minutes_non_justifiees: i64,
    /// La demi-journée atteint le seuil : elle est comptée comme absente.
    pub comptee: bool,
    /// Comptée **et** non justifiée : c'est celle du décompte réglementaire.
    pub non_justifiee: bool,
}

pub(crate) type Intervalle = (i64, i64); // secondes depuis minuit, [début, fin)

pub(crate) fn secondes(t: NaiveTime) -> i64 {
    use chrono::Timelike;
    i64::from(t.num_seconds_from_midnight())
}

/// Union d'intervalles, longueur totale en minutes (arrondie à l'inférieur).
pub(crate) fn minutes_union(mut v: Vec<Intervalle>) -> i64 {
    v.retain(|(d, f)| f > d);
    v.sort_unstable();
    let mut total = 0i64;
    let mut courant: Option<Intervalle> = None;
    for (d, f) in v {
        match courant {
            Some((cd, cf)) if d <= cf => courant = Some((cd, cf.max(f))),
            Some((cd, cf)) => {
                total += cf - cd;
                courant = Some((d, f));
            }
            None => courant = Some((d, f)),
        }
    }
    if let Some((cd, cf)) = courant {
        total += cf - cd;
    }
    total / 60
}

/// Découpe `[debut, fin)` selon la coupure : (partie matin, partie après-midi).
pub(crate) fn ventiler(debut: i64, fin: i64, coupure: i64) -> (Option<Intervalle>, Option<Intervalle>) {
    let matin = (debut.min(coupure), fin.min(coupure));
    let apres = (debut.max(coupure), fin.max(coupure));
    (
        (matin.1 > matin.0).then_some(matin),
        (apres.1 > apres.0).then_some(apres),
    )
}

/// Demi-journées d'un jour pour un élève. Ne renvoie que les demi-journées qui
/// ont reçu une contribution (une demi-journée sans absence n'apparaît pas).
pub fn deriver_demi_journees(
    contributions: &[Contribution],
    regle: &RegleAssiduite,
) -> Vec<DemiJournee> {
    let coupure = secondes(regle.heure_coupure);
    let seuil = i64::from(regle.seuil_minutes);
    let mut out = Vec::new();
    for periode in [Periode::Matin, Periode::ApresMidi] {
        let mut tous: Vec<Intervalle> = Vec::new();
        let mut injustifies: Vec<Intervalle> = Vec::new();
        let mut entiere = false;
        let mut entiere_injustifiee = false;
        let mut touchee = false;
        for c in contributions {
            match *c {
                Contribution::Seance {
                    debut,
                    fin,
                    justifiee,
                } => {
                    let (m, a) = ventiler(secondes(debut), secondes(fin), coupure);
                    let part = if periode == Periode::Matin { m } else { a };
                    if let Some(iv) = part {
                        touchee = true;
                        tous.push(iv);
                        if !justifiee {
                            injustifies.push(iv);
                        }
                    }
                }
                Contribution::DemiJournee {
                    periode: p,
                    justifiee,
                } if p == periode => {
                    touchee = true;
                    entiere = true;
                    if !justifiee {
                        entiere_injustifiee = true;
                    }
                }
                Contribution::DemiJournee { .. } => {}
            }
        }
        if !touchee {
            continue;
        }
        let mut minutes = minutes_union(tous);
        let mut non_just = minutes_union(injustifies);
        if entiere {
            minutes = minutes.max(MINUTES_DEMI_JOURNEE_ENTIERE);
        }
        if entiere_injustifiee {
            non_just = non_just.max(MINUTES_DEMI_JOURNEE_ENTIERE);
        }
        let comptee = minutes >= seuil;
        out.push(DemiJournee {
            periode,
            minutes,
            minutes_non_justifiees: non_just,
            comptee,
            non_justifiee: comptee && non_just >= seuil,
        });
    }
    out
}

/// Le créneau `[debut, fin]` est-il **entièrement** couvert par la période
/// annoncée (déclaration) ce jour-là ? Heure absente = début / fin de journée ;
/// bornes incluses (08:00–09:00 est couvert par une déclaration 08:00–09:00,
/// 09:00–10:00 ne l'est pas).
#[allow(clippy::too_many_arguments)]
pub fn seance_couverte(
    jour: NaiveDate,
    debut: NaiveTime,
    fin: NaiveTime,
    decl_debut_jour: NaiveDate,
    decl_debut_heure: Option<NaiveTime>,
    decl_fin_jour: NaiveDate,
    decl_fin_heure: Option<NaiveTime>,
) -> bool {
    if jour < decl_debut_jour || jour > decl_fin_jour {
        return false;
    }
    if jour == decl_debut_jour {
        if let Some(h) = decl_debut_heure {
            if debut < h {
                return false;
            }
        }
    }
    if jour == decl_fin_jour {
        if let Some(h) = decl_fin_heure {
            if fin > h {
                return false;
            }
        }
    }
    true
}

/// Premier jour du mois contenant `jour`, et premier jour du mois suivant.
pub fn bornes_mois(jour: NaiveDate) -> (NaiveDate, NaiveDate) {
    let debut = jour.with_day(1).expect("1er du mois");
    let suivant = if debut.month() == 12 {
        NaiveDate::from_ymd_opt(debut.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(debut.year(), debut.month() + 1, 1)
    }
    .expect("mois suivant");
    (debut, suivant)
}

/// Heure légale de Paris (UTC+1 l'hiver, UTC+2 du dernier dimanche de mars
/// 01:00 UTC au dernier dimanche d'octobre 01:00 UTC). Les horaires de l'EDT sont
/// des heures locales d'établissement : toute comparaison « maintenant » les
/// utilise (le serveur reste en UTC).
pub fn heure_paris(maintenant: DateTime<Utc>) -> NaiveDateTime {
    fn dernier_dimanche(annee: i32, mois: u32) -> NaiveDate {
        let suivant = if mois == 12 {
            NaiveDate::from_ymd_opt(annee + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(annee, mois + 1, 1)
        }
        .expect("date");
        let mut d = suivant - Duration::days(1);
        while d.weekday().num_days_from_sunday() != 0 {
            d -= Duration::days(1);
        }
        d
    }
    let annee = maintenant.year();
    let debut_ete = dernier_dimanche(annee, 3)
        .and_hms_opt(1, 0, 0)
        .expect("heure");
    let fin_ete = dernier_dimanche(annee, 10)
        .and_hms_opt(1, 0, 0)
        .expect("heure");
    let utc = maintenant.naive_utc();
    let decalage = if utc >= debut_ete && utc < fin_ete { 2 } else { 1 };
    utc + Duration::hours(decalage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }
    fn d(y: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, j).unwrap()
    }
    fn seance(a: (u32, u32), b: (u32, u32), justifiee: bool) -> Contribution {
        Contribution::Seance {
            debut: t(a.0, a.1),
            fin: t(b.0, b.1),
            justifiee,
        }
    }

    #[test]
    fn une_seance_du_matin_compte_une_demi_journee() {
        let r = RegleAssiduite::default();
        let dj = deriver_demi_journees(&[seance((8, 0), (8, 55), false)], &r);
        assert_eq!(dj.len(), 1);
        assert_eq!(dj[0].periode, Periode::Matin);
        assert_eq!(dj[0].minutes, 55);
        assert!(dj[0].comptee && dj[0].non_justifiee);
    }

    #[test]
    fn matin_et_apres_midi_sont_deux_demi_journees() {
        let r = RegleAssiduite::default();
        let dj = deriver_demi_journees(
            &[seance((9, 0), (10, 0), false), seance((14, 0), (15, 0), false)],
            &r,
        );
        assert_eq!(dj.len(), 2);
        assert_eq!(dj[0].periode, Periode::Matin);
        assert_eq!(dj[1].periode, Periode::ApresMidi);
        assert_eq!(dj.iter().filter(|x| x.non_justifiee).count(), 2);
    }

    #[test]
    fn seance_a_cheval_sur_la_coupure_est_ventilee() {
        // 11:30–12:30 : 30 min le matin, 30 min l'après-midi, seuil 30 → les deux.
        let r = RegleAssiduite::default();
        let dj = deriver_demi_journees(&[seance((11, 30), (12, 30), false)], &r);
        assert_eq!(dj.len(), 2);
        assert_eq!((dj[0].minutes, dj[1].minutes), (30, 30));
        assert!(dj[0].comptee && dj[1].comptee);
        // Seuil 31 : aucune des deux moitiés n'atteint le seuil.
        let r = RegleAssiduite {
            seuil_minutes: 31,
            ..RegleAssiduite::default()
        };
        let dj = deriver_demi_journees(&[seance((11, 30), (12, 30), false)], &r);
        assert!(dj.iter().all(|x| !x.comptee && !x.non_justifiee));
    }

    #[test]
    fn la_coupure_est_exclusive_cote_matin() {
        // 11:00–12:00 est entièrement du matin ; 12:00–13:00 entièrement après-midi.
        let r = RegleAssiduite::default();
        let m = deriver_demi_journees(&[seance((11, 0), (12, 0), false)], &r);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].periode, Periode::Matin);
        let a = deriver_demi_journees(&[seance((12, 0), (13, 0), false)], &r);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].periode, Periode::ApresMidi);
    }

    #[test]
    fn coupure_parametrable() {
        let r = RegleAssiduite {
            heure_coupure: t(12, 30),
            ..RegleAssiduite::default()
        };
        let dj = deriver_demi_journees(&[seance((12, 0), (13, 0), false)], &r);
        assert_eq!(dj.len(), 2);
        assert_eq!((dj[0].minutes, dj[1].minutes), (30, 30));
    }

    #[test]
    fn seuil_non_atteint_ne_compte_pas() {
        let r = RegleAssiduite::default();
        let dj = deriver_demi_journees(&[seance((8, 0), (8, 29), false)], &r);
        assert_eq!(dj.len(), 1);
        assert!(!dj[0].comptee && !dj[0].non_justifiee);
        // Exactement le seuil : compte.
        let dj = deriver_demi_journees(&[seance((8, 0), (8, 30), false)], &r);
        assert!(dj[0].comptee);
    }

    #[test]
    fn minutes_cumulees_sur_plusieurs_seances() {
        let r = RegleAssiduite::default();
        let dj = deriver_demi_journees(
            &[seance((8, 0), (8, 20), false), seance((9, 0), (9, 20), false)],
            &r,
        );
        assert_eq!(dj[0].minutes, 40);
        assert!(dj[0].non_justifiee);
    }

    #[test]
    fn chevauchement_non_double_compte() {
        let r = RegleAssiduite::default();
        let dj = deriver_demi_journees(
            &[seance((8, 0), (9, 0), false), seance((8, 30), (9, 30), false)],
            &r,
        );
        assert_eq!(dj[0].minutes, 90);
        // Doublon exact.
        let dj = deriver_demi_journees(
            &[seance((8, 0), (8, 40), false), seance((8, 0), (8, 40), false)],
            &r,
        );
        assert_eq!(dj[0].minutes, 40);
    }

    #[test]
    fn justification_partielle() {
        let r = RegleAssiduite::default();
        // Une séance justifiée, une non : les minutes non justifiées décident.
        let dj = deriver_demi_journees(
            &[seance((8, 0), (9, 0), true), seance((9, 0), (10, 0), false)],
            &r,
        );
        assert_eq!(dj[0].minutes, 120);
        assert_eq!(dj[0].minutes_non_justifiees, 60);
        assert!(dj[0].non_justifiee);
        // Tout justifié : comptée mais pas dans le décompte réglementaire.
        let dj = deriver_demi_journees(&[seance((8, 0), (9, 0), true)], &r);
        assert!(dj[0].comptee && !dj[0].non_justifiee);
        // Non justifiée sous le seuil, justifiée au-dessus : justifiée.
        let dj = deriver_demi_journees(
            &[seance((8, 0), (9, 0), true), seance((9, 0), (9, 10), false)],
            &r,
        );
        assert!(dj[0].comptee && !dj[0].non_justifiee);
    }

    #[test]
    fn absence_historique_a_la_journee() {
        let r = RegleAssiduite::default();
        let dj = deriver_demi_journees(
            &[
                Contribution::DemiJournee {
                    periode: Periode::Matin,
                    justifiee: false,
                },
                Contribution::DemiJournee {
                    periode: Periode::ApresMidi,
                    justifiee: true,
                },
            ],
            &r,
        );
        assert_eq!(dj.len(), 2);
        assert!(dj[0].non_justifiee);
        assert!(dj[1].comptee && !dj[1].non_justifiee);
    }

    #[test]
    fn historique_et_seance_le_meme_jour_ne_se_cumulent_pas_en_double() {
        let r = RegleAssiduite::default();
        let dj = deriver_demi_journees(
            &[
                Contribution::DemiJournee {
                    periode: Periode::Matin,
                    justifiee: false,
                },
                seance((8, 0), (9, 0), false),
            ],
            &r,
        );
        assert_eq!(dj.len(), 1);
        assert_eq!(dj[0].minutes, MINUTES_DEMI_JOURNEE_ENTIERE);
    }

    #[test]
    fn aucune_contribution_aucune_demi_journee() {
        assert!(deriver_demi_journees(&[], &RegleAssiduite::default()).is_empty());
    }

    #[test]
    fn regle_valide_bornes() {
        let mut r = RegleAssiduite::default();
        assert!(r.valider());
        r.seuil_minutes = 0;
        assert!(!r.valider());
        r.seuil_minutes = 30;
        r.seuil_signalement = 63;
        assert!(!r.valider());
    }

    #[test]
    fn couverture_par_declaration() {
        let j = d(2026, 11, 5);
        // Journée entière annoncée.
        assert!(seance_couverte(j, t(8, 0), t(9, 0), j, None, j, None));
        // Jour hors période.
        assert!(!seance_couverte(j, t(8, 0), t(9, 0), d(2026, 11, 6), None, d(2026, 11, 7), None));
        // Bornes incluses.
        assert!(seance_couverte(j, t(8, 0), t(9, 0), j, Some(t(8, 0)), j, Some(t(9, 0))));
        // Séance qui commence avant l'heure de début annoncée.
        assert!(!seance_couverte(j, t(8, 0), t(9, 0), j, Some(t(8, 30)), j, None));
        // Séance qui finit après l'heure de fin annoncée.
        assert!(!seance_couverte(j, t(9, 0), t(10, 0), j, None, j, Some(t(9, 30))));
        // Période sur plusieurs jours : heures de début/fin ne pèsent que sur les extrémités.
        let (a, b) = (d(2026, 11, 4), d(2026, 11, 6));
        assert!(seance_couverte(j, t(8, 0), t(9, 0), a, Some(t(15, 0)), b, Some(t(10, 0))));
        assert!(!seance_couverte(a, t(8, 0), t(9, 0), a, Some(t(15, 0)), b, Some(t(10, 0))));
        assert!(!seance_couverte(b, t(10, 0), t(11, 0), a, Some(t(15, 0)), b, Some(t(10, 0))));
    }

    #[test]
    fn bornes_de_mois() {
        assert_eq!(bornes_mois(d(2026, 11, 17)), (d(2026, 11, 1), d(2026, 12, 1)));
        assert_eq!(bornes_mois(d(2026, 12, 31)), (d(2026, 12, 1), d(2027, 1, 1)));
        assert_eq!(bornes_mois(d(2028, 2, 29)), (d(2028, 2, 1), d(2028, 3, 1)));
    }

    #[test]
    fn heure_de_paris_ete_et_hiver() {
        let utc = |y, m, j, h, mi| {
            NaiveDate::from_ymd_opt(y, m, j)
                .unwrap()
                .and_hms_opt(h, mi, 0)
                .unwrap()
                .and_utc()
        };
        // Hiver : UTC+1.
        assert_eq!(heure_paris(utc(2026, 11, 5, 7, 0)).time(), t(8, 0));
        // Été : UTC+2.
        assert_eq!(heure_paris(utc(2026, 7, 1, 7, 0)).time(), t(9, 0));
        // Bascules 2026 : 29 mars 01:00 UTC, 25 octobre 01:00 UTC.
        assert_eq!(heure_paris(utc(2026, 3, 29, 0, 59)).time(), t(1, 59));
        assert_eq!(heure_paris(utc(2026, 3, 29, 1, 0)).time(), t(3, 0));
        assert_eq!(heure_paris(utc(2026, 10, 25, 0, 59)).time(), t(2, 59));
        assert_eq!(heure_paris(utc(2026, 10, 25, 1, 0)).time(), t(2, 0));
        // Passage de minuit.
        assert_eq!(heure_paris(utc(2026, 11, 5, 23, 30)).date(), d(2026, 11, 6));
    }
}
