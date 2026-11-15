//! Garde « jamais de connexion » (R-MSE10, CL28-12).
//!
//! L'outil ne lit que des **fichiers** locaux. Toute valeur d'option qui ressemble à une URL de
//! connexion (base de données, S3, HTTP…) ou à une chaîne de connexion libpq est refusée **avant**
//! toute lecture. Le refus ne répète jamais la valeur reçue (elle pourrait contenir un mot de
//! passe) : il dit seulement laquelle des options est en cause.

use crate::{err, Res};
use std::path::{Path, PathBuf};

/// Schémas d'URL reconnus comme des connexions (liste volontairement large).
const SCHEMAS: &[&str] = &[
    "postgres",
    "postgresql",
    "pg",
    "jdbc",
    "mysql",
    "mariadb",
    "mongodb",
    "redis",
    "amqp",
    "s3",
    "s3a",
    "gs",
    "gcs",
    "az",
    "abfs",
    "http",
    "https",
    "ftp",
    "ftps",
    "sftp",
    "ssh",
    "tcp",
    "ws",
    "wss",
    "file+ssh",
    "scaleway",
    "oss",
    "r2",
];

/// Vrai si `valeur` ressemble à une URL de connexion ou à une chaîne de connexion libpq.
pub fn ressemble_a_une_connexion(valeur: &str) -> bool {
    let v = valeur.trim();
    if let Some((schema, reste)) = v.split_once(':') {
        let s = schema.to_ascii_lowercase();
        let schema_ok = !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
            && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
        // « c:\dossier » (lecteur Windows, une seule lettre) n'est pas un schéma.
        if schema_ok && s.len() > 1 && (reste.starts_with("//") || SCHEMAS.contains(&s.as_str())) {
            return true;
        }
    }
    // Chaîne libpq : « host=… dbname=… user=… password=… ».
    let bas = v.to_ascii_lowercase();
    [
        "host=",
        "hostaddr=",
        "dbname=",
        "user=",
        "password=",
        "port=",
        "sslmode=",
        "service=",
    ]
    .iter()
    .any(|k| bas.starts_with(k) || bas.contains(&format!(" {k}")))
}

/// Refuse une valeur d'option qui serait une connexion. `option` = nom de l'option (`--dump`…).
pub fn exiger_pas_de_connexion(option: &str, valeur: &str) -> Res<()> {
    if ressemble_a_une_connexion(valeur) {
        return Err(err!(
            "refusé : la valeur de {option} ressemble à une adresse de connexion (base ou réseau). \
             Cet outil ne lit que des fichiers locaux : fournissez le chemin d'un fichier ou d'un dossier."
        ));
    }
    Ok(())
}

/// Chemin local **existant** (fichier ou dossier), après refus des connexions.
pub fn chemin_local_existant(option: &str, valeur: &str) -> Res<PathBuf> {
    exiger_pas_de_connexion(option, valeur)?;
    let p = Path::new(valeur);
    if !p.exists() {
        return Err(err!("{option} : le chemin n'existe pas ({})", p.display()));
    }
    Ok(p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_de_connexion_refusees() {
        for v in [
            "postgres://u:p@127.0.0.1:5432/scolia",
            "postgresql://localhost/db",
            "PostgreSQL://x",
            "jdbc:postgresql://h/db",
            "s3://bucket/prefixe",
            "https://exemple.test/export",
            "host=localhost dbname=scolia user=x",
            "dbname=prod",
            "mongodb+srv://c",
            " postgres://espaces ",
        ] {
            assert!(ressemble_a_une_connexion(v), "{v}");
            assert!(exiger_pas_de_connexion("--dump", v).is_err(), "{v}");
        }
    }

    #[test]
    fn chemins_locaux_acceptes() {
        for v in [
            "export.sql",
            "./export/dump",
            "/home/florian/export.sql",
            "C:\\Users\\florian\\export.sql",
            "c:/export/dump.sql",
            "dossier avec espaces/dump.sql",
            "export-2026-12-04.sql",
            "liste.tsv",
        ] {
            assert!(!ressemble_a_une_connexion(v), "{v}");
        }
    }

    #[test]
    fn le_refus_ne_repete_pas_la_valeur() {
        let e = exiger_pas_de_connexion("--dump", "postgres://admin:motdepasse-secret@h/db")
            .unwrap_err();
        assert!(!e.0.contains("motdepasse-secret"));
        assert!(!e.0.contains("admin"));
        assert!(e.0.contains("--dump"));
    }

    #[test]
    fn chemin_inexistant_refuse() {
        assert!(chemin_local_existant("--dump", "/chemin/qui/nexiste/pas").is_err());
    }
}
