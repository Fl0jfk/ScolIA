//! Signature d'un défi WebAuthn (passkey) par l'appareil, en Rust (API du navigateur
//! `navigator.credentials.get`), au format JSON attendu par `webauthn-rs` côté serveur.
//! Aucune clé ni aucun secret ne transite par cette page : seul l'appareil signe.

/// Encodage base64url sans remplissage (format de `webauthn-rs`).
pub fn b64url_encoder(octets: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut sortie = String::with_capacity(octets.len().div_ceil(3) * 4);
    for bloc in octets.chunks(3) {
        let n = (u32::from(bloc[0]) << 16)
            | (u32::from(*bloc.get(1).unwrap_or(&0)) << 8)
            | u32::from(*bloc.get(2).unwrap_or(&0));
        let caracteres = bloc.len() + 1;
        for i in 0..caracteres {
            sortie.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    sortie
}

/// Décodage base64url (avec ou sans remplissage ; accepte aussi `+` et `/`).
pub fn b64url_decoder(texte: &str) -> Option<Vec<u8>> {
    let valeur = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            _ => return None,
        } as u32)
    };
    let propre: Vec<u8> = texte.bytes().filter(|c| *c != b'=').collect();
    if propre.len() % 4 == 1 {
        return None;
    }
    let mut sortie = Vec::with_capacity(propre.len() * 3 / 4);
    for bloc in propre.chunks(4) {
        let mut n = 0u32;
        for (i, c) in bloc.iter().enumerate() {
            n |= valeur(*c)? << (18 - 6 * i);
        }
        let octets = bloc.len() - 1;
        for i in 0..octets {
            sortie.push(((n >> (16 - 8 * i)) & 0xFF) as u8);
        }
    }
    Some(sortie)
}

/// Demande à l'appareil de signer le défi (`options` = `RequestChallengeResponse` JSON).
/// Renvoie l'assertion (`PublicKeyCredential` JSON) à transmettre au serveur.
#[cfg(target_arch = "wasm32")]
pub async fn signer_defi(options: &serde_json::Value) -> Result<serde_json::Value, String> {
    use leptos::wasm_bindgen::{JsCast, JsValue};
    use web_sys::js_sys::{Array, ArrayBuffer, Object, Reflect, Uint8Array, JSON};

    let erreur = |m: &str| m.to_string();
    let pk = options
        .get("publicKey")
        .ok_or_else(|| erreur("défi passkey invalide"))?;
    let objet = JSON::parse(&pk.to_string()).map_err(|_| erreur("défi passkey illisible"))?;
    let en_octets = |b64: &str| -> Result<JsValue, String> {
        let o = b64url_decoder(b64).ok_or_else(|| erreur("défi passkey mal encodé"))?;
        Ok(Uint8Array::from(o.as_slice()).into())
    };
    let defi = pk["challenge"]
        .as_str()
        .ok_or_else(|| erreur("défi passkey sans challenge"))?;
    Reflect::set(&objet, &"challenge".into(), &en_octets(defi)?)
        .map_err(|_| erreur("défi passkey"))?;
    if let Some(liste) = pk.get("allowCredentials").and_then(|v| v.as_array()) {
        let tableau: Array = Reflect::get(&objet, &"allowCredentials".into())
            .map_err(|_| erreur("défi passkey"))?
            .unchecked_into();
        for (i, c) in liste.iter().enumerate() {
            if let Some(id) = c["id"].as_str() {
                Reflect::set(&tableau.get(i as u32), &"id".into(), &en_octets(id)?)
                    .map_err(|_| erreur("défi passkey"))?;
            }
        }
    }
    let demande = Object::new();
    Reflect::set(&demande, &"publicKey".into(), &objet).map_err(|_| erreur("défi passkey"))?;

    let fenetre = web_sys::window().ok_or_else(|| erreur("navigateur indisponible"))?;
    let promesse = fenetre
        .navigator()
        .credentials()
        .get_with_options(demande.unchecked_ref())
        .map_err(|_| erreur("passkey non prise en charge par ce navigateur"))?;
    let reponse = wasm_bindgen_futures::JsFuture::from(promesse)
        .await
        .map_err(|_| erreur("validation par passkey annulée ou refusée"))?;

    let lire = |o: &JsValue, cle: &str| Reflect::get(o, &cle.into()).unwrap_or(JsValue::NULL);
    let b64 = |v: JsValue| -> Option<String> {
        v.dyn_into::<ArrayBuffer>()
            .ok()
            .map(|b| b64url_encoder(&Uint8Array::new(&b).to_vec()))
    };
    let r = lire(&reponse, "response");
    Ok(serde_json::json!({
        "id": lire(&reponse, "id").as_string(),
        "rawId": b64(lire(&reponse, "rawId")),
        "type": lire(&reponse, "type").as_string(),
        "extensions": {},
        "response": {
            "authenticatorData": b64(lire(&r, "authenticatorData")),
            "clientDataJSON": b64(lire(&r, "clientDataJSON")),
            "signature": b64(lire(&r, "signature")),
            "userHandle": b64(lire(&r, "userHandle")),
        },
    }))
}

/// Hors navigateur (rendu serveur, tests) : indisponible.
#[cfg(not(target_arch = "wasm32"))]
pub async fn signer_defi(_options: &serde_json::Value) -> Result<serde_json::Value, String> {
    Err("passkey disponible uniquement dans le navigateur".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_aller_retour() {
        for donnees in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            &[0xfb, 0xff, 0xfe],
        ] {
            let e = b64url_encoder(donnees);
            assert!(
                !e.contains('=') && !e.contains('+') && !e.contains('/'),
                "{e}"
            );
            assert_eq!(b64url_decoder(&e).as_deref(), Some(donnees), "{e}");
        }
        assert_eq!(b64url_encoder(b"foobar"), "Zm9vYmFy");
        assert_eq!(b64url_encoder(&[0xfb, 0xff]), "-_8");
        assert_eq!(b64url_decoder("Zm9vYg=="), Some(b"foob".to_vec()));
        assert_eq!(b64url_decoder("a"), None, "longueur impossible");
        assert_eq!(b64url_decoder("a$b="), None, "caractère invalide");
    }
}
