//! Noyau de l'application : session, droits, navigation, coquille, erreurs, dates.
//! Les modules métier en dépendent ; le noyau ne dépend d'aucun module, sauf de la
//! configuration publique de connexion.

pub mod coquille;
pub mod droits;
pub mod erreurs;
pub mod navigation;
pub mod session;
pub mod temps;
