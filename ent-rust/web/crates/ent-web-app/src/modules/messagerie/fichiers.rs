//! Accès navigateur pour les pièces jointes : lecture du fichier choisi et enregistrement
//! d'un téléchargement. Hors navigateur (rendu serveur, tests natifs) ces fonctions ne font rien.
//! Aucun style en ligne et aucune insertion HTML : un lien temporaire `blob:` est cliqué puis révoqué.

use std::fmt;

/// Fichier choisi, déjà contrôlé (nom assaini, type et taille acceptés) et lu en mémoire.
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

/// Résultat du choix d'un fichier.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub enum Lecture {
    /// Aucun fichier (sélection annulée ou vidée).
    Aucun,
    Fichier(FichierChoisi),
    /// Fichier refusé (type, taille) ou illisible : message pour la personne.
    Refuse(String),
}

/// Lit le premier fichier du champ `<input type="file">` après contrôle de nom, type et taille.
#[cfg(target_arch = "wasm32")]
pub async fn lire_fichier_choisi(input: &web_sys::HtmlInputElement) -> Lecture {
    use super::modele::{valider_piece, PIECE_MAX_OCTETS};

    let Some(fichier) = input.files().and_then(|l| l.get(0)) else {
        return Lecture::Aucun;
    };
    let valide = match valider_piece(&fichier.name(), &fichier.type_(), fichier.size() as usize) {
        Ok(v) => v,
        Err(e) => return Lecture::Refuse(e),
    };
    let Ok(tampon) = wasm_bindgen_futures::JsFuture::from(fichier.array_buffer()).await else {
        return Lecture::Refuse("Ce fichier n'a pas pu être lu.".into());
    };
    let octets = web_sys::js_sys::Uint8Array::new(&tampon).to_vec();
    if octets.is_empty() || octets.len() > PIECE_MAX_OCTETS {
        return Lecture::Refuse("Ce fichier est vide ou trop volumineux.".into());
    }
    Lecture::Fichier(FichierChoisi {
        nom: valide.nom,
        mime: valide.mime,
        octets,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn lire_fichier_choisi(_input: &web_sys::HtmlInputElement) -> Lecture {
    Lecture::Aucun
}

/// Propose le contenu à l'enregistrement sous `nom` (téléchargement, jamais affiché dans la page).
#[cfg(target_arch = "wasm32")]
pub fn enregistrer_telechargement(nom: &str, mime: &str, octets: &[u8]) -> Result<(), String> {
    use leptos::wasm_bindgen::JsCast;
    use web_sys::js_sys::{Array, Uint8Array};

    let echec = |_| "L'enregistrement du fichier a échoué.".to_string();
    let parties = Array::new();
    parties.push(&Uint8Array::from(octets));
    let options = web_sys::BlobPropertyBag::new();
    // Un type propre au serveur n'est jamais interprété par la page : simple téléchargement.
    options.set_type(mime);
    let blob =
        web_sys::Blob::new_with_u8_array_sequence_and_options(&parties, &options).map_err(echec)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(echec)?;
    let doc = leptos::prelude::document();
    let resultat = (|| {
        let lien: web_sys::HtmlAnchorElement =
            doc.create_element("a")
                .map_err(echec)?
                .dyn_into()
                .map_err(|_| "L'enregistrement du fichier a échoué.".to_string())?;
        lien.set_href(&url);
        lien.set_download(nom);
        let corps = doc
            .body()
            .ok_or_else(|| "L'enregistrement du fichier a échoué.".to_string())?;
        let _ = corps.append_child(&lien);
        lien.click();
        lien.remove();
        Ok(())
    })();
    // Révocation différée : le navigateur doit avoir lancé l'enregistrement.
    leptos::prelude::set_timeout(
        move || {
            let _ = web_sys::Url::revoke_object_url(&url);
        },
        std::time::Duration::from_secs(30),
    );
    resultat
}

#[cfg(not(target_arch = "wasm32"))]
pub fn enregistrer_telechargement(_nom: &str, _mime: &str, _octets: &[u8]) -> Result<(), String> {
    Err("Téléchargement indisponible ici.".into())
}
