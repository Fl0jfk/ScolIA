//! Client **ClamAV** écrit à la main (protocole `clamd`, commande `INSTREAM`), sans dépendance (D-062 : « client
//! ClamAV (protocole INSTREAM) : maison ; ClamAV tourne comme service séparé »). Socle 7, lot D5.
//!
//! Protocole (`man clamd`, commandes préfixées par `z` : terminées par un octet nul) :
//!
//! ```text
//! client → zINSTREAM\0
//! client → [longueur u32 grand-boutiste][octets] … (morceaux)
//! client → [0u32]                                   (fin du flux)
//! clamd  → stream: OK\0                              (sain)
//! clamd  → stream: Eicar-Test-Signature FOUND\0      (menace)
//! clamd  → INSTREAM size limit exceeded. ERROR\0     (flux trop gros pour le démon)
//! ```
//!
//! **Fermé par défaut** (D18) : toute réponse inattendue, toute erreur de réseau, tout délai dépassé, toute réponse
//! `ERROR` rend [`VerdictAnalyse::Indisponible`] — jamais « sain ». Seul `stream: OK` vaut « sain ». Le nom de la
//! signature est assaini (borné, caractères imprimables) avant d'être rendu.
//!
//! Hôte : le démon tourne dans un conteneur séparé, joint par TCP (`CLAMAV_ADDR`, ex. `clamav:3310`) ; la base de
//! signatures est tenue à jour par le démon lui-même (un hôte réseau à autoriser côté environnement).

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::upload::{AnalyseurAntivirus, ContexteAnalyse, VerdictAnalyse};

/// Taille d'un morceau envoyé au démon (le démon accepte jusqu'à `StreamMaxLength`, 25 Mio par défaut).
const MORCEAU: usize = 64 * 1024;
/// Réponse la plus longue lue : un nom de signature ne dépasse pas quelques centaines d'octets.
const REPONSE_MAX: usize = 1024;

/// Réglages du client.
#[derive(Debug, Clone)]
pub struct ClamAv {
    adresse: String,
    delai_connexion: Duration,
    delai_total: Duration,
}

impl ClamAv {
    /// `adresse` : `hôte:port` du démon (`clamd` écoute par défaut sur 3310).
    pub fn nouveau(adresse: impl Into<String>) -> Self {
        Self {
            adresse: adresse.into(),
            delai_connexion: Duration::from_secs(3),
            delai_total: Duration::from_secs(30),
        }
    }

    /// `CLAMAV_ADDR` : absente ou vide → `None` (aucun antivirus configuré : les pièces restent en quarantaine).
    pub fn depuis_env() -> Option<Self> {
        let a = std::env::var("CLAMAV_ADDR").ok()?;
        let a = a.trim();
        (!a.is_empty()).then(|| Self::nouveau(a))
    }

    pub fn avec_delais(mut self, connexion: Duration, total: Duration) -> Self {
        self.delai_connexion = connexion;
        self.delai_total = total;
        self
    }

    fn connecter(&self) -> std::io::Result<TcpStream> {
        let mut derniere = None;
        for adr in self.adresse.to_socket_addrs()? {
            match TcpStream::connect_timeout(&adr, self.delai_connexion) {
                Ok(s) => {
                    s.set_read_timeout(Some(self.delai_total))?;
                    s.set_write_timeout(Some(self.delai_total))?;
                    s.set_nodelay(true)?;
                    return Ok(s);
                }
                Err(e) => derniere = Some(e),
            }
        }
        Err(derniere.unwrap_or_else(|| std::io::Error::other("adresse sans résolution")))
    }

    /// `PING` : le démon répond `PONG`. Sert aux sondes de santé, pas à décider d'un dépôt.
    pub fn ping(&self) -> bool {
        let Ok(mut s) = self.connecter() else { return false };
        if s.write_all(b"zPING\0").is_err() {
            return false;
        }
        lire_reponse(&mut s).is_some_and(|r| r == "PONG")
    }

    /// Analyse un flux d'octets. Voir le module pour la règle « fermé par défaut ».
    pub fn analyser_octets(&self, octets: &[u8]) -> VerdictAnalyse {
        match self.analyser_interne(octets) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(erreur = %e, "antivirus : analyse impossible, pièce maintenue en quarantaine");
                VerdictAnalyse::Indisponible
            }
        }
    }

    fn analyser_interne(&self, octets: &[u8]) -> std::io::Result<VerdictAnalyse> {
        let mut s = self.connecter()?;
        s.write_all(b"zINSTREAM\0")?;
        for morceau in octets.chunks(MORCEAU) {
            s.write_all(&(morceau.len() as u32).to_be_bytes())?;
            s.write_all(morceau)?;
        }
        s.write_all(&0u32.to_be_bytes())?;
        s.flush()?;
        let Some(reponse) = lire_reponse(&mut s) else {
            return Ok(VerdictAnalyse::Indisponible);
        };
        Ok(interpreter(&reponse))
    }
}

/// Lit jusqu'au premier octet nul (ou la fin du flux) ; `None` si rien de lisible ou réponse démesurée.
fn lire_reponse(s: &mut TcpStream) -> Option<String> {
    let mut tampon = Vec::with_capacity(128);
    let mut un = [0u8; 1];
    loop {
        match s.read(&mut un) {
            Ok(0) => break,
            Ok(_) => {
                if un[0] == 0 {
                    break;
                }
                tampon.push(un[0]);
                if tampon.len() > REPONSE_MAX {
                    return None;
                }
            }
            Err(_) => return None,
        }
    }
    if tampon.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&tampon).trim().to_string())
}

/// Seul `stream: OK` est « sain » ; `… FOUND` nomme la menace ; tout le reste (`ERROR`, texte inconnu) est « indisponible ».
fn interpreter(reponse: &str) -> VerdictAnalyse {
    let r = reponse.trim();
    if r == "stream: OK" {
        return VerdictAnalyse::Sain;
    }
    if let Some(corps) = r.strip_suffix(" FOUND") {
        let nom = corps.strip_prefix("stream: ").unwrap_or(corps);
        let nom: String = nom.chars().filter(|c| !c.is_control()).take(80).collect();
        return VerdictAnalyse::Infecte(if nom.trim().is_empty() { "menace".into() } else { nom });
    }
    VerdictAnalyse::Indisponible
}

impl AnalyseurAntivirus for ClamAv {
    fn nom(&self) -> &'static str {
        "clamav"
    }

    fn analyser(&self, ctx: &ContexteAnalyse<'_>) -> VerdictAnalyse {
        self.analyser_octets(ctx.octets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    /// Faux démon : lit le protocole INSTREAM exactement comme `clamd`, réassemble le flux, puis répond ce que décide
    /// `reponse` à partir du contenu reçu.
    fn faux_demon(reponse: impl Fn(&[u8]) -> Option<Vec<u8>> + Send + 'static) -> (String, thread::JoinHandle<Vec<u8>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let adr = l.local_addr().unwrap().to_string();
        let h = thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut commande = Vec::new();
            let mut b = [0u8; 1];
            loop {
                s.read_exact(&mut b).unwrap();
                commande.push(b[0]);
                if b[0] == 0 {
                    break;
                }
            }
            if commande == b"zPING\0" {
                s.write_all(b"PONG\0").unwrap();
                return commande;
            }
            assert_eq!(commande, b"zINSTREAM\0", "commande attendue");
            let mut flux = Vec::new();
            loop {
                let mut n = [0u8; 4];
                s.read_exact(&mut n).unwrap();
                let n = u32::from_be_bytes(n) as usize;
                if n == 0 {
                    break;
                }
                let mut m = vec![0u8; n];
                s.read_exact(&mut m).unwrap();
                flux.extend_from_slice(&m);
            }
            if let Some(r) = reponse(&flux) {
                s.write_all(&r).unwrap();
            }
            flux
        });
        (adr, h)
    }

    const EICAR: &[u8] =
        b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*";

    fn reponse_type(flux: &[u8]) -> Option<Vec<u8>> {
        if flux.windows(EICAR.len()).any(|w| w == EICAR) {
            Some(b"stream: Eicar-Test-Signature FOUND\0".to_vec())
        } else {
            Some(b"stream: OK\0".to_vec())
        }
    }

    #[test]
    fn fichier_sain() {
        let (adr, h) = faux_demon(reponse_type);
        let v = ClamAv::nouveau(adr).analyser_octets(b"%PDF-1.4 un document banal");
        assert_eq!(v, VerdictAnalyse::Sain);
        assert_eq!(h.join().unwrap(), b"%PDF-1.4 un document banal");
    }

    #[test]
    fn eicar_est_detecte() {
        let (adr, h) = faux_demon(reponse_type);
        let v = ClamAv::nouveau(adr).analyser_octets(EICAR);
        assert_eq!(v, VerdictAnalyse::Infecte("Eicar-Test-Signature".into()));
        assert_eq!(h.join().unwrap(), EICAR, "le démon a reçu le fichier entier");
    }

    #[test]
    fn gros_fichier_envoye_en_plusieurs_morceaux() {
        let (adr, h) = faux_demon(reponse_type);
        let gros: Vec<u8> = (0..(MORCEAU * 3 + 17)).map(|i| (i % 251) as u8).collect();
        assert_eq!(ClamAv::nouveau(adr).analyser_octets(&gros), VerdictAnalyse::Sain);
        assert_eq!(h.join().unwrap(), gros);
    }

    #[test]
    fn fichier_vide_est_un_flux_valide() {
        let (adr, h) = faux_demon(reponse_type);
        assert_eq!(ClamAv::nouveau(adr).analyser_octets(b""), VerdictAnalyse::Sain);
        assert!(h.join().unwrap().is_empty());
    }

    #[test]
    fn demon_arrete_ferme_par_defaut() {
        // Port fermé : on réserve un port puis on le libère.
        let adr = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().to_string()
        };
        let c = ClamAv::nouveau(adr).avec_delais(Duration::from_millis(300), Duration::from_millis(300));
        assert_eq!(c.analyser_octets(b"x"), VerdictAnalyse::Indisponible);
        assert!(!c.ping());
    }

    #[test]
    fn reponse_erreur_ou_inconnue_n_est_jamais_saine() {
        for r in [
            &b"INSTREAM size limit exceeded. ERROR\0"[..],
            b"stream: OK extra\0",
            b"stream: ok\0",
            b"quelque chose\0",
            b"\0",
        ] {
            let r = r.to_vec();
            let (adr, h) = faux_demon(move |_| Some(r.clone()));
            assert_eq!(ClamAv::nouveau(adr).analyser_octets(b"x"), VerdictAnalyse::Indisponible);
            let _ = h.join();
        }
    }

    #[test]
    fn silence_du_demon_delai_depasse() {
        let (adr, h) = faux_demon(|_| None); // lit tout, ne répond jamais, ferme
        let c = ClamAv::nouveau(adr).avec_delais(Duration::from_secs(1), Duration::from_millis(300));
        assert_eq!(c.analyser_octets(b"x"), VerdictAnalyse::Indisponible);
        let _ = h.join();
    }

    #[test]
    fn nom_de_signature_assaini() {
        let v = interpreter("stream: Win.Test\u{7}.Évil FOUND");
        assert!(matches!(v, VerdictAnalyse::Infecte(ref n) if !n.chars().any(char::is_control)));
        let long = format!("stream: {} FOUND", "A".repeat(500));
        assert!(matches!(interpreter(&long), VerdictAnalyse::Infecte(n) if n.len() <= 80));
    }

    #[test]
    fn ping() {
        let (adr, h) = faux_demon(|_| None);
        assert!(ClamAv::nouveau(adr).ping());
        let _ = h.join();
    }

    #[test]
    fn depuis_env_absent() {
        // On ne touche pas à l'environnement du processus (tests parallèles) : on vérifie la règle de rognage.
        assert!(ClamAv::nouveau("clamav:3310").adresse == "clamav:3310");
    }
}
