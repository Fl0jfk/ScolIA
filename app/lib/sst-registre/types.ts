import type { SstFicheStatus, SstFicheWorkflow } from "@/db/schema-sst-registre";

export type { SstFicheStatus, SstFicheWorkflow };

export const SST_FICHE_STATUSES: SstFicheStatus[] = [
  "ouverte",
  "visa_responsable",
  "examen_cse",
  "decision",
  "realisation",
  "cloturee",
];

export const SST_FICHE_STATUS_LABELS: Record<SstFicheStatus, string> = {
  ouverte: "Ouverte",
  visa_responsable: "Visa responsable sécurité",
  examen_cse: "Examen CSE / CHSCT",
  decision: "Décision",
  realisation: "Réalisation / suivi",
  cloturee: "Clôturée",
};

export type SstCampagneDto = {
  id: string;
  anneeLabel: string;
  title: string;
  active: boolean;
};

export type SstEmargementDto = {
  id: string;
  userId: string;
  firstName: string;
  lastName: string;
  fonction: string;
  signedAt: string;
  remarques: string;
  hasSignature: boolean;
};

export type SstMyStatus = {
  campagne: SstCampagneDto;
  signed: boolean;
  emargement: SstEmargementDto | null;
  canManage: boolean;
  openFichesCount: number;
};

export type SstSuiviPerson = {
  userId: string;
  firstName: string;
  lastName: string;
  email: string;
  fonction: string;
  signed: boolean;
  signedAt: string | null;
};

export type SstSuiviPayload = {
  campagne: SstCampagneDto;
  total: number;
  signedCount: number;
  pendingCount: number;
  people: SstSuiviPerson[];
  fiches: SstFicheListItem[];
  consultations: SstConsultationDto[];
};

export type SstFicheListItem = {
  id: string;
  numero: number;
  status: SstFicheStatus;
  observedDate: string;
  lieu: string;
  observationsPreview: string;
  declarantName: string;
  createdAt: string;
  updatedAt: string;
};

export type SstFicheDetail = {
  id: string;
  numero: number;
  status: SstFicheStatus;
  campagneId: string;
  createdByUserId: string;
  declarantFirstName: string;
  declarantLastName: string;
  declarantFonction: string;
  observedDate: string;
  observedTime: string;
  lieu: string;
  observations: string;
  suggestions: string;
  hasSignature: boolean;
  workflow: SstFicheWorkflow;
  createdAt: string;
  updatedAt: string;
  canAdvance: boolean;
};

export type SstConsultationDto = {
  id: string;
  userId: string;
  firstName: string;
  lastName: string;
  fonction: string;
  consultedAt: string;
  comments: string;
  hasSignature: boolean;
};

export function isSstFicheStatus(v: unknown): v is SstFicheStatus {
  return typeof v === "string" && (SST_FICHE_STATUSES as string[]).includes(v);
}

export function parseSignaturePngDataUrl(raw: string): string | null {
  const s = raw.trim();
  if (!s.startsWith("data:image/png;base64,")) return null;
  const b64 = s.slice("data:image/png;base64,".length);
  if (b64.length < 80 || b64.length > 900_000) return null;
  if (!/^[A-Za-z0-9+/=\s]+$/.test(b64)) return null;
  return s;
}

export function previewText(text: string, max = 120): string {
  const t = text.replace(/\s+/g, " ").trim();
  if (t.length <= max) return t;
  return `${t.slice(0, max - 1)}…`;
}
