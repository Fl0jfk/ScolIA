//! Modèle **pur** de la page « Consultations du dossier » (socle 27, lot G4-1) : aucun appel réseau, aucun droit
//! calculé. Le serveur ne donne jamais le nom de l'agent (§11.3) : l'écran n'affiche que ce qu'il reçoit.

use ent_web_api::acces_traces_types::{DossierConsulteDto, LigneConsultationDto};

/// Titre d'un dossier : « Dossier de Léa » pour un élève, « Mon dossier d'agent » pour celui de l'agent.
pub fn titre_dossier(d: &DossierConsulteDto) -> String {
    match (d.sujet_type.as_str(), d.libelle.trim()) {
        ("agent", _) => "Mon dossier d'agent".to_string(),
        (_, "") => "Dossier de l'élève".to_string(),
        (_, p) => format!("Dossier de {p}"),
    }
}

/// Ligne principale d'une consultation : « Enseignant · Enseignement ». Pour le support de l'éditeur, la seule
/// mention reçue (« Support ScolIA, intervention autorisée le … »).
pub fn qui(l: &LigneConsultationDto) -> String {
    if let Some(m) = &l.mention {
        return m.clone();
    }
    match (l.fonction.trim(), l.service.trim()) {
        ("", "") => "Personnel de l'établissement".to_string(),
        (f, "") => f.to_string(),
        ("", s) => s.to_string(),
        (f, s) => format!("{f} · {s}"),
    }
}

/// Détail : « Consultation · Scolarité » + le motif quand la consultation était hors périmètre.
pub fn quoi(l: &LigneConsultationDto) -> String {
    match &l.motif {
        Some(m) => format!("{} · {} — motif : {}", l.nature, l.section, m),
        None => format!("{} · {}", l.nature, l.section),
    }
}

/// Phrase de synthèse d'un dossier.
pub fn synthese(total: i64) -> String {
    match total {
        0 => "Aucune consultation sur la période.".to_string(),
        1 => "1 consultation sur la période.".to_string(),
        n => format!("{n} consultations sur la période."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn ligne(
        fonction: &str,
        service: &str,
        mention: Option<&str>,
        motif: Option<&str>,
    ) -> LigneConsultationDto {
        LigneConsultationDto {
            le: Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap(),
            fonction: fonction.into(),
            service: service.into(),
            nature: "Consultation".into(),
            section: "Scolarité".into(),
            motif: motif.map(Into::into),
            mention: mention.map(Into::into),
        }
    }

    fn dossier(type_: &str, libelle: &str) -> DossierConsulteDto {
        DossierConsulteDto {
            sujet_type: type_.into(),
            sujet_id: Uuid::nil(),
            libelle: libelle.into(),
            total: 0,
            jours: vec![],
        }
    }

    #[test]
    fn titre_selon_le_dossier() {
        assert_eq!(titre_dossier(&dossier("eleve", "Léa")), "Dossier de Léa");
        assert_eq!(titre_dossier(&dossier("eleve", "")), "Dossier de l'élève");
        assert_eq!(titre_dossier(&dossier("agent", "")), "Mon dossier d'agent");
    }

    #[test]
    fn fonction_et_service_jamais_un_nom() {
        assert_eq!(
            qui(&ligne("Enseignant", "Enseignement", None, None)),
            "Enseignant · Enseignement"
        );
        assert_eq!(qui(&ligne("Direction", "", None, None)), "Direction");
        assert_eq!(
            qui(&ligne("", "", None, None)),
            "Personnel de l'établissement"
        );
    }

    #[test]
    fn le_support_de_l_editeur_n_affiche_que_sa_mention() {
        let l = ligne(
            "Support ScolIA",
            "",
            Some("Support ScolIA, intervention autorisée le 12/11/2026"),
            None,
        );
        assert_eq!(
            qui(&l),
            "Support ScolIA, intervention autorisée le 12/11/2026"
        );
    }

    #[test]
    fn le_motif_apparait_pour_une_consultation_hors_perimetre() {
        assert_eq!(
            quoi(&ligne("a", "b", None, None)),
            "Consultation · Scolarité"
        );
        assert_eq!(
            quoi(&ligne("a", "b", None, Some("Urgence"))),
            "Consultation · Scolarité — motif : Urgence"
        );
        assert_eq!(synthese(0), "Aucune consultation sur la période.");
        assert_eq!(synthese(2), "2 consultations sur la période.");
    }
}
