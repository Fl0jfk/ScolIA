//! Classification **commune** des erreurs Postgres en erreurs de domaine.
//!
//! Historiquement chaque `*_repo.rs` mappait *toute* erreur sqlx en
//! `DomainError::NotFound` (→ HTTP 404), y compris les conflits d'unicité ou
//! les références invalides. Ce module rétablit un sens exploitable côté API :
//!
//! | Code PG | Sens              | Variante de domaine          |
//! |---------|-------------------|------------------------------|
//! | `23505` | unicité           | `DomainError::Conflit`       |
//! | `23503` | clé étrangère     | `DomainError::ReferenceInconnue` |
//! | `23514` | CHECK             | `DomainError::ValeurInvalide` |
//! | `23502` | NOT NULL          | `DomainError::ValeurInvalide` |
//! | autre   | —                 | `DomainError::NotFound` (repli historique) |
//!
//! Le domaine reste ignorant du SQL : l'infrastructure traduit.

use domain_eleve::DomainError;

pub fn map_sqlx(scope: &str, err: sqlx::Error) -> DomainError {
    if let sqlx::Error::Database(db) = &err {
        if let Some(code) = db.code() {
            match code.as_ref() {
                "23505" => {
                    eprintln!("[ent-db] {scope} conflit d'unicité: {err}");
                    return DomainError::Conflit;
                }
                "23503" => {
                    eprintln!("[ent-db] {scope} référence inconnue: {err}");
                    return DomainError::ReferenceInconnue;
                }
                "23514" | "23502" => {
                    eprintln!("[ent-db] {scope} valeur invalide: {err}");
                    return DomainError::ValeurInvalide;
                }
                _ => {}
            }
        }
    }
    eprintln!("[ent-db] {scope} sqlx error: {err}");
    DomainError::NotFound
}
