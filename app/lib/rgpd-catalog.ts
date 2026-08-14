import type { RgpdDocumentLevel, RgpdQuestionnaireAnswers } from "@/app/lib/rgpd-types";
import { getEnabledPlatformDpas } from "@/app/lib/rgpd-platform-dpas";
export type RgpdCatalogEntry = {
  id: string;
  title: string;
  level: RgpdDocumentLevel;
  description: string;
  legalRef: string;
  templateId: string;
  weight: number;
  checklist: string[];
  /** true = document outil (incidents), pas dans le score documentaire principal */
  incidentTool?: boolean;
};

export const RGPD_CATALOG: RgpdCatalogEntry[] = [
  {
    id: "registre-traitements",
    title: "Registre des activités de traitement",
    level: "obligatoire",
    description: "Inventaire des traitements de données personnelles (article 30 RGPD).",
    legalRef: "Art. 30 RGPD",
    templateId: "registre-traitements",
    weight: 12,
    checklist: [
      "Finalités des traitements",
      "Catégories de personnes concernées",
      "Catégories de données",
      "Destinataires et sous-traitants",
      "Durées de conservation",
      "Mesures de sécurité",
      "Responsable du traitement identifié",
    ],
  },
  {
    id: "mentions-information",
    title: "Mentions d'information (élèves, parents, personnel)",
    level: "obligatoire",
    description: "Information des personnes sur leurs données et leurs droits (articles 13-14 RGPD).",
    legalRef: "Art. 13-14 RGPD",
    templateId: "mentions-information",
    weight: 10,
    checklist: [
      "Identité du responsable de traitement",
      "Finalités et base légale",
      "Destinataires des données",
      "Durées de conservation",
      "Droits des personnes (accès, rectification, effacement…)",
      "Droit de réclamation CNIL",
      "Coordonnées du DPD si désigné",
    ],
  },
  {
    id: "procedure-droits",
    title: "Procédure d'exercice des droits",
    level: "obligatoire",
    description: "Organisation interne pour traiter les demandes d'accès, rectification, opposition, etc.",
    legalRef: "Art. 12-22 RGPD",
    templateId: "procedure-droits",
    weight: 8,
    checklist: [
      "Canal de réception des demandes",
      "Délai de réponse (1 mois)",
      "Vérification de l'identité du demandeur",
      "Registre des demandes",
      "Procédure de refus motivé",
      "Rôle du DPD / référent",
    ],
  },
  {
    id: "procedure-violation",
    title: "Procédure de notification de violation (72 h)",
    level: "obligatoire",
    description: "Procédure en cas de violation de données personnelles.",
    legalRef: "Art. 33-34 RGPD",
    templateId: "procedure-violation",
    weight: 10,
    checklist: [
      "Détection et signalement interne",
      "Évaluation du risque",
      "Notification CNIL sous 72 h si requis",
      "Information des personnes si risque élevé",
      "Registre des violations",
      "Mesures correctives",
    ],
  },
  {
    id: "registre-violations",
    title: "Registre des violations de données",
    level: "obligatoire",
    description: "Traçabilité des incidents et violations même si non notifiés à la CNIL.",
    legalRef: "Art. 33 RGPD",
    templateId: "registre-violations",
    weight: 6,
    checklist: [
      "Date et nature de la violation",
      "Catégories de données concernées",
      "Nombre de personnes affectées",
      "Conséquences et mesures prises",
      "Notification CNIL (oui/non)",
    ],
  },
  {
    id: "liste-sous-traitants",
    title: "Liste des sous-traitants et clauses RGPD",
    level: "conditionnel",
    description: "Inventaire des prestataires traitant des données pour le compte de l'établissement.",
    legalRef: "Art. 28 RGPD",
    templateId: "liste-sous-traitants",
    weight: 8,
    checklist: [
      "Liste nominative des sous-traitants",
      "Finalités confiées",
      "Localisation des données",
      "Existence de clauses contractuelles / DPA",
      "Date de mise à jour",
    ],
  },
  {
    id: "politique-protection",
    title: "Politique / charte de protection des données",
    level: "recommande",
    description: "Document de référence sur la gouvernance et les engagements de l'établissement.",
    legalRef: "Bonnes pratiques CNIL",
    templateId: "politique-protection",
    weight: 5,
    checklist: [
      "Engagement de la direction",
      "Principes généraux",
      "Rôles et responsabilités",
      "Formation du personnel",
      "Révision périodique",
    ],
  },
  {
    id: "charte-informatique",
    title: "Charte informatique et usages numériques",
    level: "recommande",
    description: "Règles d'usage des outils numériques par élèves et personnel.",
    legalRef: "Bonnes pratiques CNIL éducation",
    templateId: "charte-informatique",
    weight: 5,
    checklist: [
      "Usages autorisés et interdits",
      "Sécurité des mots de passe",
      "Messagerie et réseaux sociaux",
      "Sanctions en cas de manquement",
      "Signature / validation",
    ],
  },
  {
    id: "charte-photos",
    title: "Charte photos / vidéos / droit à l'image",
    level: "recommande",
    description: "Encadrement des prises de vue et publications.",
    legalRef: "Code civil art. 9, CNIL",
    templateId: "charte-photos",
    weight: 5,
    checklist: [
      "Finalités des prises de vue",
      "Consentement ou opposition",
      "Durée de conservation des images",
      "Publications sur site web / réseaux",
      "Droit à l'image des mineurs",
    ],
  },
  {
    id: "designation-dpd",
    title: "Désignation / mission du DPD",
    level: "recommande",
    description: "Document formalisant la mission du délégué à la protection des données.",
    legalRef: "Art. 37-39 RGPD",
    templateId: "designation-dpd",
    weight: 4,
    checklist: [
      "Identité du DPD",
      "Mission et moyens",
      "Indépendance et absence de conflit d'intérêts",
      "Lien avec la CNIL",
    ],
  },
  {
    id: "aipd",
    title: "Analyse d'impact (AIPD)",
    level: "conditionnel",
    description: "Requise pour les traitements à risque élevé (vidéosurveillance, biométrie, etc.).",
    legalRef: "Art. 35 RGPD",
    templateId: "aipd",
    weight: 8,
    checklist: [
      "Description du traitement",
      "Nécessité et proportionnalité",
      "Risques pour les personnes",
      "Mesures d'atténuation",
      "Avis du DPD",
      "Consultation CNIL si risque résiduel élevé",
    ],
  },
  {
    id: "politique-conservation",
    title: "Politique de conservation des données",
    level: "recommande",
    description: "Durées de conservation par type de données et modalités d'archivage/suppression.",
    legalRef: "Art. 5.1.e RGPD",
    templateId: "politique-conservation",
    weight: 6,
    checklist: [
      "Durées par catégorie de données",
      "Archivage intermédiaire",
      "Suppression sécurisée",
      "Responsable de l'application",
      "Revue annuelle",
    ],
  },
  {
    id: "mesures-securite",
    title: "Mesures de sécurité (PSSI simplifiée)",
    level: "recommande",
    description: "Mesures techniques et organisationnelles de protection des données.",
    legalRef: "Art. 32 RGPD",
    templateId: "mesures-securite",
    weight: 5,
    checklist: [
      "Contrôle d'accès",
      "Sauvegardes",
      "Gestion des incidents",
      "Sensibilisation du personnel",
      "Mises à jour et antivirus",
    ],
  },
  {
    id: "fiche-violation",
    title: "Fiche incident — violation de données",
    level: "obligatoire",
    description: "Outil de saisie d'un incident de violation de données personnelles.",
    legalRef: "Art. 33 RGPD",
    templateId: "fiche-violation",
    weight: 0,
    incidentTool: true,
    checklist: [],
  },
  {
    id: "fiche-securite",
    title: "Fiche incident — sécurité informatique",
    level: "recommande",
    description: "Outil de saisie d'un incident de sécurité informatique.",
    legalRef: "Art. 32 RGPD",
    templateId: "fiche-securite",
    weight: 0,
    incidentTool: true,
    checklist: [],
  },
];

export function getRgpdCatalogEntry(docId: string): RgpdCatalogEntry | undefined {
  return RGPD_CATALOG.find((d) => d.id === docId);
}

type RgpdDocumentRequirement = {
  docId: string;
  applicable: boolean;
  reason: string;
};

export function evaluateDocumentApplicability(
  entry: RgpdCatalogEntry,
  answers: RgpdQuestionnaireAnswers,
): RgpdDocumentRequirement {
  if (entry.incidentTool) {
    return { docId: entry.id, applicable: false, reason: "Outil incidents (hors score documentaire)." };
  }

  if (entry.level === "obligatoire") {
    if (entry.id === "mentions-information") {
      const any =
        answers.audiences.students ||
        answers.audiences.parents ||
        answers.audiences.staff;
      return {
        docId: entry.id,
        applicable: any,
        reason: any ? "Publics concernés déclarés." : "Aucun public déclaré.",
      };
    }
    return { docId: entry.id, applicable: true, reason: "Document obligatoire." };
  }

  if (entry.id === "liste-sous-traitants") {
    const sp = answers.subprocessors;
    const hasPlatforms = getEnabledPlatformDpas(answers).length > 0;
    const has =
      sp.ent ||
      sp.microsoft365 ||
      sp.scola ||
      sp.aws ||
      sp.hosting ||
      sp.mistralAi ||
      sp.otherSaas ||
      hasPlatforms;
    return {
      docId: entry.id,
      applicable: has,
      reason: has ? "Sous-traitants déclarés." : "Aucun sous-traitant déclaré.",
    };
  }

  if (entry.id === "aipd") {
    const s = answers.sensitiveProcessing;
    const high =
      s.videoSurveillance || s.biometrics || (s.aiTools && s.healthData);
    return {
      docId: entry.id,
      applicable: high,
      reason: high
        ? "Traitement à risque élevé déclaré."
        : "Pas de traitement à risque élevé identifié.",
    };
  }

  if (entry.id === "charte-informatique") {
    const applicable =
      answers.subprocessors.microsoft365 ||
      answers.subprocessors.ent ||
      answers.subprocessors.scola;
    return {
      docId: entry.id,
      applicable,
      reason: applicable ? "Outils numériques utilisés." : "Peu d'outils numériques déclarés.",
    };
  }

  if (entry.id === "charte-photos") {
    const applicable =
      answers.sensitiveProcessing.photosVideos ||
      answers.sensitiveProcessing.publications;
    return {
      docId: entry.id,
      applicable,
      reason: applicable ? "Photos/vidéos ou publications déclarées." : "Pas de traitement image déclaré.",
    };
  }

  if (entry.id === "designation-dpd") {
    return {
      docId: entry.id,
      applicable: answers.dpdDesignated,
      reason: answers.dpdDesignated ? "DPD désigné." : "Pas de DPD désigné.",
    };
  }

  if (entry.level === "recommande") {
    return { docId: entry.id, applicable: true, reason: "Document recommandé." };
  }

  return { docId: entry.id, applicable: true, reason: "Applicable par défaut." };
}

export function evaluateAllDocumentRequirements(
  answers: RgpdQuestionnaireAnswers,
): RgpdDocumentRequirement[] {
  return RGPD_CATALOG.filter((e) => !e.incidentTool).map((e) =>
    evaluateDocumentApplicability(e, answers),
  );
}

export const RGPD_QUESTIONNAIRE_STEPS = [
  { id: 1, title: "Profil établissement", key: "profile" },
  { id: 2, title: "Gouvernance", key: "governance" },
  { id: 3, title: "Publics & données", key: "audiences" },
  { id: 4, title: "Traitements sensibles", key: "sensitive" },
  { id: 5, title: "Outils & sous-traitants", key: "subprocessors" },
  { id: 6, title: "Mesures actuelles", key: "existing" },
  { id: 7, title: "Synthèse", key: "summary" },
] as const;

export const RGPD_TOTAL_STEPS = RGPD_QUESTIONNAIRE_STEPS.length;
