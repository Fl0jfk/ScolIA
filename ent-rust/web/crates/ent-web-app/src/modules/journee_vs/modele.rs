//! Logique d'écran de la « Journée » vie scolaire / accueil (pure, sans Leptos ni réseau,
//! testée en natif) : tri des absents, compteurs, libellés de vérification d'appel.
//!
//! Le serveur reste l'autorité : droits, heure de vérification (H + délai de
//! l'établissement), rapprochement des déclarations. Ici on ne fait que présenter.

use chrono::NaiveTime;
use ent_web_api::{AbsentDuJour, IncoherenceAppel, SeanceSansAppel, TableauJour};
use ent_web_ui::TonEtat;

/// Compteurs affichés en tête de page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Compteurs {
    pub absents: usize,
    /// Absents dont l'absence n'avait pas été annoncée.
    pub non_prevenus: usize,
    pub retards: usize,
    pub declarations: usize,
    pub appels_a_verifier: usize,
    pub incoherences: usize,
    pub a_traiter: usize,
}

pub fn compteurs(t: &TableauJour) -> Compteurs {
    Compteurs {
        absents: t.absents.len(),
        non_prevenus: t.absents.iter().filter(|a| !a.prevenue).count(),
        retards: t.retards.len(),
        declarations: t.declarations.len(),
        appels_a_verifier: t.verification.seances_sans_appel.len(),
        incoherences: t.verification.incoherences.len(),
        a_traiter: usize::try_from(t.a_traiter).unwrap_or(0),
    }
}

/// Absents non prévenus d'abord (ce sont ceux qu'il faut appeler), puis par nom.
pub fn trier_absents(mut liste: Vec<AbsentDuJour>) -> Vec<AbsentDuJour> {
    liste.sort_by(|a, b| {
        a.prevenue
            .cmp(&b.prevenue)
            .then_with(|| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()))
            .then_with(|| a.prenom.to_lowercase().cmp(&b.prenom.to_lowercase()))
    });
    liste
}

pub fn nom_complet(prenom: &str, nom: &str) -> String {
    format!("{} {}", prenom.trim(), nom.trim())
        .trim()
        .to_string()
}

pub fn heure_courte(t: NaiveTime) -> String {
    t.format("%H:%M").to_string()
}

/// « Appel non fait », « Appel non clos ».
pub fn libelle_etat_appel(etat: &str) -> &'static str {
    match etat {
        "non_fait" => "Appel non fait",
        "non_clos" => "Appel non clos",
        _ => "Appel à vérifier",
    }
}

pub fn ton_etat_appel(etat: &str) -> TonEtat {
    match etat {
        "non_fait" => TonEtat::Mal,
        _ => TonEtat::Avertissement,
    }
}

/// « 4B · Potions · 08:00–09:00 ».
pub fn libelle_seance(s: &SeanceSansAppel) -> String {
    format!(
        "{} · {} · {}–{}",
        s.classe,
        s.matiere,
        heure_courte(s.heure_debut),
        heure_courte(s.heure_fin)
    )
}

/// « en retard de 25 min », « 2 relances » — information pour la vie scolaire.
pub fn libelle_retard_appel(s: &SeanceSansAppel) -> String {
    let relances = match s.nb_relances {
        0 => String::new(),
        1 => " · relancé 1 fois".to_string(),
        n => format!(" · relancé {n} fois"),
    };
    if s.minutes_depuis_debut > 0 {
        format!("commencée depuis {} min{relances}", s.minutes_depuis_debut)
    } else {
        format!("pas encore commencée{relances}")
    }
}

pub fn libelle_incoherence(kind: &str) -> &'static str {
    match kind {
        "absent_puis_present" => "Absent puis présent",
        "present_malgre_declaration" => "Présent malgré une absence annoncée",
        _ => "Incohérence",
    }
}

pub fn libelle_absent(a: &AbsentDuJour) -> String {
    nom_complet(&a.prenom, &a.nom)
}

pub fn libelle_incoherence_eleve(i: &IncoherenceAppel) -> String {
    nom_complet(&i.prenom, &i.nom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ent_web_api::{DeclarationDuJour, RetardDuJour, VerificationAppels};
    use uuid::Uuid;

    fn absent(nom: &str, prenom: &str, prevenue: bool) -> AbsentDuJour {
        AbsentDuJour {
            eleve_id: Uuid::nil(),
            nom: nom.into(),
            prenom: prenom.into(),
            absence_id: Uuid::nil(),
            statut: "a_traiter".into(),
            prevenue,
            seances: vec![],
        }
    }

    fn seance(minutes: i64, relances: i64, etat: &str) -> SeanceSansAppel {
        SeanceSansAppel {
            creneau_id: Uuid::nil(),
            classe: "4B".into(),
            matiere: "Potions".into(),
            prof_user_id: Uuid::nil(),
            heure_debut: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            heure_fin: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            etat: etat.into(),
            minutes_depuis_debut: minutes,
            nb_relances: relances,
        }
    }

    fn tableau() -> TableauJour {
        TableauJour {
            jour: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            absents: vec![
                absent("Weasley", "Ginny", true),
                absent("Granger", "Hermione", false),
                absent("Abbott", "Hannah", false),
            ],
            retards: vec![RetardDuJour {
                eleve_id: Uuid::nil(),
                nom: "Potter".into(),
                prenom: "Harry".into(),
                minutes: Some(5),
            }],
            declarations: Vec::<DeclarationDuJour>::new(),
            a_traiter: 3,
            verification: VerificationAppels {
                jour: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
                heure_locale: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                delai_minutes: 15,
                seances_sans_appel: vec![seance(60, 0, "non_fait")],
                incoherences: vec![],
            },
        }
    }

    #[test]
    fn compteurs_du_jour() {
        let c = compteurs(&tableau());
        assert_eq!(
            c,
            Compteurs {
                absents: 3,
                non_prevenus: 2,
                retards: 1,
                declarations: 0,
                appels_a_verifier: 1,
                incoherences: 0,
                a_traiter: 3
            }
        );
    }

    #[test]
    fn non_prevenus_d_abord_puis_ordre_alphabetique() {
        let tries = trier_absents(tableau().absents);
        let noms: Vec<_> = tries.iter().map(|a| a.nom.as_str()).collect();
        assert_eq!(noms, ["Abbott", "Granger", "Weasley"]);
        assert!(!tries[0].prevenue && tries[2].prevenue);
    }

    #[test]
    fn libelles_de_verification() {
        assert_eq!(libelle_etat_appel("non_fait"), "Appel non fait");
        assert_eq!(libelle_etat_appel("non_clos"), "Appel non clos");
        assert_eq!(ton_etat_appel("non_fait"), TonEtat::Mal);
        assert_eq!(ton_etat_appel("non_clos"), TonEtat::Avertissement);
        let s = seance(25, 0, "non_fait");
        assert_eq!(libelle_seance(&s), "4B · Potions · 08:00–09:00");
        assert_eq!(libelle_retard_appel(&s), "commencée depuis 25 min");
        assert_eq!(
            libelle_retard_appel(&seance(15, 2, "non_fait")),
            "commencée depuis 15 min · relancé 2 fois"
        );
        assert_eq!(
            libelle_retard_appel(&seance(0, 1, "non_clos")),
            "pas encore commencée · relancé 1 fois"
        );
        assert_eq!(
            libelle_incoherence("present_malgre_declaration"),
            "Présent malgré une absence annoncée"
        );
        assert_eq!(libelle_incoherence("zzz"), "Incohérence");
    }

    #[test]
    fn noms_nettoyes() {
        assert_eq!(nom_complet(" Harry ", " Potter"), "Harry Potter");
        assert_eq!(nom_complet("", "Potter"), "Potter");
    }
}
