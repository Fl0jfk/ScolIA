//! Logique de l'écran « EDT de la semaine » (pure, sans Leptos ni réseau, testée en natif) :
//! choix de la cible, navigation par semaine, regroupement des séances par jour et libellés de
//! statut. Le serveur reste l'autorité (droits, statuts, jours fermés).
//!
//! Les fonctions de semaine répètent volontairement celles du module `edt` : un module ne
//! dépend jamais d'un autre module.

use std::collections::BTreeSet;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use ent_types::edt_s2::{SeanceSemaineDto, SemaineEdtDto};
use ent_web_api::CibleSemaine;
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::noyau::temps::{date_longue, nom_mois};

/// Années acceptées pour la navigation (aucune panique sur une date extrême).
pub const ANNEES_ACCEPTEES: std::ops::RangeInclusive<i32> = 2000..=2100;

/// Lundi de la semaine de `jour`.
pub fn lundi_de(jour: NaiveDate) -> NaiveDate {
    jour.checked_sub_signed(Duration::days(i64::from(
        jour.weekday().num_days_from_monday(),
    )))
    .unwrap_or(jour)
}

/// Lundi de la semaine décalée de `decalage` semaines ; `None` hors de [`ANNEES_ACCEPTEES`].
pub fn semaine_voisine(lundi: NaiveDate, decalage: i64) -> Option<NaiveDate> {
    let d = lundi.checked_add_signed(Duration::try_weeks(decalage)?)?;
    ANNEES_ACCEPTEES.contains(&d.year()).then_some(d)
}

fn quantieme(d: NaiveDate) -> String {
    if d.day() == 1 {
        "1er".into()
    } else {
        d.day().to_string()
    }
}

/// « Semaine du 2 au 6 novembre », « Semaine du 28 décembre au 1er janvier ».
pub fn libelle_semaine(lundi: NaiveDate) -> String {
    let vendredi = lundi.checked_add_signed(Duration::days(4)).unwrap_or(lundi);
    if lundi.month() == vendredi.month() {
        format!(
            "Semaine du {} au {} {}",
            quantieme(lundi),
            quantieme(vendredi),
            nom_mois(vendredi.month())
        )
    } else {
        format!(
            "Semaine du {} {} au {} {}",
            quantieme(lundi),
            nom_mois(lundi.month()),
            quantieme(vendredi),
            nom_mois(vendredi.month())
        )
    }
}

/// « Semaine A » / « Semaine B » ; rien si le calendrier n'a pas d'alternance.
pub fn libelle_semaine_ab(semaine_ab: Option<&str>) -> Option<String> {
    match semaine_ab.map(|s| s.trim().to_ascii_uppercase()).as_deref() {
        Some("A") => Some("Semaine A".into()),
        Some("B") => Some("Semaine B".into()),
        _ => None,
    }
}

// ───────────────────────────── Cible ─────────────────────────────

/// Ce que le personnel peut afficher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vue {
    Moi,
    Classe,
    Salle,
    Personnel,
}

impl Vue {
    pub const TOUTES: [Vue; 4] = [Vue::Moi, Vue::Classe, Vue::Salle, Vue::Personnel];

    pub fn code(self) -> &'static str {
        match self {
            Vue::Moi => "moi",
            Vue::Classe => "classe",
            Vue::Salle => "salle",
            Vue::Personnel => "personnel",
        }
    }

    pub fn depuis_code(code: &str) -> Vue {
        Vue::TOUTES
            .into_iter()
            .find(|v| v.code() == code)
            .unwrap_or(Vue::Moi)
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Vue::Moi => "Mon emploi du temps",
            Vue::Classe => "Une classe",
            Vue::Salle => "Une salle",
            Vue::Personnel => "Un membre du personnel",
        }
    }

    /// Étiquette du choix à faire (`None` : aucun choix, c'est soi-même).
    pub fn libelle_choix(self) -> Option<&'static str> {
        match self {
            Vue::Moi => None,
            Vue::Classe => Some("Classe"),
            Vue::Salle => Some("Salle"),
            Vue::Personnel => Some("Personne"),
        }
    }

    /// Nom de l'enseignant utile à l'affichage (inutile sur son propre emploi du temps).
    pub fn montre_le_professeur(self) -> bool {
        !matches!(self, Vue::Moi | Vue::Personnel)
    }
}

/// Cible de la requête ; `None` tant que le choix demandé n'est pas fait ou est illisible.
pub fn cible_depuis(vue: Vue, choix: &str) -> Option<CibleSemaine> {
    let choix = choix.trim();
    match vue {
        Vue::Moi => Some(CibleSemaine::Moi),
        Vue::Classe => choix.parse::<Uuid>().ok().map(CibleSemaine::Classe),
        Vue::Personnel => choix.parse::<Uuid>().ok().map(CibleSemaine::Personnel),
        Vue::Salle => (!choix.is_empty()).then(|| CibleSemaine::Salle(choix.to_string())),
    }
}

// ───────────────────────────── Séances ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatutSeance {
    Prevue,
    Annulee,
    Remplacee,
    /// Cours d'origine d'un déplacement.
    Deplacee,
    /// Cours déplacé, à sa nouvelle place.
    Ajoutee,
    Inconnu,
}

impl StatutSeance {
    pub fn depuis(texte: &str) -> StatutSeance {
        match texte {
            "prevue" => StatutSeance::Prevue,
            "annulee" => StatutSeance::Annulee,
            "remplacee" => StatutSeance::Remplacee,
            "deplacee" => StatutSeance::Deplacee,
            "ajoutee" => StatutSeance::Ajoutee,
            _ => StatutSeance::Inconnu,
        }
    }

    /// Le cours n'a pas lieu à cette place (annulé ou déplacé ailleurs).
    pub fn n_a_pas_lieu(self) -> bool {
        matches!(self, StatutSeance::Annulee | StatutSeance::Deplacee)
    }

    pub fn ton(self) -> TonEtat {
        match self {
            StatutSeance::Annulee => TonEtat::Mal,
            StatutSeance::Remplacee | StatutSeance::Deplacee | StatutSeance::Ajoutee => {
                TonEtat::Avertissement
            }
            StatutSeance::Prevue | StatutSeance::Inconnu => TonEtat::Neutre,
        }
    }
}

/// Date dans une phrase : « lundi 9 novembre ».
fn date_phrase(d: NaiveDate) -> String {
    date_longue(d).to_lowercase()
}

/// « 08:00 – 09:00 ».
pub fn horaire(debut: NaiveTime, fin: NaiveTime) -> String {
    format!("{} – {}", debut.format("%H:%M"), fin.format("%H:%M"))
}

/// Mention de statut en toutes lettres (jamais la couleur seule) ; `None` pour un cours normal.
pub fn mention_statut(s: &SeanceSemaineDto) -> Option<String> {
    match StatutSeance::depuis(&s.statut) {
        StatutSeance::Prevue | StatutSeance::Inconnu => None,
        StatutSeance::Annulee => Some("Cours annulé".into()),
        StatutSeance::Remplacee => Some(
            match s
                .remplacant_nom
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
            {
                Some(nom) => format!("Remplacé par {nom}"),
                None => "Cours assuré par un autre enseignant".into(),
            },
        ),
        StatutSeance::Deplacee => Some(match s.deplace_vers {
            Some(vers) => format!("Cours déplacé au {}", date_phrase(vers)),
            None => "Cours déplacé".into(),
        }),
        StatutSeance::Ajoutee => Some(match s.deplace_depuis {
            Some(depuis) => format!("Cours déplacé depuis le {}", date_phrase(depuis)),
            None => "Cours déplacé ici".into(),
        }),
    }
}

/// Une séance prête à afficher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeanceAffichee {
    /// Clé stable pour `<For>`.
    pub cle: String,
    pub horaire: String,
    pub matiere: String,
    /// « 4B · Salle B12 · Mme Chourave ».
    pub detail: String,
    pub statut: StatutSeance,
    pub mention: Option<String>,
}

fn present(texte: &Option<String>) -> Option<&str> {
    texte.as_deref().map(str::trim).filter(|t| !t.is_empty())
}

fn afficher(s: &SeanceSemaineDto, montre_professeur: bool) -> SeanceAffichee {
    let mut parties = vec![s.classe.trim().to_string()];
    if let Some(salle) = present(&s.salle) {
        parties.push(format!("Salle {salle}"));
    }
    if montre_professeur {
        if let Some(prof) = present(&s.prof_nom) {
            parties.push(prof.to_string());
        }
    }
    parties.retain(|p| !p.is_empty());
    SeanceAffichee {
        cle: format!("{}|{}|{}", s.creneau_id, s.jour, s.statut),
        horaire: horaire(s.heure_debut, s.heure_fin),
        matiere: s.matiere_libelle.clone(),
        detail: parties.join(" · "),
        statut: StatutSeance::depuis(&s.statut),
        mention: mention_statut(s),
    }
}

/// Un jour de la semaine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JourAffiche {
    pub jour: NaiveDate,
    /// « Lundi 9 novembre ».
    pub titre: String,
    /// Libellé de la fermeture (vacances, jour férié…) ; le jour est alors sans cours.
    pub ferme: Option<String>,
    pub seances: Vec<SeanceAffichee>,
}

impl JourAffiche {
    /// Nombre de cours qui ont réellement lieu ce jour-là.
    pub fn nb_cours(&self) -> usize {
        self.seances
            .iter()
            .filter(|s| !s.statut.n_a_pas_lieu())
            .count()
    }
}

/// Regroupe la semaine par jour : lundi à vendredi toujours présents, samedi et dimanche
/// seulement s'ils ont des cours ou une fermeture signalée ; séances triées par heure.
pub fn regrouper_par_jour(semaine: &SemaineEdtDto, montre_professeur: bool) -> Vec<JourAffiche> {
    let lundi = lundi_de(semaine.lundi);
    let dans_la_semaine = |d: NaiveDate| d >= lundi && d <= lundi + Duration::days(6);
    let mut jours: BTreeSet<NaiveDate> = (0..5).map(|i| lundi + Duration::days(i)).collect();
    jours.extend(
        semaine
            .seances
            .iter()
            .map(|s| s.jour)
            .chain(semaine.jours_fermes.iter().map(|j| j.jour))
            .filter(|d| dans_la_semaine(*d)),
    );
    jours
        .into_iter()
        .map(|jour| {
            let mut du_jour: Vec<&SeanceSemaineDto> =
                semaine.seances.iter().filter(|s| s.jour == jour).collect();
            du_jour.sort_by(|a, b| {
                (
                    a.heure_debut,
                    a.heure_fin,
                    &a.classe,
                    &a.matiere_libelle,
                    a.creneau_id,
                )
                    .cmp(&(
                        b.heure_debut,
                        b.heure_fin,
                        &b.classe,
                        &b.matiere_libelle,
                        b.creneau_id,
                    ))
            });
            JourAffiche {
                jour,
                titre: date_longue(jour),
                ferme: semaine
                    .jours_fermes
                    .iter()
                    .find(|f| f.jour == jour)
                    .map(|f| f.libelle.trim().to_string()),
                seances: du_jour
                    .into_iter()
                    .map(|s| afficher(s, montre_professeur))
                    .collect(),
            }
        })
        .collect()
}

/// Nombre de cours à changement (annulé, remplacé, déplacé) de la semaine, pour l'annonce.
pub fn nb_changements(semaine: &SemaineEdtDto) -> usize {
    semaine
        .seances
        .iter()
        .filter(|s| !matches!(StatutSeance::depuis(&s.statut), StatutSeance::Prevue))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_types::edt_s2::JourFermeDto;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn h(heure: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(heure, minute, 0).unwrap()
    }

    fn seance(n: u128, jour: NaiveDate, debut: u32, statut: &str) -> SeanceSemaineDto {
        SeanceSemaineDto {
            creneau_id: Uuid::from_u128(n),
            jour,
            classe: "4B".into(),
            classe_id: None,
            groupe_id: None,
            matiere_libelle: "Maths".into(),
            prof_user_id: Uuid::nil(),
            prof_nom: Some("Mme Chourave".into()),
            remplacant_user_id: None,
            remplacant_nom: None,
            salle: Some("B12".into()),
            heure_debut: h(debut, 0),
            heure_fin: h(debut + 1, 0),
            statut: statut.into(),
            deplace_vers: None,
            deplace_depuis: None,
        }
    }

    fn semaine(seances: Vec<SeanceSemaineDto>) -> SemaineEdtDto {
        SemaineEdtDto {
            lundi: d(2026, 11, 9),
            semaine_ab: Some("A".into()),
            jours_fermes: vec![],
            seances,
        }
    }

    #[test]
    fn navigation_par_semaine() {
        assert_eq!(lundi_de(d(2026, 11, 12)), d(2026, 11, 9));
        assert_eq!(lundi_de(d(2026, 11, 15)), d(2026, 11, 9), "dimanche");
        assert_eq!(semaine_voisine(d(2026, 11, 9), 1), Some(d(2026, 11, 16)));
        assert_eq!(semaine_voisine(d(2026, 11, 9), -1), Some(d(2026, 11, 2)));
        assert_eq!(semaine_voisine(d(2100, 12, 27), 1), None, "borne haute");
        assert_eq!(semaine_voisine(d(2000, 1, 3), -1), None, "borne basse");
        assert_eq!(semaine_voisine(d(2026, 11, 9), i64::MAX), None);
        assert_eq!(
            libelle_semaine(d(2026, 11, 9)),
            "Semaine du 9 au 13 novembre"
        );
        assert_eq!(
            libelle_semaine(d(2026, 12, 28)),
            "Semaine du 28 décembre au 1er janvier"
        );
        // Dates extrêmes : aucune panique.
        let _ = (libelle_semaine(NaiveDate::MAX), lundi_de(NaiveDate::MIN));
    }

    #[test]
    fn semaine_a_b() {
        assert_eq!(libelle_semaine_ab(Some("A")).as_deref(), Some("Semaine A"));
        assert_eq!(
            libelle_semaine_ab(Some(" b ")).as_deref(),
            Some("Semaine B")
        );
        assert_eq!(libelle_semaine_ab(None), None);
        assert_eq!(libelle_semaine_ab(Some("")), None);
        assert_eq!(libelle_semaine_ab(Some("C")), None);
    }

    #[test]
    fn cible_selon_le_choix() {
        let id = Uuid::from_u128(7);
        assert_eq!(cible_depuis(Vue::Moi, ""), Some(CibleSemaine::Moi));
        assert_eq!(cible_depuis(Vue::Classe, ""), None, "choix à faire");
        assert_eq!(cible_depuis(Vue::Classe, "pas un uuid"), None);
        assert_eq!(
            cible_depuis(Vue::Classe, &id.to_string()),
            Some(CibleSemaine::Classe(id))
        );
        assert_eq!(
            cible_depuis(Vue::Personnel, &id.to_string()),
            Some(CibleSemaine::Personnel(id))
        );
        assert_eq!(
            cible_depuis(Vue::Salle, " B12 "),
            Some(CibleSemaine::Salle("B12".into()))
        );
        assert_eq!(cible_depuis(Vue::Salle, "  "), None);
        assert_eq!(Vue::depuis_code("salle"), Vue::Salle);
        assert_eq!(Vue::depuis_code("n'importe quoi"), Vue::Moi);
        assert!(Vue::Moi.libelle_choix().is_none() && Vue::Classe.libelle_choix().is_some());
        assert!(Vue::Classe.montre_le_professeur() && !Vue::Moi.montre_le_professeur());
    }

    #[test]
    fn regroupement_par_jour_et_tri_par_heure() {
        let s = semaine(vec![
            seance(3, d(2026, 11, 10), 14, "prevue"),
            seance(1, d(2026, 11, 9), 10, "prevue"),
            seance(2, d(2026, 11, 9), 8, "prevue"),
        ]);
        let jours = regrouper_par_jour(&s, true);
        assert_eq!(jours.len(), 5, "lundi à vendredi");
        assert_eq!(jours[0].titre, "Lundi 9 novembre");
        let heures: Vec<&str> = jours[0]
            .seances
            .iter()
            .map(|x| x.horaire.as_str())
            .collect();
        assert_eq!(heures, ["08:00 – 09:00", "10:00 – 11:00"]);
        assert_eq!(jours[1].seances.len(), 1);
        assert!(jours[4].seances.is_empty());
        assert_eq!(jours[0].seances[0].detail, "4B · Salle B12 · Mme Chourave");
        // Sur son propre emploi du temps, le nom de l'enseignant est inutile.
        let moi = regrouper_par_jour(&s, false);
        assert_eq!(moi[0].seances[0].detail, "4B · Salle B12");
    }

    #[test]
    fn week_end_seulement_s_il_y_a_des_cours() {
        let s = semaine(vec![seance(1, d(2026, 11, 14), 9, "prevue")]);
        let jours = regrouper_par_jour(&s, true);
        assert_eq!(jours.len(), 6);
        assert_eq!(jours[5].titre, "Samedi 14 novembre");
        // Une séance hors de la semaine annoncée n'ajoute pas de colonne.
        let hors = semaine(vec![seance(1, d(2026, 12, 25), 9, "prevue")]);
        assert_eq!(regrouper_par_jour(&hors, true).len(), 5);
    }

    #[test]
    fn jours_fermes_signales() {
        let mut s = semaine(vec![]);
        s.jours_fermes = vec![
            JourFermeDto {
                jour: d(2026, 11, 11),
                libelle: " Armistice ".into(),
            },
            JourFermeDto {
                jour: d(2026, 11, 15),
                libelle: "Dimanche férié".into(),
            },
        ];
        let jours = regrouper_par_jour(&s, true);
        assert_eq!(jours.len(), 6, "le dimanche fermé est signalé");
        assert_eq!(jours[2].ferme.as_deref(), Some("Armistice"));
        assert!(jours[0].ferme.is_none());
    }

    #[test]
    fn statuts_en_toutes_lettres() {
        let base = seance(1, d(2026, 11, 9), 8, "prevue");
        assert_eq!(mention_statut(&base), None);
        let mut s = base.clone();
        s.statut = "annulee".into();
        assert_eq!(mention_statut(&s).as_deref(), Some("Cours annulé"));
        s.statut = "remplacee".into();
        assert_eq!(
            mention_statut(&s).as_deref(),
            Some("Cours assuré par un autre enseignant")
        );
        s.remplacant_nom = Some("M. Rogue".into());
        assert_eq!(mention_statut(&s).as_deref(), Some("Remplacé par M. Rogue"));
        s.statut = "deplacee".into();
        s.deplace_vers = Some(d(2026, 11, 12));
        assert_eq!(
            mention_statut(&s).as_deref(),
            Some("Cours déplacé au jeudi 12 novembre")
        );
        s.statut = "ajoutee".into();
        s.deplace_depuis = Some(d(2026, 11, 9));
        assert_eq!(
            mention_statut(&s).as_deref(),
            Some("Cours déplacé depuis le lundi 9 novembre")
        );
        s.statut = "autre".into();
        assert_eq!(
            mention_statut(&s),
            None,
            "statut inconnu : pas de mention inventée"
        );
    }

    #[test]
    fn un_cours_annule_ne_compte_pas_et_les_changements_se_comptent() {
        let s = semaine(vec![
            seance(1, d(2026, 11, 9), 8, "prevue"),
            seance(2, d(2026, 11, 9), 9, "annulee"),
            seance(3, d(2026, 11, 9), 10, "deplacee"),
            seance(4, d(2026, 11, 9), 11, "remplacee"),
        ]);
        let jours = regrouper_par_jour(&s, true);
        assert_eq!(jours[0].nb_cours(), 2, "prévue + remplacée");
        assert_eq!(nb_changements(&s), 3);
        assert!(StatutSeance::Annulee.ton() == TonEtat::Mal);
        assert!(StatutSeance::Prevue.ton() == TonEtat::Neutre);
    }

    #[test]
    fn cles_uniques_pour_for() {
        let s = semaine(vec![
            seance(1, d(2026, 11, 9), 8, "deplacee"),
            seance(1, d(2026, 11, 9), 8, "ajoutee"),
        ]);
        let jours = regrouper_par_jour(&s, true);
        assert_ne!(jours[0].seances[0].cle, jours[0].seances[1].cle);
    }
}
