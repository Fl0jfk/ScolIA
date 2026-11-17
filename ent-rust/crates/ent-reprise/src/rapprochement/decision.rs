//! **Décisions humaines** du rapprochement (lot R3) : ce qu'une personne habilitée peut décider sur une ligne
//! « à décider », les règles de validité de la décision, et la correspondance qu'elle pose. Pur : la trace
//! (qui, quand, quoi) et l'application contre la base sont dans `ent-db`.
//!
//! Une décision est **rejouée** à chaque exécution (elle devient une correspondance enregistrée qui prime) et
//! **réversible** tant que la reprise n'est pas appliquée.

use super::normalisation::{cle_classe, cle_nom};
use super::referentiels;
use super::types::{Candidat, Confiance, Polarite, Raison};
use crate::correspondance::{id_derive, Existante, Statut, SOURCE_NEXT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionDecision {
    /// Cette fiche de l'export est cette fiche de ScolIA.
    Rattacher,
    /// Créer la fiche dans ScolIA (le cœur ne la possède pas).
    Creer,
    /// Ne pas reprendre cette fiche (raison obligatoire).
    Ignorer,
}

impl ActionDecision {
    pub fn code(self) -> &'static str {
        match self {
            ActionDecision::Rattacher => "rattacher",
            ActionDecision::Creer => "creer",
            ActionDecision::Ignorer => "ignorer",
        }
    }

    pub fn depuis(s: &str) -> Option<ActionDecision> {
        Some(match s {
            "rattacher" => ActionDecision::Rattacher,
            "creer" => ActionDecision::Creer,
            "ignorer" => ActionDecision::Ignorer,
            _ => return None,
        })
    }

    pub fn regle(self) -> &'static str {
        match self {
            ActionDecision::Rattacher => "decision_rattacher",
            ActionDecision::Creer => "decision_creer",
            ActionDecision::Ignorer => "decision_ignorer",
        }
    }
}

/// Ce que demande la personne qui décide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionDemandee {
    pub objet: String,
    pub source_id: String,
    pub action: ActionDecision,
    /// Obligatoire pour `Rattacher`, interdit sinon.
    pub cible_id: Option<String>,
    pub motif: Option<String>,
}

/// La proposition telle qu'elle a été présentée (relue en base au moment de décider).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionFile {
    pub objet: String,
    pub source_id: String,
    pub candidats: Vec<Candidat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusDecision {
    ObjetInconnu,
    CibleAbsente,
    CibleNonProposee,
    CibleInterdite,
    MotifObligatoire,
    MotifTropLong,
}

impl RefusDecision {
    /// Message affichable, sans aucune donnée personnelle.
    pub fn message(self) -> &'static str {
        match self {
            RefusDecision::ObjetInconnu => "objet non traité par le rapprochement",
            RefusDecision::CibleAbsente => "il faut désigner la fiche ScolIA à laquelle rattacher",
            RefusDecision::CibleNonProposee => "cette fiche n'est pas proposée pour cette ligne",
            RefusDecision::CibleInterdite => "créer ou ignorer ne désigne aucune fiche ScolIA",
            RefusDecision::MotifObligatoire => "un motif est obligatoire pour cette décision",
            RefusDecision::MotifTropLong => "motif trop long (500 caractères au plus)",
        }
    }
}

impl std::fmt::Display for RefusDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RefusDecision {}

/// Longueur maximale d'un motif de décision (caractères).
pub const MOTIF_MAX: usize = 500;
/// Longueur minimale d'un motif quand il est obligatoire.
pub const MOTIF_MIN: usize = 3;

/// Une proposition peut-elle être décidée **en lot** ? Confiance forte au moins, aucune contradiction
/// (adresse partagée, naissance différente, fiche déjà reliée, ambiguïté…). Un lot ne contient jamais un
/// cas qu'une personne doit regarder.
pub fn eligible_lot(c: &Candidat) -> bool {
    c.confiance >= Confiance::Forte && !c.a_contre()
}

/// Valide une décision demandée contre la proposition présentée. Aucune écriture.
pub fn valider_decision(d: &DecisionDemandee, p: &PropositionFile) -> Result<(), RefusDecision> {
    if !super::est_objet_r3(&d.objet) {
        return Err(RefusDecision::ObjetInconnu);
    }
    let motif = d.motif.as_deref().map(str::trim).filter(|m| !m.is_empty());
    if motif.is_some_and(|m| m.chars().count() > MOTIF_MAX) {
        return Err(RefusDecision::MotifTropLong);
    }
    let motif_ok = motif.is_some_and(|m| m.chars().count() >= MOTIF_MIN);
    match d.action {
        ActionDecision::Rattacher => {
            let id = d.cible_id.as_deref().ok_or(RefusDecision::CibleAbsente)?;
            let c = p
                .candidats
                .iter()
                .find(|c| c.cible_id == id)
                .ok_or(RefusDecision::CibleNonProposee)?;
            // Rattacher contre une contradiction ou sans appui solide se justifie par écrit.
            if (c.a_contre() || c.confiance < Confiance::Forte) && !motif_ok {
                return Err(RefusDecision::MotifObligatoire);
            }
        }
        ActionDecision::Creer => {
            if d.cible_id.is_some() {
                return Err(RefusDecision::CibleInterdite);
            }
            // Créer alors qu'une fiche très probable existe est un doublon en puissance : à justifier.
            let probable = p
                .candidats
                .iter()
                .any(|c| c.confiance >= Confiance::Forte && !c.a_contre());
            if probable && !motif_ok {
                return Err(RefusDecision::MotifObligatoire);
            }
        }
        ActionDecision::Ignorer => {
            if d.cible_id.is_some() {
                return Err(RefusDecision::CibleInterdite);
            }
            if !motif_ok {
                return Err(RefusDecision::MotifObligatoire);
            }
        }
    }
    Ok(())
}

/// La correspondance qu'une décision pose (elle **prime** sur la dérivation à toutes les exécutions
/// suivantes). `etablissement_cible` sert à dériver l'identifiant d'une fiche à créer.
pub fn existante_de_decision(d: &DecisionDemandee, etablissement_cible: &str) -> Existante {
    let derive = id_derive(etablissement_cible, SOURCE_NEXT, &d.objet, &d.source_id);
    match d.action {
        ActionDecision::Rattacher => Existante {
            cible_id: d.cible_id.clone().unwrap_or(derive),
            empreinte_source: None,
            modifie_depuis_reprise: false,
            statut: Statut::Rapprochee,
        },
        ActionDecision::Creer => Existante {
            cible_id: derive,
            empreinte_source: None,
            modifie_depuis_reprise: false,
            statut: Statut::Creee,
        },
        ActionDecision::Ignorer => Existante {
            cible_id: derive,
            empreinte_source: None,
            modifie_depuis_reprise: false,
            statut: Statut::Ignoree,
        },
    }
}

/// Domaine et clé de l'équivalence de libellé à mémoriser quand on rattache un site ou une classe
/// (« relue une fois par l'établissement », T0 §6.3). `None` pour les autres objets.
pub fn cle_synonyme(objet: &str, libelle_source: &str) -> Option<(&'static str, String)> {
    let (domaine, cle) = match objet {
        referentiels::OBJET_CLASSE => ("classe", cle_classe(libelle_source)),
        referentiels::OBJET_SITE => ("site", cle_nom(libelle_source)),
        _ => return None,
    };
    (!cle.is_empty()).then_some((domaine, cle))
}

/// Raisons « pour » d'un candidat, pour la trace d'une décision (codes seulement).
pub fn codes_raisons(c: &Candidat) -> Vec<&'static str> {
    c.raisons.iter().map(|r: &Raison| r.code()).collect()
}

/// Vrai si une raison est une contradiction.
pub fn est_contre(r: Raison) -> bool {
    r.polarite() == Polarite::Contre
}

#[cfg(test)]
mod tests {
    use super::super::types::Resume;
    use super::*;

    fn c(id: &str, conf: Confiance, raisons: &[Raison]) -> Candidat {
        Candidat {
            cible_id: id.into(),
            confiance: conf,
            raisons: raisons.to_vec(),
            resume: Resume::default(),
        }
    }

    fn p(cs: Vec<Candidat>) -> PropositionFile {
        PropositionFile {
            objet: "eleve".into(),
            source_id: "n1".into(),
            candidats: cs,
        }
    }

    fn dem(action: ActionDecision, cible: Option<&str>, motif: Option<&str>) -> DecisionDemandee {
        DecisionDemandee {
            objet: "eleve".into(),
            source_id: "n1".into(),
            action,
            cible_id: cible.map(str::to_string),
            motif: motif.map(str::to_string),
        }
    }

    #[test]
    fn rattacher_a_un_candidat_fort_sans_motif() {
        let prop = p(vec![c("c1", Confiance::Forte, &[Raison::NomIdentique])]);
        assert_eq!(
            valider_decision(&dem(ActionDecision::Rattacher, Some("c1"), None), &prop),
            Ok(())
        );
    }

    #[test]
    fn rattacher_hors_proposition_ou_sans_cible_est_refuse() {
        let prop = p(vec![c("c1", Confiance::Forte, &[])]);
        assert_eq!(
            valider_decision(&dem(ActionDecision::Rattacher, Some("zzz"), None), &prop),
            Err(RefusDecision::CibleNonProposee)
        );
        assert_eq!(
            valider_decision(&dem(ActionDecision::Rattacher, None, None), &prop),
            Err(RefusDecision::CibleAbsente)
        );
    }

    #[test]
    fn rattacher_contre_une_contradiction_ou_un_candidat_faible_exige_un_motif() {
        let prop = p(vec![
            c("c1", Confiance::Forte, &[Raison::NaissanceDifferente]),
            c("c2", Confiance::Faible, &[Raison::NomIdentique]),
        ]);
        for id in ["c1", "c2"] {
            assert_eq!(
                valider_decision(&dem(ActionDecision::Rattacher, Some(id), None), &prop),
                Err(RefusDecision::MotifObligatoire)
            );
            assert_eq!(
                valider_decision(&dem(ActionDecision::Rattacher, Some(id), Some("  ")), &prop),
                Err(RefusDecision::MotifObligatoire)
            );
            assert_eq!(
                valider_decision(
                    &dem(
                        ActionDecision::Rattacher,
                        Some(id),
                        Some("date corrigée par la famille")
                    ),
                    &prop
                ),
                Ok(())
            );
        }
    }

    #[test]
    fn creer_sans_candidat_est_libre_mais_justifie_si_une_fiche_est_probable() {
        assert_eq!(
            valider_decision(&dem(ActionDecision::Creer, None, None), &p(vec![])),
            Ok(())
        );
        let prop = p(vec![c("c1", Confiance::Forte, &[])]);
        assert_eq!(
            valider_decision(&dem(ActionDecision::Creer, None, None), &prop),
            Err(RefusDecision::MotifObligatoire)
        );
        assert_eq!(
            valider_decision(
                &dem(
                    ActionDecision::Creer,
                    None,
                    Some("frère jumeau, pas le même élève")
                ),
                &prop
            ),
            Ok(())
        );
        assert_eq!(
            valider_decision(
                &dem(ActionDecision::Creer, Some("c1"), Some("x y z")),
                &prop
            ),
            Err(RefusDecision::CibleInterdite)
        );
    }

    #[test]
    fn ignorer_exige_toujours_un_motif() {
        let prop = p(vec![]);
        assert_eq!(
            valider_decision(&dem(ActionDecision::Ignorer, None, None), &prop),
            Err(RefusDecision::MotifObligatoire)
        );
        assert_eq!(
            valider_decision(
                &dem(ActionDecision::Ignorer, None, Some("compte de test")),
                &prop
            ),
            Ok(())
        );
        assert_eq!(
            valider_decision(
                &dem(ActionDecision::Ignorer, None, Some(&"x".repeat(501))),
                &prop
            ),
            Err(RefusDecision::MotifTropLong)
        );
    }

    #[test]
    fn objet_hors_perimetre_refuse() {
        let mut d = dem(ActionDecision::Ignorer, None, Some("motif"));
        d.objet = "facture".into();
        assert_eq!(
            valider_decision(&d, &p(vec![])),
            Err(RefusDecision::ObjetInconnu)
        );
    }

    #[test]
    fn lot_seulement_pour_les_cas_forts_sans_contradiction() {
        assert!(eligible_lot(&c(
            "a",
            Confiance::Forte,
            &[Raison::NomIdentique]
        )));
        assert!(!eligible_lot(&c("a", Confiance::Moyenne, &[])));
        assert!(!eligible_lot(&c(
            "a",
            Confiance::Forte,
            &[Raison::EmailPartage]
        )));
        assert!(!eligible_lot(&c("a", Confiance::Faible, &[])));
    }

    #[test]
    fn la_decision_pose_la_correspondance_attendue() {
        let etab = "0e7ab115-0000-4000-8000-00000000f1c7";
        let r = existante_de_decision(&dem(ActionDecision::Rattacher, Some("c1"), None), etab);
        assert_eq!((r.cible_id.as_str(), r.statut), ("c1", Statut::Rapprochee));
        assert!(r.empreinte_source.is_none() && !r.modifie_depuis_reprise);
        let cr = existante_de_decision(&dem(ActionDecision::Creer, None, None), etab);
        assert_eq!(cr.statut, Statut::Creee);
        assert_eq!(cr.cible_id, id_derive(etab, SOURCE_NEXT, "eleve", "n1"));
        let ig = existante_de_decision(&dem(ActionDecision::Ignorer, None, Some("m")), etab);
        assert_eq!(ig.statut, Statut::Ignoree);
    }

    #[test]
    fn synonymes_seulement_pour_classes_et_sites() {
        assert_eq!(
            cle_synonyme("classe_site_mapping", "Seconde B"),
            Some(("classe", cle_classe("2nde B")))
        );
        assert_eq!(
            cle_synonyme("etablissement_site", "Collège"),
            Some(("site", "college".to_string()))
        );
        assert_eq!(cle_synonyme("eleve", "Paul"), None);
        assert_eq!(cle_synonyme("classe_site_mapping", "  "), None);
    }

    #[test]
    fn actions_aller_retour() {
        for a in [
            ActionDecision::Rattacher,
            ActionDecision::Creer,
            ActionDecision::Ignorer,
        ] {
            assert_eq!(ActionDecision::depuis(a.code()), Some(a));
            assert!(a.regle().starts_with("decision_"));
        }
        assert_eq!(ActionDecision::depuis("fusionner"), None);
    }
}
