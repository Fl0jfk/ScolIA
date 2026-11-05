//! Envoi HTTP selon la plateforme. Renvoie `(statut, corps texte)`.

use crate::{ApiError, Methode};

/// Façon de prouver la session à ent-api.
#[derive(Clone, Copy)]
#[cfg_attr(not(any(target_arch = "wasm32", feature = "natif")), allow(dead_code))]
pub(crate) enum Auth<'a> {
    /// Aucune session (connexion, routes publiques).
    Aucun,
    /// Jeton Bearer (applis mobiles, tests contre le backend).
    Bearer(&'a str),
    /// Cookie `HttpOnly` de session web, joint par le navigateur lui-même (même origine) :
    /// le code n'a jamais le jeton. Seul l'en-tête anti-CSRF est ajouté.
    Cookie,
}

/// En-tête anti-CSRF exigé par le relais `/web` (voir `ent-web-session`).
#[cfg_attr(not(any(target_arch = "wasm32", feature = "natif")), allow(dead_code))]
pub(crate) const EN_TETE_CSRF: &str = "X-ENT-CSRF";

#[cfg(target_arch = "wasm32")]
pub(crate) async fn envoyer(
    methode: Methode,
    url: &str,
    auth: Auth<'_>,
    corps: Option<serde_json::Value>,
) -> Result<(u16, String), ApiError> {
    use gloo_net::http::Request;

    let mut req = match methode {
        Methode::Get => Request::get(url),
        Methode::Post => Request::post(url),
        Methode::Put => Request::put(url),
        Methode::Delete => Request::delete(url),
    };
    req = req.header("Accept", "application/json");
    match auth {
        Auth::Bearer(j) => req = req.header("Authorization", &format!("Bearer {j}")),
        Auth::Cookie => req = req.header(EN_TETE_CSRF, "1"),
        Auth::Aucun => {}
    }
    let reponse = match corps {
        Some(c) => {
            req.json(&c)
                .map_err(|e| ApiError::Reseau(e.to_string()))?
                .send()
                .await
        }
        None => req.send().await,
    }
    .map_err(|e| ApiError::Reseau(e.to_string()))?;
    let statut = reponse.status();
    let texte = reponse.text().await.unwrap_or_default();
    Ok((statut, texte))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "natif"))]
pub(crate) async fn envoyer(
    methode: Methode,
    url: &str,
    auth: Auth<'_>,
    corps: Option<serde_json::Value>,
) -> Result<(u16, String), ApiError> {
    let client = reqwest::Client::new();
    let mut req = match methode {
        Methode::Get => client.get(url),
        Methode::Post => client.post(url),
        Methode::Put => client.put(url),
        Methode::Delete => client.delete(url),
    };
    req = req.header("Accept", "application/json");
    match auth {
        Auth::Bearer(j) => req = req.bearer_auth(j),
        Auth::Cookie => req = req.header(EN_TETE_CSRF, "1"),
        Auth::Aucun => {}
    }
    if let Some(c) = corps {
        req = req.json(&c);
    }
    let reponse = req
        .send()
        .await
        .map_err(|e| ApiError::Reseau(e.to_string()))?;
    let statut = reponse.status().as_u16();
    let texte = reponse.text().await.unwrap_or_default();
    Ok((statut, texte))
}

/// Rendu serveur sans transport : les données métier sont chargées côté navigateur.
#[cfg(all(not(target_arch = "wasm32"), not(feature = "natif")))]
pub(crate) async fn envoyer(
    _methode: Methode,
    _url: &str,
    _auth: Auth<'_>,
    _corps: Option<serde_json::Value>,
) -> Result<(u16, String), ApiError> {
    Err(ApiError::Indisponible)
}

/// `GET` d'un contenu binaire (pièces jointes). Renvoie `(statut, octets)`.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn envoyer_octets(url: &str, auth: Auth<'_>) -> Result<(u16, Vec<u8>), ApiError> {
    use gloo_net::http::Request;

    let mut req = Request::get(url);
    match auth {
        Auth::Bearer(j) => req = req.header("Authorization", &format!("Bearer {j}")),
        Auth::Cookie => req = req.header(EN_TETE_CSRF, "1"),
        Auth::Aucun => {}
    }
    let reponse = req
        .send()
        .await
        .map_err(|e| ApiError::Reseau(e.to_string()))?;
    let statut = reponse.status();
    let octets = reponse
        .binary()
        .await
        .map_err(|e| ApiError::Reseau(e.to_string()))?;
    Ok((statut, octets))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "natif"))]
pub(crate) async fn envoyer_octets(url: &str, auth: Auth<'_>) -> Result<(u16, Vec<u8>), ApiError> {
    let mut req = reqwest::Client::new().get(url);
    match auth {
        Auth::Bearer(j) => req = req.bearer_auth(j),
        Auth::Cookie => req = req.header(EN_TETE_CSRF, "1"),
        Auth::Aucun => {}
    }
    let reponse = req
        .send()
        .await
        .map_err(|e| ApiError::Reseau(e.to_string()))?;
    let statut = reponse.status().as_u16();
    let octets = reponse
        .bytes()
        .await
        .map_err(|e| ApiError::Reseau(e.to_string()))?;
    Ok((statut, octets.to_vec()))
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "natif")))]
pub(crate) async fn envoyer_octets(
    _url: &str,
    _auth: Auth<'_>,
) -> Result<(u16, Vec<u8>), ApiError> {
    Err(ApiError::Indisponible)
}
