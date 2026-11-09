//! Droits côté interface : **affichage seulement**.
//!
//! Le backend reste l'autorité : chaque route vérifie les droits et l'interface
//! traite les refus (403). Ces règles servent uniquement à ne pas proposer ce qui
//! serait refusé. Elles reproduisent `ent_auth::Role` ; un test de parité
//! (`ent-web-server/tests/droits_parite.rs`) les compare à `ent-auth` pour tous les
//! rôles, sans modifier `ent-auth`.

/// Rôles connus (mêmes alias que `ent_auth::Role::parse`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Admin,
    Direction,
    Administratif,
    Cpe,
    Compta,
    Prof,
    Infirmerie,
    Psy,
    /// Accueil (socle 4, lot A1) : désigne, joint, remet.
    Accueil,
    /// Profil de base minimal (RP1-06) : aucun droit propre, tout vient des rôles paramétrables.
    Personnel,
    Parent,
    Eleve,
}

impl Role {
    pub fn depuis(texte: &str) -> Option<Role> {
        match texte.trim().to_ascii_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "direction" => Some(Role::Direction),
            "administratif" | "staff" => Some(Role::Administratif),
            "cpe" | "vie_scolaire" | "vs" => Some(Role::Cpe),
            "compta" => Some(Role::Compta),
            "prof" | "enseignant" => Some(Role::Prof),
            "infirmerie" | "infirmiere" | "infirmier" => Some(Role::Infirmerie),
            "psy" | "psychologue" => Some(Role::Psy),
            "accueil" => Some(Role::Accueil),
            "personnel" => Some(Role::Personnel),
            "parent" => Some(Role::Parent),
            "eleve" => Some(Role::Eleve),
            _ => None,
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Role::Admin => "Administration",
            Role::Direction => "Direction",
            Role::Administratif => "Secrétariat",
            Role::Cpe => "Vie scolaire",
            Role::Compta => "Comptabilité",
            Role::Prof => "Enseignant",
            Role::Infirmerie => "Infirmerie",
            Role::Psy => "Psychologue",
            Role::Accueil => "Accueil",
            Role::Personnel => "Personnel",
            Role::Parent => "Famille",
            Role::Eleve => "Élève",
        }
    }
}

/// Libellé d'un rôle reçu du serveur (« Utilisateur » si inconnu).
pub fn libelle_role(role: &str) -> &'static str {
    Role::depuis(role)
        .map(Role::libelle)
        .unwrap_or("Utilisateur")
}

/// Ce que l'interface propose à la personne connectée.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Droits {
    /// Personnel de l'établissement (`Role::is_staff`).
    pub personnel: bool,
    /// Faire l'appel (`Role::can_faire_appel`).
    pub faire_appel: bool,
    /// Voir « mon emploi du temps » : route `/edt/mes-creneaux` (élève, ou droit d'appel).
    pub emploi_du_temps: bool,
    /// Lire les absences de l'établissement : route `/vs/absences`
    /// (`Role::can_lire_vs_absences`).
    pub lire_vs_absences: bool,
    /// Accepter ou refuser un justificatif, gérer les signaux d'absence répétée
    /// (`Role::can_trancher_justificatif`).
    pub trancher_justificatif: bool,
    /// Remettre un élève présent après une erreur d'appel
    /// (`Role::can_annuler_absence_vers_present`).
    pub annuler_absence_present: bool,
    /// Portail famille : route `/famille/enfants`, réservée au rôle responsable légal.
    pub portail_famille: bool,
    /// Consulter ses propres absences (compte élève lié à une fiche).
    pub mes_absences: bool,
    /// Voir la liste complète des classes (`/referentiel/classes`) : personnel hors enseignant.
    pub voir_classes: bool,
    /// Gérer le référentiel (`role_peut_gerer_referentiel` : Admin, Direction, Administratif).
    pub gerer_classes: bool,
    /// « Mes classes » : enseignant (classes de ses services).
    pub mes_classes: bool,
    /// Consulter et écrire dans la messagerie : personnel autorisé, famille, élève
    /// (`PgEnt::role_can_messagerie_staff` ou parent / élève).
    pub messagerie: bool,
    /// Messagerie côté personnel : écrire à propos de n'importe quel élève, modèles,
    /// archiver, classer (`PgEnt::role_can_messagerie_staff`).
    pub messagerie_personnel: bool,
    /// Politique de sécurité de l'établissement (passkey obligatoire par rôle) :
    /// `GET`/`PUT /etablissement/securite`, réservé à Admin et Direction.
    pub gerer_securite: bool,
    /// Administration des rôles paramétrables (D-017) : action `roles.gerer`, accordée au
    /// profil Admin et Direction (`modele_par_defaut`). Un rôle paramétré peut aussi la
    /// donner : l'écran revérifie `GET /roles/mes-droits` et le serveur décide.
    pub gerer_roles: bool,
    /// Tenir le cahier de textes (action `cahier.ecrire` du modèle par défaut : Admin,
    /// Direction, Enseignant). Le serveur vérifie la portée (« mes classes »).
    pub cahier_ecrire: bool,
    /// Agenda « à faire » : élève (sur lui-même) et responsable (selon le lien).
    pub agenda: bool,
}

pub fn droits_du_role(role: &str) -> Droits {
    let Some(r) = Role::depuis(role) else {
        return Droits::default();
    };
    let faire_appel = matches!(
        r,
        Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Prof
    );
    // Mêmes rôles que `role_can_messagerie_staff` (ent-db) : ni comptabilité, ni infirmerie, ni psy.
    let messagerie_personnel = faire_appel;
    Droits {
        personnel: !matches!(r, Role::Parent | Role::Eleve),
        faire_appel,
        emploi_du_temps: faire_appel || r == Role::Eleve,
        lire_vs_absences: faire_appel,
        trancher_justificatif: matches!(
            r,
            Role::Admin | Role::Direction | Role::Administratif | Role::Cpe
        ),
        annuler_absence_present: faire_appel,
        portail_famille: r == Role::Parent,
        mes_absences: r == Role::Eleve,
        voir_classes: !matches!(r, Role::Parent | Role::Eleve | Role::Prof),
        gerer_classes: matches!(r, Role::Admin | Role::Direction | Role::Administratif),
        mes_classes: r == Role::Prof,
        messagerie: messagerie_personnel || matches!(r, Role::Parent | Role::Eleve),
        messagerie_personnel,
        gerer_securite: matches!(r, Role::Admin | Role::Direction),
        gerer_roles: matches!(r, Role::Admin | Role::Direction),
        cahier_ecrire: matches!(r, Role::Admin | Role::Direction | Role::Prof),
        agenda: matches!(r, Role::Parent | Role::Eleve),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enseignant_et_famille() {
        let p = droits_du_role("prof");
        assert!(p.personnel && p.faire_appel && p.emploi_du_temps);
        let f = droits_du_role("parent");
        assert!(!f.personnel && !f.faire_appel && !f.emploi_du_temps);
        let e = droits_du_role("eleve");
        assert!(!e.faire_appel && e.emploi_du_temps);
        let c = droits_du_role("compta");
        assert!(c.personnel && !c.faire_appel);
        assert_eq!(droits_du_role("inconnu"), Droits::default());
        // Référentiel : l'enseignant a « Mes classes », la gestion gère, la famille n'a rien.
        assert!(p.mes_classes && !p.voir_classes && !p.gerer_classes);
        let a = droits_du_role("administratif");
        assert!(a.voir_classes && a.gerer_classes && !a.mes_classes);
        assert!(droits_du_role("cpe").voir_classes && !droits_du_role("cpe").gerer_classes);
        assert!(!f.voir_classes && !f.gerer_classes && !f.mes_classes);
        assert!(!e.voir_classes && !e.gerer_classes && !e.mes_classes);
        assert!(p.messagerie && p.messagerie_personnel);
        assert!(f.messagerie && !f.messagerie_personnel);
        assert!(e.messagerie && !e.messagerie_personnel);
        assert!(!c.messagerie && !c.messagerie_personnel);
        assert!(!droits_du_role("infirmerie").messagerie);
        assert_eq!(libelle_role("ENSEIGNANT"), "Enseignant");
    }

    #[test]
    fn droits_vie_scolaire() {
        let cpe = droits_du_role("cpe");
        assert!(cpe.lire_vs_absences && cpe.trancher_justificatif && cpe.annuler_absence_present);
        assert!(!cpe.portail_famille && !cpe.mes_absences);
        // Un enseignant lit les absences et corrige une erreur d'appel, mais ne tranche pas.
        let prof = droits_du_role("prof");
        assert!(
            prof.lire_vs_absences && prof.annuler_absence_present && !prof.trancher_justificatif
        );
        let parent = droits_du_role("parent");
        assert!(
            parent.portail_famille && !parent.lire_vs_absences && !parent.trancher_justificatif
        );
        let eleve = droits_du_role("eleve");
        assert!(eleve.mes_absences && !eleve.portail_famille && !eleve.trancher_justificatif);
        assert!(!droits_du_role("compta").lire_vs_absences);
    }
}
