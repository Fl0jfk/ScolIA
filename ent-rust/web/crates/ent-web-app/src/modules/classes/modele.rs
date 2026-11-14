//! Logique d'écran des classes (pure, testée) : tri, filtre, libellés, rapprochement
//! avec une séance de l'emploi du temps. Aucune règle métier : droits et effectifs
//! viennent du serveur.

use ent_web_api::{ClasseApi, EleveClasseApi, RapportAmorcageApi, ServiceApi};

pub use crate::composants_metier::classes::libelle_effectif;

/// Rang d'un niveau pour l'ordre d'affichage (6e → Tle), inconnus en dernier.
fn rang_niveau(niveau: Option<&str>) -> u8 {
    match niveau.map(str::to_lowercase).as_deref() {
        Some("6e") => 0,
        Some("5e") => 1,
        Some("4e") => 2,
        Some("3e") => 3,
        Some("2nde") => 4,
        Some("1re") => 5,
        Some("tle") => 6,
        _ => 99,
    }
}

/// Classes triées par niveau puis par code.
pub fn trier_classes(mut classes: Vec<ClasseApi>) -> Vec<ClasseApi> {
    classes.sort_by(|a, b| {
        rang_niveau(a.niveau.as_deref())
            .cmp(&rang_niveau(b.niveau.as_deref()))
            .then_with(|| a.code.to_lowercase().cmp(&b.code.to_lowercase()))
    });
    classes
}

/// Classes dont le code, le libellé ou le niveau contient la recherche (sans casse).
pub fn filtrer_classes(classes: &[ClasseApi], recherche: &str) -> Vec<ClasseApi> {
    let q = recherche.trim().to_lowercase();
    if q.is_empty() {
        return classes.to_vec();
    }
    classes
        .iter()
        .filter(|c| {
            c.code.to_lowercase().contains(&q)
                || c.libelle.to_lowercase().contains(&q)
                || c.niveau
                    .as_deref()
                    .is_some_and(|n| n.to_lowercase().contains(&q))
        })
        .cloned()
        .collect()
}

/// Description courte sous le titre d'une classe : « 4e · 28 élèves · Professeur principal : … ».
pub fn description_classe(c: &ClasseApi) -> String {
    let mut parts = Vec::new();
    if let Some(n) = c.niveau.as_deref().filter(|n| !n.is_empty()) {
        parts.push(n.to_string());
    }
    parts.push(libelle_effectif(c.effectif));
    if let Some(pp) = c
        .professeur_principal_label
        .as_deref()
        .filter(|p| !p.is_empty())
    {
        parts.push(format!("Professeur principal : {pp}"));
    }
    parts.join(" · ")
}

/// « NOM Prénom » (tableaux).
pub fn identite_eleve(e: &EleveClasseApi) -> String {
    format!("{} {}", e.nom.to_uppercase(), e.prenom)
}

/// Une colonne n'est affichée que si le serveur a renvoyé au moins une valeur
/// (identité minimisée selon le rôle).
pub fn a_des_dates_naissance(eleves: &[EleveClasseApi]) -> bool {
    eleves.iter().any(|e| e.date_naissance.is_some())
}

/// Matière affichée d'un service (« Matière non précisée » si vide).
pub fn matiere_du_service(s: &ServiceApi) -> String {
    if s.matiere_libelle.trim().is_empty() {
        "Matière non précisée".into()
    } else {
        s.matiere_libelle.clone()
    }
}

/// Enseignant affiché d'un service.
pub fn enseignant_du_service(s: &ServiceApi) -> String {
    s.prof_label.clone().unwrap_or_else(|| "Enseignant".into())
}

/// Phrase de bilan de l'amorçage (annoncée aux lecteurs d'écran).
pub fn resume_amorcage(r: &RapportAmorcageApi) -> String {
    let rien = r.classes_creees == 0 && r.affectations_creees == 0 && r.services_crees == 0;
    let mut s = if rien {
        format!("Le référentiel {} est déjà à jour.", r.annee_libelle)
    } else {
        format!(
            "Référentiel {} mis à jour : {} classe(s), {} affectation(s), {} service(s) créé(s).",
            r.annee_libelle, r.classes_creees, r.affectations_creees, r.services_crees
        )
    };
    if r.divergences > 0 {
        s.push_str(&format!(
            " {} élève(s) ont une classe différente de leur affectation.",
            r.divergences
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn classe(code: &str, niveau: Option<&str>, effectif: i64) -> ClasseApi {
        ClasseApi {
            id: Uuid::nil(),
            annee_id: Uuid::nil(),
            code: code.into(),
            libelle: code.into(),
            niveau: niveau.map(Into::into),
            effectif,
            professeur_principal_id: None,
            professeur_principal_label: None,
        }
    }

    #[test]
    fn tri_par_niveau_puis_code() {
        let v = trier_classes(vec![
            classe("ULIS", None, 0),
            classe("3C", Some("3e"), 1),
            classe("6B", Some("6e"), 1),
            classe("6a", Some("6e"), 1),
            classe("Tle S", Some("Tle"), 1),
            classe("2nde1", Some("2nde"), 1),
        ]);
        let codes: Vec<&str> = v.iter().map(|c| c.code.as_str()).collect();
        assert_eq!(codes, ["6a", "6B", "3C", "2nde1", "Tle S", "ULIS"]);
    }

    #[test]
    fn filtre_sans_casse_sur_code_libelle_et_niveau() {
        let mut c = classe("4B", Some("4e"), 12);
        c.libelle = "Quatrième B".into();
        let v = vec![c, classe("5A", Some("5e"), 20)];
        assert_eq!(filtrer_classes(&v, "").len(), 2);
        assert_eq!(filtrer_classes(&v, "  ").len(), 2);
        assert_eq!(filtrer_classes(&v, "4b")[0].code, "4B");
        assert_eq!(filtrer_classes(&v, "quatri")[0].code, "4B");
        assert_eq!(filtrer_classes(&v, "5E")[0].code, "5A");
        assert!(filtrer_classes(&v, "zzz").is_empty());
    }

    #[test]
    fn description_d_une_classe() {
        let mut c = classe("4B", Some("4e"), 12);
        assert_eq!(description_classe(&c), "4e · 12 élèves");
        c.professeur_principal_label = Some("Severus Rogue".into());
        assert_eq!(
            description_classe(&c),
            "4e · 12 élèves · Professeur principal : Severus Rogue"
        );
        assert_eq!(description_classe(&classe("ULIS", None, 0)), "Aucun élève");
    }

    #[test]
    fn bilan_de_l_amorcage() {
        let mut r = RapportAmorcageApi {
            annee_libelle: "2026-2027".into(),
            ..Default::default()
        };
        assert_eq!(
            resume_amorcage(&r),
            "Le référentiel 2026-2027 est déjà à jour."
        );
        r.classes_creees = 3;
        r.affectations_creees = 40;
        r.services_crees = 5;
        r.divergences = 2;
        let s = resume_amorcage(&r);
        assert!(
            s.contains("3 classe(s)")
                && s.contains("40 affectation(s)")
                && s.contains("5 service(s)")
        );
        assert!(s.contains("2 élève(s) ont une classe différente"));
    }

    #[test]
    fn identite_et_colonnes() {
        let e = EleveClasseApi {
            id: Uuid::nil(),
            nom: "Potter".into(),
            prenom: "Harry".into(),
            ine: None,
            date_naissance: None,
            status: "inscrit".into(),
        };
        assert_eq!(identite_eleve(&e), "POTTER Harry");
        assert!(!a_des_dates_naissance(std::slice::from_ref(&e)));
        let avec = EleveClasseApi {
            date_naissance: chrono::NaiveDate::from_ymd_opt(2012, 7, 31),
            ..e
        };
        assert!(a_des_dates_naissance(&[avec]));
    }
}
