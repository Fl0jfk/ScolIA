//! Archive ZIP **sans compression** (méthode « stored ») écrite à la main : l'export du coffre (socle 7, lot D4,
//! DCT-85) livre des PDF déjà compressés, la compression n'apporterait rien et le format reste lisible par tous les
//! outils. D-062 : aucune dépendance. Limites assumées : pas de ZIP64 (4 Gio et 65 535 entrées au plus), noms en UTF-8
//! (drapeau 11), horodatage fixe (1980-01-01) pour que deux exports du même contenu soient identiques octet pour octet.

const SIGNATURE_LOCAL: u32 = 0x0403_4b50;
const SIGNATURE_CENTRAL: u32 = 0x0201_4b50;
const SIGNATURE_FIN: u32 = 0x0605_4b50;
/// 1980-01-01 00:00:00 (format MS-DOS).
const DOS_DATE: u16 = 0x0021;
const DOS_TIME: u16 = 0;

fn table_crc() -> [u32; 256] {
    let mut t = [0u32; 256];
    for (i, e) in t.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *e = c;
    }
    t
}

/// CRC-32 (IEEE 802.3), celui que le format ZIP exige.
pub fn crc32(octets: &[u8]) -> u32 {
    let t = table_crc();
    let mut c = 0xFFFF_FFFFu32;
    for &b in octets {
        c = t[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErreurZip {
    /// Plus de 65 535 entrées, ou un fichier de plus de 4 Gio : ZIP64 n'est pas pris en charge.
    TropGros,
    /// Nom vide, absolu, avec `..` ou contenant un caractère de contrôle ou une barre oblique inverse.
    NomInvalide,
    /// Deux entrées de même nom.
    NomEnDouble,
}

/// Construit une archive : les entrées s'ajoutent dans l'ordre, `terminer` rend les octets.
#[derive(Default)]
pub struct EcrivainZip {
    corps: Vec<u8>,
    centrale: Vec<u8>,
    noms: std::collections::HashSet<String>,
    n: usize,
}

fn nom_sur(nom: &str) -> bool {
    !nom.is_empty()
        && nom.len() <= 250
        && !nom.starts_with('/')
        && !nom.contains('\\')
        && !nom.chars().any(char::is_control)
        && nom
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..")
}

impl EcrivainZip {
    pub fn nouveau() -> Self {
        Self::default()
    }

    pub fn ajouter(&mut self, nom: &str, octets: &[u8]) -> Result<(), ErreurZip> {
        if !nom_sur(nom) {
            return Err(ErreurZip::NomInvalide);
        }
        if self.n >= 65_535
            || octets.len() as u64 >= u32::MAX as u64
            || self.corps.len() as u64 >= u32::MAX as u64 - 1_000_000
        {
            return Err(ErreurZip::TropGros);
        }
        if !self.noms.insert(nom.to_string()) {
            return Err(ErreurZip::NomEnDouble);
        }
        let crc = crc32(octets);
        let taille = octets.len() as u32;
        let decalage = self.corps.len() as u32;
        let nom_b = nom.as_bytes();
        // En-tête local.
        self.corps.extend_from_slice(&SIGNATURE_LOCAL.to_le_bytes());
        self.corps.extend_from_slice(&20u16.to_le_bytes()); // version nécessaire
        self.corps.extend_from_slice(&0x0800u16.to_le_bytes()); // drapeau : noms UTF-8
        self.corps.extend_from_slice(&0u16.to_le_bytes()); // méthode : stored
        self.corps.extend_from_slice(&DOS_TIME.to_le_bytes());
        self.corps.extend_from_slice(&DOS_DATE.to_le_bytes());
        self.corps.extend_from_slice(&crc.to_le_bytes());
        self.corps.extend_from_slice(&taille.to_le_bytes());
        self.corps.extend_from_slice(&taille.to_le_bytes());
        self.corps
            .extend_from_slice(&(nom_b.len() as u16).to_le_bytes());
        self.corps.extend_from_slice(&0u16.to_le_bytes());
        self.corps.extend_from_slice(nom_b);
        self.corps.extend_from_slice(octets);
        // Entrée du répertoire central.
        self.centrale
            .extend_from_slice(&SIGNATURE_CENTRAL.to_le_bytes());
        self.centrale.extend_from_slice(&20u16.to_le_bytes()); // version créatrice
        self.centrale.extend_from_slice(&20u16.to_le_bytes()); // version nécessaire
        self.centrale.extend_from_slice(&0x0800u16.to_le_bytes());
        self.centrale.extend_from_slice(&0u16.to_le_bytes());
        self.centrale.extend_from_slice(&DOS_TIME.to_le_bytes());
        self.centrale.extend_from_slice(&DOS_DATE.to_le_bytes());
        self.centrale.extend_from_slice(&crc.to_le_bytes());
        self.centrale.extend_from_slice(&taille.to_le_bytes());
        self.centrale.extend_from_slice(&taille.to_le_bytes());
        self.centrale
            .extend_from_slice(&(nom_b.len() as u16).to_le_bytes());
        self.centrale.extend_from_slice(&0u16.to_le_bytes()); // extra
        self.centrale.extend_from_slice(&0u16.to_le_bytes()); // commentaire
        self.centrale.extend_from_slice(&0u16.to_le_bytes()); // disque
        self.centrale.extend_from_slice(&0u16.to_le_bytes()); // attributs internes
        self.centrale.extend_from_slice(&0u32.to_le_bytes()); // attributs externes
        self.centrale.extend_from_slice(&decalage.to_le_bytes());
        self.centrale.extend_from_slice(nom_b);
        self.n += 1;
        Ok(())
    }

    pub fn terminer(mut self) -> Vec<u8> {
        let debut_central = self.corps.len() as u32;
        let taille_central = self.centrale.len() as u32;
        self.corps.extend_from_slice(&self.centrale);
        self.corps.extend_from_slice(&SIGNATURE_FIN.to_le_bytes());
        self.corps.extend_from_slice(&0u16.to_le_bytes());
        self.corps.extend_from_slice(&0u16.to_le_bytes());
        self.corps.extend_from_slice(&(self.n as u16).to_le_bytes());
        self.corps.extend_from_slice(&(self.n as u16).to_le_bytes());
        self.corps.extend_from_slice(&taille_central.to_le_bytes());
        self.corps.extend_from_slice(&debut_central.to_le_bytes());
        self.corps.extend_from_slice(&0u16.to_le_bytes());
        self.corps
    }
}

/// Relit une archive produite par [`EcrivainZip`] (méthode « stored » seulement) : `(nom, octets)` dans l'ordre, après
/// **vérification du CRC** de chaque entrée. Sert aux tests et à la relecture d'un export.
pub fn lire_zip_stored(z: &[u8]) -> Result<Vec<(String, Vec<u8>)>, &'static str> {
    let u16_at = |o: usize| -> Result<u16, &'static str> {
        z.get(o..o + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .ok_or("tronqué")
    };
    let u32_at = |o: usize| -> Result<u32, &'static str> {
        z.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or("tronqué")
    };
    if z.len() < 22 {
        return Err("trop court");
    }
    let fin = z.len() - 22;
    if u32_at(fin)? != SIGNATURE_FIN {
        return Err("fin de répertoire absente");
    }
    let n = u16_at(fin + 10)? as usize;
    let mut o = u32_at(fin + 16)? as usize;
    let mut sortie = Vec::with_capacity(n);
    for _ in 0..n {
        if u32_at(o)? != SIGNATURE_CENTRAL {
            return Err("entrée centrale invalide");
        }
        if u16_at(o + 10)? != 0 {
            return Err("méthode non prise en charge");
        }
        let crc = u32_at(o + 16)?;
        let taille = u32_at(o + 24)? as usize;
        let l_nom = u16_at(o + 28)? as usize;
        let l_extra = u16_at(o + 30)? as usize;
        let l_com = u16_at(o + 32)? as usize;
        let local = u32_at(o + 42)? as usize;
        let nom = std::str::from_utf8(z.get(o + 46..o + 46 + l_nom).ok_or("tronqué")?)
            .map_err(|_| "nom non UTF-8")?
            .to_string();
        if u32_at(local)? != SIGNATURE_LOCAL {
            return Err("entrée locale invalide");
        }
        let l_nom_l = u16_at(local + 26)? as usize;
        let l_extra_l = u16_at(local + 28)? as usize;
        let debut = local + 30 + l_nom_l + l_extra_l;
        let donnees = z.get(debut..debut + taille).ok_or("tronqué")?;
        if crc32(donnees) != crc {
            return Err("CRC différent");
        }
        sortie.push((nom, donnees.to_vec()));
        o += 46 + l_nom + l_extra + l_com;
    }
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_valeur_de_reference() {
        // Valeur connue du CRC-32 IEEE de « 123456789 ».
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn aller_retour_et_determinisme() {
        let faire = || {
            let mut z = EcrivainZip::nouveau();
            z.ajouter("index.csv", "nom;titre\nÉlodie;œuvre\n".as_bytes())
                .unwrap();
            z.ajouter("documents/2026-SCO-000001.pdf", b"%PDF-1.4 test")
                .unwrap();
            z.ajouter("vide.txt", b"").unwrap();
            z.terminer()
        };
        let a = faire();
        assert_eq!(a, faire(), "deux exports du même contenu sont identiques");
        let lu = lire_zip_stored(&a).unwrap();
        assert_eq!(lu.len(), 3);
        assert_eq!(lu[0].0, "index.csv");
        assert_eq!(lu[0].1, "nom;titre\nÉlodie;œuvre\n".as_bytes());
        assert_eq!(lu[1].1, b"%PDF-1.4 test");
        assert!(lu[2].1.is_empty());
    }

    #[test]
    fn noms_dangereux_refuses() {
        let mut z = EcrivainZip::nouveau();
        for n in [
            "",
            "/etc/passwd",
            "../x",
            "a/../b",
            "a\\b",
            "a//b",
            "a\nb",
            "./x",
        ] {
            assert_eq!(z.ajouter(n, b"x"), Err(ErreurZip::NomInvalide), "{n:?}");
        }
        z.ajouter("ok.pdf", b"x").unwrap();
        assert_eq!(z.ajouter("ok.pdf", b"y"), Err(ErreurZip::NomEnDouble));
    }

    #[test]
    fn corruption_detectee() {
        let mut z = EcrivainZip::nouveau();
        z.ajouter("a.txt", b"bonjour").unwrap();
        let mut o = z.terminer();
        let pos = o.windows(7).position(|w| w == b"bonjour").unwrap();
        o[pos] = b'B';
        assert_eq!(lire_zip_stored(&o), Err("CRC différent"));
    }
}
