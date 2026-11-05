//! Session web : **abstraction unique** du transport de la session.
//!
//! Aujourd'hui (validé pour la tranche 1) : l'authentification existante, inchangée,
//! délivre un jeton opaque envoyé en `Authorization: Bearer` et rangé dans
//! `localStorage` (`ent_token`, partagé avec l'ancienne interface `/app`).
//! Demain (après décision) : cookie `HttpOnly` (prototype `ent-web-session`) ; seul
//! ce fichier changera, les écrans passent tous par [`client`] et [`ContexteSession`].
//!
//! Le serveur ne voit pas la session pendant le rendu HTML : les données sont
//! chargées par le navigateur.

use ent_web_api::{ApiClient, ApiError, Moi, MoiSecurite};
use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const CLE_JETON: &str = "ent_token";

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

/// Le navigateur refuse d'enregistrer la session (navigation privée, stockage désactivé).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StockageIndisponible;

impl StockageIndisponible {
    pub const MESSAGE: &'static str = "Votre navigateur bloque l'enregistrement de la session (navigation privée ou stockage désactivé).";
}

/// Enregistre le jeton ; l'échec (stockage absent, refusé ou non relu) est **remonté**.
pub fn enregistrer(jeton: &str) -> Result<(), StockageIndisponible> {
    #[cfg(target_arch = "wasm32")]
    {
        let s = leptos::prelude::window()
            .local_storage()
            .ok()
            .flatten()
            .ok_or(StockageIndisponible)?;
        s.set_item(CLE_JETON, jeton)
            .map_err(|_| StockageIndisponible)?;
        // Relecture : certains navigateurs n'échouent qu'à la lecture.
        if s.get_item(CLE_JETON).ok().flatten().as_deref() == Some(jeton) {
            Ok(())
        } else {
            Err(StockageIndisponible)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = jeton;
        Ok(())
    }
}

/// Message de la page de connexion quand la révocation serveur n'a pas pu être confirmée.
pub const MESSAGE_DECONNEXION_LOCALE: &str = "Déconnexion locale effectuée. Le serveur n'a pas pu confirmer la fin de la session : elle expirera automatiquement. Si vous êtes sur un poste partagé, prévenez l'établissement.";

/// Adresse de la page de connexion après une déconnexion demandée par la personne.
/// Succès (204) et 401 (session déjà invalide côté serveur) : `deconnexion` ; réseau,
/// 5xx, autre échec ou délai dépassé (`None`) : `locale`. Dans tous les cas le jeton
/// local est effacé avant d'attendre la révocation.
pub fn adresse_apres_deconnexion(resultat: Option<&Result<(), ApiError>>) -> &'static str {
    match resultat {
        Some(Ok(())) | Some(Err(ApiError::NonConnecte)) => "/web/connexion?fin=deconnexion",
        Some(Err(_)) | None => "/web/connexion?fin=locale",
    }
}

/// Durée maximale d'attente de la révocation serveur lors d'une déconnexion.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const DELAI_REVOCATION: std::time::Duration = std::time::Duration::from_secs(5);

/// Termine la session : le jeton local est effacé tout de suite (les autres onglets le voient
/// par l'événement `storage`), puis on attend la révocation en base, au plus
/// [`DELAI_REVOCATION`], avant de rejoindre la page de connexion.
pub fn deconnecter<N>(navigate: N)
where
    N: Fn(&str, NavigateOptions) + Clone + 'static,
{
    let client = client();
    effacer();
    let fini = Rc::new(Cell::new(false));
    {
        let navigate = navigate.clone();
        let fini = fini.clone();
        spawn_local(async move {
            let resultat = client.deconnexion().await;
            if !fini.replace(true) {
                navigate(
                    adresse_apres_deconnexion(Some(&resultat)),
                    Default::default(),
                );
            }
        });
    }
    set_timeout(
        move || {
            if !fini.replace(true) {
                navigate(adresse_apres_deconnexion(None), Default::default());
            }
        },
        DELAI_REVOCATION,
    );
}

/// Efface la session locale. Les autres onglets le voient (événement `storage`).
pub fn effacer() {
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = leptos::prelude::window().local_storage().ok().flatten() {
        let _ = s.remove_item(CLE_JETON);
    }
}

/// Suite à donner à un événement `storage` reçu d'un autre onglet.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuiteStockage {
    /// Sans effet pour cet onglet.
    Ignorer,
    /// Session terminée ailleurs : retour à la connexion.
    Deconnexion,
    /// Un autre compte (ou une nouvelle session) a pris la place : tout recharger,
    /// sans quoi l'identité affichée ne serait plus celle du jeton envoyé.
    Recharger,
}

/// Décision pure : `cle` vaut `None` pour un `clear()` du stockage ; `nouvelle` est la
/// valeur écrite ; `jeton_onglet` est le jeton avec lequel **cet onglet** a chargé son état.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn suite_evenement_stockage(
    cle: Option<&str>,
    nouvelle: Option<&str>,
    jeton_onglet: Option<&str>,
) -> SuiteStockage {
    if cle.is_some_and(|c| c != CLE_JETON) {
        return SuiteStockage::Ignorer;
    }
    let nouvelle = nouvelle.filter(|j| !j.is_empty());
    match (nouvelle, jeton_onglet) {
        (None, None) => SuiteStockage::Ignorer,
        (None, Some(_)) => SuiteStockage::Deconnexion,
        (Some(n), Some(c)) if n == c => SuiteStockage::Ignorer,
        (Some(_), _) => SuiteStockage::Recharger,
    }
}

/// Client API de même origine avec la session courante.
pub fn client() -> ApiClient {
    ApiClient::new("", jeton())
}

/// Adresse complète (chemin + « ? » + paramètres) : `location.search` est rendu sans « ? ».
pub fn chemin_complet(pathname: &str, search: &str) -> String {
    let recherche = search.trim_start_matches('?');
    if recherche.is_empty() {
        pathname.to_string()
    } else {
        format!("{pathname}?{recherche}")
    }
}

/// Décodage « pourcent » strict (UTF-8 valide exigé) ; `None` si malformé.
fn decoder_pourcent(s: &str) -> Option<String> {
    let o = s.as_bytes();
    let mut sortie = Vec::with_capacity(o.len());
    let mut i = 0;
    while i < o.len() {
        if o[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            sortie.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            sortie.push(o[i]);
            i += 1;
        }
    }
    String::from_utf8(sortie).ok()
}

/// Vrai si `r` est `/web` exact, ou sous `/web/` ou `/web?`, sans contournement
/// (antislash, segment `..`, caractère de contrôle), y compris sous forme encodée.
fn retour_admissible(r: &str) -> bool {
    let Some(decode) = decoder_pourcent(r) else {
        return false;
    };
    [r, decode.as_str()].iter().all(|v| {
        let Some(reste) = v.strip_prefix("/web") else {
            return false;
        };
        if !(reste.is_empty() || reste.starts_with('/') || reste.starts_with('?')) {
            return false;
        }
        if v.contains('\\') || v.chars().any(char::is_control) || v.contains("://") {
            return false;
        }
        let chemin = v.split(['?', '#']).next().unwrap_or("");
        !chemin.split('/').any(|seg| seg == "..")
    })
}

/// Vrai si l'adresse (décodée) est la page de connexion ou en dessous.
fn est_connexion(r: &str) -> bool {
    let decode = decoder_pourcent(r).unwrap_or_else(|| r.to_string());
    let chemin = decode.split(['?', '#']).next().unwrap_or("");
    chemin == "/web/connexion" || chemin.starts_with("/web/connexion/")
}

/// Adresse de la page de connexion, avec retour vers `chemin` (pages `/web/…` seulement).
pub fn adresse_connexion(chemin: &str) -> String {
    if retour_admissible(chemin) && !est_connexion(chemin) {
        format!("/web/connexion?retour={}", encoder_retour(chemin))
    } else {
        "/web/connexion".into()
    }
}

/// Comme [`adresse_connexion`], en signalant que la session a expiré pendant une saisie
/// (la page de connexion prévient que les modifications non envoyées sont perdues).
pub fn adresse_connexion_saisie_perdue(chemin: &str) -> String {
    let base = adresse_connexion(chemin);
    let sep = if base.contains('?') { '&' } else { '?' };
    format!("{base}{sep}fin=expiree-saisie")
}

/// Encodage minimal d'un chemin interne pour un paramètre d'URL.
fn encoder_retour(chemin: &str) -> String {
    chemin
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '/' | '-' | '_' | '.' => c.to_string(),
            c => {
                let mut tampon = [0u8; 4];
                c.encode_utf8(&mut tampon)
                    .bytes()
                    .map(|o| format!("%{o:02X}"))
                    .collect()
            }
        })
        .collect()
}

/// Adresse de retour acceptée après connexion : uniquement une page interne `/web/…`
/// (jamais une adresse externe, pour éviter les redirections ouvertes).
pub fn retour_sur(retour: Option<String>) -> String {
    retour
        .filter(|r| retour_admissible(r) && !est_connexion(r))
        .unwrap_or_else(|| "/web".into())
}

/// Contexte fourni par la coquille : la personne connectée, chargée **une seule fois**.
#[derive(Clone, Copy)]
pub struct ContexteSession {
    pub moi: LocalResource<Result<Moi, ApiError>>,
    /// Portée et niveau de la session (`/auth/me`), renseignés par le même chargement
    /// que `moi` ; `None` tant que la réponse n'est pas arrivée.
    pub securite: RwSignal<Option<MoiSecurite>>,
}

/// Contexte de session (dans une page de la coquille authentifiée).
pub fn use_session() -> ContexteSession {
    expect_context::<ContexteSession>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresse_apres_deconnexion_selon_la_revocation() {
        let normale = "/web/connexion?fin=deconnexion";
        let locale = "/web/connexion?fin=locale";
        assert_eq!(adresse_apres_deconnexion(Some(&Ok(()))), normale);
        assert_eq!(
            adresse_apres_deconnexion(Some(&Err(ApiError::NonConnecte))),
            normale
        );
        assert_eq!(
            adresse_apres_deconnexion(Some(&Err(ApiError::Reseau("hors ligne".into())))),
            locale
        );
        assert_eq!(
            adresse_apres_deconnexion(Some(&Err(ApiError::Serveur(503, "x".into())))),
            locale
        );
        assert_eq!(
            adresse_apres_deconnexion(Some(&Err(ApiError::Decodage("x".into())))),
            locale
        );
        // Délai dépassé : révocation non confirmée.
        assert_eq!(adresse_apres_deconnexion(None), locale);
    }

    #[test]
    fn retour_limite_aux_pages_internes() {
        assert_eq!(
            retour_sur(Some("/web/appel?jour=2026-11-03".into())),
            "/web/appel?jour=2026-11-03"
        );
        assert_eq!(retour_sur(None), "/web");
        assert_eq!(retour_sur(Some("https://pirate.example".into())), "/web");
        assert_eq!(retour_sur(Some("//pirate.example/web".into())), "/web");
        assert_eq!(retour_sur(Some("/app".into())), "/web");
        assert_eq!(
            retour_sur(Some("/web/connexion".into())),
            "/web",
            "pas de boucle"
        );
        assert_eq!(retour_sur(Some("/web".into())), "/web");
        assert_eq!(retour_sur(Some("/web?x=1".into())), "/web?x=1");
    }

    #[test]
    fn retour_refuse_les_contournements() {
        for mauvais in [
            "/webx",
            "/web/../app",
            "/web\\evil",
            "/web/%2e%2e/app",
            "/web/%2E%2E/app",
            "/web/%5cevil",
            "/web/a%00b",
            "/web/a\nb",
            "/web/%zz",
            "/web/%63onnexion",
            "/web/%63onnexion?retour=/web",
            "/web/connexion/x",
            "/web?u=http://pirate.example",
        ] {
            assert_eq!(retour_sur(Some(mauvais.into())), "/web", "{mauvais}");
        }
        assert_eq!(adresse_connexion("/web/%63onnexion"), "/web/connexion");
        assert_eq!(adresse_connexion("/webx"), "/web/connexion");
        assert_eq!(adresse_connexion("/web/../app"), "/web/connexion");
    }

    #[test]
    fn connexion_apres_saisie_perdue() {
        assert_eq!(
            adresse_connexion_saisie_perdue("/web/appel?jour=2026-11-03"),
            "/web/connexion?retour=/web/appel%3Fjour%3D2026-11-03&fin=expiree-saisie"
        );
        assert_eq!(
            adresse_connexion_saisie_perdue("/x"),
            "/web/connexion?fin=expiree-saisie"
        );
    }

    #[test]
    fn chemin_complet_retablit_le_point_d_interrogation() {
        assert_eq!(chemin_complet("/web/appel", ""), "/web/appel");
        assert_eq!(
            chemin_complet("/web/appel", "jour=2026-11-03"),
            "/web/appel?jour=2026-11-03"
        );
        assert_eq!(
            chemin_complet("/web/appel", "?jour=2026-11-03"),
            "/web/appel?jour=2026-11-03"
        );
        assert_eq!(chemin_complet("/web/appel", "?"), "/web/appel");
    }

    #[test]
    fn adresse_de_connexion_avec_retour() {
        assert_eq!(
            adresse_connexion("/web/appel"),
            "/web/connexion?retour=/web/appel"
        );
        assert_eq!(
            adresse_connexion("/web/appel?jour=2026-11-03&creneau=x"),
            "/web/connexion?retour=/web/appel%3Fjour%3D2026-11-03%26creneau%3Dx"
        );
        assert_eq!(adresse_connexion("/web/connexion"), "/web/connexion");
    }

    #[test]
    fn synchro_entre_onglets() {
        use SuiteStockage::*;
        let f = suite_evenement_stockage;
        assert_eq!(
            f(Some(CLE_JETON), None, Some("A")),
            Deconnexion,
            "déconnexion ailleurs"
        );
        assert_eq!(f(None, None, Some("A")), Deconnexion, "clear()");
        assert_eq!(
            f(Some(CLE_JETON), Some(""), Some("A")),
            Deconnexion,
            "valeur vide"
        );
        assert_eq!(
            f(Some(CLE_JETON), Some("B"), Some("A")),
            Recharger,
            "autre compte"
        );
        assert_eq!(
            f(Some(CLE_JETON), Some("A"), Some("A")),
            Ignorer,
            "même jeton"
        );
        assert_eq!(
            f(Some(CLE_JETON), Some("B"), None),
            Recharger,
            "connexion ailleurs"
        );
        assert_eq!(f(Some(CLE_JETON), None, None), Ignorer);
        assert_eq!(f(Some("autre_cle"), None, Some("A")), Ignorer);
        assert_eq!(f(Some("autre_cle"), Some("B"), Some("A")), Ignorer);
    }
}
