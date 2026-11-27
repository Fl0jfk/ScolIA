//! Session web : **abstraction unique** du transport de la session.
//!
//! Depuis A6 (socle 1), la session est un cookie `__Host-ent_session` **`HttpOnly`** posé
//! par le relais `ent-web-session` : le JavaScript et le WebAssembly de la page n'ont
//! **jamais** le jeton, qui n'est donc plus dans `localStorage` ni `sessionStorage`. Les
//! appels passent par [`client`] (relais `/web/api`, cookie joint par le navigateur, en-tête
//! anti-CSRF). Les applis mobiles gardent le jeton Bearer : elles n'utilisent pas ce code.
//!
//! Ce que la page sait de la session : l'identité (`/auth/me`) et le temps restant
//! (`/web/session/etat`), jamais le secret. Les onglets se préviennent entre eux par un
//! `BroadcastChannel` (déconnexion, nouvelle connexion, activité) : messages **sans secret**.
//!
//! Le serveur ne voit pas la session pendant le rendu HTML : les données sont
//! chargées par le navigateur.

use ent_web_api::{ApiClient, ApiError, Moi, MoiSecurite};
use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use uuid::Uuid;

/// Clé `localStorage` de l'ancien stockage du jeton (avant A6). Elle n'est plus **jamais
/// lue pour authentifier** ni écrite : seule [`purger_ancien_jeton`] la touche, pour l'effacer.
pub const ANCIENNE_CLE_JETON: &str = "ent_token";

/// Migration douce : si un ancien jeton traîne dans `localStorage` (avant A6, ou posé par
/// l'ancienne interface `/app`, figée, qui partageait cette clé), il est **effacé** puis
/// révoqué côté serveur (appel `/auth/logout`, au mieux). Navigateur seulement.
pub fn purger_ancien_jeton() {
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = leptos::prelude::window().local_storage().ok().flatten() {
        if let Ok(Some(ancien)) = s.get_item(ANCIENNE_CLE_JETON) {
            let _ = s.remove_item(ANCIENNE_CLE_JETON);
            if !ancien.is_empty() {
                spawn_local(async move {
                    let _ = ApiClient::new("", Some(ancien)).deconnexion().await;
                });
            }
        }
    }
}

/// Message de la page de connexion quand la révocation serveur n'a pas pu être confirmée.
pub const MESSAGE_DECONNEXION_LOCALE: &str = "Déconnexion locale effectuée. Le serveur n'a pas pu confirmer la fin de la session : elle expirera automatiquement. Si vous êtes sur un poste partagé, prévenez l'établissement.";

/// Adresse de la page de connexion après une déconnexion demandée par la personne.
/// Succès (204) et 401 (session déjà invalide côté serveur) : `deconnexion` ; réseau,
/// 5xx, autre échec ou délai dépassé (`None`) : `locale`.
pub fn adresse_apres_deconnexion(resultat: Option<&Result<(), ApiError>>) -> &'static str {
    match resultat {
        Some(Ok(())) | Some(Err(ApiError::NonConnecte)) => "/web/connexion?fin=deconnexion",
        Some(Err(_)) | None => "/web/connexion?fin=locale",
    }
}

/// Durée maximale d'attente de la révocation serveur lors d'une déconnexion.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const DELAI_REVOCATION: std::time::Duration = std::time::Duration::from_secs(5);

/// Termine la session : les autres onglets sont prévenus tout de suite, puis on attend la
/// révocation en base (le serveur efface aussi le cookie), au plus [`DELAI_REVOCATION`],
/// avant de rejoindre la page de connexion.
pub fn deconnecter<N>(navigate: N)
where
    N: Fn(&str, NavigateOptions) + Clone + 'static,
{
    let client = client();
    annoncer(Message::Fin);
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

/// Message diffusé aux autres onglets du même navigateur. **Aucun secret** : jamais le jeton.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    /// Session terminée (déconnexion volontaire).
    Fin,
    /// Une session vient d'être ouverte pour cette personne.
    Ouverte(Uuid),
    /// Dernière activité reconnue par le serveur (secondes Unix).
    Activite(i64),
}

impl Message {
    pub fn encoder(&self) -> String {
        match self {
            Message::Fin => "fin".into(),
            Message::Ouverte(id) => format!("ouverte:{id}"),
            Message::Activite(t) => format!("activite:{t}"),
        }
    }

    pub fn decoder(texte: &str) -> Option<Message> {
        if texte == "fin" {
            return Some(Message::Fin);
        }
        let (genre, valeur) = texte.split_once(':')?;
        match genre {
            "ouverte" => valeur.parse().ok().map(Message::Ouverte),
            "activite" => valeur.parse().ok().map(Message::Activite),
            _ => None,
        }
    }
}

/// Suite à donner dans cet onglet à un message d'un autre onglet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuiteDiffusion {
    Ignorer,
    /// Session terminée ailleurs : retour à la connexion.
    Deconnexion,
    /// Un autre compte a pris la place : tout recharger, sans quoi l'identité affichée ne
    /// serait plus celle de la session.
    Recharger,
    /// Le même compte s'est reconnecté ailleurs : fermer la fenêtre de reconnexion et
    /// relancer les chargements.
    Rouvrir,
}

/// Décision pure : `utilisateur` est la personne avec laquelle **cet onglet** a chargé son état.
pub fn suite_message(message: Message, utilisateur: Option<Uuid>) -> SuiteDiffusion {
    match message {
        Message::Fin => {
            if utilisateur.is_some() {
                SuiteDiffusion::Deconnexion
            } else {
                SuiteDiffusion::Ignorer
            }
        }
        Message::Ouverte(id) => match utilisateur {
            Some(u) if u == id => SuiteDiffusion::Rouvrir,
            Some(_) => SuiteDiffusion::Recharger,
            // Pas encore d'identité (page de connexion) : rien à corriger.
            None => SuiteDiffusion::Ignorer,
        },
        Message::Activite(_) => SuiteDiffusion::Ignorer,
    }
}

#[cfg(target_arch = "wasm32")]
mod canal {
    use super::Message;
    use std::cell::RefCell;
    use std::rc::Rc;

    use leptos::wasm_bindgen::closure::Closure;
    use leptos::wasm_bindgen::JsCast;

    type Gestionnaire = Rc<dyn Fn(Message)>;

    thread_local! {
        static CANAL: RefCell<Option<web_sys::BroadcastChannel>> = const { RefCell::new(None) };
        static GESTIONNAIRE: RefCell<Option<Gestionnaire>> = const { RefCell::new(None) };
    }

    fn canal() -> Option<web_sys::BroadcastChannel> {
        CANAL.with(|c| {
            if c.borrow().is_none() {
                let ch = web_sys::BroadcastChannel::new("ent-session").ok()?;
                let rappel = Closure::<dyn Fn(web_sys::MessageEvent)>::new(
                    move |ev: web_sys::MessageEvent| {
                        let Some(m) = ev.data().as_string().and_then(|t| Message::decoder(&t))
                        else {
                            return;
                        };
                        if let Message::Activite(t) = m {
                            ent_web_api::noter_activite(t);
                        }
                        if let Some(g) = GESTIONNAIRE.with(|g| g.borrow().clone()) {
                            g(m);
                        }
                    },
                );
                ch.set_onmessage(Some(rappel.as_ref().unchecked_ref()));
                // Un seul canal par onglet, pour toute sa durée de vie.
                rappel.forget();
                *c.borrow_mut() = Some(ch);
            }
            c.borrow().clone()
        })
    }

    pub fn annoncer(m: Message) {
        if let Some(c) = canal() {
            let _ = c.post_message(&m.encoder().into());
        }
    }

    pub fn ecouter(g: Option<Gestionnaire>) {
        if g.is_some() {
            let _ = canal();
        }
        GESTIONNAIRE.with(|cell| *cell.borrow_mut() = g);
    }
}

/// Prévient les autres onglets (navigateur seulement ; sans effet ailleurs).
pub fn annoncer(message: Message) {
    #[cfg(target_arch = "wasm32")]
    canal::annoncer(message);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = message;
}

/// Branche (ou débranche avec `None`) le gestionnaire des messages des autres onglets.
pub fn ecouter(gestionnaire: Option<Rc<dyn Fn(Message)>>) {
    #[cfg(target_arch = "wasm32")]
    canal::ecouter(gestionnaire);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = gestionnaire;
}

/// Client API de même origine : relais `/web/api`, cookie de session joint par le navigateur.
pub fn client() -> ApiClient {
    ApiClient::relais()
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
    fn messages_entre_onglets_sans_secret() {
        let id = Uuid::from_u128(7);
        for m in [Message::Fin, Message::Ouverte(id), Message::Activite(1_234)] {
            assert_eq!(Message::decoder(&m.encoder()), Some(m));
        }
        assert_eq!(Message::decoder("ouverte:pas-un-uuid"), None);
        assert_eq!(Message::decoder("inconnu:1"), None);
        assert_eq!(Message::decoder(""), None);
    }

    #[test]
    fn synchro_entre_onglets() {
        use SuiteDiffusion::*;
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        assert_eq!(suite_message(Message::Fin, Some(a)), Deconnexion);
        assert_eq!(suite_message(Message::Fin, None), Ignorer);
        assert_eq!(suite_message(Message::Ouverte(a), Some(a)), Rouvrir);
        assert_eq!(suite_message(Message::Ouverte(b), Some(a)), Recharger);
        assert_eq!(suite_message(Message::Ouverte(a), None), Ignorer);
        assert_eq!(suite_message(Message::Activite(5), Some(a)), Ignorer);
    }

    #[test]
    fn le_client_de_la_page_n_a_jamais_de_jeton() {
        let c = client();
        assert!(c.est_relais() && !c.a_un_jeton());
    }

    /// Garde-fou A6 : aucun code de la page n'écrit ni ne lit un jeton dans le stockage du
    /// navigateur. Seul `session.rs` touche `localStorage`, uniquement pour **effacer** l'ancienne
    /// clé (migration douce).
    #[test]
    fn aucun_acces_au_stockage_du_navigateur_pour_le_jeton() {
        use std::path::Path;
        fn sources(dossier: &Path, sortie: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dossier).unwrap() {
                let chemin = e.unwrap().path();
                if chemin.is_dir() {
                    sources(&chemin, sortie);
                } else if chemin.extension().is_some_and(|x| x == "rs") {
                    sortie.push(chemin);
                }
            }
        }
        let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut fichiers = vec![];
        sources(&racine, &mut fichiers);
        sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../ent-web-api/src"),
            &mut fichiers,
        );
        let interdits = [
            "local_storage",
            "session_storage",
            "set_item",
            "get_item",
            "localStorage",
            "sessionStorage",
            "IndexedDb",
            "document.cookie",
            "document().cookie",
            "html_document",
        ];
        for f in fichiers {
            let texte = std::fs::read_to_string(&f).unwrap();
            // Le code de test (ce fichier contient la liste ci-dessus) n'est pas du code de la page.
            let code: String = texte
                .split("#[cfg(test)]")
                .next()
                .unwrap()
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            let est_session = f.ends_with("noyau/session.rs");
            for mot in interdits {
                if est_session && matches!(mot, "local_storage" | "get_item") {
                    continue; // lecture de l'ancienne clé pour l'effacer (voir ci-dessous)
                }
                assert!(
                    !code.contains(mot),
                    "{} utilise « {mot} » : le jeton ne doit jamais toucher le stockage du navigateur",
                    f.display()
                );
            }
            if est_session {
                assert!(code.contains("remove_item(ANCIENNE_CLE_JETON)"));
                assert!(
                    !code.contains("set_item"),
                    "session.rs n'écrit rien dans le stockage"
                );
            }
        }
    }

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
}
