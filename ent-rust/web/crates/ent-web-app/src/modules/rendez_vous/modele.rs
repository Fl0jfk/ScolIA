//! Logique d'écran des rendez-vous et rencontres parents-professeurs (pure, testée en natif).
//!
//! Le serveur reste l'autorité : droits, appartenance de l'enfant à la classe, unicité du
//! rendez-vous par enseignant et par enfant, absence de chevauchement, réservation concurrente.
//! Les créneaux sont stockés en UTC ; l'écran saisit et affiche l'heure de Paris.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use ent_types::communication::{
    CreneauVuDto, LignePlanningDto, NouveauxCreneauxBody, NouvelleRencontreBody, RencontreDto,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::noyau::temps::{a_paris, paris_vers_utc};

pub fn composer_rencontre(titre: &str, classe_id: &str) -> Result<NouvelleRencontreBody, String> {
    let titre = titre.trim();
    if titre.is_empty() || titre.chars().count() > 200 {
        return Err("Le titre est obligatoire (200 caractères au plus).".into());
    }
    let classe_id =
        Uuid::parse_str(classe_id.trim()).map_err(|_| "Choisissez une classe.".to_string())?;
    Ok(NouvelleRencontreBody {
        titre: titre.to_string(),
        classe_id,
    })
}

/// Créneaux consécutifs d'un enseignant à partir d'un jour et d'une heure **de Paris**.
pub fn composer_creneaux(
    enseignant: &str,
    jour: &str,
    heure: &str,
    duree: &str,
    nombre: &str,
) -> Result<NouveauxCreneauxBody, String> {
    let enseignant_user_id =
        Uuid::parse_str(enseignant.trim()).map_err(|_| "Choisissez un enseignant.".to_string())?;
    let jour = NaiveDate::parse_from_str(jour.trim(), "%Y-%m-%d")
        .map_err(|_| "Jour invalide.".to_string())?;
    let heure = NaiveTime::parse_from_str(heure.trim(), "%H:%M")
        .map_err(|_| "Heure invalide.".to_string())?;
    let duree_minutes: i32 = duree
        .trim()
        .parse()
        .map_err(|_| "Durée invalide.".to_string())?;
    let nombre: i32 = nombre
        .trim()
        .parse()
        .map_err(|_| "Nombre de créneaux invalide.".to_string())?;
    if !(5..=120).contains(&duree_minutes) {
        return Err("La durée d'un créneau est de 5 à 120 minutes.".into());
    }
    if !(1..=40).contains(&nombre) {
        return Err("Proposez de 1 à 40 créneaux à la fois.".into());
    }
    Ok(NouveauxCreneauxBody {
        enseignant_user_id,
        debut: paris_vers_utc(NaiveDateTime::new(jour, heure)),
        duree_minutes,
        nombre,
    })
}

pub fn libelle_statut_rencontre(s: &str) -> &'static str {
    match s {
        "brouillon" => "En préparation",
        "ouverte" => "Ouverte aux familles",
        "close" => "Close",
        _ => "Inconnu",
    }
}

pub fn ton_statut_rencontre(s: &str) -> TonEtat {
    match s {
        "ouverte" => TonEtat::Ok,
        "brouillon" => TonEtat::Info,
        _ => TonEtat::Neutre,
    }
}

/// « 3 créneaux, 1 réservé ».
pub fn resume_rencontre(r: &RencontreDto) -> String {
    format!("{} créneau(x), {} réservé(s)", r.nb_creneaux, r.nb_reserves)
}

/// Les actions proposées selon l'état (le serveur revérifie).
pub fn peut_ouvrir(r: &RencontreDto) -> bool {
    r.statut == "brouillon" && r.nb_creneaux > 0
}

pub fn peut_cloturer(r: &RencontreDto) -> bool {
    r.statut != "close"
}

/// « 09:00–09:10 » (heure de Paris).
pub fn plage(debut: chrono::DateTime<chrono::Utc>, fin: chrono::DateTime<chrono::Utc>) -> String {
    format!(
        "{}–{}",
        a_paris(debut).format("%H:%M"),
        a_paris(fin).format("%H:%M")
    )
}

/// Jour de Paris d'un créneau.
pub fn jour_de(debut: chrono::DateTime<chrono::Utc>) -> NaiveDate {
    a_paris(debut).date()
}

pub fn libelle_etat_creneau(c: &CreneauVuDto) -> &'static str {
    match c.etat.as_str() {
        "libre" => "Libre",
        "reserve_par_moi" => "Votre rendez-vous",
        _ => "Indisponible",
    }
}

/// Planning regroupé par enseignant (ordre d'apparition conservé).
pub fn grouper_par_enseignant(lignes: &[LignePlanningDto]) -> Vec<(String, Vec<LignePlanningDto>)> {
    let mut groupes: Vec<(Uuid, String, Vec<LignePlanningDto>)> = Vec::new();
    for l in lignes {
        match groupes.iter_mut().find(|g| g.0 == l.enseignant_user_id) {
            Some(g) => g.2.push(l.clone()),
            None => groupes.push((
                l.enseignant_user_id,
                l.enseignant_nom
                    .clone()
                    .unwrap_or_else(|| "Enseignant".into()),
                vec![l.clone()],
            )),
        }
    }
    groupes.into_iter().map(|(_, n, v)| (n, v)).collect()
}

/// « Prénom Nom — responsable » d'un créneau réservé (vue de l'organisateur ou de l'enseignant).
pub fn qui(l: &LignePlanningDto) -> String {
    if l.statut != "reserve" {
        return "Libre".into();
    }
    let eleve = format!(
        "{} {}",
        l.eleve_prenom.as_deref().unwrap_or(""),
        l.eleve_nom.as_deref().unwrap_or("")
    );
    match l.responsable_nom.as_deref() {
        Some(r) => format!("{} — {r}", eleve.trim()),
        None => eleve.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn utc(h: u32, m: u32) -> DateTime<Utc> {
        DateTime::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(2026, 11, 10)
                .unwrap()
                .and_hms_opt(h, m, 0)
                .unwrap(),
            Utc,
        )
    }

    #[test]
    fn rencontre_validee() {
        let classe = Uuid::from_u128(3).to_string();
        assert!(composer_rencontre("  ", &classe).is_err());
        assert!(composer_rencontre("Rencontres 4B", "").is_err());
        assert_eq!(
            composer_rencontre(" Rencontres 4B ", &classe)
                .unwrap()
                .titre,
            "Rencontres 4B"
        );
    }

    #[test]
    fn creneaux_saisis_en_heure_de_paris() {
        let prof = Uuid::from_u128(9).to_string();
        let c = composer_creneaux(&prof, "2026-11-10", "18:00", "10", "6").unwrap();
        // Novembre : heure d'hiver, UTC+1.
        assert_eq!(c.debut, utc(17, 0));
        assert_eq!((c.duree_minutes, c.nombre), (10, 6));
        let ete = composer_creneaux(&prof, "2026-06-16", "18:00", "10", "1").unwrap();
        assert_eq!(a_paris(ete.debut).format("%H:%M").to_string(), "18:00");
        for (j, h, d, n) in [
            ("pas un jour", "18:00", "10", "6"),
            ("2026-11-10", "25:00", "10", "6"),
            ("2026-11-10", "18:00", "2", "6"),
            ("2026-11-10", "18:00", "10", "0"),
            ("2026-11-10", "18:00", "10", "41"),
        ] {
            assert!(
                composer_creneaux(&prof, j, h, d, n).is_err(),
                "{j} {h} {d} {n}"
            );
        }
        assert!(composer_creneaux("", "2026-11-10", "18:00", "10", "6").is_err());
    }

    #[test]
    fn plage_et_etats() {
        assert_eq!(plage(utc(17, 0), utc(17, 10)), "18:00–18:10");
        let r = RencontreDto {
            id: Uuid::nil(),
            titre: "t".into(),
            classe_id: Uuid::nil(),
            classe_libelle: "4B".into(),
            statut: "brouillon".into(),
            nb_creneaux: 0,
            nb_reserves: 0,
        };
        assert!(!peut_ouvrir(&r), "pas de créneau");
        let r = RencontreDto {
            nb_creneaux: 3,
            ..r
        };
        assert!(peut_ouvrir(&r) && peut_cloturer(&r));
        let close = RencontreDto {
            statut: "close".into(),
            ..r
        };
        assert!(!peut_ouvrir(&close) && !peut_cloturer(&close));
    }

    #[test]
    fn planning_groupe_et_identite() {
        let ligne = |prof: u128, h: u32, statut: &str| LignePlanningDto {
            creneau_id: Uuid::from_u128(h as u128),
            rencontre_id: Uuid::nil(),
            enseignant_user_id: Uuid::from_u128(prof),
            enseignant_nom: Some(format!("Prof {prof}")),
            debut: utc(h, 0),
            fin: utc(h, 10),
            statut: statut.into(),
            eleve_prenom: (statut == "reserve").then(|| "Harry".into()),
            eleve_nom: (statut == "reserve").then(|| "Potter".into()),
            responsable_nom: (statut == "reserve").then(|| "Lily Potter".into()),
        };
        let l = [
            ligne(1, 8, "libre"),
            ligne(2, 8, "reserve"),
            ligne(1, 9, "reserve"),
        ];
        let g = grouper_par_enseignant(&l);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].1.len(), 2);
        assert_eq!(qui(&l[0]), "Libre");
        assert_eq!(qui(&l[1]), "Harry Potter — Lily Potter");
    }
}
