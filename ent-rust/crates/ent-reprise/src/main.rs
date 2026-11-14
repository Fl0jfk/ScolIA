//! Binaire `ent-reprise` : voir `ent-reprise aide`.

use ent_reprise::anonymisation::cle::Cle;
use ent_reprise::anonymisation::moteur::{self, Echec};
use ent_reprise::empreintes::{self, Verdict};
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

  generer-cle --fichier <cle.txt>
      Crée une clé aléatoire locale (refuse d'écraser un fichier existant).

  anonymiser  --dump <fichier ou dossier> --sortie <dossier neuf> --cle <cle.txt>
              [--objets-s3 <liste.tsv>] [--mots-autorises <mots.txt>]
      Produit un export sans donnée réelle (dump.sql, objets_s3.tsv, rapport, SHA256SUMS, EMPREINTE.txt).
      Code de sortie : 0 livrable, 2 colonne ou table non déclarée, 3 valeur réelle retrouvée.

  verifier    --dossier <export anonymisé> [--empreinte <sha256 attendu de SHA256SUMS>]
      Recalcule les empreintes d'un export anonymisé et relance l'inspection.

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
        "generer-cle" => {
            let o = options(reste, &["fichier"])?;
            let f = PathBuf::from(requis(&o, "fichier")?);
            Cle::generer(&f)?;
            println!("Clé créée : {}", f.display());
            println!("Gardez-la sur votre poste, ne la transmettez JAMAIS. Détruisez-la quand vous n'en avez plus besoin.");
            Ok(0)
        }
        "anonymiser" => {
            let o = options(
                reste,
                &["dump", "objets-s3", "sortie", "cle", "mots-autorises"],
            )?;
            let opt = moteur::Options {
                dump: garde::chemin_local_existant("--dump", requis(&o, "dump")?)?,
                objets_s3: match o.get("objets-s3") {
                    Some(p) => Some(garde::chemin_local_existant("--objets-s3", p)?),
                    None => None,
                },
                sortie: PathBuf::from(requis(&o, "sortie")?),
                cle: garde::chemin_local_existant("--cle", requis(&o, "cle")?)?,
                mots_autorises: match o.get("mots-autorises") {
                    Some(p) => Some(garde::chemin_local_existant("--mots-autorises", p)?),
                    None => None,
                },
            };
            match moteur::anonymiser(&opt) {
                Ok((rapport, empreinte)) => {
                    print!("{}", rapport.texte());
                    println!("EMPREINTE DE L'EXPORT (SHA-256 de SHA256SUMS) :\n{empreinte}");
                    println!("\nExport anonymisé écrit dans : {}", opt.sortie.display());
                    Ok(0)
                }
                Err(Echec::Autre(e)) => Err(e),
                Err(e @ Echec::NonDeclare { .. }) => {
                    eprintln!("{e}");
                    Ok(2)
                }
                Err(e @ Echec::Fuites(_)) => {
                    eprintln!("{e}");
                    Ok(3)
                }
            }
        }
        "verifier" => {
            let o = options(reste, &["dossier", "empreinte"])?;
            let d = garde::chemin_local_existant("--dossier", requis(&o, "dossier")?)?;
            let sommes = d.join("SHA256SUMS");
            let v = empreintes::verifier_sommes(&sommes)?;
            let mut ok = true;
            for x in &v {
                let m = match x.verdict {
                    Verdict::Conforme => "conforme",
                    Verdict::Different => {
                        ok = false;
                        "DIFFÉRENTE"
                    }
                    Verdict::Manquant => {
                        ok = false;
                        "MANQUANT"
                    }
                };
                println!("{:<32} {m}", x.fichier);
            }
            let globale = empreintes::sha256_fichier(&sommes)?;
            println!("Empreinte de l'export (SHA256SUMS) : {globale}");
            if let Some(attendue) = o.get("empreinte") {
                if globale.eq_ignore_ascii_case(attendue.trim()) {
                    println!("Empreinte attendue : conforme");
                } else {
                    println!("Empreinte attendue : DIFFÉRENTE — l'export a été modifié ou ce n'est pas le bon.");
                    ok = false;
                }
            }
            let dump = d.join("dump.sql");
            if dump.is_file() {
                let r = inspection::inspecter(&OptInspection {
                    dump,
                    objets_s3: None,
                    sommes: None,
                    empreinte_dump: None,
                })?;
                println!(
                    "Schéma : {} ; {} ligne(s) dans {} table(s).",
                    if r.ecarts_schema.est_vide() {
                        "conforme"
                    } else {
                        "ÉCART (voir « ent-reprise inspecter »)"
                    },
                    r.lignes_total,
                    r.tables_vues_avec_donnees
                );
                ok &= r.ecarts_schema.est_vide();
            }
            println!(
                "Résultat : {}",
                if ok { "CONFORME" } else { "NON CONFORME" }
            );
            Ok(if ok { 0 } else { 2 })
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
