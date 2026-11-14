//! Logique d'écran pure du lot C6 (continuité) : sans Leptos ni réseau, testée en natif.
//!
//! **Aucune règle métier ici** : la détection des séances annulées, la séance d'arrivée, l'interdiction d'avancer une échéance
//! sans accord, les droits et le fait qu'aucun motif d'absence n'existe restent au serveur. Ce module formate ce que le serveur
//! annonce et prépare les décisions.

use ent_web_api::cahier_c6_types::{
    BandeauCoursDto, ReportDecisionBody, ReportDto, ReportsLotBody, ReportsLotDto,
};
use uuid::Uuid;

/// « Mardi 12 novembre à 08:00 » : la séance, à l'heure de l'établissement (le serveur donne jour et heure).
pub fn libelle_seance(jour: chrono::NaiveDate, heure: &str) -> String {
    format!("{} à {}", crate::noyau::temps::date_longue(jour), heure)
}

/// Phrase d'une proposition, lue telle quelle par les lecteurs d'écran.
pub fn phrase_report(r: &ReportDto) -> String {
    let objet = if r.objet == "travail" {
        "échéance du travail"
    } else {
        "contenu prévu"
    };
    let origine = libelle_seance(r.origine.jour, &r.origine.heure_debut);
    match &r.vers {
        Some(v) => format!(
            "{} — {objet} de la séance annulée du {origine}, à reporter au {}",
            r.titre,
            libelle_seance(v.jour, &v.heure_debut)
        ),
        None => format!(
            "{} — {objet} de la séance annulée du {origine} : aucune séance suivante du cours, à décider",
            r.titre
        ),
    }
}

/// Corps de la décision d'une proposition. Une échéance avancée n'est acceptée qu'avec la confirmation explicite.
pub fn corps_decision(accepter: bool, r: &ReportDto, confirmer_avance: bool) -> ReportDecisionBody {
    ReportDecisionBody {
        decision: if accepter { "accepter" } else { "refuser" }.into(),
        vers_seance_id: None,
        confirmer_avance: accepter && r.avance_l_echeance && confirmer_avance,
        motif: None,
    }
}

/// Propositions que le lot peut accepter sans rien avancer ni décider à la place du professeur : ouvertes, avec une séance
/// d'arrivée, sans échéance avancée. Les autres se traitent une à une.
pub fn ids_acceptables_en_lot(l: &[ReportDto]) -> Vec<Uuid> {
    l.iter()
        .filter(|r| r.etat == "proposee" && r.vers.is_some() && !r.avance_l_echeance)
        .map(|r| r.id)
        .collect()
}

pub fn corps_lot(ids: Vec<Uuid>, decision: &str) -> Option<ReportsLotBody> {
    (!ids.is_empty()).then(|| ReportsLotBody {
        ids,
        decision: decision.into(),
    })
}

/// Résultat d'un lot : ce qui est fait, et ce qui reste ouvert avec sa raison.
pub fn message_lot(r: &ReportsLotDto) -> String {
    let mut m = format!("{} report(s) appliqué(s), {} refusé(s)", r.appliques, r.refuses);
    if !r.ignores.is_empty() {
        m.push_str(&format!(
            ", {} laissé(s) ouvert(s) : {}",
            r.ignores.len(),
            r.ignores
                .iter()
                .map(|i| i.raison.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(" ; ")
        ));
    }
    m.push('.');
    m
}

/// Lignes du bandeau d'un cours : où en est la continuité, jamais pourquoi.
pub fn lignes_bandeau(b: &BandeauCoursDto) -> Vec<String> {
    let mut l = Vec::new();
    if b.poste_vacant {
        l.push("Professeur en cours de nomination : le cahier n'est pas compté « non tenu ».".to_string());
    }
    if b.suppleance_en_cours {
        l.push("Suppléance en cours.".to_string());
    }
    if b.copies_a_decider > 0 {
        l.push(format!(
            "{} copie(s) en attente de décision (corriger, laisser au titulaire, rendre sans correction).",
            b.copies_a_decider
        ));
    }
    if !b.eleves_en_continuite.is_empty() {
        let noms: Vec<String> = b
            .eleves_en_continuite
            .iter()
            .map(|e| format!("{} {}", e.prenom, e.nom))
            .collect();
        l.push(format!("Élève(s) en continuité à distance : {}.", noms.join(", ")));
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_web_api::cahier_c6_types::{EleveContinuiteDto, ReportIgnoreDto, SeanceReportDto};

    fn report(objet: &str, vers: bool, avance: bool, etat: &str) -> ReportDto {
        let s = |h: &str| SeanceReportDto {
            seance_id: Uuid::nil(),
            jour: chrono::NaiveDate::from_ymd_opt(2026, 11, 12).unwrap(),
            heure_debut: h.into(),
        };
        ReportDto {
            id: Uuid::from_u128(1),
            cours_id: Uuid::nil(),
            objet: objet.into(),
            objet_id: Uuid::nil(),
            titre: "Exercices".into(),
            origine: s("08:00"),
            vers: vers.then(|| s("10:00")),
            etat: etat.into(),
            avance_l_echeance: avance,
            traite_le: None,
        }
    }

    #[test]
    fn phrase_dit_la_seance_et_la_cible() {
        let p = phrase_report(&report("travail", true, false, "proposee"));
        assert!(p.contains("échéance du travail") && p.contains("à reporter au"), "{p}");
        let p = phrase_report(&report("entree", false, false, "proposee"));
        assert!(p.contains("aucune séance suivante"), "{p}");
    }

    #[test]
    fn une_echeance_avancee_exige_la_confirmation() {
        let r = report("travail", true, true, "proposee");
        assert!(!corps_decision(true, &r, false).confirmer_avance);
        assert!(corps_decision(true, &r, true).confirmer_avance);
        assert!(!corps_decision(false, &r, true).confirmer_avance, "un refus ne confirme rien");
    }

    #[test]
    fn le_lot_n_accepte_ni_avance_ni_sans_cible_ni_deja_traite() {
        let l = vec![
            report("travail", true, false, "proposee"),
            report("travail", true, true, "proposee"),
            report("entree", false, false, "proposee"),
            report("travail", true, false, "appliquee"),
        ];
        assert_eq!(ids_acceptables_en_lot(&l).len(), 1);
        assert!(corps_lot(vec![], "accepter").is_none());
    }

    #[test]
    fn message_de_lot_dit_ce_qui_reste_ouvert() {
        let r = ReportsLotDto {
            appliques: 3,
            refuses: 0,
            ignores: vec![ReportIgnoreDto { id: Uuid::nil(), raison: "accès refusé".into() }],
        };
        assert_eq!(message_lot(&r), "3 report(s) appliqué(s), 0 refusé(s), 1 laissé(s) ouvert(s) : accès refusé.");
    }

    #[test]
    fn bandeau_sans_motif() {
        let b = BandeauCoursDto {
            cours_id: Uuid::nil(),
            eleves_en_continuite: vec![EleveContinuiteDto { eleve_id: Uuid::nil(), nom: "DUPONT".into(), prenom: "Léa".into() }],
            poste_vacant: true,
            suppleance_en_cours: false,
            copies_a_decider: 2,
        };
        let l = lignes_bandeau(&b).join(" ");
        assert!(l.contains("continuité à distance") && l.contains("2 copie(s)") && l.contains("nomination"));
        assert!(!l.to_lowercase().contains("exclusion") && !l.to_lowercase().contains("hospital"));
    }
}
