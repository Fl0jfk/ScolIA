//! Découverte des fichiers de migration d'un répertoire.
//!
//! Partagé entre `build.rs` (génération de la liste au moment de la
//! compilation) et les tests : `#[path]` dans `build.rs`, module ordinaire ici.
//! Aucune dépendance, aucune lecture du contenu SQL.

use std::path::{Path, PathBuf};

/// Un fichier `NNN_nom.sql` trouvé sur le disque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FichierMigration {
    pub numero: u16,
    /// Nom sans extension, par exemple `255_email_confirmation`.
    pub nom: String,
    pub chemin: PathBuf,
}

/// Vérifie la forme `NNN_nom` (trois chiffres, tiret bas, minuscules, chiffres
/// et tirets bas) et renvoie le numéro.
pub fn numero_de(nom: &str) -> Result<u16, String> {
    let octets = nom.as_bytes();
    if octets.len() < 5
        || !octets[..3].iter().all(u8::is_ascii_digit)
        || octets[3] != b'_'
        || !octets[4..]
            .iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_')
    {
        return Err(format!(
            "nom de migration invalide « {nom} » : attendu NNN_nom (minuscules, chiffres, _)"
        ));
    }
    nom[..3]
        .parse::<u16>()
        .map_err(|e| format!("numéro illisible dans « {nom} » : {e}"))
}

/// Liste les migrations d'un répertoire, triées par numéro. Refuse un nom
/// invalide et deux fichiers de même numéro (collision entre branches).
pub fn lister(dir: &Path) -> Result<Vec<FichierMigration>, String> {
    let mut trouves = Vec::new();
    let entrees =
        std::fs::read_dir(dir).map_err(|e| format!("lecture de {} : {e}", dir.display()))?;
    for entree in entrees {
        let chemin = entree.map_err(|e| e.to_string())?.path();
        if chemin.extension().and_then(|e| e.to_str()) != Some("sql") {
            continue;
        }
        let nom = chemin
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("nom non UTF-8 : {}", chemin.display()))?
            .to_string();
        let numero = numero_de(&nom)?;
        trouves.push(FichierMigration {
            numero,
            nom,
            chemin,
        });
    }
    trouves.sort_by(|a, b| a.numero.cmp(&b.numero).then_with(|| a.nom.cmp(&b.nom)));
    for paire in trouves.windows(2) {
        if paire[0].numero == paire[1].numero {
            return Err(format!(
                "deux migrations portent le numéro {:03} : {} et {}",
                paire[0].numero, paire[0].nom, paire[1].nom
            ));
        }
    }
    Ok(trouves)
}
