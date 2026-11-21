//! Logique d'écran « Absences » de la famille et de l'élève (pure, sans Leptos ni réseau,
//! testée en natif) : accès par enfant, absences justifiables, composition du motif.
//!
//! Les droits réels (lien parent, drapeaux `peut_voir_vs` / `peut_justifier_absence`)
//! sont vérifiés par le serveur à chaque requête ; ici on décide seulement quoi proposer.

use ent_web_api::{Absence, Enfant, MotifAbsence};
use uuid::Uuid;

use crate::composants_metier::absences::StatutAbsence;

/// Longueur maximale du motif envoyé (caractères).
pub const MOTIF_MAX: usize = 500;

/// Code du motif libre « Autre » (ajouté par l'écran, absent de la liste du serveur).
pub const CODE_AUTRE: &str = "autre";

/// Motif système des absences relevées à l'appel (« Absent à l'appel ») : ce n'est pas une
/// justification, il n'est donc pas proposé à la famille.
pub const CODE_APPEL: &str = "appel";

/// Ce que le lien famille permet de voir pour un enfant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acces {
    /// `peut_voir_vs` : historique complet des absences et retards.
    Complet,
    /// Justification possible sans lecture de l'historique : seules les absences à
    /// justifier (résumé famille) sont visibles.
    JustificationSeule,
    /// Ni lecture ni justification.
    Aucun,
}

pub fn acces(e: &Enfant) -> Acces {
    if e.peut_voir_vs {
        Acces::Complet
    } else if e.peut_justifier_absence {
        Acces::JustificationSeule
    } else {
        Acces::Aucun
    }
}

/// « Harry Potter » (prénom puis nom).
pub fn nom_enfant(e: &Enfant) -> String {
    format!("{} {}", e.prenom.trim(), e.nom.trim())
        .trim()
        .to_string()
}

/// Enfant à afficher : celui demandé s'il existe, sinon le premier.
pub fn enfant_choisi(enfants: &[Enfant], demande: Option<Uuid>) -> Option<Uuid> {
    demande
        .filter(|id| enfants.iter().any(|e| e.id == *id))
        .or_else(|| enfants.first().map(|e| e.id))
}

/// Peut-on proposer « Justifier » ? Seulement si le lien l'autorise et si l'absence n'a
/// ni justification déposée ni décision (le serveur refuse sinon).
pub fn peut_justifier(e: &Enfant, a: &Absence) -> bool {
    e.peut_justifier_absence && StatutAbsence::depuis(a) == StatutAbsence::NonJustifiee
}

/// Peut-on proposer « Retirer ma justification » (justificatif en attente de décision) ?
pub fn peut_retirer(e: &Enfant, a: &Absence) -> bool {
    e.peut_justifier_absence && StatutAbsence::depuis(a) == StatutAbsence::JustificatifEnAttente
}

/// Absences encore à justifier (sans justification déposée).
pub fn a_justifier(absences: &[Absence]) -> usize {
    absences
        .iter()
        .filter(|a| StatutAbsence::depuis(a) == StatutAbsence::NonJustifiee)
        .count()
}

/// Les plus récentes d'abord.
pub fn recentes_d_abord(mut absences: Vec<Absence>) -> Vec<Absence> {
    absences.sort_by(|a, b| b.debut.cmp(&a.debut).then(b.fin.cmp(&a.fin)));
    absences
}

/// Message quand le lien ne permet pas de justifier.
pub fn message_sans_droit_de_justifier(prenom: &str) -> String {
    format!(
        "Votre lien avec {prenom} ne vous permet pas de justifier ses absences. Adressez-vous à la vie scolaire."
    )
}

/// Options du choix de motif : motifs du serveur puis « Autre ». Un code en double avec
/// « autre » est écarté (le choix libre est toujours le dernier).
pub fn options_motifs(motifs: &[MotifAbsence]) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = motifs
        .iter()
        .filter(|m| m.code != CODE_AUTRE && m.code != CODE_APPEL && !m.libelle.trim().is_empty())
        .map(|m| (m.code.clone(), m.libelle.trim().to_string()))
        .collect();
    v.push((CODE_AUTRE.to_string(), "Autre motif".to_string()));
    v
}

/// Texte envoyé au serveur (`motif_justif`) : « Maladie — fièvre depuis hier ».
/// « Autre » exige une précision ; la longueur est bornée.
pub fn composer_justification(
    motifs: &[MotifAbsence],
    code: &str,
    commentaire: &str,
) -> Result<String, String> {
    let commentaire = commentaire.trim();
    let libelle = if code == CODE_AUTRE {
        None
    } else {
        Some(
            motifs
                .iter()
                .find(|m| m.code == code && m.code != CODE_APPEL)
                .map(|m| m.libelle.trim().to_string())
                .ok_or_else(|| "Choisissez un motif dans la liste.".to_string())?,
        )
    };
    let texte = match (libelle, commentaire.is_empty()) {
        (None, true) => return Err("Précisez le motif de l'absence.".into()),
        (None, false) => commentaire.to_string(),
        (Some(l), true) => l,
        (Some(l), false) => format!("{l} — {commentaire}"),
    };
    if texte.chars().count() > MOTIF_MAX {
        return Err(format!(
            "Le texte est trop long ({MOTIF_MAX} caractères au plus)."
        ));
    }
    Ok(texte)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composants_metier::absences::essais::absence;

    fn enfant(id: u128, voir_vs: bool, justifier: bool) -> Enfant {
        Enfant {
            id: Uuid::from_u128(id),
            nom: "Potter".into(),
            prenom: "Harry".into(),
            classe: Some("4B".into()),
            status: "actif".into(),
            lien: "mere".into(),
            peut_voir_notes: true,
            peut_voir_vs: voir_vs,
            peut_justifier_absence: justifier,
            peut_autoriser_sorties: false,
        }
    }

    fn motifs() -> Vec<MotifAbsence> {
        vec![
            MotifAbsence {
                code: "appel".into(),
                libelle: "Absent à l'appel".into(),
                etablissement_id: None,
            },
            MotifAbsence {
                code: "maladie".into(),
                libelle: "Maladie".into(),
                etablissement_id: None,
            },
            MotifAbsence {
                code: "rdv_medical".into(),
                libelle: " Rendez-vous médical ".into(),
                etablissement_id: None,
            },
            MotifAbsence {
                code: "autre".into(),
                libelle: "Autre".into(),
                etablissement_id: None,
            },
        ]
    }

    #[test]
    fn acces_selon_le_lien() {
        assert_eq!(acces(&enfant(1, true, true)), Acces::Complet);
        assert_eq!(acces(&enfant(1, true, false)), Acces::Complet);
        assert_eq!(acces(&enfant(1, false, true)), Acces::JustificationSeule);
        assert_eq!(acces(&enfant(1, false, false)), Acces::Aucun);
        assert_eq!(nom_enfant(&enfant(1, true, true)), "Harry Potter");
    }

    #[test]
    fn enfant_par_defaut() {
        let e = vec![enfant(1, true, true), enfant(2, true, true)];
        assert_eq!(enfant_choisi(&e, None), Some(Uuid::from_u128(1)));
        assert_eq!(
            enfant_choisi(&e, Some(Uuid::from_u128(2))),
            Some(Uuid::from_u128(2))
        );
        // Enfant demandé inconnu (adresse modifiée à la main) : le premier, jamais un autre foyer.
        assert_eq!(
            enfant_choisi(&e, Some(Uuid::from_u128(99))),
            Some(Uuid::from_u128(1))
        );
        assert_eq!(enfant_choisi(&[], None), None);
    }

    #[test]
    fn justification_proposee_seulement_quand_possible() {
        let libre = absence(1, "a_traiter", None, "2026-11-03");
        let attente = absence(2, "a_traiter", Some("Maladie"), "2026-11-03");
        let decidee = absence(3, "justifiee", Some("Maladie"), "2026-11-03");
        let refusee = absence(4, "refusee", None, "2026-11-03");
        let ok = enfant(1, true, true);
        assert!(peut_justifier(&ok, &libre));
        assert!(!peut_justifier(&ok, &attente), "déjà justifiée, en attente");
        assert!(!peut_justifier(&ok, &decidee));
        assert!(
            !peut_justifier(&ok, &refusee),
            "le serveur refuse une absence déjà refusée"
        );
        assert!(peut_retirer(&ok, &attente));
        assert!(!peut_retirer(&ok, &libre));
        // Sans le drapeau : rien n'est proposé.
        let sans = enfant(1, true, false);
        assert!(!peut_justifier(&sans, &libre));
        assert!(!peut_retirer(&sans, &attente));
        assert_eq!(a_justifier(&[libre, attente, decidee, refusee]), 1);
    }

    #[test]
    fn tri_recent_d_abord() {
        let l = recentes_d_abord(vec![
            absence(1, "a_traiter", None, "2026-10-01"),
            absence(2, "a_traiter", None, "2026-11-03"),
        ]);
        assert_eq!(l[0].id, Uuid::from_u128(2));
    }

    #[test]
    fn options_de_motif() {
        let o = options_motifs(&motifs());
        let codes: Vec<&str> = o.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(
            codes,
            ["maladie", "rdv_medical", "autre"],
            "« autre » une seule fois, en dernier"
        );
        assert_eq!(o[1].1, "Rendez-vous médical", "libellé nettoyé");
        // Sans motif du serveur : au moins le choix libre.
        assert_eq!(options_motifs(&[]).len(), 1);
    }

    #[test]
    fn composition_du_motif() {
        let m = motifs();
        assert_eq!(
            composer_justification(&m, "maladie", "").unwrap(),
            "Maladie"
        );
        assert_eq!(
            composer_justification(&m, "maladie", "  fièvre depuis hier ").unwrap(),
            "Maladie — fièvre depuis hier"
        );
        // « Autre » : précision obligatoire.
        assert!(composer_justification(&m, "autre", "   ").is_err());
        assert_eq!(
            composer_justification(&m, "autre", "Décès dans la famille").unwrap(),
            "Décès dans la famille"
        );
        // « Absent à l'appel » n'est pas une justification : jamais proposé ni accepté.
        assert!(composer_justification(&m, "appel", "x").is_err());
        // Code inconnu : refus (on n'envoie pas un motif que la liste ne propose pas).
        assert!(composer_justification(&m, "inconnu", "x").is_err());
        // Borne.
        assert!(composer_justification(&m, "maladie", &"é".repeat(MOTIF_MAX)).is_err());
        assert!(composer_justification(&m, "autre", &"é".repeat(MOTIF_MAX)).is_ok());
        assert!(message_sans_droit_de_justifier("Harry").contains("Harry"));
    }
}
