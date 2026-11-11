//! Séries hebdomadaires de sorties (séances régulières : atelier, visite, sortie piscine…) :
//! composition de `SerieSaisieDto`, lecture du résultat (séances créées, dates écartées) et de la
//! validation de toute la série. **Aucune règle métier** : le calcul des dates, l'exclusion des
//! fériés et des vacances et le droit de valider (séance par séance) restent côté serveur.

use chrono::{NaiveDate, NaiveTime};
use ent_types::voyages::{ResultatSeanceDto, ResultatSerieDto, SerieSaisieDto};
use uuid::Uuid;

use super::modele::{composer_public, erreur, Erreurs, SaisieVoyage};

pub const JOURS_SEMAINE: [(&str, &str); 7] = [
    ("1", "Lundi"),
    ("2", "Mardi"),
    ("3", "Mercredi"),
    ("4", "Jeudi"),
    ("5", "Vendredi"),
    ("6", "Samedi"),
    ("7", "Dimanche"),
];

/// Champs du formulaire « Nouvelle série », tels que saisis. Une série est faite de sorties sans
/// nuitée : la forme n'est pas demandée.
#[derive(Clone, Debug, Default)]
pub struct SaisieSerie {
    pub titre: String,
    pub jour: String,
    pub heure_debut: String,
    pub heure_fin: String,
    pub du: String,
    pub au: String,
    pub nature: String,
    pub destination: String,
    pub pays: String,
    pub lieu: String,
    pub objectifs: String,
    pub effectif: String,
    pub adultes: String,
    pub point_rendez_vous: String,
    pub besoin_car: bool,
    pub classes: Vec<Uuid>,
}

/// Contrôle de confort avant l'envoi, puis corps de `POST /voyages/series`. Le gabarit est contrôlé
/// avec les mêmes messages que la création d'une sortie ; les erreurs portent des identifiants
/// `voy-serie-…`.
pub fn composer_serie(s: &SaisieSerie) -> Result<SerieSaisieDto, Erreurs> {
    let mut e: Erreurs = Vec::new();
    let du = NaiveDate::parse_from_str(s.du.trim(), "%Y-%m-%d").ok();
    let au = NaiveDate::parse_from_str(s.au.trim(), "%Y-%m-%d").ok();
    let hd = NaiveTime::parse_from_str(s.heure_debut.trim(), "%H:%M").ok();
    let hf = NaiveTime::parse_from_str(s.heure_fin.trim(), "%H:%M").ok();
    let jour = s
        .jour
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|j| (1..=7).contains(j));
    if jour.is_none() {
        erreur(
            &mut e,
            "voy-serie-jour",
            "Choisissez le jour de la semaine.",
        );
    }
    if du.is_none() {
        erreur(
            &mut e,
            "voy-serie-du",
            "Indiquez la date de la première semaine.",
        );
    }
    if au.is_none() {
        erreur(
            &mut e,
            "voy-serie-au",
            "Indiquez la date de fin de la série.",
        );
    }
    if let (Some(du), Some(au)) = (du, au) {
        if au < du {
            erreur(
                &mut e,
                "voy-serie-au",
                "La fin doit suivre le début de la série.",
            );
        }
    }
    if hd.is_none() {
        erreur(
            &mut e,
            "voy-serie-heure-debut",
            "Indiquez l'heure de début.",
        );
    }
    if hf.is_none() {
        erreur(&mut e, "voy-serie-heure-fin", "Indiquez l'heure de fin.");
    }
    if let (Some(hd), Some(hf)) = (hd, hf) {
        if hf <= hd {
            erreur(
                &mut e,
                "voy-serie-heure-fin",
                "L'heure de fin doit suivre l'heure de début.",
            );
        }
    }
    // Gabarit : une séance type, un seul jour (les dates réelles sont posées par le serveur).
    let jour_type = du
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "2000-01-03".into());
    let gabarit = SaisieVoyage {
        titre: s.titre.clone(),
        nature: s.nature.clone(),
        forme: "sans_nuitee".into(),
        destination: s.destination.clone(),
        pays: s.pays.clone(),
        lieu: s.lieu.clone(),
        objectifs: s.objectifs.clone(),
        date_debut: jour_type.clone(),
        heure_debut: s.heure_debut.clone(),
        date_fin: jour_type,
        heure_fin: s.heure_fin.clone(),
        effectif: s.effectif.clone(),
        adultes: s.adultes.clone(),
        point_rendez_vous: s.point_rendez_vous.clone(),
        besoin_car: s.besoin_car,
        classes: s.classes.clone(),
    };
    let modele = match composer_public(&gabarit, false) {
        Ok(m) => Some(m),
        Err(autres) => {
            // Les dates et heures ont leurs propres messages ci-dessus.
            e.extend(
                autres
                    .into_iter()
                    .filter(|(c, _)| !c.starts_with("voy-date-") && !c.starts_with("voy-heure-"))
                    .map(|(c, m)| (c.replacen("voy-", "voy-serie-", 1), m)),
            );
            None
        }
    };
    if !e.is_empty() {
        return Err(e);
    }
    match (modele, jour, du, au, hd, hf) {
        (
            Some(modele),
            Some(jour_semaine),
            Some(du),
            Some(au),
            Some(heure_debut),
            Some(heure_fin),
        ) => Ok(SerieSaisieDto {
            libelle: s.titre.trim().to_string(),
            jour_semaine,
            heure_debut,
            heure_fin,
            du,
            au,
            modele,
        }),
        // Inatteignable : chaque cas manquant a produit une erreur plus haut.
        _ => Err(e),
    }
}

/// Motif d'une date écartée (codes du calendrier de l'établissement).
pub fn libelle_motif_ecarte(code: &str) -> &'static str {
    match code {
        "ferie" => "Jour férié",
        "vacances" => "Vacances scolaires",
        "ferme" => "Établissement fermé",
        "manuel" => "Écartée à la main",
        _ => "Jour non ouvré",
    }
}

/// « 1 séance créée », « 12 séances créées » (pluriel dès 2).
fn nombre(n: usize, singulier: &str, pluriel: &str) -> String {
    format!("{n} {}", if n > 1 { pluriel } else { singulier })
}

/// Phrase de confirmation après la création : « 12 séances créées ; 3 dates écartées. »
pub fn resume_creation(r: &ResultatSerieDto) -> String {
    let creees = nombre(r.sorties.len(), "séance créée", "séances créées");
    if r.ecartees.is_empty() {
        format!("{creees}.")
    } else {
        format!(
            "{creees} ; {}.",
            nombre(r.ecartees.len(), "date écartée", "dates écartées")
        )
    }
}

/// Bilan de « Valider toute la série » : combien de séances validées, lesquelles ne l'ont pas été.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BilanValidation {
    pub validees: usize,
    pub refusees: Vec<ResultatSeanceDto>,
}

pub fn bilan_validation(r: &[ResultatSeanceDto]) -> BilanValidation {
    BilanValidation {
        validees: r.iter().filter(|s| s.ok).count(),
        refusees: r.iter().filter(|s| !s.ok).cloned().collect(),
    }
}

impl BilanValidation {
    pub fn message(&self) -> String {
        let validees = |n| nombre(n, "séance validée", "séances validées");
        let refusees = |n| nombre(n, "séance refusée", "séances refusées");
        match (self.validees, self.refusees.len()) {
            (n, 0) => format!("{}.", validees(n)),
            (0, m) => format!("Aucune séance validée ; {}.", refusees(m)),
            (n, m) => format!("{} ; {}.", validees(n), refusees(m)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use ent_types::voyages::DateEcarteeDto;

    fn valide() -> SaisieSerie {
        SaisieSerie {
            titre: "Atelier théâtre".into(),
            jour: "3".into(),
            heure_debut: "13:30".into(),
            heure_fin: "15:30".into(),
            du: "2027-01-06".into(),
            au: "2027-06-30".into(),
            nature: "obligatoire".into(),
            destination: "proximite".into(),
            lieu: "Théâtre municipal".into(),
            classes: vec![Uuid::nil()],
            ..Default::default()
        }
    }

    #[test]
    fn composition_valide() {
        let s = composer_serie(&valide()).expect("valide");
        assert_eq!(s.libelle, "Atelier théâtre");
        assert_eq!(s.jour_semaine, 3);
        assert_eq!(s.heure_debut, NaiveTime::from_hms_opt(13, 30, 0).unwrap());
        assert_eq!(s.du, NaiveDate::from_ymd_opt(2027, 1, 6).unwrap());
        assert_eq!(s.modele.forme, "sans_nuitee");
        assert_eq!(s.modele.titre, "Atelier théâtre");
        assert_eq!(
            s.modele.destination_libelle.as_deref(),
            Some("Théâtre municipal")
        );
        assert!(s.modele.fin > s.modele.debut);
    }

    #[test]
    fn composition_refuse_et_dit_pourquoi() {
        let mut s = valide();
        s.jour = String::new();
        s.au = "2026-12-31".into();
        s.heure_fin = "13:00".into();
        s.titre = " ".into();
        s.classes.clear();
        let champs: Vec<String> = composer_serie(&s)
            .unwrap_err()
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        for attendu in [
            "voy-serie-jour",
            "voy-serie-au",
            "voy-serie-heure-fin",
            "voy-serie-titre",
            "voy-serie-classes",
        ] {
            assert!(
                champs.contains(&attendu.to_string()),
                "{attendu} : {champs:?}"
            );
        }
        assert!(champs.iter().all(|c| c.starts_with("voy-serie-")));
    }

    #[test]
    fn dates_et_heures_illisibles_ne_sont_signalees_qu_une_fois() {
        let mut s = valide();
        s.du = String::new();
        s.heure_debut = "13h".into();
        let champs: Vec<String> = composer_serie(&s)
            .unwrap_err()
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        assert_eq!(
            champs.iter().filter(|c| *c == "voy-serie-du").count(),
            1,
            "{champs:?}"
        );
        assert_eq!(
            champs
                .iter()
                .filter(|c| *c == "voy-serie-heure-debut")
                .count(),
            1
        );
    }

    #[test]
    fn l_etranger_exige_le_pays_comme_a_la_creation() {
        let mut s = valide();
        s.destination = "etranger".into();
        let champs: Vec<String> = composer_serie(&s)
            .unwrap_err()
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        assert!(champs.contains(&"voy-serie-pays".to_string()));
    }

    #[test]
    fn motifs_d_ecart_lisibles() {
        assert_eq!(libelle_motif_ecarte("ferie"), "Jour férié");
        assert_eq!(libelle_motif_ecarte("vacances"), "Vacances scolaires");
        assert_eq!(libelle_motif_ecarte("ferme"), "Établissement fermé");
        assert_eq!(libelle_motif_ecarte("autre"), "Jour non ouvré");
    }

    fn resultat(creees: usize, ecartees: usize) -> ResultatSerieDto {
        let debut = Utc.with_ymd_and_hms(2027, 1, 6, 12, 30, 0).unwrap();
        ResultatSerieDto {
            serie_id: Uuid::nil(),
            sorties: (0..creees)
                .map(|_| ent_types::voyages::SortieResumeDto {
                    id: Uuid::nil(),
                    reference: String::new(),
                    titre: String::new(),
                    debut,
                    fin: debut,
                    destination: String::new(),
                    destination_libelle: None,
                    nature: String::new(),
                    forme: String::new(),
                    etat: "brouillon".into(),
                    serie_id: None,
                    porteur_nom: None,
                    nb_inscrits: None,
                    acces_detail: true,
                    est_porteur: true,
                })
                .collect(),
            ecartees: (0..ecartees)
                .map(|_| DateEcarteeDto {
                    jour: NaiveDate::from_ymd_opt(2027, 5, 5).unwrap(),
                    motif: "ferie".into(),
                })
                .collect(),
        }
    }

    #[test]
    fn resume_de_creation() {
        assert_eq!(resume_creation(&resultat(1, 0)), "1 séance créée.");
        assert_eq!(
            resume_creation(&resultat(12, 3)),
            "12 séances créées ; 3 dates écartées."
        );
    }

    #[test]
    fn bilan_de_validation_liste_les_refus() {
        let seance = |ok: bool, raison: Option<&str>| ResultatSeanceDto {
            sortie_id: Uuid::nil(),
            titre: "Atelier".into(),
            debut: Utc.with_ymd_and_hms(2027, 1, 6, 12, 30, 0).unwrap(),
            ok,
            raison: raison.map(str::to_string),
        };
        let b = bilan_validation(&[
            seance(true, None),
            seance(false, Some("droit insuffisant")),
            seance(true, None),
        ]);
        assert_eq!(b.validees, 2);
        assert_eq!(b.refusees.len(), 1);
        assert_eq!(b.message(), "2 séances validées ; 1 séance refusée.");
        assert_eq!(
            bilan_validation(&[seance(true, None)]).message(),
            "1 séance validée."
        );
        assert_eq!(
            bilan_validation(&[seance(false, None)]).message(),
            "Aucune séance validée ; 1 séance refusée."
        );
    }
}
