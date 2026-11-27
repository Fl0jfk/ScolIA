//! Sécurité du compte (socle 1) : enrôlement obligatoire d'une passkey, gestion des passkeys
//! et des appareils connectés, politique de sécurité de l'établissement (Direction).
//!
//! Routes d'`ent-api` utilisées, inchangées : `/auth/me`, `/auth/passkeys`,
//! `/auth/passkey/register/*`, `/auth/sessions`, `/etablissement/securite`.
//! Le serveur reste l'autorité (mot de passe actuel, plafond de passkeys, droits Direction).

mod compte;
mod enrolement;
pub mod modele;
mod politique;

pub use compte::PageSecurite;
pub use enrolement::PageEnrolement;
pub use politique::PagePolitiqueSecurite;

use ent_web_api::{ApiError, PasskeyCompte};

use crate::noyau::session;
use crate::noyau::webauthn;

/// Échec d'un enrôlement : refus du serveur ou de l'appareil (annulation, navigateur sans passkey).
pub(crate) enum EchecEnrolement {
    Serveur(ApiError),
    Appareil(String),
}

impl EchecEnrolement {
    pub(crate) fn message(&self) -> String {
        match self {
            EchecEnrolement::Serveur(e) => modele::message_action_sensible(e),
            EchecEnrolement::Appareil(m) => m.clone(),
        }
    }

    pub(crate) fn session_perdue(&self) -> bool {
        matches!(self, EchecEnrolement::Serveur(ApiError::NonConnecte))
    }
}

/// Enrôle une passkey sur l'appareil : défi du serveur, création par l'appareil (aucun
/// secret ne transite par la page), envoi de l'attestation. `mot_de_passe` vide en session
/// « enrôlement » (le serveur l'accepte alors).
pub(crate) async fn enroler_passkey(
    nom: &str,
    mot_de_passe: &str,
) -> Result<PasskeyCompte, EchecEnrolement> {
    let client = session::client();
    let defi = client
        .passkey_enrolement_debut()
        .await
        .map_err(EchecEnrolement::Serveur)?;
    let credential = webauthn::creer_passkey(&defi.options)
        .await
        .map_err(EchecEnrolement::Appareil)?;
    client
        .passkey_enrolement_fin(&defi.token, credential, Some(nom), mot_de_passe)
        .await
        .map_err(EchecEnrolement::Serveur)
}
