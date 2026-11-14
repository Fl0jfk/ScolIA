//! Logique d'écran pure de « À faire » (sans Leptos ni réseau, testée en natif).
//! Aucune règle métier : droits, cardinalités et neutralité entre parents restent au serveur.

use chrono::{DateTime, Duration, Utc};
use ent_types::afaire::{ChampAction, EtatAfaire, PrioriteAfaire, TacheAfaire};

/// Texte de l'échéance d'une tâche, relatif à `maintenant` (passé par l'appelant : pas d'horloge ici).
pub fn libelle_echeance(
    echeance: Option<DateTime<Utc>>,
    maintenant: DateTime<Utc>,
) -> Option<String> {
    let e = echeance?;
    if e < maintenant {
        return Some("En retard".into());
    }
    if e - maintenant < Duration::hours(24) {
        return Some("À faire aujourd'hui ou demain".into());
    }
    Some(format!("À faire avant le {}", e.format("%d/%m/%Y")))
}

/// Libellé de priorité : le sens ne repose jamais sur la couleur seule.
pub fn libelle_priorite(p: PrioriteAfaire) -> Option<&'static str> {
    match p {
        PrioriteAfaire::Bloquante => Some("Bloque une suite"),
        PrioriteAfaire::Due => Some("À traiter vite"),
        PrioriteAfaire::AFaire => None,
        PrioriteAfaire::Information => Some("Pour information"),
    }
}

/// Titre de la page selon le nombre de tâches actionnables.
pub fn titre_compteur(n: i64) -> String {
    match n {
        0 => "Rien à faire pour l'instant".into(),
        1 => "1 chose à faire".into(),
        n => format!("{n} choses à faire"),
    }
}

/// Une carte déjà traitée (close par quelqu'un, encore affichée jusqu'au point neutre) : jamais de
/// bouton d'action, et rien qui dise par qui ni quand (R-PO7, R-PO38).
pub fn est_actionnable(t: &TacheAfaire) -> bool {
    t.etat.est_ouvert()
}

/// Mention affichée sur une carte déjà traitée.
pub fn mention_traitee(t: &TacheAfaire) -> Option<&'static str> {
    (t.etat == EtatAfaire::Faite).then_some("Cette démarche est déjà traitée.")
}

/// Valeurs saisies pour une carte : le motif est le seul champ de texte aujourd'hui.
pub fn motif_valide(champs: &[ChampAction], motif: &str) -> Result<Option<String>, &'static str> {
    let requis = champs.iter().any(|c| c.code == "motif" && c.obligatoire);
    let m = motif.trim();
    if requis && m.is_empty() {
        return Err("Indiquez le motif.");
    }
    if m.chars().count() > 300 {
        return Err("Le motif est trop long (300 caractères au plus).");
    }
    Ok((!m.is_empty()).then(|| m.to_string()))
}

/// Libellé d'une réponse de carte à choix : le sens est dit en toutes lettres.
pub fn libelle_choix(valeur: &str) -> &str {
    match valeur {
        "accepte" => "J'accepte (j'autorise)",
        "refuse" => "Je refuse",
        autre => autre,
    }
}

/// Réponse choisie sur une carte à choix : obligatoire si le champ l'est, et toujours l'un des
/// choix proposés (le serveur revérifie, l'écran évite l'aller-retour inutile).
pub fn decision_valide(
    champs: &[ChampAction],
    decision: &str,
) -> Result<Option<String>, &'static str> {
    let Some(champ) = champs.iter().find(|c| c.code == "decision") else {
        return Ok(None);
    };
    let d = decision.trim();
    if d.is_empty() {
        return if champ.obligatoire {
            Err("Choisissez votre réponse.")
        } else {
            Ok(None)
        };
    }
    if !champ.choix.iter().any(|c| c == d) {
        return Err("Cette réponse n'est pas proposée.");
    }
    Ok(Some(d.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_types::afaire::ActionTache;
    use uuid::Uuid;

    fn t0() -> DateTime<Utc> {
        "2026-11-12T10:00:00Z".parse().unwrap()
    }

    fn tache(etat: EtatAfaire) -> TacheAfaire {
        TacheAfaire {
            id: Uuid::nil(),
            nature: "vs.justifier_absence".into(),
            titre: "Absence à justifier — Léa, 12/11".into(),
            eleve_id: None,
            eleve_prenom: Some("Léa".into()),
            priorite: PrioriteAfaire::AFaire,
            etat,
            echeance_le: None,
            nouveau: false,
            action: Some(ActionTache::Carte {
                libelle: "Justifier".into(),
            }),
            annee: None,
        }
    }

    #[test]
    fn echeance_en_retard_proche_ou_lointaine() {
        assert_eq!(libelle_echeance(None, t0()), None);
        assert_eq!(
            libelle_echeance(Some(t0() - Duration::hours(1)), t0()).as_deref(),
            Some("En retard")
        );
        assert_eq!(
            libelle_echeance(Some(t0() + Duration::hours(5)), t0()).as_deref(),
            Some("À faire aujourd'hui ou demain")
        );
        assert_eq!(
            libelle_echeance(Some(t0() + Duration::days(3)), t0()).as_deref(),
            Some("À faire avant le 15/11/2026")
        );
    }

    #[test]
    fn la_priorite_a_toujours_un_libelle_sauf_la_normale() {
        assert!(libelle_priorite(PrioriteAfaire::Bloquante).is_some());
        assert!(libelle_priorite(PrioriteAfaire::Due).is_some());
        assert!(libelle_priorite(PrioriteAfaire::AFaire).is_none());
    }

    #[test]
    fn compteur_au_singulier_et_au_pluriel() {
        assert_eq!(titre_compteur(0), "Rien à faire pour l'instant");
        assert_eq!(titre_compteur(1), "1 chose à faire");
        assert_eq!(titre_compteur(4), "4 choses à faire");
    }

    /// Anomalie évitée : une carte déjà traitée ne propose plus d'action et n'indique ni auteur ni date.
    #[test]
    fn une_carte_deja_traitee_n_a_pas_d_action_et_ne_nomme_personne() {
        let faite = tache(EtatAfaire::Faite);
        assert!(!est_actionnable(&faite));
        assert_eq!(
            mention_traitee(&faite),
            Some("Cette démarche est déjà traitée.")
        );
        let ouverte = tache(EtatAfaire::Ouverte);
        assert!(est_actionnable(&ouverte));
        assert_eq!(mention_traitee(&ouverte), None);
    }

    #[test]
    fn le_motif_obligatoire_est_exige_et_borne() {
        let champs = vec![ChampAction {
            code: "motif".into(),
            libelle: "Motif".into(),
            obligatoire: true,
            choix: vec![],
        }];
        assert_eq!(motif_valide(&champs, "  "), Err("Indiquez le motif."));
        assert_eq!(
            motif_valide(&champs, " Rendez-vous médical "),
            Ok(Some("Rendez-vous médical".into()))
        );
        assert!(motif_valide(&champs, &"x".repeat(301)).is_err());
        assert_eq!(
            motif_valide(&[], ""),
            Ok(None),
            "sans champ : rien à saisir"
        );
    }

    #[test]
    fn la_decision_est_exigee_et_limitee_aux_choix() {
        let champs = vec![ChampAction {
            code: "decision".into(),
            libelle: "Votre réponse".into(),
            obligatoire: true,
            choix: vec!["accepte".into(), "refuse".into()],
        }];
        assert_eq!(
            decision_valide(&champs, ""),
            Err("Choisissez votre réponse.")
        );
        assert_eq!(
            decision_valide(&champs, "accepte"),
            Ok(Some("accepte".into()))
        );
        assert!(decision_valide(&champs, "peut-etre").is_err());
        assert_eq!(
            decision_valide(&[], ""),
            Ok(None),
            "sans champ : rien à choisir"
        );
        assert_eq!(libelle_choix("refuse"), "Je refuse");
    }
}
