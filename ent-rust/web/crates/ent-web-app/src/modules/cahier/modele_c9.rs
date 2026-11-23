//! Logique d'écran pure du lot C9 (progressions et reconduction) : sans Leptos ni réseau, testée en natif.
//!
//! **Aucune règle métier ici** : confidentialité des progressions, partage, séances libres, idempotence de la planification,
//! non-copie des données d'élèves restent au serveur. Ce module lit la saisie des étapes et formate les résultats.

use ent_web_api::cahier_c9_types::*;

/// Lit le champ « une étape par ligne » : `titre | notions | durée` (notions et durée facultatives, durée 1 par défaut).
/// Les lignes vides sont ignorées ; une ligne illisible est signalée avec son numéro.
pub fn lire_etapes(texte: &str) -> Result<Vec<EtapeBody>, String> {
    let mut v = Vec::new();
    for (i, ligne) in texte.lines().enumerate() {
        let l = ligne.trim();
        if l.is_empty() {
            continue;
        }
        let mut champs = l.split('|').map(str::trim);
        let titre = champs.next().unwrap_or("");
        if titre.is_empty() {
            return Err(format!("Ligne {} : le titre de l'étape est vide.", i + 1));
        }
        let notions = champs.next().unwrap_or("").to_string();
        let duree = match champs.next().filter(|d| !d.is_empty()) {
            None => 1,
            Some(d) => d
                .parse::<i16>()
                .map_err(|_| format!("Ligne {} : durée illisible (nombre de séances).", i + 1))?,
        };
        if !(1..=30).contains(&duree) {
            return Err(format!(
                "Ligne {} : une étape dure de 1 à 30 séances.",
                i + 1
            ));
        }
        v.push(EtapeBody {
            id: None,
            titre: titre.to_string(),
            notions,
            duree_seances: duree,
        });
    }
    if v.is_empty() {
        return Err("Saisissez au moins une étape.".into());
    }
    Ok(v)
}

/// Corps de création d'une progression.
pub fn corps_progression(titre: &str, etapes: &str) -> Result<ProgressionBody, String> {
    let t = titre.trim();
    if t.is_empty() {
        return Err("Le titre est obligatoire.".into());
    }
    Ok(ProgressionBody {
        titre: t.to_string(),
        matiere_id: None,
        niveau_id: None,
        annee_id: None,
        etapes: lire_etapes(etapes)?,
    })
}

/// « 6 étapes · 14 séances ».
pub fn phrase_taille(r: &ProgressionResumeDto) -> String {
    format!("{} étape(s) · {} séance(s)", r.nb_etapes, r.nb_seances)
}

/// Résultat d'une planification, lu tel quel : ce qui est créé, ce qui l'était déjà, ce qui manque de séances.
pub fn phrase_plan(p: &PlanDto) -> String {
    let mut m = if p.apercu {
        format!(
            "Aperçu : {} séance(s) recevraient une entrée en brouillon",
            p.affectations.len()
        )
    } else {
        format!(
            "{} entrée(s) créée(s) en brouillon, aucune n'est publiée",
            p.creees
        )
    };
    if p.deja_planifiees > 0 {
        m.push_str(&format!(
            " · {} étape(s) déjà planifiée(s)",
            p.deja_planifiees
        ));
    }
    if !p.sans_seance.is_empty() {
        m.push_str(&format!(
            " · sans séance libre : {}",
            p.sans_seance.join(", ")
        ));
    }
    m.push('.');
    m
}

/// Résultat d'une copie de cours à cours.
pub fn phrase_copie(c: &CopieCoursDto) -> String {
    let mut m = if c.apercu {
        format!(
            "Aperçu : {} entrée(s) seraient copiées en brouillon",
            c.copiees.len()
        )
    } else {
        format!("{} entrée(s) copiée(s) en brouillon", c.copiees.len())
    };
    if c.sans_seance > 0 {
        m.push_str(&format!(
            " · {} sans séance libre de destination",
            c.sans_seance
        ));
    }
    if c.ignorees_autrui > 0 {
        m.push_str(&format!(
            " · {} entrée(s) d'un collègue non copiées",
            c.ignorees_autrui
        ));
    }
    m.push('.');
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn une_etape_par_ligne_avec_notions_et_duree() {
        let e = lire_etapes("Définitions | fractions ; décimaux | 2\n\n Problèmes \nBilan | | 1")
            .unwrap();
        assert_eq!(e.len(), 3);
        assert_eq!(
            (
                e[0].titre.as_str(),
                e[0].notions.as_str(),
                e[0].duree_seances
            ),
            ("Définitions", "fractions ; décimaux", 2)
        );
        assert_eq!(e[1].duree_seances, 1, "durée par défaut");
        assert_eq!(e[2].notions, "");
    }

    #[test]
    fn une_ligne_illisible_est_signalee_avec_son_numero() {
        assert!(lire_etapes("").unwrap_err().contains("au moins une étape"));
        assert!(lire_etapes("A | x | beaucoup")
            .unwrap_err()
            .contains("Ligne 1"));
        assert!(lire_etapes("ok\n | notions | 2")
            .unwrap_err()
            .contains("Ligne 2"));
        assert!(lire_etapes("A | | 31").unwrap_err().contains("1 à 30"));
        assert!(lire_etapes("A | | 0").is_err());
    }

    #[test]
    fn le_titre_de_la_progression_est_obligatoire() {
        assert!(corps_progression("  ", "A").is_err());
        assert_eq!(
            corps_progression(" Fractions ", "A").unwrap().titre,
            "Fractions"
        );
    }

    fn plan(apercu: bool) -> PlanDto {
        PlanDto {
            cours_id: Uuid::nil(),
            progression_id: Uuid::nil(),
            apercu,
            affectations: vec![],
            creees: 4,
            deja_planifiees: 1,
            sans_seance: vec!["Bilan (1 séance(s) manquante(s))".into()],
        }
    }

    #[test]
    fn la_planification_dit_toujours_que_rien_n_est_publie_et_ce_qui_manque() {
        let m = phrase_plan(&plan(false));
        assert!(
            m.contains("4 entrée(s) créée(s) en brouillon") && m.contains("aucune n'est publiée"),
            "{m}"
        );
        assert!(
            m.contains("1 étape(s) déjà planifiée(s)") && m.contains("Bilan"),
            "{m}"
        );
        assert!(phrase_plan(&plan(true)).starts_with("Aperçu"));
    }

    #[test]
    fn la_copie_signale_les_entrees_d_un_collegue_qui_ne_sont_pas_reprises() {
        let c = CopieCoursDto {
            apercu: false,
            copiees: vec![],
            sans_seance: 2,
            ignorees_autrui: 3,
        };
        let m = phrase_copie(&c);
        assert!(
            m.contains("2 sans séance libre")
                && m.contains("3 entrée(s) d'un collègue non copiées"),
            "{m}"
        );
    }
}
