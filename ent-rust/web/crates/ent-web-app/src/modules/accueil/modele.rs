//! Logique de l'accueil (pure, testée en natif) : correspondances entre les vues du serveur
//! (`ent-types::portail`) et l'affichage. Aucune règle de droit ici : le serveur ne renvoie que
//! les tuiles permises.

use ent_web_api::types_portail::{ProfilPortail, TonLigne};
use ent_web_ui::Largeur;

/// Largeur Bento proposée par le serveur (4, 6, 8, 12) ; toute autre valeur donne 12 colonnes.
pub fn largeur(colonnes: u8) -> Largeur {
    match colonnes {
        4 => Largeur::Quatre,
        6 => Largeur::Six,
        8 => Largeur::Huit,
        _ => Largeur::Douze,
    }
}

/// Classe CSS d'une ligne selon son ton (le sens reste porté par le libellé, pas seulement la couleur).
pub fn classe_ton(ton: TonLigne) -> &'static str {
    match ton {
        TonLigne::Neutre => "ui-liste__element",
        TonLigne::Attention => "ui-liste__element accueil-ligne--attention",
        TonLigne::Alerte => "ui-liste__element accueil-ligne--alerte",
    }
}

/// « 3 absences à justifier » → « absences à justifier » : le chiffre est affiché à part, en grand.
pub fn sans_nombre(principal: &str) -> String {
    let t = principal.trim_start();
    let reste = t
        .trim_start_matches(|c: char| c.is_ascii_digit())
        .trim_start();
    if reste.len() < t.len() && !reste.is_empty() {
        reste.to_string()
    } else {
        t.to_string()
    }
}

/// Le professeur garde la tuile « Faire l'appel » de la séance en cours.
pub fn est_professeur(profil: ProfilPortail) -> bool {
    profil == ProfilPortail::Professeur
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn largeur_connue_ou_pleine() {
        assert!(matches!(largeur(4), Largeur::Quatre));
        assert!(matches!(largeur(8), Largeur::Huit));
        assert!(matches!(largeur(7), Largeur::Douze));
    }

    #[test]
    fn le_chiffre_est_retire_du_libelle() {
        assert_eq!(
            sans_nombre("3 absences à justifier"),
            "absences à justifier"
        );
        assert_eq!(sans_nombre("Aucun message"), "Aucun message");
        assert_eq!(sans_nombre("12"), "12");
    }

    #[test]
    fn seul_le_professeur_a_le_bloc_appel() {
        assert!(est_professeur(ProfilPortail::Professeur));
        assert!(!est_professeur(ProfilPortail::Famille));
    }
}
