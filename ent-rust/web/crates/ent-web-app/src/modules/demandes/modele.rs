//! Logique d'écran **pure** des demandes (sans Leptos ni réseau), testée en natif. Aucune règle métier : droits,
//! décisions et effets restent dans `ent-api` / `ent-db` ; ici, des libellés en mots simples, des filtres et des
//! conversions de saisie (heures → quarts d'heure).

use ent_types::demandes::NatureGuichet;

/// Motifs de HSE proposés (liste fermée du serveur, Q-DM5).
pub const MOTIFS_HSE: &[(&str, &str)] = &[
    ("soutien", "Soutien"),
    ("atelier", "Atelier"),
    ("accompagnement_sortie", "Accompagnement d'une sortie"),
    ("stage_reussite", "Stage de réussite"),
    ("preparation_concours", "Préparation d'un concours"),
    ("tutorat", "Tutorat"),
    ("autre", "Autre"),
];

/// Motifs d'une décision partielle ou d'un refus (codés : jamais de texte libre dans l'historique).
pub const MOTIFS_DECISION: &[(&str, &str)] = &[
    ("budget", "Enveloppe insuffisante"),
    ("pas_besoin", "Pas de besoin identifié"),
    ("hors_perimetre", "Hors du périmètre des HSE"),
    ("volume_excessif", "Volume excessif"),
    ("autre", "Autre"),
];

/// L'état d'une demande, en mots simples (jamais le vocabulaire interne).
pub fn libelle_statut(statut: &str) -> &'static str {
    match statut {
        "deposee" => "Envoyée",
        "prise" => "Prise en charge",
        "en_attente_demandeur" => "On attend votre réponse",
        "en_attente_tiers" => "En attente d'un tiers",
        "en_validation" => "En validation",
        "traitee" => "Traitée",
        "close" => "Terminée",
        "rouverte" => "Rouverte",
        "annulee" => "Annulée",
        "brouillon" => "Brouillon",
        _ => "En cours",
    }
}

/// Le demandeur annule tant que personne n'a pris la demande (le serveur reste juge).
pub fn peut_annuler(statut: &str) -> bool {
    statut == "deposee"
}

pub fn libelle_etat_copies(etat: &str) -> &'static str {
    match etat {
        "a_valider" => "À valider par la direction",
        "a_imprimer" => "À imprimer",
        "en_impression" => "En impression",
        "prete" => "Prête",
        "retiree" => "Retirée",
        "refusee" => "Refusée",
        "annulee" => "Annulée",
        _ => "En cours",
    }
}

pub fn libelle_decision(decision: Option<&str>) -> &'static str {
    match decision {
        Some("accordee") => "Accordée",
        Some("accordee_partiellement") => "Accordée en partie",
        Some("refusee") => "Refusée",
        _ => "En attente de décision",
    }
}

pub fn libelle_motif_hse(code: &str) -> &'static str {
    MOTIFS_HSE.iter().find(|(c, _)| *c == code).map_or("Autre", |(_, l)| l)
}

/// Retire accents et casse : « Réparation » trouve « reparation ».
pub fn normaliser(s: &str) -> String {
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

/// Porte unique : filtre les démarches par ce que la personne écrit (libellé et mots-clés). Sans saisie, toutes.
pub fn filtrer_natures<'a>(natures: &'a [NatureGuichet], requete: &str) -> Vec<&'a NatureGuichet> {
    let mots: Vec<String> = normaliser(requete).split_whitespace().map(str::to_string).collect();
    if mots.is_empty() {
        return natures.iter().collect();
    }
    natures
        .iter()
        .filter(|n| {
            let tas = normaliser(&format!("{} {}", n.libelle, n.mots_cles.join(" ")));
            mots.iter().all(|m| tas.contains(m.as_str()))
        })
        .collect()
}

/// « 6 » heures et « 15 » minutes → 25 quarts d'heure. Les minutes sont un multiple de 15 (0, 15, 30, 45).
pub fn heures_vers_quarts(heures: &str, minutes: &str) -> Result<i32, &'static str> {
    let h: i32 = if heures.trim().is_empty() { 0 } else { heures.trim().parse().map_err(|_| "Les heures doivent être un nombre entier.")? };
    let m: i32 = if minutes.trim().is_empty() { 0 } else { minutes.trim().parse().map_err(|_| "Les minutes doivent être un nombre entier.")? };
    if !(0..=400).contains(&h) || !matches!(m, 0 | 15 | 30 | 45) {
        return Err("Indiquez des heures (0 à 400) et des minutes parmi 0, 15, 30 ou 45.");
    }
    let q = h * 4 + m / 15;
    if q < 1 {
        return Err("Le volume doit être d'au moins un quart d'heure.");
    }
    Ok(q)
}

/// 25 quarts → « 6 h 15 » (jamais un montant : les HSE se comptent en heures).
pub fn quarts_vers_texte(quarts: i64) -> String {
    let (h, m) = (quarts / 4, (quarts % 4) * 15);
    if m == 0 {
        format!("{h} h")
    } else {
        format!("{h} h {m:02}")
    }
}

/// Contrôle local d'une saisie d'HSE avant l'envoi (le serveur recontrôle tout).
pub fn controler_hse(objet: &str, quarts: Result<i32, &'static str>, du: &str, au: &str) -> Result<i32, &'static str> {
    if objet.trim().is_empty() {
        return Err("L'objet de la demande est obligatoire.");
    }
    let q = quarts?;
    if du.is_empty() || au.is_empty() {
        return Err("Indiquez la période.");
    }
    if au < du {
        return Err("La fin de la période ne peut pas précéder le début.");
    }
    Ok(q)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nature(code: &str, libelle: &str, mots: &[&str]) -> NatureGuichet {
        NatureGuichet {
            code: code.into(),
            libelle: libelle.into(),
            mots_cles: mots.iter().map(|m| (*m).into()).collect(),
            eleve_requis: false,
            role_traitant: None,
            message_accueil: None,
            delai_traitement_jours: 5,
            formulaire: None,
        }
    }

    #[test]
    fn statuts_en_mots_simples() {
        assert_eq!(libelle_statut("deposee"), "Envoyée");
        assert_eq!(libelle_statut("en_attente_demandeur"), "On attend votre réponse");
        assert_eq!(libelle_statut("inconnu"), "En cours");
        assert!(peut_annuler("deposee") && !peut_annuler("prise") && !peut_annuler("close"));
    }

    #[test]
    fn porte_unique_trouve_sans_accent_ni_casse() {
        let n = vec![
            nature("a", "Signaler un problème technique", &["panne", "réparation"]),
            nature("b", "Demander une attestation", &["certificat", "assurance"]),
        ];
        assert_eq!(filtrer_natures(&n, "").len(), 2, "sans saisie : tout est proposé");
        assert_eq!(filtrer_natures(&n, "REPARATION")[0].code, "a");
        assert_eq!(filtrer_natures(&n, "attestation assurance")[0].code, "b");
        assert_eq!(filtrer_natures(&n, "problème panne").len(), 1);
        assert!(filtrer_natures(&n, "zzz").is_empty());
    }

    #[test]
    fn heures_et_quarts_sans_euros() {
        assert_eq!(heures_vers_quarts("6", "15"), Ok(25));
        assert_eq!(heures_vers_quarts("", "30"), Ok(2));
        assert!(heures_vers_quarts("0", "0").is_err(), "au moins un quart d'heure");
        assert!(heures_vers_quarts("6", "10").is_err(), "minutes par quarts");
        assert!(heures_vers_quarts("abc", "0").is_err());
        assert!(heures_vers_quarts("401", "0").is_err());
        assert_eq!(quarts_vers_texte(25), "6 h 15");
        assert_eq!(quarts_vers_texte(24), "6 h");
        assert_eq!(quarts_vers_texte(1), "0 h 15");
    }

    #[test]
    fn controle_local_de_la_saisie_hse() {
        assert!(controler_hse("", Ok(4), "2026-01-05", "2026-01-06").is_err());
        assert!(controler_hse("Soutien", Err("x"), "2026-01-05", "2026-01-06").is_err());
        assert!(controler_hse("Soutien", Ok(4), "", "2026-01-06").is_err());
        assert!(controler_hse("Soutien", Ok(4), "2026-01-07", "2026-01-06").is_err());
        assert_eq!(controler_hse("Soutien", Ok(4), "2026-01-05", "2026-01-06"), Ok(4));
    }

    #[test]
    fn libelles_de_decision_et_de_copies() {
        assert_eq!(libelle_decision(None), "En attente de décision");
        assert_eq!(libelle_decision(Some("accordee_partiellement")), "Accordée en partie");
        assert_eq!(libelle_etat_copies("prete"), "Prête");
        assert_eq!(libelle_motif_hse("tutorat"), "Tutorat");
    }
}
