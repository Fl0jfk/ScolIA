//! Accès navigateur : lecture du fichier choisi et ouverture d'un lien signé.
//! Hors navigateur (rendu serveur, tests natifs) ces fonctions ne font rien.
//! Aucun style en ligne ni insertion HTML : un lien temporaire est créé, cliqué, retiré.

use std::fmt;

/// Fichier choisi, déjà contrôlé (nom et taille) et lu en mémoire.
#[derive(Clone, PartialEq, Eq)]
pub struct FichierChoisi {
    pub nom: String,
    pub mime: String,
    pub octets: Vec<u8>,
}

impl fmt::Debug for FichierChoisi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FichierChoisi")
            .field("nom", &self.nom)
            .field("mime", &self.mime)
            .field("taille", &self.octets.len())
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub enum Lecture {
    /// Aucun fichier (sélection annulée ou vidée).
    Aucun,
    Fichier(FichierChoisi),
    /// Fichier refusé (nom, taille) ou illisible : message pour la personne.
    Refuse(String),
}

/// Lit le premier fichier du champ `<input type="file">` après contrôle de nom et de taille.
/// Le type réel est contrôlé par le serveur (signature binaire, liste blanche).
#[cfg(target_arch = "wasm32")]
pub async fn lire_fichier_choisi(input: &web_sys::HtmlInputElement) -> Lecture {
    use super::modele::{valider_fichier, FICHIER_MAX_OCTETS};

    let Some(fichier) = input.files().and_then(|l| l.get(0)) else {
        return Lecture::Aucun;
    };
    let nom = match valider_fichier(&fichier.name(), fichier.size() as usize) {
        Ok(n) => n,
        Err(e) => return Lecture::Refuse(e),
    };
    let Ok(tampon) = wasm_bindgen_futures::JsFuture::from(fichier.array_buffer()).await else {
        return Lecture::Refuse("Ce fichier n'a pas pu être lu.".into());
    };
    let octets = web_sys::js_sys::Uint8Array::new(&tampon).to_vec();
    if octets.is_empty() || octets.len() > FICHIER_MAX_OCTETS {
        return Lecture::Refuse("Ce fichier est vide ou trop volumineux.".into());
    }
    Lecture::Fichier(FichierChoisi {
        nom,
        mime: fichier.type_(),
        octets,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn lire_fichier_choisi(_input: &web_sys::HtmlInputElement) -> Lecture {
    Lecture::Aucun
}

/// Ouvre une adresse signée (téléchargement d'une pièce). Le serveur impose
/// `Content-Disposition: attachment` : le fichier n'est jamais affiché dans la page.
#[cfg(target_arch = "wasm32")]
pub fn ouvrir_adresse(adresse: &str) -> Result<(), String> {
    use leptos::wasm_bindgen::JsCast;

    let echec = || "L'ouverture du fichier a échoué.".to_string();
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
    Err("Téléchargement indisponible ici.".into())
}
