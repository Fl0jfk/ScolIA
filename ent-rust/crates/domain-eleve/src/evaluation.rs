//! Évaluations (socle 2, lot S2-7) : règles de calcul **explicites** des moyennes,
//! arrondis et exports de livrets. Module pur (aucune base, aucun réseau).
//!
//! # Règles de calcul (testées ci-dessous)
//!
//! 1. Une note est ramenée sur 20 : `valeur / barème × 20`.
//! 2. **Moyenne d'une matière** (élève, période) = Σ(note sur 20 × coefficient du devoir)
//!    / Σ(coefficients des devoirs **notés**). Les résultats « absent », « dispensé »
//!    et « non noté » sont exclus du numérateur **et** du dénominateur. Exception
//!    paramétrable : si `absent_compte_zero`, un « absent » compte 0 avec son coefficient.
//! 3. Aucune note comptée → **pas de moyenne** (jamais 0).
//! 4. **Moyenne générale** = Σ(moyenne de matière × coefficient de matière) / Σ(coefficients
//!    des matières **ayant une moyenne**). Coefficient de matière par défaut : 1.
//! 5. **Moyenne de classe** d'une matière = moyenne arithmétique des moyennes des élèves
//!    qui en ont une ; minimum et maximum sur ces mêmes moyennes.
//! 6. **Calcul exact** (fractions entières) : aucun arrondi intermédiaire ; les moyennes
//!    générales et de classe partent des moyennes **non arrondies**.
//! 7. **Arrondi d'affichage** au centième (défaut), au dixième ou au demi-point, **demi
//!    vers le haut** (12,345 → 12,35 ; 12,25 au dixième → 12,3).
//! 8. Saisie : valeur, barème et coefficient ont **au plus deux décimales** ;
//!    0 ≤ valeur ≤ barème ; barème > 0 ; 0 < coefficient ≤ 100.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

// ---------------------------------------------------------------------------
// Statuts et règles
// ---------------------------------------------------------------------------

/// Résultat d'un élève à une évaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutResultat {
    Note,
    Absent,
    Dispense,
    NonNote,
}

impl StatutResultat {
    pub const TOUS: [StatutResultat; 4] = [Self::Note, Self::Absent, Self::Dispense, Self::NonNote];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Absent => "absent",
            Self::Dispense => "dispense",
            Self::NonNote => "non_note",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|x| x.as_str() == s.trim())
    }

    /// Libellé court imprimé sur le bulletin et le relevé.
    pub fn abreviation(self) -> &'static str {
        match self {
            Self::Note => "",
            Self::Absent => "Abs",
            Self::Dispense => "Disp",
            Self::NonNote => "NN",
        }
    }
}

/// Arrondi d'affichage des moyennes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Arrondi {
    #[default]
    Centieme,
    Dixieme,
    DemiPoint,
}

impl Arrondi {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Centieme => "centieme",
            Self::Dixieme => "dixieme",
            Self::DemiPoint => "demi_point",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "centieme" => Some(Self::Centieme),
            "dixieme" => Some(Self::Dixieme),
            "demi_point" => Some(Self::DemiPoint),
            _ => None,
        }
    }

    /// Nombre d'unités par point (100 : centième ; 10 : dixième ; 2 : demi-point).
    fn pas(self) -> i128 {
        match self {
            Self::Centieme => 100,
            Self::Dixieme => 10,
            Self::DemiPoint => 2,
        }
    }
}

/// Règles de calcul d'un établissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReglesCalcul {
    pub arrondi: Arrondi,
    pub absent_compte_zero: bool,
}

// ---------------------------------------------------------------------------
// Validation de saisie
// ---------------------------------------------------------------------------

/// Erreur de saisie d'un résultat ou d'un devoir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErreurSaisie {
    /// Plus de deux décimales, nombre non fini ou négatif.
    Format,
    /// Valeur hors de [0 ; barème].
    HorsBareme,
    /// Barème ≤ 0 ou > 1000.
    Bareme,
    /// Coefficient ≤ 0 ou > 100.
    Coefficient,
    /// Statut non numérique accompagné d'une valeur, ou note sans valeur.
    Incoherent,
}

/// Valeur en centièmes si elle a au plus deux décimales (et est finie, positive).
pub fn centiemes(x: f64) -> Option<i64> {
    if !x.is_finite() || x < 0.0 || x > 1.0e9 {
        return None;
    }
    let c = (x * 100.0).round();
    if ((c / 100.0) - x).abs() > 1e-7 {
        return None;
    }
    Some(c as i64)
}

/// Contrôle d'un devoir : barème et coefficient.
pub fn valider_devoir(sur: f64, coef: f64) -> Result<(), ErreurSaisie> {
    let s = centiemes(sur).ok_or(ErreurSaisie::Format)?;
    if s <= 0 || s > 100_000 {
        return Err(ErreurSaisie::Bareme);
    }
    let c = centiemes(coef).ok_or(ErreurSaisie::Format)?;
    if c <= 0 || c > 10_000 {
        return Err(ErreurSaisie::Coefficient);
    }
    Ok(())
}

/// Contrôle d'un résultat : la valeur stockée (0 pour un statut non numérique).
pub fn valider_resultat(
    statut: StatutResultat,
    valeur: Option<f64>,
    sur: f64,
) -> Result<f64, ErreurSaisie> {
    match (statut, valeur) {
        (StatutResultat::Note, Some(v)) => {
            let vc = centiemes(v).ok_or(ErreurSaisie::Format)?;
            let sc = centiemes(sur).ok_or(ErreurSaisie::Bareme)?;
            if vc > sc {
                return Err(ErreurSaisie::HorsBareme);
            }
            Ok(vc as f64 / 100.0)
        }
        (StatutResultat::Note, None) => Err(ErreurSaisie::Incoherent),
        (_, Some(_)) => Err(ErreurSaisie::Incoherent),
        (_, None) => Ok(0.0),
    }
}

// ---------------------------------------------------------------------------
// Fractions exactes
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

/// Nombre rationnel positif exact (dénominateur > 0, réduit).
#[derive(Debug, Clone, Copy)]
struct Frac {
    n: i128,
    d: i128,
}

impl Frac {
    fn new(n: i128, d: i128) -> Self {
        debug_assert!(d != 0);
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        let g = pgcd(n, d);
        Frac { n: n / g, d: d / g }
    }
    fn zero() -> Self {
        Frac { n: 0, d: 1 }
    }
    fn entier(n: i128) -> Self {
        Frac { n, d: 1 }
    }
    fn centiemes(c: i64) -> Self {
        Frac::new(c as i128, 100)
    }
    fn add(self, o: Frac) -> Self {
        Frac::new(self.n * o.d + o.n * self.d, self.d * o.d)
    }
    fn mul(self, o: Frac) -> Self {
        Frac::new(self.n * o.n, self.d * o.d)
    }
    fn div(self, o: Frac) -> Self {
        Frac::new(self.n * o.d, self.d * o.n)
    }
    fn est_nul(self) -> bool {
        self.n == 0
    }
    fn en_f64(self) -> f64 {
        self.n as f64 / self.d as f64
    }
    fn cmp(self, o: Frac) -> Ordering {
        (self.n * o.d).cmp(&(o.n * self.d))
    }
}

/// Moyenne exacte (fraction) ; arrondie seulement à l'affichage.
#[derive(Debug, Clone, Copy)]
pub struct Moyenne(Frac);

impl PartialEq for Moyenne {
    fn eq(&self, o: &Self) -> bool {
        self.0.cmp(o.0) == Ordering::Equal
    }
}

impl Moyenne {
    /// Valeur exacte en flottant (tri, comparaisons) — ne pas l'afficher telle quelle.
    pub fn exacte(self) -> f64 {
        self.0.en_f64()
    }

    /// Arrondi d'affichage, demi vers le haut (valeurs positives).
    pub fn arrondie(self, arrondi: Arrondi) -> f64 {
        let pas = arrondi.pas();
        // floor((2·n·pas + d) / (2·d)) : arrondi au plus proche, demi vers le haut.
        let q = (2 * self.0.n * pas + self.0.d).div_euclid(2 * self.0.d);
        q as f64 / pas as f64
    }

    /// Moyenne d'une valeur sur 20 déjà connue (tests, reprises).
    pub fn depuis_centiemes(c: i64) -> Self {
        Moyenne(Frac::centiemes(c))
    }
}

// ---------------------------------------------------------------------------
// Calculs
// ---------------------------------------------------------------------------

/// Un résultat à prendre en compte dans une moyenne de matière.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResultatCalcul {
    pub statut: StatutResultat,
    /// Valeur (ignorée si le statut n'est pas « note »).
    pub valeur: f64,
    pub sur: f64,
    pub coef: f64,
}

/// Règle 2 : moyenne pondérée sur 20 d'une matière. `None` si aucune note ne compte.
/// Une ligne mal formée (barème nul, plus de deux décimales) est ignorée : la saisie
/// l'a déjà refusée ; on ne fabrique jamais une moyenne à partir d'une donnée invalide.
pub fn moyenne_matiere(resultats: &[ResultatCalcul], regles: &ReglesCalcul) -> Option<Moyenne> {
    let mut num = Frac::zero();
    let mut den = Frac::zero();
    for r in resultats {
        let (Some(sur), Some(coef)) = (centiemes(r.sur), centiemes(r.coef)) else {
            continue;
        };
        if sur <= 0 || coef <= 0 {
            continue;
        }
        let valeur = match r.statut {
            StatutResultat::Note => match centiemes(r.valeur) {
                Some(v) if v <= sur => v,
                _ => continue,
            },
            StatutResultat::Absent if regles.absent_compte_zero => 0,
            _ => continue,
        };
        let sur20 = Frac::centiemes(valeur)
            .mul(Frac::entier(20))
            .div(Frac::centiemes(sur));
        let c = Frac::centiemes(coef);
        num = num.add(sur20.mul(c));
        den = den.add(c);
    }
    if den.est_nul() {
        None
    } else {
        Some(Moyenne(num.div(den)))
    }
}

/// Règle 4 : moyenne générale à partir des moyennes de matières (exactes) et de leur
/// coefficient. Les matières sans moyenne ne figurent pas dans la liste.
pub fn moyenne_generale(matieres: &[(Moyenne, f64)]) -> Option<Moyenne> {
    let mut num = Frac::zero();
    let mut den = Frac::zero();
    for (m, coef) in matieres {
        let Some(c) = centiemes(*coef).filter(|c| *c > 0) else {
            continue;
        };
        let c = Frac::centiemes(c);
        num = num.add(m.0.mul(c));
        den = den.add(c);
    }
    if den.est_nul() {
        None
    } else {
        Some(Moyenne(num.div(den)))
    }
}

/// Statistiques d'une série de moyennes (règle 5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Statistiques {
    pub moyenne: Moyenne,
    pub min: Moyenne,
    pub max: Moyenne,
    pub effectif: usize,
}

pub fn statistiques(moyennes: &[Moyenne]) -> Option<Statistiques> {
    let premiere = *moyennes.first()?;
    let mut somme = Frac::zero();
    let (mut min, mut max) = (premiere, premiere);
    for m in moyennes {
        somme = somme.add(m.0);
        if m.0.cmp(min.0) == Ordering::Less {
            min = *m;
        }
        if m.0.cmp(max.0) == Ordering::Greater {
            max = *m;
        }
    }
    Some(Statistiques {
        moyenne: Moyenne(somme.div(Frac::entier(moyennes.len() as i128))),
        min,
        max,
        effectif: moyennes.len(),
    })
}

/// Format français d'une note ou moyenne arrondie : « 12,5 », « 14 », « 9,25 ».
pub fn format_fr(x: f64) -> String {
    let c = (x * 100.0).round() as i64;
    let (ent, dec) = (c / 100, (c % 100).abs());
    if dec == 0 {
        format!("{ent}")
    } else if dec % 10 == 0 {
        format!("{ent},{}", dec / 10)
    } else {
        format!("{ent},{dec:02}")
    }
}

// ---------------------------------------------------------------------------
// Socle commun : composantes du bilan de fin de cycle
// ---------------------------------------------------------------------------

/// Les 8 composantes du socle commun [NV : codes repris de l'usage LSU, à vérifier
/// sur la spécification éditeur].
pub const COMPOSANTES_SOCLE: [(&str, &str); 8] = [
    ("CPD_FRA", "Comprendre, s'exprimer en utilisant la langue française à l'oral et à l'écrit"),
    ("CPD_ETR", "Comprendre, s'exprimer en utilisant une langue étrangère et, le cas échéant, une langue régionale"),
    ("CPD_SCI", "Comprendre, s'exprimer en utilisant les langages mathématiques, scientifiques et informatiques"),
    ("CPD_ART", "Comprendre, s'exprimer en utilisant les langages des arts et du corps"),
    ("MET_APP", "Les méthodes et outils pour apprendre"),
    ("FRM_CIT", "La formation de la personne et du citoyen"),
    ("SYS_NAT", "Les systèmes naturels et les systèmes techniques"),
    ("REP_MND", "Les représentations du monde et l'activité humaine"),
];

pub fn composante_connue(code: &str) -> bool {
    COMPOSANTES_SOCLE.iter().any(|(c, _)| *c == code)
}

/// Niveaux de maîtrise (1 à 4).
pub fn libelle_niveau_maitrise(n: i16) -> &'static str {
    match n {
        1 => "Maîtrise insuffisante",
        2 => "Maîtrise fragile",
        3 => "Maîtrise satisfaisante",
        4 => "Très bonne maîtrise",
        _ => "—",
    }
}

// ---------------------------------------------------------------------------
// Exports de livrets (format documenté, NON validé contre le XSD officiel)
// ---------------------------------------------------------------------------

/// Avertissement porté par chaque export (fichier et réponse de l'API).
pub const AVERTISSEMENT_LIVRETS: &str = "[NV] Format documenté par ScolIA d'après les spécifications \
publiques connues ; non validé contre le schéma XSD officiel (non disponible : réservé aux éditeurs \
validés par le ministère). Ne pas importer dans LSU/LSL sans qualification préalable.";

/// Échappement XML (texte et attributs).
pub fn xml_echapper(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // Caractères de contrôle interdits en XML 1.0 (hors tabulation et sauts).
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct LivretEleve {
    /// Identifiant national (INE) ; vide si inconnu (signalé dans les anomalies).
    pub ine: String,
    pub nom: String,
    pub prenom: String,
    pub division: String,
    pub matieres: Vec<LivretMatiere>,
    pub appreciation_generale: Option<String>,
    pub absences_justifiees: i64,
    pub absences_injustifiees: i64,
    pub retards: i64,
    /// Bilan de fin de cycle validé (composante → niveau 1..4).
    pub bilan_cycle: Option<LivretBilanCycle>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LivretMatiere {
    pub code: String,
    pub libelle: String,
    /// Moyenne de l'élève arrondie (`None` : non évalué).
    pub moyenne: Option<f64>,
    pub moyenne_classe: Option<f64>,
    pub appreciation: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LivretBilanCycle {
    pub cycle: i16,
    pub composantes: Vec<(String, i16)>,
    pub synthese: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LivretPeriode {
    /// UAI de l'établissement (vide si inconnu).
    pub uai: String,
    pub millesime: i32,
    pub periode_code: String,
    pub periode_indice: u32,
    pub nb_periodes: u32,
    pub eleves: Vec<LivretEleve>,
}

fn moyenne_attr(x: Option<f64>) -> String {
    x.map(|m| format!("{m:.2}/20")).unwrap_or_default()
}

/// Export LSU (collège) : structure `lsun-bilans` [NV].
pub fn rendre_lsu_xml(l: &LivretPeriode) -> String {
    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    x.push_str(&format!("<!-- {} -->\n", AVERTISSEMENT_LIVRETS.replace("--", "-")));
    x.push_str("<lsun-bilans xmlns=\"urn:fr:edu:scolarite:lsun:bilans:import\" schemaVersion=\"NV\">\n");
    x.push_str("  <entete>\n");
    x.push_str("    <editeur>ScolIA</editeur>\n    <application>ScolIA ENT</application>\n");
    x.push_str(&format!("    <etablissement>{}</etablissement>\n", xml_echapper(&l.uai)));
    x.push_str("  </entete>\n  <donnees>\n");
    // Disciplines (dédoublonnées par code, ordre d'apparition).
    let mut disciplines: Vec<(String, String)> = Vec::new();
    for e in &l.eleves {
        for m in &e.matieres {
            if !disciplines.iter().any(|(c, _)| *c == m.code) {
                disciplines.push((m.code.clone(), m.libelle.clone()));
            }
        }
    }
    x.push_str("    <eleves>\n");
    for (i, e) in l.eleves.iter().enumerate() {
        x.push_str(&format!(
            "      <eleve id=\"EL{}\" id-be=\"{}\" nom=\"{}\" prenom=\"{}\" code-division=\"{}\"/>\n",
            i + 1,
            xml_echapper(&e.ine),
            xml_echapper(&e.nom),
            xml_echapper(&e.prenom),
            xml_echapper(&e.division)
        ));
    }
    x.push_str("    </eleves>\n");
    x.push_str(&format!(
        "    <periodes>\n      <periode id=\"P1\" millesime=\"{}\" indice=\"{}\" nb-periodes=\"{}\" code=\"{}\"/>\n    </periodes>\n",
        l.millesime,
        l.periode_indice,
        l.nb_periodes,
        xml_echapper(&l.periode_code)
    ));
    x.push_str("    <disciplines>\n");
    for (i, (code, lib)) in disciplines.iter().enumerate() {
        x.push_str(&format!(
            "      <discipline id=\"DI{}\" code=\"{}\" libelle=\"{}\"/>\n",
            i + 1,
            xml_echapper(code),
            xml_echapper(lib)
        ));
    }
    x.push_str("    </disciplines>\n    <bilans-periodiques>\n");
    for (i, e) in l.eleves.iter().enumerate() {
        x.push_str(&format!(
            "      <bilan-periodique eleve-ref=\"EL{}\" periode-ref=\"P1\">\n        <liste-acquis>\n",
            i + 1
        ));
        for m in &e.matieres {
            let idx = disciplines.iter().position(|(c, _)| *c == m.code).unwrap_or(0) + 1;
            let eval = if m.moyenne.is_some() { "false" } else { "true" };
            x.push_str(&format!(
                "          <acquis discipline-ref=\"DI{idx}\" moyenne-eleve=\"{}\" moyenne-structure=\"{}\" eleve-non-note=\"{eval}\">\n",
                moyenne_attr(m.moyenne),
                moyenne_attr(m.moyenne_classe)
            ));
            if let Some(a) = &m.appreciation {
                x.push_str(&format!("            <appreciation>{}</appreciation>\n", xml_echapper(a)));
            }
            x.push_str("          </acquis>\n");
        }
        x.push_str("        </liste-acquis>\n");
        if let Some(a) = &e.appreciation_generale {
            x.push_str(&format!("        <acquis-conseils>{}</acquis-conseils>\n", xml_echapper(a)));
        }
        x.push_str(&format!(
            "        <vie-scolaire nb-retards=\"{}\" nb-abs-justifiees=\"{}\" nb-abs-injustifiees=\"{}\"/>\n",
            e.retards, e.absences_justifiees, e.absences_injustifiees
        ));
        x.push_str("      </bilan-periodique>\n");
    }
    x.push_str("    </bilans-periodiques>\n");
    let bilans: Vec<(usize, &LivretBilanCycle)> = l
        .eleves
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.bilan_cycle.as_ref().map(|b| (i, b)))
        .collect();
    if !bilans.is_empty() {
        x.push_str("    <bilans-cycle>\n");
        for (i, b) in bilans {
            x.push_str(&format!(
                "      <bilan-cycle eleve-ref=\"EL{}\" cycle=\"{}\">\n        <socle>\n",
                i + 1,
                b.cycle
            ));
            for (code, niveau) in &b.composantes {
                x.push_str(&format!(
                    "          <domaine code=\"{}\" positionnement=\"{niveau}\"/>\n",
                    xml_echapper(code)
                ));
            }
            x.push_str("        </socle>\n");
            x.push_str(&format!("        <synthese>{}</synthese>\n", xml_echapper(&b.synthese)));
            x.push_str("      </bilan-cycle>\n");
        }
        x.push_str("    </bilans-cycle>\n");
    }
    x.push_str("  </donnees>\n</lsun-bilans>\n");
    x
}

/// Export LSL (lycée) : format **interne ScolIA** documenté, en l'absence de
/// spécification accessible (accréditation éditeur requise) [NV].
pub fn rendre_lsl_xml(l: &LivretPeriode) -> String {
    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    x.push_str(&format!("<!-- {} -->\n", AVERTISSEMENT_LIVRETS.replace("--", "-")));
    x.push_str(&format!(
        "<scolia-lsl version=\"0.1\" uai=\"{}\" millesime=\"{}\" periode=\"{}\">\n",
        xml_echapper(&l.uai),
        l.millesime,
        xml_echapper(&l.periode_code)
    ));
    for e in &l.eleves {
        x.push_str(&format!(
            "  <eleve ine=\"{}\" nom=\"{}\" prenom=\"{}\" division=\"{}\">\n",
            xml_echapper(&e.ine),
            xml_echapper(&e.nom),
            xml_echapper(&e.prenom),
            xml_echapper(&e.division)
        ));
        for m in &e.matieres {
            x.push_str(&format!(
                "    <discipline code=\"{}\" libelle=\"{}\" moyenne=\"{}\" moyenne-classe=\"{}\">",
                xml_echapper(&m.code),
                xml_echapper(&m.libelle),
                m.moyenne.map(|v| format!("{v:.2}")).unwrap_or_default(),
                m.moyenne_classe.map(|v| format!("{v:.2}")).unwrap_or_default()
            ));
            if let Some(a) = &m.appreciation {
                x.push_str(&xml_echapper(a));
            }
            x.push_str("</discipline>\n");
        }
        x.push_str("  </eleve>\n");
    }
    x.push_str("</scolia-lsl>\n");
    x
}

/// Contrôle structurel minimal d'un document XML produit ici : balises équilibrées,
/// attributs entre guillemets, aucun `&` non échappé. Ne remplace pas une validation XSD.
pub fn xml_bien_forme(doc: &str) -> Result<(), String> {
    let mut pile: Vec<String> = Vec::new();
    let mut reste = doc;
    while let Some(p) = reste.find('<') {
        let texte = &reste[..p];
        verifier_entites(texte)?;
        reste = &reste[p..];
        if let Some(r) = reste.strip_prefix("<?") {
            let f = r.find("?>").ok_or("déclaration non fermée")?;
            reste = &r[f + 2..];
            continue;
        }
        if let Some(r) = reste.strip_prefix("<!--") {
            let f = r.find("-->").ok_or("commentaire non fermé")?;
            if r[..f].contains("--") {
                return Err("« -- » dans un commentaire".into());
            }
            reste = &r[f + 3..];
            continue;
        }
        let f = reste.find('>').ok_or("balise non fermée")?;
        let balise = &reste[1..f];
        reste = &reste[f + 1..];
        if let Some(nom) = balise.strip_prefix('/') {
            let ouvert = pile.pop().ok_or("fermeture sans ouverture")?;
            if ouvert != nom.trim() {
                return Err(format!("fermeture {nom} au lieu de {ouvert}"));
            }
            continue;
        }
        let auto = balise.ends_with('/');
        let corps = balise.trim_end_matches('/');
        let nom: String = corps.chars().take_while(|c| !c.is_whitespace()).collect();
        if nom.is_empty() {
            return Err("balise sans nom".into());
        }
        let attributs = &corps[nom.len()..];
        if attributs.matches('"').count() % 2 != 0 {
            return Err(format!("guillemets déséquilibrés dans <{nom}>"));
        }
        verifier_entites(attributs)?;
        if !auto {
            pile.push(nom);
        }
    }
    verifier_entites(reste)?;
    if let Some(n) = pile.pop() {
        return Err(format!("<{n}> non fermée"));
    }
    Ok(())
}

fn verifier_entites(s: &str) -> Result<(), String> {
    let mut i = 0;
    while let Some(p) = s[i..].find('&') {
        let r = &s[i + p..];
        let ok = ["&amp;", "&lt;", "&gt;", "&quot;", "&apos;"]
            .iter()
            .any(|e| r.starts_with(e));
        if !ok {
            return Err("« & » non échappé".into());
        }
        i += p + 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(v: f64, sur: f64, coef: f64) -> ResultatCalcul {
        ResultatCalcul {
            statut: StatutResultat::Note,
            valeur: v,
            sur,
            coef,
        }
    }

    fn statut(s: StatutResultat, sur: f64, coef: f64) -> ResultatCalcul {
        ResultatCalcul {
            statut: s,
            valeur: 0.0,
            sur,
            coef,
        }
    }

    const R: ReglesCalcul = ReglesCalcul {
        arrondi: Arrondi::Centieme,
        absent_compte_zero: false,
    };

    fn m(r: &[ResultatCalcul]) -> Option<f64> {
        moyenne_matiere(r, &R).map(|x| x.arrondie(Arrondi::Centieme))
    }

    #[test]
    fn aucune_note_pas_de_moyenne() {
        assert_eq!(m(&[]), None);
        assert_eq!(
            m(&[
                statut(StatutResultat::Absent, 20.0, 1.0),
                statut(StatutResultat::Dispense, 20.0, 2.0),
                statut(StatutResultat::NonNote, 20.0, 1.0),
            ]),
            None
        );
    }

    #[test]
    fn coefficients_et_baremes() {
        // 15/20 coef 1, 8/10 coef 2 (=16/20) → (15 + 32) / 3 = 15,666… → 15,67
        assert_eq!(m(&[note(15.0, 20.0, 1.0), note(8.0, 10.0, 2.0)]), Some(15.67));
        // coef décimal : 10/20 coef 0,5 ; 20/20 coef 1,5 → (5 + 30) / 2 = 17,5
        assert_eq!(m(&[note(10.0, 20.0, 0.5), note(20.0, 20.0, 1.5)]), Some(17.5));
        // barème 40 et note décimale : 33,5/40 = 16,75
        assert_eq!(m(&[note(33.5, 40.0, 1.0)]), Some(16.75));
    }

    #[test]
    fn non_notes_exclus_du_denominateur() {
        let r = [
            note(12.0, 20.0, 1.0),
            statut(StatutResultat::Absent, 20.0, 3.0),
            statut(StatutResultat::Dispense, 20.0, 1.0),
            statut(StatutResultat::NonNote, 20.0, 1.0),
        ];
        assert_eq!(m(&r), Some(12.0));
    }

    #[test]
    fn absent_compte_zero_parametrable() {
        let r = [note(12.0, 20.0, 1.0), statut(StatutResultat::Absent, 20.0, 1.0)];
        let regles = ReglesCalcul {
            absent_compte_zero: true,
            ..R
        };
        assert_eq!(
            moyenne_matiere(&r, &regles).map(|x| x.arrondie(Arrondi::Centieme)),
            Some(6.0)
        );
        // Dispensé et non noté restent exclus même dans ce cas.
        let r2 = [note(12.0, 20.0, 1.0), statut(StatutResultat::Dispense, 20.0, 1.0)];
        assert_eq!(
            moyenne_matiere(&r2, &regles).map(|x| x.arrondie(Arrondi::Centieme)),
            Some(12.0)
        );
    }

    #[test]
    fn zero_est_une_note() {
        assert_eq!(m(&[note(0.0, 20.0, 1.0)]), Some(0.0));
        assert_eq!(m(&[note(0.0, 20.0, 1.0), note(20.0, 20.0, 1.0)]), Some(10.0));
    }

    #[test]
    fn arrondis_demi_vers_le_haut_sans_erreur_binaire() {
        // 12,345 n'est pas représentable en binaire : le calcul exact donne 12,35.
        let x = Moyenne::depuis_centiemes(1234) // 12,34
            ;
        assert_eq!(x.arrondie(Arrondi::Centieme), 12.34);
        // (12,34 + 12,35) / 2 = 12,345 → 12,35
        let s = statistiques(&[Moyenne::depuis_centiemes(1234), Moyenne::depuis_centiemes(1235)])
            .unwrap();
        assert_eq!(s.moyenne.arrondie(Arrondi::Centieme), 12.35);
        assert_eq!(Moyenne::depuis_centiemes(1225).arrondie(Arrondi::Dixieme), 12.3);
        assert_eq!(Moyenne::depuis_centiemes(1224).arrondie(Arrondi::Dixieme), 12.2);
        assert_eq!(Moyenne::depuis_centiemes(1225).arrondie(Arrondi::DemiPoint), 12.5);
        assert_eq!(Moyenne::depuis_centiemes(1224).arrondie(Arrondi::DemiPoint), 12.0);
        assert_eq!(Moyenne::depuis_centiemes(1275).arrondie(Arrondi::DemiPoint), 13.0);
        // 2/3 → 0,67 ; 1/3 → 0,33
        assert_eq!(m(&[note(2.0, 3.0, 1.0)]), Some(13.33));
    }

    #[test]
    fn pas_d_arrondi_en_cascade() {
        // Matières : 10,005 (non arrondi) et 10,005 → générale 10,005 → 10,01 ;
        // en partant des valeurs arrondies 10,01 et 10,01 on aurait aussi 10,01, mais
        // 10,004 + 10,004 (arrondies 10,00) doit donner 10,00 et non 10,01.
        let a = moyenne_matiere(&[note(10.004, 20.0, 1.0)], &R);
        assert!(a.is_none(), "trois décimales refusées");
        // 1/3 (6,666…) et 2/3 (13,333…) : générale exacte 10 → 10 (et non 6,67+13,33=20/2=10 par chance)
        let x = moyenne_matiere(&[note(1.0, 3.0, 1.0)], &R).unwrap();
        let y = moyenne_matiere(&[note(2.0, 3.0, 1.0)], &R).unwrap();
        assert_eq!(moyenne_generale(&[(x, 1.0), (y, 1.0)]).unwrap().arrondie(Arrondi::Centieme), 10.0);
        // 1/3 seul coef 2 + 20 coef 1 : (13,333… + 20) / 3 = 11,111… → 11,11
        let z = Moyenne::depuis_centiemes(2000);
        assert_eq!(moyenne_generale(&[(x, 2.0), (z, 1.0)]).unwrap().arrondie(Arrondi::Centieme), 11.11);
    }

    #[test]
    fn moyenne_generale_ignore_coef_invalide_et_vide() {
        assert!(moyenne_generale(&[]).is_none());
        let x = Moyenne::depuis_centiemes(1200);
        assert!(moyenne_generale(&[(x, 0.0)]).is_none());
        assert_eq!(
            moyenne_generale(&[(x, 3.0), (Moyenne::depuis_centiemes(800), 1.0)])
                .unwrap()
                .arrondie(Arrondi::Centieme),
            11.0
        );
    }

    #[test]
    fn statistiques_min_max() {
        assert!(statistiques(&[]).is_none());
        let s = statistiques(&[
            Moyenne::depuis_centiemes(1450),
            Moyenne::depuis_centiemes(800),
            Moyenne::depuis_centiemes(1925),
        ])
        .unwrap();
        assert_eq!(s.effectif, 3);
        assert_eq!(s.min.arrondie(Arrondi::Centieme), 8.0);
        assert_eq!(s.max.arrondie(Arrondi::Centieme), 19.25);
        assert_eq!(s.moyenne.arrondie(Arrondi::Centieme), 13.92);
    }

    #[test]
    fn validation_saisie() {
        assert_eq!(valider_resultat(StatutResultat::Note, Some(12.5), 20.0), Ok(12.5));
        assert_eq!(valider_resultat(StatutResultat::Note, Some(20.0), 20.0), Ok(20.0));
        assert_eq!(
            valider_resultat(StatutResultat::Note, Some(20.01), 20.0),
            Err(ErreurSaisie::HorsBareme)
        );
        assert_eq!(
            valider_resultat(StatutResultat::Note, Some(-1.0), 20.0),
            Err(ErreurSaisie::Format)
        );
        assert_eq!(
            valider_resultat(StatutResultat::Note, Some(12.345), 20.0),
            Err(ErreurSaisie::Format)
        );
        assert_eq!(
            valider_resultat(StatutResultat::Note, Some(f64::NAN), 20.0),
            Err(ErreurSaisie::Format)
        );
        assert_eq!(valider_resultat(StatutResultat::Note, None, 20.0), Err(ErreurSaisie::Incoherent));
        assert_eq!(valider_resultat(StatutResultat::Absent, None, 20.0), Ok(0.0));
        assert_eq!(
            valider_resultat(StatutResultat::Dispense, Some(3.0), 20.0),
            Err(ErreurSaisie::Incoherent)
        );
        assert_eq!(valider_devoir(20.0, 1.0), Ok(()));
        assert_eq!(valider_devoir(0.0, 1.0), Err(ErreurSaisie::Bareme));
        assert_eq!(valider_devoir(20.0, 0.0), Err(ErreurSaisie::Coefficient));
        assert_eq!(valider_devoir(20.0, 101.0), Err(ErreurSaisie::Coefficient));
        assert_eq!(valider_devoir(20.0, 0.333), Err(ErreurSaisie::Format));
        for s in StatutResultat::TOUS {
            assert_eq!(StatutResultat::parse(s.as_str()), Some(s));
        }
    }

    #[test]
    fn ligne_invalide_ignoree() {
        // Une ligne déjà en base mais hors barème (donnée ancienne) n'entre pas dans le calcul.
        assert_eq!(m(&[note(25.0, 20.0, 1.0), note(10.0, 20.0, 1.0)]), Some(10.0));
        assert_eq!(m(&[note(10.0, 0.0, 1.0)]), None);
    }

    #[test]
    fn format_francais() {
        assert_eq!(format_fr(12.5), "12,5");
        assert_eq!(format_fr(14.0), "14");
        assert_eq!(format_fr(9.25), "9,25");
        assert_eq!(format_fr(0.05), "0,05");
    }

    fn livret() -> LivretPeriode {
        LivretPeriode {
            uai: "0750001A".into(),
            millesime: 2026,
            periode_code: "T1".into(),
            periode_indice: 1,
            nb_periodes: 3,
            eleves: vec![LivretEleve {
                ine: "123456789AB".into(),
                nom: "Granger".into(),
                prenom: "Hermione".into(),
                division: "4B".into(),
                matieres: vec![
                    LivretMatiere {
                        code: "MATHS".into(),
                        libelle: "Mathématiques".into(),
                        moyenne: Some(18.5),
                        moyenne_classe: Some(12.25),
                        appreciation: Some("Excellent travail & rigueur <toujours>.".into()),
                    },
                    LivretMatiere {
                        code: "EPS".into(),
                        libelle: "Éducation physique".into(),
                        moyenne: None,
                        moyenne_classe: Some(14.0),
                        appreciation: None,
                    },
                ],
                appreciation_generale: Some("Trimestre remarquable.".into()),
                absences_justifiees: 2,
                absences_injustifiees: 0,
                retards: 1,
                bilan_cycle: Some(LivretBilanCycle {
                    cycle: 4,
                    composantes: vec![("CPD_FRA".into(), 4), ("MET_APP".into(), 3)],
                    synthese: "Socle maîtrisé.".into(),
                }),
            }],
        }
    }

    /// Régénère les fichiers de référence : `SCOLIA_REGENERER_FIXTURES=1 cargo test -p domain-eleve`.
    /// À faire seulement quand le format documenté change (relire le diff).
    #[test]
    fn regenerer_fixtures_si_demande() {
        if std::env::var("SCOLIA_REGENERER_FIXTURES").as_deref() != Ok("1") {
            return;
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        std::fs::write(dir.join("lsu_reference_scolia.xml"), rendre_lsu_xml(&livret())).unwrap();
        std::fs::write(dir.join("lsl_reference_scolia.xml"), rendre_lsl_xml(&livret())).unwrap();
    }

    /// Conformité au **format documenté** (fichier de référence du dépôt) : aucun fichier
    /// de référence officiel n'existe dans le dépôt (XSD réservé aux éditeurs validés).
    #[test]
    fn lsu_conforme_au_fichier_de_reference() {
        let xml = rendre_lsu_xml(&livret());
        xml_bien_forme(&xml).unwrap();
        let attendu = include_str!("../tests/fixtures/lsu_reference_scolia.xml");
        assert_eq!(xml, attendu);
        assert!(xml.contains("&amp; rigueur &lt;toujours&gt;"));
        assert!(xml.contains("[NV]"));
    }

    #[test]
    fn lsl_conforme_au_fichier_de_reference() {
        let xml = rendre_lsl_xml(&livret());
        xml_bien_forme(&xml).unwrap();
        let attendu = include_str!("../tests/fixtures/lsl_reference_scolia.xml");
        assert_eq!(xml, attendu);
    }

    #[test]
    fn controle_xml_detecte_les_erreurs() {
        assert!(xml_bien_forme("<a><b></a>").is_err());
        assert!(xml_bien_forme("<a>x & y</a>").is_err());
        assert!(xml_bien_forme("<a x=\"1></a>").is_err());
        assert!(xml_bien_forme("<a>").is_err());
        assert!(xml_bien_forme("<a/><b>t</b>").is_ok());
        assert_eq!(xml_echapper("a\u{1}b<"), "ab&lt;");
    }

    #[test]
    fn composantes_du_socle() {
        assert_eq!(COMPOSANTES_SOCLE.len(), 8);
        assert!(composante_connue("SYS_NAT"));
        assert!(!composante_connue("D1"));
        assert_eq!(libelle_niveau_maitrise(3), "Maîtrise satisfaisante");
    }
}
