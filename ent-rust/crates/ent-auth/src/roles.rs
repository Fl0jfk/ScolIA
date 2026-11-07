//! Rôles établissement — coupe **finale** (pas un bricolage temporaire).
//!
//! Droits fins parent↔élève = flags sur le lien (CONFIGURABLE).
//! Capacités staff = qui peut faire quel geste. Certains rôles existent
//! déjà dans le cast labo avant d'avoir tout leur métier (infirmerie, psy, compta).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Admin,
    Direction,
    Administratif,
    /// Vie scolaire / CPE.
    Cpe,
    Compta,
    Prof,
    Infirmerie,
    Psy,
    Parent,
    Eleve,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Direction => "direction",
            Self::Administratif => "administratif",
            Self::Cpe => "cpe",
            Self::Compta => "compta",
            Self::Prof => "prof",
            Self::Infirmerie => "infirmerie",
            Self::Psy => "psy",
            Self::Parent => "parent",
            Self::Eleve => "eleve",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "admin" => Some(Self::Admin),
            "direction" => Some(Self::Direction),
            "administratif" => Some(Self::Administratif),
            "cpe" | "vie_scolaire" | "vs" => Some(Self::Cpe),
            "compta" => Some(Self::Compta),
            "prof" | "enseignant" => Some(Self::Prof),
            "infirmerie" | "infirmiere" | "infirmier" => Some(Self::Infirmerie),
            "psy" | "psychologue" => Some(Self::Psy),
            "parent" => Some(Self::Parent),
            "eleve" => Some(Self::Eleve),
            "staff" => Some(Self::Administratif),
            _ => None,
        }
    }

    pub fn is_staff(self) -> bool {
        !matches!(self, Self::Parent | Self::Eleve)
    }

    pub fn can_manage_eleves(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif
        )
    }

    pub fn can_manage_matieres(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif
        )
    }

    pub fn can_saisir_notes(self) -> bool {
        matches!(self, Self::Admin | Self::Direction | Self::Prof)
    }

    pub fn can_read_notes_staff(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Prof | Self::Cpe
        )
    }

    pub fn can_declare_absence_staff(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe | Self::Prof
        )
    }

    /// Accepter / refuser un justificatif parent.
    pub fn can_trancher_justificatif(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Remettre présent (erreur d'appel) — annule le fait d'absence.
    pub fn can_annuler_absence_vers_present(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe | Self::Prof
        )
    }

    pub fn can_read_all_eleves(self) -> bool {
        matches!(
            self,
            Self::Admin
                | Self::Direction
                | Self::Administratif
                | Self::Cpe
                | Self::Compta
                | Self::Prof
                | Self::Infirmerie
                | Self::Psy
        )
    }

    pub fn can_read_effectif(self) -> bool {
        self.can_read_all_eleves()
    }

    pub fn can_faire_appel(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe | Self::Prof
        )
    }

    pub fn can_lire_vs_absences(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe | Self::Prof
        )
    }

    /// Déposer / accorder un grant document élève.
    pub fn can_manage_documents(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif
        )
    }

    /// Voir le tiroir santé sans grant (infirmière + manage).
    pub fn can_read_sante_docs(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Infirmerie
        )
    }

    /// Saisir / lister les passages infirmière (note de soins).
    pub fn can_saisir_infirmerie(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Infirmerie
        )
    }

    /// Saisir / lire les notes d'entretiens psy (confidentiel).
    pub fn can_saisir_psy(self) -> bool {
        matches!(self, Self::Admin | Self::Direction | Self::Psy)
    }

    /// Poser une exception EDT (annulation / remplacement / salle).
    pub fn can_manage_edt(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Paramétrer activités repas + poser grille élève.
    pub fn can_manage_repas(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Compta
        )
    }

    /// Scanner badge cantine / lister passages du jour.
    pub fn can_scan_repas(self) -> bool {
        matches!(
            self,
            Self::Admin
                | Self::Direction
                | Self::Administratif
                | Self::Compta
                | Self::Cpe
        )
    }

    /// Affecter / trancher bourses (déduction facture).
    pub fn can_manage_bourses(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Compta
        )
    }

    /// Créer / émettre des factures foyer.
    pub fn can_manage_facturation(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Compta
        )
    }

    /// Ouvrir campagne réinscription / montée rapide (préparation rentrée).
    pub fn can_manage_continuum(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif
        )
    }

    /// Paramétrer types / motifs incidents & sanctions.
    pub fn can_manage_vs_referentiels(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Prononcer / annuler une sanction (VS).
    pub fn can_saisir_sanctions(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Déclarer un incident (prof peut signaler ; sanction = CPE+).
    pub fn can_declarer_incident(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe | Self::Prof
        )
    }

    /// Trancher demandes exceptionnelles + forbid établissement + contrôle badge.
    pub fn can_gerer_sorties(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Scanner badge portail (contrôle sortie).
    pub fn can_controle_sortie(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Chambres / affectations / appel de présence / suivis internat.
    pub fn can_gerer_internat(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Poser / clore une dispense VS (EPS…) côté staff.
    pub fn can_gerer_dispenses_vs(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe | Self::Infirmerie
        )
    }

    /// File d'alertes CPE (hors seuil absences automatique).
    pub fn can_gerer_alertes_cpe(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Publier / annuler des documents à signer.
    pub fn can_publier_docs_a_signer(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Publier une annonce de l'établissement (familles / personnels).
    pub fn can_publier_annonces(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe
        )
    }

    /// Planifier / tenir un conseil de classe + mentions.
    pub fn can_gerer_conseil_classe(self) -> bool {
        matches!(
            self,
            Self::Admin | Self::Direction | Self::Administratif | Self::Cpe | Self::Prof
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacites_par_role() {
        assert!(Role::Cpe.can_trancher_justificatif());
        assert!(!Role::Prof.can_trancher_justificatif());
        assert!(Role::Prof.can_annuler_absence_vers_present());
        assert!(Role::Infirmerie.can_read_all_eleves());
        assert!(!Role::Infirmerie.can_manage_eleves());
        assert!(!Role::Compta.can_saisir_notes());
        assert!(Role::Administratif.can_manage_documents());
        assert!(!Role::Prof.can_manage_documents());
        assert!(Role::Infirmerie.can_read_sante_docs());
        assert!(Role::Infirmerie.can_saisir_infirmerie());
        assert!(!Role::Prof.can_saisir_infirmerie());
        assert!(Role::Psy.can_saisir_psy());
        assert!(!Role::Infirmerie.can_saisir_psy());
        assert!(!Role::Cpe.can_saisir_psy());
        assert!(Role::Cpe.can_manage_edt());
        assert!(!Role::Prof.can_manage_edt());
        assert!(Role::Compta.can_manage_bourses());
        assert!(!Role::Prof.can_manage_bourses());
        assert!(Role::Administratif.can_manage_repas());
        assert!(!Role::Prof.can_manage_repas());
        assert!(Role::Direction.can_manage_continuum());
        assert!(!Role::Prof.can_manage_continuum());
        assert!(!Role::Parent.can_manage_continuum());
        assert!(Role::Cpe.can_saisir_sanctions());
        assert!(!Role::Prof.can_saisir_sanctions());
        assert!(Role::Prof.can_declarer_incident());
        assert!(Role::Cpe.can_gerer_sorties());
        assert!(!Role::Prof.can_gerer_sorties());
        assert!(Role::Cpe.can_controle_sortie());
        assert!(Role::Cpe.can_gerer_internat());
        assert!(!Role::Prof.can_gerer_internat());
        assert!(Role::Infirmerie.can_gerer_dispenses_vs());
        assert!(!Role::Prof.can_gerer_dispenses_vs());
    }
}
