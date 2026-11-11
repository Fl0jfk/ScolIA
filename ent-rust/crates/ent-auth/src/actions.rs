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
    /// Lire les fils privés des parents et des élèves (M0b, D26) : réservé à qui traite les
    /// messages des familles (direction, CPE, administratif ; assignable à un rôle D-017).
    MessagerieFilsReserves,
    /// Remettre immédiatement un message pendant la plage calme, pour une urgence, avec motif
    /// tracé (M0b, D24) : la direction.
    MessagerieUrgenceHorsPlage,
    /// Socle 42, lot V0a : liste minimale des sorties (titre, dates, destination), tout le personnel.
    VoyagesAnnuaire,
    /// Détail de toute sortie de l'établissement (direction, administratif, comptabilité). Le porteur et les encadrants lisent la leur sans ce droit.
    VoyagesLire,
    /// Créer une sortie : professeurs, CPE, administratif, direction. Le créateur en est le porteur.
    VoyagesCreer,
    /// Modifier toute sortie (secrétariat, direction). Le porteur modifie la sienne sans ce droit.
    VoyagesModifier,
    /// Choisir et retirer les participants de toute sortie (secrétariat, direction).
    VoyagesParticipants,
    /// Choisir les encadrants de toute sortie (direction).
    VoyagesEncadrement,
    /// Valider la pédagogie, refuser, annuler, rouvrir, requalifier (direction).
    VoyagesValider,
    /// Validation finale d'une sortie (chef d'établissement et ses délégués).
    VoyagesAutoriser,
    /// Valider le budget d'un voyage (comptabilité).
    VoyagesBudgetValider,
    /// Attribuer ou transférer le porteur (secrétariat de direction).
    VoyagesPorteur,
    /// Demandes de devis, avenants et courriers aux prestataires (secrétariat, comptabilité, direction) : corrige le défaut de Next.js (VD-14).
    VoyagesPrestataires,
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
    // ── Socle 2 (l'élève), lot E1 : droits d'identité et groupes de champs ──
    /// Trouver un élève et voir sa désignation (G0).
    ElevesAnnuaire,
    /// Voir la scolarité de l'élève (G1).
    ElevesLire,
    /// Voir l'état civil de l'élève (G2).
    ElevesEtatCivilLire,
    /// Voir l'INE et les identifiants de l'élève (G3).
    ElevesIdentifiantsLire,
    /// Voir l'historique d'identité de l'élève (G5).
    ElevesHistoriqueLire,
    /// Créer un élève, annuler une création erronée.
    ElevesCreer,
    /// Corriger ou changer l'identité de l'élève.
    ElevesIdentiteModifier,
    /// Gérer l'INE et les identifiants.
    ElevesIdentifiantsGerer,
    /// Gérer scolarisations, inscriptions annuelles, régime et site.
    ElevesScolariteGerer,
    /// Clôturer une scolarisation.
    ElevesSortie,
    /// Gérer les photos des élèves.
    ElevesPhotoGerer,
    /// Gérer le badge de l'élève : déclarer perdu, attribuer un nouveau badge (ELV-41).
    ElevesBadgeGerer,
    /// Décider les propositions.
    ElevesPropositionsTraiter,
    /// Fusionner et défusionner des élèves.
    ElevesFusionner,
    /// Confidentialité renforcée et versions sensibles (G6).
    ElevesConfidentielGerer,
    /// Exports nominatifs d'élèves.
    ElevesExporter,
    /// Paramétrage élève de l'établissement.
    ElevesParametrer,
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
    /// Produire les exports officiels (LSL, LSU/LSUN, Notanet, Parcoursup, ONDE…) — D-022.
    ExportsOfficiels,
    /// Annuler en masse des cours (grève, fermeture, sortie de niveau), EDT-08 (S2-5).
    EdtAnnulerMasse,
    /// Accorder, révoquer et consulter les délégations de droits datées, ADM-04 (S2-5).
    DelegationsGerer,
    // ── S2-7 : évaluations, bulletins, livrets ──────────────────────────────
    /// Changer le statut (verrou) d'une période de notes, EVA-07.
    PeriodesVerrouiller,
    /// Générer et publier les bulletins, appréciation générale de direction, EVA-10.
    BulletinsPublier,
    /// Corriger une note après fermeture de la période (motif obligatoire), EVA-14.
    NotesCorriger,
    /// Exporter les livrets (LSU, LSL), EVA-12.
    LivretsExporter,
    // ── S2-8 : communication complète ───────────────────────────────────────
    /// Préparer, prévisualiser et lancer des campagnes d'information (COM-05).
    CampagnesLancer,
    /// Organiser les rencontres parents-professeurs et leurs créneaux (AGD-02).
    RencontresGerer,
    /// Créer et publier des formulaires, autorisations et consentements (FRM-01/03).
    FormulairesGerer,
    /// Consulter qui consent, pour un objet précis (FRM-04).
    ConsentementsLire,
    /// Régler les plafonds d'envoi et consulter la traçabilité des envois (PLT-05).
    NotificationsGerer,
    /// Ouvrir, réviser et clore les dispositifs d'accompagnement (PAP, PPS, PPRE, ULIS),
    /// définir les aménagements, affecter un AESH, réunir l'équipe (S2-11, ACC-01→05).
    AccompagnementGerer,
    /// Lire les aménagements pédagogiques d'un élève (sans diagnostic) : professeurs
    /// de l'élève et personnel de suivi (S2-11, ACC-02).
    AmenagementsLire,
    /// Lire les contraintes publiées sans donnée médicale (« ne pas courir »…) destinées
    /// aux enseignants (S2-11, SAN-06).
    ContraintesLire,
    /// Lire la fiche d'urgence minimale (contacts, contraintes publiées) d'un élève
    /// pour une sortie ou un séjour (S2-11, SEJ-08).
    SanteUrgenceLire,
    // ── Socle 4, lot A1 : droits du dossier administratif (§13 de 04_ADMINISTRATIF.md) ──
    /// Voir les responsables d'un élève et les moyens de les joindre (désignation, e-mails, téléphones).
    ResponsablesLire,
    /// Voir les adresses postales ordinaires des responsables, le foyer et la résidence.
    ResponsablesAdressesLire,
    /// Voir la qualité, l'autorité, les droits du lien, les décisions (sans motif).
    ResponsablesJuridiqueLire,
    /// Voir les adresses protégées et les motifs scellés de décision ou d'interdiction.
    ResponsablesProtegeLire,
    /// Protéger une adresse et gérer les mesures de confidentialité renforcée.
    ResponsablesProtegeGerer,
    /// Qualité juridique, autorité, droits du lien, désaccords.
    LiensJuridiqueGerer,
    /// Voir les personnes autorisées du jour, les consignes d'interdiction et les contacts d'urgence.
    RemiseLire,
    /// Autorisations de remise, validation des propositions.
    RemiseGerer,
    /// Poser et lever une interdiction (remise, contact, visite).
    RemiseInterdire,
    /// Identifiants nationaux ou d'organisme des personnes.
    PersonnesIdentifiantsGerer,
    /// Décider les propositions de modification, traiter les fiches de rentrée.
    DossierAdminPropositionsTraiter,
    /// Lancer, suivre et clore les campagnes de fiche de rentrée.
    FicheRentreeGerer,
    /// Exports nominatifs, publipostage, liste électorale, extrait RGPD.
    DossierAdminExporter,
    /// Paramétrage du domaine (natures de lien, durées, options).
    DossierAdminParametrer,
    /// Ouvre F2 et F3 d'un élève hors portée, sur motif, avec alerte à la direction ; jamais F4.
    ResponsablesAccesUrgence,
    /// Valider une inscription admise (INS-05 : secrétariat ou direction ; RP2-11). Placée en fin
    /// de catalogue : la matrice historique (oracle A1) reste inchangée.
    InscriptionsValider,
    // ── Socle 6 (personnels), lot P1 : agents et engagements (`06_PERSONNELS.md` §14.2) ──
    /// Annuaire interne des agents (désignation, fonction, état) — tout le personnel, sans téléphone.
    PersonnelsAnnuaire,
    /// Fiche d'organisation d'un agent : engagements, fonctions, disciplines, qualifications.
    PersonnelsLire,
    /// Coordonnées personnelles et contact d'urgence d'un agent.
    PersonnelsPersonnelLire,
    /// Identité restreinte, motifs réservés et suspension d'engagement (confidentiel).
    PersonnelsConfidentielGerer,
    /// Créer et modifier les agents, engagements, fonctions, disciplines et qualifications.
    PersonnelsGerer,
    /// Inviter, ouvrir, suspendre, fermer l'accès d'un agent ; réinitialiser ses identifiants.
    PersonnelsAccesGerer,
    /// Lot P2 : inviter un agent (lien ou QR code), relancer, annuler une invitation. Direction,
    /// administration et secrétariat (PRS-20 à 23) ; n'ouvre ni la fiche d'organisation ni la gestion.
    PersonnelsInviter,
    // ── Socle 6, lot P3 : services d'enseignement et interventions (`06_PERSONNELS.md` §14.2) ──
    /// Répartition des services : qui enseigne quoi à qui (lecture). Tout le personnel (aujourd'hui
    /// `referentiel.lire`) ; `services.gerer` l'inclut.
    PersonnelsServicesLire,
    /// Créer, modifier, clore, annuler les services ; ajouter, terminer, corriger les interventions,
    /// nommer un suppléant, remplacer un agent (aujourd'hui `referentiel.gerer`).
    ServicesGerer,
    // ── Socle 6, lot P4 : missions et périmètre (`06_PERSONNELS.md` §14.2) ──
    /// Nommer, terminer, transférer, reconduire les missions (professeur principal, référents…)
    /// (aujourd'hui `referentiel.gerer`).
    MissionsGerer,
    /// Créer les types de missions propres à l'établissement (PRS-64).
    MissionsTypesGerer,
    /// Expliquer l'accès d'un agent à un élève et simuler une attribution (PRS-101, 102). Ne se
    /// donne que par qui la détient (anti-escalade).
    AccesRevue,
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
    /// Socle 6, lot P4 : cibles portées par l'ATTRIBUTION (mission : classe du professeur principal,
    /// niveaux du CPE référent, élèves de l'AESH…), datées ; un seul modèle de rôle sert à toutes les classes.
    MonPerimetre,
    /// Socle 6, lot P4 : l'acteur lui-même (sa fiche, ses services, ses droits). Définie ; aucune action
    /// ne l'admet encore (la délégation de ses propres missions arrive au lot P6).
    Moi,
    /// Socle 6, lot P4 : élèves des SITES désignés dans le rôle (les cibles doivent être des sites).
    SitesDesignes,
}

impl Action {
    pub const TOUTES: [Action; 127] = [
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
        Action::ElevesAnnuaire,
        Action::ElevesLire,
        Action::ElevesEtatCivilLire,
        Action::ElevesIdentifiantsLire,
        Action::ElevesHistoriqueLire,
        Action::ElevesCreer,
        Action::ElevesIdentiteModifier,
        Action::ElevesIdentifiantsGerer,
        Action::ElevesScolariteGerer,
        Action::ElevesSortie,
        Action::ElevesPhotoGerer,
        Action::ElevesBadgeGerer,
        Action::ElevesPropositionsTraiter,
        Action::ElevesFusionner,
        Action::ElevesConfidentielGerer,
        Action::ElevesExporter,
        Action::ElevesParametrer,
        Action::InscriptionsGerer,
        Action::InscriptionsDecider,
        Action::ImportsAppliquer,
        Action::CahierLire,
        Action::CahierEcrire,
        Action::CahierViser,
        Action::ExportsOfficiels,
        Action::EdtAnnulerMasse,
        Action::DelegationsGerer,
        // S2-7
        Action::PeriodesVerrouiller,
        Action::BulletinsPublier,
        Action::NotesCorriger,
        Action::LivretsExporter,
        Action::CampagnesLancer,
        Action::RencontresGerer,
        Action::FormulairesGerer,
        Action::ConsentementsLire,
        Action::NotificationsGerer,
        Action::AccompagnementGerer,
        Action::AmenagementsLire,
        Action::ContraintesLire,
        Action::SanteUrgenceLire,
        // A1
        Action::ResponsablesLire,
        Action::ResponsablesAdressesLire,
        Action::ResponsablesJuridiqueLire,
        Action::ResponsablesProtegeLire,
        Action::ResponsablesProtegeGerer,
        Action::LiensJuridiqueGerer,
        Action::RemiseLire,
        Action::RemiseGerer,
        Action::RemiseInterdire,
        Action::PersonnesIdentifiantsGerer,
        Action::DossierAdminPropositionsTraiter,
        Action::FicheRentreeGerer,
        Action::DossierAdminExporter,
        Action::DossierAdminParametrer,
        Action::ResponsablesAccesUrgence,
        Action::InscriptionsValider,
        // Socle 6, lot P1
        Action::PersonnelsAnnuaire,
        Action::PersonnelsLire,
        Action::PersonnelsPersonnelLire,
        Action::PersonnelsConfidentielGerer,
        Action::PersonnelsGerer,
        Action::PersonnelsAccesGerer,
        Action::PersonnelsInviter,
        // Socle 6, lot P3
        Action::PersonnelsServicesLire,
        Action::ServicesGerer,
        // M0b : ajoutées en fin de catalogue (le catalogue historique est repéré par position).
        Action::MessagerieFilsReserves,
        Action::MessagerieUrgenceHorsPlage,
        // Socle 42, lot V0a.
        Action::VoyagesAnnuaire,
        Action::VoyagesLire,
        Action::VoyagesCreer,
        Action::VoyagesModifier,
        Action::VoyagesParticipants,
        Action::VoyagesEncadrement,
        Action::VoyagesValider,
        Action::VoyagesAutoriser,
        Action::VoyagesBudgetValider,
        Action::VoyagesPorteur,
        Action::VoyagesPrestataires,
        // Socle 6, lot P4 (en fin de catalogue : le catalogue historique est repéré par position).
        Action::MissionsGerer,
        Action::MissionsTypesGerer,
        Action::AccesRevue,
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
            Self::MessagerieFilsReserves => "messagerie.fils_reserves.lire",
            Self::MessagerieUrgenceHorsPlage => "messagerie.urgence_hors_plage",
            Self::VoyagesAnnuaire => "voyages.annuaire",
            Self::VoyagesLire => "voyages.lire",
            Self::VoyagesCreer => "voyages.creer",
            Self::VoyagesModifier => "voyages.modifier",
            Self::VoyagesParticipants => "voyages.participants",
            Self::VoyagesEncadrement => "voyages.encadrement",
            Self::VoyagesValider => "voyages.valider",
            Self::VoyagesAutoriser => "voyages.autoriser",
            Self::VoyagesBudgetValider => "voyages.budget_valider",
            Self::VoyagesPorteur => "voyages.porteur",
            Self::VoyagesPrestataires => "voyages.prestataires",
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
            Self::ElevesAnnuaire => "eleves.annuaire",
            Self::ElevesLire => "eleves.lire",
            Self::ElevesEtatCivilLire => "eleves.etat_civil.lire",
            Self::ElevesIdentifiantsLire => "eleves.identifiants.lire",
            Self::ElevesHistoriqueLire => "eleves.historique.lire",
            Self::ElevesCreer => "eleves.creer",
            Self::ElevesIdentiteModifier => "eleves.identite.modifier",
            Self::ElevesIdentifiantsGerer => "eleves.identifiants.gerer",
            Self::ElevesScolariteGerer => "eleves.scolarite.gerer",
            Self::ElevesSortie => "eleves.sortie",
            Self::ElevesPhotoGerer => "eleves.photo.gerer",
            Self::ElevesBadgeGerer => "eleves.badge.gerer",
            Self::ElevesPropositionsTraiter => "eleves.propositions.traiter",
            Self::ElevesFusionner => "eleves.fusionner",
            Self::ElevesConfidentielGerer => "eleves.confidentiel.gerer",
            Self::ElevesExporter => "eleves.exporter",
            Self::ElevesParametrer => "eleves.parametrer",
            Self::InscriptionsGerer => "inscriptions.gerer",
            Self::InscriptionsDecider => "inscriptions.decider",
            Self::ImportsAppliquer => "imports.appliquer",
            Self::CahierLire => "cahier.lire",
            Self::CahierEcrire => "cahier.ecrire",
            Self::CahierViser => "cahier.viser",
            Self::ExportsOfficiels => "exports.officiels",
            Self::EdtAnnulerMasse => "edt.annuler_masse",
            Self::DelegationsGerer => "delegations.gerer",
            Self::PeriodesVerrouiller => "periodes.verrouiller",
            Self::BulletinsPublier => "bulletins.publier",
            Self::NotesCorriger => "notes.corriger",
            Self::LivretsExporter => "livrets.exporter",
            Self::CampagnesLancer => "campagnes.lancer",
            Self::RencontresGerer => "rencontres.gerer",
            Self::FormulairesGerer => "formulaires.gerer",
            Self::ConsentementsLire => "consentements.lire",
            Self::NotificationsGerer => "notifications.gerer",
            Self::AccompagnementGerer => "accompagnement.gerer",
            Self::AmenagementsLire => "amenagements.lire",
            Self::ContraintesLire => "contraintes.lire",
            Self::SanteUrgenceLire => "sante.urgence_lire",
            Self::ResponsablesLire => "responsables.lire",
            Self::ResponsablesAdressesLire => "responsables.adresses.lire",
            Self::ResponsablesJuridiqueLire => "responsables.juridique.lire",
            Self::ResponsablesProtegeLire => "responsables.protege.lire",
            Self::ResponsablesProtegeGerer => "responsables.protege.gerer",
            Self::LiensJuridiqueGerer => "liens.juridique.gerer",
            Self::RemiseLire => "remise.lire",
            Self::RemiseGerer => "remise.gerer",
            Self::RemiseInterdire => "remise.interdire",
            Self::PersonnesIdentifiantsGerer => "personnes.identifiants.gerer",
            Self::DossierAdminPropositionsTraiter => "dossier_admin.propositions.traiter",
            Self::FicheRentreeGerer => "fiche_rentree.gerer",
            Self::DossierAdminExporter => "dossier_admin.exporter",
            Self::DossierAdminParametrer => "dossier_admin.parametrer",
            Self::ResponsablesAccesUrgence => "responsables.acces_urgence",
            Self::InscriptionsValider => "inscriptions.valider",
            Self::PersonnelsAnnuaire => "personnels.annuaire",
            Self::PersonnelsLire => "personnels.lire",
            Self::PersonnelsPersonnelLire => "personnels.personnel.lire",
            Self::PersonnelsConfidentielGerer => "personnels.confidentiel.gerer",
            Self::PersonnelsGerer => "personnels.gerer",
            Self::PersonnelsAccesGerer => "personnels.acces.gerer",
            Self::PersonnelsInviter => "personnels.inviter",
            Self::PersonnelsServicesLire => "personnels.services.lire",
            Self::ServicesGerer => "services.gerer",
            Self::MissionsGerer => "missions.gerer",
            Self::MissionsTypesGerer => "missions.types.gerer",
            Self::AccesRevue => "acces.revue",
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
            Self::MessagerieFilsReserves => "Lire les fils privés des familles et des élèves",
            Self::MessagerieUrgenceHorsPlage => "Remettre un message en urgence pendant la plage calme",
            Self::VoyagesAnnuaire => "Liste minimale des sorties et voyages de l'établissement (titre, dates, destination)",
            Self::VoyagesLire => "Consulter le détail de toutes les sorties et voyages",
            Self::VoyagesCreer => "Créer une sortie ou un voyage (on en devient le porteur)",
            Self::VoyagesModifier => "Modifier une sortie ou un voyage dont on n'est pas le porteur",
            Self::VoyagesParticipants => "Gérer les participants d'une sortie dont on n'est pas le porteur",
            Self::VoyagesEncadrement => "Gérer l'encadrement d'une sortie dont on n'est pas le porteur",
            Self::VoyagesValider => "Valider la pédagogie, refuser, annuler, rouvrir, requalifier une sortie",
            Self::VoyagesAutoriser => "Autoriser une sortie (validation finale du chef d'établissement)",
            Self::VoyagesBudgetValider => "Valider le budget d'un voyage (comptabilité)",
            Self::VoyagesPorteur => "Attribuer ou transférer le porteur d'une sortie",
            Self::VoyagesPrestataires => "Demander des devis et écrire aux prestataires d'une sortie dont on n'est pas le porteur",
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
            Self::ElevesAnnuaire => "Trouver un élève et voir sa désignation (nom, classe, photo)",
            Self::ElevesLire => "Voir la scolarité détaillée de l'élève (régime, MEF, dates)",
            Self::ElevesEtatCivilLire => "Voir l'état civil de l'élève (sexe, date de naissance)",
            Self::ElevesIdentifiantsLire => "Voir l'INE et les identifiants de l'élève (badge…)",
            Self::ElevesHistoriqueLire => "Voir l'historique d'identité de l'élève",
            Self::ElevesCreer => "Créer un élève ou annuler une création erronée",
            Self::ElevesIdentiteModifier => "Corriger l'identité, enregistrer un changement d'état civil",
            Self::ElevesIdentifiantsGerer => "Gérer l'INE et les identifiants des élèves",
            Self::ElevesScolariteGerer => "Gérer la scolarité (entrées, inscriptions, régime, site)",
            Self::ElevesSortie => "Clôturer la scolarité d'un élève (radiation)",
            Self::ElevesPhotoGerer => "Gérer les photos des élèves",
            Self::ElevesBadgeGerer => "Gérer le badge des élèves (perte, remplacement)",
            Self::ElevesPropositionsTraiter => "Décider les propositions de modification d'identité",
            Self::ElevesFusionner => "Fusionner ou défusionner des élèves en doublon",
            Self::ElevesConfidentielGerer => "Confidentialité renforcée et données d'identité sensibles",
            Self::ElevesExporter => "Exporter des listes nominatives d'élèves",
            Self::ElevesParametrer => "Paramétrer les champs et règles de l'élève",
            Self::InscriptionsGerer => "Gérer les campagnes et instruire les dossiers d'inscription",
            Self::InscriptionsDecider => "Décider des admissions",
            Self::ImportsAppliquer => "Importer des élèves et traiter les rejets d'import",
            Self::CahierLire => "Lire le cahier de textes, le travail à faire et la charge de travail",
            Self::CahierEcrire => "Renseigner le cahier de textes, donner du travail, corriger les copies",
            Self::CahierViser => "Viser le cahier de textes (inspection, direction)",
            Self::ExportsOfficiels => "Produire les exports officiels (LSL, LSU, Notanet, Parcoursup, ONDE)",
            Self::EdtAnnulerMasse => "Annuler en masse des cours (grève, fermeture, sortie de niveau)",
            Self::DelegationsGerer => "Déléguer des droits pour une durée limitée",
            Self::PeriodesVerrouiller => "Ouvrir, verrouiller ou fermer une période de notes",
            Self::BulletinsPublier => "Générer et publier les bulletins",
            Self::NotesCorriger => "Corriger une note après fermeture de la période",
            Self::LivretsExporter => "Exporter les livrets (LSU, LSL)",
            Self::CampagnesLancer => "Lancer des campagnes d'information (e-mail, SMS, push)",
            Self::RencontresGerer => "Organiser les rencontres parents-professeurs",
            Self::FormulairesGerer => "Créer et publier formulaires, autorisations et consentements",
            Self::ConsentementsLire => "Consulter les consentements (objet par objet)",
            Self::NotificationsGerer => "Régler les plafonds d'envoi et suivre les envois",
            Self::AccompagnementGerer => {
                "Gérer l'accompagnement : PAP, PPS, aménagements, AESH, équipe éducative"
            }
            Self::AmenagementsLire => "Lire les aménagements pédagogiques d'un élève (sans diagnostic)",
            Self::ContraintesLire => "Lire les contraintes publiées pour les enseignants (sans donnée médicale)",
            Self::SanteUrgenceLire => "Lire la fiche d'urgence minimale d'un élève (sortie, séjour)",
            Self::ResponsablesLire => "Voir les responsables et les joindre (F0, F1)",
            Self::ResponsablesAdressesLire => "Voir les adresses des responsables (F2)",
            Self::ResponsablesJuridiqueLire => "Voir la situation juridique (F3)",
            Self::ResponsablesProtegeLire => "Voir les données protégées (F4)",
            Self::ResponsablesProtegeGerer => "Protéger une adresse",
            Self::LiensJuridiqueGerer => "Gérer la situation juridique des liens",
            Self::RemiseLire => "Voir qui peut récupérer l'élève et les interdictions (R)",
            Self::RemiseGerer => "Gérer les autorisations de remise",
            Self::RemiseInterdire => "Poser et lever une interdiction",
            Self::PersonnesIdentifiantsGerer => "Gérer les identifiants des personnes",
            Self::DossierAdminPropositionsTraiter => "Décider les propositions de la famille",
            Self::FicheRentreeGerer => "Gérer les campagnes de fiche de rentrée",
            Self::DossierAdminExporter => "Exports nominatifs du dossier administratif",
            Self::DossierAdminParametrer => "Paramétrer le dossier administratif",
            Self::ResponsablesAccesUrgence => "Bris de glace sur le dossier administratif",
            Self::InscriptionsValider => "Valider les inscriptions admises par la direction",
            Self::PersonnelsAnnuaire => "Annuaire interne des agents",
            Self::PersonnelsLire => "Fiche d'organisation des agents (engagements, fonctions)",
            Self::PersonnelsPersonnelLire => "Coordonnées personnelles et contact d'urgence des agents",
            Self::PersonnelsConfidentielGerer => "Identité restreinte et suspension d'engagement des agents",
            Self::PersonnelsGerer => "Créer et modifier les agents et leurs engagements",
            Self::PersonnelsAccesGerer => "Inviter, ouvrir, suspendre, fermer l'accès d'un agent",
            Self::PersonnelsInviter => "Inviter un agent (lien, QR code), relancer, annuler une invitation",
            Self::PersonnelsServicesLire => "Répartition des services d'enseignement et de leurs intervenants",
            Self::ServicesGerer => "Créer et modifier les services d'enseignement et leurs interventions",
            Self::MissionsGerer => "Nommer, terminer et transférer les missions (professeur principal, référents…)",
            Self::MissionsTypesGerer => "Créer les types de missions propres à l'établissement",
            Self::AccesRevue => "Expliquer l'accès d'un agent à un élève et simuler une attribution",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|a| a.code() == s.trim())
    }

    /// Actions de SAISIE de l'enseignant (lot P4) : avec la portée « mes élèves par l'enseignement »,
    /// seule une intervention qui saisit EFFECTIVEMENT (titulaire hors suppléance, co-enseignant,
    /// suppléant) ouvre le droit ; un assistant ou un intervenant lit sans saisir.
    pub fn est_saisie_enseignement(self) -> bool {
        matches!(
            self,
            Self::NotesSaisir | Self::CompetencesEvaluer | Self::CahierEcrire | Self::AbsencesDeclarer
        )
    }

    /// Actions qui accordent aussi celle-ci : `facturation.gerer` permet de lire.
    pub fn accordee_par(self) -> &'static [Action] {
        match self {
            Self::FacturationLire => &[Self::FacturationGerer],
            // E1 (§12.2) : seules inclusions explicites des droits « élève ».
            Self::ElevesEtatCivilLire => &[Self::ElevesIdentiteModifier],
            Self::ElevesIdentifiantsLire => &[Self::ElevesIdentifiantsGerer],
            // §13.2 : inclusions explicites, aucune autre.
            Self::ResponsablesLire | Self::ResponsablesAdressesLire => &[Self::PersonnesGerer],
            Self::ResponsablesJuridiqueLire => &[Self::LiensJuridiqueGerer],
            Self::RemiseLire => &[Self::RemiseGerer],
            Self::ResponsablesProtegeLire => &[Self::ResponsablesProtegeGerer],
            // Socle 6 (§14.2) : inclusions explicites, aucune autre.
            Self::PersonnelsLire => &[Self::PersonnelsGerer],
            Self::PersonnelsServicesLire => &[Self::ServicesGerer],
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
            | Self::CahierViser
            | Self::RencontresGerer
            | Self::FormulairesGerer
            | Self::ConsentementsLire
            | Self::NotificationsGerer
            | Self::MessagerieFilsReserves
            | Self::MessagerieUrgenceHorsPlage
            | Self::VoyagesAnnuaire
            | Self::VoyagesLire
            | Self::VoyagesCreer
            | Self::VoyagesModifier
            | Self::VoyagesParticipants
            | Self::VoyagesEncadrement
            | Self::VoyagesValider
            | Self::VoyagesAutoriser
            | Self::VoyagesBudgetValider
            | Self::VoyagesPorteur
            | Self::VoyagesPrestataires
            | Self::ExportsOfficiels => &[Portee::Etablissement],
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
            // E1 : lectures d'identité par groupe, 4 portées (comme `_` ci-dessous) ;
            // les écritures et la gouvernance sont d'établissement entier.
            Self::ElevesAnnuaire
            | Self::ElevesLire
            | Self::ElevesEtatCivilLire
            | Self::ElevesIdentifiantsLire => &[
                Portee::Etablissement,
                Portee::MesClasses,
                Portee::MesGroupes,
                Portee::Designes,
            ],
            Self::ElevesHistoriqueLire
            | Self::ElevesCreer
            | Self::ElevesIdentiteModifier
            | Self::ElevesIdentifiantsGerer
            | Self::ElevesScolariteGerer
            | Self::ElevesSortie
            | Self::ElevesPhotoGerer
            | Self::ElevesBadgeGerer
            | Self::ElevesPropositionsTraiter
            | Self::ElevesFusionner
            | Self::ElevesConfidentielGerer
            | Self::ElevesExporter
            | Self::ElevesParametrer => &[Portee::Etablissement],
            // S2-1 : actions sur les personnes (non rattachées à une classe) :
            // établissement entier seulement.
            Self::PersonnesGerer
            | Self::PayeursDefinir
            | Self::LiensSuspendre
            | Self::PersonnesFusionner
            | Self::EdtAnnulerMasse
            | Self::DelegationsGerer => &[Portee::Etablissement],
            // A1 (§13.2) : lectures par groupe à quatre portées (branche par défaut) ; le reste
            // n'admet que l'établissement (les « sites désignés » viendront avec le référentiel de sites).
            Self::ResponsablesProtegeLire
            | Self::ResponsablesProtegeGerer
            | Self::LiensJuridiqueGerer
            | Self::RemiseGerer
            | Self::RemiseInterdire
            | Self::PersonnesIdentifiantsGerer
            | Self::DossierAdminPropositionsTraiter
            | Self::FicheRentreeGerer
            | Self::DossierAdminExporter
            | Self::DossierAdminParametrer
            | Self::ResponsablesAccesUrgence
            // Socle 6, lot P1 : les « sites désignés » viendront avec le référentiel de sites ;
            // `personnels.acces.gerer` et `personnels.confidentiel.gerer` ne se donnent que par
            // qui les détient (anti-escalade, §14.2).
            | Self::PersonnelsAnnuaire
            | Self::PersonnelsLire
            | Self::PersonnelsPersonnelLire
            | Self::PersonnelsConfidentielGerer
            | Self::PersonnelsGerer
            | Self::PersonnelsAccesGerer
            | Self::PersonnelsInviter
            | Self::PersonnelsServicesLire
            | Self::ServicesGerer
            | Self::MissionsGerer
            | Self::MissionsTypesGerer
            | Self::AccesRevue => &[Portee::Etablissement],
            // S2-3 : campagnes, décisions et imports : établissement entier seulement.
            Self::InscriptionsGerer
            | Self::InscriptionsDecider
            | Self::InscriptionsValider
            | Self::ImportsAppliquer => {
                &[Portee::Etablissement]
            }
            // S2-7 : actions de direction sur toute une période ou toute une classe.
            Self::PeriodesVerrouiller
            | Self::BulletinsPublier
            | Self::NotesCorriger
            | Self::LivretsExporter => &[Portee::Etablissement],
            _ => &[
                Portee::Etablissement,
                Portee::MesClasses,
                Portee::MesGroupes,
                Portee::Designes,
                Portee::MonPerimetre,
                Portee::SitesDesignes,
            ],
        }
    }
}

impl Portee {
    pub const TOUTES: [Portee; 7] = [
        Portee::Etablissement,
        Portee::MesClasses,
        Portee::MesGroupes,
        Portee::Designes,
        Portee::MonPerimetre,
        Portee::Moi,
        Portee::SitesDesignes,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Etablissement => "etablissement",
            Self::MesClasses => "mes_classes",
            Self::MesGroupes => "mes_groupes",
            Self::Designes => "designes",
            Self::MonPerimetre => "mon_perimetre",
            Self::Moi => "moi",
            Self::SitesDesignes => "sites_designes",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Etablissement => "Tout l'établissement",
            Self::MesClasses => "Mes classes (enseignement)",
            Self::MesGroupes => "Mes groupes et ateliers (responsable)",
            Self::Designes => "Classes, groupes, sites ou niveaux désignés",
            Self::MonPerimetre => "Mon périmètre (porté par ma mission ou mon attribution)",
            Self::Moi => "Moi-même",
            Self::SitesDesignes => "Sites désignés",
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
    // Socle 6 (§14.2) : annuaire interne = comportement actuel de `/personnel/search` (tout le
    // personnel, sans le téléphone) ; fiche d'organisation, gestion et accès = l'ancien
    // `provisionner_personnel` (Admin, Direction). Le secrétariat n'obtient pas la gestion des
    // agents par défaut (Q-P23 : comportement actuel conservé, un rôle paramétré peut l'ouvrir).
    d.push(etab(PersonnelsAnnuaire));
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif) {
        d.push(etab(PersonnelsLire));
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif) {
        d.push(etab(PersonnelsInviter));
    }
    // Lot P3 : mêmes profils que `referentiel.lire` / `referentiel.gerer` (comportement actuel).
    d.push(etab(PersonnelsServicesLire));
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif) {
        d.push(etab(ServicesGerer));
        // Lot P4 : `missions.gerer` = `referentiel.gerer` (comportement actuel du professeur principal).
        d.push(etab(MissionsGerer));
    }
    if role == Role::Admin {
        d.push(etab(MissionsTypesGerer));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(AccesRevue));
    }
    // Socle 42, lot V0a (`42_SORTIES_VOYAGES.md` §21.1) : sorties et voyages collectifs. La liste
    // minimale est ouverte à tout le personnel ; le détail, la gestion et les validations sont
    // réservés. Le porteur et les encadrants d'une sortie agissent sur la leur sans droit d'établissement.
    d.push(etab(VoyagesAnnuaire));
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif | Role::Compta) {
        d.push(etab(VoyagesLire));
    }
    if matches!(
        role,
        Role::Admin | Role::Direction | Role::Administratif | Role::Prof | Role::Cpe
    ) {
        d.push(etab(VoyagesCreer));
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif) {
        for a in [VoyagesModifier, VoyagesParticipants, VoyagesPorteur] {
            d.push(etab(a));
        }
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif | Role::Compta) {
        d.push(etab(VoyagesPrestataires));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        for a in [VoyagesEncadrement, VoyagesValider, VoyagesAutoriser] {
            d.push(etab(a));
        }
    }
    if matches!(role, Role::Admin | Role::Compta) {
        d.push(etab(VoyagesBudgetValider));
    }
    if role.can_gerer_personnels() {
        for a in [
            PersonnelsPersonnelLire,
            PersonnelsConfidentielGerer,
            PersonnelsGerer,
            PersonnelsAccesGerer,
        ] {
            d.push(etab(a));
        }
    }
    // E1 : l'annuaire (désignation) est ouvert à tout le personnel, comme l'ancien
    // `can_read_all_eleves` (Q-E3). Le détail de scolarité (G1, régime, MEF, dates),
    // jusqu'ici réservé au niveau « complet », reste à ces profils : un établissement
    // l'ouvre à d'autres rôles en cochant `eleves.lire` (aucun élargissement implicite).
    // D-022 : le professeur ne trouve et ne voit que les élèves de ses classes et groupes
    // (jamais l'établissement entier) ; la portée « mes classes » suit la règle du groupe.
    if role == Role::Prof {
        d.push((ElevesAnnuaire, Portee::MesClasses));
    } else {
        d.push(etab(ElevesAnnuaire));
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif) {
        d.push(etab(ElevesLire));
        d.push(etab(ElevesIdentifiantsLire));
    }
    // État civil : niveau « complet » et niveau « santé » (infirmerie) historiques.
    if matches!(
        role,
        Role::Admin | Role::Direction | Role::Administratif | Role::Infirmerie
    ) {
        d.push(etab(ElevesEtatCivilLire));
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
            // S2-11 : aménagements et contraintes publiées des élèves qu'il enseigne ;
            // fiche d'urgence minimale pour une sortie de sa classe.
            AmenagementsLire,
            ContraintesLire,
            SanteUrgenceLire,
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
        // A1 : désignation, moyens de joindre et remise des élèves qu'il suit (sorties, voyages).
        d.push((ResponsablesLire, Portee::MesClasses));
        d.push((RemiseLire, Portee::MesClasses));
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
        // RP1-07 : la comptabilité n'a aucun besoin de la vie scolaire ni du suivi
        // (journal, alertes CPE, dispenses, conseils, sorties, internat, chronologie).
        if role != Role::Compta {
            d.push(etab(SuiviEleve));
        }
    }
    if (role.can_read_all_eleves() || role.can_manage_eleves()) && role != Role::Compta {
        d.push(etab(JournalLire));
    }
    if matches!(
        role,
        Role::Admin | Role::Direction | Role::Administratif | Role::Cpe
    ) {
        d.push(etab(MessagerieEleve));
    }
    // M0b (D26) : le droit de lire les fils privés remplace la liste de rôles codée en dur
    // (M0a, D2) ; mêmes titulaires par défaut, assignable à un rôle paramétré (un AED).
    if matches!(
        role,
        Role::Admin | Role::Direction | Role::Administratif | Role::Cpe
    ) {
        d.push(etab(MessagerieFilsReserves));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(MessagerieUrgenceHorsPlage));
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
    // S2-11 : accompagnement (direction), aménagements et contraintes publiées
    // (personnel de suivi), fiche d'urgence minimale (encadrement des sorties).
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(AccompagnementGerer));
    }
    if matches!(
        role,
        Role::Admin
            | Role::Direction
            | Role::Administratif
            | Role::Cpe
            | Role::Infirmerie
            | Role::Psy
    ) {
        d.push(etab(AmenagementsLire));
    }
    if matches!(
        role,
        Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Infirmerie
    ) {
        d.push(etab(ContraintesLire));
    }
    if matches!(
        role,
        Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Infirmerie
    ) {
        d.push(etab(SanteUrgenceLire));
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
        // F0 (FD-27) : facturation.gerer et bourses.gerer ne vont plus à l'Administratif ;
        // facturation.lire lui reste (consultation du compte du payeur présent).
        if role.can_manage_facturation() {
            d.push(etab(FacturationGerer));
        }
        if role.can_lire_facturation() {
            d.push(etab(FacturationLire));
        }
        if role.can_manage_bourses() {
            d.push(etab(BoursesGerer));
        }
        d.push(etab(RepasGerer));
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
    // E1 : écritures et gouvernance de l'élève. Les écritures reprennent exactement
    // `eleves.gerer` (Admin, Direction, Administratif) ; elles accompagnent cette
    // action, si bien qu'aucun profil existant ne change.
    if role.can_manage_eleves() {
        for a in [
            ElevesHistoriqueLire,
            ElevesCreer,
            ElevesIdentiteModifier,
            ElevesIdentifiantsGerer,
            ElevesScolariteGerer,
            ElevesSortie,
            ElevesPropositionsTraiter,
            ElevesExporter,
        ] {
            d.push(etab(a));
        }
    }
    if direction_admin || role == Role::Cpe {
        d.push(etab(ElevesPhotoGerer));
        d.push(etab(ElevesBadgeGerer));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(ElevesFusionner));
        d.push(etab(ElevesConfidentielGerer));
    }
    if role == Role::Admin {
        d.push(etab(ElevesParametrer));
    }
    // S2-3 : inscriptions (gestion des élèves) ; décision et validation ★ :
    // direction (un rôle paramétré peut l'accorder au secrétariat, D-017).
    // D-022 : les exports officiels sont faits par les administratifs (direction,
    // secrétariat) ; jamais par le modèle « professeur » (retourné plus haut).
    if role.can_manage_eleves() {
        d.push(etab(ExportsOfficiels));
    }
    if role.can_manage_eleves() {
        d.push(etab(InscriptionsGerer));
        d.push(etab(InscriptionsValider));
        d.push(etab(ImportsAppliquer));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(InscriptionsDecider));
    }
    // S2-7 : verrou des périodes, bulletins et corrections après fermeture à la
    // direction ; exports de livrets aussi au secrétariat (administratif).
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(PeriodesVerrouiller));
        d.push(etab(BulletinsPublier));
        d.push(etab(NotesCorriger));
    }
    if direction_admin {
        d.push(etab(LivretsExporter));
    }
    // S2-8 : communication. Campagnes, rencontres, plafonds : direction (COM-05 DIR★,
    // AGD-02 DIR). Formulaires et consentements : direction, administratif, CPE.
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(CampagnesLancer));
        d.push(etab(RencontresGerer));
        d.push(etab(NotificationsGerer));
    }
    if direction_admin || role == Role::Cpe {
        d.push(etab(FormulairesGerer));
        d.push(etab(ConsentementsLire));
    }
    // ── A1 (§13.2) : groupes de lecture du dossier administratif et droits associés. ──
    // Le gestionnaire (`personnes.gerer`) détient explicitement les lectures F0–F2 ; la
    // situation juridique suit `liens.juridique.gerer` (aujourd'hui incluse dans
    // `personnes.gerer` : mêmes profils, bascule à l'identique).
    if role.can_manage_eleves() {
        for a in [
            ResponsablesLire,
            ResponsablesAdressesLire,
            ResponsablesJuridiqueLire,
            LiensJuridiqueGerer,
            RemiseLire,
            RemiseGerer,
            PersonnesIdentifiantsGerer,
            DossierAdminPropositionsTraiter,
            FicheRentreeGerer,
            DossierAdminExporter,
        ] {
            d.push(etab(a));
        }
    }
    if matches!(role, Role::Admin | Role::Direction) {
        for a in [ResponsablesProtegeLire, ResponsablesProtegeGerer, RemiseInterdire] {
            d.push(etab(a));
        }
    }
    if role == Role::Admin {
        d.push(etab(DossierAdminParametrer));
    }
    // Bris de glace : modèle Direction seulement ; les autres rôles l'obtiennent par paramétrage.
    if role == Role::Direction {
        d.push(etab(ResponsablesAccesUrgence));
    }
    // Vie scolaire : désigne, joint, voit la situation juridique (sans les payeurs : ils
    // suivent `payeurs.definir`) et la remise.
    if role == Role::Cpe {
        for a in [ResponsablesLire, ResponsablesJuridiqueLire, RemiseLire] {
            d.push(etab(a));
        }
    }
    // Comptabilité : désignation, moyens de joindre et adresses de facturation.
    if role == Role::Compta {
        d.push(etab(ResponsablesLire));
        d.push(etab(ResponsablesAdressesLire));
    }
    // Infirmerie et accueil : qui appeler, qui peut récupérer l'élève.
    if matches!(role, Role::Infirmerie | Role::Accueil) {
        d.push(etab(ResponsablesLire));
        d.push(etab(RemiseLire));
    }
    // Accueil (profil nouveau, aucune matrice historique) : déclarer les absences reçues.
    if role == Role::Accueil {
        d.push(etab(AbsencesDeclarer));
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
        // D-022 : exports officiels réservés aux administratifs.
        assert_eq!(a(Role::Prof, ExportsOfficiels), None);
        assert_eq!(a(Role::Cpe, ExportsOfficiels), None);
        assert_eq!(a(Role::Direction, ExportsOfficiels), Some(Portee::Etablissement));
        assert_eq!(a(Role::Administratif, ExportsOfficiels), Some(Portee::Etablissement));
        assert_eq!(a(Role::Administratif, InscriptionsGerer), Some(Portee::Etablissement));
        assert_eq!(a(Role::Administratif, InscriptionsDecider), None);
        assert_eq!(a(Role::Direction, InscriptionsDecider), Some(Portee::Etablissement));
        assert_eq!(a(Role::Administratif, InscriptionsValider), Some(Portee::Etablissement));
        assert_eq!(a(Role::Direction, InscriptionsValider), Some(Portee::Etablissement));
        assert_eq!(a(Role::Cpe, InscriptionsValider), None);
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
        let oracle: [(Action, fn(Role) -> bool); 15] = [
            // D-022 (écart voulu avec l'historique : le professeur n'exporte plus).
            (ExportsOfficiels, Role::can_manage_eleves),
            (FacturationGerer, Role::can_manage_facturation),
            (FacturationLire, Role::can_lire_facturation),
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

    /// Socle 6, lot P1 : les actions `personnels.*` reproduisent exactement les contrôles de profil
    /// d'avant (`provisionner_personnel`, `cible_compte_autorisee`, `/personnel/search`) : aucun
    /// profil ne gagne ni ne perd de droit ; les portées sont d'établissement seulement.
    #[test]
    fn p1_modele_identique_aux_capacites_historiques() {
        use Action::*;
        let tous = [
            Role::Admin,
            Role::Direction,
            Role::Administratif,
            Role::Cpe,
            Role::Compta,
            Role::Prof,
            Role::Infirmerie,
            Role::Psy,
            Role::Accueil,
            Role::Parent,
            Role::Eleve,
        ];
        for r in tous {
            let m = modele_par_defaut(r);
            let a = |x: Action| m.iter().filter(|(y, _)| *y == x).count();
            // Annuaire : tout le personnel (comme `Role::is_staff` pour `/personnel/search`).
            assert_eq!(a(PersonnelsAnnuaire) == 1, r.is_staff(), "{r:?} annuaire");
            // Gestion et accès : l'ancien contrôle codé en dur (Admin, Direction).
            for x in [PersonnelsGerer, PersonnelsAccesGerer, PersonnelsConfidentielGerer, PersonnelsPersonnelLire] {
                assert_eq!(a(x) == 1, r.can_gerer_personnels(), "{r:?} {}", x.code());
            }
            // Fiche d'organisation : Admin, Direction, Administratif ; le secrétariat ne crée pas
            // d'agent par défaut (Q-P23).
            assert_eq!(
                a(PersonnelsLire) == 1,
                matches!(r, Role::Admin | Role::Direction | Role::Administratif),
                "{r:?} lire"
            );
            assert_eq!(a(PersonnelsGerer), usize::from(r == Role::Admin || r == Role::Direction));
            // Lot P2 : inviter = direction, administration, secrétariat (PRS-20 à 23).
            assert_eq!(
                a(PersonnelsInviter),
                usize::from(matches!(r, Role::Admin | Role::Direction | Role::Administratif)),
                "{r:?} inviter"
            );
            // Lot P3 : lecture = `referentiel.lire` (tout le personnel), gestion = `referentiel.gerer`.
            assert_eq!(a(PersonnelsServicesLire) == 1, r.is_staff(), "{r:?} services.lire");
            assert_eq!(
                a(ServicesGerer),
                usize::from(matches!(r, Role::Admin | Role::Direction | Role::Administratif)),
                "{r:?} services.gerer"
            );
            for (x, p) in &m {
                if x.code().starts_with("personnels.") || x.code() == "services.gerer" {
                    assert_eq!(*p, Portee::Etablissement, "{r:?} {}", x.code());
                }
            }
        }
        assert_eq!(Action::parse("personnels.gerer"), Some(PersonnelsGerer));
        assert_eq!(PersonnelsGerer.portees_admises(), &[Portee::Etablissement]);
        // Inclusion explicite : gérer inclut lire, et rien d'autre.
        assert_eq!(PersonnelsLire.accordee_par(), &[PersonnelsGerer]);
        assert!(PersonnelsAnnuaire.accordee_par().is_empty());
        assert!(PersonnelsAccesGerer.accordee_par().is_empty());
        assert_eq!(Action::parse("services.gerer"), Some(ServicesGerer));
        assert_eq!(PersonnelsServicesLire.accordee_par(), &[ServicesGerer]);
    }

    /// Lot P4 : `missions.gerer` = `referentiel.gerer` (Admin, Direction, Administratif) ; types propres :
    /// Admin seul ; revue des accès : Admin et Direction ; portées nouvelles et saisie d'enseignement.
    #[test]
    fn p4_missions_portees_et_actions_de_saisie() {
        use Action::*;
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof,
                  Role::Infirmerie, Role::Psy, Role::Accueil, Role::Parent, Role::Eleve] {
            let m = modele_par_defaut(r);
            let a = |x: Action| m.iter().filter(|(y, _)| *y == x).count();
            assert_eq!(
                a(MissionsGerer),
                usize::from(matches!(r, Role::Admin | Role::Direction | Role::Administratif)),
                "{r:?} missions.gerer"
            );
            assert_eq!(a(MissionsTypesGerer), usize::from(r == Role::Admin), "{r:?} missions.types.gerer");
            assert_eq!(a(AccesRevue), usize::from(matches!(r, Role::Admin | Role::Direction)), "{r:?} acces.revue");
        }
        for x in [MissionsGerer, MissionsTypesGerer, AccesRevue] {
            assert_eq!(x.portees_admises(), &[Portee::Etablissement], "{x:?} : établissement seulement");
            assert!(x.accordee_par().is_empty(), "{x:?} : aucune inclusion implicite");
        }
        assert_eq!(Action::parse("missions.gerer"), Some(MissionsGerer));
        assert_eq!(Action::parse("acces.revue"), Some(AccesRevue));
        // Les actions de saisie de l'enseignant exigent une intervention qui saisit.
        for x in [NotesSaisir, CompetencesEvaluer, CahierEcrire, AbsencesDeclarer] {
            assert!(x.est_saisie_enseignement(), "{x:?}");
        }
        for x in [NotesLire, CahierLire, DossierLire, AbsencesLire, MissionsGerer] {
            assert!(!x.est_saisie_enseignement(), "{x:?}");
        }
        // Portées : `mon_perimetre` et `sites_designes` s'ajoutent aux actions d'élèves ;
        // `moi` est définie mais aucune action ne l'admet encore (lot P6).
        assert!(NotesLire.portees_admises().contains(&Portee::MonPerimetre));
        assert!(NotesLire.portees_admises().contains(&Portee::SitesDesignes));
        assert!(Action::TOUTES.iter().all(|x| !x.portees_admises().contains(&Portee::Moi)));
        assert_eq!(Portee::parse("mon_perimetre"), Some(Portee::MonPerimetre));
        assert_eq!(Portee::parse("moi"), Some(Portee::Moi));
        assert_eq!(Portee::parse("sites_designes"), Some(Portee::SitesDesignes));
        assert_eq!(Portee::TOUTES.len(), 7);
        // L'anti-escalade ne confond pas les portées nouvelles avec l'établissement.
        assert!(!Portee::MonPerimetre.couvre(Portee::Etablissement));
        assert!(!Portee::MonPerimetre.couvre(Portee::MesClasses));
        assert!(Portee::Etablissement.couvre(Portee::MonPerimetre));
        // Le modèle d'un professeur ne reçoit AUCUNE de ces portées par défaut.
        assert!(modele_par_defaut(Role::Prof)
            .iter()
            .all(|(_, p)| !matches!(p, Portee::MonPerimetre | Portee::Moi | Portee::SitesDesignes)));
    }

    /// E1 : les nouvelles actions « élève » reproduisent exactement les capacités
    /// historiques (`Role::can_*`) pour chaque profil — aucun droit gagné ni perdu.
    #[test]
    fn e1_modele_identique_aux_capacites_historiques() {
        use Action::*;
        let tous = [
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
        ];
        let a_action = |r: Role, x: Action| {
            let m = modele_par_defaut(r);
            let dir: Vec<_> = m.iter().filter(|(y, _)| *y == x).collect();
            assert!(dir.iter().all(|(_, p)| *p == Portee::Etablissement), "{r:?} {x:?}");
            !dir.is_empty()
        };
        for r in tous {
            // Lecture : tout le personnel (ancien `can_read_all_eleves`), pas les familles.
            if r == Role::Prof {
                // D-022 : portée « mes classes » (classes et groupes), jamais l'établissement entier.
                let portees: Vec<Portee> = modele_par_defaut(r)
                    .into_iter()
                    .filter(|(y, _)| *y == ElevesAnnuaire)
                    .map(|(_, p)| p)
                    .collect();
                assert_eq!(portees, vec![Portee::MesClasses], "Prof annuaire");
            } else {
                assert_eq!(a_action(r, ElevesAnnuaire), r.can_read_all_eleves(), "{r:?} annuaire");
            }
            // Écritures et historique : ancien `can_manage_eleves`.
            for x in [
                ElevesCreer,
                ElevesIdentiteModifier,
                ElevesIdentifiantsGerer,
                ElevesScolariteGerer,
                ElevesSortie,
                ElevesPropositionsTraiter,
                ElevesExporter,
                ElevesHistoriqueLire,
            ] {
                assert_eq!(a_action(r, x), r.can_manage_eleves(), "{r:?} {}", x.code());
            }
            // Identifiants (INE, badge) et détail de scolarité : niveau « complet ».
            assert_eq!(a_action(r, ElevesIdentifiantsLire), r.can_manage_eleves(), "{r:?}");
            assert_eq!(a_action(r, ElevesLire), r.can_manage_eleves(), "{r:?}");
            // État civil : niveau « complet » et « santé ».
            assert_eq!(
                a_action(r, ElevesEtatCivilLire),
                r.can_manage_eleves() || r == Role::Infirmerie,
                "{r:?} état civil"
            );
            // Gouvernance : direction (fusion, confidentialité), admin (paramétrage).
            let direction = matches!(r, Role::Admin | Role::Direction);
            assert_eq!(a_action(r, ElevesFusionner), direction);
            assert_eq!(a_action(r, ElevesConfidentielGerer), direction);
            assert_eq!(a_action(r, ElevesParametrer), r == Role::Admin);
            assert_eq!(
                a_action(r, ElevesPhotoGerer),
                matches!(r, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe)
            );
            // ELV-41 : le badge perdu est géré aussi par la vie scolaire (CPE).
            assert_eq!(
                a_action(r, ElevesBadgeGerer),
                matches!(r, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe)
            );
        }
        // `eleves.gerer` reste dans le catalogue et accompagne les nouvelles écritures.
        for r in tous {
            assert_eq!(a_action(r, ElevesGerer), a_action(r, ElevesCreer), "{r:?}");
        }
        // Inclusions explicites (§12.2), et aucune autre.
        assert_eq!(ElevesEtatCivilLire.accordee_par(), &[ElevesIdentiteModifier]);
        assert_eq!(ElevesIdentifiantsLire.accordee_par(), &[ElevesIdentifiantsGerer]);
        assert!(ElevesLire.accordee_par().is_empty());
        // Portées : lectures par groupe sur 4 portées, le reste d'établissement.
        assert_eq!(ElevesAnnuaire.portees_admises().len(), 4);
        assert_eq!(ElevesLire.portees_admises().len(), 4);
        assert_eq!(ElevesFusionner.portees_admises(), &[Portee::Etablissement]);
        assert_eq!(Action::parse("eleves.confidentiel.gerer"), Some(ElevesConfidentielGerer));
    }

    #[test]
    fn couverture_des_portees() {
        assert!(Portee::Etablissement.couvre(Portee::MesGroupes));
        assert!(Portee::MesClasses.couvre(Portee::MesClasses));
        assert!(!Portee::MesClasses.couvre(Portee::Etablissement));
        assert!(!Portee::MesGroupes.couvre(Portee::MesClasses));
    }
}

/// Oracle A1 : la matrice profils × actions **historique** (les 53 actions d'avant le lot A1) est
/// figée telle qu'elle était avant ce lot (relevé du modèle par défaut à la base du lot). Seules
/// les actions ajoutées par A1 (indices 53 et suivants de `TOUTES`) peuvent différer : leurs écarts
/// avec le comportement historique des écrans sont documentés dans `04_ADMINISTRATIF.md` §22.
#[cfg(test)]
mod oracle_a1 {
    use super::*;

    const HISTORIQUE: [(&str, &[&str]); 10] = [
        ("admin", &[
            "absences.declarer@etablissement",
            "absences.justifier@etablissement",
            "absences.lire@etablissement",
            "annonces.publier@etablissement",
            "appel.verifier@etablissement",
            "assiduite.signaler@etablissement",
            "audit.lire@etablissement",
            "bourses.gerer@etablissement",
            "cahier.ecrire@etablissement",
            "cahier.lire@etablissement",
            "cahier.viser@etablissement",
            "competences.evaluer@etablissement",
            "conseils.gerer@etablissement",
            "discipline.gerer@etablissement",
            "docs_a_signer.publier@etablissement",
            "documents.deposer@etablissement",
            "documents.gerer_droits@etablissement",
            "documents.lire@etablissement",
            "documents.lire_sante@etablissement",
            "dossier.lire@etablissement",
            "edt.gerer@etablissement",
            "eleves.gerer@etablissement",
            "facturation.gerer@etablissement",
            "facturation.lire@etablissement",
            "imports.appliquer@etablissement",
            "incidents.declarer@etablissement",
            "inscriptions.decider@etablissement",
            "inscriptions.gerer@etablissement",
            "internat.gerer@etablissement",
            "journal.lire@etablissement",
            "liens.suspendre@etablissement",
            "matieres.gerer@etablissement",
            "messagerie.eleve@etablissement",
            "notes.lire@etablissement",
            "notes.saisir@etablissement",
            "payeurs.definir@etablissement",
            "personnes.fusionner@etablissement",
            "personnes.gerer@etablissement",
            "psy.lire@etablissement",
            "psy.saisir@etablissement",
            "punitions.donner@etablissement",
            "referentiel.gerer@etablissement",
            "referentiel.lire@etablissement",
            "repas.gerer@etablissement",
            "repas.scanner@etablissement",
            "roles.gerer@etablissement",
            "sanctions.prononcer@etablissement",
            "sante.lire@etablissement",
            "sante.saisir@etablissement",
            "securite.politique@etablissement",
            "sorties.controler@etablissement",
            "sorties.gerer@etablissement",
            "suivi.eleve@etablissement",
        ]),
        ("direction", &[
            "absences.declarer@etablissement",
            "absences.justifier@etablissement",
            "absences.lire@etablissement",
            "annonces.publier@etablissement",
            "appel.verifier@etablissement",
            "assiduite.signaler@etablissement",
            "audit.lire@etablissement",
            "bourses.gerer@etablissement",
            "cahier.ecrire@etablissement",
            "cahier.lire@etablissement",
            "cahier.viser@etablissement",
            "competences.evaluer@etablissement",
            "conseils.gerer@etablissement",
            "discipline.gerer@etablissement",
            "docs_a_signer.publier@etablissement",
            "documents.deposer@etablissement",
            "documents.gerer_droits@etablissement",
            "documents.lire@etablissement",
            "documents.lire_sante@etablissement",
            "dossier.lire@etablissement",
            "edt.gerer@etablissement",
            "eleves.gerer@etablissement",
            "facturation.gerer@etablissement",
            "facturation.lire@etablissement",
            "imports.appliquer@etablissement",
            "incidents.declarer@etablissement",
            "inscriptions.decider@etablissement",
            "inscriptions.gerer@etablissement",
            "internat.gerer@etablissement",
            "journal.lire@etablissement",
            "liens.suspendre@etablissement",
            "matieres.gerer@etablissement",
            "messagerie.eleve@etablissement",
            "notes.lire@etablissement",
            "notes.saisir@etablissement",
            "payeurs.definir@etablissement",
            "personnes.fusionner@etablissement",
            "personnes.gerer@etablissement",
            "psy.lire@etablissement",
            "psy.saisir@etablissement",
            "punitions.donner@etablissement",
            "referentiel.gerer@etablissement",
            "referentiel.lire@etablissement",
            "repas.gerer@etablissement",
            "repas.scanner@etablissement",
            "roles.gerer@etablissement",
            "sanctions.prononcer@etablissement",
            "sante.lire@etablissement",
            "sante.saisir@etablissement",
            "securite.politique@etablissement",
            "sorties.controler@etablissement",
            "sorties.gerer@etablissement",
            "suivi.eleve@etablissement",
        ]),
        ("administratif", &[
            "absences.declarer@etablissement",
            "absences.justifier@etablissement",
            "absences.lire@etablissement",
            "annonces.publier@etablissement",
            "appel.verifier@etablissement",
            "audit.lire@etablissement",
            // F0 (FD-27) : « bourses.gerer » et « facturation.gerer » retirés par défaut du
            // secrétariat (séparation des tâches) ; « facturation.lire » est conservé.
            "cahier.lire@etablissement",
            "conseils.gerer@etablissement",
            "docs_a_signer.publier@etablissement",
            "documents.deposer@etablissement",
            "documents.gerer_droits@etablissement",
            "documents.lire@etablissement",
            "documents.lire_sante@etablissement",
            "dossier.lire@etablissement",
            "edt.gerer@etablissement",
            "eleves.gerer@etablissement",
            "facturation.lire@etablissement",
            "imports.appliquer@etablissement",
            "incidents.declarer@etablissement",
            "inscriptions.gerer@etablissement",
            "internat.gerer@etablissement",
            "journal.lire@etablissement",
            "matieres.gerer@etablissement",
            "messagerie.eleve@etablissement",
            "notes.lire@etablissement",
            "payeurs.definir@etablissement",
            "personnes.gerer@etablissement",
            "punitions.donner@etablissement",
            "referentiel.gerer@etablissement",
            "referentiel.lire@etablissement",
            "repas.gerer@etablissement",
            "repas.scanner@etablissement",
            "sanctions.prononcer@etablissement",
            "sorties.controler@etablissement",
            "sorties.gerer@etablissement",
            "suivi.eleve@etablissement",
        ]),
        ("cpe", &[
            "absences.declarer@etablissement",
            "absences.justifier@etablissement",
            "absences.lire@etablissement",
            "annonces.publier@etablissement",
            "appel.verifier@etablissement",
            "assiduite.signaler@etablissement",
            "cahier.lire@etablissement",
            "conseils.gerer@etablissement",
            "docs_a_signer.publier@etablissement",
            "dossier.lire@etablissement",
            "edt.gerer@etablissement",
            "incidents.declarer@etablissement",
            "internat.gerer@etablissement",
            "journal.lire@etablissement",
            "messagerie.eleve@etablissement",
            "notes.lire@etablissement",
            "punitions.donner@etablissement",
            "referentiel.lire@etablissement",
            "repas.scanner@etablissement",
            "sanctions.prononcer@etablissement",
            "sorties.controler@etablissement",
            "sorties.gerer@etablissement",
            "suivi.eleve@etablissement",
        ]),
        // RP1-07 (recette personas 1) : la comptabilité n'a ni `journal.lire` ni `suivi.eleve`
        // (écart voulu avec la matrice historique figée par A1).
        ("compta", &[
            "bourses.gerer@etablissement",
            "dossier.lire@etablissement",
            "facturation.gerer@etablissement",
            "facturation.lire@etablissement",
            "payeurs.definir@etablissement",
            "referentiel.lire@etablissement",
            "repas.gerer@etablissement",
            "repas.scanner@etablissement",
        ]),
        ("prof", &[
            "absences.lire@mes_classes",
            "cahier.ecrire@mes_classes",
            "cahier.lire@mes_classes",
            "competences.evaluer@mes_classes",
            "conseils.gerer@etablissement",
            "dossier.lire@mes_classes",
            "incidents.declarer@etablissement",
            "journal.lire@mes_classes",
            "messagerie.eleve@mes_classes",
            "notes.lire@mes_classes",
            "notes.saisir@mes_classes",
            "punitions.donner@mes_classes",
            "referentiel.lire@etablissement",
            "suivi.eleve@mes_classes",
        ]),
        ("infirmerie", &[
            "documents.lire_sante@etablissement",
            "dossier.lire@etablissement",
            "journal.lire@etablissement",
            "referentiel.lire@etablissement",
            "sante.lire@etablissement",
            "sante.saisir@etablissement",
            "suivi.eleve@etablissement",
        ]),
        ("psy", &[
            "dossier.lire@etablissement",
            "journal.lire@etablissement",
            "psy.lire@etablissement",
            "psy.saisir@etablissement",
            "referentiel.lire@etablissement",
            "suivi.eleve@etablissement",
        ]),
        ("parent", &[
        ]),
        ("eleve", &[
        ]),
    ];

    /// Les 53 actions historiques (catalogue de la base du lot A1). Depuis l'intégration avec
    /// le socle 2 (E1 : `eleves.*`) et D-022 (`exports.officiels`), elles ne forment plus un
    /// préfixe du catalogue : on les isole par position relative, jamais par un indice figé.
    fn historiques() -> Vec<Action> {
        let pos = |a: Action| Action::TOUTES.iter().position(|x| *x == a).unwrap();
        let (e1_debut, e1_fin, fin) = (
            pos(Action::ElevesAnnuaire),
            pos(Action::ElevesParametrer),
            pos(Action::CahierViser),
        );
        Action::TOUTES
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < e1_debut || (*i > e1_fin && *i <= fin))
            .map(|(_, a)| *a)
            .collect()
    }

    #[test]
    fn matrice_historique_inchangee_par_a1() {
        for (code_role, attendu) in HISTORIQUE {
            let role = Role::parse(code_role).unwrap();
            let mut obtenu: Vec<String> = modele_par_defaut(role)
                .into_iter()
                .filter(|(a, _)| historiques().contains(a))
                .map(|(a, p)| format!("{}@{}", a.code(), p.code()))
                .collect();
            obtenu.sort();
            let attendu: Vec<String> = attendu.iter().map(|x| x.to_string()).collect();
            assert_eq!(obtenu, attendu, "profil {code_role} : la matrice historique a changé");
        }
        // Les actions historiques sont bien au nombre de 53.
        assert_eq!(historiques().len(), 53);
    }

    /// Le profil « accueil » est nouveau : il n'a aucun droit historique.
    #[test]
    fn accueil_n_a_aucun_droit_historique_hors_declaration_et_referentiel() {
        let historiques: Vec<Action> = modele_par_defaut(Role::Accueil)
            .into_iter()
            .map(|(a, _)| a)
            .filter(|a| historiques().contains(a))
            .collect();
        assert_eq!(historiques, vec![Action::ReferentielLire, Action::AbsencesDeclarer]);
    }

    /// Chaque action ajoutée par A1 respecte ses portées admises dans tous les modèles.
    #[test]
    fn modeles_a1_respectent_les_portees() {
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof, Role::Infirmerie, Role::Psy, Role::Accueil] {
            for (a, p) in modele_par_defaut(r) {
                assert!(a.portees_admises().contains(&p), "{r:?} {a:?} {p:?}");
            }
        }
        // `responsables.protege.*`, `remise.interdire` et le bris de glace : établissement seul.
        for a in [Action::ResponsablesProtegeLire, Action::ResponsablesProtegeGerer, Action::RemiseInterdire, Action::ResponsablesAccesUrgence] {
            assert_eq!(a.portees_admises(), &[Portee::Etablissement]);
        }
        // Le bris de glace n'est dans aucun modèle hors Direction.
        for r in [Role::Admin, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof, Role::Infirmerie, Role::Psy, Role::Accueil] {
            assert!(!modele_par_defaut(r).iter().any(|(a, _)| *a == Action::ResponsablesAccesUrgence), "{r:?}");
        }
        assert!(modele_par_defaut(Role::Direction).iter().any(|(a, _)| *a == Action::ResponsablesAccesUrgence));
        // Familles et élèves : aucun droit paramétrable.
        assert!(modele_par_defaut(Role::Parent).is_empty() && modele_par_defaut(Role::Eleve).is_empty());
    }

    /// Socle 42, lot V0a : modèles par défaut des onze actions `voyages.*` (`42_SORTIES_VOYAGES.md` §21.1).
    #[test]
    fn voyages_v0a_modeles_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        // Tout le personnel (jamais une famille ni un élève) lit la liste minimale.
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof, Role::Infirmerie, Role::Psy, Role::Accueil] {
            assert!(a(r, VoyagesAnnuaire), "{r:?} voyages.annuaire");
        }
        for r in [Role::Parent, Role::Eleve] {
            assert!(modele_par_defaut(r).is_empty(), "{r:?} : aucun droit paramétrable");
        }
        // Création : professeurs, CPE, secrétariat, direction ; pas l'accueil ni la comptabilité.
        for (r, attendu) in [
            (Role::Prof, true), (Role::Cpe, true), (Role::Administratif, true), (Role::Direction, true),
            (Role::Admin, true), (Role::Accueil, false), (Role::Compta, false), (Role::Infirmerie, false),
        ] {
            assert_eq!(a(r, VoyagesCreer), attendu, "{r:?} voyages.creer");
        }
        // Un professeur ne lit ni ne valide, ne modifie pas les sorties des autres, n'écrit pas aux prestataires.
        for x in [VoyagesLire, VoyagesModifier, VoyagesParticipants, VoyagesEncadrement, VoyagesValider,
                  VoyagesAutoriser, VoyagesBudgetValider, VoyagesPorteur, VoyagesPrestataires] {
            assert!(!a(Role::Prof, x), "professeur : {x:?}");
            assert!(!a(Role::Accueil, x), "accueil : {x:?}");
        }
        // Validation et autorisation : direction et administrateur seulement ; budget : comptabilité.
        for (r, v, au, b) in [
            (Role::Direction, true, true, false), (Role::Admin, true, true, true),
            (Role::Administratif, false, false, false), (Role::Compta, false, false, true),
            (Role::Cpe, false, false, false),
        ] {
            assert_eq!(a(r, VoyagesValider), v, "{r:?} valider");
            assert_eq!(a(r, VoyagesAutoriser), au, "{r:?} autoriser");
            assert_eq!(a(r, VoyagesBudgetValider), b, "{r:?} budget_valider");
        }
        // Prestataires (défaut de Next.js corrigé) : secrétariat, comptabilité, direction.
        for (r, attendu) in [(Role::Administratif, true), (Role::Compta, true), (Role::Direction, true), (Role::Prof, false), (Role::Cpe, false)] {
            assert_eq!(a(r, VoyagesPrestataires), attendu, "{r:?} prestataires");
        }
        // Codes lisibles et uniques ; établissement entier seulement.
        for x in [VoyagesAnnuaire, VoyagesLire, VoyagesCreer, VoyagesModifier, VoyagesParticipants, VoyagesEncadrement,
                  VoyagesValider, VoyagesAutoriser, VoyagesBudgetValider, VoyagesPorteur, VoyagesPrestataires] {
            assert!(x.code().starts_with("voyages."));
            assert_eq!(Action::parse(x.code()), Some(x));
            assert_eq!(x.portees_admises(), &[Portee::Etablissement]);
        }
    }
}
