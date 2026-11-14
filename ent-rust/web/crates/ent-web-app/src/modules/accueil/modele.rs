//! Logique de l'accueil (pure, testée en natif) : correspondances entre les vues du serveur
//! (`ent-types::portail`) et l'affichage. Aucune règle de droit ici : le serveur ne renvoie que
//! les tuiles permises.

use ent_web_api::types_portail::{Bandeau, GraviteBandeau, ModePortail, ProfilPortail, TonLigne};
use ent_web_ui::{Largeur, Niveau};

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

/// Début du titre d'un bandeau : la gravité est dite en toutes lettres, jamais portée par la seule couleur.
pub fn prefixe_gravite(g: GraviteBandeau) -> &'static str {
    match g {
        GraviteBandeau::Info => "Information",
        GraviteBandeau::Attention => "Attention",
        GraviteBandeau::Securite => "Sécurité",
    }
}

/// Niveau visuel d'un bandeau (la sécurité est annoncée comme une alerte).
pub fn niveau_bandeau(g: GraviteBandeau) -> Niveau {
    match g {
        GraviteBandeau::Info => Niveau::Info,
        GraviteBandeau::Attention => Niveau::Avertissement,
        GraviteBandeau::Securite => Niveau::Erreur,
    }
}

/// Bandeaux visibles une fois ceux que la personne a masqués retirés. Un bandeau non masquable (la sécurité,
/// R-PO15) reste toujours visible, même si son code figure parmi les masqués. Le masquage vit dans la page
/// (aucun stockage) : au prochain chargement, tout revient.
pub fn sans_masques(bandeaux: &[Bandeau], masques: &[String]) -> Vec<Bandeau> {
    bandeaux
        .iter()
        .filter(|b| !b.masquable || !masques.contains(&b.code))
        .cloned()
        .collect()
}

/// Phrase sous le titre selon le mode du moment (le serveur le calcule ; l'écran le dit).
pub fn sous_titre(mode: ModePortail) -> &'static str {
    match mode {
        ModePortail::JourneeModifiee => "Votre journée est modifiée aujourd'hui.",
        ModePortail::Vacances => "C'est les vacances.",
        ModePortail::Rentree => "C'est la rentrée.",
        ModePortail::Examens => "Période d'examens.",
        _ => "Voici votre journée.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(code: &str, g: GraviteBandeau, masquable: bool) -> Bandeau {
        Bandeau::nouveau(code, g, "texte", masquable, None)
    }

    #[test]
    fn la_securite_ne_se_masque_pas_et_le_reste_oui() {
        let tous = vec![
            b("alerte.ppms", GraviteBandeau::Securite, false),
            b("structure.fermeture", GraviteBandeau::Attention, true),
        ];
        let masques = vec!["alerte.ppms".to_string(), "structure.fermeture".to_string()];
        let v = sans_masques(&tous, &masques);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].code, "alerte.ppms");
        assert_eq!(sans_masques(&tous, &[]).len(), 2);
    }

    #[test]
    fn la_gravite_est_dite_en_toutes_lettres() {
        assert_eq!(prefixe_gravite(GraviteBandeau::Securite), "Sécurité");
        assert_eq!(niveau_bandeau(GraviteBandeau::Securite), Niveau::Erreur);
        assert_eq!(niveau_bandeau(GraviteBandeau::Info), Niveau::Info);
        assert_eq!(sous_titre(ModePortail::Ordinaire), "Voici votre journée.");
        assert_eq!(sous_titre(ModePortail::Vacances), "C'est les vacances.");
    }

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
