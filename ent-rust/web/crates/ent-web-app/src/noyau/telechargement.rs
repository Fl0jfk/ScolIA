//! Enregistrement d'un fichier reçu de l'API (pièce jointe, bulletin PDF).

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
