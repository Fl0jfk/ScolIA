//! Logique d'écran pure de la récupération de passkey en libre-service (D-018) :
//! textes des trois étapes, validation de confort, messages d'erreur.
//!
//! Le serveur décide de tout (jetons à usage unique, délai de sécurité, une demande
//! active par personne). L'écran n'ajoute aucune règle ; il explique le parcours.

use chrono::{DateTime, Local, NaiveDateTime, Utc};
use ent_web_api::ApiError;
use uuid::Uuid;

/// Longueur maximale d'un jeton accepté par le serveur.
pub const JETON_MAX: usize = 256;
/// Longueur maximale d'une adresse e-mail (RFC 5321).
pub const EMAIL_MAX: usize = 254;
/// Durée de validité du lien de confirmation (côté serveur : 60 minutes).
pub const VALIDITE_LIEN_MINUTES: u32 = 60;

/// Étape atteinte par un lien reçu par e-mail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtapeLien {
    Confirmer,
    Annuler,
    Finaliser,
}

impl EtapeLien {
    pub fn titre(self) -> &'static str {
        match self {
            EtapeLien::Confirmer => "Confirmer la récupération de ma passkey",
            EtapeLien::Annuler => "Annuler la récupération de ma passkey",
            EtapeLien::Finaliser => "Finaliser la récupération de ma passkey",
        }
    }

    pub fn libelle_bouton(self) -> &'static str {
        match self {
            EtapeLien::Confirmer => "Confirmer ma demande",
            EtapeLien::Annuler => "Annuler la récupération",
            EtapeLien::Finaliser => "Supprimer mes passkeys et me déconnecter partout",
        }
    }

    /// Paragraphes d'explication affichés **avant** l'action (aucune action sur simple ouverture
    /// du lien : un antivirus ou un aperçu de messagerie ne doit pas consommer le jeton).
    pub fn explications(self) -> &'static [&'static str] {
        match self {
            EtapeLien::Confirmer => &[
                "Vous avez demandé à récupérer l'accès à votre compte parce que vous avez perdu votre passkey.",
                "Pour votre sécurité, rien ne change immédiatement : en confirmant, vous ouvrez un délai de sécurité (24 heures en général). Pendant ce délai, vous recevez un e-mail avec un lien pour annuler la demande, et votre établissement en est informé dans son journal.",
                "Si vous retrouvez votre passkey et vous connectez avec elle pendant le délai, la demande est annulée automatiquement.",
                "Vous n'êtes pas à l'origine de cette demande ? Ne confirmez pas : fermez cette page.",
            ],
            EtapeLien::Annuler => &[
                "Vous annulez la demande de récupération de passkey faite sur votre compte.",
                "Rien n'est modifié : vos passkeys et vos appareils connectés restent en place. Si vous n'êtes pas à l'origine de la demande, changez ensuite votre mot de passe et prévenez votre établissement.",
            ],
            EtapeLien::Finaliser => &[
                "Le délai de sécurité est écoulé. En finalisant, toutes vos passkeys sont supprimées et tous vos appareils sont déconnectés.",
                "Vous pourrez ensuite vous connecter avec votre mot de passe (ou « mot de passe oublié ») et enregistrer une nouvelle passkey.",
                "Avant le délai, cette action est refusée : le lien reste valable, revenez l'utiliser plus tard (il expire 72 heures après la fin du délai).",
            ],
        }
    }

    /// Message de réussite.
    pub fn message_succes(self) -> &'static str {
        match self {
            EtapeLien::Confirmer => "Demande confirmée.",
            EtapeLien::Annuler => "Récupération annulée. Vos passkeys n'ont pas été modifiées.",
            EtapeLien::Finaliser => {
                "Récupération terminée : vos passkeys ont été supprimées et vos appareils déconnectés."
            }
        }
    }
}

/// Jeton lu dans l'adresse : sans espace, non vide, borné. `None` si inutilisable.
pub fn jeton_valide(brut: Option<&str>) -> Option<String> {
    let t = brut?.trim();
    (!t.is_empty() && t.len() <= JETON_MAX && !t.chars().any(char::is_whitespace))
        .then(|| t.to_string())
}

/// Adresse e-mail saisie : contrôle de confort, le serveur reste l'autorité.
pub fn valider_email(saisie: &str) -> Result<String, &'static str> {
    let t = saisie.trim();
    if t.is_empty() {
        return Err("Saisissez l'adresse e-mail de votre compte.");
    }
    if t.chars().count() > EMAIL_MAX {
        return Err("Cette adresse e-mail est trop longue.");
    }
    let mut parties = t.splitn(2, '@');
    let (local, domaine) = (parties.next().unwrap_or(""), parties.next().unwrap_or(""));
    if local.is_empty()
        || !domaine.contains('.')
        || domaine.starts_with('.')
        || domaine.ends_with('.')
        || t.chars().any(char::is_whitespace)
    {
        return Err("Cette adresse e-mail ne semble pas valide.");
    }
    Ok(t.to_string())
}

/// Identifiant d'établissement saisi ou lu dans l'adresse.
pub fn lire_etablissement(saisie: &str) -> Option<Uuid> {
    saisie.trim().parse().ok()
}

/// Message après l'envoi de la demande : **identique** que le compte existe ou non.
pub const MESSAGE_DEMANDE_NEUTRE: &str = "Si un compte correspond à cette adresse et possède une passkey, un e-mail vient d'être envoyé. Le lien qu'il contient est valable 60 minutes. Pensez à regarder vos courriers indésirables.";

/// Message d'une erreur à l'envoi de la demande (aucune information sur le compte).
pub fn message_erreur_demande(e: &ApiError) -> String {
    match e {
        ApiError::TropDeRequetes => {
            "Trop de demandes. Patientez quelques minutes avant de réessayer.".into()
        }
        ApiError::Invalide(_) => "Vérifiez l'adresse e-mail et l'établissement.".into(),
        autre => autre.message_utilisateur(),
    }
}

/// Message d'une erreur sur l'un des trois liens.
pub fn message_erreur_lien(etape: EtapeLien, e: &ApiError) -> String {
    match e {
        ApiError::Invalide(_) | ApiError::Introuvable(_) => {
            "Ce lien est invalide, expiré ou déjà utilisé. Si vous avez toujours perdu votre passkey, refaites une demande depuis la page de connexion."
                .into()
        }
        ApiError::Conflit(_) if etape == EtapeLien::Finaliser => {
            "Le délai de sécurité n'est pas encore écoulé. Le lien reste valable : revenez l'utiliser une fois le délai terminé."
                .into()
        }
        ApiError::TropDeRequetes => "Trop de tentatives. Patientez quelques minutes avant de réessayer.".into(),
        // La confirmation envoie un e-mail d'alerte : sans messagerie, le serveur refuse (503)
        // sans consommer le lien.
        ApiError::Serveur(503, _) => {
            "Le service d'envoi d'e-mails est momentanément indisponible. Le lien reste valable : réessayez dans quelques minutes."
                .into()
        }
        autre => autre.message_utilisateur(),
    }
}

/// Date et heure en français (« 6 novembre 2026 à 14 h 05 ») pour un instant local.
pub fn formater_fin(local: NaiveDateTime) -> String {
    use chrono::Timelike;
    format!(
        "{} à {} h {:02}",
        crate::noyau::temps::date_complete(local.date()),
        local.hour(),
        local.minute()
    )
}

/// Fin du délai affichée dans le fuseau du navigateur.
pub fn fin_du_delai(effective_at: DateTime<Utc>) -> String {
    formater_fin(effective_at.with_timezone(&Local).naive_local())
}

/// Adresse de la page de demande, avec l'établissement s'il est connu.
pub fn adresse_demande(etablissement: &str) -> String {
    match lire_etablissement(etablissement) {
        Some(id) => format!("/web/recuperation-passkey?etab={id}"),
        None => "/web/recuperation-passkey".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn jeton_de_l_adresse() {
        assert_eq!(jeton_valide(Some(" abc123 ")).as_deref(), Some("abc123"));
        assert_eq!(jeton_valide(None), None);
        assert_eq!(jeton_valide(Some("")), None);
        assert_eq!(jeton_valide(Some("a b")), None);
        assert_eq!(jeton_valide(Some(&"a".repeat(257))), None);
        assert!(jeton_valide(Some(&"a".repeat(256))).is_some());
    }

    #[test]
    fn validation_email() {
        assert_eq!(valider_email(" a@b.fr "), Ok("a@b.fr".into()));
        assert!(valider_email("").is_err());
        assert!(valider_email("sans-arobase").is_err());
        assert!(valider_email("@b.fr").is_err());
        assert!(valider_email("a@b").is_err());
        assert!(valider_email("a@.fr").is_err());
        assert!(valider_email("a b@c.fr").is_err());
        assert!(valider_email(&format!("{}@b.fr", "a".repeat(260))).is_err());
    }

    #[test]
    fn etablissement_de_l_adresse() {
        let id = Uuid::from_u128(5);
        assert_eq!(lire_etablissement(&format!(" {id} ")), Some(id));
        assert_eq!(lire_etablissement("pas-un-uuid"), None);
        assert_eq!(
            adresse_demande(&id.to_string()),
            format!("/web/recuperation-passkey?etab={id}")
        );
        assert_eq!(adresse_demande(""), "/web/recuperation-passkey");
    }

    #[test]
    fn la_demande_ne_dit_rien_du_compte() {
        assert!(!MESSAGE_DEMANDE_NEUTRE.contains("n'existe pas"));
        assert!(MESSAGE_DEMANDE_NEUTRE.starts_with("Si un compte"));
        // Aucune erreur serveur ne révèle l'existence du compte.
        let m = message_erreur_demande(&ApiError::Invalide("compte inconnu".into()));
        assert!(!m.contains("inconnu"));
        assert!(message_erreur_demande(&ApiError::TropDeRequetes).contains("Trop de demandes"));
    }

    #[test]
    fn erreurs_des_liens() {
        let invalide = message_erreur_lien(EtapeLien::Confirmer, &ApiError::Invalide("x".into()));
        assert!(invalide.contains("invalide, expiré ou déjà utilisé"));
        assert_eq!(
            invalide,
            message_erreur_lien(EtapeLien::Annuler, &ApiError::Introuvable("y".into()))
        );
        let delai = message_erreur_lien(EtapeLien::Finaliser, &ApiError::Conflit("délai".into()));
        assert!(delai.contains("délai de sécurité n'est pas encore écoulé"));
        assert!(delai.contains("reste valable"));
        let indispo =
            message_erreur_lien(EtapeLien::Confirmer, &ApiError::Serveur(503, "x".into()));
        assert!(indispo.contains("e-mails") && indispo.contains("reste valable"));
        // Un conflit sur une autre étape n'est pas présenté comme un délai.
        assert!(
            !message_erreur_lien(EtapeLien::Confirmer, &ApiError::Conflit("z".into()))
                .contains("délai de sécurité")
        );
    }

    #[test]
    fn chaque_etape_explique_le_delai_ou_l_effet() {
        for e in [
            EtapeLien::Confirmer,
            EtapeLien::Annuler,
            EtapeLien::Finaliser,
        ] {
            assert!(!e.titre().is_empty() && !e.libelle_bouton().is_empty());
            assert!(!e.explications().is_empty());
            assert!(!e.message_succes().is_empty());
        }
        let confirmer = EtapeLien::Confirmer.explications().join(" ");
        assert!(confirmer.contains("délai de sécurité"));
        assert!(confirmer.contains("annuler"));
        assert!(EtapeLien::Finaliser
            .explications()
            .join(" ")
            .contains("déconnectés"));
    }

    #[test]
    fn date_de_fin_lisible() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 6)
            .unwrap()
            .and_hms_opt(14, 5, 0)
            .unwrap();
        let t = formater_fin(d);
        assert!(
            t.contains("6") && t.contains("novembre") && t.contains("2026"),
            "{t}"
        );
        assert!(t.ends_with("14 h 05"), "{t}");
    }
}
