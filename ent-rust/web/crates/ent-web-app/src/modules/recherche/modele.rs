//! Logique d'écran pure de la recherche globale (lot P5 du socle 29 ; sans Leptos ni réseau).
//! Aucune règle métier : droits, plafonds et filtrage des protections restent au serveur.

use ent_types::recherche::{ResultatRecherche, SourceRecherche};

/// Longueur minimale d'une recherche (le serveur revérifie : en dessous, il ne cherche rien).
pub const MIN_CARACTERES: usize = 2;

/// Texte à envoyer, ou le message à afficher : jamais de requête vide ou d'un seul caractère.
pub fn texte_valide(saisie: &str) -> Result<String, &'static str> {
    let t = saisie.trim();
    if t.chars().count() < MIN_CARACTERES {
        return Err("Saisissez au moins deux caractères.");
    }
    Ok(t.chars().take(100).collect())
}

/// Titre d'un groupe de résultats.
pub fn titre_source(s: SourceRecherche) -> &'static str {
    match s {
        SourceRecherche::Eleves => "Élèves",
        SourceRecherche::Personnes => "Responsables et contacts",
        SourceRecherche::Personnels => "Personnels",
        SourceRecherche::Classes => "Classes",
        SourceRecherche::Groupes => "Groupes",
        SourceRecherche::Salles => "Salles",
    }
}

/// Résultats regroupés par source, dans l'ordre fixe des sources (jamais un classement entre sources).
pub fn grouper(
    resultats: Vec<ResultatRecherche>,
) -> Vec<(SourceRecherche, Vec<ResultatRecherche>)> {
    use SourceRecherche::*;
    [Eleves, Personnes, Personnels, Classes, Groupes, Salles]
        .into_iter()
        .filter_map(|s| {
            let v: Vec<ResultatRecherche> = resultats
                .iter()
                .filter(|r| r.source == s)
                .cloned()
                .collect();
            (!v.is_empty()).then_some((s, v))
        })
        .collect()
}

/// Annonce `aria-live` du nombre de résultats.
pub fn annonce(n: usize) -> String {
    match n {
        0 => "Aucun résultat. Affinez votre recherche.".to_string(),
        1 => "1 résultat.".to_string(),
        n => format!("{n} résultats."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn r(source: SourceRecherche, libelle: &str) -> ResultatRecherche {
        ResultatRecherche {
            source,
            id: Uuid::nil(),
            libelle: libelle.into(),
            lien: None,
        }
    }

    #[test]
    fn deux_caracteres_au_moins_et_borne() {
        assert_eq!(
            texte_valide(" a "),
            Err("Saisissez au moins deux caractères.")
        );
        assert_eq!(texte_valide(" ab "), Ok("ab".to_string()));
        assert_eq!(texte_valide(&"x".repeat(300)).unwrap().chars().count(), 100);
    }

    #[test]
    fn les_resultats_sont_groupes_dans_l_ordre_fixe_des_sources() {
        let g = grouper(vec![
            r(SourceRecherche::Classes, "4B"),
            r(SourceRecherche::Eleves, "POTTER Harry — 4B"),
            r(SourceRecherche::Eleves, "POTTER Lily — 5A"),
        ]);
        let sources: Vec<_> = g.iter().map(|(s, v)| (*s, v.len())).collect();
        assert_eq!(
            sources,
            [(SourceRecherche::Eleves, 2), (SourceRecherche::Classes, 1)]
        );
        assert!(grouper(vec![]).is_empty());
    }

    #[test]
    fn l_annonce_dit_le_nombre_de_resultats() {
        assert_eq!(annonce(0), "Aucun résultat. Affinez votre recherche.");
        assert_eq!(annonce(1), "1 résultat.");
        assert_eq!(annonce(12), "12 résultats.");
    }
}
