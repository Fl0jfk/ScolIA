//! Catalogue des actions et portées (D-017, `docs/ROLES_ET_DROITS.md`).
//!
//! ScolIA fournit la coquille : ce catalogue est **fixe** et contrôlé côté
//! serveur. Les établissements composent leurs rôles en cochant des actions,
//! chacune avec une portée. Les modèles par défaut reproduisent exactement les
//! droits des profils historiques ([`Role`]) : la bascule ne change rien tant
//! qu'un établissement n'a pas paramétré ses rôles.

use crate::Role;

/// Action contrôlée par le serveur. Identifiant texte stable (`code`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    /// Lire les notes et moyennes d'un élève.
    NotesLire,
    /// Saisir une note pour un élève.
    NotesSaisir,
    /// Évaluer les compétences et publier le livret.
    CompetencesEvaluer,
    /// Lire les absences, retards et la vie scolaire d'un élève.
    AbsencesLire,
    /// Accepter, refuser ou saisir un justificatif d'absence.
    AbsencesJustifier,
    /// Consulter le dossier d'un élève (sections selon les autres droits).
    DossierLire,
    /// Consulter le journal (historique) d'un élève.
    JournalLire,
    /// Lire et écrire dans les fils de messages à propos d'un élève.
    MessagerieEleve,
    /// Suivi de vie scolaire d'un élève : sorties, dispenses, alertes, conseils,
    /// vœux, informations complémentaires, conventions, internat.
    SuiviEleve,
    /// Créer, modifier, attribuer les rôles de l'établissement.
    RolesGerer,
    /// Politique de sécurité (passkey obligatoire…).
    SecuritePolitique,
    /// Consulter le référentiel scolaire (années, classes, périodes, calendrier, séances).
    ReferentielLire,
    /// Gérer le référentiel scolaire (S2-2, REF-01→14) : structures, bascule d'année.
    ReferentielGerer,
}

/// Sur quoi une action s'applique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Portee {
    /// Tout l'établissement.
    Etablissement,
    /// Élèves suivis par l'enseignement (référentiel, EDT, remplacement en cours).
    MesClasses,
    /// Élèves des groupes et ateliers dont la personne est responsable.
    MesGroupes,
    /// Élèves de classes ou groupes désignés dans le rôle.
    Designes,
}

impl Action {
    pub const TOUTES: [Action; 13] = [
        Action::NotesLire,
        Action::NotesSaisir,
        Action::CompetencesEvaluer,
        Action::AbsencesLire,
        Action::AbsencesJustifier,
        Action::DossierLire,
        Action::JournalLire,
        Action::MessagerieEleve,
        Action::SuiviEleve,
        Action::RolesGerer,
        Action::SecuritePolitique,
        Action::ReferentielLire,
        Action::ReferentielGerer,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::NotesLire => "notes.lire",
            Self::NotesSaisir => "notes.saisir",
            Self::CompetencesEvaluer => "competences.evaluer",
            Self::AbsencesLire => "absences.lire",
            Self::AbsencesJustifier => "absences.justifier",
            Self::DossierLire => "dossier.lire",
            Self::JournalLire => "journal.lire",
            Self::MessagerieEleve => "messagerie.eleve",
            Self::SuiviEleve => "suivi.eleve",
            Self::RolesGerer => "roles.gerer",
            Self::SecuritePolitique => "securite.politique",
            Self::ReferentielLire => "referentiel.lire",
            Self::ReferentielGerer => "referentiel.gerer",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::NotesLire => "Lire les notes",
            Self::NotesSaisir => "Saisir des notes",
            Self::CompetencesEvaluer => "Évaluer les compétences",
            Self::AbsencesLire => "Lire les absences et la vie scolaire",
            Self::AbsencesJustifier => "Traiter les justificatifs d'absence",
            Self::DossierLire => "Consulter le dossier de l'élève",
            Self::JournalLire => "Consulter le journal de l'élève",
            Self::MessagerieEleve => "Échanger avec les familles à propos de l'élève",
            Self::SuiviEleve => "Suivi de vie scolaire (sorties, dispenses, conseils, vœux…)",
            Self::RolesGerer => "Gérer les rôles et leurs attributions",
            Self::SecuritePolitique => "Gérer la politique de sécurité",
            Self::ReferentielLire => "Consulter le référentiel scolaire",
            Self::ReferentielGerer => "Gérer le référentiel scolaire (années, classes, périodes…)",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|a| a.code() == s.trim())
    }

    /// Portées acceptées pour cette action. Les actions d'administration
    /// n'acceptent que l'établissement entier (garde-fou 4).
    pub fn portees_admises(self) -> &'static [Portee] {
        match self {
            Self::RolesGerer
            | Self::SecuritePolitique
            | Self::ReferentielLire
            | Self::ReferentielGerer => &[Portee::Etablissement],
            _ => &[
                Portee::Etablissement,
                Portee::MesClasses,
                Portee::MesGroupes,
                Portee::Designes,
            ],
        }
    }
}

impl Portee {
    pub const TOUTES: [Portee; 4] = [
        Portee::Etablissement,
        Portee::MesClasses,
        Portee::MesGroupes,
        Portee::Designes,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Etablissement => "etablissement",
            Self::MesClasses => "mes_classes",
            Self::MesGroupes => "mes_groupes",
            Self::Designes => "designes",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Etablissement => "Tout l'établissement",
            Self::MesClasses => "Mes classes (enseignement)",
            Self::MesGroupes => "Mes groupes et ateliers (responsable)",
            Self::Designes => "Classes ou groupes désignés",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|p| p.code() == s.trim())
    }

    /// `self` couvre-t-elle `autre` pour l'anti-escalade ? L'établissement
    /// couvre tout ; sinon seulement la même portée (les cibles désignées sont
    /// comparées par l'appelant).
    pub fn couvre(self, autre: Portee) -> bool {
        self == Portee::Etablissement || self == autre
    }
}

/// Droits du modèle par défaut d'un profil historique. Reproduit exactement les
/// capacités de [`Role`] (`can_*`) et la règle « professeur → ses élèves ».
/// Familles et élèves : aucun droit paramétrable (garde-fou 1).
pub fn modele_par_defaut(role: Role) -> Vec<(Action, Portee)> {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    let mut d: Vec<(Action, Portee)> = Vec::new();
    if !role.is_staff() {
        return d;
    }
    // Référentiel scolaire : lecture pour tout le personnel, gestion Admin / Direction /
    // Administratif (équivalent exact de `role_peut_*_referentiel`, S2-2).
    d.push(etab(ReferentielLire));
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif) {
        d.push(etab(ReferentielGerer));
    }
    if role == Role::Prof {
        for a in [
            NotesLire,
            NotesSaisir,
            CompetencesEvaluer,
            AbsencesLire,
            DossierLire,
            JournalLire,
            MessagerieEleve,
            SuiviEleve,
        ] {
            d.push((a, Portee::MesClasses));
        }
        return d;
    }
    if role.can_read_notes_staff() {
        d.push(etab(NotesLire));
    }
    if role.can_saisir_notes() {
        d.push(etab(NotesSaisir));
        d.push(etab(CompetencesEvaluer));
    }
    if role.can_lire_vs_absences() {
        d.push(etab(AbsencesLire));
    }
    if role.can_trancher_justificatif() {
        d.push(etab(AbsencesJustifier));
    }
    if role.can_read_all_eleves() {
        d.push(etab(DossierLire));
        d.push(etab(SuiviEleve));
    }
    if role.can_read_all_eleves() || role.can_manage_eleves() {
        d.push(etab(JournalLire));
    }
    if matches!(
        role,
        Role::Admin | Role::Direction | Role::Administratif | Role::Cpe
    ) {
        d.push(etab(MessagerieEleve));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(RolesGerer));
        d.push(etab(SecuritePolitique));
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_uniques_et_relus() {
        let mut vus = std::collections::HashSet::new();
        for a in Action::TOUTES {
            assert!(vus.insert(a.code()), "code en double : {}", a.code());
            assert_eq!(Action::parse(a.code()), Some(a));
            assert!(!a.libelle().is_empty());
        }
        for p in Portee::TOUTES {
            assert_eq!(Portee::parse(p.code()), Some(p));
        }
        assert_eq!(Action::parse("inconnue"), None);
    }

    #[test]
    fn modeles_par_defaut_fideles_aux_profils() {
        use Action::*;
        let a = |r: Role, x: Action| {
            modele_par_defaut(r)
                .iter()
                .find(|(y, _)| *y == x)
                .map(|(_, p)| *p)
        };
        assert_eq!(a(Role::Prof, NotesLire), Some(Portee::MesClasses));
        assert_eq!(a(Role::Prof, AbsencesJustifier), None);
        assert_eq!(a(Role::Cpe, NotesLire), Some(Portee::Etablissement));
        assert_eq!(a(Role::Cpe, NotesSaisir), None);
        assert_eq!(a(Role::Compta, NotesLire), None);
        assert_eq!(a(Role::Compta, DossierLire), Some(Portee::Etablissement));
        assert_eq!(a(Role::Direction, RolesGerer), Some(Portee::Etablissement));
        assert_eq!(a(Role::Administratif, RolesGerer), None);
        assert!(modele_par_defaut(Role::Parent).is_empty());
        assert!(modele_par_defaut(Role::Eleve).is_empty());
        // Chaque droit d'un modèle respecte les portées admises.
        for r in [
            Role::Admin,
            Role::Direction,
            Role::Administratif,
            Role::Cpe,
            Role::Compta,
            Role::Prof,
            Role::Infirmerie,
            Role::Psy,
        ] {
            for (x, p) in modele_par_defaut(r) {
                assert!(x.portees_admises().contains(&p), "{r:?} {x:?} {p:?}");
            }
        }
    }

    #[test]
    fn couverture_des_portees() {
        assert!(Portee::Etablissement.couvre(Portee::MesGroupes));
        assert!(Portee::MesClasses.couvre(Portee::MesClasses));
        assert!(!Portee::MesClasses.couvre(Portee::Etablissement));
        assert!(!Portee::MesGroupes.couvre(Portee::MesClasses));
    }
}
