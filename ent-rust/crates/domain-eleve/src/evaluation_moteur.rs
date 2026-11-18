//! Socle 21, lot V2 : **moteur de calcul étendu** des moyennes (logique pure : ni base, ni
//! horloge, ni réseau). Il prolonge `evaluation.rs` (lot 7, V0) sans le modifier : le moteur
//! historique reste la référence des lectures actuelles, ce module est celui des tables `eva_*`.
//!
//! # Principes (conception `docs/socles/21_EVALUATIONS.md` §8, règles R-V3 à R-V5)
//!
//! * **Exact** : tout le calcul se fait en fractions d'entiers (`Rat`, arithmétique vérifiée :
//!   un débordement est une erreur, jamais une panique ni une valeur fausse). Les notes, barèmes
//!   et coefficients entrent en **millièmes** (`NUMERIC(7,3)` / `NUMERIC(6,3)` en base).
//! * **L'arrondi est une règle, pas un accident** : aucun arrondi intermédiaire par défaut ;
//!   chaque étape où un logiciel de notes pourrait arrondir (`EtapesArrondi`) est un réglage
//!   d'établissement, pour pouvoir caler le moteur sur Charlemagne sans recoder
//!   (`docs/socles/21_REGLES_CALCUL_CHARLEMAGNE.md`).
//! * **Une absence n'est jamais un zéro** (R-V5) : aucun réglage ne le permet ici. Le seul zéro
//!   possible est celui d'un travail non rendu, et seulement si l'établissement le choisit
//!   (`TraitementNonRendu::Zero`, question Q-EV1 ouverte).
//! * **Chaque moyenne s'explique** (D-023) : le résultat porte, ligne à ligne, ce qui a compté,
//!   ce qui n'a pas compté et pourquoi.
//! * **Aucune moyenne sans note comptée** : `None`, jamais 0.
//!
//! Les statuts du §8.2 sont tous connus : `Note`, `Absent`, `ARattraper`, `Dispense`,
//! `NonRendu`, `NonNote`, `NonConcerne`, `Annule`, `EnAttente`. Les variantes de comptage d'une
//! évaluation (§8.3) : normale, « compte si elle améliore », « ne compte pas », bonus.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Erreurs
// ---------------------------------------------------------------------------

/// Échec d'un calcul : jamais une valeur approximative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErreurCalcul {
    /// Dépassement de capacité des entiers du calcul exact (données aberrantes).
    Debordement,
    /// Division par zéro (réglage ou donnée incohérente).
    DivisionParZero,
    /// Réglage incohérent (échelle ≤ 0, précision nulle…).
    Reglage,
}

// ---------------------------------------------------------------------------
// Rationnels exacts
// ---------------------------------------------------------------------------

fn pgcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.max(1)
}

/// Nombre rationnel exact (dénominateur > 0, fraction réduite). Les opérations sont
/// **vérifiées** : un débordement rend `Err(Debordement)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rat {
    n: i128,
    d: i128,
}

impl Rat {
    pub const ZERO: Rat = Rat { n: 0, d: 1 };

    pub fn new(n: i128, d: i128) -> Result<Rat, ErreurCalcul> {
        if d == 0 {
            return Err(ErreurCalcul::DivisionParZero);
        }
        let (n, d) = if d < 0 {
            (
                n.checked_neg().ok_or(ErreurCalcul::Debordement)?,
                d.checked_neg().ok_or(ErreurCalcul::Debordement)?,
            )
        } else {
            (n, d)
        };
        let g = pgcd(n, d);
        Ok(Rat { n: n / g, d: d / g })
    }

    pub fn entier(n: i128) -> Rat {
        Rat { n, d: 1 }
    }

    /// Valeur lue en millièmes (`1000` → 1).
    pub fn milliemes(m: i64) -> Rat {
        // Un entier de 64 bits sur 1000 ne peut pas déborder.
        Rat::new(m as i128, 1000).unwrap_or(Rat::ZERO)
    }

    pub fn numerateur(self) -> i128 {
        self.n
    }

    pub fn denominateur(self) -> i128 {
        self.d
    }

    pub fn est_nul(self) -> bool {
        self.n == 0
    }

    pub fn ajouter(self, o: Rat) -> Result<Rat, ErreurCalcul> {
        let g = pgcd(self.d, o.d);
        let d1 = self.d / g;
        let d2 = o.d / g;
        let n = self
            .n
            .checked_mul(d2)
            .and_then(|a| o.n.checked_mul(d1).and_then(|b| a.checked_add(b)))
            .ok_or(ErreurCalcul::Debordement)?;
        let d = self.d.checked_mul(d2).ok_or(ErreurCalcul::Debordement)?;
        Rat::new(n, d)
    }

    pub fn retrancher(self, o: Rat) -> Result<Rat, ErreurCalcul> {
        self.ajouter(Rat {
            n: o.n.checked_neg().ok_or(ErreurCalcul::Debordement)?,
            d: o.d,
        })
    }

    pub fn multiplier(self, o: Rat) -> Result<Rat, ErreurCalcul> {
        let g1 = pgcd(self.n, o.d);
        let g2 = pgcd(o.n, self.d);
        let n = (self.n / g1)
            .checked_mul(o.n / g2)
            .ok_or(ErreurCalcul::Debordement)?;
        let d = (self.d / g2)
            .checked_mul(o.d / g1)
            .ok_or(ErreurCalcul::Debordement)?;
        Rat::new(n, d)
    }

    pub fn diviser(self, o: Rat) -> Result<Rat, ErreurCalcul> {
        if o.n == 0 {
            return Err(ErreurCalcul::DivisionParZero);
        }
        self.multiplier(Rat { n: o.d, d: o.n }.normalise_signe()?)
    }

    fn normalise_signe(self) -> Result<Rat, ErreurCalcul> {
        Rat::new(self.n, self.d)
    }

    /// Valeur flottante (tri, affichage approximatif) : **jamais** une entrée de calcul.
    pub fn en_f64(self) -> f64 {
        self.n as f64 / self.d as f64
    }

    pub fn min(self, o: Rat) -> Rat {
        if self.cmp(&o) == Ordering::Greater {
            o
        } else {
            self
        }
    }

    /// `max(self, 0)`.
    fn positif(self) -> Rat {
        if self.n < 0 {
            Rat::ZERO
        } else {
            self
        }
    }
}

impl PartialOrd for Rat {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for Rat {
    fn cmp(&self, o: &Self) -> Ordering {
        match (self.n.checked_mul(o.d), o.n.checked_mul(self.d)) {
            (Some(a), Some(b)) => a.cmp(&b),
            // Débordement du produit croisé : comparaison exacte par fractions continues.
            _ => cmp_exact(self.n, self.d, o.n, o.d),
        }
    }
}

/// Compare `a/b` et `c/d` (b, d > 0) sans produit croisé : partie entière, puis comparaison
/// inversée des restes (algorithme d'Euclide). Exact, sans débordement.
fn cmp_exact(a: i128, b: i128, c: i128, d: i128) -> Ordering {
    let (qa, ra) = (a.div_euclid(b), a.rem_euclid(b));
    let (qc, rc) = (c.div_euclid(d), c.rem_euclid(d));
    match qa.cmp(&qc) {
        Ordering::Equal => {}
        autre => return autre,
    }
    match (ra == 0, rc == 0) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        // ra/b contre rc/d dans ]0 ; 1[ : l'ordre est celui des inverses, renversé.
        (false, false) => cmp_exact(d, rc, b, ra),
    }
}

// ---------------------------------------------------------------------------
// Arrondis
// ---------------------------------------------------------------------------

/// Pas d'arrondi : on arrondit au plus proche multiple de ce pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Precision {
    Millieme,
    #[default]
    Centieme,
    Dixieme,
    /// Quart de point (0, 0,25, 0,5, 0,75…).
    Quart,
    /// Demi-point (0, 0,5, 1…).
    DemiPoint,
    /// Point entier.
    Unite,
}

impl Precision {
    pub const TOUTES: [Precision; 6] = [
        Self::Millieme,
        Self::Centieme,
        Self::Dixieme,
        Self::Quart,
        Self::DemiPoint,
        Self::Unite,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Millieme => "millieme",
            Self::Centieme => "centieme",
            Self::Dixieme => "dixieme",
            Self::Quart => "quart",
            Self::DemiPoint => "demi_point",
            Self::Unite => "unite",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|p| p.as_str() == s.trim())
    }

    /// Le pas, en millièmes de point.
    pub fn pas_milliemes(self) -> i64 {
        match self {
            Self::Millieme => 1,
            Self::Centieme => 10,
            Self::Dixieme => 100,
            Self::Quart => 250,
            Self::DemiPoint => 500,
            Self::Unite => 1000,
        }
    }
}

/// Que faire d'un reste (le « demi » compris) lors d'un arrondi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModeArrondi {
    /// Au plus proche, le **demi vers le haut** (12,345 → 12,35). Règle historique de ScolIA.
    #[default]
    DemiHaut,
    /// Au plus proche, le demi vers le bas (12,345 → 12,34).
    DemiBas,
    /// Au plus proche, le demi vers le pair (12,345 → 12,34 ; 12,355 → 12,36).
    DemiPair,
    /// Toujours vers le haut (plafond).
    Superieur,
    /// Toujours vers le bas (plancher, troncature des valeurs positives).
    Inferieur,
}

impl ModeArrondi {
    pub const TOUS: [ModeArrondi; 5] = [
        Self::DemiHaut,
        Self::DemiBas,
        Self::DemiPair,
        Self::Superieur,
        Self::Inferieur,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::DemiHaut => "demi_haut",
            Self::DemiBas => "demi_bas",
            Self::DemiPair => "demi_pair",
            Self::Superieur => "superieur",
            Self::Inferieur => "inferieur",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|m| m.as_str() == s.trim())
    }
}

/// Une règle d'arrondi complète : un pas et un mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ArrondiRegle {
    pub precision: Precision,
    pub mode: ModeArrondi,
}

impl ArrondiRegle {
    pub const fn new(precision: Precision, mode: ModeArrondi) -> Self {
        Self { precision, mode }
    }

    /// Arrondit `x` selon la règle ; le résultat est un rationnel exact (multiple du pas).
    pub fn appliquer(self, x: Rat) -> Result<Rat, ErreurCalcul> {
        // q = x / pas = x × 1000 / pas_milliemes, arrondi à l'entier, puis × pas.
        let pas = i128::from(self.precision.pas_milliemes());
        let q = x.multiplier(Rat::new(1000, pas)?)?;
        let (num, den) = (q.numerateur(), q.denominateur());
        let plancher = num.div_euclid(den);
        let reste = num.rem_euclid(den);
        let k = if reste == 0 {
            plancher
        } else {
            match self.mode {
                ModeArrondi::Inferieur => plancher,
                ModeArrondi::Superieur => plancher + 1,
                ModeArrondi::DemiHaut | ModeArrondi::DemiBas | ModeArrondi::DemiPair => {
                    match (2 * reste).cmp(&den) {
                        Ordering::Less => plancher,
                        Ordering::Greater => plancher + 1,
                        Ordering::Equal => match self.mode {
                            ModeArrondi::DemiHaut => plancher + 1,
                            ModeArrondi::DemiBas => plancher,
                            _ => {
                                if plancher % 2 == 0 {
                                    plancher
                                } else {
                                    plancher + 1
                                }
                            }
                        },
                    }
                }
            }
        };
        Rat::new(k.checked_mul(pas).ok_or(ErreurCalcul::Debordement)?, 1000)
    }

    /// Arrondi **d'affichage** en millièmes (entier exact, sans passer par un flottant).
    pub fn en_milliemes(self, x: Rat) -> Result<i64, ErreurCalcul> {
        let r = self.appliquer(x)?;
        // r a pour dénominateur un diviseur de 1000.
        let m = r
            .numerateur()
            .checked_mul(1000 / r.denominateur())
            .ok_or(ErreurCalcul::Debordement)?;
        i64::try_from(m).map_err(|_| ErreurCalcul::Debordement)
    }
}

/// Mise en forme française d'une valeur en millièmes : « 12,8 », « 14 », « 9,125 ».
pub fn formater_fr(milli: i64) -> String {
    let signe = if milli < 0 { "-" } else { "" };
    let a = milli.unsigned_abs();
    let (ent, dec) = (a / 1000, a % 1000);
    if dec == 0 {
        format!("{signe}{ent}")
    } else {
        let mut d = format!("{dec:03}");
        while d.ends_with('0') {
            d.pop();
        }
        format!("{signe}{ent},{d}")
    }
}

// ---------------------------------------------------------------------------
// Statuts, comptage, entrées
// ---------------------------------------------------------------------------

/// Statut d'un résultat (§8.2 de la conception).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutCalcul {
    Note,
    Absent,
    ARattraper,
    Dispense,
    NonRendu,
    NonNote,
    /// Calculé par le socle 5 (l'élève n'était pas dans le public à la date) : jamais saisi.
    NonConcerne,
    Annule,
    /// Case vide : copie pas encore corrigée.
    EnAttente,
}

impl StatutCalcul {
    pub const TOUS: [StatutCalcul; 9] = [
        Self::Note,
        Self::Absent,
        Self::ARattraper,
        Self::Dispense,
        Self::NonRendu,
        Self::NonNote,
        Self::NonConcerne,
        Self::Annule,
        Self::EnAttente,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Absent => "absent",
            Self::ARattraper => "a_rattraper",
            Self::Dispense => "dispense",
            Self::NonRendu => "non_rendu",
            Self::NonNote => "non_note",
            Self::NonConcerne => "non_concerne",
            Self::Annule => "annule",
            Self::EnAttente => "en_attente",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|x| x.as_str() == s.trim())
    }

    /// Statuts qu'une personne peut saisir (`non_concerne` et `en_attente` ne se saisissent pas).
    pub fn saisissable(self) -> bool {
        !matches!(self, Self::NonConcerne | Self::EnAttente)
    }

    /// Libellé court de relevé et de bulletin.
    pub fn abreviation(self) -> &'static str {
        match self {
            Self::Note | Self::NonConcerne | Self::EnAttente => "",
            Self::Absent | Self::ARattraper => "Abs",
            Self::Dispense => "Disp",
            Self::NonRendu => "NR",
            Self::NonNote => "NN",
            Self::Annule => "Ann",
        }
    }
}

impl From<crate::evaluation::StatutResultat> for StatutCalcul {
    fn from(s: crate::evaluation::StatutResultat) -> Self {
        use crate::evaluation::StatutResultat as S;
        match s {
            S::Note => Self::Note,
            S::Absent => Self::Absent,
            S::ARattraper => Self::ARattraper,
            S::Dispense => Self::Dispense,
            S::NonNote => Self::NonNote,
        }
    }
}

/// Comment une évaluation entre dans la moyenne (§8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Comptage {
    #[default]
    Normal,
    /// Comptée seulement si elle fait monter la moyenne.
    SiAmeliore,
    /// Publiée mais jamais comptée (diagnostique, entraînement).
    NeComptePas,
    /// Note bonus : traitée selon `ReglesMoteur::bonus`.
    Bonus,
}

impl Comptage {
    pub const TOUS: [Comptage; 4] = [
        Self::Normal,
        Self::SiAmeliore,
        Self::NeComptePas,
        Self::Bonus,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::SiAmeliore => "si_ameliore",
            Self::NeComptePas => "ne_compte_pas",
            Self::Bonus => "bonus",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|c| c.as_str() == s.trim())
    }
}

/// Une évaluation, vue du moteur (valeurs en millièmes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCalcul {
    pub id: Uuid,
    pub titre: String,
    /// Barème en millièmes de point (`20000` : sur 20).
    pub bareme_m: i64,
    /// Coefficient en millièmes (`1000` : coefficient 1).
    pub coef_m: i64,
    pub comptage: Comptage,
    /// Évaluation qui autorise une note au-delà du barème (bonus de sujet).
    pub hors_bareme_autorise: bool,
    /// Sous-matière visée (`None` : la matière elle-même).
    pub sous_matiere_id: Option<Uuid>,
    pub cours_id: Option<Uuid>,
    /// Évaluation annulée (incident, sujet diffusé) : jamais comptée, quels que soient les résultats.
    pub annulee: bool,
}

impl EvalCalcul {
    /// Évaluation ordinaire sur `bareme` points (entiers), coefficient 1.
    pub fn simple(titre: &str, bareme: i64) -> Self {
        Self {
            id: Uuid::new_v4(),
            titre: titre.to_string(),
            bareme_m: bareme * 1000,
            coef_m: 1000,
            comptage: Comptage::Normal,
            hors_bareme_autorise: false,
            sous_matiere_id: None,
            cours_id: None,
            annulee: false,
        }
    }

    pub fn avec_coef(mut self, coef: i64) -> Self {
        self.coef_m = coef * 1000;
        self
    }

    pub fn avec_comptage(mut self, c: Comptage) -> Self {
        self.comptage = c;
        self
    }
}

/// Le résultat d'un élève à une évaluation, avec l'évaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntreeCalcul {
    pub evaluation: EvalCalcul,
    pub statut: StatutCalcul,
    /// Valeur en millièmes ; présente si et seulement si le statut est `Note`.
    pub valeur_m: Option<i64>,
}

impl EntreeCalcul {
    pub fn note(evaluation: EvalCalcul, valeur: f64) -> Self {
        Self {
            evaluation,
            statut: StatutCalcul::Note,
            valeur_m: Some((valeur * 1000.0).round() as i64),
        }
    }

    pub fn statut(evaluation: EvalCalcul, statut: StatutCalcul) -> Self {
        Self {
            evaluation,
            statut,
            valeur_m: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Règles paramétrables par établissement
// ---------------------------------------------------------------------------

/// Comment pondérer les notes de barèmes différents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PonderationNotes {
    /// Chaque note est ramenée à l'échelle, puis moyenne pondérée par les coefficients :
    /// Σ(note/barème × échelle × coef) / Σ coef. Règle de ScolIA.
    #[default]
    NotesRamenees,
    /// Les points s'additionnent : Σ(note × coef) / Σ(barème × coef) × échelle.
    /// Diffère de la précédente dès que les barèmes diffèrent.
    PointsCumules,
}

/// Un travail non rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TraitementNonRendu {
    /// Ne compte pas (comme une absence) — recommandation, Q-EV1.
    #[default]
    Exclu,
    /// Compte 0 : décision de l'établissement, distincte de l'absence [NV].
    Zero,
}

/// Une évaluation « bonus ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TraitementBonus {
    /// Se comporte comme une évaluation normale.
    CommeNormale,
    /// N'entre jamais dans la moyenne (se comporte comme « ne compte pas »).
    #[default]
    Ignore,
    /// Seuls les points au-dessus du seuil (en millièmes de l'échelle) s'ajoutent à la moyenne :
    /// Σ max(0, note ramenée − seuil) × coef / Σ coef des évaluations non bonus.
    PointsAuDessusDe { seuil_m: i64 },
}

/// Sous-matières.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MethodeSousMatieres {
    /// Moyenne pondérée (par coefficient de sous-matière) des moyennes de sous-matières.
    #[default]
    MoyenneDesMoyennes,
    /// Toutes les notes à plat, sans regarder les sous-matières (S-105).
    ToutesLesNotes,
}

/// Moyenne générale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PonderationGenerale {
    /// Pondérée par le coefficient de chaque matière.
    #[default]
    Ponderee,
    /// Toutes les matières à poids égal (les coefficients sont ignorés).
    NonPonderee,
}

/// Moyenne annuelle d'une matière.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MethodeAnnuelle {
    /// Moyenne des moyennes de période (poids égaux, ou poids fournis) — défaut.
    #[default]
    MoyenneDesPeriodes,
    /// Moyenne de toutes les évaluations de l'année.
    ToutesLesEvaluations,
}

/// Médiane d'un effectif pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MedianePaire {
    /// Moyenne des deux valeurs centrales — usage mathématique.
    #[default]
    MoyenneDesDeux,
    Inferieure,
    Superieure,
}

/// Les étapes où un arrondi peut intervenir **avant** l'affichage. `None` : calcul exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EtapesArrondi {
    /// Chaque note ramenée à l'échelle, avant pondération.
    pub note_ramenee: Option<ArrondiRegle>,
    /// La moyenne de chaque sous-matière, avant d'être pondérée dans la matière.
    pub moyenne_sous_matiere: Option<ArrondiRegle>,
    /// La moyenne de matière, avant d'entrer dans la moyenne générale.
    pub moyenne_matiere: Option<ArrondiRegle>,
    /// La moyenne d'une période, avant d'entrer dans la moyenne annuelle.
    pub moyenne_periode: Option<ArrondiRegle>,
}

/// Toutes les règles de calcul d'un établissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglesMoteur {
    /// Échelle des moyennes, en millièmes (`20000` : sur 20 ; `10000` : sur 10).
    pub echelle_m: i64,
    pub ponderation: PonderationNotes,
    pub non_rendu: TraitementNonRendu,
    pub bonus: TraitementBonus,
    pub sous_matieres: MethodeSousMatieres,
    pub generale: PonderationGenerale,
    pub annuelle: MethodeAnnuelle,
    pub arrondis: EtapesArrondi,
    /// Arrondi d'affichage (et d'export par défaut).
    pub affichage: ArrondiRegle,
    /// Une moyenne ne dépasse jamais l'échelle (après bonus).
    pub plafonner_a_l_echelle: bool,
    /// Effectif minimal d'élèves notés pour montrer une statistique à une famille.
    pub seuil_effectif_stats: usize,
    pub mediane: MedianePaire,
}

impl Default for ReglesMoteur {
    /// Les règles recommandées par la conception : exactes, expliquées, sans arrondi
    /// intermédiaire, seuil de confidentialité de 5 élèves.
    fn default() -> Self {
        Self {
            echelle_m: 20_000,
            ponderation: PonderationNotes::NotesRamenees,
            non_rendu: TraitementNonRendu::Exclu,
            bonus: TraitementBonus::Ignore,
            sous_matieres: MethodeSousMatieres::MoyenneDesMoyennes,
            generale: PonderationGenerale::Ponderee,
            annuelle: MethodeAnnuelle::MoyenneDesPeriodes,
            arrondis: EtapesArrondi::default(),
            affichage: ArrondiRegle::default(),
            plafonner_a_l_echelle: true,
            seuil_effectif_stats: 5,
            mediane: MedianePaire::MoyenneDesDeux,
        }
    }
}

impl ReglesMoteur {
    fn echelle(&self) -> Rat {
        Rat::milliemes(self.echelle_m)
    }

    fn controle(&self) -> Result<(), ErreurCalcul> {
        if self.echelle_m <= 0 {
            return Err(ErreurCalcul::Reglage);
        }
        if let TraitementBonus::PointsAuDessusDe { seuil_m } = self.bonus {
            if seuil_m < 0 {
                return Err(ErreurCalcul::Reglage);
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Explication
// ---------------------------------------------------------------------------

/// Pourquoi un résultat n'a pas compté.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaisonNonComptee {
    Absent,
    ARattraper,
    Dispense,
    NonRendu,
    NonNote,
    NonConcerne,
    Annule,
    EnAttente,
    /// L'évaluation entière est annulée.
    EvaluationAnnulee,
    /// Évaluation « ne compte pas » (diagnostique…).
    NeComptePas,
    /// Évaluation « compte si elle améliore » : elle aurait fait baisser la moyenne.
    NonRetenueCarElleBaisse,
    /// Bonus ignoré par le réglage de l'établissement.
    BonusIgnore,
    CoefficientNul,
    /// Donnée incohérente (barème nul, valeur au-delà du barème, note sans valeur) : jamais
    /// transformée en note.
    DonneeInvalide,
}

impl RaisonNonComptee {
    pub fn libelle(self) -> &'static str {
        match self {
            Self::Absent => "absent, non compté",
            Self::ARattraper => "à rattraper, non compté tant qu'il n'est pas noté",
            Self::Dispense => "dispensé, non compté",
            Self::NonRendu => "non rendu, non compté",
            Self::NonNote => "non noté",
            Self::NonConcerne => "élève non concerné par cette évaluation",
            Self::Annule => "résultat annulé, non compté",
            Self::EnAttente => "pas encore corrigé",
            Self::EvaluationAnnulee => "évaluation annulée",
            Self::NeComptePas => "ne compte pas (évaluation diagnostique ou d'entraînement)",
            Self::NonRetenueCarElleBaisse => "non retenue : elle ferait baisser la moyenne",
            Self::BonusIgnore => "bonus non pris en compte par l'établissement",
            Self::CoefficientNul => "coefficient nul",
            Self::DonneeInvalide => "donnée incohérente, ignorée",
        }
    }
}

/// Ce qui est arrivé à une ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "effet", content = "raison", rename_all = "snake_case")]
pub enum Effet {
    Comptee,
    /// Un non rendu compté comme 0 par réglage d'établissement.
    CompteeZeroNonRendu,
    /// Comptée car elle améliore la moyenne.
    CompteeCarAmeliore,
    /// Bonus : seuls les points au-dessus du seuil comptent.
    CompteeCommeBonus,
    NonComptee(RaisonNonComptee),
}

impl Effet {
    pub fn compte(self) -> bool {
        !matches!(self, Self::NonComptee(_))
    }
}

/// Une ligne d'explication : une évaluation, ce qu'elle a donné, ce qu'elle a changé.
#[derive(Debug, Clone, PartialEq)]
pub struct LigneExplication {
    pub evaluation_id: Uuid,
    pub titre: String,
    pub statut: StatutCalcul,
    pub valeur_m: Option<i64>,
    pub bareme_m: i64,
    pub coef_m: i64,
    /// Note ramenée à l'échelle (exacte) quand elle existe.
    pub note_ramenee: Option<Rat>,
    pub effet: Effet,
}

impl LigneExplication {
    /// Phrase prête à afficher à la famille.
    pub fn texte_fr(&self, echelle_m: i64) -> String {
        let brut = match (self.statut, self.valeur_m) {
            (StatutCalcul::Note, Some(v)) => {
                format!("{} sur {}", formater_fr(v), formater_fr(self.bareme_m))
            }
            (s, _) => s.abreviation().to_string(),
        };
        let ramenee = match self.note_ramenee {
            Some(r) if self.bareme_m != echelle_m => {
                let m = ArrondiRegle::new(Precision::Centieme, ModeArrondi::DemiHaut)
                    .en_milliemes(r)
                    .unwrap_or(0);
                format!(", soit {} sur {}", formater_fr(m), formater_fr(echelle_m))
            }
            _ => String::new(),
        };
        let verdict = match self.effet {
            Effet::Comptee => format!("comptée (coefficient {})", formater_fr(self.coef_m)),
            Effet::CompteeZeroNonRendu => {
                format!(
                    "non rendu, compté 0 (coefficient {})",
                    formater_fr(self.coef_m)
                )
            }
            Effet::CompteeCarAmeliore => {
                format!(
                    "comptée car elle améliore la moyenne (coefficient {})",
                    formater_fr(self.coef_m)
                )
            }
            Effet::CompteeCommeBonus => "comptée comme bonus".to_string(),
            Effet::NonComptee(r) => r.libelle().to_string(),
        };
        if brut.is_empty() {
            format!("{} : {verdict}", self.titre)
        } else {
            format!("{} : {brut}{ramenee} — {verdict}", self.titre)
        }
    }
}

/// À quel niveau se rattache une composante de moyenne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Niveau {
    /// Groupe d'évaluations d'une sous-matière (`None` : évaluations rattachées à la matière).
    SousMatiere(Option<Uuid>),
    Matiere(Uuid),
    Periode(Uuid),
}

/// Une composante d'une moyenne composée (sous-matière, matière, période).
#[derive(Debug, Clone, PartialEq)]
pub struct Composante {
    pub niveau: Niveau,
    pub coef_m: i64,
    /// Moyenne de la composante **telle qu'entrée dans le calcul** (après arrondi d'étape).
    pub moyenne: Option<Rat>,
    pub compte: bool,
    pub raison: Option<RaisonNonComptee>,
}

/// Quelle formule a produit la moyenne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Formule {
    /// Σ(note ramenée × coef) / Σ coef.
    NotesRamenees,
    /// Σ(note × coef) / Σ(barème × coef) × échelle.
    PointsCumules,
    /// Σ(moyenne × coef) / Σ coef sur des composantes (sous-matières, matières, périodes).
    Composantes,
}

/// Une moyenne et tout ce qu'il faut pour l'expliquer.
#[derive(Debug, Clone, PartialEq)]
pub struct MoyenneCalculee {
    /// Valeur exacte, `None` si rien ne compte.
    pub exacte: Option<Rat>,
    /// Valeur d'affichage en millièmes (règle `affichage`).
    pub affichee_m: Option<i64>,
    pub formule: Formule,
    pub lignes: Vec<LigneExplication>,
    pub composantes: Vec<Composante>,
    /// Points ajoutés par des bonus (déjà inclus dans `exacte`).
    pub ajout_bonus: Option<Rat>,
    /// Plafonnée à l'échelle.
    pub plafonnee: bool,
}

impl MoyenneCalculee {
    fn vide(formule: Formule) -> Self {
        Self {
            exacte: None,
            affichee_m: None,
            formule,
            lignes: Vec::new(),
            composantes: Vec::new(),
            ajout_bonus: None,
            plafonnee: false,
        }
    }

    pub fn nb_comptees(&self) -> usize {
        self.lignes.iter().filter(|l| l.effet.compte()).count()
    }

    pub fn nb_non_rendus(&self) -> usize {
        self.lignes
            .iter()
            .filter(|l| matches!(l.statut, StatutCalcul::NonRendu))
            .count()
    }

    /// Texte d'explication, une phrase par évaluation puis la conclusion.
    pub fn explication_fr(&self, regles: &ReglesMoteur) -> Vec<String> {
        let mut out: Vec<String> = self
            .lignes
            .iter()
            .map(|l| l.texte_fr(regles.echelle_m))
            .collect();
        for c in &self.composantes {
            let lib = match c.niveau {
                Niveau::SousMatiere(Some(_)) => "sous-matière",
                Niveau::SousMatiere(None) => "évaluations de la matière",
                Niveau::Matiere(_) => "matière",
                Niveau::Periode(_) => "période",
            };
            let v = c
                .moyenne
                .and_then(|m| regles.affichage.en_milliemes(m).ok())
                .map(formater_fr)
                .unwrap_or_else(|| "aucune moyenne".to_string());
            out.push(format!(
                "{lib} : {v} (coefficient {}){}",
                formater_fr(c.coef_m),
                if c.compte { "" } else { " — non comptée" }
            ));
        }
        match self.affichee_m {
            Some(m) => out.push(format!(
                "Moyenne : {} sur {}",
                formater_fr(m),
                formater_fr(regles.echelle_m)
            )),
            None => {
                out.push("Aucune moyenne : aucune évaluation ne compte pour le moment.".to_string())
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Classement d'une entrée
// ---------------------------------------------------------------------------

/// Une entrée prête à compter : valeurs exactes.
#[derive(Clone)]
struct Compte {
    idx: usize,
    ramenee: Rat,
    valeur: Rat,
    bareme: Rat,
    coef: Rat,
}

/// Ce que devient une entrée : comptée (valeurs exactes) ou écartée (raison).
enum Classement {
    Compte {
        valeur: Rat,
        bareme: Rat,
        ramenee: Rat,
        coef: Rat,
        effet: Effet,
    },
    Exclu(RaisonNonComptee),
}

fn classer(e: &EntreeCalcul, regles: &ReglesMoteur) -> Result<Classement, ErreurCalcul> {
    let ev = &e.evaluation;
    if ev.annulee {
        return Ok(Classement::Exclu(RaisonNonComptee::EvaluationAnnulee));
    }
    let (valeur_m, effet_zero) = match e.statut {
        StatutCalcul::Note => match e.valeur_m {
            Some(v) if v >= 0 => (v, Effet::Comptee),
            _ => return Ok(Classement::Exclu(RaisonNonComptee::DonneeInvalide)),
        },
        StatutCalcul::NonRendu => match regles.non_rendu {
            TraitementNonRendu::Zero => (0, Effet::CompteeZeroNonRendu),
            TraitementNonRendu::Exclu => return Ok(Classement::Exclu(RaisonNonComptee::NonRendu)),
        },
        StatutCalcul::Absent => return Ok(Classement::Exclu(RaisonNonComptee::Absent)),
        StatutCalcul::ARattraper => return Ok(Classement::Exclu(RaisonNonComptee::ARattraper)),
        StatutCalcul::Dispense => return Ok(Classement::Exclu(RaisonNonComptee::Dispense)),
        StatutCalcul::NonNote => return Ok(Classement::Exclu(RaisonNonComptee::NonNote)),
        StatutCalcul::NonConcerne => return Ok(Classement::Exclu(RaisonNonComptee::NonConcerne)),
        StatutCalcul::Annule => return Ok(Classement::Exclu(RaisonNonComptee::Annule)),
        StatutCalcul::EnAttente => return Ok(Classement::Exclu(RaisonNonComptee::EnAttente)),
    };
    if ev.bareme_m <= 0 || ev.coef_m < 0 {
        return Ok(Classement::Exclu(RaisonNonComptee::DonneeInvalide));
    }
    if valeur_m > ev.bareme_m && !ev.hors_bareme_autorise {
        return Ok(Classement::Exclu(RaisonNonComptee::DonneeInvalide));
    }
    match ev.comptage {
        Comptage::NeComptePas => return Ok(Classement::Exclu(RaisonNonComptee::NeComptePas)),
        Comptage::Bonus if regles.bonus == TraitementBonus::Ignore => {
            return Ok(Classement::Exclu(RaisonNonComptee::BonusIgnore));
        }
        _ => {}
    }
    if ev.coef_m == 0 {
        return Ok(Classement::Exclu(RaisonNonComptee::CoefficientNul));
    }
    let valeur = Rat::milliemes(valeur_m);
    let bareme = Rat::milliemes(ev.bareme_m);
    let coef = Rat::milliemes(ev.coef_m);
    let mut ramenee = valeur.multiplier(regles.echelle())?.diviser(bareme)?;
    if let Some(r) = regles.arrondis.note_ramenee {
        ramenee = r.appliquer(ramenee)?;
    }
    let effet = if ev.comptage == Comptage::Bonus
        && matches!(regles.bonus, TraitementBonus::PointsAuDessusDe { .. })
    {
        Effet::CompteeCommeBonus
    } else {
        effet_zero
    };
    Ok(Classement::Compte {
        valeur,
        bareme,
        ramenee,
        coef,
        effet,
    })
}

fn raison_depuis_statut(s: StatutCalcul) -> RaisonNonComptee {
    match s {
        StatutCalcul::Absent => RaisonNonComptee::Absent,
        StatutCalcul::ARattraper => RaisonNonComptee::ARattraper,
        StatutCalcul::Dispense => RaisonNonComptee::Dispense,
        StatutCalcul::NonRendu => RaisonNonComptee::NonRendu,
        StatutCalcul::NonNote => RaisonNonComptee::NonNote,
        StatutCalcul::NonConcerne => RaisonNonComptee::NonConcerne,
        StatutCalcul::Annule => RaisonNonComptee::Annule,
        StatutCalcul::EnAttente => RaisonNonComptee::EnAttente,
        StatutCalcul::Note => RaisonNonComptee::DonneeInvalide,
    }
}

/// Moyenne d'une liste (somme pondérée) selon la méthode de pondération des notes.
fn moyenne_de(lots: &[&Compte], regles: &ReglesMoteur) -> Result<Option<Rat>, ErreurCalcul> {
    let mut num = Rat::ZERO;
    let mut den = Rat::ZERO;
    for c in lots {
        match regles.ponderation {
            PonderationNotes::NotesRamenees => {
                num = num.ajouter(c.ramenee.multiplier(c.coef)?)?;
                den = den.ajouter(c.coef)?;
            }
            PonderationNotes::PointsCumules => {
                num = num.ajouter(c.valeur.multiplier(c.coef)?)?;
                den = den.ajouter(c.bareme.multiplier(c.coef)?)?;
            }
        }
    }
    if den.est_nul() {
        return Ok(None);
    }
    let mut m = num.diviser(den)?;
    if regles.ponderation == PonderationNotes::PointsCumules {
        m = m.multiplier(regles.echelle())?;
    }
    Ok(Some(m))
}

// ---------------------------------------------------------------------------
// Moyenne d'un ensemble d'évaluations (cours ou matière « à plat »)
// ---------------------------------------------------------------------------

/// Moyenne pondérée d'un ensemble de résultats, **à plat** (cours, ou matière sans sous-matières).
///
/// 1. Les statuts non comptés et les évaluations exclues sont écartés (avec leur raison).
/// 2. Les évaluations normales (et bonus « comme normale ») forment la base.
/// 3. Les évaluations « compte si elle améliore » s'ajoutent **dans l'ordre de leur note ramenée
///    décroissante** (puis coefficient, puis identifiant : le résultat ne dépend jamais de
///    l'ordre des saisies) et seulement si elles font monter la moyenne ; sans moyenne de base,
///    la première compte.
/// 4. Les bonus « points au-dessus du seuil » s'ajoutent à la moyenne.
/// 5. La moyenne est plafonnée à l'échelle si le réglage l'impose.
pub fn moyenne_cours(
    entrees: &[EntreeCalcul],
    regles: &ReglesMoteur,
) -> Result<MoyenneCalculee, ErreurCalcul> {
    regles.controle()?;
    let mut lignes: Vec<Option<LigneExplication>> = vec![None; entrees.len()];
    let mut base: Vec<Compte> = Vec::new();
    let mut ameliore: Vec<Compte> = Vec::new();
    let mut bonus: Vec<Compte> = Vec::new();

    for (idx, e) in entrees.iter().enumerate() {
        let ev = &e.evaluation;
        let mut ligne = LigneExplication {
            evaluation_id: ev.id,
            titre: ev.titre.clone(),
            statut: e.statut,
            valeur_m: e.valeur_m,
            bareme_m: ev.bareme_m,
            coef_m: ev.coef_m,
            note_ramenee: None,
            effet: Effet::NonComptee(raison_depuis_statut(e.statut)),
        };
        match classer(e, regles)? {
            Classement::Exclu(raison) => ligne.effet = Effet::NonComptee(raison),
            Classement::Compte {
                valeur,
                bareme,
                ramenee,
                coef,
                effet,
            } => {
                ligne.note_ramenee = Some(ramenee);
                ligne.effet = effet;
                let c = Compte {
                    idx,
                    ramenee,
                    valeur,
                    bareme,
                    coef,
                };
                match ev.comptage {
                    Comptage::SiAmeliore => ameliore.push(c),
                    Comptage::Bonus
                        if matches!(regles.bonus, TraitementBonus::PointsAuDessusDe { .. }) =>
                    {
                        bonus.push(c);
                    }
                    _ => base.push(c),
                }
            }
        }
        lignes[idx] = Some(ligne);
    }

    // Ordre canonique des « compte si elle améliore » : indépendant de l'ordre des saisies.
    ameliore.sort_by(|a, b| {
        b.ramenee
            .cmp(&a.ramenee)
            .then_with(|| b.coef.cmp(&a.coef))
            .then_with(|| {
                entrees[a.idx]
                    .evaluation
                    .id
                    .cmp(&entrees[b.idx].evaluation.id)
            })
    });

    let mut retenues: Vec<Compte> = base;
    let mut courante = {
        let refs: Vec<&Compte> = retenues.iter().collect();
        moyenne_de(&refs, regles)?
    };
    for c in ameliore {
        let mut essai: Vec<&Compte> = retenues.iter().collect();
        essai.push(&c);
        let avec = moyenne_de(&essai, regles)?;
        let ameliore_la_moyenne = match (courante, avec) {
            (None, Some(_)) => true,
            (Some(avant), Some(apres)) => apres > avant,
            _ => false,
        };
        if ameliore_la_moyenne {
            if let Some(l) = lignes[c.idx].as_mut() {
                l.effet = Effet::CompteeCarAmeliore;
            }
            retenues.push(c);
            courante = avec;
        } else if let Some(l) = lignes[c.idx].as_mut() {
            l.effet = Effet::NonComptee(RaisonNonComptee::NonRetenueCarElleBaisse);
        }
    }

    let mut res = MoyenneCalculee::vide(match regles.ponderation {
        PonderationNotes::NotesRamenees => Formule::NotesRamenees,
        PonderationNotes::PointsCumules => Formule::PointsCumules,
    });
    res.lignes = lignes.into_iter().flatten().collect();

    let mut moyenne = courante;
    if let (Some(m), TraitementBonus::PointsAuDessusDe { seuil_m }) = (moyenne, regles.bonus) {
        if !bonus.is_empty() {
            let seuil = Rat::milliemes(seuil_m);
            let mut points = Rat::ZERO;
            for b in &bonus {
                points =
                    points.ajouter(b.ramenee.retrancher(seuil)?.positif().multiplier(b.coef)?)?;
            }
            let mut den = Rat::ZERO;
            for r in &retenues {
                den = den.ajouter(r.coef)?;
            }
            if !den.est_nul() {
                let ajout = points.diviser(den)?;
                res.ajout_bonus = Some(ajout);
                moyenne = Some(m.ajouter(ajout)?);
            }
        }
    }
    if let Some(m) = moyenne {
        let echelle = regles.echelle();
        let m = if regles.plafonner_a_l_echelle && m > echelle {
            res.plafonnee = true;
            echelle
        } else {
            m
        };
        res.exacte = Some(m);
        res.affichee_m = Some(regles.affichage.en_milliemes(m)?);
    }
    Ok(res)
}

// ---------------------------------------------------------------------------
// Moyenne de matière (sous-matières, plusieurs cours)
// ---------------------------------------------------------------------------

/// Coefficient d'une sous-matière dans sa matière.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoefSousMatiere {
    /// `None` : les évaluations rattachées directement à la matière parente.
    pub sous_matiere_id: Option<Uuid>,
    pub coef_m: i64,
}

fn arrondir_etape(
    m: Option<Rat>,
    etape: Option<ArrondiRegle>,
) -> Result<Option<Rat>, ErreurCalcul> {
    match (m, etape) {
        (Some(m), Some(r)) => Ok(Some(r.appliquer(m)?)),
        (m, _) => Ok(m),
    }
}

/// Moyenne de **matière** d'un élève sur une période : toutes les évaluations des cours de la
/// matière qu'il a suivis (un changement de groupe de maths garde les notes des deux cours,
/// CE-11), avec ou sans sous-matières.
///
/// * `MoyenneDesMoyennes` : une moyenne par sous-matière (les évaluations rattachées à la matière
///   forment leur propre ensemble), puis moyenne pondérée par `coefs` (défaut : 1). Une
///   sous-matière sans moyenne est ignorée.
/// * `ToutesLesNotes` : tout est mis à plat (S-105).
///
/// Sans aucune sous-matière, les deux méthodes donnent exactement `moyenne_cours`.
pub fn moyenne_matiere(
    entrees: &[EntreeCalcul],
    coefs: &[CoefSousMatiere],
    regles: &ReglesMoteur,
) -> Result<MoyenneCalculee, ErreurCalcul> {
    regles.controle()?;
    let a_des_sous_matieres = entrees
        .iter()
        .any(|e| e.evaluation.sous_matiere_id.is_some());
    if regles.sous_matieres == MethodeSousMatieres::ToutesLesNotes || !a_des_sous_matieres {
        return moyenne_cours(entrees, regles);
    }
    // Groupes dans l'ordre d'apparition des identifiants triés (déterminisme).
    let mut cles: Vec<Option<Uuid>> = entrees
        .iter()
        .map(|e| e.evaluation.sous_matiere_id)
        .collect();
    cles.sort();
    cles.dedup();

    let mut res = MoyenneCalculee::vide(Formule::Composantes);
    let mut num = Rat::ZERO;
    let mut den = Rat::ZERO;
    for cle in cles {
        let groupe: Vec<EntreeCalcul> = entrees
            .iter()
            .filter(|e| e.evaluation.sous_matiere_id == cle)
            .cloned()
            .collect();
        let m = moyenne_cours(&groupe, regles)?;
        let coef_m = coefs
            .iter()
            .find(|c| c.sous_matiere_id == cle)
            .map_or(1000, |c| c.coef_m);
        let entree = arrondir_etape(m.exacte, regles.arrondis.moyenne_sous_matiere)?;
        let mut compte = false;
        let mut raison = None;
        match entree {
            None => raison = Some(RaisonNonComptee::EnAttente),
            Some(_) if coef_m <= 0 => raison = Some(RaisonNonComptee::CoefficientNul),
            Some(v) => {
                let c = Rat::milliemes(coef_m);
                num = num.ajouter(v.multiplier(c)?)?;
                den = den.ajouter(c)?;
                compte = true;
            }
        }
        res.composantes.push(Composante {
            niveau: Niveau::SousMatiere(cle),
            coef_m,
            moyenne: entree,
            compte,
            raison,
        });
        res.lignes.extend(m.lignes);
    }
    if !den.est_nul() {
        let m = num.diviser(den)?;
        res.exacte = Some(m);
        res.affichee_m = Some(regles.affichage.en_milliemes(m)?);
    }
    Ok(res)
}

// ---------------------------------------------------------------------------
// Moyenne générale
// ---------------------------------------------------------------------------

/// Une matière dans la moyenne générale d'un élève.
#[derive(Debug, Clone, PartialEq)]
pub struct MatiereMoyenne {
    pub matiere_id: Uuid,
    /// `None` : aucune moyenne (rien ne compte, ou « en attente »).
    pub moyenne: Option<Rat>,
    pub coef_m: i64,
    /// Matière hors moyenne générale (vie de classe, option facultative…).
    pub hors_moyenne_generale: bool,
}

/// Moyenne générale : Σ(moyenne de matière × coefficient) / Σ coefficients des matières **qui
/// ont une moyenne**. Indicative : aucune décision n'en découle (R-V20).
pub fn moyenne_generale(
    matieres: &[MatiereMoyenne],
    regles: &ReglesMoteur,
) -> Result<MoyenneCalculee, ErreurCalcul> {
    regles.controle()?;
    let mut res = MoyenneCalculee::vide(Formule::Composantes);
    let mut num = Rat::ZERO;
    let mut den = Rat::ZERO;
    let mut tries: Vec<&MatiereMoyenne> = matieres.iter().collect();
    tries.sort_by_key(|m| m.matiere_id);
    for m in tries {
        let entree = arrondir_etape(m.moyenne, regles.arrondis.moyenne_matiere)?;
        let coef_m = match regles.generale {
            PonderationGenerale::Ponderee => m.coef_m,
            PonderationGenerale::NonPonderee => 1000,
        };
        let (compte, raison) = if m.hors_moyenne_generale {
            (false, Some(RaisonNonComptee::NeComptePas))
        } else if entree.is_none() {
            (false, Some(RaisonNonComptee::EnAttente))
        } else if coef_m <= 0 {
            (false, Some(RaisonNonComptee::CoefficientNul))
        } else {
            (true, None)
        };
        if let (true, Some(v)) = (compte, entree) {
            let c = Rat::milliemes(coef_m);
            num = num.ajouter(v.multiplier(c)?)?;
            den = den.ajouter(c)?;
        }
        res.composantes.push(Composante {
            niveau: Niveau::Matiere(m.matiere_id),
            coef_m,
            moyenne: entree,
            compte,
            raison,
        });
    }
    if !den.est_nul() {
        let m = num.diviser(den)?;
        res.exacte = Some(m);
        res.affichee_m = Some(regles.affichage.en_milliemes(m)?);
    }
    Ok(res)
}

// ---------------------------------------------------------------------------
// Moyenne annuelle
// ---------------------------------------------------------------------------

/// Moyenne d'une période, pour la moyenne annuelle.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodeMoyenne {
    pub periode_id: Uuid,
    pub moyenne: Option<Rat>,
    /// Poids de la période en millièmes (`1000` : poids égaux par défaut).
    pub poids_m: i64,
}

/// Moyenne annuelle **par la moyenne des périodes** (méthode par défaut) : Σ(moyenne × poids)
/// / Σ poids, les périodes sans moyenne étant ignorées. Sert aussi à la moyenne générale
/// annuelle calculée à partir des moyennes générales de période.
pub fn moyenne_annuelle_periodes(
    periodes: &[PeriodeMoyenne],
    regles: &ReglesMoteur,
) -> Result<MoyenneCalculee, ErreurCalcul> {
    regles.controle()?;
    let mut res = MoyenneCalculee::vide(Formule::Composantes);
    let mut num = Rat::ZERO;
    let mut den = Rat::ZERO;
    let mut tries: Vec<&PeriodeMoyenne> = periodes.iter().collect();
    tries.sort_by_key(|p| p.periode_id);
    for p in tries {
        let entree = arrondir_etape(p.moyenne, regles.arrondis.moyenne_periode)?;
        let (compte, raison) = match entree {
            None => (false, Some(RaisonNonComptee::EnAttente)),
            Some(_) if p.poids_m <= 0 => (false, Some(RaisonNonComptee::CoefficientNul)),
            Some(_) => (true, None),
        };
        if let (true, Some(v)) = (compte, entree) {
            let c = Rat::milliemes(p.poids_m);
            num = num.ajouter(v.multiplier(c)?)?;
            den = den.ajouter(c)?;
        }
        res.composantes.push(Composante {
            niveau: Niveau::Periode(p.periode_id),
            coef_m: p.poids_m,
            moyenne: entree,
            compte,
            raison,
        });
    }
    if !den.est_nul() {
        let m = num.diviser(den)?;
        res.exacte = Some(m);
        res.affichee_m = Some(regles.affichage.en_milliemes(m)?);
    }
    Ok(res)
}

/// Moyenne annuelle **par toutes les évaluations de l'année** (seconde méthode, S-106) :
/// une seule moyenne pondérée sur l'ensemble, sous-matières comprises.
pub fn moyenne_annuelle_evaluations(
    entrees: &[EntreeCalcul],
    coefs: &[CoefSousMatiere],
    regles: &ReglesMoteur,
) -> Result<MoyenneCalculee, ErreurCalcul> {
    moyenne_matiere(entrees, coefs, regles)
}

// ---------------------------------------------------------------------------
// Statistiques
// ---------------------------------------------------------------------------

/// Statistiques d'une série de moyennes (ou de notes ramenées).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Statistiques {
    pub effectif: usize,
    pub moyenne: Rat,
    pub min: Rat,
    pub max: Rat,
    pub mediane: Rat,
}

impl Statistiques {
    /// Vues en millièmes, arrondies selon la règle d'affichage.
    pub fn affichees(&self, regles: &ReglesMoteur) -> Result<StatistiquesAffichees, ErreurCalcul> {
        Ok(StatistiquesAffichees {
            effectif: self.effectif,
            moyenne_m: regles.affichage.en_milliemes(self.moyenne)?,
            min_m: regles.affichage.en_milliemes(self.min)?,
            max_m: regles.affichage.en_milliemes(self.max)?,
            mediane_m: regles.affichage.en_milliemes(self.mediane)?,
        })
    }

    /// Les statistiques qu'on peut montrer à une famille ou à un élève : `None` sous le seuil
    /// d'effectif (R-V12 : un groupe de trois élèves révèle sinon les notes des autres).
    pub fn pour_famille(&self, regles: &ReglesMoteur) -> Option<&Statistiques> {
        (self.effectif >= regles.seuil_effectif_stats.max(1)).then_some(self)
    }
}

/// Statistiques d'affichage (millièmes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatistiquesAffichees {
    pub effectif: usize,
    pub moyenne_m: i64,
    pub min_m: i64,
    pub max_m: i64,
    pub mediane_m: i64,
}

/// Moyenne, minimum, maximum, médiane d'une série de valeurs **déjà calculées** (les moyennes
/// des élèves : « moyenne de classe = moyenne des moyennes », §8.7). Les élèves sans moyenne
/// ne sont pas dans la série.
pub fn statistiques(
    valeurs: &[Rat],
    regles: &ReglesMoteur,
) -> Result<Option<Statistiques>, ErreurCalcul> {
    if valeurs.is_empty() {
        return Ok(None);
    }
    let mut v: Vec<Rat> = valeurs.to_vec();
    v.sort();
    let mut somme = Rat::ZERO;
    for x in &v {
        somme = somme.ajouter(*x)?;
    }
    let n = v.len();
    let moyenne = somme.diviser(Rat::entier(n as i128))?;
    let mediane = if n % 2 == 1 {
        v[n / 2]
    } else {
        let (bas, haut) = (v[n / 2 - 1], v[n / 2]);
        match regles.mediane {
            MedianePaire::MoyenneDesDeux => bas.ajouter(haut)?.diviser(Rat::entier(2))?,
            MedianePaire::Inferieure => bas,
            MedianePaire::Superieure => haut,
        }
    };
    Ok(Some(Statistiques {
        effectif: n,
        moyenne,
        min: v[0],
        max: v[n - 1],
        mediane,
    }))
}

/// Statistiques **d'une évaluation** : sur les notes ramenées à l'échelle des élèves notés.
/// Les statuts non comptés (absent, dispensé…) n'entrent pas ; le comptage de l'évaluation
/// (« ne compte pas ») ne change pas la statistique de l'évaluation elle-même.
pub fn statistiques_evaluation(
    resultats: &[(StatutCalcul, Option<i64>)],
    bareme_m: i64,
    regles: &ReglesMoteur,
) -> Result<Option<Statistiques>, ErreurCalcul> {
    regles.controle()?;
    if bareme_m <= 0 {
        return Ok(None);
    }
    let bareme = Rat::milliemes(bareme_m);
    let mut notes = Vec::new();
    for (statut, valeur) in resultats {
        if let (StatutCalcul::Note, Some(v)) = (statut, valeur) {
            if *v >= 0 {
                let mut r = Rat::milliemes(*v)
                    .multiplier(regles.echelle())?
                    .diviser(bareme)?;
                if let Some(a) = regles.arrondis.note_ramenee {
                    r = a.appliquer(r)?;
                }
                notes.push(r);
            }
        }
    }
    statistiques(&notes, regles)
}

// ---------------------------------------------------------------------------
// Validation de saisie (barème, pas, statut)
// ---------------------------------------------------------------------------

/// Refus de saisie d'un résultat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErreurSaisieMoteur {
    /// Statut `note` sans valeur, ou statut non numérique avec une valeur.
    Incoherent,
    /// Valeur négative.
    Negative,
    /// Au-delà du barème (« 23 sur 20 : vérifiez le barème »).
    HorsBareme,
    /// N'est pas un multiple du pas de saisie de l'établissement (le demi-point refuse 13,3).
    HorsPas,
    /// Statut qu'une personne ne peut pas saisir (`non_concerne`, `en_attente`).
    StatutNonSaisissable,
    /// Barème ≤ 0.
    Bareme,
}

/// Contrôle un résultat avant écriture. `pas_saisie_m` : 10 (centième), 500 (demi-point)…
pub fn valider_saisie(
    statut: StatutCalcul,
    valeur_m: Option<i64>,
    bareme_m: i64,
    hors_bareme_autorise: bool,
    pas_saisie_m: i64,
) -> Result<(), ErreurSaisieMoteur> {
    if !statut.saisissable() {
        return Err(ErreurSaisieMoteur::StatutNonSaisissable);
    }
    if bareme_m <= 0 {
        return Err(ErreurSaisieMoteur::Bareme);
    }
    match (statut, valeur_m) {
        (StatutCalcul::Note, Some(v)) => {
            if v < 0 {
                return Err(ErreurSaisieMoteur::Negative);
            }
            if v > bareme_m && !hors_bareme_autorise {
                return Err(ErreurSaisieMoteur::HorsBareme);
            }
            if pas_saisie_m > 0 && v % pas_saisie_m != 0 {
                return Err(ErreurSaisieMoteur::HorsPas);
            }
            Ok(())
        }
        (StatutCalcul::Note, None) | (_, Some(_)) => Err(ErreurSaisieMoteur::Incoherent),
        (_, None) => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Diagnostic de comparaison (jamais appelé par un service d'écriture ni de publication)
// ---------------------------------------------------------------------------

/// **Diagnostic uniquement** — « que vaudrait la moyenne si les absences comptaient 0 ? ».
/// Sert à *expliquer* un écart avec un logiciel qui compte les absents à zéro (outil de
/// comparaison avec Charlemagne, idée I-032) ; ne produit jamais une moyenne officielle,
/// n'est branchée sur aucune écriture ni publication, et ne dépend d'aucun réglage (R-V5).
pub fn diagnostic_absents_comptes_zero(
    entrees: &[EntreeCalcul],
    regles: &ReglesMoteur,
) -> Result<MoyenneCalculee, ErreurCalcul> {
    let converties: Vec<EntreeCalcul> = entrees
        .iter()
        .map(|e| {
            if matches!(e.statut, StatutCalcul::Absent | StatutCalcul::ARattraper) {
                EntreeCalcul {
                    evaluation: e.evaluation.clone(),
                    statut: StatutCalcul::Note,
                    valeur_m: Some(0),
                }
            } else {
                e.clone()
            }
        })
        .collect();
    moyenne_cours(&converties, regles)
}

#[cfg(test)]
mod tests;
