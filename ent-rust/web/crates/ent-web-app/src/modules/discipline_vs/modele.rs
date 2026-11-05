//! Logique d'écran de la discipline et des signalements d'assiduité : **pure**, testée en natif.
//! Le serveur reste l'autorité (droits `punitions.donner`, `discipline.gerer`,
//! `assiduite.signaler`, délais, transitions) ; ces fonctions guident la saisie et composent les
//! corps de requête.
//!
//! Distinction métier à respecter à l'écran : une **punition** (mesure éducative, donnée par un
//! enseignant ou la vie scolaire) n'est **pas** une **sanction** (disciplinaire, inscrite au
//! dossier, soumise à procédure et à effacement). Deux sections, deux libellés, deux listes.

use chrono::{Duration, NaiveDate};
use ent_types::discipline::{DonnerPunitionBody, ProcedureDto, SignalementDto, TypeVsDto};
use ent_web_ui::TonEtat;
use uuid::Uuid;

pub type Erreurs = Vec<(String, String)>;

fn err(champ: &str, message: &str) -> (String, String) {
    (champ.to_string(), message.to_string())
}

pub fn lire_date(texte: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texte.trim(), "%Y-%m-%d").ok()
}

// ───────────────────────────── Punitions ─────────────────────────────

pub fn libelle_statut_punition(statut: &str) -> &'static str {
    match statut {
        "donnee" => "Donnée",
        "effectuee" => "Effectuée",
        "annulee" => "Annulée",
        _ => "Statut inconnu",
    }
}

pub fn ton_statut_punition(statut: &str) -> TonEtat {
    match statut {
        "donnee" => TonEtat::Avertissement,
        "effectuee" => TonEtat::Ok,
        _ => TonEtat::Neutre,
    }
}

/// Une punition donnée peut être clôturée (effectuée) ou annulée ; les autres sont figées.
pub fn punition_cloturable(statut: &str) -> bool {
    statut == "donnee"
}

/// Libellé du type d'une punition (le type est connu par son identifiant).
pub fn libelle_type(types: &[TypeVsDto], id: Uuid) -> String {
    types
        .iter()
        .find(|t| t.id == id)
        .map(|t| t.libelle.clone())
        .unwrap_or_else(|| "Type inconnu".into())
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisiePunition {
    pub type_id: String,
    pub jour: String,
    pub motif: String,
    pub echeance: String,
    pub duree: String,
}

/// Compose le corps de `POST /eleves/{id}/punitions`. Champs : `pun-type`, `pun-jour`, `pun-motif`,
/// `pun-echeance`, `pun-duree`.
pub fn composer_punition(s: &SaisiePunition) -> Result<DonnerPunitionBody, Erreurs> {
    let mut e = Erreurs::new();
    let type_id = s.type_id.trim().parse::<Uuid>().ok();
    if type_id.is_none() {
        e.push(err("pun-type", "Choisissez un type de punition."));
    }
    let jour = lire_date(&s.jour);
    if jour.is_none() {
        e.push(err("pun-jour", "Date invalide."));
    }
    let motif = s.motif.trim();
    if motif.is_empty() {
        e.push(err("pun-motif", "Le motif est obligatoire."));
    } else if motif.chars().count() > 500 {
        e.push(err("pun-motif", "Motif trop long (500 caractères au plus)."));
    }
    let echeance = if s.echeance.trim().is_empty() {
        None
    } else {
        let d = lire_date(&s.echeance);
        match (d, jour) {
            (None, _) => e.push(err("pun-echeance", "Date d'échéance invalide.")),
            (Some(d), Some(j)) if d < j => {
                e.push(err("pun-echeance", "L'échéance précède la date de la punition."))
            }
            _ => {}
        }
        d
    };
    let duree = if s.duree.trim().is_empty() {
        None
    } else {
        match s.duree.trim().parse::<i32>() {
            Ok(m) if (1..=600).contains(&m) => Some(m),
            _ => {
                e.push(err("pun-duree", "Durée en minutes entre 1 et 600."));
                None
            }
        }
    };
    match (type_id, jour, e.is_empty()) {
        (Some(type_id), Some(jour), true) => Ok(DonnerPunitionBody {
            type_id,
            jour,
            motif: motif.to_string(),
            echeance,
            duree_minutes: duree,
        }),
        _ => Err(e),
    }
}

// ───────────────────────────── Sanctions (lecture) ─────────────────────────────

pub fn libelle_statut_sanction(statut: &str) -> &'static str {
    match statut {
        "active" => "En cours",
        "executee" => "Exécutée",
        "annulee" => "Annulée",
        _ => "Statut inconnu",
    }
}

// ───────────────────────────── Procédure disciplinaire ─────────────────────────────

pub const INSTANCES: [(&str, &str); 2] = [
    ("chef_etablissement", "Chef d'établissement"),
    ("conseil_discipline", "Conseil de discipline"),
];

pub fn libelle_instance(instance: &str) -> &'static str {
    INSTANCES
        .iter()
        .find(|(k, _)| *k == instance)
        .map(|(_, l)| *l)
        .unwrap_or("Instance inconnue")
}

pub fn libelle_statut_procedure(statut: &str) -> &'static str {
    match statut {
        "engagee" => "Engagée",
        "convoquee" => "Conseil convoqué",
        "decidee" => "Décidée",
        "close" => "Classée",
        _ => "Statut inconnu",
    }
}

pub fn ton_statut_procedure(statut: &str) -> TonEtat {
    match statut {
        "engagee" | "convoquee" => TonEtat::Avertissement,
        "decidee" => TonEtat::Ok,
        _ => TonEtat::Neutre,
    }
}

/// Étapes proposées pour une procédure (le serveur revérifie : ordre du contradictoire, délais).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionsProcedure {
    pub observations: bool,
    pub convoquer: bool,
    pub decider: bool,
    pub classer: bool,
}

pub fn actions_procedure(p: &ProcedureDto) -> ActionsProcedure {
    let close = matches!(p.statut.as_str(), "decidee" | "close");
    ActionsProcedure {
        observations: !close && p.observations_recueillies_le.is_none(),
        convoquer: p.instance == "conseil_discipline" && p.statut == "engagee",
        decider: match p.instance.as_str() {
            "chef_etablissement" => {
                p.statut == "engagee" && p.observations_recueillies_le.is_some()
            }
            _ => p.statut == "convoquee",
        },
        classer: !close,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieProcedure {
    pub instance: String,
    pub faits: String,
}

/// Compose le corps de `POST /vs/procedures`. Champs : `proc-instance`, `proc-faits`.
pub fn composer_procedure(
    eleve_id: Uuid,
    s: &SaisieProcedure,
) -> Result<ent_types::discipline::EngagerProcedureBody, Erreurs> {
    let mut e = Erreurs::new();
    if !INSTANCES.iter().any(|(k, _)| *k == s.instance) {
        e.push(err("proc-instance", "Choisissez l'instance."));
    }
    let faits = s.faits.trim();
    if faits.is_empty() {
        e.push(err("proc-faits", "Décrivez les faits reprochés."));
    } else if faits.chars().count() > 500 {
        e.push(err("proc-faits", "Description trop longue (500 caractères au plus)."));
    }
    if e.is_empty() {
        Ok(ent_types::discipline::EngagerProcedureBody {
            eleve_id,
            instance: s.instance.clone(),
            faits: faits.to_string(),
        })
    } else {
        Err(e)
    }
}

/// Convocation du conseil : la date du conseil respecte le délai minimal de convocation de la
/// procédure (garde-fou de saisie ; le serveur reste l'autorité). Champs : `conv-envoi`, `conv-conseil`.
pub fn composer_convocation(
    envoi: &str,
    conseil: &str,
    delai_jours: i32,
) -> Result<(NaiveDate, NaiveDate), Erreurs> {
    let mut e = Erreurs::new();
    let (d, c) = (lire_date(envoi), lire_date(conseil));
    if d.is_none() {
        e.push(err("conv-envoi", "Date d'envoi invalide."));
    }
    if c.is_none() {
        e.push(err("conv-conseil", "Date du conseil invalide."));
    }
    if let (Some(d), Some(c)) = (d, c) {
        if c < d + Duration::days(i64::from(delai_jours)) {
            e.push(err(
                "conv-conseil",
                &format!("Le conseil doit se tenir au moins {delai_jours} jour(s) après l'envoi de la convocation."),
            ));
        }
    }
    match (d, c, e.is_empty()) {
        (Some(d), Some(c), true) => Ok((d, c)),
        _ => Err(e),
    }
}

/// Décision : texte obligatoire (500 caractères au plus) et date valide. Champs : `dec-texte`, `dec-jour`.
pub fn composer_decision(texte: &str, jour: &str) -> Result<(String, NaiveDate), Erreurs> {
    let mut e = Erreurs::new();
    let t = texte.trim();
    if t.is_empty() || t.chars().count() > 500 {
        e.push(err("dec-texte", "Décision obligatoire (500 caractères au plus)."));
    }
    let j = lire_date(jour);
    if j.is_none() {
        e.push(err("dec-jour", "Date de la décision invalide."));
    }
    match (j, e.is_empty()) {
        (Some(j), true) => Ok((t.to_string(), j)),
        _ => Err(e),
    }
}

// ───────────────────────────── Signalements d'assiduité ─────────────────────────────

/// « 2026-11 » (champ `month`) ou « 2026-11-17 » → premier jour du mois.
pub fn lire_mois(texte: &str) -> Option<NaiveDate> {
    let t = texte.trim();
    let (annee, reste) = t.split_once('-')?;
    let mois = reste.split('-').next()?;
    NaiveDate::from_ymd_opt(annee.parse().ok()?, mois.parse().ok()?, 1)
}

pub fn libelle_statut_signalement(statut: &str) -> &'static str {
    match statut {
        "propose" => "Proposé",
        "valide" => "Validé",
        "refuse" => "Refusé",
        "emis" => "Émis",
        _ => "Statut inconnu",
    }
}

pub fn ton_statut_signalement(statut: &str) -> TonEtat {
    match statut {
        "propose" => TonEtat::Avertissement,
        "valide" => TonEtat::Info,
        "emis" => TonEtat::Ok,
        _ => TonEtat::Neutre,
    }
}

/// Transitions proposées : proposé → valider / refuser ; validé → émettre.
pub fn decisions_signalement(statut: &str) -> Vec<(&'static str, &'static str)> {
    match statut {
        "propose" => vec![("valider", "Valider"), ("refuser", "Refuser")],
        "valide" => vec![("emettre", "Émettre le signalement")],
        _ => Vec::new(),
    }
}

/// Résumé « n demi-journées pour un seuil de s ».
pub fn resume_signalement(s: &SignalementDto) -> String {
    format!(
        "{} demi-journée(s) non justifiée(s) pour un seuil de {}",
        s.demi_journees, s.seuil
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        lire_date(s).unwrap()
    }

    fn saisie() -> SaisiePunition {
        SaisiePunition {
            type_id: Uuid::from_u128(1).to_string(),
            jour: "2026-11-05".into(),
            motif: " Bavardages répétés ".into(),
            echeance: String::new(),
            duree: String::new(),
        }
    }

    #[test]
    fn punition_composee() {
        let b = composer_punition(&saisie()).unwrap();
        assert_eq!(b.motif, "Bavardages répétés");
        assert_eq!((b.echeance, b.duree_minutes), (None, None));
        let avec = SaisiePunition {
            echeance: "2026-11-12".into(),
            duree: "60".into(),
            ..saisie()
        };
        let b = composer_punition(&avec).unwrap();
        assert_eq!(b.echeance, Some(d("2026-11-12")));
        assert_eq!(b.duree_minutes, Some(60));
    }

    #[test]
    fn punition_refusee_champ_par_champ() {
        let e = composer_punition(&SaisiePunition::default()).unwrap_err();
        let champs: Vec<&str> = e.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(champs, ["pun-type", "pun-jour", "pun-motif"]);
        let avant = SaisiePunition { echeance: "2026-11-01".into(), ..saisie() };
        assert_eq!(composer_punition(&avant).unwrap_err()[0].0, "pun-echeance");
        let mauvais = SaisiePunition { echeance: "demain".into(), ..saisie() };
        assert!(composer_punition(&mauvais).is_err());
        for duree in ["0", "601", "abc", "-5"] {
            let s = SaisiePunition { duree: duree.into(), ..saisie() };
            assert_eq!(composer_punition(&s).unwrap_err()[0].0, "pun-duree", "{duree}");
        }
        let long = SaisiePunition { motif: "x".repeat(501), ..saisie() };
        assert!(composer_punition(&long).is_err());
    }

    #[test]
    fn statuts_de_punition_et_de_sanction_restent_distincts() {
        assert_eq!(libelle_statut_punition("donnee"), "Donnée");
        assert_eq!(libelle_statut_sanction("active"), "En cours");
        assert!(punition_cloturable("donnee"));
        assert!(!punition_cloturable("effectuee") && !punition_cloturable("annulee"));
        assert_eq!(ton_statut_punition("effectuee"), TonEtat::Ok);
        assert_eq!(libelle_statut_punition("autre"), "Statut inconnu");
        let types = vec![TypeVsDto { id: Uuid::from_u128(1), code: "RET".into(), libelle: "Retenue".into(), actif: true }];
        assert_eq!(libelle_type(&types, Uuid::from_u128(1)), "Retenue");
        assert_eq!(libelle_type(&types, Uuid::nil()), "Type inconnu");
    }

    fn procedure(instance: &str, statut: &str, obs: Option<&str>) -> ProcedureDto {
        ProcedureDto {
            id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            instance: instance.into(),
            statut: statut.into(),
            faits: "faits".into(),
            engagee_le: d("2026-11-01"),
            observations_recueillies_le: obs.map(d),
            convocation_envoyee_le: None,
            conseil_le: None,
            delai_convocation_jours: 8,
            decision: None,
            decision_le: None,
        }
    }

    #[test]
    fn etapes_proposees_selon_la_procedure() {
        // Chef d'établissement : observations d'abord, décision ensuite.
        let a = actions_procedure(&procedure("chef_etablissement", "engagee", None));
        assert!(a.observations && a.classer && !a.decider && !a.convoquer);
        let a = actions_procedure(&procedure("chef_etablissement", "engagee", Some("2026-11-02")));
        assert!(a.decider && !a.observations);
        // Conseil : convocation, puis décision.
        let a = actions_procedure(&procedure("conseil_discipline", "engagee", Some("2026-11-02")));
        assert!(a.convoquer && !a.decider);
        let a = actions_procedure(&procedure("conseil_discipline", "convoquee", Some("2026-11-02")));
        assert!(a.decider && !a.convoquer);
        // Procédure terminée : plus aucune action.
        for statut in ["decidee", "close"] {
            assert_eq!(
                actions_procedure(&procedure("chef_etablissement", statut, Some("2026-11-02"))),
                ActionsProcedure::default()
            );
        }
        assert_eq!(libelle_instance("conseil_discipline"), "Conseil de discipline");
        assert_eq!(libelle_statut_procedure("close"), "Classée");
        assert_eq!(ton_statut_procedure("decidee"), TonEtat::Ok);
    }

    #[test]
    fn procedure_decision_et_convocation_validees() {
        let id = Uuid::from_u128(9);
        let b = composer_procedure(id, &SaisieProcedure { instance: "conseil_discipline".into(), faits: " Violence ".into() }).unwrap();
        assert_eq!((b.eleve_id, b.faits.as_str()), (id, "Violence"));
        let e = composer_procedure(id, &SaisieProcedure::default()).unwrap_err();
        assert_eq!(e.len(), 2);
        // Convocation : respecte le délai minimal.
        assert!(composer_convocation("2026-11-01", "2026-11-09", 8).is_ok());
        let e = composer_convocation("2026-11-01", "2026-11-05", 8).unwrap_err();
        assert!(e[0].1.contains("au moins 8 jour"));
        assert!(composer_convocation("x", "y", 8).is_err());
        // Décision.
        assert_eq!(composer_decision(" Avertissement ", "2026-11-05").unwrap().0, "Avertissement");
        assert!(composer_decision("", "2026-11-05").is_err());
        assert_eq!(composer_decision("ok", "n'importe quoi").unwrap_err()[0].0, "dec-jour");
    }

    #[test]
    fn mois_et_signalements() {
        assert_eq!(lire_mois("2026-11"), Some(d("2026-11-01")));
        assert_eq!(lire_mois("2026-11-17"), Some(d("2026-11-01")));
        assert_eq!(lire_mois("2026-13"), None);
        assert_eq!(lire_mois("novembre"), None);
        assert_eq!(decisions_signalement("propose").len(), 2);
        assert_eq!(decisions_signalement("valide")[0].0, "emettre");
        assert!(decisions_signalement("emis").is_empty() && decisions_signalement("refuse").is_empty());
        assert_eq!(libelle_statut_signalement("emis"), "Émis");
        assert_eq!(ton_statut_signalement("valide"), TonEtat::Info);
        let s = SignalementDto {
            id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            mois: d("2026-11-01"),
            demi_journees: 5,
            seuil: 4,
            statut: "propose".into(),
            dossier: serde_json::Value::Null,
        };
        assert!(resume_signalement(&s).starts_with("5 demi-journée"));
    }
}
