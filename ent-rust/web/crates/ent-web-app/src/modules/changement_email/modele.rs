//! Logique d'écran pure des liens de changement d'adresse e-mail (RF-01) : textes des deux
//! étapes et messages d'erreur. Le serveur décide de tout (jeton à usage unique, délais,
//! rétablissement) ; l'écran explique le parcours sans rien ajouter.

use ent_web_api::ApiError;

/// Longueur maximale d'un jeton accepté par le serveur.
pub const JETON_MAX: usize = 256;

/// Jeton lu dans l'adresse : sans espace, non vide, borné. `None` si inutilisable.
pub fn jeton_valide(brut: Option<&str>) -> Option<String> {
    let t = brut?.trim();
    (!t.is_empty() && t.len() <= JETON_MAX && !t.chars().any(char::is_whitespace))
        .then(|| t.to_string())
}

/// Étape atteinte par un lien reçu par e-mail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtapeEmail {
    /// Lien reçu à la NOUVELLE adresse.
    Confirmer,
    /// Lien reçu à l'ANCIENNE adresse.
    Annuler,
}

impl EtapeEmail {
    pub fn titre(self) -> &'static str {
        match self {
            EtapeEmail::Confirmer => "Confirmer ma nouvelle adresse e-mail",
            EtapeEmail::Annuler => "Annuler le changement d'adresse e-mail",
        }
    }

    pub fn libelle_bouton(self) -> &'static str {
        match self {
            EtapeEmail::Confirmer => "Confirmer cette adresse",
            EtapeEmail::Annuler => "Annuler le changement",
        }
    }

    /// Paragraphes affichés **avant** l'action : ouvrir le lien n'agit jamais (un aperçu de
    /// messagerie ou un antivirus ne doit pas consommer le jeton).
    pub fn explications(self) -> &'static [&'static str] {
        match self {
            EtapeEmail::Confirmer => &[
                "L'adresse e-mail de votre compte ScolIA va être remplacée par celle qui a reçu ce message.",
                "En confirmant, toutes vos connexions en cours sont fermées : vous vous reconnecterez ensuite avec cette nouvelle adresse.",
                "Vous n'avez rien demandé, ou vous ne reconnaissez pas ce changement ? Ne confirmez pas : fermez cette page, rien ne sera modifié.",
            ],
            EtapeEmail::Annuler => &[
                "Un changement d'adresse e-mail a été demandé sur votre compte. En l'annulant, votre adresse actuelle est conservée (ou rétablie si le changement avait déjà été confirmé).",
                "Par précaution, vos connexions en cours sont fermées, et la réinitialisation du mot de passe ainsi que la récupération de passkey sont suspendues pendant 72 heures.",
                "Si vous n'êtes pas à l'origine de la demande, prévenez votre établissement.",
            ],
        }
    }

    pub fn message_succes(self) -> &'static str {
        match self {
            EtapeEmail::Confirmer => "Nouvelle adresse confirmée. Connectez-vous avec cette adresse.",
            EtapeEmail::Annuler => {
                "Changement annulé. Votre adresse d'origine est conservée. Pensez à prévenir votre établissement si vous n'êtes pas à l'origine de la demande."
            }
        }
    }
}

/// Message d'une erreur sur l'un des deux liens.
pub fn message_erreur_lien(e: &ApiError) -> String {
    match e {
        ApiError::Invalide(_) | ApiError::Introuvable(_) => {
            "Ce lien est invalide, expiré ou déjà utilisé. Si le changement vous concerne encore, contactez votre établissement."
                .into()
        }
        ApiError::TropDeRequetes => {
            "Trop de tentatives. Patientez quelques minutes avant de réessayer.".into()
        }
        ApiError::Serveur(503, _) => {
            "Le service est momentanément indisponible. Le lien reste valable : réessayez dans quelques minutes."
                .into()
        }
        autre => autre.message_utilisateur(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_etape_explique_avant_d_agir() {
        for e in [EtapeEmail::Confirmer, EtapeEmail::Annuler] {
            assert!(!e.titre().is_empty() && !e.libelle_bouton().is_empty());
            assert!(!e.explications().is_empty() && !e.message_succes().is_empty());
        }
        let c = EtapeEmail::Confirmer.explications().join(" ");
        assert!(c.contains("remplacée") && c.contains("Ne confirmez pas"));
        let a = EtapeEmail::Annuler.explications().join(" ");
        assert!(a.contains("72 heures") && a.contains("rétablie"));
    }

    #[test]
    fn erreurs_des_liens() {
        let invalide = message_erreur_lien(&ApiError::Invalide("x".into()));
        assert!(invalide.contains("invalide, expiré ou déjà utilisé"));
        assert_eq!(
            invalide,
            message_erreur_lien(&ApiError::Introuvable("y".into()))
        );
        assert!(message_erreur_lien(&ApiError::TropDeRequetes).contains("Trop de tentatives"));
        assert!(message_erreur_lien(&ApiError::Serveur(503, "x".into())).contains("reste valable"));
    }

    #[test]
    fn jeton_de_l_adresse() {
        assert_eq!(jeton_valide(Some(" abc ")).as_deref(), Some("abc"));
        assert_eq!(jeton_valide(Some("a b")), None);
        assert_eq!(jeton_valide(None), None);
    }
}
