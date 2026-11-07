//! # ent-kit — PROTOTYPE du kit de branchement des briques (D-026)
//!
//! Conception : `docs/ARCHITECTURE_BRIQUES.md`. Ce crate lève deux doutes
//! avant la refonte, **sans rien brancher** sur `ent-db` ni `ent-api` (aucun
//! changement de comportement) :
//!
//! 1. la liste des migrations et `MIGRATION_TAG` peuvent être **calculés**
//!    depuis le répertoire (build.rs) et donnent exactement la liste écrite à
//!    la main dans `ent-db/src/migrate.rs` (test `tests/existant.rs`) ;
//! 2. les actions D-017 peuvent être des **constantes déclarées par chaque
//!    brique**, utilisables comme motifs de `match`, ce qui permet de quitter
//!    l'`enum` fermé de `ent-auth` sans toucher les usages.
//!
//! S'y ajoutent les déclarations de routes (avec les cas d'isolation qui en
//! découlent) et la vérification d'un assemblage de briques.

pub mod actions;
pub mod brique;
pub mod decouverte;
pub mod migrations;
pub mod routes;
