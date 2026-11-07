//! Génère la liste des migrations du noyau à partir du répertoire
//! `ent-rust/migrations/` (D-026, étape 1 de `docs/ARCHITECTURE_BRIQUES.md`).
//!
//! Plus aucune liste écrite à la main : ajouter un fichier suffit. La même
//! fonction servira à chaque brique pour son propre répertoire `migrations/`.

#[path = "src/decouverte.rs"]
mod decouverte;

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let manifeste = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let dir = manifeste.join("../../migrations");
    println!("cargo:rerun-if-changed={}", dir.display());

    let fichiers = decouverte::lister(&dir).unwrap_or_else(|e| panic!("migrations : {e}"));

    let mut code = String::from(
        "/// Généré par build.rs depuis `ent-rust/migrations/` : ne pas modifier.\n\
         pub static MIGRATIONS_NOYAU: &[Migration] = &[\n",
    );
    for f in &fichiers {
        let chemin = f
            .chemin
            .canonicalize()
            .unwrap_or_else(|e| panic!("{} : {e}", f.chemin.display()));
        println!("cargo:rerun-if-changed={}", chemin.display());
        writeln!(
            code,
            "    Migration {{ numero: {}, nom: {:?}, sql: include_str!({:?}), brique: \"noyau\" }},",
            f.numero,
            f.nom,
            chemin.display().to_string()
        )
        .expect("écriture en mémoire");
    }
    code.push_str("];\n");

    let sortie =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("migrations_noyau.rs");
    std::fs::write(&sortie, code).unwrap_or_else(|e| panic!("{} : {e}", sortie.display()));
}
