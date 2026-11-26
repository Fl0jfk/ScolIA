//! Logique d'écran **pure** de la remise et de « Mes données » (socle 27, lot G3b-3) : libellés, échéances,
//! recevabilité de l'émargement. Aucune règle métier ici (`FRONTEND_LEPTOS.md` §2) : le serveur décide, l'écran
//! rend lisible et évite les envois évidemment vides.

use chrono::{DateTime, Utc};
use ent_web_api::rgpd_extrait_types::{ExtraitDto, FichierRemisDto};

/// Libellé d'un mode de remise.
pub fn libelle_mode(mode: &str) -> &'static str {
    match mode {
        "en_ligne" => "En ligne, dans « Mes données »",
        "main_propre" => "En main propre, au guichet",
        _ => "Mode inconnu",
    }
}

/// Libellé d'un renvoi tracé.
pub fn libelle_renvoi(nature: &str) -> &'static str {
    match nature {
        "rectification" => "Rectification : renvoi vers le socle propriétaire",
        "retrait_consentement" => "Retrait de consentement",
        "cada_tiers" => "Demande CADA d'un tiers",
        _ => "Renvoi",
    }
}

/// Un extrait peut-il être proposé à la remise ? Le serveur revérifie tout ; l'écran évite l'appel évident.
pub fn remettable(e: &ExtraitDto) -> bool {
    !e.perime && e.remissible_en_l_etat && e.masquages_a_decider == 0
}

/// Les deux champs de l'émargement sont-ils renseignés (deux caractères au moins) ?
pub fn emargement_recevable(nom: &str, piece_vue: &str) -> bool {
    nom.trim().chars().count() >= 2 && piece_vue.trim().chars().count() >= 2
}

/// Jours entiers restants avant la fin de la mise à disposition (0 : échue ou le jour même).
pub fn jours_restants(fin: DateTime<Utc>, maintenant: DateTime<Utc>) -> i64 {
    (fin - maintenant).num_days().max(0)
}

/// Taille lisible d'un fichier.
pub fn taille_lisible(octets: i32) -> String {
    if octets >= 1_048_576 {
        format!("{:.1} Mo", f64::from(octets) / 1_048_576.0)
    } else if octets >= 1024 {
        format!("{} Ko", octets / 1024)
    } else {
        format!("{octets} o")
    }
}

/// Nom lisible d'un fichier de l'extrait : le PDF, ou le CSV d'une section.
pub fn nom_lisible(f: &FichierRemisDto) -> String {
    if f.type_fichier == "pdf" {
        "Extrait lisible (PDF)".to_string()
    } else {
        let section = f
            .nom
            .strip_prefix("section-")
            .and_then(|n| n.strip_suffix(".csv"))
            .unwrap_or(&f.nom);
        format!("Données « {section} » (tableur CSV)")
    }
}

/// Phrase d'état d'un fichier : à télécharger jusqu'au…, ou plus disponible.
pub fn etat_fichier(f: &FichierRemisDto, maintenant: DateTime<Utc>) -> String {
    if f.purge || f.disponible_jusqu_le <= maintenant {
        return "Plus disponible : demandez un nouvel extrait.".into();
    }
    let j = jours_restants(f.disponible_jusqu_le, maintenant);
    if j == 0 {
        "Disponible jusqu'à aujourd'hui.".into()
    } else if j == 1 {
        "Disponible encore 1 jour.".into()
    } else {
        format!("Disponible encore {j} jours.")
    }
}

/// Le fichier peut-il encore être téléchargé ?
pub fn telechargeable(f: &FichierRemisDto, maintenant: DateTime<Utc>) -> bool {
    !f.purge && f.disponible_jusqu_le > maintenant
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use uuid::Uuid;

    fn fichier(type_: &str, nom: &str, fin: DateTime<Utc>, purge: bool) -> FichierRemisDto {
        FichierRemisDto {
            id: Uuid::nil(),
            nom: nom.into(),
            type_fichier: type_.into(),
            octets: 2048,
            empreinte: String::new(),
            disponible_jusqu_le: fin,
            purge,
            telechargements: 0,
        }
    }

    #[test]
    fn emargement_demande_deux_champs() {
        assert!(!emargement_recevable("", "carte d'identité"));
        assert!(!emargement_recevable("Mme Granger", " "));
        assert!(emargement_recevable("Mme Granger", "carte d'identité"));
    }

    #[test]
    fn echeance_et_telechargement() {
        let t = DateTime::parse_from_rfc3339("2027-01-15T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let f = fichier("pdf", "extrait.pdf", t + Duration::days(7), false);
        assert_eq!(etat_fichier(&f, t), "Disponible encore 7 jours.");
        assert!(telechargeable(&f, t));
        assert_eq!(
            etat_fichier(&f, t + Duration::days(8)),
            "Plus disponible : demandez un nouvel extrait."
        );
        assert!(!telechargeable(&f, t + Duration::days(8)));
        let p = fichier("pdf", "extrait.pdf", t + Duration::days(7), true);
        assert!(!telechargeable(&p, t));
    }

    #[test]
    fn noms_et_tailles_lisibles() {
        let t = DateTime::parse_from_rfc3339("2027-01-15T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            nom_lisible(&fichier("pdf", "extrait.pdf", t, false)),
            "Extrait lisible (PDF)"
        );
        assert_eq!(
            nom_lisible(&fichier("csv", "section-notes.csv", t, false)),
            "Données « notes » (tableur CSV)"
        );
        assert_eq!(taille_lisible(900), "900 o");
        assert_eq!(taille_lisible(2048), "2 Ko");
        assert_eq!(taille_lisible(3 * 1_048_576), "3.0 Mo");
    }

    #[test]
    fn libelles_des_modes_et_renvois() {
        assert!(libelle_mode("main_propre").contains("guichet"));
        assert!(libelle_renvoi("rectification").contains("socle"));
        assert_eq!(libelle_mode("x"), "Mode inconnu");
    }
}
