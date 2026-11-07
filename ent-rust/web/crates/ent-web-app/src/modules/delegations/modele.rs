//! Logique de l'écran « Délégations » (pure, sans Leptos ni réseau, testée en natif) : libellés
//! d'état, liste fermée des droits proposés, tri et contrôle de la saisie. Le serveur reste
//! l'autorité : on ne délègue que ce que l'on détient, jamais `roles.gerer`, `securite.politique`
//! ni `delegations.gerer`, et la durée est plafonnée (366 jours).

use chrono::NaiveDate;
use ent_types::edt_s2::{DelegationDto, DelegationSaisieDto, DroitDelegueDto};
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::noyau::temps::date_complete;

/// Erreurs de saisie : (identifiant du champ, message), pour `ResumeErreurs`.
pub type Erreurs = Vec<(String, String)>;

/// Portée proposée : l'établissement (les actions délégables n'en admettent pas d'autre).
pub const PORTEE_ETABLISSEMENT: &str = "etablissement";

/// Durée maximale d'une délégation (jours), comme le serveur.
pub const DUREE_MAX_JOURS: i64 = 366;

/// Longueur maximale du motif (caractères).
pub const MOTIF_MAX: usize = 200;

/// Droits proposés : liste **fermée** d'actions sûres du catalogue (code, libellé). Les autres
/// actions du catalogue ne se délèguent pas depuis cet écran.
pub const ACTIONS_DELEGABLES: [(&str, &str); 5] = [
    ("edt.gerer", "Gérer l'emploi du temps et les remplacements"),
    ("absences.lire", "Consulter les absences et la vie scolaire"),
    ("absences.declarer", "Enregistrer les absences à l'accueil"),
    ("appel.verifier", "Vérifier les appels et suivre la journée"),
    ("annonces.publier", "Publier des annonces"),
];

/// Libellé d'une action ; le code lui-même si elle n'est pas dans la liste fermée.
pub fn libelle_action(code: &str) -> String {
    ACTIONS_DELEGABLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| (*l).to_string())
        .unwrap_or_else(|| code.to_string())
}

pub fn libelle_etat(etat: &str) -> &'static str {
    match etat {
        "active" => "Active",
        "a_venir" => "À venir",
        "expiree" => "Expirée",
        "revoquee" => "Révoquée",
        _ => "État inconnu",
    }
}

pub fn ton_etat(etat: &str) -> TonEtat {
    match etat {
        "active" => TonEtat::Ok,
        "a_venir" => TonEtat::Info,
        "revoquee" => TonEtat::Mal,
        _ => TonEtat::Neutre,
    }
}

pub fn libelle_source(source: &str) -> &'static str {
    match source {
        "manuelle" => "Accordée par la direction",
        "remplacement" => "Liée à un remplacement",
        _ => "Origine inconnue",
    }
}

/// « Du lundi 9 novembre 2026 au vendredi 20 novembre 2026 ».
pub fn libelle_periode(d: &DelegationDto) -> String {
    format!(
        "Du {} au {}",
        date_complete(d.debut).to_lowercase(),
        date_complete(d.fin).to_lowercase()
    )
}

/// Droits d'une délégation en une phrase, séparés par des virgules.
pub fn libelle_droits(d: &DelegationDto) -> String {
    let mut libelles: Vec<String> = d.droits.iter().map(|x| libelle_action(&x.action)).collect();
    libelles.dedup();
    if libelles.is_empty() {
        "Aucun droit".into()
    } else {
        libelles.join(", ")
    }
}

/// Une délégation peut encore être révoquée tant qu'elle n'est ni expirée ni révoquée.
pub fn peut_revoquer(d: &DelegationDto) -> bool {
    matches!(d.etat.as_str(), "active" | "a_venir")
}

/// Rang d'affichage : actives, à venir, puis expirées et révoquées.
fn rang(etat: &str) -> u8 {
    match etat {
        "active" => 0,
        "a_venir" => 1,
        "expiree" => 2,
        "revoquee" => 3,
        _ => 4,
    }
}

/// Actives d'abord, puis à venir, expirées, révoquées ; à égalité, fin la plus proche d'abord.
pub fn trier(mut liste: Vec<DelegationDto>) -> Vec<DelegationDto> {
    liste.sort_by(|a, b| {
        (rang(&a.etat), a.fin, a.debut, a.id).cmp(&(rang(&b.etat), b.fin, b.debut, b.id))
    });
    liste
}

/// Nombre de délégations en cours (actives).
pub fn nb_actives(liste: &[DelegationDto]) -> usize {
    liste.iter().filter(|d| d.etat == "active").count()
}

/// Champs saisis du formulaire d'octroi.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieDelegation {
    /// Identifiant (texte) de la personne qui reçoit les droits.
    pub delegue: String,
    /// Codes d'actions cochés.
    pub actions: Vec<String>,
    /// Facultatif : aujourd'hui par défaut.
    pub debut: String,
    /// Obligatoire.
    pub fin: String,
    pub motif: String,
}

fn erreur(champ: &str, message: &str) -> (String, String) {
    (champ.to_string(), message.to_string())
}

fn lire_date(texte: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texte.trim(), "%Y-%m-%d").ok()
}

/// Contrôle local de la saisie (confort : le serveur revérifie tout) et composition du corps.
pub fn composer(
    saisie: &SaisieDelegation,
    aujourdhui: NaiveDate,
) -> Result<DelegationSaisieDto, Erreurs> {
    let mut erreurs: Erreurs = Vec::new();
    let delegue = saisie.delegue.trim().parse::<Uuid>().ok();
    if delegue.is_none() {
        erreurs.push(erreur(
            "dlg-personne",
            "Choisissez la personne qui reçoit les droits.",
        ));
    }
    let mut actions: Vec<&str> = Vec::new();
    for code in &saisie.actions {
        if ACTIONS_DELEGABLES.iter().any(|(c, _)| c == code) && !actions.contains(&code.as_str()) {
            actions.push(code);
        }
    }
    if actions.is_empty() {
        erreurs.push(erreur(
            "dlg-actions",
            "Cochez au moins un droit à déléguer.",
        ));
    }
    let debut = if saisie.debut.trim().is_empty() {
        None
    } else {
        match lire_date(&saisie.debut) {
            Some(d) if d < aujourdhui => {
                erreurs.push(erreur(
                    "dlg-debut",
                    "Le début ne peut pas être dans le passé.",
                ));
                None
            }
            Some(d) => Some(d),
            None => {
                erreurs.push(erreur("dlg-debut", "Date de début illisible."));
                None
            }
        }
    };
    let fin = if saisie.fin.trim().is_empty() {
        erreurs.push(erreur("dlg-fin", "La date de fin est obligatoire."));
        None
    } else {
        match lire_date(&saisie.fin) {
            None => {
                erreurs.push(erreur("dlg-fin", "Date de fin illisible."));
                None
            }
            Some(f) => {
                let depart = debut.unwrap_or(aujourdhui);
                if f < depart {
                    erreurs.push(erreur("dlg-fin", "La fin ne peut pas précéder le début."));
                    None
                } else if (f - depart).num_days() > DUREE_MAX_JOURS {
                    erreurs.push(erreur(
                        "dlg-fin",
                        &format!("Une délégation dure {DUREE_MAX_JOURS} jours au plus."),
                    ));
                    None
                } else {
                    Some(f)
                }
            }
        }
    };
    let motif = saisie.motif.trim();
    if motif.is_empty() {
        erreurs.push(erreur("dlg-motif", "Indiquez le motif de la délégation."));
    } else if motif.chars().count() > MOTIF_MAX {
        erreurs.push(erreur(
            "dlg-motif",
            &format!("Le motif ne peut pas dépasser {MOTIF_MAX} caractères."),
        ));
    }
    match (delegue, fin, erreurs.is_empty()) {
        (Some(delegue_user_id), Some(fin), true) => Ok(DelegationSaisieDto {
            delegue_user_id,
            droits: actions
                .into_iter()
                .map(|a| DroitDelegueDto {
                    action: a.to_string(),
                    portee: PORTEE_ETABLISSEMENT.to_string(),
                    cibles: Vec::new(),
                })
                .collect(),
            debut,
            fin,
            motif: motif.to_string(),
        }),
        _ => Err(erreurs),
    }
}

/// Phrase de confirmation avant l'octroi.
pub fn resume_octroi(nom: &str, saisie: &DelegationSaisieDto) -> String {
    let droits: Vec<String> = saisie
        .droits
        .iter()
        .map(|d| libelle_action(&d.action))
        .collect();
    let debut = saisie
        .debut
        .map(|d| format!("du {} ", date_complete(d).to_lowercase()))
        .unwrap_or_default();
    format!(
        "{nom} recevra, {debut}jusqu'au {} inclus, les droits suivants sur tout l'établissement : {}.",
        date_complete(saisie.fin).to_lowercase(),
        droits.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn delegation(n: u128, etat: &str, fin: NaiveDate) -> DelegationDto {
        DelegationDto {
            id: Uuid::from_u128(n),
            delegue_user_id: Uuid::from_u128(100),
            delegue_nom: Some("Mme Chourave".into()),
            delegant_user_id: Uuid::from_u128(200),
            delegant_nom: Some("Direction".into()),
            droits: vec![DroitDelegueDto {
                action: "edt.gerer".into(),
                portee: "etablissement".into(),
                cibles: vec![],
            }],
            debut: d(2026, 11, 9),
            fin,
            motif: "Sortie".into(),
            source: "manuelle".into(),
            etat: etat.into(),
        }
    }

    fn saisie() -> SaisieDelegation {
        SaisieDelegation {
            delegue: Uuid::from_u128(5).to_string(),
            actions: vec!["edt.gerer".into()],
            debut: String::new(),
            fin: "2026-11-20".into(),
            motif: " Sortie scolaire ".into(),
        }
    }

    #[test]
    fn libelles() {
        assert_eq!(libelle_etat("active"), "Active");
        assert_eq!(libelle_etat("a_venir"), "À venir");
        assert_eq!(libelle_etat("expiree"), "Expirée");
        assert_eq!(libelle_etat("revoquee"), "Révoquée");
        assert_eq!(libelle_etat("x"), "État inconnu");
        assert_eq!(libelle_source("manuelle"), "Accordée par la direction");
        assert_eq!(libelle_source("remplacement"), "Liée à un remplacement");
        assert_eq!(ton_etat("active"), TonEtat::Ok);
        assert_eq!(
            libelle_action("edt.gerer"),
            "Gérer l'emploi du temps et les remplacements"
        );
        assert_eq!(libelle_action("inconnu.x"), "inconnu.x");
        let dg = delegation(1, "active", d(2026, 11, 20));
        assert_eq!(
            libelle_periode(&dg),
            "Du lundi 9 novembre 2026 au vendredi 20 novembre 2026"
        );
        assert_eq!(
            libelle_droits(&dg),
            "Gérer l'emploi du temps et les remplacements"
        );
    }

    #[test]
    fn tri_actives_d_abord_puis_fin_proche() {
        let liste = vec![
            delegation(1, "revoquee", d(2026, 11, 12)),
            delegation(2, "active", d(2026, 12, 1)),
            delegation(3, "expiree", d(2026, 10, 1)),
            delegation(4, "active", d(2026, 11, 15)),
            delegation(5, "a_venir", d(2026, 12, 20)),
        ];
        let ids: Vec<u128> = trier(liste.clone())
            .iter()
            .map(|x| x.id.as_u128())
            .collect();
        assert_eq!(ids, [4, 2, 5, 3, 1]);
        assert_eq!(nb_actives(&liste), 2);
        assert!(peut_revoquer(&liste[1]) && peut_revoquer(&liste[4]));
        assert!(!peut_revoquer(&liste[0]) && !peut_revoquer(&liste[2]));
    }

    #[test]
    fn saisie_valide() {
        let corps = composer(&saisie(), d(2026, 11, 9)).expect("valide");
        assert_eq!(corps.fin, d(2026, 11, 20));
        assert_eq!(corps.debut, None);
        assert_eq!(corps.motif, "Sortie scolaire");
        assert_eq!(corps.droits.len(), 1);
        assert_eq!(corps.droits[0].portee, "etablissement");
        assert!(corps.droits[0].cibles.is_empty());
    }

    #[test]
    fn la_fin_est_obligatoire() {
        let mut s = saisie();
        s.fin = "  ".into();
        let e = composer(&s, d(2026, 11, 9)).unwrap_err();
        assert!(
            e.iter()
                .any(|(c, m)| c == "dlg-fin" && m.contains("obligatoire")),
            "{e:?}"
        );
        s.fin = "pas une date".into();
        assert!(composer(&s, d(2026, 11, 9)).is_err());
    }

    #[test]
    fn dates_coherentes_et_duree_plafonnee() {
        let auj = d(2026, 11, 9);
        let mut s = saisie();
        s.debut = "2026-11-01".into();
        assert!(composer(&s, auj).is_err(), "début dans le passé");
        s.debut = "2026-11-15".into();
        s.fin = "2026-11-10".into();
        assert!(composer(&s, auj).is_err(), "fin avant le début");
        s.debut = String::new();
        s.fin = "2026-11-08".into();
        assert!(composer(&s, auj).is_err(), "fin avant aujourd'hui");
        s.fin = "2026-11-09".into();
        assert!(composer(&s, auj).is_ok(), "un seul jour");
        s.fin = "2027-11-10".into();
        assert!(composer(&s, auj).is_ok(), "366 jours");
        s.fin = "2027-11-11".into();
        assert!(composer(&s, auj).is_err(), "367 jours");
        s.debut = "2026-11-10".into();
        s.fin = "2027-11-11".into();
        assert!(composer(&s, auj).is_ok(), "durée comptée depuis le début");
    }

    #[test]
    fn personne_droits_et_motif_obligatoires() {
        let auj = d(2026, 11, 9);
        let mut s = saisie();
        s.delegue = String::new();
        s.actions.clear();
        s.motif = "  ".into();
        let e = composer(&s, auj).unwrap_err();
        let champs: Vec<&str> = e.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(champs, ["dlg-personne", "dlg-actions", "dlg-motif"]);
        // Une action hors de la liste fermée est ignorée (jamais envoyée).
        let mut s = saisie();
        s.actions = vec!["roles.gerer".into(), "delegations.gerer".into()];
        assert!(composer(&s, auj).is_err());
        s.actions = vec!["roles.gerer".into(), "edt.gerer".into(), "edt.gerer".into()];
        let corps = composer(&s, auj).unwrap();
        assert_eq!(
            corps.droits.len(),
            1,
            "doublons et actions interdites écartés"
        );
        // Motif trop long.
        let mut s = saisie();
        s.motif = "x".repeat(MOTIF_MAX + 1);
        assert!(composer(&s, auj).is_err());
    }

    #[test]
    fn la_liste_fermee_ne_contient_aucune_action_interdite() {
        for (code, _) in ACTIONS_DELEGABLES {
            assert!(
                !["roles.gerer", "securite.politique", "delegations.gerer"].contains(&code),
                "{code}"
            );
        }
    }

    #[test]
    fn resume_de_confirmation() {
        let corps = composer(&saisie(), d(2026, 11, 9)).unwrap();
        let r = resume_octroi("Mme Chourave", &corps);
        assert!(r.starts_with("Mme Chourave recevra, jusqu'au vendredi 20 novembre 2026 inclus"));
        assert!(r.contains("Gérer l'emploi du temps et les remplacements"));
    }
}
