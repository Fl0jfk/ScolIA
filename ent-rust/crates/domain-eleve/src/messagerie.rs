//! Messagerie établissement ↔ famille — fil lié à un élève + messages.
//!
//! Accusés de lecture à l'ouverture du fil (pas ses propres messages).
//! Pièces jointes P0 : contenu borné en base (pas de bucket objet).
//! Modèles staff + archive + priorité / dossier boîte. Authz : parent lié ou staff.

use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Taille max d'une PJ messagerie (P0 local).
pub const MSG_PIECE_MAX_OCTETS: usize = 256 * 1024;

fn mime_piece_autorise(mime: &str) -> bool {
    matches!(
        mime,
        "application/pdf"
            | "image/jpeg"
            | "image/png"
            | "text/plain"
            | "application/octet-stream"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MsgPriorite {
    Normale,
    Haute,
}

impl MsgPriorite {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normale => "normale",
            Self::Haute => "haute",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normale" | "normal" => Some(Self::Normale),
            "haute" | "urgent" | "haute_priorite" => Some(Self::Haute),
            _ => None,
        }
    }

    /// Pour tri SQL / mémoire : haute avant normale.
    pub fn rang(self) -> i16 {
        match self {
            Self::Haute => 0,
            Self::Normale => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MsgFil {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub sujet: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_message_at: DateTime<Utc>,
    pub archive: bool,
    pub modele_id: Option<Uuid>,
    pub priorite: MsgPriorite,
    /// Libellé dossier staff (ex. `vs`, `scolarite`) — `None` = boîte générale.
    pub dossier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MsgModele {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub titre: String,
    pub sujet: String,
    pub corps: String,
    pub actif: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreerMsgModele {
    pub etablissement_id: Uuid,
    pub titre: String,
    pub sujet: String,
    pub corps: String,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ContexteModele {
    pub prenom: String,
    pub nom: String,
    pub classe: String,
}

#[derive(Debug, Clone)]
pub struct ArchiverFil {
    pub etablissement_id: Uuid,
    pub fil_id: Uuid,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ClasserFil {
    pub etablissement_id: Uuid,
    pub fil_id: Uuid,
    pub priorite: MsgPriorite,
    pub dossier: Option<String>,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub fil_id: Uuid,
    pub auteur_user_id: Uuid,
    pub corps: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MsgAccuse {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub fil_id: Uuid,
    pub message_id: Uuid,
    pub lecteur_user_id: Uuid,
    pub lu_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MsgPiece {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub fil_id: Uuid,
    pub message_id: Uuid,
    pub nom: String,
    pub mime_type: String,
    pub taille_octets: i32,
    /// Contenu binaire (attach / download uniquement — absente des listes).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub contenu: Vec<u8>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Métadonnées PJ sans bytes (listes / UI).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MsgPieceMeta {
    pub id: Uuid,
    pub message_id: Uuid,
    pub nom: String,
    pub mime_type: String,
    pub taille_octets: i32,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<&MsgPiece> for MsgPieceMeta {
    fn from(p: &MsgPiece) -> Self {
        Self {
            id: p.id,
            message_id: p.message_id,
            nom: p.nom.clone(),
            mime_type: p.mime_type.clone(),
            taille_octets: p.taille_octets,
            created_by_user_id: p.created_by_user_id,
            created_at: p.created_at,
        }
    }
}

/// Message enrichi pour l'UI (accusés + pièces).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageLue {
    pub message: Message,
    pub lu_par_moi: bool,
    pub nb_accuses: i64,
    pub pieces: Vec<MsgPieceMeta>,
}

#[derive(Debug, Clone)]
pub struct OuvrirFil {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub sujet: String,
    pub premier_message: String,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RepondreFil {
    pub etablissement_id: Uuid,
    pub fil_id: Uuid,
    pub corps: String,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct AccuserLecture {
    pub etablissement_id: Uuid,
    pub fil_id: Uuid,
    pub actor_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct AttacherPiece {
    pub etablissement_id: Uuid,
    pub message_id: Uuid,
    pub nom: String,
    pub mime_type: String,
    pub contenu: Vec<u8>,
    pub actor_user_id: Uuid,
}

pub fn plan_ouvrir_fil(cmd: OuvrirFil) -> Result<(MsgFil, Message), DomainError> {
    if cmd.etablissement_id.is_nil() || cmd.eleve_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let sujet = cmd.sujet.trim().to_string();
    let corps = cmd.premier_message.trim().to_string();
    if sujet.is_empty() || corps.is_empty() {
        return Err(DomainError::MessageVide);
    }
    let now = Utc::now();
    let fil_id = Uuid::new_v4();
    let fil = MsgFil {
        id: fil_id,
        etablissement_id: cmd.etablissement_id,
        eleve_id: cmd.eleve_id,
        sujet,
        created_by_user_id: cmd.actor_user_id,
        created_at: now,
        last_message_at: now,
        archive: false,
        modele_id: None,
        priorite: MsgPriorite::Normale,
        dossier: None,
    };
    let msg = Message {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        fil_id,
        auteur_user_id: cmd.actor_user_id,
        corps,
        created_at: now,
    };
    Ok((fil, msg))
}

pub fn plan_repondre_fil(fil: &MsgFil, cmd: RepondreFil) -> Result<(Message, DateTime<Utc>), DomainError> {
    if fil.etablissement_id != cmd.etablissement_id || fil.id != cmd.fil_id {
        return Err(DomainError::TenantMismatch);
    }
    let corps = cmd.corps.trim().to_string();
    if corps.is_empty() {
        return Err(DomainError::MessageVide);
    }
    let now = Utc::now();
    let msg = Message {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        fil_id: fil.id,
        auteur_user_id: cmd.actor_user_id,
        corps,
        created_at: now,
    };
    Ok((msg, now))
}

/// Accusés pour les messages du fil dont l'acteur n'est pas l'auteur.
/// Idempotent côté domaine (nouveaux IDs) — la persistance déduplique sur (message, lecteur).
pub fn plan_accuser_lecture(
    fil: &MsgFil,
    messages: &[Message],
    cmd: &AccuserLecture,
) -> Result<Vec<MsgAccuse>, DomainError> {
    if fil.etablissement_id != cmd.etablissement_id || fil.id != cmd.fil_id {
        return Err(DomainError::TenantMismatch);
    }
    let now = Utc::now();
    let mut out = Vec::new();
    for m in messages {
        if m.etablissement_id != cmd.etablissement_id || m.fil_id != cmd.fil_id {
            return Err(DomainError::TenantMismatch);
        }
        if m.auteur_user_id == cmd.actor_user_id {
            continue;
        }
        out.push(MsgAccuse {
            id: Uuid::new_v4(),
            etablissement_id: cmd.etablissement_id,
            fil_id: cmd.fil_id,
            message_id: m.id,
            lecteur_user_id: cmd.actor_user_id,
            lu_at: now,
        });
    }
    Ok(out)
}

/// Attache une PJ à un message existant (auteur du message uniquement).
pub fn plan_attacher_piece(msg: &Message, cmd: AttacherPiece) -> Result<MsgPiece, DomainError> {
    if msg.etablissement_id != cmd.etablissement_id || msg.id != cmd.message_id {
        return Err(DomainError::TenantMismatch);
    }
    if msg.auteur_user_id != cmd.actor_user_id {
        return Err(DomainError::MsgPiecePasAuteur);
    }
    let nom = cmd.nom.trim().to_string();
    if nom.is_empty() || nom.len() > 200 {
        return Err(DomainError::MsgPieceInvalide);
    }
    let mime = cmd.mime_type.trim().to_ascii_lowercase();
    if !mime_piece_autorise(&mime) {
        return Err(DomainError::MsgPieceInvalide);
    }
    if cmd.contenu.is_empty() || cmd.contenu.len() > MSG_PIECE_MAX_OCTETS {
        return Err(DomainError::MsgPieceInvalide);
    }
    Ok(MsgPiece {
        id: Uuid::new_v4(),
        etablissement_id: msg.etablissement_id,
        fil_id: msg.fil_id,
        message_id: msg.id,
        nom,
        mime_type: mime,
        taille_octets: cmd.contenu.len() as i32,
        contenu: cmd.contenu,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

pub fn plan_creer_msg_modele(cmd: CreerMsgModele) -> Result<MsgModele, DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let titre = cmd.titre.trim().to_string();
    let sujet = cmd.sujet.trim().to_string();
    let corps = cmd.corps.trim().to_string();
    if titre.is_empty() || titre.len() > 120 {
        return Err(DomainError::MsgModeleInvalide);
    }
    if sujet.is_empty() || sujet.len() > 200 || corps.is_empty() || corps.len() > 8000 {
        return Err(DomainError::MsgModeleInvalide);
    }
    Ok(MsgModele {
        id: Uuid::new_v4(),
        etablissement_id: cmd.etablissement_id,
        titre,
        sujet,
        corps,
        actif: true,
        created_by_user_id: cmd.actor_user_id,
        created_at: Utc::now(),
    })
}

/// Remplace `{prenom}`, `{nom}`, `{classe}` dans sujet / corps.
pub fn appliquer_modele(modele: &MsgModele, ctx: &ContexteModele) -> (String, String) {
    let sub = |s: &str| {
        s.replace("{prenom}", &ctx.prenom)
            .replace("{nom}", &ctx.nom)
            .replace("{classe}", &ctx.classe)
    };
    (sub(&modele.sujet), sub(&modele.corps))
}

pub fn plan_archiver_fil(fil: &MsgFil, cmd: &ArchiverFil) -> Result<MsgFil, DomainError> {
    if fil.etablissement_id != cmd.etablissement_id || fil.id != cmd.fil_id {
        return Err(DomainError::TenantMismatch);
    }
    if fil.archive {
        return Ok(fil.clone());
    }
    Ok(MsgFil {
        archive: true,
        ..fil.clone()
    })
}

pub fn plan_classer_fil(fil: &MsgFil, cmd: &ClasserFil) -> Result<MsgFil, DomainError> {
    if fil.etablissement_id != cmd.etablissement_id || fil.id != cmd.fil_id {
        return Err(DomainError::TenantMismatch);
    }
    let dossier = match cmd.dossier.as_ref() {
        None => None,
        Some(raw) => {
            let d = raw.trim().to_ascii_lowercase();
            if d.is_empty() {
                None
            } else if d.len() > 40 || !d.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                return Err(DomainError::MsgBoiteInvalide);
            } else {
                Some(d)
            }
        }
    };
    Ok(MsgFil {
        priorite: cmd.priorite,
        dossier,
        ..fil.clone()
    })
}

/// Enrichit les messages avec accusés + pièces (métadonnées).
pub fn enrichir_messages_lues(
    messages: &[Message],
    accuses: &[MsgAccuse],
    pieces: &[MsgPieceMeta],
    actor_user_id: Uuid,
) -> Vec<MessageLue> {
    messages
        .iter()
        .map(|m| {
            let sur_msg: Vec<&MsgAccuse> = accuses.iter().filter(|a| a.message_id == m.id).collect();
            let pcs: Vec<MsgPieceMeta> = pieces
                .iter()
                .filter(|p| p.message_id == m.id)
                .cloned()
                .collect();
            MessageLue {
                message: m.clone(),
                lu_par_moi: sur_msg.iter().any(|a| a.lecteur_user_id == actor_user_id),
                nb_accuses: sur_msg.len() as i64,
                pieces: pcs,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ouvrir_refuse_vide() {
        let err = plan_ouvrir_fil(OuvrirFil {
            etablissement_id: Uuid::new_v4(),
            eleve_id: Uuid::new_v4(),
            sujet: "  ".into(),
            premier_message: "bonjour".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap_err();
        assert_eq!(err, DomainError::MessageVide);
    }

    #[test]
    fn repondre_ok() {
        let etab = Uuid::new_v4();
        let (fil, _) = plan_ouvrir_fil(OuvrirFil {
            etablissement_id: etab,
            eleve_id: Uuid::new_v4(),
            sujet: "Absence".into(),
            premier_message: "Bonjour".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let (msg, _) = plan_repondre_fil(
            &fil,
            RepondreFil {
                etablissement_id: etab,
                fil_id: fil.id,
                corps: "Merci".into(),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(msg.corps, "Merci");
        assert_eq!(msg.fil_id, fil.id);
    }

    #[test]
    fn accuser_ignore_propre_message() {
        let etab = Uuid::new_v4();
        let auteur = Uuid::new_v4();
        let lecteur = Uuid::new_v4();
        let (fil, premier) = plan_ouvrir_fil(OuvrirFil {
            etablissement_id: etab,
            eleve_id: Uuid::new_v4(),
            sujet: "Info".into(),
            premier_message: "Hello".into(),
            actor_user_id: auteur,
        })
        .unwrap();
        let (reponse, _) = plan_repondre_fil(
            &fil,
            RepondreFil {
                etablissement_id: etab,
                fil_id: fil.id,
                corps: "OK".into(),
                actor_user_id: lecteur,
            },
        )
        .unwrap();
        let msgs = vec![premier.clone(), reponse.clone()];
        let accuses = plan_accuser_lecture(
            &fil,
            &msgs,
            &AccuserLecture {
                etablissement_id: etab,
                fil_id: fil.id,
                actor_user_id: lecteur,
            },
        )
        .unwrap();
        assert_eq!(accuses.len(), 1);
        assert_eq!(accuses[0].message_id, premier.id);
        assert_eq!(accuses[0].lecteur_user_id, lecteur);

        let lues = enrichir_messages_lues(&msgs, &accuses, &[], lecteur);
        assert!(lues[0].lu_par_moi);
        assert_eq!(lues[0].nb_accuses, 1);
        assert!(!lues[1].lu_par_moi);
        assert_eq!(lues[1].nb_accuses, 0);
    }

    #[test]
    fn piece_refuse_trop_grosse_et_non_auteur() {
        let etab = Uuid::new_v4();
        let auteur = Uuid::new_v4();
        let (fil, msg) = plan_ouvrir_fil(OuvrirFil {
            etablissement_id: etab,
            eleve_id: Uuid::new_v4(),
            sujet: "Justif".into(),
            premier_message: "Voici mon justificatif".into(),
            actor_user_id: auteur,
        })
        .unwrap();
        let _ = fil;
        let err = plan_attacher_piece(
            &msg,
            AttacherPiece {
                etablissement_id: etab,
                message_id: msg.id,
                nom: "big.bin".into(),
                mime_type: "application/octet-stream".into(),
                contenu: vec![0u8; MSG_PIECE_MAX_OCTETS + 1],
                actor_user_id: auteur,
            },
        )
        .unwrap_err();
        assert_eq!(err, DomainError::MsgPieceInvalide);

        let err2 = plan_attacher_piece(
            &msg,
            AttacherPiece {
                etablissement_id: etab,
                message_id: msg.id,
                nom: "ok.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: b"%PDF-1.4 lab".to_vec(),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(err2, DomainError::MsgPiecePasAuteur);

        let ok = plan_attacher_piece(
            &msg,
            AttacherPiece {
                etablissement_id: etab,
                message_id: msg.id,
                nom: "justif.pdf".into(),
                mime_type: "application/pdf".into(),
                contenu: b"%PDF-1.4 lab".to_vec(),
                actor_user_id: auteur,
            },
        )
        .unwrap();
        assert_eq!(ok.nom, "justif.pdf");
        assert_eq!(ok.taille_octets, 12);
    }

    #[test]
    fn modele_placeholders() {
        let m = plan_creer_msg_modele(CreerMsgModele {
            etablissement_id: Uuid::new_v4(),
            titre: "Convocation".into(),
            sujet: "RDV — {prenom} {nom}".into(),
            corps: "Bonjour, concernant {prenom} ({classe}).".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let (s, c) = appliquer_modele(
            &m,
            &ContexteModele {
                prenom: "Harry".into(),
                nom: "Potter".into(),
                classe: "4B".into(),
            },
        );
        assert_eq!(s, "RDV — Harry Potter");
        assert!(c.contains("Harry") && c.contains("4B"));
    }

    #[test]
    fn classer_fil_priorite_dossier() {
        let etab = Uuid::new_v4();
        let fil_id = Uuid::new_v4();
        let (fil, _) = plan_ouvrir_fil(OuvrirFil {
            etablissement_id: etab,
            eleve_id: Uuid::new_v4(),
            sujet: "Test".into(),
            premier_message: "Bonjour".into(),
            actor_user_id: Uuid::new_v4(),
        })
        .unwrap();
        let fil = MsgFil {
            id: fil_id,
            ..fil
        };
        let ok = plan_classer_fil(
            &fil,
            &ClasserFil {
                etablissement_id: etab,
                fil_id,
                priorite: MsgPriorite::Haute,
                dossier: Some(" VS ".into()),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap();
        assert_eq!(ok.priorite, MsgPriorite::Haute);
        assert_eq!(ok.dossier.as_deref(), Some("vs"));
        let bad = plan_classer_fil(
            &fil,
            &ClasserFil {
                etablissement_id: etab,
                fil_id,
                priorite: MsgPriorite::Normale,
                dossier: Some("mauvais dossier!".into()),
                actor_user_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();
        assert_eq!(bad, DomainError::MsgBoiteInvalide);
    }
}
