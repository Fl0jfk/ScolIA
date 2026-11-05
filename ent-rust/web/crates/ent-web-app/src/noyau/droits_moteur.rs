//! Droits effectifs du moteur D-017 (`GET /roles/mes-droits`) : **affichage seulement**.
//!
//! Le profil de base (`droits.rs`) ne connaît pas les rôles paramétrés par l'établissement
//! (un professeur à qui l'on a ajouté `absences.declarer`, un secrétaire `referentiel.gerer`…).
//! Le moteur, lui, renvoie l'union du modèle du profil et des rôles attribués : la navigation
//! s'en sert pour proposer les écrans auxquels la personne a réellement droit. Le serveur reste
//! l'autorité : chaque route revérifie, et un refus (403) est traité comme tel.

use ent_web_api::DroitEffectif;

/// Portée « établissement entier » (code du catalogue `/roles/catalogue`).
const ETABLISSEMENT: &str = "etablissement";

/// Droits effectifs de la personne connectée (vide tant que la réponse n'est pas arrivée).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DroitsMoteur {
    droits: Vec<DroitEffectif>,
}

impl DroitsMoteur {
    pub fn new(droits: Vec<DroitEffectif>) -> Self {
        Self { droits }
    }

    /// Possède l'action, quelle que soit la portée (classes, groupes, élèves désignés…).
    pub fn a(&self, action: &str) -> bool {
        self.droits.iter().any(|d| d.action == action)
    }

    /// Possède l'action sur tout l'établissement.
    pub fn a_etablissement(&self, action: &str) -> bool {
        self.droits
            .iter()
            .any(|d| d.action == action && d.portee == ETABLISSEMENT)
    }

    /// Consulter le référentiel (la gestion implique la lecture, comme côté serveur).
    pub fn peut_lire_referentiel(&self) -> bool {
        self.a_etablissement("referentiel.lire") || self.a_etablissement("referentiel.gerer")
    }

    pub fn peut_gerer_referentiel(&self) -> bool {
        self.a_etablissement("referentiel.gerer")
    }

    /// Vérifier les appels non faits ou incohérents, relancer l'enseignant.
    pub fn peut_verifier_appels(&self) -> bool {
        self.a("appel.verifier")
    }

    /// Enregistrer une absence annoncée à l'accueil.
    pub fn peut_declarer_absence(&self) -> bool {
        self.a("absences.declarer")
    }

    /// Donner une punition (professeur sur ses élèves, vie scolaire sur l'établissement).
    pub fn peut_donner_punition(&self) -> bool {
        self.a("punitions.donner")
    }

    /// Procédure disciplinaire, effacement des sanctions.
    pub fn peut_gerer_discipline(&self) -> bool {
        self.a("discipline.gerer")
    }

    /// Proposer ou décider les signalements d'assiduité.
    pub fn peut_signaler_assiduite(&self) -> bool {
        self.a("assiduite.signaler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn droit(action: &str, portee: &str) -> DroitEffectif {
        DroitEffectif {
            action: action.into(),
            portee: portee.into(),
            cibles: Vec::new(),
        }
    }

    #[test]
    fn sans_reponse_rien_n_est_propose() {
        let d = DroitsMoteur::default();
        assert!(!d.peut_lire_referentiel() && !d.peut_gerer_referentiel());
        assert!(!d.peut_verifier_appels() && !d.peut_declarer_absence());
        assert!(!d.peut_donner_punition() && !d.peut_gerer_discipline());
        assert!(!d.peut_signaler_assiduite());
    }

    #[test]
    fn la_gestion_du_referentiel_implique_la_lecture() {
        let d = DroitsMoteur::new(vec![droit("referentiel.gerer", "etablissement")]);
        assert!(d.peut_lire_referentiel() && d.peut_gerer_referentiel());
        let l = DroitsMoteur::new(vec![droit("referentiel.lire", "etablissement")]);
        assert!(l.peut_lire_referentiel() && !l.peut_gerer_referentiel());
    }

    #[test]
    fn le_referentiel_exige_la_portee_etablissement() {
        let d = DroitsMoteur::new(vec![droit("referentiel.gerer", "mes_classes")]);
        assert!(!d.peut_lire_referentiel() && !d.peut_gerer_referentiel());
        // Les actions de vie scolaire sont proposées quelle que soit la portée.
        let p = DroitsMoteur::new(vec![droit("punitions.donner", "mes_classes")]);
        assert!(p.peut_donner_punition() && !p.peut_gerer_discipline());
    }

    #[test]
    fn role_parametre_sans_profil_habilite() {
        // Un professeur à qui l'établissement a ajouté « absences.declarer ».
        let d = DroitsMoteur::new(vec![
            droit("notes.saisir", "mes_classes"),
            droit("absences.declarer", "etablissement"),
        ]);
        assert!(d.peut_declarer_absence() && !d.peut_verifier_appels());
    }
}
