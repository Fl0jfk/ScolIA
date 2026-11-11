//! Logique d'écran de « Mon agenda » (pure, sans Leptos ni réseau, testée en natif) : navigation par
//! semaine, regroupement des éléments par jour, libellés, composition d'un événement et d'une demande
//! de publication. Le serveur reste l'autorité : droits, audience sous les droits de l'auteur (R-AG8),
//! minimisation selon le lecteur (R-AG5) et fuseau de l'établissement (les jours et heures arrivent
//! déjà calculés).

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use ent_types::agenda::{
    ApercuPublication, AudienceDto, ElementAgenda, NouvelEvenement, SourceIndisponible, VueAgenda,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::noyau::temps::{date_longue, nom_mois};

/// Années acceptées pour la navigation (aucune panique sur une date extrême).
pub const ANNEES_ACCEPTEES: std::ops::RangeInclusive<i32> = 2000..=2100;

/// Sources affichables : (code, libellé du filtre).
pub const SOURCES: [(&str, &str); 4] = [
    ("evenements", "Événements et réunions"),
    ("rendez_vous", "Rendez-vous"),
    ("edt", "Cours"),
    ("calendrier", "Fermetures"),
];

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

/// Dimanche de la semaine qui commence le `lundi`.
pub fn dimanche_de(lundi: NaiveDate) -> NaiveDate {
    lundi.checked_add_signed(Duration::days(6)).unwrap_or(lundi)
}

fn quantieme(d: NaiveDate) -> String {
    if d.day() == 1 {
        "1er".into()
    } else {
        d.day().to_string()
    }
}

/// « Semaine du 2 au 8 novembre », « Semaine du 28 décembre au 3 janvier ».
pub fn libelle_semaine(lundi: NaiveDate) -> String {
    let fin = dimanche_de(lundi);
    if lundi.month() == fin.month() {
        format!(
            "Semaine du {} au {} {}",
            quantieme(lundi),
            quantieme(fin),
            nom_mois(fin.month())
        )
    } else {
        format!(
            "Semaine du {} {} au {} {}",
            quantieme(lundi),
            nom_mois(lundi.month()),
            quantieme(fin),
            nom_mois(fin.month())
        )
    }
}

pub fn libelle_source(code: &str) -> &'static str {
    match code {
        "evenements" => "Événement",
        "rendez_vous" => "Rendez-vous",
        "edt" => "Cours",
        "calendrier" => "Fermeture",
        "sorties" => "Sortie",
        _ => "Autre",
    }
}

pub fn libelle_reponse(r: &str) -> &'static str {
    match r {
        "oui" => "Je participe",
        "non" => "Je ne participe pas",
        "peut_etre" => "Peut-être",
        "en_attente" => "Réponse attendue",
        _ => "",
    }
}

pub fn ton_reponse(r: &str) -> TonEtat {
    match r {
        "oui" => TonEtat::Ok,
        "non" => TonEtat::Mal,
        "peut_etre" => TonEtat::Info,
        _ => TonEtat::Avertissement,
    }
}

pub fn libelle_statut(s: &str) -> Option<&'static str> {
    match s {
        "brouillon" => Some("Brouillon"),
        "annule" | "annulee" => Some("Annulé"),
        "clos" => Some("Clos"),
        "remplacee" => Some("Remplacé"),
        _ => None,
    }
}

pub fn ton_statut(s: &str) -> TonEtat {
    match s {
        "annule" | "annulee" => TonEtat::Mal,
        "brouillon" | "remplacee" => TonEtat::Avertissement,
        _ => TonEtat::Neutre,
    }
}

/// L'élément n'a pas lieu (barré à l'écran).
pub fn est_annule(e: &ElementAgenda) -> bool {
    matches!(e.statut.as_str(), "annule" | "annulee")
}

/// « 17:00–18:00 », « Journée entière », « Jusqu'au 12 novembre ».
pub fn horaire(e: &ElementAgenda, jour: NaiveDate) -> String {
    let h = |t: NaiveTime| t.format("%H:%M").to_string();
    match (e.heure_debut, e.heure_fin) {
        (Some(d), Some(f)) if e.jour == e.jour_fin => format!("{}–{}", h(d), h(f)),
        (Some(d), _) if jour == e.jour => format!("dès {}", h(d)),
        (_, Some(f)) if jour == e.jour_fin => format!("jusqu'à {}", h(f)),
        _ if e.jour_fin > e.jour => {
            format!("{} au {}", date_courte(e.jour), date_courte(e.jour_fin))
        }
        _ => "Journée entière".to_string(),
    }
}

fn date_courte(d: NaiveDate) -> String {
    format!("{} {}", quantieme(d), nom_mois(d.month()))
}

#[derive(Debug, Clone, PartialEq)]
pub struct JourAgenda {
    pub jour: NaiveDate,
    pub titre: String,
    pub elements: Vec<ElementAgenda>,
}

/// Les sept jours de la semaine, chacun avec ses éléments (un événement sur plusieurs jours figure
/// sur chacun), journées entières d'abord puis par heure, à heure égale par titre.
pub fn regrouper_par_jour(vue: &VueAgenda, lundi: NaiveDate) -> Vec<JourAgenda> {
    (0..7)
        .filter_map(|i| lundi.checked_add_signed(Duration::days(i)))
        .map(|jour| {
            let mut elements: Vec<ElementAgenda> = vue
                .elements
                .iter()
                .filter(|e| e.jour <= jour && jour <= e.jour_fin)
                .cloned()
                .collect();
            elements.sort_by(|a, b| (a.heure_debut, &a.titre).cmp(&(b.heure_debut, &b.titre)));
            JourAgenda {
                jour,
                titre: date_longue(jour),
                elements,
            }
        })
        .collect()
}

/// Phrase annonçant les sources absentes (jamais un silence, R-AG1).
pub fn libelle_sources_indisponibles(s: &[SourceIndisponible]) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    let liste: Vec<String> = s
        .iter()
        .map(|x| {
            format!(
                "{} ({})",
                libelle_source(&x.source).to_lowercase(),
                x.raison
            )
        })
        .collect();
    Some(format!("Sources non affichées : {}.", liste.join(" ; ")))
}

/// Une personne peut-elle répondre à l'invitation ? (le serveur contrôle)
pub fn peut_repondre(statut: &str, ma_reponse: Option<&str>) -> bool {
    statut == "publie" && ma_reponse.is_some()
}

/// Champs saisis (texte brut des champs du formulaire).
pub struct SaisieEvenement<'a> {
    pub type_code: &'a str,
    pub titre: &'a str,
    pub jour: &'a str,
    pub jour_fin: &'a str,
    pub heure_debut: &'a str,
    pub heure_fin: &'a str,
    pub lieu: &'a str,
    pub confidentialite: &'a str,
}

pub fn composer_evenement(s: &SaisieEvenement<'_>) -> Result<NouvelEvenement, String> {
    let SaisieEvenement {
        type_code,
        titre,
        jour,
        jour_fin,
        heure_debut,
        heure_fin,
        lieu,
        confidentialite,
    } = *s;
    let titre = titre.trim();
    if titre.is_empty() || titre.chars().count() > 200 {
        return Err("Le titre est obligatoire (200 caractères au plus).".into());
    }
    if type_code.trim().is_empty() {
        return Err("Choisissez un type.".into());
    }
    let jour = NaiveDate::parse_from_str(jour.trim(), "%Y-%m-%d")
        .map_err(|_| "Jour invalide.".to_string())?;
    let jour_fin = match jour_fin.trim() {
        "" => None,
        j => Some(
            NaiveDate::parse_from_str(j, "%Y-%m-%d")
                .map_err(|_| "Dernier jour invalide.".to_string())?,
        ),
    };
    if jour_fin.is_some_and(|f| f < jour) {
        return Err("Le dernier jour précède le premier.".into());
    }
    let heure = |t: &str, nom: &str| -> Result<Option<NaiveTime>, String> {
        match t.trim() {
            "" => Ok(None),
            h => NaiveTime::parse_from_str(h, "%H:%M")
                .map(Some)
                .map_err(|_| format!("{nom} invalide.")),
        }
    };
    let (hd, hf) = (
        heure(heure_debut, "Heure de début")?,
        heure(heure_fin, "Heure de fin")?,
    );
    if hd.is_some() != hf.is_some() {
        return Err("Renseignez les deux heures, ou aucune pour une journée entière.".into());
    }
    if let (Some(d), Some(f)) = (hd, hf) {
        if jour_fin.is_none_or(|j| j == jour) && f <= d {
            return Err("L'heure de fin doit suivre l'heure de début.".into());
        }
    }
    let lieu = lieu.trim();
    if lieu.chars().count() > 200 {
        return Err("Le lieu est trop long (200 caractères au plus).".into());
    }
    Ok(NouvelEvenement {
        type_code: type_code.trim().to_string(),
        titre: titre.to_string(),
        description: None,
        jour,
        jour_fin,
        heure_debut: hd,
        heure_fin: hf,
        salle_id: None,
        lieu_texte: (!lieu.is_empty()).then(|| lieu.to_string()),
        confidentialite: Some(confidentialite.to_string()).filter(|c| !c.is_empty()),
        invites: vec![],
        coupon: None,
    })
}

/// Audiences possibles à la publication : (code, libellé). `classe` demande une classe.
pub const AUDIENCES: [(&str, &str); 4] = [
    (
        "aucune",
        "Personne d'autre (brouillon personnel ou invités)",
    ),
    ("classe", "Les familles d'une classe"),
    ("etablissement", "Toutes les familles de l'établissement"),
    ("tous_personnels", "Tous les personnels"),
];

pub fn composer_audience(code: &str, classe_id: &str) -> Result<Vec<AudienceDto>, String> {
    match code {
        "aucune" | "" => Ok(vec![]),
        "classe" => {
            let id = Uuid::parse_str(classe_id.trim())
                .map_err(|_| "Choisissez une classe.".to_string())?;
            Ok(vec![AudienceDto {
                kind: "classe".into(),
                cible_id: Some(id),
            }])
        }
        "etablissement" | "tous_personnels" => Ok(vec![AudienceDto {
            kind: code.into(),
            cible_id: None,
        }]),
        _ => Err("Audience inconnue.".into()),
    }
}

/// « 24 familles (31 responsables) seront prévenues. 2 élèves exclus faute de droits. »
pub fn resume_apercu(a: &ApercuPublication) -> String {
    let mut p = Vec::new();
    if a.nb_responsables > 0 || a.nb_familles > 0 {
        p.push(format!(
            "{} élèves concernés, {} responsables seront prévenus.",
            a.nb_familles, a.nb_responsables
        ));
    }
    if a.nb_personnels > 0 {
        p.push(format!(
            "{} membres du personnel seront prévenus.",
            a.nb_personnels
        ));
    }
    if p.is_empty() {
        p.push("Aucune personne à prévenir : l'événement ne sera visible que de vous et de vos invités.".into());
    }
    if a.exclus_hors_droits > 0 {
        p.push(format!(
            "{} élèves sont exclus : vous n'avez pas le droit de leur écrire.",
            a.exclus_hors_droits
        ));
    }
    p.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn el(titre: &str, jour: NaiveDate, fin: NaiveDate, h: Option<(u32, u32)>) -> ElementAgenda {
        let t = |(a, b): (u32, u32)| NaiveTime::from_hms_opt(a, b, 0).unwrap();
        ElementAgenda {
            source: "evenements".into(),
            id: Some(Uuid::nil()),
            titre: titre.into(),
            debut: Utc.with_ymd_and_hms(2026, 11, 10, 9, 0, 0).unwrap(),
            fin: Utc.with_ymd_and_hms(2026, 11, 10, 10, 0, 0).unwrap(),
            jour,
            jour_fin: fin,
            heure_debut: h.map(t),
            heure_fin: h.map(|(a, b)| t((a + 1, b))),
            journee_entiere: h.is_none(),
            lieu: None,
            statut: "publie".into(),
            nature: "reunion".into(),
            role: Some("invite".into()),
            reponse: None,
            eleve_id: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn saisie<'a>(
        t: &'a str,
        ti: &'a str,
        j: &'a str,
        jf: &'a str,
        hd: &'a str,
        hf: &'a str,
        l: &'a str,
        c: &'a str,
    ) -> SaisieEvenement<'a> {
        SaisieEvenement {
            type_code: t,
            titre: ti,
            jour: j,
            jour_fin: jf,
            heure_debut: hd,
            heure_fin: hf,
            lieu: l,
            confidentialite: c,
        }
    }

    #[test]
    fn navigation_par_semaine_sans_panique_aux_extremes() {
        let lundi = lundi_de(d(2026, 11, 11));
        assert_eq!(lundi, d(2026, 11, 9));
        assert_eq!(dimanche_de(lundi), d(2026, 11, 15));
        assert_eq!(semaine_voisine(lundi, 1), Some(d(2026, 11, 16)));
        assert_eq!(semaine_voisine(d(2100, 12, 28), 5), None);
        assert_eq!(semaine_voisine(d(2000, 1, 3), -5), None);
        assert_eq!(libelle_semaine(lundi), "Semaine du 9 au 15 novembre");
        assert_eq!(
            libelle_semaine(d(2026, 12, 28)),
            "Semaine du 28 décembre au 3 janvier"
        );
        assert_eq!(
            libelle_semaine(d(2026, 6, 29)),
            "Semaine du 29 juin au 5 juillet"
        );
    }

    #[test]
    fn regroupement_par_jour_trie_journees_entieres_d_abord_et_etale_les_evenements_longs() {
        let lundi = d(2026, 11, 9);
        let vue = VueAgenda {
            du: lundi,
            au: dimanche_de(lundi),
            elements: vec![
                el("Réunion", d(2026, 11, 10), d(2026, 11, 10), Some((17, 0))),
                el("Cours", d(2026, 11, 10), d(2026, 11, 10), Some((8, 0))),
                el("Voyage", d(2026, 11, 10), d(2026, 11, 12), None),
            ],
            sources_indisponibles: vec![],
        };
        let j = regrouper_par_jour(&vue, lundi);
        assert_eq!(j.len(), 7);
        let titres = |i: usize| {
            j[i].elements
                .iter()
                .map(|e| e.titre.as_str())
                .collect::<Vec<_>>()
        };
        assert!(titres(0).is_empty());
        assert_eq!(titres(1), ["Voyage", "Cours", "Réunion"]);
        assert_eq!(
            titres(3),
            ["Voyage"],
            "l'événement long figure sur chaque jour"
        );
        assert!(titres(4).is_empty());
    }

    #[test]
    fn horaires_lisibles() {
        let e = el("R", d(2026, 11, 10), d(2026, 11, 10), Some((17, 0)));
        assert_eq!(horaire(&e, e.jour), "17:00–18:00");
        let v = el("V", d(2026, 11, 10), d(2026, 11, 12), None);
        assert_eq!(horaire(&v, d(2026, 11, 11)), "10 novembre au 12 novembre");
        let j = el("J", d(2026, 11, 10), d(2026, 11, 10), None);
        assert_eq!(horaire(&j, j.jour), "Journée entière");
    }

    #[test]
    fn sources_indisponibles_dites() {
        assert_eq!(libelle_sources_indisponibles(&[]), None);
        let m = libelle_sources_indisponibles(&[SourceIndisponible {
            source: "sorties".into(),
            raison: "pas encore branchée".into(),
        }])
        .unwrap();
        assert!(
            m.contains("sortie") && m.contains("pas encore branchée"),
            "{m}"
        );
    }

    #[test]
    fn composition_d_un_evenement_valide_et_refus() {
        let ok = composer_evenement(&saisie(
            "reunion",
            " Conseil ",
            "2026-11-12",
            "",
            "17:00",
            "18:30",
            " Salle 3 ",
            "participants",
        ))
        .unwrap();
        assert_eq!(
            (ok.titre.as_str(), ok.lieu_texte.as_deref(), ok.jour_fin),
            ("Conseil", Some("Salle 3"), None)
        );
        let entiere = composer_evenement(&saisie(
            "evenement",
            "Sortie",
            "2026-11-12",
            "2026-11-13",
            "",
            "",
            "",
            "",
        ))
        .unwrap();
        assert!(entiere.heure_debut.is_none() && entiere.confidentialite.is_none());
        for (t, j, jf, hd, hf) in [
            ("", "2026-11-12", "", "", ""),
            ("x", "pas-une-date", "", "", ""),
            ("x", "2026-11-12", "2026-11-11", "", ""),
            ("x", "2026-11-12", "", "17:00", ""),
            ("x", "2026-11-12", "", "18:00", "17:00"),
            ("x", "2026-11-12", "", "25:00", "26:00"),
        ] {
            assert!(
                composer_evenement(&saisie("reunion", t, j, jf, hd, hf, "", "")).is_err(),
                "{t}/{j}/{jf}/{hd}/{hf}"
            );
        }
        assert!(composer_evenement(&saisie("", "x", "2026-11-12", "", "", "", "", "")).is_err());
        assert!(composer_evenement(&saisie(
            "reunion",
            &"é".repeat(201),
            "2026-11-12",
            "",
            "",
            "",
            "",
            ""
        ))
        .is_err());
        // Plusieurs jours : une heure de fin avant celle de début est permise.
        assert!(composer_evenement(&saisie(
            "reunion",
            "Stage",
            "2026-11-12",
            "2026-11-13",
            "18:00",
            "09:00",
            "",
            ""
        ))
        .is_ok());
    }

    #[test]
    fn audience_et_apercu() {
        assert_eq!(composer_audience("aucune", "").unwrap(), vec![]);
        assert!(composer_audience("classe", "").is_err());
        let id = Uuid::from_u128(7);
        assert_eq!(
            composer_audience("classe", &id.to_string()).unwrap()[0].cible_id,
            Some(id)
        );
        assert_eq!(
            composer_audience("etablissement", "").unwrap()[0].kind,
            "etablissement"
        );
        assert!(composer_audience("tout-le-monde", "").is_err());
        let a = ApercuPublication {
            nb_familles: 24,
            nb_responsables: 31,
            nb_personnels: 0,
            exclus_hors_droits: 2,
            empreinte: "x".into(),
        };
        let r = resume_apercu(&a);
        assert!(
            r.contains("24 élèves")
                && r.contains("31 responsables")
                && r.contains("2 élèves sont exclus"),
            "{r}"
        );
        let vide = ApercuPublication {
            nb_familles: 0,
            nb_responsables: 0,
            nb_personnels: 0,
            exclus_hors_droits: 0,
            empreinte: "x".into(),
        };
        assert!(resume_apercu(&vide).contains("Aucune personne à prévenir"));
    }

    #[test]
    fn reponse_possible_seulement_sur_un_evenement_publie_ou_je_suis_invite() {
        assert!(peut_repondre("publie", Some("en_attente")));
        assert!(!peut_repondre("annule", Some("oui")));
        assert!(!peut_repondre("publie", None));
        assert_eq!(libelle_statut("annule"), Some("Annulé"));
        assert_eq!(libelle_statut("publie"), None);
        assert!(!est_annule(&el("x", d(2026, 1, 1), d(2026, 1, 1), None)));
    }
}
