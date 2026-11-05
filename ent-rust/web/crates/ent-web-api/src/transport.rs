//! Envoi HTTP selon la plateforme. Renvoie `(statut, corps texte)`.

use crate::{ApiError, Methode};

#[cfg(target_arch = "wasm32")]
pub(crate) async fn envoyer(
    methode: Methode,
    url: &str,
    jeton: Option<&str>,
    corps: Option<serde_json::Value>,
) -> Result<(u16, String), ApiError> {
    use gloo_net::http::Request;

    let mut req = match methode {
        Methode::Get => Request::get(url),
        Methode::Post => Request::post(url),
    };
    req = req.header("Accept", "application/json");
    if let Some(j) = jeton {
        req = req.header("Authorization", &format!("Bearer {j}"));
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
    jeton: Option<&str>,
    corps: Option<serde_json::Value>,
) -> Result<(u16, String), ApiError> {
    let client = reqwest::Client::new();
    let mut req = match methode {
        Methode::Get => client.get(url),
        Methode::Post => client.post(url),
    };
    req = req.header("Accept", "application/json");
    if let Some(j) = jeton {
        req = req.bearer_auth(j);
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
    _jeton: Option<&str>,
    _corps: Option<serde_json::Value>,
) -> Result<(u16, String), ApiError> {
    Err(ApiError::Indisponible)
}

/// `GET` d'un contenu binaire (pièces jointes). Renvoie `(statut, octets)`.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn envoyer_octets(
    url: &str,
    jeton: Option<&str>,
) -> Result<(u16, Vec<u8>), ApiError> {
    use gloo_net::http::Request;

    let mut req = Request::get(url);
    if let Some(j) = jeton {
        req = req.header("Authorization", &format!("Bearer {j}"));
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
pub(crate) async fn envoyer_octets(
    url: &str,
    jeton: Option<&str>,
) -> Result<(u16, Vec<u8>), ApiError> {
    let mut req = reqwest::Client::new().get(url);
    if let Some(j) = jeton {
        req = req.bearer_auth(j);
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
    _jeton: Option<&str>,
) -> Result<(u16, Vec<u8>), ApiError> {
    Err(ApiError::Indisponible)
}
