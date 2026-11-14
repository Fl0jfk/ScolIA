//! Écriture bas niveau d'un fichier PDF 1.4 : objets numérotés, table xref exacte, trailer.

pub(crate) struct Ecrivain {
    pub sortie: Vec<u8>,
    offsets: Vec<usize>,
}

impl Ecrivain {
    pub fn nouveau() -> Ecrivain {
        Ecrivain { sortie: b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec(), offsets: Vec::new() }
    }

    pub fn objet(&mut self, corps: &[u8]) -> usize {
        self.offsets.push(self.sortie.len());
        let n = self.offsets.len();
        self.sortie.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        self.sortie.extend_from_slice(corps);
        self.sortie.extend_from_slice(b"\nendobj\n");
        n
    }

    /// Objet-flux : `dict` = entrées du dictionnaire (sans `/Length`), `donnees` = octets bruts.
    pub fn flux(&mut self, dict: &str, donnees: &[u8]) -> usize {
        let mut c = format!("<< {dict} /Length {} >>\nstream\n", donnees.len()).into_bytes();
        c.extend_from_slice(donnees);
        c.extend_from_slice(b"\nendstream");
        self.objet(&c)
    }

    pub fn terminer(mut self, racine: usize, info: usize) -> Vec<u8> {
        let xref = self.sortie.len();
        let n = self.offsets.len() + 1;
        self.sortie.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
        for o in &self.offsets {
            self.sortie.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        self.sortie.extend_from_slice(
            format!("trailer\n<< /Size {n} /Root {racine} 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n")
                .as_bytes(),
        );
        self.sortie
    }
}

/// Chaîne littérale PDF `( … )` depuis des octets WinAnsi.
pub(crate) fn litterale(octets: &[u8], sortie: &mut Vec<u8>) {
    sortie.push(b'(');
    for &b in octets {
        match b {
            b'(' | b')' | b'\\' => {
                sortie.push(b'\\');
                sortie.push(b);
            }
            32..=126 => sortie.push(b),
            _ => sortie.extend_from_slice(format!("\\{b:03o}").as_bytes()),
        }
    }
    sortie.push(b')');
}

/// Chaîne de métadonnée : ASCII littéral, sinon UTF-16BE avec BOM en hexadécimal.
pub(crate) fn chaine_texte(s: &str) -> Vec<u8> {
    let mut o = Vec::new();
    if s.chars().all(|c| (' '..='~').contains(&c)) {
        litterale(s.as_bytes(), &mut o);
    } else {
        o.extend_from_slice(b"<FEFF");
        for u in s.encode_utf16() {
            o.extend_from_slice(format!("{u:04X}").as_bytes());
        }
        o.push(b'>');
    }
    o
}
