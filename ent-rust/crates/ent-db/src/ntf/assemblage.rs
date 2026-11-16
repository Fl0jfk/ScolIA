//! Socle 8, lot N1 — vérifications de l'assemblage et lint de minimisation (`08_NOTIFICATIONS.md` §6.1).
//!
//! Exécutées au démarrage (installation du consommateur) et par les tests : une nature mal déclarée ne
//! démarre pas. Chaque règle porte son identifiant pour que l'échec dise laquelle est enfreinte.

use super::contrat::*;

fn code_valide(code: &str) -> bool {
    let mut parties = code.split('.');
    let (Some(a), Some(b), None) = (parties.next(), parties.next(), parties.next()) else {
        return false;
    };
    let ok = |s: &str| {
        let mut c = s.chars();
        matches!(c.next(), Some(x) if x.is_ascii_lowercase())
            && s.chars().all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '_')
    };
    ok(a) && ok(b)
}

/// Plus haute sensibilité des variables d'un texte ; `Err(nom)` si une variable n'est pas déclarée.
fn sensibilite_max(texte: &str, g: &Gabarit) -> Result<Sensibilite, String> {
    let mut max = Sensibilite::C0;
    for nom in variables_de(texte) {
        let Some(v) = g.variables.iter().find(|v| v.nom == nom) else {
            return Err(nom.to_string());
        };
        if v.sensibilite > max {
            max = v.sensibilite;
        }
    }
    Ok(max)
}

/// Vérifie les natures et les émissions ; retourne la liste des violations (vide = assemblage valide).
pub fn verifier_assemblage(natures: &[&NatureAvisDecl], avis: &[AvisSurEvenement]) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let mut vus: Vec<&str> = Vec::new();
    for n in natures {
        let c = n.code;
        if !code_valide(c) {
            v.push(format!("{c} : code mal formé (domaine.nature en minuscules)"));
        }
        if vus.contains(&c) {
            v.push(format!("{c} : code déclaré deux fois"));
        }
        vus.push(c);
        if c.split('.').next() != Some(n.brique) {
            v.push(format!("{c} : le code doit être préfixé par la brique « {} »", n.brique));
        }
        if n.version == 0 || n.version > 99 {
            v.push(format!("{c} : version hors de 1..=99"));
        }
        // Gabarit français présent, compilé avec ses variables déclarées.
        let g = &n.gabarit;
        if g.id != c {
            v.push(format!("{c} : le gabarit doit porter l'identifiant de la nature"));
        }
        if g.titre.trim().is_empty() || g.corps.trim().is_empty() || g.sms.trim().is_empty() {
            v.push(format!("{c} : gabarit français incomplet (titre, corps, SMS)"));
        }
        // Lint de minimisation : enveloppes (titre, push, objet, SMS) ≤ C1 ; corps d'e-mail ≤ C2 ; C3 ⇒ enveloppe seule.
        for (partie, texte, plafond) in [
            ("titre", g.titre, Sensibilite::C1),
            ("SMS", g.sms, Sensibilite::C1),
            ("corps", g.corps, Sensibilite::C2),
        ] {
            let plafond = if n.sensibilite == Sensibilite::C3 { Sensibilite::C0 } else { plafond };
            match sensibilite_max(texte, g) {
                Err(nom) => v.push(format!("{c} : variable « {nom} » non déclarée dans le gabarit ({partie})")),
                Ok(s) if s > plafond => v.push(format!(
                    "{c} : lint de minimisation, une variable {} figure dans {partie} (plafond {})",
                    s.code(),
                    plafond.code()
                )),
                Ok(_) => {}
            }
        }
        for var in g.variables {
            if variables_de(g.titre).iter().chain(&variables_de(g.corps)).chain(&variables_de(g.sms)).all(|x| *x != var.nom) {
                v.push(format!("{c} : variable « {} » déclarée mais jamais utilisée", var.nom));
            }
        }
        // U3 réservé aux briques autorisées ; plancher cohérent avec l'urgence.
        if n.urgence == Urgence::U3 && !n.brique_u3_autorisee {
            v.push(format!("{c} : U3 réservé aux briques de sûreté, de santé et de plateforme"));
        }
        if n.urgence >= Urgence::U2 && n.plancher == Plancher::InApp {
            v.push(format!("{c} : plancher « InApp » interdit pour {}", n.urgence.code()));
        }
        if n.urgence == Urgence::U3 && n.plancher != Plancher::Tous {
            v.push(format!("{c} : U3 exige le plancher « Tous »"));
        }
        if !n.canaux_defaut.contains(&Canal::InApp) {
            v.push(format!("{c} : l'ENT (InApp) est toujours remis pour qui a un compte"));
        }
        if n.plancher != Plancher::InApp && !n.canaux_defaut.iter().any(|k| k.externe()) {
            v.push(format!("{c} : le plancher exige un canal externe, aucun n'est déclaré par défaut"));
        }
        let mut canaux = n.canaux_defaut.to_vec();
        canaux.dedup();
        if canaux.len() != n.canaux_defaut.len() {
            v.push(format!("{c} : canal en double"));
        }
        if let Some(e) = &n.escalade {
            if n.urgence == Urgence::U0 && e.vers == Canal::Sms {
                v.push(format!("{c} : escalade vers le SMS interdite pour U0"));
            }
            if !e.vers.externe() {
                v.push(format!("{c} : l'escalade doit viser un canal externe"));
            }
            if e.si_non_lu_apres_min < 5 {
                v.push(format!("{c} : escalade avant 5 minutes"));
            }
        }
        if n.delai_confirmation_min.is_some_and(|d| d > 240) {
            v.push(format!("{c} : délai de confirmation supérieur à 4 heures"));
        }
        // R-N58 : aucune brique ne peut prévenir la famille d'une information préoccupante ou d'un signalement.
        if n.confidentiel_famille && n.sujet == TypeSujet::Eleve {
            v.push(format!("{c} : classe « confidentiel famille » refusée pour un avis qui peut atteindre la famille (R-N58)"));
        }
        if n.texte_libre == TexteLibre::Autorise && !matches!(n.categorie, Categorie::Messagerie | Categorie::Securite) {
            v.push(format!("{c} : texte libre réservé à la messagerie et à l'urgence"));
        }
        for e in n.annulee_par {
            if e.trim().is_empty() {
                v.push(format!("{c} : événement d'annulation vide"));
            }
        }
        if n.actions_emission.iter().any(|a| a.trim().is_empty()) {
            v.push(format!("{c} : action d'émission vide"));
        }
    }
    for a in avis {
        if a.evenement.trim().is_empty() {
            v.push(format!("{} : émission sans événement", a.nature.code));
        }
        if !natures.iter().any(|n| n.code == a.nature.code) {
            v.push(format!("{} : émission d'une nature absente du catalogue", a.nature.code));
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntf::catalogue::{ABSENCE_CONSTATEE, AVIS, NATURES};

    #[test]
    fn le_catalogue_livre_est_valide() {
        let v = verifier_assemblage(NATURES, AVIS);
        assert!(v.is_empty(), "assemblage invalide : {v:#?}");
    }

    fn avec(f: impl FnOnce(&mut NatureAvisDecl)) -> Vec<String> {
        let mut n = ABSENCE_CONSTATEE;
        f(&mut n);
        verifier_assemblage(&[&n], &[])
    }

    #[test]
    fn variable_c2_refusee_dans_un_sms_ou_un_titre() {
        const MOTIF: Variable = Variable { nom: "motif", sensibilite: Sensibilite::C2 };
        const G: Gabarit = Gabarit {
            id: "vie_scolaire.absence_constatee",
            titre: "Absence",
            corps: "Motif : {motif}",
            sms: "Absence : {motif}",
            variables: &[MOTIF],
        };
        let v = avec(|n| n.gabarit = G);
        assert!(v.iter().any(|m| m.contains("lint de minimisation") && m.contains("SMS")), "{v:?}");
        // Dans le corps (e-mail, ENT) une variable C2 est admise.
        let v = avec(|n| n.gabarit = Gabarit { sms: "Absence", ..G });
        assert!(v.is_empty(), "{v:?}");
        // Dans le titre (push, objet d'e-mail) : refusée.
        let v = avec(|n| n.gabarit = Gabarit { titre: "{motif}", sms: "Absence", ..G });
        assert!(v.iter().any(|m| m.contains("titre")), "{v:?}");
    }

    #[test]
    fn c3_n_admet_que_l_enveloppe_seule() {
        let v = avec(|n| n.sensibilite = Sensibilite::C3);
        assert!(v.iter().any(|m| m.contains("lint de minimisation")), "{v:?}");
    }

    #[test]
    fn variable_non_declaree_ou_inutilisee() {
        const G: Gabarit = Gabarit {
            id: "vie_scolaire.absence_constatee",
            titre: "Absence de {inconnue}",
            corps: "x",
            sms: "x",
            variables: &[Variable { nom: "jamais", sensibilite: Sensibilite::C0 }],
        };
        let v = avec(|n| n.gabarit = G);
        assert!(v.iter().any(|m| m.contains("« inconnue » non déclarée")), "{v:?}");
        assert!(v.iter().any(|m| m.contains("« jamais » déclarée mais jamais utilisée")), "{v:?}");
    }

    #[test]
    fn u3_plancher_escalade_et_confidentiel() {
        assert!(avec(|n| n.urgence = Urgence::U3).iter().any(|m| m.contains("U3 réservé")));
        assert!(avec(|n| {
            n.urgence = Urgence::U3;
            n.brique_u3_autorisee = true;
        })
        .iter()
        .any(|m| m.contains("U3 exige le plancher")));
        assert!(avec(|n| n.plancher = Plancher::InApp).iter().any(|m| m.contains("plancher « InApp » interdit")));
        assert!(avec(|n| {
            n.urgence = Urgence::U0;
            n.plancher = Plancher::InApp;
        })
        .iter()
        .any(|m| m.contains("escalade vers le SMS interdite pour U0")));
        assert!(avec(|n| n.confidentiel_famille = true).iter().any(|m| m.contains("R-N58")));
        assert!(avec(|n| n.canaux_defaut = &[Canal::Push]).iter().any(|m| m.contains("InApp")));
        assert!(avec(|n| n.code = "absence").iter().any(|m| m.contains("mal formé")));
        assert!(avec(|n| n.code = "sante.absence_constatee").iter().any(|m| m.contains("préfixé")));
    }

    #[test]
    fn code_double_et_emission_orpheline() {
        let a = ABSENCE_CONSTATEE;
        let v = verifier_assemblage(&[&a, &a], &[]);
        assert!(v.iter().any(|m| m.contains("deux fois")));
        let o = AVIS[0];
        let v = verifier_assemblage(&[], &[o]);
        assert!(v.iter().any(|m| m.contains("absente du catalogue")));
    }

    #[test]
    fn variables_de_lit_les_accolades() {
        assert_eq!(variables_de("a {x} b {y}"), vec!["x", "y"]);
        assert!(variables_de("sans").is_empty());
        assert_eq!(variables_de("{ouvert"), Vec::<&str>::new());
    }
}
