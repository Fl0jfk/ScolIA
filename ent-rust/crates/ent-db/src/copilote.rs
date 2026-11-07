//! Copilote ENT — assistant **lecture seule** sur les données centrales.
//!
//! Déterministe (pas de LLM requis) : on détecte une intention dans une
//! question en français, puis on interroge les tables centrales **toujours**
//! filtrées par l'`etablissement_id` de la session. Aucune écriture.
//!
//! Le rôle de l'acteur est vérifié **par intention** (un prof ne lit pas les
//! impayés). Le repli « inconnu » renvoie la liste des questions supportées.
//!
//! Ce module ne contient que l'analyse pure (testable sans base) et les DTO.
//! L'exécution des requêtes vit dans [`crate::PgEnt::copilote_repondre`].

use serde::Serialize;
use serde_json::Value;

/// Nombre de jours par défaut pour la fenêtre « absences non justifiées ».
pub const COPILOTE_JOURS_DEFAUT: u32 = 30;

/// Intention détectée dans une question du personnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intention {
    /// Combien d'élèves (total ou dans une classe).
    Effectif { classe: Option<String> },
    /// Absences non justifiées sur une fenêtre glissante.
    AbsencesNonJustifiees {
        classe: Option<String>,
        jours: u32,
    },
    /// Factures non soldées (reste dû).
    Impayes,
    /// Élèves actifs sans compte ENT.
    SansCompte,
    /// Alertes CPE ouvertes.
    AlertesCpe,
    /// Liste des questions supportées.
    Capacites,
    /// Question non reconnue → aide.
    Inconnu,
}

/// Réponse du copilote — structurée et reproductible.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReponseCopilote {
    /// Intention retenue (`effectif`, `absences_non_justifiees`, …).
    pub intention: String,
    /// Question d'origine (traçabilité).
    pub question: String,
    /// Réponse en français, prête à afficher.
    pub reponse: String,
    /// Données brutes (compteurs, listes) pour l'UI / les tests.
    pub donnees: Value,
    /// `true` si le rôle de l'acteur ne permet pas cette question.
    pub refusee: bool,
}

impl ReponseCopilote {
    pub fn new(
        intention: &str,
        question: &str,
        reponse: impl Into<String>,
        donnees: Value,
    ) -> Self {
        Self {
            intention: intention.to_string(),
            question: question.to_string(),
            reponse: reponse.into(),
            donnees,
            refusee: false,
        }
    }

    pub fn refus(intention: &str, question: &str) -> Self {
        Self {
            intention: intention.to_string(),
            question: question.to_string(),
            reponse: "Votre rôle ne permet pas de consulter cette information.".to_string(),
            donnees: Value::Null,
            refusee: true,
        }
    }
}

/// Questions supportées, telles qu'affichées à l'utilisateur.
pub const CAPACITES: &[&str] = &[
    "Combien d'élèves dans la 4B ? / Combien d'élèves au total ?",
    "Absences non justifiées de la 4B ? / Absences non justifiées depuis 7 jours ?",
    "Factures impayées ? (reste dû)",
    "Combien d'élèves sans compte ?",
    "Alertes CPE ouvertes ?",
];

/// Analyse (pure) d'une question : accents/pluriels ignorés.
pub fn analyser(question: &str) -> Intention {
    let q = normaliser(question);
    let classe = extraire_classe(&q);
    let jours = extraire_jours(&q);

    // « aide » / capacités : question sur le copilote lui-même.
    if contient_un(&q, &["aide", "au secours", "que peux", "que sais", "capacite", "capacites"]) {
        return Intention::Capacites;
    }

    if contient_un(&q, &["sans compte", "sans comptes", "pas de compte", "compte manquant"]) {
        return Intention::SansCompte;
    }

    if contient_un(&q, &["alerte", "alertes", "cpe"]) {
        return Intention::AlertesCpe;
    }

    if contient_un(
        &q,
        &["impaye", "impayes", "impayee", "reste a payer", "reste du", "non soldee", "non soldees", "solde du"],
    ) {
        return Intention::Impayes;
    }

    // Absences : « absence » ou « absent », y compris « non justifie ».
    if contient_un(&q, &["absence", "absences", "absent", "absents", "non justifie", "non justifiee", "non justifiees"]) {
        let jours = jours.unwrap_or(COPILOTE_JOURS_DEFAUT);
        // « justifie » seul (sans « non ») reste utile : on garde l'intention
        // absences, le libellé final précisera « non justifiées ».
        return Intention::AbsencesNonJustifiees { classe, jours };
    }

    // Effectif : « combien … eleve » ou « effectif ».
    if q.contains("effectif") || (q.contains("combien") && q.contains("eleve")) {
        return Intention::Effectif { classe };
    }

    Intention::Inconnu
}

fn contient_un(q: &str, motifs: &[&str]) -> bool {
    motifs.iter().any(|m| q.contains(m))
}

/// Minuscule + repli des accents (comparaison robuste aux saisies).
pub fn normaliser(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ÿ' | 'ý' => 'y',
            'ñ' => 'n',
            'œ' => 'o',
            _ => c,
        })
        .collect()
}

/// Détecte une classe type `4B`, `5A`, `3C`, `6A` (ou `2ND`, `1ERE`, `TLE`).
pub fn extraire_classe(q_normalisee: &str) -> Option<String> {
    for tok in q_normalisee.split(|c: char| !c.is_alphanumeric()) {
        if tok.is_empty() {
            continue;
        }
        let up = tok.to_ascii_uppercase();
        match up.as_str() {
            "2ND" | "2NDE" | "2DE" => return Some("2ND".into()),
            "1ERE" | "1ER" | "1RE" => return Some("1ERE".into()),
            "TLE" | "TERM" | "TERMINALE" => return Some("TLE".into()),
            _ => {}
        }
        let mut it = up.chars();
        let first = it.next()?;
        if first.is_ascii_digit() {
            let rest: String = it.collect();
            if (1..=2).contains(&rest.len()) && rest.chars().all(|c| c.is_ascii_alphabetic()) {
                return Some(format!("{first}{rest}"));
            }
        }
    }
    None
}

/// Détecte `depuis N jours` (ou `sur N jours`, `N derniers jours`).
pub fn extraire_jours(q_normalisee: &str) -> Option<u32> {
    let toks: Vec<&str> = q_normalisee
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    for (i, tok) in toks.iter().enumerate() {
        if let Ok(n) = tok.parse::<u32>() {
            if (1..=365).contains(&n) {
                // « N jours » / « N derniers jours » / « N dernieres jours »
                if toks.get(i + 1).map(|t| t.starts_with("jour")).unwrap_or(false)
                    || toks.get(i + 2).map(|t| t.starts_with("jour")).unwrap_or(false)
                {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// `1250` centimes → `12,50 €` (séparateur de milliers = espace).
pub fn format_eur(centimes: i64) -> String {
    let negatif = centimes < 0;
    let abs = centimes.unsigned_abs();
    let euros = abs / 100;
    let cents = abs % 100;
    let digits = euros.to_string();
    let bytes = digits.as_bytes();
    let mut entier = String::new();
    if negatif {
        entier.push('-');
    }
    for (i, ch) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            entier.push(' ');
        }
        entier.push(*ch as char);
    }
    format!("{entier},{cents:02} €")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyser_effectif_total_et_classe() {
        assert_eq!(
            analyser("Combien d'élèves au total ?"),
            Intention::Effectif { classe: None }
        );
        assert_eq!(
            analyser("combien d'eleves dans la 4B ?"),
            Intention::Effectif {
                classe: Some("4B".into())
            }
        );
        assert_eq!(
            analyser("Effectif de la 3C stp"),
            Intention::Effectif {
                classe: Some("3C".into())
            }
        );
    }

    #[test]
    fn analyser_absences_avec_fenetre() {
        assert_eq!(
            analyser("Absences non justifiées depuis 7 jours"),
            Intention::AbsencesNonJustifiees {
                classe: None,
                jours: 7
            }
        );
        assert_eq!(
            analyser("Qui a des absences non justifiees en 5A sur 30 jours ?"),
            Intention::AbsencesNonJustifiees {
                classe: Some("5A".into()),
                jours: 30
            }
        );
        // Sans nombre → fenêtre par défaut.
        assert_eq!(
            analyser("absences"),
            Intention::AbsencesNonJustifiees {
                classe: None,
                jours: COPILOTE_JOURS_DEFAUT
            }
        );
    }

    #[test]
    fn analyser_autres_intentions() {
        assert_eq!(analyser("Quelles factures sont impayées ?"), Intention::Impayes);
        assert_eq!(analyser("reste à payer ?"), Intention::Impayes);
        assert_eq!(analyser("élèves sans compte"), Intention::SansCompte);
        assert_eq!(analyser("alertes CPE ouvertes"), Intention::AlertesCpe);
        assert_eq!(analyser("Que peux-tu faire ?"), Intention::Capacites);
        assert_eq!(analyser("bonjour"), Intention::Inconnu);
    }

    #[test]
    fn extraction_classe_et_jours() {
        assert_eq!(extraire_classe(&normaliser("les 6A")), Some("6A".into()));
        assert_eq!(extraire_classe(&normaliser("2nde")), Some("2ND".into()));
        assert_eq!(extraire_classe(&normaliser("bonjour")), None);
        assert_eq!(extraire_jours(&normaliser("depuis 15 jours")), Some(15));
        assert_eq!(extraire_jours(&normaliser("sur 400 jours")), None);
        assert_eq!(extraire_jours(&normaliser("depuis un moment")), None);
    }

    #[test]
    fn format_eur_lisible() {
        assert_eq!(format_eur(0), "0,00 €");
        assert_eq!(format_eur(125), "1,25 €");
        assert_eq!(format_eur(1250), "12,50 €");
        assert_eq!(format_eur(123456), "1 234,56 €");
        assert_eq!(format_eur(-500), "-5,00 €");
    }
}
