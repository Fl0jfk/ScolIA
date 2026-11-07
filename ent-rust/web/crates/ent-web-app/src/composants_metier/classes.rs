//! Classes du référentiel partagées entre modules (pur, testé) : rapprochement avec une
//! séance de l'emploi du temps et libellés d'effectif. Utilisé par « Classes » et par la
//! feuille d'appel.

use ent_web_api::{ClasseApi, EnseignementApi};

/// « 1 élève », « 28 élèves », « Aucun élève ».
pub fn libelle_effectif(n: i64) -> String {
    match n {
        n if n <= 0 => "Aucun élève".into(),
        1 => "1 élève".into(),
        n => format!("{n} élèves"),
    }
}

/// Classe du référentiel correspondant au texte de classe d'une séance (code exact,
/// sans tenir compte de la casse ni des espaces autour).
pub fn classe_du_creneau<'a>(
    classes: &'a [ClasseApi],
    classe_texte: &str,
) -> Option<&'a ClasseApi> {
    let t = classe_texte.trim().to_lowercase();
    if t.is_empty() {
        return None;
    }
    classes.iter().find(|c| c.code.trim().to_lowercase() == t)
}

/// Enseignement du professeur correspondant à une séance (classe + matière). Renvoie
/// `None` si aucun enseignement ne correspond **ou si plusieurs correspondent** (classe
/// entière et groupe de la même matière) : on n'affiche jamais un effectif douteux.
pub fn enseignement_du_creneau<'a>(
    enseignements: &'a [EnseignementApi],
    classe_texte: &str,
    matiere: &str,
) -> Option<&'a EnseignementApi> {
    let (c, m) = (classe_texte.trim().to_lowercase(), matiere.trim().to_lowercase());
    if c.is_empty() || m.is_empty() {
        return None;
    }
    let mut trouves = enseignements.iter().filter(|e| {
        e.matiere_libelle.trim().to_lowercase() == m
            && (e.cible_code.trim().to_lowercase() == c || e.classe_code.trim().to_lowercase() == c)
    });
    let premier = trouves.next()?;
    trouves.next().is_none().then_some(premier)
}

/// « Allemand · 4B-ALL (groupe) · 12 élèves » : le libellé unique d'un enseignement.
pub fn libelle_enseignement(e: &EnseignementApi) -> String {
    format!("{} · {}", e.libelle, libelle_effectif(e.effectif))
}

/// Complète le sous-titre de la feuille d'appel : « … · Sixième A · 28 élèves ».
/// Le libellé n'est ajouté que s'il apporte une information (différent du code).
pub fn sous_titre_appel(base: &str, classe: Option<&ClasseApi>) -> String {
    let Some(c) = classe else {
        return base.to_string();
    };
    let mut parts = vec![base.to_string()];
    if c.libelle.trim() != c.code.trim() && !c.libelle.trim().is_empty() {
        parts.push(c.libelle.trim().to_string());
    }
    parts.push(libelle_effectif(c.effectif));
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn classe(code: &str, effectif: i64) -> ClasseApi {
        ClasseApi {
            id: Uuid::nil(),
            annee_id: Uuid::nil(),
            code: code.into(),
            libelle: code.into(),
            niveau: None,
            effectif,
            professeur_principal_id: None,
            professeur_principal_label: None,
        }
    }

    #[test]
    fn effectif_au_singulier_et_au_pluriel() {
        assert_eq!(libelle_effectif(0), "Aucun élève");
        assert_eq!(libelle_effectif(-3), "Aucun élève");
        assert_eq!(libelle_effectif(1), "1 élève");
        assert_eq!(libelle_effectif(28), "28 élèves");
    }

    #[test]
    fn rapprochement_d_une_seance() {
        let v = vec![classe("4B", 12), classe("5A", 20)];
        assert_eq!(classe_du_creneau(&v, " 4b ").map(|c| c.effectif), Some(12));
        assert!(classe_du_creneau(&v, "3C").is_none());
        assert!(classe_du_creneau(&v, "").is_none());
    }

    fn ens(cible: &str, classe: &str, matiere: &str, nature: &str, effectif: i64) -> EnseignementApi {
        EnseignementApi {
            service_id: Uuid::nil(),
            nature: nature.into(),
            classe_id: None,
            groupe_id: None,
            cible_code: cible.into(),
            classe_code: classe.into(),
            matiere_id: None,
            matiere_libelle: matiere.into(),
            effectif,
            professeur_principal: false,
            libelle: if nature == "groupe" {
                format!("{matiere} · {cible} (groupe)")
            } else {
                format!("{matiere} · {cible}")
            },
        }
    }

    #[test]
    fn enseignement_d_une_seance_classe_ou_groupe() {
        let v = vec![
            ens("4B-ALL", "4B", "Allemand", "groupe", 12),
            ens("4B", "4B", "Français", "classe", 28),
        ];
        // Séance en texte de classe « 4B » pour l'allemand : le groupe, sans ambiguïté.
        assert_eq!(enseignement_du_creneau(&v, " 4b ", "allemand").map(|e| e.effectif), Some(12));
        assert_eq!(enseignement_du_creneau(&v, "4B", "Français").map(|e| e.effectif), Some(28));
        assert!(enseignement_du_creneau(&v, "4B", "Maths").is_none());
        assert!(enseignement_du_creneau(&v, "", "Français").is_none());
        // Deux enseignements possibles : aucun effectif affiché plutôt qu'un douteux.
        let mut deux = v.clone();
        deux.push(ens("4B", "4B", "Allemand", "classe", 28));
        assert!(enseignement_du_creneau(&deux, "4B", "Allemand").is_none());
        assert_eq!(libelle_enseignement(&v[0]), "Allemand · 4B-ALL (groupe) · 12 élèves");
    }

    #[test]
    fn sous_titre_de_l_appel() {
        let base = "mardi 3 novembre · 08:00 – 09:00";
        assert_eq!(sous_titre_appel(base, None), base);
        let c = classe("4B", 12);
        assert_eq!(
            sous_titre_appel(base, Some(&c)),
            format!("{base} · 12 élèves")
        );
        let mut c = classe("4B", 1);
        c.libelle = "Quatrième B".into();
        assert_eq!(
            sous_titre_appel(base, Some(&c)),
            format!("{base} · Quatrième B · 1 élève")
        );
    }
}
