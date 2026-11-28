//! Logique d'écran **pure** de l'extrait d'accès (socle 27, lot G3b-2) : libellés, compteurs, lecture des
//! éléments marqués. Aucune règle métier ici (`FRONTEND_LEPTOS.md` §2) : le serveur décide, l'écran rend lisible.

use ent_web_api::rgpd_extrait_types::{ContenuExtraitDto, ExtraitDto, MarquageDto};

/// Libellé d'un statut de section.
pub fn libelle_statut_section(statut: &str) -> &'static str {
    match statut {
        "ok" => "Assemblée",
        "vide" => "Aucune donnée",
        "echec" => "En échec",
        "non_couverte" => "Non couverte",
        "exclue" => "Écartée",
        _ => "Statut inconnu",
    }
}

/// Libellé d'une raison de marquage. Le code `a_arbitrer` est celui que voit un lecteur sans `droits.arbitrer`.
pub fn libelle_raison(raison: &str) -> &'static str {
    match raison {
        "autre_parent" => "Donnée de l'autre parent",
        "adresse_protegee" => "Information protégée",
        "parent_donnees_propres" => "Donnée propre d'un parent",
        "autre_eleve" => "Autre élève",
        "redacteur" => "Rédacteur d'une note interne",
        "signalement" => "Procédure en cours",
        "piece_tiers" => "Pièce concernant aussi un tiers",
        "manuel" => "Ajouté à la main",
        "a_arbitrer" => "À arbitrer par la direction",
        _ => "Raison inconnue",
    }
}

/// Libellé d'une décision de marquage.
pub fn libelle_decision_marquage(decision: &str) -> &'static str {
    match decision {
        "a_decider" => "À décider",
        "masque" => "Masqué",
        "leve" => "Levé",
        _ => "Décision inconnue",
    }
}

/// État d'un extrait, en une phrase.
pub fn phrase_etat(e: &ExtraitDto) -> String {
    if e.perime {
        return "Périmé : les données ont changé, assemblez une nouvelle version.".into();
    }
    if e.masquages_a_decider > 0 {
        return format!(
            "{} marquage(s) à décider avant toute remise.",
            e.masquages_a_decider
        );
    }
    if !e.complet && !e.remissible_en_l_etat {
        return "Incomplet : la direction doit décider, section par section, de remettre malgré tout.".into();
    }
    if e.remissible_en_l_etat {
        return "Prêt à être remis.".into();
    }
    "À relire.".into()
}

/// Marquages encore à décider.
pub fn restants(m: &[MarquageDto]) -> usize {
    m.iter().filter(|x| x.decision == "a_decider").count()
}

/// La valeur (visible du lecteur) de l'élément marqué, pour la montrer surlignée dans la relecture.
pub fn valeur_marquee(c: &ContenuExtraitDto, m: &MarquageDto) -> String {
    let Some(t) = c
        .sections
        .iter()
        .filter(|s| s.fournisseur == m.fournisseur)
        .flat_map(|s| s.tables.iter())
        .find(|t| t.nom == m.table)
    else {
        return String::new();
    };
    let ligne = match (m.ligne_id, m.rang) {
        (Some(id), _) => t.lignes.iter().find(|l| l.id == Some(id)),
        (None, Some(r)) => t.lignes.get(r as usize),
        // Toutes les lignes de la table (masquage manuel d'une table ou d'une colonne).
        (None, None) => {
            return match &m.colonne {
                Some(_) => "(colonne entière)".to_string(),
                None => "(table entière)".to_string(),
            }
        }
    };
    let Some(l) = ligne else {
        return String::new();
    };
    match &m.colonne {
        Some(col) => l
            .valeurs
            .iter()
            .find(|(c, _)| c == col)
            .map(|(_, v)| v.clone())
            .unwrap_or_default(),
        None => "(ligne entière)".to_string(),
    }
}

/// Où se trouve l'élément marqué.
pub fn cible(m: &MarquageDto) -> String {
    match (&m.colonne, m.rang) {
        (Some(c), _) => format!("{} · {}", m.table, c),
        (None, Some(r)) => format!("{} · ligne {}", m.table, r + 1),
        (None, None) => format!("{} · toutes les lignes", m.table),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_web_api::rgpd_extrait_types::{LigneExtraitDto, SectionContenuDto, TableExtraitDto};
    use uuid::Uuid;

    fn marquage(colonne: Option<&str>) -> MarquageDto {
        MarquageDto {
            id: Uuid::nil(),
            fournisseur: "administratif".into(),
            table: "ent_eleve".into(),
            ligne_id: None,
            rang: Some(0),
            colonne: colonne.map(str::to_string),
            raison: "autre_parent".into(),
            propose_par: "sys".into(),
            decision: "a_decider".into(),
            motif: None,
            decide_le: None,
            levee_reservee_direction: true,
        }
    }

    fn contenu() -> ContenuExtraitDto {
        ContenuExtraitDto {
            sujet_type: "eleve".into(),
            sujet_id: Uuid::nil(),
            periode_du: None,
            periode_au: None,
            sections: vec![SectionContenuDto {
                fournisseur: "administratif".into(),
                socle: "4".into(),
                statut: "ok".into(),
                motif_code: None,
                tables: vec![TableExtraitDto {
                    nom: "ent_eleve".into(),
                    lignes: vec![LigneExtraitDto {
                        id: None,
                        valeurs: vec![("ville".into(), "Lyon".into())],
                    }],
                }],
                exclusions: vec![],
                lignes: 1,
            }],
            complet: true,
            empreinte: String::new(),
            nb_lignes: 1,
            destinataires_internes: vec![],
            destinataires_externes: vec![],
            soustraitants: vec![],
            marquages: vec![],
            elements_reserves_direction: 0,
        }
    }

    #[test]
    fn la_valeur_marquee_se_retrouve_dans_le_contenu() {
        assert_eq!(valeur_marquee(&contenu(), &marquage(Some("ville"))), "Lyon");
        assert_eq!(
            valeur_marquee(&contenu(), &marquage(None)),
            "(ligne entière)"
        );
        assert_eq!(valeur_marquee(&contenu(), &marquage(Some("absente"))), "");
    }

    #[test]
    fn les_libelles_ne_revelent_pas_la_protection() {
        assert_eq!(libelle_raison("a_arbitrer"), "À arbitrer par la direction");
        assert!(!libelle_raison("a_arbitrer").contains("protég"));
        assert_eq!(restants(&[marquage(None)]), 1);
        assert_eq!(cible(&marquage(Some("ville"))), "ent_eleve · ville");
    }
}
