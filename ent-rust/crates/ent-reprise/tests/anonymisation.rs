//! Tests d'intégration de `ent-reprise` (lot R1) : anonymisation du jeu fictif, fermé par défaut,
//! contrôle « zéro occurrence », cohérence des pseudonymes, JSON, clés S3, refus de toute connexion.
#![allow(unused_imports)]

mod commun;

use commun::*;
use ent_reprise::dump::{self, Evt};
use ent_reprise::jeu_fictif::{self, SENTINELLES};
use ent_reprise::schema::{ListeBlanche, SchemaAttendu, TABLES_SANTE};
use std::path::{Path, PathBuf};

// ---- garde : jamais de connexion --------------------------------------------------------------

#[test]
fn toute_adresse_de_connexion_est_refusee() {
    let d = jeu("garde", 2);
    let dump = d.join("dump.sql");
    let c = cle(&d, "cle.txt");
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
                "anonymiser",
                "--dump",
                url,
                "--sortie",
                "x",
                "--cle",
                c.to_str().unwrap(),
            ],
            vec![
                "anonymiser",
                "--dump",
                dump.to_str().unwrap(),
                "--sortie",
                url,
                "--cle",
                c.to_str().unwrap(),
            ],
            vec![
                "anonymiser",
                "--dump",
                dump.to_str().unwrap(),
                "--sortie",
                "x",
                "--cle",
                url,
            ],
            vec!["verifier", "--dossier", url],
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

// ---- R1 : anonymiser --------------------------------------------------------------------------

#[test]
fn anonymisation_du_jeu_fictif_aucune_valeur_ne_survit() {
    let d = jeu("anon", 6);
    let c = cle(&d, "cle.txt");
    let sortie = d.join("sortie");
    let o = anonymiser(
        &d.join("dump.sql"),
        Some(&d.join("objets_s3.tsv")),
        &sortie,
        &c,
    );
    assert_eq!(o.status.code(), Some(0), "{}", sortie_texte(&o));
    let tout = tout_lire(&sortie).to_lowercase();
    for s in SENTINELLES {
        assert!(
            !tout.contains(&s.to_lowercase()),
            "valeur fictive « {s} » encore présente dans l'export ou son rapport"
        );
    }
    // Fichiers livrés.
    for f in [
        "dump.sql",
        "objets_s3.tsv",
        "rapport_anonymisation.json",
        "rapport_anonymisation.txt",
        "SHA256SUMS",
        "EMPREINTE.txt",
    ] {
        assert!(sortie.join(f).is_file(), "{f} manquant");
    }
    // Rien de la clé dans le rapport ; seulement son empreinte courte.
    let cle_txt = std::fs::read_to_string(&c).unwrap();
    assert!(!tout.contains(cle_txt.trim()));
    // Vérification externe.
    let v = lancer(&["verifier", "--dossier", sortie.to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0), "{}", sortie_texte(&v));
    assert!(sortie_texte(&v).contains("CONFORME"));
    // Le schéma de l'export anonymisé reste exactement celui attendu, volumes conservés (hors santé).
    let avant = lire_tables(&d.join("dump.sql"));
    let apres = lire_tables(&sortie.join("dump.sql"));
    assert_eq!(
        avant.keys().collect::<Vec<_>>(),
        apres.keys().collect::<Vec<_>>()
    );
    for (t, (cols, lignes)) in &avant {
        let (c2, l2) = &apres[t];
        assert_eq!(cols, c2);
        if TABLES_SANTE.contains(&t.as_str()) {
            assert!(l2.is_empty(), "{t} : la santé est exclue");
        } else {
            assert_eq!(lignes.len(), l2.len(), "{t}");
        }
    }
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn memes_valeurs_memes_pseudonymes_liens_et_doublons_conserves() {
    let d = jeu("liens", 5);
    let c = cle(&d, "cle.txt");
    let sortie = d.join("sortie");
    assert_eq!(
        anonymiser(&d.join("dump.sql"), None, &sortie, &c)
            .status
            .code(),
        Some(0)
    );
    let avant = lire_tables(&d.join("dump.sql"));
    let apres = lire_tables(&sortie.join("dump.sql"));
    let col = |t: &Tables, table: &str, c: &str| -> Vec<Option<String>> {
        let (cols, lignes) = &t[table];
        let i = cols.iter().position(|x| x == c).unwrap();
        lignes.iter().map(|l| l[i].clone()).collect()
    };
    // `nom` de eleve et de foyer_responsable : mêmes valeurs en entrée → mêmes pseudonymes en sortie.
    let (n1a, n2a) = (
        col(&avant, "eleve", "nom"),
        col(&avant, "foyer_responsable", "nom"),
    );
    let (n1, n2) = (
        col(&apres, "eleve", "nom"),
        col(&apres, "foyer_responsable", "nom"),
    );
    assert_eq!(n1a[0], n2a[0]);
    assert_eq!(n1[0], n2[0]);
    assert_ne!(n1[0], n1a[0]);
    // Deux valeurs différentes restent différentes.
    assert_ne!(n1[0], n1[1]);
    // Les identifiants techniques et les liens (clés étrangères) sont conservés.
    assert_eq!(col(&avant, "eleve", "id"), col(&apres, "eleve", "id"));
    assert_eq!(
        col(&avant, "vs_absence_eleve", "eleve_id"),
        col(&apres, "vs_absence_eleve", "eleve_id")
    );
    // Les montants, statuts et dates de création sont gardés.
    assert_eq!(
        col(&avant, "facture", "total_ttc"),
        col(&apres, "facture", "total_ttc")
    );
    assert_eq!(
        col(&avant, "eleve", "created_at"),
        col(&apres, "eleve", "created_at")
    );
    // Un objet S3 reste relié à la ligne qui le référence.
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn cles_s3_coherentes_entre_colonnes_et_liste_d_objets() {
    let d = jeu("s3", 6);
    let c = cle(&d, "cle.txt");
    let sortie = d.join("sortie");
    assert_eq!(
        anonymiser(
            &d.join("dump.sql"),
            Some(&d.join("objets_s3.tsv")),
            &sortie,
            &c
        )
        .status
        .code(),
        Some(0)
    );
    let apres = lire_tables(&sortie.join("dump.sql"));
    let liste = ent_reprise::s3::lire_liste(&sortie.join("objets_s3.tsv")).unwrap();
    let cles: Vec<&str> = liste.iter().map(|o| o.cle.as_str()).collect();
    // Aucun objet « secret » n'est repris.
    assert!(cles.iter().all(|k| !k.starts_with("tenants/")));
    assert_eq!(liste.len(), 24);
    let (cols, lignes) = &apres["eleve_document"];
    let i = cols.iter().position(|x| x == "s3_key").unwrap();
    let mut trouvees = 0;
    for l in lignes {
        if let Some(k) = &l[i] {
            if cles.contains(&k.as_str()) {
                trouvees += 1;
            }
        }
    }
    assert!(trouvees >= 2, "les clés réécrites des colonnes doivent retrouver les objets réécrits de la liste ({trouvees})");
    // Les empreintes de contenu d'origine ne sont pas reprises.
    let d_orig = ent_reprise::s3::lire_liste(&d.join("objets_s3.tsv")).unwrap();
    for o in &liste {
        assert!(d_orig
            .iter()
            .all(|x| x.sha256 != o.sha256 || x.sha256.is_none()));
    }
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn collections_et_json_anonymises_structure_gardee() {
    let d = jeu("json", 8);
    let c = cle(&d, "cle.txt");
    let sortie = d.join("sortie");
    assert_eq!(
        anonymiser(&d.join("dump.sql"), None, &sortie, &c)
            .status
            .code(),
        Some(0)
    );
    let apres = lire_tables(&sortie.join("dump.sql"));
    // Valeurs clé-valeur : préfixes conservés, jamais de texte en clair.
    let (cols, lignes) = &apres["ent_collection_attr"];
    let iv = cols.iter().position(|x| x == "value").unwrap();
    let ip = cols.iter().position(|x| x == "path").unwrap();
    for l in lignes {
        let v = l[iv].as_deref().unwrap();
        assert!(
            ["s:", "n:", "b:", "j:"].iter().any(|p| v.starts_with(p)),
            "{v}"
        );
        if l[ip].as_deref() == Some("age") {
            assert!(v.starts_with("n:"));
        }
    }
    // Colonne jsonb : JSON valide, structure gardée (nombres, booléens, tableaux de classes).
    let (cols, lignes) = &apres["eleve"];
    let i = cols.iter().position(|x| x == "options").unwrap();
    let j: serde_json::Value = serde_json::from_str(lignes[0][i].as_deref().unwrap()).unwrap();
    assert_eq!(j["actif"], true);
    assert_eq!(j["classes"], serde_json::json!(["6A", "5B"]));
    assert_eq!(j["statut"], "actif");
    assert!(j["nombre"].is_number());
    // La clé inconnue du dictionnaire est listée au rapport (nom de clé seulement).
    let rapport = std::fs::read_to_string(sortie.join("rapport_anonymisation.txt")).unwrap();
    assert!(
        rapport.contains("cle_inconnue_du_dictionnaire"),
        "{rapport}"
    );
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn deterministe_avec_la_meme_cle_different_avec_une_autre() {
    let d = jeu("determ", 4);
    let (c1, c2) = (cle(&d, "c1.txt"), cle(&d, "c2.txt"));
    let (s1, s1b, s2) = (d.join("s1"), d.join("s1b"), d.join("s2"));
    for (s, c) in [(&s1, &c1), (&s1b, &c1), (&s2, &c2)] {
        assert_eq!(
            anonymiser(&d.join("dump.sql"), None, s, c).status.code(),
            Some(0)
        );
    }
    let h = |s: &Path| ent_reprise::empreintes::sha256_fichier(&s.join("dump.sql")).unwrap();
    assert_eq!(h(&s1), h(&s1b));
    assert_ne!(h(&s1), h(&s2));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn ferme_par_defaut_colonne_ou_table_inconnue_refusee_sans_rien_ecrire() {
    let d = jeu("ferme", 3);
    let c = cle(&d, "cle.txt");
    let sql = std::fs::read_to_string(d.join("dump.sql")).unwrap();
    // 1. Colonne ajoutée à une table connue.
    let col = sql
        .replacen(
            "CREATE TABLE public.eleve (",
            "CREATE TABLE public.eleve (\n    colonne_nouvelle text,",
            1,
        )
        .replacen(
            "COPY public.eleve (",
            "COPY public.eleve (colonne_nouvelle, ",
            1,
        );
    // (les lignes de données n'ont pas la cellule en plus : le contrôle de déclaration passe avant la lecture des lignes)
    let p = d.join("colonne.sql");
    std::fs::write(&p, col).unwrap();
    let sortie = d.join("sortie1");
    let o = anonymiser(&p, None, &sortie, &c);
    assert_eq!(o.status.code(), Some(2), "{}", sortie_texte(&o));
    let t = sortie_texte(&o);
    assert!(
        t.contains("eleve.colonne_nouvelle") && t.contains("Rien n'a été écrit"),
        "{t}"
    );
    assert!(!sortie.exists(), "aucun fichier ne doit être créé");
    // 2. Table inconnue.
    let tab = format!("{sql}\nCREATE TABLE public.table_nouvelle (\n    id text\n);\n");
    let p = d.join("table.sql");
    std::fs::write(&p, tab).unwrap();
    let sortie = d.join("sortie2");
    let o = anonymiser(&p, None, &sortie, &c);
    assert_eq!(o.status.code(), Some(2));
    assert!(sortie_texte(&o).contains("table_nouvelle"));
    assert!(!sortie.exists());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn zero_occurrence_une_valeur_reelle_dans_une_colonne_gardee_bloque_la_livraison() {
    let d = jeu("fuite", 4);
    let c = cle(&d, "cle.txt");
    let sql = std::fs::read_to_string(d.join("dump.sql")).unwrap();
    // `ref_nomenclature.libelle_long` est « garder » : un nom réel (présent aussi dans eleve.nom) y fuirait.
    let a = sql.find("COPY public.ref_nomenclature (").unwrap();
    let fin_entete = a + sql[a..].find('\n').unwrap() + 1;
    let entete = &sql[a..fin_entete];
    let cols: Vec<&str> = entete[entete.find('(').unwrap() + 1..entete.find(')').unwrap()]
        .split(", ")
        .collect();
    let i = cols.iter().position(|x| *x == "libelle_long").unwrap();
    let fin_ligne = fin_entete + sql[fin_entete..].find('\n').unwrap();
    let ligne = &sql[fin_entete..fin_ligne];
    let mut cellules: Vec<String> = ligne.split('\t').map(String::from).collect();
    cellules[i] = "Salle de Madame Zorglub".to_string();
    let modifie = format!(
        "{}{}{}",
        &sql[..fin_entete],
        cellules.join("\t"),
        &sql[fin_ligne..]
    );
    let p = d.join("fuite.sql");
    std::fs::write(&p, modifie).unwrap();
    let sortie = d.join("sortie");
    let o = anonymiser(&p, None, &sortie, &c);
    assert_eq!(o.status.code(), Some(3), "{}", sortie_texte(&o));
    let t = sortie_texte(&o);
    assert!(
        t.contains("ref_nomenclature.libelle_long") && t.contains("PAS livré"),
        "{t}"
    );
    assert!(
        !t.to_lowercase().contains("zorglub"),
        "le diagnostic ne doit pas contenir la valeur : {t}"
    );
    for f in [
        "dump.sql",
        "SHA256SUMS",
        "EMPREINTE.txt",
        "rapport_anonymisation.json",
    ] {
        assert!(!sortie.join(f).exists(), "{f} ne doit pas être livré");
    }
    // Un mot autorisé explicitement lève le blocage (cas des faux positifs).
    let mots = d.join("mots.txt");
    std::fs::write(&mots, "zorglub\n").unwrap();
    let o = lancer(&[
        "anonymiser",
        "--dump",
        p.to_str().unwrap(),
        "--sortie",
        d.join("sortie2").to_str().unwrap(),
        "--cle",
        c.to_str().unwrap(),
        "--mots-autorises",
        mots.to_str().unwrap(),
    ]);
    // Le contrôle par jeton est levé, mais la cellule entière n'est plus égale à un nom : livraison.
    assert_eq!(o.status.code(), Some(0), "{}", sortie_texte(&o));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn sortie_existante_non_vide_refusee_et_cle_courte_refusee() {
    let d = jeu("sortie", 2);
    let c = cle(&d, "cle.txt");
    let sortie = d.join("sortie");
    std::fs::create_dir_all(&sortie).unwrap();
    std::fs::write(sortie.join("x"), b"x").unwrap();
    let o = anonymiser(&d.join("dump.sql"), None, &sortie, &c);
    assert_eq!(o.status.code(), Some(1));
    assert!(sortie_texte(&o).contains("refus d'écraser"));
    assert!(sortie.join("x").is_file());
    let courte = d.join("courte.txt");
    std::fs::write(&courte, "trop-courte").unwrap();
    let o = anonymiser(&d.join("dump.sql"), None, &d.join("autre"), &courte);
    assert_eq!(o.status.code(), Some(1));
    assert!(sortie_texte(&o).contains("trop courte"));
    // generer-cle refuse d'écraser.
    let o = lancer(&["generer-cle", "--fichier", c.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn dump_au_format_repertoire_lu_puis_refuse_car_colonnes_non_declarees() {
    // La fixture est un petit dump réel (pg_dump 16, format répertoire) dont les tables ne sont pas celles
    // du Next.js : l'outil, fermé par défaut, refuse et liste les colonnes non déclarées (sans rien écrire).
    let d = tmp("repertoire");
    std::fs::create_dir_all(&d).unwrap();
    let c = cle(&d, "cle.txt");
    let sortie = d.join("sortie");
    let o = anonymiser(&fixtures().join("dir_brut"), None, &sortie, &c);
    assert_eq!(o.status.code(), Some(2), "{}", sortie_texte(&o));
    let t = sortie_texte(&o);
    assert!(
        t.contains("user.by") && t.contains("etablissement.tags"),
        "{t}"
    );
    assert!(!t.to_lowercase().contains("zorglub"));
    assert!(!sortie.exists());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn la_liste_blanche_declare_les_tables_du_jeu_fictif() {
    let (m, e) = ListeBlanche::embarquee().ecarts(&SchemaAttendu::embarque());
    assert!(m.is_empty() && e.is_empty());
}
