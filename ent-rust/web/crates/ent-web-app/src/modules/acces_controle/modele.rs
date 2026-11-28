//! Modèle **pur** des écrans « Accès aux dossiers » (socle 27, lot G4-2) : libellés, validations, phrases. Aucun appel
//! réseau, aucun droit calculé : le serveur décide (`acces.consulter_traces`, `acces.controle_ouvrir`), l'écran affiche.
//! Aucune statistique par agent ni classement (§11.4) : seules des lignes datées.

use chrono::{DateTime, Utc};
use ent_web_api::acces_traces_types::{AlerteAccesDto, ControleDto, TraceLigneDto};

pub const DUREE_MAX_CONTROLE: i64 = 31;
pub const DUREE_MAX_REPORT: i64 = 90;

/// Libellé d'un motif de contrôle.
pub fn libelle_motif_controle(code: &str) -> &'static str {
    match code {
        "signalement" => "Signalement",
        "alerte" => "Alerte d'accès",
        "plainte" => "Plainte",
        "verification_periodique" => "Vérification périodique",
        _ => "Motif inconnu",
    }
}

pub fn libelle_issue(code: &str) -> &'static str {
    match code {
        "rien" => "Rien à signaler",
        "violation" => "Manquement constaté",
        "transmis" => "Transmis (autorité compétente)",
        _ => "Issue inconnue",
    }
}

pub fn libelle_suite(code: &str) -> &'static str {
    match code {
        "a_traiter" => "À traiter",
        "rien" => "Rien à signaler",
        "violation" => "Manquement constaté",
        "controle" => "Contrôle ouvert",
        _ => "Suite inconnue",
    }
}

/// Un contrôle est recevable : motif connu, précision de 5 à 1000 caractères, durée de 1 à 31 jours, agent choisi.
pub fn controle_recevable(
    agent_choisi: bool,
    motif: &str,
    texte: &str,
    jours: i64,
) -> Result<(), &'static str> {
    if !agent_choisi {
        return Err("Choisissez l'agent concerné.");
    }
    if ![
        "signalement",
        "alerte",
        "plainte",
        "verification_periodique",
    ]
    .contains(&motif)
    {
        return Err("Choisissez le motif du contrôle.");
    }
    let n = texte.trim().chars().count();
    if !(5..=1000).contains(&n) {
        return Err("Précisez le motif (5 à 1000 caractères).");
    }
    if !(1..=DUREE_MAX_CONTROLE).contains(&jours) {
        return Err("La durée d'un contrôle va de 1 à 31 jours.");
    }
    Ok(())
}

/// Un report d'affichage est recevable : motif de 5 à 500 caractères, durée de 1 à 90 jours.
pub fn report_recevable(motif: &str, jours: i64) -> Result<(), &'static str> {
    if !(5..=500).contains(&motif.trim().chars().count()) {
        return Err("Motivez la décision (5 à 500 caractères).");
    }
    if !(1..=DUREE_MAX_REPORT).contains(&jours) {
        return Err("Le report va de 1 à 90 jours.");
    }
    Ok(())
}

/// Lit une durée saisie en jours (entier positif) ; 0 si illisible (refusé ensuite par la validation).
pub fn jours_depuis_texte(t: &str) -> i64 {
    t.trim().parse::<i64>().unwrap_or(0)
}

/// Phrase d'un contrôle : « Ouvert jusqu'au 30/01/2027 » ou « Clos : rien à signaler ».
pub fn phrase_controle(c: &ControleDto) -> String {
    match (&c.issue, c.ouvert) {
        (Some(i), _) => format!("Clos : {}", libelle_issue(i).to_lowercase()),
        (None, true) => format!("Ouvert jusqu'au {}", c.echeance_le.format("%d/%m/%Y")),
        (None, false) => "Échu sans clôture : à clore".to_string(),
    }
}

/// Alerte à traiter : seule une alerte à traiter propose une suite.
pub fn alerte_a_traiter(a: &AlerteAccesDto) -> bool {
    a.suite == "a_traiter"
}

/// Phrase d'une alerte : « 16 consultations — le 15/01/2027 à 10:00 (UTC) ». L'écran affiche l'heure telle que reçue.
pub fn phrase_alerte(a: &AlerteAccesDto) -> String {
    format!(
        "{} consultation(s) — fenêtre du {}",
        a.nombre,
        a.fenetre_debut.format("%d/%m/%Y %H:%M")
    )
}

/// « Enseignant · Enseignement » ; pour le support de l'éditeur, la fonction seule.
pub fn qui(l: &TraceLigneDto) -> String {
    match (l.fonction.trim(), l.service.trim()) {
        ("", "") => "Personnel de l'établissement".to_string(),
        (f, "") => f.to_string(),
        ("", s) => s.to_string(),
        (f, s) => format!("{f} · {s}"),
    }
}

/// Précisions d'une ligne : hors périmètre avec le motif, périmètre temporaire, report, nom communiqué.
pub fn precisions(l: &TraceLigneDto) -> Vec<String> {
    let mut v = Vec::new();
    if l.hors_perimetre {
        v.push(match &l.motif {
            Some(m) => format!("Hors périmètre — motif : {m}"),
            None => "Hors périmètre".to_string(),
        });
    }
    if let Some(t) = &l.perimetre_temporaire {
        v.push(format!(
            "Périmètre temporaire : {}",
            libelle_perimetre_temporaire(t)
        ));
    }
    if l.differee {
        v.push("Affichage différé pour la famille".to_string());
    }
    if !l.sujet.trim().is_empty() {
        v.push(format!("Élève : {}", l.sujet));
    }
    v
}

pub fn libelle_perimetre_temporaire(code: &str) -> &'static str {
    match code {
        "remplacement" => "remplacement",
        "conseil_classe" => "conseil de classe",
        "sortie" => "sortie",
        _ => "autre",
    }
}

/// Date d'une ligne, sans seconde.
pub fn quand(le: DateTime<Utc>) -> String {
    le.format("%d/%m/%Y %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ligne() -> TraceLigneDto {
        TraceLigneDto {
            id: Uuid::nil(),
            le: Utc.with_ymd_and_hms(2027, 1, 15, 10, 0, 0).unwrap(),
            fonction: "Enseignant".into(),
            service: "Enseignement".into(),
            nature: "Consultation".into(),
            section: "Scolarité".into(),
            hors_perimetre: false,
            perimetre_temporaire: None,
            motif: None,
            sujet: String::new(),
            differee: false,
            nom_communique: false,
            agent_nom: None,
        }
    }

    #[test]
    fn un_controle_demande_agent_motif_precision_et_duree_bornee() {
        assert!(controle_recevable(true, "plainte", "Plainte d'une famille", 10).is_ok());
        assert!(controle_recevable(false, "plainte", "Plainte d'une famille", 10).is_err());
        assert!(controle_recevable(true, "autre", "Plainte d'une famille", 10).is_err());
        assert!(controle_recevable(true, "plainte", "ok", 10).is_err());
        assert!(controle_recevable(true, "plainte", "Plainte d'une famille", 0).is_err());
        assert!(controle_recevable(true, "plainte", "Plainte d'une famille", 32).is_err());
        assert!(controle_recevable(true, "plainte", "Plainte d'une famille", 31).is_ok());
    }

    #[test]
    fn un_report_est_motive_et_borne_a_quatre_vingt_dix_jours() {
        assert!(report_recevable("Enquête en cours", 90).is_ok());
        assert!(report_recevable("Enquête en cours", 91).is_err());
        assert!(report_recevable("Enquête en cours", 0).is_err());
        assert!(report_recevable("non", 10).is_err());
    }

    #[test]
    fn les_durees_illisibles_valent_zero() {
        assert_eq!(jours_depuis_texte(" 12 "), 12);
        assert_eq!(jours_depuis_texte("abc"), 0);
        assert_eq!(jours_depuis_texte(""), 0);
    }

    #[test]
    fn une_ligne_porte_la_fonction_et_le_service_jamais_un_nom() {
        let mut l = ligne();
        assert_eq!(qui(&l), "Enseignant · Enseignement");
        l.service.clear();
        assert_eq!(qui(&l), "Enseignant");
        l.fonction.clear();
        assert_eq!(qui(&l), "Personnel de l'établissement");
    }

    #[test]
    fn les_precisions_disent_hors_perimetre_motif_report_et_eleve() {
        let mut l = ligne();
        assert!(precisions(&l).is_empty());
        l.hors_perimetre = true;
        l.motif = Some("Urgence".into());
        l.differee = true;
        l.sujet = "Harry Potter".into();
        let p = precisions(&l);
        assert!(p.iter().any(|x| x.contains("motif : Urgence")));
        assert!(p.iter().any(|x| x.contains("différé")));
        assert!(p.iter().any(|x| x.contains("Harry Potter")));
        l.perimetre_temporaire = Some("remplacement".into());
        assert!(precisions(&l).iter().any(|x| x.contains("remplacement")));
    }

    #[test]
    fn la_phrase_d_un_controle_suit_son_etat() {
        let base = ControleDto {
            id: Uuid::nil(),
            agent_compte_id: Uuid::nil(),
            agent_nom: "A".into(),
            motif_code: "plainte".into(),
            motif_texte: "Plainte".into(),
            ouvert_le: Utc.with_ymd_and_hms(2027, 1, 15, 10, 0, 0).unwrap(),
            echeance_le: Utc.with_ymd_and_hms(2027, 1, 30, 10, 0, 0).unwrap(),
            clos_le: None,
            issue: None,
            ouvert: true,
        };
        assert_eq!(phrase_controle(&base), "Ouvert jusqu'au 30/01/2027");
        let echu = ControleDto {
            ouvert: false,
            ..base.clone()
        };
        assert!(phrase_controle(&echu).contains("à clore"));
        let clos = ControleDto {
            issue: Some("rien".into()),
            ouvert: false,
            ..base
        };
        assert_eq!(phrase_controle(&clos), "Clos : rien à signaler");
    }

    #[test]
    fn seule_une_alerte_a_traiter_propose_une_suite() {
        let mut a = AlerteAccesDto {
            id: Uuid::nil(),
            regle: "rafale_hors_perimetre".into(),
            libelle: "Rafale".into(),
            agent_compte_id: Uuid::nil(),
            agent_nom: "A".into(),
            fenetre_debut: Utc.with_ymd_and_hms(2027, 1, 15, 10, 0, 0).unwrap(),
            fenetre_fin: Utc.with_ymd_and_hms(2027, 1, 15, 10, 10, 0).unwrap(),
            nombre: 16,
            cree_le: Utc.with_ymd_and_hms(2027, 1, 15, 11, 0, 0).unwrap(),
            suite: "a_traiter".into(),
            suite_le: None,
            controle_id: None,
        };
        assert!(alerte_a_traiter(&a));
        assert!(phrase_alerte(&a).starts_with("16 consultation(s)"));
        a.suite = "rien".into();
        assert!(!alerte_a_traiter(&a));
        assert_eq!(libelle_suite("controle"), "Contrôle ouvert");
    }
}
