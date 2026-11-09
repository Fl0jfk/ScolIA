//! Types JSON partagés entre `ent-api` (serveur) et le frontend Leptos (`ent-rust/web`).
//!
//! **Lot 1** (docs/ENT_TYPES_PROPOSITION.md) : appel, emploi du temps, session.
//!
//! Règles :
//! - aucune dépendance serveur (sqlx, ent-db, domain-eleve, axum) ; compile en WebAssembly ;
//! - le JSON est **identique** à celui produit aujourd'hui par `ent-api` : mêmes noms
//!   de champs, mêmes attributs serde. Vérifié par un test « aller-retour » contre le
//!   vrai backend (`ent-rust/web/crates/ent-web-server/tests/contre_backend.rs`) ;
//! - pas de logique métier ; les états restent des chaînes (`"present"`, `"clos"`…)
//!   tant que leur typage n'a pas été décidé ;
//! - `ent-api` **n'utilise pas encore** ces types : la bascule se fera lot par lot,
//!   avec réexport des noms dans `ent-api/src/dto.rs` et un trait de conversion local
//!   (`VersDto`), voir la proposition.

pub mod appel;
pub mod auth;
pub mod cahier;
pub mod communication;
pub mod discipline;
pub mod dossier;
pub mod edt;
pub mod edt_s2;
pub mod evaluations;
pub mod finances;
pub mod fusion;
pub mod identifiants;
pub mod identite;
pub mod inscriptions;
pub mod personnels;
pub mod propositions;
pub mod referentiel;
pub mod referentiel_admin;
pub mod sante;
pub mod scolarite;
pub mod vie_scolaire;

#[cfg(test)]
mod tests;
