export type RgpdDocumentLevel = "obligatoire" | "recommande" | "conditionnel";
export type RgpdDocumentStatus =
  | "manquant"
  | "brouillon"
  | "genere"
  | "importe"
  | "valide"
  | "non_applicable";

export type RgpdPlatformDpaId = "microsoft365" | "aws" | "mistral" | "better-auth";

export type RgpdEstablishmentKind = "ecole" | "college" | "lycee" | "groupe";

export type RgpdEntPresetId =
  | "charlemagne_ecoledirecte"
  | "pronote"
  | "ecole_directe"
  | "autre";

export type RgpdEstablishmentIdentity = {
  legalName?: string;
  establishmentType?: string;
  levelsDescription?: string;
  coordinatorName?: string;
  coordinatorTitle?: string;
  coordinatorEmail?: string;
  coordinatorPhone?: string;
  dpoDesignatedBy?: string;
  dpoExternalContact?: string;
  dpoExternalEmail?: string;
  archivesDepartmentContact?: string;
  adminReferentName?: string;
  adminReferentRole?: string;
  /** Preset ENT / logiciel de gestion scolaire (fiche 1) */
  entPreset?: RgpdEntPresetId;
  entEditor?: string;
  entProducts?: string;
  /** Second logiciel ou ENT complémentaire (ex. Charlemagne + ÉcoleDirecte) */
  secondaryEntProducts?: string;
  academicEmailFormat?: string;
  schoolEmailFormat?: string;
  tempRetentionDays?: number;
  registerCreatedAt?: string;
  registerUpdatedAt?: string;
  registerStatus?: string;
};

export type RgpdProcessingActivities = {
  adminEnt: boolean;
  messaging: boolean;
  paperFiles: boolean;
  onedriveAi: boolean;
  healthData: boolean;
  disciplinary: boolean;
  historicalArchives: boolean;
  financial: boolean;
};

export type RgpdQuestionnaireAnswers = {
  establishmentKinds: RgpdEstablishmentKind[];
  isGroup: boolean;
  establishmentIdentity?: RgpdEstablishmentIdentity;
  processingActivities: RgpdProcessingActivities;
  studentCount?: number;
  teacherCount?: number;
  staffCount?: number;
  dpdDesignated: boolean;
  dpdName?: string;
  dpdExternalUserId?: string;
  dpdEmail?: string;
  dpdInternal: boolean;
  directionReferent?: string;
  audiences: {
    students: boolean;
    parents: boolean;
    staff: boolean;
    prospects: boolean;
    alumni: boolean;
  };
  sensitiveProcessing: {
    photosVideos: boolean;
    publications: boolean;
    boarding: boolean;
    healthData: boolean;
    videoSurveillance: boolean;
    biometrics: boolean;
    aiTools: boolean;
  };
  subprocessors: {
    ent: boolean;
    entName?: string;
    microsoft365: boolean;
    scola: boolean;
    /** @deprecated utiliser aws */
    hosting?: boolean;
    aws: boolean;
    mistralAi: boolean;
    otherSaas: boolean;
    otherSaasList?: string;
  };
  /** DPA des plateformes (Microsoft, AWS, Mistral, Better-Auth…) */
  platformDpas?: Partial<Record<RgpdPlatformDpaId, boolean>>;
  existingMeasures: {
    hasRegister: boolean;
    hasBreachProcedure: boolean;
    hasItCharter: boolean;
    hasPhotoCharter: boolean;
  };
  questionnaireStep: number;
  questionnaireCompleted: boolean;
};

export type RgpdDocumentAnalysis = {
  documentScore: number;
  presentCriteria: string[];
  missingCriteria: string[];
  improvements: string[];
  suggestedActions: string[];
  analyzedAt: string;
};

export type RgpdDocumentState = {
  status: RgpdDocumentStatus;
  generatedAt?: string;
  importedAt?: string;
  fileKey?: string;
  fileName?: string;
  analysis?: RgpdDocumentAnalysis;
  notApplicableReason?: string;
};

export type RgpdWorkspaceHistoryEntry = {
  at: string;
  by: string;
  action: string;
  note?: string;
};

export type RgpdComplianceScore = {
  total: number;
  mandatoryDocs: number;
  recommendedDocs: number;
  questionnaire: number;
  quality: number;
  breakdown: {
    mandatoryPresent: number;
    mandatoryTotal: number;
    recommendedPresent: number;
    recommendedTotal: number;
    questionnairePercent: number;
    averageQuality: number;
  };
};

export type RgpdIncidentKind = "violation_donnees" | "incident_securite";

export type RgpdDataBreachFields = {
  discoveredAt?: string;
  nature?: string;
  dataCategories?: string;
  affectedCount?: number;
  severity?: "faible" | "moyenne" | "elevee";
  immediateMeasures?: string;
  cnilNotificationRequired?: boolean;
  cnilNotificationPlannedAt?: string;
  dataSubjectsNotified?: boolean;
  responsibleName?: string;
  timeline?: string;
  description?: string;
};

export type RgpdSecurityIncidentFields = {
  occurredAt?: string;
  incidentType?: string;
  impactedSystems?: string;
  potentialDataImpact?: string;
  containmentMeasures?: string;
  providerContacted?: string;
  status?: "ouvert" | "en_cours" | "clos";
  description?: string;
};

export type RgpdIncident = {
  id: string;
  kind: RgpdIncidentKind;
  title: string;
  createdAt: string;
  updatedAt: string;
  createdBy: { userId: string; name: string };
  fields: RgpdDataBreachFields | RgpdSecurityIncidentFields;
  chatHistory?: Array<{ role: "user" | "assistant"; content: string }>;
};

export type RgpdDocumentSectionPreview = {
  heading: string;
  paragraphs?: string[];
  bullets?: string[];
};

export type RgpdTemplateSection = RgpdDocumentSectionPreview;

export type RgpdDocumentContentPreview = {
  docId: string;
  title: string;
  sections: RgpdDocumentSectionPreview[];
  disclaimer: string;
  applicable: boolean;
  reason: string;
  hasPdf: boolean;
  pdfFileName?: string;
  generatedAt?: string;
  importedAt?: string;
  status: string;
};

export type RgpdWorkspace = {
  version: 1;
  answers: RgpdQuestionnaireAnswers;
  documents: Record<string, RgpdDocumentState>;
  incidents: string[];
  history: RgpdWorkspaceHistoryEntry[];
  updatedAt: string;
};

export const DEFAULT_RGPD_ANSWERS: RgpdQuestionnaireAnswers = {
  establishmentKinds: [],
  isGroup: false,
  processingActivities: {
    adminEnt: true,
    messaging: true,
    paperFiles: true,
    onedriveAi: true,
    healthData: true,
    disciplinary: true,
    historicalArchives: false,
    financial: true,
  },
  establishmentIdentity: {
    entPreset: "autre",
    registerStatus: "Document en cours de rédaction / À valider avec la direction",
  },
  dpdDesignated: false,
  dpdInternal: true,
  audiences: {
    students: true,
    parents: true,
    staff: true,
    prospects: false,
    alumni: false,
  },
  sensitiveProcessing: {
    photosVideos: true,
    publications: true,
    boarding: false,
    healthData: true,
    videoSurveillance: false,
    biometrics: false,
    aiTools: false,
  },
  subprocessors: {
    ent: true,
    entName: "",
    microsoft365: true,
    scola: true,
    aws: true,
    mistralAi: true,
    otherSaas: false,
  },
  platformDpas: {
    microsoft365: true,
    aws: true,
    mistral: true,
    "better-auth": true,
  },
  existingMeasures: {
    hasRegister: false,
    hasBreachProcedure: false,
    hasItCharter: false,
    hasPhotoCharter: false,
  },
  questionnaireStep: 1,
  questionnaireCompleted: false,
};
