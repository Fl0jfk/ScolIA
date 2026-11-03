//! Session web du prototype.
//!
//! L'auth `ent-auth` n'est pas modifiée : elle délivre un jeton opaque envoyé en
//! `Authorization: Bearer`. L'ancienne interface le range dans `localStorage`
//! (`ent_token`) ; le prototype réutilise ce même emplacement, sur la même origine.
//!
//! Conséquence : le serveur ne voit pas la session lors du rendu HTML, donc les
//! données sont chargées par le navigateur. La cible (D-008) est un cookie
//! `HttpOnly; Secure; SameSite=Strict` : décision séparée, non appliquée ici.

use ent_web_api::ApiClient;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const CLE_JETON: &str = "ent_token";

/// Jeton courant (navigateur uniquement ; toujours `None` côté serveur).
pub fn jeton() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let stockage = leptos::prelude::window().local_storage().ok().flatten()?;
        stockage
            .get_item(CLE_JETON)
            .ok()
            .flatten()
            .filter(|j| !j.is_empty())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

pub fn enregistrer(jeton: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = leptos::prelude::window().local_storage().ok().flatten() {
        let _ = s.set_item(CLE_JETON, jeton);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = jeton;
}

pub fn effacer() {
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = leptos::prelude::window().local_storage().ok().flatten() {
        let _ = s.remove_item(CLE_JETON);
    }
}

/// Client API de même origine avec le jeton courant.
pub fn client() -> ApiClient {
    ApiClient::new("", jeton())
}
