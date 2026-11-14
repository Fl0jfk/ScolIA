//! Logique d'écran pure du lot C7 (adaptations) : sans Leptos ni réseau, testée en natif.
//!
//! **Aucune règle métier ici** : qui voit une variante, qui est accompagné, la majoration d'un délai et l'absence de tout motif
//! restent au serveur. Ce module prépare les corps à envoyer et formate ce que le serveur annonce — jamais un diagnostic, jamais
//! un intitulé de plan : des faits utiles (pastilles).

use chrono::NaiveDate;
use ent_web_api::cahier_c7_types::{AffichageDto, ApplicationAmenagementsDto, VarianteBody};

use super::modele_c2::{lire_heure_champ, lire_jour};

/// Corps d'une variante d'après le formulaire ; échéance facultative (jour **et** heure, ou aucun des deux).
pub fn corps_variante(
    libelle: &str,
    consigne: &str,
    duree: &str,
    jour: &str,
    heure: &str,
) -> Result<VarianteBody, String> {
    let libelle = libelle.trim();
    if libelle.is_empty() || libelle.chars().count() > 80 {
        return Err("Donnez un nom interne à cette version (80 caractères au plus) : il ne sera jamais montré aux élèves.".into());
    }
    let consigne = consigne.trim();
    if consigne.is_empty() {
        return Err("Écrivez la consigne de cette version.".into());
    }
    let duree_estimee_min = match duree.trim() {
        "" => None,
        d => Some(
            d.parse::<i16>()
                .ok()
                .filter(|d| (1..=600).contains(d))
                .ok_or("La durée doit être comprise entre 1 et 600 minutes.")?,
        ),
    };
    let (echeance_jour, echeance_heure) = match (jour.trim(), heure.trim()) {
        ("", _) => (None, None),
        (j, h) => (
            Some(lire_jour(j).ok_or("Le jour d'échéance est illisible.")?),
            Some(lire_heure_champ(if h.is_empty() { "18:00" } else { h }).ok_or("L'heure d'échéance est illisible.")?),
        ),
    };
    Ok(VarianteBody {
        libelle_interne: libelle.into(),
        consigne: consigne.into(),
        duree_estimee_min,
        echeance_jour,
        echeance_heure,
    })
}

/// Pastilles d'un élève sur une ligne : des faits, séparés par « · ».
pub fn libelle_pastilles(p: &[String]) -> String {
    if p.is_empty() {
        "—".into()
    } else {
        p.join(" · ")
    }
}

/// Phrase de l'aperçu ou du résultat de « appliquer les aménagements » ; jamais la raison de la majoration.
pub fn phrase_amenagements(a: &ApplicationAmenagementsDto) -> String {
    match (a.applique, a.eleves.len()) {
        (false, 0) => "Aucune échéance n'est à majorer : personne n'a de délai majoré à appliquer à ce travail.".into(),
        (false, n) => format!("{n} élève(s) verraient leur échéance majorée. Rien n'est encore appliqué."),
        (true, n) => format!("Échéance majorée pour {n} élève(s) : ils voient « délai accordé jusqu'au… », sans motif."),
    }
}

/// Note d'accompagnement : texte simple, borné.
pub fn valider_note(texte: &str) -> Result<String, String> {
    let t = texte.trim();
    if t.is_empty() {
        return Err("Écrivez la note.".into());
    }
    if t.chars().count() > 2000 {
        return Err("La note dépasse 2 000 caractères.".into());
    }
    if t.contains('<') && t.contains('>') {
        return Err("Le balisage HTML n'est pas accepté : écrivez du texte simple.".into());
    }
    Ok(t.to_string())
}

/// Classes de l'élément racine de l'agenda selon les réglages d'affichage de l'élève (accessibilité, sans le professeur).
pub fn classes_affichage(a: &AffichageDto) -> String {
    let mut c = vec!["aff-eleve".to_string()];
    match a.taille_police.as_str() {
        "grande" => c.push("aff-police-grande".into()),
        "tres_grande" => c.push("aff-police-tres-grande".into()),
        _ => {}
    }
    if a.interligne == "large" {
        c.push("aff-interligne-large".into());
    }
    if a.animations_reduites {
        c.push("aff-calme".into());
    }
    c.join(" ")
}

/// Jour de l'élève : « jusqu'au 12/11/2026 ».
pub fn jour_court(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_web_api::cahier_c7_types::EcheanceMajoreeDto;

    #[test]
    fn variante_exige_nom_interne_et_consigne() {
        assert!(corps_variante("", "x", "", "", "").is_err());
        assert!(corps_variante("v", " ", "", "", "").is_err());
        let b = corps_variante("Version courte", "Exercice 1", "10", "2026-12-01", "").unwrap();
        assert_eq!(b.duree_estimee_min, Some(10));
        assert!(b.echeance_jour.is_some() && b.echeance_heure.is_some());
        assert!(corps_variante("v", "x", "900", "", "").is_err());
        assert!(corps_variante("v", "x", "", "bientôt", "").is_err());
        assert!(corps_variante("v", "x", "", "", "").unwrap().echeance_jour.is_none());
    }

    #[test]
    fn pastilles_et_amenagements_sans_motif() {
        assert_eq!(libelle_pastilles(&[]), "—");
        assert_eq!(libelle_pastilles(&["Police agrandie".into(), "Temps majoré (+33 %)".into()]), "Police agrandie · Temps majoré (+33 %)");
        let apercu = ApplicationAmenagementsDto { eleves: vec![], appliques: 0, applique: false };
        assert!(phrase_amenagements(&apercu).starts_with("Aucune échéance"));
        let fait = ApplicationAmenagementsDto {
            eleves: vec![EcheanceMajoreeDto { eleve_id: uuid::Uuid::nil(), nom: "A".into(), prenom: "B".into(), echeance_le: chrono::Utc::now(), echeance_jour: NaiveDate::from_ymd_opt(2026, 11, 20).unwrap(), echeance_heure: "18:00".into() }],
            appliques: 1,
            applique: true,
        };
        let p = phrase_amenagements(&fait);
        assert!(p.contains("sans motif") && !p.to_lowercase().contains("pap"), "{p}");
    }

    #[test]
    fn note_texte_simple() {
        assert!(valider_note("").is_err());
        assert!(valider_note("<b>x</b>").is_err());
        assert!(valider_note(&"a".repeat(2001)).is_err());
        assert_eq!(valider_note("  Consigne reformulée.  ").unwrap(), "Consigne reformulée.");
    }

    #[test]
    fn classes_d_affichage() {
        assert_eq!(classes_affichage(&AffichageDto::default()), "aff-eleve");
        let a = AffichageDto { taille_police: "tres_grande".into(), interligne: "large".into(), lecture_audio: false, animations_reduites: true };
        assert_eq!(classes_affichage(&a), "aff-eleve aff-police-tres-grande aff-interligne-large aff-calme");
    }
}
