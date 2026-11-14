//! Portée de la session et décision de redirection vers l'enrôlement d'une passkey.
//!
//! Le serveur est l'autorité : une session « enrôlement » (passkey obligatoire non encore
//! enrôlée) est refusée en 403 `passkey_enrolement_requis` hors des routes d'enrôlement.
//! L'interface ne fait que **guider** la personne vers l'écran d'enrôlement, sans
//! lui laisser d'autre chemin ; contourner l'écran ne donne accès à aucune donnée.
//! Logique pure, testée en natif.

use ent_web_api::ApiError;

/// Adresse de l'écran d'enrôlement obligatoire.
pub const ENROLEMENT: &str = "/web/securite/enrolement";
/// Page d'arrivée après l'enrôlement.
pub const ACCUEIL: &str = "/web";

/// Portée de la session telle que `GET /auth/me` la renvoie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Portee {
    /// Session utilisable normalement.
    Complete,
    /// Passkey obligatoire à enrôler avant toute autre action.
    Enrolement,
}

impl Portee {
    /// `enrolement` → [`Portee::Enrolement`] ; toute autre valeur (y compris l'absence du
    /// champ sur un ancien serveur) → [`Portee::Complete`] : le serveur reste l'autorité.
    pub fn depuis(texte: &str) -> Portee {
        if texte.trim() == "enrolement" {
            Portee::Enrolement
        } else {
            Portee::Complete
        }
    }
}

/// Vrai pour l'écran d'enrôlement (avec ou sans « / » final ou paramètres).
pub fn est_enrolement(chemin: &str) -> bool {
    let chemin = chemin.split(['?', '#']).next().unwrap_or("");
    chemin.trim_end_matches('/') == ENROLEMENT
}

/// Où envoyer la personne selon la portée de sa session et la page où elle se trouve.
/// `None` : rester sur la page.
pub fn decision_redirection(portee: Portee, chemin: &str) -> Option<&'static str> {
    match (portee, est_enrolement(chemin)) {
        (Portee::Enrolement, false) => Some(ENROLEMENT),
        (Portee::Complete, true) => Some(ACCUEIL),
        _ => None,
    }
}

/// Redirection déclenchée par une erreur d'API (403 `passkey_enrolement_requis`), pour les
/// sessions dont la portée a changé depuis le chargement de l'identité.
pub fn redirection_apres_erreur(erreur: &ApiError, chemin: &str) -> Option<&'static str> {
    (erreur.est_enrolement_requis() && !est_enrolement(chemin)).then_some(ENROLEMENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portee_depuis_le_texte_du_serveur() {
        assert_eq!(Portee::depuis("enrolement"), Portee::Enrolement);
        assert_eq!(Portee::depuis(" enrolement "), Portee::Enrolement);
        assert_eq!(Portee::depuis("complete"), Portee::Complete);
        // Ancien serveur (champ absent → valeur par défaut) ou valeur inconnue.
        assert_eq!(Portee::depuis(""), Portee::Complete);
        assert_eq!(Portee::depuis("autre"), Portee::Complete);
    }

    #[test]
    fn enrolement_oblige_a_rester_sur_l_ecran() {
        for chemin in [
            "/web",
            "/web/appel",
            "/web/messages/abc",
            "/web/securite",
            "/web/direction/securite",
        ] {
            assert_eq!(
                decision_redirection(Portee::Enrolement, chemin),
                Some(ENROLEMENT),
                "{chemin}"
            );
        }
        assert_eq!(decision_redirection(Portee::Enrolement, ENROLEMENT), None);
        assert_eq!(
            decision_redirection(Portee::Enrolement, "/web/securite/enrolement/"),
            None
        );
        assert_eq!(
            decision_redirection(Portee::Enrolement, "/web/securite/enrolement?x=1"),
            None
        );
    }

    #[test]
    fn session_complete_n_a_rien_a_faire_sur_l_ecran_d_enrolement() {
        assert_eq!(
            decision_redirection(Portee::Complete, ENROLEMENT),
            Some(ACCUEIL)
        );
        assert_eq!(decision_redirection(Portee::Complete, "/web"), None);
        assert_eq!(
            decision_redirection(Portee::Complete, "/web/securite"),
            None
        );
        // Un chemin voisin n'est pas l'écran d'enrôlement.
        assert!(!est_enrolement("/web/securite/enrolementx"));
        assert!(!est_enrolement("/web/securite"));
    }

    #[test]
    fn erreur_403_enrolement_requis_renvoie_vers_l_ecran() {
        let e = ApiError::depuis_statut(403, "passkey_enrolement_requis".into());
        assert_eq!(redirection_apres_erreur(&e, "/web/appel"), Some(ENROLEMENT));
        assert_eq!(redirection_apres_erreur(&e, ENROLEMENT), None);
        let autre = ApiError::depuis_statut(403, "réservé à la direction".into());
        assert_eq!(redirection_apres_erreur(&autre, "/web/appel"), None);
        assert_eq!(
            redirection_apres_erreur(&ApiError::NonConnecte, "/web/appel"),
            None
        );
    }
}
