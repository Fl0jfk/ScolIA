//! Cycle de vie des dossiers d'inscription (`docs/socles/03_INSCRIPTIONS.md` §7.1 à §7.3, lot I2).
//!
//! **Une seule table de transitions**, ici, pour les quatre natures de dossier. Le service
//! (`inscriptions.rs`, `inscriptions_modele.rs`) refuse toute transition qui n'y figure pas ; le
//! SQL (`ent_inscription_etat_valide`, migration 304) ne connaît que les **états** valides par
//! nature, et un test compare les deux listes. Les transitions ne sont volontairement pas
//! contrôlées par la base : elles dépendent de l'auteur, de la campagne et de règles de service.
//!
//! * `admission` et `en_cours_annee` : §7.1 (l'arrivée en cours d'année n'a pas de décision).
//! * `affectes` : §7.2 (pas de décision ; l'établissement ne refuse pas un élève affecté).
//! * `reinscription` : §7.3.
//!
//! Certaines transitions sont **réservées** à une cause précise ([`Cause`]) : on ne valide pas, on
//! ne décide pas, on n'annule pas une validation et on ne bloque pas un dossier par un simple
//! changement d'étape.

/// Natures de dossier (colonne `nature`).
pub const NATURES: [&str; 4] = ["admission", "affectes", "reinscription", "en_cours_annee"];

const ADMISSION: &[&str] = &[
    "brouillon",
    "deposee",
    "en_instruction",
    "attente_famille",
    "liste_attente",
    "admis",
    "refuse",
    "confirme",
    "validee",
    "retire",
    "desiste",
    "sans_suite",
    "bloque_desaccord",
    "annule",
];
const EN_COURS_ANNEE: &[&str] = &[
    "brouillon",
    "deposee",
    "en_instruction",
    "attente_famille",
    "confirme",
    "validee",
    "retire",
    "desiste",
    "sans_suite",
    "bloque_desaccord",
    "annule",
];
const REINSCRIPTION: &[&str] = &[
    "a_confirmer",
    "confirmee",
    "en_attente_passage",
    "validee",
    "non_reinscription_famille",
    "non_reinscription_etablissement",
    "sans_reponse",
    "bloque_desaccord",
    "annule",
];
const AFFECTES: &[&str] = &[
    "importe",
    "a_completer",
    "complete",
    "validee",
    "non_presente",
    "desiste",
    "reaffecte",
    "bloque_desaccord",
    "annule",
];

/// États valides d'une nature (vide si la nature est inconnue).
pub fn etats(nature: &str) -> &'static [&'static str] {
    match nature {
        "admission" => ADMISSION,
        "en_cours_annee" => EN_COURS_ANNEE,
        "reinscription" => REINSCRIPTION,
        "affectes" => AFFECTES,
        _ => &[],
    }
}

/// État qui compte pour les places et les droits : celui d'avant le blocage pour un dossier
/// bloqué pour désaccord (il garde sa place).
pub fn etat_effectif<'a>(etat: &'a str, avant_blocage: Option<&'a str>) -> &'a str {
    if etat == "bloque_desaccord" {
        avant_blocage.unwrap_or(etat)
    } else {
        etat
    }
}

/// État dans lequel naît un dossier de cette nature.
pub fn etat_initial(nature: &str) -> &'static str {
    match nature {
        "reinscription" => "a_confirmer",
        "affectes" => "importe",
        _ => "brouillon",
    }
}

/// Cibles admises depuis `de` (table de §7.1 à §7.3). `bloque_desaccord` ne revient qu'à l'état
/// d'avant le blocage : c'est le service qui le vérifie (voir [`retour_de_blocage_admis`]).
pub fn cibles(nature: &str, de: &str) -> &'static [&'static str] {
    match (nature, de) {
        // ── Admission (§7.1) ──────────────────────────────────────────────────────────────
        ("admission", "brouillon") => &["deposee", "retire", "annule"],
        ("admission", "deposee") => &[
            "en_instruction",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        ("admission", "en_instruction") => &[
            "attente_famille",
            "liste_attente",
            "admis",
            "refuse",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        ("admission", "attente_famille") => &[
            "en_instruction",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        ("admission", "liste_attente") => &[
            "en_instruction",
            "admis",
            "refuse",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        // Une décision se revoit (nouvelle décision, jamais effacée) : §7.1.
        ("admission", "admis") => &[
            "confirme",
            "liste_attente",
            "refuse",
            "desiste",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        ("admission", "refuse") => &["admis", "liste_attente", "en_instruction", "annule"],
        ("admission", "confirme") => &[
            "validee",
            "admis",
            "desiste",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        // Retour à `confirme` : annulation d'une validation erronée seulement (INS-68).
        ("admission", "validee") => &["confirme", "desiste"],
        ("admission", "sans_suite") => &["en_instruction"],
        ("admission", "bloque_desaccord") => &[
            "deposee",
            "en_instruction",
            "attente_famille",
            "liste_attente",
            "admis",
            "confirme",
            "annule",
            "retire",
        ],
        // ── Arrivée en cours d'année (§7.5) : procédure courte, sans décision ───────────────
        ("en_cours_annee", "brouillon") => &["deposee", "retire", "annule"],
        ("en_cours_annee", "deposee") => &[
            "en_instruction",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        ("en_cours_annee", "en_instruction") => &[
            "attente_famille",
            "confirme",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        ("en_cours_annee", "attente_famille") => &[
            "en_instruction",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        ("en_cours_annee", "confirme") => &[
            "validee",
            "desiste",
            "retire",
            "annule",
            "sans_suite",
            "bloque_desaccord",
        ],
        ("en_cours_annee", "validee") => &["confirme", "desiste"],
        ("en_cours_annee", "sans_suite") => &["en_instruction"],
        ("en_cours_annee", "bloque_desaccord") => &[
            "deposee",
            "en_instruction",
            "attente_famille",
            "confirme",
            "annule",
            "retire",
        ],
        // ── Élèves affectés (§7.2) ────────────────────────────────────────────────────────
        ("affectes", "importe") => &["a_completer", "annule"],
        ("affectes", "a_completer") => &[
            "complete",
            "non_presente",
            "desiste",
            "reaffecte",
            "annule",
            "bloque_desaccord",
        ],
        ("affectes", "complete") => &[
            "validee",
            "a_completer",
            "non_presente",
            "desiste",
            "reaffecte",
            "annule",
            "bloque_desaccord",
        ],
        ("affectes", "validee") => &["complete", "non_presente", "desiste"],
        ("affectes", "non_presente") => &["a_completer", "annule"],
        ("affectes", "bloque_desaccord") => &["a_completer", "complete", "annule"],
        // ── Réinscription (§7.3) ──────────────────────────────────────────────────────────
        ("reinscription", "a_confirmer") => &[
            "confirmee",
            "non_reinscription_famille",
            "non_reinscription_etablissement",
            "sans_reponse",
            "annule",
            "bloque_desaccord",
        ],
        ("reinscription", "confirmee") => &[
            "validee",
            "en_attente_passage",
            "non_reinscription_famille",
            "non_reinscription_etablissement",
            "annule",
            "bloque_desaccord",
        ],
        ("reinscription", "en_attente_passage") => &[
            "validee",
            "non_reinscription_famille",
            "non_reinscription_etablissement",
            "annule",
            "bloque_desaccord",
        ],
        ("reinscription", "sans_reponse") => &[
            "a_confirmer",
            "confirmee",
            "non_reinscription_famille",
            "annule",
        ],
        ("reinscription", "non_reinscription_famille") => &["a_confirmer", "annule"],
        ("reinscription", "non_reinscription_etablissement") => &["a_confirmer", "annule"],
        ("reinscription", "validee") => &["confirmee", "non_reinscription_famille"],
        ("reinscription", "bloque_desaccord") => &[
            "a_confirmer",
            "confirmee",
            "en_attente_passage",
            "sans_reponse",
            "annule",
        ],
        // États terminaux (retire, desiste, annule, non_presente…) : aucune sortie.
        _ => &[],
    }
}

/// Transition présente dans la table (sans tenir compte de la cause).
pub fn transition_admise(nature: &str, de: &str, vers: &str) -> bool {
    cibles(nature, de).contains(&vers)
}

/// État qui ne mène nulle part (la démarche est close ; une nouvelle candidature = nouveau dossier).
pub fn est_terminal(nature: &str, etat: &str) -> bool {
    etats(nature).contains(&etat) && cibles(nature, etat).is_empty()
}

/// Dossier encore ouvert : ni validé, ni clos, ni refusé. Sert aux relances, aux prolongations et
/// à l'annonce d'une fermeture. (Un refus reste revisable mais n'attend plus la famille.)
pub fn est_ouvert(nature: &str, etat: &str) -> bool {
    etats(nature).contains(&etat)
        && !est_terminal(nature, etat)
        && !matches!(
            etat,
            "validee" | "refuse" | "sans_suite" | "non_presente" | "brouillon" | "importe"
        )
        && !etat.starts_with("non_reinscription")
}

/// Causes qui réservent une transition. Le service passe sa cause ; une transition réservée à une
/// autre cause est refusée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// Changement d'étape ordinaire (instruction, famille, constat).
    Etape,
    /// Décision de la direction (admis, liste d'attente, refus, revue).
    Decision,
    /// Validation : l'enfant devient élève (appelle les socles 2 et 4).
    Validation,
    /// Annulation d'une validation erronée (commandes inverses).
    AnnulationValidation,
    /// Blocage ou levée pour désaccord des responsables.
    Blocage,
}

/// Cause à laquelle la transition est réservée, s'il y en a une.
pub fn reservee_a(nature: &str, de: &str, vers: &str) -> Option<Cause> {
    if vers == "validee" {
        return Some(Cause::Validation);
    }
    if de == "validee" && matches!(vers, "confirme" | "confirmee" | "complete") {
        return Some(Cause::AnnulationValidation);
    }
    if vers == "bloque_desaccord" || (de == "bloque_desaccord" && !matches!(vers, "annule" | "retire"))
    {
        return Some(Cause::Blocage);
    }
    if nature == "admission"
        && matches!(vers, "admis" | "liste_attente" | "refuse")
        && !(de == "confirme" && vers == "admis")
    {
        return Some(Cause::Decision);
    }
    None
}

/// Toute la vérification d'une transition : présente dans la table ET permise à cette cause.
pub fn verifier_transition(
    nature: &str,
    de: &str,
    vers: &str,
    cause: Cause,
) -> Result<(), &'static str> {
    if !etats(nature).contains(&de) || !etats(nature).contains(&vers) {
        return Err("état inconnu pour cette nature de dossier");
    }
    if !transition_admise(nature, de, vers) {
        return Err("changement d'état impossible depuis l'état actuel");
    }
    match reservee_a(nature, de, vers) {
        Some(c) if c != cause => Err(match c {
            Cause::Validation => "l'état « validée » ne s'obtient que par la validation",
            Cause::AnnulationValidation => {
                "une validation ne se défait que par son annulation motivée"
            }
            Cause::Decision => "une décision se prend par la décision, pas par un changement d'étape",
            Cause::Blocage => "le blocage pour désaccord se pose et se lève par son action dédiée",
            Cause::Etape => "transition réservée",
        }),
        _ => Ok(()),
    }
}

/// À la levée d'un blocage, le dossier ne revient qu'à l'état d'avant (ou se clôt).
pub fn retour_de_blocage_admis(etat_avant: Option<&str>, vers: &str) -> bool {
    vers == "annule" || vers == "retire" || etat_avant == Some(vers)
}

/// Traduction d'un statut historique (colonne `statut`) en état. Reflet de la fonction SQL
/// `ent_inscription_etat_depuis_statut`.
pub fn etat_depuis_statut(nature: &str, statut: &str) -> Option<&'static str> {
    Some(match (nature, statut) {
        ("admission" | "en_cours_annee", "incomplet") => "attente_famille",
        ("admission" | "en_cours_annee", "desistement") => "desiste",
        ("admission" | "en_cours_annee", "brouillon") => "brouillon",
        ("admission" | "en_cours_annee", "deposee") => "deposee",
        ("admission" | "en_cours_annee", "en_instruction") => "en_instruction",
        ("admission", "liste_attente") => "liste_attente",
        ("admission", "admis") => "admis",
        ("admission", "refuse") => "refuse",
        ("admission" | "en_cours_annee", "validee") => "validee",
        ("reinscription", "brouillon") => "a_confirmer",
        ("reinscription", "deposee" | "en_instruction" | "incomplet") => "confirmee",
        ("reinscription", "validee") => "validee",
        ("reinscription", "desistement") => "non_reinscription_famille",
        ("affectes", "brouillon") => "a_completer",
        ("affectes", "deposee" | "en_instruction") => "complete",
        ("affectes", "validee") => "validee",
        ("affectes", "desistement") => "desiste",
        _ => return None,
    })
}

/// État vers lequel revient un dossier dont la validation est annulée.
pub fn etat_apres_annulation_de_validation(nature: &str) -> &'static str {
    match nature {
        "reinscription" => "confirmee",
        "affectes" => "complete",
        _ => "confirme",
    }
}

/// Prochaine échéance de la famille selon l'état atteint (`echeance_famille`, §8.3) : jours à
/// ajouter à la date du jour. `None` = la famille n'a rien à faire.
pub fn delai_famille_jours(etat: &str, delai_reponse_jours: Option<i16>) -> Option<i64> {
    let reponse = i64::from(delai_reponse_jours.unwrap_or(8));
    match etat {
        "attente_famille" | "a_completer" => Some(reponse),
        "admis" => Some(reponse),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toute_cible_est_un_etat_de_la_nature() {
        for n in NATURES {
            for de in etats(n) {
                for vers in cibles(n, de) {
                    assert!(etats(n).contains(vers), "{n}: {de} → {vers} sort de la nature");
                    assert_ne!(de, vers, "{n}: {de} → lui-même");
                }
            }
        }
    }

    #[test]
    fn les_etats_terminaux_n_ont_aucune_sortie() {
        for n in NATURES {
            for e in ["retire", "desiste", "annule", "reaffecte"] {
                if etats(n).contains(&e) {
                    assert!(cibles(n, e).is_empty(), "{n}: {e} doit être terminal");
                }
            }
        }
    }

    #[test]
    fn chaque_etat_non_initial_est_atteignable() {
        for n in NATURES {
            for e in etats(n) {
                if *e == etat_initial(n) {
                    continue;
                }
                assert!(
                    etats(n).iter().any(|de| cibles(n, de).contains(e)),
                    "{n}: {e} n'est atteignable depuis aucun état"
                );
            }
        }
    }

    #[test]
    fn la_validation_est_reservee() {
        assert_eq!(
            verifier_transition("admission", "confirme", "validee", Cause::Etape),
            Err("l'état « validée » ne s'obtient que par la validation")
        );
        assert!(verifier_transition("admission", "confirme", "validee", Cause::Validation).is_ok());
        assert!(
            verifier_transition("admission", "validee", "confirme", Cause::Etape).is_err(),
            "une validation ne se défait pas par un changement d'étape"
        );
        assert!(verifier_transition("admission", "validee", "confirme", Cause::AnnulationValidation).is_ok());
    }

    #[test]
    fn une_decision_ne_passe_pas_par_une_etape() {
        assert!(verifier_transition("admission", "en_instruction", "admis", Cause::Etape).is_err());
        assert!(verifier_transition("admission", "en_instruction", "admis", Cause::Decision).is_ok());
        // Le retour à « admis » depuis « confirmé » (condition de nouveau non remplie) est une étape.
        assert!(verifier_transition("admission", "confirme", "admis", Cause::Etape).is_ok());
    }

    #[test]
    fn le_blocage_a_son_action() {
        assert!(verifier_transition("admission", "en_instruction", "bloque_desaccord", Cause::Etape).is_err());
        assert!(verifier_transition("admission", "en_instruction", "bloque_desaccord", Cause::Blocage).is_ok());
        assert!(verifier_transition("admission", "bloque_desaccord", "annule", Cause::Etape).is_ok());
        assert!(retour_de_blocage_admis(Some("admis"), "admis"));
        assert!(!retour_de_blocage_admis(Some("admis"), "confirme"));
        assert!(retour_de_blocage_admis(Some("admis"), "annule"));
    }

    #[test]
    fn un_etablissement_ne_refuse_pas_un_eleve_affecte() {
        assert!(!etats("affectes").contains(&"refuse"));
        assert!(!etats("en_cours_annee").contains(&"refuse"));
        assert!(!etats("reinscription").contains(&"refuse"));
    }

    #[test]
    fn la_traduction_du_statut_historique_donne_un_etat_valide() {
        for n in NATURES {
            for s in [
                "brouillon", "deposee", "en_instruction", "incomplet", "liste_attente", "admis",
                "refuse", "validee", "desistement",
            ] {
                if let Some(e) = etat_depuis_statut(n, s) {
                    assert!(etats(n).contains(&e), "{n}: {s} → {e}");
                }
            }
        }
    }
}
