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
}

pub fn droits_du_role(role: &str) -> Droits {
    let Some(r) = Role::depuis(role) else {
        return Droits::default();
    };
    let faire_appel = matches!(
        r,
        Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Prof
    );
    Droits {
        personnel: !matches!(r, Role::Parent | Role::Eleve),
        faire_appel,
        emploi_du_temps: faire_appel || r == Role::Eleve,
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
        assert_eq!(libelle_role("ENSEIGNANT"), "Enseignant");
    }
}
