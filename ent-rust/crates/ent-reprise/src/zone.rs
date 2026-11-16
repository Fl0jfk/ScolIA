//! Zone de reprise : tables de **transit** `reprise.next_*`, séparées des tables métier (schéma distinct,
//! aucune clé étrangère vers le métier, aucune RLS métier), invisibles de tout utilisateur de l'ENT
//! (aucun droit pour `PUBLIC` ni pour le rôle de l'application ; seul le rôle `ent_reprise` y accède) et
//! purgées après la reprise.
//!
//! Une table de zone par table source chargée (`donnees/perimetre_reprise.tsv`), une colonne texte par
//! colonne attendue du Next.js, **sans** les colonnes secrètes (mots de passe, jetons, secrets de double
//! authentification : jamais chargés). Le DDL est **généré** ici ; le fichier de migration
//! `migrations/2566_ms4_zone_reprise.sql` est la sortie de [`ddl`] (un test vérifie qu'ils concordent).

use crate::perimetre::Perimetre;
use crate::schema::{ListeBlanche, Regle, SchemaAttendu};

/// Colonnes de transit, communes à toutes les tables de zone.
pub const COLONNES_TRANSIT: &[&str] = &[
    "_execution_id",
    "_etablissement_cible",
    "_source_ligne",
    "_cle_source",
    "_empreinte",
    "_objet",
    "_disposition",
];

#[derive(Debug, Clone)]
pub struct TableZone {
    /// Table source du Next.js.
    pub source: String,
    /// Nom qualifié de la table de zone (`reprise.next_<source>`).
    pub nom: String,
    /// Colonnes de données (source, sans les secrets), dans l'ordre du schéma attendu.
    pub colonnes: Vec<String>,
    /// Table clé-valeur : porte en plus `_type_feuille`.
    pub attr: bool,
}

/// Tables de zone du dump, dans l'ordre alphabétique des tables sources.
pub fn tables_zone() -> Vec<TableZone> {
    let (s, p, b) = (
        SchemaAttendu::embarque(),
        Perimetre::embarque(),
        ListeBlanche::embarquee(),
    );
    let mut out = Vec::new();
    for (t, cols) in &s.tables {
        if !p.table(t).is_some_and(|e| e.disposition.chargee()) {
            continue;
        }
        let colonnes: Vec<String> = cols
            .iter()
            .filter(|c| b.regle(t, &c.nom) != Some(Regle::Effacer))
            .map(|c| c.nom.clone())
            .collect();
        let attr = t.ends_with("_attr")
            && colonnes.iter().any(|c| c == "path")
            && colonnes.iter().any(|c| c == "value");
        out.push(TableZone {
            source: t.clone(),
            nom: format!("reprise.next_{t}"),
            colonnes,
            attr,
        });
    }
    out
}

/// Tables de zone des JSON S3 (typées), avec leurs colonnes de données.
pub const TABLES_ZONE_S3: &[(&str, &[&str])] = &[
    (
        "reprise.next_s3_objet",
        &["cle", "taille", "sha256", "nature"],
    ),
    (
        "reprise.next_s3_rentree_depot",
        &[
            "reference",
            "sender_email",
            "student_name",
            "file_name",
            "content_type",
            "taille",
            "file_key",
            "item_id",
            "item_title",
            "establishment_id",
            "establishment_label",
            "created_at",
            "expires_at",
        ],
    ),
    (
        "reprise.next_s3_rentree_destinataire",
        &["reference", "rang", "email"],
    ),
    (
        "reprise.next_s3_demande_attente",
        &[
            "reference",
            "first_name",
            "last_name",
            "email",
            "phone",
            "subject",
            "description",
            "created_at",
            "expires_at",
            "parent_source",
            "parent_matched",
        ],
    ),
    (
        "reprise.next_s3_demande_piece",
        &[
            "reference",
            "rang",
            "piece_id",
            "file_name",
            "content_type",
            "taille",
            "uploaded_at",
            "cle_objet",
        ],
    ),
    (
        "reprise.next_s3_demande_enfant",
        &["reference", "rang", "ine", "nom", "prenom", "classe"],
    ),
];

/// Toutes les tables de zone (dump + S3), pour la purge et les contrôles.
pub fn noms_tables_zone() -> Vec<String> {
    let mut v: Vec<String> = tables_zone().into_iter().map(|t| t.nom).collect();
    v.extend(TABLES_ZONE_S3.iter().map(|(n, _)| (*n).to_string()));
    v
}

const ENTETE: &str = "\
-- Socle 28, lot R2 (MS4 c) — migration 2566 : zone de reprise `reprise.*` (tables de transit).
-- GÉNÉRÉ par `ent-reprise zone-ddl` (crates/ent-reprise/src/zone.rs) : ne pas modifier à la main, un test
-- vérifie que ce fichier est identique à la sortie du générateur.
--
-- Zone de TRANSIT : schéma distinct des tables métier, aucune clé étrangère vers le métier, aucune
-- politique RLS d'établissement (ces tables ne sont lues que par l'outil de reprise). Invisible de tout
-- utilisateur de l'ENT : aucun droit pour PUBLIC ni pour le rôle de l'application, seul `ent_reprise`
-- (NOLOGIN, rôle de l'outil) y accède. Volatile : vidée après la reprise (MSE-179), jamais exportée,
-- jamais restaurée. Colonnes de données = texte brut, une par champ attendu du Next.js, SANS les colonnes
-- secrètes (jamais chargées). Les colonnes JSON du Next.js y sont du texte brut : instantané d'audit
-- (exception D-036 déclarée à docs/socles/MIGRATIONS.md), détruit avec la zone.

CREATE SCHEMA IF NOT EXISTS reprise;
REVOKE ALL ON SCHEMA reprise FROM PUBLIC;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ent_reprise') THEN
        BEGIN
            CREATE ROLE ent_reprise NOLOGIN;
        EXCEPTION WHEN duplicate_object OR insufficient_privilege THEN
            RAISE NOTICE 'rôle ent_reprise non créé (droits insuffisants ou déjà créé en parallèle)';
        END;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ent_reprise') THEN
        GRANT USAGE ON SCHEMA reprise TO ent_reprise;
        ALTER DEFAULT PRIVILEGES IN SCHEMA reprise GRANT ALL ON TABLES TO ent_reprise;
    END IF;
END
$$;

";

fn transit(attr: bool) -> String {
    let mut s = String::new();
    s.push_str("    _execution_id UUID NOT NULL,\n");
    s.push_str("    _etablissement_cible UUID NOT NULL,\n");
    s.push_str("    _source_ligne BIGINT NOT NULL CHECK (_source_ligne >= 1),\n");
    s.push_str("    _cle_source TEXT NOT NULL,\n");
    s.push_str("    _empreinte TEXT NOT NULL CHECK (length(_empreinte) = 64),\n");
    s.push_str("    _objet TEXT NOT NULL,\n");
    s.push_str("    _disposition TEXT NOT NULL CHECK (_disposition IN ('reprise', 'rapprochement', 'archive')),\n");
    if attr {
        s.push_str("    _type_feuille TEXT NOT NULL CHECK (_type_feuille IN ('s', 'n', 'b', 'j', 'null', 'undef')),\n");
    }
    s
}

/// Colonnes de transit d'une table de zone du dump.
pub fn colonnes_transit(attr: bool) -> Vec<&'static str> {
    let mut v = COLONNES_TRANSIT.to_vec();
    if attr {
        v.push("_type_feuille");
    }
    v
}

/// Contenu intégral de la migration 2566.
pub fn ddl() -> String {
    let mut s = String::from(ENTETE);
    for t in tables_zone() {
        s.push_str(&format!("CREATE TABLE IF NOT EXISTS {} (\n", t.nom));
        s.push_str(&transit(t.attr));
        for c in &t.colonnes {
            s.push_str(&format!("    \"{c}\" TEXT,\n"));
        }
        s.push_str("    PRIMARY KEY (_execution_id, _cle_source)\n);\n");
    }
    s.push_str(DDL_S3);
    s
}

const DDL_S3: &str = "
-- JSON stockés dans S3 (T0 §2.4), décodés strictement puis rangés en colonnes typées. La référence est
-- l'empreinte de la clé de l'objet (la clé contient un jeton d'accès : jamais conservée).
CREATE TABLE IF NOT EXISTS reprise.next_s3_objet (
    _execution_id UUID NOT NULL,
    _etablissement_cible UUID NOT NULL,
    cle TEXT NOT NULL,
    taille BIGINT NOT NULL CHECK (taille >= 0),
    sha256 TEXT,
    nature TEXT NOT NULL,
    PRIMARY KEY (_execution_id, cle)
);
CREATE TABLE IF NOT EXISTS reprise.next_s3_rentree_depot (
    _execution_id UUID NOT NULL,
    _etablissement_cible UUID NOT NULL,
    reference TEXT NOT NULL,
    sender_email TEXT NOT NULL,
    student_name TEXT,
    file_name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    taille BIGINT NOT NULL CHECK (taille >= 0),
    file_key TEXT NOT NULL,
    item_id TEXT NOT NULL,
    item_title TEXT NOT NULL,
    establishment_id TEXT NOT NULL,
    establishment_label TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (_execution_id, reference)
);
CREATE TABLE IF NOT EXISTS reprise.next_s3_rentree_destinataire (
    _execution_id UUID NOT NULL,
    _etablissement_cible UUID NOT NULL,
    reference TEXT NOT NULL,
    rang INTEGER NOT NULL CHECK (rang >= 0),
    email TEXT NOT NULL,
    PRIMARY KEY (_execution_id, reference, rang)
);
CREATE TABLE IF NOT EXISTS reprise.next_s3_demande_attente (
    _execution_id UUID NOT NULL,
    _etablissement_cible UUID NOT NULL,
    reference TEXT NOT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    email TEXT NOT NULL,
    phone TEXT NOT NULL,
    subject TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    parent_source TEXT,
    parent_matched BOOLEAN,
    PRIMARY KEY (_execution_id, reference)
);
CREATE TABLE IF NOT EXISTS reprise.next_s3_demande_piece (
    _execution_id UUID NOT NULL,
    _etablissement_cible UUID NOT NULL,
    reference TEXT NOT NULL,
    rang INTEGER NOT NULL CHECK (rang >= 0),
    piece_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    taille BIGINT NOT NULL CHECK (taille >= 0),
    uploaded_at TIMESTAMPTZ NOT NULL,
    cle_objet TEXT NOT NULL,
    PRIMARY KEY (_execution_id, reference, rang)
);
CREATE TABLE IF NOT EXISTS reprise.next_s3_demande_enfant (
    _execution_id UUID NOT NULL,
    _etablissement_cible UUID NOT NULL,
    reference TEXT NOT NULL,
    rang INTEGER NOT NULL CHECK (rang >= 0),
    ine TEXT NOT NULL,
    nom TEXT NOT NULL,
    prenom TEXT NOT NULL,
    classe TEXT,
    PRIMARY KEY (_execution_id, reference, rang)
);
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_de_tables_dans_la_limite_postgres() {
        for t in noms_tables_zone() {
            let court = t.strip_prefix("reprise.").unwrap();
            assert!(court.len() <= 63, "{t}");
        }
    }

    #[test]
    fn aucune_colonne_secrete_en_zone() {
        let b = ListeBlanche::embarquee();
        for t in tables_zone() {
            for c in &t.colonnes {
                assert_ne!(
                    b.regle(&t.source, c),
                    Some(Regle::Effacer),
                    "{}.{c}",
                    t.source
                );
            }
        }
        let account = tables_zone()
            .into_iter()
            .find(|t| t.source == "account")
            .unwrap();
        assert!(!account
            .colonnes
            .iter()
            .any(|c| c == "password" || c == "access_token"));
    }

    #[test]
    fn la_zone_ne_charge_ni_sante_ni_ignorees() {
        let noms: Vec<String> = tables_zone().into_iter().map(|t| t.source).collect();
        for t in crate::schema::TABLES_SANTE {
            assert!(!noms.contains(&(*t).to_string()));
        }
        for t in [
            "session",
            "verification",
            "two_factor",
            "passkey",
            "fd_token",
            "stage_token",
        ] {
            assert!(!noms.contains(&t.to_string()), "{t}");
        }
    }

    #[test]
    fn le_fichier_de_migration_est_la_sortie_du_generateur() {
        let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../migrations/2566_ms4_zone_reprise.sql");
        let fichier = std::fs::read_to_string(&chemin).expect("migration 2566");
        assert_eq!(
            fichier,
            ddl(),
            "régénérer : ent-reprise zone-ddl --sortie migrations/2566_ms4_zone_reprise.sql"
        );
    }

    #[test]
    fn ddl_sans_json_ni_cle_etrangere_vers_le_metier() {
        let d: String = ddl()
            .lines()
            .filter(|l| !l.starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        assert!(!d.contains("jsonb") && !d.contains(" json "));
        assert!(!d.contains("references"));
        assert!(!d.contains("row level security"));
    }
}
