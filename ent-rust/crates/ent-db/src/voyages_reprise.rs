//! Socle 42, lot V0d : reprise des séjours de Next.js — **règles pures** (`docs/socles/42_SORTIES_VOYAGES.md`
//! §26, §32.1 à §32.4 ; R-SV37 à R-SV40, R-SV46). Aucune base ici ; les commandes sont dans
//! `voyages_reprise_repo.rs`.
//!
//! * un séjour est modifiable dans **un seul outil** à la fois (propriété) ;
//! * la correspondance des statuts suit le tableau du §32.3 ;
//! * un séjour « en vol » (départ entre le gel et la reprise, ou en cours au gel) reste à Next.js jusqu'à
//!   son retour, puis passe en archive ;
//! * aucun rapprochement approximatif : identifiant ScolIA connu, puis INE unique, puis la file humaine.

use chrono::NaiveDate;

pub const STATUTS_SOURCE: [&str; 13] = [
    "BROUILLON",
    "EN_ATTENTE_VALIDATION",
    "EN_ATTENTE_DIR_INITIAL",
    "PROF_LOGISTICS",
    "EN_ATTENTE_BUS_SIGNATURE",
    "EN_ATTENTE_COMPTA",
    "EN_ATTENTE_DIR_FINAL",
    "FINALISE_DIR_ATTENTE_ELEVES",
    "VALIDE",
    "BESOIN_MODIFICATION",
    "REJETE",
    "ANNULE",
    "SEANCE_ANNULEE",
];

/// État cible d'un statut de Next.js (§32.3) et, le cas échéant, un avertissement pour le tableau de bascule.
/// `fin` et `jour` sont des jours de l'établissement : `VALIDE` avec retour passé donne `close` (archive).
/// `precedent` est le `previousStatus` d'un dossier à compléter.
pub fn etat_cible(
    statut: &str,
    precedent: Option<&str>,
    fin: NaiveDate,
    jour: NaiveDate,
) -> Option<(&'static str, Option<&'static str>, Option<&'static str>)> {
    // (état cible, étape de retour pour `a_completer`, avertissement)
    Some(match statut {
        "BROUILLON" => ("brouillon", None, Some("statut hérité BROUILLON")),
        "EN_ATTENTE_VALIDATION" => ("soumise", None, Some("statut hérité EN_ATTENTE_VALIDATION")),
        "EN_ATTENTE_DIR_INITIAL" => ("soumise", None, None),
        "PROF_LOGISTICS" => ("preparation", None, None),
        "EN_ATTENTE_BUS_SIGNATURE" => ("a_signer", None, None),
        "EN_ATTENTE_COMPTA" => ("budget_a_valider", None, None),
        "EN_ATTENTE_DIR_FINAL" => ("a_autoriser", None, None),
        "FINALISE_DIR_ATTENTE_ELEVES" => ("ouverte", None, None),
        "VALIDE" => (if fin < jour { "close" } else { "ouverte" }, None, None),
        "BESOIN_MODIFICATION" => ("a_completer", Some(etape_de_retour(precedent)), None),
        "REJETE" => ("refusee", None, None),
        "ANNULE" | "SEANCE_ANNULEE" => ("annulee", None, None),
        _ => return None,
    })
}

/// Étape d'origine d'une demande de modifications, traduite en état ScolIA (par défaut : `soumise`).
pub fn etape_de_retour(precedent: Option<&str>) -> &'static str {
    match precedent {
        Some("PROF_LOGISTICS") => "preparation",
        Some("EN_ATTENTE_BUS_SIGNATURE") => "a_signer",
        Some("EN_ATTENTE_COMPTA") => "budget_a_valider",
        Some("EN_ATTENTE_DIR_FINAL") => "a_autoriser",
        _ => "soumise",
    }
}

/// Régime d'un séjour à la bascule (§32.1) : `archive` (déjà terminé au gel, ou revenu depuis), `en_vol`
/// (en cours au gel, ou parti entre le gel et `fin_vol`, et pas encore revenu), sinon `normal`.
pub fn regime(
    debut: NaiveDate,
    fin: NaiveDate,
    gel: NaiveDate,
    fin_vol: NaiveDate,
    jour: NaiveDate,
) -> &'static str {
    if fin < gel {
        return "archive";
    }
    let vol = (debut < gel && fin >= gel) || (debut >= gel && debut <= fin_vol);
    if vol {
        if fin < jour {
            "archive"
        } else {
            "en_vol"
        }
    } else {
        "normal"
    }
}

/// INE normalisé (11 caractères alphanumériques, majuscules) ; `None` si le format est inattendu : un INE
/// douteux n'est jamais utilisé pour rapprocher.
pub fn ine_normalise(brut: &str) -> Option<String> {
    let t: String = brut.trim().to_ascii_uppercase();
    (t.len() == 11 && t.chars().all(|c| c.is_ascii_alphanumeric())).then_some(t)
}

/// Résultat d'un rapprochement automatique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rapprochement {
    /// Identifiant ScolIA fourni et connu.
    EleveId,
    /// INE valide et un seul élève correspondant.
    Ine,
    /// Aucun critère sûr : file humaine.
    ARapprocher,
}

/// Décision de rapprochement à partir de faits déjà lus en base : `eleve_id_connu` (l'identifiant fourni
/// existe dans l'établissement) et `nb_par_ine` (nombre d'élèves portant cet INE normalisé).
pub fn rapprocher(eleve_id_connu: bool, ine_valide: bool, nb_par_ine: usize) -> Rapprochement {
    if eleve_id_connu {
        Rapprochement::EleveId
    } else if ine_valide && nb_par_ine == 1 {
        Rapprochement::Ine
    } else {
        Rapprochement::ARapprocher
    }
}

/// Une vérification du tableau de bascule.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Controle {
    pub code: &'static str,
    pub ok: bool,
    pub detail: String,
}

/// Faits d'un séjour utiles au tableau de bascule.
#[derive(Debug, Clone)]
pub struct FaitsBascule {
    pub proprietaire_scolia: bool,
    pub regime: String,
    pub participants_a_rapprocher: i64,
    pub porteur_designe: bool,
    pub ecarts_non_traites: i64,
    pub pieces_attendues: i32,
    pub pieces_copiees: i32,
    pub etat_cible: String,
}

/// Contrôles du tableau de bascule (§32.4). Un séjour est basculable quand **tout** est vert ; la
/// dérogation de la direction ne lève pas « en vol » ni « déjà basculé ».
pub fn controles(f: &FaitsBascule) -> Vec<Controle> {
    vec![
        Controle {
            code: "proprietaire_nextjs",
            ok: !f.proprietaire_scolia,
            detail: if f.proprietaire_scolia { "déjà basculé".into() } else { String::new() },
        },
        Controle {
            code: "hors_vol",
            ok: f.regime != "en_vol",
            detail: if f.regime == "en_vol" { "séjour en vol : fini dans Next.js".into() } else { String::new() },
        },
        Controle {
            code: "participants_rapproches",
            ok: f.participants_a_rapprocher == 0,
            detail: format!("{} à rapprocher", f.participants_a_rapprocher),
        },
        Controle {
            code: "porteur_designe",
            ok: f.porteur_designe,
            detail: if f.porteur_designe { String::new() } else { "porteur à désigner".into() },
        },
        Controle {
            code: "ecarts_traites",
            ok: f.ecarts_non_traites == 0,
            detail: format!("{} écart(s) à traiter", f.ecarts_non_traites),
        },
        Controle {
            code: "pieces_copiees",
            ok: f.pieces_copiees >= f.pieces_attendues,
            detail: format!("{}/{} pièces", f.pieces_copiees, f.pieces_attendues),
        },
    ]
}

/// Contrôles que la dérogation ne peut jamais lever.
pub fn non_derogeable(code: &str) -> bool {
    matches!(code, "proprietaire_nextjs" | "hors_vol" | "participants_rapproches" | "porteur_designe")
}

/// Basculable : tout vert, ou seuls des contrôles dérogeables sont rouges et une dérogation motivée existe.
pub fn basculable(c: &[Controle], derogation: bool) -> bool {
    c.iter().all(|x| x.ok || (derogation && !non_derogeable(x.code)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2027, m, j).unwrap()
    }

    #[test]
    fn correspondance_des_statuts_du_32_3() {
        // Chaque statut connu a un état cible ; un statut inconnu est refusé (jamais deviné).
        for s in STATUTS_SOURCE {
            assert!(etat_cible(s, Some("PROF_LOGISTICS"), d(6, 1), d(4, 1)).is_some(), "{s}");
        }
        assert!(etat_cible("INCONNU", None, d(6, 1), d(4, 1)).is_none());
        assert_eq!(etat_cible("VALIDE", None, d(6, 1), d(4, 1)).unwrap().0, "ouverte");
        assert_eq!(etat_cible("VALIDE", None, d(3, 1), d(4, 1)).unwrap().0, "close");
        assert_eq!(etat_cible("EN_ATTENTE_DIR_INITIAL", None, d(6, 1), d(4, 1)).unwrap().0, "soumise");
        assert_eq!(etat_cible("EN_ATTENTE_COMPTA", None, d(6, 1), d(4, 1)).unwrap().0, "budget_a_valider");
        let (e, retour, _) = etat_cible("BESOIN_MODIFICATION", Some("EN_ATTENTE_COMPTA"), d(6, 1), d(4, 1)).unwrap();
        assert_eq!((e, retour), ("a_completer", Some("budget_a_valider")));
        assert_eq!(etat_cible("BESOIN_MODIFICATION", None, d(6, 1), d(4, 1)).unwrap().1, Some("soumise"));
        assert_eq!(etat_cible("SEANCE_ANNULEE", None, d(6, 1), d(4, 1)).unwrap().0, "annulee");
        assert!(etat_cible("BROUILLON", None, d(6, 1), d(4, 1)).unwrap().2.is_some(), "hérité signalé");
    }

    #[test]
    fn sejour_en_vol_londres_du_19_au_23_avril() {
        // CV-65 : gel le 16 avril, reprise le 3 mai + 2 jours.
        let (gel, fin_vol) = (d(4, 16), d(5, 5));
        assert_eq!(regime(d(4, 19), d(4, 23), gel, fin_vol, d(4, 17)), "en_vol");
        // Revenu : repris en archive par la reprise différentielle.
        assert_eq!(regime(d(4, 19), d(4, 23), gel, fin_vol, d(4, 24)), "archive");
        // Déjà terminé au gel.
        assert_eq!(regime(d(3, 1), d(3, 4), gel, fin_vol, d(4, 17)), "archive");
        // En cours au gel.
        assert_eq!(regime(d(4, 14), d(4, 18), gel, fin_vol, d(4, 17)), "en_vol");
        // Parti après la période de vol : ScolIA.
        assert_eq!(regime(d(5, 20), d(5, 22), gel, fin_vol, d(4, 17)), "normal");
    }

    #[test]
    fn rapprochement_jamais_approximatif() {
        assert_eq!(rapprocher(true, true, 5), Rapprochement::EleveId);
        assert_eq!(rapprocher(false, true, 1), Rapprochement::Ine);
        assert_eq!(rapprocher(false, true, 2), Rapprochement::ARapprocher, "INE partagé : file humaine");
        assert_eq!(rapprocher(false, true, 0), Rapprochement::ARapprocher);
        assert_eq!(rapprocher(false, false, 1), Rapprochement::ARapprocher, "INE invalide jamais utilisé");
        assert_eq!(ine_normalise(" 12345678abc "), Some("12345678ABC".into()));
        assert_eq!(ine_normalise("123"), None);
        assert_eq!(ine_normalise("1234567890é"), None);
    }

    #[test]
    fn la_derogation_ne_leve_que_les_controles_derogeables() {
        let mut f = FaitsBascule {
            proprietaire_scolia: false,
            regime: "normal".into(),
            participants_a_rapprocher: 0,
            porteur_designe: true,
            ecarts_non_traites: 2,
            pieces_attendues: 3,
            pieces_copiees: 1,
            etat_cible: "ouverte".into(),
        };
        let c = controles(&f);
        assert!(!basculable(&c, false));
        assert!(basculable(&c, true), "écarts et pièces se dérogent");
        f.participants_a_rapprocher = 1;
        assert!(!basculable(&controles(&f), true), "un participant non rapproché ne se déroge pas");
        f.participants_a_rapprocher = 0;
        f.regime = "en_vol".into();
        assert!(!basculable(&controles(&f), true), "un séjour en vol ne bascule pas");
    }
}
