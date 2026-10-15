//! Trombinoscope / photo élève (P2).

use crate::DomainError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrombiEntree {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub photo_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PoserPhotoEleve {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub content_type: String,
    pub bytes_len: usize,
    pub actor_user_id: Uuid,
}

/// Valide le geste « poser photo » — le stockage objet est hors domaine.
pub fn plan_poser_photo_eleve(cmd: PoserPhotoEleve) -> Result<String, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    if cmd.bytes_len == 0 || cmd.bytes_len > 5_000_000 {
        return Err(DomainError::TrombiInvalide);
    }
    let ct = cmd.content_type.trim().to_ascii_lowercase();
    let ext = match ct.as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => return Err(DomainError::TrombiInvalide),
    };
    Ok(ext.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_ok() {
        let ext = plan_poser_photo_eleve(PoserPhotoEleve {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            content_type: "image/jpeg".into(),
            bytes_len: 1024,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        assert_eq!(ext, "jpg");
    }
}
