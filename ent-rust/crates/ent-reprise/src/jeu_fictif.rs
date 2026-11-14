//! Jeu de données **entièrement fictif** au format du Next.js (commande `jeu-fictif`).
//!
//! Tout est inventé : noms et prénoms de personnages imaginaires, adresses de rues qui n'existent
//! pas, e-mails en `example.test` (domaine réservé à la documentation, RFC 2606), numéros hors des
//! plages attribuées, IBAN d'exemple. Aucune donnée réelle, aucune base consultée : le schéma vient
//! du fichier embarqué [`crate::schema`].
//!
//! Il sert à tester `inspecter` et `anonymiser`. Il contient volontairement des **valeurs
//! repérables** (les [`SENTINELLES`]) placées dans les colonnes sensibles, dans des textes libres,
//! des JSON, des collections clé-valeur et des clés d'objets S3 : après anonymisation, aucune ne
//! doit rester.

use crate::copy::encoder_ligne;
use crate::s3::{self, ObjetS3};
use crate::schema::{Genre, ListeBlanche, Regle, SchemaAttendu, TABLES_SANTE};
use crate::{empreintes, Res};
use std::fmt::Write as _;
use std::path::Path;

pub const NOMS: &[&str] = &[
    "Zorglub",
    "Tramoulin",
    "Quelnoir",
    "Brasimont",
    "Valdreuil",
    "Moncelard",
    "Ferbaluche",
    "Ostrapek",
    "Lumbrecy",
    "Dourvanne",
];
pub const PRENOMS: &[&str] = &[
    "Aldric",
    "Mirelle",
    "Tancrède",
    "Oriane",
    "Basile",
    "Ysolde",
    "Gaspard",
    "Nolwenn",
    "Cyprien",
    "Elvire",
];

/// Identifiant de l'établissement fictif.
pub const ETABLISSEMENT: &str = "0e7ab115-0000-4000-8000-00000000f1c7";

/// Valeurs fictives qui ne doivent **jamais** survivre à l'anonymisation.
pub const SENTINELLES: &[&str] = &[
    "Zorglub",
    "Tramoulin",
    "Quelnoir",
    "Brasimont",
    "Valdreuil",
    "Moncelard",
    "Ferbaluche",
    "Ostrapek",
    "Lumbrecy",
    "Dourvanne",
    "Aldric",
    "Mirelle",
    "Tancrède",
    "Oriane",
    "Basile",
    "Ysolde",
    "Gaspard",
    "Nolwenn",
    "Cyprien",
    "Elvire",
    "FR7630006000011234567890189",
    "Brumeville",
    "Lilas-Fictifs",
];

const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

fn uuid_det(table: &str, col: &str, n: usize) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in table.bytes().chain(col.bytes()).chain(n.to_le_bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    let h2 = h.rotate_left(32) ^ 0x9e3779b97f4a7c15;
    format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        (h >> 32) as u32,
        (h >> 16) as u16,
        (h & 0xfff) as u16,
        (h2 & 0xfff) as u16,
        h2 >> 16 & 0xffff_ffff_ffff
    )
}

fn nom(n: usize) -> &'static str {
    NOMS[n % NOMS.len()]
}
fn prenom(n: usize) -> &'static str {
    PRENOMS[(n + 3) % PRENOMS.len()]
}

fn valeur_pseudo(g: Genre, table: &str, col: &str, n: usize) -> String {
    match g {
        Genre::Nom => nom(n).to_string(),
        Genre::Prenom => prenom(n).to_string(),
        Genre::NomComplet => format!("{} {}", prenom(n), nom(n).to_uppercase()),
        Genre::Email => format!("{}.{}{n}@example.test", prenom(n).to_lowercase().replace('è', "e"), nom(n).to_lowercase()),
        Genre::Telephone => format!("06 12 34 56 {:02}", n % 100),
        Genre::Ine => format!("{:010}{}", 1_234_567_000 + n, (b'A' + (n % 26) as u8) as char),
        Genre::Iban => "FR7630006000011234567890189".to_string(),
        Genre::Bic => "AGRIFRPP".to_string(),
        Genre::Rum => format!("RUM-FICTIF-{n:05}"),
        Genre::DateNaissance => format!("2010-03-{:02}", 10 + n % 15),
        Genre::Ville => "Brumeville-sur-Fictif".to_string(),
        Genre::Adresse => format!("{} rue des Lilas-Fictifs", 12 + n),
        Genre::CodePostal => "75999".to_string(),
        Genre::Ip => format!("203.0.113.{}", 10 + n % 200),
        Genre::Agent => "Mozilla/5.0 (X11; Fictif) NavigateurInvente/1.0".to_string(),
        Genre::Identifiant => format!("user_fictif_{table}_{n}"),
        Genre::IdSouple => format!("{}-{}-{n}", prenom(n).to_lowercase().replace('è', "e"), nom(n).to_lowercase()),
        Genre::Jeton => format!("tok{}", uuid_det(table, col, n).replace('-', "")),
        Genre::Image => PNG_1X1.to_string(),
        Genre::Pdf => "JVBERi0xLjQKJSVFT0YK".to_string(),
        Genre::NomFichier => format!("Justificatif {} {}.pdf", prenom(n), nom(n)),
        Genre::Texte => format!(
            "Observation libre : {} {} a été reçu(e) le {} mars, sa mère {} {} à rappeler (texte inventé n° {n}).",
            prenom(n),
            nom(n),
            1 + n % 28,
            prenom(n + 1),
            nom(n)
        ),
    }
}

fn cle_s3_fictive(n: usize, variante: usize) -> String {
    match variante % 4 {
        0 => format!("eleves/photos/{}.jpg", uuid_det("eleve", "photo", n)),
        1 => format!(
            "documents/users/{}/CV-{}-{}.pdf",
            uuid_det("user", "doc", n),
            nom(n),
            prenom(n)
        ),
        2 => format!(
            "requests/{}/files/justificatif-{}.pdf",
            uuid_det("req", "f", n),
            nom(n).to_lowercase()
        ),
        _ => format!(
            "rentree/submissions/pending/{}/fiche-{}.json",
            uuid_det("rentree", "t", n),
            nom(n).to_lowercase()
        ),
    }
}

fn json_fictif(table: &str, col: &str, n: usize) -> String {
    let nm = nom(n);
    let pr = prenom(n);
    format!(
        "{{\"nom\":\"{nm}\",\"prenom\":\"{pr}\",\"email\":\"{}.{}{n}@example.test\",\"statut\":\"actif\",\"nombre\":{n},\"actif\":true,\"classes\":[\"6A\",\"5B\"],\"contact\":{{\"telephone\":\"06 12 34 56 {:02}\",\"adresse\":\"{} rue des Lilas-Fictifs\"}},\"commentaire\":\"Texte libre sur {pr} {nm} ({table}.{col})\",\"cle_inconnue_du_dictionnaire\":\"valeur libre {nm}\",\"photo_key\":\"{}\"}}",
        pr.to_lowercase().replace('è', "e"),
        nm.to_lowercase(),
        n % 100,
        12 + n,
        cle_s3_fictive(n, 0),
    )
}

fn attr_fictif(table: &str, n: usize) -> (String, String) {
    let nm = nom(n);
    let pr = prenom(n);
    match n % 8 {
        0 => ("nom".into(), format!("s:{nm}")),
        1 => ("prenom".into(), format!("s:{pr}")),
        2 => (
            "email".into(),
            format!(
                "s:{}.{}{n}@example.test",
                pr.to_lowercase().replace('è', "e"),
                nm.to_lowercase()
            ),
        ),
        3 => ("age".into(), format!("n:{}", 10 + n)),
        4 => ("actif".into(), "b:1".into()),
        5 => (
            "notes.0.texte".into(),
            format!("s:Texte libre sur {pr} {nm} ({table})"),
        ),
        6 => (
            "__root".into(),
            format!(
                "j:{{\"{}\":\"{:010}A\"}}",
                uuid_det(table, "k", n),
                1_234_567_000 + n
            ),
        ),
        _ => ("fichier_key".into(), format!("s:{}", cle_s3_fictive(n, n))),
    }
}

fn est_tableau(t: &str) -> bool {
    t.ends_with("[]")
}

fn litteral_tableau(elements: &[String]) -> String {
    let mut s = String::from("{");
    for (i, e) in elements.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('"');
        s.push_str(&e.replace('\\', "\\\\").replace('"', "\\\""));
        s.push('"');
    }
    s.push('}');
    s
}

#[allow(clippy::too_many_arguments)]
fn valeur(
    table: &str,
    col: &str,
    type_sql: &str,
    regle: Regle,
    n: usize,
    attr_collection: &str,
) -> Option<String> {
    if col == "etablissement_id" {
        return Some(ETABLISSEMENT.to_string());
    }
    let un = |r: Regle, n: usize| -> String {
        match r {
            Regle::Pseudo(g) => valeur_pseudo(g, table, col, n),
            Regle::CleS3 => cle_s3_fictive(n, n),
            _ => unreachable!(),
        }
    };
    let v = match regle {
        Regle::Effacer => format!("secret-{}", uuid_det(table, col, n)),
        Regle::Pseudo(_) | Regle::CleS3 => {
            if est_tableau(type_sql) {
                litteral_tableau(&[un(regle, n), un(regle, n + 1)])
            } else {
                un(regle, n)
            }
        }
        Regle::Json => json_fictif(table, col, n),
        Regle::AttrValeur => attr_fictif(table, n).1,
        Regle::Chemin => attr_fictif(table, n).0,
        Regle::Garder => {
            if col == "collection" {
                attr_collection.to_string()
            } else if est_tableau(type_sql) {
                litteral_tableau(&["role_a".to_string(), "role_b".to_string()])
            } else {
                match type_sql {
                    "uuid" => uuid_det(table, col, n),
                    "text" => match col {
                        "status" | "statut" => "actif".into(),
                        "id" | "token" => format!("{table}-{n}"),
                        "slug" => format!("etab-fictif-{n}"),
                        "name" => "Collège des Brumes (fictif)".into(),
                        "type" | "kind" | "category" => "type-fictif".into(),
                        c if c.ends_with("_at") => format!("2026-09-{:02}T08:00:00Z", 1 + n % 28),
                        c if c.ends_with("_id") || c.ends_with("_key") => format!("{c}-{n}"),
                        c => format!("{c}-{n}"),
                    },
                    "timestamp with time zone" => format!("2026-09-{:02} 08:30:00+00", 1 + n % 28),
                    "date" => format!("2026-09-{:02}", 1 + n % 28),
                    "integer" | "bigint" => (n + 1).to_string(),
                    "boolean" => (if n % 2 == 0 { "t" } else { "f" }).to_string(),
                    "numeric" => format!("{}.50", 10 + n),
                    "double precision" => "0.5".to_string(),
                    "jsonb" => "{}".to_string(),
                    _ => format!("v{n}"),
                }
            }
        }
    };
    Some(v)
}

/// Résumé de la génération.
#[derive(Debug)]
pub struct Resume {
    pub tables: usize,
    pub lignes: u64,
    pub objets_s3: usize,
}

/// Écrit le jeu fictif dans `dossier` : `dump.sql`, `objets_s3.tsv`, `SHA256SUMS`, `LISEZMOI.txt`.
pub fn generer(dossier: &Path, lignes_par_table: usize) -> Res<Resume> {
    let schema = SchemaAttendu::embarque();
    let blanche = ListeBlanche::embarquee();
    std::fs::create_dir_all(dossier)?;
    let mut sql = String::new();
    sql.push_str("--\n-- Jeu de données FICTIF au format du Next.js (généré par ent-reprise jeu-fictif).\n-- Aucune donnée réelle.\n--\n\n");
    sql.push_str("SET statement_timeout = 0;\nSET client_encoding = 'UTF8';\nSET standard_conforming_strings = on;\n\n");
    let mut lignes_total = 0u64;
    for (table, cols) in &schema.tables {
        let _ = writeln!(sql, "CREATE TABLE {} (", qualifier(table));
        for (i, c) in cols.iter().enumerate() {
            let _ = writeln!(
                sql,
                "    {} {}{}{}",
                ident(&c.nom),
                c.type_sql,
                if c.nullable { "" } else { " NOT NULL" },
                if i + 1 < cols.len() { "," } else { "" }
            );
        }
        sql.push_str(");\n\n");
    }
    for (table, cols) in &schema.tables {
        // Lignes : quelques-unes partout, une seule dans les tables « mortes » sauf `annee_bascule`
        // (non vide exprès, pour vérifier l'alerte), aucune dans les tables optionnelles.
        let n_lignes = if table == "drizzle.__drizzle_migrations" {
            2
        } else if crate::schema::TABLES_OPTIONNELLES.contains(&table.as_str()) {
            0
        } else if table == "annee_bascule" || TABLES_SANTE.contains(&table.as_str()) {
            2
        } else if crate::inspection::TABLES_MORTES.contains(&table.as_str()) {
            0
        } else {
            lignes_par_table
        };
        let noms: Vec<String> = cols.iter().map(|c| ident(&c.nom)).collect();
        let _ = writeln!(
            sql,
            "COPY {} ({}) FROM stdin;",
            qualifier(table),
            noms.join(", ")
        );
        let collections = [
            "internat__rooms",
            "pilotage__eleves-index",
            "rh__onboarding",
        ];
        for n in 0..n_lignes {
            let coll = collections[n % collections.len()];
            let cells: Vec<Option<String>> = cols
                .iter()
                .map(|c| {
                    let regle = blanche.regle(table, &c.nom)?;
                    valeur(table, &c.nom, &c.type_sql, regle, n, coll)
                })
                .collect();
            sql.push_str(&encoder_ligne(&cells));
            sql.push('\n');
            lignes_total += 1;
        }
        sql.push_str("\\.\n\n");
    }
    std::fs::write(dossier.join("dump.sql"), sql)?;

    // Objets S3 : photos, pièces, dépôts en attente… plus des objets « secrets » à ne pas reprendre.
    let mut objets: Vec<ObjetS3> = (0..24)
        .map(|n| ObjetS3 {
            cle: cle_s3_fictive(n, n),
            taille: 1000 + (n as u64) * 37,
            sha256: Some(hex_fictif(n)),
        })
        .collect();
    objets.push(ObjetS3 {
        cle: "tenants/index.json".into(),
        taille: 800,
        sha256: None,
    });
    objets.push(ObjetS3 {
        cle: "tenants/etab-fictif/secrets/cle.json".into(),
        taille: 120,
        sha256: None,
    });
    std::fs::write(dossier.join("objets_s3.tsv"), s3::ecrire_liste(&objets))?;
    std::fs::write(
        dossier.join("LISEZMOI.txt"),
        "Jeu fictif au format du Next.js, entièrement inventé (noms imaginaires, e-mails en example.test).\n\
Fichiers : dump.sql (dump texte), objets_s3.tsv (liste d'objets), SHA256SUMS (empreintes).\n\
Essai : ent-reprise inspecter --dump dump.sql --objets-s3 objets_s3.tsv --sommes SHA256SUMS\n",
    )?;
    let sommes = empreintes::ecrire_sommes(dossier, &["dump.sql", "objets_s3.tsv"])?;
    std::fs::write(dossier.join("SHA256SUMS"), sommes)?;
    Ok(Resume {
        tables: schema.tables.len(),
        lignes: lignes_total,
        objets_s3: objets.len(),
    })
}

fn hex_fictif(n: usize) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(format!("objet-fictif-{n}").as_bytes()))
}

/// Nom qualifié d'une table : `public.nom`, ou `schema.nom` pour une clé `schema.nom`.
pub fn qualifier(table: &str) -> String {
    match table.split_once('.') {
        Some((s, t)) => format!("{}.{}", ident(s), ident(t)),
        None => format!("public.{}", ident(table)),
    }
}

/// Identifiant SQL : entre guillemets pour les mots réservés (comme `pg_dump`), nu sinon.
pub fn ident(s: &str) -> String {
    const STRICTS: &[&str] = &[
        "user",
        "by",
        "order",
        "group",
        "table",
        "end",
        "from",
        "to",
        "check",
        "default",
        "limit",
        "offset",
        "primary",
        "references",
        "select",
        "when",
        "session",
        "current_user",
        "at",
    ];
    let simple = !s.is_empty()
        && s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if simple && !STRICTS.contains(&s) {
        s.to_string()
    } else {
        format!("\"{}\"", s.replace('"', "\"\""))
    }
}
