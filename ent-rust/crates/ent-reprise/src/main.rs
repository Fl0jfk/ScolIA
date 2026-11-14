//! Binaire `ent-reprise` : voir `ent-reprise aide`.

use ent_reprise::inspection::{self, Options as OptInspection};
use ent_reprise::{err, garde, jeu_fictif, Res};
use std::collections::BTreeMap;
use std::path::PathBuf;

const AIDE: &str = "\
ent-reprise — outil de reprise des données du Next.js vers ScolIA

Cet outil ne lit que des FICHIERS locaux : il n'ouvre jamais de connexion (base de données, S3, réseau)
et refuse toute adresse de connexion qu'on lui donnerait.

Commandes :
  inspecter   --dump <fichier ou dossier> [--objets-s3 <liste.tsv>] [--sommes <SHA256SUMS>]
              [--empreinte <sha256 du dump>] [--json <rapport.json>]
      Vérifie le schéma attendu, compte les lignes par table et par collection, vérifie les empreintes.
      Code de sortie : 0 conforme, 2 non conforme.

  jeu-fictif  --sortie <dossier neuf> [--lignes <n>]
      Écrit un jeu de données entièrement fictif au format du Next.js (pour essayer l'outil).

  aide | --aide | --version
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match lancer(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Erreur : {e}");
            1
        }
    };
    std::process::exit(code);
}

/// Options `--nom valeur` ou `--nom=valeur`. Toute valeur est contrôlée par la garde anti-connexion.
fn options(args: &[String], autorisees: &[&str]) -> Res<BTreeMap<String, String>> {
    let mut m = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        let Some(nom) = a.strip_prefix("--") else {
            return Err(err!(
                "argument inattendu « {a} » (utilisez --option valeur)"
            ));
        };
        let (nom, valeur) = match nom.split_once('=') {
            Some((n, v)) => (n.to_string(), v.to_string()),
            None => {
                i += 1;
                let v = args
                    .get(i)
                    .ok_or_else(|| err!("--{nom} attend une valeur"))?;
                (nom.to_string(), v.clone())
            }
        };
        if !autorisees.contains(&nom.as_str()) {
            return Err(err!("option inconnue --{nom} (voir « ent-reprise aide »)"));
        }
        garde::exiger_pas_de_connexion(&format!("--{nom}"), &valeur)?;
        m.insert(nom, valeur);
        i += 1;
    }
    Ok(m)
}

fn requis<'a>(o: &'a BTreeMap<String, String>, nom: &str) -> Res<&'a String> {
    o.get(nom).ok_or_else(|| err!("--{nom} est obligatoire"))
}

fn lancer(args: &[String]) -> Res<i32> {
    let Some(cmd) = args.first().map(String::as_str) else {
        print!("{AIDE}");
        return Ok(0);
    };
    let reste = &args[1..];
    match cmd {
        "aide" | "--aide" | "-h" | "--help" => {
            print!("{AIDE}");
            Ok(0)
        }
        "--version" | "version" => {
            println!("ent-reprise {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        "inspecter" => {
            let o = options(reste, &["dump", "objets-s3", "sommes", "empreinte", "json"])?;
            let opt = OptInspection {
                dump: garde::chemin_local_existant("--dump", requis(&o, "dump")?)?,
                objets_s3: match o.get("objets-s3") {
                    Some(p) => Some(garde::chemin_local_existant("--objets-s3", p)?),
                    None => None,
                },
                sommes: match o.get("sommes") {
                    Some(p) => Some(garde::chemin_local_existant("--sommes", p)?),
                    None => None,
                },
                empreinte_dump: o.get("empreinte").cloned(),
            };
            let r = inspection::inspecter(&opt)?;
            print!("{}", inspection::texte(&r));
            if let Some(j) = o.get("json") {
                std::fs::write(
                    j,
                    serde_json::to_string_pretty(&r).map_err(|e| err!("JSON : {e}"))?,
                )?;
                println!("Rapport JSON écrit : {j}");
            }
            Ok(if r.conforme() { 0 } else { 2 })
        }
        "jeu-fictif" => {
            let o = options(reste, &["sortie", "lignes"])?;
            let n: usize = match o.get("lignes") {
                Some(v) => v
                    .parse()
                    .map_err(|_| err!("--lignes : nombre entier attendu"))?,
                None => 4,
            };
            if !(1..=500).contains(&n) {
                return Err(err!("--lignes : entre 1 et 500"));
            }
            let d = PathBuf::from(requis(&o, "sortie")?);
            if d.exists()
                && std::fs::read_dir(&d)
                    .map(|mut x| x.next().is_some())
                    .unwrap_or(true)
            {
                return Err(err!(
                    "--sortie : {} existe déjà et n'est pas vide",
                    d.display()
                ));
            }
            let r = jeu_fictif::generer(&d, n)?;
            println!(
                "Jeu fictif écrit dans {} : {} tables, {} lignes, {} objets S3 (entièrement inventé).",
                d.display(),
                r.tables,
                r.lignes,
                r.objets_s3
            );
            Ok(0)
        }
        autre => Err(err!(
            "commande inconnue « {autre} » (voir « ent-reprise aide »)"
        )),
    }
}
