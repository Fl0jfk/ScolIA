//! Logique d'écran du remplacement de courte durée (RCD, socle 15, A11e) : **pure**, sans Leptos ni réseau,
//! testée en natif. Plan annuel versionné (R-01), volontaires et compteurs, remontée hebdomadaire **sans nom**
//! (R-02).
//!
//! Le serveur reste l'autorité (`rcd.gerer`, `rcd.remontee`). Aucun libellé ne nomme un autre volontaire : un
//! compteur ne donne que ses chiffres et sa **position** par rapport à la médiane (ABP-213).

use chrono::{Datelike, Duration, NaiveDate};
use ent_types::absences_personnels::{
    EngagementRcdBody, LigneRemonteeRcdDto, PlanRcdBody, PlanRcdDto, ProcedurePlanRcdBody,
    RemonteeRcdDto,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

// ───────────────────────────── Libellés ─────────────────────────────

pub fn libelle_etat_plan(etat: &str) -> &'static str {
    match etat {
        "brouillon" => "Brouillon",
        "presente" => "Présenté",
        "transmis" => "Transmis au recteur",
        "archive" => "Archivé",
        _ => "État inconnu",
    }
}

pub fn ton_etat_plan(etat: &str) -> TonEtat {
    match etat {
        "brouillon" => TonEtat::Avertissement,
        "presente" => TonEtat::Info,
        "transmis" => TonEtat::Ok,
        _ => TonEtat::Neutre,
    }
}

pub const INSTANCES: &[(&str, &str)] = &[
    ("ca", "Conseil d'administration"),
    ("instance_etablissement", "Instance de l'établissement"),
];

pub fn libelle_instance(code: &str) -> &'static str {
    INSTANCES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("Instance inconnue", |(_, l)| *l)
}

pub const STATUTS_ENGAGEMENT: &[(&str, &str)] = &[
    ("pacte", "Pacte"),
    ("hse", "Heures supplémentaires (HSE)"),
    ("autre", "Autre"),
];

pub fn libelle_statut_engagement(code: &str) -> &'static str {
    STATUTS_ENGAGEMENT
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("Statut inconnu", |(_, l)| *l)
}

/// Position du volontaire par rapport à la médiane, **sans nommer personne** (ABP-213).
pub fn libelle_position(position: &str) -> &'static str {
    match position {
        "en_dessous" => "Moins sollicité que la médiane des volontaires",
        "dans_la_mediane" => "Dans la médiane des volontaires",
        "au_dessus" => "Plus sollicité que la médiane des volontaires",
        _ => "Position non calculée",
    }
}

pub fn libelle_etat_remontee(etat: &str) -> &'static str {
    match etat {
        "preparee" => "Préparée, à contrôler",
        "controlee" => "Contrôlée",
        "transmise" => "Transmise",
        "rejetee" => "Rejetée",
        _ => "État inconnu",
    }
}

pub fn ton_etat_remontee(etat: &str) -> TonEtat {
    match etat {
        "preparee" => TonEtat::Avertissement,
        "controlee" => TonEtat::Info,
        "transmise" => TonEtat::Ok,
        "rejetee" => TonEtat::Mal,
        _ => TonEtat::Neutre,
    }
}

/// Catégories nationales du motif : un motif **par catégorie**, jamais la nature fine (R-02).
pub fn libelle_categorie_nationale(code: &str) -> &'static str {
    match code {
        "raison_sante" => "Raison de santé",
        "evenement_familial" => "Événement familial",
        "formation" => "Formation",
        "institutionnel" => "Institutionnel",
        "convenance_personnelle" => "Convenance personnelle",
        "en_attente_de_motif" => "En attente de motif",
        "non_detaille" => "Non détaillé (effectif trop faible)",
        _ => "Autre",
    }
}

/// Effectif d'un regroupement : le nombre s'il atteint le seuil de masquage, sinon « < 3 » (jamais le chiffre exact).
pub fn libelle_effectif(effectif: Option<i16>) -> String {
    effectif.map_or_else(|| "< 3 agents".to_string(), |e| format!("{e} agents"))
}

pub fn libelle_mode_continuite(code: &str) -> &'static str {
    match code {
        "remplacement" => "Remplacement",
        "autre_cours" => "Autre cours",
        "deplacement" => "Cours déplacé",
        "continuite" => "Continuité encadrée",
        "annulation" => "Cours annulé",
        "suppleance" => "Suppléance",
        "aucune" => "Aucune solution",
        _ => "Autre",
    }
}

// ───────────────────────────── Plan RCD ─────────────────────────────

pub const OBJECTIFS_MAX: usize = 1000;
pub const DELAI_PREVENANCE_DEFAUT: i16 = 24;
pub const DELAI_PREVENANCE_MAX: i16 = 168;
/// Comptes rendus attendus par an (R-01 : au moins deux).
pub const COMPTES_RENDUS_ATTENDUS: i64 = 2;

/// Saisie du plan (création, modification d'un brouillon, nouvelle version).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisiePlan {
    pub objectifs: String,
    pub delai: String,
    pub continuite_numerique: bool,
}

pub fn composer_plan(s: &SaisiePlan) -> Result<PlanRcdBody, String> {
    let objectifs = s.objectifs.trim();
    if objectifs.chars().count() > OBJECTIFS_MAX {
        return Err(format!(
            "Les objectifs sont limités à {OBJECTIFS_MAX} caractères."
        ));
    }
    let delai: i16 = if s.delai.trim().is_empty() {
        DELAI_PREVENANCE_DEFAUT
    } else {
        s.delai
            .trim()
            .parse()
            .map_err(|_| "Le délai de prévenance est un nombre d'heures entier.".to_string())?
    };
    if !(0..=DELAI_PREVENANCE_MAX).contains(&delai) {
        return Err(format!(
            "Le délai de prévenance va de 0 à {DELAI_PREVENANCE_MAX} heures."
        ));
    }
    Ok(PlanRcdBody {
        annee_id: None,
        objectifs: (!objectifs.is_empty()).then(|| objectifs.to_string()),
        delai_prevenance_heures: Some(delai),
        continuite_numerique: Some(s.continuite_numerique),
        referent_agent_id: None,
    })
}

pub fn saisie_depuis_plan(p: &PlanRcdDto) -> SaisiePlan {
    SaisiePlan {
        objectifs: p.objectifs.clone().unwrap_or_default(),
        delai: p.delai_prevenance_heures.to_string(),
        continuite_numerique: p.continuite_numerique,
    }
}

/// Le brouillon se modifie en place ; un plan présenté ou transmis donne une **nouvelle version**.
pub fn plan_modifiable_en_place(p: &PlanRcdDto) -> bool {
    p.etat == "brouillon"
}

/// Saisie des faits de procédure (ABP-222, ABP-223) : dates `AAAA-MM-JJ`, au moins un champ.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieProcedure {
    pub avis_cp_le: String,
    pub presente_le: String,
    pub instance: String,
    pub transmis_le: String,
}

fn date_facultative(t: &str, nom: &str) -> Result<Option<NaiveDate>, String> {
    let t = t.trim();
    if t.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(t, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| format!("{nom} : date invalide."))
}

pub fn composer_procedure(s: &SaisieProcedure) -> Result<ProcedurePlanRcdBody, String> {
    let avis = date_facultative(&s.avis_cp_le, "Avis du conseil pédagogique")?;
    let presente = date_facultative(&s.presente_le, "Présentation")?;
    let transmis = date_facultative(&s.transmis_le, "Transmission au recteur")?;
    let instance = (!s.instance.is_empty()).then(|| s.instance.clone());
    if presente.is_some() && instance.is_none() {
        return Err(
            "Indiquez l'instance devant laquelle le plan a été présenté (conseil d'administration ou instance de l'établissement)."
                .to_string(),
        );
    }
    if avis.is_none() && presente.is_none() && transmis.is_none() {
        return Err("Renseignez au moins une date.".to_string());
    }
    Ok(ProcedurePlanRcdBody {
        avis_cp_le: avis,
        presente_le: presente,
        instance,
        transmis_le: transmis,
    })
}

/// Comptes rendus qui manquent encore pour la version courante (au moins deux par an, R-01).
pub fn comptes_rendus_manquants(p: &PlanRcdDto) -> i64 {
    (COMPTES_RENDUS_ATTENDUS - p.comptes_rendus).max(0)
}

/// La campagne de volontariat se lance sur un plan vivant, une seule fois par version.
pub fn campagne_lancable(p: &PlanRcdDto) -> bool {
    p.etat != "archive" && p.campagne_lancee_le.is_none()
}

/// Période d'un compte rendu : fin après début, au plus un an.
pub fn composer_periode(du: &str, au: &str) -> Result<(NaiveDate, NaiveDate), String> {
    let d = NaiveDate::parse_from_str(du.trim(), "%Y-%m-%d")
        .map_err(|_| "Date de début invalide.".to_string())?;
    let a = NaiveDate::parse_from_str(au.trim(), "%Y-%m-%d")
        .map_err(|_| "Date de fin invalide.".to_string())?;
    if a < d {
        return Err("La fin de la période précède son début.".to_string());
    }
    if (a - d).num_days() > 366 {
        return Err("Une période de compte rendu ne dépasse pas un an.".to_string());
    }
    Ok((d, a))
}

// ───────────────────────────── Engagement ─────────────────────────────

/// Saisie d'un engagement de volontaire (soi-même : `agent` vide).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieEngagement {
    pub agent: String,
    pub volume: String,
    pub statut: String,
    pub du: String,
    pub au: String,
}

pub fn composer_engagement(s: &SaisieEngagement) -> Result<EngagementRcdBody, String> {
    let volume: f64 = s
        .volume
        .trim()
        .replace(',', ".")
        .parse()
        .map_err(|_| "Le volume annuel est un nombre d'heures.".to_string())?;
    if !(volume.is_finite() && volume > 0.0 && volume <= 500.0) {
        return Err("Le volume annuel va de plus de 0 à 500 heures.".to_string());
    }
    if !STATUTS_ENGAGEMENT.iter().any(|(c, _)| *c == s.statut) {
        return Err("Choisissez le statut de l'engagement.".to_string());
    }
    let du = NaiveDate::parse_from_str(s.du.trim(), "%Y-%m-%d")
        .map_err(|_| "Date de début invalide.".to_string())?;
    let au = NaiveDate::parse_from_str(s.au.trim(), "%Y-%m-%d")
        .map_err(|_| "Date de fin invalide.".to_string())?;
    if au < du {
        return Err("La fin de l'engagement précède son début.".to_string());
    }
    let agent_id = if s.agent.trim().is_empty() {
        None
    } else {
        Some(
            Uuid::parse_str(s.agent.trim())
                .map_err(|_| "Choisissez un agent dans la liste.".to_string())?,
        )
    };
    Ok(EngagementRcdBody {
        agent_id,
        volume_heures: (volume * 10.0).round() / 10.0,
        statut: s.statut.clone(),
        du,
        au,
    })
}

/// Libellé d'un volume : « 18 h », « 7,5 h ».
pub fn libelle_heures(h: f64) -> String {
    if (h - h.round()).abs() < 0.05 {
        format!("{} h", h.round() as i64)
    } else {
        format!("{} h", format!("{h:.1}").replace('.', ","))
    }
}

/// Part du volume engagé déjà acceptée, en pourcentage (0 à 100, plafonné) pour la jauge.
pub fn part_consommee(acceptees: f64, volume: f64) -> u32 {
    if volume <= 0.0 {
        return 0;
    }
    ((acceptees / volume) * 100.0).round().clamp(0.0, 100.0) as u32
}

// ───────────────────────────── Remontée hebdomadaire ─────────────────────────────

/// Lundi de la semaine d'une date.
pub fn lundi_de(d: NaiveDate) -> NaiveDate {
    d - Duration::days(i64::from(d.weekday().num_days_from_monday()))
}

/// Lundi de la semaine passée (celle que prépare la tâche du lundi).
pub fn semaine_passee(aujourdhui: NaiveDate) -> NaiveDate {
    lundi_de(aujourdhui) - Duration::days(7)
}

/// « du 9 au 15 novembre 2026 » pour une semaine commençant un lundi.
pub fn libelle_semaine(lundi: NaiveDate) -> String {
    use crate::noyau::temps::nom_mois;
    let dimanche = lundi + Duration::days(6);
    if lundi.month() == dimanche.month() {
        format!(
            "du {} au {} {} {}",
            lundi.day(),
            dimanche.day(),
            nom_mois(dimanche.month()),
            dimanche.year()
        )
    } else {
        format!(
            "du {} {} au {} {} {}",
            lundi.day(),
            nom_mois(lundi.month()),
            dimanche.day(),
            nom_mois(dimanche.month()),
            dimanche.year()
        )
    }
}

/// Totaux d'une remontée : heures prévues, d'absence déclarées, remplacées.
pub fn totaux_remontee(r: &RemonteeRcdDto) -> (f64, f64, f64) {
    r.lignes.iter().fold((0.0, 0.0, 0.0), |(p, a, m), l| {
        (
            p + l.heures_prevues,
            a + l.heures_absence,
            m + l.heures_remplacees,
        )
    })
}

/// Part des heures d'absence remplacées, en pourcentage (jamais au-delà de 100).
pub fn taux_remplacement(absence: f64, remplacees: f64) -> Option<u32> {
    (absence > 0.0).then(|| ((remplacees / absence) * 100.0).round().clamp(0.0, 100.0) as u32)
}

/// Lignes d'une remontée regroupées par discipline (ordre alphabétique), pour la lecture.
pub fn lignes_par_discipline(r: &RemonteeRcdDto) -> Vec<(String, Vec<&LigneRemonteeRcdDto>)> {
    let mut codes: Vec<&str> = r
        .lignes
        .iter()
        .map(|l| l.discipline_code.as_str())
        .collect();
    codes.sort_unstable();
    codes.dedup();
    codes
        .into_iter()
        .map(|c| {
            (
                c.to_string(),
                r.lignes.iter().filter(|l| l.discipline_code == c).collect(),
            )
        })
        .collect()
}

/// Étapes qu'on propose sur une remontée selon son état (le serveur contrôle l'enchaînement).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionsRemontee {
    pub controler: bool,
    pub rejeter: bool,
    pub transmettre: bool,
    pub exporter: bool,
}

pub fn actions_remontee(etat: &str) -> ActionsRemontee {
    match etat {
        "preparee" => ActionsRemontee {
            controler: true,
            rejeter: true,
            ..Default::default()
        },
        // Contrôlée : elle part, ou on la rejette ; l'export (tableau équivalent) n'attend que le contrôle.
        "controlee" => ActionsRemontee {
            rejeter: true,
            transmettre: true,
            exporter: true,
            ..Default::default()
        },
        "transmise" => ActionsRemontee {
            exporter: true,
            ..Default::default()
        },
        _ => ActionsRemontee::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identifiant distinct à chaque appel (les identifiants aléatoires ne sont pas compilés pour le navigateur).
    fn identifiant() -> Uuid {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(1000);
        Uuid::from_u128(u128::from(N.fetch_add(1, Ordering::Relaxed)))
    }
    use chrono::Utc;

    fn jour(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn plan(etat: &str, comptes_rendus: i64) -> PlanRcdDto {
        PlanRcdDto {
            id: identifiant(),
            annee_id: identifiant(),
            version: 1,
            etat: etat.into(),
            objectifs: Some("Assurer la continuité".into()),
            delai_prevenance_heures: 24,
            continuite_numerique: true,
            referent_agent_id: None,
            avis_cp_le: None,
            presente_le: None,
            instance: None,
            transmis_le: None,
            campagne_lancee_le: None,
            comptes_rendus,
            cree_le: Utc::now(),
        }
    }

    #[test]
    fn le_plan_suit_les_bornes_du_serveur() {
        let ok = composer_plan(&SaisiePlan {
            objectifs: "  Continuité  ".into(),
            delai: "48".into(),
            continuite_numerique: true,
        })
        .unwrap();
        assert_eq!(ok.objectifs.as_deref(), Some("Continuité"));
        assert_eq!(ok.delai_prevenance_heures, Some(48));
        // Délai vide : le défaut du serveur (24 h).
        let defaut = composer_plan(&SaisiePlan::default()).unwrap();
        assert_eq!(defaut.delai_prevenance_heures, Some(24));
        assert_eq!(defaut.objectifs, None);
        for mauvais in ["169", "-1", "beaucoup", "2,5"] {
            assert!(
                composer_plan(&SaisiePlan {
                    delai: mauvais.into(),
                    ..Default::default()
                })
                .is_err(),
                "{mauvais}"
            );
        }
        assert!(composer_plan(&SaisiePlan {
            objectifs: "x".repeat(1001),
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn un_brouillon_se_modifie_un_plan_presente_se_versionne() {
        assert!(plan_modifiable_en_place(&plan("brouillon", 0)));
        for etat in ["presente", "transmis", "archive"] {
            assert!(!plan_modifiable_en_place(&plan(etat, 0)), "{etat}");
        }
        let p = plan("presente", 0);
        let s = saisie_depuis_plan(&p);
        assert_eq!(s.delai, "24");
        assert!(s.continuite_numerique);
    }

    #[test]
    fn la_procedure_demande_l_instance_et_au_moins_une_date() {
        let vide = composer_procedure(&SaisieProcedure::default());
        assert_eq!(vide.unwrap_err(), "Renseignez au moins une date.");
        let sans_instance = composer_procedure(&SaisieProcedure {
            presente_le: "2026-10-03".into(),
            ..Default::default()
        });
        assert!(sans_instance.unwrap_err().contains("instance"));
        let ok = composer_procedure(&SaisieProcedure {
            avis_cp_le: "2026-09-20".into(),
            presente_le: "2026-10-03".into(),
            instance: "instance_etablissement".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(ok.presente_le, Some(jour(2026, 10, 3)));
        assert_eq!(ok.instance.as_deref(), Some("instance_etablissement"));
        assert!(composer_procedure(&SaisieProcedure {
            transmis_le: "3 octobre".into(),
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn deux_comptes_rendus_par_an_et_une_campagne_par_version() {
        assert_eq!(comptes_rendus_manquants(&plan("presente", 0)), 2);
        assert_eq!(comptes_rendus_manquants(&plan("presente", 1)), 1);
        assert_eq!(comptes_rendus_manquants(&plan("presente", 5)), 0);
        let mut p = plan("presente", 0);
        assert!(campagne_lancable(&p));
        p.campagne_lancee_le = Some(Utc::now());
        assert!(!campagne_lancable(&p));
        assert!(!campagne_lancable(&plan("archive", 0)));
    }

    #[test]
    fn la_periode_d_un_compte_rendu_est_ordonnee_et_bornee() {
        assert!(composer_periode("2026-09-01", "2026-12-31").is_ok());
        assert!(composer_periode("2026-12-31", "2026-09-01").is_err());
        assert!(composer_periode("2026-01-01", "2028-01-01").is_err());
        assert!(composer_periode("hier", "2026-01-01").is_err());
    }

    #[test]
    fn l_engagement_est_valide_avant_l_envoi() {
        let base = SaisieEngagement {
            agent: String::new(),
            volume: "18".into(),
            statut: "pacte".into(),
            du: "2026-09-01".into(),
            au: "2027-06-30".into(),
        };
        let b = composer_engagement(&base).unwrap();
        assert_eq!(b.agent_id, None, "soi-même");
        assert_eq!(b.volume_heures, 18.0);
        let virgule = composer_engagement(&SaisieEngagement {
            volume: "7,5".into(),
            ..base.clone()
        })
        .unwrap();
        assert_eq!(virgule.volume_heures, 7.5);
        for mauvais in ["0", "-3", "501", "abc"] {
            assert!(
                composer_engagement(&SaisieEngagement {
                    volume: mauvais.into(),
                    ..base.clone()
                })
                .is_err(),
                "{mauvais}"
            );
        }
        assert!(composer_engagement(&SaisieEngagement {
            statut: "bénévole".into(),
            ..base.clone()
        })
        .is_err());
        assert!(composer_engagement(&SaisieEngagement {
            du: "2027-07-01".into(),
            ..base.clone()
        })
        .is_err());
        let autre = composer_engagement(&SaisieEngagement {
            agent: Uuid::from_u128(7).to_string(),
            ..base.clone()
        })
        .unwrap();
        assert_eq!(autre.agent_id, Some(Uuid::from_u128(7)));
        assert!(composer_engagement(&SaisieEngagement {
            agent: "Mme Z".into(),
            ..base
        })
        .is_err());
    }

    #[test]
    fn les_volumes_se_lisent_et_la_jauge_est_plafonnee() {
        assert_eq!(libelle_heures(18.0), "18 h");
        assert_eq!(libelle_heures(7.5), "7,5 h");
        assert_eq!(libelle_heures(0.0), "0 h");
        assert_eq!(part_consommee(9.0, 18.0), 50);
        assert_eq!(part_consommee(30.0, 18.0), 100);
        assert_eq!(part_consommee(1.0, 0.0), 0);
    }

    #[test]
    fn la_position_ne_nomme_personne() {
        for p in ["en_dessous", "dans_la_mediane", "au_dessus"] {
            let t = libelle_position(p);
            assert!(t.contains("médiane"), "{p}");
            assert!(!t.contains("M.") && !t.contains("Mme"), "{p}");
        }
    }

    #[test]
    fn la_semaine_remontee_commence_un_lundi() {
        // Lundi 16 novembre 2026.
        let auj = jour(2026, 11, 16);
        assert_eq!(lundi_de(jour(2026, 11, 22)), auj);
        assert_eq!(lundi_de(auj), auj);
        assert_eq!(semaine_passee(auj), jour(2026, 11, 9));
        assert_eq!(semaine_passee(jour(2026, 11, 20)), jour(2026, 11, 9));
        assert_eq!(
            libelle_semaine(jour(2026, 11, 9)),
            "du 9 au 15 novembre 2026"
        );
        assert_eq!(
            libelle_semaine(jour(2026, 10, 26)),
            "du 26 octobre au 1 novembre 2026"
        );
    }

    fn ligne(d: &str, prevues: f64, absence: f64, remplacees: f64) -> LigneRemonteeRcdDto {
        LigneRemonteeRcdDto {
            discipline_code: d.into(),
            niveau_code: "4e".into(),
            categorie_nationale: "formation".into(),
            mode_continuite: "remplacement".into(),
            heures_prevues: prevues,
            heures_absence: absence,
            heures_remplacees: remplacees,
            effectif: Some(4),
        }
    }

    fn remontee(lignes: Vec<LigneRemonteeRcdDto>) -> RemonteeRcdDto {
        RemonteeRcdDto {
            id: identifiant(),
            semaine: jour(2026, 11, 9),
            etat: "preparee".into(),
            preparee_le: Utc::now(),
            controlee_le: None,
            transmise_le: None,
            lignes,
        }
    }

    #[test]
    fn la_remontee_se_totalise_et_se_regroupe_par_discipline() {
        let r = remontee(vec![
            ligne("maths", 20.0, 4.0, 3.0),
            ligne("francais", 18.0, 2.0, 2.0),
            ligne("maths", 10.0, 1.0, 0.0),
        ]);
        assert_eq!(totaux_remontee(&r), (48.0, 7.0, 5.0));
        let g = lignes_par_discipline(&r);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].0, "francais");
        assert_eq!(g[1].1.len(), 2);
        assert_eq!(taux_remplacement(7.0, 5.0), Some(71));
        assert_eq!(taux_remplacement(0.0, 0.0), None);
        assert_eq!(taux_remplacement(2.0, 5.0), Some(100));
    }

    #[test]
    fn la_remontee_ne_porte_aucun_identifiant_de_personne() {
        // Test de structure (R-02) : seules des clés de dimension et des heures, ni nom, ni nature fine.
        let r = remontee(vec![ligne("maths", 20.0, 4.0, 3.0)]);
        let v = serde_json::to_value(&r).unwrap();
        let permis_ligne = [
            "discipline_code",
            "niveau_code",
            "categorie_nationale",
            "mode_continuite",
            "heures_prevues",
            "heures_absence",
            "heures_remplacees",
            "effectif",
        ];
        for l in v["lignes"].as_array().unwrap() {
            for cle in l.as_object().unwrap().keys() {
                assert!(permis_ligne.contains(&cle.as_str()), "{cle}");
            }
        }
        for cle in v.as_object().unwrap().keys() {
            for interdit in ["agent", "nom", "nature", "motif", "texte", "absence_id"] {
                assert!(!cle.contains(interdit), "{cle}");
            }
        }
    }

    #[test]
    fn les_actions_d_une_remontee_suivent_son_etat() {
        let a = actions_remontee("preparee");
        assert!(a.controler && a.rejeter && !a.transmettre && !a.exporter);
        let a = actions_remontee("controlee");
        assert!(a.transmettre && a.exporter && a.rejeter && !a.controler);
        let a = actions_remontee("transmise");
        assert!(a.exporter && !a.transmettre && !a.rejeter);
        assert_eq!(actions_remontee("rejetee"), ActionsRemontee::default());
    }

    #[test]
    fn les_categories_nationales_sont_toutes_nommees() {
        for c in [
            "raison_sante",
            "evenement_familial",
            "formation",
            "institutionnel",
            "convenance_personnelle",
            "en_attente_de_motif",
            "non_detaille",
        ] {
            assert_ne!(libelle_categorie_nationale(c), "Autre", "{c}");
        }
        assert_eq!(libelle_effectif(None), "< 3 agents");
        assert_eq!(libelle_effectif(Some(5)), "5 agents");
        for m in [
            "remplacement",
            "autre_cours",
            "deplacement",
            "continuite",
            "annulation",
            "suppleance",
            "aucune",
        ] {
            assert_ne!(libelle_mode_continuite(m), "Autre", "{m}");
        }
    }
}
