//! Logique d'écran pure du lot C4 (charge de travail) : sans Leptos ni réseau, testée en natif.
//!
//! **Aucune règle métier ici** : le calcul, les repères, les droits et la minimisation (nombres et natures seulement)
//! restent au serveur. Ce module formate ce que le serveur annonce et décide quand interroger l'aperçu.

use chrono::NaiveDate;
use ent_web_api::cahier_c4_types::{
    AlerteChargeDto, ApercuBody, ApercuChargeDto, DistributionDto, REPERE_EVALUATIONS,
    REPERE_MINUTES, REPERE_SEMAINE,
};
use uuid::Uuid;

use super::modele::libelle_duree;
use super::modele_c2::lire_jour;

/// Corps de l'aperçu d'après le formulaire « donner un travail » : un jour posé (mode « date ») ou la prochaine séance.
/// `None` tant que le formulaire ne permet pas d'interroger (aucun cours coché, date illisible).
pub fn corps_apercu(
    cours: Vec<Uuid>,
    mode: &str,
    jour: &str,
    nature: &str,
    duree: &str,
) -> Option<ApercuBody> {
    if cours.is_empty() {
        return None;
    }
    let (echeance_jour, prochaine_seance) = if mode == "date" {
        (Some(lire_jour(jour)?), false)
    } else {
        (None, true)
    };
    Some(ApercuBody {
        cours_ids: cours,
        echeance_jour,
        prochaine_seance,
        nature: (!nature.trim().is_empty()).then(|| nature.trim().to_string()),
        duree_estimee_min: duree
            .trim()
            .parse::<i16>()
            .ok()
            .filter(|d| (1..=600).contains(d)),
    })
}

/// Phrase d'un repère dépassé, lue telle quelle par les lecteurs d'écran.
pub fn message_alerte(a: &AlerteChargeDto) -> String {
    let n = a.nb_eleves;
    let eleves = if n > 1 {
        format!("{n} élèves")
    } else {
        "1 élève".to_string()
    };
    match a.repere.as_str() {
        REPERE_MINUTES => format!(
            "{eleves} dépasseraient {} de travail le soir (jusqu'à {}).",
            libelle_duree(a.seuil),
            libelle_duree(a.valeur_max)
        ),
        REPERE_EVALUATIONS => format!(
            "{eleves} auraient plus de {} évaluation(s) ce jour-là (jusqu'à {}).",
            a.seuil, a.valeur_max
        ),
        REPERE_SEMAINE => format!(
            "{eleves} auraient plus de {} évaluations sur la semaine (jusqu'à {}).",
            a.seuil, a.valeur_max
        ),
        _ => format!("{eleves} au-dessus d'un repère de charge."),
    }
}

/// « 9 élèves sur 35 ont déjà 1 évaluation ce jour-là ».
pub fn lignes_distribution(d: &[DistributionDto], total: i32) -> Vec<String> {
    d.iter()
        .filter(|x| x.valeur > 0)
        .map(|x| {
            let eleves = if x.nb_eleves > 1 {
                "élèves"
            } else {
                "élève"
            };
            let verbe = if x.nb_eleves > 1 { "ont" } else { "a" };
            let evals = if x.valeur > 1 {
                "évaluations"
            } else {
                "évaluation"
            };
            format!(
                "{} {eleves} sur {total} {verbe} déjà {} {evals} ce jour-là.",
                x.nb_eleves, x.valeur
            )
        })
        .collect()
}

/// Résumé d'un aperçu : une ligne de tête pour le bandeau.
pub fn titre_bandeau(a: &ApercuChargeDto) -> String {
    if a.alerte {
        "Attention à la charge de vos élèves".to_string()
    } else {
        format!("Charge de vos {} élèves : dans les repères", a.nb_eleves)
    }
}

/// Date alternative lisible : « jeudi 19 novembre : 0 élève au-dessus d'un repère ».
pub fn libelle_date_proposee(jour: NaiveDate, nb_au_dessus: i32) -> String {
    let n = match nb_au_dessus {
        0 => "aucun élève au-dessus d'un repère".to_string(),
        1 => "1 élève au-dessus d'un repère".to_string(),
        n => format!("{n} élèves au-dessus d'un repère"),
    };
    format!(
        "{} : {n}",
        crate::noyau::temps::date_longue(jour).to_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alerte(repere: &str, seuil: i32, max: i32, n: i32) -> AlerteChargeDto {
        AlerteChargeDto {
            repere: repere.into(),
            seuil,
            valeur_max: max,
            nb_eleves: n,
        }
    }

    #[test]
    fn corps_apercu_selon_le_mode_et_les_champs() {
        let c = Uuid::from_u128(1);
        assert!(
            corps_apercu(vec![], "date", "2026-11-20", "exercices", "").is_none(),
            "aucun cours coché"
        );
        assert!(
            corps_apercu(vec![c], "date", "", "exercices", "").is_none(),
            "date illisible : pas d'aperçu"
        );
        let b = corps_apercu(vec![c], "date", "2026-11-20", "exercices", " 30 ").unwrap();
        assert_eq!(
            (
                b.echeance_jour.is_some(),
                b.prochaine_seance,
                b.duree_estimee_min
            ),
            (true, false, Some(30))
        );
        let b = corps_apercu(vec![c], "prochaine", "", "", "abc").unwrap();
        assert_eq!(
            (
                b.echeance_jour,
                b.prochaine_seance,
                b.nature,
                b.duree_estimee_min
            ),
            (None, true, None, None)
        );
        assert_eq!(
            corps_apercu(vec![c], "prochaine", "", "x", "9999")
                .unwrap()
                .duree_estimee_min,
            None,
            "durée hors bornes ignorée"
        );
    }

    #[test]
    fn messages_d_alerte_au_singulier_et_au_pluriel() {
        assert_eq!(
            message_alerte(&alerte("minutes", 60, 100, 1)),
            "1 élève dépasseraient 1 h de travail le soir (jusqu'à 1 h 40)."
        );
        assert!(message_alerte(&alerte("evaluations", 2, 4, 17))
            .starts_with("17 élèves auraient plus de 2 évaluation(s)"));
        assert!(message_alerte(&alerte("travaux", 5, 6, 2)).contains("sur la semaine"));
        assert!(message_alerte(&alerte("autre", 1, 2, 3)).contains("3 élèves"));
    }

    #[test]
    fn distribution_sans_la_ligne_des_zero() {
        let d = vec![
            DistributionDto {
                valeur: 0,
                nb_eleves: 9,
            },
            DistributionDto {
                valeur: 1,
                nb_eleves: 1,
            },
            DistributionDto {
                valeur: 2,
                nb_eleves: 8,
            },
        ];
        let l = lignes_distribution(&d, 18);
        assert_eq!(
            l,
            vec![
                "1 élève sur 18 a déjà 1 évaluation ce jour-là.",
                "8 élèves sur 18 ont déjà 2 évaluations ce jour-là."
            ]
        );
    }
}
