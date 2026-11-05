//! Client typé des routes JSON **existantes** d'`ent-api`.
//!
//! Le backend n'est pas modifié : ces types reproduisent le contrat JSON de
//! `ent-api/src/dto.rs` (privé et sérialisation seule côté serveur). À terme, une
//! crate de types partagée (`ent-types`) supprimerait cette copie — évolution
//! backend à valider séparément.
//!
//! Transport :
//! - navigateur (`wasm32`) : `gloo-net` (fetch), chemins relatifs (même origine) ;
//! - natif avec la feature `natif` : `reqwest` (tests d'intégration) ;
//! - natif sans feature (rendu serveur) : non disponible — les données métier sont
//!   chargées côté navigateur, car le jeton de session n'est pas visible du serveur.

use serde::{Deserialize, Serialize};

mod transport;

// Services par domaine (méthodes de `ApiClient`).
mod appel;
mod auth;
mod communication;
mod dossier;
mod edt;
mod referentiel;
mod securite;
mod vie_scolaire;

pub use communication::{
    base64_encoder, encoder_requete, Annonce, DetailFil, EleveTrouve, EnfantFamille, FilBoite,
    FilMessages, LibelleEleve, MessageFil, ModeleMessage, PieceMessage,
};
pub use securite::EtatSession;
pub use securite::{
    AppareilSession, ChallengeEnrolement, MoiSecurite, PasskeyCompte, PolitiqueSecurite,
    ENROLEMENT_REQUIS,
};
pub use vie_scolaire::DecisionApi;

// ---------------------------------------------------------------------------
// Types (contrat JSON d'ent-api)
// ---------------------------------------------------------------------------

// Types partagés avec ent-api : crate `ent-types` (lot 1). Les noms historiques
// du prototype sont conservés comme alias.
pub use ent_types::appel::{AppelDto as AppelApi, AppelLigneDto as LigneAppelApi};
pub use ent_types::auth::{
    AuthSessionDto as Session, LoginResponseDto, MeDto as Moi, PasskeyChallengeDto,
};
pub use ent_types::dossier::{
    ActionFicheDto, ContactBody, ContactUrgenceDto, EntreeChronologieDto, FicheEleveDto, LienBody,
    PersonneBody, PersonneDto, ResponsableDto, ResumeSectionsDto, SectionsDto,
};
pub use ent_types::edt::EdtCreneauDto as Creneau;
pub use ent_types::referentiel::{
    ClasseDetailDto as ClasseDetailApi, ClasseDto as ClasseApi, EleveClasseDto as EleveClasseApi,
    RapportAmorcageDto as RapportAmorcageApi, ServiceDto as ServiceApi,
};
pub use ent_types::vie_scolaire::{
    AbsenceDto as Absence, AbsenceMotifDto as MotifAbsence, EleveFicheDto as FicheEleve,
    EnfantFamilleDto as Enfant, FamilleResumeDto as ResumeFamille, RetardDto as Retard,
    SignalAbsenceDto as SignalAbsence,
};

/// Issue d'une connexion par mot de passe.
#[derive(Clone, Debug, PartialEq)]
pub enum Connexion {
    Ouverte(Session),
    /// Le compte a une passkey : second facteur exigé (défi WebAuthn à signer).
    PasskeyRequise(Box<PasskeyChallengeDto>),
}

/// État d'une ligne tel que l'API le connaît (`present|absent|retard`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EtatApi {
    Present,
    Absent,
    Retard,
}

impl EtatApi {
    pub fn as_str(self) -> &'static str {
        match self {
            EtatApi::Present => "present",
            EtatApi::Absent => "absent",
            EtatApi::Retard => "retard",
        }
    }
}

static DERNIERE_ACTIVITE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Instant (secondes Unix) du dernier échange avec le serveur où la session a été
/// reconnue (toute réponse autre que 401/5xx : le serveur a alors compté une activité).
/// 0 si aucun. Sert à l'alerte avant expiration (inactivité de 2 h, D-011).
pub fn derniere_activite() -> i64 {
    DERNIERE_ACTIVITE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Enregistre une activité (ici, ou signalée par un autre onglet) ; ne recule jamais.
pub fn noter_activite(instant: i64) {
    DERNIERE_ACTIVITE.fetch_max(instant, std::sync::atomic::Ordering::Relaxed);
}

/// Préfixe du relais de session web (`ent-web-session`) : cookie → Bearer côté serveur.
pub const PREFIXE_RELAIS: &str = "/web/api";

/// Ce que l'API sait enregistrer. L'écran s'adapte à ces capacités au lieu de
/// promettre un enregistrement qui n'existe pas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capacites {
    /// État « dispensé » sur une ligne d'appel.
    pub dispense: bool,
    /// Commentaire libre par ligne.
    pub commentaire: bool,
}

/// Capacités de l'`ent-api` actuel : ni « dispensé » ni commentaire
/// (`ent_appel_ligne.etat IN ('present','absent','retard')`, pas de colonne commentaire).
pub const CAPACITES_API_ACTUELLE: Capacites = Capacites {
    dispense: false,
    commentaire: false,
};

// ---------------------------------------------------------------------------
// Erreurs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ApiError {
    #[error("session expirée ou absente")]
    NonConnecte,
    #[error("accès refusé : {0}")]
    Interdit(String),
    #[error("introuvable : {0}")]
    Introuvable(String),
    #[error("requête refusée : {0}")]
    Invalide(String),
    #[error("conflit : {0}")]
    Conflit(String),
    #[error("trop de tentatives, réessayez plus tard")]
    TropDeRequetes,
    #[error("erreur serveur ({0}) : {1}")]
    Serveur(u16, String),
    #[error("serveur injoignable : {0}")]
    Reseau(String),
    #[error("réponse illisible : {0}")]
    Decodage(String),
    #[error("appel réseau indisponible dans ce contexte")]
    Indisponible,
}

/// Longueur maximale (en caractères) d'un message serveur repris à l'écran.
const LONGUEUR_MAX_MESSAGE: usize = 160;

/// Message serveur ramené à une seule ligne de longueur bornée (le serveur reste
/// responsable de son contenu ; le client n'affiche jamais un corps brut sans limite).
fn ligne_bornee(texte: &str) -> String {
    let une_ligne = texte.split_whitespace().collect::<Vec<_>>().join(" ");
    let une_ligne = une_ligne.trim_end_matches(['.', ' ']);
    if une_ligne.chars().count() > LONGUEUR_MAX_MESSAGE {
        let coupe: String = une_ligne.chars().take(LONGUEUR_MAX_MESSAGE).collect();
        format!("{}…", coupe.trim_end())
    } else {
        une_ligne.to_string()
    }
}

impl ApiError {
    /// Le serveur exige d'abord l'enrôlement d'une passkey (session « enrôlement ») :
    /// 403 avec le corps texte `passkey_enrolement_requis`.
    pub fn est_enrolement_requis(&self) -> bool {
        matches!(self, ApiError::Interdit(corps) if corps.trim() == ENROLEMENT_REQUIS)
    }

    /// Message destiné à l'utilisateur (français, sans détail technique inutile).
    pub fn message_utilisateur(&self) -> String {
        match self {
            ApiError::NonConnecte => "Votre session a expiré. Reconnectez-vous.".into(),
            ApiError::Interdit(_) if self.est_enrolement_requis() => {
                "Vous devez d'abord enregistrer une passkey pour utiliser votre compte.".into()
            }
            ApiError::Interdit(_) => {
                "Vous n'avez pas les droits nécessaires pour cette action.".into()
            }
            ApiError::Introuvable(_) => "Élément introuvable.".into(),
            ApiError::Invalide(m) | ApiError::Conflit(m) => {
                format!("Action refusée : {}.", ligne_bornee(m))
            }
            ApiError::TropDeRequetes => "Trop de tentatives. Patientez avant de réessayer.".into(),
            ApiError::Serveur(..) => "Le serveur a rencontré une erreur. Réessayez.".into(),
            ApiError::Reseau(_) => {
                "Connexion au serveur impossible. Vérifiez le réseau puis réessayez.".into()
            }
            ApiError::Decodage(_) => "Réponse inattendue du serveur.".into(),
            ApiError::Indisponible => "Fonction indisponible ici.".into(),
        }
    }

    /// Traduit un statut HTTP + corps texte (convention d'ent-api : `(StatusCode, String)`).
    pub fn depuis_statut(statut: u16, corps: String) -> ApiError {
        match statut {
            401 => ApiError::NonConnecte,
            403 => ApiError::Interdit(corps),
            404 => ApiError::Introuvable(corps),
            400 | 422 => ApiError::Invalide(corps),
            409 => ApiError::Conflit(corps),
            429 => ApiError::TropDeRequetes,
            s => ApiError::Serveur(s, corps),
        }
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client HTTP. `base` vide = même origine (navigateur).
#[derive(Clone, Default)]
pub struct ApiClient {
    base: String,
    jeton: Option<String>,
    /// Session web par cookie `HttpOnly` : appels via le relais `/web/api`, aucun jeton.
    relais: bool,
}

// `Debug` manuel : le jeton de session ne doit jamais apparaître dans un journal ou une panique.
impl std::fmt::Debug for ApiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiClient")
            .field("base", &self.base)
            .field("jeton", &self.jeton.as_ref().map(|_| "<masqué>"))
            .field("relais", &self.relais)
            .finish()
    }
}

pub(crate) enum Methode {
    Get,
    Post,
    Put,
    Delete,
}

impl ApiClient {
    /// Client à jeton Bearer (applis mobiles, tests contre le backend). Le navigateur
    /// n'utilise **pas** ce mode : voir [`ApiClient::relais`].
    pub fn new(base: impl Into<String>, jeton: Option<String>) -> Self {
        Self {
            base: base.into(),
            jeton,
            relais: false,
        }
    }

    /// Client du navigateur (A6) : même origine, appels via `/web/api/…` avec le cookie de
    /// session `HttpOnly` que le navigateur joint seul. Aucun jeton n'existe côté client.
    pub fn relais() -> Self {
        Self {
            base: String::new(),
            jeton: None,
            relais: true,
        }
    }

    /// Vrai pour le client du navigateur (cookie), faux pour un client à jeton Bearer.
    pub fn est_relais(&self) -> bool {
        self.relais
    }

    pub fn avec_jeton(&self, jeton: Option<String>) -> Self {
        Self {
            base: self.base.clone(),
            jeton,
            relais: false,
        }
    }

    pub(crate) fn auth(&self) -> transport::Auth<'_> {
        if self.relais {
            transport::Auth::Cookie
        } else if let Some(j) = self.jeton.as_deref() {
            transport::Auth::Bearer(j)
        } else {
            transport::Auth::Aucun
        }
    }

    pub fn a_un_jeton(&self) -> bool {
        self.jeton.is_some()
    }

    /// Adresse d'une route d'ent-api (via le relais `/web/api` en mode cookie).
    fn url(&self, chemin: &str) -> String {
        if self.relais {
            format!("{PREFIXE_RELAIS}{chemin}")
        } else {
            format!("{}{}", self.base, chemin)
        }
    }

    async fn appel<T: serde::de::DeserializeOwned>(
        &self,
        methode: Methode,
        chemin: &str,
        corps: Option<serde_json::Value>,
    ) -> Result<T, ApiError> {
        self.appel_adresse(methode, &self.url(chemin), corps).await
    }

    /// Comme [`Self::appel`], avec l'adresse complète (routes `/web/session/…` du relais).
    async fn appel_adresse<T: serde::de::DeserializeOwned>(
        &self,
        methode: Methode,
        adresse: &str,
        corps: Option<serde_json::Value>,
    ) -> Result<T, ApiError> {
        let (statut, texte) = transport::envoyer(methode, adresse, self.auth(), corps).await?;
        if statut != 401 && statut < 500 {
            noter_activite(chrono::Utc::now().timestamp());
        }
        if (200..300).contains(&statut) {
            let texte = if texte.trim().is_empty() {
                "null".to_string()
            } else {
                texte
            };
            serde_json::from_str(&texte).map_err(|e| ApiError::Decodage(e.to_string()))
        } else {
            Err(ApiError::depuis_statut(statut, texte))
        }
    }

    /// Accès générique (tests et futurs modules) : `GET` d'un chemin JSON.
    pub async fn get_json(&self, chemin: &str) -> Result<serde_json::Value, ApiError> {
        self.appel(Methode::Get, chemin, None).await
    }

    /// Accès générique : `POST` JSON.
    pub async fn post_json(
        &self,
        chemin: &str,
        corps: serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        self.appel(Methode::Post, chemin, Some(corps)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_jeton_n_apparait_pas_dans_debug() {
        let c = ApiClient::new("http://x", Some("SECRET-TOKEN-123".into()));
        let d = format!("{c:?}");
        assert!(!d.contains("SECRET-TOKEN-123"), "{d}");
        assert!(d.contains("masqué") && d.contains("http://x"));
        assert!(!format!("{:?}", ApiClient::default()).contains("masqué"));
        assert!(!format!("{:?}", ApiClient::relais()).contains("masqué"));
        let s = Session {
            token: "SECRET-TOKEN-456".into(),
            user_id: uuid::Uuid::nil(),
            email: "a@b".into(),
            etablissement_id: uuid::Uuid::nil(),
            role: "prof".into(),
            expires_at: chrono::Utc::now(),
        };
        assert!(!format!("{s:?}").contains("SECRET-TOKEN-456"));
    }

    #[test]
    fn l_activite_ne_recule_jamais() {
        noter_activite(1_000);
        noter_activite(500);
        assert!(derniere_activite() >= 1_000);
    }

    #[test]
    fn le_client_du_navigateur_passe_par_le_relais_sans_jeton() {
        let c = ApiClient::relais();
        assert!(c.est_relais() && !c.a_un_jeton());
        assert_eq!(c.url("/auth/me"), "/web/api/auth/me");
        assert_eq!(c.url("/vs/absences?x=1"), "/web/api/vs/absences?x=1");
        // Client à jeton (mobile, tests) : adresses inchangées.
        let b = ApiClient::new("http://h", Some("t".into()));
        assert!(!b.est_relais() && b.a_un_jeton());
        assert_eq!(b.url("/auth/me"), "http://h/auth/me");
        // Un jeton fourni à un client relais est impossible : `avec_jeton` repasse en Bearer.
        assert!(!c.avec_jeton(Some("t".into())).est_relais());
    }

    #[test]
    fn message_serveur_borne_et_sur_une_ligne() {
        let long = format!("ligne un\n<script>alert(1)</script>\n{}", "x".repeat(1000));
        let m = ApiError::Invalide(long).message_utilisateur();
        assert!(!m.contains('\n'));
        assert!(m.chars().count() < 200, "{}", m.chars().count());
        assert!(m.ends_with("…."));
        assert_eq!(
            ApiError::Conflit("déjà clos.".into()).message_utilisateur(),
            "Action refusée : déjà clos."
        );
    }

    #[test]
    fn statuts_http_vers_erreurs() {
        assert_eq!(
            ApiError::depuis_statut(401, "x".into()),
            ApiError::NonConnecte
        );
        assert!(matches!(
            ApiError::depuis_statut(403, "x".into()),
            ApiError::Interdit(_)
        ));
        assert!(matches!(
            ApiError::depuis_statut(400, "x".into()),
            ApiError::Invalide(_)
        ));
        assert!(matches!(
            ApiError::depuis_statut(503, "x".into()),
            ApiError::Serveur(503, _)
        ));
    }

    #[test]
    fn enrolement_requis_reconnu_dans_un_403() {
        let e = ApiError::depuis_statut(403, "passkey_enrolement_requis".into());
        assert!(e.est_enrolement_requis());
        assert!(
            ApiError::depuis_statut(403, " passkey_enrolement_requis\n".into())
                .est_enrolement_requis()
        );
        assert!(
            !ApiError::depuis_statut(403, "réservé à la direction".into()).est_enrolement_requis()
        );
        assert!(
            !ApiError::depuis_statut(401, "passkey_enrolement_requis".into())
                .est_enrolement_requis()
        );
        assert!(e.message_utilisateur().contains("passkey"));
    }

    #[test]
    fn decode_appel_du_contrat_ent_api() {
        let json = r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","classe":"4B",
            "jour":"2026-11-02","statut":"ouvert","creneau_id":null,
            "matiere_libelle":"Potions","lignes":[{"eleve_id":"6f1c3c80-0000-4000-8000-000000000002",
            "eleve_label":"POTTER Harry","etat":"present"}]}"#;
        let a: AppelApi = serde_json::from_str(json).unwrap();
        assert_eq!(a.lignes.len(), 1);
        assert!(!a.est_clos());
    }

    #[test]
    fn decode_connexion_aplatie() {
        let json = r#"{"mfa_required":false,"token":"t","user_id":"6f1c3c80-0000-4000-8000-000000000001",
            "email":"a@b","etablissement_id":"6f1c3c80-0000-4000-8000-000000000002","role":"prof",
            "expires_at":"2026-11-03T00:00:00Z"}"#;
        let r: LoginResponseDto = serde_json::from_str(json).unwrap();
        assert!(r.session.is_some());
        let json2 = r#"{"mfa_required":true,"passkey":{"token":"x","options":{},"expires_at":"2026-11-03T00:00:00Z"}}"#;
        let r2: LoginResponseDto = serde_json::from_str(json2).unwrap();
        assert!(r2.mfa_required);
    }
}
