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
    // ── R2 : domaines sensibles (D-017) ─────────────────────────────────────
    /// Lire la fiche santé, le PAI et les passages à l'infirmerie d'un élève.
    SanteLire,
    /// Saisir la fiche santé, le PAI et les passages à l'infirmerie.
    SanteSaisir,
    /// Lire le dossier et les entretiens du psychologue scolaire (confidentiel).
    PsyLire,
    /// Saisir le dossier, les entretiens et les rendez-vous du psychologue.
    PsySaisir,
    /// Déposer un document dans le dossier d'un élève (tous tiroirs).
    DocumentsDeposer,
    /// Lire tous les documents d'un élève, sans grant (hors tiroir santé).
    DocumentsLire,
    /// Lire les documents du tiroir santé d'un élève sans grant.
    DocumentsLireSante,
    /// Traiter les demandes d'accès (grants) aux documents d'un élève.
    DocumentsGererDroits,
    /// Prononcer ou annuler une sanction, tenir les commissions de vie scolaire.
    SanctionsPrononcer,
    /// Déclarer un incident de vie scolaire concernant un élève.
    IncidentsDeclarer,
    /// Consulter le journal d'audit de l'établissement (dont les consultations sensibles).
    AuditLire,
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
    pub const TOUTES: [Action; 22] = [
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
        // R2 : domaines sensibles
        Action::SanteLire,
        Action::SanteSaisir,
        Action::PsyLire,
        Action::PsySaisir,
        Action::DocumentsDeposer,
        Action::DocumentsLire,
        Action::DocumentsLireSante,
        Action::DocumentsGererDroits,
        Action::SanctionsPrononcer,
        Action::IncidentsDeclarer,
        Action::AuditLire,
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
            // R2 : domaines sensibles
            Self::SanteLire => "sante.lire",
            Self::SanteSaisir => "sante.saisir",
            Self::PsyLire => "psy.lire",
            Self::PsySaisir => "psy.saisir",
            Self::DocumentsDeposer => "documents.deposer",
            Self::DocumentsLire => "documents.lire",
            Self::DocumentsLireSante => "documents.lire_sante",
            Self::DocumentsGererDroits => "documents.gerer_droits",
            Self::SanctionsPrononcer => "sanctions.prononcer",
            Self::IncidentsDeclarer => "incidents.declarer",
            Self::AuditLire => "audit.lire",
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
            // R2 : domaines sensibles
            Self::SanteLire => "Lire la fiche santé, le PAI et les passages à l'infirmerie",
            Self::SanteSaisir => "Saisir la fiche santé, le PAI et les passages à l'infirmerie",
            Self::PsyLire => "Lire le dossier et les entretiens du psychologue",
            Self::PsySaisir => "Saisir le dossier, les entretiens et les rendez-vous du psychologue",
            Self::DocumentsDeposer => "Déposer des documents dans le dossier de l'élève",
            Self::DocumentsLire => "Lire les documents de l'élève (hors tiroir santé)",
            Self::DocumentsLireSante => "Lire les documents du tiroir santé de l'élève",
            Self::DocumentsGererDroits => "Traiter les demandes d'accès aux documents",
            Self::SanctionsPrononcer => "Prononcer et annuler des sanctions, tenir les commissions",
            Self::IncidentsDeclarer => "Déclarer des incidents de vie scolaire",
            Self::AuditLire => "Consulter le journal d'audit et des consultations sensibles",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|a| a.code() == s.trim())
    }

    /// Portées acceptées pour cette action. Les actions d'administration
    /// n'acceptent que l'établissement entier (garde-fou 4).
    pub fn portees_admises(self) -> &'static [Portee] {
        match self {
            Self::RolesGerer | Self::SecuritePolitique | Self::AuditLire => &[Portee::Etablissement],
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
        // R2 : un professeur déclare un incident sur n'importe quel élève
        // (comportement historique ; à resserrer vers « mes classes » plus tard).
        if role.can_declarer_incident() {
            d.push(etab(IncidentsDeclarer));
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
    // ── R2 : domaines sensibles (miroir exact des `can_*` historiques) ──
    if role.can_saisir_infirmerie() {
        d.push(etab(SanteLire));
        d.push(etab(SanteSaisir));
    }
    if role.can_saisir_psy() {
        d.push(etab(PsyLire));
        d.push(etab(PsySaisir));
    }
    if role.can_manage_documents() {
        d.push(etab(DocumentsDeposer));
        d.push(etab(DocumentsLire));
        d.push(etab(DocumentsGererDroits));
    }
    if role.can_read_sante_docs() {
        d.push(etab(DocumentsLireSante));
    }
    if role.can_saisir_sanctions() {
        d.push(etab(SanctionsPrononcer));
    }
    if role.can_declarer_incident() {
        d.push(etab(IncidentsDeclarer));
    }
    if role.can_manage_eleves() {
        d.push(etab(AuditLire));
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

    /// Non-régression R2 : pour chaque profil historique et chaque action
    /// sensible, le modèle par défaut donne la même décision que l'ancienne
    /// capacité figée `Role::can_*` (et la portée attendue : établissement).
    #[test]
    fn r2_modele_identique_aux_capacites_historiques() {
        use Action::*;
        type Cap = fn(Role) -> bool;
        let table: [(Action, Cap); 11] = [
            (SanteLire, |r| r.can_saisir_infirmerie()),
            (SanteSaisir, |r| r.can_saisir_infirmerie()),
            (PsyLire, |r| r.can_saisir_psy()),
            (PsySaisir, |r| r.can_saisir_psy()),
            (DocumentsDeposer, |r| r.can_manage_documents()),
            (DocumentsLire, |r| r.can_manage_documents()),
            (DocumentsLireSante, |r| r.can_read_sante_docs()),
            (DocumentsGererDroits, |r| r.can_manage_documents()),
            (SanctionsPrononcer, |r| r.can_saisir_sanctions()),
            (IncidentsDeclarer, |r| r.can_declarer_incident()),
            (AuditLire, |r| r.can_manage_eleves()),
        ];
        for r in [
            Role::Admin,
            Role::Direction,
            Role::Administratif,
            Role::Cpe,
            Role::Compta,
            Role::Prof,
            Role::Infirmerie,
            Role::Psy,
            Role::Parent,
            Role::Eleve,
        ] {
            let modele = modele_par_defaut(r);
            for (action, ancienne) in table {
                let droits: Vec<_> = modele.iter().filter(|(a, _)| *a == action).collect();
                // Les familles et élèves n'ont jamais de droit paramétrable.
                let attendu = r.is_staff() && ancienne(r);
                assert_eq!(!droits.is_empty(), attendu, "{r:?} {action:?}");
                assert!(
                    droits.iter().all(|(_, p)| *p == Portee::Etablissement),
                    "{r:?} {action:?} : portée inattendue"
                );
            }
        }
        assert_eq!(Action::parse("sante.lire"), Some(SanteLire));
        assert_eq!(Action::parse("audit.lire"), Some(AuditLire));
        assert_eq!(AuditLire.portees_admises(), &[Portee::Etablissement]);
    }

    #[test]
    fn couverture_des_portees() {
        assert!(Portee::Etablissement.couvre(Portee::MesGroupes));
        assert!(Portee::MesClasses.couvre(Portee::MesClasses));
        assert!(!Portee::MesClasses.couvre(Portee::Etablissement));
        assert!(!Portee::MesGroupes.couvre(Portee::MesClasses));
    }
}
