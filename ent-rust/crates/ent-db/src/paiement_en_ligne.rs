//! Paiement en ligne par carte (FIN-09) — **interface seulement**.
//!
//! La question Q5 (`docs/SOCLE2_DOMAINES.md` §7 : prestataire de paiement) n'est
//! **pas décidée** : aucun prestataire n'est intégré ni appelé. Ce module fixe le
//! contrat qu'un futur connecteur devra respecter, et fournit l'implémentation
//! « désactivé » utilisée partout aujourd'hui.
//!
//! Exigences pour un futur prestataire (à valider avec Q5) : prestataire européen,
//! clés jamais exposées au navigateur, montant et facture revérifiés côté serveur
//! avant l'intention, notification serveur à serveur **signée** et vérifiée,
//! encaissement idempotent par identifiant de transaction, aucun numéro de carte
//! stocké par ScolIA.

use ent_types::finances::EtatPaiementEnLigneDto;
use uuid::Uuid;

/// Demande de paiement d'une facture (montant revérifié par le service).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentionPaiement {
    pub etablissement_id: Uuid,
    pub facture_id: Uuid,
    pub foyer_id: Uuid,
    pub montant_centimes: i64,
    /// Référence interne unique (idempotence côté prestataire).
    pub reference: String,
}

/// Où envoyer le payeur pour payer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectionPaiement {
    pub url: String,
    pub reference_prestataire: String,
}

/// Paiement confirmé par le prestataire (après vérification de la signature).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPaiement {
    pub reference: String,
    pub reference_prestataire: String,
    pub montant_centimes: i64,
    pub reussi: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurPaiement {
    #[error("paiement en ligne non activé (prestataire non choisi, décision Q5)")]
    Desactive,
    #[error("notification de paiement non authentifiée")]
    SignatureInvalide,
    #[error("prestataire indisponible")]
    Indisponible,
}

/// Contrat d'un prestataire de paiement en ligne.
pub trait PrestatairePaiement: Send + Sync {
    /// Nom affichable du prestataire (`None` : aucun).
    fn nom(&self) -> Option<&'static str>;
    fn actif(&self) -> bool;
    fn creer_intention(
        &self,
        intention: &IntentionPaiement,
    ) -> Result<RedirectionPaiement, ErreurPaiement>;
    /// Vérifie et décode une notification serveur à serveur.
    fn verifier_notification(
        &self,
        corps: &[u8],
        signature: &str,
    ) -> Result<NotificationPaiement, ErreurPaiement>;
}

/// Implémentation par défaut : tout est refusé, rien n'est appelé.
#[derive(Debug, Clone, Copy, Default)]
pub struct PaiementDesactive;

impl PrestatairePaiement for PaiementDesactive {
    fn nom(&self) -> Option<&'static str> {
        None
    }

    fn actif(&self) -> bool {
        false
    }

    fn creer_intention(
        &self,
        _intention: &IntentionPaiement,
    ) -> Result<RedirectionPaiement, ErreurPaiement> {
        Err(ErreurPaiement::Desactive)
    }

    fn verifier_notification(
        &self,
        _corps: &[u8],
        _signature: &str,
    ) -> Result<NotificationPaiement, ErreurPaiement> {
        Err(ErreurPaiement::Desactive)
    }
}

pub fn etat_dto(p: &dyn PrestatairePaiement) -> EtatPaiementEnLigneDto {
    EtatPaiementEnLigneDto {
        actif: p.actif(),
        prestataire: p.nom().map(str::to_string),
        message: if p.actif() {
            "Paiement par carte disponible.".into()
        } else {
            "Le paiement en ligne n'est pas encore proposé par l'établissement. Prélèvement, virement, chèque ou espèces restent possibles.".into()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desactive_refuse_tout() {
        let p = PaiementDesactive;
        assert!(!p.actif());
        assert_eq!(p.nom(), None);
        let i = IntentionPaiement {
            etablissement_id: Uuid::nil(),
            facture_id: Uuid::nil(),
            foyer_id: Uuid::nil(),
            montant_centimes: 100,
            reference: "x".into(),
        };
        assert_eq!(p.creer_intention(&i), Err(ErreurPaiement::Desactive));
        assert_eq!(
            p.verifier_notification(b"{}", "sig"),
            Err(ErreurPaiement::Desactive)
        );
        let e = etat_dto(&p);
        assert!(!e.actif);
        assert!(e.prestataire.is_none());
    }
}
