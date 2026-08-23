import type { RgpdQuestionnaireAnswers, RgpdTemplateSection } from "@/app/lib/rgpd-types";
import {
  rgpdCoordinatorLine,
  rgpdDpoBlock,
  rgpdEffectifLine,
  rgpdEntEditor,
  rgpdEntFullLabel,
  rgpdEntLabel,
  rgpdEntSecondaryLabel,
  rgpdEstablishmentDisplayName,
} from "@/app/lib/rgpd-establishment-helpers";

type RgpdProcessingFicheDef = {
  id: string;
  catalogId: string;
  number: number;
  title: string;
  description: string;
  isApplicable: (answers: RgpdQuestionnaireAnswers) => boolean;
  getTitle?: (answers: RgpdQuestionnaireAnswers) => string;
  buildSections: (answers: RgpdQuestionnaireAnswers, establishmentLabel: string) => RgpdTemplateSection[];
};

function block(
  heading: string,
  paragraphs?: string[],
  bullets?: string[],
): RgpdTemplateSection {
  return { heading, paragraphs, bullets };
}

function pa(answers: RgpdQuestionnaireAnswers) {
  return answers.processingActivities;
}

function fiche0(answers: RgpdQuestionnaireAnswers, establishmentLabel: string): RgpdTemplateSection[] {
  const name = rgpdEstablishmentDisplayName(answers, establishmentLabel);
  const id = answers.establishmentIdentity;
  const today = new Date().toLocaleDateString("fr-FR");
  return [
    block("FICHE 0 : IDENTITÉ DE L'ÉTABLISSEMENT"),
    block("ÉTABLISSEMENT", [name]),
    block("TYPE D'ÉTABLISSEMENT", [
      id?.establishmentType ||
        "Établissement privé sous contrat (maternelle, école primaire, collège, lycée)",
    ]),
    block("NIVEAUX ACCUEILLIS", [
      id?.levelsDescription ||
        (answers.establishmentKinds.length
          ? answers.establishmentKinds
              .map((k) =>
                k === "ecole" ? "école" : k === "college" ? "collège" : "lycée",
              )
              .join(", ")
          : "[À préciser : maternelle, primaire, collège, lycée…]"),
    ]),
    block("EFFECTIFS", [rgpdEffectifLine(answers)]),
    block("RESPONSABLE DE TRAITEMENT (Coordinateur RGPD)", [
      "Coordinateur du groupe scolaire / établissement",
      `Nom : ${id?.coordinatorName || answers.directionReferent || "[À compléter]"}`,
      `Titre : ${id?.coordinatorTitle || "Chef d'établissement"}`,
      `E-mail : ${id?.coordinatorEmail || "[À compléter]"}`,
      `Téléphone : ${id?.coordinatorPhone || "[À compléter]"}`,
    ]),
    block("DÉLÉGUÉ À LA PROTECTION DES DONNÉES (DPO / DPD)", rgpdDpoBlock(answers)),
    block("SUIVI DU REGISTRE", [
      `Date de création du registre : ${id?.registerCreatedAt || today}`,
      `Dernière mise à jour : ${id?.registerUpdatedAt || "[À compléter]"}`,
      `Statut : ${id?.registerStatus || "Document en cours de rédaction / À valider avec la direction"}`,
    ]),
  ];
}

function fiche1(answers: RgpdQuestionnaireAnswers): RgpdTemplateSection[] {
  const ent = rgpdEntFullLabel(answers);
  const primary = rgpdEntLabel(answers);
  const secondary = rgpdEntSecondaryLabel(answers);
  const editor = rgpdEntEditor(answers);
  const editorSecondary =
    secondary && secondary !== primary
      ? `[Éditeur du logiciel complémentaire — à préciser si différent de ${editor}]`
      : null;

  return [
    block(`FICHE 1 : GESTION ADMINISTRATIVE ET PÉDAGOGIQUE DES ÉLÈVES (${ent})`),
    block("LOGICIEL(S) / ENT CONCERNÉ(S)", [
      `Principal : ${primary}`,
      ...(secondary && secondary !== primary ? [`Complémentaire : ${secondary}`] : []),
      `Éditeur / hébergeur principal : ${editor}`,
      ...(editorSecondary ? [editorSecondary] : []),
    ]),
    block("INTITULÉ DU TRAITEMENT", [
      `Gestion administrative, pédagogique et de vie scolaire des élèves via ${ent}`,
    ]),
    block("RESPONSABLE DE TRAITEMENT", [rgpdCoordinatorLine(answers)]),
    block("FINALITÉS", [
      "Inscription et suivi administratif (dossier, coordonnées, responsables légaux)",
      "Gestion de la scolarité (classe, cycle, cursus, passage, redoublement)",
      "Suivi pédagogique (bulletins, résultats, compétences, appréciations)",
      "Vie scolaire (absences, retards, incidents, sanctions, avertissements)",
      "Communication familles (bulletins numériques, messages, avis de direction)",
      "Facturation et frais de scolarité",
      "Documents officiels (certificats, diplômes, attestations)",
      "Transmission aux autorités académiques/diocésaines sur demande officielle",
    ]),
    block("BASE LÉGALE", [
      "Mission d'intérêt public (Service Public d'Éducation — établissement sous contrat)",
      "Obligation légale (décrets et circulaires sur la gestion administrative des élèves)",
      "Exécution du contrat de scolarisation",
    ]),
    block("PERSONNES CONCERNÉES", [
      "Élèves (tous niveaux)",
      "Responsables légaux (parents/tuteurs)",
      "Personnels enseignants et administratifs (direction, coordonnateurs)",
      "Intervenants ponctuels (psychologues, médecins scolaires, prestataires)",
    ]),
    block("CATÉGORIES DE DONNÉES", [
      "Identité : nom, prénom, date de naissance, sexe, photos pédagogiques (si consentement)",
      "Coordonnées élèves et responsables légaux",
      "Données scolaires : classe, notes, résultats, compétences, appréciations",
      "Vie scolaire : absences, retards, incidents, punitions, sanctions",
      "Données financières : frais de scolarité, facturation, n° client",
      "Options, projets pastoraux/catéchèse, BEP (si applicable)",
      "Données de santé minimales (allergies, PAI) — voir fiche dédiée santé",
    ]),
    block("DESTINATAIRES", [
      "Direction et secrétariats",
      "Enseignants et conseil pédagogique",
      "Responsables légaux (portail famille)",
      "Autorités académiques / diocésaines (demande officielle)",
      "OGEC (suivi administratif et financier)",
      `Sous-traitant principal : ${editor} — logiciel(s) ${ent} [localisation hébergement : à confirmer avec l'éditeur, France attendue]`,
      "Microsoft (si Office 365 pour exports/synchronisations)",
    ]),
    block("SOUS-TRAITANTS", [
      `${editor} — éditeur / hébergeur de ${primary} — contrat de sous-traitance RGPD, hébergement, maintenance, support, sauvegardes [À vérifier]`,
      ...(secondary && secondary !== primary
        ? [`Éditeur de ${secondary} : [À identifier et documenter — contrat DPA]`]
        : []),
      "Microsoft (si Office 365) — accord Microsoft 365 Éducation/Business — stockage exports éventuels",
    ]),
    block("DURÉE DE CONSERVATION", [
      "Données actives : durée de scolarité + archivage après départ",
      "Dossier scolaire complet : 50 ans après fin de scolarité (instruction ministérielle 2005)",
      "Réduction possible à 10 ans si récapitulatif administratif/pédagogique informatisé complet",
      "Bulletins et relevés : conservation longue recommandée (50 ans)",
      "Sanctions : effacement progressif selon règlement disciplinaire",
      "Données financières : 6 ans minimum (obligation comptable)",
    ]),
    block("SÉCURITÉ TECHNIQUE", [
      "Accès restreint par rôle (identifiants uniques)",
      "Authentification : mots de passe robustes, renouvellement régulier",
      "Chiffrement en transit et au repos (prestataire ENT)",
      "Sauvegardes régulières et testées",
      "Cloisonnement par établissement/niveau si multi-sites",
    ]),
    block("MESURES ORGANISATIONNELLES", [
      "Accès limité au personnel habilité",
      "Formation RGPD et usages sécurisés",
      "Registre des droits d'accès tenu à jour",
      "Suppression des comptes au départ des salariés",
      "Politique de mots de passe (complexité, renouvellement annuel)",
      "Signalement des incidents aux responsables",
      "Procédure d'exercice des droits documentée",
    ]),
    block("DROIT DES PERSONNES", [
      "Accès, rectification, effacement (limité par obligations légales), opposition (limitée), portabilité",
      "Exercice : demande écrite à la direction ou au secrétariat",
    ]),
    block("CONFORMITÉ", [
      `Contrat de sous-traitance ${editor} : [À vérifier / signer]`,
      "DPIA recommandée pour ce traitement majeur : [À réaliser si nécessaire]",
      "Mentions d'information aux familles à l'inscription",
    ]),
  ];
}

function fiche2(answers: RgpdQuestionnaireAnswers): RgpdTemplateSection[] {
  const id = answers.establishmentIdentity;
  const ent = rgpdEntFullLabel(answers);
  const editor = rgpdEntEditor(answers);
  return [
    block("FICHE 2 : MESSAGERIE ÉLECTRONIQUE (Outlook & messagerie ENT)"),
    block("INTITULÉ DU TRAITEMENT", [
      `Échange de messagerie entre personnels, enseignants et familles (Outlook & ${ent})`,
    ]),
    block("RESPONSABLE DE TRAITEMENT", [rgpdCoordinatorLine(answers)]),
    block("FINALITÉS", [
      "Communication interne (direction, secrétariats, enseignants)",
      "Communication avec les familles (informations, bulletins, demandes)",
      "Communication prestataires et autorités",
      "Traçabilité administrative des échanges",
    ]),
    block("BASE LÉGALE", [
      "Mission d'intérêt public",
      "Exécution du contrat de scolarisation",
    ]),
    block("PERSONNES CONCERNÉES", [
      "Responsables légaux, personnels enseignants et administratifs, prestataires, autorités",
    ]),
    block("CATÉGORIES DE DONNÉES", [
      "Adresses e-mail institutionnelles et personnelles",
      "Contenu des messages et pièces jointes",
      "Historiques et métadonnées (date, destinataires)",
      "Données sensibles éventuelles si mentionnées (santé, discipline, situation familiale)",
    ]),
    block("DESTINATAIRES", [
      "Personnels habilités, familles, prestataires support, autorités (escalade)",
    ]),
    block("SOUS-TRAITANTS", [
      "Microsoft (Outlook / Office 365) — zone d'hébergement France si configurée",
      `${editor} (messagerie intégrée ${rgpdEntLabel(answers)}) — contrat de sous-traitance`,
    ]),
    block("ADRESSES E-MAIL UTILISÉES", [
      `Académiques : ${id?.academicEmailFormat || "[ex. prenom.nom@ac-academie.fr]"}`,
      `Établissement : ${id?.schoolEmailFormat || "[ex. prenom.nom@monecole.fr]"}`,
      `Portail ENT : messagerie intégrée (identifiants uniques)`,
      "Adresses personnelles : usage limité et encadré pour données scolaires",
    ]),
    block("DURÉE DE CONSERVATION", [
      "Boîtes actives : nettoyage régulier recommandé",
      "Archives : 3 ans minimum recommandé",
      "Pièces jointes sensibles : suppression accélérée après usage",
      "Historique ENT : selon politique prestataire",
      "Compte mailbox : suppression 6 mois après départ recommandée",
    ]),
    block("SÉCURITÉ TECHNIQUE", [
      "TLS/SSL, mots de passe forts, MFA recommandée (Office 365)",
      "Sauvegardes Microsoft / prestataire ENT",
    ]),
    block("MESURES ORGANISATIONNELLES", [
      "Formation bonnes pratiques (données sensibles, CCI, copies massives)",
      "Canaux sécurisés pour santé/discipline — pas de mail non sécurisé",
      "Suppression des comptes en fin de contrat",
      "Mention de confidentialité en signature",
    ]),
    block("CONFORMITÉ", [
      "Contrats Microsoft / ENT avec clauses RGPD : [À vérifier]",
      "Sensibilisation utilisateurs à la confidentialité",
    ]),
  ];
}

function fiche3(answers: RgpdQuestionnaireAnswers): RgpdTemplateSection[] {
  const id = answers.establishmentIdentity;
  return [
    block("FICHE 3 : DOSSIERS PAPIER PHYSIQUES (Archives secrétariats)"),
    block("INTITULÉ DU TRAITEMENT", [
      "Stockage et gestion de dossiers papier dans les secrétariats (maternelle, primaire, collège, lycée)",
    ]),
    block("RESPONSABLE DE TRAITEMENT", [
      `${rgpdCoordinatorLine(answers)} (responsabilité globale)`,
      "Coordinateurs locaux par niveau/secrétariat pour l'accès quotidien",
    ]),
    block("FINALITÉS", [
      "Conservation légale des dossiers administratifs et pédagogiques",
      "Attestations, vérifications, litiges",
      "Archivage sécurisé et traçable",
      "Transition progressive vers le numérique",
    ]),
    block("BASE LÉGALE", ["Obligation légale (instruction 2005)", "Mission d'intérêt public"]),
    block("CATÉGORIES DE DONNÉES STOCKÉES", [
      "Inscriptions, identité, santé (fiches, PAI, ordonnances), scolarité, discipline",
      "Correspondance, éléments comptables, archives historiques",
    ]),
    block("LIEUX DE STOCKAGE", [
      "Secrétariats par niveau : classeurs/armoires fermées à clé",
      "Archives historiques : local dédié si possible",
    ]),
    block("ACCÈS AUTORISÉ", [
      "Direction, secrétaires, OGEC, autorités académiques (demande officielle)",
      "Archives départementales (consultation légale)",
      "Familles : consultation sur demande (copie, original non remis)",
    ]),
    block("DURÉE DE CONSERVATION", [
      "Dossier scolaire actif : scolarité + archivage après départ",
      "Dossier complet : 50 ans (10 ans si récapitulatif informatisé)",
      "Bulletins : 50 ans",
      "Copies d'identité : 10 ans après départ",
      "Fiches santé / PAI : durée minimale nécessaire + archivage PAI 5 ans min.",
      "Discipline : voir fiche dédiée ; registre sanctions 10 ans",
      "Correspondance : 5 ans après clôture",
      "Comptabilité : 6 ans minimum",
    ]),
    block("MESURES DE SÉCURITÉ", [
      "Classeurs fermés à clé, locaux restreints, clés détenues par personnel autorisé",
      "Pas de dossiers à vue, stockage protégé humidité/chute",
    ]),
    block("DESTRUCTION ET ARCHIVAGE", [
      "Tri en fin de scolarité selon tableau de conservation",
      "Destruction : bordereau d'élimination + visa Archives départementales obligatoire",
      "Art. L.214-3 code du patrimoine — destruction sans autorisation = délit",
      "Broyage sécurisé avec certificat, copie du bordereau conservée",
      `Archives départementales : ${id?.archivesDepartmentContact || "[Coordonnées à compléter]"}`,
    ]),
    block("TRANSITION NUMÉRIQUE", [
      "Numérisation progressive — mêmes durées de conservation",
      "Destruction papier après numérisation + avis Archives",
    ]),
  ];
}

function fiche4(answers: RgpdQuestionnaireAnswers): RgpdTemplateSection[] {
  const tempDays = answers.establishmentIdentity?.tempRetentionDays ?? 7;
  return [
    block("FICHE 4 : STOCKAGE NUMÉRIQUE ONEDRIVE & CLASSEMENT AUTOMATISÉ VIA IA"),
    block("INTITULÉ DU TRAITEMENT", [
      "OCR, extraction et classement intelligent (OneDrive, Mistral API, AWS Textract, plateforme Scola)",
    ]),
    block("RESPONSABLE DE TRAITEMENT", [rgpdCoordinatorLine(answers)]),
    block("FINALITÉS", [
      "OCR et reconnaissance de documents scannés",
      "Extraction automatisée pour identification élève/personnel/type de document",
      "Classement dans les dossiers OneDrive (élève, année, type)",
      "Réduction du tri manuel, archivage numérique sécurisé",
      "Support à la dématérialisation",
    ]),
    block("BASE LÉGALE", [
      "Mission d'intérêt public / contrat de scolarisation",
      "Obligations légales d'archivage",
    ]),
    block("FLUX DE TRAITEMENT", [
      "1. Acquisition : documents scannés vers sas de traitement",
      "2. IA : Mistral API (France, option sans entraînement) + AWS Textract (OCR)",
      "3. Classement : identification dossier destination → OneDrive",
      "4. Nettoyage : suppression zones temporaires AWS et OneDrive",
    ]),
    block("SOUS-TRAITANTS", [
      "Mistral AI — API analyse, France, DPA, données non utilisées pour entraînement [À documenter]",
      "AWS — Textract + S3 temporaire, zone Europe (Paris/Frankfurt) [À confirmer], SCC/RGPD",
      "Microsoft OneDrive / Office 365 — stockage permanent, zone France si configurée",
      "Scola (intranet) — orchestration, Better-Auth (authentification), AWS S3 (stockage)",
    ]),
    block("DURÉE DE CONSERVATION", [
      `Zones temporaires (AWS + OneDrive temp) : ${tempDays} jours maximum (suppression automatique)`,
      "Fichiers classés OneDrive : selon type (bulletins 50 ans, santé selon fiche dédiée, etc.)",
      "Mistral : traitement stateless, pas de conservation post-traitement (logs techniques minimaux)",
    ]),
    block("SÉCURITÉ TECHNIQUE", [
      "HTTPS/TLS, chiffrement au repos (AWS, Microsoft)",
      "Authentification forte, MFA recommandée OneDrive",
      "Permissions par rôle et besoin fonctionnel",
    ]),
    block("MESURES ORGANISATIONNELLES", [
      "Formation : scanner uniquement via sas prévu",
      "Audit régulier des suppressions automatiques zones temp",
      "Signalement d'incident de sécurité",
      "Procédure de suppression manuelle si erreur de classement",
    ]),
    block("DROIT DE NON-ENTRAÎNEMENT", [
      "Clause Mistral : données non utilisées pour entraînement/ML — conserver la preuve (configuration/e-mail)",
    ]),
    block("CONFORMITÉ", [
      "DPA Mistral, AWS, Microsoft : [À obtenir/vérifier]",
      "Configuration zone Europe AWS : [À confirmer]",
      "Test suppression zones temp : [À réaliser]",
    ]),
  ];
}

function fiche5(answers: RgpdQuestionnaireAnswers): RgpdTemplateSection[] {
  return [
    block("FICHE 5 : DONNÉES DE SANTÉ (Fiches santé, PAI, allergies, certificats médicaux)"),
    block("INTITULÉ DU TRAITEMENT", [
      "Collecte et conservation de données médicales/santé des élèves",
    ]),
    block("RESPONSABLE DE TRAITEMENT", [
      rgpdCoordinatorLine(answers),
      "Coordination locale : infirmière scolaire ou secrétariat",
    ]),
    block("FINALITÉS", [
      "Sécurité et urgence, PAI (allergies, diabète, asthme…)",
      "Suivi médical en milieu scolaire, alertes, traitements administrés",
      "Transmission services d'urgence (SAMU, médecin)",
    ]),
    block("BASE LÉGALE", [
      "Obligation légale (code de l'éducation, circulaires santé)",
      "Intérêt vital / protection de la santé",
      "Consentement explicite des responsables légaux (PAI, transmissions)",
    ]),
    block("CATÉGORIES DE DONNÉES SENSIBLES (art. 9 RGPD)", [
      "Diagnostics, ordonnances, certificats médicaux, rapports",
      "Contacts médicaux, identité et situation familiale connexe",
    ]),
    block("COLLECTE ET CONSENTEMENT", [
      "Formulaire santé à l'inscription, consentement distinct et signé",
      "Information de l'élève selon âge et maturité",
    ]),
    block("DESTINATAIRES", [
      "Direction, infirmière, enseignant (information minimale si PAI), médecin scolaire",
      "Urgences (SAMU, pompiers), parents — pas de tiers sans autorisation sauf urgence vitale",
    ]),
    block("STOCKAGE", [
      "Papier : classeur fermé à clé",
      "Numérique : OneDrive dossier sécurisé, cloisonné du dossier administratif général",
    ]),
    block("DURÉE DE CONSERVATION", [
      "Pendant scolarité : conservation active",
      "PAI / correspondance médicale : 5 ans après fin de scolarité",
      "Certificats médicaux : 5 ans",
      "Ordonnances : 1 an après fin de scolarité",
      "Rapports complexes (handicap) : jusqu'à 10 ans si suivi prolongé",
      "Destruction sécurisée papier et numérique",
    ]),
    block("SÉCURITÉ ET ORGANISATION", [
      "Accès restreint, MFA, pas de transmission par mail standard",
      "Confidentialité stricte, registre d'accès aux données santé",
      "Procédure d'urgence documentée",
    ]),
    block("DROIT DES PERSONNES", [
      "Accès, rectification ; effacement/opposition limités par obligations légales",
      "Retrait de consentement possible avec information sur conservation légale résiduelle",
    ]),
    block("CONFORMITÉ", [
      "Formulaire consentement : [À compléter / distribuer]",
      "Formation personnel confidentialité santé : [À organiser]",
      "DPIA recommandée si volume élevé : [À évaluer]",
    ]),
  ];
}

function fiche6(answers: RgpdQuestionnaireAnswers): RgpdTemplateSection[] {
  return [
    block("FICHE 6 : DOSSIERS DISCIPLINAIRES"),
    block("INTITULÉ DU TRAITEMENT", [
      "Suivi et archivage des incidents de comportement et mesures disciplinaires",
    ]),
    block("RESPONSABLE DE TRAITEMENT", [rgpdCoordinatorLine(answers)]),
    block("FINALITÉS", [
      "Documentation incidents, avertissements, blâmes, mesures de responsabilisation",
      "Décisions de sanctions, historique disciplinaire, exclusions",
      "Défense des intérêts de l'établissement en cas de litige",
    ]),
    block("BASE LÉGALE", [
      "Mission d'intérêt public (discipline, sécurité)",
      "Obligation légale (règlement intérieur, code de l'éducation)",
    ]),
    block("CATÉGORIES DE DONNÉES", [
      "Description incident, nature du manquement, sanctions, signatures, appels, exclusions",
    ]),
    block("STOCKAGE", [
      "Registre des sanctions (papier ou numérique)",
      "Dossiers individuels : classeur sécurisé / OneDrive accès restreint",
    ]),
    block("DURÉE DE CONSERVATION", [
      "Avertissements : effacement fin année scolaire suivante",
      "Blâmes : fin 2e année scolaire suivante",
      "Suspensions/exclusions : jusqu'à fin de scolarité",
      "Exclusions définitives : archivage 50 ans",
      "Registre des sanctions : 10 ans minimum",
    ]),
    block("SÉCURITÉ", [
      "Accès direction, CPE, secrétariat — pas d'affichage des noms",
      "Classeurs fermés, fichiers chiffrés",
    ]),
    block("CONFORMITÉ", [
      "Règlement intérieur affiché et remis aux familles",
      "Droit de défense respecté, procédure d'appel documentée",
    ]),
  ];
}

function fiche7(answers: RgpdQuestionnaireAnswers): RgpdTemplateSection[] {
  const id = answers.establishmentIdentity;
  return [
    block("FICHE 7 : ARCHIVES HISTORIQUES (Années 1960-70 et antérieures)"),
    block("INTITULÉ DU TRAITEMENT", [
      "Stockage d'archives scolaires anciennes (dossiers, bulletins, registres)",
    ]),
    block("RESPONSABLE DE TRAITEMENT", [rgpdCoordinatorLine(answers)]),
    block("FINALITÉS", [
      "Conservation patrimoniale/historique, attestations rétroactives",
      "Obligations légales archives publiques",
    ]),
    block("CATÉGORIES DE DONNÉES", [
      "Bulletins, inscriptions, registres présence, correspondance, résultats",
    ]),
    block("LIEU DE STOCKAGE", [
      "Salle d'archives ou local sécurisé, accès très restreint",
    ]),
    block("DURÉE DE CONSERVATION", [
      "Indéfinie (intérêt archivistique)",
      "Versement Archives départementales envisageable après délai (30+ ans)",
    ]),
    block("STATUT RGPD / CODE DU PATRIMOINE", [
      "Données anciennes — cadre RGPD atténué si personnes non identifiables vivantes",
      "Aucune destruction sans autorisation Archives départementales",
      `Contact : ${id?.archivesDepartmentContact || "[Directeur des Archives départementales]"}`,
    ]),
  ];
}

function fiche8(answers: RgpdQuestionnaireAnswers): RgpdTemplateSection[] {
  return [
    block("FICHE 8 : GESTION FINANCIÈRE / FACTURATION"),
    block("INTITULÉ DU TRAITEMENT", [
      "Frais de scolarité, facturation, paiements, relances, bourses",
    ]),
    block("RESPONSABLE DE TRAITEMENT", [
      rgpdCoordinatorLine(answers),
      "Gestionnaire financier OGEC",
    ]),
    block("FINALITÉS", [
      "Facturation, suivi paiements, impayés, réductions/bourses, comptabilité, audit",
    ]),
    block("BASE LÉGALE", [
      "Contrat de scolarisation",
      "Obligation comptable et fiscale (6 ans)",
    ]),
    block("CATÉGORIES DE DONNÉES", [
      "Identité élève et payeur, coordonnées bancaires, montants, historique transactions",
    ]),
    block("DESTINATAIRES", [
      "OGEC, comptable, direction, trésorier, organismes d'aides",
    ]),
    block("DURÉE DE CONSERVATION", [
      "Factures, reçus : 6 ans minimum",
      "Contrats scolarisation : durée scolarité + 6 ans",
    ]),
    block("SÉCURITÉ", [
      "Accès restreint, chiffrement coordonnées bancaires, pas de transmission mail non sécurisée",
    ]),
  ];
}

function ficheSynthese(): RgpdTemplateSection[] {
  return [
    block("FICHE SYNTHÈSE : TABLEAU RÉCAPITULATIF DURÉES DE CONSERVATION", undefined, [
      "Dossier scolaire complet : 50 ans après fin de scolarité (10 ans si récapitulatif informatisé)",
      "Bulletins, relevés, diplômes : 50 ans",
      "Discipline — avertissements : fin année suivante ; blâmes : fin 2e année ; registre sanctions : 10 ans",
      "Santé — PAI : 5 ans après fin scolarité ; certificats : 5 ans ; ordonnances : 1 an",
      "Copies d'identité / inscription : 10 ans après départ",
      "Factures, pièces comptables : 6 ans minimum",
      "Contrats scolarisation : scolarité + 6 ans",
      "Correspondance générale : 5 ans après clôture dossier",
      "Fichiers temporaires (AWS/OneDrive temp) : 3-7 jours",
      "Messagerie : 3 ans (nettoyage recommandé)",
      "Archives historiques : indéfini — versement Archives départementales",
    ]),
  ];
}

function ficheDeclaration(answers: RgpdQuestionnaireAnswers, establishmentLabel: string): RgpdTemplateSection[] {
  const id = answers.establishmentIdentity;
  const name = rgpdEstablishmentDisplayName(answers, establishmentLabel);
  return [
    block("FICHE FINALE : DÉCLARATION DE CONFORMITÉ & SIGNATURES"),
    block("ÉTABLISSEMENT", [name, `Date : ${new Date().toLocaleDateString("fr-FR")}`]),
    block("RESPONSABLE DE TRAITEMENT (Chef d'établissement)", [
      `Nom : ${id?.coordinatorName || answers.directionReferent || "___________________________"}`,
      "Signature : _____________________ Date : __________________________",
    ]),
    block("DÉLÉGUÉ À LA PROTECTION DES DONNÉES", [
      `Nom : ${answers.dpdName || id?.dpoExternalContact || "___________________________"}`,
      "Signature : _____________________ Date : __________________________",
    ]),
    block("GESTIONNAIRE ADMINISTRATIF / RGPD", [
      `Nom : ${id?.adminReferentName || "___________________________"}`,
      `Fonction : ${id?.adminReferentRole || "Responsable administratif / RGPD"}`,
      "Signature : _____________________ Date : __________________________",
    ]),
    block("DÉCLARATIONS", [
      "☐ Registre établi conformément au RGPD",
      "☐ Mesures de sécurité en place ou en cours",
      "☐ Sous-traitants avec DPA signés [À vérifier]",
      "☐ Droits des personnes documentés et opérationnels",
      "☐ Politique de conservation définie et communiquée",
      "☐ Sensibilisation/formation du personnel [À organiser]",
      "☐ Processus de signalement d'incidents [À documenter]",
    ]),
    block("DOCUMENTS JOINTS / RÉFÉRENCES", [
      "Mentions d'information RGPD",
      "Contrats sous-traitants (ENT, Microsoft, Mistral, AWS)",
      "Formulaire consentement données sensibles",
      "Règlement intérieur",
      "Politique mots de passe / accès",
      "Procédure suppression/archivage",
      "Plan de réponse aux incidents",
    ]),
  ];
}

export const RGPD_PROCESSING_FICHES: RgpdProcessingFicheDef[] = [
  {
    id: "fiche-0-identite",
    catalogId: "fiche-0-identite",
    number: 0,
    title: "Fiche 0 — Identité de l'établissement",
    description: "Identité, effectifs, coordinateur RGPD, DPO, suivi du registre.",
    isApplicable: () => true,
    buildSections: fiche0,
  },
  {
    id: "fiche-1-ent-admin",
    catalogId: "fiche-1-ent-admin",
    number: 1,
    title: "Fiche 1 — Gestion administrative (ENT / logiciel scolaire)",
    description: "Traitement majeur via ENT ou logiciel de gestion (Pronote, École Directe, Charlemagne…).",
    isApplicable: (a) => pa(a).adminEnt,
    getTitle: (a) => `Fiche 1 — Gestion administrative (${rgpdEntFullLabel(a)})`,
    buildSections: (a) => fiche1(a),
  },
  {
    id: "fiche-2-messagerie",
    catalogId: "fiche-2-messagerie",
    number: 2,
    title: "Fiche 2 — Messagerie électronique",
    description: "Outlook, messagerie ENT, échanges avec familles.",
    isApplicable: (a) => pa(a).messaging,
    buildSections: (a) => fiche2(a),
  },
  {
    id: "fiche-3-dossiers-papier",
    catalogId: "fiche-3-dossiers-papier",
    number: 3,
    title: "Fiche 3 — Dossiers papier physiques",
    description: "Archives secrétariats, conservation et destruction.",
    isApplicable: (a) => pa(a).paperFiles,
    buildSections: (a) => fiche3(a),
  },
  {
    id: "fiche-4-onedrive-ia",
    catalogId: "fiche-4-onedrive-ia",
    number: 4,
    title: "Fiche 4 — OneDrive & classement IA (Mistral / AWS)",
    description: "OCR, classement automatisé, zones temporaires.",
    isApplicable: (a) => pa(a).onedriveAi,
    buildSections: (a) => fiche4(a),
  },
  {
    id: "fiche-5-donnees-sante",
    catalogId: "fiche-5-donnees-sante",
    number: 5,
    title: "Fiche 5 — Données de santé",
    description: "Fiches santé, PAI, allergies, certificats médicaux.",
    isApplicable: (a) => pa(a).healthData || a.sensitiveProcessing.healthData,
    buildSections: (a) => fiche5(a),
  },
  {
    id: "fiche-6-discipline",
    catalogId: "fiche-6-discipline",
    number: 6,
    title: "Fiche 6 — Dossiers disciplinaires",
    description: "Incidents, sanctions, registre disciplinaire.",
    isApplicable: (a) => pa(a).disciplinary,
    buildSections: (a) => fiche6(a),
  },
  {
    id: "fiche-7-archives-historiques",
    catalogId: "fiche-7-archives-historiques",
    number: 7,
    title: "Fiche 7 — Archives historiques",
    description: "Archives anciennes, versement Archives départementales.",
    isApplicable: (a) => pa(a).historicalArchives,
    buildSections: (a) => fiche7(a),
  },
  {
    id: "fiche-8-gestion-financiere",
    catalogId: "fiche-8-gestion-financiere",
    number: 8,
    title: "Fiche 8 — Gestion financière / facturation",
    description: "Frais de scolarité, paiements, comptabilité.",
    isApplicable: (a) => pa(a).financial,
    buildSections: (a) => fiche8(a),
  },
  {
    id: "fiche-synthese-conservation",
    catalogId: "fiche-synthese-conservation",
    number: 9,
    title: "Fiche synthèse — Durées de conservation",
    description: "Tableau récapitulatif des durées légales et recommandées.",
    isApplicable: () => true,
    buildSections: () => ficheSynthese(),
  },
  {
    id: "fiche-declaration-conformite",
    catalogId: "fiche-declaration-conformite",
    number: 10,
    title: "Fiche finale — Déclaration de conformité",
    description: "Signatures direction, DPO, gestionnaire administratif.",
    isApplicable: () => true,
    buildSections: ficheDeclaration,
  },
];

export function getRgpdProcessingFiche(id: string): RgpdProcessingFicheDef | undefined {
  return RGPD_PROCESSING_FICHES.find((f) => f.id === id || f.catalogId === id);
}

export function buildFullRegisterSections(
  answers: RgpdQuestionnaireAnswers,
  establishmentLabel: string,
): RgpdTemplateSection[] {
  const sections: RgpdTemplateSection[] = [
    block("REGISTRE DES ACTIVITÉS DE TRAITEMENT", [
      `Établissement : ${rgpdEstablishmentDisplayName(answers, establishmentLabel)}`,
      `Document généré le ${new Date().toLocaleDateString("fr-FR")}`,
      "Conforme à l'article 30 du RGPD — registre détaillé par fiches de traitement.",
    ]),
  ];
  for (const fiche of RGPD_PROCESSING_FICHES) {
    if (!fiche.isApplicable(answers)) continue;
    sections.push(block("—".repeat(40)));
    sections.push(...fiche.buildSections(answers, establishmentLabel));
  }
  return sections;
}

export function buildFicheSections(
  ficheId: string,
  answers: RgpdQuestionnaireAnswers,
  establishmentLabel: string,
): RgpdTemplateSection[] | null {
  const fiche = getRgpdProcessingFiche(ficheId);
  if (!fiche) return null;
  return fiche.buildSections(answers, establishmentLabel);
}
