//! Droits effectifs du moteur D-017 : `GET /roles/mes-droits`.
//!
//! L'appel `ApiClient::mes_droits` est défini dans `roles.rs` (écrans rôles et droits) ;
//! `DroitEffectif` est le même type que `DroitApi` (action, portée, cibles), gardé sous
//! ce nom pour la navigation par les droits (intégration du lot 4).

/// Un droit effectif : action du catalogue (`referentiel.gerer`…) et portée.
pub type DroitEffectif = crate::roles::DroitApi;
