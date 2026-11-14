//! Socle 20, lot C4 : **calcul pur** de la charge de travail par élève (`20_CAHIER_DE_TEXTES.md` §11), sans base de données.
//!
//! Entrées : les échéances d'élèves réels (évaluations annoncées, travaux), les jours ouvrés de l'établissement. Sorties :
//! par élève et par jour, nombre d'évaluations, nombre de travaux et durée cumulée du **soir** ; repères à trois positions ;
//! alertes ; dates alternatives. Aucune lecture de l'heure : tout est donné (jour de l'établissement, jours ouvrés).
//!
//! Définitions (hypothèses de calcul, à faire relire par Florian — `docs/PROJECT_STATUS.md`) :
//! * le **soir** d'un travail court est la **veille ouvrée** de son échéance (« le travail se fait la veille ») ;
//! * un travail **long** (durée > 90 min, ou échéance à plus de 7 jours de la date où il est donné, R-C32) est réparti
//!   également sur les jours ouvrés entre le jour où il est donné et la veille ouvrée de l'échéance ; la somme est conservée ;
//! * un repère est **dépassé** quand la valeur est strictement supérieure à lui (« 2 évaluations le même jour » est permis
//!   en position normale, la troisième déclenche l'alerte) ;
//! * une évaluation annoncée et le travail qui l'annonce ne comptent qu'une fois (CC-55, géré en amont par l'appelant).

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Duration, NaiveDate};
use ent_types::cahier_c4::*;
use uuid::Uuid;

/// Durée au-delà de laquelle un travail est « long » (minutes).
pub const DUREE_LONGUE_MIN: i32 = 90;
/// Délai au-delà duquel un travail est « long » (jours entre le don et l'échéance).
pub const DELAI_LONG_JOURS: i64 = 7;

/// Position choisie par l'établissement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Souple,
    Normal,
    Strict,
}

impl Position {
    pub fn code(self) -> &'static str {
        match self {
            Self::Souple => POSITION_SOUPLE,
            Self::Normal => POSITION_NORMAL,
            Self::Strict => POSITION_STRICT,
        }
    }

    /// Position d'un code ; tout ce qui n'est pas reconnu donne la position normale (défaut de l'établissement).
    pub fn depuis(code: &str) -> Self {
        match code {
            POSITION_SOUPLE => Self::Souple,
            POSITION_STRICT => Self::Strict,
            _ => Self::Normal,
        }
    }

    pub fn valide(code: &str) -> bool {
        matches!(code, POSITION_SOUPLE | POSITION_NORMAL | POSITION_STRICT)
    }
}

/// Groupe de niveaux des repères : 6ᵉ–5ᵉ, 4ᵉ–3ᵉ, lycée.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Groupe {
    Sixieme5eme,
    Quatrieme3eme,
    Lycee,
}

impl Groupe {
    pub const TOUS: [Groupe; 3] = [Groupe::Sixieme5eme, Groupe::Quatrieme3eme, Groupe::Lycee];

    pub fn code(self) -> &'static str {
        match self {
            Self::Sixieme5eme => "6e-5e",
            Self::Quatrieme3eme => "4e-3e",
            Self::Lycee => "lycee",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Sixieme5eme => "6ᵉ – 5ᵉ",
            Self::Quatrieme3eme => "4ᵉ – 3ᵉ",
            Self::Lycee => "Lycée",
        }
    }

    /// Groupe d'une classe d'après le code de son niveau. École élémentaire et 6ᵉ–5ᵉ : première colonne ; 4ᵉ–3ᵉ :
    /// deuxième ; 2de, 1re, terminale, CAP, BTS : lycée. Niveau inconnu : la colonne intermédiaire (4ᵉ–3ᵉ), ni la plus
    /// stricte ni la plus souple.
    pub fn depuis_niveau(code: Option<&str>) -> Self {
        let c: String = code.unwrap_or("").trim().to_lowercase().replace(['é', 'è', 'ê'], "e");
        let commence = |p: &[&str]| p.iter().any(|x| c.starts_with(x));
        if commence(&["cp", "ce1", "ce2", "cm1", "cm2", "gs", "ms", "ps"]) {
            return Self::Sixieme5eme;
        }
        if commence(&["2nd", "2de", "1er", "1re", "term", "tle", "bts", "cap", "bac", "prem", "sec"]) {
            return Self::Lycee;
        }
        match c.chars().next() {
            Some('6' | '5') => Self::Sixieme5eme,
            Some('4' | '3') => Self::Quatrieme3eme,
            Some('1' | '2') => Self::Lycee,
            _ => Self::Quatrieme3eme,
        }
    }
}

/// Valeurs du tableau du §11.2 : (évaluations le même jour, minutes d'un soir, évaluations sur la semaine).
pub fn reperes(position: Position, groupe: Groupe) -> ReperePositionDto {
    let (evaluations_jour, evaluations_semaine) = match position {
        Position::Souple => (3, 6),
        Position::Normal => (2, 5),
        Position::Strict => (1, 4),
    };
    let minutes_soir = match (position, groupe) {
        (Position::Souple, Groupe::Sixieme5eme) => 60,
        (Position::Souple, Groupe::Quatrieme3eme) => 90,
        (Position::Souple, Groupe::Lycee) => 150,
        (Position::Normal, Groupe::Sixieme5eme) => 45,
        (Position::Normal, Groupe::Quatrieme3eme) => 60,
        (Position::Normal, Groupe::Lycee) => 120,
        (Position::Strict, Groupe::Sixieme5eme) => 30,
        (Position::Strict, Groupe::Quatrieme3eme) => 45,
        (Position::Strict, Groupe::Lycee) => 90,
    };
    ReperePositionDto {
        groupe: groupe.code().to_string(),
        libelle: groupe.libelle().to_string(),
        evaluations_jour,
        minutes_soir,
        evaluations_semaine,
    }
}

/// Repères d'une position pour les trois groupes.
pub fn reperes_de(position: Position) -> ReperesDto {
    ReperesDto {
        position: position.code().to_string(),
        groupes: Groupe::TOUS.iter().map(|g| reperes(position, *g)).collect(),
    }
}

/// Genre d'une échéance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Genre {
    Evaluation,
    Travail,
}

impl Genre {
    pub fn code(self) -> &'static str {
        match self {
            Self::Evaluation => "evaluation",
            Self::Travail => "travail",
        }
    }
}

/// Une échéance d'un élève réel (prolongation individuelle déjà appliquée par l'appelant).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Echeance {
    pub eleve_id: Uuid,
    /// Jour de l'établissement de l'échéance.
    pub jour: NaiveDate,
    pub genre: Genre,
    /// Durée estimée en minutes (0 si inconnue ; une évaluation compte pour 0 minute de soir, on la prépare par un travail).
    pub minutes: i32,
    /// Jour où le travail a été donné (borne basse de la répartition d'un travail long).
    pub donne_le: NaiveDate,
    /// Libellé de la nature (« Exercices », « Évaluation »…), jamais un titre.
    pub nature: String,
    /// Classe du cours de l'échéance, quand il en a une : sert à décider si la matière peut être montrée.
    pub classe_id: Option<Uuid>,
    pub matiere: String,
}

/// Charge d'un élève un jour donné.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChargeJour {
    pub nb_evaluations: i32,
    pub nb_travaux: i32,
    /// Durée cumulée du **soir de ce jour** (travaux à rendre le jour ouvré suivant, travaux longs répartis).
    pub minutes_soir: i32,
}

/// Travail long : durée > 90 min, ou plus de 7 jours entre le don et l'échéance (R-C32).
pub fn est_long(minutes: i32, donne_le: NaiveDate, echeance: NaiveDate) -> bool {
    minutes > DUREE_LONGUE_MIN || (echeance - donne_le).num_days() > DELAI_LONG_JOURS
}

/// Jours ouvrés (triés) : recherche du jour ouvré précédent ou suivant sans parcourir un calendrier entier.
#[derive(Clone, Debug)]
pub struct JoursOuvres(BTreeSet<NaiveDate>);

impl JoursOuvres {
    pub fn new(jours: impl IntoIterator<Item = NaiveDate>) -> Self {
        Self(jours.into_iter().collect())
    }

    pub fn est_ouvre(&self, j: NaiveDate) -> bool {
        self.0.contains(&j)
    }

    /// Dernier jour ouvré strictement avant `j`.
    pub fn veille(&self, j: NaiveDate) -> Option<NaiveDate> {
        self.0.range(..j).next_back().copied()
    }

    /// Premier jour ouvré strictement après `j`.
    pub fn lendemain(&self, j: NaiveDate) -> Option<NaiveDate> {
        self.0.range(j + Duration::days(1)..).next().copied()
    }

    /// Jours ouvrés dans [du, au] (bornes comprises).
    pub fn entre(&self, du: NaiveDate, au: NaiveDate) -> Vec<NaiveDate> {
        if au < du {
            return Vec::new();
        }
        self.0.range(du..=au).copied().collect()
    }

    pub fn iter_apres(&self, j: NaiveDate) -> impl Iterator<Item = NaiveDate> + '_ {
        self.0.range(j + Duration::days(1)..).copied()
    }
}

/// Charge de chaque élève, jour par jour, à partir de ses échéances.
pub fn calculer(echeances: &[Echeance], ouvres: &JoursOuvres) -> BTreeMap<Uuid, BTreeMap<NaiveDate, ChargeJour>> {
    let mut out: BTreeMap<Uuid, BTreeMap<NaiveDate, ChargeJour>> = BTreeMap::new();
    for e in echeances {
        let jours = out.entry(e.eleve_id).or_default();
        let j = jours.entry(e.jour).or_default();
        match e.genre {
            Genre::Evaluation => j.nb_evaluations += 1,
            Genre::Travail => j.nb_travaux += 1,
        }
        if e.genre == Genre::Evaluation || e.minutes <= 0 {
            continue;
        }
        let veille = ouvres.veille(e.jour).unwrap_or(e.jour - Duration::days(1));
        if est_long(e.minutes, e.donne_le, e.jour) {
            let fenetre = ouvres.entre(e.donne_le, veille);
            if fenetre.is_empty() {
                jours.entry(veille).or_default().minutes_soir += e.minutes;
            } else {
                // Répartition égale ; le reste va au dernier soir pour conserver la somme.
                let n = fenetre.len() as i32;
                let part = e.minutes / n;
                let reste = e.minutes - part * n;
                for (i, soir) in fenetre.iter().enumerate() {
                    jours.entry(*soir).or_default().minutes_soir += part + if i + 1 == fenetre.len() { reste } else { 0 };
                }
            }
        } else {
            jours.entry(veille).or_default().minutes_soir += e.minutes;
        }
    }
    out
}

/// Lundi de la semaine d'un jour.
pub fn lundi_de(j: NaiveDate) -> NaiveDate {
    j - Duration::days(i64::from(j.weekday().num_days_from_monday()))
}

/// Évaluations d'un élève sur la semaine (lundi → dimanche) qui contient `jour`.
pub fn evaluations_semaine(jours: &BTreeMap<NaiveDate, ChargeJour>, jour: NaiveDate) -> i32 {
    let lundi = lundi_de(jour);
    jours.range(lundi..=lundi + Duration::days(6)).map(|(_, c)| c.nb_evaluations).sum()
}

/// Repères dépassés à un jour donné par un élève : `minutes` (le soir de ce jour), `evaluations` (le jour même),
/// `travaux` (évaluations de la semaine).
pub fn reperes_depasses(
    jours: &BTreeMap<NaiveDate, ChargeJour>,
    jour: NaiveDate,
    r: &ReperePositionDto,
) -> Vec<&'static str> {
    let c = jours.get(&jour).copied().unwrap_or_default();
    let mut v = Vec::new();
    if c.minutes_soir > r.minutes_soir {
        v.push(REPERE_MINUTES);
    }
    if c.nb_evaluations > r.evaluations_jour {
        v.push(REPERE_EVALUATIONS);
    }
    if evaluations_semaine(jours, jour) > r.evaluations_semaine {
        v.push(REPERE_SEMAINE);
    }
    v
}

/// Repères dépassés pour une **échéance** : les minutes sont celles du soir précédent (veille ouvrée), les évaluations celles
/// du jour, la semaine celle qui contient le jour.
pub fn reperes_depasses_echeance(
    jours: &BTreeMap<NaiveDate, ChargeJour>,
    ouvres: &JoursOuvres,
    jour: NaiveDate,
    r: &ReperePositionDto,
) -> Vec<&'static str> {
    let soir = ouvres.veille(jour).and_then(|v| jours.get(&v)).map(|c| c.minutes_soir).unwrap_or(0);
    let mut v: Vec<&'static str> = reperes_depasses(jours, jour, r).into_iter().filter(|x| *x != REPERE_MINUTES).collect();
    if soir > r.minutes_soir {
        v.insert(0, REPERE_MINUTES);
    }
    v
}

/// Médiane entière (inférieure en cas de nombre pair) ; 0 pour une liste vide.
pub fn mediane(mut v: Vec<i32>) -> i32 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[(v.len() - 1) / 2]
}

/// Distribution « n élèves ont la valeur v », triée par valeur.
pub fn distribution(valeurs: impl IntoIterator<Item = i32>) -> Vec<DistributionDto> {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    for v in valeurs {
        *m.entry(v).or_insert(0) += 1;
    }
    m.into_iter().map(|(valeur, nb_eleves)| DistributionDto { valeur, nb_eleves }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, j).unwrap()
    }

    /// Novembre 2026 : lundi 2 … ; jours ouvrés lundi–vendredi, sauf le mercredi 11 (férié).
    fn ouvres() -> JoursOuvres {
        JoursOuvres::new((1..=30).map(d).filter(|j| j.weekday().number_from_monday() <= 5 && j.day() != 11))
    }

    fn ev(eleve: Uuid, jour: u32, genre: Genre, minutes: i32, donne: u32) -> Echeance {
        Echeance {
            eleve_id: eleve,
            jour: d(jour),
            genre,
            minutes,
            donne_le: d(donne),
            nature: "Exercices".into(),
            classe_id: None,
            matiere: "Maths".into(),
        }
    }

    #[test]
    fn tableau_des_reperes_aux_trois_positions() {
        let r = reperes(Position::Normal, Groupe::Quatrieme3eme);
        assert_eq!((r.evaluations_jour, r.minutes_soir, r.evaluations_semaine), (2, 60, 5));
        let r = reperes(Position::Souple, Groupe::Lycee);
        assert_eq!((r.evaluations_jour, r.minutes_soir, r.evaluations_semaine), (3, 150, 6));
        let r = reperes(Position::Strict, Groupe::Sixieme5eme);
        assert_eq!((r.evaluations_jour, r.minutes_soir, r.evaluations_semaine), (1, 30, 4));
        assert_eq!(reperes_de(Position::Normal).groupes.len(), 3);
        assert_eq!(reperes_de(Position::Strict).position, "strict");
    }

    #[test]
    fn position_inconnue_donne_normal() {
        assert_eq!(Position::depuis("souple"), Position::Souple);
        assert_eq!(Position::depuis("strict"), Position::Strict);
        assert_eq!(Position::depuis("n'importe quoi"), Position::Normal);
        assert!(Position::valide("normal") && !Position::valide("extreme"));
    }

    #[test]
    fn groupe_de_niveau_depuis_le_code() {
        for (code, g) in [
            ("6E", Groupe::Sixieme5eme),
            ("5E", Groupe::Sixieme5eme),
            ("6ème", Groupe::Sixieme5eme),
            ("CM2", Groupe::Sixieme5eme),
            ("4E", Groupe::Quatrieme3eme),
            ("3E", Groupe::Quatrieme3eme),
            ("2NDE", Groupe::Lycee),
            ("2DE", Groupe::Lycee),
            ("1ERE", Groupe::Lycee),
            ("TERM", Groupe::Lycee),
            ("TLE", Groupe::Lycee),
            ("BTS1", Groupe::Lycee),
        ] {
            assert_eq!(Groupe::depuis_niveau(Some(code)), g, "{code}");
        }
        assert_eq!(Groupe::depuis_niveau(None), Groupe::Quatrieme3eme, "inconnu : colonne intermédiaire");
        assert_eq!(Groupe::depuis_niveau(Some("???")), Groupe::Quatrieme3eme);
    }

    #[test]
    fn travail_court_pese_sur_la_veille_ouvree() {
        let e = Uuid::new_v4();
        // Échéance lundi 16 : la veille ouvrée est le vendredi 13.
        let c = calculer(&[ev(e, 16, Genre::Travail, 30, 12)], &ouvres());
        let j = &c[&e];
        assert_eq!(j[&d(13)].minutes_soir, 30);
        assert_eq!(j[&d(16)].nb_travaux, 1);
        // Échéance jeudi 12 : la veille ouvrée saute le mercredi férié ? (le 11 est férié : veille du 12 = mardi 10).
        let c = calculer(&[ev(e, 12, Genre::Travail, 20, 9)], &ouvres());
        assert_eq!(c[&e][&d(10)].minutes_soir, 20);
    }

    #[test]
    fn travail_long_reparti_sur_les_jours_ouvres_somme_conservee() {
        let e = Uuid::new_v4();
        // 100 minutes (> 90) donné lundi 2, échéance vendredi 6 : veille jeudi 5 ; jours ouvrés 2, 3, 4, 5 → 25 chacun.
        let c = calculer(&[ev(e, 6, Genre::Travail, 100, 2)], &ouvres());
        let j = &c[&e];
        let soirs: Vec<i32> = (2..=5).map(|k| j[&d(k)].minutes_soir).collect();
        assert_eq!(soirs, vec![25, 25, 25, 25]);
        // 130 minutes sur 4 jours : 32 + 32 + 32 + 34 (le reste au dernier soir) = 130.
        let c = calculer(&[ev(e, 6, Genre::Travail, 130, 2)], &ouvres());
        let soirs: Vec<i32> = (2..=5).map(|k| c[&e][&d(k)].minutes_soir).collect();
        assert_eq!(soirs, vec![32, 32, 32, 34]);
        assert_eq!(soirs.iter().sum::<i32>(), 130);
        // Délai de plus de 7 jours même pour un travail court : réparti (donné lundi 2, rendu mardi 17 : 9 jours ouvrés).
        let c = calculer(&[ev(e, 17, Genre::Travail, 45, 2)], &ouvres());
        let total: i32 = c[&e].values().map(|x| x.minutes_soir).sum();
        assert_eq!(total, 45);
        assert!(c[&e].values().filter(|x| x.minutes_soir > 0).count() > 1);
        assert!(est_long(30, d(2), d(10)) && !est_long(30, d(2), d(9)) && est_long(91, d(2), d(3)) && !est_long(90, d(2), d(3)));
    }

    #[test]
    fn evaluation_compte_un_jour_et_ne_pese_pas_sur_le_soir() {
        let e = Uuid::new_v4();
        let c = calculer(&[ev(e, 19, Genre::Evaluation, 0, 2), ev(e, 19, Genre::Evaluation, 40, 2)], &ouvres());
        assert_eq!(c[&e][&d(19)].nb_evaluations, 2);
        assert_eq!(c[&e].values().map(|x| x.minutes_soir).sum::<i32>(), 0);
    }

    #[test]
    fn le_resultat_ne_depend_pas_de_l_ordre_des_saisies() {
        let e = Uuid::new_v4();
        let mut v = vec![
            ev(e, 13, Genre::Travail, 30, 9),
            ev(e, 13, Genre::Evaluation, 0, 9),
            ev(e, 16, Genre::Travail, 50, 9),
            ev(e, 20, Genre::Travail, 120, 2),
        ];
        let a = calculer(&v, &ouvres());
        v.reverse();
        assert_eq!(a, calculer(&v, &ouvres()));
        v.swap(0, 2);
        assert_eq!(a, calculer(&v, &ouvres()));
    }

    #[test]
    fn reperes_depasses_jour_soir_et_semaine() {
        let e = Uuid::new_v4();
        let r = reperes(Position::Normal, Groupe::Quatrieme3eme); // 2 évaluations, 60 min, 5 par semaine
        let mut evts = vec![ev(e, 19, Genre::Travail, 61, 12)];
        // Trois évaluations jeudi 19.
        for _ in 0..3 {
            evts.push(ev(e, 19, Genre::Evaluation, 0, 12));
        }
        let c = calculer(&evts, &ouvres());
        // Le soir du mercredi 18 porte 61 min : au-dessus de 60 ; le jeudi 19 compte 3 évaluations : au-dessus de 2.
        assert_eq!(reperes_depasses(&c[&e], d(18), &r), vec![REPERE_MINUTES]);
        assert_eq!(reperes_depasses(&c[&e], d(19), &r), vec![REPERE_EVALUATIONS]);
        // Au repère exact : pas d'alerte.
        let c = calculer(&[ev(e, 19, Genre::Travail, 60, 12), ev(e, 19, Genre::Evaluation, 0, 12), ev(e, 19, Genre::Evaluation, 0, 12)], &ouvres());
        assert!(reperes_depasses(&c[&e], d(18), &r).is_empty());
        assert!(reperes_depasses(&c[&e], d(19), &r).is_empty());
        // Six évaluations dans la semaine du 16 au 22 : au-dessus de 5.
        let evts: Vec<_> = [16, 17, 18, 19, 20, 20].iter().map(|j| ev(e, *j, Genre::Evaluation, 0, 12)).collect();
        let c = calculer(&evts, &ouvres());
        assert_eq!(evaluations_semaine(&c[&e], d(22)), 6);
        assert!(reperes_depasses(&c[&e], d(20), &r).contains(&REPERE_SEMAINE));
    }

    #[test]
    fn mediane_et_distribution() {
        assert_eq!(mediane(vec![]), 0);
        assert_eq!(mediane(vec![5]), 5);
        assert_eq!(mediane(vec![10, 70, 30]), 30);
        assert_eq!(mediane(vec![10, 20, 30, 40]), 20, "médiane inférieure");
        let dist = distribution([1, 0, 1, 2, 1, 0]);
        assert_eq!(
            dist.iter().map(|x| (x.valeur, x.nb_eleves)).collect::<Vec<_>>(),
            vec![(0, 2), (1, 3), (2, 1)]
        );
    }

    #[test]
    fn jours_ouvres_veille_et_lendemain() {
        let o = ouvres();
        assert_eq!(o.veille(d(16)), Some(d(13)));
        assert_eq!(o.veille(d(12)), Some(d(10)), "le 11 est férié");
        assert_eq!(o.lendemain(d(13)), Some(d(16)));
        assert_eq!(o.lendemain(d(10)), Some(d(12)));
        assert_eq!(o.veille(d(2)), None);
        assert_eq!(o.entre(d(2), d(6)).len(), 5);
        assert!(o.entre(d(6), d(2)).is_empty());
    }
}
