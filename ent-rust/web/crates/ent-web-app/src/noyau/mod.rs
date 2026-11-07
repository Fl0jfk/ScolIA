//! Noyau de l'application : session, droits, navigation, coquille, erreurs, dates.
//! Les modules métier en dépendent ; le noyau ne dépend d'aucun module, sauf de la
//! configuration publique de connexion.

pub mod coquille;
pub mod droits;
pub mod droits_moteur;
pub mod ecriture;
pub mod erreurs;
pub mod expiration;
pub mod navigation;
pub mod non_lus;
pub mod portee;
pub mod reconnexion;
pub mod session;
pub mod telechargement;
pub mod temps;
pub mod webauthn;
