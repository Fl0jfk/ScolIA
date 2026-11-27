//! Programme d'une sortie (étapes : type, date-heure, durée, lieu, note, appel requis) : lignes de
//! saisie, contrôle de confort, ordre chronologique. **Aucune règle métier** : les types admis, les
//! limites et l'effet d'une modification (liste confirmée rouverte, journal) sont décidés par le serveur.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ent_types::voyages::{EtapeDto, EtapeSaisieDto};

use super::modele::{erreur, opt, Erreurs};
use crate::noyau::temps::{a_paris, paris_vers_utc};

/// Types d'étape proposés (le serveur valide la liste).
pub const TYPES_ETAPE: [(&str, &str); 8] = [
    ("depart", "Départ"),
    ("depot", "Dépôt des élèves"),
    ("reprise", "Reprise des élèves"),
    ("visite", "Visite ou activité"),
    ("hebergement", "Hébergement"),
    ("retour", "Retour"),
    ("reunion", "Réunion"),
    ("autre", "Autre"),
];

pub fn libelle_type_etape(code: &str) -> &'static str {
    TYPES_ETAPE
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| *l)
        .unwrap_or("Étape")
}

/// Durée lisible : « 45 min », « 2 h », « 1 h 30 ».
pub fn duree_lisible(minutes: i32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m:02}"),
    }
}

/// Une ligne du programme, telle que saisie (chaînes). `cle` est stable pendant l'édition
/// (clé du `<For>` et suffixe des identifiants de champs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EtapeEdition {
    pub cle: u32,
    pub type_etape: String,
    pub date: String,
    pub heure: String,
    pub duree: String,
    pub lieu: String,
    pub note: String,
    pub appel_requis: bool,
}

/// Identifiant du champ `champ` (« date », « heure », « duree », « lieu », « note ») de la ligne `cle`.
pub fn id_champ(cle: u32, champ: &str) -> String {
    format!("voy-pg-{cle}-{champ}")
}

/// Ligne de saisie d'une étape existante (heure de Paris). L'écran ne propose pas d'heure de fin :
/// une fin connue est reprise comme durée.
pub fn edition_depuis(e: &EtapeDto, cle: u32) -> EtapeEdition {
    let debut = a_paris(e.debut);
    let duree = e
        .duree_minutes
        .or_else(|| {
            e.fin
                .map(|f| (f - e.debut).num_minutes())
                .filter(|m| *m > 0)
                .and_then(|m| i32::try_from(m).ok())
        })
        .map(|m| m.to_string())
        .unwrap_or_default();
    EtapeEdition {
        cle,
        type_etape: e.type_etape.clone(),
        date: debut.format("%Y-%m-%d").to_string(),
        heure: debut.format("%H:%M").to_string(),
        duree,
        lieu: e.lieu.clone().unwrap_or_default(),
        note: e.note.clone().unwrap_or_default(),
        appel_requis: e.appel_requis,
    }
}

/// Nouvelle ligne : même jour et même heure que la dernière ligne, sinon le jour du départ de la sortie.
pub fn etape_par_defaut(
    cle: u32,
    lignes: &[EtapeEdition],
    debut_sortie: DateTime<Utc>,
) -> EtapeEdition {
    let (date, heure) = match lignes.last() {
        Some(l) => (l.date.clone(), l.heure.clone()),
        None => {
            let d = a_paris(debut_sortie);
            (
                d.format("%Y-%m-%d").to_string(),
                d.format("%H:%M").to_string(),
            )
        }
    };
    EtapeEdition {
        cle,
        type_etape: "visite".into(),
        date,
        heure,
        ..Default::default()
    }
}

fn instant(l: &EtapeEdition) -> Option<chrono::NaiveDateTime> {
    let d = NaiveDate::parse_from_str(l.date.trim(), "%Y-%m-%d").ok()?;
    let h = NaiveTime::parse_from_str(l.heure.trim(), "%H:%M").ok()?;
    Some(d.and_time(h))
}

/// Clés des lignes dans l'ordre chronologique (tri stable ; une ligne dont la date ou l'heure est
/// illisible passe après les autres, dans son ordre d'origine).
pub fn ordre_chronologique(lignes: &[EtapeEdition]) -> Vec<u32> {
    let mut v: Vec<(Option<chrono::NaiveDateTime>, u32)> =
        lignes.iter().map(|l| (instant(l), l.cle)).collect();
    v.sort_by(|a, b| match (a.0, b.0) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    v.into_iter().map(|(_, c)| c).collect()
}

/// Corps de `PUT /voyages/{id}/programme` : contrôle de confort puis lignes triées par date.
/// Les erreurs portent l'identifiant du champ (`id_champ`) pour le résumé d'erreurs.
pub fn composer_programme(lignes: &[EtapeEdition]) -> Result<Vec<EtapeSaisieDto>, Erreurs> {
    let mut e: Erreurs = Vec::new();
    let mut etapes: Vec<(chrono::NaiveDateTime, EtapeSaisieDto)> = Vec::new();
    for l in lignes {
        if !TYPES_ETAPE.iter().any(|(c, _)| *c == l.type_etape) {
            erreur(
                &mut e,
                &id_champ(l.cle, "type"),
                "Choisissez le type d'étape.",
            );
        }
        let debut = instant(l);
        if NaiveDate::parse_from_str(l.date.trim(), "%Y-%m-%d").is_err() {
            erreur(
                &mut e,
                &id_champ(l.cle, "date"),
                "Indiquez la date de l'étape.",
            );
        }
        if NaiveTime::parse_from_str(l.heure.trim(), "%H:%M").is_err() {
            erreur(
                &mut e,
                &id_champ(l.cle, "heure"),
                "Indiquez l'heure de l'étape.",
            );
        }
        let duree = l.duree.trim();
        let duree_minutes = if duree.is_empty() {
            None
        } else {
            match duree
                .parse::<i32>()
                .ok()
                .filter(|m| (1..=10_080).contains(m))
            {
                Some(m) => Some(m),
                None => {
                    erreur(
                        &mut e,
                        &id_champ(l.cle, "duree"),
                        "Durée : un nombre de minutes entre 1 et 10 080.",
                    );
                    None
                }
            }
        };
        if l.lieu.trim().chars().count() > 300 {
            erreur(
                &mut e,
                &id_champ(l.cle, "lieu"),
                "Lieu : 300 caractères au plus.",
            );
        }
        if l.note.trim().chars().count() > 1000 {
            erreur(
                &mut e,
                &id_champ(l.cle, "note"),
                "Note : 1 000 caractères au plus.",
            );
        }
        if let Some(d) = debut {
            etapes.push((
                d,
                EtapeSaisieDto {
                    type_etape: l.type_etape.clone(),
                    debut: paris_vers_utc(d),
                    fin: None,
                    duree_minutes,
                    lieu: opt(&l.lieu),
                    note: opt(&l.note),
                    appel_requis: l.appel_requis,
                },
            ));
        }
    }
    if !e.is_empty() {
        return Err(e);
    }
    etapes.sort_by_key(|(d, _)| *d);
    Ok(etapes.into_iter().map(|(_, s)| s).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn ligne(cle: u32, date: &str, heure: &str) -> EtapeEdition {
        EtapeEdition {
            cle,
            type_etape: "visite".into(),
            date: date.into(),
            heure: heure.into(),
            ..Default::default()
        }
    }

    #[test]
    fn toutes_les_etapes_ont_un_libelle() {
        for (c, _) in TYPES_ETAPE {
            assert_ne!(libelle_type_etape(c), "Étape", "{c}");
        }
        assert_eq!(libelle_type_etape("inconnu"), "Étape");
    }

    #[test]
    fn composition_en_utc_et_triee_par_date() {
        let mut a = ligne(1, "2027-03-11", "09:30");
        a.duree = "90".into();
        a.lieu = " Forum ".into();
        a.appel_requis = true;
        let b = ligne(2, "2027-03-10", "07:00");
        let r = composer_programme(&[a, b]).expect("valide");
        assert_eq!(r.len(), 2);
        // 07:00 à Paris le 10 mars 2027 (hiver) = 06:00 UTC, et passe en premier.
        assert_eq!(
            r[0].debut,
            Utc.with_ymd_and_hms(2027, 3, 10, 6, 0, 0).unwrap()
        );
        assert_eq!(r[1].duree_minutes, Some(90));
        assert_eq!(r[1].lieu.as_deref(), Some("Forum"));
        assert!(r[1].appel_requis);
        assert_eq!(r[1].fin, None);
    }

    #[test]
    fn durees_lisibles() {
        assert_eq!(duree_lisible(45), "45 min");
        assert_eq!(duree_lisible(120), "2 h");
        assert_eq!(duree_lisible(90), "1 h 30");
        assert_eq!(duree_lisible(65), "1 h 05");
    }

    #[test]
    fn un_programme_vide_est_permis() {
        assert_eq!(composer_programme(&[]), Ok(Vec::new()));
    }

    #[test]
    fn composition_refuse_ligne_par_ligne() {
        let mut a = ligne(4, "2027-13-40", "25:00");
        a.duree = "0".into();
        a.type_etape = String::new();
        a.lieu = "x".repeat(301);
        let champs: Vec<String> = composer_programme(&[ligne(3, "2027-03-10", "07:00"), a])
            .unwrap_err()
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        for attendu in ["type", "date", "heure", "duree", "lieu"] {
            assert!(
                champs.contains(&id_champ(4, attendu)),
                "{attendu} manquant : {champs:?}"
            );
        }
        assert!(!champs.iter().any(|c| c.starts_with("voy-pg-3-")));
    }

    #[test]
    fn ordre_chronologique_stable_et_lignes_illisibles_a_la_fin() {
        let l = vec![
            ligne(1, "2027-03-11", "09:00"),
            ligne(2, "", ""),
            ligne(3, "2027-03-10", "18:00"),
            ligne(4, "2027-03-10", "18:00"),
        ];
        assert_eq!(ordre_chronologique(&l), vec![3, 4, 1, 2]);
    }

    #[test]
    fn etape_existante_vers_saisie_en_heure_de_paris() {
        let debut = Utc.with_ymd_and_hms(2027, 7, 1, 6, 0, 0).unwrap();
        let e = EtapeDto {
            id: Uuid::nil(),
            ordre: 1,
            type_etape: "depot".into(),
            debut,
            fin: Some(debut + chrono::Duration::minutes(45)),
            duree_minutes: None,
            lieu: Some("Parvis".into()),
            note: None,
            appel_requis: true,
        };
        let l = edition_depuis(&e, 9);
        // Été : UTC+2.
        assert_eq!((l.date.as_str(), l.heure.as_str()), ("2027-07-01", "08:00"));
        assert_eq!(l.duree, "45");
        assert_eq!(l.cle, 9);
        assert!(l.appel_requis);
    }

    #[test]
    fn nouvelle_etape_reprend_le_dernier_jour_sinon_le_depart() {
        let depart = Utc.with_ymd_and_hms(2027, 3, 10, 6, 0, 0).unwrap();
        let premiere = etape_par_defaut(1, &[], depart);
        assert_eq!(
            (premiere.date.as_str(), premiere.heure.as_str()),
            ("2027-03-10", "07:00")
        );
        let suivante = etape_par_defaut(2, &[ligne(1, "2027-03-12", "15:00")], depart);
        assert_eq!(
            (suivante.date.as_str(), suivante.heure.as_str()),
            ("2027-03-12", "15:00")
        );
        assert_eq!(suivante.type_etape, "visite");
    }
}
