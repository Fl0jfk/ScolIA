//! Service communication (lecture seule) : fils de messages et annonces.
//!
//! Types de lecture : seulement les champs utilisés par l'accueil (les autres
//! champs du JSON d'`ent-api` sont ignorés). À déplacer dans `ent-types` lors du
//! lot qui migrera ces DTO côté `ent-api` (décision séparée, D-016).

use serde::Deserialize;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

/// Résumé d'un fil de messages (`MsgFilDto` d'ent-api, champs utiles).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FilMessages {
    pub id: Uuid,
    #[serde(default)]
    pub eleve_label: Option<String>,
    pub sujet: String,
    pub last_message_at: String,
    #[serde(default)]
    pub archive: bool,
    #[serde(default)]
    pub nb_non_lus: i64,
}

/// Annonce d'établissement (`AnnonceDto` d'ent-api, champs utiles).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Annonce {
    pub id: Uuid,
    pub titre: String,
    pub corps: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ApiClient {
    /// `GET /messages/fils` : fils visibles par la personne connectée (filtrés par le serveur).
    pub async fn fils_messages(&self) -> Result<Vec<FilMessages>, ApiError> {
        self.appel(Methode::Get, "/messages/fils", None).await
    }

    /// `GET /annonces` : annonces de l'établissement visibles selon le rôle.
    pub async fn annonces(&self) -> Result<Vec<Annonce>, ApiError> {
        self.appel(Methode::Get, "/annonces", None).await
    }
}

// ---------------------------------------------------------------------------
// Messagerie (boîte, fils, réponses, pièces jointes, modèles)
// ---------------------------------------------------------------------------

/// Fil de messages tel que la boîte l'affiche (`MsgFilDto` d'ent-api, champs utiles).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FilBoite {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(default)]
    pub eleve_label: Option<String>,
    pub sujet: String,
    #[serde(default)]
    pub created_by_user_id: Option<Uuid>,
    pub created_at: String,
    pub last_message_at: String,
    #[serde(default)]
    pub archive: bool,
    /// `normale` ou `haute`.
    #[serde(default)]
    pub priorite: String,
    #[serde(default)]
    pub dossier: Option<String>,
    #[serde(default)]
    pub nb_non_lus: i64,
}

impl FilBoite {
    pub fn est_prioritaire(&self) -> bool {
        self.priorite == "haute"
    }
}

/// Métadonnées d'une pièce jointe (`MsgPieceDto`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PieceMessage {
    pub id: Uuid,
    pub message_id: Uuid,
    pub nom: String,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub taille_octets: i64,
}

/// Message d'un fil (`MessageDto`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct MessageFil {
    pub id: Uuid,
    pub fil_id: Uuid,
    pub auteur_user_id: Uuid,
    #[serde(default)]
    pub auteur_label: Option<String>,
    pub corps: String,
    pub created_at: String,
    #[serde(default)]
    pub lu_par_moi: bool,
    /// Nombre de personnes ayant accusé réception.
    #[serde(default)]
    pub nb_accuses: i64,
    #[serde(default)]
    pub pieces: Vec<PieceMessage>,
}

/// `GET /messages/fils/{id}` (et réponses de création) : le fil et ses messages.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DetailFil {
    pub fil: FilBoite,
    #[serde(default)]
    pub messages: Vec<MessageFil>,
}

/// Modèle de message du personnel (`MsgModeleDto`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ModeleMessage {
    pub id: Uuid,
    pub titre: String,
    pub sujet: String,
    pub corps: String,
    #[serde(default)]
    pub actif: bool,
}

/// Enfant d'une famille : même type que celui du portail famille (module vie scolaire), pour
/// qu'une seule requête `GET /famille/enfants` serve les deux modules.
pub use ent_types::vie_scolaire::EnfantFamilleDto as EnfantFamille;

/// Élève trouvé par la recherche du personnel (`EleveDto`, champs utiles).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EleveTrouve {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    #[serde(default)]
    pub classe: Option<String>,
}

/// Libellé d'un élève (`GET /eleves/labels`, personnel).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct LibelleEleve {
    pub id: Uuid,
    pub label: String,
}

/// Encodage pourcent d'une valeur de requête (UTF-8, jeu non réservé de la RFC 3986).
pub fn encoder_requete(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for octet in texte.bytes() {
        match octet {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                sortie.push(octet as char)
            }
            _ => sortie.push_str(&format!("%{octet:02X}")),
        }
    }
    sortie
}

/// Base64 standard avec remplissage (contenu des pièces jointes, `contenu_base64`).
pub fn base64_encoder(octets: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut sortie = String::with_capacity(octets.len().div_ceil(3) * 4);
    for bloc in octets.chunks(3) {
        let b = [
            bloc[0],
            bloc.get(1).copied().unwrap_or(0),
            bloc.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        sortie.push(TABLE[(n >> 18) as usize & 63] as char);
        sortie.push(TABLE[(n >> 12) as usize & 63] as char);
        sortie.push(if bloc.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        sortie.push(if bloc.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    sortie
}

impl ApiClient {
    /// `GET /messages/fils` : boîte de la personne connectée. `archives` n'a d'effet que
    /// pour le personnel (le serveur inclut alors les fils archivés).
    pub async fn boite_messages(&self, archives: bool) -> Result<Vec<FilBoite>, ApiError> {
        let chemin = if archives {
            "/messages/fils?archives=true"
        } else {
            "/messages/fils"
        };
        self.appel(Methode::Get, chemin, None).await
    }

    /// `GET /messages/fils/{id}` : messages du fil ; le serveur enregistre les accusés de lecture.
    pub async fn detail_fil(&self, fil_id: Uuid) -> Result<DetailFil, ApiError> {
        self.appel(Methode::Get, &format!("/messages/fils/{fil_id}"), None)
            .await
    }

    /// `POST /messages/fils` : ouvre un fil à propos d'un élève.
    pub async fn ouvrir_fil_messages(
        &self,
        eleve_id: Uuid,
        sujet: &str,
        message: &str,
    ) -> Result<DetailFil, ApiError> {
        let corps = serde_json::json!({ "eleve_id": eleve_id, "sujet": sujet, "message": message });
        self.appel(Methode::Post, "/messages/fils", Some(corps))
            .await
    }

    /// `POST /messages/fils/depuis-modele` : ouvre un fil depuis un modèle (personnel).
    pub async fn ouvrir_fil_depuis_modele(
        &self,
        modele_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<DetailFil, ApiError> {
        let corps = serde_json::json!({ "modele_id": modele_id, "eleve_id": eleve_id });
        self.appel(Methode::Post, "/messages/fils/depuis-modele", Some(corps))
            .await
    }

    /// `POST /messages/fils/{id}/repondre`.
    pub async fn repondre_fil(&self, fil_id: Uuid, corps: &str) -> Result<MessageFil, ApiError> {
        let corps = serde_json::json!({ "corps": corps });
        self.appel(
            Methode::Post,
            &format!("/messages/fils/{fil_id}/repondre"),
            Some(corps),
        )
        .await
    }

    /// `POST /messages/fils/{id}/archiver` (personnel).
    pub async fn archiver_fil(&self, fil_id: Uuid) -> Result<FilBoite, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/messages/fils/{fil_id}/archiver"),
            None,
        )
        .await
    }

    /// `POST /messages/fils/{id}/classer` (personnel) : priorité `normale|haute` et dossier.
    pub async fn classer_fil(
        &self,
        fil_id: Uuid,
        priorite: &str,
        dossier: Option<&str>,
    ) -> Result<FilBoite, ApiError> {
        let corps = serde_json::json!({ "priorite": priorite, "dossier": dossier });
        self.appel(
            Methode::Post,
            &format!("/messages/fils/{fil_id}/classer"),
            Some(corps),
        )
        .await
    }

    /// `GET /messages/modeles` (personnel) : modèles actifs.
    pub async fn modeles_messages(&self) -> Result<Vec<ModeleMessage>, ApiError> {
        self.appel(Methode::Get, "/messages/modeles", None).await
    }

    /// `POST /messages/{id}/pieces` : pièce jointe d'un message dont on est l'auteur.
    pub async fn joindre_piece(
        &self,
        message_id: Uuid,
        nom: &str,
        mime_type: &str,
        octets: &[u8],
    ) -> Result<PieceMessage, ApiError> {
        let corps = serde_json::json!({
            "nom": nom,
            "mime_type": mime_type,
            "contenu_base64": base64_encoder(octets),
        });
        self.appel(
            Methode::Post,
            &format!("/messages/{message_id}/pieces"),
            Some(corps),
        )
        .await
    }

    /// `GET /messages/pieces/{id}` : contenu d'une pièce jointe (droits contrôlés par le serveur).
    pub async fn telecharger_piece(&self, piece_id: Uuid) -> Result<Vec<u8>, ApiError> {
        let (statut, octets) = crate::transport::envoyer_octets(
            &self.url(&format!("/messages/pieces/{piece_id}")),
            self.auth(),
        )
        .await?;
        if (200..300).contains(&statut) {
            Ok(octets)
        } else {
            Err(ApiError::depuis_statut(
                statut,
                String::from_utf8_lossy(&octets).into_owned(),
            ))
        }
    }

    /// `GET /eleves/search?q=` : recherche d'élèves (personnel uniquement, 50 résultats au plus).
    pub async fn messagerie_chercher_eleves(
        &self,
        recherche: &str,
    ) -> Result<Vec<EleveTrouve>, ApiError> {
        let chemin = format!("/eleves/search?q={}", encoder_requete(recherche.trim()));
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `GET /eleves/labels?ids=` : libellés d'élèves (personnel uniquement).
    pub async fn messagerie_libelles_eleves(
        &self,
        ids: &[Uuid],
    ) -> Result<Vec<LibelleEleve>, ApiError> {
        let ids = ids
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join(",");
        self.appel(Methode::Get, &format!("/eleves/labels?ids={ids}"), None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_vecteurs_rfc4648() {
        assert_eq!(base64_encoder(b""), "");
        assert_eq!(base64_encoder(b"f"), "Zg==");
        assert_eq!(base64_encoder(b"fo"), "Zm8=");
        assert_eq!(base64_encoder(b"foo"), "Zm9v");
        assert_eq!(base64_encoder(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encoder(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encoder(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encoder(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn requete_encodee_en_utf8() {
        assert_eq!(encoder_requete("Léa Martin"), "L%C3%A9a%20Martin");
        assert_eq!(encoder_requete("a&b=c#d"), "a%26b%3Dc%23d");
        assert_eq!(encoder_requete("abc-_.~09"), "abc-_.~09");
    }

    #[test]
    fn decode_fil_du_contrat_ent_api() {
        let json = r#"{"fil":{"id":"6f1c3c80-0000-4000-8000-000000000001",
            "eleve_id":"6f1c3c80-0000-4000-8000-000000000002","eleve_label":"POTTER Harry",
            "sujet":"Sortie","created_by_user_id":"6f1c3c80-0000-4000-8000-000000000003",
            "created_at":"2026-11-03T08:00:00+00:00","last_message_at":"2026-11-03T09:00:00+00:00",
            "archive":false,"modele_id":null,"priorite":"haute","dossier":"vs","nb_non_lus":2},
            "messages":[{"id":"6f1c3c80-0000-4000-8000-000000000004",
            "fil_id":"6f1c3c80-0000-4000-8000-000000000001",
            "auteur_user_id":"6f1c3c80-0000-4000-8000-000000000003","auteur_label":"Rogue",
            "corps":"Bonjour","created_at":"2026-11-03T08:00:00+00:00","lu_par_moi":true,
            "nb_accuses":1,"pieces":[{"id":"6f1c3c80-0000-4000-8000-000000000005",
            "message_id":"6f1c3c80-0000-4000-8000-000000000004","nom":"a.pdf",
            "mime_type":"application/pdf","taille_octets":120,"created_at":"x"}]}]}"#;
        let d: DetailFil = serde_json::from_str(json).unwrap();
        assert!(d.fil.est_prioritaire());
        assert_eq!(d.fil.dossier.as_deref(), Some("vs"));
        assert_eq!(d.messages[0].pieces[0].nom, "a.pdf");
        // Réponse de `classer` / `archiver` : libellé et compteur absents.
        let f: FilBoite = serde_json::from_str(
            r#"{"id":"6f1c3c80-0000-4000-8000-000000000001","eleve_id":"6f1c3c80-0000-4000-8000-000000000002",
            "sujet":"s","created_by_user_id":"6f1c3c80-0000-4000-8000-000000000003",
            "created_at":"a","last_message_at":"b","archive":true,"priorite":"normale","dossier":null,"nb_non_lus":0}"#,
        )
        .unwrap();
        assert!(f.archive && f.eleve_label.is_none() && !f.est_prioritaire());
    }
}
