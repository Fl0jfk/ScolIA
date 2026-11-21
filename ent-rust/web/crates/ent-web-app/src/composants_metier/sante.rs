//! Libellés et validations de saisie partagés par les écrans de santé et d'accompagnement
//! (socle 2, lot S2-11). Logique pure : ni Leptos ni réseau. Aucune règle de droit ici :
//! le serveur décide ; l'écran traduit des codes en français et contrôle la forme des saisies.
//!
//! Minimisation : les libellés des contraintes et des aménagements viennent du catalogue du
//! serveur (`GET /sante/catalogue`) ; ce module ne connaît que les codes d'états.

use chrono::{NaiveDate, NaiveTime};
use ent_web_ui::TonEtat;

pub const MOTIFS: [(&str, &str); 5] = [
    ("soins", "Soins"),
    ("malaise", "Malaise"),
    ("accident", "Accident"),
    ("pai_suivi", "Suivi PAI"),
    ("autre", "Autre"),
];

/// Issues proposables à la création (le renvoi au domicile se consigne à la clôture).
pub const ISSUES_CREATION: [(&str, &str); 4] = [
    ("non_renseignee", "À préciser"),
    ("retour_classe", "Retour en classe"),
    ("repos_infirmerie", "Repos à l'infirmerie"),
    ("famille_appelee", "Famille appelée"),
];

pub const ISSUES_CLOTURE: [(&str, &str); 5] = [
    ("retour_classe", "Retour en classe"),
    ("repos_infirmerie", "Repos à l'infirmerie"),
    ("famille_appelee", "Famille appelée"),
    ("renvoi_domicile", "Renvoi au domicile"),
    ("secours_appeles", "Secours appelés"),
];

pub fn libelle_motif(code: &str) -> &'static str {
    MOTIFS
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| *l)
        .unwrap_or("Autre")
}

pub fn libelle_issue(code: &str) -> &'static str {
    match code {
        "retour_classe" => "Retour en classe",
        "repos_infirmerie" => "Repos à l'infirmerie",
        "famille_appelee" => "Famille appelée",
        "renvoi_domicile" => "Renvoi au domicile",
        "secours_appeles" => "Secours appelés",
        _ => "À préciser",
    }
}

pub fn libelle_info_famille(code: &str) -> &'static str {
    match code {
        "informee" => "Famille informée",
        "a_informer" => "À informer",
        _ => "Famille non informée",
    }
}

pub fn ton_info_famille(code: &str) -> TonEtat {
    match code {
        "informee" => TonEtat::Ok,
        "a_informer" => TonEtat::Avertissement,
        _ => TonEtat::Neutre,
    }
}

pub fn libelle_partie(code: &str) -> &'static str {
    match code {
        "famille" => "Famille",
        "medecin" => "Médecin",
        "chef_etablissement" => "Chef d'établissement",
        "infirmier" => "Infirmier(ère)",
        "collectivite" => "Collectivité (restauration)",
        _ => "Autre partie",
    }
}

pub fn libelle_kind_dispositif(code: &str) -> &'static str {
    match code {
        "pap" => "PAP",
        "pps" => "PPS",
        "ppre" => "PPRE",
        "ulis" => "ULIS",
        "gevasco" => "GEVA-Sco",
        _ => "Autre dispositif",
    }
}

pub fn libelle_portee_amenagement(code: &str) -> &'static str {
    match code {
        "cours" => "En cours",
        "examen" => "Aux examens",
        _ => "En cours et aux examens",
    }
}

pub fn libelle_audience(code: &str) -> &'static str {
    match code {
        "restauration" => "Restauration",
        "enseignants" => "Enseignants",
        "sorties" => "Sorties et séjours",
        _ => "Autre",
    }
}

pub fn libelle_reunion(code: &str) -> &'static str {
    match code {
        "equipe_educative" => "Équipe éducative",
        _ => "Équipe de suivi",
    }
}

/// « échue depuis 3 j », « aujourd'hui », « dans 10 j ».
pub fn delai_texte(jours: i32) -> String {
    match jours {
        0 => "aujourd'hui".into(),
        j if j < 0 => format!("échue depuis {} j", -j),
        j => format!("dans {j} j"),
    }
}

pub fn ton_delai(jours: i32) -> TonEtat {
    if jours < 0 {
        TonEtat::Mal
    } else if jours <= 30 {
        TonEtat::Avertissement
    } else {
        TonEtat::Neutre
    }
}

/// Date facultative saisie dans un champ `date` (vide : aucune).
pub fn date_facultative(texte: &str, nom: &str) -> Result<Option<NaiveDate>, String> {
    let t = texte.trim();
    if t.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(t, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| format!("{nom} : date invalide."))
}

pub fn date_obligatoire(texte: &str, nom: &str) -> Result<NaiveDate, String> {
    date_facultative(texte, nom)?.ok_or_else(|| format!("{nom} : date obligatoire."))
}

/// Heure facultative d'un champ `time` (« 08:30 » ou « 08:30:00 »).
pub fn heure_facultative(texte: &str, nom: &str) -> Result<Option<NaiveTime>, String> {
    let t = texte.trim();
    if t.is_empty() {
        return Ok(None);
    }
    NaiveTime::parse_from_str(t, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(t, "%H:%M:%S"))
        .map(Some)
        .map_err(|_| format!("{nom} : heure invalide."))
}

/// Texte facultatif : `None` si vide après rognage.
pub fn texte_facultatif(texte: &str) -> Option<String> {
    let t = texte.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Heure « HH:MM » d'un champ renvoyé par l'API (`08:30:00`).
pub fn heure_affichee(h: Option<NaiveTime>) -> String {
    h.map(|h| h.format("%H:%M").to_string()).unwrap_or_default()
}

/// Nom affiché d'un élève : « Prénom NOM ».
pub fn nom_eleve(prenom: &str, nom: &str) -> String {
    format!("{prenom} {nom}").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libelles_et_tons() {
        assert_eq!(libelle_motif("pai_suivi"), "Suivi PAI");
        assert_eq!(libelle_motif("inconnu"), "Autre");
        assert_eq!(libelle_issue("renvoi_domicile"), "Renvoi au domicile");
        assert_eq!(libelle_info_famille("informee"), "Famille informée");
        assert_eq!(ton_info_famille("a_informer"), TonEtat::Avertissement);
        assert_eq!(libelle_partie("chef_etablissement"), "Chef d'établissement");
        assert_eq!(libelle_kind_dispositif("pps"), "PPS");
        assert_eq!(libelle_portee_amenagement("examen"), "Aux examens");
        assert_eq!(libelle_audience("sorties"), "Sorties et séjours");
        assert_eq!(libelle_reunion("equipe_educative"), "Équipe éducative");
    }

    #[test]
    fn delais() {
        assert_eq!(delai_texte(-3), "échue depuis 3 j");
        assert_eq!(delai_texte(0), "aujourd'hui");
        assert_eq!(delai_texte(10), "dans 10 j");
        assert_eq!(ton_delai(-1), TonEtat::Mal);
        assert_eq!(ton_delai(30), TonEtat::Avertissement);
        assert_eq!(ton_delai(31), TonEtat::Neutre);
    }

    #[test]
    fn saisies() {
        assert_eq!(date_facultative("", "Fin"), Ok(None));
        assert_eq!(
            date_facultative("2026-11-05", "Fin"),
            Ok(NaiveDate::from_ymd_opt(2026, 11, 5))
        );
        assert!(date_facultative("05/11/2026", "Fin").is_err());
        assert!(date_obligatoire("", "Début").is_err());
        assert_eq!(
            heure_facultative("08:30", "Heure"),
            Ok(NaiveTime::from_hms_opt(8, 30, 0))
        );
        assert!(heure_facultative("25:00", "Heure").is_err());
        assert_eq!(texte_facultatif("  "), None);
        assert_eq!(texte_facultatif(" a "), Some("a".into()));
        assert_eq!(nom_eleve("Harry", "Potter"), "Harry Potter");
    }

    /// Les motifs et issues proposés correspondent aux codes acceptés par le serveur.
    #[test]
    fn codes_proposes_acceptes_par_le_serveur() {
        let serveur = [
            "non_renseignee",
            "retour_classe",
            "repos_infirmerie",
            "famille_appelee",
            "renvoi_domicile",
            "secours_appeles",
        ];
        for (c, _) in ISSUES_CREATION.iter().chain(ISSUES_CLOTURE.iter()) {
            assert!(serveur.contains(c), "{c}");
        }
        assert!(!ISSUES_CREATION.iter().any(|(c, _)| *c == "renvoi_domicile"));
    }
}
