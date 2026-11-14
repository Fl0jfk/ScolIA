//! Logique de l'écran « Changements d'emploi du temps » de l'élève et de sa famille (pure, sans
//! Leptos ni réseau, testée en natif) : période, regroupement par jour et libellés par nature
//! de changement. Le serveur ne renvoie jamais de motif libre : seul le motif **public** d'un lot
//! (grève, fermeture…) est affiché.

use chrono::{Duration, NaiveDate, NaiveTime};
use ent_types::edt_s2::ChangementEdtDto;
use ent_web_ui::TonEtat;

use crate::noyau::temps::date_longue;

/// Durée de la période affichée par défaut (jours, bornes comprises : 14 jours à venir).
pub const JOURS_PAR_DEFAUT: i64 = 14;

/// Plage maximale acceptée par l'écran (jours).
pub const JOURS_MAX: i64 = 92;

/// Période par défaut : du jour aux 14 jours suivants (bornes comprises).
pub fn periode_par_defaut(aujourdhui: NaiveDate) -> (NaiveDate, NaiveDate) {
    (
        aujourdhui,
        aujourdhui
            .checked_add_signed(Duration::days(JOURS_PAR_DEFAUT - 1))
            .unwrap_or(aujourdhui),
    )
}

/// Période saisie : dates lisibles, fin après le début, plage bornée.
pub fn valider_periode(du: &str, au: &str) -> Result<(NaiveDate, NaiveDate), String> {
    let lire = |t: &str| NaiveDate::parse_from_str(t.trim(), "%Y-%m-%d").ok();
    let (Some(d), Some(a)) = (lire(du), lire(au)) else {
        return Err("Indiquez une date de début et une date de fin.".into());
    };
    if a < d {
        return Err("La date de fin doit suivre la date de début.".into());
    }
    if (a - d).num_days() >= JOURS_MAX {
        return Err(format!(
            "La période ne peut pas dépasser {JOURS_MAX} jours."
        ));
    }
    Ok((d, a))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nature {
    Annulation,
    Remplacement,
    ChangementSalle,
    Deplacement,
    Inconnue,
}

impl Nature {
    pub fn depuis(texte: &str) -> Nature {
        match texte {
            "annulation" => Nature::Annulation,
            "remplacement" => Nature::Remplacement,
            "changement_salle" => Nature::ChangementSalle,
            "deplacement" => Nature::Deplacement,
            _ => Nature::Inconnue,
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Nature::Annulation => "Cours annulé",
            Nature::Remplacement => "Cours assuré par un autre enseignant",
            Nature::ChangementSalle => "Changement de salle",
            Nature::Deplacement => "Cours déplacé",
            Nature::Inconnue => "Modification du cours",
        }
    }

    pub fn ton(self) -> TonEtat {
        match self {
            Nature::Annulation => TonEtat::Mal,
            Nature::Inconnue => TonEtat::Neutre,
            _ => TonEtat::Avertissement,
        }
    }
}

fn heure(h: NaiveTime) -> String {
    h.format("%H:%M").to_string()
}

fn present(texte: &Option<String>) -> Option<&str> {
    texte.as_deref().map(str::trim).filter(|t| !t.is_empty())
}

/// Précision propre à la nature du changement.
pub fn detail(c: &ChangementEdtDto) -> Option<String> {
    match Nature::depuis(&c.nature) {
        Nature::Remplacement => present(&c.remplacant_nom).map(|n| format!("Remplaçant : {n}")),
        Nature::ChangementSalle => {
            present(&c.nouvelle_salle).map(|s| format!("Nouvelle salle : {s}"))
        }
        Nature::Deplacement => {
            let jour = c.nouveau_jour.map(|j| date_longue(j).to_lowercase());
            let horaire = match (c.nouvelle_heure_debut, c.nouvelle_heure_fin) {
                (Some(d), Some(f)) => Some(format!("de {} à {}", heure(d), heure(f))),
                (Some(d), None) => Some(format!("à {}", heure(d))),
                _ => None,
            };
            let salle = present(&c.nouvelle_salle).map(|s| format!("salle {s}"));
            let morceaux: Vec<String> = [jour, horaire, salle].into_iter().flatten().collect();
            (!morceaux.is_empty()).then(|| format!("Nouveau créneau : {}", morceaux.join(", ")))
        }
        Nature::Annulation | Nature::Inconnue => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangementAffiche {
    pub cle: String,
    /// « 08:00 – 09:00 ».
    pub horaire: String,
    /// « Maths · 4B ».
    pub cours: String,
    pub nature: Nature,
    pub detail: Option<String>,
    /// « Fermeture exceptionnelle », motif public d'un lot seulement.
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupeJour {
    pub jour: NaiveDate,
    pub titre: String,
    pub changements: Vec<ChangementAffiche>,
}

/// Regroupe par jour (ordre chronologique) et trie par heure dans la journée.
pub fn grouper(changements: &[ChangementEdtDto]) -> Vec<GroupeJour> {
    let mut tries: Vec<&ChangementEdtDto> = changements.iter().collect();
    tries.sort_by(|a, b| {
        (
            a.jour,
            a.heure_debut,
            &a.matiere_libelle,
            &a.nature,
            a.creneau_id,
        )
            .cmp(&(
                b.jour,
                b.heure_debut,
                &b.matiere_libelle,
                &b.nature,
                b.creneau_id,
            ))
    });
    let mut groupes: Vec<GroupeJour> = Vec::new();
    for c in tries {
        let affiche = ChangementAffiche {
            cle: format!("{}|{}|{}", c.creneau_id, c.jour, c.nature),
            horaire: format!("{} – {}", heure(c.heure_debut), heure(c.heure_fin)),
            cours: [c.matiere_libelle.trim(), c.classe.trim()]
                .into_iter()
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" · "),
            nature: Nature::depuis(&c.nature),
            detail: detail(c),
            motif: present(&c.motif_public).map(str::to_string),
        };
        match groupes.last_mut().filter(|g| g.jour == c.jour) {
            Some(g) => g.changements.push(affiche),
            None => groupes.push(GroupeJour {
                jour: c.jour,
                titre: date_longue(c.jour),
                changements: vec![affiche],
            }),
        }
    }
    groupes
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn h(heure: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(heure, 0, 0).unwrap()
    }

    fn changement(n: u128, jour: NaiveDate, debut: u32, nature: &str) -> ChangementEdtDto {
        ChangementEdtDto {
            creneau_id: Uuid::from_u128(n),
            jour,
            nature: nature.into(),
            matiere_libelle: "Maths".into(),
            classe: "4B".into(),
            heure_debut: h(debut),
            heure_fin: h(debut + 1),
            nouvelle_salle: None,
            remplacant_nom: None,
            nouveau_jour: None,
            nouvelle_heure_debut: None,
            nouvelle_heure_fin: None,
            motif_public: None,
            enregistre_le: Utc.with_ymd_and_hms(2026, 11, 6, 8, 0, 0).unwrap(),
        }
    }

    #[test]
    fn periode_de_quatorze_jours() {
        let (du, au) = periode_par_defaut(d(2026, 11, 9));
        assert_eq!((du, au), (d(2026, 11, 9), d(2026, 11, 22)));
        assert_eq!((au - du).num_days() + 1, 14);
        // Date extrême : aucune panique.
        let _ = periode_par_defaut(NaiveDate::MAX);
    }

    #[test]
    fn validation_de_la_periode() {
        assert_eq!(
            valider_periode("2026-11-09", " 2026-11-12 "),
            Ok((d(2026, 11, 9), d(2026, 11, 12)))
        );
        assert!(valider_periode("", "2026-11-12").is_err());
        assert!(valider_periode("2026-11-12", "2026-11-09").is_err());
        assert!(
            valider_periode("2026-01-01", "2026-12-31").is_err(),
            "trop longue"
        );
        assert!(
            valider_periode("2026-11-09", "2026-11-09").is_ok(),
            "un seul jour"
        );
    }

    #[test]
    fn libelles_par_nature() {
        let mut c = changement(1, d(2026, 11, 9), 8, "annulation");
        assert_eq!(Nature::depuis(&c.nature).libelle(), "Cours annulé");
        assert_eq!(detail(&c), None);

        c.nature = "remplacement".into();
        assert_eq!(
            Nature::depuis(&c.nature).libelle(),
            "Cours assuré par un autre enseignant"
        );
        assert_eq!(detail(&c), None, "pas de nom fourni : rien d'inventé");
        c.remplacant_nom = Some(" M. Rogue ".into());
        assert_eq!(detail(&c).as_deref(), Some("Remplaçant : M. Rogue"));

        c.nature = "changement_salle".into();
        c.nouvelle_salle = Some("C3".into());
        assert_eq!(detail(&c).as_deref(), Some("Nouvelle salle : C3"));

        c.nature = "deplacement".into();
        c.nouveau_jour = Some(d(2026, 11, 12));
        c.nouvelle_heure_debut = Some(h(10));
        c.nouvelle_heure_fin = Some(h(11));
        assert_eq!(
            detail(&c).as_deref(),
            Some("Nouveau créneau : jeudi 12 novembre, de 10:00 à 11:00, salle C3")
        );
        c.nouveau_jour = None;
        c.nouvelle_heure_debut = None;
        c.nouvelle_salle = None;
        assert_eq!(detail(&c), None);

        c.nature = "autre".into();
        assert_eq!(Nature::depuis(&c.nature), Nature::Inconnue);
        assert_eq!(Nature::Annulation.ton(), TonEtat::Mal);
    }

    #[test]
    fn regroupement_chronologique_et_motif_public() {
        let mut lot = changement(3, d(2026, 11, 10), 9, "annulation");
        lot.motif_public = Some(" Fermeture exceptionnelle ".into());
        let liste = vec![
            lot,
            changement(2, d(2026, 11, 9), 14, "remplacement"),
            changement(1, d(2026, 11, 9), 8, "annulation"),
        ];
        let g = grouper(&liste);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].titre, "Lundi 9 novembre");
        let natures: Vec<Nature> = g[0].changements.iter().map(|c| c.nature).collect();
        assert_eq!(natures, [Nature::Annulation, Nature::Remplacement]);
        assert_eq!(g[0].changements[0].horaire, "08:00 – 09:00");
        assert_eq!(g[0].changements[0].cours, "Maths · 4B");
        assert_eq!(g[0].changements[0].motif, None);
        assert_eq!(
            g[1].changements[0].motif.as_deref(),
            Some("Fermeture exceptionnelle")
        );
        assert!(grouper(&[]).is_empty());
    }

    #[test]
    fn motif_vide_ignore() {
        let mut c = changement(1, d(2026, 11, 9), 8, "annulation");
        c.motif_public = Some("   ".into());
        assert_eq!(grouper(&[c])[0].changements[0].motif, None);
    }
}
