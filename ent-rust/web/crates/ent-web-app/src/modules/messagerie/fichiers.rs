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

// Déplacé dans le noyau (partagé avec les bulletins, S2-7).
pub use crate::noyau::telechargement::enregistrer_telechargement;
