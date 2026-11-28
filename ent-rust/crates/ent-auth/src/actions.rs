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
    /// Socle 42, lot V0d : reprise des séjours de Next.js (zone de reprise, tableau de bascule, bascule). Administrateur et direction.
    VoyagesReprise,
    /// Socle 42, lot V2a : argent des sorties en mode suivi (parts nominatives, versements, plans, avoirs proposés, acomptes fournisseurs, export). Comptabilité, direction, administrateur.
    VoyagesArgent,
    /// Socle 42, lot V2a : recevoir un bordereau de chèques de l'organisateur et pointer ses lignes (secrétariat). Ne donne pas la liste nominative des paiements.
    VoyagesRemise,
    // ── Socle 32, lots ET0–ET1 : espaces de travail (`32_BUREAUTIQUE.md` §20.1) ──
    /// Utiliser son espace personnel et les espaces dont on est membre, et voir ce qui est partagé avec soi.
    /// Les rôles d'espace (animateur, contributeur, lecteur) restent vérifiés par le service pour chaque espace.
    EspacesUtiliser,
    /// Créer un espace libre entre personnels (sans élève, sans ancrage).
    EspacesCreer,
    /// Partager un de ses éléments ou dossiers avec des personnes nommées de l'établissement (jamais de lien public).
    EspacesPartager,
    /// Consulter le journal des espaces de l'établissement (direction) : jamais les espaces personnels ni le contenu.
    EspacesJournalLire,
    // ── Socle 24, lot F1 : droits fins de la facturation (§19.1). ──
    /// Vue guichet : le payeur présent ou authentifié, son solde, ses échéances, ses documents ; aucune navigation entre comptes, aucun tableau d'impayés (socle 24, R-F37).
    FacturationGuichet,
    /// Émettre factures et avoirs (confirmation renforcée, socle 24).
    FacturationEmettre,
    /// Créer des avoirs et corriger des pièces émises (socle 24).
    FacturationAvoirs,
    /// Accorder un échéancier dérogatoire après émission (socle 24).
    FacturationEcheanciers,
    /// Accorder ou retirer des réductions (socle 24).
    FacturationReductions,
    /// Gérer articles, tarifs et définitions de réductions (socle 24).
    FacturationTarifs,
    /// Enregistrer le vote et corriger une grille votée (socle 24, R-F23).
    FacturationVoter,
    /// Simuler des tarifs et comparer des scénarios (socle 24).
    FacturationSimuler,
    /// Paramètres, identité émettrice, séries de numérotation (socle 24).
    FacturationParametrer,
    /// Comptes débiteurs, acceptation et pièce de répartition, prises en charge, transfert vers l'élève majeur (socle 24).
    FacturationEngagements,
    /// Clôturer une période comptable ; la réouverture est réservée à la direction (socle 24).
    FacturationCloturer,
    /// Écritures de ventes et exports comptables (socle 24).
    FacturationExporter,
    /// Tableaux agrégés pour la direction et le trésorier (socle 24).
    FacturationPilotage,
    /// Journal financier en ajout seul : qui, quoi, quand (socle 24, R-F27).
    FacturationJournal,
    /// Reprise de soldes et double tenue avec l'ancien logiciel (socle 24).
    FacturationReprise,
    /// Publier la grille des tarifs (socle 24).
    FacturationPublier,
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
    /// Socle 41, lot I2 : lire l'attendu, les appels, les cas et l'effectif de nuit (portée : tout
    /// l'internat ou les élèves désignés, par exemple une formation, R-IT30).
    InternatLire,
    /// Socle 41, lot I2 : ouvrir un appel de nuit et pointer les élèves de son périmètre.
    InternatAppel,
    /// Socle 41, lot I2 : clore et valider un appel, le corriger après validation, tenir un cas.
    InternatAppelValider,
    /// Socle 41, lot I3 : décider des absences annoncées et des sorties des internes (accepter, refuser,
    /// traiter une annonce tardive, régulariser, trancher un désaccord entre titulaires).
    InternatSortiesDecider,
    /// Socle 41, lot I4 : écrire un fait de liaison codé (jour / nuit) sur un interne de son périmètre.
    InternatLiaisonEcrire,
    /// Socle 41, lot I4 : lire la liaison et les événements de nuit (équipe d'internat, CPE, direction ; R-IT8).
    InternatLiaisonLire,
    /// Socle 41, lot I4 : lire, le matin, les seuls faits de liaison marqués « pour le jour » (vie scolaire de jour).
    InternatLiaisonJour,
    /// Socle 41, lot I4 : déclarer et compléter un événement de nuit, ouvrir une préoccupation.
    InternatEvenementDeclarer,
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
    /// Transverse « individu » (migration 279) : relier deux fiches du même humain (deux comptes, agent et fiche
    /// parent sans compte), consulter les liens et les propositions, écarter une proposition. Ne fait que restreindre.
    PersonnesIndividuRelier,
    /// Lever un lien d'individu (retire des refus) : direction seule, jamais l'individu concerné.
    PersonnesIndividuLever,
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
    // ── Socle 19, lot AG1 : agenda (19_AGENDA_RDV.md §21.1) ─────────────────
    /// Créer des événements et des réunions (brouillon, puis invitations nominatives) : tout le personnel.
    AgendaEvenementsCreer,
    /// Publier un événement à une audience (classes, niveau, établissement, personnels) : direction, CPE ;
    /// un rôle paramétré (D-017) peut l'accorder à un professeur principal pour sa classe.
    AgendaEvenementsPublier,
    /// Lire l'agenda d'une classe ou d'un groupe (non nominatif) : tout le personnel.
    AgendaLireStructure,
    /// Lire les journaux de l'agenda : direction, administrateur.
    AgendaJournalLire,
    /// Paramétrer l'agenda (types d'événements de l'établissement) : administrateur.
    AgendaParametrer,
    /// Tenir la feuille de semaine de l'établissement (créneaux visibles de tous) : direction, administrateur.
    AgendaSemaineGerer,
    /// Rechercher une salle libre et lire le planning des salles (sans élèves) : tout le personnel.
    AgendaLieuxLire,
    /// Réserver une salle (soi, ou pour un collègue) : tout le personnel.
    AgendaLieuxReserver,
    /// Valider ou refuser les demandes des salles « sur validation » : direction, administrateur (un valideur
    /// est aussi désigné salle par salle).
    AgendaLieuxValider,
    /// Gérer toutes les réservations (annuler celle d'un autre avec motif) : direction, administrateur, secrétariat.
    AgendaLieuxGerer,
    /// Paramétrer les règles d'une salle (réservable toujours, sur validation, jamais ; délais, durée, horizon).
    AgendaLieuxParametrer,
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
    /// Socle 8 (N1) : voir un avis, ses destinataires retenus ou exclus et l'état de ses remises (sans adresse ni numéro).
    NotificationsAvisVoir,
    /// Socle 8 (N1) : régler les paramètres de l'établissement et la bascule par nature (heures calmes, délais).
    NotificationsParametrer,
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
    // ── Socle 15, lots A0–A1 : absences des personnels (`15_ABSENCES_PERSONNELS.md` §23.1) ──
    /// Déclarer l'absence d'un AUTRE agent (secrétariat, comptabilité, direction). Déclarer la sienne ne demande aucun droit (portée « moi » traitée par le service : aucune action n'admet encore `Portee::Moi`).
    PersonnelsAbsencesDeclarer,
    /// Signaler une absence sans nature (accueil, vie scolaire, direction) : aucun effet extérieur tant qu'elle n'est pas confirmée (R-AP46).
    PersonnelsAbsencesSignaler,
    /// Lire les absences des personnels SANS la nature (adjoint, vie scolaire, direction).
    PersonnelsAbsencesLireOrganisation,
    /// Lire la nature, la décision et la zone de qualification : chef d'établissement et secrétariat. Jamais l'administrateur technique, le DPO ni la comptabilité hors circuit (R-AP2).
    PersonnelsAbsencesLireNature,
    /// Valider, refuser, requalifier, rouvrir, annuler. Jamais sa propre absence (R-AP10).
    PersonnelsAbsencesValider,
    /// Instruire : date de réception de l'arrêt, traitement administratif (secrétariat, comptabilité de l'OGEC).
    PersonnelsAbsencesInstruire,
    /// ★ Saisir une absence déjà validée (régularisation, convocation reçue) : droit réservé (AD-05).
    PersonnelsAbsencesSaisirValidee,
    /// ★ Natures de l'établissement, règles de circuit, délais (S-1500 à S-1517).
    PersonnelsAbsencesParametrer,
    /// Socle 29 : lancer un rapprochement de l'index des tâches et lire son rapport (POR-205).
    AfaireRapprocher,
    /// Socle 29, lot P4 : prendre en charge une tâche d'une file collective dont on est membre (POR-152).
    AfairePrendre,
    /// Socle 29, lot P4 : réattribuer une tâche prise à un collègue de la même file (POR-154), journalisé.
    AfaireReattribuer,
    /// Socle 29, lot P4 : superviser les files par nombres (ouvertes, en retard, délai médian), jamais le
    /// contenu nominatif ni un classement de personnes (POR-161, POR-168).
    AfaireSuperviser,
    /// Socle 29, lot P5 : recherche globale côté serveur (élèves de sa portée, personnes, personnels, classes,
    /// groupes, salles), plafonnée par jour ; le texte des recherches n'est jamais conservé (R-PO19).
    RechercheGlobale,
    /// Socle 29, lot P5 : lever un blocage anti-énumération (POR-196), journalisé.
    RechercheDebloquer,
    /// ★ Conservation des absences de personnel (A2) : gel (contentieux), purge à la demande, purges à venir,
    /// confidentialité renforcée (ABP-161 à 164). Direction par défaut ; réaffectable au DPO par un rôle paramétrable.
    PersonnelsAbsencesConserver,
    /// Absence longue et suppléance (A5) : demander une suppléance au rectorat et en tracer la réponse (ABP-180),
    /// nommer un suppléant sur le service de l'absent (ABP-181, avec `services.gerer` du socle 6). Direction et secrétariat.
    PersonnelsAbsencesSuppleer,
    /// Remplacement de courte durée (A6, ABP-220 à 228) : tenir le plan annuel versionné, lancer la campagne,
    /// gérer les engagements des volontaires et lire le vivier. Direction.
    PersonnelsRcdGerer,
    /// Remontée hebdomadaire RCD (A6, ABP-229 à 231) : préparer, contrôler, laisser partir, exporter le tableau
    /// équivalent. Direction et secrétariat. La remontée ne porte jamais de nom ni de nature fine.
    PersonnelsRcdRemontee,
    /// Socle 10 (emploi du temps), lot T3 (EDT-160 à 168, 173) : modifier le réel d'une séance (annuler, déplacer,
    /// changer l'horaire, ajouter, fusionner, changer l'intervenant) et lever une modification. Direction et secrétariat.
    EdtModifier,
    /// Socle 10, lot T3 (EDT-162) : changer le lieu d'une séance ou d'une série. Direction, secrétariat, vie scolaire.
    EdtModifierLieu,
    /// Socle 10, lot T3 (EDT-169 à 171) : lots (annulation en masse, grève du jour, salle fermée) avec aperçu serveur.
    EdtLotAppliquer,
    /// Socle 10, lot T3 (EDT-173) : corriger une séance passée (motif obligatoire, effets en aval signalés). Direction.
    EdtCorrigerPasse,
    /// Socle 10, lot T3 : lire le motif INTERNE d'une modification (jamais diffusé). Direction et secrétariat.
    EdtMotifInterneLire,
    /// Socle 10, lot T3 : lire le journal des modifications (motif public seulement). Personnel d'encadrement.
    EdtModificationsLire,
    /// Socle 10, lot T3 (§9.3) : forcer un conflit dur d'emploi du temps avec un motif journalisé (cas réel : deux
    /// demi-groupes fusionnés dans la même salle). Direction seule : le secrétariat propose, il ne force pas.
    EdtForcerConflit,
    /// Socle 10, lot T4 (§14.3) : le tableau « demain » (absences connues, séances non assurées, remplacements, salles
    /// fermées, permanences attendues, notifications) et les élèves sans cours. Direction, secrétariat, vie scolaire.
    EdtDemainLire,
    /// Socle 10, lot T4 (§14.3, R-E27) : « maintenant » — la séance en cours d'un élève (lecture journalisée), l'occupation
    /// d'une salle, la présence d'un professeur selon l'emploi du temps. Direction, secrétariat, vie scolaire, accueil.
    EdtMaintenantLire,
    /// Socle 10, lot T5 (§14.1, EDT-150 et 154) : contrôler une version avant publication, voir l'aperçu, l'historique des
    /// versions et les différences entre deux versions. Direction et secrétariat (l'adjoint chargé de l'emploi du temps).
    EdtPublicationLire,
    /// Socle 10, lot T5 (§14.1, EDT-151 à 153 et 156) : publier une version (aux personnels, puis diffusion aux familles),
    /// avancer ou retarder la diffusion, publier une version individuelle. Direction et secrétariat.
    EdtPublier,
    /// Socle 10, lot T5 (EDT-155) : revenir à la version précédente (avant sa diffusion, ou à une date future ; le passé ne
    /// change pas). Direction seule.
    EdtRevenirVersion,
    /// Heures constatées (A7, ABP-200, 213, 214, 219) : lire les heures d'un agent et l'état annuel, dans son périmètre
    /// d'employeur. Direction et secrétariat (l'OGEC lit par `paie.absences`).
    PersonnelsHeuresLire,
    /// Heures constatées (A7, ABP-201 à 204, 208) : valider, corriger, saisir sans séance, figer l'état mensuel,
    /// régulariser. Direction.
    PersonnelsHeuresValider,
    /// État mensuel des heures (A7, ABP-205) ★ : le chef d'établissement signe. Direction.
    PersonnelsHeuresSigner,
    /// Exports (A7, ABP-206) ★ : exporter l'état mensuel signé vers le rectorat ou la paie, figer et régulariser dans
    /// son employeur. Direction, secrétariat (État), comptabilité (OGEC).
    PersonnelsHeuresExporter,
    /// Paie de l'OGEC (A7, ABP-207, 209 à 211) : éléments variables de paie, heures et jours d'absence, file DSN J+5.
    /// Direction et comptabilité.
    PersonnelsPaieAbsences,
    /// Absences collectives (A9, ABP-253, 255, 256) : cocher les grévistes du matin, annuler en lot leurs séances, tableau
    /// de crise. Direction (adjoint, chargé des remplacements). La liste des grévistes, nature sensible, n'est lisible
    /// que de la direction et du secrétariat (`personnels.absences.lire_nature`).
    PersonnelsGreveGerer,
    /// Socle 39 (stages), lot ST0 : voir les stages d'un élève (établissement ; professeur : ses
    /// élèves suivis). Le CPE et l'accueil n'ont que la projection « en stage » (lot ST1).
    StagesLire,
    /// Socle 39, lot ST0 : déclarer un stage et le clôturer ou l'annuler (corrige RS-02 : le CPE
    /// ne crée plus).
    StagesPreparer,
    /// Socle 39, lot ST0 : tenir le carnet d'organismes (entreprises, administrations, associations).
    StagesOrganismesGerer,
    /// Socle 39, lot ST1 (a) : dispositifs, assurance, délibération, quotas, délais de relance
    /// (direction) ; décider qu'un organisme est « interdit ».
    StagesParametrer,
    /// Socle 39, lot ST1 (a) : planifier, modifier, publier et fermer les périodes de stage.
    StagesPeriodesGerer,
    // ── Socle N1 : demandes et flux de travail (`N1_DEMANDES.md` §24.1, D-017) ──
    /// Déposer une demande à l'établissement au nom de soi (natures ouvertes au personnel).
    DemandesDeposer,
    /// Déposer une demande au nom d'une autre personne (accueil, secrétariat, direction) : le déposant est tracé.
    DemandesDeposerPourCompte,
    /// Traiter les demandes des files dont on est membre (membres calculés ; aucun modèle ne le donne à tout un rôle).
    DemandesTraiter,
    /// Réattribuer, déléguer et reporter l'échéance des demandes des files dont on est responsable.
    DemandesFileRepartir,
    /// Superviser les files : compteurs, délais, retards, contenu C0–C1.
    DemandesFileSuperviser,
    /// Lire les demandes C0–C1 de l'établissement (chef d'établissement).
    DemandesLireToutes,
    /// Superviser, de façon motivée et tracée, une file de demandes C2 et plus (geste exceptionnel).
    DemandesSuperviserConfidentiel,
    /// Décider les heures supplémentaires effectives (HSE) demandées par les personnels.
    DemandesHseDecider,
    /// Attester le service fait des HSE (circuit facultatif, désactivé par défaut).
    DemandesHseAttester,
    /// Gérer l'enveloppe d'HSE (en heures) par année et par site.
    DemandesHseEnveloppeGerer,
    /// Valider les tirages en couleur et les gros volumes de reprographie.
    DemandesReproValider,
    /// Tenir la file d'impression des sites désignés (réceptionnaire de reprographie).
    DemandesReproImprimer,
    /// Paramétrer les natures, files, règles de routage et réponses types des demandes.
    DemandesParametrer,
    /// Lire les journaux de paramétrage et d'accès des demandes.
    DemandesJournalLire,
    /// Lire les indicateurs agrégés des demandes.
    DemandesIndicateursLire,
    /// Lancer la reprise des demandes de l'ancienne application.
    DemandesReprise,
    // ── Socle 28, lot R3 : file « à décider » de la reprise depuis Next.js ──
    /// Voir la file « à décider » de la reprise et décider les rapprochements incertains (rattacher, créer, ignorer,
    /// annuler, valider une équivalence de libellé). Secrétariat et direction (`reprise.decider`).
    RepriseDecider,
    // ── Socle 7, lot D2 : documents émis, registre, domaines restreints (`07_DOCUMENTS.md` §18) ──
    /// Émettre un document officiel (certificat, attestation, courrier) pour un élève ou une personne.
    DocumentsEmettre,
    /// Annuler un document émis par erreur (motif obligatoire, journalisé).
    DocumentsAnnuler,
    /// Lire le registre des documents émis et le journal de leurs consultations.
    DocumentsRegistreLire,
    /// Paramétrer l'identité des documents, les entités émettrices et les natures activées.
    DocumentsModelesParametrer,
    /// Lire les documents restreints du domaine justice (jugements, pièces judiciaires).
    DocumentsRestreintsLire,
    /// Lire les documents du domaine financier (avis d'imposition, quotient, bourses).
    DocumentsFinancierLire,
    /// Lire les dossiers documentaires des agents (domaine rh).
    DocumentsPersonnelsLire,
    // ── Socle 7, lots D4 et D5 : coffre (retrait, remise), pièces justificatives ──
    /// Classer : retirer une mise à disposition (motif obligatoire) ou la rendre explicitement (D4).
    DocumentsClasser,
    /// Demander, relancer, annuler une pièce justificative ; voir « ce qui manque » d'un dossier (D5).
    DocumentsPiecesDemander,
    /// Contrôler une pièce déposée : accepter, refuser avec motif, déclarer sans objet (D5).
    DocumentsPiecesControler,
    /// Socle 7, lot D6 : lancer, suivre, relancer, annuler un circuit de signature ; valider un papier signé.
    DocumentsSignaturesGerer,
    /// Socle 7, lot D6 : constater une signature au guichet (agent identifié, identité vue, confirmation).
    DocumentsSignaturesConstater,
    // ── Socle 18, lots FO0–FO1 : accords, autorisations et consentements (`18_FORMULAIRES.md` §23.1) ──
    /// Lire les **verdicts** d'accord (peut / ne peut pas / en attente) des élèves de sa portée, sans motif
    /// ni décideur (R-FC39). Portées : établissement, mes classes, périmètre, désignés.
    AutorisationsLire,
    /// Lire les décisions par décideur, les preuves et les désaccords ; corriger ou annuler une saisie
    /// (lecture journalisée, R-FC38). Établissement seulement.
    AutorisationsGerer,
    /// Activer les objets du catalogue, rédiger et publier les versions de texte et leurs usages. Établissement
    /// seulement.
    AutorisationsParametrer,
    /// Socle 18, lot FO2 : créer, rédiger et publier un formulaire à l'audience de sa portée (établissement, mes classes).
    /// Un formulaire à large audience attend une validation (`formulaires.valider`).
    FormulairesCreer,
    /// Socle 18, lot FO2 : valider un formulaire à large audience (quatre yeux, jamais par son propre auteur). Établissement seulement.
    FormulairesValider,
    /// Socle 27 : voir le tableau de bord de conformité, la politique de conservation et le registre.
    ConformiteLire,
    /// Socle 27 : désigner le DPO, compléter le registre et les documents de conformité.
    ConformiteGerer,
    /// Socle 27 : ajuster une règle de conservation dans ses bornes (motif exigé).
    ConservationReglesProposer,
    /// ★ Socle 27 : valider une règle de conservation. Rôle paramétrable (D-017) : le DPO en général ;
    /// aucun profil historique ne la porte, une règle non validée n'autorise aucune destruction (R-RG1).
    ConservationReglesValider,
    /// ★ Socle 27, lot G2 : valider une version du registre des traitements et publier les mentions d'information.
    ConformiteValider,
    /// Socle 27, lot G2 : déclarer les sous-traitants propres à l'établissement et objecter à une annonce de la plateforme.
    SoustraitantsGerer,
    /// Socle 27, lot G3a : enregistrer une demande de droit au guichet (courrier, courriel, guichet) et
    /// vérifier l'identité et la qualité du demandeur. Établissement seulement.
    DroitsRecevoir,
    /// Socle 27, lot G3a : traiter une demande de droit (qualifier, assembler, relire, rédiger la réponse,
    /// clore). Établissement seulement.
    DroitsTraiter,
    /// ★ Socle 27, lot G3a : remettre l'extrait au demandeur (archive chiffrée ou remise en main propre).
    /// Passkey récente exigée par le code de remise (R-RG42) ; la remise elle-même arrive avec le lot G3b.
    DroitsRemettre,
    /// Socle 27, lot G3a : arbitrages réservés à la direction — refus motivé, demande manifestement excessive
    /// (R-RG17), prolongation de deux mois, communication du nom d'un agent. Établissement seulement.
    DroitsArbitrer,
    // ── Socle 12, lot AS1 : appel et assiduité (`12_ASSIDUITE.md` §15.1) ──
    /// Faire l'appel des séances et activités dont on est l'intervenant (socle 12).
    AppelFaire,
    /// Faire ou reprendre l'appel de n'importe quelle séance de l'établissement.
    AppelFaireTout,
    /// Corriger un appel validé, à tout moment, avec motif obligatoire.
    AppelCorriger,
    /// Déléguer l'appel d'une de ses séances à un collègue.
    AppelDeleguer,
    /// Lire le motif et la nature déclarés d'une absence.
    AbsencesLireMotif,
    /// Qualifier une absence (motif légitime, excuse valable, non valable).
    AbsencesQualifier,
    /// Proposer une qualification d'absence, sans la décider.
    AbsencesProposerQualification,
    /// Localiser un élève (infirmerie, vie scolaire, CDI, entretien).
    LocalisationPoser,
    /// Définir la règle d'assiduité de l'établissement (versions datées).
    AssiduiteRegleGerer,
    // ── Socle 12, lot AS3 : justificatifs et autorisations (`12_ASSIDUITE.md` §10.3, §15.1, AS-125) ──
    /// Lire les justificatifs d'absence **non médicaux** (mot des parents, convocation, attestation).
    AbsencesJustificatifLire,
    /// Lire les justificatifs d'absence **médicaux** (certificat) ; chaque lecture est journalisée (E7).
    AbsencesJustificatifMedicalLire,
    /// Décider d'une demande d'autorisation d'absence (accorder, refuser, transmettre au DASEN).
    AbsencesAutorisationDecider,
    // ── Socle 12, lot AS5 : familles et notifications (`12_ASSIDUITE.md` §14.4, §15.1) ──
    /// Suivre l'information des familles : tracer un contact, voir les relances « sans réponse », préparer un
    /// courrier postal, régler le rythme de relance en lecture (D-017 : CPE, AED, secrétariat, direction).
    AssiduiteSuivre,
    // ── Socle 12, lot AS6 : vérification et file du matin (`12_ASSIDUITE.md` §17.6) ──
    /// « Prévenir maintenant » : alerter la famille en urgence (U3, sans délai de confirmation) pour une absence
    /// inquiétante, par la brique de notifications ; décision tracée (D-017 : direction, CPE).
    AssiduiteAlerter,
    // ── Socle 20, lot C2 : cahier de textes, saisie et publication (`20_CAHIER_DE_TEXTES.md` §19.1) ──
    /// Retirer de la diffusion une entrée ou un travail du cahier (motif obligatoire, versions gardées).
    CahierRetirer,
    /// Ajouter sa contribution signée à une séance d'un cours dont on est intervenant (assistant, co-intervenant).
    CahierContribuer,
    /// Régler les repères de charge de travail de l'établissement (lot C4, S20-10).
    CahierParametrer,
    // ── Socle 6, lots P5 et P6 (`06_PERSONNELS.md` §14.2) ──
    /// Créer, modifier les membres et clore les équipes constituées (conseil pédagogique, cellule de
    /// veille, équipe éducative…). N'OUVRE AUCUN DROIT sur les élèves (Q-P9).
    EquipesGerer,
    /// Habiliter des agents à signer certains documents au nom de l'établissement et organiser
    /// l'intérim de direction (PRS-81, 82). Ne se donne que par qui la détient (anti-escalade).
    SignatureHabiliter,
    /// Socle 5, lot S6 (STR-50 à 55) : créer, modifier, fermer les groupes et leurs classes d'origine.
    /// `referentiel.gerer` reste un raccourci qui la couvre (§13.2).
    StructureGerer,
    /// Socle 5, lot S6 (STR-60 à 66) : affecter les élèves aux classes et aux groupes, changer de classe,
    /// composer, retirer, corriger. `referentiel.gerer` la couvre.
    AffectationsGerer,
    /// Socle 5, lot S6 (STR-63, 66) : composer SON groupe d'activité ou de liste (portée « mes groupes »).
    GroupesComposer,
    /// Socle 5, lot S6 (STR-53) : créer un groupe d'activité dont on devient responsable.
    GroupesCreerActivite,
    /// Socle 16, lot M3 (MSG-160, 161, 169, 170, 175) : créer et paramétrer les boîtes de service (membres par
    /// mission, horaires, réponse automatique, astreinte). Direction ; jamais le contenu des boîtes.
    MessagerieBoitesGerer,
    /// Socle 16, lot M3 (MSG-162 à 168, 172) : traiter les fils d'une boîte dont on est membre. Le droit ouvre
    /// la fonction ; **la boîte** reste décidée par la mission (membre à la date), jamais par le droit seul.
    MessagerieBoiteTraiter,
    /// Socle 16, lot M3 (MSG-171) : saisir un message téléphonique (accueil, vie scolaire).
    MessagerieMessageTelSaisir,
    /// Socle 16, lot M4 (MSG-130 et suivantes, R-M13) : envoyer un lot individuel (30 fils au plus, aperçu et
    /// confirmation). Professeurs, CPE, direction.
    MessagerieLotEnvoyer,
    // ── Socle 20, lot C8 : visa, inspection, exports et archive du cahier (`20_CAHIER_DE_TEXTES.md` §19.1) ──
    /// Accorder, prolonger, révoquer un accès d'inspection au cahier de textes (durée bornée, journalisé).
    CahierAccesInspection,
    /// Produire un export figé du cahier (litige, inspection), clôturer l'année du cahier et décider du sort des archives.
    CahierExporter,
    /// Clôturer l'année du cahier, la rouvrir (motif de 20 caractères au moins, double validation par une seconde personne
    /// de la direction) et décider du sort des archives. Direction seule : l'administrateur technique ne l'a pas par défaut.
    CahierCloturer,
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
    pub const TOUTES: [Action; 288] = [
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
        // Transverse « individu » (2026-11-27) : hors des 53 historiques et hors de la fin du tableau (positions testées).
        Action::PersonnesIndividuRelier,
        Action::PersonnesIndividuLever,
        Action::ExportsOfficiels,
        Action::EdtAnnulerMasse,
        Action::DelegationsGerer,
        // S2-7
        Action::PeriodesVerrouiller,
        Action::BulletinsPublier,
        Action::NotesCorriger,
        Action::LivretsExporter,
        Action::AgendaEvenementsCreer,
        Action::AgendaEvenementsPublier,
        Action::AgendaLireStructure,
        Action::AgendaJournalLire,
        Action::AgendaParametrer,
        Action::AgendaSemaineGerer,
        Action::AgendaLieuxLire,
        Action::AgendaLieuxReserver,
        Action::AgendaLieuxValider,
        Action::AgendaLieuxGerer,
        Action::AgendaLieuxParametrer,
        Action::CampagnesLancer,
        Action::RencontresGerer,
        Action::FormulairesGerer,
        Action::ConsentementsLire,
        Action::NotificationsGerer,
        Action::NotificationsAvisVoir,
        Action::NotificationsParametrer,
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
        // Socle 29 (portails), lot P2.
        Action::AfaireRapprocher,
        Action::AfairePrendre,
        Action::AfaireReattribuer,
        Action::AfaireSuperviser,
        Action::RechercheGlobale,
        Action::RechercheDebloquer,
        // Socle 39, lot ST0 (en fin de catalogue : le catalogue historique est repéré par position).
        Action::StagesLire,
        Action::StagesPreparer,
        Action::StagesOrganismesGerer,
        Action::StagesParametrer,
        Action::StagesPeriodesGerer,
        Action::DemandesDeposer,
        Action::DemandesDeposerPourCompte,
        Action::DemandesTraiter,
        Action::DemandesFileRepartir,
        Action::DemandesFileSuperviser,
        Action::DemandesLireToutes,
        Action::DemandesSuperviserConfidentiel,
        Action::DemandesHseDecider,
        Action::DemandesHseAttester,
        Action::DemandesHseEnveloppeGerer,
        Action::DemandesReproValider,
        Action::DemandesReproImprimer,
        Action::DemandesParametrer,
        Action::DemandesJournalLire,
        Action::DemandesIndicateursLire,
        Action::DemandesReprise,
        Action::RepriseDecider,
        // Socle 27 (RGPD, conservation), lot G1.
        Action::ConformiteLire,
        Action::ConformiteGerer,
        Action::ConservationReglesProposer,
        Action::ConservationReglesValider,
        Action::ConformiteValider,
        Action::SoustraitantsGerer,
        // Socle 27, lot G3a : guichet des demandes de droits.
        Action::DroitsRecevoir,
        Action::DroitsTraiter,
        Action::DroitsRemettre,
        Action::DroitsArbitrer,
        Action::AppelFaire,
        Action::AppelFaireTout,
        Action::AppelCorriger,
        Action::AppelDeleguer,
        Action::AbsencesLireMotif,
        Action::AbsencesQualifier,
        Action::AbsencesProposerQualification,
        Action::LocalisationPoser,
        Action::AssiduiteRegleGerer,
        Action::AbsencesJustificatifLire,
        Action::AbsencesJustificatifMedicalLire,
        Action::AbsencesAutorisationDecider,
        Action::AssiduiteSuivre,
        Action::AssiduiteAlerter,
        Action::CahierRetirer,
        Action::CahierContribuer,
        Action::CahierParametrer,
        // Socle 15, lots A0–A1 (en fin de catalogue : le catalogue historique est repéré par position).
        Action::PersonnelsAbsencesDeclarer,
        Action::PersonnelsAbsencesSignaler,
        Action::PersonnelsAbsencesLireOrganisation,
        Action::PersonnelsAbsencesLireNature,
        Action::PersonnelsAbsencesValider,
        Action::PersonnelsAbsencesInstruire,
        Action::PersonnelsAbsencesSaisirValidee,
        Action::PersonnelsAbsencesParametrer,
        Action::PersonnelsAbsencesConserver,
        Action::PersonnelsAbsencesSuppleer,
        Action::PersonnelsRcdGerer,
        Action::PersonnelsRcdRemontee,
        Action::EdtModifier,
        Action::EdtModifierLieu,
        Action::EdtLotAppliquer,
        Action::EdtCorrigerPasse,
        Action::EdtMotifInterneLire,
        Action::EdtModificationsLire,
        Action::EdtForcerConflit,
        Action::EdtDemainLire,
        Action::EdtMaintenantLire,
        Action::EdtPublicationLire,
        Action::EdtPublier,
        Action::EdtRevenirVersion,
        Action::PersonnelsHeuresLire,
        Action::PersonnelsHeuresValider,
        Action::PersonnelsHeuresSigner,
        Action::PersonnelsHeuresExporter,
        Action::PersonnelsPaieAbsences,
        Action::PersonnelsGreveGerer,
        // Socle 6, lots P5 et P6 (en fin de catalogue).
        Action::EquipesGerer,
        Action::SignatureHabiliter,
        Action::StructureGerer,
        Action::AffectationsGerer,
        Action::GroupesComposer,
        Action::GroupesCreerActivite,
        // Socle 7, lot D2 (en fin de catalogue).
        Action::DocumentsEmettre,
        Action::DocumentsAnnuler,
        Action::DocumentsRegistreLire,
        Action::DocumentsModelesParametrer,
        Action::DocumentsRestreintsLire,
        Action::DocumentsFinancierLire,
        Action::DocumentsPersonnelsLire,
        Action::DocumentsClasser,
        Action::DocumentsPiecesDemander,
        Action::DocumentsPiecesControler,
        Action::DocumentsSignaturesGerer,
        Action::DocumentsSignaturesConstater,
        // Socle 16, lots M3 et M4 (en fin de catalogue).
        Action::MessagerieBoitesGerer,
        Action::MessagerieBoiteTraiter,
        Action::MessagerieMessageTelSaisir,
        Action::MessagerieLotEnvoyer,
        Action::AutorisationsLire,
        Action::AutorisationsGerer,
        Action::AutorisationsParametrer,
        Action::FormulairesCreer,
        Action::FormulairesValider,
        // Socle 42, lot V0d (en fin de catalogue : le catalogue historique est repéré par position).
        Action::VoyagesReprise,
        // Socle 32, lots ET0–ET1 (en fin de catalogue : le catalogue historique est repéré par position).
        Action::EspacesUtiliser,
        Action::EspacesCreer,
        Action::EspacesPartager,
        Action::EspacesJournalLire,
        // Socle 24, lot F1 (en fin de catalogue : le catalogue historique est repéré par position).
        // Socle 41, lot I2 (après le catalogue historique, avant les droits fins de facturation).
        Action::InternatLire,
        Action::InternatAppel,
        Action::InternatAppelValider,
        Action::InternatSortiesDecider,
        Action::InternatLiaisonEcrire,
        Action::InternatLiaisonLire,
        Action::InternatLiaisonJour,
        Action::InternatEvenementDeclarer,
        Action::FacturationGuichet,
        Action::FacturationEmettre,
        Action::FacturationAvoirs,
        Action::FacturationEcheanciers,
        Action::FacturationReductions,
        Action::FacturationTarifs,
        Action::FacturationVoter,
        Action::FacturationSimuler,
        Action::FacturationParametrer,
        Action::FacturationEngagements,
        Action::FacturationCloturer,
        Action::FacturationExporter,
        Action::FacturationPilotage,
        Action::FacturationJournal,
        Action::FacturationReprise,
        Action::FacturationPublier,
        // Socle 42, lot V2a (en fin de catalogue : le catalogue historique est repéré par position).
        Action::VoyagesArgent,
        Action::VoyagesRemise,
        // Socle 20, lot C8 (en fin de catalogue).
        Action::CahierAccesInspection,
        Action::CahierExporter,
        Action::CahierCloturer,
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
            Self::VoyagesReprise => "voyages.reprise",
            Self::VoyagesArgent => "voyages.argent",
            Self::VoyagesRemise => "voyages.remise",
            Self::EspacesUtiliser => "espaces.utiliser",
            Self::EspacesCreer => "espaces.creer",
            Self::EspacesPartager => "espaces.partager",
            Self::EspacesJournalLire => "espaces.journal.lire",
            Self::FacturationGuichet => "facturation.guichet",
            Self::FacturationEmettre => "facturation.emettre",
            Self::FacturationAvoirs => "facturation.avoirs",
            Self::FacturationEcheanciers => "facturation.echeanciers",
            Self::FacturationReductions => "facturation.reductions",
            Self::FacturationTarifs => "facturation.tarifs",
            Self::FacturationVoter => "facturation.voter",
            Self::FacturationSimuler => "facturation.simuler",
            Self::FacturationParametrer => "facturation.parametrer",
            Self::FacturationEngagements => "facturation.engagements",
            Self::FacturationCloturer => "facturation.cloturer",
            Self::FacturationExporter => "facturation.exporter",
            Self::FacturationPilotage => "facturation.pilotage",
            Self::FacturationJournal => "facturation.journal",
            Self::FacturationReprise => "facturation.reprise",
            Self::FacturationPublier => "facturation.publier",
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
            Self::InternatLire => "internat.lire",
            Self::InternatAppel => "internat.appel",
            Self::InternatAppelValider => "internat.appel.valider",
            Self::InternatSortiesDecider => "internat.sorties.decider",
            Self::InternatLiaisonEcrire => "internat.liaison.ecrire",
            Self::InternatLiaisonLire => "internat.liaison.lire",
            Self::InternatLiaisonJour => "internat.liaison.jour",
            Self::InternatEvenementDeclarer => "internat.evenement.declarer",
            Self::SortiesGerer => "sorties.gerer",
            Self::SortiesControler => "sorties.controler",
            Self::ElevesGerer => "eleves.gerer",
            Self::MatieresGerer => "matieres.gerer",
            Self::ConseilsGerer => "conseils.gerer",
            Self::DocsSignerPublier => "docs_a_signer.publier",
            Self::ReferentielLire => "referentiel.lire",
            Self::ReferentielGerer => "referentiel.gerer",
            Self::PersonnesGerer => "personnes.gerer",
            Self::PersonnesIndividuRelier => "personnes.individu.relier",
            Self::PersonnesIndividuLever => "personnes.individu.lever",
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
            Self::AgendaEvenementsCreer => "agenda.evenements.creer",
            Self::AgendaEvenementsPublier => "agenda.evenements.publier",
            Self::AgendaLireStructure => "agenda.lire_structure",
            Self::AgendaJournalLire => "agenda.journal.lire",
            Self::AgendaParametrer => "agenda.parametrer",
            Self::AgendaSemaineGerer => "agenda.semaine.gerer",
            Self::AgendaLieuxLire => "agenda.lieux.lire",
            Self::AgendaLieuxReserver => "agenda.lieux.reserver",
            Self::AgendaLieuxValider => "agenda.lieux.valider",
            Self::AgendaLieuxGerer => "agenda.lieux.gerer",
            Self::AgendaLieuxParametrer => "agenda.lieux.parametrer",
            Self::CampagnesLancer => "campagnes.lancer",
            Self::RencontresGerer => "rencontres.gerer",
            Self::FormulairesGerer => "formulaires.gerer",
            Self::ConsentementsLire => "consentements.lire",
            Self::NotificationsGerer => "notifications.gerer",
            Self::NotificationsAvisVoir => "notifications.avis_voir",
            Self::NotificationsParametrer => "notifications.parametrer",
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
            Self::PersonnelsAbsencesDeclarer => "personnels.absences.declarer",
            Self::PersonnelsAbsencesSignaler => "personnels.absences.signaler",
            Self::PersonnelsAbsencesLireOrganisation => "personnels.absences.lire_organisation",
            Self::PersonnelsAbsencesLireNature => "personnels.absences.lire_nature",
            Self::PersonnelsAbsencesValider => "personnels.absences.valider",
            Self::PersonnelsAbsencesInstruire => "personnels.absences.instruire",
            Self::PersonnelsAbsencesSaisirValidee => "personnels.absences.saisir_validee",
            Self::PersonnelsAbsencesParametrer => "personnels.absences.parametrer",
            Self::AfaireRapprocher => "afaire.rapprocher",
            Self::AfairePrendre => "afaire.file.prendre",
            Self::AfaireReattribuer => "afaire.file.reattribuer",
            Self::AfaireSuperviser => "afaire.superviser",
            Self::RechercheGlobale => "recherche.globale",
            Self::RechercheDebloquer => "recherche.debloquer",
            Self::PersonnelsAbsencesConserver => "personnels.absences.conserver",
            Self::PersonnelsAbsencesSuppleer => "personnels.absences.suppleer",
            Self::PersonnelsRcdGerer => "personnels.rcd.gerer",
            Self::PersonnelsRcdRemontee => "personnels.rcd.remontee",
            Self::EdtModifier => "edt.modifier",
            Self::EdtModifierLieu => "edt.modifier_lieu",
            Self::EdtLotAppliquer => "edt.lot_appliquer",
            Self::EdtCorrigerPasse => "edt.corriger_passe",
            Self::EdtMotifInterneLire => "edt.motif_interne_lire",
            Self::EdtModificationsLire => "edt.modifications_lire",
            Self::EdtForcerConflit => "edt.forcer_conflit",
            Self::EdtDemainLire => "edt.demain_lire",
            Self::EdtMaintenantLire => "edt.maintenant_lire",
            Self::EdtPublicationLire => "edt.publication_lire",
            Self::EdtPublier => "edt.publier",
            Self::EdtRevenirVersion => "edt.revenir_version",
            Self::PersonnelsHeuresLire => "personnels.heures.lire",
            Self::PersonnelsHeuresValider => "personnels.heures.valider",
            Self::PersonnelsHeuresSigner => "personnels.heures.signer",
            Self::PersonnelsHeuresExporter => "personnels.heures.exporter",
            Self::PersonnelsPaieAbsences => "personnels.paie.absences",
            Self::PersonnelsGreveGerer => "personnels.greve.gerer",
            Self::StagesLire => "stages.lire",
            Self::StagesPreparer => "stages.preparer",
            Self::StagesOrganismesGerer => "stages.organismes.gerer",
            Self::StagesParametrer => "stages.parametrer",
            Self::StagesPeriodesGerer => "stages.periodes.gerer",
            Self::DemandesDeposer => "demandes.deposer",
            Self::DemandesDeposerPourCompte => "demandes.deposer_pour_compte",
            Self::DemandesTraiter => "demandes.traiter",
            Self::DemandesFileRepartir => "demandes.file.repartir",
            Self::DemandesFileSuperviser => "demandes.file.superviser",
            Self::DemandesLireToutes => "demandes.lire_toutes",
            Self::DemandesSuperviserConfidentiel => "demandes.superviser_confidentiel",
            Self::DemandesHseDecider => "demandes.hse.decider",
            Self::DemandesHseAttester => "demandes.hse.attester",
            Self::DemandesHseEnveloppeGerer => "demandes.hse.enveloppe.gerer",
            Self::DemandesReproValider => "demandes.repro.valider",
            Self::DemandesReproImprimer => "demandes.repro.imprimer",
            Self::DemandesParametrer => "demandes.parametrer",
            Self::DemandesJournalLire => "demandes.journal.lire",
            Self::DemandesIndicateursLire => "demandes.indicateurs.lire",
            Self::DemandesReprise => "demandes.reprise",
            Self::RepriseDecider => "reprise.decider",
            Self::DocumentsEmettre => "documents.emettre",
            Self::DocumentsAnnuler => "documents.annuler",
            Self::DocumentsRegistreLire => "documents.registre.lire",
            Self::DocumentsModelesParametrer => "documents.modeles.parametrer",
            Self::DocumentsRestreintsLire => "documents.restreints.lire",
            Self::DocumentsFinancierLire => "documents.financier.lire",
            Self::DocumentsPersonnelsLire => "documents.personnels.lire",
            Self::DocumentsClasser => "documents.classer",
            Self::DocumentsPiecesDemander => "documents.pieces.demander",
            Self::DocumentsPiecesControler => "documents.pieces.controler",
            Self::DocumentsSignaturesGerer => "documents.signatures.gerer",
            Self::DocumentsSignaturesConstater => "documents.signatures.constater",
            Self::AutorisationsLire => "autorisations.lire",
            Self::AutorisationsGerer => "autorisations.gerer",
            Self::AutorisationsParametrer => "autorisations.parametrer",
            Self::FormulairesCreer => "formulaires.creer",
            Self::FormulairesValider => "formulaires.valider",
            Self::ConformiteLire => "conformite.lire",
            Self::ConformiteGerer => "conformite.gerer",
            Self::ConservationReglesProposer => "conservation.regles.proposer",
            Self::ConservationReglesValider => "conservation.regles.valider",
            Self::ConformiteValider => "conformite.valider",
            Self::SoustraitantsGerer => "soustraitants.gerer",
            Self::DroitsRecevoir => "droits.recevoir",
            Self::DroitsTraiter => "droits.traiter",
            Self::DroitsRemettre => "droits.remettre",
            Self::DroitsArbitrer => "droits.arbitrer",
            Self::AppelFaire => "appel.faire",
            Self::AppelFaireTout => "appel.faire_tout",
            Self::AppelCorriger => "appel.corriger",
            Self::AppelDeleguer => "appel.deleguer",
            Self::AbsencesLireMotif => "absences.lire_motif",
            Self::AbsencesQualifier => "absences.qualifier",
            Self::AbsencesProposerQualification => "absences.proposer_qualification",
            Self::LocalisationPoser => "localisation.poser",
            Self::AssiduiteRegleGerer => "assiduite.regle_gerer",
            Self::AbsencesJustificatifLire => "absences.justificatif_lire",
            Self::AbsencesJustificatifMedicalLire => "absences.justificatif_medical_lire",
            Self::AbsencesAutorisationDecider => "absences.autorisation_decider",
            Self::AssiduiteSuivre => "assiduite.suivre",
            Self::AssiduiteAlerter => "assiduite.alerter",
            Self::CahierRetirer => "cahier.retirer",
            Self::CahierContribuer => "cahier.contribuer",
            Self::CahierParametrer => "cahier.parametrer",
            Self::CahierAccesInspection => "cahier.acces_inspection",
            Self::CahierExporter => "cahier.exporter",
            Self::CahierCloturer => "cahier.cloturer",
            Self::EquipesGerer => "equipes.gerer",
            Self::SignatureHabiliter => "signature.habiliter",
            Self::StructureGerer => "structure.gerer",
            Self::AffectationsGerer => "affectations.gerer",
            Self::GroupesComposer => "groupes.composer",
            Self::GroupesCreerActivite => "groupes.creer_activite",
            Self::MessagerieBoitesGerer => "messagerie.boites.gerer",
            Self::MessagerieBoiteTraiter => "messagerie.boite.traiter",
            Self::MessagerieMessageTelSaisir => "messagerie.message_tel.saisir",
            Self::MessagerieLotEnvoyer => "messagerie.lot.envoyer",
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
            Self::VoyagesReprise => "Reprendre les séjours de Next.js : zone de reprise, tableau et bascule (administrateur, direction)",
            Self::VoyagesArgent => "Suivre l'argent des sorties en mode suivi : parts, versements, plans de paiement, avoirs proposés, acomptes fournisseurs, export comptable",
            Self::VoyagesRemise => "Recevoir les bordereaux de chèques collectés par un organisateur et pointer leurs lignes",
            Self::EspacesUtiliser => "Utiliser son espace personnel et les espaces dont on est membre (fichiers, dossiers, versions, corbeille)",
            Self::EspacesCreer => "Créer un espace libre entre personnels de l'établissement",
            Self::EspacesPartager => "Partager un élément ou un dossier avec des personnes nommées de l'établissement",
            Self::EspacesJournalLire => "Consulter le journal des espaces de l'établissement (sans le contenu ni les espaces personnels)",
            Self::FacturationGuichet => "Facturation : vue guichet du payeur présent ou authentifié (solde, échéances, documents)",
            Self::FacturationEmettre => "Facturation : émettre les pièces (★)",
            Self::FacturationAvoirs => "Facturation : avoirs et corrections (★)",
            Self::FacturationEcheanciers => "Facturation : échéanciers dérogatoires",
            Self::FacturationReductions => "Facturation : accorder ou retirer des réductions",
            Self::FacturationTarifs => "Facturation : articles, grilles, réductions (brouillon)",
            Self::FacturationVoter => "Facturation : enregistrer le vote d'une grille (★)",
            Self::FacturationSimuler => "Facturation : simulateur de tarifs",
            Self::FacturationParametrer => "Facturation : paramètres, identité émettrice, séries (★)",
            Self::FacturationEngagements => "Facturation : comptes débiteurs, payeurs, engagements",
            Self::FacturationCloturer => "Facturation : clôturer une période (★)",
            Self::FacturationExporter => "Facturation : exports et écritures comptables (★)",
            Self::FacturationPilotage => "Facturation : tableaux de pilotage agrégés",
            Self::FacturationJournal => "Facturation : journal des actions financières",
            Self::FacturationReprise => "Facturation : reprise et double tenue",
            Self::FacturationPublier => "Facturation : publier la grille publique",
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
            Self::InternatLire => "Lire l'attendu, les appels et les cas de nuit de l'internat",
            Self::InternatAppel => "Ouvrir un appel de nuit et pointer les internes de son périmètre",
            Self::InternatAppelValider => "Clore, valider et corriger un appel de nuit ; tenir un cas",
            Self::InternatSortiesDecider => "Décider des absences annoncées et des sorties des internes",
            Self::InternatLiaisonEcrire => "Écrire un fait de liaison codé sur un interne",
            Self::InternatLiaisonLire => "Lire la liaison et les événements de nuit de l'internat",
            Self::InternatLiaisonJour => "Lire, le matin, les faits de liaison marqués pour le jour",
            Self::InternatEvenementDeclarer => "Déclarer un événement de nuit ou ouvrir une préoccupation",
            Self::SortiesGerer => "Gérer les sorties (règles, autorisations, demandes)",
            Self::SortiesControler => "Contrôler les sorties au portail",
            Self::ElevesGerer => "Gérer les élèves, foyers, liens et comptes",
            Self::MatieresGerer => "Gérer les matières et compétences",
            Self::ConseilsGerer => "Planifier et tenir les conseils de classe",
            Self::DocsSignerPublier => "Publier et annuler les documents à signer",
            Self::ReferentielLire => "Consulter le référentiel scolaire",
            Self::ReferentielGerer => "Gérer le référentiel scolaire (années, classes, périodes…)",
            Self::PersonnesGerer => "Gérer les responsables, contacts et liens d'un élève",
            Self::PersonnesIndividuRelier => "Relier deux fiches d'un même individu (deux comptes, agent et parent), propositions",
            Self::PersonnesIndividuLever => "Lever un lien d'individu (jamais le sien)",
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
            Self::AgendaEvenementsCreer => "Créer des événements et des réunions dans l'agenda",
            Self::AgendaEvenementsPublier => "Publier un événement à une audience (classes, niveau, établissement)",
            Self::AgendaLireStructure => "Consulter l'agenda d'une classe ou d'un groupe (sans noms d'élèves)",
            Self::AgendaJournalLire => "Consulter les journaux de l'agenda",
            Self::AgendaParametrer => "Paramétrer l'agenda (types d'événements de l'établissement)",
            Self::AgendaSemaineGerer => "Tenir la feuille de semaine de l'établissement",
            Self::AgendaLieuxLire => "Rechercher une salle libre et consulter le planning des salles",
            Self::AgendaLieuxReserver => "Réserver une salle",
            Self::AgendaLieuxValider => "Valider ou refuser les demandes de salles",
            Self::AgendaLieuxGerer => "Gérer toutes les réservations de salles",
            Self::AgendaLieuxParametrer => "Paramétrer les règles de réservation des salles",
            Self::CampagnesLancer => "Lancer des campagnes d'information (e-mail, SMS, push)",
            Self::RencontresGerer => "Organiser les rencontres parents-professeurs",
            Self::FormulairesGerer => "Créer et publier formulaires, autorisations et consentements",
            Self::ConsentementsLire => "Consulter les consentements (objet par objet)",
            Self::NotificationsGerer => "Régler les plafonds d'envoi et suivre les envois",
            Self::NotificationsAvisVoir => "Voir un avis, ses destinataires et l'état de ses remises",
            Self::NotificationsParametrer => "Régler les paramètres de notification de l'établissement",
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
            Self::PersonnelsAbsencesDeclarer => "Déclarer l'absence d'un collègue (la sienne ne demande aucun droit)",
            Self::PersonnelsAbsencesSignaler => "Signaler l'absence d'un agent, sans nature (accueil, vie scolaire)",
            Self::PersonnelsAbsencesLireOrganisation => "Voir les absences des personnels pour organiser la journée (sans la nature)",
            Self::PersonnelsAbsencesLireNature => "Lire la nature, la décision et la zone de qualification d'une absence de personnel",
            Self::PersonnelsAbsencesValider => "Valider, refuser, requalifier et rouvrir une absence de personnel",
            Self::PersonnelsAbsencesInstruire => "Instruire une absence de personnel (réception de l'arrêt, traitement administratif)",
            Self::PersonnelsAbsencesSaisirValidee => "Saisir une absence de personnel déjà validée (régularisation)",
            Self::PersonnelsAbsencesParametrer => "Paramétrer les natures, circuits et délais des absences de personnel",
            Self::AfaireRapprocher => "Rapprocher l'index des tâches avec les sources et lire le rapport",
            Self::AfairePrendre => "Prendre en charge une tâche d'une file dont on est membre",
            Self::AfaireReattribuer => "Réattribuer une tâche prise à un collègue de la file (service)",
            Self::AfaireSuperviser => "Superviser les files par nombres (ouvertes, en retard, délai médian, tâches orphelines)",
            Self::RechercheGlobale => "Rechercher dans l'ENT (élèves de sa portée, personnes, personnels, classes, salles), plafonné par jour",
            Self::RechercheDebloquer => "Lever un blocage de recherche (anti-énumération)",
            Self::PersonnelsAbsencesConserver => "Geler, purger et protéger la conservation des absences de personnel",
            Self::PersonnelsAbsencesSuppleer => "Demander une suppléance et nommer un suppléant pour une absence longue",
            Self::PersonnelsRcdGerer => "Tenir le plan de remplacement de courte durée et le vivier des volontaires",
            Self::PersonnelsRcdRemontee => "Préparer, contrôler et transmettre la remontée hebdomadaire du remplacement de courte durée",
            Self::EdtModifier => "Modifier le réel de l'emploi du temps (annuler, déplacer, changer l'horaire, ajouter, fusionner, lever)",
            Self::EdtModifierLieu => "Changer le lieu d'une séance ou d'une série de séances",
            Self::EdtLotAppliquer => "Appliquer un lot de modifications de l'emploi du temps (grève, fermeture, salle fermée) après aperçu",
            Self::EdtCorrigerPasse => "Corriger une séance passée de l'emploi du temps (motif obligatoire, journalisé)",
            Self::EdtMotifInterneLire => "Lire le motif interne d'une modification de l'emploi du temps",
            Self::EdtModificationsLire => "Lire le journal des modifications de l'emploi du temps",
            Self::EdtForcerConflit => "Forcer un conflit dur de l'emploi du temps avec un motif journalisé",
            Self::EdtDemainLire => "Lire le tableau « demain » de l'emploi du temps et les élèves sans cours",
            Self::EdtMaintenantLire => "Lire « maintenant » dans l'emploi du temps (séance d'un élève, salle, présence d'un professeur)",
            Self::EdtPublicationLire => "Contrôler une version de l'emploi du temps avant publication, voir l'aperçu, l'historique et les différences",
            Self::EdtPublier => "Publier une version de l'emploi du temps (personnels, puis familles), régler la diffusion, publier une version individuelle",
            Self::EdtRevenirVersion => "Revenir à la version précédente de l'emploi du temps (avant sa diffusion, ou à une date future)",
            Self::PersonnelsHeuresLire => "Lire les heures constatées d'un agent et son état annuel",
            Self::PersonnelsHeuresValider => "Valider, corriger, saisir et régulariser les heures constatées, figer l'état mensuel",
            Self::PersonnelsHeuresSigner => "Signer l'état mensuel des heures",
            Self::PersonnelsHeuresExporter => "Exporter l'état mensuel des heures vers le rectorat ou la paie",
            Self::PersonnelsPaieAbsences => "Préparer les éléments variables de paie des absences et suivre la DSN (OGEC)",
            Self::PersonnelsGreveGerer => "Cocher les grévistes du matin, annuler en lot leurs séances, suivre le tableau de crise",
            Self::StagesLire => "Consulter les stages des élèves",
            Self::StagesPreparer => "Déclarer, clôturer ou annuler un stage",
            Self::StagesOrganismesGerer => "Tenir le carnet des organismes d'accueil de stage",
            Self::StagesParametrer => "Paramétrer les stages (dispositifs, assurance, quotas) et interdire un organisme",
            Self::StagesPeriodesGerer => "Planifier, publier et fermer les périodes de stage",
            Self::DemandesDeposer => "Déposer une demande à l'établissement au nom de soi (natures ouvertes au personnel)",
            Self::DemandesDeposerPourCompte => "Déposer une demande au nom d'une autre personne (accueil, secrétariat, direction) : le déposant est tracé",
            Self::DemandesTraiter => "Traiter les demandes des files dont on est membre (membres calculés ; aucun modèle ne le donne à tout un rôle)",
            Self::DemandesFileRepartir => "Réattribuer, déléguer et reporter l'échéance des demandes des files dont on est responsable",
            Self::DemandesFileSuperviser => "Superviser les files : compteurs, délais, retards, contenu C0–C1",
            Self::DemandesLireToutes => "Lire les demandes C0–C1 de l'établissement (chef d'établissement)",
            Self::DemandesSuperviserConfidentiel => "Superviser, de façon motivée et tracée, une file de demandes C2 et plus (geste exceptionnel)",
            Self::DemandesHseDecider => "Décider les heures supplémentaires effectives (HSE) demandées par les personnels",
            Self::DemandesHseAttester => "Attester le service fait des HSE (circuit facultatif, désactivé par défaut)",
            Self::DemandesHseEnveloppeGerer => "Gérer l'enveloppe d'HSE (en heures) par année et par site",
            Self::DemandesReproValider => "Valider les tirages en couleur et les gros volumes de reprographie",
            Self::DemandesReproImprimer => "Tenir la file d'impression des sites désignés (réceptionnaire de reprographie)",
            Self::DemandesParametrer => "Paramétrer les natures, files, règles de routage et réponses types des demandes",
            Self::DemandesJournalLire => "Lire les journaux de paramétrage et d'accès des demandes",
            Self::DemandesIndicateursLire => "Lire les indicateurs agrégés des demandes",
            Self::DemandesReprise => "Lancer la reprise des demandes de l'ancienne application",
            Self::RepriseDecider => "Décider les rapprochements incertains de la reprise depuis Next.js : rattacher, créer, ignorer, annuler (secrétariat, direction)",
            Self::DocumentsEmettre => "Émettre un document officiel (certificat, attestation, courrier) pour un élève ou une personne",
            Self::DocumentsAnnuler => "Annuler un document émis par erreur (motif obligatoire, journalisé)",
            Self::DocumentsRegistreLire => "Lire le registre des documents émis et le journal de leurs consultations",
            Self::DocumentsModelesParametrer => "Paramétrer l'identité des documents, les entités émettrices et les natures activées",
            Self::DocumentsRestreintsLire => "Lire les documents restreints du domaine justice (jugements, pièces judiciaires)",
            Self::DocumentsFinancierLire => "Lire les documents du domaine financier (avis d'imposition, quotient, bourses)",
            Self::DocumentsPersonnelsLire => "Lire les dossiers documentaires des agents (domaine rh)",
            Self::DocumentsClasser => "Retirer ou rendre une mise à disposition de document (coffre), avec motif",
            Self::DocumentsPiecesDemander => "Demander, relancer ou annuler une pièce justificative ; voir ce qui manque à un dossier",
            Self::DocumentsPiecesControler => "Contrôler les pièces déposées : accepter, refuser avec motif, déclarer sans objet",
            Self::DocumentsSignaturesGerer => "Lancer, suivre, relancer ou annuler un circuit de signature ; valider un papier signé",
            Self::DocumentsSignaturesConstater => "Constater une signature au guichet (agent identifié, identité vue, confirmation)",
            Self::AutorisationsLire => "Lire les verdicts d'accord (peut / ne peut pas / en attente) des élèves de sa portée",
            Self::AutorisationsGerer => "Lire les décisions, preuves et désaccords ; corriger ou annuler une saisie",
            Self::AutorisationsParametrer => "Activer les objets d'accord, rédiger et publier les versions de texte et leurs usages",
            Self::FormulairesCreer => "Créer, rédiger et publier un formulaire pour l'audience de sa portée",
            Self::FormulairesValider => "Valider un formulaire à large audience avant sa publication (jamais le sien)",
            Self::ConformiteLire => "Voir le tableau de bord de conformité, la politique de conservation et le registre",
            Self::ConformiteGerer => "Désigner le DPO, compléter le registre et les documents de conformité",
            Self::ConservationReglesProposer => "Ajuster une règle de conservation dans ses bornes, avec motif",
            Self::ConservationReglesValider => "Valider une règle de conservation (DPO ou rôle désigné par l'établissement)",
            Self::ConformiteValider => "Valider une version du registre des traitements et publier les mentions d'information",
            Self::SoustraitantsGerer => "Déclarer les sous-traitants de l'établissement et objecter à une annonce de la plateforme",
            Self::DroitsRecevoir => "Enregistrer une demande de droit et vérifier l'identité du demandeur",
            Self::DroitsTraiter => "Qualifier, instruire et clore une demande de droit",
            Self::DroitsRemettre => "Remettre l'extrait d'une demande de droit",
            Self::DroitsArbitrer => "Prolonger, refuser ou arbitrer une demande de droit (direction)",
            Self::AppelFaire => "Faire l'appel des séances et activités dont on est l'intervenant (socle 12)",
            Self::AppelFaireTout => "Faire ou reprendre l'appel de n'importe quelle séance de l'établissement",
            Self::AppelCorriger => "Corriger un appel validé, à tout moment, avec motif obligatoire",
            Self::AppelDeleguer => "Déléguer l'appel d'une de ses séances à un collègue",
            Self::AbsencesLireMotif => "Lire le motif et la nature déclarés d'une absence",
            Self::AbsencesQualifier => "Qualifier une absence (motif légitime, excuse valable, non valable)",
            Self::AbsencesProposerQualification => "Proposer une qualification d'absence, sans la décider",
            Self::LocalisationPoser => "Localiser un élève (infirmerie, vie scolaire, CDI, entretien)",
            Self::AssiduiteRegleGerer => "Définir la règle d'assiduité de l'établissement (versions datées)",
            Self::AbsencesJustificatifLire => "Lire les justificatifs d'absence non médicaux",
            Self::AbsencesJustificatifMedicalLire => "Lire les justificatifs d'absence médicaux (lecture journalisée)",
            Self::AbsencesAutorisationDecider => "Décider d'une demande d'autorisation d'absence",
            Self::AssiduiteSuivre => "Suivre l'information des familles : contacts, relances sans réponse, courriers postaux",
            Self::AssiduiteAlerter => "Prévenir maintenant une famille d'une absence inquiétante (urgence, décision tracée)",
            Self::CahierRetirer => "Retirer de la diffusion une entrée ou un travail du cahier de textes (motif obligatoire)",
            Self::CahierContribuer => "Ajouter sa contribution signée au cahier d'un cours dont on est intervenant",
            Self::CahierParametrer => "Régler les repères de charge de travail du cahier de textes",
            Self::CahierAccesInspection => "Accorder, prolonger ou révoquer un accès d'inspection au cahier de textes (durée bornée, journalisé)",
            Self::CahierExporter => "Produire un export figé du cahier de textes (litige, inspection) et lire les exports et archives",
            Self::CahierCloturer => "Clôturer l'année du cahier de textes, la rouvrir (motif et double validation) et décider du sort des archives",
            Self::EquipesGerer => "Constituer, modifier et clore les équipes (conseil pédagogique, cellule de veille, équipe éducative) — sans droit sur les élèves",
            Self::SignatureHabiliter => "Habiliter à signer au nom de l'établissement et organiser l'intérim de direction",
            Self::StructureGerer => "Gérer les groupes et leurs classes d'origine (créer, générer, modifier, fermer)",
            Self::AffectationsGerer => "Affecter les élèves aux classes et aux groupes, changer de classe, corriger une affectation",
            Self::GroupesComposer => "Composer son groupe d'activité ou de liste (entrées et sorties d'élèves)",
            Self::GroupesCreerActivite => "Créer un atelier, un club ou un groupe d'activité dont on devient responsable",
            Self::MessagerieBoitesGerer => "Créer et paramétrer les boîtes de service de la messagerie (membres, horaires, astreinte)",
            Self::MessagerieBoiteTraiter => "Traiter les messages des boîtes de service dont on est membre",
            Self::MessagerieMessageTelSaisir => "Saisir un message téléphonique pour une boîte ou une personne",
            Self::MessagerieLotEnvoyer => "Envoyer un lot de messages individuels (30 au plus, avec aperçu)",
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
            // F1 : lire tous les comptes inclut la vue guichet d'un compte.
            Self::FacturationGuichet => &[Self::FacturationLire, Self::FacturationGerer],
            // Socle 12 : faire l'appel de tout l'établissement inclut faire celui de ses séances.
            Self::AppelFaire => &[Self::AppelFaireTout],
            Self::AbsencesProposerQualification => &[Self::AbsencesQualifier],
            // Socle 12 (AS3) : lire une pièce médicale permet de lire les pièces non médicales.
            Self::AbsencesJustificatifLire => &[Self::AbsencesJustificatifMedicalLire],
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
            // Socle 5, S6 (§13.2) : `referentiel.gerer` reste un raccourci qui les couvre toutes ; composer son
            // groupe et créer une activité sont aussi couverts par les droits de gestion correspondants.
            Self::StructureGerer | Self::AffectationsGerer => &[Self::ReferentielGerer],
            Self::GroupesComposer => &[Self::AffectationsGerer, Self::ReferentielGerer],
            Self::GroupesCreerActivite => &[Self::StructureGerer, Self::ReferentielGerer],
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
            | Self::AgendaEvenementsCreer
            | Self::AgendaLireStructure
            | Self::AgendaJournalLire
            | Self::AgendaParametrer
            | Self::AgendaSemaineGerer
            | Self::AgendaLieuxLire
            | Self::AgendaLieuxReserver
            | Self::AgendaLieuxValider
            | Self::AgendaLieuxGerer
            | Self::AgendaLieuxParametrer
            | Self::RencontresGerer
            | Self::FormulairesGerer
            | Self::ConsentementsLire
            | Self::NotificationsGerer
            | Self::NotificationsAvisVoir
            | Self::NotificationsParametrer
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
            | Self::VoyagesReprise
            | Self::VoyagesArgent
            | Self::VoyagesRemise
            | Self::EspacesUtiliser
            | Self::EspacesCreer
            | Self::EspacesPartager
            | Self::EspacesJournalLire
            | Self::FacturationGuichet
            | Self::FacturationEmettre
            | Self::FacturationAvoirs
            | Self::FacturationEcheanciers
            | Self::FacturationReductions
            | Self::FacturationTarifs
            | Self::FacturationVoter
            | Self::FacturationSimuler
            | Self::FacturationParametrer
            | Self::FacturationEngagements
            | Self::FacturationCloturer
            | Self::FacturationExporter
            | Self::FacturationPilotage
            | Self::FacturationJournal
            | Self::FacturationReprise
            | Self::FacturationPublier
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
            | Self::DocsSignerPublier
            | Self::CahierAccesInspection
            | Self::CahierExporter
            | Self::CahierCloturer => &[Portee::Etablissement],
            // Socle 41, lot I2 (R-IT30) : une direction de formation (collège, lycée) lit et valide
            // seulement ses élèves : portée « désignés » (classes de la formation) en plus de l'établissement.
            Self::InternatLire
            | Self::InternatAppel
            | Self::InternatAppelValider
            | Self::InternatSortiesDecider
            | Self::InternatLiaisonEcrire
            | Self::InternatLiaisonLire
            | Self::InternatLiaisonJour
            | Self::InternatEvenementDeclarer => {
                &[Portee::Etablissement, Portee::Designes]
            }
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
            | Self::PersonnesIndividuRelier
            | Self::PersonnesIndividuLever
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
            | Self::PersonnelsAbsencesDeclarer
            | Self::PersonnelsAbsencesSignaler
            | Self::PersonnelsAbsencesLireOrganisation
            | Self::PersonnelsAbsencesLireNature
            | Self::PersonnelsAbsencesValider
            | Self::PersonnelsAbsencesInstruire
            | Self::PersonnelsAbsencesSaisirValidee
            | Self::PersonnelsAbsencesParametrer
            | Self::PersonnelsAbsencesConserver
            | Self::PersonnelsAbsencesSuppleer
            | Self::PersonnelsRcdGerer
            | Self::PersonnelsRcdRemontee
            | Self::EdtModifier
            | Self::EdtModifierLieu
            | Self::EdtLotAppliquer
            | Self::EdtCorrigerPasse
            | Self::EdtMotifInterneLire
            | Self::EdtModificationsLire
            | Self::EdtForcerConflit
            | Self::EdtDemainLire
            | Self::EdtMaintenantLire
            | Self::EdtPublicationLire
            | Self::EdtPublier
            | Self::EdtRevenirVersion
            | Self::PersonnelsHeuresLire
            | Self::PersonnelsHeuresValider
            | Self::PersonnelsHeuresSigner
            | Self::PersonnelsHeuresExporter
            | Self::PersonnelsPaieAbsences
            | Self::PersonnelsGreveGerer
            | Self::EquipesGerer
            | Self::SignatureHabiliter
            | Self::StructureGerer
            | Self::GroupesCreerActivite
            | Self::MessagerieBoitesGerer
            | Self::MessagerieBoiteTraiter
            | Self::MessagerieMessageTelSaisir
            | Self::MessagerieLotEnvoyer
            | Self::AccesRevue => &[Portee::Etablissement],
            // Socle 5, S6 : affecter peut être délégué à des classes ou groupes désignés (un adjoint « niveau 6e ») ;
            // composer son groupe : établissement, mes groupes, désignés.
            Self::AffectationsGerer => &[Portee::Etablissement, Portee::Designes],
            Self::GroupesComposer => &[Portee::Etablissement, Portee::MesGroupes, Portee::Designes],
            | Self::AfaireRapprocher
            | Self::AfairePrendre
            | Self::AfaireReattribuer
            | Self::AfaireSuperviser
            | Self::RechercheGlobale
            | Self::RechercheDebloquer
            | Self::StagesOrganismesGerer
            | Self::StagesParametrer
            | Self::StagesPeriodesGerer
            | Self::DemandesDeposer
            | Self::DemandesDeposerPourCompte
            | Self::DemandesTraiter
            | Self::DemandesFileRepartir
            | Self::DemandesFileSuperviser
            | Self::DemandesLireToutes
            | Self::DemandesSuperviserConfidentiel
            | Self::DemandesHseDecider
            | Self::DemandesHseAttester
            | Self::DemandesHseEnveloppeGerer
            | Self::DemandesReproValider
            | Self::DemandesReproImprimer
            | Self::DemandesParametrer
            | Self::DemandesJournalLire
            | Self::DemandesIndicateursLire
            | Self::DemandesReprise
            | Self::RepriseDecider
            | Self::DocumentsEmettre
            | Self::DocumentsAnnuler
            | Self::DocumentsRegistreLire
            | Self::DocumentsModelesParametrer
            | Self::DocumentsRestreintsLire
            | Self::DocumentsFinancierLire
            | Self::DocumentsPersonnelsLire
            | Self::DocumentsClasser
            | Self::DocumentsPiecesDemander
            | Self::DocumentsPiecesControler
            | Self::DocumentsSignaturesGerer
            | Self::DocumentsSignaturesConstater
            | Self::AutorisationsGerer
            | Self::AutorisationsParametrer
            | Self::FormulairesValider => &[Portee::Etablissement],
            | Self::ConformiteLire
            | Self::ConformiteGerer
            | Self::ConservationReglesProposer
            | Self::ConservationReglesValider
            | Self::ConformiteValider
            | Self::SoustraitantsGerer
            | Self::DroitsRecevoir
            | Self::DroitsTraiter
            | Self::DroitsRemettre
            | Self::DroitsArbitrer => &[Portee::Etablissement],
            // Socle 39 : stages ; « mes élèves suivis » = ses classes (le référent viendra avec la mission 6).
            Self::StagesLire | Self::StagesPreparer => {
                &[Portee::Etablissement, Portee::MesClasses, Portee::Designes]
            }
            | Self::AppelFaire
            | Self::AppelFaireTout
            | Self::AppelCorriger
            | Self::AppelDeleguer
            | Self::AbsencesLireMotif
            | Self::AbsencesQualifier
            | Self::AbsencesProposerQualification
            | Self::LocalisationPoser
            | Self::AssiduiteRegleGerer
            | Self::AbsencesJustificatifLire
            | Self::AbsencesJustificatifMedicalLire
            | Self::AbsencesAutorisationDecider
            | Self::AssiduiteSuivre
            | Self::AssiduiteAlerter
            => &[Portee::Etablissement],
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
    // Modèles des socles récents : une fonction par socle (voir plus bas). Un nouveau socle ajoute
    // sa fonction et UNE ligne ici, dans l'ordre des lots ; l'ordre de la liste est conservé.
    socle_15_absences_personnels(role, &mut d);
    socle_19_agenda(role, &mut d);
    socle_39_stages(role, &mut d);
    socle_12_assiduite(role, &mut d);
    socle_29_afaire(role, &mut d);
    socle_42_voyages(role, &mut d);
    socle_41_internat(role, &mut d);
    socle_n1_demandes(role, &mut d);
    socle_20_cahier(role, &mut d);
    socle_06_personnels(role, &mut d);
    socle_07_documents(role, &mut d);
    socle_18_formulaires(role, &mut d);
    socle_27_rgpd(role, &mut d);
    socle_10_edt(role, &mut d);
    socle_05_structure(role, &mut d);
    socle_8_notifications(role, &mut d);
    socle_16_messagerie(role, &mut d);
    socle_32_espaces(role, &mut d);
    socle_24_finances(role, &mut d);
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
        // F1 (R-F37) : facturation.lire non plus — le secrétariat passe en vue guichet
        // (`facturation.guichet`, voir `socle_24_finances`) : le payeur présent, jamais tous les comptes.
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
    // Transverse « individu » (279) : relier deux fiches du même humain (direction, secrétariat) ; lever un lien
    // retire des refus : direction seule.
    if matches!(role, Role::Direction | Role::Administratif) {
        d.push(etab(PersonnesIndividuRelier));
    }
    if role == Role::Direction {
        d.push(etab(PersonnesIndividuLever));
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

// ── Modèles par défaut, un bloc par socle (appelés par `modele_par_defaut`, dans l'ordre) ──

/// Socle 15 (absences des personnels), lots A0–A1. Bloc de `modele_par_defaut`, isolé pour que les fusions de branches ne s'y croisent pas.
/// Socle 10 (emploi du temps), lot T3 : modifier le réel (règle 21 du pilotage : une fonction par socle).
/// Le professeur n'a aucun de ces droits : il demande (T13), il ne modifie pas ; le motif interne n'est lu que
/// par la direction et le secrétariat.
/// Socle 24, lot F1 (§19.1) : droits fins de la facturation. Les profils qui détenaient
/// `facturation.gerer` (Admin, Direction, Compta) reçoivent les droits fins qui reprennent ce
/// que `gerer` permettait (aucune régression) ; la **séparation des tâches** vient des
/// profils sans `gerer` : le secrétariat n'a que la vue guichet et les engagements, l'accueil
/// que la vue guichet (sans documents). Voter, pilotage, journal et publication : direction
/// (la comptabilité ne vote pas ses propres tarifs, R-F23).
fn socle_24_finances(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    if role.can_guichet_facturation() {
        d.push(etab(FacturationGuichet));
    }
    if role.can_manage_facturation() {
        for a in [
            FacturationEmettre,
            FacturationAvoirs,
            FacturationEcheanciers,
            FacturationReductions,
            FacturationTarifs,
            FacturationSimuler,
            FacturationParametrer,
            FacturationEngagements,
            FacturationCloturer,
            FacturationExporter,
            FacturationReprise,
        ] {
            d.push(etab(a));
        }
    }
    if matches!(role, Role::Admin | Role::Direction) {
        for a in [FacturationVoter, FacturationPilotage, FacturationJournal, FacturationPublier] {
            d.push(etab(a));
        }
    }
    // Secrétariat : enregistre les engagements et les désignations (socle 4), sans toucher aux pièces.
    if role == Role::Administratif {
        d.push(etab(FacturationEngagements));
    }
}

fn socle_10_edt(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    if matches!(role, Role::Direction | Role::Administratif) {
        for a in [EdtModifier, EdtLotAppliquer, EdtMotifInterneLire] {
            d.push(etab(a));
        }
    }
    if matches!(role, Role::Direction | Role::Administratif | Role::Cpe) {
        d.push(etab(EdtModifierLieu));
        d.push(etab(EdtModificationsLire));
        // Lot T4 : le tableau « demain » et « maintenant » (vie scolaire comprise).
        d.push(etab(EdtDemainLire));
    }
    if matches!(role, Role::Direction | Role::Administratif | Role::Cpe | Role::Accueil) {
        d.push(etab(EdtMaintenantLire));
    }
    if role == Role::Direction {
        d.push(etab(EdtCorrigerPasse));
        d.push(etab(EdtForcerConflit));
    }
    // Lot T5 : la publication (contrôles, aperçu, historique ; publier ; revenir en arrière — direction seule).
    if matches!(role, Role::Direction | Role::Administratif) {
        d.push(etab(EdtPublicationLire));
        d.push(etab(EdtPublier));
    }
    if role == Role::Direction {
        d.push(etab(EdtRevenirVersion));
    }
}

fn socle_15_absences_personnels(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    // Socle 15, lots A0–A1 (§23.2). Tout le personnel peut déclarer SA propre absence sans droit
    // (le service le contrôle). L'administrateur technique ne reçoit que le paramétrage : jamais la
    // nature ni la validation ; le DPO et la comptabilité hors circuit ne lisent pas la nature.
    if matches!(role, Role::Direction | Role::Administratif | Role::Compta) {
        d.push(etab(PersonnelsAbsencesDeclarer));
    }
    if matches!(role, Role::Direction | Role::Administratif | Role::Cpe | Role::Accueil) {
        d.push(etab(PersonnelsAbsencesSignaler));
    }
    if matches!(role, Role::Direction | Role::Administratif | Role::Cpe) {
        d.push(etab(PersonnelsAbsencesLireOrganisation));
    }
    if matches!(role, Role::Direction | Role::Administratif) {
        d.push(etab(PersonnelsAbsencesLireNature));
    }
    if role == Role::Direction {
        d.push(etab(PersonnelsAbsencesValider));
    }
    if matches!(role, Role::Direction | Role::Administratif | Role::Compta) {
        d.push(etab(PersonnelsAbsencesInstruire));
    }
    if matches!(role, Role::Direction | Role::Administratif) {
        d.push(etab(PersonnelsAbsencesSaisirValidee));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(PersonnelsAbsencesParametrer));
    }
    // Conservation (gel, purge, confidentialité renforcée) : la direction seule ; jamais l'administrateur
    // technique (R-AP2 : il ne lit rien du niveau RH).
    if role == Role::Direction {
        d.push(etab(PersonnelsAbsencesConserver));
    }
    // Absence longue et suppléance (A5, ABP-180 et 181) : direction et secrétariat.
    if matches!(role, Role::Direction | Role::Administratif) {
        d.push(etab(PersonnelsAbsencesSuppleer));
    }
    // Remplacement de courte durée (A6) : plan et vivier à la direction (adjoint, chargé des remplacements) ;
    // remontée hebdomadaire à la direction et au secrétariat (§23.2).
    if role == Role::Direction {
        d.push(etab(PersonnelsRcdGerer));
    }
    if matches!(role, Role::Direction | Role::Administratif) {
        d.push(etab(PersonnelsRcdRemontee));
    }
    // Heures et états (A7, §23.2) : lecture à la direction et au secrétariat ; validation et signature à la direction ;
    // export au secrétariat (État) et à la comptabilité (OGEC) ; paie et DSN à la comptabilité. Le périmètre d'employeur
    // est contrôlé par le service (le secrétariat ne lit que l'État, la comptabilité que l'OGEC).
    if matches!(role, Role::Direction | Role::Administratif) {
        d.push(etab(PersonnelsHeuresLire));
    }
    if role == Role::Direction {
        d.push(etab(PersonnelsHeuresValider));
        d.push(etab(PersonnelsHeuresSigner));
    }
    if matches!(role, Role::Direction | Role::Administratif | Role::Compta) {
        d.push(etab(PersonnelsHeuresExporter));
    }
    if matches!(role, Role::Direction | Role::Compta) {
        d.push(etab(PersonnelsPaieAbsences));
    }
    // Absences collectives (A9, §16) : la direction (adjoint, chargé des remplacements) coche les grévistes, annule leurs
    // séances en lot et suit le tableau de crise. La liste des grévistes se lit par `lire_nature` (direction, secrétariat).
    if role == Role::Direction {
        d.push(etab(PersonnelsGreveGerer));
    }
}

/// Socle 6 (personnels), lots P5 et P6 : équipes constituées et pouvoirs de signature (règle 21 du pilotage :
/// une fonction par socle). Équipes : direction, administration, secrétariat et CPE ; elles ne donnent aucun droit
/// sur les élèves. Habilitation de signature et intérim : administrateur et direction (chef d'établissement).
fn socle_06_personnels(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe) {
        d.push(etab(EquipesGerer));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(SignatureHabiliter));
    }
}

/// Socle 19 (agenda, réservation de lieux), lots AG1 et AG3. Bloc de `modele_par_defaut`, isolé pour que les fusions de branches ne s'y croisent pas.
/// Socle 5 (structure), lot S6 : groupes et compositions (règle 21 : une fonction par socle). `structure.gerer` et
/// `affectations.gerer` suivent `referentiel.gerer` (Admin, Direction, Administratif : aucun changement de
/// population, §13.2). `groupes.composer` et `groupes.creer_activite` n'ont AUCUN défaut : ils ne se donnent que par
/// un rôle paramétré (« Responsable d'atelier »), jamais à un profil de base.
fn socle_05_structure(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif) {
        d.push((StructureGerer, Portee::Etablissement));
        d.push((AffectationsGerer, Portee::Etablissement));
    }
}

/// Socle 16 (messagerie), lots M3 et M4 (règle 21 : une fonction par socle). `messagerie.boites.gerer` : direction
/// (et Admin, comme `messagerie.urgence_hors_plage`). `messagerie.boite.traiter` : tout le personnel (le droit ouvre
/// la fonction ; la boîte est décidée par la mission, jamais par le droit). `messagerie.message_tel.saisir` : accueil,
/// vie scolaire (CPE), secrétariat, direction. `messagerie.lot.envoyer` : professeurs, CPE, direction (PRF, PP, CPE).
fn socle_16_messagerie(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    if matches!(role, Role::Admin | Role::Direction) {
        d.push((MessagerieBoitesGerer, Portee::Etablissement));
    }
    if role.is_staff() && role != Role::Compta {
        d.push((MessagerieBoiteTraiter, Portee::Etablissement));
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Accueil) {
        d.push((MessagerieMessageTelSaisir, Portee::Etablissement));
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Cpe | Role::Prof) {
        d.push((MessagerieLotEnvoyer, Portee::Etablissement));
    }
}

/// Socle 19 (agenda, réservation de lieux), lots AG1 et AG3. Bloc de `modele_par_defaut`, isolé pour que les fusions de branches ne s'y croisent pas.
fn socle_19_agenda(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    // Socle 19, lot AG1 (`19_AGENDA_RDV.md` §21.1) : créer un événement et lire l'agenda d'une classe
    // sont ouverts à tout le personnel ; publier à une audience : direction et CPE ; journaux :
    // direction ; types d'événements de l'établissement : administrateur.
    d.push(etab(AgendaEvenementsCreer));
    d.push(etab(AgendaLireStructure));
    // Réservation de lieux (AG3) : lire le planning et réserver, tout le personnel ; gérer : direction,
    // administrateur, secrétariat ; valider et paramétrer : direction, administrateur.
    d.push(etab(AgendaLieuxLire));
    d.push(etab(AgendaLieuxReserver));
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif) {
        d.push(etab(AgendaLieuxGerer));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(AgendaLieuxValider));
        d.push(etab(AgendaLieuxParametrer));
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Cpe) {
        d.push(etab(AgendaEvenementsPublier));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(AgendaJournalLire));
        d.push(etab(AgendaSemaineGerer));
    }
    if role == Role::Admin {
        d.push(etab(AgendaParametrer));
    }
}

/// Socle 39 (stages), lots ST0 et ST1. Bloc de `modele_par_defaut`, isolé pour que les fusions de branches ne s'y croisent pas.
fn socle_39_stages(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    // Socle 39, lot ST0 (`39_STAGES.md` §21.1) : le secrétariat et la direction tiennent les stages
    // et le carnet d'organismes ; le professeur lit les stages de ses élèves ; le CPE ne crée plus
    // (RS-02) et n'a que la projection « en stage ».
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif) {
        for a in [StagesLire, StagesPreparer, StagesOrganismesGerer] {
            d.push(etab(a));
        }
    }
    // Lot ST1 (a) : paramétrage et périodes = direction (le DDFPT les recevra par un rôle D-017).
    if matches!(role, Role::Admin | Role::Direction) {
        for a in [StagesParametrer, StagesPeriodesGerer] {
            d.push(etab(a));
        }
    }
    if role == Role::Prof {
        d.push((StagesLire, Portee::MesClasses));
    }
}

/// Socle 12 (appel et assiduité), lots AS1 et AS2. Bloc de `modele_par_defaut`, isolé pour que les fusions de branches ne s'y croisent pas.
fn socle_12_assiduite(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    // Socle 12 (AS1, DA-18) : les droits d'appel et d'assiduité sortent des rôles codés en dur.
    // Le modèle par défaut reproduit exactement `can_faire_appel`, `can_trancher_justificatif`,
    // `can_annuler_absence_vers_present` et `can_gerer_dispenses_vs` ; un rôle libre (D-017) peut
    // désormais ouvrir `appel.faire` à un AED ou `absences.proposer_qualification` sans qualifier.
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Prof) {
        d.push(etab(AppelFaire));
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe) {
        // `absences.proposer_qualification` est listée à part (même profils) : l'anti-escalade compare
        // les actions exactement, la vie scolaire doit donc la détenir pour pouvoir la confier à un rôle libre.
        for a in [AppelFaireTout, AppelCorriger, AbsencesLireMotif, AbsencesQualifier, AbsencesProposerQualification] {
            d.push(etab(a));
        }
    }
    if role == Role::Prof {
        d.push(etab(AppelDeleguer));
    }
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe | Role::Infirmerie) {
        d.push(etab(LocalisationPoser));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(AssiduiteRegleGerer));
    }
    // AS3 : pièces justificatives. Les pièces non médicales sont lues de la vie scolaire (comme la
    // qualification) ; les pièces **médicales** (C3) seulement de la direction et du CPE : ni le
    // secrétariat ni l'administrateur technique. Le professeur n'en lit aucune, jamais (R-AS19).
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe) {
        d.push(etab(AbsencesJustificatifLire));
    }
    if matches!(role, Role::Direction | Role::Cpe) {
        d.push(etab(AbsencesJustificatifMedicalLire));
    }
    // Décision d'autorisation d'absence (R131-5) : chef d'établissement.
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(AbsencesAutorisationDecider));
    }
    // AS5 : suivi de l'information des familles (contacts, relances, courriers). La vie scolaire et le
    // secrétariat le détiennent ; un AED l'obtient par un rôle libre (D-017). Jamais le professeur.
    if matches!(role, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe) {
        d.push(etab(AssiduiteSuivre));
    }
    // AS6 : « prévenir maintenant » (alerte urgente d'une famille) : la direction et la CPE décident ; le
    // secrétariat et le professeur ne la déclenchent pas (un AED l'obtient par un rôle libre si l'établissement le veut).
    if matches!(role, Role::Admin | Role::Direction | Role::Cpe) {
        d.push(etab(AssiduiteAlerter));
    }
}

/// Socle 27 (RGPD, conservation), lot G1 (`27_RGPD_ARCHIVES.md` §12.1). Bloc de `modele_par_defaut`, isolé
/// pour que les fusions de branches ne s'y croisent pas (règle 21).
///
/// La direction lit la conformité, désigne le DPO et ajuste les règles ; **personne** ne reçoit
/// `conservation.regles.valider` par défaut : un établissement sans DPO nommé garde « en attente de
/// DPO » et rien n'est détruit (R-RG1). L'établissement peut confier la validation à un rôle D-017.
/// L'administrateur technique n'a aucun de ces droits (il ne lit pas les décisions de conformité).
fn socle_27_rgpd(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    if role == Role::Direction {
        d.push(etab(ConformiteLire));
        d.push(etab(ConformiteGerer));
        d.push(etab(ConservationReglesProposer));
        d.push(etab(ConformiteValider));
        d.push(etab(SoustraitantsGerer));
        // Lot G3a, guichet des droits (§12.1) : la direction reçoit, traite, remet et arbitre.
        d.push(etab(DroitsRecevoir));
        d.push(etab(DroitsTraiter));
        d.push(etab(DroitsRemettre));
        d.push(etab(DroitsArbitrer));
    }
    // Secrétariat (scolarité) : enregistre une demande et l'instruit. La REMISE de l'extrait reste
    // réservée (« secrétariat habilité », §12.1) : elle s'attribue nommément, jamais par défaut ;
    // les arbitrages (refus, prolongation) sont à la direction (R-RG17).
    if role == Role::Administratif {
        d.push(etab(DroitsRecevoir));
        d.push(etab(DroitsTraiter));
    }
}

/// Socle 29 (portails, index des tâches), lot P2. Bloc de `modele_par_defaut`, isolé pour que les fusions de branches ne s'y croisent pas.
fn socle_29_afaire(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    // Socle 29, lot P2 : rapprochement de l'index des tâches (administrateur seul, §15.1).
    if role == Role::Admin {
        d.push(etab(AfaireRapprocher));
    }
    // Lot P4 : tout le personnel prend en charge dans les files dont il est membre (l'appartenance, elle,
    // est calculée : jamais « tout le personnel », R-DM7) ; réattribuer et superviser : responsables.
    if role.is_staff() {
        d.push(etab(AfairePrendre));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(AfaireReattribuer));
        d.push(etab(AfaireSuperviser));
    }
    // Lot P5 : la recherche est ouverte au personnel (chaque source applique ses propres droits) ; lever un
    // blocage est réservé à l'administrateur.
    if role.is_staff() {
        d.push(etab(RechercheGlobale));
    }
    if role == Role::Admin {
        d.push(etab(RechercheDebloquer));
    }
}

/// Socle 8 (notifications), lot N1. Bloc de `modele_par_defaut`, isolé pour que les fusions de branches ne s'y croisent pas.
/// Voir un avis : direction, administrateur, CPE (vie scolaire) ; régler l'établissement : direction et administrateur.
fn socle_8_notifications(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    if matches!(role, Role::Admin | Role::Direction | Role::Cpe) {
        d.push(etab(NotificationsAvisVoir));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(NotificationsParametrer));
    }
}

/// Socle 42 (sorties et voyages), lot V0a. Bloc de `modele_par_defaut`, isolé pour que les fusions de branches ne s'y croisent pas.
/// Socle 41, lot I2 (`41_INTERNAT.md` §21.1) : la nuit. Le surveillant d'internat, le responsable
/// d'internat et la direction d'une formation sont des rôles paramétrables (D-017) bâtis sur ces
/// actions ; le modèle par défaut ne les donne qu'à l'administration, à la direction et au CPE.
/// Professeur, infirmerie, psychologue et comptabilité n'ont AUCUN de ces droits (corrige RI-03).
fn socle_41_internat(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    if matches!(role, Role::Admin | Role::Direction | Role::Cpe) {
        for a in [
            InternatLire,
            InternatAppel,
            InternatAppelValider,
            InternatSortiesDecider,
            InternatLiaisonEcrire,
            InternatLiaisonLire,
            InternatLiaisonJour,
            InternatEvenementDeclarer,
        ] {
            d.push((a, Portee::Etablissement));
        }
    }
}

fn socle_42_voyages(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
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
    // Lot V0d : la reprise voit des séjours entiers et leurs participants : administrateur et direction seuls.
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(VoyagesReprise));
    }
    // Lot V2a (mode suivi, R-SV44) : l'argent nominatif est réservé à la comptabilité et à la direction ; le secrétariat
    // reçoit les bordereaux de chèques et pointe leurs lignes, sans voir la liste des paiements de la sortie.
    if matches!(role, Role::Admin | Role::Direction | Role::Compta) {
        d.push(etab(VoyagesArgent));
    }
    if matches!(role, Role::Administratif) {
        d.push(etab(VoyagesRemise));
    }
}

/// Socle N1 (demandes et flux de travail), lot N1-0 (`N1_DEMANDES.md` §24.1). Bloc de `modele_par_defaut`, isolé
/// pour que les fusions de branches ne s'y croisent pas (règle 21 de PILOTAGE.md).
///
/// * `demandes.traiter` : **aucun** modèle ne la donne à tout un rôle — l'appartenance à une file (membres calculés
///   depuis les missions et fonctions du socle 6) ouvre l'accès, jamais le rôle (R-DM7, R-DM8, NJ-85).
/// * `demandes.file.repartir`, `demandes.repro.imprimer` : portés par la file ou la mission, pas par le rôle.
/// * `demandes.hse.attester` : l'attestation du service fait est un réglage facultatif **désactivé par défaut**
///   (D-061) ; aucun profil ne l'a tant que l'établissement ne l'a pas activée et paramétrée.
/// * Familles et élèves : aucune action de rôle (D-017) ; leurs droits viennent des publics de la nature.
///
/// Socle 20 (cahier de textes), lot C2 : droits de retrait et de contribution. Les droits `cahier.lire`,
/// `cahier.ecrire` et `cahier.viser` existent depuis S2-6 et restent dans le bloc « Cahier de textes » de
/// `modele_par_defaut` (non déplacés, pour ne pas toucher aux blocs voisins). La direction retire sans écrire
/// (C-D10) ; le professeur retire et contribue dans ses cours, à la date de l'action (R-C2, contrôlé par le service).
fn socle_20_cahier(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    if matches!(role, Role::Admin | Role::Direction) {
        d.push((CahierRetirer, Portee::Etablissement));
        d.push((CahierParametrer, Portee::Etablissement));
    }
    // Un pouvoir = une action (D-069, règle 3) : l'accès d'inspection, les exports figés, la clôture, la réouverture et la
    // décision d'archive sont à la direction seule ; l'administrateur technique n'en a aucun par défaut (secrétariat par
    // délégation, rôle paramétrable : D-017). La réouverture exige en plus une seconde personne de la direction.
    if role == Role::Direction {
        d.push((CahierAccesInspection, Portee::Etablissement));
        d.push((CahierExporter, Portee::Etablissement));
        d.push((CahierCloturer, Portee::Etablissement));
    }
    if role == Role::Prof {
        d.push((CahierRetirer, Portee::MesClasses));
        d.push((CahierContribuer, Portee::MesClasses));
    }
}

fn socle_n1_demandes(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    // Tout le personnel dépose pour lui-même (la nature ouvre ou non le public « personnel »).
    d.push(etab(DemandesDeposer));
    if matches!(role, Role::Accueil | Role::Administratif | Role::Compta | Role::Direction) {
        d.push(etab(DemandesDeposerPourCompte));
    }
    if role == Role::Direction {
        for a in [
            DemandesFileSuperviser,
            DemandesLireToutes,
            DemandesSuperviserConfidentiel,
            DemandesHseDecider,
            DemandesReproValider,
            DemandesIndicateursLire,
        ] {
            d.push(etab(a));
        }
    }
    if matches!(role, Role::Direction | Role::Compta) {
        d.push(etab(DemandesHseEnveloppeGerer));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(DemandesParametrer));
        d.push(etab(DemandesJournalLire));
    }
    if role == Role::Admin {
        d.push(etab(DemandesReprise));
    }
    // Socle 28, lot R3 : la file « à décider » de la reprise montre des fiches d'élèves, de responsables et de personnels
    // côte à côte : secrétariat et direction seulement (moindre privilège ; jamais un enseignant ni la comptabilité).
    if matches!(role, Role::Direction | Role::Administratif) {
        d.push(etab(RepriseDecider));
    }
}

/// Socle 7 (documents), lot D2 : droits par défaut (`07_DOCUMENTS.md` §18.2). Moindre privilège : les domaines
/// restreints (justice, financier, rh) ne s'ouvrent qu'à ceux que la conception nomme ; ni les familles, ni les
/// élèves, ni les professeurs n'émettent de document officiel (D-017, D-022).
fn socle_07_documents(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    if matches!(role, Role::Administratif | Role::Cpe | Role::Direction | Role::Compta) {
        d.push(etab(DocumentsEmettre));
    }
    if matches!(role, Role::Administratif | Role::Direction) {
        d.push(etab(DocumentsAnnuler));
        d.push(etab(DocumentsRegistreLire));
    }
    if matches!(role, Role::Admin | Role::Direction) {
        d.push(etab(DocumentsModelesParametrer));
    }
    if role == Role::Direction {
        d.push(etab(DocumentsRestreintsLire));
        d.push(etab(DocumentsPersonnelsLire));
    }
    if role == Role::Compta {
        d.push(etab(DocumentsFinancierLire));
    }
    // Lot D4 : retirer / rendre une mise à disposition (secrétariat, direction).
    if matches!(role, Role::Administratif | Role::Direction) {
        d.push(etab(DocumentsClasser));
    }
    // Lot D5 : demander et contrôler les pièces (secrétariat, direction, comptabilité pour les bourses, infirmerie pour
    // la santé ; l'accueil ne contrôle que les pièces d'identité : restriction exercée par le service, par type de pièce).
    if matches!(role, Role::Administratif | Role::Direction | Role::Compta | Role::Infirmerie) {
        d.push(etab(DocumentsPiecesDemander));
    }
    if matches!(
        role,
        Role::Administratif | Role::Direction | Role::Compta | Role::Infirmerie | Role::Accueil
    ) {
        d.push(etab(DocumentsPiecesControler));
    }
    // Lot D6 : lancer et suivre les circuits (secrétariat, vie scolaire, direction) ; constater au guichet (accueil compris).
    if matches!(role, Role::Administratif | Role::Cpe | Role::Direction) {
        d.push(etab(DocumentsSignaturesGerer));
    }
    if matches!(role, Role::Administratif | Role::Cpe | Role::Direction | Role::Accueil) {
        d.push(etab(DocumentsSignaturesConstater));
    }
}

/// Socle 32, lots ET0–ET1 : espaces de travail (`32_BUREAUTIQUE.md` §20.1). Les personnels utilisent leur espace
/// personnel, créent des espaces libres entre collègues et partagent nominativement ; la direction lit le journal
/// (jamais le contenu ni les espaces personnels). Les familles et les élèves n'ont aucun droit en ET1 : l'espace
/// personnel et les espaces avec élèves arrivent avec les lots ET2 et suivants (membres calculés, protection du collectif).
/// Bloc isolé : un autre socle n'y touche pas (règle 21 du pilotage).
fn socle_32_espaces(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    d.push(etab(EspacesUtiliser));
    d.push(etab(EspacesCreer));
    d.push(etab(EspacesPartager));
    if role == Role::Direction {
        d.push(etab(EspacesJournalLire));
    }
}

/// Socle 18, lots FO0–FO1 : accords, autorisations et consentements (`18_FORMULAIRES.md` §23.1).
/// Les familles et les élèves n'ont aucun rôle ici : leurs décisions passent par les droits du lien et par leur
/// propre compte (R-FC2). `autorisations.lire` ne donne que des verdicts, jamais une décision ni un motif.
fn socle_18_formulaires(role: Role, d: &mut Vec<(Action, Portee)>) {
    use Action::*;
    let etab = |a: Action| (a, Portee::Etablissement);
    match role {
        Role::Direction => {
            d.push(etab(AutorisationsLire));
            d.push(etab(AutorisationsGerer));
            d.push(etab(AutorisationsParametrer));
            d.push(etab(FormulairesCreer));
            d.push(etab(FormulairesValider));
        }
        Role::Administratif => {
            d.push(etab(AutorisationsLire));
            d.push(etab(AutorisationsGerer));
            d.push(etab(FormulairesCreer));
        }
        Role::Cpe => {
            d.push(etab(AutorisationsLire));
            d.push(etab(FormulairesCreer));
        }
        // Professeurs : verdicts de leurs classes seulement ; formulaires pour leurs classes seulement (PP, PRF).
        Role::Prof => {
            d.push((AutorisationsLire, Portee::MesClasses));
            d.push((FormulairesCreer, Portee::MesClasses));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Socle 18 (FO0–FO1) : droits par défaut. Les verdicts se lisent dans la portée ; les décisions et les
    /// preuves restent à la direction et au secrétariat ; familles, élèves, infirmerie, comptabilité : rien.
    #[test]
    fn socle_18_droits_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action, p: Portee| modele_par_defaut(r).iter().any(|(y, q)| *y == x && *q == p);
        let tout = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        assert!(a(Role::Direction, AutorisationsParametrer, Portee::Etablissement));
        assert!(a(Role::Direction, AutorisationsGerer, Portee::Etablissement));
        assert!(a(Role::Administratif, AutorisationsGerer, Portee::Etablissement));
        assert!(!tout(Role::Administratif, AutorisationsParametrer));
        assert!(a(Role::Cpe, AutorisationsLire, Portee::Etablissement));
        assert!(!tout(Role::Cpe, AutorisationsGerer));
        assert!(a(Role::Prof, AutorisationsLire, Portee::MesClasses));
        assert!(!a(Role::Prof, AutorisationsLire, Portee::Etablissement), "un professeur ne lit que sa classe");
        // FO2 : créer un formulaire (portée), valider (direction seule, jamais l'administratif ni le professeur).
        assert!(a(Role::Direction, FormulairesCreer, Portee::Etablissement));
        assert!(a(Role::Direction, FormulairesValider, Portee::Etablissement));
        assert!(a(Role::Administratif, FormulairesCreer, Portee::Etablissement));
        assert!(a(Role::Cpe, FormulairesCreer, Portee::Etablissement));
        assert!(a(Role::Prof, FormulairesCreer, Portee::MesClasses));
        assert!(!a(Role::Prof, FormulairesCreer, Portee::Etablissement), "un professeur ne cible que ses classes");
        for r in [Role::Administratif, Role::Cpe, Role::Prof, Role::Parent, Role::Eleve] {
            assert!(!tout(r, FormulairesValider), "{r:?} ne valide pas un formulaire");
        }
        for r in [Role::Parent, Role::Eleve, Role::Infirmerie, Role::Compta, Role::Psy, Role::Accueil, Role::Personnel, Role::Admin] {
            for x in [AutorisationsLire, AutorisationsGerer, AutorisationsParametrer] {
                assert!(!tout(r, x), "{r:?} n'a aucun droit d'accord par défaut ({x:?})");
            }
        }
        assert_eq!(Action::TOUTES.iter().filter(|x| x.code().starts_with("autorisations.")).count(), 3);
    }

    /// Socle 27 : droits par défaut de la conformité. Personne ne valide une règle de conservation
    /// sans que l'établissement ait confié ce droit à un rôle (DPO) ; l'administrateur technique ne lit pas
    /// les décisions de conformité ; familles et élèves n'ont rien.
    #[test]
    fn socle_27_droits_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        for r in [
            Role::Admin, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof, Role::Infirmerie,
            Role::Psy, Role::Accueil, Role::Personnel, Role::Parent, Role::Eleve,
        ] {
            for x in [
                ConformiteLire, ConformiteGerer, ConformiteValider, SoustraitantsGerer,
                ConservationReglesProposer, ConservationReglesValider,
            ] {
                assert!(!a(r, x), "{r:?} ne doit pas avoir {x:?} par défaut");
            }
        }
        // Lot G3a : le guichet des droits n'est ouvert qu'à la direction et au secrétariat ; remise et
        // arbitrage restent à la direction (§12.1, R-RG17).
        for r in [
            Role::Admin, Role::Cpe, Role::Compta, Role::Prof, Role::Infirmerie,
            Role::Psy, Role::Accueil, Role::Personnel, Role::Parent, Role::Eleve,
        ] {
            for x in [DroitsRecevoir, DroitsTraiter, DroitsRemettre, DroitsArbitrer] {
                assert!(!a(r, x), "{r:?} ne doit pas avoir {x:?} par défaut");
            }
        }
        assert!(a(Role::Direction, DroitsRecevoir));
        assert!(a(Role::Direction, DroitsTraiter));
        assert!(a(Role::Direction, DroitsRemettre));
        assert!(a(Role::Direction, DroitsArbitrer));
        assert!(a(Role::Administratif, DroitsRecevoir));
        assert!(a(Role::Administratif, DroitsTraiter));
        assert!(
            !a(Role::Administratif, DroitsRemettre),
            "la remise s'attribue nommément (secrétariat habilité)"
        );
        assert!(
            !a(Role::Administratif, DroitsArbitrer),
            "les arbitrages sont à la direction (R-RG17)"
        );
        assert!(a(Role::Direction, ConformiteLire));
        assert!(a(Role::Direction, ConformiteGerer));
        assert!(a(Role::Direction, ConservationReglesProposer));
        assert!(a(Role::Direction, ConformiteValider));
        assert!(a(Role::Direction, SoustraitantsGerer));
        assert!(!a(Role::Direction, ConservationReglesValider), "la validation est celle du DPO");
    }

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
        // Table d'oracle de test : le type tuple est volontairement local.
        #[allow(clippy::type_complexity)]
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

    /// Socle 15 (A0–A1) : la nature d'une absence ne se lit que par la direction et le secrétariat ;
    /// l'administrateur technique, l'accueil, la comptabilité, le professeur et les profils de base
    /// n'ont ni `lire_nature` ni `valider` (R-AP2, R-AP11, AR-01, AR-07). Aucune de ces actions ne se
    /// déduit d'un droit sur les élèves.
    #[test]
    fn absences_personnels_modele_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof, Role::Infirmerie, Role::Psy, Role::Accueil, Role::Personnel, Role::Parent, Role::Eleve] {
            assert_eq!(
                a(r, PersonnelsAbsencesLireNature),
                matches!(r, Role::Direction | Role::Administratif),
                "{r:?} lire_nature"
            );
            assert_eq!(a(r, PersonnelsAbsencesValider), r == Role::Direction, "{r:?} valider");
            assert_eq!(a(r, PersonnelsAbsencesConserver), r == Role::Direction, "{r:?} conserver");
            assert_eq!(
                a(r, PersonnelsAbsencesSuppleer),
                matches!(r, Role::Direction | Role::Administratif),
                "{r:?} suppleer"
            );
            assert_eq!(a(r, PersonnelsRcdGerer), r == Role::Direction, "{r:?} rcd.gerer");
            assert_eq!(
                a(r, PersonnelsRcdRemontee),
                matches!(r, Role::Direction | Role::Administratif),
                "{r:?} rcd.remontee"
            );
            assert_eq!(a(r, PersonnelsHeuresLire), matches!(r, Role::Direction | Role::Administratif), "{r:?} heures.lire");
            assert_eq!(a(r, PersonnelsHeuresValider), r == Role::Direction, "{r:?} heures.valider");
            assert_eq!(a(r, PersonnelsHeuresSigner), r == Role::Direction, "{r:?} heures.signer");
            assert_eq!(
                a(r, PersonnelsHeuresExporter),
                matches!(r, Role::Direction | Role::Administratif | Role::Compta),
                "{r:?} heures.exporter"
            );
            assert_eq!(a(r, PersonnelsPaieAbsences), matches!(r, Role::Direction | Role::Compta), "{r:?} paie.absences");
            assert_eq!(a(r, PersonnelsGreveGerer), r == Role::Direction, "{r:?} greve.gerer");
            assert_eq!(
                a(r, PersonnelsAbsencesParametrer),
                matches!(r, Role::Admin | Role::Direction),
                "{r:?} parametrer"
            );
            if !r.is_staff() {
                for x in [
                    PersonnelsAbsencesDeclarer,
                    PersonnelsAbsencesSignaler,
                    PersonnelsAbsencesLireOrganisation,
                    PersonnelsAbsencesLireNature,
                    PersonnelsAbsencesValider,
                    PersonnelsAbsencesInstruire,
                    PersonnelsAbsencesSaisirValidee,
                    PersonnelsAbsencesParametrer,
                    PersonnelsAbsencesConserver,
                    PersonnelsAbsencesSuppleer,
                    PersonnelsRcdGerer,
                    PersonnelsRcdRemontee,
                    PersonnelsHeuresLire,
                    PersonnelsHeuresValider,
                    PersonnelsHeuresSigner,
                    PersonnelsHeuresExporter,
                    PersonnelsPaieAbsences,
                    PersonnelsGreveGerer,
                ] {
                    assert!(!a(r, x), "{r:?} {}", x.code());
                }
            }
        }
        assert!(!a(Role::Prof, PersonnelsAbsencesLireOrganisation));
        assert!(a(Role::Accueil, PersonnelsAbsencesSignaler));
        assert!(!a(Role::Accueil, PersonnelsAbsencesLireNature));
        assert!(!a(Role::Compta, PersonnelsAbsencesLireNature));
        for x in [PersonnelsAbsencesLireNature, PersonnelsAbsencesValider] {
            assert_eq!(x.portees_admises(), &[Portee::Etablissement]);
            assert!(x.accordee_par().is_empty());
        }
    }

    /// Socle 10, lot T3 : droits de modification du réel. Le professeur, les familles et les profils sans
    /// encadrement n'en ont aucun ; le motif interne reste à la direction et au secrétariat.
    #[test]
    fn edt_t3_modeles_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof, Role::Infirmerie, Role::Psy, Role::Accueil, Role::Personnel, Role::Parent, Role::Eleve] {
            let dir_adm = matches!(r, Role::Direction | Role::Administratif);
            assert_eq!(a(r, EdtModifier), dir_adm, "{r:?} edt.modifier");
            assert_eq!(a(r, EdtLotAppliquer), dir_adm, "{r:?} edt.lot_appliquer");
            assert_eq!(a(r, EdtMotifInterneLire), dir_adm, "{r:?} motif interne");
            assert_eq!(a(r, EdtModifierLieu), dir_adm || r == Role::Cpe, "{r:?} lieu");
            assert_eq!(a(r, EdtModificationsLire), dir_adm || r == Role::Cpe, "{r:?} journal");
            assert_eq!(a(r, EdtCorrigerPasse), r == Role::Direction, "{r:?} correction du passé");
            assert_eq!(a(r, EdtForcerConflit), r == Role::Direction, "{r:?} forcer");
            assert_eq!(a(r, EdtDemainLire), dir_adm || r == Role::Cpe, "{r:?} demain");
            assert_eq!(a(r, EdtMaintenantLire), dir_adm || matches!(r, Role::Cpe | Role::Accueil), "{r:?} maintenant");
        }
        for x in [EdtModifier, EdtModifierLieu, EdtLotAppliquer, EdtCorrigerPasse, EdtMotifInterneLire, EdtModificationsLire, EdtForcerConflit, EdtDemainLire, EdtMaintenantLire] {
            assert_eq!(x.portees_admises(), &[Portee::Etablissement]);
        }
    }

    /// Socle 10, lot T5 : droits de publication. Contrôler, publier : direction et secrétariat ; revenir en arrière :
    /// direction seule ; ni le professeur, ni la vie scolaire, ni les familles.
    #[test]
    fn edt_t5_modeles_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof, Role::Infirmerie, Role::Psy, Role::Accueil, Role::Personnel, Role::Parent, Role::Eleve] {
            let dir_adm = matches!(r, Role::Direction | Role::Administratif);
            assert_eq!(a(r, EdtPublicationLire), dir_adm, "{r:?} publication_lire");
            assert_eq!(a(r, EdtPublier), dir_adm, "{r:?} publier");
            assert_eq!(a(r, EdtRevenirVersion), r == Role::Direction, "{r:?} revenir");
        }
        for x in [EdtPublicationLire, EdtPublier, EdtRevenirVersion] {
            assert_eq!(x.portees_admises(), &[Portee::Etablissement]);
        }
    }

    /// Socle 39, lot ST0 : modèles par défaut des actions de stages (RS-02).
    #[test]
    fn st0_stages_modeles_par_defaut() {
        use Action::*;
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof,
                  Role::Infirmerie, Role::Psy, Role::Accueil, Role::Parent, Role::Eleve] {
            let m = modele_par_defaut(r);
            let a = |x: Action| m.iter().find(|(y, _)| *y == x).map(|(_, p)| *p);
            let gere = matches!(r, Role::Admin | Role::Direction | Role::Administratif);
            for x in [StagesPreparer, StagesOrganismesGerer] {
                assert_eq!(a(x).is_some(), gere, "{r:?} {x:?}");
            }
            for x in [StagesParametrer, StagesPeriodesGerer] {
                assert_eq!(a(x).is_some(), matches!(r, Role::Admin | Role::Direction), "{r:?} {x:?}");
            }
            let lire = if gere {
                Some(Portee::Etablissement)
            } else if r == Role::Prof {
                Some(Portee::MesClasses)
            } else {
                None
            };
            assert_eq!(a(StagesLire), lire, "{r:?} stages.lire");
        }
        assert_eq!(Action::parse("stages.lire"), Some(StagesLire));
        assert_eq!(Action::parse("stages.preparer"), Some(StagesPreparer));
        assert_eq!(Action::parse("stages.organismes.gerer"), Some(StagesOrganismesGerer));
        assert_eq!(StagesOrganismesGerer.portees_admises(), &[Portee::Etablissement]);
        assert_eq!(Action::parse("stages.parametrer"), Some(StagesParametrer));
        assert_eq!(Action::parse("stages.periodes.gerer"), Some(StagesPeriodesGerer));
        assert_eq!(StagesParametrer.portees_admises(), &[Portee::Etablissement]);
        assert!(StagesLire.portees_admises().contains(&Portee::MesClasses));
    }

    /// Lots P5 et P6 (règle 21 : une fonction par socle) : équipes constituées (Admin, Direction, Administratif, CPE),
    /// habilitation de signature et intérim (Admin, Direction) ; établissement seulement ; aucune inclusion implicite ;
    /// le reste du modèle ne bouge pas (chaque rôle reçoit exactement ces deux actions de plus, ou aucune).
    #[test]
    fn p5_p6_equipes_et_signature_modele_par_defaut() {
        use Action::*;
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof,
                  Role::Infirmerie, Role::Psy, Role::Accueil, Role::Parent, Role::Eleve] {
            let m = modele_par_defaut(r);
            let a = |x: Action| m.iter().filter(|(y, _)| *y == x).count();
            assert_eq!(a(EquipesGerer),
                       usize::from(matches!(r, Role::Admin | Role::Direction | Role::Administratif | Role::Cpe)),
                       "{r:?} equipes.gerer");
            assert_eq!(a(SignatureHabiliter), usize::from(matches!(r, Role::Admin | Role::Direction)),
                       "{r:?} signature.habiliter");
        }
        for x in [EquipesGerer, SignatureHabiliter] {
            assert_eq!(x.portees_admises(), &[Portee::Etablissement], "{x:?} : établissement seulement");
            assert!(x.accordee_par().is_empty(), "{x:?} : aucune inclusion implicite");
        }
        assert_eq!(Action::parse("equipes.gerer"), Some(EquipesGerer));
        assert_eq!(Action::parse("signature.habiliter"), Some(SignatureHabiliter));
        // Une équipe ne donne aucun droit sur les élèves : `equipes.gerer` n'est pas une action d'élève.
        assert!(!EquipesGerer.portees_admises().contains(&Portee::MesClasses));
    }

    /// Socle 5, lot S6 (règle 21) : `structure.gerer` et `affectations.gerer` suivent `referentiel.gerer`
    /// (Admin, Direction, Administratif) ; `groupes.composer` et `groupes.creer_activite` n'ont aucun défaut ;
    /// portées admises ; le raccourci `referentiel.gerer` les couvre ; aucun autre profil n'y gagne rien.
    #[test]
    fn s6_structure_groupes_modele_par_defaut() {
        use Action::*;
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof,
                  Role::Infirmerie, Role::Psy, Role::Accueil, Role::Parent, Role::Eleve] {
            let m = modele_par_defaut(r);
            let a = |x: Action| m.iter().filter(|(y, _)| *y == x).count();
            let gestion = usize::from(matches!(r, Role::Admin | Role::Direction | Role::Administratif));
            assert_eq!(a(StructureGerer), gestion, "{r:?} structure.gerer");
            assert_eq!(a(AffectationsGerer), gestion, "{r:?} affectations.gerer");
            assert_eq!(a(GroupesComposer), 0, "{r:?} groupes.composer : aucun défaut");
            assert_eq!(a(GroupesCreerActivite), 0, "{r:?} groupes.creer_activite : aucun défaut");
            // Les nouvelles actions suivent exactement `referentiel.gerer` (même population).
            assert_eq!(a(StructureGerer), a(ReferentielGerer), "{r:?}");
        }
        assert_eq!(StructureGerer.portees_admises(), &[Portee::Etablissement]);
        assert_eq!(GroupesCreerActivite.portees_admises(), &[Portee::Etablissement]);
        assert!(AffectationsGerer.portees_admises().contains(&Portee::Designes));
        assert!(!AffectationsGerer.portees_admises().contains(&Portee::MesGroupes));
        assert!(GroupesComposer.portees_admises().contains(&Portee::MesGroupes));
        assert!(!GroupesComposer.portees_admises().contains(&Portee::MesClasses));
        for x in [StructureGerer, AffectationsGerer, GroupesComposer, GroupesCreerActivite] {
            assert!(x.accordee_par().contains(&ReferentielGerer), "{x:?} : couverte par le raccourci");
        }
        assert!(ReferentielGerer.accordee_par().is_empty());
        for (code, a) in [("structure.gerer", StructureGerer), ("affectations.gerer", AffectationsGerer),
                          ("groupes.composer", GroupesComposer), ("groupes.creer_activite", GroupesCreerActivite)] {
            assert_eq!(Action::parse(code), Some(a));
            assert_eq!(a.code(), code);
        }
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
            // secrétariat (séparation des tâches) ; F1 (R-F37) : « facturation.lire » aussi, remplacé
            // par la vue guichet (« facturation.guichet », hors matrice historique).
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

    /// Socle 19, lot AG1 : modèles par défaut des actions `agenda.*` (`19_AGENDA_RDV.md` §21.1).
    #[test]
    fn agenda_ag1_modeles_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        // Créer un événement et lire la structure : tout le personnel, jamais une famille ni un élève.
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof, Role::Infirmerie, Role::Psy, Role::Accueil] {
            assert!(a(r, AgendaEvenementsCreer), "{r:?} agenda.evenements.creer");
            assert!(a(r, AgendaLireStructure), "{r:?} agenda.lire_structure");
        }
        for r in [Role::Parent, Role::Eleve] {
            assert!(modele_par_defaut(r).is_empty(), "{r:?} : aucun droit paramétrable");
        }
        // Publier à une audience : direction et CPE ; pas un professeur ni l'accueil.
        for (r, attendu) in [(Role::Admin, true), (Role::Direction, true), (Role::Cpe, true), (Role::Prof, false), (Role::Accueil, false), (Role::Administratif, false)] {
            assert_eq!(a(r, AgendaEvenementsPublier), attendu, "{r:?} publier");
        }
        // Journaux : direction ; paramétrage : administrateur seul.
        for (r, j, p) in [(Role::Admin, true, true), (Role::Direction, true, false), (Role::Cpe, false, false), (Role::Prof, false, false)] {
            assert_eq!(a(r, AgendaJournalLire), j, "{r:?} journal");
            assert_eq!(a(r, AgendaParametrer), p, "{r:?} parametrer");
        }
        // Feuille de semaine : direction et administrateur seulement.
        for (r, attendu) in [(Role::Admin, true), (Role::Direction, true), (Role::Cpe, false), (Role::Prof, false), (Role::Administratif, false)] {
            assert_eq!(a(r, AgendaSemaineGerer), attendu, "{r:?} semaine");
        }
        // Réservation de lieux : lire et réserver pour tout le personnel ; gérer ajoute le secrétariat.
        for r in [
            Role::Admin,
            Role::Direction,
            Role::Administratif,
            Role::Cpe,
            Role::Prof,
            Role::Accueil,
        ] {
            assert!(a(r, AgendaLieuxLire) && a(r, AgendaLieuxReserver), "{r:?}");
        }
        for (r, g, v) in [
            (Role::Admin, true, true),
            (Role::Direction, true, true),
            (Role::Administratif, true, false),
            (Role::Prof, false, false),
            (Role::Cpe, false, false),
        ] {
            assert_eq!(a(r, AgendaLieuxGerer), g, "{r:?} gerer");
            assert_eq!(a(r, AgendaLieuxValider), v, "{r:?} valider");
            assert_eq!(a(r, AgendaLieuxParametrer), v, "{r:?} parametrer");
        }
        for x in [
            AgendaEvenementsCreer,
            AgendaEvenementsPublier,
            AgendaLireStructure,
            AgendaJournalLire,
            AgendaParametrer,
            AgendaSemaineGerer,
            AgendaLieuxLire,
            AgendaLieuxReserver,
            AgendaLieuxValider,
            AgendaLieuxGerer,
            AgendaLieuxParametrer,
        ] {
            assert!(x.code().starts_with("agenda."));
            assert_eq!(Action::parse(x.code()), Some(x));
        }
        // Publier accepte la portée « mes classes » (un professeur principal par rôle paramétré).
        assert!(AgendaEvenementsPublier.portees_admises().contains(&Portee::MesClasses));
        assert_eq!(AgendaEvenementsCreer.portees_admises(), &[Portee::Etablissement]);
    }

    /// Socle 42, lot V0a : modèles par défaut des onze actions `voyages.*` (`42_SORTIES_VOYAGES.md` §21.1).
    #[test]
    fn demandes_n1_0_modeles_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        // Aucun rôle ne reçoit `demandes.traiter` ni `demandes.file.repartir` : la file ouvre l'accès (R-DM8).
        let tous = [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof,
                    Role::Infirmerie, Role::Psy, Role::Accueil, Role::Personnel, Role::Parent, Role::Eleve];
        for r in tous {
            assert!(!a(r, DemandesTraiter), "{r:?} ne doit pas avoir demandes.traiter");
            assert!(!a(r, DemandesFileRepartir), "{r:?} ne doit pas avoir demandes.file.repartir");
            // L'attestation du service fait est désactivée par défaut (D-061) ; l'impression vient d'une mission.
            assert!(!a(r, DemandesHseAttester), "{r:?} hse.attester");
            assert!(!a(r, DemandesReproImprimer), "{r:?} repro.imprimer");
        }
        // Familles et élèves : aucune action (D-017).
        for r in [Role::Parent, Role::Eleve] {
            assert!(modele_par_defaut(r).iter().all(|(x, _)| !x.code().starts_with("demandes.")), "{r:?}");
        }
        for r in [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof,
                  Role::Infirmerie, Role::Psy, Role::Accueil, Role::Personnel] {
            assert!(a(r, DemandesDeposer), "{r:?} dépose pour lui-même");
        }
        for (r, pour_compte, lire_toutes, hse, param) in [
            (Role::Direction, true, true, true, true), (Role::Admin, false, false, false, true),
            (Role::Administratif, true, false, false, false), (Role::Compta, true, false, false, false),
            (Role::Accueil, true, false, false, false), (Role::Prof, false, false, false, false),
            (Role::Cpe, false, false, false, false), (Role::Personnel, false, false, false, false),
        ] {
            assert_eq!(a(r, DemandesDeposerPourCompte), pour_compte, "{r:?} pour_compte");
            assert_eq!(a(r, DemandesLireToutes), lire_toutes, "{r:?} lire_toutes");
            assert_eq!(a(r, DemandesHseDecider), hse, "{r:?} hse.decider");
            assert_eq!(a(r, DemandesParametrer), param, "{r:?} parametrer");
        }
        assert!(a(Role::Admin, DemandesReprise) && !a(Role::Direction, DemandesReprise));
        for x in Action::TOUTES.into_iter().filter(|x| x.code().starts_with("demandes.")) {
            assert_eq!(Action::parse(x.code()), Some(x));
            assert_eq!(x.portees_admises(), &[Portee::Etablissement]);
        }
        assert_eq!(Action::TOUTES.iter().filter(|x| x.code().starts_with("demandes.")).count(), 16);
    }

    /// Socle 28, lot R3 : `reprise.decider` est réservée à la direction et au secrétariat, à l'échelle de l'établissement.
    #[test]
    fn reprise_decider_direction_et_secretariat_seulement() {
        let a = |r: Role| modele_par_defaut(r).iter().any(|(y, _)| *y == Action::RepriseDecider);
        for (r, attendu) in [
            (Role::Direction, true), (Role::Administratif, true), (Role::Admin, false), (Role::Compta, false),
            (Role::Prof, false), (Role::Cpe, false), (Role::Accueil, false), (Role::Infirmerie, false),
            (Role::Psy, false), (Role::Personnel, false), (Role::Parent, false), (Role::Eleve, false),
        ] {
            assert_eq!(a(r), attendu, "{r:?} reprise.decider");
        }
        assert_eq!(Action::parse("reprise.decider"), Some(Action::RepriseDecider));
        assert_eq!(Action::RepriseDecider.portees_admises(), &[Portee::Etablissement]);
    }

    /// Socle 42, lot V0d : `voyages.reprise` (reprise de Next.js) est réservée à l'administrateur et à la direction.
    #[test]
    fn voyages_v0d_reprise_admin_et_direction_seulement() {
        let a = |r: Role| modele_par_defaut(r).iter().any(|(y, _)| *y == Action::VoyagesReprise);
        for (r, attendu) in [
            (Role::Admin, true), (Role::Direction, true), (Role::Administratif, false), (Role::Compta, false),
            (Role::Prof, false), (Role::Cpe, false), (Role::Accueil, false), (Role::Infirmerie, false),
            (Role::Psy, false), (Role::Parent, false), (Role::Eleve, false),
        ] {
            assert_eq!(a(r), attendu, "{r:?} voyages.reprise");
        }
        assert_eq!(Action::parse("voyages.reprise"), Some(Action::VoyagesReprise));
        assert_eq!(Action::VoyagesReprise.portees_admises(), &[Portee::Etablissement]);
    }

    /// Socle 42, lot V2a : l'argent nominatif des sorties (`voyages.argent`) est réservé à la comptabilité, à la
    /// direction et à l'administrateur ; le secrétariat n'a que la réception des bordereaux (`voyages.remise`) ;
    /// ni un professeur, ni l'accueil, ni une famille, ni un élève n'ont l'un ou l'autre par rôle.
    #[test]
    fn voyages_v2a_argent_compta_direction_remise_secretariat() {
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        for r in [
            Role::Admin, Role::Direction, Role::Administratif, Role::Compta, Role::Prof, Role::Cpe,
            Role::Accueil, Role::Infirmerie, Role::Psy, Role::Parent, Role::Eleve,
        ] {
            assert_eq!(a(r, Action::VoyagesArgent), matches!(r, Role::Admin | Role::Direction | Role::Compta), "{r:?} voyages.argent");
            assert_eq!(a(r, Action::VoyagesRemise), matches!(r, Role::Administratif), "{r:?} voyages.remise");
        }
        assert_eq!(Action::parse("voyages.argent"), Some(Action::VoyagesArgent));
        assert_eq!(Action::parse("voyages.remise"), Some(Action::VoyagesRemise));
        assert_eq!(Action::VoyagesArgent.portees_admises(), &[Portee::Etablissement]);
        assert_eq!(Action::VoyagesRemise.portees_admises(), &[Portee::Etablissement]);
    }

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

    #[test]
    fn documents_d2_modeles_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, _)| *y == x);
        for (r, emettre, annuler, registre, param, restreint, financier, rh) in [
            (Role::Direction, true, true, true, true, true, false, true),
            (Role::Administratif, true, true, true, false, false, false, false),
            (Role::Cpe, true, false, false, false, false, false, false),
            (Role::Compta, true, false, false, false, false, true, false),
            (Role::Admin, false, false, false, true, false, false, false),
            (Role::Prof, false, false, false, false, false, false, false),
            (Role::Accueil, false, false, false, false, false, false, false),
            (Role::Infirmerie, false, false, false, false, false, false, false),
        ] {
            assert_eq!(a(r, DocumentsEmettre), emettre, "{r:?} emettre");
            assert_eq!(a(r, DocumentsAnnuler), annuler, "{r:?} annuler");
            assert_eq!(a(r, DocumentsRegistreLire), registre, "{r:?} registre");
            assert_eq!(a(r, DocumentsModelesParametrer), param, "{r:?} modeles");
            assert_eq!(a(r, DocumentsRestreintsLire), restreint, "{r:?} restreints");
            assert_eq!(a(r, DocumentsFinancierLire), financier, "{r:?} financier");
            assert_eq!(a(r, DocumentsPersonnelsLire), rh, "{r:?} personnels");
        }
        // Familles et élèves : aucune action documentaire nouvelle (D-017).
        for r in [Role::Parent, Role::Eleve] {
            assert!(modele_par_defaut(r).iter().all(|(x, _)| !x.code().starts_with("documents.emettre")), "{r:?}");
        }
        for x in [DocumentsEmettre, DocumentsAnnuler, DocumentsRegistreLire, DocumentsModelesParametrer,
                  DocumentsRestreintsLire, DocumentsFinancierLire, DocumentsPersonnelsLire] {
            assert!(x.code().starts_with("documents."));
            assert_eq!(Action::parse(x.code()), Some(x));
            assert_eq!(x.portees_admises(), &[Portee::Etablissement]);
        }
    }

    /// Socle 32, lots ET0–ET1 : les personnels utilisent, créent et partagent ; seule la direction lit le journal ;
    /// les familles et les élèves n'ont rien (le modèle par défaut est vide pour eux).
    #[test]
    fn espaces_et1_droits_par_defaut() {
        use Action::*;
        let a = |r: Role, x: Action| modele_par_defaut(r).iter().any(|(y, p)| *y == x && *p == Portee::Etablissement);
        for r in [
            Role::Admin, Role::Direction, Role::Administratif, Role::Cpe, Role::Compta, Role::Prof,
            Role::Infirmerie, Role::Psy, Role::Accueil, Role::Personnel,
        ] {
            for x in [EspacesUtiliser, EspacesCreer, EspacesPartager] {
                assert!(a(r, x), "{r:?} {}", x.code());
            }
            assert_eq!(a(r, EspacesJournalLire), r == Role::Direction, "{r:?} journal");
        }
        for r in [Role::Parent, Role::Eleve] {
            for x in [EspacesUtiliser, EspacesCreer, EspacesPartager, EspacesJournalLire] {
                assert!(!a(r, x), "{r:?} {}", x.code());
            }
        }
        for x in [EspacesUtiliser, EspacesCreer, EspacesPartager, EspacesJournalLire] {
            assert_eq!(Action::parse(x.code()), Some(x));
            assert_eq!(x.portees_admises(), &[Portee::Etablissement]);
        }
    }
}

/// Socle 24, lot F1 : droits fins de la facturation (§19.1) et modèles de rôles par défaut.
#[cfg(test)]
mod f1_droits_fins {
    use super::*;

    fn droits(r: Role) -> Vec<&'static str> {
        modele_par_defaut(r)
            .into_iter()
            .map(|(a, _)| a.code())
            .filter(|c| c.starts_with("facturation.") || *c == "bourses.gerer")
            .collect()
    }

    #[test]
    fn seize_droits_fins_en_fin_de_catalogue_sans_doublon() {
        let fins: Vec<&str> = Action::TOUTES
            .iter()
            .map(|a| a.code())
            .filter(|c| c.starts_with("facturation."))
            .collect();
        // 2 historiques (lire, gerer) + 16 droits fins (guichet, emettre, avoirs, echeanciers, reductions, tarifs,
        // voter, simuler, parametrer, engagements, cloturer, exporter, pilotage, journal, reprise, publier).
        assert_eq!(fins.len(), 18, "{fins:?}");
        for c in [
            "facturation.guichet", "facturation.emettre", "facturation.avoirs", "facturation.echeanciers",
            "facturation.reductions", "facturation.tarifs", "facturation.voter", "facturation.simuler",
            "facturation.parametrer", "facturation.engagements", "facturation.cloturer", "facturation.exporter",
            "facturation.pilotage", "facturation.journal", "facturation.reprise", "facturation.publier",
        ] {
            assert!(fins.contains(&c), "{c} manque");
            assert!(Action::parse(c).is_some(), "{c} ne se relit pas");
        }
        // Les 16 nouveaux suivent le catalogue historique (repéré par position) ; les 2 droits du socle 42 (lot V2a,
        // `voyages.argent` et `voyages.remise`) puis les 2 du socle 20 (lot C8, `cahier.acces_inspection` et
        // `cahier.exporter`) sont ajoutés après eux.
        let n = Action::TOUTES.len();
        assert_eq!(Action::TOUTES[n - 21], Action::FacturationGuichet);
        assert_eq!(Action::TOUTES[n - 6], Action::FacturationPublier);
        assert_eq!(Action::TOUTES[n - 5], Action::VoyagesArgent);
        assert_eq!(Action::TOUTES[n - 4], Action::VoyagesRemise);
        assert_eq!(Action::TOUTES[n - 3], Action::CahierAccesInspection);
        assert_eq!(Action::TOUTES[n - 2], Action::CahierExporter);
        assert_eq!(Action::TOUTES[n - 1], Action::CahierCloturer);
    }

    /// R-F37 : le secrétariat n'a que la vue guichet et les engagements ; l'accueil que la vue guichet.
    #[test]
    fn secretariat_en_vue_guichet_accueil_sans_engagements() {
        assert_eq!(droits(Role::Administratif), vec!["facturation.guichet", "facturation.engagements"]);
        assert_eq!(droits(Role::Accueil), vec!["facturation.guichet"]);
        for r in [Role::Cpe, Role::Prof, Role::Infirmerie, Role::Psy, Role::Personnel, Role::Parent, Role::Eleve] {
            assert!(droits(r).is_empty(), "{r:?} n'a aucun droit financier par défaut : {:?}", droits(r));
        }
    }

    /// R-F23 : la comptabilité ne vote pas ses tarifs, ne lit pas le journal ni le pilotage ; la
    /// direction et l'administrateur gardent tout ce que `facturation.gerer` leur donnait.
    #[test]
    fn compta_n_a_ni_vote_ni_journal_ni_pilotage_direction_a_tout() {
        let compta = droits(Role::Compta);
        for c in ["facturation.gerer", "facturation.lire", "facturation.emettre", "facturation.avoirs", "facturation.engagements", "facturation.reprise", "bourses.gerer"] {
            assert!(compta.contains(&c), "Compta : {c} manque");
        }
        for c in ["facturation.voter", "facturation.journal", "facturation.pilotage", "facturation.publier"] {
            assert!(!compta.contains(&c), "Compta ne doit pas avoir {c}");
        }
        for r in [Role::Admin, Role::Direction] {
            let d = droits(r);
            for a in Action::TOUTES.iter().filter(|a| a.code().starts_with("facturation.")) {
                assert!(d.contains(&a.code()), "{r:?} : {} manque", a.code());
            }
        }
    }

    /// Lire tous les comptes inclut la vue guichet ; rien d'autre n'est accordé par `lire`.
    #[test]
    fn lire_et_gerer_incluent_le_guichet_pas_l_emission() {
        assert!(Action::FacturationGuichet.accordee_par().contains(&Action::FacturationLire));
        assert!(Action::FacturationGuichet.accordee_par().contains(&Action::FacturationGerer));
        assert!(Action::FacturationEmettre.accordee_par().is_empty(), "émettre ne s'obtient jamais par gerer (R-F23)");
        assert!(Action::FacturationAvoirs.accordee_par().is_empty());
        assert!(Action::FacturationJournal.accordee_par().is_empty());
    }

    #[test]
    fn portees_des_droits_financiers_etablissement_seulement() {
        for a in Action::TOUTES.iter().filter(|a| a.code().starts_with("facturation.")) {
            assert_eq!(a.portees_admises(), &[Portee::Etablissement], "{}", a.code());
        }
    }
}
