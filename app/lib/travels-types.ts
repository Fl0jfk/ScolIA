/** Modèle de données Travels (S3 `travels/{id}.json`). */

const TRAVELS_STATUSES = [
  "EN_ATTENTE_DIR_INITIAL",
  "PROF_LOGISTICS",
  "EN_ATTENTE_BUS_SIGNATURE",
  "EN_ATTENTE_COMPTA",
  "EN_ATTENTE_DIR_FINAL",
  "VALIDE",
  "BESOIN_MODIFICATION",
  "REJETE",
  "SEANCE_ANNULEE",
  "ANNULE",
] as const;

export type TravelsStatus = (typeof TRAVELS_STATUSES)[number];

export const TRAVELS_STATUS_LABELS: Record<string, string> = {
  EN_ATTENTE_DIR_INITIAL: "Validation pédagogique",
  PROF_LOGISTICS: "Choix du devis transport",
  EN_ATTENTE_BUS_SIGNATURE: "Signature devis bus",
  EN_ATTENTE_COMPTA: "Validation finances",
  EN_ATTENTE_DIR_FINAL: "Validation finale",
  VALIDE: "Finalisé",
  BESOIN_MODIFICATION: "Modifications demandées",
  REJETE: "Refusé",
  SEANCE_ANNULEE: "Séance annulée",
  ANNULE: "Sortie annulée",
};

export type TravelsTripType = "SIMPLE" | "COMPLEX";

export type TransportAmendment = {
  sentAt: string;
  previousEffectif: { nbEleves: number; nbAccompagnateurs: number } | null;
  newEffectif: { nbEleves: number; nbAccompagnateurs: number };
  recipientMode: "single" | "all";
  providerName?: string | null;
  providerEmail?: string | null;
};

export type CuisineAmendment = {
  sentAt: string;
  previousSnapshot: Record<string, unknown> | null;
  newSnapshot: Record<string, unknown>;
  actor?: string;
};

export type TravelsHistoryEntry = {
  date?: string;
  user?: string;
  action?: string;
  note?: string;
};

/** Élève inscrit sur une sortie (snapshot depuis eleves.json). */
export type TravelsParticipantEleve = {
  ine: string;
  nom: string;
  prenom: string;
  classe?: string;
  /** Défaut true — responsabilité établissement (rappel UI). */
  droitImageOk: boolean;
};

export type TravelsParentComLog = {
  id: string;
  sentAt: string;
  sentBy: { userId: string; name?: string };
  subject: string;
  body: string;
  photoCount: number;
  recipientCount: number;
  /** true si un fichier .ics RDV a été joint. */
  icsAttached?: boolean;
};

/** RDV parents legacy (un créneau) — préférer `parentCalendar`. */
export type TravelsParentMeeting = {
  /** YYYY-MM-DD */
  date: string;
  /** HH:mm */
  time: string;
  durationMinutes?: number;
  place?: string;
  note?: string;
};

export type TravelsCalendarPointKind = "depot" | "recuperation" | "autre";

/** Point d’attention calendrier (dépôt, récupération, autre). */
export type TravelsCalendarPoint = {
  id: string;
  kind: TravelsCalendarPointKind;
  label?: string;
  date: string;
  time: string;
  durationMinutes?: number;
  place?: string;
  note?: string;
};

/** Calendrier parents : un .ics = séjour + points. */
export type TravelsParentCalendar = {
  /** Inclure l’événement couvrant tout le séjour (défaut true). */
  includeTripSpan?: boolean;
  points: TravelsCalendarPoint[];
};

export type TravelsTripData = {
  title?: string;
  destination?: string;
  etablissement?: string;
  classes?: string;
  objectifs?: string;
  date?: string;
  startDate?: string;
  endDate?: string;
  startTime?: string;
  endTime?: string;
  nbEleves?: number | string;
  nbAccompagnateurs?: number | string;
  nomsAccompagnateurs?: string;
  coutTotal?: number;
  finalTotalCost?: number;
  costPerStudent?: number;
  needsBus?: boolean;
  previousStatus?: string;
  /** Motif direction tant que status = BESOIN_MODIFICATION (indépendant de l'historique). */
  modificationRequestNote?: string;
  transportRequest?: Record<string, unknown>;
  selectedBusQuote?: Record<string, unknown>;
  signedQuoteUrl?: string;
  signedQuoteS3Key?: string;
  transportQuoteSnapshot?: {
    nbEleves: number;
    nbAccompagnateurs: number;
    sentAt?: string;
    type?: string;
  };
  transportDateSnapshot?: {
    startDate?: string;
    endDate?: string;
    startTime?: string;
    endTime?: string;
    sentAt?: string;
  };
  transportAmendments?: TransportAmendment[];
  /** Messages transport reçus par e-mail (IA : chauffeur, confirmation, etc.). */
  transportEmailMessages?: Array<{
    id: string;
    gmailMessageId: string;
    fromEmail: string;
    toEmail?: string;
    subject: string;
    messageType: string;
    summary: string;
    driverName?: string | null;
    driverPhone?: string | null;
    details?: string | null;
    pdfUrl?: string | null;
    s3KeyIncoming?: string | null;
    originalFilename?: string | null;
    receivedAt: string;
    matchConfidence?: string | null;
    matchMotif?: string | null;
    source?: string;
    direction?: "inbound" | "outbound";
    replyToMessageId?: string | null;
  }>;
  /** Dernière confirmation de commande transporteur reçue par e-mail (IA). */
  transportProviderConfirmation?: {
    receivedAt: string;
    summary: string;
    fromEmail?: string;
    providerName?: string | null;
    pdfUrl?: string | null;
    s3KeyIncoming?: string | null;
    originalFilename?: string | null;
    gmailMessageId?: string;
  };
  pendingAmendedQuote?: boolean;
  /** Direction : passage aux finances sans devis transport signé. */
  transportPhaseBypassedAt?: string;
  transportPhaseBypassedBy?: string;
  transportPhaseBypassNote?: string;
  piqueNique?: boolean;
  piqueNiqueDetails?: Record<string, unknown>;
  cuisineOrderSentAt?: string;
  cuisineOrderSnapshot?: Record<string, unknown>;
  cuisineAmendments?: CuisineAmendment[];
  attachments?: {
    name: string;
    url: string;
    s3Key?: string;
    source?: string;
    gmailMessageId?: string;
  }[];
  recurrenceSeriesId?: string;
  recurrenceIndex?: number;
  recurrenceTotal?: number;
  cancelledAt?: string;
  cancelReason?: string;
  remindersSent?: Record<string, string>;
  /** Liste nominative d’élèves (sélection classes → élèves). */
  participantEleves?: TravelsParticipantEleve[];
  listeElevesStatus?: "draft" | "confirmed";
  listeElevesConfirmedAt?: string;
  listeElevesConfirmedBy?: { userId: string; email?: string; name?: string };
  listeEnvoyeeTransporteurAt?: string;
  /** Historique des envois com’ parents (métadonnées, pas les photos). */
  parentComLogs?: TravelsParentComLog[];
  /** Heure / lieu de rendez-vous parents (legacy — un créneau). */
  parentMeeting?: TravelsParentMeeting;
  /** Calendrier parents multi-points (séjour + dépôt + récupération…). */
  parentCalendar?: TravelsParentCalendar;
  /** Fiche budget compta (OCR devis + saisie manuelle). */
  comptaSheet?: import("@/app/lib/travels-compta-sheet").TravelsComptaSheet;
  [key: string]: unknown;
};

export type TravelsTrip = {
  id: string;
  type: TravelsTripType;
  status: TravelsStatus | string;
  ownerName?: string;
  ownerEmail?: string;
  ownerId?: string;
  createdAt?: string;
  updatedAt?: string;
  imageUrl?: string;
  imageConfigId?: string;
  data: TravelsTripData;
  receivedDevis?: Record<string, unknown>[];
  history?: TravelsHistoryEntry[];
  messages?: Array<{ id: string; user: string; role: string; text: string; date: string }>;
};

export type TravelsHubTab =
  | "overview"
  | "eleves"
  | "communication"
  | "transport"
  | "cuisine"
  | "documents"
  | "journal"
  | "messages"
  | "actions"
  | "compta";

export const TRAVELS_HUB_TABS: { id: TravelsHubTab; label: string; icon: string }[] = [
  { id: "overview", label: "Vue d'ensemble", icon: "🏠" },
  { id: "eleves", label: "Élèves", icon: "👥" },
  { id: "communication", label: "Communication", icon: "📸" },
  { id: "transport", label: "Transport", icon: "🚌" },
  { id: "cuisine", label: "Cuisine", icon: "🍽️" },
  { id: "documents", label: "Documents", icon: "📁" },
  { id: "journal", label: "Journal", icon: "📜" },
  { id: "messages", label: "Messagerie", icon: "💬" },
  { id: "actions", label: "Actions", icon: "⚡" },
  { id: "compta", label: "Compta", icon: "📊" },
];
