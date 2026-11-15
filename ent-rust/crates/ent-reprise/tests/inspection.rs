//! Tests d'intégration de `ent-reprise` (lot R0) : lecture des dumps (texte et répertoire, fixtures
//! produites par un vrai `pg_dump` 16), inspection du jeu fictif, écarts de schéma, empreintes, refus
//! de toute connexion, absence de dépendance réseau ou base de données.
#![allow(unused_imports)]

mod commun;

use commun::*;
use ent_reprise::dump::{self, Evt};
use ent_reprise::empreintes::Verdict;
use ent_reprise::inspection::{inspecter, Options};
use ent_reprise::jeu_fictif::{self, SENTINELLES};
use ent_reprise::schema::{
    ListeBlanche, SchemaAttendu, TABLES_DEPUIS_T0, TABLES_INVENTAIRE_T0, TABLES_SANTE,
};
use std::path::{Path, PathBuf};

// ---- lecture des dumps réels ------------------------------------------------------------------

#[test]
fn texte_et_repertoire_non_compresse_donnent_les_memes_donnees() {
    let f = fixtures();
    let texte = lire_tables(&f.join("plain.sql"));
    let brut = lire_tables(&f.join("dir_brut"));
    assert_eq!(texte, brut);
    let (cols, lignes) = &texte["etablissement"];
    assert_eq!(
        cols,
        &["id", "slug", "name", "data_bucket", "created_at", "tags"]
    );
    assert_eq!(lignes.len(), 2);
    // NULL, tabulation, antislash et saut de ligne décodés.
    assert_eq!(lignes[0][3], None);
    assert_eq!(
        lignes[1][2].as_deref(),
        Some("Ligne avec\ttabulation et \\ antislash")
    );
    assert_eq!(lignes[1][3].as_deref(), Some("ligne1\nligne2"));
    // Table à nom réservé et colonne `by`.
    let (cols, lignes) = &texte["user"];
    assert_eq!(cols, &["id", "name", "by", "montant", "options"]);
    assert_eq!(lignes[1][1], None);
    assert_eq!(lignes[1][2].as_deref(), Some(""));
}

#[test]
fn repertoire_donne_aussi_les_definitions() {
    let mut c = dump::CollecteTables::default();
    dump::lire(
        &dump::ouvrir(&fixtures().join("dir_brut")).unwrap(),
        &mut |e| {
            if let Evt::Ddl(l) = e {
                c.ligne(&l);
            }
            Ok(())
        },
    )
    .unwrap();
    let noms: Vec<&str> = c.tables.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        noms.contains(&"etablissement") && noms.contains(&"user"),
        "{noms:?}"
    );
    let (_, cols) = c.tables.iter().find(|(t, _)| t == "user").unwrap();
    assert_eq!(
        cols.iter().map(|c| c.nom.as_str()).collect::<Vec<_>>(),
        ["id", "name", "by", "montant", "options"]
    );
}

#[test]
fn repertoire_compresse_refuse_avec_le_moyen_de_le_convertir() {
    let e = dump::lire(
        &dump::ouvrir(&fixtures().join("dir_gz")).unwrap(),
        &mut |_| Ok(()),
    )
    .unwrap_err();
    assert!(e.0.contains("-Z 0") && e.0.contains("pg_restore"), "{e}");
}

#[test]
fn version_d_archive_inconnue_refusee_avec_un_message_clair() {
    let d = tmp("toc");
    std::fs::create_dir_all(&d).unwrap();
    let mut toc = std::fs::read(fixtures().join("dir_brut/toc.dat")).unwrap();
    toc[6] = 99; // version 1.99
    std::fs::write(d.join("toc.dat"), toc).unwrap();
    let e = dump::lire_toc(&d).unwrap_err();
    assert!(e.0.contains("pg_restore"), "{e}");
    std::fs::write(d.join("toc.dat"), b"pas un sommaire").unwrap();
    assert!(dump::lire_toc(&d).is_err());
    let _ = std::fs::remove_dir_all(d);
}

// ---- R0 : inspecter ---------------------------------------------------------------------------

#[test]
fn jeu_fictif_est_complet_et_conforme() {
    let d = jeu("fictif", 4);
    let r = inspecter(&Options {
        dump: d.join("dump.sql"),
        objets_s3: Some(d.join("objets_s3.tsv")),
        sommes: Some(d.join("SHA256SUMS")),
        empreinte_dump: None,
    })
    .unwrap();
    assert!(r.conforme(), "{:?}", r.ecarts_schema);
    assert_eq!(
        r.tables_attendues,
        TABLES_INVENTAIRE_T0 + TABLES_DEPUIS_T0.len()
    );
    assert!(r.lignes_total > 500);
    assert!(r.empreintes.iter().all(|v| v.verdict == Verdict::Conforme) && r.empreintes.len() == 2);
    // Collections par nom.
    assert!(
        r.collections.contains_key("internat__rooms")
            && r.collections.contains_key("rh__onboarding")
    );
    assert!(
        r.collections["internat__rooms"].attributs > 0
            && r.collections["internat__rooms"].enregistrements > 0
    );
    // Alertes : table « morte » non vide, santé non vide, objets secrets.
    assert!(r
        .tables_mortes_non_vides
        .contains(&"annee_bascule".to_string()));
    assert_eq!(r.tables_sante_non_vides.len(), TABLES_SANTE.len());
    let s3 = r.s3.unwrap();
    assert_eq!((s3.objets, s3.objets_secrets_a_ne_pas_reprendre), (26, 2));
    assert!(s3.par_prefixe.contains_key("documents") && s3.par_prefixe.contains_key("tenants"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn le_rapport_d_inspection_ne_contient_aucune_valeur() {
    let d = jeu("rapport", 4);
    let o = lancer(&[
        "inspecter",
        "--dump",
        d.join("dump.sql").to_str().unwrap(),
        "--objets-s3",
        d.join("objets_s3.tsv").to_str().unwrap(),
        "--sommes",
        d.join("SHA256SUMS").to_str().unwrap(),
        "--json",
        d.join("rapport.json").to_str().unwrap(),
    ]);
    assert_eq!(o.status.code(), Some(0), "{}", sortie_texte(&o));
    let texte = sortie_texte(&o) + &std::fs::read_to_string(d.join("rapport.json")).unwrap();
    for s in SENTINELLES {
        assert!(
            !texte.to_lowercase().contains(&s.to_lowercase()),
            "valeur « {s} » dans le rapport"
        );
    }
    assert!(texte.contains("CONFORME"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn ecart_de_schema_detecte_et_code_de_sortie_2() {
    let d = jeu("ecart", 3);
    let sql = std::fs::read_to_string(d.join("dump.sql")).unwrap();
    // Table manquante, colonne en trop, type changé.
    let sans_foyer: String = {
        let a = sql.find("CREATE TABLE public.foyer (").unwrap();
        let b = a + sql[a..].find(");\n").unwrap() + 3;
        let sans_create = format!("{}{}", &sql[..a], &sql[b..]);
        let c = sans_create.find("COPY public.foyer (").unwrap();
        let d2 = c + sans_create[c..].find("\\.\n").unwrap() + 3;
        format!("{}{}", &sans_create[..c], &sans_create[d2..])
    };
    let modifie = sans_foyer
        .replacen(
            "CREATE TABLE public.eleve (",
            "CREATE TABLE public.eleve (\n    colonne_inventee text,",
            1,
        )
        .replacen("    ine text,", "    ine integer,", 1);
    let p = d.join("ecart.sql");
    std::fs::write(&p, modifie).unwrap();
    let o = lancer(&["inspecter", "--dump", p.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(2));
    let t = sortie_texte(&o);
    assert!(
        t.contains("ÉCART DE SCHÉMA") && t.contains("tables manquantes") && t.contains("- foyer"),
        "{t}"
    );
    assert!(
        t.contains("eleve.colonne_inventee") && t.contains("eleve.ine"),
        "{t}"
    );
    assert!(t.contains("NON CONFORME"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn empreinte_modifiee_detectee() {
    let d = jeu("empreinte", 3);
    let mut sql = std::fs::read(d.join("dump.sql")).unwrap();
    sql.extend_from_slice(b"-- modifie apres coup\n");
    std::fs::write(d.join("dump.sql"), sql).unwrap();
    let o = lancer(&[
        "inspecter",
        "--dump",
        d.join("dump.sql").to_str().unwrap(),
        "--sommes",
        d.join("SHA256SUMS").to_str().unwrap(),
    ]);
    assert_eq!(o.status.code(), Some(2), "{}", sortie_texte(&o));
    assert!(sortie_texte(&o).contains("DIFFÉRENTE"));
    // Empreinte attendue fausse.
    let o = lancer(&[
        "inspecter",
        "--dump",
        d.join("dump.sql").to_str().unwrap(),
        "--empreinte",
        &"0".repeat(64),
    ]);
    assert_eq!(o.status.code(), Some(2));
    // Empreinte attendue mal formée : erreur d'usage.
    let o = lancer(&[
        "inspecter",
        "--dump",
        d.join("dump.sql").to_str().unwrap(),
        "--empreinte",
        "abc",
    ]);
    assert_eq!(o.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn le_dump_au_format_repertoire_s_inspecte_aussi() {
    let r = inspecter(&Options {
        dump: fixtures().join("dir_brut"),
        objets_s3: None,
        sommes: None,
        empreinte_dump: None,
    })
    .unwrap();
    assert_eq!(r.type_de_dump, "répertoire");
    assert_eq!(r.tables["etablissement"].lignes, 2);
    assert!(!r.ecarts_schema.est_vide(), "la fixture n'a que 2 tables");
}

// ---- garde : jamais de connexion --------------------------------------------------------------

#[test]
fn toute_adresse_de_connexion_est_refusee() {
    let d = jeu("garde", 2);
    let dump = d.join("dump.sql");
    for url in [
        "postgres://admin:motdepasse-secret@10.0.0.1:5432/scolia",
        "postgresql://127.0.0.1/db",
        "host=localhost dbname=scolia user=x",
        "s3://bucket/x",
        "https://exemple.test/dump.sql",
    ] {
        for args in [
            vec!["inspecter", "--dump", url],
            vec![
                "inspecter",
                "--dump",
                dump.to_str().unwrap(),
                "--objets-s3",
                url,
            ],
            vec![
                "inspecter",
                "--dump",
                dump.to_str().unwrap(),
                "--sommes",
                url,
            ],
            vec!["jeu-fictif", "--sortie", url],
        ] {
            let o = lancer(&args);
            assert_eq!(o.status.code(), Some(1), "{args:?}");
            let t = sortie_texte(&o);
            assert!(
                t.contains("refusé") || t.contains("adresse de connexion"),
                "{t}"
            );
            assert!(
                !t.contains("motdepasse-secret") && !t.contains("admin"),
                "la valeur ne doit pas être répétée : {t}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn aucune_dependance_reseau_ni_base_de_donnees() {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR"));
    let toml = std::fs::read_to_string(racine.join("Cargo.toml")).unwrap();
    for interdit in [
        "sqlx",
        "tokio",
        "reqwest",
        "hyper",
        "postgres",
        "tokio-postgres",
        "native-tls",
        "rustls",
        "aws-",
        "rusoto",
        "ureq",
        "curl",
        "axum",
        "ent-db",
        "ent-api",
        "ent-object",
    ] {
        assert!(
            !toml.contains(interdit),
            "dépendance interdite : {interdit}"
        );
    }
    // Aucun usage du réseau dans le code.
    fn parcourir(d: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                parcourir(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut fichiers = vec![];
    parcourir(&racine.join("src"), &mut fichiers);
    for f in fichiers {
        let s = std::fs::read_to_string(&f).unwrap();
        for interdit in [
            "std::net",
            "TcpStream",
            "UdpSocket",
            "std::env::var(\"DATABASE",
            "ENT_DATABASE_URL",
            "PGPASSWORD",
        ] {
            // Les tests de la garde citent des URL, pas ces symboles.
            assert!(!s.contains(interdit), "{} utilise {interdit}", f.display());
        }
    }
}
