//! PAI (Projet d'Accueil Individualisé) — une fiche structurée par élève.
//!
//! Famille : motif, mesures, protocole, dates, signature, ordonnances jointes.
//! Note interne jamais exposée. Signature parent = geste interactif P0.

use crate::DomainError;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Taille max d'une ordonnance / PJ PAI (256 Kio, comme messagerie P0).
pub const PAI_PIECE_MAX_OCTETS: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pai {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub motif: String,
    pub mesures_accueil: String,
    pub protocole_urgence: String,
    /// Staff only.
    pub note_interne: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    pub actif: bool,
    pub signe_famille_at: Option<DateTime<Utc>>,
    pub signe_famille_user_id: Option<Uuid>,
    pub updated_by_user_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertPai {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub motif: String,
    pub mesures_accueil: String,
    pub protocole_urgence: String,
    pub note_interne: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    pub actif: bool,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct SignerPaiFamille {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub actor_user_id: Uuid,
}

/// Vue famille : note interne vidée.
pub fn vue_famille(p: &Pai) -> Pai {
    Pai {
        note_interne: String::new(),
        ..p.clone()
    }
}

fn trim_field(s: String, max: usize) -> Result<String, DomainError> {
    let t = s.trim().to_string();
    if t.len() > max {
        return Err(DomainError::PaiInvalide);
    }
    Ok(t)
}

pub fn plan_upsert_pai(cmd: UpsertPai) -> Result<Pai, DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let motif = trim_field(cmd.motif, 500)?;
    let mesures = trim_field(cmd.mesures_accueil, 4000)?;
    let protocole = trim_field(cmd.protocole_urgence, 4000)?;
    let note = trim_field(cmd.note_interne, 4000)?;
    if motif.is_empty() || mesures.is_empty() {
        return Err(DomainError::PaiInvalide);
    }
    if let Some(fin) = cmd.date_fin {
        if fin < cmd.date_debut {
            return Err(DomainError::PaiInvalide);
        }
    }
    Ok(Pai {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        motif,
        mesures_accueil: mesures,
        protocole_urgence: protocole,
        note_interne: note,
        date_debut: cmd.date_debut,
        date_fin: cmd.date_fin,
        actif: cmd.actif,
        // Nouvelle version / maj staff → signature à refaire.
        signe_famille_at: None,
        signe_famille_user_id: None,
        updated_by_user_id: cmd.actor_user_id,
        updated_at: Utc::now(),
    })
}

pub fn plan_signer_pai_famille(current: &Pai, cmd: &SignerPaiFamille) -> Result<Pai, DomainError> {
    if current.etablissement_id != cmd.etablissement_id || current.eleve_id != cmd.eleve_id {
        return Err(DomainError::NotFound);
    }
    if !current.actif {
        return Err(DomainError::PaiInactif);
    }
    if current.signe_famille_at.is_some() {
        return Err(DomainError::PaiDejaSigne);
    }
    Ok(Pai {
        signe_famille_at: Some(Utc::now()),
        signe_famille_user_id: Some(cmd.actor_user_id),
        ..current.clone()
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaiPiece {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub pai_id: Uuid,
    pub nom: String,
    pub mime_type: String,
    pub taille_octets: i32,
    /// Contenu inline (legacy) — vide si `storage_key` renseigné.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub contenu: Vec<u8>,
    /// Clé objet (filesystem / S3) — prioritaire pour le téléchargement.
    pub storage_key: Option<String>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaiPieceMeta {
    pub id: Uuid,
    pub pai_id: Uuid,
    pub nom: String,
    pub mime_type: String,
    pub taille_octets: i32,
    pub created_at: DateTime<Utc>,
}

impl From<&PaiPiece> for PaiPieceMeta {
    fn from(p: &PaiPiece) -> Self {
        Self {
            id: p.id,
            pai_id: p.pai_id,
            nom: p.nom.clone(),
            mime_type: p.mime_type.clone(),
            taille_octets: p.taille_octets,
            created_at: p.created_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AttacherOrdonnancePai {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub mime_type: String,
    pub contenu: Vec<u8>,
    pub actor_user_id: Uuid,
}

fn mime_ordonnance_ok(mime: &str) -> bool {
    matches!(
        mime,
        "application/pdf" | "image/jpeg" | "image/png" | "image/jpg"
    )
}

pub fn plan_attacher_ordonnance_pai(
    pai: &Pai,
    cmd: AttacherOrdonnancePai,
) -> Result<PaiPiece, DomainError> {
    if pai.etablissement_id != cmd.etablissement_id || pai.eleve_id != cmd.eleve_id {
        return Err(DomainError::NotFound);
    }
    if !pai.actif {
        return Err(DomainError::PaiInactif);
    }
    let nom = cmd.nom.trim().to_string();
    if nom.is_empty() || nom.len() > 200 {
        return Err(DomainError::PaiPieceInvalide);
    }
    let mime = cmd.mime_type.trim().to_lowercase();
    if !mime_ordonnance_ok(&mime) {
        return Err(DomainError::PaiPieceInvalide);
    }
    if cmd.contenu.is_empty() || cmd.contenu.len() > PAI_PIECE_MAX_OCTETS {
        return Err(DomainError::PaiPieceInvalide);
    }
    let taille = i32::try_from(cmd.contenu.len()).map_err(|_| DomainError::PaiPieceInvalide)?;
    Ok(PaiPiece {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        pai_id: pai.id,
        nom,
        mime_type: mime,
        taille_octets: taille,
        contenu: cmd.contenu,
        storage_key: None,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_sans_motif_mesures() {
        let err = plan_upsert_pai(UpsertPai {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            motif: "  ".into(),
            mesures_accueil: "inhalateur".into(),
            protocole_urgence: String::new(),
            note_interne: String::new(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: None,
            actif: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::PaiInvalide);
    }

    #[test]
    fn signature_et_vue_famille() {
        let p = plan_upsert_pai(UpsertPai {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            motif: "asthme".into(),
            mesures_accueil: "Ventoline à l'infirmerie".into(),
            protocole_urgence: "Appeler SAMU si crise".into(),
            note_interne: "antécédent sévère".into(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: Some(NaiveDate::from_ymd_opt(2027, 7, 5).unwrap()),
            actif: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let v = vue_famille(&p);
        assert!(v.note_interne.is_empty());
        assert_eq!(v.motif, "asthme");
        let parent = Uuid::new_v4();
        let signe = plan_signer_pai_famille(
            &p,
            &SignerPaiFamille {
                etablissement_id: p.etablissement_id,
                eleve_id: p.eleve_id,
                actor_user_id: parent,
            },
        )
        .unwrap();
        assert!(signe.signe_famille_at.is_some());
        assert_eq!(signe.signe_famille_user_id, Some(parent));
        let again = plan_signer_pai_famille(
            &signe,
            &SignerPaiFamille {
                etablissement_id: p.etablissement_id,
                eleve_id: p.eleve_id,
                actor_user_id: parent,
            },
        )
        .unwrap_err();
        assert_eq!(again, DomainError::PaiDejaSigne);
    }

    #[test]
    fn refuse_signer_inactif() {
        let mut p = plan_upsert_pai(UpsertPai {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            motif: "allergie".into(),
            mesures_accueil: "éviction".into(),
            protocole_urgence: String::new(),
            note_interne: String::new(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: None,
            actif: false,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        p.actif = false;
        let err = plan_signer_pai_famille(
            &p,
            &SignerPaiFamille {
                etablissement_id: p.etablissement_id,
                eleve_id: p.eleve_id,
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::PaiInactif);
    }

    #[test]
    fn ordonnance_pdf_ok() {
        let p = plan_upsert_pai(UpsertPai {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            motif: "asthme".into(),
            mesures_accueil: "Ventoline".into(),
            protocole_urgence: String::new(),
            note_interne: String::new(),
            date_debut: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            date_fin: None,
            actif: true,
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let piece = plan_attacher_ordonnance_pai(
            &p,
            AttacherOrdonnancePai {
                etablissement_id: p.etablissement_id,
                eleve_id: p.eleve_id,
                nom: "ordo-asthme.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: b"%PDF-1.4 fake".to_vec(),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(piece.nom, "ordo-asthme.pdf");
        assert_eq!(piece.pai_id, p.id);
        let bad = plan_attacher_ordonnance_pai(
            &p,
            AttacherOrdonnancePai {
                etablissement_id: p.etablissement_id,
                eleve_id: p.eleve_id,
                nom: "x.exe".into(),
                mime_type: "application/octet-stream".into(),
                contenu: vec![1, 2, 3],
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(bad, DomainError::PaiPieceInvalide);
    }
}
