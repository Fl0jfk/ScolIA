//! Absences (pure, testée) : statut d'affichage, période, libellés. Partagé par l'accueil,
//! la vie scolaire (personnel) et l'écran « Absences » (famille, élève).
//!
//! Le serveur reste l'autorité : ici on ne fait que traduire ce qu'il renvoie
//! (`statut` + présence d'un `motif_justif`) en un état lisible.

use chrono::{Datelike, NaiveDate};
use ent_web_api::{Absence, Retard};
use ent_web_ui::TonEtat;

use crate::noyau::temps::{nom_jour, nom_mois};

/// Statut d'une absence tel que l'utilisateur le comprend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatutAbsence {
    /// À traiter, aucune justification déposée.
    NonJustifiee,
    /// À traiter, la famille a déposé un motif : la vie scolaire doit trancher.
    JustificatifEnAttente,
    Justifiee,
    Refusee,
    /// Erreur d'appel corrigée : l'élève était présent.
    AnnuleePresent,
}

impl StatutAbsence {
    /// Tous les statuts, dans l'ordre d'affichage des filtres.
    pub const TOUS: [StatutAbsence; 5] = [
        StatutAbsence::NonJustifiee,
        StatutAbsence::JustificatifEnAttente,
        StatutAbsence::Justifiee,
        StatutAbsence::Refusee,
        StatutAbsence::AnnuleePresent,
    ];

    /// Statut d'affichage d'une absence. Un statut inconnu est traité comme « à traiter »
    /// (même repli que le serveur : `AbsenceStatut::parse`).
    pub fn depuis(a: &Absence) -> StatutAbsence {
        match a.statut.as_str() {
            "justifiee" => StatutAbsence::Justifiee,
            "refusee" => StatutAbsence::Refusee,
            "annulee_present" => StatutAbsence::AnnuleePresent,
            _ if a_un_justificatif(a) => StatutAbsence::JustificatifEnAttente,
            _ => StatutAbsence::NonJustifiee,
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            StatutAbsence::NonJustifiee => "Non justifiée",
            StatutAbsence::JustificatifEnAttente => "Justificatif en attente",
            StatutAbsence::Justifiee => "Justifiée",
            StatutAbsence::Refusee => "Refusée",
            StatutAbsence::AnnuleePresent => "Annulée (présent)",
        }
    }

    pub fn ton(self) -> TonEtat {
        match self {
            StatutAbsence::NonJustifiee => TonEtat::Mal,
            StatutAbsence::JustificatifEnAttente => TonEtat::Avertissement,
            StatutAbsence::Justifiee => TonEtat::Ok,
            StatutAbsence::Refusee => TonEtat::Mal,
            StatutAbsence::AnnuleePresent => TonEtat::Neutre,
        }
    }

    /// Valeur stable pour les contrôles (filtre, `data-*`).
    pub fn code(self) -> &'static str {
        match self {
            StatutAbsence::NonJustifiee => "non_justifiee",
            StatutAbsence::JustificatifEnAttente => "en_attente",
            StatutAbsence::Justifiee => "justifiee",
            StatutAbsence::Refusee => "refusee",
            StatutAbsence::AnnuleePresent => "annulee",
        }
    }

    pub fn depuis_code(code: &str) -> Option<StatutAbsence> {
        StatutAbsence::TOUS.into_iter().find(|s| s.code() == code)
    }

    /// Encore à traiter par la vie scolaire (aucune décision prise).
    pub fn est_a_traiter(self) -> bool {
        matches!(
            self,
            StatutAbsence::NonJustifiee | StatutAbsence::JustificatifEnAttente
        )
    }

    /// Compte comme une absence (non annulée).
    pub fn compte_comme_absence(self) -> bool {
        self != StatutAbsence::AnnuleePresent
    }
}

/// Un motif de justification a-t-il été déposé ?
pub fn a_un_justificatif(a: &Absence) -> bool {
    a.motif_justif
        .as_deref()
        .is_some_and(|m| !m.trim().is_empty())
}

/// L'absence couvre-t-elle ce jour ?
pub fn couvre(a: &Absence, jour: NaiveDate) -> bool {
    a.debut <= jour && jour <= a.fin
}

fn numero_jour(d: NaiveDate) -> String {
    if d.day() == 1 {
        "1er".to_string()
    } else {
        d.day().to_string()
    }
}

/// « 3 novembre ».
fn jour_mois(d: NaiveDate) -> String {
    format!("{} {}", numero_jour(d), nom_mois(d.month()))
}

/// « mardi 3 novembre ».
pub fn jour_semaine_mois(d: NaiveDate) -> String {
    format!("{} {}", nom_jour(d.weekday()), jour_mois(d))
}

/// Période lisible : « 3 novembre », « du 3 au 5 novembre », « du 30 octobre au 2 novembre ».
pub fn libelle_periode(debut: NaiveDate, fin: NaiveDate) -> String {
    if debut == fin {
        jour_mois(debut)
    } else if debut.month() == fin.month() && debut.year() == fin.year() {
        format!("du {} au {}", numero_jour(debut), jour_mois(fin))
    } else {
        format!("du {} au {}", jour_mois(debut), jour_mois(fin))
    }
}

/// Nombre de jours calendaires couverts (au moins 1).
pub fn nombre_de_jours(debut: NaiveDate, fin: NaiveDate) -> i64 {
    ((fin - debut).num_days() + 1).max(1)
}

/// Compteurs par statut d'affichage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Compteurs {
    pub non_justifiees: usize,
    pub en_attente: usize,
    pub justifiees: usize,
    pub refusees: usize,
    pub annulees: usize,
}

impl Compteurs {
    pub fn de(absences: &[Absence]) -> Compteurs {
        let mut c = Compteurs::default();
        for a in absences {
            match StatutAbsence::depuis(a) {
                StatutAbsence::NonJustifiee => c.non_justifiees += 1,
                StatutAbsence::JustificatifEnAttente => c.en_attente += 1,
                StatutAbsence::Justifiee => c.justifiees += 1,
                StatutAbsence::Refusee => c.refusees += 1,
                StatutAbsence::AnnuleePresent => c.annulees += 1,
            }
        }
        c
    }

    pub fn pour(&self, s: StatutAbsence) -> usize {
        match s {
            StatutAbsence::NonJustifiee => self.non_justifiees,
            StatutAbsence::JustificatifEnAttente => self.en_attente,
            StatutAbsence::Justifiee => self.justifiees,
            StatutAbsence::Refusee => self.refusees,
            StatutAbsence::AnnuleePresent => self.annulees,
        }
    }
}

/// Nombre d'élèves distincts absents (hors absences annulées).
pub fn eleves_absents(absences: &[Absence]) -> usize {
    let mut ids: Vec<_> = absences
        .iter()
        .filter(|a| StatutAbsence::depuis(a).compte_comme_absence())
        .map(|a| a.eleve_id)
        .collect();
    ids.sort();
    ids.dedup();
    ids.len()
}

/// Nom de l'élève affiché (« Élève » si le serveur n'a pas fourni de libellé).
pub fn nom_eleve(a: &Absence) -> String {
    a.eleve_label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Élève")
        .to_string()
}

/// Jour demandé dans l'adresse (`?jour=AAAA-MM-JJ`) ; valeur absente ou invalide : le défaut.
/// Le booléen indique une valeur présente mais invalide.
pub fn jour_demande(parametre: Option<&str>, defaut: NaiveDate) -> (NaiveDate, bool) {
    match parametre {
        None => (defaut, false),
        Some(p) => match p.parse::<NaiveDate>() {
            Ok(d) if (2000..=2100).contains(&d.year()) => (d, false),
            _ => (defaut, true),
        },
    }
}

/// Libellé d'un retard : « Retard de 7 min », « Retard ».
pub fn libelle_retard(r: &Retard) -> String {
    match r.minutes.filter(|m| *m > 0) {
        Some(m) => format!("Retard de {m} min"),
        None => "Retard".to_string(),
    }
}

/// Un événement de vie scolaire d'un élève, pour la chronologie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evenement {
    Absence(Absence),
    Retard(Retard),
}

impl Evenement {
    /// Date de référence (début de l'absence, jour du retard).
    pub fn date(&self) -> NaiveDate {
        match self {
            Evenement::Absence(a) => a.debut,
            Evenement::Retard(r) => r.jour,
        }
    }
}

/// Absences et retards mêlés, du plus récent au plus ancien (à date égale : absences d'abord).
pub fn chronologie(absences: Vec<Absence>, retards: Vec<Retard>) -> Vec<Evenement> {
    let mut v: Vec<Evenement> = absences
        .into_iter()
        .map(Evenement::Absence)
        .chain(retards.into_iter().map(Evenement::Retard))
        .collect();
    v.sort_by(|a, b| {
        b.date().cmp(&a.date()).then_with(|| {
            let rang = |e: &Evenement| matches!(e, Evenement::Retard(_));
            rang(a).cmp(&rang(b))
        })
    });
    v
}

#[cfg(test)]
pub(crate) mod essais {
    use super::*;
    use uuid::Uuid;

    /// Absence de test d'un seul jour.
    pub fn absence(n: u128, statut: &str, justif: Option<&str>, jour: &str) -> Absence {
        let j: NaiveDate = jour.parse().unwrap();
        Absence {
            id: Uuid::from_u128(n),
            etablissement_id: Uuid::from_u128(999),
            eleve_id: Uuid::from_u128(n),
            eleve_label: Some(format!("Élève {n}")),
            debut: j,
            fin: j,
            motif: "Absence à l'appel".into(),
            statut: statut.into(),
            motif_justif: justif.map(String::from),
            decision_note: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::essais::absence;
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn statut_d_affichage() {
        let s = |statut, justif| StatutAbsence::depuis(&absence(1, statut, justif, "2026-11-03"));
        assert_eq!(s("a_traiter", None), StatutAbsence::NonJustifiee);
        assert_eq!(s("a_traiter", Some("   ")), StatutAbsence::NonJustifiee);
        assert_eq!(
            s("a_traiter", Some("Maladie")),
            StatutAbsence::JustificatifEnAttente
        );
        assert_eq!(s("justifiee", Some("Maladie")), StatutAbsence::Justifiee);
        assert_eq!(s("refusee", None), StatutAbsence::Refusee);
        assert_eq!(s("annulee_present", None), StatutAbsence::AnnuleePresent);
        // Statut inconnu : traité comme à traiter, comme le serveur.
        assert_eq!(s("nouveau_statut", None), StatutAbsence::NonJustifiee);
    }

    #[test]
    fn codes_et_libelles_sont_stables() {
        for s in StatutAbsence::TOUS {
            assert_eq!(StatutAbsence::depuis_code(s.code()), Some(s));
            assert!(!s.libelle().is_empty());
        }
        assert_eq!(StatutAbsence::depuis_code("autre"), None);
        assert!(StatutAbsence::NonJustifiee.est_a_traiter());
        assert!(!StatutAbsence::Justifiee.est_a_traiter());
        assert!(!StatutAbsence::AnnuleePresent.compte_comme_absence());
    }

    #[test]
    fn periodes_en_francais() {
        assert_eq!(
            libelle_periode(d("2026-11-03"), d("2026-11-03")),
            "3 novembre"
        );
        assert_eq!(
            libelle_periode(d("2026-11-01"), d("2026-11-01")),
            "1er novembre"
        );
        assert_eq!(
            libelle_periode(d("2026-11-03"), d("2026-11-05")),
            "du 3 au 5 novembre"
        );
        assert_eq!(
            libelle_periode(d("2026-10-30"), d("2026-11-02")),
            "du 30 octobre au 2 novembre"
        );
        assert_eq!(jour_semaine_mois(d("2026-11-03")), "mardi 3 novembre");
        assert_eq!(nombre_de_jours(d("2026-11-03"), d("2026-11-05")), 3);
        assert_eq!(nombre_de_jours(d("2026-11-05"), d("2026-11-03")), 1);
    }

    #[test]
    fn compteurs_et_absents_distincts() {
        let liste = vec![
            absence(1, "a_traiter", None, "2026-11-03"),
            absence(2, "a_traiter", Some("Maladie"), "2026-11-03"),
            absence(3, "justifiee", Some("RDV"), "2026-11-03"),
            absence(4, "refusee", None, "2026-11-03"),
            absence(5, "annulee_present", None, "2026-11-03"),
        ];
        let c = Compteurs::de(&liste);
        assert_eq!(
            (
                c.non_justifiees,
                c.en_attente,
                c.justifiees,
                c.refusees,
                c.annulees
            ),
            (1, 1, 1, 1, 1)
        );
        assert_eq!(c.pour(StatutAbsence::Justifiee), 1);
        // Les absences annulées ne comptent pas parmi les absents.
        assert_eq!(eleves_absents(&liste), 4);
        let mut deux = liste.clone();
        deux[1].eleve_id = deux[0].eleve_id;
        assert_eq!(
            eleves_absents(&deux),
            3,
            "un élève absent deux fois compte une fois"
        );
    }

    #[test]
    fn couverture_d_un_jour() {
        let mut a = absence(1, "a_traiter", None, "2026-11-03");
        a.fin = d("2026-11-05");
        assert!(couvre(&a, d("2026-11-04")));
        assert!(!couvre(&a, d("2026-11-06")));
        a.eleve_label = Some("  ".into());
        assert_eq!(nom_eleve(&a), "Élève");
    }

    #[test]
    fn chronologie_recente_d_abord() {
        use uuid::Uuid;
        let retard = |n: u128, jour: &str, minutes: Option<i32>| Retard {
            id: Uuid::from_u128(n),
            eleve_id: Uuid::from_u128(1),
            jour: d(jour),
            minutes,
            motif: None,
            source_appel_id: None,
        };
        let c = chronologie(
            vec![
                absence(1, "a_traiter", None, "2026-11-02"),
                absence(2, "justifiee", Some("x"), "2026-11-04"),
            ],
            vec![
                retard(10, "2026-11-04", Some(7)),
                retard(11, "2026-10-20", None),
            ],
        );
        let dates: Vec<String> = c.iter().map(|e| e.date().to_string()).collect();
        assert_eq!(
            dates,
            ["2026-11-04", "2026-11-04", "2026-11-02", "2026-10-20"]
        );
        assert!(
            matches!(c[0], Evenement::Absence(_)),
            "absence avant retard à date égale"
        );
        assert_eq!(
            libelle_retard(&retard(1, "2026-11-04", Some(7))),
            "Retard de 7 min"
        );
        assert_eq!(libelle_retard(&retard(1, "2026-11-04", Some(0))), "Retard");
        assert_eq!(libelle_retard(&retard(1, "2026-11-04", None)), "Retard");
    }

    #[test]
    fn jour_demande_signale_les_valeurs_invalides() {
        let auj = d("2026-11-03");
        assert_eq!(jour_demande(None, auj), (auj, false));
        assert_eq!(
            jour_demande(Some("2026-11-05"), auj),
            (d("2026-11-05"), false)
        );
        assert_eq!(jour_demande(Some("demain"), auj), (auj, true));
        assert_eq!(jour_demande(Some("2026-13-45"), auj), (auj, true));
        assert_eq!(jour_demande(Some(""), auj), (auj, true));
        assert_eq!(jour_demande(Some("9999-01-01"), auj), (auj, true));
    }
}
