//! Logique d'écran : quelles listes de contraintes proposer selon les droits du moteur.
//! Affichage seulement ; le serveur revérifie chaque audience.

use crate::noyau::droits_moteur::DroitsMoteur;

/// Audiences dont la personne peut lire la liste, dans l'ordre d'affichage.
pub fn audiences_proposees(m: &DroitsMoteur) -> Vec<&'static str> {
    let mut v = Vec::new();
    if m.a("contraintes.lire") {
        v.push("enseignants");
    }
    if m.a_etablissement("repas.gerer") || m.a_etablissement("repas.scanner") {
        v.push("restauration");
    }
    if m.a_etablissement("sorties.gerer") || m.a_etablissement("sorties.controler") {
        v.push("sorties");
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_web_api::DroitEffectif;

    fn droit(action: &str, portee: &str) -> DroitEffectif {
        DroitEffectif {
            action: action.into(),
            portee: portee.into(),
            cibles: Vec::new(),
        }
    }

    #[test]
    fn audiences_selon_les_droits() {
        let prof = DroitsMoteur::new(vec![
            droit("amenagements.lire", "mes_classes"),
            droit("contraintes.lire", "mes_classes"),
        ]);
        assert_eq!(audiences_proposees(&prof), ["enseignants"]);
        let cantine = DroitsMoteur::new(vec![droit("repas.scanner", "etablissement")]);
        assert_eq!(audiences_proposees(&cantine), ["restauration"]);
        let cpe = DroitsMoteur::new(vec![
            droit("contraintes.lire", "etablissement"),
            droit("sorties.controler", "etablissement"),
            droit("repas.scanner", "etablissement"),
        ]);
        assert_eq!(
            audiences_proposees(&cpe),
            ["enseignants", "restauration", "sorties"]
        );
        assert!(audiences_proposees(&DroitsMoteur::default()).is_empty());
        // Un droit limité à des classes n'ouvre ni la restauration ni les sorties.
        let limite = DroitsMoteur::new(vec![droit("repas.gerer", "mes_classes")]);
        assert!(audiences_proposees(&limite).is_empty());
    }
}
