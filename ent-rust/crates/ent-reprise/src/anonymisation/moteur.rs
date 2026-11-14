//! Moteur d'anonymisation : lecture en flux, transformation cellule par cellule selon la liste
//! blanche, écriture, contrôle « zéro occurrence », rapport, empreintes.

use super::cle::Cle;
use super::cle_s3::reecrire_cle;
use super::controle::Sensibles;
use super::json::{anonymiser_attr, Contexte, StatsJson};
use super::pseudo::{pseudonymiser, texte};
use super::rapport::{
    ColonneRapport, ControleRapport, FichierRapport, FuiteRapport, RapportAnonymisation, S3Rapport,
    TableRapportAnon,
};
use crate::copy::encoder_ligne;
use crate::dump::{self, CollecteTables, Evt};
use crate::empreintes::sha256_fichier;
use crate::s3::{self, ObjetS3};
use crate::schema::{Genre, ListeBlanche, Regle, SchemaAttendu, TABLES_SANTE};
use crate::{err, garde, Erreur};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Options {
    pub dump: PathBuf,
    pub objets_s3: Option<PathBuf>,
    pub sortie: PathBuf,
    pub cle: PathBuf,
    /// Fichier de mots à ne pas contrôler par jeton (un par ligne) — voir la documentation.
    pub mots_autorises: Option<PathBuf>,
}

/// Échec de l'anonymisation.
#[derive(Debug)]
pub enum Echec {
    /// Une table ou une colonne du dump n'est pas déclarée à la liste blanche : rien n'a été écrit.
    NonDeclare {
        tables: Vec<String>,
        colonnes: Vec<String>,
    },
    /// Le contrôle final a retrouvé des valeurs réelles : l'export n'est pas livré.
    Fuites(Vec<FuiteRapport>),
    Autre(Erreur),
}

impl From<Erreur> for Echec {
    fn from(e: Erreur) -> Self {
        Echec::Autre(e)
    }
}
impl From<std::io::Error> for Echec {
    fn from(e: std::io::Error) -> Self {
        Echec::Autre(e.into())
    }
}

impl std::fmt::Display for Echec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Echec::NonDeclare { tables, colonnes } => {
                writeln!(f, "REFUS : l'export contient des éléments non déclarés à la liste blanche (l'outil est fermé par défaut). Rien n'a été écrit.")?;
                if !tables.is_empty() {
                    writeln!(f, "Tables non déclarées ({}) :", tables.len())?;
                    for t in tables.iter().take(100) {
                        writeln!(f, "  - {t}")?;
                    }
                }
                if !colonnes.is_empty() {
                    writeln!(f, "Colonnes non déclarées ({}) :", colonnes.len())?;
                    for c in colonnes.iter().take(100) {
                        writeln!(f, "  - {c}")?;
                    }
                }
                write!(f, "Envoyez cette liste à l'équipe (elle ne contient que des noms de tables et de colonnes, aucune donnée).")
            }
            Echec::Fuites(v) => {
                writeln!(f, "REFUS : le contrôle final a retrouvé des valeurs réelles dans l'export. Il n'est PAS livré (fichiers supprimés).")?;
                for x in v {
                    writeln!(f, "  - {} cellule(s) dans {}", x.cellules, x.colonne)?;
                }
                write!(f, "Relancez avec une autre clé (une collision fortuite est possible) ; si l'écart persiste, envoyez cette liste à l'équipe (aucune valeur n'y figure).")
            }
            Echec::Autre(e) => write!(f, "{e}"),
        }
    }
}

struct ColCtx {
    nom: String,
    regle: Regle,
    nullable: bool,
    tableau: bool,
}

struct TableCtx {
    nom: String,
    cols: Vec<ColCtx>,
    idx_path: Option<usize>,
    sante: bool,
    lignes: u64,
    compteurs: Vec<(u64, u64)>,
}

struct Etat {
    cle: Cle,
    sens: Sensibles,
    stats: StatsJson,
}

fn nom_regle(r: Regle) -> String {
    match r {
        Regle::Garder => "garder".into(),
        Regle::Effacer => "effacer".into(),
        Regle::CleS3 => "cle_s3".into(),
        Regle::Json => "json".into(),
        Regle::AttrValeur => "attr_valeur".into(),
        Regle::Chemin => "chemin".into(),
        Regle::Pseudo(g) => format!("pseudo:{}", nom_genre(g)),
    }
}

fn nom_genre(g: Genre) -> &'static str {
    match g {
        Genre::Nom => "nom",
        Genre::Prenom => "prenom",
        Genre::NomComplet => "nom_complet",
        Genre::Email => "email",
        Genre::Telephone => "telephone",
        Genre::Ine => "ine",
        Genre::Iban => "iban",
        Genre::Bic => "bic",
        Genre::Rum => "rum",
        Genre::DateNaissance => "date_naissance",
        Genre::Ville => "ville",
        Genre::Adresse => "adresse",
        Genre::CodePostal => "code_postal",
        Genre::Ip => "ip",
        Genre::Agent => "agent",
        Genre::Identifiant => "identifiant",
        Genre::IdSouple => "id_souple",
        Genre::Jeton => "jeton",
        Genre::Image => "image",
        Genre::Pdf => "pdf",
        Genre::NomFichier => "nom_fichier",
        Genre::Texte => "texte",
    }
}

// ---- tableaux PostgreSQL `{a,"b c",NULL}` ----------------------------------------------------

fn lire_tableau(s: &str) -> Option<Vec<Option<String>>> {
    let t = s.trim();
    let interieur = t.strip_prefix('{')?.strip_suffix('}')?;
    let mut v = Vec::new();
    let mut cs = interieur.chars().peekable();
    if interieur.is_empty() {
        return Some(v);
    }
    loop {
        let mut elt = String::new();
        let mut cite = false;
        if cs.peek() == Some(&'"') {
            cite = true;
            cs.next();
            loop {
                match cs.next()? {
                    '\\' => elt.push(cs.next()?),
                    '"' => break,
                    c => elt.push(c),
                }
            }
        } else {
            while let Some(&c) = cs.peek() {
                if c == ',' {
                    break;
                }
                if c == '{' || c == '}' {
                    return None; // tableaux imbriqués : non pris en charge
                }
                elt.push(c);
                cs.next();
            }
        }
        v.push(if !cite && elt.eq_ignore_ascii_case("NULL") {
            None
        } else {
            Some(elt)
        });
        match cs.next() {
            None => break,
            Some(',') => {}
            Some(_) => return None,
        }
    }
    Some(v)
}

fn ecrire_tableau(v: &[Option<String>]) -> String {
    let mut s = String::from("{");
    for (i, e) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        match e {
            None => s.push_str("NULL"),
            Some(x) => {
                let besoin = x.is_empty()
                    || x.eq_ignore_ascii_case("null")
                    || x.chars()
                        .any(|c| matches!(c, ',' | '{' | '}' | '"' | '\\') || c.is_whitespace());
                if besoin {
                    s.push('"');
                    for c in x.chars() {
                        if c == '"' || c == '\\' {
                            s.push('\\');
                        }
                        s.push(c);
                    }
                    s.push('"');
                } else {
                    s.push_str(x);
                }
            }
        }
    }
    s.push('}');
    s
}

// ---- transformation d'une valeur -------------------------------------------------------------

fn scalaire(etat: &mut Etat, regle: Regle, v: &str, chemin: Option<&str>) -> String {
    match regle {
        Regle::Garder => v.to_string(),
        Regle::Effacer => String::new(),
        Regle::Pseudo(g) => {
            etat.sens.noter(g, v);
            pseudonymiser(&etat.cle, g, v)
        }
        Regle::CleS3 => reecrire_cle(&etat.cle, v),
        Regle::Chemin => super::json::reecrire_chemin(&etat.cle, v),
        Regle::Json => {
            let mut ctx = Contexte {
                cle: &etat.cle,
                stats: &mut etat.stats,
                sensibles: &mut etat.sens,
            };
            match serde_json::from_str::<Value>(v) {
                Ok(j) => {
                    serde_json::to_string(&ctx.valeur(&j, None)).unwrap_or_else(|_| "{}".into())
                }
                Err(_) => {
                    serde_json::to_string(&texte(ctx.cle, v)).unwrap_or_else(|_| "\"\"".into())
                }
            }
        }
        Regle::AttrValeur => {
            let mut ctx = Contexte {
                cle: &etat.cle,
                stats: &mut etat.stats,
                sensibles: &mut etat.sens,
            };
            anonymiser_attr(&mut ctx, chemin.unwrap_or(""), v)
        }
    }
}

fn cellule(
    etat: &mut Etat,
    c: &ColCtx,
    v: Option<String>,
    chemin: Option<&str>,
) -> (Option<String>, bool) {
    let Some(v) = v else { return (None, false) };
    if matches!(c.regle, Regle::Garder) {
        return (Some(v), false);
    }
    if matches!(c.regle, Regle::Effacer) {
        let vide = if c.nullable {
            None
        } else if c.tableau {
            Some("{}".to_string())
        } else {
            Some(String::new())
        };
        return (vide, true);
    }
    if c.tableau && !matches!(c.regle, Regle::Json) {
        return match lire_tableau(&v) {
            Some(elts) => {
                let sortie: Vec<Option<String>> = elts
                    .into_iter()
                    .map(|e| e.map(|x| scalaire(etat, c.regle, &x, chemin)))
                    .collect();
                (Some(ecrire_tableau(&sortie)), true)
            }
            // Tableau illisible : jamais gardé en clair.
            None => (Some("{}".to_string()), true),
        };
    }
    (Some(scalaire(etat, c.regle, &v, chemin)), true)
}

// ---- passes ----------------------------------------------------------------------------------

fn contexte_table(
    table: &str,
    colonnes: &[String],
    blanche: &ListeBlanche,
    schema: &SchemaAttendu,
) -> Result<TableCtx, Echec> {
    let attendu = schema.tables.get(table);
    let mut cols = Vec::new();
    let mut manquantes = Vec::new();
    for c in colonnes {
        match blanche.regle(table, c) {
            Some(regle) => {
                let meta = attendu.and_then(|t| t.iter().find(|x| &x.nom == c));
                cols.push(ColCtx {
                    nom: c.clone(),
                    regle,
                    nullable: meta.map(|m| m.nullable).unwrap_or(true),
                    tableau: meta.map(|m| m.type_sql.ends_with("[]")).unwrap_or(false),
                });
            }
            None => manquantes.push(format!("{table}.{c}")),
        }
    }
    if !manquantes.is_empty() {
        return Err(Echec::NonDeclare {
            tables: vec![],
            colonnes: manquantes,
        });
    }
    let idx_path = colonnes.iter().position(|c| c == "path");
    Ok(TableCtx {
        nom: table.to_string(),
        compteurs: vec![(0, 0); cols.len()],
        cols,
        idx_path,
        sante: TABLES_SANTE.contains(&table),
        lignes: 0,
    })
}

/// Vérifie, **avant d'écrire**, que tout ce que contient le dump est déclaré.
fn verifier_declare(
    source: &dump::Source,
    blanche: &ListeBlanche,
    schema: &SchemaAttendu,
) -> Result<(), Echec> {
    let mut collecte = CollecteTables::default();
    let mut copies: Vec<(String, Vec<String>)> = Vec::new();
    dump::lire(source, &mut |e| {
        match e {
            Evt::Ddl(l) => collecte.ligne(&l),
            Evt::CopyDebut {
                schema: sch,
                table,
                colonnes,
            } => copies.push((dump::cle_table(&sch, &table), colonnes)),
            _ => {}
        }
        Ok(())
    })?;
    let mut tables: BTreeSet<String> = BTreeSet::new();
    let mut colonnes: BTreeSet<String> = BTreeSet::new();
    let connues: BTreeSet<&str> = schema.tables.keys().map(String::as_str).collect();
    let mut a_verifier: Vec<(String, Vec<String>)> = copies;
    for (t, cols) in &collecte.tables {
        a_verifier.push((t.clone(), cols.iter().map(|c| c.nom.clone()).collect()));
    }
    for (t, cols) in a_verifier {
        if !connues.contains(t.as_str()) {
            tables.insert(t);
            continue;
        }
        for c in cols {
            if blanche.regle(&t, &c).is_none() {
                colonnes.insert(format!("{t}.{c}"));
            }
        }
    }
    if tables.is_empty() && colonnes.is_empty() {
        Ok(())
    } else {
        Err(Echec::NonDeclare {
            tables: tables.into_iter().collect(),
            colonnes: colonnes.into_iter().collect(),
        })
    }
}

fn supprimer_sorties(dossier: &std::path::Path) {
    for f in [
        "dump.sql",
        "objets_s3.tsv",
        "SHA256SUMS",
        "EMPREINTE.txt",
        "rapport_anonymisation.json",
        "rapport_anonymisation.txt",
    ] {
        let _ = std::fs::remove_file(dossier.join(f));
    }
}

/// Lance l'anonymisation. En cas de succès, retourne le rapport et l'empreinte globale de l'export.
pub fn anonymiser(o: &Options) -> Result<(RapportAnonymisation, String), Echec> {
    // 0. Gardes : jamais de connexion ; sortie vierge.
    for (opt, p) in [
        ("--dump", Some(&o.dump)),
        ("--objets-s3", o.objets_s3.as_ref()),
        ("--cle", Some(&o.cle)),
        ("--sortie", Some(&o.sortie)),
    ] {
        if let Some(p) = p {
            garde::exiger_pas_de_connexion(opt, &p.to_string_lossy())?;
        }
    }
    if o.sortie.exists() {
        let vide = o.sortie.is_dir() && std::fs::read_dir(&o.sortie)?.next().is_none();
        if !vide {
            return Err(err!(
                "--sortie : {} existe déjà et n'est pas vide (refus d'écraser un export)",
                o.sortie.display()
            )
            .into());
        }
    }
    let source = dump::ouvrir(&o.dump)?;
    let cle = Cle::depuis_fichier(&o.cle)?;
    let blanche = ListeBlanche::embarquee();
    let schema = SchemaAttendu::embarque();
    let liste_s3 = match &o.objets_s3 {
        Some(p) => Some(s3::lire_liste(p)?),
        None => None,
    };

    // 1. Fermé par défaut : tout ce qui est dans le dump doit être déclaré. Rien n'est écrit avant.
    verifier_declare(&source, &blanche, &schema)?;

    std::fs::create_dir_all(&o.sortie)?;
    let resultat = anonymiser_ecrire(o, &source, cle, &blanche, &schema, liste_s3);
    if resultat.is_err() {
        supprimer_sorties(&o.sortie);
    }
    resultat
}

fn anonymiser_ecrire(
    o: &Options,
    source: &dump::Source,
    cle: Cle,
    blanche: &ListeBlanche,
    schema: &SchemaAttendu,
    liste_s3: Option<Vec<ObjetS3>>,
) -> Result<(RapportAnonymisation, String), Echec> {
    let mut etat = Etat {
        cle,
        sens: Sensibles::default(),
        stats: StatsJson::default(),
    };
    if let Some(p) = &o.mots_autorises {
        garde::exiger_pas_de_connexion("--mots-autorises", &p.to_string_lossy())?;
        for l in std::fs::read_to_string(p)?.lines() {
            etat.sens.autoriser(l);
        }
    }
    let mut rapport = RapportAnonymisation {
        outil_version: env!("CARGO_PKG_VERSION").to_string(),
        cle_empreinte: etat.cle.empreinte(),
        ..Default::default()
    };

    // 2. Transformation en flux.
    let chemin_dump = o.sortie.join("dump.sql");
    let mut sortie = BufWriter::with_capacity(1 << 20, File::create(&chemin_dump)?);
    let mut courante: Option<TableCtx> = None;
    dump::lire(source, &mut |e| {
        match e {
            Evt::Ddl(l) => {
                writeln!(sortie, "{l}")?;
            }
            Evt::CopyDebut {
                schema: s,
                table,
                colonnes,
            } => {
                let t = contexte_table(&dump::cle_table(&s, &table), &colonnes, blanche, schema)
                    .map_err(|e| err!("{e}"))?;
                let noms: Vec<String> = colonnes
                    .iter()
                    .map(|c| crate::jeu_fictif::ident(c))
                    .collect();
                writeln!(
                    sortie,
                    "COPY {}.{} ({}) FROM stdin;",
                    crate::jeu_fictif::ident(&s),
                    crate::jeu_fictif::ident(&table),
                    noms.join(", ")
                )?;
                courante = Some(t);
            }
            Evt::Ligne(cells) => {
                let t = courante
                    .as_mut()
                    .ok_or_else(|| err!("ligne de données hors d'un bloc COPY"))?;
                if t.sante {
                    return Ok(());
                }
                if cells.len() != t.cols.len() {
                    return Err(err!(
                        "table {} : {} cellules pour {} colonnes déclarées",
                        t.nom,
                        cells.len(),
                        t.cols.len()
                    ));
                }
                let chemin: Option<String> = t.idx_path.and_then(|i| cells[i].clone());
                let mut sorties: Vec<Option<String>> = Vec::with_capacity(cells.len());
                for (i, v) in cells.into_iter().enumerate() {
                    let nul = v.is_none();
                    let (nv, transformee) = cellule(&mut etat, &t.cols[i], v, chemin.as_deref());
                    if transformee {
                        t.compteurs[i].0 += 1;
                    }
                    if nul {
                        t.compteurs[i].1 += 1;
                    }
                    sorties.push(nv);
                }
                t.lignes += 1;
                writeln!(sortie, "{}", encoder_ligne(&sorties))?;
            }
            Evt::CopyFin => {
                writeln!(sortie, "\\.")?;
                if let Some(t) = courante.take() {
                    rapport.lignes_total += t.lignes;
                    rapport.tables.insert(
                        t.nom.clone(),
                        TableRapportAnon {
                            lignes: t.lignes,
                            exclue_sante: t.sante,
                        },
                    );
                    for (c, (tr, nu)) in t.cols.iter().zip(t.compteurs.iter()) {
                        rapport.colonnes.insert(
                            format!("{}.{}", t.nom, c.nom),
                            ColonneRapport {
                                regle: nom_regle(c.regle),
                                cellules_transformees: *tr,
                                cellules_nulles: *nu,
                            },
                        );
                    }
                }
            }
        }
        Ok(())
    })?;
    sortie.flush()?;
    drop(sortie);

    // 3. Objets S3.
    let mut cles_sortie: Vec<String> = Vec::new();
    if let Some(objets) = liste_s3 {
        let mut r = S3Rapport {
            objets_lus: objets.len() as u64,
            ..Default::default()
        };
        let mut nouveaux = Vec::new();
        for ob in objets {
            if s3::est_secret(&ob.cle) {
                r.objets_secrets_ecartes += 1;
                continue;
            }
            let nc = reecrire_cle(&etat.cle, &ob.cle);
            cles_sortie.push(nc.clone());
            // Empreinte du contenu : l'objet n'est pas copié ; une empreinte fictive déterministe la remplace.
            let sha = ob
                .sha256
                .as_ref()
                .map(|h| hex::encode(etat.cle.mac("sha256-objet", h.as_bytes(), 0)));
            nouveaux.push(ObjetS3 {
                cle: nc,
                taille: ob.taille,
                sha256: sha,
            });
            r.objets_reecrits += 1;
        }
        std::fs::write(o.sortie.join("objets_s3.tsv"), s3::ecrire_liste(&nouveaux))?;
        rapport.s3 = Some(r);
    }

    // 4. Contrôle « zéro occurrence » : relecture de l'export produit.
    let mut fuites: BTreeMap<String, u64> = BTreeMap::new();
    let mut cellules_controlees = 0u64;
    {
        let relu = dump::Source::Texte(chemin_dump.clone());
        let mut courante: Option<(String, Vec<String>)> = None;
        dump::lire(&relu, &mut |e| {
            match e {
                Evt::CopyDebut {
                    table, colonnes, ..
                } => courante = Some((table, colonnes)),
                Evt::Ligne(cells) => {
                    if let Some((t, cols)) = &courante {
                        for (i, c) in cells.iter().enumerate() {
                            if let Some(v) = c {
                                cellules_controlees += 1;
                                if etat.sens.fuite(v) {
                                    *fuites
                                        .entry(format!(
                                            "{t}.{}",
                                            cols.get(i).map(String::as_str).unwrap_or("?")
                                        ))
                                        .or_default() += 1;
                                }
                            }
                        }
                    }
                }
                Evt::CopyFin => courante = None,
                Evt::Ddl(_) => {}
            }
            Ok(())
        })?;
        for k in &cles_sortie {
            cellules_controlees += 1;
            if etat.sens.fuite(k) {
                *fuites.entry("objets_s3.cle".to_string()).or_default() += 1;
            }
        }
    }
    rapport.controle = ControleRapport {
        valeurs_sensibles_controlees: etat.sens.nombre,
        cellules_controlees,
        fuites: fuites
            .into_iter()
            .map(|(colonne, cellules)| FuiteRapport { colonne, cellules })
            .collect(),
        resultat: String::new(),
    };
    if !rapport.controle.fuites.is_empty() {
        return Err(Echec::Fuites(rapport.controle.fuites.clone()));
    }
    rapport.controle.resultat = "ZÉRO occurrence : export livrable".to_string();
    rapport.cles_json_inconnues = etat.stats.cles_inconnues.clone();
    rapport.feuilles_json_traitees = etat.stats.feuilles_traitees;

    // 5. Empreintes et rapport.
    for f in ["dump.sql", "objets_s3.tsv"] {
        let p = o.sortie.join(f);
        if p.is_file() {
            rapport.fichiers.push(FichierRapport {
                nom: f.to_string(),
                octets: std::fs::metadata(&p)?.len(),
                sha256: sha256_fichier(&p)?,
            });
        }
    }
    std::fs::write(
        o.sortie.join("rapport_anonymisation.json"),
        serde_json::to_string_pretty(&rapport).map_err(|e| err!("rapport JSON : {e}"))?,
    )?;
    std::fs::write(o.sortie.join("rapport_anonymisation.txt"), rapport.texte())?;
    let fichiers: Vec<&str> = [
        "dump.sql",
        "objets_s3.tsv",
        "rapport_anonymisation.json",
        "rapport_anonymisation.txt",
    ]
    .into_iter()
    .filter(|f| o.sortie.join(f).is_file())
    .collect();
    std::fs::write(
        o.sortie.join("SHA256SUMS"),
        crate::empreintes::ecrire_sommes(&o.sortie, &fichiers)?,
    )?;
    let globale = sha256_fichier(&o.sortie.join("SHA256SUMS"))?;
    std::fs::write(
        o.sortie.join("EMPREINTE.txt"),
        format!(
            "Empreinte SHA-256 de l'export anonymisé (celle du fichier SHA256SUMS) :\n{globale}\n\n\
Communiquez cette ligne par un AUTRE canal que celui des fichiers (message, appel) : le destinataire la compare à celle qu'il recalcule.\n"
        ),
    )?;
    Ok((rapport, globale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tableaux_aller_retour() {
        let t = lire_tableau("{a,\"b c\",NULL,\"x\\\"y\",\"\"}").unwrap();
        assert_eq!(
            t,
            vec![
                Some("a".into()),
                Some("b c".into()),
                None,
                Some("x\"y".into()),
                Some(String::new())
            ]
        );
        assert_eq!(lire_tableau(&ecrire_tableau(&t)).unwrap(), t);
        assert_eq!(lire_tableau("{}").unwrap(), Vec::<Option<String>>::new());
        assert!(lire_tableau("{{a},{b}}").is_none());
        assert!(lire_tableau("pas un tableau").is_none());
    }
}
