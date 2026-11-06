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
    /// Déclarer une absence à l'accueil (appel ou visite d'un responsable), VS-06.
    AbsencesDeclarer,
    /// Vérifier les appels (non faits, incohérences), relancer, tableau du jour, VS-04/25.
    AppelVerifier,
    /// Donner une punition scolaire (distincte d'une sanction), VS-15.
    PunitionsDonner,
    /// Procédure disciplinaire, effacement des sanctions, décision sur les
    /// signalements d'assiduité, VS-17/18/21.
    DisciplineGerer,
    /// Décompte d'assiduité, règles et proposition de signalement, VS-20/21.
    AssiduiteSignaler,
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
    // ── R2 (D-017) : domaines de gestion — actions d'établissement entier ──
    /// Créer, émettre, encaisser les factures et prélèvements (facturation).
    FacturationGerer,
    /// Consulter la facturation (factures, mandats, rappels) sans émettre.
    FacturationLire,
    /// Affecter et trancher les bourses.
    BoursesGerer,
    /// Gérer l'emploi du temps : exceptions, import, calendrier, contraintes.
    EdtGerer,
    /// Publier ou annuler des annonces de l'établissement.
    AnnoncesPublier,
    /// Paramétrer la restauration (activités, grilles, dispenses, menus).
    RepasGerer,
    /// Scanner les badges de cantine et lister les passages.
    RepasScanner,
    /// Gérer l'internat (chambres, affectations, présence, suivis).
    InternatGerer,
    /// Gérer les sorties (règles, autorisations, demandes exceptionnelles).
    SortiesGerer,
    /// Contrôler les sorties au portail (scan de badge).
    SortiesControler,
    /// Gérer les élèves, foyers, liens et comptes (inscriptions, fiches).
    ElevesGerer,
    /// Gérer les matières et référentiels de compétences.
    MatieresGerer,
    /// Planifier et tenir les conseils de classe.
    ConseilsGerer,
    /// Publier et annuler les documents à signer.
    DocsSignerPublier,
    /// Consulter le référentiel scolaire (années, classes, périodes, calendrier, séances).
    ReferentielLire,
    /// Gérer le référentiel scolaire (S2-2, REF-01→14) : structures, bascule d'année.
    ReferentielGerer,
    /// Créer et modifier les personnes (responsables, contacts), leurs liens
    /// à l'élève et les contacts d'urgence (DOS-03, DOS-05, DOS-08).
    PersonnesGerer,
    /// Désigner les payeurs et leurs parts (DOS-07).
    PayeursDefinir,
    /// Suspendre ou rétablir le lien d'un responsable (décision de justice, DOS-06).
    LiensSuspendre,
    /// Fusionner ou défusionner des personnes en doublon (DOS-15).
    PersonnesFusionner,
    /// Campagnes d'admission / de réinscription et instruction des dossiers
    /// (pièces, étapes, commentaires, relances) — S2-3, INS-01/03/06/15.
    InscriptionsGerer,
    /// Décider d'une admission et valider une inscription ★ (S2-3, INS-04/05).
    InscriptionsDecider,
    /// Importer des élèves en lot et traiter les rejets (S2-3, IOP-01/04).
    ImportsAppliquer,
    /// Lire le cahier de textes, le travail à faire et la charge de travail
    /// d'une classe ou d'un groupe (S2-6, CDT).
    CahierLire,
    /// Renseigner le cahier (contenu de séance, travail, pièces) et corriger les copies.
    CahierEcrire,
    /// Viser le cahier de textes d'une classe ou d'un professeur (inspection, direction).
    CahierViser,
    /// Annuler en masse des cours (grève, fermeture, sortie de niveau), EDT-08 (S2-5).
    EdtAnnulerMasse,
    /// Accorder, révoquer et consulter les délégations de droits datées, ADM-04 (S2-5).
    DelegationsGerer,
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
    pub const TOUTES: [Action; 55] = [
        Action::NotesLire,
        Action::NotesSaisir,
        Action::CompetencesEvaluer,
        Action::AbsencesLire,
        Action::AbsencesJustifier,
        Action::DossierLire,
        Action::JournalLire,
        Action::MessagerieEleve,
        Action::SuiviEleve,
        Action::AbsencesDeclarer,
        Action::AppelVerifier,
        Action::PunitionsDonner,
        Action::DisciplineGerer,
        Action::AssiduiteSignaler,
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
        // R2
        Action::FacturationGerer,
        Action::FacturationLire,
        Action::BoursesGerer,
        Action::EdtGerer,
        Action::AnnoncesPublier,
        Action::RepasGerer,
        Action::RepasScanner,
        Action::InternatGerer,
        Action::SortiesGerer,
        Action::SortiesControler,
        Action::ElevesGerer,
        Action::MatieresGerer,
        Action::ConseilsGerer,
        Action::DocsSignerPublier,
        Action::ReferentielLire,
        Action::ReferentielGerer,
        Action::PersonnesGerer,
        Action::PayeursDefinir,
        Action::LiensSuspendre,
        Action::PersonnesFusionner,
        Action::InscriptionsGerer,
        Action::InscriptionsDecider,
        Action::ImportsAppliquer,
        Action::CahierLire,
        Action::CahierEcrire,
        Action::CahierViser,
        Action::EdtAnnulerMasse,
        Action::DelegationsGerer,
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
            Self::AbsencesDeclarer => "absences.declarer",
            Self::AppelVerifier => "appel.verifier",
            Self::PunitionsDonner => "punitions.donner",
            Self::DisciplineGerer => "discipline.gerer",
            Self::AssiduiteSignaler => "assiduite.signaler",
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
            Self::FacturationGerer => "facturation.gerer",
            Self::FacturationLire => "facturation.lire",
            Self::BoursesGerer => "bourses.gerer",
            Self::EdtGerer => "edt.gerer",
            Self::AnnoncesPublier => "annonces.publier",
            Self::RepasGerer => "repas.gerer",
            Self::RepasScanner => "repas.scanner",
            Self::InternatGerer => "internat.gerer",
            Self::SortiesGerer => "sorties.gerer",
            Self::SortiesControler => "sorties.controler",
            Self::ElevesGerer => "eleves.gerer",
            Self::MatieresGerer => "matieres.gerer",
            Self::ConseilsGerer => "conseils.gerer",
            Self::DocsSignerPublier => "docs_a_signer.publier",
            Self::ReferentielLire => "referentiel.lire",
            Self::ReferentielGerer => "referentiel.gerer",
            Self::PersonnesGerer => "personnes.gerer",
            Self::PayeursDefinir => "payeurs.definir",
            Self::LiensSuspendre => "liens.suspendre",
            Self::PersonnesFusionner => "personnes.fusionner",
            Self::InscriptionsGerer => "inscriptions.gerer",
            Self::InscriptionsDecider => "inscriptions.decider",
            Self::ImportsAppliquer => "imports.appliquer",
            Self::CahierLire => "cahier.lire",
            Self::CahierEcrire => "cahier.ecrire",
            Self::CahierViser => "cahier.viser",
            Self::EdtAnnulerMasse => "edt.annuler_masse",
            Self::DelegationsGerer => "delegations.gerer",
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
            Self::AbsencesDeclarer => "Déclarer une absence à l'accueil",
            Self::AppelVerifier => "Vérifier les appels et suivre la journée de vie scolaire",
            Self::PunitionsDonner => "Donner des punitions scolaires",
            Self::DisciplineGerer => "Conduire la discipline (procédures, effacements, signalements)",
            Self::AssiduiteSignaler => "Suivre l'assiduité et préparer les signalements",
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
            Self::FacturationGerer => "Facturation : créer, émettre, encaisser",
            Self::FacturationLire => "Facturation : consulter sans émettre",
            Self::BoursesGerer => "Affecter et trancher les bourses",
            Self::EdtGerer => "Gérer l'emploi du temps (exceptions, import, contraintes)",
            Self::AnnoncesPublier => "Publier des annonces de l'établissement",
            Self::RepasGerer => "Paramétrer la restauration (activités, grilles, menus)",
            Self::RepasScanner => "Scanner les badges de cantine",
            Self::InternatGerer => "Gérer l'internat (chambres, présence, suivis)",
            Self::SortiesGerer => "Gérer les sorties (règles, autorisations, demandes)",
            Self::SortiesControler => "Contrôler les sorties au portail",
            Self::ElevesGerer => "Gérer les élèves, foyers, liens et comptes",
            Self::MatieresGerer => "Gérer les matières et compétences",
            Self::ConseilsGerer => "Planifier et tenir les conseils de classe",
            Self::DocsSignerPublier => "Publier et annuler les documents à signer",
            Self::ReferentielLire => "Consulter le référentiel scolaire",
            Self::ReferentielGerer => "Gérer le référentiel scolaire (années, classes, périodes…)",
            Self::PersonnesGerer => "Gérer les responsables, contacts et liens d'un élève",
            Self::PayeursDefinir => "Désigner les payeurs et leurs parts",
            Self::LiensSuspendre => "Suspendre ou rétablir le lien d'un responsable",
            Self::PersonnesFusionner => "Fusionner ou défusionner des personnes en doublon",
            Self::InscriptionsGerer => "Gérer les campagnes et instruire les dossiers d'inscription",
            Self::InscriptionsDecider => "Décider des admissions et valider les inscriptions",
            Self::ImportsAppliquer => "Importer des élèves et traiter les rejets d'import",
            Self::CahierLire => "Lire le cahier de textes, le travail à faire et la charge de travail",
            Self::CahierEcrire => "Renseigner le cahier de textes, donner du travail, corriger les copies",
            Self::CahierViser => "Viser le cahier de textes (inspection, direction)",
            Self::EdtAnnulerMasse => "Annuler en masse des cours (grève, fermeture, sortie de niveau)",
            Self::DelegationsGerer => "Déléguer des droits pour une durée limitée",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|a| a.code() == s.trim())
    }

    /// Actions qui accordent aussi celle-ci : `facturation.gerer` permet de lire.
    pub fn accordee_par(self) -> &'static [Action] {
        match self {
            Self::FacturationLire => &[Self::FacturationGerer],
            _ => &[],
        }
    }

    /// Portées acceptées pour cette action. Les actions d'administration
    /// n'acceptent que l'établissement entier (garde-fou 4).
    pub fn portees_admises(self) -> &'static [Portee] {
        match self {
            Self::RolesGerer
            | Self::SecuritePolitique
            | Self::AuditLire
            | Self::ReferentielLire
            | Self::ReferentielGerer
            | Self::CahierViser => &[Portee::Etablissement],
            // R2 : contrôles d'établissement entier aujourd'hui ; les portées par
            // élève viendront avec les écrans (lot ultérieur), jamais en silence.
            Self::FacturationGerer
            | Self::FacturationLire
            | Self::BoursesGerer
            | Self::EdtGerer
            | Self::AnnoncesPublier
            | Self::RepasGerer
            | Self::RepasScanner
            | Self::InternatGerer
            | Self::SortiesGerer
            | Self::SortiesControler
            | Self::ElevesGerer
            | Self::MatieresGerer
            | Self::ConseilsGerer
            | Self::DocsSignerPublier => &[Portee::Etablissement],
            // S2-1 : actions sur les personnes (non rattachées à une classe) :
            // établissement entier seulement.
            Self::PersonnesGerer
            | Self::PayeursDefinir
            | Self::LiensSuspendre
            | Self::PersonnesFusionner
            | Self::EdtAnnulerMasse
            | Self::DelegationsGerer => &[Portee::Etablissement],
            // S2-3 : campagnes, décisions et imports : établissement entier seulement.
            Self::InscriptionsGerer | Self::InscriptionsDecider | Self::ImportsAppliquer => {
                &[Portee::Etablissement]
            }
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
    // Cahier de textes (S2-6) : lecture pour la direction, l'administratif et la vie scolaire ;
    // écriture (contenu, travail, correction des copies) pour la direction et les professeurs
    // seulement : les copies d'élèves ne sont pas lisibles de la vie scolaire ni du secrétariat ;
    // visa réservé à la direction. (L'ancienne route `/cahier-texte` garde ses règles.)
    if matches!(
        role,
        Role::Admin | Role::Direction | Role::Administratif | Role::Cpe
    ) {
        d.push(etab(CahierLire));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(CahierEcrire));
        d.push(etab(CahierViser));
    }
    if role == Role::Prof {
        d.push((CahierLire, Portee::MesClasses));
        d.push((CahierEcrire, Portee::MesClasses));
        for a in [
            NotesLire,
            NotesSaisir,
            CompetencesEvaluer,
            AbsencesLire,
            DossierLire,
            JournalLire,
            MessagerieEleve,
            SuiviEleve,
            PunitionsDonner,
        ] {
            d.push((a, Portee::MesClasses));
        }
        // R2 : un professeur déclare un incident sur n'importe quel élève
        // (comportement historique ; à resserrer vers « mes classes » plus tard).
        if role.can_declarer_incident() {
            d.push(etab(IncidentsDeclarer));
        }
        // R2 : le professeur tient les conseils de classe (contrôle d'établissement).
        d.push(etab(ConseilsGerer));
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
    // Vie scolaire (lot S2-4) : modèles par défaut calés sur les profils déjà
    // habilités à la vie scolaire (déclarer, vérifier, punir) ; la discipline
    // et la décision sur les signalements restent à la direction.
    if matches!(
        role,
        Role::Admin | Role::Direction | Role::Administratif | Role::Cpe
    ) {
        d.push(etab(AbsencesDeclarer));
        d.push(etab(AppelVerifier));
        d.push(etab(PunitionsDonner));
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Cpe) {
        d.push(etab(AssiduiteSignaler));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(DisciplineGerer));
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
    // ── R2 : domaines de gestion. Listes explicites par profil (la table de
    // référence `Role::can_*` sert d'oracle au test de non-régression). ──
    let direction_admin = matches!(role, Role::Admin | Role::Direction | Role::Administratif);
    if direction_admin || role == Role::Compta {
        for a in [FacturationGerer, FacturationLire, BoursesGerer, RepasGerer] {
            d.push(etab(a));
        }
    }
    if direction_admin || matches!(role, Role::Compta | Role::Cpe) {
        d.push(etab(RepasScanner));
    }
    if direction_admin || role == Role::Cpe {
        for a in [
            EdtGerer,
            AnnoncesPublier,
            InternatGerer,
            SortiesGerer,
            SortiesControler,
            DocsSignerPublier,
            ConseilsGerer,
        ] {
            d.push(etab(a));
        }
    }
    if direction_admin {
        d.push(etab(ElevesGerer));
        d.push(etab(MatieresGerer));
    }
    // S2-1 : personnes et liens (mêmes profils que la gestion des élèves) ;
    // payeurs aussi pour la comptabilité ; suspension et fusion : direction.
    if role.can_manage_eleves() {
        d.push(etab(PersonnesGerer));
        d.push(etab(PayeursDefinir));
    }
    if role == Role::Compta {
        d.push(etab(PayeursDefinir));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(LiensSuspendre));
        d.push(etab(PersonnesFusionner));
        // S2-5 : annulation en masse et délégations (EDT-08 ★, ADM-04) : direction.
        d.push(etab(EdtAnnulerMasse));
        d.push(etab(DelegationsGerer));
    }
    // S2-3 : inscriptions (gestion des élèves) ; décision et validation ★ :
    // direction (un rôle paramétré peut l'accorder au secrétariat, D-017).
    if role.can_manage_eleves() {
        d.push(etab(InscriptionsGerer));
        d.push(etab(ImportsAppliquer));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(InscriptionsDecider));
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
        assert_eq!(a(Role::Administratif, PersonnesGerer), Some(Portee::Etablissement));
        assert_eq!(a(Role::Compta, PayeursDefinir), Some(Portee::Etablissement));
        assert_eq!(a(Role::Compta, PersonnesGerer), None);
        assert_eq!(a(Role::Administratif, LiensSuspendre), None);
        assert_eq!(a(Role::Direction, LiensSuspendre), Some(Portee::Etablissement));
        assert_eq!(a(Role::Cpe, PersonnesFusionner), None);
        assert_eq!(a(Role::Prof, PersonnesGerer), None);
        assert_eq!(a(Role::Administratif, InscriptionsGerer), Some(Portee::Etablissement));
        assert_eq!(a(Role::Administratif, InscriptionsDecider), None);
        assert_eq!(a(Role::Direction, InscriptionsDecider), Some(Portee::Etablissement));
        assert_eq!(a(Role::Cpe, InscriptionsGerer), None);
        assert_eq!(a(Role::Compta, ImportsAppliquer), None);
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

    /// R2 : chaque profil × action de gestion donne la même décision que la
    /// capacité figée d'origine (`Role::can_*`, conservée comme oracle).
    #[test]
    fn r2_non_regression_profil_par_action() {
        use Action::*;
        let oracle: [(Action, fn(Role) -> bool); 14] = [
            (FacturationGerer, Role::can_manage_facturation),
            (FacturationLire, Role::can_manage_facturation),
            (BoursesGerer, Role::can_manage_bourses),
            (EdtGerer, Role::can_manage_edt),
            (AnnoncesPublier, Role::can_publier_annonces),
            (RepasGerer, Role::can_manage_repas),
            (RepasScanner, Role::can_scan_repas),
            (InternatGerer, Role::can_gerer_internat),
            (SortiesGerer, Role::can_gerer_sorties),
            (SortiesControler, Role::can_controle_sortie),
            (ElevesGerer, Role::can_manage_eleves),
            (MatieresGerer, Role::can_manage_matieres),
            (ConseilsGerer, Role::can_gerer_conseil_classe),
            (DocsSignerPublier, Role::can_publier_docs_a_signer),
        ];
        for role in [
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
            let modele = modele_par_defaut(role);
            for (action, ancien) in oracle {
                let nouveau = modele.contains(&(action, Portee::Etablissement));
                assert_eq!(nouveau, ancien(role), "{role:?} × {}", action.code());
                assert_eq!(action.portees_admises(), &[Portee::Etablissement]);
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
