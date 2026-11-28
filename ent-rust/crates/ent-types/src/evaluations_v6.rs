//! Évaluations, lot V6 « appréciations » (socle 21, §12 et §17.10 de `docs/socles/21_EVALUATIONS.md`). Compile pour
//! `wasm32` (aucune dépendance serveur) : les contrôles de cohérence ([`controler`]) sont déterministes et servent au
//! serveur comme à l'écran (compteur, avertissements avant l'enregistrement).
//!
//! Principes :
//! * **chaque enregistrement est une version** ; l'éditeur de chaque version est gardé (historique en ajout seul) ;
//! * **un brouillon n'est montré qu'à son auteur**, jamais imprimé ni lu par la famille ;
//! * **la fenêtre suit l'état de la période de la classe de l'élève** : une appréciation de cours s'écrit jusqu'à la
//!   clôture des appréciations, une générale ou de vie scolaire jusqu'au conseil ; après, la direction seule corrige,
//!   avec un motif journalisé ;
//! * **les contrôles n'empêchent rien** : ils avertissent l'auteur (texte identique pour plus de 5 élèves, prénom d'un
//!   autre élève, éloge sous 8 de moyenne, information de santé, fait disciplinaire, mot interdit de l'établissement) ;
//! * **jamais de troncature silencieuse** : un texte trop long est refusé avec sa longueur et la limite.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types d'appréciation écrits par ce lot (liste fermée, identique à la contrainte de la base, 465).
pub const TYPES_APPRECIATION: [&str; 5] = [
    "cours",
    "generale",
    "vie_scolaire",
    "intermediaire",
    "dispositif",
];

/// Longueur maximale (en caractères) par type (§12.2 : 400 pour une appréciation de cours, 600 pour la générale ;
/// D-069 : 400 pour les autres types, faute de modèle de bulletin réglé).
pub fn limite_caracteres(type_appreciation: &str) -> usize {
    if type_appreciation == "generale" {
        600
    } else {
        400
    }
}

/// Le type porte-t-il un cours (cours, intermédiaire) ?
pub fn type_avec_cours(type_appreciation: &str) -> bool {
    matches!(type_appreciation, "cours" | "intermediaire")
}

/// Seuil du contrôle « texte identique » (§12.2 : identique pour plus de 5 élèves).
pub const SEUIL_IDENTIQUES: usize = 5;
/// Seuil du contrôle « éloge sous la moyenne » (§12.2 : « excellent travail » sous 8), en millièmes sur 20.
pub const SEUIL_ELOGE_M: i64 = 8000;
/// Nombre maximal de phrases dans la banque d'une personne (coût borné).
pub const MAX_PHRASES_BANQUE: i64 = 500;
/// Nombre maximal de mots interdits par établissement (S-117 : liste courte).
pub const MAX_MOTS_INTERDITS: i64 = 100;

/// Écrire (ou réécrire) l'appréciation d'un élève pour une période. La clé est (période, élève, type, cours) : il n'y a
/// qu'une appréciation vivante par clé ; la réécrire en fait une nouvelle version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaisirAppreciationBody {
    pub periode_id: Uuid,
    pub eleve_id: Uuid,
    /// Un de [`TYPES_APPRECIATION`].
    #[serde(rename = "type")]
    pub type_appreciation: String,
    /// Obligatoire pour `cours` et `intermediaire`, interdit sinon.
    #[serde(default)]
    pub cours_id: Option<Uuid>,
    pub texte: String,
    /// `true` : la version est validée (lisible par l'équipe, imprimée au bulletin) ; `false` : brouillon de l'auteur.
    #[serde(default)]
    pub valider: bool,
    /// Version que l'écran avait lue (absente pour une première saisie) : sans elle sur une appréciation existante, ou si
    /// elle diffère, la saisie est refusée en conflit (deux onglets, deux auteurs).
    #[serde(default)]
    pub version_attendue: Option<i32>,
    /// Correction de la direction après la fenêtre : 10 à 500 caractères, journalisé.
    #[serde(default)]
    pub motif_correction: Option<String>,
}

/// Une appréciation telle que la voit le personnel. L'auteur n'est jamais exposé : seulement « c'est moi ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppreciationV6Dto {
    pub id: Uuid,
    pub periode_id: Uuid,
    pub eleve_id: Uuid,
    #[serde(rename = "type")]
    pub type_appreciation: String,
    pub cours_id: Option<Uuid>,
    pub texte: String,
    /// `brouillon` | `validee`.
    pub etat: String,
    pub version: i32,
    pub aide_ia: bool,
    /// L'acteur a écrit la dernière version.
    pub de_moi: bool,
    pub maj_le: DateTime<Utc>,
    pub longueur: usize,
    pub limite: usize,
}

/// Un avertissement de cohérence (non bloquant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControleDto {
    /// `identique` | `autre_prenom` | `eloge_sous_moyenne` | `sante` | `disciplinaire` | `mot_interdit`.
    pub code: String,
    pub message: String,
}

/// Réponse d'une saisie : l'appréciation enregistrée et les avertissements sur son texte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaisieAppreciationDto {
    pub appreciation: AppreciationV6Dto,
    pub controles: Vec<ControleDto>,
}

/// Une ligne de la saisie en série (EVA-195) : l'élève, sa moyenne du cours à côté, son appréciation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneSerieDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    /// Moyenne du cours (millièmes, sur 20), toutes notes saisies comprises ; `None` sans note comptée.
    pub moyenne_m: Option<i64>,
    pub nb_comptees: usize,
    /// État de la période de la classe de l'élève (`ouverte`, `notes_arretees`, …) ; `None` sans classe.
    pub etat_periode: Option<String>,
    /// L'acteur peut écrire l'appréciation de cours de cet élève maintenant (fenêtre ouverte et droit sur le cours).
    pub saisie_ouverte: bool,
    /// Appréciation validée, ou brouillon de l'acteur.
    pub appreciation: Option<AppreciationV6Dto>,
    /// Un autre auteur tient un brouillon sur cette clé (son texte n'est jamais montré).
    pub brouillon_d_un_autre: bool,
    pub controles: Vec<ControleDto>,
}

/// Saisie en série des appréciations d'un cours pour une période (EVA-195, EVA-200).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerieCoursDto {
    pub cours_id: Uuid,
    pub periode_id: Uuid,
    pub libelle: String,
    pub peut_saisir: bool,
    pub limite: usize,
    pub eleves: Vec<LigneSerieDto>,
}

/// Les appréciations validées d'un élève pour une période (relecture du professeur principal et de la direction,
/// EVA-204), plus les brouillons de l'acteur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppreciationsEleveDto {
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    pub etat_periode: Option<String>,
    pub appreciations: Vec<AppreciationV6Dto>,
}

/// Une version de l'historique. L'éditeur n'est exposé que par « c'est moi ».
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionAppreciationDto {
    pub version: i32,
    pub texte: String,
    pub etat: String,
    pub aide_ia: bool,
    pub de_moi: bool,
    pub le: DateTime<Utc>,
    /// Motif d'une correction de la direction après la fenêtre.
    pub motif_correction: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JeterBrouillonBody {
    pub version_attendue: i32,
}

/// Une phrase de la banque personnelle (EVA-196 ; rangée par personne et établissement, R-V35).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhraseBanqueDto {
    pub id: Uuid,
    pub texte: String,
    pub etiquettes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AjouterPhraseBody {
    /// 1 à 500 caractères.
    pub texte: String,
    /// 200 caractères au plus.
    #[serde(default)]
    pub etiquettes: Option<String>,
}

/// Mot interdit de l'établissement (S-117).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotInterditDto {
    pub id: Uuid,
    pub mot: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AjouterMotInterditBody {
    /// 2 à 60 caractères.
    pub mot: String,
}

/// Remarque de relecture à l'auteur (EVA-204).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemarqueBody {
    /// 3 à 500 caractères.
    pub texte: String,
    /// Version relue (la remarque vise cette version).
    pub version: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemarqueDto {
    pub id: Uuid,
    pub appreciation_id: Uuid,
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    #[serde(rename = "type")]
    pub type_appreciation: String,
    pub cours_id: Option<Uuid>,
    pub version: i32,
    pub texte: String,
    pub cree_le: DateTime<Utc>,
    pub traitee: bool,
    /// L'acteur est l'auteur de la remarque (sinon, il en est le destinataire).
    pub de_moi: bool,
}

// ───────────────────────────── Contrôles de cohérence ─────────────────────────────

/// Ce que le serveur sait autour du texte contrôlé.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContexteControle {
    /// Prénom (d'usage) de l'élève visé : jamais signalé.
    pub prenom_eleve: String,
    /// Prénoms des autres élèves du cours (ou de la classe).
    pub autres_prenoms: Vec<String>,
    /// Moyenne de l'élève dans le cours (millièmes sur 20), si elle existe.
    pub moyenne_m: Option<i64>,
    /// Nombre d'AUTRES élèves dont l'appréciation (même cours, même période, même type) a exactement ce texte.
    pub identiques_ailleurs: usize,
    /// Mots interdits de l'établissement.
    pub mots_interdits: Vec<String>,
}

/// Termes de santé ou de diagnostic (R-V18 : « information de santé, à retirer »).
const TERMES_SANTE: [&str; 24] = [
    "dyslexie",
    "dyslexique",
    "dyspraxie",
    "dyspraxique",
    "dyscalculie",
    "dyscalculique",
    "dysorthographie",
    "dysphasie",
    "tdah",
    "tda",
    "hyperactif",
    "hyperactive",
    "hyperactivite",
    "autiste",
    "autisme",
    "asperger",
    "epilepsie",
    "epileptique",
    "diabete",
    "diabetique",
    "depression",
    "depressif",
    "anorexie",
    "handicap",
];

/// Faits disciplinaires (CE-93 et R-V32 : aucune sanction au bulletin).
const TERMES_DISCIPLINE: [&str; 9] = [
    "exclusion",
    "exclu",
    "exclue",
    "sanction",
    "sanctionne",
    "sanctionnee",
    "blame",
    "conseil de discipline",
    "avertissement disciplinaire",
];

/// Éloges (§12.2 : contradiction avec la moyenne).
const ELOGES: [&str; 9] = [
    "excellent",
    "excellente",
    "tres bon",
    "tres bonne",
    "tres bien",
    "felicitations",
    "remarquable",
    "brillant",
    "brillante",
];

/// Texte normalisé pour la comparaison : minuscules, sans accents, ponctuation en espaces, espaces simples, bordé
/// d'un espace (pour chercher des mots entiers : `" mot "`).
pub fn normaliser(texte: &str) -> String {
    let mut s = String::with_capacity(texte.len() + 2);
    s.push(' ');
    let mut espace = true;
    for c in texte.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            'ñ' => 'n',
            'œ' => {
                s.push('o');
                'e'
            }
            'æ' => {
                s.push('a');
                'e'
            }
            c => c,
        };
        if c.is_alphanumeric() {
            s.push(c);
            espace = false;
        } else if !espace {
            s.push(' ');
            espace = true;
        }
    }
    if !espace {
        s.push(' ');
    }
    s
}

/// Le texte normalisé contient-il le terme comme mot (ou suite de mots) entier ?
fn contient(texte_normalise: &str, terme: &str) -> bool {
    let t = normaliser(terme);
    t.trim().len() >= 2 && texte_normalise.contains(&t)
}

/// Clé de comparaison du contrôle « identique » : le texte normalisé, sans les bords.
pub fn cle_identique(texte: &str) -> String {
    normaliser(texte).trim().to_string()
}

/// Contrôles de cohérence d'un texte (§12.2, §12.3, R-V18, CE-93). Déterministes, non bloquants, sans IA.
pub fn controler(texte: &str, ctx: &ContexteControle) -> Vec<ControleDto> {
    let n = normaliser(texte);
    let mut out = Vec::new();
    if ctx.identiques_ailleurs + 1 > SEUIL_IDENTIQUES {
        out.push(ControleDto {
            code: "identique".into(),
            message: format!(
                "Texte identique pour {} élèves : personnalisez-le si possible.",
                ctx.identiques_ailleurs + 1
            ),
        });
    }
    let soi = normaliser(&ctx.prenom_eleve);
    let mut vus: Vec<String> = Vec::new();
    for p in &ctx.autres_prenoms {
        let np = normaliser(p);
        if np.trim().chars().count() < 3 || np == soi || vus.contains(&np) {
            continue;
        }
        if n.contains(&np) {
            vus.push(np);
            out.push(ControleDto {
                code: "autre_prenom".into(),
                message: format!("Le texte cite « {} », prénom d'un autre élève.", p.trim()),
            });
        }
    }
    if let Some(m) = ctx.moyenne_m {
        if m < SEUIL_ELOGE_M {
            if let Some(e) = ELOGES.iter().find(|e| contient(&n, e)) {
                out.push(ControleDto {
                    code: "eloge_sous_moyenne".into(),
                    message: format!(
                        "« {e} » avec une moyenne sous 8 : le texte contredit-il la moyenne ?"
                    ),
                });
            }
        }
    }
    if let Some(t) = TERMES_SANTE.iter().find(|t| contient(&n, t)) {
        out.push(ControleDto {
            code: "sante".into(),
            message: format!("« {t} » : information de santé, à retirer de l'appréciation."),
        });
    }
    if let Some(t) = TERMES_DISCIPLINE.iter().find(|t| contient(&n, t)) {
        out.push(ControleDto {
            code: "disciplinaire".into(),
            message: format!("« {t} » : un fait disciplinaire n'a pas sa place au bulletin."),
        });
    }
    for m in &ctx.mots_interdits {
        if contient(&n, m) {
            out.push(ControleDto {
                code: "mot_interdit".into(),
                message: format!(
                    "« {} » figure dans la liste des mots à éviter de l'établissement.",
                    m.trim()
                ),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ContexteControle {
        ContexteControle {
            prenom_eleve: "Hannah".into(),
            autres_prenoms: vec!["Ernie".into(), "Su".into(), "Hannah".into(), "Zoé".into()],
            moyenne_m: Some(12000),
            identiques_ailleurs: 0,
            mots_interdits: vec!["paresseux".into()],
        }
    }

    fn codes(v: &[ControleDto]) -> Vec<&str> {
        v.iter().map(|c| c.code.as_str()).collect()
    }

    #[test]
    fn un_texte_ordinaire_ne_declenche_rien() {
        assert!(controler("Bon trimestre, travail régulier.", &ctx()).is_empty());
    }

    #[test]
    fn normalisation_sans_accents_ni_ponctuation() {
        assert_eq!(normaliser("Très BIEN, Zoé !"), " tres bien zoe ");
        assert_eq!(normaliser("  "), " ");
        assert_eq!(cle_identique("Bon  travail."), cle_identique("bon travail"));
    }

    #[test]
    fn identique_au_dela_de_cinq_eleves() {
        let mut c = ctx();
        c.identiques_ailleurs = 4;
        assert!(controler("Bien.", &c).is_empty(), "5 élèves : pas encore");
        c.identiques_ailleurs = 5;
        assert_eq!(codes(&controler("Bien.", &c)), ["identique"]);
    }

    #[test]
    fn prenom_d_un_autre_eleve_mot_entier_seulement() {
        assert_eq!(
            codes(&controler("Travaille bien avec Ernie.", &ctx())),
            ["autre_prenom"]
        );
        assert!(
            controler("Hannah progresse.", &ctx()).is_empty(),
            "son propre prénom"
        );
        assert!(
            controler("Il est sûr de lui.", &ctx()).is_empty(),
            "prénom trop court ignoré"
        );
        assert!(controler("Les zoés", &ctx()).is_empty(), "mot entier");
        assert_eq!(
            codes(&controler("Aide ZOE souvent.", &ctx())),
            ["autre_prenom"]
        );
    }

    #[test]
    fn eloge_sous_huit_de_moyenne() {
        let mut c = ctx();
        c.moyenne_m = Some(7990);
        assert_eq!(
            codes(&controler("Excellent trimestre !", &c)),
            ["eloge_sous_moyenne"]
        );
        c.moyenne_m = Some(8000);
        assert!(controler("Excellent trimestre !", &c).is_empty());
        c.moyenne_m = None;
        assert!(
            controler("Excellent trimestre !", &c).is_empty(),
            "sans moyenne : rien"
        );
    }

    #[test]
    fn sante_disciplinaire_et_mots_interdits() {
        assert_eq!(
            codes(&controler("Élève dyslexique, courageux.", &ctx())),
            ["sante"]
        );
        assert_eq!(
            codes(&controler("Depuis son exclusion, mieux.", &ctx())),
            ["disciplinaire"]
        );
        assert_eq!(
            codes(&controler(
                "Un peu paresseux, passé un conseil de discipline.",
                &ctx()
            )),
            ["disciplinaire", "mot_interdit"]
        );
        assert!(
            controler("Exclusivement à l'oral.", &ctx()).is_empty(),
            "mot entier"
        );
    }

    #[test]
    fn limites_par_type() {
        assert_eq!(limite_caracteres("generale"), 600);
        assert_eq!(limite_caracteres("cours"), 400);
        assert!(type_avec_cours("intermediaire") && !type_avec_cours("vie_scolaire"));
    }
}
