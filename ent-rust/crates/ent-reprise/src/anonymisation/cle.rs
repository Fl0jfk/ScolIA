//! Clé locale de pseudonymisation (HMAC-SHA256).
//!
//! La clé est générée **sur le poste de Florian**, jamais transmise, jamais écrite dans un rapport
//! (seule une empreinte courte, non réversible, permet de reconnaître la clé utilisée). Sans la clé,
//! on ne peut ni retrouver une valeur réelle depuis son pseudonyme, ni refaire la correspondance.

use crate::{err, Res};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

type HmacSha256 = Hmac<Sha256>;

/// Taille minimale de la clé, en octets (après décodage hexadécimal si c'en est).
pub const TAILLE_MIN: usize = 32;

pub struct Cle(Vec<u8>);

impl std::fmt::Debug for Cle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Cle(<masquée>)")
    }
}

impl Cle {
    /// Lit la clé dans un fichier : 64 caractères hexadécimaux ou plus, ou tout texte d'au moins 32 octets.
    pub fn depuis_fichier(p: &Path) -> Res<Cle> {
        let brut =
            std::fs::read_to_string(p).map_err(|e| err!("fichier de clé illisible : {e}"))?;
        Self::depuis_texte(brut.trim())
    }

    pub fn depuis_texte(t: &str) -> Res<Cle> {
        let octets = if t.len() >= 2 * TAILLE_MIN
            && t.len().is_multiple_of(2)
            && t.chars().all(|c| c.is_ascii_hexdigit())
        {
            hex::decode(t).map_err(|_| err!("clé hexadécimale invalide"))?
        } else {
            t.as_bytes().to_vec()
        };
        if octets.len() < TAILLE_MIN {
            return Err(err!(
                "clé trop courte ({} octets, minimum {TAILLE_MIN}) : générez-en une avec « ent-reprise generer-cle »",
                octets.len()
            ));
        }
        Ok(Cle(octets))
    }

    /// Génère une clé aléatoire de 32 octets dans un **nouveau** fichier (refuse d'écraser), lisible
    /// par le seul propriétaire sous Unix.
    pub fn generer(p: &Path) -> Res<()> {
        if p.exists() {
            return Err(err!("{} existe déjà : refus de l'écraser (une clé perdue rend deux exports incomparables)", p.display()));
        }
        let mut octets = [0u8; 32];
        getrandom::getrandom(&mut octets).map_err(|e| err!("aléa système indisponible : {e}"))?;
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts
            .open(p)
            .map_err(|e| err!("création du fichier de clé impossible : {e}"))?;
        f.write_all(hex::encode(octets).as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }

    /// Empreinte courte et non réversible, pour reconnaître la clé utilisée sans la révéler.
    pub fn empreinte(&self) -> String {
        let mut h = Sha256::new();
        h.update(b"ent-reprise/empreinte-de-cle/v1");
        h.update(&self.0);
        let d = h.finalize();
        // Double hachage : l'empreinte ne permet pas de tester une clé candidate bon marché.
        let d2 = Sha256::digest(d);
        hex::encode(&d2[..6])
    }

    /// MAC d'une valeur pour une nature donnée et un compteur (flux déterministe).
    pub(crate) fn mac(&self, etiquette: &str, valeur: &[u8], compteur: u32) -> [u8; 32] {
        let mut m =
            <HmacSha256 as Mac>::new_from_slice(&self.0).expect("HMAC accepte toute longueur");
        m.update(etiquette.as_bytes());
        m.update(&[0]);
        m.update(valeur);
        m.update(&[0]);
        m.update(&compteur.to_le_bytes());
        m.finalize().into_bytes().into()
    }
}

/// Flux d'octets déterministe issu de (clé, étiquette, valeur).
pub struct Flux<'a> {
    cle: &'a Cle,
    etiquette: &'a str,
    valeur: &'a [u8],
    compteur: u32,
    tampon: [u8; 32],
    pos: usize,
}

impl<'a> Flux<'a> {
    pub fn nouveau(cle: &'a Cle, etiquette: &'a str, valeur: &'a str) -> Flux<'a> {
        Flux {
            cle,
            etiquette,
            valeur: valeur.as_bytes(),
            compteur: 0,
            tampon: [0; 32],
            pos: 32,
        }
    }

    pub fn octet(&mut self) -> u8 {
        if self.pos >= 32 {
            self.tampon = self.cle.mac(self.etiquette, self.valeur, self.compteur);
            self.compteur += 1;
            self.pos = 0;
        }
        let o = self.tampon[self.pos];
        self.pos += 1;
        o
    }

    /// Entier uniforme dans `0..n` (n > 0), sans biais notable (16 bits).
    pub fn tirer(&mut self, n: usize) -> usize {
        let v = (usize::from(self.octet()) << 8) | usize::from(self.octet());
        v % n.max(1)
    }

    pub fn chiffre(&mut self) -> char {
        (b'0' + self.tirer(10) as u8) as char
    }

    pub fn lettre_minuscule(&mut self) -> char {
        (b'a' + self.tirer(26) as u8) as char
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cle(n: u8) -> Cle {
        Cle::depuis_texte(&format!("{:02x}", n).repeat(32)).unwrap()
    }

    #[test]
    fn deterministe_et_dependant_de_la_cle() {
        let (a, b) = (cle(1), cle(2));
        let mut f1 = Flux::nouveau(&a, "nom", "Dupont");
        let mut f2 = Flux::nouveau(&a, "nom", "Dupont");
        let mut f3 = Flux::nouveau(&b, "nom", "Dupont");
        let mut f4 = Flux::nouveau(&a, "nom", "Durand");
        let s1: Vec<u8> = (0..40).map(|_| f1.octet()).collect();
        assert_eq!(s1, (0..40).map(|_| f2.octet()).collect::<Vec<_>>());
        assert_ne!(s1, (0..40).map(|_| f3.octet()).collect::<Vec<_>>());
        assert_ne!(s1, (0..40).map(|_| f4.octet()).collect::<Vec<_>>());
    }

    #[test]
    fn cle_courte_refusee_et_debug_masque() {
        assert!(Cle::depuis_texte("trop-courte").is_err());
        let c = cle(7);
        assert!(!format!("{c:?}").contains("07"));
    }

    #[test]
    fn empreinte_courte_et_stable() {
        assert_eq!(cle(1).empreinte(), cle(1).empreinte());
        assert_ne!(cle(1).empreinte(), cle(2).empreinte());
        assert_eq!(cle(1).empreinte().len(), 12);
    }

    #[test]
    fn generation_refuse_d_ecraser() {
        let p = std::env::temp_dir().join(format!("ent-reprise-cle-{}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        Cle::generer(&p).unwrap();
        assert!(Cle::generer(&p).is_err());
        assert!(Cle::depuis_fichier(&p).is_ok());
        let _ = std::fs::remove_file(&p);
    }
}
