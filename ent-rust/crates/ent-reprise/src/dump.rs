//! Lecture d'un dump PostgreSQL **déjà exporté** : format texte (`pg_dump --format=plain`) ou
//! répertoire (`pg_dump --format=directory -Z 0`, données **non compressées**).
//!
//! La décompression gzip (compression par défaut du format répertoire) n'est **pas** prise en
//! charge : elle exigerait une dépendance de plus (règle 23, D-062, à présenter à Florian). Un dossier
//! compressé est refusé avec le moyen de le convertir.
//!
//! Tout est présenté comme un flux d'événements : lignes de définition (`Ddl`), début d'un bloc
//! `COPY`, lignes de données décodées, fin du bloc. Les dumps au format `INSERT` ne sont pas pris en
//! charge (il faut le format `COPY`, celui par défaut de `pg_dump`).
//!
//! Le format répertoire est lu via son `toc.dat` (versions d'archive 1.14 et 1.15 vérifiées sur un
//! dump réel de PostgreSQL 16 ; 1.16, PostgreSQL 17 et 18, lue d'après la documentation du format
//! mais **non vérifiée sur un dump réel** : en cas de doute, convertir avec
//! `pg_restore --file=export.sql <dossier>` puis fournir ce fichier texte).

use crate::copy::decouper_ligne;
use crate::{err, Res};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Clé d'une table : le nom nu pour le schéma `public`, `schema.nom` sinon (ex. `drizzle.__drizzle_migrations`).
pub fn cle_table(schema: &str, table: &str) -> String {
    if schema == "public" {
        table.to_string()
    } else {
        format!("{schema}.{table}")
    }
}

/// Événement du flux de lecture.
#[derive(Debug, Clone, PartialEq)]
pub enum Evt {
    /// Ligne de définition ou de commentaire (hors données), sans fin de ligne.
    Ddl(String),
    /// Début d'un bloc de données d'une table.
    CopyDebut {
        schema: String,
        table: String,
        colonnes: Vec<String>,
    },
    /// Une ligne de données, cellules décodées (`None` = NULL).
    Ligne(Vec<Option<String>>),
    /// Fin du bloc de données.
    CopyFin,
}

/// Où se trouve le dump.
#[derive(Debug, Clone)]
pub enum Source {
    Texte(PathBuf),
    Repertoire(PathBuf),
}

/// Reconnaît le type de dump d'après le chemin : fichier → texte ; dossier contenant `toc.dat` → répertoire.
pub fn ouvrir(chemin: &Path) -> Res<Source> {
    if chemin.is_file() {
        return Ok(Source::Texte(chemin.to_path_buf()));
    }
    if chemin.is_dir() {
        if chemin.join("toc.dat").is_file() {
            return Ok(Source::Repertoire(chemin.to_path_buf()));
        }
        return Err(err!(
            "le dossier {} ne contient pas de toc.dat : ce n'est pas un dump pg_dump au format répertoire",
            chemin.display()
        ));
    }
    Err(err!("dump introuvable : {}", chemin.display()))
}

/// Lit tout le dump et appelle `f` pour chaque événement.
pub fn lire(source: &Source, f: &mut dyn FnMut(Evt) -> Res<()>) -> Res<()> {
    match source {
        Source::Texte(p) => {
            let fichier = File::open(p).map_err(|e| err!("ouverture du dump impossible : {e}"))?;
            lire_texte(BufReader::with_capacity(1 << 20, fichier), f)
        }
        Source::Repertoire(d) => lire_repertoire(d, f),
    }
}

/// Lecture d'un dump au format texte.
pub fn lire_texte<R: BufRead>(mut r: R, f: &mut dyn FnMut(Evt) -> Res<()>) -> Res<()> {
    let mut tampon: Vec<u8> = Vec::new();
    let mut numero = 0u64;
    let mut dans_copy = false;
    loop {
        tampon.clear();
        let n = r.read_until(b'\n', &mut tampon)?;
        if n == 0 {
            break;
        }
        numero += 1;
        if tampon.last() == Some(&b'\n') {
            tampon.pop();
        }
        if tampon.last() == Some(&b'\r') {
            tampon.pop();
        }
        let ligne = std::str::from_utf8(&tampon).map_err(|_| {
            err!("ligne {numero} : le dump n'est pas en UTF-8 (réexporter avec l'encodage UTF8)")
        })?;
        if dans_copy {
            if ligne == "\\." {
                dans_copy = false;
                f(Evt::CopyFin)?;
            } else {
                f(Evt::Ligne(decouper_ligne(ligne)))?;
            }
            continue;
        }
        if ligne.starts_with("COPY ") && ligne.ends_with("FROM stdin;") {
            let (schema, table, colonnes) = analyser_copy(ligne)
                .ok_or_else(|| err!("ligne {numero} : instruction COPY non reconnue"))?;
            dans_copy = true;
            f(Evt::CopyDebut {
                schema,
                table,
                colonnes,
            })?;
            continue;
        }
        if ligne.starts_with("INSERT INTO ") {
            return Err(err!(
                "ligne {numero} : ce dump contient des INSERT. Seul le format COPY (par défaut de pg_dump, sans --inserts ni --column-inserts) est pris en charge"
            ));
        }
        f(Evt::Ddl(ligne.to_string()))?;
    }
    if dans_copy {
        return Err(err!(
            "le dump se termine au milieu d'un bloc COPY (fichier tronqué ?)"
        ));
    }
    Ok(())
}

/// Découpe un identifiant éventuellement entre guillemets au début de `s` ; rend (identifiant, reste).
fn identifiant(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    if let Some(reste) = s.strip_prefix('"') {
        let mut id = String::new();
        let mut it = reste.char_indices().peekable();
        while let Some((i, c)) = it.next() {
            if c == '"' {
                if matches!(it.peek(), Some((_, '"'))) {
                    id.push('"');
                    it.next();
                } else {
                    return Some((id, &reste[i + 1..]));
                }
            } else {
                id.push(c);
            }
        }
        None
    } else {
        let fin = s
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
            .unwrap_or(s.len());
        if fin == 0 {
            return None;
        }
        Some((s[..fin].to_string(), &s[fin..]))
    }
}

/// `COPY public.eleve (id, nom) FROM stdin;` → (`public`, `eleve`, [`id`, `nom`]).
pub fn analyser_copy(ligne: &str) -> Option<(String, String, Vec<String>)> {
    let reste = ligne.trim().strip_prefix("COPY ")?;
    let (premier, mut reste) = identifiant(reste)?;
    let (schema, table) = if let Some(r) = reste.strip_prefix('.') {
        let (t, r2) = identifiant(r)?;
        reste = r2;
        (premier, t)
    } else {
        ("public".to_string(), premier)
    };
    let reste = reste.trim_start();
    let mut colonnes = Vec::new();
    let reste = if let Some(mut r) = reste.strip_prefix('(') {
        loop {
            let (c, r2) = identifiant(r)?;
            colonnes.push(c);
            let r2 = r2.trim_start();
            if let Some(r3) = r2.strip_prefix(',') {
                r = r3;
            } else {
                break r2.strip_prefix(')')?;
            }
        }
    } else {
        reste
    };
    if !reste.trim_start().starts_with("FROM stdin") {
        return None;
    }
    Some((schema, table, colonnes))
}

// ----------------------------------------------------------------------------------------------
// Format répertoire
// ----------------------------------------------------------------------------------------------

/// Entrée du sommaire (`toc.dat`) utile à la lecture.
#[derive(Debug, Clone)]
pub struct EntreeToc {
    pub id: i64,
    pub tag: String,
    pub desc: String,
    pub definition: String,
    pub copy: String,
    pub fichier: Option<String>,
}

struct Lecteur<'a> {
    b: &'a [u8],
    p: usize,
    taille_int: usize,
}

impl Lecteur<'_> {
    fn octet(&mut self) -> Res<u8> {
        let v = *self.b.get(self.p).ok_or_else(|| err!("toc.dat tronqué"))?;
        self.p += 1;
        Ok(v)
    }
    fn entier(&mut self) -> Res<i64> {
        let signe = self.octet()?;
        let mut v: i64 = 0;
        for k in 0..self.taille_int {
            v |= i64::from(self.octet()?) << (8 * k);
        }
        Ok(if signe != 0 { -v } else { v })
    }
    fn chaine(&mut self) -> Res<Option<String>> {
        let n = self.entier()?;
        if n < 0 {
            return Ok(None);
        }
        let n = n as usize;
        let fin = self
            .p
            .checked_add(n)
            .filter(|f| *f <= self.b.len())
            .ok_or_else(|| err!("toc.dat corrompu (longueur de chaîne)"))?;
        let s = String::from_utf8_lossy(&self.b[self.p..fin]).into_owned();
        self.p = fin;
        Ok(Some(s))
    }
}

/// Lit le sommaire d'un dump au format répertoire.
pub fn lire_toc(dossier: &Path) -> Res<(Vec<EntreeToc>, u8)> {
    let b = std::fs::read(dossier.join("toc.dat")).map_err(|e| err!("lecture de toc.dat : {e}"))?;
    if b.len() < 16 || &b[..5] != b"PGDMP" {
        return Err(err!(
            "toc.dat : signature PGDMP absente (ce n'est pas un sommaire pg_dump)"
        ));
    }
    let (vmaj, vmin) = (b[5], b[6]);
    let taille_int = usize::from(b[8]);
    if vmaj != 1 || !(14..=16).contains(&vmin) {
        return Err(err!(
            "toc.dat : version d'archive {}.{} non prise en charge (1.14 à 1.16). Convertir avec « pg_restore --file=export.sql <dossier> » et fournir ce fichier texte.",
            vmaj, vmin
        ));
    }
    if taille_int != 4 {
        return Err(err!(
            "toc.dat : taille d'entier {taille_int} non prise en charge"
        ));
    }
    let mut l = Lecteur {
        b: &b,
        p: 10,
        taille_int,
    };
    let _format = l.octet()?;
    let compression = l.octet()?;
    if compression != 0 {
        return Err(err!(
            "les données du dossier sont compressées : non pris en charge. Réexporter avec « pg_dump --format=directory -Z 0 », ou avec « --format=plain », ou convertir avec « pg_restore --file=export.sql <dossier> » puis fournir ce fichier texte."
        ));
    }
    for _ in 0..7 {
        l.entier()?;
    }
    for _ in 0..3 {
        l.chaine()?;
    }
    let n = l.entier()?;
    if !(0..=1_000_000).contains(&n) {
        return Err(err!("toc.dat corrompu (nombre d'entrées)"));
    }
    let mut v = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let id = l.entier()?;
        let _a_un_dumper = l.entier()?;
        let _tableoid = l.chaine()?;
        let _oid = l.chaine()?;
        let tag = l.chaine()?.unwrap_or_default();
        let desc = l.chaine()?.unwrap_or_default();
        let _section = l.entier()?;
        let definition = l.chaine()?.unwrap_or_default();
        let _drop = l.chaine()?;
        let copy = l.chaine()?.unwrap_or_default();
        let _namespace = l.chaine()?;
        let _tablespace = l.chaine()?;
        let _tableam = l.chaine()?;
        if vmin >= 16 {
            let _relkind = l.entier()?;
        }
        let _owner = l.chaine()?;
        let _with_oids = l.chaine()?;
        while l.chaine()?.is_some() {} // dépendances
        let fichier = l.chaine()?;
        v.push(EntreeToc {
            id,
            tag,
            desc,
            definition,
            copy,
            fichier,
        });
    }
    Ok((v, compression))
}

fn ouvrir_donnees(dossier: &Path, nom: &str) -> Res<Box<dyn BufRead>> {
    if nom.contains('/') || nom.contains('\\') || nom.contains("..") {
        return Err(err!("nom de fichier de données suspect dans toc.dat"));
    }
    let chemin = dossier.join(nom);
    if chemin.is_file() {
        Ok(Box::new(BufReader::with_capacity(
            1 << 20,
            File::open(&chemin)?,
        )))
    } else if dossier.join(format!("{nom}.gz")).is_file() {
        Err(err!("fichier de données compressé (.gz) : non pris en charge, réexporter avec -Z 0 (voir la documentation)"))
    } else {
        Err(err!("fichier de données manquant dans le dossier : {nom}"))
    }
}

/// Lecture d'un dump au format répertoire.
pub fn lire_repertoire(dossier: &Path, f: &mut dyn FnMut(Evt) -> Res<()>) -> Res<()> {
    let (entrees, _) = lire_toc(dossier)?;
    for e in entrees {
        if e.desc == "TABLE DATA" {
            let (schema, table, colonnes) = analyser_copy(e.copy.trim())
                .ok_or_else(|| err!("toc.dat : instruction COPY non reconnue pour une table"))?;
            let nom = e
                .fichier
                .clone()
                .ok_or_else(|| err!("toc.dat : fichier de données absent pour une table"))?;
            f(Evt::CopyDebut {
                schema,
                table,
                colonnes,
            })?;
            let mut lecteur = ouvrir_donnees(dossier, &nom)?;
            let mut tampon = Vec::new();
            loop {
                tampon.clear();
                if lecteur.read_until(b'\n', &mut tampon)? == 0 {
                    break;
                }
                while matches!(tampon.last(), Some(b'\n' | b'\r')) {
                    tampon.pop();
                }
                let ligne = std::str::from_utf8(&tampon)
                    .map_err(|_| err!("données non UTF-8 dans le dossier"))?;
                if ligne == "\\." {
                    break;
                }
                f(Evt::Ligne(decouper_ligne(ligne)))?;
            }
            f(Evt::CopyFin)?;
        } else if e.desc == "BLOBS" || e.desc == "BLOB" {
            return Err(err!("le dump contient des objets binaires PostgreSQL (large objects) : non pris en charge"));
        } else if !e.definition.trim().is_empty() {
            for l in e.definition.lines() {
                f(Evt::Ddl(l.to_string()))?;
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------------------------
// CREATE TABLE : colonnes déclarées
// ----------------------------------------------------------------------------------------------

/// Colonne d'un `CREATE TABLE` lu dans le dump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColonneDump {
    pub nom: String,
    pub type_sql: String,
}

/// Accumule les `CREATE TABLE` vus dans les lignes `Ddl`.
#[derive(Default)]
pub struct CollecteTables {
    courante: Option<(String, Vec<ColonneDump>)>,
    pub tables: Vec<(String, Vec<ColonneDump>)>,
}

impl CollecteTables {
    pub fn ligne(&mut self, l: &str) {
        if let Some((_, cols)) = self.courante.as_mut() {
            let t = l.trim();
            if t.starts_with(')') {
                if let Some(c) = self.courante.take() {
                    self.tables.push(c);
                }
                return;
            }
            let t = t.trim_end_matches(',');
            if t.is_empty() || t.starts_with("CONSTRAINT ") {
                return;
            }
            if let Some((nom, reste)) = identifiant(t) {
                cols.push(ColonneDump {
                    nom,
                    type_sql: type_de_colonne(reste.trim()),
                });
            }
            return;
        }
        if let Some(r) = l.strip_prefix("CREATE TABLE ") {
            if let Some((p, mut reste)) = identifiant(r) {
                let table = if let Some(r2) = reste.strip_prefix('.') {
                    match identifiant(r2) {
                        Some((t, r3)) => {
                            reste = r3;
                            cle_table(&p, &t)
                        }
                        None => return,
                    }
                } else {
                    p
                };
                if reste.trim_start().starts_with('(') {
                    self.courante = Some((table, Vec::new()));
                }
            }
        }
    }
}

/// Type normalisé d'une colonne : retire `NOT NULL`, `DEFAULT …` et la précision.
pub fn type_de_colonne(reste: &str) -> String {
    let mut t = reste;
    for mot in [
        " DEFAULT ",
        " NOT NULL",
        " NULL",
        " GENERATED ",
        " COLLATE ",
        " CONSTRAINT ",
    ] {
        if let Some(i) = t.find(mot) {
            t = &t[..i];
        }
    }
    let t = t.trim();
    // « numeric(12,2) » → « numeric » ; « character varying(20) » → « character varying ».
    let base = match t.find('(') {
        Some(i) => format!(
            "{}{}",
            &t[..i],
            t[i..].split_once(')').map(|x| x.1).unwrap_or("")
        ),
        None => t.to_string(),
    };
    base.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn evenements(s: &str) -> Vec<Evt> {
        let mut v = Vec::new();
        lire_texte(Cursor::new(s.as_bytes().to_vec()), &mut |e| {
            v.push(e);
            Ok(())
        })
        .unwrap();
        v
    }

    #[test]
    fn copy_et_ddl() {
        let e = evenements(
            "SET x = 1;\nCOPY public.eleve (id, \"by\", nom) FROM stdin;\na\t\\N\tb\\tc\n\\.\n\nSELECT 1;\n",
        );
        assert_eq!(e[0], Evt::Ddl("SET x = 1;".into()));
        assert_eq!(
            e[1],
            Evt::CopyDebut {
                schema: "public".into(),
                table: "eleve".into(),
                colonnes: vec!["id".into(), "by".into(), "nom".into()]
            }
        );
        assert_eq!(
            e[2],
            Evt::Ligne(vec![Some("a".into()), None, Some("b\tc".into())])
        );
        assert_eq!(e[3], Evt::CopyFin);
        assert_eq!(e[4], Evt::Ddl(String::new()));
    }

    #[test]
    fn table_entre_guillemets() {
        let (s, t, c) = analyser_copy("COPY public.\"user\" (id, name) FROM stdin;").unwrap();
        assert_eq!((s.as_str(), t.as_str(), c.len()), ("public", "user", 2));
    }

    #[test]
    fn insert_refuse_et_troncature_detectee() {
        assert!(lire_texte(
            Cursor::new(b"INSERT INTO public.x VALUES (1);\n".to_vec()),
            &mut |_| Ok(())
        )
        .is_err());
        assert!(lire_texte(
            Cursor::new(b"COPY public.x (a) FROM stdin;\n1\n".to_vec()),
            &mut |_| Ok(())
        )
        .is_err());
    }

    #[test]
    fn colonnes_d_un_create_table() {
        let mut c = CollecteTables::default();
        for l in [
            "CREATE TABLE public.\"user\" (",
            "    id text NOT NULL,",
            "    created_at timestamp with time zone DEFAULT now() NOT NULL,",
            "    montant numeric(12,2),",
            "    \"by\" text,",
            "    CONSTRAINT x CHECK ((id <> ''::text))",
            ");",
        ] {
            c.ligne(l);
        }
        assert_eq!(c.tables.len(), 1);
        let (t, cols) = &c.tables[0];
        assert_eq!(t, "user");
        let noms: Vec<_> = cols
            .iter()
            .map(|c| (c.nom.as_str(), c.type_sql.as_str()))
            .collect();
        assert_eq!(
            noms,
            vec![
                ("id", "text"),
                ("created_at", "timestamp with time zone"),
                ("montant", "numeric"),
                ("by", "text")
            ]
        );
    }
}
