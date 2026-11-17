//! Tests du décodage strict (lot R2) sur l'export fictif au format Next.js : jeu « décodable » (aucun refus
//! hors santé et secrets) et mutations ciblées (une anomalie par test). Aucun accès réseau ni base.

mod commun;

use chrono::{TimeZone, Utc};
use commun::*;
use ent_reprise::decodage::attr::TypeFeuille;
use ent_reprise::decodage::{decoder_export, texte, Decodage, OptionsDecodage};
use ent_reprise::jeu_fictif;
use ent_reprise::perimetre::Disposition;
use ent_reprise::schema::TABLES_SANTE;
use std::path::{Path, PathBuf};

fn maintenant() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 16, 12, 0, 0).unwrap()
}

fn jeu_decodable(nom: &str) -> PathBuf {
    let d = tmp(nom);
    jeu_fictif::generer_decodable(&d, 3).unwrap();
    d
}

fn options(d: &Path) -> OptionsDecodage {
    OptionsDecodage {
        dump: d.join("dump.sql"),
        objets_s3: Some(d.join("objets_s3.tsv")),
        contenus_s3: Some(d.join("contenus_s3")),
        maintenant: maintenant(),
    }
}

fn decoder(d: &Path) -> Decodage {
    decoder_export(&options(d)).unwrap()
}

/// Remplace la cellule `colonne` de la ligne `k` (0-based) du bloc COPY de `table`.
fn modifier(d: &Path, table: &str, k: usize, colonne: &str, valeur: &str) {
    let chemin = d.join("dump.sql");
    let src = std::fs::read_to_string(&chemin).unwrap();
    let mut out = Vec::new();
    let mut idx_col = None;
    let mut ligne_dans_bloc = None;
    let mut fait = false;
    for l in src.lines() {
        if l.starts_with(&format!("COPY public.{table} (")) {
            let cols = l.split_once('(').unwrap().1.split_once(')').unwrap().0;
            idx_col = cols
                .split(", ")
                .position(|c| c.trim_matches('"') == colonne);
            ligne_dans_bloc = Some(0usize);
            out.push(l.to_string());
            continue;
        }
        if l == "\\." {
            ligne_dans_bloc = None;
        }
        if let (Some(n), Some(i)) = (ligne_dans_bloc, idx_col) {
            ligne_dans_bloc = Some(n + 1);
            if n == k {
                let mut cells: Vec<&str> = l.split('\t').collect();
                cells[i] = valeur;
                out.push(cells.join("\t"));
                fait = true;
                continue;
            }
        }
        out.push(l.to_string());
    }
    assert!(fait, "cellule introuvable : {table}[{k}].{colonne}");
    std::fs::write(&chemin, out.join("\n") + "\n").unwrap();
}

fn codes(r: &Decodage, table: &str) -> Vec<String> {
    r.rapport.tables[table]
        .refus_par_code
        .keys()
        .cloned()
        .collect()
}

#[test]
fn jeu_decodable_sans_refus_hors_sante_et_secrets() {
    let d = jeu_decodable("dec-ok");
    let r = decoder(&d);
    println!("{}", texte(&r.rapport));
    assert!(r.rapport.invariants_ok());
    assert!(
        r.rapport.ecarts_schema.is_empty(),
        "{:?}",
        r.rapport.ecarts_schema
    );
    assert!(r.rapport.colonnes_a_cataloguer.is_empty());
    // Les seuls refus : les 7 tables de santé (R-MSE5) et les 2 objets S3 secrets.
    let refus_sante: u64 = TABLES_SANTE
        .iter()
        .map(|t| r.rapport.tables[*t].refusees)
        .sum();
    assert!(refus_sante > 0);
    let autres: Vec<_> = r
        .rapport
        .tables
        .iter()
        .filter(|(t, o)| o.refusees > 0 && !TABLES_SANTE.contains(&t.as_str()))
        .map(|(t, o)| (t.clone(), o.refus_par_code.clone()))
        .collect();
    assert!(autres.is_empty(), "refus inattendus : {autres:?}");
    assert_eq!(r.rapport.s3["Secret"].refusees, 2);
    // Santé : refusée, jamais chargée.
    for t in TABLES_SANTE {
        assert_eq!(
            r.rapport.tables[*t].refus_par_code["sante_hds"],
            r.rapport.tables[*t].lues
        );
        assert!(!r.tables.contains_key(*t));
    }
    // Ignorées : comptées, non chargées.
    assert_eq!(
        r.rapport.tables["session"].ignorees,
        r.rapport.tables["session"].lues
    );
    assert!(!r.tables.contains_key("session"));
    // Collections ventilées par famille.
    assert_eq!(
        r.rapport.collections["internat__rooms"].disposition,
        "reprise"
    );
    assert_eq!(
        r.rapport.collections["pilotage__eleves-index"].disposition,
        "rapprochement"
    );
}

#[test]
fn les_secrets_ne_sont_jamais_charges() {
    let d = jeu_decodable("dec-secret");
    let r = decoder(&d);
    let account = &r.tables["account"];
    for c in ["password", "access_token", "refresh_token", "id_token"] {
        assert!(
            !account.colonnes.contains(&c.to_string()),
            "{c} chargé en zone"
        );
    }
    assert!(r.rapport.secrets_ecartes["account.password"] > 0);
    // Aucune valeur « secret-… » du jeu fictif dans les lignes chargées.
    for t in r.tables.values() {
        for l in &t.lignes {
            assert!(l
                .cellules
                .iter()
                .flatten()
                .all(|c| !c.starts_with("secret-")));
        }
    }
}

#[test]
fn decodage_deterministe_et_rapport_sans_donnee() {
    let d = jeu_decodable("dec-det");
    let a = decoder(&d);
    let b = decoder(&d);
    assert_eq!(
        serde_json::to_string(&a.rapport).unwrap(),
        serde_json::to_string(&b.rapport).unwrap()
    );
    let (la, lb) = (&a.tables["travel"].lignes, &b.tables["travel"].lignes);
    assert_eq!(
        la.iter().map(|l| &l.empreinte).collect::<Vec<_>>(),
        lb.iter().map(|l| &l.empreinte).collect::<Vec<_>>()
    );
    // Aucune sentinelle ni e-mail du jeu dans le rapport.
    let rapport = serde_json::to_string(&a.rapport).unwrap() + &texte(&a.rapport);
    for s in jeu_fictif::SENTINELLES {
        assert!(!rapport.contains(s), "{s} dans le rapport");
    }
    assert!(!rapport.contains("example.test"));
}

#[test]
fn json_type_inattendu_cle_inconnue_et_enumeration() {
    let d = jeu_decodable("dec-json");
    modifier(&d, "eleve", 0, "options", "{\"a\":1}");
    modifier(
        &d,
        "fd_campagne",
        0,
        "appel_config",
        "{\"enabled\":true,\"inconnue\":1}",
    );
    modifier(&d, "fd_fiche", 1, "appel", "{\"decision\":\"autre\"}");
    modifier(&d, "personnel", 0, "payload", "{\"conges\":1}");
    let r = decoder(&d);
    assert_eq!(codes(&r, "eleve"), ["json_type"]);
    assert_eq!(codes(&r, "fd_campagne"), ["json_cle_inconnue"]);
    assert_eq!(codes(&r, "fd_fiche"), ["json_enum"]);
    assert_eq!(codes(&r, "personnel"), ["json_cle_inconnue"]);
    assert_eq!(r.rapport.colonnes_a_cataloguer, ["personnel.payload"]);
    // Les autres lignes de la table restent comprises ; la ligne refusée n'est pas chargée.
    assert_eq!(r.rapport.tables["eleve"].comprises, 2);
    assert_eq!(r.tables["eleve"].lignes.len(), 2);
    assert!(r.rapport.invariants_ok());
    // Le détail nomme la colonne et le chemin, jamais une valeur.
    let det = r
        .rapport
        .refus
        .iter()
        .find(|x| x.objet == "fd_campagne")
        .unwrap();
    assert_eq!(det.colonne.as_deref(), Some("appel_config"));
    assert!(det.message.contains("inconnue"));
}

#[test]
fn valeurs_hors_domaine() {
    let d = jeu_decodable("dec-domaine");
    modifier(&d, "absence", 0, "etablissement_id", "pas-un-uuid");
    modifier(
        &d,
        "calendrier_scolaire",
        1,
        "created_at",
        "2026-02-30 08:00:00+00",
    );
    modifier(
        &d,
        "annee_scolaire",
        2,
        "created_at",
        "3026-01-01 00:00:00+00",
    );
    let r = decoder(&d);
    assert!(codes(&r, "absence").contains(&"uuid_invalide".to_string()));
    assert!(codes(&r, "calendrier_scolaire").contains(&"instant_invalide".to_string()));
    assert!(codes(&r, "annee_scolaire").contains(&"instant_hors_domaine".to_string()));
    assert!(r.rapport.invariants_ok());
}

#[test]
fn ecart_de_schema_colonne_inconnue_et_table_inconnue() {
    let d = jeu_decodable("dec-ecart");
    let chemin = d.join("dump.sql");
    let src = std::fs::read_to_string(&chemin).unwrap();
    // Colonne inconnue dans l'en-tête COPY de `tarif` + une table que le schéma ne connaît pas.
    let src = src.replacen("COPY public.tarif (id,", "COPY public.tarif (id_x,", 1);
    let src = format!("{src}\nCOPY public.table_inconnue (a) FROM stdin;\n1\n\\.\n");
    std::fs::write(&chemin, src).unwrap();
    let r = decoder(&d);
    let e = r.rapport.ecarts_schema.join("|");
    assert!(e.contains("colonne inconnue : tarif.id_x"), "{e}");
    assert!(e.contains("colonne manquante : tarif.id"), "{e}");
    assert!(e.contains("table inconnue : table_inconnue"), "{e}");
    assert_eq!(codes(&r, "tarif"), ["schema_ecart"]);
    assert_eq!(r.rapport.tables["tarif"].comprises, 0);
    assert!(!r.rapport.conforme());
}

#[test]
fn table_requise_absente() {
    let d = jeu_decodable("dec-absente");
    let chemin = d.join("dump.sql");
    let src = std::fs::read_to_string(&chemin).unwrap();
    // On retire le CREATE TABLE et le COPY de `tarif`.
    let a = src.find("CREATE TABLE public.tarif (").unwrap();
    let b = src[a..].find(");\n").unwrap() + a + 3;
    let src = format!("{}{}", &src[..a], &src[b..]);
    let a = src.find("COPY public.tarif (").unwrap();
    let b = src[a..].find("\\.\n").unwrap() + a + 3;
    std::fs::write(&chemin, format!("{}{}", &src[..a], &src[b..])).unwrap();
    let r = decoder(&d);
    assert!(r
        .rapport
        .ecarts_schema
        .contains(&"table manquante : tarif".to_string()));
}

#[test]
fn cle_dupliquee_et_cle_absente() {
    let d = jeu_decodable("dec-cle");
    let src = std::fs::read_to_string(d.join("dump.sql")).unwrap();
    let id0 = {
        let a = src.find("COPY public.tarif (").unwrap();
        let ligne = src[a..].lines().nth(1).unwrap();
        ligne.split('\t').next().unwrap().to_string()
    };
    modifier(&d, "tarif", 1, "id", &id0);
    modifier(&d, "tarif", 2, "id", "\\N");
    let r = decoder(&d);
    let mut c = codes(&r, "tarif");
    c.sort();
    assert!(c.contains(&"cle_dupliquee".to_string()), "{c:?}");
    assert!(
        c.contains(&"null_interdit".to_string()) || c.contains(&"cle_absente".to_string()),
        "{c:?}"
    );
    assert_eq!(r.rapport.tables["tarif"].comprises, 1);
}

#[test]
fn documents_aplatis_decodage_strict() {
    let d = jeu_decodable("dec-attr");
    let sain = decoder(&d);
    assert!(sain.rapport.documents_attr > 0);
    assert!(sain.tables["travel_attr"]
        .lignes
        .iter()
        .all(|l| l.type_feuille.is_some()));
    modifier(&d, "travel_attr", 0, "value", "x:inconnu");
    modifier(&d, "travel_attr", 1, "value", "n:pas-un-nombre");
    modifier(&d, "request_attr", 2, "path", "a..b");
    let r = decoder(&d);
    let c = codes(&r, "travel_attr");
    assert!(
        c.contains(&"attr_prefixe".to_string()) && c.contains(&"attr_nombre".to_string()),
        "{c:?}"
    );
    assert!(codes(&r, "request_attr").contains(&"attr_chemin".to_string()));
    assert!(r.tables["travel_attr"]
        .lignes
        .iter()
        .any(|l| l.type_feuille == Some(TypeFeuille::Texte)
            || l.type_feuille == Some(TypeFeuille::Json)
            || l.type_feuille == Some(TypeFeuille::Booleen)));
}

#[test]
fn document_aplati_incoherent_est_refuse_en_entier() {
    let d = jeu_decodable("dec-attr-conflit");
    // Deux feuilles du même document : `a` (feuille) et `a.b` (enfant).
    modifier(&d, "travel_attr", 0, "path", "a");
    modifier(&d, "travel_attr", 1, "path", "a.b");
    let src = std::fs::read_to_string(d.join("dump.sql")).unwrap();
    let a = src.find("COPY public.travel_attr (").unwrap();
    let l0: Vec<&str> = src[a..].lines().nth(1).unwrap().split('\t').collect();
    let l1: Vec<&str> = src[a..].lines().nth(2).unwrap().split('\t').collect();
    // Même propriétaire (colonne travel_id) pour les deux lignes.
    let cols = src[a..].lines().next().unwrap();
    let i = cols
        .split_once('(')
        .unwrap()
        .1
        .split_once(')')
        .unwrap()
        .0
        .split(", ")
        .position(|c| c == "travel_id")
        .unwrap();
    modifier(&d, "travel_attr", 1, "travel_id", l0[i]);
    let _ = l1;
    let r = decoder(&d);
    assert!(r
        .rapport
        .refus
        .iter()
        .any(|x| x.code == "attr_feuille_et_enfants"));
    assert!(r.rapport.invariants_ok());
    assert_eq!(
        r.rapport.tables["travel_attr"].comprises as usize,
        r.tables["travel_attr"].lignes.len()
    );
}

#[test]
fn collection_inconnue_et_sante() {
    let d = jeu_decodable("dec-coll");
    modifier(
        &d,
        "ent_collection_record",
        0,
        "collection",
        "inventee__chose",
    );
    modifier(
        &d,
        "ent_collection_attr",
        0,
        "collection",
        "settings__bien-etre",
    );
    modifier(
        &d,
        "ent_collection_attr",
        1,
        "collection",
        "stages__student-tokens",
    );
    let r = decoder(&d);
    assert_eq!(
        r.rapport.collections["inventee__chose"].refus_par_code["collection_inconnue"],
        1
    );
    assert_eq!(
        r.rapport.collections["settings__bien-etre"].refus_par_code["sante_hds"],
        1
    );
    assert_eq!(
        r.rapport.collections["stages__student-tokens"].ignorees_par_raison["jeton"],
        1
    );
    assert!(r.rapport.invariants_ok());
}

#[test]
fn json_s3_decodes_expires_et_refuses() {
    let d = jeu_decodable("dec-s3");
    let r = decoder(&d);
    // Valide + expiré + demande : 3 documents ; l'expiré est ignoré, pas perdu en silence.
    assert_eq!(r.documents_s3.len(), 3);
    assert_eq!(r.rapport.s3["RentreeEnAttente"].comprises, 1);
    assert_eq!(
        r.rapport.s3["RentreeEnAttente"].ignorees_par_raison["jeton_expire"],
        1
    );
    assert_eq!(r.rapport.s3["DemandeEnAttente"].comprises, 1);
    // Clé inconnue dans un meta.json : refusé.
    let p = d.join("contenus_s3/requests/pending/demande-1/meta.json");
    let t = std::fs::read_to_string(&p)
        .unwrap()
        .replace("\"phone\"", "\"inconnue\":1,\"phone\"");
    std::fs::write(&p, t).unwrap();
    // Contenu manquant : refusé.
    std::fs::remove_file(d.join("contenus_s3/rentree/submissions/pending/jeton-expire/meta.json"))
        .unwrap();
    let r = decoder(&d);
    assert_eq!(
        r.rapport.s3["DemandeEnAttente"].refus_par_code["json_cle_inconnue"],
        1
    );
    assert_eq!(
        r.rapport.s3["RentreeEnAttente"].refus_par_code["s3_contenu_absent"],
        1
    );
    assert!(r.rapport.invariants_ok());
}

#[test]
fn json_s3_sans_contenus_est_compte_non_lu() {
    let d = jeu_decodable("dec-s3-nolu");
    let mut o = options(&d);
    o.contenus_s3 = None;
    let r = decoder_export(&o).unwrap();
    assert_eq!(
        r.rapport.s3["RentreeEnAttente"].ignorees_par_raison["contenu_non_fourni"],
        2
    );
    assert!(r.documents_s3.is_empty());
}

#[test]
fn les_lignes_chargees_portent_disposition_et_empreinte() {
    let d = jeu_decodable("dec-disp");
    let r = decoder(&d);
    let l = &r.tables["eleve"].lignes[0];
    assert_eq!(l.disposition, Disposition::Rapprochement);
    assert_eq!(l.empreinte.len(), 64);
    assert_eq!(l.cellules.len(), r.tables["eleve"].colonnes.len());
    let c = r.tables["ent_collection_record"]
        .lignes
        .iter()
        .find(|l| l.objet == "collection:internat__rooms")
        .unwrap();
    assert_eq!(c.disposition, Disposition::Reprise);
}

#[test]
fn refuse_une_adresse_de_connexion() {
    let mut o = options(&jeu_decodable("dec-garde"));
    o.dump = PathBuf::from("postgres://u:p@hote/db");
    assert!(decoder_export(&o).is_err());
}

#[test]
fn commandes_decoder_et_simuler() {
    let d = jeu_decodable("dec-cli");
    let o = lancer(&[
        "decoder",
        "--dump",
        d.join("dump.sql").to_str().unwrap(),
        "--objets-s3",
        d.join("objets_s3.tsv").to_str().unwrap(),
        "--contenus-s3",
        d.join("contenus_s3").to_str().unwrap(),
        "--maintenant",
        "2026-11-16T12:00:00Z",
        "--json",
        d.join("r.json").to_str().unwrap(),
    ]);
    let t = sortie_texte(&o);
    assert!(t.contains("Décodage strict"), "{t}");
    // Santé refusée et objets S3 secrets : le verdict n'est pas « conforme » sur ce jeu.
    assert_eq!(o.status.code(), Some(2), "{t}");
    assert!(std::fs::read_to_string(d.join("r.json"))
        .unwrap()
        .contains("sante_hds"));
    let o = lancer(&[
        "simuler",
        "--dump",
        d.join("dump.sql").to_str().unwrap(),
        "--etablissement-cible",
        "5c011a00-0000-4000-8000-000000000028",
        "--maintenant",
        "2026-11-16T12:00:00Z",
    ]);
    let t = sortie_texte(&o);
    assert!(
        t.contains("Écritures dans les tables métier : 0") && t.contains("Invariant : respecté"),
        "{t}"
    );
    // UUID invalide et adresse de connexion : refusés.
    let o = lancer(&[
        "simuler",
        "--dump",
        d.join("dump.sql").to_str().unwrap(),
        "--etablissement-cible",
        "x",
    ]);
    assert_eq!(o.status.code(), Some(1));
    let o = lancer(&["decoder", "--dump", "postgres://u:p@hote/db"]);
    assert_eq!(o.status.code(), Some(1));
}
