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

    /// Infirmerie : lire les passages, le PAI et la fiche d'urgence complète (S2-11).
    pub fn peut_lire_sante(&self) -> bool {
        self.a("sante.lire")
    }

    /// Infirmerie : saisir passages, PAI, contraintes publiées, protocoles.
    pub fn peut_saisir_sante(&self) -> bool {
        self.a("sante.saisir")
    }

    /// Ouvrir et suivre les dispositifs d'accompagnement (PAP, PPS…), définir les aménagements.
    pub fn peut_gerer_accompagnement(&self) -> bool {
        self.a("accompagnement.gerer")
    }

    /// Lire les aménagements des élèves dont on a la charge (sans diagnostic).
    pub fn peut_lire_amenagements(&self) -> bool {
        self.a("amenagements.lire")
    }

    /// Lire les contraintes publiées destinées aux enseignants, à la restauration ou aux sorties.
    pub fn peut_lire_contraintes(&self) -> bool {
        self.a("contraintes.lire")
            || self.a_etablissement("repas.gerer")
            || self.a_etablissement("repas.scanner")
            || self.a_etablissement("sorties.gerer")
            || self.a_etablissement("sorties.controler")
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
        assert!(!d.peut_lire_sante() && !d.peut_saisir_sante());
        assert!(!d.peut_gerer_accompagnement() && !d.peut_lire_amenagements());
        assert!(!d.peut_lire_contraintes());
    }

    #[test]
    fn sante_et_accompagnement_selon_les_actions() {
        let infirmiere = DroitsMoteur::new(vec![
            droit("sante.lire", "etablissement"),
            droit("sante.saisir", "etablissement"),
        ]);
        assert!(infirmiere.peut_lire_sante() && infirmiere.peut_saisir_sante());
        assert!(!infirmiere.peut_gerer_accompagnement());
        // Un professeur : aménagements et contraintes de ses élèves seulement.
        let prof = DroitsMoteur::new(vec![
            droit("amenagements.lire", "mes_classes"),
            droit("contraintes.lire", "mes_classes"),
        ]);
        assert!(prof.peut_lire_amenagements() && prof.peut_lire_contraintes());
        assert!(!prof.peut_lire_sante());
        // La restauration lit les contraintes sans voir la santé.
        let cantine = DroitsMoteur::new(vec![droit("repas.gerer", "etablissement")]);
        assert!(cantine.peut_lire_contraintes() && !cantine.peut_lire_sante());
        // « repas.gerer » limité à des classes ne suffit pas (le catalogue n'admet que l'établissement).
        let limite = DroitsMoteur::new(vec![droit("repas.gerer", "mes_classes")]);
        assert!(!limite.peut_lire_contraintes());
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
