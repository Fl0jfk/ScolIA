//! Logique d'écran du paramétrage des absences (socle 15, A11f) : **pure**, sans Leptos ni réseau, testée en natif.
//! Seuil de la mention « répond rarement aux sollicitations », emploi du temps de référence, règles de circuit.
//!
//! Le serveur reste l'autorité (droit `parametrer` pour écrire ; l'organisateur lit le seuil). Aucune règle ne
//! nomme une nature d'absence : le circuit se règle par employeur et par **famille** de nature.

use ent_types::absences_personnels::{CreerRegleCircuitBody, RegleCircuitDto};
use uuid::Uuid;

pub const SEUIL_MIN: i16 = 1;
pub const SEUIL_MAX: i16 = 20;

/// Lit le seuil saisi : un entier de 1 à 20, comme le serveur.
pub fn analyser_seuil(texte: &str) -> Result<i16, String> {
    let v: i16 = texte
        .trim()
        .parse()
        .map_err(|_| "Le seuil est un nombre entier de demandes.".to_string())?;
    if !(SEUIL_MIN..=SEUIL_MAX).contains(&v) {
        return Err(format!(
            "Le seuil va de {SEUIL_MIN} à {SEUIL_MAX} demandes restées sans réponse."
        ));
    }
    Ok(v)
}

/// Phrase qui dit, sans détour, ce que le seuil déclenche.
pub fn phrase_seuil(seuil: i16) -> String {
    let n = if seuil == 1 {
        "1 demande restée sans réponse".to_string()
    } else {
        format!("{seuil} demandes restées sans réponse")
    };
    format!(
        "La mention est posée à partir de {n} sur les douze derniers mois, si elles forment au moins la moitié des demandes traitées. Un refus, même sans motif, est une réponse : il ne compte pas."
    )
}

pub const EMPLOYEURS_CIRCUIT: &[(&str, &str)] = &[
    ("tous", "Tous les personnels"),
    ("etat", "Agents de l'État"),
    ("ogec", "Salariés de l'OGEC"),
];

pub fn libelle_employeur_circuit(code: &str) -> &'static str {
    EMPLOYEURS_CIRCUIT
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("Employeur inconnu", |(_, l)| *l)
}

pub const ROLES_CIRCUIT: &[(&str, &str)] = &[
    ("valideur", "Valideur"),
    ("instructeur", "Instructeur"),
    ("informe", "Informé de l'organisation"),
];

pub const FAMILLES_CIRCUIT: &[(&str, &str)] = &[
    ("", "Toutes les familles de nature"),
    ("sante", "Santé"),
    ("parentalite", "Parentalité"),
    ("famille", "Famille"),
    ("autorisation", "Autorisation d'absence"),
    ("service", "Service"),
    ("representation", "Représentation"),
    ("greve", "Grève"),
    ("conges", "Congés"),
    ("autre", "Autre"),
];

pub fn libelle_famille_circuit(famille: Option<&str>) -> &'static str {
    let code = famille.unwrap_or("");
    FAMILLES_CIRCUIT
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("Famille inconnue", |(_, l)| *l)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieRegle {
    pub employeur: String,
    pub famille: String,
    pub role: String,
    pub agent: String,
    pub rang: String,
}

pub fn composer_regle(s: &SaisieRegle) -> Result<CreerRegleCircuitBody, String> {
    if !EMPLOYEURS_CIRCUIT.iter().any(|(c, _)| *c == s.employeur) {
        return Err("Choisissez les personnels concernés.".to_string());
    }
    if !ROLES_CIRCUIT.iter().any(|(c, _)| *c == s.role) {
        return Err("Choisissez le rôle.".to_string());
    }
    if !FAMILLES_CIRCUIT.iter().any(|(c, _)| *c == s.famille) {
        return Err("Famille de nature inconnue.".to_string());
    }
    let agent_id = Uuid::parse_str(s.agent.trim())
        .map_err(|_| "Choisissez un agent dans la liste.".to_string())?;
    let rang: i16 = if s.rang.trim().is_empty() {
        1
    } else {
        s.rang
            .trim()
            .parse()
            .map_err(|_| "Le rang est un nombre de 1 à 5.".to_string())?
    };
    if !(1..=5).contains(&rang) {
        return Err("Le rang va de 1 à 5.".to_string());
    }
    Ok(CreerRegleCircuitBody {
        employeur: s.employeur.clone(),
        famille: (!s.famille.is_empty()).then(|| s.famille.clone()),
        role: s.role.clone(),
        agent_id,
        rang: Some(rang),
    })
}

/// Règles groupées par rôle (valideur, instructeur, informé), puis par rang, pour la lecture.
pub fn regles_par_role(regles: &[RegleCircuitDto]) -> Vec<(&'static str, Vec<RegleCircuitDto>)> {
    ROLES_CIRCUIT
        .iter()
        .filter_map(|(code, libelle)| {
            let mut g: Vec<RegleCircuitDto> =
                regles.iter().filter(|r| r.role == *code).cloned().collect();
            g.sort_by(|a, b| {
                a.rang
                    .cmp(&b.rang)
                    .then_with(|| a.employeur.cmp(&b.employeur))
            });
            (!g.is_empty()).then_some((*libelle, g))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identifiant distinct à chaque appel (les identifiants aléatoires ne sont pas compilés pour le navigateur).
    fn identifiant() -> Uuid {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(1000);
        Uuid::from_u128(u128::from(N.fetch_add(1, Ordering::Relaxed)))
    }

    #[test]
    fn le_seuil_suit_les_bornes_du_serveur() {
        assert_eq!(analyser_seuil("3"), Ok(3));
        assert_eq!(analyser_seuil(" 20 "), Ok(20));
        assert_eq!(analyser_seuil("1"), Ok(1));
        for mauvais in ["0", "21", "-2", "trois", "", "2,5"] {
            assert!(analyser_seuil(mauvais).is_err(), "{mauvais}");
        }
    }

    #[test]
    fn la_phrase_du_seuil_dit_ce_qu_il_declenche_sans_designer_personne() {
        let t = phrase_seuil(3);
        assert!(t.contains("3 demandes restées sans réponse"));
        assert!(t.contains("douze derniers mois"));
        assert!(t.contains("Un refus"));
        assert!(phrase_seuil(1).contains("1 demande restée sans réponse"));
        let bas = t.to_lowercase();
        for interdit in ["professeur", "enseignant", "nom"] {
            assert!(!bas.contains(interdit), "{interdit}");
        }
    }

    #[test]
    fn une_regle_de_circuit_est_validee_avant_l_envoi() {
        let ok = SaisieRegle {
            employeur: "etat".into(),
            famille: "sante".into(),
            role: "valideur".into(),
            agent: Uuid::from_u128(4).to_string(),
            rang: String::new(),
        };
        let b = composer_regle(&ok).unwrap();
        assert_eq!(b.rang, Some(1), "rang 1 par défaut");
        assert_eq!(b.famille.as_deref(), Some("sante"));
        let toutes = composer_regle(&SaisieRegle {
            famille: String::new(),
            ..ok.clone()
        })
        .unwrap();
        assert_eq!(toutes.famille, None, "toutes les familles");
        for mauvaise in [
            SaisieRegle {
                employeur: "rectorat".into(),
                ..ok.clone()
            },
            SaisieRegle {
                role: "chef".into(),
                ..ok.clone()
            },
            SaisieRegle {
                famille: "maladie".into(),
                ..ok.clone()
            },
            SaisieRegle {
                agent: "Mme Roux".into(),
                ..ok.clone()
            },
            SaisieRegle {
                rang: "6".into(),
                ..ok.clone()
            },
            SaisieRegle {
                rang: "0".into(),
                ..ok.clone()
            },
            SaisieRegle {
                rang: "x".into(),
                ..ok.clone()
            },
        ] {
            assert!(composer_regle(&mauvaise).is_err(), "{mauvaise:?}");
        }
    }

    #[test]
    fn les_regles_se_lisent_par_role_puis_par_rang() {
        let r = |role: &str, rang: i16, employeur: &str| RegleCircuitDto {
            id: identifiant(),
            employeur: employeur.into(),
            famille: None,
            role: role.into(),
            agent_id: Uuid::nil(),
            rang,
        };
        let g = regles_par_role(&[
            r("instructeur", 1, "etat"),
            r("valideur", 2, "tous"),
            r("valideur", 1, "ogec"),
        ]);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].0, "Valideur");
        assert_eq!(g[0].1[0].rang, 1);
        assert_eq!(g[1].0, "Instructeur");
        assert_eq!(libelle_employeur_circuit("ogec"), "Salariés de l'OGEC");
        assert_eq!(
            libelle_famille_circuit(None),
            "Toutes les familles de nature"
        );
        assert_eq!(libelle_famille_circuit(Some("greve")), "Grève");
    }
}
