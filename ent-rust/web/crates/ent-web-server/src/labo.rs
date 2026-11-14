//! Données de labo du prototype, créées **uniquement via l'API existante**
//! (droits et règles métier appliqués par `ent-api`). Données fictives.
//!
//! - `POST /labo/bootstrap` : distribution de labo (`seed_labo_cast`) ;
//! - classe `PROTO38` (38 élèves) et classe `PROTO400` (400 élèves), séances du
//!   jour pour le professeur de labo, via `POST /eleves` et `POST /edt/import`.
//!
//! Idempotent : les élèves manquants seulement sont créés.

use chrono::{Datelike, NaiveDate};
use ent_web_api::{ApiClient, ApiError, Connexion, Session};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct Persona {
    pub email: String,
    pub role: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Personnel {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    #[serde(default)]
    pub label: String,
}

/// Élève de la distribution de labo (`POST /labo/bootstrap`).
#[derive(Clone, Debug, Deserialize)]
pub struct EleveLabo {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    #[serde(default)]
    pub classe: Option<String>,
}

/// Réponse de `POST /labo/bootstrap` (champs utiles).
#[derive(Clone, Deserialize)]
pub struct Labo {
    pub etablissement_id: Uuid,
    pub password: String,
    pub personas: Vec<Persona>,
    pub personnel: Vec<Personnel>,
    #[serde(default)]
    pub eleves: Vec<EleveLabo>,
}

// `Debug` manuel : le mot de passe de labo ne doit pas fuiter dans un journal ou une panique.
impl std::fmt::Debug for Labo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Labo")
            .field("etablissement_id", &self.etablissement_id)
            .field("password", &"<masqué>")
            .field("personas", &self.personas)
            .field("personnel", &self.personnel)
            .field("eleves", &self.eleves.len())
            .finish()
    }
}

impl Labo {
    pub fn premier(&self, role: &str) -> Option<&Personnel> {
        self.personnel.iter().find(|p| p.role == role)
    }

    /// Élève de labo par prénom (« Harry », « Draco »…).
    pub fn eleve(&self, prenom: &str) -> Option<&EleveLabo> {
        self.eleves.iter().find(|e| e.prenom == prenom)
    }

    pub fn persona(&self, role: &str) -> Option<&Persona> {
        self.personas.iter().find(|p| p.role == role)
    }
}

/// Vrai seulement si `base` est une URL `http://` dont l'hôte est **exactement**
/// `127.0.0.1`, `localhost` ou `[::1]` (port numérique optionnel, pas d'identifiants).
/// Analyse stricte : `http://127.0.0.1.evil.com`, `http://localhost@evil.com` ou
/// `http://127.0.0.1\@evil.com` sont refusées.
pub fn est_serveur_local(base: &str) -> bool {
    let Some(reste) = base.strip_prefix("http://") else {
        return false;
    };
    let autorite = reste.split(['/', '?', '#']).next().unwrap_or("");
    if autorite.is_empty()
        || autorite.contains(['@', '\\', '%'])
        || autorite
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
    {
        return false;
    }
    let (hote, port) = if let Some(apres) = autorite.strip_prefix('[') {
        match apres.split_once(']') {
            Some((h, p)) => (
                format!("[{h}]"),
                p.strip_prefix(':').or(p.is_empty().then_some("")),
            ),
            None => return false,
        }
    } else {
        match autorite.split_once(':') {
            Some((h, p)) => (h.to_string(), Some(p)),
            None => (autorite.to_string(), Some("")),
        }
    };
    let Some(port) = port else { return false };
    let port_ok = port.is_empty() || (port.len() <= 5 && port.bytes().all(|b| b.is_ascii_digit()));
    port_ok
        && matches!(
            hote.to_ascii_lowercase().as_str(),
            "127.0.0.1" | "localhost" | "[::1]"
        )
}

pub async fn bootstrap(base: &str) -> Result<Labo, ApiError> {
    let v = ApiClient::new(base, None)
        .post_json("/labo/bootstrap", serde_json::json!({}))
        .await?;
    serde_json::from_value(v).map_err(|e| ApiError::Decodage(e.to_string()))
}

pub async fn connecter(base: &str, labo: &Labo, email: &str) -> Result<Session, ApiError> {
    match ApiClient::new(base, None)
        .connexion(email, &labo.password, labo.etablissement_id)
        .await?
    {
        Connexion::Ouverte(s) => Ok(s),
        Connexion::PasskeyRequise(_) => Err(ApiError::Invalide("passkey requise".into())),
    }
}

/// Classe de test et séance du jour associée.
pub struct ClasseTest {
    pub classe: &'static str,
    pub effectif: usize,
    pub heure_debut: &'static str,
    pub heure_fin: &'static str,
}

pub const CLASSES_TEST: [ClasseTest; 2] = [
    ClasseTest {
        classe: "PROTO38",
        effectif: 38,
        heure_debut: "13:00",
        heure_fin: "14:00",
    },
    ClasseTest {
        classe: "PROTO400",
        effectif: 400,
        heure_debut: "14:00",
        heure_fin: "15:00",
    },
];

const PRENOMS: [&str; 20] = [
    "Léa", "Hugo", "Chloé", "Lucas", "Inès", "Nathan", "Zoé", "Théo", "Manon", "Enzo", "Jade",
    "Louis", "Anaïs", "Noé", "Éloïse", "Gabriel", "Maëlle", "Raphaël", "Océane", "Adèle",
];
const NOMS: [&str; 20] = [
    "Martin", "Bernard", "Dubois", "Thomas", "Robert", "Richard", "Petit", "Durand", "Leroy",
    "Moreau", "Simon", "Laurent", "Lefèvre", "Michel", "Garcia", "David", "Bertrand", "Roux",
    "Vincent", "Fournier",
];

/// Nom fictif déterministe (aucune donnée réelle).
pub fn eleve_fictif(i: usize) -> (String, String) {
    let nom = format!("{}-{i:03}", NOMS[i % NOMS.len()].to_uppercase());
    let prenom = PRENOMS[(i * 7 + i / NOMS.len()) % PRENOMS.len()].to_string();
    (nom, prenom)
}

/// Crée les classes de test et leurs séances du jour (`jour`) pour `prof_id`.
pub async fn preparer_classes(
    base: &str,
    admin: &Session,
    prof_id: Uuid,
    jour: NaiveDate,
) -> Result<Vec<String>, ApiError> {
    let api = ApiClient::new(base, Some(admin.token.clone()));
    let entree = NaiveDate::from_ymd_opt(jour.year() - 1, 9, 1).unwrap();
    let mut rapport = Vec::new();
    for c in CLASSES_TEST.iter() {
        let eff = api
            .get_json(&format!("/classes/{}/effectif?jour={jour}", c.classe))
            .await?;
        let deja = eff["eleves"].as_array().map(|a| a.len()).unwrap_or(0);
        for i in deja..c.effectif {
            let (nom, prenom) = eleve_fictif(i);
            api.post_json(
                "/eleves",
                serde_json::json!({
                    "nom": nom, "prenom": prenom, "classe": c.classe, "date_entree": entree,
                }),
            )
            .await?;
        }
        rapport.push(format!(
            "{} : {} élèves ({} créés)",
            c.classe,
            c.effectif.max(deja),
            c.effectif.saturating_sub(deja)
        ));
    }

    // Séances : créées seulement si absentes pour ce jour.
    let existants = api
        .get_json(&format!("/edt/creneaux?jour={jour}"))
        .await
        .unwrap_or_default();
    let deja: Vec<String> = existants
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|c| c["classe"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let jour_iso = jour.weekday().number_from_monday();
    let mut csv = String::from(
        "classe;matiere;prof_user_id;jour_semaine;heure_debut;heure_fin;salle;semaine_ab\n",
    );
    let mut a_creer = 0;
    for c in CLASSES_TEST
        .iter()
        .filter(|c| !deja.iter().any(|d| d == c.classe))
    {
        csv.push_str(&format!(
            "{};Sciences;{prof_id};{jour_iso};{};{};Labo 1;toutes\n",
            c.classe, c.heure_debut, c.heure_fin
        ));
        a_creer += 1;
    }
    if a_creer > 0 {
        let r = api
            .post_json(
                "/edt/import",
                serde_json::json!({ "csv": csv, "dry_run": false }),
            )
            .await?;
        rapport.push(format!("séances importées : {}", r["ok_count"]));
    } else {
        rapport.push("séances déjà présentes".into());
    }
    Ok(rapport)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_masque_le_mot_de_passe() {
        let l = Labo {
            etablissement_id: Uuid::nil(),
            password: "MOT-DE-PASSE-SECRET".into(),
            personas: vec![],
            personnel: vec![],
            eleves: vec![],
        };
        let d = format!("{l:?}");
        assert!(!d.contains("MOT-DE-PASSE-SECRET"), "{d}");
        assert!(d.contains("masqué"));
    }

    #[test]
    fn seuls_les_hotes_locaux_exacts_sont_acceptes() {
        for ok in [
            "http://127.0.0.1",
            "http://127.0.0.1:8090",
            "http://127.0.0.1:8090/",
            "http://localhost:8090",
            "http://LOCALHOST",
            "http://[::1]:8090",
            "http://[::1]",
        ] {
            assert!(est_serveur_local(ok), "{ok}");
        }
        for ko in [
            "http://127.0.0.1.evil.com",
            "http://127.0.0.1.evil.com:8090",
            "http://localhost.evil.com",
            "http://localhostx",
            "http://localhost@evil.com",
            "http://127.0.0.1@evil.com",
            "http://127.0.0.1\\@evil.com",
            "http://evil.com/127.0.0.1",
            "http://evil.com?h=127.0.0.1",
            "http://127.0.0.1:80a",
            "http://127.0.0.1:123456",
            "http://[::1]x",
            "http://[::2]",
            "https://127.0.0.1",
            "ftp://127.0.0.1",
            "127.0.0.1",
            "http://",
            "",
        ] {
            assert!(!est_serveur_local(ko), "{ko}");
        }
    }
}
