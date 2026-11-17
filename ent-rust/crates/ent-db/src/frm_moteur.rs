//! Socle 18, lots FO2 et FO3 — le moteur de formulaires : types, modèles prêts, lint de sensibilité, contrôle des
//! conditions d'affichage, empreinte de structure (`docs/socles/18_FORMULAIRES.md` §11, §21.2, §27).
//!
//! Ce module est **pur** (aucun accès base, aucune horloge) : tout ce qui se vérifie sans état se vérifie ici et se teste
//! sans base. Le service (`frm_service.rs`) l'appelle, la base (`migrations/2364`) le re-vérifie.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

pub const NATURES_FORMULAIRE: [&str; 3] = ["information", "collecte", "mixte"];
pub const UNITES: [&str; 5] = [
    "eleve_un_titulaire",
    "eleve_chaque_titulaire",
    "foyer",
    "personne",
    "eleve_lui_meme",
];
pub const CONFIDENTIALITES: [&str; 2] = ["nominatif", "confidentiel"];
pub const RELANCES: [&str; 3] = ["j7_j3_j1", "j3_j1", "aucune"];
pub const OPERATEURS: [&str; 2] = ["egal", "different"];
pub const MODES_CRITERE: [&str; 2] = ["inclure", "exclure"];
pub const TYPES_CRITERE: [&str; 5] = ["etablissement", "niveau", "classe", "groupe", "eleve"];

/// Types de questions du §11.2 que ce lot sait créer. Les autres existent en base (schéma cible) mais restent refusés :
/// `fichier` (quarantaine du socle 7), `creneau` (socle 19), `choix_tarifie` (socle 24, lot FO9).
pub const TYPES_ACTIFS: [&str; 15] = [
    "texte_court",
    "texte_long",
    "choix_unique",
    "choix_multiple",
    "liste",
    "oui_non",
    "nombre",
    "date",
    "heure",
    "echelle",
    "grille",
    "classement",
    "accord",
    "prise_connaissance",
    "information",
];
pub const TYPES_REFUSES: [&str; 3] = ["fichier", "creneau", "choix_tarifie"];

/// Effectif au-delà duquel un formulaire attend la validation d'un autre que son auteur (conception : 200, §27
/// `ent_frm_reglage.seuil_validation`, réglable au lot FO6).
pub const SEUIL_VALIDATION: usize = 200;

/// Limites de structure (le déclencheur et les contraintes de base les répètent par colonne).
pub const MAX_SECTIONS: usize = 30;
pub const MAX_QUESTIONS: usize = 150;
pub const MAX_OPTIONS: usize = 60;
pub const MAX_LIGNES: usize = 40;

pub fn type_a_options(t: &str) -> bool {
    matches!(
        t,
        "choix_unique" | "choix_multiple" | "liste" | "grille" | "classement"
    )
}

/// `oui_non` porte deux options **système**, créées avec la question.
pub fn type_oui_non(t: &str) -> bool {
    t == "oui_non"
}

/// Types dont la réponse est une option choisie : seuls ceux-là peuvent fonder une condition d'affichage.
pub fn type_a_condition(t: &str) -> bool {
    matches!(t, "choix_unique" | "choix_multiple" | "liste" | "oui_non")
}

/// Types qui reçoivent une réponse de la personne (hors texte affiché).
pub fn type_repondable(t: &str) -> bool {
    t != "information"
}

// ---------------------------------------------------------------------------
// Lint de sensibilité (R-FC22)
// ---------------------------------------------------------------------------

/// Classe de sensibilité détectée dans un libellé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sensibilite {
    C0,
    C1,
    C2,
    /// Santé et handicap : jamais dans un formulaire ordinaire (socle 34, désactivé sans hébergeur HDS, D-034).
    C3,
}

impl Sensibilite {
    pub fn code(self) -> &'static str {
        match self {
            Self::C0 => "C0",
            Self::C1 => "C1",
            Self::C2 => "C2",
            Self::C3 => "C3",
        }
    }
}

/// Minuscules sans accents (le lint compare des racines, pas des phrases).
pub fn plier(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            autre => autre,
        })
        .collect()
}

const RACINES_C3: [&str; 16] = [
    "sante",
    "maladie",
    "allergi",
    "medicament",
    "traitement",
    "handicap",
    "diagnostic",
    "vaccin",
    "pathologi",
    "asthm",
    "diabet",
    "epilep",
    "trouble dys",
    "dyslexi",
    "psycholog",
    "medical",
];
const RACINES_C2: [&str; 14] = [
    "religion",
    "religieu",
    "croyance",
    "opinion",
    "politique",
    "syndic",
    "origine ethnique",
    "origine raciale",
    "orientation sexuelle",
    "revenu",
    "situation sociale",
    "casier",
    "judiciaire",
    "boursier",
];

/// Classe la plus élevée détectée dans un libellé ou une aide. Une détection **n'est pas** une preuve d'absence :
/// l'auteur classe chaque question, le lint n'est qu'un garde-fou (§11.2).
pub fn lint_sensibilite(texte: &str) -> Sensibilite {
    let t = plier(texte);
    if RACINES_C3.iter().any(|r| t.contains(r)) {
        Sensibilite::C3
    } else if RACINES_C2.iter().any(|r| t.contains(r)) {
        Sensibilite::C2
    } else {
        Sensibilite::C0
    }
}

pub fn classe_valide(c: &str) -> Option<Sensibilite> {
    match c {
        "C0" => Some(Sensibilite::C0),
        "C1" => Some(Sensibilite::C1),
        "C2" => Some(Sensibilite::C2),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Conditions d'affichage : aucune boucle
// ---------------------------------------------------------------------------

/// Rang d'une question dans la version : (ordre de la section, ordre de la question).
pub type Rang = (i32, i32);

/// Une condition est valide si la question dont elle dépend est **strictement avant** la question conditionnée (même
/// version). Une telle relation ne peut pas faire de boucle ; [`a_une_boucle`] le re-vérifie sur le graphe entier.
pub fn condition_vers_l_avant(rang_question: Rang, rang_depend: Rang) -> bool {
    rang_depend < rang_question
}

/// Cherche une boucle dans un graphe de dépendances `(question, depend_de)`. Parcours en profondeur, itératif.
pub fn a_une_boucle(aretes: &[(Uuid, Uuid)]) -> bool {
    let mut voisins: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (de, vers) in aretes {
        voisins.entry(*de).or_default().push(*vers);
    }
    // 0 = non vu, 1 = en cours, 2 = terminé.
    let mut etat: HashMap<Uuid, u8> = HashMap::new();
    for depart in voisins.keys().copied().collect::<Vec<_>>() {
        if etat.get(&depart).copied().unwrap_or(0) != 0 {
            continue;
        }
        let mut pile: Vec<(Uuid, usize)> = vec![(depart, 0)];
        etat.insert(depart, 1);
        while let Some((noeud, idx)) = pile.pop() {
            let suivants = voisins.get(&noeud).map(Vec::as_slice).unwrap_or(&[]);
            if idx < suivants.len() {
                pile.push((noeud, idx + 1));
                let v = suivants[idx];
                match etat.get(&v).copied().unwrap_or(0) {
                    1 => return true,
                    0 => {
                        etat.insert(v, 1);
                        pile.push((v, 0));
                    }
                    _ => {}
                }
            } else {
                etat.insert(noeud, 2);
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Types échangés
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct NouveauFormulaire {
    pub titre: String,
    #[serde(default)]
    pub description: Option<String>,
    /// `information`, `collecte` ou `mixte` (un formulaire qui contient une question « accord » est `mixte`).
    pub nature: String,
    pub unite: String,
    #[serde(default)]
    pub confidentialite: Option<String>,
    /// Destinataire des réponses d'un formulaire confidentiel (un membre du personnel de l'établissement).
    #[serde(default)]
    pub destinataire_confidentiel_user_id: Option<Uuid>,
    #[serde(default)]
    pub ouverture: Option<DateTime<Utc>>,
    #[serde(default)]
    pub echeance: Option<DateTime<Utc>>,
    #[serde(default)]
    pub relances: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormulaireVue {
    pub id: Uuid,
    pub titre: String,
    pub description: Option<String>,
    pub nature: String,
    pub unite: String,
    pub confidentialite: String,
    pub statut: String,
    pub modele_code: Option<String>,
    pub ouverture: Option<DateTime<Utc>>,
    pub echeance: Option<DateTime<Utc>>,
    pub relances: String,
    pub audience_nb: Option<i32>,
    pub auteur_user_id: Uuid,
    /// Numéro de la dernière version publiée.
    pub version_publiee: Option<i32>,
    /// Une version en brouillon existe (la première, ou une modification après publication).
    pub brouillon: bool,
    pub cree_le: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NouvelleSection {
    pub titre: String,
    #[serde(default)]
    pub aide: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NouvelleQuestion {
    pub section_id: Uuid,
    #[serde(rename = "type")]
    pub type_question: String,
    pub libelle: String,
    #[serde(default)]
    pub aide: Option<String>,
    #[serde(default)]
    pub obligatoire: bool,
    /// `C0`, `C1` ou `C2` ; absent : `C0`. Le lint peut exiger davantage (C3 refusée).
    #[serde(default)]
    pub sensibilite: Option<String>,
    #[serde(default)]
    pub min_nombre: Option<f64>,
    #[serde(default)]
    pub max_nombre: Option<f64>,
    #[serde(default)]
    pub longueur_max: Option<i32>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub lignes: Vec<String>,
    /// Question « accord » (FRM-124) : code d'un objet d'accord **actif** de l'établissement.
    #[serde(default)]
    pub objet_accord_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NouvelleCondition {
    pub question_id: Uuid,
    pub depend_de_question_id: Uuid,
    pub option_id: Uuid,
    pub operateur: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CritereAudience {
    /// `inclure` ou `exclure`.
    pub mode: String,
    /// `etablissement`, `niveau`, `classe`, `groupe` ou `eleve`.
    #[serde(rename = "type")]
    pub type_critere: String,
    #[serde(default)]
    pub cible_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApercuAudience {
    /// Élèves visés une fois les exclusions retirées.
    pub total: usize,
    /// Élèves retirés par un critère « exclure » (le nombre seulement, jamais leurs noms : exclus fondus).
    pub exclus: usize,
    /// Au-delà du seuil, la publication attend la validation d'un autre que l'auteur.
    pub validation_requise: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptionVue {
    pub id: Uuid,
    pub ordre: i32,
    pub libelle: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConditionVue {
    pub id: Uuid,
    pub depend_de_question_id: Uuid,
    pub option_id: Uuid,
    pub operateur: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuestionVue {
    pub id: Uuid,
    pub section_id: Uuid,
    pub ordre: i32,
    #[serde(rename = "type")]
    pub type_question: String,
    pub libelle: String,
    pub aide: Option<String>,
    pub obligatoire: bool,
    pub sensibilite: String,
    pub min_nombre: Option<f64>,
    pub max_nombre: Option<f64>,
    pub longueur_max: Option<i32>,
    pub objet_accord_code: Option<String>,
    pub options: Vec<OptionVue>,
    pub lignes: Vec<OptionVue>,
    pub conditions: Vec<ConditionVue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionVue {
    pub id: Uuid,
    pub ordre: i32,
    pub titre: String,
    pub aide: Option<String>,
    pub questions: Vec<QuestionVue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructureVue {
    pub formulaire_id: Uuid,
    pub version_id: Uuid,
    pub numero: i32,
    pub publiee: bool,
    pub sections: Vec<SectionVue>,
}

/// Ce que voit un répondant : la structure sans la sensibilité interne ni les codes de gestion.
#[derive(Debug, Clone, Serialize)]
pub struct BilanPublication {
    pub formulaire_id: Uuid,
    pub version_numero: i32,
    pub statut: String,
    pub cibles: usize,
}

// ---------------------------------------------------------------------------
// Modèles prêts (FRM-121, D-023)
// ---------------------------------------------------------------------------

pub struct ModeleQuestion {
    pub type_question: &'static str,
    pub libelle: &'static str,
    pub aide: Option<&'static str>,
    pub obligatoire: bool,
    pub options: &'static [&'static str],
    pub min_nombre: Option<f64>,
    pub max_nombre: Option<f64>,
}

pub struct ModeleSection {
    pub titre: &'static str,
    pub questions: &'static [ModeleQuestion],
}

pub struct Modele {
    pub code: &'static str,
    pub titre: &'static str,
    pub description: &'static str,
    pub nature: &'static str,
    pub unite: &'static str,
    pub sections: &'static [ModeleSection],
    /// Objet d'accord que le modèle exige (question « accord » ajoutée en tête), s'il y en a un.
    pub objet_accord: Option<&'static str>,
}

const fn q(
    t: &'static str,
    libelle: &'static str,
    obligatoire: bool,
    options: &'static [&'static str],
) -> ModeleQuestion {
    ModeleQuestion {
        type_question: t,
        libelle,
        aide: None,
        obligatoire,
        options,
        min_nombre: None,
        max_nombre: None,
    }
}

pub const MODELES: &[Modele] = &[
    Modele {
        code: "coupon_reunion",
        titre: "Coupon-réponse de réunion",
        description: "Réponse des familles à une invitation à une réunion.",
        nature: "collecte",
        unite: "eleve_un_titulaire",
        objet_accord: None,
        sections: &[ModeleSection {
            titre: "Votre réponse",
            questions: &[
                q("oui_non", "Je participerai à la réunion", true, &[]),
                ModeleQuestion {
                    type_question: "nombre",
                    libelle: "Nombre de personnes présentes",
                    aide: Some("Vous comprise ou compris."),
                    obligatoire: false,
                    options: &[],
                    min_nombre: Some(0.0),
                    max_nombre: Some(10.0),
                },
                q("texte_long", "Un mot pour l'équipe (facultatif)", false, &[]),
            ],
        }],
    },
    Modele {
        code: "choix_atelier",
        titre: "Choix d'un atelier",
        description: "L'élève ou sa famille choisit un atelier parmi ceux proposés.",
        nature: "collecte",
        unite: "eleve_un_titulaire",
        objet_accord: None,
        sections: &[ModeleSection {
            titre: "Atelier",
            questions: &[q("choix_unique", "Atelier souhaité", true, &["Atelier 1", "Atelier 2", "Atelier 3"])],
        }],
    },
    Modele {
        code: "enquete_satisfaction",
        titre: "Enquête de satisfaction",
        description: "Avis de chaque responsable sur un temps de la vie de l'établissement.",
        nature: "collecte",
        unite: "eleve_chaque_titulaire",
        objet_accord: None,
        sections: &[ModeleSection {
            titre: "Votre avis",
            questions: &[
                ModeleQuestion {
                    type_question: "echelle",
                    libelle: "Dans l'ensemble, vous êtes satisfait",
                    aide: Some("De 1 (pas du tout) à 5 (tout à fait)."),
                    obligatoire: true,
                    options: &[],
                    min_nombre: Some(1.0),
                    max_nombre: Some(5.0),
                },
                q("texte_long", "Ce qui pourrait être amélioré", false, &[]),
            ],
        }],
    },
    Modele {
        code: "sortie",
        titre: "Sortie scolaire",
        description: "Information et autorisation pour une sortie facultative (exige l'objet d'accord « sortie facultative »).",
        nature: "mixte",
        unite: "eleve_un_titulaire",
        objet_accord: Some("sortie_facultative"),
        sections: &[ModeleSection {
            titre: "La sortie",
            questions: &[
                q("information", "Date, lieu, horaires et encadrement de la sortie : à compléter avant publication", false, &[]),
                q("texte_long", "Remarque pour l'équipe (facultatif)", false, &[]),
            ],
        }],
    },
    Modele {
        code: "invitation",
        titre: "Invitation",
        description: "Invitation à un événement, avec réponse.",
        nature: "collecte",
        unite: "eleve_un_titulaire",
        objet_accord: None,
        sections: &[ModeleSection {
            titre: "Invitation",
            questions: &[
                q("oui_non", "Nous serons présents", true, &[]),
                ModeleQuestion {
                    type_question: "nombre",
                    libelle: "Nombre d'invités",
                    aide: None,
                    obligatoire: false,
                    options: &[],
                    min_nombre: Some(0.0),
                    max_nombre: Some(20.0),
                },
            ],
        }],
    },
];

pub fn modele(code: &str) -> Option<&'static Modele> {
    MODELES.iter().find(|m| m.code == code)
}

// ---------------------------------------------------------------------------
// Validation d'une question (pure)
// ---------------------------------------------------------------------------

/// Contrôle d'une question avant écriture : type, libellé, bornes, options, lignes, sensibilité, objet d'accord.
/// Rend la sensibilité **retenue** (la plus élevée entre celle de l'auteur et celle que le lint détecte, sans jamais
/// descendre sous ce que l'auteur a déclaré).
pub fn controler_question(
    q: &NouvelleQuestion,
    auteur_direction: bool,
) -> Result<Sensibilite, &'static str> {
    let t = q.type_question.as_str();
    if TYPES_REFUSES.contains(&t) {
        return Err(
            "ce type de question n'est pas encore disponible (fichier, créneau, choix tarifé)",
        );
    }
    if !TYPES_ACTIFS.contains(&t) {
        return Err("type de question inconnu");
    }
    let libelle = q.libelle.trim();
    if libelle.is_empty() || libelle.chars().count() > 500 {
        return Err("libellé invalide (1 à 500 caractères)");
    }
    if q.aide.as_deref().is_some_and(|a| a.chars().count() > 1000) {
        return Err("aide trop longue (1000 caractères au plus)");
    }
    if (t == "accord") != q.objet_accord_code.is_some() {
        return Err("une question « accord » désigne un objet d'accord, et seulement elle");
    }
    if t == "information" && q.obligatoire {
        return Err("un texte affiché n'attend pas de réponse : il ne peut être obligatoire");
    }
    if let (Some(a), Some(b)) = (q.min_nombre, q.max_nombre) {
        if a > b {
            return Err("bornes inversées");
        }
    }
    if (q.min_nombre.is_some() || q.max_nombre.is_some()) && !matches!(t, "nombre" | "echelle") {
        return Err("des bornes ne se posent que sur un nombre ou une échelle");
    }
    if t == "echelle" && (q.min_nombre.is_none() || q.max_nombre.is_none()) {
        return Err("une échelle déclare ses bornes (1 à 5 par exemple)");
    }
    if let Some(l) = q.longueur_max {
        if !matches!(t, "texte_court" | "texte_long") {
            return Err("une longueur maximale ne se pose que sur un texte");
        }
        let plafond = if t == "texte_court" { 200 } else { 4000 };
        if !(1..=plafond).contains(&l) {
            return Err("longueur maximale hors des limites du type");
        }
    }
    if type_a_options(t) {
        if q.options.len() < 2 || q.options.len() > MAX_OPTIONS {
            return Err("au moins deux options (60 au plus)");
        }
        let mut vues = HashSet::new();
        for o in &q.options {
            let o = o.trim();
            if o.is_empty() || o.chars().count() > 200 || !vues.insert(plier(o)) {
                return Err("option vide, trop longue ou en double");
            }
        }
    } else if !q.options.is_empty() && t != "oui_non" {
        return Err("ce type de question ne porte pas d'options");
    }
    if t == "oui_non" && !q.options.is_empty() {
        return Err("« oui / non » a ses deux options système : ne pas en fournir");
    }
    if t == "grille" {
        if q.lignes.is_empty() || q.lignes.len() > MAX_LIGNES {
            return Err("une grille a au moins une ligne (40 au plus)");
        }
        let mut vues = HashSet::new();
        for l in &q.lignes {
            let l = l.trim();
            if l.is_empty() || l.chars().count() > 200 || !vues.insert(plier(l)) {
                return Err("ligne vide, trop longue ou en double");
            }
        }
    } else if !q.lignes.is_empty() {
        return Err("seule une grille porte des lignes");
    }
    // Sensibilité : C3 refusée, C2 réservée à la direction, jamais une déclaration inférieure à la détection.
    let declaree = match q.sensibilite.as_deref() {
        None => Sensibilite::C0,
        Some(c) => classe_valide(c).ok_or("sensibilité inconnue (C0, C1 ou C2)")?,
    };
    let detectee = lint_sensibilite(&format!("{} {}", libelle, q.aide.as_deref().unwrap_or("")));
    if detectee == Sensibilite::C3 {
        return Err("donnée de santé : interdite dans un formulaire ordinaire (socle 34, désactivé sans hébergeur HDS)");
    }
    let retenue = declaree.max(detectee);
    if retenue == Sensibilite::C2 && declaree != Sensibilite::C2 {
        return Err("cette question semble porter sur une donnée sensible : classez-la en C2 (direction seulement)");
    }
    if retenue == Sensibilite::C2 && !auteur_direction {
        return Err("une question sensible (C2) est réservée à la direction");
    }
    Ok(retenue)
}

// ---------------------------------------------------------------------------
// Empreinte de structure
// ---------------------------------------------------------------------------

/// Éléments à encoder pour l'empreinte d'une version. L'ordre est canonique (sections, questions, options, lignes,
/// conditions triées) : deux versions identiques ont la même empreinte, quels que soient les identifiants.
pub fn elements_empreinte(s: &StructureVue, formulaire_titre: &str) -> Vec<String> {
    let mut e = vec![formulaire_titre.to_string()];
    // Les conditions se désignent par rang (section, ordre), jamais par identifiant.
    let mut rang_question: BTreeMap<Uuid, (i32, i32)> = BTreeMap::new();
    let mut option_ordre: BTreeMap<Uuid, i32> = BTreeMap::new();
    for sec in &s.sections {
        for q in &sec.questions {
            rang_question.insert(q.id, (sec.ordre, q.ordre));
            for o in &q.options {
                option_ordre.insert(o.id, o.ordre);
            }
        }
    }
    for sec in &s.sections {
        e.push(format!(
            "S{}|{}|{}",
            sec.ordre,
            sec.titre,
            sec.aide.clone().unwrap_or_default()
        ));
        for q in &sec.questions {
            e.push(format!(
                "Q{}.{}|{}|{}|{}|{}|{}|{:?}|{:?}|{:?}|{}",
                sec.ordre,
                q.ordre,
                q.type_question,
                q.libelle,
                q.aide.clone().unwrap_or_default(),
                u8::from(q.obligatoire),
                q.sensibilite,
                q.min_nombre,
                q.max_nombre,
                q.longueur_max,
                q.objet_accord_code.clone().unwrap_or_default(),
            ));
            for o in &q.options {
                e.push(format!(
                    "O{}.{}.{}|{}",
                    sec.ordre, q.ordre, o.ordre, o.libelle
                ));
            }
            for l in &q.lignes {
                e.push(format!(
                    "L{}.{}.{}|{}",
                    sec.ordre, q.ordre, l.ordre, l.libelle
                ));
            }
            let mut conds: Vec<String> = q
                .conditions
                .iter()
                .map(|c| {
                    let r = rang_question
                        .get(&c.depend_de_question_id)
                        .copied()
                        .unwrap_or((0, 0));
                    let o = option_ordre.get(&c.option_id).copied().unwrap_or(0);
                    format!(
                        "C{}.{}|{}.{}|{}|{}",
                        sec.ordre, q.ordre, r.0, r.1, o, c.operateur
                    )
                })
                .collect();
            conds.sort();
            e.extend(conds);
        }
    }
    e
}

// ---------------------------------------------------------------------------
// Audience : validation pure
// ---------------------------------------------------------------------------

pub fn controler_critere(c: &CritereAudience) -> Result<(), &'static str> {
    if !MODES_CRITERE.contains(&c.mode.as_str()) {
        return Err("mode de critère inconnu (inclure ou exclure)");
    }
    if !TYPES_CRITERE.contains(&c.type_critere.as_str()) {
        return Err("type de critère inconnu");
    }
    if (c.type_critere == "etablissement") != c.cible_id.is_none() {
        return Err("« établissement » n'a pas de cible, les autres en exigent une");
    }
    if c.type_critere == "etablissement" && c.mode == "exclure" {
        return Err("exclure tout l'établissement n'a pas de sens");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Réponse : visibilité des questions et contrôle des valeurs (pur)
// ---------------------------------------------------------------------------

use chrono::{NaiveDate, NaiveTime};

#[derive(Debug, Clone, Deserialize)]
pub struct ReponseLigne {
    pub ligne_id: Uuid,
    pub option_id: Uuid,
}

/// Réponse à une question (hors « accord », qui a sa propre saisie). Seuls les champs du type de la question sont lus.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ValeurSaisie {
    pub question_id: Uuid,
    #[serde(default)]
    pub option_ids: Vec<Uuid>,
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub nombre: Option<f64>,
    #[serde(default)]
    pub date: Option<NaiveDate>,
    #[serde(default)]
    pub heure: Option<NaiveTime>,
    #[serde(default)]
    pub grille: Vec<ReponseLigne>,
    /// Classement : les options, de la première à la dernière.
    #[serde(default)]
    pub classement: Vec<Uuid>,
    /// Prise de connaissance : « j'ai pris connaissance ».
    #[serde(default)]
    pub confirme: Option<bool>,
}

/// Une ligne de valeur à écrire (une colonne typée renseignée, comme la base l'exige).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LigneValeur {
    pub option_id: Option<Uuid>,
    pub ligne_id: Option<Uuid>,
    pub rang: Option<i16>,
    pub texte: Option<String>,
    pub nombre: Option<f64>,
    pub date: Option<NaiveDate>,
    pub heure: Option<NaiveTime>,
}

/// Contrôle d'une réponse selon le type de sa question et rend les lignes à écrire. Une question facultative laissée vide
/// n'est pas passée ici (l'appelant ne l'appelle que pour une réponse donnée).
pub fn controler_valeur(
    q: &QuestionVue,
    v: &ValeurSaisie,
) -> Result<Vec<LigneValeur>, &'static str> {
    let appartient = |id: &Uuid| q.options.iter().any(|o| o.id == *id);
    let plusieurs_fois = |ids: &[Uuid]| ids.iter().collect::<HashSet<_>>().len() != ids.len();
    match q.type_question.as_str() {
        "texte_court" | "texte_long" => {
            let t = v
                .texte
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .ok_or("une réponse écrite est attendue")?;
            let plafond = q
                .longueur_max
                .unwrap_or(if q.type_question == "texte_court" {
                    200
                } else {
                    4000
                }) as usize;
            if t.chars().count() > plafond {
                return Err("réponse trop longue");
            }
            Ok(vec![LigneValeur {
                texte: Some(t.to_string()),
                ..Default::default()
            }])
        }
        "nombre" | "echelle" => {
            let n = v
                .nombre
                .filter(|n| n.is_finite())
                .ok_or("un nombre est attendu")?;
            if q.min_nombre.is_some_and(|m| n < m) || q.max_nombre.is_some_and(|m| n > m) {
                return Err("nombre hors des bornes");
            }
            if q.type_question == "echelle" && n.fract() != 0.0 {
                return Err("une échelle se note par un nombre entier");
            }
            Ok(vec![LigneValeur {
                nombre: Some(n),
                ..Default::default()
            }])
        }
        "date" => Ok(vec![LigneValeur {
            date: Some(v.date.ok_or("une date est attendue")?),
            ..Default::default()
        }]),
        "heure" => Ok(vec![LigneValeur {
            heure: Some(v.heure.ok_or("une heure est attendue")?),
            ..Default::default()
        }]),
        "choix_unique" | "liste" | "oui_non" => match v.option_ids.as_slice() {
            [o] if appartient(o) => Ok(vec![LigneValeur {
                option_id: Some(*o),
                ..Default::default()
            }]),
            _ => Err("une seule option de la question est attendue"),
        },
        "choix_multiple" => {
            if v.option_ids.is_empty()
                || plusieurs_fois(&v.option_ids)
                || !v.option_ids.iter().all(appartient)
            {
                return Err("une ou plusieurs options distinctes de la question sont attendues");
            }
            Ok(v.option_ids
                .iter()
                .map(|o| LigneValeur {
                    option_id: Some(*o),
                    ..Default::default()
                })
                .collect())
        }
        "grille" => {
            if v.grille.is_empty() {
                return Err("la grille attend au moins une réponse");
            }
            let lignes: Vec<Uuid> = v.grille.iter().map(|g| g.ligne_id).collect();
            if plusieurs_fois(&lignes)
                || !v.grille.iter().all(|g| {
                    q.lignes.iter().any(|l| l.id == g.ligne_id) && appartient(&g.option_id)
                })
            {
                return Err("une réponse par ligne de la grille, avec une option de la question");
            }
            if q.obligatoire && v.grille.len() != q.lignes.len() {
                return Err("toutes les lignes de la grille doivent être renseignées");
            }
            Ok(v.grille
                .iter()
                .map(|g| LigneValeur {
                    ligne_id: Some(g.ligne_id),
                    option_id: Some(g.option_id),
                    ..Default::default()
                })
                .collect())
        }
        "classement" => {
            if v.classement.len() != q.options.len()
                || plusieurs_fois(&v.classement)
                || !v.classement.iter().all(appartient)
            {
                return Err("le classement range toutes les options, chacune une fois");
            }
            Ok(v.classement
                .iter()
                .enumerate()
                .map(|(i, o)| LigneValeur {
                    option_id: Some(*o),
                    rang: Some(i as i16 + 1),
                    ..Default::default()
                })
                .collect())
        }
        "prise_connaissance" => {
            if v.confirme != Some(true) {
                return Err("la prise de connaissance se confirme");
            }
            Ok(vec![LigneValeur {
                nombre: Some(1.0),
                ..Default::default()
            }])
        }
        _ => Err("cette question ne reçoit pas de réponse de cette façon"),
    }
}

/// Questions affichées selon les réponses données (`choix` : options choisies par question). Les conditions d'une question
/// s'ajoutent (toutes doivent tenir) ; une question dépendant d'une question elle-même masquée n'a pas de réponse à regarder :
/// « égal » est faux, « différent » est vrai. Une seule passe suffit : une condition ne regarde que vers l'avant.
pub fn questions_visibles(s: &StructureVue, choix: &HashMap<Uuid, HashSet<Uuid>>) -> HashSet<Uuid> {
    let mut visibles = HashSet::new();
    for sec in &s.sections {
        for q in &sec.questions {
            let ok = q.conditions.iter().all(|c| {
                let choisie = visibles.contains(&c.depend_de_question_id)
                    && choix
                        .get(&c.depend_de_question_id)
                        .is_some_and(|set| set.contains(&c.option_id));
                if c.operateur == "egal" {
                    choisie
                } else {
                    !choisie
                }
            });
            if ok {
                visibles.insert(q.id);
            }
        }
    }
    visibles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(t: &str, libelle: &str) -> NouvelleQuestion {
        NouvelleQuestion {
            section_id: Uuid::nil(),
            type_question: t.into(),
            libelle: libelle.into(),
            aide: None,
            obligatoire: false,
            sensibilite: None,
            min_nombre: None,
            max_nombre: None,
            longueur_max: None,
            options: vec![],
            lignes: vec![],
            objet_accord_code: None,
        }
    }

    #[test]
    fn le_lint_classe_la_sante_et_les_donnees_sensibles() {
        assert_eq!(
            lint_sensibilite("Votre enfant a-t-il une Allergie alimentaire ?"),
            Sensibilite::C3
        );
        assert_eq!(
            lint_sensibilite("Quelle est votre religion"),
            Sensibilite::C2
        );
        assert_eq!(
            lint_sensibilite("Situation sociale de la famille"),
            Sensibilite::C2
        );
        assert_eq!(
            lint_sensibilite("Heure d'arrivée souhaitée"),
            Sensibilite::C0
        );
        assert_eq!(plier("Élève Hélène à Noël"), "eleve helene a noel");
    }

    #[test]
    fn controle_de_question_refuse_la_sante_et_reserve_c2_a_la_direction() {
        let sante = question("texte_court", "Maladie chronique ?");
        assert!(controler_question(&sante, true)
            .unwrap_err()
            .contains("santé"));
        let religion = question("texte_court", "Votre religion ?");
        assert!(
            controler_question(&religion, true)
                .unwrap_err()
                .contains("C2"),
            "à classer"
        );
        let mut classee = question("texte_court", "Votre religion ?");
        classee.sensibilite = Some("C2".into());
        assert!(controler_question(&classee, false)
            .unwrap_err()
            .contains("direction"));
        assert_eq!(controler_question(&classee, true), Ok(Sensibilite::C2));
        assert_eq!(
            controler_question(&question("texte_court", "Prénom du frère"), false),
            Ok(Sensibilite::C0)
        );
    }

    #[test]
    fn controle_de_question_types_options_et_bornes() {
        assert!(controler_question(&question("fichier", "Pièce"), true).is_err());
        assert!(controler_question(&question("inconnu", "X"), true).is_err());
        let mut choix = question("choix_unique", "Atelier");
        assert!(
            controler_question(&choix, true).is_err(),
            "deux options au moins"
        );
        choix.options = vec!["A".into(), "a".into()];
        assert!(
            controler_question(&choix, true).is_err(),
            "doublon insensible à la casse"
        );
        choix.options = vec!["A".into(), "B".into()];
        assert!(controler_question(&choix, true).is_ok());
        let mut oui = question("oui_non", "Venez-vous");
        oui.options = vec!["Oui".into(), "Non".into()];
        assert!(controler_question(&oui, true).is_err(), "options système");
        let mut ech = question("echelle", "Note");
        assert!(
            controler_question(&ech, true).is_err(),
            "bornes obligatoires"
        );
        ech.min_nombre = Some(5.0);
        ech.max_nombre = Some(1.0);
        assert!(controler_question(&ech, true).is_err(), "bornes inversées");
        ech.min_nombre = Some(1.0);
        ech.max_nombre = Some(5.0);
        assert!(controler_question(&ech, true).is_ok());
        let mut grille = question("grille", "Matières");
        grille.options = vec!["Bien".into(), "Moyen".into()];
        assert!(
            controler_question(&grille, true).is_err(),
            "une grille a des lignes"
        );
        grille.lignes = vec!["Maths".into()];
        assert!(controler_question(&grille, true).is_ok());
        let mut accord = question("accord", "Droit à l'image");
        assert!(
            controler_question(&accord, true).is_err(),
            "un accord désigne un objet"
        );
        accord.objet_accord_code = Some("droit_image".into());
        assert!(controler_question(&accord, true).is_ok());
        let mut texte = question("texte_court", "Nom");
        texte.objet_accord_code = Some("droit_image".into());
        assert!(
            controler_question(&texte, true).is_err(),
            "un objet d'accord n'accompagne que le type accord"
        );
    }

    #[test]
    fn une_condition_ne_regarde_que_vers_l_avant_et_la_boucle_est_detectee() {
        assert!(condition_vers_l_avant((1, 2), (1, 1)));
        assert!(condition_vers_l_avant((2, 1), (1, 9)));
        assert!(!condition_vers_l_avant((1, 1), (1, 1)));
        assert!(!condition_vers_l_avant((1, 1), (1, 2)));
        let (a, b, c) = (Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));
        assert!(!a_une_boucle(&[(b, a), (c, b)]));
        assert!(a_une_boucle(&[(b, a), (a, b)]));
        assert!(a_une_boucle(&[(a, b), (b, c), (c, a)]));
        assert!(a_une_boucle(&[(a, a)]));
        assert!(!a_une_boucle(&[]));
        assert!(
            !a_une_boucle(&[(b, a), (c, a), (c, b)]),
            "un losange n'est pas une boucle"
        );
    }

    #[test]
    fn les_modeles_sont_coherents_avec_le_controle_de_question() {
        for m in MODELES {
            assert!(NATURES_FORMULAIRE.contains(&m.nature), "{}", m.code);
            assert!(UNITES.contains(&m.unite), "{}", m.code);
            assert_eq!(
                m.nature == "mixte",
                m.objet_accord.is_some(),
                "{} : mixte si et seulement si un accord",
                m.code
            );
            for s in m.sections {
                for mq in s.questions {
                    let nq = NouvelleQuestion {
                        section_id: Uuid::nil(),
                        type_question: mq.type_question.into(),
                        libelle: mq.libelle.into(),
                        aide: mq.aide.map(String::from),
                        obligatoire: mq.obligatoire,
                        sensibilite: None,
                        min_nombre: mq.min_nombre,
                        max_nombre: mq.max_nombre,
                        longueur_max: None,
                        options: mq.options.iter().map(|o| (*o).to_string()).collect(),
                        lignes: vec![],
                        objet_accord_code: None,
                    };
                    assert!(
                        controler_question(&nq, false).is_ok(),
                        "{} : {}",
                        m.code,
                        mq.libelle
                    );
                }
            }
        }
        assert_eq!(MODELES.len(), 5);
        assert!(modele("sortie").is_some() && modele("inconnu").is_none());
    }

    #[test]
    fn les_criteres_d_audience_se_controlent() {
        let c = |mode: &str, t: &str, id: Option<Uuid>| CritereAudience {
            mode: mode.into(),
            type_critere: t.into(),
            cible_id: id,
        };
        assert!(controler_critere(&c("inclure", "etablissement", None)).is_ok());
        assert!(controler_critere(&c("inclure", "classe", Some(Uuid::nil()))).is_ok());
        assert!(controler_critere(&c("inclure", "classe", None)).is_err());
        assert!(controler_critere(&c("inclure", "etablissement", Some(Uuid::nil()))).is_err());
        assert!(controler_critere(&c("exclure", "etablissement", None)).is_err());
        assert!(controler_critere(&c("autre", "classe", Some(Uuid::nil()))).is_err());
        assert!(controler_critere(&c("inclure", "famille", Some(Uuid::nil()))).is_err());
    }

    fn q_choix(t: &str, nb: usize) -> QuestionVue {
        QuestionVue {
            id: Uuid::from_u128(100),
            section_id: Uuid::nil(),
            ordre: 1,
            type_question: t.into(),
            libelle: "Q".into(),
            aide: None,
            obligatoire: false,
            sensibilite: "C0".into(),
            min_nombre: Some(1.0),
            max_nombre: Some(5.0),
            longueur_max: None,
            objet_accord_code: None,
            options: (0..nb)
                .map(|i| OptionVue {
                    id: Uuid::from_u128(200 + i as u128),
                    ordre: i as i32 + 1,
                    libelle: format!("O{i}"),
                })
                .collect(),
            lignes: (0..2)
                .map(|i| OptionVue {
                    id: Uuid::from_u128(300 + i as u128),
                    ordre: i + 1,
                    libelle: format!("L{i}"),
                })
                .collect(),
            conditions: vec![],
        }
    }

    #[test]
    fn controle_des_valeurs_par_type() {
        let o = |i: u128| Uuid::from_u128(200 + i);
        let v = |f: &dyn Fn(&mut ValeurSaisie)| {
            let mut x = ValeurSaisie {
                question_id: Uuid::from_u128(100),
                ..Default::default()
            };
            f(&mut x);
            x
        };
        // choix unique : une option de la question, pas deux, pas une étrangère.
        let q = q_choix("choix_unique", 3);
        assert!(controler_valeur(&q, &v(&|x| x.option_ids = vec![o(1)])).is_ok());
        assert!(controler_valeur(&q, &v(&|x| x.option_ids = vec![o(1), o(2)])).is_err());
        assert!(controler_valeur(&q, &v(&|x| x.option_ids = vec![Uuid::from_u128(999)])).is_err());
        // choix multiple : distinctes.
        let q = q_choix("choix_multiple", 3);
        assert_eq!(
            controler_valeur(&q, &v(&|x| x.option_ids = vec![o(0), o(2)]))
                .unwrap()
                .len(),
            2
        );
        assert!(controler_valeur(&q, &v(&|x| x.option_ids = vec![o(0), o(0)])).is_err());
        // échelle : entier dans les bornes.
        let q = q_choix("echelle", 0);
        assert!(controler_valeur(&q, &v(&|x| x.nombre = Some(3.0))).is_ok());
        assert!(controler_valeur(&q, &v(&|x| x.nombre = Some(3.5))).is_err());
        assert!(controler_valeur(&q, &v(&|x| x.nombre = Some(6.0))).is_err());
        assert!(controler_valeur(&q, &v(&|x| x.nombre = Some(f64::NAN))).is_err());
        // texte : non vide, longueur.
        let mut q = q_choix("texte_court", 0);
        q.longueur_max = Some(5);
        assert!(controler_valeur(&q, &v(&|x| x.texte = Some("  ".into()))).is_err());
        assert!(controler_valeur(&q, &v(&|x| x.texte = Some("abcdef".into()))).is_err());
        assert!(controler_valeur(&q, &v(&|x| x.texte = Some("abc".into()))).is_ok());
        // classement : permutation complète.
        let q = q_choix("classement", 3);
        assert_eq!(
            controler_valeur(&q, &v(&|x| x.classement = vec![o(2), o(0), o(1)])).unwrap()[0].rang,
            Some(1)
        );
        assert!(controler_valeur(&q, &v(&|x| x.classement = vec![o(2), o(0)])).is_err());
        // grille obligatoire : toutes les lignes.
        let mut q = q_choix("grille", 2);
        q.obligatoire = true;
        let l = |i: u128| Uuid::from_u128(300 + i);
        assert!(controler_valeur(
            &q,
            &v(&|x| x.grille = vec![ReponseLigne {
                ligne_id: l(0),
                option_id: o(0)
            }])
        )
        .is_err());
        assert!(controler_valeur(
            &q,
            &v(&|x| x.grille = vec![
                ReponseLigne {
                    ligne_id: l(0),
                    option_id: o(0)
                },
                ReponseLigne {
                    ligne_id: l(1),
                    option_id: o(1)
                }
            ])
        )
        .is_ok());
        // prise de connaissance : confirmée.
        let q = q_choix("prise_connaissance", 0);
        assert!(controler_valeur(&q, &v(&|x| x.confirme = Some(false))).is_err());
        assert!(controler_valeur(&q, &v(&|x| x.confirme = Some(true))).is_ok());
        // information et accord ne reçoivent pas de valeur ici.
        assert!(controler_valeur(&q_choix("accord", 0), &v(&|_| {})).is_err());
        assert!(controler_valeur(&q_choix("information", 0), &v(&|_| {})).is_err());
    }

    #[test]
    fn la_visibilite_suit_les_conditions_et_masque_en_cascade() {
        let (q1, q2, q3) = (Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));
        let (oui, non) = (Uuid::from_u128(11), Uuid::from_u128(12));
        let mk = |id: Uuid, ordre: i32, conds: Vec<ConditionVue>| QuestionVue {
            id,
            section_id: Uuid::nil(),
            ordre,
            type_question: "oui_non".into(),
            libelle: "Q".into(),
            aide: None,
            obligatoire: false,
            sensibilite: "C0".into(),
            min_nombre: None,
            max_nombre: None,
            longueur_max: None,
            objet_accord_code: None,
            options: vec![],
            lignes: vec![],
            conditions: conds,
        };
        let cond = |dep: Uuid, o: Uuid, op: &str| ConditionVue {
            id: Uuid::nil(),
            depend_de_question_id: dep,
            option_id: o,
            operateur: op.into(),
        };
        let s = StructureVue {
            formulaire_id: Uuid::nil(),
            version_id: Uuid::nil(),
            numero: 1,
            publiee: true,
            sections: vec![SectionVue {
                id: Uuid::nil(),
                ordre: 1,
                titre: "S".into(),
                aide: None,
                questions: vec![
                    mk(q1, 1, vec![]),
                    mk(q2, 2, vec![cond(q1, oui, "egal")]),
                    mk(q3, 3, vec![cond(q2, oui, "egal")]),
                ],
            }],
        };
        let mut choix: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
        choix.insert(q1, HashSet::from([oui]));
        choix.insert(q2, HashSet::from([oui]));
        let v = questions_visibles(&s, &choix);
        assert!(v.contains(&q1) && v.contains(&q2) && v.contains(&q3));
        choix.insert(q1, HashSet::from([non]));
        let v = questions_visibles(&s, &choix);
        assert!(v.contains(&q1) && !v.contains(&q2), "q2 masquée");
        assert!(
            !v.contains(&q3),
            "q3 masquée en cascade même si une ancienne réponse à q2 traîne"
        );
        // « différent » : vrai quand rien n'est choisi.
        let s2 = StructureVue {
            sections: vec![SectionVue {
                questions: vec![
                    mk(q1, 1, vec![]),
                    mk(q2, 2, vec![cond(q1, oui, "different")]),
                ],
                ..s.sections[0].clone()
            }],
            ..s.clone()
        };
        assert!(questions_visibles(&s2, &HashMap::new()).contains(&q2));
    }

    #[test]
    fn l_empreinte_ne_depend_pas_des_identifiants() {
        let struct_avec = |ids: [u128; 3]| {
            let (q1, o1, q2) = (
                Uuid::from_u128(ids[0]),
                Uuid::from_u128(ids[1]),
                Uuid::from_u128(ids[2]),
            );
            StructureVue {
                formulaire_id: Uuid::nil(),
                version_id: Uuid::nil(),
                numero: 1,
                publiee: false,
                sections: vec![SectionVue {
                    id: Uuid::nil(),
                    ordre: 1,
                    titre: "S".into(),
                    aide: None,
                    questions: vec![
                        QuestionVue {
                            id: q1,
                            section_id: Uuid::nil(),
                            ordre: 1,
                            type_question: "oui_non".into(),
                            libelle: "Q1".into(),
                            aide: None,
                            obligatoire: true,
                            sensibilite: "C0".into(),
                            min_nombre: None,
                            max_nombre: None,
                            longueur_max: None,
                            objet_accord_code: None,
                            options: vec![OptionVue {
                                id: o1,
                                ordre: 1,
                                libelle: "Oui".into(),
                            }],
                            lignes: vec![],
                            conditions: vec![],
                        },
                        QuestionVue {
                            id: q2,
                            section_id: Uuid::nil(),
                            ordre: 2,
                            type_question: "texte_court".into(),
                            libelle: "Q2".into(),
                            aide: None,
                            obligatoire: false,
                            sensibilite: "C0".into(),
                            min_nombre: None,
                            max_nombre: None,
                            longueur_max: None,
                            objet_accord_code: None,
                            options: vec![],
                            lignes: vec![],
                            conditions: vec![ConditionVue {
                                id: Uuid::nil(),
                                depend_de_question_id: q1,
                                option_id: o1,
                                operateur: "egal".into(),
                            }],
                        },
                    ],
                }],
            }
        };
        let a = elements_empreinte(&struct_avec([1, 2, 3]), "Titre");
        let b = elements_empreinte(&struct_avec([10, 20, 30]), "Titre");
        assert_eq!(a, b);
        assert_ne!(
            a,
            elements_empreinte(&struct_avec([1, 2, 3]), "Autre titre")
        );
    }
}
