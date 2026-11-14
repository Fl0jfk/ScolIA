//! `ent-reprise` — outil de reprise des données du Next.js vers ScolIA (socle 28, lots R0 et R1).
//!
//! **Ce qu'il fait** : lire un **export** (dump PostgreSQL au format texte ou répertoire + liste
//! d'objets S3), l'inspecter (`inspecter`) et en produire une version anonymisée (`anonymiser`).
//!
//! **Ce qu'il ne fait jamais** : se connecter à une base, à un service S3 ou à un réseau. Il ne
//! contient ni client PostgreSQL ni client HTTP (voir `Cargo.toml`) et refuse toute URL de
//! connexion ([`garde`]). Il ne lit aucun secret d'environnement.

pub mod anonymisation;
pub mod copy;
pub mod dump;
pub mod empreintes;
pub mod garde;
pub mod inspection;
pub mod jeu_fictif;
pub mod s3;
pub mod schema;

/// Erreur de l'outil : message lisible en français, **jamais** une valeur de donnée.
#[derive(Debug)]
pub struct Erreur(pub String);

impl std::fmt::Display for Erreur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Erreur {}

impl From<std::io::Error> for Erreur {
    fn from(e: std::io::Error) -> Self {
        Erreur(format!("erreur d'entrée/sortie : {e}"))
    }
}

pub type Res<T> = Result<T, Erreur>;

/// Raccourci : `err!("texte {}", x)`.
#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => { $crate::Erreur(format!($($arg)*)) };
}
