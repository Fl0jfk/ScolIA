//! Socle 7, lot D3 : **rendu PDF des attestations** sur le moteur maison `ent-pdf` (D-062, lot D1) — branche du port
//! [`RenduDocument`]. Famille « attestation » (§9.3) : titre, phrase fixe, cartouche de données, lieu-date, mention
//! « par délégation », code de vérification et QR, pied de page de l'établissement.
//!
//! Le moteur ne reçoit que des **valeurs** : aucun texte venant des données n'est interprété. Le rendu est déterministe
//! (aucune date de création dans les métadonnées : même données, même fichier). Limites du moteur D1, reprises ici :
//! police TrueType incorporée en sous-ensemble (D1b : tout nom latin, grec ou cyrillique ; pas de PDF/A ni de PDF/UA), pas de couleur, pas de logo (le style et la couleur
//! principale de l'identité sont transmis mais pas encore appliqués), français seul.

use ent_pdf::{Align, Colonne, Document, Ligne, Meta, Tableau};

use crate::documents::*;
use crate::documents_natures::{
    CLE_ANNEE, CLE_ENTREE, CLE_NAISSANCE, CLE_NIVEAU, CLE_NOM, CLE_PRENOM,
};

/// Le moteur de rendu des natures de la famille « attestation » (certificat de scolarité, attestation d'inscription).
#[derive(Debug, Default, Clone, Copy)]
pub struct RenduPdfAttestation;

fn valeur<'a>(d: &'a DemandeRendu<'_>, cle: &str) -> Option<&'a str> {
    d.donnees
        .iter()
        .find(|x| x.cle == cle)
        .map(|x| x.valeur.as_str())
        .filter(|v| !v.trim().is_empty())
}

/// `2012-03-10` → `10/03/2012` ; toute autre forme est rendue telle quelle.
fn date_fr(iso: &str) -> String {
    let p: Vec<&str> = iso.split('-').collect();
    match p.as_slice() {
        [a, m, j] if a.len() == 4 && m.len() == 2 && j.len() == 2 => format!("{j}/{m}/{a}"),
        _ => iso.to_string(),
    }
}

impl RenduDocument for RenduPdfAttestation {
    fn rendre(&self, d: &DemandeRendu<'_>) -> Result<DocumentRendu, ErreurRendu> {
        if d.nature.famille != FamilleTrame::Attestation {
            return Err(ErreurRendu::Impossible(
                "famille de nature non prise en charge par ce moteur".into(),
            ));
        }
        let nom = valeur(d, CLE_NOM).ok_or_else(|| ErreurRendu::Impossible("nom absent".into()))?;
        let prenom =
            valeur(d, CLE_PRENOM).ok_or_else(|| ErreurRendu::Impossible("prénom absent".into()))?;
        let naissance = valeur(d, CLE_NAISSANCE)
            .map(date_fr)
            .ok_or_else(|| ErreurRendu::Impossible("naissance absente".into()))?;
        let meta = Meta {
            titre: d.nature.libelle.to_string(),
            auteur: d.entite_raison_sociale.to_string(),
            creation: None,
        };
        let mut pied = d.mentions_pied.unwrap_or("").to_string();
        if !pied.is_empty() {
            pied.push_str(" — ");
        }
        pied.push_str(&format!("N° {}", d.numero_affiche));
        let mut doc = Document::new(meta)
            .avec_en_tete(d.entite_raison_sociale)
            .avec_pied(&pied);
        doc.titre(d.nature.libelle);
        doc.texte(
            &format!("N° {} — version {}", d.numero_affiche, d.numero_version),
            ent_pdf::Police::Helvetica,
            9.5,
            Align::Gauche,
        );
        doc.espace(14.0);
        let phrase = if d.nature.code == ATTESTATION_INSCRIPTION.code {
            format!(
                "Nous attestons que {prenom} {} est inscrit(e) dans notre établissement.",
                nom.to_uppercase()
            )
        } else {
            format!("Nous certifions que {prenom} {} est scolarisé(e) dans notre établissement à la date de ce document.", nom.to_uppercase())
        };
        doc.paragraphe(&phrase);
        doc.espace(10.0);
        let mut t = Tableau::new(vec![
            Colonne::new("Information", 1.0, Align::Gauche),
            Colonne::new("Valeur", 2.0, Align::Gauche),
        ]);
        t.lignes
            .push(Ligne::new(vec!["Nom".to_string(), nom.to_uppercase()]));
        t.lignes
            .push(Ligne::new(vec!["Prénom".to_string(), prenom.to_string()]));
        t.lignes
            .push(Ligne::new(vec!["Date de naissance".to_string(), naissance]));
        if let Some(n) = valeur(d, CLE_NIVEAU) {
            t.lignes
                .push(Ligne::new(vec!["Niveau".to_string(), n.to_string()]));
        }
        if let Some(a) = valeur(d, CLE_ANNEE) {
            t.lignes.push(Ligne::new(vec![
                "Année scolaire".to_string(),
                a.to_string(),
            ]));
        }
        if let Some(e) = valeur(d, CLE_ENTREE) {
            t.lignes
                .push(Ligne::new(vec!["Date d'entrée".to_string(), date_fr(e)]));
        }
        doc.tableau(&t);
        for (_, texte) in d.textes_encadres {
            doc.espace(8.0);
            doc.paragraphe(texte);
        }
        doc.espace(14.0);
        doc.paragraphe(&format!("Établi le {}.", date_fr(d.date_emission)));
        if let Some(m) = d.motif {
            doc.paragraphe(&format!(
                "Cette version remplace la précédente. Motif : {m}."
            ));
        }
        if let Some(p) = d.par_delegation_de {
            doc.paragraphe(&format!("Par délégation de {p}."));
        }
        if let Some(n) = d.destinataire_nom {
            doc.paragraphe(&format!("Exemplaire remis à : {n}."));
        }
        if let (Some(code), Some(url)) = (d.code_verification, d.url_verification) {
            doc.espace(14.0);
            doc.paragraphe(&format!(
                "Vérifier l'authenticité de ce document : {url} — code {code}."
            ));
            doc.qr(url, 90.0, Align::Gauche)
                .map_err(|e| ErreurRendu::Impossible(e.to_string()))?;
        }
        let octets = doc.octets();
        let nb_pages = compter_pages(&octets);
        Ok(DocumentRendu {
            octets,
            nb_pages: nb_pages.max(1),
        })
    }
}

/// Nombre de pages : lecture du `/Count` de l'objet `/Pages` écrit par le moteur (jamais une panique).
fn compter_pages(pdf: &[u8]) -> u32 {
    let t = String::from_utf8_lossy(pdf);
    t.find("/Type /Pages")
        .and_then(|i| t[i..].find("/Count "))
        .and_then(|j| {
            let i = t.find("/Type /Pages")? + j + "/Count ".len();
            t[i..]
                .split(|c: char| !c.is_ascii_digit())
                .next()?
                .parse()
                .ok()
        })
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_pdf::releve::{texte, verifier_structure};

    /// D1b : les noms de toutes langues sortent par la police incorporée (jamais « ? »).
    #[test]
    fn noms_de_toutes_langues_par_la_police_incorporee() {
        for (nom, prenom) in [
            ("Nguyễn", "Thị Hằng"),
            ("Wróblewski", "Łukasz"),
            ("Öztürk", "Şükrü"),
            ("Țăran", "Ștefan"),
            ("Ærøskøbing", "Ελένη"),
        ] {
            let donnees = vec![
                DonneeRendu {
                    cle: CLE_NOM.into(),
                    valeur: nom.into(),
                },
                DonneeRendu {
                    cle: CLE_PRENOM.into(),
                    valeur: prenom.into(),
                },
                DonneeRendu {
                    cle: CLE_NAISSANCE.into(),
                    valeur: "2012-03-10".into(),
                },
            ];
            let d = DemandeRendu {
                nature: &ATTESTATION_INSCRIPTION,
                style: "sobre",
                couleur_principale: None,
                mentions_pied: None,
                entite_raison_sociale: "Collège Saint-Étienne",
                entite_identifiants: &[],
                donnees: &donnees,
                textes_encadres: &[],
                numero_affiche: "2026-SCO-000001",
                code_verification: None,
                url_verification: None,
                numero_version: 1,
                motif: None,
                date_emission: "2026-11-15",
                par_delegation_de: None,
                destinataire_nom: None,
                destinataire_adresse: None,
                langue: "fr",
            };
            let pdf = RenduPdfAttestation.rendre(&d).unwrap().octets;
            verifier_structure(&pdf);
            let t = texte(&pdf);
            assert!(t.contains(&nom.to_uppercase()), "{nom} : {t}");
            assert!(t.contains(prenom), "{prenom} : {t}");
            assert!(!t.contains('?') && !t.contains('\u{fffd}'), "{t}");
            assert!(
                pdf.windows(10).any(|w| w == b"/FontFile2"),
                "police incorporée"
            );
        }
    }
}
