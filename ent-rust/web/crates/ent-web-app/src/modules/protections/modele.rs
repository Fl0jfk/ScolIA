//! Logique d'écran pure de « Protections » (direction) : libellés, tri, messages. Le serveur
//! décide des droits et du contenu ; l'adresse protégée n'est jamais conservée par ce modèle.

use ent_web_api::{AdresseParqueeDto, ProtectionADeclarerDto};

/// Nom affiché d'une personne : « Prénom NOM » (ou « NOM » sans prénom).
pub fn nom_affiche(nom: &str, prenom: Option<&str>) -> String {
    match prenom.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => format!("{p} {}", nom.trim().to_uppercase()),
        None => nom.trim().to_uppercase(),
    }
}

/// Origine d'une adresse mise de côté, en clair.
pub fn libelle_origine(origine: &str) -> String {
    match origine {
        "inscription" => "Issue d'une inscription".into(),
        "saisie" => "Saisie par l'établissement".into(),
        "famille" => "Transmise par la famille".into(),
        "" => "Origine non précisée".into(),
        autre => format!("Origine : {autre}"),
    }
}

/// Adresses les plus anciennes d'abord (à traiter dans l'ordre d'arrivée).
pub fn trier_adresses(mut v: Vec<AdresseParqueeDto>) -> Vec<AdresseParqueeDto> {
    v.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
    v
}

/// Déclarations les plus anciennes d'abord ; sans date, en dernier.
pub fn trier_declarations(mut v: Vec<ProtectionADeclarerDto>) -> Vec<ProtectionADeclarerDto> {
    v.sort_by(|a, b| match (a.declaree_le, b.declaree_le) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    v
}

/// Message de retour d'un « ne pas solliciter ».
pub fn message_ne_pas_solliciter(actif: bool, nom: &str) -> String {
    if actif {
        format!("« Ne pas solliciter » confirmé pour {nom}.")
    } else {
        format!("« Ne pas solliciter » levé pour {nom}.")
    }
}

/// Message affiché quand le serveur refuse (403) : l'écran n'est pas ouvert à ce profil.
pub const MESSAGE_RESERVE_DIRECTION: &str = "Cette page est réservée à la direction.";

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    fn t(s: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(s, 0).unwrap()
    }

    #[test]
    fn noms_et_origines() {
        assert_eq!(nom_affiche("martin", Some(" Léa ")), "Léa MARTIN");
        assert_eq!(nom_affiche("martin", Some(" ")), "MARTIN");
        assert_eq!(nom_affiche("martin", None), "MARTIN");
        assert_eq!(libelle_origine("inscription"), "Issue d'une inscription");
        assert_eq!(libelle_origine(""), "Origine non précisée");
        assert_eq!(libelle_origine("x"), "Origine : x");
    }

    #[test]
    fn tris_anciens_d_abord() {
        let a = |n: u128, d: i64| AdresseParqueeDto {
            id: Uuid::from_u128(n),
            personne_id: Uuid::from_u128(n),
            nom: "N".into(),
            prenom: None,
            origine: "saisie".into(),
            created_at: t(d),
        };
        let v = trier_adresses(vec![a(1, 30), a(2, 10), a(3, 20)]);
        assert_eq!(
            v.iter().map(|x| x.id.as_u128()).collect::<Vec<_>>(),
            [2, 3, 1]
        );
        let d = |n: u128, d: Option<i64>| ProtectionADeclarerDto {
            personne_id: Uuid::from_u128(n),
            nom: "N".into(),
            prenom: None,
            declaree_le: d.map(t),
        };
        let v = trier_declarations(vec![d(1, None), d(2, Some(50)), d(3, Some(5))]);
        assert_eq!(
            v.iter().map(|x| x.personne_id.as_u128()).collect::<Vec<_>>(),
            [3, 2, 1]
        );
    }

    #[test]
    fn messages() {
        assert!(message_ne_pas_solliciter(true, "A").contains("confirmé"));
        assert!(message_ne_pas_solliciter(false, "A").contains("levé"));
        assert!(MESSAGE_RESERVE_DIRECTION.contains("direction"));
    }
}
