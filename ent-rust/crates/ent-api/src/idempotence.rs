//! `Idempotency-Key` générique (socle 11, lot MB1, `docs/socles/11_APPLICATIONS_MOBILES.md` §5.2 et §5.5).
//!
//! Couche commune posée **à l'intérieur** de la résolution de session ([`crate::auth::contexte_droits`]) : elle lit la
//! session déjà résolue, jamais ses propres droits. Sur toute écriture authentifiée (`POST`, `PUT`, `PATCH`,
//! `DELETE`) :
//!
//! | `Idempotency-Key` | requête d'appli (`X-ScolIA-Appli`) | requête web (une route `/mobile/*` sans cet en-tête compte comme le web) |
//! |---|---|---|
//! | absente | `400 idempotence.requise` | exécutée normalement |
//! | invalide (pas un UUID) | `400 idempotence.cle_invalide` | `400 idempotence.cle_invalide` |
//! | présente | registre d'idempotence (24 h) | registre d'idempotence (24 h) |
//!
//! Avec le registre (`ent_idempotence_cle`, migration 2227, cloisonné par établissement **et** session) : même clé +
//! même empreinte (méthode, chemin, corps) → réponse rejouée (`Idempotent-Replayed: true`) ; même clé + autre
//! empreinte → `409 idempotence.conflit` ; deux requêtes concurrentes de même clé → une seule s'exécute, l'autre attend
//! sa réponse (≤ 10 s) puis la rejoue, sinon `409 idempotence.en_cours`.
//!
//! Règles prudentes (D-069, `docs/socles/11_MB1_SERVEUR_LIVRAISON.md`) : une requête **sans session** (connexion,
//! rattachement) n'est pas concernée (rien à cloisonner) ; seules les réponses de succès (2xx, 3xx) sont conservées —
//! un refus ou une erreur libère la clé (un nouvel essai s'exécute pour de bon, et ne rejoue jamais un refus de droit
//! devenu faux) ; une réponse de plus de 256 Kio ou non textuelle est marquée non rejouable (un rejeu reçoit
//! `409 idempotence.non_rejouable`, jamais une seconde exécution).
//!
//! Limites connues et assumées (faibles F2, F3 et F6 de la contre-revue MB1, D-069) :
//! - **F2** : la couche est posée **à l'extérieur** des gardes `sante::exiger_module` et
//!   `eleve_garde::exiger_eleve_visible`. Un rejeu ne repasse donc pas par elles. Ce n'est pas une élévation : seule
//!   la **même session** rejoue **sa propre** réponse de succès, dans les 24 h, et les réponses de santé, de psy et
//!   de finances ne sont jamais conservées (`reponse_sensible`). Une écriture refusée par une garde (403, 404) n'est
//!   jamais conservée non plus.
//! - **F3** : une requête dont la clé est « en cours » interroge le registre toutes les 50 ms, 10 s au plus (200
//!   lectures). Palier des faibles n° 2 : **au plus [`ATTENTES_MAX_PAR_SESSION`] requêtes d'une même session
//!   attendent à la fois** (compteur du processus) ; au-delà, `409 idempotence.en_cours` tout de suite, avec
//!   `Retry-After`. Une session ne peut donc plus occuper le registre par une rafale de requêtes de même clé.
//! - **F6** : le corps d'une écriture avec clé est lu en mémoire pour l'empreinte, dans la limite de
//!   `max_body_bytes()` (la même que `RequestBodyLimitLayer`) : aucune lecture non bornée.
//!
//! **Délai d'exécution** (palier des faibles n° 2, reste du moyen M2 de la contre-revue MB1). L'exécution tourne
//! dans une tâche détachée : le délai de requête (`TimeoutLayer`, `ENT_REQUEST_TIMEOUT_SECS`) ne l'arrête pas.
//! Sans borne propre, une exécution plus longue que [`IDEMPOTENCE_ABANDON_S`] voyait sa revendication reprise par
//! un nouvel essai : seconde exécution. Elle est donc bornée à [`IDEMPOTENCE_EXECUTION_MAX_S`] (strictement moins
//! que le délai d'abandon) ; au-delà, l'exécution est interrompue (sa transaction est annulée) et la clé est close
//! **non rejouable** (D-069 : jamais une seconde exécution, même si l'écriture avait abouti juste avant la borne).
//! La réponse est `408 delai.depasse`. La borne vaut quel que soit `ENT_REQUEST_TIMEOUT_SECS` : un délai de requête
//! plus long ne prolonge pas une écriture idempotente. Limites connues : un handler interrompu après son propre
//! `commit` (effets qui suivent la validation) l'est à mi-chemin ; si la clôture de la clé échoue trois fois (base
//! indisponible), la clé reste « en cours » et peut être reprise après le délai d'abandon (journalisé en erreur).

use crate::enveloppe::{avec_code, declare_appli};
use crate::AppState;
use axum::body::{Body, HttpBody};
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, Response, StatusCode};
use axum::middleware::Next;
use ent_db::idempotence::{
    empreinte_requete, ReponseConservee, RevendicationCle, IDEMPOTENCE_ABANDON_S,
    IDEMPOTENCE_CORPS_MAX, IDEMPOTENCE_EXECUTION_MAX_S,
};
use ent_types::erreurs::CodeErreur;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

/// Attente maximale d'une requête dont la clé est en cours chez une autre requête.
const ATTENTE_MAX_MS: u64 = 10_000;
const PAS_ATTENTE_MS: u64 = 50;

/// F3 (palier des faibles n° 2) : nombre maximal de requêtes d'une même session qui attendent à la fois une clé en
/// cours, dans ce processus.
pub(crate) const ATTENTES_MAX_PAR_SESSION: usize = 4;

const _: () = assert!(IDEMPOTENCE_EXECUTION_MAX_S < IDEMPOTENCE_ABANDON_S);

fn attentes() -> &'static Mutex<HashMap<Uuid, usize>> {
    static A: OnceLock<Mutex<HashMap<Uuid, usize>>> = OnceLock::new();
    A.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Place d'attente d'une session : rendue à la destruction (fin de l'attente, réponse, abandon du client).
struct PlaceAttente(Uuid);

impl PlaceAttente {
    /// `None` si la session a déjà [`ATTENTES_MAX_PAR_SESSION`] requêtes en attente.
    fn prendre(session: Uuid) -> Option<Self> {
        let mut a = attentes().lock().unwrap_or_else(|e| e.into_inner());
        let n = a.entry(session).or_insert(0);
        if *n >= ATTENTES_MAX_PAR_SESSION {
            return None;
        }
        *n += 1;
        Some(PlaceAttente(session))
    }
}

impl Drop for PlaceAttente {
    fn drop(&mut self) {
        let mut a = attentes().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = a.get_mut(&self.0) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                a.remove(&self.0);
            }
        }
    }
}

fn reponse_en_cours() -> Response<Body> {
    let mut res = avec_code(
        CodeErreur::IdempotenceEnCours,
        "Cette opération est déjà en cours de traitement.",
    );
    res.headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    res
}

/// Borne de l'exécution d'une écriture idempotente. Les tests peuvent la raccourcir pour **leur** établissement
/// (aucun effet sur les autres tests du même processus).
fn delai_execution(etab: Uuid) -> std::time::Duration {
    #[cfg(test)]
    if let Some(ms) = delais_de_test()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&etab)
    {
        return std::time::Duration::from_millis(*ms);
    }
    let _ = etab;
    std::time::Duration::from_secs(IDEMPOTENCE_EXECUTION_MAX_S as u64)
}

#[cfg(test)]
fn delais_de_test() -> &'static Mutex<HashMap<Uuid, u64>> {
    static D: OnceLock<Mutex<HashMap<Uuid, u64>>> = OnceLock::new();
    D.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Tests : borne d'exécution raccourcie pour un établissement.
#[cfg(all(test, feature = "labo"))]
pub(crate) fn raccourcir_delai_execution(etab: Uuid, ms: u64) {
    delais_de_test()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(etab, ms);
}

/// Tests : nombre de requêtes en attente d'une session.
#[cfg(test)]
pub(crate) fn attentes_de(session: Uuid) -> usize {
    attentes()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&session)
        .copied()
        .unwrap_or(0)
}

fn est_ecriture(m: &Method) -> bool {
    matches!(
        *m,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// Contre-revue du pilote (MB1, M3) : l'authentification (mots de passe, jetons) n'entre **jamais** au registre.
fn hors_registre(chemin: &str) -> bool {
    chemin.starts_with("/auth")
}

/// Contre-revue du pilote (MB1, M1, M3) : routes dont la réponse porte des données sensibles (santé, psy, finances,
/// coordonnées bancaires) ou un lien pré-signé à durée courte : aucune réponse n'est conservée (rejeu = `409
/// idempotence.non_rejouable`, jamais une seconde exécution) et l'empreinte ne porte pas sur le corps (aucun secret
/// haché au repos).
fn reponse_sensible(chemin: &str) -> bool {
    let p = chemin.split('?').next().unwrap_or(chemin);
    [
        "/sante",
        "/psy",
        "/finances",
        "/facturation",
        "/paiement",
        "/iban",
        "/fin/",
    ]
    .iter()
    .any(|m| p.contains(m))
        || p.ends_with("/lien")
}

const TYPE_CONTENU_MAX: usize = 120;
const EMPLACEMENT_MAX: usize = 500;

fn rejouer(r: ReponseConservee) -> Response<Body> {
    let statut = StatusCode::from_u16(r.statut_http).unwrap_or(StatusCode::OK);
    let mut res = Response::new(Body::from(r.corps.unwrap_or_default()));
    *res.status_mut() = statut;
    if let Some(Ok(v)) = r.type_contenu.as_deref().map(HeaderValue::from_str) {
        res.headers_mut().insert(header::CONTENT_TYPE, v);
    }
    if let Some(Ok(v)) = r.emplacement.as_deref().map(HeaderValue::from_str) {
        res.headers_mut().insert(header::LOCATION, v);
    }
    res.headers_mut()
        .insert("idempotent-replayed", HeaderValue::from_static("true"));
    res
}

pub async fn idempotence(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response<Body> {
    if !est_ecriture(req.method()) || hors_registre(req.uri().path()) {
        return next.run(req).await;
    }
    // Sans session résolue (connexion, rattachement, jeton invalide) : rien à cloisonner, la route répond seule.
    let Some(info) = req
        .extensions()
        .get::<crate::auth::SessionPreResolue>()
        .map(|s| s.0.clone())
    else {
        return next.run(req).await;
    };
    let appli = declare_appli(req.headers());
    let cle_brute = req
        .headers()
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_string());
    let Some(cle_brute) = cle_brute else {
        if appli {
            return avec_code(
                CodeErreur::IdempotenceRequise,
                "L'en-tête Idempotency-Key est obligatoire sur une écriture.",
            );
        }
        return next.run(req).await;
    };
    let Some(cle) = (cle_brute.len() == 36)
        .then(|| Uuid::parse_str(&cle_brute).ok())
        .flatten()
    else {
        return avec_code(
            CodeErreur::IdempotenceCleInvalide,
            "L'en-tête Idempotency-Key doit être un UUID.",
        );
    };

    let (parts, body) = req.into_parts();
    let Ok(octets) = axum::body::to_bytes(body, crate::max_body_bytes()).await else {
        return avec_code(
            CodeErreur::CorpsTropGros,
            "Le contenu envoyé est trop volumineux.",
        );
    };
    let chemin = parts
        .uri
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| parts.uri.path().to_string());
    let sensible = reponse_sensible(&chemin);
    let empreinte = if sensible {
        empreinte_requete(parts.method.as_str(), &chemin, b"")
    } else {
        empreinte_requete(parts.method.as_str(), &chemin, &octets)
    };

    let (etab, session) = (info.etablissement_id, info.session_id);
    let mut attendu_ms = 0;
    let mut place: Option<PlaceAttente> = None;
    loop {
        let maintenant = (state.horloge)();
        match state
            .ent
            .idem_revendiquer(etab, session, cle, &empreinte, maintenant)
            .await
        {
            Ok(RevendicationCle::Nouvelle) => break,
            Ok(RevendicationCle::Rejeu(r)) => return rejouer(r),
            Ok(RevendicationCle::Conflit) => {
                return avec_code(
                    CodeErreur::IdempotenceConflit,
                    "Cet identifiant d'idempotence a déjà servi pour une autre requête.",
                )
            }
            Ok(RevendicationCle::NonRejouable) => {
                return avec_code(
                    CodeErreur::IdempotenceNonRejouable,
                    "Cette opération a déjà été traitée ; sa réponse n'est plus disponible.",
                )
            }
            Ok(RevendicationCle::EnCours) => {
                if attendu_ms >= ATTENTE_MAX_MS {
                    return reponse_en_cours();
                }
                // F3 : une place d'attente par requête, au plus ATTENTES_MAX_PAR_SESSION par session.
                if place.is_none() {
                    match PlaceAttente::prendre(session) {
                        Some(p) => place = Some(p),
                        None => return reponse_en_cours(),
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(PAS_ATTENTE_MS)).await;
                attendu_ms += PAS_ATTENTE_MS;
            }
            Err(e) => {
                tracing::error!(error = %e, "registre d'idempotence indisponible");
                return avec_code(CodeErreur::ErreurInterne, "Erreur interne.");
            }
        }
    }

    // Fin de l'attente : la place est rendue avant l'exécution (reprise d'une revendication abandonnée).
    drop(place);

    // Contre-revue du pilote (MB1, M2) : l'exécution ET la conservation tournent dans une tâche détachée : un client qui
    // coupe la connexion après l'écriture métier n'empêche plus la conservation (sinon la clé restait « en cours » et une
    // reprise après abandon exécutait l'écriture une seconde fois).
    let tache = tokio::spawn(executer_et_conserver(
        state, next, parts, octets, etab, session, cle, sensible,
    ));
    match tache.await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "exécution idempotente interrompue");
            avec_code(CodeErreur::ErreurInterne, "Erreur interne.")
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn executer_et_conserver(
    state: AppState,
    next: Next,
    parts: axum::http::request::Parts,
    octets: axum::body::Bytes,
    etab: Uuid,
    session: Uuid,
    cle: Uuid,
    sensible: bool,
) -> Response<Body> {
    let execution = next.run(Request::from_parts(parts, Body::from(octets)));
    let res = match tokio::time::timeout(delai_execution(etab), execution).await {
        Ok(res) => res,
        Err(_) => {
            // Exécution interrompue à la borne (sa transaction est annulée à l'abandon du futur) : clé close non
            // rejouable, jamais libérée (un nouvel essai ne s'exécute pas une seconde fois).
            tracing::error!(
                borne_s = IDEMPOTENCE_EXECUTION_MAX_S,
                "écriture idempotente interrompue à la borne d'exécution ; clé close non rejouable"
            );
            // La table n'admet que des statuts de succès (200 à 399, migration 2227) : 202 (« accepté, issue
            // inconnue ») marque la clé ; une clé non rejouable ne sert jamais son statut (le rejeu reçoit 409).
            let close = ReponseConservee {
                statut_http: StatusCode::ACCEPTED.as_u16(),
                type_contenu: None,
                emplacement: None,
                corps: None,
                rejouable: false,
            };
            // Contre-revue du palier : trois essais (base souvent lente à la borne) ; une clé restée « en cours »
            // serait reprise après le délai d'abandon, donc réexécutée.
            let mut close_ok = false;
            for essai in 0..3u64 {
                match state
                    .ent
                    .idem_terminer(etab, session, cle, &close, (state.horloge)())
                    .await
                {
                    Ok(()) => {
                        close_ok = true;
                        break;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, essai, "clôture de la clé après la borne d'exécution impossible");
                        tokio::time::sleep(std::time::Duration::from_millis(200 * (essai + 1)))
                            .await;
                    }
                }
            }
            if !close_ok {
                tracing::error!("clé d'idempotence restée en cours après la borne d'exécution");
            }
            return avec_code(
                CodeErreur::DelaiDepasse,
                "Le délai de traitement est dépassé.",
            );
        }
    };
    let statut = res.status();
    if !(statut.is_success() || statut.is_redirection()) {
        if let Err(e) = state.ent.idem_abandonner(etab, session, cle).await {
            tracing::error!(error = %e, "libération d'une clé d'idempotence impossible");
        }
        return res;
    }

    let (res_parts, res_body) = res.into_parts();
    let stockable = res_body
        .size_hint()
        .upper()
        .is_some_and(|n| n <= IDEMPOTENCE_CORPS_MAX as u64);
    let (corps_octets, rendu_body) = if stockable {
        match axum::body::to_bytes(res_body, IDEMPOTENCE_CORPS_MAX).await {
            Ok(b) => (Some(b.clone()), Body::from(b)),
            // Ne devrait pas arriver (taille connue) : réponse perdue, on libère la clé.
            Err(_) => {
                let _ = state.ent.idem_abandonner(etab, session, cle).await;
                return avec_code(CodeErreur::ErreurInterne, "Erreur interne.");
            }
        }
    } else {
        (None, res_body)
    };
    let texte = corps_octets
        .as_ref()
        .and_then(|b| String::from_utf8(b.to_vec()).ok());
    let rejouable = stockable && texte.is_some();
    let conservee = ReponseConservee {
        statut_http: statut.as_u16(),
        type_contenu: res_parts
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string),
        emplacement: res_parts
            .headers
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string),
        corps: if rejouable { texte } else { None },
        rejouable,
    };
    // M1 : réponse sensible jamais conservée ; M2 : valeurs hors des bornes de la table (ou `\0`, refusé par Postgres)
    // → non rejouable plutôt qu'un échec de conservation.
    let mut conservee = conservee;
    let hors_bornes = conservee
        .type_contenu
        .as_ref()
        .is_some_and(|t| t.len() > TYPE_CONTENU_MAX)
        || conservee
            .emplacement
            .as_ref()
            .is_some_and(|l| l.len() > EMPLACEMENT_MAX)
        || conservee.corps.as_ref().is_some_and(|c| c.contains('\0'));
    if sensible || hors_bornes {
        conservee = ReponseConservee {
            statut_http: conservee.statut_http,
            type_contenu: None,
            emplacement: None,
            corps: None,
            rejouable: false,
        };
    }
    if let Err(e) = state
        .ent
        .idem_terminer(etab, session, cle, &conservee, (state.horloge)())
        .await
    {
        tracing::error!(error = %e, "conservation d'une réponse d'idempotence impossible ; marquée non rejouable");
        // M2 : en dernier recours, la clé est close « non rejouable » : jamais laissée « en cours » (seconde exécution).
        let minimale = ReponseConservee {
            statut_http: conservee.statut_http,
            type_contenu: None,
            emplacement: None,
            corps: None,
            rejouable: false,
        };
        if let Err(e) = state
            .ent
            .idem_terminer(etab, session, cle, &minimale, (state.horloge)())
            .await
        {
            tracing::error!(error = %e, "clé d'idempotence restée en cours");
        }
    }
    Response::from_parts(res_parts, rendu_body)
}

#[cfg(test)]
mod tests_pilote {
    use super::*;

    /// Contre-revue du pilote (MB1, M1, M3) : l'authentification n'entre jamais au registre ; les routes sensibles et les
    /// liens pré-signés ne conservent aucune réponse.
    #[test]
    fn authentification_hors_registre_et_reponses_sensibles_non_conservees() {
        assert!(hors_registre("/auth/password"));
        assert!(hors_registre("/auth/login"));
        assert!(!hors_registre("/mobile/labo/ecrire"));
        for p in [
            "/eleves/1/sante/fiche",
            "/psy/suivis",
            "/finances/factures/1",
            "/documents/1/lien",
            "/familles/1/iban?x=1",
        ] {
            assert!(reponse_sensible(p), "{p}");
        }
        assert!(!reponse_sensible("/mobile/labo/ecrire"));
        assert!(!reponse_sensible("/messages/lien-utile/envoi"));
    }

    /// F3 (palier des faibles n° 2) : au plus `ATTENTES_MAX_PAR_SESSION` places d'attente par session, rendues à la
    /// destruction ; une autre session a les siennes.
    #[test]
    fn places_d_attente_plafonnees_par_session_et_rendues() {
        let (s1, s2) = (Uuid::new_v4(), Uuid::new_v4());
        let places: Vec<_> = (0..ATTENTES_MAX_PAR_SESSION)
            .map(|_| PlaceAttente::prendre(s1).expect("place libre"))
            .collect();
        assert!(PlaceAttente::prendre(s1).is_none(), "plafond atteint");
        assert!(
            PlaceAttente::prendre(s2).is_some(),
            "autre session : ses propres places"
        );
        assert_eq!(attentes_de(s1), ATTENTES_MAX_PAR_SESSION);
        drop(places);
        assert_eq!(attentes_de(s1), 0, "places rendues");
        assert!(PlaceAttente::prendre(s1).is_some());
    }
}
