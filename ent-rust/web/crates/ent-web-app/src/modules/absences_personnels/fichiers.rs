//! Accès navigateur : lecture de la pièce choisie et ouverture d'un lien signé.
//! Hors navigateur (rendu serveur, tests natifs) ces fonctions ne font rien. Aucun style en ligne ni
//! insertion HTML : un lien temporaire est créé, cliqué, retiré.

use std::fmt;

/// Taille maximale d'une pièce (alignée sur la limite du serveur, qui reste juge).
pub const PIECE_MAX_OCTETS: usize = 10 * 1024 * 1024;

/// Pièce choisie, déjà contrôlée (nom et taille) et lue en mémoire.
#[derive(Clone, PartialEq, Eq)]
pub struct PieceChoisie {
    pub nom: String,
    pub mime: String,
    pub octets: Vec<u8>,
}

impl fmt::Debug for PieceChoisie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PieceChoisie")
            .field("nom", &self.nom)
            .field("taille", &self.octets.len())
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub enum Lecture {
    /// Aucun fichier (sélection annulée ou vidée).
    Aucun,
    Fichier(PieceChoisie),
    /// Fichier refusé (nom, taille) ou illisible : message pour la personne.
    Refuse(String),
}

/// Contrôle de forme : nom et taille (le type réel est contrôlé par le serveur).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn valider_piece(nom: &str, taille: usize) -> Result<String, String> {
    let nom = nom.trim();
    if nom.is_empty() || nom.chars().count() > 120 || nom.contains(['/', '\\', '\0']) {
        return Err("Ce nom de fichier n'est pas accepté.".into());
    }
    if taille == 0 {
        return Err("Ce fichier est vide.".into());
    }
    if taille > PIECE_MAX_OCTETS {
        return Err("Ce fichier dépasse 10 Mo.".into());
    }
    Ok(nom.to_string())
}

/// Lit le premier fichier du champ `<input type="file">` après contrôle de nom et de taille.
#[cfg(target_arch = "wasm32")]
pub async fn lire_piece_choisie(input: &web_sys::HtmlInputElement) -> Lecture {
    let Some(fichier) = input.files().and_then(|l| l.get(0)) else {
        return Lecture::Aucun;
    };
    let nom = match valider_piece(&fichier.name(), fichier.size() as usize) {
        Ok(n) => n,
        Err(e) => return Lecture::Refuse(e),
    };
    let Ok(tampon) = wasm_bindgen_futures::JsFuture::from(fichier.array_buffer()).await else {
        return Lecture::Refuse("Ce fichier n'a pas pu être lu.".into());
    };
    let octets = web_sys::js_sys::Uint8Array::new(&tampon).to_vec();
    if octets.is_empty() || octets.len() > PIECE_MAX_OCTETS {
        return Lecture::Refuse("Ce fichier est vide ou trop volumineux.".into());
    }
    Lecture::Fichier(PieceChoisie {
        nom,
        mime: fichier.type_(),
        octets,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn lire_piece_choisie(_input: &web_sys::HtmlInputElement) -> Lecture {
    Lecture::Aucun
}

/// Ouvre une adresse signée (lecture d'une pièce). Le serveur impose `Content-Disposition: attachment`.
#[cfg(target_arch = "wasm32")]
pub fn ouvrir_adresse(adresse: &str) -> Result<(), String> {
    use leptos::wasm_bindgen::JsCast;

    let echec = || "L'ouverture de la pièce a échoué.".to_string();
    let doc = leptos::prelude::document();
    let lien: web_sys::HtmlAnchorElement = doc
        .create_element("a")
        .map_err(|_| echec())?
        .dyn_into()
        .map_err(|_| echec())?;
    lien.set_href(adresse);
    lien.set_rel("noopener noreferrer");
    let corps = doc.body().ok_or_else(echec)?;
    let _ = corps.append_child(&lien);
    lien.click();
    lien.remove();
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ouvrir_adresse(_adresse: &str) -> Result<(), String> {
    Err("Ouverture indisponible ici.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_piece_vide_ou_trop_grosse_ou_au_nom_douteux_est_refusee() {
        assert!(valider_piece("arret.pdf", 1200).is_ok());
        assert!(valider_piece("", 10).is_err());
        assert!(valider_piece("../etc/passwd", 10).is_err());
        assert!(valider_piece("a.pdf", 0).is_err());
        assert!(valider_piece("a.pdf", PIECE_MAX_OCTETS + 1).is_err());
    }
}
