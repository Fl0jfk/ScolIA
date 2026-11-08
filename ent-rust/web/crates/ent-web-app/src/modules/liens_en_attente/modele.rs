//! Logique d'écran pure de « Mes liens à confirmer » (sans Leptos ni réseau), testée en natif.
//! Le serveur décide de tout ; ces fonctions préparent le libellé, l'ordre et le message de retour.

use ent_web_api::MonLienEnAttenteDto;

/// Phrase affichée pour une demande : « Vous êtes déclaré(e) responsable de Léa M. (mère) ».
pub fn libelle(l: &MonLienEnAttenteDto) -> String {
    let prenom = l.eleve_prenom.trim();
    let initiale = l.eleve_initiale.trim().trim_end_matches('.').trim();
    let mut eleve = prenom.to_string();
    if !initiale.is_empty() {
        if !eleve.is_empty() {
            eleve.push(' ');
        }
        eleve.push_str(&initiale.to_uppercase());
        eleve.push('.');
    }
    if eleve.is_empty() {
        eleve = "un élève".into();
    }
    let nature = l.lien.trim();
    if nature.is_empty() {
        format!("Vous êtes déclaré(e) responsable de {eleve}")
    } else {
        format!("Vous êtes déclaré(e) responsable de {eleve} ({nature})")
    }
}

/// Libellé accessible d'un bouton, qui rappelle l'élève concerné.
pub fn libelle_bouton(action: &str, l: &MonLienEnAttenteDto) -> String {
    format!("{action} : {}", libelle(l))
}

/// Demandes les plus anciennes d'abord, puis ordre alphabétique du prénom (stable).
pub fn trier(mut v: Vec<MonLienEnAttenteDto>) -> Vec<MonLienEnAttenteDto> {
    v.sort_by(|a, b| {
        a.demande_le
            .cmp(&b.demande_le)
            .then_with(|| a.eleve_prenom.cmp(&b.eleve_prenom))
            .then_with(|| a.id.cmp(&b.id))
    });
    v
}

/// Message de retour annoncé selon l'issue de la réponse.
pub fn message_issue(accepter: bool) -> &'static str {
    if accepter {
        "Lien confirmé : vous avez maintenant accès aux informations de cet élève."
    } else {
        "Demande refusée : aucun accès n'a été ouvert. L'établissement en est informé."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    fn dto(prenom: &str, initiale: &str, lien: &str, t: i64) -> MonLienEnAttenteDto {
        MonLienEnAttenteDto {
            id: Uuid::from_u128(t as u128),
            eleve_prenom: prenom.into(),
            eleve_initiale: initiale.into(),
            lien: lien.into(),
            demande_le: DateTime::<Utc>::from_timestamp(t, 0).unwrap(),
        }
    }

    #[test]
    fn libelle_complet_et_degrade() {
        assert_eq!(
            libelle(&dto("Léa", "M", "mère", 1)),
            "Vous êtes déclaré(e) responsable de Léa M. (mère)"
        );
        // Initiale déjà pointée ou en minuscule : un seul point, en majuscule.
        assert_eq!(
            libelle(&dto("Léa", "m.", "mère", 1)),
            "Vous êtes déclaré(e) responsable de Léa M. (mère)"
        );
        assert_eq!(
            libelle(&dto("Léa", "", "", 1)),
            "Vous êtes déclaré(e) responsable de Léa"
        );
        assert_eq!(
            libelle(&dto("", "", "tuteur", 1)),
            "Vous êtes déclaré(e) responsable de un élève (tuteur)"
        );
    }

    #[test]
    fn bouton_accessible_nomme_l_eleve() {
        let l = dto("Léa", "M", "mère", 1);
        assert!(libelle_bouton("Confirmer", &l).starts_with("Confirmer : "));
        assert!(libelle_bouton("Refuser", &l).contains("Léa M."));
    }

    #[test]
    fn tri_par_anciennete_puis_prenom() {
        let v = trier(vec![
            dto("Zoé", "A", "mère", 20),
            dto("Léa", "B", "mère", 10),
            dto("Adam", "C", "père", 20),
        ]);
        let p: Vec<&str> = v.iter().map(|l| l.eleve_prenom.as_str()).collect();
        assert_eq!(p, ["Léa", "Adam", "Zoé"]);
    }

    #[test]
    fn messages_selon_l_issue() {
        assert!(message_issue(true).starts_with("Lien confirmé"));
        assert!(message_issue(false).starts_with("Demande refusée"));
        assert_ne!(message_issue(true), message_issue(false));
    }
}
